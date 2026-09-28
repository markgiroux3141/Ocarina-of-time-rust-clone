//! Background collision, ported from `z_bgcheck.c` and the `Math3D_*` helpers in
//! `sys_math3d.c` that it uses: the static scene mesh, plus the actor-owned `DynaPoly`
//! meshes in `dyna` (tested after the static mesh, as `BgCheck_*Impl` do).
//!
//! The game splits a scene into a grid of `StaticLookup` subdivisions, each holding three
//! linked lists (floor / wall / ceiling) sorted by the polys' lowest vertex. This port uses a
//! single subdivision covering the whole mesh, built with the same insertion routine
//! (`StaticLookup_AddPolyToSSList`), so polys are visited in the same relative order. The
//! game's subdivisions overlap by `BGCHECK_SUBDIV_OVERLAP` (50) units, more than any radius
//! the entity checks use, so a single cell sees the same candidate polys.

use glam::Vec3;
use oot_core::collision::{CollisionHeader, CollisionPoly};

pub use crate::dyna::BGCHECK_SCENE;
use crate::dyna::{Dyna, line_vs_sph};
use crate::math::is_zero;

pub const BGCHECK_Y_MIN: f32 = -32000.0;
pub const BGCHECK_SUBDIV_OVERLAP: f32 = 50.0;
/// `COLPOLY_NORMAL_FRAC`.
pub const NORMAL_FRAC: f32 = 1.0 / 32767.0;

// bccFlags
pub const CHECK_WALL: u32 = 1 << 0;
pub const CHECK_FLOOR: u32 = 1 << 1;
pub const CHECK_CEILING: u32 = 1 << 2;
pub const CHECK_ONE_FACE: u32 = 1 << 3;
pub const CHECK_DYNA: u32 = 1 << 4;
pub const CHECK_ALL: u32 = CHECK_WALL | CHECK_FLOOR | CHECK_CEILING | CHECK_ONE_FACE | CHECK_DYNA;

// xpFlags
pub const IGNORE_NONE: u16 = 0;
pub const IGNORE_CAMERA: u16 = 1 << 0;
pub const IGNORE_ENTITY: u16 = 1 << 1;

// downChkFlags
pub const DOWN_CHECK_CEILINGS: u32 = 1 << 0;
pub const DOWN_CHECK_WALLS: u32 = 1 << 1;
pub const DOWN_CHECK_FLOORS: u32 = 1 << 2;
pub const DOWN_CHECK_WALLS_SIMPLE: u32 = 1 << 3;
pub const DOWN_CHECK_GROUND_ONLY: u32 = 1 << 4;

/// `D_80119D90`: wall type → wall flags.
pub const WALL_FLAGS: [u32; 8] = [0, 1, 1 | 2, 1 | 4, 8, 16, 32, 64];
pub const WALL_FLAG_0: u32 = 1;
pub const WALL_FLAG_1: u32 = 2;
pub const WALL_FLAG_3: u32 = 8;
pub const WALL_FLAG_6: u32 = 64;

/// A collision poly and the mesh it belongs to: the game's (`CollisionPoly*`, `bgId`) pair.
/// `bg` is `BGCHECK_SCENE` for the static mesh, else the `DynaPoly` bg actor, and `idx`
/// indexes that mesh's poly list (the shared dyna list for bg actors).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PolyId {
    pub bg: u16,
    pub idx: u16,
}

impl PolyId {
    pub fn scene(idx: u16) -> PolyId {
        PolyId { bg: BGCHECK_SCENE, idx }
    }
    pub fn is_scene(&self) -> bool {
        self.bg == BGCHECK_SCENE
    }
}

pub struct StaticCollision {
    pub header: CollisionHeader,
    floor: Vec<u16>,
    wall: Vec<u16>,
    ceiling: Vec<u16>,
    min: Vec3,
    max: Vec3,
    verts: Vec<Vec3>,
    /// `colCtx->dyna`.
    pub dyna: Dyna,
}

/// `COLPOLY_VIA_FLAG_TEST`.
fn xp_test(poly: &CollisionPoly, flags: u16) -> bool {
    poly.vtx[0] & ((flags & 7) << 13) != 0
}

fn normal(p: &CollisionPoly) -> Vec3 {
    Vec3::new(p.normal[0] as f32, p.normal[1] as f32, p.normal[2] as f32) * NORMAL_FRAC
}

fn tri(p: &CollisionPoly, v: &[Vec3]) -> [Vec3; 3] {
    [v[p.a()], v[p.b()], v[p.c()]]
}

fn vy(p: &CollisionPoly, v: &[Vec3]) -> [f32; 3] {
    [v[p.a()].y, v[p.b()].y, v[p.c()].y]
}

/// `CollisionPoly_CheckYIntersect` (detMax 0) / `..Approx1` (detMax 300).
fn check_y_intersect(p: &CollisionPoly, verts: &[Vec3], x: f32, z: f32, chk_dist: f32, det_max: f32) -> Option<f32> {
    let n = normal(p);
    if is_zero(n.y) {
        return None;
    }
    let [v0, v1, v2] = tri(p, verts);
    if tri_chk_point_para_y(v0, v1, v2, z, x, det_max, chk_dist, n.y) {
        Some(((-n.x * x) - (n.z * z) - p.dist as f32) / n.y)
    } else {
        None
    }
}

/// `CollisionPoly_CheckXIntersectApprox`.
fn check_x_intersect(p: &CollisionPoly, verts: &[Vec3], y: f32, z: f32) -> Option<f32> {
    let n = normal(p);
    if is_zero(n.x) {
        return None;
    }
    let [v0, v1, v2] = tri(p, verts);
    tri_chk_point_para_x(v0, v1, v2, y, z, 300.0, 1.0, n.x).then(|| ((-n.y * y) - (n.z * z) - p.dist as f32) / n.x)
}

