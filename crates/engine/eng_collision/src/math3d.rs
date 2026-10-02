//! Geometry primitives and tests ported from `sys_math3d.c` (decomp commit 52a510f): spheres,
//! cylinders, triangles with planes, and line segments.
//!
//! Faithful f32 port: every expression keeps the C's order of operations and the places where
//! the C converts `s16` fields of `Sphere16` / `Cylinder16` to f32. C out-parameters are return
//! values (`(hit, outs...)` in C parameter order); an out the C leaves unwritten on a path is
//! `Default::default()` there. `IS_ZERO(f)` is `fabsf(f) < 0.008f` (`eng_math::is_zero`).
//!
//! The C's `sqrt` (double) calls on f32 sums (`Math3D_Vec3fMagnitude`,
//! `Math3D_SphVsSphOverlapCenterDist`) are f32 `sqrt` here: a correctly rounded double sqrt of an
//! f32, rounded back to f32, equals the correctly rounded f32 sqrt.

use eng_math::is_zero;
use glam::Vec3;

pub use crate::bgcheck::{dist_plane_to_pos, udist_plane_to_pos};
/// `Math3D_TriChkPointParaXImpl` (shared with `bgcheck`, where it is `tri_chk_point_para_x`).
pub use crate::bgcheck::tri_chk_point_para_x as tri_chk_point_para_x_impl;
/// `Math3D_TriChkPointParaYImpl` (shared with `bgcheck`, where it is `tri_chk_point_para_y`).
pub use crate::bgcheck::tri_chk_point_para_y as tri_chk_point_para_y_impl;
/// `Math3D_TriChkPointParaZImpl` (shared with `bgcheck`, where it is `tri_chk_point_para_z`).
pub use crate::bgcheck::tri_chk_point_para_z as tri_chk_point_para_z_impl;

/// `Sphere16` (`z_math.h`): a sphere with integer center and radius.
#[derive(Debug, Clone, Copy, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Sphere16 {
    pub center: [i16; 3],
    pub radius: i16,
}

/// `Cylinder16` (`z_math.h`): a vertical cylinder whose bottom is at `pos.y + yShift`.
#[derive(Debug, Clone, Copy, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Cylinder16 {
    pub radius: i16,
    pub height: i16,
    pub y_shift: i16,
    pub pos: [i16; 3],
}

/// `Plane` (`z_math.h`): `normal · p + originDist = 0`.
#[derive(Debug, Clone, Copy, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Plane {
    pub normal: Vec3,
    pub origin_dist: f32,
}

/// `TriNorm` (`z_math.h`): a triangle with its plane.
#[derive(Debug, Clone, Copy, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TriNorm {
    pub vtx: [Vec3; 3],
    pub plane: Plane,
}

/// `Linef` (`z_math.h`): a line segment from `a` to `b`.
#[derive(Debug, Clone, Copy, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Linef {
    pub a: Vec3,
    pub b: Vec3,
}

/// `Vec3s` → `Vec3f` component by component, as the C's implicit `s16` → `f32` conversions.
fn s16_to_f(v: [i16; 3]) -> Vec3 {
    Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32)
}

/// `SQ(s16)`: the C squares the `s16` in `int` arithmetic, then converts the product to f32.
fn sq_s16(v: i16) -> f32 {
    (v as i32 * v as i32) as f32
}

/// `Math_Vec3f_Diff(a, b)` (`z_lib.c`): `a - b`.
fn diff(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

/// `Math3D_Vec3fMagnitudeSq`.
pub fn vec3f_magnitude_sq(v: Vec3) -> f32 {
    v.x * v.x + v.y * v.y + v.z * v.z
}

/// `Math3D_Vec3fMagnitude` (C `sqrt` in double; see the module doc).
pub fn vec3f_magnitude(v: Vec3) -> f32 {
    vec3f_magnitude_sq(v).sqrt()
}

/// `Math3D_Vec3fDistSq`: `|a - b|²`.
pub fn vec3f_dist_sq(a: Vec3, b: Vec3) -> f32 {
    vec3f_magnitude_sq(diff(a, b))
}

/// `Math3D_Vec3f_DistXYZ` → `Math_Vec3f_DistXYZ` (`z_lib.c`): `sqrtf` of `b - a` squared.
pub fn vec3f_dist_xyz(a: Vec3, b: Vec3) -> f32 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let dz = b.z - a.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// `Math3D_DistXYZ16toF`: distance from the integer point `a` to `b`.
pub fn dist_xyz16_to_f(a: [i16; 3], b: Vec3) -> f32 {
    let d = Vec3::new(a[0] as f32 - b.x, a[1] as f32 - b.y, a[2] as f32 - b.z);
    vec3f_magnitude(d)
}

/// `Math3D_Vec3f_Cross`: `a × b`.
pub fn vec3f_cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.y * b.z - a.z * b.y, a.z * b.x - a.x * b.z, a.x * b.y - a.y * b.x)
}

/// `Math3D_SurfaceNorm`: the unnormalised normal `(vb - va) × (vc - va)`.
pub fn surface_norm(va: Vec3, vb: Vec3, vc: Vec3) -> Vec3 {
    vec3f_cross(diff(vb, va), diff(vc, va))
}

/// `Math3D_DefPlane`: the unit normal and origin distance of the plane through `va`, `vb`,
/// `vc` (counter-clockwise seen from the front). A degenerate triangle (`IS_ZERO` normal
/// length) gives a zero normal and a zero distance.
pub fn def_plane(va: Vec3, vb: Vec3, vc: Vec3) -> (Vec3, f32) {
    let normal = surface_norm(va, vb, vc);
    let norm_magnitude = (normal.x * normal.x + normal.y * normal.y + normal.z * normal.z).sqrt();
    if !is_zero(norm_magnitude) {
        let inv = 1.0 / norm_magnitude;
        let n = Vec3::new(normal.x * inv, normal.y * inv, normal.z * inv);
        let origin_dist = -((n.x * va.x) + (n.y * va.y) + (n.z * va.z));
        (n, origin_dist)
    } else {
        (Vec3::ZERO, 0.0)
    }
}

/// `Math3D_Planef`: `nx·p.x + ny·p.y + nz·p.z + originDist`.
pub fn planef(nx: f32, ny: f32, nz: f32, origin_dist: f32, p: Vec3) -> f32 {
    (nx * p.x) + (ny * p.y) + (nz * p.z) + origin_dist
}

/// `Math3D_Plane`: the plane equation of `plane` at `p`.
pub fn plane(plane: &Plane, p: Vec3) -> f32 {
    (plane.normal.x * p.x) + (plane.normal.y * p.y) + (plane.normal.z * p.z) + plane.origin_dist
}

/// `Math3D_PointOnInfiniteLine`: `v0 + dir·dist`.
pub fn point_on_infinite_line(v0: Vec3, dir: Vec3, dist: f32) -> Vec3 {
    Vec3::new((dir.x * dist) + v0.x, (dir.y * dist) + v0.y, (dir.z * dist) + v0.z)
}

/// `Math3D_LineClosestToPoint`: the point of the line through `point` along `dir` closest to
/// `pos`. @bug (game): with a zero `dir` it copies `pos`, then goes on without returning and
/// divides by zero, as the C does.
pub fn line_closest_to_point(point: Vec3, dir: Vec3, pos: Vec3) -> Vec3 {
    let len_sq = vec3f_magnitude_sq(dir);
    let t = (((pos.x - point.x) * dir.x) + ((pos.y - point.y) * dir.y) + ((pos.z - point.z) * dir.z)) / len_sq;
    Vec3::new((dir.x * t) + point.x, (dir.y * t) + point.y, (dir.z * t) + point.z)
}

/// `Math3D_LineSplitRatio`: `v0 + (v1 - v0)·ratio`.
pub fn line_split_ratio(v0: Vec3, v1: Vec3, ratio: f32) -> Vec3 {
    point_on_infinite_line(v0, diff(v1, v0), ratio)
}

