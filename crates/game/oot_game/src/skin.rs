//! The skin skeletons (`z_skin.c`, `z_skin_awb.c`, `z_skin_matrix.c`): the horses' models.
//!
//! A skin skeleton's limbs are of two kinds. A *normal* limb (`SKIN_LIMB_TYPE_NORMAL`) draws its
//! display list under its limb matrix, as any skeleton's limb. An *animated* limb
//! (`SKIN_LIMB_TYPE_ANIMATED`) draws a list whose vertices the game rewrites every frame
//! (`Skin_ApplyLimbModifications`): its vertices are in groups (`SkinLimbModif`), and every
//! vertex of a group goes to the same point, a blend of offsets from limb matrices
//! (`SkinTransformation`s, weighted by `scale / 100`), with its normal turned by one limb's
//! rotation (`limbTransformations[unk_4]`).
//!
//! The pack holds the skeleton as a `SkinSkeleton` (`keys::skin`) and its mesh as a bake
//! (`BakeBody::Skin`) in which every group is a bone of its own, its vertices at the bone's
//! origin: normal limb `i` is bone `i`, and the groups follow, limb by limb
//! (`SkinSkeleton::modif_bone`). Each frame `skin_bones` gives a group's bone as the translation
//! to its point (truncated to the vertex's s16, as `Skin_UpdateVertices` stores it) times the
//! normal's limb rotation, and the draw's transform is the skin's matrix (`skin->mtx`).
//!
//! The matrix functions are `z_skin_matrix.c`'s, on `MtxF` with the C's operation order.

use eng_math::{cos_s, sin_s};
use glam::{Mat4, Vec3, Vec4};

use crate::sys_matrix::MtxF;

/// `SKIN_LIMB_TYPE_ANIMATED`, `SKIN_LIMB_TYPE_NORMAL` (`z64skin.h`).
pub const SKIN_LIMB_TYPE_ANIMATED: i32 = 4;
pub const SKIN_LIMB_TYPE_NORMAL: i32 = 11;
/// `LIMB_DONE`.
pub const LIMB_DONE: u8 = 0xFF;

/// `SkinTransformation`: an offset from a limb's matrix and its weight (`scale` percent).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SkinTransformation {
    pub limb_index: u8,
    pub x: i16,
    pub y: i16,
    pub z: i16,
    pub scale: u8,
}

/// `SkinLimbModif`: a group of vertices moved to one point.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SkinLimbModif {
    pub vtx_count: u16,
    /// `unk_4`: the transformation whose limb turns the normals.
    pub unk_4: u16,
    pub transformations: Vec<SkinTransformation>,
}

/// `SkinLimb`, with its `SkinAnimatedLimbData`'s groups for an animated limb.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SkinLimb {
    pub joint_pos: [i16; 3],
    pub child: u8,
    pub sibling: u8,
    pub segment_type: i32,
    /// `segment != NULL`.
    pub has_segment: bool,
    /// An animated limb's `limbModifications`.
    pub modifs: Vec<SkinLimbModif>,
}

/// A skin skeleton (`SkeletonHeader` with `SkinLimb`s).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SkinSkeleton {
    pub limbs: Vec<SkinLimb>,
}

impl SkinSkeleton {
    /// The bake's bone for group `k` of limb `limb`: after the limbs, the groups in order.
    pub fn modif_bone(&self, limb: usize, k: usize) -> usize {
        self.limbs.len() + self.limbs[..limb].iter().map(|l| l.modifs.len()).sum::<usize>() + k
    }

    /// Bones in the bake: the limbs, then every group.
    pub fn bone_count(&self) -> usize {
        self.limbs.len() + self.limbs.iter().map(|l| l.modifs.len()).sum::<usize>()
    }
}

/// `SkinMatrix_SetTranslate`.
pub fn set_translate(x: f32, y: f32, z: f32) -> MtxF {
    MtxF { xw: x, yw: y, zw: z, ..MtxF::IDENTITY }
}

/// `SkinMatrix_SetScale`.
pub fn set_scale(x: f32, y: f32, z: f32) -> MtxF {
    MtxF { xx: x, yy: y, zz: z, ..MtxF::IDENTITY }
}

