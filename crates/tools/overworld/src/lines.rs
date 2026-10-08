//! Things drawn along lines of nodes (`Doc::lines`): dirt paths painted into the ground as Kokiri
//! Forest's are, fences standing on it, hanging bridges and walk-through hedges (ADR 0036).
//!
//! A dirt path doesn't lie on the ground as a decal (which would fight the floor for depth). It
//! is cut into the floor: the floors take extra points on two rings round its centre line (with
//! constraint edges between them), one where the dirt is full and one where it ends. Every floor
//! vertex gets a weight from its distance to the centre line, so the floor between the rings
//! fades from grass to dirt, and its triangles are drawn with a two-texture blend (the floor's
//! texture and the same with the dirt drawn over it, `textures::composite_name`) by that weight.
//! The edge wanders a little (smooth noise along the line), as the decals' blotches do.

use crate::doc::Line;
use crate::geom::{dist, dist_to_seg, lerp, segments_cross, signed_area, Sampling, P2, P3};
use crate::mesh::Mesh;
use crate::noise::relief;
use crate::paths::centre_line;
use crate::props::Ground;
use crate::theme::{Dirt, FenceStyle, Hanging, HedgeStyle};

/// One dirt path: its centre line, and where its weight is full (`r1`) and nothing (`r2`).
pub struct DirtLine {
    st: Vec<(P2, f64)>,
    r1: f64,
    r2: f64,
    wobble: f64,
    seed: u32,
    bb: [f64; 4],
}

impl DirtLine {
    pub fn new(line: &Line, theme: &Dirt, sampling: Sampling, seed: u32) -> Result<DirtLine, String> {
        let xy: Vec<P2> = line.nodes.iter().filter(|n| n.len() >= 2).map(|n| [n[0], n[1]]).collect();
        if xy.len() < 2 {
            return Err(format!("dirt path {:?} needs two nodes", line.name));
        }
        let width = line.width.unwrap_or(theme.width).max(10.0);
        let soft = theme.soft.clamp(1.0, width);
        let (st, _) = centre_line(&xy, sampling);
        let (r1, r2) = (width / 2.0 - soft / 2.0, width / 2.0 + soft / 2.0);
        let reach = r2 + theme.wobble.abs();
        let mut bb = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
        for (p, _) in &st {
            bb = [bb[0].min(p[0] - reach), bb[1].min(p[1] - reach), bb[2].max(p[0] + reach), bb[3].max(p[1] + reach)];
        }
        Ok(DirtLine { st, r1, r2, wobble: theme.wobble, seed, bb })
    }

    /// The edge's wander at arc length s.
    fn wob(&self, s: f64) -> f64 {
        self.wobble * relief([s / 110.0, 0.37], self.seed)
    }

    /// The distance from the centre line to p, and the arc length there.
    fn nearest(&self, p: P2) -> (f64, f64) {
        let mut best = (f64::INFINITY, 0.0);
        for w in self.st.windows(2) {
            let (d, t) = dist_to_seg(p, w[0].0, w[1].0);
            if d < best.0 {
                best = (d, w[0].1 + t * (w[1].1 - w[0].1));
            }
        }
        best
    }

