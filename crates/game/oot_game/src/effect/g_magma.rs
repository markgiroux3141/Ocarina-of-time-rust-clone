//! `Effect_Ss_G_Magma` (`ovl_Effect_Ss_G_Magma/z_eff_ss_g_magma.c`): a bubble bursting on lava
//! (King Dodongo's floor): a quad facing the camera through `gEffMagmaBubble1Tex` to `8Tex`, half a
//! texture a frame, 16 frames, yellow to red.

use glam::{Mat4, Vec3};

use super::{EffectDraws, EffectSs, SEG_COLOR, SEG_SETUP, SEG_TEX, SsDraw, SsSpawn, SsUpdate, billboard_mtx, colored};
use crate::gbi::setup_dl;
use crate::pack::{BakeBody, BakeSegment, MeshBake};

// The common regs (`effect.h`'s `rg*`).
const RG_TEX_IDX: usize = 0;
const RG_SCALE: usize = 1;
const RG_TEX_IDX_STEP: usize = 2;
const RG_PRIM_COLOR_R: usize = 3;
const RG_PRIM_COLOR_G: usize = 4;
const RG_PRIM_COLOR_B: usize = 5;
const RG_PRIM_COLOR_A: usize = 6;
const RG_ENV_COLOR_R: usize = 7;
const RG_ENV_COLOR_G: usize = 8;
const RG_ENV_COLOR_B: usize = 9;
const RG_ENV_COLOR_A: usize = 10;

/// `EffectSsGMagmaInitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct GMagmaInit {
    pub pos: Vec3,
}

/// `EffectSsGMagma_Init`: still at `pos`, 16 frames, `rgScale` 200 to 299, the texture index up 50
/// a frame (a texture every two), prim yellow, env red with alpha 0.
pub fn init(s: &mut SsSpawn<'_>, this: &mut EffectSs, p: &GMagmaInit) -> bool {
    this.velocity = Vec3::ZERO;
    this.accel = Vec3::ZERO;
    this.pos = p.pos;
    this.draw = Some(SsDraw::GMagma);
    this.update = Some(SsUpdate::GMagma);
    this.gfx = Some(("gameplay_keep", "gEffMagmaBubbleDL"));
    this.life = 16;
    let r = &mut this.regs;
    r[RG_SCALE] = (s.rand.zero_one() * 100.0) as i16 + 200;
    r[RG_TEX_IDX] = 0;
    r[RG_TEX_IDX_STEP] = 50;
    r[RG_PRIM_COLOR_R] = 255;
    r[RG_PRIM_COLOR_G] = 255;
    r[RG_PRIM_COLOR_B] = 0;
    r[RG_PRIM_COLOR_A] = 255;
    r[RG_ENV_COLOR_R] = 255;
    r[RG_ENV_COLOR_G] = 0;
    r[RG_ENV_COLOR_B] = 0;
    r[RG_ENV_COLOR_A] = 0;
    true
}

/// `EffectSsGMagma_Update`.
pub fn update(this: &mut EffectSs) {
    this.regs[RG_TEX_IDX] = this.regs[RG_TEX_IDX].wrapping_add(this.regs[RG_TEX_IDX_STEP]);
}

fn bake_name(n: usize) -> String {
    format!("EffectSs/g_magma/{n}")
}

/// One bake per texture (`sTextures`: `gEffMagmaBubble1Tex` to `8Tex` on segment 8):
/// `EffectSs_DrawGEffect`'s `Gfx_SetupDL_61Xlu`, the colours dynamic, `gEffMagmaBubbleDL`.
pub fn bakes() -> Vec<MeshBake> {
    let mut setup = setup_dl::setup_dl_61();
    setup.end();
    (1..=8)
        .map(|n| MeshBake {
            name: bake_name(n),
            object: "gameplay_keep".into(),
            segments: vec![
                (SEG_TEX, BakeSegment::Texture { file: "gameplay_keep".into(), symbol: format!("gEffMagmaBubble{n}Tex") }),
                (SEG_SETUP, BakeSegment::Commands(setup.0.clone())),
                (SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true }),
            ],
            prelude: vec![SEG_SETUP, SEG_COLOR],
            body: BakeBody::DLists(vec![("gameplay_keep".into(), "gEffMagmaBubbleDL".into())]),
        })
        .collect()
}

/// `EffectSsGMagma_Draw`: texture `rgTexIdx / 100` (at most 7) through `EffectSs_DrawGEffect`: at
/// its position facing the camera, `rgScale × 0.0025` every way.
pub fn draw(this: &EffectSs, billboard: Mat4, out: &mut EffectDraws) {
    let r = &this.regs;
    let tex = (r[RG_TEX_IDX] / 100).min(7) as usize;
    let scale = r[RG_SCALE] as f32 * 0.0025;
    let m = billboard_mtx(this.pos, billboard, Vec3::splat(scale));
    let prim = [r[RG_PRIM_COLOR_R] as u8, r[RG_PRIM_COLOR_G] as u8, r[RG_PRIM_COLOR_B] as u8, r[RG_PRIM_COLOR_A] as u8];
    let env = [r[RG_ENV_COLOR_R] as u8, r[RG_ENV_COLOR_G] as u8, r[RG_ENV_COLOR_B] as u8, r[RG_ENV_COLOR_A] as u8];
    out.xlu.push(colored(&bake_name(tex + 1), m, SEG_COLOR, prim, env));
}