/// `SkinMatrix_SetRotateZYX`.
pub fn set_rotate_zyx(x: i16, y: i16, z: i16) -> MtxF {
    let mut mf = MtxF::IDENTITY;
    let sin_z = sin_s(z);
    let cos_z = cos_s(z);
    mf.yy = cos_z;
    mf.xy = -sin_z;
    mf.wx = 0.0;
    mf.wy = 0.0;
    mf.wz = 0.0;
    mf.xw = 0.0;
    mf.yw = 0.0;
    mf.zw = 0.0;
    mf.ww = 1.0;
    if y != 0 {
        let sin = sin_s(y);
        let cos = cos_s(y);
        mf.xx = cos_z * cos;
        mf.xz = cos_z * sin;
        mf.yx = sin_z * cos;
        mf.yz = sin_z * sin;
        mf.zx = -sin;
        mf.zz = cos;
    } else {
        mf.xx = cos_z;
        mf.yx = sin_z;
        mf.zx = 0.0;
        mf.xz = 0.0;
        mf.yz = 0.0;
        mf.zz = 1.0;
    }
    if x != 0 {
        let sin = sin_s(x);
        let cos = cos_s(x);
        let (xy, xz) = (mf.xy, mf.xz);
        mf.xy = (xy * cos) + (xz * sin);
        mf.xz = (xz * cos) - (xy * sin);
        let (yz, yy) = (mf.yz, mf.yy);
        mf.yy = (yy * cos) + (yz * sin);
        mf.yz = (yz * cos) - (yy * sin);
        mf.zy = mf.zz * sin;
        mf.zz *= cos;
    } else {
        mf.zy = 0.0;
    }
    mf
}

/// `SkinMatrix_SetRotateYXZ`.
pub fn set_rotate_yxz(x: i16, y: i16, z: i16) -> MtxF {
    let mut mf = MtxF::IDENTITY;
    let sin_y = sin_s(y);
    let cos_y = cos_s(y);
    mf.xx = cos_y;
    mf.zx = -sin_y;
    mf.wz = 0.0;
    mf.wy = 0.0;
    mf.wx = 0.0;
    mf.zw = 0.0;
    mf.yw = 0.0;
    mf.xw = 0.0;
    mf.ww = 1.0;
    if x != 0 {
        let sin = sin_s(x);
        let cos = cos_s(x);
        mf.zz = cos_y * cos;
        mf.zy = cos_y * sin;
        mf.xz = sin_y * cos;
        mf.xy = sin_y * sin;
        mf.yz = -sin;
        mf.yy = cos;
    } else {
        mf.zz = cos_y;
        mf.xz = sin_y;
        mf.xy = 0.0;
        mf.zy = 0.0;
        mf.yz = 0.0;
        mf.yy = 1.0;
    }
    if z != 0 {
        let sin = sin_s(z);
        let cos = cos_s(z);
        let (xx, xy) = (mf.xx, mf.xy);
        mf.xx = (xx * cos) + (xy * sin);
        mf.xy = xy * cos - (xx * sin);
        let (zy, zx) = (mf.zy, mf.zx);
        mf.zx = (zx * cos) + (zy * sin);
        mf.zy = (zy * cos) - (zx * sin);
        mf.yx = mf.yy * sin;
        mf.yy *= cos;
    } else {
        mf.yx = 0.0;
    }
    mf
}

/// `SkinMatrix_MtxFMtxFMult(a, b, dest)`: `a × b`, each element summed in the C's order.
pub fn mult(a: &MtxF, b: &MtxF) -> MtxF {
    let row = |rx: f32, ry: f32, rz: f32, rw: f32| {
        (
            (rx * b.xx) + (ry * b.yx) + (rz * b.zx) + (rw * b.wx),
            (rx * b.xy) + (ry * b.yy) + (rz * b.zy) + (rw * b.wy),
            (rx * b.xz) + (ry * b.yz) + (rz * b.zz) + (rw * b.wz),
            (rx * b.xw) + (ry * b.yw) + (rz * b.zw) + (rw * b.ww),
        )
    };
    let (xx, xy, xz, xw) = row(a.xx, a.xy, a.xz, a.xw);
    let (yx, yy, yz, yw) = row(a.yx, a.yy, a.yz, a.yw);
    let (zx, zy, zz, zw) = row(a.zx, a.zy, a.zz, a.zw);
    let (wx, wy, wz, ww) = row(a.wx, a.wy, a.wz, a.ww);
    MtxF { xx, yx, zx, wx, xy, yy, zy, wy, xz, yz, zz, wz, xw, yw, zw, ww }
}

/// `SkinMatrix_Vec3fMtxFMultXYZ`.
pub fn mult_xyz(mf: &MtxF, src: Vec3) -> Vec3 {
    Vec3::new(
        mf.xw + ((src.x * mf.xx) + (src.y * mf.xy) + (src.z * mf.xz)),
        mf.yw + ((src.x * mf.yx) + (src.y * mf.yy) + (src.z * mf.yz)),
        mf.zw + ((src.x * mf.zx) + (src.y * mf.zy) + (src.z * mf.zz)),
    )
}

/// `SkinMatrix_SetTranslateRotateZYX`.
pub fn set_translate_rotate_zyx(rx: i16, ry: i16, rz: i16, tx: f32, ty: f32, tz: f32) -> MtxF {
    mult(&set_translate(tx, ty, tz), &set_rotate_zyx(rx, ry, rz))
}

