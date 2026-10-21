//! GAME-10 milestone 2 against the C: Zora's Domain's diving game (`z_en_diving_game.c`) and its
//! rupees (`z_en_ex_ruppy.c`'s type 0). The placement (`ootx scene-info --scene spot07`): both
//! rooms, `En_Diving_Game` 0xFFFF at (-149, 857, -1020), yaw -23848, standing in the upper pool
//! (`sZorasDomainWaterBox`, surface 877, floor 857); the pool below the falls has its surface at 0
//! and its floor at -120 where the rupees sink.
//!
//! Child Link enters at `ENTR_ZORAS_DOMAIN_0` (spawn 0, room 1) with `fairy-magic`, the Domain's
//! intro seen (`EVENTCHKINF_A4`) and 50 rupees, at 10:00, the sound log on; he talks to the Zora
//! from 50 in front of him (-187, 857, -1053), in the upper pool. Expected values are worked out
//! from the C in the comments; nothing depends on a `Rand` draw's value.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_diving_game::*;
use oot_actors::en_ex_ruppy::{self, EnExRuppy};
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::bgm::start_seq;
use oot_game::audio::sfx::{NA_SE_EV_BOMB_DROP_WATER, NA_SE_SY_FOUND, NA_SE_SY_GET_RUPY};
use oot_game::audio::{NA_BGM_SMALL_ITEM_GET, NA_BGM_TIMED_MINI_GAME, SEQ_PLAYER_BGM_MAIN};
use oot_game::camera::CAM_ID_MAIN;
use oot_game::collision_check::MASS_IMMOVABLE;
use oot_game::item::UPG_SCALE;
use oot_game::message::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::*;

/// The Zora's placement.
const ZORA_POS: Vec3 = Vec3::new(-149.0, 857.0, -1020.0);
/// Where Link talks to him: 50 in front of him (his yaw -23848), facing him.
const TALK_POS: Vec3 = Vec3::new(-187.0, 857.0, -1053.0);
const TALK_YAW: i16 = 0x2000;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Link in Zora's Domain (room 1) with 50 rupees and `setup`, two frames on.
fn domain(a: &Arc<GameAssets>, setup: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_ZORAS_DOMAIN_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("fairy-magic").unwrap();
    // EVENTCHKINF_A4: the Domain's intro seen.
    save.set_event_chk_inf(0xA4);
    save.rupees = 50;
    setup(&mut save);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    w.audio_side = Some(Box::new(oot_game::audio::offline::OfflineAudio::new(&a.pack.audio_data().unwrap(), false)));
    idle(&mut w, 2);
    w
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn a_button() -> PadState {
    PadState { button: BTN_A, ..Default::default() }
}

fn press_a(w: &mut PlayState) {
    w.tick_with(scripted_input(PadState::default(), a_button()));
    w.tick_with(scripted_input(a_button(), PadState::default()));
}

/// Link let go (`Player_SetCsActionWithHaltedActors(.., PLAYER_CSACTION_7)`): the let-go waiting,
/// or already taken by Player (in water at once, `func_808513BC`'s hold over).
fn let_go(w: &PlayState) {
    let cs = w.player().cs_mode;
    assert!(cs == PLAYER_CSACTION_7 || cs == 0, "Link still held: {cs}");
}

/// The live diving game Zoras.
fn zoras(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnDivingGame>(h).is_some_and(|g| !g.actor.killed)).collect()
}

fn zora(w: &PlayState) -> ActorHandle {
    let z = zoras(w);
    assert_eq!(z.len(), 1, "one diving game Zora");
    z[0]
}

fn dg(w: &PlayState, h: ActorHandle) -> &EnDivingGame {
    w.actors.downcast::<EnDivingGame>(h).expect("En_Diving_Game")
}

/// The live diving game rupees, in the order thrown (their `throwDistance` counting down).
fn rupees(w: &PlayState) -> Vec<ActorHandle> {
    let mut v: Vec<ActorHandle> = w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnExRuppy>(h).is_some_and(|r| !r.actor.killed && r.ty == en_ex_ruppy::ENEXRUPPY_TYPE_DIVING_GAME)).collect();
    v.sort_by_key(|&h| -ruppy(w, h).throw_distance);
    v
}

