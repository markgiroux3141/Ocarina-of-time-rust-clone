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

/// spot04's material N -> what its texture is. The dirt path is decals (45 to 47, and 51 in the
/// training area): white blotches with soft alpha, tinted yellow-brown by the combiner's prim.
pub const KOKIRI_ROLES: [&str; 52] = [
    "house_bark", "house_door_dark", "stump_top", "door_curtain", "porch_floor", "porch_trim", "porch_under",
    "hanging_vines", "porch_rail", "ladder", "ladder_back", "mushroom", "door_frame", "mido_bark", "log_bark",
    "tunnel_mouth", "ground", "forest_trunks", "forest_foliage", "cliff", "cliff_strip", "cliff_strip_dark",
    "grass_skirt", "hedge_top", "fence", "fence_post", "log_end", "log_side", "roof_leaf", "hanging_roots",
    "tunnel_ring", "stone_top", "stone_side", "post_bark", "saria_bark", "shop_bark", "shop_sign", "deku_bark",
    "deku_leaves", "shadow", "shadow_link", "graffiti_a", "graffiti_b", "graffiti_c", "water", "dirt_end",
    "dirt_strip", "dirt_junction", "vines", "deku_face", "shadow_deku", "dirt_patch",
];

/// The roles the Kokiri theme draws with: (role, width, height, wrap u, wrap v).
const CHECKED: [(&str, u32, u32, &str, &str); 10] = [
    ("ground", 32, 32, "repeat", "repeat"),
    ("forest_trunks", 64, 64, "repeat", "clamp"),
    ("forest_foliage", 32, 32, "repeat", "clamp"),
    ("cliff", 32, 32, "repeat", "repeat"),
    ("cliff_strip", 32, 16, "repeat", "repeat"),
    ("cliff_strip_dark", 32, 16, "repeat", "repeat"),
    ("grass_skirt", 64, 16, "repeat", "clamp"),
    ("hanging_roots", 64, 32, "repeat", "clamp"),
    ("water", 32, 32, "repeat", "repeat"),
    ("dirt_strip", 64, 64, "clamp", "repeat"),
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
            decal: ex["n64_decal"].as_bool() == Some(true),
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
    ground_with_detail(&j, bin, &mats, &texs, &images, &views, out, &mut lib)?;
    let json = serde_json::to_string_pretty(&lib).map_err(|e| e.to_string())?;
    std::fs::write(out.join("textures.json"), json).map_err(|e| format!("{}: {e}", out.display()))?;
    Ok(lib.len())
}

