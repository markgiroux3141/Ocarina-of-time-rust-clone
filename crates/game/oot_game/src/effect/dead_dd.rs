//! `Effect_Ss_Dead_Dd` (`ovl_Effect_Ss_Dead_Dd/z_eff_ss_dead_dd.c`): a soft round flash facing the
//! camera (`gLensFlareCircleDL`), growing or shrinking by `scaleStep` a frame while its alpha
//! fades in or out. The Beamos' eye flashes with it as it fires (`EnVm_Wait`: blue in white,
//! from 150 big shrinking 25 a frame, its alpha up 16 a frame, 20 frames). Its type 1
//! (`EffectSsDeadDd_SpawnRandYellow`, yellow at a random spot near `pos`) is spawned by nothing
//! in the game.

use glam::{Mat4, Vec3};

use super::{EffectDraws, EffectSs, SEG_COLOR, SEG_SETUP, SsDraw, SsSpawn, SsUpdate, colored};
use crate::gbi::{G_CD_DISABLE, ac, cc_ab, cc_c, cc_d, setup_dl};
use crate::pack::{BakeBody, BakeSegment, MeshBake};

// The overlay's regs.
const R_SCALE: usize = 0;
const R_PRIM_COLOR_R: usize = 2;
const R_PRIM_COLOR_G: usize = 3;
const R_PRIM_COLOR_B: usize = 4;
const R_ALPHA: usize = 5;
const R_ENV_COLOR_R: usize = 6;
const R_ENV_COLOR_G: usize = 7;
const R_ENV_COLOR_B: usize = 8;
const R_SCALE_STEP: usize = 9;
const R_ALPHA_STEP: usize = 10;
/// `rAlphaMode`: 0, the alpha falls by `rAlphaStep` a frame; else it rises.
const R_ALPHA_MODE: usize = 11;

/// `EffectSsDeadDdInitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct DeadDdInit {
    pub pos: Vec3,
    pub velocity: Vec3,
    pub accel: Vec3,
    pub scale: i16,
    pub scale_step: i16,
    pub prim_color: [u8; 3],
    pub alpha: u8,
    pub env_color: [u8; 3],
    pub alpha_step: i16,
    pub life: i32,
    pub rand_pos_scale: f32,
    pub rand_iter: i32,
    /// `type`: 0 the colours given, 1 yellow at a random spot.
    pub ty: u8,
}

/// `EffectSsDeadDd_Init`.
/// - type 0: as given; a non-zero `alphaStep` is the step and the alpha rises
///   (`rAlphaMode`), 0 the alpha falls `alpha / life` a frame;
/// - type 1: life `life`, the alpha falling from 155 by `155 / life`, prim (255, 255, 155), env
///   (250, 180, 0); `randIter` times a position up to `randPosScale / 2` from `pos` each way and a
///   velocity and acceleration up to 1 each way (the last time's kept);
/// - any other: "mode_sw is strange", not spawned.
pub fn init(s: &mut SsSpawn<'_>, this: &mut EffectSs, p: &DeadDdInit) -> bool {
    match p.ty {
        0 => {
            this.pos = p.pos;
            this.velocity = p.velocity;
            this.accel = p.accel;
            this.life = p.life as i16;
            let r = &mut this.regs;
            r[R_SCALE_STEP] = p.scale_step;
            r[R_ALPHA_MODE] = p.alpha_step;
            r[R_ALPHA_STEP] = if p.alpha_step != 0 {
                p.alpha_step
            } else if p.life != 0 {
                (p.alpha as i32 / p.life) as i16
            } else {
                // The C divides by zero (IDO's `break 7`).
                log::warn!("Effect_Ss_Dead_Dd: alpha / life with life 0");
                0
            };
            this.draw = Some(SsDraw::DeadDd);
            this.update = Some(SsUpdate::DeadDd);
            r[R_SCALE] = p.scale;
            r[R_PRIM_COLOR_R] = p.prim_color[0] as i16;
            r[R_PRIM_COLOR_G] = p.prim_color[1] as i16;
            r[R_PRIM_COLOR_B] = p.prim_color[2] as i16;
            r[R_ALPHA] = p.alpha as i16;
            r[R_ENV_COLOR_R] = p.env_color[0] as i16;
            r[R_ENV_COLOR_G] = p.env_color[1] as i16;
            r[R_ENV_COLOR_B] = p.env_color[2] as i16;
            true
        }
        1 => {
            this.life = p.life as i16;
            let r = &mut this.regs;
            r[R_SCALE_STEP] = p.scale_step;
            r[R_ALPHA_MODE] = 0;
            r[R_ALPHA_STEP] = if p.life != 0 {
                (155 / p.life) as i16
            } else {
                log::warn!("Effect_Ss_Dead_Dd: 155 / life with life 0");
                0
            };
            r[R_SCALE] = p.scale;
            r[R_PRIM_COLOR_R] = 255;
            r[R_PRIM_COLOR_G] = 255;
            r[R_PRIM_COLOR_B] = 155;
            r[R_ALPHA] = 155;
            r[R_ENV_COLOR_R] = 250;
            r[R_ENV_COLOR_G] = 180;
            r[R_ENV_COLOR_B] = 0;
            this.draw = Some(SsDraw::DeadDd);
            this.update = Some(SsUpdate::DeadDd);
            let mut i = p.rand_iter;
            while i > 0 {
                this.pos.x = ((s.rand.zero_one() - 0.5) * p.rand_pos_scale) + p.pos.x;
                this.pos.y = ((s.rand.zero_one() - 0.5) * p.rand_pos_scale) + p.pos.y;
                this.pos.z = ((s.rand.zero_one() - 0.5) * p.rand_pos_scale) + p.pos.z;
                this.velocity.x = (s.rand.zero_one() - 0.5) * 2.0;
                this.accel.x = this.velocity.x;
                this.velocity.y = (s.rand.zero_one() - 0.5) * 2.0;
                this.accel.y = this.velocity.y;
                this.velocity.z = (s.rand.zero_one() - 0.5) * 2.0;
                this.accel.z = this.velocity.z;
                i -= 1;
            }
            true
        }
        _ => {
            log::debug!("Effect_SS_Dd_disp_mode():mode_sw is strange.");
            false
        }
    }
}

