//! From the planar map to meshes, textured by the theme's rules.
//!
//! - Floors: each face triangulated (constrained Delaunay, with interior points every
//!   `settings.steiner` for hills later), world-projected UVs so floors are seamless.
//! - Walls: wherever two faces differ in height, facing the lower one. The style comes from the
//!   higher region's `edge` or the theme's rules (height, water below). U runs along the face's
//!   boundary, continuous round corners, snapped to whole repeats round a closed loop so there's
//!   no seam; v is one stretched band (or a few on very tall walls). Wall columns are split at
//!   every height another wall or floor meets that corner, so there are no T-junctions.
//! - Paths (`paths.rs`): an attached run's footprint joins the map, and over it the ground is the
//!   higher of the region and the path's surface, so an embankment meets cliffs and the outline
//!   edge to edge. Its sides take the theme's embankment style. Floating runs are bridge decks.
//! - Overlays: grass along wall feet (not under water), roots hanging from tall cliffs.
//! - Boundary: a cliff from each floor up to the rim line, a bank back to the tree line (the
//!   outline grown by the bank's depth, simplified to long straight panels), trunks standing on
//!   the bank's edge and foliage over them. The rim rises over high floors near the edge, never
//!   faster than `rise_slope`.
//!
//! Objects: ground, water, walls, cliffs (the boundary's), bank, trees (trunks), foliage,
//! bridges, overlays.

use crate::doc::{Doc, Noise};
use crate::geom::*;
use crate::map::{Map, VOID};
use crate::mesh::Mesh;
use crate::noise::{fbm3, relief};
use crate::lines::{DirtLine, DirtPaths};
use crate::paths::{self, PathGeo};
use crate::pieces::Kit;
use crate::props::{self, Placed};
use crate::terrain::Field;
use crate::theme::{Rock, Theme, WallStyle};
use spade::{ConstrainedDelaunayTriangulation, Point2, Triangulation};
use std::collections::HashMap;

pub struct Level {
    pub mesh: Mesh,
    pub problems: Vec<String>,
    /// Lowest and highest rim (tree line base).
    pub rim: (f64, f64),
    pub faces: usize,
    /// The kit pieces placed (`props.rs`).
    pub props: Vec<Placed>,
    /// What the game's collision will hold (`Mesh::collision_vertices`): at most 8192.
    pub collision_vertices: usize,
}

struct Info {
    z: f64,
    water: Option<f64>,
    edge: Option<String>,
    noise: Option<Noise>,
}

struct Builder<'a> {
    doc: &'a Doc,
    theme: &'a Theme,
    map: Map,
    regions: Vec<Info>,
    paths: Vec<PathGeo>,
    mesh: Mesh,
    problems: Vec<String>,
    /// Heights at which something meets each vertex (floors, cliff tops): wall columns split there.
    levels: Vec<Vec<f64>>,
    rim: (f64, f64),
    /// Walls to emit once every wall's band heights are known at its corners.
    jobs: Vec<WallJob<'a>>,
    /// Where a wall changes sides along a segment (a ramp rising past a plateau): segment -> vertex.
    mids: HashMap<usize, usize>,
    /// Points a floor must have on its edges so walls meet it vertex to vertex (where an
    /// embankment's texture bands meet the ground, where a wall changes sides): half-edge ->
    /// (fraction along it, point). The floor on the half-edge's left takes them.
    extra: HashMap<usize, Vec<(f64, P2)>>,
    /// Capped walls in three bands (`Detail::walls3`).
    walls3: bool,
    /// Dirt paths painted into the floors.
    dirt: DirtPaths,
}

struct WallJob<'a> {
    obj: &'static str,
    /// The half-edge the wall stands on (its left face the higher), if it's a map edge.
    half: Option<usize>,
    p: usize,
    q: usize,
    bot: [f64; 2],
    top: [f64; 2],
    ws: &'a WallStyle,
    u: [f64; 2],
    tile: f64,
    reps: usize,
    over_water: bool,
}

/// Builds a level without a kit: props are reported, not placed.
pub fn build(doc: &Doc, theme: &Theme) -> Result<Level, String> {
    build_with(doc, theme, None)
}

/// Builds a level, placing its props from `kit` (`pieces.rs`).
pub fn build_with(doc: &Doc, theme: &Theme, kit: Option<&Kit>) -> Result<Level, String> {
    // OW_TIMING=1 prints how long each phase takes
    let timing = std::env::var_os("OW_TIMING").is_some();
    let mut clock = std::time::Instant::now();
    let mut lap = |what: &str| {
        if timing {
            eprintln!("  {what:14} {:6.1} ms", clock.elapsed().as_secs_f64() * 1000.0);
            clock = std::time::Instant::now();
        }
    };
    let mut regions = vec![Info { z: doc.outline.z, water: None, edge: None, noise: doc.outline.noise.clone() }];
    for (i, r) in doc.regions.iter().enumerate() {
        let water = match r.kind.as_str() {
            "floor" => None,
            "water" => Some(r.surface.unwrap_or(r.z + 60.0)),
            k => return Err(format!("region {i} ({}): unknown kind {k:?}", r.name)),
        };
        if let Some(e) = &r.edge {
            if !theme.wall_styles.contains_key(e) {
                return Err(format!("region {i} ({}): theme {} has no wall style {e:?}", r.name, theme.name));
            }
        }
        if let Some(n) = &r.noise {
            if n.scale <= 0.0 {
                return Err(format!("region {i} ({}): noise scale must be positive", r.name));
            }
        }
        regions.push(Info { z: r.z, water, edge: r.edge.clone(), noise: r.noise.clone() });
    }
    // paths lay themselves out on the regions' ground, then their footprints join the map
    let pre = Map::build(doc)?;
    lap("map");
    let paths: Vec<PathGeo> = {
        let areas: Vec<f64> = pre.loop_polys.iter().map(|p| signed_area(p).abs()).collect();
        let base = |p: P2| -> f64 {
            (0..pre.loop_polys.len())
                .filter(|&l| point_in_poly(p, &pre.loop_polys[l]))
                .min_by(|&x, &y| areas[x].total_cmp(&areas[y]))
                .map_or(doc.outline.z, |l| regions[l].z)
        };
        let sampling = doc.settings.detail()?.paths;
        doc.paths.iter().map(|p| paths::layout(p, &base, sampling)).collect::<Result<_, _>>()?
    };
    let mut problems = vec![];
    if !paths.is_empty() {
        let pt = theme.paths.as_ref().ok_or_else(|| format!("theme {} has no paths section", theme.name))?;
        for g in &paths {
            let side = g.edge.as_ref().unwrap_or(&pt.side);
            if !theme.wall_styles.contains_key(side) {
                return Err(format!("path {}: theme {} has no wall style {side:?}", g.name, theme.name));
            }
            if g.max_slope > pt.max_slope + 1e-9 {
                problems.push(format!("path {}: {:.0} degrees at its steepest, over the walkable {:.0}", g.name, g.max_slope, pt.max_slope));
            }
        }
    }
    let ribbons: Vec<(usize, Vec<P2>)> = paths
        .iter()
        .enumerate()
        .flat_map(|(k, g)| g.runs.iter().filter(|r| !r.floating).map(move |r| (k, g.ribbon(r))))
        .collect();
    // a deck that lands on a floor ends along the floor's edge: its sides' crossings with that
    // edge become map vertices, which the deck's end then shares
    let mut probes: Vec<Vec<P2>> = vec![];
    for g in &paths {
        for r in g.runs.iter().filter(|r| r.floating) {
            for (landed, i) in [(r.land0, r.i0), (r.land1, r.i1)] {
                if landed {
                    let (a, b) = (i.saturating_sub(16), (i + 16).min(g.st.len() - 1));
                    probes.push((a..=b).map(|j| g.sides(j).0).collect());
                    probes.push((a..=b).map(|j| g.sides(j).1).collect());
                }
            }
        }
    }
    lap("paths");
    let map = if ribbons.is_empty() && probes.is_empty() { pre } else { Map::build_with(doc, &ribbons, &probes)? };
    lap("map + paths");
    // dirt paths (lines of kind dirt): the other kinds come with fences and bridges
    let mut dirt = DirtPaths::default();
    for (i, l) in doc.lines.iter().enumerate() {
        match l.kind.as_str() {
            "dirt" => {
                let th = theme.dirt.as_ref().ok_or_else(|| format!("line {i} ({}): theme {} has no dirt section", l.name, theme.name))?;
                let seed = doc.settings.seed.wrapping_mul(0x9E37_79B9) ^ (i as u32).wrapping_mul(7919);
                dirt.lines.push(DirtLine::new(l, th, doc.settings.detail()?.paths, seed).map_err(|e| format!("line {i}: {e}"))?);
            }
            k if theme.fences.contains_key(k) => {}
            "bridge" if theme.hanging.is_some() => {}
            k => problems.push(format!("line {i} ({}): {k} lines aren't built yet", l.name)),
        }
    }
    let n = map.verts.len();
    let mut b = Builder {
        doc,
        theme,
        map,
        regions,
        paths,
        mesh: Mesh::default(),
        problems,
        levels: vec![vec![]; n],
        rim: (f64::INFINITY, f64::NEG_INFINITY),
        jobs: vec![],
        mids: HashMap::new(),
        extra: HashMap::new(),
        walls3: doc.settings.detail()?.walls3,
        dirt,
    };
    for h in 0..b.map.half_face.len() {
        let f = b.map.half_face[h];
        if f != VOID {
            let v = b.map.from(h);
            let z = b.hv(f, v);
            b.levels[v].push(z);
        }
    }
    let rims = b.rim_profiles();
    lap("rim");
    for l in b.levels.iter_mut() {
        l.sort_by(f64::total_cmp);
        l.dedup_by(|a, b| (*a - *b).abs() < 0.5);
    }
    b.walls();
    lap("walls");
    b.boundary(&rims);
    lap("boundary");
    b.edge_points();
    lap("edge_points");
    b.floors();
    lap("floors");
    b.bridges();
    lap("bridges");
    b.emit_walls();
    lap("emit_walls");
    b.paint_dirt();
    lap("dirt");
    // painted terrain deforms the finished level (before lighting, so hills are shaded)
    if let Some(t) = doc.terrain.as_ref().filter(|t| !t.is_empty()) {
        let field = Field::new(t, doc, &b.map.loop_polys);
        b.mesh.displace(|p| field.at(p));
        // the terrain can steepen a path past walkable
        if let Some(pt) = &theme.paths {
            for g in &b.paths {
                let steepest = g
                    .st
                    .windows(2)
                    .map(|w| {
                        let dz = (w[1].z + field.at(w[1].p)) - (w[0].z + field.at(w[0].p));
                        (dz.abs() / (w[1].s - w[0].s).max(1e-9)).atan().to_degrees()
                    })
                    .fold(0.0, f64::max);
                if steepest > pt.max_slope + 1e-9 && g.max_slope <= pt.max_slope + 1e-9 {
                    b.problems.push(format!("path {}: {:.0} degrees at its steepest with the painted terrain, over the walkable {:.0}", g.name, steepest, pt.max_slope));
                }
            }
        }
    }
    lap("terrain");
    // kit pieces and fences stand on the finished ground, and are lit with it
    let placed = props::place(&doc.props, kit, &mut b.mesh, &mut b.problems);
    lap("props");
    if doc.lines.iter().any(|l| theme.fences.contains_key(&l.kind) || l.kind == "bridge") {
        let ground = props::Ground::new(&b.mesh);
        let max_slope = theme.paths.as_ref().map_or(35.0, |p| p.max_slope);
        for l in &doc.lines {
            if let Some(style) = theme.fences.get(&l.kind) {
                let mut pr = crate::lines::fence(l, style, &mut b.mesh, &ground);
                b.problems.append(&mut pr);
            } else if let (true, Some(h)) = (l.kind == "bridge", &theme.hanging) {
                let mut pr = crate::lines::bridge(l, h, max_slope, &mut b.mesh, &ground);
                b.problems.append(&mut pr);
            }
        }
        lap("fences and bridges");
    }
    if let Some(l) = &theme.light {
        let seed = b.seed();
        b.mesh.shade(l, theme.variation.as_ref().map(|v| (v, seed)));
    }
    lap("lighting");
    let collision_vertices = b.mesh.collision_vertices();
    Ok(Level { faces: b.map.faces.len(), mesh: b.mesh, problems: b.problems, rim: b.rim, props: placed, collision_vertices })
}

impl<'a> Builder<'a> {
    fn seed(&self) -> u32 {
        self.theme.variation.as_ref().map_or(0, |v| v.seed) ^ self.doc.settings.seed.wrapping_mul(0x9E37_79B9)
    }

    /// Half-edge h's length in wall u: its length times the texture speed at its middle.
    fn ulen(&self, h: usize) -> f64 {
        let l = self.map.len(h);
        let Some(v) = &self.theme.variation else { return l };
        let m = lerp(self.map.verts[self.map.from(h)], self.map.verts[self.map.to(h)], 0.5);
        l * (1.0 + v.u_speed * fbm3([m[0] / v.u_scale, m[1] / v.u_scale, 0.37], self.seed())).max(0.3)
    }

