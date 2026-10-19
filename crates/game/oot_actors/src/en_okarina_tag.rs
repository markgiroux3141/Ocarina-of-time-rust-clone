//! `En_Okarina_Tag` (`ovl_En_Okarina_Tag/z_en_okarina_tag.c`): an invisible spot where a song
//! does something. Its params: the type (bits 10 to 15), the song (6 to 9; 0xF: any song is
//! right, `unk_158`), the switch flag (0 to 5; 0x3F none; set: the spot is spent and gone).
//! `world.rot.z` widens it by 40 a unit.
//! - Type 7: the song's check (`OCARINA_ACTION_CHECK_SARIA` + the song; the scarecrow's spot is
//!   the Song of Time's check) when Link plays near it, the switch flag set and the chime when
//!   it's right; close by, Link's ocarina turns to it (`unk_6A8`).
//! - Types 1, 2 (not for an adult), 4 and 6: Zelda's Lullaby (1: `EVENTCHKINF_39`; 6: the royal
//!   tomb's opening, `EVENTCHKINF_1D`), the Song of Storms (2: the windmill, faster), the Song of
//!   Time (4: the Door of Time, until `EVENTCHKINF_OPENED_DOOR_OF_TIME`): Link's ocarina near it
//!   starts the check, and when it's right their cutscenes play.
//! - Type 5: the royal tomb's wall (text 0x5021), its talk teaching the Sun's Song
//!   (`gSunSongGraveSunSongTeachCs`) until it's known.
//!
//! The whole overlay is ported (the debug display aside: `BREG(0)` 0).
//!
//! Params: `type` bits 10 to 15 (1, 2, 4, 6: a song's cutscene, 2 not for an adult; 5: the royal
//! tomb's wall; 7: a song's check; any other is killed at init); `ocarinaSong` bits 6 to 9 (0 to
//! 5 Saria's Song to the Song of Storms, 6 the scarecrow's, 0xF any); `switchFlag` bits 0 to 5
//! (0x3F: none). `world.rot.z`: the range, 40 a unit (0 and below: none).
//! Flags: reads the switch flag `switchFlag` (set: gone) and sets it when the song is right;
//! reads EVENTCHKINF_OPENED_DOOR_OF_TIME (type 4) and EVENTCHKINF_1D (type 6), sets EVENTCHKINF_39
//! (type 1) and EVENTCHKINF_1D (type 6); reads `scarecrowSpawnSongSet` (song 6).

use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_UPDATE_DURING_OCARINA, Actor};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, Source, TextLine};
use oot_game::event_chk_inf;
use oot_game::audio::sfx::NA_SE_SY_CORRECT_CHIME;
use oot_game::message::{
    MSGMODE_PAUSED, OCARINA_ACTION_CHECK_LULLABY, OCARINA_ACTION_CHECK_SARIA, OCARINA_ACTION_CHECK_STORMS, OCARINA_ACTION_CHECK_TIME, OCARINA_MODE_01, OCARINA_MODE_03, OCARINA_MODE_04,
    TEXT_STATE_EVENT,
};
use oot_game::play::PlayState;

use crate::player::{STATE2_23, STATE2_24, STATE2_25};

/// `ACTOR_EN_OKARINA_TAG` (`actor_table.h`: 0x012E).
pub const ACTOR_EN_OKARINA_TAG: i16 = 0x012E;

/// `En_Okarina_Tag_Profile`.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_EN_OKARINA_TAG, name: "En_Okarina_Tag", category: ACTORCAT_PROP, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA, object: "gameplay_keep", schema: &SCHEMA };

