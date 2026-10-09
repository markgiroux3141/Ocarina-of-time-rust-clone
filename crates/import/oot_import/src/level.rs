//! Custom levels: an overworld build (`crates/tools/overworld`'s export: `level.json` and its
//! `textures/`) as one room and its collision, for playing in the clone with child Link.
//!
//! The export is engine-neutral: x east, y north, z up, triangles counter-clockwise from the
//! front, per-corner UVs with v up, baked vertex colours (0-255) and, per triangle, a material
//! (a texture with its wrap, alpha and culling) and a collision surface role. Here:
//! - Positions become the game's (x, z, -y): a rotation, so windings stay counter-clockwise.
//! - Each material becomes a `Material` drawing TEXEL0 x SHADE (colour and alpha): unlit, the
//!   baked colours as shade, as the game's own rooms draw. Opaque and cutout materials go in
//!   the room's OPA list, translucent ones (water, at their opacity) in the XLU list.
//! - Surface roles become Kokiri Forest's own surface types (`ROLES`). Water surfaces are not
//!   collision: each pond becomes a water box at its surface.
//! - Objects marked `"render": false` (kit pieces' collision meshes) are collision only, and
//!   materials marked `"decal"` (door shadows) draw in the decal depth mode, as spot04's do.
//! - Scene cameras (`cameras`): a crawlspace's line becomes a `CAM_SET_CRAWLSPACE` bg camera
//!   after a `CAM_SET_NORMAL0` one at index 0 (every other floor's), and a surface role
//!   `<role>#k` calls for bg camera k from its floor, as spot04's crawlspace floor does.
//! - A material with a second texture (`file2`, the ground under a dirt path) blends it in by the
//!   vertices' weights (the object's `alpha`), two cycles as Kokiri's own ground:
//!   (TEXEL1 - TEXEL0) x SHADE_ALPHA + TEXEL0, then x SHADE.
//!
//! The collision header indexes vertices in 13 bits, so a level's collision may have at most
//! 8192 vertices; bigger levels are refused with a hint (build at a lower detail).

use anyhow::{Context, Result, bail, ensure};
use eng_collision::collision::{CollisionBuilder, CollisionHeader};
use eng_gfx::combiner::{self, Combiner};
use eng_gfx::texture::{DecodedImage, WrapMode};
use eng_gfx::{Batch, BlendMode, CullMode, DrawList, Material, NO_BONE, TextureImage, TextureSlot, Vertex};
use glam::{Vec2, Vec3};
use oot_game::env::EnvLights;
use oot_game::play_scene::SceneState;
use oot_game::scene::{ActorEntry, EntryMesh, LayerData, RoomData, SceneData, ShapeKind, SkyboxSettings, Spawn};
use oot_game::scene_table::DrawConfigState;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// Collision surface roles -> surface type words, from Kokiri Forest (spot04's collision,
/// with the bg camera index cleared: custom levels have no bg cameras). Word 1's low nibble is
/// the footstep sound (SURFACE_SFX_TYPE: 0 dirt, 2 stone, 8 grass, 9 bridge, 10 wood); bit 17
/// lets the hookshot hold.
pub const ROLES: [(&str, u32, u32); 18] = [
    // spot04 surface 10: grass footsteps
    ("ground", 0x0000_0000, 0x0000_0FC8),
    // ledges Link may grab
    ("wall", 0x0000_0000, 0x0000_0FC8),
    // spot04 surface 14: WALL_TYPE 1, no ledge grab (the edge of the world's cliffs)
    ("wall_nograb", 0x0020_0000, 0x0000_0FC8),
    // spot04 surface 15: WALL_TYPE 4, climbable vines
    ("vines", 0x0080_0000, 0x0000_0FCA),
    // kit pieces (crates/tools/overworld/kit/kokiri.json maps spot04's types to these)
    // spot04 13: the dirt path, dirt footsteps
    ("dirt", 0x0000_0000, 0x0000_0FC0),
    // 27: the stepping stones
    ("stone", 0x0000_0000, 0x0000_0FC2),
    // sand footsteps (the desert's, Gerudo's): kit pieces from those regions
    ("sand", 0x0000_0000, 0x0000_0FC1),
    // 20: the floor inside the hedge's tall grass (tall-grass footsteps)
    ("tall_grass", 0x0000_0000, 0x0000_0FC6),
    // 28: the log walkways' planks
    ("planks", 0x0000_0000, 0x0000_0FC9),
    // 0, 23: houses and stumps
    ("wood", 0x0000_0000, 0x0000_0FCA),
    // 29: the training ground's fences (the hookshot holds)
    ("fence", 0x0000_0000, 0x0002_0FCA),
    // 26 and 25: Link's ladder (WALL_TYPE 2) and its top (3)
    ("ladder", 0x0040_0000, 0x0002_0FCA),
    ("ladder_top", 0x0060_0000, 0x0000_0FCA),
    // 32: the crawlspace's mouths (WALL_TYPE 5)
    ("crawl", 0x00A0_0000, 0x0000_0FCA),
    // 44: the crawlspace's floor (its bg camera, the crawlspace's line, comes from `#k`)
    ("crawl_floor", 0x0000_0000, 0x0000_0FCA),
    // doorways (spot04 1 to 7) and exits (22, 24): plain floor until levels load through
    // Play_Init, as an exit in the spikes' view stops on a black screen
    ("door", 0x0000_0000, 0x0000_0FC8),
    ("exit", 0x0000_0000, 0x0000_0FCA),
    // a pit's floor: FLOOR_PROPERTY_12, the void (Player_HandleExitsAndVoids voids Link out as he
    // falls onto it), as the game's bottomless pits
    ("void", 0x3000_0000, 0x0000_0FC8),
];

