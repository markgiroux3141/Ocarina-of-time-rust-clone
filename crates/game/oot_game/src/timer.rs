//! The HUD's timers (`z_parameter.c`): `Interface_SetTimer`, `Interface_SetSubTimer`,
//! `Interface_SetSubTimerToFinalSecond`, and `Interface_Draw`'s timer part.
//!
//! The main timer (`timerState`, `timerSeconds`) counts down from what an actor set (Talon's
//! Super Cucco game, Ingo's horse rental) or up from 0, or down from half Link's health in a hot
//! room or under water (`TIMER_STATE_ENV_HAZARD_*`, started by `Interface_Update`); the sub timer
//! (`subTimerState`, `subTimerSeconds`) counts while the main one is off (the trade quest's
//! spoiling items, the marathon man). Each shows in the middle of the screen for 20 frames
//! (`*_PREVIEW`), slides to the top left under the hearts in 20 (`*_MOVE`), and ticks a second
//! every 20 frames (`*_TICK`) while no text is open (`msgLength` 0, the main timer only), with
//! beeps: every 10 seconds above a minute, every other second above 10, every second below.
//!
//! In the C the state machine runs in `Interface_Draw`, in the draw pass: the port runs it in the
//! game frame where `Play_Draw` would (`PlayState::interface_draw_timers`, after the reticle and
//! before the message box), and draws what it left (`TimerStatics::draw`). Its counters are
//! `Interface_Draw`'s function statics and `sEnvHazardActive` is the file's: they outlive the play
//! state (`PlayState::reinit` carries them).
//!
//! `gSaveContext.minigameState` (horseback archery) and `play->shootingGalleryStatus` aren't
//! ported: they read as 0, which lets the timers run.

use crate::interface::{CLOCK_ICON, digit_sprite};
use crate::play::PlayState;
use crate::save::*;
use crate::sprite::Sprite;

/// `Interface_Draw`'s timer statics (`sTimerNextSecondTimer`, `sTimerStateTimer`,
/// `sSubTimerNextSecondTimer`, `sSubTimerStateTimer`, `sTimerDigits`) and `sEnvHazardActive`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TimerStatics {
    pub timer_next_second_timer: i16,
    pub timer_state_timer: i16,
    pub sub_timer_next_second_timer: i16,
    pub sub_timer_state_timer: i16,
    /// Minutes' tens and units, the colon (10), seconds' tens and units.
    pub timer_digits: [i16; 5],
    /// `sEnvHazardActive`: the main timer is the hot room's or the water's (running out kills).
    pub env_hazard_active: bool,
    /// The timer the last `Interface_Draw` drew (`timerId`), if it drew one.
    pub drawn: Option<usize>,
}

/// `timerDigitLeftPos`, `sDigitWidths`.
const TIMER_DIGIT_LEFT_POS: [i16; 5] = [16, 25, 34, 42, 51];
const DIGIT_WIDTHS: [i16; 5] = [9, 9, 8, 9, 9];
/// `VREG(43) << 1` (`z_construct.c`: `VREG(43) = 440`): the digits' `dsdx` and `dtdy`, s5.10
/// texels per pixel.
const DIGIT_STEP: f32 = (440 << 1) as f32 / 1024.0;

/// `EVENTINF_MARATHON_ACTIVE` (`save.h`: 0x10); `MARATHON_TIME_LIMIT` (240: four minutes).
const EVENTINF_MARATHON_ACTIVE: u16 = 0x10;
const MARATHON_TIME_LIMIT: i16 = 240;
/// `PLAYER_STATE2_24` (the ocarina out).
const PLAYER_STATE2_24: u32 = 1 << 24;
/// `SCENE_BOMBCHU_BOWLING_ALLEY` (`scene_table.h`: 0x4B).
const SCENE_BOMBCHU_BOWLING_ALLEY: u16 = 0x4B;
/// `eventInf[EVENTINF_INDEX_HORSES]`'s Ingo race bits the sub timer's respawn clears
/// (`EVENTINF_INGO_RACE_STATE_MASK` 0xF, `EVENTINF_MASK` of `_HORSETYPE` 4, `_LOST_ONCE` 5,
/// `_SECOND_RACE` 6 and `_0F` 0xF).
const INGO_RACE_BITS: u16 = 0xF | (1 << 4) | (1 << 5) | (1 << 6) | (1 << 0xF);
/// `spoilingItemEntrances`.
const SPOILING_ITEM_ENTRANCES: [&str; 3] = ["ENTR_LOST_WOODS_2", "ENTR_ZORAS_DOMAIN_3", "ENTR_ZORAS_DOMAIN_3"];

