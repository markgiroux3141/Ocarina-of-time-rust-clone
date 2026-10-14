//! GAME-07 milestone 4 against the C: the guards' courtyard (`z_en_heishi1.c` and its search ball
//! `z_eff_ss_solder_srch_ball.c`, `Camera_Special6`, the courtyard's rupees `z_en_ex_ruppy.c`),
//! Zelda (`z_en_zl4.c`), Impa (`z_demo_im.c`'s types 3 to 5), and the exit's runs (`Route::Zelda`,
//! `Route::ZeldaTalk`).
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::demo_im::{self, DemoIm};
use oot_actors::en_ex_ruppy::EnExRuppy;
use oot_actors::en_heishi1::{self, EnHeishi1};
use oot_actors::en_zl4::{self, EnZl4};
use oot_actors::player::Player;
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::effect::EFFECT_SS_SOLDER_SRCH_BALL;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

/// `SCENE_HYRULE_CASTLE`, `SCENE_CASTLE_COURTYARD_GUARDS_DAY`, `_NIGHT`, `SCENE_CASTLE_COURTYARD_ZELDA`,
/// `SCENE_HYRULE_FIELD` (`scene_table.h`).
const SCENE_HYRULE_CASTLE: u16 = 0x5F;
const SCENE_CASTLE_COURTYARD_GUARDS_DAY: u16 = 0x45;
const SCENE_CASTLE_COURTYARD_GUARDS_NIGHT: u16 = 0x46;
const SCENE_CASTLE_COURTYARD_ZELDA: u16 = 0x4A;
const SCENE_HYRULE_FIELD: u16 = 0x51;
/// `CAM_SET_PREREND_SIDE_SCROLL`, `CAM_SET_FREE0`.
const CAM_SET_PREREND_SIDE_SCROLL: i16 = 0x1B;
const CAM_SET_FREE0: i16 = 0x21;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Child Link entered at `entrance` at `hour`:00 with `preset`, `edit` applied to the save last;
/// two frames on.
fn enter_with(a: &Arc<GameAssets>, entrance: &str, hour: i32, preset: &str, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(hour, 0) as u16);
    save.apply_preset(preset).unwrap();
    edit(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    idle(&mut w, 2);
    w
}

fn courtyard(a: &Arc<GameAssets>, hour: i32) -> PlayState {
    enter_with(a, "ENTR_CASTLE_COURTYARD_GUARDS_DAY_0", hour, "castle-courtyard", |_| {})
}

/// Zelda's courtyard, four frames on (its room's objects come in a frame late: Zelda's and
/// Impa's first updates are on the third).
fn zeldas(a: &Arc<GameAssets>, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    let mut w = enter_with(a, "ENTR_CASTLE_COURTYARD_ZELDA_0", 10, "castle-courtyard", edit);
    idle(&mut w, 2);
    w
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// One frame with A, then one released.
fn press_a(w: &mut PlayState) {
    let a = PadState { button: BTN_A, ..Default::default() };
    w.tick_with(scripted_input(PadState::default(), a));
    w.tick_with(scripted_input(a, PadState::default()));
}

fn all<T: 'static + ActorImpl>(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<T>(h).is_some_and(|a| !a.base().killed)).collect()
}

fn one<T: 'static + ActorImpl>(w: &PlayState) -> ActorHandle {
    let v = all::<T>(w);
    assert_eq!(v.len(), 1, "one {}", std::any::type_name::<T>());
    v[0]
}

fn guard(w: &PlayState, path: i16) -> ActorHandle {
    let v: Vec<_> = all::<EnHeishi1>(w).into_iter().filter(|&h| w.actors.downcast::<EnHeishi1>(h).unwrap().path == path).collect();
    assert_eq!(v.len(), 1, "one guard on path {path}");
    v[0]
}

fn base(w: &PlayState, h: ActorHandle) -> &Actor {
    w.actors.actor(h).unwrap()
}

fn player(w: &PlayState) -> &Player {
    w.actors.downcast::<Player>(w.player.unwrap()).unwrap()
}

