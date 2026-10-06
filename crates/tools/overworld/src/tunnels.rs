//! Tunnels (lines of kind "tunnel"): a passage Link walks through, from a mouth in one wall to a
//! mouth in another: through a ridge, under a plateau, or from one area's edge of the world to
//! another's (an area being a region drawn outside the outline, with an edge of its own).
//!
//! - **Mouths.** The line is drawn from a floor, through the wall it goes into, to a floor beyond
//!   (the curve through its nodes, as a dirt path's). From each end, walking along it over the
//!   ground, the mouth is where it first meets a wall taller than the tunnel (or the edge of the
//!   world, past which there's no ground at all). Lower walls on the way are climbed: they're just
//!   ground. The mouth goes on that wall's face, facing square out of it, and slides along it clear
//!   of corners as an opening does (`openings::seat`); the wall may bend up to 40 off flat across
//!   it. It goes in within 60 degrees of square on.
//! - **The bore.** It runs straight in from each mouth (the walls' normals), then through the
//!   line's nodes between the mouths, as one smooth curve. Its cross-section is upright: a floor
//!   `width` across, walls rising to an arch `height` above the floor. The floor runs from one
//!   mouth's floor to the other's, with any node's own height (its third value) on the way.
//! - **The gaps.** Each mouth's wall is cut to the bore's cross-section there
//!   (`openings::punch_with`), so the wall meets the bore edge for edge: the whole is watertight.
//! - **Rough walls** (the line's `noise`): the walls and roof push in and out by up to `amplitude`,
//!   the passage wanders from side to side by half that, and the floor rolls up to a third of it,
//!   in features about `scale` across, all fading to nothing within `edge` of the mouths.
//! - **Room.** Ground over the roof by at least `COVER` all along (or none: past the edge of the
//!   world), or it's reported where it comes out. Turns too tight for its width are refused.
//!
//! Built on the finished ground (after the painted terrain), before props. Floors are the theme's
//! tunnel floor (Kokiri: the ground's grass), walls and roof walls with no ledge to grab. The light falls off inside
//! (`TunnelStyle::dark`), baked with the rest.

use crate::doc::{Line, Noise};
use crate::geom::*;
use crate::mesh::Mesh;
use crate::noise::relief;
use crate::openings::{nearest_wall_facing, punch_with, seat, survey, Seat};
use crate::paths::centre_line;
use crate::props::Ground;
use crate::theme::{Theme, TunnelStyle};

/// Solid ground over the roof, at least.
pub const COVER: f64 = 20.0;
/// The most a tunnel may go into its wall away from square on, in degrees.
pub const MAX_SKEW: f64 = 60.0;
/// How far a mouth's wall may stray from flat across it.
const BEND: f64 = 40.0;
/// Width and height limits.
pub const WIDTH: (f64, f64) = (60.0, 2000.0);
pub const HEIGHT: (f64, f64) = (60.0, 1000.0);

/// A built tunnel's middle line, for its lighting: each station's point (half way up) and how far
/// it is from the nearer mouth.
pub struct Bore {
    pub st: Vec<(P3, f64)>,
    /// The floor under the middle line.
    pub floor: Vec<P3>,
}

/// A wall the tunnel goes into.
struct Mouth {
    obj: usize,
    base: P2,
    /// Out of the wall, over the floor in front.
    n: P2,
    /// How far along the drawn line it is.
    at: f64,
}

/// A point of the cross-section: across (positive to the left, looking along the tunnel) and up
/// from the floor, its outward normal, and how far round the section it is.
#[derive(Clone, Copy)]
struct Pt {
    a: f64,
    b: f64,
    nrm: P2,
    floor: bool,
    wall: bool,
    arc: f64,
}

fn unit(v: P2) -> P2 {
    let l = v[0].hypot(v[1]).max(1e-12);
    [v[0] / l, v[1] / l]
}

fn dot(a: P2, b: P2) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

fn point_at(line: &[(P2, f64)], s: f64) -> P2 {
    for w in line.windows(2) {
        if s <= w[1].1 {
            let t = if w[1].1 - w[0].1 > 1e-12 { (s - w[0].1) / (w[1].1 - w[0].1) } else { 0.0 };
            return lerp(w[0].0, w[1].0, t.clamp(0.0, 1.0));
        }
    }
    line[line.len() - 1].0
}