fn ruppy(w: &PlayState, h: ActorHandle) -> &EnExRuppy {
    w.actors.downcast::<EnExRuppy>(h).expect("En_Ex_Ruppy")
}

fn sfx_count(w: &PlayState, id: u16) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).count()
}

fn seq_cmd_queued(w: &PlayState, cmd: u32) -> bool {
    w.audio.log.as_ref().unwrap().seq_cmds.iter().any(|&(_, c)| c == cmd)
}

/// Through texts until `f` holds (at most `max` frames): A on a frame of its own whenever a text
/// waits (done, an event, a choice: `choice`), each text id recorded.
fn answer_until(w: &mut PlayState, max: usize, what: &str, texts: &mut Vec<u16>, choice: u8, f: impl Fn(&PlayState) -> bool) {
    let mut pressed = false;
    for _ in 0..max {
        if f(w) {
            return;
        }
        let st = w.message_state();
        if st == TEXT_STATE_CHOICE && !pressed {
            w.msg_ctx.choice_index = choice;
        }
        let a = !pressed && matches!(st, TEXT_STATE_CHOICE | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_EVENT | TEXT_STATE_AWAITING_NEXT);
        let mut record = |w: &PlayState| {
            if w.message_state() != TEXT_STATE_NONE && w.msg_ctx.text_id != 0 && texts.last() != Some(&w.msg_ctx.text_id) {
                texts.push(w.msg_ctx.text_id);
            }
        };
        record(w);
        let pad = if a { a_button() } else { PadState::default() };
        w.tick_with(scripted_input(PadState::default(), pad));
        pressed = a;
        record(w);
    }
    panic!("{what}: not after {max} frames (msgMode {:#x}, text {:#x}, texts {texts:x?})", w.msg_ctx.msg_mode, w.msg_ctx.text_id);
}

/// Link at `TALK_POS` facing the Zora, idle until he's Link's talk target (`Actor_OfferTalk`
/// within 80) with the message box idle; then A until a text opens.
fn talk_to_zora(w: &mut PlayState) {
    w.place_player(TALK_POS, TALK_YAW);
    idle(w, 2);
    let h = zora(w);
    for _ in 0..600 {
        if w.player().target_actor == Some(h) && w.msg_ctx.msg_mode == 0 {
            break;
        }
        idle(w, 1);
    }
    for _ in 0..10 {
        press_a(w);
        if w.message_state() != TEXT_STATE_NONE {
            return;
        }
    }
    let p = w.player();
    panic!("the Zora never talked: Link at {} ({}, cs {}), target {:?}, msgMode {:#x}, Zora {:?}", p.actor.world_pos, p.action.decomp_name(), p.cs_mode, p.target_actor, w.msg_ctx.msg_mode, dg(w, h).action);
}

/// Talked to, "yes": until the frame `EnDivingGame_SetupRupeeThrowCam` ran (the update that made
/// the action `ThrowRupees`).
fn start_the_throw(w: &mut PlayState, texts: &mut Vec<u16>) -> ActorHandle {
    let h = zora(w);
    talk_to_zora(w);
    answer_until(w, 3000, "the throw", texts, 0, |w| dg(w, h).action == Action::ThrowRupees);
    h
}

/// The throw, the sinking and 0x405A read: the game running.
fn start_the_game(w: &mut PlayState) -> ActorHandle {
    let mut texts = Vec::new();
    let h = start_the_throw(w, &mut texts);
    answer_until(w, 3000, "the game", &mut texts, 0, |w| dg(w, h).state == ENDIVINGGAME_STATE_PLAYING);
    // Player's talk ends on the box's closing frame (Player_Action_Talk: ACTOR_FLAG_TALK cleared,
    // the cutscene's let-go, PLAYER_CSACTION_7, taken): Link isn't moved before.
    for _ in 0..30 {
        if w.player().cs_mode == 0 {
            break;
        }
        idle(w, 1);
    }
    assert_eq!(w.player().cs_mode, 0);
    h
}

/// Link put on rupee `r` (on the pool's floor, under water) for a frame: `EnExRuppy_WaitInGame`
/// takes it within 30.
fn take(w: &mut PlayState, r: ActorHandle) {
    let pos = ruppy(w, r).actor.world_pos;
    w.place_player(pos, 0);
    idle(w, 1);
}