fn balls(w: &PlayState) -> Vec<&oot_game::effect::EffectSs> {
    w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == EFFECT_SS_SOLDER_SRCH_BALL).collect()
}

// The guards' courtyard.

#[test]
fn the_courtyard_has_its_patrols_by_day_and_its_drain_guards_by_night() {
    let Some(a) = assets() else { return };
    // EnHeishi1_Init by day (10:00 < CLOCK_TIME(17, 18) - 1): the eight walkers (types 0 to 3, paths
    // 0 to 7: params 0x0000, 0x0001, 0x0102, 0x0303, 0x0204, 0x0305, 0x0206, 0x0307) set up to walk;
    // the two type 5s by the drain (0x05FF) killed.
    let w = courtyard(&a, 10);
    assert_eq!(w.scene_id, SCENE_CASTLE_COURTYARD_GUARDS_DAY);
    let mut types: Vec<(i16, i16)> = all::<EnHeishi1>(&w).into_iter().map(|h| w.actors.downcast::<EnHeishi1>(h).unwrap()).map(|g| (g.path, g.ty)).collect();
    types.sort();
    assert_eq!(types, vec![(0, 0), (1, 0), (2, 1), (3, 3), (4, 2), (5, 3), (6, 2), (7, 3)]);
    // Two frames in, each is in EnHeishi1_Walk: its anim params by type (sAnimParamsInit), headed for
    // waypoint 0.
    for h in all::<EnHeishi1>(&w) {
        let g = w.actors.downcast::<EnHeishi1>(h).unwrap();
        assert!(matches!(g.action, en_heishi1::Action::SetupWalk | en_heishi1::Action::Walk | en_heishi1::Action::SetupWait | en_heishi1::Action::Wait), "path {}: {:?}", g.path, g.action);
        let expected_speed = if g.ty == 1 || g.ty == 3 { 3.0 } else { 1.0 };
        assert_eq!(g.anim_params[0], expected_speed, "path {}", g.path);
    }
    // The path 3 guard spawned the courtyard's eight rupees (En_Ex_Ruppy type 3) at sRupeePositions:
    // the world positions themselves.
    let mut rupees: Vec<(i32, i32)> = all::<EnExRuppy>(&w).into_iter().map(|h| base(&w, h).home_pos).map(|p| (p.x as i32, p.z as i32)).collect();
    rupees.sort();
    assert_eq!(rupees, vec![(-55, -30), (-55, 30), (-55, 90), (0, -30), (0, 90), (55, -30), (55, 30), (55, 90)]);
    for h in all::<EnExRuppy>(&w) {
        let r = w.actors.downcast::<EnExRuppy>(h).unwrap();
        // ENEXRUPPY_TYPE_HYRULE_COURTYARD: scale 0.02, gravity -3, green (or blue 3 in 31, red 1 in 31).
        assert_eq!((r.ty, r.actor.scale.x, r.actor.gravity), (3, 0.02, -3.0));
        assert!(r.color_idx <= 2);
        assert_eq!(r.action, oot_actors::en_ex_ruppy::Action::WaitAsCollectible);
    }
    // By night (22:00: the night scene by the entrance table's layer 1): the two by the drain stand
    // (EnHeishi1_SetupWaitNight).
    let w = courtyard(&a, 22);
    assert_eq!(w.scene_id, SCENE_CASTLE_COURTYARD_GUARDS_NIGHT);
    let night = all::<EnHeishi1>(&w);
    assert_eq!(night.len(), 2);
    for h in night {
        let g = w.actors.downcast::<EnHeishi1>(h).unwrap();
        // (Its first update is a frame late: the room's objects come in late.)
        assert!(g.ty == 5 && matches!(g.action, en_heishi1::Action::SetupWaitNight | en_heishi1::Action::WaitNight), "{:?}", g.action);
    }
}

