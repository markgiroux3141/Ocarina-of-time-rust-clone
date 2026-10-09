//! The pause menu (GAME-05 milestone 5b-1): its frame and the item page against the C
//! (`z_kaleido_setup.c`, `z_kaleido_scope_call.c`, `z_kaleido_scope.c`, `z_kaleido_item.c`,
//! `z_parameter.c`'s pause parts). Expected values are worked out from the C in the comments.
//! The REGs are `Regs_InitDataImpl`'s (`z_construct.c`): `R_PAUSE_UI_ANIMS_DURATION` 8, `WREG(4)`
//! 8, `R_PAUSE_BUTTON_LEFT_MOVE_OFFSET_X` 40, `_RIGHT_` -40, `R_PAUSE_STICK_REPEAT_DELAY_FIRST` 10,
//! `R_PAUSE_STICK_REPEAT_DELAY` 2, `WREG(87)` 80, `WREG(90)` 320, `WREG(91)` 40.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_CLEFT, BTN_CRIGHT, BTN_R, BTN_START, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::player::Action;
use oot_game::actor_ctx::ACTORCAT_ENEMY;
use oot_game::audio::sfx::*;
use oot_game::camera::CAM_ID_MAIN;
use oot_game::interface::{BTN_DISABLED, BTN_ENABLED, DO_ACTION_SAVE};
use oot_game::item::*;
use oot_game::kaleido::gfx::{KQuad, KTex};
use oot_game::kaleido::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn frame(w: &mut PlayState, prev: PadState, cur: PadState) -> PadState {
    w.tick_with(scripted_input(prev, cur));
    cur
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        frame(w, PadState::default(), PadState::default());
    }
}

fn press(w: &mut PlayState, button: u16) {
    frame(w, PadState::default(), PadState { button, ..Default::default() });
}

/// A push of the stick for a frame, then a release (a first push passes the repeat at once).
fn nudge(w: &mut PlayState, x: i8, y: i8) {
    frame(w, PadState::default(), PadState { button: 0, stick_x: x, stick_y: y });
    idle(w, 1);
}

fn sfx_on(w: &PlayState, frame: u32, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == frame && s == id)
}

/// Inside the Deku Tree's room 0 at its spawn with `save` (the sound log on), the room's
/// enemies gone, Link standing with the main camera and the transition over.
fn deku_tree_with(a: &Arc<GameAssets>, save: SaveContext) -> PlayState {
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 20);
    for h in w.actors.category(ACTORCAT_ENEMY).to_vec() {
        if let Some(a) = w.actors.actor_mut(h) {
            a.kill();
        }
    }
    idle(&mut w, 1);
    for _ in 0..600 {
        let p = w.player();
        if w.active_cam_id == CAM_ID_MAIN && p.action == Action::StandingStill && p.grounded() && w.transition.mode == oot_game::transition::TRANS_MODE_OFF && w.letterbox.size == 0 {
            return w;
        }
        idle(&mut w, 1);
    }
    panic!("never settled");
}

fn preset_save(a: &Arc<GameAssets>, preset: &str) -> SaveContext {
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset(preset).unwrap();
    save
}

fn deku_tree(a: &Arc<GameAssets>, preset: &str) -> PlayState {
    deku_tree_with(a, preset_save(a, preset))
}

/// Start, then idle until the menu is open and idle; the states each frame after Start's.
fn open(w: &mut PlayState) -> Vec<u16> {
    press(w, BTN_START);
    let mut states = vec![w.pause_ctx.state];
    while !(w.pause_ctx.state == PAUSE_STATE_MAIN && w.pause_ctx.main_state == PAUSE_MAIN_STATE_IDLE) {
        assert!(states.len() < 100, "the menu never opened: {states:?}");
        idle(w, 1);
        states.push(w.pause_ctx.state);
    }
    states
}

/// How many frames ended in each state, in order.
fn runs(states: &[u16]) -> Vec<(u16, usize)> {
    let mut v: Vec<(u16, usize)> = Vec::new();
    for &s in states {
        match v.last_mut() {
            Some((t, n)) if *t == s => *n += 1,
            _ => v.push((s, 1)),
        }
    }
    v
}

