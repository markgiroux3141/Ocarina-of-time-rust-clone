//! The importer: builds the game's asset pack (docs/adr/0008-asset-pack.md) from the ROM and
//! the decomp, so the game never reads either.
//!
//! 1. **Tables** (`table/*`): the maths tables, Player's constants and animations
//!    (`GameData`, with each animation its own `anim/gameplay_keep/<symbol>` record), Player's
//!    draw rules, the environment's light configs and the scene table, all read from the C by
//!    `crate::tables`, `crate::player` and `crate::room`; and the English messages from the
//!    ROM (`crate::text`).
//! 2. **Objects**: every texture, skeleton (with its full mesh), standard animation, standalone
//!    display list and collision header the XMLs name, decoded or interpreted
//!    (`crate::objects`).
//! 3. **Link**: one mesh per age, model group and hand state, and the eye/mouth textures
//!    (`oot_game::player_lib::LinkVariant`); checked against interpreting with each face bound.
//! 4. **Scenes**: every scene of the scene table, for each of the four game layers: its header,
//!    collision and rooms, with every room entry interpreted for that layer's state.
//! 5. **Manifest**: what the XMLs list against what the pack holds, per kind, with the reason
//!    for everything left out, and the timings.
//!
//! Files are processed on several threads; identical records are stored once.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use anyhow::{Context, Result};
use eng_asset::{PackHeader, PackStats, PackWriter};
use eng_collision::collision::CollisionHeader;
use eng_math::Tables;
use oot_game::data::GameData;
use oot_game::env::{EnvTables, clock_time, scene_times};
use oot_game::pack::{FORMAT_VERSION, IMPORTER, IMPORTER_VERSION, Manifest, SceneCounts, Texture, keys};
use oot_game::player_lib::{Age, LinkFaces, LinkVariant, Loadout, PlayerRules};
use oot_game::scene::{EntryMesh, GAME_LAYERS, LayerData, RoomData, SceneData, SceneEntry, SceneTable};

use crate::drawcfg;
use crate::objects::{self, Files, ObjectSegments, Palette};
use crate::player::{LoadPlayerRules, PlayerModel, player_animations};
use crate::project::Project;
use crate::room::{SceneDraw, SceneTables};
use crate::symbols::AssetFile;
use crate::tables::{LoadActorTable, LoadEnvTables, LoadGameData, LoadMathTables};
use oot_game::actor_table::ActorTable;

/// Where to write the pack.
#[derive(Debug, Clone)]
pub enum Output {
    /// One pack file.
    Pack(PathBuf),
    /// A folder of loose records (the dev mode).
    Loose(PathBuf),
}

impl Output {
    pub fn path(&self) -> &Path {
        match self {
            Output::Pack(p) | Output::Loose(p) => p,
        }
    }
}

pub struct ImportReport {
    pub manifest: Manifest,
    pub stats: PackStats,
    pub rom_sha1: String,
    pub seconds: f64,
}

/// SHA-1 of the ROM file as given (the pack is keyed by it).
pub fn rom_sha1(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(sha1_smol::Sha1::from(&bytes).digest().to_string())
}

/// The decomp checkout's commit: `.git/HEAD`, following a ref through the ref files or
/// `packed-refs`.
pub fn decomp_commit(decomp: &Path) -> Option<String> {
    let git = decomp.join(".git");
    let head = std::fs::read_to_string(git.join("HEAD")).ok()?;
    let head = head.trim();
    let Some(r) = head.strip_prefix("ref: ") else { return Some(head.to_string()) };
    if let Ok(h) = std::fs::read_to_string(git.join(r)) {
        return Some(h.trim().to_string());
    }
    let packed = std::fs::read_to_string(git.join("packed-refs")).ok()?;
    packed.lines().find_map(|l| l.strip_suffix(r).map(|h| h.trim().to_string()))
}

/// Counts per asset kind, merged across threads into the manifest.
#[derive(Default)]
struct Tally {
    listed: BTreeMap<String, usize>,
    imported: BTreeMap<String, usize>,
    skipped: BTreeMap<String, BTreeMap<String, usize>>,
    notes: Vec<String>,
}

impl Tally {
    fn ok(&mut self, kind: &str) {
        *self.imported.entry(kind.to_string()).or_default() += 1;
    }
    fn skip(&mut self, kind: &str, why: impl Into<String>) {
        *self.skipped.entry(kind.to_string()).or_default().entry(why.into()).or_default() += 1;
    }
    fn merge(&mut self, o: Tally) {
        for (k, v) in o.listed {
            *self.listed.entry(k).or_default() += v;
        }
        for (k, v) in o.imported {
            *self.imported.entry(k).or_default() += v;
        }
        for (k, m) in o.skipped {
            let e = self.skipped.entry(k).or_default();
            for (r, n) in m {
                *e.entry(r).or_default() += n;
            }
        }
        self.notes.extend(o.notes);
    }
}

