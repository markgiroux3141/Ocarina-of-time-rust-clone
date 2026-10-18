//! The magic meter (`z_parameter.c`): `Magic_Fill`, `Magic_Reset`, `Magic_RequestChange`,
//! `Magic_Update` (and `Magic_DrawMeter`, in `crate::interface`).
//!
//! - **The state** is `gSaveContext`'s, outside the save (not written to SRAM):
//!   `SaveContext::magic_state`, `prev_magic_state`, `magic_capacity`, `magic_fill_target`,
//!   `magic_target`; the save's own `magic_level`, `magic`, `is_magic_acquired`,
//!   `is_double_magic_acquired`.
//! - **The frame:** `Interface_Update` starts the meter (`magicLevel` from 0 with magic acquired:
//!   `MAGIC_STATE_STEP_CAPACITY`) and runs `Magic_Update` outside the pause menu, a message, a
//!   transition and a game over, and in a cutscene only while Player isn't in its cutscene mode
//!   (`crate::interface::InterfaceContext::update`).
//! - **The border's flash** (`sMagicBorderR`, `G`, `B`, `sMagicBorderRatio`, `sMagicBorderStep`)
//!   are `z_parameter.c`'s statics: [`MagicStatics`], kept by the interface and carried over scene
//!   changes.
//! - The actors ask through the play state ([`PlayState::magic_request_change`] and the rest).
//!
//! The Lens of Truth isn't ported: nothing asks for `MAGIC_CONSUME_LENS` yet, and
//! `actorCtx.lensActive` (`ActorContext::lens_active`) stays false; its state is ported as written.

use crate::audio::GameAudio;
use crate::audio::sfx::{NA_SE_SY_ERROR, SFX_FLAG};
use crate::play::PlayState;
use crate::save::{MAGIC_NORMAL_METER, SaveContext};

// MagicState (save.h).
/// Regular play.
pub const MAGIC_STATE_IDLE: i16 = 0x0;
/// Sets the speed at which the border flashes.
pub const MAGIC_STATE_CONSUME_SETUP: i16 = 0x1;
/// Consumes magic until the target is reached or none is left.
pub const MAGIC_STATE_CONSUME: i16 = 0x2;
/// Flashes the border (and freezes Dark Link).
pub const MAGIC_STATE_METER_FLASH_1: i16 = 0x3;
/// Flashes the border and draws the yellow preview of the magic to be consumed.
pub const MAGIC_STATE_METER_FLASH_2: i16 = 0x4;
/// Resets the colours and goes idle.
pub const MAGIC_STATE_RESET: i16 = 0x5;
/// Flashes the border, nothing more.
pub const MAGIC_STATE_METER_FLASH_3: i16 = 0x6;
/// Magic slowly consumed by the Lens of Truth.
pub const MAGIC_STATE_CONSUME_LENS: i16 = 0x7;
/// Steps `magicCapacity` to the full capacity.
pub const MAGIC_STATE_STEP_CAPACITY: i16 = 0x8;
/// Adds magic until `magicFillTarget`.
pub const MAGIC_STATE_FILL: i16 = 0x9;
/// Adds the requested magic.
pub const MAGIC_STATE_ADD: i16 = 0xA;

// MagicChangeType (save.h).
/// Consume the magic at once, without the preview.
pub const MAGIC_CONSUME_NOW: i16 = 0;
/// Set the target but wait to consume, no preview (unused).
pub const MAGIC_CONSUME_WAIT_NO_PREVIEW: i16 = 1;
/// The same as `MAGIC_CONSUME_NOW` (unused).
pub const MAGIC_CONSUME_NOW_ALT: i16 = 2;
/// The Lens of Truth's.
pub const MAGIC_CONSUME_LENS: i16 = 3;
/// Set the target but wait to consume, the yellow preview drawn.
pub const MAGIC_CONSUME_WAIT_PREVIEW: i16 = 4;
/// Set a target to add to.
pub const MAGIC_ADD: i16 = 5;

/// `MAGIC_DOUBLE_METER` (`save.h`).
pub const MAGIC_DOUBLE_METER: i8 = 2 * MAGIC_NORMAL_METER;

/// `NA_SE_SY_GAUGE_UP` (`systembank_table.h`: 0x481F).
pub const NA_SE_SY_GAUGE_UP: u16 = 0x481F;
/// `NA_SE_SY_GLASSMODE_OFF` (`systembank_table.h`: 0x482B).
pub const NA_SE_SY_GLASSMODE_OFF: u16 = 0x482B;

