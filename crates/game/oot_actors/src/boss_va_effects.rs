//! Barinade's particles (`z_boss_va.c`'s `BossVaEffect`, `sEffects[400]`, `:3304-3973`): one
//! array the whole overlay shares (`Statics::effects`), spawned by every part of him and updated and
//! drawn by the body (`BossVa_UpdateEffects` in its update, `BossVa_DrawEffects` in its draw).
//!
//! - **Sparks** (`BossVa_SpawnSpark`, by mode): large ones (`VA_LARGE_SPARK`, facing the camera)
//!   on a Bari's tether (`SPARK_TETHER`: along the line from the Bari to the body's `unk_1D8`, kept
//!   there as the Bari turns) or held at a Bari (`SPARK_BARI`); small ones (`VA_SMALL_SPARK`, turned
//!   at random about z and y) held at the body or a part of him (`SPARK_BODY`) or jumping about
//!   Link's body parts (`SPARK_LINK`, while the zappers' lightning holds him); the lightning's
//!   (`VA_BLAST_SPARK`, `SPARK_BLAST`) stay where they were made. All fade 50 a frame once their
//!   timer (111 to 120) is under 100.
//! - **Spark balls** (`VA_SPARK_BALL`, phase 2): over the body's top, their texture cycling
//!   through `gBarinadeSparkBall1Tex` to `8Tex`.
//! - **The zapper's charge** (`VA_ZAP_CHARGE`): at its head, turned as it aims, fading 20 a frame.
//! - **Blood** (`VA_BLOOD`): droplets from a cut support's arm (`BLOOD_DROPLET`) and splatter from a
//!   bursting tumour (`BLOOD_SPLATTER`), falling (-1 ± 0.15 a frame) until the floor under them
//!   (`BgCheck_EntityRaycastDown1`, 4 below where its velocity takes it) stops them: a spot
//!   (`BLOOD_SPOT`), flat, for 80 frames before his shell bursts, for good after.
//! - **Tumours** (`VA_TUMOR`): growing on the body (`TUMOR_BODY`, drawn only on the camera's side
//!   of him) and on the supports' and zappers' arms (`TUMOR_ARM`) as he dies; when their part
//!   bursts (`burst`) each throws blood and gore.
//! - **Gore** (`VA_GORE`): falling and spinning; before the shell bursts it's gone in 20 frames
//!   (`GORE_FADING`), after it lands (`GORE_FLOOR`, the same raycast) and pulses there.
//!
//! The parents (`effect->parent`, a `BossVa*`) are actor handles. A parent's state is read each
//! update; once it's gone (a Bari killed while its sparks fade) the C reads the freed actor's
//! memory: here its last state read (`ParentState`) stands in.

use eng_math::{cos_s, sin_s, smooth_step_to_f, smooth_step_to_s, vec3f_pitch, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor_ctx::{ActorHandle, PLAYER_BODYPART_MAX};
use oot_game::audio::sfx::SfxPos;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, SEGMENT_CALL, keys};
use oot_game::play::{DrawOut, PlayState, ViewInfo};
use oot_game::sys_matrix::MtxF;

use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};

use crate::boss_va::{DEATH_SHELL_BURST, NA_SE_EN_BALINADE_BREAK, OBJECT};

/// `BOSS_VA_EFFECT_COUNT`.
pub const BOSS_VA_EFFECT_COUNT: usize = 400;

/// `BossVaEffectType`.
pub const VA_NONE: u8 = 0;
pub const VA_LARGE_SPARK: u8 = 1;
pub const VA_BLAST_SPARK: u8 = 2;
pub const VA_SMALL_SPARK: u8 = 3;
pub const VA_SPARK_BALL: u8 = 4;
pub const VA_ZAP_CHARGE: u8 = 5;
pub const VA_BLOOD: u8 = 6;
pub const VA_TUMOR: u8 = 7;
pub const VA_GORE: u8 = 8;

/// `BossVaSparkMode`.
pub const SPARK_TETHER: u8 = 1;
pub const SPARK_BARI: u8 = 2;
pub const SPARK_BLAST: u8 = 3;
pub const SPARK_UNUSED: u8 = 4;
pub const SPARK_BODY: u8 = 5;
pub const SPARK_LINK: u8 = 6;

/// `BossVaBloodMode`.
pub const BLOOD_DROPLET: i16 = 0;
pub const BLOOD_SPLATTER: i16 = 1;
pub const BLOOD_SPOT: i16 = 2;

/// `BossVaTumorMode`.
pub const TUMOR_UNUSED: u8 = 0;
pub const TUMOR_BODY: u8 = 1;
pub const TUMOR_ARM: u8 = 2;

/// `BossVaGoreMode`.
pub const GORE_PERMANENT: i16 = 0;
pub const GORE_FLOOR: i16 = 1;
pub const GORE_FADING: i16 = 2;