/// `Math3D_SphCubeVsTriCube`: whether the triangle's bounding box grown by `radius` contains
/// `center`.
pub fn sph_cube_vs_tri_cube(v0: Vec3, v1: Vec3, v2: Vec3, center: Vec3, radius: f32) -> bool {
    let (mut min_x, mut max_x) = (v0.x, v0.x);
    let (mut min_y, mut max_y) = (v0.y, v0.y);
    let (mut min_z, mut max_z) = (v0.z, v0.z);
    for v in [v1, v2] {
        if v.x < min_x {
            min_x = v.x;
        } else if max_x < v.x {
            max_x = v.x;
        }
        if v.y < min_y {
            min_y = v.y;
        } else if max_y < v.y {
            max_y = v.y;
        }
        if v.z < min_z {
            min_z = v.z;
        } else if max_z < v.z {
            max_z = v.z;
        }
    }
    center.x >= (min_x - radius)
        && center.x <= (max_x + radius)
        && center.y >= (min_y - radius)
        && center.y <= (max_y + radius)
        && center.z >= (min_z - radius)
        && center.z <= (max_z + radius)
}

/// `Math3D_TriChkPointParaYDeterminate`: `Math3D_TriChkPointParaYImpl` with `chkDist` 1.
pub fn tri_chk_point_para_y_determinate(v0: Vec3, v1: Vec3, v2: Vec3, z: f32, x: f32, det_max: f32, ny: f32) -> bool {
    tri_chk_point_para_y_impl(v0, v1, v2, z, x, det_max, 1.0, ny)
}

/// `Math3D_TriChkPointParaY`: whether (`z`, `x`) lies in the triangle projected along Y
/// (`detMax` 300, `chkDist` 1). False when `IS_ZERO(ny)`.
pub fn tri_chk_point_para_y(v0: Vec3, v1: Vec3, v2: Vec3, ny: f32, z: f32, x: f32) -> bool {
    if is_zero(ny) {
        return false;
    }
    tri_chk_point_para_y_impl(v0, v1, v2, z, x, 300.0, 1.0, ny)
}

/// `Math3D_TriChkLineSegParaYIntersect`: whether the vertical segment at (`x`, `z`) from `y0`
/// to `y1` crosses the triangle's plane inside the triangle (`detMax` 300, `chkDist` 1), and
/// the plane's y there. False when `IS_ZERO(ny)` or both ends are strictly on one side.
#[allow(clippy::too_many_arguments)]
pub fn tri_chk_line_seg_para_y_intersect(
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    nx: f32,
    ny: f32,
    nz: f32,
    origin_dist: f32,
    z: f32,
    x: f32,
    y0: f32,
    y1: f32,
) -> (bool, f32) {
    if is_zero(ny) {
        return (false, 0.0);
    }
    let point_a_dist = planef(nx, ny, nz, origin_dist, Vec3::new(x, y0, z));
    let point_b_dist = planef(nx, ny, nz, origin_dist, Vec3::new(x, y1, z));
    if (point_a_dist > 0.0 && point_b_dist > 0.0) || (point_a_dist < 0.0 && point_b_dist < 0.0) {
        return (false, 0.0);
    }
    if tri_chk_point_para_y_impl(v0, v1, v2, z, x, 300.0, 1.0, ny) {
        return (true, (((-nx * x) - (nz * z)) - origin_dist) / ny);
    }
    (false, 0.0)
}

/// `Math3D_TriChkPointParaXDeterminate`: `Math3D_TriChkPointParaXImpl` with `chkDist` 1.
pub fn tri_chk_point_para_x_determinate(v0: Vec3, v1: Vec3, v2: Vec3, y: f32, z: f32, det_max: f32, nx: f32) -> bool {
    tri_chk_point_para_x_impl(v0, v1, v2, y, z, det_max, 1.0, nx)
}

/// `Math3D_TriChkPointParaX`: whether (`y`, `z`) lies in the triangle projected along X
/// (`detMax` 300, `chkDist` 1). False when `IS_ZERO(nx)`.
pub fn tri_chk_point_para_x(v0: Vec3, v1: Vec3, v2: Vec3, nx: f32, y: f32, z: f32) -> bool {
    if is_zero(nx) {
        return false;
    }
    tri_chk_point_para_x_impl(v0, v1, v2, y, z, 300.0, 1.0, nx)
}

/// `Math3D_TriChkPointParaZDeterminate`: `Math3D_TriChkPointParaZImpl` with `chkDist` 1.
pub fn tri_chk_point_para_z_determinate(v0: Vec3, v1: Vec3, v2: Vec3, x: f32, y: f32, det_max: f32, nz: f32) -> bool {
    tri_chk_point_para_z_impl(v0, v1, v2, x, y, det_max, 1.0, nz)
}

/// `Math3D_TriChkPointParaZ`: whether (`x`, `y`) lies in the triangle projected along Z
/// (`detMax` 300, `chkDist` 1). False when `IS_ZERO(nz)`.
pub fn tri_chk_point_para_z(v0: Vec3, v1: Vec3, v2: Vec3, nz: f32, x: f32, y: f32) -> bool {
    if is_zero(nz) {
        return false;
    }
    tri_chk_point_para_z_impl(v0, v1, v2, x, y, 300.0, 1.0, nz)
}

/// `Math3D_LineSegFindPlaneIntersect`: the point of segment `a`→`b` on the plane, from the
/// ends' plane values. False (and `b`) when `IS_ZERO(pointADist - pointBDist)`.
pub fn line_seg_find_plane_intersect(point_a_dist: f32, point_b_dist: f32, a: Vec3, b: Vec3) -> (bool, Vec3) {
    let dist_diff = point_a_dist - point_b_dist;
    if is_zero(dist_diff) {
        return (false, b);
    }
    if point_a_dist == 0.0 {
        (true, a)
    } else if point_b_dist == 0.0 {
        (true, b)
    } else {
        (true, line_split_ratio(a, b, point_a_dist / dist_diff))
    }
}

/// `Math3D_LineSegVsPlane`: whether segment `a`→`b` crosses the plane, and where. With
/// `from_front`, a segment going from the back (a < 0) to the front (b > 0) does not count.
/// On a miss the out is `b`, as the C writes it.
#[allow(clippy::too_many_arguments)]
pub fn line_seg_vs_plane(nx: f32, ny: f32, nz: f32, origin_dist: f32, a: Vec3, b: Vec3, from_front: bool) -> (bool, Vec3) {
    let point_a_dist = planef(nx, ny, nz, origin_dist, a);
    let point_b_dist = planef(nx, ny, nz, origin_dist, b);
    if (point_a_dist * point_b_dist) > 0.0 {
        return (false, b);
    }
    if from_front && point_a_dist < 0.0 && point_b_dist > 0.0 {
        return (false, b);
    }
    line_seg_find_plane_intersect(point_a_dist, point_b_dist, a, b)
}

/// `Math3D_TriLineIntersect`: whether segment `a`→`b` crosses the triangle, and where (`b` on
/// a miss after the plane test passed, as the C writes it).
///
/// @bug (game): an axis is skipped only when its normal component is exactly 0, but
/// `Math3D_TriChkPointParaX/Y/Z` reject `IS_ZERO` components (|n| < 0.008), so a triangle
/// with a normal component in (0, 0.008) is never hit.
#[allow(clippy::too_many_arguments)]
pub fn tri_line_intersect(
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    nx: f32,
    ny: f32,
    nz: f32,
    origin_dist: f32,
    a: Vec3,
    b: Vec3,
    from_front: bool,
) -> (bool, Vec3) {
    let (hit, intersect) = line_seg_vs_plane(nx, ny, nz, origin_dist, a, b, from_front);
    if !hit {
        return (false, intersect);
    }
    if (nx == 0.0 || tri_chk_point_para_x(v0, v1, v2, nx, intersect.y, intersect.z))
        && (ny == 0.0 || tri_chk_point_para_y(v0, v1, v2, ny, intersect.z, intersect.x))
        && (nz == 0.0 || tri_chk_point_para_z(v0, v1, v2, nz, intersect.x, intersect.y))
    {
        return (true, intersect);
    }
    (false, b)
}

