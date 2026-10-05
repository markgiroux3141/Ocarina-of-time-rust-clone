//! `Effect_Ss_Dead_Db` (`ovl_Effect_Ss_Dead_Db/z_eff_ss_dead_db.c`): the flame and sound when
//! some enemies die (the Deku Scrubs, the Peahats, the Poes): ten textures over its life, its
//! colours fading to black, with `NA_SE_EN_EXTINCT` at its second texture.

use glam::{Mat4, Vec3};

use super::{EffectDraws, EffectSs, SEG_COLOR, SEG_SETUP, SEG_TEX, SsDraw, SsUpdate, colored};
use crate::audio::sfx::{NA_SE_EN_EXTINCT, SfxPos};
use crate::gbi::{G_CD_DISABLE, setup_dl};
use crate::pack::{BakeBody, BakeSegment, MeshBake};
use crate::play::PlayState;

// The overlay's regs.
const R_SCALE: usize = 0;
const R_TEXT_IDX: usize = 1;
const R_PRIM_COLOR_R: usize = 2;
const R_PRIM_COLOR_G: usize = 3;
const R_PRIM_COLOR_B: usize = 4;
const R_PRIM_COLOR_A: usize = 5;
const R_ENV_COLOR_R: usize = 6;
const R_ENV_COLOR_G: usize = 7;
const R_ENV_COLOR_B: usize = 8;
const R_SCALE_STEP: usize = 9;
const R_PLAY_SFX: usize = 10;
const R_REG11: usize = 11;

/// `EffectSsDeadDbInitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct DeadDbInit {
    pub pos: Vec3,
    pub velocity: Vec3,
    pub accel: Vec3,
    pub scale: i16,
    pub scale_step: i16,
    pub prim_color: [u8; 4],
    pub env_color: [u8; 4],
    pub unused: i16,
    pub unk_34: i32,
    pub play_sfx: i16,
}

/// `EffectSsDeadDb_Init`: its life `unk_34`; flag 4, so its sound at `vec` stops with it.
pub fn init(this: &mut EffectSs, p: &DeadDbInit) -> bool {
    this.pos = p.pos;
    this.velocity = p.velocity;
    this.accel = p.accel;
    this.gfx = Some(("gameplay_keep", "gEffEnemyDeathFlameDL"));
    this.life = p.unk_34 as i16;
    this.flags = 4;
    let r = &mut this.regs;
    r[R_SCALE_STEP] = p.scale_step;
    r[R_REG11] = p.unk_34 as i16;
    this.draw = Some(SsDraw::DeadDb);
    this.update = Some(SsUpdate::DeadDb);
    r[R_SCALE] = p.scale;
    r[R_TEXT_IDX] = 0;
    r[R_PLAY_SFX] = p.play_sfx;
    r[R_PRIM_COLOR_R] = p.prim_color[0] as i16;
    r[R_PRIM_COLOR_G] = p.prim_color[1] as i16;
    r[R_PRIM_COLOR_B] = p.prim_color[2] as i16;
    r[R_PRIM_COLOR_A] = p.prim_color[3] as i16;
    r[R_ENV_COLOR_R] = p.env_color[0] as i16;
    r[R_ENV_COLOR_G] = p.env_color[1] as i16;
    r[R_ENV_COLOR_B] = p.env_color[2] as i16;
    true
}

/// `EffectSsDeadDb_Update`: the texture `(rReg11 - life) × 9 / rReg11`, the scale growing, every
/// colour 10 darker; on the second texture, the sound at its position through the view
/// (`SkinMatrix_Vec3fMtxFMultXYZW` into `vec`).
pub fn update(play: &mut PlayState, index: usize, this: &mut EffectSs) {
    let r = &mut this.regs;
    r[R_TEXT_IDX] = (((r[R_REG11] - this.life) * 9) as f32 / r[R_REG11] as f32) as i16;
    r[R_SCALE] = r[R_SCALE].wrapping_add(r[R_SCALE_STEP]);
    for k in [R_PRIM_COLOR_R, R_PRIM_COLOR_G, R_PRIM_COLOR_B, R_ENV_COLOR_R, R_ENV_COLOR_G, R_ENV_COLOR_B] {
        r[k] -= 10;
        if r[k] < 0 {
            r[k] = 0;
        }
    }
    if r[R_PLAY_SFX] != 0 && r[R_TEXT_IDX] == 1 {
        let c = play.view_proj * this.pos.extend(1.0);
        this.vec = c.truncate();
        // SFX_PLAY_AT_POS(&this->vec, NA_SE_EN_EXTINCT): the position is read from the slot
        // when the sound plays, so the slot is written back first.
        play.effect_ss.table[index].vec = this.vec;
        play.audio.play_sfx_at_pos(SfxPos::EffectSsVec(index as u8), NA_SE_EN_EXTINCT);
    }
}

fn bake_name(n: usize) -> String {
    format!("EffectSs/dead_db/{n}")
}

/// One bake per texture (`gEffEnemyDeathFlame1Tex` to `10`): `Gfx_SetupDL_60NoCDXlu` (with
/// `gDPSetColorDither(G_CD_DISABLE)`), the colours dynamic, `gEffEnemyDeathFlameDL` (which
/// loads the billboard from segment 1: the draw multiplies it in).
pub fn bakes() -> Vec<MeshBake> {
    let mut setup = setup_dl::setup_dl_60();
    setup.color_dither(G_CD_DISABLE);
    setup.end();
    (1..=10)
        .map(|n| MeshBake {
            name: bake_name(n),
            object: "gameplay_keep".into(),
            segments: vec![
                (SEG_TEX, BakeSegment::Texture { file: "gameplay_keep".into(), symbol: format!("gEffEnemyDeathFlame{n}Tex") }),
                (SEG_SETUP, BakeSegment::Commands(setup.0.clone())),
                (SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true }),
                // gSPMatrix(0x01000000): the billboard, multiplied in by the draw.
                (0x01, BakeSegment::Bytes(super::identity_mtx())),
            ],
            prelude: vec![SEG_SETUP, SEG_COLOR],
            body: BakeBody::DLists(vec![("gameplay_keep".into(), "gEffEnemyDeathFlameDL".into())]),
        })
        .collect()
}

/// `EffectSsDeadDb_Draw`: at its position scaled `rScale × 0.01`, then the list's billboard;
/// env alpha 0.
pub fn draw(this: &EffectSs, billboard: Mat4, out: &mut EffectDraws) {
    let r = &this.regs;
    let scale = r[R_SCALE] as f32 * 0.01;
    let m = Mat4::from_translation(this.pos) * Mat4::from_scale(Vec3::splat(scale)) * billboard;
    let tex = (r[R_TEXT_IDX] as usize).min(9) + 1;
    let prim = [r[R_PRIM_COLOR_R] as u8, r[R_PRIM_COLOR_G] as u8, r[R_PRIM_COLOR_B] as u8, r[R_PRIM_COLOR_A] as u8];
    let env = [r[R_ENV_COLOR_R] as u8, r[R_ENV_COLOR_G] as u8, r[R_ENV_COLOR_B] as u8, 0];
    out.xlu.push(colored(&bake_name(tex), m, SEG_COLOR, prim, env));
}