/// `CollisionPoly_CheckZIntersectApprox`.
fn check_z_intersect(p: &CollisionPoly, verts: &[Vec3], x: f32, y: f32) -> Option<f32> {
    let n = normal(p);
    if is_zero(n.z) {
        return None;
    }
    let [v0, v1, v2] = tri(p, verts);
    tri_chk_point_para_z(v0, v1, v2, x, y, 300.0, 1.0, n.z).then(|| ((-n.x * x) - (n.y * y) - p.dist as f32) / n.z)
}

/// `CollisionPoly_LineVsPoly`.
fn line_vs_poly(p: &CollisionPoly, verts: &[Vec3], a: Vec3, b: Vec3, one_face: bool, chk_dist: f32) -> Option<Vec3> {
    let n = normal(p);
    let raw = Vec3::new(p.normal[0] as f32, p.normal[1] as f32, p.normal[2] as f32);
    let da = raw.dot(a) * NORMAL_FRAC + p.dist as f32;
    let db = raw.dot(b) * NORMAL_FRAC + p.dist as f32;
    let delta = da - db;
    if (da >= 0.0 && db >= 0.0) || (da < 0.0 && db < 0.0) || (one_face && da < 0.0 && db > 0.0) || is_zero(delta) {
        return None;
    }
    let [v0, v1, v2] = tri(p, verts);
    let hit = a + (b - a) * (da / delta);
    let ok = (n.x.abs() > 0.5 && !is_zero(n.x) && tri_chk_point_para_x(v0, v1, v2, hit.y, hit.z, 0.0, chk_dist, n.x))
        || (n.y.abs() > 0.5 && !is_zero(n.y) && tri_chk_point_para_y(v0, v1, v2, hit.z, hit.x, 0.0, chk_dist, n.y))
        || (n.z.abs() > 0.5 && !is_zero(n.z) && tri_chk_point_para_z(v0, v1, v2, hit.x, hit.y, 0.0, chk_dist, n.z));
    ok.then_some(hit)
}