/// `Math3D_TriNorm`: the triangle `va`, `vb`, `vc` with its `Math3D_DefPlane` plane.
pub fn tri_norm(va: Vec3, vb: Vec3, vc: Vec3) -> TriNorm {
    let (normal, origin_dist) = def_plane(va, vb, vc);
    TriNorm { vtx: [va, vb, vc], plane: Plane { normal, origin_dist } }
}

/// `Math3D_PointInSph`: whether `point` is strictly inside the sphere.
pub fn point_in_sph(sphere: &Sphere16, point: Vec3) -> bool {
    dist_xyz16_to_f(sphere.center, point) < sphere.radius as f32
}

/// `Math3D_LineVsSph`: whether the segment touches the sphere (an end strictly inside, or the
/// perpendicular foot within the segment at distance <= radius).
// The C's `t < 0 || t > 1` is kept over `RangeInclusive::contains`: they differ on NaN.
#[allow(clippy::manual_range_contains)]
pub fn line_vs_sph(sphere: &Sphere16, line: &Linef) -> bool {
    if point_in_sph(sphere, line.a) || point_in_sph(sphere, line.b) {
        return true;
    }
    let d = Vec3::new(line.b.x - line.a.x, line.b.y - line.a.y, line.b.z - line.a.z);
    let line_len_sq = d.x * d.x + d.y * d.y + d.z * d.z;
    if is_zero(line_len_sq) {
        return false;
    }
    let c = s16_to_f(sphere.center);
    let t = ((((c.x - line.a.x) * d.x) + ((c.y - line.a.y) * d.y)) + ((c.z - line.a.z) * d.z)) / line_len_sq;
    if t < 0.0 || t > 1.0 {
        return false;
    }
    let p = Vec3::new((d.x * t) + line.a.x, (d.y * t) + line.a.y, (d.z * t) + line.a.z);
    let r = sphere.radius as f32;
    (p.x - c.x) * (p.x - c.x) + (p.y - c.y) * (p.y - c.y) + (p.z - c.z) * (p.z - c.z) <= r * r
}

/// `Math3D_GetSphVsTriIntersectPoint`: the point `radius` away from the sphere's center
/// towards the midpoint of the triangle's `vtx[0]`-`vtx[1]` edge (the center itself when that
/// midpoint is `IS_ZERO` away). Not necessarily on the triangle.
pub fn get_sph_vs_tri_intersect_point(sphere: &Sphere16, tri: &TriNorm) -> Vec3 {
    let [v0, v1, _] = tri.vtx;
    let mid = Vec3::new((v0.x + v1.x) * 0.5, (v0.y + v1.y) * 0.5, (v0.z + v1.z) * 0.5);
    let center = s16_to_f(sphere.center);
    let dist = vec3f_dist_xyz(mid, center);
    if is_zero(dist) {
        return center;
    }
    let split_ratio = sphere.radius as f32 / dist;
    line_split_ratio(center, mid, split_ratio)
}

/// `Math3D_TriVsSphIntersect`: whether the sphere touches the triangle; the point is
/// `Math3D_GetSphVsTriIntersectPoint`.
pub fn tri_vs_sph_intersect(sphere: &Sphere16, tri: &TriNorm) -> (bool, Vec3) {
    let center = s16_to_f(sphere.center);
    let radius = sphere.radius as f32;
    let [v0, v1, v2] = tri.vtx;
    let n = tri.plane.normal;

    if !sph_cube_vs_tri_cube(v0, v1, v2, center, radius) {
        return (false, Vec3::default());
    }
    let plane_dist = udist_plane_to_pos(n, tri.plane.origin_dist, center);
    if radius < plane_dist {
        return (false, Vec3::default());
    }
    for (a, b) in [(v0, v1), (v1, v2), (v2, v0)] {
        if line_vs_sph(sphere, &Linef { a, b }) {
            return (true, get_sph_vs_tri_intersect_point(sphere, tri));
        }
    }

    let nx = n.x * plane_dist;
    let ny = n.y * plane_dist;
    let nz = n.z * plane_dist;
    // The sphere center projected onto the plane.
    let sph_plane_pos = if planef(n.x, n.y, n.z, tri.plane.origin_dist, center) > 0.0 {
        Vec3::new(center.x - nx, center.y - ny, center.z - nz)
    } else {
        Vec3::new(center.x + nx, center.y + ny, center.z + nz)
    };

    let inside = if n.y.abs() > 0.5 {
        tri_chk_point_para_y_determinate(v0, v1, v2, sph_plane_pos.z, sph_plane_pos.x, 0.0, n.y)
    } else if n.x.abs() > 0.5 {
        tri_chk_point_para_x_determinate(v0, v1, v2, sph_plane_pos.y, sph_plane_pos.z, 0.0, n.x)
    } else {
        tri_chk_point_para_z_determinate(v0, v1, v2, sph_plane_pos.x, sph_plane_pos.y, 0.0, n.z)
    };
    if inside {
        return (true, get_sph_vs_tri_intersect_point(sphere, tri));
    }
    (false, Vec3::default())
}

/// `Math3D_PointInCyl`: whether `point` is strictly inside the cylinder.
pub fn point_in_cyl(cyl: &Cylinder16, point: Vec3) -> bool {
    let x = cyl.pos[0] as f32 - point.x;
    let z = cyl.pos[2] as f32 - point.z;
    let bottom = cyl.pos[1] as f32 + cyl.y_shift as f32;
    let top = cyl.height as f32 + bottom;
    (x * x + z * z) < sq_s16(cyl.radius) && bottom < point.y && point.y < top
}

