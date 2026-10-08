//! Editing a level document: which nodes are one (welded), moving, inserting and deleting them,
//! and picking things under the pointer. No UI here, so it's tested on its own.
//!
//! Loops are numbered as the builder numbers them: 0 is the outline, i + 1 is region i.

use overworld::doc::{node_sharp, Beyond, Doc, Line, Outline, Path, Profile, Region};
use overworld::geom::{dist, dist_to_seg, lerp, point_in_poly, signed_area, P2};
use overworld::map::sample_loops;
use overworld::paths::centre_line;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeRef {
    /// (loop, node index)
    Loop(usize, usize),
    /// (path, node index)
    Path(usize, usize),
    /// (line, node index): dirt paths, fences, bridges
    Line(usize, usize),
}

pub fn loop_count(doc: &Doc) -> usize {
    doc.regions.len() + 1
}

pub fn loop_nodes(doc: &Doc, l: usize) -> &Vec<Vec<f64>> {
    if l == 0 {
        &doc.outline.nodes
    } else {
        &doc.regions[l - 1].nodes
    }
}

pub fn loop_nodes_mut(doc: &mut Doc, l: usize) -> &mut Vec<Vec<f64>> {
    if l == 0 {
        &mut doc.outline.nodes
    } else {
        &mut doc.regions[l - 1].nodes
    }
}

pub fn loop_z(doc: &Doc, l: usize) -> f64 {
    if l == 0 {
        doc.outline.z
    } else {
        doc.regions[l - 1].z
    }
}

pub fn loop_name(doc: &Doc, l: usize) -> String {
    match l {
        0 => "outline".into(),
        _ if doc.regions[l - 1].name.is_empty() => format!("region {}", l - 1),
        _ => doc.regions[l - 1].name.clone(),
    }
}

pub fn path_name(doc: &Doc, p: usize) -> String {
    if doc.paths[p].name.is_empty() {
        format!("path {p}")
    } else {
        doc.paths[p].name.clone()
    }
}

pub fn line_name(doc: &Doc, k: usize) -> String {
    let l = &doc.lines[k];
    if l.name.is_empty() {
        format!("{} {k}", l.kind)
    } else {
        l.name.clone()
    }
}

pub fn node_pos(doc: &Doc, r: NodeRef) -> P2 {
    match r {
        NodeRef::Loop(l, i) => {
            let n = &loop_nodes(doc, l)[i];
            [n[0], n[1]]
        }
        NodeRef::Path(p, i) => {
            let n = &doc.paths[p].nodes[i];
            [n[0].unwrap_or(0.0), n[1].unwrap_or(0.0)]
        }
        NodeRef::Line(k, i) => {
            let n = &doc.lines[k].nodes[i];
            [n[0], n[1]]
        }
    }
}

fn set_pos(doc: &mut Doc, r: NodeRef, p: P2) {
    match r {
        NodeRef::Loop(l, i) => {
            let n = &mut loop_nodes_mut(doc, l)[i];
            n[0] = p[0];
            n[1] = p[1];
        }
        NodeRef::Path(k, i) => {
            let n = &mut doc.paths[k].nodes[i];
            n[0] = Some(p[0]);
            n[1] = Some(p[1]);
        }
        NodeRef::Line(k, i) => {
            let n = &mut doc.lines[k].nodes[i];
            n[0] = p[0];
            n[1] = p[1];
        }
    }
}

/// The node and every loop node welded to it (the builder treats them as one node).
pub fn group(doc: &Doc, r: NodeRef) -> Vec<NodeRef> {
    match r {
        NodeRef::Path(..) | NodeRef::Line(..) => vec![r],
        NodeRef::Loop(..) => {
            let p = node_pos(doc, r);
            let mut out = vec![r];
            for l in 0..loop_count(doc) {
                for (i, n) in loop_nodes(doc, l).iter().enumerate() {
                    let q = NodeRef::Loop(l, i);
                    if q != r && dist(p, [n[0], n[1]]) <= doc.settings.weld {
                        out.push(q);
                    }
                }
            }
            out
        }
    }
}

pub fn move_group(doc: &mut Doc, refs: &[NodeRef], p: P2) {
    for &r in refs {
        set_pos(doc, r, p);
    }
}

pub fn is_sharp(doc: &Doc, refs: &[NodeRef]) -> bool {
    refs.iter().any(|&r| match r {
        NodeRef::Loop(l, i) => node_sharp(&loop_nodes(doc, l)[i]),
        NodeRef::Path(..) | NodeRef::Line(..) => false,
    })
}

