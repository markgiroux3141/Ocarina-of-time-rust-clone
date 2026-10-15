//! GAME-08 milestone 3 against the C: Talon in Lon Lon Ranch's house by day (`z_en_ta.c`'s
//! `EnTa_SetTextForTalkInLonLonHouse` to `EnTa_IdleAfterCuccoGameFinished`): his three super cuccos
//! (`En_Niw` 0xD), the Super Cucco game and the milk. The placement (`ootx scene-info --scene
//! souko`): room 1, `En_Ta` 0xFFFF at (1054, 0, -29), yaw 0 (facing +z).
//!
//! Child Link enters the house (`ENTR_LON_LON_BUILDINGS_0`: spawn 0 at (981, 0, 184), room 1) with
//! `lost-woods-games` (`EVENTCHKINF_TALON_RETURNED_FROM_CASTLE`, 26 rupees) at 10:00, the sound log
//! and the audio side on. Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_niw::EnNiw;
use oot_actors::en_ta::*;
use oot_actors::player::{Player, STATE1_11};
use oot_game::actor::{ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED, ACTOR_FLAG_UPDATE_CULLING_DISABLED};
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::bgm::{seq_cmd1, seq_cmd_b, start_seq};
use oot_game::audio::sfx::NA_SE_SY_FOUND;
use oot_game::audio::{NA_BGM_SMALL_ITEM_GET, NA_BGM_TIMED_MINI_GAME, SEQ_PLAYER_BGM_MAIN};
use oot_game::camera::CAM_ID_MAIN;
use oot_game::item::{ITEM_BOTTLE_EMPTY, ITEM_BOTTLE_MILK_FULL, SLOT_BOTTLE_1};
use oot_game::message::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::*;
use oot_game::transition::{TCA_STARBURST, TCC_BLACK, TCC_WHITE, TCS_FAST, TRANS_TRIGGER_START, TRANS_TYPE_FADE_BLACK, TRANS_TYPE_FADE_WHITE, trans_type_circle};

/// Talon's placement.
const TALON_POS: Vec3 = Vec3::new(1054.0, 0.0, -29.0);
/// `EVENTCHKINF_TALON_RETURNED_FROM_CASTLE` (`save.h`).
const EVENTCHKINF_TALON_RETURNED_FROM_CASTLE: u16 = 0x14;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Link entered at `entrance` at `hour`:00 with `lost-woods-games` and `setup`, two frames on.
fn house(a: &Arc<GameAssets>, entrance: &str, hour: i32, setup: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(hour, 0) as u16);
    save.apply_preset("lost-woods-games").unwrap();
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

/// One frame with A, then one released.
fn press_a(w: &mut PlayState) {
    w.tick_with(scripted_input(PadState::default(), a_button()));
    w.tick_with(scripted_input(a_button(), PadState::default()));
}

/// The live Talons.
fn talons(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnTa>(h).is_some_and(|t| !t.actor.killed)).collect()
}

fn talon(w: &PlayState) -> ActorHandle {
    let t = talons(w);
    assert_eq!(t.len(), 1, "one Talon");
    t[0]
}

fn ta(w: &PlayState, h: ActorHandle) -> &EnTa {
    w.actors.downcast::<EnTa>(h).expect("En_Ta")
}

fn niw(w: &PlayState, h: ActorHandle) -> &EnNiw {
    w.actors.downcast::<EnNiw>(h).expect("En_Niw")
}

/// The live `En_Niw` 0xD.
fn super_cuccos(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnNiw>(h).is_some_and(|n| !n.actor.killed && n.actor.params == 0xD)).collect()
}

fn player(w: &PlayState) -> &Player {
    w.actors.downcast::<Player>(w.player.unwrap()).unwrap()
}

fn sfx_frames(w: &PlayState, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, _)| f).collect()
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

/// Link 60 in front of Talon, facing him, and A until a text opens (`EnTa_RequestTalk`: within
/// 100, Link within 0x4300 of his facing).
fn talk_to_talon(w: &mut PlayState) {
    w.place_player(TALON_POS + Vec3::new(0.0, 0.0, 69.0), -0x7E00);
    idle(w, 2);
    for _ in 0..10 {
        press_a(w);
        if w.message_state() != TEXT_STATE_NONE {
            return;
        }
    }
    panic!("Talon never talked");
}

