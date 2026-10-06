//! GAME-05 milestone 4b's exit (`Route::Stick`, the `stick` script's run, also a golden trace):
//! inside the Deku Tree, from a debug start by room 0's middle-floor golden torch (ten Deku
//! Sticks on C-Left, the torches lit by flag 0x27), Link takes a stick out, lights it at the
//! torch, runs round the floor and jumps its gap, burns the web over room 1's door with it,
//! and goes through the door into room 1.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use oot_actors::PlayExt;
use oot_actors::bg_ydan_sp::BgYdanSp;
use oot_actors::player::PLAYER_IA_DEKU_STICK;
use oot_actors::playthrough::{Playthrough, Route, STICK_WEB_HOME, Step};
use oot_game::audio::sfx::*;
use oot_game::item::ITEM_DEKU_STICK;
use oot_game::play::scripted_input;
use oot_game::play_scene::GameAssets;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

#[test]
fn exit_a_stick_lit_at_the_torch_burns_the_web_over_room_1s_door() {
    let Some(a) = assets() else { return };
    let route = Route::Stick;
    let e = a.scenes.entrance_index(route.entrance()).expect("entrance");
    let save = route.save(e);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = oot_game::play::PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    route.debug_start(&mut w);
    assert_eq!(w.room_ctx.cur.num, 0);
    let web = |w: &oot_game::play::PlayState| w.actors.all().into_iter().any(|h| w.actors.downcast::<BgYdanSp>(h).is_some_and(|s| s.actor.home_pos.distance(STICK_WEB_HOME) < 1.0));
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    let mut jumped = false;
    let mut lit_at = None;
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
        jumped |= w.player().action == oot_actors::player::Action::Midair;
        match run.take_done() {
            Some(Step::StickOut) => {
                let p = w.player();
                assert_eq!((p.held_item_ap, p.unk_85c), (PLAYER_IA_DEKU_STICK, 1.0));
            }
            Some(Step::StickLit) => {
                // Obj_Syokudai: unk_860 210 and NA_SE_EV_FLAME_IGNITION (the step is seen the
                // frame after; Navi's hint by the torch holds Link there, the torch keeping the
                // stick at 200 or more).
                assert!(w.player().unk_860 >= 200);
                assert!(w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f + 2 >= w.audio.frames && s == NA_SE_EV_FLAME_IGNITION));
                lit_at = Some(w.audio.frames);
            }
            Some(Step::WebBurnt) => assert!(!web(&w)),
            Some(Step::DoorOpened) => assert_eq!(w.room_ctx.cur.num, 1),
            Some(Step::ThroughDoor) => assert_eq!((w.room_ctx.cur.num, w.room_ctx.prev.num), (1, -1)),
            _ => {}
        }
    }
    assert!(run.failure.is_none(), "{:?}", run.failure);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::StickOut, Step::StickLit, Step::WebBurnt, Step::DoorOpened, Step::ThroughDoor]);
    assert!(jumped, "Link never jumped the gap");
    assert!(lit_at.is_some());
    // The stick burnt down on the way (210 frames, the torch's hint read with it held there):
    // one stick less, nothing in hand.
    assert_eq!((w.save.ammo(ITEM_DEKU_STICK), w.player().held_item_ap), (9, 0));
    eprintln!("steps {:?}; Link at {:?}", run.steps, w.player().actor.world_pos);
}
