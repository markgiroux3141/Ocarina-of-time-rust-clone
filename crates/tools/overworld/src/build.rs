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
//!   path's surface: an embankment where it's above the region, a cutting where it's below (a
//!   ramp running on into a plateau). Embankment sides take the theme's embankment style; a
//!   cutting's sides are the region's own walls. Floating runs are bridge decks.
//! - Overlays: grass along wall feet (not under water), roots hanging from tall cliffs.
//! - Boundary: a cliff from each floor up to the rim line, a bank back to the tree line (the
//!   outline grown by the bank's depth, simplified to long straight panels), trunks standing on
//!   the bank's edge and foliage over them. The rim rises over high floors near the edge, never
//!   faster than `rise_slope`.
//!
//! Objects: ground, water, walls, cliffs (the boundary's), bank, trees (trunks), foliage,
//! bridges, overlays.

use crate::doc::{Doc, Noise, Section, WallTexture};
use crate::geom::*;
use crate::map::{Map, VOID};
use crate::mesh::Mesh;
use crate::noise::{fbm3, relief, relief3};
use crate::lines::{DirtLine, DirtPaths};
use crate::paths::{self, PathGeo};
use crate::pieces::Kit;
use crate::profiles::{self, Wall};
use crate::props::{self, Placed};
use crate::rocks::{self, Frame, Half, Lumps};
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
    /// The tunnels built: (line index, the floor along its middle).
    pub tunnels: Vec<(usize, Vec<P3>)>,
}

struct Info {
    z: f64,
    water: Option<f64>,
    /// A pit: no ground, a void floor at z.
    pit: bool,
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
    /// How capped walls are textured (`Detail::walls`).
    wall_texture: WallTexture,
    /// Dirt paths painted into the floors.
    dirt: DirtPaths,
    /// Where props level the bumps under them.
    pads: Vec<props::Pad>,
    /// Regions' edge profiles: slopes and terraces (`profiles.rs`).
    field: profiles::Field,
    /// A point well inside each face of a region with terraces, where its tread's height is read.
    anchors: Vec<Option<P2>>,
    /// The bent cliffs' columns (overhangs, ragged rock), by map vertex (`bends`).
    bent: HashMap<usize, Bent>,
    /// The outer edges with no forest beyond them: the bands' far edges (false: nothing at all)
    /// and side edges (true: a face closing the band off). By position: step lines ending on a
    /// band's side split its pieces.
    sky: Vec<(P2, P2, bool)>,
    /// The bands added for what lies beyond the outline (`beyond.rs`).
    bands: crate::beyond::Bands,
    /// Each path's attached runs' footprints, and the pairs of paths blended where they overlap
    /// (`paths::blends`).
    ribbons: Vec<Vec<Vec<P2>>>,
    blends: std::collections::HashSet<(usize, usize)>,
}

/// A bent cliff's column: which way is out (towards the lower floor), how far into the bend it is
/// (0 at a bent run's ends, easing to 1), and how it's bent.
#[derive(Clone, Copy, Debug)]
struct Bent {
    out: P2,
    w: f64,
    wall: Wall,
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
    /// An overhang or ragged rock on its edge (`profiles::Wall`).
    bend: Option<Wall>,
    /// A top-anchored wall's run's tallest height: stretched, its texture is once over that
    /// (`emit_anchored`).
    span: f64,
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
    // what lies beyond the outline's edges: bands, regions outside it (`beyond.rs`)
    let (expanded, bands) = crate::beyond::expand(doc)?;
    let doc: &Doc = &expanded;
    let mut problems = vec![];
    let mut regions = vec![Info { z: doc.outline.z, water: None, pit: false, edge: None, noise: doc.outline.noise.clone() }];
    for (i, r) in doc.regions.iter().enumerate() {
        let water = match r.kind.as_str() {
            "floor" | "pit" => None,
            "water" => Some(r.surface.unwrap_or(r.z + 60.0)),
            k => return Err(format!("region {i} ({}): unknown kind {k:?} (floor, water or pit)", r.name)),
        };
        // a style the theme doesn't have (a level switched to another theme) falls back to the
        // theme's own walls
        let mut edge = r.edge.clone();
        if let Some(e) = &r.edge {
            if !theme.wall_styles.contains_key(e) {
                problems.push(format!("region {i} ({}): theme {} has no wall style {e:?}, so its walls are the theme's own", r.name, theme.name));
                edge = None;
            }
        }
        if let Some(n) = &r.noise {
            if n.scale <= 0.0 {
                return Err(format!("region {i} ({}): noise scale must be positive", r.name));
            }
        }
        regions.push(Info { z: r.z, water, pit: r.kind == "pit", edge, noise: r.noise.clone() });
    }
    // paths lay themselves out on the regions' ground (their profiles included), then their
    // footprints join the map
    let pre = Map::build(doc)?;
    lap("map");
    let max_slope = theme.paths.as_ref().map_or(35.0, |p| p.max_slope);
    let zs: Vec<f64> = regions.iter().map(|r| r.z).collect();
    let field = profiles::Field::new_with(doc, &pre, &zs, max_slope, &mut problems, &bands.ends)?;
    let mut paths: Vec<PathGeo> = {
        let base = |p: P2| field.base(&pre.loop_polys, p);
        let sampling = doc.settings.detail()?.paths;
        paths::layout_all(&doc.paths, &base, sampling).into_iter().collect::<Result<_, _>>()?
    };
    // terraces' step lines cut the regions' floors into treads
    let cuts = field.cuts(&pre, curve_tol(doc.settings.detail()?.curves));
    lap("profiles");
    if !paths.is_empty() {
        let pt = theme.paths.as_ref().ok_or_else(|| format!("theme {} has no paths section", theme.name))?;
        for g in &mut paths {
            if let Some(e) = g.edge.as_ref().filter(|e| !theme.wall_styles.contains_key(*e)) {
                problems.push(format!("path {}: theme {} has no wall style {e:?}, so its sides are the theme's own", g.name, theme.name));
                g.edge = None;
            }
            if !theme.wall_styles.contains_key(&pt.side) {
                return Err(format!("theme {} has no wall style {:?} for paths' sides", theme.name, pt.side));
            }
            problems.extend(g.notes.iter().cloned());
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
    let map = if ribbons.is_empty() && probes.is_empty() && cuts.is_empty() { pre } else { Map::build_with(doc, &ribbons, &probes, &cuts)? };
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
            "hedge" if theme.hedge.is_some() => {}
            "tunnel" if theme.tunnel.is_some() => {}
            "rock" | "arch" if theme.rocks.is_some() => {}
            k => problems.push(format!("line {i} ({}): {k} lines aren't built yet", l.name)),
        }
    }
    // paths that look like dirt: a dirt line along each attached run, as wide as the path less the dirt's soft
    // edge and wander, so the dirt fades out by the path's own edge
    for (k, g) in paths.iter().enumerate().filter(|(_, g)| g.look.as_deref() == Some("dirt")) {
        let Some(th) = theme.dirt.as_ref() else {
            problems.push(format!("path {}: theme {} has no dirt, so it's the ground", g.name, theme.name));
            continue;
        };
        // on its attached runs only: a floating run's deck isn't the ground under it
        for r in g.runs.iter().filter(|r| !r.floating && r.i1 > r.i0) {
            let st = &g.st[r.i0..=r.i1];
            let w = st.iter().map(|s| s.w).fold(f64::INFINITY, f64::min);
            let line = crate::doc::Line {
                name: g.name.clone(),
                kind: "dirt".into(),
                nodes: st.iter().map(|s| vec![s.p[0], s.p[1]]).collect(),
                width: Some((w - th.soft - 2.0 * th.wobble.abs()).max(10.0)),
                ..Default::default()
            };
            let seed = doc.settings.seed.wrapping_mul(0x9E37_79B9) ^ (k as u32).wrapping_mul(104_729);
            dirt.lines.push(DirtLine::new(&line, th, Sampling::Straight { max: 1e9 }, seed).map_err(|e| format!("path {}: {e}", g.name))?);
        }
    }
    let n = map.verts.len();
    let anchors = (0..map.faces.len()).map(|f| field.stepped(map.faces[f].region).then(|| anchor(&map, f))).collect();
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
        wall_texture: doc.settings.detail()?.walls,
        dirt,
        pads: props::pads(&doc.props, kit),
        field,
        anchors,
        bent: HashMap::new(),
        sky: vec![],
        bands,
        ribbons: vec![],
        blends: Default::default(),
    };
    b.ribbons = b.paths.iter().map(|g| g.runs.iter().filter(|r| !r.floating).map(|r| g.ribbon(r)).collect()).collect();
    b.blends = paths::blends(&b.paths);
    // the bands' far edges (nothing beyond them) and side edges (a face closing them off)
    for (j, far) in b.bands.far.iter().enumerate() {
        let l = b.bands.first + 1 + j;
        let lp = &b.map.loops[l];
        for m in 0..lp.len() {
            let e = b.map.loop_edges[l][m];
            let side = b.bands.sides[j].contains(&e);
            if side || far.contains(&e) {
                let (u, v) = (lp[m], lp[(m + 1) % lp.len()]);
                b.sky.push((b.map.verts[u], b.map.verts[v], side));
            }
        }
    }
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
    b.skylines();
    lap("boundary");
    b.bends();
    b.edge_points();
    lap("edge_points");
    b.floors();
    lap("floors");
    b.bridges();
    lap("bridges");
    b.emit_walls();
    b.stair_sides();
    b.undercuts();
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
    // tunnels go through the finished walls, before props stand on the ground
    let mut built_tunnels = vec![];
    if let Some(tt) = theme.tunnel.as_ref().filter(|_| doc.lines.iter().any(|l| l.kind == "tunnel")) {
        let ground = props::Ground::new(&b.mesh);
        let detail = doc.settings.detail()?;
        let fine: u8 = match doc.settings.detail.as_str() {
            "medium" => 1,
            "low" => 2,
            _ => 0,
        };
        let max_slope = theme.paths.as_ref().map_or(35.0, |p| p.max_slope);
        let mut bores = vec![];
        for (i, l) in doc.lines.iter().enumerate().filter(|(_, l)| l.kind == "tunnel") {
            let mut step = doc.settings.sample.max(1.0) * [0.5, 1.0, 2.0][fine as usize];
            if let Some(n) = l.noise.as_ref().filter(|n| n.amplitude != 0.0 && n.scale > 0.0) {
                step = step.min((n.scale / 4.0).max(10.0));
            }
            let seed = doc.settings.seed.wrapping_mul(0x9E37_79B9) ^ (i as u32).wrapping_mul(7919);
            match crate::tunnels::build(l, tt, theme, detail.paths, fine, step, max_slope, seed, &mut b.mesh, &ground) {
                Ok((bore, pr)) => {
                    built_tunnels.push((i, bore.floor.clone()));
                    bores.push(bore);
                    b.problems.extend(pr.into_iter().map(|p| format!("line {i} ({}): {p}", l.name)));
                }
                Err(e) => b.problems.push(format!("line {i} ({}): {e}", l.name)),
            }
        }
        crate::tunnels::finish(&mut b.mesh, &bores, tt);
        lap("tunnels");
    }
    // rocks and arches stand on the finished ground, before props (which can stand on them)
    if let Some(rk) = theme.rocks.as_ref().filter(|_| doc.lines.iter().any(|l| l.kind == "rock" || l.kind == "arch")) {
        let ground = props::Ground::new(&b.mesh);
        let detail = doc.settings.detail()?;
        let k = match doc.settings.detail.as_str() {
            "medium" => 2.0,
            "low" => 3.0,
            _ => 1.0,
        };
        let pieces = [8, 6, 4][k as usize - 1];
        let fine = rocks::Fineness { curves: detail.curves, step: doc.settings.sample.max(1.0) * k, walls: detail.walls, pieces };
        let arch_fine = rocks::Fineness { curves: detail.paths, step: rocks::arch_step(&doc.settings), walls: detail.walls, pieces };
        for (i, l) in doc.lines.iter().enumerate().filter(|(_, l)| l.kind == "rock" || l.kind == "arch") {
            let name = if l.name.is_empty() { format!("{} {i}", l.kind) } else { l.name.clone() };
            let style = match l.style.as_ref() {
                Some(s) if theme.wall_styles.contains_key(s) => s.clone(),
                Some(s) => {
                    b.problems.push(format!("line {i} ({name}): theme {} has no wall style {s:?}, so it's the theme's rock", theme.name));
                    rk.style.clone()
                }
                None => rk.style.clone(),
            };
            let Some(ws) = theme.wall_styles.get(&style) else {
                b.problems.push(format!("line {i} ({name}): theme {} has no wall style {style:?} for rocks", theme.name));
                continue;
            };
            let seed = b.seed() ^ (i as u32).wrapping_mul(7919).wrapping_add(31);
            let made = if l.kind == "rock" {
                rocks::rock(l, rk, ws, &fine, seed, &mut b.mesh, &ground)
            } else {
                rocks::arch(l, rk, ws, &arch_fine, seed, &mut b.mesh, &ground)
            };
            match made {
                Ok(pr) => b.problems.extend(pr.into_iter().map(|p| format!("line {i} ({name}): {p}"))),
                Err(e) => b.problems.push(format!("line {i} ({name}): {e}")),
            }
        }
        lap("rocks");
    }
    // kit pieces, fences and hedges stand on the finished ground, and are lit with it
    let placed = props::place(&doc.props, kit, &mut b.mesh, &mut b.problems);
    lap("props");
    let railed = b.paths.iter().any(|g| g.railings.is_some());
    if railed || doc.lines.iter().any(|l| theme.fences.contains_key(&l.kind) || l.kind == "bridge" || l.kind == "hedge") {
        let ground = props::Ground::new(&b.mesh);
        // paths' railings, where their sides drop away
        for g in &b.paths {
            let Some(kind) = &g.railings else { continue };
            match theme.fences.get(kind) {
                Some(style) => {
                    for l in railings(g, kind, &b.mesh, &ground) {
                        let mut pr = crate::lines::fence(&l, style, &mut b.mesh, &ground);
                        b.problems.append(&mut pr);
                    }
                }
                None => b.problems.push(format!("path {}: theme {} has no fence {kind:?} for its railings", g.name, theme.name)),
            }
        }
        let max_slope = theme.paths.as_ref().map_or(35.0, |p| p.max_slope);
        for l in &doc.lines {
            if let Some(style) = theme.fences.get(&l.kind) {
                let mut pr = crate::lines::fence(l, style, &mut b.mesh, &ground);
                b.problems.append(&mut pr);
            } else if let (true, Some(h)) = (l.kind == "bridge", &theme.hanging) {
                let mut pr = crate::lines::bridge(l, h, max_slope, &mut b.mesh, &ground);
                b.problems.append(&mut pr);
            } else if let (true, Some(h)) = (l.kind == "hedge", &theme.hedge) {
                let mut pr = crate::lines::hedge(l, h, &mut b.mesh, &ground);
                b.problems.append(&mut pr);
            }
        }
        lap("fences, bridges and hedges");
    }
    b.pit_tints();
    if let Some(l) = &theme.light {
        let seed = b.seed();
        b.mesh.shade(l, theme.variation.as_ref().map(|v| (v, seed)));
    }
    lap("lighting");
    let collision_vertices = b.mesh.collision_vertices();
    Ok(Level { faces: b.map.faces.len(), mesh: b.mesh, problems: b.problems, rim: b.rim, props: placed, collision_vertices, tunnels: built_tunnels })
}

/// Railings stand this far in from a path's side.
const RAIL_INSET: f64 = 8.0;

/// A path's railings: fence lines of `kind` along each side, `RAIL_INSET` in, wherever the floor
/// just outside it is `RAIL_DROP` or more below the path's own (read from the built ground, so a
/// deck, a causeway, a ledge's drop and painted terrain are all seen as built). A railing runs
/// unbroken through short gaps and isn't put up shorter than a few repeats.
fn railings(g: &PathGeo, kind: &str, mesh: &Mesh, ground: &props::Ground) -> Vec<crate::doc::Line> {
    let mut out = vec![];
    for left in [true, false] {
        let mut run: Vec<P2> = vec![];
        let flush = |run: &mut Vec<P2>, out: &mut Vec<crate::doc::Line>| {
            let len: f64 = run.windows(2).map(|w| dist(w[0], w[1])).sum();
            if run.len() >= 2 && len >= 3.0 * crate::doc::RAIL_DROP {
                let pts = douglas_peucker(run, 2.0);
                out.push(crate::doc::Line { name: format!("{} railing", g.name), kind: kind.into(), nodes: pts.iter().map(|p| vec![p[0], p[1]]).collect(), ..Default::default() });
            }
            run.clear();
        };
        for i in 0..g.st.len() {
            let (l, r) = g.sides(i);
            let (e, c) = (if left { l } else { r }, g.st[i].p);
            let out_dir = sub(e, c);
            let d = (out_dir[0].hypot(out_dir[1])).max(1e-9);
            let u = [out_dir[0] / d, out_dir[1] / d];
            let inner = [e[0] - u[0] * RAIL_INSET, e[1] - u[1] * RAIL_INSET];
            let beyond = [e[0] + u[0] * 12.0, e[1] + u[1] * 12.0];
            let on = ground.at(mesh, inner).0;
            let below = ground.at(mesh, beyond).0;
            let drops = match (on, below) {
                (Some(z), Some(b)) => z - b >= crate::doc::RAIL_DROP,
                (Some(_), None) => true,
                _ => false,
            };
            if drops {
                run.push(inner);
            } else {
                flush(&mut run, &mut out);
            }
        }
        flush(&mut run, &mut out);
    }
    out
}

/// A boardwalk's deck, top to underside.
const BOARD: f64 = 12.0;

/// The collision role of a pit's floor: Link falling onto it voids out (`oot_import::level`
/// makes it floor property 12, as the game's bottomless pits).
pub const PIT_SURFACE: &str = "void";
/// How dark a pit's floor is drawn, and its walls at the bottom.
const PIT_DARK: f64 = 0.06;

/// A prop's pad (`props::Pad`) is level this far past its base outline, under its walls' feet.
const PAD_MARGIN: f64 = 15.0;
/// The steepest a pad's skirt climbs back to the bumps, in degrees.
const PAD_SLOPE: f64 = 30.0;

