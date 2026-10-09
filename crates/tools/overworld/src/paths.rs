//! Paths: ramps, embankments and bridges along a line of 3D nodes.
//!
//! The centre line is a smooth curve through the nodes (centripetal Catmull-Rom), sampled into
//! stations. Heights run linearly between nodes along the line. A node with no height takes the
//! floor's height there (at an end) or is interpolated (between). Widths are per node or the
//! path's.
//!
//! **Cuttings.** An attached end's slope runs all the way to its node. A ramp drawn from the
//! ground to a point halfway into a plateau rises to the plateau's edge as an embankment and
//! goes on into the plateau as a cutting, reaching the top at the node: inside its footprint the
//! path's surface is the ground, above or below the region's (`build.rs`).
//!
//! **Landing.** A floating end that stands on a floor of its own height, with the ground falling
//! away below the path further in, lands where the floor's edge is: the deck starts there (not
//! at the node, which may be well inside the plateau). So a bridge is drawn between points
//! anywhere on the floors it joins, with no heights at all.
//!
//! **Runs.** Each segment between nodes is attached (an embankment or cutting: its footprint,
//! the `ribbon`, joins the ground's map) or floating (a deck with open space under it).
//! Consecutive segments of one kind make a run.
//!
//! **Sections** (`doc::Section`). A causeway, a sunken lane or a boardwalk raises or sinks the
//! surface from the height its line gives, by the same amount all along its segments; at the
//! path's ends, and where one section meets another, the surface ramps back at `RAMP`, so the ends
//! still meet the ground as drawn. A boardwalk's segments float. A ledge changes only its sides'
//! look (`build.rs`).
//!
//! **Junctions** (`layout_all`). An attached end whose node lies on another path's attached
//! footprint joins it: with no height of its own its line takes the other's line height there,
//! and its section ramps to the other's raise there, so the two surfaces meet. Two ends that join
//! each other (an L, or one path continuing another) both run on past the node by the other's
//! half width, flat, so the outer corner is covered; the earlier path keeps its section to the end
//! and the later one ramps to it. Where joined paths' footprints overlap, and where two paths cross
//! within `JOIN_DZ` of each other's height, the ground is their surfaces blended (`build.rs`).

use crate::doc::{Path, Section};
use crate::geom::*;

/// Two attached paths crossing with their surfaces this close in height there are blended where
/// they overlap (a crossroads); further apart, the higher is the ground (one ramp over another's
/// cutting).
pub const JOIN_DZ: f64 = 40.0;

/// What an end of a path does (`layout_with`), from its junction (`layout_all`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct End {
    /// The line's height at the end when its node gives none (the other path's line there), else
    /// the floor's.
    pub z: Option<f64>,
    /// The raise its section ramps to at the end, else nothing (it meets the ground).
    pub raise: Option<f64>,
    /// How far the end runs on past its node, flat (an end meeting another's end).
    pub extend: f64,
}

/// How steep a section's ramps are, in degrees: from its raised or sunk surface back to its line
/// at the path's ends, and between sections. Each is kept within the half of the segments either
/// side of where it is, so a short segment ramps more steeply (and is reported if too steep).
pub const RAMP: f64 = 25.0;

/// A path's sides at a bend are mitred (`PathGeo::sides`) at most this far out: a turn of up to 120
/// degrees keeps its full width.
const MITRE_MAX: f64 = 2.0;

/// Round a bend, how far from it the measure across a path turns (`PathGeo::cross_sections`): this
/// many times as far as the mitre reaches along the line at the sides. A stair's steps stay level
/// as they fan round, so its inner side climbs this over this less 1 times as steeply as its line
/// (1: the steps would all meet at the inner corner).
const FAN: f64 = 3.0;

/// Stairs bending by this many degrees or more at a point of their line get stations where their
/// steps are square again either side (`layout`), for the fan between to be drawn as one.
const STAIR_TURN: f64 = 15.0;

#[derive(Debug, Clone)]
pub struct Station {
    pub p: P2,
    pub s: f64,
    pub z: f64,
    pub w: f64,
    /// Unit direction along the line.
    pub dir: P2,
    /// The segment (between nodes `seg` and `seg + 1`) it lies on.
    pub seg: usize,
    /// The height its line gives here, and how much its section raises it (`z` is their sum).
    pub line: f64,
    pub raise: f64,
}

#[derive(Debug, Clone)]
pub struct Run {
    pub floating: bool,
    /// Station range, inclusive.
    pub i0: usize,
    pub i1: usize,
    /// Floating runs: whether each end needs an end face (it's free, not landed on a floor or
    /// continuing an embankment).
    pub cap0: bool,
    pub cap1: bool,
    /// Floating runs: whether each end lands on a floor (its deck meets the floor's edge).
    pub land0: bool,
    pub land1: bool,
}

/// Where a point is along a path (`PathGeo::along`): the run's index, how far along it is
/// (horizontally), how far along its surface, how far to the left of the line, how far from it,
/// and the path's width there.
pub type Along = (usize, f64, f64, f64, f64, f64);

#[derive(Debug, Clone)]
pub struct PathGeo {
    pub name: String,
    pub st: Vec<Station>,
    pub runs: Vec<Run>,
    pub edge: Option<String>,
    /// Floating runs' shape ("rock" or "slab"), if the path sets one.
    pub shape: Option<String>,
    /// Steepest slope, in degrees.
    pub max_slope: f64,
    /// Its attached runs' look (`Path::look`).
    pub look: Option<String>,
    /// Each segment's section (`Path::section_of`).
    pub sections: Vec<Option<Section>>,
    /// Railings where its sides drop away: a fence kind (`Path::railings`).
    pub railings: Option<String>,
    /// The paths its ends join, or that join it (`layout_all`).
    pub joins: Vec<usize>,
    /// The paths its ends run into (it joins them, rather than they it): near them its surface
    /// eases onto theirs (`build.rs`).
    pub into: Vec<usize>,
    /// What its layout couldn't do, for the build's problems (a switchback corridor too narrow).
    pub notes: Vec<String>,
}

impl PathGeo {
    /// Station i's section.
    pub fn section(&self, i: usize) -> Option<&Section> {
        self.sections.get(self.st[i].seg).and_then(Option::as_ref)
    }

    /// The section at the station nearest p.
    pub fn section_near(&self, p: P2) -> Option<&Section> {
        let i = (0..self.st.len()).min_by(|&a, &b| dist(self.st[a].p, p).total_cmp(&dist(self.st[b].p, p)))?;
        self.section(i)
    }

    /// The path's height at p: inside its footprint, level along its cross-sections (`in_footprint`),
    /// so round a bend a stair's steps stay level; outside, at the centre-line point nearest p.
    pub fn z_at(&self, p: P2) -> f64 {
        let (mut best, mut z) = (f64::INFINITY, self.st[0].z);
        let mut inside = f64::INFINITY;
        for (i, w) in self.st.windows(2).enumerate() {
            if let Some((t, l)) = self.in_footprint(p, i).filter(|x| x.1.abs() < inside) {
                inside = l.abs();
                z = w[0].z + (w[1].z - w[0].z) * t;
            }
            if inside.is_finite() {
                continue;
            }
            let (d, t) = dist_to_seg(p, w[0].p, w[1].p);
            if d < best {
                best = d;
                z = w[0].z + (w[1].z - w[0].z) * t;
            }
        }
        z
    }