/// The timer's place under the hearts: one row (46) or two (54, more than ten hearts).
fn hearts_row_y(save: &SaveContext) -> i16 {
    if save.health_capacity > 0xA0 { 54 } else { 46 }
}

impl PlayState {
    /// `Interface_SetTimer`: the main timer at (140, 80), counting down from `seconds`
    /// (`TIMER_STATE_DOWN_INIT`), or up from 0 when `seconds` is 0 (`TIMER_STATE_UP_INIT`); not
    /// the hot room's (`sEnvHazardActive`).
    pub fn interface_set_timer(&mut self, seconds: i16) {
        let s = &mut self.save;
        s.timer_x[TIMER_ID_MAIN] = 140;
        s.timer_y[TIMER_ID_MAIN] = 80;
        self.interface_ctx.timers.env_hazard_active = false;
        s.timer_seconds = seconds;
        s.timer_state = if seconds != 0 { TIMER_STATE_DOWN_INIT } else { TIMER_STATE_UP_INIT };
    }

    /// `Interface_SetSubTimer`: the sub timer the same way.
    pub fn interface_set_sub_timer(&mut self, seconds: i16) {
        let s = &mut self.save;
        s.timer_x[TIMER_ID_SUB] = 140;
        s.timer_y[TIMER_ID_SUB] = 80;
        self.interface_ctx.timers.env_hazard_active = false;
        s.sub_timer_seconds = seconds;
        s.sub_timer_state = if seconds != 0 { SUBTIMER_STATE_DOWN_INIT } else { SUBTIMER_STATE_UP_INIT };
    }

    /// `Interface_SetSubTimerToFinalSecond`: a running sub timer to its last second (the
    /// marathon's, which counts up, to a second short of its limit).
    pub fn interface_set_sub_timer_to_final_second(&mut self) {
        let s = &mut self.save;
        if s.sub_timer_state != SUBTIMER_STATE_OFF {
            s.sub_timer_seconds = if s.get_event_inf(EVENTINF_MARATHON_ACTIVE) { MARATHON_TIME_LIMIT - 1 } else { 1 };
        }
    }

