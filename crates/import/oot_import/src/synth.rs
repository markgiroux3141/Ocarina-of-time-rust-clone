//! A synthetic, original test character ("Tock", a small wind-up automaton) encoded in the
//! same binary formats OoT uses: F3DEX2 display lists, CI4/I4/I8/IA8/RGBA16 textures with a
//! TLUT, a flex LOD skeleton whose limb DLs stitch joints through segment-0x0D matrices, and
//! compressed `AnimationHeader` animations. It lets the whole decode + render pipeline be
//! exercised and inspected visually without any game data.

use eng_anim::skeleton::{LimbType, Skeleton};
use eng_gfx::combiner;
use glam::Vec3;

use crate::z64::ParseSkeleton;

pub const SEGMENT: u8 = 6;
/// Face textures are fetched through this segment so the viewer can swap them per frame,
/// the same mechanism the game uses for eye/mouth textures.
pub const FACE_SEGMENT: u8 = 8;

pub struct SynthObject {
    pub data: Vec<u8>,
    pub skeleton_offset: usize,
    pub animations: Vec<(String, usize)>,
    pub face_textures: Vec<(String, usize)>,
    pub limb_names: Vec<&'static str>,
}

impl SynthObject {
    pub fn skeleton(&self) -> Skeleton {
        Skeleton::parse(&self.data, SEGMENT, self.skeleton_offset, LimbType::Lod, true)
            .expect("synthetic skeleton must parse")
    }
}

// ---------------------------------------------------------------------------------------
// GBI encoding helpers (mirroring the gbi.h macros).

const fn seg_addr(off: usize) -> u32 {
    ((SEGMENT as u32) << 24) | off as u32
}

#[derive(Default)]
struct Dl(Vec<(u32, u32)>);

impl Dl {
    fn cmd(&mut self, w0: u32, w1: u32) {
        self.0.push((w0, w1));
    }
    fn pipe_sync(&mut self) {
        self.cmd(0xE700_0000, 0);
    }
    fn tile_sync(&mut self) {
        self.cmd(0xE800_0000, 0);
    }
    fn load_sync(&mut self) {
        self.cmd(0xE600_0000, 0);
    }
    fn texture(&mut self, on: bool) {
        self.cmd(0xD700_0000 | ((on as u32) << 1), 0xFFFF_FFFF);
    }
    fn geometry(&mut self, clear: u32, set: u32) {
        self.cmd(0xD900_0000 | (!clear & 0x00FF_FFFF), set);
    }
    fn othermode_h(&mut self, shift: u32, len: u32, val: u32) {
        self.cmd(0xE300_0000 | ((32 - shift - len) << 8) | (len - 1), val);
    }
    fn othermode_l(&mut self, shift: u32, len: u32, val: u32) {
        self.cmd(0xE200_0000 | ((32 - shift - len) << 8) | (len - 1), val);
    }
    fn cycle(&mut self, two: bool) {
        self.othermode_h(20, 2, if two { 1 << 20 } else { 0 });
    }
    fn tlut(&mut self, rgba16: bool) {
        self.othermode_h(14, 2, if rgba16 { 2 << 14 } else { 0 });
    }
    fn render_mode(&mut self, mode: u32) {
        self.othermode_l(3, 29, mode);
    }
    fn combine(&mut self, c0: [u32; 8], c1: [u32; 8]) {
        let raw = combiner::encode(c0, c1);
        self.cmd(0xFC00_0000 | (raw >> 32) as u32, raw as u32);
    }
    fn prim(&mut self, c: [u8; 4]) {
        self.cmd(0xFA00_0000, u32::from_be_bytes(c));
    }
    fn settimg(&mut self, fmt: u32, siz: u32, width: u32, addr: u32) {
        self.cmd(0xFD00_0000 | (fmt << 21) | (siz << 19) | (width - 1), addr);
    }
    #[allow(clippy::too_many_arguments)]
    fn settile(&mut self, fmt: u32, siz: u32, line: u32, tmem: u32, tile: u32, pal: u32, cm: [u32; 2], mask: [u32; 2]) {
        let [cms, cmt] = cm;
        let [masks, maskt] = mask;
        self.cmd(
            0xF500_0000 | (fmt << 21) | (siz << 19) | (line << 9) | tmem,
            (tile << 24) | (pal << 20) | (cmt << 18) | (maskt << 14) | (cms << 8) | (masks << 4),
        );
    }
    /// gsDPLoadTextureBlock / _4b.
    fn load_texture_block(&mut self, addr: u32, fmt: u32, siz: u32, w: u32, h: u32, pal: u32, cm: [u32; 2]) {
        let load_siz = if siz == 3 { 3 } else { 2 };
        let bits = 4u32 << siz;
        let texels16 = if siz == 3 { w * h } else { (w * h * bits).div_ceil(16) };
        let words = ((w * bits) / 64).max(1);
        let dxt = ((1 << 11) + words - 1) / words;
        let line = if siz == 3 { (w * 2).div_ceil(8) } else { (w * bits).div_ceil(64) };
        let mask = [w.trailing_zeros(), h.trailing_zeros()];
        self.settimg(fmt, load_siz, 1, addr);
        self.settile(fmt, load_siz, 0, 0, 7, 0, cm, mask);
        self.load_sync();
        self.cmd(0xF300_0000, (7 << 24) | ((texels16 - 1) << 12) | dxt);
        self.pipe_sync();
        self.settile(fmt, siz, line, 0, 0, pal, cm, mask);
        self.cmd(0xF200_0000, ((w - 1) << 14) | ((h - 1) << 2));
    }
    fn load_tlut_pal16(&mut self, pal: u32, addr: u32) {
        self.settimg(0, 2, 1, addr);
        self.tile_sync();
        self.settile(0, 0, 0, 256 + pal * 16, 7, 0, [0, 0], [0, 0]);
        self.load_sync();
        self.cmd(0xF000_0000, (7 << 24) | (15 << 14));
        self.pipe_sync();
    }
    fn vtx(&mut self, addr: u32, n: u32, v0: u32) {
        self.cmd(0x0100_0000 | (n << 12) | ((v0 + n) << 1), addr);
    }
    fn tri(&mut self, a: u32, b: u32, c: u32) {
        self.cmd(0x0500_0000 | (a * 2) << 16 | (b * 2) << 8 | c * 2, 0);
    }
    fn mtx_load_flex(&mut self, matrix_index: u32) {
        // G_MTX_NOPUSH | G_MTX_LOAD | G_MTX_MODELVIEW, stored as params ^ G_MTX_PUSH.
        self.cmd(0xDA38_0003, 0x0D00_0000 + matrix_index * 0x40);
    }
    fn end(&mut self) {
        self.cmd(0xDF00_0000, 0);
    }
}