/// `sMagicBorderColors` (`Magic_Update`): white, grey, and two unused yellows.
const MAGIC_BORDER_COLORS: [[i16; 3]; 4] = [[255, 255, 255], [150, 150, 150], [255, 255, 150], [255, 255, 50]];
/// `sMagicBorderIndices`.
const MAGIC_BORDER_INDICES: [usize; 4] = [0, 1, 1, 0];
/// `YREG(40)` to `YREG(43)` (`z_construct.c`'s `Regs_InitDataImpl`): the border's flash ratio
/// by step.
pub const YREG_40: [i16; 4] = [2, 1, 2, 1];

/// `z_parameter.c`'s magic statics: the border's colour (`sMagicBorderR`, `G`, `B`, which
/// `Magic_DrawMeter` draws it with), `sMagicBorderRatio` and `sMagicBorderStep` (`Magic_Update`'s
/// own statics). The code segment's: they live as long as the game.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MagicStatics {
    pub border_r: i16,
    pub border_g: i16,
    pub border_b: i16,
    pub border_ratio: i16,
    pub border_step: i16,
}

impl Default for MagicStatics {
    fn default() -> MagicStatics {
        MagicStatics { border_r: 255, border_g: 255, border_b: 255, border_ratio: 2, border_step: 1 }
    }
}

/// What `Magic_Update` reads of the rest of play, beyond the save.
#[derive(Debug, Clone, Copy, Default)]
pub struct MagicFrame {
    /// `gSaveContext.gameMode == GAMEMODE_NORMAL && !IS_CUTSCENE_LAYER`: the fill is heard.
    pub fill_sfx: bool,
    /// The lens's checks (`MAGIC_STATE_CONSUME_LENS`): `!IS_PAUSED`, no message, no game over, no
    /// transition, `!Play_InCsMode`.
    pub lens_frame: bool,
    /// `Player_GetEnvironmentalHazard` is under water (`PLAYER_ENV_HAZARD_UNDERWATER_FLOOR` to
    /// `_FREE`).
    pub underwater: bool,
}

/// `Magic_Fill`: with magic, fill to the whole meter (`magicFillTarget`: one or two
/// `MAGIC_NORMAL_METER`s), coming back to the state it was in after.
pub fn magic_fill(save: &mut SaveContext) {
    if save.is_magic_acquired {
        save.prev_magic_state = save.magic_state;
        save.magic_fill_target = (save.is_double_magic_acquired as i16 + 1) * MAGIC_NORMAL_METER as i16;
        save.magic_state = MAGIC_STATE_FILL;
    }
}

/// `Magic_Reset`: back to idle (the border's colours reset), unless the meter is growing or
/// filling; an addition in progress is remembered as the state to come back to.
pub fn magic_reset(save: &mut SaveContext) {
    if save.magic_state != MAGIC_STATE_STEP_CAPACITY && save.magic_state != MAGIC_STATE_FILL {
        if save.magic_state == MAGIC_STATE_ADD {
            save.prev_magic_state = save.magic_state;
        }
        save.magic_state = MAGIC_STATE_RESET;
    }
}

