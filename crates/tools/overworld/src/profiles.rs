//! Edge profiles: how a region's edge meets the floor beside it, when it isn't a cliff.
//!
//! A profile is built **inward** from the region's edge, so the region's footprint stays exactly as
//! drawn and nothing beside it moves. Each profiled edge is a *foot*: at the edge the region's floor
//! is at the height of the floor beside it (`n`), and it rises (or, for a sunken region or a pond's
//! bed, falls) to the region's own height going in. Where an edge is shared by two regions that both
//! profile it, the higher one's profile wins, as the higher side owns a wall.
//!
//! - **Slope:** the floor follows a slope from `n` at the foot to the region's height, at most the
//!   profile's angle steep (`round` eases its crest and foot). It's a smooth height field over the
//!   region's floor faces, so nothing changes in the map: floors take extra points along lines
//!   across the slope (`rings`), and every vertex takes the field's height.
//! - **Terraces:** treads `depth` deep, `rise` apart. Their step lines are cut into the map
//!   (`cuts`), so each tread is a face of its own, flat, and the risers between them are walls like
//!   any other: textured by the theme's rules, and watertight by construction.
//!
//! Near several feet, the region's floor takes the lowest of what the raised feet give and the
//! highest of what the sunken ones give: the field is continuous. How far in a point is depends on
//! `settings.edges`: with smooth edges it's the plain distance, so the steps and slopes round off
//! round a corner and the end of a profiled stretch; with hard or faceted edges it's mitred
//! (`Foot::dist`), so they keep sharp corners, as the edges do.
//!
//! Overhangs and ragged rock don't change the floors: they bend the cliff on the edge (`Wall`,
//! looked up by `Field::wall`; `build.rs` bends the wall's columns).

use crate::doc::{Doc, Profile};
use crate::geom::*;
use crate::map::{Map, VOID};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
enum Kind {
    Slope { run: f64, round: f64 },
    Steps { steps: u32, rise: f64, depth: f64 },
}

/// A profiled piece of a region's edge, with the height of the floor beside it.
#[derive(Clone, Copy, Debug)]
struct Foot {
    a: P2,
    b: P2,
    n: f64,
    kind: Kind,
    /// Mitred distances (hard or faceted edges): at each end, the tangent half way between this
    /// piece's and the next profiled piece's (the joint's bisector), or None at a free end.
    mitre: Option<[Option<P2>; 2]>,
    /// Which loop piece it is (to find its neighbours).
    m: usize,
}

impl Foot {
    /// How far in the profile reaches (beyond, the region's own height).
    fn reach(&self) -> f64 {
        match self.kind {
            Kind::Slope { run, .. } => run,
            Kind::Steps { steps, depth, .. } => steps.saturating_sub(1) as f64 * depth,
        }
    }

    fn stepped(&self) -> bool {
        matches!(self.kind, Kind::Steps { .. })
    }

    /// The floor's height less the region's, `d` in from this foot.
    fn dev(&self, top: f64, d: f64) -> f64 {
        match self.kind {
            Kind::Slope { run, round } => {
                let s = (d / run).clamp(0.0, 1.0);
                let g = s + (s * s * (3.0 - 2.0 * s) - s) * round;
                (self.n - top) * (1.0 - g)
            }
            Kind::Steps { steps, rise, depth } => {
                let j = (d / depth).floor().max(0.0) as u32;
                let below = steps.saturating_sub(1).saturating_sub(j) as f64;
                (self.n - top).signum() * (below * rise).min((self.n - top).abs())
            }
        }
    }

    /// How far p is in from this foot. Plain, or mitred: off the piece's line square, past a joint
    /// only on this piece's side of its bisector (the next piece measures the rest), and round a
    /// free end square (the larger of how far out and how far past it).
    fn dist(&self, p: P2) -> f64 {
        let Some(ends) = self.mitre else { return dist_to_seg(p, self.a, self.b).0 };
        let d = sub(self.b, self.a);
        let len = d[0].hypot(d[1]).max(1e-9);
        let u = [d[0] / len, d[1] / len];
        let q = sub(p, self.a);
        let t = q[0] * u[0] + q[1] * u[1];
        let perp = (q[0] * u[1] - q[1] * u[0]).abs();
        if t < 0.0 {
            match ends[0] {
                Some(n) if q[0] * n[0] + q[1] * n[1] >= -1e-9 => perp,
                Some(_) => f64::INFINITY,
                None => perp.max(-t),
            }
        } else if t > len {
            let r = sub(p, self.b);
            match ends[1] {
                Some(n) if r[0] * n[0] + r[1] * n[1] <= 1e-9 => perp,
                Some(_) => f64::INFINITY,
                None => perp.max(t - len),
            }
        } else {
            perp
        }
    }

