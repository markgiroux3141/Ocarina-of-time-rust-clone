//! Drawing helpers for the play mode: collision meshes as unlit vertex-coloured draw lists,
//! a blob shadow, and Link posed with Player's look/lean overrides.

use std::collections::HashMap;

use glam::{Mat4, Vec2, Vec3};
use oot_core::anim::JointTable;
use oot_core::collision::CollisionHeader;
use oot_core::combiner::{Combiner, encode};
use oot_core::gbi::{Batch, BlendMode, CullMode, DrawList, Material, NO_BONE, Vertex};
use oot_core::player::{Age, Loadout, PlayerModel, PlayerRules};
use oot_core::skeleton::{Skeleton, local_transform};
use oot_game::player::LookRotations;
use oot_render::{GpuModel, LineVertex, Renderer};

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
    let (h, r) = (60.0, 15.0);
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
        let col = [(200.0 * s) as u8, (70.0 * s) as u8, (60.0 * s) as u8, 255];
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

/// The Z-target reticle: three triangles pointing at the focus, `size` units out (the game's
/// `unk_44`, 500 → 80 while locking), in the camera's plane.
pub fn reticle_lines(focus: Vec3, size: f32, eye: Vec3, spin: f32) -> Vec<LineVertex> {
    let fwd = (focus - eye).normalize_or_zero();
    let right = fwd.cross(Vec3::Y).normalize_or_zero();
    let up = right.cross(fwd);
    let col = [1.0, 0.25, 0.2, 1.0];
    let mut out = Vec::new();
    let r = size * 0.15;
    for k in 0..3 {
        let a = spin + k as f32 * std::f32::consts::TAU / 3.0;
        let dir = right * a.cos() + up * a.sin();
        let side = right * (-a.sin()) + up * a.cos();
        let tip = focus + dir * r;
        let (b0, b1) = (focus + dir * (r + 8.0) + side * 5.0, focus + dir * (r + 8.0) - side * 5.0);
        for (p, q) in [(tip, b0), (b0, b1), (b1, tip)] {
            out.push(LineVertex { pos: p.to_array(), color: col });
            out.push(LineVertex { pos: q.to_array(), color: col });
        }
    }
    out
}

