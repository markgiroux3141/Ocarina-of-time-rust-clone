//! `Obj_Lift` (`ovl_Obj_Lift/z_obj_lift.c`): the square platform that collapses under Link, a
//! DynaPoly actor of `object_d_lift` (`gCollapsingPlatformCol`, `DYNA_TRANSFORM_POS`: it carries
//! him) drawn with `gCollapsingPlatformDL`.
//!
//! Params: bit 1 the size (0: scale 0.1, falling 18 a check; 1: 0.05, 9), bits 2..7 the switch
//! flag set once it's broken (a broken one isn't spawned again), bits 8..10 the frames Link
//! stands on it before it shakes (`sFallTimerDurations`: 0 to 60; 7 falls without shaking).
//!
//! Stood on (`DynaPolyActor_IsPlayerOnTop`) for its wait, it shakes 20 frames with a quake
//! (`QUAKE_TYPE_1`) and `NA_SE_EV_BLOCK_SHAKE`, then falls (`Actor_MoveXZGravity`, gravity
//! -0.6 down to -15) until the floor under it is within its fall distance
//! (`BgCheck_EntityRaycastDown4`): it breaks into nine pieces (`EffectSsKakera`) and dust
//! (`func_80033480`), `NA_SE_EV_BOX_BREAK`, its flag set, gone.
//!
//! The Master Quest Deku Tree's is in room 2, params 0x0080 at (-1214, 390, 1208): size 0,
//! flag 0x20, no wait. The whole overlay is ported; the cull zone isn't, for any actor.

use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource, DYNA_INTERACT_PLAYER_ON_TOP, DYNA_TRANSFORM_POS};
use eng_math::{cos_s, sin_s};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile, func_80033480};
use oot_game::camera::CAM_ID_NONE;
use oot_game::effect::kakera::KAKERA_COLOR_NONE;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::quake::QUAKE_TYPE_1;

/// `ACTOR_OBJ_LIFT` (`actor_table.h`: 0x012C).
pub const ACTOR_OBJ_LIFT: i16 = 0x012C;
pub const OBJECT: &str = "object_d_lift";
/// `OBJECT_D_LIFT` (`object_table.h`: 0x011D).
pub const OBJECT_D_LIFT: i16 = 0x011D;
const COLLISION: &str = "gCollapsingPlatformCol";

/// `NA_SE_EV_BLOCK_SHAKE`, `NA_SE_EV_BOX_BREAK` (`environmentbank_table.h`: 0x2838, 0x2839).
pub const NA_SE_EV_BLOCK_SHAKE: u16 = 0x2838;
pub const NA_SE_EV_BOX_BREAK: u16 = 0x2839;

/// `Obj_Lift_Profile`: `ACTOR_FLAG_UPDATE_CULLING_DISABLED`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_OBJ_LIFT, name: "Obj_Lift", category: ACTORCAT_BG, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT };

/// `sFallTimerDurations`.
pub const FALL_TIMER_DURATIONS: [i16; 7] = [0, 10, 20, 30, 40, 50, 60];

/// `sFragmentScales`: the nine pieces' places, in model units across x and z.
pub const FRAGMENT_SCALES: [(i16, i16); 9] = [(120, -120), (120, 0), (120, 120), (0, -120), (0, 0), (0, 120), (-120, -120), (-120, 0), (-120, 120)];

/// `sScales`, `sMaxFallDistances`, by size.
pub const SCALES: [f32; 2] = [0.1, 0.05];
pub const MAX_FALL_DISTANCES: [f32; 2] = [-18.0, -9.0];

/// `sFallTimerDurations[PARAMS_GET_U(params, 8, 3)]`. `@bug (game)`: 7 reads past the table, the
/// next static's first value (`sFragmentScales[0].x`, 120); 7 skips the shake and the wait is
/// reset to that while Link is off.
pub fn fall_timer_duration(params: i16) -> i16 {
    let i = ((params as u16 >> 8) & 7) as usize;
    FALL_TIMER_DURATIONS.get(i).copied().unwrap_or(FRAGMENT_SCALES[0].0)
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `ObjLift_Wait`.
    Wait,
    /// `ObjLift_Shake`.
    Shake,
    /// `ObjLift_Fall`.
    Fall,
}

pub struct ObjLift {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub action: Action,
    /// `shakeOrientation`: the shake's three phases.
    pub shake_orientation: [i16; 3],
    pub timer: i16,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

impl ObjLift {
    /// `PARAMS_GET_U(params, 1, 1)`: the size.
    fn size(&self) -> usize {
        ((self.actor.params as u16 >> 1) & 1) as usize
    }