/// Runs `work` for each of `items` on all cores and returns the results in order.
fn parallel<T: Sync, R: Send>(items: &[T], work: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let n = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(items.len().max(1));
    let next = AtomicUsize::new(0);
    let out: Mutex<Vec<Option<R>>> = Mutex::new((0..items.len()).map(|_| None).collect());
    std::thread::scope(|s| {
        for _ in 0..n {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= items.len() {
                        break;
                    }
                    let r = work(&items[i]);
                    out.lock().unwrap()[i] = Some(r);
                }
            });
        }
    });
    out.into_inner().unwrap().into_iter().map(|r| r.unwrap()).collect()
}

/// Imports everything and writes the pack.
pub fn import(p: &Project, out: &Output) -> Result<ImportReport> {
    let t0 = Instant::now();
    let sha = rom_sha1(&p.config.rom)?;
    let mut info = BTreeMap::new();
    info.insert("rom_title".into(), p.rom.title.clone());
    info.insert("rom_game_code".into(), p.rom.game_code.clone());
    info.insert("rom_size".into(), p.rom.data.len().to_string());
    if let Some(c) = decomp_commit(&p.config.decomp) {
        info.insert("decomp_commit".into(), c);
    }
    info.insert("importer_crate".into(), format!("oot_import {}", env!("CARGO_PKG_VERSION")));
    let unix = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    info.insert("imported_at_unix".into(), unix.to_string());
    let w = PackWriter::new(PackHeader {
        format_version: FORMAT_VERSION,
        importer: IMPORTER.into(),
        importer_version: IMPORTER_VERSION,
        source_sha1: sha.clone(),
        info,
    });
    let mut m = Manifest::default();
    let mut tally = Tally::default();
    // Every element the XMLs name, by kind.
    for f in &p.symbols.files {
        for s in &f.symbols {
            *tally.listed.entry(s.kind.clone()).or_default() += 1;
        }
    }

    let t = Instant::now();
    let rules = import_tables(p, &w, &mut tally)?;
    m.timings.push(("tables".into(), t.elapsed().as_secs_f64()));

    let t = Instant::now();
    let segs = ObjectSegments::load(p);
    let files = Files::new(p);
    let per_file = parallel(&p.symbols.files, |f| import_file(f, &segs, &files, &w));
    for r in per_file {
        tally.merge(r?);
    }
    m.timings.push(("objects".into(), t.elapsed().as_secs_f64()));

    let t = Instant::now();
    import_bakes(p, &segs, &files, &w, &mut tally)?;
    m.timings.push(("bakes".into(), t.elapsed().as_secs_f64()));

    let t = Instant::now();
    import_link(p, &rules, &w, &mut tally)?;
    m.timings.push(("link".into(), t.elapsed().as_secs_f64()));

    let t = Instant::now();
    m.scenes = import_scenes(p, &w, &mut tally)?;
    m.timings.push(("scenes".into(), t.elapsed().as_secs_f64()));

    let t = Instant::now();
    import_audio(p, &w, &mut tally)?;
    m.timings.push(("audio".into(), t.elapsed().as_secs_f64()));

    for (k, n) in &tally.listed {
        m.counts.insert(k.clone(), (*n, tally.imported.get(k).copied().unwrap_or(0)));
    }
    for (k, n) in &tally.imported {
        m.counts.entry(k.clone()).or_insert((0, *n));
    }
    m.skipped = tally.skipped;
    m.notes = tally.notes;
    m.notes.sort();
    m.notes.dedup();
    let t = Instant::now();
    w.put(keys::MANIFEST, &m)?;
    let stats = match out {
        Output::Pack(path) => w.write(path)?,
        Output::Loose(dir) => w.write_loose(dir)?,
    };
    m.timings.push(("write".into(), t.elapsed().as_secs_f64()));
    Ok(ImportReport { manifest: m, stats, rom_sha1: sha, seconds: t0.elapsed().as_secs_f64() })
}

/// The game tables. Returns Player's draw rules for the Link phase.
fn import_tables(p: &Project, w: &PackWriter, tally: &mut Tally) -> Result<PlayerRules> {
    let decomp = &p.config.decomp;
    w.put(keys::MATH, &Tables::load(decomp)?)?;
    let gd = GameData::load(p).context("GameData")?;
    w.put(keys::PLAYER, &gd)?;
    // Link's animations, each under its gameplay_keep symbol.
    for (name, a) in player_animations(p)? {
        w.put(&keys::anim("gameplay_keep", &name), &a)?;
        tally.ok("PlayerAnimation");
    }
    let rules = PlayerRules::load(decomp).context("PlayerRules")?;
    w.put(keys::PLAYER_RULES, &rules)?;
    w.put(keys::ENV, &EnvTables::load(decomp).context("EnvTables")?)?;
    let st = SceneTables::load(decomp).context("scene tables")?;
    let table = SceneTable {
        scenes: st
            .scenes
            .iter()
            .map(|s| SceneEntry { id: s.id as u16, file: s.file.clone(), enum_name: s.enum_name.clone(), draw_config: s.draw_config.clone() })
            .collect(),
        objects: st.objects.clone(),
        entrances: st.entrances.clone(),
        room_skyboxes: st.room_skyboxes.clone(),
    };
    w.put(keys::SCENES, &table)?;
    w.put(keys::ACTORS, &ActorTable::load(decomp).context("the actor table")?)?;
    w.put(keys::MESSAGES, &crate::text::load_messages(p).context("the message table")?)?;
    w.put(keys::ITEM_DROPS, &crate::tables::load_item_drops(decomp).context("the item drop tables")?)?;
    w.put(keys::INTERFACE, &crate::tables::load_interface(decomp, &st).context("the interface tables")?)?;
    w.put(keys::ITEMS, &crate::tables::load_items(decomp).context("the item tables")?)?;
    import_cutscenes(p, &st, w, tally).context("the cutscenes")?;
    w.put(keys::ELF_MESSAGES, &crate::elf_message::load(decomp, &p.rom).context("the C-Up texts")?)?;
    Ok(rules)
}