/// `BgYdanHasi_Draw` for the floating block: `Gfx_DrawDListOpa(gDTSlidingPlatformDL)` with
/// `object_ydan_objects` on segment 6 (`Gfx_SetupDL_25Opa` first). Pose it with the bg
/// actor's transform (`Actor_Draw`'s matrix is the same translate/rotate/scale).
pub fn platform_draw_list(p: &oot_core::project::Project) -> anyhow::Result<DrawList> {
    use anyhow::Context;
    use oot_game::bg_ydan_hasi::{DISPLAY_LIST, OBJECT};
    let sym = p.symbols.file(OBJECT).context("object_ydan_objects.xml")?.find(DISPLAY_LIST).context(DISPLAY_LIST)?;
    let mut segments: [Option<oot_core::gbi::Segment>; 16] = Default::default();
    segments[6] = Some(oot_core::gbi::Segment::Data { buf: p.rom.file_by_name(OBJECT)?, base: 0 });
    segments[4] = Some(oot_core::gbi::Segment::Data { buf: p.rom.file_by_name("gameplay_keep")?, base: 0 });
    if let Some(code) = oot_core::room::code_ram_image(p) {
        segments[0] = Some(oot_core::gbi::Segment::Data { buf: code, base: 0 });
    }
    let segs = oot_core::room::BufferSegments { segments, pre: Vec::new(), dynamic: 0 };
    Ok(oot_core::room::run_dls(&segs, &[0x0600_0000 | sym.offset]))
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

/// Poses Link's skeleton like `SkelAnime_DrawFlexLod` with
/// `Player_OverrideLimbDrawGameplayCommon`: the head limb's rotation gets the look offsets,
/// and the upper body is pre-rotated (Y, then X, then Z) before its own transform.
pub fn pose_player(skel: &Skeleton, joints: &JointTable, look: &LookRotations, head: usize, upper: usize) -> Vec<Mat4> {
    let r = |a: i16| oot_game::math::binang_to_rad(a);
    let mut out = vec![Mat4::IDENTITY; skel.limbs.len()];
    for &l in &skel.draw_order {
        let l = l as usize;
        let limb = &skel.limbs[l];
        let parent = skel.parents[l].map(|p| out[p as usize]).unwrap_or(Mat4::IDENTITY);
        let pos = if l == 0 {
            let t = joints.rot[0];
            Vec3::new(t[0] as f32, t[1] as f32, t[2] as f32)
        } else {
            Vec3::new(limb.joint_pos[0] as f32, limb.joint_pos[1] as f32, limb.joint_pos[2] as f32)
        };
        let mut rot = joints.rot.get(l + 1).copied().unwrap_or([0; 3]);
        let mut pre = Mat4::IDENTITY;
        if l == head {
            rot = [rot[0].wrapping_add(look.head[0]), rot[1].wrapping_add(look.head[1]), rot[2].wrapping_add(look.head[2])];
        } else if l == upper {
            pre = Mat4::from_rotation_y(r(look.upper_y)) * Mat4::from_rotation_x(r(look.upper_x)) * Mat4::from_rotation_z(r(look.upper_z));
        }
        if l == 0 && look.root_pitch != 0 {
            // Player_OverrideLimbDrawGameplayCommon: the dive pitch about a point 200 above
            // the root, T(pos.x, (cos(unk_6C2) - 1) * 200 + pos.y, pos.z) * RotX(unk_6C2).
            let p = look.root_pitch;
            let t = Vec3::new(pos.x, (oot_game::math::cos_s(p) - 1.0) * 200.0 + pos.y, pos.z);
            out[l] = parent * Mat4::from_translation(t) * Mat4::from_rotation_x(r(p)) * local_transform(Vec3::ZERO, rot);
            continue;
        }
        out[l] = parent * pre * local_transform(pos, rot);
    }
    out
}

/// `Actor_Draw`'s model matrix: translate, rotate by the shape yaw, scale 0.01.
pub fn actor_matrix(pos: Vec3, yaw: i16, scale: f32) -> Mat4 {
    Mat4::from_translation(pos) * Mat4::from_rotation_y(oot_game::math::binang_to_rad(yaw)) * Mat4::from_scale(Vec3::splat(scale))
}

/// Link's two ages with a cache of uploaded draw lists keyed by what changes them.
pub struct LinkGfx {
    pub rules: PlayerRules,
    pub models: [PlayerModel; 2],
    cache: HashMap<(u8, usize, usize, bool, usize), GpuModel>,
}

impl LinkGfx {
    pub fn new(rules: PlayerRules, adult: PlayerModel, child: PlayerModel) -> LinkGfx {
        LinkGfx { rules, models: [adult, child], cache: HashMap::new() }
    }

    pub fn model(&self, age: Age) -> &PlayerModel {
        &self.models[age as usize]
    }

    /// The GPU model for this frame's face (animation face field and blink state), hands and
    /// model group (`PLAYER_MODELGROUP_*` name, e.g. `SWORD` with the sword in hand).
    #[allow(clippy::too_many_arguments)]
    pub fn get(&mut self, renderer: &mut Renderer, device: &wgpu::Device, queue: &wgpu::Queue, age: Age, anim_face: u16, blink_face: usize, fists: bool, model_group: &str) -> &mut GpuModel {
        let (eye, mouth) = self.rules.face_indices(anim_face, blink_face);
        let mut lo = Loadout::default_for(&self.rules, age);
        if let Some(g) = self.rules.model_group(model_group) {
            lo.model_group = g;
        }
        let key = (age as u8, eye, mouth, fists, lo.model_group);
        if !self.cache.contains_key(&key) {
            lo.moving_fast = fists;
            let draw = match self.models[age as usize].draw_list(&self.rules, &lo, eye, mouth, 0) {
                Ok((d, _)) => d,
                Err(e) => {
                    log::error!("Link draw list: {e:#}");
                    DrawList::default()
                }
            };
            self.cache.insert(key, renderer.upload(device, queue, &draw));
        }
        self.cache.get_mut(&key).unwrap()
    }
}
