//! The game over's screens (GAME-05 milestone 5b-2) against the C: `KaleidoScope_DrawGameOver`,
//! `KaleidoScope_DrawPages`' prompt page in a game over, and the game over's states' fields the
//! draw reads (`KaleidoScope_Update`'s `PAUSE_STATE_GAME_OVER_*`, `z_kaleido_scope.c`). The states'
//! timing is `oot_actors --test damage`'s (ADR 0032). Expected values are worked out from the C in
//! the comments; the REGs are `Regs_InitDataImpl`'s (`z_construct.c`): `VREG(87)` 64, `VREG(89)`
//! 0, `R_PAUSE_UI_ANIMS_DURATION` 8, `WREG(4)` 8, the prompt's quads' y `YREG(60..64)` 14, -2, -2,
//! -18, -18.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::player::Action;
use oot_game::camera::CAM_ID_MAIN;
use oot_game::collision_check::HIT_SPECIAL_EFFECT_NONE;
use oot_game::kaleido::gfx::{Cc, GameOverPart, KTex};
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

/// Inside the Deku Tree at its spawn with a quarter heart, a dummy that hurts in front of Link:
/// idle until the game over's message is drawn (`PAUSE_STATE_GAME_OVER_SHOW_MESSAGE`, its first
/// frame: `INIT` turned into it).
fn dying(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-quarter-heart").unwrap();
    assert_eq!(save.health, 4);
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
    let ahead = pos + Vec3::new(eng_math::sin_s(yaw), 0.0, eng_math::cos_s(yaw)) * 20.0;
    w.spawn_hurting_target(ahead, HIT_SPECIAL_EFFECT_NONE);
    for _ in 0..400 {
        if w.pause_ctx.state == PAUSE_STATE_GAME_OVER_SHOW_MESSAGE {
            return w;
        }
        idle(&mut w, 1);
    }
    panic!("no game over (pause state {}, health {})", w.pause_ctx.state, w.save.health);
}

/// The C's colour step (`KaleidoScope_Update`'s game over states): `ABS(c - target) / timer`
/// towards the target.
fn approach(c: &mut i16, target: i16, timer: i16) {
    let s = (*c - target).abs() / timer;
    if *c >= target {
        *c -= s;
    } else {
        *c += s;
    }
}

#[test]
fn the_message_scrolls_its_mask_and_takes_its_colours() {
    let Some(a) = assets() else { return };
    let mut w = dying(&a);
    // INIT's: D_8082AB8C.. (255, 130, 0, 0), (30, 0, 0); VREG(88) 98; the frame's draw:
    // VREG(89) 0 - 2, tile 1's ult (VREG(89) & 0x7F) 126; three 64x32 rectangles from
    // (VREG(87), VREG(88)).
    let p = &w.pause_ctx;
    assert_eq!(p.regs.vreg88, 98);
    let r = &p.gfx.rects;
    assert_eq!(
        r.iter().map(|r| (r.part, r.rect)).collect::<Vec<_>>(),
        vec![(GameOverPart::P1, [64.0, 98.0, 128.0, 130.0]), (GameOverPart::P2, [128.0, 98.0, 192.0, 130.0]), (GameOverPart::P3, [192.0, 98.0, 256.0, 130.0]),]
    );
    assert!(r.iter().all(|r| (r.prim, r.env, r.mask_ult) == ([255, 130, 0, 0], [30, 0, 0, 255], 126)));
    // The pages aren't drawn in a game over, nor the UI overlay: the prompt page only.
    assert!(p.gfx.quads.iter().all(|q| matches!(q.tex, KTex::PageBg(s) if s.starts_with("gPauseSave") || s == "gPauseGameOver10Tex")));
    // SHOW_MESSAGE: 30 updates step the prim colour to (30, 0, 0, 255) and env to (255, 130, 0)
    // (the 30th sets them); each draw scrolls the mask 2 more.
    let (mut prim, mut env) = ([255i16, 130, 0, 0], [30i16, 0, 0]);
    let mut timer = 30i16;
    let mut vreg89 = -2i16;
    while w.pause_ctx.state == PAUSE_STATE_GAME_OVER_SHOW_MESSAGE {
        for (c, t) in prim.iter_mut().zip([30, 0, 0, 255]) {
            approach(c, t, timer);
        }
        for (c, t) in env.iter_mut().zip([255, 130, 0]) {
            approach(c, t, timer);
        }
        timer -= 1;
        if timer == 0 {
            (prim, env) = ([30, 0, 0, 255], [255, 130, 0]);
        }
        vreg89 -= 2;
        idle(&mut w, 1);
        let r = &w.pause_ctx.gfx.rects[0];
        assert_eq!((r.prim, r.env, r.mask_ult), (prim.map(|c| c as u8), [env[0] as u8, env[1] as u8, env[2] as u8, 255], (vreg89 & 0x7F) as u16));
    }
    assert_eq!(timer, 0);
    assert_eq!(w.pause_ctx.state, PAUSE_STATE_GAME_OVER_WINDOW_DELAY);
}

