//! `ootx`: numeric validation of the decoding pipeline against the user's ROM.
//! Produces statistics and JSON reports only.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use eng_anim::anim::{Animation, LinkAnimation, StandardAnimation};
use eng_anim::skeleton::{LimbType, Skeleton};
use eng_gbi::gbi::DrawList;
use eng_gbi::model::{Binding, BuildOptions, build_draw_list};
use glam::{Mat4, Vec3, Vec4Swizzles};
use oot_import::project::Project;
use oot_import::symbols::AssetFile;
use oot_import::z64::{ParseLinkAnimation, ParseSkeleton, ParseStandardAnimation};
use serde::Serialize;
use oot_import::player::LoadPlayerRules;

#[derive(Parser)]
#[command(about = "Validate OoT ROM decoding (skeletons, display lists, animations)")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
    /// Directory for JSON reports.
    #[arg(long, default_value = "out")]
    out: PathBuf,
}

#[derive(Subcommand)]
enum Cmd {
    /// ROM header, file table and name mapping.
    Info,
    /// Build the game's asset pack (the same as `oot import`), or a loose folder of records.
    Import {
        /// Write loose records to this folder instead of the default pack.
        #[arg(long)]
        loose: Option<PathBuf>,
    },
    /// Decode every skeleton listed in the decomp XMLs and every animation next to it.
    ScanSkeletons {
        /// Only scan files whose name contains this string.
        #[arg(long)]
        filter: Option<String>,
    },
    /// Validate all Player animations (gameplay_keep headers + link_animetion data)
    /// against the skeleton in the given object file.
    PlayerAnims {
        #[arg(long, default_value = "object_link_boy")]
        object: String,
        /// Scale applied to the root translation (the game uses 0.64 for child Link).
        #[arg(long, default_value_t = 1.0)]
        root_scale: f32,
    },
    /// Build Link's draw list for every age, model group and shield using Player's draw
    /// rules read from z_player_lib.c, and report missing or unresolved references.
    PlayerDraw,
    /// Load every scene through the runtime loader (`oot_import::room`) and interpret every room
    /// display list: unknown opcodes, unresolved segment references, dynamic materials.
    ScanScenes {
        /// Only scenes whose file name contains this string.
        #[arg(long)]
        filter: Option<String>,
        /// Also scan the child-night, adult-day and adult-night headers when they differ.
        #[arg(long)]
        all_layers: bool,
    },
    /// Materials of every entry of one room, as the runtime loader builds them, with the
    /// world-space bounds of each batch (for tracking down rendering differences).
    DumpRoom {
        #[arg(long)]
        scene: String,
        #[arg(long)]
        room: usize,
        #[arg(long, default_value_t = 0)]
        layer: usize,
        /// Also write every decoded texture to this directory as PNG.
        #[arg(long)]
        png: Option<PathBuf>,
    },
    /// What the asset pack holds for one scene layer: spawns and the entrance list, exits,
    /// transition actors, and each room's objects and actor placements (by actor name).
    SceneInfo {
        #[arg(long)]
        scene: String,
        #[arg(long, default_value_t = 0)]
        layer: usize,
    },
    /// The asset pack's record names that start with a prefix (e.g. `mesh/object_kusa/`); meshes
    /// with their triangle count and unresolved segment references.
    PackLs {
        prefix: String,
    },
    /// The raw commands of a display list symbol (`file symbol`), up to its end, following
    /// calls into the same file.
    DlDump {
        file: String,
        symbol: String,
    },
    /// Extract assets into editable formats (PNG, glTF, WAV, JSON) in a git-ignored folder.
    /// Local development only: the output is derived from the ROM and must not be shared.
    Extract {
        /// Output directory.
        #[arg(long, default_value = "extracted")]
        dir: PathBuf,
        /// Only these parts: raw, textures, models, scenes, audio, text (default: all).
        #[arg(long, value_delimiter = ',')]
        only: Vec<String>,
    },
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let cli = Cli::parse();
    let project = Project::open_default()?;
    std::fs::create_dir_all(&cli.out)?;
    match cli.cmd {
        Cmd::Info => info(&project),
        Cmd::SceneInfo { scene, layer } => scene_info(&scene, layer),
        Cmd::PackLs { prefix } => pack_ls(&prefix),
        Cmd::DlDump { file, symbol } => dl_dump(&project, &file, &symbol),
        Cmd::Import { loose } => {
            let (r, path) = oot_import::pack::import_to_default(&project, loose.as_deref())?;
            print!("{}", oot_import::pack::summary(&r, &path));
            let json = cli.out.join("import_manifest.json");
            std::fs::write(&json, serde_json::to_string_pretty(&r.manifest)?)?;
            println!("manifest: {}", json.display());
            Ok(())
        }
        Cmd::ScanSkeletons { filter } => scan_skeletons(&project, filter.as_deref(), &cli.out),
        Cmd::PlayerAnims { object, root_scale } => player_anims(&project, &object, root_scale, &cli.out),
        Cmd::PlayerDraw => player_draw(&project, &cli.out),
        Cmd::Extract { dir, only } => extract(&project, &dir, &only),
        Cmd::ScanScenes { filter, all_layers } => scan_scenes(&project, filter.as_deref(), all_layers, &cli.out),
        Cmd::DumpRoom { scene, room, layer, png } => dump_room(&project, &scene, room, layer, png.as_deref()),
    }
}

