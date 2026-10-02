//! The game's asset pack: what's in it, where it lives, and typed access to its records.
//!
//! `oot_import` writes the pack from the ROM (`oot import`); the game reads it through
//! `GamePack` and never touches the ROM or the decomp. The record names below are the
//! contract between the two, and `FORMAT_VERSION` / `IMPORTER_VERSION` say which records and
//! layouts a pack has: a pack with other versions is stale and gets rebuilt
//! (docs/adr/0008-asset-pack.md).
//!
//! Names are `kind/…` with the decomp's file and symbol names: `tex/<file>/<symbol>`,
//! `mesh/<file>/<symbol>`, `skel/…`, `anim/…`, `col/…`, plus `table/…` for game tables,
//! `scene/<scene file>`, `room/<scene file>/<layer>/<room>` and `player/<age>/…` for Link.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use eng_anim::anim::{LinkAnimation, StandardAnimation};
use eng_anim::skeleton::Skeleton;
use eng_asset::{Assets, PackHeader};
use eng_collision::collision::CollisionHeader;
use eng_gfx::DrawList;

use crate::data::GameData;
use crate::env::EnvTables;
use crate::player_lib::{Age, LinkFaces, LinkVariant, PlayerRules};
use crate::scene::{RoomData, SceneData, SceneTable};

/// Bumped whenever a record type or the set of records changes.
pub const FORMAT_VERSION: u32 = 15;
/// The importer that writes game packs, and the version of its output.
pub const IMPORTER: &str = "oot_import";
pub const IMPORTER_VERSION: u32 = 1;

/// The per-user folder name, and the environment variables that override where packs are.
pub const APP_DIR: &str = "oot-clone";
/// A pack file or loose folder to use instead of the default one.
pub const ENV_PACK: &str = "OOT_PACK";
/// The data folder to use instead of the per-user one.
pub const ENV_DATA_DIR: &str = "OOT_DATA_DIR";

/// Record names.
pub mod keys {
    use crate::player_lib::Age;

    /// `eng_math::Tables`.
    pub const MATH: &str = "table/math";
    /// `data::GameData` (without its animations).
    pub const PLAYER: &str = "table/player";
    /// `player_lib::PlayerRules`.
    pub const PLAYER_RULES: &str = "table/player_rules";
    /// `env::EnvTables`.
    pub const ENV: &str = "table/env";
    /// `scene::SceneTable`.
    pub const SCENES: &str = "table/scenes";
    /// `actor_table::ActorTable`.
    pub const ACTORS: &str = "table/actors";
    /// `pack::Manifest`: what the import covered.
    pub const MANIFEST: &str = "meta/manifest";
    /// `message::MessageTable`: the English messages.
    pub const MESSAGES: &str = "table/messages";
    /// `item::ItemDropTables`.
    pub const ITEM_DROPS: &str = "table/item_drops";
    /// `interface::InterfaceTables`.
    pub const INTERFACE: &str = "table/interface";
    /// `item::ItemTables`: `sGetItemTable` and `sDrawItemTable`.
    pub const ITEMS: &str = "table/items";
    /// `cutscene::CutsceneTables`: `sEntranceCutsceneTable` and every script's key.
    pub const CUTSCENES: &str = "table/cutscenes";
    /// `elf_message::ElfMessageTables`: Navi's C-Up texts (`sNaviMsgFiles`) and Saria's.
    pub const ELF_MESSAGES: &str = "table/elf_messages";
    /// `eng_audio::AudioTables`: the audio tables, `audio_data.c`'s, the audio specs, the
    /// microcode's resampler filters (docs/adr/0024-audio-data.md).
    pub const AUDIO_TABLES: &str = "audio/tables";
    /// `audio::AudioGameTables`: the game's audio tables (`sSeqFlags`, `sSpecReverbs`,
    /// `sNatureAmbienceDataIO`, `gSoundModeList`; docs/adr/0026-the-games-audio.md).
    pub const AUDIO_GAME: &str = "table/audio";
    /// The ROM files the audio library loads from (`Audiobank`, `Audioseq`, `Audiotable`).
    pub const AUDIO_ROM_FILES: [&str; 3] = ["Audiobank", "Audioseq", "Audiotable"];

    /// `eng_audio::RomFile`: one of `AUDIO_ROM_FILES`.
    pub fn audio_rom(name: &str) -> String {
        format!("audio/rom/{name}")
    }

