//! Minimal glTF 2.0 writer (.glb, everything embedded), plus conversion of decoded
//! display lists into glTF meshes and materials.
//!
//! N64 material state that glTF can't express (combiner, render mode, geometry mode,
//! prim/env colours) is kept in `material.extras`; Blender imports extras as custom
//! properties and exports them back.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use eng_gbi::gbi::{BlendMode, CullMode, DrawList, Material, NO_BONE};
use eng_gbi::texture::WrapMode;
use eng_gfx::combiner::{Combiner, Input};
use glam::{Mat4, Quat, Vec3};
use serde_json::{Value, json};

const ARRAY_BUFFER: u32 = 34962;
const FLOAT: u32 = 5126;
const UNSIGNED_BYTE: u32 = 5121;
const UNSIGNED_SHORT: u32 = 5123;

#[derive(Default)]
pub struct Gltf {
    bin: Vec<u8>,
    buffer_views: Vec<Value>,
    accessors: Vec<Value>,
    images: Vec<Value>,
    samplers: Vec<Value>,
    textures: Vec<Value>,
    materials: Vec<Value>,
    meshes: Vec<Value>,
    pub nodes: Vec<Value>,
    skins: Vec<Value>,
    animations: Vec<Value>,
    pub extras: Option<Value>,
    texture_cache: HashMap<(u64, u32, u32, bool), usize>,
    image_cache: HashMap<u64, usize>,
    sampler_cache: HashMap<(u32, u32, bool), usize>,
    material_cache: HashMap<String, usize>,
}

fn wrap_code(w: WrapMode) -> u32 {
    match w {
        WrapMode::Repeat => 10497,
        WrapMode::Mirror => 33648,
        WrapMode::Clamp => 33071,
    }
}

impl Gltf {
    pub fn new() -> Gltf {
        Gltf::default()
    }

    fn view(&mut self, bytes: &[u8], target: Option<u32>) -> usize {
        while self.bin.len() % 4 != 0 {
            self.bin.push(0);
        }
        let offset = self.bin.len();
        self.bin.extend_from_slice(bytes);
        let mut v = json!({ "buffer": 0, "byteOffset": offset, "byteLength": bytes.len() });
        if let Some(t) = target {
            v["target"] = json!(t);
        }
        self.buffer_views.push(v);
        self.buffer_views.len() - 1
    }

    /// Float accessor. `kind` is SCALAR/VEC2/VEC3/VEC4/MAT4. Adds min/max when `bounds`.
    pub fn accessor_f32(&mut self, data: &[f32], kind: &str, bounds: bool, vertex_attr: bool) -> usize {
        let n = components(kind);
        let bytes: Vec<u8> = data.iter().flat_map(|f| f.to_le_bytes()).collect();
        let view = self.view(&bytes, vertex_attr.then_some(ARRAY_BUFFER));
        let mut a = json!({ "bufferView": view, "componentType": FLOAT, "count": data.len() / n, "type": kind });
        if bounds && !data.is_empty() {
            let mut lo = vec![f32::MAX; n];
            let mut hi = vec![f32::MIN; n];
            for c in data.chunks_exact(n) {
                for k in 0..n {
                    lo[k] = lo[k].min(c[k]);
                    hi[k] = hi[k].max(c[k]);
                }
            }
            a["min"] = json!(lo);
            a["max"] = json!(hi);
        }
        self.accessors.push(a);
        self.accessors.len() - 1
    }

    fn accessor_u8(&mut self, data: &[u8], kind: &str, normalized: bool) -> usize {
        let view = self.view(data, Some(ARRAY_BUFFER));
        let mut a = json!({ "bufferView": view, "componentType": UNSIGNED_BYTE, "count": data.len() / components(kind), "type": kind });
        if normalized {
            a["normalized"] = json!(true);
        }
        self.accessors.push(a);
        self.accessors.len() - 1
    }

    fn accessor_u16(&mut self, data: &[u16], kind: &str) -> usize {
        let bytes: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
        let view = self.view(&bytes, Some(ARRAY_BUFFER));
        self.accessors.push(json!({ "bufferView": view, "componentType": UNSIGNED_SHORT, "count": data.len() / components(kind), "type": kind }));
        self.accessors.len() - 1
    }

