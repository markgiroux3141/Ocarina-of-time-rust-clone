//! The plan view: a camera looking straight down (x east, y north), and the built level drawn
//! from above, textured with its baked lighting or coloured by height. Triangles are drawn
//! lowest first, so a bridge covers the river under it.

use eframe::egui::{self, epaint, Color32, Pos2, Rect, TextureId};
use overworld::geom::P2;
use overworld::textures::{Library, TexInfo};
use overworld::{Level, Theme};

#[derive(Clone, Copy, Debug)]
pub struct View {
    pub center: P2,
    /// Pixels per unit.
    pub scale: f64,
    pub rect: Rect,
}

impl View {
    pub fn to_screen(&self, p: P2) -> Pos2 {
        let c = self.rect.center();
        Pos2::new(c.x + ((p[0] - self.center[0]) * self.scale) as f32, c.y - ((p[1] - self.center[1]) * self.scale) as f32)
    }

    pub fn to_world(&self, s: Pos2) -> P2 {
        let c = self.rect.center();
        [self.center[0] + (s.x - c.x) as f64 / self.scale, self.center[1] - (s.y - c.y) as f64 / self.scale]
    }

    /// Zoom by `f` keeping the world point under `s` where it is.
    pub fn zoom_at(&mut self, s: Pos2, f: f64) {
        let before = self.to_world(s);
        self.scale = (self.scale * f).clamp(0.005, 20.0);
        let after = self.to_world(s);
        self.center[0] += before[0] - after[0];
        self.center[1] += before[1] - after[1];
    }