impl StaticCollision {
    pub fn new(header: CollisionHeader) -> StaticCollision {
        let verts: Vec<Vec3> = (0..header.vertices.len()).map(|i| header.vertex(i)).collect();
        let b = |v: [i16; 3]| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32);
        let mut c = StaticCollision {
            min: b(header.min_bounds),
            max: b(header.max_bounds),
            floor: Vec::new(),
            wall: Vec::new(),
            ceiling: Vec::new(),
            verts,
            header,
            dyna: Dyna::default(),
        };
        // `StaticLookup_AddPoly`: floor if ny > 0.5, ceiling if ny < -0.8, else wall.
        let snormal = |x: f32| (x * 32767.0) as i16;
        for i in 0..c.header.polys.len() {
            let ny = c.header.polys[i].normal[1];
            let list = if ny > snormal(0.5) {
                0
            } else if ny < snormal(-0.8) {
                2
            } else {
                1
            };
            c.add_to_list(list, i as u16);
        }
        c
    }

    /// The poly, from the static mesh or the dyna list.
    pub fn poly(&self, id: PolyId) -> &CollisionPoly {
        if id.is_scene() { &self.header.polys[id.idx as usize] } else { &self.dyna.polys[id.idx as usize] }
    }

    /// `CollisionPoly_GetVerticesByBgId`.
    pub fn poly_vertices(&self, id: PolyId) -> [Vec3; 3] {
        if id.is_scene() { tri(self.poly(id), &self.verts) } else { tri(self.poly(id), &self.dyna.verts) }
    }

    fn spoly(&self, i: u16) -> &CollisionPoly {
        &self.header.polys[i as usize]
    }

    pub fn list_sizes(&self) -> (usize, usize, usize) {
        (self.floor.len(), self.wall.len(), self.ceiling.len())
    }

    /// `CollisionPoly_GetMinY`, including its optimisation for flat polys.
    fn min_y(&self, p: &CollisionPoly) -> i16 {
        let v = &self.header.vertices;
        if p.normal[1] == 32767 || p.normal[1] == -32767 {
            return v[p.a()][1];
        }
        let (a, b, c) = (v[p.a()][1], v[p.b()][1], v[p.c()][1]);
        let min = a.min(b);
        if min < c { min } else { c }
    }

    /// `StaticLookup_AddPolyToSSList`.
    fn add_to_list(&mut self, which: usize, id: u16) {
        let p = *self.spoly(id);
        let y_min = self.min_y(&p) as f32;
        let above = |s: &StaticCollision, other: u16| {
            let [a, b, c] = vy(s.spoly(other), &s.verts);
            y_min < a && y_min < b && y_min < c
        };
        let list = match which {
            0 => &self.floor,
            1 => &self.wall,
            _ => &self.ceiling,
        };
        let pos = if list.is_empty() || above(self, list[0]) {
            0
        } else {
            let mut at = list.len();
            for k in 1..list.len() {
                if above(self, list[k]) {
                    at = k;
                    break;
                }
            }
            at
        };
        match which {
            0 => self.floor.insert(pos, id),
            1 => self.wall.insert(pos, id),
            _ => self.ceiling.insert(pos, id),
        }
    }

    /// `BgCheck_PosInStaticBoundingBox`.
    pub fn in_bounds(&self, p: Vec3) -> bool {
        let o = BGCHECK_SUBDIV_OVERLAP;
        !(p.x < self.min.x - o || self.max.x + o < p.x || p.y < self.min.y - o || self.max.y + o < p.y || p.z < self.min.z - o || self.max.z + o < p.z)
    }

    // ---- surface types (SurfaceType_Get*) ---------------------------------------------

    /// `SurfaceType_GetData`: the surface type list of the poly's own mesh (`bgId`).
    fn surface(&self, id: PolyId, idx: usize) -> u32 {
        let header = if id.is_scene() { &self.header } else { &*self.dyna.actors[id.bg as usize].header };
        header.surface_types.get(self.poly(id).ty as usize).map(|s| s.data[idx]).unwrap_or(0)
    }
    pub fn floor_type(&self, id: PolyId) -> u32 {
        self.surface(id, 0) >> 13 & 0x1F
    }
    pub fn wall_type(&self, id: PolyId) -> u32 {
        self.surface(id, 0) >> 21 & 0x1F
    }
    pub fn wall_flags(&self, id: PolyId) -> u32 {
        WALL_FLAGS.get(self.wall_type(id) as usize).copied().unwrap_or(0)
    }
    pub fn floor_property(&self, id: PolyId) -> u32 {
        self.surface(id, 0) >> 26 & 0xF
    }
    pub fn is_soft(&self, id: PolyId) -> bool {
        self.surface(id, 0) >> 30 & 1 != 0
    }
    pub fn sfx_type(&self, id: PolyId) -> u32 {
        self.surface(id, 1) & 0xF
    }
    pub fn floor_effect(&self, id: PolyId) -> u32 {
        self.surface(id, 1) >> 4 & 3
    }
    pub fn conveyor_speed(&self, id: PolyId) -> u32 {
        self.surface(id, 1) >> 18 & 7
    }
    /// `func_80042108`: surface data[1] bit 27.
    pub fn flag27(&self, id: PolyId) -> bool {
        self.surface(id, 1) & 0x0800_0000 != 0
    }

    // ---- raycast down -----------------------------------------------------------------

    /// `BgCheck_RaycastDownStaticList`.
    #[allow(clippy::too_many_arguments)]
    fn raycast_down_list(&self, list: &[u16], xp: u16, out: &mut Option<PolyId>, pos: Vec3, y_min: f32, chk_dist: f32, ground_chk: bool) -> f32 {
        let mut result = y_min;
        for &id in list {
            let p = self.spoly(id);
            if xp_test(p, xp) || (ground_chk && p.normal[1] < 0) {
                continue;
            }
            let [a, b, c] = vy(p, &self.verts);
            if pos.y < a && pos.y < b && pos.y < c {
                break;
            }
            if let Some(y) = check_y_intersect(p, &self.verts, pos.x, pos.z, chk_dist, 0.0) {
                if y < pos.y && result < y {
                    result = y;
                    *out = Some(PolyId::scene(id));
                }
            }
        }
        result
    }

    /// `BgCheck_RaycastDownDynaList`.
    #[allow(clippy::too_many_arguments)]
    fn raycast_down_dyna_list(&self, bg: u16, list: &[u16], walls_or_ceilings: bool, xp: u16, down_flags: u32, out: &mut Option<PolyId>, pos: Vec3, y_start: f32, chk_dist: f32) -> f32 {
        let d = &self.dyna;
        let mut result = y_start;
        for &id in list {
            let p = &d.polys[id as usize];
            if xp_test(p, xp) {
                continue;
            }
            if walls_or_ceilings && down_flags & DOWN_CHECK_GROUND_ONLY != 0 && (p.normal[1] as f32 * NORMAL_FRAC) < 0.0 {
                continue;
            }
            if let Some(y) = check_y_intersect(p, &d.verts, pos.x, pos.z, chk_dist, 300.0) {
                if y < pos.y && result < y {
                    result = y;
                    *out = Some(PolyId { bg, idx: id });
                }
            }
        }
        result
    }

    /// `BgCheck_RaycastDownDyna`: returns the best dyna floor above `y_static`, or
    /// `BGCHECK_Y_MIN`. (Its re-check for bg actors being deleted, `BGACTOR_1`, never runs
    /// here.)
    fn raycast_down_dyna(&self, pos: Vec3, xp: u16, down_flags: u32, chk_dist: f32, y_static: f32, out: &mut Option<PolyId>) -> f32 {
        let mut result = BGCHECK_Y_MIN;
        let mut best = y_static;
        for (i, a) in self.dyna.actors.iter().enumerate() {
            if a.collision_disabled || pos.y < a.min_y || !a.xz_in_sphere(pos.x, pos.z) {
                continue;
            }
            let bg = i as u16;
            if down_flags & DOWN_CHECK_FLOORS != 0 {
                let y = self.raycast_down_dyna_list(bg, &a.floor, false, xp, down_flags, out, pos, best, chk_dist);
                if best < y {
                    best = y;
                    result = y;
                }
            }
            if down_flags & DOWN_CHECK_WALLS != 0 || (out.is_none() && down_flags & DOWN_CHECK_WALLS_SIMPLE != 0) {
                let y = self.raycast_down_dyna_list(bg, &a.wall, true, xp, down_flags, out, pos, best, chk_dist);
                if best < y {
                    best = y;
                    result = y;
                }
            }
            if down_flags & DOWN_CHECK_CEILINGS != 0 {
                let y = self.raycast_down_dyna_list(bg, &a.ceiling, true, xp, down_flags, out, pos, best, chk_dist);
                if best < y {
                    best = y;
                    result = y;
                }
            }
        }
        result
    }

    /// `BgCheck_RaycastDownImpl`. Returns the floor height under `pos` and the poly, or
    /// `BGCHECK_Y_MIN`.
    pub fn raycast_down(&self, pos: Vec3, xp: u16, down_flags: u32, chk_dist: f32) -> (f32, Option<PolyId>) {
        let mut out = None;
        let mut y = BGCHECK_Y_MIN;
        let o = BGCHECK_SUBDIV_OVERLAP;
        // With one subdivision, the downward walk through the grid either lands in it or not.
        let inside_xz = !(pos.x < self.min.x - o || self.max.x + o < pos.x || pos.z < self.min.z - o || self.max.z + o < pos.z);
        if inside_xz && pos.y >= self.min.y - o {
            if down_flags & DOWN_CHECK_FLOORS != 0 {
                y = self.raycast_down_list(&self.floor, xp, &mut out, pos, y, chk_dist, false);
            }
            let ground = down_flags & DOWN_CHECK_GROUND_ONLY != 0;
            if down_flags & (DOWN_CHECK_WALLS | DOWN_CHECK_WALLS_SIMPLE) != 0 {
                y = self.raycast_down_list(&self.wall, xp, &mut out, pos, y, chk_dist, ground);
            }
            if down_flags & DOWN_CHECK_CEILINGS != 0 {
                y = self.raycast_down_list(&self.ceiling, xp, &mut out, pos, y, chk_dist, ground);
            }
        }
        if !self.dyna.is_empty() {
            let yd = self.raycast_down_dyna(pos, xp, down_flags, chk_dist, y, &mut out);
            if y < yd {
                y = yd;
            }
        }
        if y != BGCHECK_Y_MIN && out.is_some_and(|p| self.is_soft(p)) {
            y -= 1.0;
        }
        (y, out)
    }

    /// `BgCheck_EntityRaycastDown1/3/5` (entity ground check, chkDist 1).
    pub fn entity_raycast_down(&self, pos: Vec3) -> (f32, Option<PolyId>) {
        self.raycast_down(pos, IGNORE_ENTITY, DOWN_CHECK_WALLS_SIMPLE | DOWN_CHECK_FLOORS | DOWN_CHECK_GROUND_ONLY, 1.0)
    }

    // ---- walls ------------------------------------------------------------------------

    /// `BgCheck_ComputeWallDisplacement`. @bug (game): the previous wall's flag 27 is read
    /// from the scene's surface types even when it's a dyna poly.
    #[allow(clippy::too_many_arguments)]
    fn wall_displacement(&self, id: PolyId, x: &mut f32, z: &mut f32, n: Vec3, inv_xz: f32, plane_dist: f32, radius: f32, wall: &mut Option<PolyId>) -> bool {
        let d = (radius - plane_dist) * inv_xz;
        *x += d * n.x;
        *z += d * n.z;
        match *wall {
            None => {
                *wall = Some(id);
                true
            }
            Some(w) => {
                let ty = self.poly(w).ty as usize;
                let data1 = self.header.surface_types.get(ty).map(|s| s.data[1]).unwrap_or(0);
                if data1 & 0x0800_0000 == 0 {
                    *wall = Some(id);
                    true
                } else {
                    false
                }
            }
        }
    }

    /// One pass of `BgCheck_SphVsStaticWall` / `BgCheck_SphVsDynaWallInBgActor` over `list`:
    /// pass 0 tests walls facing mostly ±z, pass 1 walls facing mostly ±x. `early_out` is the
    /// static list's "all remaining polys are above the sphere" break.
    #[allow(clippy::too_many_arguments)]
    fn sph_wall_pass(&self, pass: u32, bg: u16, list: &[u16], verts: &[Vec3], xp: u16, rp: &mut Vec3, center_y: f32, radius: f32, wall: &mut Option<PolyId>, out_bg: Option<&mut u16>, early_out: bool) -> bool {
        let mut result = false;
        let mut out_bg = out_bg;
        for &idx in list {
            let id = PolyId { bg, idx };
            let p = self.poly(id);
            if early_out {
                let [ya, yb, yc] = vy(p, verts);
                if center_y < ya && center_y < yb && center_y < yc {
                    break;
                }
            }
            let n = normal(p);
            let nxz = (n.x * n.x + n.z * n.z).sqrt();
            let plane_dist = dist_plane_to_pos(n, p.dist as f32, *rp);
            if radius < plane_dist.abs() || xp_test(p, xp) {
                continue;
            }
            let inv = 1.0 / nxz;
            let [v0, v1, v2] = tri(p, verts);
            let hit = if pass == 0 {
                let t = n.z.abs() * inv;
                if t < 0.4 {
                    continue;
                }
                let (mut lo, mut hi) = (v0.z, v0.z);
                if v1.z < lo { lo = v1.z } else if hi < v1.z { hi = v1.z }
                if v2.z < lo { lo = v2.z } else if v2.z > hi { hi = v2.z }
                if rp.z < lo - radius || rp.z > hi + radius {
                    continue;
                }
                check_z_intersect(p, verts, rp.x, center_y).is_some_and(|i| (i - rp.z).abs() <= radius / t && (i - rp.z) * n.z <= 4.0)
            } else {
                let t = n.x.abs() * inv;
                if t < 0.4 {
                    continue;
                }
                let (mut lo, mut hi) = (v0.x, v0.x);
                if v1.x < lo { lo = v1.x } else if hi < v1.x { hi = v1.x }
                if v2.x < lo { lo = v2.x } else if hi < v2.x { hi = v2.x }
                if rp.x < lo - radius || hi + radius < rp.x {
                    continue;
                }
                check_x_intersect(p, verts, center_y, rp.z).is_some_and(|i| (i - rp.x).abs() <= radius / t && (i - rp.x) * n.x <= 4.0)
            };
            if hit {
                let (mut x, mut z) = (rp.x, rp.z);
                if self.wall_displacement(id, &mut x, &mut z, n, inv, plane_dist, radius, wall)
                    && let Some(b) = out_bg.as_deref_mut()
                {
                    *b = bg;
                }
                rp.x = x;
                rp.z = z;
                result = true;
            }
        }
        result
    }

    /// `BgCheck_SphVsStaticWall`: pushes (`x`, `z`) out of every wall within `radius` of
    /// `center`. Two passes: walls facing mostly ±z, then walls facing mostly ±x.
    pub fn sph_vs_static_wall(&self, xp: u16, x: &mut f32, z: &mut f32, center: Vec3, radius: f32, wall: &mut Option<PolyId>) -> bool {
        let mut rp = center;
        let mut result = false;
        for pass in 0..2 {
            result |= self.sph_wall_pass(pass, BGCHECK_SCENE, &self.wall, &self.verts, xp, &mut rp, center.y, radius, wall, None, true);
        }
        *x = rp.x;
        *z = rp.z;
        result
    }

    /// `BgCheck_SphVsDynaWall`: the same against every bg actor whose bounding sphere (grown
    /// by the radius) reaches the sphere.
    #[allow(clippy::too_many_arguments)]
    pub fn sph_vs_dyna_wall(&self, xp: u16, x: &mut f32, z: &mut f32, center: Vec3, radius: f32, wall: &mut Option<PolyId>, out_bg: &mut u16) -> bool {
        let mut result = false;
        let mut rp = center;
        for (i, a) in self.dyna.actors.iter().enumerate() {
            if a.collision_disabled || a.min_y > rp.y || a.max_y < rp.y {
                continue;
            }
            let r = a.sphere_radius.wrapping_add(radius as i16) as f32;
            let c = a.sphere_center();
            let (dx, dz) = (c.x - rp.x, c.z - rp.z);
            let xy_in = (c.x - rp.x).powi(2) + (c.y - rp.y).powi(2) <= r * r;
            let yz_in = (c.y - rp.y).powi(2) + (c.z - rp.z).powi(2) <= r * r;
            if r * r < dx * dx + dz * dz || (!xy_in && !yz_in) {
                continue;
            }
            // BgCheck_SphVsDynaWallInBgActor: both passes from the same starting point.
            let mut local = rp;
            let mut hit = false;
            for pass in 0..2 {
                hit |= self.sph_wall_pass(pass, i as u16, &a.wall, &self.dyna.verts, xp, &mut local, center.y, radius, wall, Some(out_bg), false);
            }
            if hit {
                rp.x = local.x;
                rp.z = local.z;
                result = true;
            }
        }
        *x = rp.x;
        *z = rp.z;
        result
    }

    /// `BgCheck_CheckWallImpl`, used by `BgCheck_EntitySphVsWall3/4`.
    /// Returns (hit, resolved position, wall poly).
    pub fn check_wall(&self, xp: u16, pos_next: Vec3, pos_prev: Vec3, radius: f32, check_height: f32, arg_a: u8) -> (bool, Vec3, Option<PolyId>) {
        let mut result = false;
        let mut out_poly = None;
        let mut out_bg = BGCHECK_SCENE;
        let mut res = pos_next;
        let d = pos_next - pos_prev;
        if (d.x != 0.0 || d.z != 0.0) && (arg_a & 1) == 0 {
            if check_height + d.y < 5.0 {
                // @bug (game): checkHeight is not applied to posPrev/posNext here.
                if let Some((hit, poly)) = self.check_line(xp, IGNORE_NONE, pos_prev, pos_next, 1.0, CHECK_ALL & !CHECK_CEILING) {
                    let n = normal(self.poly(poly));
                    if n.y > 0.5 {
                        res.x = hit.x;
                        res.y = if check_height > 1.0 { hit.y - 1.0 } else { hit.y - check_height };
                        res.z = hit.z;
                    } else {
                        res = Vec3::new(radius * n.x + hit.x, radius * n.y + hit.y, radius * n.z + hit.z);
                    }
                    out_poly = Some(poly);
                    out_bg = poly.bg;
                    result = true;
                }
            } else {
                let bcc = if radius * radius < d.x * d.x + d.z * d.z {
                    CHECK_ALL & !CHECK_CEILING
                } else {
                    CHECK_ALL & !CHECK_FLOOR & !CHECK_CEILING
                };
                let next = Vec3::new(pos_next.x, pos_next.y + check_height, pos_next.z);
                let prev = Vec3::new(pos_prev.x, next.y, pos_prev.z);
                if let Some((hit, poly)) = self.check_line(xp, IGNORE_NONE, prev, next, 1.0, bcc) {
                    let n = normal(self.poly(poly));
                    let nxz = (n.x * n.x + n.z * n.z).sqrt();
                    if !is_zero(nxz) {
                        let k = radius * (1.0 / nxz);
                        res.x = k * n.x + hit.x;
                        res.z = k * n.z + hit.z;
                        out_poly = Some(poly);
                        out_bg = poly.bg;
                        result = true;
                    }
                }
            }
        }
        let mut center = Vec3::new(res.x, res.y + check_height, res.z);
        let mut dyna_hit = false;
        if !self.dyna.is_empty() {
            let (mut x, mut z) = (res.x, res.z);
            if self.sph_vs_dyna_wall(xp, &mut x, &mut z, center, radius, &mut out_poly, &mut out_bg) {
                res.x = x;
                res.z = z;
                result = true;
                dyna_hit = true;
                center = Vec3::new(res.x, res.y + check_height, res.z);
            }
        }
        if self.in_bounds(pos_next) {
            let (mut x, mut z) = (res.x, res.z);
            if self.sph_vs_static_wall(xp, &mut x, &mut z, center, radius, &mut out_poly) {
                out_bg = BGCHECK_SCENE;
                result = true;
            }
            res.x = x;
            res.z = z;
        }
        // After a dyna wall: make sure the push didn't go through a static wall (no dyna).
        if dyna_hit || out_bg != BGCHECK_SCENE {
            if let Some((hit, poly)) = self.check_line(xp, IGNORE_NONE, pos_prev, res, 1.0, CHECK_ONE_FACE | CHECK_WALL) {
                let n = normal(self.poly(poly));
                let nxz = (n.x * n.x + n.z * n.z).sqrt();
                if !is_zero(nxz) {
                    let k = radius * (1.0 / nxz);
                    res.x = k * n.x + hit.x;
                    res.z = k * n.z + hit.z;
                    out_poly = Some(poly);
                    result = true;
                }
            }
        }
        (result, res, out_poly)
    }

    // ---- ceilings ---------------------------------------------------------------------

    /// `BgCheck_CheckCeilingImpl`: returns the y `pos` must be lowered to so that
    /// `check_height` above it clears the ceiling.
    pub fn check_ceiling(&self, xp: u16, pos: Vec3, check_height: f32) -> Option<(f32, PolyId)> {
        if !self.in_bounds(pos) {
            return None;
        }
        // BgCheck_CheckStaticCeiling.
        let mut out_y = pos.y;
        let mut found = None;
        for &idx in &self.ceiling {
            let p = self.spoly(idx);
            if xp_test(p, xp) {
                continue;
            }
            if let Some(cy) = check_y_intersect(p, &self.verts, pos.x, pos.z, 1.0, 300.0) {
                let d = cy - out_y;
                let ny = normal(p).y;
                if d > 0.0 && d < check_height && d * ny <= 0.0 {
                    out_y = cy - check_height;
                    found = Some(PolyId::scene(idx));
                }
            }
        }
        // BgCheck_CheckDynaCeiling from the static result.
        let test = Vec3::new(pos.x, out_y, pos.z);
        let mut result_y = check_height + test.y;
        let mut dyna_found = None;
        for (i, a) in self.dyna.actors.iter().enumerate() {
            if a.collision_disabled || !a.xz_in_sphere(test.x, test.z) {
                continue;
            }
            if let Some((y, id)) = self.dyna_ceiling_list(i as u16, &a.ceiling, xp, test, check_height) {
                if y < result_y {
                    result_y = y;
                    dyna_found = Some(id);
                }
            }
        }
        if let Some(id) = dyna_found {
            return Some((result_y, id));
        }
        found.map(|id| (out_y, id))
    }

    /// `BgCheck_CheckDynaCeilingList`.
    fn dyna_ceiling_list(&self, bg: u16, list: &[u16], xp: u16, pos: Vec3, check_height: f32) -> Option<(f32, PolyId)> {
        let d = &self.dyna;
        let mut t = pos;
        let mut found = None;
        for &idx in list {
            let p = &d.polys[idx as usize];
            if xp_test(p, xp) {
                continue;
            }
            let n = normal(p);
            if check_height < udist_plane_to_pos(n, p.dist as f32, t) {
                continue;
            }
            if let Some(cy) = check_y_intersect(p, &d.verts, t.x, t.z, 1.0, 300.0) {
                let dist = cy - t.y;
                if t.y < cy && dist < check_height && dist * n.y <= 0.0 {
                    let sign = if 0.0 <= n.y { 1.0 } else { -1.0 };
                    t.y = sign * check_height + cy;
                    found = Some(PolyId { bg, idx });
                }
            }
        }
        found.map(|id| (t.y, id))
    }

    // ---- line tests -------------------------------------------------------------------

    /// `BgCheck_CheckLineAgainstSSList`.
    #[allow(clippy::too_many_arguments)]
    fn line_list(&self, list: &[u16], xp1: u16, xp2: u16, a: Vec3, b: &mut Vec3, out: &mut Option<(Vec3, PolyId)>, dist_sq: &mut f32, chk_dist: f32, bcc: u32) -> bool {
        let mut result = false;
        for &idx in list {
            let p = self.spoly(idx);
            if xp_test(p, xp1) || !(xp2 == 0 || xp_test(p, xp2)) {
                continue;
            }
            let min_y = self.min_y(p) as f32;
            if a.y < min_y && b.y < min_y {
                break;
            }
            if let Some(hit) = line_vs_poly(p, &self.verts, a, *b, bcc & CHECK_ONE_FACE != 0, chk_dist) {
                let d = (hit - a).length_squared();
                if d < *dist_sq {
                    *dist_sq = d;
                    *b = hit;
                    *out = Some((hit, PolyId::scene(idx)));
                    result = true;
                }
            }
        }
        result
    }

    /// `BgCheck_CheckLineAgainstDyna` (with `BgCheck_CheckLineAgainstBgActor`: walls, floors,
    /// then ceilings of each bg actor the segment's bounding sphere test passes).
    #[allow(clippy::too_many_arguments)]
    fn line_dyna(&self, xp: u16, a: Vec3, b: &mut Vec3, out: &mut Option<(Vec3, PolyId)>, dist_sq: &mut f32, chk_dist: f32, bcc: u32) -> bool {
        let d = &self.dyna;
        let mut result = false;
        for (i, act) in d.actors.iter().enumerate() {
            if act.collision_disabled {
                continue;
            }
            let (ay, by) = (a.y, b.y);
            if (ay < act.min_y && by < act.min_y) || (act.max_y < ay && act.max_y < by) {
                continue;
            }
            if !line_vs_sph(act.sphere_center(), act.sphere_radius, a, *b) {
                continue;
            }
            for (flag, list) in [(CHECK_WALL, &act.wall), (CHECK_FLOOR, &act.floor), (CHECK_CEILING, &act.ceiling)] {
                if bcc & flag == 0 {
                    continue;
                }
                for &idx in list.iter() {
                    let p = &d.polys[idx as usize];
                    if xp_test(p, xp) {
                        continue;
                    }
                    if let Some(hit) = line_vs_poly(p, &d.verts, a, *b, bcc & CHECK_ONE_FACE != 0, chk_dist) {
                        let ds = (hit - a).length_squared();
                        if ds < *dist_sq {
                            *dist_sq = ds;
                            *b = hit;
                            *out = Some((hit, PolyId { bg: i as u16, idx }));
                            result = true;
                        }
                    }
                }
            }
        }
        result
    }

    /// `BgCheck_CheckLineImpl`: the closest poly the segment `a`→`b` crosses (bg actors too
    /// with `CHECK_DYNA`).
    pub fn check_line(&self, xp1: u16, xp2: u16, a: Vec3, b: Vec3, chk_dist: f32, bcc: u32) -> Option<(Vec3, PolyId)> {
        if !self.in_bounds(a) {
            return None;
        }
        let mut bt = b;
        let mut dist_sq = 1.0e38f32;
        let mut out = None;
        if bcc & CHECK_FLOOR != 0 {
            self.line_list(&self.floor, xp1, xp2, a, &mut bt, &mut out, &mut dist_sq, chk_dist, bcc);
        }
        if bcc & CHECK_WALL != 0 {
            self.line_list(&self.wall, xp1, xp2, a, &mut bt, &mut out, &mut dist_sq, chk_dist, bcc);
        }
        if bcc & CHECK_CEILING != 0 {
            self.line_list(&self.ceiling, xp1, xp2, a, &mut bt, &mut out, &mut dist_sq, chk_dist, bcc);
        }
        if let Some((hit, _)) = out {
            dist_sq = (hit - a).length_squared();
        }
        if bcc & CHECK_DYNA != 0 && !self.dyna.is_empty() {
            self.line_dyna(xp1, a, &mut bt, &mut out, &mut dist_sq, chk_dist, bcc);
        }
        out
    }

    /// `BgCheck_EntityLineTest1`.
    #[allow(clippy::too_many_arguments)]
    pub fn entity_line_test(&self, a: Vec3, b: Vec3, wall: bool, floor: bool, ceil: bool, one_face: bool) -> Option<(Vec3, PolyId)> {
        let mut bcc = CHECK_DYNA;
        if wall {
            bcc |= CHECK_WALL;
        }
        if floor {
            bcc |= CHECK_FLOOR;
        }
        if ceil {
            bcc |= CHECK_CEILING;
        }
        if one_face {
            bcc |= CHECK_ONE_FACE;
        }
        self.check_line(IGNORE_ENTITY, IGNORE_NONE, a, b, 1.0, bcc)
    }

    /// `WaterBox_GetSurfaceImpl`: the surface height of the first water box (for `room`, or one
    /// marked for all rooms, `WATERBOX_ROOM_ALL`) whose x/z extent contains the point.
    pub fn water_surface(&self, x: f32, z: f32, room: u32) -> Option<f32> {
        for w in &self.header.water_boxes {
            let r = (w.properties >> 13) & 0x3F;
            if (r == room || r == 0x3F) && w.properties & (1 << 19) == 0 {
                let (x0, z0) = (w.x_min as f32, w.z_min as f32);
                if x0 < x && x < x0 + w.x_length as f32 && z0 < z && z < z0 + w.z_length as f32 {
                    return Some(w.y_surface as f32);
                }
            }
        }
        None
    }

    pub fn poly_normal(&self, id: PolyId) -> Vec3 {
        normal(self.poly(id))
    }
}

