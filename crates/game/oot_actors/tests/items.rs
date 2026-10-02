//! `En_Item00` and the drops (`z_en_item00.c`), and `Item_Give` (`z_parameter.c`), in Kokiri
//! Forest after `Play_Init` (child Link, a new save: three hearts, no rupees). Expected values
//! come from the C: `sItemDropIds` / `sDropQuantities`, `Item_DropCollectibleRandom`,
//! `func_8001F404`, `EnItem00_Update`.

mod common;

use std::sync::Arc;

use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_item00::{self, *};
use oot_game::actor_ctx::ActorHandle;
use oot_game::play::{PlayState, Rand, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn enter() -> Option<PlayState> {
    let a = assets()?;
    let e = a.scenes.entrance_index("ENTR_KOKIRI_FOREST_0").expect("entrance");
    let save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    let mut w = oot_actors::play_entrance(a, common::data()?, common::rules()?, save).expect("Play_Init");
    frames(&mut w, 30);
    Some(w)
}

fn frames(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// A `Rand` whose next `Rand_ZeroOne() * 16` is `index`.
fn rand_for(index: i16) -> Rand {
    (1..).map(|s| Rand { state: s }).find(|r| (r.clone().zero_one() * 16.0) as i16 == index).unwrap()
}

fn items(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnItem00>(h).is_some()).collect()
}

#[test]
fn the_drop_tables_are_the_cs() {
    let Some(a) = assets() else { return };
    let t = &a.item_drops;
    // 15 tables of 16 each (the decomp before 52a510f had 4 zeros of padding after the quantities).
    assert_eq!((t.ids.len(), t.quantities.len()), (240, 240));
    // Table 2 (Kokiri Forest's bushes: params 0x0200), entries 0x20..0x2F.
    let row: Vec<i16> = t.ids[0x20..0x30].iter().map(|&v| v as i16).collect();
    assert_eq!(
        row,
        [
            ITEM00_RUPEE_GREEN,
            ITEM00_RUPEE_GREEN,
            ITEM00_MAGIC_SMALL,
            ITEM00_NONE,
            ITEM00_RECOVERY_HEART,
            ITEM00_NONE,
            ITEM00_NONE,
            ITEM00_RECOVERY_HEART,
            ITEM00_NONE,
            ITEM00_SEEDS,
            ITEM00_SEEDS,
            ITEM00_NONE,
            ITEM00_BOMBS_A,
            ITEM00_NONE,
            ITEM00_FLEXIBLE,
            ITEM00_MAGIC_SMALL
        ]
    );
    assert!(t.quantities[0x20..0x30].iter().all(|&q| q == 1));
    // sDropQuantities' first 3: table 4 drops three of its entry 11.
    assert_eq!((t.quantities[0x4A], t.quantities[0x4B]), (1, 3));
}

#[test]
fn func_8001f404_keeps_only_what_link_can_use() {
    let Some(mut w) = enter() else { return };
    // No bomb bag, bow, slingshot or magic meter: those drops are cancelled; a recovery heart at
    // full health is a green rupee.
    for id in [ITEM00_BOMBS_A, ITEM00_ARROWS_SMALL, ITEM00_SEEDS, ITEM00_MAGIC_SMALL] {
        assert_eq!(func_8001f404(&w, id), -1);
    }
    assert_eq!(func_8001f404(&w, ITEM00_RECOVERY_HEART), ITEM00_RUPEE_GREEN);
    w.save.health = 0x20;
    assert_eq!(func_8001f404(&w, ITEM00_RECOVERY_HEART), ITEM00_RECOVERY_HEART);
    // A child's arrows are seeds (then cancelled: no slingshot).
    assert_eq!(func_8001f404(&w, ITEM00_ARROWS_MEDIUM), -1);
}

#[test]
fn a_bush_drop_pops_up_lands_and_is_collected() {
    let Some(mut w) = enter() else { return };
    let pos = Vec3::new(100.0, 0.0, 700.0);
    let before = items(&w);
    // Entry 0 of table 2: a green rupee.
    w.rand = rand_for(0);
    en_item00::item_drop_collectible_random(&mut w, None, pos, 2 << 4);
    let new: Vec<_> = items(&w).into_iter().filter(|h| !before.contains(h)).collect();
    assert_eq!(new.len(), 1, "one drop (sDropQuantities 1)");
    let h = new[0];
    let e = w.actors.downcast::<EnItem00>(h).unwrap();
    // Item_DropCollectibleRandom's pop: up at 8, 2 across, gravity -0.9, scale 0, 220 frames,
    // in no room; EnItem00_Init's rupee scale 0.015 and yOffset 750.
    assert_eq!((e.actor.params, e.action, e.despawn_timer, e.actor.room), (ITEM00_RUPEE_GREEN, en_item00::Action::Pop, 220, -1));
    assert_eq!((e.actor.velocity.y, e.actor.speed_xz, e.actor.gravity, e.actor.scale.x, e.scale, e.actor.shape_y_offset), (8.0, 2.0, -0.9, 0.0, 0.015, 750.0));
    // It rises, falls back and settles (func_8001E304, then func_8001DFC8 on the floor).
    let mut landed = None;
    for f in 0..60 {
        frames(&mut w, 1);
        let e = w.actors.downcast::<EnItem00>(h).unwrap();
        if e.action == en_item00::Action::Rest {
            landed = Some(f);
            break;
        }
    }
    assert!(landed.is_some(), "it lands");
    let e = w.actors.downcast::<EnItem00>(h).unwrap();
    let (floor, _) = w.col.entity_raycast_down(e.actor.world_pos + Vec3::Y * 50.0);
    assert!((e.actor.world_pos.y - floor).abs() < 1.0, "on the floor ({floor}): {}", e.actor.world_pos.y);
    // The scale closes a tenth of the gap a frame, at most 10% of the target
    // (Math_SmoothStepToF with minStep 0: it never quite gets there).
    let s0 = w.actors.downcast::<EnItem00>(h).unwrap().actor.scale.x;
    frames(&mut w, 1);
    let s1 = w.actors.downcast::<EnItem00>(h).unwrap().actor.scale.x;
    assert!((s1 - (s0 + ((0.015 - s0) * 0.1).min(0.0015))).abs() < 1e-7);
    frames(&mut w, 40);
    assert!((w.actors.downcast::<EnItem00>(h).unwrap().actor.scale.x - 0.015).abs() < 1e-4);
    // Link walks onto it: Item_Give(ITEM_RUPEE_GREEN), 1 rupee into the accumulator, then 15
    // frames over his head.
    let at = w.actors.downcast::<EnItem00>(h).unwrap().actor.world_pos;
    w.place_player(at, 0);
    frames(&mut w, 1);
    let e = w.actors.downcast::<EnItem00>(h).unwrap();
    assert_eq!((e.action, e.despawn_timer), (en_item00::Action::Collected, 15));
    assert_eq!(w.save.rupees + w.save.rupee_accumulator, 1);
    frames(&mut w, 16);
    assert!(w.actors.downcast::<EnItem00>(h).is_none(), "gone after 15 frames");
}

#[test]
fn a_heart_drop_heals() {
    let Some(mut w) = enter() else { return };
    w.save.health = 0x20;
    let before = items(&w);
    // Entry 4 of table 2: a recovery heart (kept: Link isn't at full health).
    w.rand = rand_for(4);
    en_item00::item_drop_collectible_random(&mut w, None, Vec3::new(100.0, 0.0, 700.0), 2 << 4);
    let h = items(&w).into_iter().find(|h| !before.contains(h)).expect("a drop");
    assert_eq!(w.actors.downcast::<EnItem00>(h).unwrap().actor.params, ITEM00_RECOVERY_HEART);
    frames(&mut w, 80);
    let at = w.actors.downcast::<EnItem00>(h).unwrap().actor.world_pos;
    w.place_player(at, 0);
    frames(&mut w, 1);
    // Item_Give(ITEM_RECOVERY_HEART): Health_ChangeBy(0x10).
    assert_eq!(w.save.health, 0x30);
}
