//! `Effect_Ss_Bomb2` (`ovl_Effect_Ss_Bomb2/z_eff_ss_bomb2.c`): a bomb blast with colour, eight
//! textures (`gEffBombExplosion1Tex` to `8Tex`) over its 24 frames, its colours going from white
//! and blue to yellow and red, then to grey. `EffectSsBomb2_SpawnLayered` draws it with two more
//! layers behind it, each further back, smaller and turned (`EffectSsBomb2_DrawLayered`: the
//! royal tomb's gravestone blowing up, the Gold Skulltula stone's launch, the diving game's
//! rupees); `EffectSsBomb2_SpawnFade` (unused in the game) draws the one quad
//! (`EffectSsBomb2_DrawFade`). The whole overlay is ported.

use glam::{Mat4, Vec3};

use super::{DrawCtx, EffectDraws, EffectSs, SEG_COLOR, SEG_SETUP, SEG_TEX, SsDraw, SsUpdate, billboard_mtx, colored, lerp_inv};
use crate::gbi::{G_CD_DISABLE, setup_dl};
use crate::pack::{BakeBody, BakeSegment, MeshBake};

// The overlay's regs.
pub const R_SCALE: usize = 0;
pub const R_TEX_INDEX: usize = 1;
pub const R_PRIM_COLOR_R: usize = 2;
pub const R_PRIM_COLOR_G: usize = 3;
pub const R_PRIM_COLOR_B: usize = 4;
pub const R_PRIM_COLOR_A: usize = 5;
pub const R_ENV_COLOR_R: usize = 6;
pub const R_ENV_COLOR_G: usize = 7;
pub const R_ENV_COLOR_B: usize = 8;
pub const R_SCALE_STEP: usize = 9;
pub const R_DEPTH: usize = 10;

/// `EffectSsBomb2InitParams.drawMode`: `sDrawFuncs`' index.
pub const BOMB2_DRAW_FADE: u8 = 0;
pub const BOMB2_DRAW_LAYERED: u8 = 1;

/// `EffectSsBomb2InitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct Bomb2Init {
    pub pos: Vec3,
    pub velocity: Vec3,
    pub accel: Vec3,
    pub scale: i16,
    pub scale_step: i16,
    pub draw_mode: u8,
}

/// `EffectSsBomb2_Init`: 24 frames, `gEffBombExplosion1DL`, the draw by `drawMode`, prim white,
/// env `(0, 0, 200)`.
pub fn init(this: &mut EffectSs, p: &Bomb2Init) -> bool {
    this.pos = p.pos;
    this.velocity = p.velocity;
    this.accel = p.accel;
    this.gfx = Some(("gameplay_keep", "gEffBombExplosion1DL"));
    this.life = 24;
    this.update = Some(SsUpdate::Bomb2);
    // sDrawFuncs[initParams->drawMode] (only 0 and 1 are ever passed).
    this.draw = Some(if p.draw_mode == BOMB2_DRAW_FADE { SsDraw::Bomb2Fade } else { SsDraw::Bomb2Layered });
    let r = &mut this.regs;
    r[R_SCALE] = p.scale;
    r[R_SCALE_STEP] = p.scale_step;
    r[R_PRIM_COLOR_R] = 255;
    r[R_PRIM_COLOR_G] = 255;
    r[R_PRIM_COLOR_B] = 255;
    r[R_PRIM_COLOR_A] = 255;
    r[R_ENV_COLOR_R] = 0;
    r[R_ENV_COLOR_G] = 0;
    r[R_ENV_COLOR_B] = 200;
    true
}

