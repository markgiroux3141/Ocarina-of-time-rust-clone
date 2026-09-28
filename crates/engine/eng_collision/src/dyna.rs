//! Dynamic (actor-owned) collision, ported from `z_bgcheck.c`'s `DynaPoly_*` and
//! `BgCheck_*Dyna*` functions and `code_800430A0.c` (carrying actors that stand on them).
//!
//! Each frame `DynaPoly_UpdateContext` transforms every registered collision header by its
//! actor's scale/rotation/position (`SkinMatrix_SetTranslateRotateYXZScale`) into one shared
//! vertex and poly list, with per-actor floor/wall/ceiling lists and a bounding sphere. The
//! entity checks in `bgcheck` test these after the static mesh. At the end of the frame
//! `DynaPoly_UpdateBgActorTransforms` stores the transform as "previous", which is what
//! `func_800430A0` uses to move an actor standing on the platform by the platform's motion.

use std::sync::Arc;

use eng_math::{cos_s, is_zero, sin_s};
use glam::{Mat4, Vec3, Vec4};

use crate::collision::{CollisionHeader, CollisionPoly, VTX_INDEX_MASK};

/// `BG_ACTOR_MAX`; also `BGCHECK_SCENE`, the bg id of the static mesh.
pub const BG_ACTOR_MAX: u16 = 50;
pub const BGCHECK_SCENE: u16 = BG_ACTOR_MAX;

/// `DynaPolyActor.unk_15C`: `DPM_PLAYER` (1) carries actors standing on it; bit 1 also turns
/// them with it.
pub const DPM_PLAYER: u32 = 1;
pub const DPM_ROTATE: u32 = 2;

/// `ScaleRotPos`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ScaleRotPos {
    pub scale: Vec3,
    pub rot: [i16; 3],
    pub pos: Vec3,
}

/// `SkinMatrix_SetTranslateRotateYXZScale`: T · RotYXZ · S, with `SkinMatrix_SetRotateYXZ`'s
/// entries (Math_SinS / Math_CosS). Columns-major glam matrix of the same row/column values.
pub fn srt_matrix(t: &ScaleRotPos) -> Mat4 {
    let [x, y, z] = t.rot;
    let (sy, cy) = (sin_s(y), cos_s(y));
    // Row-major entries named row·column as in MtxF (xx, xy, xz / yx ... ).
    let (mut xx, mut xy, xz, yx, mut yy, yz, mut zx, mut zy, zz);
    xx = cy;
    zx = -sy;
    if x != 0 {
        let (s, c) = (sin_s(x), cos_s(x));
        zz = cy * c;
        zy = cy * s;
        xz = sy * c;
        xy = sy * s;
        yz = -s;
        yy = c;
    } else {
        zz = cy;
        xz = sy;
        xy = 0.0;
        zy = 0.0;
        yz = 0.0;
        yy = 1.0;
    }
    if z != 0 {
        let (s, c) = (sin_s(z), cos_s(z));
        let (oxx, oxy) = (xx, xy);
        xx = oxx * c + oxy * s;
        xy = oxy * c - oxx * s;
        let (ozy, ozx) = (zy, zx);
        zx = ozx * c + ozy * s;
        zy = ozy * c - ozx * s;
        yx = yy * s;
        yy *= c;
    } else {
        yx = 0.0;
    }
    let r = Mat4::from_cols(Vec4::new(xx, yx, zx, 0.0), Vec4::new(xy, yy, zy, 0.0), Vec4::new(xz, yz, zz, 0.0), Vec4::W);
    Mat4::from_translation(t.pos) * r * Mat4::from_scale(t.scale)
}

/// What `DynaPoly_UpdateContext` reads from the owning actor each frame.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BgActorSource {
    pub pos: Vec3,
    pub shape_rot: [i16; 3],
    pub scale: Vec3,
    pub shape_y_offset: f32,
}

/// `BgActor` (plus the owner's `DynaPolyActor.unk_15C`).
pub struct BgActor {
    pub header: Arc<CollisionHeader>,
    pub source: BgActorSource,
    pub move_flags: u32,
    pub prev: ScaleRotPos,
    pub cur: ScaleRotPos,
    pub collision_disabled: bool,
    pub ceiling_disabled: bool,
    pub poly_start: usize,
    pub vtx_start: usize,
    /// Visit order of the `DynaLookup` lists (each poly is inserted at the head).
    pub floor: Vec<u16>,
    pub wall: Vec<u16>,
    pub ceiling: Vec<u16>,
    /// `Sphere16` bounding sphere.
    pub sphere_center: [i16; 3],
    pub sphere_radius: i16,
    pub min_y: f32,
    pub max_y: f32,
}

