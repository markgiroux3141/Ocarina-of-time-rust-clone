//! Every room of the Deku Tree (MQ) has a debug start (`playthrough::DEKU_TREE_ROOM_STARTS`), and
//! the Deku Stick's, the push block's and the slingshot's have theirs (`playthrough::STICK_STARTS`,
//! `PUSH_STARTS`, `SLINGSHOT_STARTS`):
//! Link placed there stands, and the room doesn't change.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use oot_actors::PlayExt;
use oot_actors::playthrough::{DEKU_TREE_ROOM_STARTS, PUSH_STARTS, SLINGSHOT_STARTS, STICK_STARTS};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

#[test]
fn every_rooms_debug_start_stands_in_its_room() {
    let Some(a) = assets() else { return };
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    for &(room, pos, yaw, name) in DEKU_TREE_ROOM_STARTS.iter() {
        let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
        save.apply_preset("deku-tree-inside").unwrap();
        let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, oot_game::audio::GameAudio::default()).expect("Play_Init");
        idle(&mut w, 2);
        oot_actors::playthrough::deku_tree_room_start(&mut w, room, pos, yaw);
        // The enemies out of it (they'd move Link at Rand's times).
        for h in w.actors.category(oot_game::actor_ctx::ACTORCAT_ENEMY).to_vec() {
            if let Some(a) = w.actors.actor_mut(h) {
                a.kill();
            }
        }
        let (floor, _) = w.col.entity_raycast_down(pos + glam::Vec3::Y * 1.0);
        idle(&mut w, 40);
        let p = w.player();
        let at = p.actor.world_pos;
        println!("{name}: room {room} start {pos:?} floor {floor:.1} -> {at:?} {:?} grounded {} rooms {} {}", p.action, p.grounded(), w.room_ctx.cur.num, w.room_ctx.prev.num);
        assert_eq!((w.room_ctx.cur.num, w.room_ctx.prev.num), (room, -1), "{name}: the room changed");
        assert!(p.grounded(), "{name}: not on the ground at {at:?}");
        assert!((at - pos).length() < 1.0, "{name}: moved from {pos:?} to {at:?} ({:?})", p.action);
    }
}

#[test]
fn the_stick_starts_stand_in_their_rooms() {
    let Some(a) = assets() else { return };
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    for &(room, pos, yaw, name, switches) in STICK_STARTS.iter() {
        let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
        save.apply_preset("deku-tree-sticks").unwrap();
        let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, oot_game::audio::GameAudio::default()).expect("Play_Init");
        idle(&mut w, 2);
        // The game's --room, --switch, --at.
        if room != w.room_ctx.cur.num && w.room_request(room) {
            idle(&mut w, 1);
            w.room_change_done();
        }
        for &f in switches {
            w.flags.set_switch(f);
        }
        w.place_player(pos, yaw);
        for h in w.actors.category(oot_game::actor_ctx::ACTORCAT_ENEMY).to_vec() {
            if let Some(a) = w.actors.actor_mut(h) {
                a.kill();
            }
        }
        // The lift starts shaking and falls with Link on it: a few frames there.
        idle(&mut w, if name == "room2" { 3 } else { 40 });
        let p = w.player();
        let at = p.actor.world_pos;
        println!("{name}: room {room} start {pos:?} -> {at:?} {:?} grounded {} floor bg {:?} rooms {} {}", p.action, p.grounded(), p.actor.floor_bg_id, w.room_ctx.cur.num, w.room_ctx.prev.num);
        assert_eq!((w.room_ctx.cur.num, w.room_ctx.prev.num), (room, -1), "{name}: the room changed");
        assert!(p.grounded(), "{name}: not on the ground at {at:?}");
        assert!((at - pos).length() < 2.0, "{name}: moved from {pos:?} to {at:?} ({:?})", p.action);
        for &f in switches {
            assert!(w.flags.get_switch(f), "{name}: flag {f:#x}");
        }
    }
}

#[test]
fn the_push_starts_stand_in_their_rooms() {
    let Some(a) = assets() else { return };
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    for &(room, pos, yaw, name) in PUSH_STARTS.iter() {
        let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
        save.apply_preset("deku-tree-inside").unwrap();
        let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, oot_game::audio::GameAudio::default()).expect("Play_Init");
        idle(&mut w, 2);
        oot_actors::playthrough::deku_tree_room_start(&mut w, room, pos, yaw);
        for h in w.actors.category(oot_game::actor_ctx::ACTORCAT_ENEMY).to_vec() {
            if let Some(a) = w.actors.actor_mut(h) {
                a.kill();
            }
        }
        idle(&mut w, 40);
        let p = w.player();
        let at = p.actor.world_pos;
        println!("{name}: room {room} start {pos:?} -> {at:?} {:?} grounded {} rooms {} {}", p.action, p.grounded(), w.room_ctx.cur.num, w.room_ctx.prev.num);
        assert_eq!((w.room_ctx.cur.num, w.room_ctx.prev.num), (room, -1), "{name}: the room changed");
        assert!(p.grounded(), "{name}: not on the ground at {at:?}");
        assert!((at - pos).length() < 1.0, "{name}: moved from {pos:?} to {at:?} ({:?})", p.action);
    }
}

#[test]
fn the_slingshot_starts_stand_in_their_rooms() {
    let Some(a) = assets() else { return };
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    for &(room, pos, yaw, name) in SLINGSHOT_STARTS.iter() {
        let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
        save.apply_preset("deku-tree-slingshot").unwrap();
        let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, oot_game::audio::GameAudio::default()).expect("Play_Init");
        idle(&mut w, 2);
        oot_actors::playthrough::deku_tree_room_start(&mut w, room, pos, yaw);
        for h in w.actors.category(oot_game::actor_ctx::ACTORCAT_ENEMY).to_vec() {
            if let Some(a) = w.actors.actor_mut(h) {
                a.kill();
            }
        }
        idle(&mut w, 40);
        let p = w.player();
        let at = p.actor.world_pos;
        println!("{name}: room {room} start {pos:?} -> {at:?} {:?} grounded {} rooms {} {}", p.action, p.grounded(), w.room_ctx.cur.num, w.room_ctx.prev.num);
        assert_eq!((w.room_ctx.cur.num, w.room_ctx.prev.num), (room, -1), "{name}: the room changed");
        assert!(p.grounded(), "{name}: not on the ground at {at:?}");
        assert!((at - pos).length() < 1.0, "{name}: moved from {pos:?} to {at:?} ({:?})", p.action);
    }
}
