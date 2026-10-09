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
//! - **Stacks:** parts from the foot in, walls and slopes (`Stack`). Walls past the edge are step
//!   lines cut into the map, as terraces' are; between them the floor follows the slopes, so each
//!   face between two step lines is a *segment* of the stack, read at the face's `anchor` (which
//!   segment) and at the point (how high in it). A wall right at the edge raises the edge itself.
//!   Walls and slopes may have their own wall style (`Field::wall_style`, `Field::look`).
//!
//! Overhangs and ragged rock don't change the floors: they bend the cliff on the edge (`Wall`,
//! looked up by `Field::wall`; `build.rs` bends the wall's columns).

use crate::doc::{Doc, Part, Profile, STEEP};
use crate::geom::*;
use crate::map::{Map, VOID};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
enum Kind {
    Slope { run: f64, round: f64 },
    Steps { steps: u32, rise: f64, depth: f64 },
    /// `Field::stacks[id]`, reaching `reach` in.
    Stack { id: usize, reach: f64 },
}

/// How a piece of a stack's slope is drawn.
#[derive(Clone, Debug, PartialEq)]
pub enum Look {
    /// The floor's own material, world-projected.
    Floor,
    /// As this wall style, along the edge and up the face.
    Style(String),
    /// Steeper than `STEEP` with no style: the theme's cliff.
    Steep,
}

/// A slope piece of a stack: from `d0` to `d1` in, `h0` to `h1` up from the foot.
#[derive(Clone, Debug)]
struct Piece {
    d0: f64,
    d1: f64,
    h0: f64,
    h1: f64,
    round: f64,
    look: Look,
}

/// Flat ground between two walls that would stand on the same line (a brick wall under a rock
/// wall): a ledge this deep, so each is a wall of its own.
const LEDGE: f64 = 12.0;

/// A line where a stack is buried on one side (`RegionField::bury`) has a point at least this often.
const BURY_STEP: f64 = 100.0;

/// A stacked profile for one drop (`Profile::Stack`): its segments, the floor between walls.
#[derive(Clone, Debug)]
struct Stack {
    /// Each segment's slope pieces, and its height where it starts (just past its wall).
    segs: Vec<(f64, Vec<Piece>)>,
    /// The walls past the edge (distances in, ascending): segment k runs from cut k - 1 to cut k.
    cuts: Vec<f64>,
    /// Every wall, the edge's (at 0) too: its distance in, its style and its foot's height up from
    /// the stack's.
    walls: Vec<(f64, Option<String>, f64)>,
    reach: f64,
}

impl Stack {
    /// The parts' set heights: more than the drop and the stack doesn't fit (`Field::new` makes
    /// the edge a cliff).
    fn fixed(parts: &[Part]) -> f64 {
        parts.iter().filter_map(|p| p.rise).map(|r| r.max(0.0)).sum()
    }

    fn new(parts: &[Part], drop: f64) -> Stack {
        let fixed = Stack::fixed(parts);
        let free = parts.iter().filter(|p| p.rise.is_none()).count();
        let share = if free > 0 { (drop - fixed).max(0.0) / free as f64 } else { 0.0 };
        // every part with a height, climbing less than the drop: the rest is a cliff at the edge
        let rest = if free == 0 && fixed < drop { drop - fixed } else { 0.0 };
        let mut st = Stack { segs: vec![(rest, vec![])], cuts: vec![], walls: vec![], reach: 0.0 };
        let (mut d, mut h) = (0.0, rest);
        let mut wall_here = rest > 0.0;
        if rest > 0.0 {
            st.walls.push((0.0, None, 0.0));
        }
        for p in parts {
            let r = p.rise.map_or(share, |r| r.max(0.0));
            if r < 0.5 {
                continue;
            }
            if p.kind == "wall" {
                if wall_here {
                    st.segs.last_mut().unwrap().1.push(Piece { d0: d, d1: d + LEDGE, h0: h, h1: h, round: 0.0, look: Look::Floor });
                    d += LEDGE;
                }
                st.walls.push((d, p.style.clone(), h));
                h += r;
                if d > 0.0 {
                    st.cuts.push(d);
                    st.segs.push((h, vec![]));
                } else {
                    st.segs[0].0 = h;
                }
                wall_here = true;
            } else {
                let round = p.round.clamp(0.0, 1.0);
                let run = r / p.angle.clamp(1.0, 89.0).to_radians().tan() * (1.0 + 0.5 * round);
                let look = match &p.style {
                    Some(s) => Look::Style(s.clone()),
                    None if p.angle > STEEP => Look::Steep,
                    None => Look::Floor,
                };
                st.segs.last_mut().unwrap().1.push(Piece { d0: d, d1: d + run, h0: h, h1: h + r, round, look });
                d += run;
                h += r;
                wall_here = false;
            }
        }
        st.reach = d;
        st
    }

    /// Which segment a point `d` in is in.
    fn segment(&self, d: f64) -> usize {
        self.cuts.iter().filter(|&&c| d > c).count()
    }

