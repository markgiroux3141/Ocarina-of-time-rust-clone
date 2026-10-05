//! `Effect_Ss_Dust` (`ovl_Effect_Ss_Dust/z_eff_ss_dust.c`): a puff of dust, a billboard
//! cycling through eight textures (`gDust1Tex` to `gDust8Tex`) and growing by `scaleStep`,
//! drifting at random.

use glam::{Mat4, Vec3};

use super::{DrawCtx, EffectDraws, EffectSs, SEG_COLOR, SEG_SETUP, SEG_TEX, SsDraw, SsSpawn, SsUpdate, billboard_mtx, colored};
use crate::pack::{BakeBody, BakeSegment, MeshBake};
use crate::play::PlayState;

// The overlay's regs.
const R_PRIM_COLOR_R: usize = 0;
const R_PRIM_COLOR_G: usize = 1;
const R_PRIM_COLOR_B: usize = 2;
const R_PRIM_COLOR_A: usize = 3;
const R_ENV_COLOR_R: usize = 4;
const R_ENV_COLOR_G: usize = 5;
const R_ENV_COLOR_B: usize = 6;
const R_ENV_COLOR_A: usize = 7;
/// `rTexIndex`: also picks the fire update's colours.
pub const R_TEX_INDEX: usize = 8;
pub const R_SCALE: usize = 9;
const R_SCALE_STEP: usize = 10;
pub const R_DRAW_FLAGS: usize = 11;
const R_LIFESPAN: usize = 12;

/// `EffectSsDustInitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct DustInit {
    pub pos: Vec3,
    pub velocity: Vec3,
    pub accel: Vec3,
    pub prim_color: [u8; 4],
    pub env_color: [u8; 4],
    pub scale: i16,
    pub scale_step: i16,
    pub life: i16,
    pub draw_flags: u16,
    pub update_mode: u8,
}

/// `EffectSsDust_Init`: draw flag 4 shifts every colour by one random offset in -10..10.
pub fn init(s: &mut SsSpawn<'_>, this: &mut EffectSs, p: &DustInit) -> bool {
    this.pos = p.pos;
    this.velocity = p.velocity;
    this.accel = p.accel;
    this.life = p.life;
    // sUpdateFuncs[initParams->updateMode].
    this.update = Some(if p.update_mode == 0 { SsUpdate::Dust } else { SsUpdate::DustFire });
    this.draw = Some(SsDraw::Dust);
    let r = &mut this.regs;
    if p.draw_flags & 4 != 0 {
        let off = (s.rand.zero_one() * 20.0 - 10.0) as i32;
        let c = |v: u8| (v as i32 + off) as i16;
        r[R_PRIM_COLOR_R] = c(p.prim_color[0]);
        r[R_PRIM_COLOR_G] = c(p.prim_color[1]);
        r[R_PRIM_COLOR_B] = c(p.prim_color[2]);
        r[R_ENV_COLOR_R] = c(p.env_color[0]);
        r[R_ENV_COLOR_G] = c(p.env_color[1]);
        r[R_ENV_COLOR_B] = c(p.env_color[2]);
    } else {
        r[R_PRIM_COLOR_R] = p.prim_color[0] as i16;
        r[R_PRIM_COLOR_G] = p.prim_color[1] as i16;
        r[R_PRIM_COLOR_B] = p.prim_color[2] as i16;
        r[R_ENV_COLOR_R] = p.env_color[0] as i16;
        r[R_ENV_COLOR_G] = p.env_color[1] as i16;
        r[R_ENV_COLOR_B] = p.env_color[2] as i16;
    }
    r[R_PRIM_COLOR_A] = p.prim_color[3] as i16;
    r[R_ENV_COLOR_A] = p.env_color[3] as i16;
    r[R_TEX_INDEX] = 0;
    r[R_SCALE] = p.scale;
    r[R_SCALE_STEP] = p.scale_step;
    r[R_LIFESPAN] = p.life;
    r[R_DRAW_FLAGS] = p.draw_flags as i16;
    true
}

/// `EffectSsDust_Update`: a random sideways drift; the texture follows the life for the first
/// eight frames (spread over them for a life under 5), then stays on the last; the scale grows.
pub fn update(play: &mut PlayState, this: &mut EffectSs) {
    this.accel.x = (play.rand.zero_one() * 0.4) - 0.2;
    this.accel.z = (play.rand.zero_one() * 0.4) - 0.2;
    let r = &mut this.regs;
    let lifespan = r[R_LIFESPAN];
    if this.life <= lifespan && this.life >= lifespan - 7 {
        if lifespan >= 5 {
            r[R_TEX_INDEX] = lifespan - this.life;
        } else {
            r[R_TEX_INDEX] = (lifespan - this.life) * (8 / lifespan);
        }
    } else {
        r[R_TEX_INDEX] = 7;
    }
    r[R_SCALE] = r[R_SCALE].wrapping_add(r[R_SCALE_STEP]);
}

