//! The dive off a diving board (`FLOOR_PROPERTY_11`): Zora's Domain's top of the falls, where the
//! diving game is played. Running off it fast and straight (`func_8083AA10`: speed over 3, within
//! 0x2000 of the stick's way) with water over 50 above the floor 100 ahead jumps
//! (`func_808389E8`, `link_normal_run_jump_water_fall`: `Player_Action_80844A44`); into the water
//! it dives (`func_8083D36C`: `Player_Action_8084DC48`, `func_8083D330`), not a swim.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// The board's two triangles (the scene's only `FLOOR_PROPERTY_11` floor), their centre.
const BOARD: Vec3 = Vec3::new(-195.0, 857.0, -1096.0);

/// Link on the board facing `yaw`, run forward for 120 frames: the actions he went through.
fn run_off(a: &Arc<GameAssets>, yaw: i16) -> Vec<&'static str> {
    let e = a.scenes.entrance_index("ENTR_ZORAS_DOMAIN_0").unwrap();
    let mut save = oot_game::save::SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("zora-diving").unwrap();
    let mut w: PlayState = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap();
    let mut prev = PadState::default();
    for _ in 0..20 {
        w.tick_with(scripted_input(prev, PadState::default()));
    }
    w.place_player(BOARD, yaw);
    for _ in 0..10 {
        w.tick_with(scripted_input(prev, PadState::default()));
    }
    let mut seen: Vec<&'static str> = Vec::new();
    let pad = stick(0, 127);
    for _ in 0..120 {
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        let name = w.player().action.decomp_name();
        if seen.last() != Some(&name) {
            seen.push(name);
        }
    }
    seen
}

#[test]
fn running_off_the_diving_board_dives() {
    let Some(a) = assets() else { return };
    let mut dived = false;
    for yaw in [0i16, 0x4000, i16::MIN, -0x4000] {
        let seen = run_off(&a, yaw);
        eprintln!("yaw {yaw:#x}: {seen:?}");
        if let Some(i) = seen.iter().position(|&s| s == "Player_Action_80844A44") {
            // The jump goes straight into the dive (no swim between).
            assert_eq!(seen.get(i + 1), Some(&"Player_Action_8084DC48"), "{seen:?}");
            dived = true;
        }
    }
    assert!(dived, "one way off the board dives");
}