fn dump_room(p: &Project, name: &str, room: usize, layer: usize, png: Option<&std::path::Path>) -> Result<()> {
    use oot_import::room::{SceneDraw, SceneTables};
    let tables = SceneTables::load(&p.config.decomp)?;
    let s = SceneDraw::load(p, &tables, name, layer)?;
    let mut notes = std::collections::BTreeSet::new();
    let meshes = s.build(p, &tables, &oot_import::drawcfg::State::default(), &mut notes);
    let r = meshes.iter().find(|m| m.index == room).context("no such room")?;
    println!("{} room {room}: {:?}, {} entries", s.scene.name, r.kind, r.entries.len());
    for (ei, e) in r.entries.iter().enumerate() {
        println!("entry {ei}: bounds {:?}", e.bounds);
        for (kind, d) in [("opa", &e.opa), ("xlu", &e.xlu)] {
            let Some(d) = d else { continue };
            if let Some(dir) = png {
                std::fs::create_dir_all(dir)?;
                for (ti, t) in d.textures.iter().enumerate() {
                    let f = dir.join(format!("room{room}_e{ei}_{kind}_t{ti}.png"));
                    image::save_buffer(&f, &t.image.rgba, t.image.width, t.image.height, image::ColorType::Rgba8)?;
                }
            }
            for b in &d.batches {
                let m = &d.materials[b.material];
                let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
                for v in &b.vertices {
                    lo = lo.min(v.pos);
                    hi = hi.max(v.pos);
                }
                let tex: Vec<String> = m.textures.iter().flatten().map(|t| {
                    let im = &d.textures[t.image];
                    format!("{}x{} f{}s{}", im.image.width, im.image.height, im.fmt, im.siz)
                }).collect();
                println!(
                    "  {kind} mat {:>2} tris {:>4} {:?} cc {:014X} 2cyc {} gm {:06X} omL {:08X} env {:?} prim {:?} tex {:?} dyn {:?}/{:?} fog {} bounds {:.0?}..{:.0?}",
                    b.material, b.vertices.len() / 3, m.blend, m.combiner.raw, m.two_cycle, m.geometry_mode, m.othermode_l, m.env, m.prim, tex,
                    m.uv_dyn.map(|x| x.map(|d| d.segment)), m.env_dyn, m.fog_blend, lo, hi
                );
            }
        }
    }
    Ok(())
}

fn scan_scenes(p: &Project, filter: Option<&str>, all_layers: bool, out: &std::path::Path) -> Result<()> {
    use oot_import::room::{SceneDraw, SceneTables};
    let tables = SceneTables::load(&p.config.decomp)?;
    let state = oot_import::drawcfg::State::default();
    let mut rows = Vec::new();
    let (mut n_scenes, mut n_layers, mut n_rooms, mut n_entries, mut tris, mut dyn_mats, mut errors) = (0, 0, 0, 0, 0usize, 0usize, 0);
    let mut unknown: BTreeMap<String, usize> = BTreeMap::new();
    let mut unresolved: BTreeMap<String, usize> = BTreeMap::new();
    for sd in &tables.scenes {
        if filter.is_some_and(|f| !sd.file.contains(f)) || p.rom.index_of(&sd.file).is_none() {
            continue;
        }
        n_scenes += 1;
        let mut seen = Vec::new();
        for layer in 0..if all_layers { 4 } else { 1 } {
            let s = match SceneDraw::load(p, &tables, &sd.file, layer) {
                Ok(s) => s,
                Err(e) => {
                    errors += 1;
                    rows.push(serde_json::json!({ "scene": sd.file, "layer": layer, "error": format!("{e:#}") }));
                    continue;
                }
            };
            if seen.contains(&s.scene.header_offset) {
                continue;
            }
            seen.push(s.scene.header_offset);
            n_layers += 1;
            let mut notes = std::collections::BTreeSet::new();
            let meshes = s.build(p, &tables, &state, &mut notes);
            let (mut t, mut m, mut unk, mut unres) = (0usize, 0usize, BTreeMap::<String, usize>::new(), BTreeMap::<String, usize>::new());
            for r in &meshes {
                n_rooms += 1;
                for e in &r.entries {
                    n_entries += 1;
                    for d in [&e.opa, &e.xlu].into_iter().flatten() {
                        t += d.triangle_count();
                        m += d.materials.iter().filter(|x| x.is_dynamic()).count();
                        for (k, v) in &d.stats.unknown_opcodes {
                            *unk.entry(k.clone()).or_default() += v;
                        }
                        for (k, v) in &d.stats.unresolved_addresses {
                            *unres.entry(k.clone()).or_default() += v;
                        }
                    }
                }
            }
            tris += t;
            dyn_mats += m;
            for (k, v) in &unk {
                *unknown.entry(k.clone()).or_default() += v;
            }
            for (k, v) in &unres {
                *unresolved.entry(format!("{} {k}", sd.file)).or_default() += v;
            }
            rows.push(serde_json::json!({
                "scene": sd.file, "layer": layer, "draw_config": s.draw_fn, "keep": s.keep_file,
                "rooms": meshes.len(), "triangles": t, "dynamic_materials": m,
                "light_settings": s.scene.light_settings.len(), "skybox": s.scene.skybox.skybox_id, "light_mode": s.scene.skybox.light_mode,
                "unknown_opcodes": unk, "unresolved": unres, "notes": notes,
            }));
        }
    }
    let summary = serde_json::json!({
        "scenes": n_scenes, "layers_scanned": n_layers, "rooms": n_rooms, "shape_entries": n_entries, "triangles": tris,
        "dynamic_materials": dyn_mats, "unknown_opcodes": unknown, "unresolved_references": unresolved, "load_errors": errors,
    });
    let path = out.join("scene_scan.json");
    std::fs::write(&path, serde_json::to_string_pretty(&serde_json::json!({ "summary": summary, "scenes": rows }))?)?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    println!("{}", path.display());
    Ok(())
}

