//! GAME-10 milestone 5 against the C: the room clear and its timer (`z_obj_roomtimer.c`), as
//! Master Quest's Jabu-Jabu places it (`ootx scene-info --scene bdan_scene`): room 8's 0x2FFF (no
//! time, flag 0x0B) at (-113, -340, -5669), with the red tentacle (`En_Ba` 0x0B00, the same flag).
//!
//! Child Link enters at `ENTR_JABU_JABU_0` on a `jabu-boomerang` save, and the room changes to room
//! 8 (`Room_RequestNewRoom`). The room's enemies are taken out directly (no fight); a timed one is
//! spawned with its params. Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::en_ba::EnBa;
use oot_actors::obj_roomtimer::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ACTORCAT_PROP, ActorHandle};
use oot_game::audio::sfx::{NA_SE_OC_ABYSS, NA_SE_SY_CORRECT_CHIME};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::{SaveContext, TIMER_STATE_DOWN_INIT, TIMER_STATE_STOP};
use oot_game::transition::TRANS_TRIGGER_START;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// Child Link in Jabu-Jabu (`jabu-boomerang`), the sound log on, in room 8, its actors in.
fn room_8(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_JABU_JABU_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("jabu-boomerang").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    assert!(w.room_request(8));
    idle(&mut w, 1);
    w.room_change_done();
    assert_eq!(w.room_ctx.cur.num, 8);
    idle(&mut w, 20);
    w
}

const HOME: Vec3 = Vec3::new(-113.0, -340.0, -5669.0);

fn timers(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<ObjRoomtimer>(h).is_some_and(|t| !t.actor.killed)).collect()
}

fn rt(w: &PlayState, h: ActorHandle) -> &ObjRoomtimer {
    w.actors.downcast::<ObjRoomtimer>(h).expect("Obj_Roomtimer")
}

fn gone(w: &PlayState, h: ActorHandle) -> bool {
    w.actors.downcast::<ObjRoomtimer>(h).is_none_or(|t| t.actor.killed)
}

/// The actor's update alone (`Actor_UpdateAll`'s call), as the updating actor.
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// The times a sound was asked for with no position (`Sfx_PlaySfxCentered`).
fn sfx_any(w: &PlayState, id: u16) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).count()
}

/// The tentacle taken out (killed: gone next frame).
fn kill_the_tentacle(w: &mut PlayState) {
    for h in w.actors.all() {
        if w.actors.downcast::<EnBa>(h).is_some() {
            w.actors.actor_mut(h).unwrap().kill();
        }
    }
}

#[test]
fn room_8s_timer_sets_the_red_flag_once_the_tentacle_is_gone() {
    let Some(a) = assets() else { return };
    let mut w = room_8(&a);
    let hs = timers(&w);
    assert_eq!(hs.len(), 1);
    let h = hs[0];
    // ObjRoomtimer_Init: switchFlag PARAMS_GET_U(0x2FFF, 10, 6) = 0x0B, params the time 0x3FF
    // (none); func_80B9D054 on its first update: no timer, Actor_ChangeCategory to a prop;
    // func_80B9D0B0.
    let t = rt(&w, h);
    assert_eq!((t.actor.home_pos, t.switch_flag, t.time(), t.action), (HOME, 0x0B, NO_TIME, Action::Wait));
    assert_eq!(t.actor.category, ACTORCAT_PROP);
    assert!(w.actors.category(ACTORCAT_PROP).contains(&h));
    assert!(!w.actors.category(ACTORCAT_ENEMY).contains(&h));
    let timer_state = w.save.timer_state;
    // The tentacle alive: the room's enemies aren't gone; nothing.
    idle(&mut w, 5);
    assert!(!gone(&w, h));
    assert!(!w.flags.get_temp_clear(8) && !w.flags.get_clear(8) && !w.flags.get_switch(0x0B));
    // The tentacle gone: the last enemy (Actor_RemoveFromCategory) sets the temporary clear flag;
    // func_80B9D0B0 then: the room's clear flag, switch flag 0x0B, NA_SE_SY_CORRECT_CHIME, gone;
    // the timer left alone (no time).
    let chimes = sfx_any(&w, NA_SE_SY_CORRECT_CHIME);
    kill_the_tentacle(&mut w);
    idle(&mut w, 2);
    assert!(w.flags.get_temp_clear(8));
    assert!(w.flags.get_clear(8));
    assert!(w.flags.get_switch(0x0B));
    assert_eq!(sfx_any(&w, NA_SE_SY_CORRECT_CHIME), chimes + 1);
    assert!(gone(&w, h));
    assert_eq!(w.save.timer_state, timer_state);
}