pub fn set_sharp(doc: &mut Doc, refs: &[NodeRef], sharp: bool) {
    for &r in refs {
        if let NodeRef::Loop(l, i) = r {
            let n = &mut loop_nodes_mut(doc, l)[i];
            n.truncate(2);
            if sharp {
                n.push(1.0);
            }
        }
    }
}

/// Deletes a node and the loop nodes welded to it. A loop keeps at least 3 nodes and a path 2.
pub fn delete_node(doc: &mut Doc, r: NodeRef) -> Result<(), String> {
    let mut refs = group(doc, r);
    for &q in &refs {
        match q {
            NodeRef::Loop(l, _) if loop_nodes(doc, l).len() <= 3 => {
                return Err(format!("{} needs at least 3 nodes: delete the region instead", loop_name(doc, l)))
            }
            NodeRef::Path(p, _) if doc.paths[p].nodes.len() <= 2 => {
                return Err(format!("{} needs at least 2 nodes: delete the path instead", path_name(doc, p)))
            }
            NodeRef::Line(k, _) if doc.lines[k].nodes.len() <= 2 => {
                return Err(format!("{} needs at least 2 nodes: delete it instead", line_name(doc, k)))
            }
            _ => {}
        }
    }
    // highest index first, so earlier indices stay valid
    refs.sort_by_key(|q| std::cmp::Reverse(match *q {
        NodeRef::Loop(_, i) | NodeRef::Path(_, i) | NodeRef::Line(_, i) => i,
    }));
    for q in refs {
        match q {
            NodeRef::Loop(l, i) => {
                loop_nodes_mut(doc, l).remove(i);
                // its two edges become one, which keeps the first one's profile (or what's beyond it)
                if l > 0 {
                    let r = &mut doc.regions[l - 1];
                    if i < r.profiles.len() {
                        r.profiles.remove(i);
                    }
                    tidy_profiles(r);
                } else {
                    let o = &mut doc.outline;
                    if i < o.beyond.len() {
                        o.beyond.remove(i);
                    }
                    tidy_beyond(o);
                }
            }
            NodeRef::Path(p, i) => {
                let path = &mut doc.paths[p];
                path.nodes.remove(i);
                if i < path.modes.len() {
                    path.modes.remove(i.min(path.modes.len() - 1));
                }
            }
            NodeRef::Line(k, i) => {
                doc.lines[k].nodes.remove(i);
            }
        }
    }
    Ok(())
}

/// Inserts a node at p on loop l's edge from its node k to the next, and on every other loop
/// that shares that edge, so it stays shared. Returns the new node in loop l.
pub fn insert_loop_node(doc: &mut Doc, l: usize, k: usize, p: P2) -> NodeRef {
    let p = [p[0].round(), p[1].round()];
    let nodes = loop_nodes(doc, l);
    let (a, b) = ([nodes[k][0], nodes[k][1]], {
        let n = &nodes[(k + 1) % nodes.len()];
        [n[0], n[1]]
    });
    let weld = doc.settings.weld;
    let mut at: Vec<(usize, usize)> = vec![];
    for m in 0..loop_count(doc) {
        let ns = loop_nodes(doc, m);
        let n = ns.len();
        for j in 0..n {
            let (x, y) = ([ns[j][0], ns[j][1]], [ns[(j + 1) % n][0], ns[(j + 1) % n][1]]);
            if (dist(x, a) <= weld && dist(y, b) <= weld) || (dist(x, b) <= weld && dist(y, a) <= weld) {
                at.push((m, j));
            }
        }
    }
    for &(m, j) in &at {
        loop_nodes_mut(doc, m).insert(j + 1, vec![p[0], p[1]]);
        // both halves of the edge keep its profile (or what's beyond it)
        if m > 0 {
            let r = &mut doc.regions[m - 1];
            if j < r.profiles.len() {
                let e = r.profiles[j].clone();
                r.profiles.insert(j + 1, e);
            }
        } else if j < doc.outline.beyond.len() {
            let e = doc.outline.beyond[j].clone();
            doc.outline.beyond.insert(j + 1, e);
        }
    }
    NodeRef::Loop(l, k + 1)
}