/// `EffectSsBomb2_Update`: the texture `(23 - life) / 3` (0 to 7), the scale up by its step, the
/// layers' depth 4 further (2 unless the step is 30); from life 22 to 14 the colours step to
/// yellow and red (`EffectSs_LerpInv` by `life - 13`), from 13 to 0 to grey (by `life + 1`).
pub fn update(this: &mut EffectSs) {
    let life = this.life;
    let r = &mut this.regs;
    r[R_TEX_INDEX] = (23 - life) / 3;
    r[R_SCALE] = r[R_SCALE].wrapping_add(r[R_SCALE_STEP]);
    // rDepth += 4.0f (an s16 through a float).
    r[R_DEPTH] = (r[R_DEPTH] as f32 + if r[R_SCALE_STEP] == 30 { 4.0 } else { 2.0 }) as i32 as i16;
    if life < 23 && life > 13 {
        let divisor = life as i32 - 13;
        r[R_PRIM_COLOR_R] = lerp_inv(r[R_PRIM_COLOR_R], 255, divisor);
        r[R_PRIM_COLOR_G] = lerp_inv(r[R_PRIM_COLOR_G], 255, divisor);
        r[R_PRIM_COLOR_B] = lerp_inv(r[R_PRIM_COLOR_B], 150, divisor);
        r[R_PRIM_COLOR_A] = lerp_inv(r[R_PRIM_COLOR_A], 255, divisor);
        r[R_ENV_COLOR_R] = lerp_inv(r[R_ENV_COLOR_R], 150, divisor);
        r[R_ENV_COLOR_G] = lerp_inv(r[R_ENV_COLOR_G], 0, divisor);
        r[R_ENV_COLOR_B] = lerp_inv(r[R_ENV_COLOR_B], 0, divisor);
    } else if life < 14 && life > -1 {
        let divisor = life as i32 + 1;
        r[R_PRIM_COLOR_R] = lerp_inv(r[R_PRIM_COLOR_R], 50, divisor);
        r[R_PRIM_COLOR_G] = lerp_inv(r[R_PRIM_COLOR_G], 50, divisor);
        r[R_PRIM_COLOR_B] = lerp_inv(r[R_PRIM_COLOR_B], 50, divisor);
        r[R_PRIM_COLOR_A] = lerp_inv(r[R_PRIM_COLOR_A], 150, divisor);
        r[R_ENV_COLOR_R] = lerp_inv(r[R_ENV_COLOR_R], 10, divisor);
        r[R_ENV_COLOR_G] = lerp_inv(r[R_ENV_COLOR_G], 10, divisor);
        r[R_ENV_COLOR_B] = lerp_inv(r[R_ENV_COLOR_B], 10, divisor);
    }
}

/// The fade draw's bake for texture `n` (1 to 8).
pub fn fade_bake_name(n: usize) -> String {
    format!("EffectSs/bomb2/fade/{n}")
}

/// The layered draw's bake for texture `n` (1 to 8).
pub fn layered_bake_name(n: usize) -> String {
    format!("EffectSs/bomb2/layered/{n}")
}

/// Per texture (`textures[]`: `gEffBombExplosion1Tex` to `8Tex` on segment 8), after
/// `Gfx_SetupDL_60NoCDXlu` (`SETUPDL_60` and `gDPSetColorDither(G_CD_DISABLE)`) with the
/// colours dynamic: the fade draw's `gEffBombExplosion1DL` (`this->gfx`), and the layered draw's
/// `gEffBombExplosion2DL` then `gEffBombExplosion3DL` (the material, then the quad). The layered
/// draw's two layers behind are `gEffBombExplosion3DL` again under the same material: drawn as
/// this bake under their own matrices.
pub fn bakes() -> Vec<MeshBake> {
    let keep = "gameplay_keep";
    let mut setup = setup_dl::setup_dl_60();
    setup.color_dither(G_CD_DISABLE);
    setup.end();
    let bake = |name: String, n: usize, lists: &[&str]| MeshBake {
        name,
        object: keep.into(),
        segments: vec![
            (SEG_TEX, BakeSegment::Texture { file: keep.into(), symbol: format!("gEffBombExplosion{n}Tex") }),
            (SEG_SETUP, BakeSegment::Commands(setup.0.clone())),
            (SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true }),
        ],
        prelude: vec![SEG_SETUP, SEG_COLOR],
        body: BakeBody::DLists(lists.iter().map(|l| (keep.to_string(), l.to_string())).collect()),
    };
    let mut v = Vec::new();
    for n in 1..=8 {
        v.push(bake(fade_bake_name(n), n, &["gEffBombExplosion1DL"]));
        v.push(bake(layered_bake_name(n), n, &["gEffBombExplosion2DL", "gEffBombExplosion3DL"]));
    }
    v
}