/// The audio library's data (docs/adr/0024-audio-data.md): its tables, and the ROM's three
/// audio files as they are.
fn import_audio(p: &Project, w: &PackWriter, tally: &mut Tally) -> Result<()> {
    use crate::audio::LoadAudioData;
    let a = eng_audio::AudioData::load(p).context("the audio data")?;
    w.put(keys::AUDIO_TABLES, &a.tables)?;
    let game = crate::audio::audio_game_tables(&p.config.decomp).context("the game's audio tables")?;
    w.put(keys::AUDIO_GAME, &game)?;
    tally.notes.push(format!(
        "audio: {} sequence flags, {} spec reverbs, {} nature ambiences, {} sound modes",
        game.seq_flags.len(),
        game.spec_reverbs.len(),
        game.nature_ambience.len(),
        game.sound_mode_list.len()
    ));
    for f in [&a.audiobank, &a.audioseq, &a.audiotable] {
        w.put(&keys::audio_rom(&f.name), f)?;
        tally.ok("AudioRomFile");
    }
    let fonts = eng_audio::context::AudioTable::parse(&a.tables.sound_font_table).entries.len();
    let seqs = eng_audio::context::AudioTable::parse(&a.tables.sequence_table).entries.len();
    let banks = eng_audio::context::AudioTable::parse(&a.tables.sample_bank_table).entries.len();
    tally.notes.push(format!("audio: {fonts} fonts, {seqs} sequences, {banks} sample banks, {} audio specs", a.tables.specs.len()));
    Ok(())
}

/// The overlays' cutscene scripts, and `CutsceneTables`: every script's key (the scene ones
/// are written with their files' other symbols) and `sEntranceCutsceneTable`.
fn import_cutscenes(p: &Project, st: &SceneTables, w: &PackWriter, tally: &mut Tally) -> Result<()> {
    let mut scripts = Vec::new();
    for f in &p.symbols.files {
        if matches!(f.segment, Some(2)) && p.rom.index_of(&f.name).is_some() {
            for s in f.of_kind("Cutscene") {
                scripts.push((s.name.clone(), keys::cutscene(&f.name, &s.name)));
            }
        }
    }
    for s in crate::cutscene::overlay_scripts(&p.config.decomp, &p.rom)? {
        let key = keys::cutscene(&s.file, &s.name);
        anyhow::ensure!(!scripts.iter().any(|(n, _)| *n == s.name), "two scripts named {}", s.name);
        scripts.push((s.name.clone(), key.clone()));
        w.put(&key, &s)?;
        tally.ok("OverlayCutscene");
    }
    // The scene layers' scripts no XML names (the cutscene layers': docs/adr/0023), by offset.
    for def in &st.scenes {
        let Ok(data) = p.rom.file_by_name(&def.file) else { continue };
        let mut seen = BTreeSet::new();
        for layer in 0..crate::scene::layer_count(&data) {
            let Ok((_, cmds)) = crate::scene::layer_commands(&data, crate::scene::SCENE_SEGMENT, layer) else { continue };
            let Some(c) = cmds.iter().find(|c| c.code == crate::scene::CMD_CUTSCENE_DATA && c.data2 >> 24 == crate::scene::SCENE_SEGMENT as u32) else { continue };
            let off = (c.data2 & 0xFF_FFFF) as usize;
            if xml_cutscene_at(p, &def.file, off).is_some() || !seen.insert(off) {
                continue;
            }
            let bytes = crate::cutscene::scene_script(&data, off).with_context(|| format!("{} layer {layer}: the script at {off:#x}", def.file))?;
            let key = keys::cutscene_at(&def.file, off);
            let name = keys::cutscene_offset_name(off);
            w.put(&key, &oot_game::cutscene::CutsceneScript { file: def.file.clone(), name: name.clone(), data: bytes })?;
            scripts.push((format!("{}/{name}", def.file), key));
            tally.ok("LayerCutscene");
        }
    }
    let entrance_cutscenes = crate::cutscene::entrance_cutscenes(&p.config.decomp, &st.entrances, |n| scripts.iter().find(|(s, _)| s == n).map(|(_, k)| k.clone()))?;
    w.put(keys::CUTSCENES, &oot_game::cutscene::CutsceneTables { entrance_cutscenes, scripts })?;
    Ok(())
}