/// Region loop l's edges `ks` take profile `p` of their own (None: the region's again).
pub fn set_edge_profile(doc: &mut Doc, l: usize, ks: &[usize], p: Option<Profile>) {
    if l == 0 {
        return;
    }
    let r = &mut doc.regions[l - 1];
    let n = r.nodes.len();
    for &k in ks.iter().filter(|&&k| k < n) {
        if r.profiles.len() <= k {
            r.profiles.resize(k + 1, None);
        }
        r.profiles[k] = p.clone();
    }
    tidy_profiles(r);
}

/// The outline's edges `ks` have `b` beyond them (None: the forest).
pub fn set_beyond(doc: &mut Doc, ks: &[usize], b: Option<Beyond>) {
    let o = &mut doc.outline;
    let n = o.nodes.len();
    for &k in ks.iter().filter(|&&k| k < n) {
        if o.beyond.len() <= k {
            o.beyond.resize(k + 1, None);
        }
        o.beyond[k] = b.clone();
    }
    tidy_beyond(o);
}

/// After the level's theme changed: Kakariko ends at the sky, so an outline that's forest all
/// round gets Kakariko's grass slope beyond every edge; switched away again while every edge is
/// still just that, it's the forest again. Returns whether it changed.
pub fn theme_switched(doc: &mut Doc) -> bool {
    let n = doc.outline.nodes.len();
    let grass = overworld::doc::stack_looks()[0].beyond(doc.outline.z);
    let all = |doc: &Doc, b: Option<&Beyond>| (0..n).all(|k| beyond_of(doc, k) == b);
    if doc.settings.theme == "kakariko" && all(doc, None) {
        set_beyond(doc, &(0..n).collect::<Vec<_>>(), Some(grass));
        true
    } else if doc.settings.theme != "kakariko" && n > 0 && all(doc, Some(&grass)) {
        set_beyond(doc, &(0..n).collect::<Vec<_>>(), None);
        true
    } else {
        false
    }
}

/// What's beyond the outline's edge k, if not the forest.
pub fn beyond_of(doc: &Doc, k: usize) -> Option<&Beyond> {
    doc.outline.beyond.get(k).and_then(|b| b.as_ref())
}

/// What's beyond the outline's edges: none past the last edge, and no list at all when it's all
/// forest.
fn tidy_beyond(o: &mut Outline) {
    o.beyond.truncate(o.nodes.len());
    while o.beyond.last().is_some_and(|b| b.is_none()) {
        o.beyond.pop();
    }
}

/// Region loop l's edge k's own profile, if it has one.
pub fn own_edge_profile(doc: &Doc, l: usize, k: usize) -> Option<&Profile> {
    (l > 0).then(|| doc.regions[l - 1].profiles.get(k).and_then(|p| p.as_ref())).flatten()
}

/// Per-edge profiles: none past the last edge, and no list at all when none is set.
fn tidy_profiles(r: &mut Region) {
    r.profiles.truncate(r.nodes.len());
    while r.profiles.last().is_some_and(|p| p.is_none()) {
        r.profiles.pop();
    }
}

/// Inserts a node at p on a path between its nodes k and k + 1. Height is left to be
/// interpolated; a width is the neighbours' mean if both have one.
pub fn insert_path_node(doc: &mut Doc, pi: usize, k: usize, p: P2) -> NodeRef {
    let path = &mut doc.paths[pi];
    let w = |n: &Vec<Option<f64>>| n.get(3).copied().flatten();
    let mut node = vec![Some(p[0].round()), Some(p[1].round())];
    if let (Some(a), Some(b)) = (w(&path.nodes[k]), w(&path.nodes[k + 1])) {
        node.push(None);
        node.push(Some(0.5 * (a + b)));
    }
    path.nodes.insert(k + 1, node);
    if k < path.modes.len() {
        let m = path.modes[k].clone();
        path.modes.insert(k, m);
    }
    NodeRef::Path(pi, k + 1)
}

/// Inserts a node at p on a line between its nodes k and k + 1 (or, closed, after its last).
pub fn insert_line_node(doc: &mut Doc, li: usize, k: usize, p: P2) -> NodeRef {
    doc.lines[li].nodes.insert(k + 1, vec![p[0].round(), p[1].round()]);
    NodeRef::Line(li, k + 1)
}

/// The curves the builder draws, sampled, with the document segment each point starts.
pub struct Shapes {
    /// Per loop: closed polyline points, each with the node index whose edge it lies on.
    pub loops: Vec<Vec<(P2, usize)>>,
    pub areas: Vec<f64>,
    /// Per path: the centre line, each point with the node index whose segment it lies on.
    pub paths: Vec<Vec<(P2, usize)>>,
    /// Per line: as the builder draws it (dirt: a smooth curve; fences and bridges: straight).
    pub lines: Vec<Vec<(P2, usize)>>,
    /// Why the loops couldn't be sampled (then each loop is drawn straight between its nodes).
    pub error: Option<String>,
}

