//! overworld build <level.json> <out dir> [--theme <theme.json>] [--textures <dir>] [--kit <dir>]
//! overworld kit-textures [<spot04.glb>] [<out dir>]
//! overworld kit-pieces [<manifest.json>] [<out dir>]
//!
//! `kit-textures` makes the Kokiri texture library (`kit.rs`) from the clone's extracted scene,
//! by default `extracted/scenes/overworld/spot04/spot04.glb` into `out/overworld/textures/kokiri`.
//!
//! `kit-pieces` cuts the pieces of a kit manifest (`kit/kokiri.json`: houses, stumps, stones, the log
//! tunnel, the crawlspace...) out of the extract it names, by default into `out/overworld/kit/kokiri`
//! (`pieces.rs`). Paths in the manifest are relative to the working directory (the repo root).
//!
//! `build` writes <out>/level.json (meshes with materials and collision surfaces), level.obj and
//! level.mtl, and copies the textures the level uses from the texture library `--textures`
//! (PNGs plus textures.json) into <out>/textures. Props come from the kit `--kit` (by default
//! `out/overworld/kit/kokiri`, if it's there). Prints a summary. Exit code 1 on errors.

use overworld::textures::Library;
use overworld::pieces::Kit;
use overworld::{build_with, export, Doc, Theme};
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("kit-textures") {
        let glb = args.get(2).cloned().unwrap_or("extracted/scenes/overworld/spot04/spot04.glb".into());
        let out = args.get(3).cloned().unwrap_or("out/overworld/textures/kokiri".into());
        match overworld::kit::export_kokiri(Path::new(&glb), Path::new(&out)) {
            Ok(n) => println!("{n} textures in {out}"),
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
        return;
    }
    if args.get(1).map(String::as_str) == Some("kit-pieces") {
        let manifest = args.get(2).map(std::path::PathBuf::from).unwrap_or(overworld::pieces::kokiri_manifest().to_path_buf());
        let out = args.get(3).cloned().unwrap_or("out/overworld/kit/kokiri".into());
        match overworld::pieces::export(&manifest, Path::new("."), Path::new(&out)) {
            Ok(kit) => {
                for p in &kit.pieces {
                    println!(
                        "  {:16} {:4} triangles, {:4} collision ({:4} vertices), {:.0} x {:.0} x {:.0}{}",
                        p.name,
                        p.tris.len(),
                        p.col_tris.len(),
                        p.collision_vertices(),
                        p.bounds[1][0] - p.bounds[0][0],
                        p.bounds[1][1] - p.bounds[0][1],
                        p.bounds[1][2] - p.bounds[0][2],
                        if p.functions.is_empty() { String::new() } else { format!(" [{}]", p.functions.join(", ")) }
                    );
                }
                println!("{} pieces in {out}/pieces.json", kit.pieces.len());
            }
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
        return;
    }
    if args.len() < 4 || args[1] != "build" {
        eprintln!("usage: overworld build <level.json> <out dir> [--theme <theme.json>] [--textures <dir>] [--kit <dir>]");
        eprintln!("       overworld kit-textures [<spot04.glb>] [<out dir>]");
        eprintln!("       overworld kit-pieces [<manifest.json>] [<out dir>]");
        std::process::exit(2);
    }
    let opt = |name: &str| args.iter().position(|a| a == name).map(|i| args.get(i + 1).cloned().ok_or(format!("{name} needs a path")));
    let run = || -> Result<(), String> {
        let doc: Doc = serde_json::from_str(&std::fs::read_to_string(&args[2]).map_err(|e| format!("{}: {e}", args[2]))?)
            .map_err(|e| format!("{}: {e}", args[2]))?;
        let theme = match opt("--theme") {
            Some(p) => Theme::load(&p?)?,
            None => Theme::kokiri(),
        };
        let lib = match opt("--textures") {
            Some(p) => Some(Library::load(Path::new(&p?))?),
            None => None,
        };
        let kit = match opt("--kit") {
            Some(p) => Some(Kit::load(Path::new(&p?))?),
            None => Kit::load(Path::new("out/overworld/kit/kokiri")).ok(),
        };
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
