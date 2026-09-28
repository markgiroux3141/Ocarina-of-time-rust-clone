//! Decoders for the z64 engine's binary asset formats: skeleton headers and limbs, standard
//! and Link animations, and collision headers (plus the collision encoder the synthetic test
//! course uses). They build the engine's types (`eng_anim`, `eng_collision`) and are provided
//! as extension traits, so `Skeleton::parse(..)` reads as before with the trait in scope
//! (`use oot_import::z64::*`).

use anyhow::{Context, Result, bail};
use eng_anim::anim::{JointTable, LINK_ANIM_FRAME_BYTES, LINK_ANIM_JOINTS, LinkAnimation, StandardAnimation};
use eng_anim::skeleton::{Limb, LimbType, Skeleton};
use eng_collision::collision::{CollisionHeader, CollisionPoly, SurfaceType, WaterBox};

fn be16(b: &[u8], o: usize) -> i16 {
    i16::from_be_bytes([b[o], b[o + 1]])
}
fn be32(b: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
fn beu16(b: &[u8], o: usize) -> u16 {
    u16::from_be_bytes([b[o], b[o + 1]])
}

/// `SkeletonHeader` / `FlexSkeletonHeader` with `StandardLimb` or `LodLimb` limbs.
pub trait ParseSkeleton: Sized {
    fn parse(file: &[u8], segment: u8, offset: usize, limb_type: LimbType, flex: bool) -> Result<Self>;
}

impl ParseSkeleton for Skeleton {
    /// Parse a skeleton header at `offset` within `file`, which is mapped to `segment`.
    fn parse(file: &[u8], segment: u8, offset: usize, limb_type: LimbType, flex: bool) -> Result<Skeleton> {
        let local = |addr: u32| -> Result<usize> {
            if (addr >> 24) as u8 != segment {
                bail!("pointer {addr:08X} is not in segment {segment:02X}");
            }
            let o = (addr & 0xFF_FFFF) as usize;
            if o >= file.len() {
                bail!("pointer {addr:08X} outside file");
            }
            Ok(o)
        };
        if offset + 8 > file.len() {
            bail!("skeleton header outside file");
        }
        let limb_table = local(be32(file, offset)).context("skeleton limb table")?;
        let limb_count = file[offset + 4] as usize;
        let dlist_count = if flex { file[offset + 8] } else { 0 };
        let mut limbs = Vec::with_capacity(limb_count);
        for i in 0..limb_count {
            let lo = local(be32(file, limb_table + i * 4)).with_context(|| format!("limb {i}"))?;
            let opt = |v: u8| if v == 0xFF { None } else { Some(v) };
            let n_dl = if limb_type == LimbType::Lod { 2 } else { 1 };
            limbs.push(Limb {
                joint_pos: [be16(file, lo), be16(file, lo + 2), be16(file, lo + 4)],
                child: opt(file[lo + 6]),
                sibling: opt(file[lo + 7]),
                dlists: (0..n_dl).map(|k| be32(file, lo + 8 + k * 4)).collect(),
            });
        }
        let mut parents = vec![None; limb_count];
        let mut draw_order = Vec::with_capacity(limb_count);
        // Iterative version of the game's recursive walk.
        let mut stack: Vec<(u8, Option<u8>)> = vec![(0, None)];
        while let Some((idx, parent)) = stack.pop() {
            let Some(limb) = limbs.get(idx as usize) else { bail!("limb index {idx} out of range") };
            if draw_order.contains(&idx) {
                bail!("limb graph has a cycle at {idx}");
            }
            parents[idx as usize] = parent;
            draw_order.push(idx);
            if let Some(s) = limb.sibling {
                stack.push((s, parent));
            }
            if let Some(c) = limb.child {
                stack.push((c, Some(idx)));
            }
        }
        Ok(Skeleton { limbs, limb_type, flex, dlist_count, parents, draw_order })
    }
}

/// `AnimationHeader`.
pub trait ParseStandardAnimation: Sized {
    fn parse(file: &[u8], segment: u8, offset: usize, limb_count: usize) -> Result<Self>;
}

impl ParseStandardAnimation for StandardAnimation {
    /// `limb_count` is the skeleton's limb count; the animation has limb_count + 1 joint entries.
    fn parse(file: &[u8], segment: u8, offset: usize, limb_count: usize) -> Result<StandardAnimation> {
        let local = |addr: u32| -> Result<usize> {
            if (addr >> 24) as u8 != segment {
                bail!("pointer {addr:08X} is not in segment {segment:02X}");
            }
            Ok((addr & 0xFF_FFFF) as usize)
        };
        if offset + 0x10 > file.len() {
            bail!("animation header outside file");
        }
        let frame_count = be16(file, offset).max(1) as usize;
        let data = local(be32(file, offset + 4))?;
        let indices = local(be32(file, offset + 8))?;
        let static_max = u16::from_be_bytes([file[offset + 12], file[offset + 13]]) as usize;
        let joints = limb_count + 1;
        let rd = |i: usize| -> i16 {
            let o = data + i * 2;
            if o + 2 <= file.len() { be16(file, o) } else { 0 }
        };
        let mut frames = Vec::with_capacity(frame_count);
        for f in 0..frame_count {
            let mut jt = JointTable::zeroed(joints);
            for j in 0..joints {
                let io = indices + j * 6;
                if io + 6 > file.len() {
                    bail!("joint index table outside file");
                }
                for k in 0..3 {
                    let idx = u16::from_be_bytes([file[io + k * 2], file[io + k * 2 + 1]]) as usize;
                    jt.rot[j][k] = if idx >= static_max { rd(idx + f) } else { rd(idx) };
                }
            }
            frames.push(jt);
        }
        Ok(StandardAnimation { frames })
    }
}

/// `LinkAnimationHeader` and its frames in `link_animetion`.
pub trait ParseLinkAnimation: Sized {
    fn parse(header: &[u8], link_animetion: &[u8]) -> Result<Self>;
    fn from_data(link_animetion: &[u8], data_offset: usize, frame_count: usize) -> Result<Self>;
}

impl ParseLinkAnimation for LinkAnimation {
    /// `header` is the LinkAnimationHeader bytes (frame count + segment-7 pointer).
    fn parse(header: &[u8], link_animetion: &[u8]) -> Result<LinkAnimation> {
        let frame_count = be16(header, 0).max(1) as usize;
        let ptr = be32(header, 4);
        if ptr >> 24 != 0x07 {
            bail!("Link animation pointer {ptr:08X} is not in segment 07");
        }
        let data_offset = (ptr & 0xFF_FFFF) as usize;
        Self::from_data(link_animetion, data_offset, frame_count)
    }

    fn from_data(link_animetion: &[u8], data_offset: usize, frame_count: usize) -> Result<LinkAnimation> {
        let end = data_offset + frame_count * LINK_ANIM_FRAME_BYTES;
        if end > link_animetion.len() {
            bail!("Link animation data {data_offset:X}..{end:X} outside link_animetion");
        }
        let frames = (0..frame_count)
            .map(|f| {
                let base = data_offset + f * LINK_ANIM_FRAME_BYTES;
                let rot = (0..LINK_ANIM_JOINTS)
                    .map(|j| {
                        let o = base + j * 6;
                        [be16(link_animetion, o), be16(link_animetion, o + 2), be16(link_animetion, o + 4)]
                    })
                    .collect();
                let face = u16::from_be_bytes([
                    link_animetion[base + LINK_ANIM_JOINTS * 6],
                    link_animetion[base + LINK_ANIM_JOINTS * 6 + 1],
                ]);
                JointTable { rot, face }
            })
            .collect();
        Ok(LinkAnimation { frames, data_offset })
    }
}

/// `CollisionHeader`: a 0x2C-byte header pointing at the vertex, poly, surface type and water
/// box lists.
pub trait CollisionCodec: Sized {
    fn parse(file: &[u8], segment: u8, offset: usize) -> Result<Self>;
    fn encode(&self, segment: u8) -> Vec<u8>;
}

impl CollisionCodec for CollisionHeader {
    /// Decodes a header at `offset` in `file`, whose pointers are in `segment`.
    fn parse(file: &[u8], segment: u8, offset: usize) -> Result<CollisionHeader> {
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
        let s3 = |o: usize| [beu16(h, o) as i16, beu16(h, o + 2) as i16, beu16(h, o + 4) as i16];
        let num_vtx = beu16(h, 0x0C) as usize;
        let num_polys = beu16(h, 0x14) as usize;
        let num_water = beu16(h, 0x24) as usize;

        let vo = local(be32(h, 0x10), num_vtx * 6, "vertex list")?;
        let vertices = (0..num_vtx)
            .map(|i| {
                let o = vo + i * 6;
                [beu16(file, o) as i16, beu16(file, o + 2) as i16, beu16(file, o + 4) as i16]
            })
            .collect();
        let po = local(be32(h, 0x18), num_polys * 16, "poly list")?;
        let polys: Vec<CollisionPoly> = (0..num_polys)
            .map(|i| {
                let o = po + i * 16;
                CollisionPoly {
                    ty: beu16(file, o),
                    vtx: [beu16(file, o + 2), beu16(file, o + 4), beu16(file, o + 6)],
                    normal: [beu16(file, o + 8) as i16, beu16(file, o + 10) as i16, beu16(file, o + 12) as i16],
                    dist: beu16(file, o + 14) as i16,
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
                    x_min: beu16(file, o) as i16,
                    y_surface: beu16(file, o + 2) as i16,
                    z_min: beu16(file, o + 4) as i16,
                    x_length: beu16(file, o + 6) as i16,
                    z_length: beu16(file, o + 8) as i16,
                    properties: be32(file, o + 12),
                }
            })
            .collect();
        Ok(CollisionHeader { min_bounds: s3(0), max_bounds: s3(6), vertices, polys, surface_types, water_boxes })
    }

    /// Encodes the header and its lists into one buffer mapped at `segment`, header first.
    /// `parse(&bytes, segment, 0)` returns an equal header.
    fn encode(&self, segment: u8) -> Vec<u8> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use eng_collision::collision::CollisionBuilder;
    use glam::Vec3;

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