    /// A texture from the decomp's XMLs (`pack::Texture`).
    pub fn texture(file: &str, symbol: &str) -> String {
        format!("tex/{file}/{symbol}")
    }
    /// A display list, or a skeleton's full mesh, interpreted (`eng_gfx::DrawList`).
    pub fn mesh(file: &str, symbol: &str) -> String {
        format!("mesh/{file}/{symbol}")
    }
    /// `eng_anim::skeleton::Skeleton`.
    pub fn skeleton(file: &str, symbol: &str) -> String {
        format!("skel/{file}/{symbol}")
    }
    /// `skin::SkinSkeleton`: a skeleton of skin limbs (the horses').
    pub fn skin(file: &str, symbol: &str) -> String {
        format!("skin/{file}/{symbol}")
    }
    /// `StandardAnimation`, or `LinkAnimation` for `gameplay_keep`'s `gPlayerAnim_*`.
    pub fn anim(file: &str, symbol: &str) -> String {
        format!("anim/{file}/{symbol}")
    }
    /// `eng_collision::collision::CollisionHeader`.
    pub fn collision(file: &str, symbol: &str) -> String {
        format!("col/{file}/{symbol}")
    }
    /// `scene::SceneData`.
    pub fn scene(file: &str) -> String {
        format!("scene/{file}")
    }
    /// A scene layer's collision (`CollisionHeader`).
    pub fn scene_collision(file: &str, layer: usize) -> String {
        format!("col/{file}/layer{layer}")
    }
    /// `scene::RoomData`.
    pub fn room(file: &str, layer: usize, room: usize) -> String {
        format!("room/{file}/{layer}/{room}")
    }
    /// `player_lib::LinkVariant`: Link for `age` with these hand, sheath and waist lists
    /// (`PlayerRules::limb_dlists`, the lists a loadout draws; `-` for none).
    pub fn link_variant(age: Age, limbs: &[(u8, Option<String>)]) -> String {
        let names: Vec<&str> = limbs.iter().map(|(_, n)| n.as_deref().unwrap_or("-")).collect();
        format!("player/{}/{}", age.name(), names.join("+"))
    }
    /// `player_lib::LinkFaces`.
    pub fn link_faces(age: Age) -> String {
        format!("player/{}/faces", age.name())
    }
    /// `cutscene::CutsceneScript`: a scene's or an overlay's script, e.g.
    /// `cutscene/ovl_Bg_Treemouth/D_808BCE20` (docs/adr/0022-cutscenes.md).
    pub fn cutscene(file: &str, symbol: &str) -> String {
        format!("cutscene/{file}/{symbol}")
    }
    /// A scene layer's script that no XML names, by its offset in the scene file, e.g.
    /// `cutscene/spot04_scene/0xA6D0` (docs/adr/0023-navi-and-the-opening.md).
    pub fn cutscene_at(file: &str, offset: usize) -> String {
        format!("cutscene/{file}/{}", cutscene_offset_name(offset))
    }
    /// The name a script found by its offset goes by (`0xA6D0`).
    pub fn cutscene_offset_name(offset: usize) -> String {
        format!("0x{offset:X}")
    }
    /// An actor's baked mesh (`MeshBake`), e.g. `bake/En_Ko/km1_opa`.
    pub fn bake(name: &str) -> String {
        format!("bake/{name}")
    }
}

/// A mesh an actor draws that the pack's plain meshes don't give as they are
/// (docs/adr/0012-actor-bakes.md): several display lists in a row, a skeleton with a limb's list
/// replaced, or lists that read segments the draw code binds (a texture, a colour it sets each
/// frame, a render mode). The content crate lists them (`oot_actors::bakes()`); the importer
/// builds each into `keys::bake(name)`, an `eng_gfx::DrawList` like any mesh.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshBake {
    pub name: String,
    /// The object on segment 6 (the keeps are on 4 and 5 as for every mesh).
    pub object: String,
    /// What the draw code binds to segments.
    pub segments: Vec<(u8, BakeSegment)>,
    /// Segments the draw code's own commands are in, run in this order after
    /// `Gfx_SetupDL_25Opa` and before the body (`gDPSetEnvColor` before `SkelAnime_DrawFlex`).
    pub prelude: Vec<u8>,
    pub body: BakeBody,
}

