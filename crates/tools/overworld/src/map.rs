//! The planar map: every loop in the document (the outline and each region) is one web of
//! shared nodes and curved edges, and the web cuts the plane into faces. Each face belongs to
//! the innermost loop around it, which says how high it is. Walls stand wherever two faces
//! of different heights meet; the outside of the web is the void, where the boundary goes.
//!
//! Edges are sampled once, so every face and wall that touches an edge uses the same points:
//! the result is watertight by construction.

use crate::doc::{node_sharp, node_xy, Doc};
use crate::geom::*;
use std::collections::HashMap;

pub const VOID: usize = usize::MAX;

#[derive(Debug)]
pub struct Face {
    /// The outer cycle (counter-clockwise) and hole cycles (clockwise), as cycle ids.
    pub outer: usize,
    pub holes: Vec<usize>,
    /// 0 = the outline's ground, i + 1 = the document's region i.
    pub region: usize,
    /// The attached paths (indices into the ribbons' owners) whose footprint covers this face.
    pub paths: Vec<usize>,
}

#[derive(Debug)]
pub struct Map {
    pub verts: Vec<P2>,
    /// Segments (pairs of vertex ids). Half-edge 2s runs segs[s][0] -> [1], 2s + 1 back.
    pub segs: Vec<[usize; 2]>,
    pub next: Vec<usize>,
    /// The face on each half-edge's left, or VOID.
    pub half_face: Vec<usize>,
    pub cycles: Vec<Vec<usize>>,
    pub faces: Vec<Face>,
    /// The level's outer edges, walked clockwise (the void on the left), as half-edge loops.
    /// Faces outside every loop (a path's footprint reaching past the outline) are void too.
    pub void_cycles: Vec<Vec<usize>>,
    /// The document's loops (0 = outline) as vertex ids, and their polygons.
    pub loops: Vec<Vec<usize>>,
    pub loop_polys: Vec<Vec<P2>>,
}

/// The document's loops (the outline, then each region) welded into one set of nodes and
/// sampled: each edge between two nodes is sampled once, by the first loop to use it, so loops
/// sharing an edge share its points. No crossing checks: that's `Map::build`'s.
pub struct Loops {
    /// The welded nodes, then the curves' sample points.
    pub verts: Vec<P2>,
    pub sharp: Vec<bool>,
    /// Each loop's welded node ids (repeats dropped), and the document node each came from.
    pub nodes: Vec<Vec<usize>>,
    pub doc_index: Vec<Vec<usize>>,
    /// Each edge's vertex ids, from its lower node id to its higher.
    pub edges: HashMap<(usize, usize), Vec<usize>>,
    /// Each loop's vertex ids, and its polygon.
    pub loops: Vec<Vec<usize>>,
    pub polys: Vec<Vec<P2>>,
}

