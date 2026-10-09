//! The texture libraries, one per region, made from the clone's extracted scenes
//! (`extracted/scenes/overworld/<scene>/<scene>.glb`, written by `oot_extract`): each render
//! material's image as `<prefix><role>.png`, and `textures.json` with its wrap per axis (the glb's
//! samplers), alpha (`n64_blend`), translucent opacity (`baseColorFactor[3]`) and culling
//! (`n64_cull`). The decomp's scene XMLs name no textures, so the glb is the only source. Each
//! region has its own prefix (Kokiri Forest `kf_`, Kakariko `kak_`...), so the libraries never
//! clash and a level can draw from several.
//!
//! The regions (`REGIONS`) are the overworld scenes, each described by its kit manifest
//! (`kit/<region>.json`): its label, its source, its pieces (`pieces.rs`) and its `textures`:
//! materials are named `room_<r>_<opa|xlu>_mat<N>`; N picks the role from `roles`, which were read
//! off the textures and where the scene uses them (a material without one is `mat<N>`). N follows
//! the extractor's order, so the roles a theme uses are `checked` against their known size and
//! wrap: if a new extraction reorders them, the export fails instead of mixing textures up.
//! The Market Entrance (`entra`) has no region: it's drawn from prerendered backgrounds.

use crate::textures::TexInfo;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The overworld regions, in the order the editor lists them: each a kit manifest
/// `kit/<region>.json` (its scene is the manifest's source).
pub const REGIONS: [&str; 20] = [
    "kokiri",
    "lost_woods",
    "sacred_meadow",
    "hyrule_field",
    "lon_lon",
    "lon_lon_buildings",
    "hyrule_castle",
    "kakariko",
    "graveyard",
    "dm_trail",
    "dm_crater",
    "goron_city",
    "zora_river",
    "zora_domain",
    "zora_fountain",
    "lake_hylia",
    "gerudo_valley",
    "gerudo_fortress",
    "wasteland",
    "colossus",
];

/// Where the kit manifests are.
pub fn manifest_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("kit")
}

/// A region's kit manifest.
pub fn manifest_path(region: &str) -> PathBuf {
    manifest_dir().join(format!("{region}.json"))
}

/// A region's texture library: where it comes from, its prefix, its material roles, the roles its
/// theme draws with (role, width, height, wrap u, wrap v), checked on every export, and the roles
/// drawn as the camo mixed with a finer detail texture (baked into one, `ground_with_detail`).
#[derive(Debug, Clone)]
pub struct SceneTextures {
    /// The region (and its theme's name).
    pub theme: String,
    pub label: String,
    pub glb: String,
    pub prefix: String,
    pub roles: Vec<String>,
    pub checked: Vec<(String, u32, u32, String, String)>,
    pub detailed: Vec<String>,
    /// Roles drawn opaque whatever the scene's blend says: intensity-alpha textures whose alpha
    /// is their brightness, which the extract marks cut-out (Hyrule Castle's bricks).
    pub opaque: Vec<String>,
}

#[derive(Deserialize)]
struct ManifestHead {
    name: String,
    label: String,
    source: crate::pieces::Source,
    textures: TexturesDef,
}

/// A manifest's `textures`.
#[derive(Debug, Clone, Deserialize)]
pub struct TexturesDef {
    pub prefix: String,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub checked: Vec<(String, u32, u32, String, String)>,
    #[serde(default)]
    pub detailed: Vec<String>,
    #[serde(default)]
    pub opaque: Vec<String>,
}

impl SceneTextures {
    /// Material N's role: its name in `roles`, or `mat<N>`.
    pub fn role(&self, n: usize) -> String {
        match self.roles.get(n).filter(|r| !r.is_empty()) {
            Some(r) => r.clone(),
            None => format!("mat{n}"),
        }
    }

    /// The library's name for a role.
    pub fn texture(&self, role: &str) -> String {
        format!("{}{role}", self.prefix)
    }
}

/// A region's texture library, read from its manifest now (`scenes` reads them once).
pub fn load_scene(region: &str) -> Result<SceneTextures, String> {
    let p = manifest_path(region);
    let s = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    let m: ManifestHead = serde_json::from_str(&s).map_err(|e| format!("{}: {e}", p.display()))?;
    if m.name != region {
        return Err(format!("{}: its name is {:?}", p.display(), m.name));
    }
    let mut seen = std::collections::BTreeSet::new();
    for r in m.textures.roles.iter().filter(|r| !r.is_empty()) {
        if !seen.insert(r) {
            return Err(format!("{}: role {r:?} is given to two materials", p.display()));
        }
    }
    Ok(SceneTextures {
        theme: m.name,
        label: m.label,
        glb: m.source.glb,
        prefix: m.textures.prefix,
        roles: m.textures.roles,
        checked: m.textures.checked,
        detailed: m.textures.detailed,
        opaque: m.textures.opaque,
    })
}

