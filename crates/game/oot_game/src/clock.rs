//! The clock (GAME-06 milestone 3, docs/adr/0056-the-clock.md): the time of day passing, as
//! `z_kankyo.c`, `z_scene.c`, `z_play.c` and `z_parameter.c` keep it.
//!
//! - The time is the save's (`gSaveContext.save.dayTime`), with the sky's own
//!   (`gSaveContext.skyboxTime`, what the lights and the normal sky read) and `nightFlag`.
//! - `gTimeSpeed` (`EnvStatics::time_speed`) is 0 from `Environment_Init` until the room's
//!   header sets it (`Scene_CommandTimeSettings`, `PlayState::scene_command_time_settings`):
//!   10 in Hyrule Field, 0 in the forest.
//! - `Environment_Update` adds it to the time every frame with no message open, no game over
//!   and no transition (twice as fast by night unless the Sun's Song speeds it), the sky
//!   following (`PlayState::environment_update_clock`).
//! - A new day (`nextDayTime`, after the Sun's Song or the night's end) counts the days and
//!   hatches the eggs (`PlayState::play_init_next_day_time`, `Environment_PlayTimeBasedSequence`).
//! - The Sun's Song's part of `Interface_Update` (`PlayState::interface_update_suns_song`):
//!   reached only once the song is played (`Oceff_Spot`, not ported) or a cutscene starts it.

use eng_math::{cos_s, sin_s};
use glam::Vec3;

use crate::env::clock_time;
use crate::play::PlayState;

/// `NEXT_TIME_*` (`environment.h`): `gSaveContext.nextDayTime`'s values: none, the day's
/// (`CLOCK_TIME(12, 0) + 1`), the night's (`CLOCK_TIME(0, 0)`), and the magic values the day's
/// sound effects count down from.
pub const NEXT_TIME_NONE: u16 = 0xFFFF;
pub const NEXT_TIME_DAY: u16 = 0x8001;
pub const NEXT_TIME_NIGHT: u16 = 0x0000;
pub const NEXT_TIME_DAY_SET: u16 = 0xFFFE;
pub const NEXT_TIME_NIGHT_SET: u16 = 0xFFFD;

/// `SunsSongState` (`save.h`).
pub const SUNSSONG_INACTIVE: i16 = 0;
pub const SUNSSONG_START: i16 = 1;
pub const SUNSSONG_SPEED_TIME: i16 = 2;
pub const SUNSSONG_SPECIAL: i16 = 3;

/// `GAMEMODE_END_CREDITS` (`save.h`: 3).
const GAMEMODE_END_CREDITS: u8 = 3;
/// `SCENE_HAUNTED_WASTELAND` (`scene_table.h`: 0x5E).
const SCENE_HAUNTED_WASTELAND: u16 = 0x5E;
/// `ROOM_TYPE_DUNGEON` (`room.h`: 1).
const ROOM_TYPE_DUNGEON: u8 = 1;
/// `OCARINA_ACTION_CHECK_NOWARP_DONE` (`ocarina.h`: 0x31), `OCARINA_MODE_04` (4).
const OCARINA_ACTION_CHECK_NOWARP_DONE: u16 = 0x31;
const OCARINA_MODE_04: u16 = 4;

/// `IS_DAY`'s bound for the Sun's Song (`CLOCK_TIME(6, 30)` .. `CLOCK_TIME(18, 0) + 1`).
fn suns_song_day(t: u16) -> (bool, bool) {
    let (dawn, dusk) = (clock_time(6, 30) as u16, clock_time(18, 0) as u16 + 1);
    (t >= dawn && t <= dusk, t >= dawn && t < dusk)
}

/// `envCtx->sunPos` at `day_time`: `(-sin, cos, cos / 6)` of the time from noon, times 120,
/// times 25 (`Environment_Init`, `Scene_CommandTimeSettings`, `Environment_DrawSunAndMoon`).
pub fn sun_pos(day_time: u16) -> Vec3 {
    let t = day_time.wrapping_sub(clock_time(12, 0) as u16) as i16;
    Vec3::new(-(sin_s(t) * 120.0) * 25.0, (cos_s(t) * 120.0) * 25.0, (cos_s(t) * 20.0) * 25.0)
}