/// What an effect reads of its parent (`effect->parent`): its place, yaw, model offset and
/// scale, `burst`, and its body's `unk_1D8` (`GET_BODY(refActor)->unk_1D8`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ParentState {
    pub world_pos: Vec3,
    pub world_rot_y: i16,
    pub y_offset: f32,
    pub scale_y: f32,
    pub burst: u8,
    pub body_unk_1d8: Vec3,
}

/// `BossVaEffect`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BossVaEffect {
    pub pos: Vec3,
    pub velocity: Vec3,
    pub accel: Vec3,
    pub ty: u8,
    pub timer: u16,
    pub mode: i16,
    pub rot: [i16; 3],
    pub prim_color: [i16; 4],
    pub env_color: [i16; 4],
    pub scale: f32,
    pub scale_mod: f32,
    /// `offset`; a gore's `vaGorePulse` (x) and `vaGorePulseRate` (y).
    pub offset: Vec3,
    pub parent: Option<ActorHandle>,
    /// Not in the C: the parent as last read (see the module's docs).
    pub parent_state: ParentState,
}

/// The spawning part (`BossVa* this`): its handle and state.
#[derive(Debug, Clone, Copy)]
pub struct Spawner {
    pub handle: Option<ActorHandle>,
    pub state: ParentState,
}

const HIDDEN: Vec3 = Vec3::new(0.0, -1000.0, 0.0);

/// Player's `bodyPartsPos[i]` (the origin without him).
fn player_body_part(play: &PlayState, i: usize) -> Vec3 {
    play.player.and_then(|h| play.actors.downcast::<crate::player::Player>(h)).map_or(Vec3::ZERO, |p| p.body_parts_pos.get(i).copied().unwrap_or(Vec3::ZERO))
}

fn dist_xz(a: Vec3, b: Vec3) -> f32 {
    ((b.x - a.x) * (b.x - a.x) + (b.z - a.z) * (b.z - a.z)).sqrt()
}

/// `BossVa_SpawnSpark`.
pub fn spawn_spark(play: &mut PlayState, effects: &mut [BossVaEffect], this: &Spawner, offset: Vec3, scale: i16, mode: u8) {
    for effect in effects.iter_mut() {
        if effect.ty != VA_NONE {
            continue;
        }
        effect.ty = VA_LARGE_SPARK;
        effect.parent = this.handle;
        effect.parent_state = this.state;
        effect.pos = HIDDEN;
        effect.timer = ((play.rand.zero_one() * 10.0) as i16 + 111) as u16;
        effect.velocity = Vec3::ZERO;
        effect.accel = Vec3::ZERO;
        effect.mode = mode as i16;
        let w = this.state.world_pos;
        match mode {
            SPARK_UNUSED | SPARK_TETHER => {
                if mode == SPARK_UNUSED {
                    effect.ty = VA_SMALL_SPARK;
                }
                let temp = Vec3::new(offset.x + w.x, offset.y, offset.z + w.z);
                effect.offset.x = dist_xz(w, temp);
                effect.rot[0] = vec3f_pitch(w, this.state.body_unk_1d8);
            }
            SPARK_BODY | SPARK_BARI => {
                if mode == SPARK_BODY {
                    effect.ty = VA_SMALL_SPARK;
                }
                effect.offset.x = offset.x;
                effect.offset.z = offset.z;
            }
            SPARK_BLAST => {
                effect.ty = VA_BLAST_SPARK;
                effect.pos = offset + w;
                effect.timer = 111;
            }
            SPARK_LINK => {
                effect.ty = VA_SMALL_SPARK;
                let index = play.rand.zero_float(PLAYER_BODYPART_MAX as f32 - 0.1) as i16 as usize;
                let p = player_body_part(play, index);
                effect.pos.x = p.x + play.rand.centered_float(10.0);
                effect.pos.y = p.y + play.rand.centered_float(15.0);
                effect.pos.z = p.z + play.rand.centered_float(10.0);
            }
            _ => {}
        }
        effect.offset.y = offset.y;
        effect.scale = (play.rand.zero_float(scale as f32) + scale as f32) * 0.01;
        effect.prim_color[3] = 255;
        break;
    }
}

/// `BossVa_SpawnSparkBall`.
pub fn spawn_spark_ball(play: &mut PlayState, effects: &mut [BossVaEffect], this: &Spawner, offset: Vec3, scale: i16) {
    for effect in effects.iter_mut() {
        if effect.ty != VA_NONE {
            continue;
        }
        effect.ty = VA_SPARK_BALL;
        effect.parent = this.handle;
        effect.parent_state = this.state;
        effect.pos = HIDDEN;
        effect.velocity = Vec3::ZERO;
        effect.accel = Vec3::ZERO;
        effect.mode = 0;
        effect.offset = offset;
        effect.timer = ((play.rand.zero_one() * 10.0) as i16 + 111) as u16;
        effect.prim_color = [230; 4];
        effect.env_color = [0, 100, 220, 160];
        effect.scale = (play.rand.zero_float(scale as f32) + scale as f32) * 0.01;
        return;
    }
}

