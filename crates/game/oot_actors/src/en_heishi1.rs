//! `En_Heishi1` (`ovl_En_Heishi1/z_en_heishi1.c`): the guards' courtyard's patrols. By day
//! (before 17:17, or by `IS_DAY`, and until Zelda has fled) eight guards walk their paths' corners
//! (0 to 3) and, for types 2 and 3, the middles (4 to 7) in the order 0 4 1 5 2 6 3 7, waiting at
//! each, the head turning left and right. A guard looks only while the active camera shows his
//! screen (`sBgCamIndices`, by path): each frame he throws an invisible search ball 30 a frame
//! along his look (`Effect_Ss_Solder_Srch_Ball`); one that reaches Link within 70 before it
//! touches a wall sets his `linkDetected`, as does Link within 60 of him (70 unless falling).
//! Seen, and standing within 60 of the guard's height: "Hey you!" (0x702D, `NA_SE_SY_FOUND`),
//! Link held (`PLAYER_CSACTION_1`), the guard walks to him, turns, and when the text's event is
//! read Link is kicked out to `ENTR_HYRULE_CASTLE_3` (by Talon's crates) with a white starburst
//! circle wipe. One catch at a time: `sPlayerIsCaught`, the overlay's static. Type 5, the two by
//! the drain by night (after 17:17, by night, or once Zelda has fled), only stand and catch Link
//! within 100. Nothing moves while Player is frozen (`Camera_Special6`'s screen change).
//!
//! The whole overlay is ported. The guard's skeleton is `En_Heishi4`'s bake; its draw puts the
//! circle shadow down itself (`func_80033C30`, 0.3: `oot_game::shadow::func_80033c30`).
//!
//! Params: `type` bits 8 to 15 (0 to 3: a day patrol, its animation parameters
//! `sAnimParamsInit`, 2 and 3 using the middle points; 5: a night guard, whose parameters are read
//! past the table, `@bug (game)`); `path` bits 0 to 7 (the scene's path, 0 to 7 for its screens in
//! `sBgCamIndices`; 3 also spawns the courtyard's eight rupees, `En_Ex_Ruppy` type 3; 0xFF for the
//! night guards).
//! Flags: sets EVENTCHKINF_CAUGHT_BY_CASTLE_GUARDS when he kicks Link out (read by nothing); reads
//! EVENTCHKINF_ZELDA_FLED_CASTLE at init (day patrols gone, night guards there).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{approach_f, approach_zero_f, smooth_step_to_s};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind};
use oot_game::event_chk_inf;
use oot_game::env::clock_time;
use oot_game::interface::DO_ACTION_STOP;
use oot_game::message::TEXT_STATE_EVENT;
use oot_game::path::rad_to_binang_trunc;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::*;
use oot_game::sys_matrix::MtxF;

use crate::en_ex_ruppy::{ACTOR_EN_EX_RUPPY, ENEXRUPPY_TYPE_HYRULE_COURTYARD};
use crate::en_heishi3::{EVENTCHKINF_CAUGHT_BY_CASTLE_GUARDS, NA_SE_EV_KNIGHT_WALK, NA_SE_SY_FOUND, TRANS_TYPE_CAUGHT};
use crate::en_heishi4::{self, EnHeishi4};

/// `ACTOR_EN_HEISHI1` (`actor_table.h`: 0x008F).
pub const ACTOR_EN_HEISHI1: i16 = 0x008F;
pub const OBJECT: &str = "object_sd";

