//! The pause menu (`ovl_kaleido_scope`, with `z_kaleido_setup.c` and `z_kaleido_scope_call.c`;
//! docs/adr/0047-the-pause-menu.md).
//!
//! - **Opening** (`KaleidoSetup_Update`, in `Play_Update` before the actors): Start, when
//!   nothing stops it, sets the page to the right of the last one viewed with its eye, to scroll
//!   left onto it, and `PAUSE_STATE_WAIT_LETTERBOX`; the game runs at `R_UPDATE_RATE` 2 (30
//!   frames a second), the letterbox closes and `func_800F64E0(1)` plays `NA_SE_SY_WIN_OPEN`
//!   and mutes the sequences.
//! - **While paused** `Play_Update` stops the actors, collision, cutscenes, effects, cameras and
//!   the environment, and runs `KaleidoScopeCall_Update` instead of `Message_Update`: the
//!   letterbox's wait, the background's prerender (`R_PAUSE_BG_PRERENDER_STATE`, stepped by
//!   `Play_Draw`), then `KaleidoScope_Update` (`scope`). `Play_Draw` stops drawing the scene once
//!   it's saved; the menu draws over it (`KaleidoScopeCall_Draw`, `KaleidoScope_Draw`), then the
//!   HUD over the menu.
//! - **Closing** (Start): the pages turn away, then `PAUSE_STATE_RESUME_GAMEPLAY` restores the
//!   buttons and the HUD, runs `Player_SetEquipmentData` and the game's 20 frames a second.
//!
//! The pages: the item page whole (`item`), the dungeon map page whole (`map`, with its marks:
//! `lmap_mark`); the world map's contents and the equipment and quest status pages' contents log
//! what they'd do (their backgrounds are drawn). The save prompt (B) logs: saving is milestone
//! 5c's. The game over's states and screens are this module's too (docs/adr/0032,
//! docs/adr/0048-the-pause-map-and-the-game-over.md).
//!
//! **Not in the C: the equipment page's stand-in.** Its A button equips swords, shields, tunics
//! and boots (`KaleidoScope_DrawEquipment`, logged); in its place the menu's resume puts on what's
//! owned and unworn (`SaveContext::equip_owned_unworn`, docs/adr/0021).
//!
//! **The overlay's statics** (`KaleidoStatics`): `KaleidoManager_LoadOvl` loads
//! `ovl_kaleido_scope` from the ROM whenever the menu (or the game over) starts its update, so
//! its statics start from their initial values each time; the `PauseContext` and the REGs
//! (`PauseRegs`) live on.

pub mod gfx;
pub mod item;
pub mod lmap_mark;
pub mod map;
pub mod scope;
#[cfg(test)]
mod tests;

use glam::Vec3;

use crate::play::PlayState;
use gfx::{KaleidoGfx, Vtx};

// `PauseState` (`pause.h`).
pub const PAUSE_STATE_OFF: u16 = 0;
pub const PAUSE_STATE_WAIT_LETTERBOX: u16 = 1;
pub const PAUSE_STATE_WAIT_BG_PRERENDER: u16 = 2;
pub const PAUSE_STATE_INIT: u16 = 3;
pub const PAUSE_STATE_OPENING_1: u16 = 4;
pub const PAUSE_STATE_OPENING_2: u16 = 5;
pub const PAUSE_STATE_MAIN: u16 = 6;
pub const PAUSE_STATE_SAVE_PROMPT: u16 = 7;
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
pub const PAUSE_STATE_CLOSING: u16 = 18;
pub const PAUSE_STATE_RESUME_GAMEPLAY: u16 = 19;

// `PauseDebugState`.
pub const PAUSE_DEBUG_STATE_CLOSED: u16 = 0;
pub const PAUSE_DEBUG_STATE_INVENTORY_EDITOR_OPENING: u16 = 1;
pub const PAUSE_DEBUG_STATE_INVENTORY_EDITOR_OPEN: u16 = 2;
pub const PAUSE_DEBUG_STATE_FLAG_SET_OPEN: u16 = 3;

// `PauseMainState`.
pub const PAUSE_MAIN_STATE_IDLE: u16 = 0;
pub const PAUSE_MAIN_STATE_SWITCHING_PAGE: u16 = 1;
pub const PAUSE_MAIN_STATE_SONG_PLAYBACK: u16 = 2;
/// The item page's equip (the icon flying to its C button).
pub const PAUSE_MAIN_STATE_3: u16 = 3;
pub const PAUSE_MAIN_STATE_SONG_PROMPT_INIT: u16 = 4;
pub const PAUSE_MAIN_STATE_SONG_PROMPT: u16 = 5;
pub const PAUSE_MAIN_STATE_SONG_PROMPT_DONE: u16 = 6;
pub const PAUSE_MAIN_STATE_EQUIP_CHANGED: u16 = 7;
pub const PAUSE_MAIN_STATE_IDLE_CURSOR_ON_SONG: u16 = 8;
pub const PAUSE_MAIN_STATE_SONG_PLAYBACK_START: u16 = 9;

// `PauseSavePromptState`.
pub const PAUSE_SAVE_PROMPT_STATE_APPEARING: u16 = 0;
pub const PAUSE_SAVE_PROMPT_STATE_WAIT_CHOICE: u16 = 1;
pub const PAUSE_SAVE_PROMPT_STATE_CLOSING: u16 = 2;
pub const PAUSE_SAVE_PROMPT_STATE_SAVED: u16 = 4;
pub const PAUSE_SAVE_PROMPT_STATE_CLOSING_AFTER_SAVED: u16 = 5;

