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
//! Played two ways: in the spikes' view (`scene`: the room, collision and spawn handed to a play
//! state directly), or by `Play_Init` as a scene of its own (`add_scene` gives it a new scene id
//! and entrances in the game's tables, `write_scene` its records over the pack, in its theme's
//! scene's header: docs/EDITOR-PLAY-IN-THE-GAME.md).
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
    /// The game scene its theme was cut from (spot07 for Zora's Domain), whose environment (lights
    /// and fog) it's played in.
    pub scene: Option<String>,
    /// How it plays when entered by `Play_Init` (`write_scene`).
    pub game: Dynamics,
    /// The game's actors placed in it (`write_scene` puts them in the room).
    pub actors: Vec<LevelActor>,
}

/// An actor placed in a level (`level.json`'s `actors`, the editor's actors tab): by its
/// overlay's name, at a position in the game's axes, with the game's rotation and params.
#[derive(Debug, Clone, PartialEq)]
pub struct LevelActor {
    /// The overlay: "En_Kusa".
    pub actor: String,
    pub pos: Vec3,
    pub rot: [i16; 3],
    pub params: i16,
    /// The game layers it's in (child day, child night, adult day, adult night).
    pub layers: [bool; 4],
}

impl LevelActor {
    fn read(j: &Value) -> Option<LevelActor> {
        let p = floats(j, "pos");
        let r = ints(j, "rot");
        let layers = match j["layers"].as_array() {
            Some(a) => [0, 1, 2, 3].map(|l| a.get(l).and_then(Value::as_bool).unwrap_or(false)),
            None => [true; 4],
        };
        Some(LevelActor {
            actor: j["actor"].as_str()?.to_string(),
            pos: game([*p.first()?, *p.get(1)?, *p.get(2)?]),
            rot: [0, 1, 2].map(|k| r.get(k).copied().unwrap_or(0) as i16),
            params: j["params"].as_i64().unwrap_or(0) as u16 as i16,
            layers,
        })
    }
}

/// How a level plays when entered by `Play_Init` (`level.json`'s `game`, the editor's Level
/// settings): the scene whose header and first room it plays with, and changes to them. `None`
/// takes the start's.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Dynamics {
    /// The scene file (spot17) it starts from; its theme's scene if `None`.
    pub start_from: Option<String>,
    /// Another scene's music and nature ambience, or "none" for silence.
    pub music: Option<String>,
    /// A hot room (`ROOM_ENV_HOT`: the heat timer), or not.
    pub hot: Option<bool>,
    /// The sky drawn (the room's skybox on), or not.
    pub sky: Option<bool>,
    /// The clock running (the room's time speed), or stopped.
    pub clock: Option<bool>,
    /// Objects the room loads besides its actors' own, by file (`object_kusa`).
    pub objects: Vec<String>,
}

impl Dynamics {
    fn read(j: &Value) -> Dynamics {
        let s = |k: &str| j[k].as_str().filter(|s| !s.is_empty()).map(str::to_string);
        let objects = j["objects"].as_array().map(|a| a.iter().filter_map(|o| o.as_str().map(str::to_string)).collect()).unwrap_or_default();
        Dynamics { start_from: s("start_from"), music: s("music"), hot: j["hot"].as_bool(), sky: j["sky"].as_bool(), clock: j["clock"].as_bool(), objects }
    }
}

/// `ROOM_ENV_HOT` (`room.h`): a hot room's `environmentType` (`behaviorType2`).
pub const ROOM_ENV_HOT: u8 = 3;

/// The time speed a running clock gets where its start's stands still: the overworld's
/// (Kokiri Forest's room header, `SCENE_CMD_TIME_SETTINGS(0xFF, 0xFF, 10)`).
const CLOCK_SPEED: u8 = 10;

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
                source_addr: None,
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
        d.batches.push(Batch { material: k.1, vertices, sources: Vec::new() });
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
        wind: None,
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
    let scene = j["scene"].as_str().map(str::to_string);
    let game = Dynamics::read(&j["game"]);
    let mut actors = vec![];
    for (i, a) in j["actors"].as_array().map(Vec::as_slice).unwrap_or_default().iter().enumerate() {
        match LevelActor::read(a) {
            Some(a) => actors.push(a),
            None => notes.push(format!("actor {i}: no overlay name or position: left out")),
        }
    }
    Ok(Level { name, room, collision, spawn: (Vec3::new(mid.x, y, mid.z), -0x8000), triangles, notes, scene, game, actors })
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
        segment_textures: Default::default(),
        texture_cache: Default::default(),
        all_rooms: true,
    }
}

