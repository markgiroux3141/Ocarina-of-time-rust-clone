//! Ceilings: what's over each face. A region (or the whole level) can be covered
//! (`doc::Ceiling::Closed`), and each face is then a **column** of open air above its point,
//! bottom to top: `[floor, ceiling]`, then `[roof, ∞)` above the solid over the ceiling (and more
//! where covers nest: a hut in a cavern). Open ground is `[floor, ∞)`, as before.
//!
//! - Each interval's bottom is a floor facing up (the first is the face's own floor, the others
//!   roofs: the hill over a cave), each finite top a ceiling facing down.
//! - On each edge, the wall facing a side is that side's air minus the other side's
//!   (`minus`). With no ceilings that's the floor-to-floor wall `build.rs` has always made; with
//!   them it's a cave's walls, the lintel over a cave's mouth, a skylight's shaft.
//! - A cover belongs to its **owner** loop, and the loops inside it are under it too unless
//!   they're open (`Sky`) or covered themselves. Its shape (a dome, a vault, rough rock) is a
//!   field over the owner's whole loop, fading to the flat height at its edge, so the faces under
//!   it share one continuous ceiling and walls meet it at its flat height.
//! - An enclosed level (the outline covered) has nothing above its ceiling: outside the outline
//!   is solid, and a skylight's shaft goes up to `top`, open to the sky there.

use crate::doc::{Ceiling, Doc, Noise, ROOF_MIN};
use crate::geom::*;
use crate::map::Map;
use crate::noise::relief;

pub const INF: f64 = f64::INFINITY;

/// How far a skylight's shaft goes above an enclosed level's highest ceiling, when its outline's
/// cover gives no `roof`.
pub const SHAFT: f64 = 600.0;

/// How dark it is under a ceiling: the shade there is multiplied by this.
pub const DARK: f64 = 0.55;

/// A loop's own cover, resolved.
pub struct Own {
    /// The ceiling at its edges (absolute).
    pub z: f64,
    pub rise: f64,
    pub noise: Option<Noise>,
    /// The solid's top (absolute; at least `ROOF_MIN` over the ceiling wherever it's lower), or
    /// INF: nothing above (an enclosed level's).
    pub roof: f64,
    /// The style of the walls it adds, as the document gives it (`build.rs` checks it against the
    /// theme).
    pub style: Option<String>,
    seed: u32,
    /// How far in the shape takes to rise: `reach`, or the deepest point's distance from the edge.
    cap: f64,
    index: SegIndex,
}

impl Own {
    /// The ceiling's height at p (inside its owner's loop).
    pub fn at(&self, p: P2) -> f64 {
        let mut z = self.z;
        if self.rise == 0.0 && self.noise.is_none() {
            return z;
        }
        let reach = self.cap.max(self.noise.as_ref().map_or(0.0, |n| n.edge.max(1.0)));
        let d = self.index.dist_within(p, reach);
        if self.rise != 0.0 {
            // a quarter ellipse: steep off the walls, level at the middle (a barrel vault along
            // a long narrow loop, a dome over a round one)
            let t = (d / self.cap.max(1.0)).min(1.0);
            z += self.rise * (1.0 - (1.0 - t) * (1.0 - t)).max(0.0).sqrt();
        }
        if let Some(n) = &self.noise {
            let t = (d / n.edge.max(1.0)).min(1.0);
            let fade = t * t * (3.0 - 2.0 * t);
            z += n.amplitude * fade * relief([p[0] / n.scale.max(1.0), p[1] / n.scale.max(1.0)], self.seed);
        }
        z
    }

    /// The highest it reaches, about.
    fn peak(&self) -> f64 {
        self.z + self.rise.max(0.0) + self.noise.as_ref().map_or(0.0, |n| n.amplitude.abs())
    }

