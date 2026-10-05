//! Plane geometry helpers: points are `[x, y]` (x east, y north), heights are separate.

pub type P2 = [f64; 2];
pub type P3 = [f64; 3];

pub fn sub(a: P2, b: P2) -> P2 {
    [a[0] - b[0], a[1] - b[1]]
}

pub fn dist(a: P2, b: P2) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

pub fn lerp(a: P2, b: P2, t: f64) -> P2 {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

pub fn cross(o: P2, a: P2, b: P2) -> f64 {
    (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
}

/// Positive for counter-clockwise loops.
pub fn signed_area(pts: &[P2]) -> f64 {
    let n = pts.len();
    let mut s = 0.0;
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        s += a[0] * b[1] - b[0] * a[1];
    }
    s * 0.5
}

pub fn point_in_poly(p: P2, poly: &[P2]) -> bool {
    let n = poly.len();
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (poly[i], poly[j]);
        if (a[1] > p[1]) != (b[1] > p[1]) {
            let x = a[0] + (p[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0]);
            if p[0] < x {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

/// Distance from p to segment ab, and the parameter of the nearest point (0 at a, 1 at b).
pub fn dist_to_seg(p: P2, a: P2, b: P2) -> (f64, f64) {
    let d = sub(b, a);
    let l2 = d[0] * d[0] + d[1] * d[1];
    let t = if l2 < 1e-12 { 0.0 } else { (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / l2).clamp(0.0, 1.0) };
    (dist(p, lerp(a, b, t)), t)
}

/// Distance from p to a closed polygon's outline.
pub fn dist_to_loop(p: P2, pts: &[P2]) -> f64 {
    let n = pts.len();
    (0..n).map(|i| dist_to_seg(p, pts[i], pts[(i + 1) % n]).0).fold(f64::INFINITY, f64::min)
}

/// The x positions where a row at height y crosses a closed polygon's edges, sorted, by the
/// same rule as `point_in_poly`: p is inside when an odd number of them lie right of p[0].
pub fn row_crossings(poly: &[P2], y: f64) -> Vec<f64> {
    let n = poly.len();
    let mut xs = vec![];
    let mut j = n.wrapping_sub(1);
    for i in 0..n {
        let (a, b) = (poly[i], poly[j]);
        if (a[1] > y) != (b[1] > y) {
            xs.push(a[0] + (y - a[1]) / (b[1] - a[1]) * (b[0] - a[0]));
        }
        j = i;
    }
    xs.sort_by(f64::total_cmp);
    xs
}

/// Inside test against a row's `row_crossings`: the same answer as `point_in_poly`.
pub fn inside_row(xs: &[f64], x: f64) -> bool {
    let right = xs.len() - xs.partition_point(|&c| c <= x);
    right % 2 == 1
}

/// Closed loops' segments in grid buckets, for distances that only matter within a reach.
pub struct SegIndex {
    cell: f64,
    segs: Vec<(P2, P2)>,
    buckets: std::collections::HashMap<(i64, i64), Vec<u32>>,
}

impl SegIndex {
    /// `cell` about the reach the queries will use.
    pub fn new(loops: &[&[P2]], cell: f64) -> SegIndex {
        let cell = cell.max(1.0);
        let mut segs = vec![];
        let mut buckets: std::collections::HashMap<(i64, i64), Vec<u32>> = Default::default();
        for lp in loops {
            let n = lp.len();
            for i in 0..n {
                let (a, b) = (lp[i], lp[(i + 1) % n]);
                let k = segs.len() as u32;
                segs.push((a, b));
                let (i0, i1) = ((a[0].min(b[0]) / cell).floor() as i64, (a[0].max(b[0]) / cell).floor() as i64);
                let (j0, j1) = ((a[1].min(b[1]) / cell).floor() as i64, (a[1].max(b[1]) / cell).floor() as i64);
                for bi in i0..=i1 {
                    for bj in j0..=j1 {
                        buckets.entry((bi, bj)).or_default().push(k);
                    }
                }
            }
        }
        SegIndex { cell, segs, buckets }
    }

    /// The distance from p to the nearest segment, exactly as `dist_to_loop` gives it when it's
    /// under `reach`; otherwise `reach` (or more).
    pub fn dist_within(&self, p: P2, reach: f64) -> f64 {
        let (i0, i1) = (((p[0] - reach) / self.cell).floor() as i64, ((p[0] + reach) / self.cell).floor() as i64);
        let (j0, j1) = (((p[1] - reach) / self.cell).floor() as i64, ((p[1] + reach) / self.cell).floor() as i64);
        let mut best = f64::INFINITY;
        for bi in i0..=i1 {
            for bj in j0..=j1 {
                if let Some(ks) = self.buckets.get(&(bi, bj)) {
                    for &k in ks {
                        let (a, b) = self.segs[k as usize];
                        best = best.min(dist_to_seg(p, a, b).0);
                    }
                }
            }
        }
        best.min(reach)
    }
}

/// True if segments ab and cd cross or overlap anywhere except at a shared end point.
pub fn segments_cross(a: P2, b: P2, c: P2, d: P2) -> bool {
    let eps = 1e-9;
    let shared = |p: P2, q: P2| dist(p, q) < 1e-6;
    let (d1, d2) = (cross(c, d, a), cross(c, d, b));
    let (d3, d4) = (cross(a, b, c), cross(a, b, d));
    if ((d1 > eps && d2 < -eps) || (d1 < -eps && d2 > eps)) && ((d3 > eps && d4 < -eps) || (d3 < -eps && d4 > eps)) {
        return true;
    }
    // touching or collinear: a point of one lying on the other, unless it's a shared end
    let on = |p: P2, q: P2, r: P2| {
        cross(p, q, r).abs() <= 1e-6 * dist(p, q).max(1.0)
            && r[0] >= p[0].min(q[0]) - eps && r[0] <= p[0].max(q[0]) + eps
            && r[1] >= p[1].min(q[1]) - eps && r[1] <= p[1].max(q[1]) + eps
    };
    for (p, q, r) in [(c, d, a), (c, d, b), (a, b, c), (a, b, d)] {
        if on(p, q, r) && !(shared(r, p) || shared(r, q)) {
            return true;
        }
    }
    false
}

/// Centripetal Catmull-Rom from p1 to p2 (p0 and p3 shape the tangents), `n` pieces:
/// returns n + 1 points, p1 first and p2 last, exactly.
pub fn catmull_rom(p0: P2, p1: P2, p2: P2, p3: P2, n: usize) -> Vec<P2> {
    let knot = |a: P2, b: P2| dist(a, b).sqrt().max(1e-6);
    let t0 = 0.0;
    let t1 = t0 + knot(p0, p1);
    let t2 = t1 + knot(p1, p2);
    let t3 = t2 + knot(p2, p3);
    let mix = |a: P2, b: P2, ta: f64, tb: f64, t: f64| lerp(a, b, (t - ta) / (tb - ta));
    let mut out = vec![p1];
    for k in 1..n {
        let t = t1 + (t2 - t1) * k as f64 / n as f64;
        let a1 = mix(p0, p1, t0, t1, t);
        let a2 = mix(p1, p2, t1, t2, t);
        let a3 = mix(p2, p3, t2, t3, t);
        let b1 = mix(a1, a2, t0, t2, t);
        let b2 = mix(a2, a3, t1, t3, t);
        out.push(mix(b1, b2, t1, t2, t));
    }
    out.push(p2);
    out
}

/// How a curve is sampled: every so often (the same everywhere), or adaptively, keeping only
/// the points needed to stay within `tol` of the true curve, with no piece longer than `max`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sampling {
    Every(f64),
    Within { tol: f64, max: f64 },
}

/// The centripetal Catmull-Rom curve from p1 to p2 (p0 and p3 shape the tangents), sampled:
/// p1 first and p2 last, exactly.
pub fn sample_curve(p0: P2, p1: P2, p2: P2, p3: P2, s: Sampling) -> Vec<P2> {
    match s {
        Sampling::Every(d) => catmull_rom(p0, p1, p2, p3, ((dist(p1, p2) / d).ceil() as usize).max(1)),
        Sampling::Within { tol, max } => {
            let dense = catmull_rom(p0, p1, p2, p3, ((dist(p1, p2) / 8.0).ceil() as usize).clamp(4, 4000));
            let last = dense.len() - 1;
            let mut keep = vec![false; dense.len()];
            keep[0] = true;
            keep[last] = true;
            dp_mark(&dense, 0, last, tol, &mut keep);
            let mut arc = vec![0.0];
            for w in dense.windows(2) {
                arc.push(arc.last().unwrap() + dist(w[0], w[1]));
            }
            let kept: Vec<usize> = (0..dense.len()).filter(|&i| keep[i]).collect();
            let mut out = vec![dense[0]];
            for w in kept.windows(2) {
                let (i, j) = (w[0], w[1]);
                // long pieces are cut evenly, at dense points
                let n = ((arc[j] - arc[i]) / max.max(1.0)).ceil() as usize;
                let mut k = i;
                for m in 1..n.max(1) {
                    let target = arc[i] + (arc[j] - arc[i]) * m as f64 / n as f64;
                    while k < j && arc[k] < target {
                        k += 1;
                    }
                    if k > i && k < j && out.last() != Some(&dense[k]) {
                        out.push(dense[k]);
                    }
                }
                out.push(dense[j]);
            }
            out
        }
    }
}

/// Marks the points Douglas-Peucker keeps between a and b.
fn dp_mark(pts: &[P2], a: usize, b: usize, tol: f64, keep: &mut [bool]) {
    if b <= a + 1 {
        return;
    }
    let (mut far, mut dmax) = (a, -1.0);
    for i in a + 1..b {
        let d = dist_to_seg(pts[i], pts[a], pts[b]).0;
        if d > dmax {
            dmax = d;
            far = i;
        }
    }
    if dmax > tol {
        keep[far] = true;
        dp_mark(pts, a, far, tol, keep);
        dp_mark(pts, far, b, tol, keep);
    }
}

/// Douglas-Peucker on an open polyline (end points kept).
pub fn douglas_peucker(pts: &[P2], tol: f64) -> Vec<P2> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let (a, b) = (pts[0], pts[pts.len() - 1]);
    let (mut far, mut dmax) = (0, -1.0);
    for (i, &p) in pts.iter().enumerate().take(pts.len() - 1).skip(1) {
        let d = dist_to_seg(p, a, b).0;
        if d > dmax {
            dmax = d;
            far = i;
        }
    }
    if dmax <= tol {
        return vec![a, b];
    }
    let mut left = douglas_peucker(&pts[..=far], tol);
    let right = douglas_peucker(&pts[far..], tol);
    left.pop();
    left.extend(right);
    left
}

/// Douglas-Peucker on a closed loop: split at the point farthest from the first.
pub fn simplify_closed(pts: &[P2], tol: f64) -> Vec<P2> {
    let far = (0..pts.len()).max_by(|&i, &j| dist(pts[i], pts[0]).total_cmp(&dist(pts[j], pts[0]))).unwrap_or(0);
    let mut a = douglas_peucker(&pts[..=far], tol);
    let mut tail = pts[far..].to_vec();
    tail.push(pts[0]);
    let b = douglas_peucker(&tail, tol);
    a.pop();
    a.extend_from_slice(&b[..b.len() - 1]);
    a
}

/// A closed loop moved `d` to the right of its walking direction, with mitred joints.
pub fn offset_right(pts: &[P2], d: f64) -> Vec<P2> {
    let n = pts.len();
    let right = |a: P2, b: P2| {
        let v = sub(b, a);
        let l = v[0].hypot(v[1]).max(1e-9);
        [v[1] / l, -v[0] / l]
    };
    (0..n)
        .map(|i| {
            let (a, b) = (right(pts[(i + n - 1) % n], pts[i]), right(pts[i], pts[(i + 1) % n]));
            let m = [a[0] + b[0], a[1] + b[1]];
            let l = m[0].hypot(m[1]).max(1e-9);
            let m = [m[0] / l, m[1] / l];
            let cosa = (m[0] * a[0] + m[1] * a[1]).max(0.5);
            [pts[i][0] + m[0] * d / cosa, pts[i][1] + m[1] * d / cosa]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catmull_rom_hits_its_ends() {
        let p = catmull_rom([0.0, 0.0], [10.0, 0.0], [20.0, 5.0], [30.0, 5.0], 4);
        assert_eq!(p.len(), 5);
        assert_eq!(p[0], [10.0, 0.0]);
        assert_eq!(p[4], [20.0, 5.0]);
    }

    #[test]
    fn crossing_and_shared_ends() {
        assert!(segments_cross([0.0, 0.0], [10.0, 10.0], [0.0, 10.0], [10.0, 0.0]));
        assert!(!segments_cross([0.0, 0.0], [10.0, 0.0], [10.0, 0.0], [10.0, 10.0]));
        assert!(segments_cross([0.0, 0.0], [10.0, 0.0], [5.0, 0.0], [5.0, 10.0])); // T touch
    }

    #[test]
    fn simplify_keeps_corners() {
        let sq: Vec<P2> = (0..40)
            .map(|i| {
                let t = i as f64 / 10.0;
                match i / 10 {
                    0 => [t * 10.0, 0.0],
                    1 => [10.0, (t - 1.0) * 10.0],
                    2 => [10.0 - (t - 2.0) * 10.0, 10.0],
                    _ => [0.0, 10.0 - (t - 3.0) * 10.0],
                }
            })
            .collect();
        assert_eq!(simplify_closed(&sq, 0.1).len(), 4);
    }
}
