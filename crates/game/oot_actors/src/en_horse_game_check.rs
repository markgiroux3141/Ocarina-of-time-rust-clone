//! `En_Horse_Game_Check` (`ovl_En_Horse_Game_Check/z_en_horse_game_check.c`): the horse games'
//! referee, an invisible actor (`EnHorseGameCheck_Draw` draws nothing; never culled) running one
//! of four games by its params (`EnHorseGameCheckType`), each an init, an update and a destroy
//! (`sInitFuncs`, `sUpdateFuncs`, `sDestroyFuncs`; the destroys do nothing). On Lon Lon Ranch with
//! Epona had any of them is Malon's obstacle course.
//!
//! - **1 Ingo's race** (`HORSEGAME_INGO_RACE`, Lon Lon Ranch's cutscene layer 4 before Epona):
//!   its init spawns Ingo's horse (`En_Horse` 0x8003, the rental horse with Ingo riding, at (-250,
//!   1, -1650) facing 0x4000). Its update: the timer (`Interface_SetTimer(0)`, counting up) once
//!   `startTimer` is past 50, Link's horse off (`inRace`) past 80, Ingo's past 81 with the start
//!   shot (`NA_SE_SY_START_SHOT`). Three checkpoints in turn ((1700, 1, -540), (117, 1, 488),
//!   (-1700, 1, -540), within 400) for each; after the third, the finish line (x -200 to 80, z
//!   -2300 to -1470): Link's horse there first wins (55 frames on), Ingo's (`ENHORSE_INGO_WON`, 70
//!   on), with the goal's music (`NA_BGM_HORSE_GOAL`) and the shot. Link at the ranch's way out
//!   (x 800 to 1000, z -2900 to -2700), on horseback or not, is Ingo's win (20 on); the timer at
//!   180 seconds is time up (20 on). Then (`EnHorseGameCheck_FinishIngoRace`): won, to
//!   `ENTR_LON_LON_RANCH_7` with Ingo's music forced (`NA_BGM_INGO`): the second race (for Epona,
//!   `EVENTINF_INGO_RACE_SECOND_RACE`) to `INGO_RACE_STATE_TRAPPED_WIN_EPONA` in a white fade, the
//!   first to `INGO_RACE_STATE_FIRST_WIN` in a white starburst; else to `ENTR_LON_LON_RANCH_8`
//!   with `INGO_RACE_STATE_PLAYER_LOSE` in a black circle. `EVENTINF_INGO_RACE_0F` set, the timer
//!   off.
//! - **2 the Gerudo's horseback archery** (`HORSEGAME_GERUDO_ARCHERY`, the Gerudo's Fortress's
//!   layers 4 and 5): with Link on horseback, his horse's `hbaStarted` once `startTimer` is past 90
//!   (the horse's archery itself is Phase 18's, logged by `En_Horse`).
//! - **3** (`HORSEGAME_TYPE3`): nothing (none placed).
//! - **4 Malon's obstacle course** (`HORSEGAME_MALON_RACE`): the same start as Ingo's race but for
//!   Ingo (the timer past 50, Link's horse past 80, the shot past 81), and Link's start: on the
//!   finish line, then off it. The course's eight fences (`sFencePos`, within 250 across) jumped
//!   (`ENHORSE_JUMPING`) in turn on two laps (`fenceCheck[0..16]`); one out of turn is a foul
//!   (0x208C, done 30 frames on). Back at the line after the start: the first lap's eight fences
//!   jumped begin the second (0x208D); all sixteen, the course run (the goal's music, the shot, the
//!   timer frozen, done 70 frames on); the line crossed without them, or from the wrong side
//!   (`prevPos.x` past 80; on the second lap under -200), a foul. The timer at 180 seconds: time
//!   up (the timer to 240 and off, done 30 on). Then (`EnHorseGameCheck_FinishMalonRace`):
//!   `ENTR_LON_LON_RANCH_7` in a white starburst (a foul freezes the timer at 240 first; no result
//!   goes to `ENTR_LON_LON_RANCH_0`, "not supported").
//!
//! The whole overlay is ported. `DREG(25) = 0` (a debug register nothing reads) is left out.
//!
//! `@bug (game)`: the obstacle course reads Link's horse without checking for one (the finish
//! line's first test and every fence's distance: a null read in the C; here, with no horse,
//! nothing is at the line or near a fence). The first fence's foul check reads `fenceCheck[-1]`,
//! which is `raceFlags` (the struct's member before it): kept, so the first fence is never a foul
//! once a flag is set (the start sets two before any jump).
//!
//! Params: `params` whole, the game (`EnHorseGameCheckType`): 0 none, 1 Ingo's race (placed on Lon
//! Lon Ranch's layer 4), 2 the Gerudo's horseback archery (placed on the Gerudo's Fortress's
//! layers 4 and 5), 3 nothing, 4 Malon's obstacle course (the ranch's placement once Epona is had:
//! the init writes it).
//! Flags: reads EVENTCHKINF_EPONA_OBTAINED (the ranch's game is Malon's) and the debug
//! `R_DEBUG_FORCE_EPONA_OBTAINED` (0); reads EVENTINF_INGO_RACE_SECOND_RACE (Ingo's race won: which
//! state); sets `eventInf[EVENTINF_INDEX_HORSES]`'s race state and EVENTINF_INGO_RACE_0F as Ingo's
//! race ends.

