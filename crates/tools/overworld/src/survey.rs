//! `overworld kit-survey <region>`: what a region's scene is made of, for choosing its texture
//! roles, its theme's measurements and its kit pieces. Writes into `out/overworld/survey/<region>`:
//!
//! - `textures.png`: every material's texture (`#N role`), from the region's texture library
//!   (made first, into `out/overworld/textures/<region>`);
//! - `materials.txt`: per material N: its role, texture size, wrap, alpha, where it's used (rooms,
//!   triangles, how much of it faces up / sideways / down) and how many units one repeat of its
//!   texture covers along u and v (the median over its triangles: a theme's `tile`s);
//! - `plan.png`: the scene from above, north up, the pieces a kit could cut outlined and numbered
//!   (each connected piece of a room's mesh smaller than `BIG`), with a grid line every 1000;
//!   `--box x0 y0 x1 y1` draws only that part of it, bigger (`plan_box.png`), and `--below z`
//!   leaves out every triangle wholly above z (ceilings, roofs over caves);
//! - `components.txt`: those pieces: number, room, bounds (builder axes: x east, y north, z up),
//!   triangles, roles, and the collision inside their bounds (surface types, with the role each
//!   would get, and exits);
//! - `islands.txt`: per material, its connected islands (bounds, triangles): where a structure's
//!   faces are welded into the terrain (Kakariko's houses), its own materials' islands show where
//!   it is, to cut by `take.materials` in a box;
//! - `comps/<n>.png`: each piece drawn as the Kit panel draws one.

use crate::kit::{default_role, SceneTextures};
use crate::pieces::{components, scene_tris, Piece, PieceMaterial, Tri};
use crate::textures::Library;
use crate::thumb::{self, Image, Texels, View};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

/// Pieces wider than this (either way across) are terrain, not kit candidates.
pub const BIG: f64 = 3000.0;

/// One connected piece of a room's mesh.
pub struct Component {
    pub id: usize,
    pub room: String,
    pub tris: Vec<usize>,
    pub lo: [f64; 3],
    pub hi: [f64; 3],
}