#[test]
fn a_guard_looks_only_while_the_camera_shows_his_screen() {
    let Some(a) = assets() else { return };
    // EnHeishi1_Update: the search ball (EffectSsSolderSrchBall_Spawn) only while sBgCamIndices for his
    // path has the active camera's bgCamIndex. At the drain's spawn the camera is on bg camera 0
    // (CAM_SET_START1): nobody looks.
    let mut w = courtyard(&a, 10);
    for h in all::<EnHeishi1>(&w) {
        w.actors.actor_mut(h).unwrap().flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
    }
    idle(&mut w, 30);
    assert!(balls(&w).is_empty());
    // On screen 7's floor (south of path 0's round): the camera goes to bg camera 7, and path 0's
    // guard (sBgCamIndices 7, 7) throws a ball each frame from 60 up at 30 a frame along his look.
    w.place_player(Vec3::new(1700.0, 0.0, 255.0), -0x4000);
    idle(&mut w, 20);
    assert_eq!(w.active_camera().bg_cam_index, 7);
    let g = guard(&w, 0);
    let gy = base(&w, g).world_pos.y;
    let b = balls(&w);
    assert!(!b.is_empty());
    for e in &b {
        assert!((e.velocity.length() - 30.0).abs() < 1e-3, "{:?}", e.velocity);
        assert_eq!(e.velocity.y, 0.0);
        assert_eq!(e.pos.y, gy + 60.0);
        assert_eq!(e.actor, Some(g));
    }
    // Only path 0's: path 1 and 2 look on bg camera 2.
    assert!(b.iter().all(|e| e.actor == Some(g)));
}

#[test]
fn a_search_ball_lives_100_frames_unless_it_touches_a_poly() {
    let Some(a) = assets() else { return };
    // EffectSsSolderSrchBall_Init: life 100; its update ends it the next frame (life 1) once a sphere
    // of 30 at it touches a poly. One thrown north at the courtyard's north wall (z -240) from 60
    // up at (1300, 100), clear of path 0's box and the wall at x 1260, touches it within 30.
    let mut w = courtyard(&a, 10);
    for h in all::<EnHeishi1>(&w) {
        w.actors.actor_mut(h).unwrap().kill();
    }
    idle(&mut w, 2);
    w.with_ss(|s| s.solder_srch_ball_spawn(Vec3::new(1300.0, 60.0, 100.0), Vec3::new(0.0, 0.0, -30.0), Vec3::ZERO, 2, None));
    let i = w.effect_ss.table.iter().position(|e| e.life > -1 && e.ty == EFFECT_SS_SOLDER_SRCH_BALL).unwrap();
    assert_eq!(w.effect_ss.table[i].life, 100);
    let mut life_one_at = None;
    for f in 1..=20 {
        idle(&mut w, 1);
        let e = &w.effect_ss.table[i];
        if life_one_at.is_none() && e.life == 1 {
            life_one_at = Some((f, e.pos.z));
        }
    }
    let (f, z) = life_one_at.expect("the ball touched the wall");
    assert!(z - 30.0 <= -240.0 + 1.0, "life 1 at z {z}, frame {f}");
    assert!(w.effect_ss.table[i].life == -1, "gone after its last frame");
}

