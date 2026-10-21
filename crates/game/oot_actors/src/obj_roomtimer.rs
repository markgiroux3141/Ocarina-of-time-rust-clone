//! `Obj_Roomtimer` (`ovl_Obj_Roomtimer/z_obj_roomtimer.c`): a room's clear (GAME-10 milestone 5):
//! once the room's enemies are gone (its temporary clear flag), the chime, the room's clear flag
//! and a switch flag; with a time, the HUD's timer counting down meanwhile, and Link voided out of
//! the room when it runs out.
//!
//! Master Quest's Jabu-Jabu (`bdan`) places five, all without a time (0x3FF), the same in layers
//! 0 to 3: 0x2FFF (flag 0x0B) in room 8 at (-113, -340, -5669) with the red tentacle (`En_Ba`
//! 0x0B00, the same flag), 0x2BFF (0x0A) in room 9 at (1543, -340, -4829) with the gray, 0x33FF
//! (0x0C) in room 10 at (-1558, 33, -4664) with the green, 0x07FF (0x01) in room 11 at (628, -60,
//! -5468) and 0x17FF (0x05) in room 13 at (-992, -1233, -2378).
//!
//! - **Its first update** (`func_80B9D054`): with a time, the timer (`Interface_SetTimer`: counting
//!   down from it, the HUD's main timer); it stops counting as an enemy (`Actor_ChangeCategory` to
//!   `ACTORCAT_PROP`: an enemy until then, so a room it alone holds is cleared there, as
//!   `Actor_RemoveFromCategory` sets the temporary clear flag when the room's last enemy goes).
//! - **Waiting** (`func_80B9D0B0`): the room's temporary clear flag set (`Flags_GetTempClear`): the
//!   timer stopped (`TIMER_STATE_STOP`, with a time), the room's clear flag (`Flags_SetClear`), its
//!   switch flag, `NA_SE_SY_CORRECT_CHIME`, and it's gone. With a time run out (`timerSeconds` 0):
//!   `NA_SE_OC_ABYSS` and a void out (`Play_TriggerVoidOut`), and it's gone.
//! - **Gone** (`ObjRoomtimer_Destroy`) with a time and seconds left, the timer stops.
//!
//! The whole overlay is ported. It has no draw.
//!
//! Params: `time` bits 0 to 9 (seconds, at most 600; 0x3FF none); `switchFlag` bits 10 to 15 (set
//! when the room is cleared).
//! Flags: reads the room's temporary clear flag (and sets it, the room's last enemy, as it turns
//! prop); sets the room's clear flag and switch flag `switchFlag`.

use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ACTORCAT_PROP, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::{NA_SE_OC_ABYSS, NA_SE_SY_CORRECT_CHIME};
use oot_game::play::PlayState;
use oot_game::save::TIMER_STATE_STOP;

/// `ACTOR_OBJ_ROOMTIMER` (`actor_table.h`: 0x0187).
pub const ACTOR_OBJ_ROOMTIMER: i16 = 0x0187;

/// `Obj_Roomtimer_Profile`: `ACTORCAT_ENEMY` (a prop after its first update), always updating,
/// `OBJECT_GAMEPLAY_KEEP`, no draw.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_OBJ_ROOMTIMER, name: "Obj_Roomtimer", category: ACTORCAT_ENEMY, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: "gameplay_keep", schema: &SCHEMA };

/// `PARAMS_GET_U(params, 0, 10)`: the time.
pub const TIME: Field = Field::params("time", 0, 10, Kind::Range(0, 0x3FF), "the timer's seconds (over 600: 600; 0x3FF none)");
/// `PARAMS_GET_U(params, 10, 6)`: `switchFlag`.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 10, 6, Kind::SwitchFlag, "set when the room is cleared");

/// Obj_Roomtimer's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TIME, SWITCH_FLAG],
    flags: &[
        FlagUse::new(Flag::TempClear).reads().sets().note("the room's: its enemies gone (set by its own change of category when it was the room's last enemy)"),
        FlagUse::new(Flag::Clear).sets().note("the room's, once its enemies are gone"),
        FlagUse::new(Flag::Switch("switchFlag")).sets().note("once the room's enemies are gone"),
    ],
    todo: false,
    objects: &[],
    texts: &[],
};

/// No time (`0x3FF`).
pub const NO_TIME: i16 = 0x3FF;

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_80B9D054`.
    Start,
    /// `func_80B9D0B0`.
    Wait,
}

pub struct ObjRoomtimer {
    pub actor: Actor,
    pub action: Action,
    /// `switchFlag`.
    pub switch_flag: i32,
}

/// `Actor_ChangeCategory` of the updating actor: out of its list (`Actor_RemoveFromCategory`: the
/// current room's last enemy gone sets its temporary clear flag) and onto `category`'s.
// shared: z_actor.c's Actor_ChangeCategory with Actor_RemoveFromCategory's room clear
pub fn actor_change_category(play: &mut PlayState, actor: &mut Actor, category: usize) {
    let Some(me) = play.cur_actor else { return };
    let was = actor.category;
    play.actors.change_category(me, category);
    actor.category = category;
    let cur = play.room_ctx.cur.num;
    if actor.room == cur && was == ACTORCAT_ENEMY && play.actors.category(ACTORCAT_ENEMY).is_empty() {
        play.flags.set_temp_clear(cur);
    }
}

impl ObjRoomtimer {
    /// `ObjRoomtimer_Init`: the flag and the time apart, the time at most 600.
    pub fn init(mut actor: Actor, _play: &mut PlayState) -> Box<dyn ActorImpl> {
        let switch_flag = SWITCH_FLAG.get(actor.params) as i32;
        // schema: params = the time alone
        actor.params = TIME.get(actor.params);
        if TIME.get(actor.params) != NO_TIME {
            // schema: CLAMP_MAX(params, 600)
            actor.params = actor.params.min(600);
        }
        Box::new(ObjRoomtimer { actor, action: Action::Start, switch_flag })
    }

    /// The time (`params` after init).
    pub fn time(&self) -> i16 {
        TIME.get(self.actor.params)
    }

    /// `func_80B9D054`: the timer, a prop from now.
    fn start(&mut self, play: &mut PlayState) {
        if self.time() != NO_TIME {
            play.interface_set_timer(self.time());
        }
        actor_change_category(play, &mut self.actor, ACTORCAT_PROP);
        self.action = Action::Wait;
    }

    /// `func_80B9D0B0`: see the module's docs.
    fn wait(&mut self, play: &mut PlayState) {
        if play.flags.get_temp_clear(self.actor.room) {
            if self.time() != NO_TIME {
                play.save.timer_state = TIMER_STATE_STOP;
            }
            play.flags.set_clear(self.actor.room);
            play.flags.set_switch(self.switch_flag);
            play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
            self.actor.kill();
            return;
        }
        if self.time() != NO_TIME && play.save.timer_seconds == 0 {
            play.audio.play_sfx_centered(NA_SE_OC_ABYSS);
            play.trigger_void_out();
            self.actor.kill();
        }
    }
}

impl ActorImpl for ObjRoomtimer {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `ObjRoomtimer_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Start => self.start(play),
            Action::Wait => self.wait(play),
        }
    }
    /// `ObjRoomtimer_Destroy`: with a time and seconds left, the timer stopped.
    fn destroy(&mut self, play: &mut PlayState) {
        if self.time() != NO_TIME && play.save.timer_seconds > 0 {
            play.save.timer_state = TIMER_STATE_STOP;
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