// Combiner selector numbers as used by gsDPSetCombineLERP.
mod cc {
    pub const COMBINED: u32 = 0;
    pub const TEXEL0: u32 = 1;
    pub const PRIMITIVE: u32 = 3;
    pub const SHADE: u32 = 4;
    pub const ENVIRONMENT: u32 = 5;
    pub const ZERO_AB: u32 = 15;
    pub const ZERO_C: u32 = 31;
    pub const ZERO_D: u32 = 7;
    pub const A_ZERO: u32 = 7;
    pub const A_ONE: u32 = 6;
    pub const A_TEXEL0: u32 = 1;
    pub const A_PRIMITIVE: u32 = 3;
}

// Render modes.
const RM_AA_ZB_OPA_SURF: u32 = 0x8 | 0x10 | 0x20 | 0x40 | 0x2000 | rm_blend(0, 0, 1, 1);
const RM_AA_ZB_TEX_EDGE: u32 = 0x8 | 0x10 | 0x20 | 0x40 | 0x1000 | 0x2000 | 0x100 | rm_blend(0, 0, 1, 1);
const RM_AA_ZB_XLU_SURF: u32 = 0x8 | 0x10 | 0x40 | 0x80 | 0x100 | 0x800 | 0x4000 | rm_blend(0, 0, 1, 0);
const fn rm_blend(p: u32, a: u32, m: u32, b: u32) -> u32 {
    (p << 30) | (a << 26) | (m << 22) | (b << 18) | (p << 28) | (a << 24) | (m << 20) | (b << 16)
}

const G_LIGHTING: u32 = eng_gbi::gbi::G_LIGHTING;
const G_CULL_BACK: u32 = eng_gbi::gbi::G_CULL_BACK;

// ---------------------------------------------------------------------------------------
// Geometry.

#[derive(Clone, Copy)]
struct SVert {
    pos: Vec3,
    normal: Vec3,
    st: [f32; 2],
    color: Option<[u8; 4]>,
}

fn v(pos: Vec3, normal: Vec3, st: [f32; 2]) -> SVert {
    SVert { pos, normal: normal.normalize_or_zero(), st, color: None }
}

#[derive(Default)]
struct Mesh {
    verts: Vec<SVert>,
    tris: Vec<[u32; 3]>,
}

impl Mesh {
    fn add(&mut self, v: SVert) -> u32 {
        self.verts.push(v);
        self.verts.len() as u32 - 1
    }
    /// Adds a triangle, flipping it if needed so its winding agrees with its vertex normals.
    fn tri(&mut self, a: u32, b: u32, c: u32) {
        let (pa, pb, pc) = (self.verts[a as usize].pos, self.verts[b as usize].pos, self.verts[c as usize].pos);
        let face = (pb - pa).cross(pc - pa);
        let n = self.verts[a as usize].normal + self.verts[b as usize].normal + self.verts[c as usize].normal;
        if face.dot(n) < 0.0 { self.tris.push([a, c, b]) } else { self.tris.push([a, b, c]) }
    }
    fn quad(&mut self, q: [SVert; 4]) {
        let i: Vec<u32> = q.iter().map(|&x| self.add(x)).collect();
        self.tri(i[0], i[1], i[2]);
        self.tri(i[0], i[2], i[3]);
    }
    fn colored(mut self, c: [u8; 4]) -> Mesh {
        for v in &mut self.verts {
            v.color = Some(c);
        }
        self
    }

    /// Axis-aligned box with per-face normals. `tex` is the texel size mapped onto each face.
    fn cube(lo: Vec3, hi: Vec3, tex: [f32; 2]) -> Mesh {
        let mut m = Mesh::default();
        let d = hi - lo;
        let faces = [
            (Vec3::new(hi.x, lo.y, hi.z), Vec3::new(0., 0., -d.z), Vec3::new(0., d.y, 0.), Vec3::X),
            (Vec3::new(lo.x, lo.y, lo.z), Vec3::new(0., 0., d.z), Vec3::new(0., d.y, 0.), Vec3::NEG_X),
            (Vec3::new(lo.x, hi.y, hi.z), Vec3::new(d.x, 0., 0.), Vec3::new(0., 0., -d.z), Vec3::Y),
            (Vec3::new(lo.x, lo.y, lo.z), Vec3::new(d.x, 0., 0.), Vec3::new(0., 0., d.z), Vec3::NEG_Y),
            (Vec3::new(lo.x, lo.y, hi.z), Vec3::new(d.x, 0., 0.), Vec3::new(0., d.y, 0.), Vec3::Z),
            (Vec3::new(hi.x, lo.y, lo.z), Vec3::new(-d.x, 0., 0.), Vec3::new(0., d.y, 0.), Vec3::NEG_Z),
        ];
        let [w, h] = tex;
        for (o, u, vv, n) in faces {
            m.quad([v(o, n, [0., h]), v(o + u, n, [w, h]), v(o + u + vv, n, [w, 0.]), v(o + vv, n, [0., 0.])]);
        }
        m
    }