use eng_collision::math3d::vec3f_dist_xyz;
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_BG, ActorHandle, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, TextLine};
use oot_game::audio::bgm::start_seq;
use oot_game::audio::sfx::NA_SE_SY_START_SHOT;
use oot_game::audio::{NA_BGM_HORSE_GOAL, NA_BGM_INGO, SEQ_PLAYER_BGM_MAIN};
use oot_game::horse::*;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::save::{TIMER_STATE_OFF, TIMER_STATE_UP_FREEZE};
use oot_game::transition::{TRANS_TRIGGER_START, TRANS_TYPE_FADE_WHITE};
use oot_game::{event_chk_inf, event_inf};

use crate::en_horse::{ENHORSE_INGO_WON, ENHORSE_JUMPING, EnHorse};

/// `ACTOR_EN_HORSE_GAME_CHECK` (`actor_table.h`: 0x00DB).
pub const ACTOR_EN_HORSE_GAME_CHECK: i16 = 0x00DB;
/// `OBJECT_GAMEPLAY_KEEP`.
pub const OBJECT: &str = "gameplay_keep";

/// `En_Horse_Game_Check_Profile`: `ACTORCAT_BG`, `ACTOR_FLAG_UPDATE_CULLING_DISABLED`.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_EN_HORSE_GAME_CHECK, name: "En_Horse_Game_Check", category: ACTORCAT_BG, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

// `EnHorseGameCheckType` (`z_en_horse_game_check.h`).
pub const HORSEGAME_NONE: i32 = 0;
pub const HORSEGAME_INGO_RACE: i32 = 1;
pub const HORSEGAME_GERUDO_ARCHERY: i32 = 2;
pub const HORSEGAME_TYPE3: i32 = 3;
pub const HORSEGAME_MALON_RACE: i32 = 4;

/// `this->actor.params`: the game (`EnHorseGameCheckType`).
pub const TYPE: Field = Field::whole(
    "params",
    Kind::Named(&[
        (HORSEGAME_NONE, "none: nothing"),
        (HORSEGAME_INGO_RACE, "Ingo's race (Lon Lon Ranch before Epona; Malon's obstacle course after)"),
        (HORSEGAME_GERUDO_ARCHERY, "the Gerudo's horseback archery: Link's horse's hbaStarted after 90 frames"),
        (HORSEGAME_TYPE3, "nothing"),
        (HORSEGAME_MALON_RACE, "Malon's obstacle course (any placement on Lon Lon Ranch once Epona is had)"),
    ]),
    "which horse game it referees (EnHorseGameCheckType)",
);

/// Malon's texts: a foul, the second lap.
pub const TEXT_FOUL: u16 = 0x208C;
pub const TEXT_SECOND_LAP: u16 = 0x208D;