    /// Where p is in the footprint's quads between stations i and i + 1 (`cross_sections`), if it's
    /// in one: (t from 0 at i to 1, from -1 on the right side to 1 on the left).
    fn in_footprint(&self, p: P2, i: usize) -> Option<(f64, f64)> {
        let (a, b) = (&self.st[i], &self.st[i + 1]);
        // every corner is within the longer half cross-section of the line between: so is the quad
        let reach = [self.half(i), self.half(i + 1)].iter().map(|m| m[0].hypot(m[1])).fold(0.0, f64::max);
        if dist_to_seg(p, a.p, b.p).0 > reach + 1e-6 {
            return None;
        }
        self.cross_sections(i)
            .windows(2)
            .flat_map(|w| {
                let ((t0, l0, r0), (t1, l1, r1)) = (w[0], w[1]);
                in_quad(p, [l0, r0], [l1, r1]).into_iter().flatten().map(move |(u, l)| (t0 + (t1 - t0) * u, l))
            })
            .filter(|x| x.1.abs() <= 1.0 + 1e-3)
            .min_by(|x, y| x.1.abs().total_cmp(&y.1.abs()))
    }

    /// Where p is along the attached runs' centre line: the run's index, how far along it is (the
    /// station's `s`, horizontal) and how far along the surface (its slope included), how far to
    /// the left of the line, how far from it, and the path's width there.
    ///
    /// Inside the footprint it's measured across the footprint's own cross-sections (`sides`): each
    /// quad between two stations is mapped back exactly, so where the line bends the measure fans
    /// round the corner and stays continuous (stairs' steps fan as a winding stair's do, rather than
    /// smearing over the outside of the bend and jumping on the inside). Outside, the nearest point
    /// on the line.
    pub fn along(&self, p: P2) -> Option<Along> {
        let mut nearest: Option<Along> = None;
        // in the footprint: how far from the line (in half widths), and where
        let mut inside: Option<(f64, Along)> = None;
        for (k, r) in self.runs.iter().enumerate().filter(|(_, r)| !r.floating) {
            let mut s3 = 0.0;
            for i in r.i0..r.i1 {
                let (a, b) = (&self.st[i], &self.st[i + 1]);
                let l3 = (b.s - a.s).hypot(b.z - a.z);
                let (d, t) = dist_to_seg(p, a.p, b.p);
                if nearest.is_none_or(|x| d < x.4) {
                    let q = sub(p, lerp(a.p, b.p, t));
                    let left = a.dir[0] * q[1] - a.dir[1] * q[0];
                    nearest = Some((k, a.s + (b.s - a.s) * t, s3 + l3 * t, left, d, a.w + (b.w - a.w) * t));
                }
                if let Some((t, l)) = self.in_footprint(p, i).filter(|x| inside.is_none_or(|y| x.1.abs() < y.0)) {
                    let w = a.w + (b.w - a.w) * t;
                    let left = l.clamp(-1.0, 1.0) * 0.5 * w;
                    inside = Some((l.abs(), (k, a.s + (b.s - a.s) * t, s3 + l3 * t, left, left.abs(), w)));
                }
                s3 += l3;
            }
        }
        inside.map(|x| x.1).or(nearest)
    }

    /// The line's height and the section's raise at the centre-line point nearest p.
    pub fn line_raise_at(&self, p: P2) -> (f64, f64) {
        let (mut best, mut out) = (f64::INFINITY, (self.st[0].line, self.st[0].raise));
        for w in self.st.windows(2) {
            let (d, t) = dist_to_seg(p, w[0].p, w[1].p);
            if d < best {
                best = d;
                out = (w[0].line + (w[1].line - w[0].line) * t, w[0].raise + (w[1].raise - w[0].raise) * t);
            }
        }
        out
    }

    /// Where p is along the whole centre line: (s, how far to the left of it, the width there).
    pub fn project(&self, p: P2) -> (f64, f64, f64) {
        let mut best = (f64::INFINITY, 0.0, 0.0, 0.0);
        for w in self.st.windows(2) {
            let (a, b) = (&w[0], &w[1]);
            let (d, t) = dist_to_seg(p, a.p, b.p);
            if d < best.0 {
                let q = sub(p, lerp(a.p, b.p, t));
                best = (d, a.s + (b.s - a.s) * t, a.dir[0] * q[1] - a.dir[1] * q[0], a.w + (b.w - a.w) * t);
            }
        }
        (best.1, best.2, best.3)
    }

    /// Distance from p to the centre line of the attached runs (infinite if none).
    pub fn dist_attached(&self, p: P2) -> (f64, f64) {
        let (mut best, mut z) = (f64::INFINITY, 0.0);
        for r in self.runs.iter().filter(|r| !r.floating) {
            for i in r.i0..r.i1 {
                let (d, t) = dist_to_seg(p, self.st[i].p, self.st[i + 1].p);
                if d < best {
                    best = d;
                    z = self.st[i].z + (self.st[i + 1].z - self.st[i].z) * t;
                }
            }
        }
        (best, z)
    }

    /// The left and right edge points at station i.
    pub fn sides(&self, i: usize) -> (P2, P2) {
        let (s, n) = (&self.st[i], self.half(i));
        ([s.p[0] + n[0], s.p[1] + n[1]], [s.p[0] - n[0], s.p[1] - n[1]])
    }

    /// The footprint's cross-sections between stations i and i + 1, (t from 0 at i to 1, left edge
    /// point, right): the stations' own, and where a station's is mitred, one square to the line
    /// `FAN` times as far from it as the mitre reaches along the line at the sides. Between a mitre
    /// and that one the measure across turns (`along`), round the bend; on past it, it's square.
    fn cross_sections(&self, i: usize) -> Vec<(f64, P2, P2)> {
        let (a, b) = (&self.st[i], &self.st[i + 1]);
        let ((la, ra), (lb, rb)) = (self.sides(i), self.sides(i + 1));
        let mut out = vec![(0.0, la, ra)];
        let len = dist(a.p, b.p);
        if len > 1e-9 {
            let u = [(b.p[0] - a.p[0]) / len, (b.p[1] - a.p[1]) / len];
            let dot = |v: P2| v[0] * u[0] + v[1] * u[1];
            // how far along the line each end's mitre reaches at the sides, and where to square up
            let (ra_, rb_) = (dot(sub(la, a.p)).abs(), dot(sub(lb, b.p)).abs());
            let (mut xa, mut xb) = (FAN * ra_, FAN * rb_);
            if xa + xb > len {
                let k = len / (xa + xb);
                (xa, xb) = (xa * k, xb * k);
            }
            // square cross-sections only where they don't cross a mitre inside the footprint
            if xa + 1e-6 >= ra_ && xb + 1e-6 >= rb_ {
                // the square cross-section at t meets the sides where they cross it
                let at = |t: f64| {
                    let c = lerp(a.p, b.p, t);
                    let meet = |p0: P2, p1: P2| lerp(p0, p1, dot(sub(c, p0)) / dot(sub(p1, p0)).max(1e-9));
                    (t, meet(la, lb), meet(ra, rb))
                };
                // (none on a station: a stairs' are there already, `layout`)
                let (ta, tb) = (xa / len, 1.0 - xb / len);
                if xa > 1e-6 && ta < 1.0 - 1e-9 {
                    out.push(at(ta));
                }
                if xb > 1e-6 && tb > 1e-9 && tb > ta + 1e-9 {
                    out.push(at(tb));
                }
            }
        }
        out.push((1.0, lb, rb));
        out
    }