    /// Embeds an RGBA image as PNG; deduplicated by `hash`.
    pub fn image(&mut self, img: &eng_gbi::texture::DecodedImage, hash: u64, name: &str) -> usize {
        if let Some(&i) = self.image_cache.get(&hash) {
            return i;
        }
        let mut png = Vec::new();
        let enc = image::codecs::png::PngEncoder::new(&mut png);
        use image::ImageEncoder;
        if enc.write_image(&img.rgba, img.width, img.height, image::ExtendedColorType::Rgba8).is_err() {
            png.clear();
        }
        let view = self.view(&png, None);
        self.images.push(json!({ "bufferView": view, "mimeType": "image/png", "name": name }));
        let i = self.images.len() - 1;
        self.image_cache.insert(hash, i);
        i
    }

    fn texture(&mut self, image: usize, hash: u64, s: WrapMode, t: WrapMode, bilinear: bool) -> usize {
        let key = (hash, wrap_code(s), wrap_code(t), bilinear);
        if let Some(&i) = self.texture_cache.get(&key) {
            return i;
        }
        let skey = (key.1, key.2, bilinear);
        let sampler = match self.sampler_cache.get(&skey) {
            Some(&i) => i,
            None => {
                let f = if bilinear { (9729, 9729) } else { (9728, 9728) };
                self.samplers.push(json!({ "wrapS": key.1, "wrapT": key.2, "magFilter": f.0, "minFilter": f.1 }));
                self.sampler_cache.insert(skey, self.samplers.len() - 1);
                self.samplers.len() - 1
            }
        };
        self.textures.push(json!({ "source": image, "sampler": sampler }));
        self.texture_cache.insert(key, self.textures.len() - 1);
        self.textures.len() - 1
    }

    fn material(&mut self, draw: &DrawList, m: &Material, name_hint: &str) -> usize {
        // Single-texture materials get the combiner baked into the texture; otherwise the
        // raw texture is used with the combiner's constant tint as the colour factor.
        let bakeable = m.textures[0].is_some() && m.textures[1].is_none();
        let mut tex_index = [None, None];
        for (i, slot) in m.textures.iter().enumerate() {
            if let Some(slot) = slot {
                let ti = &draw.textures[slot.image];
                let (img, hash) = if bakeable {
                    let h = crate::gltf::hash64(&(ti.hash, m.combiner.raw, m.two_cycle, m.prim, m.env, m.prim_lod_frac));
                    (bake(m, &ti.image), h)
                } else {
                    (ti.image.clone(), ti.hash)
                };
                let name = format!("{name_hint}_tex{hash:016x}");
                let img = self.image(&img, hash, &name);
                tex_index[i] = Some(self.texture(img, hash, slot.wrap_s, slot.wrap_t, m.bilinear));
            }
        }
        let factor = if bakeable { let f = combiner_factor(m); if alpha_uses_texture(m) { [1.0; 4] } else { [1.0, 1.0, 1.0, f[3]] } } else { combiner_factor(m) };
        let mut mat = json!({
            "pbrMetallicRoughness": { "baseColorFactor": factor, "metallicFactor": 0.0, "roughnessFactor": 1.0 },
            "doubleSided": m.cull == CullMode::None,
            "extras": n64_extras(m, tex_index),
        });
        mat["extras"]["n64_combiner_baked_into_texture"] = json!(bakeable);
        if let Some(t) = tex_index[0].or(tex_index[1]) {
            mat["pbrMetallicRoughness"]["baseColorTexture"] = json!({ "index": t, "texCoord": if tex_index[0].is_some() { 0 } else { 1 } });
        }
        // glTF would cut out by texture alpha even when the N64 combiner ignores it.
        let tex_alpha = alpha_uses_texture(m) || bakeable;
        match m.blend {
            BlendMode::Opaque => {}
            BlendMode::Cutout(_) if !tex_alpha => {}
            BlendMode::Cutout(t) => {
                mat["alphaMode"] = json!("MASK");
                mat["alphaCutoff"] = json!(t as f32 / 255.0);
            }
            BlendMode::Translucent => mat["alphaMode"] = json!("BLEND"),
        }
        if !m.lit {
            // Vertex colours carry the shading; keep glTF lighting neutral.
            mat["extensions"] = json!({ "KHR_materials_unlit": {} });
        }
        let key = mat.to_string();
        if let Some(&i) = self.material_cache.get(&key) {
            return i;
        }
        mat["name"] = json!(format!("{name_hint}_mat{}", self.materials.len()));
        self.materials.push(mat);
        self.material_cache.insert(key, self.materials.len() - 1);
        self.materials.len() - 1
    }

