//! overworld build <level.json> <out dir> [--theme <theme.json>] [--textures <dir>]... [--kit <dir>]...
//! overworld kit-textures [<region>...]
//! overworld kit-pieces [<region>...]
//! overworld kit-pieces <manifest.json> <out dir>
//! overworld kit-survey <region> [--box <x0> <y0> <x1> <y1>] [--below <z>]
//! overworld preview <level.json> <out.png> [--theme <theme.json>]
//!
//! The regions are the overworld scenes (`kit::REGIONS`: kokiri, kakariko, hyrule_field...), each
//! with a kit manifest `kit/<region>.json`.
//!
//! `kit-textures` makes the regions' texture libraries (`kit.rs`) from the clone's extracted scenes,
//! each into `out/overworld/textures/<region>`: all of them, or the regions named.
//!
//! `kit-pieces` cuts the pieces of the regions' manifests (Kokiri's houses, stumps, stones, the log
//! tunnel, the crawlspace...) out of the extracts they name, each into `out/overworld/kit/<region>`
//! (`pieces.rs`), with a picture of each piece in its `thumbs` folder: all of them, or the regions
//! named, or one manifest into the folder given. Paths in a manifest are relative to the working
//! directory (the repo root).
//!
//! `kit-survey` shows what a region's scene is made of (its textures, how big they're drawn, the
//! pieces a kit could cut, numbered on a plan), in `out/overworld/survey/<region>` (`survey.rs`).
//!
//! `preview` builds a level and draws it (from the south-west and from above, side by side, with
//! the texture libraries and every region's kit) into a PNG: a quick look at a theme.
//!
//! `build` writes <out>/level.json (meshes with materials and collision surfaces), level.obj and
//! level.mtl, and copies the textures the level uses from the texture libraries `--textures` (PNGs
//! plus textures.json; by default every region's in `out/overworld/textures`) into <out>/textures.
//! The theme is the document's `settings.theme` unless `--theme` gives a file. Props come from the
//! kits `--kit` (by default every region's in `out/overworld/kit` that's been cut). Prints a
//! summary. Exit code 1 on errors.

use overworld::pieces::Kit;
use overworld::textures::Library;
use overworld::{build_with, export, Doc, Theme};
use std::path::{Path, PathBuf};

/// Where the regions' texture libraries go, one folder each.
const TEXTURES: &str = "out/overworld/textures";

fn fail(e: impl std::fmt::Display) -> ! {
    eprintln!("error: {e}");
    std::process::exit(1);
}

/// The regions named (each must be one), or all of them.
fn regions(named: &[String]) -> Vec<&'static str> {
    for n in named {
        if !overworld::kit::REGIONS.contains(&n.as_str()) {
            fail(format!("no region {n:?} (regions: {})", overworld::kit::REGIONS.join(", ")));
        }
    }
    overworld::kit::REGIONS.iter().copied().filter(|r| named.is_empty() || named.iter().any(|n| n == r)).collect()
}