    /// The spacing of points a floor under it needs for its shape to show, if it isn't flat.
    pub fn spacing(&self) -> Option<f64> {
        let mut s = INF;
        if self.rise != 0.0 {
            s = s.min(self.cap / 4.0);
        }
        if let Some(n) = self.noise.as_ref().filter(|n| n.amplitude != 0.0) {
            s = s.min(n.scale / 4.0);
        }
        (s < INF).then(|| s.max(40.0))
    }
}

/// Every loop's cover (0 = the outline, i + 1 = region i).
pub struct Ceilings {
    pub own: Vec<Option<Own>>,
    /// The loop whose cover is over each loop (its own, or a parent's), None: open sky.
    pub owner: Vec<Option<usize>>,
    /// Each loop's parent: the innermost loop round it (None for the outline and areas).
    pub parent: Vec<Option<usize>>,
    /// Each loop's floor height (the document's).
    zs: Vec<f64>,
    polys: Vec<Vec<P2>>,
    areas: Vec<f64>,
    boxes: Vec<[f64; 4]>,
    /// The outline is covered: no edge of the world, walls up to the ceiling.
    pub enclosed: bool,
    /// An enclosed level's top, where skylights' shafts end.
    pub top: f64,
    /// Whether anything is covered at all (if not, the level builds as it always has).
    pub any: bool,
}