/// `BossVa_SpawnBloodDroplets`.
pub fn spawn_blood_droplets(play: &mut PlayState, effects: &mut [BossVaEffect], pos: Vec3, scale: i16, phase: i16, yaw: i16) {
    for effect in effects.iter_mut() {
        if effect.ty != VA_NONE {
            continue;
        }
        effect.ty = VA_BLOOD;
        effect.pos = pos;
        effect.mode = BLOOD_DROPLET;
        let xz_vel = sin_s(phase) * 6.0;
        let vx = play.rand.centered_float(1.0) + (-sin_s(yaw) * xz_vel);
        let vz = play.rand.centered_float(1.0) + (-cos_s(yaw) * xz_vel);
        effect.velocity = Vec3::new(vx, 0.0, vz);
        effect.accel = Vec3::new(0.0, play.rand.centered_float(0.3) - 1.0, 0.0);
        effect.timer = 20;
        effect.env_color[3] = 100;
        effect.prim_color[3] = 200;
        effect.scale = (play.rand.zero_float(scale as f32) + scale as f32) * 0.01;
        break;
    }
}

/// `BossVa_SpawnBloodSplatter`.
pub fn spawn_blood_splatter(play: &mut PlayState, effects: &mut [BossVaEffect], pos: Vec3, yaw: i16, scale: i16, cs_state: i8) {
    for effect in effects.iter_mut() {
        if effect.ty != VA_NONE {
            continue;
        }
        effect.ty = VA_BLOOD;
        effect.pos = pos;
        effect.mode = BLOOD_SPLATTER;
        let xz_vel = play.rand.zero_one() * 7.0;
        let vx = sin_s(yaw) * xz_vel;
        let vy = play.rand.centered_float(4.0) + 4.0;
        let vz = cos_s(yaw) * xz_vel;
        effect.velocity = Vec3::new(vx, vy, vz);
        effect.accel = Vec3::new(0.0, play.rand.centered_float(0.3) - 1.0, 0.0);
        effect.timer = if cs_state <= DEATH_SHELL_BURST { 20 } else { 60 };
        effect.env_color[3] = 100;
        effect.prim_color[3] = 200;
        effect.scale = scale as f32 * 0.01;
        break;
    }
}

/// `BossVa_SpawnTumor`: `NA_SE_EN_BALINADE_BREAK` for every fourth slot and each arm's.
/// `SFX_PLAY_AT_POS(&effect->pos)` hands the hidden place (0, -1000, 0) as a projected position:
/// played at `gSfxDefaultPos` here.
pub fn spawn_tumor(play: &mut PlayState, effects: &mut [BossVaEffect], this: &Spawner, offset: Vec3, scale: i16, mode: u8) {
    for (i, effect) in effects.iter_mut().enumerate() {
        if effect.ty != VA_NONE {
            continue;
        }
        effect.ty = VA_TUMOR;
        effect.parent = this.handle;
        effect.parent_state = this.state;
        effect.pos = HIDDEN;
        effect.velocity = Vec3::ZERO;
        effect.accel = Vec3::ZERO;
        effect.mode = mode as i16;
        effect.rot[2] = 0;
        effect.offset = offset;
        effect.timer = ((play.rand.zero_one() * 10.0) as i16 + 10) as u16;
        effect.env_color[3] = 100;
        effect.scale_mod = scale as f32 * 0.01;
        effect.scale = 0.0;
        if i % 4 == 0 || mode == TUMOR_ARM {
            play.audio.play_sfx_at_pos(SfxPos::Default, NA_SE_EN_BALINADE_BREAK);
        }
        break;
    }
}

/// `BossVa_SpawnGore`.
pub fn spawn_gore(play: &mut PlayState, effects: &mut [BossVaEffect], pos: Vec3, yaw: i16, scale: i16, cs_state: i8) {
    for effect in effects.iter_mut() {
        if effect.ty != VA_NONE {
            continue;
        }
        effect.ty = VA_GORE;
        effect.pos = pos;
        effect.scale_mod = 0.0;
        let xz_vel = (play.rand.zero_one() * 4.0) + 4.0;
        let vx = sin_s(yaw) * xz_vel;
        let vy = play.rand.centered_float(8.0);
        let vz = cos_s(yaw) * xz_vel;
        effect.velocity = Vec3::new(vx, vy, vz);
        effect.accel = Vec3::new(0.0, play.rand.centered_float(0.3) - 1.0, 0.0);
        effect.timer = 20;
        effect.mode = if cs_state <= DEATH_SHELL_BURST { GORE_FADING } else { GORE_PERMANENT };
        effect.env_color = [255; 4];
        effect.prim_color = [155, 55, 55, 255];
        effect.rot[0] = play.rand.centered_float(65536.0) as i32 as i16;
        effect.rot[1] = play.rand.centered_float(65536.0) as i32 as i16;
        effect.scale = (play.rand.zero_float(scale as f32) + scale as f32) * 0.01;
        effect.offset.y = (play.rand.zero_one() * 0.25) + 0.9;
        break;
    }
}

