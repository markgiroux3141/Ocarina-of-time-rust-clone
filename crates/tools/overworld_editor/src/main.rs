//! overworld-editor [level.json] [--theme <theme.json>] [--textures <dir>] [--select <region or path>]
//!
//! Draw an overworld level's outline, regions and paths over a plan of the built level, which
//! rebuilds in the background as you edit. Walk it in pd-walk from the editor. See README.md.

mod app;
mod edit;
mod profile;
mod scene;
mod view3d;
mod worker;

use std::path::PathBuf;

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut opts = app::Options { doc: None, theme: None, textures: None, select: None };
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--theme" => opts.theme = it.next().map(PathBuf::from),
            "--textures" => opts.textures = it.next().map(PathBuf::from),
            "--select" => opts.select = it.next(),
            "-h" | "--help" => {
                println!("usage: overworld-editor [level.json] [--theme <theme.json>] [--textures <dir>] [--select <region or path>]");
                return Ok(());
            }
            _ => opts.doc = Some(PathBuf::from(a)),
        }
    }
    let native = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([1500.0, 950.0]).with_title("Overworld editor"),
        ..Default::default()
    };
    eframe::run_native("Overworld editor", native, Box::new(|cc| Ok(Box::new(app::App::new(cc, opts)))))
}