fn info(p: &Project) -> Result<()> {
    let rom = &p.rom;
    println!("title:        {}", rom.title);
    println!("game code:    {}", rom.game_code);
    println!("size:         {} bytes", rom.data.len());
    println!("dmadata at:   0x{:X}", rom.dmadata_offset);
    println!("files:        {}", rom.files.len());
    println!("names (spec): {}", if p.names_applied { "applied" } else { "MISMATCH" });
    let compressed = rom.files.iter().filter(|e| e.is_compressed()).count();
    println!("compressed:   {compressed}");
    println!("xml files:    {}", p.symbols.files.len());
    let skels: usize = p.symbols.files.iter().map(|f| f.of_kind("Skeleton").count()).sum();
    let anims: usize = p.symbols.files.iter().map(|f| f.of_kind("Animation").count()).sum();
    let panims: usize = p.symbols.files.iter().map(|f| f.of_kind("PlayerAnimation").count()).sum();
    println!("symbols:      {skels} skeletons, {anims} animations, {panims} player animations");
    // Spot-check that named files decode.
    for name in ["gameplay_keep", "link_animetion", "object_link_boy", "object_link_child"] {
        match rom.file_by_name(name) {
            Ok(f) => println!("  {name:<20} {:>8} bytes", f.len()),
            Err(e) => println!("  {name:<20} ERROR {e:#}"),
        }
    }
    Ok(())
}

fn limb_type_of(attr: Option<&str>) -> LimbType {
    match attr {
        Some("LOD") => LimbType::Lod,
        _ => LimbType::Standard,
    }
}

/// Standard segment bindings for an object: 4 = gameplay_keep, 5 = gameplay_field_keep, 6 = the object.
fn object_bindings(p: &Project, file: &AssetFile, data: Arc<[u8]>) -> Vec<Binding> {
    let mut b = Vec::new();
    for (seg, name) in [(4u8, "gameplay_keep"), (5u8, "gameplay_field_keep")] {
        if let Ok(buf) = p.rom.file_by_name(name) {
            b.push(Binding { segment: seg, buf, base: 0 });
        }
    }
    b.push(Binding { segment: file.segment.unwrap_or(6), buf: data, base: 0 });
    b
}

#[derive(Serialize, Default)]
struct DrawSummary {
    triangles: usize,
    batches: usize,
    materials: usize,
    textures: usize,
    texture_formats: BTreeMap<String, usize>,
    commands: usize,
    matrix_loads: usize,
    unknown_opcodes: BTreeMap<String, usize>,
    ignored_opcodes: BTreeMap<String, usize>,
    /// Unresolved addresses grouped by segment number.
    unresolved_by_segment: BTreeMap<String, usize>,
    blend_modes: BTreeMap<String, usize>,
    two_cycle_materials: usize,
    lit_materials: usize,
}

