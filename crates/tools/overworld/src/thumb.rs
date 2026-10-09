//! Pictures of kit pieces (and of whole scenes, for `kit-survey`): a small software rasteriser,
//! textured from the texture library and lit (or shaded by the piece's vertex colours, as its
//! scene does), so the pictures need no GPU and come out the same everywhere. The editor's Kit
//! panel thumbnails are these; `kit-pieces` writes them next to the kit.

use crate::pieces::Piece;
use crate::textures::Library;
use std::collections::HashMap;

/// An image, straight (not premultiplied) RGBA.
pub struct Image {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u8>,
}

/// Where the picture is seen from.
#[derive(Clone, Copy, Debug)]
pub enum View {
    /// In front (+y, where a piece faces) and to the right, looking down: the Kit panel's.
    Oblique,
    /// From `az` degrees clockwise from south (the front), `el` degrees above the horizon,
    /// everything fitted in.
    Angle { az: f64, el: f64 },
    /// Straight down, north up, `scale` pixels per unit from the world point `min` (x west, y
    /// north edge) at the image's top-left: a plan. The image is the size asked for.
    Top { min: [f64; 2], scale: f64 },
}

/// Library textures, decoded once.
#[derive(Default)]
pub struct Texels {
    made: HashMap<String, Option<Tex>>,
}

struct Tex {
    w: usize,
    h: usize,
    px: Vec<u8>,
    wrap: [u8; 2],
    cutout: bool,
    blend: Option<f32>,
}

impl Texels {
    fn get(&mut self, name: &str, lib: Option<&Library>) -> Option<&Tex> {
        if !self.made.contains_key(name) {
            let t = lib.and_then(|l| {
                let (w, h, px) = l.rgba(name)?;
                // derived textures (`@swap`, a wall's middle rows...) aren't listed: made like their base
                let info = Some(l.get(name));
                let info = info.as_ref();
                let wrap = |s: Option<&String>| match s.map(|s| s.as_str()) {
                    Some("clamp") => 1,
                    Some("mirror") => 2,
                    _ => 0,
                };
                let alpha = info.map_or("opaque", |i| i.alpha.as_str());
                Some(Tex {
                    w: w as usize,
                    h: h as usize,
                    px,
                    wrap: [wrap(info.map(|i| &i.wrap_u)), wrap(info.map(|i| &i.wrap_v))],
                    cutout: alpha == "cutout",
                    // translucent pieces (the waterfall) a little stronger than in the game, to read small
                    blend: (alpha == "blend").then(|| info.map_or(1.0, |i| i.opacity as f32).max(0.85)),
                })
            });
            self.made.insert(name.to_string(), t);
        }
        self.made[name].as_ref()
    }
}

/// The piece drawn `size` pixels square from the Kit panel's angle (drawn at twice that and
/// shrunk, for smooth edges). None if it has no triangles.
pub fn render(piece: &Piece, lib: Option<&Library>, texels: &mut Texels, size: usize) -> Option<Image> {
    render_view(piece, lib, texels, size, size, View::Oblique)
}

