//! Scene header commands (`z64scene.h`): 8-byte `{ u8 code, u8 data1, u16 pad, u32 data2 }`
//! entries ending at `SCENE_CMD_ID_END`. Decodes what the game needs to play and draw a scene:
//! collision, spawns, the room list, the keep object, light settings and skybox settings,
//! for the header layer the game would pick (`Scene_CommandAlternateHeaderList`).

use std::sync::Arc;

use anyhow::{Result, bail};
use eng_collision::collision::CollisionHeader;

use crate::rom::Rom;
pub use oot_game::scene::{ActorEntry, EntranceEntry, EnvLightSettings, Path, SkyboxSettings, TransitionActorEntry};
use crate::z64::CollisionCodec;

pub const CMD_SPAWN_LIST: u8 = 0x00;
pub const CMD_ACTOR_LIST: u8 = 0x01;
pub const CMD_COLLISION_HEADER: u8 = 0x03;
pub const CMD_ROOM_LIST: u8 = 0x04;
pub const CMD_ENTRANCE_LIST: u8 = 0x06;
pub const CMD_SPECIAL_FILES: u8 = 0x07;
pub const CMD_ROOM_BEHAVIOR: u8 = 0x08;
pub const CMD_ROOM_SHAPE: u8 = 0x0A;
pub const CMD_OBJECT_LIST: u8 = 0x0B;
pub const CMD_PATH_LIST: u8 = 0x0D;
pub const CMD_TRANSITION_ACTOR_LIST: u8 = 0x0E;
pub const CMD_LIGHT_SETTINGS_LIST: u8 = 0x0F;
pub const CMD_TIME_SETTINGS: u8 = 0x10;
pub const CMD_SKYBOX_SETTINGS: u8 = 0x11;
pub const CMD_SKYBOX_DISABLES: u8 = 0x12;
pub const CMD_EXIT_LIST: u8 = 0x13;
pub const CMD_END: u8 = 0x14;
pub const CMD_ECHO_SETTINGS: u8 = 0x16;
pub const CMD_CUTSCENE_DATA: u8 = 0x17;
pub const CMD_ALTERNATE_HEADER_LIST: u8 = 0x18;
pub const CMD_MISC_SETTINGS: u8 = 0x19;
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
    /// `SCENE_CMD_ID_ENTRANCE_LIST`, `SCENE_CMD_ID_EXIT_LIST` and
    /// `SCENE_CMD_ID_TRANSITION_ACTOR_LIST`. The first two carry no length: see
    /// [`list_extent`].
    pub entrances: Vec<EntranceEntry>,
    pub exits: Vec<u16>,
    pub transition_actors: Vec<TransitionActorEntry>,
    /// `SCENE_CMD_ID_PATH_LIST` (`play->setupPathList`), which carries no length either: see
    /// [`path_list`].
    pub paths: Vec<Path>,
    /// `SCENE_CMD_ID_MISC_SETTINGS`' `sceneCamType` (`R_SCENE_CAM_TYPE`, `SCENE_CAM_TYPE_*`).
    pub scene_cam_type: u8,
    /// `SCENE_CMD_ID_CUTSCENE_DATA`: the offset of the script `Scene_CommandCutsceneData` puts in
    /// `play->csCtx.segment`.
    pub cutscene: Option<usize>,
}

