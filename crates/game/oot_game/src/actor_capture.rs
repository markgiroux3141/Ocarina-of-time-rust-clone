//! The game editor's captures of what each actor draws (docs/EDITOR-ACTORS.md, "Models in the
//! views"; ADR 0078): the game spawns an actor in each configuration the editor offers, runs its
//! init and a few frames, and keeps what its own draw code submitted that frame, flattened into one
//! mesh in the actor's space (`eng_gfx::flatten`). The editor draws them in its 3D view and its
//! palette's thumbnails with the game's renderer. Derived from the ROM: written under
//! `out\editor\actors\`, never committed.
//!
//! The layout of the folder: `manifest.json` (every configuration tried, and what came of it),
//! and a file per actor, `<overlay>.bin` (`ActorCaptures`, as a pack stores a record). Each file
//! carries the stamp it was made under (`Stamp`): the pack it read, and when the actor's module was
//! last changed, so a capture is made again when either changes.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Bumped when a capture's content or layout changes: every capture is made again.
pub const FORMAT: u32 = 1;

/// The manifest's file name in the captures' folder.
pub const MANIFEST: &str = "manifest.json";

/// The requests file the editor writes for configurations of its own (a placed actor's params
/// that the game doesn't place): a JSON list of `CaptureKey`.
pub const EXTRA: &str = "extra.json";

/// How many of an actor's game configurations are captured, most placed first (the palette's
/// "New ..." list shows as many).
pub const VARIANTS: usize = 24;

/// What a capture was made from: the pack, the game that made it and the actor's module (each
/// one's last change, in seconds since the epoch; 0 when it isn't there), and the format. Any of
/// them changed, the actor's captures are made again.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stamp {
    pub format: u32,
    pub pack: String,
    pub game: u64,
    pub module: u64,
}

/// One configuration of an actor: its overlay's name, its params and the rotation components it
/// reads as settings (`ActorSchema::setting_rot`; the rest 0).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CaptureKey {
    pub actor: String,
    pub params: u16,
    pub rot: [i16; 3],
}

/// One configuration's capture: what the actor drew, in its own space (its position at the
/// origin, facing its y rotation 0, the game's axes: y up, +z the way yaw 0 faces), and the
/// mesh's bounds there.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capture {
    pub params: u16,
    pub rot: [i16; 3],
    pub mesh: eng_gfx::DrawList,
    pub lo: [f32; 3],
    pub hi: [f32; 3],
}

/// An actor's file: its captures, and the stamp they were made under.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorCaptures {
    pub actor: String,
    pub stamp: Stamp,
    pub captures: Vec<Capture>,
}

/// What came of a configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Outcome {
    /// Drawn and kept.
    Captured,
    /// It ran, but drew nothing in any setting tried (an invisible actor, or one that hides
    /// outside its scene, time, age or flags).
    Nothing,
    /// It wouldn't spawn (no object), or its code stopped (a panic).
    Failed,
}

/// One configuration in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub actor: String,
    pub params: u16,
    pub rot: [i16; 3],
    pub outcome: Outcome,
    /// The setting it was captured in ("child, day", "as in Kakariko Village, adult, night") or
    /// why it failed.
    pub note: String,
    /// The game frame it was taken on, after its spawn.
    pub frame: u32,
    pub triangles: usize,
    /// Its size: the capture's box, across x, up y and along z (game units).
    #[serde(default)]
    pub size: [f32; 3],
}

/// The lights the captures were made under, in the game's axes: what the editor lights them
/// with (the environment's ambient and two directional lights, `lightCtx`'s for the setting).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CaptureLight {
    pub ambient: [u8; 3],
    pub dirs: [[i8; 3]; 2],
    pub colors: [[u8; 3]; 2],
}

/// The folder's manifest.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Manifest {
    pub format: u32,
    pub pack: String,
    pub light: CaptureLight,
    pub entries: Vec<ManifestEntry>,
}

impl Manifest {
    pub fn read(dir: &Path) -> Option<Manifest> {
        let m: Manifest = serde_json::from_slice(&std::fs::read(dir.join(MANIFEST)).ok()?).ok()?;
        (m.format == FORMAT).then_some(m)
    }

    pub fn write(&self, dir: &Path) -> anyhow::Result<()> {
        eng_asset::write_file_atomic(&dir.join(MANIFEST), serde_json::to_string_pretty(self)?.as_bytes())
    }
}

/// An actor's capture file in `dir`.
pub fn actor_file(dir: &Path, actor: &str) -> PathBuf {
    dir.join(format!("{actor}.bin"))
}

impl ActorCaptures {
    pub fn read(dir: &Path, actor: &str) -> anyhow::Result<ActorCaptures> {
        eng_asset::decode_compressed(&std::fs::read(actor_file(dir, actor))?)
    }

    pub fn write(&self, dir: &Path) -> anyhow::Result<()> {
        eng_asset::write_file_atomic(&actor_file(dir, &self.actor), &eng_asset::encode_compressed(self)?)
    }

    pub fn get(&self, params: u16, rot: [i16; 3]) -> Option<&Capture> {
        self.captures.iter().find(|c| c.params == params && c.rot == rot)
    }
}

/// A pack's id for the stamps: its format, importer version, ROM and when it was imported.
pub fn pack_id(h: &eng_asset::PackHeader) -> String {
    let at = h.info.get("imported_at_unix").map_or("", String::as_str);
    format!("{}.{}.{}.{at}", h.format_version, h.importer_version, &h.source_sha1[..h.source_sha1.len().min(12)])
}

/// An actor's module in the source tree under `root` (`crates/game/oot_actors/src/<name>.rs`, or
/// its folder's `mod.rs`).
pub fn module_path(root: &Path, actor: &str) -> Option<PathBuf> {
    let dir = root.join("crates/game/oot_actors/src");
    let m = if actor == "Player" { "player".to_string() } else { actor.to_lowercase() };
    [dir.join(format!("{m}.rs")), dir.join(&m).join("mod.rs")].into_iter().find(|p| p.exists())
}

/// When a file was last changed, in seconds since the epoch; 0 if it isn't there.
pub fn file_stamp(p: &Path) -> u64 {
    std::fs::metadata(p).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs())
}

/// When an actor's module (its file, or every file of its folder) was last changed, in seconds
/// since the epoch; 0 without the source.
pub fn module_stamp(root: &Path, actor: &str) -> u64 {
    let Some(p) = module_path(root, actor) else { return 0 };
    let secs = |p: &Path| file_stamp(p);
    if p.ends_with("mod.rs") {
        let dir = p.parent().unwrap();
        std::fs::read_dir(dir).map(|rd| rd.flatten().map(|e| secs(&e.path())).max().unwrap_or(0)).unwrap_or(0)
    } else {
        secs(&p)
    }
}
