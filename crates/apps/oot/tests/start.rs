//! The game's debug start: `--entrance` with `--room` and `--at` (`game-sword-chest.bat`).

use eng_input::pad::PadState;
use oot::Options;
use oot_actors::PlayExt;
use oot_game::play::scripted_input;

#[test]
fn room_and_at_start_in_front_of_the_sword_chest() {
    // Only with a current pack: load_assets would import one otherwise.
    if !oot_game::pack::default_pack_path().is_ok_and(|p| oot_game::pack::is_current(&p, None)) {
        return;
    }
    let o = Options { entrance: Some("ENTR_KOKIRI_FOREST_0".into()), room: Some(2), at: vec![-232.0, 178.0, 2211.0, 0.0], time: "10:00".into(), child: true, ..Default::default() };
    let a = oot::load_assets(&o).expect("the assets");
    let mut w = oot::new_play(&a, true);
    let none = PadState::default();
    for _ in 0..3 {
        w.tick_with(scripted_input(none, none));
    }
    assert_eq!(w.room_ctx.cur.num, 2);
    let p = w.player();
    assert!(p.actor.world_pos.distance(glam::Vec3::new(-232.0, 178.0, 2211.0)) < 1.0, "{:?}", p.actor.world_pos);
    // A new save, and the chest offering the Kokiri Sword (-GI_SWORD_KOKIRI).
    assert_eq!(w.save.equips.button_items[0], oot_game::save::ITEM_NONE);
    assert!(p.interact_range_actor.is_some());
    assert_eq!(p.get_item_id, -oot_game::item::GI_SWORD_KOKIRI);
}

/// `--clear 9 --room 9 --at ...` (`game-boss-room.bat`): room 9 cleared before the room change,
/// so its door to room 11 (transition 4, barred until the room is cleared) starts unbarred.
#[test]
fn clear_starts_room_9_with_its_door_open() {
    if !oot_game::pack::default_pack_path().is_ok_and(|p| oot_game::pack::is_current(&p, None)) {
        return;
    }
    let door = |clears: Vec<i8>| {
        let o = Options { entrance: Some("ENTR_DEKU_TREE_0".into()), preset: Some("deku-tree-slingshot".into()), room: Some(9), clears, at: vec![-660.0, -1880.0, -620.0, 32768.0], time: "10:00".into(), child: true, ..Default::default() };
        let a = oot::load_assets(&o).expect("the assets");
        let mut w = oot::new_play(&a, true);
        let none = PadState::default();
        for _ in 0..3 {
            w.tick_with(scripted_input(none, none));
        }
        assert_eq!(w.room_ctx.cur.num, 9);
        let d = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<oot_actors::door_shutter::DoorShutter>(h).filter(|d| d.transition_index() == 4)).expect("room 9's door");
        (w.flags.get_clear(9), d.bars_closed_amount)
    };
    assert_eq!(door(vec![9]), (true, 0.0));
    let (cleared, bars) = door(Vec::new());
    assert!(!cleared && bars > 0.0, "{bars}");
}

/// `--cutscene 0xFFF2` (`game-emerald.bat`): the debug start enters Kokiri Forest's cutscene
/// layer 6 (`CS_INDEX_2`), its script running: the Kokiri Emerald's last part.
#[test]
fn cutscene_starts_kokiri_forest_in_its_layer_6() {
    if !oot_game::pack::default_pack_path().is_ok_and(|p| oot_game::pack::is_current(&p, None)) {
        return;
    }
    let o = Options { entrance: Some("ENTR_KOKIRI_FOREST_0".into()), preset: Some("deku-tree-dead".into()), cutscene: Some(0xFFF2), time: "10:00".into(), child: true, ..Default::default() };
    let a = oot::load_assets(&o).expect("the assets");
    let mut w = oot::new_play(&a, true);
    let none = PadState::default();
    for _ in 0..3 {
        w.tick_with(scripted_input(none, none));
    }
    assert_eq!(w.save.scene_layer, 6);
    assert_eq!(w.cs_ctx.segment.as_ref().map(|s| s.name.as_str()), Some("gKokiriForestKokiriEmeraldPart9Cs"));
    assert_ne!(w.cs_ctx.state, oot_game::cutscene::CS_STATE_IDLE);
    // Without --entrance, it's refused.
    let o = Options { cutscene: Some(0xFFF2), ..Default::default() };
    assert!(oot::load_assets(&o).is_err());
}
