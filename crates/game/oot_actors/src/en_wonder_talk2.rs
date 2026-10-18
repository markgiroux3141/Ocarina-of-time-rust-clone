//! `En_Wonder_Talk2` (`ovl_En_Wonder_Talk2/z_en_wonder_talk2.c`): an invisible spot with text
//! (`0x200 | baseMsgId`), such as the one by the window in Link's house.
//!
//! `params`: bits 0..5 a switch flag (0x3F: none), 6..13 the text, 14..15 the talk mode:
//! - 0 (normal) and 2 (check only): it offers to talk (`Actor_OfferTalkExchange`, within
//!   `triggerRange` + 50) when Link is within 40 + `triggerRange` in front of it; talking sets
//!   the switch flag (not in mode 2);
//! - 1 (forced) and 4 (the Gerudo Training Ground's forced check): the text opens by itself
//!   when Link comes within range, holding Link (`Player_SetCsActionWithHaltedActors` mode 8) until it's read;
//! - 3 (lock only): nothing, but it can be targeted.
//!
//! `rot.z` gives the range: past each 10 the next `targetMode` (`D_80B3A8E0`), and what's left
//! (1 to 10) times 40 the trigger range.
//!
//! A forced text holds Link in Player's cutscene mode 8 (`Player_SetCsActionWithHaltedActors`) until it's read, then
//! mode 7 lets him go.
//!
//! Params: `switchFlag` bits 0 to 5 (0x3F: none; set: gone);
//! `baseMsgId` bits 6 to 13 (the text, `0x200 | baseMsgId`); `talkMode` bits 14 and 15 (0 talk,
//! 1 a forced text, 4 in the Gerudo Training Ground but for flags 0x08, 0x16 and 0x2F; 2 talk
//! without the flag; 3 only targetable). `world.rot.z`: the range (0 and below: none), past each 10
//! the next `targetMode` (`D_80B3A8E0`: 6, 0 to 5; past 70 back to 6), the rest (1 to 10) times 40.
//! Flags: reads switch flag `switchFlag` (set: gone, or no more talk) and sets it once its text
//! is read (not in mode 2, nor the Gerudo Training Ground's mode 4).

use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_LOCK_ON_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, Source};
use oot_game::message::{TEXT_STATE_DONE, TEXT_STATE_EVENT, TEXT_STATE_NONE};
use oot_game::npc::{EXCH_ITEM_NONE, offer_talk_range, process_talk_request};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

pub const ACTOR_EN_WONDER_TALK2: i16 = 0x0185;

/// `En_Wonder_Talk2_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_WONDER_TALK2, name: "En_Wonder_Talk2", category: ACTORCAT_ITEMACTION, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_LOCK_ON_DISABLED, object: "gameplay_keep", schema: &SCHEMA };

/// `PARAMS_GET_U(this->actor.params, 0, 6)`: the switch flag.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 0, 6, Kind::SwitchFlag, "set once its text is read (not in modes 2 and 4); set at init, it's gone (0x3F: none)");
/// `PARAMS_GET_U(this->actor.params, 6, 8)`: the text, `0x200 | baseMsgId`.
pub const BASE_MSG_ID: Field = Field::params("baseMsgId", 6, 8, Kind::Range(0, 0xFF), "its text, 0x200 | this");
/// `PARAMS_GET_U(this->actor.params, 14, 2)`: how it talks.
pub const TALK_MODE: Field = Field::params(
    "talkMode",
    14,
    2,
    Kind::Named(&[
        (0, "talk: Link checks it, the flag set"),
        (1, "forced: the text opens as Link comes near (mode 4 in the Gerudo Training Ground)"),
        (2, "talk, the flag not set"),
        (3, "nothing: only targetable"),
    ]),
    "how its text opens",
);
/// `this->actor.world.rot.z`: the range.
pub const RANGE: Field = Field::rot(
    "world.rot.z",
    Source::RotZ,
    0,
    16,
    Kind::Range(-0x8000, 0x7FFF),
    "the range: past each 10 the next targetMode, the rest (1 to 10) times 40 (0 and below: none)",
);

/// En_Wonder_Talk2's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[SWITCH_FLAG, BASE_MSG_ID, TALK_MODE, RANGE],
    flags: &[FlagUse::new(Flag::Switch("switchFlag")).reads().sets().note("set: gone; set once its text is read (not in modes 2 and 4)")],
    todo: false,
    objects: &[],
};

/// `SCENE_GERUDO_TRAINING_GROUND` (the Gerudo Training Ground).
const SCENE_GERUDO_TRAINING_GROUND: u16 = 0x0B;