/// The piece drawn `w` x `h` pixels from `view` (drawn at twice that and shrunk).
pub fn render_view(piece: &Piece, lib: Option<&Library>, texels: &mut Texels, w: usize, h: usize, view: View) -> Option<Image> {
    if piece.tris.is_empty() {
        return None;
    }
    let (nw, nh) = (w * 2, h * 2);
    let (r, u, f) = match view {
        View::Oblique | View::Angle { .. } => {
            let (az, el) = match view {
                View::Angle { az, el } => (az.to_radians(), el.to_radians()),
                _ => (32f64.to_radians(), 24f64.to_radians()),
            };
            let eye = [az.sin() * el.cos(), az.cos() * el.cos(), el.sin()];
            let f = [-eye[0], -eye[1], -eye[2]];
            let r = norm(cross(f, [0.0, 0.0, 1.0]));
            (r, cross(r, f), f)
        }
        View::Top { .. } => ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]),
    };
    let proj = |v: [f64; 3]| (dot(v, r), dot(v, u), dot(v, f));
    let pts: Vec<(f64, f64, f64)> = piece.verts.iter().map(|&v| proj(v)).collect();
    let screen: Vec<[f64; 3]> = match view {
        View::Oblique | View::Angle { .. } => {
            // fit everything in, a little in from the edges
            let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
            for p in &pts {
                x0 = x0.min(p.0);
                x1 = x1.max(p.0);
                y0 = y0.min(p.1);
                y1 = y1.max(p.1);
            }
            let k = (nw.min(nh)) as f64 * 0.84 / (x1 - x0).max(y1 - y0).max(1e-3);
            let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
            pts.iter().map(|p| [nw as f64 / 2.0 + (p.0 - cx) * k, nh as f64 / 2.0 - (p.1 - cy) * k, p.2]).collect()
        }
        View::Top { min, scale } => pts.iter().map(|p| [(p.0 - min[0]) * scale * 2.0, (min[1] - p.1) * scale * 2.0, p.2]).collect(),
    };
    let light = norm([-0.45, 0.55, 0.75]);
    let mut col = vec![[0f32; 4]; nw * nh];
    let mut depth = vec![f64::INFINITY; nw * nh];
    // opaque and cut-out triangles first, then translucent ones over them
    for pass in 0..2 {
        for (t, tri) in piece.tris.iter().enumerate() {
            let m = piece.mat.get(t).copied().unwrap_or(0) as usize;
            let Some(mat) = piece.materials.get(m) else { continue };
            let tint = mat.tint.map(|c| c as f32);
            let tex = texels.get(&mat.texture, lib);
            let blend = tex.and_then(|t| t.blend);
            if (pass == 1) != blend.is_some() {
                continue;
            }
            let idx = tri.map(|i| i as usize);
            let (a, b, c) = (screen[idx[0]], screen[idx[1]], screen[idx[2]]);
            // the face's light: its vertex normals averaged, or its own; or its vertex colours
            let shade: [f32; 3] = if mat.vertex_colors && piece.colors.len() == piece.verts.len() {
                let s = idx.iter().fold([0.0; 3], |acc, &i| add(acc, piece.colors[i]));
                // a little brighter: the scene's colours assume its own lights and fog
                s.map(|x| (x / 3.0 * 1.15) as f32)
            } else {
                let nv = if piece.normals.len() == piece.verts.len() {
                    norm(add(add(piece.normals[idx[0]], piece.normals[idx[1]]), piece.normals[idx[2]]))
                } else {
                    let (p0, p1, p2) = (piece.verts[idx[0]], piece.verts[idx[1]], piece.verts[idx[2]]);
                    norm(cross(sub(p1, p0), sub(p2, p0)))
                };
                [(0.5 + 0.55 * dot(nv, light).abs().max(dot(nv, light))) as f32; 3]
            };
            let uv = piece.uvs.get(t).copied().unwrap_or([[0.0; 2]; 3]);
            let area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
            if area.abs() < 1e-9 {
                continue;
            }
            let minx = a[0].min(b[0]).min(c[0]).floor().max(0.0) as usize;
            let maxx = (a[0].max(b[0]).max(c[0]).ceil().max(0.0) as usize).min(nw - 1);
            let miny = a[1].min(b[1]).min(c[1]).floor().max(0.0) as usize;
            let maxy = (a[1].max(b[1]).max(c[1]).ceil().max(0.0) as usize).min(nh - 1);
            if minx > maxx || miny > maxy {
                continue;
            }
            for y in miny..=maxy {
                for x in minx..=maxx {
                    let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
                    let w0 = ((b[0] - px) * (c[1] - py) - (b[1] - py) * (c[0] - px)) / area;
                    let w1 = ((c[0] - px) * (a[1] - py) - (c[1] - py) * (a[0] - px)) / area;
                    let w2 = 1.0 - w0 - w1;
                    if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                        continue;
                    }
                    let z = w0 * a[2] + w1 * b[2] + w2 * c[2];
                    let i = y * nw + x;
                    if z >= depth[i] {
                        continue;
                    }
                    let (s, tt) = (w0 * uv[0][0] + w1 * uv[1][0] + w2 * uv[2][0], w0 * uv[0][1] + w1 * uv[1][1] + w2 * uv[2][1]);
                    let texel = tex.map_or([0.75, 0.75, 0.75, 1.0], |t| sample(t, s, 1.0 - tt));
                    if tex.is_some_and(|t| t.cutout) && texel[3] < 0.5 {
                        continue;
                    }
                    let rgb = [0, 1, 2].map(|ch| texel[ch] * tint[ch] * shade[ch]);
                    match blend {
                        Some(op) => {
                            let al = (texel[3] * op).clamp(0.0, 1.0);
                            let d = &mut col[i];
                            for ch in 0..3 {
                                d[ch] = d[ch] * (1.0 - al) + rgb[ch] * al;
                            }
                            d[3] = d[3] + (1.0 - d[3]) * al;
                        }
                        None => {
                            col[i] = [rgb[0], rgb[1], rgb[2], 1.0];
                            depth[i] = z;
                        }
                    }
                }
            }
        }
    }
    // two by two into one
    let mut px = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0f32; 4];
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let c = col[(y * 2 + dy) * nw + x * 2 + dx];
                for ch in 0..3 {
                    acc[ch] += c[ch].min(1.0) * c[3];
                }
                acc[3] += c[3];
            }
            let o = (y * w + x) * 4;
            let a = acc[3] / 4.0;
            for ch in 0..3 {
                // straight colour: the coverage-weighted mean
                px[o + ch] = if acc[3] > 0.0 { (acc[ch] / acc[3] * 255.0).round().clamp(0.0, 255.0) as u8 } else { 0 };
            }
            px[o + 3] = (a * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
    Some(Image { w, h, px })
}