/// `BossVa_SpawnZapperCharge`.
pub fn spawn_zapper_charge(play: &mut PlayState, effects: &mut [BossVaEffect], this: &Spawner, pos: Vec3, rot: [i16; 3], scale: i16, mode: u8) {
    for effect in effects.iter_mut() {
        if effect.ty != VA_NONE {
            continue;
        }
        effect.ty = VA_ZAP_CHARGE;
        effect.parent = this.handle;
        effect.parent_state = this.state;
        effect.pos = pos;
        effect.velocity = Vec3::ZERO;
        effect.accel = Vec3::ZERO;
        effect.mode = mode as i16;
        effect.rot[0] = rot[0].wrapping_add(0x4000);
        effect.rot[1] = rot[1];
        effect.timer = ((play.rand.zero_one() * 10.0) as i16 + 10) as u16;
        effect.prim_color[3] = 240;
        effect.scale = scale as f32 * 0.01;
        break;
    }
}

/// `BossVa_Spark`/`BossVa_Tumor`'s index: `range - 0.6` (fixed) or a random one below it, as an
/// `s16`. @bug (game): a range of 11 or 12 reads `effectPos[10]` and `[11]`, past the array:
/// `unk_274` and `unk_280` (the port's `effect_pos` holds them as entries 10 and 11).
fn spawn_index(play: &mut PlayState, range: f32, fixed: bool) -> usize {
    let i = if !fixed { (play.rand.zero_one() * (range - 0.6)) as i16 } else { (range - 0.6) as i16 };
    (i.max(0) as usize).min(11)
}

/// `BossVa_Spark`: `count` sparks round `effect_pos[index]`, spread `xz_spread` across and
/// `y_spread` up, as offsets from `this`.
#[allow(clippy::too_many_arguments)]
pub fn spark(play: &mut PlayState, effects: &mut [BossVaEffect], this: &Spawner, effect_pos: &[Vec3; 12], count: i32, scale: i16, xz_spread: f32, y_spread: f32, mode: u8, range: f32, fixed: bool) {
    let w = this.state.world_pos;
    for _ in 0..count {
        let index = spawn_index(play, range, fixed);
        let p = effect_pos[index];
        let x = play.rand.centered_float(xz_spread) + p.x - w.x;
        let y = play.rand.centered_float(y_spread) + p.y - w.y;
        let z = play.rand.centered_float(xz_spread) + p.z - w.z;
        spawn_spark(play, effects, this, Vec3::new(x, y, z), scale, mode);
    }
}

/// `BossVa_Tumor`: as `spark`, tumours.
#[allow(clippy::too_many_arguments)]
pub fn tumor(play: &mut PlayState, effects: &mut [BossVaEffect], this: &Spawner, effect_pos: &[Vec3; 12], count: i32, scale: i16, xz_spread: f32, y_spread: f32, mode: u8, range: f32, fixed: bool) {
    let w = this.state.world_pos;
    for _ in 0..count {
        let index = spawn_index(play, range, fixed);
        let p = effect_pos[index];
        let x = play.rand.centered_float(xz_spread) + p.x - w.x;
        let y = play.rand.centered_float(y_spread) + p.y - w.y;
        let z = play.rand.centered_float(xz_spread) + p.z - w.z;
        spawn_tumor(play, effects, this, Vec3::new(x, y, z), scale, mode);
    }
}

/// `BossVa_BloodDroplets`: two droplets about `pos` (scale 65).
pub fn blood_droplets(play: &mut PlayState, effects: &mut [BossVaEffect], pos: Vec3, phase: i16, yaw: i16) {
    for _ in 0..2 {
        let x = play.rand.centered_float(10.0) + pos.x;
        let y = pos.y - (play.rand.zero_one() * 15.0);
        let z = play.rand.centered_float(10.0) + pos.z;
        spawn_blood_droplets(play, effects, Vec3::new(x, y, z), 65, phase, yaw);
    }
}

/// `BossVa_BloodSplatter`: `count` splatters about `src`, thrown round `yaw`.
pub fn blood_splatter(play: &mut PlayState, effects: &mut [BossVaEffect], src: Vec3, yaw: i16, scale: i16, count: i32, cs_state: i8) {
    for _ in 0..count {
        let x = play.rand.centered_float(10.0) + src.x;
        let y = src.y - (play.rand.zero_one() * 15.0);
        let z = play.rand.centered_float(10.0) + src.z;
        let yaw = (play.rand.centered_float(0x6590 as f32) as i32 as i16).wrapping_add(yaw);
        spawn_blood_splatter(play, effects, Vec3::new(x, y, z), yaw, scale, cs_state);
    }
}