    /// Converts a decoded draw list into one glTF mesh (one primitive per material).
    ///
    /// Vertex positions are in their bone's local space; `bone_mats` moves them into the
    /// mesh's space (pass the bind pose for skinned meshes, or None for world-space data
    /// such as rooms). `joints` maps bone index -> skin joint index for skinning.
    /// Returns None when the draw list has no triangles.
    pub fn add_draw_list(&mut self, draw: &DrawList, name: &str, bone_mats: Option<&[Mat4]>, joints: Option<&[u16]>) -> Option<usize> {
        match joints {
            Some(j) => {
                let f = |v: &eng_gbi::gbi::Vertex| {
                    let b = if v.bone == NO_BONE { 0 } else { v.bone as usize };
                    ([j.get(b).copied().unwrap_or(0), 0, 0, 0], [1.0, 0.0, 0.0, 0.0])
                };
                self.add_draw_list_weighted(draw, name, bone_mats, Some(&f))
            }
            None => self.add_draw_list_weighted(draw, name, bone_mats, None),
        }
    }

    /// Like `add_draw_list`, with explicit per-vertex joints and weights (up to 4).
    #[allow(clippy::type_complexity)]
    pub fn add_draw_list_weighted(
        &mut self,
        draw: &DrawList,
        name: &str,
        bone_mats: Option<&[Mat4]>,
        skin: Option<&dyn Fn(&eng_gbi::gbi::Vertex) -> ([u16; 4], [f32; 4])>,
    ) -> Option<usize> {
        let mut by_material: Vec<(usize, Vec<&eng_gbi::gbi::Vertex>)> = Vec::new();
        for b in &draw.batches {
            match by_material.iter_mut().find(|(m, _)| *m == b.material) {
                Some((_, v)) => v.extend(b.vertices.iter()),
                None => by_material.push((b.material, b.vertices.iter().collect())),
            }
        }
        let mut prims = Vec::new();
        for (mi, verts) in by_material {
            if verts.len() < 3 {
                continue;
            }
            let m = &draw.materials[mi];
            let mut pos = Vec::with_capacity(verts.len() * 3);
            let mut nrm = Vec::new();
            let mut col = Vec::new();
            let mut uv0 = Vec::new();
            let mut uv1 = Vec::new();
            let mut jnt = Vec::new();
            let mut wgt = Vec::new();
            for v in &verts {
                let mat = match (bone_mats, v.bone) {
                    (Some(bm), b) if b != NO_BONE => bm.get(b as usize).copied().unwrap_or(Mat4::IDENTITY),
                    _ => Mat4::IDENTITY,
                };
                pos.extend_from_slice(&mat.transform_point3(v.pos).to_array());
                if m.lit {
                    let n = mat.transform_vector3(v.normal).normalize_or(Vec3::Y);
                    nrm.extend_from_slice(&n.to_array());
                } else {
                    col.extend_from_slice(&v.color);
                }
                uv0.extend_from_slice(&v.uv[0].to_array());
                uv1.extend_from_slice(&v.uv[1].to_array());
                if let Some(f) = skin {
                    let (j, w) = f(v);
                    jnt.extend_from_slice(&j);
                    wgt.extend_from_slice(&w);
                }
            }
            let mut attrs = serde_json::Map::new();
            attrs.insert("POSITION".into(), json!(self.accessor_f32(&pos, "VEC3", true, true)));
            if m.lit {
                attrs.insert("NORMAL".into(), json!(self.accessor_f32(&nrm, "VEC3", false, true)));
            } else {
                attrs.insert("COLOR_0".into(), json!(self.accessor_u8(&col, "VEC4", true)));
            }
            if m.textures[0].is_some() || m.textures[1].is_some() {
                attrs.insert("TEXCOORD_0".into(), json!(self.accessor_f32(&uv0, "VEC2", false, true)));
            }
            if m.textures[1].is_some() {
                attrs.insert("TEXCOORD_1".into(), json!(self.accessor_f32(&uv1, "VEC2", false, true)));
            }
            if skin.is_some() {
                attrs.insert("JOINTS_0".into(), json!(self.accessor_u16(&jnt, "VEC4")));
                attrs.insert("WEIGHTS_0".into(), json!(self.accessor_f32(&wgt, "VEC4", false, true)));
            }
            let material = self.material(draw, m, name);
            prims.push(json!({ "attributes": attrs, "material": material, "mode": 4 }));
        }
        if prims.is_empty() {
            return None;
        }
        self.meshes.push(json!({ "name": name, "primitives": prims }));
        Some(self.meshes.len() - 1)
    }