/// Until every rupee waits on the bottom (`EnExRuppy_WaitInGame`).
fn until_all_wait(w: &mut PlayState) {
    for _ in 0..600 {
        let rs = rupees(w);
        if !rs.is_empty() && rs.iter().all(|&r| ruppy(w, r).action == en_ex_ruppy::Action::WaitInGame) {
            return;
        }
        idle(w, 1);
    }
    panic!("the rupees never settled");
}

#[test]
fn one_zora_for_both_rooms() {
    let Some(a) = assets() else { return };
    let mut w = domain(&a, |_| {});
    // EnDivingGame_Init: room -1, scale (0.01, 0.013, 0.014), ATTENTION_RANGE_0, immovable; the
    // first update's EnDivingGame_Reset: gZoraIdleAnim, the fidget, EnDivingGame_Idle.
    let h = zora(&w);
    let g = dg(&w, h);
    assert_eq!(g.actor.world_pos, ZORA_POS);
    assert_eq!(g.actor.room, -1);
    assert_eq!(g.actor.scale, Vec3::new(0.01, 0.012999999, 0.0139999995));
    assert_eq!(g.actor.target_mode, 0);
    assert_eq!(g.actor.col_chk_info.mass, MASS_IMMOVABLE);
    assert_eq!(g.actor.shape_shadow_scale, 30.0);
    assert!(!g.is_duplicate);
    idle(&mut w, 2);
    let g = dg(&w, h);
    assert_eq!(g.action, Action::Idle);
    assert!(g.not_playing_minigame);
    // Offered before the scale: 0x4053, a choice, phase 1.
    assert_eq!((g.actor.text_id, g.next_text_state, g.phase), (0x4053, TEXT_STATE_CHOICE, ENDIVINGGAME_PHASE_1));
    assert!(statics(&mut w).is_initialized);

    // The other room's placement (sIsInitialized set): a duplicate, killed in its init; its
    // destroy leaves the timer alone (a running timer stands in for the game's).
    w.save.timer_seconds = 30;
    w.save.timer_state = TIMER_STATE_DOWN_TICK;
    let dup = w.actor_spawn(ACTOR_EN_DIVING_GAME, ZORA_POS, [0, -23848, 0], -1).expect("spawned");
    let d = dg(&w, dup);
    assert!(d.is_duplicate);
    assert!(d.actor.killed);
    assert_eq!(d.actor.room, -1);
    idle(&mut w, 2);
    assert!(w.actors.downcast::<EnDivingGame>(dup).is_none(), "deleted");
    assert_eq!(w.save.timer_state, TIMER_STATE_DOWN_TICK);
    assert_eq!(zoras(&w), vec![h]);
}

#[test]
fn no_and_too_few_rupees_end_the_talk() {
    let Some(a) = assets() else { return };
    // "No": 0x2D, nothing paid; its event closes it, Link let go (PLAYER_CSACTION_7), idle again.
    let mut w = domain(&a, |_| {});
    let h = zora(&w);
    talk_to_zora(&mut w);
    // The talk taken: Link held (PLAYER_CSACTION_8), EnDivingGame_HandlePlayChoice.
    assert_eq!(w.msg_ctx.text_id, 0x4053);
    assert_eq!(dg(&w, h).action, Action::HandlePlayChoice);
    assert_eq!(w.player().cs_mode, PLAYER_CSACTION_8);
    let mut texts = Vec::new();
    answer_until(&mut w, 3000, "no", &mut texts, 1, |w| dg(w, h).action == Action::Idle);
    assert_eq!(texts, vec![0x4053, 0x002D]);
    assert_eq!(w.save.rupees + w.save.rupee_accumulator, 50);
    let g = dg(&w, h);
    assert_eq!((g.state, g.phase), (ENDIVINGGAME_STATE_NOTPLAYING, ENDIVINGGAME_PHASE_ENDED));
    let_go(&w);
    // Offered again the next frame: phase 1 with the text.
    idle(&mut w, 1);
    assert_eq!(dg(&w, h).phase, ENDIVINGGAME_PHASE_1);

    // "Yes" with 19 rupees: 0x85, nothing paid, the same end.
    let mut w = domain(&a, |s| s.rupees = 19);
    let h = zora(&w);
    talk_to_zora(&mut w);
    let mut texts = Vec::new();
    answer_until(&mut w, 3000, "too few", &mut texts, 0, |w| dg(w, h).action == Action::Idle);
    assert_eq!(texts, vec![0x4053, 0x0085]);
    assert_eq!(w.save.rupees + w.save.rupee_accumulator, 19);
    let_go(&w);
}