    /// `Interface_Draw`'s timers (under `pauseCtx->debugState == PAUSE_DEBUG_STATE_CLOSED`, which
    /// the port's pause menu always is): the sub timer's run out answered, then each timer's state,
    /// the beeps and the digits. What it leaves is drawn by `TimerStatics::draw`.
    pub fn interface_draw_timers(&mut self) {
        let mut t = self.interface_ctx.timers;
        t.drawn = None;
        // The sub timer's run out (the trade quest's), once its text waits: the respawn's
        // transition, B's item back, the spoiling items reverted.
        if self.save.sub_timer_state == SUBTIMER_STATE_RESPAWN && self.message_state() == crate::message::TEXT_STATE_EVENT {
            t.sub_timer_state_timer = 40;
            self.save.cutscene_index = 0;
            self.transition.trigger = crate::transition::TRANS_TRIGGER_START;
            self.transition.ty = crate::transition::TRANS_TYPE_FADE_WHITE;
            self.save.sub_timer_state = SUBTIMER_STATE_OFF;
            use crate::item::{ITEM_GIANTS_KNIFE, ITEM_SWORD_BIGGORON, ITEM_SWORD_KOKIRI, ITEM_SWORD_MASTER, slot};
            let b = self.save.equips.button_items[0];
            if b != ITEM_SWORD_KOKIRI && b != ITEM_SWORD_MASTER && b != ITEM_SWORD_BIGGORON && b != ITEM_GIANTS_KNIFE {
                // BTN_ENABLED (0): a button item stood in for the sword's.
                self.save.equips.button_items[0] = if self.save.button_status[0] != 0 { self.save.button_status[0] } else { ITEM_NONE };
            }
            for i in 0..crate::game_over::G_SPOILING_ITEMS.len() {
                // INV_CONTENT(ITEM_TRADE_ADULT) (the Pocket Egg's slot).
                if self.save.inventory.items[slot(crate::item::ITEM_POCKET_EGG)] == crate::game_over::G_SPOILING_ITEMS[i] {
                    self.save.event_inf[0] &= !INGO_RACE_BITS;
                    match self.entrance_by_name(SPOILING_ITEM_ENTRANCES[i]) {
                        Some(e) => self.transition.next_entrance_index = e,
                        None => log::error!("no entrance {}", SPOILING_ITEM_ENTRANCES[i]),
                    }
                    let revert = crate::game_over::G_SPOILING_ITEM_REVERTS[i];
                    self.save.inventory.items[slot(revert)] = revert;
                    for j in 1..4 {
                        if self.save.equips.button_items[j] == crate::game_over::G_SPOILING_ITEMS[i] {
                            // (Interface_LoadItemIcon1: the HUD draws the button's item.)
                            self.save.equips.button_items[j] = revert;
                        }
                    }
                }
            }
        }
        let state2 = self.player.and_then(|h| self.actors.get(h)).and_then(|p| p.as_player()).map(|p| p.state_flags2()).unwrap_or(0);
        let bowling_closed = self.scene_id == SCENE_BOMBCHU_BOWLING_ALLEY && self.flags.get_switch(0x38);
        let runs = !self.pause_ctx.is_paused()
            && self.game_over_ctx.state == crate::game_over::GAMEOVER_INACTIVE
            && self.msg_ctx.msg_mode == crate::message::MSGMODE_NONE
            && state2 & PLAYER_STATE2_24 == 0
            && self.transition.trigger == crate::transition::TRANS_TRIGGER_OFF
            && self.transition.mode == crate::transition::TRANS_MODE_OFF
            && !self.play_in_cs_mode()
            && !bowling_closed;
        if runs {
            self.timers_update(&mut t);
        }
        self.interface_ctx.timers = t;
    }

