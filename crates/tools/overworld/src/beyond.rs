//! What lies beyond the outline's edges when it isn't the forest (`Outline::beyond`): ground that
//! climbs from the edge to a crest, where the level ends at the sky (Kakariko's grass slopes,
//! its rock face under Death Mountain Trail, its mossy south wall).
//!
//! Each run of outline edges with the same `Beyond` becomes a **band**: a region outside the
//! outline, drawn against that run (sharing its nodes, so its edge is the outline's), as high as
//! the crest and as deep as its profile reaches, plus a little. Its profile is built inward from
//! the shared edge as any region's is (`profiles.rs`), which is outward from the level: the floor
//! stays as drawn, and walls, slopes, step lines and styles are the regions' own, watertight with
//! the level. The band's far edges have no edge of the world (`Bands::far`): the level just ends
//! there. Its two side edges get a plain face of the theme's boundary cliff, from the band's floor
//! down, facing out (`Bands::sides`), closing off its end where the forest (or nothing) is beside
//! it; the forest's rim only follows the forest's own edges.
//!
//! A side edge with the forest beside it (not another band) is the band's **end**: the ground
//! slopes down to it (`hip`), to the height of the forest's cliff tops (`Bands::ends`), so the
//! ground rolls off into the trees rather than stopping at a face as tall as the crest.

use crate::doc::{Doc, Profile, Region, ROUGH_SCALE};
use crate::geom::*;
use std::borrow::Cow;

/// A band's floor runs on this far past where its profile reaches: the crest.
pub const CREST: f64 = 60.0;

/// The bands added to a document (`expand`): they're its last regions.
#[derive(Debug, Default)]
pub struct Bands {
    /// The first band's region index (its loop is one more).
    pub first: usize,
    /// Per band, in order: its far edges (edge k runs from node k to the next).
    pub far: Vec<Vec<usize>>,
    /// Per band: its two side edges, from the outline out.
    pub sides: Vec<[usize; 2]>,
    /// The bands whose ends slope down (loop -> the height they slope down to, and how that slope
    /// is drawn): the floor beside their open side edges, for `profiles::Field::new_with`.
    pub ends: crate::profiles::Ends,
    /// The rough bands (loop -> how): their height above `base` varies (`Rough::at`).
    pub rough: std::collections::HashMap<usize, Rough>,
    /// The walls standing on bands' far edges (`Beyond::skyline`).
    pub skylines: Vec<SkylineWall>,
}

/// How a rough band's height varies (`Beyond::rough`): above `base` (the top of the walls at its
/// foot), heights are scaled by 1 + `amount` x noise, about `ROUGH_SCALE` across.
#[derive(Debug, Clone)]
pub struct Rough {
    pub base: f64,
    pub amount: f64,
    pub seed: u32,
    /// Its far edge's nodes, in order: along it, heights run straight from node to node (so the
    /// map's other points on a straight stretch of it stay in line).
    pub far: Vec<P2>,
}

impl Rough {
    /// The noise's factor at p.
    pub fn factor(&self, p: P2) -> f64 {
        (1.0 + self.amount * crate::noise::relief([p[0] / ROUGH_SCALE, p[1] / ROUGH_SCALE], self.seed)).max(0.2)
    }

    /// Height z at p, roughened.
    pub fn at(&self, p: P2, z: f64) -> f64 {
        if z <= self.base { z } else { self.base + (z - self.base) * self.factor(p) }
    }
}

/// A wall standing on a band's far edge (`Beyond::skyline`).
#[derive(Debug, Clone)]
pub struct SkylineWall {
    /// The band's loop.
    pub l: usize,
    /// Along the far edge, at least every `FAR_STEP`, the level on the right.
    pub pts: Vec<P2>,
    /// Whether the first and the last point's ends have the forest beside them: the wall comes
    /// down towards them.
    pub open: [bool; 2],
    pub style: String,
    pub height: f64,
    pub amount: f64,
    pub seed: u32,
}

/// A skyline wall has a column at least this often along its band's far edge, for its height to
/// vary.
const FAR_STEP: f64 = 300.0;

/// How steep a band's ends are, at most: as its own slopes, for the ground to look like one hill.
const HIP_MAX: f64 = 45.0;