#[test]
fn start_opens_the_menu_frame_by_frame() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, "deku-tree-slingshot");
    let status = w.save.button_status;
    let start_frame = w.audio.frames + 1;
    press(&mut w, BTN_START);
    let p = &w.pause_ctx;
    // KaleidoSetup_Update: the page right of the last one viewed (the item page's: the map, its
    // eye PAUSE_EYE_DIST * -PAUSE_MAP_X), to scroll left (nextPageMode PAUSE_MAP * 2 + 1);
    // R_UPDATE_RATE 2, func_800F64E0(1): NA_SE_SY_WIN_OPEN.
    assert_eq!((p.page_index, p.eye, p.next_page_mode), (PAUSE_MAP, Vec3::new(-64.0, 0.0, 0.0), 3));
    assert_eq!(w.r_update_rate, 2);
    assert_eq!(w.frame_seconds(), 2.0 / 60.0);
    assert!(sfx_on(&w, start_frame, NA_SE_SY_WIN_OPEN));
    // With no letterbox, KaleidoScopeCall_Update the same frame: the prerender's setup and
    // PAUSE_STATE_WAIT_BG_PRERENDER; Play_Draw saves the frame (PROCESS).
    assert_eq!((p.state, p.bg_prerender_state), (PAUSE_STATE_WAIT_BG_PRERENDER, PAUSE_BG_PRERENDER_PROCESS));
    let mut states = vec![p.state];
    while !(w.pause_ctx.state == PAUSE_STATE_MAIN && w.pause_ctx.main_state == PAUSE_MAIN_STATE_IDLE) {
        idle(&mut w, 1);
        states.push(w.pause_ctx.state);
        let p = &w.pause_ctx;
        if p.state == PAUSE_STATE_OPENING_1 && p.item_page_pitch < 160.0 {
            // Each OPENING_1 update: the pitches 160 / 8 less, the alpha 255 / 16 more.
            let n = (160.0 - p.item_page_pitch) / 20.0;
            assert_eq!(p.alpha as f32, 15.0 * n);
        }
        assert!(states.len() < 40);
    }
    // The prerender's two frames (PROCESS, then READY), PAUSE_STATE_INIT's frame, the 8
    // OPENING_1 updates (the pitches to 0) and OPENING_2's until the scroll's 16th step.
    assert_eq!(runs(&states), vec![(PAUSE_STATE_WAIT_BG_PRERENDER, 2), (PAUSE_STATE_INIT, 1), (PAUSE_STATE_OPENING_1, 8), (PAUSE_STATE_OPENING_2, 8), (PAUSE_STATE_MAIN, 1)]);
    let p = &w.pause_ctx;
    // KaleidoScope_UpdateOpening's end: the item page, alpha 255, the eye 16 steps of (4, 4) on.
    assert_eq!((p.page_index, p.alpha, p.eye), (PAUSE_ITEM, 255, Vec3::new(0.0, 0.0, 64.0)));
    // The info panel 40 / 8 a frame up to 0; START's alpha 31 a frame, then 255; L and R 40 / 8 in.
    assert_eq!((p.info_panel_offset_y, w.interface_ctx.start_alpha), (0, 255));
    assert_eq!((p.regs.button_left_x, p.regs.button_right_x, p.regs.pages_y_origin_2), (-135, 115, 0));
    // gPageSwitchNextButtonStatus[PAUSE_MAP + PAGE_SWITCH_PT_LEFT]: the item page's buttons (A
    // off); B's label "SAVE" (Interface_LoadActionLabelB); the A button's "DECIDE".
    assert_eq!(w.save.button_status, [BTN_ENABLED, BTN_ENABLED, BTN_ENABLED, BTN_ENABLED, BTN_DISABLED]);
    assert!(w.interface_ctx.unk_1fa && w.interface_ctx.do_action_segment[1] == Some(DO_ACTION_SAVE));
    assert_eq!(w.interface_ctx.unk_1f0, oot_game::interface::DO_ACTION_DECIDE);
    // The saved buttons for the resume (sSavedButtonStatus, at PAUSE_STATE_INIT).
    assert_eq!(p.statics.saved_button_status, status);
    // KaleidoSetup_Init left cursorItem PAUSE_ITEM_NONE: the first idle draw pushes the cursor
    // right (stickAdjX 40), from slot 0 (sticks) to the nuts, with NA_SE_SY_CURSOR.
    assert_eq!((p.cursor_point[0], p.cursor_item[0]), (1, ITEM_DEKU_NUT as u16));
    assert!(sfx_on(&w, w.audio.frames, NA_SE_SY_CURSOR));
}

