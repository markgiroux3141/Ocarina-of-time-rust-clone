//! The pause menu's game over screens, as a stand-in (docs/adr/0032-damage-death-and-the-game-over-stand-in.md).
//!
//! The pause menu (`ovl_kaleido_scope`, about 7,700 lines) isn't ported. When `GameOver_Update`
//! opens its game over screens (`PAUSE_STATE_GAME_OVER_START`), this module runs their states as
//! `KaleidoScopeCall_Update` and `KaleidoScope_Update` run them, with the C's timers, inputs and
//! effects:
//!
//! - the pause background's prerender (`R_PAUSE_BG_PRERENDER_STATE`, stepped where `Play_Draw`
//!   steps it);
//! - "GAME OVER" fading in (`PAUSE_STATE_GAME_OVER_SHOW_MESSAGE`, 30 frames), the 40 frames' wait,
//!   the window turning in (`PAUSE_STATE_GAME_OVER_SHOW_WINDOW`, until `promptPitch` passes -628),
//!   and `deaths` counted;
//! - "Save?" (`PAUSE_STATE_GAME_OVER_SAVE_PROMPT`): A on Yes saves (`Play_SaveSceneFlags`,
//!   `savedSceneId`; writing the save to SRAM, `Sram_WriteSave`, isn't ported), on No goes on;
//! - "Continue?" (`PAUSE_STATE_GAME_OVER_CONTINUE_PROMPT`): A or Start; Yes respawns at the last
//!   entrance (a boss room's at its dungeon's), No would go back to the title screen (not
//!   ported: it's logged and respawns too);
//! - the fade to black (`interfaceCtx->unk_244`), then the respawn.
//!
//! The stick moves the prompts' cursor (`KaleidoScope_UpdatePrompt`, in the pause menu's draw).
//! Nothing of the menu is drawn: the game over message, the window and the prompts
//! (`KaleidoScope_DrawGameOver`, `KaleidoScope_DrawPages`) need the pause menu's textures and
//! pages. The scene stays as Link fell, frozen, and the fade to black is drawn.

use crate::play::PlayState;

// `PauseState` (`pause.h`): the game over ones.
pub const PAUSE_STATE_OFF: u16 = 0;
pub const PAUSE_STATE_GAME_OVER_START: u16 = 8;
pub const PAUSE_STATE_GAME_OVER_WAIT_BG_PRERENDER: u16 = 9;
pub const PAUSE_STATE_GAME_OVER_INIT: u16 = 10;
pub const PAUSE_STATE_GAME_OVER_SHOW_MESSAGE: u16 = 11;
pub const PAUSE_STATE_GAME_OVER_WINDOW_DELAY: u16 = 12;
/// The window turns in.
pub const PAUSE_STATE_GAME_OVER_SHOW_WINDOW: u16 = 13;
/// "Would you like to save?", and the choice.
pub const PAUSE_STATE_GAME_OVER_SAVE_PROMPT: u16 = 14;
/// "Game saved.", until the delay or a button.
pub const PAUSE_STATE_GAME_OVER_SAVED: u16 = 15;
/// "Continue playing?"
pub const PAUSE_STATE_GAME_OVER_CONTINUE_PROMPT: u16 = 16;
/// The fade out, then the choice.
pub const PAUSE_STATE_GAME_OVER_FINISH: u16 = 17;
/// `PAUSE_DEBUG_STATE_CLOSED`.
pub const PAUSE_DEBUG_STATE_CLOSED: u16 = 0;

// `PauseBgPreRenderState` (`play_state.h`).
pub const PAUSE_BG_PRERENDER_OFF: u8 = 0;
pub const PAUSE_BG_PRERENDER_SETUP: u8 = 1;
pub const PAUSE_BG_PRERENDER_PROCESS: u8 = 2;
pub const PAUSE_BG_PRERENDER_READY: u8 = 3;
pub const PAUSE_BG_PRERENDER_MAX: u8 = 4;

/// `R_PAUSE_UI_ANIMS_DURATION` (`WREG(6)`, 8 from `Regs_InitDataImpl`, `z_construct.c`) and
/// `WREG(4)` (8): the window's turn.
const R_PAUSE_UI_ANIMS_DURATION: f32 = 8.0;
/// `YREG(8)`'s part isn't needed: the save prompt's closing isn't a game over state.
/// `sDelayTimer` after saving on the GameCube versions (`PLATFORM_GC`).
const SAVED_DELAY: i16 = 3;

