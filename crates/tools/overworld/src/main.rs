//! overworld build <level.json> <out dir> [--theme <theme.json>] [--textures <dir>]
//! overworld kit-textures [<spot04.glb>] [<out dir>]
//!
//! `kit-textures` makes the Kokiri texture library (`kit.rs`) from the clone's extracted scene,
//! by default `extracted/scenes/overworld/spot04/spot04.glb` into `out/overworld/textures/kokiri`.
//!
//! `build` writes <out>/level.json (meshes with materials and collision surfaces), level.obj and
//! level.mtl, and copies the textures the level uses from the texture library `--textures`
//! (PNGs plus textures.json) into <out>/textures. Prints a summary. Exit code 1 on errors.

use overworld::textures::Library;
use overworld::{build, export, Doc, Theme};
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
    if args.len() < 4 || args[1] != "build" {
        eprintln!("usage: overworld build <level.json> <out dir> [--theme <theme.json>] [--textures <dir>]");
        eprintln!("       overworld kit-textures [<spot04.glb>] [<out dir>]");
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
        let lvl = build(&doc, &theme)?;
        let problems = export::write(&doc, &theme, &lvl, lib.as_ref(), Path::new(&args[3]))?;
        println!("{}: {} faces, {} triangles, rim {:.0} to {:.0}", doc.name, lvl.faces, lvl.mesh.triangles(), lvl.rim.0, lvl.rim.1);
        for o in &lvl.mesh.objects {
            println!("  {:9} {:6} triangles", o.name, o.tris.len());
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
