//! `z_game_over.c`: what follows Link's death. Player's `func_80836448` starts it
//! (`GAMEOVER_DEATH_START`, or `GAMEOVER_REVIVE_START` with a fairy in a bottle), and
//! `Play_Update` runs `GameOver_Update` in `Message_Update`'s place while it's on:
//!
//! - **Death:** the timers and the interface are reset, the game over lights come up around
//!   Link (`Environment_InitGameOverLights`), and once Player has fallen
//!   (`GAMEOVER_DEATH_WAIT_GROUND`, which Player's `func_80843AE8` ends) a second later the game
//!   over menu opens (`PAUSE_STATE_GAME_OVER_START`, `crate::kaleido`).
//! - **Revival:** the lights come up, the fairy rises out of Link, then the lights fade out
//!   (`Environment_FadeOutGameOverLights`) and Player gets up.
//!
//! The rumble (`Rumble_Request`, `Rumble_Reset`) isn't ported.

use crate::play::PlayState;

// `GameOverState` (`game_over.h`).
pub const GAMEOVER_INACTIVE: u16 = 0;
pub const GAMEOVER_DEATH_START: u16 = 1;
/// Waiting for Link to fall and hit the ground.
pub const GAMEOVER_DEATH_WAIT_GROUND: u16 = 2;
/// Waiting a second before the game over menu.
pub const GAMEOVER_DEATH_DELAY_MENU: u16 = 3;
/// Nothing while the game over menu runs (`crate::kaleido`).
pub const GAMEOVER_DEATH_MENU: u16 = 4;
pub const GAMEOVER_REVIVE_START: u16 = 20;
pub const GAMEOVER_REVIVE_RUMBLE: u16 = 21;
/// Waiting for Link to fall and hit the ground.
pub const GAMEOVER_REVIVE_WAIT_GROUND: u16 = 22;
/// Waiting for the fairy to rise all the way out of Link's body.
pub const GAMEOVER_REVIVE_WAIT_FAIRY: u16 = 23;
/// The game over lights fade out as Link is revived and gets up.
pub const GAMEOVER_REVIVE_FADE_OUT: u16 = 24;

/// `GameOverContext`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GameOverContext {
    pub state: u16,
}

/// `gSpoilingItems` and `gSpoilingItemReverts` (`z_parameter.c`): the adult trade items that
/// spoil when Link dies, and what they go back to (`ITEM_ODD_MUSHROOM` 0x30, `ITEM_EYEBALL_FROG`
/// 0x35, `ITEM_EYE_DROPS` 0x36; `ITEM_COJIRO` 0x2F, `ITEM_PRESCRIPTION` 0x34, `item.h`).
const G_SPOILING_ITEMS: [u8; 3] = [0x30, 0x35, 0x36];
const G_SPOILING_ITEM_REVERTS: [u8; 3] = [0x2F, 0x34, 0x34];

/// `BTN_ENABLED` (`save.h`).
const BTN_ENABLED: u8 = 0;

impl PlayState {
    /// `GameOver_Init` (from `Play_Init`).
    pub fn game_over_init(&mut self) {
        self.game_over_ctx.state = GAMEOVER_INACTIVE;
    }

    /// `GameOver_FadeInLights` (`Play_DrawOverlayElements`, while a game over is on): the
    /// lights brighten from Link's fall until the revival's fade out.
    pub fn game_over_fade_in_lights(&mut self) {
        let s = self.game_over_ctx.state;
        if (GAMEOVER_DEATH_WAIT_GROUND..GAMEOVER_REVIVE_START).contains(&s) || (GAMEOVER_REVIVE_RUMBLE..GAMEOVER_REVIVE_FADE_OUT).contains(&s) {
            self.environment_fade_in_game_over_lights();
        }
    }

