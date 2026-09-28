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
    let o = Options { entrance: Some("ENTR_SPOT04_0".into()), room: Some(2), at: vec![-232.0, 178.0, 2211.0, 0.0], time: "10:00".into(), child: true, ..Default::default() };
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