impl Shapes {
    pub fn new(doc: &Doc) -> Shapes {
        let (loops, error) = match sample_loops(doc) {
            Ok(s) => {
                let nn = s.sharp.len();
                let loops = (0..s.loops.len())
                    .map(|l| {
                        let (ids, from) = (&s.nodes[l], &s.doc_index[l]);
                        let mut j = 0;
                        s.loops[l]
                            .iter()
                            .map(|&v| {
                                if v < nn && ids.get(j + 1) == Some(&v) {
                                    j += 1;
                                }
                                (s.verts[v], from[j])
                            })
                            .collect()
                    })
                    .collect();
                (loops, None)
            }
            Err(e) => {
                let loops = (0..loop_count(doc))
                    .map(|l| loop_nodes(doc, l).iter().enumerate().filter(|(_, n)| n.len() >= 2).map(|(i, n)| ([n[0], n[1]], i)).collect())
                    .collect::<Vec<Vec<(P2, usize)>>>();
                (loops, Some(e))
            }
        };
        let areas = loops.iter().map(|c: &Vec<(P2, usize)>| signed_area(&c.iter().map(|x| x.0).collect::<Vec<_>>()).abs()).collect();
        let paths = doc
            .paths
            .iter()
            .map(|p| {
                let xy: Vec<P2> = p.nodes.iter().filter_map(|n| Some([n.first().copied().flatten()?, n.get(1).copied().flatten()?])).collect();
                if xy.len() < 2 {
                    return xy.into_iter().map(|q| (q, 0)).collect();
                }
                let (line, node_s) = centre_line(&xy, path_sampling(doc));
                line.into_iter()
                    .map(|(q, s)| {
                        let k = (0..xy.len() - 1).rev().find(|&k| node_s[k] <= s + 1e-9).unwrap_or(0);
                        (q, k)
                    })
                    .collect()
            })
            .collect();
        let lines = doc
            .lines
            .iter()
            .map(|l| {
                let xy: Vec<P2> = l.nodes.iter().filter(|n| n.len() >= 2).map(|n| [n[0], n[1]]).collect();
                if xy.len() < 2 {
                    return xy.into_iter().map(|q| (q, 0)).collect();
                }
                if l.kind == "dirt" || l.kind == "tunnel" {
                    let (line, node_s) = centre_line(&xy, path_sampling(doc));
                    return line.into_iter().map(|(q, s)| ((q), (0..xy.len() - 1).rev().find(|&k| node_s[k] <= s + 1e-9).unwrap_or(0))).collect();
                }
                let mut pts: Vec<(P2, usize)> = xy.iter().enumerate().map(|(i, &q)| (q, i)).collect();
                if l.closed || l.kind == "hedge" {
                    pts.push((xy[0], xy.len() - 1));
                }
                pts
            })
            .collect();
        Shapes { loops, areas, paths, lines, error }
    }

    /// The nearest line to p within `tol` (or a dirt path's half width, if `width`): (line, node
    /// index of the segment, nearest point).
    pub fn line_near(&self, doc: &Doc, p: P2, tol: f64, width: bool) -> Option<(usize, usize, P2)> {
        let mut best: Option<(f64, usize, usize, P2)> = None;
        for (k, c) in self.lines.iter().enumerate() {
            let reach = if width { tol.max(doc.lines[k].width.unwrap_or(0.0) * 0.5) } else { tol };
            for w in c.windows(2) {
                let (d, t) = dist_to_seg(p, w[0].0, w[1].0);
                if d <= reach && best.is_none_or(|x| d < x.0) {
                    best = Some((d, k, w[0].1, lerp(w[0].0, w[1].0, t)));
                }
            }
        }
        best.map(|(_, k, i, q)| (k, i, q))
    }

    pub fn poly(&self, l: usize) -> Vec<P2> {
        self.loops[l].iter().map(|x| x.0).collect()
    }

    /// The innermost loop containing p (0 for the outline), if any.
    pub fn loop_at(&self, p: P2) -> Option<usize> {
        (0..self.loops.len())
            .filter(|&l| self.loops[l].len() >= 3 && point_in_poly(p, &self.poly(l)))
            .min_by(|&a, &b| self.areas[a].total_cmp(&self.areas[b]))
    }