/// `Magic_RequestChange`: ask to add `amount` (`MAGIC_ADD`) or consume it (the `MAGIC_CONSUME_*`
/// types). False (with `NA_SE_SY_ERROR` where the C plays it) when there's no magic, too little,
/// or the meter is busy. A consumption while the lens is on turns it off. `lens_active` is
/// `actorCtx.lensActive`, `lens_timer` `interfaceCtx->lensMagicConsumptionTimer`.
pub fn magic_request_change(save: &mut SaveContext, audio: &mut GameAudio, lens_active: &mut bool, lens_timer: &mut i16, amount: i16, ty: i16) -> bool {
    if !save.is_magic_acquired {
        return false;
    }
    if ty != MAGIC_ADD && (save.magic as i16 - amount) < 0 {
        if save.magic_capacity != 0 {
            audio.play_sfx_centered(NA_SE_SY_ERROR);
        }
        return false;
    }
    match ty {
        MAGIC_CONSUME_NOW | MAGIC_CONSUME_NOW_ALT | MAGIC_CONSUME_WAIT_NO_PREVIEW | MAGIC_CONSUME_WAIT_PREVIEW => {
            if save.magic_state == MAGIC_STATE_IDLE || save.magic_state == MAGIC_STATE_CONSUME_LENS {
                if save.magic_state == MAGIC_STATE_CONSUME_LENS {
                    *lens_active = false;
                }
                save.magic_target = save.magic as i16 - amount;
                save.magic_state = match ty {
                    MAGIC_CONSUME_WAIT_NO_PREVIEW => MAGIC_STATE_METER_FLASH_3,
                    MAGIC_CONSUME_WAIT_PREVIEW => MAGIC_STATE_METER_FLASH_2,
                    _ => MAGIC_STATE_CONSUME_SETUP,
                };
                true
            } else {
                audio.play_sfx_centered(NA_SE_SY_ERROR);
                false
            }
        }
        MAGIC_CONSUME_LENS => {
            if save.magic_state == MAGIC_STATE_IDLE {
                if save.magic != 0 {
                    *lens_timer = 80;
                    save.magic_state = MAGIC_STATE_CONSUME_LENS;
                    true
                } else {
                    false
                }
            } else {
                save.magic_state == MAGIC_STATE_CONSUME_LENS
            }
        }
        MAGIC_ADD => magic_request_add(save, amount),
        _ => false,
    }
}

/// `Magic_RequestChange(play, amount, MAGIC_ADD)`, which touches neither the sound nor the lens:
/// the target `amount` up, at most the capacity. False without magic or above the capacity.
pub fn magic_request_add(save: &mut SaveContext, amount: i16) -> bool {
    if !save.is_magic_acquired {
        return false;
    }
    if save.magic as i16 <= save.magic_capacity {
        save.magic_target = save.magic as i16 + amount;
        if save.magic_target >= save.magic_capacity {
            save.magic_target = save.magic_capacity;
        }
        save.magic_state = MAGIC_STATE_ADD;
        true
    } else {
        false
    }
}

/// The border's flash, one step towards `sMagicBorderColors[sMagicBorderIndices[step]]`
/// (`Magic_Update`'s shared tail of the flashing states).
fn border_flash(st: &mut MagicStatics) {
    let temp = MAGIC_BORDER_INDICES[st.border_step as usize];
    let c = MAGIC_BORDER_COLORS[temp];
    let ratio = st.border_ratio;
    let change_r = (st.border_r - c[0]).abs() / ratio;
    let change_g = (st.border_g - c[1]).abs() / ratio;
    let change_b = (st.border_b - c[2]).abs() / ratio;
    if st.border_r >= c[0] {
        st.border_r -= change_r;
    } else {
        st.border_r += change_r;
    }
    if st.border_g >= c[1] {
        st.border_g -= change_g;
    } else {
        st.border_g += change_g;
    }
    if st.border_b >= c[2] {
        st.border_b -= change_b;
    } else {
        st.border_b += change_b;
    }
    st.border_ratio -= 1;
    if st.border_ratio == 0 {
        st.border_r = c[0];
        st.border_g = c[1];
        st.border_b = c[2];
        st.border_ratio = YREG_40[st.border_step as usize];
        st.border_step += 1;
        if st.border_step >= 4 {
            st.border_step = 0;
        }
    }
}

