//! GAME-10 milestone 3 against the C: Lake Hylia's objects (`z_bg_spot06_objects.c`). A child's
//! lake: the water (`LHO_WATER_PLANE`, full, drawn), the Water Temple's gate waiting for its switch
//! and rising when it's set, the lock waiting for the hookshot; the ice block killed for a child;
//! an adult's lake: the ice block, the gate found open, and the drained lake's and the lock's
//! placeholders chosen at init. Also `SDC_LAKE_HYLIA`'s env alpha by age.
//!
//! Entered from `fairy-magic` with the lake's intro seen (`EVENTCHKINF_B1`). Expected values are
//! worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::bg_spot06_objects::{self as spot06, Action, BgSpot06Objects};
use oot_game::actor::{ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED};
use oot_game::actor_ctx::ActorHandle;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

/// `EVENTCHKINF_B1` (`save.h`): Lake Hylia's intro seen.
const EVENTCHKINF_B1: u16 = 0xB1;
/// The gate's and the lock's switch flag in the game's layers (0x001F, 0x011F).
const GATE_SWITCH: i32 = 0x1F;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Lake Hylia (`ENTR_LAKE_HYLIA_0`) at 10:00 from `fairy-magic`, its intro seen, `tweak` last;
/// eight frames on.
fn lake(a: &Arc<GameAssets>, adult: bool, tweak: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_LAKE_HYLIA_0").unwrap();
    let mut save = SaveContext::new(e, adult, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("fairy-magic").unwrap();
    save.adult = adult;
    save.set_event_chk_inf(EVENTCHKINF_B1);
    tweak(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap();
    idle(&mut w, 8);
    w
}

fn idle(w: &mut PlayState, n: u32) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// The live `Bg_Spot06_Objects` by type, and the placeholders standing in for it.
fn objects(w: &PlayState) -> (Vec<ActorHandle>, usize) {
    let all: Vec<ActorHandle> = w.actors.all().into_iter().filter(|&h| w.actors.actor(h).is_some_and(|a| a.id == spot06::ACTOR_BG_SPOT06_OBJECTS && !a.killed)).collect();
    let ported: Vec<ActorHandle> = all.iter().copied().filter(|&h| w.actors.downcast::<BgSpot06Objects>(h).is_some()).collect();
    let placeholders = all.len() - ported.len();
    (ported, placeholders)
}

fn obj(w: &PlayState, ty: i16) -> &BgSpot06Objects {
    objects(w).0.into_iter().map(|h| w.actors.downcast::<BgSpot06Objects>(h).unwrap()).find(|o| o.ty == ty).unwrap_or_else(|| panic!("no type {ty}"))
}

#[test]
fn a_child_s_lake_has_its_water_its_gate_and_its_lock() {
    let Some(a) = assets() else { return };
    let w = lake(&a, false, |_| {});
    let (v, placeholders) = objects(&w);
    // Layer 0 places 0x02FF, 0x001F and 0x011F; no ice block.
    assert_eq!((v.len(), placeholders), (3, 0));
    // LHO_WATER_PLANE: scale 1 (sInitChainWaterPlane), never culled, full (lakeHyliaWaterLevel 0):
    // the high water drawn; the scene's main water boxes untouched at WATER_LEVEL_RAISED.
    let water = obj(&w, spot06::LHO_WATER_PLANE);
    assert_eq!((water.actor.scale, water.action, water.lake_hylia_water_level), (Vec3::splat(1.0), Action::DoNothing, 0.0));
    assert_eq!(water.actor.flags, ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED);
    assert_eq!(water.actor.params, spot06::LHO_WATER_PLANE);
    assert_eq!(w.col.water_box_surface(2), Some(spot06::WATER_LEVEL_RAISED));
    assert_eq!(w.col.water_box_surface(3), Some(spot06::WATER_LEVEL_RAISED));
    // The gate: scale 0.1, its collision registered, waiting for switch 0x1F at home.
    let gate = obj(&w, spot06::LHO_WATER_TEMPLE_ENTRACE_GATE);
    assert_eq!((gate.actor.scale, gate.action, gate.switch_flag), (Vec3::splat(0.1), Action::GateWaitForSwitch, 0x1F));
    assert!(gate.bg != eng_collision::dyna::BG_ACTOR_MAX);
    assert_eq!(gate.actor.world_pos, gate.actor.home_pos);
    // The lock: waiting for the hookshot, its sphere 18 (the model's) at its position, immovable.
    let lock = obj(&w, spot06::LHO_WATER_TEMPLE_ENTRANCE_LOCK);
    assert_eq!(lock.action, Action::LockWait);
    let s = lock.collider.elements[0].dim.world_sphere;
    assert_eq!(s.radius, 18);
    let p = lock.actor.world_pos;
    assert_eq!(s.center, [p.x as i16, p.y as i16, p.z as i16]);
    assert_eq!(lock.actor.col_chk_info.mass, oot_game::collision_check::MASS_IMMOVABLE);
    // The water's bakes are in the pack.
    for b in [spot06::BAKE_HIGH_WATER, spot06::BAKE_LOW_WATER] {
        assert!(a.pack.assets.contains(&oot_game::pack::keys::bake(b)), "{b}");
    }
}

#[test]
fn the_gate_rises_once_its_switch_is_set() {
    let Some(a) = assets() else { return };
    let mut w = lake(&a, false, |_| {});
    let home = obj(&w, spot06::LHO_WATER_TEMPLE_ENTRACE_GATE).actor.home_pos.y;
    w.flags.set_switch(GATE_SWITCH);
    // The gate updates only in its culling volume (its profile's flags): seen from the shore, at
    // the lake's bottom, it waits on.
    idle(&mut w, 5);
    assert_eq!(obj(&w, spot06::LHO_WATER_TEMPLE_ENTRACE_GATE).action, Action::GateWaitForSwitch);
    // In view (as the lock's one-point 4120 points the camera at it): its update runs.
    let h = objects(&w).0.into_iter().find(|&h| w.actors.downcast::<BgSpot06Objects>(h).unwrap().ty == spot06::LHO_WATER_TEMPLE_ENTRACE_GATE).unwrap();
    w.actors.actor_mut(h).unwrap().flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
    idle(&mut w, 1);
    // BgSpot06Objects_GateWaitForSwitch: up 3, 100 frames.
    let g = obj(&w, spot06::LHO_WATER_TEMPLE_ENTRACE_GATE);
    assert_eq!((g.action, g.timer, g.actor.world_pos.y), (Action::GateWaitToOpen, 100, home + 3.0));
    // GateWaitToOpen counts to 0 (the 100th frame turns to GateOpen).
    idle(&mut w, 100);
    assert_eq!(obj(&w, spot06::LHO_WATER_TEMPLE_ENTRACE_GATE).action, Action::GateOpen);
    // GateOpen: 0.6 a frame from home + 3 to home + 120: 195 frames.
    let mut n = 0;
    while obj(&w, spot06::LHO_WATER_TEMPLE_ENTRACE_GATE).action == Action::GateOpen {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 400);
    }
    let g = obj(&w, spot06::LHO_WATER_TEMPLE_ENTRACE_GATE);
    assert!((194..=196).contains(&n), "{n}");
    assert_eq!((g.action, g.actor.world_pos.y), (Action::DoNothing, home + 120.0));
}

#[test]
fn the_ice_block_is_killed_for_a_child() {
    let Some(a) = assets() else { return };
    let mut w = lake(&a, false, |_| {});
    let h = w.actor_spawn(spot06::ACTOR_BG_SPOT06_OBJECTS, Vec3::new(-916.0, -1553.0, 3677.0), [0, 0, 0], 0x03FF).unwrap();
    assert!(w.actors.actor(h).is_none_or(|x| x.killed));
}

#[test]
fn an_adult_s_lake() {
    let Some(a) = assets() else { return };
    // Before the Water Temple: the drained lake and the lock are placeholders chosen at init; the
    // gate waits; the ice block stands (layer 2's 0x03FF), its collision registered.
    let w = lake(&a, true, |_| {});
    let (v, placeholders) = objects(&w);
    assert_eq!((v.len(), placeholders), (2, 2));
    assert_eq!(obj(&w, spot06::LHO_WATER_TEMPLE_ENTRACE_GATE).action, Action::GateWaitForSwitch);
    let ice = obj(&w, spot06::LHO_ICE_BLOCK);
    assert_eq!((ice.action, ice.actor.scale), (Action::DoNothing, Vec3::splat(0.1)));
    assert!(ice.bg != eng_collision::dyna::BG_ACTOR_MAX);
    // SDC_LAKE_HYLIA for an adult before the temple: drawParams[0] 87, the alpha 255.
    let draw = &w.scene.as_ref().unwrap().draw;
    assert_eq!(draw.room_draw_params[0], 87);

    // With the gate's switch set (the lock pulled), an adult finds the gate up.
    // SCENE_LAKE_HYLIA (scene_table.h: 0x57).
    let w = lake(&a, true, |s| s.scene_flags[0x57].swch |= 1u32 << GATE_SWITCH);
    let g = obj(&w, spot06::LHO_WATER_TEMPLE_ENTRACE_GATE);
    assert_eq!((g.action, g.actor.world_pos.y), (Action::DoNothing, g.actor.home_pos.y + 120.0));

    // The lake restored (EVENTCHKINF_RESTORED_LAKE_HYLIA): the water as a child's.
    let w = lake(&a, true, |s| s.set_event_chk_inf(spot06::EVENTCHKINF_RESTORED_LAKE_HYLIA));
    let water = obj(&w, spot06::LHO_WATER_PLANE);
    assert_eq!((water.action, water.lake_hylia_water_level), (Action::DoNothing, 0.0));
    assert_eq!(w.scene.as_ref().unwrap().draw.room_draw_params[0], 0);
}