#[test]
fn the_window_turns_in_then_the_prompts() {
    let Some(a) = assets() else { return };
    let mut w = dying(&a);
    while w.pause_ctx.state != PAUSE_STATE_GAME_OVER_SHOW_WINDOW {
        idle(&mut w, 1);
    }
    // WINDOW_DELAY's last update turned it to SHOW_WINDOW: the prompt page's draw set the page
    // looked at's (the item page's) pitch to promptPitch + 314 (-434 + 314).
    assert_eq!((w.pause_ctx.prompt_pitch, w.pause_ctx.item_page_pitch), (-434.0, -120.0));
    // SHOW_WINDOW's first update, the next frame's: promptPitch -434 - 160 / 8, the four pages'
    // pitches with it (the item page's then promptPitch + 314 again in the draw),
    // infoPanelOffsetY -40 + 5, startAlpha + 31, VREG(88) 98 - 3, the buttons -175 + 5 and
    // 155 - 5, alpha + 255 / 16.
    idle(&mut w, 1);
    let p = &w.pause_ctx;
    assert_eq!((p.prompt_pitch, p.item_page_pitch, p.map_page_pitch, p.quest_page_pitch, p.equip_page_pitch), (-454.0, -140.0, -454.0, -454.0, -454.0));
    assert_eq!((p.info_panel_offset_y, p.regs.vreg88, p.regs.button_left_x, p.regs.button_right_x, p.alpha), (-35, 95, -170, 150, 15));
    // ... then until promptPitch is past -628 (10 updates): -628, startAlpha 255, VREG(88) 66,
    // R_PAUSE_PAGES_Y_ORIGIN_2 0, alpha 255, SAVE_PROMPT.
    while w.pause_ctx.state == PAUSE_STATE_GAME_OVER_SHOW_WINDOW {
        idle(&mut w, 1);
    }
    let p = &w.pause_ctx;
    assert_eq!(p.state, PAUSE_STATE_GAME_OVER_SAVE_PROMPT);
    assert_eq!((p.prompt_pitch, p.regs.vreg88, p.regs.pages_y_origin_2, p.alpha, w.interface_ctx.start_alpha), (-628.0, 66, 0, 255, 255));
    // The prompt page (drawn the frame it got there): its page's pitch promptPitch + 314, the 15
    // tiles (sGameOverTexs, by column), "Would you like to save?" on the message's quad (x -76,
    // 152 wide; YREG(60) 14), the cursor's on "Yes" (quad 1: YREG(61) -2, 48 square) in
    // (100, 255, 100, R_KALEIDO_PROMPT_CURSOR_ALPHA), "Yes" and "No" (YREG(63), YREG(64) -18)
    // under G_CC_MODULATEIA.
    assert_eq!(p.item_page_pitch, -314.0);
    let q = &p.gfx.quads;
    let tiles: Vec<_> = q.iter().take(15).map(|q| q.tex).collect();
    assert_eq!(tiles[0], KTex::PageBg("gPauseSave00Tex"));
    assert_eq!(tiles[5], KTex::PageBg("gPauseGameOver10Tex"));
    assert_eq!(tiles[14], KTex::PageBg("gPauseSave24Tex"));
    let rest: Vec<_> = q.iter().skip(15).map(|q| (q.tex, q.cc)).collect();
    assert_eq!(
        rest,
        vec![
            (KTex::Label("gPauseSavePromptENGTex", 152), Cc::ModulateIa),
            (KTex::PromptCursor, Cc::PrimTexelAlpha),
            (KTex::Label("gPauseYesENGTex", 48), Cc::ModulateIa),
            (KTex::Label("gPauseNoENGTex", 48), Cc::ModulateIa),
        ]
    );
    let cursor = &q[16];
    assert_eq!(cursor.prim, [100, 255, 100, p.regs.prompt_cursor_alpha as u8]);
    let v = |i: usize| q[i].resolve(&p.cursor_vtx);
    assert_eq!((v(15)[0].ob, v(15)[3].ob), ([-76, 14, 0], [76, -2, 0]));
    assert_eq!((v(16)[0].ob, v(16)[3].ob), ([-58, -2, 0], [-10, -50, 0]));
    assert_eq!((v(17)[0].ob, v(18)[0].ob), ([-58, -18, 0], [10, -18, 0]));
    // The stick right: "No" (promptChoice 4), the cursor on quad 2 (x 10); A: CONTINUE_PROMPT,
    // "Continue playing?" on the message's quad, the cursor back on "Yes".
    frame(&mut w, PadState::default(), PadState { button: 0, stick_x: 80, stick_y: 0 });
    let p = &w.pause_ctx;
    assert_eq!(p.prompt_choice, 4);
    let c = p.gfx.quads.iter().find(|q| q.tex == KTex::PromptCursor).unwrap().resolve(&p.cursor_vtx);
    assert_eq!(c[0].ob, [10, -2, 0]);
    frame(&mut w, PadState::default(), PadState { button: BTN_A, ..Default::default() });
    let p = &w.pause_ctx;
    assert_eq!((p.state, p.prompt_choice), (PAUSE_STATE_GAME_OVER_CONTINUE_PROMPT, 0));
    let rest: Vec<_> = p.gfx.quads.iter().skip(15).map(|q| q.tex).collect();
    assert_eq!(rest, vec![KTex::ContinuePlaying, KTex::PromptCursor, KTex::Label("gPauseYesENGTex", 48), KTex::Label("gPauseNoENGTex", 48)]);
    // The message is still drawn, at VREG(88) 66.
    assert_eq!(p.gfx.rects[0].rect, [64.0, 66.0, 128.0, 98.0]);
}