    /// Tapered prism along Y from (y0, r0) to (y1, r1) with smooth side normals and caps.
    fn prism(sides: u32, y0: f32, r0: f32, y1: f32, r1: f32, tex: [f32; 2], caps: bool) -> Mesh {
        let mut m = Mesh::default();
        let ring = |m: &mut Mesh, y: f32, r: f32, t: f32| -> Vec<u32> {
            (0..=sides)
                .map(|i| {
                    let a = i as f32 / sides as f32 * std::f32::consts::TAU;
                    let dir = Vec3::new(a.sin(), 0., a.cos());
                    let slope = (r0 - r1) / (y1 - y0).abs().max(1.0);
                    m.add(v(dir * r + Vec3::Y * y, dir + Vec3::Y * slope * (y1 - y0).signum(), [tex[0] * i as f32 / sides as f32, t]))
                })
                .collect()
        };
        let top = if y1 > y0 { (y1, r1, y0, r0) } else { (y0, r0, y1, r1) };
        let a = ring(&mut m, top.0, top.1, 0.0);
        let b = ring(&mut m, top.2, top.3, tex[1]);
        for i in 0..sides as usize {
            m.tri(a[i], b[i], b[i + 1]);
            m.tri(a[i], b[i + 1], a[i + 1]);
        }
        if caps {
            for (y, r, n) in [(top.0, top.1, Vec3::Y), (top.2, top.3, Vec3::NEG_Y)] {
                let c = m.add(v(Vec3::Y * y, n, [tex[0] * 0.5, tex[1] * 0.5]));
                let rim: Vec<u32> = (0..sides)
                    .map(|i| {
                        let a = i as f32 / sides as f32 * std::f32::consts::TAU;
                        let p = Vec3::new(a.sin() * r, y, a.cos() * r);
                        m.add(v(p, n, [tex[0] * (0.5 + a.sin() * 0.5), tex[1] * (0.5 + a.cos() * 0.5)]))
                    })
                    .collect();
                for i in 0..sides as usize {
                    m.tri(c, rim[i], rim[(i + 1) % sides as usize]);
                }
            }
        }
        m
    }

    fn sphere(center: Vec3, r: f32, rings: u32, segs: u32) -> Mesh {
        let mut m = Mesh::default();
        let mut grid = Vec::new();
        for i in 0..=rings {
            let phi = i as f32 / rings as f32 * std::f32::consts::PI;
            let row: Vec<u32> = (0..=segs)
                .map(|j| {
                    let th = j as f32 / segs as f32 * std::f32::consts::TAU;
                    let n = Vec3::new(phi.sin() * th.sin(), phi.cos(), phi.sin() * th.cos());
                    m.add(v(center + n * r, n, [j as f32 * 4.0, i as f32 * 4.0]))
                })
                .collect();
            grid.push(row);
        }
        for i in 0..rings as usize {
            for j in 0..segs as usize {
                let (a, b, c, d) = (grid[i][j], grid[i + 1][j], grid[i + 1][j + 1], grid[i][j + 1]);
                if i > 0 {
                    m.tri(a, b, c);
                }
                if i + 1 < rings as usize {
                    m.tri(a, c, d);
                }
            }
        }
        m
    }

    fn translate(mut self, t: Vec3) -> Mesh {
        for v in &mut self.verts {
            v.pos += t;
        }
        self
    }
}

/// Owns the object's byte buffer while it is assembled.
struct Builder {
    data: Vec<u8>,
}

impl Builder {
    fn align(&mut self, a: usize) {
        while !self.data.len().is_multiple_of(a) {
            self.data.push(0);
        }
    }
    fn put(&mut self, bytes: &[u8]) -> usize {
        self.align(8);
        let off = self.data.len();
        self.data.extend_from_slice(bytes);
        off
    }
    fn encode_vertex(sv: &SVert) -> [u8; 16] {
        let mut b = [0u8; 16];
        let p = sv.pos.round();
        b[0..2].copy_from_slice(&(p.x as i16).to_be_bytes());
        b[2..4].copy_from_slice(&(p.y as i16).to_be_bytes());
        b[4..6].copy_from_slice(&(p.z as i16).to_be_bytes());
        b[8..10].copy_from_slice(&((sv.st[0] * 32.0).round() as i16).to_be_bytes());
        b[10..12].copy_from_slice(&((sv.st[1] * 32.0).round() as i16).to_be_bytes());
        match sv.color {
            Some(c) => b[12..16].copy_from_slice(&c),
            None => {
                let n = sv.normal * 127.0;
                b[12] = n.x.round() as i8 as u8;
                b[13] = n.y.round() as i8 as u8;
                b[14] = n.z.round() as i8 as u8;
                b[15] = 0xFF;
            }
        }
        b
    }

    /// Emits a mesh in chunks that fit the 32-entry vertex cache, starting at slot `v0`.
    fn emit(&mut self, dl: &mut Dl, mesh: &Mesh, v0: u32) {
        let cap = 32 - v0 as usize;
        let mut i = 0;
        while i < mesh.tris.len() {
            let mut remap: Vec<(u32, u32)> = Vec::new();
            let mut tris = Vec::new();
            while i < mesh.tris.len() {
                let t = mesh.tris[i];
                let new = t.iter().filter(|x| !remap.iter().any(|(o, _)| o == *x)).count();
                if remap.len() + new > cap {
                    break;
                }
                let mapped = t.map(|x| {
                    if let Some((_, n)) = remap.iter().find(|(o, _)| *o == x) {
                        *n
                    } else {
                        remap.push((x, v0 + remap.len() as u32));
                        v0 + remap.len() as u32 - 1
                    }
                });
                tris.push(mapped);
                i += 1;
            }
            let bytes: Vec<u8> = remap.iter().flat_map(|(o, _)| Self::encode_vertex(&mesh.verts[*o as usize])).collect();
            let off = self.put(&bytes);
            dl.vtx(seg_addr(off), remap.len() as u32, v0);
            for t in tris {
                dl.tri(t[0], t[1], t[2]);
            }
        }
    }