/// Talon by day in the house, talked to, "yes" to his game: until his throw starts.
fn start_the_game(w: &mut PlayState, texts: &mut Vec<u16>) -> ActorHandle {
    let h = talon(w);
    talk_to_talon(w);
    answer_until(w, 3000, "the throw", texts, 0, |w| ta(w, h).action == Action::ThrowSuperCuccos);
    h
}

#[test]
fn by_day_talon_sits_with_three_super_cuccos() {
    let Some(a) = assets() else { return };
    let w = house(&a, "ENTR_LON_LON_BUILDINGS_0", 10, |_| {});
    let h = talon(&w);
    let t = ta(&w, h);
    // EnTa_Init's day branch: ACTOR_FLAG_UPDATE_CULLING_DISABLED, superCuccoTimers 7,
    // EnTa_IdleSittingInLonLonHouse with EnTa_AnimSitSleeping, the text 0x207E (first talk).
    assert_eq!(t.action, Action::IdleSittingInLonLonHouse);
    assert_eq!(t.anim_func, AnimFunc::SitSleeping);
    assert!(t.actor.flags & ACTOR_FLAG_UPDATE_CULLING_DISABLED != 0);
    assert_eq!(t.super_cucco_timers, [7; 3]);
    assert_eq!(t.actor.text_id, 0x207E);
    // The three En_Niw 0xD at (x + 5, y + 3, z + 26), (x - 20, y + 40, z - 30), (x + 20, y + 40,
    // z - 30), in superCuccos' order.
    let homes = [TALON_POS + Vec3::new(5.0, 3.0, 26.0), TALON_POS + Vec3::new(-20.0, 40.0, -30.0), TALON_POS + Vec3::new(20.0, 40.0, -30.0)];
    assert_eq!(super_cuccos(&w).len(), 3);
    for (i, home) in homes.iter().enumerate() {
        let c = t.super_cuccos[i].expect("spawned");
        assert_eq!(niw(&w, c).actor.params, 0xD);
        assert_eq!(niw(&w, c).actor.home_pos, *home);
        // En_Niw's init for 0xD: no gravity until thrown.
        assert_eq!(niw(&w, c).actor.gravity, 0.0);
        assert_eq!(niw(&w, c).unk_308, 0);
    }
}

#[test]
fn by_night_talon_sleeps_and_spawns_no_cuccos() {
    let Some(a) = assets() else { return };
    let w = house(&a, "ENTR_LON_LON_BUILDINGS_0", 22, |_| {});
    let t = ta(&w, talon(&w));
    // !IS_DAY: EnTa_IdleAsleepInLonLonHouse, gTalonSleepAnim, shadow 54; no super cuccos.
    assert_eq!(t.action, Action::IdleAsleepInLonLonHouse);
    assert_eq!(t.anim_func, AnimFunc::Sleeping);
    assert_eq!(t.shadow_scale, 54.0);
    assert_eq!(t.super_cuccos, [None; 3]);
    assert!(super_cuccos(&w).is_empty());
}

#[test]
fn before_his_return_talon_is_gone() {
    let Some(a) = assets() else { return };
    // !EVENTCHKINF_TALON_RETURNED_FROM_CASTLE: Actor_Kill before anything spawns.
    let w = house(&a, "ENTR_LON_LON_BUILDINGS_0", 10, |s| s.clear_event_chk_inf(EVENTCHKINF_TALON_RETURNED_FROM_CASTLE));
    assert!(talons(&w).is_empty());
    assert!(super_cuccos(&w).is_empty());
}