#[test]
fn seen_up_close_hey_you_and_kicked_out_by_the_crates() {
    let Some(a) = assets() else { return };
    // On path 0's screen, 50 from his guard (EnHeishi1_Update: xzDistToPlayer < 60 sets linkDetected;
    // Link standing, velocity.y not above -3.9; within 60 of his height): NA_SE_SY_FOUND, Link held
    // (PLAYER_CSACTION_1), sPlayerIsCaught, EnHeishi1_SetupMoveToLink: "Hey you!" (0x702D).
    let mut w = courtyard(&a, 10);
    for h in all::<EnHeishi1>(&w) {
        w.actors.actor_mut(h).unwrap().flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
    }
    w.place_player(Vec3::new(1700.0, 0.0, 255.0), -0x4000);
    idle(&mut w, 20);
    let g = guard(&w, 0);
    let gp = base(&w, g).world_pos;
    w.place_player(Vec3::new(gp.x + 50.0, 0.0, gp.z), -0x4000);
    // (The move restarts Camera_Special6's 12 frames of freeze: the guards don't update meanwhile.)
    let mut caught = false;
    for _ in 0..30 {
        idle(&mut w, 1);
        if w.msg_ctx.text_id == 0x702D {
            caught = true;
            break;
        }
    }
    if !caught {
        let gg = w.actors.downcast::<EnHeishi1>(g).unwrap();
        panic!("the guard never saw Link: guard {:?} {:?} det {} xz {} , link {:?} vy {} freeze {}, cam {}", gg.actor.world_pos, gg.action, gg.link_detected, gg.actor.xz_dist_to_player, base(&w, w.player.unwrap()).world_pos, base(&w, w.player.unwrap()).velocity.y, base(&w, w.player.unwrap()).freeze_timer, w.active_camera().bg_cam_index);
    }
    assert_eq!(player(&w).cs_mode, 1);
    assert!(w.overlay_static::<en_heishi1::Statics>(en_heishi1::ACTOR_EN_HEISHI1).player_is_caught);
    // A through the text: on its event, EnHeishi1_Kick: EVENTCHKINF_CAUGHT_BY_CASTLE_GUARDS, out to
    // ENTR_HYRULE_CASTLE_3 (by Talon's crates, 0x4FA) with the white starburst, sPlayerIsCaught false.
    let mut out = false;
    for _ in 0..600 {
        if w.transition.next_entrance_index == en_heishi1::ENTR_HYRULE_CASTLE_3 && w.transition.trigger != oot_game::transition::TRANS_TRIGGER_OFF {
            out = true;
            break;
        }
        if w.message_state() != oot_game::message::TEXT_STATE_NONE {
            press_a(&mut w);
        } else {
            idle(&mut w, 1);
        }
    }
    assert!(out, "never kicked out");
    assert_eq!(w.transition.ty, oot_actors::en_heishi3::TRANS_TYPE_CAUGHT);
    assert!(w.save.get_event_chk_inf(oot_actors::en_heishi3::EVENTCHKINF_CAUGHT_BY_CASTLE_GUARDS));
    let changes = w.scene_changes;
    for _ in 0..200 {
        if w.scene_changes > changes {
            break;
        }
        idle(&mut w, 1);
    }
    assert_eq!(w.scene_id, SCENE_HYRULE_CASTLE);
    assert_eq!(w.save.entrance_index, en_heishi1::ENTR_HYRULE_CASTLE_3);
    // Spawn 3's params 0x0700: Player_StartMode_KnockedOver, func_80837C0C's large knockback from
    // behind (rot.y + 0x8000, so arg5 is 0x8000 from his facing): link_normal_front_downA, thrown
    // along world.rot.y 0 at 2 while facing the spawn's -0x8000 (shape.rot.y turned back by 0x8000).
    let p = player(&w);
    assert_eq!(p.action, oot_actors::player::Action::KnockedDown);
    assert_eq!(w.data.anim_name(p.skel.animation), "link_normal_front_downA");
    assert_eq!((p.actor.world_rot.y, p.actor.shape_rot.y), (0, -0x8000));
    // He lands, gets up (Player_Action_8084377C, Player_Action_80843954) and stands (Player_Action_Idle).
    let mut up = false;
    for _ in 0..120 {
        idle(&mut w, 1);
        if player(&w).action == oot_actors::player::Action::StandingStill {
            up = true;
            break;
        }
    }
    assert!(up, "still {:?}", player(&w).action);
}

