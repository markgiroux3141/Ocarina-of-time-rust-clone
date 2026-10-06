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

use crate::doc::Path;
use crate::geom::*;

#[derive(Debug, Clone)]
pub struct Station {
    pub p: P2,
    pub s: f64,
    pub z: f64,
    pub w: f64,
    /// Unit direction along the line.
    pub dir: P2,
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
}

impl PathGeo {
    /// The path's height at the centre-line point nearest p.
    pub fn z_at(&self, p: P2) -> f64 {
        let (mut best, mut z) = (f64::INFINITY, self.st[0].z);
        for w in self.st.windows(2) {
            let (d, t) = dist_to_seg(p, w[0].p, w[1].p);
            if d < best {
                best = d;
                z = w[0].z + (w[1].z - w[0].z) * t;
            }
        }
        z
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
        let s = &self.st[i];
        let n = [-s.dir[1] * s.w * 0.5, s.dir[0] * s.w * 0.5];
        ([s.p[0] + n[0], s.p[1] + n[1]], [s.p[0] - n[0], s.p[1] - n[1]])
    }

    /// A run's footprint, counter-clockwise: the right side forwards, the left side back.
    pub fn ribbon(&self, r: &Run) -> Vec<P2> {
        let mut out: Vec<P2> = (r.i0..=r.i1).map(|i| self.sides(i).1).collect();
        out.extend((r.i0..=r.i1).rev().map(|i| self.sides(i).0));
        out
    }
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
    let floating: Vec<bool> = (0..n - 1)
        .map(|k| match path.modes.get(k).map(String::as_str).unwrap_or(path.mode.as_str()) {
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
    for k in [0, n - 1] {
        if zk[k].is_none() {
            zk[k] = Some(base(xy[k]));
        }
    }
    // landing (floating ends): walk in from each end until the ground under the line falls below
    // the end's height
    let mut land: [Option<f64>; 2] = [None, None];
    for (e, (k, inward)) in [(0usize, true), (n - 1, false)].into_iter().enumerate() {
        if !floating[if inward { 0 } else { n - 2 }] {
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
    // the stations, with the landing points added
    let mut cuts: Vec<f64> = land.iter().flatten().copied().filter(|&s| s > 1e-6 && s < total - 1e-6).collect();
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
    // heights: linear between nodes that have one; landed ends reach their height at the landing
    let mut keys: Vec<(f64, f64)> = vec![];
    for k in 0..n {
        if let Some(z) = zk[k] {
            let s = if k == 0 { land[0].unwrap_or(node_s[k]) } else if k == n - 1 { land[1].unwrap_or(node_s[k]) } else { node_s[k] };
            keys.push((s, z));
        }
    }
    keys.sort_by(|a, b| a.0.total_cmp(&b.0));
    let z_of = |s: f64| -> f64 {
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
    let m = st_s.len();
    let st: Vec<Station> = (0..m)
        .map(|i| {
            let (a, b) = (st_s[i.saturating_sub(1)].0, st_s[(i + 1).min(m - 1)].0);
            let d = sub(b, a);
            let l = d[0].hypot(d[1]).max(1e-9);
            Station { p: st_s[i].0, s: st_s[i].1, z: z_of(st_s[i].1), w: w_of(st_s[i].1), dir: [d[0] / l, d[1] / l] }
        })
        .collect();
    let at = |s: f64| -> usize { st.iter().position(|x| (x.s - s).abs() < 1e-6).unwrap_or(0) };
    // runs of one kind; floating runs start and end at landings, and need no end face where
    // they land or continue an embankment
    let mut runs: Vec<Run> = vec![];
    let mut k = 0;
    while k < n - 1 {
        let mut e = k;
        while e + 1 < n - 1 && floating[e + 1] == floating[k] {
            e += 1;
        }
        let (mut i0, mut i1) = (at(node_s[k]), at(node_s[e + 1]));
        // runs alternate kinds, so a floating run's inner ends continue an embankment
        let (mut cap0, mut cap1) = (k == 0, e + 1 == n - 1);
        let (mut land0, mut land1) = (false, false);
        if floating[k] {
            if k == 0 {
                if let Some(s) = land[0] {
                    i0 = at(s);
                    cap0 = false;
                    land0 = true;
                }
            }
            if e + 1 == n - 1 {
                if let Some(s) = land[1] {
                    i1 = at(s);
                    cap1 = false;
                    land1 = true;
                }
            }
        }
        runs.push(Run { floating: floating[k], i0, i1, cap0, cap1, land0, land1 });
        k = e + 1;
    }
    let max_slope = st
        .windows(2)
        .map(|w| ((w[1].z - w[0].z).abs() / (w[1].s - w[0].s).max(1e-9)).atan().to_degrees())
        .fold(0.0, f64::max);
    let geo = PathGeo { name: name.clone(), st, runs, edge: path.edge.clone(), shape: path.shape.clone(), max_slope };
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
        Path { name: "t".into(), nodes, width: 100.0, mode: "attached".into(), modes: modes.into_iter().map(String::from).collect(), edge: None, shape: None }
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
    fn too_tight_a_turn_is_refused() {
        let p = path(
            vec![vec![Some(0.0), Some(0.0)], vec![Some(100.0), Some(0.0)], vec![Some(100.0), Some(60.0)], vec![Some(0.0), Some(60.0)]],
            vec![],
        );
        assert!(layout(&p, &|_| 0.0, Sampling::Every(30.0)).is_err());
    }
}