    /// The state machine (`switch (gSaveContext.timerState)`, the sub timer's in its default),
    /// then the digits.
    fn timers_update(&mut self, t: &mut TimerStatics) {
        let msg_length = self.msg_ctx.msg_length;
        let mut timer_id = TIMER_ID_MAIN;
        let s = &mut self.save;
        let row = hearts_row_y(s);
        match s.timer_state {
            TIMER_STATE_ENV_HAZARD_INIT => {
                t.timer_state_timer = 20;
                t.timer_next_second_timer = 20;
                s.timer_seconds = s.health >> 1;
                s.timer_state = TIMER_STATE_ENV_HAZARD_PREVIEW;
            }
            TIMER_STATE_ENV_HAZARD_PREVIEW => {
                t.timer_state_timer -= 1;
                if t.timer_state_timer == 0 {
                    t.timer_state_timer = 20;
                    s.timer_state = TIMER_STATE_ENV_HAZARD_MOVE;
                }
            }
            TIMER_STATE_DOWN_INIT | TIMER_STATE_UP_INIT => {
                t.timer_state_timer = 20;
                t.timer_next_second_timer = 20;
                s.timer_state = if s.timer_state == TIMER_STATE_DOWN_INIT { TIMER_STATE_DOWN_PREVIEW } else { TIMER_STATE_UP_PREVIEW };
            }
            TIMER_STATE_DOWN_PREVIEW | TIMER_STATE_UP_PREVIEW => {
                t.timer_state_timer -= 1;
                if t.timer_state_timer == 0 {
                    t.timer_state_timer = 20;
                    s.timer_state = if s.timer_state == TIMER_STATE_DOWN_PREVIEW { TIMER_STATE_DOWN_MOVE } else { TIMER_STATE_UP_MOVE };
                }
            }
            TIMER_STATE_ENV_HAZARD_MOVE | TIMER_STATE_DOWN_MOVE | TIMER_STATE_ENV_HAZARD_TICK | TIMER_STATE_DOWN_TICK => {
                if s.timer_state == TIMER_STATE_ENV_HAZARD_MOVE || s.timer_state == TIMER_STATE_DOWN_MOVE {
                    Self::timer_slide(s, TIMER_ID_MAIN, &mut t.timer_state_timer, row);
                    if t.timer_state_timer == 0 {
                        t.timer_state_timer = 20;
                        s.timer_x[TIMER_ID_MAIN] = 26;
                        s.timer_y[TIMER_ID_MAIN] = row;
                        s.timer_state = if s.timer_state == TIMER_STATE_ENV_HAZARD_MOVE { TIMER_STATE_ENV_HAZARD_TICK } else { TIMER_STATE_DOWN_TICK };
                    }
                }
                // (FALLTHROUGH into the tick.)
                if s.timer_state == TIMER_STATE_ENV_HAZARD_TICK || s.timer_state == TIMER_STATE_DOWN_TICK {
                    s.timer_y[TIMER_ID_MAIN] = row;
                }
                if s.timer_state >= TIMER_STATE_ENV_HAZARD_MOVE && msg_length == 0 {
                    t.timer_next_second_timer -= 1;
                    if t.timer_next_second_timer == 0 {
                        if s.timer_seconds != 0 {
                            s.timer_seconds -= 1;
                        }
                        t.timer_next_second_timer = 20;
                        if s.timer_seconds == 0 {
                            // Out of time.
                            s.timer_state = TIMER_STATE_STOP;
                            if t.env_hazard_active {
                                s.health = 0;
                                let damage = -(s.health as i32 + 2);
                                if let Some(f) = self.damage_player {
                                    f(self, damage);
                                }
                            }
                            t.env_hazard_active = false;
                        } else if self.save.timer_seconds > 60 {
                            // A beep at "xx:x1": every 10 seconds (sTimerDigits from the last frame).
                            if t.timer_digits[4] == 1 {
                                self.audio.play_sfx_centered(crate::audio::sfx::NA_SE_SY_MESSAGE_WOMAN);
                            }
                        } else if self.save.timer_seconds > 10 {
                            if t.timer_digits[4] % 2 != 0 {
                                self.audio.play_sfx_centered(crate::audio::sfx::NA_SE_SY_WARNING_COUNT_N);
                            }
                        } else {
                            self.audio.play_sfx_centered(crate::audio::sfx::NA_SE_SY_WARNING_COUNT_E);
                        }
                    }
                }
            }
            TIMER_STATE_UP_MOVE | TIMER_STATE_UP_TICK => {
                if s.timer_state == TIMER_STATE_UP_MOVE {
                    Self::timer_slide(s, TIMER_ID_MAIN, &mut t.timer_state_timer, row);
                    if t.timer_state_timer == 0 {
                        t.timer_state_timer = 20;
                        s.timer_x[TIMER_ID_MAIN] = 26;
                        s.timer_y[TIMER_ID_MAIN] = row;
                        s.timer_state = TIMER_STATE_UP_TICK;
                    }
                }
                if s.timer_state == TIMER_STATE_UP_TICK {
                    s.timer_y[TIMER_ID_MAIN] = row;
                }
                if s.timer_state >= TIMER_STATE_ENV_HAZARD_MOVE {
                    t.timer_next_second_timer -= 1;
                    if t.timer_next_second_timer == 0 {
                        s.timer_seconds += 1;
                        t.timer_next_second_timer = 20;
                        if s.timer_seconds == 3599 {
                            // 59:59.
                            t.timer_state_timer = 40;
                            s.timer_state = TIMER_STATE_UP_FREEZE;
                        } else {
                            // A beep every second.
                            self.audio.play_sfx_centered(crate::audio::sfx::NA_SE_SY_WARNING_COUNT_N);
                        }
                    }
                }
            }
            TIMER_STATE_STOP => {
                if s.sub_timer_state != SUBTIMER_STATE_OFF {
                    t.sub_timer_state_timer = 20;
                    t.sub_timer_next_second_timer = 20;
                    s.timer_x[TIMER_ID_SUB] = 140;
                    s.timer_y[TIMER_ID_SUB] = 80;
                    s.sub_timer_state = if s.sub_timer_state <= SUBTIMER_STATE_STOP { SUBTIMER_STATE_DOWN_PREVIEW } else { SUBTIMER_STATE_UP_PREVIEW };
                }
                s.timer_state = TIMER_STATE_OFF;
                // (FALLTHROUGH to TIMER_STATE_UP_FREEZE: nothing.)
            }
            TIMER_STATE_UP_FREEZE => {}
            _ => {
                // TIMER_STATE_OFF: the sub timer, only with the main one off.
                timer_id = TIMER_ID_SUB;
                self.sub_timer_update(t, row);
            }
        }
        let s = &self.save;
        if (s.timer_state != TIMER_STATE_OFF && s.timer_state != TIMER_STATE_STOP) || s.sub_timer_state != SUBTIMER_STATE_OFF {
            let d = &mut t.timer_digits;
            d[0] = 0;
            d[1] = 0;
            d[3] = 0;
            // Digit 10 is the colon.
            d[2] = 10;
            d[4] = if s.timer_state != TIMER_STATE_OFF { s.timer_seconds } else { s.sub_timer_seconds };
            while d[4] >= 60 {
                d[1] += 1;
                if d[1] >= 10 {
                    d[0] += 1;
                    d[1] -= 10;
                }
                d[4] -= 60;
            }
            while d[4] >= 10 {
                d[3] += 1;
                d[4] -= 10;
            }
            t.drawn = Some(timer_id);
        }
    }