/// The collision header's vertex indices are 13 bits.
pub const MAX_COLLISION_VERTICES: usize = 1 << 13;

/// A loaded level: its room, collision, where Link starts, and what it has.
pub struct Level {
    pub name: String,
    pub room: RoomData,
    pub collision: CollisionHeader,
    /// Feet position and facing (binary angle): the middle of the level, facing north.
    pub spawn: (Vec3, i16),
    pub triangles: usize,
    pub notes: Vec<String>,
}

fn game(p: [f32; 3]) -> Vec3 {
    Vec3::new(p[0], p[2], -p[1])
}

fn wrap(s: &str) -> WrapMode {
    match s {
        "clamp" => WrapMode::Clamp,
        "mirror" => WrapMode::Mirror,
        _ => WrapMode::Repeat,
    }
}

/// TEXEL0 x SHADE, colour and alpha, one cycle; with a second texture, TEXEL1 blended over TEXEL0
/// by SHADE_ALPHA, then x SHADE, alpha one (two cycles).
fn material(texture: usize, texture2: Option<usize>, wrap_s: WrapMode, wrap_t: WrapMode, blend: BlendMode, cull: CullMode, decal: bool) -> Material {
    // GBI selectors: colour TEXEL0 1, TEXEL1 2, SHADE 4, SHADE_ALPHA 11, COMBINED 0, zero 15 (a, b),
    // 31 (c), 7 (d); alpha TEXEL0 1, SHADE 4, ONE 6, COMBINED 0, zero 7
    let (cc, two_cycle) = match texture2 {
        None => (combiner::encode([1, 15, 4, 7, 1, 7, 4, 7], [1, 15, 4, 7, 1, 7, 4, 7]), false),
        Some(_) => (combiner::encode([2, 1, 11, 1, 7, 7, 7, 6], [0, 15, 4, 7, 7, 7, 7, 0]), true),
    };
    let translucent = blend == BlendMode::Translucent;
    Material {
        combiner: Combiner::decode(cc),
        two_cycle,
        prim: [255; 4],
        prim_lod_frac: 0,
        env: [255; 4],
        fog: [0; 4],
        blend_color: [0; 4],
        geometry_mode: 0,
        othermode_h: 0,
        othermode_l: 0,
        textures: [Some(TextureSlot { image: texture, wrap_s, wrap_t }), texture2.map(|image| TextureSlot { image, wrap_s, wrap_t })],
        blend,
        cull,
        depth_test: true,
        depth_write: !translucent,
        decal,
        lit: false,
        texgen: false,
        bilinear: true,
        uv_dyn: [None, None],
        env_dyn: None,
        prim_dyn: None,
        // the scene's fog, without taking over the vertex alpha
        fog_blend: true,
    }
}