fn summarize(d: &DrawList) -> DrawSummary {
    let mut s = DrawSummary {
        triangles: d.triangle_count(),
        batches: d.batches.len(),
        materials: d.materials.len(),
        textures: d.textures.len(),
        commands: d.stats.commands,
        matrix_loads: d.stats.matrix_loads,
        ..Default::default()
    };
    for t in &d.textures {
        let k = format!(
            "{} {}x{}",
            eng_gbi::texture::format_name(t.fmt, t.siz),
            t.image.width,
            t.image.height
        );
        *s.texture_formats.entry(k).or_default() += 1;
    }
    s.unknown_opcodes = d.stats.unknown_opcodes.iter().map(|(k, v)| (k.clone(), *v)).collect();
    s.ignored_opcodes = d.stats.ignored_opcodes.iter().map(|(k, v)| (k.clone(), *v)).collect();
    for (addr, n) in &d.stats.unresolved_addresses {
        *s.unresolved_by_segment.entry(format!("seg {}", &addr[..2])).or_default() += n;
    }
    for m in &d.materials {
        *s.blend_modes.entry(format!("{:?}", m.blend)).or_default() += 1;
        s.two_cycle_materials += m.two_cycle as usize;
        s.lit_materials += m.lit as usize;
    }
    s
}

#[derive(Serialize)]
struct SkeletonReport {
    file: String,
    skeleton: String,
    ok: bool,
    error: Option<String>,
    limbs: usize,
    flex: bool,
    dlist_count: u8,
    matrix_map_len: usize,
    lods: Vec<DrawSummary>,
    animations_total: usize,
    animations_ok: usize,
    animation_errors: Vec<String>,
}

fn scan_skeletons(p: &Project, filter: Option<&str>, out: &std::path::Path) -> Result<()> {
    let mut reports = Vec::new();
    for file in &p.symbols.files {
        if filter.is_some_and(|f| !file.name.contains(f)) {
            continue;
        }
        let skels: Vec<_> = file.of_kind("Skeleton").collect();
        if skels.is_empty() {
            continue;
        }
        let Ok(data) = p.rom.file_by_name(&file.name) else {
            log::warn!("{} not in ROM", file.name);
            continue;
        };
        let seg = file.segment.unwrap_or(6);
        for sym in skels {
            let flex = sym.attr("Type") == Some("Flex");
            let lt = limb_type_of(sym.attr("LimbType"));
            let mut r = SkeletonReport {
                file: file.name.clone(),
                skeleton: sym.name.clone(),
                ok: false,
                error: None,
                limbs: 0,
                flex,
                dlist_count: 0,
                matrix_map_len: 0,
                lods: Vec::new(),
                animations_total: 0,
                animations_ok: 0,
                animation_errors: Vec::new(),
            };
            // Only standard and LOD limb types are implemented; skip Skin/Curve/Legacy honestly.
            if !matches!(sym.attr("LimbType"), Some("Standard") | Some("LOD")) {
                r.error = Some(format!("limb type {:?} not implemented", sym.attr("LimbType")));
                reports.push(r);
                continue;
            }
            // Some XMLs label flex skeletons as normal; the header's dListCount byte gives it away.
            let parsed = Skeleton::parse(&data, seg, sym.offset as usize, lt, flex).and_then(|s| {
                let o = sym.offset as usize + 8;
                if !flex && data.get(o).is_some_and(|&n| n != 0 && n as usize == s.flex_matrix_map(0).len()) {
                    r.flex = true;
                    Skeleton::parse(&data, seg, sym.offset as usize, lt, true)
                } else {
                    Ok(s)
                }
            });
            match parsed {
                Ok(skel) => {
                    r.limbs = skel.limbs.len();
                    r.dlist_count = skel.dlist_count;
                    r.matrix_map_len = skel.flex_matrix_map(0).len();
                    let n_lod = if lt == LimbType::Lod { 2 } else { 1 };
                    for lod in 0..n_lod {
                        let opts = BuildOptions { lod, bindings: object_bindings(p, file, data.clone()), ..Default::default() };
                        match build_draw_list(&skel, &opts) {
                            Ok(d) => r.lods.push(summarize(&d)),
                            Err(e) => r.error = Some(format!("lod {lod}: {e:#}")),
                        }
                    }
                    for a in file.of_kind("Animation") {
                        r.animations_total += 1;
                        match StandardAnimation::parse(&data, seg, a.offset as usize, skel.limbs.len()) {
                            Ok(anim) => {
                                let jt = anim.sample(0);
                                if jt.rot.len() == skel.limbs.len() + 1 {
                                    r.animations_ok += 1;
                                }
                            }
                            Err(e) => r.animation_errors.push(format!("{}: {e:#}", a.name)),
                        }
                    }
                    r.ok = r.error.is_none();
                }
                Err(e) => r.error = Some(format!("{e:#}")),
            }
            reports.push(r);
        }
    }

    // Console summary.
    let total = reports.len();
    let ok = reports.iter().filter(|r| r.ok).count();
    let clean = reports
        .iter()
        .filter(|r| r.ok && r.lods.iter().all(|l| l.unknown_opcodes.is_empty() && l.unresolved_by_segment.is_empty()))
        .count();
    let tris: usize = reports.iter().filter_map(|r| r.lods.first()).map(|l| l.triangles).sum();
    let anims_total: usize = reports.iter().map(|r| r.animations_total).sum();
    let anims_ok: usize = reports.iter().map(|r| r.animations_ok).sum();
    let mut unresolved: BTreeMap<String, usize> = BTreeMap::new();
    let mut unknown: BTreeMap<String, usize> = BTreeMap::new();
    for r in &reports {
        for l in &r.lods {
            for (k, v) in &l.unresolved_by_segment {
                *unresolved.entry(k.clone()).or_default() += v;
            }
            for (k, v) in &l.unknown_opcodes {
                *unknown.entry(k.clone()).or_default() += v;
            }
        }
    }
    println!("skeletons scanned:          {total}");
    println!("parsed + drawn without err: {ok}");
    println!("fully clean (no unknown ops, no unresolved segments): {clean}");
    println!("triangles (lod 0, all):     {tris}");
    println!("animations parsed:          {anims_ok}/{anims_total} (joint count matched first skeleton in file)");
    println!("unresolved refs by segment: {unresolved:?}");
    println!("unknown opcodes:            {unknown:?}");
    let failures: Vec<_> = reports.iter().filter(|r| !r.ok).collect();
    if !failures.is_empty() {
        println!("failures ({}):", failures.len());
        for r in failures.iter().take(25) {
            println!("  {} / {}: {}", r.file, r.skeleton, r.error.as_deref().unwrap_or("?"));
        }
    }
    let path = out.join("skeleton_scan.json");
    std::fs::write(&path, serde_json::to_string_pretty(&reports)?)?;
    println!("report: {}", path.display());
    Ok(())
}

