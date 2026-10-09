//! Saving (GAME-05 milestone 5c) in the game against the C: B's save prompt in the pause menu
//! (`KaleidoScope_Update`'s `PAUSE_STATE_MAIN` and `PAUSE_STATE_SAVE_PROMPT`,
//! `KaleidoScope_DrawPages`' save prompt page, `z_kaleido_scope.c`), the game over's
//! `Sram_WriteSave`, and the exit run (`Route::Save`): saved from the menu, the console's reset,
//! the file loaded back. `z_sram.c` itself is `oot_game`'s `sram::tests`. Expected values are
//! worked out from the C in the comments; the REGs are `Regs_InitDataImpl`'s (`z_construct.c`):
//! `R_PAUSE_UI_ANIMS_DURATION` 8, `R_PAUSE_BUTTON_LEFT_MOVE_OFFSET_X` 40, `_RIGHT_` -40.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, BTN_B, BTN_START, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::player::Action;
use oot_game::actor_ctx::ACTORCAT_ENEMY;
use oot_game::audio::sfx::*;
use oot_game::camera::CAM_ID_MAIN;
use oot_game::collision_check::HIT_SPECIAL_EFFECT_NONE;
use oot_game::interface::{BTN_DISABLED, BTN_ENABLED};
use oot_game::item::*;
use oot_game::kaleido::gfx::{Cc, KTex};
use oot_game::kaleido::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::sram::*;

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

fn sfx_on(w: &PlayState, frame: u32, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == frame && s == id)
}

/// Inside the Deku Tree at its spawn on the `deku-tree-save` preset as a debug start makes it
/// (file 2, "ZELDAZ", an SRAM in memory), the sound log on, the room's enemies gone, Link
/// standing with the main camera and the transition over.
fn deku_tree(a: &Arc<GameAssets>) -> PlayState {
    use oot_actors::playthrough::Route;
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    let save = Route::Save.save(e);
    assert_eq!((save.file_num, &save.newf, save.health), (1, b"ZELDAZ", 0x20));
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

/// Start, then idle until the menu is open and idle.
fn open(w: &mut PlayState) {
    press(w, BTN_START);
    for _ in 0..100 {
        if w.pause_ctx.state == PAUSE_STATE_MAIN && w.pause_ctx.main_state == PAUSE_MAIN_STATE_IDLE {
            return;
        }
        idle(w, 1);
    }
    panic!("the menu never opened");
}

/// B, then idle until the prompt waits on a choice; the prompt's pitch each frame from B's.
fn open_prompt(w: &mut PlayState) -> Vec<f32> {
    press(w, BTN_B);
    let mut pitches = vec![w.pause_ctx.prompt_pitch];
    while w.pause_ctx.save_prompt_state != PAUSE_SAVE_PROMPT_STATE_WAIT_CHOICE {
        assert!(pitches.len() < 20, "the prompt never turned in");
        idle(w, 1);
        pitches.push(w.pause_ctx.prompt_pitch);
    }
    pitches
}

/// Idle until the game resumes; the pause states and the prompt's pitches after each frame.
fn until_resumed(w: &mut PlayState) -> (Vec<u16>, Vec<f32>) {
    let (mut states, mut pitches) = (Vec::new(), Vec::new());
    while w.pause_ctx.is_paused() {
        idle(w, 1);
        states.push(w.pause_ctx.state);
        pitches.push(w.pause_ctx.prompt_pitch);
        assert!(states.len() < 30, "the menu never closed: {states:?}");
    }
    (states, pitches)
}

#[test]
fn b_opens_the_save_prompt_and_it_turns_in() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a);
    open(&mut w);
    let (left, right) = (w.pause_ctx.regs.button_left_x, w.pause_ctx.regs.button_right_x);
    let f = w.audio.frames + 1;
    let pitches = open_prompt(&mut w);
    // B: NA_SE_SY_DECIDE, nextPageMode 0, promptChoice 0 (Yes), every button disabled but A
    // (buttonStatus[4]), HUD_VISIBILITY_ALL, PAUSE_SAVE_PROMPT_STATE_APPEARING.
    assert!(sfx_on(&w, f, NA_SE_SY_DECIDE));
    assert_eq!(w.save.button_status, [BTN_DISABLED, BTN_DISABLED, BTN_DISABLED, BTN_DISABLED, BTN_ENABLED]);
    assert_eq!((w.save.hud_visibility_mode, w.pause_ctx.next_page_mode, w.pause_ctx.prompt_choice), (50, 0, 0));
    // B's frame left promptPitch at -314; then 314 / 8 a frame to -628 (WAIT_CHOICE on the 8th).
    let want: Vec<f32> = (0..9).map(|i| -314.0 - 39.25 * i as f32).collect();
    assert_eq!(pitches, want);
    // L and R out: 40 / 8 a frame each way.
    assert_eq!((w.pause_ctx.regs.button_left_x, w.pause_ctx.regs.button_right_x), (left - 40, right + 40));
    // The page looked at (the item page) is drawn half a turn on from the prompt: -628 + 314.
    assert_eq!((w.pause_ctx.state, w.pause_ctx.item_page_pitch), (PAUSE_STATE_SAVE_PROMPT, -314.0));
}