/// En_Horse_Game_Check's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_EPONA_OBTAINED)).reads().note("on Lon Lon Ranch: the game is Malon's obstacle course"),
        FlagUse::new(Flag::Other("R_DEBUG_FORCE_EPONA_OBTAINED")).reads().note("the debug register DREG(1): Epona's as obtained (0)"),
        FlagUse::new(event_inf!(EVENTINF_INGO_RACE_SECOND_RACE)).reads().note("Ingo's race won: the race for Epona traps Link"),
        FlagUse::new(Flag::Other("eventInf[EVENTINF_INDEX_HORSES]'s race state (EVENTINF_INGO_RACE_STATE_MASK)"))
            .sets()
            .note("Ingo's race's end: INGO_RACE_STATE_TRAPPED_WIN_EPONA, _FIRST_WIN or _PLAYER_LOSE"),
        FlagUse::new(event_inf!(EVENTINF_INGO_RACE_0F)).sets().note("Ingo's race's end (WRITE_EVENTINF_INGO_RACE_0F(1))"),
    ],
    todo: false,
    objects: &[],
    texts: &[
        TextLine::new(0x208C).when("params", &[HORSEGAME_MALON_RACE]).cond("a fence jumped out of turn, or the line crossed without the lap's fences").note("Malon: you broke the rules"),
        TextLine::new(0x208D).when("params", &[HORSEGAME_MALON_RACE]).cond("the line crossed after the first lap's eight fences").note("Malon: the second lap"),
    ],
};

// `INGO_RACE_*` (`z_en_horse_game_check.c`): the race's start flags.
pub const INGO_RACE_PLAYER_MOVE: i32 = 1 << 0;
pub const INGO_RACE_SET_TIMER: i32 = 1 << 1;
pub const INGO_RACE_INGO_MOVE: i32 = 1 << 2;

// `HorseGameIngoRaceResult`.
pub const INGO_RACE_NO_RESULT: i32 = 0;
pub const INGO_RACE_PLAYER_WIN: i32 = 1;
pub const INGO_RACE_INGO_WIN: i32 = 2;
pub const INGO_RACE_TIME_UP: i32 = 3;

// `MALONRACE_*`: the course's flags.
pub const MALONRACE_PLAYER_MOVE: i32 = 1 << 0;
pub const MALONRACE_SET_TIMER: i32 = 1 << 1;
pub const MALONRACE_SECOND_LAP: i32 = 1 << 2;
pub const MALONRACE_BROKE_RULE: i32 = 1 << 3;
pub const MALONRACE_START_SFX: i32 = 1 << 4;
pub const MALONRACE_PLAYER_START: i32 = 1 << 5;
pub const MALONRACE_PLAYER_ON_MARK: i32 = 1 << 6;

// `HorseGameMalonRaceResult`.
pub const MALONRACE_NO_RESULT: i32 = 0;
pub const MALONRACE_SUCCESS: i32 = 1;
pub const MALONRACE_TIME_UP: i32 = 2;
pub const MALONRACE_UNUSED: i32 = 3;
pub const MALONRACE_FAILURE: i32 = 4;

/// `CS_INDEX_NONE` (`save.h`: 0x0000).
const CS_INDEX_NONE: u16 = 0;
/// `ACTOR_EN_HORSE` (`actor_table.h`: 0x0014) params 0x8003: the rental horse (`HORSE_HNI`, bit
/// 15) with Ingo riding (`HORSE_PTYPE_INGO_SPAWNED_RIDING`).
pub const INGO_HORSE_PARAMS: i16 = 0x8003u16 as i16;

/// `sIngoRaceCheckpoints`.
pub const INGO_RACE_CHECKPOINTS: [Vec3; 3] = [Vec3::new(1700.0, 1.0, -540.0), Vec3::new(117.0, 1.0, 488.0), Vec3::new(-1700.0, 1.0, -540.0)];
/// `sFinishLine`: x -200 to 80, z -2300 to -1470.
pub const FINISH_LINE: [f32; 4] = [-200.0, 80.0, -2300.0, -1470.0];
/// `sRanchExit`: x 800 to 1000, z -2900 to -2700.
pub const RANCH_EXIT: [f32; 4] = [800.0, 1000.0, -2900.0, -2700.0];
/// `sFencePos`: the obstacle course's eight fences.
pub const FENCE_POS: [Vec3; 8] = [
    Vec3::new(820.0, -44.0, -1655.0),
    Vec3::new(1497.0, -21.0, -1198.0),
    Vec3::new(1655.0, -44.0, -396.0),
    Vec3::new(1291.0, -44.0, 205.0),
    Vec3::new(379.0, -21.0, 455.0),
    Vec3::new(-95.0, -21.0, 455.0),
    Vec3::new(-939.0, 1.0, 455.0),
    Vec3::new(-1644.0, -21.0, -1035.0),
];

