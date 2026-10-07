//! GAME-05 milestone 4c's exit (`Route::Push`, the `push` script's run, also a golden trace):
//! inside the Deku Tree, from a debug start on room 3's upper floor behind its push block, Link
//! holds on to the block and pushes it along the floor's channel and off its end into the pit
//! (`Obj_Makeoshihiki`'s flag 0x10, `NA_SE_SY_TRE_BOX_APPEAR`), then goes down into the pit
//! beside it and climbs onto it.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use oot_actors::PlayExt;
use oot_actors::obj_oshihiki::ObjOshihiki;
use oot_actors::playthrough::{PUSH_BLOCK_FLAG, PUSH_BLOCK_PIT, Playthrough, Route, Step};
use oot_game::audio::sfx::*;
use oot_game::play::scripted_input;
use oot_game::play_scene::GameAssets;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

#[test]
fn exit_room_3s_block_pushed_into_the_pit_and_climbed() {
    let Some(a) = assets() else { return };
    let route = Route::Push;
    let e = a.scenes.entrance_index(route.entrance()).expect("entrance");
    let save = route.save(e);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = oot_game::play::PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    route.debug_start(&mut w);
    assert_eq!(w.room_ctx.cur.num, 3);
    let block = |w: &oot_game::play::PlayState| w.actors.all().into_iter().find_map(|h| w.actors.downcast::<ObjOshihiki>(h).map(|b| (b.actor.world_pos, b.bg, b.cant_move)));
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    let mut pushes = 0;
    let mut was_pushing = false;
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
        if std::env::var("PUSH_TRACE").is_ok() {
            let pl = w.player();
            eprintln!(
                "{} {} {:?} {:?} msg {} mode {} text {:#x} pad {:?} cam {}",
                w.audio.frames,
                run.at(),
                pl.action,
                pl.actor.world_pos,
                w.message_state(),
                w.msg_ctx.msg_mode,
                w.msg_ctx.text_id,
                p,
                w.active_cam_id
            );
        }
        let pushing = w.actors.all().into_iter().any(|h| w.actors.downcast::<ObjOshihiki>(h).is_some_and(|b| b.action == Some(oot_actors::obj_oshihiki::Action::Push)));
        if pushing && !was_pushing {
            pushes += 1;
        }
        was_pushing = pushing;
        match run.take_done() {
            Some(Step::BlockGrabbed) => assert_eq!(w.player().action, oot_actors::player::Action::PushWait),
            Some(Step::BlockInPit) => {
                // Obj_Makeoshihiki's draw: the block at sBlocks[1]'s pit place, flag 0x10 set,
                // the chime, and the block immovable now (unk_24[1] & 2).
                let (pos, _, cant_move) = block(&w).unwrap();
                assert!(pos.distance_squared(PUSH_BLOCK_PIT) < 0.001, "the block at {pos:?}");
                assert!(w.flags.get_switch(PUSH_BLOCK_FLAG));
                assert!(cant_move);
                assert!(w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f + 2 >= w.audio.frames && s == NA_SE_SY_TRE_BOX_APPEAR));
            }
            Some(Step::OnBlock) => {
                let (pos, bg, _) = block(&w).unwrap();
                let p = w.player();
                assert_eq!(p.actor.floor_bg_id, bg);
                assert!((p.actor.world_pos.y - (pos.y + 60.0)).abs() < 0.5, "Link at {:?} on the block at {pos:?}", p.actor.world_pos);
            }
            _ => {}
        }
    }
    assert!(run.failure.is_none(), "{:?}", run.failure);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::BlockGrabbed, Step::BlockInPit, Step::OnBlock]);
    // 240 from (-605) to (-365): twelve pushes of 20.
    assert_eq!(pushes, 12);
    assert_eq!(w.room_ctx.cur.num, 3);
    eprintln!("steps {:?}; texts {:?}; Link at {:?}", run.steps, run.texts, w.player().actor.world_pos);
}