#[test]
fn the_courtyards_camera_slides_along_each_screen() {
    let Some(a) = assets() else { return };
    // Camera_Special6 (CAM_SET_PREREND_SIDE_SCROLL): a new bg camera freezes Player 12 frames
    // (actor.freezeTimer) and eases the eye over 12 frames; then the eye is on the bg camera's plane,
    // halfway along x to Player (eyeNext.x + (player.x - eyeNext.x) * 0.5), its y + (player.y -
    // initialPlayerY) * 0.2, at 100 along its rotation.
    let mut w = courtyard(&a, 10);
    w.place_player(Vec3::new(1700.0, 0.0, 255.0), -0x4000);
    let mut froze = None;
    for f in 1..=40 {
        idle(&mut w, 1);
        let ft = base(&w, w.player.unwrap()).freeze_timer;
        if froze.is_none() && ft > 0 {
            froze = Some((f, ft));
        }
    }
    let (_, ft) = froze.expect("Player frozen at the screen change");
    assert!(ft <= 12);
    let c = w.active_camera();
    assert_eq!(c.setting, CAM_SET_PREREND_SIDE_SCROLL);
    assert_eq!(c.bg_cam_index, 7);
    // bg camera 7: pos (2028, 270, 440), rot (4733, -25668, 0).
    assert_eq!(c.eye_next, Vec3::new(2028.0, 270.0, 440.0));
    let p = base(&w, w.player.unwrap()).world_pos;
    assert_eq!(c.eye.x, 2028.0 + (p.x - 2028.0) * 0.5);
    assert_eq!(c.eye.z, 440.0);
    // The HUD all back once settled (CAM_HUD_VISIBILITY_ALL: the 0x0F00 bits clear).
    assert_eq!(c.interface_flags as u16 & 0x0F00, 0);
}

#[test]
fn a_courtyard_rupee_is_taken_within_30() {
    let Some(a) = assets() else { return };
    // EnExRuppy_WaitAsCollectible: Link within 30 (xyzDistToPlayerSq < 900): NA_SE_SY_GET_RUPY, an
    // En_Item00 of its colour dropped to be taken at once (0x8000), the rupee gone.
    let mut w = courtyard(&a, 10);
    for h in all::<EnHeishi1>(&w) {
        w.actors.actor_mut(h).unwrap().kill();
    }
    idle(&mut w, 20);
    let h = all::<EnExRuppy>(&w)[0];
    let color = w.actors.downcast::<EnExRuppy>(h).unwrap().color_idx;
    let value = [1, 5, 20][color as usize];
    let pos = base(&w, h).world_pos;
    let before = w.save.rupees;
    w.place_player(pos + Vec3::new(10.0, 0.0, 0.0), 0);
    idle(&mut w, 30);
    assert!(w.actors.downcast::<EnExRuppy>(h).is_none_or(|r| r.actor.killed));
    assert_eq!(w.save.rupees, before + value);
}

// Zelda.

#[test]
fn zelda_waits_at_the_window_until_link_is_in_front_of_her() {
    let Some(a) = assets() else { return };
    // EnZl4_Init on the courtyard's layer 0 without the letter, entered by ENTR_CASTLE_COURTYARD_ZELDA_0:
    // ZL4_ANIM_21 (looking in the window), EnZl4_Cutscene in ZL4_CS_WAIT, text -1.
    let mut w = zeldas(&a, |_| {});
    assert_eq!(w.scene_id, SCENE_CASTLE_COURTYARD_ZELDA);
    let z = one::<EnZl4>(&w);
    let zl = w.actors.downcast::<EnZl4>(z).unwrap();
    assert_eq!((zl.action, zl.cs_state), (en_zl4::Action::Cutscene, 0));
    assert!(zl.skel.is("gChildZeldaAnim_019600"));
    assert_eq!(zl.actor.text_id, 0xFFFF);
    let zp = zl.actor.world_pos;
    // EnZl4_CsWaitForPlayer: on her floor (y equal) and in front (|yaw to Link - her yaw| < 0x3FFC),
    // she offers the talk within cylinder radius + 60 = 70. Behind her: none.
    w.place_player(zp + Vec3::new(-60.0, 0.0, 0.0), 0x4000);
    idle(&mut w, 3);
    assert_ne!(player(&w).target_actor, Some(z));
    w.place_player(zp + Vec3::new(60.0, 0.0, 0.0), -0x4000);
    idle(&mut w, 3);
    assert_eq!(player(&w).target_actor, Some(z));
    // A: Player_StartTalking with her text -1 holds Link (PLAYER_CSACTION_1); taken, Link 56 in front
    // of her; ZL4_CS_START the next frame: ZL4_ANIM_3, NA_BGM_APPEAR, the camera's fixed view 0 on
    // CAM_SET_FREE0 (at (-490, 120, 0), eye (-440, 117, 0), fov 45).
    press_a(&mut w);
    idle(&mut w, 2);
    let zl = w.actors.downcast::<EnZl4>(z).unwrap();
    assert!(zl.cs_state >= 2, "cs state {}", zl.cs_state);
    let p = base(&w, w.player.unwrap()).world_pos;
    assert_eq!((p.x, p.z), (zp.x + 56.0, zp.z));
    let c = w.active_camera();
    assert_eq!(c.setting, CAM_SET_FREE0);
    assert_eq!((c.at, c.eye, c.fov), (Vec3::new(-490.0, 120.0, 0.0), Vec3::new(-440.0, 117.0, 0.0), 45.0));
}

