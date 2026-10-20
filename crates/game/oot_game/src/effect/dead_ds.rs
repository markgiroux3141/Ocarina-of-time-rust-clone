//! `Effect_Ss_Dead_Ds` (`ovl_Effect_Ss_Dead_Ds/z_eff_ss_dead_ds.c`): a burn mark, a black round
//! shadow (`gLensFlareCircleDL`) laid flat on the surface under or beside it. Its first draw finds
//! the surface: a wall within 1.5 (`BgCheck_EntitySphVsWall1`, pushing it out), else the floor
//! below (just above it, 1.5); the mark lies on that plane from then on. The first half of its
//! life it holds; the second it shrinks by `scaleStep` and fades by `alpha / (life / 2)` a frame.
//! The Beamos' beam scorches where it meets a surface with it (`EnVm_Update`: 20 big, shrinking 1,
//! 20 frames, every frame the beam touches).

use glam::{Mat4, Vec3};

use super::{DrawCtx, EffectDraws, EffectSs, SEG_COLOR, SEG_SETUP, SsDraw, SsUpdate, colored};
use crate::gbi::{G_CD_DISABLE, ac, cc_ab, cc_c, cc_d, setup_dl};
use crate::pack::{BakeBody, BakeSegment, MeshBake};
use crate::sys_matrix::MtxF;

// The overlay's regs.
const R_SCALE: usize = 0;
const R_TIMER: usize = 1;
const R_ROLL: usize = 2;
const R_PITCH: usize = 3;
const R_YAW: usize = 4;
const R_ALPHA: usize = 5;
const R_SCALE_STEP: usize = 9;
const R_ALPHA_STEP: usize = 10;
const R_HALF_OF_LIFE: usize = 11;

/// `EffectSsDeadDsInitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct DeadDsInit {
    pub pos: Vec3,
    pub velocity: Vec3,
    pub accel: Vec3,
    pub scale: i16,
    pub scale_step: i16,
    pub alpha: i16,
    pub life: i32,
}

/// `EffectSsDeadDs_Init`: `rHalfOfLife` `life / 2`, the alpha's step `alpha / rHalfOfLife`, the
/// surface not looked for yet (`rTimer` 0).
pub fn init(this: &mut EffectSs, p: &DeadDsInit) -> bool {
    this.pos = p.pos;
    this.velocity = p.velocity;
    this.accel = p.accel;
    this.life = p.life as i16;
    let r = &mut this.regs;
    r[R_SCALE_STEP] = p.scale_step;
    r[R_HALF_OF_LIFE] = (p.life / 2) as i16;
    r[R_ALPHA_STEP] = if r[R_HALF_OF_LIFE] != 0 {
        p.alpha / r[R_HALF_OF_LIFE]
    } else {
        // The C divides by zero (IDO's `break 7`).
        log::warn!("Effect_Ss_Dead_Ds: alpha / (life / 2) with a life under 2");
        0
    };
    this.draw = Some(SsDraw::DeadDs);
    this.update = Some(SsUpdate::DeadDs);
    r[R_SCALE] = p.scale;
    r[R_ALPHA] = p.alpha;
    r[R_TIMER] = 0;
    true
}

/// `EffectSsDeadDs_Update`: in the second half of its life the scale by its step and the alpha
/// down by its step, neither below 0.
pub fn update(this: &mut EffectSs) {
    let r = &mut this.regs;
    if this.life < r[R_HALF_OF_LIFE] {
        r[R_SCALE] = r[R_SCALE].wrapping_add(r[R_SCALE_STEP]);
        if r[R_SCALE] < 0 {
            r[R_SCALE] = 0;
        }
        r[R_ALPHA] = r[R_ALPHA].wrapping_sub(r[R_ALPHA_STEP]);
        if r[R_ALPHA] < 0 {
            r[R_ALPHA] = 0;
        }
    }
}

/// The bake's name.
pub const BAKE: &str = "EffectSs/dead_ds";