/// `PARAMS_GET_U(params, 10, 6)`: what playing here does.
pub const TYPE: Field = Field::params(
    "type",
    10,
    6,
    Kind::Named(&[
        (1, "Zelda's Lullaby: its cutscene (EVENTCHKINF_39)"),
        (2, "the Song of Storms: the windmill (a child's only)"),
        (4, "the Song of Time: the Door of Time"),
        (5, "the royal tomb's wall: the Sun's Song taught"),
        (6, "Zelda's Lullaby: the royal tomb's opening"),
        (7, "a song's check: the switch flag set when it's right"),
    ]),
    "what playing here does",
);
/// `PARAMS_GET_U(params, 6, 4)`: the song.
pub const OCARINA_SONG: Field = Field::params(
    "ocarinaSong",
    6,
    4,
    Kind::Named(&[
        (0, "Saria's Song"),
        (1, "Epona's Song"),
        (2, "Zelda's Lullaby"),
        (3, "the Sun's Song"),
        (4, "the Song of Time"),
        (5, "the Song of Storms"),
        (6, "the scarecrow's song"),
        (0xF, "any song"),
    ]),
    "the song a check takes (type 7); 6 waits for the scarecrow's song to be set",
);
/// `PARAMS_GET_U(params, 0, 6)`: the switch flag.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 0, 6, Kind::SwitchFlag, "set when the song is right; set at init, the spot is gone (0x3F: none)");
/// `world.rot.z`: the range.
pub const RANGE: Field = Field::rot("interactRange", Source::RotZ, 0, 16, Kind::Range(-0x8000, 0x7FFF), "the spot's range, 40 a unit (0 and below: none)");

/// En_Okarina_Tag's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, OCARINA_SONG, SWITCH_FLAG, RANGE],
    flags: &[
        FlagUse::new(Flag::Switch("switchFlag")).reads().sets(),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_OPENED_DOOR_OF_TIME)).reads().note("type 4: the Door of Time open, no song"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_1D)).reads().sets().note("type 6: the royal tomb opened"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_39)).sets().note("type 1: Zelda's Lullaby's cutscene"),
        FlagUse::new(Flag::Other("scarecrowSpawnSongSet")).reads().note("song 6: no check until it's set"),
    ],
    todo: false,
    objects: &[],
    texts: &[TextLine::new(0x5021).when("type", &[5]).note("the royal tomb's poem; the Sun's Song taught after it until known")],
};