    /// Adds a mesh from plain triangles (e.g. collision): positions and per-primitive groups.
    pub fn add_triangles(&mut self, name: &str, groups: &[(Vec<[f32; 3]>, Value)], color: [f32; 4]) -> Option<usize> {
        let mut prims = Vec::new();
        for (tris, extras) in groups {
            if tris.len() < 3 {
                continue;
            }
            let flat: Vec<f32> = tris.iter().flatten().copied().collect();
            let pos = self.accessor_f32(&flat, "VEC3", true, true);
            let mat = json!({
                "name": format!("{name}_{}", self.materials.len()),
                "pbrMetallicRoughness": { "baseColorFactor": color, "metallicFactor": 0.0, "roughnessFactor": 1.0 },
                "doubleSided": true,
                "alphaMode": if color[3] < 1.0 { "BLEND" } else { "OPAQUE" },
                "extras": extras,
            });
            self.materials.push(mat);
            prims.push(json!({ "attributes": { "POSITION": pos }, "material": self.materials.len() - 1, "mode": 4, "extras": extras }));
        }
        if prims.is_empty() {
            return None;
        }
        self.meshes.push(json!({ "name": name, "primitives": prims }));
        Some(self.meshes.len() - 1)
    }

    pub fn node(&mut self, node: Value) -> usize {
        self.nodes.push(node);
        self.nodes.len() - 1
    }

    pub fn add_child(&mut self, parent: usize, child: usize) {
        let n = &mut self.nodes[parent];
        match n.get_mut("children").and_then(|c| c.as_array_mut()) {
            Some(c) => c.push(json!(child)),
            None => n["children"] = json!([child]),
        }
    }

    pub fn skin(&mut self, name: &str, joints: &[usize], inverse_bind: &[Mat4], skeleton_root: usize) -> usize {
        let flat: Vec<f32> = inverse_bind.iter().flat_map(|m| m.to_cols_array()).collect();
        let ibm = self.accessor_f32(&flat, "MAT4", false, false);
        self.skins.push(json!({ "name": name, "joints": joints, "inverseBindMatrices": ibm, "skeleton": skeleton_root }));
        self.skins.len() - 1
    }