/// The colours both draws set: prim as the regs, env as the regs with alpha 0.
fn colors(r: &[i16; 13]) -> ([u8; 4], [u8; 4]) {
    let prim = [r[R_PRIM_COLOR_R] as u8, r[R_PRIM_COLOR_G] as u8, r[R_PRIM_COLOR_B] as u8, r[R_PRIM_COLOR_A] as u8];
    let env = [r[R_ENV_COLOR_R] as u8, r[R_ENV_COLOR_G] as u8, r[R_ENV_COLOR_B] as u8, 0];
    (prim, env)
}

/// `textures[this->rTexIndex]`'s number (1 to 8).
fn texture_number(r: &[i16; 13]) -> usize {
    (r[R_TEX_INDEX].clamp(0, 7) as usize) + 1
}

/// `EffectSsBomb2_DrawFade` (unused in the game): the billboard at its position scaled
/// `rScale × 0.01` across, `this->gfx` with the texture.
pub fn draw_fade(this: &EffectSs, ctx: &DrawCtx<'_>, out: &mut EffectDraws) {
    let r = &this.regs;
    let scale = r[R_SCALE] as f32 * 0.01;
    let m = billboard_mtx(this.pos, ctx.billboard, Vec3::new(scale, scale, 1.0));
    let (prim, env) = colors(r);
    out.xlu.push(colored(&fade_bake_name(texture_number(r)), m, SEG_COLOR, prim, env));
}

/// `EffectSsBomb2_DrawLayered`'s matrices: the billboard at `pos` scaled `scale` across
/// (`mfResult`), then the two layers behind (`i` 1 and 0), each from the last: `depth` along its
/// z, turned `life × 0.02 + 180` radians about z (`Matrix_RotateZ` takes radians: 180 is as the
/// C has it), scaled 0.925 then 0.775 (`layer2Scale` 0.15 less each time).
pub fn layered_matrices(pos: Vec3, billboard: Mat4, scale: f32, depth: f32, life: i16) -> [Mat4; 3] {
    let m0 = billboard_mtx(pos, billboard, Vec3::new(scale, scale, 1.0));
    let mut layers = [m0; 3];
    let mut m = m0;
    let mut layer2_scale = 0.925f32;
    for layer in layers.iter_mut().skip(1) {
        m = m * Mat4::from_translation(Vec3::new(0.0, 0.0, depth)) * Mat4::from_rotation_z((life as f32 * 0.02) + 180.0) * Mat4::from_scale(Vec3::splat(layer2_scale));
        *layer = m;
        layer2_scale -= 0.15;
    }
    layers
}

/// `EffectSsBomb2_DrawLayered`: the billboard at its position scaled `rScale × 0.01` across,
/// `gEffBombExplosion2DL` and `3DL` with the texture; then the two layers behind it
/// (`layered_matrices`), `gEffBombExplosion3DL` each.
pub fn draw_layered(this: &EffectSs, ctx: &DrawCtx<'_>, out: &mut EffectDraws) {
    let r = &this.regs;
    let depth = r[R_DEPTH] as f32;
    let scale = r[R_SCALE] as f32 * 0.01;
    let (prim, env) = colors(r);
    let bake = layered_bake_name(texture_number(r));
    for m in layered_matrices(this.pos, ctx.billboard, scale, depth, this.life) {
        out.xlu.push(colored(&bake, m, SEG_COLOR, prim, env));
    }
}
