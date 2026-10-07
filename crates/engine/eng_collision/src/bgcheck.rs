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

use eng_math::is_zero;
use glam::Vec3;

pub use crate::dyna::BGCHECK_SCENE;
use crate::collision::{CollisionHeader, CollisionPoly};
use crate::dyna::{Dyna, line_vs_sph};

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
/// `COLPOLY_IGNORE_PROJECTILES` (`bgcheck.h`).
pub const IGNORE_PROJECTILES: u16 = 1 << 2;

// downChkFlags
pub const DOWN_CHECK_CEILINGS: u32 = 1 << 0;
pub const DOWN_CHECK_WALLS: u32 = 1 << 1;
pub const DOWN_CHECK_FLOORS: u32 = 1 << 2;
pub const DOWN_CHECK_WALLS_SIMPLE: u32 = 1 << 3;
pub const DOWN_CHECK_GROUND_ONLY: u32 = 1 << 4;

// bciFlags (`z_bgcheck.c`'s `BGCHECK_IGNORE_*`)
pub const BGCHECK_IGNORE_NONE: u16 = 0;
pub const BGCHECK_IGNORE_CEILING: u16 = 1 << 0;
pub const BGCHECK_IGNORE_WALL: u16 = 1 << 1;
pub const BGCHECK_IGNORE_FLOOR: u16 = 1 << 2;


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

pub struct CollisionContext {
    pub header: CollisionHeader,
    floor: Vec<u16>,
    wall: Vec<u16>,
    ceiling: Vec<u16>,
    min: Vec3,
    max: Vec3,
    verts: Vec<Vec3>,
    /// `colCtx->dyna`.
    pub dyna: Dyna,
    /// The room water boxes are filtered by (`BgCheck_GetWaterSurface` compares with the
    /// current room); set by whoever owns the rooms.
    pub water_room: u32,
}

/// `COLPOLY_VTX_CHECK_FLAGS_ANY`.
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

