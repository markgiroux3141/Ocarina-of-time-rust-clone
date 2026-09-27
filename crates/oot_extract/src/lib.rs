//! Extracts assets from the user's ROM into editable formats, for local development only.
//! Output goes to a git-ignored folder (default `extracted/`) and must never be committed
//! or distributed: it is derived from Nintendo's data.
//!
//! Every module exposes `extract(project, out_dir) -> Result<serde_json::Value>`, where the
//! returned value is a summary for `manifest.json`.

pub mod audio;
pub mod gltf;
pub mod models;
pub mod raw;
pub mod scenes;
pub mod text;
pub mod textures;

use std::path::{Path, PathBuf};

/// README written into the output folder.
pub const README: &str = include_str!("readme.md");

use anyhow::Result;

/// Directory for an asset file: `<out>/<kind>/<category>/<file>` where category is the
/// XML's folder under assets/xml (objects, scenes/dungeons, textures, ...).
pub fn asset_dir(out: &Path, kind: &str, file: &oot_core::symbols::AssetFile, xml_root: &Path) -> PathBuf {
    let rel = file.xml_path.strip_prefix(xml_root).ok().and_then(|p| p.parent()).map(|p| p.to_path_buf()).unwrap_or_default();
    out.join(kind).join(rel).join(sanitize(&file.name))
}

/// Makes a string safe to use as a file name.
pub fn sanitize(name: &str) -> String {
    name.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.') { c } else { '_' }).collect()
}

pub fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}

pub fn write_png(path: &Path, img: &oot_core::texture::DecodedImage) -> Result<()> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    image::save_buffer(path, &img.rgba, img.width, img.height, image::ColorType::Rgba8)?;
    Ok(())
}

pub fn xml_root(p: &oot_core::project::Project) -> PathBuf {
    p.config.decomp.join("assets").join("xml")
}
