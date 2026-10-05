//! The Kokiri Forest texture library, made from the clone's extracted scene
//! (`extracted/scenes/overworld/spot04/spot04.glb`, written by `oot_extract`): each render
//! material's image as `kf_<role>.png`, and `textures.json` with its wrap per axis (the glb's
//! samplers), alpha (`n64_blend`), translucent opacity (`baseColorFactor[3]`) and culling
//! (`n64_cull`). The decomp's spot04 XML names no textures, so the glb is the only source.
//!
//! Materials are named `room_<r>_<opa|xlu>_mat<N>`; N picks the role from `KOKIRI_ROLES`, which
//! was read off the textures and where the scene uses them. N follows the extractor's order, so
//! the roles the Kokiri theme uses are checked against their known size and wrap: if a new
//! extraction reorders them, the export fails instead of mixing textures up.

use crate::textures::TexInfo;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

/// spot04's material N -> what its texture is.
pub const KOKIRI_ROLES: [&str; 52] = [
    "house_bark", "house_door_dark", "stump_top", "door_curtain", "porch_floor", "porch_trim", "porch_under",
    "hanging_vines", "porch_rail", "ladder", "ladder_back", "mushroom", "door_frame", "mido_bark", "log_bark",
    "tunnel_mouth", "ground", "forest_trunks", "forest_foliage", "cliff", "cliff_strip", "cliff_strip_dark",
    "grass_skirt", "hedge_top", "fence", "fence_post", "log_end", "log_side", "roof_leaf", "hanging_roots",
    "tunnel_ring", "stone_top", "stone_side", "post_bark", "saria_bark", "shop_bark", "shop_sign", "deku_bark",
    "deku_leaves", "shadow", "shadow_link", "graffiti_a", "graffiti_b", "graffiti_c", "water", "water_foam",
    "water_ripple", "water_ripple_b", "vines", "deku_face", "shadow_deku", "water_ripple_c",
];

/// The roles the Kokiri theme draws with: (role, width, height, wrap u, wrap v).
const CHECKED: [(&str, u32, u32, &str, &str); 9] = [
    ("ground", 32, 32, "repeat", "repeat"),
    ("forest_trunks", 64, 64, "repeat", "clamp"),
    ("forest_foliage", 32, 32, "repeat", "clamp"),
    ("cliff", 32, 32, "repeat", "repeat"),
    ("cliff_strip", 32, 16, "repeat", "repeat"),
    ("cliff_strip_dark", 32, 16, "repeat", "repeat"),
    ("grass_skirt", 64, 16, "repeat", "clamp"),
    ("hanging_roots", 64, 32, "repeat", "clamp"),
    ("water", 32, 32, "repeat", "repeat"),
];

/// A glb's JSON and binary chunk.
pub fn read_glb(bytes: &[u8]) -> Result<(Value, &[u8]), String> {
    let u32_at = |i: usize| -> Result<usize, String> {
        bytes.get(i..i + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize).ok_or("glb: truncated".to_string())
    };
    if bytes.get(0..4) != Some(b"glTF") {
        return Err("not a glb".into());
    }
    let jlen = u32_at(12)?;
    if bytes.get(16..20) != Some(b"JSON") {
        return Err("glb: the first chunk isn't JSON".into());
    }
    let json: Value = serde_json::from_slice(bytes.get(20..20 + jlen).ok_or("glb: truncated JSON")?).map_err(|e| format!("glb JSON: {e}"))?;
    let b0 = 20 + jlen;
    let blen = u32_at(b0)?;
    if bytes.get(b0 + 4..b0 + 8) != Some(b"BIN\0") {
        return Err("glb: no binary chunk".into());
    }
    Ok((json, bytes.get(b0 + 8..b0 + 8 + blen).ok_or("glb: truncated binary chunk")?))
}

fn wrap(code: Option<u64>) -> &'static str {
    match code {
        Some(33648) => "mirror",
        Some(33071) => "clamp",
        _ => "repeat",
    }
}

