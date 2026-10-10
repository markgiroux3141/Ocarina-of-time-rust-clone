//! Curve skeletons and their animations (`curve.h`) read from an object file: the runtime's
//! `oot_game::skel_curve::CurveSkeleton` and `CurveAnimation`.

use anyhow::{Context, Result, bail};
use oot_game::skel_curve::{CurveAnimation, CurveInterpKnot, CurveLimb, CurveSkeleton};

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

/// The `CurveSkeletonHeader` at `offset` (`limbs`, `limbCount`) and its `SkelCurveLimb`s
/// (`child`, `sibling`, `dList[2]`).
pub fn parse_skeleton(data: &[u8], seg: u8, offset: usize) -> Result<CurveSkeleton> {
    let limbs_at = off(be32(data, offset)?, seg)?;
    let count = *data.get(offset + 4).context("past the file's end")? as usize;
    let mut limbs = Vec::with_capacity(count);
    for i in 0..count {
        let l = off(be32(data, limbs_at + i * 4)?, seg)?;
        let (child, sibling) = (data[l], data[l + 1]);
        limbs.push(CurveLimb { child, sibling, dlists: [be32(data, l + 4)?, be32(data, l + 8)?] });
    }
    Ok(CurveSkeleton { limbs })
}

/// The `CurveAnimationHeader` at `offset` for a skeleton of `limbs` limbs: nine knot counts per
/// limb, as many knots as their sum, a constant per zero count.
pub fn parse_animation(data: &[u8], seg: u8, offset: usize, limbs: usize) -> Result<CurveAnimation> {
    let counts_at = off(be32(data, offset)?, seg)?;
    let knots_at = off(be32(data, offset + 4)?, seg)?;
    let constants_at = off(be32(data, offset + 8)?, seg)?;
    let knot_counts = data.get(counts_at..counts_at + limbs * 9).context("knot counts past the file's end")?.to_vec();
    let knots: usize = knot_counts.iter().map(|&c| c as usize).sum();
    let constants = knot_counts.iter().filter(|&&c| c == 0).count();
    let mut interpolation_data = Vec::with_capacity(knots);
    for k in 0..knots {
        let o = knots_at + k * 0xC;
        interpolation_data.push(CurveInterpKnot {
            flags: be16(data, o)?,
            abscissa: be16(data, o + 2)? as i16,
            left_gradient: be16(data, o + 4)? as i16,
            right_gradient: be16(data, o + 6)? as i16,
            ordinate: f32::from_bits(be32(data, o + 8)?),
        });
    }
    let constant_data = (0..constants).map(|k| be16(data, constants_at + k * 2)).collect::<Result<Vec<_>>>()?;
    Ok(CurveAnimation { knot_counts, interpolation_data, constant_data, unk_0c: be16(data, offset + 0xC)? as i16, frame_count: be16(data, offset + 0xE)? as i16 })
}