/// `Magic_Update`: the meter's state machine, a frame.
#[allow(clippy::too_many_arguments)]
pub fn magic_update(save: &mut SaveContext, st: &mut MagicStatics, audio: &mut GameAudio, lens_active: &mut bool, lens_timer: &mut i16, f: &MagicFrame) {
    match save.magic_state {
        MAGIC_STATE_STEP_CAPACITY => {
            // Step magicCapacity to the capacity magicLevel gives (the meter's width), then fill.
            let temp = save.magic_level as i16 * MAGIC_NORMAL_METER as i16;
            if temp != save.magic_capacity {
                if save.magic_capacity < temp {
                    save.magic_capacity += 8;
                    if save.magic_capacity > temp {
                        save.magic_capacity = temp;
                    }
                } else {
                    save.magic_capacity -= 8;
                    if save.magic_capacity <= temp {
                        save.magic_capacity = temp;
                    }
                }
            } else {
                save.magic_state = MAGIC_STATE_FILL;
            }
        }
        MAGIC_STATE_FILL => {
            save.magic = save.magic.wrapping_add(4);
            if f.fill_sfx {
                audio.play_sfx_centered(NA_SE_SY_GAUGE_UP - SFX_FLAG);
            }
            if save.magic as i16 >= save.magic_fill_target {
                save.magic = save.magic_fill_target as i8;
                save.magic_state = save.prev_magic_state;
                save.prev_magic_state = MAGIC_STATE_IDLE;
            }
        }
        MAGIC_STATE_CONSUME_SETUP => {
            st.border_ratio = 2;
            save.magic_state = MAGIC_STATE_CONSUME;
        }
        MAGIC_STATE_CONSUME | MAGIC_STATE_METER_FLASH_1 | MAGIC_STATE_METER_FLASH_2 | MAGIC_STATE_METER_FLASH_3 => {
            if save.magic_state == MAGIC_STATE_CONSUME {
                save.magic = save.magic.wrapping_sub(2);
                if save.magic <= 0 {
                    save.magic = 0;
                    save.magic_state = MAGIC_STATE_METER_FLASH_1;
                    st.border_r = 255;
                    st.border_g = 255;
                    st.border_b = 255;
                } else if save.magic as i16 == save.magic_target {
                    save.magic_state = MAGIC_STATE_METER_FLASH_1;
                    st.border_r = 255;
                    st.border_g = 255;
                    st.border_b = 255;
                }
                // FALLTHROUGH: the border flashes while magic is consumed.
            }
            border_flash(st);
        }
        MAGIC_STATE_RESET => {
            st.border_r = 255;
            st.border_g = 255;
            st.border_b = 255;
            save.magic_state = MAGIC_STATE_IDLE;
        }
        MAGIC_STATE_CONSUME_LENS => {
            // Slowly consume magic while the lens is on.
            if f.lens_frame {
                let lens_held = save.equips.button_items[1..4].contains(&crate::item::ITEM_LENS_OF_TRUTH);
                if save.magic == 0 || f.underwater || !lens_held || !*lens_active {
                    *lens_active = false;
                    audio.play_sfx_centered(NA_SE_SY_GLASSMODE_OFF);
                    save.magic_state = MAGIC_STATE_IDLE;
                    st.border_r = 255;
                    st.border_g = 255;
                    st.border_b = 255;
                    return;
                }
                *lens_timer -= 1;
                if *lens_timer == 0 {
                    save.magic -= 1;
                    *lens_timer = 80;
                }
            }
            border_flash(st);
        }
        MAGIC_STATE_ADD => {
            save.magic = save.magic.wrapping_add(4);
            audio.play_sfx_centered(NA_SE_SY_GAUGE_UP - SFX_FLAG);
            if save.magic as i16 >= save.magic_target {
                save.magic = save.magic_target as i8;
                save.magic_state = save.prev_magic_state;
                save.prev_magic_state = MAGIC_STATE_IDLE;
            }
        }
        _ => save.magic_state = MAGIC_STATE_IDLE,
    }
}

impl PlayState {
    /// `Magic_Fill(play)`.
    pub fn magic_fill(&mut self) {
        magic_fill(&mut self.save);
    }

    /// `Magic_Reset(play)`.
    pub fn magic_reset(&mut self) {
        magic_reset(&mut self.save);
    }