#[test]
fn first_0x207e_then_0x207f_and_no_puts_him_back_to_sleep() {
    let Some(a) = assets() else { return };
    let mut w = house(&a, "ENTR_LON_LON_BUILDINGS_0", 10, |_| {});
    let h = talon(&w);
    let rupees = w.save.rupees;
    talk_to_talon(&mut w);
    // EnTa_IdleSittingInLonLonHouse: NA_SE_VO_TA_SURPRISE, INFTABLE_TALKED_TO_TALON_IN_RANCH_HOUSE,
    // 0x207E: EnTa_WaitForPlayCuccoGameResponse with gTalonSitWakeUpAnim, eyes half open.
    assert_eq!(w.msg_ctx.text_id, 0x207E);
    assert!(w.save.get_inf_table(INFTABLE_TALKED_TO_TALON_IN_RANCH_HOUSE));
    assert_eq!(ta(&w, h).action, Action::WaitForPlayCuccoGameResponse);
    assert_eq!(ta(&w, h).anim_func, AnimFunc::RunToEnd);
    assert!(!sfx_frames(&w, NA_SE_VO_TA_SURPRISE).is_empty());
    // "No" (choice 1): EnTa_SetupActionWithSleepAnimation, the text again: 0x207F now.
    let mut texts = Vec::new();
    answer_until(&mut w, 3000, "no", &mut texts, 1, |w| ta(w, h).action == Action::IdleSittingInLonLonHouse);
    assert_eq!(texts[0], 0x207E);
    assert_eq!(ta(&w, h).anim_func, AnimFunc::SitSleeping);
    assert_eq!(ta(&w, h).eye_index, TALON_EYE_INDEX_CLOSED);
    assert_eq!(ta(&w, h).actor.text_id, 0x207F);
    assert_eq!(w.save.rupees + w.save.rupee_accumulator, rupees, "nothing paid");
    // Talked to again: 0x207F.
    idle(&mut w, 30);
    talk_to_talon(&mut w);
    assert_eq!(w.msg_ctx.text_id, 0x207F);
    assert_eq!(ta(&w, h).action, Action::WaitForPlayCuccoGameResponse);
}

#[test]
fn yes_with_under_10_rupees_is_0x85() {
    let Some(a) = assets() else { return };
    let mut w = house(&a, "ENTR_LON_LON_BUILDINGS_0", 10, |s| s.rupees = 9);
    let h = talon(&w);
    talk_to_talon(&mut w);
    // "Yes" with 9 < 10 rupees: 0x85, EnTa_TalkNotEnoughRupees; on its event the text closes and
    // he nods off again.
    let mut texts = Vec::new();
    answer_until(&mut w, 3000, "0x85", &mut texts, 0, |w| ta(w, h).action == Action::TalkNotEnoughRupees);
    assert_eq!(w.msg_ctx.text_id, 0x85);
    answer_until(&mut w, 3000, "back asleep", &mut texts, 0, |w| ta(w, h).action == Action::IdleSittingInLonLonHouse);
    assert_eq!(texts, vec![0x207E, 0x85]);
    assert_eq!(w.save.rupees + w.save.rupee_accumulator, 9);
    assert!(super_cuccos(&w).iter().all(|&c| niw(&w, c).unk_308 == 0), "no game");
}

/// Link next to super cucco `c` (wherever it landed), facing it, until it offers itself to his
/// lift (`Actor_OfferCarry`, `GI_NONE`).
fn offered(w: &mut PlayState, c: ActorHandle) {
    for _ in 0..300 {
        let p = niw(w, c).actor.world_pos;
        w.place_player(Vec3::new(p.x + 2.0, p.y, p.z - 20.0), -0x200);
        idle(w, 1);
        if player(w).interact_range_actor == Some(c) {
            return;
        }
    }
    panic!("the super cucco at {} never offered itself", niw(w, c).actor.world_pos);
}

/// Lifts super cucco `i`: the frames Link has carried it when Talon finds it (he holds the
/// talk at once, `EnTa_IdleFoundSuperCucco`).
fn lift_until_found(w: &mut PlayState, h: ActorHandle, i: usize, found: Action) -> usize {
    let c = ta(w, h).super_cuccos[i].unwrap();
    offered(w, c);
    w.tick_with(scripted_input(PadState::default(), a_button()));
    let mut held = 0;
    for n in 0..20 {
        let p = player(w);
        if p.state1 & STATE1_11 != 0 && p.interact_range_actor == Some(c) {
            held += 1;
        }
        if ta(w, h).action == found {
            return held;
        }
        // EnTa_RunCuccoGame: superCuccoTimers[i] down a frame while it's held.
        assert_eq!(ta(w, h).super_cucco_timers[i], 7 - held as i16, "held {held} frames ({n})");
        w.tick_with(scripted_input(if n == 0 { a_button() } else { PadState::default() }, PadState::default()));
    }
    panic!("Talon never found the super cucco");
}

