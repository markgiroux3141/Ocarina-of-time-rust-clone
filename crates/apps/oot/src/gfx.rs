//! Drawing helpers for the play mode: collision meshes as unlit vertex-coloured draw lists,
//! a blob shadow, and Link, from the pack's meshes, posed with Player's look/lean overrides.

use std::collections::HashMap;

use eng_collision::collision::CollisionHeader;
use eng_gfx::{Batch, BlendMode, CullMode, DrawList, Material, NO_BONE, Vertex};
use eng_gfx::combiner::{Combiner, encode};
use eng_render::LineVertex;
use glam::{Vec2, Vec3};
use oot_game::pack::GamePack;
use oot_game::player_lib::{Age, LinkFaces, LinkVariant};

/// A material whose colour is the vertex colour (`G_CC_SHADE`), unlit.
fn shade_material(blend: BlendMode) -> Material {
    // Colour: (0 - 0) * 0 + SHADE; alpha: (0 - 0) * 0 + SHADE.
    let raw = encode([15, 15, 31, 4, 7, 7, 7, 4], [15, 15, 31, 4, 7, 7, 7, 4]);
    Material {
        combiner: Combiner::decode(raw),
        two_cycle: false,
        prim: [255; 4],
        prim_lod_frac: 0,
        env: [255; 4],
        fog: [0; 4],
        blend_color: [0; 4],
        geometry_mode: 0,
        othermode_h: 0,
        othermode_l: 0,
        textures: [None, None],
        blend,
        cull: CullMode::None,
        depth_test: true,
        depth_write: blend != BlendMode::Translucent,
        decal: blend == BlendMode::Translucent,
        lit: false,
        texgen: false,
        bilinear: false,
        uv_dyn: [None, None],
        env_dyn: None,
        prim_dyn: None,
        fog_blend: false,
    }
}

fn vtx(pos: Vec3, color: [u8; 4]) -> Vertex {
    Vertex { bone: NO_BONE, pos, normal: Vec3::Y, color, uv: [Vec2::ZERO; 2] }
}

/// Colours a collision mesh by surface class with light baked in: floors green with a
/// 100-unit checkerboard, walls stone, ceilings dark. Returns the draw list.
pub fn collision_draw_list(h: &CollisionHeader) -> DrawList {
    let light = Vec3::new(0.35, 0.85, 0.4).normalize();
    let mut verts = Vec::with_capacity(h.polys.len() * 3);
    for p in &h.polys {
        let [a, b, c] = h.triangle(p);
        let n = Vec3::new(p.normal[0] as f32, p.normal[1] as f32, p.normal[2] as f32) / 32767.0;
        let centre = (a + b + c) / 3.0;
        let base = if n.y > 0.5 {
            let checker = ((centre.x / 100.0).floor() as i32 + (centre.z / 100.0).floor() as i32) & 1 == 0;
            // Tint by height so levels read apart.
            let t = (centre.y / 300.0).clamp(-1.0, 1.0);
            let g = if checker { Vec3::new(0.36, 0.55, 0.30) } else { Vec3::new(0.30, 0.48, 0.26) };
            g + Vec3::new(0.12, 0.08, 0.02) * t.max(0.0) - Vec3::new(0.05, 0.1, 0.05) * (-t).max(0.0)
        } else if n.y < -0.8 {
            Vec3::new(0.30, 0.28, 0.33)
        } else {
            let checker = (((centre.x + centre.z) / 100.0).floor() as i32 + (centre.y / 100.0).floor() as i32) & 1 == 0;
            if checker { Vec3::new(0.62, 0.58, 0.52) } else { Vec3::new(0.56, 0.52, 0.47) }
        };
        let shade = 0.45 + 0.55 * n.dot(light).abs();
        let c8 = |v: Vec3| {
            let v = (v * shade).clamp(Vec3::ZERO, Vec3::ONE) * 255.0;
            [v.x as u8, v.y as u8, v.z as u8, 255]
        };
        let col = c8(base);
        verts.push(vtx(a, col));
        verts.push(vtx(b, col));
        verts.push(vtx(c, col));
    }
    let mut d = DrawList::default();
    d.materials.push(shade_material(BlendMode::Opaque));
    d.batches.push(Batch { material: 0, vertices: verts });
    d
}

/// Polygon edges as line pairs (for the collision overlay).
pub fn collision_lines(h: &CollisionHeader, lift: f32) -> Vec<LineVertex> {
    let mut out = Vec::with_capacity(h.polys.len() * 6);
    for p in &h.polys {
        let n = Vec3::new(p.normal[0] as f32, p.normal[1] as f32, p.normal[2] as f32) / 32767.0;
        let col = if n.y > 0.5 {
            [0.3, 1.0, 0.4, 0.8]
        } else if n.y < -0.8 {
            [1.0, 0.4, 0.9, 0.8]
        } else {
            [1.0, 0.75, 0.3, 0.8]
        };
        let t = h.triangle(p).map(|v| v + n * lift);
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            out.push(LineVertex { pos: a.to_array(), color: col });
            out.push(LineVertex { pos: b.to_array(), color: col });
        }
    }
    out
}

/// A dummy target actor: a 30×60×30 box (origin at its base centre), lit by a fixed light.
pub fn target_draw_list() -> DrawList {
    box_draw_list(60.0, 15.0, [200, 70, 60])
}