/// The XML's `<Cutscene>` symbol at `offset` in scene file `file`.
fn xml_cutscene_at(p: &Project, file: &str, offset: usize) -> Option<String> {
    p.symbols.file(file).and_then(|f| f.of_kind("Cutscene").find(|s| s.offset as usize == offset).map(|s| s.name.clone()))
}

/// The pack key of the script at `offset` in scene file `file`: the XML symbol's, or, for the
/// scripts no XML names, the offset's (`keys::cutscene_at`).
pub fn scene_cutscene_key(p: &Project, file: &str, offset: usize) -> String {
    match xml_cutscene_at(p, file, offset) {
        Some(sym) => keys::cutscene(file, &sym),
        None => keys::cutscene_at(file, offset),
    }
}

/// Every texture, collision header, skeleton, animation and standalone display list of one
/// XML file.
fn import_file(f: &AssetFile, segs: &ObjectSegments, files: &Files, w: &PackWriter) -> Result<Tally> {
    let mut t = Tally::default();
    let Some(data) = files.get(&f.name) else {
        for s in &f.symbols {
            t.skip(&s.kind, "file not in the ROM");
        }
        return Ok(t);
    };
    let is_scene = matches!(f.segment, Some(2) | Some(3));
    let is_overlay = f.name.starts_with("ovl_");
    let seg = f.segment.unwrap_or(6);

    for s in f.of_kind("Texture") {
        match objects::decode_texture(f, &data, s, files) {
            Ok(d) => {
                let tex = Texture {
                    fmt: d.fmt,
                    siz: d.siz,
                    width: d.width,
                    height: d.height,
                    rgba: d.image.rgba,
                    texels: d.texels,
                    tlut: d.tlut,
                    palette_from_code: d.palette == Palette::Code,
                };
                w.put(&keys::texture(&f.name, &s.name), &tex)?;
                t.ok("Texture");
            }
            Err(e) => {
                t.skip("Texture", "decode error");
                t.notes.push(format!("{} / {}: {e:#}", f.name, s.name));
            }
        }
    }

    for s in f.of_kind("Collision") {
        use crate::z64::CollisionCodec;
        if is_overlay {
            t.skip("Collision", "in an overlay (relocated code pointers)");
            continue;
        }
        match eng_collision::collision::CollisionHeader::parse(&data, seg, s.offset as usize) {
            Ok(h) => {
                w.put(&keys::collision(&f.name, &s.name), &h)?;
                t.ok("Collision");
            }
            Err(e) => {
                t.skip("Collision", "decode error");
                t.notes.push(format!("{} / {}: {e:#}", f.name, s.name));
            }
        }
    }

    // Display lists that are limbs of this file's skeletons are in the skeletons' meshes.
    let mut limb_dls: BTreeSet<u32> = BTreeSet::new();
    let skeletons: Vec<_> = f.of_kind("Skeleton").collect();
    let mut parsed = Vec::new();
    for s in &skeletons {
        if is_overlay {
            t.skip("Skeleton", "in an overlay (relocated code pointers)");
            continue;
        }
        if let Some(why) = objects::unsupported_limb_type(s) {
            t.skip("Skeleton", why);
            continue;
        }
        match objects::parse_skeleton(&data, seg, s) {
            Ok(sk) => {
                for l in &sk.limbs {
                    limb_dls.extend(l.dlists.iter().copied().filter(|&d| d != 0));
                }
                let mesh = segs.skeleton_mesh(f, &data, &sk)?;
                w.put(&keys::mesh(&f.name, &s.name), &mesh)?;
                w.put(&keys::skeleton(&f.name, &s.name), &sk)?;
                t.ok("Skeleton");
                parsed.push(sk);
            }
            Err(e) => {
                t.skip("Skeleton", "decode error");
                t.notes.push(format!("{} / {}: {e:#}", f.name, s.name));
            }
        }
    }

    for a in f.of_kind("Animation") {
        if is_overlay {
            t.skip("Animation", "in an overlay (relocated code pointers)");
            continue;
        }
        // The joint count: from the layout (the index table right before the header), else the
        // file's skeletons, largest first. Joint j reads the j-th index entry, so decoding for
        // a bigger skeleton gives a superset: a smaller one uses the first joints.
        let mut counts: Vec<usize> = match objects::anim_joint_count(&data, seg, a.offset as usize) {
            Some(j) => vec![j],
            None => parsed.iter().map(|s| s.limbs.len() + 1).collect(),
        };
        counts.sort_unstable_by(|x, y| y.cmp(x));
        counts.dedup();
        if counts.is_empty() {
            t.skip("Animation", "joint count unknown: an animation-only object, the actor that plays it names the skeleton");
            continue;
        }
        let mut done = false;
        let mut last_err = None;
        for (k, &joints) in counts.iter().enumerate() {
            match objects::parse_animation(&data, seg, a, joints - 1) {
                Ok(anim) => {
                    w.put(&keys::anim(&f.name, &a.name), &anim)?;
                    t.ok("Animation");
                    if counts.len() > 1 && k == 0 {
                        t.notes.push(format!("{}: animations decoded for the largest of the file's skeletons ({} joints)", f.name, joints));
                    }
                    done = true;
                    break;
                }
                Err(e) => last_err = Some(e),
            }
        }
        if !done {
            t.skip("Animation", "decode error");
            t.notes.push(format!("{} / {}: {:#}", f.name, a.name, last_err.unwrap()));
        }
    }

    // A scene's cutscene scripts, each through its CS_END (docs/adr/0022-cutscenes.md).
    for s in f.of_kind("Cutscene") {
        if !is_scene {
            t.skip("Cutscene", "not in a scene file");
            continue;
        }
        match crate::cutscene::scene_script(&data, s.offset as usize) {
            Ok(bytes) => {
                w.put(&keys::cutscene(&f.name, &s.name), &oot_game::cutscene::CutsceneScript { file: f.name.clone(), name: s.name.clone(), data: bytes })?;
                t.ok("Cutscene");
            }
            Err(e) => anyhow::bail!("{} / {}: {e:#}", f.name, s.name),
        }
    }

    for d in f.of_kind("DList") {
        if is_scene {
            t.skip("DList", "scene or room display list (in the room meshes)");
            continue;
        }
        if is_overlay {
            t.skip("DList", "in an overlay (relocated code pointers)");
            continue;
        }
        let addr = ((seg as u32) << 24) | d.offset;
        if limb_dls.contains(&addr) {
            t.skip("DList", "limb of a skeleton (in the skeleton's mesh)");
            continue;
        }
        let mesh = segs.dlist_mesh(f, &data, addr);
        if mesh.triangle_count() == 0 {
            // Material-only lists (setup, texture swaps): kept, they're what draw code calls.
            t.notes.push(format!("{} / {}: no triangles", f.name, d.name));
        }
        w.put(&keys::mesh(&f.name, &d.name), &mesh)?;
        t.ok("DList");
    }

    // Kinds the pack doesn't hold (yet), and the ones that live inside other records.
    for s in &f.symbols {
        let why = match s.kind.as_str() {
            "Texture" | "Collision" | "Skeleton" | "Animation" | "DList" | "PlayerAnimation" | "Scene" | "Room" | "Cutscene" => continue,
            "Limb" | "LimbTable" => "stored in its skeleton",
            "PlayerAnimationData" => "stored in its PlayerAnimation",
            "Path" => "in the scene's layers (LayerData.paths)",
            _ => "not used by the game",
        };
        t.skip(&s.kind, why);
    }
    Ok(t)
}

