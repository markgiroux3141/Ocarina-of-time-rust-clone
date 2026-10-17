//! GAME-09 milestone 4's runs in King Dodongo's room (`ddan_boss`), each a short start of one of
//! its moments: `Route::KingDodongo` (from the room's spawn on `cavern-boss`: the cracked floor
//! bombed, the drop, his intro to the fight's start) and `Route::Ruby` (after his defeat: the heart
//! container, the blue warp, the Goron's Ruby's cutscene to Death Mountain Trail's spawn 5, Phase
//! 10's exit). The fight itself is the hand test's. Each run's end that a later one starts from is
//! a preset, checked here against the run.
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

/// Runs `route` from its start; the play state at its end, its run and its start's save.
fn run(a: &Arc<GameAssets>, route: Route) -> (PlayState, Playthrough, SaveContext) {
    let e = a.scenes.entrance_index(route.entrance()).unwrap();
    let start = route.save(e);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), start.clone()).unwrap();
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
    (w, run, start)
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
fn the_floor_bombed_the_drop_and_king_dodongo_s_intro() {
    let Some(a) = assets() else { return };
    let (mut w, run, _start) = run(&a, Route::KingDodongo);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::BombOut, Step::PutDown, Step::FloorBombed, Step::KingDodongoBattle]);
    ends_as_preset(&mut w, "cavern-boss-fight", |_| {});
}

#[test]
fn the_heart_the_warp_and_the_goron_s_ruby() {
    let Some(a) = assets() else { return };
    let (mut w, run, _start) = run(&a, Route::Ruby);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::HeartTaken, Step::WarpEntered, Step::WarpedOut, Step::Arrived, Step::RubyOver, Step::Trail5]);
    ends_as_preset(&mut w, "goron-ruby", |_| {});
}