/// `PauseContext`: what the game over screens use of it, with `KaleidoScope_Update`'s statics.
#[derive(Debug, Clone, PartialEq)]
pub struct PauseContext {
    pub state: u16,
    pub debug_state: u16,
    /// `promptChoice`: 0 Yes, 4 No.
    pub prompt_choice: u16,
    /// `promptPitch`: the window's turn.
    pub prompt_pitch: f32,
    /// `R_PAUSE_BG_PRERENDER_STATE`.
    pub bg_prerender_state: u8,
    /// `KaleidoScope_Update`'s `sDelayTimer` and `D_8082B260` (the game over message's fade and
    /// wait).
    pub delay_timer: i16,
    pub d_8082b260: i16,
    /// `D_8082AB8C` .. `D_8082ABA4`: the message's two colours (prim RGBA, env RGB) as the fade
    /// steps them; kept, not drawn.
    pub message_colors: [i16; 7],
}

impl Default for PauseContext {
    fn default() -> PauseContext {
        // KaleidoScope_Update's statics: sDelayTimer = 10, D_8082B260 = 0.
        PauseContext { state: PAUSE_STATE_OFF, debug_state: PAUSE_DEBUG_STATE_CLOSED, prompt_choice: 0, prompt_pitch: 0.0, bg_prerender_state: PAUSE_BG_PRERENDER_OFF, delay_timer: 10, d_8082b260: 0, message_colors: [0; 7] }
    }
}

impl PauseContext {
    /// `IS_PAUSED`.
    pub fn is_paused(&self) -> bool {
        self.state != PAUSE_STATE_OFF || self.debug_state != PAUSE_DEBUG_STATE_CLOSED
    }

    /// `IS_PAUSE_STATE_GAMEOVER`.
    pub fn is_game_over(&self) -> bool {
        (PAUSE_STATE_GAME_OVER_START..=PAUSE_STATE_GAME_OVER_FINISH).contains(&self.state)
    }
}

/// `ABS(a) / b` on the C's s16s.
fn step(a: i16, b: i16) -> i16 {
    a.wrapping_abs() / b
}

impl PlayState {
    /// `KaleidoScopeCall_Update` for the game over screens (`Play_Update`, while paused): the
    /// prerender's setup, its wait, then `KaleidoScope_Update`.
    pub fn kaleido_scope_call_update(&mut self) {
        let p = &mut self.pause_ctx;
        if !p.is_paused() {
            return;
        }
        if p.state == PAUSE_STATE_GAME_OVER_START {
            p.bg_prerender_state = PAUSE_BG_PRERENDER_SETUP;
            p.state += 1;
        } else if p.state == PAUSE_STATE_GAME_OVER_WAIT_BG_PRERENDER {
            if p.bg_prerender_state >= PAUSE_BG_PRERENDER_READY {
                p.state += 1;
            }
        } else if p.state != PAUSE_STATE_OFF {
            self.kaleido_scope_update();
        }
    }

    /// `Play_Draw`'s pause background prerender: the frame after the setup is the one saved
    /// (`PAUSE_BG_PRERENDER_PROCESS`), the next one has it ready. Then the pause menu's draw,
    /// whose prompt reads the stick (`KaleidoScope_UpdatePrompt`, from `KaleidoScope_Draw`).
    pub fn kaleido_scope_draw_update(&mut self) {
        let p = &mut self.pause_ctx;
        if p.bg_prerender_state == PAUSE_BG_PRERENDER_PROCESS {
            p.bg_prerender_state = PAUSE_BG_PRERENDER_READY;
        } else if p.bg_prerender_state >= PAUSE_BG_PRERENDER_MAX {
            p.bg_prerender_state = PAUSE_BG_PRERENDER_OFF;
        }
        if p.bg_prerender_state == PAUSE_BG_PRERENDER_SETUP {
            p.bg_prerender_state = PAUSE_BG_PRERENDER_PROCESS;
            return;
        }
        // Play_DrawOverlayElements: KaleidoScopeCall_Draw while paused, from SHOW_MESSAGE.
        if p.is_paused() && p.bg_prerender_state >= PAUSE_BG_PRERENDER_READY && (PAUSE_STATE_GAME_OVER_SHOW_MESSAGE..=PAUSE_STATE_GAME_OVER_FINISH).contains(&p.state) {
            self.kaleido_scope_update_prompt();
        }
    }