/// Writes the library into `out` from spot04's glb. Returns the number of textures.
pub fn export_kokiri(glb: &Path, out: &Path) -> Result<usize, String> {
    let bytes = std::fs::read(glb).map_err(|e| format!("{}: {e}", glb.display()))?;
    let (j, bin) = read_glb(&bytes)?;
    let arr = |k: &str| j[k].as_array().cloned().unwrap_or_default();
    let (mats, texs, samplers, images, views) = (arr("materials"), arr("textures"), arr("samplers"), arr("images"), arr("bufferViews"));
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut lib: BTreeMap<String, TexInfo> = BTreeMap::new();
    for m in &mats {
        let ex = &m["extras"];
        let Some(blend) = ex["n64_blend"].as_str() else { continue }; // collision materials have none
        let name = m["name"].as_str().unwrap_or("");
        let Some(n) = name.rsplit("mat").next().and_then(|s| s.parse::<usize>().ok()) else { continue };
        let Some(role) = KOKIRI_ROLES.get(n) else { continue };
        let pbr = &m["pbrMetallicRoughness"];
        let Some(ti) = pbr["baseColorTexture"]["index"].as_u64() else { continue };
        let t = &texs[ti as usize];
        let s = t["sampler"].as_u64().map(|i| &samplers[i as usize]);
        let img = &images[t["source"].as_u64().ok_or(format!("{name}: texture without an image"))? as usize];
        let v = &views[img["bufferView"].as_u64().ok_or(format!("{name}: image without a buffer view"))? as usize];
        let (off, len) = (v["byteOffset"].as_u64().unwrap_or(0) as usize, v["byteLength"].as_u64().unwrap_or(0) as usize);
        let png = bin.get(off..off + len).ok_or(format!("{name}: image outside the binary chunk"))?;
        if png.get(1..4) != Some(b"PNG") {
            return Err(format!("{name}: image isn't a PNG"));
        }
        let (w, h) = (u32::from_be_bytes([png[16], png[17], png[18], png[19]]), u32::from_be_bytes([png[20], png[21], png[22], png[23]]));
        let alpha = if blend.starts_with("Cutout") {
            "cutout"
        } else if blend == "Translucent" {
            "blend"
        } else {
            "opaque"
        };
        let opacity = if alpha == "blend" { pbr["baseColorFactor"][3].as_f64().unwrap_or(1.0) } else { 1.0 };
        let info = TexInfo {
            file: format!("kf_{role}.png"),
            size: vec![w, h],
            wrap_u: wrap(s.and_then(|s| s["wrapS"].as_u64())).into(),
            wrap_v: wrap(s.and_then(|s| s["wrapT"].as_u64())).into(),
            alpha: alpha.into(),
            opacity: (opacity * 10000.0).round() / 10000.0,
            cull: if ex["n64_cull"].as_str() == Some("None") { "none".into() } else { "back".into() },
        };
        std::fs::write(out.join(&info.file), png).map_err(|e| format!("{}: {e}", out.display()))?;
        lib.insert(format!("kf_{role}"), info);
    }
    for (role, w, h, wu, wv) in CHECKED {
        let t = lib.get(&format!("kf_{role}")).ok_or(format!("spot04 has no material for {role}: has the extraction changed?"))?;
        if t.size != [w, h] || t.wrap_u != wu || t.wrap_v != wv {
            return Err(format!(
                "kf_{role} is {:?} {}/{}, expected [{w}, {h}] {wu}/{wv}: the extraction's material order changed, so KOKIRI_ROLES needs updating",
                t.size, t.wrap_u, t.wrap_v
            ));
        }
    }
    let json = serde_json::to_string_pretty(&lib).map_err(|e| e.to_string())?;
    std::fs::write(out.join("textures.json"), json).map_err(|e| format!("{}: {e}", out.display()))?;
    Ok(lib.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs the clone's extract (git-ignored ROM data): skipped without it.
    #[test]
    fn the_kokiri_library_comes_out_of_the_extracted_scene() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let glb = root.join("extracted/scenes/overworld/spot04/spot04.glb");
        if !glb.exists() {
            eprintln!("skipping: no {}", glb.display());
            return;
        }
        let out = std::env::temp_dir().join(format!("ow_kit_test_{}", std::process::id()));
        let n = export_kokiri(&glb, &out).unwrap();
        assert_eq!(n, KOKIRI_ROLES.len());
        let lib = crate::textures::Library::load(&out).unwrap();
        assert_eq!(lib.get("kf_water").alpha, "blend");
        assert!((lib.get("kf_water").opacity - 0.3882).abs() < 1e-3);
        assert_eq!(lib.get("kf_forest_foliage").alpha, "cutout");
        // the old Blender export read these as clamp; the glb says mirror
        assert_eq!((lib.get("kf_stone_top").wrap_u.as_str(), lib.get("kf_stone_top").wrap_v.as_str()), ("mirror", "mirror"));
        assert_eq!(lib.rgba("kf_cliff").unwrap().0, 32);
        let _ = std::fs::remove_dir_all(&out);
    }
}