#[test]
fn the_cucco_game_won() {
    let Some(a) = assets() else { return };
    let mut w = house(&a, "ENTR_LON_LON_BUILDINGS_0", 10, |_| {});
    let mut texts = Vec::new();
    let h = start_the_game(&mut w, &mut texts);
    // "Yes" (26 >= 10 rupees): Rupees_ChangeBy(-10), EnTa_StartCuccoGame: 0x2080; on its event
    // and 5 frames on (EnTa_StartingCuccoGame3) the throw: the timer at 30 (Interface_SetTimer:
    // TIMER_STATE_DOWN_INIT), the timed mini-game's music (func_800F5ACC), Link held
    // (PLAYER_CSACTION_1), the text closed, Talon's timer 50 (49 after his update).
    assert_eq!(texts, vec![0x207E, 0x2080]);
    assert_eq!(w.save.rupees + w.save.rupee_accumulator, 26 - 10);
    assert_eq!(w.save.timer_seconds, 30);
    assert_eq!(w.save.timer_state, TIMER_STATE_DOWN_INIT);
    assert!(seq_cmd_queued(&w, start_seq(SEQ_PLAYER_BGM_MAIN, 0, NA_BGM_TIMED_MINI_GAME)));
    assert!(ta(&w, h).state_flags & TALON_STATE_FLAG_RESTORE_BGM_ON_DESTROY != 0);
    assert_eq!(player(&w).cs_mode, 1);
    assert_eq!(ta(&w, h).timer, 49);
    let cuccos: Vec<ActorHandle> = ta(&w, h).super_cuccos.iter().map(|c| c.unwrap()).collect();

    // EnTa_ThrowSuperCuccos: above 35 (14 updates, 49 to 36) the cuccos on the table (1, 2) turn
    // towards i * -10000 - 3000 (Math_SmoothStepToS: half the way, 0x100 to 0x800 a frame); none
    // flies. Cucco 1 from 0: 5 steps of 0x800 to -10240, then 1380, 690, 345, 256 (-12911) and
    // the last 89 (under the 0x100 step: there), -13000 after 10. Cucco 2: 9 of 0x800 to
    // -18432, then 0x800 (-20480), 1260, 630, 315 and 256: -22941 after 14.
    let rot0 = niw(&w, cuccos[0]).actor.world_rot.y;
    while ta(&w, h).timer > 35 {
        idle(&mut w, 1);
        assert!(cuccos.iter().all(|&c| niw(&w, c).unk_308 == 0));
        // Link held all through the throw: the timer waits (Play_InCsMode).
        assert_eq!(w.save.timer_state, TIMER_STATE_DOWN_INIT);
    }
    assert_eq!(niw(&w, cuccos[0]).actor.world_rot.y, rot0, "the first cucco doesn't turn");
    assert_eq!(niw(&w, cuccos[1]).actor.world_rot.y, -13000);
    assert_eq!(niw(&w, cuccos[2]).actor.world_rot.y, -22941);
    assert_eq!(niw(&w, cuccos[2]).actor.shape_rot.y, -22941);
    // The update that reads 35: each cucco's unk_308 (it flies: En_Niw's func_80AB6324), gravity
    // 0, and 7 to 12 frames (Rand_CenteredFloat(6) + 10) without gravity.
    idle(&mut w, 1);
    assert_eq!(ta(&w, h).timer, 34);
    let timers = ta(&w, h).super_cucco_timers;
    for (i, &c) in cuccos.iter().enumerate() {
        assert_eq!(niw(&w, c).unk_308, 1);
        assert_eq!(niw(&w, c).actor.gravity, 0.0);
        assert!((7..=12).contains(&timers[i]), "{timers:?}");
    }
    // Then down 0.03 a frame for the timer's values below 35 - superCuccoTimers[i] (34 to 0): 35 -
    // superCuccoTimers[i] steps by the throw's end.
    while ta(&w, h).action == Action::ThrowSuperCuccos {
        idle(&mut w, 1);
    }
    assert_eq!(ta(&w, h).action, Action::RunCuccoGame);
    for (i, &c) in cuccos.iter().enumerate() {
        let mut g = 0.0f32;
        for _ in 0..(35 - timers[i]) {
            g -= 0.03;
        }
        assert_eq!(niw(&w, c).actor.gravity, g, "cucco {i}");
    }
    // Link let go (PLAYER_CSACTION_7); the timer runs: 20 frames in the middle, 20 sliding, a
    // second every 20 frames.
    idle(&mut w, 1);
    for &c in &cuccos {
        assert!(niw(&w, c).actor.gravity < -0.03 * 23.0, "EnTa_RunCuccoGame's gravity steps");
    }
    let mut n = 0;
    while w.save.timer_seconds == 30 {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 80, "the timer never ran");
    }
    assert_eq!(w.save.timer_seconds, 29);

    // Each super cucco lifted and held: on the 8th frame Link carries it (7 frames counted down)
    // Talon finds it and talks at once: 0x2082 (two left), 0x2083 (one left).
    for (i, (text, sfx)) in [(0x2082u16, NA_SE_VO_TA_SURPRISE), (0x2083, NA_SE_VO_TA_CRY_1)].into_iter().enumerate() {
        let sfx_before = sfx_frames(&w, sfx).len();
        let held = lift_until_found(&mut w, h, i, Action::IdleFoundSuperCucco);
        assert_eq!(held, 8);
        assert_eq!(ta(&w, h).last_found_super_cucco_idx, i as u8);
        assert_eq!(ta(&w, h).actor.text_id, text);
        assert!(ta(&w, h).actor.flags & ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED != 0);
        assert!(sfx_frames(&w, sfx).len() > sfx_before);
        let mut t = Vec::new();
        answer_until(&mut w, 3000, "found", &mut t, 0, |w| ta(w, h).action == Action::RunCuccoGame);
        assert_eq!(t, vec![text]);
        // EnTa_TalkFoundSuperCucco: the cucco goes up out of Link's hands and is forgotten.
        assert_eq!(ta(&w, h).super_cuccos[i], None);
        let p = player(&w);
        assert_eq!(p.held_actor, None);
        assert_eq!(p.interact_range_actor, None);
        assert!(p.state1 & STATE1_11 == 0);
        let c = &niw(&w, cuccos[i]).actor;
        assert_eq!(c.gravity, 0.1);
        assert_eq!(c.parent, None);
        assert!(ta(&w, h).actor.flags & ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED == 0);
        // (The text's close seen by Player before Link is moved on.)
        idle(&mut w, 10);
    }
    // The last: 0x2084 at once (Message_StartTextbox), the game won: the timer off, the music
    // stopped, the fanfare (NA_BGM_SMALL_ITEM_GET), EVENTINF_CUCCO_GAME_WON, Link held.
    let held = lift_until_found(&mut w, h, 2, Action::TalkCuccoGameEnd);
    assert_eq!(held, 8);
    assert_eq!(w.msg_ctx.text_id, 0x2084);
    assert_eq!(w.save.timer_state, TIMER_STATE_OFF);
    assert!(w.save.get_event_inf(EVENTINF_CUCCO_GAME_WON));
    assert!(seq_cmd_queued(&w, seq_cmd1(SEQ_PLAYER_BGM_MAIN, 0)));
    assert_eq!(w.audio.fanfare_seq_id, NA_BGM_SMALL_ITEM_GET);
    assert!(ta(&w, h).state_flags & TALON_STATE_FLAG_RESTORE_BGM_ON_DESTROY == 0);
    assert_eq!(player(&w).cs_mode, 1);
    // On 0x2084's event (EnTa_TalkCuccoGameEnd): ENTR_LON_LON_BUILDINGS_2 through a white
    // starburst, EVENTINF_CUCCO_GAME_FINISHED.
    let mut t = Vec::new();
    answer_until(&mut w, 3000, "the game's end", &mut t, 0, |w| ta(w, h).action == Action::TransitionToPostCuccoGame);
    assert_eq!(t, vec![0x2084]);
    assert_eq!(w.transition.next_entrance_index, ENTR_LON_LON_BUILDINGS_2);
    assert_eq!(w.transition.ty, trans_type_circle(TCA_STARBURST, TCC_WHITE, TCS_FAST));
    assert_eq!(w.save.next_transition_type, TRANS_TYPE_FADE_WHITE);
    assert!(w.save.get_event_inf(EVENTINF_CUCCO_GAME_FINISHED));
    assert_eq!(a.scenes.entrance_index("ENTR_LON_LON_BUILDINGS_2"), Some(ENTR_LON_LON_BUILDINGS_2));

    // Back in the house by Talon (spawn 2): EnTa_IdleAfterCuccoGameFinished, 0x2086 at once (the
    // first win), the flags cleared by his init.
    let mut n = 0;
    while !talons(&w).first().is_some_and(|&h2| h2 != h || ta(&w, h2).action == Action::IdleAfterCuccoGameFinished) {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 200, "never back in the house");
    }
    let h = talon(&w);
    assert_eq!(w.save.entrance_index, ENTR_LON_LON_BUILDINGS_2);
    assert!(player(&w).actor.world_pos.distance(Vec3::new(1050.0, 0.0, 12.0)) < 30.0, "Link at spawn 2: {}", player(&w).actor.world_pos);
    assert_eq!(ta(&w, h).action, Action::IdleAfterCuccoGameFinished);
    assert_eq!(ta(&w, h).actor.text_id, 0x2086);
    assert!(!w.save.get_event_inf(EVENTINF_CUCCO_GAME_FINISHED));
    assert!(!w.save.get_event_inf(EVENTINF_CUCCO_GAME_WON));
    first_win_bottle(&mut w, h);
}

