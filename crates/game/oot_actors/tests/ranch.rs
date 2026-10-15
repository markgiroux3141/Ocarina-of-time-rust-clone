//! GAME-08 milestone 3's run (`Route::Ranch`): from Lon Lon Ranch's gate with the Lost Woods' games
//! won (`lost-woods-games`), the ranch's intro, Malon's three talks and her lesson (Epona's Song
//! played back), the song's cutscene (`gLonLonRanchEponasSongCs`, layer 5) and back to the ranch
//! by its terminator (destination 94, `ENTR_LON_LON_RANCH_1`); the foal comes to Link, goes home
//! after her 300 frames, and comes again when he plays the song (step (a)). Then into Talon's
//! house, the Super Cucco game won and its bottle of milk (step (b)).
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
    let mut want = vec![Malon, Malon, Malon, OcarinaOut, EponasSongPlayed, EponasSongCs, RanchFromSong, FoalCame, FoalHome, EponasSong, FoalCalled, TalonsHouse, CuccoGame];
    want.extend([SuperCucco; 3]);
    want.extend([CuccoGameOver, MilkBottle]);
    assert_eq!(steps, want);
    // EnMa1_GetTextId at the ranch after Talon's return: 0x2047, 0x2048, 0x2049; the ocarina out by
    // her, 0x2061 (EnMa1_IdleTeachSong). The cutscene's TEXT_LIST: 0xD2 (its frames 10 to 60).
    // Talon (EnTa_SetTextForTalkInLonLonHouse, the game's): 0x207E, 0x2080, 0x2082 (two left),
    // 0x2083 (one), 0x2084 (won); back in the house 0x2086, 0x2087, and the bottle's 0x98
    // (sGetItemTable's GI_BOTTLE_MILK_FULL).
    for t in [0x2047, 0x2048, 0x2049, 0x2061, 0xD2, 0x207E, 0x2080, 0x2082, 0x2083, 0x2084, 0x2086, 0x2087, 0x98] {
        assert!(run.texts.contains(&t), "text {t:#x}: {:x?}", run.texts);
    }
    let s = &w.save;
    // EnMa1_UpdateTalkState's flags as the texts close (save.h: EVENTCHKINF_TALKED_TO_CHILD_MALON_AT_RANCH
    // 0x15, EVENTCHKINF_CAN_LEARN_EPONAS_SONG 0x16, INFTABLE_TOLD_EPONA_IS_SCARED 0x85).
    assert!(s.get_event_chk_inf(0x15) && s.get_event_chk_inf(0x16) && s.get_inf_table(0x85));
    // The playback's success gives the song.
    assert!(s.check_quest_item(QUEST_SONG_EPONA));
    // The game's end: ENTR_LON_LON_BUILDINGS_2 (by Talon).
    assert_eq!(s.entrance_index, a.scenes.entrance_index("ENTR_LON_LON_BUILDINGS_2").unwrap());
    // ITEMGETINF_TALON_BOTTLE (0x02), INFTABLE_TALKED_TO_TALON_IN_RANCH_HOUSE (0x7E); the game's
    // flags cleared again (EVENTINF_CUCCO_GAME_WON by EnTa_SetTextForTalkInLonLonHouse,
    // _FINISHED by the init); the bottle of milk (ITEM_BOTTLE_MILK_FULL 0x1A) in the first
    // bottle's slot (SLOT_BOTTLE_1 0x12); 10 rupees paid; the timer off.
    assert!(s.get_item_get_inf(0x02) && s.get_inf_table(0x7E));
    // EVENTINF_CUCCO_GAME_WON (EVENTINF_HORSES_08), _FINISHED (EVENTINF_HORSES_0A).
    assert!(!s.get_event_inf(0x08) && !s.get_event_inf(0x0A));
    assert_eq!(s.inventory.items[0x12], 0x1A);
    assert_eq!(s.rupees + s.rupee_accumulator, start.rupees - 10);
    assert_eq!(s.timer_state, oot_game::save::TIMER_STATE_OFF);
    // The run's end is the `ranch-epona` preset (Phase 9's end, Phase 10's start): the same flags,
    // inventory and rupees.
    let mut p = oot_game::save::SaveContext::new(e, false, route.day_time());
    p.apply_preset("ranch-epona").unwrap();
    assert_eq!((&p.event_chk_inf, &p.inf_table, &p.item_get_inf, &p.event_inf), (&s.event_chk_inf, &s.inf_table, &s.item_get_inf, &s.event_inf));
    assert_eq!((&p.inventory.items, &p.inventory.ammo, p.inventory.upgrades, p.inventory.quest_items), (&s.inventory.items, &s.inventory.ammo, s.inventory.upgrades, s.inventory.quest_items));
    assert_eq!((p.rupees, p.ocarina_game_round_num, &p.equips.button_items), (s.rupees + s.rupee_accumulator, s.ocarina_game_round_num, &s.equips.button_items));
    for (i, (a, b)) in p.scene_flags.iter().zip(s.scene_flags.iter()).enumerate() {
        assert_eq!(format!("{a:?}"), format!("{b:?}"), "scene {i:#x}'s flags");
    }
    // The ranch's intro seen (sEntranceCutsceneTable's EVENTCHKINF_B4).
    assert!(s.get_event_chk_inf(0xB4));
    assert_eq!(s.health, start.health);
}