impl PlayState {
    /// `Scene_CommandTimeSettings` (`SCENE_CMD_TIME_SETTINGS(hour, min, timeSpeed)`, run with the
    /// room's header): the time set unless the hour or minute is 0xFF; the scene's time speed
    /// (0 for 0xFF), which becomes `gTimeSpeed` unless the Sun's Song runs; the sun put by the
    /// time; and with time stopped outside a cutscene (or at `ENTR_LAKE_HYLIA_8`) the sky's
    /// time snapped out of the dawn's and the dusk's blends (PAL: from the time).
    pub fn scene_command_time_settings(&mut self, [hour, min, speed]: [u8; 3]) {
        if hour != 0xFF && min != 0xFF {
            let t = (((hour as f32 + (min as f32 / 60.0)) * 60.0) / ((24 * 60) as f32 / 65536.0)) as i32 as u16;
            self.save.day_time = t;
            self.save.skybox_time = t;
        }
        self.env_ctx.scene_time_speed = if speed != 0xFF { speed } else { 0 };
        if self.save.suns_song_state == SUNSSONG_INACTIVE {
            self.env_statics.time_speed = self.env_ctx.scene_time_speed as u16;
        }
        self.env_ctx.sun_pos = sun_pos(self.save.day_time);
        let lake_hylia_8 = self.assets.as_ref().and_then(|a| a.scenes.entrance_index("ENTR_LAKE_HYLIA_8"));
        if (self.env_ctx.scene_time_speed == 0 && self.save.cutscene_index < 0xFFF0) || Some(self.save.entrance_index) == lake_hylia_8 {
            self.save.skybox_time = self.save.day_time;
            let c = |h, m| clock_time(h, m) as u16;
            let s = self.save.skybox_time;
            if s > c(4, 0) && s < c(6, 30) {
                self.save.skybox_time = c(5, 0) + 1;
            } else if s >= c(6, 30) && s <= c(8, 0) {
                self.save.skybox_time = c(8, 0) + 1;
            } else if s >= c(16, 0) && s <= c(17, 0) {
                self.save.skybox_time = c(17, 0) + 1;
            } else if s >= c(18, 0) + 1 && s <= c(19, 0) {
                self.save.skybox_time = c(19, 0) + 1;
            }
        }
    }

    /// `Environment_Update`'s clock (after the time-based music, before the lights): the new
    /// day's sound effect counted down 15 frames (`nextDayTime` from 0xFFFE or 0xFFFD by 0x10:
    /// the cock's crow, the dog's howl); the time advanced by `gTimeSpeed` (twice that by night,
    /// unless it's the Sun's Song's 400) with no message open (or in the credits), no game over,
    /// no sky change and no transition (or outside normal play); the sky's time following it in
    /// a cutscene layer from 5 or while time passes, and from midnight to 1:00; `nightFlag`.
    pub fn environment_update_clock(&mut self) {
        use crate::audio::sfx::{NA_SE_EV_CHICKEN_CRY_M, NA_SE_EV_DOG_CRY_EVENING};
        let next = self.save.next_day_time;
        if next >= 0xFF00 && next != NEXT_TIME_NONE {
            self.save.next_day_time -= 0x10;
            let next = self.save.next_day_time;
            log::debug!("next_zelda_time=[{next:x}]");
            if next == NEXT_TIME_DAY_SET - 15 * 0x10 {
                self.audio.play_sfx_centered(NA_SE_EV_CHICKEN_CRY_M);
                self.save.next_day_time = NEXT_TIME_NONE;
            } else if next == NEXT_TIME_NIGHT_SET - 15 * 0x10 {
                self.audio.play_sfx_centered2(NA_SE_EV_DOG_CRY_EVENING);
                self.save.next_day_time = NEXT_TIME_NONE;
            }
        }
        let speed = self.env_statics.time_speed;
        // (pauseCtx->state is PAUSE_STATE_OFF here: Environment_Update's body runs only then.)
        if self.game_over_ctx.state == crate::game_over::GAMEOVER_INACTIVE
            && ((self.msg_ctx.msg_length == 0 && self.msg_ctx.msg_mode == crate::message::MSGMODE_NONE) || self.save.game_mode == GAMEMODE_END_CREDITS)
            && self.env_ctx.change_skybox_timer == 0
            && (self.transition.mode == crate::transition::TRANS_MODE_OFF || self.save.game_mode != crate::save::GAMEMODE_NORMAL)
        {
            let step = if self.save.is_day() || speed >= 400 { speed } else { speed.wrapping_mul(2) };
            self.save.day_time = self.save.day_time.wrapping_add(step);
        }
        // PAL's rule (gTimeSpeed is unsigned: its `< 0` never holds).
        let s = &mut self.save;
        if ((s.scene_layer >= 5 || speed != 0) && s.day_time > s.skybox_time) || s.day_time < clock_time(1, 0) as u16 {
            s.skybox_time = s.day_time;
        }
        let t = s.day_time;
        s.night_flag = t > clock_time(18, 0) as u16 || t < clock_time(6, 30) as u16;
    }

    /// `Play_Init`'s new day, after `Interface_Init`: a day passed (`NEXT_TIME_DAY`) counts the
    /// days (`totalDays`, `bgsDayCount`), loses the dog again and hatches the eggs (text 0x3066),
    /// then marks the cock's crow due; a night marks the dog's howl.
    pub fn play_init_next_day_time(&mut self) {
        use crate::item::{ITEM_CHICKEN, ITEM_POCKET_CUCCO, ITEM_POCKET_EGG, ITEM_WEIRD_EGG, inventory_replace_item};
        if self.save.next_day_time == NEXT_TIME_NONE {
            return;
        }
        if self.save.next_day_time == NEXT_TIME_DAY {
            self.save.total_days += 1;
            self.save.bgs_day_count += 1;
            self.save.dog_is_lost = true;
            if inventory_replace_item(&mut self.save, ITEM_WEIRD_EGG, ITEM_CHICKEN) || inventory_replace_item(&mut self.save, ITEM_POCKET_EGG, ITEM_POCKET_CUCCO) {
                self.start_textbox(0x3066, None);
            }
            self.save.next_day_time = NEXT_TIME_DAY_SET;
        } else {
            self.save.next_day_time = NEXT_TIME_NIGHT_SET;
        }
    }

