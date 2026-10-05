//! Painted terrain: a smooth height offset over the whole level, kept as a grid of offsets and
//! changed with brushes (raise, lower, smooth, flatten, bumps, erase).
//!
//! It's applied to the finished level as a deformation: every vertex moves up by the offset at
//! its (x, y). Shared vertices therefore stay shared (the level stays watertight), and walls,
//! ramps, bridges and the rim keep their shapes and heights relative to the ground around them:
//! a hill painted under a plateau lifts the plateau with it. The one exception is water, which
//! has to stay level: a pond takes the offset at its middle all over, blending back to the
//! painted offset within `POND_BLEND` of its shore.

use crate::doc::Doc;
use crate::geom::*;
use crate::noise::relief;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Grid nodes per chunk side.
pub const CHUNK: i64 = 16;
/// How far round a pond the painted offset blends to the pond's own.
pub const POND_BLEND: f64 = 250.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Terrain {
    /// Grid spacing: node (i, j) is at (i * cell, j * cell).
    pub cell: f64,
    /// Floors get extra points this far apart where the terrain isn't flat.
    pub detail: f64,
    /// Offsets at the grid nodes, in chunks of CHUNK x CHUNK nodes keyed "cx,cy", row by row
    /// (x fastest). Missing chunks are flat.
    pub chunks: BTreeMap<String, Vec<f64>>,
}