/// The cross-section, counter-clockwise from the right floor corner: the floor across to the
/// left corner, up the left wall, over the arch and down the right wall. `fine` 0 (high detail)
/// to 2 (low); rough walls get more points up the walls.
fn profile(w: f64, h: f64, fine: u8, noise: Option<&Noise>) -> Vec<Pt> {
    let (floor_n, arch_n) = match fine {
        0 => (4, 12),
        1 => (2, 8),
        _ => (2, 6),
    };
    let spring = (h - w / 2.0).max(0.3 * h);
    let rise = h - spring;
    let wall_n = noise.map_or(1, |n| ((spring / (n.scale / 4.0).max(10.0)).ceil() as usize).clamp(1, 6));
    let mut pts = vec![];
    for k in 0..floor_n {
        let a = -w / 2.0 + w * k as f64 / floor_n as f64;
        pts.push(Pt { a, b: 0.0, nrm: [0.0, -1.0], floor: true, wall: k == 0, arc: 0.0 });
    }
    // the left corner, then up the wall
    for k in 0..wall_n {
        let b = spring * k as f64 / wall_n as f64;
        pts.push(Pt { a: w / 2.0, b, nrm: [1.0, 0.0], floor: k == 0, wall: true, arc: 0.0 });
    }
    // the arch, from the left spring (theta 0) to the right one (pi)
    for k in 0..arch_n {
        let th = std::f64::consts::PI * k as f64 / arch_n as f64;
        let nrm = unit([th.cos() / (w / 2.0), th.sin() / rise.max(1e-6)]);
        pts.push(Pt { a: w / 2.0 * th.cos(), b: spring + rise * th.sin(), nrm, floor: false, wall: true, arc: 0.0 });
    }
    // down the right wall to just above its corner (the corner is the first point)
    for k in 0..wall_n {
        let b = spring * (1.0 - k as f64 / wall_n as f64);
        pts.push(Pt { a: -w / 2.0, b, nrm: [-1.0, 0.0], floor: false, wall: true, arc: 0.0 });
    }
    // the right corner is the first point, closing the section; how far round each point is
    let mut arc = 0.0;
    for i in 1..pts.len() {
        arc += (pts[i].a - pts[i - 1].a).hypot(pts[i].b - pts[i - 1].b);
        pts[i].arc = arc;
    }
    pts
}