/// `SkinMatrix_SetTranslateRotateYXZScale`.
#[allow(clippy::too_many_arguments)]
pub fn set_translate_rotate_yxz_scale(sx: f32, sy: f32, sz: f32, rx: i16, ry: i16, rz: i16, tx: f32, ty: f32, tz: f32) -> MtxF {
    let t = mult(&set_translate(tx, ty, tz), &set_rotate_yxz(rx, ry, rz));
    mult(&t, &set_scale(sx, sy, sz))
}

/// An `MtxF` as a column-vector `Mat4`.
pub fn to_mat4(m: &MtxF) -> Mat4 {
    Mat4::from_cols(Vec4::new(m.xx, m.yx, m.zx, m.wx), Vec4::new(m.xy, m.yy, m.zy, m.wy), Vec4::new(m.xz, m.yz, m.zz, m.wz), Vec4::new(m.xw, m.yw, m.zw, m.ww))
}

/// What `Skin_ApplyAnimTransformations` reads of the actor: its scale, shape rotation, place and
/// `shape.yOffset`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkinPlace {
    pub scale: Vec3,
    pub rot: [i16; 3],
    pub pos: Vec3,
    pub y_offset: f32,
}

/// `Skin_ApplyAnimTransformations` (with `func_800A698C`): the limbs' matrices in model space
/// (`gSkinLimbMatrices`) and the skin's matrix (`skin->mtx`), from the joint table (entry 0 the
/// root's translation, used when `set_translation`).
pub fn apply_anim_transformations(skel: &SkinSkeleton, joints: &[[i16; 3]], place: &SkinPlace, set_translation: bool) -> (Vec<MtxF>, MtxF) {
    let n = skel.limbs.len();
    let mut mats = vec![MtxF::IDENTITY; n];
    let j = |i: usize| joints.get(i).copied().unwrap_or([0; 3]);
    let root = j(1);
    mats[0] = if set_translation {
        let t = j(0);
        set_translate_rotate_zyx(root[0], root[1], root[2], t[0] as f32, t[1] as f32, t[2] as f32)
    } else {
        set_translate_rotate_zyx(root[0], root[1], root[2], 0.0, 0.0, 0.0)
    };
    for (i, m) in mats.iter_mut().enumerate().skip(1) {
        let p = skel.limbs[i].joint_pos;
        let r = j(i + 1);
        *m = set_translate_rotate_zyx(r[0], r[1], r[2], p[0] as f32, p[1] as f32, p[2] as f32);
    }
    let skin_mtx =
        set_translate_rotate_yxz_scale(place.scale.x, place.scale.y, place.scale.z, place.rot[0], place.rot[1], place.rot[2], place.pos.x, place.pos.y + (place.y_offset * place.scale.y), place.pos.z);
    concat(skel, &mut mats, LIMB_DONE, 0);
    (mats, skin_mtx)
}

/// `func_800A698C`: each limb's matrix times its parent's (already concatenated), then its
/// child, then its sibling. Walked with a stack rather than recursively: the recursive form
/// (as the C writes it) comes out wrong from rustc 1.95 and 1.98 at `opt-level` 1 and up (a
/// grandchild took its grandparent's matrix; correct unoptimised and with 1.92). The order of
/// the products is the C's: a limb is done after its parent, before its children.
fn concat(skel: &SkinSkeleton, mats: &mut [MtxF], parent: u8, limb: u8) {
    let mut stack = vec![(parent, limb)];
    while let Some((parent, limb)) = stack.pop() {
        let l = limb as usize;
        let p = if parent == LIMB_DONE { MtxF::IDENTITY } else { mats[parent as usize] };
        mats[l] = mult(&p, &mats[l]);
        let (child, sibling) = (skel.limbs[l].child, skel.limbs[l].sibling);
        if sibling != LIMB_DONE {
            stack.push((parent, sibling));
        }
        if child != LIMB_DONE {
            stack.push((limb, child));
        }
    }
}

/// The group's point (`Skin_ApplyLimbModifications`' `vtxPoint`): one transformation's offset
/// through its limb, or (with `arg3`) `unk_4`'s, else the weighted sum.
pub fn modif_point(m: &SkinLimbModif, mats: &[MtxF], arg3: bool) -> Vec3 {
    let t = &m.transformations;
    let at = |e: &SkinTransformation| mult_xyz(&mats[e.limb_index as usize], Vec3::new(e.x as f32, e.y as f32, e.z as f32));
    if t.len() == 1 {
        at(&t[0])
    } else if arg3 {
        at(&t[m.unk_4 as usize])
    } else {
        let mut sum = Vec3::ZERO;
        for e in t {
            let scale = e.scale as f32 * 0.01;
            let p = at(e);
            sum.x += p.x * scale;
            sum.y += p.y * scale;
            sum.z += p.z * scale;
        }
        sum
    }
}

