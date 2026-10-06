//! GAME-05 milestone 4a's exit (`Route::Shutter`, the `shutter` script's run, also a golden
//! trace): inside the Deku Tree, from a debug start on room 0's top floor, Link steps on the
//! floor switch (flag 0x27): the web over room 10's door burns and the golden torches light,
//! with the attention cameras. He opens room 10's sliding door and walks through; room 10
//! loads, room 0 goes, and the door slams and bars behind him (room 10's enemies are alive),
//! holding him a moment.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use oot_actors::PlayExt;
use oot_actors::door_shutter::{Action as DoorAction, DoorShutter};
use oot_actors::playthrough::{Playthrough, Route, SHUTTER_DOOR, SHUTTER_SWITCH_FLAG, SHUTTER_WEB_HOME, Step};
use oot_game::audio::sfx::*;
use oot_game::play::scripted_input;
use oot_game::play_scene::GameAssets;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

#[test]
fn exit_the_switch_burns_the_web_and_room_10s_door_bars_behind_link() {
    let Some(a) = assets() else { return };
    let route = Route::Shutter;
    let e = a.scenes.entrance_index(route.entrance()).expect("entrance");
    let save = route.save(e);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = oot_game::play::PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    route.debug_start(&mut w);
    assert_eq!(w.room_ctx.cur.num, 0);
    let door = |w: &oot_game::play::PlayState| {
        w.actors.all().into_iter().find_map(|h| w.actors.downcast::<DoorShutter>(h).filter(|d| d.transition_index() == SHUTTER_DOOR)).map(|d| (d.action, d.actor.room, d.bars_closed_amount))
    };
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    let mut slammed = None;
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
        if slammed.is_none() && w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == w.audio.frames && s == NA_SE_EV_STONE_BOUND) {
            slammed = Some(w.audio.frames);
        }
        match run.take_done() {
            Some(Step::SwitchPressed) => assert!(w.flags.get_switch(SHUTTER_SWITCH_FLAG)),
            Some(Step::WebBurnt) => {
                // Bg_Ydan_Sp 0x19CA's burn flag: gone, its destroyed flag 0x0A set.
                assert!(w.flags.get_switch(0x0A));
                assert!(!w.actors.all().into_iter().any(|h| w.actors.downcast::<oot_actors::bg_ydan_sp::BgYdanSp>(h).is_some_and(|s| s.actor.world_pos.distance(SHUTTER_WEB_HOME) < 1.0)));
                // From room 0 (its front room), the door is a plain one, waiting for Link.
                assert_eq!(door(&w), Some((DoorAction::Idle, 0, 0.0)));
            }
            Some(Step::DoorOpened) => {
                // Room 10 requested behind the door; the door is room 10's now.
                assert_eq!(w.room_ctx.cur.num, 10);
                assert_eq!(door(&w).map(|d| d.1), Some(10));
            }
            Some(Step::DoorBarred) => {
                // Room 0 gone; from room 10 the door's type 1 is barred until it's cleared.
                assert_eq!((w.room_ctx.cur.num, w.room_ctx.prev.num), (10, -1));
                assert_eq!(door(&w), Some((DoorAction::WaitClear, 10, 1.0)));
            }
            _ => {}
        }
    }
    assert!(run.failure.is_none(), "{:?}", run.failure);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::SwitchPressed, Step::WebBurnt, Step::DoorOpened, Step::DoorBarred]);
    assert!(slammed.is_some(), "the door never slammed");
    eprintln!("steps {:?}; slammed at {slammed:?}; Link at {:?}", run.steps, w.player().actor.world_pos);
    // Link stands in room 10, past the door.
    assert!(w.player().actor.world_pos.x < -600.0);
}