    /// Emits a tube whose top ring is bound to the parent limb's matrix and whose bottom ring
    /// is bound to this limb's, so the surface stretches across the joint when it bends. This is
    /// the flex-skeleton technique the game's limb display lists use.
    #[allow(clippy::too_many_arguments)]
    fn emit_seam(&mut self, dl: &mut Dl, parent_mtx: u32, own_mtx: u32, sides: u32, top: (f32, f32), bottom: (f32, f32), tex: [f32; 2]) {
        let ring = |y: f32, r: f32, t: f32| -> Vec<SVert> {
            (0..=sides)
                .map(|i| {
                    let a = i as f32 / sides as f32 * std::f32::consts::TAU;
                    let dir = Vec3::new(a.sin(), 0., a.cos());
                    v(dir * r + Vec3::Y * y, dir, [tex[0] * i as f32 / sides as f32, t])
                })
                .collect()
        };
        let n = sides + 1;
        let a = ring(top.0, top.1, 0.0);
        let b = ring(bottom.0, bottom.1, tex[1]);
        dl.mtx_load_flex(parent_mtx);
        let off = self.put(&a.iter().flat_map(Self::encode_vertex).collect::<Vec<_>>());
        dl.vtx(seg_addr(off), n, 0);
        dl.mtx_load_flex(own_mtx);
        let off = self.put(&b.iter().flat_map(Self::encode_vertex).collect::<Vec<_>>());
        dl.vtx(seg_addr(off), n, n);
        // Outward-facing winding for rings ordered by increasing angle around +Y.
        for i in 0..sides {
            dl.tri(i, n + i, n + i + 1);
            dl.tri(i, n + i + 1, i + 1);
        }
    }
}

// ---------------------------------------------------------------------------------------
// Procedural textures (original artwork).

fn rgba16(r: u8, g: u8, b: u8, a: bool) -> u16 {
    ((r as u16 >> 3) << 11) | ((g as u16 >> 3) << 6) | ((b as u16 >> 3) << 1) | a as u16
}

fn brass_ci4() -> (Vec<u8>, Vec<u8>) {
    let pal: Vec<u16> = (0..16)
        .map(|i| {
            let t = i as f32 / 15.0;
            rgba16((90.0 + 165.0 * t) as u8, (55.0 + 150.0 * t) as u8, (20.0 + 70.0 * t) as u8, true)
        })
        .collect();
    let mut px = vec![0u8; 16 * 16];
    for y in 0..16 {
        for x in 0..16 {
            let mut v = 9.0 + 2.0 * ((x as f32 * 0.9).sin() + (y as f32 * 0.4).cos());
            if x == 0 || y == 0 {
                v = 4.0; // panel seam
            }
            let rivet = |cx: i32, cy: i32| ((x - cx) * (x - cx) + (y - cy) * (y - cy)) <= 2;
            if rivet(3, 3) || rivet(12, 3) || rivet(3, 12) || rivet(12, 12) {
                v = 15.0;
            }
            px[(y * 16 + x) as usize] = v.clamp(0.0, 15.0) as u8;
        }
    }
    let tex = px.chunks(2).map(|p| (p[0] << 4) | p[1]).collect();
    let tlut = pal.iter().flat_map(|c| c.to_be_bytes()).collect();
    (tex, tlut)
}

fn gear_i4() -> Vec<u8> {
    let mut px = vec![0u8; 32 * 32];
    for y in 0..32 {
        for x in 0..32 {
            let (dx, dy) = (x as f32 - 15.5, y as f32 - 15.5);
            let r = (dx * dx + dy * dy).sqrt();
            let a = dy.atan2(dx);
            let tooth = if (a * 8.0 / std::f32::consts::TAU * 2.0).rem_euclid(2.0) < 1.0 { 13.5 } else { 11.0 };
            let on = (r < tooth && r > 5.0) || r < 2.5;
            px[y * 32 + x] = if on { 15 } else { 0 };
        }
    }
    px.chunks(2).map(|p| (p[0] << 4) | p[1]).collect()
}

fn stripes_i8() -> Vec<u8> {
    let mut px = vec![0u8; 16 * 16];
    for y in 0..16 {
        for x in 0..16 {
            let band = if (y / 4) % 2 == 0 { 210 } else { 120 };
            px[y * 16 + x] = (band as i32 + (x as i32 - 8) * 2).clamp(0, 255) as u8;
        }
    }
    px
}

/// Wind-up key bow: IA8 with alpha cut-out.
fn key_ia8() -> Vec<u8> {
    let mut px = vec![0u8; 32 * 32];
    for y in 0..32 {
        for x in 0..32 {
            let lobe = |cx: f32| ((x as f32 - cx).powi(2) + (y as f32 - 12.0).powi(2)).sqrt();
            let (l, r) = (lobe(9.0), lobe(22.0));
            let in_lobe = (l < 8.5 && l > 3.5) || (r < 8.5 && r > 3.5);
            let stem = (13..19).contains(&x) && y >= 12;
            if in_lobe || stem {
                let i = (10 + (y as u32 * 5 / 32)).min(15) as u8;
                px[y * 32 + x] = (i << 4) | 0xF;
            }
        }
    }
    px
}

/// 32x16 RGBA16 face plate. `kind`: 0 = open, 1 = blink, 2 = happy.
fn face_rgba16(kind: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(32 * 16 * 2);
    for y in 0..16i32 {
        for x in 0..32i32 {
            let mut c = rgba16(38, 52, 58, true);
            for cx in [9, 22] {
                let (dx, dy) = (x - cx, y - 6);
                let d2 = dx * dx + dy * dy;
                match kind {
                    0 => {
                        if d2 <= 16 {
                            c = rgba16(40, 210, 230, true);
                        }
                        if d2 <= 3 && dx <= 0 && dy <= 0 {
                            c = rgba16(240, 255, 255, true);
                        }
                    }
                    1 => {
                        if dy == 0 && dx.abs() <= 4 {
                            c = rgba16(40, 210, 230, true);
                        }
                    }
                    _ => {
                        // Upturned arcs: "^ ^".
                        if (-3..=0).contains(&dy) && dx.abs() <= 4 && dx.abs() == dy + 4 - 1 + (dy == -3) as i32 {
                            c = rgba16(40, 210, 230, true);
                        }
                    }
                }
            }
            // Speaker grille mouth.
            if (11..14).contains(&y) && (12..20).contains(&x) && x % 2 == 0 {
                c = rgba16(150, 160, 160, true);
            }
            out.extend_from_slice(&c.to_be_bytes());
        }
    }
    out
}

