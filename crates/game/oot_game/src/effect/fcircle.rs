//! `Effect_Ss_Fcircle` (`ovl_Effect_Ss_Fcircle/z_eff_ss_fcircle.c`): a ring of fire round an
//! actor set alight (`EffectSsFCircle_Spawn`: a Mad Scrub hit by fire), following it and turning
//! with it, growing to full size in 5 frames and burning as long as the actor's colour filter
//! does (20 frames at most).

use glam::{Mat4, Vec3};

use super::{EffectDraws, EffectSs, SEG_COLOR, SEG_TEX, SsActor, SsDraw, SsUpdate};
use crate::pack::{BakeBody, BakeSegment, MeshBake, keys};
use crate::play::PlayState;
use crate::scene_table::gfx_two_tex_scroll;
use crate::sys_matrix::binang_to_rad;

// The overlay's regs.
/// `rUnused` ("probably supposed to be an alpha").
const R_UNUSED: usize = 3;
const R_RADIUS: usize = 8;
const R_HEIGHT: usize = 9;
const R_YAW: usize = 10;
const R_SCALE: usize = 11;

const BAKE: &str = "EffectSs/fcircle";

/// `EffectSsFcircleInitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct FcircleInit {
    pub actor: SsActor,
    pub pos: Vec3,
    pub radius: i16,
    pub height: i16,
}

/// `EffectSsFcircle_Init`: 20 frames at `pos`, kept at its offset from the actor (`vec`),
/// `gEffFireCircleDL`, turned as the actor is.
pub fn init(this: &mut EffectSs, p: &FcircleInit) -> bool {
    this.pos = p.pos;
    this.actor = Some(p.actor.handle);
    this.vec = p.pos - p.actor.world_pos;
    this.gfx = Some(("gameplay_keep", "gEffFireCircleDL"));
    this.life = 20;
    this.draw = Some(SsDraw::Fcircle);
    this.update = Some(SsUpdate::Fcircle);
    this.regs[R_UNUSED] = 255;
    this.regs[R_RADIUS] = p.radius;
    this.regs[R_HEIGHT] = p.height;
    this.regs[R_YAW] = p.actor.shape_rot[1];
    true
}

/// `EffectSsFcircle_Update`: while its actor lives (`actor->update != NULL`), at its offset from
/// it and turned with it, its life the actor's colour filter's timer (20 at most), growing to
/// 100 by 20; once the actor's gone, left where it was.
pub fn update(play: &mut PlayState, this: &mut EffectSs) {
    let Some(h) = this.actor else { return };
    // A deleted actor reads as it was left: update NULL.
    match play.actors.actor(h).filter(|a| !a.killed) {
        Some(a) => {
            this.pos = a.world_pos + this.vec;
            this.regs[R_YAW] = a.shape_rot.y;
            this.life = if a.color_filter_timer > 20 { 20 } else { a.color_filter_timer as i16 };
            eng_math::step_to_s(&mut this.regs[R_SCALE], 100, 20);
        }
        None => this.actor = None,
    }
}

/// `Gfx_TwoTexScroll(G_TX_RENDERTILE, gameplayFrames % 128, 0, 32, 64, 1, 0,
/// (gameplayFrames * -0xF) % 256, 32, 64)`: the flames' scroll (`gameplayFrames` is a `u32`).
fn scroll(gameplay_frames: u32) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, gameplay_frames % 128, 0, 32, 64, 1, 0, gameplay_frames.wrapping_mul(-0xF_i32 as u32) % 256, 32, 64)
}

/// `gEffFireCircleDL` after `Gfx_SetupDL_25Xlu` (the bake's start), its colours (prim with LOD
/// fraction 0x80 and minimum LOD 0x80) and the scroll on segment 8 dynamic.
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: BAKE.into(),
        object: "gameplay_keep".into(),
        segments: vec![(SEG_COLOR, BakeSegment::Dynamic(vec![(0xFA00_8080, 0xFFDC_00FF), (0xFB00_0000, 0xFF00_0000), (0xDF00_0000, 0)])), (SEG_TEX, BakeSegment::Dynamic(scroll(0)))],
        prelude: vec![SEG_COLOR],
        body: BakeBody::DLists(vec![("gameplay_keep".into(), "gEffFireCircleDL".into())]),
    }]
}

/// `EffectSsFcircle_Draw`: `rScale × (0.5 + life × 0.025)` of its radius and height (in
/// thousandths), turned to `rYaw`; yellow (255, 220, 0) fading with its life (`life × 12.75`)
/// over red.
pub fn draw(this: &EffectSs, gameplay_frames: u32, out: &mut EffectDraws) {
    let r = &this.regs;
    let scale = (r[R_SCALE] as f32 * (0.5 + (this.life as f32 * 0.025))) * 0.01;
    let y_scale = (r[R_HEIGHT] as f32 * 0.001) * scale;
    let xz_scale = (r[R_RADIUS] as f32 * 0.001) * scale;
    let m = Mat4::from_translation(this.pos) * Mat4::from_scale(Vec3::new(xz_scale, y_scale, xz_scale)) * Mat4::from_rotation_y(binang_to_rad(r[R_YAW]));
    let mut sv = eng_gfx::SegmentValues::default();
    sv.read(SEG_TEX, &scroll(gameplay_frames));
    sv.prim[SEG_COLOR as usize] = Some([255, 220, 0, (this.life as f32 * 12.75) as i32 as u8]);
    sv.env[SEG_COLOR as usize] = Some([255, 0, 0, 0]);
    out.xlu.push(eng_gfx::DrawCmd { mesh: eng_gfx::MeshKey::named(keys::bake(BAKE)), transform: m, bones: Vec::new(), params: eng_gfx::DrawParams { segments: Some(sv), ..Default::default() } });
}
