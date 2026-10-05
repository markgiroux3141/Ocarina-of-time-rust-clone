//! Kit pieces placed in a level (`Doc::props`): each piece (`pieces.rs`) turned, scaled and stood
//! on the finished ground, its triangles drawn with their own normals and tints (object `props`)
//! and its collision added as never-drawn triangles (object `props_collision`), surface roles and
//! all. Done after the painted terrain moves the ground and before lighting, so props stand on the
//! hills and are lit like the rest.
//!
//! Standing on the ground: a piece's origin goes on the highest floor under its anchor (a house's
//! door at its base, so its doorway meets the ground; otherwise the origin), or at the prop's `z`.
//! A stone (kind "stone") stands in water with its top 15 above the surface, as spot04's do. Where
//! the ground under a piece's footprint falls away more than 20 below its base, it's reported.

use crate::doc::Prop;
use crate::geom::{point_in_poly, P2, P3};
use crate::mesh::Mesh;
use crate::pieces::{Kit, Piece};

/// A placed prop, for the editor: where it ended up and what it costs.
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    /// Index into `Doc::props`.
    pub index: usize,
    pub piece: String,
    /// The origin in the level, after standing on the ground.
    pub origin: P3,
    pub yaw: f64,
    pub scale: [f64; 3],
    /// The footprint seen from above, in level coordinates.
    pub footprint: Vec<P2>,
    pub top: f64,
    pub collision_vertices: usize,
}

/// The game indexes collision vertices in 13 bits (`oot_import::level`).
pub const MAX_COLLISION_VERTICES: usize = 1 << 13;

/// spot04's stepping stones' tops stand this far above the pond's surface.
const STONE_ABOVE_WATER: f64 = 15.0;

/// The highest floor and the water surface at points of the level, from the built mesh.
pub struct Ground {
    cell: f64,
    origin: P2,
    w: usize,
    h: usize,
    /// Per cell: floor triangles (object, triangle) touching it.
    cells: Vec<Vec<(usize, usize, bool)>>,
}

impl Ground {
    pub fn new(mesh: &Mesh) -> Ground {
        let water = mesh.surfaces.iter().position(|s| s == "water").map(|i| i as i64);
        let mut items: Vec<(usize, usize, bool, [f64; 4])> = vec![];
        for (oi, o) in mesh.objects.iter().enumerate() {
            if o.collision_only {
                continue;
            }
            for (t, tri) in o.tris.iter().enumerate() {
                let s = o.surf[t];
                let is_water = s >= 0 && Some(s) == water;
                if s < 0 && !is_water {
                    continue;
                }
                let [a, b, c] = tri.map(|v| o.verts[v]);
                let n = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
                if n <= 1e-6 {
                    continue; // walls, ceilings
                }
                let bb = [a[0].min(b[0]).min(c[0]), a[1].min(b[1]).min(c[1]), a[0].max(b[0]).max(c[0]), a[1].max(b[1]).max(c[1])];
                items.push((oi, t, is_water, bb));
            }
        }
        let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
        for i in &items {
            lo = [lo[0].min(i.3[0]), lo[1].min(i.3[1])];
            hi = [hi[0].max(i.3[2]), hi[1].max(i.3[3])];
        }
        if items.is_empty() {
            return Ground { cell: 1.0, origin: [0.0; 2], w: 0, h: 0, cells: vec![] };
        }
        let cell = 200.0;
        let (w, h) = (((hi[0] - lo[0]) / cell) as usize + 1, ((hi[1] - lo[1]) / cell) as usize + 1);
        let mut cells = vec![vec![]; w * h];
        for (oi, t, wt, bb) in items {
            let (x0, y0) = (((bb[0] - lo[0]) / cell) as usize, ((bb[1] - lo[1]) / cell) as usize);
            let (x1, y1) = ((((bb[2] - lo[0]) / cell) as usize).min(w - 1), (((bb[3] - lo[1]) / cell) as usize).min(h - 1));
            for y in y0..=y1 {
                for x in x0..=x1 {
                    cells[y * w + x].push((oi, t, wt));
                }
            }
        }
        Ground { cell, origin: lo, w, h, cells }
    }