/// A built level as one piece, for pictures: its drawn objects' triangles with their textures (as
/// the export names them) and baked shades (or lit by their normals if it has none).
pub fn level_piece(lvl: &crate::Level, theme: &crate::Theme) -> Piece {
    let mut p = Piece::default();
    for o in lvl.mesh.objects.iter().filter(|o| !o.collision_only) {
        let shaded = o.colors.len() == o.verts.len();
        let base = p.verts.len() as u32;
        for (i, v) in o.verts.iter().enumerate() {
            p.verts.push(*v);
            p.normals.push(o.normals.get(i).copied().unwrap_or([0.0, 0.0, 1.0]));
            p.colors.push(if shaded { o.colors[i].map(|c| c as f64 / 255.0) } else { [1.0; 3] });
        }
        for (t, tri) in o.tris.iter().enumerate() {
            let role = &lvl.mesh.materials[o.mat[t]];
            // a blend (the ground under a dirt path) as its second texture where that mostly shows
            let blended = o.blend.len() == o.verts.len() && tri.iter().map(|&v| o.blend[v]).sum::<f64>() > 1.5;
            let name = match theme.overlay_texture(role).filter(|_| blended) {
                Some(over) => over,
                None => theme.texture_name(role),
            };
            let m = match p.materials.iter().position(|m| m.texture == name && m.vertex_colors == shaded) {
                Some(i) => i as u32,
                None => {
                    p.materials.push(crate::pieces::PieceMaterial { texture: name, tint: [1.0; 3], vertex_colors: shaded });
                    p.materials.len() as u32 - 1
                }
            };
            p.tris.push(tri.map(|v| base + v as u32));
            p.uvs.push(o.uvs[t]);
            p.mat.push(m);
        }
    }
    // pieces lit by normals need them per vertex: the faces' own where the mesh has none
    if p.normals.iter().all(|n| *n == [0.0, 0.0, 1.0]) {
        p.normals.clear();
    }
    p
}