    /// The height up from the foot `d` in, in segment k (clamped to it).
    fn height(&self, k: usize, d: f64) -> f64 {
        let k = k.min(self.segs.len() - 1);
        let (start, pieces) = &self.segs[k];
        let lo = if k == 0 { 0.0 } else { self.cuts[k - 1] };
        let d = d.clamp(lo, self.cuts.get(k).copied().unwrap_or(f64::INFINITY));
        let mut h = *start;
        for p in pieces {
            if d <= p.d0 {
                break;
            }
            let s = ((d - p.d0) / (p.d1 - p.d0).max(1e-9)).clamp(0.0, 1.0);
            let g = s + (s * s * (3.0 - 2.0 * s) - s) * p.round;
            h = p.h0 + (p.h1 - p.h0) * g;
        }
        h
    }

    /// The slope piece `d` in, in segment k.
    fn piece(&self, k: usize, d: f64) -> Option<&Piece> {
        self.segs.get(k)?.1.iter().find(|p| d >= p.d0 - 1e-9 && d <= p.d1 + 1e-9)
    }
}

/// A profiled piece of a region's edge, with the height of the floor beside it.
#[derive(Clone, Copy, Debug)]
struct Foot {
    a: P2,
    b: P2,
    n: f64,
    /// Where its profile climbs from: `n`, but a stack's is the region's (the lowest floor beside
    /// any of its raised stacked edges, or the highest beside its sunken ones), so its walls and
    /// ledges are at the same heights all along, and a floor beside that is higher just buries
    /// its foot (a region raised against a band's rock face hides the bottom of its brick wall)
    /// rather than lifting the whole stack and leaving a step where it ends. A floor beside past
    /// all the stack's set parts keeps its own (`n`).
    base: f64,
    kind: Kind,
    /// Mitred distances (hard or faceted edges): at each end, the tangent half way between this
    /// piece's and the next profiled piece's (the joint's bisector), or None at a free end.
    mitre: Option<[Option<P2>; 2]>,
    /// Which loop pieces it is, the first and the last (to find its neighbours): consecutive
    /// pieces in one line, with the same profile, are one foot.
    m: usize,
    m_end: usize,
    /// A band's end, facing the void (`Field::new_with`'s `ends`).
    end: bool,
}

impl Foot {
    /// How far in the profile reaches (beyond, the region's own height).
    fn reach(&self) -> f64 {
        match self.kind {
            Kind::Slope { run, .. } => run,
            Kind::Steps { steps, depth, .. } => steps.saturating_sub(1) as f64 * depth,
            Kind::Stack { reach, .. } => reach,
        }
    }

    fn stepped(&self) -> bool {
        matches!(self.kind, Kind::Steps { .. })
    }

    /// Slope 0, terraces 1, stack 2: feet of one class mitre with each other.
    fn class(&self) -> usize {
        match self.kind {
            Kind::Slope { .. } => 0,
            Kind::Steps { .. } => 1,
            Kind::Stack { .. } => 2,
        }
    }

    fn stack(&self) -> Option<usize> {
        match self.kind {
            Kind::Stack { id, .. } => Some(id),
            _ => None,
        }
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
            Kind::Stack { .. } => 0.0,
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
            Kind::Stack { .. } => d,
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
    /// The slopes', the terraces' and the stacks' longest reach in.
    reach: [f64; 3],
    /// Whether a stack has walls past the edge (step lines: faces are read at their anchors).
    stack_cuts: bool,
    /// Lines in from where the floor beside a raised stack steps up and buries it on one side
    /// (`Foot::base`), unclipped: each runs from the joint to past the region's far side. Faces
    /// are then read at their anchors, the floor beside them the nearest foot's there.
    bury: Vec<Vec<P2>>,
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
    /// The stacked profiles, one per stacked foot (`Kind::Stack`'s id).
    stacks: Vec<Stack>,
    /// Per loop with ends (a band's), the wall style its ends' slopes are drawn as, if any.
    end_looks: HashMap<usize, String>,
}

/// A band's ends (`beyond.rs`): the floor beside the loop's edges that face the void, and the wall
/// style their slope is drawn as (none: the floor).
pub type Ends = HashMap<usize, (f64, Option<String>)>;

impl Field {
    /// From the map of the document's loops alone (`Map::build`: no paths' ribbons yet), with each
    /// region's height (`zs[0]` the outline's). Profiles steeper than `max_slope` are reported.
    pub fn new(doc: &Doc, map: &Map, zs: &[f64], max_slope: f64, problems: &mut Vec<String>) -> Result<Field, String> {
        Field::new_with(doc, map, zs, max_slope, problems, &Ends::new())
    }