pub fn sample_loops(doc: &Doc) -> Result<Loops, String> {
    let detail = doc.settings.detail()?;
    let s = &doc.settings;
    // 1. weld the loops' nodes
    let mut raw = vec![(&doc.outline.nodes, "outline".to_string())];
    for (i, r) in doc.regions.iter().enumerate() {
        raw.push((&r.nodes, if r.name.is_empty() { format!("region {i}") } else { r.name.clone() }));
    }
    let (mut nodes, mut sharp): (Vec<P2>, Vec<bool>) = (vec![], vec![]);
    let mut loop_nodes: Vec<Vec<usize>> = vec![];
    let mut doc_index: Vec<Vec<usize>> = vec![];
    for (ns, name) in &raw {
        let mut ids: Vec<usize> = vec![];
        let mut from: Vec<usize> = vec![];
        for (k, n) in ns.iter().enumerate() {
            if n.len() < 2 {
                return Err(format!("{name}: a node needs x and y"));
            }
            let p = node_xy(n);
            let id = match nodes.iter().position(|&q| dist(p, q) <= s.weld) {
                Some(id) => id,
                None => {
                    nodes.push(p);
                    sharp.push(false);
                    nodes.len() - 1
                }
            };
            sharp[id] |= node_sharp(n);
            if ids.last() != Some(&id) {
                ids.push(id);
                from.push(k);
            }
        }
        while ids.len() > 1 && ids[0] == ids[ids.len() - 1] {
            ids.pop();
            from.pop();
        }
        if ids.len() < 3 {
            return Err(format!("{name}: a loop needs at least 3 distinct nodes"));
        }
        loop_nodes.push(ids);
        doc_index.push(from);
    }
    // 2. sample each edge once; the first loop to use an edge shapes its curve
    let mut verts = nodes.clone();
    let mut edges: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
    for ids in &loop_nodes {
        let n = ids.len();
        for k in 0..n {
            let (a, b) = (ids[k], ids[(k + 1) % n]);
            let key = (a.min(b), a.max(b));
            if edges.contains_key(&key) {
                continue;
            }
            let (p1, p2) = (nodes[a], nodes[b]);
            let p0 = if sharp[a] { [2.0 * p1[0] - p2[0], 2.0 * p1[1] - p2[1]] } else { nodes[ids[(k + n - 1) % n]] };
            let p3 = if sharp[b] { [2.0 * p2[0] - p1[0], 2.0 * p2[1] - p1[1]] } else { nodes[ids[(k + 2) % n]] };
            let pts = sample_curve(p0, p1, p2, p3, detail.curves);
            let mut vids = vec![a];
            for p in &pts[1..pts.len() - 1] {
                verts.push(*p);
                vids.push(verts.len() - 1);
            }
            vids.push(b);
            if a > b {
                vids.reverse();
            }
            edges.insert(key, vids);
        }
    }
    let loops: Vec<Vec<usize>> = loop_nodes
        .iter()
        .map(|ids| {
            let n = ids.len();
            let mut out = vec![];
            for k in 0..n {
                let (a, b) = (ids[k], ids[(k + 1) % n]);
                let mut v = edges[&(a.min(b), a.max(b))].clone();
                if a > b {
                    v.reverse();
                }
                out.extend_from_slice(&v[..v.len() - 1]);
            }
            out
        })
        .collect();
    let loop_polys: Vec<Vec<P2>> = loops.iter().map(|l| l.iter().map(|&v| verts[v]).collect()).collect();
    Ok(Loops { verts, sharp, nodes: loop_nodes, doc_index, edges, loops, polys: loop_polys })
}

impl Map {
    pub fn from(&self, h: usize) -> usize {
        self.segs[h / 2][h % 2]
    }

    pub fn to(&self, h: usize) -> usize {
        self.segs[h / 2][1 - h % 2]
    }

    pub fn len(&self, h: usize) -> f64 {
        dist(self.verts[self.from(h)], self.verts[self.to(h)])
    }

    pub fn cycle_pts(&self, c: usize) -> Vec<P2> {
        self.half_pts(&self.cycles[c])
    }

    pub fn half_pts(&self, hs: &[usize]) -> Vec<P2> {
        hs.iter().map(|&h| self.verts[self.from(h)]).collect()
    }

    /// A point just left of half-edge h's middle: inside the face it bounds.
    fn left_point(&self, h: usize) -> P2 {
        let (a, b) = (self.verts[self.from(h)], self.verts[self.to(h)]);
        let m = lerp(a, b, 0.5);
        let d = sub(b, a);
        let l = d[0].hypot(d[1]).max(1e-9);
        [m[0] - d[1] / l * 0.01, m[1] + d[0] / l * 0.01]
    }

    pub fn build(doc: &Doc) -> Result<Map, String> {
        Self::build_with(doc, &[], &[])
    }