/// Every region whose manifest reads, in `REGIONS` order (read once; one that doesn't read is
/// left out, with a message on stderr).
pub fn scenes() -> &'static [SceneTextures] {
    static SCENES: OnceLock<Vec<SceneTextures>> = OnceLock::new();
    SCENES.get_or_init(|| {
        REGIONS
            .iter()
            .filter_map(|r| match load_scene(r) {
                Ok(sc) => Some(sc),
                Err(e) => {
                    eprintln!("warning: region {r}: {e}");
                    None
                }
            })
            .collect()
    })
}

/// The region whose library a theme draws from.
pub fn scene(theme: &str) -> Option<&'static SceneTextures> {
    scenes().iter().find(|s| s.theme == theme)
}

/// A region's label ("Kakariko Village"), or the region's own name if its manifest doesn't read.
pub fn label(region: &str) -> String {
    scene(region).map_or(region.to_string(), |s| s.label.clone())
}

/// The collision role a scene's surface type has unless its manifest says otherwise: an exit
/// (doorways too: a manifest calls a house's `door`), a wall type's (ladders, vines, crawlspaces,
/// ledges Link may not grab), a pit (FLOOR_PROPERTY_12), else its footstep sound's.
pub fn default_role(st: &Value) -> &'static str {
    let n = |k: &str| st[k].as_u64().unwrap_or(0);
    if n("exit_index") > 0 {
        return "exit";
    }
    match n("wall_type") {
        1 => return "wall_nograb",
        2 => return "ladder",
        3 => return "ladder_top",
        4 => return "vines",
        5 => return "crawl",
        _ => {}
    }
    if n("floor_property") == 12 {
        return "void";
    }
    // SURFACE_SFX_OFFSET_*: dirt, sand, stone, jabu, shallow water, deep water, tall grass, lava,
    // grass, bridge, wood, soft dirt, ice, carpet
    match n("sfx_type") {
        0 | 7 | 11 => "dirt",
        1 => "sand",
        2 | 3 | 12 => "stone",
        6 => "tall_grass",
        9 => "planks",
        10 | 13 => "wood",
        _ => "ground",
    }
}

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

/// Writes a scene's library into `out` from its glb. Returns the number of textures.
pub fn export(sc: &SceneTextures, glb: &Path, out: &Path) -> Result<usize, String> {
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
        let role = sc.role(n);
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
        let forced = sc.opaque.contains(&role);
        let alpha = if forced {
            "opaque"
        } else if blend.starts_with("Cutout") {
            "cutout"
        } else if blend == "Translucent" {
            "blend"
        } else {
            "opaque"
        };
        let opacity = if alpha == "blend" { pbr["baseColorFactor"][3].as_f64().unwrap_or(1.0) } else { 1.0 };
        let info = TexInfo {
            file: format!("{}{role}.png", sc.prefix),
            size: vec![w, h],
            wrap_u: wrap(s.and_then(|s| s["wrapS"].as_u64())).into(),
            wrap_v: wrap(s.and_then(|s| s["wrapT"].as_u64())).into(),
            alpha: alpha.into(),
            opacity: (opacity * 10000.0).round() / 10000.0,
            cull: if ex["n64_cull"].as_str() == Some("None") { "none".into() } else { "back".into() },
            decal: ex["n64_decal"].as_bool() == Some(true),
        };
        // forced opaque: the alpha goes too, so nothing that reads the PNG cuts it out
        let bytes = match forced.then(|| crate::textures::decode_png(png)).flatten() {
            Some((w, h, mut px)) => {
                px.chunks_mut(4).for_each(|c| c[3] = 255);
                crate::textures::encode_png(w, h, &px)
            }
            None => png.to_vec(),
        };
        std::fs::write(out.join(&info.file), bytes).map_err(|e| format!("{}: {e}", out.display()))?;
        lib.insert(format!("{}{role}", sc.prefix), info);
    }
    for (role, w, h, wu, wv) in &sc.checked {
        let (role, w, h) = (role.as_str(), *w, *h);
        let p = &sc.prefix;
        let t = lib.get(&format!("{p}{role}")).ok_or(format!("{} has no material for {role}: has the extraction changed?", sc.glb))?;
        if t.size != [w, h] || &t.wrap_u != wu || &t.wrap_v != wv {
            return Err(format!(
                "{p}{role} is {:?} {}/{}, expected [{w}, {h}] {wu}/{wv}: the extraction's material order changed, so the {} roles need updating",
                t.size, t.wrap_u, t.wrap_v, sc.theme
            ));
        }
    }
    for role in &sc.detailed {
        ground_with_detail(sc, role, &j, bin, &mats, &texs, &images, &views, out, &mut lib)?;
    }
    let json = serde_json::to_string_pretty(&lib).map_err(|e| e.to_string())?;
    std::fs::write(out.join("textures.json"), json).map_err(|e| format!("{}: {e}", out.display()))?;
    // textures of roles since renamed
    let files: std::collections::BTreeSet<&str> = lib.values().map(|t| t.file.as_str()).collect();
    for e in std::fs::read_dir(out).map_err(|e| format!("{}: {e}", out.display()))?.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with(sc.prefix.as_str()) && name.ends_with(".png") && !files.contains(name.as_str()) {
            let _ = std::fs::remove_file(e.path());
        }
    }
    Ok(lib.len())
}