/// Every offset in a scene file that a header command points at, across all its headers (the
/// main one and every alternate), plus the headers themselves and the file's end: the places
/// a list with no stored length has to end before.
pub fn pointer_targets(data: &[u8], segment: u8) -> Vec<usize> {
    let mut out = vec![data.len()];
    let mut headers = vec![0usize];
    if let Ok(main) = commands_at(data, 0)
        && let Some(alt) = main.iter().find(|c| c.code == CMD_ALTERNATE_HEADER_LIST)
        && alt.data2 >> 24 == segment as u32
    {
        let list = (alt.data2 & 0xFF_FFFF) as usize;
        out.push(list);
        let mut o = list;
        while o + 4 <= data.len() && headers.len() < 32 {
            let v = be32(data, o);
            if v != 0 && v >> 24 != segment as u32 {
                break;
            }
            if v != 0 {
                headers.push((v & 0xFF_FFFF) as usize);
            }
            o += 4;
        }
    }
    for &h in &headers {
        out.push(h);
        for c in commands_at(data, h).unwrap_or_default() {
            if c.data2 >> 24 == segment as u32 {
                out.push((c.data2 & 0xFF_FFFF) as usize);
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// How many `size`-byte elements a length-less list at `offset` can have: up to the next
/// pointer target (`pointer_targets`), and only while `valid` holds. The decomp's asset
/// extraction bounds these lists the same way (the next symbol ends a list); the game only
/// ever indexes them with numbers from elsewhere (spawn numbers from the entrance table, exit
/// indices from collision), which the importer's tests check against these lengths.
pub fn list_extent(data: &[u8], offset: usize, size: usize, targets: &[usize], valid: impl Fn(&[u8]) -> bool) -> usize {
    let end = targets.iter().copied().find(|&t| t > offset).unwrap_or(data.len()).min(data.len());
    let mut n = 0;
    while offset + (n + 1) * size <= end && valid(&data[offset + n * size..offset + (n + 1) * size]) {
        n += 1;
    }
    n
}

/// The `Path` list at `offset` (`SCENE_CMD_ID_PATH_LIST`): 8-byte `{ u8 count, pad, Vec3s*
/// points }` entries, as many as [`list_extent`] allows while each has points in this file
/// (segment `segment`, zero padding, the points within the file). The game only indexes it
/// with numbers from actor params.
pub fn path_list(data: &[u8], segment: u8, offset: usize, targets: &[usize]) -> Vec<Path> {
    let points_in_file = |b: &[u8]| {
        let (count, ptr) = (b[0] as usize, be32(b, 4));
        count > 0 && b[1..4] == [0, 0, 0] && ptr >> 24 == segment as u32 && (ptr & 0xFF_FFFF) as usize + count * 6 <= data.len()
    };
    let n = list_extent(data, offset, 8, targets, points_in_file);
    (0..n)
        .map(|i| {
            let b = &data[offset + i * 8..offset + i * 8 + 8];
            let at = (be32(b, 4) & 0xFF_FFFF) as usize;
            let points = (0..b[0] as usize).map(|k| [be16(data, at + k * 6) as i16, be16(data, at + k * 6 + 2) as i16, be16(data, at + k * 6 + 4) as i16]).collect();
            Path { points }
        })
        .collect()
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
        let spawns = find(CMD_SPAWN_LIST).map(|c| actor_entries(&file, local(c.data2), c.data1 as usize)).unwrap_or_default();
        // SCENE_CMD_MISC_SETTINGS(sceneCamType, worldMapLocation): R_SCENE_CAM_TYPE.
        let scene_cam_type = find(CMD_MISC_SETTINGS).map(|c| c.data1).unwrap_or(0);
        let rooms: Vec<RoomRef> = find(CMD_ROOM_LIST)
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
        let targets = pointer_targets(&file, SCENE_SEGMENT);
        let (n_spawns, n_rooms) = (spawns.len(), rooms.len());
        let entrances = find(CMD_ENTRANCE_LIST)
            .map(|c| {
                let o = local(c.data2);
                let n = list_extent(&file, o, 2, &targets, |b| (b[0] as usize) < n_spawns && (b[1] as usize) < n_rooms);
                (0..n).map(|i| EntranceEntry { spawn: file[o + i * 2], room: file[o + i * 2 + 1] }).collect()
            })
            .unwrap_or_default();
        // Entrance indices: gEntranceTable rows, or the ENTR_RETURN_* values (0x7FF9..).
        let exits = find(CMD_EXIT_LIST)
            .map(|c| {
                let o = local(c.data2);
                let n = list_extent(&file, o, 2, &targets, |b| {
                    let v = be16(b, 0);
                    !(0x0700..0x7FF9).contains(&v)
                });
                (0..n).map(|i| be16(&file, o + i * 2)).collect()
            })
            .unwrap_or_default();
        let transition_actors = find(CMD_TRANSITION_ACTOR_LIST)
            .map(|c| {
                let o = local(c.data2);
                (0..c.data1 as usize)
                    .filter_map(|i| {
                        let b = file.get(o + i * 16..o + i * 16 + 16)?;
                        Some(TransitionActorEntry {
                            sides: [(b[0] as i8, b[1] as i8), (b[2] as i8, b[3] as i8)],
                            id: be16(b, 4) as i16,
                            pos: [be16(b, 6) as i16, be16(b, 8) as i16, be16(b, 10) as i16],
                            rot_y: be16(b, 12) as i16,
                            params: be16(b, 14) as i16,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let paths = find(CMD_PATH_LIST).map(|c| path_list(&file, SCENE_SEGMENT, local(c.data2), &targets)).unwrap_or_default();
        // The bg cameras something outside the collision names: a spawn's start camera
        // (`params & 0xFF`, 0xFF for none), a transition actor's sides, and the two a fixed
        // viewpoint scene toggles between (BGCAM_INDEX_TOGGLE_LOCKED / _PIVOT).
        let mut min_cams = if matches!(scene_cam_type, 0x10 | 0x20) { 2 } else { 0 };
        for s in &spawns {
            if s.params as u16 & 0xFF != 0xFF {
                min_cams = min_cams.max((s.params as u16 & 0xFF) as usize + 1);
            }
        }
        for t in &transition_actors {
            let t: &TransitionActorEntry = t;
            for (_, cam) in t.sides {
                if cam >= 0 {
                    min_cams = min_cams.max(cam as usize + 1);
                }
            }
        }
        let collision = CollisionHeader::parse_with_cams(&file, SCENE_SEGMENT, local(col.data2), min_cams)?;
        let cutscene = find(CMD_CUTSCENE_DATA).filter(|c| c.data2 >> 24 == SCENE_SEGMENT as u32).map(|c| local(c.data2));
        Ok(Scene {
            cutscene,
            entrances,
            exits,
            transition_actors,
            paths,
            scene_cam_type,
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
