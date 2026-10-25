//! `En_Horse_Game_Check` (GAME-12 milestone 2) against the C (`z_en_horse_game_check.c`): Ingo's
//! race on Lon Lon Ranch's cutscene layer (Ingo's horse, the start's steps, the checkpoints in
//! turn, the finish and where each result goes) and Malon's obstacle course (a fence out of turn).
//! No race is run: the horses are put where the C looks, and the referee is updated by hand.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use glam::Vec3;
use oot_actors::en_horse::{ENHORSE_INGO_WON, ENHORSE_JUMPING, EnHorse, HORSE_HNI};
use oot_actors::en_horse_game_check::*;
use oot_game::actor_ctx::ActorHandle;
use eng_input::pad::PadState;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Lon Lon Ranch's race (`ENTR_LON_LON_RANCH_0` with `CS_INDEX_0`, as `EnIn_StartTransition`
/// enters it) as an adult with `preset` (and `edit` on the save), at 10:00, two frames on: the
/// room's actors spawn on the first (the referee, and in its init Ingo's horse), the horses'
/// objects are in by the second.
fn race(a: &Arc<GameAssets>, preset: &str, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_LON_LON_RANCH_0").unwrap();
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset(preset).unwrap();
    save.cutscene_index = 0xFFF0;
    edit(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap();
    for _ in 0..2 {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
    w
}

/// The referee layer 4 places (params 1 at (600, 0, 375)).
fn referee(w: &PlayState) -> ActorHandle {
    let v: Vec<_> = w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnHorseGameCheck>(h).is_some()).collect();
    assert_eq!(v.len(), 1);
    v[0]
}

fn check(w: &PlayState) -> &EnHorseGameCheck {
    w.actors.downcast::<EnHorseGameCheck>(referee(w)).unwrap()
}

fn check_mut(w: &mut PlayState) -> &mut EnHorseGameCheck {
    let h = referee(w);
    w.actors.downcast_mut::<EnHorseGameCheck>(h).unwrap()
}

/// `EnHorseGameCheck_Update` alone, `n` times.
fn update(w: &mut PlayState, n: usize) {
    let h = referee(w);
    for _ in 0..n {
        let mut x = w.actors.take(h).unwrap();
        x.update(w);
        w.actors.put_back(h, x);
    }
}

/// Link's horse (Player's `rideActor`).
fn ride(w: &PlayState) -> ActorHandle {
    w.player.and_then(|h| w.actors.get(h)).and_then(|p| p.as_player()).and_then(|p| p.ride_actor()).expect("Link on horseback")
}

fn put(w: &mut PlayState, h: ActorHandle, pos: Vec3) {
    let a = w.actors.actor_mut(h).unwrap();
    a.world_pos = pos;
    a.prev_pos = pos;
}

#[test]
fn ingos_race_spawns_his_horse_and_starts_in_steps() {
    let Some(a) = assets() else { return };
    let mut w = race(&a, "adult-awake", |_| {});
    // EnHorseGameCheck_InitIngoRace: type 1; Ingo's horse is En_Horse 0x8003 at (-250, 1, -1650)
    // facing 0x4000 (EnHorse_Init takes bit 15 off as the type: HORSE_HNI, params 3).
    let g = check(&w);
    assert_eq!(g.ty, HORSEGAME_INGO_RACE);
    let ingo = g.ingo.ingo_horse.expect("Ingo's horse");
    let ih = w.actors.downcast::<EnHorse>(ingo).unwrap();
    assert_eq!((ih.ty, ih.actor.params, ih.actor.world_pos, ih.actor.shape_rot.y), (HORSE_HNI, 3, Vec3::new(-250.0, 1.0, -1650.0), 0x4000));
    assert!(!ih.actor.killed);
    // Link's horse (Horse_SetupInCutscene: HORSE_PTYPE_6 at (-250, 1, -1580)) is kept off the
    // checkpoints and the line; so is Ingo's.
    let link = ride(&w);
    put(&mut w, link, Vec3::new(500.0, 0.0, -1000.0));
    put(&mut w, ingo, Vec3::new(500.0, 0.0, -1100.0));
    // EnHorseGameCheck_UpdateIngoRace, startTimer from 0 and counted after the checks: the timer
    // (Interface_SetTimer(0), counting up) on the update that finds it at 51, Link's horse's
    // inRace at 81, Ingo's (with the start shot) at 82.
    let (mut timer, mut link_go, mut ingo_go) = (None, None, None);
    for _ in 0..90 {
        update(&mut w, 1);
        let g = check(&w);
        let seen = g.ingo.start_timer - 1;
        if timer.is_none() && g.ingo.start_flags & INGO_RACE_SET_TIMER != 0 {
            timer = Some(seen);
            assert_eq!((w.save.timer_state, w.save.timer_seconds), (oot_game::save::TIMER_STATE_UP_INIT, 0));
        }
        if link_go.is_none() && w.actors.downcast::<EnHorse>(link).unwrap().in_race {
            link_go = Some(seen);
        }
        if ingo_go.is_none() && w.actors.downcast::<EnHorse>(ingo).unwrap().in_race {
            ingo_go = Some(seen);
        }
    }
    assert_eq!((timer, link_go, ingo_go), (Some(51), Some(81), Some(82)));
    assert_eq!(check(&w).ingo.start_flags, INGO_RACE_SET_TIMER | INGO_RACE_PLAYER_MOVE | INGO_RACE_INGO_MOVE);
}

#[test]
fn ingos_race_finishes_after_the_checkpoints_in_turn() {
    let Some(a) = assets() else { return };
    let mut w = race(&a, "adult-awake", |_| {});
    let (link, ingo) = (ride(&w), check(&w).ingo.ingo_horse.unwrap());
    put(&mut w, ingo, Vec3::new(500.0, 0.0, -1100.0));
    // The checkpoints count only in turn (within 400): the second first counts nothing.
    let cps = INGO_RACE_CHECKPOINTS;
    put(&mut w, link, cps[1]);
    update(&mut w, 1);
    assert_eq!(check(&w).ingo.player_check, [0, 0, 0]);
    for (i, want) in [[1, 0, 0], [1, 1, 0], [1, 1, 1]].into_iter().enumerate() {
        put(&mut w, link, cps[i] + Vec3::new(0.0, 0.0, 300.0));
        update(&mut w, 1);
        assert_eq!(check(&w).ingo.player_check, want, "checkpoint {i}");
    }
    // At the finish line (x -200 to 80, z -2300 to -1470) after the third: Link wins, 55 frames to
    // go (the goal's music, the shot), the checkpoints cleared.
    put(&mut w, link, Vec3::new(-60.0, 0.0, -1900.0));
    update(&mut w, 1);
    let g = check(&w);
    assert_eq!((g.ingo.result, g.ingo.finish_timer, g.ingo.player_check), (INGO_RACE_PLAYER_WIN, 55, [0, 0, 0]));
    // 55 updates count it down; the next, EnHorseGameCheck_FinishIngoRace: the first race won,
    // ENTR_LON_LON_RANCH_7 (0x4CE) in TRANS_TYPE_CIRCLE(TCA_STARBURST, TCC_WHITE, TCS_FAST)
    // ((1 << 5) | (1 << 3) | (3 << 1) | 0 = 0x2E), INGO_RACE_STATE_FIRST_WIN (4) with
    // EVENTINF_INGO_RACE_0F (bit 15), Ingo's music forced (NA_BGM_INGO 0x42), the timer off.
    update(&mut w, 55);
    assert_ne!(w.transition.trigger, 20);
    update(&mut w, 1);
    assert_eq!((w.transition.trigger, w.transition.next_entrance_index, w.transition.ty), (20, 0x4CE, 0x2E));
    assert_eq!(w.save.event_inf[0] & 0x800F, 0x8004);
    assert_eq!((w.save.forced_seq_id, w.save.timer_state, w.save.cutscene_index), (0x42, 0, 0));
}

#[test]
fn ingo_first_at_the_line_and_the_race_for_epona() {
    let Some(a) = assets() else { return };
    // Ingo's horse past his three checkpoints and at the line: Ingo wins (ENHORSE_INGO_WON on his
    // horse), 70 frames to go; then ENTR_LON_LON_RANCH_8 (0x558) in TRANS_TYPE_CIRCLE(TCA_NORMAL,
    // TCC_BLACK, TCS_FAST) (0x20), INGO_RACE_STATE_PLAYER_LOSE (3).
    let mut w = race(&a, "adult-awake", |_| {});
    let (link, ingo) = (ride(&w), check(&w).ingo.ingo_horse.unwrap());
    put(&mut w, link, Vec3::new(500.0, 0.0, -1000.0));
    check_mut(&mut w).ingo.ingo_check = [1, 1, 1];
    put(&mut w, ingo, Vec3::new(0.0, 0.0, -2000.0));
    update(&mut w, 1);
    let g = check(&w);
    assert_eq!((g.ingo.result, g.ingo.finish_timer, g.ingo.ingo_check), (INGO_RACE_INGO_WIN, 70, [0, 0, 0]));
    assert_ne!(w.actors.downcast::<EnHorse>(ingo).unwrap().state_flags & ENHORSE_INGO_WON, 0);
    update(&mut w, 71);
    assert_eq!((w.transition.trigger, w.transition.next_entrance_index, w.transition.ty), (20, 0x558, 0x20));
    assert_eq!(w.save.event_inf[0] & 0x800F, 0x8003);

    // The race for Epona (EVENTINF_INGO_RACE_SECOND_RACE, bit 6) won: INGO_RACE_STATE_TRAPPED_WIN_EPONA
    // (6) in a white fade (TRANS_TYPE_FADE_WHITE 3), still to ENTR_LON_LON_RANCH_7.
    let mut w = race(&a, "adult-awake", |s| s.event_inf[0] |= 1 << 6);
    let (link, ingo) = (ride(&w), check(&w).ingo.ingo_horse.unwrap());
    put(&mut w, ingo, Vec3::new(500.0, 0.0, -1100.0));
    check_mut(&mut w).ingo.player_check = [1, 1, 1];
    put(&mut w, link, Vec3::new(-60.0, 0.0, -1900.0));
    update(&mut w, 57);
    assert_eq!((w.transition.trigger, w.transition.next_entrance_index, w.transition.ty), (20, 0x4CE, 3));
    assert_eq!(w.save.event_inf[0] & 0x800F, 0x8006);
}

#[test]
fn malons_course_fouls_a_fence_out_of_turn() {
    let Some(a) = assets() else { return };
    // With Epona had, the ranch's referee is Malon's obstacle course (params written 4).
    let mut w = race(&a, "epona", |_| {});
    let g = check(&w);
    assert_eq!((g.ty, g.actor.params), (HORSEGAME_MALON_RACE, 4));
    // Link's horse kept off the line and the fences: 51 updates, then the 52nd sets the timer
    // (MALONRACE_SET_TIMER: raceFlags 2).
    let link = ride(&w);
    put(&mut w, link, Vec3::new(500.0, 0.0, -1000.0));
    update(&mut w, 52);
    assert_eq!(check(&w).malon.race_flags, MALONRACE_SET_TIMER);
    // Jumping (ENHORSE_JUMPING) within 250 of the first fence: fenceCheck[0], no foul (the check of
    // "fenceCheck[-1]" reads raceFlags, 2).
    w.actors.downcast_mut::<EnHorse>(link).unwrap().state_flags |= ENHORSE_JUMPING;
    put(&mut w, link, FENCE_POS[0] + Vec3::new(100.0, 0.0, 0.0));
    update(&mut w, 1);
    let g = check(&w);
    assert_eq!((g.malon.fence_check[0], g.malon.result), (1, MALONRACE_NO_RESULT));
    // The third fence before the second: a foul (Malon's 0x208C), done 30 frames on.
    put(&mut w, link, FENCE_POS[2]);
    update(&mut w, 1);
    let g = check(&w);
    assert_eq!((g.malon.fence_check[2], g.malon.result, g.malon.finish_timer), (0, MALONRACE_FAILURE, 30));
    assert_ne!(g.malon.race_flags & MALONRACE_BROKE_RULE, 0);
    assert_eq!(w.msg_ctx.text_id, 0x208C);
    // 30 updates count down; the next, EnHorseGameCheck_FinishMalonRace: the timer frozen at 240,
    // ENTR_LON_LON_RANCH_7 in the white starburst (0x2E).
    update(&mut w, 31);
    assert_eq!((w.transition.trigger, w.transition.next_entrance_index, w.transition.ty), (20, 0x4CE, 0x2E));
    assert_eq!((w.save.timer_seconds, w.save.timer_state), (240, oot_game::save::TIMER_STATE_UP_FREEZE));
}