impl Image {
    /// A blank image of one colour.
    pub fn filled(w: usize, h: usize, rgba: [u8; 4]) -> Image {
        Image { w, h, px: rgba.iter().copied().cycle().take(w * h * 4).collect() }
    }

    /// `other` drawn over this one with its top-left at (x, y), alpha blended.
    pub fn draw(&mut self, other: &Image, x: i64, y: i64) {
        for oy in 0..other.h {
            for ox in 0..other.w {
                let (tx, ty) = (x + ox as i64, y + oy as i64);
                if tx < 0 || ty < 0 || tx >= self.w as i64 || ty >= self.h as i64 {
                    continue;
                }
                let s = &other.px[(oy * other.w + ox) * 4..][..4];
                self.blend_px(tx as usize, ty as usize, [s[0], s[1], s[2], s[3]]);
            }
        }
    }

    pub fn blend_px(&mut self, x: usize, y: usize, c: [u8; 4]) {
        if x >= self.w || y >= self.h {
            return;
        }
        let d = &mut self.px[(y * self.w + x) * 4..][..4];
        let a = c[3] as f32 / 255.0;
        for ch in 0..3 {
            d[ch] = (d[ch] as f32 * (1.0 - a) + c[ch] as f32 * a).round() as u8;
        }
        d[3] = (d[3] as f32 + (255.0 - d[3] as f32) * a).round() as u8;
    }

    /// A rectangle's outline, `t` pixels thick.
    pub fn rect(&mut self, x0: i64, y0: i64, x1: i64, y1: i64, t: i64, c: [u8; 4]) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                let edge = x - x0 < t || x1 - x < t || y - y0 < t || y1 - y < t;
                if edge && x >= 0 && y >= 0 {
                    self.blend_px(x as usize, y as usize, c);
                }
            }
        }
    }

    /// A filled rectangle.
    pub fn fill(&mut self, x0: i64, y0: i64, x1: i64, y1: i64, c: [u8; 4]) {
        for y in y0.max(0)..=y1 {
            for x in x0.max(0)..=x1 {
                self.blend_px(x as usize, y as usize, c);
            }
        }
    }

    /// Text in a 3 x 5 pixel font (digits, capitals, a few marks), `k` times as big, on a dark
    /// box. Returns its width.
    pub fn text(&mut self, x: i64, y: i64, s: &str, k: i64, c: [u8; 4]) -> i64 {
        let n = s.chars().count() as i64;
        let wd = n * 4 * k + k;
        self.fill(x, y, x + wd - 1, y + 7 * k - 1, [0, 0, 0, 170]);
        for (i, ch) in s.chars().enumerate() {
            let g = glyph(ch);
            for (row, bits) in g.iter().enumerate() {
                for col in 0..3 {
                    if bits & (4 >> col) != 0 {
                        let (gx, gy) = (x + k + (i as i64 * 4 + col) * k, y + k + row as i64 * k);
                        self.fill(gx, gy, gx + k - 1, gy + k - 1, c);
                    }
                }
            }
        }
        wd
    }

    pub fn png(&self) -> Vec<u8> {
        crate::textures::encode_png(self.w as u32, self.h as u32, &self.px)
    }
}

