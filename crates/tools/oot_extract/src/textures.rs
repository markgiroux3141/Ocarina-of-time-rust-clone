//! Every texture named in the decomp XMLs, decoded to PNG, with a per-file `textures.json`
//! recording the N64 format, size, palette source and ROM offset so it can be re-encoded.

use std::collections::HashMap;
use std::path::Path;

pub use oot_import::objects::parse_format;
use anyhow::{Context, Result};
use oot_import::objects::{self, Palette};
use oot_import::project::Project;
use serde_json::json;

use crate::{asset_dir, write_json, write_png, xml_root};

pub fn extract(p: &Project, out: &Path) -> Result<serde_json::Value> {
    let root = xml_root(p);
    let files = objects::Files::new(p);
    let (mut ok, mut failed, mut grey) = (0usize, 0usize, 0usize);
    let mut by_format: HashMap<String, usize> = HashMap::new();
    let mut errors = Vec::new();
    for f in &p.symbols.files {
        let texs: Vec<_> = f.of_kind("Texture").collect();
        if texs.is_empty() {
            continue;
        }
        let Some(data) = files.get(&f.name) else {
            errors.push(format!("{}: not in ROM", f.name));
            failed += texs.len();
            continue;
        };
        let dir = asset_dir(out, "textures", f, &root);
        let mut entries = Vec::new();
        for t in texs {
            let res = (|| -> Result<serde_json::Value> {
                let fmt_name = t.attr("Format").context("no Format")?;
                let d = objects::decode_texture(f, &data, t, &files)?;
                let palette = match &d.palette {
                    Palette::None => serde_json::Value::Null,
                    Palette::File { file, offset, entries } => json!({ "file": file, "offset": format!("0x{offset:X}"), "entries": entries }),
                    Palette::Code => {
                        grey += 1;
                        json!("missing: set at runtime; exported as a grey ramp of palette indices")
                    }
                };
                let png = format!("{}.png", crate::sanitize(&t.name));
                write_png(&dir.join(&png), &d.image)?;
                *by_format.entry(fmt_name.to_string()).or_default() += 1;
                Ok(json!({
                    "name": t.name,
                    "png": png,
                    "format": fmt_name,
                    "width": d.width,
                    "height": d.height,
                    "offset": format!("0x{:X}", t.offset),
                    "palette": palette,
                }))
            })();
            match res {
                Ok(e) => {
                    ok += 1;
                    entries.push(e);
                }
                Err(e) => {
                    failed += 1;
                    errors.push(format!("{} / {}: {e:#}", f.name, t.name));
                }
            }
        }
        if !entries.is_empty() {
            write_json(&dir.join("textures.json"), &json!({ "file": f.name, "segment": f.segment, "textures": entries }))?;
        }
    }
    errors.truncate(50);
    Ok(json!({ "textures": ok, "failed": failed, "grey_ramp_palettes": grey, "by_format": by_format, "errors": errors }))
}