// `PauseMenuPage`.
pub const PAUSE_ITEM: u16 = 0;
pub const PAUSE_MAP: u16 = 1;
pub const PAUSE_QUEST: u16 = 2;
pub const PAUSE_EQUIP: u16 = 3;
pub const PAUSE_WORLD_MAP: u16 = 4;

/// `PAUSE_ITEM_NONE`: no item under the cursor.
pub const PAUSE_ITEM_NONE: u16 = 999;
/// `PAUSE_CURSOR_PAGE_LEFT`, `PAUSE_CURSOR_PAGE_RIGHT`: the cursor on a page arrow
/// (`cursorSpecialPos`).
pub const PAUSE_CURSOR_PAGE_LEFT: i16 = 10;
pub const PAUSE_CURSOR_PAGE_RIGHT: i16 = 11;

/// `PAUSE_EYE_DIST`.
pub const PAUSE_EYE_DIST: f32 = 64.0;
/// `PAUSE_PAGES_Y_ORIGIN_1_LOWER`, `PAUSE_PAGES_Y_ORIGIN_2_LOWER` (`(s16)(-80 * 0.78 * 100)`):
/// the pages turn about their lower edge while opening and closing.
pub const PAUSE_PAGES_Y_ORIGIN_1_LOWER: i16 = 80;
pub const PAUSE_PAGES_Y_ORIGIN_2_LOWER: i16 = -6240;

// `PauseBgPreRenderState` (`play_state.h`).
pub const PAUSE_BG_PRERENDER_OFF: u8 = 0;
pub const PAUSE_BG_PRERENDER_SETUP: u8 = 1;
pub const PAUSE_BG_PRERENDER_PROCESS: u8 = 2;
pub const PAUSE_BG_PRERENDER_READY: u8 = 3;
pub const PAUSE_BG_PRERENDER_MAX: u8 = 4;

/// `WORLD_MAP_POINT_MAX`.
pub const WORLD_MAP_POINT_MAX: usize = 12;

/// The REGs the menu reads and writes (`regs.h`), with `Regs_InitDataImpl`'s values
/// (`z_construct.c`, the PAL ones). They live in the play state, not the overlay.
#[derive(Debug, Clone, PartialEq)]
pub struct PauseRegs {
    /// `R_PAUSE_PAGES_Y_ORIGIN_2` (`WREG(2)`, -6080).
    pub pages_y_origin_2: i16,
    /// `R_PAUSE_DEPTH_OFFSET` (`WREG(3)`, 9355).
    pub depth_offset: i16,
    /// `WREG(4)` (8): the alpha's fade-in adds it to the animations' duration.
    pub wreg4: i16,
    /// `R_PAUSE_UI_ANIMS_DURATION` (`WREG(6)`, 8).
    pub ui_anims_duration: i16,
    /// `R_PAUSE_BUTTON_LEFT_X`, `R_PAUSE_BUTTON_RIGHT_X`, `R_PAUSE_BUTTON_LEFT_RIGHT_Y`
    /// (`WREG(16..18)`: -175, 155, 10).
    pub button_left_x: i16,
    pub button_right_x: i16,
    pub button_left_right_y: i16,
    /// `R_PAUSE_BUTTON_LEFT_MOVE_OFFSET_X`, `_RIGHT_` (`WREG(25)`, `WREG(26)`: 40, -40).
    pub button_left_move_offset_x: i16,
    pub button_right_move_offset_x: i16,
    /// `R_PAUSE_INFO_PANEL_ICON_C_ITEM_X(ENG)` (`WREG(49)`, -48), `_TEXT_X` (`WREG(52)`, 22),
    /// `_ICON_PLAY_SONG_X` (`WREG(55)`, -53), `_TEXT_C_ITEM_X` (`WREG(58)`, 47),
    /// `_ICON_SAVE_PROMPT_X` (`WREG(61)`, -42), `_ICON_EQUIP_X` (`WREG(64)`, -37).
    pub info_panel_icon_c_item_x: i16,
    pub info_panel_text_x: i16,
    pub info_panel_icon_play_song_x: i16,
    pub info_panel_text_c_item_x: i16,
    pub info_panel_icon_save_prompt_x: i16,
    pub info_panel_icon_equip_x: i16,
    /// `WREG(87)` (80), `WREG(88)` (70: the name panel's timer's period), `WREG(89)` (40: the
    /// name shows while the timer is under it), `WREG(90)` (320: the equipped icon's size, in
    /// tenths), `WREG(91)` (40).
    pub wreg87: i16,
    pub wreg88: i16,
    pub wreg89: i16,
    pub wreg90: i16,
    pub wreg91: i16,
    /// `XREG(5)` (0): stepped with the opening and closing, read by nothing ported.
    pub xreg5: i16,
    /// `VREG(87)`, `VREG(88)` (64, 66): the game over's message's top left; `VREG(89)` (0): its
    /// mask's scroll.
    pub vreg87: i16,
    pub vreg88: i16,
    pub vreg89: i16,
    /// `GREG(92)`, `GREG(93)` (0): `DEBUG_FEATURES`' offset of the pause map's marks.
    pub greg92: i16,
    pub greg93: i16,
    /// `R_PAUSE_STICK_REPEAT_DELAY` (`XREG(6)`, 2), `R_PAUSE_STICK_REPEAT_DELAY_FIRST`
    /// (`XREG(8)`, 10).
    pub stick_repeat_delay: i16,
    pub stick_repeat_delay_first: i16,
    /// `R_PAUSE_PAGE_SWITCH_FRAME_ADVANCE_ON` (`ZREG(13)`, false).
    pub page_switch_frame_advance_on: bool,
    /// `ZREG(28..31)` (20, 4, 20, 10): the cursor colour's phases; `ZREG(28)` is also
    /// `R_PAUSE_BUTTON_L_R_SELECTED_PRIM_TIMER`.
    pub zreg28: [i16; 4],
    /// `ZREG(46)`, `ZREG(47)` (1, 1): the opening's steps.
    pub zreg46: i16,
    pub zreg47: i16,
    /// `ZREG(48)`: `R_START_LABEL_DD(LANGUAGE_ENG)` on PAL (100), read by `KaleidoSetup_Update`.
    pub zreg48: i16,
    /// `YREG(8)` (10): the save prompt's pitch as it closes.
    pub yreg8: i16,
    /// `R_KALEIDO_PROMPT_CURSOR_ALPHA_TIMER_BASE`, `_ALPHA`, `_ALPHA_STATE`, `_ALPHA_TIMER`
    /// (`VREG(60..63)`: 20, 100, 0, 10).
    pub prompt_cursor_alpha_timer_base: i16,
    pub prompt_cursor_alpha: i16,
    pub prompt_cursor_alpha_state: i16,
    pub prompt_cursor_alpha_timer: i16,
}