/// An animation in an object without a skeleton (an animation-only object such as
/// `object_opening_demo1`), with the skeleton the actor plays it on: its joint count comes from
/// that skeleton. The content crate lists them (`oot_actors::foreign_anims()`); the importer
/// decodes each into `keys::anim(anim_file, anim)`.
#[derive(Debug, Clone, PartialEq)]
pub struct ForeignAnim {
    pub anim_file: String,
    pub anim: String,
    pub skel_file: String,
    pub skel: String,
}

/// What a segment holds for a bake.
#[derive(Debug, Clone, PartialEq)]
pub enum BakeSegment {
    /// A texture from the XMLs: `gSPSegment(seg, SEGMENTED_TO_VIRTUAL(tex))` (eyes).
    Texture { file: String, symbol: String },
    /// A colour the draw sets every frame (`gsDPSetEnvColor` and/or `gsDPSetPrimColor`), marked
    /// dynamic: the materials that take it read the draw's `SegmentValues` (`env[seg]`,
    /// `prim[seg]`). The bake uses black, alpha 255.
    DynamicColor { env: bool, prim: bool },
    /// Fixed commands (`gsDPSetRenderMode`, ...).
    Commands(Vec<(u32, u32)>),
    /// Commands the draw builds every frame (`Gfx_TwoTexScroll`'s tile sizes), baked with these
    /// (the first frame's) and marked dynamic: the materials whose tile sizes or colours they
    /// set read the draw's `SegmentValues` instead.
    Dynamic(Vec<(u32, u32)>),
    /// A whole ROM file (a skybox's `vr_*_static` textures and palettes).
    File(String),
    /// Data the draw code builds (a skybox's `roomVtx`).
    Bytes(Vec<u8>),
}

/// What a bake draws.
#[derive(Debug, Clone, PartialEq)]
pub enum BakeBody {
    /// Display lists `(file, symbol)` run in order.
    DLists(Vec<(String, String)>),
    /// A skeleton's mesh as `SkelAnime_DrawFlex*` draws it, with limbs' lists replaced
    /// (`OverrideLimbDraw` setting `*dList`).
    Skeleton { file: String, symbol: String, limbs: Vec<LimbOverride> },
    /// A skin skeleton's mesh as `Skin_DrawImpl` draws it: normal limb `i` under bone `i`, and
    /// each animated limb's vertex groups under bones of their own (`crate::skin`).
    Skin { file: String, symbol: String },
}

/// A limb whose list is replaced by `file`'s `symbol`, drawn with `file` on segment 6; an empty
/// `symbol` draws nothing (`*dList = NULL`).
#[derive(Debug, Clone, PartialEq)]
pub struct LimbOverride {
    /// The limb's index in the skeleton (0-based: `OverrideLimbDraw`'s `limbIndex - 1`).
    pub limb: u8,
    pub file: String,
    pub symbol: String,
}

/// A texture from the decomp's XMLs: decoded, and its N64 data for effects that need it.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Texture {
    pub fmt: u8,
    pub siz: u8,
    pub width: u32,
    pub height: u32,
    /// RGBA8, `width * height * 4` bytes.
    pub rgba: Vec<u8>,
    /// The texels as stored in the ROM.
    pub texels: Vec<u8>,
    /// The palette (RGBA16) for CI textures, when the XML names one.
    pub tlut: Option<Vec<u8>>,
    /// CI texture whose palette is set by code: decoded against a grey ramp of the indices.
    pub palette_from_code: bool,
}

/// What an import covered: per kind of asset, how many the decomp's XMLs list, how many are
/// in the pack, and why the rest aren't.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Manifest {
    /// Kind → (listed, imported).
    pub counts: std::collections::BTreeMap<String, (usize, usize)>,
    /// Kind → reason → how many.
    pub skipped: std::collections::BTreeMap<String, std::collections::BTreeMap<String, usize>>,
    /// Scene statistics that are compared with `ootx scan-scenes`.
    pub scenes: SceneCounts,
    pub notes: Vec<String>,
    /// Seconds per import phase.
    pub timings: Vec<(String, f64)>,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SceneCounts {
    pub scenes: usize,
    /// Distinct headers over layers 0..3.
    pub headers: usize,
    /// Rooms and shape entries of the distinct headers.
    pub rooms: usize,
    pub entries: usize,
    /// Triangles of the main headers (layer 0), and of the distinct headers.
    pub main_triangles: usize,
    pub triangles: usize,
    pub unknown_opcodes: usize,
    /// Unresolved segment references, `"<scene> <address>"`.
    pub unresolved: Vec<String>,
}