/// `BossVa_Gore`: two pieces before the shell bursts, one after.
pub fn gore(play: &mut PlayState, effects: &mut [BossVaEffect], src: Vec3, yaw: i16, scale: i16, cs_state: i8) {
    let n = if cs_state <= DEATH_SHELL_BURST { 2 } else { 1 };
    for _ in 0..n {
        let x = play.rand.centered_float(10.0) + src.x;
        let y = play.rand.centered_float(10.0) + src.y;
        let z = play.rand.centered_float(10.0) + src.z;
        let yaw = (play.rand.centered_float(0x6590 as f32) as i32 as i16).wrapping_add(yaw);
        spawn_gore(play, effects, Vec3::new(x, y, z), yaw, scale, cs_state);
    }
}

/// A spark's, spark ball's and blast's fade: 50 a frame under 100 frames left; gone at 0.
fn fade(effect: &mut BossVaEffect) {
    if effect.timer < 100 {
        effect.prim_color[3] -= 50;
        if effect.prim_color[3] < 0 {
            effect.prim_color[3] = 0;
            effect.timer = 0;
            effect.ty = VA_NONE;
        }
    }
}

/// `BgCheck_EntityRaycastDown1` from 4 below where the velocity takes it: the floor's y, if any.
fn floor_under(play: &PlayState, effect: &BossVaEffect) -> Option<f32> {
    let mut check = effect.pos;
    check.y -= effect.velocity.y + 4.0;
    let (y, poly) = play.col.entity_raycast_down(check);
    poly.map(|_| y)
}

/// `BossVa_UpdateEffects`: `resolve` gives a parent's state (the body's own while it updates).
/// @bug (game): a tumour's gore is thrown at the tumour's scale × 1.2 as an `s16` (`BossVa_Gore`'s
/// parameter): 0, so every gore piece is spawned (and drawn) at scale 0; kept.
pub fn update_effects(play: &mut PlayState, effects: &mut [BossVaEffect], cs_state: i8, resolve: &dyn Fn(&PlayState, ActorHandle) -> Option<ParentState>) {
    for i in 0..effects.len() {
        if effects[i].ty == VA_NONE {
            continue;
        }
        if let Some(h) = effects[i].parent
            && let Some(s) = resolve(play, h)
        {
            effects[i].parent_state = s;
        }
        let effect = &mut effects[i];
        let r = effect.parent_state;
        effect.timer = effect.timer.wrapping_sub(1);
        effect.pos += effect.velocity;
        effect.velocity += effect.accel;

        if effect.ty == VA_LARGE_SPARK || effect.ty == VA_SMALL_SPARK {
            let z = (play.rand.zero_one() * 0x4E20 as f32) as i16;
            effect.rot[2] = effect.rot[2].wrapping_add(z.wrapping_add(0x2000));
            let y = (play.rand.zero_one() * 0x2710 as f32) as i16;
            effect.rot[1] = effect.rot[1].wrapping_add(y.wrapping_add(0x2000));
            let mode = effect.mode as u8;
            if mode == SPARK_TETHER || mode == SPARK_UNUSED {
                let pitch = effect.rot[0].wrapping_sub(vec3f_pitch(r.world_pos, r.body_unk_1d8));
                effect.pos.x = r.world_pos.x - (effect.offset.x * sin_s(r.world_rot_y));
                effect.pos.z = r.world_pos.z - (effect.offset.x * cos_s(r.world_rot_y));
                effect.pos.y = (effect.offset.y * cos_s(pitch.wrapping_neg())) + r.world_pos.y;
            } else if mode == SPARK_BARI || mode == SPARK_BODY {
                effect.pos = effect.offset + r.world_pos;
            } else {
                let index = play.rand.zero_float(PLAYER_BODYPART_MAX as f32 - 0.1) as i16 as usize;
                let p = player_body_part(play, index);
                let x = p.x + play.rand.centered_float(10.0);
                let y = p.y + play.rand.centered_float(15.0);
                let z = p.z + play.rand.centered_float(10.0);
                effects[i].pos = Vec3::new(x, y, z);
            }
            fade(&mut effects[i]);
        }
        let effect = &mut effects[i];

        if effect.ty == VA_BLAST_SPARK {
            let z = (play.rand.zero_one() * 0x4E20 as f32) as i16;
            effect.rot[2] = effect.rot[2].wrapping_add(z.wrapping_add(0x4000));
            fade(effect);
        }

        if effect.ty == VA_SPARK_BALL {
            let z = (play.rand.zero_one() * 0x2710 as f32) as i16;
            effect.rot[2] = effect.rot[2].wrapping_add(z.wrapping_add(0x24A8));
            effect.pos.x = effect.offset.x + r.world_pos.x;
            effect.pos.y = r.world_pos.y + 310.0 + (r.y_offset * r.scale_y);
            effect.pos.z = effect.offset.z + r.world_pos.z;
            effect.mode = (effect.mode + 1) & 7;
            fade(effect);
        }

        if effect.ty == VA_ZAP_CHARGE {
            effect.mode = (effect.mode + 1) & 7;
            effect.prim_color[3] -= 20;
            if effect.prim_color[3] <= 0 {
                effect.prim_color[3] = 0;
                effect.timer = 0;
                effect.ty = VA_NONE;
            }
        }

        if effect.ty == VA_BLOOD {
            if effect.mode < BLOOD_SPOT {
                let floor = floor_under(play, &effects[i]);
                let effect = &mut effects[i];
                if let Some(floor_y) = floor
                    && effect.pos.y <= floor_y
                {
                    effect.mode = BLOOD_SPOT;
                    effect.pos.y = floor_y + 1.0;
                    effect.timer = if cs_state <= DEATH_SHELL_BURST { 80 } else { 60000 };
                    effect.accel = Vec3::ZERO;
                    effect.velocity = Vec3::ZERO;
                }
                if effect.timer == 0 {
                    effect.ty = VA_NONE;
                }
            } else if effect.timer < 20 {
                effect.env_color[3] = effect.timer as i16 * 5;
                effect.prim_color[3] = effect.timer as i16 * 10;
            } else if effect.timer > 50000 {
                effect.timer += 1;
            }
            let effect = &mut effects[i];
            if effect.timer == 0 {
                effect.ty = VA_NONE;
            }
        }
        let effect = &mut effects[i];

        if effect.ty == VA_GORE {
            if effect.mode == GORE_PERMANENT {
                effect.rot[0] = effect.rot[0].wrapping_add(0x1770);
                let floor = floor_under(play, &effects[i]);
                let effect = &mut effects[i];
                if let Some(floor_y) = floor
                    && effect.pos.y <= floor_y
                {
                    effect.mode = GORE_FLOOR;
                    effect.timer = 30;
                    effect.pos.y = floor_y + 1.0;
                    effect.accel = Vec3::ZERO;
                    effect.velocity = Vec3::ZERO;
                    effect.rot[0] = -0x4000;
                }
                if effect.timer == 0 {
                    effect.ty = VA_NONE;
                }
            } else if effect.mode == GORE_FADING {
                if effect.timer == 0 {
                    effect.ty = VA_NONE;
                }
            } else {
                smooth_step_to_f(&mut effect.scale_mod, 0.075, 1.0, 0.005, 0.0);
                smooth_step_to_f(&mut effect.offset.y, 0.0, 0.6, 0.005, 0.0013);
                if play.gameplay_frames % 4 == 0 {
                    smooth_step_to_s(&mut effect.prim_color[0], 95, 1, 1, 0);
                }
            }
            let effect = &mut effects[i];
            effect.offset.x += effect.offset.y;
        }
        let effect = &mut effects[i];

        if effect.ty == VA_TUMOR {
            effect.rot[2] = effect.rot[2].wrapping_add(0x157C);
            effect.env_color[3] = (sin_s(effect.rot[2]) * 50.0) as i16 + 80;
            let target = effect.scale_mod;
            smooth_step_to_f(&mut effect.scale, target, 1.0, 0.01, 0.005);
            effect.pos = effect.offset + r.world_pos;
            let mode = effect.mode as u8;
            let burst = match mode {
                TUMOR_UNUSED => effect.timer == 0,
                TUMOR_BODY | TUMOR_ARM => r.burst != 0,
                _ => false,
            };
            if burst {
                effect.ty = VA_NONE;
                let yaw = vec3f_yaw(r.world_pos, effect.pos);
                let (pos, scale) = (effect.pos, effect.scale);
                blood_splatter(play, effects, pos, yaw, (scale * 4500.0) as i32 as i16, 1, cs_state);
                gore(play, effects, pos, yaw, (scale * 1.2) as i32 as i16, cs_state);
            }
        }
    }
}