    /// Fit a box (x0, y0, x1, y1) in the view, with a margin.
    pub fn fit(&mut self, bb: [f64; 4]) {
        let (w, h) = ((bb[2] - bb[0]).max(1.0), (bb[3] - bb[1]).max(1.0));
        self.center = [0.5 * (bb[0] + bb[2]), 0.5 * (bb[1] + bb[3])];
        self.scale = ((self.rect.width() as f64 / w).min(self.rect.height() as f64 / h) * 0.9).clamp(0.005, 20.0);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shading {
    Textured,
    Height,
    Off,
}

struct Tri {
    p: [[f32; 3]; 3],
    uv: [[f32; 2]; 3],
    c: [[u8; 3]; 3],
    mat: u16,
    bb: [f32; 4],
}

pub struct Mat {
    pub role: String,
    /// The texture's name in the library (derived ones too: a wall's middle rows).
    pub name: String,
    pub info: TexInfo,
    /// A blend material's second texture (the ground with dirt over it), drawn by vertex weight.
    pub overlay: Option<String>,
}

/// The mean colour of a library texture (0-1).
fn mean_colour(lib: &Library, name: &str) -> Option<[f64; 3]> {
    let (_, _, px) = lib.rgba(name)?;
    let n = (px.len() / 4).max(1) as f64;
    let mut c = [0.0; 3];
    for q in px.chunks(4) {
        for k in 0..3 {
            c[k] += q[k] as f64 / 255.0;
        }
    }
    Some(c.map(|x| x / n))
}

pub struct Scene {
    tris: Vec<Tri>,
    pub mats: Vec<Mat>,
    zr: (f32, f32),
    /// Collision floors (up-facing), for heights under the cursor: triangles in grid cells.
    floors: Vec<[[f64; 3]; 3]>,
    /// The walls and cliffs (drawn, vertical), for putting wall pieces on them in 3D.
    walls: Vec<[[f64; 3]; 3]>,
    cells: std::collections::HashMap<(i32, i32), Vec<u32>>,
    pub bounds: [f64; 4],
    pub triangles: usize,
    pub rim: (f64, f64),
}

const CELL: f64 = 256.0;

impl Scene {
    pub fn new(lvl: &Level, theme: &Theme, lib: Option<&Library>) -> Scene {
        let mats: Vec<Mat> = lvl
            .mesh
            .materials
            .iter()
            .map(|role| {
                let name = theme.texture_name(role);
                let info = lib.map(|l| l.get(&name)).unwrap_or_else(|| TexInfo::plain(&name));
                Mat { role: role.clone(), name, info, overlay: theme.overlay_texture(role) }
            })
            .collect();
        // seen from above, a blend shows its second texture as a tint of the first: their mean
        // colours' ratio, by each vertex's weight
        let tints: Vec<Option<[f64; 3]>> = mats
            .iter()
            .map(|m| {
                let (o, l) = (m.overlay.as_ref()?, lib?);
                let (a, b) = (mean_colour(l, &m.name)?, mean_colour(l, o)?);
                Some([0, 1, 2].map(|k| b[k] / a[k].max(1e-3)))
            })
            .collect();
        let (mut tris, mut floors, mut walls) = (vec![], vec![], vec![]);
        let mut cells: std::collections::HashMap<(i32, i32), Vec<u32>> = Default::default();
        let mut bounds = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
        let mut zr = (f32::INFINITY, f32::NEG_INFINITY);
        for o in &lvl.mesh.objects {
            for (t, tri) in o.tris.iter().enumerate() {
                let p = tri.map(|v| o.verts[v]);
                let a = [p[1][0] - p[0][0], p[1][1] - p[0][1]];
                let b = [p[2][0] - p[0][0], p[2][1] - p[0][1]];
                let up = a[0] * b[1] - a[1] * b[0];
                for q in &p {
                    bounds = [bounds[0].min(q[0]), bounds[1].min(q[1]), bounds[2].max(q[0]), bounds[3].max(q[1])];
                }
                if up.abs() < 1e-3 {
                    if !o.collision_only && (o.name == "walls" || o.name == "cliffs") {
                        walls.push(p);
                    }
                    continue; // vertical: nothing to see from above
                }
                // collision floors (props' too, though their collision isn't drawn)
                if o.surf[t] >= 0 && up > 0.0 {
                    let i = floors.len() as u32;
                    floors.push(p);
                    let (x0, x1) = (p.iter().map(|q| q[0]).fold(f64::INFINITY, f64::min), p.iter().map(|q| q[0]).fold(f64::NEG_INFINITY, f64::max));
                    let (y0, y1) = (p.iter().map(|q| q[1]).fold(f64::INFINITY, f64::min), p.iter().map(|q| q[1]).fold(f64::NEG_INFINITY, f64::max));
                    for cx in (x0 / CELL).floor() as i32..=(x1 / CELL).floor() as i32 {
                        for cy in (y0 / CELL).floor() as i32..=(y1 / CELL).floor() as i32 {
                            cells.entry((cx, cy)).or_default().push(i);
                        }
                    }
                }
                if o.collision_only {
                    continue;
                }
                let mut c = tri.map(|v| o.colors.get(v).copied().unwrap_or([255, 255, 255]));
                if let (Some(Some(r)), false) = (tints.get(o.mat[t]), o.blend.is_empty()) {
                    for (k, &v) in tri.iter().enumerate() {
                        let w = o.blend[v];
                        c[k] = [0, 1, 2].map(|j| (c[k][j] as f64 * (1.0 + (r[j] - 1.0) * w)).round().clamp(0.0, 255.0) as u8);
                    }
                }
                let pf = p.map(|q| [q[0] as f32, q[1] as f32, q[2] as f32]);
                if o.name == "ground" {
                    for q in &pf {
                        zr = (zr.0.min(q[2]), zr.1.max(q[2]));
                    }
                }
                let bb = [
                    pf.iter().map(|q| q[0]).fold(f32::INFINITY, f32::min),
                    pf.iter().map(|q| q[1]).fold(f32::INFINITY, f32::min),
                    pf.iter().map(|q| q[0]).fold(f32::NEG_INFINITY, f32::max),
                    pf.iter().map(|q| q[1]).fold(f32::NEG_INFINITY, f32::max),
                ];
                tris.push(Tri { p: pf, uv: o.uvs[t].map(|u| [u[0] as f32, u[1] as f32]), c, mat: o.mat[t] as u16, bb });
            }
        }
        // lowest first; on ties keep the builder's order
        tris.sort_by(|a, b| {
            let z = |t: &Tri| t.p.iter().map(|q| q[2]).sum::<f32>();
            z(a).total_cmp(&z(b))
        });
        // heights are coloured over the floors' range (the bank and rim above it are the top colour)
        if !zr.0.is_finite() || zr.1 - zr.0 < 1.0 {
            let b = if zr.0.is_finite() { zr.0 } else { 0.0 };
            zr = (b, b + 100.0);
        }
        Scene { tris, mats, zr, floors, walls, cells, bounds, triangles: lvl.mesh.triangles(), rim: lvl.rim }
    }

    /// The highest collision floor at p.
    pub fn floor_z(&self, p: P2) -> Option<f64> {
        let key = ((p[0] / CELL).floor() as i32, (p[1] / CELL).floor() as i32);
        let mut best: Option<f64> = None;
        for &i in self.cells.get(&key)? {
            let [a, b, c] = self.floors[i as usize];
            let d = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
            if d.abs() < 1e-9 {
                continue;
            }
            let l1 = ((b[1] - c[1]) * (p[0] - c[0]) + (c[0] - b[0]) * (p[1] - c[1])) / d;
            let l2 = ((c[1] - a[1]) * (p[0] - c[0]) + (a[0] - c[0]) * (p[1] - c[1])) / d;
            let l3 = 1.0 - l1 - l2;
            if l1 >= -1e-6 && l2 >= -1e-6 && l3 >= -1e-6 {
                let z = l1 * a[2] + l2 * b[2] + l3 * c[2];
                best = Some(best.map_or(z, |m: f64| m.max(z)));
            }
        }
        best
    }

    /// The nearest collision floor a ray from `o` along `d` hits: (distance along d, point).
    pub fn raycast(&self, o: [f64; 3], d: [f64; 3]) -> Option<(f64, [f64; 3])> {
        let best = self.floors.iter().filter_map(|t| ray_tri(o, d, t)).fold(None, |b: Option<f64>, t| Some(b.map_or(t, |b| b.min(t))));
        best.map(|t| (t, [o[0] + d[0] * t, o[1] + d[1] * t, o[2] + d[2] * t]))
    }

    /// The nearest wall or cliff a ray hits: (distance along d, point, the wall's normal seen from
    /// above, turned to face the ray's origin).
    pub fn raycast_wall(&self, o: [f64; 3], d: [f64; 3]) -> Option<(f64, [f64; 3], P2)> {
        let mut best: Option<(f64, usize)> = None;
        for (i, tri) in self.walls.iter().enumerate() {
            if let Some(t) = ray_tri(o, d, tri) {
                if best.is_none_or(|b| t < b.0) {
                    best = Some((t, i));
                }
            }
        }
        let (t, i) = best?;
        let [a, b, c] = self.walls[i];
        let n = [(b[1] - a[1]) * (c[2] - a[2]) - (b[2] - a[2]) * (c[1] - a[1]), (b[2] - a[2]) * (c[0] - a[0]) - (b[0] - a[0]) * (c[2] - a[2])];
        let l = n[0].hypot(n[1]).max(1e-9);
        let mut n = [n[0] / l, n[1] / l];
        if n[0] * d[0] + n[1] * d[1] > 0.0 {
            n = [-n[0], -n[1]];
        }
        Some((t, [o[0] + d[0] * t, o[1] + d[1] * t, o[2] + d[2] * t], n))
    }

    /// Draws the level. `tex(mat)` gives a material's texture, if it's loaded.
    pub fn paint(&self, painter: &egui::Painter, view: &View, shading: Shading, tex: &dyn Fn(usize) -> Option<TextureId>) {
        if shading == Shading::Off {
            return;
        }
        let w0 = view.to_world(view.rect.left_bottom());
        let w1 = view.to_world(view.rect.right_top());
        let mut batch: Option<(TextureId, epaint::Mesh)> = None;
        let flush = |b: &mut Option<(TextureId, epaint::Mesh)>| {
            if let Some((_, m)) = b.take() {
                painter.add(egui::Shape::mesh(m));
            }
        };
        let (z0, z1) = self.zr;
        for t in &self.tris {
            if (t.bb[2] as f64) < w0[0] || (t.bb[0] as f64) > w1[0] || (t.bb[3] as f64) < w0[1] || (t.bb[1] as f64) > w1[1] {
                continue;
            }
            let m = &self.mats[t.mat as usize];
            let tid = if shading == Shading::Textured { tex(t.mat as usize) } else { None };
            let id = tid.unwrap_or_default();
            if batch.as_ref().is_none_or(|b| b.0 != id || b.1.vertices.len() > 60_000) {
                flush(&mut batch);
                batch = Some((id, epaint::Mesh::with_texture(id)));
            }
            let mesh = &mut batch.as_mut().unwrap().1;
            let alpha = if m.info.alpha == "blend" { (m.info.opacity * 255.0) as u8 } else { 255 };
            let base = mesh.vertices.len() as u32;
            for k in 0..3 {
                let p = t.p[k];
                let c = t.c[k];
                let h = ((p[2] - z0) / (z1 - z0)).clamp(0.0, 1.0);
                let color = match (shading, tid) {
                    (Shading::Textured, Some(_)) => {
                        // from above, every floor is the same grass: higher ground is drawn lighter
                        let f = 0.6 + 0.65 * h;
                        let s = |x: u8| (x as f32 * f).min(255.0) as u8;
                        Color32::from_rgba_unmultiplied(s(c[0]), s(c[1]), s(c[2]), alpha)
                    }
                    _ => {
                        let shade = (c[0] as f32 * 0.3 + c[1] as f32 * 0.59 + c[2] as f32 * 0.11) / 160.0;
                        let col = if m.role.contains("water") { [70.0, 120.0, 190.0] } else { height_colour(h) };
                        let s = shade.clamp(0.4, 1.4);
                        Color32::from_rgba_unmultiplied(
                            (col[0] * s).min(255.0) as u8,
                            (col[1] * s).min(255.0) as u8,
                            (col[2] * s).min(255.0) as u8,
                            alpha,
                        )
                    }
                };
                let uv = if tid.is_some() { Pos2::new(t.uv[k][0], 1.0 - t.uv[k][1]) } else { epaint::WHITE_UV };
                mesh.vertices.push(epaint::Vertex { pos: view.to_screen([p[0] as f64, p[1] as f64]), uv, color });
            }
            mesh.indices.extend_from_slice(&[base, base + 1, base + 2]);
        }
        flush(&mut batch);
    }
}

/// Where a ray from `o` along `d` meets triangle `tri` (Moller-Trumbore): its distance along d.
fn ray_tri(o: [f64; 3], d: [f64; 3], tri: &[[f64; 3]; 3]) -> Option<f64> {
    let sub = |a: [f64; 3], b: [f64; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let cross = |a: [f64; 3], b: [f64; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let [a, b, c] = *tri;
    let (e1, e2) = (sub(b, a), sub(c, a));
    let p = cross(d, e2);
    let det = dot(e1, p);
    if det.abs() < 1e-12 {
        return None;
    }
    let t0 = sub(o, a);
    let u = dot(t0, p) / det;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = cross(t0, e1);
    let v = dot(d, q) / det;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = dot(e2, q) / det;
    (t > 0.0).then_some(t)
}

/// Low ground dark green, through grass and sand, to pale rock at the top.
fn height_colour(h: f32) -> [f32; 3] {
    const STOPS: [(f32, [f32; 3]); 5] = [
        (0.0, [38.0, 70.0, 48.0]),
        (0.3, [76.0, 120.0, 62.0]),
        (0.55, [150.0, 160.0, 90.0]),
        (0.8, [176.0, 150.0, 112.0]),
        (1.0, [222.0, 214.0, 200.0]),
    ];
    let h = h.clamp(0.0, 1.0);
    for w in STOPS.windows(2) {
        if h <= w[1].0 {
            let t = (h - w[0].0) / (w[1].0 - w[0].0);
            return [0, 1, 2].map(|i| w[0].1[i] + (w[1].1[i] - w[0].1[i]) * t);
        }
    }
    STOPS[4].1
}

/// A material's texture options: linear, wrapped as the texture library says.
pub fn texture_options(info: &TexInfo) -> egui::TextureOptions {
    let wrap = match (info.wrap_u.as_str(), info.wrap_v.as_str()) {
        ("clamp", _) | (_, "clamp") => egui::TextureWrapMode::ClampToEdge,
        ("mirror", _) | (_, "mirror") => egui::TextureWrapMode::MirroredRepeat,
        _ => egui::TextureWrapMode::Repeat,
    };
    egui::TextureOptions { magnification: egui::TextureFilter::Linear, minification: egui::TextureFilter::Linear, wrap_mode: wrap, mipmap_mode: None }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ray across the level at knee height hits a wall (a region's or the edge's), and the
    /// wall's normal comes back facing where the ray came from; a ray straight down hits no wall.
    #[test]
    fn rays_hit_walls_facing_back() {
        let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../overworld/examples/sketch/sketch_plateau.json");
        let doc: overworld::Doc = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
        let theme = Theme::kokiri();
        let lvl = overworld::build(&doc, &theme).unwrap();
        let s = Scene::new(&lvl, &theme, None);
        let pts: Vec<P2> = doc.outline.nodes.iter().map(|n| [n[0], n[1]]).collect();
        let c = [pts.iter().map(|q| q[0]).sum::<f64>() / pts.len() as f64, pts.iter().map(|q| q[1]).sum::<f64>() / pts.len() as f64];
        for a in 0..8 {
            let a = a as f64 / 8.0 * std::f64::consts::TAU;
            let d = [a.cos(), a.sin(), 0.0];
            let (t, q, n) = s.raycast_wall([c[0], c[1], doc.outline.z + 50.0], d).expect("a wall all round");
            assert!(t > 0.0 && (q[2] - doc.outline.z - 50.0).abs() < 1e-6);
            assert!(n[0] * d[0] + n[1] * d[1] < 0.0 && (n[0].hypot(n[1]) - 1.0).abs() < 1e-9);
        }
        assert!(s.raycast_wall([c[0], c[1], 5000.0], [0.0, 0.0, -1.0]).is_none());
    }
}
