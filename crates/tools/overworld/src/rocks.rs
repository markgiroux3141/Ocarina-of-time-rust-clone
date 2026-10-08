//! Freestanding rocks and arches (`lines` of kind "rock" and "arch"), and the swept rock body
//! under bridge decks, which an arch's body is too (`sweep`).
//!
//! **A rock** is lofted from its footprint (the closed shape of its nodes) up through its
//! contours: rings at heights, each a copy of the footprint scaled about its centre and shifted,
//! the scale and the shift running smoothly from contour to contour (a cubic through them). Its
//! faces push in and out (lumps), and layers cut grooves round it. It stands on the finished
//! ground: its foot is the lowest ground round its footprint, its heights are over that, and it
//! goes on straight down to a little below the lowest ground under it, closed underneath, so it's
//! one closed solid however the ground lies. Its top is flat and collides as floor. Its sides are
//! a wall style, u round it (whole repeats, so there's no seam) and v down from the top: a capped
//! style keeps its caps at the top and the foot, its middle repeating between.
//!
//! **An arch** rises from the ground at its first node to its crown and comes down to the ground
//! at its last, along the curve through its nodes. Its cross-section is a bridge rock's (a top to
//! walk on, sides bulging out and curving round to a rounded underside), square to the arch, so
//! its legs are as thick as its crown is deep; thicker and wider towards its feet, which go a
//! little into the ground.

use crate::doc::{node_sharp, Line};
use crate::geom::*;
use crate::mesh::Mesh;
use crate::noise::{fbm3, relief3};
use crate::paths::centre_line;
use crate::props::Ground;
use crate::doc::WallTexture;
use crate::theme::{Rocks, WallStyle};

/// How far a rock or an arch goes on below the lowest ground under it.
pub const SINK: f64 = 30.0;
/// Arch tops steeper than this (degrees) are drawn as the rock, not the top.
pub const ARCH_TOP: f64 = 50.0;

/// A cross-section's frame along a swept body: its centre on the top surface, the way to its left
/// and down into it (square to the way along, `fwd`), and how far along it is (u).
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub o: P3,
    pub left: P3,
    pub down: P3,
    pub fwd: P3,
    pub s: f64,
}

/// Half a cross-section, (out from the centre, down from the top) from the top's edge round to the
/// bottom's centre, with its arc length at each point. Lumps are scaled by `taper`.
pub struct Half {
    pub pts: Vec<[f64; 2]>,
    pub arc: Vec<f64>,
    pub taper: f64,
}

impl Half {
    /// The bridge rock's half: from the top's edge `hw` out, down `lip`, out by the bulge (`rx`)
    /// and round to the bottom at `depth`.
    pub fn rock(hw: f64, rx: f64, lip: f64, depth: f64, taper: f64) -> Half {
        let mut pts = vec![[hw, 0.0]];
        let k = 14;
        for j in 0..=k {
            let th = j as f64 / k as f64 * std::f64::consts::FRAC_PI_2;
            pts.push([rx * th.cos().powf(0.8), lip + (depth - lip) * th.sin().powf(0.8)]);
        }
        let mut arc = vec![0.0];
        for w in pts.windows(2) {
            arc.push(arc.last().unwrap() + dist(w[0], w[1]));
        }
        Half { pts, arc, taper }
    }
}

/// Lumps on a swept body: up to `amount`, about `scale` across.
#[derive(Clone, Copy, Debug)]
pub struct Lumps {
    pub amount: f64,
    pub scale: f64,
    pub seed: u32,
}

fn add3(a: P3, b: P3) -> P3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale3(a: P3, k: f64) -> P3 {
    [a[0] * k, a[1] * k, a[2] * k]
}

fn sub3(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn norm3(a: P3) -> P3 {
    let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt().max(1e-12);
    [a[0] / l, a[1] / l, a[2] / l]
}

fn cross3(a: P3, b: P3) -> P3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn len3(a: P3) -> f64 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}

/// A triangle turned to face `want`.
pub fn tri_facing(mesh: &mut Mesh, obj: &str, p: [P3; 3], uv: [[f64; 2]; 3], mat: &str, surface: &str, want: [f64; 3]) {
    let (a, b) = ([p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]], [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]]);
    let n = [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    if n[0] * want[0] + n[1] * want[1] + n[2] * want[2] >= 0.0 {
        mesh.tri(obj, p, uv, mat, surface);
    } else {
        mesh.tri(obj, [p[0], p[2], p[1]], [uv[0], uv[2], uv[1]], mat, surface);
    }
}

/// A texture band down a face: (from, to, v at from, v at to), measured down from the top.
type Band = (f64, f64, f64, f64);

/// How a style's texture runs down a swept body's section from the top's edge: the top band's
/// height, then each band's (from, to, v at from, v at to) as far down as `reach`. A capped style:
/// its top cap, then its middle repeating (top-anchored: no bottom cap); a banded style a band at
/// a time; any other once every `tile_u`. Also the texture's height in units (an end face's v).
fn section_bands(ws: &WallStyle, reach: f64) -> (f64, Vec<Band>, f64) {
    match &ws.caps {
        Some(c) => {
            let (m0, m1) = c.middle();
            let unit = c.unit();
            let ht = c.top * c.tile_v;
            let mut bands = vec![(0.0, ht, 1.0, 1.0 - c.top)];
            let (mut d, mut k) = (ht, 0);
            while d < reach {
                let (va, vb) = if c.mirror && k % 2 == 1 { (m0, m1) } else { (m1, m0) };
                bands.push((d, d + unit, va, vb));
                d += unit;
                k += 1;
            }
            (ht, bands, c.tile_v)
        }
        None => {
            let b = ws.band.unwrap_or(ws.tile_u).max(1.0);
            let mut bands = vec![];
            let mut d = 0.0;
            while d < reach || bands.is_empty() {
                bands.push((d, d + b, 1.0, 0.0));
                d += b;
            }
            (b.min(30.0), bands, b)
        }
    }
}