#[test]
fn twenty_rupees_and_the_throw() {
    let Some(a) = assets() else { return };
    let mut w = domain(&a, |_| {});
    let mut texts = Vec::new();
    let h = start_the_throw(&mut w, &mut texts);
    // "Yes" with 50: Rupees_ChangeBy(-20), 0x4054; its event pauses the box (MSGMODE_PAUSED), Link
    // held; gZoraThrowRupeesAnim from frame 15: the sub camera.
    assert_eq!(texts, vec![0x4053, 0x4054]);
    assert_eq!(w.save.rupees + w.save.rupee_accumulator, 30);
    assert_eq!(w.msg_ctx.msg_mode, MSGMODE_PAUSED);
    assert_eq!(w.player().cs_mode, PLAYER_CSACTION_8);
    let g = dg(&w, h);
    assert!(!g.not_playing_minigame);
    assert!(g.skel.cur_frame >= 15.0);
    // EnDivingGame_SetupRupeeThrowCam: the main camera waits, the sub camera is active, from the
    // view's eye (+80 up, +250 along z) at its at; five to throw, the first in 10 frames.
    let sub = g.sub_cam_id;
    assert_ne!(sub, CAM_ID_MAIN);
    assert_eq!(w.active_cam_id, sub);
    assert_eq!((g.rupees_left_to_throw, g.spawn_ruppy_timer, g.sub_cam_timer), (5, 10, 60));
    assert_eq!((g.sub_cam_at_next, g.sub_cam_eye_next), (SUB_CAM_AT_NEXT, SUB_CAM_EYE_NEXT));
    assert_eq!(g.sub_cam_vel_factor, 0.0);
    assert_eq!(g.sub_cam_eye_vel, (g.sub_cam_eye - SUB_CAM_EYE_NEXT).abs() * 0.04);
    let eye_y = g.sub_cam_eye.y;

    // A rupee every 5 frames from the 10th (S + 10, 15, 20, 25, 30): En_Ex_Ruppy type 0, his
    // child, about 30 above him, its world.rot.z (throwDistance) the rupees left (5 to 1), thrown
    // at 12 forward and 6 up, turned Rand_CenteredFloat(3500) - 1000; throwTimer 30.
    let mut thrown = Vec::new();
    for t in 1..=60 {
        idle(&mut w, 1);
        let rs = rupees(&w);
        let expected = if t < 10 { 0 } else { ((t - 10) / 5 + 1).min(5) };
        assert_eq!(rs.len(), expected, "frame S + {t}");
        if rs.len() > thrown.len() {
            let r = *rs.last().unwrap();
            let e = ruppy(&w, r);
            assert_eq!(e.throw_distance, 5 - thrown.len() as i16);
            assert_eq!(e.actor.parent, Some(h));
            assert_eq!(e.actor.world_rot.z, 0);
            assert!((-1000 - 1750..=-1000 + 1750).contains(&(e.actor.world_rot.y as i32)), "{}", e.actor.world_rot.y);
            let home = e.actor.home_pos;
            assert!((home.x - ZORA_POS.x).abs() <= 15.0 && (home.y - (ZORA_POS.y + 30.0)).abs() <= 10.0 && (home.z - ZORA_POS.z).abs() <= 10.0, "{home}");
            // Before the scale: blue, 5 rupees each.
            assert_eq!((e.rupee_value, e.color_idx), (5, 1));
            assert_eq!(dg(&w, h).throw_timer, 30);
            thrown.push(r);
        }
        // The pan: the eye's height never eases (@bug (game)).
        assert_eq!(dg(&w, h).sub_cam_eye.y, eye_y);
        let g = dg(&w, h);
        if t < 60 {
            assert_eq!(g.action, Action::ThrowRupees, "frame S + {t}");
        }
    }
    assert!(dg(&w, h).all_rupees_thrown);
    // S + 60: the pan timed out (60 frames) with the rupees still in the air (none under water):
    // EnDivingGame_SetupUnderwaterViewCs. The last throw's throwTimer ran out the same frame, so
    // every rupee's fall has ended: hidden, stopped, waiting (EnExRuppy_EnterWater).
    assert_eq!(dg(&w, h).action, Action::SetupUnderwaterViewCs);
    assert_eq!(dg(&w, h).throw_timer, 0);
    for &r in &thrown {
        let e = ruppy(&w, r);
        assert_eq!(e.action, en_ex_ruppy::Action::EnterWater);
        assert_eq!(e.invisible, 1);
        assert_eq!((e.actor.speed_xz, e.actor.velocity, e.actor.gravity), (0.0, Vec3::ZERO, 0.0));
    }
    // S + 61: the cut to the pool (rupeePhase 1, the camera at its targets); S + 62 the pan is
    // there at once: the rupees sink (rupeePhase 2) for 70 frames.
    idle(&mut w, 1);
    let g = dg(&w, h);
    assert_eq!((g.action, g.rupee_phase, g.sub_cam_timer), (Action::ThrowRupees, ENDIVINGGAME_RUPEE_PHASE_SWITCHING_CAM, 100));
    assert_eq!((g.sub_cam_at, g.sub_cam_eye), (SUB_CAM_AT_NEXT, SUB_CAM_EYE_NEXT));
    idle(&mut w, 1);
    let g = dg(&w, h);
    assert_eq!((g.action, g.rupee_phase, g.sub_cam_timer), (Action::UnderwaterViewCs, ENDIVINGGAME_RUPEE_PHASE_SINKING, 70));
    let cam = w.camera(sub).expect("the sub camera");
    assert_eq!((cam.at, cam.eye), (SUB_CAM_AT_NEXT, SUB_CAM_EYE_NEXT));
    // The same frame each rupee is shown in the pool (EnExRuppy_EnterWater): x -260 ± 150,
    // y 370 ± 100, z -500 - 50 × throwDistance ± 40, falling (gravity -1, a frame of it moved).
    for &r in &thrown {
        let e = ruppy(&w, r);
        assert_eq!(e.action, en_ex_ruppy::Action::Sink);
        assert_eq!(e.invisible, 0);
        assert_eq!(e.actor.gravity, -1.0);
        assert_eq!(e.actor.velocity.y, -1.0);
        let p = e.actor.world_pos;
        let z0 = -500.0 - 50.0 * e.throw_distance as f32;
        assert!((p.x + 260.0).abs() <= 150.0 && p.y <= 470.0 - 1.0 && p.y >= 270.0 - 1.0 && (p.z - z0).abs() <= 40.0, "{p} ({})", e.throw_distance);
    }
    // 70 frames on (S + 132): the main camera back, 0x405A continuing the paused box.
    for t in 63..132 {
        idle(&mut w, 1);
        assert_eq!(dg(&w, h).action, Action::UnderwaterViewCs, "frame S + {t}");
        assert_eq!(w.active_cam_id, sub);
    }
    idle(&mut w, 1);
    assert_eq!(dg(&w, h).action, Action::StartMinigame);
    assert_eq!(w.active_cam_id, CAM_ID_MAIN);
    assert!(w.camera(sub).is_none());
    assert_eq!(w.msg_ctx.text_id, 0x405A);
}

