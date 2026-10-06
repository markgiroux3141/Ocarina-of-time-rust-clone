//! `Effect_Ss_Sibuki` (`ovl_Effect_Ss_Sibuki/z_eff_ss_sibuki.c`): a bubble of the spray a
//! Gohma or a Gohma Larva gives off when hit (`EffectSsSibuki_SpawnBurst`, and the collision
//! check's `CollisionCheck_WaterBurst` for `COL_MATERIAL_HIT4`, which no actor has): it waits
//! `moveDelay` frames, then flies off to the camera's left or right (`direction`) and up,
//! falling and shrinking, gone below Player's floor.
//!
//! The overlay reads debug registers (`KREG(2)`, `KREG(3)`, `KREG(4)`, `KREG(18..27)`,
//! `KREG(64)`, `KREG(65)`), 0 in the retail game: taken as 0, so the bubble is always
//! `gEffBubble1Tex`.

use glam::{Mat4, Vec3};

use super::{EffectDraws, EffectSs, SEG_COLOR, SEG_TEX, SsDraw, SsSpawn, SsUpdate, colored};
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
const R_MOVE_DELAY: usize = 8;
const R_DIRECTION: usize = 9;
const R_SCALE: usize = 10;

/// `EffectSsSibukiInitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct SibukiInit {
    pub pos: Vec3,
    pub velocity: Vec3,
    pub accel: Vec3,
    pub move_delay: i16,
    pub direction: i16,
    pub scale: i16,
}

/// `EffectSsSibuki_Init`: `gEffBubble1Tex` (`KREG(2)` is 0), 10 to 14 frames
/// (`Rand_ZeroOne() × 500 × 0.01`, truncated, + 10), grey prim (100, alpha 100) and white env.
pub fn init(s: &mut SsSpawn<'_>, this: &mut EffectSs, p: &SibukiInit) -> bool {
    this.pos = p.pos;
    this.velocity = p.velocity;
    this.accel = p.accel;
    this.gfx = Some(("gameplay_keep", "gEffBubble1Tex"));
    // ((s16)((Rand_ZeroOne() * (500.0f + KREG(64))) * 0.01f)) + KREG(65) + 10.
    this.life = ((s.rand.zero_one() * 500.0) * 0.01) as i16 + 10;
    let r = &mut this.regs;
    r[R_MOVE_DELAY] = p.move_delay + 1;
    this.draw = Some(SsDraw::Sibuki);
    this.update = Some(SsUpdate::Sibuki);
    r[R_DIRECTION] = p.direction;
    r[R_SCALE] = p.scale;
    r[R_PRIM_COLOR_R] = 100;
    r[R_PRIM_COLOR_G] = 100;
    r[R_PRIM_COLOR_B] = 100;
    r[R_PRIM_COLOR_A] = 100;
    r[R_ENV_COLOR_R] = 255;
    r[R_ENV_COLOR_G] = 255;
    r[R_ENV_COLOR_B] = 255;
    r[R_ENV_COLOR_A] = 255;
    true
}

/// `EffectSsSibuki_Update`: at or below Player's floor it ends; its delay counting down, and
/// at 0 it's thrown along the main camera's input yaw (`Camera_GetInputDirYaw`), to the right
/// (or the left with `direction`) at 2 to 4 and up at 7 to 9, falling by 1 (three `Rand_ZeroOne`
/// calls, the last scaled by `KREG(25)`, 0); afterwards it shrinks by 3 a frame (while not 0).
pub fn update(play: &mut PlayState, this: &mut EffectSs) {
    let floor = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.floor_height).unwrap_or(f32::MIN);
    if this.pos.y <= floor {
        this.life = 0;
    }
    let r = &mut this.regs;
    if r[R_MOVE_DELAY] != 0 {
        r[R_MOVE_DELAY] -= 1;
        if r[R_MOVE_DELAY] == 0 {
            // Camera_GetInputDirYaw(Play_GetCamera(play, CAM_ID_MAIN)).
            let yaw = match play.camera_kind {
                crate::camera::CameraKind::Game => play.game_camera.input_dir_yaw(),
                crate::camera::CameraKind::Follow => play.follow_camera.input_dir_yaw(),
            };
            // ((200.0f + KREG(20)) * 0.01f) + ((0.1f * Rand_ZeroOne()) * (KREG(23) + 20.0f)).
            let mut xz_vel_scale = (200.0 * 0.01) + ((0.1 * play.rand.zero_one()) * 20.0);
            if r[R_DIRECTION] != 0 {
                xz_vel_scale *= -1.0;
            }
            this.velocity.x = eng_math::cos_s(yaw) * xz_vel_scale;
            this.velocity.z = -eng_math::sin_s(yaw) * xz_vel_scale;
            // ((700.0f + KREG(21)) * 0.01f) + ((0.1f * Rand_ZeroOne()) * (KREG(24) + 20.0f)).
            this.velocity.y = (700.0 * 0.01) + ((0.1 * play.rand.zero_one()) * 20.0);
            // ((-100.0f + KREG(22)) * 0.01f) + ((0.1f * Rand_ZeroOne()) * KREG(25)).
            this.accel.y = (-100.0 * 0.01) + ((0.1 * play.rand.zero_one()) * 0.0);
            // KREG(3) is 0: no rescaling.
        }
    } else if r[R_SCALE] != 0 {
        // (rScale - KREG(26)) - 3.
        r[R_SCALE] = r[R_SCALE].wrapping_sub(3);
    }
}

const BAKE: &str = "EffectSs/sibuki";

/// `gEffBubbleDL` after `Gfx_SetupDL_25Opa`, its texture `gEffBubble1Tex` on segment 8, the
/// colours dynamic; the list loads the billboard from segment 1 (the draw multiplies it in).
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: BAKE.into(),
        object: "gameplay_keep".into(),
        segments: vec![
            (SEG_TEX, BakeSegment::Texture { file: "gameplay_keep".into(), symbol: "gEffBubble1Tex".into() }),
            (SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true }),
            // gSPMatrix(0x01000000): the billboard, multiplied in by the draw.
            (0x01, BakeSegment::Bytes(super::identity_mtx())),
        ],
        prelude: vec![SEG_COLOR],
        body: BakeBody::DLists(vec![("gameplay_keep".into(), "gEffBubbleDL".into())]),
    }]
}

/// `EffectSsSibuki_Draw`: at its position scaled `rScale / 100`, then the list's billboard;
/// opaque, with its colours.
pub fn draw(this: &EffectSs, billboard: Mat4, out: &mut EffectDraws) {
    let r = &this.regs;
    let scale = r[R_SCALE] as f32 / 100.0;
    let m = Mat4::from_translation(this.pos) * Mat4::from_scale(Vec3::splat(scale)) * billboard;
    let prim = [r[R_PRIM_COLOR_R] as u8, r[R_PRIM_COLOR_G] as u8, r[R_PRIM_COLOR_B] as u8, r[R_PRIM_COLOR_A] as u8];
    let env = [r[R_ENV_COLOR_R] as u8, r[R_ENV_COLOR_G] as u8, r[R_ENV_COLOR_B] as u8, r[R_ENV_COLOR_A] as u8];
    out.opa.push(colored(BAKE, m, SEG_COLOR, prim, env));
}
