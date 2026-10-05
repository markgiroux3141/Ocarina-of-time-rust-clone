//! Kit pieces set into walls (ADR 0036): openings (a log tunnel, a crawlspace) and wall pieces
//! (a vine patch, a waterfall). A prop of either kind is fitted to the wall nearest its `at`: its
//! origin goes on the wall's face at the floor in front, facing out over that floor.
//!
//! An opening also needs room, and gets a gap:
//! - **Room.** The wall must be tall enough for the mouth, straight enough across it (its points
//!   within a few units of a plane), and have space behind: solid ground at least the piece's
//!   `min_depth` deep. A log's cone is cut back (y scale) to the space there is. A crawlspace goes
//!   through a ridge to the floor beyond, so it's stretched to the ridge's depth and the far wall
//!   gets a gap too.
//! - **The gap.** The wall's triangles round the mouth come out. The area they covered, less the
//!   mouth's outline (the piece's front seen face on), is triangulated again in the wall's plane,
//!   so the wall meets the piece's rim with no hole round it and the rest of the wall edge for
//!   edge. Each new triangle takes its material and collision from the old triangle under it, and
//!   its UVs from that triangle's own, so the texture carries on across.

use crate::build::triangulate_with;
use crate::doc::Prop;
use crate::geom::{cross, P2, P3};
use crate::mesh::Mesh;
use crate::pieces::Piece;
use crate::props::Ground;

/// Where a prop goes on a wall: its origin, turn and depth scale, and the far wall's gap point.
pub struct Fit {
    pub origin: P3,
    pub yaw: f64,
    /// The y scale (depth) the room behind allows.
    pub depth: f64,
    /// For a crawlspace: the far floor's wall, (point on it, its outward normal).
    pub far: Option<(P2, P2)>,
    /// For a wall piece: the z scale that takes it to the wall's top.
    pub height: f64,
    /// The wall's object and its triangles near the front.
    obj: usize,
    n: P2,
    base: P2,
}

/// The wall nearest p within `reach`: (object, a point on it below p, its outward normal).
fn nearest_wall(mesh: &Mesh, p: P2, reach: f64, facing: Option<P2>) -> Option<(usize, P2, P2)> {
    let mut best: Option<(f64, usize, P2, P2)> = None;
    for (oi, o) in mesh.objects.iter().enumerate() {
        if o.collision_only || !["walls", "cliffs"].contains(&o.name.as_str()) {
            continue;
        }
        for tri in &o.tris {
            let [a, b, c] = tri.map(|v| o.verts[v]);
            let (e, f) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
            let nn = [e[1] * f[2] - e[2] * f[1], e[2] * f[0] - e[0] * f[2], e[0] * f[1] - e[1] * f[0]];
            let l = (nn[0] * nn[0] + nn[1] * nn[1] + nn[2] * nn[2]).sqrt();
            let lxy = nn[0].hypot(nn[1]);
            if l < 1e-6 || lxy / l < 0.95 {
                continue; // not a vertical wall
            }
            let n = [nn[0] / lxy, nn[1] / lxy];
            if facing.is_some_and(|f| n[0] * f[0] + n[1] * f[1] < 0.9) {
                continue;
            }
            // the triangle seen from above is a segment along the wall
            let t = [-n[1], n[0]];
            let ss = [a, b, c].map(|q| (q[0] - a[0]) * t[0] + (q[1] - a[1]) * t[1]);
            let (s0, s1) = (ss.iter().cloned().fold(f64::INFINITY, f64::min), ss.iter().cloned().fold(f64::NEG_INFINITY, f64::max));
            let sp = (p[0] - a[0]) * t[0] + (p[1] - a[1]) * t[1];
            let dp = (p[0] - a[0]) * n[0] + (p[1] - a[1]) * n[1];
            let off = if sp < s0 { s0 - sp } else if sp > s1 { sp - s1 } else { 0.0 };
            let d = off.hypot(dp);
            if d <= reach && best.is_none_or(|x| d < x.0) {
                let sc = sp.clamp(s0, s1);
                best = Some((d, oi, [a[0] + t[0] * sc, a[1] + t[1] * sc], n));
            }
        }
    }
    best.map(|(_, o, b, n)| (o, b, n))
}