#[test]
fn a_rupee_sinks_and_is_taken() {
    let Some(a) = assets() else { return };
    let mut w = domain(&a, |_| {});
    let mut texts = Vec::new();
    let h = start_the_throw(&mut w, &mut texts);
    let splashes = sfx_count(&w, NA_SE_EV_BOMB_DROP_WATER);
    // Each rupee more than 15 under the pool's surface (0): EnExRuppy_Sink's splash
    // (EffectSsGSplash at the surface, NA_SE_EV_BOMB_DROP_WATER), -1 down, gravity -0.2, then
    // waiting on the floor (-120), a bubble every 10 frames.
    let mut seen = std::collections::HashSet::new();
    for _ in 0..400 {
        let before: Vec<(ActorHandle, en_ex_ruppy::Action)> = rupees(&w).into_iter().map(|r| (r, ruppy(&w, r).action)).collect();
        idle(&mut w, 1);
        for (r, act) in before {
            let e = ruppy(&w, r);
            if act == en_ex_ruppy::Action::Sink && e.action == en_ex_ruppy::Action::WaitInGame {
                // The check read the last frame's depth; this frame -1 + -0.2.
                assert!((e.actor.velocity.y - -1.2).abs() < 1e-5, "{}", e.actor.velocity.y);
                assert_eq!(e.actor.gravity, -0.2);
                assert!(e.actor.world_pos.y < -15.0 + 2.0, "{}", e.actor.world_pos.y);
                seen.insert(r);
            }
        }
        if seen.len() == 5 {
            break;
        }
    }
    assert_eq!(seen.len(), 5, "every rupee sank");
    // Five sinking splashes (plus the five drops' ends, which came before).
    assert!(sfx_count(&w, NA_SE_EV_BOMB_DROP_WATER) >= splashes + 5);
    // Down to the floor.
    idle(&mut w, 200);
    for r in rupees(&w) {
        let e = ruppy(&w, r);
        assert!((e.actor.world_pos.y - -120.0).abs() < 1.0, "{}", e.actor.world_pos.y);
        assert_eq!(e.action, en_ex_ruppy::Action::WaitInGame);
        assert!(e.timer <= 10);
    }

    // 0x405A read (EnDivingGame_StartMinigame): the timer at 50 (TIMER_STATE_DOWN_INIT), the timed
    // mini-game's music (func_800F5ACC), Link let go, 0x405B for a talk, playing.
    answer_until(&mut w, 3000, "the game", &mut texts, 0, |w| dg(w, h).state == ENDIVINGGAME_STATE_PLAYING);
    assert_eq!(texts, vec![0x4053, 0x4054, 0x405A]);
    let g = dg(&w, h);
    assert_eq!((g.action, g.actor.text_id, g.next_text_state), (Action::Idle, 0x405B, TEXT_STATE_EVENT));
    assert_eq!(w.save.timer_seconds, 50);
    assert_eq!(w.save.timer_state, TIMER_STATE_DOWN_INIT);
    assert!(seq_cmd_queued(&w, start_seq(SEQ_PLAYER_BGM_MAIN, 0, NA_BGM_TIMED_MINI_GAME)));
    let_go(&w);

    // Link on a rupee: Rupees_ChangeBy(5), NA_SE_SY_GET_RUPY, grabbedRupees 1, gone; the game goes on.
    let money = w.save.rupees + w.save.rupee_accumulator;
    let got = sfx_count(&w, NA_SE_SY_GET_RUPY);
    let r = rupees(&w)[0];
    take(&mut w, r);
    assert!(w.actors.downcast::<EnExRuppy>(r).is_none_or(|e| e.actor.killed), "taken");
    assert_eq!(rupees(&w).len(), 4);
    assert_eq!(dg(&w, h).grabbed_rupees, 1);
    assert_eq!(w.save.rupees + w.save.rupee_accumulator, money + 5);
    assert_eq!(sfx_count(&w, NA_SE_SY_GET_RUPY), got + 1);
    idle(&mut w, 1);
    assert_eq!(dg(&w, h).state, ENDIVINGGAME_STATE_PLAYING);
}