    /// (highest floor, water surface) at p.
    pub fn at(&self, mesh: &Mesh, p: P2) -> (Option<f64>, Option<f64>) {
        let (fx, fy) = ((p[0] - self.origin[0]) / self.cell, (p[1] - self.origin[1]) / self.cell);
        if fx < 0.0 || fy < 0.0 || fx as usize >= self.w || fy as usize >= self.h {
            return (None, None);
        }
        let (mut floor, mut water) = (None::<f64>, None::<f64>);
        for &(oi, t, wt) in &self.cells[fy as usize * self.w + fx as usize] {
            let o = &mesh.objects[oi];
            let [a, b, c] = o.tris[t].map(|v| o.verts[v]);
            let d = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
            if d.abs() < 1e-9 {
                continue;
            }
            let l1 = ((b[1] - c[1]) * (p[0] - c[0]) + (c[0] - b[0]) * (p[1] - c[1])) / d;
            let l2 = ((c[1] - a[1]) * (p[0] - c[0]) + (a[0] - c[0]) * (p[1] - c[1])) / d;
            let l3 = 1.0 - l1 - l2;
            if l1 < -1e-6 || l2 < -1e-6 || l3 < -1e-6 {
                continue;
            }
            let z = l1 * a[2] + l2 * b[2] + l3 * c[2];
            let slot = if wt { &mut water } else { &mut floor };
            *slot = Some(slot.map_or(z, |m| m.max(z)));
        }
        (floor, water)
    }
}

/// Turns, scales and moves a point of a piece's frame into the level.
pub fn transform(p: P3, origin: P3, yaw_deg: f64, scale: [f64; 3]) -> P3 {
    let (s, c) = yaw_deg.to_radians().sin_cos();
    let q = [p[0] * scale[0], p[1] * scale[1], p[2] * scale[2]];
    [origin[0] + q[0] * c - q[1] * s, origin[1] + q[0] * s + q[1] * c, origin[2] + q[2]]
}

fn normal(n: P3, yaw_deg: f64, scale: [f64; 3]) -> P3 {
    // normals take the inverse scale (then renormalise), and the same turn
    let q = [n[0] / scale[0], n[1] / scale[1], n[2] / scale[2]];
    let l = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2]).sqrt().max(1e-12);
    let (s, c) = yaw_deg.to_radians().sin_cos();
    let q = q.map(|x| x / l);
    [q[0] * c - q[1] * s, q[0] * s + q[1] * c, q[2]]
}

/// A prop's footprint seen from above, in level coordinates (its scale within the piece's limits).
pub fn footprint(piece: &Piece, prop: &Prop) -> Vec<P2> {
    let scale = piece.scale.clamp(prop.scale);
    piece
        .footprint
        .iter()
        .map(|q| {
            let w = transform([q[0], q[1], 0.0], [prop.at[0], prop.at[1], 0.0], prop.yaw, scale);
            [w[0], w[1]]
        })
        .collect()
}

/// The way a prop faces, in level coordinates (yaw 0: north).
pub fn facing(yaw_deg: f64) -> P2 {
    let (s, c) = yaw_deg.to_radians().sin_cos();
    [-s, c]
}

