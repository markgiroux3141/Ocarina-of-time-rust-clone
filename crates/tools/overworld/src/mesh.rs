//! Output meshes: named objects of triangles with per-corner UVs, a material role and a
//! collision surface role per triangle. Written as JSON (everything) and OBJ + MTL (any tool).

use crate::geom::P3;
use crate::textures::TexInfo;
use crate::noise::fbm3;
use crate::theme::{Light, Variation};
use serde_json::json;
use std::collections::HashMap;
use std::fmt::Write as _;

pub type UV = [f64; 2];

#[derive(Default)]
pub struct Object {
    pub name: String,
    pub verts: Vec<P3>,
    index: HashMap<(i64, i64, i64), usize>,
    pub tris: Vec<[usize; 3]>,
    pub uvs: Vec<[UV; 3]>,
    pub mat: Vec<usize>,
    /// Collision surface, or -1 for none (overlays, foliage, water surfaces are separate).
    pub surf: Vec<i64>,
    /// Baked lighting per vertex (0-255), empty if unlit.
    pub colors: Vec<[u8; 3]>,
}

#[derive(Default)]
pub struct Mesh {
    pub materials: Vec<String>,
    pub surfaces: Vec<String>,
    pub objects: Vec<Object>,
}

fn id(list: &mut Vec<String>, name: &str) -> usize {
    match list.iter().position(|m| m == name) {
        Some(i) => i,
        None => {
            list.push(name.to_string());
            list.len() - 1
        }
    }
}