impl Default for PauseRegs {
    fn default() -> PauseRegs {
        PauseRegs {
            pages_y_origin_2: -6080,
            depth_offset: 9355,
            wreg4: 8,
            ui_anims_duration: 8,
            button_left_x: -175,
            button_right_x: 155,
            button_left_right_y: 10,
            button_left_move_offset_x: 40,
            button_right_move_offset_x: -40,
            info_panel_icon_c_item_x: -48,
            info_panel_text_x: 22,
            info_panel_icon_play_song_x: -53,
            info_panel_text_c_item_x: 47,
            info_panel_icon_save_prompt_x: -42,
            info_panel_icon_equip_x: -37,
            wreg87: 80,
            wreg88: 70,
            wreg89: 40,
            wreg90: 320,
            wreg91: 40,
            xreg5: 0,
            vreg87: 64,
            vreg88: 66,
            vreg89: 0,
            greg92: 0,
            greg93: 0,
            stick_repeat_delay: 2,
            stick_repeat_delay_first: 10,
            page_switch_frame_advance_on: false,
            zreg28: [20, 4, 20, 10],
            zreg46: 1,
            zreg47: 1,
            zreg48: 100,
            yreg8: 10,
            prompt_cursor_alpha_timer_base: 20,
            prompt_cursor_alpha: 100,
            prompt_cursor_alpha_state: 0,
            prompt_cursor_alpha_timer: 10,
        }
    }
}

/// `ovl_kaleido_scope`'s statics, at their initial values each time the overlay loads.
#[derive(Debug, Clone, PartialEq)]
pub struct KaleidoStatics {
    /// `D_8082AB8C`, `D_8082AB90`, `D_8082AB94`, `D_8082AB98`, `D_8082AB9C`, `D_8082ABA0`,
    /// `D_8082ABA4`: the cursor's env colour (the first three), and the game over message's
    /// prim and env colours.
    pub d_8082ab8c: [i16; 7],
    /// `sInDungeonScene`.
    pub in_dungeon_scene: bool,
    /// `KaleidoScope_DrawPages`' `D_8082AD3C` (20) and `D_8082AD40`: the cursor colour's timer and
    /// phase; `sStickXRepeatTimer`, `sStickYRepeatTimer`, `sStickXRepeatState`,
    /// `sStickYRepeatState`.
    pub d_8082ad3c: i16,
    pub d_8082ad40: i16,
    pub stick_x_repeat_timer: i16,
    pub stick_y_repeat_timer: i16,
    pub stick_x_repeat_state: i16,
    pub stick_y_repeat_state: i16,
    /// `KaleidoScope_DrawUIOverlay`'s `sLRSelectedPrimTimer` (20), `sLRSelectedPrimState`, and
    /// `sLRSelectedPrimR`..`A` (bss: 0).
    pub lr_selected_prim_timer: i16,
    pub lr_selected_prim_state: i16,
    pub lr_selected_prim: [i16; 4],
    /// `z_kaleido_item.c`'s `sEquipState`, `sEquipAnimTimer`, `sEquipMoveTimer` (10), and
    /// `KaleidoScope_UpdateItemEquip`'s `D_8082A488`.
    pub equip_state: i16,
    pub equip_anim_timer: i16,
    pub equip_move_timer: i16,
    pub d_8082a488: i16,
    /// `KaleidoScope_Update`'s `sMainStateAfterSongPlayerPlayingDone`, `sDelayTimer` (10),
    /// `D_8082B260`.
    pub main_state_after_song_player_playing_done: u16,
    pub delay_timer: i16,
    pub d_8082b260: i16,
    /// `KaleidoScope_SetPageVertices`' `sTradeQuestMarkerBobY`, `_BobTimer` (1), `_BobState`.
    pub trade_quest_marker_bob_y: i16,
    pub trade_quest_marker_bob_timer: i16,
    pub trade_quest_marker_bob_state: i16,
    /// `sSavedButtonStatus` (bss).
    pub saved_button_status: [u8; 5],
    /// `KaleidoScope_DrawDungeonMap`'s `mapBgPulseR`, `_G`, `_B` (0, 200 / 8, 140 / 8),
    /// `mapBgPulseTimer` (20) and `mapBgPulseStage`: the current room's colour.
    pub map_bg_pulse_r: i16,
    pub map_bg_pulse_g: i16,
    pub map_bg_pulse_b: i16,
    pub map_bg_pulse_timer: u16,
    pub map_bg_pulse_stage: u16,
    /// Not in the C: which pages' unported contents were logged since the overlay loaded.
    pub logged: u32,
}