/// `En_Heishi1_Profile`: `FLAGS` `ACTOR_FLAG_UPDATE_CULLING_DISABLED`. Its id is 0, not
/// `ACTOR_EN_HEISHI1` (the decomp's `/**/ 0,`): the guards' `actor.id` is Player's.
pub const PROFILE: ActorProfile = ActorProfile { id: 0, name: "En_Heishi1", category: ACTORCAT_NPC, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `ENTR_HYRULE_CASTLE_3` (`entrance_table.h`: 0x4FA): by Talon's crates.
pub const ENTR_HYRULE_CASTLE_3: u16 = 0x4FA;
/// `EVENTCHKINF_ZELDA_FLED_CASTLE` (`save.h`: 0x80).
pub const EVENTCHKINF_ZELDA_FLED_CASTLE: u16 = 0x80;

/// `PARAMS_GET_U(this->actor.params, 0, 8)`: the path.
pub const PATH: Field = Field::params("path", 0, 8, Kind::PathIndex, "the path he patrols, 0 to 7 (sBgCamIndices: his screens); 3 spawns the rupees; 0xFF for a night guard");
/// `PARAMS_GET_U(this->actor.params, 8, 8)`: the type.
pub const TYPE: Field = Field::params(
    "type",
    8,
    8,
    Kind::Named(&[
        (0, "a day patrol, slow, the corners"),
        (1, "a day patrol, quick, the corners"),
        (2, "a day patrol, slow, the corners and middles"),
        (3, "a day patrol, quick, the corners and middles"),
        (5, "a night guard by the drain, standing"),
    ]),
    "his patrol (sAnimParamsInit), or a night guard",
);

/// En_Heishi1's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PATH, TYPE],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_CAUGHT_BY_CASTLE_GUARDS)).sets().note("Link caught (read by nothing)"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_ZELDA_FLED_CASTLE)).reads().note("day patrols gone, night guards there"),
    ],
    todo: false,
    objects: &[],
};

/// `PLAYER_CSACTION_1`: Link stands still.
const PLAYER_CSACTION_1: u8 = 1;

/// `sAnimParamsInit`: by type, `animSpeed`, `animMorphFrames`, `moveSpeedTarget`,
/// `moveSpeedMax`, `bodyTurnSpeedTarget`, `bodyTurnSpeedMax`, `headTurnSpeedScale`,
/// `headTurnSpeedMax`. `@bug (game)`: type 5 reads `sAnimParamsInit[5]`, past the table's four
/// rows: the overlay's data after it (`sBaseHeadTimers`, then `sRupeePositions` from its sixth
/// float), rows 4 and 5 here. The night guards never walk or wait, so nothing but the init's
/// prints reads them.
const S_ANIM_PARAMS_INIT: [[f32; 8]; 6] = [
    [1.0, -10.0, 3.0, 0.5, 1000.0, 200.0, 0.3, 1000.0],
    [3.0, -3.0, 6.0, 0.8, 2000.0, 400.0, 0.5, 2000.0],
    [1.0, -10.0, 3.0, 0.5, 1000.0, 200.0, 0.3, 1000.0],
    [3.0, -3.0, 6.0, 0.8, 2000.0, 400.0, 0.5, 2000.0],
    // sBaseHeadTimers' twelve bytes and sRupeePositions' first five floats, as floats (not read).
    [0.0; 8],
    // sRupeePositions' floats 5 to 12.
    [90.0, -55.0, 0.0, 30.0, -55.0, 0.0, -30.0, 0.0],
];

/// `sBaseHeadTimers`.
const S_BASE_HEAD_TIMERS: [i16; 6] = [20, 10, 20, 10, 13, 0];

/// `sRupeePositions`: round the path 3 guard's home.
const S_RUPEE_POSITIONS: [Vec3; 8] = [
    Vec3::new(0.0, 0.0, 90.0),
    Vec3::new(-55.0, 0.0, 90.0),
    Vec3::new(-55.0, 0.0, 30.0),
    Vec3::new(-55.0, 0.0, -30.0),
    Vec3::new(0.0, 0.0, -30.0),
    Vec3::new(55.0, 0.0, -30.0),
    Vec3::new(55.0, 0.0, 30.0),
    Vec3::new(55.0, 0.0, 90.0),
];

/// `sBgCamIndices`: by path, the two bg cameras (the courtyard's screens) a guard looks on.
const S_BG_CAM_INDICES: [i16; 16] = [7, 7, 2, 2, 2, 2, 3, 3, 4, 4, 5, 6, 4, 4, 5, 6];

/// The two bg cameras (`sBgCamIndices`) a guard on `path` looks on.
pub fn bg_cam_indices(path: i16) -> [i16; 2] {
    let i = (path * 2) as usize;
    [S_BG_CAM_INDICES.get(i).copied().unwrap_or(-1), S_BG_CAM_INDICES.get(i + 1).copied().unwrap_or(-1)]
}