    /// `KaleidoScope_UpdatePrompt`: on a prompt, the stick right to No, left to Yes. (The
    /// cursor's blinking alpha is drawing only.)
    fn kaleido_scope_update_prompt(&mut self) {
        use crate::audio::sfx::NA_SE_SY_CURSOR;
        let p = &mut self.pause_ctx;
        let stick_adj_x = self.input.rel.stick_x;
        if p.state == PAUSE_STATE_GAME_OVER_SAVE_PROMPT || p.state == PAUSE_STATE_GAME_OVER_CONTINUE_PROMPT {
            if p.prompt_choice == 0 && stick_adj_x >= 30 {
                self.audio.play_sfx_centered(NA_SE_SY_CURSOR);
                p.prompt_choice = 4;
            } else if p.prompt_choice != 0 && stick_adj_x <= -30 {
                self.audio.play_sfx_centered(NA_SE_SY_CURSOR);
                p.prompt_choice = 0;
            }
        }
    }

    /// `KaleidoScope_Update`'s game over states.
    fn kaleido_scope_update(&mut self) {
        use crate::audio::sfx::{NA_SE_SY_DECIDE, NA_SE_SY_PIECE_OF_HEART};
        use eng_input::pad::{BTN_A, BTN_START};
        let press_a = self.input.press.held(BTN_A);
        let press_start = self.input.press.held(BTN_START);
        match self.pause_ctx.state {
            PAUSE_STATE_GAME_OVER_INIT => {
                let p = &mut self.pause_ctx;
                // (The map cursor, the buttons' positions and the icon segments' DMA.)
                p.prompt_pitch = -434.0;
                crate::interface::change_alpha(&mut self.save, crate::interface::HUD_VISIBILITY_NOTHING);
                p.message_colors = [255, 130, 0, 0, 30, 0, 0];
                p.d_8082b260 = 30;
                p.prompt_choice = 0;
                p.state += 1;
            }
            PAUSE_STATE_GAME_OVER_SHOW_MESSAGE => {
                // The message's colours step to (30, 0, 0, 255) and (255, 130, 0) over 30 frames.
                let p = &mut self.pause_ctx;
                let c = &mut p.message_colors;
                let n = p.d_8082b260;
                let targets = [30i16, 0, 0, 255];
                let steps: Vec<i16> = (0..4).map(|k| step(c[k] - targets[k], n)).collect();
                for k in 0..4 {
                    if c[k] >= targets[k] {
                        c[k] -= steps[k];
                    } else {
                        c[k] += steps[k];
                    }
                }
                let env_targets = [255i16, 130, 0];
                let steps: Vec<i16> = (0..3).map(|k| step(c[4 + k] - env_targets[k], n)).collect();
                for k in 0..3 {
                    if c[4 + k] >= env_targets[k] {
                        c[4 + k] -= steps[k];
                    } else {
                        c[4 + k] += steps[k];
                    }
                }
                p.d_8082b260 -= 1;
                if p.d_8082b260 == 0 {
                    p.message_colors = [30, 0, 0, 255, 255, 130, 0];
                    p.d_8082b260 = 40;
                    p.state += 1;
                }
            }
            PAUSE_STATE_GAME_OVER_WINDOW_DELAY => {
                let p = &mut self.pause_ctx;
                p.d_8082b260 -= 1;
                if p.d_8082b260 == 0 {
                    p.state = PAUSE_STATE_GAME_OVER_SHOW_WINDOW;
                }
            }
            PAUSE_STATE_GAME_OVER_SHOW_WINDOW => {
                // (The pages' pitches, the panels, the buttons and the alphas turn with it.)
                let p = &mut self.pause_ctx;
                p.prompt_pitch -= 160.0 / R_PAUSE_UI_ANIMS_DURATION;
                if p.prompt_pitch < -628.0 {
                    p.prompt_pitch = -628.0;
                    p.state = PAUSE_STATE_GAME_OVER_SAVE_PROMPT;
                    self.save.deaths += 1;
                    if self.save.deaths > 999 {
                        self.save.deaths = 999;
                    }
                    log::info!("game over: \"Save?\" (the pause menu's stand-in: A, the stick for No)");
                }
            }
            PAUSE_STATE_GAME_OVER_SAVE_PROMPT => {
                if press_a {
                    if self.pause_ctx.prompt_choice != 0 {
                        self.pause_ctx.prompt_choice = 0;
                        self.audio.play_sfx_centered(NA_SE_SY_DECIDE);
                        self.pause_ctx.state = PAUSE_STATE_GAME_OVER_CONTINUE_PROMPT;
                        self.game_over_ctx.state += 1;
                        log::info!("game over: \"Continue?\" (A or Start)");
                    } else {
                        self.audio.play_sfx_centered(NA_SE_SY_PIECE_OF_HEART);
                        self.pause_ctx.prompt_choice = 0;
                        self.save_scene_flags();
                        self.save.saved_scene_id = self.scene_id;
                        log::info!("game over: saved (Sram_WriteSave, the write to SRAM, isn't ported)");
                        self.pause_ctx.state = PAUSE_STATE_GAME_OVER_SAVED;
                        self.pause_ctx.delay_timer = SAVED_DELAY;
                    }
                }
            }
            PAUSE_STATE_GAME_OVER_SAVED => {
                self.pause_ctx.delay_timer -= 1;
                if self.pause_ctx.delay_timer == 0 {
                    self.pause_ctx.state = PAUSE_STATE_GAME_OVER_CONTINUE_PROMPT;
                    self.game_over_ctx.state += 1;
                    log::info!("game over: \"Continue?\" (A or Start)");
                } else if self.pause_ctx.delay_timer <= 80 && (press_a || press_start) {
                    self.pause_ctx.state = PAUSE_STATE_GAME_OVER_CONTINUE_PROMPT;
                    self.game_over_ctx.state += 1;
                    self.audio.func_800f64e0(0);
                }
            }
            PAUSE_STATE_GAME_OVER_CONTINUE_PROMPT => {
                if press_a || press_start {
                    if self.pause_ctx.prompt_choice == 0 {
                        self.audio.play_sfx_centered(NA_SE_SY_PIECE_OF_HEART);
                        self.save_scene_flags();
                        self.continue_entrance();
                    } else {
                        self.audio.play_sfx_centered(NA_SE_SY_DECIDE);
                    }
                    self.pause_ctx.state = PAUSE_STATE_GAME_OVER_FINISH;
                }
            }
            PAUSE_STATE_GAME_OVER_FINISH => {
                if self.interface_ctx.unk_244 != 255 {
                    self.interface_ctx.unk_244 += 10;
                    if self.interface_ctx.unk_244 >= 255 {
                        self.interface_ctx.unk_244 = 255;
                        self.pause_ctx.state = PAUSE_STATE_OFF;
                        self.pause_ctx.bg_prerender_state = PAUSE_BG_PRERENDER_OFF;
                        // (R_UPDATE_RATE = 3; Object_ReloadAll and func_800418D0: the pause menu
                        // borrowed no object or collision space here.)
                        if self.pause_ctx.prompt_choice != 0 {
                            // play->state.running = false into TitleSetup_Init: the title screen
                            // isn't ported.
                            log::info!("game over: \"No\" goes back to the title screen, which isn't ported: continuing");
                        }
                        self.trigger_respawn();
                        self.save.respawn_flag = -2;
                        self.save.next_transition_type = crate::transition::TRANS_TYPE_FADE_BLACK;
                        self.save.health = 0x30;
                        // SEQCMD_RESET_AUDIO_HEAP(0, 10).
                        self.audio.queue_seq_cmd((0xF << 28) | 10);
                        self.save.health_accumulator = 0;
                        // (The magic meter's refill: magic isn't ported.)
                    }
                }
            }
            _ => {}
        }
    }