#[test]
fn the_cursor_moves_as_the_c_moves_it() {
    let Some(a) = assets() else { return };
    // Sticks in slot 0, nuts in 1, the slingshot in 6 (the second row's first).
    let mut w = deku_tree(&a, "deku-tree-slingshot");
    open(&mut w);
    let cur = |w: &PlayState| (w.pause_ctx.cursor_point[0], w.pause_ctx.cursor_special_pos);
    assert_eq!(cur(&w), (1, 0));
    // Right from the nuts: along the row, then the next rows from the starting column (1), never
    // column 0: back on the starting row, the right arrow (KaleidoScope_MoveCursorToSpecialPos:
    // NA_SE_SY_DECIDE), the point kept.
    nudge(&mut w, 80, 0);
    assert_eq!(cur(&w), (1, PAUSE_CURSOR_PAGE_RIGHT));
    assert!(sfx_on(&w, w.audio.frames - 1, NA_SE_SY_DECIDE));
    // From the right arrow, left: column by column from the last, top down: the nuts.
    nudge(&mut w, -80, 0);
    assert_eq!(cur(&w), (1, 0));
    // Left to the sticks; left again from column 0: the next rows from column 0 only, so the
    // left arrow.
    nudge(&mut w, -80, 0);
    assert_eq!(cur(&w), (0, 0));
    nudge(&mut w, -80, 0);
    assert_eq!(cur(&w), (0, PAUSE_CURSOR_PAGE_LEFT));
    // From the left arrow, right: column by column from the first, top down: the sticks.
    nudge(&mut w, 80, 0);
    assert_eq!(cur(&w), (0, 0));
    // Down to the slingshot; down again: no item below, the point restored (no wrap).
    nudge(&mut w, 0, -80);
    assert_eq!(cur(&w), (6, 0));
    assert_eq!(w.pause_ctx.cursor_item[0], ITEM_SLINGSHOT as u16);
    nudge(&mut w, 0, -80);
    assert_eq!(cur(&w), (6, 0));
    nudge(&mut w, 0, 80);
    assert_eq!(cur(&w), (0, 0));
}

#[test]
fn a_held_stick_moves_on_its_first_frame_then_after_10_and_every_3() {
    let Some(a) = assets() else { return };
    let mut save = preset_save(&a, "deku-tree-slingshot");
    // The first row full: bombs, the bow, fire arrows, Din's Fire after the sticks and nuts.
    for (slot, item) in [(2, ITEM_BOMB), (3, ITEM_BOW), (4, ITEM_ARROW_FIRE), (5, ITEM_DINS_FIRE)] {
        save.inventory.items[slot] = item;
    }
    let mut w = deku_tree_with(&a, save);
    open(&mut w);
    assert_eq!(w.pause_ctx.cursor_point[0], 1);
    // KaleidoScope_DrawPages' repeat: a first push passes (the timer 10); then each frame the
    // timer less one, the stick zeroed while it's 0 or more; under 0 it passes, the timer 2.
    let right = PadState { button: 0, stick_x: 80, stick_y: 0 };
    let mut moves = Vec::new();
    let mut prev = PadState::default();
    for f in 0..15 {
        let before = w.pause_ctx.cursor_point[0];
        prev = frame(&mut w, prev, right);
        if w.pause_ctx.cursor_point[0] != before {
            moves.push(f);
        }
    }
    assert_eq!(moves, vec![0, 11, 14]);
    assert_eq!(w.pause_ctx.cursor_point[0], 4);
}