    /// Whether a new loop through `pts` is outside every loop (and has none inside it): a new
    /// area, apart from the rest, with its own edge of the world.
    pub fn outside_everything(&self, pts: &[P2]) -> bool {
        let n = pts.len().max(1) as f64;
        let c = [pts.iter().map(|p| p[0]).sum::<f64>() / n, pts.iter().map(|p| p[1]).sum::<f64>() / n];
        pts.iter().chain(std::iter::once(&c)).all(|&p| self.loop_at(p).is_none()) && !self.loops.iter().any(|l| l.first().is_some_and(|x| point_in_poly(x.0, pts)))
    }

    /// The nearest loop edge to p within `tol`: (loop, node index of the edge, nearest point).
    pub fn loop_edge_near(&self, p: P2, tol: f64) -> Option<(usize, usize, P2)> {
        let mut best: Option<(f64, usize, usize, P2)> = None;
        for (l, c) in self.loops.iter().enumerate() {
            let n = c.len();
            for i in 0..n {
                let (a, b) = (c[i].0, c[(i + 1) % n].0);
                let (d, t) = dist_to_seg(p, a, b);
                if d <= tol && best.is_none_or(|x| d < x.0) {
                    best = Some((d, l, c[i].1, lerp(a, b, t)));
                }
            }
        }
        best.map(|(_, l, k, q)| (l, k, q))
    }

    /// Every loop with an edge within `tol` of p, nearest first: (loop, node index of the edge,
    /// distance). A shared edge is every sharing loop's.
    pub fn loop_edges_near(&self, p: P2, tol: f64) -> Vec<(usize, usize, f64)> {
        let mut out: Vec<(usize, usize, f64)> = vec![];
        for (l, c) in self.loops.iter().enumerate() {
            let n = c.len();
            let mut best: Option<(f64, usize)> = None;
            for i in 0..n {
                let d = dist_to_seg(p, c[i].0, c[(i + 1) % n].0).0;
                if d <= tol && best.is_none_or(|x| d < x.0) {
                    best = Some((d, c[i].1));
                }
            }
            if let Some((d, k)) = best {
                out.push((l, k, d));
            }
        }
        out.sort_by(|a, b| a.2.total_cmp(&b.2));
        out
    }

    /// The nearest path centre line to p within `tol` (or within each path's half width, if
    /// `width`): (path, node index of the segment, nearest point).
    pub fn path_near(&self, doc: &Doc, p: P2, tol: f64, width: bool) -> Option<(usize, usize, P2)> {
        let mut best: Option<(f64, usize, usize, P2)> = None;
        for (k, c) in self.paths.iter().enumerate() {
            let reach = if width { tol.max(doc.paths[k].width * 0.5) } else { tol };
            for w in c.windows(2) {
                let (d, t) = dist_to_seg(p, w[0].0, w[1].0);
                if d <= reach && best.is_none_or(|x| d < x.0) {
                    best = Some((d, k, w[0].1, lerp(w[0].0, w[1].0, t)));
                }
            }
        }
        best.map(|(_, k, i, q)| (k, i, q))
    }
}

/// How the builder samples paths at the document's detail.
pub fn path_sampling(doc: &Doc) -> overworld::geom::Sampling {
    doc.settings.detail().map(|d| d.paths).unwrap_or(overworld::geom::Sampling::Every(doc.settings.sample.max(1.0) * 0.5))
}

/// The nearest node to p within `tol`. Loop nodes come before path nodes at the same spot.
pub fn node_near(doc: &Doc, p: P2, tol: f64) -> Option<NodeRef> {
    node_near_except(doc, p, tol, &[])
}

/// The nearest node to p within `tol`, other than those in `except` (the ones being dragged).
pub fn node_near_except(doc: &Doc, p: P2, tol: f64, except: &[NodeRef]) -> Option<NodeRef> {
    let mut best: Option<(f64, NodeRef)> = None;
    let mut consider = |d: f64, r: NodeRef| {
        if d <= tol && !except.contains(&r) && best.is_none_or(|x| d < x.0 - 1e-9) {
            best = Some((d, r));
        }
    };
    for (k, path) in doc.paths.iter().enumerate() {
        for i in 0..path.nodes.len() {
            let r = NodeRef::Path(k, i);
            consider(dist(p, node_pos(doc, r)), r);
        }
    }
    for (k, line) in doc.lines.iter().enumerate() {
        for i in 0..line.nodes.len() {
            let r = NodeRef::Line(k, i);
            consider(dist(p, node_pos(doc, r)), r);
        }
    }
    for l in 0..loop_count(doc) {
        for i in 0..loop_nodes(doc, l).len() {
            let r = NodeRef::Loop(l, i);
            consider(dist(p, node_pos(doc, r)) - 1e-6, r);
        }
    }
    best.map(|x| x.1)
}