/// The bake's bones for this frame: normal limb `i`'s matrix; each group's translation to its
/// point, as `Skin_UpdateVertices` stores it in the vertex (`ob`, s16), times the normal limb's
/// rotation (its matrix without the translation).
pub fn skin_bones(skel: &SkinSkeleton, mats: &[MtxF], arg3: bool) -> Vec<Mat4> {
    let mut bones: Vec<Mat4> = mats.iter().map(to_mat4).collect();
    bones.reserve(skel.bone_count() - mats.len());
    for l in &skel.limbs {
        for m in &l.modifs {
            let p = modif_point(m, mats, arg3);
            let ob = Vec3::new(p.x as i32 as i16 as f32, p.y as i32 as i16 as f32, p.z as i32 as i16 as f32);
            let nl = m.transformations.get(m.unk_4 as usize).map(|t| t.limb_index as usize).unwrap_or(0);
            let r = MtxF { xw: 0.0, yw: 0.0, zw: 0.0, ..mats[nl] };
            bones.push(Mat4::from_translation(ob) * to_mat4(&r));
        }
    }
    bones
}

/// `Skin_DrawImpl` (`func_800A6330` without an override): the skin's transform and bones for
/// joint table `joints` at `place`.
pub fn skin_draw(skel: &SkinSkeleton, joints: &[[i16; 3]], place: &SkinPlace, set_translation: bool) -> (Mat4, Vec<Mat4>) {
    let (mats, skin_mtx) = apply_anim_transformations(skel, joints, place, set_translation);
    (to_mat4(&skin_mtx), skin_bones(skel, &mats, false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotate_zyx_is_z_then_y_then_x() {
        let (x, y, z) = (0x1234i16, -0x2345i16, 0x3456i16);
        let m = to_mat4(&set_rotate_zyx(x, y, z));
        let r = |a: i16| eng_math::binang_to_rad(a);
        let g = Mat4::from_rotation_z(r(z)) * Mat4::from_rotation_y(r(y)) * Mat4::from_rotation_x(r(x));
        assert!(m.abs_diff_eq(g, 1e-3), "{m:?} {g:?}");
    }

    #[test]
    fn rotate_yxz_is_y_then_x_then_z() {
        let (x, y, z) = (0x1234i16, -0x2345i16, 0x3456i16);
        let m = to_mat4(&set_rotate_yxz(x, y, z));
        let r = |a: i16| eng_math::binang_to_rad(a);
        let g = Mat4::from_rotation_y(r(y)) * Mat4::from_rotation_x(r(x)) * Mat4::from_rotation_z(r(z));
        assert!(m.abs_diff_eq(g, 1e-3), "{m:?} {g:?}");
    }

    #[test]
    fn children_follow_their_parents() {
        let limb = |pos: [i16; 3], child: u8, sibling: u8| SkinLimb { joint_pos: pos, child, sibling, segment_type: 0, has_segment: false, modifs: Vec::new() };
        // 0 -> 1 -> 2, and 3 a sibling of 1.
        let skel = SkinSkeleton { limbs: vec![limb([0, 0, 0], 1, LIMB_DONE), limb([0, 100, 0], 2, 3), limb([0, 0, 0], LIMB_DONE, LIMB_DONE), limb([50, 0, 0], LIMB_DONE, LIMB_DONE)] };
        let joints = [[0, 10, 0], [0, 0, 0], [0x4000, 0, 0], [0, 0, 0], [0, 0, 0]];
        let place = SkinPlace { scale: Vec3::ONE, rot: [0; 3], pos: Vec3::ZERO, y_offset: 0.0 };
        let (mats, _) = apply_anim_transformations(&skel, &joints, &place, true);
        let o = |i: usize| mult_xyz(&mats[i], Vec3::ZERO);
        assert_eq!(o(1), Vec3::new(0.0, 110.0, 0.0));
        assert_eq!(o(2), o(1));
        assert_eq!(o(3), Vec3::new(50.0, 10.0, 0.0));
    }

    #[test]
    fn mult_matches_mat4() {
        let a = set_translate_rotate_zyx(0x1000, 0x2000, -0x800, 10.0, -20.0, 30.0);
        let b = set_translate_rotate_yxz_scale(0.5, 2.0, 1.5, -0x700, 0x300, 0x1100, 1.0, 2.0, 3.0);
        let p = Vec3::new(7.0, -3.0, 11.0);
        let want = to_mat4(&a) * to_mat4(&b);
        assert!(to_mat4(&mult(&a, &b)).abs_diff_eq(want, 1e-3));
        assert!((mult_xyz(&mult(&a, &b), p) - want.transform_point3(p)).length() < 1e-3);
    }
}