/// Every region's texture library that's been made.
fn libraries() -> Vec<PathBuf> {
    overworld::kit::REGIONS.iter().map(|r| Path::new(TEXTURES).join(r)).filter(|d| d.join("textures.json").exists()).collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("kit-textures") {
        for r in regions(&args[2..]) {
            let sc = overworld::kit::load_scene(r).unwrap_or_else(|e| fail(e));
            let out = format!("{TEXTURES}/{r}");
            match overworld::kit::export(&sc, Path::new(&sc.glb), Path::new(&out)) {
                Ok(n) => println!("{}: {n} textures in {out}", sc.label),
                Err(e) => fail(format!("{}: {e}", sc.label)),
            }
        }
        return;
    }
    if args.get(1).map(String::as_str) == Some("kit-pieces") {
        let jobs: Vec<(PathBuf, PathBuf)> = if args.get(2).is_some_and(|a| a.ends_with(".json")) {
            vec![(PathBuf::from(&args[2]), PathBuf::from(args.get(3).unwrap_or_else(|| fail("kit-pieces <manifest.json> needs an out dir"))))]
        } else {
            regions(&args[2..]).into_iter().map(|r| (overworld::kit::manifest_path(r), overworld::pieces::kit_dir(Path::new("."), r))).collect()
        };
        let lib = Library::load_all(&libraries()).ok();
        for (manifest, out) in jobs {
            match overworld::pieces::export(&manifest, Path::new("."), &out) {
                Ok(kit) => {
                    if kit.pieces.is_empty() {
                        continue;
                    }
                    println!("{}:", kit.name);
                    let mut texels = overworld::thumb::Texels::default();
                    // thumbnails of pieces since dropped go with the old ones
                    let _ = std::fs::remove_dir_all(out.join("thumbs"));
                    let _ = std::fs::create_dir_all(out.join("thumbs"));
                    for p in &kit.pieces {
                        println!(
                            "  {:22} {:4} triangles, {:4} collision ({:4} vertices), {:.0} x {:.0} x {:.0}{}",
                            p.name,
                            p.tris.len(),
                            p.col_tris.len(),
                            p.collision_vertices(),
                            p.bounds[1][0] - p.bounds[0][0],
                            p.bounds[1][1] - p.bounds[0][1],
                            p.bounds[1][2] - p.bounds[0][2],
                            if p.functions.is_empty() { String::new() } else { format!(" [{}]", p.functions.join(", ")) }
                        );
                        if let Some(img) = overworld::thumb::render(p, lib.as_ref(), &mut texels, 192) {
                            let _ = std::fs::write(out.join("thumbs").join(format!("{}.png", p.name)), img.png());
                        }
                    }
                    println!("{} pieces in {}", kit.pieces.len(), out.join("pieces.json").display());
                }
                Err(e) => fail(e),
            }
        }
        // names must be unique across every region's kit
        if let Err(e) = overworld::pieces::load_regions(Path::new(".")) {
            fail(e);
        }
        return;
    }
    if args.get(1).map(String::as_str) == Some("kit-survey") {
        let region = args.get(2).unwrap_or_else(|| fail("kit-survey <region>"));
        regions(std::slice::from_ref(region));
        let zoom = args.iter().position(|a| a == "--box").map(|i| {
            let v: Vec<f64> = args.iter().skip(i + 1).take(4).map(|s| s.parse().unwrap_or_else(|_| fail("--box x0 y0 x1 y1"))).collect();
            if v.len() != 4 {
                fail("--box x0 y0 x1 y1");
            }
            [v[0].min(v[2]), v[1].min(v[3]), v[0].max(v[2]), v[1].max(v[3])]
        });
        let below = args.iter().position(|a| a == "--below").map(|i| args.get(i + 1).and_then(|s| s.parse::<f64>().ok()).unwrap_or_else(|| fail("--below z")));
        match overworld::survey::survey(region, Path::new("."), &Path::new("out/overworld/survey").join(region), zoom, below) {
            Ok(s) => println!("{s}"),
            Err(e) => fail(e),
        }
        return;
    }
    if args.get(1).map(String::as_str) == Some("preview") {
        let (Some(level), Some(png)) = (args.get(2), args.get(3)) else { fail("preview <level.json> <out.png> [--theme <theme.json>]") };
        let run = || -> Result<String, String> {
            let doc: Doc = serde_json::from_str(&std::fs::read_to_string(level).map_err(|e| format!("{level}: {e}"))?).map_err(|e| format!("{level}: {e}"))?;
            let file = args.iter().position(|a| a == "--theme").and_then(|i| args.get(i + 1));
            let theme = Theme::for_doc(&doc, file.map(|p| Theme::load(p)).transpose()?)?;
            let lib = Library::load_all(&libraries()).ok();
            let kit = overworld::pieces::load_regions(Path::new(".")).ok();
            let lvl = build_with(&doc, &theme, kit.as_ref())?;
            let piece = overworld::thumb::level_piece(&lvl, &theme);
            let mut tx = overworld::thumb::Texels::default();
            let side = 900;
            let mut img = overworld::thumb::Image::filled(side * 2, side, [40, 44, 52, 255]);
            if let Some(a) = overworld::thumb::render_view(&piece, lib.as_ref(), &mut tx, side, side, overworld::thumb::View::Angle { az: 215.0, el: 38.0 }) {
                img.draw(&a, 0, 0);
            }
            if let Some(b) = overworld::thumb::render_view(&piece, lib.as_ref(), &mut tx, side, side, overworld::thumb::View::Angle { az: 180.0, el: 89.9 }) {
                img.draw(&b, side as i64, 0);
            }
            std::fs::write(png, img.png()).map_err(|e| format!("{png}: {e}"))?;
            let missing: Vec<String> = lvl.mesh.materials.iter().map(|m| theme.texture_name(m)).filter(|n| lib.as_ref().is_none_or(|l| l.rgba(n).is_none())).collect();
            Ok(format!(
                "{png}: {} triangles, theme {}{}{}",
                lvl.mesh.triangles(),
                theme.name,
                if missing.is_empty() { String::new() } else { format!("; textures not in any library: {}", missing.join(", ")) },
                lvl.problems.iter().map(|p| format!("
  problem: {p}")).collect::<String>()
            ))
        };
        match run() {
            Ok(s) => println!("{s}"),
            Err(e) => fail(e),
        }
        return;
    }
    if args.len() < 4 || args[1] != "build" {
        eprintln!("usage: overworld build <level.json> <out dir> [--theme <theme.json>] [--textures <dir>]... [--kit <dir>]...");
        eprintln!("       overworld kit-textures [<region>...]");
        eprintln!("       overworld kit-pieces [<region>...]");
        eprintln!("       overworld kit-pieces <manifest.json> <out dir>");
        eprintln!("       overworld kit-survey <region> [--box <x0> <y0> <x1> <y1>] [--below <z>]");
        eprintln!("       overworld preview <level.json> <out.png> [--theme <theme.json>]");
        std::process::exit(2);
    }
    let opt = |name: &str| args.iter().position(|a| a == name).map(|i| args.get(i + 1).cloned().ok_or(format!("{name} needs a path")));
    let run = || -> Result<(), String> {
        let doc: Doc = serde_json::from_str(&std::fs::read_to_string(&args[2]).map_err(|e| format!("{}: {e}", args[2]))?)
            .map_err(|e| format!("{}: {e}", args[2]))?;
        let theme = Theme::for_doc(&doc, opt("--theme").map(|p| Theme::load(&p?)).transpose()?)?;
        let mut dirs: Vec<PathBuf> =
            args.iter().enumerate().filter(|(_, a)| *a == "--textures").filter_map(|(i, _)| args.get(i + 1)).map(PathBuf::from).collect();
        if dirs.is_empty() {
            dirs = libraries();
        }
        let lib = if dirs.is_empty() { None } else { Some(Library::load_all(&dirs)?) };
        let kits: Vec<PathBuf> = args.iter().enumerate().filter(|(_, a)| *a == "--kit").filter_map(|(i, _)| args.get(i + 1)).map(PathBuf::from).collect();
        let kit = if kits.is_empty() { overworld::pieces::load_regions(Path::new(".")).ok().filter(|k| !k.pieces.is_empty()) } else { Some(Kit::load_all(&kits)?) };
        let lvl = build_with(&doc, &theme, kit.as_ref())?;
        let problems = export::write(&doc, &theme, &lvl, lib.as_ref(), Path::new(&args[3]))?;
        println!(
            "{}: {} faces, {} triangles, {} collision vertices (of 8192), {} props, rim {:.0} to {:.0}",
            doc.name,
            lvl.faces,
            lvl.mesh.triangles(),
            lvl.collision_vertices,
            lvl.props.len(),
            lvl.rim.0,
            lvl.rim.1
        );
        for o in &lvl.mesh.objects {
            println!("  {:9} {:6} triangles", o.name, o.tris.len());
        }
        for p in &lvl.props {
            println!(
                "  prop {:2} {:16} at ({:.0}, {:.0}, {:.0}), turned {:.0}, scale {:.2} {:.2} {:.2}",
                p.index, p.piece, p.origin[0], p.origin[1], p.origin[2], p.yaw, p.scale[0], p.scale[1], p.scale[2]
            );
        }
        for p in &problems {
            println!("  problem: {p}");
        }
        Ok(())
    };
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