/// `EffectSsDust_UpdateFire` (no spawner uses update mode 1): the texture steps every frame,
/// the colours going from orange to black over the first four.
pub fn update_fire(play: &mut PlayState, this: &mut EffectSs) {
    this.accel.x = (play.rand.zero_one() * 0.4) - 0.2;
    this.accel.z = (play.rand.zero_one() * 0.4) - 0.2;
    let r = &mut this.regs;
    let set = |r: &mut [i16; 13], p: [i16; 3], e: [i16; 3]| {
        r[R_PRIM_COLOR_R] = p[0];
        r[R_PRIM_COLOR_G] = p[1];
        r[R_PRIM_COLOR_B] = p[2];
        r[R_ENV_COLOR_R] = e[0];
        r[R_ENV_COLOR_G] = e[1];
        r[R_ENV_COLOR_B] = e[2];
    };
    match r[R_TEX_INDEX] {
        0 => set(r, [255, 150, 0], [150, 50, 0]),
        1 => set(r, [200, 50, 0], [100, 0, 0]),
        2 => set(r, [50, 0, 0], [0, 0, 0]),
        3 => set(r, [50, 0, 0], [0, 0, 0]),
        _ => {}
    }
    if r[R_TEX_INDEX] < 7 {
        r[R_TEX_INDEX] += 1;
    }
    r[R_SCALE] = r[R_SCALE].wrapping_add(r[R_SCALE_STEP]);
}

/// `dustTextures`.
const DUST_TEXTURES: [&str; 8] = ["gDust1Tex", "gDust2Tex", "gDust3Tex", "gDust4Tex", "gDust5Tex", "gDust6Tex", "gDust7Tex", "gDust8Tex"];

/// The draw flags' three modes (`EffectSsDust_Draw`): 1 fogged and lit with the combiner's
/// second cycle taking the shade, else 2 unfogged with no blend, else unlit.
fn mode_of(draw_flags: i16) -> usize {
    if draw_flags & 1 != 0 {
        0
    } else if draw_flags & 2 != 0 {
        1
    } else {
        2
    }
}

fn bake_name(mode: usize, tex: usize) -> String {
    format!("EffectSs/dust/{mode}/{tex}")
}

/// The 24 bakes: a texture (`gSPSegment(0x08, dustTextures[rTexIndex])`) per mode, after
/// `Gfx_SetupDL(SETUPDL_0)` and the mode's commands, the colours dynamic.
pub fn bakes() -> Vec<MeshBake> {
    use crate::gbi::*;
    let mut v = Vec::new();
    for mode in 0..3 {
        let mut d = setup_dl::setup_dl_0();
        d.pipe_sync();
        match mode {
            0 => {
                // gDPSetCombineLERP(PRIMITIVE, ENVIRONMENT, TEXEL0, ENVIRONMENT, PRIMITIVE, 0,
                // TEXEL0, 0, COMBINED, 0, SHADE, 0, 0, 0, 0, COMBINED).
                let c1 = [cc_ab::COMBINED, cc_ab::ZERO, cc_c::SHADE, cc_d::ZERO, ac::ZERO, ac::ZERO, ac::ZERO, ac::COMBINED];
                d.combine_lerp(setup_dl::PRIM_ENV_TEXEL0, c1);
                d.render_mode(G_RM_FOG_SHADE_A, G_RM_ZB_CLD_SURF2);
                d.set_geometry_mode(G_FOG | G_LIGHTING);
            }
            1 => {
                d.render_mode(G_RM_PASS, G_RM_ZB_CLD_SURF2);
                d.clear_geometry_mode(G_FOG | G_LIGHTING);
            }
            _ => d.clear_geometry_mode(G_LIGHTING),
        }
        d.pipe_sync();
        d.end();
        for (tex, sym) in DUST_TEXTURES.iter().enumerate() {
            v.push(MeshBake {
                name: bake_name(mode, tex),
                object: "gameplay_keep".into(),
                segments: vec![
                    (SEG_TEX, BakeSegment::Texture { file: "gameplay_keep".into(), symbol: (*sym).into() }),
                    (SEG_SETUP, BakeSegment::Commands(d.0.clone())),
                    (SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true }),
                ],
                prelude: vec![SEG_SETUP, SEG_COLOR],
                body: BakeBody::DLists(vec![("gameplay_keep".into(), "gEffDustDL".into())]),
            });
        }
    }
    v
}

/// `EffectSsDust_Draw`: the billboard scaled `rScale × 0.0025` across (1 deep), prim at full
/// alpha.
pub fn draw(this: &EffectSs, ctx: &DrawCtx<'_>, out: &mut EffectDraws) {
    let r = &this.regs;
    let scale = r[R_SCALE] as f32 * 0.0025;
    let m: Mat4 = billboard_mtx(this.pos, ctx.billboard, Vec3::new(scale, scale, 1.0));
    // @bug (game): a lifespan of 4 reaches index 8 on its last frame, past `dustTextures` (the
    // overlay's next data); clamped to the last texture here.
    let tex = (r[R_TEX_INDEX] as usize).min(7);
    let prim = [r[R_PRIM_COLOR_R] as u8, r[R_PRIM_COLOR_G] as u8, r[R_PRIM_COLOR_B] as u8, 255];
    let env = [r[R_ENV_COLOR_R] as u8, r[R_ENV_COLOR_G] as u8, r[R_ENV_COLOR_B] as u8, r[R_ENV_COLOR_A] as u8];
    out.xlu.push(colored(&bake_name(mode_of(r[R_DRAW_FLAGS]), tex), m, SEG_COLOR, prim, env));
}