#[test]
fn no_b_and_start_close_the_prompt_without_saving() {
    let Some(a) = assets() else { return };
    for how in ["no", "b", "start"] {
        let mut w = deku_tree(&a);
        let status = w.save.button_status;
        open(&mut w);
        open_prompt(&mut w);
        let f = w.audio.frames + 1;
        match how {
            // The stick right: "No" (KaleidoScope_UpdatePrompt), then A.
            "no" => {
                frame(&mut w, PadState::default(), PadState { button: 0, stick_x: 80, stick_y: 0 });
                assert_eq!(w.pause_ctx.prompt_choice, 4);
                idle(&mut w, 1);
                let f = w.audio.frames + 1;
                press(&mut w, BTN_A);
                assert!(sfx_on(&w, f, NA_SE_SY_WIN_CLOSE), "{how}");
            }
            "b" => press(&mut w, BTN_B),
            _ => press(&mut w, BTN_START),
        }
        if how != "no" {
            assert!(sfx_on(&w, f, NA_SE_SY_WIN_CLOSE), "{how}");
        }
        // PAUSE_SAVE_PROMPT_STATE_CLOSING: YREG(8) the prompt's pitch, the pages' origin lowered,
        // the buttons back but A (still enabled).
        let p = &w.pause_ctx;
        assert_eq!((p.save_prompt_state, p.regs.yreg8, p.regs.pages_y_origin_2), (PAUSE_SAVE_PROMPT_STATE_CLOSING, -628, PAUSE_PAGES_Y_ORIGIN_2_LOWER), "{how}");
        assert_eq!(w.save.button_status, [BTN_ENABLED; 5], "{how}");
        // 8 frames of 160 / 8 to YREG(8) + 160 (the alpha 0 on the last), the next
        // RESUME_GAMEPLAY, the next OFF; nothing written.
        let (states, pitches) = until_resumed(&mut w);
        assert_eq!(states, [vec![PAUSE_STATE_SAVE_PROMPT; 8], vec![PAUSE_STATE_RESUME_GAMEPLAY, PAUSE_STATE_OFF]].concat(), "{how}");
        assert_eq!(&pitches[..8], &(1..=8).map(|i| -628.0 + 20.0 * i as f32).collect::<Vec<_>>()[..], "{how}");
        assert_eq!((w.sram.writes, w.save.button_status, w.r_update_rate), (0, status, 3), "{how}");
    }
}