fn hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

fn floats(v: &Value, key: &str) -> Vec<f32> {
    v[key].as_array().map(|a| a.iter().map(|x| x.as_f64().unwrap_or(0.0) as f32).collect()).unwrap_or_default()
}

fn ints(v: &Value, key: &str) -> Vec<i64> {
    v[key].as_array().map(|a| a.iter().map(|x| x.as_i64().unwrap_or(-1)).collect()).unwrap_or_default()
}

/// Loads the level an overworld export wrote into `dir`.
pub fn load(dir: &Path) -> Result<Level> {
    let path = dir.join("level.json");
    let j: Value = serde_json::from_slice(&std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?)
        .with_context(|| format!("parsing {}", path.display()))?;
    let name = j["name"].as_str().unwrap_or("custom level").to_string();
    let mut notes = vec![];

    // materials: textures, how they wrap and blend
    let mut opa = DrawList::default();
    let mut xlu = DrawList::default();
    struct Mat {
        xlu: bool,
        index: usize,
        alpha: u8,
        /// Blends a second texture by vertex weight.
        blend: bool,
    }
    let mut mats: Vec<Mat> = vec![];
    let mut open_image = |key: &str, m: &Value| {
        let file = dir.join(m[key].as_str().unwrap_or(""));
        match image::open(&file) {
            Ok(i) => i.to_rgba8(),
            Err(e) => {
                notes.push(format!("{}: {e}; drawn white", file.display()));
                image::RgbaImage::from_pixel(1, 1, image::Rgba([255; 4]))
            }
        }
    };
    for m in j["materials"].as_array().context("level.json: no materials")? {
        let img = open_image("file", m);
        let img2 = m.get("file2").and_then(|f| f.as_str()).map(|_| open_image("file2", m));
        let (blend, alpha) = match m["alpha"].as_str() {
            Some("cutout") => (BlendMode::Cutout(128), 255),
            Some("blend") => (BlendMode::Translucent, (m["opacity"].as_f64().unwrap_or(1.0) * 255.0).round() as u8),
            _ => (BlendMode::Opaque, 255),
        };
        let cull = if m["cull"].as_str() == Some("none") { CullMode::None } else { CullMode::Back };
        let translucent = blend == BlendMode::Translucent;
        let d = if translucent { &mut xlu } else { &mut opa };
        let mut intern = |img: image::RgbaImage| {
            let h = hash(img.as_raw()) ^ (img.width() as u64) << 48;
            let (w, ht) = img.dimensions();
            d.intern_texture(h, || TextureImage {
                image: DecodedImage { width: w, height: ht, rgba: img.into_raw() },
                fmt: 0,
                siz: 3,
                hash: h,
                source_segments: 0,
            })
        };
        let tex = intern(img);
        let tex2 = img2.map(&mut intern);
        let decal = m["decal"].as_bool() == Some(true);
        let index = d.intern_material(material(tex, tex2, wrap(m["wrap_u"].as_str().unwrap_or("")), wrap(m["wrap_v"].as_str().unwrap_or("")), blend, cull, decal));
        mats.push(Mat { xlu: translucent, index, alpha, blend: tex2.is_some() });
    }
    let roles: Vec<String> = j["surfaces"].as_array().map(|a| a.iter().map(|s| s.as_str().unwrap_or("").to_string()).collect()).unwrap_or_default();

    // geometry, collision and ponds
    let mut col = CollisionBuilder::new();
    let mut surfaces: HashMap<usize, Option<u16>> = HashMap::new();
    let mut batches: HashMap<(bool, usize), Vec<Vertex>> = HashMap::new();
    let mut water: Vec<[Vec3; 3]> = vec![];
    let mut triangles = 0;
    let (mut lo, mut hi) = (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY));
    for o in j["objects"].as_array().context("level.json: no objects")? {
        let (verts, tris, uvs, colors) = (floats(o, "verts"), ints(o, "tris"), floats(o, "uvs"), ints(o, "colors"));
        let weights = ints(o, "alpha");
        let (mat, surf) = (ints(o, "material"), ints(o, "surface"));
        let render = o["render"].as_bool() != Some(false);
        ensure!(verts.len() % 3 == 0 && tris.len() % 3 == 0 && uvs.len() == tris.len() * 2, "{name}: object {} is malformed", o["name"]);
        for t in 0..tris.len() / 3 {
            let idx = [tris[3 * t] as usize, tris[3 * t + 1] as usize, tris[3 * t + 2] as usize];
            let p = idx.map(|i| game([verts[3 * i], verts[3 * i + 1], verts[3 * i + 2]]));
            let drawn = render && mat.get(t).is_some_and(|&m| m >= 0);
            for (c, &i) in idx.iter().enumerate().filter(|_| drawn) {
                let m = &mats[mat[t] as usize];
                let rgb = if colors.len() >= 3 * (i + 1) { [colors[3 * i] as u8, colors[3 * i + 1] as u8, colors[3 * i + 2] as u8] } else { [255; 3] };
                let uv = Vec2::new(uvs[6 * t + 2 * c], 1.0 - uvs[6 * t + 2 * c + 1]);
                // a blend material's vertex alpha is how much of its second texture shows
                let a = if m.blend { weights.get(i).map_or(0, |&w| w.clamp(0, 255) as u8) } else { m.alpha };
                batches.entry((m.xlu, m.index)).or_default().push(Vertex {
                    bone: NO_BONE,
                    pos: p[c],
                    normal: Vec3::Y,
                    color: [rgb[0], rgb[1], rgb[2], a],
                    uv: [uv, uv],
                });
            }
            for q in p {
                lo = lo.min(q);
                hi = hi.max(q);
            }
            if drawn {
                triangles += 1;
            }
            let s = surf.get(t).copied().unwrap_or(-1);
            if s < 0 {
                continue;
            }
            let role = roles.get(s as usize).map(String::as_str).unwrap_or("");
            if role == "water" {
                water.push(p);
                continue;
            }
            // `<role>#k`: the role's words with bg camera k
            let (role, cam) = match role.split_once('#') {
                Some((r, k)) => (r, k.parse::<u32>().unwrap_or(0).min(0xFF)),
                None => (role, 0),
            };
            let ty = *surfaces.entry(s as usize).or_insert_with(|| match ROLES.iter().find(|r| r.0 == role) {
                Some(&(_, w0, w1)) => Some(col.surface(w0 | cam, w1)),
                None if role.is_empty() => None,
                None => {
                    notes.push(format!("surface role {role:?} unknown: collides as ground"));
                    Some(col.surface(ROLES[0].1, ROLES[0].2))
                }
            });
            if let Some(ty) = ty {
                col.tri(p[0], p[1], p[2], ty);
            }
        }
    }
    let mut keys: Vec<_> = batches.keys().copied().collect();
    keys.sort();
    for k in keys {
        let vertices = batches.remove(&k).unwrap();
        let d = if k.0 { &mut xlu } else { &mut opa };
        d.stats.triangles += vertices.len() / 3;
        d.batches.push(Batch { material: k.1, vertices });
    }
    // ponds: each connected patch of water surface is a water box at its height
    for (bl, bh, y) in water_patches(&water) {
        let (x0, z0) = (bl.x.floor(), bl.y.floor());
        col.water_box(x0 as i16, z0 as i16, (bh.x.ceil() - x0) as i16, (bh.y.ceil() - z0) as i16, y.round() as i16, 0x3F);
    }
    let mut collision = col.finish();
    // scene cameras: the normal one every floor calls for (index 0), then the level's
    let cams = j["cameras"].as_array().cloned().unwrap_or_default();
    if !cams.is_empty() {
        collision.bg_cams.push(eng_collision::collision::BgCamInfo { setting: 0x01, count: 0, data: vec![] });
        for c in &cams {
            let pts: Vec<[i16; 3]> = c["points"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|p| {
                            let q = game([0, 1, 2].map(|k| p[k].as_f64().unwrap_or(0.0) as f32));
                            [q.x.round() as i16, q.y.round() as i16, q.z.round() as i16]
                        })
                        .collect()
                })
                .unwrap_or_default();
            match c["setting"].as_str() {
                // CAM_SET_CRAWLSPACE's data: each end three times, as spot04's
                Some("crawlspace") if pts.len() == 2 => {
                    let data = vec![pts[0], pts[0], pts[0], pts[1], pts[1], pts[1]];
                    collision.bg_cams.push(eng_collision::collision::BgCamInfo { setting: 0x1E, count: data.len() as i16, data });
                }
                s => {
                    notes.push(format!("camera {s:?}: not a crawlspace line; the normal camera instead"));
                    collision.bg_cams.push(eng_collision::collision::BgCamInfo { setting: 0x01, count: 0, data: vec![] });
                }
            }
        }
    }
    if collision.vertices.len() > MAX_COLLISION_VERTICES {
        bail!(
            "{name}: its collision has {} vertices, and the game indexes at most {MAX_COLLISION_VERTICES}: build it at a lower detail (Level settings: Medium or Low)",
            collision.vertices.len()
        );
    }
    for v in [lo, hi] {
        ensure!(v.abs().max_element() < 32000.0, "{name}: reaches {v}, past the game's 16-bit coordinates");
    }

    // Link starts at the middle, on the highest floor there, facing north (-z); or, where there's
    // none (a level of separate areas, with the void between them), on the floor nearest it
    let mut mid = (lo + hi) * 0.5;
    let reach = (hi - lo).max_element();
    let found = floor_at(&collision, mid.x, mid.z).or_else(|| {
        let mut r = 50.0;
        while r < reach {
            for k in 0..32 {
                let a = k as f32 / 32.0 * std::f32::consts::TAU;
                let (x, z) = (mid.x + r * a.cos(), mid.z + r * a.sin());
                if let Some(y) = floor_at(&collision, x, z) {
                    (mid.x, mid.z) = (x, z);
                    return Some(y);
                }
            }
            r += 50.0;
        }
        None
    });
    let y = found.unwrap_or_else(|| {
        notes.push("no floor under the middle: starting above it".into());
        hi.y
    });
    let room = RoomData {
        file: format!("{name}_room_0"),
        index: 0,
        behavior: [0, 0],
        echo: 0,
        time: None,
        skybox_disabled: false,
        sun_moon_disabled: false,
        actors: vec![],
        objects: vec![],
        shape: Some(ShapeKind::Normal),
        entries: vec![EntryMesh {
            bounds: None,
            opa: (!opa.batches.is_empty()).then_some(opa),
            xlu: (!xlu.batches.is_empty()).then_some(xlu),
        }],
        backgrounds: vec![],
    };
    Ok(Level { name, room, collision, spawn: (Vec3::new(mid.x, y, mid.z), -0x8000), triangles, notes })
}

