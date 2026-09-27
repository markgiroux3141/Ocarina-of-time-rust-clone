//! Scene header commands (`z64scene.h`): 8-byte `{ u8 code, u8 data1, u16 pad, u32 data2 }`
//! entries ending at `SCENE_CMD_ID_END`. Only what the movement spike needs is decoded: the
//! collision header and the player spawn points.

use anyhow::{Result, bail};

use crate::collision::CollisionHeader;
use crate::rom::Rom;

pub const CMD_SPAWN_LIST: u8 = 0x00;
pub const CMD_COLLISION_HEADER: u8 = 0x03;
pub const CMD_END: u8 = 0x14;
/// Scene files are mapped to segment 2 while loaded.
pub const SCENE_SEGMENT: u8 = 0x02;

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

fn be16(b: &[u8], o: usize) -> u16 {
    u16::from_be_bytes([b[o], b[o + 1]])
}
fn be32(b: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

/// Reads the main header's commands at the start of a scene file.
pub fn commands(scene: &[u8]) -> Result<Vec<SceneCommand>> {
    let mut out = Vec::new();
    for i in 0..64 {
        let o = i * 8;
        if o + 8 > scene.len() {
            bail!("scene header runs past the file");
        }
        let cmd = SceneCommand { code: scene[o], data1: scene[o + 1], data2: be32(scene, o + 4) };
        if cmd.code == CMD_END {
            return Ok(out);
        }
        out.push(cmd);
    }
    bail!("no SCENE_CMD_ID_END in the first 64 commands")
}

pub struct Scene {
    pub name: String,
    pub collision: CollisionHeader,
    pub spawns: Vec<ActorEntry>,
}

impl Scene {
    pub fn parse(name: &str, file: &[u8]) -> Result<Scene> {
        let cmds = commands(file)?;
        let local = |a: u32| (a & 0xFF_FFFF) as usize;
        let col = cmds
            .iter()
            .find(|c| c.code == CMD_COLLISION_HEADER)
            .ok_or_else(|| anyhow::anyhow!("{name}: no collision header command"))?;
        let collision = CollisionHeader::parse(file, SCENE_SEGMENT, local(col.data2))?;
        let spawns = cmds
            .iter()
            .find(|c| c.code == CMD_SPAWN_LIST)
            .map(|c| {
                (0..c.data1 as usize)
                    .filter_map(|i| {
                        let o = local(c.data2) + i * 16;
                        (o + 16 <= file.len()).then(|| ActorEntry {
                            id: be16(file, o) as i16,
                            pos: [be16(file, o + 2) as i16, be16(file, o + 4) as i16, be16(file, o + 6) as i16],
                            rot: [be16(file, o + 8) as i16, be16(file, o + 10) as i16, be16(file, o + 12) as i16],
                            params: be16(file, o + 14) as i16,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(Scene { name: name.to_string(), collision, spawns })
    }

    /// Loads `<name>_scene` (or `name` if it already ends in `_scene`) from the ROM.
    pub fn load(rom: &Rom, name: &str) -> Result<Scene> {
        let file_name = if name.ends_with("_scene") { name.to_string() } else { format!("{name}_scene") };
        let data = rom.file_by_name(&file_name)?;
        Scene::parse(&file_name, &data)
    }
}