/// `Math3D_DistPlaneToPos`.
pub fn dist_plane_to_pos(n: Vec3, origin_dist: f32, p: Vec3) -> f32 {
    let m = n.length();
    if is_zero(m) {
        return 0.0;
    }
    (n.dot(p) + origin_dist) / m
}

/// `Math3D_UDistPlaneToPos`.
pub fn udist_plane_to_pos(n: Vec3, origin_dist: f32, p: Vec3) -> f32 {
    if is_zero(n.length()) {
        return 0.0;
    }
    dist_plane_to_pos(n, origin_dist, p).abs()
}

/// `Math3D_CirSquareVsTriSquare`.
#[allow(clippy::too_many_arguments)]
fn cir_square_vs_tri_square(x0: f32, y0: f32, x1: f32, y1: f32, x2: f32, y2: f32, cx: f32, cy: f32, r: f32) -> bool {
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (x0, x0, y0, y0);
    if x1 < min_x { min_x = x1 } else if max_x < x1 { max_x = x1 }
    if y1 < min_y { min_y = y1 } else if max_y < y1 { max_y = y1 }
    if x2 < min_x { min_x = x2 } else if max_x < x2 { max_x = x2 }
    if y2 < min_y { min_y = y2 } else if max_y < y2 { max_y = y2 }
    (min_x - r) <= cx && (max_x + r) >= cx && (min_y - r) <= cy && (max_y + r) >= cy
}

