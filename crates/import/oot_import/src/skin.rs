//! Skin skeletons (`z64skin.h`) read from an object file: the runtime's `SkinSkeleton`, and the
//! mesh bake that gives every animated limb's vertex group a bone of its own
//! (`oot_game::skin`).

use std::sync::Arc;

use anyhow::{Context, Result, bail};
use eng_gbi::gbi::{BoneId, DrawList, Interpreter, NO_BONE, Segment};
use oot_game::skin::{SKIN_LIMB_TYPE_ANIMATED, SKIN_LIMB_TYPE_NORMAL, SkinLimb, SkinLimbModif, SkinSkeleton, SkinTransformation};

/// `SkinVertex`: a vertex of a group, as `Skin_InitAnimatedLimb` writes it into the limb's
/// vertex buffer (its texture coordinates, normal and alpha).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkinVertex {
    pub index: u16,
    pub s: i16,
    pub t: i16,
    pub normal: [i8; 3],
    pub alpha: u8,
}

/// A skin skeleton with what only the bake needs: each limb's display list (a normal limb's
/// `segment`, an animated limb's `SkinAnimatedLimbData.dlist`), an animated limb's
/// `totalVtxCount` and its groups' vertices.
#[derive(Debug, Clone, PartialEq)]
pub struct RawSkin {
    pub skeleton: SkinSkeleton,
    pub dlists: Vec<u32>,
    pub total_vtx: Vec<u16>,
    pub vertices: Vec<Vec<Vec<SkinVertex>>>,
}

fn be16(b: &[u8], o: usize) -> Result<u16> {
    Ok(u16::from_be_bytes(b.get(o..o + 2).context("past the file's end")?.try_into()?))
}
fn be32(b: &[u8], o: usize) -> Result<u32> {
    Ok(u32::from_be_bytes(b.get(o..o + 4).context("past the file's end")?.try_into()?))
}

/// A pointer into the file on segment `seg`, as an offset.
fn off(ptr: u32, seg: u8) -> Result<usize> {
    if (ptr >> 24) as u8 != seg {
        bail!("pointer {ptr:08X} not on segment {seg:#x}");
    }
    Ok((ptr & 0x00FF_FFFF) as usize)
}

/// Parses the `SkeletonHeader` at `offset` (`segment`, `limbCount`) and its `SkinLimb`s.
pub fn parse(data: &[u8], seg: u8, offset: usize) -> Result<RawSkin> {
    let limbs_at = off(be32(data, offset)?, seg)?;
    let count = data[offset + 4] as usize;
    let mut raw = RawSkin { skeleton: SkinSkeleton { limbs: Vec::new() }, dlists: Vec::new(), total_vtx: Vec::new(), vertices: Vec::new() };
    for i in 0..count {
        let l = off(be32(data, limbs_at + i * 4)?, seg)?;
        let joint_pos = [be16(data, l)? as i16, be16(data, l + 2)? as i16, be16(data, l + 4)? as i16];
        let (child, sibling) = (data[l + 6], data[l + 7]);
        let segment_type = be32(data, l + 8)? as i32;
        let segment = be32(data, l + 12)?;
        let mut limb = SkinLimb { joint_pos, child, sibling, segment_type, has_segment: segment != 0, modifs: Vec::new() };
        let mut dlist = 0;
        let mut total = 0;
        let mut verts = Vec::new();
        if segment != 0 && segment_type == SKIN_LIMB_TYPE_NORMAL {
            dlist = segment;
        } else if segment != 0 && segment_type == SKIN_LIMB_TYPE_ANIMATED {
            // SkinAnimatedLimbData: totalVtxCount, limbModifCount, limbModifications, dlist.
            let d = off(segment, seg)?;
            total = be16(data, d)?;
            let modif_count = be16(data, d + 2)? as usize;
            let modifs_at = off(be32(data, d + 4)?, seg)?;
            dlist = be32(data, d + 8)?;
            for k in 0..modif_count {
                // SkinLimbModif: vtxCount, transformCount, unk_4, skinVertices, limbTransformations.
                let m = modifs_at + k * 0x10;
                let vtx_count = be16(data, m)?;
                let transform_count = be16(data, m + 2)? as usize;
                let unk_4 = be16(data, m + 4)?;
                let sv = off(be32(data, m + 8)?, seg)?;
                let tr = off(be32(data, m + 12)?, seg)?;
                let mut transformations = Vec::new();
                for t in 0..transform_count {
                    // SkinTransformation: limbIndex, x, y, z, scale (size 0xA).
                    let e = tr + t * 0xA;
                    transformations.push(SkinTransformation { limb_index: data[e], x: be16(data, e + 2)? as i16, y: be16(data, e + 4)? as i16, z: be16(data, e + 6)? as i16, scale: data[e + 8] });
                }
                let mut group = Vec::new();
                for v in 0..vtx_count as usize {
                    // SkinVertex: index, s, t, normX, normY, normZ, alpha (size 0xA).
                    let e = sv + v * 0xA;
                    group.push(SkinVertex {
                        index: be16(data, e)?,
                        s: be16(data, e + 2)? as i16,
                        t: be16(data, e + 4)? as i16,
                        normal: [data[e + 6] as i8, data[e + 7] as i8, data[e + 8] as i8],
                        alpha: data[e + 9],
                    });
                }
                if transformations.is_empty() || unk_4 as usize >= transformations.len() {
                    bail!("limb {i} group {k}: transformation {unk_4} of {}", transformations.len());
                }
                limb.modifs.push(SkinLimbModif { vtx_count, unk_4, transformations });
                verts.push(group);
            }
        }
        raw.skeleton.limbs.push(limb);
        raw.dlists.push(dlist);
        raw.total_vtx.push(total);
        raw.vertices.push(verts);
    }
    Ok(raw)
}