/// Builds the tunnel along `line` into `mesh` (its object "tunnels"), cutting its mouths into the
/// walls it goes through. `fine` is the detail (0 high, 1 medium, 2 low), `step` the spacing of
/// its cross-sections. Returns its middle line and the problems that don't stop it being built.
#[allow(clippy::too_many_arguments)]
pub fn build(line: &Line, th: &TunnelStyle, theme: &Theme, sampling: Sampling, fine: u8, step: f64, max_slope: f64, seed: u32, mesh: &mut Mesh, ground: &Ground) -> Result<(Bore, Vec<String>), String> {
    let w = line.width.unwrap_or(th.width);
    let h = line.height.unwrap_or(th.height);
    if !(WIDTH.0..=WIDTH.1).contains(&w) || !(HEIGHT.0..=HEIGHT.1).contains(&h) {
        return Err(format!("it's {w:.0} wide and {h:.0} tall; it can be {:.0} to {:.0} wide and {:.0} to {:.0} tall", WIDTH.0, WIDTH.1, HEIGHT.0, HEIGHT.1));
    }
    let noise = line.noise.as_ref().filter(|n| n.amplitude != 0.0);
    if noise.is_some_and(|n| n.scale <= 0.0) {
        return Err("its roughness needs a positive size".into());
    }
    let amp = noise.map_or(0.0, |n| n.amplitude.abs().min(w / 4.0));
    let xy: Vec<P2> = line.nodes.iter().filter(|n| n.len() >= 2).map(|n| [n[0], n[1]]).collect();
    if xy.len() < 2 {
        return Err("it needs two nodes".into());
    }
    let (drawn, node_s) = centre_line(&xy, sampling);
    let need = h + amp + COVER;
    let ma = find_mouth(mesh, ground, &drawn, true, need, w / 2.0 + amp + 20.0, h + amp)?;
    let mb = find_mouth(mesh, ground, &drawn, false, need, w / 2.0 + amp + 20.0, h + amp)?;
    if ma.at >= mb.at - 1.0 || dist(ma.base, mb.base) < 10.0 {
        return Err("both its ends go into the same wall: draw it from a floor, through the wall, to a floor beyond".into());
    }

    // the middle line: straight in from each mouth, then through the nodes between
    let depth = dist(ma.base, mb.base);
    let lead = (0.6 * w + 20.0).min(0.3 * depth);
    let into = |m: &Mouth| [m.base[0] - m.n[0] * lead, m.base[1] - m.n[1] * lead];
    let mut ctrl: Vec<(P2, Option<f64>)> = vec![(ma.base, None), (into(&ma), None)];
    for (j, &p) in xy.iter().enumerate().take(xy.len() - 1).skip(1) {
        if node_s[j] > ma.at && node_s[j] < mb.at && dist(p, ma.base) > 1.5 * lead && dist(p, mb.base) > 1.5 * lead {
            ctrl.push((p, line.nodes[j].get(2).copied()));
        }
    }
    ctrl.push((into(&mb), None));
    ctrl.push((mb.base, None));
    let cxy: Vec<P2> = ctrl.iter().map(|c| c.0).collect();
    let (cl, cs) = centre_line(&cxy, Sampling::Every(step.max(5.0)));
    let total = cl[cl.len() - 1].1;
    let k_last = cl.len() - 1;
    let dirs: Vec<P2> = (0..cl.len())
        .map(|k| match k {
            0 => [-ma.n[0], -ma.n[1]],
            _ if k == k_last => mb.n,
            _ => unit(sub(cl[k + 1].0, cl[k - 1].0)),
        })
        .collect();
    let left = |k: usize| [-dirs[k][1], dirs[k][0]];
    let half = w / 2.0 + amp;
    for k in 0..k_last {
        for side in [-1.0, 1.0] {
            let (l0, l1) = (left(k), left(k + 1));
            let q0 = [cl[k].0[0] + l0[0] * side * half, cl[k].0[1] + l0[1] * side * half];
            let q1 = [cl[k + 1].0[0] + l1[0] * side * half, cl[k + 1].0[1] + l1[1] * side * half];
            if dot(sub(q1, q0), dirs[k]) <= 0.0 {
                return Err(format!("it turns too tightly for its width near ({:.0}, {:.0})", cl[k].0[0], cl[k].0[1]));
            }
        }
    }
    // each mouth's sections stay behind its wall
    for (m, from_end) in [(&ma, false), (&mb, true)] {
        for j in 1..k_last {
            let k = if from_end { k_last - j } else { j };
            let s = if from_end { total - cl[k].1 } else { cl[k].1 };
            if s > 2.0 * w {
                break;
            }
            for side in [-1.0, 1.0] {
                let l = left(k);
                let q = [cl[k].0[0] + l[0] * side * half, cl[k].0[1] + l[1] * side * half];
                if dot(sub(q, m.base), m.n) > -0.5 {
                    return Err(format!("it turns too soon after its mouth at ({:.0}, {:.0}): give it room to go straight in", m.base[0], m.base[1]));
                }
            }
        }
    }

    // the cross-section and its roughness
    let prof = profile(w, h, fine, noise);
    let rough_seed = seed ^ noise.map_or(0, |n| n.seed.wrapping_mul(0x85EB_CA6B));
    let smooth = |x: f64| {
        let t = x.clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    let disp = |s: f64, pt: &Pt| -> (f64, f64) {
        let Some(n) = noise else { return (0.0, 0.0) };
        let amp = n.amplitude.signum() * amp;
        let from_mouth = s.min(total - s);
        let f = if n.edge > 0.0 { smooth(from_mouth / n.edge) } else { 1.0 };
        let wander = 0.5 * amp * f * relief([s / (2.0 * n.scale), 0.37], rough_seed ^ 0x5BD1);
        // the floor rolls up from the mouths' floors, so the way in is flush
        let lift = 0.3 * amp.abs() * smooth(from_mouth / n.edge.max(40.0)) * 0.5 * (1.0 + relief([s / n.scale, pt.a / n.scale + 50.0], rough_seed.wrapping_add(7)));
        let r = amp * f * relief([s / n.scale, pt.arc / n.scale], rough_seed);
        match (pt.floor, pt.wall) {
            (true, false) => (wander, lift),
            (true, true) => (wander + r * pt.nrm[0], lift),
            _ => (wander + r * pt.nrm[0], r * pt.nrm[1]),
        }
    };
    let mouth_ring = |s: f64, sign: f64, bottom: f64| -> Vec<P2> {
        prof.iter()
            .map(|pt| {
                let (da, db) = disp(s, pt);
                [sign * (pt.a + da), bottom + 1.0 + pt.b + db]
            })
            .collect()
    };
    let extent = |floor: f64| vec![[-half, floor - 10.0], [half, floor - 10.0], [half, floor + h + amp + 10.0], [-half, floor + h + amp + 10.0]];

    // the floor's height along it: the mouths' floors (a unit up, over the walls' feet), and the
    // nodes' own between, linear between them
    let heights = |za: f64, zb: f64| {
        let mut known: Vec<(f64, f64)> = vec![(0.0, za)];
        for (j, c) in ctrl.iter().enumerate().skip(1).take(ctrl.len() - 2) {
            if let Some(z) = c.1 {
                known.push((cs[j], z));
            }
        }
        known.push((total, zb));
        known
    };
    let floor_z = |known: &[(f64, f64)], s: f64| {
        for w in known.windows(2) {
            if s <= w[1].0 {
                let t = if w[1].0 - w[0].0 > 1e-9 { (s - w[0].0) / (w[1].0 - w[0].0) } else { 0.0 };
                return w[0].1 + (w[1].1 - w[0].1) * t.clamp(0.0, 1.0);
            }
        }
        known[known.len() - 1].1
    };
    let floor_in_front = |m: &Mouth| ground.at(mesh, [m.base[0] + m.n[0] * 12.0, m.base[1] + m.n[1] * 12.0]).0.unwrap_or(0.0);
    let (fa, fb) = (floor_in_front(&ma), floor_in_front(&mb));
    let mut problems = vec![];
    // ground over the roof all along (none at all, past the edge of the world, is fine); asked
    // before the walls are cut (the ground's index is by triangle)
    let guess = heights(fa + 1.0, fb + 1.0);
    let mut worst: Option<(P2, f64)> = None;
    for k in 1..k_last {
        let (p, s, l) = (cl[k].0, cl[k].1, left(k));
        if s.min(total - s) < 50.0 {
            continue;
        }
        let z = floor_z(&guess, s);
        for side in [-1.0, 0.0, 1.0] {
            let q = [p[0] + l[0] * side * w / 2.0, p[1] + l[1] * side * w / 2.0];
            if let Some(g) = ground.at(mesh, q).0.filter(|g| g - z < need && worst.is_none_or(|x| g - z < x.1)) {
                worst = Some((q, g - z));
            }
        }
    }
    if let Some((q, d)) = worst {
        problems.push(format!("it comes out of the ground near ({:.0}, {:.0}): the ground is {:.0} over its floor there; it needs {need:.0}", q[0], q[1], d.max(0.0)));
    }

    // cut both mouths (putting the walls back if the second can't be cut)
    let saved: Vec<(usize, crate::mesh::Object)> = {
        let mut objs = vec![ma.obj, mb.obj];
        objs.dedup();
        objs.into_iter().map(|o| (o, mesh.objects[o].clone())).collect()
    };
    let restore = |mesh: &mut Mesh| {
        for (o, obj) in &saved {
            mesh.objects[*o] = obj.clone();
        }
    };
    // looking into A's wall, left is the wall's -along; coming out of B's, its +along
    let (ring_a, bottom_a) = punch_with(mesh, ma.obj, ma.base, ma.n, &extent(fa), &|b| Ok(mouth_ring(0.0, -1.0, b))).map_err(|e| format!("its mouth at ({:.0}, {:.0}): {e}", ma.base[0], ma.base[1]))?;
    let (ring_b, bottom_b) = match punch_with(mesh, mb.obj, mb.base, mb.n, &extent(fb), &|b| Ok(mouth_ring(total, 1.0, b))) {
        Ok(x) => x,
        Err(e) => {
            restore(mesh);
            return Err(format!("its mouth at ({:.0}, {:.0}): {e}", mb.base[0], mb.base[1]));
        }
    };

    let known = heights(bottom_a + 1.0, bottom_b + 1.0);
    let steepest = known
        .windows(2)
        .map(|w| ((w[1].1 - w[0].1).abs() / (w[1].0 - w[0].0).max(1e-9)).atan().to_degrees())
        .fold(0.0, f64::max);
    if steepest > max_slope + 1e-9 {
        problems.push(format!("its floor is {steepest:.0} degrees at its steepest, over the walkable {max_slope:.0}"));
    }
    let floor_z = |s: f64| floor_z(&known, s);

    // the cross-sections
    let rings: Vec<Vec<P3>> = (0..=k_last)
        .map(|k| {
            if k == 0 {
                return ring_a.clone();
            }
            if k == k_last {
                return ring_b.clone();
            }
            let (p, s, l) = (cl[k].0, cl[k].1, left(k));
            let z = floor_z(s);
            prof.iter()
                .map(|pt| {
                    let (da, db) = disp(s, pt);
                    [p[0] + l[0] * (pt.a + da), p[1] + l[1] * (pt.a + da), z + pt.b + db]
                })
                .collect()
        })
        .collect();

    // the surfaces, facing in
    let floor_mat = {
        let m = &th.floor.material;
        if m.contains('+') && theme.overlay_texture(m).is_none() { m.split('+').next().unwrap_or(m).to_string() } else { m.clone() }
    };
    let m = prof.len();
    let perimeter = prof[m - 1].arc + (prof[m - 1].a - prof[0].a).hypot(prof[m - 1].b - prof[0].b);
    for k in 0..k_last {
        let (s0, s1) = (cl[k].1, cl[k + 1].1);
        let axis = |k: usize| [cl[k].0[0], cl[k].0[1], floor_z(cl[k].1) + 0.5 * h];
        let (x0, x1) = (axis(k), axis(k + 1));
        let mid = [(x0[0] + x1[0]) / 2.0, (x0[1] + x1[1]) / 2.0, (x0[2] + x1[2]) / 2.0];
        for i in 0..m {
            let j = (i + 1) % m;
            let quad = [rings[k][i], rings[k][j], rings[k + 1][j], rings[k + 1][i]];
            let is_floor = prof[i].floor && prof[j].floor;
            let (mat, surface) = if is_floor { (floor_mat.as_str(), th.floor.surface.as_str()) } else { (th.wall.as_str(), th.wall_surface.as_str()) };
            // round the section: the last piece runs on to its full length, not back to 0
            let arc = |x: usize| if x == 0 && i == m - 1 { perimeter } else { prof[x].arc };
            let uv: [[f64; 2]; 4] = if is_floor {
                quad.map(|q| [q[0] / th.floor.tile, q[1] / th.floor.tile])
            } else {
                [[s0 / th.tile_u, arc(i) / th.tile_v], [s0 / th.tile_u, arc(j) / th.tile_v], [s1 / th.tile_u, arc(j) / th.tile_v], [s1 / th.tile_u, arc(i) / th.tile_v]]
            };
            for (a, b, c) in [(0, 1, 2), (0, 2, 3)] {
                let (p, t) = ([quad[a], quad[b], quad[c]], [uv[a], uv[b], uv[c]]);
                let e = [p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]];
                let f = [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]];
                let nn = [e[1] * f[2] - e[2] * f[1], e[2] * f[0] - e[0] * f[2], e[0] * f[1] - e[1] * f[0]];
                let c3 = [(p[0][0] + p[1][0] + p[2][0]) / 3.0, (p[0][1] + p[1][1] + p[2][1]) / 3.0, (p[0][2] + p[1][2] + p[2][2]) / 3.0];
                let toward = [mid[0] - c3[0], mid[1] - c3[1], mid[2] - c3[2]];
                if nn[0] * toward[0] + nn[1] * toward[1] + nn[2] * toward[2] >= 0.0 {
                    mesh.tri("tunnels", p, t, mat, surface);
                } else {
                    mesh.tri("tunnels", [p[0], p[2], p[1]], [t[0], t[2], t[1]], mat, surface);
                }
            }
        }
    }
    let st = (0..=k_last).map(|k| {
        let (p, s) = (cl[k].0, cl[k].1);
        ([p[0], p[1], floor_z(s) + 0.5 * h], s.min(total - s))
    }).collect();
    let floor = cl.iter().map(|(p, s)| [p[0], p[1], floor_z(*s)]).collect();
    Ok((Bore { st, floor }, problems))
}

