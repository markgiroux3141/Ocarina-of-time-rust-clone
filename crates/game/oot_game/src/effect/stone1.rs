//! `Effect_Ss_Stone1` (`ovl_Effect_Ss_Stone1/z_eff_ss_stone1.c`): the burst where a Deku Seed or
//! a Deku Nut hits (`EffectSsStone1_Spawn`, from `En_Arrow`'s `EnArrow_Fly`): eight frames, a
//! texture and a pair of colours a frame (`sDrawInfo`, by the life left: white and cyan first,
//! through yellow and orange to dark red), billboarded, 3 big (growing with the view depth past
//! 1500, so it keeps its size on screen).

use glam::{Mat4, Vec3};

use super::{DrawCtx, EffectDraws, EffectSs, SEG_COLOR, SEG_SETUP, SEG_TEX, SsDraw, SsUpdate, colored};
use crate::gbi::setup_dl;
use crate::pack::{BakeBody, BakeSegment, MeshBake};
use crate::play::PlayState;

// The overlay's regs.
/// `rFreezeFadeFlash` (`regs[0]`).
const R_FREEZE_FADE_FLASH: usize = 0;

/// `sDrawInfo`: by the life left (7 down to 0, so in reverse order), the texture
/// (`gUnknownEffStone<n>Tex`), the prim colour and the env colour (both drawn opaque).
const S_DRAW_INFO: [(u8, [u8; 3], [u8; 3]); 8] = [
    (8, [200, 0, 0], [0, 0, 0]),
    (7, [255, 100, 0], [100, 0, 0]),
    (6, [255, 200, 0], [200, 0, 0]),
    (5, [255, 255, 0], [255, 0, 0]),
    (4, [255, 255, 150], [255, 150, 0]),
    (3, [255, 255, 255], [255, 255, 0]),
    (2, [255, 255, 255], [0, 255, 0]),
    (1, [255, 255, 255], [0, 255, 255]),
];

/// `EffectSsStone1InitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct Stone1Init {
    pub pos: Vec3,
    /// `freezeFadeFlash`: see `update`. Every caller passes false.
    pub freeze_fade_flash: bool,
}

/// `EffectSsStone1_Init`: at `pos` (`vec` too), 8 frames.
pub fn init(this: &mut EffectSs, p: &Stone1Init) -> bool {
    this.pos = p.pos;
    this.vec = p.pos;
    this.life = 8;
    this.regs[R_FREEZE_FADE_FLASH] = p.freeze_fade_flash as i16;
    this.draw = Some(SsDraw::Stone1);
    this.update = Some(SsUpdate::Stone1);
    true
}

/// `EffectSsStone1_Update`: with `rFreezeFadeFlash`, at life 6 (its second frame)
/// `R_TRANS_FADE_FLASH_ALPHA_STEP` 0.
///
/// @bug (game): that freezes Play's flash where it is, the screen covered as it is two frames
/// into a flash until the next one or the next `Play_Init` (`z_eff_ss_stone1.h`). No caller
/// asks for it.
pub fn update(play: &mut PlayState, this: &mut EffectSs) {
    if this.life == 6 && this.regs[R_FREEZE_FADE_FLASH] != 0 {
        play.trans_fade_flash_alpha_step = 0;
    }
}

fn bake_name(n: u8) -> String {
    format!("EffectSs/stone1/{n}")
}

/// One bake per texture (`gUnknownEffStone1Tex` to `gUnknownEffStone8Tex`, on segment 8):
/// `Gfx_SetupDL_61Xlu`, the colours dynamic, `gUnknownEffStoneDL` (its `gSPMatrix` of segment 1,
/// the billboard, is multiplied in by the draw).
pub fn bakes() -> Vec<MeshBake> {
    let mut setup = setup_dl::setup_dl_61();
    setup.end();
    (1..=8)
        .map(|n| MeshBake {
            name: bake_name(n),
            object: "gameplay_keep".into(),
            segments: vec![
                (SEG_TEX, BakeSegment::Texture { file: "gameplay_keep".into(), symbol: format!("gUnknownEffStone{n}Tex") }),
                (SEG_SETUP, BakeSegment::Commands(setup.0.clone())),
                (SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true }),
                (0x01, BakeSegment::Bytes(super::identity_mtx())),
            ],
            prelude: vec![SEG_SETUP, SEG_COLOR],
            body: BakeBody::DLists(vec![("gameplay_keep".into(), "gUnknownEffStoneDL".into())]),
        })
        .collect()
}

/// The effect's scale: 3, or past a view depth of 1500 (`SkinMatrix_Vec3fMtxFMultXYZW` through
/// `viewProjectionMtxF`: its w) `depth / 1500 × 3`.
pub fn scale(view_proj: Mat4, pos: Vec3) -> f32 {
    let view_depth = (view_proj * pos.extend(1.0)).w;
    if view_depth < 1500.0 { 3.0 } else { (view_depth / 1500.0) * 3.0 }
}

/// `EffectSsStone1_Draw`: `Matrix_Translate(pos)`, `Matrix_Scale(scale)`, then the list's
/// billboard; `sDrawInfo[life]`'s texture and colours (alpha 255).
pub fn draw(this: &EffectSs, ctx: &DrawCtx<'_>, out: &mut EffectDraws) {
    let Some(&(tex, prim, env)) = S_DRAW_INFO.get(this.life.max(0) as usize) else { return };
    let s = scale(ctx.view_proj, this.pos);
    let m = Mat4::from_translation(this.pos) * Mat4::from_scale(Vec3::splat(s)) * ctx.billboard;
    out.xlu.push(colored(&bake_name(tex), m, SEG_COLOR, [prim[0], prim[1], prim[2], 255], [env[0], env[1], env[2], 255]));
}

/// `sDrawInfo[life]`: the texture number and the prim and env colours (for the tests).
pub fn draw_info(life: i16) -> Option<(u8, [u8; 3], [u8; 3])> {
    S_DRAW_INFO.get(usize::try_from(life).ok()?).copied()
}