/// A rock body swept along `frames` (one half cross-section per frame and side): its surface
/// textured as `ws`, by arc length down from the top's edges, band by band, each band clipped
/// where the cross-section ends, so both halves meet on the bottom line, vertex for vertex. End
/// faces (its whole cross-section) where `caps` says.
///
/// A capped style with `walls` stretched isn't tiled: each cross-section is cut at fractions of
/// its own length instead, and the texture runs once from the top's edge to the bottom (its middle
/// once under the top cap, for `StretchedMiddle`), as walls do with those settings; each half is
/// then `pieces` pieces round.
#[allow(clippy::too_many_arguments)]
pub fn sweep(mesh: &mut Mesh, obj: &str, frames: &[Frame], halves: &[Half], ws: &WallStyle, lumps: Lumps, caps: (bool, bool), walls: WallTexture, pieces: usize) {
    let smooth = |a: f64, b: f64, x: f64| {
        let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    let reach = halves.iter().map(|h| *h.arc.last().unwrap()).fold(0.0, f64::max);
    let (ht, all_bands, tile_v) = section_bands(ws, reach);
    // a point on frame ri's half (left or right), `a` down from the top's edge, and its outward normal
    let point = |ri: usize, left: bool, a: f64| -> (P3, [f64; 3]) {
        let h = &halves[ri];
        let total = *h.arc.last().unwrap();
        let a = a.clamp(0.0, total);
        let k = (1..h.arc.len()).find(|&k| h.arc[k] >= a).unwrap_or(h.arc.len() - 1);
        let seg = (h.arc[k] - h.arc[k - 1]).max(1e-9);
        let t = (a - h.arc[k - 1]) / seg;
        let (p0, p1) = (h.pts[k - 1], h.pts[k]);
        let (mut x, y) = (p0[0] + (p1[0] - p0[0]) * t, p0[1] + (p1[1] - p0[1]) * t);
        let (dx, dy) = ((p1[0] - p0[0]) / seg, (p1[1] - p0[1]) / seg);
        let (mut nx, ny) = (dy, -dx);
        // the bottom's centre, where the halves meet: square down, so lumps keep them together
        if a >= total - 1e-6 {
            x = 0.0;
            nx = 0.0;
        }
        let f = &frames[ri];
        let lat = if left { f.left } else { [-f.left[0], -f.left[1], -f.left[2]] };
        let at = |x: f64, y: f64| [f.o[0] + lat[0] * x + f.down[0] * y, f.o[1] + lat[1] * x + f.down[1] * y, f.o[2] + lat[2] * x + f.down[2] * y];
        let base = at(x, y);
        let amp = lumps.amount * h.taper * smooth(0.0, ht, a);
        let d = if amp > 0.0 { amp * fbm3([base[0] / lumps.scale, base[1] / lumps.scale, base[2] / lumps.scale], lumps.seed) } else { 0.0 };
        let (x2, y2) = (x + nx * d, y + ny * d);
        (at(x2, y2), [lat[0] * nx + f.down[0] * ny, lat[1] * nx + f.down[1] * ny, lat[2] * nx + f.down[2] * ny])
    };
    // stretched: cuts at (an arc down from the top, then a fraction of the rest), with their v
    let knots: Option<Vec<(f64, f64, f64)>> = ws.caps.as_ref().and_then(|c| {
        let n = pieces.max(2);
        let (m0, m1) = c.middle();
        match walls {
            WallTexture::Tiled => None,
            WallTexture::Stretched => Some((0..=n).map(|j| (0.0, j as f64 / n as f64, 1.0 - j as f64 / n as f64)).collect()),
            WallTexture::StretchedMiddle => {
                let mut k = vec![(0.0, 0.0, 1.0), (ht, 0.0, 1.0 - c.top)];
                k.extend((1..=n).map(|j| (ht, j as f64 / n as f64, m1 + (m0 - m1) * j as f64 / n as f64)));
                Some(k)
            }
        }
    });
    let cut = |knot: (f64, f64, f64), total: f64| {
        let c = knot.0.min(0.5 * total);
        c + knot.1 * (total - c)
    };
    if let Some(knots) = &knots {
        // across, as many units a repeat as keep the texture's shape over a section's mean length
        let mean = halves.iter().map(|h| *h.arc.last().unwrap()).sum::<f64>() / halves.len() as f64;
        let tu = match walls {
            WallTexture::Stretched => ws.tile_u * (mean / tile_v).max(0.25),
            _ => ws.tile_u,
        };
        for ri in 0..halves.len() - 1 {
            let (a0, a1) = (*halves[ri].arc.last().unwrap(), *halves[ri + 1].arc.last().unwrap());
            let (u0, u1) = (frames[ri].s / tu, frames[ri + 1].s / tu);
            for left in [true, false] {
                for w in knots.windows(2) {
                    let (p00, n00) = point(ri, left, cut(w[0], a0));
                    let (p10, n10) = point(ri + 1, left, cut(w[0], a1));
                    let (p11, n11) = point(ri + 1, left, cut(w[1], a1));
                    let (p01, n01) = point(ri, left, cut(w[1], a0));
                    let want = [n00[0] + n10[0] + n11[0] + n01[0], n00[1] + n10[1] + n11[1] + n01[1], n00[2] + n10[2] + n11[2] + n01[2]];
                    let (va, vb) = (w[0].2, w[1].2);
                    tri_facing(mesh, obj, [p00, p10, p11], [[u0, va], [u1, va], [u1, vb]], &ws.material, &ws.surface, want);
                    tri_facing(mesh, obj, [p00, p11, p01], [[u0, va], [u1, vb], [u0, vb]], &ws.material, &ws.surface, want);
                }
            }
        }
    }
    for ri in (0..halves.len() - 1).filter(|_| knots.is_none()) {
        let (a0, a1) = (*halves[ri].arc.last().unwrap(), *halves[ri + 1].arc.last().unwrap());
        let (s0, s1) = (frames[ri].s, frames[ri + 1].s);
        let bands: Vec<Band> = all_bands.iter().copied().take_while(|b| b.0 < a0.max(a1)).collect();
        for left in [true, false] {
            for &(d0, d1, va, vb) in &bands {
                // the band in (t along the strip, arc a), clipped to a <= the arc's length there
                let rect = [(0.0, d0), (1.0, d0), (1.0, d1), (0.0, d1)];
                let lim = |t: f64, a: f64| a0 + (a1 - a0) * t - a;
                let mut poly: Vec<(f64, f64)> = vec![];
                for i in 0..4 {
                    let (a, b) = (rect[i], rect[(i + 1) % 4]);
                    let (fa, fb) = (lim(a.0, a.1), lim(b.0, b.1));
                    if fa >= -1e-9 {
                        poly.push(a);
                    }
                    if (fa >= -1e-9) != (fb >= -1e-9) {
                        let t = fa / (fa - fb);
                        poly.push((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
                    }
                }
                if poly.len() < 3 {
                    continue;
                }
                let vs: Vec<(P3, [f64; 2], [f64; 3])> = poly
                    .iter()
                    .map(|&(t, a)| {
                        let at = a0 + (a1 - a0) * t;
                        let f = if at > 1e-9 { a / at } else { 0.0 };
                        let (p0, n0) = point(ri, left, f * a0);
                        let (p1, n1) = point(ri + 1, left, f * a1);
                        let p = [p0[0] + (p1[0] - p0[0]) * t, p0[1] + (p1[1] - p0[1]) * t, p0[2] + (p1[2] - p0[2]) * t];
                        let n = [n0[0] + n1[0], n0[1] + n1[1], n0[2] + n1[2]];
                        (p, [(s0 + (s1 - s0) * t) / ws.tile_u, va + (vb - va) * (a - d0) / (d1 - d0)], n)
                    })
                    .collect();
                let want = vs.iter().fold([0.0; 3], |acc, v| [acc[0] + v.2[0], acc[1] + v.2[1], acc[2] + v.2[2]]);
                for i in 1..vs.len() - 1 {
                    tri_facing(mesh, obj, [vs[0].0, vs[i].0, vs[i + 1].0], [vs[0].1, vs[i].1, vs[i + 1].1], &ws.material, &ws.surface, want);
                }
            }
        }
    }
    // end faces where the rock ends in the open: its whole cross-section, through the points the
    // body's end has (down each half at the bands' ends, lumps and all), so they share its edges
    for (cap, ri, back) in [(caps.0, 0usize, true), (caps.1, halves.len() - 1, false)] {
        if !cap {
            continue;
        }
        let f = &frames[ri];
        let total = *halves[ri].arc.last().unwrap();
        let mut arcs: Vec<f64> = vec![0.0];
        match &knots {
            Some(k) => arcs.extend(k.iter().map(|&q| cut(q, total)).filter(|&a| a > 1e-9 && a < total - 1e-9)),
            None => arcs.extend(all_bands.iter().flat_map(|b| [b.0, b.1]).filter(|&a| a > 1e-9 && a < total - 1e-9)),
        }
        arcs.push(total);
        arcs.sort_by(f64::total_cmp);
        arcs.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        // the arcs as the body's vertices were made: tiled, through f * total (f = a / total)
        let exact = knots.is_none();
        let at = |left: bool, a: f64| point(ri, left, if exact && total > 1e-9 { a / total * total } else { a }).0;
        let mut ring: Vec<P3> = arcs.iter().map(|&a| at(true, a)).collect();
        ring.extend(arcs.iter().rev().skip(1).map(|&a| at(false, a)));
        // in the section's plane: across (to the right) and down
        let right = [-f.left[0], -f.left[1], -f.left[2]];
        let flat: Vec<P2> = ring.iter().map(|p| {
            let d = sub3(*p, f.o);
            [d[0] * right[0] + d[1] * right[1] + d[2] * right[2], d[0] * f.down[0] + d[1] * f.down[1] + d[2] * f.down[2]]
        }).collect();
        let want = if back { [-f.fwd[0], -f.fwd[1], -f.fwd[2]] } else { f.fwd };
        let key = |p: P2| ((p[0] * 1e6).round() as i64, (p[1] * 1e6).round() as i64);
        let by: std::collections::HashMap<(i64, i64), P3> = flat.iter().zip(&ring).map(|(&q, &p)| (key(q), p)).collect();
        for t in crate::build::triangulate(&flat, &[], 0.0) {
            let q = t.map(|p| by.get(&key(p)).copied().unwrap_or_else(|| add3(f.o, add3(scale3(right, p[0]), scale3(f.down, p[1])))));
            let uv = t.map(|[x, y]| [x / ws.tile_u, 1.0 - y / tile_v]);
            tri_facing(mesh, obj, q, uv, &ws.material, &ws.surface, want);
        }
    }
}

/// A line's nodes as points, repeats dropped (and a closed shape's last, if it's its first).
fn points(line: &Line, closed: bool) -> (Vec<P2>, Vec<bool>) {
    let mut xy: Vec<P2> = vec![];
    let mut sharp: Vec<bool> = vec![];
    for n in line.nodes.iter().filter(|n| n.len() >= 2) {
        let p = [n[0], n[1]];
        if xy.last().is_some_and(|q| dist(*q, p) < 1.0) {
            continue;
        }
        xy.push(p);
        sharp.push(node_sharp(n));
    }
    while closed && xy.len() > 1 && dist(xy[0], xy[xy.len() - 1]) < 1.0 {
        xy.pop();
        sharp.pop();
    }
    (xy, sharp)
}

/// Where a closed polygon's edge crosses itself, if it does.
fn crosses_itself(xy: &[P2]) -> Option<P2> {
    let n = xy.len();
    for i in 0..n {
        for j in i + 2..n {
            if (j + 1) % n == i {
                continue;
            }
            if segments_cross(xy[i], xy[(i + 1) % n], xy[j], xy[(j + 1) % n]) {
                return Some(lerp(xy[i], xy[(i + 1) % n], 0.5));
            }
        }
    }
    None
}

/// The closed curve through `xy` (counter-clockwise), sampled as `curves` says, sharp nodes kept
/// as corners: the same curve a region through these nodes would have.
pub fn footprint(xy: &[P2], sharp: &[bool], curves: Sampling) -> Vec<P2> {
    footprint_pieces(xy, sharp, curves).into_iter().map(|q| q.0).collect()
}

/// `footprint`, each point with the node its piece starts from.
pub fn footprint_pieces(xy: &[P2], sharp: &[bool], curves: Sampling) -> Vec<(P2, usize)> {
    let n = xy.len();
    let mut out = vec![];
    for k in 0..n {
        let (p1, p2) = (xy[k], xy[(k + 1) % n]);
        let p0 = if sharp[k] { [2.0 * p1[0] - p2[0], 2.0 * p1[1] - p2[1]] } else { xy[(k + n - 1) % n] };
        let p3 = if sharp[(k + 1) % n] { [2.0 * p2[0] - p1[0], 2.0 * p2[1] - p1[1]] } else { xy[(k + 2) % n] };
        let pts = sample_curve(p0, p1, p2, p3, curves);
        out.extend(pts[..pts.len() - 1].iter().map(|&q| (q, k)));
    }
    out
}

/// A polygon's centre (of area).
pub fn centroid(pts: &[P2]) -> P2 {
    let (mut a, mut cx, mut cy) = (0.0, 0.0, 0.0);
    for i in 0..pts.len() {
        let (p, q) = (pts[i], pts[(i + 1) % pts.len()]);
        let c = p[0] * q[1] - q[0] * p[1];
        a += c;
        cx += (p[0] + q[0]) * c;
        cy += (p[1] + q[1]) * c;
    }
    if a.abs() < 1e-9 {
        let n = pts.len() as f64;
        return [pts.iter().map(|p| p[0]).sum::<f64>() / n, pts.iter().map(|p| p[1]).sum::<f64>() / n];
    }
    [cx / (3.0 * a), cy / (3.0 * a)]
}

/// A smooth curve through knots (z, value), z increasing: cubic pieces with Catmull-Rom tangents
/// (straight between two knots), constant past the ends.
fn smooth_through(knots: &[(f64, f64)], z: f64) -> f64 {
    let n = knots.len();
    if n == 1 || z <= knots[0].0 {
        return knots[0].1;
    }
    if z >= knots[n - 1].0 {
        return knots[n - 1].1;
    }
    let k = (0..n - 1).find(|&k| z <= knots[k + 1].0).unwrap();
    let ((z0, v0), (z1, v1)) = (knots[k], knots[k + 1]);
    let slope = |a: usize, b: usize| (knots[b].1 - knots[a].1) / (knots[b].0 - knots[a].0);
    let m0 = if k == 0 { slope(0, 1) } else { slope(k - 1, k + 1) };
    let m1 = if k + 2 == n { slope(k, k + 1) } else { slope(k, k + 2) };
    let h = z1 - z0;
    let t = (z - z0) / h;
    let (t2, t3) = (t * t, t * t * t);
    (2.0 * t3 - 3.0 * t2 + 1.0) * v0 + (t3 - 2.0 * t2 + t) * h * m0 + (-2.0 * t3 + 3.0 * t2) * v1 + (t3 - t2) * h * m1
}

/// A rock's shape above its footprint (`Line::contours`, the footprint itself at 0): how much it's
/// scaled and how far it's shifted at each height, smoothly from contour to contour.
pub struct Shape {
    scale: Vec<(f64, f64)>,
    x: Vec<(f64, f64)>,
    y: Vec<(f64, f64)>,
}

impl Shape {
    /// The shape through `contours`, and those left out because they aren't above the one below.
    pub fn new(contours: &[crate::doc::Contour]) -> (Shape, Vec<usize>) {
        let mut sh = Shape { scale: vec![(0.0, 1.0)], x: vec![(0.0, 0.0)], y: vec![(0.0, 0.0)] };
        let mut dropped = vec![];
        for (k, c) in contours.iter().enumerate() {
            if c.z <= sh.top() + 1.0 {
                dropped.push(k);
                continue;
            }
            sh.scale.push((c.z, c.scale.max(0.01)));
            sh.x.push((c.z, c.shift[0]));
            sh.y.push((c.z, c.shift[1]));
        }
        (sh, dropped)
    }

    /// Its top's height over the ground (0 with no contours).
    pub fn top(&self) -> f64 {
        self.scale.last().unwrap().0
    }

    /// The footprint's and each contour's height.
    pub fn heights(&self) -> Vec<f64> {
        self.scale.iter().map(|k| k.0).collect()
    }

    pub fn scale(&self, z: f64) -> f64 {
        smooth_through(&self.scale, z).max(0.01)
    }

    pub fn shift(&self, z: f64) -> P2 {
        [smooth_through(&self.x, z), smooth_through(&self.y, z)]
    }
}

/// How a wall style's texture runs down a rock's side `total` long (from its top to its foot):
/// each band's (from, to, v at from, v at to), measured down from the top. A capped style: its
/// top cap, its middle repeating, its bottom cap at the foot (an odd count if mirrored, stretched
/// to fit; top-anchored, the middle runs on down with no bottom cap); a banded style a band at a
/// time; any other once over it all. With `walls` stretched, a capped style is as on walls: once
/// over it all, or its caps at their size and its middle once between.
fn side_bands(ws: &WallStyle, total: f64, walls: WallTexture) -> Vec<Band> {
    if let Some(c) = &ws.caps {
        // stretched as walls are: once over it all, or the caps at their size and the middle once
        let (m0, m1) = c.middle();
        let (top, bottom) = (c.top * c.tile_v, c.bottom * c.tile_v);
        match walls {
            WallTexture::StretchedMiddle if total > c.tile_v => {
                return vec![(0.0, top, 1.0, 1.0 - c.top), (top, total - bottom, m1, m0), (total - bottom, total, c.bottom, 0.0)];
            }
            WallTexture::Stretched | WallTexture::StretchedMiddle => return vec![(0.0, total.max(1e-6), 1.0, 0.0)],
            WallTexture::Tiled => {}
        }
        if c.anchor == "top" {
            let mut out = vec![(0.0, top, 1.0, 1.0 - c.top)];
            let (mut d, mut k) = (top, 0);
            while d < total {
                let (va, vb) = if c.mirror && k % 2 == 1 { (m0, m1) } else { (m1, m0) };
                out.push((d, d + c.unit(), va, vb));
                d += c.unit();
                k += 1;
            }
            return out;
        }
        let reps = c.repeats(total, false);
        if reps == 0 {
            return vec![(0.0, total, 1.0, 0.0)];
        }
        let unit = (total - top - bottom) / reps as f64;
        let mut out = vec![(0.0, top, 1.0, 1.0 - c.top)];
        for k in 0..reps {
            let (va, vb) = if c.mirror && k % 2 == 1 { (m0, m1) } else { (m1, m0) };
            out.push((top + unit * k as f64, top + unit * (k + 1) as f64, va, vb));
        }
        out.push((total - bottom, total, c.bottom, 0.0));
        return out;
    }
    match ws.band {
        Some(b) if b > 0.0 => {
            let mut out = vec![];
            let mut d = 0.0;
            while d < total {
                out.push((d, d + b, 1.0, 0.0));
                d += b;
            }
            out
        }
        _ => vec![(0.0, total.max(1e-6), 1.0, 0.0)],
    }
}

/// How a rock or an arch is built: rings or cross-sections about every `step` along it, and how
/// a capped style is textured on it (`Settings::wall_texture`, as on walls), each half of an arch's
/// cross-section in `pieces` when it's stretched.
pub struct Fineness {
    pub curves: Sampling,
    pub step: f64,
    pub walls: WallTexture,
    pub pieces: usize,
}

/// Builds a rock (a line of kind "rock") on the ground in `mesh`: objects `rocks` (its sides and
/// underside) and `rock_tops`. Returns what's worth reporting.
pub fn rock(line: &Line, rk: &Rocks, ws: &WallStyle, fine: &Fineness, seed: u32, mesh: &mut Mesh, ground: &Ground) -> Result<Vec<String>, String> {
    let top = &rk.top;
    let mut problems = vec![];
    let (mut xy, mut sharp) = points(line, true);
    if xy.len() < 3 {
        return Err("it needs 3 nodes".into());
    }
    if let Some(p) = crosses_itself(&xy) {
        return Err(format!("its edge crosses itself near ({:.0}, {:.0})", p[0], p[1]));
    }
    if signed_area(&xy) < 0.0 {
        xy.reverse();
        sharp.reverse();
    }
    let ring0 = footprint(&xy, &sharp, fine.curves);
    if let Some(p) = crosses_itself(&ring0) {
        return Err(format!("its edge crosses itself near ({:.0}, {:.0})", p[0], p[1]));
    }
    let n = ring0.len();
    let c = centroid(&ring0);
    // outward, square to the footprint at each point (the mean of the two pieces either side)
    let out: Vec<P2> = (0..n)
        .map(|i| {
            let (a, b, d) = (ring0[(i + n - 1) % n], ring0[i], ring0[(i + 1) % n]);
            let e = |p: P2, q: P2| {
                let l = dist(p, q).max(1e-9);
                [(q[1] - p[1]) / l, -(q[0] - p[0]) / l]
            };
            let (n0, n1) = (e(a, b), e(b, d));
            let m = [n0[0] + n1[0], n0[1] + n1[1]];
            let l = (m[0] * m[0] + m[1] * m[1]).sqrt().max(1e-9);
            [m[0] / l, m[1] / l]
        })
        .collect();
    // the ground round it and under it: its foot is the lowest round it, and it goes on down to
    // below the lowest under it
    let mut missing = 0;
    let mut floor = |p: P2| {
        let z = ground.at(mesh, p).0;
        if z.is_none() {
            missing += 1;
        }
        z
    };
    let round: Vec<Option<f64>> = ring0.iter().map(|&p| floor(p)).collect();
    let under: Vec<Option<f64>> = std::iter::once(c).chain(ring0.iter().step_by(2).map(|&p| lerp(p, c, 0.5))).map(&mut floor).collect();
    let base = round.iter().flatten().copied().fold(f64::INFINITY, f64::min);
    if !base.is_finite() {
        return Err("it isn't on the ground".into());
    }
    if missing > 0 {
        problems.push(format!("{missing} of its points are off the ground"));
    }
    let lowest = round.iter().chain(&under).flatten().copied().fold(base, f64::min);
    let highest = round.iter().chain(&under).flatten().copied().fold(base, f64::max);
    // its shape: the footprint at the ground, then each contour above it
    let contours = if line.contours.is_empty() { vec![crate::doc::Contour::new(300.0, 0.85)] } else { line.contours.clone() };
    let (shape, dropped) = Shape::new(&contours);
    for k in dropped {
        problems.push(format!("contour {} ({:.0}) isn't above the one below it, so it's left out", k + 1, contours[k].z));
    }
    let h = shape.top();
    if h <= 0.0 {
        return Err("it has no contour above the ground".into());
    }
    if base + h <= highest + 1.0 {
        problems.push(format!("its top ({:.0}) is under the ground it stands on ({:.0})", base + h, highest));
    }
    let scale_at = |z: f64| shape.scale(z);
    let shift_at = |z: f64| shape.shift(z);
    // a typical radius, which limits lumps and grooves where the rock is thin
    let area = signed_area(&ring0).abs();
    let r_ref = 0.5 * area.sqrt();
    let r_mean = ring0.iter().map(|&p| dist(p, c)).sum::<f64>() / n as f64;
    let lumps = match &line.noise {
        Some(nz) => (nz.amplitude.max(0.0), nz.scale.max(10.0), nz.seed),
        None => (rk.lumps, rk.lump_scale.max(10.0), 0),
    };
    let lseed = seed ^ lumps.2.wrapping_mul(0x85EB_CA6B);
    let layers = line.layers.as_ref().filter(|l| l.height > 1.0 && l.depth > 0.0);
    // a layer's groove: in by `depth` halfway up it, out again at its ends (a ring at each)
    let inset = |z: f64, s: f64| match layers {
        Some(l) if z > 0.0 && z < h => {
            let d = l.depth.min(0.25 * s * r_ref);
            let f = (z / l.height).fract();
            d * (1.0 - (2.0 * f - 1.0).abs())
        }
        _ => 0.0,
    };
    let ring_at = |z: f64| -> Vec<P3> {
        let (s, sh) = (scale_at(z), shift_at(z));
        let amp = lumps.0.min(0.25 * s * r_ref);
        let ins = inset(z, s);
        (0..n)
            .map(|i| {
                let p = [c[0] + (ring0[i][0] - c[0]) * s + sh[0], c[1] + (ring0[i][1] - c[1]) * s + sh[1]];
                let zz = base + z;
                let d = if amp > 0.0 { amp * relief3([p[0] / lumps.1, p[1] / lumps.1, zz / lumps.1], lseed) } else { 0.0 } - ins;
                [p[0] + out[i][0] * d, p[1] + out[i][1] * d, zz]
            })
            .collect()
    };
    // arc length up the side, from the foot (a dense table), so the texture keeps its size up
    // slopes and overhangs, and the rings come about every `step` along it
    let dz = (h / 600.0).clamp(0.5, 4.0);
    let mut table: Vec<(f64, f64)> = vec![(0.0, 0.0)];
    let radial = |z: f64| {
        let s = scale_at(z);
        (s * r_mean - inset(z, s), shift_at(z))
    };
    let mut prev = radial(0.0);
    let mut z = 0.0;
    while z < h {
        let z1 = (z + dz).min(h);
        let cur = radial(z1);
        let dr = (cur.0 - prev.0).abs() + dist(cur.1, prev.1);
        let a = table.last().unwrap().1 + ((z1 - z) * (z1 - z) + dr * dr).sqrt();
        table.push((z1, a));
        prev = cur;
        z = z1;
    }
    let total = table.last().unwrap().1;
    let z_of = |a: f64| -> f64 {
        let k = table.partition_point(|e| e.1 < a).clamp(1, table.len() - 1);
        let (p, q) = (table[k - 1], table[k]);
        if q.1 - p.1 < 1e-12 {
            q.0
        } else {
            p.0 + (q.0 - p.0) * ((a - p.1) / (q.1 - p.1)).clamp(0.0, 1.0)
        }
    };
    let arc_of = |z: f64| -> f64 {
        let k = table.partition_point(|e| e.0 < z).clamp(1, table.len() - 1);
        let (p, q) = (table[k - 1], table[k]);
        p.1 + (q.1 - p.1) * ((z - p.0) / (q.0 - p.0).max(1e-12)).clamp(0.0, 1.0)
    };
    let bands = side_bands(ws, total, fine.walls);
    let step = fine.step.max(4.0);
    // rings: at the foot, the top, every contour, every band's ends and each layer's ends and
    // middle (exactly), then more where those are further apart than `step` up the side
    let mut need: Vec<f64> = shape.heights();
    need.extend(bands.iter().flat_map(|b| [b.0, b.1]).filter(|&a| a > 0.0 && a < total).map(|a| z_of(total - a)));
    if let Some(l) = layers {
        let half = 0.5 * l.height;
        need.extend((1..).map(|k| k as f64 * half).take_while(|&z| z < h));
    }
    need.sort_by(f64::total_cmp);
    need.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    let mut zs = vec![need[0]];
    for w in need.windows(2) {
        let (a0, a1) = (arc_of(w[0]), arc_of(w[1]));
        let k = ((a1 - a0) / step).ceil().max(1.0) as usize;
        zs.extend((1..k).map(|j| z_of(a0 + (a1 - a0) * j as f64 / k as f64)));
        zs.push(w[1]);
    }
    let rings: Vec<Vec<P3>> = zs.iter().map(|&z| ring_at(z)).collect();
    // u round it: the footprint's arc length, scaled to the rock's mean width, in whole repeats
    let mut u0 = vec![0.0];
    for i in 0..n {
        u0.push(u0[i] + dist(ring0[i], ring0[(i + 1) % n]));
    }
    let perim = u0[n];
    let mean_s = {
        let k = 64;
        (0..k).map(|j| scale_at(h * (j as f64 + 0.5) / k as f64)).sum::<f64>() / k as f64
    };
    // stretched (a capped style): as wide a repeat as keeps the texture's shape over its height
    let tile_u = match (&ws.caps, fine.walls) {
        (Some(c), WallTexture::Stretched) => ws.tile_u * (total / c.tile_v).max(0.25),
        _ => ws.tile_u,
    };
    let reps = ((perim * mean_s / tile_u).round()).max(1.0);
    let u: Vec<f64> = u0.iter().map(|x| x / perim * reps).collect();
    // the sides, strip by strip, each strip within one band
    for j in 0..rings.len() - 1 {
        let (za, zb) = (zs[j], zs[j + 1]);
        let (da, db) = (total - arc_of(za), total - arc_of(zb));
        let mid = 0.5 * (da + db);
        let band = bands.iter().copied().find(|b| mid >= b.0 && mid <= b.1).unwrap_or(*bands.last().unwrap());
        let v = |d: f64| band.2 + (band.3 - band.2) * ((d - band.0) / (band.1 - band.0).max(1e-9));
        let (va, vb) = (v(da), v(db));
        for i in 0..n {
            let k = (i + 1) % n;
            let quad = [rings[j][i], rings[j][k], rings[j + 1][k], rings[j + 1][i]];
            mesh.quad("rocks", quad, [[u[i], va], [u[i + 1], va], [u[i + 1], vb], [u[i], vb]], &ws.material, &ws.surface);
        }
    }
    // straight on down into the ground, closed underneath
    let foot = &rings[0];
    let deep = lowest - SINK;
    let sunk: Vec<P3> = foot.iter().map(|p| [p[0], p[1], deep]).collect();
    let v_foot = bands.last().map_or(0.0, |b| b.3);
    for i in 0..n {
        let k = (i + 1) % n;
        mesh.quad("rocks", [sunk[i], sunk[k], foot[k], foot[i]], [[u[i], v_foot], [u[i + 1], v_foot], [u[i + 1], v_foot], [u[i], v_foot]], &ws.material, &ws.surface);
    }
    let flat = |r: &[P3]| r.iter().map(|p| [p[0], p[1]]).collect::<Vec<P2>>();
    let bottom = flat(&sunk);
    for t in crate::build::triangulate(&bottom, &[], 0.0) {
        let q = t.map(|p| [p[0], p[1], deep]);
        mesh.tri("rocks", [q[0], q[2], q[1]], [[0.0; 2]; 3], &ws.material, "");
    }
    // the flat top
    let last = rings.last().unwrap();
    let ztop = base + h;
    let tt = top.tile.max(1.0);
    let lid = flat(last);
    if crosses_itself(&lid).is_some() {
        problems.push("its top folds over itself: less lumps or grooves".into());
    }
    for t in crate::build::triangulate(&lid, &[], 0.0) {
        mesh.tri("rock_tops", t.map(|p| [p[0], p[1], ztop]), t.map(|p| [p[0] / tt, p[1] / tt]), &top.material, &top.surface);
    }
    Ok(problems)
}

/// How many segments an arch can have (`Line::segments`).
pub const ARCH_SEGMENTS: (u32, u32) = (4, 200);

/// An arch's segments either side of its crown, which is `a_top` along its top of `total`: its own
/// count shared out by length, else about one every `step` (at least 4 a side).
pub fn arch_split(a_top: f64, total: f64, step: f64, segments: Option<u32>) -> (usize, usize) {
    match segments {
        Some(n) => {
            let n = n.clamp(ARCH_SEGMENTS.0, ARCH_SEGMENTS.1) as usize;
            let m0 = ((n as f64 * a_top / total.max(1e-9)).round() as usize).clamp(2, n - 2);
            (m0, n - m0)
        }
        None => {
            let step = step.max(4.0);
            (((a_top / step).ceil() as usize).max(4), (((total - a_top) / step).ceil() as usize).max(4))
        }
    }
}

/// How far apart an arch's cross-sections are with no segments of its own: half `sample` at high
/// detail, as far again at medium, twice at low.
pub fn arch_step(settings: &crate::doc::Settings) -> f64 {
    let k = match settings.detail.as_str() {
        "medium" => 2.0,
        "low" => 3.0,
        _ => 1.0,
    };
    settings.sample.max(1.0) * 0.5 * k
}

/// Along an arch whose plan line is `plan` (as the level samples it), standing on `feet`, `height`
/// tall: how far along its top its crown is, and its top's whole length.
pub fn arch_lengths(plan: &[P2], feet: [f64; 2], height: f64) -> (f64, f64) {
    let len: f64 = plan.windows(2).map(|w| dist(w[0], w[1])).sum::<f64>().max(1e-9);
    let (mut s, mut a, mut prev) = (0.0, 0.0, None::<P3>);
    let (mut a_top, mut z_top) = (0.0, f64::NEG_INFINITY);
    for w in plan.windows(2) {
        let k = (dist(w[0], w[1]) / 4.0).ceil().max(1.0) as usize;
        for j in 0..=k {
            let p = lerp(w[0], w[1], j as f64 / k as f64);
            let q = [p[0], p[1], arch_z(feet, height, (s + dist(w[0], p)) / len)];
            if let Some(o) = prev {
                a += len3(sub3(q, o));
            }
            if q[2] > z_top {
                (a_top, z_top) = (a, q[2]);
            }
            prev = Some(q);
        }
        s += dist(w[0], w[1]);
    }
    (a_top, a)
}

/// An arch's top along its centre, `t` (0 to 1) of the way from its first foot to its last: from
/// `SINK` under the ground at each foot (`feet`) up to `height` over them halfway, steep at the feet
/// and rounded at the crown.
pub fn arch_z(feet: [f64; 2], height: f64, t: f64) -> f64 {
    let rise = (4.0 * t * (1.0 - t)).max(0.0).powf(0.6);
    feet[0] - SINK + (feet[1] - feet[0]) * t + (height + SINK) * rise
}

/// Builds an arch (a line of kind "arch") on the ground in `mesh`: its top (object `rock_tops`;
/// past `ARCH_TOP` steep, the rock) and its body (`rocks`). Returns what's worth reporting.
pub fn arch(line: &Line, rk: &Rocks, ws: &WallStyle, fine: &Fineness, seed: u32, mesh: &mut Mesh, ground: &Ground) -> Result<Vec<String>, String> {
    let top = &rk.top;
    let mut problems = vec![];
    let a = &rk.arch;
    let (xy, _) = points(line, false);
    if xy.len() < 2 {
        return Err("it needs 2 nodes".into());
    }
    let w = line.width.unwrap_or(a.width).max(10.0);
    let height = line.height.unwrap_or(a.height).max(10.0);
    let depth = line.depth.unwrap_or(a.depth).max(10.0);
    let foot = a.foot.max(1.0);
    // the plan line as the level's edges draw it (straight between nodes when they're hard), cut
    // finely, with its length so far
    let (coarse, _) = centre_line(&xy, fine.curves);
    let mut plan: Vec<(P2, f64)> = vec![coarse[0]];
    for w in coarse.windows(2) {
        let k = ((w[1].1 - w[0].1) / 4.0).ceil().max(1.0) as usize;
        plan.extend((1..=k).map(|j| (lerp(w[0].0, w[1].0, j as f64 / k as f64), w[0].1 + (w[1].1 - w[0].1) * j as f64 / k as f64)));
    }
    let len = plan.last().unwrap().1;
    if len < w {
        return Err(format!("its feet are {len:.0} apart, less than its width ({w:.0})"));
    }
    // each foot on the lowest ground under it
    let hw_foot = 0.5 * w * (1.0 + 0.5 * (foot - 1.0)) * (1.0 + a.bulge);
    let reach = hw_foot.max(depth * foot);
    let mut feet = [0.0; 2];
    for (e, &p) in [xy[0], xy[xy.len() - 1]].iter().enumerate() {
        let mut lo = f64::INFINITY;
        for (r, k) in [(0.0, 1), (0.5 * reach, 8), (reach, 12)] {
            for j in 0..k {
                let th = std::f64::consts::TAU * j as f64 / k as f64;
                if let Some(z) = ground.at(mesh, [p[0] + r * th.cos(), p[1] + r * th.sin()]).0 {
                    lo = lo.min(z);
                }
            }
        }
        if !lo.is_finite() {
            return Err(format!("its {} foot isn't on the ground", if e == 0 { "first" } else { "last" }));
        }
        feet[e] = lo;
    }
    // up from a little under each foot to `height` over them at the middle
    let z_at = |t: f64| arch_z(feet, height, t);
    let dense: Vec<(P3, P2)> = plan
        .iter()
        .enumerate()
        .map(|(i, &(p, s))| {
            let (q0, q1) = (plan[i.saturating_sub(1)].0, plan[(i + 1).min(plan.len() - 1)].0);
            let d = sub(q1, q0);
            let l = (d[0] * d[0] + d[1] * d[1]).sqrt().max(1e-9);
            ([p[0], p[1], z_at(s / len)], [d[0] / l, d[1] / l])
        })
        .collect();
    let mut arc3 = vec![0.0];
    for wd in dense.windows(2) {
        arc3.push(arc3.last().unwrap() + len3(sub3(wd[1].0, wd[0].0)));
    }
    let total = *arc3.last().unwrap();
    // its segments (cross-sections between them), one at the crown
    let crest = (0..dense.len()).max_by(|&i, &j| dense[i].0[2].total_cmp(&dense[j].0[2])).unwrap();
    let a_top = arc3[crest];
    let (m0, m1) = arch_split(a_top, total, fine.step, line.segments);
    let m = m0 + m1;
    let arcs: Vec<f64> = (0..=m).map(|i| if i <= m0 { a_top * i as f64 / m0 as f64 } else { a_top + (total - a_top) * (i - m0) as f64 / m1 as f64 }).collect();
    let at = |a: f64| -> (P3, P2, f64) {
        let k = arc3.partition_point(|&x| x < a).clamp(1, arc3.len() - 1);
        let t = ((a - arc3[k - 1]) / (arc3[k] - arc3[k - 1]).max(1e-12)).clamp(0.0, 1.0);
        let (p, q) = (dense[k - 1], dense[k]);
        let d = [p.1[0] + (q.1[0] - p.1[0]) * t, p.1[1] + (q.1[1] - p.1[1]) * t];
        let l = (d[0] * d[0] + d[1] * d[1]).sqrt().max(1e-9);
        (add3(p.0, scale3(sub3(q.0, p.0), t)), [d[0] / l, d[1] / l], plan[k - 1].1 + (plan[k].1 - plan[k - 1].1) * t)
    };
    let pts: Vec<(P3, P2, f64)> = arcs.iter().map(|&a| at(a)).collect();
    let mut frames = vec![];
    let mut halves = vec![];
    for i in 0..=m {
        let (o, dir, s) = pts[i];
        let fwd = norm3(sub3(pts[(i + 1).min(m)].0, pts[i.saturating_sub(1)].0));
        let left = [-dir[1], dir[0], 0.0];
        let up = norm3(cross3(fwd, left));
        frames.push(Frame { o, left, down: scale3(up, -1.0), fwd, s: arcs[i] });
        let t = s / len;
        let fw = (1.0 - (std::f64::consts::PI * t).sin()).max(0.0).powf(1.5);
        let hw = 0.5 * w * (1.0 + 0.5 * (foot - 1.0) * fw);
        let d = depth * (1.0 + (foot - 1.0) * fw);
        halves.push(Half::rock(hw, hw * (1.0 + a.bulge), a.lip.min(d * 0.5), d, 1.0));
    }
    // room under it: its underside at the crown over the ground under the crown
    let crown = (0..=m).max_by(|&i, &j| frames[i].o[2].total_cmp(&frames[j].o[2])).unwrap();
    let under = frames[crown].o[2] - depth;
    let below = ground.at(mesh, [frames[crown].o[0], frames[crown].o[1]]).0.unwrap_or(0.5 * (feet[0] + feet[1]));
    if under - below < 40.0 {
        problems.push(format!("there's no room under it ({:.0} at its crown): make it taller or thinner", (under - below).max(0.0)));
    }
    // too thick for its bend: its underside would fold over itself at the crown
    if crown > 0 && crown < m {
        let (p, q, r) = (frames[crown - 1].o, frames[crown].o, frames[crown + 1].o);
        let (u, v) = (sub3(q, p), sub3(r, q));
        let turn = len3(cross3(norm3(u), norm3(v)));
        let rad = 0.5 * (len3(u) + len3(v)) / turn.max(1e-9);
        if depth > 0.95 * rad {
            problems.push(format!("it's too thick ({depth:.0}) for how tightly it bends at its crown ({rad:.0}): make it wider apart, lower or thinner"));
        }
    }
    // the top, quad by quad: the top's look while it's gentle, the rock past ARCH_TOP
    let tt = top.tile.max(1.0);
    let (m0, m1) = ws.caps.as_ref().map_or((0.0, 1.0), |c| c.middle());
    for i in 0..m {
        let (f0, f1) = (&frames[i], &frames[i + 1]);
        let (h0, h1) = (halves[i].pts[0][0], halves[i + 1].pts[0][0]);
        let edge = |f: &Frame, hw: f64, k: f64| add3(f.o, scale3(f.left, hw * k));
        let (l0, r0, l1, r1) = (edge(f0, h0, 1.0), edge(f0, h0, -1.0), edge(f1, h1, 1.0), edge(f1, h1, -1.0));
        let d = sub3(f1.o, f0.o);
        let steep = d[2].abs().atan2((d[0] * d[0] + d[1] * d[1]).sqrt()).to_degrees() > ARCH_TOP;
        if steep {
            let (ua, ub) = (f0.s / ws.tile_u, f1.s / ws.tile_u);
            mesh.quad("rock_tops", [r0, r1, l1, l0], [[ua, m0], [ub, m0], [ub, m1], [ua, m1]], &ws.material, &ws.surface);
        } else {
            let (ua, ub) = (f0.s / tt, f1.s / tt);
            mesh.quad("rock_tops", [r0, r1, l1, l0], [[ua, -h0 / tt], [ub, -h1 / tt], [ub, h1 / tt], [ua, h0 / tt]], &top.material, &top.surface);
        }
    }
    let lumps = match &line.noise {
        Some(nz) => Lumps { amount: nz.amplitude.max(0.0), scale: nz.scale.max(10.0), seed: seed ^ nz.seed.wrapping_mul(0x85EB_CA6B) },
        None => Lumps { amount: rk.lumps, scale: rk.lump_scale.max(10.0), seed },
    };
    sweep(mesh, "rocks", &frames, &halves, ws, lumps, (true, true), fine.walls, fine.pieces);
    Ok(problems)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_smooth_curve_through_contours_passes_through_them() {
        let k = [(0.0, 1.0), (100.0, 1.2), (250.0, 0.5), (300.0, 0.1)];
        for &(z, v) in &k {
            assert!((smooth_through(&k, z) - v).abs() < 1e-9);
        }
        // two knots: straight between them
        let two = [(0.0, 1.0), (200.0, 0.5)];
        assert!((smooth_through(&two, 50.0) - 0.875).abs() < 1e-9);
        assert_eq!(smooth_through(&two, -10.0), 1.0);
        assert_eq!(smooth_through(&two, 999.0), 0.5);
    }
}