#[test]
fn a_timed_one_starts_the_hud_timer_and_voids_link_out_when_it_runs_out() {
    let Some(a) = assets() else { return };
    let mut w = room_8(&a);
    // Params (0x05 << 10) | 30: flag 5, 30 seconds. ObjRoomtimer_Init; its first update
    // (func_80B9D054): Interface_SetTimer(30): the main timer at (140, 80) counting down
    // (TIMER_STATE_DOWN_INIT), a prop.
    let h = w.actor_spawn(ACTOR_OBJ_ROOMTIMER, HOME, [0, 0, 0], (0x05 << 10) | 30).expect("Obj_Roomtimer");
    assert_eq!((rt(&w, h).switch_flag, rt(&w, h).time(), rt(&w, h).action), (5, 30, Action::Start));
    assert_eq!(rt(&w, h).actor.category, ACTORCAT_ENEMY);
    update(&mut w, h);
    assert_eq!((w.save.timer_seconds, w.save.timer_state), (30, TIMER_STATE_DOWN_INIT));
    assert_eq!((w.save.timer_x[0], w.save.timer_y[0]), (140, 80));
    assert_eq!((rt(&w, h).actor.category, rt(&w, h).action), (ACTORCAT_PROP, Action::Wait));
    // Time left, the tentacle alive: nothing.
    update(&mut w, h);
    assert!(!gone(&w, h));
    // Out of time (timerSeconds 0): NA_SE_OC_ABYSS, Play_TriggerVoidOut, gone.
    w.save.timer_seconds = 0;
    update(&mut w, h);
    assert!(gone(&w, h));
    assert_eq!(sfx_any(&w, NA_SE_OC_ABYSS), 1);
    assert_eq!((w.transition.trigger, w.save.respawn_flag), (TRANS_TRIGGER_START, 1));
    assert!(!w.flags.get_switch(5));
}

#[test]
fn a_timed_one_stops_the_timer_when_the_room_is_cleared_or_it_goes() {
    let Some(a) = assets() else { return };
    let mut w = room_8(&a);
    // ObjRoomtimer_Destroy with a time and seconds left: stopped.
    let h = w.actor_spawn(ACTOR_OBJ_ROOMTIMER, HOME, [0, 0, 0], (0x07 << 10) | 20).expect("Obj_Roomtimer");
    update(&mut w, h);
    assert_eq!((w.save.timer_seconds, w.save.timer_state), (20, TIMER_STATE_DOWN_INIT));
    w.delete_actor(h);
    assert_eq!(w.save.timer_state, TIMER_STATE_STOP);
    // A time over 600 is 600 (CLAMP_MAX).
    let h = w.actor_spawn(ACTOR_OBJ_ROOMTIMER, HOME, [0, 0, 0], (0x06 << 10) | 0x3FE).expect("Obj_Roomtimer");
    assert_eq!(rt(&w, h).time(), 600);
    update(&mut w, h);
    assert_eq!((w.save.timer_seconds, w.save.timer_state), (600, TIMER_STATE_DOWN_INIT));
    // Cleared (the temporary clear flag): the timer stopped (TIMER_STATE_STOP), the room's clear
    // flag (no enemy spawns in it after: Actor_Spawn), flag 6.
    w.flags.set_temp_clear(8);
    update(&mut w, h);
    assert!(gone(&w, h));
    assert_eq!(w.save.timer_state, TIMER_STATE_STOP);
    assert!(w.flags.get_switch(6) && w.flags.get_clear(8));
    assert!(w.actor_spawn(ACTOR_OBJ_ROOMTIMER, HOME, [0, 0, 0], (0x07 << 10) | 20).is_err());
}

#[test]
fn alone_in_a_room_it_clears_it_at_once() {
    let Some(a) = assets() else { return };
    let mut w = room_8(&a);
    // Room 8's timer and tentacle taken out before the timer sees the room clear.
    for h in timers(&w) {
        w.actors.actor_mut(h).unwrap().kill();
    }
    kill_the_tentacle(&mut w);
    idle(&mut w, 2);
    assert!(w.flags.get_temp_clear(8));
    assert!(!w.flags.get_clear(8));
    w.flags.unset_temp_clear(8);
    // A new one is the room's only enemy: its first update (Actor_ChangeCategory, whose
    // Actor_RemoveFromCategory sees the room's last enemy go) sets the temporary clear flag, and
    // the next clears the room.
    let h = w.actor_spawn(ACTOR_OBJ_ROOMTIMER, HOME, [0, 0, 0], (0x08 << 10) | NO_TIME).expect("Obj_Roomtimer");
    update(&mut w, h);
    assert!(w.flags.get_temp_clear(8));
    assert!(!w.flags.get_switch(8));
    update(&mut w, h);
    assert!(w.flags.get_clear(8) && w.flags.get_switch(8));
    assert!(gone(&w, h));
}