/// `Math3D_CylVsLineSeg`: intersections of segment `a`→`b` with the cylinder. Returns the C's
/// count and `intersectA`, `intersectB`: 2 with (`a`, `b`) when both ends are inside the
/// cylinder, otherwise 0 or 1 (see the bugs), `intersectB` being `Default` unless two points
/// were found.
///
/// @bug (game): the result is 0 whenever no side (wall) intersection survives the segment and
/// height filters, even if the segment crosses the base or top: a vertical segment through
/// both caps, or one ending inside the cylinder without crossing the wall, is a miss.
///
/// @bug (game): when two points are found the loop `break`s before `count++`, so the count is
/// 1, never 2 (callers only test `!= 0`).
///
/// @bug (game): the two points are ordered by comparing `|A' - a|²` with `|A' - P|²` (A' the
/// first point found, P the second) instead of each point's distance to `a`, so
/// `intersectA` is not reliably the point nearer `a`.
// The C's `f < 0 || 1 < f` is kept over `RangeInclusive::contains`: they differ on NaN.
#[allow(clippy::manual_range_contains)]
pub fn cyl_vs_line_seg(cyl: &Cylinder16, a: Vec3, b: Vec3) -> (i32, Vec3, Vec3) {
    if point_in_cyl(cyl, a) && point_in_cyl(cyl, b) {
        return (2, a, b);
    }
    let miss = (0, Vec3::default(), Vec3::default());
    let (px, py, pz) = (cyl.pos[0] as f32, cyl.pos[1] as f32, cyl.pos[2] as f32);
    let y_shift = cyl.y_shift as f32;
    let height = cyl.height as f32;

    let cyl_to_a = Vec3::new(a.x - px, a.y - py - y_shift, a.z - pz);
    let cyl_to_b = Vec3::new(b.x - px, b.y - py - y_shift, b.z - pz);
    let ab = diff(cyl_to_b, cyl_to_a);
    let cyl_radius_sq = sq_s16(cyl.radius);

    let mut int_pts = [Vec3::ZERO; 4];
    let mut int_flags = 0u32;
    let mut frac_a = 0.0f32;
    let mut frac_b = 0.0f32;

    // Base and top.
    if !is_zero(ab.y) {
        let frac_base = -cyl_to_a.y / ab.y;
        if (0.0..=1.0).contains(&frac_base) {
            let base_x = (ab.x * frac_base) + cyl_to_a.x;
            let base_z = (ab.z * frac_base) + cyl_to_a.z;
            if base_x * base_x + base_z * base_z < cyl_radius_sq {
                int_pts[0] = Vec3::new(px + base_x, py + y_shift, pz + base_z);
                int_flags |= 1;
            }
        }
        frac_a = (height - cyl_to_a.y) / ab.y;
        if (0.0..=1.0).contains(&frac_a) {
            let top_x = ab.x * frac_a + cyl_to_a.x;
            let top_z = ab.z * frac_a + cyl_to_a.z;
            if top_x * top_x + top_z * top_z < cyl_radius_sq {
                int_pts[1] = Vec3::new(px + top_x, py + y_shift + height, pz + top_z);
                int_flags |= 2;
            }
        }
    }

    // The infinite line against the infinite cylinder wall (xz quadratic).
    let sqxz_ab = ab.x * ab.x + ab.z * ab.z;
    let dotxz = ab.x * cyl_to_a.x + ab.z * cyl_to_a.z;
    let rad_sq_diff = (cyl_to_a.x * cyl_to_a.x + cyl_to_a.z * cyl_to_a.z) - cyl_radius_sq;
    let mut side_a;
    let mut side_b;
    if !is_zero(2.0 * sqxz_ab) {
        let dot2ab = 2.0 * dotxz;
        let disc_term = 4.0 * sqxz_ab * rad_sq_diff;
        if dot2ab * dot2ab < disc_term {
            return miss;
        }
        if dot2ab * dot2ab - disc_term > 0.0 {
            side_a = true;
            side_b = true;
        } else {
            // Tangent in xz: at most one wall intersection.
            side_a = true;
            side_b = false;
        }
        let dist_cent2 = (dot2ab * dot2ab - disc_term).sqrt();
        if side_a {
            frac_a = (dist_cent2 - dot2ab) / (2.0 * sqxz_ab);
        }
        if side_b {
            frac_b = (-dot2ab - dist_cent2) / (2.0 * sqxz_ab);
        }
    } else if !is_zero(2.0 * dotxz) {
        // Nearly vertical segment.
        frac_a = -rad_sq_diff / (2.0 * dotxz);
        side_a = true;
        side_b = false;
    } else {
        return miss;
    }

    // Wall intersections beyond the segment.
    if !side_b {
        if frac_a < 0.0 || 1.0 < frac_a {
            return miss;
        }
    } else {
        let beyond_a = frac_a < 0.0 || 1.0 < frac_a;
        let beyond_b = frac_b < 0.0 || 1.0 < frac_b;
        if beyond_a && beyond_b {
            return miss;
        }
        if beyond_a {
            side_a = false;
        }
        if beyond_b {
            side_b = false;
        }
    }
    // Wall intersections below the base or above the top.
    if side_a && ((frac_a * ab.y + cyl_to_a.y) < 0.0 || height < (frac_a * ab.y + cyl_to_a.y)) {
        side_a = false;
    }
    if side_b && ((frac_b * ab.y + cyl_to_a.y) < 0.0 || height < (frac_b * ab.y + cyl_to_a.y)) {
        side_b = false;
    }
    if !side_a && !side_b {
        return miss;
    }

    let wall_point = |frac: f32| {
        Vec3::new(
            (frac * ab.x + cyl_to_a.x) + px,
            (frac * ab.y + cyl_to_a.y) + py + y_shift,
            (frac * ab.z + cyl_to_a.z) + pz,
        )
    };
    if side_a && side_b {
        int_pts[2] = wall_point(frac_a);
        int_flags |= 4;
        int_pts[3] = wall_point(frac_b);
        int_flags |= 8;
    } else if side_a {
        int_pts[2] = wall_point(frac_a);
        int_flags |= 4;
    } else if side_b {
        int_pts[2] = wall_point(frac_b);
        int_flags |= 4;
    }

    let mut intersect_a = Vec3::default();
    let mut intersect_b = Vec3::default();
    let mut count = 0;
    for (i, &pt) in int_pts.iter().enumerate() {
        if int_flags & (1 << i) != 0 {
            if count == 0 {
                intersect_a = pt;
            } else if count == 1 {
                if vec3f_dist_sq(intersect_a, a) < vec3f_dist_sq(intersect_a, pt) {
                    intersect_b = pt;
                } else {
                    intersect_b = intersect_a;
                    intersect_a = pt;
                }
                break;
            }
            count += 1;
        }
    }
    (count, intersect_a, intersect_b)
}

/// `Math3D_CylTriVsIntersect`: whether the cylinder touches the triangle, and a contact point:
/// the edge's `Math3D_CylVsLineSeg` `intersectA` nearest its first vertex (edges v0→v1,
/// v2→v1, v0→v2; nearest wins, `1e38` start), else the cylinder axis crossing the triangle
/// (`Math3D_TriChkLineSegParaYIntersect` from bottom to top) pushed `radius` towards the v0-v1
/// midpoint, else `Math3D_TriVsSphIntersect` with spheres of the cylinder's radius centred on
/// its top then its bottom.
pub fn cyl_tri_vs_intersect(cyl: &Cylinder16, tri: &TriNorm) -> (bool, Vec3) {
    let [v0, v1, v2] = tri.vtx;
    let cyl_bottom = cyl.pos[1] as f32 + cyl.y_shift as f32;
    let cyl_top = cyl.height as f32 + cyl_bottom;

    if (v0.y < cyl_bottom && v1.y < cyl_bottom && v2.y < cyl_bottom) || (cyl_top < v0.y && cyl_top < v1.y && cyl_top < v2.y) {
        return (false, Vec3::default());
    }

    let mut intersect = Vec3::default();
    let mut min_dist_sq = 1.0e38f32;
    let (n, cyl_a, _) = cyl_vs_line_seg(cyl, v0, v1);
    if n != 0 {
        min_dist_sq = vec3f_dist_sq(cyl_a, v0);
        intersect = cyl_a;
    }
    let (n, cyl_a, _) = cyl_vs_line_seg(cyl, v2, v1);
    if n != 0 {
        let d = vec3f_dist_sq(cyl_a, v2);
        if d < min_dist_sq {
            intersect = cyl_a;
            min_dist_sq = d;
        }
    }
    let (n, cyl_a, _) = cyl_vs_line_seg(cyl, v0, v2);
    if n != 0 {
        let d = vec3f_dist_sq(cyl_a, v0);
        if d < min_dist_sq {
            intersect = cyl_a;
            min_dist_sq = d;
        }
    }
    if min_dist_sq != 1.0e38 {
        return (true, intersect);
    }

    let p = tri.plane;
    let (px, pz) = (cyl.pos[0] as f32, cyl.pos[2] as f32);
    let (hit, y_intersect) = tri_chk_line_seg_para_y_intersect(
        v0,
        v1,
        v2,
        p.normal.x,
        p.normal.y,
        p.normal.z,
        p.origin_dist,
        pz,
        px,
        cyl_bottom,
        cyl_top,
    );
    if hit {
        let center = Vec3::new(px, y_intersect, pz);
        let mid = Vec3::new((v0.x + v1.x) * 0.5, (v0.y + v1.y) * 0.5, (v0.z + v1.z) * 0.5);
        let d = diff(mid, center);
        let dist = (d.x * d.x + d.z * d.z).sqrt();
        if is_zero(dist) {
            return (true, mid);
        }
        let ratio = cyl.radius as f32 / dist;
        return (true, point_on_infinite_line(center, d, ratio));
    }

    // The C stores the f32 top/bottom into `s16` sphere centers (truncation, then the halfword
    // store wraps).
    let top_sphere = Sphere16 { center: [cyl.pos[0], cyl_top as i32 as i16, cyl.pos[2]], radius: cyl.radius };
    let bottom_sphere = Sphere16 { center: [cyl.pos[0], cyl_bottom as i32 as i16, cyl.pos[2]], radius: cyl.radius };
    let (hit, i) = tri_vs_sph_intersect(&top_sphere, tri);
    if hit {
        return (true, i);
    }
    let (hit, i) = tri_vs_sph_intersect(&bottom_sphere, tri);
    if hit {
        return (true, i);
    }
    (false, Vec3::default())
}

/// `Math3D_CylVsTri`: `Math3D_CylTriVsIntersect` without the point.
pub fn cyl_vs_tri(cyl: &Cylinder16, tri: &TriNorm) -> bool {
    cyl_tri_vs_intersect(cyl, tri).0
}