#[test]
fn five_rupees_win_the_silver_scale() {
    let Some(a) = assets() else { return };
    let mut w = domain(&a, |_| {});
    let h = start_the_game(&mut w);
    until_all_wait(&mut w);
    let money = w.save.rupees + w.save.rupee_accumulator;
    // Four taken: no win yet (EnDivingGame_HasMinigameFinished wants 5 before the scale).
    for i in 0..4 {
        let r = rupees(&w)[0];
        take(&mut w, r);
        idle(&mut w, 1);
        assert_eq!(dg(&w, h).grabbed_rupees, i + 1);
        assert_eq!(dg(&w, h).state, ENDIVINGGAME_STATE_PLAYING);
    }
    // The fifth: counted in the rupee's update (after his), the win in his next: the timer off,
    // the music back (func_800F5B58), NA_BGM_SMALL_ITEM_GET, 0x4055 at once, Link held, the game's
    // fields cleared, EnDivingGame_EarnedScale.
    let r = rupees(&w)[0];
    take(&mut w, r);
    assert_eq!(dg(&w, h).grabbed_rupees, 5);
    idle(&mut w, 1);
    assert_eq!(w.save.rupees + w.save.rupee_accumulator, money + 25);
    let g = dg(&w, h);
    assert_eq!(g.action, Action::EarnedScale);
    assert_eq!((g.state, g.phase, g.rupee_phase, g.grabbed_rupees, g.all_rupees_thrown), (0, 0, 0, 0, false));
    assert_eq!(w.save.timer_state, TIMER_STATE_OFF);
    assert_eq!(w.audio.fanfare_seq_id, NA_BGM_SMALL_ITEM_GET);
    assert_eq!(w.msg_ctx.text_id, 0x4055);
    assert_eq!(w.player().cs_mode, PLAYER_CSACTION_8);
    // Read: Link let go, the prize waiting (EnDivingGame_Idle in ENDIVINGGAME_STATE_AWARDPRIZE: 0x4056).
    let mut texts = Vec::new();
    answer_until(&mut w, 3000, "the win", &mut texts, 0, |w| dg(w, h).action == Action::Idle);
    assert_eq!(texts, vec![0x4055]);
    assert_eq!(dg(&w, h).state, ENDIVINGGAME_STATE_AWARDPRIZE);
    let_go(&w);
    idle(&mut w, 20);
    assert_eq!(dg(&w, h).actor.text_id, 0x4056);

    // Talked to again: 0x4056; read, Actor_OfferGetItem(GI_SCALE_SILVER) within 90 across and 10
    // up or down until Link takes it; its text read, EVENTCHKINF_OBTAINED_SILVER_SCALE and idle.
    assert_eq!(w.save.cur_upg_value(UPG_SCALE), 0);
    talk_to_zora(&mut w);
    let mut texts = Vec::new();
    answer_until(&mut w, 3000, "the scale", &mut texts, 0, |w| dg(w, h).action == Action::EndMinigameAfterScaleGiven);
    assert_eq!(texts.first(), Some(&0x4056));
    // Taken (Actor_HasParent): the flag waits for the item's text (sGetItemTable's 0xCD).
    assert!(!w.save.get_event_chk_inf(EVENTCHKINF_OBTAINED_SILVER_SCALE));
    answer_until(&mut w, 3000, "the scale's text", &mut texts, 0, |w| dg(w, h).action == Action::Reset);
    assert_eq!(texts, vec![0x4056, 0x00CD]);
    assert_eq!(w.save.cur_upg_value(UPG_SCALE), 1, "the silver scale");
    assert!(w.save.get_event_chk_inf(EVENTCHKINF_OBTAINED_SILVER_SCALE));
    idle(&mut w, 1);
    let g = dg(&w, h);
    assert_eq!(g.state, ENDIVINGGAME_STATE_NOTPLAYING);
    // Offered again: 0x405C, phase 2.
    idle(&mut w, 1);
    let g = dg(&w, h);
    assert_eq!((g.actor.text_id, g.phase), (0x405C, ENDIVINGGAME_PHASE_2));
}