impl Default for KaleidoStatics {
    fn default() -> KaleidoStatics {
        KaleidoStatics {
            d_8082ab8c: [0, 0, 0, 255, 255, 0, 0],
            in_dungeon_scene: false,
            d_8082ad3c: 20,
            d_8082ad40: 0,
            stick_x_repeat_timer: 0,
            stick_y_repeat_timer: 0,
            stick_x_repeat_state: 0,
            stick_y_repeat_state: 0,
            lr_selected_prim_timer: 20,
            lr_selected_prim_state: 0,
            lr_selected_prim: [0; 4],
            equip_state: 0,
            equip_anim_timer: 0,
            equip_move_timer: 10,
            d_8082a488: 0,
            main_state_after_song_player_playing_done: PAUSE_MAIN_STATE_IDLE,
            delay_timer: 10,
            d_8082b260: 0,
            trade_quest_marker_bob_y: 0,
            trade_quest_marker_bob_timer: 1,
            trade_quest_marker_bob_state: 0,
            saved_button_status: [0; 5],
            map_bg_pulse_r: 0 / 8,
            map_bg_pulse_g: 200 / 8,
            map_bg_pulse_b: 140 / 8,
            map_bg_pulse_timer: 20,
            map_bg_pulse_stage: 0,
            logged: 0,
        }
    }
}

/// `PauseContext` (`pause.h`), with the overlay's statics and the frame's vertex arrays.
#[derive(Debug, Clone)]
pub struct PauseContext {
    pub state: u16,
    pub debug_state: u16,
    pub eye: Vec3,
    pub main_state: u16,
    /// `nextPageMode`: the page switched from and the direction, `page * 2 + (left ? 1 : 0)`.
    pub next_page_mode: u16,
    pub page_index: u16,
    pub page_switch_timer: u16,
    pub save_prompt_state: u16,
    pub prompt_depth_offset: f32,
    pub item_page_pitch: f32,
    pub equip_page_pitch: f32,
    pub map_page_pitch: f32,
    pub quest_page_pitch: f32,
    /// `promptPitch`: the save and game over prompts' turn.
    pub prompt_pitch: f32,
    pub alpha: u16,
    pub pages_y_origin1: i16,
    pub stick_adj_x: i16,
    pub stick_adj_y: i16,
    pub cursor_point: [i16; 5],
    pub cursor_x: [i16; 5],
    pub cursor_y: [i16; 5],
    pub dungeon_map_slot: i16,
    pub cursor_special_pos: i16,
    pub page_switch_input_timer: i16,
    /// `namedItem`: the name `nameSegment` holds.
    pub named_item: u16,
    pub cursor_item: [u16; 4],
    pub cursor_slot: [u16; 4],
    pub equip_target_item: u16,
    pub equip_target_slot: u16,
    pub equip_target_c_btn: u16,
    pub equip_anim_x: i16,
    pub equip_anim_y: i16,
    pub equip_anim_alpha: i16,
    pub info_panel_offset_y: i16,
    pub name_display_timer: u16,
    /// 0 white, 1 grey.
    pub name_color_set: u16,
    /// 0 white, 4 yellow, 8 green.
    pub cursor_color_set: i16,
    /// The save and continue prompts' choice: 0 yes, 4 no.
    pub prompt_choice: i16,
    pub ocarina_song_idx: i16,
    pub world_map_points: [u8; 20],
    pub trade_quest_marker: u8,
    /// `R_PAUSE_BG_PRERENDER_STATE` (`SREG(94)`).
    pub bg_prerender_state: u8,
    /// Not in the C: the fills (`PlayState::draw_fills`) of the frame the background was saved
    /// from, which the saved frame shows while the scene isn't drawn.
    pub bg_fills: Option<(Option<[u8; 4]>, Option<[u8; 4]>)>,
    pub regs: PauseRegs,
    pub statics: KaleidoStatics,
    /// `gBossMarkState` (`z_kaleido_manager.c`), `gBossMarkScale` (`z_kaleido_scope_call.c`): the
    /// pause map's boss mark's pulse (`PauseMapMark_Init` resets both every draw).
    pub boss_mark_state: u8,
    pub boss_mark_scale: f32,
    /// `gKaleidoMgrCurOvl == kaleidoScopeOvl`: the overlay is loaded.
    pub ovl_loaded: bool,
    /// The vertex arrays `KaleidoScope_SetVertices` builds each frame (`itemPageVtx` ..
    /// `promptPageVtx`), kept as the next frame's update reads them.
    pub item_page_vtx: Vec<Vtx>,
    pub equip_page_vtx: Vec<Vtx>,
    pub map_page_vtx: Vec<Vtx>,
    pub quest_page_vtx: Vec<Vtx>,
    pub ui_overlay_vtx: Vec<Vtx>,
    pub item_vtx: Vec<Vtx>,
    pub equip_vtx: Vec<Vtx>,
    pub quest_vtx: Vec<Vtx>,
    pub cursor_vtx: Vec<Vtx>,
    pub prompt_page_vtx: Vec<Vtx>,
    /// The menu's draw this frame (`KaleidoScope_Draw`), drawn by `PlayState::draw`.
    pub gfx: KaleidoGfx,
}