    /// `Magic_RequestChange(play, amount, type)`.
    pub fn magic_request_change(&mut self, amount: i16, ty: i16) -> bool {
        magic_request_change(&mut self.save, &mut self.audio, &mut self.actors.lens_active, &mut self.interface_ctx.lens_magic_consumption_timer, amount, ty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn save() -> SaveContext {
        let mut s = SaveContext::debug(0, false, 0);
        s.magic_fill_target = MAGIC_NORMAL_METER as i16;
        s
    }

    fn run(s: &mut SaveContext, st: &mut MagicStatics, frames: usize) {
        let mut audio = GameAudio::default();
        let (mut lens, mut timer) = (false, 0);
        for _ in 0..frames {
            magic_update(s, st, &mut audio, &mut lens, &mut timer, &MagicFrame { fill_sfx: true, ..Default::default() });
        }
    }

    #[test]
    fn the_meter_grows_then_fills_to_the_saved_amount() {
        // Interface_Update's start with magic acquired and magicLevel 0 (a file loaded, the map
        // select's): level 1, MAGIC_STATE_STEP_CAPACITY; the capacity by 8 a frame to 0x30 (six
        // frames), a frame to see it's there, then 4 a frame to magicFillTarget (12 frames).
        let mut s = save();
        let mut st = MagicStatics::default();
        assert_eq!((s.magic, s.magic_level, s.magic_capacity), (0, 0, 0));
        s.magic_level = s.is_double_magic_acquired as i8 + 1;
        s.magic_state = MAGIC_STATE_STEP_CAPACITY;
        run(&mut s, &mut st, 6);
        assert_eq!((s.magic_capacity, s.magic_state), (0x30, MAGIC_STATE_STEP_CAPACITY));
        run(&mut s, &mut st, 1);
        assert_eq!(s.magic_state, MAGIC_STATE_FILL);
        run(&mut s, &mut st, 11);
        assert_eq!((s.magic, s.magic_state), (44, MAGIC_STATE_FILL));
        run(&mut s, &mut st, 1);
        assert_eq!((s.magic, s.magic_state, s.prev_magic_state), (0x30, MAGIC_STATE_IDLE, MAGIC_STATE_IDLE));
    }

    #[test]
    fn a_consumption_steps_down_by_two_and_flashes() {
        let mut s = save();
        s.magic = 0x30;
        s.magic_capacity = 0x30;
        let mut st = MagicStatics::default();
        let mut audio = GameAudio::default();
        let (mut lens, mut timer) = (false, 0);
        // Too much: refused, the error heard (the capacity isn't 0).
        assert!(!magic_request_change(&mut s, &mut audio, &mut lens, &mut timer, 0x31, MAGIC_CONSUME_NOW));
        assert!(magic_request_change(&mut s, &mut audio, &mut lens, &mut timer, 12, MAGIC_CONSUME_NOW));
        assert_eq!((s.magic_state, s.magic_target), (MAGIC_STATE_CONSUME_SETUP, 36));
        // Busy: refused.
        assert!(!magic_request_change(&mut s, &mut audio, &mut lens, &mut timer, 2, MAGIC_CONSUME_NOW));
        run(&mut s, &mut st, 1);
        assert_eq!((s.magic_state, st.border_ratio), (MAGIC_STATE_CONSUME, 2));
        // 48 to 36 by 2: six frames; the border towards grey (sMagicBorderIndices[1]) by halves.
        run(&mut s, &mut st, 1);
        assert_eq!((s.magic, st.border_r, st.border_ratio), (46, 203, 1));
        run(&mut s, &mut st, 5);
        assert_eq!((s.magic, s.magic_state), (36, MAGIC_STATE_METER_FLASH_1));
        // Magic_Reset: white again, idle a frame later.
        magic_reset(&mut s);
        run(&mut s, &mut st, 1);
        assert_eq!((s.magic_state, st.border_r, st.border_g, st.border_b), (MAGIC_STATE_IDLE, 255, 255, 255));
    }

    #[test]
    fn magic_fill_and_add_come_back_to_the_state_before() {
        let mut s = save();
        s.magic = 8;
        s.magic_capacity = 0x30;
        let mut st = MagicStatics::default();
        s.magic_state = MAGIC_STATE_METER_FLASH_1;
        magic_fill(&mut s);
        assert_eq!((s.magic_state, s.prev_magic_state, s.magic_fill_target), (MAGIC_STATE_FILL, MAGIC_STATE_METER_FLASH_1, 0x30));
        run(&mut s, &mut st, 10);
        assert_eq!((s.magic, s.magic_state), (0x30, MAGIC_STATE_METER_FLASH_1));
        // Magic_RequestChange(12, MAGIC_ADD) at the top: the target is the capacity.
        let mut audio = GameAudio::default();
        let (mut lens, mut timer) = (false, 0);
        s.magic_state = MAGIC_STATE_IDLE;
        assert!(magic_request_change(&mut s, &mut audio, &mut lens, &mut timer, 12, MAGIC_ADD));
        assert_eq!(s.magic_target, 0x30);
        // Without magic, nothing.
        s.is_magic_acquired = false;
        assert!(!magic_request_change(&mut s, &mut audio, &mut lens, &mut timer, 12, MAGIC_ADD));
        s.magic_state = MAGIC_STATE_ADD;
        magic_fill(&mut s);
        assert_eq!(s.magic_state, MAGIC_STATE_ADD);
    }
}