// ---------------------------------------------------------------------------------------
// Skeleton definition.

pub const LIMB_NAMES: [&str; 13] = [
    "Root", "Pelvis", "Torso", "Head", "Antenna", "LUpperArm", "LForearm", "RUpperArm", "RForearm", "LThigh",
    "LShin", "RThigh", "RShin",
];

struct LimbDef {
    pos: [i16; 3],
    child: u8,
    sibling: u8,
}

const NONE: u8 = 0xFF;

fn limb_defs() -> Vec<LimbDef> {
    let l = |pos: [i16; 3], child: u8, sibling: u8| LimbDef { pos, child, sibling };
    vec![
        l([0, 1450, 0], 1, NONE),    // 0 root
        l([0, 0, 0], 2, NONE),       // 1 pelvis
        l([0, 150, 0], 3, 9),        // 2 torso
        l([0, 1150, 0], 4, 5),       // 3 head
        l([0, 720, 0], NONE, NONE),  // 4 antenna
        l([620, 900, 0], 6, 7),      // 5 L upper arm
        l([0, -520, 0], NONE, NONE), // 6 L forearm
        l([-620, 900, 0], 8, NONE),  // 7 R upper arm
        l([0, -520, 0], NONE, NONE), // 8 R forearm
        l([260, -60, 0], 10, 11),    // 9 L thigh
        l([0, -680, 0], NONE, NONE), // 10 L shin
        l([-260, -60, 0], 12, NONE), // 11 R thigh
        l([0, -680, 0], NONE, NONE), // 12 R shin
    ]
}