    /// The ground's height in face f at p: its region's, or an attached path's surface above it.
    fn height(&self, f: usize, p: P2) -> f64 {
        let face = &self.map.faces[f];
        let mut z = self.regions[face.region].z;
        for &k in &face.paths {
            z = z.max(self.paths[k].z_at(p));
        }
        z
    }

    fn hv(&self, f: usize, v: usize) -> f64 {
        self.height(f, self.map.verts[v])
    }

    /// The path whose surface is face f's ground at p, if one is (above the region).
    fn top_path(&self, f: usize, p: P2) -> Option<usize> {
        let face = &self.map.faces[f];
        let base = self.regions[face.region].z;
        face.paths
            .iter()
            .map(|&k| (k, self.paths[k].z_at(p)))
            .filter(|&(_, z)| z > base + 0.5)
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(k, _)| k)
    }

    /// Water lies in a pond region's faces that no path covers.
    fn water(&self, f: usize) -> bool {
        let face = &self.map.faces[f];
        face.paths.is_empty() && self.regions[face.region].water.is_some()
    }

    /// The vertex where segment h's wall changes sides, `t` along h (shared by both half-edges).
    fn mid(&mut self, h: usize, t: f64) -> usize {
        let s = h / 2;
        if let Some(&v) = self.mids.get(&s) {
            return v;
        }
        let ts = if h % 2 == 0 { t } else { 1.0 - t };
        let [a, b] = self.map.segs[s];
        let p = lerp(self.map.verts[a], self.map.verts[b], ts);
        self.map.verts.push(p);
        self.levels.push(vec![]);
        let v = self.map.verts.len() - 1;
        self.mids.insert(s, v);
        v
    }

    /// The rim line (cliff tops) at each vertex of each void loop.
    fn rim_profiles(&mut self) -> Vec<(Vec<usize>, Vec<f64>)> {
        let bd = &self.doc.boundary;
        let mut out = vec![];
        for hs in self.map.void_cycles.clone() {
            let n = hs.len();
            let mut s = vec![0.0; n];
            for i in 1..n {
                s[i] = s[i - 1] + self.map.len(hs[i - 1]);
            }
            let total = s[n - 1] + self.map.len(hs[n - 1]);
            let raw: Vec<f64> = (0..n)
                .map(|i| {
                    let v = self.map.from(hs[i]);
                    let p = self.map.verts[v];
                    let f = self.hv(self.map.half_face[hs[i] ^ 1], v).max(self.hv(self.map.half_face[hs[(i + n - 1) % n] ^ 1], v));
                    let mut near = f;
                    for (r, poly) in self.map.loop_polys.iter().enumerate().skip(1) {
                        let reg = &self.regions[r];
                        if reg.water.is_none() && reg.z > near && (point_in_poly(p, poly) || dist_to_loop(p, poly) <= bd.reach) {
                            near = reg.z;
                        }
                    }
                    for g in &self.paths {
                        let (d, z) = g.dist_attached(p);
                        if d <= bd.reach && z > near {
                            near = z;
                        }
                    }
                    (self.doc.outline.z.max(near)) + bd.cliff_min
                })
                .collect();
            let top: Vec<f64> = (0..n)
                .map(|i| {
                    (0..n)
                        .map(|j| {
                            let d = (s[i] - s[j]).abs();
                            raw[j] - bd.rise_slope * d.min(total - d)
                        })
                        .fold(f64::NEG_INFINITY, f64::max)
                })
                .collect();
            for i in 0..n {
                self.levels[self.map.from(hs[i])].push(top[i]);
            }
            out.push((hs, top));
        }
        out
    }

    fn floors(&mut self) {
        let th = self.theme;
        for f in 0..self.map.faces.len() {
            let face = &self.map.faces[f];
            let outer = self.ring(&self.map.cycles[face.outer]);
            let holes: Vec<Vec<P2>> = face.holes.iter().map(|&c| self.ring(&self.map.cycles[c])).collect();
            let water = if self.water(f) { self.regions[face.region].water } else { None };
            let surf = if water.is_some() { &th.water.bed } else { &th.floor };
            // bumps: not under a path (its surface wins), denser points to show them
            let noise = self.regions[face.region].noise.as_ref().filter(|n| n.amplitude != 0.0 && face.paths.is_empty());
            let detail = self.doc.settings.detail().expect("checked by the map");
            let steiner = match noise {
                Some(n) => detail.steiner.min((n.scale / detail.bumps).max(40.0)),
                None => detail.steiner,
            };
            let mut rings = vec![outer.clone()];
            rings.extend(holes.iter().cloned());
            let ring_refs: Vec<&[P2]> = rings.iter().map(|r| r.as_slice()).collect();
            let index = SegIndex::new(&ring_refs, 200.0);
            let mut cache: HashMap<(i64, i64), f64> = HashMap::new();
            // painted terrain bends floors: they need points wherever it varies
            let mut extra = vec![];
            if let Some(t) = self.doc.terrain.as_ref().filter(|t| !t.is_empty()) {
                let d = (t.detail * detail.terrain).max(t.cell);
                let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
                for p in &outer {
                    (x0, y0, x1, y1) = (x0.min(p[0]), y0.min(p[1]), x1.max(p[0]), y1.max(p[1]));
                }
                let mut y = (y0 / d).ceil() * d;
                while y < y1 {
                    let mut x = (x0 / d).ceil() * d;
                    while x < x1 {
                        let p = [x, y];
                        if t.busy(p, d)
                            && point_in_poly(p, &outer)
                            && !holes.iter().any(|h| point_in_poly(p, h))
                            && index.dist_within(p, 0.4 * d + 1.0) > 0.4 * d
                        {
                            extra.push(p);
                        }
                        x += d;
                    }
                    y += d;
                }
            }
            // dirt paths: points on their rings (full, none) with edges between, inside this face
            let mut segs: Vec<[P2; 2]> = vec![];
            if water.is_none() && !self.dirt.is_empty() {
                let usable = |p: P2| point_in_poly(p, &outer) && !holes.iter().any(|h| point_in_poly(p, h)) && index.dist_within(p, 4.0) > 3.0;
                for run in self.dirt.rings() {
                    let mut prev: Option<P2> = None;
                    for p in run {
                        if usable(p) {
                            extra.push(p);
                            if let Some(q) = prev {
                                segs.push([q, p]);
                            }
                            prev = Some(p);
                        } else {
                            prev = None;
                        }
                    }
                }
            }
            let tris: Vec<[P3; 3]> = triangulate_full(&outer, &holes, steiner, &extra, &segs)
                .into_iter()
                .map(|t| {
                    t.map(|q| {
                        let key = ((q[0] * 1000.0).round() as i64, (q[1] * 1000.0).round() as i64);
                        let z = *cache.entry(key).or_insert_with(|| {
                            let base = self.height(f, q);
                            match noise {
                                Some(n) => {
                                    let z = base + self.bump(n, face.region, q, &index);
                                    // a pond's bed stays under its surface
                                    water.map_or(z, |w| if base < w { z.min(w - 5.0) } else { z })
                                }
                                None => base,
                            }
                        });
                        [q[0], q[1], z]
                    })
                })
                .collect();
            for p in tris {
                let uv = p.map(|q| [q[0] / surf.tile, q[1] / surf.tile]);
                self.mesh.tri("ground", p, uv, &surf.material, &surf.surface);
                if let Some(w) = water {
                    let uv = p.map(|q| [q[0] / th.water.tile, q[1] / th.water.tile]);
                    self.mesh.tri("water", p.map(|q| [q[0], q[1], w]), uv, &th.water.material, &th.water.surface);
                }
            }
        }
    }

    /// Dirt paths: each floor vertex's weight, and the floor's triangles where there's any dirt
    /// drawn with the blend material `<floor>+dirt` (the dirt's footsteps where it's mostly dirt).
    fn paint_dirt(&mut self) {
        if self.dirt.is_empty() {
            return;
        }
        let th = self.theme;
        let Some(d) = &th.dirt else { return };
        let floor = self.mesh.material_id(&th.floor.material);
        let blend = self.mesh.material_id(&format!("{}+dirt", th.floor.material));
        let surface = self.mesh.surface_id(&d.surface);
        let Some(o) = self.mesh.objects.iter_mut().find(|o| o.name == "ground") else { return };
        o.blend = o.verts.iter().map(|v| self.dirt.weight([v[0], v[1]])).collect();
        for t in 0..o.tris.len() {
            if o.mat[t] != floor {
                continue;
            }
            let w = o.tris[t].map(|v| o.blend[v]);
            if w.iter().any(|&x| x > 1e-3) {
                o.mat[t] = blend;
                if o.surf[t] >= 0 && (w[0] + w[1] + w[2]) / 3.0 >= 0.5 {
                    o.surf[t] = surface;
                }
            }
        }
    }

    /// A floor's bump at p (inside a face bounded by `rings`): the region's noise, faded out
    /// towards the face's edges so they keep the floor's height.
    fn bump(&self, n: &Noise, region: usize, p: P2, rings: &SegIndex) -> f64 {
        let d = rings.dist_within(p, n.edge.max(1.0));
        if d < 1e-6 {
            return 0.0;
        }
        let t = (d / n.edge.max(1.0)).min(1.0);
        let fade = t * t * (3.0 - 2.0 * t);
        let seed = n.seed.wrapping_add(region as u32 * 7919) ^ self.doc.settings.seed.wrapping_mul(0x85EB_CA6B);
        n.amplitude * fade * relief([p[0] / n.scale, p[1] / n.scale], seed)
    }

    /// A face boundary's points, with the extra points walls need on its edges.
    fn ring(&self, hs: &[usize]) -> Vec<P2> {
        let mut out = vec![];
        for &h in hs {
            out.push(self.map.verts[self.map.from(h)]);
            if let Some(x) = self.extra.get(&h) {
                let mut x = x.clone();
                x.sort_by(|a, b| a.0.total_cmp(&b.0));
                for (t, p) in x {
                    if t > 1e-9 && t < 1.0 - 1e-9 && out.last().is_none_or(|q: &P2| dist(*q, p) > 1e-6) {
                        out.push(p);
                    }
                }
            }
        }
        out
    }

    fn add_extra(&mut self, h: usize, p: P2) {
        let (a, b) = (self.map.verts[self.map.from(h)], self.map.verts[self.map.to(h)]);
        let t = dist_to_seg(p, a, b).1;
        self.extra.entry(h).or_default().push((t, p));
    }

    /// The points floors need on their edges: where an embankment side's texture bands meet the
    /// ground (the floor below takes them), and where a wall changes sides (both floors).
    fn edge_points(&mut self) {
        let mids: Vec<(usize, usize)> = self.mids.iter().map(|(&s, &v)| (s, v)).collect();
        for (s, v) in mids {
            let p = self.map.verts[v];
            self.add_extra(2 * s, p);
            self.add_extra(2 * s + 1, p);
        }
        let mut pts = vec![];
        for j in &self.jobs {
            let (Some(h), Some(c)) = (j.half, j.ws.caps.as_ref().filter(|c| c.anchor == "top")) else { continue };
            let (pp, qq) = (self.map.verts[j.p], self.map.verts[j.q]);
            let hh = [j.top[0] - j.bot[0], j.top[1] - j.bot[1]];
            let mut d = c.top * c.tile_v;
            while d < hh[0].max(hh[1]) {
                if (hh[0] - d) * (hh[1] - d) < 0.0 {
                    pts.push((h ^ 1, lerp(pp, qq, (d - hh[0]) / (hh[1] - hh[0]))));
                }
                d += c.unit();
            }
        }
        for (h, p) in pts {
            self.add_extra(h, p);
        }
    }

    fn walls(&mut self) {
        let th = self.theme;
        let nh = self.map.half_face.len();
        // each half-edge's wall, where its left face stands higher: (p, q, bot, top, t0, t1) with
        // t the fraction along the half-edge (a wall can change sides partway along a segment)
        type Part = (usize, usize, [f64; 2], [f64; 2], f64, f64);
        let mut part: Vec<Option<Part>> = vec![None; nh];
        let mut style: Vec<Option<String>> = vec![None; nh];
        for h in 0..nh {
            let (fl, fr) = (self.map.half_face[h], self.map.half_face[h ^ 1]);
            if fl == VOID || fr == VOID {
                continue;
            }
            let (p, q) = (self.map.from(h), self.map.to(h));
            let (hl, hr) = ([self.hv(fl, p), self.hv(fl, q)], [self.hv(fr, p), self.hv(fr, q)]);
            let (d0, d1) = (hl[0] - hr[0], hl[1] - hr[1]);
            if d0.max(d1) <= 0.5 {
                continue;
            }
            let pr: Part = if d0 >= -0.5 && d1 >= -0.5 {
                (p, q, hr, [hl[0].max(hr[0]), hl[1].max(hr[1])], 0.0, 1.0)
            } else {
                let t = d0 / (d0 - d1);
                let m = self.mid(h, t);
                let zm = hr[0] + (hr[1] - hr[0]) * t;
                if d0 > 0.0 {
                    (p, m, [hr[0], zm], [hl[0], zm], 0.0, t)
                } else {
                    (m, q, [zm, hr[1]], [zm, hl[1]], t, 1.0)
                }
            };
            let mp = lerp(self.map.verts[p], self.map.verts[q], 0.5 * (pr.4 + pr.5));
            let name = match self.top_path(fl, mp) {
                Some(k) => self.paths[k].edge.clone().unwrap_or_else(|| th.paths.as_ref().map_or("cliff".into(), |t| t.side.clone())),
                None => {
                    let own = self.regions[self.map.faces[fl].region].edge.clone();
                    let hgt = 0.5 * ((pr.3[0] - pr.2[0]) + (pr.3[1] - pr.2[1]));
                    own.unwrap_or_else(|| th.wall_style(hgt, self.water(fr)).to_string())
                }
            };
            part[h] = Some(pr);
            style[h] = Some(name);
        }
        for c in 0..self.map.cycles.len() {
            let hs = self.map.cycles[c].clone();
            if hs.iter().all(|&h| style[h].is_none()) {
                continue;
            }
            let total: f64 = hs.iter().map(|&h| self.ulen(h)).sum();
            let same = hs.iter().all(|&h| style[h] == style[hs[0]]);
            // u runs round the whole cycle; each run of one style shares its middle repeat count,
            // so neighbouring segments meet exactly
            let n = hs.len();
            let mut u_at = vec![0.0; n];
            for k in 1..n {
                u_at[k] = u_at[k - 1] + self.ulen(hs[k - 1]);
            }
            for run in runs(&hs.iter().map(|&h| style[h].clone()).collect::<Vec<_>>()) {
                let Some(name) = &style[hs[run[0]]] else { continue };
                let Some(ws) = th.wall_styles.get(name) else {
                    self.problems.push(format!("theme {} has no wall style {name:?}", th.name));
                    continue;
                };
                let tile = if same { snap(total, ws.tile_u) } else { ws.tile_u };
                let (mut lsum, mut hsum) = (0.0, 0.0);
                for &i in &run {
                    let (_, _, bot, top, t0, t1) = part[hs[i]].unwrap();
                    let l = self.map.len(hs[i]) * (t1 - t0);
                    lsum += l;
                    hsum += l * 0.5 * ((top[0] - bot[0]) + (top[1] - bot[1]));
                }
                let reps = ws.caps.as_ref().map_or(0, |c| c.repeats(hsum / lsum.max(1e-9)));
                for &i in &run {
                    let h = hs[i];
                    let (p, q, bot, top, t0, t1) = part[h].unwrap();
                    let over_water = self.water(self.map.half_face[h ^ 1]);
                    let l = self.ulen(h);
                    self.wall("walls", Some(h), p, q, bot, top, ws, [u_at[i] + t0 * l, u_at[i] + t1 * l], tile, reps, over_water);
                }
            }
        }
    }

    /// A wall column's texture bands from b up to t: (z0, z1, v0, v1). With caps and `reps`
    /// middle repeats: the bottom cap, the middle `reps` times, the top cap, the caps at their
    /// own size (squeezed on walls too short for them). Otherwise one band, the texture
    /// stretched over it (repeated per `band` units on walls taller than that).
    fn bands(ws: &WallStyle, reps: usize, b: f64, t: f64, walls3: bool) -> Vec<(f64, f64, f64, f64, bool)> {
        let h = t - b;
        match &ws.caps {
            Some(c) if reps > 0 => {
                let (hb, ht, half) = (c.bottom * c.tile_v, c.top * c.tile_v, 0.5 * c.unit() * reps as f64);
                let (hb, ht, m) = if h >= hb + ht + half {
                    (hb, ht, (h - hb - ht) / reps as f64)
                } else {
                    let s = h / (hb + ht + half);
                    (hb * s, ht * s, 0.5 * c.unit() * s)
                };
                let mut out = vec![(b, b + hb, 0.0, c.bottom, false)];
                if walls3 && c.rows > 0 {
                    // one band of the middle's own texture, v counting repeats (mirrored by its wrap)
                    out.push((b + hb, t - ht, 0.0, reps as f64, true));
                } else {
                    let (m0, m1) = c.middle();
                    for k in 0..reps {
                        let (v0, v1) = if c.mirror && k % 2 == 1 { (m1, m0) } else { (m0, m1) };
                        out.push((b + hb + k as f64 * m, b + hb + (k + 1) as f64 * m, v0, v1, false));
                    }
                }
                out.push((t - ht, t, 1.0 - c.top, 1.0, false));
                out
            }
            _ => {
                let n = match ws.band {
                    Some(band) if h > band => (h / band).round().max(1.0),
                    _ => 1.0,
                };
                vec![(b, t, 0.0, n, false)]
            }
        }
    }

    /// A wall over p -> q facing the right of p -> q, from `bot` to `top` (per end). Each texture
    /// band is zipped between the two ends on its own, so no triangle spans a jump in v; inside a
    /// band, columns are split at every height something else meets them (no T-junctions).
    #[allow(clippy::too_many_arguments)]
    fn wall(&mut self, obj: &'static str, half: Option<usize>, p: usize, q: usize, bot: [f64; 2], top: [f64; 2], ws: &'a WallStyle, u: [f64; 2], tile: f64, reps: usize, over_water: bool) {
        if (top[0] - bot[0]).max(top[1] - bot[1]) >= 0.5 {
            self.jobs.push(WallJob { obj, half, p, q, bot, top, ws, u, tile, reps, over_water });
        }
    }

    /// Every wall's band heights become levels at its corners (so the walls meeting there split
    /// at them too), then the walls are drawn.
    fn emit_walls(&mut self) {
        let jobs = std::mem::take(&mut self.jobs);
        for j in &jobs {
            for (v, b, t) in [(j.p, j.bot[0], j.top[0]), (j.q, j.bot[1], j.top[1])] {
                if let Some(c) = j.ws.caps.as_ref().filter(|c| c.anchor == "top") {
                    // band edges at fixed depths below the top
                    self.levels[v].push(b);
                    let mut d = c.top * c.tile_v;
                    while d < t - b {
                        self.levels[v].push(t - d);
                        d += c.unit();
                    }
                    self.levels[v].push(t);
                } else {
                    for (z0, z1, ..) in Self::bands(j.ws, j.reps, b, t, self.walls3) {
                        self.levels[v].push(z0);
                        self.levels[v].push(z1);
                    }
                }
            }
        }
        for l in self.levels.iter_mut() {
            l.sort_by(f64::total_cmp);
            l.dedup_by(|a, b| (*a - *b).abs() < 0.5);
        }
        for j in &jobs {
            self.emit_wall(j.obj, j.p, j.q, j.bot, j.top, j.ws, j.u, j.tile, j.reps, j.over_water);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_wall(&mut self, obj: &str, p: usize, q: usize, bot: [f64; 2], top: [f64; 2], ws: &WallStyle, u: [f64; 2], tile: f64, reps: usize, over_water: bool) {
        if ws.caps.as_ref().is_some_and(|c| c.anchor == "top") {
            self.emit_anchored(obj, p, q, bot, top, ws, u, tile);
            self.overlays(p, q, bot, top, ws, u, over_water);
            return;
        }
        let (bp, bq) = (Self::bands(ws, reps, bot[0], top[0], self.walls3), Self::bands(ws, reps, bot[1], top[1], self.walls3));
        let mid = format!("{}~mid", ws.material);
        let (pp, qq) = (self.map.verts[p], self.map.verts[q]);
        let (up, uq) = (u[0] / tile, u[1] / tile);
        let col = |s: &Self, v: usize, z0: f64, z1: f64| -> Vec<f64> {
            let mut c = vec![z0];
            c.extend(s.levels[v].iter().copied().filter(|&z| z > z0 + 0.5 && z < z1 - 0.5));
            if z1 - z0 > 1e-6 {
                c.push(z1);
            }
            c
        };
        let f = |z: f64, a: f64, b: f64| if b - a > 1e-6 { (z - a) / (b - a) } else { 0.0 };
        for (&(p0, p1, pv0, pv1, middle), &(q0, q1, qv0, qv1, _)) in bp.iter().zip(bq.iter()) {
            let mat = if middle { mid.as_str() } else { ws.material.as_str() };
            let (cp, cq) = (col(self, p, p0, p1), col(self, q, q0, q1));
            let tp: Vec<f64> = cp.iter().map(|&z| f(z, p0, p1)).collect();
            let tq: Vec<f64> = cq.iter().map(|&z| f(z, q0, q1)).collect();
            let vp = |t: f64| pv0 + (pv1 - pv0) * t;
            let vq = |t: f64| qv0 + (qv1 - qv0) * t;
            let (mut i, mut j) = (0, 0);
            while i + 1 < cp.len() || j + 1 < cq.len() {
                if j + 1 >= cq.len() || (i + 1 < cp.len() && tp[i + 1] <= tq[j + 1]) {
                    self.mesh.tri(
                        obj,
                        [[pp[0], pp[1], cp[i]], [qq[0], qq[1], cq[j]], [pp[0], pp[1], cp[i + 1]]],
                        [[up, vp(tp[i])], [uq, vq(tq[j])], [up, vp(tp[i + 1])]],
                        mat,
                        &ws.surface,
                    );
                    i += 1;
                } else {
                    self.mesh.tri(
                        obj,
                        [[pp[0], pp[1], cp[i]], [qq[0], qq[1], cq[j]], [qq[0], qq[1], cq[j + 1]]],
                        [[up, vp(tp[i])], [uq, vq(tq[j])], [uq, vq(tq[j + 1])]],
                        mat,
                        &ws.surface,
                    );
                    j += 1;
                }
            }
        }
        self.overlays(p, q, bot, top, ws, u, over_water);
    }

    /// A top-anchored wall: the top cap along the top edge, then the middle repeating downwards at
    /// fixed depths, each band clipped where the wall's foot cuts it. Bands are strips parallel
    /// to the top edge, so v is exact everywhere however the height changes along the wall.
    #[allow(clippy::too_many_arguments)]
    fn emit_anchored(&mut self, obj: &str, p: usize, q: usize, bot: [f64; 2], top: [f64; 2], ws: &WallStyle, u: [f64; 2], tile: f64) {
        let c = ws.caps.as_ref().unwrap();
        let (pp, qq) = (self.map.verts[p], self.map.verts[q]);
        let len = dist(pp, qq).max(1e-6);
        let hh = [top[0] - bot[0], top[1] - bot[1]];
        let hmax = hh[0].max(hh[1]);
        let (ht, unit) = (c.top * c.tile_v, c.unit());
        let (m0, m1) = c.middle();
        let mut bands = vec![(0.0, ht, 1.0, 1.0 - c.top)];
        let (mut d, mut k) = (ht, 0);
        while d < hmax {
            let (va, vb) = if c.mirror && k % 2 == 1 { (m0, m1) } else { (m1, m0) };
            bands.push((d, d + unit, va, vb));
            d += unit;
            k += 1;
        }
        // what meets each end column, as depths below the top
        let lv: [Vec<f64>; 2] = [(p, 0), (q, 1)].map(|(v, e)| {
            self.levels[v].iter().filter(|&&z| z > bot[e] + 0.5 && z < top[e] - 0.5).map(|&z| top[e] - z).collect()
        });
        let foot = |x: f64, d: f64| hh[0] + (hh[1] - hh[0]) * x - d;
        let want = [qq[1] - pp[1], -(qq[0] - pp[0]), 0.0];
        for (d0, d1, va, vb) in bands {
            if d0 >= hmax {
                break;
            }
            // the band [0, 1] x [d0, d1] (along the wall, depth below the top), clipped to above the foot
            let rect = [(0.0, d0), (1.0, d0), (1.0, d1), (0.0, d1)];
            let mut poly: Vec<(f64, f64)> = vec![];
            for i in 0..4 {
                let (a, b) = (rect[i], rect[(i + 1) % 4]);
                let (fa, fb) = (foot(a.0, a.1), foot(b.0, b.1));
                if fa >= -1e-9 {
                    poly.push(a);
                }
                if (fa >= -1e-9) != (fb >= -1e-9) {
                    let t = fa / (fa - fb);
                    poly.push((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
                }
            }
            if poly.len() < 3 {
                continue;
            }
            // the levels other walls meet on the end columns
            let mut full: Vec<(f64, f64)> = vec![];
            for i in 0..poly.len() {
                let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
                full.push(a);
                for (e, x) in [(0usize, 0.0), (1, 1.0)] {
                    if (a.0 - x).abs() < 1e-9 && (b.0 - x).abs() < 1e-9 {
                        let mut ds: Vec<f64> = lv[e].iter().copied().filter(|&d| d > a.1.min(b.1) + 0.25 && d < a.1.max(b.1) - 0.25).collect();
                        ds.sort_by(f64::total_cmp);
                        if b.1 < a.1 {
                            ds.reverse();
                        }
                        full.extend(ds.into_iter().map(|d| (x, d)));
                    }
                }
            }
            let flat: Vec<P2> = full.iter().map(|&(x, d)| [x * len, d]).collect();
            let flat = if signed_area(&flat) < 0.0 { flat.into_iter().rev().collect() } else { flat };
            for t in triangulate(&flat, &[], 0.0) {
                let vs = t.map(|[xl, d]| {
                    let x = xl / len;
                    let xy = lerp(pp, qq, x);
                    let z = top[0] + (top[1] - top[0]) * x - d;
                    ([xy[0], xy[1], z], [(u[0] + (u[1] - u[0]) * x) / tile, va + (vb - va) * (d - d0) / (d1 - d0)])
                });
                self.tri_facing(obj, [vs[0].0, vs[1].0, vs[2].0], [vs[0].1, vs[1].1, vs[2].1], &ws.material, &ws.surface, want);
            }
        }
    }

    /// A triangle turned to face `want`.
    #[allow(clippy::too_many_arguments)]
    fn tri_facing(&mut self, obj: &str, p: [P3; 3], uv: [[f64; 2]; 3], mat: &str, surface: &str, want: [f64; 3]) {
        let (a, b) = ([p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]], [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]]);
        let n = [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
        if n[0] * want[0] + n[1] * want[1] + n[2] * want[2] >= 0.0 {
            self.mesh.tri(obj, p, uv, mat, surface);
        } else {
            self.mesh.tri(obj, [p[0], p[2], p[1]], [uv[0], uv[2], uv[1]], mat, surface);
        }
    }

    /// The face containing p.
    fn face_at(&self, p: P2) -> Option<usize> {
        (0..self.map.faces.len()).find(|&f| self.in_face(f, p))
    }

    fn in_face(&self, f: usize, p: P2) -> bool {
        let face = &self.map.faces[f];
        point_in_poly(p, &self.map.cycle_pts(face.outer)) && !face.holes.iter().any(|&c| point_in_poly(p, &self.map.cycle_pts(c)))
    }

    /// Where a deck lands on a floor (station `il`, the floor towards the run's end if `fwd`):
    /// the last station whose sides are both off the floor, and the deck's end polygon from there
    /// to the floor's edge, following the edge's own vertices (the probes put the deck sides'
    /// crossings there).
    fn deck_end(&self, g: &PathGeo, il: usize, fwd: bool) -> Option<(usize, Vec<P3>)> {
        let n = g.st.len();
        let step = |i: usize, onward: bool| -> Option<usize> {
            if onward == fwd { (i + 1 < n).then_some(i + 1) } else { i.checked_sub(1) }
        };
        let dir = if fwd { g.st[il].dir } else { [-g.st[il].dir[0], -g.st[il].dir[1]] };
        let pin = [g.st[il].p[0] + dir[0] * 2.0, g.st[il].p[1] + dir[1] * 2.0];
        let f = self.face_at(pin)?;
        let mut j = il;
        while self.in_face(f, g.sides(j).0) || self.in_face(f, g.sides(j).1) {
            j = step(j, false)?;
        }
        let face = &self.map.faces[f];
        let rings: Vec<Vec<usize>> = std::iter::once(face.outer).chain(face.holes.iter().copied()).map(|c| self.map.cycles[c].iter().map(|&h| self.map.from(h)).collect()).collect();
        let cross = |left: bool| -> Option<(Vec<P2>, usize)> {
            let side = |i: usize| if left { g.sides(i).0 } else { g.sides(i).1 };
            let (mut pts, mut i) = (vec![], j);
            for _ in 0..64 {
                let ni = step(i, true)?;
                let (a, b) = (side(i), side(ni));
                let hit = rings
                    .iter()
                    .flatten()
                    .map(|&v| (v, dist_to_seg(self.map.verts[v], a, b)))
                    .filter(|(_, (d, _))| *d < 1e-3)
                    .min_by(|x, y| x.1 .1.total_cmp(&y.1 .1));
                if let Some((v, _)) = hit {
                    return Some((pts, v));
                }
                pts.push(b);
                i = ni;
            }
            None
        };
        let (pl, vl) = cross(true)?;
        let (pr, vr) = cross(false)?;
        // the floor's edge between the two crossings: the shorter way round the ring holding both
        let ring = rings.iter().find(|r| r.contains(&vl) && r.contains(&vr))?;
        let (il_, ir_) = (ring.iter().position(|&v| v == vl)?, ring.iter().position(|&v| v == vr)?);
        let walk = |a: usize, b: usize| -> Vec<usize> {
            let m = ring.len();
            let mut out = vec![];
            let mut k = (a + 1) % m;
            while k != b {
                out.push(ring[k]);
                k = (k + 1) % m;
            }
            out
        };
        let (c1, mut c2) = (walk(ir_, il_), walk(il_, ir_));
        c2.reverse();
        let length = |c: &[usize]| {
            let mut pts = vec![self.map.verts[vr]];
            pts.extend(c.iter().map(|&v| self.map.verts[v]));
            pts.push(self.map.verts[vl]);
            pts.windows(2).map(|w| dist(w[0], w[1])).sum::<f64>()
        };
        let chain = if length(&c1) <= length(&c2) { c1 } else { c2 };
        let mut poly: Vec<P3> = vec![];
        let (lj, rj) = g.sides(j);
        let at = |p: P2| [p[0], p[1], g.z_at(p)];
        poly.push(at(rj));
        poly.extend(pr.iter().map(|&p| at(p)));
        for v in std::iter::once(vr).chain(chain).chain(std::iter::once(vl)) {
            let p = self.map.verts[v];
            poly.push([p[0], p[1], self.height(f, p)]);
        }
        poly.extend(pl.iter().rev().map(|&p| at(p)));
        poly.push(at(lj));
        Some((j, poly))
    }

    /// Floating runs: a deck, whose top meets a landed floor along the floor's own edge, over a
    /// body: a rock arch or a slab (underside, edge strips), carried on into the ground it lands
    /// on (hidden behind the cliff) and, for rock, back into an embankment it continues.
    fn bridges(&mut self) {
        let Some(pt) = self.theme.paths.as_ref() else { return };
        let br = &pt.bridge;
        let p3 = |p: P2, z: f64| [p[0], p[1], z];
        let wuv = |p: P2, t: f64| [p[0] / t, p[1] / t];
        for g in self.paths.clone() {
            let n = g.st.len();
            let shape = g.shape.clone().unwrap_or_else(|| br.shape.clone());
            for r in g.runs.iter().filter(|r| r.floating && r.i1 > r.i0) {
                // the top: quads between the stations, end polygons where it lands
                let e0 = if r.land0 { self.deck_end(&g, r.i0, false) } else { None };
                let e1 = if r.land1 { self.deck_end(&g, r.i1, true) } else { None };
                if (r.land0 && e0.is_none()) || (r.land1 && e1.is_none()) {
                    self.problems.push(format!("path {}: couldn't find the edge a deck end lands on", g.name));
                }
                let (t0, t1) = (e0.as_ref().map_or(r.i0, |e| e.0), e1.as_ref().map_or(r.i1, |e| e.0));
                let tt = br.top.tile;
                for i in t0..t1 {
                    let ((l0, r0), (l1, r1)) = (g.sides(i), g.sides(i + 1));
                    let (z0, z1) = (g.st[i].z, g.st[i + 1].z);
                    self.mesh.quad("bridges", [p3(r0, z0), p3(r1, z1), p3(l1, z1), p3(l0, z0)],
                                   [wuv(r0, tt), wuv(r1, tt), wuv(l1, tt), wuv(l0, tt)], &br.top.material, &br.top.surface);
                }
                for (_, poly) in e0.iter().chain(e1.iter()) {
                    let flat: Vec<P2> = poly.iter().map(|p| [p[0], p[1]]).collect();
                    let zs: HashMap<(i64, i64), f64> = poly.iter().map(|p| (((p[0] * 1000.0).round() as i64, (p[1] * 1000.0).round() as i64), p[2])).collect();
                    for t in triangulate(&flat, &[], 0.0) {
                        let q = t.map(|q| [q[0], q[1], zs.get(&((q[0] * 1000.0).round() as i64, (q[1] * 1000.0).round() as i64)).copied().unwrap_or_else(|| g.z_at(q))]);
                        self.mesh.tri("bridges", q, t.map(|q| wuv(q, tt)), &br.top.material, &br.top.surface);
                    }
                }
                // the body runs on into the floor it lands on (behind the cliff, under the floor)
                let ext = |i: usize, fwd: bool, by: f64| -> usize {
                    let mut k = i;
                    while (if fwd { k + 1 < n } else { k > 0 }) && (g.st[k].s - g.st[i].s).abs() < by {
                        k = if fwd { k + 1 } else { k - 1 };
                    }
                    k
                };
                let rock = shape == "rock" && br.rock.is_some();
                let abut0 = !r.land0 && !r.cap0;
                let abut1 = !r.land1 && !r.cap1;
                let w = g.st[r.i0].w.max(g.st[r.i1].w);
                let b0 = if r.land0 || (rock && abut0) { ext(r.i0, false, 0.75 * w) } else { r.i0 };
                let b1 = if r.land1 || (rock && abut1) { ext(r.i1, true, 0.75 * w) } else { r.i1 };
                if rock {
                    let rk = br.rock.clone().unwrap();
                    let abut: Vec<(f64, f64)> = [(abut0, r.i0, 1.0), (abut1, r.i1, -1.0)]
                        .iter()
                        .filter(|a| a.0)
                        .map(|a| (g.st[a.1].s, a.2))
                        .collect();
                    self.rock_body(&g, b0, b1, (g.st[r.i0].s, g.st[r.i1].s), &abut, &rk, (r.cap0 && b0 == r.i0, r.cap1 && b1 == r.i1));
                } else {
                    self.slab_body(&g, b0, b1, (r.cap0, r.cap1));
                }
            }
        }
    }

    /// A slab under a deck: underside, edge strips along both sides, end faces where free.
    fn slab_body(&mut self, g: &PathGeo, b0: usize, b1: usize, caps: (bool, bool)) {
        let br = self.theme.paths.as_ref().unwrap().bridge.clone();
        let th = br.thickness;
        let p3 = |p: P2, z: f64| [p[0], p[1], z];
        let wuv = |p: P2, t: f64| [p[0] / t, p[1] / t];
        let tu = br.side.tile_u;
        let (mut ul, mut ur) = (0.0, 0.0);
        for i in b0..b1 {
            let ((l0, r0), (l1, r1)) = (g.sides(i), g.sides(i + 1));
            let (z0, z1) = (g.st[i].z, g.st[i + 1].z);
            let ut = br.under.tile;
            self.mesh.quad("bridges", [p3(l0, z0 - th), p3(l1, z1 - th), p3(r1, z1 - th), p3(r0, z0 - th)],
                           [wuv(l0, ut), wuv(l1, ut), wuv(r1, ut), wuv(r0, ut)], &br.under.material, &br.under.surface);
            let (lr, ll) = (dist(r0, r1), dist(l0, l1));
            self.mesh.quad("bridges", [p3(r0, z0 - th), p3(r1, z1 - th), p3(r1, z1), p3(r0, z0)],
                           [[ur / tu, 0.0], [(ur + lr) / tu, 0.0], [(ur + lr) / tu, 1.0], [ur / tu, 1.0]],
                           &br.side.material, &br.side.surface);
            self.mesh.quad("bridges", [p3(l1, z1 - th), p3(l0, z0 - th), p3(l0, z0), p3(l1, z1)],
                           [[-(ul + ll) / tu, 0.0], [-ul / tu, 0.0], [-ul / tu, 1.0], [-(ul + ll) / tu, 1.0]],
                           &br.side.material, &br.side.surface);
            ur += lr;
            ul += ll;
        }
        for (cap, i, back) in [(caps.0, b0, true), (caps.1, b1, false)] {
            if !cap {
                continue;
            }
            let (l, rr) = g.sides(i);
            let z = g.st[i].z;
            let (a, b) = if back { (l, rr) } else { (rr, l) };
            let w = dist(a, b) / tu;
            self.mesh.quad("bridges", [p3(a, z - th), p3(b, z - th), p3(b, z), p3(a, z)],
                           [[0.0, 0.0], [w, 0.0], [w, 1.0], [0.0, 1.0]], &br.side.material, &br.side.surface);
        }
    }

    /// A rock arch under a deck, stations b0..=b1. Each station has a half cross-section per side:
    /// down `lip` from the deck's edge, out by the bulge, round to the bottom centre at the arch's
    /// depth there. Its surface is textured like a top-anchored wall, by arc length down from the
    /// deck's edge, band by band, each band clipped where the cross-section ends: both halves meet
    /// on the bottom line, vertex for vertex. `abut`: (s, +1 or -1 into the bridge) where the run
    /// continues an embankment (no bulge or lumps there, so the rock stays inside it).
    #[allow(clippy::too_many_arguments)]
    fn rock_body(&mut self, g: &PathGeo, b0: usize, b1: usize, span: (f64, f64), abut: &[(f64, f64)], rk: &Rock, caps: (bool, bool)) {
        let Some(ws) = self.theme.wall_styles.get(&rk.style).cloned() else {
            self.problems.push(format!("theme {} has no wall style {:?} for rock", self.theme.name, rk.style));
            return;
        };
        let Some(c) = ws.caps.clone() else { return };
        let seed = self.seed().wrapping_add(23);
        let smooth = |a: f64, b: f64, x: f64| {
            let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        };
        // each station's half cross-section: (x out from the centre line, y down from the deck)
        struct Half {
            pts: Vec<[f64; 2]>,
            arc: Vec<f64>,
            taper: f64,
        }
        let halves: Vec<Half> = (b0..=b1)
            .map(|i| {
                let st = &g.st[i];
                let t = if span.1 - span.0 > 1e-6 { ((st.s - span.0) / (span.1 - span.0)).clamp(0.0, 1.0) } else { 0.5 };
                let depth = rk.depth_mid + (rk.depth_end - rk.depth_mid) * (1.0 - (std::f64::consts::PI * t).sin()).powf(1.5);
                let taper = abut.iter().map(|&(s, dir)| smooth(0.0, st.w, (st.s - s) * dir)).fold(1.0, f64::min);
                let hw = st.w * 0.5;
                let rx = hw * (1.0 + rk.bulge * taper);
                let lip = rk.lip.min(depth * 0.5);
                let mut pts = vec![[hw, 0.0]];
                let k = 14;
                for j in 0..=k {
                    let th = j as f64 / k as f64 * std::f64::consts::FRAC_PI_2;
                    pts.push([rx * th.cos().powf(0.8), lip + (depth - lip) * th.sin().powf(0.8)]);
                }
                let mut arc = vec![0.0];
                for w in pts.windows(2) {
                    arc.push(arc.last().unwrap() + dist(w[0], w[1]));
                }
                Half { pts, arc, taper }
            })
            .collect();
        let ht = c.top * c.tile_v;
        // a point on station ri's half (left or right), `a` down from the deck's edge, and its outward normal
        let point = |ri: usize, left: bool, a: f64| -> (P3, [f64; 3]) {
            let h = &halves[ri];
            let total = *h.arc.last().unwrap();
            let a = a.clamp(0.0, total);
            let k = (1..h.arc.len()).find(|&k| h.arc[k] >= a).unwrap_or(h.arc.len() - 1);
            let seg = (h.arc[k] - h.arc[k - 1]).max(1e-9);
            let t = (a - h.arc[k - 1]) / seg;
            let (p0, p1) = (h.pts[k - 1], h.pts[k]);
            let (x, y) = (p0[0] + (p1[0] - p0[0]) * t, p0[1] + (p1[1] - p0[1]) * t);
            let (dx, dy) = ((p1[0] - p0[0]) / seg, (p1[1] - p0[1]) / seg);
            let (nx, ny) = (dy, -dx);
            let st = &g.st[b0 + ri];
            let lat = if left { [-st.dir[1], st.dir[0]] } else { [st.dir[1], -st.dir[0]] };
            let base = [st.p[0] + lat[0] * x, st.p[1] + lat[1] * x, st.z - y];
            let amp = rk.lumps * h.taper * smooth(0.0, ht, a);
            let d = if amp > 0.0 { amp * fbm3([base[0] / rk.lump_scale, base[1] / rk.lump_scale, base[2] / rk.lump_scale], seed) } else { 0.0 };
            let (x2, y2) = (x + nx * d, y + ny * d);
            ([st.p[0] + lat[0] * x2, st.p[1] + lat[1] * x2, st.z - y2], [lat[0] * nx, lat[1] * nx, -ny])
        };
        let (m0, m1) = c.middle();
        let unit = c.unit();
        for ri in 0..halves.len() - 1 {
            let (a0, a1) = (*halves[ri].arc.last().unwrap(), *halves[ri + 1].arc.last().unwrap());
            let (s0, s1) = (g.st[b0 + ri].s, g.st[b0 + ri + 1].s);
            let mut bands = vec![(0.0, ht, 1.0, 1.0 - c.top)];
            let (mut d, mut k) = (ht, 0);
            while d < a0.max(a1) {
                let (va, vb) = if c.mirror && k % 2 == 1 { (m0, m1) } else { (m1, m0) };
                bands.push((d, d + unit, va, vb));
                d += unit;
                k += 1;
            }
            for left in [true, false] {
                for &(d0, d1, va, vb) in &bands {
                    // the band in (t along the strip, arc a), clipped to a <= the arc's length there
                    let rect = [(0.0, d0), (1.0, d0), (1.0, d1), (0.0, d1)];
                    let lim = |t: f64, a: f64| a0 + (a1 - a0) * t - a;
                    let mut poly: Vec<(f64, f64)> = vec![];
                    for i in 0..4 {
                        let (a, b) = (rect[i], rect[(i + 1) % 4]);
                        let (fa, fb) = (lim(a.0, a.1), lim(b.0, b.1));
                        if fa >= -1e-9 {
                            poly.push(a);
                        }
                        if (fa >= -1e-9) != (fb >= -1e-9) {
                            let t = fa / (fa - fb);
                            poly.push((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
                        }
                    }
                    if poly.len() < 3 {
                        continue;
                    }
                    let vs: Vec<(P3, [f64; 2], [f64; 3])> = poly
                        .iter()
                        .map(|&(t, a)| {
                            let at = a0 + (a1 - a0) * t;
                            let f = if at > 1e-9 { a / at } else { 0.0 };
                            let (p0, n0) = point(ri, left, f * a0);
                            let (p1, n1) = point(ri + 1, left, f * a1);
                            let p = [p0[0] + (p1[0] - p0[0]) * t, p0[1] + (p1[1] - p0[1]) * t, p0[2] + (p1[2] - p0[2]) * t];
                            let n = [n0[0] + n1[0], n0[1] + n1[1], n0[2] + n1[2]];
                            (p, [(s0 + (s1 - s0) * t) / ws.tile_u, va + (vb - va) * (a - d0) / (d1 - d0)], n)
                        })
                        .collect();
                    let want = vs.iter().fold([0.0; 3], |acc, v| [acc[0] + v.2[0], acc[1] + v.2[1], acc[2] + v.2[2]]);
                    for i in 1..vs.len() - 1 {
                        self.tri_facing("bridge_rock", [vs[0].0, vs[i].0, vs[i + 1].0], [vs[0].1, vs[i].1, vs[i + 1].1], &ws.material, &ws.surface, want);
                    }
                }
            }
        }
        // end faces where the rock ends in the open: its whole cross-section
        for (cap, ri, back) in [(caps.0, 0usize, true), (caps.1, halves.len() - 1, false)] {
            if !cap {
                continue;
            }
            let st = &g.st[b0 + ri];
            let h = &halves[ri];
            let mut ring: Vec<P2> = h.pts.iter().map(|p| [p[0], p[1]]).collect();
            ring.extend(h.pts.iter().rev().skip(1).map(|p| [-p[0], p[1]]));
            ring.pop();
            let ring = if signed_area(&ring) < 0.0 { ring.into_iter().rev().collect::<Vec<_>>() } else { ring };
            let right = [st.dir[1], -st.dir[0]];
            let want = if back { [-st.dir[0], -st.dir[1], 0.0] } else { [st.dir[0], st.dir[1], 0.0] };
            for t in triangulate(&ring, &[], 0.0) {
                let q = t.map(|[x, y]| [st.p[0] + right[0] * x, st.p[1] + right[1] * x, st.z - y]);
                let uv = t.map(|[x, y]| [x / ws.tile_u, 1.0 - y / c.tile_v]);
                self.tri_facing("bridge_rock", q, uv, &ws.material, &ws.surface, want);
            }
        }
    }

    /// Skirt along the foot (not under water) and fringe from the top, 1.5 in front, for styles
    /// that have them.
    #[allow(clippy::too_many_arguments)]
    fn overlays(&mut self, p: usize, q: usize, bot: [f64; 2], top: [f64; 2], ws: &WallStyle, u: [f64; 2], over_water: bool) {
        let hgt = (top[0] - bot[0]).max(top[1] - bot[1]);
        let (pp, qq) = (self.map.verts[p], self.map.verts[q]);
        let d = sub(qq, pp);
        let l = d[0].hypot(d[1]).max(1e-9);
        let (ox, oy) = (1.5 * d[1] / l, -1.5 * d[0] / l);
        let (po, qo) = ([pp[0] + ox, pp[1] + oy], [qq[0] + ox, qq[1] + oy]);
        if let Some(sk) = &ws.skirt {
            if !over_water && hgt > 20.0 {
                let zt = [(bot[0] + sk.height).min(top[0]), (bot[1] + sk.height).min(top[1])];
                let (a, b) = (u[0] / sk.tile, u[1] / sk.tile);
                self.mesh.quad(
                    "overlays",
                    [[po[0], po[1], bot[0]], [qo[0], qo[1], bot[1]], [qo[0], qo[1], zt[1]], [po[0], po[1], zt[0]]],
                    [[a, 0.0], [b, 0.0], [b, 1.0], [a, 1.0]],
                    &sk.material,
                    "",
                );
            }
        }
        if let Some(fr) = &ws.fringe {
            if hgt > ws.fringe_min && (top[0] - bot[0]).min(top[1] - bot[1]) > fr.height {
                let (a, b) = (u[0] / fr.tile, u[1] / fr.tile);
                self.mesh.quad(
                    "overlays",
                    [[po[0], po[1], top[0] - fr.height], [qo[0], qo[1], top[1] - fr.height], [qo[0], qo[1], top[1]], [po[0], po[1], top[0]]],
                    [[a, 0.0], [b, 0.0], [b, 1.0], [a, 1.0]],
                    &fr.material,
                    "",
                );
            }
        }
    }

    fn boundary(&mut self, rims: &[(Vec<usize>, Vec<f64>)]) {
        let th = self.theme;
        let bd = &self.doc.boundary;
        let Some(cliff) = th.wall_styles.get(&th.boundary_style) else {
            self.problems.push(format!("theme {} has no boundary style {:?}", th.name, th.boundary_style));
            return;
        };
        for (hs, top) in rims {
            let hs = hs.clone();
            let n = hs.len();
            let o = self.map.half_pts(&hs); // clockwise, the level on the right
            // the cliffs
            let total: f64 = hs.iter().map(|&h| self.map.len(h)).sum();
            let tile = snap(hs.iter().map(|&h| self.ulen(h)).sum(), cliff.tile_u);
            let floor = |b: &Self, i: usize| {
                let (h, f) = (hs[i], b.map.half_face[hs[i] ^ 1]);
                [b.hv(f, b.map.from(h)), b.hv(f, b.map.to(h))]
            };
            let mean: f64 = (0..n)
                .map(|i| {
                    let fl = floor(self, i);
                    self.map.len(hs[i]) * (0.5 * (top[i] + top[(i + 1) % n]) - 0.5 * (fl[0] + fl[1]))
                })
                .sum::<f64>()
                / total;
            let reps = cliff.caps.as_ref().map_or(0, |c| c.repeats(mean));
            let mut u = 0.0;
            for i in 0..n {
                let h = hs[i];
                let l = self.ulen(h);
                let f = self.map.half_face[h ^ 1];
                let w = self.water(f);
                self.wall("cliffs", Some(h), self.map.from(h), self.map.to(h), floor(self, i), [top[i], top[(i + 1) % n]], cliff, [u, u + l], tile, reps, w);
                u += l;
            }
            // the tree line and its rim heights (from the nearest cliff top)
            let t = tree_line(&o, bd.bank, bd.panel_tol);
            if t.len() < 3 {
                self.problems.push("no tree line could be traced round the outline".into());
                continue;
            }
            let rim_t: Vec<f64> = t
                .iter()
                .map(|&p| {
                    let (mut best, mut z) = (f64::INFINITY, 0.0);
                    for i in 0..n {
                        let (d, s) = dist_to_seg(p, o[i], o[(i + 1) % n]);
                        if d < best {
                            best = d;
                            z = top[i] + (top[(i + 1) % n] - top[i]) * s;
                        }
                    }
                    z + bd.bank_rise
                })
                .collect();
            for &z in &rim_t {
                self.rim = (self.rim.0.min(z), self.rim.1.max(z));
            }
            // the bank, from the cliff tops out to the tree line
            let key = |p: P2| ((p[0] * 1000.0).round() as i64, (p[1] * 1000.0).round() as i64);
            let mut zs: HashMap<(i64, i64), f64> = HashMap::new();
            for i in 0..n {
                zs.insert(key(o[i]), top[i]);
            }
            for (k, &p) in t.iter().enumerate() {
                zs.insert(key(p), rim_t[k]);
            }
            for tri in triangulate(&t, std::slice::from_ref(&o), 0.0) {
                let Some(p) = tri.iter().map(|q| zs.get(&key(*q)).map(|&z| [q[0], q[1], z])).collect::<Option<Vec<_>>>() else {
                    self.problems.push("bank triangle off the outline and tree line".into());
                    continue;
                };
                let uv = tri.map(|q| [q[0] / th.bank.tile, q[1] / th.bank.tile]);
                self.mesh.tri("bank", [p[0], p[1], p[2]], uv, &th.bank.material, &th.bank.surface);
            }
            // trunks on the tree line facing in, foliage over them and a little in front
            let tr = &th.trees;
            let r: Vec<P2> = t.iter().rev().copied().collect(); // clockwise: the level on the right
            let rz: Vec<f64> = rim_t.iter().rev().copied().collect();
            let fol = offset_right(&r, tr.foliage_in);
            let m = r.len();
            let per = |pts: &[P2]| (0..pts.len()).map(|i| dist(pts[i], pts[(i + 1) % pts.len()])).sum::<f64>();
            let (tt, tf) = (snap(per(&r), tr.trunk_tile), snap(per(&fol), tr.foliage_tile));
            let (mut ut, mut uf) = (0.0, 0.0);
            for i in 0..m {
                let j = (i + 1) % m;
                let (a, b) = (rz[i], rz[j]);
                let lt = dist(r[i], r[j]);
                self.mesh.quad(
                    "trees",
                    [[r[i][0], r[i][1], a], [r[j][0], r[j][1], b], [r[j][0], r[j][1], b + tr.trunks], [r[i][0], r[i][1], a + tr.trunks]],
                    [[ut / tt, 0.0], [(ut + lt) / tt, 0.0], [(ut + lt) / tt, 1.0], [ut / tt, 1.0]],
                    &tr.trunk_material,
                    &tr.surface,
                );
                let (fa, fb) = (a + tr.trunks - tr.overlap, b + tr.trunks - tr.overlap);
                let lf = dist(fol[i], fol[j]);
                self.mesh.quad(
                    "foliage",
                    [[fol[i][0], fol[i][1], fa], [fol[j][0], fol[j][1], fb], [fol[j][0], fol[j][1], fb + tr.foliage], [fol[i][0], fol[i][1], fa + tr.foliage]],
                    [[uf / tf, 0.0], [(uf + lf) / tf, 0.0], [(uf + lf) / tf, 1.0], [uf / tf, 1.0]],
                    &tr.foliage_material,
                    "",
                );
                ut += lt;
                uf += lf;
            }
        }
    }
}

/// Index runs of equal values round a cycle (a run never wraps unless the whole cycle is one).
fn runs<T: PartialEq>(vals: &[T]) -> Vec<Vec<usize>> {
    let n = vals.len();
    let first = (0..n).find(|&k| vals[k] != vals[(k + n - 1) % n]).unwrap_or(0);
    let mut out: Vec<Vec<usize>> = vec![];
    for k in 0..n {
        let i = (first + k) % n;
        match out.last_mut() {
            Some(r) if vals[r[0]] == vals[i] => r.push(i),
            _ => out.push(vec![i]),
        }
    }
    out
}

/// The tile length nearest `tile` that fits a whole number of times into `total`.
fn snap(total: f64, tile: f64) -> f64 {
    total / (total / tile).round().max(1.0)
}

/// Triangles (counter-clockwise) filling `outer` minus `holes`, with interior points about
/// `steiner` apart (0 for none).
pub fn triangulate(outer: &[P2], holes: &[Vec<P2>], steiner: f64) -> Vec<[P2; 3]> {
    triangulate_with(outer, holes, steiner, &[])
}

/// `triangulate` with `extra` interior points as well.
pub fn triangulate_with(outer: &[P2], holes: &[Vec<P2>], steiner: f64, extra: &[P2]) -> Vec<[P2; 3]> {
    triangulate_full(outer, holes, steiner, extra, &[])
}

/// `triangulate_with` and edges `segs` between interior points that triangles mustn't cross (a
/// dirt path's rings), where they can be added (crossing another, one is left out).
pub fn triangulate_full(outer: &[P2], holes: &[Vec<P2>], steiner: f64, extra: &[P2], segs: &[[P2; 2]]) -> Vec<[P2; 3]> {
    let mut cdt = ConstrainedDelaunayTriangulation::<Point2<f64>>::new();
    let mut rings: Vec<&[P2]> = vec![outer];
    rings.extend(holes.iter().map(|h| h.as_slice()));
    for ring in &rings {
        let hs: Vec<_> = ring.iter().map(|p| cdt.insert(Point2::new(p[0], p[1])).expect("finite point")).collect();
        for i in 0..hs.len() {
            let (a, b) = (hs[i], hs[(i + 1) % hs.len()]);
            if a != b && cdt.can_add_constraint(a, b) {
                cdt.add_constraint(a, b);
            }
        }
    }
    let inside = |p: P2| point_in_poly(p, outer) && !holes.iter().any(|h| point_in_poly(p, h));
    if steiner > 0.0 {
        let index = SegIndex::new(&rings, steiner);
        let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for p in outer {
            (x0, y0, x1, y1) = (x0.min(p[0]), y0.min(p[1]), x1.max(p[0]), y1.max(p[1]));
        }
        let mut y = (y0 / steiner).ceil() * steiner;
        while y < y1 {
            let mut x = (x0 / steiner).ceil() * steiner;
            while x < x1 {
                let p = [x, y];
                if index.dist_within(p, 0.4 * steiner + 1.0) > 0.4 * steiner && inside(p) {
                    let _ = cdt.insert(Point2::new(x, y));
                }
                x += steiner;
            }
            y += steiner;
        }
    }
    for p in extra {
        let _ = cdt.insert(Point2::new(p[0], p[1]));
    }
    for [a, b] in segs {
        if let (Ok(a), Ok(b)) = (cdt.insert(Point2::new(a[0], a[1])), cdt.insert(Point2::new(b[0], b[1]))) {
            if a != b && cdt.can_add_constraint(a, b) {
                cdt.add_constraint(a, b);
            }
        }
    }
    let mut out = vec![];
    for f in cdt.inner_faces() {
        let [a, b, c] = f.positions().map(|p| [p.x, p.y]);
        let m = [(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0];
        if inside(m) {
            out.push(if cross(a, b, c) > 0.0 { [a, b, c] } else { [a, c, b] });
        }
    }
    out
}

/// The outline grown by `bank` (a distance field traced with marching squares), simplified to
/// long straight panels within `tol`. Counter-clockwise. Notches narrower than twice the bank
/// are bridged, as a forest edge would be.
pub fn tree_line(o: &[P2], bank: f64, tol: f64) -> Vec<P2> {
    let cell = (bank / 8.0).clamp(8.0, 40.0);
    let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in o {
        (x0, y0, x1, y1) = (x0.min(p[0]), y0.min(p[1]), x1.max(p[0]), y1.max(p[1]));
    }
    let margin = bank + 4.0 * cell;
    (x0, y0, x1, y1) = (x0 - margin, y0 - margin, x1 + margin, y1 + margin);
    let nx = ((x1 - x0) / cell).ceil() as usize + 1;
    let ny = ((y1 - y0) / cell).ceil() as usize + 1;
    let at = |i: usize, j: usize| [x0 + i as f64 * cell, y0 + j as f64 * cell];
    // only the contour at distance `bank` matters: nodes further out than bank + 2 cells can't
    // neighbour a node inside it, so they needn't know how far out they are
    let reach = bank + 2.0 * cell;
    let index = SegIndex::new(&[o], reach);
    let mut f: Vec<f64> = Vec::with_capacity(nx * ny);
    for j in 0..ny {
        let xs = row_crossings(o, at(0, j)[1]);
        for i in 0..nx {
            let p = at(i, j);
            f.push(if inside_row(&xs, p[0]) { -bank } else { index.dist_within(p, reach) - bank });
        }
    }
    let val = |i: usize, j: usize| f[j * nx + i];
    type Key = (u8, usize, usize);
    let mut pos: HashMap<Key, P2> = HashMap::new();
    let mut segs: Vec<(Key, Key)> = vec![];
    for j in 0..ny - 1 {
        for i in 0..nx - 1 {
            let c = [(i, j), (i + 1, j), (i + 1, j + 1), (i, j + 1)];
            let e: [(Key, usize, usize); 4] = [((0, i, j), 0, 1), ((1, i + 1, j), 1, 2), ((0, i, j + 1), 3, 2), ((1, i, j), 0, 3)];
            let mut hit = vec![];
            for (k, (key, a, b)) in e.iter().enumerate() {
                let (fa, fb) = (val(c[*a].0, c[*a].1), val(c[*b].0, c[*b].1));
                if (fa < 0.0) != (fb < 0.0) {
                    pos.entry(*key).or_insert_with(|| lerp(at(c[*a].0, c[*a].1), at(c[*b].0, c[*b].1), fa / (fa - fb)));
                    hit.push(k);
                }
            }
            match hit.len() {
                2 => segs.push((e[hit[0]].0, e[hit[1]].0)),
                4 => {
                    segs.push((e[0].0, e[3].0));
                    segs.push((e[1].0, e[2].0));
                }
                _ => {}
            }
        }
    }
    let mut adj: HashMap<Key, Vec<usize>> = HashMap::new();
    for (s, (a, b)) in segs.iter().enumerate() {
        adj.entry(*a).or_default().push(s);
        adj.entry(*b).or_default().push(s);
    }
    let mut used = vec![false; segs.len()];
    let mut best: Vec<P2> = vec![];
    for s0 in 0..segs.len() {
        if used[s0] {
            continue;
        }
        used[s0] = true;
        let start = segs[s0].0;
        let mut k = segs[s0].1;
        let mut lp = vec![pos[&start]];
        while k != start {
            lp.push(pos[&k]);
            match adj[&k].iter().find(|&&s| !used[s]) {
                Some(&s) => {
                    used[s] = true;
                    k = if segs[s].0 == k { segs[s].1 } else { segs[s].0 };
                }
                None => break,
            }
        }
        if signed_area(&lp).abs() > signed_area(&best).abs() {
            best = lp;
        }
    }
    if best.len() < 3 {
        return best;
    }
    if signed_area(&best) < 0.0 {
        best.reverse();
    }
    simplify_closed(&best, tol)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::*;
    use std::collections::HashMap;

    /// A wobbly outline with an island plateau, a plateau against the north edge and a pond.
    pub fn sample_doc() -> Doc {
        let outline: Vec<Vec<f64>> = (0..16)
            .map(|k| {
                let a = k as f64 / 16.0 * std::f64::consts::TAU;
                let r = 1500.0 + 200.0 * (3.0 * a).sin();
                vec![r * a.cos(), r * a.sin()]
            })
            .collect();
        // the north plateau: outline nodes 3..=5, closed by two new nodes
        let mut north: Vec<Vec<f64>> = (3..=5).map(|k| outline[k].clone()).collect();
        north.push(vec![-300.0, 700.0]);
        north.push(vec![400.0, 700.0]);
        let ring = |cx: f64, cy: f64, r: f64| -> Vec<Vec<f64>> {
            (0..7).map(|k| {
                let a = k as f64 / 7.0 * std::f64::consts::TAU;
                vec![cx + r * a.cos(), cy + r * a.sin()]
            }).collect()
        };
        Doc {
            name: "test".into(),
            outline: Outline { nodes: outline, z: 0.0, noise: None },
            regions: vec![
                Region { name: "north".into(), nodes: north, z: 160.0, kind: "floor".into(), surface: None, edge: None, noise: None },
                Region { name: "island".into(), nodes: ring(-500.0, -300.0, 300.0), z: 120.0, kind: "floor".into(), surface: None, edge: None, noise: None },
                Region { name: "pond".into(), nodes: ring(500.0, -400.0, 280.0), z: -100.0, kind: "water".into(), surface: Some(-20.0), edge: None, noise: None },
            ],
            paths: vec![],
            boundary: BoundaryDesign::default(),
            settings: Settings::default(),
            terrain: None,
            props: vec![],
            lines: vec![],
        }
    }

    #[test]
    fn solid_is_watertight_up_to_the_tree_tops() {
        let doc = sample_doc();
        let lvl = build(&doc, &Theme::kokiri()).unwrap();
        assert!(lvl.problems.is_empty(), "{:?}", lvl.problems);
        let key = |p: &P3| ((p[0] * 100.0).round() as i64, (p[1] * 100.0).round() as i64, (p[2] * 100.0).round() as i64);
        let mut edges: HashMap<_, usize> = HashMap::new();
        for o in lvl.mesh.objects.iter().filter(|o| ["ground", "walls", "cliffs", "bank", "trees"].contains(&o.name.as_str())) {
            for t in &o.tris {
                for k in 0..3 {
                    let (a, b) = (key(&o.verts[t[k]]), key(&o.verts[t[(k + 1) % 3]]));
                    *edges.entry(if a < b { (a, b) } else { (b, a) }).or_default() += 1;
                }
            }
        }
        let trunks = Theme::kokiri().trees.trunks;
        let low_top = ((lvl.rim.0 + trunks) * 100.0).round() as i64 - 1;
        let bad: Vec<_> = edges.iter().filter(|&(ref e, &n)| n != 2 && !(e.0 .2 >= low_top && e.1 .2 >= low_top)).collect();
        assert!(bad.is_empty(), "{} open or non-manifold edges, e.g. {:?}", bad.len(), &bad[..bad.len().min(5)]);
    }

    #[test]
    fn rim_rises_gradually_over_the_north_plateau() {
        let doc = sample_doc();
        let mut b_doc = doc.clone();
        b_doc.regions.truncate(0);
        let flat = build(&b_doc, &Theme::kokiri()).unwrap();
        let lvl = build(&doc, &Theme::kokiri()).unwrap();
        let bd = &doc.boundary;
        assert!((flat.rim.1 - flat.rim.0).abs() < 1e-6, "a flat level has one rim height");
        assert!((flat.rim.0 - (bd.cliff_min + bd.bank_rise)).abs() < 1e-6);
        assert!((lvl.rim.1 - (160.0 + bd.cliff_min + bd.bank_rise)).abs() < 1e-6, "rim {:?}", lvl.rim);
        // and never steeper than the slope along the edge
        let map = Map::build(&doc).unwrap();
        let mut b = Builder { doc: &doc, theme: &Theme::kokiri(), regions: vec![], paths: vec![], mesh: Mesh::default(), problems: vec![], levels: vec![vec![]; map.verts.len()], rim: (0.0, 0.0), jobs: vec![], mids: HashMap::new(), extra: HashMap::new(), walls3: false, dirt: DirtPaths::default(), map };
        b.regions = std::iter::once(Info { z: 0.0, water: None, edge: None, noise: None })
            .chain(doc.regions.iter().map(|r| Info { z: r.z, water: (r.kind == "water").then_some(-20.0), edge: None, noise: None }))
            .collect();
        for (hs, top) in b.rim_profiles() {
            for i in 0..hs.len() {
                let rise = (top[(i + 1) % hs.len()] - top[i]).abs();
                assert!(rise <= bd.rise_slope * b.map.len(hs[i]) + 1e-6);
            }
        }
    }

    #[test]
    fn cliff_caps_keep_their_size_and_the_middle_repeats() {
        let th = Theme::kokiri();
        let ws = &th.wall_styles["cliff"];
        let c = ws.caps.as_ref().unwrap();
        // a 400-tall wall: caps at their own size, the middle an odd number of times (mirrored),
        // v continuous at every join
        let reps = c.repeats(400.0);
        assert!(c.mirror && reps % 2 == 1 && reps > 1, "{reps}");
        let b = Builder::bands(ws, reps, 100.0, 500.0, false);
        for w in b.windows(2) {
            assert!((w[0].3 - w[1].2).abs() < 0.5 / c.rows as f64 + 1e-9, "v meets across every join");
        }
        assert_eq!(b.len(), reps + 2);
        assert!((b[0].1 - b[0].0 - c.bottom * c.tile_v).abs() < 1e-9);
        assert!((b[reps + 1].1 - b[reps + 1].0 - c.top * c.tile_v).abs() < 1e-9);
        for w in b.windows(2) {
            assert!((w[0].1 - w[1].0).abs() < 1e-9, "bands meet");
        }
        let (m0, m1) = c.middle();
        assert!(b[1..=reps].iter().enumerate().all(|(k, m)| (m.2, m.3) == if k % 2 == 0 { (m0, m1) } else { (m1, m0) }));
        // short walls show the texture once, stretched
        assert_eq!(c.repeats(120.0), 0);
        assert_eq!(Builder::bands(ws, 0, 0.0, 120.0, false), vec![(0.0, 120.0, 0.0, 1.0, false)]);
        // just over the texture's own height, one middle about its own size
        let b = Builder::bands(ws, c.repeats(170.0), 0.0, 170.0, false);
        assert_eq!(b.len(), 3);
        assert!((b[1].1 - b[1].0 - c.unit()).abs() < 5.0);
        // three-band walls: the same caps, and one middle band counting its repeats in v
        let b3 = Builder::bands(ws, reps, 100.0, 500.0, true);
        let b = Builder::bands(ws, reps, 100.0, 500.0, false);
        assert_eq!(b3.len(), 3);
        assert_eq!((b3[0], b3[2]), (b[0], b[reps + 1]));
        assert_eq!((b3[1].0, b3[1].1, b3[1].2, b3[1].3, b3[1].4), (b[1].0, b[reps].1, 0.0, reps as f64, true));
    }

    /// A square level with a plateau (200) in its north-east, a ramp up to it from the south, a
    /// bridge from the plateau west over the ground to a second plateau, and a path that climbs
    /// as an embankment and goes on as a bridge.
    fn paths_doc() -> Doc {
        let sq = |x0: f64, y0: f64, w: f64, h: f64| -> Vec<Vec<f64>> {
            vec![vec![x0, y0, 1.0], vec![x0 + w, y0, 1.0], vec![x0 + w, y0 + h, 1.0], vec![x0, y0 + h, 1.0]]
        };
        let path = |name: &str, nodes: Vec<Vec<Option<f64>>>, modes: Vec<&str>| Path {
            name: name.into(),
            nodes,
            width: 160.0,
            mode: "attached".into(),
            modes: modes.into_iter().map(String::from).collect(),
            edge: None,
            shape: None,
        };
        let n = |x: f64, y: f64| vec![Some(x), Some(y)];
        Doc {
            name: "paths".into(),
            outline: Outline { nodes: sq(-2000.0, -2000.0, 4000.0, 4000.0), z: 0.0, noise: None },
            regions: vec![
                Region { name: "east".into(), nodes: sq(600.0, 600.0, 800.0, 800.0), z: 240.0, kind: "floor".into(), surface: None, edge: None, noise: None },
                Region { name: "west".into(), nodes: sq(-1400.0, 600.0, 800.0, 800.0), z: 240.0, kind: "floor".into(), surface: None, edge: None, noise: None },
            ],
            paths: vec![
                path("ramp", vec![n(1000.0, -300.0), n(1000.0, 900.0)], vec![]),
                path("bridge", vec![n(900.0, 1000.0), n(-900.0, 1000.0)], vec!["floating"]),
                path("climb", vec![n(-1000.0, -900.0), vec![Some(-1000.0), Some(-200.0), Some(200.0)], n(-1000.0, 900.0)], vec!["attached", "floating"]),
            ],
            boundary: BoundaryDesign::default(),
            settings: Settings::default(),
            terrain: None,
            props: vec![],
            lines: vec![],
        }
    }

    #[test]
    fn paths_build_ramps_embankments_and_decks() {
        let lvl = build(&paths_doc(), &Theme::kokiri()).unwrap();
        assert!(lvl.problems.is_empty(), "{:?}", lvl.problems);
        let obj = |n: &str| lvl.mesh.objects.iter().find(|o| o.name == n).unwrap_or_else(|| panic!("no {n}"));
        // the ramp's surface climbs from 0 to 240 in the ground
        let ground = obj("ground");
        let on_ramp: Vec<f64> = ground.verts.iter().filter(|v| (v[0] - 1000.0).abs() < 90.0 && v[1] > -250.0 && v[1] < 550.0).map(|v| v[2]).collect();
        assert!(on_ramp.iter().any(|&z| z > 50.0 && z < 190.0), "a sloped surface");
        // its sides are embankment walls (the cliff texture)
        assert!(obj("walls").mat.iter().any(|&m| lvl.mesh.materials[m] == "cliff"));
        // the bridge deck spans between the plateaus at 240, with nothing under it but ground at 0
        let br = obj("bridges");
        let tops: Vec<&P3> = br.verts.iter().filter(|v| (v[2] - 240.0).abs() < 1e-6 && (v[1] - 1000.0).abs() < 100.0).collect();
        assert!(tops.iter().any(|v| v[0].abs() < 100.0), "the deck crosses the middle");
        assert!(tops.iter().all(|v| v[0] >= -601.0 && v[0] <= 601.0), "and starts at the plateaus' edges");
        assert!(ground.verts.iter().filter(|v| v[0].abs() < 300.0 && (v[1] - 1000.0).abs() < 300.0).all(|v| v[2].abs() < 1e-6));
    }

    fn open_edges(lvl: &Level, objs: &[&str]) -> Vec<((i64, i64, i64), (i64, i64, i64))> {
        let key = |p: &P3| ((p[0] * 100.0).round() as i64, (p[1] * 100.0).round() as i64, (p[2] * 100.0).round() as i64);
        let mut edges: HashMap<_, usize> = HashMap::new();
        for o in lvl.mesh.objects.iter().filter(|o| objs.contains(&o.name.as_str())) {
            for t in &o.tris {
                for k in 0..3 {
                    let (a, b) = (key(&o.verts[t[k]]), key(&o.verts[t[(k + 1) % 3]]));
                    *edges.entry(if a < b { (a, b) } else { (b, a) }).or_default() += 1;
                }
            }
        }
        let trunks = Theme::kokiri().trees.trunks;
        let low_top = ((lvl.rim.0 + trunks) * 100.0).round() as i64 - 1;
        edges.into_iter().filter(|(e, n)| *n != 2 && !(e.0 .2 >= low_top && e.1 .2 >= low_top)).map(|(e, _)| e).collect()
    }

    /// The same level with bumps on the ground and every region.
    fn bumpy(mut doc: Doc) -> Doc {
        doc.outline.noise = Some(Noise { amplitude: 40.0, scale: 500.0, edge: 150.0, seed: 3 });
        for r in &mut doc.regions {
            r.noise = Some(Noise { amplitude: 25.0, ..Default::default() });
        }
        doc
    }

    #[test]
    fn bumps_change_only_the_ground_inside_floors() {
        let th = Theme::kokiri();
        for doc in [sample_doc(), paths_doc()] {
            let flat = build(&doc, &th).unwrap();
            let lvl = build(&bumpy(doc.clone()), &th).unwrap();
            assert!(lvl.problems.is_empty(), "{:?}", lvl.problems);
            // walls, cliffs, the bank, trees and bridges are exactly as they were
            for o in flat.mesh.objects.iter().filter(|o| !["ground", "water"].contains(&o.name.as_str())) {
                let b = lvl.mesh.objects.iter().find(|x| x.name == o.name).unwrap();
                assert!(o.verts == b.verts && o.tris == b.tris && o.colors == b.colors, "{} changed", o.name);
            }
            // the ground's edges keep their heights: still watertight
            let bad = open_edges(&lvl, &["ground", "walls", "cliffs", "bank", "trees"]);
            assert!(bad.is_empty(), "{}: {} open edges, e.g. {:?}", doc.name, bad.len(), &bad[..bad.len().min(6)]);
            // and inside, the ground is bumpy, within the amplitudes
            let bases: Vec<f64> = std::iter::once(doc.outline.z).chain(doc.regions.iter().map(|r| r.z)).collect();
            let off = |z: f64| bases.iter().map(|b| (z - b).abs()).fold(f64::INFINITY, f64::min);
            let ground = lvl.mesh.objects.iter().find(|o| o.name == "ground").unwrap();
            let bumped = ground.verts.iter().filter(|v| off(v[2]) > 8.0).count();
            assert!(bumped > 40, "{}: only {bumped} bumped vertices", doc.name);
            if doc.paths.is_empty() {
                assert!(ground.verts.iter().all(|v| off(v[2]) <= 40.0 + 1e-6));
            }
        }
    }

    #[test]
    fn paths_keep_their_surface_over_bumpy_ground() {
        let th = Theme::kokiri();
        let flat = build(&paths_doc(), &th).unwrap();
        let lvl = build(&bumpy(paths_doc()), &th).unwrap();
        // the ramp's surface (its footprint, x 920..1080) is the same with and without bumps
        let on_ramp = |l: &Level| {
            let g = l.mesh.objects.iter().find(|o| o.name == "ground").unwrap();
            let mut v: Vec<(i64, i64, i64)> = g
                .verts
                .iter()
                .filter(|v| (v[0] - 1000.0).abs() <= 80.5 && v[1] > -250.0 && v[1] < 550.0)
                .map(|v| ((v[0] * 100.0).round() as i64, (v[1] * 100.0).round() as i64, (v[2] * 100.0).round() as i64))
                .collect();
            v.sort();
            v
        };
        assert!(!on_ramp(&flat).is_empty());
        assert_eq!(on_ramp(&flat), on_ramp(&lvl));
    }

    /// A hill painted across the island and the pond's shore, and bumps over the middle.
    fn painted(mut doc: Doc) -> Doc {
        use crate::terrain::{Brush, Mode, Terrain};
        let mut t = Terrain::default();
        t.paint(&Brush { radius: 900.0, strength: 150.0, ..Default::default() }, [-500.0, -300.0], 1.0, 0.0);
        t.paint(&Brush { mode: Mode::Bumps, radius: 1200.0, strength: 60.0, bump_scale: 300.0, ..Default::default() }, [0.0, 0.0], 1.0, 0.0);
        doc.terrain = Some(t);
        doc
    }

    #[test]
    fn painted_terrain_moves_everything_with_the_ground() {
        let th = Theme::kokiri();
        for doc in [sample_doc(), paths_doc()] {
            let flat = build(&doc, &th).unwrap();
            let pd = painted(doc.clone());
            let lvl = build(&pd, &th).unwrap();
            let polys = crate::map::sample_loops(&pd).unwrap().polys;
            let field = Field::new(pd.terrain.as_ref().unwrap(), &pd, &polys);
            // every vertex outside the ground (which gains points) is where it was, raised by the
            // offset under it: walls keep their heights, decks still meet their landings
            let mut moved = 0;
            for o in flat.mesh.objects.iter().filter(|o| !["ground", "water"].contains(&o.name.as_str())) {
                let b = lvl.mesh.objects.iter().find(|x| x.name == o.name).unwrap();
                assert_eq!(o.verts.len(), b.verts.len(), "{}", o.name);
                for (v, w) in o.verts.iter().zip(&b.verts) {
                    let dz = field.at([v[0], v[1]]);
                    assert!(v[0] == w[0] && v[1] == w[1] && (w[2] - v[2] - dz).abs() < 1e-6, "{}: {v:?} -> {w:?}, offset {dz}", o.name);
                    moved += (dz.abs() > 1.0) as usize;
                }
            }
            assert!(moved > 100, "{}: the hill moved only {moved} vertices", doc.name);
            // still watertight
            let bad = open_edges(&lvl, &["ground", "walls", "cliffs", "bank", "trees"]);
            assert!(bad.is_empty(), "{}: {} open edges, e.g. {:?}", doc.name, bad.len(), &bad[..bad.len().min(6)]);
            // the floors gained points to bend
            let g = |l: &Level| l.mesh.objects.iter().find(|o| o.name == "ground").unwrap().verts.len();
            assert!(g(&lvl) > g(&flat) + 50, "{} -> {}", g(&flat), g(&lvl));
            // water stays level
            if let Some(w) = lvl.mesh.objects.iter().find(|o| o.name == "water") {
                let z0 = w.verts[0][2];
                assert!(w.verts.iter().all(|v| (v[2] - z0).abs() < 1e-6), "the pond tilted");
            }
        }
    }

    #[test]
    fn lower_detail_is_lighter_and_still_watertight() {
        let th = Theme::kokiri();
        for doc in [sample_doc(), paths_doc(), painted(bumpy(paths_doc()))] {
            let mut counts = vec![];
            for detail in ["high", "medium", "low"] {
                let mut d = doc.clone();
                d.settings.detail = detail.into();
                let lvl = build(&d, &th).unwrap();
                assert!(lvl.problems.is_empty(), "{} {detail}: {:?}", doc.name, lvl.problems);
                let bad = open_edges(&lvl, &["ground", "walls", "cliffs", "bank", "trees"]);
                assert!(bad.is_empty(), "{} {detail}: {} open edges, e.g. {:?}", doc.name, bad.len(), &bad[..bad.len().min(6)]);
                // three-band walls use the middle's own texture
                assert_eq!(lvl.mesh.materials.iter().any(|m| m.ends_with("~mid")), detail != "high", "{detail}");
                counts.push(lvl.mesh.triangles());
            }
            assert!(counts[1] < counts[0] && counts[2] < counts[1], "{}: {counts:?}", doc.name);
        }
        let mut d = sample_doc();
        d.settings.detail = "ultra".into();
        assert!(build(&d, &th).is_err());
    }

    #[test]
    fn levels_with_paths_are_watertight_too() {
        let lvl = build(&paths_doc(), &Theme::kokiri()).unwrap();
        let bad = open_edges(&lvl, &["ground", "walls", "cliffs", "bank", "trees"]);
        assert!(bad.is_empty(), "{} open or non-manifold edges, e.g. {:?}", bad.len(), &bad[..bad.len().min(6)]);
    }

    /// An opening set into a region's wall: fitted on the wall facing out, the wall's triangles
    /// round its mouth replaced so the wall is still closed everywhere but the mouth itself, and
    /// a crawlspace stretched through to the floor beyond, its far wall cut too.
    #[test]
    fn openings_are_set_into_walls() {
        use crate::doc::Prop;
        use crate::pieces::{Kit, Opening, Piece, PieceMaterial, Scale};
        let doc = sample_doc();
        let theme = Theme::kokiri();
        // a 60 x 50 tunnel 150 deep: a front frame at y 0, the far end at y -150
        let box_tunnel = |name: &str, exit: Option<u32>| {
            let ring = |y: f64| vec![[-30.0, y, 0.0], [30.0, y, 0.0], [30.0, y, 50.0], [-30.0, y, 50.0]];
            let mut verts = ring(0.0);
            verts.extend(ring(-150.0));
            Piece {
                name: name.into(),
                label: name.into(),
                kind: "opening".into(),
                materials: vec![PieceMaterial { texture: "kf_tunnel_mouth".into(), tint: [1.0; 3] }],
                normals: vec![[0.0, 0.0, 1.0]; 8],
                tris: vec![[0, 4, 5], [0, 5, 1], [1, 5, 6], [1, 6, 2], [2, 6, 7], [2, 7, 3], [3, 7, 4], [3, 4, 0]],
                uvs: vec![[[0.0; 2]; 3]; 8],
                mat: vec![0; 8],
                bounds: [[-30.0, -150.0, 0.0], [30.0, 0.0, 50.0]],
                footprint: vec![[-30.0, -150.0], [30.0, -150.0], [30.0, 0.0], [-30.0, 0.0]],
                scale: Scale { min: [1.0, 0.4, 1.0], max: [1.0, 5.0, 1.0], uniform: false },
                opening: Some(Opening { width: 60.0, height: 50.0, depth: 150.0, min_depth: 60.0, exit }),
                verts,
                ..Default::default()
            }
        };
        let kit = Kit { pieces: vec![box_tunnel("log", Some(1)), box_tunnel("crawl", None)], ..Default::default() };
        // the island squared off (sharp corners: flat walls, north and south parallel), 400 across
        let mut doc = doc;
        doc.regions[1].nodes = vec![vec![-700.0, -500.0, 1.0], vec![-300.0, -500.0, 1.0], vec![-300.0, -100.0, 1.0], vec![-700.0, -100.0, 1.0]];
        let (at, n) = ([-500.0, -540.0], [0.0, -1.0]);
        // the round island: a crawlspace can't go through it (its walls aren't flat or parallel)
        let mut round = sample_doc();
        round.props.push(Prop { piece: "crawl".into(), at: [-500.0, -640.0], z: None, yaw: 0.0, scale: [1.0; 3] });
        let lvl = build_with(&round, &theme, Some(&kit)).unwrap();
        assert!(lvl.props.is_empty() && lvl.problems.iter().any(|p| p.contains("off flat") || p.contains("parallel")), "{:?}", lvl.problems);
        for (piece, far) in [("log", false), ("crawl", true)] {
            let mut d = doc.clone();
            d.props.push(Prop { piece: piece.into(), at, z: None, yaw: 0.0, scale: [1.0; 3] });
            let lvl = build_with(&d, &theme, Some(&kit)).unwrap();
            assert_eq!(lvl.props.len(), 1, "{piece}: {:?}", lvl.problems);
            let pl = &lvl.props[0];
            // on the wall's foot, facing out
            let f = crate::props::facing(pl.yaw);
            assert!(f[0] * n[0] + f[1] * n[1] > 0.95, "{piece} faces {f:?}, the wall {n:?}");
            assert!(pl.origin[2].abs() < 1.0);
            // the crawlspace reaches the floor beyond the ridge (400 across), the log fits as it is
            if far {
                assert!((pl.scale[1] * 150.0 - 400.0).abs() < 2.0, "{piece}: {:?}", pl.scale);
            } else {
                assert_eq!(pl.scale[1], 1.0);
            }
            // closed everywhere but the mouths: open edges only round them, inside their outline
            let bad = open_edges(&lvl, &["ground", "walls", "cliffs", "bank", "trees"]);
            let mouths = if far { 2 } else { 1 };
            let near_mouth = |e: &((i64, i64, i64), (i64, i64, i64))| {
                [e.0, e.1].iter().all(|q| {
                    let p = [q.0 as f64 / 100.0, q.1 as f64 / 100.0, q.2 as f64 / 100.0];
                    let local = [(p[0] - pl.origin[0]) * f[1] - (p[1] - pl.origin[1]) * f[0], p[2] - pl.origin[2]];
                    local[0].abs() <= 31.0 && local[1] <= 51.0
                })
            };
            assert!(!bad.is_empty() && bad.iter().all(near_mouth), "{piece}: {} open edges, e.g. {:?}", bad.len(), &bad[..bad.len().min(4)]);
            let tops = bad.iter().filter(|e| (e.0 .2 - 5000).abs() < 150 && (e.1 .2 - 5000).abs() < 150).count();
            assert!(tops >= mouths, "{piece}: the mouth's top edge(s) should be open: {bad:?}");
        }
    }

    /// A dirt path from the ground up a ramp: cut into the floors (points on its rings), drawn
    /// with the blend material by vertex weight, dirt footsteps where it's mostly dirt, and the
    /// level as watertight as before.
    #[test]
    fn dirt_paths_are_cut_into_the_floor() {
        let mut doc = paths_doc();
        doc.lines.push(crate::doc::Line { name: "dirt".into(), kind: "dirt".into(), nodes: vec![vec![1000.0, -1500.0], vec![1000.0, -300.0], vec![1000.0, 900.0]], width: None, closed: false });
        let lvl = build(&doc, &Theme::kokiri()).unwrap();
        let bad = open_edges(&lvl, &["ground", "walls", "cliffs", "bank", "trees"]);
        assert!(bad.is_empty(), "{} open or non-manifold edges, e.g. {:?}", bad.len(), &bad[..bad.len().min(6)]);
        let g = lvl.mesh.objects.iter().find(|o| o.name == "ground").unwrap();
        assert_eq!(g.blend.len(), g.verts.len());
        let mat = |n: &str| lvl.mesh.materials.iter().position(|m| m == n).unwrap();
        let (blend, dirt) = (mat("ground+dirt"), lvl.mesh.surfaces.iter().position(|s| s == "dirt").unwrap() as i64);
        // the soft edge has vertices: the floor's points lie on both rings (full and none)
        let edge = g.verts.iter().zip(&g.blend).filter(|(v, _)| v[1] < -400.0 && (v[0] - 1000.0).abs() > 30.0 && (v[0] - 1000.0).abs() < 150.0);
        let (full, none) = edge.fold((0, 0), |(f, n), (_, &w)| (f + (w == 1.0) as usize, n + (w == 0.0) as usize));
        assert!(full > 20 && none > 20, "{full} full and {none} bare vertices by the path");
        // every blended triangle touches dirt, every triangle of the floor near the path blends,
        // and the middle of the path collides as dirt
        for t in 0..g.tris.len() {
            let w = g.tris[t].map(|v| g.blend[v]);
            let c = g.tris[t].map(|v| g.verts[v]);
            let m = [(c[0][0] + c[1][0] + c[2][0]) / 3.0, (c[0][1] + c[1][1] + c[2][1]) / 3.0];
            if g.mat[t] == blend {
                assert!(w.iter().any(|&x| x > 0.0));
            }
            if (m[0] - 1000.0).abs() < 50.0 && m[1] < -500.0 && m[1] > -1400.0 {
                assert_eq!((g.mat[t], g.surf[t]), (blend, dirt), "triangle at {m:?}");
            }
            if (m[0] - 1000.0).abs() > 300.0 {
                assert_ne!(g.mat[t], blend, "triangle at {m:?} is far from the path");
            }
        }
        // the export carries the weights and the second texture's name
        let j = lvl.mesh.to_json(&|r| (r.to_string(), crate::textures::TexInfo::plain(r)));
        assert!(j["objects"].as_array().unwrap().iter().any(|o| o["alpha"].as_array().is_some_and(|a| !a.is_empty())));
        assert_eq!(Theme::kokiri().overlay_texture("ground+dirt").unwrap(), "kf_ground+kf_dirt_strip@9b8c34-70-2-16-48");
    }

    #[test]
    fn a_deck_meets_the_floor_it_lands_on_edge_for_edge() {
        // a bridge landing on a plateau at a slant, so its end crosses the plateau's edge obliquely
        let mut doc = paths_doc();
        doc.paths[1].nodes = vec![vec![Some(1000.0), Some(1200.0)], vec![Some(-1000.0), Some(700.0)]];
        let lvl = build(&doc, &Theme::kokiri()).unwrap();
        assert!(lvl.problems.is_empty(), "{:?}", lvl.problems);
        let key = |p: &P3| ((p[0] * 100.0).round() as i64, (p[1] * 100.0).round() as i64, (p[2] * 100.0).round() as i64);
        let edges = |name: &str, top_only: bool| -> HashMap<((i64, i64, i64), (i64, i64, i64)), usize> {
            let o = lvl.mesh.objects.iter().find(|o| o.name == name).unwrap();
            let mut m = HashMap::new();
            for (ti, t) in o.tris.iter().enumerate() {
                if top_only && lvl.mesh.materials[o.mat[ti]] != "ground" {
                    continue;
                }
                for k in 0..3 {
                    let (a, b) = (key(&o.verts[t[k]]), key(&o.verts[t[(k + 1) % 3]]));
                    *m.entry(if a < b { (a, b) } else { (b, a) }).or_default() += 1;
                }
            }
            m
        };
        let (deck, ground) = (edges("bridges", true), edges("ground", false));
        // the deck's open edges at a plateau's height (its landed ends) are all the plateau's edges
        let ends: Vec<_> = deck.iter().filter(|&(ref e, &n)| n == 1 && e.0 .2 == 24000 && e.1 .2 == 24000).map(|(e, _)| *e).collect();
        let at_edge: Vec<_> = ends.iter().filter(|e| {
            let on = |p: &(i64, i64, i64)| (p.0.abs() - 140000).abs() <= 1 || (p.0.abs() - 60000).abs() <= 1 || (p.1 - 60000).abs() <= 1 || (p.1 - 140000).abs() <= 1;
            on(&e.0) && on(&e.1)
        }).collect();
        assert!(!at_edge.is_empty(), "the deck reaches the plateaus' edges");
        for e in at_edge {
            assert!(ground.contains_key(e), "deck edge {e:?} isn't a floor edge");
        }
    }

    #[test]
    fn walls_pick_their_style_from_what_they_are() {
        let lvl = build(&sample_doc(), &Theme::kokiri()).unwrap();
        let mats = &lvl.mesh.materials;
        let walls = lvl.mesh.objects.iter().find(|o| o.name == "walls").unwrap();
        let used: std::collections::BTreeSet<&str> = walls.mat.iter().map(|&m| mats[m].as_str()).collect();
        // 120 and 160 tall cliffs, and the pond's shore
        assert!(used.contains("cliff") && used.contains("cliff_strip_dark"), "{used:?}");
    }
}