/// Where a prop's origin goes: its xy, and its height on the ground (or its own `z`).
pub fn stand(piece: &Piece, prop: &Prop, ground: &Ground, mesh: &Mesh) -> (P3, Vec<String>) {
    let scale = piece.scale.clamp(prop.scale);
    let mut problems = vec![];
    // a door at the piece's base (not up on a porch) stands on the ground
    let anchor = match piece.door.as_ref().filter(|d| d.pos[2].abs() < 1.0) {
        Some(d) => {
            let a = transform([d.pos[0], d.pos[1], 0.0], [prop.at[0], prop.at[1], 0.0], prop.yaw, scale);
            [a[0], a[1]]
        }
        None => prop.at,
    };
    let z = match prop.z {
        Some(z) => z,
        None => {
            let (floor, water) = ground.at(mesh, anchor);
            match (floor, water) {
                (_, Some(w)) if piece.kind == "stone" => w + STONE_ABOVE_WATER,
                (Some(f), _) => f,
                (None, Some(w)) => w,
                (None, None) => {
                    problems.push(format!("prop {} stands outside the level", piece.name));
                    0.0
                }
            }
        }
    };
    (([prop.at[0], prop.at[1], z]), problems)
}

/// Places every prop of `props` into `mesh` and reports where they went. Unknown pieces and
/// props outside the level are problems, not errors.
pub fn place(props: &[Prop], kit: Option<&Kit>, mesh: &mut Mesh, problems: &mut Vec<String>) -> Vec<Placed> {
    if props.is_empty() {
        return vec![];
    }
    let Some(kit) = kit else {
        problems.push(format!("{} props not built: no kit (make it with `overworld kit-pieces`)", props.len()));
        return vec![];
    };
    let ground = Ground::new(mesh);
    let mut stood = vec![];
    let mut fitted: Vec<(usize, crate::openings::Fit)> = vec![];
    for (i, prop) in props.iter().enumerate() {
        let Some(piece) = kit.get(&prop.piece) else {
            problems.push(format!("prop {i}: the kit has no piece {:?}", prop.piece));
            continue;
        };
        // openings and wall pieces fit themselves to the nearest wall
        if piece.kind == "opening" || piece.kind == "wall" {
            match crate::openings::fit(piece, prop, mesh, &ground) {
                Ok(f) => {
                    let mut scale = piece.scale.clamp(prop.scale);
                    scale[1] = f.depth;
                    if piece.opening.is_none() {
                        scale[2] = f.height;
                    }
                    let fit_prop = Prop { at: [f.origin[0], f.origin[1]], yaw: f.yaw, scale, z: Some(f.origin[2]), piece: prop.piece.clone() };
                    stood.push((i, piece, f.origin, scale, fit_prop));
                    fitted.push((stood.len() - 1, f));
                }
                Err(e) => problems.push(format!("prop {i}: {e}")),
            }
            continue;
        }
        let (origin, mut pr) = stand(piece, prop, &ground, mesh);
        problems.append(&mut pr);
        let scale = piece.scale.clamp(prop.scale);
        // uneven ground under the footprint
        if prop.z.is_none() && piece.kind != "stone" && piece.kind != "opening" && piece.kind != "wall" {
            let mut lowest = origin[2];
            for q in &piece.footprint {
                let w = transform([q[0] * 0.9, q[1] * 0.9, 0.0], origin, prop.yaw, scale);
                if let (Some(f), _) = ground.at(mesh, [w[0], w[1]]) {
                    lowest = lowest.min(f);
                }
            }
            if origin[2] - lowest > 20.0 {
                problems.push(format!("prop {i} ({}): the ground falls {:.0} below it at its edge", piece.name, origin[2] - lowest));
            }
        }
        stood.push((i, piece, origin, scale, prop.clone()));
    }
    // the gaps in the walls, once every fit is known
    for (k, f) in &fitted {
        let (i, piece, _, _, fit_prop) = &stood[*k];
        if piece.kind == "opening" {
            if let Err(e) = crate::openings::punch_fit(piece, fit_prop, f, mesh) {
                problems.push(format!("prop {i}: {e}"));
            }
        }
    }
    let mut out = vec![];
    for (i, piece, origin, scale, prop) in stood {
        let prop = &prop;
        for (t, tri) in piece.tris.iter().enumerate() {
            let m = &piece.materials[piece.mat[t] as usize];
            let p = tri.map(|v| transform(piece.verts[v as usize], origin, prop.yaw, scale));
            let n = tri.map(|v| normal(piece.normals[v as usize], prop.yaw, scale));
            mesh.tri_lit("props", p, n, piece.uvs[t], &m.texture, m.tint);
        }
        // a crawlspace's floor calls for its own camera: the line through it, 22 past each mouth
        // and 12 up, as spot04's (Camera_Subj4 carries Link along it)
        let mut roles = piece.surfaces.clone();
        if let Some(k) = roles.iter().position(|r| r == "crawl_floor") {
            let depth = piece.opening.as_ref().map_or(piece.bounds[1][1] - piece.bounds[0][1], |o| o.depth);
            let ends = [[0.0, 22.0 / scale[1], 12.0], [0.0, -depth - 22.0 / scale[1], 12.0]];
            mesh.cameras.push(crate::mesh::Camera { setting: "crawlspace".into(), points: ends.iter().map(|&e| transform(e, origin, prop.yaw, scale)).collect() });
            roles[k] = format!("crawl_floor#{}", mesh.cameras.len());
        }
        for (t, tri) in piece.col_tris.iter().enumerate() {
            let p = tri.map(|v| transform(piece.col_verts[v as usize], origin, prop.yaw, scale));
            mesh.col_tri("props_collision", p, &roles[piece.col_surf[t] as usize]);
        }
        let footprint = footprint(piece, prop);
        out.push(Placed {
            index: i,
            piece: piece.name.clone(),
            origin,
            yaw: prop.yaw,
            scale,
            footprint,
            top: origin[2] + piece.bounds[1][2] * scale[2],
            collision_vertices: piece.collision_vertices(),
        });
    }
    out
}