/// Runs the survey. `zoom`: only that box of the plan (x0, y0, x1, y1); `below`: the plan without
/// triangles wholly above it. Returns a summary line.
pub fn survey(region: &str, root: &Path, out: &Path, zoom: Option<[f64; 4]>, below: Option<f64>) -> Result<String, String> {
    let manifest = crate::pieces::Manifest::load(&crate::kit::manifest_path(region))?;
    let sc = manifest.scene();
    let glb_path = root.join(&sc.glb);
    let glb = std::fs::read(&glb_path).map_err(|e| format!("{}: {e}", glb_path.display()))?;
    let col_path = root.join(&manifest.source.collision);
    let collision: Value = serde_json::from_str(&std::fs::read_to_string(&col_path).map_err(|e| format!("{}: {e}", col_path.display()))?)
        .map_err(|e| format!("{}: {e}", col_path.display()))?;
    let lib_dir = root.join("out/overworld/textures").join(region);
    // the library as the manifest names it now (a check that fails still leaves the PNGs)
    let lib_note = match crate::kit::export(&sc, &glb_path, &lib_dir) {
        Ok(n) => format!("{n} textures in {}", lib_dir.display()),
        Err(e) => format!("texture library: {e}"),
    };
    let lib = Library::load(&lib_dir).ok();
    std::fs::create_dir_all(out.join("comps")).map_err(|e| format!("{}: {e}", out.display()))?;
    let write = |name: &str, bytes: &[u8]| std::fs::write(out.join(name), bytes).map_err(|e| format!("{}: {e}", out.join(name).display()));

    let (tris, tints) = scene_tris(&glb, &sc)?;
    let (j, _) = crate::kit::read_glb(&glb)?;
    let mats = j["materials"].as_array().cloned().unwrap_or_default();

    write("materials.txt", materials_table(&sc, &tris, &mats, lib.as_ref()).as_bytes())?;
    if let Some(lib) = &lib {
        write("textures.png", &texture_sheet(&sc, &mats, lib).png())?;
    }

    // components, numbered by room, then west to east
    let mut rooms: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (i, t) in tris.iter().enumerate() {
        rooms.entry(t.room.as_str()).or_default().push(i);
    }
    let mut comps: Vec<Component> = vec![];
    for (room, idx) in &rooms {
        let refs: Vec<&Tri> = idx.iter().map(|&i| &tris[i]).collect();
        let mut cs: Vec<Component> = components(&refs)
            .into_iter()
            .map(|c| {
                let ts: Vec<usize> = c.iter().map(|&k| idx[k]).collect();
                let (lo, hi) = bounds(ts.iter().flat_map(|&t| tris[t].p.iter()));
                Component { id: 0, room: room.to_string(), tris: ts, lo, hi }
            })
            .collect();
        cs.sort_by(|a, b| a.lo[0].total_cmp(&b.lo[0]));
        comps.extend(cs);
    }
    for (k, c) in comps.iter_mut().enumerate() {
        c.id = k + 1;
    }
    let big = |c: &Component| c.hi[0] - c.lo[0] > BIG || c.hi[1] - c.lo[1] > BIG;

    // the collision in each candidate's bounds
    let cverts: Vec<[f64; 3]> = collision["vertices"].as_array().ok_or("collision.json: no vertices")?.iter().map(|v| [v[0].as_f64().unwrap_or(0.0), -v[2].as_f64().unwrap_or(0.0), v[1].as_f64().unwrap_or(0.0)]).collect();
    let stypes = collision["surface_types"].as_array().cloned().unwrap_or_default();
    let polys = collision["polys"].as_array().cloned().unwrap_or_default();
    let mut text = String::new();
    let _ = writeln!(text, "{} ({}): {} components; the pieces under {BIG} across are kit candidates (plan.png, comps/<n>.png).", sc.label, region, comps.len());
    let _ = writeln!(text, "bounds are builder axes (x east, y north, z up); collision: surface type (role it'd get [manifest surfaces override]) x polys; exit = scene exit index\n");
    for c in &comps {
        let mut roles: BTreeMap<&str, usize> = BTreeMap::new();
        for &t in &c.tris {
            *roles.entry(tris[t].role.as_str()).or_default() += 1;
        }
        let mut roles: Vec<(&str, usize)> = roles.into_iter().collect();
        roles.sort_by(|a, b| b.1.cmp(&a.1));
        let mut surf: BTreeMap<u64, usize> = BTreeMap::new();
        for p in &polys {
            let vi: Vec<usize> = p["vertices"].as_array().map(|a| a.iter().filter_map(|x| x.as_u64().map(|x| x as usize)).collect()).unwrap_or_default();
            if vi.len() == 3 && vi.iter().all(|&v| (0..3).all(|k| cverts[v][k] >= c.lo[k] - 1.0 && cverts[v][k] <= c.hi[k] + 1.0)) {
                *surf.entry(p["surface_type"].as_u64().unwrap_or(0)).or_default() += 1;
            }
        }
        let surf: Vec<String> = surf
            .iter()
            .map(|(st, n)| {
                let s = stypes.get(*st as usize).cloned().unwrap_or(Value::Null);
                let role = manifest.surfaces.get(&st.to_string()).cloned().unwrap_or_else(|| default_role(&s).to_string());
                let exit = s["exit_index"].as_u64().filter(|&e| e > 0).map_or(String::new(), |e| format!(" exit {e}"));
                format!("{st}({role}{exit})x{n}")
            })
            .collect();
        let _ = writeln!(
            text,
            "#{:<4} {:12} [{:.0}, {:.0}, {:.0}] .. [{:.0}, {:.0}, {:.0}]  {:.0} x {:.0} x {:.0}  {} tris{}  roles: {}  collision: {}",
            c.id,
            c.room,
            c.lo[0],
            c.lo[1],
            c.lo[2],
            c.hi[0],
            c.hi[1],
            c.hi[2],
            c.hi[0] - c.lo[0],
            c.hi[1] - c.lo[1],
            c.hi[2] - c.lo[2],
            c.tris.len(),
            if big(c) { " BIG (terrain)" } else { "" },
            roles.iter().take(6).map(|(r, n)| format!("{r} {n}")).collect::<Vec<_>>().join(", "),
            if surf.is_empty() { "none".to_string() } else { surf.join(" ") }
        );
    }
    write("components.txt", text.as_bytes())?;

    // each material's islands
    let mut isl = String::new();
    let _ = writeln!(isl, "{}: each material's connected islands (sharing vertices within the material), room, bounds (builder axes), triangles. Islands of one structure's materials in one place are a piece to cut with take.materials.
", sc.label);
    let mut by_mat: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (i, t) in tris.iter().enumerate() {
        by_mat.entry(t.mat).or_default().push(i);
    }
    for (_, idx) in by_mat {
        let refs: Vec<&Tri> = idx.iter().map(|&i| &tris[i]).collect();
        let t0 = refs[0];
        let mut found: Vec<(String, [f64; 3], [f64; 3], usize)> = components(&refs)
            .into_iter()
            .map(|c| {
                let (lo, hi) = bounds(c.iter().flat_map(|&k| refs[k].p.iter()));
                (refs[c[0]].room.clone(), lo, hi, c.len())
            })
            .collect();
        found.sort_by(|a, b| a.0.cmp(&b.0).then(a.1[0].total_cmp(&b.1[0])));
        let _ = writeln!(isl, "{} ({} islands, {} tris):", t0.role, found.len(), refs.len());
        for (room, lo, hi, n) in found {
            let _ = writeln!(isl, "    {:12} [{:.0}, {:.0}, {:.0}] .. [{:.0}, {:.0}, {:.0}]  {n} tris", room, lo[0], lo[1], lo[2], hi[0], hi[1], hi[2]);
        }
    }
    write("islands.txt", isl.as_bytes())?;

    // pictures: each candidate, then the plan
    let mut texels = Texels::default();
    let as_piece = |ids: &[usize]| piece_of(ids.iter().map(|&t| &tris[t]), &sc, &tints);
    for c in comps.iter().filter(|c| !big(c)) {
        if let Some(img) = thumb::render(&as_piece(&c.tris), lib.as_ref(), &mut texels, 160) {
            write(&format!("comps/{}.png", c.id), &img.png())?;
        }
    }
    let all: Vec<usize> = (0..tris.len()).filter(|&t| below.is_none_or(|z| tris[t].p.iter().any(|q| q[2] <= z))).collect();
    let scene = as_piece(&all);
    let (lo, hi) = bounds(tris.iter().flat_map(|t| t.p.iter()));
    let [x0, y0, x1, y1] = zoom.unwrap_or([lo[0], lo[1], hi[0], hi[1]]);
    let long = if zoom.is_some() { 2000.0 } else { 1600.0 };
    let scale = long / (x1 - x0).max(y1 - y0).max(1.0);
    let (w, h) = (((x1 - x0) * scale).ceil() as usize + 1, ((y1 - y0) * scale).ceil() as usize + 1);
    let mut plan = Image::filled(w, h, [40, 44, 52, 255]);
    if let Some(img) = thumb::render_view(&scene, lib.as_ref(), &mut texels, w, h, View::Top { min: [x0, y1], scale }) {
        plan.draw(&img, 0, 0);
    }
    let to_px = |x: f64, y: f64| (((x - x0) * scale).round() as i64, ((y1 - y) * scale).round() as i64);
    // a grid line every 1000, labelled
    let mut g = (x0 / 1000.0).ceil() * 1000.0;
    while g <= x1 {
        let (px, _) = to_px(g, y1);
        plan.fill(px, 0, px, h as i64 - 1, [255, 255, 255, 50]);
        plan.text(px + 2, 2, &format!("{g:.0}"), 2, [255, 255, 255, 220]);
        g += 1000.0;
    }
    let mut g = (y0 / 1000.0).ceil() * 1000.0;
    while g <= y1 {
        let (_, py) = to_px(x0, g);
        plan.fill(0, py, w as i64 - 1, py, [255, 255, 255, 50]);
        plan.text(2, py + 2, &format!("{g:.0}"), 2, [255, 255, 255, 220]);
        g += 1000.0;
    }
    for c in comps.iter().filter(|c| !big(c) && c.hi[0] >= x0 && c.lo[0] <= x1 && c.hi[1] >= y0 && c.lo[1] <= y1) {
        let (a, b) = (to_px(c.lo[0], c.hi[1]), to_px(c.hi[0], c.lo[1]));
        plan.rect(a.0, a.1, b.0.max(a.0 + 2), b.1.max(a.1 + 2), 1, [255, 220, 60, 230]);
        plan.text(a.0, a.1, &c.id.to_string(), if zoom.is_some() { 2 } else { 1 }, [255, 230, 90, 255]);
    }
    write(if zoom.is_some() || below.is_some() { "plan_box.png" } else { "plan.png" }, &plan.png())?;
    Ok(format!(
        "{}: {} materials, {} components ({} kit candidates); {lib_note}; survey in {}",
        sc.label,
        mats.iter().filter(|m| m["extras"]["n64_blend"].is_string()).count(),
        comps.len(),
        comps.iter().filter(|c| !big(c)).count(),
        out.display()
    ))
}

fn bounds<'a>(pts: impl Iterator<Item = &'a [f64; 3]>) -> ([f64; 3], [f64; 3]) {
    let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
    for q in pts {
        for k in 0..3 {
            lo[k] = lo[k].min(q[k]);
            hi[k] = hi[k].max(q[k]);
        }
    }
    (lo, hi)
}