/// `Math3D_PointInSquare2D`.
fn point_in_square_2d(upper_left_x: f32, lower_right_x: f32, upper_left_y: f32, lower_right_y: f32, x: f32, y: f32) -> bool {
    x >= upper_left_x && x <= lower_right_x && y >= upper_left_y && y <= lower_right_y
}

/// `AT_FINISH_LINE(actor)`.
pub fn at_finish_line(pos: Vec3) -> bool {
    point_in_square_2d(FINISH_LINE[0], FINISH_LINE[1], FINISH_LINE[2], FINISH_LINE[3], pos.x, pos.z)
}

/// `AT_RANCH_EXIT(actor)`.
pub fn at_ranch_exit(pos: Vec3) -> bool {
    point_in_square_2d(RANCH_EXIT[0], RANCH_EXIT[1], RANCH_EXIT[2], RANCH_EXIT[3], pos.x, pos.z)
}

/// `Math_Vec3f_DistXZ`.
fn dist_xz(a: Vec3, b: Vec3) -> f32 {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    (dx * dx + dz * dz).sqrt()
}

/// Player's `rideActor`.
fn ride_actor(play: &PlayState) -> Option<ActorHandle> {
    play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).and_then(|p| p.ride_actor())
}

/// `EnHorseGameCheckIngoRace`'s members.
#[derive(Debug, Clone, Default)]
pub struct IngoRace {
    pub start_flags: i32,
    pub player_check: [i32; 3],
    pub ingo_check: [i32; 3],
    /// `ingoHorse`.
    pub ingo_horse: Option<ActorHandle>,
    pub start_timer: i32,
    pub result: i32,
    pub finish_timer: i32,
    pub player_finish: i32,
    pub ingo_finish: i32,
}

/// `EnHorseGameCheckGerudoArchery`'s members.
#[derive(Debug, Clone, Default)]
pub struct GerudoArchery {
    pub unk_150: i32,
    pub start_timer: u32,
}

/// `EnHorseGameCheck3`'s members.
#[derive(Debug, Clone, Default)]
pub struct Type3 {
    pub unk_150: i32,
}

/// `EnHorseGameCheckMalonRace`'s members.
#[derive(Debug, Clone, Default)]
pub struct MalonRace {
    pub start_timer: i32,
    pub race_flags: i32,
    pub fence_check: [i32; 16],
    pub finish_timer: i32,
    pub result: i32,
    pub lap_count: i32,
}

impl MalonRace {
    /// `this->fenceCheck[i]` for `i` from -1: `fenceCheck[-1]` is `raceFlags`, the member before
    /// the array (`@bug (game)`: the first fence's foul check reads it).
    fn fence_check_at(&self, i: i32) -> i32 {
        if i < 0 { self.race_flags } else { self.fence_check[i as usize] }
    }
}

/// `EnHorseGameCheck` (the union of the games' structs: only `type`'s is used).
pub struct EnHorseGameCheck {
    pub actor: Actor,
    /// `base.type`: the game its init set up (0 none).
    pub ty: i32,
    pub ingo: IngoRace,
    pub gerudo: GerudoArchery,
    pub type3: Type3,
    pub malon: MalonRace,
}

