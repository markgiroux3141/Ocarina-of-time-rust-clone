//! Link's foot IK: `func_8008F87C` in `z_player_lib.c`, which `Player_OverrideLimbDrawGameplayCommon`
//! runs for each thigh while drawing. It finds the floor under the foot; if the animated ankle
//! would be below it, it solves the two-bone leg (thigh, shin) so the ankle rests on the floor,
//! and writes the new thigh, shin and foot Z rotations **into `skelAnime.jointTable`** (so they
//! also feed the next frame's morphs, as in the game).
//!
//! The constants (`D_80126038`..`D_80126070`) are `z_player_lib.c`'s, and the leg hierarchy
//! (parents and joint positions) is Link's skeleton; both come from the asset pack.
//!
//! Not modelled: the `unk_6C2` root tilt (diving), `EffectSsGFire` footprints on fire floors.

use eng_anim::skeleton::local_transform;
use eng_collision::bgcheck::{BGCHECK_Y_MIN, CollisionContext};
use eng_math::binang_to_rad;
use glam::{Mat4, Vec3};

use crate::camera::f_atan2f;

/// The per-age constants of `func_8008F87C` (index 0 adult, 1 child, as `gSaveContext.linkAge`
/// indexes them: `LINK_AGE_ADULT` = 0).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FootIkData {
    /// `D_80126038`: the shin's offset from the thigh (model units).
    pub shin_offset: [Vec3; 2],
    /// `D_80126050`: the ankle's offset along the shin's x axis.
    pub foot_x: [f32; 2],
    /// `D_80126058`: `SQ(thigh length)` in world units.
    pub thigh_len_sq: [f32; 2],
    /// `D_80126060`: `SQ(shin) - SQ(thigh)`-style constant of the law-of-cosines solve.
    pub len_diff: [f32; 2],
    /// `D_80126068`: how far above the floor the ankle is kept.
    pub floor_offset: [f32; 2],
    /// `D_80126070`: the point below the ankle used for the floor probe.
    pub footprint: Vec3,
}

/// Link's limb hierarchy for one age.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Rig {
    pub parents: Vec<Option<u8>>,
    pub joint_pos: Vec<[i16; 3]>,
}

/// `RAD_TO_BINANG`: `(s16)((radians) * (0x8000 / M_PI))`.
fn rad_to_binang(r: f32) -> i16 {
    (r * (32768.0 / std::f32::consts::PI)) as i32 as i16
}

/// The model-space matrix of `limb` (its parent chain applied), as `SkelAnime_DrawFlexLod`
/// builds it under `Actor_Draw`'s matrix `actor`. `joints[0]` is the root translation (already
/// adjusted like Player's root override), `joints[l + 1]` limb `l`'s rotation.
pub fn limb_matrix(rig: &Rig, actor: Mat4, root: Vec3, joints: &[[i16; 3]], limb: usize) -> Mat4 {
    let mut chain = vec![limb];
    while let Some(p) = rig.parents[*chain.last().unwrap()] {
        chain.push(p as usize);
    }
    let mut m = actor;
    for &l in chain.iter().rev() {
        let pos = if l == 0 { root } else { Vec3::from(rig.joint_pos[l].map(|v| v as f32)) };
        m *= local_transform(pos, joints.get(l + 1).copied().unwrap_or([0; 3]));
    }
    m
}

/// Where the legs are, for inspection: (hip, ankle, floor target) of one leg after IK.
#[derive(Debug, Clone, Copy)]
pub struct LegResult {
    pub hip: Vec3,
    pub ankle: Vec3,
    pub floor: f32,
    pub adjusted: bool,
}

