//! GAME-10 milestone 7's runs: `Route::Escape` (Hyrule Field with the three stones: the
//! drawbridge's trigger and Zelda's escape) and `Route::OcarinaOfTime` (from the moat's edge: the
//! dive to the Ocarina of Time, the Song of Time's cutscenes through the Temple of Time, and the
//! field after: Phase 11's exit, part 2). Each run's end that a later one starts from is a preset,
//! checked here against the run.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use oot_actors::PlayExt;
use oot_actors::item_ocarina::ItemOcarina;
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Runs `route` from its start, `watch` called after each frame; the play state at its end and
/// its run.
fn run(a: &Arc<GameAssets>, route: Route, mut watch: impl FnMut(&PlayState)) -> (PlayState, Playthrough) {
    let e = a.scenes.entrance_index(route.entrance()).unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), route.save(e)).unwrap();
    w.audio.log = Some(Default::default());
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
        watch(&w);
    }
    eprintln!("steps {:?}", run.steps);
    eprintln!("texts {:x?}", run.texts);
    eprintln!("at {:?} health {} scene {:#x} layer {} entrance {:#x}", w.player().actor.world_pos, w.save.health, w.scene_id, w.save.scene_layer, w.save.entrance_index);
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
fn the_drawbridge_and_zeldas_escape() {
    let Some(a) = assets() else { return };
    // Impa's throw (`En_Viewer` type 1 spawns Item_Ocarina 0 at the escape's frame 845): where its
    // update leaves it on the frames 846 (thrown up), 881 (put at (250, 60, 1075) and thrown at
    // (1, -5, -7)), 897 (into the water: stopped) and 906 (sinking). The script's frame counts on
    // after the actors' update: after a tick `curFrame` is one past the frame they saw.
    let mut seen: Vec<(u16, glam::Vec3, glam::Vec3)> = Vec::new();
    // Ganondorf (En_Viewer type 3) spawns his cape (En_Ganon_Mant, params 35) as his child.
    let mut cape = false;
    let (mut w, run) = run(&a, Route::Escape, |w| {
        if w.save.scene_layer != 5 || w.cs_ctx.state == oot_game::cutscene::CS_STATE_IDLE {
            return;
        }
        cape |= w.actors.all().into_iter().any(|h| w.actors.downcast::<oot_actors::en_ganon_mant::EnGanonMant>(h).is_some_and(|c| c.actor.params == 35 && !c.actor.killed));
        let frame = w.cs_ctx.frames.wrapping_sub(1);
        for h in w.actors.all() {
            if let Some(o) = w.actors.downcast::<ItemOcarina>(h).filter(|o| o.actor.params == 0 && !o.actor.killed)
                && [846, 881, 897, 906].contains(&frame)
                && seen.last().is_none_or(|s| s.0 != frame)
            {
                seen.push((frame, o.actor.world_pos, o.actor.velocity));
            }
        }
    });
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::EscapeStarted, Step::Escaped]);
    eprintln!("the throw: {seen:?}");
    assert!(cape, "no En_Ganon_Mant in the escape");
    let frames: Vec<u16> = seen.iter().map(|s| s.0).collect();
    assert_eq!(frames, vec![846, 881, 897, 906]);
    // 846: spawned during Impa's update on 845, its first update: ItemOcarina_GetThrown (6 up,
    // gravity -0.3 to -5), not yet moved.
    assert_eq!((seen[0].1, seen[0].2), (glam::Vec3::new(4.0, 81.0, 2600.0), glam::Vec3::new(0.0, 6.0, 0.0)));
    // 881: put at (250, 60, 1075) after its move, thrown at (1, -5, -7).
    assert_eq!((seen[1].1, seen[1].2), (glam::Vec3::new(250.0, 60.0, 1075.0), glam::Vec3::new(1.0, -5.0, -7.0)));
    // 897: 16 moves of (1, -5, -7) at 1.5 on: (274, -60, 907), the C's own second ripple's spot,
    // in the moat's surface; stopped.
    assert_eq!((seen[2].1, seen[2].2), (glam::Vec3::new(274.0, -60.0, 907.0), glam::Vec3::ZERO));
    // 906: sinking (gravity -0.1 to -0.5).
    assert!(seen[3].1.y < seen[2].1.y && seen[3].2.y < 0.0);
    assert!(w.save.get_event_chk_inf(oot_game::save::EVENTCHKINF_ZELDA_FLED_CASTLE));
    assert_eq!(w.save.scene_layer, 1);
    ends_as_preset(&mut w, "zelda-fled", |_| {});
}

#[test]
fn the_ocarina_of_time_and_the_song_of_time() {
    let Some(a) = assets() else { return };
    let (mut w, run) = run(&a, Route::OcarinaOfTime, |_| {});
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::OcarinaTaken, Step::SongOfTimeCs, Step::TempleOfTime, Step::SongOfTimePlayed, Step::FieldAfterOcarina]);
    // The ocarina's get-item text (sGetItemTable's 0x3A), then the Temple of Time's texts round the
    // Song of Time's lesson and playback.
    assert_eq!(run.texts.first(), Some(&0x3A));
    assert!(w.save.check_quest_item(oot_game::item::QUEST_SONG_TIME));
    assert_eq!(w.save.inventory.items[oot_game::item::SLOT_OCARINA], oot_game::item::ITEM_OCARINA_OF_TIME);
    // Hyrule Field at ENTR_HYRULE_FIELD_16 (layer 1), gHyruleFieldGetOoTCs seen (EVENTCHKINF_A9).
    assert_eq!((w.scene_id, w.save.scene_layer), (0x51, 1));
    assert!(w.save.get_event_chk_inf(0xA9));
    ends_as_preset(&mut w, "ocarina-of-time", |_| {});
}
