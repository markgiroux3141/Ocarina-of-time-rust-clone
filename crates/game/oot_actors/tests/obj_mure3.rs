//! `Obj_Mure3` (GAME-13 milestone 2) against the C (`z_obj_mure3.c`): Kokiri Forest's ring of
//! rupees (adult layer 2, 0x40FF at (1304, 240, -548): type 2, switch flag 0x3F) spawned within
//! 1,150 of the camera, a rupee taken gone for good, the rest taken back past 1,450.
//!
//! Its `projectedPos` (`Actor_DrawAll`'s, the camera's view of it) is set by hand and its update
//! run alone (`Actor_UpdateAll`'s call). Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use glam::Vec3;
use oot_actors::en_item00::{ACTOR_EN_ITEM00, ITEM00_RUPEE_GREEN, ITEM00_RUPEE_RED};
use oot_actors::obj_mure3::*;
use oot_game::actor_ctx::ActorHandle;
use oot_game::play::PlayState;
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

const RING: Vec3 = Vec3::new(1304.0, 240.0, -548.0);

/// Kokiri Forest's spawn 11 (`ENTR_KOKIRI_FOREST_11`, 840 from the ring) on `hookshot` (adult
/// layer 2), and its `Obj_Mure3`.
fn forest(a: &Arc<GameAssets>) -> (PlayState, ActorHandle) {
    let e = a.scenes.entrance_index("ENTR_KOKIRI_FOREST_11").expect("entrance");
    let mut save = SaveContext::new(e, true, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("hookshot").unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    settle(&mut w, ACTOR_OBJ_MURE3);
    let h = w.actors.all().into_iter().find(|&h| w.actors.downcast::<ObjMure3>(h).is_some()).expect("the ring");
    (w, h)
}

/// Ticks until the room's actors of `id` are in and initialised (`Actor_UpdateAll` runs an init
/// once its object is loaded, without the update that frame); stops on that tick.
fn settle(w: &mut PlayState, id: i16) {
    for _ in 0..60 {
        let mine: Vec<&dyn oot_game::actor_ctx::ActorImpl> = w.actors.all().into_iter().filter_map(|h| w.actors.get(h)).filter(|a| a.base().id == id).collect();
        if !mine.is_empty() && mine.iter().all(|a| a.name() != "(not initialised)") {
            return;
        }
        w.tick_with(oot_game::play::scripted_input(eng_input::pad::PadState::default(), eng_input::pad::PadState::default()));
    }
    panic!("actor {id:#x} not initialised");
}

fn mure(w: &mut PlayState, h: ActorHandle) -> &mut ObjMure3 {
    w.actors.downcast_mut::<ObjMure3>(h).unwrap()
}

/// Its update alone, with `projectedPos` at `dist` straight ahead of the camera.
fn update_at(w: &mut PlayState, h: ActorHandle, dist: f32) {
    mure(w, h).actor.projected_pos = Vec3::new(0.0, 0.0, dist);
    let mut a = w.actors.take(h).unwrap();
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// The live rupees out (their handles, params and places).
fn rupees(w: &PlayState) -> Vec<(ActorHandle, i16, Vec3)> {
    w.actors.all().into_iter().filter_map(|h| w.actors.actor(h).filter(|a| a.id == ACTOR_EN_ITEM00 && !a.killed && a.world_pos.distance(RING) < 60.0).map(|a| (h, a.params, a.world_pos))).collect()
}

#[test]
fn the_ring_comes_out_near_the_camera_and_goes_back_far() {
    let Some(a) = assets() else { return };
    let (mut w, h) = forest(&a);
    let x = mure(&mut w, h);
    assert_eq!((TYPE.get(x.actor.params), SWITCH_FLAG.get(x.actor.params)), (2, 0x3F));
    // ObjMure3_Init: switch 0x3F clear, waiting (func_80B9AF34, then func_80B9AF64); past 1,450
    // whatever came out with the camera near is taken back.
    update_at(&mut w, h, 1500.0);
    update_at(&mut w, h, 1500.0);
    update_at(&mut w, h, 1200.0);
    assert_eq!(mure(&mut w, h).action, Action::WaitNear);
    assert!(rupees(&w).is_empty());
    // Within 1,150: func_80B9ABA0: six green rupees 40 round it (from its facing, 0x2AAA apart) and
    // a red one in the middle, hanging (Item_DropCollectible2 with 0x4000: no gravity).
    update_at(&mut w, h, 1100.0);
    assert_eq!(mure(&mut w, h).action, Action::WaitFar);
    let out = rupees(&w);
    assert_eq!(out.len(), 7);
    assert_eq!(out.iter().filter(|r| r.1 & 0xFF == ITEM00_RUPEE_GREEN).count(), 6);
    let red: Vec<_> = out.iter().filter(|r| r.1 & 0xFF == ITEM00_RUPEE_RED).collect();
    assert_eq!(red.len(), 1);
    assert!(red[0].2.distance(RING) < 0.5);
    for r in out.iter().filter(|r| r.1 & 0xFF == ITEM00_RUPEE_GREEN) {
        assert!(((r.2.x - RING.x).hypot(r.2.z - RING.z) - 40.0).abs() < 0.5 && r.2.y == RING.y, "{}", r.2);
        assert_eq!(w.actors.actor(r.0).unwrap().gravity, 0.0);
    }
    // One taken (its update gone): func_80B9ADCC marks it gone for good.
    let taken = out[0].0;
    w.actors.actor_mut(taken).unwrap().kill();
    update_at(&mut w, h, 1100.0);
    assert_eq!(mure(&mut w, h).unk_16c.count_ones(), 1);
    // Past 1,450: func_80B9ACE4 takes the rest back; near again, six come out.
    update_at(&mut w, h, 1500.0);
    assert_eq!(mure(&mut w, h).action, Action::WaitNear);
    assert!(rupees(&w).is_empty());
    update_at(&mut w, h, 1000.0);
    assert_eq!(rupees(&w).len(), 6);
    // No rupee carried off (Actor_HasParent): switch 0x3F stays clear.
    assert!(!w.flags.get_switch(0x3F));
}