impl Mesh {
    /// A triangle, counter-clockwise seen from its front. Degenerate ones are dropped.
    pub fn tri(&mut self, obj: &str, p: [P3; 3], uv: [UV; 3], mat: &str, surface: &str) {
        let (a, b) = ([p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]], [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]]);
        let n = [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
        if (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt() < 1e-4 {
            return;
        }
        let m = id(&mut self.materials, mat);
        let s = if surface.is_empty() { -1 } else { id(&mut self.surfaces, surface) as i64 };
        let oi = match self.objects.iter().position(|o| o.name == obj) {
            Some(i) => i,
            None => {
                self.objects.push(Object { name: obj.to_string(), ..Default::default() });
                self.objects.len() - 1
            }
        };
        let o = &mut self.objects[oi];
        let mut ids = [0; 3];
        for (k, q) in p.iter().enumerate() {
            let key = ((q[0] * 1000.0).round() as i64, (q[1] * 1000.0).round() as i64, (q[2] * 1000.0).round() as i64);
            ids[k] = *o.index.entry(key).or_insert_with(|| {
                o.verts.push(*q);
                o.verts.len() - 1
            });
        }
        if ids[0] == ids[1] || ids[1] == ids[2] || ids[0] == ids[2] {
            return;
        }
        o.tris.push(ids);
        o.uvs.push(uv);
        o.mat.push(m);
        o.surf.push(s);
    }

    /// A quad p0 p1 p2 p3 (counter-clockwise from the front) as two triangles.
    pub fn quad(&mut self, obj: &str, p: [P3; 4], uv: [UV; 4], mat: &str, surface: &str) {
        self.tri(obj, [p[0], p[1], p[2]], [uv[0], uv[1], uv[2]], mat, surface);
        self.tri(obj, [p[0], p[2], p[3]], [uv[0], uv[2], uv[3]], mat, surface);
    }

    /// Bake lighting into vertex colours as the N64 lights by normals: per-vertex normals (the
    /// area-weighted mean of the faces round each vertex, within its object, so a curved cliff
    /// shades smoothly), then ambient + sum of max(0, n . l) * colour per light.
    /// With `var`, each vertex's shade is also multiplied by 1 +- var.shade of smooth noise
    /// (not on water, whose surface is a flat sheet).
    pub fn shade(&mut self, light: &Light, var: Option<(&Variation, u32)>) {
        let dirs: Vec<([f64; 3], [f64; 3])> = light
            .lights
            .iter()
            .map(|l| {
                let n = (l.dir[0] * l.dir[0] + l.dir[1] * l.dir[1] + l.dir[2] * l.dir[2]).sqrt().max(1e-9);
                ([l.dir[0] / n, l.dir[1] / n, l.dir[2] / n], l.color)
            })
            .collect();
        for o in &mut self.objects {
            let vary = if o.name == "water" { None } else { var };
            let mut nrm = vec![[0.0f64; 3]; o.verts.len()];
            for t in &o.tris {
                let (a, b, c) = (o.verts[t[0]], o.verts[t[1]], o.verts[t[2]]);
                let (e, f) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
                let n = [e[1] * f[2] - e[2] * f[1], e[2] * f[0] - e[0] * f[2], e[0] * f[1] - e[1] * f[0]];
                for &v in t {
                    for k in 0..3 {
                        nrm[v][k] += n[k];
                    }
                }
            }
            let verts = &o.verts;
            o.colors = nrm
                .iter()
                .enumerate()
                .map(|(vi, n)| {
                    let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-9);
                    let mut c = light.ambient;
                    for (d, col) in &dirs {
                        let k = ((n[0] * d[0] + n[1] * d[1] + n[2] * d[2]) / l).max(0.0);
                        for i in 0..3 {
                            c[i] += k * col[i];
                        }
                    }
                    if let Some((v, seed)) = vary {
                        let p = verts[vi];
                        let k = 1.0 + v.shade * fbm3([p[0] / v.shade_scale, p[1] / v.shade_scale, p[2] / v.shade_scale], seed.wrapping_add(11));
                        c = c.map(|x| x * k);
                    }
                    c.map(|x| x.round().clamp(0.0, 255.0) as u8)
                })
                .collect();
        }
    }

    /// Moves every vertex up by `dz(x, y)`. Call it once the mesh is complete: the vertex index
    /// that welds new triangles' corners is keyed by the old positions.
    pub fn displace(&mut self, dz: impl Fn([f64; 2]) -> f64) {
        for o in &mut self.objects {
            for v in &mut o.verts {
                v[2] += dz([v[0], v[1]]);
            }
        }
    }

    pub fn triangles(&self) -> usize {
        self.objects.iter().map(|o| o.tris.len()).sum()
    }

    /// `tex(role)` gives a material role's texture name and how it's drawn.
    pub fn to_json(&self, tex: &dyn Fn(&str) -> (String, TexInfo)) -> serde_json::Value {
        let r = |x: f64| (x * 1000.0).round() / 1000.0;
        json!({
            "axes": "x east, y north, z up",
            "materials": self.materials.iter().map(|m| {
                let (name, info) = tex(m);
                json!({"name": m, "texture": name, "file": format!("textures/{}", info.file), "wrap_u": info.wrap_u,
                       "wrap_v": info.wrap_v, "alpha": info.alpha, "opacity": info.opacity, "cull": info.cull})
            }).collect::<Vec<_>>(),
            "surfaces": self.surfaces,
            "objects": self.objects.iter().map(|o| json!({
                "name": o.name,
                "verts": o.verts.iter().flat_map(|v| v.iter().map(|&x| r(x))).collect::<Vec<_>>(),
                "tris": o.tris.iter().flatten().collect::<Vec<_>>(),
                "uvs": o.uvs.iter().flatten().flat_map(|uv| uv.iter().map(|&x| (x * 1e5).round() / 1e5)).collect::<Vec<_>>(),
                "colors": o.colors.iter().flatten().collect::<Vec<_>>(),
                "material": o.mat,
                "surface": o.surf,
            })).collect::<Vec<_>>(),
        })
    }

    /// The OBJ name of a material: its role, plus the GE64 dialect's flags (pd-walk reads them).
    pub fn obj_material(role: &str, info: &TexInfo) -> String {
        let role = role.replace('~', "-");
        std::iter::once(role.as_str()).chain(info.flags()).collect::<Vec<_>>().join("_")
    }

    /// OBJ (y up, as most tools expect: obj = (x, z, -y)) and its MTL, with textures in
    /// `textures/` next to them.
    pub fn to_obj(&self, mtl_name: &str, tex: &dyn Fn(&str) -> (String, TexInfo)) -> (String, String) {
        let names: Vec<(String, TexInfo)> = self.materials.iter().map(|m| {
            let (_, info) = tex(m);
            (Self::obj_material(m, &info), info)
        }).collect();
        let mut obj = format!("# overworld level\nmtllib {mtl_name}\n");
        let (mut vbase, mut tbase) = (1, 1);
        for o in &self.objects {
            let _ = writeln!(obj, "o {}", o.name);
            for (i, v) in o.verts.iter().enumerate() {
                // colours twice: `v x y z r g b` for most tools, `#vcolor` (0-255) for the GE64
                // dialect (pd-walk)
                match o.colors.get(i) {
                    Some(c) => {
                        let _ = writeln!(
                            obj,
                            "v {:.3} {:.3} {:.3} {:.4} {:.4} {:.4}\n#vcolor {} {} {}",
                            v[0], v[2], -v[1], c[0] as f64 / 255.0, c[1] as f64 / 255.0, c[2] as f64 / 255.0, c[0], c[1], c[2]
                        );
                    }
                    None => {
                        let _ = writeln!(obj, "v {:.3} {:.3} {:.3}", v[0], v[2], -v[1]);
                    }
                }
            }
            for uv in o.uvs.iter().flatten() {
                let _ = writeln!(obj, "vt {:.5} {:.5}", uv[0], uv[1]);
            }
            let mut cur = usize::MAX;
            for (t, tri) in o.tris.iter().enumerate() {
                if o.mat[t] != cur {
                    cur = o.mat[t];
                    let _ = writeln!(obj, "usemtl {}", names[cur].0);
                }
                let _ = writeln!(
                    obj,
                    "f {}/{} {}/{} {}/{}",
                    tri[0] + vbase, tbase + 3 * t, tri[1] + vbase, tbase + 3 * t + 1, tri[2] + vbase, tbase + 3 * t + 2
                );
            }
            vbase += o.verts.len();
            tbase += 3 * o.tris.len();
        }
        let mut mtl = String::new();
        for (name, info) in &names {
            let _ = writeln!(mtl, "newmtl {name}\nKd 1 1 1\nmap_Kd textures/{}", info.file);
            if info.alpha != "opaque" {
                let _ = writeln!(mtl, "map_d textures/{}", info.file);
            }
            if info.alpha == "blend" {
                let _ = writeln!(mtl, "d {}", info.opacity);
            }
            mtl.push('\n');
        }
        (obj, mtl)
    }
}