/// Connected patches of water triangles (sharing corners): their (x, z) bounds and surface height.
fn water_patches(tris: &[[Vec3; 3]]) -> Vec<(Vec2, Vec2, f32)> {
    let key = |p: Vec3| ((p.x * 10.0).round() as i64, (p.z * 10.0).round() as i64);
    let mut parent: Vec<usize> = (0..tris.len()).collect();
    fn find(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    let mut owner: HashMap<(i64, i64), usize> = HashMap::new();
    for (t, tri) in tris.iter().enumerate() {
        for &p in tri {
            if let Some(&o) = owner.get(&key(p)) {
                let (a, b) = (find(&mut parent, o), find(&mut parent, t));
                parent[a] = b;
            } else {
                owner.insert(key(p), t);
            }
        }
    }
    let mut out: HashMap<usize, (Vec2, Vec2, f32)> = HashMap::new();
    for (t, tri) in tris.iter().enumerate() {
        let r = find(&mut parent, t);
        let e = out.entry(r).or_insert((Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY), tri[0].y));
        for p in tri {
            e.0 = e.0.min(Vec2::new(p.x, p.z));
            e.1 = e.1.max(Vec2::new(p.x, p.z));
        }
    }
    let mut v: Vec<_> = out.into_values().collect();
    v.sort_by(|a, b| a.0.x.total_cmp(&b.0.x));
    v
}