// The draws' bakes.
pub const BAKE_SPARK: &str = "Boss_Va/spark";
pub const BAKE_SMALL_SPARK: &str = "Boss_Va/small_spark";
pub const BAKE_BLOOD: &str = "Boss_Va/blood";
pub const BAKE_TUMOR: &str = "Boss_Va/tumor";
pub const BAKE_GORE: &str = "Boss_Va/gore";
pub const BAKE_ZAP_CHARGE: &str = "Boss_Va/zap_charge";
/// A spark ball's bake with `sSparkBallTex[i]` on segment 8.
pub fn spark_ball_bake(i: usize) -> String {
    format!("Boss_Va/spark_ball{i}")
}
/// `sSparkBallTex`.
pub const SPARK_BALL_TEX: [&str; 8] = [
    "gBarinadeSparkBall1Tex",
    "gBarinadeSparkBall2Tex",
    "gBarinadeSparkBall3Tex",
    "gBarinadeSparkBall4Tex",
    "gBarinadeSparkBall5Tex",
    "gBarinadeSparkBall6Tex",
    "gBarinadeSparkBall7Tex",
    "gBarinadeSparkBall8Tex",
];

/// The fixed env colour of the sparks' materials (segment 0x0D, run first) and the effects'
/// colours set per effect (0x0E, called between the material and the model).
const SEG_FIXED: u8 = 0x0D;
const SEG_COLOR: u8 = 0x0E;
const SEG_TEX: u8 = 0x08;
const SEG_BILLBOARD: u8 = 0x01;