    /// The distance in over the profile's scale: 1 at a slope's crest, k at a terrace's k-th step.
    fn scaled(&self, p: P2) -> f64 {
        let d = self.dist(p);
        match self.kind {
            Kind::Slope { run, .. } => d / run,
            Kind::Steps { depth, .. } => d / depth,
        }
    }
}

/// How a cliff on a profiled edge is bent (`Profile::Overhang`, `Profile::Ragged`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Wall {
    /// Undercut `depth` into the higher side, the lip curving back out at the top.
    Overhang { depth: f64 },
    /// The face pushed in and out by smooth noise.
    Ragged { amplitude: f64, scale: f64, seed: u32 },
}

/// One region's feet, bucketed by where they reach.
struct RegionField {
    top: f64,
    feet: Vec<Foot>,
    cell: f64,
    buckets: HashMap<(i64, i64), Vec<u32>>,
    /// The region's box.
    bb: [f64; 4],
    /// The slopes' and the terraces' longest reach in.
    reach: [f64; 2],
}

impl RegionField {
    fn near(&self, p: P2) -> impl Iterator<Item = &Foot> {
        let c = self.cell;
        let (i, j) = ((p[0] / c).floor() as i64, (p[1] / c).floor() as i64);
        (i - 1..=i + 1).flat_map(move |bi| (j - 1..=j + 1).map(move |bj| (bi, bj))).filter_map(|k| self.buckets.get(&k)).flatten().map(|&k| &self.feet[k as usize])
    }
}

/// Every region's profiled edges (`Field::new`), and the floor heights they give.
pub struct Field {
    zs: Vec<f64>,
    regions: Vec<Option<RegionField>>,
    loop_areas: Vec<f64>,
    /// The loop pieces whose cliffs are bent, in buckets 100 across.
    walls: Vec<(P2, P2, Wall)>,
    wall_buckets: HashMap<(i64, i64), Vec<u32>>,
    /// Hard or faceted edges: distances are mitred, and the traced lines get their corners back.
    mitre: bool,
}