#[test]
fn after_the_scale_ten_rupees_win_again() {
    let Some(a) = assets() else { return };
    let mut w = domain(&a, |s| s.set_event_chk_inf(EVENTCHKINF_OBTAINED_SILVER_SCALE));
    let h = zora(&w);
    // 0x405C; "yes" pays and goes straight to the throw (no 0x4054): ten rupees, throwTimer 5
    // each, sinking 50 back a throwDistance from -300.
    let mut texts = Vec::new();
    talk_to_zora(&mut w);
    answer_until(&mut w, 3000, "the throw", &mut texts, 0, |w| dg(w, h).action == Action::ThrowRupees);
    assert_eq!(texts, vec![0x405C]);
    assert_eq!(w.save.rupees + w.save.rupee_accumulator, 30);
    assert_eq!(dg(&w, h).rupees_left_to_throw, 10);
    let mut last = 0;
    while !dg(&w, h).all_rupees_thrown {
        idle(&mut w, 1);
        if rupees(&w).len() > last {
            last = rupees(&w).len();
            assert_eq!(dg(&w, h).throw_timer, 5);
        }
    }
    assert_eq!(rupees(&w).len(), 10);
    answer_until(&mut w, 3000, "the game", &mut texts, 0, |w| dg(w, h).state == ENDIVINGGAME_STATE_PLAYING);
    until_all_wait(&mut w);
    let mut value = 0;
    for r in rupees(&w) {
        let e = ruppy(&w, r);
        // After the scale: from -300, ± 30 (and fallen straight down since).
        let z0 = -300.0 - 50.0 * e.throw_distance as f32;
        assert!((e.actor.world_pos.z - z0).abs() <= 30.0, "{} ({})", e.actor.world_pos.z, e.throw_distance);
        assert!([1, 5, 20, 50, 500].contains(&e.rupee_value));
        value += e.rupee_value;
    }
    let money = w.save.rupees + w.save.rupee_accumulator;
    for i in 0..9 {
        let r = rupees(&w)[0];
        take(&mut w, r);
        idle(&mut w, 1);
        assert_eq!(dg(&w, h).grabbed_rupees, i + 1);
        assert_eq!(dg(&w, h).state, ENDIVINGGAME_STATE_PLAYING, "nine aren't enough");
    }
    let r = rupees(&w)[0];
    take(&mut w, r);
    idle(&mut w, 1);
    // The tenth: 0x405D, extraWinCount 1, straight back to the talk's end (no prize).
    assert_eq!(w.save.rupees + w.save.rupee_accumulator, money + value);
    let g = dg(&w, h);
    assert_eq!((g.action, g.extra_win_count), (Action::TalkOutsideMinigame, 1));
    assert_eq!(w.msg_ctx.text_id, 0x405D);
    let mut texts = Vec::new();
    answer_until(&mut w, 3000, "the win", &mut texts, 0, |w| dg(w, h).action == Action::Idle);
    assert_eq!(texts, vec![0x405D]);
    assert_eq!(dg(&w, h).state, ENDIVINGGAME_STATE_NOTPLAYING);
    let_go(&w);
}

