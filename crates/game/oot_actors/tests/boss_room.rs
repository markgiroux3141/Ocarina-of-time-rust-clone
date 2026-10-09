//! GAME-05 milestone 6c's exit (`Route::BossRoom`, the `boss-room` script's run, also a golden
//! trace): from room 9's debug start with the room cleared (the hint scrubs' puzzle solved, its
//! door to room 11 unbarred), Link opens the door, walks onto room 11's floor (exit 2: the drop
//! into Queen Gohma's room) and stands in her room's corridor. The whole Deku Tree from one start
//! is the travel test's (`travel.rs`).

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use oot_actors::PlayExt;
use oot_actors::player::Action as PA;
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::play::scripted_input;
use oot_game::play_scene::GameAssets;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

#[test]
fn exit_from_room_9_into_queen_gohmas_room() {
    let Some(a) = assets() else { return };
    let route = Route::BossRoom;
    let e = a.scenes.entrance_index(route.entrance()).expect("entrance");
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), route.save(e)).expect("Play_Init");
    route.debug_start(&mut w);
    // SCENE_DEKU_TREE (0), room 9, cleared (Flags_GetClear: DoorShutter_Init leaves the door
    // unbarred).
    assert_eq!((w.scene_id, w.room_ctx.cur.num), (0, 9));
    assert!(w.flags.get_clear(9));
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    let mut room11 = false;
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
        room11 |= w.scene_id == 0 && w.room_ctx.cur.num == 11;
    }
    assert!(run.failure.is_none(), "{:?}", run.failure);
    eprintln!("steps {:?}", run.steps);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::DoorOpened, Step::BossRoom]);
    // Through the door: Room_RequestNewRoom(11). Room 11's floor is exit 2 (the scene's exit
    // list: ENTR_DEKU_TREE_BOSS_0), one scene change.
    assert!(room11);
    assert_eq!(w.scene_changes, 1);
    // SCENE_DEKU_TREE_BOSS (0x11) at ENTR_DEKU_TREE_BOSS_0's spawn: room 1, the corridor (room
    // 0 is hers); Link standing.
    assert_eq!(w.save.entrance_index, a.scenes.entrance_index("ENTR_DEKU_TREE_BOSS_0").unwrap());
    assert_eq!((w.scene_id, w.room_ctx.cur.num), (0x11, 1));
    assert_eq!(w.player().action, PA::StandingStill);
}
