//! `En_Wonder_Talk` (`ovl_En_Wonder_Talk/z_en_wonder_talk.c`): a spot to check (Navi green), such
//! as the Temple of Time's altar. `params`: `>> 11` the kind (`unk_150`), `(>> 6) & 0x1F` unused
//! (`unk_152`), `& 0x3F` a switch flag (0x3F none) that, once set, ends it.
//!
//! Kinds (`func_80B391CC`): 1 the altar's slate (0x7040 for a child, 0x7088 for an adult), 2 the
//! diary (0x5002, a choice: 0x5001 / 0x5003 by age, or 0x5004), 3 and 5 the graveyard's spots that
//! call a Poe (`En_Poh` types 2 and 3), 4 one gone after `EVENTCHKINF_1D` (0x5020), any other
//! 0x7072. It offers to talk within `unk_15C` when Link is in front of it (`func_80B3943C`).
//!
//! Not ported here: the debug display (`BREG(0)`, off).
//!
//! Params: `unk_150` bits 11 to 15 (the kind: 1 the altar's slate, 2 the diary, 3 and 5 a Poe's
//! spot, 4 the royal tomb's hint, any other 0x7072); `unk_152` bits 6 to 10 unused (0x1F in the
//! data; stored, never read); `switchFlag` bits 0 to 5 (set once read: gone; 0x3F none).
//! `world.rot.z`: debug (printed as the "specified range").
//! Flags: reads the switch flag `switchFlag` (set: gone) and sets it once the text is read; reads
//! EVENTCHKINF_1D (kind 4: gone once the royal tomb is open).

use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_LOCK_ON_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, Source};
use oot_game::event_chk_inf;
use oot_game::message::{TEXT_STATE_CHOICE, TEXT_STATE_DONE, TEXT_STATE_EVENT};
use oot_game::npc::{offer_talk, process_talk_request};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

/// `ACTOR_EN_WONDER_TALK` (`actor_table.h`: 0x0147).
pub const ACTOR_EN_WONDER_TALK: i16 = 0x0147;
/// `ACTOR_EN_POH` (`actor_table.h`: 0x000D).
const ACTOR_EN_POH: i16 = 0x000D;

/// `En_Wonder_Talk_Profile`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_WONDER_TALK,
    name: "En_Wonder_Talk",
    category: ACTORCAT_PROP,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_LOCK_ON_DISABLED,
    object: "gameplay_keep",
    schema: &SCHEMA,
};

/// `ATTENTION_RANGE_1`.
const ATTENTION_RANGE_1: u8 = 1;
/// `EVENTCHKINF_1D` (`save.h`).
const EVENTCHKINF_1D: u16 = 0x1D;

/// `PARAMS_GET_U(this->actor.params, 6, 5)` into `unk_152`, which nothing reads.
pub const UNK_152: Field = Field::params("unk_152", 6, 5, Kind::Unused, "0x1F in the data; stored, never read");
/// `PARAMS_GET_U(this->actor.params, 0, 6)`: the switch flag.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 0, 6, Kind::SwitchFlag, "set once the text is read: gone (0x3F: none)");
/// `PARAMS_GET_U(this->actor.params, 11, 5)`: the kind.
pub const KIND: Field = Field::params(
    "unk_150",
    11,
    5,
    Kind::Named(&[
        (0, "0x7072"),
        (1, "the Temple of Time's slate (0x7040 a child, 0x7088 an adult)"),
        (2, "the diary (0x5002, a choice)"),
        (3, "a Poe's spot (0x501E: En_Poh type 2)"),
        (4, "the royal tomb's hint (0x5020; gone with EVENTCHKINF_1D)"),
        (5, "a Poe's spot (0x501F: En_Poh type 3)"),
    ]),
    "what it says (any other: 0x7072)",
);
/// `this->actor.world.rot.z`: printed only ("specified range").
pub const RANGE: Field = Field::rot("world.rot.z", Source::RotZ, 0, 16, Kind::Debug, "printed as the \"specified range\"; nothing else reads it");

