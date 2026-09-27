//! Every texture named in the decomp XMLs, decoded to PNG, with a per-file `textures.json`
//! recording the N64 format, size, palette source and ROM offset so it can be re-encoded.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use oot_core::project::Project;
use oot_core::texture::{self, decode_linear};
use serde_json::json;

use crate::{asset_dir, write_json, write_png, xml_root};

pub fn parse_format(s: &str) -> Option<(u8, u8)> {
    use texture::*;
    Some(match s {
        "rgba16" => (G_IM_FMT_RGBA, G_IM_SIZ_16B),
        "rgba32" => (G_IM_FMT_RGBA, G_IM_SIZ_32B),
        "ci4" => (G_IM_FMT_CI, G_IM_SIZ_4B),
        "ci8" => (G_IM_FMT_CI, G_IM_SIZ_8B),
        "i4" => (G_IM_FMT_I, G_IM_SIZ_4B),
        "i8" => (G_IM_FMT_I, G_IM_SIZ_8B),
        "ia4" => (G_IM_FMT_IA, G_IM_SIZ_4B),
        "ia8" => (G_IM_FMT_IA, G_IM_SIZ_8B),
        "ia16" => (G_IM_FMT_IA, G_IM_SIZ_16B),
        _ => return None,
    })
}

fn hex_attr(v: Option<&str>) -> Option<usize> {
    let t = v?.trim();
    usize::from_str_radix(t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")).unwrap_or(t), 16).ok()
}

/// A grey ramp palette (RGBA16) for colour-indexed textures whose palette is set by code.
fn grey_ramp(entries: usize) -> Vec<u8> {
    (0..entries)
        .flat_map(|i| {
            let v = (i * 31 / (entries - 1).max(1)) as u16;
            ((v << 11) | (v << 6) | (v << 1) | 1).to_be_bytes()
        })
        .collect()
}

pub fn extract(p: &Project, out: &Path) -> Result<serde_json::Value> {
    let root = xml_root(p);
    let mut cache: HashMap<String, Option<Arc<[u8]>>> = HashMap::new();
    let mut file = |name: &str| -> Option<Arc<[u8]>> {
        cache.entry(name.to_string()).or_insert_with(|| p.rom.file_by_name(name).ok()).clone()
    };
    let (mut ok, mut failed, mut grey) = (0usize, 0usize, 0usize);
    let mut by_format: HashMap<String, usize> = HashMap::new();
    let mut errors = Vec::new();
    for f in &p.symbols.files {
        let texs: Vec<_> = f.of_kind("Texture").collect();
        if texs.is_empty() {
            continue;
        }
        let Some(data) = file(&f.name) else {
            errors.push(format!("{}: not in ROM", f.name));
            failed += texs.len();
            continue;
        };
        let dir = asset_dir(out, "textures", f, &root);
        let mut entries = Vec::new();
        for t in texs {
            let res = (|| -> Result<serde_json::Value> {
                let fmt_name = t.attr("Format").context("no Format")?;
                let (fmt, siz) = parse_format(fmt_name).with_context(|| format!("format {fmt_name}"))?;
                let w: u32 = t.attr("Width").context("no Width")?.parse()?;
                let h: u32 = t.attr("Height").context("no Height")?.parse()?;
                let off = t.offset as usize;
                let bytes = (w * h) as usize * texture::bits_per_texel(siz) / 8;
                if off + bytes > data.len() {
                    bail!("texture {:X}+{:X} outside {} ({:X})", off, bytes, f.name, data.len());
                }
                let mut palette = serde_json::Value::Null;
                let tlut = if fmt == texture::G_IM_FMT_CI {
                    let n = if siz == texture::G_IM_SIZ_4B { 16 } else { 256 };
                    let src = match (hex_attr(t.attr("TlutOffset")), t.attr("ExternalTlut"), hex_attr(t.attr("ExternalTlutOffset"))) {
                        (Some(o), _, _) => Some((f.name.clone(), data.clone(), o)),
                        (None, Some(ext), Some(o)) => file(ext).map(|d| (ext.to_string(), d, o)),
                        _ => None,
                    };
                    match src {
                        Some((pf, pd, o)) if o < pd.len() => {
                            palette = json!({ "file": pf, "offset": format!("0x{o:X}"), "entries": n });
                            let mut t = pd[o..(o + n * 2).min(pd.len())].to_vec();
                            t.resize(n * 2, 0);
                            t
                        }
                        _ => {
                            grey += 1;
                            palette = json!("missing: set at runtime; exported as a grey ramp of palette indices");
                            grey_ramp(n)
                        }
                    }
                } else {
                    Vec::new()
                };
                let img = decode_linear(&data[off..off + bytes], fmt, siz, w, h, (!tlut.is_empty()).then_some(&tlut[..]));
                let png = format!("{}.png", crate::sanitize(&t.name));
                write_png(&dir.join(&png), &img)?;
                *by_format.entry(fmt_name.to_string()).or_default() += 1;
                Ok(json!({
                    "name": t.name,
                    "png": png,
                    "format": fmt_name,
                    "width": w,
                    "height": h,
                    "offset": format!("0x{off:X}"),
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