/// `func_8008F87C` for one leg. `thigh`, `shin`, `foot` are 0-based limb indices.
#[allow(clippy::too_many_arguments)]
pub fn solve_leg(
    d: &FootIkData,
    rig: &Rig,
    col: &CollisionContext,
    adult: bool,
    actor: Mat4,
    root: Vec3,
    joints: &mut [[i16; 3]],
    unk_6c4: f32,
    thigh: usize,
    shin: usize,
    foot: usize,
) -> LegResult {
    let age = if adult { 0 } else { 1 };
    let sp7c = d.thigh_len_sq[age];
    let sp78 = d.len_diff[age];
    let sp74 = d.floor_offset[age] - unk_6c4;
    let m_thigh = limb_matrix(rig, actor, root, joints, thigh);
    let hip = m_thigh.transform_point3(Vec3::ZERO);
    let m_foot = m_thigh * local_transform(d.shin_offset[age], joints[shin + 1]) * Mat4::from_translation(Vec3::new(d.foot_x[age], 0.0, 0.0));
    let ankle = m_foot.transform_point3(Vec3::ZERO);
    let mut footprint = m_foot.transform_point3(d.footprint);
    footprint.y += 15.0;
    // BgCheck_EntityRaycastDown4 (COLPOLY_IGNORE_ENTITY, walls simple + floors, ground only).
    let (floor_y, _) = col.entity_raycast_down(footprint);
    let sp80 = floor_y + sp74;
    let mut out = LegResult { hip, ankle, floor: sp80, adjusted: false };
    if floor_y == BGCHECK_Y_MIN || !(ankle.y < sp80) {
        return out;
    }
    let (sp70, mut sp6c, sp68) = (ankle.x - hip.x, ankle.y - hip.y, ankle.z - hip.z);
    let mut sp64 = (sp70 * sp70 + sp6c * sp6c + sp68 * sp68).sqrt();
    let mut sp60 = (sp64 * sp64 + sp78) / (2.0 * sp64);
    let mut sp58 = if sp7c < sp60 * sp60 { 0.0 } else { (sp7c - sp60 * sp60).sqrt() };
    let sp54 = f_atan2f(sp58, sp60);

    sp6c = sp80 - hip.y;
    sp64 = (sp70 * sp70 + sp6c * sp6c + sp68 * sp68).sqrt();
    sp60 = (sp64 * sp64 + sp78) / (2.0 * sp64);
    let sp5c = sp64 - sp60;
    sp58 = if sp7c < sp60 * sp60 { 0.0 } else { (sp7c - sp60 * sp60).sqrt() };
    let sp50 = f_atan2f(sp58, sp60);

    use std::f32::consts::{FRAC_PI_2, PI};
    let mut temp1 = rad_to_binang(PI - (f_atan2f(sp5c, sp58) + (FRAC_PI_2 - sp50)));
    let s = joints[shin + 1];
    temp1 = temp1.wrapping_sub(s[2]);
    if (((s[0] as i32).abs() + (s[1] as i32).abs()) as i16) < 0 {
        temp1 = temp1.wrapping_add(i16::MIN);
    }
    let temp2 = rad_to_binang(sp50 - sp54);
    joints[thigh + 1][2] = joints[thigh + 1][2].wrapping_sub(temp2);
    joints[shin + 1][2] = joints[shin + 1][2].wrapping_add(temp1);
    joints[foot + 1][2] = joints[foot + 1][2].wrapping_add(temp2).wrapping_sub(temp1);
    // Where the ankle ends up after the change.
    let m_thigh = limb_matrix(rig, actor, root, joints, thigh);
    let m_foot = m_thigh * local_transform(d.shin_offset[age], joints[shin + 1]) * Mat4::from_translation(Vec3::new(d.foot_x[age], 0.0, 0.0));
    out.ankle = m_foot.transform_point3(Vec3::ZERO);
    out.adjusted = true;
    out
}

/// `Actor_Draw`'s matrix: `Matrix_SetTranslateRotateYXZ(pos.y + yOffset * scale.y, shape.rot)`
/// then `Matrix_Scale(0.01)`.
pub fn actor_matrix(pos: Vec3, y_offset: f32, shape_rot: [i16; 3]) -> Mat4 {
    Mat4::from_translation(pos + Vec3::Y * (y_offset * 0.01))
        * Mat4::from_rotation_y(binang_to_rad(shape_rot[1]))
        * Mat4::from_rotation_x(binang_to_rad(shape_rot[0]))
        * Mat4::from_rotation_z(binang_to_rad(shape_rot[2]))
        * Mat4::from_scale(Vec3::splat(0.01))
}