pub fn build() -> SynthObject {
    let mut b = Builder { data: Vec::new() };

    // Textures and palettes.
    let (brass, brass_tlut) = brass_ci4();
    let brass_off = b.put(&brass);
    let brass_tlut_off = b.put(&brass_tlut);
    let gear_off = b.put(&gear_i4());
    let stripes_off = b.put(&stripes_i8());
    let key_off = b.put(&key_ia8());
    let faces: Vec<(String, usize)> = ["open", "blink", "happy"]
        .iter()
        .enumerate()
        .map(|(i, n)| (n.to_string(), b.put(&face_rgba16(i as u32))))
        .collect();

    let defs = limb_defs();
    // Matrix index of each limb in the segment-0x0D array: draw order of limbs that have a DL
    // (every limb except the root).
    let mtx = |limb: usize| -> u32 { limb as u32 - 1 };

    use cc::*;
    let brass_mat = |dl: &mut Dl| {
        dl.pipe_sync();
        dl.cycle(false);
        dl.render_mode(RM_AA_ZB_OPA_SURF);
        dl.geometry(0, G_LIGHTING | G_CULL_BACK);
        dl.tlut(true);
        dl.combine([TEXEL0, ZERO_AB, SHADE, ZERO_D, A_ZERO, A_ZERO, A_ZERO, A_ONE], [TEXEL0, ZERO_AB, SHADE, ZERO_D, A_ZERO, A_ZERO, A_ZERO, A_ONE]);
        dl.texture(true);
        dl.load_tlut_pal16(0, seg_addr(brass_tlut_off));
        dl.load_texture_block(seg_addr(brass_off), 2, 0, 16, 16, 0, [0, 0]);
    };
    // Limb stripes: two-cycle, cycle 1 lights the I8 texel, cycle 2 tints by primitive colour.
    let steel_mat = |dl: &mut Dl| {
        dl.pipe_sync();
        dl.cycle(true);
        dl.render_mode(RM_AA_ZB_OPA_SURF);
        dl.geometry(0, G_LIGHTING | G_CULL_BACK);
        dl.tlut(false);
        dl.combine([TEXEL0, ZERO_AB, SHADE, ZERO_D, A_ZERO, A_ZERO, A_ZERO, A_ONE], [COMBINED, ZERO_AB, PRIMITIVE, ZERO_D, A_ZERO, A_ZERO, A_ZERO, A_ONE]);
        dl.prim([150, 170, 185, 255]);
        dl.texture(true);
        dl.load_texture_block(seg_addr(stripes_off), 4, 1, 16, 16, 0, [0, 1]);
    };
    let rubber_mat = |dl: &mut Dl| {
        dl.pipe_sync();
        dl.cycle(false);
        dl.render_mode(RM_AA_ZB_OPA_SURF);
        dl.geometry(G_LIGHTING, G_CULL_BACK);
        dl.texture(false);
        dl.combine([ZERO_AB, ZERO_AB, ZERO_C, SHADE, A_ZERO, A_ZERO, A_ZERO, A_ONE], [ZERO_AB, ZERO_AB, ZERO_C, SHADE, A_ZERO, A_ZERO, A_ZERO, A_ONE]);
    };

    let mut dls: Vec<Option<Dl>> = (0..defs.len()).map(|_| None).collect();

    // Pelvis.
    let mut dl = Dl::default();
    brass_mat(&mut dl);
    b.emit(&mut dl, &Mesh::cube(Vec3::new(-330., -120., -230.), Vec3::new(330., 160., 230.), [32., 16.]), 0);
    dl.end();
    dls[1] = Some(dl);

    // Torso: brass body, env-tinted gear emblem, wind-up key.
    let mut dl = Dl::default();
    brass_mat(&mut dl);
    b.emit(&mut dl, &Mesh::prism(10, 0., 420., 1050., 500., [64., 48.], true), 0);
    dl.pipe_sync();
    dl.tlut(false);
    // (ENV - SHADE) * TEXEL0 + SHADE: paint the emblem with the environment colour, like tunic tinting.
    dl.combine([ENVIRONMENT, SHADE, TEXEL0, SHADE, A_ZERO, A_ZERO, A_ZERO, A_ONE], [ENVIRONMENT, SHADE, TEXEL0, SHADE, A_ZERO, A_ZERO, A_ZERO, A_ONE]);
    dl.load_texture_block(seg_addr(gear_off), 4, 0, 32, 32, 0, [2, 2]);
    let mut emblem = Mesh::default();
    let (z, n) = (480.0, Vec3::Z);
    emblem.quad([
        v(Vec3::new(-230., 420., z), n, [0., 32.]),
        v(Vec3::new(230., 420., z), n, [32., 32.]),
        v(Vec3::new(230., 880., z), n, [32., 0.]),
        v(Vec3::new(-230., 880., z), n, [0., 0.]),
    ]);
    b.emit(&mut dl, &emblem, 0);
    rubber_mat(&mut dl);
    b.emit(&mut dl, &Mesh::cube(Vec3::new(-40., 520., -640.), Vec3::new(40., 600., -420.), [1., 1.]).colored([70, 60, 55, 255]), 0);
    dl.pipe_sync();
    dl.render_mode(RM_AA_ZB_TEX_EDGE);
    dl.geometry(G_LIGHTING | G_CULL_BACK, 0);
    dl.combine([TEXEL0, ZERO_AB, SHADE, ZERO_D, A_TEXEL0, A_ZERO, 4, A_ZERO], [TEXEL0, ZERO_AB, SHADE, ZERO_D, A_TEXEL0, A_ZERO, 4, A_ZERO]);
    dl.texture(true);
    dl.load_texture_block(seg_addr(key_off), 3, 1, 32, 32, 0, [2, 2]);
    let mut key = Mesh::default();
    let (x, n) = (0.0, Vec3::X);
    key.quad([
        v(Vec3::new(x, 400., -900.), n, [0., 32.]),
        v(Vec3::new(x, 400., -620.), n, [32., 32.]),
        v(Vec3::new(x, 760., -620.), n, [32., 0.]),
        v(Vec3::new(x, 760., -900.), n, [0., 0.]),
    ]);
    b.emit(&mut dl, &key.colored([235, 235, 240, 255]), 0);
    dl.end();
    dls[2] = Some(dl);

    // Head: neck seam from the torso, brass box, face plate through segment 8.
    let mut dl = Dl::default();
    steel_mat(&mut dl);
    b.emit_seam(&mut dl, mtx(2), mtx(3), 8, (1050., 170.), (80., 170.), [32., 16.]);
    brass_mat(&mut dl);
    b.emit(&mut dl, &Mesh::cube(Vec3::new(-380., 60., -340.), Vec3::new(380., 700., 340.), [32., 32.]), 0);
    dl.pipe_sync();
    dl.tlut(false);
    dl.combine([TEXEL0, ZERO_AB, SHADE, ZERO_D, A_ZERO, A_ZERO, A_ZERO, A_ONE], [TEXEL0, ZERO_AB, SHADE, ZERO_D, A_ZERO, A_ZERO, A_ZERO, A_ONE]);
    dl.load_texture_block((FACE_SEGMENT as u32) << 24, 0, 2, 32, 16, 0, [2, 2]);
    let mut face = Mesh::default();
    let (z, n) = (346.0, Vec3::Z);
    face.quad([
        v(Vec3::new(-300., 170., z), n, [0., 16.]),
        v(Vec3::new(300., 170., z), n, [32., 16.]),
        v(Vec3::new(300., 600., z), n, [32., 0.]),
        v(Vec3::new(-300., 600., z), n, [0., 0.]),
    ]);
    b.emit(&mut dl, &face, 0);
    dl.end();
    dls[3] = Some(dl);

    // Antenna: untextured rod (SHADE * PRIM) and a translucent glowing bulb (PRIM, unlit).
    let mut dl = Dl::default();
    dl.pipe_sync();
    dl.cycle(false);
    dl.texture(false);
    dl.geometry(0, G_LIGHTING | G_CULL_BACK);
    dl.combine([PRIMITIVE, ZERO_AB, SHADE, ZERO_D, A_ZERO, A_ZERO, A_ZERO, A_ONE], [PRIMITIVE, ZERO_AB, SHADE, ZERO_D, A_ZERO, A_ZERO, A_ZERO, A_ONE]);
    dl.prim([120, 125, 135, 255]);
    b.emit(&mut dl, &Mesh::prism(6, 0., 28., 430., 22., [1., 1.], false), 0);
    dl.pipe_sync();
    dl.render_mode(RM_AA_ZB_XLU_SURF);
    dl.geometry(G_LIGHTING | G_CULL_BACK, 0);
    dl.combine([ZERO_AB, ZERO_AB, ZERO_C, PRIMITIVE, A_ZERO, A_ZERO, A_ZERO, A_PRIMITIVE], [ZERO_AB, ZERO_AB, ZERO_C, PRIMITIVE, A_ZERO, A_ZERO, A_ZERO, A_PRIMITIVE]);
    dl.prim([255, 210, 90, 150]);
    b.emit(&mut dl, &Mesh::sphere(Vec3::new(0., 540., 0.), 115., 6, 10).colored([255, 255, 255, 255]), 0);
    dl.end();
    dls[4] = Some(dl);

    // Arms and legs.
    for (upper, lower, side) in [(5usize, 6usize, 1.0f32), (7, 8, -1.0)] {
        let mut dl = Dl::default();
        brass_mat(&mut dl);
        b.emit(&mut dl, &Mesh::sphere(Vec3::ZERO, 150., 5, 8), 0);
        b.emit(&mut dl, &Mesh::prism(8, 0., 125., -520., 105., [32., 32.], false), 0);
        dl.end();
        dls[upper] = Some(dl);

        let mut dl = Dl::default();
        steel_mat(&mut dl);
        b.emit_seam(&mut dl, mtx(upper), mtx(lower), 8, (-520., 105.), (-400., 95.), [32., 24.]);
        rubber_mat(&mut dl);
        let mitt = Mesh::cube(Vec3::new(-95., -620., -115.), Vec3::new(95., -400., 115.), [1., 1.])
            .translate(Vec3::new(side * 10., 0., 20.))
            .colored([60, 50, 48, 255]);
        b.emit(&mut dl, &mitt, 0);
        dl.end();
        dls[lower] = Some(dl);
    }
    for (thigh, shin) in [(9usize, 10usize), (11, 12)] {
        let mut dl = Dl::default();
        steel_mat(&mut dl);
        b.emit(&mut dl, &Mesh::prism(8, 0., 150., -680., 125., [32., 40.], true), 0);
        dl.end();
        dls[thigh] = Some(dl);

        let mut dl = Dl::default();
        steel_mat(&mut dl);
        b.emit_seam(&mut dl, mtx(thigh), mtx(shin), 8, (-680., 125.), (-540., 110.), [32., 24.]);
        rubber_mat(&mut dl);
        let foot = Mesh::cube(Vec3::new(-125., -710., -120.), Vec3::new(125., -540., 280.), [1., 1.]).colored([55, 45, 42, 255]);
        b.emit(&mut dl, &foot, 0);
        dl.end();
        dls[shin] = Some(dl);
    }

    // Serialize DLs, then limbs, the limb pointer table and the flex skeleton header.
    let dl_addrs: Vec<u32> = dls
        .iter()
        .map(|d| match d {
            Some(dl) => {
                let bytes: Vec<u8> = dl.0.iter().flat_map(|(w0, w1)| [w0.to_be_bytes(), w1.to_be_bytes()].concat()).collect();
                seg_addr(b.put(&bytes))
            }
            None => 0,
        })
        .collect();
    let limb_offs: Vec<usize> = defs
        .iter()
        .zip(&dl_addrs)
        .map(|(d, &dl)| {
            let mut l = Vec::with_capacity(16);
            for c in d.pos {
                l.extend_from_slice(&c.to_be_bytes());
            }
            l.push(d.child);
            l.push(d.sibling);
            l.extend_from_slice(&dl.to_be_bytes()); // near
            l.extend_from_slice(&dl.to_be_bytes()); // far
            b.put(&l)
        })
        .collect();
    let table: Vec<u8> = limb_offs.iter().flat_map(|&o| seg_addr(o).to_be_bytes()).collect();
    let table_off = b.put(&table);
    let dl_count = dl_addrs.iter().filter(|&&a| a != 0).count() as u8;
    let mut hdr = seg_addr(table_off).to_be_bytes().to_vec();
    hdr.extend_from_slice(&[defs.len() as u8, 0, 0, 0, dl_count, 0, 0, 0]);
    let skeleton_offset = b.put(&hdr);

    let animations = anims::all().into_iter().map(|(name, frames)| (name.to_string(), encode_animation(&mut b, &frames))).collect();

    SynthObject { data: b.data, skeleton_offset, animations, face_textures: faces, limb_names: LIMB_NAMES.to_vec() }
}