/// A 3 x 5 glyph, rows top to bottom, bit 2 the left column.
fn glyph(c: char) -> [u8; 5] {
    match c.to_ascii_uppercase() {
        '0' => [7, 5, 5, 5, 7],
        '1' => [2, 6, 2, 2, 7],
        '2' => [7, 1, 7, 4, 7],
        '3' => [7, 1, 3, 1, 7],
        '4' => [5, 5, 7, 1, 1],
        '5' => [7, 4, 7, 1, 7],
        '6' => [7, 4, 7, 5, 7],
        '7' => [7, 1, 1, 2, 2],
        '8' => [7, 5, 7, 5, 7],
        '9' => [7, 5, 7, 1, 7],
        'A' => [2, 5, 7, 5, 5],
        'B' => [6, 5, 6, 5, 6],
        'C' => [3, 4, 4, 4, 3],
        'D' => [6, 5, 5, 5, 6],
        'E' => [7, 4, 6, 4, 7],
        'F' => [7, 4, 6, 4, 4],
        'G' => [3, 4, 5, 5, 3],
        'H' => [5, 5, 7, 5, 5],
        'I' => [7, 2, 2, 2, 7],
        'J' => [1, 1, 1, 5, 2],
        'K' => [5, 5, 6, 5, 5],
        'L' => [4, 4, 4, 4, 7],
        'M' => [5, 7, 7, 5, 5],
        'N' => [6, 5, 5, 5, 5],
        'O' => [2, 5, 5, 5, 2],
        'P' => [6, 5, 6, 4, 4],
        'Q' => [2, 5, 5, 6, 3],
        'R' => [6, 5, 6, 5, 5],
        'S' => [3, 4, 2, 1, 6],
        'T' => [7, 2, 2, 2, 2],
        'U' => [5, 5, 5, 5, 7],
        'V' => [5, 5, 5, 5, 2],
        'W' => [5, 5, 7, 7, 5],
        'X' => [5, 5, 2, 5, 5],
        'Y' => [5, 5, 2, 2, 2],
        'Z' => [7, 1, 2, 4, 7],
        '-' => [0, 0, 7, 0, 0],
        '_' => [0, 0, 0, 0, 7],
        '.' => [0, 0, 0, 0, 2],
        ':' => [0, 2, 0, 2, 0],
        '/' => [1, 1, 2, 4, 4],
        '#' => [5, 7, 5, 7, 5],
        _ => [0; 5],
    }
}

fn sample(t: &Tex, s: f64, v: f64) -> [f32; 4] {
    let wrap = |c: f64, n: usize, mode: u8| -> usize {
        let n = n as f64;
        let x = match mode {
            1 => c.clamp(0.0, 1.0) * n,
            2 => {
                let m = c.rem_euclid(2.0);
                if m > 1.0 {
                    (2.0 - m) * n
                } else {
                    m * n
                }
            }
            _ => c.rem_euclid(1.0) * n,
        };
        (x.floor() as usize).min(n as usize - 1)
    };
    let (x, y) = (wrap(s, t.w, t.wrap[0]), wrap(v, t.h, t.wrap[1]));
    let i = (y * t.w + x) * 4;
    [t.px[i] as f32 / 255.0, t.px[i + 1] as f32 / 255.0, t.px[i + 2] as f32 / 255.0, t.px[i + 3] as f32 / 255.0]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn norm(a: [f64; 3]) -> [f64; 3] {
    let l = dot(a, a).sqrt().max(1e-12);
    [a[0] / l, a[1] / l, a[2] / l]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_square_seen_from_above_fills_its_pixels() {
        let piece = Piece {
            verts: vec![[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [10.0, 10.0, 0.0], [0.0, 10.0, 0.0]],
            normals: vec![[0.0, 0.0, 1.0]; 4],
            tris: vec![[0, 1, 2], [0, 2, 3]],
            uvs: vec![[[0.0; 2]; 3]; 2],
            mat: vec![0, 0],
            materials: vec![crate::pieces::PieceMaterial { texture: "none".into(), tint: [1.0; 3], vertex_colors: false }],
            ..Default::default()
        };
        let img = render_view(&piece, None, &mut Texels::default(), 20, 20, View::Top { min: [0.0, 10.0], scale: 1.0 }).unwrap();
        assert_eq!(img.px[(5 * 20 + 5) * 4 + 3], 255, "inside the square");
        assert_eq!(img.px[(15 * 20 + 15) * 4 + 3], 0, "outside it");
        let mut sheet = Image::filled(40, 10, [255; 4]);
        assert_eq!(sheet.text(0, 0, "A1", 1, [255, 0, 0, 255]), 9);
    }
}