/// How wide a pad's skirt is on a floor bumped by n: the bumps reach 2 x amplitude from the pad,
/// and the smoothstep climbs at most 1.5 x its average slope.
fn pad_skirt(n: &Noise) -> f64 {
    (1.5 * 2.0 * n.amplitude.abs() / PAD_SLOPE.to_radians().tan()).max(0.5 * n.edge).max(20.0)
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

    /// The ground's height in face f at p: an attached path's surface where one covers it (the
    /// highest where they overlap), raised over the region's or cut into it; else the region's,
    /// with its edges' profiles.
    fn height(&self, f: usize, p: P2) -> f64 {
        let face = &self.map.faces[f];
        match face.paths.len() {
            0 => {}
            1 => return self.path_z(face.paths[0], p),
            _ => return self.surface(&face.paths, p),
        }
        face.paths.iter().map(|&k| self.paths[k].z_at(p)).reduce(f64::max).unwrap_or_else(|| {
            let z = self.field.z(face.region, p, self.anchors[f].unwrap_or(p));
            // a rough band's (`beyond.rs`): every height there goes through here, so its walls
            // and floors still meet
            match self.bands.rough.get(&face.region) {
                Some(r) => self.rough_z(face.region, r, p, z),
                None => z,
            }
        })
    }

    /// The ground at p where several paths' footprints overlap: those blended with each other
    /// (`paths::blends`, joined or crossing at about one height) are their surfaces weighted by how
    /// far inside each footprint p is, so the ground meets each path's own along its edge; of
    /// those that aren't, the highest.
    fn surface(&self, ks: &[usize], p: P2) -> f64 {
        let mut done = vec![false; ks.len()];
        let mut best = f64::NEG_INFINITY;
        for i in 0..ks.len() {
            if done[i] {
                continue;
            }
            // i's group: everything blended with it, directly or through another
            let mut group = vec![i];
            done[i] = true;
            let mut g = 0;
            while g < group.len() {
                let a = ks[group[g]];
                for j in 0..ks.len() {
                    if !done[j] && self.blends.contains(&(a.min(ks[j]), a.max(ks[j]))) {
                        done[j] = true;
                        group.push(j);
                    }
                }
                g += 1;
            }
            let zs: Vec<f64> = group.iter().map(|&j| self.path_z(ks[j], p)).collect();
            let ws: Vec<f64> = group.iter().map(|&j| self.inside(ks[j], p)).collect();
            let sum: f64 = ws.iter().sum();
            let z = if group.len() == 1 || sum < 1e-9 {
                zs.iter().copied().fold(f64::NEG_INFINITY, f64::max)
            } else {
                zs.iter().zip(&ws).map(|(z, w)| z * w).sum::<f64>() / sum
            };
            best = best.max(z);
        }
        best
    }

    /// Path k's surface at p. Near a path it runs into (`PathGeo::into`), within its own width of
    /// that path's footprint, it eases onto that path's surface, which it is on its edge: so where
    /// a branch meets a sloping path's side, the corners meet too.
    fn path_z(&self, k: usize, p: P2) -> f64 {
        let g = &self.paths[k];
        let z = g.z_at(p);
        let mut out = z;
        for &a in &g.into {
            let near = self.ribbons[a].iter().map(|rb| if point_in_poly(p, rb) { 0.0 } else { dist_to_loop(p, rb) }).fold(f64::INFINITY, f64::min);
            let reach = g.project(p).2.max(1.0);
            if near < reach {
                let t = near / reach;
                let t = t * t * (3.0 - 2.0 * t);
                out = self.paths[a].z_at(p) + (out - self.paths[a].z_at(p)) * t;
            }
        }
        out
    }

    /// How far inside path k's footprint p is (0 on its edge or outside).
    fn inside(&self, k: usize, p: P2) -> f64 {
        self.ribbons[k].iter().filter(|rb| point_in_poly(p, rb)).map(|rb| dist_to_loop(p, rb)).fold(0.0, f64::max)
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

    /// The path cut into the ground beside face `beside` (a floor no path covers) along face f,
    /// if f's ground is a path's: the highest covering f.
    fn cut_path(&self, f: usize, beside: usize, p: P2) -> Option<usize> {
        if !self.map.faces[beside].paths.is_empty() {
            return None;
        }
        self.map.faces[f].paths.iter().map(|&k| (k, self.paths[k].z_at(p))).max_by(|a, b| a.1.total_cmp(&b.1)).map(|(k, _)| k)
    }

    /// Water lies in a pond region's faces that no path covers.
    fn water(&self, f: usize) -> bool {
        let face = &self.map.faces[f];
        face.paths.is_empty() && self.regions[face.region].water.is_some()
    }

    /// A pit's floor: the void Link falls into (no path over it).
    fn pit(&self, f: usize) -> bool {
        let face = &self.map.faces[f];
        face.paths.is_empty() && self.regions[face.region].pit
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

    /// The band edge (far or side) the outer half-edge h lies on, if any: whether it's a side.
    fn sky_at(&self, h: usize) -> Option<bool> {
        if self.sky.is_empty() {
            return None;
        }
        let m = lerp(self.map.verts[self.map.from(h)], self.map.verts[self.map.to(h)], 0.5);
        self.sky.iter().find(|(a, b, _)| dist_to_seg(m, *a, *b).0 < 1e-6).map(|x| x.2)
    }

    /// A rough band's height at p, z before roughness. Its far edge drops a little, so the strip
    /// past the ridge falls away behind it (else, roughened differently, it stands up over the
    /// ridge in places), and along it heights run straight from one of the band's far nodes to the
    /// next: the map's other points on it stay in line, so no sliver of a triangle between three
    /// of them stands up.
    fn rough_z(&self, l: usize, r: &crate::beyond::Rough, p: P2, z: f64) -> f64 {
        if self.sky_at_point(p) == Some(false) {
            if let Some(k) = (0..r.far.len().saturating_sub(1)).find(|&k| dist_to_seg(p, r.far[k], r.far[k + 1]).0 < 1e-6) {
                let (a, b) = (r.far[k], r.far[k + 1]);
                let at = |q: P2| r.at(q, self.field.z(l, q, q)) - crate::beyond::CREST;
                let s = dist_to_seg(p, a, b).1;
                return at(a) + (at(b) - at(a)) * s;
            }
            return r.at(p, z) - crate::beyond::CREST;
        }
        r.at(p, z)
    }

    /// The band edge (far or side) p lies on, if any: whether it's a side.
    fn sky_at_point(&self, p: P2) -> Option<bool> {
        self.sky.iter().find(|(a, b, _)| dist_to_seg(p, *a, *b).0 < 1e-6).map(|x| x.2)
    }

    /// Whether the outer half-edge h has the forest beyond it (not a band's far or side edge).
    fn forest(&self, h: usize) -> bool {
        self.sky_at(h).is_none()
    }

    /// Whether the outer half-edge h is a band's side edge.
    fn side(&self, h: usize) -> bool {
        self.sky_at(h) == Some(true)
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
                    let (here, before) = (hs[i], hs[(i + n - 1) % n]);
                    // only the forest's own edges set its rim (none beside bands' ends and far edges)
                    if !self.forest(here) && !self.forest(before) {
                        return f64::NEG_INFINITY;
                    }
                    let fz = |h: usize| if self.forest(h) { self.hv(self.map.half_face[h ^ 1], v) } else { f64::NEG_INFINITY };
                    let f = fz(here).max(fz(before));
                    let mut near = f;
                    for (r, poly) in self.map.loop_polys.iter().enumerate().skip(1) {
                        if self.bands.is_band(r) {
                            continue;
                        }
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
                if self.forest(hs[i]) || self.forest(hs[(i + n - 1) % n]) {
                    self.levels[self.map.from(hs[i])].push(top[i]);
                }
            }
            out.push((hs, top));
        }
        out
    }

    fn floors(&mut self) {
        let th = self.theme;
        let detail = self.doc.settings.detail().expect("checked by the map");
        let (spacing, rounded, max) = match self.doc.settings.detail.as_str() {
            "medium" => (150.0, 4, 2.0 * self.doc.settings.sample),
            "low" => (250.0, 3, 4.0 * self.doc.settings.sample),
            _ => (80.0, 6, self.doc.settings.sample),
        };
        let slope_rings: Vec<Vec<Vec<P2>>> = (0..self.regions.len()).map(|r| self.field.rings(r, spacing, rounded, curve_tol(detail.curves), max)).collect();
        for f in 0..self.map.faces.len() {
            let face = &self.map.faces[f];
            let outer = self.ring(&self.map.cycles[face.outer]);
            let holes: Vec<Vec<P2>> = face.holes.iter().map(|&c| self.ring(&self.map.cycles[c])).collect();
            let water = if self.water(f) { self.regions[face.region].water } else { None };
            let surf = if water.is_some() { &th.water.bed } else { &th.floor };
            // bumps: not under a path (its surface wins), denser points to show them
            let noise = self.regions[face.region].noise.as_ref().filter(|n| n.amplitude != 0.0 && face.paths.is_empty() && !self.regions[face.region].pit);
            let pit = self.pit(f);
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
            let mut segs: Vec<[P2; 2]> = vec![];
            // slopes: points along lines across them, the crest held by constrained edges
            if face.paths.is_empty() {
                let usable = |p: P2| point_in_poly(p, &outer) && !holes.iter().any(|h| point_in_poly(p, h)) && index.dist_within(p, 4.0) > 3.0;
                for run in &slope_rings[face.region] {
                    let mut prev: Option<P2> = None;
                    for &p in run {
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
            // dirt paths: points on their rings (full, none) with edges between, inside this face
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
            // props level the bumps under them: flat at the bump at the prop's anchor out to
            // PAD_MARGIN past its base, easing back to the bumps over the skirt; the flat part's
            // edge is held by constrained edges, so every triangle under the prop is level, and
            // the floor's own points (as dense as its bumps) take the skirt
            let mut pads: Vec<(&props::Pad, f64, f64)> = vec![];
            if let Some(n) = noise {
                let skirt = pad_skirt(n);
                let reach = PAD_MARGIN + skirt;
                let bb = outer.iter().fold([f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY], |b, q| [b[0].min(q[0]), b[1].min(q[1]), b[2].max(q[0]), b[3].max(q[1])]);
                let inside = |p: P2| point_in_poly(p, &outer) && !holes.iter().any(|h| point_in_poly(p, h));
                let usable = |p: P2| inside(p) && index.dist_within(p, 4.0) > 3.0;
                for pd in &self.pads {
                    if pd.bb[0] - reach > bb[2] || pd.bb[2] + reach < bb[0] || pd.bb[1] - reach > bb[3] || pd.bb[3] + reach < bb[1] {
                        continue;
                    }
                    // a prop standing in another face meets this one at its unbumped height
                    let h = if inside(pd.anchor) { self.bump(n, face.region, pd.anchor, &index) } else { 0.0 };
                    let ring = pd.grown(PAD_MARGIN, steiner.max(40.0));
                    for i in 0..ring.len() {
                        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                        if usable(a) {
                            extra.push(a);
                            if usable(b) {
                                segs.push([a, b]);
                            }
                        }
                    }
                    pads.push((pd, h, skirt));
                }
            }
            // the extra points on its edges (`ring`) are where walls meet it: their height is the
            // edge's, linear between its ends as the walls' feet are, which a path's surface on a
            // curve isn't quite
            let mut on_edges: HashMap<(i64, i64), f64> = HashMap::new();
            for &c in std::iter::once(&face.outer).chain(&face.holes) {
                for &h in &self.map.cycles[c] {
                    let Some(x) = self.extra.get(&h) else { continue };
                    let (za, zb) = (self.hv(f, self.map.from(h)), self.hv(f, self.map.to(h)));
                    for &(t, q) in x {
                        on_edges.insert(((q[0] * 1000.0).round() as i64, (q[1] * 1000.0).round() as i64), za + (zb - za) * t);
                    }
                }
            }
            let tris: Vec<[P3; 3]> = triangulate_full(&outer, &holes, steiner, &extra, &segs)
                .into_iter()
                .map(|t| {
                    t.map(|q| {
                        let key = ((q[0] * 1000.0).round() as i64, (q[1] * 1000.0).round() as i64);
                        let z = *cache.entry(key).or_insert_with(|| {
                            let base = on_edges.get(&key).copied().unwrap_or_else(|| self.height(f, q));
                            match noise {
                                Some(n) => {
                                    let mut bump = self.bump(n, face.region, q, &index);
                                    for &(pd, h, skirt) in &pads {
                                        let t = ((pd.dist(q) - PAD_MARGIN) / skirt).clamp(0.0, 1.0);
                                        bump = h + (bump - h) * t * t * (3.0 - 2.0 * t);
                                    }
                                    let z = base + bump;
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
            let anchor = self.anchors[f].unwrap_or(outer[0]);
            let (fregion, bare) = (face.region, face.paths.is_empty());
            let face_paths = face.paths.clone();
            for p in tris {
                // stairs: the step texture along the path's surface, one step every `step`
                let c = [(p[0][0] + p[1][0] + p[2][0]) / 3.0, (p[0][1] + p[1][1] + p[2][1]) / 3.0];
                let top = face_paths.iter().copied().max_by(|&a, &b| self.paths[a].z_at(c).total_cmp(&self.paths[b].z_at(c)));
                if let Some((k, st)) = top.and_then(|k| Some((k, self.stairs(k)?.clone()))).filter(|_| water.is_none() && !pit) {
                    let g = &self.paths[k];
                    let uv = p.map(|q| g.along([q[0], q[1]]).map_or([0.0, 0.0], |(_, _, s3, left, _, w)| [st.across * (0.5 + left / w.max(1e-9)), s3 / st.step]));
                    self.mesh.tri("ground", p, uv, &st.tread, &st.surface);
                    continue;
                }
                // a stack's slope drawn as a wall: u along its foot, v up the face
                let c = [(p[0][0] + p[1][0] + p[2][0]) / 3.0, (p[0][1] + p[1][1] + p[2][1]) / 3.0];
                let look = (water.is_none() && !pit && bare).then(|| self.field.look(fregion, c, anchor)).flatten();
                if let Some((ws, t, zs)) = look.and_then(|(lk, t, zs)| self.slope_style(&lk).map(|ws| (ws, t, zs))) {
                    // capped styles repeat their middle rows, banded ones their band; the rest are
                    // stretched once over the slope, as a wall of that style is over its height
                    let (mat, v): (String, Box<dyn Fn(f64) -> f64>) = match (&ws.caps, ws.band) {
                        (Some(c), _) if c.rows > 0 => (format!("{}~mid", ws.material), Box::new(move |z| z / c.unit())),
                        (Some(c), _) => (ws.material.clone(), Box::new(move |z| z / c.tile_v)),
                        (None, Some(b)) => (ws.material.clone(), Box::new(move |z| z / b)),
                        (None, None) => (ws.material.clone(), Box::new(move |z| (z - zs[0]) / (zs[1] - zs[0]).max(1e-9))),
                    };
                    let uv = p.map(|q| [(q[0] * t[0] + q[1] * t[1]) / ws.tile_u, v(q[2])]);
                    self.mesh.tri("ground", p, uv, &mat, &ws.surface);
                    continue;
                }
                let uv = p.map(|q| [q[0] / surf.tile, q[1] / surf.tile]);
                if pit {
                    // drawn dark (`pit_tints`), colliding as the void
                    self.mesh.tri("pits", p, uv, &surf.material, PIT_SURFACE);
                } else {
                    self.mesh.tri("ground", p, uv, &surf.material, &surf.surface);
                }
                if let Some(w) = water {
                    let uv = p.map(|q| [q[0] / th.water.tile, q[1] / th.water.tile]);
                    self.mesh.tri("water", p.map(|q| [q[0], q[1], w]), uv, &th.water.material, &th.water.surface);
                }
            }
        }
    }

    /// Path k's stairs, if its look is steps and the theme (or another it borrows from) has them.
    fn stairs(&self, k: usize) -> Option<&'a crate::theme::Steps> {
        (self.paths[k].look.as_deref() == Some("steps")).then(|| self.theme.steps.as_ref()).flatten()
    }

    /// The style of path k's sides if it's stairs: the stairs' profile on stairs of Kakariko's slope,
    /// a plain wall repeating along the rest (`Steps::tiled`).
    fn stair_side(&self, k: usize) -> Option<String> {
        let st = self.stairs(k)?;
        let fits = (self.paths[k].max_slope - 0.5f64.atan().to_degrees()).abs() <= crate::theme::PROFILE_FIT;
        Some(if fits { st.side.clone() } else { st.tiled.clone().unwrap_or_else(|| st.side.clone()) })
    }

    /// Stairs' sides: their texture stretched once over each stair, u from its low end to its high
    /// end, v from its foot to its top, so the stairs' profile drawn in it runs along the slope (as
    /// Kakariko's are).
    fn stair_sides(&mut self) {
        let Some(st) = &self.theme.steps else { return };
        // each stairs path's sides' style, and whether it's the profile (else a tiled wall)
        let mut stairs: Vec<(usize, &WallStyle, bool)> = vec![];
        for k in 0..self.paths.len() {
            let Some(name) = self.stair_side(k).filter(|_| self.paths[k].edge.is_none()) else { continue };
            match self.theme.wall_styles.get(&name) {
                Some(ws) => stairs.push((k, ws, name == st.side)),
                None => self.problems.push(format!("theme {} has no wall style {name:?} for stairs' sides", self.theme.name)),
            }
        }
        if stairs.is_empty() {
            return;
        }
        let mats: Vec<Option<usize>> = stairs.iter().map(|s| self.mesh.materials.iter().position(|m| *m == s.1.material)).collect();
        let paths = &self.paths;
        let Some(o) = self.mesh.objects.iter_mut().find(|o| o.name == "walls") else { return };
        for t in 0..o.tris.len() {
            if !mats.contains(&Some(o.mat[t])) {
                continue;
            }
            let c = o.tris[t].map(|v| o.verts[v]);
            let m = [(c[0][0] + c[1][0] + c[2][0]) / 3.0, (c[0][1] + c[1][1] + c[2][1]) / 3.0];
            // the stair it's beside: the nearest stairs path of this material, its run there
            let Some((j, (run, ..))) = (0..stairs.len())
                .filter(|&j| mats[j] == Some(o.mat[t]))
                .filter_map(|j| Some((j, paths[stairs[j].0].along(m)?)))
                // on its sides (half its width out), not another wall of the same style nearby
                .filter(|(_, x)| x.4 <= 0.5 * x.5 + 2.0)
                .min_by(|a, b| (a.1).4.total_cmp(&(b.1).4))
            else {
                continue;
            };
            let (k, ws, profile) = stairs[j];
            let g = &paths[k];
            let r = &g.runs[run];
            let (a, b) = (&g.st[r.i0], &g.st[r.i1]);
            let (lo, hi) = if a.z <= b.z { (a, b) } else { (b, a) };
            let (len, rise) = ((hi.s - lo.s).abs().max(1e-9), (hi.z - lo.z).max(1e-9));
            let dir = (hi.s - lo.s).signum();
            if profile {
                // the profile: once over the stair, its diagonal along the slope
                o.uvs[t] = c.map(|q| {
                    let s = g.along([q[0], q[1]]).map_or(lo.s, |x| x.1);
                    [((s - lo.s) * dir / len).clamp(0.0, 1.0), (q[2] - lo.z) / rise]
                });
            } else {
                // a tiled wall: its bricks one size all over, along the stair and up from its foot
                // (a wall's own v is per column: stretched on a side whose height runs to nothing)
                let band = ws.band.or(ws.caps.as_ref().map(|c| c.tile_v)).unwrap_or(ws.tile_u);
                o.uvs[t] = c.map(|q| {
                    let s = g.along([q[0], q[1]]).map_or(lo.s, |x| x.1);
                    [(s - lo.s) * dir / ws.tile_u, (q[2] - lo.z) / band]
                });
            }
        }
    }

    /// The wall style a stack's slope is drawn as: its own (if the theme has it), or for a steep
    /// slope with none, the theme's style for a tall wall.
    fn slope_style(&mut self, look: &profiles::Look) -> Option<WallStyle> {
        let th = self.theme;
        let name = match look {
            profiles::Look::Floor => return None,
            profiles::Look::Style(s) if th.wall_styles.contains_key(s) => s.clone(),
            profiles::Look::Style(s) => {
                let msg = format!("theme {} has no wall style {s:?} for a slope, so it's the theme's cliff", th.name);
                if !self.problems.contains(&msg) {
                    self.problems.push(msg);
                }
                th.wall_style(f64::INFINITY, false).to_string()
            }
            profiles::Look::Steep => th.wall_style(f64::INFINITY, false).to_string(),
        };
        th.wall_styles.get(&name).cloned()
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
            for (_, d, ..) in self.anchored_bands(c, j.span, hh[0].max(hh[1])) {
                if (hh[0] - d) * (hh[1] - d) < 0.0 {
                    pts.push((h ^ 1, lerp(pp, qq, (d - hh[0]) / (hh[1] - hh[0]))));
                }
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
        // a ledge's walls are those of the highest region it's cut into
        let ledges: Vec<Option<usize>> = (0..self.paths.len())
            .map(|k| {
                self.paths[k].sections.iter().any(|s| matches!(s, Some(Section::Ledge))).then(|| {
                    self.map.faces.iter().filter(|f| f.paths.contains(&k)).map(|f| f.region).max_by(|&a, &b| self.regions[a].z.total_cmp(&self.regions[b].z)).unwrap_or(0)
                })
            })
            .collect();
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
            // an embankment's sides (the higher ground a path's) and a cutting's (the lower) are
            // the path's: a cutting's sides shrink to nothing, where the region's rules would
            // change style partway along
            // a path the higher side (an embankment's side) or the lower (a cutting's); stairs' cuttings
            // are the stairs' `cutting` wall, not their sides' profile
            let top = self.top_path(fl, mp);
            let on = top.map(|k| (k, true)).or_else(|| self.cut_path(fr, fl, mp).map(|k| (k, false)));
            let ledge = on.and_then(|(k, _)| ledges[k].filter(|_| matches!(self.paths[k].section_near(mp), Some(Section::Ledge))));
            let name = match on.filter(|_| ledge.is_none()) {
                Some((k, embankment)) => self.paths[k]
                    .edge
                    .clone()
                    .or_else(|| if embankment { self.stair_side(k) } else { self.stairs(k).and_then(|st| st.cutting.clone()) })
                    .unwrap_or_else(|| th.paths.as_ref().map_or("cliff".into(), |t| t.side.clone())),
                None => {
                    let (rl, rr) = (self.map.faces[fl].region, self.map.faces[fr].region);
                    // a ledge: the region it's cut into, above it and below
                    let (rl, rr) = ledge.map_or((rl, rr), |r| (r, r));
                    let (a, b) = (self.map.verts[p], self.map.verts[q]);
                    let tol = 2.0 * curve_tol(self.doc.settings.detail().expect("checked by the map").curves) + 2.0;
                    let stacked = self.field.wall_style(rl, a, b, tol).or_else(|| self.field.wall_style(rr, a, b, tol)).filter(|s| th.wall_styles.contains_key(s));
                    let own = self.regions[rl].edge.clone();
                    let hgt = 0.5 * ((pr.3[0] - pr.2[0]) + (pr.3[1] - pr.2[1]));
                    stacked.or(own).unwrap_or_else(|| th.wall_style(hgt, self.water(fr)).to_string())
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
                let (mut lsum, mut hsum) = (0.0, 0.0);
                for &i in &run {
                    let (_, _, bot, top, t0, t1) = part[hs[i]].unwrap();
                    let l = self.map.len(hs[i]) * (t1 - t0);
                    lsum += l;
                    hsum += l * 0.5 * ((top[0] - bot[0]) + (top[1] - bot[1]));
                }
                // top-anchored walls (an embankment's sides) stretched are sized by the run's tallest
                // height, as a wall that tall is, so they match the cliffs beside them
                let span = run.iter().map(|&i| part[hs[i]].unwrap()).map(|(_, _, bot, top, _, _)| (top[0] - bot[0]).max(top[1] - bot[1])).fold(0.0, f64::max);
                let anchored = ws.caps.as_ref().is_some_and(|c| c.anchor == "top") && self.wall_texture != WallTexture::Tiled;
                let (tile, reps) = self.wall_tiling(ws, if anchored { span } else { hsum / lsum.max(1e-9) }, same.then_some(total));
                for &i in &run {
                    let h = hs[i];
                    let (p, q, bot, top, t0, t1) = part[h].unwrap();
                    let over_water = self.water(self.map.half_face[h ^ 1]);
                    let l = self.ulen(h);
                    self.wall("walls", Some(h), p, q, bot, top, ws, [u_at[i] + t0 * l, u_at[i] + t1 * l], tile, reps, over_water, span);
                }
            }
        }
    }

    /// A run of wall's u tile (units per repeat along it) and middle repeats, for its mean height
    /// `h`; `around`: the run is a whole loop this long (in u), so u is snapped to whole repeats.
    /// Tiled, the caps keep their size and the middle repeats (`Caps::repeats`, folded to fit).
    /// With the middle stretched, the caps keep their size and the middle is one band over the
    /// rest. Stretched (`Settings::wall_texture`), a capped wall shows its texture once over its
    /// height, and the texture grows across with it, keeping its shape: Kokiri's own way, blurrier
    /// on tall walls, and walls of different heights don't meet texel for texel.
    fn wall_tiling(&self, ws: &WallStyle, h: f64, around: Option<f64>) -> (f64, usize) {
        let along = match (&ws.caps, self.wall_texture) {
            (Some(c), WallTexture::Stretched) => ws.tile_u * (h / c.tile_v).max(0.25),
            _ => ws.tile_u,
        };
        let tile = around.map_or(along, |t| snap(t, along));
        let reps = match (&ws.caps, self.wall_texture) {
            (Some(c), WallTexture::Tiled) => c.repeats(h, self.fold()),
            (Some(c), WallTexture::StretchedMiddle) => usize::from(h > c.tile_v),
            _ => 0,
        };
        (tile, reps)
    }

    /// Whether a capped wall's middle folds to keep its size (`bands`): tiled, at high detail.
    /// The fold's two bands cost about a fifth more triangles, so three-band walls (medium and
    /// low) stretch their repeats a little instead (at most about 1.7 x).
    fn fold(&self) -> bool {
        self.wall_texture == WallTexture::Tiled && !self.walls3
    }

    /// A wall column's texture bands from b up to t: (z0, z1, v0, v1, middle texture). With caps
    /// and `reps` middle repeats: the bottom cap, the middle `reps` times, the top cap, the caps
    /// at their own size (squeezed on walls too short for them). With `fold` (and a mirrored
    /// middle), the whole repeats keep their size and what's left over is folded at the middle's
    /// foot: a part-repeat up into it and back down (two bands, empty when nothing's left over,
    /// so every column of a wall has the same bands). Otherwise the repeats stretch to fill.
    /// Without caps, one band, the texture stretched over it (repeated per `band` units on
    /// walls taller than that).
    fn bands(ws: &WallStyle, reps: usize, b: f64, t: f64, walls3: bool, fold: bool) -> Vec<(f64, f64, f64, f64, bool)> {
        let h = t - b;
        match &ws.caps {
            Some(c) if reps > 0 => {
                let unit = c.unit();
                let (hb, ht, half) = (c.bottom * c.tile_v, c.top * c.tile_v, 0.5 * unit * reps as f64);
                let (hb, ht, mid) = if h >= hb + ht + half {
                    (hb, ht, h - hb - ht)
                } else {
                    let s = h / (hb + ht + half);
                    (hb * s, ht * s, half * s)
                };
                let fold = fold && c.mirror;
                let p = if fold { ((mid - reps as f64 * unit) / 2.0).clamp(0.0, unit * 0.98) } else { 0.0 };
                let m = (mid - 2.0 * p) / reps as f64;
                let derived = walls3 && c.rows > 0;
                let (m0, m1) = c.middle();
                let mut out = vec![(b, b + hb, 0.0, c.bottom, false)];
                let z = b + hb;
                if fold {
                    // up into the middle a share f of a repeat, and back down to its foot
                    let f = p / unit;
                    let (v0, vk) = if derived { (0.0, f) } else { (m0, m0 + (m1 - m0) * f) };
                    out.push((z, z + p, v0, vk, derived));
                    out.push((z + p, z + 2.0 * p, vk, v0, derived));
                }
                let z = z + 2.0 * p;
                if derived {
                    // one band of the middle's own texture, v counting repeats (mirrored by its wrap)
                    out.push((z, t - ht, 0.0, reps as f64, true));
                } else {
                    for k in 0..reps {
                        let (v0, v1) = if c.mirror && k % 2 == 1 { (m1, m0) } else { (m0, m1) };
                        out.push((z + k as f64 * m, z + (k + 1) as f64 * m, v0, v1, false));
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
    fn wall(&mut self, obj: &'static str, half: Option<usize>, p: usize, q: usize, bot: [f64; 2], top: [f64; 2], ws: &'a WallStyle, u: [f64; 2], tile: f64, reps: usize, over_water: bool, span: f64) {
        if (top[0] - bot[0]).max(top[1] - bot[1]) >= 0.5 {
            // a region's own cliffs (not an embankment's sides, nor the edge of the world's) may be bent
            let whole = half.is_some_and(|h| self.map.from(h) == p && self.map.to(h) == q);
            let bend = (obj == "walls" && whole && ws.caps.as_ref().is_none_or(|c| c.anchor != "top")).then(|| self.field.wall(self.map.verts[p], self.map.verts[q])).flatten();
            let bend = match bend {
                Some(Wall::Overhang { .. }) if over_water => {
                    let m = lerp(self.map.verts[p], self.map.verts[q], 0.5);
                    let msg = format!("an overhang over water near ({:.0}, {:.0}) isn't built yet: a cliff instead", m[0], m[1]);
                    if !self.problems.iter().any(|x| x.starts_with("an overhang over water")) {
                        self.problems.push(msg);
                    }
                    None
                }
                b => b,
            };
            self.jobs.push(WallJob { obj, half, p, q, bot, top, ws, u, tile, reps, over_water, bend, span });
        }
    }

    /// Pits are dark: their floors, and their walls fading down to it from the floor round them.
    fn pit_tints(&mut self) {
        // each pit face's floor and the highest floor round it
        let mut pits: Vec<(Vec<P2>, f64, f64)> = vec![];
        for f in 0..self.map.faces.len() {
            if !self.pit(f) {
                continue;
            }
            let face = &self.map.faces[f];
            let mut rim = f64::NEG_INFINITY;
            for &c in std::iter::once(&face.outer).chain(face.holes.iter()) {
                for &h in &self.map.cycles[c] {
                    let g = self.map.half_face[h ^ 1];
                    if g != VOID {
                        rim = rim.max(self.hv(g, self.map.from(h)));
                    }
                }
            }
            let bottom = self.regions[face.region].z;
            if rim > bottom {
                pits.push((self.map.cycle_pts(face.outer), bottom, rim));
            }
        }
        if pits.is_empty() {
            return;
        }
        for o in self.mesh.objects.iter_mut() {
            match o.name.as_str() {
                "pits" => o.tints = vec![[PIT_DARK; 3]; o.verts.len()],
                "walls" => {
                    o.tints = o
                        .verts
                        .iter()
                        .map(|v| {
                            let p = [v[0], v[1]];
                            let k = pits
                                .iter()
                                .filter(|(poly, b, r)| v[2] < *r && v[2] >= b - 1.0 && (point_in_poly(p, poly) || dist_to_loop(p, poly) < 1.0))
                                .map(|(_, b, r)| {
                                    let t = ((v[2] - b) / (r - b)).clamp(0.0, 1.0);
                                    PIT_DARK + (1.0 - PIT_DARK) * t * t * (3.0 - 2.0 * t)
                                })
                                .fold(1.0, f64::min);
                            [k; 3]
                        })
                        .collect();
                }
                _ => {}
            }
        }
    }

    /// The bent cliffs' columns (`Bent`): out is the mean of the bent walls' outward normals there;
    /// the bend eases in over `taper` from each end of a run of bent walls (where a bent run meets
    /// a straight wall, or another kind of bend), so it meets the walls beyond exactly. Bent
    /// columns get extra levels, so the bend shows.
    fn bends(&mut self) {
        let bent: Vec<usize> = (0..self.jobs.len()).filter(|&j| self.jobs[j].bend.is_some()).collect();
        if bent.is_empty() {
            return;
        }
        let mut at: HashMap<usize, Vec<usize>> = HashMap::new();
        for &j in &bent {
            at.entry(self.jobs[j].p).or_default().push(j);
            at.entry(self.jobs[j].q).or_default().push(j);
        }
        let out_of = |s: &Self, j: usize| {
            let (a, b) = (s.map.verts[s.jobs[j].p], s.map.verts[s.jobs[j].q]);
            let d = sub(b, a);
            let l = d[0].hypot(d[1]).max(1e-9);
            [d[1] / l, -d[0] / l]
        };
        // ends: a column one bent wall reaches, or where two kinds meet
        let mut dist_to_end: HashMap<usize, f64> = HashMap::new();
        let mut queue: Vec<usize> = vec![];
        for (&v, js) in &at {
            let kinds_differ = js.iter().any(|&j| self.jobs[j].bend != self.jobs[js[0]].bend);
            if js.len() < 2 || kinds_differ {
                dist_to_end.insert(v, 0.0);
                queue.push(v);
            }
        }
        // distances along the runs from their ends (runs are chains: a few passes settle them)
        for _ in 0..at.len() {
            let mut changed = false;
            for &j in &bent {
                let (p, q) = (self.jobs[j].p, self.jobs[j].q);
                let l = dist(self.map.verts[p], self.map.verts[q]);
                for (a, b) in [(p, q), (q, p)] {
                    if let Some(&da) = dist_to_end.get(&a) {
                        if dist_to_end.get(&b).is_none_or(|&db| da + l < db - 1e-9) {
                            dist_to_end.insert(b, da + l);
                            changed = true;
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }
        for (&v, js) in &at {
            let mut o = [0.0, 0.0];
            for &j in js {
                let n = out_of(self, j);
                o = [o[0] + n[0], o[1] + n[1]];
            }
            let l = o[0].hypot(o[1]);
            if l < 1e-6 {
                continue;
            }
            let wall = self.jobs[js[0]].bend.unwrap();
            let taper = match wall {
                Wall::Overhang { depth } => (2.0 * depth).max(40.0),
                Wall::Ragged { scale, .. } => (0.5 * scale).max(20.0),
            };
            let d = dist_to_end.get(&v).copied().unwrap_or(f64::INFINITY);
            let t = (d / taper).min(1.0);
            self.bent.insert(v, Bent { out: [o[0] / l, o[1] / l], w: t * t * (3.0 - 2.0 * t), wall });
            // rows to show the bend: through an overhang's lip, every so often up ragged rock
            for &j in js {
                let (b, t) = if self.jobs[j].p == v { (self.jobs[j].bot[0], self.jobs[j].top[0]) } else { (self.jobs[j].bot[1], self.jobs[j].top[1]) };
                let h = t - b;
                if h < 2.0 {
                    continue;
                }
                match wall {
                    Wall::Overhang { depth } => {
                        let lip = Self::lip(h, depth);
                        for k in 1..=6 {
                            self.levels[v].push(t - lip * k as f64 / 6.0);
                        }
                    }
                    Wall::Ragged { scale, .. } => {
                        let n = (h / (scale / 5.0).max(12.0)).ceil().max(2.0) as usize;
                        for k in 1..n {
                            self.levels[v].push(b + h * k as f64 / n as f64);
                        }
                    }
                }
            }
        }
    }

    /// An overhang's lip: the top part of a cliff `h` tall that curves back out to its edge.
    fn lip(h: f64, depth: f64) -> f64 {
        (0.45 * h).min(1.5 * depth).max(1.0)
    }

    /// A wall column's point at height z (its foot b, its top t): on the map vertex, or bent.
    fn column(&self, v: usize, z: f64, b: f64, t: f64) -> P3 {
        let p = self.map.verts[v];
        let Some(bt) = self.bent.get(&v).filter(|bt| bt.w > 0.0) else { return [p[0], p[1], z] };
        let h = t - b;
        if h < 1.0 {
            return [p[0], p[1], z];
        }
        let s = ((z - b) / h).clamp(0.0, 1.0);
        let off = match bt.wall {
            Wall::Overhang { depth } => {
                // in under the higher side (against out), vertical up to the lip, then curving out
                let lip = Self::lip(h, depth);
                let s0 = 1.0 - lip / h;
                let g = if s <= s0 { 1.0 } else { (std::f64::consts::FRAC_PI_2 * (s - s0) / (1.0 - s0)).cos() };
                -depth.min(h) * bt.w * g
            }
            Wall::Ragged { amplitude, scale, seed } => {
                let e = |x: f64| {
                    let x = (x / 0.2).clamp(0.0, 1.0);
                    x * x * (3.0 - 2.0 * x)
                };
                amplitude * bt.w * e(s) * e(1.0 - s) * relief3([p[0] / scale, p[1] / scale, z / scale], seed)
            }
        };
        [p[0] + bt.out[0] * off, p[1] + bt.out[1] * off, z]
    }

    /// Under each overhang the lower floor runs in to the wall's foot: a strip from the floor's
    /// edge (with the points it has there) to the foot of the bent wall.
    fn undercuts(&mut self) {
        let th = self.theme;
        let jobs: Vec<(usize, usize, usize, [f64; 2], [f64; 2])> = self
            .jobs
            .iter()
            .filter(|j| matches!(j.bend, Some(Wall::Overhang { .. })))
            .filter_map(|j| Some((j.half?, j.p, j.q, j.bot, j.top)))
            .collect();
        let key = |p: P2| ((p[0] * 1000.0).round() as i64, (p[1] * 1000.0).round() as i64);
        for (h, p, q, bot, top) in jobs {
            let fr = self.map.half_face[h ^ 1];
            if fr == VOID {
                continue;
            }
            let (pp, qq) = (self.map.verts[p], self.map.verts[q]);
            // the floor's edge from p to q: its points on h's twin, in order from p
            let mut edge = vec![pp];
            if let Some(x) = self.extra.get(&(h ^ 1)) {
                let mut x = x.clone();
                x.sort_by(|a, b| b.0.total_cmp(&a.0));
                edge.extend(x.into_iter().filter(|e| e.0 > 1e-9 && e.0 < 1.0 - 1e-9).map(|e| e.1));
            }
            edge.push(qq);
            let (fp, fq) = (self.column(p, bot[0], bot[0], top[0]), self.column(q, bot[1], bot[1], top[1]));
            let mut zs: HashMap<(i64, i64), f64> = HashMap::new();
            for &e in &edge {
                zs.insert(key(e), self.height(fr, e));
            }
            let mut poly = edge.clone();
            for f in [fq, fp] {
                let f2 = [f[0], f[1]];
                if !zs.contains_key(&key(f2)) {
                    zs.insert(key(f2), f[2]);
                    poly.push(f2);
                }
            }
            if poly.len() < 3 || signed_area(&poly).abs() < 1e-3 {
                continue;
            }
            let poly = if signed_area(&poly) < 0.0 { poly.into_iter().rev().collect::<Vec<_>>() } else { poly };
            let surf = &th.floor;
            for t in triangulate(&poly, &[], 0.0) {
                let Some(pts) = t.iter().map(|q| zs.get(&key(*q)).map(|&z| [q[0], q[1], z])).collect::<Option<Vec<P3>>>() else { continue };
                let uv = t.map(|q| [q[0] / surf.tile, q[1] / surf.tile]);
                self.tri_facing("ground", [pts[0], pts[1], pts[2]], uv, &surf.material, &surf.surface, [0.0, 0.0, 1.0]);
            }
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
                    for (z0, z1, ..) in Self::bands(j.ws, j.reps, b, t, self.walls3, self.fold()) {
                        self.levels[v].push(z0);
                        self.levels[v].push(z1);
                    }
                }
            }
        }
        for l in self.levels.iter_mut() {
            l.sort_by(f64::total_cmp);
            l.dedup_by(|a, b| (*a - *b).abs() < 0.01);
        }
        for j in &jobs {
            self.emit_wall(j.obj, j.p, j.q, j.bot, j.top, j.ws, j.u, j.tile, j.reps, j.over_water, j.span);
        }
        // kept for the floors under overhangs (`undercuts`)
        self.jobs = jobs;
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_wall(&mut self, obj: &str, p: usize, q: usize, bot: [f64; 2], top: [f64; 2], ws: &WallStyle, u: [f64; 2], tile: f64, reps: usize, over_water: bool, span: f64) {
        if ws.caps.as_ref().is_some_and(|c| c.anchor == "top") {
            self.emit_anchored(obj, p, q, bot, top, ws, u, tile, span);
            self.overlays(p, q, bot, top, ws, u, over_water);
            return;
        }
        let fold = self.fold();
        let (bp, bq) = (Self::bands(ws, reps, bot[0], top[0], self.walls3, fold), Self::bands(ws, reps, bot[1], top[1], self.walls3, fold));
        let mid = format!("{}~mid", ws.material);
        let (pp, qq) = (self.map.verts[p], self.map.verts[q]);
        let (up, uq) = (u[0] / tile, u[1] / tile);
        let col = |s: &Self, v: usize, z0: f64, z1: f64| -> Vec<f64> {
            let mut c = vec![z0];
            c.extend(s.levels[v].iter().copied().filter(|&z| z > z0 + 0.01 && z < z1 - 0.01));
            if z1 - z0 > 1e-6 {
                c.push(z1);
            }
            c
        };
        let f = |z: f64, a: f64, b: f64| if b - a > 1e-6 { (z - a) / (b - a) } else { 0.0 };
        let _ = (pp, qq);
        for (&(p0, p1, pv0, pv1, middle), &(q0, q1, qv0, qv1, _)) in bp.iter().zip(bq.iter()) {
            let mat = if middle { mid.as_str() } else { ws.material.as_str() };
            let (cp, cq) = (col(self, p, p0, p1), col(self, q, q0, q1));
            let tp: Vec<f64> = cp.iter().map(|&z| f(z, p0, p1)).collect();
            let tq: Vec<f64> = cq.iter().map(|&z| f(z, q0, q1)).collect();
            // the columns' points, bent where the cliff is (an overhang, ragged rock)
            let xp: Vec<P3> = cp.iter().map(|&z| self.column(p, z, bot[0], top[0])).collect();
            let xq: Vec<P3> = cq.iter().map(|&z| self.column(q, z, bot[1], top[1])).collect();
            let vp = |t: f64| pv0 + (pv1 - pv0) * t;
            let vq = |t: f64| qv0 + (qv1 - qv0) * t;
            let (mut i, mut j) = (0, 0);
            while i + 1 < cp.len() || j + 1 < cq.len() {
                if j + 1 >= cq.len() || (i + 1 < cp.len() && tp[i + 1] <= tq[j + 1]) {
                    self.mesh.tri(obj, [xp[i], xq[j], xp[i + 1]], [[up, vp(tp[i])], [uq, vq(tq[j])], [up, vp(tp[i + 1])]], mat, &ws.surface);
                    i += 1;
                } else {
                    self.mesh.tri(obj, [xp[i], xq[j], xq[j + 1]], [[up, vp(tp[i])], [uq, vq(tq[j])], [uq, vq(tq[j + 1])]], mat, &ws.surface);
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
    #[allow(clippy::too_many_arguments)]
    fn emit_anchored(&mut self, obj: &str, p: usize, q: usize, bot: [f64; 2], top: [f64; 2], ws: &WallStyle, u: [f64; 2], tile: f64, span: f64) {
        let c = ws.caps.as_ref().unwrap();
        let (pp, qq) = (self.map.verts[p], self.map.verts[q]);
        let len = dist(pp, qq).max(1e-6);
        let hh = [top[0] - bot[0], top[1] - bot[1]];
        let hmax = hh[0].max(hh[1]);
        let bands = self.anchored_bands(c, span, hmax);
        // what meets each end column, as depths below the top
        let lv: [Vec<f64>; 2] = [(p, 0), (q, 1)].map(|(v, e)| {
            self.levels[v].iter().filter(|&&z| z > bot[e] + 0.01 && z < top[e] - 0.01).map(|&z| top[e] - z).collect()
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
                        let mut ds: Vec<f64> = lv[e].iter().copied().filter(|&d| d > a.1.min(b.1) + 0.005 && d < a.1.max(b.1) - 0.005).collect();
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

    /// A top-anchored wall's texture bands, as (depth from, depth to below its top, v there, v
    /// there), down past `hmax`. Tiled: the top cap at its size and the middle repeating down.
    /// Stretched: the texture once over `span` (its run's tallest height), as a wall that tall
    /// shows it; stretched middle: the caps at their size, the middle once between them over it.
    fn anchored_bands(&self, c: &crate::theme::Caps, span: f64, hmax: f64) -> Vec<(f64, f64, f64, f64)> {
        let (ht, unit) = (c.top * c.tile_v, c.unit());
        let (m0, m1) = c.middle();
        let span = span.max(hmax);
        let hb = c.bottom * c.tile_v;
        let mut bands = match self.wall_texture {
            WallTexture::Stretched => vec![(0.0, span, 1.0, 0.0)],
            WallTexture::StretchedMiddle if span > ht + hb => vec![(0.0, ht, 1.0, 1.0 - c.top), (ht, span - hb, 1.0 - c.top, c.bottom), (span - hb, span, c.bottom, 0.0)],
            WallTexture::StretchedMiddle => vec![(0.0, span, 1.0, 0.0)],
            WallTexture::Tiled => vec![(0.0, ht, 1.0, 1.0 - c.top)],
        };
        let (mut d, mut k) = (bands.last().map_or(ht, |b| b.1), 0);
        while d < hmax {
            let (va, vb) = if c.mirror && k % 2 == 1 { (m0, m1) } else { (m1, m0) };
            bands.push((d, d + unit, va, vb));
            d += unit;
            k += 1;
        }
        bands
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
                // a boardwalk's deck is the hanging bridges' planks, one every `plank` along, laid across
                let hang = self.theme.hanging.as_ref();
                let planks = |i: usize| hang.filter(|_| matches!(g.section(i), Some(Section::Boardwalk { .. })));
                let board_uv = |h: &crate::theme::Hanging, q: P2| {
                    let (s, left, w) = g.project(q);
                    [h.across * (0.5 - left / w.max(1e-9)), s / h.plank]
                };
                for i in t0..t1 {
                    let ((l0, r0), (l1, r1)) = (g.sides(i), g.sides(i + 1));
                    let (z0, z1) = (g.st[i].z, g.st[i + 1].z);
                    let quad = [p3(r0, z0), p3(r1, z1), p3(l1, z1), p3(l0, z0)];
                    match planks(i) {
                        Some(h) => self.mesh.quad("bridges", quad, [r0, r1, l1, l0].map(|q| board_uv(h, q)), &h.deck, &h.surface),
                        None => self.mesh.quad("bridges", quad, [wuv(r0, tt), wuv(r1, tt), wuv(l1, tt), wuv(l0, tt)], &br.top.material, &br.top.surface),
                    }
                }
                for (j, poly) in e0.iter().chain(e1.iter()) {
                    let flat: Vec<P2> = poly.iter().map(|p| [p[0], p[1]]).collect();
                    let zs: HashMap<(i64, i64), f64> = poly.iter().map(|p| (((p[0] * 1000.0).round() as i64, (p[1] * 1000.0).round() as i64), p[2])).collect();
                    let walk = planks(*j);
                    for t in triangulate(&flat, &[], 0.0) {
                        let q = t.map(|q| [q[0], q[1], zs.get(&((q[0] * 1000.0).round() as i64, (q[1] * 1000.0).round() as i64)).copied().unwrap_or_else(|| g.z_at(q))]);
                        match walk {
                            Some(h) => self.mesh.tri("bridges", q, t.map(|q| board_uv(h, q)), &h.deck, &h.surface),
                            None => self.mesh.tri("bridges", q, t.map(|q| wuv(q, tt)), &br.top.material, &br.top.surface),
                        }
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
                // a run that's all boardwalk: a thin deck on posts
                if let Some(h) = hang.filter(|_| (r.i0..r.i1).all(|i| planks(i).is_some())) {
                    let landed = [e0.as_ref().map(|e| e.0), e1.as_ref().map(|e| e.0)];
                    self.boardwalk_body(&g, r.i0, r.i1, landed, (r.cap0, r.cap1), h);
                    continue;
                }
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

    /// A boardwalk's body, stations b0..=b1 of a floating run: the deck `BOARD` thick (the planks'
    /// ends round its edges, the planks again beneath), and a pair of posts every `spacing` from its
    /// underside down into the ground, wherever it stands clear of it. `landed`: the stations where
    /// it lands on a floor (its deck ends there, on the floor's edge), so the posts start past them.
    #[allow(clippy::too_many_arguments)]
    fn boardwalk_body(&mut self, g: &PathGeo, b0: usize, b1: usize, landed: [Option<usize>; 2], caps: (bool, bool), h: &crate::theme::Hanging) {
        let p3 = |p: P2, z: f64| [p[0], p[1], z];
        let th = BOARD;
        let (u0, u1) = (landed[0].unwrap_or(b0), landed[1].unwrap_or(b1));
        let v = |i: usize| g.st[i].s / h.plank;
        for i in u0..u1 {
            let ((l0, r0), (l1, r1)) = (g.sides(i), g.sides(i + 1));
            let (z0, z1) = (g.st[i].z, g.st[i + 1].z);
            let (v0, v1) = (v(i), v(i + 1));
            self.mesh.quad("bridges", [p3(l0, z0 - th), p3(l1, z1 - th), p3(r1, z1 - th), p3(r0, z0 - th)],
                           [[h.across, v0], [h.across, v1], [0.0, v1], [0.0, v0]], &h.under, "");
            self.mesh.quad("bridges", [p3(r0, z0 - th), p3(r1, z1 - th), p3(r1, z1), p3(r0, z0)], [[0.0, v0], [0.0, v1], [0.1, v1], [0.1, v0]], &h.under, "");
            self.mesh.quad("bridges", [p3(l1, z1 - th), p3(l0, z0 - th), p3(l0, z0), p3(l1, z1)], [[0.0, v1], [0.0, v0], [0.1, v0], [0.1, v1]], &h.under, "");
        }
        for (cap, i, back) in [(caps.0 && landed[0].is_none(), b0, true), (caps.1 && landed[1].is_none(), b1, false)] {
            if cap {
                let (l, rr) = g.sides(i);
                let z = g.st[i].z;
                let (a, b) = if back { (l, rr) } else { (rr, l) };
                self.mesh.quad("bridges", [p3(a, z - th), p3(b, z - th), p3(b, z), p3(a, z)], [[0.0, 0.0], [h.across, 0.0], [h.across, 0.1], [0.0, 0.1]], &h.under, "");
            }
        }
        // the posts: `spacing` apart, from the deck's underside into the ground, square, inset
        let spacing = match g.section((b0 + b1) / 2) {
            Some(Section::Boardwalk { spacing, .. }) => spacing.max(20.0),
            _ => 200.0,
        };
        let (s0, s1) = (g.st[u0].s, g.st[u1].s);
        let n = ((s1 - s0) / spacing).round().max(1.0) as usize;
        let pw = 2.0 * h.post_size[0];
        for j in 0..=n {
            let s = s0 + (s1 - s0) * j as f64 / n as f64;
            let i = (u0..u1).find(|&i| g.st[i + 1].s >= s).unwrap_or(u1.saturating_sub(1).max(u0));
            let (a, b) = (&g.st[i], &g.st[(i + 1).min(g.st.len() - 1)]);
            let t = if b.s - a.s > 1e-9 { ((s - a.s) / (b.s - a.s)).clamp(0.0, 1.0) } else { 0.0 };
            let (c, z, w, dir) = (lerp(a.p, b.p, t), a.z + (b.z - a.z) * t - th, a.w + (b.w - a.w) * t, a.dir);
            let across = [-dir[1], dir[0]];
            for k in [1.0, -1.0] {
                let o = [c[0] + across[0] * (0.5 * w - pw) * k, c[1] + across[1] * (0.5 * w - pw) * k];
                let Some(f) = self.face_at(o) else { continue };
                let foot = self.height(f, o);
                if foot > z - 10.0 {
                    continue; // on the ground already, or nearly
                }
                let corner = |sx: f64, sy: f64| [o[0] + (dir[0] * sx + across[0] * sy) * pw / 2.0, o[1] + (dir[1] * sx + across[1] * sy) * pw / 2.0];
                let ring = [corner(-1.0, -1.0), corner(1.0, -1.0), corner(1.0, 1.0), corner(-1.0, 1.0)];
                let z0 = foot - 20.0;
                for m in 0..4 {
                    let (p, q) = (ring[m], ring[(m + 1) % 4]);
                    let face = [p3(p, z0), p3(q, z0), p3(q, z), p3(p, z)];
                    self.mesh.quad("bridges", face, [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]], &h.post, "");
                    self.mesh.col_tri("bridges_collision", [face[0], face[1], face[2]], &h.side_surface);
                    self.mesh.col_tri("bridges_collision", [face[0], face[2], face[3]], &h.side_surface);
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

    /// A rock arch under a deck, stations b0..=b1 (`rocks::sweep`). Each station has a half
    /// cross-section per side: down `lip` from the deck's edge, out by the bulge, round to the
    /// bottom centre at the arch's depth there. `abut`: (s, +1 or -1 into the bridge) where the run
    /// continues an embankment (no bulge or lumps there, so the rock stays inside it).
    #[allow(clippy::too_many_arguments)]
    fn rock_body(&mut self, g: &PathGeo, b0: usize, b1: usize, span: (f64, f64), abut: &[(f64, f64)], rk: &Rock, caps: (bool, bool)) {
        let Some(ws) = self.theme.wall_styles.get(&rk.style).cloned() else {
            self.problems.push(format!("theme {} has no wall style {:?} for rock", self.theme.name, rk.style));
            return;
        };
        let seed = self.seed().wrapping_add(23);
        let smooth = |a: f64, b: f64, x: f64| {
            let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        };
        let mut frames = vec![];
        let mut halves = vec![];
        for i in b0..=b1 {
            let st = &g.st[i];
            let t = if span.1 - span.0 > 1e-6 { ((st.s - span.0) / (span.1 - span.0)).clamp(0.0, 1.0) } else { 0.5 };
            let depth = rk.depth_mid + (rk.depth_end - rk.depth_mid) * (1.0 - (std::f64::consts::PI * t).sin()).powf(1.5);
            let taper = abut.iter().map(|&(s, dir)| smooth(0.0, st.w, (st.s - s) * dir)).fold(1.0, f64::min);
            let hw = st.w * 0.5;
            halves.push(Half::rock(hw, hw * (1.0 + rk.bulge * taper), rk.lip.min(depth * 0.5), depth, taper));
            frames.push(Frame { o: [st.p[0], st.p[1], st.z], left: [-st.dir[1], st.dir[0], 0.0], down: [0.0, 0.0, -1.0], fwd: [st.dir[0], st.dir[1], 0.0], s: st.s });
        }
        rocks::sweep(&mut self.mesh, "bridge_rock", &frames, &halves, &ws, Lumps { amount: rk.lumps, scale: rk.lump_scale, seed }, caps, WallTexture::Tiled, 8);
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
        let mut lines: Vec<Vec<P2>> = vec![];
        for (hs, top) in rims {
            let hs = hs.clone();
            let n = hs.len();
            // part forest, part sky (bands' far edges): the forest only where it stands
            if !hs.iter().all(|&h| self.forest(h)) {
                if hs.iter().any(|&h| self.forest(h)) {
                    self.partial_boundary(&hs, top, cliff);
                }
                continue;
            }
            let o = self.map.half_pts(&hs); // clockwise, the level on the right
            // the cliffs
            let total: f64 = hs.iter().map(|&h| self.map.len(h)).sum();
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
            let (tile, reps) = self.wall_tiling(cliff, mean, Some(hs.iter().map(|&h| self.ulen(h)).sum()));
            let mut u = 0.0;
            for i in 0..n {
                let h = hs[i];
                let l = self.ulen(h);
                let f = self.map.half_face[h ^ 1];
                let w = self.water(f);
                self.wall("cliffs", Some(h), self.map.from(h), self.map.to(h), floor(self, i), [top[i], top[(i + 1) % n]], cliff, [u, u + l], tile, reps, w, 0.0);
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
            lines.push(t.clone());
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
        // separate areas (regions outside the outline) each have their own edge of the world: their
        // forests mustn't run into each other
        for i in 0..lines.len() {
            for j in i + 1..lines.len() {
                let (a, b) = (&lines[i], &lines[j]);
                let hit = (0..a.len()).find_map(|k| {
                    let (p, q) = (a[k], a[(k + 1) % a.len()]);
                    (0..b.len()).find(|&m| segments_cross(p, q, b[m], b[(m + 1) % b.len()])).map(|_| p)
                });
                if let Some(p) = hit.or_else(|| point_in_poly(a[0], b).then_some(a[0])).or_else(|| point_in_poly(b[0], a).then_some(b[0])) {
                    self.problems.push(format!(
                        "two areas' edges of the world run into each other near ({:.0}, {:.0}): keep separate areas at least {:.0} apart (twice the bank)",
                        p[0], p[1], 2.0 * bd.bank
                    ));
                }
            }
        }
    }
}

impl<'a> Builder<'a> {
    /// The walls standing on bands' far edges (`Beyond::skyline`, Kakariko's mossy wall): from a
    /// little under the ground there up its height, which varies with the band's roughness and
    /// comes down towards an end the forest is beside; the style stretched once over each column,
    /// as spot01's is, so a cut-out top is the skyline.
    fn skylines(&mut self) {
        let th = self.theme;
        for sk in self.bands.skylines.clone() {
            let ws = match th.wall_styles.get(&sk.style) {
                Some(ws) => ws,
                None => {
                    self.problems.push(format!("theme {} has no wall style {:?} for a skyline wall, so it's the theme's cliff", th.name, sk.style));
                    th.wall_styles.get(th.wall_style(f64::INFINITY, false)).expect("the theme's own style")
                }
            };
            let n = sk.pts.len();
            let mut s = vec![0.0; n];
            for i in 1..n {
                s[i] = s[i - 1] + dist(sk.pts[i - 1], sk.pts[i]);
            }
            let total = s[n - 1];
            // the ground under each point (the band's crest there), and how high the wall rises
            let taper = 1.2 * sk.height;
            let rough = crate::beyond::Rough { base: 0.0, amount: sk.amount, seed: sk.seed, far: vec![] };
            let cols: Vec<(f64, f64)> = (0..n)
                .map(|i| {
                    let p = sk.pts[i];
                    let z = self.field.z(sk.l, p, p);
                    let z = self.bands.rough.get(&sk.l).map_or(z, |r| self.rough_z(sk.l, r, p, z));
                    let mut e = 1.0f64;
                    for (open, d) in [(sk.open[0], s[i]), (sk.open[1], total - s[i])] {
                        if open {
                            let t = (d / taper).clamp(0.0, 1.0);
                            e = e.min(0.12 + 0.88 * t * t * (3.0 - 2.0 * t));
                        }
                    }
                    (z, sk.height * e * rough.factor(p))
                })
                .collect();
            // a little under the ground, which runs straight between the points as the wall does
            const SINK: f64 = 30.0;
            for i in 0..n - 1 {
                let (a, b) = (sk.pts[i], sk.pts[i + 1]);
                let ((za, ha), (zb, hb)) = (cols[i], cols[i + 1]);
                let (ua, ub) = (s[i] / ws.tile_u, s[i + 1] / ws.tile_u);
                let (ba, bb) = (za - SINK, zb - SINK);
                let (ta, tb) = (za + ha, zb + hb);
                self.mesh.quad(
                    "cliffs",
                    [[a[0], a[1], ba], [b[0], b[1], bb], [b[0], b[1], tb], [a[0], a[1], ta]],
                    [[ua, 0.0], [ub, 0.0], [ub, 1.0], [ua, 1.0]],
                    &ws.material,
                    &ws.surface,
                );
            }
        }
    }

    /// The edge of the world round an outer loop that's forest only in places (`beyond.rs`): the
    /// cliffs on its forest half-edges, and the bank and trees along the stretches of the tree
    /// line nearest them, each closed off where it ends.
    fn partial_boundary(&mut self, hs: &[usize], top: &[f64], cliff: &'a WallStyle) {
        let th = self.theme;
        let bd = &self.doc.boundary;
        let n = hs.len();
        let o = self.map.half_pts(hs); // clockwise, the level on the right
        let forest: Vec<bool> = hs.iter().map(|&h| self.forest(h)).collect();
        let floor = |b: &Self, i: usize| {
            let (h, f) = (hs[i], b.map.half_face[hs[i] ^ 1]);
            [b.hv(f, b.map.from(h)), b.hv(f, b.map.to(h))]
        };
        // the cliffs
        let total: f64 = (0..n).filter(|&i| forest[i]).map(|i| self.map.len(hs[i])).sum();
        let mean: f64 = (0..n)
            .filter(|&i| forest[i])
            .map(|i| {
                let fl = floor(self, i);
                self.map.len(hs[i]) * (0.5 * (top[i] + top[(i + 1) % n]) - 0.5 * (fl[0] + fl[1]))
            })
            .sum::<f64>()
            / total.max(1e-9);
        let (tile, reps) = self.wall_tiling(cliff, mean, None);
        let mut u = 0.0;
        for i in (0..n).filter(|&i| forest[i]) {
            let h = hs[i];
            let l = self.ulen(h);
            let w = self.water(self.map.half_face[h ^ 1]);
            self.wall("cliffs", Some(h), self.map.from(h), self.map.to(h), floor(self, i), [top[i], top[(i + 1) % n]], cliff, [u, u + l], tile, reps, w, 0.0);
            u += l;
        }
        // the bands' side edges: a face from the band's floor down to the level's, facing out
        let base = self.doc.outline.z;
        let sides: Vec<usize> = (0..n).filter(|&i| self.side(hs[i])).collect();
        for i in sides {
            let h = hs[i];
            let fl = floor(self, i);
            let low = base.min(fl[0]).min(fl[1]);
            self.wall("cliffs", None, self.map.to(h), self.map.from(h), [low, low], [fl[1], fl[0]], cliff, [0.0, self.ulen(h)], tile, reps, false, 0.0);
        }
        // the tree line, kept where its nearest outer edge is forest
        let traced = tree_line(&o, bd.bank, bd.panel_tol);
        if traced.len() < 3 {
            self.problems.push("no tree line could be traced round the outline".into());
            return;
        }
        // the outer edge nearest p (of the forest's only, or any) and how far along it
        let nearest = |p: P2, forest_only: bool| {
            let (mut best, mut at) = (f64::INFINITY, (0, 0.0));
            for i in (0..n).filter(|&i| !forest_only || forest[i]) {
                let (d, s) = dist_to_seg(p, o[i], o[(i + 1) % n]);
                if d < best {
                    best = d;
                    at = (i, s);
                }
            }
            at
        };
        // the tree line is simplified into long pieces, which may run from the forest's part to a
        // band's, or lie beside the forest only in their middle: each is walked in short steps
        // and split exactly where its nearest edge changes, so the forest runs right up to a band
        // (else a whole outline edge beside a band could lose its trees). Each stretch kept is
        // simplified again below.
        let step = (bd.bank / 4.0).max(10.0);
        let (mut t, mut near, mut keep) = (vec![], vec![], vec![]);
        for j in 0..traced.len() {
            let (p, q) = (traced[j], traced[(j + 1) % traced.len()]);
            let k = (dist(p, q) / step).ceil().max(1.0) as usize;
            for i in 0..k {
                let (s0, s1) = (i as f64 / k as f64, (i + 1) as f64 / k as f64);
                let x0 = lerp(p, q, s0);
                let n0 = nearest(x0, false);
                t.push(x0);
                near.push(n0);
                keep.push(forest[n0.0]);
                if forest[n0.0] == forest[nearest(lerp(p, q, s1), false).0] {
                    continue;
                }
                // a: the forest's end of the step, b: the band's
                let (mut a, mut b) = if forest[n0.0] { (s0, s1) } else { (s1, s0) };
                for _ in 0..40 {
                    let mid = 0.5 * (a + b);
                    if forest[nearest(lerp(p, q, mid), false).0] {
                        a = mid;
                    } else {
                        b = mid;
                    }
                }
                let x = lerp(p, q, a);
                if dist(x, x0) > 1.0 && dist(x, lerp(p, q, s1)) > 1.0 {
                    t.push(x);
                    near.push(nearest(x, true));
                    keep.push(true);
                }
            }
        }
        let m = t.len();
        let Some(start) = (0..m).find(|&k| keep[k] && !keep[(k + m - 1) % m]) else { return };
        let mut k = 0;
        while k < m {
            let i0 = (start + k) % m;
            if !keep[i0] {
                k += 1;
                continue;
            }
            let mut run = vec![];
            while k < m && keep[(start + k) % m] {
                run.push((start + k) % m);
                k += 1;
            }
            if run.len() < 2 {
                continue;
            }
            let (mut last, mut first) = (near[run[run.len() - 1]].0, near[run[0]].0);
            // beside a band's side edge, the stretch reaches over to it: its end dropped square
            // onto the side edge, at the band's floor there, so the trees and the bank end at the
            // foot of the band's ground. The outer edges are in pieces: from the piece nearest the
            // end, along the forest's to the corner, then along the side edge's out from it
            // (`step`: n - 1 going back round the loop, 1 forward).
            let reach_over = |end: usize, from: usize, step: usize| -> Option<(usize, P2, f64)> {
                let mut c = from;
                for _ in 0..n {
                    let next = (c + step) % n;
                    if !forest[next] {
                        break;
                    }
                    c = next;
                }
                // the corner, and the side edge's pieces from it
                let mut i = (c + step) % n;
                let corner = if step == 1 { o[i] } else { o[c] };
                let mut best: Option<(f64, P2, f64)> = None;
                for _ in 0..n {
                    if !self.side(hs[i]) {
                        break;
                    }
                    let (a, b) = (o[i], o[(i + 1) % n]);
                    let (d, s) = dist_to_seg(t[end], a, b);
                    if best.is_none_or(|x| d < x.0) {
                        let fl = floor(self, i);
                        best = Some((d, lerp(a, b, s), fl[0] + (fl[1] - fl[0]) * s));
                    }
                    i = (i + step) % n;
                }
                let (d, q, z) = best?;
                (d >= 1.0 && dist(q, corner) > 1.0).then_some((c, q, z))
            };
            let (end, start) = (run[run.len() - 1], run[0]);
            let after = reach_over(end, last, n - 1);
            let before = reach_over(start, first, 1);
            if let Some((c, ..)) = after {
                last = c;
            }
            if let Some((c, ..)) = before {
                first = c;
            }
            // the stretch as few points as the tree line had (its ends kept)
            let line = simplify_line(&run.iter().map(|&j| t[j]).collect::<Vec<_>>(), bd.panel_tol, 0.0);
            let rim_at = |p: P2| {
                let (i, s) = nearest(p, true);
                top[i] + (top[(i + 1) % n] - top[i]) * s + bd.bank_rise
            };
            let (pts, zt): (Vec<P2>, Vec<f64>) = before.map(|x| (x.1, x.2)).into_iter().chain(line.into_iter().map(|p| (p, rim_at(p)))).chain(after.map(|x| (x.1, x.2))).unzip();
            for &z in &zt {
                self.rim = (self.rim.0.min(z), self.rim.1.max(z));
            }
            // the bank: the tree line's stretch (counter-clockwise), then back along the outer
            // edges nearest it (clockwise), from the last's start to the first's end
            let mut poly = pts.clone();
            let mut i = last;
            loop {
                poly.push(o[i]);
                if i == (first + 1) % n {
                    break;
                }
                i = (i + 1) % n;
                if poly.len() > n + pts.len() + 1 {
                    break;
                }
            }
            let key = |p: P2| ((p[0] * 1000.0).round() as i64, (p[1] * 1000.0).round() as i64);
            let mut zs: HashMap<(i64, i64), f64> = HashMap::new();
            for i in 0..n {
                zs.insert(key(o[i]), top[i]);
            }
            for (p, z) in pts.iter().zip(&zt) {
                zs.insert(key(*p), *z);
            }
            for tri in triangulate(&poly, &[], 0.0) {
                let Some(p) = tri.iter().map(|q| zs.get(&key(*q)).map(|&z| [q[0], q[1], z])).collect::<Option<Vec<_>>>() else {
                    self.problems.push("bank triangle off the outline and tree line".into());
                    continue;
                };
                let uv = tri.map(|q| [q[0] / th.bank.tile, q[1] / th.bank.tile]);
                self.mesh.tri("bank", [p[0], p[1], p[2]], uv, &th.bank.material, &th.bank.surface);
            }
            // trunks along the stretch facing in, foliage over them and a little in front
            let tr = &th.trees;
            let r: Vec<P2> = pts.iter().rev().copied().collect(); // the level on the right
            let rz: Vec<f64> = zt.iter().rev().copied().collect();
            let fol = offset_open_right(&r, tr.foliage_in);
            let len = |q: &[P2]| q.windows(2).map(|w| dist(w[0], w[1])).sum::<f64>();
            let (tt, tf) = (snap(len(&r), tr.trunk_tile), snap(len(&fol), tr.foliage_tile));
            let (mut ut, mut uf) = (0.0, 0.0);
            for i in 0..r.len() - 1 {
                let j = i + 1;
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

/// An open line moved `d` to its right (each point along the mean of its pieces' normals).
fn offset_open_right(pts: &[P2], d: f64) -> Vec<P2> {
    let n = pts.len();
    let right = |a: P2, b: P2| {
        let v = sub(b, a);
        let l = v[0].hypot(v[1]).max(1e-9);
        [v[1] / l, -v[0] / l]
    };
    (0..n)
        .map(|i| {
            let a = if i > 0 { right(pts[i - 1], pts[i]) } else { right(pts[0], pts[1]) };
            let b = if i + 1 < n { right(pts[i], pts[i + 1]) } else { a };
            let m = [a[0] + b[0], a[1] + b[1]];
            let l = m[0].hypot(m[1]).max(1e-9);
            let m = [m[0] / l, m[1] / l];
            let cosa = (m[0] * a[0] + m[1] * a[1]).max(0.5);
            [pts[i][0] + m[0] * d / cosa, pts[i][1] + m[1] * d / cosa]
        })
        .collect()
}

/// How closely lines derived from the curves (terraces' steps, slopes' rings) follow them.
fn curve_tol(s: Sampling) -> f64 {
    match s {
        Sampling::Within { tol, .. } | Sampling::Facets { tol, .. } => tol.min(5.0),
        _ => 1.0,
    }
}

/// A point well inside face f: the middle of the roundest triangle of its triangulation.
fn anchor(map: &Map, f: usize) -> P2 {
    let face = &map.faces[f];
    let outer = map.cycle_pts(face.outer);
    let holes: Vec<Vec<P2>> = face.holes.iter().map(|&c| map.cycle_pts(c)).collect();
    let mut best = (f64::NEG_INFINITY, outer[0]);
    for [a, b, c] in triangulate(&outer, &holes, 0.0) {
        let (la, lb, lc) = (dist(b, c), dist(a, c), dist(a, b));
        let per = la + lb + lc;
        if per < 1e-12 {
            continue;
        }
        let r = cross(a, b, c).abs() / per;
        if r > best.0 {
            best = (r, [(la * a[0] + lb * b[0] + lc * c[0]) / per, (la * a[1] + lb * b[1] + lc * c[1]) / per]);
        }
    }
    best.1
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
            outline: Outline { nodes: outline, z: 0.0, noise: None, beyond: vec![] },
            regions: vec![
                Region { name: "north".into(), nodes: north, z: 160.0, kind: "floor".into(), surface: None, edge: None, noise: None, profile: None, profiles: vec![] },
                Region { name: "island".into(), nodes: ring(-500.0, -300.0, 300.0), z: 120.0, kind: "floor".into(), surface: None, edge: None, noise: None, profile: None, profiles: vec![] },
                Region { name: "pond".into(), nodes: ring(500.0, -400.0, 280.0), z: -100.0, kind: "water".into(), surface: Some(-20.0), edge: None, noise: None, profile: None, profiles: vec![] },
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
        let zs: Vec<f64> = std::iter::once(0.0).chain(doc.regions.iter().map(|r| r.z)).collect();
        let field = profiles::Field::new(&doc, &map, &zs, 35.0, &mut vec![]).unwrap();
        let anchors = vec![None; map.faces.len()];
        let mut b = Builder { doc: &doc, theme: &Theme::kokiri(), regions: vec![], paths: vec![], mesh: Mesh::default(), problems: vec![], levels: vec![vec![]; map.verts.len()], rim: (0.0, 0.0), jobs: vec![], mids: HashMap::new(), extra: HashMap::new(), walls3: false, wall_texture: WallTexture::Tiled, dirt: DirtPaths::default(), pads: vec![], map, field, anchors, bent: HashMap::new(), sky: vec![], bands: Default::default(), ribbons: vec![], blends: Default::default() };
        b.regions = std::iter::once(Info { z: 0.0, water: None, pit: false, edge: None, noise: None })
            .chain(doc.regions.iter().map(|r| Info { z: r.z, water: (r.kind == "water").then_some(-20.0), pit: false, edge: None, noise: None }))
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
        let (m0, m1) = c.middle();
        let check = |b: &[(f64, f64, f64, f64, bool)]| {
            for w in b.windows(2) {
                assert!((w[0].1 - w[1].0).abs() < 1e-9, "bands meet");
                assert!((w[0].3 - w[1].2).abs() < 0.5 / c.rows as f64 + 1e-9, "v meets across every join: {b:?}");
            }
        };
        // every wall taller than the texture: caps at their own size, the middle an odd number
        // of whole repeats (mirrored), each exactly its own size, the rest folded at its foot
        for h in [170.0, 230.0, 300.0, 400.0, 520.0, 777.0, 1500.0] {
            let reps = c.repeats(h, true);
            assert!(c.mirror && reps % 2 == 1, "{h}: {reps}");
            // without the fold, the odd count that stretches least: never more than about 1.7 x
            let r = c.repeats(h, false);
            let s = (h - (c.bottom + c.top) * c.tile_v) / (r as f64 * c.unit());
            assert!(r % 2 == 1 && (1.0 / 1.75..=1.75).contains(&s), "{h}: {r} repeats at {s}");
            let b = Builder::bands(ws, reps, 100.0, 100.0 + h, false, true);
            check(&b);
            assert_eq!(b.len(), reps + 4);
            assert!((b[0].1 - b[0].0 - c.bottom * c.tile_v).abs() < 1e-9);
            assert!((b[reps + 3].1 - b[reps + 3].0 - c.top * c.tile_v).abs() < 1e-9);
            let fold = b[1].1 - b[1].0;
            assert!(fold >= 0.0 && fold < c.unit() && (b[2].1 - b[2].0 - fold).abs() < 1e-9, "{h}: fold {fold}");
            assert_eq!((b[1].2, b[2].3), (m0, m0), "the fold starts and ends at the middle's foot");
            for (k, m) in b[3..3 + reps].iter().enumerate() {
                assert!((m.1 - m.0 - c.unit()).abs() < 1e-6, "{h}: a whole repeat is {}", m.1 - m.0);
                assert_eq!((m.2, m.3), if k % 2 == 0 { (m0, m1) } else { (m1, m0) });
            }
            // three-band walls: the same caps and fold, and one middle band counting its repeats in v
            let b3 = Builder::bands(ws, reps, 100.0, 100.0 + h, true, true);
            assert_eq!(b3.len(), 5);
            assert_eq!((b3[0], b3[4]), (b[0], b[reps + 3]));
            assert!((b3[1].1 - b[1].1).abs() < 1e-9 && b3[1].2 == 0.0 && b3[2].3 == 0.0 && b3[1].4 && b3[2].4);
            assert_eq!((b3[3].0, b3[3].1, b3[3].2, b3[3].3, b3[3].4), (b[3].0, b[reps + 2].1, 0.0, reps as f64, true));
        }
        // the middle stretched instead: no fold, the repeat fills the rest
        let b = Builder::bands(ws, 1, 0.0, 400.0, false, false);
        check(&b);
        assert_eq!(b.len(), 3);
        assert!((b[1].1 - b[1].0 - (400.0 - (c.bottom + c.top) * c.tile_v)).abs() < 1e-9);
        // short walls show the texture once, stretched
        assert_eq!((c.repeats(120.0, true), c.repeats(120.0, false)), (0, 0));
        assert_eq!(Builder::bands(ws, 0, 0.0, 120.0, false, true), vec![(0.0, 120.0, 0.0, 1.0, false)]);
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
            ..Default::default()
        };
        let n = |x: f64, y: f64| vec![Some(x), Some(y)];
        Doc {
            name: "paths".into(),
            outline: Outline { nodes: sq(-2000.0, -2000.0, 4000.0, 4000.0), z: 0.0, noise: None, beyond: vec![] },
            regions: vec![
                Region { name: "east".into(), nodes: sq(600.0, 600.0, 800.0, 800.0), z: 240.0, kind: "floor".into(), surface: None, edge: None, noise: None, profile: None, profiles: vec![] },
                Region { name: "west".into(), nodes: sq(-1400.0, 600.0, 800.0, 800.0), z: 240.0, kind: "floor".into(), surface: None, edge: None, noise: None, profile: None, profiles: vec![] },
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

    /// The ramp runs on 300 into the east plateau (from its edge at y 600 to its node at y 900):
    /// a cutting, its floor below the plateau's 240 and rising to it at the node, walled on both
    /// sides by the plateau's own cliff, and open where it meets the plateau's edge.
    #[test]
    fn a_ramp_into_a_plateau_cuts_in() {
        let lvl = build(&paths_doc(), &Theme::kokiri()).unwrap();
        let obj = |n: &str| lvl.mesh.objects.iter().find(|o| o.name == n).unwrap_or_else(|| panic!("no {n}"));
        // the ramp climbs 240 over its 1200: 180 at the plateau's edge
        // (the plateau's floor shares the cutting's edges, at 240)
        let want = |v: &P3| 240.0 * (v[1] + 300.0) / 1200.0;
        let cut: Vec<&P3> = obj("ground").verts.iter().filter(|v| (v[0] - 1000.0).abs() < 81.0 && v[1] > 610.0 && v[1] < 890.0).collect();
        assert!(cut.iter().any(|v| (v[2] - want(v)).abs() < 2.0 && v[2] < 230.0), "the cutting's floor");
        for v in &cut {
            let edge = (v[0] - 1000.0).abs() > 79.0 && (v[2] - 240.0).abs() < 1e-6;
            assert!((v[2] - want(v)).abs() < 2.0 || edge, "the cutting's floor at {v:?}, not {}", want(v));
        }
        // walls on both sides, from the cutting's floor up to the plateau's top
        let walls = obj("walls");
        for side in [920.0, 1080.0] {
            let w: Vec<&P3> = walls.verts.iter().filter(|v| (v[0] - side).abs() < 1.0 && v[1] > 650.0 && v[1] < 850.0).collect();
            assert!(w.iter().any(|v| (v[2] - 240.0).abs() < 1e-6) && w.iter().any(|v| v[2] < 220.0), "the cutting's side at x {side}");
        }
        // no wall across the mouth, above the ramp, where it passes the plateau's edge
        assert!(!walls.verts.iter().any(|v| (v[0] - 1000.0).abs() < 70.0 && (v[1] - 600.0).abs() < 1.0 && v[2] > 182.0));
    }

    fn open_edges(lvl: &Level, objs: &[&str]) -> Vec<((i64, i64, i64), (i64, i64, i64))> {
        open_edges_under(lvl, objs, Theme::kokiri().trees.trunks)
    }

    /// Open edges, but for the tree tops (`trunks` over the rim).
    fn open_edges_under(lvl: &Level, objs: &[&str], trunks: f64) -> Vec<((i64, i64, i64), (i64, i64, i64))> {
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
    fn props_level_the_bumps_under_them() {
        use crate::doc::Prop;
        use crate::pieces::{Door, Piece};
        // a 200 x 160 hut 120 tall, door at the front, eaves 60 out all round up at 120
        let mut verts = vec![];
        for (r, z) in [(1.0, 0.0), (1.0, 120.0), (1.6, 120.0)] {
            verts.extend([[-100.0 * r, -80.0 * r, z], [100.0 * r, -80.0 * r, z], [100.0 * r, 80.0 * r, z], [-100.0 * r, 80.0 * r, z]]);
        }
        let hut = Piece {
            name: "hut".into(),
            kind: "house".into(),
            footprint: crate::pieces::hull(verts.iter().map(|q| [q[0], q[1]]).collect()),
            bounds: [[-160.0, -128.0, 0.0], [160.0, 128.0, 120.0]],
            verts,
            door: Some(Door { pos: [0.0, 80.0, 0.0], exit: 1, entrance: "X".into() }),
            ..Default::default()
        };
        assert_eq!(hut.base_outline().len(), 4, "the walls' feet, not the eaves");
        let kit = Kit { pieces: vec![hut.clone()], ..Default::default() };
        let th = Theme::kokiri();
        let hut_at = |at: [f64; 2], level: Option<bool>| Prop { level, piece: "hut".into(), at, z: None, yaw: 30.0, scale: [1.0; 3] };
        let ground_of = |lvl: &Level| props::Ground::new(&lvl.mesh);
        // the ground's heights under a hut: its base's corners, its middle and its doorway
        let under = |lvl: &Level, prop: &Prop| -> Vec<f64> {
            let g = ground_of(lvl);
            let pad = &props::pads(std::slice::from_ref(&Prop { level: Some(true), ..prop.clone() }), Some(&kit))[0];
            pad.ring.iter().chain([&prop.at, &pad.anchor]).map(|&p| g.at(&lvl.mesh, p).0.unwrap()).collect()
        };
        let spread = |zs: &[f64]| zs.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)) - zs.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let (here, there) = ([0.0, 200.0], [-900.0, 300.0]);
        let mut doc = bumpy(sample_doc());
        let bare = build_with(&doc, &th, Some(&kit)).unwrap();
        // left alone (or told not to), the hut's corners stand on bumps
        doc.props = vec![hut_at(here, Some(false))];
        let lvl = build_with(&doc, &th, Some(&kit)).unwrap();
        assert!(spread(&under(&lvl, &doc.props[0])) > 2.0, "{:?}", under(&lvl, &doc.props[0]));
        // levelled: flat under it, at the bump at its door, and still watertight
        doc.props = vec![hut_at(here, None)];
        let lvl = build_with(&doc, &th, Some(&kit)).unwrap();
        let zs = under(&lvl, &doc.props[0]);
        assert!(spread(&zs) < 1e-6, "{zs:?}");
        assert!((lvl.props[0].origin[2] - zs[0]).abs() < 1e-6);
        assert!(!lvl.problems.iter().any(|p| p.contains("falls")), "{:?}", lvl.problems);
        let bad = open_edges(&lvl, &["ground", "walls", "cliffs", "bank", "trees"]);
        assert!(bad.is_empty(), "{} open edges, e.g. {:?}", bad.len(), &bad[..bad.len().min(6)]);
        // moved: level where it is now, and its old spot is as it was with no hut
        doc.props = vec![hut_at(there, None)];
        let lvl = build_with(&doc, &th, Some(&kit)).unwrap();
        assert!(spread(&under(&lvl, &doc.props[0])) < 1e-6);
        let old = under(&lvl, &hut_at(here, None));
        let was = under(&bare, &hut_at(here, None));
        assert!(old.iter().zip(&was).all(|(a, b)| (a - b).abs() < 1e-6), "{old:?} vs {was:?}");
        // deleted: the ground is just the bumps again
        doc.props.clear();
        let lvl = build_with(&doc, &th, Some(&kit)).unwrap();
        let (a, b) = (lvl.mesh.objects.iter().find(|o| o.name == "ground").unwrap(), bare.mesh.objects.iter().find(|o| o.name == "ground").unwrap());
        assert!(a.verts == b.verts && a.tris == b.tris);
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

    /// Faceted and hard edges: fewer corners round the curves (the straight pieces keep their
    /// points, for terrain and walls), the hard ones straight from node to node, and every level
    /// still closed.
    #[test]
    fn faceted_and_hard_edges_are_lighter_and_still_watertight() {
        let th = Theme::kokiri();
        let mut total = [0; 3];
        for doc in [sample_doc(), paths_doc(), painted(bumpy(paths_doc()))] {
            let mut corners = vec![];
            for edges in ["smooth", "faceted", "hard"] {
                let mut d = doc.clone();
                d.settings.edges = edges.into();
                let lvl = build(&d, &th).unwrap();
                assert!(lvl.problems.is_empty(), "{} {edges}: {:?}", doc.name, lvl.problems);
                let bad = open_edges(&lvl, &["ground", "walls", "cliffs", "bank", "trees"]);
                assert!(bad.is_empty(), "{} {edges}: {} open edges, e.g. {:?}", doc.name, bad.len(), &bad[..bad.len().min(6)]);
                let lp = crate::map::sample_loops(&d).unwrap();
                corners.push(lp.polys.iter().map(|p| simplify_closed(p, 0.5).len()).sum::<usize>());
                if edges == "hard" {
                    // every loop point lies on the straight line between two of its nodes
                    for (ids, poly) in lp.nodes.iter().zip(&lp.polys) {
                        let nodes: Vec<P2> = ids.iter().map(|&i| lp.verts[i]).collect();
                        let on = |p: P2| (0..nodes.len()).any(|k| dist_to_seg(p, nodes[k], nodes[(k + 1) % nodes.len()]).0 < 1e-6);
                        assert!(poly.iter().all(|&p| on(p)), "{}", doc.name);
                    }
                }
            }
            assert!(corners[1] <= corners[0] && corners[2] <= corners[1], "{}: {corners:?}", doc.name);
            for (t, c) in total.iter_mut().zip(corners) {
                *t += c;
            }
        }
        assert!(total[1] < total[0] && total[2] < total[1], "{total:?}");
        let mut d = sample_doc();
        d.settings.edges = "wavy".into();
        assert!(build(&d, &th).is_err());
    }

    /// Stretched walls: the texture once over each wall's height (v from 0 at the foot to 1 at the
    /// top), and across it repeats only as often as keeps its shape; tiled ones repeat far more.
    #[test]
    fn walls_tile_or_stretch() {
        let th = Theme::kokiri();
        let mut d = sample_doc();
        d.settings.edges = "hard".into();
        d.regions = vec![Region {
            name: "block".into(),
            nodes: vec![vec![-700.0, -500.0], vec![-300.0, -500.0], vec![-300.0, -100.0], vec![-700.0, -100.0]],
            z: 400.0,
            kind: "floor".into(),
            surface: None,
            edge: None,
            noise: None,
            profile: None,
            profiles: vec![],
        }];
        let mut most_u = vec![];
        for (mode, detail) in [("tiled", "high"), ("stretched", "high"), ("stretched", "low"), ("stretched_middle", "high")] {
            let mut d = d.clone();
            d.settings.wall_texture = mode.into();
            d.settings.detail = detail.into();
            let lvl = build(&d, &th).unwrap();
            assert!(lvl.problems.is_empty(), "{mode}: {:?}", lvl.problems);
            let o = lvl.mesh.objects.iter().find(|o| o.name == "walls").unwrap();
            if mode == "stretched" {
                for (t, uv) in o.tris.iter().zip(&o.uvs) {
                    for k in 0..3 {
                        let z = o.verts[t[k]][2];
                        assert!((uv[k][1] - z / 400.0).abs() < 1e-6, "{mode} {detail}: v {} at z {z}", uv[k][1]);
                    }
                }
                assert!(!lvl.mesh.materials.iter().any(|m| m.ends_with("~mid")), "no middle band to repeat");
            }
            if mode == "stretched_middle" {
                // the caps at their own size, the middle once over the rest
                let c = th.wall_styles["cliff"].caps.clone().unwrap();
                let (hb, ht) = (c.bottom * c.tile_v, c.top * c.tile_v);
                let (m0, m1) = c.middle();
                for (t, uv) in o.tris.iter().zip(&o.uvs) {
                    let zs = t.map(|v| o.verts[v][2]);
                    // which band the triangle is in, by its middle
                    let zc = (zs[0] + zs[1] + zs[2]) / 3.0;
                    for k in 0..3 {
                        let z = zs[k];
                        let v = if zc < hb {
                            z / hb * c.bottom
                        } else if zc > 400.0 - ht {
                            1.0 - c.top + (z - (400.0 - ht)) / ht * c.top
                        } else {
                            m0 + (z - hb) / (400.0 - hb - ht) * (m1 - m0)
                        };
                        assert!((uv[k][1] - v).abs() < 1e-6, "v {} at z {z}, not {v}", uv[k][1]);
                    }
                }
            }
            let u = o.uvs.iter().flatten().map(|q| q[0]).fold(f64::NEG_INFINITY, f64::max);
            assert!((u - u.round()).abs() < 1e-6, "{mode}: whole repeats round the block, {u}");
            most_u.push(u);
        }
        // round the 1600 of wall: 167 per repeat tiled (and with the middle stretched), 400 (the
        // height) stretched
        assert!(most_u[1] < most_u[0] / 2.0 && most_u[1] == most_u[2] && most_u[3] == most_u[0], "{most_u:?}");
        let mut bad = d.clone();
        bad.settings.wall_texture = "wavy".into();
        assert!(build(&bad, &th).is_err());
    }

    /// A hedge drawn as a closed line: a top 28 over the ground covering the shape, skirts facing
    /// out, and a tall-grass floor under it that only collides.
    #[test]
    fn hedges_cover_their_shape() {
        let th = Theme::kokiri();
        let mut d = sample_doc();
        // drawn clockwise: the builder turns it round
        let sq = [[-200.0, 0.0], [-200.0, 300.0], [200.0, 300.0], [200.0, 0.0]];
        d.lines.push(Line { name: "h".into(), kind: "hedge".into(), nodes: sq.iter().map(|p| vec![p[0], p[1]]).collect(), width: None, closed: false, height: None, noise: None, ..Default::default() });
        let lvl = build(&d, &th).unwrap();
        assert!(lvl.problems.is_empty(), "{:?}", lvl.problems);
        let o = lvl.mesh.objects.iter().find(|o| o.name == "hedges").unwrap();
        assert!(o.verts.iter().all(|v| v[2].abs() < 1e-6 || (v[2] - 28.0).abs() < 1e-6));
        let (mut top, mut sides) = (0.0, 0);
        for t in &o.tris {
            let p = t.map(|v| o.verts[v]);
            let n = [
                (p[1][1] - p[0][1]) * (p[2][2] - p[0][2]) - (p[1][2] - p[0][2]) * (p[2][1] - p[0][1]),
                (p[1][2] - p[0][2]) * (p[2][0] - p[0][0]) - (p[1][0] - p[0][0]) * (p[2][2] - p[0][2]),
                (p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[1][1] - p[0][1]) * (p[2][0] - p[0][0]),
            ];
            if n[2].abs() > 1e-6 {
                assert!(n[2] > 0.0, "the top faces up");
                top += n[2] / 2.0;
            } else {
                // out from the middle (0, 150)
                let c = [(p[0][0] + p[1][0] + p[2][0]) / 3.0, (p[0][1] + p[1][1] + p[2][1]) / 3.0 - 150.0];
                assert!(n[0] * c[0] + n[1] * c[1] > 0.0, "a skirt faces in at {c:?}");
                sides += 1;
            }
        }
        assert!((top - 400.0 * 300.0).abs() < 1.0, "{top}");
        assert!(sides >= 2 * 14, "{sides}: a quad every 100 round 1400");
        let c = lvl.mesh.objects.iter().find(|o| o.name == "hedges_collision").unwrap();
        assert!(c.collision_only && c.verts.iter().all(|v| (v[2] - 2.0).abs() < 1e-6));
        assert!(lvl.mesh.surfaces.iter().any(|s| s == "tall_grass"));
        // a shape crossing itself is reported
        let mut d = sample_doc();
        let bow = [[-200.0, 0.0], [200.0, 300.0], [200.0, 0.0], [-200.0, 300.0]];
        d.lines.push(Line { name: "bow".into(), kind: "hedge".into(), nodes: bow.iter().map(|p| vec![p[0], p[1]]).collect(), width: None, closed: true, height: None, noise: None, ..Default::default() });
        let lvl = build(&d, &th).unwrap();
        assert!(lvl.problems.iter().any(|p| p.contains("crosses itself")), "{:?}", lvl.problems);
    }

    /// Vines (a tiling wall piece) reach from the floor to the wall's top however tall, repeat
    /// their texture instead of stretching, and are as wide as the flat face lets them be.
    #[test]
    fn vines_tile_and_reach_the_top() {
        use crate::doc::Prop;
        use crate::pieces::{Kit, Piece, PieceMaterial, Scale};
        let th = Theme::kokiri();
        // 200 x 140 on the wall's face (y 0, facing +y), its texture every 50 across and 70 up
        let verts = vec![[-100.0, 0.0, 0.0], [100.0, 0.0, 0.0], [100.0, 0.0, 140.0], [-100.0, 0.0, 140.0]];
        let uv = |v: [f64; 3]| [v[0] / 50.0, v[2] / 70.0];
        let tris = vec![[0, 2, 1], [0, 3, 2]];
        let vines = Piece {
            name: "vines".into(),
            label: "Vines".into(),
            kind: "wall".into(),
            materials: vec![PieceMaterial { texture: "kf_vines".into(), tint: [1.0; 3], vertex_colors: false }],
            normals: vec![[0.0, 1.0, 0.0]; 4],
            uvs: tris.iter().map(|t: &[u32; 3]| t.map(|v| uv(verts[v as usize]))).collect(),
            tris,
            mat: vec![0, 0],
            bounds: [[-100.0, 0.0, 0.0], [100.0, 0.0, 140.0]],
            footprint: vec![[-100.0, 0.0], [100.0, 0.0]],
            scale: Scale { min: [0.4, 1.0, 0.1], max: [10.0, 1.0, 50.0], uniform: false },
            tiles: true,
            verts,
            ..Default::default()
        };
        let kit = Kit { pieces: vec![vines], ..Default::default() };
        // the island squared off: its south face is flat and 400 wide
        let mut doc = sample_doc();
        doc.settings.edges = "hard".into();
        doc.regions[1].nodes = vec![vec![-700.0, -500.0], vec![-300.0, -500.0], vec![-300.0, -100.0], vec![-700.0, -100.0]];
        for (z, sx) in [(120.0, 1.0), (900.0, 1.0), (120.0, 1.8)] {
            let mut d = doc.clone();
            d.regions[1].z = z;
            d.props.push(Prop { level: None, piece: "vines".into(), at: [-480.0, -540.0], z: None, yaw: 0.0, scale: [sx, 1.0, 1.0] });
            let lvl = build_with(&d, &th, Some(&kit)).unwrap();
            assert_eq!(lvl.props.len(), 1, "{z} {sx}: {:?}", lvl.problems);
            let o = lvl.mesh.objects.iter().find(|o| o.name == "props").unwrap();
            let top = o.verts.iter().map(|v| v[2]).fold(f64::NEG_INFINITY, f64::max);
            assert!((top - z).abs() < 1e-6, "{z}: reaches {top}");
            let (u0, u1) = o.uvs.iter().flatten().fold((f64::INFINITY, f64::NEG_INFINITY), |a, q| (a.0.min(q[0]), a.1.max(q[0])));
            let v1 = o.uvs.iter().flatten().map(|q| q[1]).fold(f64::NEG_INFINITY, f64::max);
            assert!(((u1 - u0) - 200.0 * sx / 50.0).abs() < 1e-6, "{z} {sx}: u {u0}..{u1}");
            assert!((v1 - z / 70.0).abs() < 1e-6, "{z}: v to {v1}");
            // the editor's ghost is where it was built, and knows the face is 400 wide
            let ground = crate::props::Ground::new(&lvl.mesh);
            let pv = crate::openings::preview(&kit.pieces[0], &d.props[0], &lvl.mesh, &ground).unwrap();
            assert!((pv.prop.at[0] - lvl.props[0].origin[0]).abs() < 1e-6 && pv.prop.scale == lvl.props[0].scale);
            assert!((pv.room - 400.0).abs() < 1.0, "{}", pv.room);
            assert_eq!(pv.tris.len(), 2);
        }
        // wider than the flat face: refused, saying how wide it is
        let mut d = doc.clone();
        d.props.push(Prop { level: None, piece: "vines".into(), at: [-500.0, -540.0], z: None, yaw: 0.0, scale: [2.0, 1.0, 1.0] });
        let lvl = build_with(&d, &th, Some(&kit)).unwrap();
        assert!(lvl.props.is_empty() && lvl.problems.iter().any(|p| p.contains("400 wide here (flat")), "{:?}", lvl.problems);
    }

    #[test]
    fn levels_with_paths_are_watertight_too() {
        let lvl = build(&paths_doc(), &Theme::kokiri()).unwrap();
        let bad = open_edges(&lvl, &["ground", "walls", "cliffs", "bank", "trees"]);
        assert!(bad.is_empty(), "{} open or non-manifold edges, e.g. {:?}", bad.len(), &bad[..bad.len().min(6)]);
    }

    /// The sample level with the island sloping down to the ground, the north plateau terraced
    /// on its three inner edges (its two on the outline face the void: cliffs) and the pond's bed
    /// shelving up to its shore.
    fn profiled_doc() -> Doc {
        let mut doc = sample_doc();
        doc.regions[0].profile = Some(Profile::Terraces { steps: 4, rise: None, depth: 80.0 });
        doc.regions[1].profile = Some(Profile::Slope { angle: 35.0, round: 0.5 });
        doc.regions[2].profile = Some(Profile::Slope { angle: 20.0, round: 0.0 });
        doc
    }

    #[test]
    fn edge_profiles_slope_and_step_and_stay_watertight() {
        let th = Theme::kokiri();
        let plain = build(&sample_doc(), &th).unwrap();
        for (detail, edges) in [("high", "smooth"), ("low", "smooth"), ("low", "hard"), ("high", "faceted")] {
            let mut doc = profiled_doc();
            doc.settings.detail = detail.into();
            doc.settings.edges = edges.into();
            let detail = format!("{detail} {edges}");
            let detail = detail.as_str();
            let lvl = build(&doc, &th).unwrap();
            assert!(lvl.problems.is_empty(), "{detail}: {:?}", lvl.problems);
            let bad = open_edges(&lvl, &["ground", "walls", "cliffs", "bank", "trees"]);
            assert!(bad.is_empty(), "{detail}: {} open or non-manifold edges, e.g. {:?}", bad.len(), &bad[..bad.len().min(6)]);
            let ground = lvl.mesh.objects.iter().find(|o| o.name == "ground").unwrap();
            let inside = |poly: &[P2]| -> Vec<f64> { ground.verts.iter().filter(|v| point_in_poly([v[0], v[1]], poly)).map(|v| v[2]).collect() };
            let polys = Map::build(&doc).unwrap().loop_polys;
            // the island: from the ground at its edge up to 120, with no wall round it
            let zs = inside(&polys[2]);
            assert!(zs.iter().all(|&z| (-1e-6..=120.0 + 1e-6).contains(&z)), "{detail}: island {:?}", zs.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, &z| (a.0.min(z), a.1.max(z))));
            assert!(zs.iter().any(|&z| z > 119.9) && zs.iter().any(|&z| z > 30.0 && z < 90.0));
            let walls = lvl.mesh.objects.iter().find(|o| o.name == "walls").unwrap();
            let near_island = walls.verts.iter().filter(|v| (dist([v[0], v[1]], [-500.0, -300.0]) - 300.0).abs() < 40.0).count();
            assert_eq!(near_island, 0, "{detail}: the island's cliff is gone");
            // the plateau: treads at 40, 80, 120 and its top at 160, risers between them
            for z in [40.0, 80.0, 120.0, 160.0] {
                assert!(inside(&polys[1]).iter().any(|x| (x - z).abs() < 1e-6), "{detail}: a tread at {z}");
            }
            assert!(inside(&polys[1]).iter().all(|x| [0.0, 40.0, 80.0, 120.0, 160.0].iter().any(|z| (x - z).abs() < 1e-6)), "{detail}: treads are flat");
            assert!(walls.verts.iter().any(|v| point_in_poly([v[0], v[1]], &polys[1]) && dist_to_loop([v[0], v[1]], &polys[1]) > 70.0), "{detail}: risers inside");
            // the pond's bed shelves from the shore (0) down to -100
            let bed = inside(&polys[3]);
            assert!(bed.iter().all(|&z| (-100.0 - 1e-6..=1e-6).contains(&z)) && bed.iter().any(|&z| z > -60.0 && z < -40.0), "{detail}: pond");
            if detail == "high smooth" {
                assert!(lvl.mesh.triangles() < 2 * plain.mesh.triangles(), "{} vs {}", lvl.mesh.triangles(), plain.mesh.triangles());
            }
        }
        // a cliff profile is the same as none
        let mut doc = sample_doc();
        doc.regions[1].profile = Some(Profile::Cliff);
        assert_eq!(build(&doc, &th).unwrap().mesh.triangles(), plain.mesh.triangles());
    }

    #[test]
    fn overhangs_undercut_and_ragged_rock_stays_on_its_edges() {
        let th = Theme::kokiri();
        for detail in ["high", "low"] {
            let mut doc = sample_doc();
            doc.settings.detail = detail.into();
            // the island overhangs all round (no ends: no taper); the plateau's inner cliffs are ragged
            doc.regions[1].profile = Some(Profile::Overhang { depth: 50.0 });
            doc.regions[0].profile = Some(Profile::Ragged { amplitude: 20.0, scale: 150.0, seed: 0 });
            let lvl = build(&doc, &th).unwrap();
            assert!(lvl.problems.is_empty(), "{detail}: {:?}", lvl.problems);
            let bad = open_edges(&lvl, &["ground", "walls", "cliffs", "bank", "trees"]);
            assert!(bad.is_empty(), "{detail}: {} open or non-manifold edges, e.g. {:?}", bad.len(), &bad[..bad.len().min(6)]);
            let polys = Map::build(&doc).unwrap().loop_polys;
            let walls = lvl.mesh.objects.iter().find(|o| o.name == "walls").unwrap();
            let ground = lvl.mesh.objects.iter().find(|o| o.name == "ground").unwrap();
            // the island: its cliff's foot 50 in under it, the ground running in to meet it
            let island = &polys[2];
            let under = |v: &P3| point_in_poly([v[0], v[1]], island) && dist_to_loop([v[0], v[1]], island) > 45.0;
            assert!(walls.verts.iter().any(|v| under(v) && v[2] < 1.0), "{detail}: the wall's foot is undercut");
            assert!(ground.verts.iter().any(|v| under(v) && v[2].abs() < 1e-6), "{detail}: the ground runs in under it");
            assert!(walls.verts.iter().filter(|v| (v[2] - 120.0).abs() < 1e-6 && point_in_poly([v[0], v[1]], island)).all(|v| dist_to_loop([v[0], v[1]], island) < 1e-3), "{detail}: the lip is on the edge");
            // the plateau: its face within 20 of the edge, its top and foot exactly on it
            let north = &polys[1];
            let near: Vec<&P3> = walls.verts.iter().filter(|v| dist_to_loop([v[0], v[1]], north) < 40.0 && v[2] > -1e-6 && v[2] < 160.0 + 1e-6).collect();
            let off = |v: &P3| dist_to_loop([v[0], v[1]], north);
            assert!(near.iter().all(|v| off(v) <= 20.0 + 1e-6), "{detail}: within the amplitude");
            assert!(near.iter().any(|v| off(v) > 5.0), "{detail}: the face is bent");
            assert!(near.iter().filter(|v| v[2].abs() < 1e-6 || (v[2] - 160.0).abs() < 1e-6).all(|v| off(v) < 1e-3), "{detail}: top and foot on the edge");
        }
    }

    #[test]
    fn pits_drop_into_a_dark_void() {
        let th = Theme::kokiri();
        let mut doc = sample_doc();
        let ring: Vec<Vec<f64>> = (0..8)
            .map(|k| {
                let a = k as f64 / 8.0 * std::f64::consts::TAU;
                vec![300.0 + 200.0 * a.cos(), 300.0 + 200.0 * a.sin()]
            })
            .collect();
        doc.regions.push(Region { name: "chasm".into(), nodes: ring, z: -600.0, kind: "pit".into(), surface: None, edge: None, noise: None, profile: None, profiles: vec![] });
        let lvl = build(&doc, &th).unwrap();
        assert!(lvl.problems.is_empty(), "{:?}", lvl.problems);
        let bad = open_edges(&lvl, &["ground", "pits", "walls", "cliffs", "bank", "trees"]);
        assert!(bad.is_empty(), "{} open or non-manifold edges, e.g. {:?}", bad.len(), &bad[..bad.len().min(6)]);
        let pits = lvl.mesh.objects.iter().find(|o| o.name == "pits").expect("a pit floor");
        assert!(pits.verts.iter().all(|v| (v[2] + 600.0).abs() < 1e-6));
        let void = lvl.mesh.surfaces.iter().position(|s| s == PIT_SURFACE).unwrap() as i64;
        assert!(pits.surf.iter().all(|&s| s == void), "the void underfoot");
        assert!(pits.colors.iter().all(|c| c.iter().all(|&x| x < 30)), "dark");
        // its walls darken going down: bright at the top, dark at the bottom
        let walls = lvl.mesh.objects.iter().find(|o| o.name == "walls").unwrap();
        let shade = |z: f64| walls.verts.iter().zip(&walls.colors).filter(|(v, _)| (v[2] - z).abs() < 1e-6 && dist([v[0], v[1]], [300.0, 300.0]) < 205.0).map(|(_, c)| c[1] as f64).fold(0.0, f64::max);
        assert!(shade(0.0) > 4.0 * shade(-600.0), "{} vs {}", shade(0.0), shade(-600.0));
        // and the ground has no floor there
        let ground = lvl.mesh.objects.iter().find(|o| o.name == "ground").unwrap();
        assert!(!ground.verts.iter().any(|v| dist([v[0], v[1]], [300.0, 300.0]) < 150.0));
    }

    #[test]
    fn one_edge_slopes_and_the_others_stay_cliffs() {
        // the island's edge from node 2 to 3 only (its nodes are a ring of 7)
        let th = Theme::kokiri();
        let mut doc = sample_doc();
        doc.regions[1].profiles = vec![None, None, Some(Profile::Slope { angle: 30.0, round: 0.0 })];
        let lvl = build(&doc, &th).unwrap();
        assert!(lvl.problems.is_empty(), "{:?}", lvl.problems);
        let bad = open_edges(&lvl, &["ground", "walls", "cliffs", "bank", "trees"]);
        assert!(bad.is_empty(), "{} open or non-manifold edges, e.g. {:?}", bad.len(), &bad[..bad.len().min(6)]);
        let walls = lvl.mesh.objects.iter().find(|o| o.name == "walls").unwrap();
        let ring = &doc.regions[1].nodes;
        let mid = |k: usize| lerp([ring[k][0], ring[k][1]], [ring[k + 1][0], ring[k + 1][1]], 0.5);
        let wall_near = |p: P2| walls.verts.iter().any(|v| dist([v[0], v[1]], p) < 60.0 && v[2] > 100.0);
        assert!(!wall_near(mid(2)), "the sloped edge has no cliff");
        assert!(wall_near(mid(4)) && wall_near(mid(5)), "the others do");
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
                materials: vec![PieceMaterial { texture: "kf_tunnel_mouth".into(), tint: [1.0; 3], vertex_colors: false }],
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
        round.props.push(Prop { level: None, piece: "crawl".into(), at: [-500.0, -640.0], z: None, yaw: 0.0, scale: [1.0; 3] });
        let lvl = build_with(&round, &theme, Some(&kit)).unwrap();
        assert!(lvl.props.is_empty() && lvl.problems.iter().any(|p| p.contains("off flat") || p.contains("parallel")), "{:?}", lvl.problems);
        for (piece, far) in [("log", false), ("crawl", true)] {
            let mut d = doc.clone();
            d.props.push(Prop { level: None, piece: piece.into(), at, z: None, yaw: 0.0, scale: [1.0; 3] });
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
        // near a corner of hard edges: put down 20 from a square corner, the log slides along the
        // wall until it's all on the one face; 20 short of a 25-degree turn, it reaches round it
        // (a long face at low detail, which strays far from the plane beyond the mouth)
        let mut bent = doc.clone();
        bent.regions[1].nodes = vec![vec![-700.0, -500.0], vec![-400.0, -500.0], vec![-218.7, -415.5], vec![-300.0, -100.0], vec![-700.0, -100.0]];
        for (d, at, x) in [(&doc, [-320.0, -540.0], (-700.0, -350.0)), (&bent, [-420.0, -540.0], (-421.0, -419.0))] {
            for detail in ["high", "low"] {
                let mut d = d.clone();
                d.settings.edges = "hard".into();
                d.settings.detail = detail.into();
                d.props.push(Prop { level: None, piece: "log".into(), at, z: None, yaw: 0.0, scale: [1.0; 3] });
                let lvl = build_with(&d, &theme, Some(&kit)).unwrap();
                assert_eq!(lvl.props.len(), 1, "{detail} {at:?}: {:?}", lvl.problems);
                let pl = &lvl.props[0];
                assert!(pl.origin[0] >= x.0 && pl.origin[0] <= x.1, "{detail} {at:?}: at {:?}", pl.origin);
                let f = crate::props::facing(pl.yaw);
                let bad = open_edges(&lvl, &["ground", "walls", "cliffs", "bank", "trees"]);
                let near_mouth = |q: (i64, i64, i64)| {
                    let p = [q.0 as f64 / 100.0, q.1 as f64 / 100.0, q.2 as f64 / 100.0];
                    ((p[0] - pl.origin[0]) * f[1] - (p[1] - pl.origin[1]) * f[0]).abs() <= 31.0 && p[2] - pl.origin[2] <= 51.0
                };
                assert!(!bad.is_empty() && bad.iter().all(|e| near_mouth(e.0) && near_mouth(e.1)), "{detail} {at:?}: {} open edges, e.g. {:?}", bad.len(), &bad[..bad.len().min(4)]);
            }
        }
        // a face too narrow for it is refused, saying so
        let mut narrow = doc.clone();
        narrow.settings.edges = "hard".into();
        narrow.regions[1].nodes = vec![vec![-560.0, -500.0], vec![-500.0, -500.0], vec![-300.0, -100.0], vec![-700.0, -100.0]];
        narrow.props.push(Prop { level: None, piece: "log".into(), at: [-530.0, -540.0], z: None, yaw: 0.0, scale: [1.0; 3] });
        let lvl = build_with(&narrow, &theme, Some(&kit)).unwrap();
        assert!(lvl.props.is_empty() && lvl.problems.iter().any(|p| p.contains("corner to corner")), "{:?}", lvl.problems);
    }

    /// A square plateau 1000 across and `z` high in a square outline 3000 by 2000, and a tunnel
    /// drawn through it from the ground on one side to the ground on the other.
    fn tunnel_doc(z: f64) -> Doc {
        let sq = |x0: f64, y0: f64, x1: f64, y1: f64| vec![vec![x0, y0, 1.0], vec![x1, y0, 1.0], vec![x1, y1, 1.0], vec![x0, y1, 1.0]];
        let mut doc: Doc = serde_json::from_value(serde_json::json!({ "name": "tunnel", "outline": { "nodes": sq(0.0, 0.0, 3000.0, 2000.0) } })).unwrap();
        doc.regions.push(Region { name: "plateau".into(), nodes: sq(1000.0, 500.0, 2000.0, 1500.0), z, kind: "floor".into(), surface: None, edge: None, noise: None, profile: None, profiles: vec![] });
        doc.lines.push(Line { name: "cave".into(), kind: "tunnel".into(), nodes: vec![vec![500.0, 1000.0], vec![1500.0, 1150.0], vec![2500.0, 1000.0]], width: None, closed: false, height: None, noise: None, ..Default::default() });
        doc
    }

    #[test]
    fn tunnels_go_through_walls_and_close_round_their_mouths() {
        let theme = Theme::kokiri();
        let solid = ["ground", "walls", "cliffs", "bank", "trees", "tunnels"];
        let tt = theme.tunnel.clone().unwrap();
        // through a plateau, smooth and rough, at each detail
        for detail in ["high", "low"] {
            for rough in [false, true] {
                let mut doc = tunnel_doc(300.0);
                doc.settings.detail = detail.into();
                if rough {
                    doc.lines[0].noise = Some(Noise { amplitude: 25.0, scale: 300.0, edge: 120.0, seed: 1 });
                }
                let lvl = build(&doc, &theme).unwrap();
                assert!(lvl.problems.is_empty(), "{detail} {rough}: {:?}", lvl.problems);
                let bad = open_edges(&lvl, &solid);
                assert!(bad.is_empty(), "{detail} {rough}: {} open edges, e.g. {:?}", bad.len(), &bad[..bad.len().min(4)]);
                // its mouths are in the plateau's west and east walls, its floor a unit over the ground
                let t = lvl.mesh.objects.iter().find(|o| o.name == "tunnels").unwrap();
                let (x0, x1) = t.verts.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, v| (a.0.min(v[0]), a.1.max(v[0])));
                assert!((x0 - 1000.0).abs() < 1.0 && (x1 - 2000.0).abs() < 1.0, "{detail} {rough}: from x {x0} to {x1}");
                let top = t.verts.iter().map(|v| v[2]).fold(f64::NEG_INFINITY, f64::max);
                assert!(top <= 1.0 + tt.height + if rough { 25.0 } else { 0.0 } + 1e-6, "{detail} {rough}: roof at {top}");
                let floor = t.verts.iter().filter(|v| (v[0] - 1500.0).abs() < 40.0).map(|v| v[2]).fold(f64::INFINITY, f64::min);
                assert!((floor - 1.0).abs() < 1e-6 || rough && floor >= 1.0, "{detail} {rough}: floor at {floor}");
                // dark inside, the ground's grass underfoot
                assert_eq!(t.tints.len(), t.verts.len());
                assert!(t.tints.iter().any(|k| (k[0] - tt.dark).abs() < 1e-6) && t.tints.iter().any(|k| k[0] > 0.95));
                let ground = lvl.mesh.surfaces.iter().position(|s| s == "ground").unwrap() as i64;
                let grass = lvl.mesh.materials.iter().position(|m| m == "ground").unwrap();
                assert!(t.surf.contains(&ground) && t.mat.contains(&grass));
            }
        }
        // from one area's edge of the world to another's (a region drawn outside the outline)
        let mut doc = tunnel_doc(300.0);
        doc.regions[0].nodes = vec![vec![3600.0, 0.0, 1.0], vec![5000.0, 0.0, 1.0], vec![5000.0, 2000.0, 1.0], vec![3600.0, 2000.0, 1.0]];
        doc.regions[0].z = 0.0;
        doc.lines[0].nodes = vec![vec![2500.0, 1000.0], vec![3300.0, 1200.0], vec![4100.0, 1000.0]];
        let lvl = build(&doc, &theme).unwrap();
        assert!(lvl.problems.is_empty(), "{:?}", lvl.problems);
        assert!(open_edges(&lvl, &solid).is_empty());
        let t = lvl.mesh.objects.iter().find(|o| o.name == "tunnels").unwrap();
        let (x0, x1) = t.verts.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, v| (a.0.min(v[0]), a.1.max(v[0])));
        assert!((x0 - 3000.0).abs() < 1.0 && (x1 - 3600.0).abs() < 1.0, "from x {x0} to {x1}");
        // areas closer than twice the bank: their forests would run into each other
        doc.regions[0].nodes = vec![vec![3300.0, 0.0, 1.0], vec![5000.0, 0.0, 1.0], vec![5000.0, 2000.0, 1.0], vec![3300.0, 2000.0, 1.0]];
        let lvl = build(&doc, &theme).unwrap();
        assert!(lvl.problems.iter().any(|p| p.starts_with("two areas' edges of the world run into each other")), "{:?}", lvl.problems);
        // too little ground over it (a dip in the plateau): built, and reported
        let mut doc = tunnel_doc(300.0);
        doc.regions.push(Region { name: "dip".into(), nodes: vec![vec![1350.0, 800.0, 1.0], vec![1650.0, 800.0, 1.0], vec![1650.0, 1400.0, 1.0], vec![1350.0, 1400.0, 1.0]], z: 150.0, kind: "floor".into(), surface: None, edge: None, noise: None, profile: None, profiles: vec![] });
        let lvl = build(&doc, &theme).unwrap();
        assert!(lvl.problems.iter().any(|p| p.starts_with("line 0 (cave): it comes out of the ground")), "{:?}", lvl.problems);
        // a wall too low for it: not built, and why
        let lvl = build(&tunnel_doc(150.0), &theme).unwrap();
        assert!(lvl.mesh.objects.iter().all(|o| o.name != "tunnels"));
        assert!(lvl.problems.iter().any(|p| p.contains("doesn't go into a wall (the tallest it meets from there is 150")), "{:?}", lvl.problems);
        // a node's own height: the floor passes through it
        let mut doc = tunnel_doc(400.0);
        doc.lines[0].nodes[1].push(60.0);
        let lvl = build(&doc, &theme).unwrap();
        assert!(lvl.problems.is_empty(), "{:?}", lvl.problems);
        let t = lvl.mesh.objects.iter().find(|o| o.name == "tunnels").unwrap();
        // (the floor's highest near the node: it rises to it and falls away)
        let high = t.verts.iter().filter(|v| (v[0] - 1500.0).hypot(v[1] - 1150.0) < 130.0 && v[2] < 100.0).map(|v| v[2]).fold(f64::NEG_INFINITY, f64::max);
        assert!((high - 60.0).abs() < 3.0, "floor under the node at {high}");
    }

    /// A dirt path from the ground up a ramp: cut into the floors (points on its rings), drawn
    /// with the blend material by vertex weight, dirt footsteps where it's mostly dirt, and the
    /// level as watertight as before.
    #[test]
    fn dirt_paths_are_cut_into_the_floor() {
        let mut doc = paths_doc();
        doc.lines.push(crate::doc::Line { name: "dirt".into(), kind: "dirt".into(), nodes: vec![vec![1000.0, -1500.0], vec![1000.0, -300.0], vec![1000.0, 900.0]], width: None, closed: false, height: None, noise: None, ..Default::default() });
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

    /// The sample level with a mushroom rock (its cap overhangs), a layered mesa and an arch, all
    /// on the ground (z 0) south and west of the regions.
    fn rocks_doc() -> Doc {
        let mut d = sample_doc();
        let oval = |cx: f64, cy: f64, r: f64| -> Vec<Vec<f64>> {
            (0..9).map(|k| {
                let a = k as f64 / 9.0 * std::f64::consts::TAU;
                vec![cx + r * a.cos(), cy + 0.8 * r * a.sin()]
            }).collect()
        };
        d.lines.push(Line { name: "mushroom".into(), kind: "rock".into(), nodes: oval(300.0, -1000.0, 150.0), contours: vec![Contour::new(150.0, 0.5), Contour::new(260.0, 1.3)], ..Default::default() });
        d.lines.push(Line {
            name: "mesa".into(),
            kind: "rock".into(),
            nodes: oval(-800.0, 500.0, 220.0),
            contours: vec![Contour::new(300.0, 0.85)],
            layers: Some(Layers { height: 100.0, depth: 10.0 }),
            noise: Some(Noise { amplitude: 12.0, scale: 150.0, edge: 0.0, seed: 2 }),
            ..Default::default()
        });
        d.lines.push(Line { name: "arch".into(), kind: "arch".into(), nodes: vec![vec![-1000.0, -700.0], vec![-200.0, -1200.0]], height: Some(450.0), ..Default::default() });
        d
    }

    /// Rocks and arches are closed solids standing on the ground: a rock's foot is the lowest
    /// ground round it and it goes on down into the ground, its flat top at its last contour and
    /// colliding as floor, a contour wider than the one below overhanging; an arch's feet are in
    /// the ground and its crown's top at its height.
    #[test]
    fn rocks_and_arches_are_closed_solids_on_the_ground() {
        let th = Theme::kokiri();
        for (detail, walls) in [("high", "tiled"), ("low", "tiled"), ("high", "stretched"), ("medium", "stretched_middle")] {
            let mut d = rocks_doc();
            d.settings.detail = detail.into();
            d.settings.wall_texture = walls.into();
            let detail = format!("{detail} {walls}");
            let lvl = build(&d, &th).unwrap();
            assert!(lvl.problems.is_empty(), "{detail}: {:?}", lvl.problems);
            let open = open_edges(&lvl, &["rocks", "rock_tops"]);
            assert!(open.is_empty(), "{detail}: {} open edges, e.g. {:?}", open.len(), &open[..open.len().min(4)]);
            let rocks = lvl.mesh.objects.iter().find(|o| o.name == "rocks").unwrap();
            let tops = lvl.mesh.objects.iter().find(|o| o.name == "rock_tops").unwrap();
            let near = |o: &crate::mesh::Object, c: P2, r: f64| -> Vec<P3> { o.verts.iter().copied().filter(|v| dist([v[0], v[1]], c) < r).collect() };
            // the mushroom: down into the ground, its top at 260, its cap wider than its foot
            let m = near(rocks, [300.0, -1000.0], 400.0);
            assert!(m.iter().any(|v| v[2] < -rocks::SINK + 1e-6), "{detail}: it goes into the ground");
            let mt = near(tops, [300.0, -1000.0], 400.0);
            assert!(!mt.is_empty() && mt.iter().all(|v| (v[2] - 260.0).abs() < 1e-6), "{detail}: the mushroom's top");
            let reach = |vs: &[P3]| vs.iter().map(|v| dist([v[0], v[1]], [300.0, -1000.0])).fold(0.0, f64::max);
            let foot: Vec<P3> = m.iter().copied().filter(|v| v[2].abs() < 1e-6).collect();
            assert!(reach(&mt) > 1.15 * reach(&foot), "{detail}: the cap overhangs ({:.0} over {:.0})", reach(&mt), reach(&foot));
            // the mesa's top is at 300 and collides as the ground does
            let ground = lvl.mesh.surfaces.iter().position(|s| s == "ground").unwrap() as i64;
            let on_mesa = |t: &[usize; 3]| t.iter().all(|&v| dist([tops.verts[v][0], tops.verts[v][1]], [-800.0, 500.0]) < 400.0);
            assert!(tops.tris.iter().enumerate().filter(|(_, t)| on_mesa(t)).all(|(k, t)| tops.surf[k] == ground && t.iter().all(|&v| (tops.verts[v][2] - 300.0).abs() < 1e-6)));
            // its grooves cut in: points well inside its foot halfway up a layer
            let mesa = near(rocks, [-800.0, 500.0], 500.0);
            let r_at = |z: f64| mesa.iter().filter(|v| (v[2] - z).abs() < 1e-6).map(|v| dist([v[0], v[1]], [-800.0, 500.0])).sum::<f64>();
            assert!(r_at(50.0) < r_at(100.0), "{detail}: a groove halfway up the first layer");
            // the rocks' sides keep within the clamped cliff texture
            let cliff = lvl.mesh.materials.iter().position(|m| m == "cliff").unwrap();
            let on_rock = |t: &[usize; 3]| t.iter().all(|&v| [[300.0, -1000.0], [-800.0, 500.0]].iter().any(|&c| dist([rocks.verts[v][0], rocks.verts[v][1]], c) < 400.0));
            for (k, uv) in rocks.uvs.iter().enumerate() {
                if rocks.mat[k] == cliff && on_rock(&rocks.tris[k]) {
                    assert!(uv.iter().all(|q| q[1] > -1e-6 && q[1] < 1.0 + 1e-6), "{detail}: v {uv:?}");
                    // stretched, the texture is once over the side: its top at the top, its foot at the foot
                    if walls == "stretched" {
                        for (c, q) in rocks.tris[k].iter().zip(uv) {
                            let z = rocks.verts[*c][2];
                            if (z - 260.0).abs() < 1e-6 || (z - 300.0).abs() < 1e-6 {
                                assert!((q[1] - 1.0).abs() < 1e-6, "{detail}: v {} at the top", q[1]);
                            } else if z.abs() < 1e-6 {
                                assert!(q[1].abs() < 1e-6, "{detail}: v {} at the foot", q[1]);
                            }
                        }
                    }
                }
            }
            // the arch: feet in the ground, the crown's top at 450 over it
            let a = near(rocks, [-600.0, -950.0], 900.0);
            assert!(a.iter().any(|v| v[2] < -rocks::SINK + 1e-6), "{detail}: its feet are in the ground");
            let crown = near(tops, [-600.0, -950.0], 900.0).iter().map(|v| v[2]).fold(f64::NEG_INFINITY, f64::max);
            assert!((crown - 450.0).abs() < 2.0, "{detail}: the crown at {crown:.1}");
        }
        // lower detail is lighter
        let tris = |detail: &str| {
            let mut d = rocks_doc();
            d.settings.detail = detail.into();
            let lvl = build(&d, &th).unwrap();
            lvl.mesh.objects.iter().filter(|o| o.name.starts_with("rock")).map(|o| o.tris.len()).sum::<usize>()
        };
        assert!(tris("low") * 5 < tris("high") * 3, "{} {}", tris("low"), tris("high"));
    }

    /// An arch's own segments set how many pieces it's built of along it (its top a quad each),
    /// keeping one at its crown; with none, `arch_lengths` and `arch_split` say how many it gets.
    #[test]
    fn arches_take_their_segments() {
        let th = Theme::kokiri();
        let nodes = vec![vec![-1000.0, -700.0], vec![-200.0, -1200.0]];
        for segments in [Some(6), Some(40), None] {
            let mut d = sample_doc();
            d.lines.push(Line { name: "arch".into(), kind: "arch".into(), nodes: nodes.clone(), height: Some(450.0), segments, ..Default::default() });
            let lvl = build(&d, &th).unwrap();
            assert!(lvl.problems.is_empty(), "{segments:?}: {:?}", lvl.problems);
            assert!(open_edges(&lvl, &["rocks", "rock_tops"]).is_empty(), "{segments:?}: closed");
            let tops = lvl.mesh.objects.iter().find(|o| o.name == "rock_tops").unwrap();
            let want = segments.map_or_else(
                || {
                    let plan: Vec<P2> = nodes.iter().map(|n| [n[0], n[1]]).collect();
                    let (a_top, total) = rocks::arch_lengths(&plan, [0.0, 0.0], 450.0);
                    let (m0, m1) = rocks::arch_split(a_top, total, rocks::arch_step(&d.settings), None);
                    m0 + m1
                },
                |n| n as usize,
            );
            assert_eq!(tops.tris.len(), 2 * want, "{segments:?}");
            let crown = tops.verts.iter().map(|v| v[2]).fold(f64::NEG_INFINITY, f64::max);
            assert!((crown - 450.0).abs() < 1.0, "{segments:?}: crown {crown:.1}");
        }
    }

    /// What can't be built is reported: contours out of order, an arch too short for its width or
    /// too thick for its height, a rock off the ground.
    #[test]
    fn rocks_and_arches_report_what_they_cant_do() {
        let th = Theme::kokiri();
        let problems = |l: Line| {
            let mut d = sample_doc();
            d.lines.push(l);
            build(&d, &th).unwrap().problems
        };
        let square = |cx: f64, cy: f64| vec![vec![cx - 100.0, cy - 100.0], vec![cx + 100.0, cy - 100.0], vec![cx + 100.0, cy + 100.0], vec![cx - 100.0, cy + 100.0]];
        let p = problems(Line { name: "r".into(), kind: "rock".into(), nodes: square(300.0, -1000.0), contours: vec![Contour::new(200.0, 0.8), Contour::new(150.0, 0.5)], ..Default::default() });
        assert!(p.iter().any(|p| p.contains("isn't above the one below")), "{p:?}");
        let p = problems(Line { name: "r".into(), kind: "rock".into(), nodes: square(9000.0, 0.0), ..Default::default() });
        assert!(p.iter().any(|p| p.contains("isn't on the ground")), "{p:?}");
        let p = problems(Line { name: "a".into(), kind: "arch".into(), nodes: vec![vec![0.0, -1000.0], vec![100.0, -1000.0]], ..Default::default() });
        assert!(p.iter().any(|p| p.contains("less than its width")), "{p:?}");
        let p = problems(Line { name: "a".into(), kind: "arch".into(), nodes: vec![vec![-600.0, -1000.0], vec![400.0, -1000.0]], height: Some(120.0), ..Default::default() });
        assert!(p.iter().any(|p| p.contains("no room under it")), "{p:?}");
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
    fn themes_switch_and_mix_and_stay_watertight() {
        let wall_mats = |lvl: &Level| -> std::collections::BTreeSet<String> {
            let walls = lvl.mesh.objects.iter().find(|o| o.name == "walls").unwrap();
            walls.mat.iter().map(|&m| lvl.mesh.materials[m].clone()).collect()
        };
        let mut doc = paths_doc();
        doc.settings.theme = "kakariko".into();
        let th = Theme::for_doc(&doc, None).unwrap();
        assert_eq!(th.name, "kakariko");
        let lvl = build(&doc, &th).unwrap();
        assert!(lvl.problems.is_empty(), "{:?}", lvl.problems);
        let bad = open_edges_under(&lvl, &["ground", "walls", "cliffs", "bank", "trees"], th.trees.trunks);
        assert!(bad.is_empty(), "{} open or non-manifold edges, e.g. {:?}", bad.len(), &bad[..bad.len().min(6)]);
        // the 240 plateaus' walls are Kakariko's brick; its ground and cliffs its own
        assert!(wall_mats(&lvl).contains("brick"), "{:?}", wall_mats(&lvl));
        assert_eq!(th.texture_name("brick"), "kak_brick");
        assert_eq!(th.texture_name(&th.floor.material), "kak_ground");
        assert_eq!(th.texture_name("cliff~mid"), crate::textures::derived_name("kak_cliff", 3, 30, true));
        // a region pinned to Kokiri's cliff keeps it under Kakariko, its middle rows Kokiri's
        doc.regions[0].edge = Some("kokiri:cliff".into());
        doc.settings.detail = "low".into();
        let lvl = build(&doc, &th).unwrap();
        assert!(lvl.problems.is_empty(), "{:?}", lvl.problems);
        let mats = wall_mats(&lvl);
        assert!(mats.contains("kokiri:cliff") || mats.contains("kokiri:cliff~mid"), "{mats:?}");
        assert_eq!(th.texture_name("kokiri:cliff"), "kf_cliff");
        assert_eq!(th.texture_name("kokiri:cliff~mid"), Theme::kokiri().texture_name("cliff~mid"));
        // pinned to the level's own theme, it's the plain style
        assert_eq!(th.wall_styles["kakariko:cliff"].material, "kakariko:cliff");
        assert_eq!(th.texture_name("kakariko:cliff"), "kak_cliff");
        // a style the theme doesn't have falls back to the theme's walls, and says so
        doc.settings.theme = "kokiri".into();
        doc.regions[0].edge = Some("brick".into());
        doc.paths[0].edge = Some("rock".into());
        let th = Theme::for_doc(&doc, None).unwrap();
        let lvl = build(&doc, &th).unwrap();
        assert_eq!(lvl.problems.len(), 2, "{:?}", lvl.problems);
        assert!(lvl.problems[0].contains("no wall style \"brick\""), "{:?}", lvl.problems);
        assert!(!wall_mats(&lvl).contains("brick"));
        // the editor lists the level's theme's styles first, then the others' pinned
        let names = th.style_names();
        let first_pinned = names.iter().position(|n| n.contains(':')).unwrap();
        assert!(names[..first_pinned].contains(&"cliff".to_string()) && names[first_pinned..].contains(&"kakariko:brick".to_string()));
        assert!(!names.iter().any(|n| n.starts_with("kokiri:")), "the level's own theme isn't listed twice");
    }

    #[test]
    fn stacked_edges_build_walls_and_slopes_and_stay_watertight() {
        // the sample level in Kakariko's look: the island a cliff with a grass slope above it, the
        // north plateau (raised to 700) a brick wall, a rock wall a ledge behind it, then steep rock
        let mut doc = sample_doc();
        doc.settings.theme = "kakariko".into();
        doc.regions[0].z = 700.0;
        doc.regions[1].z = 500.0;
        doc.regions[1].profile = Some(Profile::Stack { parts: vec![Part::wall(Some(200.0), None), Part::slope(55.0, None, None)] });
        doc.regions[0].profile = Some(Profile::Stack {
            parts: vec![Part::wall(Some(200.0), Some("brick")), Part::wall(Some(120.0), Some("rock")), Part::slope(69.0, Some(200.0), Some("rock")), Part::slope(40.0, None, None)],
        });
        let th = Theme::for_doc(&doc, None).unwrap();
        for detail in ["high", "low"] {
            doc.settings.detail = detail.into();
            let lvl = build(&doc, &th).unwrap();
            let steep: Vec<_> = lvl.problems.iter().filter(|p| !p.contains("degrees")).collect();
            assert!(steep.is_empty(), "{detail}: {:?}", lvl.problems);
            let bad = open_edges_under(&lvl, &["ground", "walls", "cliffs", "bank", "trees"], th.trees.trunks);
            assert!(bad.is_empty(), "{detail}: {} open or non-manifold edges, e.g. {:?}", bad.len(), &bad[..bad.len().min(6)]);
            let mats = |name: &str| -> std::collections::BTreeSet<String> {
                let o = lvl.mesh.objects.iter().find(|o| o.name == name).unwrap();
                o.mat.iter().map(|&m| lvl.mesh.materials[m].clone()).collect()
            };
            // the plateau's brick and rock walls; its steep rock drawn as rock on the ground
            let walls = mats("walls");
            assert!(walls.contains("brick") || walls.contains("brick~mid"), "{detail}: {walls:?}");
            assert!(walls.contains("rock"), "{detail}: {walls:?}");
            assert!(mats("ground").contains("rock"), "{detail}: {:?}", mats("ground"));
            // the island's edge stands 200 up from the ground beside it, then slopes to its top
            let g = lvl.mesh.objects.iter().find(|o| o.name == "ground").unwrap();
            let island_top = g.verts.iter().filter(|v| dist([v[0], v[1]], [-500.0, -300.0]) < 300.0).map(|v| v[2]).fold(f64::NEG_INFINITY, f64::max);
            assert!((island_top - 500.0).abs() < 1e-6, "{detail}: {island_top}");
            let lowest = g.verts.iter().filter(|v| dist([v[0], v[1]], [-500.0, -300.0]) < 260.0).map(|v| v[2]).fold(f64::INFINITY, f64::min);
            assert!(lowest > 199.0, "{detail}: the island's floor starts 200 up at its edge: {lowest}");
        }
    }

    #[test]
    fn beyond_the_outline_ground_climbs_to_a_crest_and_the_forest_stands_only_where_it_should() {
        // the paths level (a square 4000 across) in Kakariko's look: beyond its north edge a cliff
        // and a grass slope up to 900, beyond its east edge the mossy wall up to 1200; forest on
        // the other two
        let mut doc = paths_doc();
        doc.settings.theme = "kakariko".into();
        let west_wing = Profile::Stack { parts: vec![Part::wall(Some(330.0), Some("cliff")), Part::slope(36.0, None, None)] };
        let mossy = Profile::Stack { parts: vec![Part::wall(Some(330.0), Some("cliff")), Part::slope(36.0, Some(75.0), None), Part::slope(76.0, None, Some("mountain"))] };
        // the outline runs (-2000, -2000), (2000, -2000), (2000, 2000), (-2000, 2000): edge 1 east, 2 north
        doc.outline.beyond = vec![None, Some(Beyond::new(1200.0, mossy)), Some(Beyond::new(900.0, west_wing)), None];
        let th = Theme::for_doc(&doc, None).unwrap();
        for detail in ["high", "low"] {
            doc.settings.detail = detail.into();
            let lvl = build(&doc, &th).unwrap();
            let problems: Vec<_> = lvl.problems.iter().filter(|p| !p.contains("degrees")).collect();
            assert!(problems.is_empty(), "{detail}: {problems:?}");
            let obj = |n: &str| lvl.mesh.objects.iter().find(|o| o.name == n).unwrap();
            // the ground beyond the north edge climbs to its crest at 900 (the east's to 1200), and
            // the level's own floor along the edges is where it was
            let g = obj("ground");
            let top = |f: &dyn Fn(&P3) -> bool| g.verts.iter().filter(|v| f(v)).map(|v| v[2]).fold(f64::NEG_INFINITY, f64::max);
            assert!((top(&|v| v[1] > 2000.5 && v[0] < 1500.0) - 900.0).abs() < 1e-6, "{detail}: {}", top(&|v| v[1] > 2000.5 && v[0] < 1500.0));
            // the north band's west end (the forest beside it) slopes down to the forest's cliff
            // tops, and is the band's full depth there, square to its edge
            let foot = doc.outline.z + doc.boundary.cliff_min;
            let end: Vec<&P3> = g.verts.iter().filter(|v| (v[0] + 2000.0).abs() < 1e-6 && v[1] > 2000.5).collect();
            assert!(!end.is_empty() && end.iter().all(|v| (v[2] - foot).abs() < 1e-6), "{detail}: the west end {end:?}");
            assert!(end.iter().any(|v| v[1] > 2000.0 + crate::profiles::reach(&doc.outline.beyond[2].as_ref().unwrap().profile, 900.0)), "{detail}: the band's full depth");
            assert!((top(&|v| v[0] > 2000.5 && v[1] < 1500.0) - 1200.0).abs() < 1e-6, "{detail}");
            assert!(g.verts.iter().filter(|v| (v[1] - 1999.0).abs() < 1.0 && v[0].abs() < 1500.0).all(|v| v[2].abs() < 1e-6), "{detail}: the floor inside the north edge");
            // the forest stands along the south and west, not past the bands
            for name in ["trees", "foliage"] {
                assert!(obj(name).verts.iter().all(|v| v[1] < 2000.0 + 1.0 && v[0] < 2000.0 + 1.0), "{detail}: {name} beyond a band");
            }
            assert!(obj("trees").verts.iter().any(|v| v[1] < -2100.0), "{detail}: trees along the south");
            // and along the whole west edge, up to the north band's end
            assert!(obj("trees").verts.iter().any(|v| v[0] < -2000.0 && v[1] > 1990.0), "{detail}: trees up to the band's end");
            // the forest's rim isn't raised by the bands' crests
            assert!(lvl.rim.1 < 400.0, "{detail}: rim {:?}", lvl.rim);
            // watertight but for the bands' far edges (the level ends there) and the forest's ends
            let (expanded, bands) = crate::beyond::expand(&doc).unwrap();
            let far: Vec<(P2, P2)> = bands
                .far
                .iter()
                .enumerate()
                .flat_map(|(j, es)| {
                    let r = &expanded.regions[bands.first + j];
                    let n = r.nodes.len();
                    es.iter().map(move |&k| ([r.nodes[k][0], r.nodes[k][1]], [r.nodes[(k + 1) % n][0], r.nodes[(k + 1) % n][1]])).collect::<Vec<_>>()
                })
                .collect();
            let inside = |x: i64, y: i64| x.abs() <= 200_001 && y.abs() <= 200_001;
            let bad: Vec<_> = open_edges_under(&lvl, &["ground", "walls", "cliffs", "bank", "trees"], th.trees.trunks)
                .into_iter()
                .filter(|(a, b)| {
                    let m = [(a.0 + b.0) as f64 / 200.0, (a.1 + b.1) as f64 / 200.0];
                    // the bands' far edges
                    !far.iter().any(|&(p, q)| dist_to_seg(m, p, q).0 < 1.0)
                        // the bands' side faces' feet, under the forest's bank
                        && !(a.2 <= 0 && b.2 <= 0 && !(inside(a.0, a.1) && inside(b.0, b.1)))
                        // where a band, its side face and the forest's cliff meet (three walls at a corner)
                        && !(a.0 == b.0 && a.1 == b.1 && a.0.abs() == 200_000 && a.1.abs() == 200_000)
                })
                .collect();
            // the cliffs' tops meet the bank only where the forest is; elsewhere (the band's side
            // faces' bottoms, the forest cliff's ends) edges are open by design: just few
            // and the forest's two ends: each its bank's end, its trunks' end and the cliff top's last piece
            assert!(bad.len() <= 6, "{detail}: {} open edges, e.g. {:?}", bad.len(), &bad[..bad.len().min(8)]);
            let ends = bad.iter().filter(|(a, b)| a.0 == b.0 && a.1 == b.1).count();
            assert_eq!(ends, 2, "{detail}: the trunks' two ends: {bad:?}");
        }
    }

    #[test]
    fn kakariko_looks_rough_ridges_and_a_mossy_skyline_wall() {
        // the paths level on Kokiri's theme: beyond its north edge Kakariko's rock face, beyond its
        // east edge the mossy wall (both in Kakariko's styles, pinned)
        let mut doc = paths_doc();
        let looks = crate::doc::stack_looks();
        let (rock, mossy) = (looks.iter().find(|l| l.name == "Rock face").unwrap(), looks.iter().find(|l| l.name == "Mossy wall").unwrap());
        doc.outline.beyond = vec![None, Some(mossy.beyond(0.0)), Some(rock.beyond(0.0)), None];
        let th = Theme::for_doc(&doc, None).unwrap();
        for detail in ["high", "low"] {
            doc.settings.detail = detail.into();
            let lvl = build(&doc, &th).unwrap();
            let problems: Vec<_> = lvl.problems.iter().filter(|p| !p.contains("degrees")).collect();
            assert!(problems.is_empty(), "{detail}: {problems:?}");
            let obj = |n: &str| lvl.mesh.objects.iter().find(|o| o.name == n).unwrap();
            // the rock's ridge rises and falls: along its crest (far from its ends), the floor's
            // highest points differ by a few hundred
            let g = obj("ground");
            let crest: Vec<f64> = (0..6)
                .map(|k| {
                    let x = -1200.0 + 400.0 * k as f64;
                    g.verts.iter().filter(|v| v[1] > 2300.0 && (v[0] - x).abs() < 200.0).map(|v| v[2]).fold(f64::NEG_INFINITY, f64::max)
                })
                .collect();
            let (lo, hi) = crest.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &z| (a.min(z), b.max(z)));
            assert!(lo > 700.0 && hi - lo > 100.0, "{detail}: the ridge {crest:?}");
            // the level's floor beside it stays where it was
            assert!(g.verts.iter().filter(|v| (v[1] - 1999.0).abs() < 1.0 && v[0].abs() < 1500.0).all(|v| v[2].abs() < 1e-6), "{detail}: the floor inside");
            // the mossy wall stands past the east band's ledge, its top well above it, coming down
            // towards the band's south end (the forest beside it), not its north (the rock beside)
            let wall: Vec<&P3> = obj("cliffs").verts.iter().filter(|v| v[0] > 2100.0).collect();
            let top = |f: &dyn Fn(&P3) -> bool| wall.iter().filter(|v| f(v)).map(|v| v[2]).fold(f64::NEG_INFINITY, f64::max);
            let (south, middle) = (top(&|v| v[1] < -1900.0), top(&|v| v[1].abs() < 600.0));
            assert!(middle > 405.0 + 300.0, "{detail}: the mossy wall's top {middle}");
            assert!(south < middle - 200.0, "{detail}: it comes down at its end: {south} vs {middle}");
        }
    }

    #[test]
    fn stairs_are_a_ramp_with_steps_drawn_on_it() {
        // the paths level's ramp as Kakariko's stairs, in a Kokiri level (borrowing Kakariko's)
        let mut doc = paths_doc();
        doc.paths[0].look = Some("steps".into());
        let th = Theme::for_doc(&doc, None).unwrap();
        assert_eq!(th.steps.as_ref().map(|s| s.tread.as_str()), Some("kakariko:steps"));
        assert_eq!(th.texture_name("kakariko:steps"), "kak_steps");
        let lvl = build(&doc, &th).unwrap();
        assert!(lvl.problems.is_empty(), "{:?}", lvl.problems);
        let bad = open_edges(&lvl, &["ground", "walls", "cliffs", "bank", "trees"]);
        assert!(bad.is_empty(), "{} open edges, e.g. {:?}", bad.len(), &bad[..bad.len().min(6)]);
        let mats = &lvl.mesh.materials;
        let st = th.steps.as_ref().unwrap();
        // the tread: twice across, one step every 22.4 along its surface
        let g = lvl.mesh.objects.iter().find(|o| o.name == "ground").unwrap();
        let treads: Vec<usize> = (0..g.tris.len()).filter(|&t| mats[g.mat[t]] == st.tread).collect();
        assert!(!treads.is_empty());
        let (mut lo, mut hi) = ((f64::INFINITY, 0.0), (f64::NEG_INFINITY, 0.0));
        for &t in &treads {
            for k in 0..3 {
                let (v, uv) = (g.verts[g.tris[t][k]], g.uvs[t][k]);
                assert!((-1e-6..=2.0 + 1e-6).contains(&uv[0]), "u {}", uv[0]);
                if v[2] < lo.0 {
                    lo = (v[2], uv[1]);
                }
                if v[2] > hi.0 {
                    hi = (v[2], uv[1]);
                }
            }
        }
        // the ramp climbs from the ground to the plateau's 240 over 1200: its surface is about
        // 1224 long, 55 steps
        let steps = (hi.1 - lo.1).abs();
        assert!((hi.0 - lo.0 - 240.0).abs() < 1e-6, "{lo:?} {hi:?}");
        assert!((steps - 1200f64.hypot(240.0) / 22.4).abs() < 0.5, "{steps} steps");
        // an 11 degree stair's sides are plain brick, repeating along them
        let walls = |lvl: &Level| -> std::collections::BTreeSet<String> {
            let w = lvl.mesh.objects.iter().find(|o| o.name == "walls").unwrap();
            w.mat.iter().map(|&m| lvl.mesh.materials[m].clone()).collect()
        };
        let (side, tiled) = (&th.wall_styles[&st.side].material, &th.wall_styles[st.tiled.as_ref().unwrap()].material);
        assert!(walls(&lvl).contains(tiled) && !walls(&lvl).contains(side), "{:?}", walls(&lvl));
        // its bricks one size all over: v up from the stair's foot, the same band everywhere (a
        // wall's own v is per column, stretched where the side's height runs to nothing)
        let ws = &th.wall_styles[st.tiled.as_ref().unwrap()];
        let band = ws.band.unwrap();
        let w = lvl.mesh.objects.iter().find(|o| o.name == "walls").unwrap();
        let offs: Vec<f64> = (0..w.tris.len())
            .filter(|&t| &lvl.mesh.materials[w.mat[t]] == tiled)
            .flat_map(|t| (0..3).map(move |k| (t, k)))
            .map(|(t, k)| w.verts[w.tris[t][k]][2] - w.uvs[t][k][1] * band)
            .collect();
        assert!(offs.iter().all(|o| (o - offs[0]).abs() < 1e-6), "{:?}", &offs[..offs.len().min(8)]);
        // one of Kakariko's slope (240 up over 480) has the stairs' profile on its sides, the
        // texture once over the stair (u and v within 0 to 1)
        doc.paths[0].nodes[0] = vec![Some(1000.0), Some(420.0)];
        let lvl = build(&doc, &th).unwrap();
        assert!(lvl.problems.is_empty(), "{:?}", lvl.problems);
        let w = lvl.mesh.objects.iter().find(|o| o.name == "walls").unwrap();
        let mats = &lvl.mesh.materials;
        let sides: Vec<usize> = (0..w.tris.len()).filter(|&t| &mats[w.mat[t]] == side).collect();
        assert!(!sides.is_empty() && !walls(&lvl).contains(tiled), "{:?}", walls(&lvl));
        // where it cuts into the plateau, the walls above it are brick with the grass on top
        // (top-anchored), not its sides' profile
        let cutting = &th.wall_styles[st.cutting.as_ref().unwrap()];
        assert_eq!(cutting.caps.as_ref().map(|c| c.anchor.as_str()), Some("top"));
        assert!(walls(&lvl).contains(&cutting.material), "{:?}", walls(&lvl));
        // every profile triangle is under the stairs' surface (240 up over y 420 to 900): sides, not cuttings
        for &t in &sides {
            let c = w.tris[t].map(|v| w.verts[v]);
            let (y, z) = ((c[0][1] + c[1][1] + c[2][1]) / 3.0, (c[0][2] + c[1][2] + c[2][2]) / 3.0);
            assert!(z <= 240.0 * ((y - 420.0) / 480.0).clamp(0.0, 1.0) + 1.0, "a profile triangle above the stairs at y {y:.0}, z {z:.0}");
        }
        for &t in &sides {
            for uv in w.uvs[t] {
                assert!((-1e-6..=1.0 + 1e-6).contains(&uv[0]) && uv[1] <= 1.0 + 1e-6, "{uv:?}");
            }
        }
        // an unknown look is refused
        doc.paths[0].look = Some("cobbles".into());
        assert!(build(&doc, &th).is_err());
    }

    /// The paths level with a path of each section across the south: a causeway with railings, a
    /// sunken lane, a boardwalk with railings, and a ledge climbing the east plateau's west edge
    /// (the plateau's walls in the `ledge` style, so they tell from an embankment's).
    fn sections_doc() -> Doc {
        let mut doc = paths_doc();
        doc.regions[0].edge = Some("ledge".into());
        let n = |x: f64, y: f64| vec![Some(x), Some(y)];
        let path = |name: &str, nodes: Vec<Vec<Option<f64>>>, section: Section, railings: bool| Path {
            name: name.into(),
            nodes,
            section: Some(section),
            railings: railings.then(|| "fence".into()),
            ..Default::default()
        };
        doc.paths.push(path("causeway", vec![n(-1700.0, -1600.0), n(-300.0, -1600.0)], Section::Causeway { height: 60.0 }, true));
        doc.paths.push(path("lane", vec![n(-1700.0, -1100.0), n(-300.0, -1100.0)], Section::Sunken { depth: 120.0 }, true));
        doc.paths.push(path("walk", vec![n(100.0, -1600.0), n(1500.0, -1600.0)], Section::Boardwalk { height: 80.0, spacing: 200.0 }, true));
        doc.paths.push(path("ledge", vec![n(600.0, 300.0), vec![Some(600.0), Some(1300.0), Some(200.0)]], Section::Ledge, false));
        doc
    }

    #[test]
    fn sections_raise_sink_cut_ledges_and_build_boardwalks() {
        let lvl = build(&sections_doc(), &Theme::kokiri()).unwrap();
        assert!(lvl.problems.is_empty(), "{:?}", lvl.problems);
        let bad = open_edges(&lvl, &["ground", "walls", "cliffs", "bank", "trees"]);
        assert!(bad.is_empty(), "{} open or non-manifold edges, e.g. {:?}", bad.len(), &bad[..bad.len().min(6)]);
        let obj = |n: &str| lvl.mesh.objects.iter().find(|o| o.name == n).unwrap_or_else(|| panic!("no {n}"));
        let ground = obj("ground");
        let at = |x0: f64, x1: f64, y: f64| -> Vec<f64> { ground.verts.iter().filter(|v| v[0] > x0 && v[0] < x1 && (v[1] - y).abs() < 81.0).map(|v| v[2]).collect() };
        // the causeway's top 60 up in the middle, its ends ramping down to the ground at 25 degrees
        let mid = at(-1100.0, -900.0, -1600.0);
        // (the ground beside shares the sides' points, at 0)
        assert!(mid.iter().any(|&z| (z - 60.0).abs() < 1e-6) && mid.iter().all(|&z| (z - 60.0).abs() < 1e-6 || z.abs() < 1e-6), "{mid:?}");
        let ramp = 60.0 / crate::paths::RAMP.to_radians().tan();
        let foot = at(-1700.0 + 0.5 * ramp - 20.0, -1700.0 + 0.5 * ramp + 20.0, -1600.0);
        assert!(foot.iter().any(|&z| z > 10.0 && z < 50.0), "half way up its ramp: {foot:?}");
        // the lane 120 down, walled both sides
        let lane = at(-1100.0, -900.0, -1100.0);
        assert!(lane.iter().any(|&z| (z + 120.0).abs() < 1e-6) && lane.iter().all(|&z| (z + 120.0).abs() < 1e-6 || z.abs() < 1e-6), "{lane:?}");
        // the ledge's walls are the plateau's own (`ledge`, cliff_strip), not an embankment's (cliff)
        let mats = &lvl.mesh.materials;
        let walls = obj("walls");
        let by_ledge: std::collections::BTreeSet<&str> = walls
            .tris
            .iter()
            .zip(&walls.mat)
            .filter(|(t, _)| t.iter().all(|&v| (walls.verts[v][0] - 600.0).abs() < 90.0 && walls.verts[v][1] > 700.0 && walls.verts[v][1] < 1200.0))
            .map(|(_, &m)| mats[m].as_str())
            .collect();
        assert!(by_ledge.contains("cliff_strip") && !by_ledge.iter().any(|m| m.starts_with("cliff~") || *m == "cliff"), "{by_ledge:?}");
        // the boardwalk: planks 80 up in the middle, on posts down into the ground
        let br = obj("bridges");
        let deck = mats.iter().position(|m| m == "log_side").expect("planks");
        let tops: Vec<f64> = br.tris.iter().zip(&br.mat).filter(|(_, m)| **m == deck).flat_map(|(t, _)| t.map(|v| br.verts[v])).filter(|v| (v[0] - 800.0).abs() < 100.0).map(|v| v[2]).collect();
        assert!(!tops.is_empty() && tops.iter().all(|&z| (z - 80.0).abs() < 1e-6), "{tops:?}");
        let posts = obj("bridges_collision");
        assert!(posts.verts.iter().any(|v| v[2] < -10.0) && posts.verts.iter().any(|v| (v[2] - 68.0).abs() < 1e-6));
        // railings: along the causeway and the boardwalk, on their tops; none down in the lane
        let rails = obj("fences");
        let spans = |x: f64, z: f64| {
            let near: Vec<&P3> = rails.verts.iter().filter(|v| (v[1] + 1600.0).abs() < 90.0 && (v[2] - z).abs() < 1e-6).collect();
            near.iter().any(|v| v[0] < x - 300.0) && near.iter().any(|v| v[0] > x + 300.0)
        };
        assert!(spans(-1000.0, 60.0), "along the causeway's top");
        assert!(spans(800.0, 80.0), "along the boardwalk's deck");
        assert!(!rails.verts.iter().any(|v| (v[1] + 1100.0).abs() < 90.0), "the lane's sides go up, not down");
    }

    /// Flat ground with a main path climbing 0 to 300 west to east, a branch from the south ending on
    /// its middle (a T), a path on from its east end north (an L) and one crossing it at x -800 at
    /// its own height there (a crossroads).
    fn junctions_doc() -> Doc {
        let mut doc = paths_doc();
        doc.regions.clear();
        let n = |x: f64, y: f64| vec![Some(x), Some(y)];
        let path = |name: &str, nodes: Vec<Vec<Option<f64>>>| Path { name: name.into(), nodes, ..Default::default() };
        doc.paths = vec![
            Path { look: Some("dirt".into()), ..path("main", vec![vec![Some(-1500.0), Some(0.0), Some(0.0)], vec![Some(1500.0), Some(0.0), Some(300.0)]]) },
            path("branch", vec![n(0.0, -1200.0), n(0.0, 0.0)]),
            path("corner", vec![n(1500.0, 0.0), n(1500.0, 1300.0)]),
            path("cross", vec![n(-800.0, -1200.0), vec![Some(-800.0), Some(0.0), Some(70.0)], n(-800.0, 1200.0)]),
        ];
        doc
    }

    #[test]
    fn paths_join_without_steps() {
        let doc = junctions_doc();
        let geos: Vec<PathGeo> = paths::layout_all(&doc.paths, &|_| 0.0, Sampling::Every(60.0)).into_iter().map(Result::unwrap).collect();
        // the branch ends at main's height there, the corner starts at main's end height
        assert!((geos[1].st.last().unwrap().z - 150.0).abs() < 1.0, "{}", geos[1].st.last().unwrap().z);
        assert!((geos[2].z_at([1500.0, 100.0]) - 300.0).abs() < 30.0);
        assert_eq!(geos[1].into, vec![0]);
        assert!(geos[2].into == vec![0] && geos[0].into.is_empty(), "the later of two ends meeting runs into the earlier");
        // the two ends meeting run on past their node, so the outer corner is covered
        assert!(geos[2].st[0].p[1] < -78.0 && geos[0].st.last().unwrap().p[0] > 1578.0);
        let bl = paths::blends(&geos);
        assert!(bl.contains(&(0, 1)) && bl.contains(&(0, 2)) && bl.contains(&(0, 3)), "{bl:?}");

        let lvl = build(&doc, &Theme::kokiri()).unwrap();
        assert!(lvl.problems.is_empty(), "{:?}", lvl.problems);
        let bad = open_edges(&lvl, &["ground", "walls", "cliffs", "bank", "trees"]);
        assert!(bad.is_empty(), "{} open or non-manifold edges, e.g. {:?}", bad.len(), &bad[..bad.len().min(6)]);
        // no wall at any junction: walls stand only along the paths' outer sides, never across
        // where they meet
        let walls = lvl.mesh.objects.iter().find(|o| o.name == "walls").unwrap();
        for (c, r) in [([0.0, 0.0], 79.0), ([1500.0, 0.0], 79.0), ([-800.0, 0.0], 79.0)] {
            let inside: Vec<_> = walls.tris.iter().filter(|t| t.iter().all(|&v| dist([walls.verts[v][0], walls.verts[v][1]], c) < r)).collect();
            assert!(inside.is_empty(), "{} wall triangles inside the junction at {c:?}", inside.len());
        }
        // and the ground there is continuous: the crossroads' middle at 70
        let ground = lvl.mesh.objects.iter().find(|o| o.name == "ground").unwrap();
        let mid: Vec<f64> = ground.verts.iter().filter(|v| dist([v[0], v[1]], [-800.0, 0.0]) < 1.0).map(|v| v[2]).collect();
        assert!(mid.iter().all(|z| (z - 70.0).abs() < 1.0), "{mid:?}");
        // main looks like dirt: full along its middle, none past its edges
        let blend: Vec<(P3, f64)> = ground.verts.iter().copied().zip(ground.blend.iter().copied()).collect();
        assert!(blend.iter().any(|(v, w)| v[1].abs() < 1.0 && v[0].abs() < 1000.0 && (w - 1.0).abs() < 1e-9));
        assert!(blend.iter().filter(|(v, _)| v[1].abs() > 81.0 && v[0].abs() < 1000.0 && (v[0] + 800.0).abs() > 81.0 && v[0].abs() > 81.0).all(|(_, w)| *w == 0.0));
    }

    #[test]
    fn switchbacks_climb_a_cliff_at_a_walkable_slope() {
        // the paths level's east plateau raised to 700, its ramp a switchback up to it
        let mut doc = paths_doc();
        doc.regions[0].z = 700.0;
        doc.paths.truncate(1);
        doc.paths[0].nodes = vec![vec![Some(1000.0), Some(-900.0)], vec![Some(1000.0), Some(900.0)]];
        doc.paths[0].switchbacks = Some(Switchbacks { width: Some(800.0), ..Default::default() });
        let lvl = build(&doc, &Theme::kokiri()).unwrap();
        assert!(lvl.problems.is_empty(), "{:?}", lvl.problems);
        let bad = open_edges(&lvl, &["ground", "walls", "cliffs", "bank", "trees"]);
        assert!(bad.is_empty(), "{} open or non-manifold edges, e.g. {:?}", bad.len(), &bad[..bad.len().min(6)]);
        // the ground along it climbs from 0 to 700, and sideways it stays in its corridor
        let ground = lvl.mesh.objects.iter().find(|o| o.name == "ground").unwrap();
        let on: Vec<&P3> = ground.verts.iter().filter(|v| (v[0] - 1000.0).abs() < 400.0 && v[1] > -950.0 && v[1] < 600.0 && v[2] > 1.0).collect();
        assert!(on.iter().any(|v| v[2] > 300.0 && v[2] < 500.0), "half way up");
        assert!(on.iter().all(|v| (v[0] - 1000.0).abs() <= 400.0));

        // floating: the deck lands at the plateau's edge, short of its last node, and the climb
        // still ends there (no jump to the node's height)
        let mut fl = doc.clone();
        fl.paths[0].mode = "floating".into();
        let lvl = build(&fl, &Theme::kokiri()).unwrap();
        assert!(lvl.problems.is_empty(), "{:?}", lvl.problems);
        // ...and a switchback doesn't land: it climbs on to its node, a cutting in the plateau
        let ground = lvl.mesh.objects.iter().find(|o| o.name == "ground").unwrap();
        assert!(ground.verts.iter().any(|v| v[1] > 650.0 && (v[0] - 1000.0).abs() < 400.0 && v[2] > 550.0 && v[2] < 690.0), "a cutting into the plateau");
        assert!(lvl.mesh.objects.iter().any(|o| o.name == "bridges" && !o.tris.is_empty()), "a deck where it's clear of the ground");
        let bad = open_edges(&lvl, &["ground", "walls", "cliffs", "bank", "trees"]);
        assert!(bad.is_empty(), "{} open or non-manifold edges, e.g. {:?}", bad.len(), &bad[..bad.len().min(6)]);

        // stretched walls: its sides show their texture once over their tallest height, so they're
        // in fewer bands than tiled, and still meet the ground vertex for vertex
        let mut st = doc.clone();
        st.settings.wall_texture = "stretched".into();
        let stretched = build(&st, &Theme::kokiri()).unwrap();
        let bad = open_edges(&stretched, &["ground", "walls", "cliffs", "bank", "trees"]);
        assert!(bad.is_empty(), "{} open or non-manifold edges, e.g. {:?}", bad.len(), &bad[..bad.len().min(6)]);
        let tiled = build(&doc, &Theme::kokiri()).unwrap();
        let walls = |l: &Level| l.mesh.objects.iter().find(|o| o.name == "walls").unwrap().tris.len();
        assert!(walls(&stretched) < walls(&tiled), "{} vs {}", walls(&stretched), walls(&tiled));
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
