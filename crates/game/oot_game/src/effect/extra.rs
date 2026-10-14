//! `Effect_Ss_Extra` (`ovl_Effect_Ss_Extra/z_eff_ss_extra.c`): a mini-game's score points (30, 60
//! or 100), the Lost Woods' target scrub's and the horseback archery's. It needs
//! `object_yabusame_point` loaded (else it isn't made); it rises for five frames, then stays, for
//! 50, facing the camera; one frame before it stops rising it gives its score to the interface
//! (`interfaceCtx.unk_23C`, which the horseback archery's counter reads).

use glam::{Mat4, Vec3};

use super::{EffectDraws, EffectSs, SEG_TEX, SsDraw, SsSpawn, SsUpdate};
use eng_gfx::{DrawCmd, DrawParams, MeshKey};
use crate::pack::{BakeBody, BakeSegment, MeshBake};
use crate::play::PlayState;

// The overlay's regs.
/// `rObjectSlot` (`regs[0]`).
const R_OBJECT_SLOT: usize = 0;
/// `rTimer` (`regs[1]`).
const R_TIMER: usize = 1;
/// `rScoreIdx` (`regs[2]`).
const R_SCORE_IDX: usize = 2;
/// `rScale` (`regs[3]`).
const R_SCALE: usize = 3;

/// `OBJECT_YABUSAME_POINT` (`object_table.h`).
const OBJECT_YABUSAME_POINT: i16 = 0x0117;

/// `ExtraScoreIdx` (`z_eff_ss_extra.h`).
pub const EXTRA_SCORE_30: i16 = 0;
pub const EXTRA_SCORE_60: i16 = 1;
pub const EXTRA_SCORE_100: i16 = 2;

/// `sScores`.
const SCORES: [i16; 3] = [30, 60, 100];

/// `sTextures`: the score's texture by `rScoreIdx`.
const TEXTURES: [&str; 3] = ["object_yabusame_point_Tex_000000", "object_yabusame_point_Tex_000480", "object_yabusame_point_Tex_000900"];

/// `EffectSsExtraInitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtraInit {
    pub pos: Vec3,
    pub velocity: Vec3,
    pub accel: Vec3,
    pub scale: i16,
    pub score_idx: i16,
}

/// `EffectSsExtra_Init`: with `object_yabusame_point` loaded, 50 frames at `pos`; else nothing.
pub fn init(s: &mut SsSpawn<'_>, this: &mut EffectSs, p: &ExtraInit) -> bool {
    let slot = s.objects.and_then(|o| o.get_index(OBJECT_YABUSAME_POINT).filter(|&b| o.is_loaded(b)));
    let Some(slot) = slot else { return false };
    this.pos = p.pos;
    this.velocity = p.velocity;
    this.accel = p.accel;
    this.draw = Some(SsDraw::Extra);
    this.update = Some(SsUpdate::Extra);
    this.life = 50;
    this.regs[R_SCORE_IDX] = p.score_idx;
    this.regs[R_SCALE] = p.scale;
    this.regs[R_TIMER] = 5;
    this.regs[R_OBJECT_SLOT] = slot as i16;
    true
}

/// `EffectSsExtra_Update`: five frames rising, then still; at the timer's 1 the score to the
/// interface.
pub fn update(play: &mut PlayState, this: &mut EffectSs) {
    if this.regs[R_TIMER] != 0 {
        this.regs[R_TIMER] -= 1;
    } else {
        this.velocity.y = 0.0;
    }
    if this.regs[R_TIMER] == 1 {
        play.interface_ctx.unk_23c = SCORES.get(this.regs[R_SCORE_IDX] as usize).copied().unwrap_or(0) as u16;
    }
}

fn bake_name(score_idx: usize) -> String {
    format!("EffectSs/extra/{score_idx}")
}

/// One bake per score (`sTextures`, on segment 8): `Gfx_SetupDL_25Xlu` (the bake's start), then
/// `object_yabusame_point_DL_000DC0`.
pub fn bakes() -> Vec<MeshBake> {
    TEXTURES
        .iter()
        .enumerate()
        .map(|(i, tex)| MeshBake {
            name: bake_name(i),
            object: "object_yabusame_point".into(),
            segments: vec![(SEG_TEX, BakeSegment::Texture { file: "object_yabusame_point".into(), symbol: (*tex).into() })],
            prelude: vec![],
            body: BakeBody::DLists(vec![("object_yabusame_point".into(), "object_yabusame_point_DL_000DC0".into())]),
        })
        .collect()
}

/// `EffectSsExtra_Draw`: `Matrix_Translate(pos)`, `Matrix_Scale(rScale / 100)`,
/// `Matrix_ReplaceRotation(&play->billboardMtxF)`, the score's texture.
pub fn draw(this: &EffectSs, billboard: Mat4, out: &mut EffectDraws) {
    let Some(i) = usize::try_from(this.regs[R_SCORE_IDX]).ok().filter(|&i| i < TEXTURES.len()) else { return };
    let scale = this.regs[R_SCALE] as f32 / 100.0;
    let m = Mat4::from_translation(this.pos) * Mat4::from_scale(Vec3::splat(scale));
    let m = crate::draw::replace_rotation(m, billboard);
    out.xlu.push(DrawCmd { mesh: MeshKey::named(crate::pack::keys::bake(&bake_name(i))), transform: m, bones: Vec::new(), params: DrawParams::default() });
}