/// The data folder: `$OOT_DATA_DIR`, else the per-user one (`eng_asset::user_data_dir`).
pub fn data_dir() -> Result<PathBuf> {
    if let Some(d) = std::env::var_os(ENV_DATA_DIR).filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(d));
    }
    eng_asset::user_data_dir(APP_DIR).context("no per-user data folder (set OOT_DATA_DIR)")
}

pub fn packs_dir() -> Result<PathBuf> {
    Ok(data_dir()?.join("packs"))
}

/// Where the pack for a ROM with this SHA-1 goes.
pub fn pack_path(rom_sha1: &str) -> Result<PathBuf> {
    Ok(packs_dir()?.join(format!("{}.pak", rom_sha1.to_ascii_lowercase())))
}

/// The file naming the ROM of the pack to use (written by the last import).
fn default_marker() -> Result<PathBuf> {
    Ok(packs_dir()?.join("default"))
}

/// Records `rom_sha1`'s pack as the one to use.
pub fn set_default(rom_sha1: &str) -> Result<()> {
    let m = default_marker()?;
    if let Some(d) = m.parent() {
        std::fs::create_dir_all(d)?;
    }
    std::fs::write(&m, rom_sha1.to_ascii_lowercase())?;
    Ok(())
}

/// The pack to use: `$OOT_PACK`, else the one the last import made the default.
pub fn default_pack_path() -> Result<PathBuf> {
    if let Some(p) = std::env::var_os(ENV_PACK).filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(p));
    }
    let m = default_marker()?;
    let sha = std::fs::read_to_string(&m).with_context(|| format!("no default pack ({} missing): run `oot import` first", m.display()))?;
    pack_path(sha.trim())
}

/// True if a pack at `path` exists and was written by this game's importer version, in this
/// format (and from this ROM, if given).
pub fn is_current(path: &Path, rom_sha1: Option<&str>) -> bool {
    let header = if path.is_dir() {
        std::fs::read_to_string(path.join("header.json")).ok().and_then(|t| serde_json::from_str::<PackHeader>(&t).ok())
    } else {
        eng_asset::PackFile::read_header(path).ok()
    };
    header.is_some_and(|h| h.is_current(FORMAT_VERSION, IMPORTER, IMPORTER_VERSION, rom_sha1))
}

/// The game's view of an asset pack.
pub struct GamePack {
    pub assets: Assets,
}

impl GamePack {
    /// Opens a pack file or loose folder, and checks it's current.
    pub fn open(path: &Path) -> Result<GamePack> {
        let assets = Assets::open(path)?;
        let h = assets.header();
        if !h.is_current(FORMAT_VERSION, IMPORTER, IMPORTER_VERSION, None) {
            bail!(
                "{} is stale (format {}, {} {}; this game reads format {FORMAT_VERSION}, {IMPORTER} {IMPORTER_VERSION}): run `oot import` again",
                path.display(),
                h.format_version,
                h.importer,
                h.importer_version
            );
        }
        Ok(GamePack { assets })
    }

    /// Opens `default_pack_path()`.
    pub fn open_default() -> Result<GamePack> {
        Self::open(&default_pack_path()?)
    }

    pub fn header(&self) -> &PackHeader {
        self.assets.header()
    }

    /// Player's tables and animations, with the maths tables installed (`eng_math::install`).
    pub fn game_data(&self) -> Result<GameData> {
        let tables: eng_math::Tables = self.assets.get(keys::MATH)?;
        eng_math::install(tables);
        let mut gd: GameData = self.assets.get(keys::PLAYER)?;
        let anims = gd
            .anim_symbols
            .iter()
            .map(|sym| -> Result<crate::data::Anim> {
                let a: LinkAnimation = self.assets.get(&keys::anim("gameplay_keep", sym))?;
                let short = sym.strip_prefix("gPlayerAnim_").unwrap_or(sym);
                Ok(crate::data::Anim { name: short.to_string(), frames: a.frames })
            })
            .collect::<Result<Vec<_>>>()?;
        gd.set_anims(anims);
        Ok(gd)
    }

    pub fn env_tables(&self) -> Result<EnvTables> {
        self.assets.get(keys::ENV)
    }

    pub fn player_rules(&self) -> Result<PlayerRules> {
        self.assets.get(keys::PLAYER_RULES)
    }

    pub fn scene_table(&self) -> Result<SceneTable> {
        self.assets.get(keys::SCENES)
    }

