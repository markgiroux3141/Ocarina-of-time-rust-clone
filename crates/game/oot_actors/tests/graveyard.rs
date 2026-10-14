//! GAME-07 milestone 6's exit (`Route::Graveyard`): the graveyard at 18:30 with Zelda's Lullaby, a
//! dig with Dampé, the lullaby at the royal tomb, the tomb's Keese and door, past the ReDeads to
//! the wall, the Sun's Song taught and its chain back, and the song played at the ReDeads; and the
//! milestone's presets.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use oot_actors::en_rd::EnRd;
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::play::scripted_input;
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// `save.h`'s flags the run sets.
const EVENTCHKINF_1D: u16 = 0x1D;
const EVENTCHKINF_5A: u16 = 0x5A;
const EVENTCHKINF_B6: u16 = 0xB6;
const INFTABLE_D9: u16 = 0xD9;
/// `QUEST_SONG_SUN` (`item.h`: 0x0F), `Item_Give(ITEM_SONG_SUN)`'s bit.
const QUEST_SONG_SUN: u32 = 0x0F;
/// `ENTR_ROYAL_FAMILYS_TOMB_1` (`entrance_table.h`: 0x574).
const ENTR_ROYAL_FAMILYS_TOMB_1: u16 = 0x574;

#[test]
fn exit_a_dig_the_royal_tomb_opened_and_the_suns_song_taught() {
    let Some(a) = assets() else { return };
    let route = Route::Graveyard;
    let e = a.scenes.entrance_index(route.entrance()).unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), route.save(e)).unwrap();
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
    eprintln!("rupees {} drop {:?}", w.save.rupees, run.drop);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    use Step::*;
    assert_eq!(
        steps,
        vec![
            Dug, DugUp, OcarinaOut, LullabyPlayed, TombOpened, TombEntered, Blocked, KeeseKilled, Blocked, KeeseKilled, Blocked, KeeseKilled, Blocked, KeeseKilled, DoorSteps, DoorSteps, DoorSteps,
            DoorOpened, DoorSteps, SunsSongLesson, SunsSongPlayed, SunsSongTaught, SunsSong, ReDeadsStunned
        ]
    );
    // Dampé's 0x5018 running on into 0x5019's choice; the lullaby (the free staff 0x86E, "You
    // played" 0x86F, the song 0x89B); the wall's 0x5021 (its init's text, EnOkarinaTag_Init type
    // 5), the lesson's 0x5009 and 0x500B, the song shown (0x879, TEACH) and played back (0x86D,
    // PLAYBACK), 0x86F and 0x89C; the graveyard's part 2 (0xD3, "You've learned the Sun's Song")
    // and the tomb's part 3 (0x500A); the song again at the ReDeads (0x86E, 0x86F, 0x89C).
    assert_eq!(run.texts, vec![0x5018, 0x5019, 0x86E, 0x86F, 0x89B, 0x5021, 0x5009, 0x500B, 0x879, 0x86D, 0x86F, 0x89C, 0xD3, 0x500A, 0x86E, 0x86F, 0x89C]);
    // 20 rupees, 10 paid to Dampé, then his prize and the Keese's drops (the run's Rand draws).
    assert_eq!(w.save.rupees, 33);
    // The graveyard's intro played (sEntranceCutsceneTable: EVENTCHKINF_B6); Dampé was paid
    // (EnTk_UpdateTalkState: INFTABLE_D9 with 0x5019's yes); the tomb opened (EVENTCHKINF_1D, by
    // En_Okarina_Tag and the gravestone's cue); the Sun's Song given (the playback's Item_Give)
    // and its part 3 seen (ENTR_ROYAL_FAMILYS_TOMB_1's EVENTCHKINF_5A).
    for f in [EVENTCHKINF_B6, EVENTCHKINF_1D, EVENTCHKINF_5A] {
        assert!(w.save.get_event_chk_inf(f), "EVENTCHKINF_{f:X}");
    }
    assert!(w.save.get_inf_table(INFTABLE_D9));
    assert!(w.save.check_quest_item(QUEST_SONG_SUN));
    assert_eq!(w.save.entrance_index, ENTR_ROYAL_FAMILYS_TOMB_1);
    // The ReDeads: all three stunned by the song (EnRd_SetupStunned: 600 frames).
    let rd: Vec<&EnRd> = w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<EnRd>(h)).collect();
    assert_eq!(rd.len(), 3);
    assert!(rd.iter().all(|r| r.stunned_by_suns_song && r.suns_song_stun_timer > 0));
}

#[test]
fn the_presets() {
    // graveyard-lullaby: the run's start (milestone 5's end): the lullaby, the letter shown, the
    // graveyard not yet seen. graveyard-sun: the run's end.
    let mut s = SaveContext::new(0, false, 0);
    s.apply_preset("graveyard-lullaby").unwrap();
    assert!(s.check_quest_item(0x0C)); // QUEST_SONG_LULLABY
    assert!(!s.check_quest_item(QUEST_SONG_SUN));
    assert!(s.get_inf_table(0x76) && s.get_event_chk_inf(0xA3) && !s.get_event_chk_inf(EVENTCHKINF_B6));
    assert_eq!(s.rupees, 20);
    let mut s = SaveContext::new(0, false, 0);
    s.apply_preset("graveyard-sun").unwrap();
    assert!(s.check_quest_item(QUEST_SONG_SUN));
    for f in [EVENTCHKINF_B6, EVENTCHKINF_1D, EVENTCHKINF_5A] {
        assert!(s.get_event_chk_inf(f));
    }
}