impl Default for PauseContext {
    /// `KaleidoSetup_Init` (`KaleidoScopeCall_Init` in `Play_Init`, `VREG(30)` 0).
    fn default() -> PauseContext {
        PauseContext {
            state: PAUSE_STATE_OFF,
            debug_state: PAUSE_DEBUG_STATE_CLOSED,
            eye: Vec3::new(0.0, 0.0, 64.0),
            main_state: PAUSE_MAIN_STATE_IDLE,
            next_page_mode: 0,
            page_index: PAUSE_ITEM,
            page_switch_timer: 0,
            save_prompt_state: 0,
            prompt_depth_offset: 936.0,
            item_page_pitch: 160.0,
            equip_page_pitch: 160.0,
            map_page_pitch: 160.0,
            quest_page_pitch: 160.0,
            prompt_pitch: -314.0,
            alpha: 0,
            pages_y_origin1: 0,
            stick_adj_x: 0,
            stick_adj_y: 0,
            // cursorPoint: PAUSE_ITEM 0, PAUSE_MAP VREG(30) + 3, PAUSE_QUEST QUEST_MEDALLION_FOREST,
            // PAUSE_EQUIP 1, PAUSE_WORLD_MAP 10.
            cursor_point: [0, 3, 0, 1, 10],
            // cursorX[PAUSE_EQUIP] EQUIP_VALUE_SWORD_KOKIRI, cursorY[PAUSE_EQUIP] EQUIP_TYPE_SWORD.
            cursor_x: [0, 0, 0, 1, 0],
            cursor_y: [0, 0, 0, 0, 0],
            dungeon_map_slot: 0,
            cursor_special_pos: 0,
            page_switch_input_timer: 0,
            named_item: 0,
            // cursorItem: PAUSE_ITEM_NONE, VREG(30) + 3, PAUSE_ITEM_NONE, ITEM_SWORD_KOKIRI.
            cursor_item: [PAUSE_ITEM_NONE, 3, PAUSE_ITEM_NONE, crate::item::ITEM_SWORD_KOKIRI as u16],
            // cursorSlot: 0, VREG(30) + 3, 0, cursorPoint[PAUSE_EQUIP].
            cursor_slot: [0, 3, 0, 1],
            equip_target_item: 0,
            equip_target_slot: 0,
            equip_target_c_btn: 0,
            equip_anim_x: 0,
            equip_anim_y: 0,
            equip_anim_alpha: 0,
            info_panel_offset_y: -40,
            name_display_timer: 0,
            name_color_set: 0,
            cursor_color_set: 4,
            prompt_choice: 0,
            ocarina_song_idx: -1,
            world_map_points: [0; 20],
            trade_quest_marker: 0,
            bg_prerender_state: PAUSE_BG_PRERENDER_OFF,
            bg_fills: None,
            regs: PauseRegs::default(),
            statics: KaleidoStatics::default(),
            boss_mark_state: 0,
            boss_mark_scale: 0.0,
            ovl_loaded: false,
            item_page_vtx: Vec::new(),
            equip_page_vtx: Vec::new(),
            map_page_vtx: Vec::new(),
            quest_page_vtx: Vec::new(),
            ui_overlay_vtx: vec![Vtx::default(); 7 * 4],
            item_vtx: Vec::new(),
            equip_vtx: Vec::new(),
            quest_vtx: Vec::new(),
            cursor_vtx: vec![Vtx::default(); 5 * 4],
            prompt_page_vtx: Vec::new(),
            gfx: KaleidoGfx::default(),
        }
    }
}

