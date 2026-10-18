//! GAME-09 milestone 6's runs on Death Mountain's summit and in the Great Fairy's fountain, each a
//! short debug start of one moment: `Route::FairyWall` (a bomb breaks the Great Fairy's wall, in
//! through the hole), `Route::GreatFairy` (Zelda's Lullaby on the Triforce, her cutscene and the
//! magic, out by the return entrance to the trail's spawn 4) and `Route::SummitOwl` (with magic, the
//! owl's flight to Kakariko). The climb through the falling rocks and the Skullwalltulas is the hand
//! test's. Each run's end that a later one starts from is a preset, checked here against the run.
//!
//! And the owl without magic (`EnOwl_WaitDeathMountainShortcut`: 0x3062, he stays).
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use oot_actors::PlayExt;
use oot_actors::playthrough::{Playthrough, Route, SUMMIT_OWL, Step};
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
    let start = route.save(e);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), start).unwrap();
    route.debug_start(&mut w);
    // The ocarina's routes: AudioOcarina_Update runs in Audio_Update, on the audio side.
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
    eprintln!("at {:?} health {} scene {:#x}", w.player().actor.world_pos, w.save.health, w.scene_id);
    assert!(run.failure.is_none(), "{:?}", run.failure);
    (w, run)
}

/// The run's save is the preset's in its flags, inventory, equipment and magic.
fn ends_as_preset(w: &mut PlayState, preset: &str) {
    // The scene's flags as leaving it saves them (Play_SaveSceneFlags).
    w.save_scene_flags();
    let s = w.save.clone();
    let mut p = SaveContext::new(s.entrance_index, false, s.day_time);
    p.apply_preset(preset).unwrap();
    assert_eq!((&p.event_chk_inf, &p.inf_table, &p.item_get_inf, &p.event_inf), (&s.event_chk_inf, &s.inf_table, &s.item_get_inf, &s.event_inf));
    assert_eq!((&p.inventory.items, &p.inventory.ammo, p.inventory.upgrades, p.inventory.quest_items), (&s.inventory.items, &s.inventory.ammo, s.inventory.upgrades, s.inventory.quest_items));
    assert_eq!((p.rupees, &p.equips.button_items, p.health_capacity), (s.rupees + s.rupee_accumulator, &s.equips.button_items, s.health_capacity));
    assert_eq!(
        (p.is_magic_acquired, p.is_double_magic_acquired, p.magic_level, p.magic, p.magic_capacity, p.magic_fill_target),
        (s.is_magic_acquired, s.is_double_magic_acquired, s.magic_level, s.magic, s.magic_capacity, s.magic_fill_target)
    );
    for (i, (a, b)) in p.scene_flags.iter().zip(s.scene_flags.iter()).enumerate() {
        assert_eq!(format!("{a:?}"), format!("{b:?}"), "scene {i:#x}'s flags");
    }
}

fn steps(run: &Playthrough) -> Vec<Step> {
    run.steps.iter().map(|s| s.0).collect()
}

/// `SCENE_GREAT_FAIRYS_FOUNTAIN_MAGIC`, `SCENE_KAKARIKO_VILLAGE`, `SCENE_DEATH_MOUNTAIN_TRAIL`
/// (`scene_table.h`: 0x3B, 0x52, 0x60).
const SCENE_GREAT_FAIRYS_FOUNTAIN_MAGIC: u16 = 0x3B;
const SCENE_KAKARIKO_VILLAGE: u16 = 0x52;
const SCENE_DEATH_MOUNTAIN_TRAIL: u16 = 0x60;

#[test]
fn a_bomb_breaks_the_great_fairy_s_wall() {
    let Some(a) = assets() else { return };
    let (mut w, run) = run(&a, Route::FairyWall);
    assert_eq!(steps(&run), vec![Step::BombOut, Step::PutDown, Step::FairyWall, Step::Fountain]);
    // Exit 5 (ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_0): the fountain, entered at spawn 0.
    assert_eq!((w.scene_id, w.cur_spawn), (SCENE_GREAT_FAIRYS_FOUNTAIN_MAGIC, 0));
    ends_as_preset(&mut w, "fairy-fountain");
}

#[test]
fn the_great_fairy_gives_magic_and_the_return_entrance_leads_back() {
    let Some(a) = assets() else { return };
    let (mut w, run) = run(&a, Route::GreatFairy);
    assert_eq!(steps(&run), vec![Step::OcarinaOut, Step::LullabyPlayed, Step::Magic, Step::Trail4]);
    // The fountain's exit 0x7FFE with spawn 0: sReturnEntranceGroupData[0],
    // ENTR_DEATH_MOUNTAIN_TRAIL_4 (spawn 4, in the wall's hole).
    assert_eq!((w.scene_id, w.cur_spawn), (SCENE_DEATH_MOUNTAIN_TRAIL, 4));
    // The meter grown and filled on the way out (Interface_Update's start: magicLevel 1,
    // MAGIC_STATE_STEP_CAPACITY to 0x30, then the fill to magicFillTarget 0x30).
    assert_eq!((w.save.magic_level, w.save.magic_capacity, w.save.magic, w.save.magic_state), (1, 0x30, 0x30, oot_game::magic::MAGIC_STATE_IDLE));
    ends_as_preset(&mut w, "fairy-magic");
}

#[test]
fn with_magic_the_owl_flies_link_to_kakariko() {
    let Some(a) = assets() else { return };
    let (w, run) = run(&a, Route::SummitOwl);
    assert_eq!(steps(&run), vec![Step::OwlTalk, Step::Kakariko14]);
    // EnOwl_WaitDeathMountainShortcut with isMagicAcquired: 0x3063 (func_80ACB22C, then the flight,
    // gDMTOwlCs, whose terminator 66 is ENTR_KAKARIKO_VILLAGE_14).
    assert!(run.texts.contains(&0x3063), "{:x?}", run.texts);
    assert_eq!(w.scene_id, SCENE_KAKARIKO_VILLAGE);
    assert_eq!(Some(w.save.entrance_index), a.scenes.entrance_index("ENTR_KAKARIKO_VILLAGE_14"));
}

#[test]
fn without_magic_the_owl_only_talks() {
    let Some(a) = assets() else { return };
    let e = a.scenes.entrance_index("ENTR_DEATH_MOUNTAIN_TRAIL_4").unwrap();
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("fairy-fountain").unwrap();
    assert!(!save.is_magic_acquired);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap();
    for _ in 0..30 {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
    // Within 120 of him (func_80ACA558's reach), facing him: he offers 0x3062.
    w.place_player(SUMMIT_OWL + glam::Vec3::new(-80.0, -33.0, 0.0), 0x4000);
    let mut prev = PadState::default();
    let mut seen = None;
    for i in 0..600 {
        let mut p = PadState::default();
        if i % 2 == 0 {
            p.button |= BTN_A;
        }
        w.tick_with(scripted_input(prev, p));
        prev = p;
        if w.msg_ctx.text_id != 0 && seen.is_none() {
            seen = Some(w.msg_ctx.text_id);
        }
    }
    assert_eq!(seen, Some(0x3062));
    // func_80ACB274: the text closed, back to waiting; no flight, no scene change.
    assert_eq!(w.scene_id, SCENE_DEATH_MOUNTAIN_TRAIL);
    let owl = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<oot_actors::en_owl::EnOwl>(h)).expect("the owl");
    assert!(!owl.actor.killed);
    assert!(w.cs_ctx.state == oot_game::cutscene::CS_STATE_IDLE);
}