/// The piece's points in the level for a fit (scale within its limits, depth as fitted).
fn placed_points<'a>(piece: &'a Piece, prop: &'a Prop, fit: &'a Fit) -> impl Fn([f64; 3]) -> P3 + 'a {
    let mut scale = piece.scale.clamp(prop.scale);
    scale[1] = fit.depth;
    move |v| crate::props::transform(v, fit.origin, fit.yaw, scale)
}

/// Fits a wall piece or opening to the wall near its `at`.
/// A wall's shape across `half` either side of `base` (facing `n`), below `z_hi`: how far its
/// points stray from the plane there (its bend), and its top.
fn survey(mesh: &Mesh, obj: usize, base: P2, n: P2, half: f64, z_hi: f64) -> (f64, f64) {
    let o = &mesh.objects[obj];
    let t = [-n[1], n[0]];
    let (mut bend, mut top): (f64, f64) = (0.0, f64::NEG_INFINITY);
    for tri in &o.tris {
        // only this wall's faces: facing the same way
        let [a, b, c] = tri.map(|v| o.verts[v]);
        let (e, f) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
        let nn = [e[1] * f[2] - e[2] * f[1], e[2] * f[0] - e[0] * f[2]];
        let l = nn[0].hypot(nn[1]);
        if l < 1e-6 || (nn[0] * n[0] + nn[1] * n[1]) / l < 0.5 {
            continue;
        }
        // a triangle overlapping the span (a long straight panel may have no point inside it)
        let sd = [a, b, c].map(|q| ((q[0] - base[0]) * t[0] + (q[1] - base[1]) * t[1], (q[0] - base[0]) * n[0] + (q[1] - base[1]) * n[1]));
        let (s0, s1) = (sd.iter().map(|x| x.0).fold(f64::INFINITY, f64::min), sd.iter().map(|x| x.0).fold(f64::NEG_INFINITY, f64::max));
        if s1 < -half || s0 > half || sd.iter().any(|x| x.1.abs() >= 60.0) {
            continue;
        }
        for (k, q) in [a, b, c].iter().enumerate() {
            // its points beyond the span count only for their height
            let (s, d) = sd[k];
            if s.abs() <= half && q[2] < z_hi {
                bend = bend.max(d.abs());
            }
            top = top.max(q[2]);
        }
        // and where it crosses the span's ends, how far it is from the plane there
        for e in [-half, half] {
            for (i, j) in [(0, 1), (1, 2), (2, 0)] {
                let (p, q) = (sd[i], sd[j]);
                if (p.0 - e) * (q.0 - e) < 0.0 {
                    let f = (e - p.0) / (q.0 - p.0);
                    let z = [a, b, c][i][2] + f * ([a, b, c][j][2] - [a, b, c][i][2]);
                    if z < z_hi {
                        bend = bend.max((p.1 + f * (q.1 - p.1)).abs());
                    }
                }
            }
        }
    }
    (bend, top)
}

/// How far an opening's wall may stray from flat: a log's rim stands well out from the wall and
/// covers a curve; a crawlspace's arches lie flat on the wall, so its walls must be flat.
fn bend_allowed(piece: &Piece) -> f64 {
    match piece.opening.as_ref().and_then(|o| o.exit) {
        Some(_) => 40.0,
        None => 4.0,
    }
}