    /// With `ribbons`: attached paths' footprints, (path index, counter-clockwise polygon). These
    /// may cross anything: every crossing becomes a vertex. `probes` are polylines that only
    /// split the edges they cross (adding a vertex there) and are then dropped: a bridge deck's
    /// sides, so its end can share the floor edge's vertices.
    pub fn build_with(doc: &Doc, ribbons: &[(usize, Vec<P2>)], probes: &[Vec<P2>]) -> Result<Map, String> {
        let Loops { mut verts, edges, loops, polys: loop_polys, .. } = sample_loops(doc)?;
        // 3. segments, checked for crossings
        let mut keys: Vec<_> = edges.keys().copied().collect();
        keys.sort();
        let mut segs: Vec<[usize; 2]> = vec![];
        for k in &keys {
            for w in edges[k].windows(2) {
                segs.push([w[0], w[1]]);
            }
        }
        let bb: Vec<[f64; 4]> = segs
            .iter()
            .map(|&[a, b]| {
                let (p, q) = (verts[a], verts[b]);
                [p[0].min(q[0]), p[1].min(q[1]), p[0].max(q[0]), p[1].max(q[1])]
            })
            .collect();
        for i in 0..segs.len() {
            for j in i + 1..segs.len() {
                let (x, y) = (bb[i], bb[j]);
                if x[2] < y[0] || y[2] < x[0] || x[3] < y[1] || y[3] < x[1] {
                    continue;
                }
                let [a, b] = segs[i];
                let [c, d] = segs[j];
                // shared end points are fine; segments sharing one and overlapping are not
                if segments_cross(verts[a], verts[b], verts[c], verts[d]) {
                    let p = verts[a];
                    return Err(format!(
                        "loops cross near ({:.0}, {:.0}): regions may share nodes and edges but not cross",
                        p[0], p[1]
                    ));
                }
            }
        }
        // ribbons, then every crossing split
        for (_, rb) in ribbons {
            let first = verts.len();
            verts.extend_from_slice(rb);
            for i in 0..rb.len() {
                segs.push([first + i, first + (i + 1) % rb.len()]);
            }
        }
        let real = segs.len();
        for pr in probes {
            let first = verts.len();
            verts.extend_from_slice(pr);
            for i in 0..pr.len().saturating_sub(1) {
                segs.push([first + i, first + i + 1]);
            }
        }
        if !ribbons.is_empty() || !probes.is_empty() {
            let probe: Vec<bool> = (0..segs.len()).map(|i| i >= real).collect();
            segs = split_segments(&mut verts, segs, &probe);
        }
        // 4. half-edges: around each vertex by angle; next = the next one clockwise from the twin
        let nh = segs.len() * 2;
        let mut out: Vec<Vec<usize>> = vec![vec![]; verts.len()];
        for h in 0..nh {
            out[segs[h / 2][h % 2]].push(h);
        }
        let ang = |h: usize| {
            let (a, b) = (verts[segs[h / 2][h % 2]], verts[segs[h / 2][1 - h % 2]]);
            (b[1] - a[1]).atan2(b[0] - a[0])
        };
        let mut pos = vec![0; nh];
        for list in out.iter_mut() {
            list.sort_by(|&x, &y| ang(x).total_cmp(&ang(y)));
            for (i, &h) in list.iter().enumerate() {
                pos[h] = i;
            }
        }
        let mut next = vec![0; nh];
        for h in 0..nh {
            let v = segs[h / 2][1 - h % 2];
            let list = &out[v];
            let i = pos[h ^ 1];
            next[h] = list[(i + list.len() - 1) % list.len()];
        }
        let mut map = Map {
            verts,
            segs,
            next,
            half_face: vec![VOID; nh],
            cycles: vec![],
            faces: vec![],
            void_cycles: vec![],
            loops,
            loop_polys,
        };
        let mut seen = vec![false; nh];
        for h0 in 0..nh {
            if seen[h0] {
                continue;
            }
            let mut c = vec![];
            let mut h = h0;
            while !seen[h] {
                seen[h] = true;
                c.push(h);
                h = map.next[h];
            }
            map.cycles.push(c);
        }
        // 5. faces: counter-clockwise cycles; clockwise ones are holes in the smallest face
        //    around them, or the void's edge
        let areas: Vec<f64> = (0..map.cycles.len()).map(|c| signed_area(&map.cycle_pts(c))).collect();
        let polys: Vec<Vec<P2>> = (0..map.cycles.len()).map(|c| map.cycle_pts(c)).collect();
        // each counter-clockwise cycle is a face of the innermost loop around it, or void if it's
        // in none (a footprint reaching past the outline)
        let loop_areas: Vec<f64> = map.loop_polys.iter().map(|p| signed_area(p).abs()).collect();
        let mut face_of_cycle = vec![VOID; map.cycles.len()];
        for c in 0..map.cycles.len() {
            if areas[c] <= 0.0 {
                continue;
            }
            let p = map.left_point(map.cycles[c][0]);
            let region = (0..map.loops.len())
                .filter(|&l| point_in_poly(p, &map.loop_polys[l]))
                .min_by(|&x, &y| loop_areas[x].total_cmp(&loop_areas[y]));
            if let Some(region) = region {
                let paths = ribbons.iter().filter(|(_, rb)| point_in_poly(p, rb)).map(|(k, _)| *k).collect();
                face_of_cycle[c] = map.faces.len();
                map.faces.push(Face { outer: c, holes: vec![], region, paths });
            }
        }
        // clockwise cycles are holes in the smallest face around them
        for c in 0..map.cycles.len() {
            if areas[c] > 0.0 {
                continue;
            }
            let p = map.left_point(map.cycles[c][0]);
            let owner = (0..map.cycles.len())
                .filter(|&o| areas[o] > 0.0 && point_in_poly(p, &polys[o]))
                .min_by(|&x, &y| areas[x].total_cmp(&areas[y]));
            if let Some(o) = owner {
                if face_of_cycle[o] != VOID {
                    map.faces[face_of_cycle[o]].holes.push(c);
                }
            }
        }
        for f in 0..map.faces.len() {
            let mut cs = vec![map.faces[f].outer];
            cs.extend(map.faces[f].holes.iter().copied());
            for c in cs {
                for &h in &map.cycles[c] {
                    map.half_face[h] = f;
                }
            }
        }
        // the void's edge: half-edges with the void on the left and a face on the right, walked
        // round the void (turning clockwise past edges between two void faces)
        let boundary = |m: &Map, h: usize| m.half_face[h] == VOID && m.half_face[h ^ 1] != VOID;
        let mut used = vec![false; nh];
        for h0 in 0..nh {
            if used[h0] || !boundary(&map, h0) {
                continue;
            }
            let mut lp = vec![];
            let mut h = h0;
            while !used[h] {
                used[h] = true;
                lp.push(h);
                let v = map.to(h);
                let list = &out[v];
                let mut g = h ^ 1;
                let mut found = None;
                for _ in 0..list.len() {
                    g = list[(pos[g] + list.len() - 1) % list.len()];
                    if boundary(&map, g) {
                        found = Some(g);
                        break;
                    }
                }
                match found {
                    Some(g) => h = g,
                    None => break,
                }
            }
            map.void_cycles.push(lp);
        }
        Ok(map)
    }
}