    /// `PARAMS_GET_U(params, 2, 6)`: the switch flag.
    pub fn switch_flag(&self) -> i32 {
        ((self.actor.params as u16 >> 2) & 0x3F) as i32
    }

    /// `ObjLift_Init`: the collision (`ObjLift_InitDynaPoly`, `DYNA_TRANSFORM_POS`); gone if its
    /// flag is set; the size's scale, gravity -0.6 down to -15 (`sInitChain`), the shake's
    /// phases from `Rand` (x, y, z), the wait.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // ObjLift_InitDynaPoly(DYNA_TRANSFORM_POS): the bg actor reads the actor's transform at
        // the next DynaPoly_UpdateContext (the source is set again below).
        let bg = match crate::obj_kibako2::load_collision(play, OBJECT, COLLISION) {
            Some(h) => play.col.dyna.set_bg_actor(h, source(&actor), DYNA_TRANSFORM_POS),
            None => BG_ACTOR_MAX,
        };
        if bg == BG_ACTOR_MAX {
            log::warn!("Warning : move BG registration failed (../z_obj_lift.c 188)(name {})(arg_data 0x{:04x})", actor.id, actor.params);
        }
        let mut l = ObjLift { actor, bg, action: Action::Wait, shake_orientation: [0; 3], timer: 0 };
        if play.flags.get_switch(l.switch_flag()) {
            l.actor.kill();
            return Box::new(l);
        }
        // Actor_SetScale(sScales[size]).
        l.actor.scale = Vec3::splat(SCALES[l.size()]);
        // sInitChain: gravity -0.6, minVelocityY -15; cullingVolumeDistance 2000,
        // cullingVolumeScale 500, cullingVolumeDownward 2000 (the cull zone isn't ported).
        l.actor.gravity = -0.6;
        l.actor.min_velocity_y = -15.0;
        for i in 0..3 {
            l.shake_orientation[i] = (play.rand.zero_one() * 65535.5) as i32 as i16;
        }
        l.setup_wait();
        log::debug!("(Dungeon Lift)(arg_data 0x{:04x})", l.actor.params);
        play.col.dyna.set_source(l.bg, source(&l.actor));
        Box::new(l)
    }

    /// `ObjLift_SetupWait`.
    fn setup_wait(&mut self) {
        self.timer = fall_timer_duration(self.actor.params);
        self.action = Action::Wait;
    }

    /// `ObjLift_Wait`: with Link on it and its wait over, it falls (type 7) or shakes with a
    /// quake on the active camera (speed 10000, 2 up and down, 20 frames); off it, the wait
    /// starts again.
    fn wait(&mut self, play: &mut PlayState) {
        if play.col.dyna.interact_flag(self.bg, DYNA_INTERACT_PLAYER_ON_TOP) {
            if self.timer <= 0 {
                if (self.actor.params as u16 >> 8) & 7 == 7 {
                    self.setup_fall();
                } else {
                    let quake_index = play.quake_request(CAM_ID_NONE, QUAKE_TYPE_1);
                    play.quake_set_speed(quake_index, 10000);
                    play.quake_set_perturbations(quake_index, 2, 0, 0, 0);
                    play.quake_set_duration(quake_index, 20);
                    self.setup_shake();
                }
            }
        } else {
            self.timer = fall_timer_duration(self.actor.params);
        }
    }

    /// `ObjLift_SetupShake`: 20 frames.
    fn setup_shake(&mut self) {
        self.timer = 20;
        self.action = Action::Shake;
    }