/// Scene triangles as a piece in scene coordinates (for pictures only).
fn piece_of<'a>(tris: impl Iterator<Item = &'a Tri>, sc: &SceneTextures, tints: &[[f64; 3]]) -> Piece {
    let mut p = Piece::default();
    let mut mindex: BTreeMap<(usize, bool), u32> = BTreeMap::new();
    for t in tris {
        let m = *mindex.entry((t.mat, t.c.is_some())).or_insert_with(|| {
            p.materials.push(PieceMaterial { texture: sc.texture(&t.role), tint: tints[t.mat], vertex_colors: t.c.is_some() });
            p.materials.len() as u32 - 1
        });
        let base = p.verts.len() as u32;
        for k in 0..3 {
            p.verts.push(t.p[k]);
            p.normals.push(t.n[k]);
            p.colors.push(t.c.map_or([1.0; 3], |c| c[k]));
        }
        p.tris.push([base, base + 1, base + 2]);
        p.uvs.push(t.uv);
        p.mat.push(m);
    }
    p
}

/// Per material: role, texture, use and texel density.
fn materials_table(sc: &SceneTextures, tris: &[Tri], mats: &[Value], lib: Option<&Library>) -> String {
    struct Use {
        rooms: std::collections::BTreeSet<String>,
        n: usize,
        area: [f64; 3],
        du: Vec<(f64, f64)>,
        dv: Vec<(f64, f64)>,
        vc: bool,
    }
    let mut uses: BTreeMap<usize, Use> = BTreeMap::new();
    for t in tris {
        let u = uses.entry(t.mat).or_insert_with(|| Use { rooms: Default::default(), n: 0, area: [0.0; 3], du: vec![], dv: vec![], vc: false });
        u.rooms.insert(t.room.trim_start_matches("room_").to_string());
        u.n += 1;
        u.vc |= t.c.is_some();
        let (e1, e2) = (sub(t.p[1], t.p[0]), sub(t.p[2], t.p[0]));
        let n = cross(e1, e2);
        let a = len(n) / 2.0;
        if a < 1e-6 {
            continue;
        }
        let nz = n[2] / (2.0 * a);
        u.area[if nz > 0.7 { 0 } else if nz < -0.7 { 2 } else { 1 }] += a;
        // dP/du and dP/dv: the units one repeat covers
        let (du1, dv1, du2, dv2) = (t.uv[1][0] - t.uv[0][0], t.uv[1][1] - t.uv[0][1], t.uv[2][0] - t.uv[0][0], t.uv[2][1] - t.uv[0][1]);
        let det = du1 * dv2 - du2 * dv1;
        if det.abs() < 1e-9 {
            continue;
        }
        let pu = [0, 1, 2].map(|k| (e1[k] * dv2 - e2[k] * dv1) / det);
        let pv = [0, 1, 2].map(|k| (e2[k] * du1 - e1[k] * du2) / det);
        u.du.push((len(pu), a));
        u.dv.push((len(pv), a));
    }
    let median = |v: &mut Vec<(f64, f64)>| -> f64 {
        if v.is_empty() {
            return 0.0;
        }
        v.sort_by(|a, b| a.0.total_cmp(&b.0));
        let total: f64 = v.iter().map(|x| x.1).sum();
        let mut acc = 0.0;
        for x in v.iter() {
            acc += x.1;
            if acc >= total / 2.0 {
                return x.0;
            }
        }
        v.last().unwrap().0
    };
    let mut s = String::new();
    let _ = writeln!(s, "{}: material N -> role. size wrap(u/v) alpha; rooms; triangles; area % facing up/side/down; units per texture repeat along u and v (area-weighted median); vc = shaded by vertex colours; tex1 = a second texture (TEXCOORD_1: a detail or blend)\n", sc.label);
    for (mi, m) in mats.iter().enumerate() {
        let ex = &m["extras"];
        let Some(blend) = ex["n64_blend"].as_str() else { continue };
        let name = m["name"].as_str().unwrap_or("");
        let Some(n) = name.rsplit("mat").next().and_then(|s| s.parse::<usize>().ok()) else { continue };
        let role = sc.role(n);
        let info = lib.and_then(|l| l.info.get(&sc.texture(&role)));
        let tex = info.map_or("no texture".to_string(), |i| format!("{}x{} {}/{} {}", i.size[0], i.size[1], i.wrap_u, i.wrap_v, i.alpha));
        let tex1 = if ex["n64_texture1"].is_u64() { " tex1" } else { "" };
        match uses.get_mut(&mi) {
            Some(u) => {
                let total = (u.area[0] + u.area[1] + u.area[2]).max(1e-9);
                let (mu, mv) = (median(&mut u.du), median(&mut u.dv));
                let _ = writeln!(
                    s,
                    "#{n:<3} {role:22} {tex:28} {blend:12} rooms {:8} {:5} tris  up {:3.0}% side {:3.0}% down {:3.0}%  u {:6.0} v {:6.0}{}{tex1}",
                    u.rooms.iter().cloned().collect::<Vec<_>>().join(","),
                    u.n,
                    u.area[0] / total * 100.0,
                    u.area[1] / total * 100.0,
                    u.area[2] / total * 100.0,
                    mu,
                    mv,
                    if u.vc { " vc" } else { "" }
                );
            }
            None => {
                let _ = writeln!(s, "#{n:<3} {role:22} {tex:28} {blend:12} (no triangles){tex1}");
            }
        }
    }
    s
}

