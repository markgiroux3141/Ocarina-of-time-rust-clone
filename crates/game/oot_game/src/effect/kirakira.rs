//! `Effect_Ss_KiraKira` (`ovl_Effect_Ss_KiraKira/z_eff_ss_kirakira.c`): sparkles. A spinning
//! sparkle facing the camera (`gEffSparklesDL`), its env alpha pulsing, drifting at random
//! (`func_809AABF0`), or slowing as it drifts (`func_809AACAC`), or with a negative life growing
//! and shrinking in place (`func_809AAD6C`).

use glam::{Mat4, Vec3};

use super::{EffectDraws, EffectSs, SEG_COLOR, SsDraw, SsUpdate, SsSpawn, colored};
use crate::pack::{BakeBody, BakeSegment, MeshBake};
use crate::play::PlayState;

// The overlay's regs.
const R_ROT_SPEED: usize = 0;
const R_YAW: usize = 1;
const R_PRIM_COLOR_R: usize = 2;
const R_PRIM_COLOR_G: usize = 3;
const R_PRIM_COLOR_B: usize = 4;
const R_PRIM_COLOR_A: usize = 5;
const R_ENV_COLOR_R: usize = 6;
const R_ENV_COLOR_G: usize = 7;
const R_ENV_COLOR_B: usize = 8;
const R_ENV_COLOR_A: usize = 9;
const R_ALPHA_STEP: usize = 10;
const R_SCALE: usize = 11;
const R_LIFESPAN: usize = 12;

const BAKE: &str = "EffectSs/kirakira";

/// `EffectSsKiraKiraInitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct KiraKiraInit {
    pub pos: Vec3,
    pub velocity: Vec3,
    pub accel: Vec3,
    pub prim_color: [u8; 4],
    pub env_color: [u8; 4],
    pub alpha_step: i16,
    pub scale: i16,
    pub life: i32,
    pub rot_speed: i16,
    pub yaw: i16,
    pub update_mode: u8,
}

/// `EffectSsKiraKira_Init`: a negative life grows and shrinks (`func_809AAD6C`, its scale's peak in
/// the env alpha's reg); else update mode 0 (`func_809AABF0`) or 1 (`func_809AACAC`).
pub fn init(this: &mut EffectSs, p: &KiraKiraInit) -> bool {
    this.pos = p.pos;
    this.velocity = p.velocity;
    this.accel = p.accel;
    this.life = p.life as i16;
    this.gfx = Some(("gameplay_keep", "gEffSparklesDL"));
    if this.life < 0 {
        this.life = -this.life;
        this.update = Some(SsUpdate::KiraKiraGrow);
        this.regs[R_ENV_COLOR_A] = p.scale;
        this.regs[R_SCALE] = 0;
    } else {
        this.update = Some(if p.update_mode == 0 { SsUpdate::KiraKira } else { SsUpdate::KiraKiraFocused });
        this.regs[R_ENV_COLOR_A] = p.env_color[3] as i16;
        this.regs[R_SCALE] = p.scale;
    }
    this.draw = Some(SsDraw::KiraKira);
    let r = &mut this.regs;
    r[R_ROT_SPEED] = p.rot_speed;
    r[R_YAW] = p.yaw;
    r[R_PRIM_COLOR_R] = p.prim_color[0] as i16;
    r[R_PRIM_COLOR_G] = p.prim_color[1] as i16;
    r[R_PRIM_COLOR_B] = p.prim_color[2] as i16;
    r[R_PRIM_COLOR_A] = p.prim_color[3] as i16;
    r[R_ENV_COLOR_R] = p.env_color[0] as i16;
    r[R_ENV_COLOR_G] = p.env_color[1] as i16;
    r[R_ENV_COLOR_B] = p.env_color[2] as i16;
    r[R_ALPHA_STEP] = p.alpha_step;
    r[R_LIFESPAN] = p.life as i16;
    true
}

/// The env alpha stepped and bounced off 0 and 255, and the spin (both update modes' tails).
fn pulse_and_spin(this: &mut EffectSs) {
    let r = &mut this.regs;
    r[R_ENV_COLOR_A] = r[R_ENV_COLOR_A].wrapping_add(r[R_ALPHA_STEP]);
    if r[R_ENV_COLOR_A] < 0 {
        r[R_ENV_COLOR_A] = 0;
        r[R_ALPHA_STEP] = -r[R_ALPHA_STEP];
    } else if r[R_ENV_COLOR_A] > 255 {
        r[R_ENV_COLOR_A] = 255;
        r[R_ALPHA_STEP] = -r[R_ALPHA_STEP];
    }
    r[R_YAW] = r[R_YAW].wrapping_add(r[R_ROT_SPEED]);
}

/// `func_809AABF0`: a random sideways drift.
pub fn update(play: &mut PlayState, this: &mut EffectSs) {
    this.accel.x = (play.rand.zero_one() * 0.4) - 0.2;
    this.accel.z = (play.rand.zero_one() * 0.4) - 0.2;
    pulse_and_spin(this);
}

/// `func_809AACAC`: slowing sideways, a random drift.
pub fn update_focused(play: &mut PlayState, this: &mut EffectSs) {
    this.velocity.x *= 0.95;
    this.velocity.z *= 0.95;
    this.accel.x = play.rand.centered_float(0.2);
    this.accel.z = play.rand.centered_float(0.2);
    pulse_and_spin(this);
}