/// `Math3D_SphVsSph`: `Math3D_SphVsSphOverlap` without the overlap.
pub fn sph_vs_sph(sphere_a: &Sphere16, sphere_b: &Sphere16) -> bool {
    sph_vs_sph_overlap(sphere_a, sphere_b).0
}

/// `Math3D_SphVsSphOverlap`: whether the spheres overlap, and by how much.
pub fn sph_vs_sph_overlap(sphere_a: &Sphere16, sphere_b: &Sphere16) -> (bool, f32) {
    let (hit, overlap, _) = sph_vs_sph_overlap_center(sphere_a, sphere_b);
    (hit, overlap)
}

/// `Math3D_SphVsSphOverlapCenterDist`: whether the spheres overlap by more than 0.008 (the C's
/// literal), the overlap (`rA + rB - dist`, 0 on a miss) and the center distance.
pub fn sph_vs_sph_overlap_center(sphere_a: &Sphere16, sphere_b: &Sphere16) -> (bool, f32, f32) {
    let dx = sphere_a.center[0] as f32 - sphere_b.center[0] as f32;
    let dy = sphere_a.center[1] as f32 - sphere_b.center[1] as f32;
    let dz = sphere_a.center[2] as f32 - sphere_b.center[2] as f32;
    let center_dist = (dx * dx + dy * dy + dz * dz).sqrt();
    let overlap = (sphere_a.radius as f32 + sphere_b.radius as f32) - center_dist;
    if overlap > 0.008 {
        return (true, overlap, center_dist);
    }
    (false, 0.0, center_dist)
}

/// `Math3D_SphVsCylOverlap`: `Math3D_SphVsCylOverlapCenterDist` without the distance.
pub fn sph_vs_cyl_overlap_dist(sph: &Sphere16, cyl: &Cylinder16) -> (bool, f32) {
    let (hit, overlap, _) = sph_vs_cyl_overlap_center_dist(sph, cyl);
    (hit, overlap)
}

/// `Math3D_SphVsCylOverlapCenterDist`: whether the sphere touches the cylinder (xz circles
/// within the combined radius, y ranges touching), the xz overlap `rS + rC - xzDist` and the
/// xz center distance. False with both outs 0 if either radius is <= 0; the distance is
/// written as soon as both radii are positive.
pub fn sph_vs_cyl_overlap_center_dist(sph: &Sphere16, cyl: &Cylinder16) -> (bool, f32, f32) {
    if sph.radius <= 0 || cyl.radius <= 0 {
        return (false, 0.0, 0.0);
    }
    let sph_center_y = sph.center[1] as f32;
    let sph_radius = sph.radius as f32;
    let cyl_pos_y = cyl.pos[1] as f32;
    let cyl_y_shift = cyl.y_shift as f32;
    let cyl_height = cyl.height as f32;
    let x = sph.center[0] as f32 - cyl.pos[0] as f32;
    let z = sph.center[2] as f32 - cyl.pos[2] as f32;
    let combined_radius = sph.radius as f32 + cyl.radius as f32;
    let center_dist = (x * x + z * z).sqrt();
    if combined_radius < center_dist {
        return (false, 0.0, center_dist);
    }
    let cyl_bottom = cyl_pos_y + cyl_y_shift;
    let cyl_top = cyl_bottom + cyl_height;
    let sph_bottom = sph_center_y - sph_radius;
    let sph_top = sph_center_y + sph_radius;
    if sph_top >= cyl_bottom && sph_bottom <= cyl_top {
        return (true, combined_radius - center_dist, center_dist);
    }
    (false, 0.0, center_dist)
}

/// `Math3D_CylVsCylOverlap`: `Math3D_CylVsCylOverlapCenterDist` without the distance.
pub fn cyl_outside_cyl(ca: &Cylinder16, cb: &Cylinder16) -> (bool, f32) {
    let (hit, dead_space, _) = cyl_outside_cyl_dist(ca, cb);
    (hit, dead_space)
}

/// `Math3D_CylVsCylOverlapCenterDist`: despite the name, whether the cylinders overlap (xz circles
/// within the combined radius, y ranges touching); `deadSpace` is `rA + rB - xzDist` (0 on a
/// miss) and `xzDist` the xz center distance (always written).
pub fn cyl_outside_cyl_dist(ca: &Cylinder16, cb: &Cylinder16) -> (bool, f32, f32) {
    let (ap, bp) = (s16_to_f(ca.pos), s16_to_f(cb.pos));
    let (ar, ays, ah) = (ca.radius as f32, ca.y_shift as f32, ca.height as f32);
    let (br, bys, bh) = (cb.radius as f32, cb.y_shift as f32, cb.height as f32);
    let xz_dist = ((ap.x - bp.x) * (ap.x - bp.x) + (ap.z - bp.z) * (ap.z - bp.z)).sqrt();
    if (ar + br) < xz_dist {
        return (false, 0.0, xz_dist);
    }
    if ((ap.y + ays) + ah) < (bp.y + bys) || ((bp.y + bys) + bh) < (ap.y + ays) {
        return (false, 0.0, xz_dist);
    }
    (true, ar + br - xz_dist, xz_dist)
}