    /// Adds an animation from sampled keyframes at `fps`: per node, optional translations
    /// and rotations (one entry per frame).
    pub fn animation(&mut self, name: &str, fps: f32, frames: usize, tracks: &[(usize, Option<Vec<Vec3>>, Option<Vec<Quat>>)]) {
        let times: Vec<f32> = (0..frames).map(|f| f as f32 / fps).collect();
        let input = self.accessor_f32(&times, "SCALAR", true, false);
        let mut samplers = Vec::new();
        let mut channels = Vec::new();
        for (node, trans, rot) in tracks {
            if let Some(t) = trans {
                let flat: Vec<f32> = t.iter().flat_map(|v| v.to_array()).collect();
                let out = self.accessor_f32(&flat, "VEC3", false, false);
                samplers.push(json!({ "input": input, "output": out, "interpolation": "LINEAR" }));
                channels.push(json!({ "sampler": samplers.len() - 1, "target": { "node": node, "path": "translation" } }));
            }
            if let Some(r) = rot {
                // Keep consecutive quaternions in the same hemisphere so LINEAR takes the short way.
                let mut prev = Quat::IDENTITY;
                let flat: Vec<f32> = r
                    .iter()
                    .flat_map(|q| {
                        let q = if prev.dot(*q) < 0.0 { -*q } else { *q };
                        prev = q;
                        q.to_array()
                    })
                    .collect();
                let out = self.accessor_f32(&flat, "VEC4", false, false);
                samplers.push(json!({ "input": input, "output": out, "interpolation": "LINEAR" }));
                channels.push(json!({ "sampler": samplers.len() - 1, "target": { "node": node, "path": "rotation" } }));
            }
        }
        self.animations.push(json!({ "name": name, "samplers": samplers, "channels": channels }));
    }

    pub fn mesh_count(&self) -> usize {
        self.meshes.len()
    }

    /// Writes a .glb whose default scene contains `roots`.
    pub fn write_glb(&mut self, path: &Path, roots: &[usize]) -> Result<()> {
        while self.bin.len() % 4 != 0 {
            self.bin.push(0);
        }
        let mut doc = json!({
            "asset": { "version": "2.0", "generator": "oot_extract (local, derived from user ROM)" },
            "scene": 0,
            "scenes": [{ "nodes": roots }],
            "nodes": self.nodes,
            "buffers": [{ "byteLength": self.bin.len() }],
            "bufferViews": self.buffer_views,
            "accessors": self.accessors,
        });
        for (k, v) in [
            ("images", &self.images),
            ("samplers", &self.samplers),
            ("textures", &self.textures),
            ("materials", &self.materials),
            ("meshes", &self.meshes),
            ("skins", &self.skins),
            ("animations", &self.animations),
        ] {
            if !v.is_empty() {
                doc[k] = json!(v);
            }
        }
        if self.materials.iter().any(|m| m.get("extensions").is_some()) {
            doc["extensionsUsed"] = json!(["KHR_materials_unlit"]);
        }
        if let Some(e) = &self.extras {
            doc["extras"] = e.clone();
        }
        let mut js = serde_json::to_vec(&doc)?;
        while js.len() % 4 != 0 {
            js.push(b' ');
        }
        let total = 12 + 8 + js.len() + 8 + self.bin.len();
        let mut out = Vec::with_capacity(total);
        out.extend_from_slice(b"glTF");
        out.extend_from_slice(&2u32.to_le_bytes());
        out.extend_from_slice(&(total as u32).to_le_bytes());
        out.extend_from_slice(&(js.len() as u32).to_le_bytes());
        out.extend_from_slice(b"JSON");
        out.extend_from_slice(&js);
        out.extend_from_slice(&(self.bin.len() as u32).to_le_bytes());
        out.extend_from_slice(b"BIN\0");
        out.extend_from_slice(&self.bin);
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(path, out)?;
        Ok(())
    }
}

pub fn hash64(v: &impl std::hash::Hash) -> u64 {
    use std::hash::Hasher;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}

fn components(kind: &str) -> usize {
    match kind {
        "SCALAR" => 1,
        "VEC2" => 2,
        "VEC3" => 3,
        "VEC4" => 4,
        "MAT4" => 16,
        _ => 1,
    }
}

fn rgba(c: [u8; 4]) -> [f32; 4] {
    c.map(|v| v as f32 / 255.0)
}

