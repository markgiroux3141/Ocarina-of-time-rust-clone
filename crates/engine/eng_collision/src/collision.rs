//! Collision meshes with the game's `CollisionHeader` layout (`z64bgcheck.h`): Vec3s
//! vertices, `CollisionPoly`s, `SurfaceType` pairs and `WaterBox`es. The binary codec (from a
//! ROM file, and back for synthetic geometry) is in `oot_import::z64`.

use glam::Vec3;

/// `COLPOLY_VTX_INDEX`: the low 13 bits of a vertex index; the top 3 bits are flags.
pub const VTX_INDEX_MASK: u16 = 0x1FFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct SurfaceType {
    pub data: [u32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct WaterBox {
    pub x_min: i16,
    pub y_surface: i16,
    pub z_min: i16,
    pub x_length: i16,
    pub z_length: i16,
    pub properties: u32,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CollisionHeader {
    pub min_bounds: [i16; 3],
    pub max_bounds: [i16; 3],
    pub vertices: Vec<[i16; 3]>,
    pub polys: Vec<CollisionPoly>,
    pub surface_types: Vec<SurfaceType>,
    pub water_boxes: Vec<WaterBox>,
}

impl CollisionHeader {
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