/// `func_809AAD6C`: the scale `peak × sinS((32768 / lifespan) × life)`.
pub fn update_grow(this: &mut EffectSs) {
    let r = &mut this.regs;
    let angle = ((32768.0 / r[R_LIFESPAN] as f32) * this.life as f32) as i32 as i16;
    r[R_SCALE] = (r[R_ENV_COLOR_A] as f32 * eng_math::sin_s(angle)) as i16;
}

/// `gEffSparklesDL` after `Gfx_SetupDL_25Xlu2` (the list's own setup), the colours dynamic.
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: BAKE.into(),
        object: "gameplay_keep".into(),
        segments: vec![(SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true })],
        prelude: vec![SEG_COLOR],
        body: BakeBody::DLists(vec![("gameplay_keep".into(), "gEffSparklesDL".into())]),
    }]
}

/// `EffectSsKiraKira_Draw`: at its position facing the camera (`billboardMtxF`), turned by its
/// yaw about the view axis, scaled `rScale / 10000` (flat in depth); the prim alpha
/// `(s8)((55 / lifespan) × life) + 200`.
pub fn draw(this: &EffectSs, billboard: Mat4, out: &mut EffectDraws) {
    let r = &this.regs;
    let scale = r[R_SCALE] as f32 / 10000.0;
    let yaw = r[R_YAW] as f32 * (std::f32::consts::PI / 32768.0);
    let m = Mat4::from_translation(this.pos) * billboard * Mat4::from_rotation_z(yaw) * Mat4::from_scale(Vec3::new(scale, scale, 1.0));
    let alpha = ((55.0 / r[R_LIFESPAN] as f32 * this.life as f32) as i32 as i8 as i32 + 200) as u8;
    let prim = [r[R_PRIM_COLOR_R] as u8, r[R_PRIM_COLOR_G] as u8, r[R_PRIM_COLOR_B] as u8, alpha];
    let env = [r[R_ENV_COLOR_R] as u8, r[R_ENV_COLOR_G] as u8, r[R_ENV_COLOR_B] as u8, r[R_ENV_COLOR_A] as u8];
    out.xlu.push(colored(BAKE, m, SEG_COLOR, prim, env));
}

impl SsSpawn<'_> {
    /// `EffectSsKiraKira_SpawnDispersed`: the vertical velocity and acceleration halved to
    /// `[0.5, 1) ×` at random, update mode 0, spinning 0x1518 from a random yaw, the alpha stepping
    /// down twice `255 / life` a frame.
    #[allow(clippy::too_many_arguments)]
    pub fn kirakira_spawn_dispersed(&mut self, pos: Vec3, velocity: Vec3, accel: Vec3, prim: [u8; 4], env: [u8; 4], scale: i16, life: i32) {
        let mut velocity = velocity;
        velocity.y = ((self.rand.zero_one() * velocity.y) + velocity.y) * 0.5;
        let mut accel = accel;
        accel.y = ((self.rand.zero_one() * accel.y) + accel.y) * 0.5;
        let yaw = (self.rand.zero_one() * 16384.0) as i16;
        let alpha_step = (-(255.0 / life as f32) + -(255.0 / life as f32)) as i16;
        let p = KiraKiraInit { pos, velocity, accel, prim_color: prim, env_color: env, alpha_step, scale, life, rot_speed: 0x1518, yaw, update_mode: 0 };
        self.spawn(super::EFFECT_SS_KIRAKIRA, 128, super::SsInit::KiraKira(p));
    }

    /// `EffectSsKiraKira_SpawnSmall`: dispersed, scale 1000, 16 frames.
    pub fn kirakira_spawn_small(&mut self, pos: Vec3, velocity: Vec3, accel: Vec3, prim: [u8; 4], env: [u8; 4]) {
        self.kirakira_spawn_dispersed(pos, velocity, accel, prim, env, 1000, 16);
    }

    /// `EffectSsKiraKira_SpawnSmallYellow`.
    pub fn kirakira_spawn_small_yellow(&mut self, pos: Vec3, velocity: Vec3, accel: Vec3) {
        self.kirakira_spawn_dispersed(pos, velocity, accel, [255, 255, 200, 255], [255, 200, 0, 0], 1000, 16);
    }

    /// `EffectSsKiraKira_SpawnFocused`: update mode 1, spinning 0x1518 from a random yaw, the
    /// alpha stepping down twice `255 / life` a frame.
    #[allow(clippy::too_many_arguments)]
    pub fn kirakira_spawn_focused(&mut self, pos: Vec3, velocity: Vec3, accel: Vec3, prim: [u8; 4], env: [u8; 4], scale: i16, life: i32) {
        let yaw = (self.rand.zero_one() * 16384.0) as i16;
        let alpha_step = (-(255.0 / life as f32) + -(255.0 / life as f32)) as i16;
        let p = KiraKiraInit { pos, velocity, accel, prim_color: prim, env_color: env, alpha_step, scale, life, rot_speed: 0x1518, yaw, update_mode: 1 };
        self.spawn(super::EFFECT_SS_KIRAKIRA, 128, super::SsInit::KiraKira(p));
    }
}