/// The highest collision floor at (x, z): polys less than 60 degrees from level, the
/// ones the game sorts as floors (`StaticLookup_AddPoly`: normal y over 0.5).
fn floor_at(h: &CollisionHeader, x: f32, z: f32) -> Option<f32> {
    let mut best: Option<f32> = None;
    for p in &h.polys {
        if p.normal[1] <= 0x3FFF {
            continue;
        }
        let [a, b, c] = [0, 1, 2].map(|k| h.vertex((p.vtx[k] & 0x1FFF) as usize));
        let d = (b.z - c.z) * (a.x - c.x) + (c.x - b.x) * (a.z - c.z);
        if d.abs() < 1e-6 {
            continue;
        }
        let l1 = ((b.z - c.z) * (x - c.x) + (c.x - b.x) * (z - c.z)) / d;
        let l2 = ((c.z - a.z) * (x - c.x) + (a.x - c.x) * (z - c.z)) / d;
        let l3 = 1.0 - l1 - l2;
        if l1 >= -1e-4 && l2 >= -1e-4 && l3 >= -1e-4 {
            let y = l1 * a.y + l2 * b.y + l3 * c.y;
            best = Some(best.map_or(y, |m: f32| m.max(y)));
        }
    }
    best
}

/// The level as play's scene: one layer, one room drawn whatever the room context says, the
/// given lights (custom levels borrow a real scene's, e.g. Kokiri Forest's), Link's spawn.
pub fn scene(level: &Level, lights: EnvLights, child: bool) -> SceneState {
    let (pos, yaw) = level.spawn;
    let layer = LayerData {
        header_offset: 0,
        collision: format!("custom/{}", level.name),
        spawns: vec![ActorEntry { id: 0, pos: [pos.x as i16, pos.y as i16, pos.z as i16], rot: [0, yaw, 0], params: 0x0FFF }],
        light_settings: vec![],
        skybox: SkyboxSettings { skybox_id: 0, config: 0, light_mode: 1 },
        keep_object: None,
        keep_object_id: None,
        c_up_elf_msg_num: 0,
        entrances: vec![Spawn { spawn: 0, room: 0 }],
        exits: vec![],
        transition_actors: vec![],
        paths: vec![],
        scene_cam_type: 0,
        sound: None,
        cutscene: None,
        rooms: vec![level.room.file.clone()],
        bake_day_time: 0,
        notes: level.notes.clone(),
    };
    SceneState {
        data: SceneData { name: format!("{}_scene", level.name), id: 0, draw_config: String::new(), layers: vec![layer] },
        layer: 0,
        rooms: vec![Arc::new(level.room.clone())],
        lights,
        draw: DrawConfigState { child, ..Default::default() },
        segments: None,
        all_rooms: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 2000-square floor of two triangles, a wall, and a pond's water surface, as the
    /// overworld export writes them.
    fn write_level(dir: &Path) {
        std::fs::create_dir_all(dir.join("textures")).unwrap();
        image::RgbaImage::from_pixel(2, 2, image::Rgba([200, 180, 120, 255])).save(dir.join("textures/g.png")).unwrap();
        image::RgbaImage::from_pixel(2, 2, image::Rgba([150, 140, 50, 255])).save(dir.join("textures/d.png")).unwrap();
        let j = serde_json::json!({
            "name": "test_level",
            "materials": [
                {"name": "ground", "file": "textures/g.png", "wrap_u": "repeat", "wrap_v": "repeat", "alpha": "opaque", "opacity": 1.0, "cull": "back"},
                {"name": "water", "file": "textures/g.png", "wrap_u": "repeat", "wrap_v": "repeat", "alpha": "blend", "opacity": 0.5, "cull": "none"},
                {"name": "kf_shadow", "file": "textures/g.png", "wrap_u": "clamp", "wrap_v": "clamp", "alpha": "blend", "opacity": 1.0, "cull": "back", "decal": true},
                {"name": "ground+dirt", "file": "textures/g.png", "file2": "textures/d.png", "blend": "vertex", "wrap_u": "repeat", "wrap_v": "repeat", "alpha": "opaque", "opacity": 1.0, "cull": "back"}
            ],
            "surfaces": ["ground", "wall_nograb", "water", "crawl"],
            "objects": [
                {"name": "ground", "verts": [-1000, -1000, 0, 1000, -1000, 0, 1000, 1000, 0, -1000, 1000, 0], "tris": [0, 1, 2, 0, 2, 3],
                 "uvs": [0, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1], "colors": [100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100],
                 "material": [0, 0], "surface": [0, 0]},
                {"name": "walls", "verts": [-1000, 1000, 0, 1000, 1000, 0, 1000, 1000, 300], "tris": [0, 2, 1],
                 "uvs": [0, 0, 1, 1, 1, 0], "colors": [], "material": [0], "surface": [1]},
                {"name": "water", "verts": [0, 0, -20, 200, 0, -20, 200, 100, -20], "tris": [0, 1, 2],
                 "uvs": [0, 0, 1, 0, 1, 1], "colors": [], "material": [1], "surface": [2]},
                {"name": "props", "verts": [0, 0, 0, 50, 0, 0, 50, 50, 0], "tris": [0, 1, 2],
                 "uvs": [0, 0, 1, 0, 1, 1], "colors": [], "material": [2], "surface": [-1]},
                {"name": "props_collision", "render": false, "verts": [-500, -500, 100, -400, -500, 100, -400, -400, 100], "tris": [0, 1, 2],
                 "uvs": [0, 0, 0, 0, 0, 0], "colors": [], "material": [-1], "surface": [3]},
                {"name": "dirt", "verts": [0, 0, 1, 10, 0, 1, 10, 10, 1], "tris": [0, 1, 2], "alpha": [255, 128, 0],
                 "uvs": [0, 0, 1, 0, 1, 1], "colors": [], "material": [3], "surface": [-1]}
            ]
        });
        std::fs::write(dir.join("level.json"), j.to_string()).unwrap();
    }

    #[test]
    fn a_level_becomes_a_room_collision_and_water() {
        let dir = std::env::temp_dir().join(format!("oot_level_test_{}", std::process::id()));
        write_level(&dir);
        let l = load(&dir).unwrap();
        assert!(l.notes.is_empty(), "{:?}", l.notes);
        assert_eq!(l.triangles, 6, "the collision-only triangle isn't drawn");
        let e = &l.room.entries[0];
        let opa = e.opa.as_ref().unwrap();
        assert_eq!(opa.triangle_count(), 4);
        // the dirt: two textures, two cycles, the weights as vertex alpha
        let b = opa.batches.iter().find(|b| opa.materials[b.material].textures[1].is_some()).unwrap();
        assert!(opa.materials[b.material].two_cycle);
        assert_eq!(b.vertices.iter().map(|v| v.color[3]).collect::<Vec<_>>(), vec![255, 128, 0]);
        // water is translucent, at half alpha, and not collision but a water box
        let x = e.xlu.as_ref().unwrap();
        assert_eq!((x.triangle_count(), x.batches[0].vertices[0].color[3]), (2, 128));
        // the door shadow draws as a decal
        assert!(x.batches.iter().any(|b| x.materials[b.material].decal));
        assert_eq!(l.collision.polys.len(), 4);
        // the kit's collision-only crawlspace wall is WALL_TYPE 5
        let crawl = l.collision.polys.iter().find(|p| p.normal[1] > 0 && l.collision.vertex(p.vtx[0] as usize & 0x1FFF).y > 50.0).unwrap();
        assert_eq!(l.collision.surface_types[crawl.ty as usize].data, [0x00A0_0000, 0x0000_0FCA]);
        let wb = &l.collision.water_boxes[0];
        assert_eq!((wb.x_min, wb.z_min, wb.x_length, wb.z_length, wb.y_surface), (0, -100, 200, 100, -20));
        // the floor faces up in the game's axes (y up), the wall can't be grabbed
        assert!(l.collision.polys[0].normal[1] > 32000);
        let wall = l.collision.polys.iter().find(|p| p.normal[1] == 0).unwrap();
        assert_eq!(l.collision.surface_types[wall.ty as usize].data, [0x0020_0000, 0x0000_0FC8]);
        // Link starts on the floor in the middle, facing north
        assert_eq!(l.spawn, (Vec3::ZERO, -0x8000));
        let s = scene(&l, EnvLights { ambient: [80; 3], light1_dir: [0; 3], light1_color: [0; 3], light2_dir: [0; 3], light2_color: [0; 3], fog_color: [0; 3], fog_near: 990, fog_far: 12800 }, true);
        assert_eq!((s.rooms.len(), s.layer_data().spawns.len()), (1, 1));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