#[test]
fn c_right_equips_the_slingshot_its_icon_flying_to_the_button() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, "deku-tree-slingshot-owned");
    assert_eq!(w.save.equips.button_items[3], ITEM_NONE);
    open(&mut w);
    nudge(&mut w, -80, 0);
    nudge(&mut w, 0, -80);
    assert_eq!(w.pause_ctx.cursor_point[0], 6);
    press(&mut w, BTN_CRIGHT);
    let p = &w.pause_ctx;
    // KaleidoScope_DrawItemSelect: PAUSE_MAIN_STATE_3 from the slot's top left times 10: slot 6
    // is the second row's first, x -96 + 2, y 64 - 6 - 32 - 2 (KaleidoScope_SetVertices).
    assert_eq!((p.main_state, p.equip_target_c_btn, p.equip_target_item, p.equip_target_slot), (PAUSE_MAIN_STATE_3, 2, ITEM_SLINGSHOT as u16, 6));
    assert_eq!((p.equip_anim_x, p.equip_anim_y, p.equip_anim_alpha), (-940, 240, 255));
    assert!(sfx_on(&w, w.audio.frames, NA_SE_SY_DECIDE));
    // KaleidoScope_UpdateItemEquip: each frame |anim - sCButtonPos| / sEquipMoveTimer towards
    // C-Right (1140, 1100): 2080 / 10 = 208 and 860 / 10 = 86, the same each frame after; the
    // icon's size WREG(90) less WREG(87) / sEquipMoveTimer (8 a frame), WREG(87) the same.
    for k in 1..10 {
        idle(&mut w, 1);
        let p = &w.pause_ctx;
        assert_eq!((p.equip_anim_x, p.equip_anim_y), (-940 + 208 * k, 240 + 86 * k), "frame {k}");
        assert_eq!((p.regs.wreg90, p.regs.wreg87), (320 - 8 * k, 80 - 8 * k), "frame {k}");
        assert_eq!(p.main_state, PAUSE_MAIN_STATE_3);
        // Interface_Draw's icon at equipAnim / 10, WREG(90) / 10 across.
        let hud = w.hud_pause();
        assert_eq!(hud.equip.map(|e| (e.x, e.y, e.size)), Some((p.equip_anim_x, p.equip_anim_y, p.regs.wreg90)));
    }
    // The tenth: at the button, the equip; WREG(90) back to 320, WREG(87) to WREG(91) (40, so
    // later equips shrink the icon 4 a frame).
    idle(&mut w, 1);
    let p = &w.pause_ctx;
    assert_eq!((p.equip_anim_x, p.equip_anim_y), (1140, 1100));
    assert_eq!((p.main_state, p.regs.wreg90, p.regs.wreg87), (PAUSE_MAIN_STATE_IDLE, 320, 40));
    assert_eq!((w.save.equips.button_items[3], w.save.equips.c_button_slots[2]), (ITEM_SLINGSHOT, 6));
    // Now C-Left from the slingshot (its slot on C-Right): C-Right gets C-Left's sticks.
    press(&mut w, BTN_CLEFT);
    idle(&mut w, 1);
    assert_eq!(w.pause_ctx.regs.wreg90, 316);
    idle(&mut w, 9);
    let b = w.save.equips.button_items;
    assert_eq!((b[1], b[3]), (ITEM_SLINGSHOT, ITEM_DEKU_STICK));
    assert_eq!(w.save.equips.c_button_slots, [6, SLOT_DEKU_NUT as u8, SLOT_DEKU_STICK as u8]);
}

