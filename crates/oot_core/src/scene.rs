//! Scene header commands (`z64scene.h`): 8-byte `{ u8 code, u8 data1, u16 pad, u32 data2 }`
//! entries ending at `SCENE_CMD_ID_END`. Decodes what the game needs to play and draw a scene:
//! collision, spawns, the room list, the keep object, light settings and skybox settings,
//! for the header layer the game would pick (`Scene_CommandAlternateHeaderList`).

use std::sync::Arc;

use anyhow::{Result, bail};

use crate::collision::CollisionHeader;
use crate::rom::Rom;

pub const CMD_SPAWN_LIST: u8 = 0x00;
pub const CMD_ACTOR_LIST: u8 = 0x01;
pub const CMD_COLLISION_HEADER: u8 = 0x03;
pub const CMD_ROOM_LIST: u8 = 0x04;
pub const CMD_SPECIAL_FILES: u8 = 0x07;
pub const CMD_ROOM_BEHAVIOR: u8 = 0x08;
pub const CMD_ROOM_SHAPE: u8 = 0x0A;
pub const CMD_LIGHT_SETTINGS_LIST: u8 = 0x0F;
pub const CMD_TIME_SETTINGS: u8 = 0x10;
pub const CMD_SKYBOX_SETTINGS: u8 = 0x11;
pub const CMD_SKYBOX_DISABLES: u8 = 0x12;
pub const CMD_END: u8 = 0x14;
pub const CMD_ECHO_SETTINGS: u8 = 0x16;
pub const CMD_ALTERNATE_HEADER_LIST: u8 = 0x18;
/// Scene files are mapped to segment 2 while loaded.
pub const SCENE_SEGMENT: u8 = 0x02;
/// Room files are mapped to segment 3.
pub const ROOM_SEGMENT: u8 = 0x03;

/// `SceneLayer`: which alternate header a load uses.
pub const LAYER_CHILD_DAY: usize = 0;
pub const LAYER_CHILD_NIGHT: usize = 1;
pub const LAYER_ADULT_DAY: usize = 2;
pub const LAYER_ADULT_NIGHT: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneCommand {
    pub code: u8,
    pub data1: u8,
    pub data2: u32,
}

/// `ActorEntry` from the spawn/actor lists (0x10 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorEntry {
    pub id: i16,
    pub pos: [i16; 3],
    pub rot: [i16; 3],
    pub params: i16,
}

/// `EnvLightSettings` (`z64environment.h`, 0x16 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvLightSettings {
    pub ambient: [u8; 3],
    pub light1_dir: [i8; 3],
    pub light1_color: [u8; 3],
    pub light2_dir: [i8; 3],
    pub light2_color: [u8; 3],
    pub fog_color: [u8; 3],
    /// Blend rate in the top 6 bits (`>> 10`, × 4 per frame), fog near in the low 10.
    pub fog_near_raw: u16,
    pub fog_far: i16,
}

impl EnvLightSettings {
    pub fn fog_near(&self) -> i16 {
        (self.fog_near_raw & 0x3FF) as i16
    }
}

/// `SCmdSkyboxSettings`: skybox id, skybox config and `LIGHT_MODE_*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SkyboxSettings {
    pub skybox_id: u8,
    pub config: u8,
    pub light_mode: u8,
}

/// A `RomFile` from the room list: the room file's VROM range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomRef {
    pub vrom_start: u32,
    pub vrom_end: u32,
}