/// `Math3D_TriVsTriIntersect`: whether the triangles intersect, and a point: the first of
/// `ta`'s edges (v0→v1, v1→v2, v2→v0) crossing `tb`, then `tb`'s edges crossing `ta`
/// (`Math3D_TriLineIntersect`, `fromFront` 0). After the plane rejections pass, the out
/// holds the last `Math3D_TriLineIntersect` write (`tb.vtx[0]` on a full miss), as in the C.
pub fn tri_vs_tri_intersect(ta: &TriNorm, tb: &TriNorm) -> (bool, Vec3) {
    let straddles = |p: &Plane, t: &TriNorm| {
        let d0 = plane(p, t.vtx[0]);
        let d1 = plane(p, t.vtx[1]);
        let d2 = plane(p, t.vtx[2]);
        !((d0 > 0.0 && d1 > 0.0 && d2 > 0.0) || (d0 < 0.0 && d1 < 0.0 && d2 < 0.0))
    };
    if !straddles(&ta.plane, tb) || !straddles(&tb.plane, ta) {
        return (false, Vec3::default());
    }
    let mut intersect = Vec3::default();
    for (tri, other) in [(tb, ta), (ta, tb)] {
        let [v0, v1, v2] = tri.vtx;
        let n = tri.plane.normal;
        let [o0, o1, o2] = other.vtx;
        for (a, b) in [(o0, o1), (o1, o2), (o2, o0)] {
            let (hit, i) = tri_line_intersect(v0, v1, v2, n.x, n.y, n.z, tri.plane.origin_dist, a, b, false);
            intersect = i;
            if hit {
                return (true, intersect);
            }
        }
    }
    (false, intersect)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3::new(x, y, z)
    }

    fn sph(x: i16, y: i16, z: i16, r: i16) -> Sphere16 {
        Sphere16 { center: [x, y, z], radius: r }
    }

    fn cyl(r: i16, h: i16, ys: i16, x: i16, y: i16, z: i16) -> Cylinder16 {
        Cylinder16 { radius: r, height: h, y_shift: ys, pos: [x, y, z] }
    }

    /// Floor triangle (0,0,0), (0,0,100), (100,0,0): (vb-va)×(vc-va) = (0,0,100)×(100,0,0)
    /// = (0, 10000, 0), so the normal is +Y and originDist is 0.
    fn floor_tri() -> TriNorm {
        tri_norm(v(0.0, 0.0, 0.0), v(0.0, 0.0, 100.0), v(100.0, 0.0, 0.0))
    }

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).abs().max_element() < 1e-4
    }

    #[test]
    fn def_plane_and_tri_norm() {
        let t = floor_tri();
        assert_eq!(t.plane.normal, v(0.0, 1.0, 0.0));
        assert_eq!(t.plane.origin_dist, 0.0);
        assert_eq!(t.vtx[1], v(0.0, 0.0, 100.0));
        // Raised to y=10: same normal, originDist = -(1*10) = -10.
        let (n, d) = def_plane(v(0.0, 10.0, 0.0), v(0.0, 10.0, 1.0), v(1.0, 10.0, 0.0));
        assert_eq!((n, d), (v(0.0, 1.0, 0.0), -10.0));
        // 45° slope: (0,0,1)×(1,1,0) = (-1, 1, 0), magnitude sqrtf(2), scaled by 1/sqrtf(2).
        let (n, d) = def_plane(v(0.0, 0.0, 0.0), v(0.0, 0.0, 1.0), v(1.0, 1.0, 0.0));
        let inv = 1.0 / 2.0f32.sqrt();
        assert_eq!(n, v(-inv, inv, 0.0));
        assert_eq!(d, 0.0);
        // Collinear points: IS_ZERO magnitude, everything 0.
        assert_eq!(def_plane(v(0.0, 0.0, 0.0), v(1.0, 0.0, 0.0), v(2.0, 0.0, 0.0)), (Vec3::ZERO, 0.0));
    }

    #[test]
    fn small_helpers() {
        // (1,2,3)-(4,6,3) = (-3,-4,0): 9 + 16 = 25.
        assert_eq!(vec3f_dist_sq(v(1.0, 2.0, 3.0), v(4.0, 6.0, 3.0)), 25.0);
        assert_eq!(vec3f_magnitude_sq(v(1.0, 2.0, 2.0)), 9.0);
        assert_eq!(vec3f_magnitude(v(1.0, 2.0, 2.0)), 3.0);
        assert_eq!(vec3f_dist_xyz(v(0.0, 0.0, 0.0), v(3.0, 0.0, 4.0)), 5.0);
        assert_eq!(dist_xyz16_to_f([3, 0, 4], v(0.0, 0.0, 0.0)), 5.0);
        assert_eq!(vec3f_cross(v(1.0, 0.0, 0.0), v(0.0, 1.0, 0.0)), v(0.0, 0.0, 1.0));
        assert_eq!(surface_norm(v(0.0, 0.0, 0.0), v(0.0, 0.0, 100.0), v(100.0, 0.0, 0.0)), v(0.0, 10000.0, 0.0));
        // 2*1 + 3*2 + 4*3 + 5 = 25.
        assert_eq!(planef(2.0, 3.0, 4.0, 5.0, v(1.0, 2.0, 3.0)), 25.0);
        assert_eq!(plane(&Plane { normal: v(2.0, 3.0, 4.0), origin_dist: 5.0 }, v(1.0, 2.0, 3.0)), 25.0);
        // (1,1,1) + (2,0,-2)*1.5 = (4, 1, -2).
        assert_eq!(point_on_infinite_line(v(1.0, 1.0, 1.0), v(2.0, 0.0, -2.0), 1.5), v(4.0, 1.0, -2.0));
        // (0,0,0) + ((8,4,0)-(0,0,0))*0.25 = (2,1,0).
        assert_eq!(line_split_ratio(v(0.0, 0.0, 0.0), v(8.0, 4.0, 0.0), 0.25), v(2.0, 1.0, 0.0));
        // Box x,z in [0,100], y in [0,0]; grown by 10.
        let [a, b, c] = floor_tri().vtx;
        assert!(sph_cube_vs_tri_cube(a, b, c, v(-10.0, 10.0, 50.0), 10.0));
        assert!(!sph_cube_vs_tri_cube(a, b, c, v(50.0, 10.5, 50.0), 10.0));
    }

    #[test]
    fn tri_chk_point_para_variants() {
        let t = floor_tri();
        let [a, b, c] = t.vtx;
        // ParaY at (z,x)=(20,20): projected vertices (z,x) = (0,0), (100,0), (0,100); the
        // determinants are 2000, 6000, 2000, all >= -300 -> inside.
        assert!(tri_chk_point_para_y(a, b, c, 1.0, 20.0, 20.0));
        // (80,80): determinants 8000, -6000, 8000 (mixed), and the v1-v2 edge is ~42 away.
        assert!(!tri_chk_point_para_y(a, b, c, 1.0, 80.0, 80.0));
        // IS_ZERO(ny) is rejected outright.
        assert!(!tri_chk_point_para_y(a, b, c, 0.007, 20.0, 20.0));
        assert!(tri_chk_point_para_y_determinate(a, b, c, 20.0, 20.0, 0.0, 1.0));
        // Wall in the plane x=0: (y,z) = (-20,-10), (20,-10), (0,20). At (0,0) the
        // determinants are 400, 400, 400.
        let w = [v(0.0, -20.0, -10.0), v(0.0, 20.0, -10.0), v(0.0, 0.0, 20.0)];
        assert!(tri_chk_point_para_x(w[0], w[1], w[2], 1.0, 0.0, 0.0));
        assert!(!tri_chk_point_para_x(w[0], w[1], w[2], 1.0, 0.0, 50.0));
        assert!(!tri_chk_point_para_x(w[0], w[1], w[2], 0.0, 0.0, 0.0));
        // detMax 0: 400s are all >= -0 -> inside.
        assert!(tri_chk_point_para_x_determinate(w[0], w[1], w[2], 0.0, 0.0, 0.0, 1.0));
        // Wall in the plane z=0 with (x,y) = (-20,-10), (20,-10), (0,20): same numbers.
        let w = [v(-20.0, -10.0, 0.0), v(20.0, -10.0, 0.0), v(0.0, 20.0, 0.0)];
        assert!(tri_chk_point_para_z(w[0], w[1], w[2], 1.0, 0.0, 0.0));
        assert!(!tri_chk_point_para_z(w[0], w[1], w[2], 1.0, 50.0, 0.0));
        assert!(tri_chk_point_para_z_determinate(w[0], w[1], w[2], 0.0, 0.0, 0.0, 1.0));
    }

    #[test]
    fn line_seg_para_y_intersect() {
        let t = floor_tri();
        let [a, b, c] = t.vtx;
        let n = t.plane.normal;
        // y from 10 to -10 at (x,z)=(20,20): plane values 10 and -10 straddle; inside (see
        // above); y = ((-0*20 - 0*20) - 0) / 1 = 0.
        let (hit, y) = tri_chk_line_seg_para_y_intersect(a, b, c, n.x, n.y, n.z, t.plane.origin_dist, 20.0, 20.0, 10.0, -10.0);
        assert!(hit);
        assert_eq!(y, 0.0);
        // Both ends above.
        let r = tri_chk_line_seg_para_y_intersect(a, b, c, n.x, n.y, n.z, t.plane.origin_dist, 20.0, 20.0, 10.0, 5.0);
        assert_eq!(r, (false, 0.0));
    }

    #[test]
    fn line_seg_vs_plane_cases() {
        // Plane y=0; a=(0,10,0) (dist 10), b=(0,-30,0) (dist -30): ratio 10/(10-(-30)) = 0.25,
        // a + (b-a)*0.25 = (0, 10 - 40*0.25, 0) = origin.
        let (a, b) = (v(0.0, 10.0, 0.0), v(0.0, -30.0, 0.0));
        assert_eq!(line_seg_vs_plane(0.0, 1.0, 0.0, 0.0, a, b, false), (true, v(0.0, 0.0, 0.0)));
        // Same with from_front: a is in front, so it still counts.
        assert!(line_seg_vs_plane(0.0, 1.0, 0.0, 0.0, a, b, true).0);
        // Back to front with from_front: rejected, out = b.
        assert_eq!(line_seg_vs_plane(0.0, 1.0, 0.0, 0.0, b, a, true), (false, a));
        // Both on the same side: product > 0, out = b.
        let c = v(0.0, 5.0, 0.0);
        assert_eq!(line_seg_vs_plane(0.0, 1.0, 0.0, 0.0, a, c, false), (false, c));
        // Both on the plane: dist diff IS_ZERO -> false, out = b.
        let (p, q) = (v(0.0, 0.0, 0.0), v(5.0, 0.0, 0.0));
        assert_eq!(line_seg_vs_plane(0.0, 1.0, 0.0, 0.0, p, q, false), (false, q));
        // Direct: a exactly on the plane wins.
        assert_eq!(line_seg_find_plane_intersect(0.0, -3.0, p, a), (true, p));
    }

    #[test]
    fn tri_line_intersect_cases() {
        let t = floor_tri();
        let [v0, v1, v2] = t.vtx;
        let n = t.plane.normal;
        let d = t.plane.origin_dist;
        // (20,10,20)->(20,-10,20) crosses y=0 at ratio 0.5 = (20,0,20); nx, nz are exactly 0
        // and ParaY(z=20, x=20) is inside.
        let (a, b) = (v(20.0, 10.0, 20.0), v(20.0, -10.0, 20.0));
        assert_eq!(tri_line_intersect(v0, v1, v2, n.x, n.y, n.z, d, a, b, false), (true, v(20.0, 0.0, 20.0)));
        // At (80, 80) the plane is crossed outside the triangle: false, out = b.
        let (a, b) = (v(80.0, 10.0, 80.0), v(80.0, -10.0, 80.0));
        assert_eq!(tri_line_intersect(v0, v1, v2, n.x, n.y, n.z, d, a, b, false), (false, b));
        // @bug (game): tilt the floor slightly in x. (0,0,100)×(100,0.5,0) = (-50, 10000, 0),
        // so nx ≈ -0.005: not exactly 0, but IS_ZERO, so ParaX rejects every point.
        let s = tri_norm(v(0.0, 0.0, 0.0), v(0.0, 0.0, 100.0), v(100.0, 0.5, 0.0));
        let sn = s.plane.normal;
        assert!(sn.x != 0.0 && is_zero(sn.x));
        let (a, b) = (v(20.0, 10.0, 20.0), v(20.0, -10.0, 20.0));
        let [s0, s1, s2] = s.vtx;
        assert!(!tri_line_intersect(s0, s1, s2, sn.x, sn.y, sn.z, s.plane.origin_dist, a, b, false).0);
    }

    #[test]
    fn point_in_sph_and_cyl() {
        let s = sph(10, 0, 0, 5);
        assert!(point_in_sph(&s, v(13.0, 0.0, 0.0))); // distance 3 < 5
        assert!(!point_in_sph(&s, v(15.0, 0.0, 0.0))); // distance 5 is not < 5
        // Bottom = 100 + 5 = 105, top = 20 + 105 = 125, radius² = 100 (all strict).
        let c = cyl(10, 20, 5, 0, 100, 0);
        assert!(point_in_cyl(&c, v(6.0, 110.0, 7.0))); // 36 + 49 = 85 < 100
        assert!(!point_in_cyl(&c, v(6.0, 110.0, 8.0))); // 36 + 64 = 100
        assert!(!point_in_cyl(&c, v(0.0, 105.0, 0.0))); // on the bottom
        assert!(point_in_cyl(&c, v(0.0, 124.0, 0.0)));
        assert!(!point_in_cyl(&c, v(0.0, 125.0, 0.0))); // on the top
    }

    #[test]
    fn line_vs_sph_cases() {
        let s = sph(0, 0, 0, 10);
        // (-20,5,0)->(20,5,0): ends outside; t = (20*40)/1600 = 0.5, foot (0,5,0), 25 <= 100.
        assert!(line_vs_sph(&s, &Linef { a: v(-20.0, 5.0, 0.0), b: v(20.0, 5.0, 0.0) }));
        // At y=11 the foot is 121 > 100 away.
        assert!(!line_vs_sph(&s, &Linef { a: v(-20.0, 11.0, 0.0), b: v(20.0, 11.0, 0.0) }));
        // (15,0,0)->(30,0,0): t = (-15*15)/225 = -1 < 0.
        assert!(!line_vs_sph(&s, &Linef { a: v(15.0, 0.0, 0.0), b: v(30.0, 0.0, 0.0) }));
        // An end strictly inside.
        assert!(line_vs_sph(&s, &Linef { a: v(5.0, 0.0, 0.0), b: v(30.0, 0.0, 0.0) }));
    }

    #[test]
    fn sph_vs_tri() {
        let t = floor_tri();
        let s = sph(20, 5, 20, 10);
        // Box check passes; plane distance |5| <= 10. Edges: v0-v1 foot (0,0,20) is
        // 400+25 = 425 > 100 away; v1-v2 foot (50,0,50): 900+25+900; v2-v0 foot (20,0,0): 425.
        // Projected center (20,0,20) is inside (determinants 2000, 6000, 2000 with detMax 0).
        // Point: v0-v1 midpoint (0,0,50), center (20,5,20), dist sqrtf(400+25+900),
        // center + (mid - center) * (10/dist).
        let r = 10.0f32 / 1325.0f32.sqrt();
        let expect = v(20.0 + (-20.0 * r), 5.0 + (-5.0 * r), 20.0 + (30.0 * r));
        assert_eq!(get_sph_vs_tri_intersect_point(&s, &t), expect);
        assert_eq!(tri_vs_sph_intersect(&s, &t), (true, expect));
        // 15 above the plane: plane distance 15 > radius 10.
        assert_eq!(tri_vs_sph_intersect(&sph(20, 15, 20, 10), &t), (false, Vec3::ZERO));
        // Far away in the plane: box check fails.
        assert!(!tri_vs_sph_intersect(&sph(200, 5, 200, 10), &t).0);
        // Center on the v0-v1 midpoint: the point is the center itself.
        assert_eq!(get_sph_vs_tri_intersect_point(&sph(0, 0, 50, 10), &t), v(0.0, 0.0, 50.0));
    }

    #[test]
    fn cyl_vs_line_seg_cases() {
        let c = cyl(10, 20, 0, 0, 0, 0);
        // (-20,10,0)->(20,10,0): ab=(40,0,0), caps skipped (ab.y = 0). radSqDiff = 400-100 =
        // 300, dot2AB = 2*(40*-20) = -1600, 1600² - 4*1600*300 = 640000 > 0, sqrt = 800.
        // fracA = (800+1600)/3200 = 0.75 -> (10,10,0); fracB = (1600-800)/3200 = 0.25 ->
        // (-10,10,0). Loop: A=(10,10,0); |A-a|² = 900 is not < |A-P|² = 400, so they swap.
        // @bug (game): count stays 1 because of the `break` before `count++`.
        let r = cyl_vs_line_seg(&c, v(-20.0, 10.0, 0.0), v(20.0, 10.0, 0.0));
        assert_eq!(r, (1, v(-10.0, 10.0, 0.0), v(10.0, 10.0, 0.0)));
        // Both ends inside: 2 with the ends.
        let (a, b) = (v(1.0, 5.0, 0.0), v(-1.0, 15.0, 2.0));
        assert_eq!(cyl_vs_line_seg(&c, a, b), (2, a, b));
        // At z=15: radSqDiff = 400+225-100 = 525, 1600² < 4*1600*525 -> 0.
        assert_eq!(cyl_vs_line_seg(&c, v(-20.0, 10.0, 15.0), v(20.0, 10.0, 15.0)).0, 0);
        // @bug (game): a vertical segment through both caps (ab.x = ab.z = 0) hits the base and
        // top, but SQXZ and DOTXZ are 0 so it returns 0.
        assert_eq!(cyl_vs_line_seg(&c, v(0.0, -10.0, 0.0), v(0.0, 30.0, 0.0)).0, 0);
        // @bug (game): (0,30,0)->(5,10,0) enters through the top (fracA 0.5) and ends inside,
        // but the wall roots are fracs ±2 (dot2AB 0, sqrt(0 + 4*25*100) = 100, /50), both
        // beyond the segment -> 0.
        assert_eq!(cyl_vs_line_seg(&c, v(0.0, 30.0, 0.0), v(5.0, 10.0, 0.0)).0, 0);
    }

    #[test]
    fn cyl_vs_tri_cases() {
        // Floor quad half (-100,0,-100), (-100,0,100), (100,0,0): (0,0,200)×(200,0,100) =
        // (0, 40000, 0), normal +Y.
        let t = tri_norm(v(-100.0, 0.0, -100.0), v(-100.0, 0.0, 100.0), v(100.0, 0.0, 0.0));
        assert_eq!(t.plane.normal, v(0.0, 1.0, 0.0));

        // Cylinder r10 h50 standing at (30,0,0): every edge's line is >= 31 away in xz, so
        // CylVsLineSeg misses them all (e.g. v0->v1: 1.6e9 < 4*40000*(10000+10000-100)).
        // The axis y 0..50 crosses the plane (values 0 and 50) inside the triangle at y = 0.
        // v0-v1 midpoint (-100,0,0) minus center (30,0,0) = (-130,0,0), xz length 130, so
        // the point is center + (-130,0,0) * (10/130).
        let c = cyl(10, 50, 0, 30, 0, 0);
        let r = 10.0f32 / 130.0;
        assert_eq!(cyl_tri_vs_intersect(&c, &t), (true, v(-130.0 * r + 30.0, 0.0, 0.0)));
        assert!(cyl_vs_tri(&c, &t));

        // Cylinder on the v0-v1 edge (x=-100): cylToA = (0,0,-100), ab = (0,0,200),
        // radSqDiff 9900, dot2AB -40000, 1.6e9 - 1.584e9 = 1.6e7, sqrt 4000;
        // fracA = 44000/80000 = 0.55 -> z 10, fracB = 36000/80000 = 0.45 -> z -10.
        // Ordering (bug above) puts (-100,0,-10) first; its distance² to v0 is 90² = 8100.
        // The other two edges are ~89 away.
        let c = cyl(10, 50, 0, -100, 0, 0);
        let (hit, p) = cyl_tri_vs_intersect(&c, &t);
        assert!(hit);
        assert!(close(p, v(-100.0, 0.0, -10.0)), "{p}");

        // Hovering: bottom 100 above every vertex.
        assert_eq!(cyl_tri_vs_intersect(&cyl(10, 50, 0, 30, 100, 0), &t), (false, Vec3::ZERO));
        assert!(!cyl_vs_tri(&cyl(10, 50, 0, 30, 100, 0), &t));
        // Below: top = -60 + 50 = -10 < 0.
        assert!(!cyl_vs_tri(&cyl(10, 50, 0, 30, -60, 0), &t));
        // Outside in xz (x = 300): nothing but the sphere boxes, which miss too.
        assert!(!cyl_vs_tri(&cyl(10, 50, 0, 300, 0, 0), &t));
    }

    #[test]
    fn sph_vs_sph_cases() {
        let a = sph(0, 0, 0, 10);
        // Distance 15, overlap (10 + 10) - 15 = 5.
        assert_eq!(sph_vs_sph_overlap_center(&a, &sph(15, 0, 0, 10)), (true, 5.0, 15.0));
        assert_eq!(sph_vs_sph_overlap(&a, &sph(15, 0, 0, 10)), (true, 5.0));
        assert!(sph_vs_sph(&a, &sph(15, 0, 0, 10)));
        // Distance 5 on a 3-4-0 triangle: overlap 20 - 5 = 15.
        assert_eq!(sph_vs_sph_overlap_center(&a, &sph(3, 4, 0, 10)), (true, 15.0, 5.0));
        // Just touching: overlap 0 is not > 0.008 -> 0 overlap, distance still written.
        assert_eq!(sph_vs_sph_overlap_center(&a, &sph(20, 0, 0, 10)), (false, 0.0, 20.0));
        assert!(!sph_vs_sph(&a, &sph(30, 0, 0, 10)));
    }

    #[test]
    fn sph_vs_cyl_cases() {
        let c = cyl(20, 40, 0, 25, 0, 0);
        // xz distance 25 <= 30; sphere y 40..60 touches cylinder y 0..40; overlap 30 - 25 = 5.
        assert_eq!(sph_vs_cyl_overlap_center_dist(&sph(0, 50, 0, 10), &c), (true, 5.0, 25.0));
        assert_eq!(sph_vs_cyl_overlap_dist(&sph(0, 50, 0, 10), &c), (true, 5.0));
        // Sphere bottom 41 > top 40: miss, distance written.
        assert_eq!(sph_vs_cyl_overlap_center_dist(&sph(0, 51, 0, 10), &c), (false, 0.0, 25.0));
        // xz distance 35 > 30.
        assert_eq!(sph_vs_cyl_overlap_center_dist(&sph(-10, 20, 0, 10), &c), (false, 0.0, 35.0));
        // Zero radius: nothing written.
        assert_eq!(sph_vs_cyl_overlap_center_dist(&sph(25, 20, 0, 0), &c), (false, 0.0, 0.0));
    }

    #[test]
    fn cyl_outside_cyl_cases() {
        let a = cyl(10, 50, 0, 0, 0, 0);
        // xz distance 20 <= 25; A spans y 0..50, B spans (40-10)..(30+20) = 30..50.
        let b = cyl(15, 20, -10, 0, 40, 20);
        assert_eq!(cyl_outside_cyl_dist(&a, &b), (true, 5.0, 20.0));
        assert_eq!(cyl_outside_cyl(&a, &b), (true, 5.0));
        // B raised to 60..80: above A's top.
        assert_eq!(cyl_outside_cyl_dist(&a, &cyl(15, 20, -10, 0, 70, 20)), (false, 0.0, 20.0));
        // xz distance 30 > 25.
        assert_eq!(cyl_outside_cyl_dist(&a, &cyl(15, 20, -10, 0, 40, 30)), (false, 0.0, 30.0));
    }

    #[test]
    fn tri_vs_tri_cases() {
        // Floor quad (-50..50)² as two +Y triangles (normals checked like floor_tri).
        let q1 = tri_norm(v(-50.0, 0.0, -50.0), v(-50.0, 0.0, 50.0), v(50.0, 0.0, 50.0));
        let q2 = tri_norm(v(-50.0, 0.0, -50.0), v(50.0, 0.0, 50.0), v(50.0, 0.0, -50.0));
        assert_eq!(q1.plane.normal, v(0.0, 1.0, 0.0));
        assert_eq!(q2.plane.normal, v(0.0, 1.0, 0.0));
        // Wall in x=0 with (y,z) = (-20,-10), (20,-10), (0,20): (0,40,0)×(0,20,30) =
        // (1200,0,0), normal +X.
        let w = tri_norm(v(0.0, -20.0, -10.0), v(0.0, 20.0, -10.0), v(0.0, 0.0, 20.0));
        assert_eq!(w.plane.normal, v(1.0, 0.0, 0.0));

        // q1 vs wall: wall y values -20, 20, 0 straddle the floor; q1 x values -50, -50, 50
        // straddle the wall. q1 v0->v1 stays at x=-50; v1->v2 crosses x=0 at ratio 0.5 =
        // (0,0,50), outside the wall (z 50 > 20); v2->v0 crosses at (0,0,0), inside the wall
        // (determinants 400, 400, 400).
        assert_eq!(tri_vs_tri_intersect(&q1, &w), (true, v(0.0, 0.0, 0.0)));
        // q2 v0->v1 is the diagonal, crossing x=0 at ratio 0.5 = (0,0,0).
        assert_eq!(tri_vs_tri_intersect(&q2, &w), (true, v(0.0, 0.0, 0.0)));

        // Wall moved to x=60: all q1 vertices are behind it.
        let far = tri_norm(v(60.0, -20.0, -10.0), v(60.0, 20.0, -10.0), v(60.0, 0.0, 20.0));
        assert_eq!(tri_vs_tri_intersect(&q1, &far), (false, Vec3::ZERO));
        // Wall lifted to y 10..50: all its vertices are above the floor.
        let high = tri_norm(v(0.0, 10.0, -10.0), v(0.0, 50.0, -10.0), v(0.0, 30.0, 20.0));
        assert!(!tri_vs_tri_intersect(&q1, &high).0);
        // Planes straddle but the triangles miss: wall at z 60..90 (off the quad's z range).
        // Every edge test fails; the out is the last TriLineIntersect write, tb.vtx[0].
        let off = tri_norm(v(0.0, -20.0, 60.0), v(0.0, 20.0, 60.0), v(0.0, 0.0, 90.0));
        assert_eq!(tri_vs_tri_intersect(&q1, &off), (false, off.vtx[0]));
    }
}