#[test]
fn a_wrong_age_item_errors_and_its_icon_is_grey() {
    let Some(a) = assets() else { return };
    let mut save = preset_save(&a, "deku-tree-slingshot");
    save.inventory.items[SLOT_BOW] = ITEM_BOW;
    let mut w = deku_tree_with(&a, save);
    open(&mut w);
    // Right from the nuts to the bow (slot 2 empty).
    nudge(&mut w, 80, 0);
    assert_eq!(w.pause_ctx.cursor_point[0], 3);
    // CHECK_AGE_REQ_SLOT(SLOT_BOW) is the adult's: the name in grey, C-Left NA_SE_SY_ERROR.
    assert_eq!(w.pause_ctx.name_color_set, 1);
    press(&mut w, BTN_CLEFT);
    assert_eq!(w.pause_ctx.main_state, PAUSE_MAIN_STATE_IDLE);
    assert!(sfx_on(&w, w.audio.frames, NA_SE_SY_ERROR));
    assert_eq!(w.save.equips.button_items[1], ITEM_DEKU_STICK);
    // KaleidoScope_GrayOutTextureRGBA32 greyed it at PAUSE_STATE_INIT (!CHECK_AGE_REQ_ITEM).
    let icons: Vec<KTex> = w.pause_ctx.gfx.quads.iter().map(|q| q.tex).filter(|t| matches!(t, KTex::ItemIcon(_) | KTex::ItemIconGray(_))).collect();
    assert!(icons.contains(&KTex::ItemIconGray(ITEM_BOW)) && icons.contains(&KTex::ItemIcon(ITEM_SLINGSHOT)));
}

#[test]
fn r_turns_the_page_in_16_frames() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, "deku-tree-slingshot");
    open(&mut w);
    let f = w.audio.frames + 1;
    press(&mut w, BTN_R);
    let p = &w.pause_ctx;
    // KaleidoScope_SetupPageSwitch(PAGE_SWITCH_PT_RIGHT): nextPageMode PAUSE_ITEM * 2, the
    // cursor on the left arrow, NA_SE_SY_WIN_SCROLL_RIGHT, the map's buttons (the C buttons and
    // A off; B's not set on this version).
    assert_eq!((p.main_state, p.next_page_mode, p.cursor_special_pos), (PAUSE_MAIN_STATE_SWITCHING_PAGE, 0, PAUSE_CURSOR_PAGE_LEFT));
    assert_eq!(p.page_switch_timer, 4);
    assert!(sfx_on(&w, f, NA_SE_SY_WIN_SCROLL_RIGHT));
    assert_eq!(&w.save.button_status[1..], &[BTN_DISABLED; 4]);
    // KaleidoScope_UpdatePageSwitch: 16 steps of the eye (-4, -4), the first on R's own frame
    // (KaleidoScope_HandlePageToggles runs before the update's main state); L and R out 5 a step
    // for 8, then back.
    for k in 1..=16 {
        if k > 1 {
            idle(&mut w, 1);
        }
        let p = &w.pause_ctx;
        if k < 16 {
            assert_eq!(p.eye, Vec3::new(-4.0 * k as f32, 0.0, 64.0 - 4.0 * k as f32), "step {k}");
            let out = if k <= 8 { k } else { 16 - k };
            assert_eq!(p.regs.button_left_x, -135 - 5 * out, "step {k}");
        }
    }
    let p = &w.pause_ctx;
    assert_eq!((p.page_index, p.main_state, p.eye), (PAUSE_MAP, PAUSE_MAIN_STATE_IDLE, Vec3::new(-64.0, 0.0, 0.0)));
    assert_eq!((p.regs.button_left_x, p.regs.button_right_x), (-135, 115));
}