/// Encodes frames (each: root translation + one rotation per limb) as a compressed
/// `AnimationHeader`: constant components are stored once below `staticIndexMax`, animated
/// components as contiguous per-frame tracks above it.
fn encode_animation(b: &mut Builder, frames: &[Vec<[i16; 3]>]) -> usize {
    let n_frames = frames.len();
    let joints = frames[0].len();
    let mut statics: Vec<i16> = Vec::new();
    let mut tracks: Vec<Vec<i16>> = Vec::new();
    let mut refs: Vec<(bool, usize)> = Vec::new();
    for j in 0..joints {
        for k in 0..3 {
            let track: Vec<i16> = frames.iter().map(|f| f[j][k]).collect();
            if track.iter().all(|&x| x == track[0]) {
                let idx = statics.iter().position(|&s| s == track[0]).unwrap_or_else(|| {
                    statics.push(track[0]);
                    statics.len() - 1
                });
                refs.push((false, idx));
            } else {
                tracks.push(track);
                refs.push((true, tracks.len() - 1));
            }
        }
    }
    let static_max = statics.len();
    let mut data: Vec<i16> = statics;
    for t in &tracks {
        data.extend_from_slice(t);
    }
    let indices: Vec<u8> = refs
        .iter()
        .map(|&(dynamic, i)| if dynamic { (static_max + i * n_frames) as u16 } else { i as u16 })
        .flat_map(|x| x.to_be_bytes())
        .collect();
    let data_off = b.put(&data.iter().flat_map(|x| x.to_be_bytes()).collect::<Vec<_>>());
    let idx_off = b.put(&indices);
    let mut hdr = Vec::with_capacity(16);
    hdr.extend_from_slice(&(n_frames as i16).to_be_bytes());
    hdr.extend_from_slice(&[0, 0]);
    hdr.extend_from_slice(&seg_addr(data_off).to_be_bytes());
    hdr.extend_from_slice(&seg_addr(idx_off).to_be_bytes());
    hdr.extend_from_slice(&(static_max as u16).to_be_bytes());
    hdr.extend_from_slice(&[0, 0]);
    b.put(&hdr)
}

mod anims {
    use std::f32::consts::TAU;

    fn deg(d: f32) -> i16 {
        ((d / 360.0 * 65536.0).round() as i32) as i16
    }

    /// One frame: [root translation, then rotation (x, y, z) for each of the 13 limbs].
    type Frame = Vec<[i16; 3]>;

    fn frame(root_y: f32, rots: &[(usize, [f32; 3])]) -> Frame {
        let mut f = vec![[0i16; 3]; 14];
        f[0] = [0, root_y.round() as i16, 0];
        for &(limb, [x, y, z]) in rots {
            f[limb + 1] = [deg(x), deg(y), deg(z)];
        }
        f
    }