#[derive(Serialize)]
struct PlayerAnimReport {
    name: String,
    frames: usize,
    xml_frame_count: Option<usize>,
    ok: bool,
    error: Option<String>,
    /// Largest frame-to-frame change of any limb rotation component (binary angle units).
    /// Misleading on its own: Euler flips produce ~0x8000 jumps for identical orientations.
    max_rot_delta: i32,
    /// Largest true rotation (degrees) of any limb between consecutive frames.
    max_step_deg: f32,
    /// Lowest posed vertex per frame: min and max over the animation (feet-on-ground check).
    lowest_y_min: f32,
    lowest_y_max: f32,
    /// Posed mesh height range over all frames (model units).
    min_y: f32,
    max_y: f32,
}

fn player_anims(p: &Project, object: &str, root_scale: f32, out: &std::path::Path) -> Result<()> {
    let keep_sym = p.symbols.file("gameplay_keep").context("gameplay_keep.xml")?;
    let keep = p.rom.file_by_name("gameplay_keep")?;
    let anim_data = p.rom.file_by_name("link_animetion")?;
    let data_syms = p.symbols.file("link_animetion").context("link_animetion.xml")?;
    let xml_counts: BTreeMap<u32, usize> = data_syms
        .of_kind("PlayerAnimationData")
        .filter_map(|s| Some((s.offset, s.attr("FrameCount")?.parse().ok()?)))
        .collect();

    let obj_sym = p.symbols.file(object).with_context(|| format!("{object}.xml"))?;
    let obj = p.rom.file_by_name(object)?;
    let skel_sym = obj_sym.of_kind("Skeleton").next().context("no skeleton in object")?;
    let skel = Skeleton::parse(&obj, 6, skel_sym.offset as usize, LimbType::Lod, true)?;
    let opts = BuildOptions { lod: 0, bindings: object_bindings(p, obj_sym, obj.clone()), ..Default::default() };
    let draw = build_draw_list(&skel, &opts)?;
    let d = summarize(&draw);
    println!("{object}: {} limbs, dListCount {}, matrix map {}, {} tris, {} textures, {} materials",
        skel.limbs.len(), skel.dlist_count, skel.flex_matrix_map(0).len(), d.triangles, d.textures, d.materials);
    println!("  unresolved: {:?}  unknown opcodes: {:?}", d.unresolved_by_segment, d.unknown_opcodes);

    let mut reports = Vec::new();
    for sym in keep_sym.of_kind("PlayerAnimation") {
        let o = sym.offset as usize;
        let mut r = PlayerAnimReport {
            name: sym.name.clone(),
            frames: 0,
            xml_frame_count: None,
            ok: false,
            error: None,
            max_rot_delta: 0,
            max_step_deg: 0.0,
            lowest_y_min: f32::MAX,
            lowest_y_max: f32::MIN,
            min_y: f32::MAX,
            max_y: f32::MIN,
        };
        match LinkAnimation::parse(&keep[o..o + 8], &anim_data) {
            Ok(a) => {
                r.frames = a.frame_count();
                r.xml_frame_count = xml_counts.get(&(a.data_offset as u32)).copied();
                for w in a.frames.windows(2) {
                    for (j, (x, y)) in w[0].rot.iter().zip(&w[1].rot).enumerate().skip(1) {
                        let _ = j;
                        for k in 0..3 {
                            r.max_rot_delta = r.max_rot_delta.max((y[k].wrapping_sub(x[k]) as i32).abs());
                        }
                    }
                }
                for w in a.frames.windows(2) {
                    for (x, y) in w[0].rot.iter().zip(&w[1].rot).skip(1) {
                        let q0 = limb_quat(*x);
                        let q1 = limb_quat(*y);
                        let ang = q0.angle_between(q1).to_degrees();
                        r.max_step_deg = r.max_step_deg.max(ang);
                    }
                }
                for f in 0..a.frame_count() {
                    let mut jt = a.sample(f);
                    jt.rot[0] = jt.rot[0].map(|c| (c as f32 * root_scale) as i16);
                    let mats = skel.pose(&jt);
                    let (lo, hi) = posed_bounds(&draw, &mats);
                    r.min_y = r.min_y.min(lo.y);
                    r.max_y = r.max_y.max(hi.y);
                    r.lowest_y_min = r.lowest_y_min.min(lo.y);
                    r.lowest_y_max = r.lowest_y_max.max(lo.y);
                }
                r.ok = r.xml_frame_count.is_none_or(|c| c == r.frames);
            }
            Err(e) => r.error = Some(format!("{e:#}")),
        }
        reports.push(r);
    }
    let ok = reports.iter().filter(|r| r.ok).count();
    let matched = reports.iter().filter(|r| r.xml_frame_count.is_some()).count();
    let frames: usize = reports.iter().map(|r| r.frames).sum();
    let mut deltas: Vec<i32> = reports.iter().filter(|r| r.ok).map(|r| r.max_rot_delta).collect();
    deltas.sort();
    let pct = |q: f32| deltas.get(((deltas.len() as f32 - 1.0) * q) as usize).copied().unwrap_or(0);
    println!("player animations: {ok}/{} parsed, {matched} matched to XML frame counts, {frames} frames total", reports.len());
    println!("max per-frame rotation delta (binang): p50 {} p90 {} p99 {} max {}", pct(0.5), pct(0.9), pct(0.99), pct(1.0));
    let mut steps: Vec<f32> = reports.iter().filter(|r| r.ok).map(|r| r.max_step_deg).collect();
    steps.sort_by(|a, b| a.total_cmp(b));
    let spct = |q: f32| steps.get(((steps.len() as f32 - 1.0) * q) as usize).copied().unwrap_or(0.0);
    println!("max per-frame true limb rotation (deg): p50 {:.1} p90 {:.1} p99 {:.1} max {:.1}", spct(0.5), spct(0.9), spct(0.99), spct(1.0));
    for name in ["gPlayerAnim_link_normal_wait", "gPlayerAnim_link_normal_walk", "gPlayerAnim_link_fighter_run"] {
        if let Some(r) = reports.iter().find(|r| r.name == name) {
            println!("  {name}: {} frames, lowest vertex y {:.0}..{:.0}, max step {:.1} deg", r.frames, r.lowest_y_min, r.lowest_y_max, r.max_step_deg);
        }
    }
    let heights: Vec<f32> = reports.iter().filter(|r| r.ok).map(|r| r.max_y - r.min_y).collect();
    let hmin = heights.iter().cloned().fold(f32::MAX, f32::min);
    let hmax = heights.iter().cloned().fold(f32::MIN, f32::max);
    println!("posed vertical extent across animations: {hmin:.0} .. {hmax:.0} model units");
    let bind = skel.bind_pose();
    let (lo, hi) = posed_bounds(&draw, &bind);
    println!("rest-pose bounds: {lo:?} .. {hi:?}");

    let path = out.join(format!("player_anims_{object}.json"));
    std::fs::write(&path, serde_json::to_string_pretty(&reports)?)?;
    println!("report: {}", path.display());
    Ok(())
}