    pub fn actor_table(&self) -> Result<crate::actor_table::ActorTable> {
        self.assets.get(keys::ACTORS)
    }

    pub fn manifest(&self) -> Result<Manifest> {
        self.assets.get(keys::MANIFEST)
    }

    pub fn messages(&self) -> Result<crate::message::MessageTable> {
        self.assets.get(keys::MESSAGES)
    }

    pub fn item_drops(&self) -> Result<crate::item::ItemDropTables> {
        self.assets.get(keys::ITEM_DROPS)
    }

    pub fn items(&self) -> Result<crate::item::ItemTables> {
        self.assets.get(keys::ITEMS)
    }

    pub fn interface(&self) -> Result<crate::interface::InterfaceTables> {
        self.assets.get(keys::INTERFACE)
    }

    /// A scene by name: `spot04` or `spot04_scene`.
    pub fn scene(&self, name: &str) -> Result<SceneData> {
        let file = if name.ends_with("_scene") { name.to_string() } else { format!("{name}_scene") };
        self.assets.get(&keys::scene(&file))
    }

    pub fn room(&self, key: &str) -> Result<RoomData> {
        self.assets.get(key)
    }

    pub fn collision(&self, key: &str) -> Result<CollisionHeader> {
        self.assets.get(key)
    }

    pub fn mesh(&self, file: &str, symbol: &str) -> Result<DrawList> {
        self.assets.get(&keys::mesh(file, symbol))
    }

    pub fn skin_skeleton(&self, file: &str, symbol: &str) -> Result<crate::skin::SkinSkeleton> {
        self.assets.get(&keys::skin(file, symbol))
    }

    pub fn skeleton(&self, file: &str, symbol: &str) -> Result<Skeleton> {
        self.assets.get(&keys::skeleton(file, symbol))
    }

    pub fn standard_animation(&self, file: &str, symbol: &str) -> Result<StandardAnimation> {
        self.assets.get(&keys::anim(file, symbol))
    }

    pub fn texture(&self, file: &str, symbol: &str) -> Result<Texture> {
        self.assets.get(&keys::texture(file, symbol))
    }

    /// Link's skeleton for `age` (`gLinkAdultSkel` / `gLinkChildSkel`).
    pub fn link_skeleton(&self, age: Age) -> Result<Skeleton> {
        let sym = match age {
            Age::Adult => "gLinkAdultSkel",
            Age::Child => "gLinkChildSkel",
        };
        self.skeleton(age.object(), sym)
    }

    pub fn link_variant(&self, age: Age, limbs: &[(u8, Option<String>)]) -> Result<LinkVariant> {
        self.assets.get(&keys::link_variant(age, limbs))
    }

    /// Every Link variant's record name for `age`.
    pub fn link_variant_names(&self, age: Age) -> Vec<String> {
        self.assets.names(&format!("player/{}/", age.name())).into_iter().filter(|n| n != &keys::link_faces(age)).collect()
    }

    pub fn link_faces(&self, age: Age) -> Result<LinkFaces> {
        self.assets.get(&keys::link_faces(age))
    }

    pub fn cutscene_tables(&self) -> Result<crate::cutscene::CutsceneTables> {
        self.assets.get(keys::CUTSCENES)
    }

    /// `elf_message::ElfMessageTables`.
    pub fn elf_messages(&self) -> Result<crate::elf_message::ElfMessageTables> {
        self.assets.get(keys::ELF_MESSAGES)
    }

    /// A cutscene script by its pack key.
    pub fn cutscene(&self, key: &str) -> Result<crate::cutscene::CutsceneScript> {
        self.assets.get(key)
    }

    /// The audio library's tables, without the ROM files.
    pub fn audio_tables(&self) -> Result<eng_audio::AudioTables> {
        self.assets.get(keys::AUDIO_TABLES)
    }

    /// The game's audio tables.
    pub fn audio_game_tables(&self) -> Result<crate::audio::AudioGameTables> {
        self.assets.get(keys::AUDIO_GAME)
    }

    /// The audio library's data: its tables and the three ROM files.
    pub fn audio_data(&self) -> Result<eng_audio::AudioData> {
        let rom = |n: &str| -> Result<eng_audio::RomFile> { self.assets.get(&keys::audio_rom(n)) };
        Ok(eng_audio::AudioData { tables: self.assets.get(keys::AUDIO_TABLES)?, audiobank: rom("Audiobank")?, audioseq: rom("Audioseq")?, audiotable: rom("Audiotable")? })
    }
}