    /// The "Continue" choice's entrance: a boss room's is its dungeon's entrance (the
    /// dungeons' own entrances stay).
    fn continue_entrance(&mut self) {
        const BOSS_TO_DUNGEON: [(&str, &str); 9] = [
            ("ENTR_DEKU_TREE_BOSS_0", "ENTR_DEKU_TREE_0"),
            ("ENTR_DODONGOS_CAVERN_BOSS_0", "ENTR_DODONGOS_CAVERN_0"),
            ("ENTR_JABU_JABU_BOSS_0", "ENTR_JABU_JABU_0"),
            ("ENTR_FOREST_TEMPLE_BOSS_0", "ENTR_FOREST_TEMPLE_0"),
            ("ENTR_FIRE_TEMPLE_BOSS_0", "ENTR_FIRE_TEMPLE_0"),
            ("ENTR_WATER_TEMPLE_BOSS_0", "ENTR_WATER_TEMPLE_0"),
            ("ENTR_SPIRIT_TEMPLE_BOSS_0", "ENTR_SPIRIT_TEMPLE_0"),
            ("ENTR_SHADOW_TEMPLE_BOSS_0", "ENTR_SHADOW_TEMPLE_0"),
            ("ENTR_GANONDORF_BOSS_0", "ENTR_GANONS_TOWER_0"),
        ];
        let Some(a) = self.assets.clone() else { return };
        for (boss, dungeon) in BOSS_TO_DUNGEON {
            if a.scenes.entrance_index(boss) == Some(self.save.entrance_index)
                && let Some(e) = a.scenes.entrance_index(dungeon)
            {
                self.save.entrance_index = e;
                return;
            }
        }
    }
}
