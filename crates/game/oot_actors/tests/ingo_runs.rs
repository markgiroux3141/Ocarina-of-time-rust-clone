//! GAME-12 milestone 2's runs: Ingo's rental (`Route::IngoRental`), the race offered and taken
//! to its start (`Route::IngoRace`), and its two outcomes (`Route::IngoLost`, `Route::IngoWon`).
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use oot_actors::PlayExt;
use oot_actors::en_in::*;
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;

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
    eprintln!("at {:?} scene {:#x} layer {} entrance {:#x} rupees {} race {}", w.player().actor.world_pos, w.scene_id, w.save.scene_layer, w.save.entrance_index, w.save.rupees, race_state(&w));
    assert!(run.failure.is_none(), "{:?}", run.failure);
    (w, run)
}

fn steps(run: &Playthrough) -> Vec<Step> {
    run.steps.iter().map(|s| s.0).collect()
}

#[test]
fn ingos_rental() {
    let Some(a) = assets() else { return };
    let (w, run) = run(&a, Route::IngoRental, |_| {});
    assert_eq!(steps(&run), vec![Step::RentalPaid, Step::InCorral, Step::OcarinaOut, Step::EponaSongPlayed, Step::Mounted]);
    // 0x2030 (his first words to the adult) running on into 0x2031, "Ride": 0x2034, "Don't listen":
    // 10 Rupees and 0x205C.
    assert_eq!(&run.texts[..4], &[0x2030, 0x2031, 0x2034, 0x205C]);
    // adult-awake's 16 Rupees, less the rental's 10.
    assert_eq!(w.save.rupees + w.save.rupee_accumulator, 6);
    // EnIn_StartRentalPeriod: INGO_RACE_STATE_HORSE_RENTAL_PERIOD; in the corral the rental's Ingo
    // (params 3) sets the timer to 60 (EnIn_WaitForObject) and, Link on a horse, INFTABLE_AB.
    assert_eq!(race_state(&w), INGO_RACE_STATE_HORSE_RENTAL_PERIOD);
    assert!(w.save.get_inf_table(INFTABLE_INGO_TALKED_TO_ADULT_LINK) && w.save.get_inf_table(INFTABLE_AB));
    assert!(!w.save.get_event_chk_inf(EVENTCHKINF_1B));
    assert!(w.save.timer_seconds > 30 && w.save.timer_seconds <= 60, "{}", w.save.timer_seconds);
}

#[test]
fn ingos_race_offered_and_started() {
    let Some(a) = assets() else { return };
    let (w, run) = run(&a, Route::IngoRace, |_| {});
    assert_eq!(steps(&run), vec![Step::RentalPaid, Step::InCorral, Step::OcarinaOut, Step::EponaSongPlayed, Step::Mounted, Step::RaceTaken, Step::RaceStarted]);
    // 0x2031 (Ingo has met the adult), "Ride", 0x2034, "Don't listen", 0x205C; Epona's Song's
    // 0x086E, 0x086F, 0x089A; from horseback 0x2038, "Do it".
    assert_eq!(run.texts, vec![0x2031, 0x2034, 0x205C, 0x086E, 0x086F, 0x089A, 0x2038]);
    // ingo-rental's 60, less the rental's 10; the bet is taken only by a loss.
    assert_eq!(w.save.rupees + w.save.rupee_accumulator, 50);
    // EnIn_RentalPeriod's yes: INGO_RACE_STATE_RACING, the horse's type (Epona: 0), the race's intro
    // (ENTR_LON_LON_RANCH_0 with CS_INDEX_0: layer 4).
    assert_eq!(race_state(&w), INGO_RACE_STATE_RACING);
    assert!(!w.save.get_event_inf(EVENTINF_INGO_RACE_HORSETYPE));
    assert_eq!(w.save.scene_layer, 4);
    // EnHorseGameCheck_UpdateIngoRace: the timer from its 51st frame, Link's horse racing from its
    // 81st, Ingo's (`En_Horse` 0x8003, Ingo on it) from its 82nd.
    let horses: Vec<_> = w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<oot_actors::en_horse::EnHorse>(h)).collect();
    let epona = horses.iter().find(|h| h.ty == oot_actors::en_horse::HORSE_EPONA).unwrap();
    let ingos = horses.iter().find(|h| h.ty == oot_actors::en_horse::HORSE_HNI).unwrap();
    assert!(epona.in_race && ingos.in_race);
    assert!(ingos.rider.is_some());
    assert_ne!(w.save.timer_state, 0);
    assert_eq!(w.player().ride_actor, Some(w.actors.all().into_iter().find(|&h| w.actors.downcast::<oot_actors::en_horse::EnHorse>(h).is_some_and(|e| e.ty == oot_actors::en_horse::HORSE_EPONA)).unwrap()));
}

#[test]
fn ingo_wins_and_gloats() {
    let Some(a) = assets() else { return };
    let (w, run) = run(&a, Route::IngoLost, |_| {});
    assert_eq!(steps(&run), vec![Step::IngoGloated]);
    // EnIn_PlayerLostRace: 0x203D (the first loss), -50 Rupees, the rematch's state, LOST_ONCE.
    assert_eq!(run.texts, vec![0x203D]);
    assert_eq!(w.save.rupees + w.save.rupee_accumulator, 0);
    assert!(w.save.get_event_inf(EVENTINF_INGO_RACE_LOST_ONCE));
}

#[test]
fn ingo_loses_the_first_race() {
    let Some(a) = assets() else { return };
    let (w, run) = run(&a, Route::IngoWon, |_| {});
    assert_eq!(steps(&run), vec![Step::SecondRaceCalled]);
    // EnIn_PlayerWonFirstRace: 0x203A, then the second race (ENTR_LON_LON_RANCH_0 with CS_INDEX_0,
    // INGO_RACE_STATE_RACING, EVENTINF_INGO_RACE_SECOND_RACE).
    assert_eq!(run.texts, vec![0x203A]);
    assert_eq!(race_state(&w), INGO_RACE_STATE_RACING);
    assert!(w.save.get_event_inf(EVENTINF_INGO_RACE_SECOND_RACE));
    // The race's intro: ENTR_LON_LON_RANCH_0 with CS_INDEX_0 is the ranch's cutscene layer 4
    // (gLonLonRanchRaceIntroCs).
    assert_eq!(w.save.entrance_index, a.scenes.entrance_index("ENTR_LON_LON_RANCH_0").unwrap());
    assert_eq!(w.save.scene_layer, 4);
}
