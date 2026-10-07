//! GAME-05 milestone 5a's exit (`Route::Slingshot`, the `slingshot` script's run, also a golden
//! trace): inside the Deku Tree, from a debug start in room 1 250 in front of its eye switch, Link
//! takes the Fairy Slingshot out (C-Right), aims in first person at the eye and shoots a seed
//! into it: the eye closes (flag 0x0C), the door to room 2 unbars with its camera, and he goes
//! through it into room 2.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use oot_actors::PlayExt;
use oot_actors::playthrough::{ROOM1_EYE_FLAG, Playthrough, Route, Step};
use oot_game::camera::{CAM_ID_MAIN, CAM_MODE_AIM_CHILD};
use oot_game::item::ITEM_SLINGSHOT;
use oot_game::play::scripted_input;
use oot_game::play_scene::GameAssets;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

#[test]
fn exit_a_seed_into_room_1s_eye_and_through_its_door() {
    let Some(a) = assets() else { return };
    let route = Route::Slingshot;
    let e = a.scenes.entrance_index(route.entrance()).expect("entrance");
    let save = route.save(e);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = oot_game::play::PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    route.debug_start(&mut w);
    assert_eq!(w.room_ctx.cur.num, 1);
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
        if std::env::var("SLINGSHOT_TRACE").is_ok() {
            let pl = w.player();
            let seed = pl.held_actor.and_then(|h| w.actors.actor(h)).map(|s| (s.world_pos, s.world_rot));
            eprintln!(
                "{} {} {:?} {:?} up {:?} 6AD {} focus {:?} seed {:?} pad {:?} cam {} mode {}",
                w.audio.frames,
                run.at(),
                pl.action,
                pl.actor.world_pos,
                pl.upper,
                pl.unk_6AD,
                pl.actor.focus_rot,
                seed,
                p,
                w.active_cam_id,
                w.game_camera.mode
            );
        }
        match run.take_done() {
            Some(Step::SlingshotDrawn) => {
                // In first person with the aim's camera, a seed in hand.
                let p = w.player();
                assert_eq!((p.unk_6AD, w.game_camera.mode), (2, CAM_MODE_AIM_CHILD));
                assert!(p.held_actor.is_some());
            }
            Some(Step::EyeShot) => {
                // One seed spent; the eye closed (its flag).
                assert!(w.flags.get_switch(ROOM1_EYE_FLAG));
                assert_eq!(w.save.ammo(ITEM_SLINGSHOT), 29);
            }
            Some(Step::ThroughDoor) => {
                assert_eq!(w.room_ctx.cur.num, 2);
                assert_eq!(w.active_cam_id, CAM_ID_MAIN);
            }
            _ => {}
        }
    }
    assert!(run.failure.is_none(), "{:?}", run.failure);
    println!("health {} of {}", w.save.health, w.save.health_capacity);
    let names: Vec<_> = run.steps.iter().map(|(s, _)| s.name()).collect();
    assert_eq!(names, ["slingshot_drawn", "eye_shot", "door_opened", "through_door"]);
    println!("steps: {:?}", run.steps.iter().map(|(s, f)| (s.name(), f)).collect::<Vec<_>>());
}