/// `EffectSsDeadDs_Draw`'s mesh: `Gfx_SetupDL_60NoCDXlu`, prim `(0, 0, 0, rAlpha)` and env 0
/// (dynamic), `gDPSetCombineLERP(0, 0, 0, PRIMITIVE, TEXEL0, 0, PRIMITIVE, 0, ..)`,
/// `gLensFlareCircleDL`. The same commands as `Effect_Ss_Dead_Dd`'s, its own bake by name.
pub fn bakes() -> Vec<MeshBake> {
    let mut setup = setup_dl::setup_dl_60();
    setup.color_dither(G_CD_DISABLE);
    let c = [cc_ab::ZERO, cc_ab::ZERO, cc_c::ZERO, cc_d::PRIMITIVE, ac::TEXEL0, ac::ZERO, ac::PRIMITIVE, ac::ZERO];
    setup.combine_lerp(c, c);
    setup.end();
    vec![MeshBake {
        name: BAKE.into(),
        object: "gameplay_keep".into(),
        segments: vec![(SEG_SETUP, BakeSegment::Commands(setup.0)), (SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true })],
        prelude: vec![SEG_SETUP, SEG_COLOR],
        body: BakeBody::DLists(vec![("gameplay_keep".into(), "gLensFlareCircleDL".into())]),
    }]
}

/// `EffectSsDeadDs_Draw`. The first time (`rTimer` 0): a wall within 1.5 of it (from where its
/// velocity brought it: `BgCheck_EntitySphVsWall1`, its position pushed out) gives the plane;
/// else the floor under it (`BgCheck_EntityRaycastDown1` from 1 up), the mark 1.5 above; with
/// neither, no tilt. The plane's angles (`Matrix_MtxFToZYXRotS`) kept in `rRoll`, `rPitch`, `rYaw`,
/// its height the mark's y. Then at its position turned by those, laid flat (`Matrix_RotateX(1.57)`,
/// radians), scaled `rScale × 0.01`; black at `rAlpha`.
pub fn draw(this: &mut EffectSs, ctx: &DrawCtx<'_>, out: &mut EffectDraws) {
    let mut pos = this.pos;
    if this.regs[R_TIMER] == 0 {
        let prev_pos = pos - this.velocity;
        let (hit, result, wall) = ctx.col.check_wall(eng_collision::bgcheck::IGNORE_ENTITY, pos, prev_pos, 1.5, 1.0, 0);
        // BgCheck_EntitySphVsWall1's posResult is this->pos.
        this.pos = result;
        let mf = match (hit, wall) {
            (true, Some(w)) => MtxF::from_mat4(crate::shadow::func_80038a28(ctx.col.poly(w).normal, this.pos)),
            _ => {
                pos.y += 1.0;
                let (y_intersect, ground) = ctx.col.entity_raycast_down(pos);
                match ground {
                    Some(g) => MtxF::from_mat4(crate::shadow::func_80038a28(ctx.col.poly(g).normal, Vec3::new(this.pos.x, y_intersect + 1.5, this.pos.z))),
                    // Matrix_Translate(.., MTXMODE_NEW), Matrix_Get.
                    None => MtxF::set_translate(this.pos.x, this.pos.y, this.pos.z),
                }
            }
        };
        let rpy = mf.to_zyx_rot_s(false);
        this.regs[R_ROLL] = rpy[0];
        this.regs[R_PITCH] = rpy[1];
        this.regs[R_YAW] = rpy[2];
        this.pos.y = mf.yw;
        this.regs[R_TIMER] += 1;
    }
    let prim = [0, 0, 0, this.regs[R_ALPHA] as u8];
    out.xlu.push(colored(BAKE, mark_matrix(this), SEG_COLOR, prim, [0, 0, 0, 0]));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Matrix_MtxFToZYXRotS` gives back the angles `Matrix_RotateZYX` was given (away from the
    /// poles), to within `Math_SinS`'s table (its steps of 0x40 hold the error under 16).
    #[test]
    fn zyx_angles_round_trip() {
        for (x, y, z) in [(0x1000i16, 0x0800i16, -0x2000i16), (-0x3000, 0x2000, 0x0100), (0, 0, 0)] {
            let mut m = MtxF::IDENTITY;
            m.rotate_zyx(x, y, z);
            let r = m.to_zyx_rot_s(false);
            for (a, b) in r.iter().zip([x, y, z]) {
                assert!((*a as i32 - b as i32).abs() <= 16, "{r:?} against {:?}", [x, y, z]);
            }
        }
    }
}

/// The draw's matrix: `Matrix_Translate(pos, NEW)`, `Matrix_RotateZYX(rRoll, rPitch, rYaw)`,
/// `Matrix_RotateX(1.57)`, `Matrix_Scale(rScale × 0.01)`.
pub fn mark_matrix(this: &EffectSs) -> Mat4 {
    let r = &this.regs;
    let scale = r[R_SCALE] as f32 * 0.01;
    let mut m = MtxF::set_translate(this.pos.x, this.pos.y, this.pos.z);
    m.rotate_zyx(r[R_ROLL], r[R_PITCH], r[R_YAW]);
    m.rotate_x(1.57);
    m.scale(scale, scale, scale);
    m.to_mat4()
}