/// The meshes the ported actors need baked (`oot_actors::bakes()`, docs/adr/0012-actor-bakes.md).
/// A bake whose lists leave segments unresolved is an error: it would draw with holes.
fn import_bakes(p: &Project, segs: &ObjectSegments, files: &Files, w: &PackWriter, tally: &mut Tally) -> Result<()> {
    for b in oot_actors::bakes() {
        let d = segs.bake_mesh(p, files, &b).with_context(|| format!("bake {}", b.name))?;
        anyhow::ensure!(d.stats.unresolved_addresses.is_empty(), "bake {}: unresolved {:?}", b.name, d.stats.unresolved_addresses.keys().collect::<Vec<_>>());
        anyhow::ensure!(d.stats.unknown_opcodes.is_empty(), "bake {}: unknown opcodes {:?}", b.name, d.stats.unknown_opcodes);
        w.put(&keys::bake(&b.name), &d)?;
        tally.ok("ActorBake");
    }
    // GetItem_Draw's models (oot_game::draw), from sDrawItemTable: each display list is in the
    // object whose XML names it.
    let items = crate::tables::load_items(&p.config.decomp)?;
    let file_of = |sym: &str| -> Option<String> {
        p.symbols
            .files
            .iter()
            .filter(|f| !f.name.starts_with("ovl_") && !matches!(f.segment, Some(2) | Some(3)))
            .find(|f| f.symbols.iter().any(|s| s.kind == "DList" && s.name == sym))
            .map(|f| f.name.clone())
    };
    for b in oot_game::draw::bakes(&items, &file_of)? {
        let d = segs.bake_mesh(p, files, &b).with_context(|| format!("bake {}", b.name))?;
        anyhow::ensure!(d.stats.unresolved_addresses.is_empty(), "bake {}: unresolved {:?}", b.name, d.stats.unresolved_addresses.keys().collect::<Vec<_>>());
        anyhow::ensure!(d.stats.unknown_opcodes.is_empty(), "bake {}: unknown opcodes {:?}", b.name, d.stats.unknown_opcodes);
        w.put(&keys::bake(&b.name), &d)?;
        tally.ok("GetItemBake");
    }
    // The room skyboxes (oot_game::skybox), drawn around the eye in houses and shops.
    for s in crate::room::load_room_skyboxes(&p.config.decomp)? {
        let b = oot_game::skybox::bake(&s);
        let d = segs.bake_mesh(p, files, &b).with_context(|| format!("bake {}", b.name))?;
        if !d.stats.unresolved_addresses.is_empty() {
            // @bug (game): SKYBOX_HAPPY_MASK_SHOP gets four faces (func_800AEFC8) from files
            // that hold two; the last two read past them.
            tally.notes.push(format!("bake {}: {} addresses past its files (the game reads past them too)", b.name, d.stats.unresolved_addresses.len()));
        }
        anyhow::ensure!(d.stats.unknown_opcodes.is_empty(), "bake {}: unknown opcodes {:?}", b.name, d.stats.unknown_opcodes);
        w.put(&keys::bake(&b.name), &d)?;
        tally.ok("SkyboxBake");
    }
    Ok(())
}