/// `sWaypoints`: corners 0 to 3, middles 4 to 7, in the order the middle-walkers go.
const S_WAYPOINTS: [i16; 8] = [0, 4, 1, 5, 2, 6, 3, 7];

/// The overlay's statics: `sPlayerIsCaught`.
#[derive(Debug, Default)]
pub struct Statics {
    pub player_is_caught: bool,
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    SetupWait,
    SetupWalk,
    SetupMoveToLink,
    SetupTurnTowardLink,
    SetupKick,
    SetupWaitNight,
    Wait,
    Walk,
    MoveToLink,
    TurnTowardLink,
    Kick,
    WaitNight,
}

pub struct EnHeishi1 {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub action: Action,
    pub active_timer: i16,
    pub head_direction: i16,
    pub head_behavior_decided: bool,
    pub link_detected: bool,
    pub load_started: bool,
    pub ty: i16,
    pub path: i16,
    pub waypoint_timer: i16,
    pub move_speed: f32,
    pub body_turn_speed: f32,
    pub head_angle: f32,
    pub head_angle_target: f32,
    /// `animParams`: `animSpeed`, `animMorphFrames`, `moveSpeedTarget`, `moveSpeedMax`,
    /// `bodyTurnSpeedTarget`, `bodyTurnSpeedMax`, `headTurnSpeedScale`, `headTurnSpeedMax`.
    pub anim_params: [f32; 8],
    /// `timers`: `headTimer`, `waitTimer`, `kickTimer`.
    pub timers: [i16; 3],
    pub waypoint: i16,
    idle: Anim,
    walk: Anim,
}

const HEAD_TIMER: usize = 0;
const WAIT_TIMER: usize = 1;
const KICK_TIMER: usize = 2;

/// `BINANG_TO_RAD_ALT` of a float sum (`shape.rot.y + headAngle`, not wrapped to 16 bits):
/// `((f32)(binang) / (f32)0x8000) * M_PI`, the product in double precision.
fn binang_to_rad_alt(b: f32) -> f32 {
    ((b / 32768.0) as f64 * std::f64::consts::PI) as f32
}