impl Field {
    /// From the map of the document's loops alone (`Map::build`: no paths' ribbons yet), with each
    /// region's height (`zs[0]` the outline's). Profiles steeper than `max_slope` are reported.
    pub fn new(doc: &Doc, map: &Map, zs: &[f64], max_slope: f64, problems: &mut Vec<String>) -> Result<Field, String> {
        let mut half: HashMap<(usize, usize), usize> = HashMap::new();
        for (s, &[a, b]) in map.segs.iter().enumerate() {
            half.insert((a, b), 2 * s);
            half.insert((b, a), 2 * s + 1);
        }
        let name = |l: usize| {
            let r = &doc.regions[l - 1];
            if r.name.is_empty() { format!("region {}", l - 1) } else { format!("region {} ({})", l - 1, r.name) }
        };
        // each loop piece's profile, and who else profiles the same piece
        let n_loops = map.loops.len();
        let mut owners: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
        let mut prof: Vec<Vec<Option<&Profile>>> = vec![vec![]; n_loops];
        for l in 1..n_loops {
            let r = &doc.regions[l - 1];
            for p in r.profile.iter().chain(r.profiles.iter().flatten()) {
                check(p).map_err(|e| format!("{}: {e}", name(l)))?;
            }
            let lp = &map.loops[l];
            prof[l] = (0..lp.len()).map(|m| r.edge_profile(map.loop_edges[l][m])).collect();
            for m in 0..lp.len() {
                if prof[l][m].is_some() {
                    let (u, v) = (lp[m], lp[(m + 1) % lp.len()]);
                    owners.entry((u.min(v), u.max(v))).or_default().push(l);
                }
            }
        }
        let mut regions: Vec<Option<RegionField>> = (0..n_loops).map(|_| None).collect();
        let mut walls: Vec<(P2, P2, Wall)> = vec![];
        for l in 1..n_loops {
            let lp = &map.loops[l];
            let ccw = signed_area(&map.loop_polys[l]) > 0.0;
            let top = zs[l];
            let mut feet = vec![];
            let mut steep = 0.0f64;
            for m in 0..lp.len() {
                let Some(p) = prof[l][m] else { continue };
                let (u, v) = (lp[m], lp[(m + 1) % lp.len()]);
                // a higher region profiling the same piece wins
                if owners[&(u.min(v), u.max(v))].iter().any(|&o| o != l && zs[o] > top) {
                    continue;
                }
                let Some(&h) = half.get(&(u, v)) else { continue };
                let (fin, fout) = if ccw { (map.half_face[h], map.half_face[h ^ 1]) } else { (map.half_face[h ^ 1], map.half_face[h]) };
                // the region's own floor on the inside (not a region drawn against it), a floor outside
                if fin == VOID || fout == VOID || map.faces[fin].region != l {
                    continue;
                }
                let n = zs[map.faces[fout].region];
                let drop = (top - n).abs();
                if drop < 0.5 {
                    continue;
                }
                let kind = match *p {
                    Profile::Overhang { depth } => {
                        walls.push((map.verts[u], map.verts[v], Wall::Overhang { depth }));
                        continue;
                    }
                    Profile::Ragged { amplitude, scale, seed } => {
                        walls.push((map.verts[u], map.verts[v], Wall::Ragged { amplitude, scale, seed: seed.wrapping_add(l as u32 * 7919) }));
                        continue;
                    }
                    Profile::Slope { angle, round } => {
                        steep = steep.max(angle);
                        let round = round.clamp(0.0, 1.0);
                        Kind::Slope { run: drop / angle.to_radians().tan() * (1.0 + 0.5 * round), round }
                    }
                    Profile::Terraces { steps, rise, depth } => Kind::Steps { steps, rise: rise.unwrap_or(drop / steps as f64), depth },
                    Profile::Cliff => continue,
                };
                feet.push(Foot { a: map.verts[u], b: map.verts[v], n, kind, mitre: None, m });
            }
            if feet.is_empty() {
                continue;
            }
            // hard or faceted edges: mitred joints between profiled pieces of the same kind
            if matches!(doc.settings.edges.as_str(), "hard" | "faceted") {
                let at: HashMap<usize, usize> = feet.iter().enumerate().map(|(i, f)| (f.m, i)).collect();
                let tangent = |f: &Foot| {
                    let d = sub(f.b, f.a);
                    let l = d[0].hypot(d[1]).max(1e-9);
                    [d[0] / l, d[1] / l]
                };
                let n = lp.len();
                let joints: Vec<[Option<P2>; 2]> = feet
                    .iter()
                    .map(|f| {
                        let t = tangent(f);
                        [(f.m + n - 1) % n, (f.m + 1) % n].map(|m| {
                            let g = &feet[*at.get(&m)?];
                            if g.stepped() != f.stepped() {
                                return None;
                            }
                            let s = tangent(g);
                            let h = [t[0] + s[0], t[1] + s[1]];
                            let l = h[0].hypot(h[1]);
                            (l > 1e-6).then(|| [h[0] / l, h[1] / l])
                        })
                    })
                    .collect();
                for (f, j) in feet.iter_mut().zip(joints) {
                    f.mitre = Some(j);
                }
            }
            if steep > max_slope + 1e-9 {
                problems.push(format!("{}: its slopes are {steep:.0} degrees at their steepest, over the walkable {max_slope:.0}", name(l)));
            }
            let reach = |stepped: bool| feet.iter().filter(|f| f.stepped() == stepped).map(Foot::reach).fold(0.0, f64::max);
            let reach = [reach(false), reach(true)];
            let cell = reach[0].max(reach[1]).max(20.0);
            let mut buckets: HashMap<(i64, i64), Vec<u32>> = HashMap::new();
            for (k, f) in feet.iter().enumerate() {
                let (i0, i1) = ((f.a[0].min(f.b[0]) / cell).floor() as i64, (f.a[0].max(f.b[0]) / cell).floor() as i64);
                let (j0, j1) = ((f.a[1].min(f.b[1]) / cell).floor() as i64, (f.a[1].max(f.b[1]) / cell).floor() as i64);
                for bi in i0..=i1 {
                    for bj in j0..=j1 {
                        buckets.entry((bi, bj)).or_default().push(k as u32);
                    }
                }
            }
            let bb = map.loop_polys[l].iter().fold([f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY], |b, q| [b[0].min(q[0]), b[1].min(q[1]), b[2].max(q[0]), b[3].max(q[1])]);
            regions[l] = Some(RegionField { top, feet, cell, buckets, bb, reach });
        }
        let loop_areas = map.loop_polys.iter().map(|p| signed_area(p).abs()).collect();
        let mut wall_buckets: HashMap<(i64, i64), Vec<u32>> = HashMap::new();
        for (k, &(a, b, _)) in walls.iter().enumerate() {
            for key in cells(a, b, WALL_CELL) {
                wall_buckets.entry(key).or_default().push(k as u32);
            }
        }
        let mitre = matches!(doc.settings.edges.as_str(), "hard" | "faceted");
        Ok(Field { zs: zs.to_vec(), regions, loop_areas, walls, wall_buckets, mitre })
    }

