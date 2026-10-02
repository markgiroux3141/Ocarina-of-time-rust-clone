//! `En_Wonder_Item` (`ovl_En_Wonder_Item/z_en_wonder_item.c`), with Kokiri Forest's placements.
//!
//! Params: `wonderMode` bits 11..15, `itemDrop` bits 6..10, the switch flag bits 0..5 (0x3F for
//! none); `rot.z` the count. From the scene data:
//! - room 2, 0x123F at (-590, 120, 1635) and (-584, 120, 1762), `rot.z` 1: mode 2 (proximity
//!   drop), drop 8 (`ITEM00_RUPEE_GREEN`), no switch flag, one drop;
//! - room 0, 0x0260 at (364, 0, 28), `rot.z` 2: mode 0 (free multitag), drop 9
//!   (`ITEM00_RUPEE_BLUE`), switch 0x20, two points; its points 0x0FE0 (mode 1) at
//!   (548, 3, -158) `rot.z` 0 and (188, 3, -198) `rot.z` 1;
//! - room 0, 0x2A63 at (1074, 0, 178), `rot.z` 2: mode 5 (ordered multitag), drop 9, switch
//!   0x23, two points; its points 0x37E3 (mode 6) at (1069, 0, 406) `rot.z` 0 and
//!   (1074, 0, -80) `rot.z` 1.

mod common;

use std::sync::Arc;

use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_item00::{EnItem00, ITEM00_RUPEE_BLUE, ITEM00_RUPEE_GREEN};
use oot_actors::en_wonder_item::{EnWonderItem, WONDERITEM_MULTITAG_FREE, WONDERITEM_MULTITAG_ORDERED, WONDERITEM_PROXIMITY_DROP};
use oot_game::actor_ctx::ActorHandle;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn tick(w: &mut PlayState) {
    w.tick_with(scripted_input(PadState::default(), PadState::default()));
}

/// Kokiri Forest on a new save, 20 frames in (room 0 loaded), room 2 too if `room2`.
fn kokiri(a: &Arc<GameAssets>, room2: bool) -> Option<PlayState> {
    let e = a.scenes.entrance_index("ENTR_KOKIRI_FOREST_0").expect("entrance");
    let save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    let mut w = oot_actors::play_entrance(a.clone(), common::data()?, common::rules()?, save).expect("Play_Init");
    for _ in 0..20 {
        tick(&mut w);
    }
    if room2 {
        w.room_request(2);
        tick(&mut w);
        w.room_change_done();
        tick(&mut w);
    }
    Some(w)
}

fn wonder_item(w: &PlayState, home: Vec3) -> Option<ActorHandle> {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnWonderItem>(h).is_some_and(|i| i.actor.home_pos == home))
}

fn gone(w: &PlayState, h: ActorHandle) -> bool {
    w.actors.get(h).is_none_or(|a| a.base().killed)
}

fn drops(w: &PlayState) -> Vec<i16> {
    w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<EnItem00>(h)).map(|i| i.actor.params).collect()
}

#[test]
fn a_proximity_drop_gives_a_green_rupee_when_link_is_near() {
    let Some(a) = assets() else { return };
    let Some(mut w) = kokiri(&a, true) else { return };
    let home = Vec3::new(-590.0, 120.0, 1635.0);
    let h = wonder_item(&w, home).expect("room 2's wonder item");
    let i = w.actors.downcast::<EnWonderItem>(h).unwrap();
    assert_eq!((i.wonder_mode, i.item_drop, i.switch_flag, i.drop_count), (WONDERITEM_PROXIMITY_DROP, 8, -1, 1));
    // 60 away: nothing (EnWonderItem_ProximityDrop wants xzDistToPlayer under 50).
    w.place_player(home + Vec3::new(60.0, 0.0, 0.0), 0);
    for _ in 0..5 {
        tick(&mut w);
    }
    assert!(!gone(&w, h));
    // 40 away: EnWonderItem_DropCollectible(this, play, true): Item_DropCollectible(pos,
    // ITEM00_RUPEE_GREEN | 0x8000), and it goes. The drop collects itself.
    let rupees = w.save.rupees + w.save.rupee_accumulator;
    w.place_player(home + Vec3::new(40.0, 0.0, 0.0), 0);
    let mut saw_drop = false;
    for _ in 0..30 {
        tick(&mut w);
        saw_drop |= drops(&w).contains(&ITEM00_RUPEE_GREEN);
    }
    assert!(gone(&w, h));
    assert!(saw_drop, "a green rupee dropped");
    assert_eq!(w.save.rupees + w.save.rupee_accumulator, rupees + 1);
}

