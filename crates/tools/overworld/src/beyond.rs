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

use crate::doc::{Doc, Region};
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
        return Ok((Cow::Borrowed(doc), Bands { first, far: vec![], sides: vec![] }));
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
    let mut d = doc.clone();
    let mut far = vec![];
    let mut sides = vec![];
    for &(s, c) in &runs {
        let b = edge(s).unwrap();
        let w = width(b);
        let node = |q: P2, sh: bool| if sh { vec![q[0], q[1], 1.0] } else { vec![q[0], q[1]] };
        let mut nodes: Vec<Vec<f64>> = (0..=c).map(|j| node(p(s + j), sharp(s + j))).collect();
        for j in (0..=c).rev() {
            let i = s + j;
            let ww = if j == 0 || j == c { far_at[&(i % n)] } else { w };
            let o = out(i);
            nodes.push(node([p(i)[0] + o[0] * ww, p(i)[1] + o[1] * ww], j == 0 || j == c || sharp(i)));
        }
        far.push((c + 1..2 * c + 1).collect());
        sides.push([c, 2 * c + 1]);
        d.regions.push(Region {
            name: format!("beyond outline edges {s} to {}", (s + c - 1) % n),
            nodes,
            z: b.z,
            kind: "floor".into(),
            surface: None,
            edge: None,
            noise: None,
            profile: Some(b.profile.clone()),
            profiles: vec![],
        });
    }
    Ok((Cow::Owned(d), Bands { first, far, sides }))
}