/// `gDPSetEnvColor(r, g, b, a)` as a segment's commands.
fn env_commands(c: [u8; 4]) -> BakeSegment {
    BakeSegment::Commands(vec![(0xFB00_0000, u32::from_be_bytes(c)), (0xDF00_0000, 0)])
}

/// A material list, the colour segment, the model list.
fn dl3(material: &str, model: &str) -> BakeBody {
    BakeBody::DLists(vec![(OBJECT.into(), material.into()), (SEGMENT_CALL.into(), format!("{SEG_COLOR:x}")), (OBJECT.into(), model.into())])
}

/// `BossVa_DrawEffects`' meshes: each kind's material list (`gBarinadeDL_0156A0` and the rest),
/// the colours it sets per effect, its model list.
pub fn bakes() -> Vec<MeshBake> {
    let color = |env: bool, prim: bool| (SEG_COLOR, BakeSegment::DynamicColor { env, prim });
    let mut v: Vec<MeshBake> = vec![
        // Large and blast sparks: env (130, 130, 30, 0) before the material, the prim per spark.
        MeshBake {
            name: BAKE_SPARK.into(),
            object: OBJECT.into(),
            segments: vec![(SEG_FIXED, env_commands([130, 130, 30, 0])), color(false, true)],
            prelude: vec![SEG_FIXED],
            body: dl3("gBarinadeDL_0156A0", "gBarinadeDL_015710"),
        },
        // Small sparks: env (255, 255, 100, 0).
        MeshBake {
            name: BAKE_SMALL_SPARK.into(),
            object: OBJECT.into(),
            segments: vec![(SEG_FIXED, env_commands([255, 255, 100, 0])), color(false, true)],
            prelude: vec![SEG_FIXED],
            body: dl3("gBarinadeDL_008F08", "gBarinadeDL_008F70"),
        },
        // Blood: gEffBubble1Tex on segment 8.
        MeshBake {
            name: BAKE_BLOOD.into(),
            object: OBJECT.into(),
            segments: vec![(SEG_TEX, BakeSegment::Texture { file: "gameplay_keep".into(), symbol: "gEffBubble1Tex".into() }), color(true, true)],
            prelude: vec![],
            body: dl3("gBarinadeDL_009430", "gBarinadeDL_009468"),
        },
        // Tumours: the env colour set once before the material (the first tumour's alpha).
        MeshBake {
            name: BAKE_TUMOR.into(),
            object: OBJECT.into(),
            segments: vec![color(true, false)],
            prelude: vec![SEG_COLOR],
            body: BakeBody::DLists(vec![(OBJECT.into(), "gBarinadeDL_0128B8".into()), (OBJECT.into(), "gBarinadeDL_012948".into())]),
        },
        MeshBake { name: BAKE_GORE.into(), object: OBJECT.into(), segments: vec![color(true, true)], prelude: vec![], body: dl3("gBarinadeDL_012BA0", "gBarinadeDL_012C50") },
        MeshBake { name: BAKE_ZAP_CHARGE.into(), object: OBJECT.into(), segments: vec![color(true, true)], prelude: vec![], body: dl3("gBarinadeDL_0135B0", "gBarinadeDL_013638") },
    ];
    for (i, tex) in SPARK_BALL_TEX.iter().enumerate() {
        v.push(MeshBake {
            name: spark_ball_bake(i),
            object: OBJECT.into(),
            segments: vec![(SEG_TEX, BakeSegment::Texture { file: OBJECT.into(), symbol: (*tex).into() }), color(true, true)],
            prelude: vec![],
            body: dl3("gBarinadeDL_011738", "gBarinadeDL_011768"),
        });
    }
    // Segment 1 (`play->billboardMtx`, which the blood's list multiplies in): the identity, the
    // draw's matrix already facing the camera (`Matrix_ReplaceRotation`), as `En_Item00`'s do.
    for b in &mut v {
        b.segments.push((SEG_BILLBOARD, BakeSegment::Bytes(crate::en_item00::identity_mtx())));
    }
    v
}

fn c8(c: i16) -> u8 {
    c as u8
}

fn cmd(bake: &str, m: Mat4, env: Option<[u8; 4]>, prim: Option<[u8; 4]>) -> DrawCmd {
    let mut sv = SegmentValues::default();
    sv.env[SEG_COLOR as usize] = env;
    sv.prim[SEG_COLOR as usize] = prim;
    DrawCmd { mesh: MeshKey::named(keys::bake(bake)), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } }
}

/// `Matrix_RotateZ((rot / (f32)0x8000) * 3.1416f)`.
fn rot_rad(r: i16) -> f32 {
    (r as f32 / 32768.0) * 3.1416
}