    /// The end of `Interface_Update`: the Sun's Song played. Its ocarina mode ends once the song
    /// is done; where time passes it runs at 400 until the next dawn or dusk; elsewhere (outside
    /// dungeon rooms, where the scene allows it) the scene reloads at the next noon or midnight
    /// (`nextDayTime`), faded black or white; in a dungeon room it's only marked played.
    pub fn interface_update_suns_song(&mut self) {
        use crate::transition::{TRANS_TRIGGER_START, TRANS_TYPE_FADE_BLACK, TRANS_TYPE_FADE_BLACK_FAST, TRANS_TYPE_FADE_WHITE, TRANS_TYPE_FADE_WHITE_FAST, TRANS_TYPE_SANDSTORM_PERSIST};
        if self.save.suns_song_state == SUNSSONG_INACTIVE {
            return;
        }
        if self.msg_ctx.ocarina_action != OCARINA_ACTION_CHECK_NOWARP_DONE && self.save.suns_song_state == SUNSSONG_START {
            self.msg_ctx.ocarina_mode = OCARINA_MODE_04;
        }
        let (day_inclusive, day) = suns_song_day(self.save.day_time);
        if self.env_ctx.scene_time_speed != 0 {
            if self.save.suns_song_state != SUNSSONG_SPEED_TIME {
                self.env_statics.suns_song_from_day = day_inclusive;
                self.save.suns_song_state = SUNSSONG_SPEED_TIME;
                self.env_statics.prev_time_speed = self.env_statics.time_speed;
                self.env_statics.time_speed = 400;
            } else if !self.env_statics.suns_song_from_day {
                if day_inclusive {
                    self.save.suns_song_state = SUNSSONG_INACTIVE;
                    self.env_statics.time_speed = self.env_statics.prev_time_speed;
                    self.msg_ctx.ocarina_mode = OCARINA_MODE_04;
                }
            } else if self.save.day_time > clock_time(18, 0) as u16 + 1 {
                self.save.suns_song_state = SUNSSONG_INACTIVE;
                self.env_statics.time_speed = self.env_statics.prev_time_speed;
                self.msg_ctx.ocarina_mode = OCARINA_MODE_04;
            }
        } else if self.room_ctx.cur.behavior_type1 != ROOM_TYPE_DUNGEON && self.interface_ctx.restrictions.suns_song != 3 {
            if day {
                self.save.next_day_time = NEXT_TIME_NIGHT;
                self.transition.ty = TRANS_TYPE_FADE_BLACK_FAST;
                self.save.next_transition_type = TRANS_TYPE_FADE_BLACK;
            } else {
                self.save.next_day_time = NEXT_TIME_DAY;
                self.transition.ty = TRANS_TYPE_FADE_WHITE_FAST;
                self.save.next_transition_type = TRANS_TYPE_FADE_WHITE;
            }
            self.halt_all_actors = true;
            if self.scene_id == SCENE_HAUNTED_WASTELAND {
                self.transition.ty = TRANS_TYPE_SANDSTORM_PERSIST;
                self.save.next_transition_type = TRANS_TYPE_SANDSTORM_PERSIST;
            }
            self.save.respawn_flag = -2;
            self.transition.next_entrance_index = self.save.entrance_index;
            self.transition.trigger = TRANS_TRIGGER_START;
            self.save.suns_song_state = SUNSSONG_INACTIVE;
            // PAL: the music faded over 30 frames, and nothing recorded as playing.
            self.audio.func_800f6964(30);
            self.save.seq_id = crate::audio::NA_BGM_DISABLED as u8;
            self.save.nature_ambience_id = crate::audio::NATURE_ID_DISABLED;
        } else {
            self.save.suns_song_state = SUNSSONG_SPECIAL;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_time_constants_are_the_clock_times() {
        // NEXT_TIME_DAY is CLOCK_TIME(12, 0) + 1, NEXT_TIME_NIGHT CLOCK_TIME(0, 0).
        assert_eq!(NEXT_TIME_DAY as i32, clock_time(12, 0) + 1);
        assert_eq!(NEXT_TIME_NIGHT as i32, clock_time(0, 0));
    }

    #[test]
    fn the_sun_is_overhead_at_noon_and_below_at_midnight() {
        // sunPos at noon: (-sin 0, cos 0, cos 0 / 6) * 120 * 25.
        assert_eq!(sun_pos(clock_time(12, 0) as u16), Vec3::new(0.0, 3000.0, 500.0));
        let m = sun_pos(0);
        assert!(m.y < -2999.0 && m.x.abs() < 1.0, "{m:?}");
        // At 18:00 the sun is on the horizon to the west (-x is the sin's sign: at +0x4000, -1).
        let d = sun_pos(clock_time(18, 0) as u16);
        assert!((d.x + 3000.0).abs() < 1.0 && d.y.abs() < 1.0, "{d:?}");
    }
}