/// Fits a wall piece or opening to the wall near its `at`.
pub fn fit(piece: &Piece, prop: &Prop, mesh: &Mesh, ground: &Ground) -> Result<Fit, String> {
    let name = &piece.label;
    let (obj, base, n) = nearest_wall(mesh, prop.at, 300.0, None).ok_or(format!("{name}: no wall within 300 of it"))?;
    let floor = ground.at(mesh, [base[0] + n[0] * 12.0, base[1] + n[1] * 12.0]).0.ok_or(format!("{name}: no floor in front of the wall"))?;
    let yaw = (-n[0]).atan2(n[1]).to_degrees();
    let mut fit = Fit { origin: [base[0], base[1], floor], yaw, depth: piece.scale.clamp(prop.scale)[1], far: None, height: piece.scale.clamp(prop.scale)[2], obj, n, base };
    let Some(op) = &piece.opening else {
        // wall pieces (vines, a waterfall) lie flat on the wall's face, a hair out so they don't
        // fight it for depth, from the floor to the wall's top: the wall must be flat there
        let width = (piece.bounds[1][0] - piece.bounds[0][0]) * piece.scale.clamp(prop.scale)[0];
        let (bend, top) = survey(mesh, obj, base, n, width / 2.0 + 10.0, f64::INFINITY);
        if bend > 3.0 {
            return Err(format!("{name}: the wall isn't flat here ({bend:.0} off); it goes on a flat stretch {width:.0} wide"));
        }
        let height = (piece.bounds[1][2] - piece.bounds[0][2]).max(1.0);
        let need = (top - floor) / height;
        if need < piece.scale.min[2] - 1e-9 || need > piece.scale.max[2] + 1e-9 {
            return Err(format!("{name}: the wall is {:.0} tall here; it reaches {:.0} to {:.0}", top - floor, height * piece.scale.min[2], height * piece.scale.max[2]));
        }
        fit.origin = [base[0] + n[0] * 1.5, base[1] + n[1] * 1.5, floor];
        fit.height = need;
        return Ok(fit);
    };
    // height and straightness across the mouth
    let (bend, top) = survey(mesh, obj, base, n, op.width / 2.0 + 20.0, floor + op.height + 10.0);
    let allowed = bend_allowed(piece);
    if bend > allowed {
        return Err(format!("{name}: the wall bends {bend:.0} off flat across the mouth; it needs {:.0} of wall within {allowed:.0} of flat", op.width + 40.0));
    }
    if top - floor < op.height + 10.0 {
        return Err(format!("{name}: the wall is {:.0} tall here, it needs {:.0}", (top - floor).max(0.0), op.height + 10.0));
    }
    // room behind: solid ground (or the edge of the world) above the mouth, as deep as it goes
    let full = op.depth;
    let (lo, hi) = (piece.scale.min[1], piece.scale.max[1]);
    let mut room = 0.0;
    let mut far = None;
    let mut s = 10.0;
    while s <= full * hi + 10.0 {
        let q = [base[0] - n[0] * s, base[1] - n[1] * s];
        match ground.at(mesh, q).0 {
            Some(z) if z < floor + op.height => {
                far = Some(q);
                break;
            }
            _ => room = s,
        }
        s += 10.0;
    }
    match (op.exit, far) {
        // an exit's tunnel ends in the wall: cut back to the room there is
        (Some(_), _) => {
            let d = (room / full).min(1.0);
            if d < lo || room < op.min_depth {
                return Err(format!("{name}: {room:.0} of room behind the wall, it needs {:.0}", op.min_depth));
            }
            fit.depth = d.clamp(lo, hi);
        }
        // a crawlspace goes through a ridge to the floor beyond: the far wall has to be flat too,
        // and face the other way (parallel), so both arches sit flush
        (None, Some(q)) => {
            let (fobj, fb, fnorm) = nearest_wall(mesh, [q[0] + n[0] * 10.0, q[1] + n[1] * 10.0], 60.0, Some([-n[0], -n[1]]))
                .ok_or(format!("{name}: no wall on the far side facing the other way"))?;
            let angle = (-(fnorm[0] * n[0] + fnorm[1] * n[1])).clamp(-1.0, 1.0).acos().to_degrees();
            if angle > 4.0 {
                return Err(format!("{name}: the far wall is {angle:.0} degrees off parallel; the ridge's two walls must be parallel (within 4)"));
            }
            let depth = (base[0] - fb[0]) * n[0] + (base[1] - fb[1]) * n[1];
            // where the tunnel comes out, straight through
            let fb = [base[0] - n[0] * depth, base[1] - n[1] * depth];
            let ffloor = ground.at(mesh, [fb[0] - n[0] * 12.0, fb[1] - n[1] * 12.0]).0.unwrap_or(f64::NEG_INFINITY);
            if (ffloor - floor).abs() > 6.0 {
                return Err(format!("{name}: the floor beyond is {:.0} off this one; both ends must be level", ffloor - floor));
            }
            let (fbend, _) = survey(mesh, fobj, fb, [-n[0], -n[1]], op.width / 2.0 + 20.0, floor + op.height + 10.0);
            if fbend > allowed {
                return Err(format!("{name}: the far wall bends {fbend:.0} off flat; it needs {:.0} of flat wall", op.width + 40.0));
            }
            let d = depth / full;
            if d < lo || d > hi {
                return Err(format!("{name}: the ridge is {depth:.0} deep here; it reaches {:.0} to {:.0}", full * lo, full * hi));
            }
            fit.depth = d;
            fit.far = Some((fb, [-n[0], -n[1]]));
        }
        (None, None) => return Err(format!("{name}: no floor beyond the wall within {:.0}", full * hi)),
    }
    Ok(fit)
}

