//! Thumbnails of the kit's pieces for the Kit panel: each piece drawn from the front and a little
//! to the side, from above, textured and lit, by a small software rasteriser (so they need no
//! GPU readback and come out the same everywhere). Made once per piece, when first shown.

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use overworld::pieces::Piece;
use overworld::textures::Library;
use std::collections::HashMap;

/// The thumbnail's side, in pixels (drawn at twice this and shrunk, for smooth edges).
pub const SIZE: usize = 112;

#[derive(Default)]
pub struct Thumbs {
    made: HashMap<String, Option<TextureHandle>>,
    /// Library textures, decoded once: (width, height, rgba, wrap u, wrap v, alpha mode, opacity).
    texels: HashMap<String, Option<Tex>>,
}

struct Tex {
    w: usize,
    h: usize,
    px: Vec<u8>,
    wrap: [u8; 2],
    cutout: bool,
    blend: Option<f32>,
}

impl Thumbs {
    /// The piece's thumbnail, made now if it hasn't been.
    pub fn get(&mut self, ctx: &egui::Context, piece: &Piece, lib: Option<&Library>) -> Option<egui::TextureId> {
        if !self.made.contains_key(&piece.name) {
            let img = self.render(piece, lib);
            let h = img.map(|img| ctx.load_texture(format!("thumb:{}", piece.name), img, TextureOptions::LINEAR));
            self.made.insert(piece.name.clone(), h);
        }
        self.made[&piece.name].as_ref().map(|h| h.id())
    }

    fn tex(&mut self, name: &str, lib: Option<&Library>) -> Option<&Tex> {
        if !self.texels.contains_key(name) {
            let t = lib.and_then(|l| {
                let (w, h, px) = l.rgba(name)?;
                let info = l.info.get(name);
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
                    // translucent pieces (the waterfall) a little stronger than in the game, to read at this size
                    blend: (alpha == "blend").then(|| info.map_or(1.0, |i| i.opacity as f32).max(0.85)),
                })
            });
            self.texels.insert(name.to_string(), t);
        }
        self.texels[name].as_ref()
    }

    fn render(&mut self, piece: &Piece, lib: Option<&Library>) -> Option<ColorImage> {
        if piece.tris.is_empty() {
            return None;
        }
        let n = SIZE * 2;
        // the camera: in front (+y, where the piece faces) and to the right, looking down
        let (az, el) = (32f64.to_radians(), 24f64.to_radians());
        let eye = [az.sin() * el.cos(), az.cos() * el.cos(), el.sin()];
        let f = [-eye[0], -eye[1], -eye[2]];
        let r = norm(cross(f, [0.0, 0.0, 1.0]));
        let u = cross(r, f);
        let proj = |v: [f64; 3]| (dot(v, r), dot(v, u), dot(v, f));
        // fit everything in, a little in from the edges
        let pts: Vec<(f64, f64, f64)> = piece.verts.iter().map(|&v| proj(v)).collect();
        let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for p in &pts {
            x0 = x0.min(p.0);
            x1 = x1.max(p.0);
            y0 = y0.min(p.1);
            y1 = y1.max(p.1);
        }
        let span = (x1 - x0).max(y1 - y0).max(1e-3);
        let k = n as f64 * 0.84 / span;
        let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        let screen: Vec<[f64; 3]> = pts.iter().map(|p| [n as f64 / 2.0 + (p.0 - cx) * k, n as f64 / 2.0 - (p.1 - cy) * k, p.2]).collect();
        let light = norm([-0.45, 0.55, 0.75]);
        let mut col = vec![[0f32; 4]; n * n];
        let mut depth = vec![f64::INFINITY; n * n];
        // opaque and cut-out triangles first, then translucent ones over them
        for pass in 0..2 {
            for (t, tri) in piece.tris.iter().enumerate() {
                let m = piece.mat.get(t).copied().unwrap_or(0) as usize;
                let Some(mat) = piece.materials.get(m) else { continue };
                let tint = mat.tint.map(|c| c as f32);
                let tex = self.tex(&mat.texture, lib);
                let blend = tex.and_then(|t| t.blend);
                if (pass == 1) != blend.is_some() {
                    continue;
                }
                let idx = tri.map(|i| i as usize);
                let (a, b, c) = (screen[idx[0]], screen[idx[1]], screen[idx[2]]);
                // the face's light: its vertex normals averaged, or its own
                let nv = if piece.normals.len() == piece.verts.len() {
                    norm(add(add(piece.normals[idx[0]], piece.normals[idx[1]]), piece.normals[idx[2]]))
                } else {
                    let (p0, p1, p2) = (piece.verts[idx[0]], piece.verts[idx[1]], piece.verts[idx[2]]);
                    norm(cross(sub(p1, p0), sub(p2, p0)))
                };
                let shade = (0.5 + 0.55 * dot(nv, light).abs().max(dot(nv, light))) as f32;
                let uv = piece.uvs.get(t).copied().unwrap_or([[0.0; 2]; 3]);
                let area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
                if area.abs() < 1e-9 {
                    continue;
                }
                let minx = a[0].min(b[0]).min(c[0]).floor().max(0.0) as usize;
                let maxx = (a[0].max(b[0]).max(c[0]).ceil() as usize).min(n - 1);
                let miny = a[1].min(b[1]).min(c[1]).floor().max(0.0) as usize;
                let maxy = (a[1].max(b[1]).max(c[1]).ceil() as usize).min(n - 1);
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
                        let i = y * n + x;
                        if z >= depth[i] {
                            continue;
                        }
                        let (s, tt) = (w0 * uv[0][0] + w1 * uv[1][0] + w2 * uv[2][0], w0 * uv[0][1] + w1 * uv[1][1] + w2 * uv[2][1]);
                        let texel = tex.map_or([0.75, 0.75, 0.75, 1.0], |t| sample(t, s, 1.0 - tt));
                        if tex.is_some_and(|t| t.cutout) && texel[3] < 0.5 {
                            continue;
                        }
                        let rgb = [texel[0] * tint[0] * shade, texel[1] * tint[1] * shade, texel[2] * tint[2] * shade];
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
        // two by two into one, premultiplied
        let mut out = ColorImage::new([SIZE, SIZE], vec![egui::Color32::TRANSPARENT; SIZE * SIZE]);
        for y in 0..SIZE {
            for x in 0..SIZE {
                let mut acc = [0f32; 4];
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let c = col[(y * 2 + dy) * n + x * 2 + dx];
                    for ch in 0..3 {
                        acc[ch] += c[ch].min(1.0) * c[3];
                    }
                    acc[3] += c[3];
                }
                let a = acc[3] / 4.0;
                let to = |v: f32| (v / 4.0 * 255.0).round().clamp(0.0, 255.0) as u8;
                out.pixels[y * SIZE + x] = egui::Color32::from_rgba_premultiplied(to(acc[0]), to(acc[1]), to(acc[2]), (a * 255.0).round() as u8);
            }
        }
        Some(out)
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