#[test]
fn yes_saves_and_the_menu_closes() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a);
    let status = w.save.button_status;
    // A chest's flag, for Play_SaveSceneFlags to take into the save.
    w.flags.chest |= 1 << 3;
    open(&mut w);
    open_prompt(&mut w);
    let f = w.audio.frames + 1;
    press(&mut w, BTN_A);
    // NA_SE_SY_PIECE_OF_HEART, Play_SaveSceneFlags, savedSceneId (the Deku Tree, 0),
    // Sram_WriteSave: file 2 and its backup.
    assert!(sfx_on(&w, f, NA_SE_SY_PIECE_OF_HEART));
    assert_eq!((w.pause_ctx.save_prompt_state, w.save.saved_scene_id, w.save.scene_flags[0].chest & (1 << 3)), (PAUSE_SAVE_PROMPT_STATE_SAVED, 0, 1 << 3));
    assert_eq!(w.sram.writes, 2);
    let (o, k) = (SRAM_SLOT_OFFSETS[1] as usize, SRAM_SLOT_OFFSETS[4] as usize);
    let slot = w.sram.bytes[o..o + SLOT_SIZE].to_vec();
    assert_eq!(&slot[..], &w.sram.bytes[k..k + SLOT_SIZE]);
    assert_eq!(checksum(&slot[..SAVE_SIZE]), u16::from_be_bytes([slot[CHECKSUM_OFFSET], slot[CHECKSUM_OFFSET + 1]]));
    // What was saved: two hearts, the Deku Tree, the chest's flag (sceneFlags[0].chest at 0xD4).
    assert_eq!(u16::from_be_bytes([slot[HEALTH], slot[HEALTH + 1]]), 0x20);
    assert_eq!(u32::from_be_bytes([slot[0xD4], slot[0xD5], slot[0xD6], slot[0xD7]]) & (1 << 3), 1 << 3);
    // GameCube: sDelayTimer 3, no "Game saved.": two frames SAVED, the third
    // CLOSING_AFTER_SAVED; then 8 frames turning away, RESUME_GAMEPLAY, OFF.
    let (states, _) = until_resumed(&mut w);
    assert_eq!(states, [vec![PAUSE_STATE_SAVE_PROMPT; 11], vec![PAUSE_STATE_RESUME_GAMEPLAY, PAUSE_STATE_OFF]].concat());
    assert_eq!((w.save.button_status, w.sram.writes), (status, 2));
    // A in SAVED closes it at once (the timer isn't counted down on that frame).
    let mut w = deku_tree(&a);
    open(&mut w);
    open_prompt(&mut w);
    press(&mut w, BTN_A);
    idle(&mut w, 1);
    press(&mut w, BTN_A);
    assert_eq!((w.pause_ctx.save_prompt_state, w.pause_ctx.statics.delay_timer), (PAUSE_SAVE_PROMPT_STATE_CLOSING_AFTER_SAVED, 2));
}

#[test]
fn the_save_prompt_page_is_drawn_as_the_c_draws_it() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a);
    open(&mut w);
    open_prompt(&mut w);
    // The four pages, then the prompt page (then the UI overlay): SAVE_TEXS(LANGUAGE_ENG)'s 15 tiles (by column, its
    // second column's top gPauseSave10ENGTex); outside a game over the prompt's quads are at
    // sVtxPagePromptQuadsY's (the game over's YREG(60..64) are 22 lower): "Would you like to
    // save?" (x -76, 152 wide; y 36), the cursor on "Yes" (y 10, 48 square) in (100, 255, 100,
    // R_KALEIDO_PROMPT_CURSOR_ALPHA), "Yes" and "No" (y -6).
    let p = &w.pause_ctx;
    let q = &p.gfx.quads;
    let n = 4 + q.iter().position(|q| q.tex == KTex::Label("gPauseSavePromptENGTex", 152)).expect("the message");
    let tiles: Vec<_> = q[n - 19..n - 4].iter().map(|q| (q.tex, q.cc)).collect();
    assert_eq!(tiles[0], (KTex::PageBg("gPauseSave00Tex"), Cc::ModulateIa));
    assert_eq!(tiles[5], (KTex::PageBg("gPauseSave10ENGTex"), Cc::ModulateIa));
    assert_eq!(tiles[14], (KTex::PageBg("gPauseSave24Tex"), Cc::ModulateIa));
    let rest: Vec<_> = q[n - 4..n].iter().map(|q| (q.tex, q.cc)).collect();
    assert_eq!(
        rest,
        vec![
            (KTex::Label("gPauseSavePromptENGTex", 152), Cc::ModulateIa),
            (KTex::PromptCursor, Cc::PrimTexelAlpha),
            (KTex::Label("gPauseYesENGTex", 48), Cc::ModulateIa),
            (KTex::Label("gPauseNoENGTex", 48), Cc::ModulateIa),
        ]
    );
    assert_eq!(q[n - 3].prim, [100, 255, 100, p.regs.prompt_cursor_alpha as u8]);
    let v = |i: usize| q[i].resolve(&p.cursor_vtx);
    assert_eq!((v(n - 4)[0].ob, v(n - 4)[3].ob), ([-76, 36, 0], [76, 20, 0]));
    assert_eq!((v(n - 3)[0].ob, v(n - 3)[3].ob), ([-58, 10, 0], [-10, -38, 0]));
    assert_eq!((v(n - 2)[0].ob, v(n - 1)[0].ob), ([-58, -6, 0], [10, -6, 0]));
    // Saved: the tiles alone ("Game saved." is !PLATFORM_GC's).
    press(&mut w, BTN_A);
    let q = &w.pause_ctx.gfx.quads;
    let last_tile = q.iter().rposition(|q| q.tex == KTex::PageBg("gPauseSave24Tex")).expect("the page");
    assert!(!q.iter().any(|q| matches!(q.tex, KTex::Label("gPauseSavePromptENGTex", _) | KTex::PromptCursor | KTex::Label("gPauseYesENGTex", _))));
    assert!(!matches!(q.get(last_tile + 1).map(|q| q.tex), Some(KTex::Label(..)) if q[last_tile + 1].cc == Cc::ModulateIa));
}