/// The scene file a custom level is entered as by `Play_Init` (`add_scene`, `write_scene`).
pub const SCENE_FILE: &str = "custom_level_scene";

/// The scene a level that names none is played as: Kokiri Forest, the first theme's.
pub const DEFAULT_THEME: &str = "spot04";

/// `PLAYER_PARAMS(PLAYER_START_MODE_IDLE, PLAYER_START_BG_CAM_DEFAULT)`: Link standing at the
/// spawn, as the void-out respawn puts him.
const SPAWN_PARAMS: i16 = 0x0DFF;

/// A custom level as the game has it: its own scene id after the table's last, and the entrance
/// to its spawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entered {
    pub scene_id: u16,
    pub entrance: u16,
}

/// Adds a custom level to the game's tables as a scene of its own, for `Play_Init` to enter: a
/// scene entry with a new id after the table's last (`SCENE_FILE`, the default draw config, no
/// title card), four entrance rows to its spawn (one per game layer: child day and night, adult
/// day and night), and the HUD's restrictions copied from the scene it starts from (its
/// `Dynamics::start_from`, else its theme's; Kokiri Forest's if that isn't in the table). The
/// scene's header and room come from `write_scene`.
pub fn add_scene(g: &mut oot_game::play_scene::GameAssets, level: &Level) -> Result<Entered> {
    let theme = level.game.start_from.as_deref().or(level.scene.as_deref()).unwrap_or(DEFAULT_THEME);
    use oot_game::scene::{EntranceInfo, SceneEntry};
    use oot_game::transition::TRANS_TYPE_FADE_BLACK;
    let t = &mut g.scenes;
    ensure!(t.by_file(SCENE_FILE).is_none(), "a custom level is already in the scene table");
    let scene_id = t.scenes.len() as u16;
    // the save keeps scene flags for the first 124 ids (sceneFlags[124])
    ensure!((scene_id as usize) < oot_game::save::SCENE_FLAGS_COUNT, "the scene table has no room for a custom level (id {scene_id:#x})");
    t.scenes.push(SceneEntry {
        id: scene_id,
        file: SCENE_FILE.to_string(),
        enum_name: "SCENE_CUSTOM_LEVEL".to_string(),
        draw_config: "SDC_DEFAULT".to_string(),
        title_file: String::new(),
    });
    let entrance = t.entrances.len() as u16;
    for layer in 0..4 {
        t.entrances.push(EntranceInfo {
            name: format!("ENTR_CUSTOM_LEVEL_{layer}"),
            scene: scene_id,
            spawn: 0,
            continue_bgm: false,
            title_card: false,
            end_trans_type: TRANS_TYPE_FADE_BLACK,
            start_trans_type: TRANS_TYPE_FADE_BLACK,
        });
    }
    let theme_id = t.by_file(&format!("{theme}_scene")).or_else(|| t.by_file(&format!("{DEFAULT_THEME}_scene"))).map(|s| s.id);
    let r = &mut g.interface.restrictions;
    let row = theme_id.and_then(|id| r.iter().take_while(|e| e[0] != 0xFF).find(|e| e[0] as u16 == id).copied());
    if let Some([_, f1, f2, f3]) = row {
        let end = r.iter().position(|e| e[0] == 0xFF).unwrap_or(r.len());
        r.insert(end, [scene_id as u8, f1, f2, f3]);
    }
    Ok(Entered { scene_id, entrance })
}

/// How many objects a room may list: the object banks (`OBJECT_EXCHANGE_BANK_MAX`) less the
/// ones loaded before the room's: `gameplay_keep`, the scene's keep (if it has one) and Link's.
pub fn room_object_limit(keep: bool) -> usize {
    oot_game::object_ctx::OBJECT_EXCHANGE_BANK_MAX - 2 - keep as usize
}

