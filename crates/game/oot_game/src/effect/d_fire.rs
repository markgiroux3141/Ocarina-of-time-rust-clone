//! `Effect_Ss_D_Fire` (`ovl_Effect_Ss_D_Fire/z_eff_ss_d_fire.c`): the Dodongo's fire breath, a
//! puff of flame facing the camera (`object_dodongo`'s `gDodongoFireDL`, its four textures in turn
//! from one picked by `state.frames`), growing by its scale step, fading in 15 a frame until its
//! life is down to `rFadeDelay`, then out 5 a frame. Not spawned without `OBJECT_DODONGO` loaded
//! (`Object_GetSlot`).
//!
//! `rYAccelStep` (regs\[11\]) has no effect: it's 0 from the init and only ever added 0 (the C's
//! comment).
//!
//! @bug (game) (the C's comment): the init takes `gDodongoFireDL` through `SEGMENTED_TO_VIRTUAL`
//! with segment 6 not set to the object; it works because the only spawner, `En_Dodongo`, uses the
//! object and set segment 6 already. Here the bake names the object.

use glam::{Mat4, Vec3};

use super::{DrawCtx, EffectDraws, EffectSs, SEG_COLOR, SEG_SETUP, SEG_TEX, SsDraw, SsSpawn, SsUpdate, billboard_mtx, colored};
use crate::gbi::{G_CD_DISABLE, setup_dl};
use crate::pack::{BakeBody, BakeSegment, MeshBake};

// The overlay's regs.
const R_SCALE: usize = 0;
const R_TEX_INDEX: usize = 1;
const R_PRIM_COLOR_R: usize = 2;
const R_PRIM_COLOR_G: usize = 3;
const R_PRIM_COLOR_B: usize = 4;
const R_PRIM_COLOR_A: usize = 5;
const R_FADE_DELAY: usize = 6;
const R_SCALE_STEP: usize = 9;
const R_OBJECT_SLOT: usize = 10;
/// "Has no effect due to how it's implemented."
const R_Y_ACCEL_STEP: usize = 11;

/// `OBJECT_DODONGO` (`object_table.h`: 0x000C).
pub const OBJECT_DODONGO: i16 = 0x000C;
const OBJECT: &str = "object_dodongo";
const DLIST: &str = "gDodongoFireDL";

/// `sTextures`.
pub const TEXTURES: [&str; 4] = ["gDodongoFire0Tex", "gDodongoFire1Tex", "gDodongoFire2Tex", "gDodongoFire3Tex"];

/// `EffectSsDFireInitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct DFireInit {
    pub pos: Vec3,
    pub velocity: Vec3,
    pub accel: Vec3,
    pub scale: i16,
    pub scale_step: i16,
    pub alpha: i16,
    pub fade_delay: i16,
    pub life: i32,
}

/// `EffectSsDFire_Init`: with `OBJECT_DODONGO` in a slot, at `pos` moving as given for `life`
/// frames, prim (255, 255, 50, `alpha`), the texture `(state.frames % 4) ^ 3`; else nothing
/// spawns.
pub fn init(s: &mut SsSpawn<'_>, this: &mut EffectSs, p: &DFireInit) -> bool {
    // Object_GetSlot(&play->objectCtx, OBJECT_DODONGO): loaded or pending.
    let Some(slot) = s.objects.and_then(|o| o.get_index(OBJECT_DODONGO)) else { return false };
    this.pos = p.pos;
    this.velocity = p.velocity;
    this.accel = p.accel;
    // @bug (game): SEGMENTED_TO_VIRTUAL with segment 6 not set to the object (see the module docs).
    this.gfx = Some((OBJECT, DLIST));
    this.life = p.life as i16;
    let r = &mut this.regs;
    r[R_SCALE] = p.scale;
    r[R_SCALE_STEP] = p.scale_step;
    r[R_Y_ACCEL_STEP] = 0;
    r[R_OBJECT_SLOT] = slot as i16;
    this.draw = Some(SsDraw::DFire);
    this.update = Some(SsUpdate::DFire);
    r[R_TEX_INDEX] = ((s.state_frames % 4) as i16) ^ 3;
    r[R_PRIM_COLOR_R] = 255;
    r[R_PRIM_COLOR_G] = 255;
    r[R_PRIM_COLOR_B] = 50;
    r[R_PRIM_COLOR_A] = p.alpha;
    r[R_FADE_DELAY] = p.fade_delay;
    true
}