#[test]
fn start_closes_the_menu_and_the_game_resumes() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, "deku-tree-slingshot");
    let status = w.save.button_status;
    let frames = w.gameplay_frames;
    open(&mut w);
    let f = w.audio.frames + 1;
    press(&mut w, BTN_START);
    // PAUSE_STATE_CLOSING; func_800F64E0(0): NA_SE_SY_WIN_CLOSE (and the unmute) on Start's frame.
    assert_eq!(w.pause_ctx.state, PAUSE_STATE_CLOSING);
    assert!(sfx_on(&w, f, NA_SE_SY_WIN_CLOSE));
    // 8 frames of the pitches 20 up to 160 (the alpha 31 down, then 0), the next one
    // PAUSE_STATE_RESUME_GAMEPLAY, the next PAUSE_STATE_OFF.
    let mut states = Vec::new();
    while w.pause_ctx.is_paused() {
        idle(&mut w, 1);
        states.push(w.pause_ctx.state);
        assert!(states.len() < 20);
    }
    assert_eq!(runs(&states), vec![(PAUSE_STATE_CLOSING, 8), (PAUSE_STATE_RESUME_GAMEPLAY, 1), (PAUSE_STATE_OFF, 1)]);
    // R_UPDATE_RATE 3, the buttons as they were, B's label gone, the background prerender off.
    assert_eq!((w.r_update_rate, w.pause_ctx.bg_prerender_state, w.interface_ctx.start_alpha), (3, PAUSE_BG_PRERENDER_OFF, 0));
    assert_eq!(w.save.button_status, status);
    assert!(!w.interface_ctx.unk_1fa);
    // While paused nothing played (gameplayFrames stopped: from Start's frame, whose
    // KaleidoSetup_Update runs before Play_Update's IS_PAUSED, to the one that resumes).
    assert_eq!(w.gameplay_frames, frames);
}

#[test]
fn the_equipment_stand_in_runs_as_the_menu_resumes() {
    let Some(a) = assets() else { return };
    // The Kokiri Sword owned but not worn (what its chest leaves).
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    item_give(&mut save, None, ITEM_SWORD_KOKIRI);
    assert_eq!(save.cur_equip_value(EQUIP_TYPE_SWORD), 0);
    let mut w = deku_tree_with(&a, save);
    open(&mut w);
    assert_eq!(w.save.cur_equip_value(EQUIP_TYPE_SWORD), 0, "nothing happens while the menu's open");
    press(&mut w, BTN_START);
    while w.pause_ctx.is_paused() {
        idle(&mut w, 1);
    }
    assert_eq!(w.save.cur_equip_value(EQUIP_TYPE_SWORD), EQUIP_VALUE_SWORD_KOKIRI);
    assert_eq!(w.save.equips.button_items[0], ITEM_SWORD_KOKIRI);
    assert_eq!(w.player().current_sword_item_id, ITEM_SWORD_KOKIRI);
}

#[test]
fn start_does_nothing_during_the_scenes_fade_in() {
    let Some(a) = assets() else { return };
    let save = preset_save(&a, "deku-tree-slingshot");
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 1);
    // KaleidoSetup_Update: transitionMode isn't TRANS_MODE_OFF yet.
    assert_ne!(w.transition.mode, oot_game::transition::TRANS_MODE_OFF);
    press(&mut w, BTN_START);
    assert_eq!((w.pause_ctx.state, w.r_update_rate), (PAUSE_STATE_OFF, 3));
}

/// Every quad of a session through the menu: open, the cursor about, an equip, the four pages,
/// closed.
fn session_quads(w: &mut PlayState) -> Vec<(KQuad, Vec<oot_game::kaleido::gfx::Vtx>)> {
    let mut all = Vec::new();
    let grab = |w: &PlayState, all: &mut Vec<_>| {
        for q in &w.pause_ctx.gfx.quads {
            all.push((q.clone(), w.pause_ctx.cursor_vtx.clone()));
        }
    };
    press(w, BTN_START);
    for _ in 0..30 {
        grab(w, &mut all);
        idle(w, 1);
    }
    for (x, y) in [(-80, 0), (0, -80), (80, 0), (80, 0)] {
        nudge(w, x, y);
        grab(w, &mut all);
    }
    nudge(w, -80, 0);
    press(w, BTN_CRIGHT);
    for _ in 0..12 {
        grab(w, &mut all);
        idle(w, 1);
    }
    for _ in 0..4 {
        press(w, BTN_R);
        for _ in 0..20 {
            grab(w, &mut all);
            idle(w, 1);
        }
    }
    press(w, BTN_START);
    for _ in 0..12 {
        grab(w, &mut all);
        idle(w, 1);
    }
    all
}