impl PartialEq for PauseContext {
    /// The menu's state (the frame's draw aside).
    fn eq(&self, o: &PauseContext) -> bool {
        (self.state, self.debug_state, self.main_state, self.page_index, self.cursor_point, self.cursor_special_pos, self.cursor_item, self.cursor_slot)
            == (o.state, o.debug_state, o.main_state, o.page_index, o.cursor_point, o.cursor_special_pos, o.cursor_item, o.cursor_slot)
            && (self.eye, self.alpha, self.prompt_pitch, self.item_page_pitch, self.bg_prerender_state) == (o.eye, o.alpha, o.prompt_pitch, o.item_page_pitch, o.bg_prerender_state)
            && self.statics == o.statics
            && self.regs == o.regs
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

    /// Logs a page's unported part once per opening (`bit` its flag).
    fn log_once(&mut self, bit: u32, what: &str) {
        if self.statics.logged & (1 << bit) == 0 {
            self.statics.logged |= 1 << bit;
            log::info!("{what}");
        }
    }
}

/// `AGE_REQ_ADULT`, `AGE_REQ_CHILD` (`LINK_AGE_ADULT` 0, `LINK_AGE_CHILD` 1), `AGE_REQ_NONE`
/// (`z_kaleido_scope.h`).
pub const AGE_REQ_ADULT: u8 = 0;
pub const AGE_REQ_CHILD: u8 = 1;
pub const AGE_REQ_NONE: u8 = 9;

/// `gSlotAgeReqs` (`z_kaleido_scope.c`): by inventory slot.
pub const SLOT_AGE_REQS: [u8; 24] = [
    AGE_REQ_CHILD, // SLOT_DEKU_STICK
    AGE_REQ_NONE,  // SLOT_DEKU_NUT
    AGE_REQ_NONE,  // SLOT_BOMB
    AGE_REQ_ADULT, // SLOT_BOW
    AGE_REQ_ADULT, // SLOT_ARROW_FIRE
    AGE_REQ_NONE,  // SLOT_DINS_FIRE
    AGE_REQ_CHILD, // SLOT_SLINGSHOT
    AGE_REQ_NONE,  // SLOT_OCARINA
    AGE_REQ_NONE,  // SLOT_BOMBCHU
    AGE_REQ_ADULT, // SLOT_HOOKSHOT
    AGE_REQ_ADULT, // SLOT_ARROW_ICE
    AGE_REQ_NONE,  // SLOT_FARORES_WIND
    AGE_REQ_CHILD, // SLOT_BOOMERANG
    AGE_REQ_NONE,  // SLOT_LENS_OF_TRUTH
    AGE_REQ_CHILD, // SLOT_MAGIC_BEAN
    AGE_REQ_ADULT, // SLOT_HAMMER
    AGE_REQ_ADULT, // SLOT_ARROW_LIGHT
    AGE_REQ_NONE,  // SLOT_NAYRUS_LOVE
    AGE_REQ_NONE,  // SLOT_BOTTLE_1
    AGE_REQ_NONE,  // SLOT_BOTTLE_2
    AGE_REQ_NONE,  // SLOT_BOTTLE_3
    AGE_REQ_NONE,  // SLOT_BOTTLE_4
    AGE_REQ_ADULT, // SLOT_TRADE_ADULT
    AGE_REQ_CHILD, // SLOT_TRADE_CHILD
];

/// `gItemAgeReqs` (`z_kaleido_scope.c`): by item, up to `ITEM_GIANTS_KNIFE`.
pub const ITEM_AGE_REQS: [u8; 0x56] = {
    const A: u8 = AGE_REQ_ADULT;
    const C: u8 = AGE_REQ_CHILD;
    const N: u8 = AGE_REQ_NONE;
    [
        C, N, N, A, A, N, C, N, N, N, A, A, A, N, C, N, // ITEM_DEKU_STICK .. ITEM_LENS_OF_TRUTH
        C, A, A, N, N, N, N, N, N, N, N, N, N, N, N, N, // ITEM_MAGIC_BEAN .. ITEM_BOTTLE_MILK_HALF
        N, C, C, C, C, C, C, C, C, C, C, C, C, A, A, A, // ITEM_BOTTLE_POE .. ITEM_COJIRO
        A, A, A, A, A, A, A, A, A, A, A, C, A, A, C, N, // ITEM_ODD_MUSHROOM .. ITEM_SHIELD_HYLIAN
        A, N, A, A, N, A, A, C, C, C, A, A, A, N, N, N, // ITEM_SHIELD_MIRROR .. ITEM_BOMB_BAG_40
        C, A, A, N, N, A, // ITEM_STRENGTH_GORONS_BRACELET .. ITEM_GIANTS_KNIFE
    ]
};

/// `CHECK_AGE_REQ_SLOT(slot)`: Link's age can use the slot (`linkAge`: 0 adult, 1 child).
pub fn check_age_req_slot(slot: usize, adult: bool) -> bool {
    let age = if adult { AGE_REQ_ADULT } else { AGE_REQ_CHILD };
    SLOT_AGE_REQS.get(slot).is_none_or(|&r| r == AGE_REQ_NONE || r == age)
}

/// `CHECK_AGE_REQ_ITEM(item)`.
pub fn check_age_req_item(item: usize, adult: bool) -> bool {
    let age = if adult { AGE_REQ_ADULT } else { AGE_REQ_CHILD };
    ITEM_AGE_REQS.get(item).is_none_or(|&r| r == AGE_REQ_NONE || r == age)
}

/// `KaleidoScope_GrayOutTextureRGBA32`: each texel with a colour (`& 0xFFFFFF00`) becomes the
/// grey `(r + 2g + b) / 7`, its alpha kept. Run by the importer on the icons the menu greys (the
/// runtime has no ROM).
pub fn gray_out_texture_rgba32(texture: &mut [u8]) {
    for t in texture.chunks_exact_mut(4) {
        let texel = u32::from_be_bytes([t[0], t[1], t[2], t[3]]);
        if texel & 0xFFFF_FF00 != 0 {
            let rgb = texel >> 8;
            // ((rgb & 0xFF00) >> 7) is green times 2 (its low bit dropped).
            let gray = ((((rgb & 0xFF_0000) >> 16) + ((rgb & 0xFF00) >> 7) + (rgb & 0xFF)) / 7) as u8;
            t[0] = gray;
            t[1] = gray;
            t[2] = gray;
        }
    }
}

/// `sKaleidoSetupRightPageIndex`, `sKaleidoSetupRightPageEyeX`, `_EyeZ` (`z_kaleido_setup.c`):
/// to open on page P, the menu starts on the page to its right and scrolls left.
const SETUP_RIGHT_PAGE_INDEX: [u16; 4] = [PAUSE_MAP, PAUSE_QUEST, PAUSE_EQUIP, PAUSE_ITEM];
const SETUP_RIGHT_PAGE_EYE_X: [f32; 4] = [PAUSE_EYE_DIST * -1.0, 0.0, PAUSE_EYE_DIST * 1.0, 0.0];
const SETUP_RIGHT_PAGE_EYE_Z: [f32; 4] = [0.0, PAUSE_EYE_DIST * -1.0, 0.0, PAUSE_EYE_DIST * 1.0];
/// `sKaleidoSetupUnusedPageIndex`, `_EyeX`, `_EyeZ`: the opposite page's (`ZREG(48)` 0 only).
const SETUP_UNUSED_PAGE_INDEX: [u16; 4] = [PAUSE_QUEST, PAUSE_EQUIP, PAUSE_ITEM, PAUSE_MAP];
const SETUP_UNUSED_EYE_X: [f32; 4] = [0.0, PAUSE_EYE_DIST * 1.0, 0.0, PAUSE_EYE_DIST * -1.0];
const SETUP_UNUSED_EYE_Z: [f32; 4] = [PAUSE_EYE_DIST * -1.0, 0.0, PAUSE_EYE_DIST * 1.0, 0.0];

/// `CS_INDEX_0` (`z64cutscene.h`).
const CS_INDEX_0: u16 = 0xFFF0;

impl PlayState {
    /// `KaleidoSetup_Update` (`z_kaleido_setup.c`): Start opens the menu when nothing stops it.
    /// L with C-Up is the flag set debug screen, which needs `BREG(0)`: nothing.
    pub(crate) fn kaleido_setup_update(&mut self) {
        use crate::transition::{TRANS_MODE_OFF, TRANS_TRIGGER_OFF};
        use eng_input::pad::{BTN_CUP, BTN_L, BTN_START};
        // (shootingGalleryStatus <= 1, magicState not filling: no shooting gallery or magic
        // is ported. The Bombchu Bowling Alley's switch 0x38: its scene isn't.)
        if !(!self.pause_ctx.is_paused()
            && self.game_over_ctx.state == crate::game_over::GAMEOVER_INACTIVE
            && self.transition.trigger == TRANS_TRIGGER_OFF
            && self.transition.mode == TRANS_MODE_OFF
            && self.save.cutscene_index < CS_INDEX_0
            && self.save.next_cutscene_index < CS_INDEX_0
            && !self.play_in_cs_mode())
        {
            return;
        }
        let p = &mut self.pause_ctx;
        if self.input.cur.held(BTN_L) && self.input.press.held(BTN_CUP) {
            // DEBUG_FEATURES && BREG(0): PAUSE_DEBUG_STATE_FLAG_SET_OPEN. BREG(0) is 0.
        } else if self.input.press.held(BTN_START) {
            self.save.prev_hud_visibility_mode = self.save.hud_visibility_mode;
            p.regs.button_left_x = -175;
            p.regs.button_right_x = 155;
            p.page_switch_timer = 0;
            // (Irrelevant: mainState is set again before it's read.)
            p.main_state = PAUSE_MAIN_STATE_SWITCHING_PAGE;
            // @bug (game): REG collision: ZREG(48) is also R_START_LABEL_DD(0) on PAL (100), so
            // the opposite page's arrays (never right) aren't used.
            let page = p.page_index as usize;
            if p.regs.zreg48 == 0 {
                p.eye.x = SETUP_UNUSED_EYE_X[page];
                p.eye.z = SETUP_UNUSED_EYE_Z[page];
                p.page_index = SETUP_UNUSED_PAGE_INDEX[page];
            } else {
                p.eye.x = SETUP_RIGHT_PAGE_EYE_X[page];
                p.eye.z = SETUP_RIGHT_PAGE_EYE_Z[page];
                p.page_index = SETUP_RIGHT_PAGE_INDEX[page];
            }
            p.next_page_mode = p.page_index * 2 + 1;
            p.state = PAUSE_STATE_WAIT_LETTERBOX;
        }
        if p.state == PAUSE_STATE_WAIT_LETTERBOX {
            p.regs.pages_y_origin_2 = PAUSE_PAGES_Y_ORIGIN_2_LOWER;
            self.r_update_rate = 2;
            if self.letterbox.size_target != 0 {
                self.letterbox.set_size_target(0);
            }
            self.audio.func_800f64e0(1);
        }
    }