/// `EffectSsDeadDd_Update`: the scale by its step (not below 0); the alpha up by its step to 255
/// at most (`rAlphaMode` set), else down to 0.
pub fn update(this: &mut EffectSs) {
    let r = &mut this.regs;
    r[R_SCALE] = r[R_SCALE].wrapping_add(r[R_SCALE_STEP]);
    if r[R_SCALE] < 0 {
        r[R_SCALE] = 0;
    }
    if r[R_ALPHA_MODE] != 0 {
        r[R_ALPHA] = r[R_ALPHA].wrapping_add(r[R_ALPHA_STEP]);
        if r[R_ALPHA] > 255 {
            r[R_ALPHA] = 255;
        }
    } else if r[R_ALPHA] < r[R_ALPHA_STEP] {
        r[R_ALPHA] = 0;
    } else {
        r[R_ALPHA] -= r[R_ALPHA_STEP];
    }
}

/// The bake's name.
pub const BAKE: &str = "EffectSs/dead_dd";

/// `EffectSsDeadDd_Draw`'s mesh: `Gfx_SetupDL_60NoCDXlu` (with `gDPSetColorDither(G_CD_DISABLE)`),
/// the colours dynamic, `gDPSetCombineLERP(0, 0, 0, PRIMITIVE, TEXEL0, 0, PRIMITIVE, 0, ..)` in
/// both cycles, `gLensFlareCircleDL` (no combine of its own).
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

/// `EffectSsDeadDd_Draw`: at its position scaled `rScale × 0.01` (`SkinMatrix_SetTranslate` ×
/// `SkinMatrix_SetScale`), the billboard multiplied in after (`gSPMatrix(&D_01000000, G_MTX_MUL)`);
/// prim and env their colours, both at `rAlpha`.
pub fn draw(this: &EffectSs, billboard: Mat4, out: &mut EffectDraws) {
    let r = &this.regs;
    let scale = r[R_SCALE] as f32 * 0.01;
    let m = Mat4::from_translation(this.pos) * Mat4::from_scale(Vec3::splat(scale)) * billboard;
    let a = r[R_ALPHA] as u8;
    let prim = [r[R_PRIM_COLOR_R] as u8, r[R_PRIM_COLOR_G] as u8, r[R_PRIM_COLOR_B] as u8, a];
    let env = [r[R_ENV_COLOR_R] as u8, r[R_ENV_COLOR_G] as u8, r[R_ENV_COLOR_B] as u8, a];
    out.xlu.push(colored(BAKE, m, SEG_COLOR, prim, env));
}