/// After the first win (0x2086 offered at once): the "marry Malon" choice, 0x2087, the bottle of
/// milk (`GI_BOTTLE_MILK_FULL`), `ITEMGETINF_TALON_BOTTLE`.
fn first_win_bottle(w: &mut PlayState, h: ActorHandle) {
    assert!(!w.save.get_item_get_inf(ITEMGETINF_TALON_BOTTLE));
    let bottles = |w: &PlayState| (0..4).filter(|&i| w.save.inventory.items[SLOT_BOTTLE_1 + i] == ITEM_BOTTLE_MILK_FULL).count();
    let before = bottles(w);
    let mut texts = Vec::new();
    answer_until(w, 3000, "the choice", &mut texts, 1, |w| ta(w, h).action == Action::TalkAfterCuccoGameFirstWon);
    assert_eq!(w.msg_ctx.text_id, 0x2087);
    answer_until(w, 3000, "the bottle", &mut texts, 0, |w| ta(w, h).action == Action::IdleSittingInLonLonHouse);
    assert_eq!(&texts[..2], &[0x2086, 0x2087]);
    assert!(w.save.get_item_get_inf(ITEMGETINF_TALON_BOTTLE));
    assert_eq!(bottles(w), before + 1, "a bottle of Lon Lon Milk");
    // Nodding off again: talked to, 0x208B (milk or the game).
    assert_eq!(ta(w, h).actor.text_id, 0x208B);
}