    /// How much dirt there is at p: 1 on the path, fading to 0 across the soft edge.
    pub fn weight(&self, p: P2) -> f64 {
        if p[0] < self.bb[0] || p[1] < self.bb[1] || p[0] > self.bb[2] || p[1] > self.bb[3] {
            return 0.0;
        }
        let (d, s) = self.nearest(p);
        let e = d - self.wob(s);
        let t = ((self.r2 - e) / (self.r2 - self.r1)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    /// The ring at radius r (plus the wander) round the centre line, closed round both ends.
    fn ring(&self, r: f64) -> Vec<P2> {
        let n = self.st.len();
        let normal = |i: usize| {
            let (a, b) = (self.st[i.saturating_sub(1)].0, self.st[(i + 1).min(n - 1)].0);
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let l = dx.hypot(dy).max(1e-9);
            [-dy / l, dx / l]
        };
        let at = |i: usize, side: f64| {
            let (c, s) = self.st[i];
            let m = normal(i);
            let k = side * (r + self.wob(s));
            [c[0] + m[0] * k, c[1] + m[1] * k]
        };
        // half circles round the ends, turning clockwise: at the far end from the left side
        // (+normal) through straight ahead to the right; at the near end from the right through
        // straight back to the left
        let cap = |i: usize, from: f64, out: &mut Vec<P2>| {
            let (c, s) = self.st[i];
            let rr = r + self.wob(s);
            for k in 1..6 {
                let a = from - std::f64::consts::PI * k as f64 / 6.0;
                out.push([c[0] + rr * a.cos(), c[1] + rr * a.sin()]);
            }
        };
        let angle = |i: usize| {
            let m = normal(i);
            m[1].atan2(m[0])
        };
        let mut out: Vec<P2> = (0..n).map(|i| at(i, 1.0)).collect();
        cap(n - 1, angle(n - 1), &mut out);
        out.extend((0..n).rev().map(|i| at(i, -1.0)));
        cap(0, angle(0) + std::f64::consts::PI, &mut out);
        out
    }
}

/// A level's dirt paths.
#[derive(Default)]
pub struct DirtPaths {
    pub lines: Vec<DirtLine>,
}

impl DirtPaths {
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn weight(&self, p: P2) -> f64 {
        self.lines.iter().map(|l| l.weight(p)).fold(0.0, f64::max)
    }

    /// Points where the dirt is exactly full or exactly gone, in runs to join with constraint
    /// edges. A point whose weight isn't what its ring says (another stretch of path, or a tight
    /// turn, overlaps it) is left out, which breaks its run there.
    pub fn rings(&self) -> Vec<Vec<P2>> {
        let mut runs = vec![];
        for l in &self.lines {
            for (r, target) in [(l.r1, 1.0), (l.r2, 0.0)] {
                let ring = l.ring(r);
                let mut run: Vec<P2> = vec![];
                for p in ring.iter().chain(ring.first()) {
                    let ok = (self.weight(*p) - target).abs() < 0.08 && run.last().is_none_or(|q| dist(*q, *p) > 1.0);
                    if ok {
                        run.push(*p);
                    } else if run.len() > 1 {
                        runs.push(std::mem::take(&mut run));
                    } else {
                        run.clear();
                    }
                }
                if run.len() > 1 {
                    runs.push(run);
                }
            }
        }
        runs
    }
}

/// A fence along `line`: straight between its nodes (closed: back to the first), a whole number of
/// texture repeats on each stretch so a post stands at every node, its foot on the ground. Points
/// along it every repeat, kept only where the ground bends (more than 3 off the straight line
/// between the kept ones either side). Drawn once (double-sided material) in object `fences`;
/// collides from both sides in `fences_collision`. Returns the problems (stretches off the ground).
pub fn fence(line: &Line, style: &FenceStyle, mesh: &mut Mesh, ground: &Ground) -> Vec<String> {
    let mut problems = vec![];
    let mut xy: Vec<P2> = line.nodes.iter().filter(|n| n.len() >= 2).map(|n| [n[0], n[1]]).collect();
    if line.closed && xy.len() > 2 {
        xy.push(xy[0]);
    }
    let floor = |p: P2| ground.at(mesh, p).0;
    let mut missing = 0;
    let mut panels: Vec<([P3; 4], f64, f64)> = vec![];
    for w in xy.windows(2) {
        let (a, b) = (w[0], w[1]);
        let len = dist(a, b);
        if len < 1.0 {
            continue;
        }
        let n = (len / style.tile).round().max(1.0) as usize;
        // a point at every repeat, with the ground's height there
        let mut pts: Vec<(f64, P2, f64)> = (0..=n)
            .map(|k| {
                let t = k as f64 / n as f64;
                let p = lerp(a, b, t);
                (t, p, floor(p).unwrap_or_else(|| {
                    missing += 1;
                    0.0
                }))
            })
            .collect();
        // keep only the bends
        let mut keep = vec![false; pts.len()];
        keep[0] = true;
        *keep.last_mut().unwrap() = true;
        fn mark(pts: &[(f64, P2, f64)], i: usize, j: usize, keep: &mut [bool]) {
            if j <= i + 1 {
                return;
            }
            let (mut far, mut dmax) = (i, 3.0);
            for k in i + 1..j {
                let f = (pts[k].0 - pts[i].0) / (pts[j].0 - pts[i].0);
                let d = (pts[k].2 - (pts[i].2 + f * (pts[j].2 - pts[i].2))).abs();
                if d > dmax {
                    (far, dmax) = (k, d);
                }
            }
            if far != i {
                keep[far] = true;
                mark(pts, i, far, keep);
                mark(pts, far, j, keep);
            }
        }
        let last = pts.len() - 1;
        mark(&pts, 0, last, &mut keep);
        let mut i = 0;
        pts.retain(|_| {
            i += 1;
            keep[i - 1]
        });
        for q in pts.windows(2) {
            let ((t0, p0, z0), (t1, p1, z1)) = (q[0], q[1]);
            let (u0, u1) = (t0 * n as f64, t1 * n as f64);
            let h = style.height;
            panels.push(([[p0[0], p0[1], z0], [p1[0], p1[1], z1], [p1[0], p1[1], z1 + h], [p0[0], p0[1], z0 + h]], u0, u1));
        }
    }
    for (quad, u0, u1) in panels {
        mesh.quad("fences", quad, [[u0, 0.0], [u1, 0.0], [u1, 1.0], [u0, 1.0]], &style.material, "");
        let back = [quad[1], quad[0], quad[3], quad[2]];
        for f in [quad, back] {
            mesh.col_tri("fences_collision", [f[0], f[1], f[2]], &style.surface);
            mesh.col_tri("fences_collision", [f[0], f[2], f[3]], &style.surface);
        }
    }
    if missing > 0 {
        problems.push(format!("{} {:?}: {missing} of its points are off the ground", line.kind, line.name));
    }
    problems
}

/// A walk-through hedge over the closed loop of `line`'s nodes, straight between them: its top
/// `height` above the ground everywhere (points every `spacing` inside and along the edge), grass
/// skirts round it facing out, down to the ground, both in object `hedges`. Under it, collision
/// only (`hedges_collision`), a floor of tall-grass footsteps a hair above the ground. Its top
/// and skirts don't collide: Link wades through, as spot04's. Returns the problems.
pub fn hedge(line: &Line, st: &HedgeStyle, mesh: &mut Mesh, ground: &Ground) -> Vec<String> {
    let name = if line.name.is_empty() { "hedge".to_string() } else { line.name.clone() };
    let mut xy: Vec<P2> = line.nodes.iter().filter(|n| n.len() >= 2).map(|n| [n[0], n[1]]).collect();
    xy.dedup_by(|a, b| dist(*a, *b) < 1.0);
    while xy.len() > 1 && dist(xy[0], xy[xy.len() - 1]) < 1.0 {
        xy.pop();
    }
    if xy.len() < 3 {
        return vec![format!("hedge {name}: it needs 3 nodes")];
    }
    let n = xy.len();
    for i in 0..n {
        for j in i + 2..n {
            if (j + 1) % n == i {
                continue;
            }
            if segments_cross(xy[i], xy[(i + 1) % n], xy[j], xy[(j + 1) % n]) {
                let p = lerp(xy[i], xy[(i + 1) % n], 0.5);
                return vec![format!("hedge {name}: its edge crosses itself near ({:.0}, {:.0})", p[0], p[1])];
            }
        }
    }
    if signed_area(&xy) < 0.0 {
        xy.reverse();
    }
    // the edge, with points every `spacing`
    let spacing = st.spacing.max(10.0);
    let mut ring: Vec<P2> = vec![];
    for i in 0..n {
        let (a, b) = (xy[i], xy[(i + 1) % n]);
        let k = ((dist(a, b) / spacing).ceil() as usize).max(1);
        ring.extend((0..k).map(|m| lerp(a, b, m as f64 / k as f64)));
    }
    let mut missing = 0;
    let mut floor = |p: P2| {
        ground.at(mesh, p).0.unwrap_or_else(|| {
            missing += 1;
            0.0
        })
    };
    let tris: Vec<[P3; 3]> = crate::build::triangulate_with(&ring, &[], spacing, &[])
        .into_iter()
        .map(|t| {
            let t = if crate::geom::cross(t[0], t[1], t[2]) < 0.0 { [t[0], t[2], t[1]] } else { t };
            t.map(|p| [p[0], p[1], floor(p)])
        })
        .collect();
    let base: Vec<f64> = ring.iter().map(|&p| floor(p)).collect();
    let h = st.height;
    let tt = st.top_tile.max(1.0);
    for t in &tris {
        let top = t.map(|q| [q[0], q[1], q[2] + h]);
        mesh.tri("hedges", top, t.map(|q| [q[0] / tt, q[1] / tt]), &st.top, "");
        mesh.col_tri("hedges_collision", t.map(|q| [q[0], q[1], q[2] + 2.0]), &st.surface);
    }
    // the skirts, the texture running on round the edge
    let mut u = 0.0;
    for i in 0..ring.len() {
        let j = (i + 1) % ring.len();
        let (a, b, za, zb) = (ring[i], ring[j], base[i], base[j]);
        let du = dist(a, b) / st.side_tile.max(1.0);
        let quad = [[a[0], a[1], za], [b[0], b[1], zb], [b[0], b[1], zb + h], [a[0], a[1], za + h]];
        mesh.quad("hedges", quad, [[u, 0.0], [u + du, 0.0], [u + du, 1.0], [u, 1.0]], &st.side, "");
        u += du;
    }
    if missing > 0 {
        vec![format!("hedge {name}: {missing} of its points are off the ground")]
    } else {
        vec![]
    }
}

/// The catenary's parameter for a span `l` sagging `d` in the middle: a (cosh(l / 2a) - 1) = d.
fn catenary_a(l: f64, d: f64) -> f64 {
    if d <= 1e-9 {
        return f64::INFINITY;
    }
    let (mut lo, mut hi): (f64, f64) = (1e-3, 1e9);
    for _ in 0..200 {
        let a = (lo * hi).sqrt();
        if a * ((l / (2.0 * a)).cosh() - 1.0) > d {
            lo = a;
        } else {
            hi = a;
        }
    }
    (lo * hi).sqrt()
}

/// A hanging bridge along `line`: a span between each pair of nodes, each anchor at its node's
/// height (a third value) or the ground's. Its two ends land at their floors' edges (put them
/// anywhere on the floor they start from). The deck sags on a catenary by the theme's share of
/// the span. Returns the problems: anchors off the ground, decks too steep to walk.
pub fn bridge(line: &Line, th: &Hanging, max_slope: f64, mesh: &mut Mesh, ground: &Ground) -> Vec<String> {
    let mut problems = vec![];
    let name = if line.name.is_empty() { "bridge".to_string() } else { line.name.clone() };
    let width = line.width.unwrap_or(th.width).max(20.0);
    let anchors: Vec<(P2, f64)> = line
        .nodes
        .iter()
        .filter(|n| n.len() >= 2)
        .map(|n| {
            let p = [n[0], n[1]];
            let z = n.get(2).copied().or_else(|| ground.at(mesh, p).0).unwrap_or_else(|| {
                problems.push(format!("bridge {name}: an anchor at ({:.0}, {:.0}) is off the ground", p[0], p[1]));
                0.0
            });
            (p, z)
        })
        .collect();
    if anchors.len() < 2 {
        problems.push(format!("bridge {name}: it needs two anchors"));
        return problems;
    }
    // the ends land at their floor's edge, as paths do: each moves towards the other anchor as long
    // as the floor under it stays at its height
    let mut anchors = anchors;
    let last = anchors.len() - 1;
    for (end, toward) in [(0, 1), (last, last - 1)] {
        let ((a, z), b) = (anchors[end], anchors[toward].0);
        let l = dist(a, b);
        let mut land = a;
        let mut s = 5.0;
        while s < l * 0.45 {
            let p = lerp(a, b, s / l);
            match ground.at(mesh, p).0 {
                Some(f) if (f - z).abs() < 6.0 => land = p,
                _ => break,
            }
            s += 5.0;
        }
        anchors[end].0 = land;
    }
    // (deck centre, height, across) at stations along each span, then everything from them
    let mut spans: Vec<Vec<(P2, f64, P2, f64)>> = vec![];
    for w in anchors.windows(2) {
        let ((a, za), (b, zb)) = (w[0], w[1]);
        let l = dist(a, b);
        if l < 1.0 {
            continue;
        }
        let dir = [(b[0] - a[0]) / l, (b[1] - a[1]) / l];
        let across = [-dir[1], dir[0]];
        let k = catenary_a(l, th.sag * l);
        let n = ((l / 40.0).ceil() as usize).max(2);
        let mut st = vec![];
        let mut steepest: f64 = 0.0;
        for i in 0..=n {
            let s = l * i as f64 / n as f64;
            let sag = if k.is_finite() { th.sag * l - k * (((s - l / 2.0) / k).cosh() - 1.0) } else { 0.0 };
            let z = za + (zb - za) * s / l - sag;
            let slope = (zb - za) / l + if k.is_finite() { ((s - l / 2.0) / k).sinh() } else { 0.0 };
            steepest = steepest.max(slope.atan().abs().to_degrees());
            st.push((lerp(a, b, s / l), z, across, s));
        }
        if steepest > max_slope + 1e-9 {
            problems.push(format!("bridge {name}: {steepest:.0} degrees at its steepest, over the walkable {max_slope:.0}"));
        }
        spans.push(st);
    }
    let h = width / 2.0;
    let side = |c: P2, across: P2, k: f64| [c[0] + across[0] * h * k, c[1] + across[1] * h * k];
    for st in &spans {
        for q in st.windows(2) {
            let ((c0, z0, x0, s0), (c1, z1, x1, s1)) = (q[0], q[1]);
            let (l0, r0, l1, r1) = (side(c0, x0, 1.0), side(c0, x0, -1.0), side(c1, x1, 1.0), side(c1, x1, -1.0));
            let (v0, v1) = (s0 / th.plank, s1 / th.plank);
            // the deck: planks on top, the same from beneath
            let top = [[r0[0], r0[1], z0], [r1[0], r1[1], z1], [l1[0], l1[1], z1], [l0[0], l0[1], z0]];
            mesh.quad("hanging", top, [[0.0, v0], [0.0, v1], [th.across, v1], [th.across, v0]], &th.deck, "");
            let under = [top[3], top[2], top[1], top[0]];
            mesh.quad("hanging", under, [[th.across, v0], [th.across, v1], [0.0, v1], [0.0, v0]], &th.under, "");
            mesh.col_tri("hanging_collision", [top[0], top[1], top[2]], &th.surface);
            mesh.col_tri("hanging_collision", [top[0], top[2], top[3]], &th.surface);
            // invisible walls up both edges, facing in
            for (p, q, zp, zq) in [(r1, r0, z1, z0), (l0, l1, z0, z1)] {
                let wall = [[p[0], p[1], zp], [q[0], q[1], zq], [q[0], q[1], zq + th.side], [p[0], p[1], zp + th.side]];
                mesh.col_tri("hanging_collision", [wall[0], wall[1], wall[2]], &th.side_surface);
                mesh.col_tri("hanging_collision", [wall[0], wall[2], wall[3]], &th.side_surface);
            }
            // the hand ropes, a strip on each edge, seen from both sides
            let rw = th.rope_width / 2.0;
            for (p, q) in [(r0, r1), (l0, l1)] {
                let (zp, zq) = (z0 + th.rail, z1 + th.rail);
                let strip = [[p[0], p[1], zp - rw], [q[0], q[1], zq - rw], [q[0], q[1], zq + rw], [p[0], p[1], zp + rw]];
                let (u0, u1) = (s0 / 40.0, s1 / 40.0);
                mesh.quad("hanging", strip, [[u0, 1.0], [u1, 1.0], [u1, 0.0], [u0, 0.0]], &th.rope, "");
                mesh.quad("hanging", [strip[1], strip[0], strip[3], strip[2]], [[u1, 1.0], [u0, 1.0], [u0, 0.0], [u1, 0.0]], &th.rope, "");
            }
        }
        // uprights from the deck's edges to the hand ropes
        let len = st.last().map_or(0.0, |x| x.3);
        let n = ((len / th.spacing).round() as usize).max(1);
        for i in 1..n {
            let s = len * i as f64 / n as f64;
            let j = st.iter().position(|x| x.3 >= s).unwrap_or(st.len() - 1).max(1);
            let (a, b) = (st[j - 1], st[j]);
            let t = if b.3 > a.3 { (s - a.3) / (b.3 - a.3) } else { 0.0 };
            let (c, z) = (lerp(a.0, b.0, t), a.1 + (b.1 - a.1) * t);
            let dir = [a.2[1], -a.2[0]];
            for k in [1.0, -1.0] {
                let e = side(c, a.2, k);
                let (p, q) = ([e[0] - dir[0] * 2.0, e[1] - dir[1] * 2.0], [e[0] + dir[0] * 2.0, e[1] + dir[1] * 2.0]);
                let up = [[p[0], p[1], z], [q[0], q[1], z], [q[0], q[1], z + th.rail], [p[0], p[1], z + th.rail]];
                mesh.quad("hanging", up, [[0.0, 1.0], [0.0, 0.0], [th.rail / 40.0, 0.0], [th.rail / 40.0, 1.0]], &th.rope, "");
                mesh.quad("hanging", [up[1], up[0], up[3], up[2]], [[0.0, 0.0], [0.0, 1.0], [th.rail / 40.0, 1.0], [th.rail / 40.0, 0.0]], &th.rope, "");
            }
        }
    }
    // posts at the anchors' corners
    let [pw, ph] = th.post_size;
    for (i, (a, za)) in anchors.iter().enumerate() {
        let span = spans.get(i).or(spans.last());
        let Some(across) = span.and_then(|s| s.first()).map(|x| x.2) else { continue };
        let dir = [across[1], -across[0]];
        for k in [1.0, -1.0] {
            let c = [a[0] + across[0] * (h + pw / 2.0) * k, a[1] + across[1] * (h + pw / 2.0) * k];
            let corner = |sx: f64, sy: f64| [c[0] + (dir[0] * sx + across[0] * sy) * pw / 2.0, c[1] + (dir[1] * sx + across[1] * sy) * pw / 2.0];
            let ring = [corner(-1.0, -1.0), corner(1.0, -1.0), corner(1.0, 1.0), corner(-1.0, 1.0)];
            let (z0, z1) = (za - 20.0, za + ph);
            for m in 0..4 {
                let (p, q) = (ring[m], ring[(m + 1) % 4]);
                let face = [[p[0], p[1], z0], [q[0], q[1], z0], [q[0], q[1], z1], [p[0], p[1], z1]];
                mesh.quad("hanging", face, [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]], &th.post, "");
                mesh.col_tri("hanging_collision", [face[0], face[1], face[2]], &th.side_surface);
                mesh.col_tri("hanging_collision", [face[0], face[2], face[3]], &th.side_surface);
            }
            let top = ring.map(|p| [p[0], p[1], z1]);
            mesh.quad("hanging", top, [[0.0, 0.0], [1.0, 0.0], [1.0, 0.2], [0.0, 0.2]], &th.post, "");
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    fn theme() -> Dirt {
        Dirt { texture: "dirt_strip".into(), tint: [155.0, 140.0, 52.0], opacity: 0.7, repeats: 2, columns: [16, 48], width: 160.0, soft: 40.0, wobble: 0.0, surface: "dirt".into() }
    }

    fn straight() -> DirtPaths {
        let line = Line { name: "d".into(), kind: "dirt".into(), nodes: vec![vec![0.0, 0.0], vec![1000.0, 0.0]], width: None, closed: false, height: None, noise: None, ..Default::default() };
        DirtPaths { lines: vec![DirtLine::new(&line, &theme(), Sampling::Every(50.0), 0).unwrap()] }
    }

    #[test]
    fn dirt_is_full_on_the_path_and_fades_across_its_edge() {
        let d = straight();
        assert_eq!(d.weight([500.0, 0.0]), 1.0);
        assert_eq!(d.weight([500.0, 59.9]), 1.0);
        assert!((d.weight([500.0, 80.0]) - 0.5).abs() < 1e-9, "half way across the soft edge");
        assert_eq!(d.weight([500.0, 100.1]), 0.0);
        // round ends
        assert_eq!(d.weight([-59.0, 0.0]), 1.0);
        assert_eq!(d.weight([1101.0, 0.0]), 0.0);
        assert_eq!(d.weight([5000.0, 0.0]), 0.0);
    }

    #[test]
    fn fences_stand_on_the_ground_with_a_post_at_every_node() {
        // ground rising 1 in 10 eastward, flat north of y 500
        let mut m = Mesh::default();
        m.quad("ground", [[-1000.0, -1000.0, -100.0], [1000.0, -1000.0, 100.0], [1000.0, 500.0, 100.0], [-1000.0, 500.0, -100.0]], [[0.0; 2]; 4], "ground", "ground");
        let g = Ground::new(&m);
        let style = FenceStyle { material: "fence".into(), height: 40.0, tile: 40.0, surface: "fence".into() };
        // 410 long east-west (on the slope), then 395 north-south (level)
        let line = Line { name: "f".into(), kind: "fence".into(), nodes: vec![vec![-205.0, 0.0], vec![205.0, 0.0], vec![205.0, -395.0]], width: None, closed: false, height: None, noise: None, ..Default::default() };
        assert!(fence(&line, &style, &mut m, &g).is_empty());
        let f = m.objects.iter().find(|o| o.name == "fences").unwrap();
        // whole repeats on each stretch: 410 / 40 -> 10, 395 / 40 -> 10, so u ends on whole numbers
        assert!(f.uvs.iter().flatten().all(|uv| (uv[0] - uv[0].round()).abs() < 1e-9));
        let us: Vec<f64> = f.uvs.iter().flatten().map(|uv| uv[0]).collect();
        assert_eq!(us.iter().cloned().fold(0.0, f64::max), 10.0);
        // on the slope the panels follow the ground (bottom on it, 40 tall); the level stretch is
        // one panel, the even slope too (nothing bends)
        for v in &f.verts {
            let z = v[0] / 10.0;
            assert!((v[2] - z).abs() < 1e-6 || (v[2] - z - 40.0).abs() < 1e-6, "{v:?}");
        }
        assert_eq!(f.tris.len(), 4);
        // collision both ways
        let c = m.objects.iter().find(|o| o.name == "fences_collision").unwrap();
        assert!(c.collision_only && c.tris.len() == 8);
        // off the level: reported
        let away = Line { nodes: vec![vec![5000.0, 0.0], vec![5100.0, 0.0]], ..line };
        assert!(fence(&away, &style, &mut m, &g)[0].contains("off the ground"));
    }

    #[test]
    fn hanging_bridges_sag_by_their_span_and_report_steep_ends() {
        // two plateaus at 200, 600 apart, with a gully at 0 between them
        let mut m = Mesh::default();
        m.quad("ground", [[-800.0, -200.0, 200.0], [-300.0, -200.0, 200.0], [-300.0, 200.0, 200.0], [-800.0, 200.0, 200.0]], [[0.0; 2]; 4], "ground", "ground");
        m.quad("ground", [[300.0, -200.0, 200.0], [800.0, -200.0, 200.0], [800.0, 200.0, 200.0], [300.0, 200.0, 200.0]], [[0.0; 2]; 4], "ground", "ground");
        let g = Ground::new(&m);
        let th = Theme::kokiri().hanging.unwrap();
        let line = Line { name: "b".into(), kind: "bridge".into(), nodes: vec![vec![-300.0, 0.0], vec![300.0, 0.0]], width: None, closed: false, height: None, noise: None, ..Default::default() };
        assert!(bridge(&line, &th, 35.0, &mut m, &g).is_empty());
        let deck = m.objects.iter().find(|o| o.name == "hanging_collision").unwrap();
        let floor_z: Vec<f64> = deck.verts.iter().filter(|v| v[1].abs() < 41.0 && v[2] < 205.0).map(|v| v[2]).collect();
        // it sags 6% of 600 in the middle, and meets the anchors at their height
        let lowest = floor_z.iter().cloned().fold(f64::INFINITY, f64::min);
        assert!((lowest - (200.0 - 36.0)).abs() < 0.5, "{lowest}");
        assert!(deck.verts.iter().any(|v| (v[0] + 300.0).abs() < 1e-6 && (v[2] - 200.0).abs() < 1e-6));
        // drawn: deck, ropes, uprights and posts; the rope's texture is the fence's top rail
        let h = m.objects.iter().find(|o| o.name == "hanging").unwrap();
        assert!(h.tris.len() > 60);
        assert_eq!(Theme::kokiri().texture_name("rope"), "kf_fence@1-8");
        // a deep sag on a short span is too steep to walk
        let steep = Hanging { sag: 0.4, ..th };
        let pr = bridge(&line, &steep, 35.0, &mut m, &g);
        assert!(pr.iter().any(|p| p.contains("over the walkable")), "{pr:?}");
    }

    #[test]
    fn rings_lie_where_the_dirt_is_full_and_where_it_ends() {
        let d = straight();
        let runs = d.rings();
        assert_eq!(runs.len(), 2, "an inner and an outer ring, each one closed run");
        for (run, target) in runs.iter().zip([1.0, 0.0]) {
            assert!(run.len() > 40);
            for p in run {
                assert!((d.weight(*p) - target).abs() < 0.08, "{p:?}");
            }
        }
        // the outer ring goes round the ends 100 out
        assert!(runs[1].iter().any(|p| (p[0] + 100.0).abs() < 1.0 && p[1].abs() < 30.0));
    }
}