    /// From station i to its left edge point: half the width across the line, mitred where the line
    /// bends (along the bisector, longer by 1 / cos of half the turn, at most `MITRE_MAX` times), so
    /// the sides stay half the width from the line either side of a sharp corner rather than
    /// pinching in to it.
    fn half(&self, i: usize) -> P2 {
        let s = &self.st[i];
        let unit = |v: P2| {
            let l = v[0].hypot(v[1]);
            (l > 1e-9).then(|| [v[0] / l, v[1] / l])
        };
        let (mut d, mut k) = (s.dir, 1.0);
        if i > 0
            && i + 1 < self.st.len()
            && let (Some(a), Some(b)) = (unit(sub(s.p, self.st[i - 1].p)), unit(sub(self.st[i + 1].p, s.p)))
            && let Some(m) = unit([a[0] + b[0], a[1] + b[1]])
        {
            d = m;
            k = 1.0 / (m[0] * a[0] + m[1] * a[1]).max(1.0 / MITRE_MAX);
        }
        [-d[1] * s.w * 0.5 * k, d[0] * s.w * 0.5 * k]
    }

    /// A run's footprint, counter-clockwise: the right side forwards, the left side back.
    pub fn ribbon(&self, r: &Run) -> Vec<P2> {
        let mut out: Vec<P2> = (r.i0..=r.i1).map(|i| self.sides(i).1).collect();
        out.extend((r.i0..=r.i1).rev().map(|i| self.sides(i).0));
        out
    }
}

/// Where p is in the quad between cross-sections `a` and `b` (each [left, right]): p = c(t) + l m(t),
/// with c(t) running between their middles and m(t) between their halves, both linear in t; so
/// cross(p - c(t), m(t)) = 0, a quadratic in t. Each (t, l) with t in 0 to 1 (l -1 on the right,
/// 1 on the left).
fn in_quad(p: P2, a: [P2; 2], b: [P2; 2]) -> [Option<(f64, f64)>; 2] {
    let cr = |x: P2, y: P2| x[0] * y[1] - x[1] * y[0];
    let mid = |s: [P2; 2]| lerp(s[0], s[1], 0.5);
    let half = |s: [P2; 2]| [(s[0][0] - s[1][0]) * 0.5, (s[0][1] - s[1][1]) * 0.5];
    let (ca, ma, mb) = (mid(a), half(a), half(b));
    let (e, f, g) = (sub(p, ca), sub(ca, mid(b)), sub(mb, ma));
    let (c0, c1, c2) = (cr(e, ma), cr(e, g) + cr(f, ma), cr(f, g));
    let mut ts = [f64::NAN; 2];
    if c2.abs() <= 1e-9 * (c1.abs() + c0.abs()) {
        if c1.abs() > 1e-12 {
            ts[0] = -c0 / c1;
        }
    } else {
        let disc = c1 * c1 - 4.0 * c2 * c0;
        if disc >= 0.0 {
            let h = -0.5 * (c1 + c1.signum() * disc.sqrt());
            ts = [h / c2, if h != 0.0 { c0 / h } else { f64::NAN }];
        }
    }
    ts.map(|t| {
        (-1e-6..=1.0 + 1e-6).contains(&t).then(|| {
            let t = t.clamp(0.0, 1.0);
            let (q, m) = ([e[0] + t * f[0], e[1] + t * f[1]], [ma[0] + t * g[0], ma[1] + t * g[1]]);
            (t, (q[0] * m[0] + q[1] * m[1]) / (m[0] * m[0] + m[1] * m[1]).max(1e-12))
        })
    })
}

fn point_at(st: &[(P2, f64)], s: f64) -> P2 {
    for w in st.windows(2) {
        if s <= w[1].1 {
            let t = if w[1].1 - w[0].1 > 1e-12 { (s - w[0].1) / (w[1].1 - w[0].1) } else { 0.0 };
            return lerp(w[0].0, w[1].0, t.clamp(0.0, 1.0));
        }
    }
    st[st.len() - 1].0
}

/// The value at s of a piecewise-linear function through (s, value) keys, sorted by s, flat
/// past either end.
fn along_keys(keys: &[(f64, f64)], s: f64) -> f64 {
    let Some(first) = keys.first() else { return 0.0 };
    if s <= first.0 {
        return first.1;
    }
    for w in keys.windows(2) {
        if s <= w[1].0 {
            let t = if w[1].0 - w[0].0 > 1e-9 { (s - w[0].0) / (w[1].0 - w[0].0) } else { 1.0 };
            return w[0].1 + (w[1].1 - w[0].1) * t;
        }
    }
    keys[keys.len() - 1].1
}

/// The sections' raise along the line, as (s, raise) keys: each segment's `raise` all along it,
/// ramping at `RAMP` where it changes (centred on the node) and from nothing at the path's ends
/// (inside the end segment). A ramp keeps to the half of each segment beside it.
fn offsets(node_s: &[f64], raise: &[f64], ends: [f64; 2]) -> Vec<(f64, f64)> {
    let n = node_s.len();
    let grade = RAMP.to_radians().tan();
    let seg = |k: usize| node_s[k + 1] - node_s[k];
    let mut keys = vec![];
    // the changes: at each node, from the value before to the value after (past the ends, `ends`)
    for k in 0..n {
        let before = if k == 0 { ends[0] } else { raise[k - 1] };
        let after = if k == n - 1 { ends[1] } else { raise[k] };
        if (after - before).abs() < 1e-9 {
            continue;
        }
        let l = (after - before).abs() / grade;
        let (mut a, mut b) = (0.5 * l, 0.5 * l);
        if k == 0 {
            (a, b) = (0.0, l);
        } else if k == n - 1 {
            (a, b) = (l, 0.0);
        }
        if k > 0 {
            a = a.min(0.5 * seg(k - 1));
        }
        if k < n - 1 {
            b = b.min(0.5 * seg(k));
        }
        keys.push((node_s[k] - a, before));
        keys.push((node_s[k] + b, after));
    }
    if keys.is_empty() {
        return vec![(0.0, raise[0])];
    }
    keys
}

/// A path's centre line through its nodes `xy` (centripetal Catmull-Rom, ends extended
/// straight), sampled as `Detail::paths` says: (point, arc length) pairs, and each node's arc
/// length.
pub fn centre_line(xy: &[P2], sampling: Sampling) -> (Vec<(P2, f64)>, Vec<f64>) {
    let n = xy.len();
    let mut line: Vec<(P2, f64)> = vec![(xy[0], 0.0)];
    let mut node_s = vec![0.0; n];
    for k in 0..n - 1 {
        let (p1, p2) = (xy[k], xy[k + 1]);
        let p0 = if k == 0 { [2.0 * p1[0] - p2[0], 2.0 * p1[1] - p2[1]] } else { xy[k - 1] };
        let p3 = if k + 2 < n { xy[k + 2] } else { [2.0 * p2[0] - p1[0], 2.0 * p2[1] - p1[1]] };
        for p in sample_curve(p0, p1, p2, p3, sampling).into_iter().skip(1) {
            let s = line.last().unwrap().1 + dist(line.last().unwrap().0, p);
            line.push((p, s));
        }
        node_s[k + 1] = line.last().unwrap().1;
    }
    (line, node_s)
}