fn limb_quat(r: [i16; 3]) -> glam::Quat {
    use eng_anim::skeleton::binang_to_rad as b;
    glam::Quat::from_rotation_z(b(r[2])) * glam::Quat::from_rotation_y(b(r[1])) * glam::Quat::from_rotation_x(b(r[0]))
}

fn posed_bounds(draw: &DrawList, mats: &[Mat4]) -> (Vec3, Vec3) {
    let mut lo = Vec3::splat(f32::MAX);
    let mut hi = Vec3::splat(f32::MIN);
    for b in &draw.batches {
        for v in &b.vertices {
            let m = mats.get(v.bone as usize).copied().unwrap_or(Mat4::IDENTITY);
            let p = (m * v.pos.extend(1.0)).xyz();
            lo = lo.min(p);
            hi = hi.max(p);
        }
    }
    (lo, hi)
}

#[derive(Serialize)]
struct PlayerDrawReport {
    age: String,
    model_group: String,
    shield: String,
    limb_dlists: Vec<(u8, Option<String>)>,
    missing_names: Vec<String>,
    summary: DrawSummary,
}

fn player_draw(p: &Project, out: &std::path::Path) -> Result<()> {
    use oot_import::player::{Age, Loadout, PlayerModel, PlayerRules};
    let rules = PlayerRules::load(&p.config.decomp)?;
    println!(
        "rules: {} model types, {} model groups, {} shields, {} tunics, {} eye / {} mouth textures",
        rules.model_types.len(),
        rules.model_groups.len(),
        rules.shields.len(),
        rules.tunics.len(),
        rules.eye_textures.len(),
        rules.mouth_textures.len()
    );
    let mut reports = Vec::new();
    for age in [Age::Adult, Age::Child] {
        let model = PlayerModel::load(p, &rules, age)?;
        for (gi, g) in rules.model_groups.iter().enumerate() {
            for (si, s) in rules.shields.iter().enumerate() {
                let lo = Loadout { model_group: gi, shield: si, ..Loadout::default_for(&rules, age) };
                let (draw, missing) = model.draw_list(&rules, &lo, 0, 0, 0)?;
                reports.push(PlayerDrawReport {
                    age: format!("{age:?}"),
                    model_group: g.name.clone(),
                    shield: s.clone(),
                    limb_dlists: rules.limb_dlists(&lo, 0),
                    missing_names: missing,
                    summary: summarize(&draw),
                });
            }
        }
    }
    let missing: usize = reports.iter().map(|r| r.missing_names.len()).sum();
    let unknown: usize = reports.iter().map(|r| r.summary.unknown_opcodes.values().sum::<usize>()).sum();
    let mut unresolved: BTreeMap<String, usize> = BTreeMap::new();
    for r in &reports {
        for (k, v) in &r.summary.unresolved_by_segment {
            *unresolved.entry(k.clone()).or_default() += v;
        }
    }
    let tris: Vec<usize> = reports.iter().map(|r| r.summary.triangles).collect();
    println!("loadouts built: {}", reports.len());
    println!("missing DL names: {missing}, unknown opcodes: {unknown}");
    println!("unresolved refs by segment: {unresolved:?}");
    println!("triangles: {} .. {}", tris.iter().min().unwrap_or(&0), tris.iter().max().unwrap_or(&0));
    for r in reports.iter().filter(|r| r.model_group == "DEFAULT" && (r.shield == "HYLIAN" || r.shield == "DEKU")) {
        println!("  {} DEFAULT/{}: {} tris, {:?}", r.age, r.shield, r.summary.triangles, r.limb_dlists);
    }
    let path = out.join("player_draw.json");
    std::fs::write(&path, serde_json::to_string_pretty(&reports)?)?;
    println!("report: {}", path.display());
    Ok(())
}

