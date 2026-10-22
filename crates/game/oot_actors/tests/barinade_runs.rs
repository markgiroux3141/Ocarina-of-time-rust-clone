//! GAME-10 milestone 6's runs: `Route::Barinade` (Link falls into Barinade's room, his intro to
//! the fight's start) and `Route::Sapphire` (after his defeat, a debug start: the heart container,
//! Ruto's warp, the Zora's Sapphire's cutscene at Zora's Fountain: Phase 11's exit, part 1). The
//! fight itself is the hand test's. Each run's end that a later one starts from is a preset,
//! checked here against the run.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use oot_actors::PlayExt;
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Runs `route` from its start; the play state at its end and its run.
fn run(a: &Arc<GameAssets>, route: Route) -> (PlayState, Playthrough) {
    let e = a.scenes.entrance_index(route.entrance()).unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), route.save(e)).unwrap();
    w.audio.log = Some(Default::default());
    route.debug_start(&mut w);
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
    }
    eprintln!("steps {:?}", run.steps);
    eprintln!("texts {:x?}", run.texts);
    eprintln!("at {:?} health {} scene {:#x}", w.player().actor.world_pos, w.save.health, w.scene_id);
    assert!(run.failure.is_none(), "{:?}", run.failure);
    (w, run)
}

/// The run's save is the preset's in its flags, inventory and equipment, with `extra` applied to
/// the run's (what the run doesn't do that the preset has).
fn ends_as_preset(w: &mut PlayState, preset: &str, extra: impl Fn(&mut SaveContext)) {
    // The scene's flags as leaving it saves them (Play_SaveSceneFlags).
    w.save_scene_flags();
    let mut s = w.save.clone();
    extra(&mut s);
    let mut p = SaveContext::new(s.entrance_index, false, s.day_time);
    p.apply_preset(preset).unwrap();
    assert_eq!((&p.event_chk_inf, &p.inf_table, &p.item_get_inf, &p.event_inf), (&s.event_chk_inf, &s.inf_table, &s.item_get_inf, &s.event_inf));
    assert_eq!((&p.inventory.items, &p.inventory.ammo, p.inventory.upgrades, p.inventory.quest_items), (&s.inventory.items, &s.inventory.ammo, s.inventory.upgrades, s.inventory.quest_items));
    assert_eq!((p.rupees, &p.equips.button_items, p.health_capacity), (s.rupees + s.rupee_accumulator, &s.equips.button_items, s.health_capacity));
    for (i, (a, b)) in p.scene_flags.iter().zip(s.scene_flags.iter()).enumerate() {
        assert_eq!(format!("{a:?}"), format!("{b:?}"), "scene {i:#x}'s flags");
    }
}

#[test]
fn the_fall_and_barinades_intro_to_the_fight() {
    let Some(a) = assets() else { return };
    let (mut w, run) = run(&a, Route::Barinade);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::BarinadeBattle]);
    // The preset's boomerang on C-Down is the hand test's (the pause menu's), not the run's.
    ends_as_preset(&mut w, "barinade-fight", |s| s.equip_item_on_c(1, oot_game::item::ITEM_BOOMERANG));
}

#[test]
fn the_heart_rutos_warp_and_the_zoras_sapphire() {
    let Some(a) = assets() else { return };
    let (mut w, run) = run(&a, Route::Sapphire);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::HeartTaken, Step::RutoWarpEntered, Step::RutoWarpTalked, Step::WarpedOut, Step::Arrived, Step::SapphireOver, Step::ZorasFountain]);
    // Ruto's words in her warp (0x4022), then the fountain's: 0x4025 (a choice), 0x4026, 0x4027,
    // 0x0082 and, the Ruby held, 0x4035 (CS_TEXT_ZORA_SAPPHIRE), 0x4028.
    assert!(run.texts.contains(&0x4022));
    assert!(w.save.check_quest_item(oot_game::item::QUEST_ZORA_SAPPHIRE));
    assert_eq!(w.save.scene_layer, 0);
    ends_as_preset(&mut w, "zora-sapphire", |_| {});
}
