//! Scene/actor collision in the game's binary `CollisionHeader` format (`z64bgcheck.h`):
//! a 0x2C-byte header pointing at Vec3s vertices, 16-byte `CollisionPoly`s, `SurfaceType`
//! pairs and `WaterBox`es. Decoding reads it from a ROM file; encoding writes the same layout
//! so synthetic test geometry goes through the identical path.

use anyhow::{Result, bail};
use glam::Vec3;

/// `COLPOLY_VTX_INDEX`: the low 13 bits of a vertex index; the top 3 bits are flags.
pub const VTX_INDEX_MASK: u16 = 0x1FFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CollisionPoly {
    /// Index into `surface_types`.
    pub ty: u16,
    /// `flags_vIA`, `flags_vIB`, `vIC`. The top 3 bits of the first are the exclusion
    /// flags (`COLPOLY_IGNORE_*`), the top bits of the second are conveyor flags.
    pub vtx: [u16; 3],
    /// Unit normal scaled to ±0x7FFF.
    pub normal: [i16; 3],
    /// Plane distance from the origin: `n·p + dist = 0` for points on the plane.
    pub dist: i16,
}

impl CollisionPoly {
    pub fn a(&self) -> usize {
        (self.vtx[0] & VTX_INDEX_MASK) as usize
    }
    pub fn b(&self) -> usize {
        (self.vtx[1] & VTX_INDEX_MASK) as usize
    }
    pub fn c(&self) -> usize {
        self.vtx[2] as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SurfaceType {
    pub data: [u32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WaterBox {
    pub x_min: i16,
    pub y_surface: i16,
    pub z_min: i16,
    pub x_length: i16,
    pub z_length: i16,
    pub properties: u32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CollisionHeader {
    pub min_bounds: [i16; 3],
    pub max_bounds: [i16; 3],
    pub vertices: Vec<[i16; 3]>,
    pub polys: Vec<CollisionPoly>,
    pub surface_types: Vec<SurfaceType>,
    pub water_boxes: Vec<WaterBox>,
}

fn be16(b: &[u8], o: usize) -> u16 {
    u16::from_be_bytes([b[o], b[o + 1]])
}
fn be32(b: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

impl CollisionHeader {
    /// Decodes a header at `offset` in `file`, whose pointers are in `segment`.
    pub fn parse(file: &[u8], segment: u8, offset: usize) -> Result<CollisionHeader> {
        let local = |addr: u32, len: usize, what: &str| -> Result<usize> {
            if addr == 0 && len == 0 {
                return Ok(0);
            }
            if (addr >> 24) as u8 != segment {
                bail!("{what} pointer {addr:08X} is not in segment {segment:02X}");
            }
            let o = (addr & 0xFF_FFFF) as usize;
            if o + len > file.len() {
                bail!("{what} {o:X}+{len:X} outside file ({:X})", file.len());
            }
            Ok(o)
        };
        if offset + 0x2C > file.len() {
            bail!("collision header outside file");
        }
        let h = &file[offset..offset + 0x2C];
        let s3 = |o: usize| [be16(h, o) as i16, be16(h, o + 2) as i16, be16(h, o + 4) as i16];
        let num_vtx = be16(h, 0x0C) as usize;
        let num_polys = be16(h, 0x14) as usize;
        let num_water = be16(h, 0x24) as usize;

        let vo = local(be32(h, 0x10), num_vtx * 6, "vertex list")?;
        let vertices = (0..num_vtx)
            .map(|i| {
                let o = vo + i * 6;
                [be16(file, o) as i16, be16(file, o + 2) as i16, be16(file, o + 4) as i16]
            })
            .collect();
        let po = local(be32(h, 0x18), num_polys * 16, "poly list")?;
        let polys: Vec<CollisionPoly> = (0..num_polys)
            .map(|i| {
                let o = po + i * 16;
                CollisionPoly {
                    ty: be16(file, o),
                    vtx: [be16(file, o + 2), be16(file, o + 4), be16(file, o + 6)],
                    normal: [be16(file, o + 8) as i16, be16(file, o + 10) as i16, be16(file, o + 12) as i16],
                    dist: be16(file, o + 14) as i16,
                }
            })
            .collect();
        for (i, p) in polys.iter().enumerate() {
            if p.a() >= num_vtx || p.b() >= num_vtx || p.c() >= num_vtx {
                bail!("poly {i} references a vertex beyond {num_vtx}");
            }
        }
        // The header does not store the surface type count; it is one past the largest used.
        let num_types = polys.iter().map(|p| p.ty as usize + 1).max().unwrap_or(0);
        let so = local(be32(h, 0x1C), num_types * 8, "surface type list")?;
        let surface_types = (0..num_types)
            .map(|i| SurfaceType { data: [be32(file, so + i * 8), be32(file, so + i * 8 + 4)] })
            .collect();
        let wo = local(be32(h, 0x28), num_water * 16, "water box list")?;
        let water_boxes = (0..num_water)
            .map(|i| {
                let o = wo + i * 16;
                WaterBox {
                    x_min: be16(file, o) as i16,
                    y_surface: be16(file, o + 2) as i16,
                    z_min: be16(file, o + 4) as i16,
                    x_length: be16(file, o + 6) as i16,
                    z_length: be16(file, o + 8) as i16,
                    properties: be32(file, o + 12),
                }
            })
            .collect();
        Ok(CollisionHeader { min_bounds: s3(0), max_bounds: s3(6), vertices, polys, surface_types, water_boxes })
    }

    /// Encodes the header and its lists into one buffer mapped at `segment`, header first.
    /// `parse(&bytes, segment, 0)` returns an equal header.
    pub fn encode(&self, segment: u8) -> Vec<u8> {
        let mut out = vec![0u8; 0x2C];
        let seg = |o: usize| ((segment as u32) << 24) | o as u32;
        let align4 = |v: &mut Vec<u8>| v.resize(v.len().div_ceil(4) * 4, 0);

        let vtx_off = out.len();
        for v in &self.vertices {
            for c in v {
                out.extend_from_slice(&c.to_be_bytes());
            }
        }
        align4(&mut out);
        let poly_off = out.len();
        for p in &self.polys {
            out.extend_from_slice(&p.ty.to_be_bytes());
            for v in p.vtx {
                out.extend_from_slice(&v.to_be_bytes());
            }
            for n in p.normal {
                out.extend_from_slice(&n.to_be_bytes());
            }
            out.extend_from_slice(&p.dist.to_be_bytes());
        }
        let surf_off = out.len();
        for s in &self.surface_types {
            out.extend_from_slice(&s.data[0].to_be_bytes());
            out.extend_from_slice(&s.data[1].to_be_bytes());
        }
        let water_off = out.len();
        for w in &self.water_boxes {
            for v in [w.x_min, w.y_surface, w.z_min, w.x_length, w.z_length, 0] {
                out.extend_from_slice(&v.to_be_bytes());
            }
            out.extend_from_slice(&w.properties.to_be_bytes());
        }

        let mut h = Vec::with_capacity(0x2C);
        for c in self.min_bounds.iter().chain(&self.max_bounds) {
            h.extend_from_slice(&c.to_be_bytes());
        }
        h.extend_from_slice(&(self.vertices.len() as u16).to_be_bytes());
        h.extend_from_slice(&[0, 0]);
        h.extend_from_slice(&seg(vtx_off).to_be_bytes());
        h.extend_from_slice(&(self.polys.len() as u16).to_be_bytes());
        h.extend_from_slice(&[0, 0]);
        h.extend_from_slice(&seg(poly_off).to_be_bytes());
        h.extend_from_slice(&seg(surf_off).to_be_bytes());
        h.extend_from_slice(&0u32.to_be_bytes()); // bgCamList: none
        h.extend_from_slice(&(self.water_boxes.len() as u16).to_be_bytes());
        h.extend_from_slice(&[0, 0]);
        let water_ptr = if self.water_boxes.is_empty() { 0 } else { seg(water_off) };
        h.extend_from_slice(&water_ptr.to_be_bytes());
        out[..0x2C].copy_from_slice(&h);
        out
    }

    pub fn vertex(&self, i: usize) -> Vec3 {
        let v = self.vertices[i];
        Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32)
    }

    pub fn triangle(&self, p: &CollisionPoly) -> [Vec3; 3] {
        [self.vertex(p.a()), self.vertex(p.b()), self.vertex(p.c())]
    }
}

/// Builds collision geometry the way the game's tools do: vertices are deduplicated, each
/// triangle's normal is the normalized cross product of its edges scaled to s16, and `dist`
/// is `-n·v0` rounded to s16.
#[derive(Debug, Default, Clone)]
pub struct CollisionBuilder {
    pub header: CollisionHeader,
}

impl CollisionBuilder {
    pub fn new() -> CollisionBuilder {
        CollisionBuilder::default()
    }

    /// Adds a surface type and returns its index.
    pub fn surface(&mut self, data0: u32, data1: u32) -> u16 {
        let s = SurfaceType { data: [data0, data1] };
        if let Some(i) = self.header.surface_types.iter().position(|t| *t == s) {
            return i as u16;
        }
        self.header.surface_types.push(s);
        (self.header.surface_types.len() - 1) as u16
    }

    fn vertex(&mut self, p: Vec3) -> u16 {
        let v = [p.x.round() as i16, p.y.round() as i16, p.z.round() as i16];
        if let Some(i) = self.header.vertices.iter().position(|&w| w == v) {
            return i as u16;
        }
        self.header.vertices.push(v);
        (self.header.vertices.len() - 1) as u16
    }

    /// Adds a triangle (counter-clockwise when viewed from the side the normal faces).
    pub fn tri(&mut self, a: Vec3, b: Vec3, c: Vec3, ty: u16) {
        let (ia, ib, ic) = (self.vertex(a), self.vertex(b), self.vertex(c));
        let (va, vb, vc) = (self.header.vertex(ia as usize), self.header.vertex(ib as usize), self.header.vertex(ic as usize));
        let n = (vb - va).cross(vc - va);
        if n.length_squared() < 1e-6 {
            return;
        }
        let n = n.normalize();
        let normal = [(n.x * 32767.0) as i16, (n.y * 32767.0) as i16, (n.z * 32767.0) as i16];
        let dist = (-n.dot(va)).round() as i16;
        self.header.polys.push(CollisionPoly { ty, vtx: [ia, ib, ic], normal, dist });
    }

    /// Adds a water box (`WaterBox`); `room` 0x3F is `WATERBOX_ROOM_ALL`.
    pub fn water_box(&mut self, x_min: i16, z_min: i16, x_length: i16, z_length: i16, y_surface: i16, room: u32) {
        self.header.water_boxes.push(WaterBox { x_min, y_surface, z_min, x_length, z_length, properties: (room & 0x3F) << 13 });
    }

    /// Adds a quad as two triangles; corners counter-clockwise from the facing side.
    pub fn quad(&mut self, a: Vec3, b: Vec3, c: Vec3, d: Vec3, ty: u16) {
        self.tri(a, b, c, ty);
        self.tri(a, c, d, ty);
    }

    pub fn finish(mut self) -> CollisionHeader {
        let h = &mut self.header;
        let mut min = [i16::MAX; 3];
        let mut max = [i16::MIN; 3];
        for v in &h.vertices {
            for k in 0..3 {
                min[k] = min[k].min(v[k]);
                max[k] = max[k].max(v[k]);
            }
        }
        if h.vertices.is_empty() {
            min = [0; 3];
            max = [0; 3];
        }
        h.min_bounds = min;
        h.max_bounds = max;
        self.header
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_parse_roundtrip() {
        let mut b = CollisionBuilder::new();
        let floor = b.surface(0, 0x0000_07C0);
        let wall = b.surface(0x0020_0000, 0);
        b.quad(Vec3::new(-100.0, 0.0, 100.0), Vec3::new(100.0, 0.0, 100.0), Vec3::new(100.0, 0.0, -100.0), Vec3::new(-100.0, 0.0, -100.0), floor);
        b.quad(Vec3::new(-100.0, 0.0, -100.0), Vec3::new(100.0, 0.0, -100.0), Vec3::new(100.0, 80.0, -100.0), Vec3::new(-100.0, 80.0, -100.0), wall);
        let mut h = b.finish();
        h.water_boxes.push(WaterBox { x_min: -10, y_surface: -5, z_min: -10, x_length: 20, z_length: 20, properties: 0x3F << 13 });
        let bytes = h.encode(2);
        let back = CollisionHeader::parse(&bytes, 2, 0).unwrap();
        assert_eq!(back, h);
        // Floor faces up, wall faces +z (towards the floor's centre).
        assert_eq!(back.polys[0].normal, [0, 32767, 0]);
        assert_eq!(back.polys[2].normal, [0, 0, 32767]);
        assert_eq!(back.polys[2].dist, 100);
        assert_eq!(back.min_bounds, [-100, 0, -100]);
    }
}
