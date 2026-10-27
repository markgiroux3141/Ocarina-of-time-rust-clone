//! GAME-13 milestone 1's runs: Dampé's race's start (`Route::DampeRace`, on `epona-won`) and the
//! hookshot (`Route::Hookshot`, on the `hookshot` preset).
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
use glam::Vec3;

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
    eprintln!("at {:?} scene {:#x} action {:?} state1 {:#x} cs {}", w.player().actor.world_pos, w.scene_id, w.player().action, w.player().state1, w.player().cs_mode);
    assert!(run.failure.is_none(), "{:?}", run.failure);
    (w, run)
}

fn steps(run: &Playthrough) -> Vec<Step> {
    run.steps.iter().map(|s| s.0).collect()
}

#[test]
fn the_hookshot_to_goron_citys_post() {
    let Some(a) = assets() else { return };
    let mut cam = false;
    let (w, run) = run(&a, Route::Hookshot, |w| cam |= w.game_camera.mode == oot_game::camera::CAM_MODE_HOOKSHOT_FLY);
    assert_eq!(steps(&run), vec![Step::HookshotRaised, Step::HookshotPulled, Step::HookshotLanded, Step::PostClimbed]);
    // Camera_Special5 while he flies.
    assert!(cam);
    // On the post's top (1161, 580, -148), 100 up from the ledge he stood on (961, 480).
    let p = w.player().actor.world_pos;
    assert!((p - Vec3::new(1161.0, 580.0, -148.0)).length() < 30.0, "{p}");
}

#[test]
fn dampes_race_starts() {
    let Some(a) = assets() else { return };
    let (w, run) = run(&a, Route::DampeRace, |_| {});
    assert_eq!(steps(&run), vec![Step::DampeRaceStarted, Step::DampeFirstDoor]);
    // EnPoRelay_Talk: the race on as the greeting closes, the timer counting up
    // (TIMER_STATE_UP_*); at his eighth point the first door's switch (0x35).
    assert_eq!(run.texts.first(), Some(&0x5041));
    assert!(w.save.timer_state >= oot_game::save::TIMER_STATE_UP_INIT, "{}", w.save.timer_state);
    assert!(w.flags.get_switch(0x35));
}