#[test]
fn after_the_first_win_the_bottle_of_milk() {
    let Some(a) = assets() else { return };
    // Entered by the game's end (ENTR_LON_LON_BUILDINGS_2) with EVENTINF_CUCCO_GAME_FINISHED and
    // _WON: EnTa_SetTextForTalkInLonLonHouse 0x2086 (no ITEMGETINF_TALON_BOTTLE), the win cleared;
    // EnTa_IdleAfterCuccoGameFinished at gTalonSitWakeUpAnim's end, the game's flag cleared.
    // (Talked to before the game: INFTABLE_TALKED_TO_TALON_IN_RANCH_HOUSE.)
    let mut w = house(&a, "ENTR_LON_LON_BUILDINGS_2", 10, |s| {
        s.set_event_inf(EVENTINF_CUCCO_GAME_FINISHED);
        s.set_event_inf(EVENTINF_CUCCO_GAME_WON);
        s.set_inf_table(INFTABLE_TALKED_TO_TALON_IN_RANCH_HOUSE);
    });
    let h = talon(&w);
    assert_eq!(ta(&w, h).actor.text_id, 0x2086);
    assert!(!w.save.get_event_inf(EVENTINF_CUCCO_GAME_FINISHED));
    assert!(!w.save.get_event_inf(EVENTINF_CUCCO_GAME_WON));
    first_win_bottle(&mut w, h);
}