/// A ground as the game draws it (Kokiri's, Kakariko's two). Its material mixes two textures half
/// and half, (TEXEL1 - TEXEL0) x ENV_ALPHA (0x80) + TEXEL0: the camo, one repeat every 400 units
/// in Kokiri (480 in Kakariko), and a detail texture on tile 1, several times finer (its own UVs,
/// TEXCOORD_1; eight times in Kokiri). The library's texture is that mix baked into one tile: the
/// camo filtered up to the detail's resolution with the detail repeating across it, so a floor
/// textured once per camo repeat looks as the game's does.
#[allow(clippy::too_many_arguments)]
fn ground_with_detail(
    sc: &SceneTextures,
    role: &str,
    j: &Value,
    bin: &[u8],
    mats: &[Value],
    texs: &[Value],
    images: &[Value],
    views: &[Value],
    out: &Path,
    lib: &mut BTreeMap<String, TexInfo>,
) -> Result<(), String> {
    let Some((mi, m)) = mats.iter().enumerate().find(|(_, m)| {
        m["name"].as_str().and_then(|n| n.rsplit("mat").next()).and_then(|s| s.parse::<usize>().ok()).map(|n| sc.role(n)).as_deref() == Some(role)
    }) else {
        return Err(format!("{} has no {role} material", sc.glb));
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
                return Err(format!("can't tell {role}'s detail texture: re-extract {}", sc.glb));
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
    let k = ratio.filter(|&r| (1..=16).contains(&r)).ok_or(format!("can't tell {role}'s detail scale"))?;
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
    let name = format!("{}{role}", sc.prefix);
    let info = lib.get_mut(&name).ok_or(format!("no {name}"))?;
    std::fs::write(out.join(&info.file), crate::textures::encode_png(w, h, &px)).map_err(|e| format!("{}: {e}", out.display()))?;
    info.size = vec![w, h];
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs the clone's extract (git-ignored ROM data): skipped without it.
    #[test]
    fn every_region_manifest_reads() {
        for r in REGIONS {
            let sc = load_scene(r).unwrap_or_else(|e| panic!("{e}"));
            assert!(sc.prefix.ends_with('_') && !sc.label.is_empty(), "{r}");
        }
        let prefixes: std::collections::BTreeSet<&str> = scenes().iter().map(|s| s.prefix.as_str()).collect();
        assert_eq!(prefixes.len(), REGIONS.len(), "two regions share a prefix");
    }

    #[test]
    fn the_kokiri_library_comes_out_of_the_extracted_scene() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let glb = root.join("extracted/scenes/overworld/spot04/spot04.glb");
        if !glb.exists() {
            eprintln!("skipping: no {}", glb.display());
            return;
        }
        let out = std::env::temp_dir().join(format!("ow_kit_test_{}", std::process::id()));
        let sc = load_scene("kokiri").unwrap();
        let n = export(&sc, &glb, &out).unwrap();
        assert_eq!(n, sc.roles.len());
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

    /// Needs the clone's extract: skipped without it.
    #[test]
    fn the_kakariko_library_comes_out_of_the_extracted_scene() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let sc = load_scene("kakariko").unwrap();
        let glb = root.join(&sc.glb);
        if !glb.exists() {
            eprintln!("skipping: no {}", glb.display());
            return;
        }
        let out = std::env::temp_dir().join(format!("ow_kit_kak_test_{}", std::process::id()));
        let n = export(&sc, &glb, &out).unwrap();
        assert_eq!(n, sc.roles.len());
        let lib = crate::textures::Library::load(&out).unwrap();
        // the mountain wall's grassy top is cut out: the skyline
        assert_eq!(lib.get("kak_mountain").alpha, "cutout");
        assert_eq!(lib.get("kak_brick").alpha, "opaque");
        // both grounds have their detail mixed in, finer than the camo
        for g in ["kak_ground", "kak_ground_light"] {
            assert!(lib.get(g).size[0] > 32, "{g}: {:?}", lib.get(g).size);
        }
        assert_ne!(lib.rgba("kak_ground").unwrap().2, lib.rgba("kak_ground_light").unwrap().2);
        let _ = std::fs::remove_dir_all(&out);
    }
}