    fn idle() -> Vec<Frame> {
        (0..60)
            .map(|i| {
                let p = i as f32 / 60.0 * TAU;
                frame(1450.0 + 10.0 * p.sin(), &[
                    (2, [2.0 * p.sin(), 0.0, 0.0]),
                    (3, [-2.0 * p.sin(), 18.0 * p.sin(), 0.0]),
                    (4, [0.0, 0.0, 8.0 * (p + 0.8).sin()]),
                    (5, [0.0, 0.0, 8.0 + 3.0 * p.sin()]),
                    (6, [-12.0, 0.0, 0.0]),
                    (7, [0.0, 0.0, -8.0 - 3.0 * p.sin()]),
                    (8, [-12.0, 0.0, 0.0]),
                ])
            })
            .collect()
    }

    fn walk() -> Vec<Frame> {
        (0..32)
            .map(|i| {
                let p = i as f32 / 32.0 * TAU;
                let knee = |ph: f32| 8.0 + 45.0 * (-(ph.cos())).max(0.0);
                frame(1440.0 + 30.0 * (2.0 * p).cos(), &[
                    (0, [0.0, 5.0 * p.sin(), 0.0]),
                    (1, [0.0, 0.0, 3.0 * p.sin()]),
                    (2, [5.0, -7.0 * p.sin(), 0.0]),
                    (3, [-4.0, 5.0 * p.sin(), 0.0]),
                    (4, [14.0 * (2.0 * p + 1.0).sin(), 0.0, 0.0]),
                    (5, [26.0 * p.sin(), 0.0, 7.0]),
                    (6, [-25.0 + 10.0 * p.sin(), 0.0, 0.0]),
                    (7, [-26.0 * p.sin(), 0.0, -7.0]),
                    (8, [-25.0 - 10.0 * p.sin(), 0.0, 0.0]),
                    (9, [-30.0 * p.sin(), 0.0, 0.0]),
                    (10, [knee(p), 0.0, 0.0]),
                    (11, [30.0 * p.sin(), 0.0, 0.0]),
                    (12, [knee(p + std::f32::consts::PI), 0.0, 0.0]),
                ])
            })
            .collect()
    }

    fn wave() -> Vec<Frame> {
        (0..40)
            .map(|i| {
                let p = i as f32 / 40.0 * TAU;
                frame(1450.0, &[
                    (2, [0.0, 0.0, 3.0]),
                    (3, [0.0, -10.0, 9.0 * (p * 2.0).sin()]),
                    (4, [0.0, 0.0, -12.0 * (p * 2.0).sin()]),
                    (5, [0.0, 0.0, 8.0]),
                    (6, [-10.0, 0.0, 0.0]),
                    (7, [0.0, 0.0, -150.0]),
                    (8, [0.0, 0.0, 25.0 * (p * 2.0).sin()]),
                ])
            })
            .collect()
    }

    /// Full turn about Y: exercises binary-angle wrap-around in playback interpolation.
    fn spin() -> Vec<Frame> {
        (0..24)
            .map(|i| {
                let t = i as f32 / 24.0;
                frame(1450.0 + 120.0 * (t * TAU).sin().abs(), &[
                    (0, [0.0, 360.0 * t, 0.0]),
                    (5, [0.0, 0.0, 80.0]),
                    (7, [0.0, 0.0, -80.0]),
                    (4, [0.0, 0.0, -25.0]),
                ])
            })
            .collect()
    }

    pub fn all() -> Vec<(&'static str, Vec<Frame>)> {
        vec![("idle", idle()), ("walk", walk()), ("wave", wave()), ("spin", spin()), ("rest", vec![frame(1450.0, &[])])]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::z64::ParseStandardAnimation;
    use eng_anim::anim::{Animation, StandardAnimation};
    use eng_gbi::model::{Binding, BuildOptions, build_draw_list};
    use std::sync::Arc;

    #[test]
    fn synthetic_object_decodes_cleanly() {
        let obj = build();
        let skel = obj.skeleton();
        assert_eq!(skel.limbs.len(), 13);
        assert_eq!(skel.flex_matrix_map(0).len(), skel.dlist_count as usize);
        let buf: Arc<[u8]> = obj.data.clone().into();
        let opts = BuildOptions {
            bindings: vec![
                Binding { segment: SEGMENT, buf: buf.clone(), base: 0 },
                Binding { segment: FACE_SEGMENT, buf: buf.clone(), base: obj.face_textures[0].1 },
            ],
            env_color: Some([200, 60, 40, 255]),
            ..Default::default()
        };
        let draw = build_draw_list(&skel, &opts).unwrap();
        assert!(draw.stats.unknown_opcodes.is_empty(), "{:?}", draw.stats.unknown_opcodes);
        assert!(draw.stats.unresolved_addresses.is_empty(), "{:?}", draw.stats.unresolved_addresses);
        assert!(draw.triangle_count() > 500);
        // CI4 brass, I4 gear, I8 stripes, IA8 key, RGBA16 face.
        assert_eq!(draw.textures.len(), 5);
        // Seam rings are bound to the parent limb: the forearm DL must emit vertices for the upper arm bone.
        let bones: std::collections::HashSet<u16> = draw.batches.iter().flat_map(|b| b.vertices.iter().map(|v| v.bone)).collect();
        assert!(bones.contains(&5) && bones.contains(&6));
        for (name, off) in &obj.animations {
            let a = StandardAnimation::parse(&obj.data, SEGMENT, *off, skel.limbs.len()).unwrap();
            assert!(a.frame_count() >= 1, "{name}");
            assert_eq!(a.sample(0).rot.len(), 14);
        }
        let walk = StandardAnimation::parse(&obj.data, SEGMENT, obj.animations[1].1, 13).unwrap();
        assert_eq!(walk.frame_count(), 32);
    }
}