#[test]
fn every_quad_drawn_is_baked_and_covers_its_whole_texture() {
    let Some(a) = assets() else { return };
    let Some(pack) = pack() else { return };
    let mut save = preset_save(&a, "deku-tree-slingshot-owned");
    save.inventory.items[SLOT_BOW] = ITEM_BOW;
    let mut w = deku_tree_with(&a, save);
    let quads = session_quads(&mut w);
    assert!(quads.len() > 1000);
    let baked = oot_game::kaleido::gfx::bake_list();
    let mut kinds = std::collections::BTreeSet::new();
    for (q, cursor) in &quads {
        assert!(baked.contains(&(q.tex, q.cc)), "{:?} under {:?} isn't baked", q.tex, q.cc);
        kinds.insert((q.tex, q.cc));
        // The recorder's premise: the quad's texture coordinates span its texture (10.5).
        let v = q.resolve(cursor);
        let (w, h) = q.tex.size();
        assert_eq!((v[1].tc[0] as u32, v[2].tc[1] as u32), (w * 32, h * 32), "{:?}", q.tex);
    }
    for (t, c) in kinds {
        let key = oot_game::pack::keys::bake(&oot_game::kaleido::gfx::bake_name(t, c));
        assert!(pack.assets.try_get::<eng_gfx::DrawList>(&key).ok().flatten().is_some(), "{key} isn't in the pack");
    }
    // The HUD's pause part: START's label ("Return": DO_ACTION_RETURN), B's ("SAVE").
    for action in [oot_game::interface::DO_ACTION_RETURN, DO_ACTION_SAVE] {
        let key = oot_game::pack::keys::bake(&format!("hud/do_action_rect/{action:02x}"));
        assert!(pack.assets.try_get::<eng_gfx::DrawList>(&key).ok().flatten().is_some(), "{key}");
    }
}

#[test]
fn a_quads_bake_is_a_unit_quad_coloured_vertex_by_vertex() {
    let Some(pack) = pack() else { return };
    use oot_game::kaleido::gfx::{Cc, MESH_ORDER, bake_name};
    // The bake's triangles in the C's quadrangle order (0, 2, 3) and (0, 3, 1) over the unit
    // quad's corners (0, 0), (1, 0), (0, -1), (1, -1): the order DrawParams::vertex_colors follows.
    let key = oot_game::pack::keys::bake(&bake_name(KTex::Cursor(0), Cc::PrimEnvTexel));
    let d = pack.assets.try_get::<eng_gfx::DrawList>(&key).unwrap().unwrap();
    let verts: Vec<[f32; 2]> = d.batches.iter().flat_map(|b| b.vertices.iter().map(|v| [v.pos.x, v.pos.y])).collect();
    let corner = [[0.0, 0.0], [1.0, 0.0], [0.0, -1.0], [1.0, -1.0]];
    assert_eq!(verts, MESH_ORDER.iter().map(|&k| corner[k]).collect::<Vec<_>>());
}

#[test]
fn the_scene_behind_the_menu_stops() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, "deku-tree-slingshot");
    open(&mut w);
    let draw = |w: &PlayState| {
        let mut out = oot_game::play::DrawOut::default();
        let f = w.current_frame();
        w.draw(&f, &oot_game::play::ViewInfo::new(Vec3::ZERO, glam::Mat4::IDENTITY), &mut out);
        out
    };
    let (gf, time) = (w.gameplay_frames, w.save.day_time);
    let before = draw(&w);
    idle(&mut w, 10);
    let after = draw(&w);
    // Play_Update's actors, effects and cameras stop; Environment_Update's time with them; the
    // scene's lists (Play_Draw doesn't draw it over the saved frame) the same.
    assert_eq!((w.gameplay_frames, w.save.day_time), (gf, time));
    assert_eq!((before.opa, before.xlu), (after.opa, after.xlu));
    // The menu's list, in its own perspective.
    assert!(!after.pause.is_empty());
    assert_eq!(after.pause_view, Some(oot_game::kaleido::gfx::PAUSE_VIEW));
}