    /// How the cliff on the piece p -> q is bent, if it lies on a loop piece with an overhang or
    /// ragged rock (a piece of the final map may be part of one: cut by a path).
    pub fn wall(&self, p: P2, q: P2) -> Option<Wall> {
        let m = lerp(p, q, 0.5);
        let key = ((m[0] / WALL_CELL).floor() as i64, (m[1] / WALL_CELL).floor() as i64);
        self.wall_buckets.get(&key)?.iter().map(|&k| self.walls[k as usize]).find(|&(a, b, _)| dist_to_seg(p, a, b).0 < 1e-6 && dist_to_seg(q, a, b).0 < 1e-6).map(|w| w.2)
    }

    /// Whether any region has a profile.
    pub fn is_empty(&self) -> bool {
        self.regions.iter().all(Option::is_none)
    }

    /// Whether region r's floor has terraces (its faces are flat treads: see `z`).
    pub fn stepped(&self, r: usize) -> bool {
        self.regions.get(r).and_then(|x| x.as_ref()).is_some_and(|f| f.reach[1] > 0.0)
    }

    /// Region r's floor height at p. Terraces are read at `anchor` (a point well inside p's face:
    /// a tread is flat, and p may be on the step line between two), slopes at p.
    pub fn z(&self, r: usize, p: P2, anchor: P2) -> f64 {
        let Some(rf) = self.regions.get(r).and_then(|x| x.as_ref()) else { return self.zs[r] };
        let (mut down, mut up) = (0.0f64, 0.0f64);
        for (q, stepped) in [(p, false), (anchor, true)] {
            if rf.reach[usize::from(stepped)] <= 0.0 {
                continue;
            }
            for f in rf.near(q).filter(|f| f.stepped() == stepped) {
                let d = f.dist(q);
                if d >= f.reach() {
                    continue;
                }
                let dev = f.dev(rf.top, d);
                down = down.min(dev);
                up = up.max(dev);
            }
        }
        rf.top + down + up
    }

    /// The floor's height at p under the document's loops alone: the innermost loop's (its
    /// profiles included), or the outline's outside every loop. `polys`: the map's loop polygons.
    pub fn base(&self, polys: &[Vec<P2>], p: P2) -> f64 {
        (0..polys.len()).filter(|&l| point_in_poly(p, &polys[l])).min_by(|&x, &y| self.loop_areas[x].total_cmp(&self.loop_areas[y])).map_or(self.zs[0], |l| self.z(l, p, p))
    }

    /// The scaled distance in from the region's nearest foot of one kind (`Foot::scaled`), capped.
    fn scaled(rf: &RegionField, p: P2, stepped: bool, cap: f64) -> f64 {
        rf.near(p).filter(|f| f.stepped() == stepped).map(|f| f.scaled(p)).fold(cap, f64::min)
    }

    /// Terraces' step lines, cut into the map (`Map::build_with`'s cuts): for each region with
    /// terraces, the lines k treads in (k = 1 to the most steps less one), simplified within `tol`,
    /// clipped to where the region's own floor is (each piece ends on a loop's edge or closes).
    pub fn cuts(&self, map: &Map, tol: f64) -> Vec<Vec<P2>> {
        let edges = Edges::new(&map.loop_polys);
        let mut out = vec![];
        for (r, rf) in self.regions.iter().enumerate() {
            let Some(rf) = rf.as_ref().filter(|f| f.reach[1] > 0.0) else { continue };
            let most = rf.feet.iter().filter_map(|f| match f.kind {
                Kind::Steps { steps, .. } => Some(steps),
                _ => None,
            }).max().unwrap_or(1);
            let depth = rf.feet.iter().filter_map(|f| match f.kind {
                Kind::Steps { depth, .. } => Some(depth),
                _ => None,
            }).fold(f64::INFINITY, f64::min);
            let cell = (depth / 10.0).clamp(1.5, 12.0);
            let m = 2.0 * cell;
            let bb = [rf.bb[0] - m, rf.bb[1] - m, rf.bb[2] + m, rf.bb[3] + m];
            let grid = Grid::new(bb, cell, |p| Self::scaled(rf, p, true, most as f64));
            for k in 1..most {
                for line in grid.level_set(k as f64) {
                    let mut line = simplify_line(&line, tol, 0.0);
                    if self.mitre {
                        line = sharpen(&line, 2.5 * cell);
                    }
                    out.extend(clip(&line, &edges, &map.loop_polys, &self.loop_areas, r));
                }
            }
        }
        out
    }