impl Default for Terrain {
    fn default() -> Self {
        Terrain { cell: 50.0, detail: 100.0, chunks: BTreeMap::new() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Raise,
    Lower,
    Smooth,
    /// Towards the offset where the stroke started.
    Flatten,
    /// Adds smooth noise.
    Bumps,
    /// Back to no offset.
    Erase,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Brush {
    pub mode: Mode,
    pub radius: f64,
    /// Raise, lower and bumps: units per second at the centre. Smooth, flatten and erase: how
    /// fast they act (strength / 60 of the way per second at the centre).
    pub strength: f64,
    /// The share of the radius at full strength (0: soft all the way, 1: a hard edge).
    pub falloff: f64,
    /// Bumps: feature size.
    pub bump_scale: f64,
    pub seed: u32,
}

impl Default for Brush {
    fn default() -> Self {
        Brush { mode: Mode::Raise, radius: 400.0, strength: 120.0, falloff: 0.3, bump_scale: 500.0, seed: 0 }
    }
}

impl Brush {
    /// Strength at distance d from the centre: 1 inside the full-strength core, easing to 0 at
    /// the radius.
    pub fn weight(&self, d: f64) -> f64 {
        let r = self.radius.max(1e-6);
        if d >= r {
            return 0.0;
        }
        let inner = r * self.falloff.clamp(0.0, 0.99);
        if d <= inner {
            return 1.0;
        }
        let t = (d - inner) / (r - inner);
        1.0 - t * t * (3.0 - 2.0 * t)
    }
}

fn key(cx: i64, cy: i64) -> String {
    format!("{cx},{cy}")
}

impl Terrain {
    pub fn node(&self, i: i64, j: i64) -> f64 {
        match self.chunks.get(&key(i.div_euclid(CHUNK), j.div_euclid(CHUNK))) {
            Some(c) => c[(j.rem_euclid(CHUNK) * CHUNK + i.rem_euclid(CHUNK)) as usize],
            None => 0.0,
        }
    }

    pub fn set(&mut self, i: i64, j: i64, v: f64) {
        let k = key(i.div_euclid(CHUNK), j.div_euclid(CHUNK));
        if v == 0.0 && !self.chunks.contains_key(&k) {
            return;
        }
        let c = self.chunks.entry(k).or_insert_with(|| vec![0.0; (CHUNK * CHUNK) as usize]);
        c[(j.rem_euclid(CHUNK) * CHUNK + i.rem_euclid(CHUNK)) as usize] = v;
    }

    /// The offset at p, bilinear between the grid nodes.
    pub fn at(&self, p: P2) -> f64 {
        let (x, y) = (p[0] / self.cell, p[1] / self.cell);
        let (i, j) = (x.floor() as i64, y.floor() as i64);
        let (fx, fy) = (x - i as f64, y - j as f64);
        let (a, b, c, d) = (self.node(i, j), self.node(i + 1, j), self.node(i, j + 1), self.node(i + 1, j + 1));
        (a * (1.0 - fx) + b * fx) * (1.0 - fy) + (c * (1.0 - fx) + d * fx) * fy
    }

    pub fn is_empty(&self) -> bool {
        self.chunks.values().all(|c| c.iter().all(|&v| v == 0.0))
    }

    /// Whether the terrain varies anywhere within `r` of p (so a floor there needs points).
    pub fn busy(&self, p: P2, r: f64) -> bool {
        let n = (r / self.cell).ceil() as i64 + 1;
        let (i, j) = ((p[0] / self.cell).floor() as i64, (p[1] / self.cell).floor() as i64);
        let first = self.node(i, j);
        (i - n..=i + n).any(|a| (j - n..=j + n).any(|b| (self.node(a, b) - first).abs() > 0.05))
    }

    /// Drops chunks that are all zero.
    pub fn prune(&mut self) {
        self.chunks.retain(|_, c| c.iter().any(|&v| v != 0.0));
    }

    /// One brush dab at c over `dt` seconds. `target` is the flatten height (the offset where
    /// the stroke started).
    pub fn paint(&mut self, b: &Brush, c: P2, dt: f64, target: f64) {
        let r = b.radius;
        let (i0, i1) = (((c[0] - r) / self.cell).floor() as i64, ((c[0] + r) / self.cell).ceil() as i64);
        let (j0, j1) = (((c[1] - r) / self.cell).floor() as i64, ((c[1] + r) / self.cell).ceil() as i64);
        // smoothing averages over a kernel that grows with the brush, so big brushes smooth big hills
        let kh = ((r / (4.0 * self.cell)).round() as i64).clamp(1, 4);
        let rate = |w: f64| (dt * w * b.strength / 60.0).clamp(0.0, 1.0);
        let mut out = vec![];
        for i in i0..=i1 {
            for j in j0..=j1 {
                let p = [i as f64 * self.cell, j as f64 * self.cell];
                let w = b.weight(dist(p, c));
                if w <= 0.0 {
                    continue;
                }
                let v = self.node(i, j);
                let nv = match b.mode {
                    Mode::Raise => v + b.strength * dt * w,
                    Mode::Lower => v - b.strength * dt * w,
                    Mode::Bumps => v + b.strength * dt * w * relief([p[0] / b.bump_scale.max(1.0), p[1] / b.bump_scale.max(1.0)], b.seed),
                    Mode::Smooth => {
                        let mut sum = 0.0;
                        for a in -kh..=kh {
                            for e in -kh..=kh {
                                sum += self.node(i + a, j + e);
                            }
                        }
                        let avg = sum / ((2 * kh + 1) * (2 * kh + 1)) as f64;
                        v + (avg - v) * rate(w)
                    }
                    Mode::Flatten => v + (target - v) * rate(w),
                    Mode::Erase => v * (1.0 - rate(w)),
                };
                // hundredths, so near-zero leftovers become zero and chunks can be dropped
                out.push((i, j, (nv * 100.0).round() / 100.0));
            }
        }
        for (i, j, v) in out {
            self.set(i, j, v);
        }
        self.prune();
    }
}

/// The terrain as applied to a level: the painted offsets, held level over ponds.
pub struct Field<'a> {
    t: &'a Terrain,
    /// Each pond's polygon and the one offset its water takes.
    ponds: Vec<(Vec<P2>, f64)>,
}

impl<'a> Field<'a> {
    /// `polys` are the document's loops as the builder samples them (0 = outline, i + 1 =
    /// region i): `map::sample_loops(doc).polys` or `Map::loop_polys`.
    pub fn new(t: &'a Terrain, doc: &Doc, polys: &[Vec<P2>]) -> Field<'a> {
        let ponds = doc
            .regions
            .iter()
            .enumerate()
            .filter(|(i, r)| r.kind == "water" && polys.get(i + 1).is_some_and(|p| p.len() >= 3))
            .map(|(i, _)| {
                let poly = polys[i + 1].clone();
                let n = poly.len() as f64;
                let m = [poly.iter().map(|q| q[0]).sum::<f64>() / n, poly.iter().map(|q| q[1]).sum::<f64>() / n];
                let c = t.at(m);
                (poly, c)
            })
            .collect();
        Field { t, ponds }
    }

    pub fn at(&self, p: P2) -> f64 {
        let mut v = self.t.at(p);
        for (poly, c) in &self.ponds {
            if point_in_poly(p, poly) {
                return *c;
            }
            let d = dist_to_loop(p, poly);
            if d < POND_BLEND {
                let s = d / POND_BLEND;
                v = c + (v - c) * s * s * (3.0 - 2.0 * s);
            }
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nodes_live_in_chunks_on_both_sides_of_zero() {
        let mut t = Terrain::default();
        for (i, j) in [(0, 0), (-1, -1), (15, 16), (-17, 3)] {
            t.set(i, j, (i * 10 + j + 1000) as f64);
        }
        for (i, j) in [(0, 0), (-1, -1), (15, 16), (-17, 3)] {
            assert_eq!(t.node(i, j), (i * 10 + j + 1000) as f64);
        }
        assert_eq!(t.chunks.len(), 4);
        // bilinear between nodes
        let mut t = Terrain::default();
        t.set(1, 0, 10.0);
        assert!((t.at([25.0, 0.0]) - 5.0).abs() < 1e-9 && t.at([50.0, 0.0]) == 10.0 && t.at([-10.0, 0.0]) == 0.0);
    }

    #[test]
    fn brushes_raise_smooth_flatten_and_erase() {
        let mut t = Terrain::default();
        let b = Brush { radius: 300.0, strength: 100.0, falloff: 0.3, ..Default::default() };
        for _ in 0..30 {
            t.paint(&b, [0.0, 0.0], 1.0 / 30.0, 0.0);
        }
        // a hill: about 100 high in the middle, lower towards the edge, nothing outside
        assert!((t.at([0.0, 0.0]) - 100.0).abs() < 1.0, "{}", t.at([0.0, 0.0]));
        assert!(t.at([200.0, 0.0]) > 5.0 && t.at([200.0, 0.0]) < 90.0);
        assert_eq!(t.at([400.0, 0.0]), 0.0);
        // smoothing takes the top off
        let s = Brush { mode: Mode::Smooth, strength: 300.0, ..b };
        for _ in 0..60 {
            t.paint(&s, [0.0, 0.0], 1.0 / 30.0, 0.0);
        }
        assert!(t.at([0.0, 0.0]) < 95.0);
        // flatten levels to the target
        let f = Brush { mode: Mode::Flatten, strength: 600.0, falloff: 0.9, ..b };
        for _ in 0..120 {
            t.paint(&f, [0.0, 0.0], 1.0 / 30.0, 40.0);
        }
        assert!((t.at([100.0, 50.0]) - 40.0).abs() < 1.0);
        // erase everything: the chunks go
        let e = Brush { mode: Mode::Erase, radius: 1000.0, strength: 6000.0, falloff: 0.9, ..b };
        for _ in 0..30 {
            t.paint(&e, [0.0, 0.0], 1.0 / 30.0, 0.0);
        }
        assert!(t.is_empty() && t.chunks.is_empty(), "{:?}", t.chunks.keys());
    }

    #[test]
    fn bumps_go_both_ways() {
        let mut t = Terrain::default();
        let b = Brush { mode: Mode::Bumps, radius: 1500.0, strength: 60.0, falloff: 0.8, bump_scale: 300.0, ..Default::default() };
        t.paint(&b, [0.0, 0.0], 1.0, 0.0);
        let vals: Vec<f64> = (-20..=20).flat_map(|i| (-20..=20).map(move |j| (i, j))).map(|(i, j)| t.node(i, j)).collect();
        assert!(vals.iter().any(|&v| v > 20.0) && vals.iter().any(|&v| v < -20.0));
    }
}