    /// `ObjLift_Shake`: tilting 300 each way about x and z on one phase (10000 a frame), bobbing
    /// 1 on another and circling 3 across on the third (18000 a frame each); then the fall.
    /// `NA_SE_EV_BLOCK_SHAKE` (16 frames at its position) when the timer's low bits are 3.
    fn shake(&mut self, play: &mut PlayState) {
        if self.timer <= 0 {
            self.setup_fall();
        } else {
            let a = &mut self.actor;
            let so = &mut self.shake_orientation;
            so[0] = so[0].wrapping_add(10000);
            a.world_rot.x = ((sin_s(so[0]) * 300.0) as i32 as i16).wrapping_add(a.home_rot.x);
            a.world_rot.z = ((cos_s(so[0]) * 300.0) as i32 as i16).wrapping_add(a.home_rot.z);
            a.shape_rot.x = a.world_rot.x;
            a.shape_rot.z = a.world_rot.z;
            so[1] = so[1].wrapping_add(18000);
            a.world_pos.y = sin_s(so[1]) + a.home_pos.y;
            so[2] = so[2].wrapping_add(18000);
            a.world_pos.x = sin_s(so[2]) * 3.0 + a.home_pos.x;
            a.world_pos.z = cos_s(so[2]) * 3.0 + a.home_pos.z;
        }
        if self.timer & 3 == 3 {
            play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, 16, NA_SE_EV_BLOCK_SHAKE);
        }
    }

    /// `ObjLift_SetupFall`: back at home, unturned.
    fn setup_fall(&mut self) {
        self.action = Action::Fall;
        self.actor.world_pos = self.actor.home_pos;
        self.actor.world_rot = self.actor.home_rot;
        self.actor.shape_rot = self.actor.home_rot;
    }

    /// `ObjLift_Fall`: `Actor_MoveXZGravity`; the floor under where it was, its fall distance
    /// lower (`BgCheck_EntityRaycastDown4`, its own collision skipped); once that's within the
    /// distance of it, it breaks.
    fn fall(&mut self, play: &mut PlayState) {
        self.actor.move_forward();
        let mut pos = self.actor.prev_pos;
        pos.y += MAX_FALL_DISTANCES[self.size()];
        let (floor, poly) = play.col.entity_raycast_down_actor(pos, self.bg);
        self.actor.floor_height = floor;
        self.actor.floor_poly = poly;
        if (self.actor.floor_height - self.actor.world_pos.y) >= (MAX_FALL_DISTANCES[self.size()] - 0.001) {
            self.spawn_fragments(play);
            play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, 20, NA_SE_EV_BOX_BREAK);
            play.flags.set_switch(self.switch_flag());
            self.actor.kill();
        }
    }

    /// `ObjLift_SpawnFragments`: nine pieces from its places (`sFragmentScales`, scaled),
    /// flying out at 0.8 of that and up at 6 to 16 (tumbling fast or medium, half and half,
    /// 50 to 100 big times the scale), pulled back towards the platform's position
    /// (`vec`); then dust (size 0: 13 puffs 120 across, 120 big; size 1: 9, 60, 60).
    pub fn spawn_fragments(&mut self, play: &mut PlayState) {
        let temp_s3 = self.actor.world_pos;
        let scale = self.actor.scale;
        play.with_ss(|ss| {
            for &(fx, fz) in &FRAGMENT_SCALES {
                let pos = Vec3::new(fx as f32 * scale.x + temp_s3.x, temp_s3.y, fz as f32 * scale.z + temp_s3.z);
                let vy = ss.rand.zero_one() * 10.0 + 6.0;
                let velocity = Vec3::new(fx as f32 * scale.x * 0.8, vy, fz as f32 * scale.z * 0.8);
                // The argument list's two Rand_ZeroOne calls, left to right (as IDO evaluates
                // them): the tumble, then the scale.
                let tumble = if ss.rand.zero_one() < 0.5 { 64 } else { 32 };
                let s = ((ss.rand.zero_one() * 50.0 + 50.0) * scale.x) as i16;
                ss.kakera_spawn(pos, velocity, temp_s3, -256, tumble, 15, 15, 0, s, 0, 32, 50, KAKERA_COLOR_NONE, OBJECT_D_LIFT, Some((OBJECT, "gCollapsingPlatformDL")));
            }
        });
        match self.size() {
            0 => func_80033480(play, self.actor.world_pos, 120.0, 12, 120, 100, 1),
            _ => func_80033480(play, self.actor.world_pos, 60.0, 8, 60, 100, 1),
        }
    }
}

impl ActorImpl for ObjLift {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `ObjLift_Update`: the timer down, then the action; the new transform for
    /// `DynaPoly_UpdateContext`.
    fn update(&mut self, play: &mut PlayState) {
        if self.timer > 0 {
            self.timer -= 1;
        }
        match self.action {
            Action::Wait => self.wait(play),
            Action::Shake => self.shake(play),
            Action::Fall => self.fall(play),
        }
        play.col.dyna.set_source(self.bg, source(&self.actor));
    }

    /// `ObjLift_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }

    /// `DynaPolyActor_IsPlayerOnTop` reads the interact flags `Actor_UpdateAll` clears after it.
    fn dyna_bg_id(&self) -> Option<u16> {
        Some(self.bg)
    }

    /// `ObjLift_Draw`: `Gfx_DrawDListOpa(gCollapsingPlatformDL)`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        crate::gfx_draw_dlist_opa(out, OBJECT, "gCollapsingPlatformDL", rs);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