/// `Math3D_PointDistSqToLine2D`: returns (perpendicular foot inside the segment, distance²).
fn point_dist_sq_to_line_2d(x0: f32, y0: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> (bool, f32) {
    let (xd, yd) = (x2 - x1, y2 - y1);
    let dsq = xd * xd + yd * yd;
    if is_zero(dsq) {
        return (false, 0.0);
    }
    let r = ((x0 - x1) * xd + (y0 - y1) * yd) / dsq;
    let (px, py) = (xd * r + x1, yd * r + y1);
    ((0.0..=1.0).contains(&r), (px - x0) * (px - x0) + (py - y0) * (py - y0))
}

/// The shared body of `Math3D_TriChkPointParaYImpl` / `XImpl` / `ZImpl` on 2D coordinates
/// (`u`, `v`) of the triangle and point.
#[allow(clippy::too_many_arguments)]
fn tri_chk_point_2d(t: [(f32, f32); 3], u: f32, v: f32, det_max: f32, chk_dist: f32, n_axis: f32) -> bool {
    let [(u0, v0), (u1, v1), (u2, v2)] = t;
    if !cir_square_vs_tri_square(u0, v0, u1, v1, u2, v2, u, v, chk_dist) {
        return false;
    }
    let c = chk_dist * chk_dist;
    let sq = |a: f32, b: f32| (a - u) * (a - u) + (b - v) * (b - v);
    if sq(u0, v0) < c || sq(u1, v1) < c || sq(u2, v2) < c {
        return true;
    }
    let d01 = (u0 - u) * (v1 - v) - (v0 - v) * (u1 - u);
    let d12 = (u1 - u) * (v2 - v) - (v1 - v) * (u2 - u);
    let d20 = (u2 - u) * (v0 - v) - (v2 - v) * (u0 - u);
    if (det_max >= d01 && det_max >= d12 && det_max >= d20) || (-det_max <= d01 && -det_max <= d12 && -det_max <= d20) {
        return true;
    }
    if n_axis.abs() > 0.5 {
        for ((a, b), (e, f)) in [((u0, v0), (u1, v1)), ((u1, v1), (u2, v2)), ((u2, v2), (u0, v0))] {
            let (inside, d) = point_dist_sq_to_line_2d(u, v, a, b, e, f);
            if inside && d < c {
                return true;
            }
        }
    }
    false
}

/// `Math3D_TriChkPointParaYImpl(v0, v1, v2, z, x, ..)`: the triangle projected on (z, x).
#[allow(clippy::too_many_arguments)]
pub fn tri_chk_point_para_y(v0: Vec3, v1: Vec3, v2: Vec3, z: f32, x: f32, det_max: f32, chk_dist: f32, ny: f32) -> bool {
    tri_chk_point_2d([(v0.z, v0.x), (v1.z, v1.x), (v2.z, v2.x)], z, x, det_max, chk_dist, ny)
}

/// `Math3D_TriChkPointParaXImpl(v0, v1, v2, y, z, ..)`: projected on (y, z).
#[allow(clippy::too_many_arguments)]
pub fn tri_chk_point_para_x(v0: Vec3, v1: Vec3, v2: Vec3, y: f32, z: f32, det_max: f32, chk_dist: f32, nx: f32) -> bool {
    tri_chk_point_2d([(v0.y, v0.z), (v1.y, v1.z), (v2.y, v2.z)], y, z, det_max, chk_dist, nx)
}

/// `Math3D_TriChkPointParaZImpl(v0, v1, v2, x, y, ..)`: projected on (x, y).
#[allow(clippy::too_many_arguments)]
pub fn tri_chk_point_para_z(v0: Vec3, v1: Vec3, v2: Vec3, x: f32, y: f32, det_max: f32, chk_dist: f32, nz: f32) -> bool {
    tri_chk_point_2d([(v0.x, v0.y), (v1.x, v1.y), (v2.x, v2.y)], x, y, det_max, chk_dist, nz)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oot_core::collision::CollisionBuilder;

    fn box_room() -> StaticCollision {
        let mut b = CollisionBuilder::new();
        let s = b.surface(0, 0);
        // Floor at y=0, 400x400, and one wall at z=-200 facing +z.
        b.quad(Vec3::new(-200.0, 0.0, 200.0), Vec3::new(200.0, 0.0, 200.0), Vec3::new(200.0, 0.0, -200.0), Vec3::new(-200.0, 0.0, -200.0), s);
        b.quad(Vec3::new(-200.0, 0.0, -200.0), Vec3::new(200.0, 0.0, -200.0), Vec3::new(200.0, 200.0, -200.0), Vec3::new(-200.0, 200.0, -200.0), s);
        // A raised block top at y=40 over x in [100,200].
        b.quad(Vec3::new(100.0, 40.0, 200.0), Vec3::new(200.0, 40.0, 200.0), Vec3::new(200.0, 40.0, 100.0), Vec3::new(100.0, 40.0, 100.0), s);
        StaticCollision::new(b.finish())
    }

    #[test]
    fn raycast_down_finds_highest_floor_below() {
        let c = box_room();
        assert_eq!(c.list_sizes(), (4, 2, 0));
        assert_eq!(c.entity_raycast_down(Vec3::new(0.0, 50.0, 0.0)).0, 0.0);
        assert_eq!(c.entity_raycast_down(Vec3::new(150.0, 90.0, 150.0)).0, 40.0);
        assert_eq!(c.entity_raycast_down(Vec3::new(150.0, 30.0, 150.0)).0, 0.0);
        assert_eq!(c.entity_raycast_down(Vec3::new(900.0, 30.0, 0.0)).0, BGCHECK_Y_MIN);
    }

    #[test]
    fn wall_pushes_sphere_out() {
        let c = box_room();
        let (hit, p, poly) = c.check_wall(IGNORE_ENTITY, Vec3::new(0.0, 0.0, -190.0), Vec3::new(0.0, 0.0, -170.0), 18.0, 26.0, 0);
        assert!(hit);
        assert!(poly.is_some());
        assert!((p.z - (-182.0)).abs() < 0.01, "{p}");
    }
}