/// `EffectSsDFire_Update`: the next texture (of four), the scale up its step; with its life down
/// to `rFadeDelay` the alpha down 5 (not below 0), else up 15 (not above 255); falling, its
/// acceleration up `rYAccelStep / 100` (0).
pub fn update(this: &mut EffectSs) {
    let r = &mut this.regs;
    r[R_TEX_INDEX] += 1;
    r[R_TEX_INDEX] &= 3;
    r[R_SCALE] = r[R_SCALE].wrapping_add(r[R_SCALE_STEP]);
    if r[R_FADE_DELAY] >= this.life {
        r[R_PRIM_COLOR_A] -= 5;
        if r[R_PRIM_COLOR_A] < 0 {
            r[R_PRIM_COLOR_A] = 0;
        }
    } else {
        r[R_PRIM_COLOR_A] += 15;
        if r[R_PRIM_COLOR_A] > 255 {
            r[R_PRIM_COLOR_A] = 255;
        }
    }
    if this.accel.y < 0.0 {
        this.accel.y += r[R_Y_ACCEL_STEP] as f32 * 0.01;
    }
    if this.life <= 0 {
        r[R_Y_ACCEL_STEP] += 0;
    }
}

/// The bake of texture `n`.
pub fn bake_name(n: usize) -> String {
    format!("EffectSs/d_fire/{n}")
}

/// One bake per texture (`gDodongoFire0Tex` to `3Tex` on segment 8): `Gfx_SetupDL_60NoCDXlu`
/// (`SETUPDL_60` with `gDPSetColorDither(G_CD_DISABLE)`), the colours dynamic, `gDodongoFireDL`
/// (no matrix of its own: the draw's).
pub fn bakes() -> Vec<MeshBake> {
    let mut setup = setup_dl::setup_dl_60();
    setup.color_dither(G_CD_DISABLE);
    setup.end();
    TEXTURES
        .iter()
        .enumerate()
        .map(|(n, tex)| MeshBake {
            name: bake_name(n),
            object: OBJECT.into(),
            segments: vec![
                (SEG_TEX, BakeSegment::Texture { file: OBJECT.into(), symbol: (*tex).into() }),
                (SEG_SETUP, BakeSegment::Commands(setup.0.clone())),
                (SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true }),
            ],
            prelude: vec![SEG_SETUP, SEG_COLOR],
            body: BakeBody::DLists(vec![(OBJECT.into(), DLIST.into())]),
        })
        .collect()
}

/// `EffectSsDFire_Draw`'s matrix: `SkinMatrix_SetTranslate(pos)` × `billboardMtxF` ×
/// `SkinMatrix_SetScale(rScale / 100, rScale / 100, 1)`.
pub fn draw_mtx(this: &EffectSs, billboard: Mat4) -> Mat4 {
    let scale = this.regs[R_SCALE] as f32 / 100.0;
    billboard_mtx(this.pos, billboard, Vec3::new(scale, scale, 1.0))
}

/// `EffectSsDFire_Draw`: with `OBJECT_DODONGO` still in a slot, the puff at `draw_mtx`, env
/// (255, 0, 0, 0), prim (`rPrimColorR`..`A`), the texture `rTexIndex`.
pub fn draw(this: &EffectSs, ctx: &DrawCtx<'_>, out: &mut EffectDraws) {
    if ctx.objects.is_some_and(|o| o.get_index(OBJECT_DODONGO).is_none()) {
        return;
    }
    let r = &this.regs;
    let m = draw_mtx(this, ctx.billboard);
    let prim = [r[R_PRIM_COLOR_R] as u8, r[R_PRIM_COLOR_G] as u8, r[R_PRIM_COLOR_B] as u8, r[R_PRIM_COLOR_A] as u8];
    let env = [255, 0, 0, 0];
    let tex = (r[R_TEX_INDEX] & 3) as usize;
    out.xlu.push(colored(&bake_name(tex), m, SEG_COLOR, prim, env));
}

/// The regs a test reads: `rScale`, `rTexIndex`, the prim colour's alpha, `rFadeDelay`.
pub fn regs_of(this: &EffectSs) -> (i16, i16, i16, i16) {
    let r = &this.regs;
    (r[R_SCALE], r[R_TEX_INDEX], r[R_PRIM_COLOR_A], r[R_FADE_DELAY])
}
