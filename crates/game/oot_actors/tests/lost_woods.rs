//! GAME-08 milestone 2's exit run (`Route::LostWoods`): from the Lost Woods' entrance with Saria's
//! Song and the slingshot (`meadow-saria`), Saria's Song to the Skull Kid on his stump in room 9
//! (a piece of heart), the target's three bullseyes in room 1 (the bigger seed bag), and the
//! memory game on the stump in room 1's hollow, its three rounds won (a green rupee, a blue rupee,
//! a piece of heart).
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::item::{QUEST_HEART_PIECE_COUNT, UPG_BULLET_BAG};
use oot_game::play::scripted_input;
use oot_game::play_scene::GameAssets;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

#[test]
fn the_skull_kids_and_the_target_scrub() {
    let Some(a) = assets() else { return };
    let route = Route::LostWoods;
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
    // The memory game: each round starts at three notes (AudioOcarina_MemoryGameInit) and ends
    // at sOcaMemoryGameNumNotes' 5, 6 and 8: three turns, four and six.
    let mut want = vec![OcarinaOut, SkullKidSong, SkullKidHeart, SlingshotDrawn, Bullseye, Bullseye, Bullseye, BulletBag, MemoryGame];
    want.extend([MemoryRound; 13]);
    want.push(MemoryGameWon);
    assert_eq!(steps, want);
    // Room 9: "You played Saria's Song" (0x899), the Skull Kid's 0x10BB, the piece of heart's
    // text. Room 1: the target scrub's 0x10AF; the stump's 0x10BE, the box's "Play using [A] and
    // [C]" (0x86D), each won round's 0x10BF and the offer of another (0x102E).
    for t in [0x899, 0x10BB, 0x10AF, 0x10BE, 0x86D, 0x10BF, 0x102E] {
        assert!(run.texts.contains(&t), "text {t:#x}: {:x?}", run.texts);
    }
    let s = &w.save;
    // ITEMGETINF_16 (the stump's heart), ITEMGETINF_1D (the seed bag), ITEMGETINF_17 (the game).
    for f in [0x16, 0x1D, 0x17] {
        assert!(s.get_item_get_inf(f), "ITEMGETINF_{f:X}");
    }
    // Two pieces of heart (questItems' top bits), the 40 seed bag (upgrade 2) full.
    assert_eq!((s.inventory.quest_items >> QUEST_HEART_PIECE_COUNT) - (start.inventory.quest_items >> QUEST_HEART_PIECE_COUNT), 2);
    assert_eq!(s.cur_upg_value(UPG_BULLET_BAG), 2);
    assert_eq!(s.ammo(oot_game::item::ITEM_SLINGSHOT), 40);
    // sOcarinaGameRewards: a green rupee (1) and a blue one (5) on the way; three rounds counted.
    assert_eq!(s.ocarina_game_round_num, 3);
    assert_eq!(s.rupees + s.rupee_accumulator, start.rupees + 6);
    assert_eq!(s.health, start.health);
}