/// `BossVa_DrawEffects`: each kind in its own pass, in the C's order (large sparks, spark balls,
/// blood, tumours, gore, zapper charges, blasts, small sparks). `cam_eye` is `sSubCamId`'s camera's
/// eye (the main camera's when it's `SUB_CAM_ID_DONE`). The blasts' `Gfx_SetupDL_25Xlu2` (in place
/// of `25Xlu`) isn't modelled: they share the large sparks' bake.
pub fn draw_effects(effects: &[BossVaEffect], view: &ViewInfo, cam_eye: Vec3, out: &mut DrawOut) {
    let bb = |pos: Vec3| oot_game::draw::replace_rotation(Mat4::from_translation(pos), view.billboard);
    for e in effects.iter().filter(|e| e.ty == VA_LARGE_SPARK) {
        let m = bb(e.pos) * Mat4::from_rotation_z(rot_rad(e.rot[2])) * Mat4::from_scale(Vec3::new(e.scale * 0.0185, e.scale * 0.0185, 1.0));
        out.xlu.push(cmd(BAKE_SPARK, m, None, Some([230, 230, 230, c8(e.prim_color[3])])));
    }
    for e in effects.iter().filter(|e| e.ty == VA_SPARK_BALL) {
        let m = bb(e.pos) * Mat4::from_scale(Vec3::splat(e.scale)) * Mat4::from_rotation_z(rot_rad(e.rot[2]));
        let p = e.prim_color.map(c8);
        let en = e.env_color.map(c8);
        out.xlu.push(cmd(&spark_ball_bake((e.mode & 7) as usize), m, Some(en), Some(p)));
    }
    for e in effects.iter().filter(|e| e.ty == VA_BLOOD) {
        let base = if e.mode == BLOOD_SPOT { Mat4::from_translation(e.pos) * Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2) } else { bb(e.pos) };
        let m = base * Mat4::from_scale(Vec3::new(e.scale, e.scale, 1.0));
        out.xlu.push(cmd(BAKE_BLOOD, m, Some([0, 100, 0, c8(e.env_color[3])]), Some([0, 150, 0, c8(e.prim_color[3])])));
    }
    // The tumours' env colour is set once, with the first one's alpha.
    let tumor_env = effects.iter().find(|e| e.ty == VA_TUMOR).map(|e| [0, 0, 0, c8(e.env_color[3])]);
    for e in effects.iter().filter(|e| e.ty == VA_TUMOR) {
        let shown = e.mode as u8 != TUMOR_BODY || (dist_xz(cam_eye, e.pos) - dist_xz(cam_eye, e.parent_state.world_pos)) < 10.0;
        if shown {
            let m = Mat4::from_translation(e.pos) * Mat4::from_scale(Vec3::splat(e.scale));
            out.opa.push(cmd(BAKE_TUMOR, m, tumor_env, None));
        }
    }
    for e in effects.iter().filter(|e| e.ty == VA_GORE) {
        let mut mf = MtxF::set_translate(e.pos.x, e.pos.y, e.pos.z);
        mf.rotate_zyx(e.rot[0], e.rot[1], 0);
        mf.scale(e.scale, e.scale, e.scale);
        let p = e.offset.x;
        mf.rotate_x(p * 0.115);
        mf.rotate_y(p * 0.13);
        mf.rotate_z(p * 0.1);
        mf.scale(1.0 - e.scale_mod, e.scale_mod + 1.0, 1.0 - e.scale_mod);
        mf.rotate_z(-(p * 0.1));
        mf.rotate_y(-(p * 0.13));
        mf.rotate_x(-(p * 0.115));
        let a = c8(e.prim_color[3]);
        out.opa.push(cmd(BAKE_GORE, mf.to_mat4(), Some([255, 255, 255, a]), Some([c8(e.prim_color[0]), c8(e.prim_color[1]), c8(e.prim_color[2]), a])));
    }
    for e in effects.iter().filter(|e| e.ty == VA_ZAP_CHARGE) {
        let mut mf = MtxF::set_translate(e.pos.x, e.pos.y, e.pos.z);
        mf.rotate_zyx(e.rot[0], e.rot[1], 0);
        mf.scale(e.scale, e.scale, e.scale);
        let a = c8(e.prim_color[3]);
        out.xlu.push(cmd(BAKE_ZAP_CHARGE, mf.to_mat4(), Some([255, 255, 50, a]), Some([255, 255, 255, a])));
    }
    for e in effects.iter().filter(|e| e.ty == VA_BLAST_SPARK) {
        let m = bb(e.pos) * Mat4::from_rotation_z(rot_rad(e.rot[2])) * Mat4::from_scale(Vec3::new(e.scale * 0.02, e.scale * 0.02, 1.0));
        out.xlu.push(cmd(BAKE_SPARK, m, None, Some([230, 230, 230, c8(e.prim_color[3])])));
    }
    for e in effects.iter().filter(|e| e.ty == VA_SMALL_SPARK) {
        let m = Mat4::from_translation(e.pos) * Mat4::from_rotation_z(rot_rad(e.rot[2])) * Mat4::from_rotation_y(rot_rad(e.rot[1])) * Mat4::from_scale(Vec3::new(e.scale, e.scale, 1.0));
        out.xlu.push(cmd(BAKE_SMALL_SPARK, m, None, Some([255, 255, 255, c8(e.prim_color[3])])));
    }
}