/// Split segments wherever they cross or touch: each crossing becomes a new vertex shared by
/// both, and a segment's end lying on another splits that one there. Duplicates (overlaps) go,
/// and so do the pieces of `probe` segments (they only leave their crossing vertices behind).
fn split_segments(verts: &mut Vec<P2>, segs: Vec<[usize; 2]>, probe: &[bool]) -> Vec<[usize; 2]> {
    let n = segs.len();
    let bb: Vec<[f64; 4]> = segs
        .iter()
        .map(|&[a, b]| {
            let (p, q) = (verts[a], verts[b]);
            [p[0].min(q[0]) - 1e-6, p[1].min(q[1]) - 1e-6, p[0].max(q[0]) + 1e-6, p[1].max(q[1]) + 1e-6]
        })
        .collect();
    let mut cuts: Vec<Vec<(f64, usize)>> = vec![vec![]; n];
    let mut index: HashMap<(i64, i64), usize> = HashMap::new();
    let eps = 1e-7;
    for i in 0..n {
        for j in i + 1..n {
            let (x, y) = (bb[i], bb[j]);
            if x[2] < y[0] || y[2] < x[0] || x[3] < y[1] || y[3] < x[1] {
                continue;
            }
            let ([a, b], [c, d]) = (segs[i], segs[j]);
            let (pa, pb, pc, pd) = (verts[a], verts[b], verts[c], verts[d]);
            for (e, s0, s1, target) in [(c, a, b, i), (d, a, b, i), (a, c, d, j), (b, c, d, j)] {
                if e == s0 || e == s1 {
                    continue;
                }
                let (dd, t) = dist_to_seg(verts[e], verts[s0], verts[s1]);
                if dd < 1e-5 && t > eps && t < 1.0 - eps {
                    cuts[target].push((t, e));
                }
            }
            let (d1, d2, d3, d4) = (cross(pc, pd, pa), cross(pc, pd, pb), cross(pa, pb, pc), cross(pa, pb, pd));
            if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0)) && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0)) {
                let (t, u) = (d1 / (d1 - d2), d3 / (d3 - d4));
                if t > eps && t < 1.0 - eps && u > eps && u < 1.0 - eps {
                    let x = lerp(pa, pb, t);
                    let key = ((x[0] * 1e5).round() as i64, (x[1] * 1e5).round() as i64);
                    let id = *index.entry(key).or_insert_with(|| {
                        verts.push(x);
                        verts.len() - 1
                    });
                    cuts[i].push((t, id));
                    cuts[j].push((u, id));
                }
            }
        }
    }
    let mut seen = std::collections::HashSet::new();
    let mut out = vec![];
    for (i, &[a, b]) in segs.iter().enumerate() {
        let mut c = cuts[i].clone();
        c.sort_by(|x, y| x.0.total_cmp(&y.0));
        let mut chain = vec![a];
        for (_, id) in c {
            if *chain.last().unwrap() != id {
                chain.push(id);
            }
        }
        if *chain.last().unwrap() != b {
            chain.push(b);
        }
        for w in chain.windows(2) {
            if w[0] != w[1] && seen.insert((w[0].min(w[1]), w[0].max(w[1]))) && !probe[i] {
                out.push([w[0], w[1]]);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::*;

    fn sq(x0: f64, y0: f64, s: f64) -> Vec<Vec<f64>> {
        // sharp corners, so the edges stay straight
        vec![vec![x0, y0, 1.0], vec![x0 + s, y0, 1.0], vec![x0 + s, y0 + s, 1.0], vec![x0, y0 + s, 1.0]]
    }

    fn doc(outline: Vec<Vec<f64>>, regions: Vec<(Vec<Vec<f64>>, f64)>) -> Doc {
        Doc {
            name: "t".into(),
            outline: Outline { nodes: outline, z: 0.0, noise: None },
            regions: regions
                .into_iter()
                .map(|(nodes, z)| Region { name: String::new(), nodes, z, kind: "floor".into(), surface: None, edge: None, noise: None })
                .collect(),
            paths: vec![],
            boundary: BoundaryDesign::default(),
            settings: Settings::default(),
            terrain: None,
        }
    }

    #[test]
    fn island_is_a_hole_in_the_ground() {
        let m = Map::build(&doc(sq(0.0, 0.0, 1000.0), vec![(sq(300.0, 300.0, 200.0), 120.0)])).unwrap();
        assert_eq!(m.faces.len(), 2);
        let ground = m.faces.iter().find(|f| f.region == 0).unwrap();
        assert_eq!(ground.holes.len(), 1);
        assert!(m.faces.iter().any(|f| f.region == 1 && f.holes.is_empty()));
        assert_eq!(m.void_cycles.len(), 1);
    }

    #[test]
    fn region_against_the_outline_shares_its_edge() {
        // the outline with an extra node on its north side; the region takes its north-west part
        let outline = vec![
            vec![0.0, 0.0, 1.0],
            vec![1000.0, 0.0, 1.0],
            vec![1000.0, 1000.0, 1.0],
            vec![500.0, 1000.0, 1.0],
            vec![0.0, 1000.0, 1.0],
            vec![0.0, 600.0, 1.0],
        ];
        let region = vec![vec![0.0, 1000.0, 1.0], vec![0.0, 600.0, 1.0], vec![500.0, 600.0, 1.0], vec![500.0, 1000.0, 1.0]];
        let m = Map::build(&doc(outline, vec![(region, 200.0)])).unwrap();
        assert_eq!(m.faces.len(), 2);
        assert!(m.faces.iter().all(|f| f.holes.is_empty()));
        // both faces reach the void
        let vc = &m.void_cycles[0];
        let touching: std::collections::BTreeSet<_> = vc.iter().map(|&h| m.half_face[h ^ 1]).collect();
        assert_eq!(touching.len(), 2);
    }

    #[test]
    fn overlapping_edges_without_shared_nodes_are_refused() {
        // runs along the outline's west side from its corner without sharing a node at (0, 600)
        let region = vec![vec![0.0, 1000.0, 1.0], vec![0.0, 600.0, 1.0], vec![500.0, 600.0, 1.0], vec![500.0, 1000.0, 1.0]];
        assert!(Map::build(&doc(sq(0.0, 0.0, 1000.0), vec![(region, 200.0)])).is_err());
    }

    #[test]
    fn a_ribbon_past_the_outline_is_cut_by_it() {
        // a footprint from inside the square to well outside its east side
        let rb = vec![[600.0, 400.0], [1400.0, 400.0], [1400.0, 600.0], [600.0, 600.0]];
        let m = Map::build_with(&doc(sq(0.0, 0.0, 1000.0), vec![]), &[(0, rb)], &[]).unwrap();
        assert_eq!(m.faces.len(), 2, "the ground and the path's part inside");
        assert_eq!(m.faces.iter().filter(|f| f.paths == vec![0]).count(), 1);
        assert_eq!(m.void_cycles.len(), 1);
        // the void's edge is still the square: 4000 long
        let len: f64 = m.void_cycles[0].iter().map(|&h| m.len(h)).sum();
        assert!((len - 4000.0).abs() < 1e-6, "{len}");
    }

    #[test]
    fn crossing_loops_are_refused() {
        let e = Map::build(&doc(sq(0.0, 0.0, 1000.0), vec![(sq(800.0, 300.0, 400.0), 50.0)])).unwrap_err();
        assert!(e.contains("cross"), "{e}");
    }
}