/// Link's meshes and faces for both ages.
fn import_link(p: &Project, rules: &PlayerRules, w: &PackWriter, tally: &mut Tally) -> Result<()> {
    for age in [Age::Adult, Age::Child] {
        let model = PlayerModel::load(p, rules, age)?;
        let default = Loadout::default_for(rules, age);
        let notes = std::cell::RefCell::new(BTreeSet::new());
        let build = |lo: &Loadout, eye: usize, mouth: usize| -> Result<eng_gfx::DrawList> {
            let (d, missing) = model.draw_list(rules, lo, eye, mouth, 0)?;
            if !missing.is_empty() {
                notes.borrow_mut().insert(format!("{} {}: display lists not in the XML: {}", age.name(), rules.model_groups[lo.model_group].name, missing.join(", ")));
            }
            Ok(d)
        };
        // The textures each face index puts in the eye and mouth slots, for a loadout.
        let base = build(&default, 0, 0)?;
        let (eye_slots, mouth_slots) = (LinkVariant::slots_from(&base, 8), LinkVariant::slots_from(&base, 9));
        anyhow::ensure!(!eye_slots.is_empty() && !mouth_slots.is_empty(), "{}: no textures from segments 8/9 in Link's mesh", age.name());
        let faces_of = |lo: &Loadout| -> Result<LinkFaces> {
            let mut faces = LinkFaces { eyes: Vec::new(), mouths: Vec::new() };
            for e in 0..rules.eye_textures.len() {
                let d = build(lo, e, 0)?;
                faces.eyes.push(LinkVariant::slots_from(&d, 8).into_iter().map(|i| d.textures[i].clone()).collect());
            }
            for mo in 0..rules.mouth_textures.len() {
                let d = build(lo, 0, mo)?;
                faces.mouths.push(LinkVariant::slots_from(&d, 9).into_iter().map(|i| d.textures[i].clone()).collect());
            }
            Ok(faces)
        };
        let faces = faces_of(&default)?;
        w.put(&keys::link_faces(age), &faces)?;
        let (last_eye, last_mouth) = (rules.eye_textures.len() - 1, rules.mouth_textures.len() - 1);
        // Every model group, hand state, shield and (the child's) sword on B or not, each set of
        // hand, sheath and waist lists once.
        let mut loadouts = Vec::new();
        for (gi, g) in rules.model_groups.iter().enumerate() {
            for fists in [false, true] {
                for shield in 0..rules.shields.len() {
                    for sword in [true, false] {
                        let lo = Loadout { model_group: gi, moving_fast: fists, shield, child_has_kokiri_sword: sword, ..default.clone() };
                        loadouts.push((g, fists, lo));
                    }
                }
            }
        }
        let mut seen = BTreeSet::new();
        for (g, fists, lo) in loadouts {
            let key = lo.variant_key(rules);
            if !seen.insert(key.clone()) {
                continue;
            }
            {
                let draw = build(&lo, 0, 0)?;
                let mut v = LinkVariant { eye_slots: LinkVariant::slots_from(&draw, 8), mouth_slots: LinkVariant::slots_from(&draw, 9), draw, faces: None };
                // The shared faces fit most variants. Where a face slot also holds texels this
                // loadout's hand or sheath lists left in TMEM, the variant keeps its own.
                if v.with_face(&faces, last_eye, last_mouth) != build(&lo, last_eye, last_mouth)? {
                    v.faces = Some(faces_of(&lo)?);
                    tally.ok("LinkVariantOwnFaces");
                }
                // The face swap must give exactly what interpreting with the face bound gives:
                // every face for the default group, the last eye and mouth for the others.
                let checks: Vec<(usize, usize)> = if g.name == "DEFAULT" && lo.shield == default.shield && lo.child_has_kokiri_sword {
                    (0..=last_eye).flat_map(|e| (0..=last_mouth).map(move |m| (e, m))).collect()
                } else {
                    vec![(last_eye, last_mouth)]
                };
                for (e, mo) in checks {
                    let want = build(&lo, e, mo)?;
                    let got = v.with_face(&faces, e, mo);
                    if got != want {
                        let tex_diff: Vec<usize> = (0..got.textures.len().max(want.textures.len())).filter(|&i| got.textures.get(i) != want.textures.get(i)).collect();
                        anyhow::bail!(
                            "{} {} fists={fists}: the face swap for eye {e} mouth {mo} differs from interpreting it: batches {} vs {} (equal {}), materials {} vs {} (equal {}), textures {} vs {}, differing textures {tex_diff:?} (eye slots {:?}, mouth slots {:?}), stats equal {}",
                            age.name(),
                            g.name,
                            got.batches.len(),
                            want.batches.len(),
                            got.batches == want.batches,
                            got.materials.len(),
                            want.materials.len(),
                            got.materials == want.materials,
                            got.textures.len(),
                            want.textures.len(),
                            v.eye_slots,
                            v.mouth_slots,
                            got.stats == want.stats
                        );
                    }
                }
                w.put(&key, &v)?;
                tally.ok("LinkVariant");
            }
        }
        tally.notes.extend(notes.into_inner());
    }
    Ok(())
}