impl EnHeishi1 {
    /// `EnHeishi1_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = (|| -> anyhow::Result<_> { Ok((assets.skeleton(OBJECT, "gEnHeishiSkel")?, assets.animation(OBJECT, "gEnHeishiIdleAnim")?, assets.animation(OBJECT, "gEnHeishiWalkAnim")?)) })();
        let (skeleton, idle, walk) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Heishi1: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let ty = TYPE.get(actor.params);
        let path = PATH.get(actor.params);
        let mut this = EnHeishi1 {
            // SkelAnime_Init(.., gEnHeishiSkel, gEnHeishiIdleAnim, .., 17).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(idle.clone())),
            skeleton,
            action: Action::SetupWalk,
            active_timer: 0,
            head_direction: 0,
            head_behavior_decided: false,
            link_detected: false,
            load_started: false,
            ty,
            path,
            waypoint_timer: 0,
            move_speed: 0.0,
            body_turn_speed: 0.0,
            head_angle: 0.0,
            head_angle_target: 0.0,
            anim_params: S_ANIM_PARAMS_INIT[ty as usize % S_ANIM_PARAMS_INIT.len()],
            timers: [0; 3],
            waypoint: 0,
            idle,
            walk,
            actor,
        };
        this.actor.scale = Vec3::splat(0.01);
        log::debug!(" type ☆☆☆☆☆☆☆☆☆☆☆☆☆ {}, rail data ☆☆☆☆☆☆☆☆ {}, anime_frame_speed ☆☆☆☆☆☆ {}", ty, path, this.anim_params[0]);
        if this.path == 3 {
            for p in S_RUPEE_POSITIONS {
                let _ = play.actor_spawn_as_child(&mut this.actor, ACTOR_EN_EX_RUPPY, p, [0, 0, 0], ENEXRUPPY_TYPE_HYRULE_COURTYARD);
            }
        }
        let day_time = play.save.day_time as i32;
        let is_day = play.save.is_day();
        let fled = play.save.get_event_chk_inf(EVENTCHKINF_ZELDA_FLED_CASTLE);
        if this.ty != 5 {
            if (day_time < clock_time(17, 18) - 1 || is_day) && !fled {
                this.action = Action::SetupWalk;
            } else {
                this.actor.kill();
            }
        } else if day_time > clock_time(17, 18) - 1 || !is_day || fled {
            this.action = Action::SetupWaitNight;
        } else {
            this.actor.kill();
        }
        Box::new(this)
    }

    fn anim_speed(&self) -> f32 {
        self.anim_params[0]
    }
    fn anim_morph_frames(&self) -> f32 {
        self.anim_params[1]
    }
    fn move_speed_target(&self) -> f32 {
        self.anim_params[2]
    }
    fn move_speed_max(&self) -> f32 {
        self.anim_params[3]
    }
    fn body_turn_speed_target(&self) -> f32 {
        self.anim_params[4]
    }
    fn body_turn_speed_max(&self) -> f32 {
        self.anim_params[5]
    }
    fn head_turn_speed_scale(&self) -> f32 {
        self.anim_params[6]
    }
    fn head_turn_speed_max(&self) -> f32 {
        self.anim_params[7]
    }

    /// `Animation_Change(&skelAnime, anim, speed, 0, (s16)lastFrame, LOOP, morph)`.
    fn change_loop(&mut self, anim: Anim, speed: f32, morph: f32) {
        let frames = anim.last_frame() as i16 as f32;
        self.skel.change(anim, speed, 0.0, frames, ANIMMODE_LOOP, morph);
    }

    /// `EnHeishi1_SetupWalk`.
    fn setup_walk(&mut self, play: &mut PlayState) {
        self.change_loop(self.walk.clone(), self.anim_speed(), self.anim_morph_frames());
        self.body_turn_speed = 0.0;
        self.move_speed = 0.0;
        self.head_direction = play.rand.zero_float(1.99) as i16;
        self.action = Action::Walk;
    }

    /// `EnHeishi1_Walk`: towards the waypoint, the body turning to it and the head left and right;
    /// within 20 of it, a middle-walker may skip on (half the time), else it waits.
    fn walk(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.skel.on_frame(1.0) || self.skel.on_frame(17.0) {
            audio_play_actor_sfx2(play, NA_SE_EV_KNIGHT_WALK);
        }
        if play.overlay_static::<Statics>(ACTOR_EN_HEISHI1).player_is_caught {
            return;
        }
        let Some(point) = play.setup_path_list().get(self.path as usize).map(|p| p.point(self.waypoint as usize)) else {
            log::error!("En_Heishi1: no path {}", self.path);
            return;
        };
        approach_f(&mut self.actor.world_pos.x, point.x, 1.0, self.move_speed);
        approach_f(&mut self.actor.world_pos.z, point.z, 1.0, self.move_speed);
        let (target, max) = (self.move_speed_target(), self.move_speed_max());
        approach_f(&mut self.move_speed, target, 1.0, max);
        let path_diff_x = point.x - self.actor.world_pos.x;
        let path_diff_z = point.z - self.actor.world_pos.z;
        smooth_step_to_s(&mut self.actor.shape_rot.y, rad_to_binang_trunc(oot_game::camera::f_atan2f(path_diff_x, path_diff_z)), 3, self.body_turn_speed as i32 as i16, 0);
        let (target, max) = (self.body_turn_speed_target(), self.body_turn_speed_max());
        approach_f(&mut self.body_turn_speed, target, 1.0, max);
        if self.timers[HEAD_TIMER] == 0 {
            self.head_direction = self.head_direction.wrapping_add(1);
            self.head_angle_target = 0x2000 as f32;
            // An odd headDirection faces 45 degrees left.
            if self.head_direction & 1 != 0 {
                self.head_angle_target *= -1.0;
            }
            let rand_offset = play.rand.zero_float(30.0) as i16;
            self.timers[HEAD_TIMER] = S_BASE_HEAD_TIMERS[self.ty as usize % S_BASE_HEAD_TIMERS.len()].wrapping_add(rand_offset);
        }
        let (scale, max) = (self.head_turn_speed_scale(), self.head_turn_speed_max());
        approach_f(&mut self.head_angle, self.head_angle_target, scale, max);
        // Within 20 of a waypoint: a middle one may be skipped.
        if path_diff_x.abs() < 20.0 && path_diff_z.abs() < 20.0 && self.waypoint_timer == 0 {
            if self.ty >= 2 && self.waypoint >= 4 && play.rand.zero_float(1.99) > 1.0 {
                if self.waypoint == 7 {
                    self.waypoint = 0;
                }
                if self.waypoint >= 4 {
                    self.waypoint -= 3;
                }
                self.waypoint_timer = 5;
                return;
            }
            self.action = Action::SetupWait;
        }
    }

    /// `EnHeishi1_SetupMoveToLink`: "Hey you!".
    fn setup_move_to_link(&mut self, play: &mut PlayState) {
        self.change_loop(self.walk.clone(), 3.0, -3.0);
        self.body_turn_speed = 0.0;
        self.move_speed = 0.0;
        play.start_textbox(0x702D, play.cur_actor);
        play.interface_ctx.set_do_action(DO_ACTION_STOP);
        self.action = Action::MoveToLink;
    }

    /// `EnHeishi1_MoveToLink`: up to 6 a frame to Link, until within 70.
    fn move_to_link(&mut self, play: &mut PlayState) {
        let player_pos = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(self.actor.world_pos);
        self.skel.update();
        approach_f(&mut self.actor.world_pos.x, player_pos.x, 1.0, self.move_speed);
        approach_f(&mut self.actor.world_pos.z, player_pos.z, 1.0, self.move_speed);
        approach_f(&mut self.move_speed, 6.0, 1.0, 0.4);
        smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 3, self.body_turn_speed as i32 as i16, 0);
        approach_f(&mut self.body_turn_speed, 3000.0, 1.0, 300.0);
        approach_zero_f(&mut self.head_angle, 0.5, 2000.0);
        if self.actor.xz_dist_to_player < 70.0 {
            self.action = Action::SetupTurnTowardLink;
        }
    }

    /// `EnHeishi1_SetupWait`.
    fn setup_wait(&mut self, play: &mut PlayState) {
        self.change_loop(self.idle.clone(), self.anim_speed(), self.anim_morph_frames());
        self.head_behavior_decided = false;
        self.head_direction = play.rand.zero_float(1.99) as i16;
        let rand = play.rand.zero_float(50.0) as i16;
        self.timers[WAIT_TIMER] = rand.wrapping_add(50);
        self.action = Action::Wait;
    }

    /// `EnHeishi1_Wait`: the head turned left and right (52 degrees); when the wait's done, on to
    /// the next waypoint.
    fn wait(&mut self, play: &mut PlayState) {
        self.skel.update();
        if play.overlay_static::<Statics>(ACTOR_EN_HEISHI1).player_is_caught {
            return;
        }
        if !self.head_behavior_decided {
            self.head_direction = self.head_direction.wrapping_add(1);
            // An odd headDirection faces 52 degrees left.
            self.head_angle_target = if self.head_direction & 1 != 0 { 0x2500 as f32 } else { -0x2500 as f32 };
            let rand_offset = play.rand.zero_float(30.0) as i16;
            self.timers[HEAD_TIMER] = S_BASE_HEAD_TIMERS[self.ty as usize % S_BASE_HEAD_TIMERS.len()].wrapping_add(rand_offset);
            self.head_behavior_decided = true;
        } else if self.timers[HEAD_TIMER] == 0 {
            if self.timers[WAIT_TIMER] == 0 {
                if self.ty == 0 || self.ty == 1 {
                    self.waypoint += 1;
                    if self.waypoint >= 4 {
                        self.waypoint = 0;
                    }
                } else {
                    // The corners are 0 to 3 and the middles 4 to 7: the next in "04152637".
                    if let Some(i) = S_WAYPOINTS.iter().position(|&w| w == self.waypoint) {
                        self.waypoint = S_WAYPOINTS[(i + 1) % S_WAYPOINTS.len()];
                    }
                    self.waypoint_timer = 5;
                }
                self.action = Action::SetupWalk;
            } else {
                self.head_behavior_decided = false;
            }
        }
        let (scale, max) = (self.head_turn_speed_scale(), self.head_turn_speed_max());
        approach_f(&mut self.head_angle, self.head_angle_target, scale, max + max);
    }

    /// `EnHeishi1_SetupTurnTowardLink`.
    fn setup_turn_toward_link(&mut self) {
        self.change_loop(self.idle.clone(), 1.0, -10.0);
        self.timers[KICK_TIMER] = 30;
        self.action = Action::TurnTowardLink;
    }

    /// `EnHeishi1_TurnTowardLink`: 30 frames turning to Link (the night guards don't turn).
    fn turn_toward_link(&mut self) {
        self.skel.update();
        if self.ty != 5 {
            smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 3, self.body_turn_speed as i32 as i16, 0);
            approach_f(&mut self.body_turn_speed, 3000.0, 1.0, 300.0);
            approach_zero_f(&mut self.head_angle, 0.5, 2000.0);
        }
        if self.timers[KICK_TIMER] == 0 {
            self.action = Action::SetupKick;
        }
    }

    /// `EnHeishi1_SetupKick`.
    fn setup_kick(&mut self) {
        self.change_loop(self.idle.clone(), 1.0, -10.0);
        self.action = Action::Kick;
    }

    /// `EnHeishi1_Kick`: on the text's event, Link out to `ENTR_HYRULE_CASTLE_3`.
    fn kick(&mut self, play: &mut PlayState) {
        self.skel.update();
        if !self.load_started && play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            play.with_msg(|m, f| m.close_textbox(f.audio));
            if !self.load_started {
                play.save.set_event_chk_inf(EVENTCHKINF_CAUGHT_BY_CASTLE_GUARDS);
                play.transition.next_entrance_index = ENTR_HYRULE_CASTLE_3;
                play.transition.trigger = oot_game::transition::TRANS_TRIGGER_START;
                self.load_started = true;
                play.overlay_static::<Statics>(ACTOR_EN_HEISHI1).player_is_caught = false;
                play.transition.ty = TRANS_TYPE_CAUGHT;
                play.save.next_transition_type = TRANS_TYPE_CAUGHT;
            }
        }
    }

    /// `EnHeishi1_SetupWaitNight`.
    fn setup_wait_night(&mut self) {
        self.change_loop(self.idle.clone(), 1.0, -10.0);
        self.action = Action::WaitNight;
    }

    /// `EnHeishi1_WaitNight`: Link within 100 is caught.
    fn wait_night(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.actor.xz_dist_to_player < 100.0 {
            play.start_textbox(0x702D, play.cur_actor);
            play.audio.play_sfx_centered(NA_SE_SY_FOUND);
            log::debug!("☆☆☆☆☆ Discovered! ☆☆☆☆☆");
            play.player_set_cs_action_with_halted_actors(play.cur_actor, PLAYER_CSACTION_1);
            self.action = Action::SetupKick;
        }
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            Action::SetupWait => self.setup_wait(play),
            Action::SetupWalk => self.setup_walk(play),
            Action::SetupMoveToLink => self.setup_move_to_link(play),
            Action::SetupTurnTowardLink => self.setup_turn_toward_link(),
            Action::SetupKick => self.setup_kick(),
            Action::SetupWaitNight => self.setup_wait_night(),
            Action::Wait => self.wait(play),
            Action::Walk => self.walk(play),
            Action::MoveToLink => self.move_to_link(play),
            Action::TurnTowardLink => self.turn_toward_link(),
            Action::Kick => self.kick(play),
            Action::WaitNight => self.wait_night(play),
        }
    }
}