/// Every material's texture, 96 pixels a side, labelled `#N role`.
fn texture_sheet(sc: &SceneTextures, mats: &[Value], lib: &Library) -> Image {
    let mut items: Vec<(usize, String)> = vec![];
    for m in mats {
        if !m["extras"]["n64_blend"].is_string() {
            continue;
        }
        let Some(n) = m["name"].as_str().and_then(|s| s.rsplit("mat").next()).and_then(|s| s.parse::<usize>().ok()) else { continue };
        if !items.iter().any(|i| i.0 == n) {
            items.push((n, sc.role(n)));
        }
    }
    items.sort();
    let (cell, cols) = (176i64, 8i64);
    let rows = (items.len() as i64 + cols - 1) / cols;
    let mut img = Image::filled((cell * cols) as usize, (cell * rows.max(1)) as usize, [30, 32, 38, 255]);
    for (k, (n, role)) in items.iter().enumerate() {
        let (cx, cy) = ((k as i64 % cols) * cell, (k as i64 / cols) * cell);
        // a checkerboard under it shows the alpha
        for y in 0..144 {
            for x in 0..144 {
                let c = if (x / 12 + y / 12) % 2 == 0 { 90 } else { 60 };
                img.blend_px((cx + 16 + x) as usize, (cy + 4 + y) as usize, [c, c, c, 255]);
            }
        }
        if let Some((w, h, px)) = lib.rgba(&sc.texture(role)) {
            // the longer side 144, nearest texel
            let k = 144.0 / w.max(h) as f64;
            let (tw, th) = ((w as f64 * k) as i64, (h as f64 * k) as i64);
            for y in 0..th {
                for x in 0..tw {
                    let (sx, sy) = (((x as f64 / k) as u32).min(w - 1), ((y as f64 / k) as u32).min(h - 1));
                    let i = ((sy * w + sx) * 4) as usize;
                    img.blend_px((cx + 16 + x) as usize, (cy + 4 + y) as usize, [px[i], px[i + 1], px[i + 2], px[i + 3]]);
                }
            }
        }
        let label: String = format!("{n} {role}").chars().take(21).collect();
        img.text(cx + 2, cy + 150, &label, 2, [255, 255, 255, 255]);
    }
    img
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn len(a: [f64; 3]) -> f64 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}