#[test]
fn every_quad_and_rectangle_of_the_game_over_is_baked() {
    let Some(a) = assets() else { return };
    let pack = oot_game::pack::GamePack::open_default().unwrap();
    let mut w = dying(&a);
    let baked = oot_game::kaleido::gfx::bake_list();
    let has = |key: String| pack.assets.try_get::<eng_gfx::DrawList>(&oot_game::pack::keys::bake(&key)).ok().flatten().is_some();
    let mut n = 0;
    for _ in 0..200 {
        let p = &w.pause_ctx;
        for q in &p.gfx.quads {
            assert!(baked.contains(&(q.tex, q.cc)), "{:?} under {:?} isn't baked", q.tex, q.cc);
            assert!(has(oot_game::kaleido::gfx::bake_name(q.tex, q.cc)), "{:?} isn't in the pack", q.tex);
            let v = q.resolve(&p.cursor_vtx);
            let (tw, th) = q.tex.size();
            assert_eq!((v[1].tc[0] as u32, v[2].tc[1] as u32), (tw * 32, th * 32), "{:?}", q.tex);
            n += 1;
        }
        for r in &p.gfx.rects {
            assert!(has(oot_game::kaleido::gfx::game_over_bake_name(r.part)), "{:?}", r.part);
        }
        // Through the save prompt ("No") to the continue prompt.
        if p.state == PAUSE_STATE_GAME_OVER_CONTINUE_PROMPT {
            break;
        }
        let pad = match (p.state, p.prompt_choice) {
            (PAUSE_STATE_GAME_OVER_SAVE_PROMPT, 0) => PadState { button: 0, stick_x: 80, stick_y: 0 },
            (PAUSE_STATE_GAME_OVER_SAVE_PROMPT, _) => PadState { button: BTN_A, ..Default::default() },
            _ => PadState::default(),
        };
        frame(&mut w, PadState::default(), pad);
    }
    assert!(n > 1000);
    // The rectangles' draws: screen space in the menu's list, tile 1's scroll in SEG_TILE.
    let cmds = w.kaleido_draw_cmds();
    let last = cmds.last().unwrap();
    assert!(last.params.screen && last.mesh.name.ends_with("game_over/P3"));
}

#[test]
fn exit_the_game_over_run() {
    use oot_actors::playthrough::{Playthrough, Route, Step};
    let Some(a) = assets() else { return };
    let route = Route::GameOver;
    let e = a.scenes.entrance_index(route.entrance()).unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), route.save(e)).expect("Play_Init");
    route.debug_start(&mut w);
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    let mut states = Vec::new();
    for _ in 0..route.max_frames() {
        let Some(pad) = run.next(&w) else { break };
        prev = frame(&mut w, prev, pad);
        states.push(w.pause_ctx.state);
    }
    assert!(run.failure.is_none() && run.finished(), "{:?} at {}", run.failure, run.at());
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::Died, Step::SavePrompt, Step::ContinuePrompt, Step::Respawned]);
    // The message was drawn from SHOW_MESSAGE to FINISH; respawned with 3 hearts, unpaused.
    assert!(states.contains(&PAUSE_STATE_GAME_OVER_FINISH));
    assert_eq!((w.pause_ctx.state, w.save.health), (PAUSE_STATE_OFF, 0x30));
    assert_eq!(w.player().action, Action::StandingStill);
}