/// The time the rooms of a layer are baked at: 10:00 (a new save's start) for day layers and the
/// cutscene layers, midnight for night layers. Draw configs that depend on the time within a
/// layer (none of the ported ones for their geometry) are baked at that time.
pub fn bake_time(layer: usize) -> u16 {
    let night = layer == oot_game::scene::LAYER_CHILD_NIGHT || layer == oot_game::scene::LAYER_ADULT_NIGHT;
    clock_time(if night { 0 } else { 10 }, 0) as u16
}

/// The draw config state a layer's rooms are built with, as the game's scene load sets it
/// (`gameplayFrames` 0, `Scene_CommandTimeSettings` applied to the bake time). A cutscene layer
/// (4 and up) says nothing of Link's age or the time: it's baked for a child by day, the new
/// file's opening (docs/adr/0023-navi-and-the-opening.md).
pub fn bake_state(layer: usize, room0_time: Option<[u8; 3]>) -> drawcfg::State {
    let night = layer == oot_game::scene::LAYER_CHILD_NIGHT || layer == oot_game::scene::LAYER_ADULT_NIGHT;
    let child = layer == oot_game::scene::LAYER_CHILD_DAY || layer == oot_game::scene::LAYER_CHILD_NIGHT || layer >= oot_game::scene::SCENE_LAYER_CUTSCENE_FIRST;
    let (day, _, _) = scene_times(bake_time(layer), room0_time);
    drawcfg::State { gameplay_frames: 0, child, night, scene_layer: layer as i64, day_time: day }
}

/// The records of one layer of a scene: its `LayerData` and its rooms.
pub fn layer_records(p: &Project, tables: &SceneTables, file: &str, layer: usize) -> Result<(LayerData, Vec<RoomData>, CollisionHeader)> {
    let sd = SceneDraw::load(p, tables, file, layer)?;
    let state = bake_state(layer, sd.rooms.first().and_then(|r| r.time));
    let mut notes = BTreeSet::new();
    let meshes = sd.build(p, tables, &state, &mut notes);
    let mut rooms = Vec::new();
    for (ri, r) in sd.rooms.iter().enumerate() {
        let mesh = meshes.iter().find(|m| m.index == ri);
        rooms.push(RoomData {
            file: r.file_name.clone(),
            index: ri,
            behavior: r.behavior,
            echo: r.echo,
            time: r.time,
            skybox_disabled: r.skybox_disabled,
            sun_moon_disabled: r.sun_moon_disabled,
            actors: r.actors.clone(),
            objects: r.objects.clone(),
            shape: r.shape.as_ref().map(|s| s.kind),
            entries: mesh
                .map(|m| m.entries.iter().map(|e| EntryMesh { bounds: e.bounds, opa: e.opa.clone(), xlu: e.xlu.clone() }).collect())
                .unwrap_or_default(),
            backgrounds: match &r.shape {
                Some(s) => s.background_meshes(&sd.scene.file, &r.data).with_context(|| format!("{} room {ri}", file))?,
                None => Vec::new(),
            },
        });
    }
    let layer_data = LayerData {
        header_offset: sd.scene.header_offset as u32,
        collision: keys::scene_collision(file, layer),
        spawns: sd.scene.spawns.clone(),
        light_settings: sd.scene.light_settings.clone(),
        skybox: sd.scene.skybox,
        keep_object: sd.keep_file.clone(),
        keep_object_id: sd.scene.keep_object.map(|k| k as i16),
        c_up_elf_msg_num: sd.scene.c_up_elf_msg_num,
        entrances: sd.scene.entrances.clone(),
        exits: sd.scene.exits.clone(),
        transition_actors: sd.scene.transition_actors.clone(),
        paths: sd.scene.paths.clone(),
        scene_cam_type: sd.scene.scene_cam_type,
        sound: sd.scene.sound,
        // The script at the command's offset: the XML's Cutscene symbol there, or the offset's.
        cutscene: sd.scene.cutscene.map(|off| scene_cutscene_key(p, file, off)),
        rooms: (0..rooms.len()).map(|ri| keys::room(file, layer, ri)).collect(),
        bake_day_time: state.day_time,
        notes: notes.iter().cloned().collect(),
    };
    Ok((layer_data, rooms, sd.scene.collision))
}