#[test]
fn every_quad_of_the_save_prompt_is_baked() {
    let Some(a) = assets() else { return };
    let Some(pack) = pack() else { return };
    let mut w = deku_tree(&a);
    open(&mut w);
    let mut all = Vec::new();
    let mut grab = |w: &PlayState| all.extend(w.pause_ctx.gfx.quads.iter().map(|q| (q.clone(), w.pause_ctx.cursor_vtx.clone())));
    press(&mut w, BTN_B);
    for _ in 0..10 {
        grab(&w);
        idle(&mut w, 1);
    }
    for x in [80, -80] {
        frame(&mut w, PadState::default(), PadState { button: 0, stick_x: x, stick_y: 0 });
        grab(&w);
        idle(&mut w, 1);
    }
    press(&mut w, BTN_A);
    while w.pause_ctx.is_paused() {
        grab(&w);
        idle(&mut w, 1);
    }
    let baked = oot_game::kaleido::gfx::bake_list();
    let mut kinds = std::collections::BTreeSet::new();
    for (q, cursor) in &all {
        assert!(baked.contains(&(q.tex, q.cc)), "{:?} under {:?} isn't baked", q.tex, q.cc);
        kinds.insert((q.tex, q.cc));
        let v = q.resolve(cursor);
        let (w, h) = q.tex.size();
        assert_eq!((v[1].tc[0] as u32, v[2].tc[1] as u32), (w * 32, h * 32), "{:?}", q.tex);
    }
    assert!(kinds.contains(&(KTex::PageBg("gPauseSave10ENGTex"), Cc::ModulateIa)));
    for (t, c) in kinds {
        let key = oot_game::pack::keys::bake(&oot_game::kaleido::gfx::bake_name(t, c));
        assert!(pack.assets.try_get::<eng_gfx::DrawList>(&key).ok().flatten().is_some(), "{key} isn't in the pack");
    }
}