/// `DynaCollisionContext`.
#[derive(Default)]
pub struct Dyna {
    pub actors: Vec<BgActor>,
    pub polys: Vec<CollisionPoly>,
    /// `vtxList` (Vec3s), and the same as floats for the geometry tests.
    pub verts_s: Vec<[i16; 3]>,
    pub verts: Vec<Vec3>,
    invalidate: bool,
}

impl BgActor {
    pub fn sphere_center(&self) -> Vec3 {
        Vec3::new(self.sphere_center[0] as f32, self.sphere_center[1] as f32, self.sphere_center[2] as f32)
    }

    /// `Math3D_XZInSphere`.
    pub fn xz_in_sphere(&self, x: f32, z: f32) -> bool {
        let dx = self.sphere_center[0] as f32 - x;
        let dz = self.sphere_center[2] as f32 - z;
        let r = self.sphere_radius as f32;
        dx * dx + dz * dz <= r * r
    }
}

impl Dyna {
    /// `DynaPoly_SetBgActor` + `BgActor_SetActor`: registers `header` for an actor and returns
    /// its bg id. The previous transform's x rotation is offset by one so the first update
    /// always expands it.
    pub fn set_bg_actor(&mut self, header: Arc<CollisionHeader>, source: BgActorSource, move_flags: u32) -> u16 {
        let cur = ScaleRotPos { scale: source.scale, rot: source.shape_rot, pos: source.pos };
        let mut prev = cur;
        prev.rot[0] = prev.rot[0].wrapping_sub(1);
        self.actors.push(BgActor {
            header,
            source,
            move_flags,
            prev,
            cur,
            collision_disabled: false,
            ceiling_disabled: false,
            poly_start: 0,
            vtx_start: 0,
            floor: Vec::new(),
            wall: Vec::new(),
            ceiling: Vec::new(),
            sphere_center: [0; 3],
            sphere_radius: 0,
            min_y: 0.0,
            max_y: 0.0,
        });
        self.invalidate = true;
        (self.actors.len() - 1) as u16
    }

    /// The owning actor's per-frame state (what its `update` changed).
    pub fn set_source(&mut self, bg: u16, source: BgActorSource) {
        self.actors[bg as usize].source = source;
    }

    pub fn is_empty(&self) -> bool {
        self.actors.is_empty()
    }

    /// `DynaPoly_UpdateContext` (after the BG category's actors update).
    pub fn update_context(&mut self) {
        let (mut vtx_start, mut poly_start) = (0usize, 0usize);
        for i in 0..self.actors.len() {
            self.add_to_lookup(i, &mut vtx_start, &mut poly_start);
        }
        self.invalidate = false;
    }

    /// `DynaPoly_UpdateBgActorTransforms` (end of `Actor_UpdateAll`).
    pub fn update_prev_transforms(&mut self) {
        for a in &mut self.actors {
            a.prev = a.cur;
        }
    }