fn be16(b: &[u8], o: usize) -> u16 {
    u16::from_be_bytes([b[o], b[o + 1]])
}
fn be32(b: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

/// Reads the main header's commands at the start of a scene file.
pub fn commands(scene: &[u8]) -> Result<Vec<SceneCommand>> {
    commands_at(scene, 0)
}

/// Reads header commands starting at `offset` up to (not including) END.
pub fn commands_at(data: &[u8], offset: usize) -> Result<Vec<SceneCommand>> {
    let mut out = Vec::new();
    for i in 0..64 {
        let o = offset + i * 8;
        if o + 8 > data.len() {
            bail!("header runs past the file");
        }
        let cmd = SceneCommand { code: data[o], data1: data[o + 1], data2: be32(data, o + 4) };
        if cmd.code == CMD_END {
            return Ok(out);
        }
        out.push(cmd);
    }
    bail!("no SCENE_CMD_ID_END in the first 64 commands")
}

/// The commands the game executes for `layer`, as `Scene_ExecuteCommands` +
/// `Scene_CommandAlternateHeaderList` do: the alternate header list is the first command;
/// for a non-zero layer its entry `layer - 1` replaces the rest of the main header. A missing
/// adult-night header falls back to adult day; any other missing header keeps the main one.
/// Returns the header's offset in the file and its commands.
pub fn layer_commands(data: &[u8], segment: u8, layer: usize) -> Result<(usize, Vec<SceneCommand>)> {
    let main = commands_at(data, 0)?;
    if layer == 0 {
        return Ok((0, main));
    }
    let Some(alt) = main.iter().find(|c| c.code == CMD_ALTERNATE_HEADER_LIST) else { return Ok((0, main)) };
    if alt.data2 >> 24 != segment as u32 {
        return Ok((0, main));
    }
    let list = (alt.data2 & 0xFF_FFFF) as usize;
    let entry = |l: usize| -> Option<usize> {
        let o = list + (l - 1) * 4;
        let v = (o + 4 <= data.len()).then(|| be32(data, o))?;
        (v != 0 && v >> 24 == segment as u32).then_some((v & 0xFF_FFFF) as usize)
    };
    let pick = entry(layer).or(if layer == LAYER_ADULT_NIGHT { entry(LAYER_ADULT_DAY) } else { None });
    match pick {
        Some(off) => Ok((off, commands_at(data, off)?)),
        None => Ok((0, main)),
    }
}

/// Reads `n` actor entries (0x10 bytes each) at a segment-local offset.
pub fn actor_entries(data: &[u8], offset: usize, n: usize) -> Vec<ActorEntry> {
    (0..n)
        .filter_map(|i| {
            let o = offset + i * 16;
            (o + 16 <= data.len()).then(|| ActorEntry {
                id: be16(data, o) as i16,
                pos: [be16(data, o + 2) as i16, be16(data, o + 4) as i16, be16(data, o + 6) as i16],
                rot: [be16(data, o + 8) as i16, be16(data, o + 10) as i16, be16(data, o + 12) as i16],
                params: be16(data, o + 14) as i16,
            })
        })
        .collect()
}

pub struct Scene {
    pub name: String,
    pub file: Arc<[u8]>,
    /// The layer this scene was loaded for and its header's commands.
    pub layer: usize,
    /// Offset of the header used (0 = the main header).
    pub header_offset: usize,
    pub header: Vec<SceneCommand>,
    pub collision: CollisionHeader,
    pub spawns: Vec<ActorEntry>,
    pub rooms: Vec<RoomRef>,
    /// `SCENE_CMD_ID_SPECIAL_FILES`: the keep object loaded into segment 5.
    pub keep_object: Option<u16>,
    pub light_settings: Vec<EnvLightSettings>,
    pub skybox: SkyboxSettings,
}

impl Scene {
    pub fn parse(name: &str, file: &[u8]) -> Result<Scene> {
        Self::parse_layer(name, file.into(), LAYER_CHILD_DAY)
    }

    pub fn parse_layer(name: &str, file: Arc<[u8]>, layer: usize) -> Result<Scene> {
        let (header_offset, cmds) = layer_commands(&file, SCENE_SEGMENT, layer)?;
        let local = |a: u32| (a & 0xFF_FFFF) as usize;
        let find = |code: u8| cmds.iter().find(|c| c.code == code).copied();
        let col = find(CMD_COLLISION_HEADER).ok_or_else(|| anyhow::anyhow!("{name}: no collision header command"))?;
        let collision = CollisionHeader::parse(&file, SCENE_SEGMENT, local(col.data2))?;
        let spawns = find(CMD_SPAWN_LIST).map(|c| actor_entries(&file, local(c.data2), c.data1 as usize)).unwrap_or_default();
        let rooms = find(CMD_ROOM_LIST)
            .map(|c| {
                (0..c.data1 as usize)
                    .filter_map(|i| {
                        let o = local(c.data2) + i * 8;
                        (o + 8 <= file.len()).then(|| RoomRef { vrom_start: be32(&file, o), vrom_end: be32(&file, o + 4) })
                    })
                    .collect()
            })
            .unwrap_or_default();
        // OBJECT_INVALID is 0 (object_table.h: the first entry is DEFINE_OBJECT_UNSET).
        let keep_object = find(CMD_SPECIAL_FILES).map(|c| (c.data2 & 0xFFFF) as u16).filter(|&k| k != 0);
        let light_settings = find(CMD_LIGHT_SETTINGS_LIST)
            .map(|c| {
                (0..c.data1 as usize)
                    .filter_map(|i| {
                        let o = local(c.data2) + i * 0x16;
                        let b = file.get(o..o + 0x16)?;
                        let rgb = |k: usize| [b[k], b[k + 1], b[k + 2]];
                        let dir = |k: usize| [b[k] as i8, b[k + 1] as i8, b[k + 2] as i8];
                        Some(EnvLightSettings {
                            ambient: rgb(0),
                            light1_dir: dir(3),
                            light1_color: rgb(6),
                            light2_dir: dir(9),
                            light2_color: rgb(12),
                            fog_color: rgb(15),
                            fog_near_raw: be16(b, 0x12),
                            fog_far: be16(b, 0x14) as i16,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let skybox = find(CMD_SKYBOX_SETTINGS)
            .map(|c| {
                let b = c.data2.to_be_bytes();
                SkyboxSettings { skybox_id: b[0], config: b[1], light_mode: b[2] }
            })
            .unwrap_or_default();
        Ok(Scene {
            name: name.to_string(),
            layer,
            header_offset,
            header: cmds,
            collision,
            spawns,
            rooms,
            keep_object,
            light_settings,
            skybox,
            file,
        })
    }

    /// Loads `<name>_scene` (or `name` if it already ends in `_scene`) from the ROM.
    pub fn load(rom: &Rom, name: &str) -> Result<Scene> {
        Self::load_layer(rom, name, LAYER_CHILD_DAY)
    }

    pub fn load_layer(rom: &Rom, name: &str, layer: usize) -> Result<Scene> {
        let file_name = file_name(name);
        let data = rom.file_by_name(&file_name)?;
        Scene::parse_layer(&file_name, data, layer)
    }

    /// The scene's name without the `_scene` suffix.
    pub fn short_name(&self) -> &str {
        self.name.strip_suffix("_scene").unwrap_or(&self.name)
    }
}

/// `spot04` → `spot04_scene`.
pub fn file_name(name: &str) -> String {
    if name.ends_with("_scene") { name.to_string() } else { format!("{name}_scene") }
}