/// Lay out a path. `base(p)` is the ground's height at p (regions only, before any path).
pub fn layout(path: &Path, base: &dyn Fn(P2) -> f64, sampling: Sampling) -> Result<PathGeo, String> {
    layout_with(path, base, sampling, [End::default(); 2])
}

/// Lays out every path, joining ends that stop on another path (see Junctions above): each path
/// on its own first, then again with its ends' junctions, until nothing changes. A path that
/// can't be laid out is its error; the others go on without it.
pub fn layout_all(paths: &[Path], base: &dyn Fn(P2) -> f64, sampling: Sampling) -> Vec<Result<PathGeo, String>> {
    let n = paths.len();
    let mut ends = vec![[End::default(); 2]; n];
    let mut geos: Vec<Result<PathGeo, String>> = paths.iter().map(|p| layout(p, base, sampling)).collect();
    let mut joins: Vec<(Vec<usize>, Vec<usize>)> = vec![(vec![], vec![]); n];
    for _ in 0..8 {
        let (next, js) = junctions(paths, &geos);
        joins = js;
        if next == ends {
            break;
        }
        ends = next;
        geos = paths.iter().zip(&ends).map(|(p, e)| layout_with(p, base, sampling, *e)).collect();
    }
    for (g, (js, into)) in geos.iter_mut().zip(joins) {
        if let Ok(g) = g {
            g.joins = js;
            g.into = into;
        }
    }
    geos
}

/// Each path's ends from the paths as laid out so far, and the paths each joins (either way) and
/// runs into.
#[allow(clippy::type_complexity)]
fn junctions(paths: &[Path], geos: &[Result<PathGeo, String>]) -> (Vec<[End; 2]>, Vec<(Vec<usize>, Vec<usize>)>) {
    let n = paths.len();
    let xy = |b: usize, e: usize| -> P2 {
        let nd = &paths[b].nodes[if e == 0 { 0 } else { paths[b].nodes.len() - 1 }];
        [nd[0].unwrap_or(0.0), nd[1].unwrap_or(0.0)]
    };
    // each attached end's node on another path's attached footprint: the nearest such path
    let mut on: Vec<[Option<usize>; 2]> = vec![[None; 2]; n];
    for b in 0..n {
        let Ok(gb) = &geos[b] else { continue };
        for e in 0..2 {
            let run = if e == 0 { gb.runs.first() } else { gb.runs.last() };
            if run.is_none_or(|r| r.floating) {
                continue;
            }
            let p = xy(b, e);
            on[b][e] = (0..n)
                .filter(|&a| a != b)
                .filter_map(|a| {
                    let ga = geos[a].as_ref().ok()?;
                    let (d, _) = ga.dist_attached(p);
                    (d <= 0.5 * ga.project(p).2 + 1e-6).then_some((a, d))
                })
                .min_by(|x, y| x.1.total_cmp(&y.1))
                .map(|(a, _)| a);
        }
    }
    let mut ends = vec![[End::default(); 2]; n];
    let mut joins: Vec<(Vec<usize>, Vec<usize>)> = vec![(vec![], vec![]); n];
    for b in 0..n {
        let Ok(gb) = &geos[b] else { continue };
        for e in 0..2 {
            let Some(a) = on[b][e] else { continue };
            let Ok(ga) = &geos[a] else { continue };
            let p = xy(b, e);
            // an end of a's meeting this one: the two ends join each other
            let wb = gb.st[if e == 0 { 0 } else { gb.st.len() - 1 }].w;
            let wa = ga.project(p).2;
            let mutual = (0..2).any(|ea| on[a][ea] == Some(b) && dist(xy(a, ea), p) <= 0.5 * (wa + wb) + 1e-6);
            let own = paths[b].section_of(if e == 0 { 0 } else { paths[b].nodes.len() - 2 }).map_or(0.0, Section::offset);
            let extend = if mutual { 0.5 * wa - 1.0 } else { 0.0 };
            ends[b][e] = if mutual && b < a {
                // the earlier of two ends meeting: it keeps its own height and section to its end
                End { z: None, raise: Some(own), extend }
            } else {
                if !joins[b].1.contains(&a) {
                    joins[b].1.push(a);
                }
                let (line, raise) = ga.line_raise_at(p);
                End { z: Some(line), raise: Some(raise), extend }
            };
            for (x, y) in [(a, b), (b, a)] {
                if !joins[x].0.contains(&y) {
                    joins[x].0.push(y);
                }
            }
        }
    }
    (ends, joins)
}

/// The pairs of paths (lower index first) whose surfaces are blended where their footprints
/// overlap: those joined at an end, and those whose attached centre lines cross within `JOIN_DZ`
/// of each other's height.
pub fn blends(geos: &[PathGeo]) -> std::collections::HashSet<(usize, usize)> {
    let mut out = std::collections::HashSet::new();
    for (a, g) in geos.iter().enumerate() {
        for &b in &g.joins {
            out.insert((a.min(b), a.max(b)));
        }
    }
    let segs = |g: &PathGeo| -> Vec<(usize, usize)> { g.runs.iter().filter(|r| !r.floating).flat_map(|r| (r.i0..r.i1).map(|i| (i, i + 1))).collect() };
    for a in 0..geos.len() {
        for b in a + 1..geos.len() {
            if out.contains(&(a, b)) {
                continue;
            }
            let (ga, gb) = (&geos[a], &geos[b]);
            'pairs: for (i, j) in segs(ga) {
                for (k, l) in segs(gb) {
                    let (p, q, r, s) = (ga.st[i].p, ga.st[j].p, gb.st[k].p, gb.st[l].p);
                    if !segments_cross(p, q, r, s) {
                        continue;
                    }
                    // where they cross: heights along each
                    let d = cross([0.0, 0.0], sub(q, p), sub(s, r));
                    if d.abs() < 1e-12 {
                        continue;
                    }
                    let t = cross([0.0, 0.0], sub(r, p), sub(s, r)) / d;
                    let x = lerp(p, q, t);
                    if (ga.z_at(x) - gb.z_at(x)).abs() < JOIN_DZ {
                        out.insert((a, b));
                        break 'pairs;
                    }
                }
            }
        }
    }
    out
}

/// Adjacent legs of a switchback are at least this many path widths apart, centre to centre.
pub const LEG_SPACING: f64 = 1.25;

/// A switchback path's zig-zag (`meander`): its nodes, each with its height, and what couldn't be
/// done (a corridor too narrow or too short for the turns it needs).
#[derive(Debug, Clone, PartialEq)]
pub struct Meander {
    pub xy: Vec<P2>,
    pub z: Vec<f64>,
    /// How many turns it takes.
    pub turns: usize,
    pub note: Option<String>,
    /// Per stretch between nodes: whether it climbs (flat turns don't).
    pub climbs: Vec<bool>,
}