    /// `DynaPoly_AddBgActorToLookup` (`DynaPolyInfo_expandSRT`).
    fn add_to_lookup(&mut self, i: usize, vtx_start: &mut usize, poly_start: &mut usize) {
        let invalidate = self.invalidate;
        let a = &mut self.actors[i];
        a.floor.clear();
        a.wall.clear();
        a.ceiling.clear();
        a.poly_start = *poly_start;
        a.vtx_start = *vtx_start;
        let mut pos = a.source.pos;
        pos.y += a.source.shape_y_offset * a.source.scale.y;
        a.cur = ScaleRotPos { scale: a.source.scale, rot: a.source.shape_rot, pos };
        if a.collision_disabled {
            return;
        }
        let h = a.header.clone();
        let (np, nv) = (h.polys.len(), h.vertices.len());
        if self.polys.len() < *poly_start + np {
            self.polys.resize(*poly_start + np, CollisionPoly::default());
        }
        if self.verts_s.len() < *vtx_start + nv {
            self.verts_s.resize(*vtx_start + nv, [0; 3]);
            self.verts.resize(*vtx_start + nv, Vec3::ZERO);
        }
        let head = |list: &mut Vec<u16>, id: usize| list.insert(0, id as u16);
        if !invalidate && a.prev == a.cur {
            // Unchanged: rebuild the lists from last frame's polys.
            for pi in *poly_start..*poly_start + np {
                let ny = self.polys[pi].normal[1];
                if ny > (0.5f32 * 32767.0) as i16 {
                    head(&mut a.floor, pi);
                } else if ny < (-0.8f32 * 32767.0) as i16 {
                    if !a.ceiling_disabled {
                        head(&mut a.ceiling, pi);
                    }
                } else {
                    head(&mut a.wall, pi);
                }
            }
        } else {
            let m = srt_matrix(&a.cur);
            let inv_n = 1.0 / nv as f32;
            let mut center = Vec3::ZERO;
            for k in 0..nv {
                let v = h.vertices[k];
                let t = mtx_mult_xyz(&m, Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32));
                // BgCheck_Vec3fToVec3s truncates.
                let s = [t.x as i16, t.y as i16, t.z as i16];
                self.verts_s[*vtx_start + k] = s;
                self.verts[*vtx_start + k] = Vec3::new(s[0] as f32, s[1] as f32, s[2] as f32);
                if k == 0 {
                    a.min_y = t.y;
                    a.max_y = t.y;
                } else if t.y < a.min_y {
                    a.min_y = t.y;
                } else if a.max_y < t.y {
                    a.max_y = t.y;
                }
                center += t;
            }
            center *= inv_n;
            a.sphere_center = [center.x as i16, center.y as i16, center.z as i16];
            let mut r2 = -100.0f32;
            for k in 0..nv {
                let d = self.verts[*vtx_start + k].distance_squared(center);
                if r2 < d {
                    r2 = d;
                }
            }
            a.sphere_radius = (r2.sqrt() * 1.1) as i16;
            for k in 0..np {
                let mut p = h.polys[k];
                let vs = *vtx_start as u16;
                p.vtx[0] = ((p.vtx[0] & VTX_INDEX_MASK) + vs) | (p.vtx[0] & 0xE000);
                p.vtx[1] = ((p.vtx[1] & VTX_INDEX_MASK) + vs) | (p.vtx[1] & 0xE000);
                p.vtx[2] = vs.wrapping_add(p.vtx[2]);
                let (va, vb, vc) = (self.verts[p.a()], self.verts[p.b()], self.verts[p.c()]);
                // Math3D_SurfaceNorm.
                let mut n = (vb - va).cross(vc - va);
                let mag = n.length();
                if !is_zero(mag) {
                    n *= 1.0 / mag;
                    p.normal = [(n.x * 32767.0) as i16, (n.y * 32767.0) as i16, (n.z * 32767.0) as i16];
                }
                p.dist = (-n.dot(va)) as i16;
                let id = *poly_start + k;
                self.polys[id] = p;
                if n.y > 0.5 {
                    head(&mut a.floor, id);
                } else if n.y < -0.8 {
                    head(&mut a.ceiling, id);
                } else {
                    head(&mut a.wall, id);
                }
            }
        }
        *poly_start += np;
        *vtx_start += nv;
    }

    /// `func_800433A4`: moves (and with `DPM_ROTATE`, turns) an actor standing on bg actor
    /// `bg` by the platform's motion since last frame. Returns the new position and the yaw
    /// change (`func_800430A0`, `func_800432A0`).
    pub fn carry(&self, bg: u16, pos: Vec3) -> Option<(Vec3, i16)> {
        let a = self.actors.get(bg as usize)?;
        let mut out = pos;
        let mut dyaw = 0i16;
        let mut moved = false;
        if a.move_flags & DPM_PLAYER != 0 {
            // SkinMatrix_Invert fails (returns 2) only for singular matrices.
            let prev = srt_matrix(&a.prev);
            if prev.determinant() != 0.0 {
                let local = mtx_mult_xyz(&prev.inverse(), pos);
                out = mtx_mult_xyz(&srt_matrix(&a.cur), local);
            }
            moved = true;
        }
        if a.move_flags & DPM_ROTATE != 0 {
            dyaw = a.cur.rot[1].wrapping_sub(a.prev.rot[1]);
            moved = true;
        }
        moved.then_some((out, dyaw))
    }
}

/// `SkinMatrix_Vec3fMtxFMultXYZ`.
pub fn mtx_mult_xyz(m: &Mat4, v: Vec3) -> Vec3 {
    let r = |i: usize| m.row(i);
    let f = |row: Vec4| row.w + ((v.x * row.x) + (v.y * row.y) + (v.z * row.z));
    Vec3::new(f(r(0)), f(r(1)), f(r(2)))
}

/// `Math3D_LineVsSph`.
pub fn line_vs_sph(center: Vec3, radius: i16, a: Vec3, b: Vec3) -> bool {
    let r = radius as f32;
    // Math3D_PointInSph: Math3D_DistXYZ16toF(center, point) < radius.
    if center.distance(a) < r || center.distance(b) < r {
        return true;
    }
    let d = b - a;
    let len_sq = d.x * d.x + d.y * d.y + d.z * d.z;
    if is_zero(len_sq) {
        return false;
    }
    let t = ((center.x - a.x) * d.x + (center.y - a.y) * d.y + (center.z - a.z) * d.z) / len_sq;
    if !(0.0..=1.0).contains(&t) {
        return false;
    }
    let p = d * t + a;
    (p - center).length_squared() <= r * r
}
