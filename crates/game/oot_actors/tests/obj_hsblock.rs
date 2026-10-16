//! GAME-09 milestone 2 against the C: the stone hookshot targets (`z_obj_hsblock.c`). Goron City's
//! room 3 (`ootx scene-info --scene spot18`) has a post, params 0xFFC0 (type 0) at (1161, 480,
//! -148); the other types are spawned in the same room (its object list has `object_d_hsblock`).
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::obj_hsblock::{self as hsblock, Action, ObjHsblock};
use oot_game::actor::ACTOR_FLAG_UPDATE_CULLING_DISABLED;
use oot_game::actor_ctx::ActorHandle;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

const POST: Vec3 = Vec3::new(1161.0, 480.0, -148.0);

/// A child at Goron City's bottom (room 3), four frames on.
fn room_3(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_GORON_CITY_2").expect("entrance");
    let save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    idle(&mut w, 4);
    w
}

fn blocks(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<ObjHsblock>(h).is_some_and(|b| !b.actor.killed)).collect()
}

fn hs(w: &PlayState, h: ActorHandle) -> &ObjHsblock {
    w.actors.downcast::<ObjHsblock>(h).unwrap()
}

#[test]
fn goron_citys_post_stands_and_takes_the_hookshot() {
    let Some(a) = assets() else { return };
    let mut w = room_3(&a);
    let v = blocks(&w);
    assert_eq!(v.len(), 1);
    let h = v[0];
    let o = hs(&w, h);
    // ObjHsblock_Init: type 0 (0xFFC0 & 3), gHookshotPostCol (func_80B93B68, transform flags 0),
    // scale 0.1 (sInitChain), no ice (bit 5 clear), standing (no action).
    assert_eq!((o.actor.home_pos, o.action), (POST, Action::None));
    assert_eq!(o.actor.scale, Vec3::splat(0.1));
    assert_eq!(w.col.dyna.actors[o.bg as usize].move_flags, 0);
    // ObjHsblock_Update: Actor_SetFocus(D_80B940C0[0]): 85 up.
    idle(&mut w, 1);
    assert_eq!(hs(&w, h).actor.focus_pos, POST + Vec3::Y * 85.0);
    // Its collision: down its middle from above, its top, above its base, of its bg actor; the
    // surface takes the hookshot.
    let (y, poly) = w.col.entity_raycast_down(POST + Vec3::Y * 400.0);
    assert_eq!(poly.map(|p| p.bg), Some(hs(&w, h).bg), "the post's top at {y}");
    assert!(y > POST.y);
    // Link walking into it is stopped by it (its walls): pushed along +x from 60 short of it.
    w.place_player(POST + Vec3::new(-80.0, 0.0, 0.0), 0x4000);
    let push = PadState { stick_x: 0, stick_y: 60, ..Default::default() };
    let mut prev = PadState::default();
    for _ in 0..40 {
        w.tick_with(scripted_input(prev, push));
        prev = push;
    }
    let p = w.player().actor.world_pos;
    assert!(p.x < POST.x - 10.0 && p.x > POST.x - 60.0 && p.y > POST.y - 5.0, "Link at {p}: through the post, held short or off its floor");
    let header = &w.col.dyna.actors[hs(&w, h).bg as usize].header;
    // gHookshotPostCol's surface types: one takes the hookshot (SurfaceType_CanHookshot: word
    // 1's bit 17).
    assert!(header.surface_types.iter().any(|s| s.data[1] >> 17 & 1 != 0), "SURFACETYPE1's canHookshot");
}

#[test]
fn a_type_1_post_rises_on_its_switch_flag() {
    let Some(a) = assets() else { return };
    let mut w = room_3(&a);
    // Type 1, switch flag 5 (params 0x0501): func_80B93DB0: updated off screen, 105 below its
    // home, waiting (func_80B93DF4).
    let at = Vec3::new(1000.0, 480.0, -148.0);
    let h = w.actor_spawn(hsblock::ACTOR_OBJ_HSBLOCK, at, [0; 3], 0x0501).expect("Obj_Hsblock");
    let o = hs(&w, h);
    assert_eq!((o.action, o.actor.world_pos.y), (Action::WaitForSwitch, at.y - 105.0));
    assert_ne!(o.actor.flags & ACTOR_FLAG_UPDATE_CULLING_DISABLED, 0);
    idle(&mut w, 3);
    assert_eq!(hs(&w, h).action, Action::WaitForSwitch);
    // The flag set: func_80B93E38 (rising) the next frame; velocity.y easing to 16 (a tenth, at
    // most 0.8), the height easing home (three tenths, at most velocity.y, at least 0.3).
    w.flags.set_switch(5);
    idle(&mut w, 1);
    assert_eq!(hs(&w, h).action, Action::Rise);
    let mut vy = 0.0f32;
    let mut y = at.y - 105.0;
    idle(&mut w, 1);
    eng_math::smooth_step_to_f(&mut vy, 16.0, 0.1, 0.8, 0.0);
    eng_math::smooth_step_to_f(&mut y, at.y, 0.3, vy, 0.3);
    assert_eq!((hs(&w, h).actor.velocity.y, hs(&w, h).actor.world_pos.y), (vy, y));
    // Home: standing, culled again.
    for _ in 0..200 {
        idle(&mut w, 1);
        if hs(&w, h).action == Action::None {
            break;
        }
    }
    let o = hs(&w, h);
    assert_eq!((o.action, o.actor.world_pos.y), (Action::None, at.y));
    assert_eq!(o.actor.flags & ACTOR_FLAG_UPDATE_CULLING_DISABLED, 0);
    // With the flag already set it starts home.
    let h2 = w.actor_spawn(hsblock::ACTOR_OBJ_HSBLOCK, at + Vec3::Z * 100.0, [0; 3], 0x0501).expect("Obj_Hsblock");
    assert_eq!((hs(&w, h2).action, hs(&w, h2).actor.world_pos.y), (Action::None, at.y));
}

#[test]
fn a_type_2_target_is_flat_with_its_focus_at_its_base() {
    let Some(a) = assets() else { return };
    let mut w = room_3(&a);
    let at = Vec3::new(1000.0, 480.0, -300.0);
    let h = w.actor_spawn(hsblock::ACTOR_OBJ_HSBLOCK, at, [0; 3], 0x0002).expect("Obj_Hsblock");
    idle(&mut w, 1);
    let o = hs(&w, h);
    assert_eq!(o.action, Action::None);
    // D_80B940C0[2]: 0.
    assert_eq!(o.actor.focus_pos, at);
    // gHookshotTargetCol: its surface takes the hookshot.
    let header = &w.col.dyna.actors[o.bg as usize].header;
    assert!(header.surface_types.iter().any(|s| s.data[1] >> 17 & 1 != 0));
}

#[test]
fn the_ice_bit_spawns_obj_ice_poly_as_its_child() {
    let Some(a) = assets() else { return };
    let mut w = room_3(&a);
    // func_80B93BF0: bit 5 set: Actor_SpawnAsChild(ACTOR_OBJ_ICE_POLY, params 1) (not ported: a
    // placeholder).
    let h = w.actor_spawn(hsblock::ACTOR_OBJ_HSBLOCK, Vec3::new(1000.0, 480.0, 0.0), [0; 3], 0x0020).expect("Obj_Hsblock");
    let child = hs(&w, h).actor.child.expect("the ice");
    let c = w.actors.actor(child).unwrap();
    assert_eq!((c.id, c.params, c.parent), (hsblock::ACTOR_OBJ_ICE_POLY, 1, Some(h)));
}