    /// Lines across region r's slopes for its floors to follow, the last along the crest: about
    /// `spacing` apart over the longest slope (at least `rounded` of them over a rounded one, at
    /// most 8), simplified within `tol`, in pieces at most `max` long. Not clipped: the floors take
    /// the points inside them.
    pub fn rings(&self, r: usize, spacing: f64, rounded: usize, tol: f64, max: f64) -> Vec<Vec<P2>> {
        let Some(rf) = self.regions.get(r).and_then(|x| x.as_ref()).filter(|f| f.reach[0] > 0.0) else { return vec![] };
        let (mut run, mut longest, mut round) = (f64::INFINITY, 0.0f64, false);
        for f in &rf.feet {
            if let Kind::Slope { run: r, round: o } = f.kind {
                run = run.min(r);
                longest = longest.max(r);
                round |= o > 0.0;
            }
        }
        let per = ((longest / spacing.max(1.0)).ceil() as usize).max(if round { rounded } else { 1 }).clamp(1, 8);
        let cell = (run / per as f64 / 6.0).clamp(1.5, 15.0);
        let m = 2.0 * cell;
        let bb = [rf.bb[0] - m, rf.bb[1] - m, rf.bb[2] + m, rf.bb[3] + m];
        let grid = Grid::new(bb, cell, |p| Self::scaled(rf, p, false, 2.0));
        let mut out = vec![];
        for k in 1..=per {
            for line in grid.level_set(k as f64 / per as f64) {
                let mut line = simplify_line(&line, tol, 0.0);
                if self.mitre {
                    line = sharpen(&line, 2.5 * cell);
                }
                out.push(simplify_line(&line, 0.0, max));
            }
        }
        out
    }
}

/// The floor's height anywhere under the document's loops, their profiles included: what paths
/// lay themselves out on (no paths, bumps or painted terrain).
pub struct Ground {
    polys: Vec<Vec<P2>>,
    field: Field,
}

impl Ground {
    pub fn new(doc: &Doc) -> Result<Ground, String> {
        let map = Map::build(doc)?;
        let zs: Vec<f64> = std::iter::once(doc.outline.z).chain(doc.regions.iter().map(|r| r.z)).collect();
        let field = Field::new(doc, &map, &zs, 90.0, &mut vec![])?;
        Ok(Ground { polys: map.loop_polys, field })
    }

    pub fn at(&self, p: P2) -> f64 {
        self.field.base(&self.polys, p)
    }
}

const WALL_CELL: f64 = 100.0;

/// The grid cells a segment's box covers.
fn cells(a: P2, b: P2, cell: f64) -> Vec<(i64, i64)> {
    let mut out = vec![];
    for i in (a[0].min(b[0]) / cell).floor() as i64..=(a[0].max(b[0]) / cell).floor() as i64 {
        for j in (a[1].min(b[1]) / cell).floor() as i64..=(a[1].max(b[1]) / cell).floor() as i64 {
            out.push((i, j));
        }
    }
    out
}

/// Whether a profile's numbers make sense.
fn check(p: &Profile) -> Result<(), String> {
    match *p {
        Profile::Cliff => Ok(()),
        Profile::Slope { angle, round } if angle > 0.5 && angle < 89.5 && (0.0..=1.0).contains(&round) => Ok(()),
        Profile::Slope { .. } => Err("a slope's angle must be between 1 and 89 degrees, its rounding between 0 and 1".into()),
        Profile::Terraces { steps, rise, depth } if steps >= 1 && steps <= 64 && depth >= 5.0 && rise.is_none_or(|r| r > 0.0) => Ok(()),
        Profile::Terraces { .. } => Err("terraces need 1 to 64 steps, at least 5 deep, rising more than 0".into()),
        Profile::Overhang { depth } if depth > 0.0 && depth <= 1000.0 => Ok(()),
        Profile::Overhang { .. } => Err("an overhang's depth must be more than 0 (and at most 1000)".into()),
        Profile::Ragged { amplitude, scale, .. } if amplitude >= 0.0 && scale >= 10.0 => Ok(()),
        Profile::Ragged { .. } => Err("ragged rock needs a size of at least 10 and a height of 0 or more".into()),
    }
}

