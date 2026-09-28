//! `En_Wonder_Talk2` (`ovl_En_Wonder_Talk2/z_en_wonder_talk2.c`): an invisible spot with text
//! (`0x200 | baseMsgId`), such as the one by the window in Link's house.
//!
//! `params`: bits 0..5 a switch flag (0x3F: none), 6..13 the text, 14..15 the talk mode:
//! - 0 (normal) and 2 (check only): it offers to talk (`func_8002F1C4`, within
//!   `triggerRange` + 50) when Link is within 40 + `triggerRange` in front of it; talking sets
//!   the switch flag (not in mode 2);
//! - 1 (forced) and 4 (the Gerudo Training Ground's forced check): the text opens by itself
//!   when Link comes within range, holding Link (`func_8002DF54` mode 8) until it's read;
//! - 3 (lock only): nothing, but it can be targeted.
//!
//! `rot.z` gives the range: `rot.z % 10` times 40, and `rot.z / 10` the `targetMode`.
//!
//! Not ported: Player's cutscene modes (`func_8002DF54` 7 and 8), so a forced text doesn't
//! hold Link.

use oot_game::actor::{ACTOR_FLAG_0, ACTOR_FLAG_3, ACTOR_FLAG_4, ACTOR_FLAG_27, Actor};
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorImpl, ActorProfile};
use oot_game::message::{TEXT_STATE_DONE, TEXT_STATE_EVENT, TEXT_STATE_NONE, should_advance};
use oot_game::npc::{EXCH_ITEM_NONE, offer_talk_range, process_talk_request};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

pub const ACTOR_EN_WONDER_TALK2: i16 = 0x0185;

/// `En_Wonder_Talk2_InitVars`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_WONDER_TALK2, name: "En_Wonder_Talk2", category: ACTORCAT_ITEMACTION, flags: ACTOR_FLAG_0 | ACTOR_FLAG_3 | ACTOR_FLAG_27, object: "gameplay_keep" };

/// `SCENE_MEN` (the Gerudo Training Ground).
const SCENE_MEN: u16 = 0x0B;

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
        let base_msg_id = (actor.params >> 6) & 0xFF;
        let mut trigger_range = 0.0;
        if actor.world_rot.z > 0 {
            let mut range_index = 0;
            let mut rot_z_mod10 = actor.world_rot.z;
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
        let mut switch_flag = actor.params & 0x3F;
        let mut talk_mode = (actor.params >> 0xE) & 3;
        if switch_flag == 0x3F {
            switch_flag = -1;
        }
        let mut w = EnWonderTalk2 { actor, action: Action::SetText, base_msg_id, switch_flag, talk_mode, unk_156: false, unk_158: 0, unk_15a: false, trigger_range, height: 0.0, init_pos };
        if switch_flag >= 0 && play.flags.get_switch(switch_flag as i32) {
            w.actor.kill();
            return Box::new(w);
        }
        if talk_mode == 1 && play.scene_id == SCENE_MEN && !matches!(switch_flag, 0x08 | 0x16 | 0x2F) {
            w.unk_15a = false;
            talk_mode = 4;
            w.talk_mode = talk_mode;
        }
        if talk_mode == 3 {
            w.actor.flags &= !ACTOR_FLAG_27;
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
                self.actor.flags &= !ACTOR_FLAG_0;
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
                if should_advance(&play.input) {
                    if state == TEXT_STATE_EVENT {
                        play.msg_ctx.close_textbox();
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
            self.actor.flags &= !(ACTOR_FLAG_0 | ACTOR_FLAG_4);
            // func_8002DF54(play, NULL, 7): Player's cutscene modes aren't ported.
            self.unk_156 = true;
            self.action = Action::WaitForced;
        }
    }

    /// `func_80B3A4F8`: a forced text opens when Link comes within range (once per entry).
    fn func_80b3a4f8(&mut self, play: &mut PlayState) {
        self.unk_158 += 1;
        if self.switch_flag >= 0 && play.flags.get_switch(self.switch_flag as i32) {
            if !self.unk_15a {
                self.actor.flags &= !ACTOR_FLAG_0;
                self.unk_15a = true;
            }
        } else if self.talk_mode != 4 || !self.unk_15a {
            let player_y = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos.y).unwrap_or(0.0);
            if self.actor.xz_dist_to_player < 40.0 + self.trigger_range && (player_y - self.actor.world_pos.y).abs() < 100.0 && !play.player_in_cs_mode() {
                self.unk_158 = 0;
                if !self.unk_156 {
                    play.start_textbox(self.actor.text_id, None);
                    // func_8002DF54(play, NULL, 8): Player's cutscene modes aren't ported.
                    self.actor.flags |= ACTOR_FLAG_0 | ACTOR_FLAG_4;
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