#[test]
fn after_the_letter_zelda_only_talks() {
    let Some(a) = assets() else { return };
    // EnZl4_Init with EVENTCHKINF_OBTAINED_ZELDAS_LETTER: ZL4_ANIM_0, EnZl4_Idle; her text by stones
    // (EnZl4_GetTextId): one (the Kokiri Emerald) 0x703C.
    let mut w = zeldas(&a, |s| s.set_event_chk_inf(en_zl4::EVENTCHKINF_OBTAINED_ZELDAS_LETTER));
    let z = one::<EnZl4>(&w);
    assert_eq!(w.actors.downcast::<EnZl4>(z).unwrap().action, en_zl4::Action::Idle);
    let zp = base(&w, z).world_pos;
    w.place_player(zp + Vec3::new(50.0, 0.0, 0.0), -0x4000);
    idle(&mut w, 3);
    assert_eq!(base(&w, z).text_id, 0x703C);
}

/// `Route::ZeldaTalk` run with these choices (yes past their end), to the letter.
fn zelda_talk(choices: Vec<u8>) -> (Playthrough, PlayState) {
    let a = assets().unwrap();
    let route = Route::ZeldaTalk;
    let e = a.scenes.entrance_index(route.entrance()).unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), route.save(e)).unwrap();
    route.debug_start(&mut w);
    w.audio_side = Some(Box::new(oot_game::audio::offline::OfflineAudio::new(&a.pack.audio_data().unwrap(), false)));
    let mut run = Playthrough::for_route(route).with_choices(choices);
    let mut prev = PadState::default();
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
    }
    assert!(run.failure.is_none(), "{:?}", run.failure);
    (run, w)
}

#[test]
fn zeldas_questions_come_again_on_no() {
    if assets().is_none() {
        return;
    }
    // No at the stone (0x7030: 0x7031, then 0x7030 again), at the names (0x7033: 0x7034, then 0x7033
    // again), at the legend's question (0x2077: 0x7038, 0x7037, 0x2076, 0x2077 again), at the window
    // (0x2078: 0x700B first) and at Ganondorf (0x703A: 0x7073, then 0x703A again).
    let (run, _) = zelda_talk(vec![1, 0, 1, 0, 1, 0, 1, 1, 0]);
    let t = &run.texts;
    let has = |seq: &[u16]| t.windows(seq.len()).any(|w| w == seq);
    assert!(has(&[0x7030, 0x7031, 0x7030, 0x7032]), "{t:x?}");
    assert!(has(&[0x7033, 0x7034, 0x7033, 0x7035]), "{t:x?}");
    assert!(has(&[0x2077, 0x7038, 0x7037, 0x2076, 0x2077, 0x7005]), "{t:x?}");
    assert!(has(&[0x2078, 0x700B]), "{t:x?}");
    assert!(has(&[0x703A, 0x7073, 0x703A, 0x703B]), "{t:x?}");
}