/// Kokiri's ground as the game draws it. Its material mixes two textures half and half,
/// (TEXEL1 - TEXEL0) x ENV_ALPHA (0x80) + TEXEL0: the camo, one repeat every 400 units, and a
/// detail texture on tile 1, eight times finer (its own UVs, TEXCOORD_1). The library's `kf_ground`
/// is that mix baked into one tile: the camo filtered up to the detail's resolution with the detail
/// repeating across it, so a floor textured once per 400 units looks as the game's does.
#[allow(clippy::too_many_arguments)]
fn ground_with_detail(j: &Value, bin: &[u8], mats: &[Value], texs: &[Value], images: &[Value], views: &[Value], out: &Path, lib: &mut BTreeMap<String, TexInfo>) -> Result<(), String> {
    let Some((mi, m)) = mats.iter().enumerate().find(|(_, m)| {
        m["name"].as_str().and_then(|n| n.rsplit("mat").next()).and_then(|s| s.parse::<usize>().ok()).and_then(|n| KOKIRI_ROLES.get(n)) == Some(&"ground")
    }) else {
        return Err("spot04 has no ground material".into());
    };
    let ex = &m["extras"];
    let base = m["pbrMetallicRoughness"]["baseColorTexture"]["index"].as_u64().ok_or("the ground has no texture")? as usize;
    // tile 1's texture: recorded by newer extractions; else the one the extractor added right after
    // tile 0's (it adds a material's textures in turn), which no material uses as its own
    let t1 = match ex["n64_texture1"].as_u64() {
        Some(i) => i as usize,
        None => {
            let used: Vec<u64> = mats.iter().filter_map(|m| m["pbrMetallicRoughness"]["baseColorTexture"]["index"].as_u64()).collect();
            if used.contains(&(base as u64 + 1)) || base + 1 >= texs.len() {
                return Err("can't tell the ground's detail texture: re-extract spot04".into());
            }
            base + 1
        }
    };
    let png = |ti: usize| -> Result<(u32, u32, Vec<u8>), String> {
        let img = &images[texs[ti]["source"].as_u64().ok_or("texture without an image")? as usize];
        let v = &views[img["bufferView"].as_u64().ok_or("image without a buffer view")? as usize];
        let (off, len) = (v["byteOffset"].as_u64().unwrap_or(0) as usize, v["byteLength"].as_u64().unwrap_or(0) as usize);
        crate::textures::decode_png(bin.get(off..off + len).ok_or("image outside the binary chunk")?).ok_or("bad PNG".into())
    };
    // how much finer tile 1's UVs run than tile 0's, from a primitive using the material
    let mut ratio = None;
    'find: for node in j["nodes"].as_array().cloned().unwrap_or_default() {
        let Some(mesh) = node["mesh"].as_u64() else { continue };
        for prim in j["meshes"][mesh as usize]["primitives"].as_array().cloned().unwrap_or_default() {
            if prim["material"].as_u64() != Some(mi as u64) {
                continue;
            }
            let at = &prim["attributes"];
            let (Some(a0), Some(a1)) = (at["TEXCOORD_0"].as_u64(), at["TEXCOORD_1"].as_u64()) else { continue };
            let (u0, u1) = (crate::pieces::accessor(j, bin, a0 as usize, 2)?, crate::pieces::accessor(j, bin, a1 as usize, 2)?);
            let span = |u: &[f64]| {
                let xs = u.iter().step_by(2);
                xs.clone().cloned().fold(f64::NEG_INFINITY, f64::max) - xs.cloned().fold(f64::INFINITY, f64::min)
            };
            if span(&u0) > 0.5 {
                ratio = Some((span(&u1) / span(&u0)).round() as u32);
                break 'find;
            }
        }
    }
    let k = ratio.filter(|&r| (1..=16).contains(&r)).ok_or("can't tell the ground's detail scale")?;
    let a = u8::from_str_radix(ex["n64_env"].as_str().unwrap_or("#80808080").get(7..9).unwrap_or("80"), 16).unwrap_or(0x80) as f64 / 255.0;
    let ((w0, h0, p0), (w1, h1, p1)) = (png(base)?, png(t1)?);
    let (w, h) = (w1 * k, h1 * k);
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            // the camo filtered up (bilinear, repeating), the detail texel for texel
            let fx = (x as f64 + 0.5) * w0 as f64 / w as f64 - 0.5;
            let fy = (y as f64 + 0.5) * h0 as f64 / h as f64 - 0.5;
            let (ix, iy) = (fx.floor(), fy.floor());
            let (tx, ty) = (fx - ix, fy - iy);
            let at = |i: i64, j: i64, c: usize| p0[((j.rem_euclid(h0 as i64) as u32 * w0 + i.rem_euclid(w0 as i64) as u32) * 4) as usize + c] as f64;
            let d = ((y % h1) * w1 + x % w1) as usize * 4;
            for c in 0..3 {
                let (i, jj) = (ix as i64, iy as i64);
                let c0 = (at(i, jj, c) * (1.0 - tx) + at(i + 1, jj, c) * tx) * (1.0 - ty) + (at(i, jj + 1, c) * (1.0 - tx) + at(i + 1, jj + 1, c) * tx) * ty;
                px.push((c0 + (p1[d + c] as f64 - c0) * a).round().clamp(0.0, 255.0) as u8);
            }
            px.push(255);
        }
    }
    let info = lib.get_mut("kf_ground").ok_or("no kf_ground")?;
    std::fs::write(out.join(&info.file), crate::textures::encode_png(w, h, &px)).map_err(|e| format!("{}: {e}", out.display()))?;
    info.size = vec![w, h];
    Ok(())
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
        // the ground is its camo with the detail texture, eight times finer, mixed in
        assert_eq!(lib.get("kf_ground").size, vec![256, 256]);
        // the dirt path's decals and the door shadows are drawn as decals; the ground isn't
        assert!(lib.get("kf_dirt_strip").decal && lib.get("kf_shadow").decal && !lib.get("kf_ground").decal);
        let _ = std::fs::remove_dir_all(&out);
    }
}
