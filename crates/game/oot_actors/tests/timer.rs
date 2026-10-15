//! The HUD's timers (`oot_game::timer`, `z_parameter.c`'s `Interface_SetTimer` and
//! `Interface_Draw`'s timers), in the stable (no entrance cutscene). Expected values are worked
//! out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::*;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// The stable by day with `lost-woods-games`, settled (the transition over).
fn stable() -> Option<PlayState> {
    let a = assets()?;
    let e = a.scenes.entrance_index("ENTR_STABLE_0").unwrap();
    let mut s = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    s.apply_preset("lost-woods-games").unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), s).unwrap();
    for _ in 0..60 {
        idle(&mut w);
    }
    assert_eq!(w.transition.trigger, oot_game::transition::TRANS_TRIGGER_OFF);
    Some(w)
}

fn idle(w: &mut PlayState) {
    w.tick_with(scripted_input(PadState::default(), PadState::default()));
}

#[test]
fn a_countdown_shows_slides_and_ticks() {
    let Some(mut w) = stable() else { return };
    w.interface_set_timer(30);
    assert_eq!((w.save.timer_state, w.save.timer_seconds, w.save.timer_x[0], w.save.timer_y[0]), (TIMER_STATE_DOWN_INIT, 30, 140, 80));
    // Frame 1: TIMER_STATE_DOWN_INIT -> _PREVIEW, sTimerStateTimer and sTimerNextSecondTimer 20.
    idle(&mut w);
    assert_eq!(w.save.timer_state, TIMER_STATE_DOWN_PREVIEW);
    // Frames 2 to 21: the preview counts its 20 down, then _MOVE.
    for _ in 0..19 {
        idle(&mut w);
    }
    assert_eq!(w.save.timer_state, TIMER_STATE_DOWN_PREVIEW);
    idle(&mut w);
    assert_eq!(w.save.timer_state, TIMER_STATE_DOWN_MOVE);
    // Frame 22: the first slide step: (140 - 26) / 20 = 5, (80 - 46) / 20 = 1 (three hearts:
    // one row, y 46).
    idle(&mut w);
    assert_eq!((w.save.timer_x[0], w.save.timer_y[0]), (135, 79));
    // From _MOVE on the seconds count (timerState >= TIMER_STATE_ENV_HAZARD_MOVE): frame 41 both
    // the slide ends (_TICK at (26, 46)) and the first second goes.
    for _ in 0..18 {
        idle(&mut w);
    }
    assert_eq!((w.save.timer_state, w.save.timer_seconds), (TIMER_STATE_DOWN_MOVE, 30));
    idle(&mut w);
    assert_eq!((w.save.timer_state, w.save.timer_seconds, w.save.timer_x[0], w.save.timer_y[0]), (TIMER_STATE_DOWN_TICK, 29, 26, 46));
    // Drawn: the clock and five digits, "00:29" (sTimerDigits).
    assert_eq!(w.interface_ctx.timers.timer_digits, [0, 0, 10, 2, 9]);
    assert_eq!(w.interface_ctx.timers.drawn, Some(TIMER_ID_MAIN));
    // A second every 20 frames after.
    for _ in 0..20 {
        idle(&mut w);
    }
    assert_eq!(w.save.timer_seconds, 28);
    // A text open stops it (msgMode not MSGMODE_NONE).
    w.start_textbox(0x2045, None);
    for _ in 0..60 {
        idle(&mut w);
    }
    assert_eq!(w.save.timer_seconds, 28);
}

#[test]
fn run_out_it_stops_then_goes_off() {
    let Some(mut w) = stable() else { return };
    w.interface_set_timer(2);
    // 41 frames to the first second (as above), 20 more to 0: TIMER_STATE_STOP on frame 61,
    // then TIMER_STATE_OFF the next (`case TIMER_STATE_STOP`, no sub timer).
    for _ in 0..41 {
        idle(&mut w);
    }
    assert_eq!(w.save.timer_seconds, 1);
    for _ in 0..19 {
        idle(&mut w);
    }
    assert_eq!(w.save.timer_state, TIMER_STATE_DOWN_TICK);
    idle(&mut w);
    assert_eq!((w.save.timer_state, w.save.timer_seconds), (TIMER_STATE_STOP, 0));
    // Not drawn once stopped ("timerState != OFF && != STOP").
    assert_eq!(w.interface_ctx.timers.drawn, None);
    idle(&mut w);
    assert_eq!(w.save.timer_state, TIMER_STATE_OFF);
}

#[test]
fn counting_up_and_the_sub_timer() {
    let Some(mut w) = stable() else { return };
    // Interface_SetTimer(0): up from 0.
    w.interface_set_timer(0);
    assert_eq!(w.save.timer_state, TIMER_STATE_UP_INIT);
    for _ in 0..41 {
        idle(&mut w);
    }
    assert_eq!((w.save.timer_state, w.save.timer_seconds), (TIMER_STATE_UP_TICK, 1));
    // The sub timer waits while the main one runs (it's the main switch's default).
    w.interface_set_sub_timer(3);
    for _ in 0..30 {
        idle(&mut w);
    }
    assert_eq!(w.save.sub_timer_state, SUBTIMER_STATE_DOWN_INIT);
    // With the main one off, it goes as the main one does: drawn as the sub timer.
    w.save.timer_state = TIMER_STATE_OFF;
    for _ in 0..41 {
        idle(&mut w);
    }
    assert_eq!((w.save.sub_timer_state, w.save.sub_timer_seconds), (SUBTIMER_STATE_DOWN_TICK, 2));
    assert_eq!(w.interface_ctx.timers.drawn, Some(TIMER_ID_SUB));
    // At 0 (PAL: <= 0), outside Ganon's collapse: SUBTIMER_STATE_RESPAWN and text 0x71B0.
    for _ in 0..40 {
        idle(&mut w);
    }
    assert_eq!(w.save.sub_timer_state, SUBTIMER_STATE_RESPAWN);
    assert_eq!(w.msg_ctx.text_id, 0x71B0);
    // Interface_SetSubTimerToFinalSecond only touches a running sub timer.
    w.save.sub_timer_state = SUBTIMER_STATE_OFF;
    w.save.sub_timer_seconds = 50;
    w.interface_set_sub_timer_to_final_second();
    assert_eq!(w.save.sub_timer_seconds, 50);
}

#[test]
fn the_hazard_timer_stops_out_of_the_hazard() {
    let Some(mut w) = stable() else { return };
    // Interface_Update: with the timer the hazard's (TIMER_STATE_ENV_HAZARD_*) and Link on dry
    // land (sEnvHazard PLAYER_ENV_HAZARD_NONE), it goes off at once.
    w.save.timer_state = TIMER_STATE_ENV_HAZARD_INIT;
    idle(&mut w);
    assert_eq!(w.save.timer_state, TIMER_STATE_OFF);
    // A minigame's timer isn't the hazard's: it stays.
    w.interface_set_timer(30);
    idle(&mut w);
    assert_eq!(w.save.timer_state, TIMER_STATE_DOWN_PREVIEW);
    assert!(!w.interface_ctx.timers.env_hazard_active);
}