// Impa.

#[test]
fn impa_waits_for_the_letter_then_her_lullaby_starts_within_130() {
    let Some(a) = assets() else { return };
    // Demo_Im type 5 (func_809869B0): action 18, then (no EVENTCHKINF_59) 23: hidden until the letter.
    let w = zeldas(&a, |_| {});
    let h = one::<DemoIm>(&w);
    let im = w.actors.downcast::<DemoIm>(h).unwrap();
    assert_eq!((im.actor.params, im.action, im.draw_config), (5, 23, 0));
    // With the letter (func_80986BF8): action 24, drawn, her turns on.
    let mut w = zeldas(&a, |s| s.set_event_chk_inf(en_zl4::EVENTCHKINF_OBTAINED_ZELDAS_LETTER));
    let h = one::<DemoIm>(&w);
    let im = w.actors.downcast::<DemoIm>(h).unwrap();
    assert_eq!((im.action, im.draw_config, im.unk_280), (24, 1, 1));
    let ip = im.actor.world_pos;
    // func_80986A5C: Link west of her by more than 130: nothing.
    w.place_player(Vec3::new(ip.x - 140.0, ip.y, ip.z), 0x4000);
    idle(&mut w, 3);
    assert_eq!(w.actors.downcast::<DemoIm>(h).unwrap().action, 24);
    assert!(!w.save.check_quest_item(oot_game::item::QUEST_SONG_LULLABY));
    // Within 130 (func_80986C30): gZeldasCourtyardLullabyCs, EVENTCHKINF_59, Zelda's Lullaby given at
    // once, her action 10 and hidden (the script's cue 2 shows her).
    w.place_player(Vec3::new(ip.x - 120.0, ip.y, ip.z), 0x4000);
    idle(&mut w, 2);
    assert!(w.save.get_event_chk_inf(demo_im::EVENTCHKINF_59));
    assert!(w.save.check_quest_item(oot_game::item::QUEST_SONG_LULLABY));
    assert_ne!(w.cs_ctx.state, oot_game::cutscene::CS_STATE_IDLE);
    let im = w.actors.downcast::<DemoIm>(h).unwrap();
    assert!(matches!(im.action, 10 | 11), "action {}", im.action);
}

#[test]
fn impa_throws_her_nut_in_the_field_and_is_gone() {
    let Some(a) = assets() else { return };
    // Hyrule Field's layer 12 (CS_INDEX_8, 0xFFF8: gHyruleFieldImpaCs): Demo_Im type 4 (func_8098652C,
    // action 15). Cue 2 shows her (16); cue 10 the throw (17): when gImpaThrowDekuNutAnim ends,
    // func_809865F8 spawns En_Arrow ARROW_CS_NUT 30 in front of her (rot x 0xFA0); cue 11 kills her.
    let mut w = enter_with(&a, "ENTR_HYRULE_FIELD_0", 10, "castle-courtyard", |s| s.cutscene_index = 0xFFF8);
    assert_eq!(w.save.scene_layer, 12);
    idle(&mut w, 2);
    let h = one::<DemoIm>(&w);
    assert_eq!(w.actors.downcast::<DemoIm>(h).unwrap().actor.params, 4);
    let mut nut = None;
    let mut actions = Vec::new();
    for _ in 0..1500 {
        if let Some(im) = w.actors.downcast::<DemoIm>(h) {
            if actions.last() != Some(&im.action) {
                actions.push(im.action);
            }
        }
        if nut.is_none() {
            // (EnArrow_Init turns ARROW_CS_NUT into ARROW_NUT with isCsNut.)
            nut = w.actors.all().into_iter().find(|&n| w.actors.downcast::<oot_actors::en_arrow::EnArrow>(n).is_some_and(|ar| ar.is_cs_nut)).map(|n| (n, w.cs_ctx.frames));
        }
        if w.message_state() != oot_game::message::TEXT_STATE_NONE {
            press_a(&mut w);
        } else {
            idle(&mut w, 1);
        }
        if w.actors.downcast::<DemoIm>(h).is_none_or(|im| im.actor.killed) {
            break;
        }
    }
    // gImpaThrowDekuNutAnim's last frame is 29: from cue 10's frame 421 the throw ends on 452, and the
    // nut is out on the next.
    let (_, at) = nut.expect("no Deku Nut thrown");
    assert_eq!(a.animation(demo_im::OBJECT, "gImpaThrowDekuNutAnim").unwrap().last_frame(), 29.0);
    assert!((452..=454).contains(&at), "the nut at script frame {at}");
    assert!(actions.starts_with(&[15, 16, 17]), "{actions:?}");
    assert!(w.actors.downcast::<DemoIm>(h).is_none_or(|im| im.actor.killed), "Impa still there");
}