    /// The sub timer's states (`switch (gSaveContext.subTimerState)`).
    fn sub_timer_update(&mut self, t: &mut TimerStatics, row: i16) {
        let s = &mut self.save;
        match s.sub_timer_state {
            SUBTIMER_STATE_DOWN_INIT | SUBTIMER_STATE_UP_INIT => {
                t.sub_timer_state_timer = 20;
                t.sub_timer_next_second_timer = 20;
                s.timer_x[TIMER_ID_SUB] = 140;
                s.timer_y[TIMER_ID_SUB] = 80;
                s.sub_timer_state = if s.sub_timer_state == SUBTIMER_STATE_DOWN_INIT { SUBTIMER_STATE_DOWN_PREVIEW } else { SUBTIMER_STATE_UP_PREVIEW };
            }
            SUBTIMER_STATE_DOWN_PREVIEW | SUBTIMER_STATE_UP_PREVIEW => {
                t.sub_timer_state_timer -= 1;
                if t.sub_timer_state_timer == 0 {
                    t.sub_timer_state_timer = 20;
                    s.sub_timer_state = if s.sub_timer_state == SUBTIMER_STATE_DOWN_PREVIEW { SUBTIMER_STATE_DOWN_MOVE } else { SUBTIMER_STATE_UP_MOVE };
                }
            }
            SUBTIMER_STATE_DOWN_MOVE | SUBTIMER_STATE_UP_MOVE | SUBTIMER_STATE_DOWN_TICK | SUBTIMER_STATE_UP_TICK => {
                if s.sub_timer_state == SUBTIMER_STATE_DOWN_MOVE || s.sub_timer_state == SUBTIMER_STATE_UP_MOVE {
                    Self::timer_slide(s, TIMER_ID_SUB, &mut t.sub_timer_state_timer, row);
                    if t.sub_timer_state_timer == 0 {
                        t.sub_timer_state_timer = 20;
                        s.timer_x[TIMER_ID_SUB] = 26;
                        s.timer_y[TIMER_ID_SUB] = row;
                        s.sub_timer_state = if s.sub_timer_state == SUBTIMER_STATE_DOWN_MOVE { SUBTIMER_STATE_DOWN_TICK } else { SUBTIMER_STATE_UP_TICK };
                    }
                }
                // (FALLTHROUGH into the tick.)
                if s.sub_timer_state == SUBTIMER_STATE_DOWN_TICK || s.sub_timer_state == SUBTIMER_STATE_UP_TICK {
                    s.timer_y[TIMER_ID_SUB] = row;
                }
                if s.sub_timer_state >= SUBTIMER_STATE_DOWN_MOVE {
                    t.sub_timer_next_second_timer -= 1;
                    if t.sub_timer_next_second_timer == 0 {
                        t.sub_timer_next_second_timer = 20;
                        if s.sub_timer_state == SUBTIMER_STATE_DOWN_TICK {
                            s.sub_timer_seconds -= 1;
                            // (OOT_VERSION >= PAL_1_0: `<= 0`.)
                            if s.sub_timer_seconds <= 0 {
                                // Out of time: the trade quest's respawn, except in Ganon's
                                // collapse with switch 0x37.
                                // SCENE_GANON_BOSS 0x4F, SCENE_GANONS_TOWER_COLLAPSE_EXTERIOR 0x1A,
                                // _INTERIOR 0x0E, SCENE_INSIDE_GANONS_CASTLE_COLLAPSE 0x0F.
                                const COLLAPSE: [u16; 4] = [0x4F, 0x1A, 0x0E, 0x0F];
                                if !self.flags.get_switch(0x37) || !COLLAPSE.contains(&self.scene_id) {
                                    t.sub_timer_state_timer = 40;
                                    self.save.sub_timer_state = SUBTIMER_STATE_RESPAWN;
                                    self.save.cutscene_index = 0;
                                    self.start_textbox(0x71B0, None);
                                    // Player_SetCsActionWithHaltedActors(play, NULL, PLAYER_CSACTION_8).
                                    self.player_set_cs_action_with_halted_actors(None, 8);
                                } else {
                                    t.sub_timer_state_timer = 40;
                                    self.save.sub_timer_state = SUBTIMER_STATE_STOP;
                                }
                            } else if self.save.sub_timer_seconds > 60 {
                                if t.timer_digits[4] == 1 {
                                    self.audio.play_sfx_centered(crate::audio::sfx::NA_SE_SY_MESSAGE_WOMAN);
                                }
                            } else if self.save.sub_timer_seconds > 10 {
                                if t.timer_digits[4] % 2 != 0 {
                                    self.audio.play_sfx_centered(crate::audio::sfx::NA_SE_SY_WARNING_COUNT_N);
                                }
                            } else {
                                self.audio.play_sfx_centered(crate::audio::sfx::NA_SE_SY_WARNING_COUNT_E);
                            }
                        } else {
                            // SUBTIMER_STATE_UP_TICK.
                            s.sub_timer_seconds += 1;
                            // The marathon man's race: called off after four minutes.
                            if s.get_event_inf(EVENTINF_MARATHON_ACTIVE) && s.sub_timer_seconds == MARATHON_TIME_LIMIT {
                                self.start_textbox(0x6083, None);
                                self.save.clear_event_inf(EVENTINF_MARATHON_ACTIVE);
                                self.save.sub_timer_state = SUBTIMER_STATE_OFF;
                            }
                        }
                        // A beep at the minute mark.
                        if self.save.sub_timer_seconds % 60 == 0 {
                            self.audio.play_sfx_centered(crate::audio::sfx::NA_SE_SY_WARNING_COUNT_N);
                        }
                    }
                }
            }
            SUBTIMER_STATE_STOP => {
                t.sub_timer_state_timer -= 1;
                if t.sub_timer_state_timer == 0 {
                    s.sub_timer_state = SUBTIMER_STATE_OFF;
                }
            }
            _ => {}
        }
    }