/// The floor height the document gives at p (before paths): the innermost loop's.
pub fn base_z(doc: &Doc, shapes: &Shapes, p: P2) -> f64 {
    shapes.loop_at(p).map_or(doc.outline.z, |l| loop_z(doc, l))
}

pub fn new_region(doc: &Doc, nodes: Vec<P2>, z: f64) -> Region {
    let names: Vec<&str> = doc.regions.iter().map(|r| r.name.as_str()).collect();
    let name = (1..).map(|i| format!("region {i}")).find(|n| !names.contains(&n.as_str())).unwrap();
    Region { name, nodes: nodes.into_iter().map(|p| vec![p[0], p[1]]).collect(), z, kind: "floor".into(), surface: None, edge: None, noise: None, profile: None, profiles: vec![] }
}

pub fn new_path(doc: &Doc, nodes: Vec<P2>) -> Path {
    let names: Vec<&str> = doc.paths.iter().map(|r| r.name.as_str()).collect();
    let name = (1..).map(|i| format!("path {i}")).find(|n| !names.contains(&n.as_str())).unwrap();
    Path {
        name,
        nodes: nodes.into_iter().map(|p| vec![Some(p[0]), Some(p[1])]).collect(),
        width: 160.0,
        mode: "attached".into(),
        modes: vec![],
        edge: None,
        shape: None,
        // in Kakariko a path is stairs to start with: its ramps are
        look: (doc.settings.theme == "kakariko").then(|| "steps".into()),
    }
}

pub fn new_line(doc: &Doc, kind: &str, nodes: Vec<P2>) -> Line {
    let names: Vec<&str> = doc.lines.iter().map(|r| r.name.as_str()).collect();
    let name = (1..).map(|i| format!("{kind} {i}")).find(|n| !names.contains(&n.as_str())).unwrap();
    Line { name, kind: kind.into(), nodes: nodes.into_iter().map(|p| vec![p[0], p[1]]).collect(), width: None, closed: kind == "hedge", height: None, noise: None }
}

/// A tunnel's rough walls when they're first turned on.
pub fn rough_walls() -> overworld::doc::Noise {
    overworld::doc::Noise { amplitude: 25.0, scale: 300.0, edge: 120.0, seed: 0 }
}

/// A new level: an oval outline about 4000 by 2800.
pub fn blank_doc() -> Doc {
    let nodes: Vec<Vec<f64>> = (0..10)
        .map(|i| {
            let a = i as f64 / 10.0 * std::f64::consts::TAU;
            vec![(2000.0 * a.cos()).round(), (1400.0 * a.sin()).round()]
        })
        .collect();
    // new levels start light and low-poly, as the game's own: low detail, hard edges, walls
    // textured once over their height (a document without these settings keeps the old defaults)
    serde_json::from_value(serde_json::json!({
        "name": "untitled",
        "outline": { "nodes": nodes, "z": 0 },
        "settings": { "detail": "low", "edges": "hard", "wall_texture": "stretched" }
    }))
    .unwrap()
}