/// The objects an actor needs besides its profile's, by its overlay's name, params and rotation
/// (its schema's `ObjectNeed`s, ADR 0066: the app hands in `oot_actors::PROFILES`' own).
pub type Needs<'a> = &'a dyn Fn(&str, i16, [i16; 3]) -> Vec<&'static str>;

/// A game layer's room actors and object list, from the level's actors in that layer (`layer`
/// 0 to 3): each actor's id by its overlay's name (`table`), and its objects in the list (its
/// profile's, then what it needs for its params, `needs`) unless it's `gameplay_keep` or the
/// scene's keep (`keep`), then `extra`'s objects (`objects` is the object table's file names).
/// An actor the table hasn't got is left out with a note; objects past the room's limit too
/// (their actors then wait for them, never appearing).
#[allow(clippy::too_many_arguments)]
pub fn room_actors(
    actors: &[LevelActor],
    extra: &[String],
    layer: usize,
    table: &oot_game::actor_table::ActorTable,
    objects: &[String],
    keep: Option<i16>,
    needs: Needs,
    notes: &mut Vec<String>,
) -> (Vec<ActorEntry>, Vec<i16>) {
    use oot_game::object_ctx::{OBJECT_GAMEPLAY_KEEP, OBJECT_INVALID};
    let mut entries = vec![];
    let mut list: Vec<i16> = vec![];
    let add = |id: i16, list: &mut Vec<i16>| {
        if id != OBJECT_INVALID && id != OBJECT_GAMEPLAY_KEEP && Some(id) != keep && !list.contains(&id) {
            list.push(id);
        }
    };
    for (i, a) in actors.iter().enumerate().filter(|(_, a)| a.layers[layer]) {
        let Some(id) = table.actors.iter().position(|t| !t.name.is_empty() && t.name == a.actor) else {
            // once, in the first layer it's in
            if a.layers.iter().position(|&x| x) == Some(layer) {
                notes.push(format!("actor {i}: no actor {:?} in the game's table: left out", a.actor));
            }
            continue;
        };
        let p = a.pos.round().clamp(Vec3::splat(-32768.0), Vec3::splat(32767.0));
        entries.push(ActorEntry { id: id as i16, pos: [p.x as i16, p.y as i16, p.z as i16], rot: a.rot, params: a.params });
        if let Some(init) = &table.actors[id].init {
            add(init.object_id, &mut list);
        }
        for o in needs(&a.actor, a.params, a.rot) {
            match objects.iter().rposition(|f| f == o) {
                Some(oid) => add(oid as i16, &mut list),
                None => notes.push(format!("actor {i} ({}): no object {o:?} in the game's table", a.actor)),
            }
        }
    }
    for o in extra {
        match objects.iter().rposition(|f| f == o) {
            Some(id) => add(id as i16, &mut list),
            None => notes.push(format!("no object {o:?} in the game's table: left out")),
        }
    }
    let limit = room_object_limit(keep.is_some());
    if list.len() > limit {
        let names: Vec<&str> = list[limit..].iter().map(|&id| objects.get(id as usize).map_or("?", String::as_str)).collect();
        notes.push(format!("layer {layer}: the room can load {limit} objects, not {}: {} left out, and their actors won't appear", list.len(), names.join(", ")));
        list.truncate(limit);
    }
    (entries, list)
}