/// Cuts the gap for a fitted opening (and its far end), and returns the problems.
pub fn punch_fit(piece: &Piece, prop: &Prop, fit: &Fit, mesh: &mut Mesh) -> Result<(), String> {
    let at = placed_points(piece, prop, fit);
    let depth = piece.opening.as_ref().map_or(0.0, |o| o.depth) * fit.depth;
    let front: Vec<P3> = piece.verts.iter().filter(|v| v[1] * fit.depth > -40.0).map(|&v| at(v)).collect();
    punch(mesh, fit.obj, fit.base, fit.n, &front).map_err(|e| format!("{}: {e}", piece.label))?;
    if let Some((fb, fnorm)) = fit.far {
        let back: Vec<P3> = piece.verts.iter().filter(|v| v[1] * fit.depth < -(depth - 10.0)).map(|&v| at(v)).collect();
        let obj = nearest_wall(mesh, fb, 20.0, Some(fnorm)).map(|x| x.0).ok_or(format!("{}: the far wall moved", piece.label))?;
        punch(mesh, obj, fb, fnorm, &back).map_err(|e| format!("{}: its far end: {e}", piece.label))?;
    }
    Ok(())
}

/// The convex hull of points (counter-clockwise).
fn hull(pts: Vec<P2>) -> Vec<P2> {
    crate::pieces::hull(pts)
}

/// Clips a convex polygon to z >= z0 (in (s, z)).
fn clip_below(poly: &[P2], z0: f64) -> Vec<P2> {
    let mut out = vec![];
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        let (ia, ib) = (a[1] >= z0, b[1] >= z0);
        if ia {
            out.push(a);
        }
        if ia != ib {
            let t = (z0 - a[1]) / (b[1] - a[1]);
            out.push([a[0] + (b[0] - a[0]) * t, z0]);
        }
    }
    out
}