#[test]
fn the_game_overs_yes_writes_the_save() {
    let Some(a) = assets() else { return };
    // Inside the Deku Tree with a quarter heart (file 1's slot: a plain SaveContext::new), a
    // dummy that hurts in front of Link.
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-quarter-heart").unwrap();
    save.newf = *b"ZELDAZ";
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    for _ in 0..200 {
        let p = w.player();
        if p.action == Action::StandingStill && w.active_cam_id == CAM_ID_MAIN && w.transition.mode == oot_game::transition::TRANS_MODE_OFF {
            break;
        }
        idle(&mut w, 1);
    }
    let pos = w.player().actor.world_pos;
    let yaw = w.player().actor.shape_rot.y;
    w.spawn_hurting_target(pos + Vec3::new(eng_math::sin_s(yaw), 0.0, eng_math::cos_s(yaw)) * 20.0, HIT_SPECIAL_EFFECT_NONE);
    for _ in 0..800 {
        if w.pause_ctx.state == PAUSE_STATE_GAME_OVER_SAVE_PROMPT {
            break;
        }
        idle(&mut w, 1);
    }
    assert_eq!(w.pause_ctx.state, PAUSE_STATE_GAME_OVER_SAVE_PROMPT);
    assert_eq!(w.sram.writes, 0);
    press(&mut w, BTN_A);
    // Sram_WriteSave: file 1 and its backup, the death counted, Link dead (health 0).
    assert_eq!((w.pause_ctx.state, w.sram.writes), (PAUSE_STATE_GAME_OVER_SAVED, 2));
    let o = SRAM_SLOT_OFFSETS[0] as usize;
    let slot = &w.sram.bytes[o..o + SAVE_SIZE];
    assert_eq!((u16::from_be_bytes([slot[DEATHS], slot[DEATHS + 1]]), u16::from_be_bytes([slot[HEALTH], slot[HEALTH + 1]])), (1, 0));
    // Loaded (file 1, through the map select's entrance): Sram_OpenSave's three hearts.
    let mut sram = w.sram.clone();
    let s = oot_game::file_select::load_game(&mut sram, 1, Some(e)).unwrap();
    assert_eq!((s.health, s.deaths, s.saved_scene_id), (0x30, 1, 0));
}

#[test]
fn exit_save_from_the_menu_then_load_the_file_back() {
    use oot_actors::playthrough::{Playthrough, Route, SAVE_C_BUTTON, SAVE_FILE, Step};
    let Some(a) = assets() else { return };
    let route = Route::Save;
    let e = a.scenes.entrance_index(route.entrance()).unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), route.save(e)).expect("Play_Init");
    route.debug_start(&mut w);
    assert_eq!((w.save.health, w.save.equips.button_items[SAVE_C_BUTTON + 1]), (0x20, ITEM_NONE));
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    let mut scene_changes = None;
    while let Some(pad) = run.next(&w) {
        if let Some(file) = run.take_reset() {
            assert_eq!(file, SAVE_FILE);
            // What was saved, as the reset finds it.
            let o = SRAM_SLOT_OFFSETS[SAVE_FILE - 1] as usize;
            assert!(slot_occupied(&w.sram.bytes, SAVE_FILE - 1));
            assert_eq!(w.sram.bytes[o + 0x68 + 1 + SAVE_C_BUTTON], ITEM_SLINGSHOT);
            scene_changes = Some(w.scene_changes);
            w.console_reset(file, None).expect("the reset");
        }
        prev = frame(&mut w, prev, pad);
        match run.take_done() {
            Some(Step::Saved) => assert_eq!(w.sram.writes, 2),
            Some(Step::MenuClosed) => assert_eq!(w.pause_ctx.state, PAUSE_STATE_OFF),
            _ => {}
        }
    }
    assert!(run.failure.is_none() && run.finished(), "{:?} at {}", run.failure, run.at());
    let names: Vec<_> = run.steps.iter().map(|(s, _)| s.name()).collect();
    assert_eq!(names, ["menu_opened", "cursor_on_item", "item_equipped", "save_prompt", "saved", "menu_closed", "loaded"]);
    // The file loaded: a new play state (Play_Init), at the Deku Tree's entrance (savedSceneId
    // SCENE_DEKU_TREE: sDungeonEntrances), three hearts (Sram_OpenSave), the slingshot on C-Left,
    // file 2's.
    assert_eq!(w.scene_changes, scene_changes.unwrap() + 1);
    assert_eq!((w.save.entrance_index, w.scene_id, w.save.file_num), (e, 0, 1));
    assert_eq!((w.save.health, w.save.equips.button_items[SAVE_C_BUTTON + 1]), (0x30, ITEM_SLINGSHOT));
    assert_eq!(w.player().action, Action::StandingStill);
    // The SRAM came across: the save, and the header the reset's Sram_InitSram wrote.
    assert_eq!(w.sram.writes, 3);
    assert_eq!(&w.sram.bytes[..12], &SRAM_DEFAULT_HEADER);
    println!("steps: {:?}", run.steps.iter().map(|(s, f)| (s.name(), f)).collect::<Vec<_>>());
}