// The exit's runs.

#[test]
fn exit_past_the_guards_to_zelda_her_letter_impas_lullaby_and_goodbye() {
    let Some(a) = assets() else { return };
    let route = Route::Zelda;
    let e = a.scenes.entrance_index(route.entrance()).unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), route.save(e)).unwrap();
    route.debug_start(&mut w);
    w.audio_side = Some(Box::new(oot_game::audio::offline::OfflineAudio::new(&a.pack.audio_data().unwrap(), false)));
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
    }
    assert!(run.failure.is_none(), "{:?}", run.failure);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    use Step::*;
    assert_eq!(steps, vec![ZeldaCourtyard, ZeldasLetter, LetterRead, LullabyCs, LullabyPlayed, ImpaGone]);
    // Zelda's texts in the C's order, yes at each question: the meeting, the stone, the Triforce, the
    // names, the legend (0x7036, the cutscene map's), her questions, the window and Ganondorf (0x7009,
    // the script's), Ganondorf, the plan, the letter (0x69); Impa's (0x708D, 0x7090), the lesson
    // (0x878), the playback (0x86D, the wrong note's 0x88B, 0x86D), "You played" (0x86F) and the song
    // (0x89B), 0xD4, Impa's 0x708E; Hyrule Field's goodbye (0x2060, 0x204F, 0x2062, 0x2063).
    assert_eq!(
        run.texts,
        vec![
            0x702E, 0x702F, 0x70F9, 0x70FA, 0x70FB, 0x7030, 0x7032, 0x70FC, 0x70FD, 0x70FE, 0x70FF, 0x2073, 0x2074, 0x2075, 0x7033, 0x7035, 0x7036, 0x7037, 0x2076, 0x2077, 0x7005, 0x2078, 0x7009, 0x7039, 0x2079, 0x207A, 0x207B, 0x703A, 0x703B, 0x7123, 0x207C, 0x207D, 0x69, 0x708D, 0x7090, 0x878, 0x86D, 0x88B, 0x86D, 0x86F, 0x89B, 0xD4, 0x708E, 0x2060, 0x204F, 0x2062, 0x2063
        ]
    );
    // The letter (Item_Give(ITEM_ZELDAS_LETTER) in the child's trade slot), the lullaby, EVENTCHKINF_59;
    // Link in Hyrule Field at ENTR_HYRULE_FIELD_17 (0x594).
    assert!(w.save.get_event_chk_inf(en_zl4::EVENTCHKINF_OBTAINED_ZELDAS_LETTER));
    assert_eq!(w.save.inv_content(oot_game::item::ITEM_ZELDAS_LETTER), oot_game::item::ITEM_ZELDAS_LETTER);
    assert!(w.save.check_quest_item(oot_game::item::QUEST_SONG_LULLABY));
    assert!(w.save.get_event_chk_inf(demo_im::EVENTCHKINF_59));
    assert_eq!(w.scene_id, SCENE_HYRULE_FIELD);
    assert_eq!(w.save.entrance_index, 0x594);
    // Impa threw her Deku Nut (cue 10: En_Arrow ARROW_CS_NUT) and was gone at cue 11.
    assert!(all::<DemoIm>(&w).is_empty());
}