impl Ceilings {
    /// `doc` and `map` as built (bands included); `zs`: each loop's floor height.
    pub fn new(doc: &Doc, map: &Map, zs: &[f64], problems: &mut Vec<String>) -> Ceilings {
        let n = map.loop_polys.len();
        let polys = map.loop_polys.clone();
        let areas: Vec<f64> = polys.iter().map(|p| signed_area(p).abs()).collect();
        let boxes: Vec<[f64; 4]> = polys.iter().map(|p| p.iter().fold([INF, INF, -INF, -INF], |b, q| [b[0].min(q[0]), b[1].min(q[1]), b[2].max(q[0]), b[3].max(q[1])])).collect();
        let ceiling = |l: usize| if l == 0 { doc.outline.ceiling.as_ref() } else { doc.regions.get(l - 1).and_then(|r| r.ceiling.as_ref()) };
        let name = |l: usize| if l == 0 { "the outline".to_string() } else { doc.regions.get(l - 1).map_or(format!("region {}", l - 1), |r| if r.name.is_empty() { format!("region {}", l - 1) } else { format!("region {} ({})", l - 1, r.name) }) };
        let any = (0..n).any(|l| matches!(ceiling(l), Some(Ceiling::Closed(_))));
        let mut out = Ceilings { own: (0..n).map(|_| None).collect(), owner: vec![None; n], parent: vec![None; n], zs: zs.to_vec(), polys, areas, boxes, enclosed: false, top: INF, any };
        if !any {
            return out;
        }
        // parents: the innermost other loop round a point inside the loop's own faces
        for l in 1..n {
            let Some(f) = map.faces.iter().find(|f| f.region == l) else {
                out.parent[l] = Some(0);
                continue;
            };
            let p = map.left_point(map.cycles[f.outer][0]);
            out.parent[l] = (0..n).filter(|&m| m != l && point_in_poly(p, &out.polys[m])).min_by(|&a, &b| out.areas[a].total_cmp(&out.areas[b]));
        }
        // parents before children: by area, largest first
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| out.areas[b].total_cmp(&out.areas[a]));
        order.retain(|&l| l != 0);
        order.insert(0, 0);
        for &l in &order {
            match ceiling(l) {
                Some(Ceiling::Closed(c)) => {
                    let z = zs[l] + c.height;
                    if c.height < ROOF_MIN {
                        problems.push(format!("{}: a ceiling only {:.0} over its floor", name(l), c.height));
                    }
                    let roof = match (l, out.parent[l]) {
                        (0, _) => INF,
                        (_, parent) => c.roof.unwrap_or_else(|| parent.map_or(-INF, |pp| zs[pp])).max(z + ROOF_MIN),
                    };
                    let poly = &out.polys[l];
                    let deepest = deepest(poly);
                    let cap = if c.reach > 0.0 { c.reach.min(deepest) } else { deepest };
                    let index = SegIndex::new(&[poly.as_slice()], (cap / 4.0).max(50.0));
                    let seed = c.noise.as_ref().map_or(0, |n| n.seed).wrapping_add(l as u32 * 7919) ^ doc.settings.seed.wrapping_mul(0x27D4_EB2F);
                    out.own[l] = Some(Own { z, rise: c.rise, noise: c.noise.clone().filter(|n| n.amplitude != 0.0), roof, style: c.style.clone(), seed, cap: cap.max(1.0), index });
                    out.owner[l] = Some(l);
                    if l > 0 && (doc.regions[l - 1].profile.is_some() || doc.regions[l - 1].profiles.iter().any(Option::is_some)) {
                        problems.push(format!("{}: edge profiles under a ceiling aren't built yet, so its slopes climb through it", name(l)));
                    }
                }
                Some(Ceiling::Sky) => out.owner[l] = None,
                None => out.owner[l] = if l == 0 { None } else { out.owner[out.parent[l].unwrap_or(0)] },
            }
        }
        out.enclosed = out.owner[0] == Some(0);
        if out.enclosed {
            let peak = out.own.iter().flatten().map(Own::peak).fold(-INF, f64::max);
            let roof = match ceiling(0) {
                Some(Ceiling::Closed(c)) => c.roof,
                _ => None,
            };
            out.top = roof.unwrap_or(peak + SHAFT).max(peak + ROOF_MIN);
        }
        // a floor at or above the ceiling over it
        for l in 0..n {
            if let Some(o) = out.owner[l] {
                if zs[l] >= out.own[o].as_ref().unwrap().z - 1.0 {
                    problems.push(format!("{}: its floor ({:.0}) is up to its ceiling ({:.0})", name(l), zs[l], out.own[o].as_ref().unwrap().z));
                }
            }
        }
        out
    }

    /// The cover over loop l, if any.
    pub fn over(&self, l: usize) -> Option<&Own> {
        self.owner.get(l).copied().flatten().map(|o| self.own[o].as_ref().unwrap())
    }

    /// The ceiling over loop l at p, if it's covered.
    pub fn ceiling(&self, l: usize, p: P2) -> Option<f64> {
        self.over(l).map(|o| o.at(p))
    }

    /// The open air over loop l's ground at p, `floor` high there: intervals bottom to top, the
    /// last INF if it's open to the sky (an enclosed level's never are, but for skylights).
    pub fn column(&self, l: usize, floor: f64, p: P2) -> Vec<(f64, f64)> {
        let Some(o) = self.owner[l] else { return vec![(floor, INF)] };
        let own = self.own[o].as_ref().unwrap();
        let c = own.at(p);
        let mut out = vec![];
        if c > floor + 1e-6 {
            out.push((floor, c));
        }
        if own.roof < INF {
            // over the solid: the parent's air from the roof up
            let r = own.roof.max(c + ROOF_MIN);
            let up = match self.parent[o] {
                Some(pp) => self.column(pp, self.zs[pp], p),
                None => vec![(-INF, INF)],
            };
            for (a, b) in up {
                let a = a.max(r);
                if b > a + 1e-6 {
                    out.push((a, b));
                }
            }
        }
        out
    }

    /// The innermost loop round p (None: outside every loop).
    pub fn loop_at(&self, p: P2) -> Option<usize> {
        (0..self.polys.len())
            .filter(|&l| {
                let b = self.boxes[l];
                p[0] >= b[0] && p[0] <= b[2] && p[1] >= b[1] && p[1] <= b[3] && point_in_poly(p, &self.polys[l])
            })
            .min_by(|&a, &b| self.areas[a].total_cmp(&self.areas[b]))
    }

    /// Whether p is under a ceiling (on or below it).
    pub fn covered(&self, p: P3) -> bool {
        let xy = [p[0], p[1]];
        // quick: nowhere near a covered loop
        if !self.enclosed
            && !self.own.iter().enumerate().any(|(l, o)| {
                let b = self.boxes[l];
                o.is_some() && xy[0] >= b[0] && xy[0] <= b[2] && xy[1] >= b[1] && xy[1] <= b[3]
            })
        {
            return false;
        }
        let l = self.loop_at(xy).unwrap_or(0);
        self.ceiling(l, xy).is_some_and(|c| p[2] <= c + 1.0)
    }
}