    /// `KaleidoScopeCall_Update` (`Play_Update`, while paused): the letterbox's wait and the
    /// background's prerender, then `KaleidoScope_Update` with the overlay loaded.
    pub fn kaleido_scope_call_update(&mut self) {
        let p = &mut self.pause_ctx;
        if !p.is_paused() {
            return;
        }
        if p.state == PAUSE_STATE_WAIT_LETTERBOX {
            if self.letterbox.size == 0 {
                // (DEBUG_FEATURES: the microcode disassembler's registers.)
                p.bg_prerender_state = PAUSE_BG_PRERENDER_SETUP;
                p.main_state = PAUSE_MAIN_STATE_IDLE;
                p.save_prompt_state = PAUSE_SAVE_PROMPT_STATE_APPEARING;
                p.state += 1;
            }
        } else if p.state == PAUSE_STATE_GAME_OVER_START {
            p.bg_prerender_state = PAUSE_BG_PRERENDER_SETUP;
            p.main_state = PAUSE_MAIN_STATE_IDLE;
            // Copied from the pause menu, not needed here.
            p.save_prompt_state = PAUSE_SAVE_PROMPT_STATE_APPEARING;
            p.state += 1;
        } else if p.state == PAUSE_STATE_WAIT_BG_PRERENDER || p.state == PAUSE_STATE_GAME_OVER_WAIT_BG_PRERENDER {
            if p.bg_prerender_state >= PAUSE_BG_PRERENDER_READY {
                p.state += 1;
            }
        } else if p.state != PAUSE_STATE_OFF {
            // KaleidoManager_LoadOvl: the overlay from the ROM, its statics fresh (Player's
            // overlay, which shares its RAM, isn't modelled: the port keeps Player's statics).
            if !p.ovl_loaded {
                p.statics = KaleidoStatics::default();
                p.ovl_loaded = true;
            }
            self.kaleido_scope_update();
            if !self.pause_ctx.is_paused() {
                // KaleidoManager_ClearOvl, KaleidoScopeCall_LoadPlayer.
                self.pause_ctx.ovl_loaded = false;
            }
        }
    }