fn extract(p: &Project, dir: &std::path::Path, only: &[String]) -> Result<()> {
    type Part = fn(&Project, &std::path::Path) -> Result<serde_json::Value>;
    let parts: [(&str, Part); 6] = [
        ("raw", oot_extract::raw::extract),
        ("textures", oot_extract::textures::extract),
        ("models", oot_extract::models::extract),
        ("scenes", oot_extract::scenes::extract),
        ("audio", oot_extract::audio::extract),
        ("text", oot_extract::text::extract),
    ];
    std::fs::create_dir_all(dir)?;
    let manifest_path = dir.join("manifest.json");
    let mut manifest: serde_json::Map<String, serde_json::Value> = std::fs::read_to_string(&manifest_path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    manifest.insert("rom".into(), serde_json::json!({ "title": p.rom.title, "game_code": p.rom.game_code, "size": p.rom.data.len() }));
    manifest.insert("note".into(), "Derived from your ROM for local development. Do not commit or distribute.".into());
    for (name, f) in parts {
        if !only.is_empty() && !only.iter().any(|o| o == name) {
            continue;
        }
        let t = std::time::Instant::now();
        let v = match f(p, dir) {
            Ok(v) => v,
            Err(e) => serde_json::json!({ "error": format!("{e:#}") }),
        };
        println!("{name:<9} {:>6.1}s  {v}", t.elapsed().as_secs_f32());
        manifest.insert(name.to_string(), v);
    }
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;
    std::fs::write(dir.join("README.md"), oot_extract::README)?;
    println!("manifest: {}", manifest_path.display());
    Ok(())
}

/// `ootx scene-info`: reads the default asset pack (no ROM access).
fn pack_ls(prefix: &str) -> Result<()> {
    let pack = oot_game::pack::GamePack::open_default()?;
    for name in pack.assets.names(prefix) {
        if name.starts_with("mesh/") {
            let d: eng_gfx::DrawList = pack.assets.get(&name)?;
            let unresolved: Vec<String> = d.stats.unresolved_addresses.keys().cloned().collect();
            println!("{name}: {} triangles, {} textures{}", d.triangle_count(), d.textures.len(), if unresolved.is_empty() { String::new() } else { format!(", unresolved {}", unresolved.join(" ")) });
        } else {
            println!("{name}");
        }
    }
    Ok(())
}

fn dl_dump(p: &Project, file: &str, symbol: &str) -> Result<()> {
    let (f, s) = oot_import::objects::symbol_in(p, file, symbol)?;
    let data = p.rom.file_by_name(&f.name)?;
    let seg = f.segment.unwrap_or(6) as u32;
    let mut stack = vec![s.offset as usize];
    while let Some(mut at) = stack.pop() {
        println!("-- {:08X}", (seg << 24) | at as u32);
        while at + 8 <= data.len() {
            let w0 = u32::from_be_bytes(data[at..at + 4].try_into()?);
            let w1 = u32::from_be_bytes(data[at + 4..at + 8].try_into()?);
            println!("{:06X}: {w0:08X} {w1:08X}", at);
            at += 8;
            match w0 >> 24 {
                0xDF => break,
                // G_VTX in the same file: x y z, s t, colour/normal.
                0x01 if w1 >> 24 == seg => {
                    let n = ((w0 >> 12) & 0xFF) as usize;
                    let base = (w1 & 0xFF_FFFF) as usize;
                    for i in 0..n {
                        let v = &data[base + i * 16..base + i * 16 + 16];
                        let s16 = |o: usize| i16::from_be_bytes([v[o], v[o + 1]]);
                        println!("        v{i}: ({}, {}, {}) st ({}, {}) rgba {:02X}{:02X}{:02X}{:02X}", s16(0), s16(2), s16(4), s16(8), s16(10), v[12], v[13], v[14], v[15]);
                    }
                }
                0xDE if w1 >> 24 == seg => {
                    if (w0 >> 16) & 0xFF == 1 {
                        at = (w1 & 0xFF_FFFF) as usize;
                    } else {
                        stack.push((w1 & 0xFF_FFFF) as usize);
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn scene_info(scene: &str, layer: usize) -> Result<()> {
    let pack = oot_game::pack::GamePack::open_default()?;
    let at = pack.actor_table()?;
    let st = pack.scene_table()?;
    let sd = pack.scene(scene)?;
    let ld = sd.layers.get(layer).context("no such layer")?;
    let obj = |id: i16| st.objects.get(id as usize).cloned().unwrap_or_else(|| format!("{id}"));
    println!("{} (id {}, layer {layer}, header {:#x}): keep {:?} ({:?})", sd.name, sd.id, ld.header_offset, ld.keep_object, ld.keep_object_id);
    let sky = st.room_skybox(ld.skybox.skybox_id).map(|s| s.name.as_str()).unwrap_or("-");
    println!("  scene cam type {:#04x}, skybox {} (room skybox {sky})", ld.scene_cam_type, ld.skybox.skybox_id);
    for (i, e) in ld.spawns.iter().enumerate() {
        println!("  spawn {i}: {} pos {:?} rot {:?} params {:#06x}", at.name(e.id), e.pos, e.rot, e.params as u16);
    }
    for (i, e) in ld.entrances.iter().enumerate() {
        println!("  entrance list {i}: spawn {} room {}", e.spawn, e.room);
    }
    for (i, e) in ld.exits.iter().enumerate() {
        let name = st.entrances.get(*e as usize).map(|x| x.name.as_str()).unwrap_or("?");
        println!("  exit {}: {e:#06x} {name}", i + 1);
    }
    for (i, t) in ld.transition_actors.iter().enumerate() {
        println!("  transition {i}: {} sides {:?} pos {:?} rotY {:#06x} params {:#06x}", at.name(t.id), t.sides, t.pos, t.rot_y as u16, t.params as u16);
    }
    for key in &ld.rooms {
        let r = pack.room(key)?;
        let objs: Vec<String> = r.objects.iter().map(|&o| obj(o)).collect();
        println!("  room {} ({}): objects {objs:?}", r.index, r.file);
        for (i, b) in r.backgrounds.iter().enumerate() {
            let t = &b.mesh.textures[0].image;
            println!("    background {i}: {}x{} for bg camera {:?}", t.width, t.height, b.bg_cam_index);
        }
        for a in &r.actors {
            let info = at.get(a.id).and_then(|i| i.init.as_ref());
            let o = info.map(|i| obj(i.object_id)).unwrap_or_default();
            println!("    {:<16} params {:#06x} pos {:?} rot {:?} object {o}", at.name(a.id), a.params as u16, a.pos, a.rot);
        }
    }
    let col = pack.collision(&ld.collision)?;
    let mut exits: BTreeMap<u32, usize> = BTreeMap::new();
    for poly in &col.polys {
        let st = col.surface_types.get(poly.ty as usize).map(|s| s.data[0]).unwrap_or(0);
        let exit = (st >> 8) & 0x1F;
        if exit != 0 {
            *exits.entry(exit).or_default() += 1;
        }
    }
    println!("  collision: {} polys; exit index -> polys {exits:?}", col.polys.len());
    let data = pack.game_data()?;
    for (i, c) in col.bg_cams.iter().enumerate() {
        let name = data.camera.setting(c.setting as i16).map(|s| s.name.as_str()).unwrap_or("?");
        println!("  bg camera {i}: {name} ({:#04x}) data {:?}", c.setting, c.data);
    }
    Ok(())
}