impl ActorImpl for EnHeishi1 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnHeishi1_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.active_timer = self.active_timer.wrapping_add(1);
        for t in &mut self.timers {
            if *t != 0 {
                *t -= 1;
            }
        }
        if self.waypoint_timer != 0 {
            self.waypoint_timer -= 1;
        }
        let active_bg_cam_index = play.active_camera().bg_cam_index;
        let (player_freeze, player_pos, player_velocity_y) = play.player.and_then(|h| play.actors.actor(h)).map(|a| (a.freeze_timer, a.world_pos, a.velocity.y)).unwrap_or((0, Vec3::ZERO, 0.0));
        if player_freeze != 0 {
            return;
        }
        self.run_action(play);
        self.actor.culling_volume_distance = 550.0;
        self.actor.culling_volume_scale = 350.0;
        self.actor.culling_volume_downward = 700.0;
        if self.ty == 5 {
            return;
        }
        let path = (self.path * 2) as usize;
        let on_screen = S_BG_CAM_INDICES.get(path).is_some_and(|&c| c == active_bg_cam_index) || S_BG_CAM_INDICES.get(path + 1).is_some_and(|&c| c == active_bg_cam_index);
        if !on_screen || play.overlay_static::<Statics>(ACTOR_EN_HEISHI1).player_is_caught || !matches!(self.action, Action::Walk | Action::Wait) {
            return;
        }
        let search_ball_pos = self.actor.world_pos + Vec3::new(0.0, 60.0, 0.0);
        // Matrix_RotateY(BINANG_TO_RAD_ALT(shape.rot.y + headAngle), MTXMODE_NEW), then
        // Matrix_MultVec3f of (0, 0, 30).
        let mut m = MtxF::IDENTITY;
        m.rotate_y(binang_to_rad_alt(self.actor.shape_rot.y as f32 + self.head_angle));
        let search_ball_vel = m.mult_vec3f(Vec3::new(0.0, 0.0, 30.0));
        let me = play.cur_actor;
        play.with_ss(|s| s.solder_srch_ball_spawn(search_ball_pos, search_ball_vel, Vec3::ZERO, 2, me));
        if self.actor.xz_dist_to_player < 60.0 {
            self.link_detected = true;
        } else if self.actor.xz_dist_to_player < 70.0 {
            // Probably for Link landing from a jump a little further off.
            if player_velocity_y > -4.0 {
                self.link_detected = true;
            }
        }
        if self.link_detected {
            // @bug (game): meant to catch Link only on the ground, but a backflip or a fall from the
            // upper path is below -4 before it lands, and there's a frame landing from a sidehop
            // where another sidehop keeps him above -3.9: frame-perfect sidehops get through.
            if !(player_velocity_y > -3.9) {
                self.link_detected = false;
                // Not from the upper path.
                if (player_pos.y - self.actor.world_pos.y).abs() < 60.0 {
                    play.audio.play_sfx_centered(NA_SE_SY_FOUND);
                    log::debug!("☆☆☆☆☆ Discovered! ☆☆☆☆☆");
                    play.player_set_cs_action_with_halted_actors(me, PLAYER_CSACTION_1);
                    play.overlay_static::<Statics>(ACTOR_EN_HEISHI1).player_is_caught = true;
                    self.action = Action::SetupMoveToLink;
                }
            }
        }
    }
    fn solder_srch_ball_detect(&mut self) {
        self.link_detected = true;
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.angles = vec![self.head_angle as i32 as i16];
        rs
    }
    /// `EnHeishi1_Draw`: `EnHeishi1_OverrideLimbDraw` adds `(s16)headAngle` to the head's (limb 16)
    /// x, `En_Heishi4`'s head yaw place; then the circle shadow (`func_80033C30`, 0.3).
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        let head = rs.angles.first().copied().unwrap_or(0);
        let bones = EnHeishi4::pose(&self.skeleton, &joints.rot, [0, head, 0], [0, 0, 0]);
        out.opa.push(DrawCmd { mesh: MeshKey::named(oot_game::pack::keys::bake(en_heishi4::BAKE)), transform: oot_game::play::actor_draw_matrix(rs), bones, params: Default::default() });
        out.opa.push(oot_game::shadow::func_80033c30(&play.col, rs.pos, Vec3::splat(0.3), 0xFF));
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