#[test]
fn a_later_win_fills_an_empty_bottle_and_milk_is_30_rupees() {
    let Some(a) = assets() else { return };
    // A later win (ITEMGETINF_TALON_BOTTLE): 0x2088, then GI_MILK into the empty bottle.
    let mut w = house(&a, "ENTR_LON_LON_BUILDINGS_2", 10, |s| {
        s.set_event_inf(EVENTINF_CUCCO_GAME_FINISHED);
        s.set_event_inf(EVENTINF_CUCCO_GAME_WON);
        s.set_item_get_inf(ITEMGETINF_TALON_BOTTLE);
        s.set_inf_table(INFTABLE_TALKED_TO_TALON_IN_RANCH_HOUSE);
        s.inventory.items[SLOT_BOTTLE_1] = ITEM_BOTTLE_EMPTY;
        s.rupees = 40;
    });
    let h = talon(&w);
    assert_eq!(ta(&w, h).actor.text_id, 0x2088);
    let mut texts = Vec::new();
    answer_until(&mut w, 3000, "the milk", &mut texts, 0, |w| ta(w, h).action == Action::IdleSittingInLonLonHouse);
    assert_eq!(texts[0], 0x2088);
    assert_eq!(w.save.inventory.items[SLOT_BOTTLE_1], ITEM_BOTTLE_MILK_FULL);
    assert_eq!(w.save.rupees + w.save.rupee_accumulator, 40, "the prize is free");
    // Talked to again: 0x208B; buy milk (choice 0) with no empty bottle: 0x208A.
    idle(&mut w, 30);
    talk_to_talon(&mut w);
    assert_eq!(w.msg_ctx.text_id, 0x208B);
    let mut texts = Vec::new();
    answer_until(&mut w, 3000, "no bottle", &mut texts, 0, |w| ta(w, h).action == Action::IdleSittingInLonLonHouse);
    assert_eq!(&texts[..2], &[0x208B, 0x208A]);
    // With an empty bottle: 30 rupees (Rupees_ChangeBy(-30)) and GI_MILK.
    w.save.inventory.items[SLOT_BOTTLE_1 + 1] = ITEM_BOTTLE_EMPTY;
    idle(&mut w, 30);
    talk_to_talon(&mut w);
    let mut texts = Vec::new();
    answer_until(&mut w, 3000, "bought", &mut texts, 0, |w| ta(w, h).action == Action::GiveItemInLonLonHouse);
    assert_eq!(w.save.rupees + w.save.rupee_accumulator, 10);
    answer_until(&mut w, 3000, "the milk given", &mut texts, 0, |w| ta(w, h).action == Action::IdleSittingInLonLonHouse);
    assert_eq!(w.save.inventory.items[SLOT_BOTTLE_1 + 1], ITEM_BOTTLE_MILK_FULL);
    // Short of 30 rupees: 0x85.
    w.save.inventory.items[SLOT_BOTTLE_1 + 1] = ITEM_BOTTLE_EMPTY;
    idle(&mut w, 60);
    talk_to_talon(&mut w);
    let mut texts = Vec::new();
    answer_until(&mut w, 3000, "short", &mut texts, 0, |w| ta(w, h).action == Action::IdleSittingInLonLonHouse);
    assert_eq!(&texts[..2], &[0x208B, 0x85]);
}

#[test]
fn after_a_loss_the_retry_is_5_rupees() {
    let Some(a) = assets() else { return };
    // EVENTINF_CUCCO_GAME_FINISHED without _WON: 0x2085, the retry for 5 rupees
    // (EnTa_WaitForPlayCuccoGameResponse's price by the text): 0x2080 and the game.
    let mut w = house(&a, "ENTR_LON_LON_BUILDINGS_2", 10, |s| s.set_event_inf(EVENTINF_CUCCO_GAME_FINISHED));
    let h = talon(&w);
    assert_eq!(ta(&w, h).actor.text_id, 0x2085);
    assert_eq!(ta(&w, h).action, Action::IdleAfterCuccoGameFinished);
    let mut texts = Vec::new();
    answer_until(&mut w, 3000, "the retry", &mut texts, 0, |w| ta(w, h).action == Action::ThrowSuperCuccos);
    assert_eq!(texts, vec![0x2085, 0x2080]);
    assert_eq!(w.save.rupees + w.save.rupee_accumulator, 26 - 5);
}

