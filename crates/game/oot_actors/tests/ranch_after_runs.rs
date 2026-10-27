//! GAME-12 milestone 4's runs: the ranch after Epona (Malon's obstacle course offered and started,
//! its record, Malon singing by night before Epona) and Hyrule Field's Big Poe.
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
    // Epona's Song: AudioOcarina_Update runs in Audio_Update, on the audio side.
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
    eprintln!("at {:?} scene {:#x} layer {} entrance {:#x}", w.player().actor.world_pos, w.scene_id, w.save.scene_layer, w.save.entrance_index);
    assert!(run.failure.is_none(), "{:?}", run.failure);
    (w, run)
}

fn steps(run: &Playthrough) -> Vec<Step> {
    run.steps.iter().map(|s| s.0).collect()
}

#[test]
fn malon_course() {
    let Some(a) = assets() else { return };
    let (w, run) = run(&a, Route::MalonCourse, |_| {});
    assert_eq!(steps(&run), vec![Step::MalonThanked, Step::OcarinaOut, Step::EponaSongPlayed, Step::EponaCame, Step::CourseTaken, Step::CourseStarted]);
    // EnMa3_GetTextId: 0x2000 the first time (INFTABLE_B8); with Epona near, 0x2002 running on into
    // 0x2003, "Let's go" (0), and with no high score 0x2092 (EnMa3_UpdateTalkState's choice).
    // Between them Epona's Song's three.
    assert_eq!(run.texts.first(), Some(&0x2000));
    assert_eq!(run.texts[run.texts.len() - 3..], [0x2002, 0x2003, 0x2092]);
    assert!(w.save.get_inf_table(0xB8) && w.save.get_inf_table(0xB9));
    // The event: ENTR_LON_LON_RANCH_0 with CS_INDEX_0 (layer 4), EVENTINF_HORSES_0A, the timer.
    assert_eq!(w.save.scene_layer, 4);
    assert!(w.save.get_event_inf(0x0A));
    // Horse_SetupInCutscene: Link on Epona (the race word cleared by Ingo's init at spawn 0).
    assert_ne!(w.player().state1 & oot_actors::player::STATE1_23, 0);
}

#[test]
fn malon_record() {
    let Some(a) = assets() else { return };
    let (w, run) = run(&a, Route::MalonRecord, |_| {});
    assert_eq!(steps(&run), vec![Step::CourseRecord]);
    // EnMa3_GetTextId with EVENTINF_HORSES_0A: 45 seconds, under 50 without the cow: 0x208F; its
    // closing sets the cow's flag, keeps 45 (from the 180 an empty score is made), clears the
    // course's bit and stops the timer.
    assert_eq!(run.texts, vec![0x208F]);
    assert!(w.save.get_event_chk_inf(0x1E));
    assert_eq!(w.save.high_scores[3], 45);
    assert!(!w.save.get_event_inf(0x0A));
}

#[test]
fn malon_song() {
    let Some(a) = assets() else { return };
    let (w, run) = run(&a, Route::MalonSong, |_| {});
    assert_eq!(steps(&run), vec![Step::OcarinaOut, Step::EponaSongPlayed, Step::MalonSang]);
    // EnMa2_WaitForEponasSong: INFTABLE_PLAYED_SONG_FOR_ADULT_MALON; EnMa2_ForceTalkAfterSong:
    // 0x2051 (running on into 0x2052), its closing INFTABLE_TALKED_TO_ADULT_MALON_AFTER_SONG.
    assert!(w.save.get_inf_table(0x8E) && w.save.get_inf_table(0x8C));
    assert_eq!(run.texts[run.texts.len() - 2..], [0x2051, 0x2052]);
}

#[test]
fn big_poe() {
    let Some(a) = assets() else { return };
    use oot_actors::en_po_field::*;
    // The live instance's place index and where the Big Poe came out, and Link then.
    let mut out: Option<(u8, [f32; 2], [f32; 2])> = None;
    let (w, run) = run(&a, Route::BigPoe, |w| {
        if out.is_none() {
            if let Some(p) = w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<EnPoField>(h)).find(|p| !p.actor.killed && p.action == Action::Appear) {
                let l = w.player().actor.world_pos;
                out = Some((p.spawn_flag_index, [p.actor.world_pos.x, p.actor.world_pos.z], [l.x, l.z]));
            }
        }
    });
    assert_eq!(steps(&run), vec![Step::OcarinaOut, Step::EponaSongPlayed, Step::Mounted, Step::BigPoeAppeared, Step::BigPoeFleeing]);
    let (i, poe, link) = out.expect("a Poe out");
    eprintln!("place index {i}, the Poe at {poe:?}, Link at {link:?}");
    // EnPoField_WaitForSpawn: Link within 150 on x and z of (-1933, 5455), riding, switch 0x17
    // clear: a Big Poe (no roll), 480 ahead of him (then moved at its speed 12 the same frame:
    // EnPoField_Update's Actor_MoveXZGravity in EnPoField_Appear). sSpawn* run backwards over the placements
    // (each init's actor at its category's head): 0x17 is index 8.
    assert_eq!(i, 8);
    assert!((link[0] + 1933.0).abs() < 150.0 && (link[1] - 5455.0).abs() < 150.0, "{link:?}");
    assert!(((poe[0] - link[0]).hypot(poe[1] - link[1]) - 480.0).abs() <= 12.5, "{poe:?} {link:?}");
    assert_ne!(w.player().state1 & oot_actors::player::STATE1_23, 0);
    assert!(!w.flags.get_switch(0x17));
}