/// The deepest point's distance from the polygon's edge, about (on a grid).
fn deepest(poly: &[P2]) -> f64 {
    let b = poly.iter().fold([INF, INF, -INF, -INF], |b, q| [b[0].min(q[0]), b[1].min(q[1]), b[2].max(q[0]), b[3].max(q[1])]);
    let step = ((b[2] - b[0]).max(b[3] - b[1]) / 48.0).max(1.0);
    let mut best: f64 = 0.0;
    let mut y = b[1] + 0.5 * step;
    while y < b[3] {
        let mut x = b[0] + 0.5 * step;
        while x < b[2] {
            if point_in_poly([x, y], poly) {
                best = best.max(dist_to_loop([x, y], poly));
            }
            x += step;
        }
        y += step;
    }
    best.max(1.0)
}

/// Intervals `a` minus intervals `b` (both sorted, apart).
pub fn minus(a: &[(f64, f64)], b: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut out = vec![];
    for &(lo, hi) in a {
        let mut cur = lo;
        for &(blo, bhi) in b {
            if bhi <= cur || blo >= hi {
                continue;
            }
            if blo > cur {
                out.push((cur, blo));
            }
            cur = cur.max(bhi);
            if cur >= hi {
                break;
            }
        }
        if cur < hi {
            out.push((cur, hi));
        }
    }
    out.retain(|&(x, y)| y - x > 1e-6);
    out
}

/// A wall's pieces at its two ends paired up: in order where both ends have as many, else each
/// piece with the other end's that overlaps it most, or pinched to its own middle there.
pub fn pair(p: &[(f64, f64)], q: &[(f64, f64)]) -> Vec<[(f64, f64); 2]> {
    if p.len() == q.len() {
        return p.iter().zip(q).map(|(&a, &b)| [a, b]).collect();
    }
    let overlap = |a: (f64, f64), b: (f64, f64)| a.1.min(b.1) - a.0.max(b.0);
    let (long, short, flip) = if p.len() > q.len() { (p, q, false) } else { (q, p, true) };
    let mut used = vec![false; short.len()];
    let mut out = vec![];
    for &a in long {
        let best = (0..short.len()).filter(|&k| !used[k] && overlap(a, short[k]) > 0.0).max_by(|&x, &y| overlap(a, short[x]).total_cmp(&overlap(a, short[y])));
        let b = match best {
            Some(k) => {
                used[k] = true;
                short[k]
            }
            None => {
                let m = 0.5 * (a.0 + a.1);
                (m, m)
            }
        };
        out.push(if flip { [b, a] } else { [a, b] });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intervals_subtract() {
        assert_eq!(minus(&[(0.0, 300.0), (600.0, INF)], &[(0.0, INF)]), vec![]);
        assert_eq!(minus(&[(0.0, INF)], &[(0.0, 300.0), (600.0, INF)]), vec![(300.0, 600.0)]);
        assert_eq!(minus(&[(0.0, INF)], &[(200.0, INF)]), vec![(0.0, 200.0)]);
        assert_eq!(minus(&[(0.0, 300.0), (600.0, INF)], &[(600.0, INF)]), vec![(0.0, 300.0)]);
        assert_eq!(minus(&[(0.0, 300.0)], &[]), vec![(0.0, 300.0)]);
    }
}