    /// `Play_Draw`'s pause background and `Play_DrawOverlayElements`' `KaleidoScopeCall_Draw`:
    /// the prerender's steps (the frame after the setup's is the one saved, `PROCESS`, then
    /// `READY`), then the menu's draw from `PAUSE_STATE_OPENING_1` (and the game over's from its
    /// message) while the background is ready. Returns whether `Play_Draw` skips the overlay
    /// elements this frame (the setup's frame: it's saved, not shown).
    pub(crate) fn kaleido_scope_draw_update(&mut self) -> bool {
        let fills = (self.pause_ctx.bg_prerender_state == PAUSE_BG_PRERENDER_SETUP).then(|| self.draw_fills());
        let p = &mut self.pause_ctx;
        p.gfx = KaleidoGfx::default();
        if p.bg_prerender_state == PAUSE_BG_PRERENDER_OFF {
            p.bg_fills = None;
        }
        if p.bg_prerender_state == PAUSE_BG_PRERENDER_PROCESS {
            // (Sched_FlushTaskQueue, PreRender_ApplyFilters: see docs/adr/0047.)
            p.bg_prerender_state = PAUSE_BG_PRERENDER_READY;
        } else if p.bg_prerender_state >= PAUSE_BG_PRERENDER_MAX {
            p.bg_prerender_state = PAUSE_BG_PRERENDER_OFF;
        }
        if p.bg_prerender_state == PAUSE_BG_PRERENDER_SETUP {
            // PreRender_SaveFramebuffer, PreRender_DrawCoverage: the frame as drawn (its
            // fills), not shown (R_GRAPH_TASKSET00_FLAGS); the overlay elements skipped.
            p.bg_fills = fills;
            p.bg_prerender_state = PAUSE_BG_PRERENDER_PROCESS;
            return true;
        }
        if !p.is_paused() || p.bg_prerender_state < PAUSE_BG_PRERENDER_READY || !p.ovl_loaded {
            return false;
        }
        let s = p.state;
        if (PAUSE_STATE_OPENING_1..=PAUSE_STATE_SAVE_PROMPT).contains(&s) || (PAUSE_STATE_GAME_OVER_SHOW_MESSAGE..=PAUSE_STATE_CLOSING).contains(&s) {
            self.kaleido_scope_draw();
        }
        false
    }

    /// Whether `Play_Draw` draws the scene: not once the pause background is saved
    /// (`PreRender_RestoreFramebuffer` and a jump to the overlay elements from `PROCESS` on).
    pub fn pause_bg_ready(&self) -> bool {
        self.pause_ctx.bg_prerender_state >= PAUSE_BG_PRERENDER_PROCESS
    }

    /// What `Interface_Draw` reads of the menu: the START button while paused (not in a game
    /// over), the item page's equip in flight.
    pub fn hud_pause(&self) -> crate::interface::HudPause {
        let p = &self.pause_ctx;
        let equip = (p.state == PAUSE_STATE_MAIN && p.main_state == PAUSE_MAIN_STATE_3).then(|| crate::interface::EquipAnim {
            item: p.equip_target_item,
            x: p.equip_anim_x,
            y: p.equip_anim_y,
            size: p.regs.wreg90,
            alpha: p.equip_anim_alpha,
            gray: p.equip_target_item < 0xBF && !check_age_req_item(p.equip_target_item as usize, self.save.adult),
        });
        crate::interface::HudPause { start: p.is_paused() && !p.is_game_over(), equip }
    }

    /// The menu's draws for the frame (`DrawLists::pause`), the cursor's vertices as the RSP
    /// reads them; the game over's message last, in screen space.
    pub fn kaleido_draw_cmds(&self) -> Vec<eng_gfx::DrawCmd> {
        let p = &self.pause_ctx;
        p.gfx.quads.iter().map(|q| q.draw_cmd(&p.cursor_vtx)).chain(p.gfx.rects.iter().map(|r| r.draw_cmd())).collect()
    }
}