/// Evaluates the colour combiner for given texel and shade values (RGBA, 0..1).
pub fn eval_combiner(m: &Material, texel0: [f32; 4], texel1: [f32; 4], shade: [f32; 4]) -> [f32; 4] {
    let prim = rgba(m.prim);
    let env = rgba(m.env);
    let n = if m.two_cycle { 2 } else { 1 };
    let mut combined = [0.0f32; 4];
    for c in &m.combiner.cycles[..n] {
        let val = |i: Input, k: usize| -> f32 {
            match i {
                Input::Combined => combined[k],
                Input::Texel0 => texel0[k],
                Input::Texel1 => texel1[k],
                Input::Shade => shade[k],
                Input::One => 1.0,
                Input::Prim => prim[k],
                Input::Env => env[k],
                Input::CombinedAlpha => combined[3],
                Input::Texel0Alpha => texel0[3],
                Input::Texel1Alpha => texel1[3],
                Input::ShadeAlpha => shade[3],
                Input::PrimAlpha => prim[3],
                Input::EnvAlpha => env[3],
                Input::PrimLodFrac => m.prim_lod_frac as f32 / 255.0,
                _ => 0.0,
            }
        };
        let mut out = [0.0f32; 4];
        for (k, o) in out.iter_mut().enumerate().take(3) {
            *o = ((val(c.a, k) - val(c.b, k)) * val(c.c, k) + val(c.d, k)).clamp(0.0, 1.0);
        }
        out[3] = ((val(c.aa, 3) - val(c.ab, 3)) * val(c.ac, 3) + val(c.ad, 3)).clamp(0.0, 1.0);
        combined = out;
    }
    combined
}

/// The constant tint the combiner applies with textures and shade at white.
pub fn combiner_factor(m: &Material) -> [f32; 4] {
    eval_combiner(m, [1.0; 4], [1.0; 4], [1.0; 4])
}

/// Whether the final alpha depends on a texture (glTF always multiplies texture alpha in).
pub fn alpha_uses_texture(m: &Material) -> bool {
    let n = if m.two_cycle { 2 } else { 1 };
    let mut prev = false;
    for c in &m.combiner.cycles[..n] {
        prev = [c.aa, c.ab, c.ac, c.ad].iter().any(|i| {
            matches!(i, Input::Texel0 | Input::Texel1 | Input::Texel0Alpha | Input::Texel1Alpha)
                || (matches!(i, Input::Combined | Input::CombinedAlpha) && prev)
        });
    }
    prev
}

/// Bakes the combiner's constant inputs (prim, env, LOD fraction) into texture 0, with shade
/// at white so glTF's vertex colour / lighting multiply stands in for SHADE.
fn bake(m: &Material, img: &eng_gbi::texture::DecodedImage) -> eng_gbi::texture::DecodedImage {
    let mut out = img.clone();
    for px in out.rgba.chunks_exact_mut(4) {
        let t = [px[0], px[1], px[2], px[3]].map(|v| v as f32 / 255.0);
        let c = eval_combiner(m, t, t, [1.0; 4]);
        for k in 0..4 {
            px[k] = (c[k] * 255.0).round() as u8;
        }
    }
    out
}

fn hex(c: [u8; 4]) -> String {
    format!("#{:02X}{:02X}{:02X}{:02X}", c[0], c[1], c[2], c[3])
}

fn n64_extras(m: &Material, tex: [Option<usize>; 2]) -> Value {
    let comb: &Combiner = &m.combiner;
    json!({
        "n64_combiner": comb.describe(m.two_cycle),
        "n64_combiner_raw": format!("0x{:014X}", comb.raw),
        "n64_two_cycle": m.two_cycle,
        "n64_prim": hex(m.prim),
        "n64_env": hex(m.env),
        "n64_blend_color": hex(m.blend_color),
        "n64_fog": hex(m.fog),
        "n64_geometry_mode": format!("0x{:08X}", m.geometry_mode),
        "n64_othermode_h": format!("0x{:08X}", m.othermode_h),
        "n64_othermode_l": format!("0x{:08X}", m.othermode_l),
        "n64_blend": format!("{:?}", m.blend),
        "n64_cull": format!("{:?}", m.cull),
        "n64_lit": m.lit,
        "n64_texgen": m.texgen,
        "n64_decal": m.decal,
        "n64_texture1_used": tex[1].is_some(),
    })
}