/// The loops' pieces in buckets, for finding where a line crosses them.
struct Edges {
    cell: f64,
    segs: Vec<(P2, P2)>,
    buckets: HashMap<(i64, i64), Vec<u32>>,
}

impl Edges {
    fn new(polys: &[Vec<P2>]) -> Edges {
        let cell = 100.0;
        let mut segs = vec![];
        let mut buckets: HashMap<(i64, i64), Vec<u32>> = HashMap::new();
        for lp in polys {
            for i in 0..lp.len() {
                let (a, b) = (lp[i], lp[(i + 1) % lp.len()]);
                let k = segs.len() as u32;
                segs.push((a, b));
                for bi in (a[0].min(b[0]) / cell).floor() as i64..=(a[0].max(b[0]) / cell).floor() as i64 {
                    for bj in (a[1].min(b[1]) / cell).floor() as i64..=(a[1].max(b[1]) / cell).floor() as i64 {
                        buckets.entry((bi, bj)).or_default().push(k);
                    }
                }
            }
        }
        Edges { cell, segs, buckets }
    }

    /// Where p -> q crosses or touches a loop: (fraction along p -> q, the point, snapped onto a
    /// loop's vertex when it's that close).
    fn crossings(&self, p: P2, q: P2) -> Vec<(f64, P2)> {
        let c = self.cell;
        let mut ks: Vec<u32> = vec![];
        for bi in (p[0].min(q[0]) / c).floor() as i64..=(p[0].max(q[0]) / c).floor() as i64 {
            for bj in (p[1].min(q[1]) / c).floor() as i64..=(p[1].max(q[1]) / c).floor() as i64 {
                if let Some(b) = self.buckets.get(&(bi, bj)) {
                    ks.extend_from_slice(b);
                }
            }
        }
        ks.sort_unstable();
        ks.dedup();
        let mut out = vec![];
        for k in ks {
            let (a, b) = self.segs[k as usize];
            let (d1, d2, d3, d4) = (cross(a, b, p), cross(a, b, q), cross(p, q, a), cross(p, q, b));
            if (d1 > 0.0) == (d2 > 0.0) || (d3 > 0.0) == (d4 > 0.0) || d1 == d2 || d3 == d4 {
                continue;
            }
            let t = d1 / (d1 - d2);
            let mut x = lerp(p, q, t);
            // a crossing at a loop's vertex is that vertex
            if dist(x, a) < 0.05 {
                x = a;
            } else if dist(x, b) < 0.05 {
                x = b;
            }
            out.push((t, x));
        }
        out.sort_by(|x, y| x.0.total_cmp(&y.0));
        out
    }
}