    /// As `new`, with the floor beside some loops' edges that face the void (`ends`): a band's
    /// ends, which slope down to the forest's cliff tops (`beyond.rs`).
    pub fn new_with(doc: &Doc, map: &Map, zs: &[f64], max_slope: f64, problems: &mut Vec<String>, ends: &Ends) -> Result<Field, String> {
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
        let mut stacks: Vec<Stack> = vec![];
        for l in 1..n_loops {
            let lp = &map.loops[l];
            let ccw = signed_area(&map.loop_polys[l]) > 0.0;
            let top = zs[l];
            let mut feet = vec![];
            // which profile each foot is (the same one gives the same kind for the same floor beside)
            let mut keys: Vec<*const Profile> = vec![];
            let mut steep = 0.0f64;
            // the region's stacks climb from one base (`Foot::base`): the lowest floor beside its
            // raised stacked edges, the highest beside its sunken ones
            let (mut base_up, mut base_down) = (f64::INFINITY, f64::NEG_INFINITY);
            let mut cands = vec![];
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
                // (or a band's end)
                if fin == VOID || map.faces[fin].region != l {
                    continue;
                }
                let n = match (fout == VOID, ends.get(&l)) {
                    (false, _) => zs[map.faces[fout].region],
                    (true, Some(&(n, _))) => n,
                    (true, None) => continue,
                };
                if (top - n).abs() < 0.5 {
                    continue;
                }
                if matches!(p, Profile::Stack { .. }) {
                    if n < top {
                        base_up = base_up.min(n);
                    } else {
                        base_down = base_down.max(n);
                    }
                }
                cands.push((m, u, v, n, fout, p));
            }
            // one stack per profile, way (up or down) and base, shared by its feet
            // (profile, raised, base's bits) -> its stack and reach, or none: a cliff
            type Key = (*const Profile, bool, u64);
            let mut shared: HashMap<Key, Option<(usize, f64)>> = HashMap::new();
            for (m, u, v, n, fout, p) in cands {
                let drop = (top - n).abs();
                let base = if n < top { base_up } else { base_down };
                // a floor beside past the stack's set parts (another band's crest beside a band's
                // side) would bury all of them: there the stack climbs from that floor, as its own
                let base = match p {
                    Profile::Stack { parts } if (n - base).abs() >= Stack::fixed(parts) - 0.5 => n,
                    _ => base,
                };
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
                        // a band's ends are scenery, not for walking up
                        if fout != VOID {
                            steep = steep.max(angle);
                        }
                        let round = round.clamp(0.0, 1.0);
                        Kind::Slope { run: drop / angle.to_radians().tan() * (1.0 + 0.5 * round), round }
                    }
                    Profile::Terraces { steps, rise, depth } => Kind::Steps { steps, rise: rise.unwrap_or(drop / steps as f64), depth },
                    Profile::Stack { ref parts } => {
                        let entry = shared.entry((p as *const Profile, n < top, base.to_bits())).or_insert_with(|| {
                            // a stack taller than the drop (a short side of a stacked ridge) is a cliff
                            let drop = (top - base).abs();
                            (Stack::fixed(parts) <= drop + 0.5).then(|| {
                                stacks.push(Stack::new(parts, drop));
                                (stacks.len() - 1, stacks[stacks.len() - 1].reach)
                            })
                        });
                        let Some((id, reach)) = *entry else { continue };
                        Kind::Stack { id, reach }
                    }
                    Profile::Cliff => continue,
                };
                let base = if matches!(kind, Kind::Stack { .. }) { base } else { n };
                feet.push(Foot { a: map.verts[u], b: map.verts[v], n, base, kind, mitre: None, m, m_end: m, end: fout == VOID });
                keys.push(p as *const Profile);
            }
            if feet.is_empty() {
                continue;
            }
            // consecutive pieces of one straight edge (the map cuts edges into many) with the same
            // profile and floor beside are one foot: the same distances, far fewer to measure
            let mut merged: Vec<Foot> = vec![];
            let mut last_key: Option<*const Profile> = None;
            for (f, key) in feet.into_iter().zip(keys) {
                if let Some(g) = merged.last_mut() {
                    let (s, t) = (sub(g.b, g.a), sub(f.b, f.a));
                    let in_line = (s[0] * t[1] - s[1] * t[0]).abs() <= 1e-9 * s[0].hypot(s[1]) * t[0].hypot(t[1]) && s[0] * t[0] + s[1] * t[1] > 0.0;
                    if last_key == Some(key) && g.m_end + 1 == f.m && g.end == f.end && (g.n - f.n).abs() < 1e-9 && dist(g.b, f.a) < 1e-9 && in_line {
                        g.b = f.b;
                        g.m_end = f.m;
                        continue;
                    }
                }
                merged.push(f);
                last_key = Some(key);
            }
            let mut feet = merged;
            // hard or faceted edges: mitred joints between profiled pieces of the same kind
            if matches!(doc.settings.edges.as_str(), "hard" | "faceted") {
                let at: HashMap<usize, usize> = feet.iter().enumerate().flat_map(|(i, f)| [(f.m, i), (f.m_end, i)]).collect();
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
                        [(f.m + n - 1) % n, (f.m_end + 1) % n].map(|m| {
                            let g = &feet[*at.get(&m)?];
                            if g.class() != f.class() {
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
            let reach = |class: usize| feet.iter().filter(|f| f.class() == class).map(Foot::reach).fold(0.0, f64::max);
            let reach = [reach(0), reach(1), reach(2)];
            let stack_cuts = feet.iter().filter_map(Foot::stack).any(|id| !stacks[id].cuts.is_empty());
            // where the floor beside a raised stack steps up between two feet and buries the stack
            // on one side only, a line in from their joint along its bisector, to the far side: the
            // wall between those floors carried on into the region where it's buried (and nothing,
            // no taller than 0, further in)
            let mut bury = vec![];
            let first: HashMap<usize, usize> = feet.iter().enumerate().map(|(i, f)| (f.m, i)).collect();
            let bb = map.loop_polys[l].iter().fold([f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY], |b, q| [b[0].min(q[0]), b[1].min(q[1]), b[2].max(q[0]), b[3].max(q[1])]);
            for f in &feet {
                let Some(g) = first.get(&((f.m_end + 1) % lp.len())).map(|&j| &feet[j]) else { continue };
                let (Some(fid), Some(gid)) = (f.stack(), g.stack()) else { continue };
                if f.base >= top || g.base != f.base || (f.n - g.n).abs() < 0.5 {
                    continue;
                }
                let (hi, st) = if f.n > g.n { (f.n, &stacks[fid]) } else { (g.n, &stacks[gid]) };
                // buried at the edge at all: the stack starts below the higher floor
                if f.base + st.height(0, 0.0) >= hi - 0.5 {
                    continue;
                }
                let inward = |x: &Foot| {
                    let t = sub(x.b, x.a);
                    let l = t[0].hypot(t[1]).max(1e-9);
                    if ccw { [-t[1] / l, t[0] / l] } else { [t[1] / l, -t[0] / l] }
                };
                let (a, b) = (inward(f), inward(g));
                let m = [a[0] + b[0], a[1] + b[1]];
                let ml = m[0].hypot(m[1]);
                if ml < 1e-6 {
                    continue;
                }
                let (m, span) = ([m[0] / ml, m[1] / ml], (bb[2] - bb[0]).hypot(bb[3] - bb[1]) + 1.0);
                // a point wherever the stack's slopes bend and every `BURY_STEP`, so the floors
                // either side follow them; none where its walls are (the line crosses their step
                // lines there, and the map puts its own point there: one of ours close by makes
                // slivers)
                let k = (m[0] * a[0] + m[1] * a[1]).max(0.2);
                let walls = |t: f64| st.cuts.iter().any(|c| (c / k - t).abs() < 2.0 * LEDGE);
                let mut ts: Vec<f64> = st.segs.iter().flat_map(|s| &s.1).flat_map(|pc| [pc.d0 / k, pc.d1 / k]).filter(|&t| t > 0.5 && t < span && !walls(t)).collect();
                ts.extend([0.0, span]);
                ts.sort_by(f64::total_cmp);
                ts.dedup_by(|x, y| (*x - *y).abs() < 0.5);
                let mut line = vec![f.b];
                for w in ts.windows(2) {
                    let n = ((w[1] - w[0]) / BURY_STEP).ceil().max(1.0) as usize;
                    line.extend((1..=n).map(|i| w[0] + (w[1] - w[0]) * i as f64 / n as f64).map(|t| [f.b[0] + m[0] * t, f.b[1] + m[1] * t]));
                }
                bury.push(line);
            }
            let cell = reach[0].max(reach[1]).max(reach[2]).max(20.0);
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
            regions[l] = Some(RegionField { top, feet, cell, buckets, bb, reach, stack_cuts, bury });
        }
        let loop_areas = map.loop_polys.iter().map(|p| signed_area(p).abs()).collect();
        let mut wall_buckets: HashMap<(i64, i64), Vec<u32>> = HashMap::new();
        for (k, &(a, b, _)) in walls.iter().enumerate() {
            for key in cells(a, b, WALL_CELL) {
                wall_buckets.entry(key).or_default().push(k as u32);
            }
        }
        let mitre = matches!(doc.settings.edges.as_str(), "hard" | "faceted");
        let end_looks = ends.iter().filter_map(|(&l, (_, s))| Some((l, s.clone()?))).collect();
        Ok(Field { zs: zs.to_vec(), regions, loop_areas, walls, wall_buckets, mitre, stacks, end_looks })
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

    /// Whether region r's floor has step lines (terraces' treads, a stack's walls past its edge):
    /// its faces are then read at their anchors (see `z`).
    pub fn stepped(&self, r: usize) -> bool {
        self.regions.get(r).and_then(|x| x.as_ref()).is_some_and(|f| f.reach[1] > 0.0 || f.stack_cuts || !f.bury.is_empty())
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
            for f in rf.near(q).filter(|f| f.stepped() == stepped && f.stack().is_none()) {
                let d = f.dist(q);
                if d >= f.reach() {
                    continue;
                }
                let dev = f.dev(rf.top, d);
                down = down.min(dev);
                up = up.max(dev);
            }
        }
        // stacks: the segment the face is in (at its anchor), the height at p within it; raised and
        // sunken apart, each kept to the floor beside its nearest foot (`Foot::base`: that floor
        // buries a raised stack's foot, cuts off a sunken one's top)
        if rf.reach[2] > 0.0 {
            let ways = self.stack_ways(rf, p, anchor);
            if let Some((dev, _, beside)) = ways[0] {
                down = down.min(dev.max(beside));
            }
            if let Some((dev, _, beside)) = ways[1] {
                up = up.max(dev.min(beside));
            }
        }
        rf.top + down + up
    }

    /// Region r's stacks at p, raised and sunken: (the lowest raised or highest sunken height,
    /// the nearest foot's distance, the floor beside it), all less the region's height. Nearest
    /// p, or the face's anchor where lines split the floor where it's buried (`RegionField::bury`).
    fn stack_ways(&self, rf: &RegionField, p: P2, anchor: P2) -> [Option<(f64, f64, f64)>; 2] {
        let seg = rf.stack_cuts.then(|| self.segment_at(rf, anchor)).flatten();
        let mut ways: [Option<(f64, f64, f64)>; 2] = [None, None];
        let owner = if rf.bury.is_empty() { p } else { anchor };
        for f in rf.near(p) {
            let Some(id) = f.stack() else { continue };
            let st = &self.stacks[id];
            let d = f.dist(p);
            // an anchor with no stacked foot near it is past every wall
            let k = if st.cuts.is_empty() { 0 } else { seg.map_or(st.segs.len() - 1, |k| k.min(st.segs.len() - 1)) };
            if d >= st.reach && k + 1 >= st.segs.len() {
                continue;
            }
            let raised = f.base < rf.top;
            let dev = f.base + if raised { 1.0 } else { -1.0 } * st.height(k, d) - rf.top;
            let w = &mut ways[usize::from(!raised)];
            let od = if rf.bury.is_empty() { d } else { f.dist(owner) };
            let x = w.get_or_insert((dev, od, f.n - rf.top));
            x.0 = if raised { x.0.min(dev) } else { x.0.max(dev) };
            if od < x.1 {
                (x.1, x.2) = (od, f.n - rf.top);
            }
        }
        ways
    }

    /// The floor beside region r's raised stack nearest p (in the face whose anchor is `anchor`),
    /// which may bury its foot: the ground there is no lower, whatever's done to it after (a rough
    /// band's).
    pub fn buried_to(&self, r: usize, p: P2, anchor: P2) -> Option<f64> {
        let rf = self.regions.get(r)?.as_ref().filter(|f| f.reach[2] > 0.0)?;
        Some(rf.top + self.stack_ways(rf, p, anchor)[0]?.2)
    }

    /// Which segment of its stack the anchor of a face is in: by the stacked foot nearest it (a long
    /// face, a ledge along a whole edge, reaches far from the feet near some of its points).
    fn segment_at(&self, rf: &RegionField, anchor: P2) -> Option<usize> {
        rf.near(anchor).filter_map(|f| Some((f.dist(anchor), f.stack()?))).min_by(|a, b| a.0.total_cmp(&b.0)).map(|(d, id)| self.stacks[id].segment(d))
    }

    /// The stacked foot nearest p in region r (within its reach), with its stack and p's distance in.
    fn stack_at(&self, r: usize, p: P2) -> Option<(&Foot, &Stack, f64)> {
        let rf = self.regions.get(r)?.as_ref().filter(|f| f.reach[2] > 0.0)?;
        rf.near(p).filter_map(|f| Some((f, &self.stacks[f.stack()?], f.dist(p)))).filter(|x| x.2 < x.1.reach + 1e-6).min_by(|a, b| a.2.total_cmp(&b.2))
    }

    /// How region r's floor at p (in the face whose anchor is `anchor`) is drawn, if it's a stack's
    /// slope (or a band's end) not drawn as the floor: the look, the direction along its foot (a
    /// wall-textured slope's u runs that way) and the heights of the slope's foot and crest.
    pub fn look(&self, r: usize, p: P2, anchor: P2) -> Option<(Look, P2, [f64; 2])> {
        let rf = self.regions.get(r)?.as_ref()?;
        let here = self.z(r, p, anchor);
        let along = |f: &Foot| {
            let t = sub(f.b, f.a);
            let l = t[0].hypot(t[1]).max(1e-9);
            [t[0] / l, t[1] / l]
        };
        if let Some((f, st, d)) = self.stack_at(r, p) {
            let k = if st.cuts.is_empty() { st.segment(d) } else { self.segment_at(rf, anchor).map_or(st.segs.len() - 1, |k| k.min(st.segs.len() - 1)) };
            let z = |h: f64| f.base + (rf.top - f.base).signum() * h;
            // the stack sets the floor here (not a band's end, lower, nor the floor beside burying it)
            if (here - z(st.height(k, d))).abs() <= 0.5 {
                let pc = st.piece(k, d)?;
                if pc.look == Look::Floor || (pc.h1 - pc.h0).abs() < 1e-9 {
                    return None;
                }
                let (z0, z1) = (z(pc.h0), z(pc.h1));
                return Some((pc.look.clone(), along(f), [z0.min(z1), z0.max(z1)]));
            }
        }
        // a band's end, drawn as its stack's last slope
        let style = self.end_looks.get(&r)?;
        let f = rf.near(p).filter(|f| f.end).find(|f| {
            let d = f.dist(p);
            d < f.reach() && (rf.top + f.dev(rf.top, d) - here).abs() <= 0.5
        })?;
        Some((Look::Style(style.clone()), along(f), [f.n.min(rf.top), f.n.max(rf.top)]))
    }

    /// The style a stack gives the wall p -> q in region r, if it's one of a stack's walls with a
    /// style: on a stacked edge (a wall at the edge), or along one of its step lines (`tol`: how far
    /// off the line its ends may be, the traced lines' tolerance).
    pub fn wall_style(&self, r: usize, p: P2, q: P2, tol: f64) -> Option<String> {
        self.stack_wall(r, p, q, tol)?.0
    }

    /// The stack's wall p -> q in region r, if it's one (as `wall_style`): its style, and where a
    /// raised stack's wall stands as tall as the stack makes it, its foot's height (lower than the
    /// floor below it where that buries it: `Foot::base`).
    pub fn stack_wall(&self, r: usize, p: P2, q: P2, tol: f64) -> Option<(Option<String>, Option<f64>)> {
        let rf = self.regions.get(r)?.as_ref().filter(|f| f.reach[2] > 0.0)?;
        let m = lerp(p, q, 0.5);
        let foot = |f: &Foot, below: f64| (f.base < rf.top).then_some(f.base + below);
        // on the edge: on a stacked loop piece (p -> q is it, or part of it)
        if let Some(f) = rf.near(m).find(|f| f.stack().is_some() && dist_to_seg(m, f.a, f.b).0 < 1e-6) {
            return self.stacks[f.stack()?].walls.iter().find(|w| w.0 == 0.0).map(|w| (w.1.clone(), foot(f, w.2)));
        }
        // along a step line: both ends that far in from the nearest stacked foot
        let nearest = |x: P2| rf.near(x).filter(|f| f.stack().is_some()).map(|f| (f.dist(x), f)).min_by(|a, b| a.0.total_cmp(&b.0));
        let (_, f) = nearest(m)?;
        let (dp, dq) = (nearest(p)?.0, nearest(q)?.0);
        self.stacks[f.stack()?].walls.iter().filter(|w| w.0 > 0.0).find(|w| (dp - w.0).abs() <= tol && (dq - w.0).abs() <= tol).map(|w| (w.1.clone(), foot(f, w.2)))
    }

    /// The floor's height at p under the document's loops alone: the innermost loop's (its
    /// profiles included), or the outline's outside every loop. `polys`: the map's loop polygons.
    pub fn base(&self, polys: &[Vec<P2>], p: P2) -> f64 {
        (0..polys.len()).filter(|&l| point_in_poly(p, &polys[l])).min_by(|&x, &y| self.loop_areas[x].total_cmp(&self.loop_areas[y])).map_or(self.zs[0], |l| self.z(l, p, p))
    }

    /// Lines across region r's stacks' slopes: along each slope piece's foot and crest (where
    /// they aren't step lines already) and between them about `spacing` apart, as `rings`.
    fn stack_rings(&self, r: usize, spacing: f64, rounded: usize, tol: f64, max: f64) -> Vec<Vec<P2>> {
        let Some(rf) = self.regions.get(r).and_then(|x| x.as_ref()).filter(|f| f.reach[2] > 0.0) else { return vec![] };
        let mut ds: Vec<f64> = vec![];
        let mut cuts: Vec<f64> = vec![];
        for id in rf.feet.iter().filter_map(Foot::stack) {
            let st = &self.stacks[id];
            cuts.extend(st.cuts.iter().copied());
            for (_, pieces) in &st.segs {
                for p in pieces.iter().filter(|p| (p.h1 - p.h0).abs() > 1e-9) {
                    let per = ((p.d1 - p.d0) / spacing.max(1.0)).ceil().max(if p.round > 0.0 { rounded as f64 } else { 1.0 }).clamp(1.0, 8.0) as usize;
                    ds.extend((0..=per).map(|k| p.d0 + (p.d1 - p.d0) * k as f64 / per as f64));
                }
            }
        }
        ds.retain(|&d| d > 0.5 && !cuts.iter().any(|c| (c - d).abs() < 1.0));
        ds.sort_by(f64::total_cmp);
        ds.dedup_by(|a, b| (*a - *b).abs() < 1.0);
        if ds.is_empty() {
            return vec![];
        }
        let cell = (ds[0] / 3.0).clamp(3.0, 15.0);
        let m = 2.0 * cell;
        let bb = [rf.bb[0] - m, rf.bb[1] - m, rf.bb[2] + m, rf.bb[3] + m];
        let cap = ds[ds.len() - 1] + 2.0 * cell;
        let grid = Self::stack_grid(rf, bb, cell, cap);
        let mut out = vec![];
        for &d in &ds {
            for line in grid.level_set(d) {
                let mut line = simplify_line(&line, tol, 0.0);
                if self.mitre {
                    line = sharpen(&line, 2.5 * cell);
                }
                out.push(simplify_line(&line, 0.0, max));
            }
        }
        out
    }

    /// The distance in from region's nearest stacked foot over its box, capped. Points further
    /// than half as far again from every stacked foot (a mitred distance is at least the plain one
    /// over the square root of 2) are the cap without measuring each foot.
    fn stack_grid(rf: &RegionField, bb: [f64; 4], cell: f64, cap: f64) -> Grid {
        let segs: Vec<[P2; 2]> = rf.feet.iter().filter(|f| f.stack().is_some()).map(|f| [f.a, f.b]).collect();
        let index = SegIndex::new(&segs.iter().map(|s| &s[..]).collect::<Vec<_>>(), 1.5 * cap);
        Grid::new(bb, cell, |p| {
            if index.dist_within(p, 1.5 * cap) >= 1.5 * cap {
                return cap;
            }
            rf.near(p).filter(|f| f.stack().is_some()).map(|f| f.dist(p)).fold(cap, f64::min)
        })
    }

    /// The scaled distance in from the region's nearest foot of one kind (`Foot::scaled`), capped.
    fn scaled(rf: &RegionField, p: P2, stepped: bool, cap: f64) -> f64 {
        rf.near(p).filter(|f| f.stepped() == stepped && f.stack().is_none()).map(|f| f.scaled(p)).fold(cap, f64::min)
    }

    /// Terraces' step lines, cut into the map (`Map::build_with`'s cuts): for each region with
    /// terraces, the lines k treads in (k = 1 to the most steps less one), simplified within `tol`,
    /// clipped to where the region's own floor is (each piece ends on a loop's edge or closes).
    pub fn cuts(&self, map: &Map, tol: f64) -> Vec<Vec<P2>> {
        let edges = Edges::new(&map.loop_polys);
        let mut out = vec![];
        // each region's stacks' step lines' points, for the lines where it's buried to go through
        let mut steps: Vec<Vec<P2>> = vec![vec![]; self.regions.len()];
        // stacks' walls past the edge: lines that far in from their feet
        for (r, rf) in self.regions.iter().enumerate() {
            let Some(rf) = rf.as_ref().filter(|f| f.stack_cuts) else { continue };
            let mut ds: Vec<f64> = rf.feet.iter().filter_map(Foot::stack).flat_map(|id| self.stacks[id].cuts.iter().copied()).collect();
            ds.sort_by(f64::total_cmp);
            ds.dedup_by(|a, b| (*a - *b).abs() < 1.0);
            // each line is traced on its own, so the cell needn't be finer than the gaps between them
            let cell = (ds[0] / 3.0).clamp(3.0, 12.0);
            let m = 2.0 * cell;
            let bb = [rf.bb[0] - m, rf.bb[1] - m, rf.bb[2] + m, rf.bb[3] + m];
            let cap = ds[ds.len() - 1] + 2.0 * cell;
            let grid = Self::stack_grid(rf, bb, cell, cap);
            for &d in &ds {
                for line in grid.level_set(d) {
                    let mut line = simplify_line(&line, tol, 0.0);
                    if self.mitre {
                        line = sharpen(&line, 2.5 * cell);
                    }
                    for piece in clip(&line, &edges, &map.loop_polys, &self.loop_areas, r) {
                        steps[r].extend_from_slice(&piece);
                        out.push(piece);
                    }
                }
            }
        }
        // where a raised stack is buried on one side of a joint: from the joint to the far side,
        // through the step lines' own points where it passes close to them (their corners lie on
        // its bisector: crossing a step line a hair from its point would make slivers the map
        // can't tell apart)
        for (r, rf) in self.regions.iter().enumerate() {
            for line in rf.iter().flat_map(|f| &f.bury) {
                let line = through(line, &steps[r], 0.5 * LEDGE);
                out.extend(clip(&line, &edges, &map.loop_polys, &self.loop_areas, r).into_iter().filter(|c| dist(c[0], line[0]) < 1e-6).take(1));
            }
        }
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
        let mut out = self.stack_rings(r, spacing, rounded, tol, max);
        let Some(rf) = self.regions.get(r).and_then(|x| x.as_ref()).filter(|f| f.reach[0] > 0.0) else { return out };
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

/// How far in a profile reaches for a drop (0 for a cliff of any kind).
pub fn reach(p: &Profile, drop: f64) -> f64 {
    match p {
        Profile::Slope { angle, round } => drop / angle.clamp(1.0, 89.0).to_radians().tan() * (1.0 + 0.5 * round.clamp(0.0, 1.0)),
        Profile::Terraces { steps, depth, .. } => steps.saturating_sub(1) as f64 * depth,
        Profile::Stack { parts } => Stack::new(parts, drop).reach,
        Profile::Cliff | Profile::Overhang { .. } | Profile::Ragged { .. } => 0.0,
    }
}

/// Whether a profile's numbers make sense.
pub fn check(p: &Profile) -> Result<(), String> {
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
        Profile::Stack { ref parts } => {
            if parts.is_empty() || parts.len() > 16 {
                return Err("a stack needs 1 to 16 parts".into());
            }
            for (i, p) in parts.iter().enumerate() {
                match p.kind.as_str() {
                    "wall" => {}
                    "slope" if p.angle > 0.5 && p.angle < 89.5 && (0.0..=1.0).contains(&p.round) => {}
                    "slope" => return Err(format!("stack part {i}: a slope's angle must be between 1 and 89 degrees, its rounding between 0 and 1")),
                    k => return Err(format!("stack part {i}: {k:?} isn't a part (wall or slope)")),
                }
                if p.rise.is_some_and(|r| r.is_nan() || r < 0.0) {
                    return Err(format!("stack part {i}: its rise can't be below 0"));
                }
            }
            Ok(())
        }
    }
}

/// The line, bent to go through each of `pts` within `tol` of it (its own points that close to one
/// left out, but its first).
fn through(line: &[P2], pts: &[P2], tol: f64) -> Vec<P2> {
    // (where along the line, as segment index + fraction, the point)
    let mut on: Vec<(f64, P2)> = vec![];
    for &q in pts {
        let best = line.windows(2).enumerate().map(|(i, w)| (dist_to_seg(q, w[0], w[1]), i)).min_by(|a, b| (a.0).0.total_cmp(&(b.0).0));
        if let Some(((_, t), i)) = best.filter(|x| (x.0).0 < tol) {
            on.push((i as f64 + t, q));
        }
    }
    if on.is_empty() {
        return line.to_vec();
    }
    let mut all: Vec<(f64, P2)> = line.iter().enumerate().skip(1).filter(|(_, p)| on.iter().all(|x| dist(x.1, **p) >= tol)).map(|(i, p)| (i as f64, *p)).collect();
    all.extend(on.into_iter().filter(|x| dist(x.1, line[0]) >= 1e-6));
    all.sort_by(|a, b| a.0.total_cmp(&b.0));
    std::iter::once(line[0]).chain(all.into_iter().map(|x| x.1)).collect()
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
    fn a_stack_climbs_wall_then_slope_and_cuts_its_inner_walls() {
        // Kakariko's west wing: a cliff 330 tall at the edge, a grass slope at 36 degrees the rest
        let parts = vec![Part::wall(Some(330.0), Some("cliff")), Part::slope(36.0, None, None)];
        let d = doc(Profile::Stack { parts }, 900.0);
        let (map, f) = field(&d);
        let run = 570.0 / 36f64.to_radians().tan();
        // the edge stands 330 up (the wall), the slope climbs from there to the top
        assert!((f.z(1, [0.0, -1000.0], [0.0, -900.0]) - 330.0).abs() < 1e-6);
        assert!((f.z(1, [0.0, -1000.0 + run / 2.0], [0.0, 0.0]) - 615.0).abs() < 1e-6);
        assert!((f.z(1, [0.0, -1000.0 + run + 1.0], [0.0, 0.0]) - 900.0).abs() < 1e-9);
        assert!(!f.stepped(1), "no walls past the edge");
        assert!(f.cuts(&map, 0.5).is_empty());
        assert_eq!(f.wall_style(1, [-500.0, -1000.0], [500.0, -1000.0], 1.0).as_deref(), Some("cliff"));
        assert!(f.look(1, [0.0, -1000.0 + run / 2.0], [0.0, 0.0]).is_none(), "a gentle slope is the floor");
        // Kakariko's north: brick 320, rock 160 above it, rock at 69 degrees for 300, then 37 degrees
        let parts = vec![
            Part::wall(Some(320.0), Some("brick")),
            Part::wall(Some(160.0), Some("rock")),
            Part::slope(69.0, Some(300.0), Some("rock")),
            Part::slope(37.0, None, Some("rock")),
        ];
        let d = doc(Profile::Stack { parts }, 1160.0);
        let (map, f) = field(&d);
        assert!(f.stepped(1));
        // the two walls can't stand on one line: the rock wall stands a ledge in
        let cuts = f.cuts(&map, 0.5);
        assert_eq!(cuts.len(), 1, "{cuts:?}");
        assert!(cuts[0].iter().all(|p| (p[1] - (-1000.0 + LEDGE)).abs() < 0.05), "{cuts:?}");
        // on the ledge (read in its own face) 320 up; past the rock wall 480, rising
        let ledge = [0.0, -1000.0 + LEDGE / 2.0];
        assert!((f.z(1, ledge, ledge) - 320.0).abs() < 1e-6);
        let on_cut = [0.0, -1000.0 + LEDGE];
        assert!((f.z(1, on_cut, ledge) - 320.0).abs() < 1e-6, "the foot of the rock wall");
        assert!((f.z(1, on_cut, [0.0, 0.0]) - 480.0).abs() < 1e-6, "its top");
        assert_eq!(f.wall_style(1, [-500.0, -1000.0], [500.0, -1000.0], 1.0).as_deref(), Some("brick"));
        assert_eq!(f.wall_style(1, [-500.0, -1000.0 + LEDGE], [500.0, -1000.0 + LEDGE], 1.0).as_deref(), Some("rock"));
        // the steep rock is drawn as rock, along the foot
        let up = 300.0 / 69f64.to_radians().tan();
        let (look, t, zs) = f.look(1, [0.0, -1000.0 + LEDGE + up / 2.0], [0.0, 0.0]).unwrap();
        assert!((zs[0] - 480.0).abs() < 1e-6 && (zs[1] - 780.0).abs() < 1e-6, "{zs:?}");
        assert_eq!(look, Look::Style("rock".into()));
        assert!((t[0].abs() - 1.0).abs() < 1e-9);
        // and it reaches the top
        let reach = LEDGE + up + (1160.0 - 780.0) / 37f64.to_radians().tan();
        assert!((f.z(1, [0.0, -1000.0 + reach + 1.0], [0.0, 0.0]) - 1160.0).abs() < 1e-9);
        // more than the drop: a cliff there; less, with every part's height set: a cliff for the rest
        let d = doc(Profile::Stack { parts: vec![Part::wall(Some(400.0), None), Part::slope(30.0, Some(400.0), None)] }, 400.0);
        let (_, f) = field(&d);
        assert!(f.is_empty());
        let st = Stack::new(&[Part::slope(30.0, Some(100.0), None)], 400.0);
        assert_eq!(st.walls, vec![(0.0, None, 0.0)]);
        assert!((st.segs[0].0 - 300.0).abs() < 1e-9 && (st.height(0, 1e9) - 400.0).abs() < 1e-9);
        // nonsense refused
        let d = doc(Profile::Stack { parts: vec![] }, 200.0);
        assert!(Field::new(&d, &Map::build(&d).unwrap(), &[0.0, 200.0], 35.0, &mut vec![]).is_err());
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
