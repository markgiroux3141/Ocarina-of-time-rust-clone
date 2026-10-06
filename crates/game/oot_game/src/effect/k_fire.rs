//! `Effect_Ss_K_Fire` (`ovl_Effect_Ss_K_Fire/z_eff_ss_k_fire.c`): a flame facing the camera
//! that grows to its size and fades out (the Gohma Larvae's death, the bosses' fires): white with
//! a cyan edge, or (`type` 100 and up) yellow with a red one; `type` 3 keeps growing taller.
//!
//! It draws `gEffFire1DL` as `Effect_Ss_En_Fire` does (the same setup, colours and scroll), so it
//! shares that overlay's bake (`en_fire::bakes`, `EffectSs/fire/1`).

use glam::{Mat4, Vec3};

use super::en_fire::bake_name as fire_bake_name;
use super::{DrawCtx, EffectDraws, EffectSs, SEG_COLOR, SEG_TEX, SsDraw, SsSpawn, SsUpdate};
use crate::pack::keys;
use crate::scene_table::gfx_two_tex_scroll;

// The overlay's regs.
const R_ALPHA: usize = 0;
const R_SCROLL: usize = 2;
const R_TYPE: usize = 3;
const R_Y_SCALE: usize = 4;
const R_XZ_SCALE: usize = 5;
const R_SCALE_MAX: usize = 6;

/// `EffectSsKFireInitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct KFireInit {
    pub pos: Vec3,
    pub velocity: Vec3,
    pub accel: Vec3,
    pub scale_max: i16,
    pub ty: u8,
}

/// `EffectSsKFire_Init`: 100 frames, opaque, scrolling up by 21 to 25 (`Rand_ZeroFloat(5) -
/// 0x19`, negative) a frame.
pub fn init(s: &mut SsSpawn<'_>, this: &mut EffectSs, p: &KFireInit) -> bool {
    this.pos = p.pos;
    this.velocity = p.velocity;
    this.accel = p.accel;
    this.life = 100;
    this.regs[R_SCALE_MAX] = p.scale_max;
    this.regs[R_ALPHA] = 255;
    this.regs[R_SCROLL] = (s.rand.zero_float(5.0) as i16).wrapping_sub(0x19);
    this.regs[R_TYPE] = p.ty as i16;
    this.draw = Some(SsDraw::KFire);
    this.update = Some(SsUpdate::KFire);
    true
}

/// `EffectSsKFire_Update`: growing by 4 a frame to `rScaleMax` (type 3's height left as it is);
/// then fading by 10 a frame, ending at 0. Type 3 grows 1 taller every frame.
pub fn update(this: &mut EffectSs) {
    let r = &mut this.regs;
    if r[R_XZ_SCALE] < r[R_SCALE_MAX] {
        r[R_XZ_SCALE] += 4;
        r[R_Y_SCALE] += 4;
        if r[R_XZ_SCALE] > r[R_SCALE_MAX] {
            r[R_XZ_SCALE] = r[R_SCALE_MAX];
            if r[R_TYPE] != 3 {
                r[R_Y_SCALE] = r[R_SCALE_MAX];
            }
        }
    } else if r[R_ALPHA] > 0 {
        r[R_ALPHA] -= 10;
        if r[R_ALPHA] <= 0 {
            r[R_ALPHA] = 0;
            this.life = 0;
        }
    }
    if r[R_TYPE] == 3 {
        r[R_Y_SCALE] += 1;
    }
}

/// `EffectSsKFire_Draw`: at its position scaled `rXZScale / 10000` across and `rYScale / 10000`
/// up, turned to the camera (`Matrix_ReplaceRotation(&play->billboardMtxF)`), and the odd slots
/// turned round (`Matrix_RotateY(M_PI)`); `Gfx_SetupDL_25Xlu`, the scroll
/// (`Gfx_TwoTexScroll(.., 0, 0, 0, 32, 64, 1, 0, frames × rScroll, 32, 128)`), the colours
/// (prim with LOD fraction 0x80), `gEffFire1DL`.
pub fn draw(this: &EffectSs, index: usize, ctx: &DrawCtx<'_>, out: &mut EffectDraws) {
    let r = &this.regs;
    let xz_scale = r[R_XZ_SCALE] as f32 / 10000.0;
    let y_scale = r[R_Y_SCALE] as f32 / 10000.0;
    let mut m = Mat4::from_translation(this.pos) * Mat4::from_scale(Vec3::new(xz_scale, y_scale, xz_scale));
    m = crate::draw::replace_rotation(m, ctx.billboard);
    if index & 1 != 0 {
        m *= Mat4::from_rotation_y(std::f32::consts::PI);
    }
    let alpha = r[R_ALPHA] as u8;
    let (prim, env) = if r[R_TYPE] >= 100 { ([255, 255, 0, alpha], [255, 10, 0, 0]) } else { ([255, 255, 255, alpha], [0, 255, 255, 0]) };
    // play->state.frames * rScroll, as the u32 Gfx_TwoTexScroll takes.
    let y2 = (ctx.state_frames as i32).wrapping_mul(r[R_SCROLL] as i32) as u32;
    let mut sv = eng_gfx::SegmentValues::default();
    sv.read(SEG_TEX, &gfx_two_tex_scroll(0, 0, 0, 0x20, 0x40, 1, 0, y2, 0x20, 0x80));
    sv.prim[SEG_COLOR as usize] = Some(prim);
    sv.env[SEG_COLOR as usize] = Some(env);
    out.xlu.push(eng_gfx::DrawCmd {
        mesh: eng_gfx::MeshKey::named(keys::bake(&fire_bake_name(1))),
        transform: m,
        bones: Vec::new(),
        params: eng_gfx::DrawParams { segments: Some(sv), ..Default::default() },
    });
}
