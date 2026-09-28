//! Ladder climbing on Link's house in Kokiri Forest (`func_8083F7BC` → `func_8083EC18`,
//! `func_8084BF1C`, `func_8084C5F8`). The ladder is collision polys 786 and 787 of the scene
//! (`WALL_FLAG_1`: wall type 2), facing −z at z = 1004 from the ground (y −80) to the porch
//! (y 100); its top (polys 784 and 785, `WALL_FLAG_2`) faces the porch.

mod common;

use std::sync::Arc;

use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::player::Action;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn enter(entrance: &str) -> Option<PlayState> {
    let a = assets()?;
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    Some(oot_actors::play_entrance(a, common::data()?, common::rules()?, save).expect("Play_Init"))
}

/// Runs `n` frames holding the stick at `stick_y`, returning each frame's action and position.
fn hold(w: &mut PlayState, stick_y: i8, n: usize) -> Vec<(Action, Vec3)> {
    let mut out = Vec::new();
    let mut prev = PadState::default();
    for _ in 0..n {
        let pad = PadState { stick_y, ..Default::default() };
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        out.push((w.player().action, w.player().actor.world_pos));
    }
    out
}

#[test]
fn child_link_climbs_the_ladder_to_his_house() {
    let Some(mut w) = enter("ENTR_SPOT04_3") else { return };
    // In front of the ladder, facing it (+z), with the camera behind: stick up walks at it.
    w.place_player(Vec3::new(-29.0, -80.0, 960.0), 0);
    // Stick up until the step off the top, then let go.
    let mut run = Vec::new();
    for _ in 0..200 {
        run.extend(hold(&mut w, 60, 1));
        if run.last().unwrap().0 == Action::ClimbEnd {
            break;
        }
    }
    run.extend(hold(&mut w, 0, 40));
    let first = run.iter().position(|(a, _)| *a == Action::Climb || *a == Action::ItemPutAway).expect("Link grabs the ladder");
    let (_, at) = run[first];
    // func_8083EC18: lined up with the ladder (within 8 of its middle, x -29) and
    // `wallDistance - 1` off its plane.
    assert!((at.x + 29.0).abs() < 8.0, "grabbed at {at:?}");
    assert!(at.z < 1004.0 && at.z > 990.0, "grabbed at {at:?}");
    // It climbs (y rises through the rungs) and steps off the top (func_8084C5F8) onto the
    // porch, standing at y 100.
    let top = run.iter().position(|(a, _)| *a == Action::ClimbEnd).expect("Link reaches the top");
    assert!(run[first..top].iter().all(|(a, _)| matches!(a, Action::Climb | Action::ItemPutAway)));
    let ys: Vec<f32> = run[first..top].iter().map(|(_, p)| p.y).collect();
    assert!(ys.last().unwrap() > &50.0, "climbed up to {:?}", ys.last());
    let (end_action, end) = *run.last().unwrap();
    assert!(!matches!(end_action, Action::Climb | Action::ClimbEnd), "off the ladder: {end_action:?}");
    assert!((end.y - 100.0).abs() < 1.0, "on the porch: {end:?}");
    assert!(end.z > 1004.0, "past the ladder's plane: {end:?}");
}

#[test]
fn child_link_climbs_down_from_his_porch() {
    let Some(mut w) = enter("ENTR_SPOT04_3") else { return };
    // On the porch behind the ladder's top, facing the drop (-z).
    w.place_player(Vec3::new(-29.0, 100.0, 1040.0), i16::MIN);
    let mut run = Vec::new();
    for _ in 0..60 {
        run.extend(hold(&mut w, 60, 1));
        if matches!(run.last().unwrap().0, Action::Climb | Action::ItemPutAway) {
            break;
        }
    }
    let grab = run.iter().position(|(a, _)| matches!(a, Action::Climb | Action::ItemPutAway));
    let path: Vec<String> = run.iter().map(|(a, p)| format!("{a:?}@{:.0},{:.0},{:.0}", p.x, p.y, p.z)).collect();
    let grab = grab.unwrap_or_else(|| panic!("Link gets onto the ladder from the porch: {path:?}"));
    // func_8083A6AC: walking off the edge over a climbable wall (FLOOR_PROPERTY_6, the
    // ladder's top), Link hangs from the edge facing back (+z) and climbs down.
    assert!(run[grab].1.y > 50.0);
    assert_eq!(w.player().actor.shape_rot.y & !0xFF, 0, "facing the ladder (+z): {:#x}", w.player().actor.shape_rot.y);
    // Down to the ground: stick down, then off the bottom (func_8084C5F8) standing.
    let mut down = Vec::new();
    for _ in 0..300 {
        down.extend(hold(&mut w, -60, 1));
        if down.last().unwrap().0 == Action::ClimbEnd {
            break;
        }
    }
    down.extend(hold(&mut w, 0, 30));
    let (a, p) = *down.last().unwrap();
    assert!(!matches!(a, Action::Climb | Action::ClimbEnd), "off the ladder: {a:?} at {p:?}");
    assert!((p.y + 80.0).abs() < 1.0, "on the ground: {p:?}");
}
