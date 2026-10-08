//! GAME-05 milestone 5b-1's exit (`Route::Pause`, the `pause` script's run, also a golden trace
//! with screenshots): inside the Deku Tree with the Fairy Slingshot owned but on no button, Start
//! opens the pause menu, the item page's cursor goes to the slingshot, C-Right equips it (its icon
//! flying to the button), R turns the menu to the map page, and Start closes it: the slingshot is
//! on C-Right.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use oot_actors::playthrough::{PAUSE_C_BUTTON, PAUSE_SLOT, Playthrough, Route, Step};
use oot_game::item::{ITEM_DEKU_NUT, ITEM_DEKU_STICK, ITEM_NONE, ITEM_SLINGSHOT};
use oot_game::kaleido::*;
use oot_game::play::scripted_input;
use oot_game::play_scene::GameAssets;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

#[test]
fn exit_the_slingshot_onto_c_right_from_the_pause_menu() {
    let Some(a) = assets() else { return };
    let route = Route::Pause;
    let e = a.scenes.entrance_index(route.entrance()).expect("entrance");
    let save = route.save(e);
    assert_eq!(&save.equips.button_items[1..], &[ITEM_DEKU_STICK, ITEM_DEKU_NUT, ITEM_NONE]);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = oot_game::play::PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    route.debug_start(&mut w);
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    let mut rates = Vec::new();
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
        rates.push(w.r_update_rate);
        let pc = &w.pause_ctx;
        match run.take_done() {
            Some(Step::MenuOpened) => {
                // The item page, the menu at 30 frames a second.
                assert_eq!((pc.page_index, w.r_update_rate), (PAUSE_ITEM, 2));
            }
            Some(Step::CursorOnItem) => {
                assert_eq!((pc.cursor_point[0] as u16, pc.cursor_item[0]), (PAUSE_SLOT, ITEM_SLINGSHOT as u16));
            }
            Some(Step::ItemEquipped) => {
                // KaleidoScope_UpdateItemEquip's end: the icon at C-Right's place (sCButtonPosX/Y).
                assert_eq!((pc.equip_anim_x, pc.equip_anim_y), (1140, 1100));
                assert_eq!(w.save.equips.button_items[PAUSE_C_BUTTON + 1], ITEM_SLINGSHOT);
            }
            Some(Step::PageTurned) => {
                assert_eq!(pc.page_index, PAUSE_MAP);
            }
            Some(Step::MenuClosed) => {
                assert_eq!((pc.state, w.r_update_rate), (PAUSE_STATE_OFF, 3));
            }
            _ => {}
        }
    }
    assert!(run.failure.is_none(), "{:?}", run.failure);
    let names: Vec<_> = run.steps.iter().map(|(s, _)| s.name()).collect();
    assert_eq!(names, ["menu_opened", "cursor_on_item", "item_equipped", "page_turned", "menu_closed"]);
    assert_eq!(&w.save.equips.button_items[1..], &[ITEM_DEKU_STICK, ITEM_DEKU_NUT, ITEM_SLINGSHOT]);
    assert_eq!(w.save.equips.c_button_slots[PAUSE_C_BUTTON], PAUSE_SLOT as u8);
    println!("steps: {:?}", run.steps.iter().map(|(s, f)| (s.name(), f)).collect::<Vec<_>>());
}