/// JSON with each node (an array of numbers) on one line.
pub fn to_json(doc: &Doc) -> String {
    fn scalar(v: &serde_json::Value) -> bool {
        !v.is_array() && !v.is_object()
    }
    fn put(v: &serde_json::Value, ind: usize, out: &mut String) {
        let pad = " ".repeat(ind + 2);
        match v {
            serde_json::Value::Array(a) if a.iter().all(scalar) => {
                out.push('[');
                out.push_str(&a.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "));
                out.push(']');
            }
            serde_json::Value::Array(a) => {
                out.push_str("[\n");
                for (i, x) in a.iter().enumerate() {
                    out.push_str(&pad);
                    put(x, ind + 2, out);
                    out.push_str(if i + 1 < a.len() { ",\n" } else { "\n" });
                }
                out.push_str(&" ".repeat(ind));
                out.push(']');
            }
            serde_json::Value::Object(o) => {
                out.push_str("{\n");
                for (i, (k, x)) in o.iter().enumerate() {
                    out.push_str(&pad);
                    out.push_str(&serde_json::Value::String(k.clone()).to_string());
                    out.push_str(": ");
                    put(x, ind + 2, out);
                    out.push_str(if i + 1 < o.len() { ",\n" } else { "\n" });
                }
                out.push_str(&" ".repeat(ind));
                out.push('}');
            }
            _ => out.push_str(&v.to_string()),
        }
    }
    let mut out = String::new();
    put(&serde_json::to_value(doc).unwrap(), 0, &mut out);
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An outline square with a region sharing its left edge (nodes 0 and 3 of the outline).
    fn doc() -> Doc {
        serde_json::from_value(serde_json::json!({
            "outline": { "nodes": [[0, 0, 1], [1000, 0, 1], [1000, 1000, 1], [0, 1000, 1]] },
            "regions": [ { "name": "ledge", "nodes": [[0, 0], [400, 300, 1], [400, 700, 1], [0, 1000]], "z": 120 } ],
            "paths": [ { "name": "ramp", "nodes": [[600, 500], [300, 500]] } ]
        }))
        .unwrap()
    }

    #[test]
    fn welded_nodes_move_together() {
        let mut d = doc();
        let g = group(&d, NodeRef::Loop(0, 0));
        assert_eq!(g, vec![NodeRef::Loop(0, 0), NodeRef::Loop(1, 0)]);
        move_group(&mut d, &g, [-50.0, -20.0]);
        assert_eq!(d.outline.nodes[0][..2], [-50.0, -20.0][..]);
        assert_eq!(d.regions[0].nodes[0][..2], [-50.0, -20.0][..]);
        // still one loop web: the builder accepts it
        overworld::map::Map::build(&d).unwrap();
    }

    #[test]
    fn kakariko_ends_at_a_grass_slope_unless_the_outline_has_its_own() {
        let mut d = blank_doc();
        let n = d.outline.nodes.len();
        let grass = overworld::doc::stack_looks()[0].beyond(d.outline.z);
        // forest all round: switching to Kakariko puts its grass slope beyond every edge
        d.settings.theme = "kakariko".into();
        assert!(theme_switched(&mut d));
        assert!((0..n).all(|k| beyond_of(&d, k) == Some(&grass)));
        // and back: still just that, so the forest again
        d.settings.theme = "kokiri".into();
        assert!(theme_switched(&mut d));
        assert!(d.outline.beyond.is_empty());
        // an edge set by hand: switching leaves the outline as it is
        set_beyond(&mut d, &[2], Some(stack_looks_rock()));
        let before = d.outline.clone();
        d.settings.theme = "kakariko".into();
        assert!(!theme_switched(&mut d));
        assert_eq!(d.outline, before);
    }

    fn stack_looks_rock() -> Beyond {
        overworld::doc::stack_looks()[1].beyond(0.0)
    }

    #[test]
    fn what_is_beyond_the_outline_follows_its_edges() {
        let mut d = doc();
        let n = d.outline.nodes.len();
        let rise = Beyond::new(900.0, Profile::Slope { angle: 30.0, round: 0.0 });
        set_beyond(&mut d, &[1], Some(rise.clone()));
        assert_eq!(d.outline.beyond, vec![None, Some(rise.clone())]);
        // a node on it: both halves have it
        let (a, b) = (&d.outline.nodes[1], &d.outline.nodes[2]);
        let mid = [0.5 * (a[0] + b[0]), 0.5 * (a[1] + b[1])];
        insert_loop_node(&mut d, 0, 1, mid);
        assert_eq!(d.outline.nodes.len(), n + 1);
        assert_eq!(d.outline.beyond, vec![None, Some(rise.clone()), Some(rise.clone())]);
        assert_eq!(beyond_of(&d, 2), Some(&rise));
        // the node goes again: the merged edge keeps it
        delete_node(&mut d, NodeRef::Loop(0, 2)).unwrap();
        assert_eq!(d.outline.beyond, vec![None, Some(rise.clone())]);
        // back to the forest: no list left
        set_beyond(&mut d, &[1], None);
        assert!(d.outline.beyond.is_empty());
    }

    #[test]
    fn inserting_on_a_shared_edge_inserts_in_both_loops() {
        let mut d = doc();
        // the outline's edge 3 -> 0 is the region's edge 3 -> 0 (reversed in neither: same order)
        let r = insert_loop_node(&mut d, 0, 3, [0.0, 500.0]);
        assert_eq!(r, NodeRef::Loop(0, 4));
        assert_eq!(d.outline.nodes.len(), 5);
        assert_eq!(d.regions[0].nodes.len(), 5);
        assert_eq!(d.regions[0].nodes[4][..2], [0.0, 500.0][..]);
        assert_eq!(group(&d, r).len(), 2);
        overworld::map::Map::build(&d).unwrap();
    }

    #[test]
    fn deleting_keeps_loops_closed() {
        let mut d = doc();
        // a shared node goes from both loops
        delete_node(&mut d, NodeRef::Loop(1, 0)).unwrap();
        assert_eq!((d.outline.nodes.len(), d.regions[0].nodes.len()), (3, 3));
        assert!(delete_node(&mut d, NodeRef::Loop(0, 1)).is_err());
        assert!(delete_node(&mut d, NodeRef::Path(0, 0)).is_err());
    }

    #[test]
    fn picking() {
        let d = doc();
        let s = Shapes::new(&d);
        assert!(s.error.is_none());
        assert_eq!(s.loop_at([200.0, 500.0]), Some(1));
        assert_eq!(s.loop_at([800.0, 500.0]), Some(0));
        assert_eq!(s.loop_at([2000.0, 500.0]), None);
        assert_eq!(base_z(&d, &s, [200.0, 500.0]), 120.0);
        // a new area: beside the outline, not round it or inside it
        assert!(s.outside_everything(&[[1500.0, 0.0], [2500.0, 0.0], [2500.0, 1000.0], [1500.0, 1000.0]]));
        assert!(!s.outside_everything(&[[-500.0, -500.0], [1500.0, -500.0], [1500.0, 1500.0], [-500.0, 1500.0]]));
        assert!(!s.outside_everything(&[[600.0, 100.0], [900.0, 100.0], [900.0, 400.0]]));
        // the region's right edge runs from node 1 (400, 300) to node 2 (400, 700)
        let (l, k, q) = s.loop_edge_near([405.0, 500.0], 10.0).unwrap();
        assert_eq!((l, k), (1, 1));
        assert!((q[0] - 400.0).abs() < 1.0);
        assert_eq!(s.path_near(&d, [450.0, 560.0], 10.0, true).map(|x| x.0), Some(0));
        assert_eq!(node_near(&d, [2.0, 2.0], 5.0), Some(NodeRef::Loop(0, 0)));
        // dragging the outline's corner: the region's node welded to it isn't a snap target,
        // the next corner is
        let g = group(&d, NodeRef::Loop(0, 0));
        assert_eq!(node_near_except(&d, [2.0, 2.0], 5.0, &g), None);
        assert_eq!(node_near_except(&d, [998.0, 3.0], 5.0, &g), Some(NodeRef::Loop(0, 1)));
    }

    #[test]
    fn edge_profiles_follow_their_edges() {
        let mut d = doc();
        let slope = Profile::Slope { angle: 30.0, round: 0.0 };
        // the region's right edge (node 1 to 2) slopes
        set_edge_profile(&mut d, 1, &[1], Some(slope.clone()));
        assert_eq!(d.regions[0].profiles, vec![None, Some(slope.clone())]);
        // a node on it: both halves slope
        insert_loop_node(&mut d, 1, 1, [400.0, 500.0]);
        assert_eq!(d.regions[0].profiles, vec![None, Some(slope.clone()), Some(slope.clone())]);
        // a node before it goes: the edge before it keeps its own (none)
        delete_node(&mut d, NodeRef::Loop(1, 1)).unwrap();
        assert_eq!(d.regions[0].profiles, vec![None, Some(slope.clone())]);
        assert_eq!(own_edge_profile(&d, 1, 1), Some(&slope));
        // back to the region's: no list left
        set_edge_profile(&mut d, 1, &[1], None);
        assert!(d.regions[0].profiles.is_empty());
        // shared edges are found for every loop sharing them
        let s = Shapes::new(&d);
        let near: Vec<usize> = s.loop_edges_near([0.0, 500.0], 5.0).iter().map(|x| x.0).collect();
        assert_eq!(near, vec![0, 1]);
        overworld::map::Map::build(&d).unwrap();
    }

    #[test]
    fn json_keeps_nodes_on_one_line_and_round_trips() {
        let d = doc();
        let s = to_json(&d);
        assert!(s.contains("[400.0, 300.0, 1.0]"), "{s}");
        let back: Doc = serde_json::from_str(&s).unwrap();
        assert_eq!(back, d);
    }
}