/// Takes the wall triangles of object `obj` round `outline` (points in the level, on or near the
/// wall through `base` facing `n`) out, and fills what they covered, less the outline seen face
/// on, with new triangles in the wall's plane carrying the old ones' materials, collision and UVs.
pub fn punch(mesh: &mut Mesh, obj: usize, base: P2, n: P2, outline: &[P3]) -> Result<(), String> {
    let t = [-n[1], n[0]];
    let sz = |q: P3| [(q[0] - base[0]) * t[0] + (q[1] - base[1]) * t[1], q[2]];
    let dn = |q: P3| (q[0] - base[0]) * n[0] + (q[1] - base[1]) * n[1];
    let m = hull(outline.iter().map(|&q| sz(q)).collect());
    if m.len() < 3 {
        return Err("no mouth to cut".into());
    }
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for q in &m {
        lo = [lo[0].min(q[0]), lo[1].min(q[1])];
        hi = [hi[0].max(q[0]), hi[1].max(q[1])];
    }
    let pad = 6.0;
    let o = &mesh.objects[obj];
    // this wall's own faces: facing the same way, near the plane
    let same_wall = |ti: usize| {
        let [a, b, c] = o.tris[ti].map(|v| o.verts[v]);
        let (e, f) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
        let nn = [e[1] * f[2] - e[2] * f[1], e[2] * f[0] - e[0] * f[2]];
        let l = nn[0].hypot(nn[1]);
        l > 1e-6 && (nn[0] * n[0] + nn[1] * n[1]) / l > 0.5 && [a, b, c].iter().all(|&q| dn(q).abs() <= 80.0)
    };
    // the wall's triangles over the mouth's box (and a little round it)
    let cut: Vec<usize> = (0..o.tris.len())
        .filter(|&ti| {
            let p = o.tris[ti].map(|v| o.verts[v]);
            if !same_wall(ti) {
                return false;
            }
            let s = p.map(sz);
            let (a, b) = ([s.iter().map(|q| q[0]).fold(f64::INFINITY, f64::min), s.iter().map(|q| q[1]).fold(f64::INFINITY, f64::min)], [
                s.iter().map(|q| q[0]).fold(f64::NEG_INFINITY, f64::max),
                s.iter().map(|q| q[1]).fold(f64::NEG_INFINITY, f64::max),
            ]);
            a[0] < hi[0] + pad && b[0] > lo[0] - pad && a[1] < hi[1] + pad && b[1] > lo[1] - pad
        })
        .collect();
    if cut.is_empty() {
        return Err("no wall there".into());
    }
    // the cut area's outline (edges used once, chained), grown until the mouth is inside it:
    // where only half a quad touched the mouth's box, the outline would cross the mouth
    let mut cut = cut;
    let (ring, outer, mouth) = 'grow: {
        for _ in 0..24 {
            let mut count: std::collections::HashMap<(usize, usize), (usize, usize)> = Default::default();
            for &ti in &cut {
                let tr = o.tris[ti];
                for k in 0..3 {
                    let (a, b) = (tr[k], tr[(k + 1) % 3]);
                    let e = count.entry((a.min(b), a.max(b))).or_insert((0, a));
                    e.0 += 1;
                    e.1 = a;
                }
            }
            let mut next: std::collections::HashMap<usize, usize> = Default::default();
            for (&(a, b), &(c, from)) in &count {
                if c == 1 {
                    next.insert(from, if from == a { b } else { a });
                }
            }
            let start = *next.keys().min().ok_or("the cut has no outline")?;
            let mut ring = vec![start];
            let mut cur = start;
            while let Some(&nx) = next.get(&cur) {
                if nx == start {
                    break;
                }
                ring.push(nx);
                cur = nx;
                if ring.len() > next.len() + 1 {
                    return Err("the wall round it isn't in one piece".into());
                }
            }
            if ring.len() != next.len() {
                return Err("the wall round it has holes".into());
            }
            let outer: Vec<P2> = ring.iter().map(|&v| sz(o.verts[v])).collect();
            // the mouth, kept a hair above the cut area's lowest point under it (the floor)
            // the floor under the mouth: the cut outline's lowest crossing at the mouth's sides and
            // middle, and its points between them; the mouth stays above the highest of those
            let foot = |sx: f64| {
                let mut z = f64::INFINITY;
                for i in 0..outer.len() {
                    let (a, b) = (outer[i], outer[(i + 1) % outer.len()]);
                    if (a[0] - sx) * (b[0] - sx) <= 0.0 && (a[0] - b[0]).abs() > 1e-9 {
                        z = z.min(a[1] + (sx - a[0]) / (b[0] - a[0]) * (b[1] - a[1]));
                    }
                }
                z
            };
            let mut bottom = [lo[0], 0.5 * (lo[0] + hi[0]), hi[0]].map(foot).iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            for q in outer.iter().filter(|q| q[0] > lo[0] && q[0] < hi[0] && q[1] < lo[1] + 20.0) {
                bottom = bottom.max(q[1]);
            }
            let mouth = clip_below(&m, bottom + 1.0);
            if mouth.len() < 3 {
                return Err("the mouth is below the floor".into());
            }
            let inside: Vec<usize> = ring.iter().zip(&outer).filter(|(_, q)| crate::geom::point_in_poly(**q, &mouth)).map(|(&v, _)| v).collect();
            if inside.is_empty() {
                break 'grow (ring, outer, mouth);
            }
            let before = cut.len();
            for ti in 0..o.tris.len() {
                if !cut.contains(&ti) && same_wall(ti) && o.tris[ti].iter().any(|v| inside.contains(v)) {
                    cut.push(ti);
                }
            }
            if cut.len() == before {
                break;
            }
        }
        return Err("the wall is too small round the mouth".into());
    };
    let tris = triangulate_with(&outer, std::slice::from_ref(&mouth), 0.0, &[]);
    // the old triangles, to carry their material, collision and UVs over
    let old: Vec<([P2; 3], [[f64; 2]; 3], usize, i64, [P3; 3])> =
        cut.iter().map(|&ti| (o.tris[ti].map(|v| sz(o.verts[v])), o.uvs[ti], o.mat[ti], o.surf[ti], o.tris[ti].map(|v| o.verts[v]))).collect();
    let from = |q: P2| -> Option<(usize, [f64; 3])> {
        let mut best: Option<(f64, usize, [f64; 3])> = None;
        for (k, (s, _, _, _, _)) in old.iter().enumerate() {
            let d = cross(s[0], s[1], s[2]);
            if d.abs() < 1e-9 {
                continue;
            }
            let l1 = cross(q, s[1], s[2]) / d;
            let l2 = cross(s[0], q, s[2]) / d;
            let l3 = 1.0 - l1 - l2;
            let outside = (-l1).max(-l2).max(-l3);
            if best.is_none_or(|b| outside < b.0) {
                best = Some((outside, k, [l1, l2, l3]));
            }
        }
        best.map(|(_, k, l)| (k, l))
    };
    // back into the level: on the old wall's own surface (the triangle under each point), so a
    // curved wall stays curved
    let (mats, surfs) = (mesh.materials.clone(), mesh.surfaces.clone());
    let name = mesh.objects[obj].name.clone();
    let mut keep = vec![true; mesh.objects[obj].tris.len()];
    for &ti in &cut {
        keep[ti] = false;
    }
    {
        let o = &mut mesh.objects[obj];
        let mut i = 0;
        o.tris.retain(|_| {
            i += 1;
            keep[i - 1]
        });
        let mut i = 0;
        o.uvs.retain(|_| {
            i += 1;
            keep[i - 1]
        });
        let mut i = 0;
        o.mat.retain(|_| {
            i += 1;
            keep[i - 1]
        });
        let mut i = 0;
        o.surf.retain(|_| {
            i += 1;
            keep[i - 1]
        });
    }
    // the outline's own points keep their exact positions (they're the wall's)
    let exact: std::collections::HashMap<(i64, i64), P3> = ring.iter().map(|&v| {
        let q = mesh.objects[obj].verts[v];
        let s = sz(q);
        (((s[0] * 1000.0).round() as i64, (s[1] * 1000.0).round() as i64), q)
    }).collect();
    // the mouth's own points lie on the piece's front (the plane through base), so the wall
    // bends to meet it however the old wall curved
    let key = |q: P2| ((q[0] * 1000.0).round() as i64, (q[1] * 1000.0).round() as i64);
    let on_mouth: std::collections::HashSet<(i64, i64)> = mouth.iter().map(|&q| key(q)).collect();
    let place = |q: P2| {
        if on_mouth.contains(&key(q)) {
            return [base[0] + t[0] * q[0], base[1] + t[1] * q[0], q[1]];
        }
        exact.get(&key(q)).copied().unwrap_or_else(|| match from(q) {
            Some((k, l)) => {
                let p = old[k].4;
                [0, 1, 2].map(|i| p[0][i] * l[0] + p[1][i] * l[1] + p[2][i] * l[2])
            }
            None => [base[0] + t[0] * q[0], base[1] + t[1] * q[0], q[1]],
        })
    };
    for tri in tris {
        let c = [(tri[0][0] + tri[1][0] + tri[2][0]) / 3.0, (tri[0][1] + tri[1][1] + tri[2][1]) / 3.0];
        let Some((k, _)) = from(c) else { continue };
        let (s, uv, mat, surf, _) = &old[k];
        let d = cross(s[0], s[1], s[2]);
        let uvs = tri.map(|q| {
            let l1 = cross(q, s[1], s[2]) / d;
            let l2 = cross(s[0], q, s[2]) / d;
            let l3 = 1.0 - l1 - l2;
            [uv[0][0] * l1 + uv[1][0] * l2 + uv[2][0] * l3, uv[0][1] * l1 + uv[1][1] * l2 + uv[2][1] * l3]
        });
        let surface = if *surf >= 0 { surfs[*surf as usize].as_str() } else { "" };
        mesh.tri(&name, tri.map(place), uvs, &mats[*mat], surface);
    }
    Ok(())
}