/// En_Wonder_Talk's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[SWITCH_FLAG, UNK_152, KIND, RANGE],
    flags: &[
        FlagUse::new(Flag::Switch("switchFlag")).reads().sets().note("set once read: gone"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_1D)).reads().note("kind 4: gone once the royal tomb is open"),
    ],
    todo: false,
};

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_80B391CC`: the text by kind.
    SetText,
    /// `func_80B3943C`: offering to talk.
    Offer,
    /// `func_80B395F0`: the text up.
    Talking,
}

pub struct EnWonderTalk {
    pub actor: Actor,
    pub action: Action,
    pub unk_150: i16,
    pub unk_152: i16,
    pub switch_flag: i16,
    /// `unk_156`: the message state the text ends on.
    pub unk_156: u8,
    pub unk_158: i16,
    pub unk_15a: i16,
    /// `unk_15C`: the range.
    pub unk_15c: f32,
    pub height: f32,
    /// `unk_164`: the Poe called.
    pub unk_164: u8,
}

impl EnWonderTalk {
    /// `EnWonderTalk_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // schema: printed
        log::debug!("En_Wonder_Talk: special conversation {:#x}", actor.params);
        let mut this = EnWonderTalk {
            unk_150: KIND.get(actor.params),
            unk_152: UNK_152.get(actor.params),
            switch_flag: SWITCH_FLAG.get(actor.params),
            action: Action::SetText,
            unk_156: 0,
            unk_158: 0,
            unk_15a: 0,
            unk_15c: 0.0,
            height: 0.0,
            unk_164: 0,
            actor,
        };
        if this.switch_flag == 0x3F {
            this.switch_flag = -1;
        }
        this.actor.target_mode = ATTENTION_RANGE_1;
        if this.switch_flag >= 0 && play.flags.get_switch(this.switch_flag as i32) {
            log::debug!("En_Wonder_Talk: you are shock! {}", this.switch_flag);
            this.actor.kill();
            return Box::new(this);
        }
        this.unk_15c = 40.0;
        Box::new(this)
    }

    fn switch_set(&self, play: &PlayState) -> bool {
        self.switch_flag >= 0 && play.flags.get_switch(self.switch_flag as i32)
    }

    /// `func_80B391CC`.
    fn set_text(&mut self, play: &PlayState) {
        if self.switch_set(play) {
            return;
        }
        match self.unk_150 {
            1 => {
                // The slate.
                self.height = 0.0;
                self.unk_15c = 80.0;
                self.actor.text_id = if !play.save.adult { 0x7040 } else { 0x7088 };
                self.unk_156 = TEXT_STATE_EVENT;
            }
            2 => {
                // The diary.
                self.actor.text_id = 0x5002;
                self.unk_156 = TEXT_STATE_CHOICE;
                self.unk_15c = 40.0;
                self.height = 30.0;
            }
            3 => {
                self.actor.text_id = 0x501E;
                self.unk_156 = TEXT_STATE_EVENT;
                self.height = 0.0;
                self.unk_15c = 110.0;
            }
            4 => {
                self.actor.text_id = 0x5020;
                self.unk_156 = TEXT_STATE_DONE;
                self.height = 0.0;
                self.unk_15c = 120.0;
                if play.save.get_event_chk_inf(EVENTCHKINF_1D) {
                    self.actor.kill();
                }
            }
            5 => {
                self.actor.text_id = 0x501F;
                self.unk_156 = TEXT_STATE_EVENT;
                self.height = 0.0;
                self.unk_15c = 110.0;
            }
            _ => {
                self.actor.text_id = 0x7072;
                self.unk_156 = TEXT_STATE_EVENT;
            }
        }
        log::debug!("En_Wonder_Talk: this->actor.talk_message {:#x}", self.actor.text_id);
        self.action = Action::Offer;
    }

    /// `func_80B3943C`: offering to talk to Link in front of it, within `unk_15C`.
    fn offer(&mut self, play: &mut PlayState) {
        self.unk_15a = self.unk_15a.wrapping_add(1);
        if self.unk_150 == 4 && play.save.get_event_chk_inf(EVENTCHKINF_1D) {
            self.actor.kill();
            return;
        }
        if self.switch_set(play) {
            return;
        }
        if process_talk_request(&mut self.actor) {
            if self.unk_156 != TEXT_STATE_DONE {
                self.action = Action::Talking;
            } else {
                if self.switch_flag >= 0 {
                    self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
                    play.flags.set_switch(self.switch_flag as i32);
                }
                self.action = Action::SetText;
            }
        } else if !(self.unk_15c < self.actor.xz_dist_to_player) {
            let yaw_diff_temp = self.actor.yaw_towards_player.wrapping_sub(self.actor.world_rot.y);
            let yaw_diff = (yaw_diff_temp as i32).abs();
            if yaw_diff < 0x4000 {
                if self.unk_15a >= 2 {
                    log::trace!("En_Wonder_Talk: switch {} kind {} text {:#x} range {}", self.switch_flag, self.unk_150, self.actor.text_id, RANGE.get(self.actor.world_rot.z));
                }
                self.unk_15a = 0;
                let a = self.actor.clone();
                offer_talk(play, &a, self.unk_15c);
            }
        }
    }

    /// `func_80B395F0`: at the text's end, the kind's reply.
    fn talking(&mut self, play: &mut PlayState) {
        if !(self.unk_156 == play.message_state() && play.message_should_advance()) {
            return;
        }
        if self.switch_flag >= 0 {
            self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
            play.flags.set_switch(self.switch_flag as i32);
        }
        match self.unk_150 {
            1 => {
                play.with_msg(|m, f| m.close_textbox(f.audio));
                self.action = Action::SetText;
            }
            2 => {
                match play.msg_ctx.choice_index {
                    0 => self.actor.text_id = if !play.save.adult { 0x5001 } else { 0x5003 },
                    1 => self.actor.text_id = 0x5004,
                    _ => {}
                }
                self.unk_156 = TEXT_STATE_DONE;
                play.continue_textbox(self.actor.text_id);
                self.action = Action::SetText;
            }
            3 | 5 => {
                play.with_msg(|m, f| m.close_textbox(f.audio));
                if self.unk_164 == 0 {
                    let params = if self.unk_150 == 3 { 2 } else { 3 };
                    let _ = play.actor_spawn(ACTOR_EN_POH, self.actor.world_pos, [0; 3], params);
                    self.unk_164 = 1;
                }
                self.action = Action::SetText;
            }
            _ => {}
        }
    }
}

impl ActorImpl for EnWonderTalk {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnWonderTalk_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if self.unk_158 != 0 {
            self.unk_158 -= 1;
        }
        match self.action {
            Action::SetText => self.set_text(play),
            Action::Offer => self.offer(play),
            Action::Talking => self.talking(play),
        }
        self.actor.set_focus(self.height);
    }
    fn render_state(&self) -> RenderState {
        RenderState::of(&self.actor)
    }
    /// No draw (`En_Wonder_Talk_Profile.draw` is NULL).
    fn draw(&self, _rs: &RenderState, _play: &PlayState, _view: &ViewInfo, _out: &mut DrawOut) {}
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