impl CollisionContext {
    pub fn new(header: CollisionHeader) -> CollisionContext {
        let verts: Vec<Vec3> = (0..header.vertices.len()).map(|i| header.vertex(i)).collect();
        let b = |v: [i16; 3]| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32);
        let mut c = CollisionContext {
            min: b(header.min_bounds),
            max: b(header.max_bounds),
            floor: Vec::new(),
            wall: Vec::new(),
            ceiling: Vec::new(),
            verts,
            header,
            dyna: Dyna::default(),
            water_room: 0,
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
        let above = |s: &CollisionContext, other: u16| {
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

    // ---- surface types ----------------------------------------------------------------

    /// `SurfaceType_GetData`: word `idx` (0 or 1) of the poly's surface type, from the surface
    /// type list of the poly's own mesh (`bgId`). What the bits mean is the game's business
    /// (`oot_game::surface`); the collision code itself reads only the two bits below.
    pub fn surface_word(&self, id: PolyId, idx: usize) -> u32 {
        let header = if id.is_scene() { &self.header } else { &*self.dyna.actors[id.bg as usize].header };
        header.surface_types.get(self.poly(id).ty as usize).map(|s| s.data[idx]).unwrap_or(0)
    }
    /// `SurfaceType_IsSoft`: word 0 bit 30. `BgCheck_RaycastDownImpl` lowers soft floors by 1.
    fn surface_is_soft(&self, id: PolyId) -> bool {
        self.surface_word(id, 0) >> 30 & 1 != 0
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
    fn raycast_down_dyna(&self, pos: Vec3, xp: u16, down_flags: u32, chk_dist: f32, y_static: f32, out: &mut Option<PolyId>, skip: Option<u16>) -> f32 {
        let mut result = BGCHECK_Y_MIN;
        let mut best = y_static;
        for (i, a) in self.dyna.actors.iter().enumerate() {
            // The asking actor's own bg actor (dynaRaycastDown->actor == bgActors[i].actor).
            if !a.in_use() || a.collision_disabled || pos.y < a.min_y || !a.xz_in_sphere(pos.x, pos.z) || skip == Some(i as u16) {
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
        self.raycast_down_skip(pos, xp, down_flags, chk_dist, None)
    }

    /// `BgCheck_RaycastDownImpl` for an actor with its own bg actor `skip`, which it ignores.
    pub fn raycast_down_skip(&self, pos: Vec3, xp: u16, down_flags: u32, chk_dist: f32, skip: Option<u16>) -> (f32, Option<PolyId>) {
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
            let yd = self.raycast_down_dyna(pos, xp, down_flags, chk_dist, y, &mut out, skip);
            if y < yd {
                y = yd;
            }
        }
        if y != BGCHECK_Y_MIN && out.is_some_and(|p| self.surface_is_soft(p)) {
            y -= 1.0;
        }
        (y, out)
    }

    /// `BgCheck_EntityRaycastDown1/3/5` (entity ground check, chkDist 1).
    pub fn entity_raycast_down(&self, pos: Vec3) -> (f32, Option<PolyId>) {
        self.raycast_down(pos, IGNORE_ENTITY, DOWN_CHECK_WALLS_SIMPLE | DOWN_CHECK_FLOORS | DOWN_CHECK_GROUND_ONLY, 1.0)
    }

    /// `BgCheck_EntityRaycastDown4`: the same, for an actor ignoring its own bg actor `own_bg`.
    pub fn entity_raycast_down_actor(&self, pos: Vec3, own_bg: u16) -> (f32, Option<PolyId>) {
        self.raycast_down_skip(pos, IGNORE_ENTITY, DOWN_CHECK_WALLS_SIMPLE | DOWN_CHECK_FLOORS | DOWN_CHECK_GROUND_ONLY, 1.0, Some(own_bg))
    }

    /// `BgCheck_EntityRaycastDown6`: `BgCheck_EntityRaycastDown4` with its own `chk_dist`.
    pub fn entity_raycast_down6(&self, pos: Vec3, own_bg: u16, chk_dist: f32) -> (f32, Option<PolyId>) {
        self.raycast_down_skip(pos, IGNORE_ENTITY, DOWN_CHECK_WALLS_SIMPLE | DOWN_CHECK_FLOORS | DOWN_CHECK_GROUND_ONLY, chk_dist, Some(own_bg))
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
            if !a.in_use() || a.collision_disabled || a.min_y > rp.y || a.max_y < rp.y {
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

    // ---- first poly in a sphere -------------------------------------------------------

    /// `CollisionPoly_SphVsPoly`: `Math3D_TriVsSphIntersect` of the poly and the sphere (its
    /// centre and radius truncated to a `Sphere16`).
    fn sph_vs_poly(p: &CollisionPoly, verts: &[Vec3], center: Vec3, radius: f32) -> bool {
        let tri = crate::math3d::TriNorm { vtx: tri(p, verts), plane: crate::math3d::Plane { normal: normal(p), origin_dist: p.dist as f32 } };
        let sphere = crate::math3d::Sphere16 { center: [center.x as i16, center.y as i16, center.z as i16], radius: radius as i16 };
        crate::math3d::tri_vs_sph_intersect(&sphere, &tri).0
    }

    /// `BgCheck_SphVsFirstStaticPolyList`: the list's first poly the sphere touches. The list is
    /// sorted by the polys' lowest vertex: it stops at the first poly wholly above the sphere.
    fn sph_vs_first_static_poly_list(&self, list: &[u16], xp: u16, center: Vec3, radius: f32) -> Option<PolyId> {
        for &idx in list {
            let p = self.spoly(idx);
            if xp_test(p, xp) {
                continue;
            }
            let [ya, yb, yc] = vy(p, &self.verts);
            if center.y + radius < ya && center.y + radius < yb && center.y + radius < yc {
                break;
            }
            if Self::sph_vs_poly(p, &self.verts, center, radius) {
                return Some(PolyId::scene(idx));
            }
        }
        None
    }

    /// `BgCheck_SphVsFirstDynaPolyList`.
    fn sph_vs_first_dyna_poly_list(&self, bg: u16, list: &[u16], xp: u16, center: Vec3, radius: f32) -> Option<PolyId> {
        for &idx in list {
            let p = &self.dyna.polys[idx as usize];
            if xp_test(p, xp) {
                continue;
            }
            if Self::sph_vs_poly(p, &self.dyna.verts, center, radius) {
                return Some(PolyId { bg, idx });
            }
        }
        None
    }

    /// `BgCheck_SphVsFirstPolyImpl`: the first poly the sphere at `center` touches, the static
    /// mesh's floors, walls and ceilings (`BgCheck_SphVsFirstStaticPoly`), then each bg actor's
    /// ceilings, walls and floors (`BgCheck_SphVsFirstDynaPoly`) whose bounding sphere it
    /// reaches, but `skip`'s (the actor's own). `ignore` is `bciFlags` (`BGCHECK_IGNORE_*`).
    /// None outside the static mesh's bounds (`BgCheck_GetStaticLookup` gives no lookup).
    /// (`@bug (game)`: the C's `outBgId` stays `BGCHECK_SCENE` for a dyna poly; the id here
    /// is the true one, and the public callers don't read it.)
    pub fn sph_vs_first_poly_impl(&self, xp: u16, center: Vec3, radius: f32, skip: Option<u16>, ignore: u16) -> Option<PolyId> {
        if !self.in_bounds(center) {
            return None;
        }
        // BgCheck_SphVsFirstStaticPoly.
        let lists: [(&[u16], u16); 3] = [(&self.floor, BGCHECK_IGNORE_FLOOR), (&self.wall, BGCHECK_IGNORE_WALL), (&self.ceiling, BGCHECK_IGNORE_CEILING)];
        for (list, flag) in lists {
            if !list.is_empty()
                && ignore & flag == 0
                && let Some(id) = self.sph_vs_first_static_poly_list(list, xp, center, radius)
            {
                return Some(id);
            }
        }
        // BgCheck_SphVsFirstDynaPoly.
        let test = crate::math3d::Sphere16 { center: [center.x as i16, center.y as i16, center.z as i16], radius: radius as i16 };
        for (i, a) in self.dyna.actors.iter().enumerate() {
            if !a.in_use() || skip == Some(i as u16) {
                continue;
            }
            let bounding = crate::math3d::Sphere16 { center: a.sphere_center, radius: a.sphere_radius };
            if !crate::math3d::sph_vs_sph(&test, &bounding) {
                continue;
            }
            // BgCheck_SphVsFirstDynaPolyInBgActor.
            let lists: [(&[u16], u16); 3] = [(&a.ceiling, BGCHECK_IGNORE_CEILING), (&a.wall, BGCHECK_IGNORE_WALL), (&a.floor, BGCHECK_IGNORE_FLOOR)];
            for (list, flag) in lists {
                if ignore & flag == 0
                    && let Some(id) = self.sph_vs_first_dyna_poly_list(i as u16, list, xp, center, radius)
                {
                    return Some(id);
                }
            }
        }
        None
    }

    /// `BgCheck_SphVsFirstPoly`: whether the sphere at `center` touches any poly.
    pub fn sph_vs_first_poly(&self, center: Vec3, radius: f32) -> bool {
        self.sph_vs_first_poly_impl(IGNORE_NONE, center, radius, None, BGCHECK_IGNORE_NONE).is_some()
    }

    /// `BgCheck_SphVsFirstWall`: whether the sphere at `center` touches any wall.
    pub fn sph_vs_first_wall(&self, center: Vec3, radius: f32) -> bool {
        self.sph_vs_first_poly_impl(IGNORE_NONE, center, radius, None, BGCHECK_IGNORE_FLOOR | BGCHECK_IGNORE_CEILING).is_some()
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
            if !a.in_use() || a.collision_disabled || !a.xz_in_sphere(test.x, test.z) {
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
    fn line_dyna(&self, xp: u16, a: Vec3, b: &mut Vec3, out: &mut Option<(Vec3, PolyId)>, dist_sq: &mut f32, chk_dist: f32, bcc: u32, skip: Option<u16>) -> bool {
        let d = &self.dyna;
        let mut result = false;
        for (i, act) in d.actors.iter().enumerate() {
            // The actor making the test is skipped (`actor != bgActors[i].actor`).
            if !act.in_use() || act.collision_disabled || skip == Some(i as u16) {
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
        self.check_line_skip(xp1, xp2, a, b, chk_dist, bcc, None)
    }

    /// `BgCheck_CheckLineImpl` for an actor with its own bg actor `skip`, which it ignores.
    #[allow(clippy::too_many_arguments)]
    pub fn check_line_skip(&self, xp1: u16, xp2: u16, a: Vec3, b: Vec3, chk_dist: f32, bcc: u32, skip: Option<u16>) -> Option<(Vec3, PolyId)> {
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
            self.line_dyna(xp1, a, &mut bt, &mut out, &mut dist_sq, chk_dist, bcc, skip);
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

    /// `BgCheck_EntityLineTest3`: `BgCheck_EntityLineTest1` for an actor, skipping its own bg
    /// actor `own_bg`, with `chk_dist`.
    #[allow(clippy::too_many_arguments)]
    pub fn entity_line_test3(&self, a: Vec3, b: Vec3, wall: bool, floor: bool, ceil: bool, one_face: bool, own_bg: u16, chk_dist: f32) -> Option<(Vec3, PolyId)> {
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
        self.check_line_skip(IGNORE_ENTITY, IGNORE_NONE, a, b, chk_dist, bcc, Some(own_bg))
    }

    /// `BgCheck_ProjectileLineTest`: `BgCheck_EntityLineTest1` past the polys projectiles go
    /// through (`COLPOLY_IGNORE_PROJECTILES` rather than `COLPOLY_IGNORE_ENTITY`), bg actors
    /// included.
    pub fn projectile_line_test(&self, a: Vec3, b: Vec3, wall: bool, floor: bool, ceil: bool, one_face: bool) -> Option<(Vec3, PolyId)> {
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
        self.check_line(IGNORE_PROJECTILES, IGNORE_NONE, a, b, 1.0, bcc)
    }

    /// `BgCheck_GetWaterSurface`: the surface height of the first water box (for `room`, or one
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

    /// `colCtx->colHeader->waterBoxes[index].ySurface`, if the scene has that many water boxes.
    pub fn water_box_surface(&self, index: usize) -> Option<i16> {
        self.header.water_boxes.get(index).map(|w| w.y_surface)
    }

    /// `colCtx->colHeader->waterBoxes[index].ySurface = y_surface`: an actor moving a water
    /// box's surface (`Bg_Ydan_Hasi`'s water). The C writes the scene's header in place, so every
    /// later `BgCheck_GetWaterSurface` (`WaterBox_GetSurface1`, Player's `depthInWater`) sees it,
    /// until the scene is loaded again (a new context). Returns whether that box exists (the C
    /// has no check; past the end it writes over whatever follows).
    pub fn set_water_box_surface(&mut self, index: usize, y_surface: i16) -> bool {
        match self.header.water_boxes.get_mut(index) {
            Some(w) => {
                w.y_surface = y_surface;
                true
            }
            None => false,
        }
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
    use crate::collision::CollisionBuilder;

    fn box_room() -> CollisionContext {
        let mut b = CollisionBuilder::new();
        let s = b.surface(0, 0);
        // Floor at y=0, 400x400, and one wall at z=-200 facing +z.
        b.quad(Vec3::new(-200.0, 0.0, 200.0), Vec3::new(200.0, 0.0, 200.0), Vec3::new(200.0, 0.0, -200.0), Vec3::new(-200.0, 0.0, -200.0), s);
        b.quad(Vec3::new(-200.0, 0.0, -200.0), Vec3::new(200.0, 0.0, -200.0), Vec3::new(200.0, 200.0, -200.0), Vec3::new(-200.0, 200.0, -200.0), s);
        // A raised block top at y=40 over x in [100,200].
        b.quad(Vec3::new(100.0, 40.0, 200.0), Vec3::new(200.0, 40.0, 200.0), Vec3::new(200.0, 40.0, 100.0), Vec3::new(100.0, 40.0, 100.0), s);
        CollisionContext::new(b.finish())
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

    /// `DynaPoly_DeleteBgActor` marks the slot; it still collides until the next
    /// `DynaPoly_UpdateContext` frees it, and `DynaPoly_SetBgActor` takes the first free slot.
    #[test]
    fn a_deleted_bg_actor_collides_until_the_next_update_and_its_slot_is_reused() {
        use crate::dyna::{BG_ACTOR_MAX, BgActorSource};
        let mut c = box_room();
        let mut b = CollisionBuilder::new();
        let s = b.surface(0, 0);
        b.quad(Vec3::new(-50.0, 0.0, 50.0), Vec3::new(50.0, 0.0, 50.0), Vec3::new(50.0, 0.0, -50.0), Vec3::new(-50.0, 0.0, -50.0), s);
        let plat = std::sync::Arc::new(b.finish());
        let at = |y: f32| BgActorSource { pos: Vec3::new(0.0, y, 0.0), shape_rot: [0; 3], scale: Vec3::ONE, shape_y_offset: 0.0 };
        let first = c.dyna.set_bg_actor(plat.clone(), at(10.0), 0);
        let second = c.dyna.set_bg_actor(plat.clone(), at(20.0), 0);
        assert_eq!((first, second), (0, 1));
        c.dyna.update_context();
        let top = |c: &CollisionContext| c.entity_raycast_down(Vec3::new(0.0, 100.0, 0.0));
        assert_eq!((top(&c).0, top(&c).1.map(|p| p.bg)), (20.0, Some(1)));

        c.dyna.delete_bg_actor(second);
        assert!(!c.dyna.is_bg_actor(second));
        assert_eq!(top(&c).0, 20.0, "still in the lookup until the update");
        c.dyna.update_context();
        assert_eq!((top(&c).0, top(&c).1.map(|p| p.bg)), (10.0, Some(0)));
        // Deleting it again, or an id that was never set, does nothing.
        c.dyna.delete_bg_actor(second);
        c.dyna.delete_bg_actor(7);

        // The freed slot is the first free one.
        assert_eq!(c.dyna.set_bg_actor(plat.clone(), at(30.0), 0), 1);
        c.dyna.update_context();
        assert_eq!((top(&c).0, top(&c).1.map(|p| p.bg)), (30.0, Some(1)));
        // BG_ACTOR_MAX slots, then no more.
        for _ in 2..BG_ACTOR_MAX {
            c.dyna.set_bg_actor(plat.clone(), at(0.0), 0);
        }
        assert_eq!(c.dyna.set_bg_actor(plat, at(0.0), 0), BG_ACTOR_MAX);
    }

    /// Two bg actors made from one header (as `DynaPoly_SetBgActor` points both at their object's
    /// `CollisionHeader`): a write into it reaches both, but `DynaPoly_AddBgActorToLookup` reads
    /// the header again only for one whose transform changed, or when the lookup is invalidated.
    #[test]
    fn a_shared_headers_new_vertices_reach_a_bg_actor_when_it_moves_or_the_lookup_is_invalidated() {
        use crate::dyna::BgActorSource;
        let mut c = box_room();
        let mut b = CollisionBuilder::new();
        let s = b.surface(0, 0);
        b.quad(Vec3::new(-50.0, 0.0, 50.0), Vec3::new(50.0, 0.0, 50.0), Vec3::new(50.0, 0.0, -50.0), Vec3::new(-50.0, 0.0, -50.0), s);
        let plat = std::sync::Arc::new(b.finish());
        let at = |x: f32, y: f32| BgActorSource { pos: Vec3::new(x, y, 0.0), shape_rot: [0; 3], scale: Vec3::ONE, shape_y_offset: 0.0 };
        let still = c.dyna.set_bg_actor(plat.clone(), at(-120.0, 10.0), 0);
        let moving = c.dyna.set_bg_actor(plat.clone(), at(0.0, 10.0), 0);
        c.dyna.update_context();
        c.dyna.update_prev_transforms();
        let top = |c: &CollisionContext, x: f32| c.entity_raycast_down(Vec3::new(x, 100.0, 0.0)).0;
        assert_eq!((top(&c, -120.0), top(&c, 0.0)), (10.0, 10.0));

        // The shared header's vertices go up 5.
        let mut raised = (*plat).clone();
        raised.vertices.iter_mut().for_each(|v| v[1] += 5);
        let raised = std::sync::Arc::new(raised);
        c.dyna.replace_shared_header(&plat, raised.clone());
        assert!(std::sync::Arc::ptr_eq(&c.dyna.actors[still as usize].header, &raised));
        c.dyna.set_source(moving, at(0.0, 11.0));
        c.dyna.update_context();
        c.dyna.update_prev_transforms();
        // The one that moved reads the new vertices; the one standing still keeps its old ones.
        assert_eq!((top(&c, -120.0), top(&c, 0.0)), (10.0, 16.0));
        // DynaPoly_EnableCollision invalidates the lookup: both are expanded again.
        c.dyna.set_collision_disabled(still, false);
        c.dyna.update_context();
        assert_eq!((top(&c, -120.0), top(&c, 0.0)), (15.0, 16.0));
    }

    /// `waterBoxes[i].ySurface` written in place (`Bg_Ydan_Hasi`'s water): the next
    /// `BgCheck_GetWaterSurface` reads the new height; the other boxes keep theirs.
    #[test]
    fn a_water_boxs_surface_written_in_place_is_what_the_water_checks_see() {
        let mut b = CollisionBuilder::new();
        let s = b.surface(0, 0);
        b.quad(Vec3::new(-200.0, -100.0, 200.0), Vec3::new(200.0, -100.0, 200.0), Vec3::new(200.0, -100.0, -200.0), Vec3::new(-200.0, -100.0, -200.0), s);
        // Box 0 for room 3 only, box 1 for every room (WATERBOX_ROOM_ALL).
        b.water_box(-200, -200, 100, 400, -10, 3);
        b.water_box(0, -200, 200, 400, -20, 0x3F);
        let mut c = CollisionContext::new(b.finish());
        assert_eq!((c.water_box_surface(0), c.water_box_surface(1), c.water_box_surface(2)), (Some(-10), Some(-20), None));
        assert_eq!(c.water_surface(100.0, 0.0, 0), Some(-20.0));
        // home.pos.y - 47, truncated to the s16.
        assert!(c.set_water_box_surface(1, (-20.0f32 - 47.0) as i16));
        assert_eq!(c.water_surface(100.0, 0.0, 0), Some(-67.0));
        assert_eq!(c.water_surface(-150.0, 0.0, 3), Some(-10.0));
        assert_eq!(c.header.water_boxes[1].y_surface, -67);
        // No box 2: nothing written.
        assert!(!c.set_water_box_surface(2, 0));
        assert_eq!(c.header.water_boxes.len(), 2);
    }

    /// `DynaPolyActor.unk_150`/`unk_158` on the bg actor slot (`func_8002DFA4` adds and sets them
    /// only for a bg actor in use; `DynaPolyActor_Init` zeroes them), and the tests an actor makes
    /// past its own collision: `BgCheck_EntityRaycastDown6` and `BgCheck_EntityLineTest3`.
    #[test]
    fn a_bg_actors_push_and_the_tests_that_skip_its_own_collision() {
        use crate::dyna::BgActorSource;
        let mut c = box_room();
        let mut b = CollisionBuilder::new();
        let s = b.surface(0, 0);
        // A 40-high box's top and its +x face (facing +x).
        b.quad(Vec3::new(-20.0, 40.0, 20.0), Vec3::new(20.0, 40.0, 20.0), Vec3::new(20.0, 40.0, -20.0), Vec3::new(-20.0, 40.0, -20.0), s);
        b.quad(Vec3::new(20.0, 0.0, 20.0), Vec3::new(20.0, 0.0, -20.0), Vec3::new(20.0, 40.0, -20.0), Vec3::new(20.0, 40.0, 20.0), s);
        let block = std::sync::Arc::new(b.finish());
        let src = BgActorSource { pos: Vec3::ZERO, shape_rot: [0; 3], scale: Vec3::ONE, shape_y_offset: 0.0 };
        let bg = c.dyna.set_bg_actor(block.clone(), src, 1);
        c.dyna.update_context();
        assert_eq!((c.dyna.unk_150(bg), c.dyna.unk_154(bg), c.dyna.unk_158(bg)), (0.0, 0.0, 0));
        c.dyna.func_8002DFA4(bg, 2.0, 0x4000);
        c.dyna.func_8002DFA4(bg, 2.0, 0x4000);
        assert_eq!((c.dyna.unk_150(bg), c.dyna.unk_158(bg)), (4.0, 0x4000));
        c.dyna.func_8002DF90(bg);
        assert_eq!((c.dyna.unk_150(bg), c.dyna.unk_158(bg)), (0.0, 0x4000));
        // An id not in use: nothing.
        c.dyna.func_8002DFA4(7, 2.0, 0);
        assert_eq!(c.dyna.unk_150(7), 0.0);
        // Down onto its top, or past it to the floor when it's the actor's own.
        assert_eq!(c.entity_raycast_down(Vec3::new(0.0, 100.0, 0.0)).0, 40.0);
        assert_eq!(c.entity_raycast_down6(Vec3::new(0.0, 100.0, 0.0), bg, 0.0).0, 0.0);
        // Across its +x face from outside, or through it for its own test.
        let (a, z) = (Vec3::new(60.0, 10.0, 0.0), Vec3::new(-60.0, 10.0, 0.0));
        assert_eq!(c.entity_line_test(a, z, true, false, false, true).map(|h| h.1.bg), Some(bg));
        assert_eq!(c.entity_line_test3(a, z, true, false, false, true, bg, 0.0), None);
    }
}