impl EnHorseGameCheck {
    /// `EnHorseGameCheck_Init`: on Lon Lon Ranch with Epona had, the game is Malon's obstacle
    /// course; then the game's init (`sInitFuncs`).
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = EnHorseGameCheck { actor, ty: HORSEGAME_NONE, ingo: IngoRace::default(), gerudo: GerudoArchery::default(), type3: Type3::default(), malon: MalonRace::default() };
        if play.scene_id == SCENE_LON_LON_RANCH && (play.save.get_event_chk_inf(EVENTCHKINF_EPONA_OBTAINED) || R_DEBUG_FORCE_EPONA_OBTAINED != 0) {
            this.actor.params = HORSEGAME_MALON_RACE as i16;
        }
        match TYPE.value(this.actor.params) {
            HORSEGAME_INGO_RACE => this.init_ingo_race(play),
            HORSEGAME_GERUDO_ARCHERY => this.init_gerudo_archery(),
            HORSEGAME_TYPE3 => this.init_type3(),
            HORSEGAME_MALON_RACE => this.init_malon_race(),
            HORSEGAME_NONE => {}
            // sInitFuncs[params] past its end: the C reads beyond the table (none placed).
            t => log::warn!("En_Horse_Game_Check: params {t} isn't a game"),
        }
        Box::new(this)
    }

    /// `EnHorseGameCheck_InitIngoRace`: Ingo's horse spawned (`En_Horse` 0x8003 at (-250, 1, -1650)
    /// facing 0x4000); the counts and result cleared.
    fn init_ingo_race(&mut self, play: &mut PlayState) {
        self.ty = HORSEGAME_INGO_RACE;
        self.ingo.start_flags = 0;
        for i in 0..3 {
            self.ingo.player_check[i] = 0;
        }
        match play.actor_spawn(ACTOR_EN_HORSE, Vec3::new(-250.0, 1.0, -1650.0), [0, 0x4000, 0], INGO_HORSE_PARAMS) {
            Ok(h) => self.ingo.ingo_horse = Some(h),
            // LogUtils_HungupThread: the C stops the game.
            Err(e) => log::error!("En_Horse_Game_Check: Ingo's horse didn't spawn ({e:?}); the C hangs here"),
        }
        self.ingo.start_timer = 0;
        self.ingo.finish_timer = 0;
        self.ingo.result = INGO_RACE_NO_RESULT;
        self.ingo.player_finish = 0;
        self.ingo.ingo_finish = 0;
    }

    /// `EnHorseGameCheck_FinishIngoRace`: the race's state by the result, and the transition out.
    fn finish_ingo_race(&mut self, play: &mut PlayState) {
        play.save.cutscene_index = CS_INDEX_NONE;
        if self.ingo.result == INGO_RACE_PLAYER_WIN {
            play.transition.next_entrance_index = ENTR_LON_LON_RANCH_7;
            if play.save.get_event_inf(EVENTINF_INGO_RACE_SECOND_RACE) {
                set_ingo_race_state(&mut play.save, INGO_RACE_STATE_TRAPPED_WIN_EPONA);
                // WRITE_EVENTINF_INGO_RACE_0F(1).
                play.save.set_event_inf(EVENTINF_INGO_RACE_0F);
                play.transition.ty = TRANS_TYPE_FADE_WHITE;
                play.environment_force_play_sequence(NA_BGM_INGO);
            } else {
                set_ingo_race_state(&mut play.save, INGO_RACE_STATE_FIRST_WIN);
                // WRITE_EVENTINF_INGO_RACE_0F(1).
                play.save.set_event_inf(EVENTINF_INGO_RACE_0F);
                play.environment_force_play_sequence(NA_BGM_INGO);
                play.transition.ty = trans_type_circle(TCA_STARBURST, TCC_WHITE, TCS_FAST);
            }
        } else {
            play.transition.next_entrance_index = ENTR_LON_LON_RANCH_8;
            set_ingo_race_state(&mut play.save, INGO_RACE_STATE_PLAYER_LOSE);
            play.transition.ty = trans_type_circle(TCA_NORMAL, TCC_BLACK, TCS_FAST);
            // WRITE_EVENTINF_INGO_RACE_0F(1).
            play.save.set_event_inf(EVENTINF_INGO_RACE_0F);
        }
        // DREG(25) = 0: a debug register nothing reads.
        play.transition.trigger = TRANS_TRIGGER_START;
        play.save.timer_state = TIMER_STATE_OFF;
    }

    /// `EnHorseGameCheck_UpdateIngoRace`.
    fn update_ingo_race(&mut self, play: &mut PlayState) {
        let ride = ride_actor(play);
        let r = &mut self.ingo;
        if r.start_timer > 50 && r.start_flags & INGO_RACE_SET_TIMER == 0 {
            r.start_flags |= INGO_RACE_SET_TIMER;
            play.interface_set_timer(0);
        } else if r.start_timer > 80 && ride.is_some() && r.start_flags & INGO_RACE_PLAYER_MOVE == 0 {
            r.start_flags |= INGO_RACE_PLAYER_MOVE;
            if let Some(horse) = ride.and_then(|h| play.actors.downcast_mut::<EnHorse>(h)) {
                horse.in_race = true;
            }
        } else if r.start_timer > 81 && r.start_flags & INGO_RACE_INGO_MOVE == 0 {
            if let Some(ingo_horse) = r.ingo_horse.and_then(|h| play.actors.downcast_mut::<EnHorse>(h)) {
                ingo_horse.in_race = true;
            }
            r.start_flags |= INGO_RACE_INGO_MOVE;
            play.audio.play_sfx_centered(NA_SE_SY_START_SHOT);
        }

        r.start_timer += 1;

        let ride_pos = ride.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos);
        let ingo_pos = r.ingo_horse.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos);
        for i in 0..3 {
            if let Some(p) = ride_pos
                && vec3f_dist_xyz(INGO_RACE_CHECKPOINTS[i], p) < 400.0
            {
                if i > 0 && r.player_check[i - 1] == 1 {
                    r.player_check[i] = 1;
                } else if i == 0 {
                    r.player_check[i] = 1;
                }
            }
            if let Some(p) = ingo_pos
                && vec3f_dist_xyz(INGO_RACE_CHECKPOINTS[i], p) < 400.0
            {
                if i > 0 && r.ingo_check[i - 1] == 1 {
                    r.ingo_check[i] = 1;
                } else if i == 0 {
                    r.ingo_check[i] = 1;
                }
            }
        }

        if r.result == INGO_RACE_NO_RESULT {
            if let Some(p) = ride_pos
                && r.player_check[2] == 1
                && at_finish_line(p)
            {
                r.player_finish += 1;
                if r.player_finish > 0 {
                    r.result = INGO_RACE_PLAYER_WIN;
                    r.finish_timer = 55;
                    play.audio.queue_seq_cmd(start_seq(SEQ_PLAYER_BGM_MAIN, 0, NA_BGM_HORSE_GOAL));
                    play.audio.play_sfx_centered(NA_SE_SY_START_SHOT);
                }
                for i in 0..3 {
                    r.player_check[i] = 0;
                }
            }
            if let Some(p) = ingo_pos
                && r.ingo_check[2] == 1
                && at_finish_line(p)
            {
                r.ingo_finish += 1;
                if r.ingo_finish > 0 {
                    r.result = INGO_RACE_INGO_WIN;
                    r.finish_timer = 70;
                    if let Some(ingo_horse) = r.ingo_horse.and_then(|h| play.actors.downcast_mut::<EnHorse>(h)) {
                        ingo_horse.state_flags |= ENHORSE_INGO_WON;
                    }
                    play.audio.queue_seq_cmd(start_seq(SEQ_PLAYER_BGM_MAIN, 0, NA_BGM_HORSE_GOAL));
                    play.audio.play_sfx_centered(NA_SE_SY_START_SHOT);
                }
                for i in 0..3 {
                    r.ingo_check[i] = 0;
                }
            }
            let player_pos = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos);
            if ride_pos.is_some_and(at_ranch_exit) || player_pos.is_some_and(at_ranch_exit) {
                play.audio.queue_seq_cmd(start_seq(SEQ_PLAYER_BGM_MAIN, 0, NA_BGM_HORSE_GOAL));
                r.result = INGO_RACE_INGO_WIN;
                r.finish_timer = 20;
            }
            if play.save.timer_seconds >= 180 && r.start_flags & INGO_RACE_SET_TIMER != 0 {
                play.audio.queue_seq_cmd(start_seq(SEQ_PLAYER_BGM_MAIN, 0, NA_BGM_HORSE_GOAL));
                r.result = INGO_RACE_TIME_UP;
                r.finish_timer = 20;
            }
        } else if r.finish_timer > 0 {
            r.finish_timer -= 1;
        } else {
            self.finish_ingo_race(play);
        }
    }

    /// `EnHorseGameCheck_InitGerudoArchery`.
    fn init_gerudo_archery(&mut self) {
        self.ty = HORSEGAME_GERUDO_ARCHERY;
        self.gerudo.unk_150 = 0;
        self.gerudo.start_timer = 0;
    }

    /// `EnHorseGameCheck_UpdateGerudoArchery`: with Link on horseback, the horse's `hbaStarted`
    /// once past 90 frames.
    fn update_gerudo_archery(&mut self, play: &mut PlayState) {
        let Some(horse) = ride_actor(play).and_then(|h| play.actors.downcast_mut::<EnHorse>(h)) else { return };
        if self.gerudo.start_timer > 90 {
            horse.hba_started = 1;
        }
        self.gerudo.start_timer += 1;
    }

    /// `EnHorseGameCheck_InitType3`.
    fn init_type3(&mut self) {
        self.ty = HORSEGAME_TYPE3;
        self.type3.unk_150 = 0;
    }

    /// `EnHorseGameCheck_InitMalonRace`.
    fn init_malon_race(&mut self) {
        self.ty = HORSEGAME_MALON_RACE;
        self.malon.race_flags = 0;
        self.malon.finish_timer = 0;
        self.malon.result = MALONRACE_NO_RESULT;
        for i in 0..16 {
            self.malon.fence_check[i] = 0;
        }
        self.malon.lap_count = 0;
    }

    /// `EnHorseGameCheck_FinishMalonRace`: back to the ranch (`ENTR_LON_LON_RANCH_7`) in a white
    /// starburst; a foul freezes the timer at 240 first.
    fn finish_malon_race(&mut self, play: &mut PlayState) {
        let r = &self.malon;
        if r.result == MALONRACE_SUCCESS || r.result == MALONRACE_TIME_UP {
            play.save.cutscene_index = CS_INDEX_NONE;
            play.transition.next_entrance_index = ENTR_LON_LON_RANCH_7;
            play.transition.ty = trans_type_circle(TCA_STARBURST, TCC_WHITE, TCS_FAST);
            play.transition.trigger = TRANS_TRIGGER_START;
        } else if r.result == MALONRACE_FAILURE {
            play.save.timer_seconds = 240;
            play.save.timer_state = TIMER_STATE_UP_FREEZE;
            play.save.cutscene_index = CS_INDEX_NONE;
            play.transition.next_entrance_index = ENTR_LON_LON_RANCH_7;
            play.transition.ty = trans_type_circle(TCA_STARBURST, TCC_WHITE, TCS_FAST);
            play.transition.trigger = TRANS_TRIGGER_START;
        } else {
            log::debug!("En_HGC_Spot20_Ta_end(): not supported");
            play.save.cutscene_index = CS_INDEX_NONE;
            play.transition.next_entrance_index = ENTR_LON_LON_RANCH_0;
            play.transition.ty = trans_type_circle(TCA_STARBURST, TCC_WHITE, TCS_FAST);
            play.transition.trigger = TRANS_TRIGGER_START;
        }
    }

    /// `EnHorseGameCheck_UpdateMalonRace`.
    fn update_malon_race(&mut self, play: &mut PlayState) {
        let ride = ride_actor(play);
        // @bug (game): AT_FINISH_LINE(player->rideActor) and the fences' distances read Link's horse
        // without checking for one; here, without one, nothing is at the line or near a fence.
        let ride_actor = ride.and_then(|h| play.actors.actor(h)).map(|a| (a.world_pos, a.prev_pos));
        let ride_at_line = ride_actor.is_some_and(|(p, _)| at_finish_line(p));
        let r = &mut self.malon;
        if r.race_flags & MALONRACE_PLAYER_ON_MARK == 0 && ride_at_line {
            r.race_flags |= MALONRACE_PLAYER_ON_MARK;
        } else if r.race_flags & MALONRACE_PLAYER_ON_MARK != 0 && r.race_flags & MALONRACE_PLAYER_START == 0 && !ride_at_line {
            r.race_flags |= MALONRACE_PLAYER_START;
        }
        if r.start_timer > 50 && r.race_flags & MALONRACE_SET_TIMER == 0 {
            r.race_flags |= MALONRACE_SET_TIMER;
            play.interface_set_timer(0);
        } else if r.start_timer > 80 && ride.is_some() && r.race_flags & MALONRACE_PLAYER_MOVE == 0 {
            r.race_flags |= MALONRACE_PLAYER_MOVE;
            if let Some(horse) = ride.and_then(|h| play.actors.downcast_mut::<EnHorse>(h)) {
                horse.in_race = true;
            }
        } else if r.start_timer > 81 && r.race_flags & MALONRACE_START_SFX == 0 {
            r.race_flags |= MALONRACE_START_SFX;
            play.audio.play_sfx_centered(NA_SE_SY_START_SHOT);
        }

        r.start_timer += 1;
        if r.result == MALONRACE_NO_RESULT {
            let jumping = ride.and_then(|h| play.actors.downcast::<EnHorse>(h)).is_some_and(|h| h.state_flags & ENHORSE_JUMPING != 0);
            for i in 0..16i32 {
                if r.lap_count == 0 && i >= 8 {
                    break;
                }
                let Some((pos, _)) = ride_actor else { continue };
                let dist = dist_xz(FENCE_POS[(i % 8) as usize], pos);
                if dist < 250.0 && jumping {
                    if i > 0 && r.fence_check_at(i - 1) == 1 {
                        r.fence_check[i as usize] = 1;
                    } else if i == 0 {
                        r.fence_check[i as usize] = 1;
                    }

                    if r.fence_check_at(i - 1) == 0 && r.race_flags & MALONRACE_BROKE_RULE == 0 {
                        r.race_flags |= MALONRACE_BROKE_RULE;
                        play.start_textbox(TEXT_FOUL, None);
                        r.result = MALONRACE_FAILURE;
                        r.finish_timer = 30;
                    }
                }
            }
            if let Some((pos, prev_pos)) = ride_actor
                && r.race_flags & MALONRACE_PLAYER_START != 0
                && at_finish_line(pos)
            {
                if r.lap_count == 1 && r.fence_check[15] == 0 && prev_pos.x < -200.0 {
                    r.race_flags |= MALONRACE_BROKE_RULE;
                    play.start_textbox(TEXT_FOUL, None);
                    r.result = MALONRACE_FAILURE;
                    r.finish_timer = 30;
                } else if r.fence_check[15] == 1 {
                    r.lap_count = 2;
                    play.audio.queue_seq_cmd(start_seq(SEQ_PLAYER_BGM_MAIN, 0, NA_BGM_HORSE_GOAL));
                    play.audio.play_sfx_centered(NA_SE_SY_START_SHOT);
                    r.result = MALONRACE_SUCCESS;
                    r.finish_timer = 70;
                    play.save.timer_state = TIMER_STATE_UP_FREEZE;
                } else if r.fence_check[7] == 1 && r.race_flags & MALONRACE_SECOND_LAP == 0 {
                    r.lap_count = 1;
                    r.race_flags |= MALONRACE_SECOND_LAP;
                    play.start_textbox(TEXT_SECOND_LAP, None);
                } else if r.fence_check[7] == 0 {
                    r.race_flags |= MALONRACE_BROKE_RULE;
                    play.start_textbox(TEXT_FOUL, None);
                    r.result = MALONRACE_FAILURE;
                    r.finish_timer = 30;
                } else if prev_pos.x > 80.0 {
                    r.race_flags |= MALONRACE_BROKE_RULE;
                    play.start_textbox(TEXT_FOUL, None);
                    r.result = MALONRACE_FAILURE;
                    r.finish_timer = 30;
                }
            }
            if play.save.timer_seconds >= 180 && r.race_flags & MALONRACE_SET_TIMER != 0 {
                play.save.timer_seconds = 240;
                r.result = MALONRACE_TIME_UP;
                r.finish_timer = 30;
                play.save.timer_state = TIMER_STATE_OFF;
            }
        } else if r.finish_timer > 0 {
            r.finish_timer -= 1;
        } else {
            self.finish_malon_race(play);
        }
    }
}

impl ActorImpl for EnHorseGameCheck {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnHorseGameCheck_Update`: the game's update (`sUpdateFuncs[type]`).
    fn update(&mut self, play: &mut PlayState) {
        match self.ty {
            HORSEGAME_INGO_RACE => self.update_ingo_race(play),
            HORSEGAME_GERUDO_ARCHERY => self.update_gerudo_archery(play),
            // EnHorseGameCheck_UpdateType3: nothing.
            HORSEGAME_TYPE3 => {}
            HORSEGAME_MALON_RACE => self.update_malon_race(play),
            _ => {}
        }
    }

    /// `EnHorseGameCheck_Destroy`: the game's destroy (`sDestroyFuncs`), each nothing.
    fn destroy(&mut self, _play: &mut PlayState) {}

    /// `EnHorseGameCheck_Draw`: nothing.
    fn draw(&self, _rs: &RenderState, _play: &PlayState, _view: &ViewInfo, _out: &mut DrawOut) {}

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