/// `OcarinaMode`s past `OCARINA_MODE_04` (`ocarina.h`): a wrong song's (5 to 13), and 0xE.
const OCARINA_MODE_05: u16 = 0x05;
const OCARINA_MODE_0D: u16 = 0x0D;
const OCARINA_MODE_0E: u16 = 0x0E;
/// `ATTENTION_RANGE_1` (`actor.h`).
const ATTENTION_RANGE_1: u8 = 1;
/// `EVENTCHKINF_1D`, `EVENTCHKINF_39`, `EVENTCHKINF_OPENED_DOOR_OF_TIME` (`save.h`).
const EVENTCHKINF_1D: u16 = 0x1D;
const EVENTCHKINF_39: u16 = 0x39;
const EVENTCHKINF_OPENED_DOOR_OF_TIME: u16 = 0x4B;
/// `QUEST_SONG_SUN` (`item.h`).
const QUEST_SONG_SUN: u32 = 0x0F;
/// `SCENE_WATER_TEMPLE`, `SCENE_GREAT_FAIRYS_FOUNTAIN_MAGIC`, `_SPELLS` (`scene_table.h`).
const SCENE_WATER_TEMPLE: u16 = 0x05;
const SCENE_GREAT_FAIRYS_FOUNTAIN_MAGIC: u16 = 0x3B;
const SCENE_GREAT_FAIRYS_FOUNTAIN_SPELLS: u16 = 0x3D;

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_80ABEF2C`: type 7's wait.
    Func80abef2c,
    /// `func_80ABF0CC`: type 7's check running.
    Func80abf0cc,
    /// `func_80ABF28C`: the song spots' wait.
    Func80abf28c,
    /// `func_80ABF4C8`: their check running.
    Func80abf4c8,
    /// `func_80ABF708`: the tomb's wall's offer to talk.
    Func80abf708,
    /// `func_80ABF7CC`: its talk.
    Func80abf7cc,
}

pub struct EnOkarinaTag {
    pub actor: Actor,
    pub type_: i16,
    pub ocarina_song: i16,
    pub switch_flag: i32,
    pub unk_158: i16,
    pub unk_15a: i16,
    pub interact_range: f32,
    pub action: Action,
}

/// `player->stateFlags2`.
fn player_state2(play: &PlayState) -> u32 {
    play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map_or(0, |p| p.state_flags2())
}

/// `player->stateFlags2 |= flag`.
fn set_player_state2(play: &mut PlayState, flag: u32) {
    if let Some(p) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
        p.change_state_flags2(flag, 0);
    }
}

/// Player's `world.pos.y`.
fn player_y(play: &PlayState) -> f32 {
    play.player.and_then(|h| play.actors.actor(h)).map_or(0.0, |p| p.world_pos.y)
}

impl EnOkarinaTag {
    /// `EnOkarinaTag_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = EnOkarinaTag { actor, type_: 0, ocarina_song: 0, switch_flag: -1, unk_158: 0, unk_15a: 0, interact_range: 0.0, action: Action::Func80abf28c };
        this.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        this.type_ = TYPE.get(this.actor.params);
        this.ocarina_song = OCARINA_SONG.get(this.actor.params);
        this.switch_flag = SWITCH_FLAG.get(this.actor.params) as i32;
        if this.switch_flag == 0x3F {
            this.switch_flag = -1;
        }
        if this.ocarina_song == 0xF {
            this.ocarina_song = 0;
            this.unk_158 = 1;
        }
        this.actor.target_mode = ATTENTION_RANGE_1;
        if RANGE.get(this.actor.world_rot.z) > 0 {
            this.interact_range = RANGE.get(this.actor.world_rot.z) as f32 * 40.0;
        }
        log::debug!(
            "Ocarina tag spawn {:x}: save {} type {} song {} range {} ({}) hit? {}",
            // schema: printed
            this.actor.params as u16,
            this.switch_flag,
            this.type_,
            this.ocarina_song,
            this.actor.world_rot.z,
            this.interact_range,
            this.unk_158
        );
        if this.switch_flag >= 0 && play.flags.get_switch(this.switch_flag) {
            this.actor.kill();
        } else {
            match this.type_ {
                7 => this.action = Action::Func80abef2c,
                1 | 2 | 4 | 6 => {
                    if this.type_ == 2 && play.save.adult {
                        this.actor.kill();
                    } else {
                        this.action = Action::Func80abf28c;
                    }
                }
                5 => {
                    this.actor.text_id = 0x5021;
                    this.action = Action::Func80abf708;
                }
                _ => this.actor.kill(),
            }
        }
        Box::new(this)
    }

    /// `func_80ABEF2C`: type 7. Within 90 (plus its range) and 80 high, the ocarina turned to it
    /// starts the song's check; within 50 and 40 high the ocarina turns to it (`unk_6A8`). The
    /// scarecrow's song only once it's set.
    fn func_80abef2c(&mut self, play: &mut PlayState) {
        self.unk_15a = self.unk_15a.wrapping_add(1);
        if self.switch_flag >= 0 && play.flags.get_switch(self.switch_flag) {
            self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
            return;
        }
        if self.ocarina_song == 6 && play.save.scarecrow_spawn_song_set == 0 {
            return;
        }
        let state2 = player_state2(play);
        if state2 & STATE2_24 != 0 {
            log::trace!("North!!!!! {}", self.actor.xz_dist_to_player);
        }
        let dy = (player_y(play) - self.actor.world_pos.y).abs();
        if self.actor.xz_dist_to_player < 90.0 + self.interact_range && dy < 80.0 {
            if state2 & STATE2_25 != 0 {
                let mut song = self.ocarina_song as u16;
                if song == 6 {
                    song = 0xA;
                }
                set_player_state2(play, STATE2_23);
                play.message_start_ocarina(song + OCARINA_ACTION_CHECK_SARIA);
                self.action = Action::Func80abf0cc;
            } else if self.actor.xz_dist_to_player < 50.0 + self.interact_range && dy < 40.0 {
                self.unk_15a = 0;
                if let Some(me) = play.cur_actor
                    && let Some(p) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut())
                {
                    p.set_unk_6a8(Some(me));
                }
            }
        }
    }

    /// `func_80ABF0CC`: type 7's check: right, the flag and the chime (the ocarina put away
    /// unless in a Great Fairy's fountain; the Water Temple's box paused); any song for a spot
    /// that takes any; wrong, back to waiting.
    fn func_80abf0cc(&mut self, play: &mut PlayState) {
        let mode = play.msg_ctx.ocarina_mode;
        if mode == OCARINA_MODE_04 {
            self.action = Action::Func80abef2c;
            return;
        }
        if mode == OCARINA_MODE_03 {
            if self.switch_flag >= 0 {
                play.flags.set_switch(self.switch_flag);
            }
            if play.scene_id == SCENE_WATER_TEMPLE {
                play.msg_ctx.msg_mode = MSGMODE_PAUSED;
            }
            // Not OOT_VERSION < NTSC_1_1: a Great Fairy's fountain keeps the ocarina's mode.
            if play.scene_id != SCENE_GREAT_FAIRYS_FOUNTAIN_MAGIC && play.scene_id != SCENE_GREAT_FAIRYS_FOUNTAIN_SPELLS {
                play.msg_ctx.ocarina_mode = OCARINA_MODE_04;
            }
            play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
            self.action = Action::Func80abef2c;
            return;
        }
        if self.unk_158 != 0 && (matches!(mode, 0x05..=0x0A) || mode == OCARINA_MODE_0D) {
            if self.switch_flag >= 0 {
                play.flags.set_switch(self.switch_flag);
            }
            play.msg_ctx.ocarina_mode = OCARINA_MODE_04;
            play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
            self.action = Action::Func80abef2c;
            return;
        }
        if (OCARINA_MODE_05..OCARINA_MODE_0E).contains(&mode) {
            play.msg_ctx.ocarina_mode = OCARINA_MODE_04;
            self.action = Action::Func80abef2c;
        } else if mode == OCARINA_MODE_01 {
            set_player_state2(play, STATE2_23);
        }
    }

    /// `func_80ABF28C`: the song spots. Within 90 (plus its range) and 80 high (the Door of Time
    /// until it's opened, the royal tomb until it's open), the ocarina out starts the check; within
    /// 50 and 40 high the ocarina's asked for.
    fn func_80abf28c(&mut self, play: &mut PlayState) {
        self.unk_15a = self.unk_15a.wrapping_add(1);
        if self.ocarina_song == 6 && play.save.scarecrow_spawn_song_set == 0 {
            return;
        }
        if self.switch_flag >= 0 && play.flags.get_switch(self.switch_flag) {
            self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
            return;
        }
        let dy = (player_y(play) - self.actor.world_pos.y).abs();
        if (self.type_ != 4 || !play.save.get_event_chk_inf(EVENTCHKINF_OPENED_DOOR_OF_TIME))
            && (self.type_ != 6 || !play.save.get_event_chk_inf(EVENTCHKINF_1D))
            && self.actor.xz_dist_to_player < 90.0 + self.interact_range
            && dy < 80.0
        {
            if player_state2(play) & STATE2_24 != 0 {
                match self.type_ {
                    1 | 6 => play.message_start_ocarina(OCARINA_ACTION_CHECK_LULLABY),
                    2 => play.message_start_ocarina(OCARINA_ACTION_CHECK_STORMS),
                    4 => play.message_start_ocarina(OCARINA_ACTION_CHECK_TIME),
                    t => {
                        log::warn!("Ocarina Invisible-kun demo start check error -Hara {t}");
                        self.actor.kill();
                    }
                }
                set_player_state2(play, STATE2_23);
                self.action = Action::Func80abf4c8;
            } else if self.actor.xz_dist_to_player < 50.0 + self.interact_range && dy < 40.0 {
                self.unk_15a = 0;
                set_player_state2(play, STATE2_23);
            }
        }
    }

    /// `func_80ABF4C8`: the song spots' check: right, the chime, the flags, and the spot's
    /// cutscene (the windmill's music faster after the fanfare); wrong, back to waiting.
    fn func_80abf4c8(&mut self, play: &mut PlayState) {
        let mode = play.msg_ctx.ocarina_mode;
        if mode == OCARINA_MODE_04 {
            self.action = Action::Func80abf28c;
        } else if mode == OCARINA_MODE_03 {
            play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
            if self.switch_flag >= 0 {
                play.flags.set_switch(self.switch_flag);
            }
            match self.type_ {
                1 => {
                    play.flags.set_switch(self.switch_flag);
                    play.save.set_event_chk_inf(EVENTCHKINF_39);
                }
                2 => {
                    play.cs_ctx.segment = play.cutscene_script("gWindmillSpinningFasterCs");
                    play.save.cutscene_trigger = 1;
                    // Three semitones up, 2^(3/12), the tempo by the same: the windmill's music once
                    // the Song of Storms' fanfare ends.
                    play.audio.audio_set_main_bgm_tempo_freq_after_fanfare(1.18921, 90);
                }
                4 => {
                    play.cs_ctx.segment = play.cutscene_script("gDoorOfTimeOpeningCs");
                    play.save.cutscene_trigger = 1;
                }
                6 => {
                    play.cs_ctx.segment = play.cutscene_script(if play.save.adult { "gGraveyardTombOpeningAdultCs" } else { "gGraveyardTombOpeningChildCs" });
                    play.save.cutscene_trigger = 1;
                    play.save.set_event_chk_inf(EVENTCHKINF_1D);
                    play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
                }
                _ => {}
            }
            play.msg_ctx.ocarina_mode = OCARINA_MODE_04;
            self.action = Action::Func80abf28c;
        } else {
            if mode >= OCARINA_MODE_05 && mode < OCARINA_MODE_0E {
                play.msg_ctx.ocarina_mode = OCARINA_MODE_04;
                self.action = Action::Func80abf28c;
                return;
            }
            if mode == OCARINA_MODE_01 {
                set_player_state2(play, STATE2_23);
            }
        }
    }

    /// `func_80ABF708`: the tomb's wall offers its talk within 120, facing it within 0x4300 (its
    /// text 0x5021 either way).
    fn func_80abf708(&mut self, play: &mut PlayState) {
        if oot_game::npc::process_talk_request(&mut self.actor) {
            self.action = Action::Func80abf7cc;
            return;
        }
        let yaw_diff = self.actor.yaw_towards_player.wrapping_sub(self.actor.world_rot.y);
        self.unk_15a = self.unk_15a.wrapping_add(1);
        if !(self.actor.xz_dist_to_player > 120.0) {
            if play.save.check_quest_item(QUEST_SONG_SUN) {
                self.actor.text_id = 0x5021;
            }
            if (yaw_diff as i32).abs() < 0x4300 {
                self.unk_15a = 0;
                let a = self.actor.clone();
                oot_game::npc::offer_talk(play, &a, 70.0);
            }
        }
    }

    /// `func_80ABF7CC`: at the text's event, the box closes and, unless it's known, the Sun's
    /// Song's lesson plays.
    fn func_80abf7cc(&mut self, play: &mut PlayState) {
        log::trace!("Open gomagomagoma! {}", play.message_state());
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            play.with_msg(|m, f| m.close_textbox(f.audio));
            if !play.save.check_quest_item(QUEST_SONG_SUN) {
                play.cs_ctx.segment = play.cutscene_script("gSunSongGraveSunSongTeachCs");
                play.save.cutscene_trigger = 1;
            }
            self.action = Action::Func80abf708;
        }
    }
}

impl ActorImpl for EnOkarinaTag {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnOkarinaTag_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Func80abef2c => self.func_80abef2c(play),
            Action::Func80abf0cc => self.func_80abf0cc(play),
            Action::Func80abf28c => self.func_80abf28c(play),
            Action::Func80abf4c8 => self.func_80abf4c8(play),
            Action::Func80abf708 => self.func_80abf708(play),
            Action::Func80abf7cc => self.func_80abf7cc(play),
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
