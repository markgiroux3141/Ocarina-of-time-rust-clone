//! `Effect_Ss_HitMark` (`ovl_Effect_Ss_HitMark/z_eff_ss_hitmark.c`): the flash where a hit
//! lands, eight textures in eight frames (sixteen for dust, two frames each), fading from its
//! first colours to its second.

use glam::Vec3;

use super::{DrawCtx, EffectDraws, EffectSs, SEG_COLOR, SEG_SETUP, SEG_TEX, SsDraw, SsUpdate, billboard_mtx, colored, lerp_inv};
use crate::gbi::setup_dl;
use crate::pack::{BakeBody, BakeSegment, MeshBake};

// The overlay's regs.
const R_TEX_INDEX: usize = 0;
const R_TYPE: usize = 1;
const R_PRIM_COLOR_R: usize = 2;
const R_PRIM_COLOR_G: usize = 3;
const R_PRIM_COLOR_B: usize = 4;
const R_ENV_COLOR_R: usize = 5;
const R_ENV_COLOR_G: usize = 6;
const R_ENV_COLOR_B: usize = 7;
const R_SCALE: usize = 8;

/// `EFFECT_HITMARK_*` (`z_eff_ss_hitmark.h`).
pub const EFFECT_HITMARK_WHITE: i32 = 0;
pub const EFFECT_HITMARK_DUST: i32 = 1;
pub const EFFECT_HITMARK_RED: i32 = 2;
pub const EFFECT_HITMARK_METAL: i32 = 3;

/// `sColors`: per type, the prim and env colours to start with, then to fade to.
const S_COLORS: [[u8; 3]; 16] = [
    [255, 255, 255],
    [255, 255, 0],
    [255, 255, 255],
    [255, 0, 0],
    [255, 200, 100],
    [200, 150, 0],
    [150, 100, 0],
    [100, 50, 0],
    [255, 255, 255],
    [255, 0, 0],
    [255, 255, 0],
    [255, 0, 0],
    [255, 255, 255],
    [0, 255, 200],
    [255, 255, 255],
    [150, 0, 255],
];

/// `sTextures`: eight per type; the metal type's are the white's again.
fn texture_number(i: usize) -> usize {
    (i % 24) + 1
}

/// `EffectSsHitMarkInitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct HitMarkInit {
    pub ty: i32,
    pub scale: i16,
    pub pos: Vec3,
}

/// `EffectSsHitMark_Init`.
pub fn init(this: &mut EffectSs, p: &HitMarkInit) -> bool {
    this.pos = p.pos;
    this.gfx = Some(("gameplay_keep", "gEffHitMarkDL"));
    this.life = if p.ty == EFFECT_HITMARK_DUST { 16 } else { 8 };
    this.draw = Some(SsDraw::HitMark);
    this.update = Some(SsUpdate::HitMark);
    let c = (p.ty * 4) as usize;
    let r = &mut this.regs;
    r[R_TEX_INDEX] = 0;
    r[R_TYPE] = p.ty as i16;
    r[R_PRIM_COLOR_R] = S_COLORS[c][0] as i16;
    r[R_PRIM_COLOR_G] = S_COLORS[c][1] as i16;
    r[R_PRIM_COLOR_B] = S_COLORS[c][2] as i16;
    r[R_ENV_COLOR_R] = S_COLORS[c + 1][0] as i16;
    r[R_ENV_COLOR_G] = S_COLORS[c + 1][1] as i16;
    r[R_ENV_COLOR_B] = S_COLORS[c + 1][2] as i16;
    r[R_SCALE] = p.scale;
    true
}

/// `EffectSsHitMark_Update`: the texture by the life left; past the first, the colours step
/// towards the type's second pair (`EffectSs_LerpInv` by the frames left).
pub fn update(this: &mut EffectSs) {
    let r = &mut this.regs;
    r[R_TEX_INDEX] = if r[R_TYPE] as i32 == EFFECT_HITMARK_DUST { (15 - this.life) / 2 } else { 7 - this.life };
    if r[R_TEX_INDEX] != 0 {
        let c = (r[R_TYPE] as i32 * 4 + 2) as usize;
        let w = this.life as i32 + 1;
        r[R_PRIM_COLOR_R] = lerp_inv(r[R_PRIM_COLOR_R], S_COLORS[c][0] as i16, w);
        r[R_PRIM_COLOR_G] = lerp_inv(r[R_PRIM_COLOR_G], S_COLORS[c][1] as i16, w);
        r[R_PRIM_COLOR_B] = lerp_inv(r[R_PRIM_COLOR_B], S_COLORS[c][2] as i16, w);
        r[R_ENV_COLOR_R] = lerp_inv(r[R_ENV_COLOR_R], S_COLORS[c + 1][0] as i16, w);
        r[R_ENV_COLOR_G] = lerp_inv(r[R_ENV_COLOR_G], S_COLORS[c + 1][1] as i16, w);
        r[R_ENV_COLOR_B] = lerp_inv(r[R_ENV_COLOR_B], S_COLORS[c + 1][2] as i16, w);
    }
}

fn bake_name(n: usize) -> String {
    format!("EffectSs/hitmark/{n}")
}

/// One bake per texture (`gEffHitMark1Tex` to `gEffHitMark24Tex`): `Gfx_SetupDL_61Xlu`, the
/// colours dynamic, `gEffHitMarkDL`.
pub fn bakes() -> Vec<MeshBake> {
    let mut setup = setup_dl::setup_dl_61();
    setup.end();
    (1..=24)
        .map(|n| MeshBake {
            name: bake_name(n),
            object: "gameplay_keep".into(),
            segments: vec![
                (SEG_TEX, BakeSegment::Texture { file: "gameplay_keep".into(), symbol: format!("gEffHitMark{n}Tex") }),
                (SEG_SETUP, BakeSegment::Commands(setup.0.clone())),
                (SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true }),
            ],
            prelude: vec![SEG_SETUP, SEG_COLOR],
            body: BakeBody::DLists(vec![("gameplay_keep".into(), "gEffHitMarkDL".into())]),
        })
        .collect()
}

/// `EffectSsHitMark_Draw`: the billboard scaled `rScale / 100` across, prim opaque, env alpha
/// 0, `sTextures[rType * 8 + rTexIndex]`.
pub fn draw(this: &EffectSs, ctx: &DrawCtx<'_>, out: &mut EffectDraws) {
    let r = &this.regs;
    let scale = r[R_SCALE] as f32 / 100.0;
    let m = billboard_mtx(this.pos, ctx.billboard, Vec3::new(scale, scale, 1.0));
    let i = (r[R_TYPE] as i32 * 8 + r[R_TEX_INDEX] as i32).clamp(0, 31) as usize;
    let prim = [r[R_PRIM_COLOR_R] as u8, r[R_PRIM_COLOR_G] as u8, r[R_PRIM_COLOR_B] as u8, 255];
    let env = [r[R_ENV_COLOR_R] as u8, r[R_ENV_COLOR_G] as u8, r[R_ENV_COLOR_B] as u8, 0];
    out.xlu.push(colored(&bake_name(texture_number(i)), m, SEG_COLOR, prim, env));
}