    /// `*_MOVE`'s slide: x and y a `1 / stateTimer` of the way to (26, the hearts' row) (s16
    /// division), then the state timer down.
    fn timer_slide(s: &mut SaveContext, id: usize, state_timer: &mut i16, row: i16) {
        let dx = (s.timer_x[id] - 26) / *state_timer;
        s.timer_x[id] -= dx;
        let dy = (s.timer_y[id] - row) / *state_timer;
        s.timer_y[id] -= dy;
        *state_timer -= 1;
    }
}

impl TimerStatics {
    /// The timer `interface_draw_timers` left: the clock (`Gfx_TextureIA8(gClockIconTex)` at
    /// (x, y + 2), prim white, env black), then the five digits (`Gfx_TextureI8` of
    /// `gCounterDigit0Tex + 8 * 16 * digit`, 10 the colon) at `timerDigitLeftPos`, red under 10
    /// seconds while counting (the main timer up to `TIMER_STATE_STOP`, the sub timer up to its
    /// respawn), else white (the main timer) or yellow (the sub timer).
    ///
    /// Each digit's rectangle is `sDigitWidths` wide and `VREG(42)` (250) tall at `VREG(43) << 1`
    /// (880/1024 texels a pixel): its 8 by 16 texels cover 9.3 by 18.6 pixels, and with no mask
    /// the tile clamps, repeating the digit's blank last row below. The port draws the texels'
    /// 9.3 by 18.6 (the C's rectangle cuts the last 0.3 off at 9 wide).
    pub fn draw(&self, save: &SaveContext, out: &mut Vec<Sprite>) {
        let Some(id) = self.drawn else { return };
        let (x, y) = (save.timer_x[id] as f32, save.timer_y[id] as f32);
        out.push(Sprite::rect(CLOCK_ICON, x, y + 2.0, x + 16.0, y + 18.0, Some([255, 255, 255, 255]), Some([0, 0, 0, 0])));
        let rgb = if save.timer_state != TIMER_STATE_OFF {
            if save.timer_seconds < 10 && save.timer_state <= TIMER_STATE_STOP { [255, 50, 0] } else { [255, 255, 255] }
        } else if save.sub_timer_seconds < 10 && save.sub_timer_state <= SUBTIMER_STATE_RESPAWN {
            [255, 50, 0]
        } else {
            [255, 255, 0]
        };
        let (w, h) = (8.0 / DIGIT_STEP, 16.0 / DIGIT_STEP);
        for (i, &d) in self.timer_digits.iter().enumerate() {
            let dx = x + TIMER_DIGIT_LEFT_POS[i] as f32;
            let _ = DIGIT_WIDTHS[i];
            out.push(Sprite::rect(digit_sprite(d.clamp(0, 10) as usize), dx, y, dx + w, y + h, Some([rgb[0], rgb[1], rgb[2], 255]), None));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_slide_divides_as_s16() {
        let mut s = SaveContext::default();
        s.timer_x[TIMER_ID_MAIN] = 140;
        s.timer_y[TIMER_ID_MAIN] = 80;
        let mut st = 20;
        PlayState::timer_slide(&mut s, TIMER_ID_MAIN, &mut st, 46);
        // (140 - 26) / 20 = 5, (80 - 46) / 20 = 1.
        assert_eq!((s.timer_x[0], s.timer_y[0], st), (135, 79, 19));
        for _ in 0..19 {
            PlayState::timer_slide(&mut s, TIMER_ID_MAIN, &mut st, 46);
        }
        // The last step's divisor is 1: there.
        assert_eq!((s.timer_x[0], s.timer_y[0], st), (26, 46, 0));
    }
}