#[test]
fn the_time_running_out_loses() {
    let Some(a) = assets() else { return };
    let mut w = domain(&a, |_| {});
    let h = start_the_game(&mut w);
    until_all_wait(&mut w);
    // Link back on top; one rupee taken first.
    let r = rupees(&w)[0];
    take(&mut w, r);
    w.place_player(TALK_POS, TALK_YAW);
    // 50 seconds (20 frames each, after 20 shown and 20 sliding): the stop (TIMER_STATE_STOP, the
    // one frame Interface_Draw leaves it so) out of a cutscene: NA_SE_SY_FOUND, 0x71AD, Link held,
    // the game over (phase 0).
    let found = sfx_count(&w, NA_SE_SY_FOUND);
    let mut n = 0;
    while dg(&w, h).action != Action::TalkOutsideMinigame {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 1200, "time never ran out");
    }
    assert!(n > 50 * 20, "{n}");
    assert_eq!(w.save.timer_seconds, 0);
    assert_eq!(w.save.timer_state, TIMER_STATE_OFF);
    assert_eq!(sfx_count(&w, NA_SE_SY_FOUND), found + 1);
    assert_eq!(w.msg_ctx.text_id, 0x71AD);
    assert_eq!(w.player().cs_mode, PLAYER_CSACTION_8);
    let g = dg(&w, h);
    assert_eq!((g.state, g.phase, g.grabbed_rupees), (ENDIVINGGAME_STATE_NOTPLAYING, ENDIVINGGAME_PHASE_ENDED, 0));
    // The rupees left blink out (EnExRuppy_Kill, 20 frames) and are gone.
    let left = rupees(&w);
    assert_eq!(left.len(), 4);
    for &r in &left {
        assert_eq!(ruppy(&w, r).action, en_ex_ruppy::Action::Kill);
    }
    let shown: Vec<i16> = (0..4)
        .map(|_| {
            idle(&mut w, 1);
            ruppy(&w, left[0]).invisible
        })
        .collect();
    assert!(shown == vec![1, 0, 1, 0] || shown == vec![0, 1, 0, 1], "{shown:?}");
    idle(&mut w, 20);
    assert!(rupees(&w).is_empty());
    // 0x71AD read: Link let go, idle again.
    let mut texts = Vec::new();
    answer_until(&mut w, 3000, "the loss", &mut texts, 0, |w| dg(w, h).action == Action::Idle);
    assert_eq!(texts, vec![0x71AD]);
    let_go(&w);
}