#[test]
fn the_free_multitag_drops_a_blue_rupee_when_both_points_are_touched() {
    let Some(a) = assets() else { return };
    let Some(mut w) = kokiri(&a, false) else { return };
    let h = wonder_item(&w, Vec3::new(364.0, 0.0, 28.0)).expect("room 0's free multitag");
    let i = w.actors.downcast::<EnWonderItem>(h).unwrap();
    assert_eq!((i.wonder_mode, i.item_drop, i.switch_flag, i.num_tag_points), (WONDERITEM_MULTITAG_FREE, 9, 0x20, 2));
    // Its tag points noted their places and went (EnWonderItem_Init, mode 1).
    assert!(wonder_item(&w, Vec3::new(548.0, 3.0, -158.0)).is_none() && wonder_item(&w, Vec3::new(188.0, 3.0, -198.0)).is_none());
    // Point 1, then point 0 (any order): within 50 of each, the timer restarting (81 frames).
    w.place_player(Vec3::new(188.0, 0.0, -198.0), 0);
    tick(&mut w);
    tick(&mut w);
    let i = w.actors.downcast::<EnWonderItem>(h).unwrap();
    assert_eq!((i.tag_flags, i.tag_count, i.timer), (0b10, 1, 80));
    assert!(!w.flags.get_switch(0x20));
    w.place_player(Vec3::new(548.0, 0.0, -158.0), 0);
    let rupees = w.save.rupees + w.save.rupee_accumulator;
    let mut saw_drop = false;
    for _ in 0..40 {
        tick(&mut w);
        saw_drop |= drops(&w).contains(&ITEM00_RUPEE_BLUE);
    }
    // Both tagged: the switch flag (twice: here and in the drop), a blue rupee that collects
    // itself (5), and it goes.
    assert!(gone(&w, h));
    assert!(w.flags.get_switch(0x20));
    assert!(saw_drop);
    assert_eq!(w.save.rupees + w.save.rupee_accumulator, rupees + 5);
}

#[test]
fn the_ordered_multitag_ends_on_a_point_out_of_order() {
    let Some(a) = assets() else { return };
    let home = Vec3::new(1074.0, 0.0, 178.0);
    // Point 1 first: EnWonderItem_MultitagOrdered goes, with nothing.
    let Some(mut w) = kokiri(&a, false) else { return };
    let h = wonder_item(&w, home).expect("room 0's ordered multitag");
    let i = w.actors.downcast::<EnWonderItem>(h).unwrap();
    assert_eq!((i.wonder_mode, i.item_drop, i.switch_flag, i.num_tag_points), (WONDERITEM_MULTITAG_ORDERED, 9, 0x23, 2));
    w.place_player(Vec3::new(1074.0, 0.0, -80.0), 0);
    tick(&mut w);
    tick(&mut w);
    assert!(gone(&w, h));
    assert!(!w.flags.get_switch(0x23));

    // In order: point 0, then point 1 within the 81 frames: the blue rupee and switch 0x23.
    let Some(mut w) = kokiri(&a, false) else { return };
    let h = wonder_item(&w, home).unwrap();
    w.place_player(Vec3::new(1069.0, 0.0, 406.0), 0);
    tick(&mut w);
    tick(&mut w);
    let i = w.actors.downcast::<EnWonderItem>(h).unwrap();
    assert_eq!((i.tag_flags, i.next_tag), (0b01, 1));
    w.place_player(Vec3::new(1074.0, 0.0, -80.0), 0);
    for _ in 0..3 {
        tick(&mut w);
    }
    assert!(gone(&w, h));
    assert!(w.flags.get_switch(0x23));

    // In order but too slowly: the timer runs out (timer == 1) and it goes.
    let Some(mut w) = kokiri(&a, false) else { return };
    let h = wonder_item(&w, home).unwrap();
    w.place_player(Vec3::new(1069.0, 0.0, 406.0), 0);
    tick(&mut w);
    w.place_player(Vec3::new(900.0, 0.0, 178.0), 0);
    for _ in 0..81 {
        tick(&mut w);
    }
    assert!(gone(&w, h));
    assert!(!w.flags.get_switch(0x23));
}