/// The prop whose footprint holds p (the last placed wins, as it's drawn on top).
pub fn pick(placed: &[Placed], p: P2) -> Option<usize> {
    placed.iter().rev().find(|pl| point_in_poly(p, &pl.footprint)).map(|pl| pl.index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pieces::{Door, PieceMaterial, Scale};

    /// A 100-square box 50 tall with a door at +y, its collision a floor on top.
    pub fn test_piece(kind: &str) -> Piece {
        let verts = vec![[-50.0, -50.0, 0.0], [50.0, -50.0, 0.0], [50.0, 50.0, 0.0], [-50.0, 50.0, 0.0], [-50.0, -50.0, 50.0], [50.0, -50.0, 50.0], [50.0, 50.0, 50.0], [-50.0, 50.0, 50.0]];
        Piece {
            name: "box".into(),
            label: "Box".into(),
            kind: kind.into(),
            materials: vec![PieceMaterial { texture: "kf_house_bark".into(), tint: [1.0, 0.5, 1.0] }],
            normals: vec![[0.0, 0.0, 1.0]; 8],
            verts: verts.clone(),
            tris: vec![[4, 5, 6], [4, 6, 7]],
            uvs: vec![[[0.0; 2]; 3]; 2],
            mat: vec![0, 0],
            surfaces: vec!["wood".into()],
            col_verts: verts[4..].to_vec(),
            col_tris: vec![[0, 1, 2], [0, 2, 3]],
            col_surf: vec![0, 0],
            bounds: [[-50.0, -50.0, 0.0], [50.0, 50.0, 50.0]],
            footprint: vec![[-50.0, -50.0], [50.0, -50.0], [50.0, 50.0], [-50.0, 50.0]],
            scale: Scale { min: [1.0; 3], max: [2.0; 3], uniform: true },
            door: (kind == "house").then(|| Door { pos: [0.0, 50.0, 0.0], exit: 1, entrance: "X".into() }),
            ..Default::default()
        }
    }

    fn slope_mesh() -> Mesh {
        // ground rising 1 in 4 eastward, from x -1000 to 1000
        let mut m = Mesh::default();
        let z = |x: f64| x / 4.0;
        m.quad("ground", [[-1000.0, -1000.0, z(-1000.0)], [1000.0, -1000.0, z(1000.0)], [1000.0, 1000.0, z(1000.0)], [-1000.0, 1000.0, z(-1000.0)]], [[0.0; 2]; 4], "ground", "ground");
        m.quad("water", [[300.0, 300.0, -50.0], [600.0, 300.0, -50.0], [600.0, 600.0, -50.0], [300.0, 600.0, -50.0]], [[0.0; 2]; 4], "water", "water");
        m
    }

    #[test]
    fn props_stand_on_the_ground_turned_and_scaled() {
        let kit = Kit { pieces: vec![test_piece("tower")], ..Default::default() };
        let mut m = slope_mesh();
        let props = vec![Prop { piece: "box".into(), at: [200.0, 0.0], z: None, yaw: 90.0, scale: [5.0, 1.0, 1.0] }];
        let mut problems = vec![];
        let placed = place(&props, Some(&kit), &mut m, &mut problems);
        assert_eq!(placed.len(), 1);
        // on the ground at x 200, scaled uniformly within its limits (2)
        assert!((placed[0].origin[2] - 50.0).abs() < 1e-6);
        assert_eq!(placed[0].scale, [2.0; 3]);
        let o = m.objects.iter().find(|o| o.name == "props").unwrap();
        assert_eq!(o.tris.len(), 2);
        assert!(o.verts.iter().all(|v| (v[2] - 150.0).abs() < 1e-6), "the top is 2 x 50 above the base");
        // turned a quarter: its +x corner (50, -50) went to (+100, +100) from the origin, scaled 2
        assert!(o.verts.iter().any(|v| (v[0] - 300.0).abs() < 1e-6 && (v[1] - 100.0).abs() < 1e-6));
        assert_eq!(o.tints[0], [1.0, 0.5, 1.0]);
        // its collision is collision only, and counts
        let c = m.objects.iter().find(|o| o.name == "props_collision").unwrap();
        assert!(c.collision_only && c.tris.len() == 2);
        assert_eq!(m.surfaces.last().unwrap(), "wood");
        assert_eq!(m.triangles(), 2 + 2 + 2, "ground, water, the prop: not its collision");
        // the 4 ground corners and the prop's 4 top corners
        assert_eq!(m.collision_vertices(), 8);
        // uneven: 90 west of the origin (the footprint's corners, a little in) the ground is 22.5 lower
        assert!(problems.iter().any(|p| p.contains("falls")), "{problems:?}");
        assert_eq!(pick(&placed, [200.0, 90.0]), Some(0));
        assert_eq!(pick(&placed, [200.0, 110.0]), None);
    }

    #[test]
    fn houses_stand_on_their_doorway_and_stones_in_water() {
        let kit = Kit { pieces: vec![test_piece("house"), Piece { name: "stone".into(), ..test_piece("stone") }], ..Default::default() };
        let mut m = slope_mesh();
        // facing east (yaw -90): the door is 50 east of the origin, where the ground is 12.5 up
        let props = vec![
            Prop { piece: "box".into(), at: [0.0, -500.0], z: None, yaw: -90.0, scale: [1.0; 3] },
            Prop { piece: "stone".into(), at: [450.0, 450.0], z: None, yaw: 0.0, scale: [1.0; 3] },
            Prop { piece: "nothing".into(), at: [0.0, 0.0], z: Some(3.0), yaw: 0.0, scale: [1.0; 3] },
        ];
        let mut problems = vec![];
        let placed = place(&props, Some(&kit), &mut m, &mut problems);
        assert!((placed[0].origin[2] - 12.5).abs() < 1e-6, "{:?}", placed[0].origin);
        assert!((placed[1].origin[2] - (-50.0 + STONE_ABOVE_WATER)).abs() < 1e-6);
        assert_eq!(placed.len(), 2);
        assert!(problems.iter().any(|p| p.contains("no piece \"nothing\"")));
        // no kit: reported, nothing placed
        let mut problems = vec![];
        assert!(place(&props, None, &mut slope_mesh(), &mut problems).is_empty());
        assert!(problems[0].contains("no kit"));
    }
}
