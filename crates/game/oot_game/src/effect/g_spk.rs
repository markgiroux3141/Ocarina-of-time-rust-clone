//! `Effect_Ss_G_Spk` (`ovl_Effect_Ss_G_Spk/z_eff_ss_g_spk.c`): the sparks of a lit fuse (a
//! bomb's, a Bomb Flower's): a small quad facing the camera, its four textures in turn, growing,
//! drifting at random beside the explosive it follows while that's still one
//! (`ACTORCAT_EXPLOSIVE`) and there.
//!
//! The update mode 1 (`EffectSsGSpk_UpdateNoAccel`, `EffectSsGSpk_SpawnNoAccel`) and
//! `EffectSsGSpk_SpawnRandColor` are unused in the game; they're ported with the rest.

use glam::{Mat4, Vec3};

use super::{EffectDraws, EffectSs, SEG_COLOR, SEG_SETUP, SEG_TEX, SsActor, SsDraw, SsSpawn, SsUpdate, billboard_mtx, colored};
use crate::actor_ctx::ACTORCAT_EXPLOSIVE;
use crate::gbi::{G_CD_DISABLE, setup_dl};
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
const R_TEX_INDEX: usize = 8;
const R_SCALE: usize = 9;
const R_SCALE_STEP: usize = 10;

/// `EffectSsGSpkInitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct GSpkInit {
    pub actor: Option<SsActor>,
    pub pos: Vec3,
    pub velocity: Vec3,
    pub accel: Vec3,
    pub prim_color: [u8; 4],
    pub env_color: [u8; 4],
    pub scale: i16,
    pub scale_step: i16,
    pub update_mode: u8,
}

/// `EffectSsGSpk_Init`: update mode 0 lives 10 frames and keeps its offset from the actor
/// (`vec`; the C reads the actor through `initParams->actor`, which every caller gives); mode 1
/// lives 5.
pub fn init(this: &mut EffectSs, p: &GSpkInit) -> bool {
    this.pos = p.pos;
    this.velocity = p.velocity;
    this.accel = p.accel;
    this.gfx = Some(("gameplay_keep", "gEffSparkDL"));
    if p.update_mode == 0 {
        this.life = 10;
        if let Some(a) = p.actor {
            this.vec = p.pos - a.world_pos;
        }
        this.update = Some(SsUpdate::GSpk);
    } else {
        this.life = 5;
        this.update = Some(SsUpdate::GSpkNoAccel);
    }
    this.draw = Some(SsDraw::GSpk);
    let r = &mut this.regs;
    r[R_PRIM_COLOR_R] = p.prim_color[0] as i16;
    r[R_PRIM_COLOR_G] = p.prim_color[1] as i16;
    r[R_PRIM_COLOR_B] = p.prim_color[2] as i16;
    r[R_PRIM_COLOR_A] = p.prim_color[3] as i16;
    r[R_ENV_COLOR_R] = p.env_color[0] as i16;
    r[R_ENV_COLOR_G] = p.env_color[1] as i16;
    r[R_ENV_COLOR_B] = p.env_color[2] as i16;
    r[R_ENV_COLOR_A] = p.env_color[3] as i16;
    r[R_TEX_INDEX] = 0;
    r[R_SCALE] = p.scale;
    r[R_SCALE_STEP] = p.scale_step;
    this.actor = p.actor.map(|a| a.handle);
    true
}

/// The actor the spark follows, while it's an explosive still there (`update != NULL`; a deleted
/// one reads as killed).
fn explosive(play: &PlayState, this: &EffectSs) -> Option<(Vec3, i16, f32)> {
    let a = play.actors.actor(this.actor?)?;
    (a.category == ACTORCAT_EXPLOSIVE && !a.killed).then_some((a.world_pos, a.world_rot.y, a.speed_xz))
}

/// `EffectSsGSpk_Update`: a random push across (±1.5) for next frame; at the explosive plus its
/// offset while there is one; the offset drifting by the push, the next texture, the scale's step.
pub fn update(play: &mut PlayState, this: &mut EffectSs) {
    this.accel.x = (play.rand.zero_one() - 0.5) * 3.0;
    this.accel.z = (play.rand.zero_one() - 0.5) * 3.0;
    if let Some((pos, _, _)) = explosive(play, this) {
        this.pos = pos + this.vec;
    }
    this.vec.x += this.accel.x;
    this.vec.z += this.accel.z;
    step(this);
}

/// `EffectSsGSpk_UpdateNoAccel` (unused in the game): moved along with the explosive's speed.
pub fn update_no_accel(play: &mut PlayState, this: &mut EffectSs) {
    if let Some((_, yaw, speed)) = explosive(play, this) {
        this.pos.x += eng_math::sin_s(yaw) * speed;
        this.pos.z += eng_math::cos_s(yaw) * speed;
    }
    step(this);
}

/// Both updates' end: the next of the four textures, the scale's step.
fn step(this: &mut EffectSs) {
    let r = &mut this.regs;
    r[R_TEX_INDEX] = (r[R_TEX_INDEX] + 1) & 3;
    r[R_SCALE] = r[R_SCALE].wrapping_add(r[R_SCALE_STEP]);
}