/// The zig-zag a path with `switchbacks` climbs in, from its first node at height z0 to its last
/// at z1 (None if it has none). Turns are evenly spaced along the line between the nodes, each a
/// half circle out to the corridor's side (radius half their spacing, or less if the corridor is
/// narrower), the legs straight between them, alternating sides. It takes the fewest turns that
/// keep the climb at `grade` or under, as long as legs stay `LEG_SPACING` widths apart; if they
/// can't, it takes the most that fit and says how wide the corridor would have to be. Round turns
/// climb with the legs (one slope all along); flat turns are landings, the legs steeper between.
pub fn meander(path: &Path, z0: f64, z1: f64) -> Result<Option<Meander>, String> {
    let Some(sb) = &path.switchbacks else { return Ok(None) };
    let name = if path.name.is_empty() { "path" } else { path.name.as_str() };
    if path.nodes.len() != 2 {
        return Err(format!("path {name}: switchbacks climb between two nodes, and it has {}", path.nodes.len()));
    }
    let xy = |i: usize| [path.nodes[i][0].unwrap_or(0.0), path.nodes[i][1].unwrap_or(0.0)];
    let (a, b) = (xy(0), xy(1));
    let d = dist(a, b);
    let w = path.width;
    let corridor = sb.width.unwrap_or(4.0 * w);
    let lat = (corridor - w).max(0.0);
    let grade = sb.grade.clamp(1.0, 60.0).to_radians().tan();
    let need = (z1 - z0).abs() / grade;
    let flat = sb.turns == "flat";
    let straight = Meander { xy: vec![a, b], z: vec![z0, z1], turns: 0, note: None, climbs: vec![true] };
    if d < 1.0 || d >= need {
        return Ok(Some(straight));
    }
    // a turn's half circle has to be wider than the path, or its inside folds over
    if lat < LEG_SPACING * w {
        let note = format!("path {name}: a switchback corridor {corridor:.0} wide is too narrow to turn in: it needs at least {:.0}", (1.0 + LEG_SPACING) * w);
        return Ok(Some(Meander { note: Some(note), ..straight }));
    }
    let along = [(b[0] - a[0]) / d, (b[1] - a[1]) / d];
    let side0 = if sb.first == "right" { -1.0 } else { 1.0 };
    let left = [-along[1] * side0, along[0] * side0];
    let at = |x: f64, y: f64| [a[0] + along[0] * x + left[0] * y, a[1] + along[1] * x + left[1] * y];
    // the nodes for k turns, each with whether the stretch from it to the next climbs
    let lay = |k: usize| -> (Vec<P2>, Vec<bool>) {
        let dx = d / k as f64;
        let r = (0.5 * dx).min(0.5 * lat).max(1.0);
        let (mut pts, mut climb) = (vec![a], vec![true]);
        for i in 0..k {
            let c = (i as f64 + 0.5) * dx;
            let s = if i % 2 == 0 { 1.0 } else { -1.0 };
            for j in 0..5 {
                let phi = std::f64::consts::PI * j as f64 / 4.0;
                pts.push(at(c - r * phi.cos(), s * (0.5 * lat - r + r * phi.sin())));
                climb.push(j == 4 || !flat);
            }
        }
        pts.push(b);
        (pts, climb)
    };
    let climbing = |k: usize| -> f64 {
        let (pts, climb) = lay(k);
        pts.windows(2).zip(&climb).filter(|(_, c)| **c).map(|(w, _)| dist(w[0], w[1])).sum()
    };
    let most = ((d / (LEG_SPACING * w)).floor() as usize).max(1);
    let k = (1..=most).find(|&k| climbing(k) >= need);
    let note = match k {
        Some(_) => None,
        None => {
            // as many turns as fit: how wide a corridor they'd need
            let extra = (need - climbing(most)) / most as f64;
            Some(format!(
                "path {name}: {most} switchback turns are all that fit in {d:.0}, which need a corridor about {:.0} wide to climb at {:.0} degrees (it's {corridor:.0})",
                corridor + extra.max(0.0),
                sb.grade
            ))
        }
    };
    let k = k.unwrap_or(most);
    let (pts, climb) = lay(k);
    // heights: the climb shared out over the stretches that climb, by length
    let total: f64 = pts.windows(2).zip(&climb).filter(|(_, c)| **c).map(|(w, _)| dist(w[0], w[1])).sum::<f64>().max(1e-9);
    let mut z = vec![z0];
    let mut acc = 0.0;
    for (w, &c) in pts.windows(2).zip(&climb) {
        if c {
            acc += dist(w[0], w[1]);
        }
        z.push(z0 + (z1 - z0) * acc / total);
    }
    Ok(Some(Meander { xy: pts, z, turns: k, note, climbs: climb }))
}

/// Lays out a path with its ends' junctions (`End`).
pub fn layout_with(path: &Path, base: &dyn Fn(P2) -> f64, sampling: Sampling, ends: [End; 2]) -> Result<PathGeo, String> {
    layout_inner(path, base, sampling, ends, None)
}