/// A placeholder's marker (an actor that isn't ported yet): a magenta box 20 wide and 20 high.
pub fn placeholder_draw_list() -> DrawList {
    box_draw_list(20.0, 10.0, [220, 60, 220])
}

/// A box `h` high and `2r` wide standing on the origin, flat-shaded in `rgb`.
fn box_draw_list(h: f32, r: f32, rgb: [u8; 3]) -> DrawList {
    let light = Vec3::new(0.35, 0.85, 0.4).normalize();
    let mut verts = Vec::new();
    let c = |x: f32, y: f32, z: f32| Vec3::new(x * r, y * h, z * r);
    let faces = [
        ([c(-1.0, 0.0, 1.0), c(1.0, 0.0, 1.0), c(1.0, 1.0, 1.0), c(-1.0, 1.0, 1.0)], Vec3::Z),
        ([c(1.0, 0.0, -1.0), c(-1.0, 0.0, -1.0), c(-1.0, 1.0, -1.0), c(1.0, 1.0, -1.0)], -Vec3::Z),
        ([c(1.0, 0.0, 1.0), c(1.0, 0.0, -1.0), c(1.0, 1.0, -1.0), c(1.0, 1.0, 1.0)], Vec3::X),
        ([c(-1.0, 0.0, -1.0), c(-1.0, 0.0, 1.0), c(-1.0, 1.0, 1.0), c(-1.0, 1.0, -1.0)], -Vec3::X),
        ([c(-1.0, 1.0, 1.0), c(1.0, 1.0, 1.0), c(1.0, 1.0, -1.0), c(-1.0, 1.0, -1.0)], Vec3::Y),
    ];
    for (q, n) in faces {
        let s = 0.45 + 0.55 * n.dot(light).abs();
        let col = [(rgb[0] as f32 * s) as u8, (rgb[1] as f32 * s) as u8, (rgb[2] as f32 * s) as u8, 255];
        for i in [0, 1, 2, 0, 2, 3] {
            verts.push(vtx(q[i], col));
        }
    }
    let mut d = DrawList::default();
    let mut m = shade_material(BlendMode::Opaque);
    m.cull = CullMode::Back;
    d.materials.push(m);
    d.batches.push(Batch { material: 0, vertices: verts });
    d
}

/// The course's water boxes as translucent blue planes at their surface height (scenes draw
/// their own water in the room meshes).
pub fn water_draw_list(h: &CollisionHeader) -> DrawList {
    let mut verts = Vec::new();
    let col = [40, 90, 170, 130];
    for w in &h.water_boxes {
        let (x0, z0, y) = (w.x_min as f32, w.z_min as f32, w.y_surface as f32);
        let (x1, z1) = (x0 + w.x_length as f32, z0 + w.z_length as f32);
        let q = [Vec3::new(x0, y, z1), Vec3::new(x1, y, z1), Vec3::new(x1, y, z0), Vec3::new(x0, y, z0)];
        for i in [0, 1, 2, 0, 2, 3] {
            verts.push(vtx(q[i], col));
        }
    }
    let mut d = DrawList::default();
    d.materials.push(shade_material(BlendMode::Translucent));
    d.batches.push(Batch { material: 0, vertices: verts });
    d
}

/// A soft dark disc (radius 1, at the origin) for the blob shadow; place it with the root
/// matrix.
pub fn shadow_draw_list() -> DrawList {
    let mut verts = Vec::new();
    let n = 24;
    for i in 0..n {
        let a0 = i as f32 / n as f32 * std::f32::consts::TAU;
        let a1 = (i + 1) as f32 / n as f32 * std::f32::consts::TAU;
        verts.push(vtx(Vec3::ZERO, [0, 0, 0, 140]));
        verts.push(vtx(Vec3::new(a0.cos(), 0.0, a0.sin()), [0, 0, 0, 0]));
        verts.push(vtx(Vec3::new(a1.cos(), 0.0, a1.sin()), [0, 0, 0, 0]));
    }
    let mut d = DrawList::default();
    d.materials.push(shade_material(BlendMode::Translucent));
    d.batches.push(Batch { material: 0, vertices: verts });
    d
}

/// Link's meshes from the pack: every age, model group and hand state (by record name), and
/// the face textures they're drawn with.
pub struct LinkGfx {
    variants: HashMap<String, (Age, LinkVariant)>,
    faces: [LinkFaces; 2],
}

impl LinkGfx {
    pub fn load(pack: &GamePack) -> anyhow::Result<LinkGfx> {
        let mut variants = HashMap::new();
        for age in [Age::Adult, Age::Child] {
            for key in pack.link_variant_names(age) {
                let v: LinkVariant = pack.assets.get(&key)?;
                variants.insert(key, (age, v));
            }
        }
        Ok(LinkGfx { variants, faces: [pack.link_faces(Age::Adult)?, pack.link_faces(Age::Child)?] })
    }

    /// The mesh `name` (a `player/<age>/<limb lists>` record) with `eye` and `mouth` bound to
    /// segments 8 and 9.
    pub fn mesh(&self, name: &str, eye: usize, mouth: usize) -> Option<DrawList> {
        let (age, v) = self.variants.get(name)?;
        Some(v.with_face(&self.faces[*age as usize], eye, mouth))
    }
}