/// Every scene in the scene table that the ROM has.
fn import_scenes(p: &Project, w: &PackWriter, tally: &mut Tally) -> Result<SceneCounts> {
    let tables = SceneTables::load(&p.config.decomp)?;
    struct One {
        tally: Tally,
        counts: SceneCounts,
    }
    let results = parallel(&tables.scenes, |def| -> Result<One> {
        let mut t = Tally::default();
        let mut c = SceneCounts::default();
        if p.rom.index_of(&def.file).is_none() {
            t.skip("Scene", "in the scene table but not in the ROM");
            return Ok(One { tally: t, counts: c });
        }
        let mut layers = Vec::new();
        let mut seen_headers = Vec::new();
        let mut room_files = BTreeSet::new();
        // The game layers, then the cutscene layers the alternate header list names.
        let n_layers = crate::scene::layer_count(&p.rom.file_by_name(&def.file)?).max(GAME_LAYERS);
        for layer in 0..n_layers {
            let (ld, rooms, collision) = layer_records(p, &tables, &def.file, layer).with_context(|| format!("{} layer {layer}", def.file))?;
            // Identical headers of other layers are stored once.
            w.put(&ld.collision, &collision)?;
            let distinct = !seen_headers.contains(&ld.header_offset);
            if distinct {
                seen_headers.push(ld.header_offset);
                c.headers += 1;
            }
            for r in &rooms {
                let tris = r.triangles();
                if r.shape.is_some() && distinct {
                    c.rooms += 1;
                    c.entries += r.entries.len();
                    c.triangles += tris;
                }
                if layer == 0 && r.shape.is_some() {
                    c.main_triangles += tris;
                }
                for e in &r.entries {
                    for d in [&e.opa, &e.xlu].into_iter().flatten() {
                        c.unknown_opcodes += d.stats.unknown_opcodes.values().sum::<usize>();
                        if distinct {
                            for a in d.stats.unresolved_addresses.keys() {
                                c.unresolved.push(format!("{} {a}", def.file));
                            }
                        }
                    }
                }
                room_files.insert(r.file.clone());
                w.put(&keys::room(&def.file, layer, r.index), r)?;
            }
            for n in &ld.notes {
                t.notes.push(format!("{} layer {layer}: {n}", def.file));
            }
            layers.push(ld);
        }
        for _ in &room_files {
            t.ok("Room");
        }
        let data = SceneData { name: def.file.clone(), id: def.id as u16, draw_config: def.draw_config.clone(), layers };
        w.put(&keys::scene(&def.file), &data)?;
        t.ok("Scene");
        c.scenes = 1;
        Ok(One { tally: t, counts: c })
    });
    let mut total = SceneCounts::default();
    for r in results {
        let r = r?;
        tally.merge(r.tally);
        let c = r.counts;
        total.scenes += c.scenes;
        total.headers += c.headers;
        total.rooms += c.rooms;
        total.entries += c.entries;
        total.main_triangles += c.main_triangles;
        total.triangles += c.triangles;
        total.unknown_opcodes += c.unknown_opcodes;
        total.unresolved.extend(c.unresolved);
    }
    total.unresolved.sort();
    total.unresolved.dedup();
    Ok(total)
}

/// Imports into the default place (`oot_game::pack::pack_path` for the ROM's SHA-1, or the
/// loose folder given) and makes it the pack the game uses.
pub fn import_to_default(p: &Project, loose: Option<&Path>) -> Result<(ImportReport, PathBuf)> {
    let sha = rom_sha1(&p.config.rom)?;
    let out = match loose {
        Some(d) => Output::Loose(d.to_path_buf()),
        None => Output::Pack(oot_game::pack::pack_path(&sha)?),
    };
    let r = import(p, &out)?;
    if loose.is_none() {
        oot_game::pack::set_default(&r.rom_sha1)?;
    }
    Ok((r, out.path().to_path_buf()))
}

/// A one-screen summary of an import.
pub fn summary(r: &ImportReport, path: &Path) -> String {
    let mut s = String::new();
    let m = &r.manifest;
    s += &format!("pack: {} ({:.1} MB, {} records, {} distinct; {:.1} MB before compression)\n", path.display(), r.stats.file_bytes as f64 / 1e6, r.stats.records, r.stats.blobs, r.stats.raw_bytes as f64 / 1e6);
    s += &format!("ROM SHA-1 {}, {:.1} s:", r.rom_sha1, r.seconds);
    for (phase, t) in &m.timings {
        s += &format!(" {phase} {t:.1} s");
    }
    s += "\n";
    for (kind, (listed, imported)) in &m.counts {
        s += &format!("  {kind:<20} {imported:>5} of {listed:>5}");
        if let Some(sk) = m.skipped.get(kind) {
            let reasons: Vec<String> = sk.iter().map(|(why, n)| format!("{n} {why}")).collect();
            s += &format!("  (skipped: {})", reasons.join("; "));
        }
        s += "\n";
    }
    let c = &m.scenes;
    s += &format!(
        "  scenes {}: {} distinct headers, {} rooms, {} entries, {} triangles ({} in the main headers), {} unknown opcodes, {} unresolved references\n",
        c.scenes, c.headers, c.rooms, c.entries, c.triangles, c.main_triangles, c.unknown_opcodes, c.unresolved.len()
    );
    s
}