    /// `GameOver_Update`.
    pub fn game_over_update(&mut self) {
        use crate::item::{ITEM_GIANTS_KNIFE, ITEM_NONE, ITEM_POCKET_EGG, ITEM_SWORD_BIGGORON, ITEM_SWORD_KOKIRI, ITEM_SWORD_MASTER, slot};
        match self.game_over_ctx.state {
            GAMEOVER_DEATH_START => {
                self.with_msg(|m, f| m.close_textbox(f.audio));
                // (gSaveContext.timerState, subTimerState: no timers are ported.)
                // CLEAR_EVENTINF(EVENTINF_MARATHON_ACTIVE): eventInf is cleared whole below.
                let save = &mut self.save;
                // The spoiling trade items go back, and so do the C buttons holding them.
                for i in 0..G_SPOILING_ITEMS.len() {
                    if save.inventory.items[slot(ITEM_POCKET_EGG)] == G_SPOILING_ITEMS[i] {
                        let revert = G_SPOILING_ITEM_REVERTS[i];
                        save.inventory.items[slot(revert)] = revert;
                        for j in 1..save.equips.button_items.len() {
                            if save.equips.button_items[j] == G_SPOILING_ITEMS[i] {
                                // (Interface_LoadItemIcon1: the icons are drawn from the item.)
                                save.equips.button_items[j] = revert;
                            }
                        }
                    }
                }
                // "Temporary B": B gets back what it had before (buttonStatus[0] keeps it).
                let b = save.equips.button_items[0];
                if b != ITEM_SWORD_KOKIRI && b != ITEM_SWORD_MASTER && b != ITEM_SWORD_BIGGORON && b != ITEM_GIANTS_KNIFE {
                    save.equips.button_items[0] = if save.button_status[0] != BTN_ENABLED { save.button_status[0] } else { ITEM_NONE };
                }
                // (nayrusLoveTimer = 2000 from PAL 1.1 on: magic isn't ported.)
                save.navi_timer = 0;
                save.seq_id = crate::audio::NA_BGM_DISABLED as u8;
                save.nature_ambience_id = crate::audio::NATURE_ID_DISABLED;
                save.event_inf = [0; 4];
                save.button_status = [BTN_ENABLED; 5];
                save.force_rising_button_alphas = 0;
                save.next_hud_visibility_mode = 0;
                save.hud_visibility_mode = 0;
                save.hud_visibility_mode_timer = 0;
                self.environment_init_game_over_lights();
                self.game_over_timer = 20;
                // Rumble_Request(R_GAME_OVER_RUMBLE_*): not ported.
                self.game_over_ctx.state = GAMEOVER_DEATH_WAIT_GROUND;
            }
            GAMEOVER_DEATH_WAIT_GROUND => {}
            GAMEOVER_DEATH_DELAY_MENU => {
                self.game_over_timer -= 1;
                if self.game_over_timer == 0 {
                    self.pause_ctx.state = crate::kaleido::PAUSE_STATE_GAME_OVER_START;
                    self.game_over_ctx.state += 1;
                    // (Rumble_Reset.)
                }
            }
            GAMEOVER_REVIVE_START => {
                self.game_over_ctx.state += 1;
                self.game_over_timer = 0;
                self.environment_init_game_over_lights();
                self.letterbox.set_size_target(32);
            }
            GAMEOVER_REVIVE_RUMBLE => {
                self.game_over_timer = 50;
                self.game_over_ctx.state += 1;
                // Rumble_Request: not ported.
            }
            GAMEOVER_REVIVE_WAIT_GROUND => {
                self.game_over_timer -= 1;
                if self.game_over_timer == 0 {
                    self.game_over_timer = 64;
                    self.game_over_ctx.state += 1;
                }
            }
            GAMEOVER_REVIVE_WAIT_FAIRY => {
                self.game_over_timer -= 1;
                if self.game_over_timer == 0 {
                    self.game_over_timer = 50;
                    self.game_over_ctx.state += 1;
                }
            }
            GAMEOVER_REVIVE_FADE_OUT => {
                self.environment_fade_out_game_over_lights();
                self.game_over_timer -= 1;
                if self.game_over_timer == 0 {
                    self.game_over_ctx.state = GAMEOVER_INACTIVE;
                }
            }
            _ => {}
        }
    }
}