/// The pieces of a line (closed if it ends on its first point) inside region r's own floor: cut
/// where it crosses any loop, keeping the pieces whose middle's innermost loop is r.
fn clip(line: &[P2], edges: &Edges, polys: &[Vec<P2>], areas: &[f64], r: usize) -> Vec<Vec<P2>> {
    let closed = line.len() > 3 && dist(line[0], line[line.len() - 1]) < 1e-9;
    // the line with its crossings, each marked
    let mut pts: Vec<(P2, bool)> = vec![(line[0], false)];
    for w in line.windows(2) {
        for (_, x) in edges.crossings(w[0], w[1]) {
            pts.push((x, true));
        }
        pts.push((w[1], false));
    }
    let cut: Vec<usize> = (0..pts.len()).filter(|&i| pts[i].1).collect();
    let own = |piece: &[P2]| -> bool {
        let len: f64 = piece.windows(2).map(|w| dist(w[0], w[1])).sum();
        if len < 2.0 {
            return false;
        }
        // the point halfway along
        let mut s = 0.0;
        let mut mid = piece[0];
        for w in piece.windows(2) {
            let l = dist(w[0], w[1]);
            if s + l >= len / 2.0 {
                mid = lerp(w[0], w[1], (len / 2.0 - s) / l.max(1e-12));
                break;
            }
            s += l;
        }
        (0..polys.len()).filter(|&l| point_in_poly(mid, &polys[l])).min_by(|&x, &y| areas[x].total_cmp(&areas[y])) == Some(r)
    };
    let dedup = |v: Vec<P2>| -> Vec<P2> {
        let mut o: Vec<P2> = vec![];
        for p in v {
            if o.last().is_none_or(|q| dist(*q, p) > 1e-6) {
                o.push(p);
            }
        }
        o
    };
    if cut.is_empty() {
        let l = dedup(line.to_vec());
        return if l.len() >= 2 && own(&l) { vec![l] } else { vec![] };
    }
    let mut out = vec![];
    let mut pieces: Vec<Vec<P2>> = cut.windows(2).map(|w| pts[w[0]..=w[1]].iter().map(|x| x.0).collect()).collect();
    // the ends: a closed line's run from its last crossing round to its first is one piece
    let (first, last) = (cut[0], cut[cut.len() - 1]);
    if closed {
        let mut wrap: Vec<P2> = pts[last..].iter().map(|x| x.0).collect();
        wrap.extend(pts[1..=first].iter().map(|x| x.0));
        pieces.push(wrap);
    } else {
        pieces.push(pts[..=first].iter().map(|x| x.0).collect());
        pieces.push(pts[last..].iter().map(|x| x.0).collect());
    }
    for p in pieces {
        let p = dedup(p);
        if p.len() >= 2 && own(&p) {
            out.push(p);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(profile: Profile, z: f64) -> Doc {
        serde_json::from_value(serde_json::json!({
            "outline": { "nodes": [[-2000, -2000, 1], [2000, -2000, 1], [2000, 2000, 1], [-2000, 2000, 1]], "z": 0 },
            "regions": [ { "name": "hill", "nodes": [[-1000, -1000, 1], [1000, -1000, 1], [1000, 1000, 1], [-1000, 1000, 1]], "z": z,
                           "profiles": [serde_json::to_value(&profile).unwrap(), null, null, null] } ]
        }))
        .unwrap()
    }

    fn field(d: &Doc) -> (Map, Field) {
        let map = Map::build(d).unwrap();
        let zs = [d.outline.z, d.regions[0].z];
        let f = Field::new(d, &map, &zs, 35.0, &mut vec![]).unwrap();
        (map, f)
    }

    #[test]
    fn a_slope_rises_from_its_foot_to_the_top() {
        let d = doc(Profile::Slope { angle: 30.0, round: 0.0 }, 200.0);
        let (_, f) = field(&d);
        let run = 200.0 / 30f64.to_radians().tan();
        // the south edge (node 0 to 1) is the foot; the others stay cliffs
        assert!(f.z(1, [0.0, -1000.0], [0.0, -1000.0]).abs() < 1e-9);
        assert!((f.z(1, [0.0, -1000.0 + run / 2.0], [0.0, 0.0]) - 100.0).abs() < 1e-6);
        assert!((f.z(1, [0.0, -1000.0 + run + 1.0], [0.0, 0.0]) - 200.0).abs() < 1e-9);
        assert!((f.z(1, [0.0, 1000.0], [0.0, 0.0]) - 200.0).abs() < 1e-9);
        // rounded: the same height halfway, flat at both ends
        let d = doc(Profile::Slope { angle: 30.0, round: 1.0 }, 200.0);
        let (_, f) = field(&d);
        let run = run * 1.5;
        assert!((f.z(1, [0.0, -1000.0 + run / 2.0], [0.0, 0.0]) - 100.0).abs() < 1e-6);
        assert!(f.z(1, [0.0, -1000.0 + 0.02 * run], [0.0, 0.0]) < 1.0);
        assert!(f.z(1, [0.0, -1000.0 + 0.98 * run], [0.0, 0.0]) > 199.0);
    }

    #[test]
    fn terraces_step_up_and_cut_the_map() {
        let d = doc(Profile::Terraces { steps: 4, rise: None, depth: 100.0 }, 200.0);
        let (map, f) = field(&d);
        assert!(f.stepped(1));
        for (y, z) in [(-950.0, 50.0), (-850.0, 100.0), (-750.0, 150.0), (-650.0, 200.0), (0.0, 200.0)] {
            assert!((f.z(1, [0.0, y], [0.0, y]) - z).abs() < 1e-9, "{y}: {}", f.z(1, [0.0, y], [0.0, y]));
        }
        // three step lines across the square, edge to edge
        let cuts = f.cuts(&map, 0.5);
        assert_eq!(cuts.len(), 3, "{cuts:?}");
        let mut ys: Vec<f64> = cuts.iter().map(|c| c[0][1]).collect();
        ys.sort_by(f64::total_cmp);
        for (k, &y) in ys.iter().enumerate() {
            assert!((y - (-1000.0 + 100.0 * (k + 1) as f64)).abs() < 0.01, "{ys:?}");
            let c = &cuts.iter().find(|c| c[0][1] == y).unwrap();
            assert!(c.iter().all(|p| (p[1] - y).abs() < 0.01), "{c:?}");
            let (e0, e1) = (c[0][0].min(c[c.len() - 1][0]), c[0][0].max(c[c.len() - 1][0]));
            assert!((e0 + 1000.0).abs() < 1e-6 && (e1 - 1000.0).abs() < 1e-6, "{c:?}");
        }
        // too few steps for the drop: a cliff at the edge
        let d = doc(Profile::Terraces { steps: 2, rise: Some(40.0), depth: 100.0 }, 200.0);
        let (_, f) = field(&d);
        assert!((f.z(1, [0.0, -950.0], [0.0, -950.0]) - 160.0).abs() < 1e-9);
    }

    #[test]
    fn hard_edges_keep_sharp_corners() {
        // an L (its inner corner at (0, 0)) terraced all round, and a square terraced on one edge
        let doc_with = |edges: &str| -> Doc {
            serde_json::from_value(serde_json::json!({
                "outline": { "nodes": [[-3000, -3000, 1], [3000, -3000, 1], [3000, 3000, 1], [-3000, 3000, 1]], "z": 0 },
                "regions": [
                    { "name": "l", "nodes": [[-1000, -1000, 1], [1000, -1000, 1], [1000, 0, 1], [0, 0, 1], [0, 1000, 1], [-1000, 1000, 1]], "z": 200,
                      "profile": { "kind": "terraces", "steps": 3, "depth": 100 } },
                    { "name": "sq", "nodes": [[1500, 1500, 1], [2500, 1500, 1], [2500, 2500, 1], [1500, 2500, 1]], "z": 200,
                      "profiles": [{ "kind": "terraces", "steps": 3, "depth": 100 }, null, null, null] }
                ],
                "settings": { "edges": edges }
            }))
            .unwrap()
        };
        let lines = |edges: &str| {
            let d = doc_with(edges);
            let map = Map::build(&d).unwrap();
            let f = Field::new(&d, &map, &[0.0, 200.0, 200.0], 35.0, &mut vec![]).unwrap();
            f.cuts(&map, 1.0)
        };
        let hard = lines("hard");
        // the L's two step lines are rings of six straight sides (a point more where a ring starts); the square's run into its cliffs
        // or square round the ends of its one terraced edge, in straight pieces
        assert!(hard.iter().all(|c| c.len() <= 8), "{:?}", hard.iter().map(|c| c.len()).collect::<Vec<_>>());
        let ring = hard.iter().filter(|c| c.len() > 2).min_by(|a, b| a.iter().map(|p| p[0]).fold(0.0, f64::min).total_cmp(&b.iter().map(|p| p[0]).fold(0.0, f64::min))).expect("the L's first ring");
        // round the L's inner corner, a sharp corner 100 in from both edges: (-100, -100), exactly
        assert!(ring.iter().any(|p| (p[0] + 100.0).abs() < 1e-6 && (p[1] + 100.0).abs() < 1e-6), "{ring:?}");
        assert_eq!(ring.len(), 7, "six corners: {ring:?}");
        // smooth edges round the inner corner off
        let smooth = lines("smooth");
        assert!(smooth.iter().any(|c| c.len() > 10));
    }

    #[test]
    fn a_sunken_region_slopes_down_from_its_edge() {
        let d = doc(Profile::Slope { angle: 20.0, round: 0.0 }, -150.0);
        let (_, f) = field(&d);
        assert!(f.z(1, [0.0, -1000.0], [0.0, 0.0]).abs() < 1e-9);
        assert!((f.z(1, [0.0, 0.0], [0.0, 0.0]) + 150.0).abs() < 1e-9);
        let z = f.z(1, [0.0, -900.0], [0.0, 0.0]);
        assert!(z < 0.0 && z > -150.0, "{z}");
    }

    #[test]
    fn steep_slopes_are_reported_and_nonsense_refused() {
        let d = doc(Profile::Slope { angle: 50.0, round: 0.0 }, 200.0);
        let map = Map::build(&d).unwrap();
        let mut pr = vec![];
        Field::new(&d, &map, &[0.0, 200.0], 35.0, &mut pr).unwrap();
        assert_eq!(pr.len(), 1, "{pr:?}");
        let d = doc(Profile::Terraces { steps: 0, rise: None, depth: 100.0 }, 200.0);
        assert!(Field::new(&d, &Map::build(&d).unwrap(), &[0.0, 200.0], 35.0, &mut vec![]).is_err());
    }
}
