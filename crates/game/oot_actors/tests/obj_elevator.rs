//! GAME-09 milestone 3 against the C: `Obj_Elevator` (`z_obj_elevator.c`), Dodongo's Cavern's small
//! stone lifts: room 1's 0x0001 at (2140, -159, -361) and 0x1201 at (2140, -159, -480), room 2's
//! 0x2200 at (-1957, -97, -1800) (Master Quest).
//!
//! Entered by `ENTR_DODONGOS_CAVERN_0` on a `cavern-open` save with the entrance wall broken, the
//! room loaded with `Room_RequestNewRoom`. Expected values are worked out from the C in the
//! comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::obj_elevator::*;
use oot_game::actor_ctx::ActorHandle;
use oot_game::env::clock_time;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// `SCENE_DODONGOS_CAVERN` (`scene_table.h`: 0x01).
const SCENE_DODONGOS_CAVERN: usize = 0x01;

/// Child Link in the cavern (`cavern-open`, the entrance wall and Navi's word about the pit done),
/// in room `room` (loaded from the lobby), three frames on, the audio logged.
fn cavern_room(a: &Arc<GameAssets>, room: i8, at: Vec3) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DODONGOS_CAVERN_0").expect("entrance");
    let mut save = SaveContext::new(e, false, clock_time(10, 0) as u16);
    save.apply_preset("cavern-open").unwrap();
    save.set_event_chk_inf(oot_actors::bg_dodoago::EVENTCHKINF_B0);
    save.scene_flags[SCENE_DODONGOS_CAVERN].swch |= (1 << 7) | (1 << 0x0C);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    w.audio.log = Some(Default::default());
    idle(&mut w, 2);
    assert!(w.room_request(room));
    idle(&mut w, 1);
    w.room_change_done();
    w.place_player(at, 0);
    idle(&mut w, 3);
    w
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn lift_at(w: &PlayState, home: Vec3) -> ActorHandle {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<ObjElevator>(h).is_some_and(|e| e.actor.home_pos.distance(home) < 1.0)).expect("the lift")
}

fn lift(w: &PlayState, h: ActorHandle) -> &ObjElevator {
    w.actors.downcast::<ObjElevator>(h).unwrap()
}

fn sfx_frames(w: &PlayState, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, _)| f).collect()
}

/// The lift's top: the floor over its home.
fn top(w: &PlayState, home: Vec3) -> f32 {
    w.col.entity_raycast_down(home + Vec3::new(0.0, 600.0, 0.0)).0
}

#[test]
fn room_1_s_0x1201_rises_80_at_4_when_stood_on_and_goes_back_when_stepped_on_again() {
    let Some(a) = assets() else { return };
    let home = Vec3::new(2140.0, -159.0, -480.0);
    let off = Vec3::new(2140.0, 0.0, -650.0);
    let mut w = cavern_room(&a, 1, off);
    let h = lift_at(&w, home);
    // ObjElevator_Init: size 1 (scale 0.05), unk_16C twice the speed 2, waiting; the culling
    // volume (2000, 600, 2000).
    let l = lift(&w, h);
    assert_eq!((l.actor.scale.x, l.unk_16c, l.action), (0.05, 4.0, Action::Wait));
    assert_eq!((l.actor.culling_volume_distance, l.actor.culling_volume_scale, l.actor.culling_volume_downward), (2000.0, 600.0, 2000.0));
    let t = top(&w, home);
    // Its top 200 above it (4000 in the model at 0.05).
    assert_eq!(t, home.y + 200.0);
    // Link steps on (DYNA_INTERACT_PLAYER_ON_TOP, not last frame's: unk_170): at home, to home +
    // 1 × 80, moving.
    w.place_player(Vec3::new(home.x, t, home.z), 0);
    let mut n = 0;
    while lift(&w, h).action != Action::Move {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 5, "not stepped on");
    }
    assert_eq!(lift(&w, h).unk_168, home.y + 80.0);
    // func_80B92D44: Math_SmoothStepToF (all of the gap, at most 4) with NA_SE_EV_STONE_STATUE_OPEN;
    // 80 in 20 frames, then the gap 0 (under 0.001): NA_SE_EV_FOOT_SWITCH, waiting. Link carried.
    let start = w.audio.frames;
    let mut n = 0;
    while lift(&w, h).action == Action::Move {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 40);
    }
    assert_eq!(n, 20);
    assert_eq!(lift(&w, h).actor.world_pos.y, home.y + 80.0);
    assert_eq!(sfx_frames(&w, NA_SE_EV_FOOT_SWITCH).iter().filter(|&&f| f > start).count(), 1);
    let p = w.player().actor.world_pos;
    assert!((p.y - (t + 80.0)).abs() < 1.0, "carried: {p}");
    // Standing on, nothing more (the flag was there last frame).
    idle(&mut w, 10);
    assert_eq!((lift(&w, h).action, lift(&w, h).actor.world_pos.y), (Action::Wait, home.y + 80.0));
    // Off, and on again: not at home, so back home.
    w.place_player(off, 0);
    idle(&mut w, 3);
    w.place_player(Vec3::new(home.x, t + 80.0, home.z), 0);
    let mut n = 0;
    while lift(&w, h).action != Action::Move {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 5, "not stepped on");
    }
    assert_eq!(lift(&w, h).unk_168, home.y);
    let mut n = 0;
    while lift(&w, h).action == Action::Move {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 40);
    }
    assert_eq!((n, lift(&w, h).actor.world_pos.y), (20, home.y));
}

#[test]
fn room_1_s_0x0001_never_moves() {
    let Some(a) = assets() else { return };
    let home = Vec3::new(2140.0, -159.0, -361.0);
    let mut w = cavern_room(&a, 1, Vec3::new(2140.0, 0.0, -650.0));
    let h = lift_at(&w, home);
    // Speed 0, rise 0: stepped on, its target is its home; the next frame the gap is 0: the
    // switch's sound, waiting.
    assert_eq!((lift(&w, h).actor.scale.x, lift(&w, h).unk_16c), (0.05, 0.0));
    let t = top(&w, home);
    w.place_player(Vec3::new(home.x, t, home.z), 0);
    let start = w.audio.frames;
    for _ in 0..10 {
        idle(&mut w, 1);
        assert_eq!(lift(&w, h).actor.world_pos.y, home.y);
    }
    assert_eq!(sfx_frames(&w, NA_SE_EV_FOOT_SWITCH).iter().filter(|&&f| f > start).count(), 1);
    assert_eq!(lift(&w, h).action, Action::Wait);
}

#[test]
fn room_2_s_0x2200_rises_160() {
    let Some(a) = assets() else { return };
    let home = Vec3::new(-1957.0, -97.0, -1800.0);
    let mut w = cavern_room(&a, 2, Vec3::new(-1957.0, 0.0, -1600.0));
    let h = lift_at(&w, home);
    // Size 0 (scale 0.1), speed 2 (4 a frame), rise 2 (160): 40 frames up.
    assert_eq!((lift(&w, h).actor.scale.x, lift(&w, h).unk_16c), (0.1, 4.0));
    let t = top(&w, home);
    w.place_player(Vec3::new(home.x, t, home.z), 0);
    let mut n = 0;
    while lift(&w, h).action != Action::Move {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 5, "not stepped on");
    }
    assert_eq!(lift(&w, h).unk_168, home.y + 160.0);
    let mut n = 0;
    while lift(&w, h).action == Action::Move {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 80);
    }
    assert_eq!((n, lift(&w, h).actor.world_pos.y), (40, home.y + 160.0));
}