#[test]
fn time_running_out_is_0x2081_with_the_floor_camera() {
    let Some(a) = assets() else { return };
    let mut w = house(&a, "ENTR_LON_LON_BUILDINGS_0", 10, |_| {});
    let mut texts = Vec::new();
    let h = start_the_game(&mut w, &mut texts);
    // Nothing found: at 10 seconds left the fast tempo (Audio_SetFastTempoForTimedMinigame:
    // SEQCMD_SET_SEQPLAYER_TEMPO(.., 5, 0, 210) while the timed mini-game plays), at 0 (not in a
    // cutscene) NA_SE_SY_FOUND, 0x2081, the floor camera, the win cleared.
    let fast = seq_cmd_b(SEQ_PLAYER_BGM_MAIN, 5, 0, 0xD2);
    let mut n = 0;
    let mut fast_at = None;
    while ta(&w, h).action != Action::TalkCuccoGameEnd {
        let seconds = w.save.timer_seconds;
        idle(&mut w, 1);
        if fast_at.is_none() && seq_cmd_queued(&w, fast) {
            fast_at = Some(seconds);
        }
        n += 1;
        assert!(n < 1000, "time never ran out");
    }
    assert_eq!(fast_at, Some(10), "the fast tempo first at 10 seconds");
    assert_eq!(w.save.timer_seconds, 0);
    assert_eq!(w.save.timer_state, TIMER_STATE_OFF);
    assert_eq!(w.msg_ctx.text_id, 0x2081);
    assert!(!sfx_frames(&w, NA_SE_SY_FOUND).is_empty());
    assert!(!w.save.get_event_inf(EVENTINF_CUCCO_GAME_WON));
    let t = ta(&w, h);
    assert!(t.state_flags & TALON_STATE_FLAG_FLOOR_CAMERA_ACTIVE != 0);
    assert_eq!(t.return_to_cam_id, CAM_ID_MAIN);
    assert_eq!(w.active_cam_id, t.sub_cam_id);
    let cam = w.camera(t.sub_cam_id).expect("the floor camera");
    assert_eq!((cam.at, cam.eye), (FLOOR_CAMERA_AT, FLOOR_CAMERA_EYE));
    assert_eq!(player(&w).cs_mode, 1);
    // On 0x2081's event: a black starburst to ENTR_LON_LON_BUILDINGS_2, the game finished; 22
    // frames on the main camera back.
    let mut t = Vec::new();
    answer_until(&mut w, 3000, "the game's end", &mut t, 0, |w| ta(w, h).action == Action::TransitionToPostCuccoGame);
    assert_eq!(t, vec![0x2081]);
    assert_eq!(w.transition.next_entrance_index, ENTR_LON_LON_BUILDINGS_2);
    assert_eq!(w.transition.ty, trans_type_circle(TCA_STARBURST, TCC_BLACK, TCS_FAST));
    assert_eq!(w.save.next_transition_type, TRANS_TYPE_FADE_BLACK);
    assert_eq!(w.transition.trigger, TRANS_TRIGGER_START);
    assert!(w.save.get_event_inf(EVENTINF_CUCCO_GAME_FINISHED));
    assert_eq!(ta(&w, h).timer, 21);
    let sub = ta(&w, h).sub_cam_id;
    // His updates read 21 to 1, then 0: the camera goes on the 22nd.
    for _ in 0..21 {
        idle(&mut w, 1);
        assert_eq!(w.active_cam_id, sub);
    }
    idle(&mut w, 1);
    assert!(ta(&w, h).state_flags & TALON_STATE_FLAG_FLOOR_CAMERA_ACTIVE == 0);
    assert_eq!(w.active_cam_id, CAM_ID_MAIN);
    assert!(w.camera(sub).is_none());
}