/// The slope a band's ends come down by: as steep as the profile's last slope (at most `HIP_MAX`),
/// else 40 degrees; and drawn as that slope is (a style of its own, else the floor).
pub fn hip(p: &Profile) -> (Profile, Option<String>) {
    let (angle, style) = match p {
        Profile::Slope { angle, .. } => (*angle, None),
        Profile::Stack { parts } => parts.iter().rev().find(|q| q.kind == "slope").map_or((40.0, None), |q| (q.angle, q.style.clone())),
        _ => (40.0, None),
    };
    (Profile::Slope { angle: angle.min(HIP_MAX), round: 0.0 }, style)
}

impl Bands {
    /// Whether loop l is a band's.
    pub fn is_band(&self, l: usize) -> bool {
        l > self.first && l <= self.first + self.far.len()
    }
}

/// The document with a band region for each run of outline edges that has something beyond it
/// (the document itself if none has).
pub fn expand(doc: &Doc) -> Result<(Cow<'_, Doc>, Bands), String> {
    let n = doc.outline.nodes.len();
    let first = doc.regions.len();
    let edge = |k: usize| doc.outline.beyond.get(k % n).and_then(|b| b.as_ref());
    if n < 3 || (0..n).all(|k| edge(k).is_none()) {
        return Ok((Cow::Borrowed(doc), Bands { first, ..Default::default() }));
    }
    for (k, b) in doc.outline.beyond.iter().enumerate() {
        if let Some(b) = b {
            crate::profiles::check(&b.profile).map_err(|e| format!("beyond outline edge {k}: {e}"))?;
        }
    }
    // runs of edges with the same beyond; one round the whole outline is cut in two (a band
    // can't be a ring)
    let start = (0..n).find(|&k| edge(k) != edge(k + n - 1)).unwrap_or(0);
    let mut runs: Vec<(usize, usize)> = vec![]; // first edge, edge count
    for k in 0..n {
        let e = (start + k) % n;
        if edge(e).is_none() {
            continue;
        }
        match runs.last_mut() {
            Some((s, c)) if (*s + *c) % n == e && edge(*s) == edge(e) => *c += 1,
            _ => runs.push((e, 1)),
        }
    }
    if runs.len() == 1 && runs[0].1 == n {
        runs = vec![(start, n / 2), ((start + n / 2) % n, n - n / 2)];
    }
    let p = |i: usize| -> P2 { [doc.outline.nodes[i % n][0], doc.outline.nodes[i % n][1]] };
    let sharp = |i: usize| doc.outline.nodes[i % n].get(2).is_some_and(|&s| s != 0.0);
    let ccw = signed_area(&(0..n).map(p).collect::<Vec<_>>()) > 0.0;
    // outward at node i, across the tangent through its neighbours
    let out = |i: usize| -> P2 {
        let t = sub(p(i + 1), p(i + n - 1));
        let l = t[0].hypot(t[1]).max(1e-9);
        if ccw { [t[1] / l, -t[0] / l] } else { [-t[1] / l, t[0] / l] }
    };
    // outward across edge k (from node k to the next)
    let normal = |k: usize| -> P2 {
        let t = sub(p(k + 1), p(k));
        let l = t[0].hypot(t[1]).max(1e-9);
        if ccw { [t[1] / l, -t[0] / l] } else { [-t[1] / l, t[0] / l] }
    };
    let width = |b: &crate::doc::Beyond| crate::profiles::reach(&b.profile, (b.z - doc.outline.z).abs()) + CREST;
    // a node two runs share gets one far point, as far out as the wider needs
    let mut far_at: std::collections::HashMap<usize, f64> = Default::default();
    for &(s, c) in &runs {
        let w = width(edge(s).unwrap());
        for i in [s, s + c] {
            let e = far_at.entry(i % n).or_insert(0.0);
            *e = e.max(w);
        }
    }
    // nodes two runs share: their side edges are one, between two bands, not an end
    let mut touching: std::collections::HashMap<usize, usize> = Default::default();
    for &(s, c) in &runs {
        for i in [s, s + c] {
            *touching.entry(i % n).or_default() += 1;
        }
    }
    // a band's end slopes down to the forest's cliff tops beside it
    let foot = doc.outline.z + doc.boundary.cliff_min;
    let mut ends = std::collections::HashMap::new();
    let mut rough = std::collections::HashMap::new();
    let mut skylines = vec![];
    let mut d = doc.clone();
    let mut far = vec![];
    let mut sides = vec![];
    for &(s, c) in &runs {
        let b = edge(s).unwrap();
        let w = width(b);
        let node = |q: P2, sh: bool| if sh { vec![q[0], q[1], 1.0] } else { vec![q[0], q[1]] };
        let mut nodes: Vec<Vec<f64>> = (0..=c).map(|j| node(p(s + j), sharp(s + j))).collect();
        // each node's far point, so the band is its full width all along: an open end straight
        // out from the band's own edge (its side edge square to it), a node two bands share out
        // across the tangent (one far point for both), a sharp corner mitred
        for j in (0..=c).rev() {
            let i = s + j;
            let shared = (j == 0 || j == c) && touching[&(i % n)] >= 2;
            let (o, ww) = if shared {
                (out(i), far_at[&(i % n)])
            } else if j == 0 || j == c {
                (normal(if j == 0 { i } else { i + n - 1 }), w)
            } else if sharp(i) {
                let (a, b) = (normal(i + n - 1), normal(i));
                let m = [a[0] + b[0], a[1] + b[1]];
                let l = m[0].hypot(m[1]).max(1e-9);
                let m = [m[0] / l, m[1] / l];
                (m, w / (m[0] * a[0] + m[1] * a[1]).max(0.5))
            } else {
                (out(i), w)
            };
            let q = [p(i)[0] + o[0] * ww, p(i)[1] + o[1] * ww];
            nodes.push(node(q, j == 0 || j == c || sharp(i)));
        }
        // the far edges run from node c + 1 to the last; the side edges are c and the last
        let total = nodes.len();
        far.push((c + 1..total - 1).collect());
        sides.push([c, total - 1]);
        let l = d.regions.len() + 1;
        let opens = [touching[&((s + c) % n)] < 2, touching[&(s % n)] < 2];
        let seed = doc.settings.seed.wrapping_add(s as u32 * 7919).wrapping_mul(0x9E37_79B1);
        if b.rough > 0.0 {
            // above the walls at its foot
            let walls: f64 = match &b.profile {
                Profile::Stack { parts } => parts.iter().take_while(|q| q.kind == "wall").map_while(|q| q.rise).sum(),
                _ => 0.0,
            };
            rough.insert(l, Rough { base: doc.outline.z + walls, amount: b.rough, seed, far: nodes[c + 1..].iter().map(|q| [q[0], q[1]]).collect() });
        }
        if let Some(sk) = &b.skyline {
            // the far points, the level on the right: they run from node s + c's back to node s's
            let far: Vec<P2> = nodes[c + 1..].iter().map(|q| [q[0], q[1]]).collect();
            let mut pts = vec![far[0]];
            for w in far.windows(2) {
                let k = (dist(w[0], w[1]) / FAR_STEP).ceil().max(1.0) as usize;
                pts.extend((1..=k).map(|m| lerp(w[0], w[1], m as f64 / k as f64)));
            }
            let mut open = opens;
            if !ccw {
                pts.reverse();
                open.reverse();
            }
            skylines.push(SkylineWall { l, pts, open, style: sk.style.clone(), height: sk.height, amount: b.rough, seed: seed ^ 0x5bd1_e995 });
        }
        // its open ends slope down (side edge c at node s + c, the last at node s); the far edges
        // stay cliffs, as nothing is beyond them
        let mut profiles = vec![];
        let open = [(c, opens[0]), (total - 1, opens[1])].into_iter().filter(|x| x.1).map(|x| x.0).collect::<Vec<_>>();
        if !open.is_empty() && b.z > foot + 0.5 {
            profiles = vec![None; total];
            for e in c + 1..total - 1 {
                profiles[e] = Some(Profile::Cliff);
            }
            let (slope, style) = hip(&b.profile);
            for e in open {
                profiles[e] = Some(slope.clone());
            }
            ends.insert(l, (foot, style));
        }
        d.regions.push(Region {
            name: format!("beyond outline edges {s} to {}", (s + c - 1) % n),
            nodes,
            z: b.z,
            kind: "floor".into(),
            surface: None,
            edge: None,
            noise: None,
            profile: Some(b.profile.clone()),
            profiles,
        });
    }
    Ok((Cow::Owned(d), Bands { first, far, sides, ends, rough, skylines }))
}