/// `D_80B3A8E0`: `targetMode` by `rot.z / 10`.
const D_80B3A8E0: [u8; 7] = [6, 0, 1, 2, 3, 4, 5];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_80B3A10C`: sets the text, then waits (mode 0, 2) or checks the range (1, 4).
    SetText,
    /// `func_80B3A15C`: offering to talk.
    Offer,
    /// `func_80B3A3D4`: a forced text is up.
    Forced,
    /// `func_80B3A4F8`: waiting for Link to come in range of a forced text.
    WaitForced,
    /// `EnWonderTalk2_DoNothing`.
    DoNothing,
}

pub struct EnWonderTalk2 {
    pub actor: Actor,
    pub action: Action,
    pub base_msg_id: i16,
    pub switch_flag: i16,
    pub talk_mode: i16,
    pub unk_156: bool,
    pub unk_158: i16,
    pub unk_15a: bool,
    pub trigger_range: f32,
    pub height: f32,
    pub init_pos: glam::Vec3,
}

impl EnWonderTalk2 {
    /// `EnWonderTalk2_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let base_msg_id = BASE_MSG_ID.get(actor.params);
        let mut trigger_range = 0.0;
        if RANGE.get(actor.world_rot.z) > 0 {
            let mut range_index = 0;
            let mut rot_z_mod10 = RANGE.get(actor.world_rot.z);
            while rot_z_mod10 > 10 {
                rot_z_mod10 -= 10;
                range_index += 1;
            }
            trigger_range = rot_z_mod10 as f32 * 40.0;
            if range_index > 6 {
                range_index = 0;
            }
            actor.target_mode = D_80B3A8E0[range_index];
        }
        let init_pos = actor.world_pos;
        let mut switch_flag = SWITCH_FLAG.get(actor.params);
        let mut talk_mode = TALK_MODE.get(actor.params);
        if switch_flag == 0x3F {
            switch_flag = -1;
        }
        let mut w = EnWonderTalk2 { actor, action: Action::SetText, base_msg_id, switch_flag, talk_mode, unk_156: false, unk_158: 0, unk_15a: false, trigger_range, height: 0.0, init_pos };
        if switch_flag >= 0 && play.flags.get_switch(switch_flag as i32) {
            w.actor.kill();
            return Box::new(w);
        }
        if talk_mode == 1 && play.scene_id == SCENE_GERUDO_TRAINING_GROUND && !matches!(switch_flag, 0x08 | 0x16 | 0x2F) {
            w.unk_15a = false;
            talk_mode = 4;
            w.talk_mode = talk_mode;
        }
        if talk_mode == 3 {
            w.actor.flags &= !ACTOR_FLAG_LOCK_ON_DISABLED;
            w.action = Action::DoNothing;
        }
        Box::new(w)
    }

    /// `func_80B3A10C`.
    fn func_80b3a10c(&mut self) {
        self.actor.text_id = 0x200 | self.base_msg_id as u16;
        self.action = if self.talk_mode == 1 || self.talk_mode == 4 { Action::WaitForced } else { Action::Offer };
    }

    /// `func_80B3A15C`: offer to talk while Link is in front and within range.
    fn func_80b3a15c(&mut self, play: &mut PlayState) {
        self.unk_158 += 1;
        if self.switch_flag >= 0 && play.flags.get_switch(self.switch_flag as i32) {
            if !self.unk_15a {
                self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
                self.unk_15a = true;
            }
        } else if process_talk_request(&mut self.actor) {
            if self.switch_flag >= 0 && self.talk_mode != 2 {
                play.flags.set_switch(self.switch_flag as i32);
            }
            self.action = Action::SetText;
        } else {
            let yaw_diff = (self.actor.yaw_towards_player.wrapping_sub(self.actor.world_rot.y) as i32).abs();
            let player_y = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos.y).unwrap_or(0.0);
            if !(self.actor.xz_dist_to_player > 40.0 + self.trigger_range || (player_y - self.actor.world_pos.y).abs() > 100.0 || yaw_diff >= 0x4000) {
                self.unk_158 = 0;
                offer_talk_range(play, &self.actor, self.trigger_range + 50.0, 100.0, EXCH_ITEM_NONE);
            }
        }
    }

    /// `func_80B3A3D4`: a forced text; once it's read (or closed), the switch flag and back to
    /// waiting.
    fn func_80b3a3d4(&mut self, play: &mut PlayState) {
        let state = play.message_state();
        let done = match state {
            TEXT_STATE_EVENT | TEXT_STATE_DONE => {
                if play.message_should_advance() {
                    if state == TEXT_STATE_EVENT {
                        play.msg_ctx.close_textbox(&mut play.audio);
                    }
                    true
                } else {
                    false
                }
            }
            TEXT_STATE_NONE => true,
            _ => false,
        };
        if done {
            if self.switch_flag >= 0 && self.talk_mode != 4 {
                play.flags.set_switch(self.switch_flag as i32);
            }
            if self.talk_mode == 4 {
                self.unk_15a = true;
            }
            self.actor.flags &= !(ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_UPDATE_CULLING_DISABLED);
            play.player_set_cs_action_with_halted_actors(None, 7);
            self.unk_156 = true;
            self.action = Action::WaitForced;
        }
    }

    /// `func_80B3A4F8`: a forced text opens when Link comes within range (once per entry).
    fn func_80b3a4f8(&mut self, play: &mut PlayState) {
        self.unk_158 += 1;
        if self.switch_flag >= 0 && play.flags.get_switch(self.switch_flag as i32) {
            if !self.unk_15a {
                self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
                self.unk_15a = true;
            }
        } else if self.talk_mode != 4 || !self.unk_15a {
            let player_y = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos.y).unwrap_or(0.0);
            if self.actor.xz_dist_to_player < 40.0 + self.trigger_range && (player_y - self.actor.world_pos.y).abs() < 100.0 && !play.player_in_cs_mode() {
                self.unk_158 = 0;
                if !self.unk_156 {
                    play.start_textbox(self.actor.text_id, None);
                    play.player_set_cs_action_with_halted_actors(None, 8);
                    self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_UPDATE_CULLING_DISABLED;
                    self.action = Action::Forced;
                }
            } else {
                self.unk_156 = false;
            }
        }
    }
}

impl ActorImpl for EnWonderTalk2 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnWonderTalk2_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::SetText => self.func_80b3a10c(),
            Action::Offer => self.func_80b3a15c(play),
            Action::Forced => self.func_80b3a3d4(play),
            Action::WaitForced => self.func_80b3a4f8(play),
            Action::DoNothing => {}
        }
        self.actor.world_pos.y = self.init_pos.y;
        self.actor.set_focus(self.height);
    }
    fn draw(&self, _rs: &RenderState, _play: &PlayState, _view: &ViewInfo, _out: &mut DrawOut) {}
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