/// An animated limb's vertex buffer as `Skin_InitAnimatedLimb` leaves it (`flag` 0, the
/// groups' texture coordinates and alpha; the normals as the bake's), each vertex at the origin
/// of its group's bone, and the bones.
pub fn limb_vertices(raw: &RawSkin, limb: usize) -> (Arc<[u8]>, Arc<[BoneId]>) {
    let n = raw.total_vtx[limb] as usize;
    let mut buf = vec![0u8; n * 16];
    let mut bones = vec![NO_BONE; n];
    for (k, group) in raw.vertices[limb].iter().enumerate() {
        let bone = raw.skeleton.modif_bone(limb, k) as BoneId;
        for v in group {
            let i = v.index as usize;
            if i >= n {
                continue;
            }
            let o = i * 16;
            buf[o + 8..o + 10].copy_from_slice(&v.s.to_be_bytes());
            buf[o + 10..o + 12].copy_from_slice(&v.t.to_be_bytes());
            buf[o + 12] = v.normal[0] as u8;
            buf[o + 13] = v.normal[1] as u8;
            buf[o + 14] = v.normal[2] as u8;
            buf[o + 15] = v.alpha;
            bones[i] = bone;
        }
    }
    (buf.into(), bones.into())
}

/// `Skin_DrawImpl`'s lists on an interpreter already set up (segments, `Gfx_SetupDL_25Opa`, the
/// prelude): every limb in index order, a normal one under its bone, an animated one with its
/// vertex buffer on segment 8.
pub fn draw(mut it: Interpreter, raw: &RawSkin) -> DrawList {
    for (i, limb) in raw.skeleton.limbs.iter().enumerate() {
        let dl = raw.dlists[i];
        if dl == 0 {
            continue;
        }
        if limb.segment_type == SKIN_LIMB_TYPE_NORMAL {
            it.set_bone(i as BoneId);
            it.run(dl);
        } else if limb.segment_type == SKIN_LIMB_TYPE_ANIMATED {
            let (buf, bones) = limb_vertices(raw, i);
            it.segments[8] = Some(Segment::Data { buf, base: 0 });
            it.vertex_bones[8] = Some(bones);
            it.set_bone(NO_BONE);
            it.run(dl);
            it.vertex_bones[8] = None;
        }
    }
    it.draw
}
