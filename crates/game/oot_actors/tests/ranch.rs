//! GAME-08 milestone 3's run, step (a) (`Route::Ranch`): from Lon Lon Ranch's gate with the Lost
//! Woods' games won (`lost-woods-games`), the ranch's intro, Malon's three talks and her lesson
//! (Epona's Song played back), the song's cutscene (`gLonLonRanchEponasSongCs`, layer 5) and back
//! to the ranch by its terminator (destination 94, `ENTR_LON_LON_RANCH_1`); the foal comes to Link,
//! goes home after her 300 frames, and comes again when he plays the song.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::item::QUEST_SONG_EPONA;
use oot_game::play::scripted_input;
use oot_game::play_scene::GameAssets;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

#[test]
fn malons_lesson_and_the_foal() {
    let Some(a) = assets() else { return };
    let route = Route::Ranch;
    let e = a.scenes.entrance_index(route.entrance()).unwrap();
    let start = route.save(e);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), start.clone()).unwrap();
    route.debug_start(&mut w);
    w.audio_side = Some(Box::new(oot_game::audio::offline::OfflineAudio::new(&a.pack.audio_data().unwrap(), false)));
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
    }
    assert!(run.failure.is_none(), "{:?}", run.failure);
    eprintln!("steps {:?}", run.steps);
    eprintln!("texts {:x?}", run.texts);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    use Step::*;
    assert_eq!(steps, vec![Malon, Malon, Malon, OcarinaOut, EponasSongPlayed, EponasSongCs, RanchFromSong, FoalCame, FoalHome, EponasSong, FoalCalled]);
    // EnMa1_GetTextId at the ranch after Talon's return: 0x2047, 0x2048, 0x2049; the ocarina out by
    // her, 0x2061 (EnMa1_IdleTeachSong). The cutscene's TEXT_LIST: 0xD2 (its frames 10 to 60).
    for t in [0x2047, 0x2048, 0x2049, 0x2061, 0xD2] {
        assert!(run.texts.contains(&t), "text {t:#x}: {:x?}", run.texts);
    }
    let s = &w.save;
    // EnMa1_UpdateTalkState's flags as the texts close (save.h: EVENTCHKINF_TALKED_TO_CHILD_MALON_AT_RANCH
    // 0x15, EVENTCHKINF_CAN_LEARN_EPONAS_SONG 0x16, INFTABLE_TOLD_EPONA_IS_SCARED 0x85).
    assert!(s.get_event_chk_inf(0x15) && s.get_event_chk_inf(0x16) && s.get_inf_table(0x85));
    // The playback's success gives the song.
    assert!(s.check_quest_item(QUEST_SONG_EPONA));
    // Destination 94: ENTR_LON_LON_RANCH_1.
    assert_eq!(s.entrance_index, a.scenes.entrance_index("ENTR_LON_LON_RANCH_1").unwrap());
    // The ranch's intro seen (sEntranceCutsceneTable's EVENTCHKINF_B4).
    assert!(s.get_event_chk_inf(0xB4));
    assert_eq!(s.health, start.health);
}