/// Where the drawn line, walked over the ground from its first node (or its last), first meets a
/// wall at least `need` tall (or the edge of the world), with room on it for a mouth `half` wide
/// each side and `tall` high.
fn find_mouth(mesh: &Mesh, ground: &Ground, drawn: &[(P2, f64)], forward: bool, need: f64, half: f64, tall: f64) -> Result<Mouth, String> {
    let end = if forward { "first" } else { "last" };
    let total = drawn[drawn.len() - 1].1;
    let s_at = |d: f64| if forward { d } else { total - d };
    let p0 = point_at(drawn, s_at(0.0));
    let mut floor = ground.at(mesh, p0).0.ok_or(format!("its {end} node isn't on a floor"))?;
    let solid = |p: P2, floor: f64| match ground.at(mesh, p).0 {
        None => true,
        Some(z) => z > floor + need,
    };
    let mut tallest: f64 = 0.0;
    let (mut prev, mut d) = (0.0, 5.0f64);
    loop {
        let d1 = d.min(total);
        let p = point_at(drawn, s_at(d1));
        if solid(p, floor) {
            let (mut lo, mut hi) = (prev, d1);
            for _ in 0..12 {
                let m = 0.5 * (lo + hi);
                if solid(point_at(drawn, s_at(m)), floor) { hi = m } else { lo = m }
            }
            let (a, b) = (point_at(drawn, s_at(lo)), point_at(drawn, s_at(hi)));
            let dir = unit(sub(b, a));
            let back = [-dir[0], -dir[1]];
            let (obj, base, n) = nearest_wall_facing(mesh, a, 40.0, Some((back, MAX_SKEW.to_radians().cos()))).ok_or(format!(
                "no wall where it goes in from its {end} node near ({:.0}, {:.0}), or it goes in more than {MAX_SKEW:.0} degrees from square on",
                a[0], a[1]
            ))?;
            let Seat { base, n, floor, .. } = seat(mesh, ground, obj, base, n, half, tall, 60.0, "corner to corner").map_err(|e| format!("where it goes in from its {end} node: {e}"))?;
            let (bend, top) = survey(mesh, obj, base, n, half, floor + tall + 10.0);
            if bend > BEND {
                return Err(format!("the wall it goes into from its {end} node bends {bend:.0} off flat across its mouth; it needs {:.0} of wall within {BEND:.0} of flat", 2.0 * half));
            }
            if top - floor < tall + 10.0 {
                return Err(format!("the wall it goes into from its {end} node is {:.0} tall there; it needs {:.0}", (top - floor).max(0.0), tall + 10.0));
            }
            let at = if forward { hi } else { total - hi };
            return Ok(Mouth { obj, base, n, at });
        }
        let z = ground.at(mesh, p).0.unwrap_or(floor);
        tallest = tallest.max(z - floor);
        floor = z;
        prev = d1;
        if d1 >= total {
            break;
        }
        d += 5.0;
    }
    let lower = if tallest > 30.0 { format!(" (the tallest it meets from there is {tallest:.0}; it needs {need:.0})") } else { String::new() };
    Err(format!("from its {end} node it doesn't go into a wall{lower}: draw it from a floor, through a wall, to a floor beyond"))
}

/// After every tunnel is built: a floor of a blend material (`<floor>+dirt`) shows its second
/// texture all over, and the light falls off inside, to `dark` at `dark_depth` from the nearer mouth.
pub fn finish(mesh: &mut Mesh, bores: &[Bore], th: &TunnelStyle) {
    let Some(o) = mesh.objects.iter_mut().find(|o| o.name == "tunnels") else { return };
    if th.floor.material.contains('+') {
        o.blend = vec![1.0; o.verts.len()];
    }
    o.tints = o
        .verts
        .iter()
        .map(|v| {
            let depth = bores
                .iter()
                .flat_map(|b| b.st.iter())
                .map(|(c, d)| ((c[0] - v[0]).powi(2) + (c[1] - v[1]).powi(2) + (c[2] - v[2]).powi(2), *d))
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .map_or(0.0, |x| x.1);
            let t = (depth / th.dark_depth.max(1.0)).clamp(0.0, 1.0);
            let k = 1.0 - (1.0 - th.dark) * t * t * (3.0 - 2.0 * t);
            [k; 3]
        })
        .collect();
}
