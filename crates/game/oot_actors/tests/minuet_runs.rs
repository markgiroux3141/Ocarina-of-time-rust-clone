//! GAME-13 milestone 2's runs: the Lost Woods as an adult (`Route::LostWoodsAdult`, on `hookshot`),
//! Sheik's Minuet at the meadow's top (`Route::Minuet`, a debug start on `hookshot`), the Minuet's
//! warp (`Route::MinuetWarp`) and the hookshot up to the Forest Temple (`Route::ForestTemple`, a
//! debug start on `minuet`).
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

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn run(a: &Arc<GameAssets>, route: Route, mut watch: impl FnMut(&PlayState)) -> (PlayState, Playthrough) {
    let e = a.scenes.entrance_index(route.entrance()).unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), route.save(e)).unwrap();
    w.audio.log = Some(Default::default());
    if route.needs_audio() {
        w.audio_side = Some(Box::new(oot_game::audio::offline::OfflineAudio::new(&a.pack.audio_data().unwrap(), false)));
    }
    route.debug_start(&mut w);
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
        watch(&w);
    }
    eprintln!("steps {:?}", run.steps);
    eprintln!("texts {:x?}", run.texts);
    eprintln!("at {:?} scene {:#x} action {:?} state1 {:#x} cs {}", w.player().actor.world_pos, w.scene_id, w.player().action, w.player().state1, w.player().cs_mode);
    assert!(run.failure.is_none(), "{:?}", run.failure);
    (w, run)
}

fn steps(run: &Playthrough) -> Vec<Step> {
    run.steps.iter().map(|s| s.0).collect()
}

/// `SCENE_SACRED_FOREST_MEADOW`, `SCENE_FOREST_TEMPLE` (`scene_table.h`: 0x56, 0x03).
const SCENE_SACRED_FOREST_MEADOW: u16 = 0x56;
const SCENE_FOREST_TEMPLE: u16 = 0x03;

#[test]
fn adult_mido_lets_link_by_for_sarias_song() {
    let Some(a) = assets() else { return };
    let (w, run) = run(&a, Route::LostWoodsAdult, |_| {});
    assert_eq!(steps(&run), vec![Step::OcarinaOut, Step::MidoSong, Step::MidoPast, Step::MeadowEntered]);
    // EnMd_ListenToOcarina: Saria's Song (OCARINA_MODE_03) offers 0x1067; read through,
    // EnMd_BlockPath sets EVENTCHKINF_0A in the Lost Woods.
    assert!(run.texts.contains(&0x1067), "{:x?}", run.texts);
    assert!(w.save.get_event_chk_inf(0x0A));
    assert_eq!(w.scene_id, SCENE_SACRED_FOREST_MEADOW);
}

#[test]
fn sheik_teaches_the_minuet_at_the_meadows_top() {
    let Some(a) = assets() else { return };
    let mut fell = false;
    let (w, run) = run(&a, Route::Minuet, |w| {
        fell |= w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<oot_actors::en_xc::EnXc>(h)).any(|x| x.action == oot_actors::en_xc::SHEIK_ACTION_GRACEFUL_FALL);
    });
    assert_eq!(steps(&run), vec![Step::MinuetLesson, Step::MinuetPlayed, Step::MinuetLearned]);
    // EnXc_MinuetCS: EVENTCHKINF_50 and the song (QUEST_SONG_MINUET); her fall on cue 2.
    assert!(fell);
    assert!(w.save.get_event_chk_inf(0x50));
    assert!(w.save.check_quest_item(oot_game::item::QUEST_SONG_MINUET));
}

#[test]
fn the_minuet_warps_to_the_meadow() {
    let Some(a) = assets() else { return };
    let (w, run) = run(&a, Route::MinuetWarp, |_| {});
    assert_eq!(steps(&run), vec![Step::OcarinaOut, Step::MinuetWarpPlayed, Step::MinuetWarpOut, Step::MinuetWarped]);
    // sWarpSongEntrances[0]: ENTR_SACRED_FOREST_MEADOW_2, spawn 2 on the pad (10, 500, -2610).
    assert_eq!(w.scene_id, SCENE_SACRED_FOREST_MEADOW);
    let p = w.player().actor.world_pos;
    assert!((p.x - 10.0).abs() < 40.0 && (p.z + 2610.0).abs() < 40.0, "{p}");
}

#[test]
fn the_hookshot_up_to_the_forest_temple() {
    let Some(a) = assets() else { return };
    let mut cam = false;
    let (w, run) = run(&a, Route::ForestTemple, |w| cam |= w.game_camera.mode == oot_game::camera::CAM_MODE_HOOKSHOT_FLY);
    assert_eq!(
        steps(&run),
        vec![Step::HookshotRaised, Step::HookshotPulled, Step::HookshotLanded, Step::OnTheLedge, Step::ForestTempleEntered]
    );
    assert!(cam);
    assert_eq!(w.scene_id, SCENE_FOREST_TEMPLE);
}

/// The meadow's maze at patrol 0's corridor (300, 0, 1475), as an adult or a child, 30 frames on:
/// the main camera's setting.
fn maze_camera(a: &Arc<GameAssets>, adult: bool) -> i16 {
    use oot_game::save::SaveContext;
    let e = a.scenes.entrance_index("ENTR_SACRED_FOREST_MEADOW_0").unwrap();
    let mut save = SaveContext::new(e, adult, oot_game::env::clock_time(10, 0) as u16);
    if adult {
        save.apply_preset("hookshot").unwrap();
    }
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap();
    w.place_player(glam::Vec3::new(300.0, 0.0, 1475.0), -0x4000);
    for _ in 0..30 {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
    w.game_camera.setting
}

#[test]
fn the_maze_looks_down_on_a_child_only() {
    use oot_game::camera::CAM_SET_MEADOW_BIRDS_EYE;
    let Some(a) = assets() else { return };
    // The maze's floor asks for bg camera 5 (CAM_SET_MEADOW_BIRDS_EYE); Camera_RequestSettingImpl
    // refuses it (-5) to an adult in SCENE_SACRED_FOREST_MEADOW.
    assert_eq!(maze_camera(&a, false), CAM_SET_MEADOW_BIRDS_EYE);
    assert_ne!(maze_camera(&a, true), CAM_SET_MEADOW_BIRDS_EYE);
}