/// `layout_with`, and `climbs`: per segment, whether it climbs (else all do). Heights between
/// nodes that have one are linear in the distance climbed, so a stretch that doesn't climb is
/// level, and where a floating end lands short of its node the climb still ends there.
fn layout_inner(path: &Path, base: &dyn Fn(P2) -> f64, sampling: Sampling, ends: [End; 2], climbs: Option<&[bool]>) -> Result<PathGeo, String> {
    if path.switchbacks.is_some() {
        // the zig-zag between its ends' heights, laid out as a path of its own
        let end_z = |i: usize, e: usize| path.nodes.get(i).and_then(|n| n.get(2).copied().flatten()).or(ends[e].z);
        let xy = |i: usize| [path.nodes[i].first().copied().flatten().unwrap_or(0.0), path.nodes[i].get(1).copied().flatten().unwrap_or(0.0)];
        let last = path.nodes.len().saturating_sub(1);
        let z0 = end_z(0, 0).unwrap_or_else(|| base(xy(0)));
        let z1 = end_z(last, 1).unwrap_or_else(|| base(xy(last)));
        if let Some(m) = meander(path, z0, z1)? {
            // heights at the ends only: between, the climb is shared by distance climbed, so it
            // ends where a floating end lands, wherever that is along the zig-zag
            let last = m.xy.len() - 1;
            let laid = Path {
                nodes: m.xy.iter().enumerate().map(|(i, p)| vec![Some(p[0]), Some(p[1]), [Some(z0), Some(z1)].get(if i == 0 { 0 } else { 1 }).copied().flatten().filter(|_| i == 0 || i == last)]).collect(),
                modes: vec![],
                sections: vec![],
                switchbacks: None,
                ..path.clone()
            };
            let mut g = layout_inner(&laid, base, sampling, ends, Some(&m.climbs))?;
            g.notes.extend(m.note);
            return Ok(g);
        }
    }
    let name = if path.name.is_empty() { "path".to_string() } else { path.name.clone() };
    let n = path.nodes.len();
    if n < 2 {
        return Err(format!("path {name}: needs at least 2 nodes"));
    }
    let mut xy = vec![];
    for nd in &path.nodes {
        match (nd.first().copied().flatten(), nd.get(1).copied().flatten()) {
            (Some(x), Some(y)) => xy.push([x, y]),
            _ => return Err(format!("path {name}: a node needs x and y")),
        }
    }
    let zs: Vec<Option<f64>> = path.nodes.iter().map(|nd| nd.get(2).copied().flatten()).collect();
    let ws: Vec<f64> = path.nodes.iter().map(|nd| nd.get(3).copied().flatten().unwrap_or(path.width)).collect();
    let sections: Vec<Option<Section>> = (0..n - 1).map(|k| path.section_of(k).cloned()).collect();
    // a boardwalk floats whatever its mode
    let floating: Vec<bool> = (0..n - 1)
        .map(|k| match path.modes.get(k).map(String::as_str).unwrap_or(path.mode.as_str()) {
            _ if matches!(sections[k], Some(Section::Boardwalk { .. })) => Ok(true),
            "floating" => Ok(true),
            "attached" => Ok(false),
            m => Err(format!("path {name}: unknown mode {m:?} (attached or floating)")),
        })
        .collect::<Result<_, _>>()?;

    // the centre line, and each node's arc position
    let (line, node_s) = centre_line(&xy, sampling);
    let total = node_s[n - 1];

    // heights at the ends: given, or the floor's
    let mut zk = zs.clone();
    for (e, k) in [(0, 0), (1, n - 1)] {
        if zk[k].is_none() {
            zk[k] = Some(ends[e].z.unwrap_or_else(|| base(xy[k])));
        }
    }
    // landing (floating ends): walk in from each end until the ground under the line falls below
    // the end's height
    let mut land: [Option<f64>; 2] = [None, None];
    for (e, (k, inward)) in [(0usize, true), (n - 1, false)].into_iter().enumerate() {
        // a switchback (`climbs`) doesn't: its climb carries on to its end node, cutting in; nor
        // does an end its section raises or sinks (a boardwalk): it ramps on to its node, where it
        // meets the ground
        let seg = if inward { 0 } else { n - 2 };
        if climbs.is_some() || !floating[seg] || sections[seg].as_ref().is_some_and(|s| s.offset().abs() > 1e-9) {
            continue; // an attached end slopes on to its node, cutting in
        }
        let ze = zk[k].unwrap();
        if base(xy[k]) < ze - 0.5 {
            continue; // the end itself is in mid air
        }
        let order: Vec<usize> = if inward { (0..line.len()).collect() } else { (0..line.len()).rev().collect() };
        let mut prev = order[0];
        for &i in &order[1..] {
            if base(line[i].0) < ze - 0.5 {
                // the edge is between stations prev and i: bisect along the line
                let (mut a, mut b) = (line[prev].1, line[i].1);
                for _ in 0..40 {
                    let m = 0.5 * (a + b);
                    if base(point_at(&line, m)) < ze - 0.5 {
                        b = m;
                    } else {
                        a = m;
                    }
                }
                land[e] = Some(0.5 * (a + b));
                break;
            }
            prev = i;
        }
    }
    // heights: linear between nodes that have one, by the distance climbed; landed ends reach
    // their height at the landing (a node with a height past a landing is on the floor, so it's
    // left out)
    let climb_of = |k: usize| climbs.and_then(|c| c.get(k).copied()).unwrap_or(true);
    let mut node_m = vec![0.0; n];
    for k in 1..n {
        node_m[k] = node_m[k - 1] + if climb_of(k - 1) { node_s[k] - node_s[k - 1] } else { 0.0 };
    }
    let metric = |s: f64| -> f64 {
        let k = (0..n - 1).rev().find(|&k| s >= node_s[k]).unwrap_or(0);
        node_m[k] + if climb_of(k) { s - node_s[k] } else { 0.0 }
    };
    let mut keys: Vec<(f64, f64)> = vec![];
    for k in 0..n {
        if let Some(z) = zk[k] {
            let s = if k == 0 { land[0].unwrap_or(node_s[k]) } else if k == n - 1 { land[1].unwrap_or(node_s[k]) } else { node_s[k] };
            if k > 0 && k < n - 1 && (land[0].is_some_and(|l| s < l) || land[1].is_some_and(|l| s > l)) {
                continue;
            }
            keys.push((metric(s), z));
        }
    }
    keys.sort_by(|a, b| a.0.total_cmp(&b.0));
    let z_of = |s: f64| -> f64 {
        let s = metric(s);
        if s <= keys[0].0 {
            return keys[0].1;
        }
        for w in keys.windows(2) {
            if s <= w[1].0 {
                let t = if w[1].0 - w[0].0 > 1e-9 { (s - w[0].0) / (w[1].0 - w[0].0) } else { 1.0 };
                return w[0].1 + (w[1].1 - w[0].1) * t;
            }
        }
        keys[keys.len() - 1].1
    };
    let w_of = |s: f64| -> f64 {
        for k in 0..n - 1 {
            if s <= node_s[k + 1] {
                let t = if node_s[k + 1] - node_s[k] > 1e-9 { (s - node_s[k]) / (node_s[k + 1] - node_s[k]) } else { 0.0 };
                return ws[k] + (ws[k + 1] - ws[k]) * t;
            }
        }
        ws[n - 1]
    };
    let raise = offsets(&node_s, &sections.iter().map(|s| s.as_ref().map_or(0.0, Section::offset)).collect::<Vec<_>>(), ends.map(|e| e.raise.unwrap_or(0.0)));
    let seg_of = |s: f64| -> usize { (0..n - 1).find(|&k| s < node_s[k + 1] - 1e-6).unwrap_or(n - 2) };
    // a floating stretch below the ground under it, anywhere across its width (running on into a
    // higher floor), is a cutting there, as an attached one is, so the cutting covers the higher
    // floor's edge where it crosses and the deck starts clear of it: where it changes, a station
    // (next to a landing, the deck's end follows the floor's edge instead)
    let buried = |s: f64| -> bool {
        let w = w_of(s);
        if !floating[seg_of(s)] || land[0].is_some_and(|l| s < l + w) || land[1].is_some_and(|l| s > l - w) {
            return false;
        }
        let (c, d) = (point_at(&line, s), sub(point_at(&line, (s + 1.0).min(total)), point_at(&line, (s - 1.0).max(0.0))));
        let l = d[0].hypot(d[1]).max(1e-9);
        let h = 0.5 * w;
        let side = [-d[1] / l * h, d[0] / l * h];
        let ground = [c, [c[0] + side[0], c[1] + side[1]], [c[0] - side[0], c[1] - side[1]]].map(|q| base(q)).into_iter().fold(f64::NEG_INFINITY, f64::max);
        ground > z_of(s) + along_keys(&raise, s) + 0.5
    };
    let (lo, hi) = (land[0].unwrap_or(0.0), land[1].unwrap_or(total));
    let mut cuts: Vec<f64> = land.iter().flatten().copied().filter(|&s| s > 1e-6 && s < total - 1e-6).collect();
    for w in line.windows(2) {
        let (mut a, mut b) = (w[0].1.max(lo), w[1].1.min(hi));
        if b - a < 1e-6 || buried(a + 1e-4) == buried(b - 1e-4) {
            continue;
        }
        let at_a = buried(a + 1e-4);
        for _ in 0..40 {
            let m = 0.5 * (a + b);
            if buried(m) == at_a {
                a = m;
            } else {
                b = m;
            }
        }
        cuts.push(0.5 * (a + b));
    }
    // stairs: a station each side of a sharp bend where the steps are square to the line again
    // (`PathGeo::cross_sections`), so the fan round the bend is a quad of its own and the stretch
    // on is square
    if path.look.as_deref() == Some("steps") {
        let unit = |v: P2| {
            let l = v[0].hypot(v[1]);
            (l > 1e-9).then(|| [v[0] / l, v[1] / l])
        };
        for j in 1..line.len().saturating_sub(1) {
            let (p, s) = line[j];
            let (Some(a), Some(b)) = (unit(sub(p, line[j - 1].0)), unit(sub(line[j + 1].0, p))) else { continue };
            let turn = (a[0] * b[0] + a[1] * b[1]).clamp(-1.0, 1.0).acos();
            if turn < STAIR_TURN.to_radians() {
                continue;
            }
            // how far along the line the mitre reaches at the sides (`PathGeo::half`)
            let reach = 0.5 * w_of(s) * (0.5 * turn).sin() / (0.5 * turn).cos().max(1.0 / MITRE_MAX);
            for (room, dir) in [(s - line[j - 1].1, -1.0), (line[j + 1].1 - s, 1.0)] {
                let x = (FAN * reach).min(0.5 * room);
                if x >= reach {
                    cuts.push(s + dir * x);
                }
            }
        }
    }
    // the stations, with the landing points and those added
    cuts.sort_by(f64::total_cmp);
    let mut st_s: Vec<(P2, f64)> = vec![];
    let mut ci = 0;
    for &(p, s) in &line {
        while ci < cuts.len() && cuts[ci] < s - 1e-6 {
            st_s.push((point_at(&line, cuts[ci]), cuts[ci]));
            ci += 1;
        }
        if ci < cuts.len() && (cuts[ci] - s).abs() <= 1e-6 {
            ci += 1;
        }
        st_s.push((p, s));
    }
    let m = st_s.len();
    let st: Vec<Station> = (0..m)
        .map(|i| {
            let (a, b) = (st_s[i.saturating_sub(1)].0, st_s[(i + 1).min(m - 1)].0);
            let d = sub(b, a);
            let l = d[0].hypot(d[1]).max(1e-9);
            let s = st_s[i].1;
            let (line, raise) = (z_of(s), along_keys(&raise, s));
            Station { p: st_s[i].0, s, z: line + raise, w: w_of(s), dir: [d[0] / l, d[1] / l], seg: seg_of(s), line, raise }
        })
        .collect();
    // runs of one kind, between each pair of stations floating (its segment's mode, unless buried)
    // or attached; past a landing, none. Floating runs start and end at landings, and need no end
    // face where they land or continue an embankment or cutting
    let kind = |i: usize| -> Option<bool> {
        let s = 0.5 * (st[i].s + st[i + 1].s);
        (s > lo && s < hi).then(|| floating[seg_of(s)] && !buried(s))
    };
    let mut runs: Vec<Run> = vec![];
    let mut i = 0;
    while i + 1 < m {
        let Some(fl) = kind(i) else {
            i += 1;
            continue;
        };
        let mut e = i + 1;
        while e + 1 < m && kind(e) == Some(fl) {
            e += 1;
        }
        let land0 = fl && land[0].is_some_and(|l| (st[i].s - l).abs() < 1e-6);
        let land1 = fl && land[1].is_some_and(|l| (st[e].s - l).abs() < 1e-6);
        runs.push(Run { floating: fl, i0: i, i1: e, cap0: i == 0 && !land0, cap1: e == m - 1 && !land1, land0, land1 });
        i = e;
    }
    // joined ends meeting another's end run on past their node, flat
    let mut st = st;
    if ends[0].extend > 1e-6 && !floating[0] {
        let f = st[0].clone();
        st.insert(0, Station { p: [f.p[0] - f.dir[0] * ends[0].extend, f.p[1] - f.dir[1] * ends[0].extend], s: f.s - ends[0].extend, ..f });
        for r in &mut runs {
            r.i0 += 1;
            r.i1 += 1;
        }
        runs[0].i0 = 0;
    }
    if ends[1].extend > 1e-6 && !floating[n - 2] {
        let l = st[st.len() - 1].clone();
        st.push(Station { p: [l.p[0] + l.dir[0] * ends[1].extend, l.p[1] + l.dir[1] * ends[1].extend], s: l.s + ends[1].extend, ..l });
        let last = st.len() - 1;
        runs.last_mut().unwrap().i1 = last;
    }
    let max_slope = st
        .windows(2)
        .map(|w| ((w[1].z - w[0].z).abs() / (w[1].s - w[0].s).max(1e-9)).atan().to_degrees())
        .fold(0.0, f64::max);
    match path.look.as_deref() {
        None | Some("steps") | Some("dirt") => {}
        Some(l) => return Err(format!("path {name}: unknown look {l:?} (steps, dirt, or none for the ground)")),
    }
    let geo = PathGeo {
        name: name.clone(),
        st,
        runs,
        edge: path.edge.clone(),
        shape: path.shape.clone(),
        max_slope,
        look: path.look.clone(),
        sections,
        railings: path.railings.clone(),
        joins: vec![],
        into: vec![],
        notes: vec![],
    };
    // a footprint that folds over itself turns too tightly for its width
    for r in geo.runs.iter().filter(|r| !r.floating) {
        let rb = geo.ribbon(r);
        let nr = rb.len();
        for i in 0..nr {
            for j in i + 2..nr {
                if i == 0 && j == nr - 1 {
                    continue;
                }
                if segments_cross(rb[i], rb[(i + 1) % nr], rb[j], rb[(j + 1) % nr]) {
                    return Err(format!("path {name}: turns too tightly for its width near ({:.0}, {:.0})", rb[i][0], rb[i][1]));
                }
            }
        }
    }
    Ok(geo)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(nodes: Vec<Vec<Option<f64>>>, modes: Vec<&str>) -> Path {
        Path { name: "t".into(), nodes, width: 100.0, modes: modes.into_iter().map(String::from).collect(), ..Default::default() }
    }

    /// Ground at 0 except a plateau at 200 for x > 1000.
    fn base(p: P2) -> f64 {
        if p[0] > 1000.0 { 200.0 } else { 0.0 }
    }

    #[test]
    fn a_ramp_slopes_on_to_its_end_node() {
        let g = layout(&path(vec![vec![Some(0.0), Some(0.0)], vec![Some(1300.0), Some(0.0)]], vec![]), &base, Sampling::Every(30.0)).unwrap();
        // z 0 at the start, 200 at the node (x 1300) well inside the plateau: below the
        // plateau's 200 from its edge (x 1000) on, so it cuts in
        assert!(g.st[0].z.abs() < 1e-9);
        assert!((g.z_at([1300.0, 0.0]) - 200.0).abs() < 1e-6);
        assert!((g.z_at([650.0, 0.0]) - 100.0).abs() < 1.0);
        assert!((g.z_at([1000.0, 0.0]) - 200.0 * 1000.0 / 1300.0).abs() < 1.0, "{}", g.z_at([1000.0, 0.0]));
        assert_eq!(g.runs.len(), 1);
    }

    #[test]
    fn a_stair_turning_a_corner_keeps_its_width_and_its_steps_level() {
        // stairs 100 wide climbing 0 to 200, turning 90 degrees at (600, 0)
        let mut p = path(vec![vec![Some(0.0), Some(0.0), Some(0.0)], vec![Some(600.0), Some(0.0)], vec![Some(600.0), Some(600.0), Some(200.0)]], vec![]);
        p.look = Some("steps".into());
        let g = layout(&p, &|_| 0.0, Sampling::Straight { max: 1000.0 }).unwrap();
        let c = g.st.iter().position(|s| dist(s.p, [600.0, 0.0]) < 1e-6).unwrap();
        // mitred: the corner's sides half the width from both legs' lines (not pinched in)
        let (l, r) = g.sides(c);
        for q in [l, r] {
            assert!((q[1].abs() - 50.0).abs() < 1e-6 && ((q[0] - 600.0).abs() - 50.0).abs() < 1e-6, "{q:?}");
        }
        // square to the line again either side of the bend, where the steps' fan ends
        for q in [[600.0 - FAN * 50.0, 0.0], [600.0, FAN * 50.0]] {
            assert!(g.st.iter().any(|s| dist(s.p, q) < 1e-6), "a station at {q:?}");
        }
        // each step level across the stair: the corner's sides at its height
        for q in [l, r] {
            assert!((g.z_at(q) - g.st[c].z).abs() < 1e-6, "{q:?}: {} not {}", g.z_at(q), g.st[c].z);
        }
        // and the measure along it continuous through the bend, on the inside and the outside
        // (round a corner a step's inner end is narrower: up to about twice as much per unit there)
        let tangent = [0.5f64.sqrt(), 0.5f64.sqrt()];
        for t in [0.05, 0.5, 0.95] {
            let m = lerp(r, l, t);
            let at = |k: f64| g.along([m[0] + k * tangent[0], m[1] + k * tangent[1]]).unwrap();
            let (a, b) = (at(-0.5), at(0.5));
            assert!(b.2 > a.2 && b.2 - a.2 < 4.0, "{t}: {} to {}", a.2, b.2);
            assert!((b.3 - a.3).abs() < 1.0, "{t}: across {} to {}", a.3, b.3);
        }
    }

    #[test]
    fn a_bridge_starts_and_ends_at_the_edges_it_spans() {
        // from the plateau (x 1300) back over the ground to a second plateau... here: back to x -300
        // with an explicit far height in mid air
        let p = path(vec![vec![Some(1300.0), Some(0.0)], vec![Some(-300.0), Some(0.0), Some(150.0)]], vec!["floating"]);
        let g = layout(&p, &base, Sampling::Every(30.0)).unwrap();
        let r = &g.runs[0];
        assert!(r.floating && !r.cap0 && r.cap1, "landed at the plateau, free at the far end");
        assert!((g.st[r.i0].p[0] - 1000.0).abs() < 1.0, "deck starts at the plateau's edge: {}", g.st[r.i0].p[0]);
    }

    #[test]
    fn attached_then_floating_makes_two_runs_with_no_face_between() {
        let p = path(
            vec![vec![Some(0.0), Some(0.0)], vec![Some(400.0), Some(0.0), Some(150.0)], vec![Some(1300.0), Some(0.0)]],
            vec!["attached", "floating"],
        );
        let g = layout(&p, &base, Sampling::Every(30.0)).unwrap();
        assert_eq!(g.runs.len(), 2);
        assert!(!g.runs[0].floating && g.runs[1].floating);
        assert!(!g.runs[1].cap0 && !g.runs[1].cap1);
        assert_eq!(g.runs[0].i1, g.runs[1].i0);
    }

    #[test]
    fn switchbacks_zig_zag_to_keep_the_climb_walkable() {
        use crate::doc::Switchbacks;
        // 600 up over 1200: 26.6 degrees straight, so it zig-zags at 20 across a 800 corridor
        let mut p = path(vec![vec![Some(0.0), Some(0.0)], vec![Some(0.0), Some(1200.0), Some(600.0)]], vec![]);
        p.width = 120.0;
        p.switchbacks = Some(Switchbacks { width: Some(800.0), ..Default::default() });
        let m = meander(&p, 0.0, 600.0).unwrap().unwrap();
        assert!(m.turns >= 1 && m.note.is_none(), "{m:?}");
        let g = layout(&p, &|_| 0.0, Sampling::Every(20.0)).unwrap();
        assert!(g.max_slope <= 20.5, "{}", g.max_slope);
        assert!((g.st[0].z).abs() < 1e-6 && (g.st.last().unwrap().z - 600.0).abs() < 1e-6);
        // inside the corridor: the legs' centre lines within its half width less half the path's
        assert!(g.st.iter().all(|s| s.p[0].abs() <= 400.0 - 60.0 + 1.0), "{:?}", g.st.iter().map(|s| s.p[0]).fold(0.0, f64::max));
        // a corridor too narrow: as many turns as fit, and how wide it would need to be
        p.switchbacks = Some(Switchbacks { width: Some(300.0), ..Default::default() });
        let narrow = meander(&p, 0.0, 900.0).unwrap().unwrap();
        assert!(narrow.note.as_ref().is_some_and(|n| n.contains("corridor about")), "{narrow:?}");
        assert_eq!(narrow.turns, (1200.0 / (LEG_SPACING * 120.0)) as usize);
        // too narrow to turn at all: straight, and said so
        p.switchbacks = Some(Switchbacks { width: Some(250.0), ..Default::default() });
        let none = meander(&p, 0.0, 600.0).unwrap().unwrap();
        assert!(none.turns == 0 && none.note.as_ref().is_some_and(|n| n.contains("too narrow")), "{none:?}");
        // flat turns: each turn's nodes level
        p.switchbacks = Some(Switchbacks { width: Some(800.0), turns: "flat".into(), ..Default::default() });
        let flat = meander(&p, 0.0, 600.0).unwrap().unwrap();
        for t in 0..flat.turns {
            let zs = &flat.z[1 + 5 * t..1 + 5 * t + 5];
            assert!(zs.iter().all(|z| (z - zs[0]).abs() < 1e-9), "{zs:?}");
        }
        // gentle enough already: straight
        let easy = meander(&p, 0.0, 100.0).unwrap().unwrap();
        assert_eq!(easy.turns, 0);
    }

    #[test]
    fn a_boardwalk_over_a_pond_ramps_down_to_its_ends_instead_of_landing() {
        // a pond 292 deep for |x| < 300: a bridge would land on its shores, but a boardwalk raised
        // 80 runs on to its nodes on the ground and meets it there
        let pond = |p: P2| if p[0].abs() < 300.0 { -292.0 } else { 0.0 };
        let mut p = path(vec![vec![Some(-370.0), Some(0.0)], vec![Some(370.0), Some(0.0)]], vec![]);
        p.section = Some(Section::Boardwalk { height: 80.0, spacing: 200.0 });
        let g = layout(&p, &pond, Sampling::Every(20.0)).unwrap();
        assert_eq!(g.runs.len(), 1);
        let r = &g.runs[0];
        assert!(r.floating && !r.land0 && !r.land1 && r.i0 == 0 && r.i1 == g.st.len() - 1, "{r:?}");
        assert!(g.st[0].z.abs() < 1e-9 && g.st.last().unwrap().z.abs() < 1e-9);
        assert!((g.z_at([0.0, 0.0]) - 80.0).abs() < 1e-6);
        // a plain floating bridge still lands
        p.section = None;
        p.mode = "floating".into();
        let g = layout(&p, &pond, Sampling::Every(20.0)).unwrap();
        assert!(g.runs[0].land0 && g.runs[0].land1);
    }

    #[test]
    fn too_tight_a_turn_is_refused() {
        let p = path(
            vec![vec![Some(0.0), Some(0.0)], vec![Some(100.0), Some(0.0)], vec![Some(100.0), Some(60.0)], vec![Some(0.0), Some(60.0)]],
            vec![],
        );
        assert!(layout(&p, &|_| 0.0, Sampling::Every(30.0)).is_err());
    }
}