fn bake_name(n: usize) -> String {
    format!("EffectSs/g_spk/{n}")
}

/// One bake per texture (`sparkTextures`: `gEffSpark1Tex` to `4`): `Gfx_SetupDL_60NoCDXlu`, the
/// colours dynamic, `gEffSparkDL`.
pub fn bakes() -> Vec<MeshBake> {
    let mut setup = setup_dl::setup_dl_60();
    setup.color_dither(G_CD_DISABLE);
    setup.end();
    (1..=4)
        .map(|n| MeshBake {
            name: bake_name(n),
            object: "gameplay_keep".into(),
            segments: vec![
                (SEG_TEX, BakeSegment::Texture { file: "gameplay_keep".into(), symbol: format!("gEffSpark{n}Tex") }),
                (SEG_SETUP, BakeSegment::Commands(setup.0.clone())),
                (SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true }),
            ],
            prelude: vec![SEG_SETUP, SEG_COLOR],
            body: BakeBody::DLists(vec![("gameplay_keep".into(), "gEffSparkDL".into())]),
        })
        .collect()
}

/// `EffectSsGSpk_Draw`: at its position facing the camera, `rScale × 0.0025` across (1 deep),
/// prim alpha 255.
pub fn draw(this: &EffectSs, billboard: Mat4, out: &mut EffectDraws) {
    let r = &this.regs;
    let scale = r[R_SCALE] as f32 * 0.0025;
    let m = billboard_mtx(this.pos, billboard, Vec3::new(scale, scale, 1.0));
    let tex = (r[R_TEX_INDEX] as usize & 3) + 1;
    let prim = [r[R_PRIM_COLOR_R] as u8, r[R_PRIM_COLOR_G] as u8, r[R_PRIM_COLOR_B] as u8, 255];
    let env = [r[R_ENV_COLOR_R] as u8, r[R_ENV_COLOR_G] as u8, r[R_ENV_COLOR_B] as u8, r[R_ENV_COLOR_A] as u8];
    out.xlu.push(colored(&bake_name(tex), m, SEG_COLOR, prim, env));
}

impl SsSpawn<'_> {
    /// `EffectSsGSpk_SpawnAccel`: update mode 0, priority 128.
    #[allow(clippy::too_many_arguments)]
    pub fn g_spk_spawn_accel(&mut self, actor: Option<SsActor>, pos: Vec3, velocity: Vec3, accel: Vec3, prim: [u8; 4], env: [u8; 4], scale: i16, scale_step: i16) {
        let p = GSpkInit { actor, pos, velocity, accel, prim_color: prim, env_color: env, scale, scale_step, update_mode: 0 };
        self.spawn(super::EFFECT_SS_G_SPK, 128, super::SsInit::GSpk(p));
    }

    /// `EffectSsGSpk_SpawnNoAccel` (unused in the game): update mode 1.
    #[allow(clippy::too_many_arguments)]
    pub fn g_spk_spawn_no_accel(&mut self, actor: Option<SsActor>, pos: Vec3, velocity: Vec3, accel: Vec3, prim: [u8; 4], env: [u8; 4], scale: i16, scale_step: i16) {
        let p = GSpkInit { actor, pos, velocity, accel, prim_color: prim, env_color: env, scale, scale_step, update_mode: 1 };
        self.spawn(super::EFFECT_SS_G_SPK, 128, super::SsInit::GSpk(p));
    }

    /// `EffectSsGSpk_SpawnSmall`: scale 100, growing 5 a frame.
    pub fn g_spk_spawn_small(&mut self, actor: Option<SsActor>, pos: Vec3, velocity: Vec3, accel: Vec3, prim: [u8; 4], env: [u8; 4]) {
        self.g_spk_spawn_accel(actor, pos, velocity, accel, prim, env, 100, 5);
    }

    /// `EffectSsGSpk_SpawnFuse`: a fuse's spark, pale yellow (255, 255, 150) on red.
    pub fn g_spk_spawn_fuse(&mut self, actor: Option<SsActor>, pos: Vec3, velocity: Vec3, accel: Vec3) {
        self.g_spk_spawn_small(actor, pos, velocity, accel, [255, 255, 150, 255], [255, 0, 0, 0]);
    }

    /// `EffectSsGSpk_SpawnRandColor` (unused in the game): both colours shifted by one random
    /// amount from -10 to 10 (truncated), each channel wrapping as a `u8`.
    #[allow(clippy::too_many_arguments)]
    pub fn g_spk_spawn_rand_color(&mut self, actor: Option<SsActor>, pos: Vec3, velocity: Vec3, accel: Vec3, scale: i16, scale_step: i16) {
        let rand_offset = ((self.rand.zero_one() * 20.0) - 10.0) as i32;
        let shift = |c: [u8; 4]| c.map(|v| (v as i32 + rand_offset) as u8);
        let (prim, env) = (shift([255, 255, 150, 255]), shift([255, 0, 0, 0]));
        self.g_spk_spawn_accel(actor, pos, velocity, accel, prim, env, scale, scale_step);
    }
}