/// Writes `level` into `src` (layered over the pack) as `add_scene`'s scene. For each game
/// layer, the header of the scene it starts from (`Dynamics::start_from`, else its theme's): the
/// music and nature ambience, the keep object, Navi's C-Up texts, the scene camera type; with its
/// theme's scene's look (the environment's lights and fog, the skybox and light mode), the level's
/// spawn at `spawn`, its collision and one room. The room's header (its behaviour, such as a hot
/// room's, the echo, the time settings, skybox and sun off) is the start's first room's. Then the
/// level's changes (`Dynamics`): another scene's music or none, hot or not, the sky, the clock.
/// Each game layer has a room of its own with the level's actors in that layer (`room_actors`).
/// A scene that won't load gives way to Kokiri Forest. Rewritten each time the level is rebuilt;
/// `Play_Init` reads the new records.
pub fn write_scene(level: &Level, pack: &oot_game::pack::GamePack, src: &eng_asset::MemSource, spawn: (Vec3, i16), needs: Needs) -> Result<()> {
    use oot_game::audio::{NA_BGM_NO_MUSIC, NATURE_ID_NONE};
    use oot_game::pack::keys;
    use oot_game::scene::SoundSettings;
    let load = |name: &str| match pack.scene(name) {
        Ok(t) => Ok(t),
        Err(e) if name != DEFAULT_THEME => {
            log::warn!("{name} won't load ({e:#}): Kokiri Forest's instead");
            pack.scene(DEFAULT_THEME)
        }
        Err(e) => Err(e),
    };
    let g = &level.game;
    let look = load(level.scene.as_deref().unwrap_or(DEFAULT_THEME))?;
    let start = match &g.start_from {
        Some(s) => load(s)?,
        None => look.clone(),
    };
    let music = match g.music.as_deref() {
        None => None,
        Some("none") => Some(None),
        Some(m) => Some(Some(load(m)?)),
    };
    let (pos, yaw) = spawn;
    let collision = keys::scene_collision(SCENE_FILE, 0);
    let mut layers = Vec::new();
    for l in 0..4 {
        let (_, sl) = start.layer(l);
        let (_, ll) = look.layer(l);
        let sound = match &music {
            None => sl.sound,
            Some(Some(m)) => m.layer(l).1.sound,
            Some(None) => Some(SoundSettings { spec_id: sl.sound.map_or(0, |s| s.spec_id), nature_ambience_id: NATURE_ID_NONE, seq_id: NA_BGM_NO_MUSIC as u8 }),
        };
        layers.push(LayerData {
            header_offset: 0,
            collision: collision.clone(),
            spawns: vec![ActorEntry { id: 0, pos: [pos.x.round() as i16, pos.y.round() as i16, pos.z.round() as i16], rot: [0, yaw, 0], params: SPAWN_PARAMS }],
            light_settings: ll.light_settings.clone(),
            skybox: ll.skybox,
            entrances: vec![Spawn { spawn: 0, room: 0 }],
            exits: vec![],
            transition_actors: vec![],
            paths: vec![],
            sound,
            cutscene: None,
            rooms: vec![keys::room(SCENE_FILE, l, 0)],
            notes: level.notes.clone(),
            ..sl.clone()
        });
    }
    let first = start.layers[0].rooms.first().map(|k| pack.room(k)).transpose()?;
    let mut room = RoomData { file: format!("{SCENE_FILE}_room_0"), ..level.room.clone() };
    if let Some(r) = first {
        room.behavior = r.behavior;
        room.echo = r.echo;
        room.time = r.time;
        room.wind = r.wind;
        room.skybox_disabled = r.skybox_disabled;
        room.sun_moon_disabled = r.sun_moon_disabled;
    }
    if let Some(hot) = g.hot {
        room.behavior[1] = if hot {
            ROOM_ENV_HOT
        } else if room.behavior[1] == ROOM_ENV_HOT {
            0
        } else {
            room.behavior[1]
        };
    }
    if let Some(sky) = g.sky {
        room.skybox_disabled = !sky;
    }
    if let Some(clock) = g.clock {
        let [h, m, speed] = room.time.unwrap_or([0xFF, 0xFF, 0]);
        let running = speed != 0 && speed != 0xFF;
        room.time = Some([h, m, if !clock { 0 } else if running { speed } else { CLOCK_SPEED }]);
    }
    let table = pack.actor_table()?;
    let objects = pack.scene_table()?.objects;
    for (l, layer) in layers.iter_mut().enumerate() {
        let mut notes = vec![];
        let (actors, list) = room_actors(&level.actors, &g.objects, l, &table, &objects, layer.keep_object_id, needs, &mut notes);
        for n in &notes {
            log::warn!("{n}");
        }
        layer.notes.extend(notes);
        src.put(&keys::room(SCENE_FILE, l, 0), &RoomData { actors, objects: list, ..room.clone() })?;
    }
    let scene = SceneData { name: SCENE_FILE.to_string(), id: 0, draw_config: "SDC_DEFAULT".to_string(), layers };
    src.put(&keys::scene(SCENE_FILE), &scene)?;
    src.put(&collision, &level.collision)?;
    Ok(())
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

    /// A room's actors and objects from the level's: ids by name, each actor's object once (not
    /// the keeps), the layers, the extra objects, and the room's object limit.
    #[test]
    fn room_actors_take_their_ids_and_objects() {
        use oot_game::actor_table::{ActorInfo, ActorInitInfo, ActorTable};
        let row = |name: &str, object_id: i16| ActorInfo {
            enum_name: format!("ACTOR_{}", name.to_uppercase()),
            name: name.to_string(),
            alloc: 0,
            internal: false,
            init: Some(ActorInitInfo { id: 0, category: 6, flags: 0, object_id, init: None, destroy: None, update: None, draw: None }),
        };
        // ids 0..4: Player, a pot (gameplay_keep), a sign (object 5), a rock (field keep 2), a Keese (object 6)
        let table = ActorTable { actors: vec![row("Player", 1), row("Obj_Tsubo", 1), row("En_Kanban", 5), row("En_Ishi", 2), row("En_Firefly", 6)] };
        let objects: Vec<String> = (0..30).map(|i| format!("object_{i}")).collect();
        let at = |actor: &str, layers: [bool; 4]| LevelActor { actor: actor.into(), pos: Vec3::new(10.4, 0.0, -20.6), rot: [0, 0x4000, 0], params: 0x0103, layers };
        let actors = vec![at("Obj_Tsubo", [true; 4]), at("En_Kanban", [true; 4]), at("En_Ishi", [true; 4]), at("En_Firefly", [false, true, false, true]), at("En_Kanban", [true; 4]), at("Nobody", [true; 4])];
        // the pot needs object 7 for its params (bit 8 set, as `Obj_Tsubo`'s object_tsubo)
        let none = |_: &str, _: i16, _: [i16; 3]| vec![];
        let pot = |n: &str, params: i16, _: [i16; 3]| if n == "Obj_Tsubo" && params & 0x100 != 0 { vec!["object_7"] } else { vec![] };
        let mut notes = vec![];
        let (e, list) = room_actors(&actors, &["object_9".into(), "object_nowhere".into()], 0, &table, &objects, Some(2), &none, &mut notes);
        assert_eq!(e.iter().map(|x| x.id).collect::<Vec<_>>(), vec![1, 2, 3, 2]);
        assert_eq!((e[0].pos, e[0].rot, e[0].params), ([10, 0, -21], [0, 0x4000, 0], 0x0103));
        // the sign's object once, no keeps, then the extra object
        assert_eq!(list, vec![5, 9]);
        assert!(notes.iter().any(|n| n.contains("Nobody")) && notes.iter().any(|n| n.contains("object_nowhere")), "{notes:?}");
        // by night the Keese comes, with its object; the unknown actor isn't noted again
        let mut notes = vec![];
        let (e, list) = room_actors(&actors, &[], 1, &table, &objects, Some(2), &none, &mut notes);
        assert_eq!((e.len(), list), (5, vec![5, 6]));
        assert!(notes.is_empty(), "{notes:?}");
        // a scene without a keep lists the field keep's actors' object
        let (_, list) = room_actors(&actors, &[], 0, &table, &objects, None, &none, &mut vec![]);
        assert_eq!(list, vec![5, 2]);
        // what an actor needs for its params comes first after its profile's
        let (_, list) = room_actors(&actors, &[], 0, &table, &objects, Some(2), &pot, &mut vec![]);
        assert_eq!(list, vec![7, 5]);
        // past the limit: cut, with a note
        let many: Vec<String> = (10..30).map(|i| format!("object_{i}")).collect();
        let mut notes = vec![];
        let (_, list) = room_actors(&[], &many, 0, &table, &objects, Some(2), &none, &mut notes);
        assert_eq!(list.len(), room_object_limit(true));
        assert_eq!(room_object_limit(true), 16);
        assert!(notes[0].contains("can load 16 objects, not 20"), "{notes:?}");
    }
}
