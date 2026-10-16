//! GAME-09 milestone 2's runs: `Route::Darunia` (from Goron City's bottom floor on `ranch-epona`:
//! Zelda's Lullaby at Darunia's door, `En_Okarina_Tag` 0x1C9B and switch 0x1B; Saria's Song to
//! Darunia, `gGoronCityDaruniaCorrectSongCs`; the Goron's Bracelet) and `Route::Boulder` (from a
//! debug start on the cliff above Dodongo's Cavern's mouth on `goron-bracelet`: the cliff's Bomb
//! Flower thrown down onto the boulder, `Bg_Spot16_Bombstone`'s watch and blast). Each run's end is
//! the next one's preset.
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
    eprintln!("at {:?} health {}", w.player().actor.world_pos, w.save.health);
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
    assert_eq!((p.rupees, &p.equips.button_items), (s.rupees + s.rupee_accumulator, &s.equips.button_items));
    for (i, (a, b)) in p.scene_flags.iter().zip(s.scene_flags.iter()).enumerate() {
        assert_eq!(format!("{a:?}"), format!("{b:?}"), "scene {i:#x}'s flags");
    }
}

#[test]
fn darunias_door_his_dance_and_the_bracelet() {
    let Some(a) = assets() else { return };
    let (mut w, run, start) = run(&a, Route::Darunia);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    use Step::*;
    // The song spot turns Link's ocarina to itself (OcarinaOut), the lullaby sets switch 0x1B, the
    // door rises 180 at 1.44 a frame; Saria's Song by Darunia, his dance, the bracelet.
    assert_eq!(steps, vec![OcarinaOut, LullabyPlayed, DaruniaDoor, SariasSong, Bracelet]);
    // The correct song's script shows 0x3007 (its frame 335); after it 0x301C (EnDu_GetTextId:
    // no bracelet yet), and the bracelet's get-item text 0x79 (sGetItemTable's GI_GORONS_BRACELET).
    for t in [0x3007, 0x301C, 0x79] {
        assert!(run.texts.contains(&t), "text {t:#x}: {:x?}", run.texts);
    }
    // The strength upgrade (UPG_STRENGTH 1: Player_GetStrength's PLAYER_STR_BRACELET for a child);
    // no hit taken.
    assert_eq!(oot_game::player_lib::player_get_strength(&w.save), oot_game::player_lib::PLAYER_STR_BRACELET);
    assert_eq!(w.save.health, start.health);
    // The run's end is `goron-bracelet`, but for the two intros the run's entrances don't play
    // (ENTR_GORON_CITY_2 has no row in sEntranceCutsceneTable).
    ends_as_preset(&mut w, "goron-bracelet", |s| {
        s.set_event_chk_inf(0xA1);
        s.set_event_chk_inf(0xA6);
    });
}

#[test]
fn the_cliffs_bomb_flower_blows_the_boulder() {
    let Some(a) = assets() else { return };
    let (mut w, run, start) = run(&a, Route::Boulder);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    use Step::*;
    assert_eq!(steps, vec![FlowerLifted, Thrown, BoulderBlown]);
    // func_808B5950: the trail's switch 4 and EVENTCHKINF_23; the boulder gone (Actor_Kill at the
    // end of its dust, frame 61).
    assert!(w.flags.get_switch(4) && w.save.get_event_chk_inf(0x23));
    assert!(!w.actors.all().into_iter().any(|h| w.actors.downcast::<oot_actors::bg_spot16_bombstone::BgSpot16Bombstone>(h).is_some_and(|b| b.actor.params == 0x04FF)));
    assert_eq!(w.save.health, start.health);
    // The run's end is `cavern-open`.
    ends_as_preset(&mut w, "cavern-open", |_| {});
}

#[test]
fn no_attention_arrow_over_darunia_during_his_dance() {
    let Some(a) = assets() else { return };
    // Attention_FindActor searches nothing while Player_InCsMode: no arrow (Attention_Draw's
    // arrowHoverActor) over Darunia in his dance's cutscene, though he's in reach; before it, with
    // Link by him, the arrow is over him.
    let route = Route::Darunia;
    let e = a.scenes.entrance_index(route.entrance()).unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), route.save(e)).unwrap();
    w.audio_side = Some(Box::new(oot_game::audio::offline::OfflineAudio::new(&a.pack.audio_data().unwrap(), false)));
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    let du = |w: &PlayState| w.actors.all().into_iter().find(|&h| w.actors.downcast::<oot_actors::en_du::EnDu>(h).is_some());
    let (mut over_him, mut in_cs, mut frame) = (false, 0, 0);
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
        frame += 1;
        let d = du(&w);
        if w.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE && w.player_in_cs_mode() {
            in_cs += 1;
            assert_eq!(w.target_ctx.arrow_hover_actor, None, "frame {frame}: an arrow in the cutscene");
        } else if d.is_some() && w.target_ctx.arrow_hover_actor == d {
            over_him = true;
        }
    }
    assert!(run.failure.is_none(), "{:?}", run.failure);
    assert!(in_cs > 300 && over_him, "{in_cs} cutscene frames, the arrow over him before: {over_him}");
}
