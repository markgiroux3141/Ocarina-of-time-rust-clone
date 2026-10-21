//! GAME-10 milestones 2 and 3's runs, each a short debug start of one step of the story, no fights:
//! `Route::ZorasRiver` (Zelda's Lullaby at the falls, `Bg_Spot03_Taki` and one-point 4100, the jump
//! over the falls' gap into Zora's Domain and its intro), `Route::KingZora` (Ruto's letter shown,
//! his mweep aside) and `Route::JabuFish` (a fish let out by Lord Jabu-Jabu, `gJabuInhalingCs`,
//! inside). Each run's end that a preset holds is checked against the run.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::item::{ITEM_BOTTLE_EMPTY, ITEM_BOTTLE_FISH, SLOT_BOTTLE_1};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Runs `route` from its start; the play state at its end and its run. The ocarina's route gets
/// the audio side (`needs_audio`), as the sandbox gives it (tests/summit.rs's).
fn run(a: &Arc<GameAssets>, route: Route) -> (PlayState, Playthrough) {
    let e = a.scenes.entrance_index(route.entrance()).unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), route.save(e)).unwrap();
    w.audio.log = Some(Default::default());
    route.debug_start(&mut w);
    if route.needs_audio() {
        w.audio_side = Some(Box::new(oot_game::audio::offline::OfflineAudio::new(&a.pack.audio_data().unwrap(), false)));
    }
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
    }
    eprintln!("steps {:?}", run.steps);
    eprintln!("texts {:x?}", run.texts);
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
    assert_eq!((p.rupees, &p.equips.button_items), (s.rupees + s.rupee_accumulator, &s.equips.button_items));
    for (i, (a, b)) in p.scene_flags.iter().zip(s.scene_flags.iter()).enumerate() {
        assert_eq!(format!("{a:?}"), format!("{b:?}"), "scene {i:#x}'s flags");
    }
}

#[test]
fn the_lullaby_opens_the_falls_into_zora_s_domain() {
    let Some(a) = assets() else { return };
    let (mut w, run) = run(&a, Route::ZorasRiver);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::OcarinaOut, Step::LullabyPlayed, Step::WaterfallOpened, Step::ZorasDomain, Step::DomainIntro]);
    // SCENE_ZORAS_DOMAIN (scene_table.h: 0x58), the intro seen (EVENTCHKINF_A4).
    assert_eq!(w.scene_id, 0x58);
    // zora-diving is this save with its 50 rupees.
    ends_as_preset(&mut w, "zora-diving", |s| s.rupees = 50);
}

#[test]
fn ruto_s_letter_moves_king_zora_aside() {
    let Some(a) = assets() else { return };
    let (mut w, run) = run(&a, Route::KingZora);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::KingZoraLetter, Step::KingMoved]);
    // The letter shown answers 0x401A at once (EnKz's exchange: 0x401B, then its next 0x401C).
    assert_eq!(run.texts, vec![0x401B, 0x401C]);
    assert_eq!(w.save.inventory.items[SLOT_BOTTLE_1], ITEM_BOTTLE_EMPTY);
    // jabu-fish is this save with the fountain's intro seen (EVENTCHKINF_B7) and a fish caught in
    // the emptied bottle (the bottle run's catch).
    ends_as_preset(&mut w, "jabu-fish", |s| {
        s.set_event_chk_inf(0xB7);
        s.inventory.items[SLOT_BOTTLE_1] = ITEM_BOTTLE_FISH;
        s.equips.button_items[1] = ITEM_BOTTLE_FISH;
    });
}

#[test]
fn a_fish_for_lord_jabu_jabu_and_he_swallows_link() {
    let Some(a) = assets() else { return };
    let (w, run) = run(&a, Route::JabuFish);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::FishLetOut, Step::JabuSwallowed]);
    // SCENE_JABU_JABU (scene_table.h: 0x02); EVENTCHKINF_OPENED_JABU_JABU (save.h: 0x3A) set as
    // the cutscene starts; the bottle empty.
    assert_eq!(w.scene_id, 0x02);
    assert!(w.save.get_event_chk_inf(0x3A));
    assert_eq!(w.save.inventory.items[SLOT_BOTTLE_1], ITEM_BOTTLE_EMPTY);
    // GAME-10 milestone 4 starts from this save.
    let mut w = w;
    ends_as_preset(&mut w, "jabu-jabu", |_| {});
}
