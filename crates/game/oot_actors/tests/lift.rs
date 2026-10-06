//! The collapsing platform (`Obj_Lift`, `z_obj_lift.c`) against the C, in the Master Quest Deku
//! Tree's room 2 (`ootx scene-info --scene ydan`): params 0x0080 at (-1214, 390, 1208) rot
//! (0, -8192, 0): size 0 (scale 0.1, a fall check 18 down), switch flag 0x20, no wait
//! (`sFallTimerDurations[0]`).
//!
//! The tests start in room 2 (`DEKU_TREE_ROOM_STARTS`' debug start) and place Link on the
//! platform. Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_collision::dyna::DYNA_TRANSFORM_POS;
use eng_input::pad::PadState;
use eng_math::{cos_s, sin_s};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::obj_lift::{self, ACTOR_OBJ_LIFT, Action, NA_SE_EV_BLOCK_SHAKE, NA_SE_EV_BOX_BREAK, OBJECT_D_LIFT, ObjLift};
use oot_actors::playthrough::{DEKU_TREE_ROOM_STARTS, deku_tree_room_start};
use oot_game::actor::ACTOR_FLAG_UPDATE_CULLING_DISABLED;
use oot_game::actor_ctx::ActorHandle;
use oot_game::effect::kakera::{self as k, KAKERA_COLOR_NONE};
use oot_game::effect::{EFFECT_SS_DUST, EFFECT_SS_KAKERA};
use oot_game::play::{PlayState, Rand, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::quake::QUAKE_TYPE_1;
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

fn sfx_on(w: &PlayState, frame: u32, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == frame && s == id)
}

const HOME: Vec3 = Vec3::new(-1214.0, 390.0, 1208.0);
const SWITCH_FLAG: i32 = 0x20;

/// Inside the Deku Tree (`deku-tree-inside`), the sound log on, 60 frames in, then room 2's
/// debug start and a few frames for its objects and actors.
fn room_2(a: &Arc<GameAssets>, save_flag: bool) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-inside").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    w.flags.set_switch(0x1F);
    idle(&mut w, 60);
    if save_flag {
        w.flags.set_switch(SWITCH_FLAG);
    }
    let (room, pos, yaw, _) = DEKU_TREE_ROOM_STARTS.iter().copied().find(|s| s.3 == "room2").expect("room 2's start");
    deku_tree_room_start(&mut w, room, pos, yaw);
    idle(&mut w, 4);
    w
}

fn lift_handle(w: &PlayState) -> Option<ActorHandle> {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<ObjLift>(h).is_some_and(|l| !l.actor.killed))
}

fn lift(w: &PlayState, h: ActorHandle) -> &ObjLift {
    w.actors.downcast::<ObjLift>(h).expect("Obj_Lift")
}

#[test]
fn room_2s_platform_is_placed_whole() {
    let Some(a) = assets() else { return };
    let w = room_2(&a, false);
    let h = lift_handle(&w).expect("Obj_Lift");
    let l = lift(&w, h);
    // ObjLift_Init: sScales[0] 0.1, gravity -0.6, minVelocityY -15 (sInitChain), the wait
    // (sFallTimerDurations[0] 0), FLAGS ACTOR_FLAG_UPDATE_CULLING_DISABLED; the collision with
    // DYNA_TRANSFORM_POS; flag 0x20 not set.
    assert_eq!((l.actor.params, l.actor.home_pos, l.actor.world_pos), (0x0080, HOME, HOME));
    assert_eq!((l.actor.scale, l.actor.gravity, l.actor.min_velocity_y), (Vec3::splat(0.1), -0.6, -15.0));
    assert_eq!((l.action, l.timer, l.switch_flag()), (Action::Wait, 0, SWITCH_FLAG));
    assert_eq!(l.actor.shape_rot.y, -8192);
    assert_ne!(l.actor.flags & ACTOR_FLAG_UPDATE_CULLING_DISABLED, 0);
    let bg = &w.col.dyna.actors[l.bg as usize];
    assert!(bg.in_use() && !bg.collision_disabled);
    assert_eq!(bg.move_flags, DYNA_TRANSFORM_POS);
    // sFallTimerDurations, and the @bug (game) read past it for 7.
    assert_eq!(obj_lift::fall_timer_duration(0x0080), 0);
    assert_eq!(obj_lift::fall_timer_duration(0x0680), 60);
    assert_eq!(obj_lift::fall_timer_duration(0x0780), 120);
    assert!(!w.flags.get_switch(SWITCH_FLAG));
}

#[test]
fn the_platform_shakes_falls_and_breaks_under_link() {
    let Some(a) = assets() else { return };
    let mut w = room_2(&a, false);
    let h = lift_handle(&w).expect("Obj_Lift");
    let bg = lift(&w, h).bg;
    // Its top: the floor under its position, from above (its own collision).
    let (top, poly) = w.col.entity_raycast_down(HOME + Vec3::Y * 100.0);
    assert!(poly.is_some_and(|p| p.bg == bg), "the platform's top");
    // Its fall's floor: what's under it, its own collision skipped.
    let (ground, _) = w.col.entity_raycast_down_actor(HOME + Vec3::new(0.0, -18.0, 0.0), bg);
    assert!(ground > -1000.0 && ground < HOME.y, "the floor under it at {ground}");
    // Off it, the wait (0) is reset every frame and nothing happens.
    idle(&mut w, 5);
    assert_eq!((lift(&w, h).action, lift(&w, h).timer), (Action::Wait, 0));
    // Link onto it. Player's floor check sets DYNA_INTERACT_PLAYER_ON_TOP during his update,
    // after the platform's (ACTORCAT_BG first): it reads it the next frame.
    w.place_player(Vec3::new(HOME.x, top, HOME.z), 0);
    let mut n = 0;
    while lift(&w, h).action == Action::Wait {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 5, "never stood on");
    }
    // ObjLift_Wait with the timer at 0: Quake_Request(GET_ACTIVE_CAM, QUAKE_TYPE_1), speed 10000,
    // y 2, duration 20; ObjLift_SetupShake: 20 frames.
    let l = lift(&w, h);
    assert_eq!((l.action, l.timer), (Action::Shake, 20));
    let q = w.quake.requests.iter().find(|r| r.type_ == QUAKE_TYPE_1).expect("the quake");
    assert_eq!((q.speed, q.y, q.x, q.fov, q.up_pitch_offset, q.duration), (10000, 2, 0, 0, 0, 20));
    assert_eq!(q.cam_id, w.active_cam_id);
    // ObjLift_Shake, a frame at a time: the timer down first (ObjLift_Update), then while it's
    // over 0 the phases on (x 10000, y and z 18000) and the platform tilted (300 sin/cos of x
    // about x and z), bobbing (sin y) and circling (3 sin/cos z) round home; the shake's sound
    // when the timer's low bits are 3 (19, 15, 11, 7, 3).
    let mut so = lift(&w, h).shake_orientation;
    let mut timer = 20i16;
    let mut sounds = 0;
    let mut frames = 0;
    loop {
        idle(&mut w, 1);
        frames += 1;
        timer -= 1;
        let l = lift(&w, h);
        if timer <= 0 {
            // ObjLift_SetupFall: home, unturned.
            assert_eq!((l.action, l.actor.world_pos, l.actor.shape_rot.x, l.actor.shape_rot.z), (Action::Fall, HOME, 0, 0));
            break;
        }
        so[0] = so[0].wrapping_add(10000);
        so[1] = so[1].wrapping_add(18000);
        so[2] = so[2].wrapping_add(18000);
        assert_eq!(l.shake_orientation, so, "frame {frames}");
        let rx = (sin_s(so[0]) * 300.0) as i32 as i16;
        let rz = (cos_s(so[0]) * 300.0) as i32 as i16;
        assert_eq!((l.actor.world_rot.x, l.actor.world_rot.z, l.actor.shape_rot.x, l.actor.shape_rot.z), (rx, rz, rx, rz), "frame {frames}");
        let p = Vec3::new(sin_s(so[2]) * 3.0 + HOME.x, sin_s(so[1]) + HOME.y, cos_s(so[2]) * 3.0 + HOME.z);
        assert_eq!(l.actor.world_pos, p, "frame {frames}");
        let shook = sfx_on(&w, w.audio.frames, NA_SE_EV_BLOCK_SHAKE);
        assert_eq!(shook, timer & 3 == 3, "frame {frames}, timer {timer}");
        sounds += shook as u32;
    }
    assert_eq!((frames, sounds), (20, 5));
    // ObjLift_Fall: Actor_MoveXZGravity (velocity.y -0.6 a frame, down to -15, × 1.5), then
    // BgCheck_EntityRaycastDown4 from where it was, 18 lower; it breaks once that floor is within
    // 18 (less 0.001) under it. Link rides it down (DYNA_TRANSFORM_POS).
    let (mut y, mut vy) = (HOME.y, 0.0f32);
    let mut falls = 0;
    let break_frame = loop {
        let prev_y = y;
        vy = (vy - 0.6).max(-15.0);
        y += vy * 1.5;
        idle(&mut w, 1);
        falls += 1;
        assert!(falls < 200, "never landed");
        let (floor, _) = w.col.entity_raycast_down_actor(Vec3::new(HOME.x, prev_y - 18.0, HOME.z), bg);
        if floor - y >= -18.0 - 0.001 {
            break w.audio.frames;
        }
        let l = lift(&w, h);
        assert_eq!((l.action, l.actor.world_pos, l.actor.velocity.y), (Action::Fall, Vec3::new(HOME.x, y, HOME.z), vy), "fall frame {falls}");
    };
    eprintln!("fell {falls} frames to {y} (floor {ground})");
    // The break (ObjLift_SpawnFragments): nine pieces (sFragmentScales: 50 frames), 13 puffs of
    // dust (func_80033480(pos, 120, 12, 120, 100, 1): 10 frames); NA_SE_EV_BOX_BREAK; flag 0x20;
    // gone.
    assert!(lift_handle(&w).is_none());
    let pieces: Vec<_> = w.effect_ss.table.iter().filter(|e| e.ty == EFFECT_SS_KAKERA && e.life == 49).collect();
    assert_eq!(pieces.len(), 9);
    assert!(pieces.iter().all(|e| e.gfx == Some(("object_d_lift", "gCollapsingPlatformDL")) && e.regs[k::R_GRAVITY] == -256 && e.regs[k::R_REG9] == 32));
    assert_eq!(w.effect_ss.table.iter().filter(|e| e.ty == EFFECT_SS_DUST && e.life == 9).count(), 13);
    assert!(sfx_on(&w, break_frame, NA_SE_EV_BOX_BREAK));
    assert!(w.flags.get_switch(SWITCH_FLAG));
    // Not spawned again: ObjLift_Init with flag 0x20 set kills it.
    let again = w.actor_spawn(ACTOR_OBJ_LIFT, HOME, [0, -8192, 0], 0x0080).expect("Obj_Lift");
    assert!(w.actors.actor(again).unwrap().killed);
}

#[test]
fn a_broken_platform_isnt_there_when_room_2_loads() {
    let Some(a) = assets() else { return };
    let w = room_2(&a, true);
    assert!(lift_handle(&w).is_none());
    // Its collision went with it: the floor under its position is the room's.
    let (_, poly) = w.col.entity_raycast_down(HOME + Vec3::Y * 100.0);
    assert!(poly.is_none_or(|p| p.is_scene()));
}

/// `ObjLift_SpawnFragments` the C's way, from `r`: for each of `sFragmentScales`, its place
/// (`x × scale.x + pos.x`, `pos.y`, `z × scale.z + pos.z`), the velocity 0.8 of that out and
/// `Rand × 10 + 6` up, then the argument list's two `Rand_ZeroOne` calls left to right (the
/// tumble 64 or 32, the scale `(Rand × 50 + 50) × scale.x`), `EffectSsKakera_Init`'s pitch and
/// yaw; then 13 puffs (`func_80033480(pos, 120, 12, 120, 100, 1)`).
#[test]
fn the_breaks_pieces_follow_the_c() {
    let Some(a) = assets() else { return };
    let mut w = room_2(&a, false);
    let h = lift_handle(&w).expect("Obj_Lift");
    w.effect_ss = Default::default();
    let mut r: Rand = w.rand;
    let at = HOME;
    let mut want = Vec::new();
    for &(fx, fz) in &obj_lift::FRAGMENT_SCALES {
        let pos = Vec3::new(fx as f32 * 0.1 + at.x, at.y, fz as f32 * 0.1 + at.z);
        let vy = r.zero_one() * 10.0 + 6.0;
        let velocity = Vec3::new(fx as f32 * 0.1 * 0.8, vy, fz as f32 * 0.1 * 0.8);
        let tumble: i16 = if r.zero_one() < 0.5 { 64 } else { 32 };
        let scale = ((r.zero_one() * 50.0 + 50.0) * 0.1) as i16;
        let pitch = (r.zero_one() * 32767.0) as i16;
        let yaw = (r.zero_one() * 32767.0) as i16;
        want.push((pos, velocity, tumble, scale, pitch, yaw));
    }
    let mut puffs = Vec::new();
    for _ in 0..13 {
        let x = at.x + (r.zero_one() - 0.5) * 120.0;
        let y = at.y + (r.zero_one() - 0.5) * 120.0;
        let z = at.z + (r.zero_one() - 0.5) * 120.0;
        let s = (((120.0 * r.zero_one()) * 0.2) as i16).wrapping_add(120);
        r.zero_one(); // EffectSsDust_Init's colour offset (draw flags 5).
        puffs.push((Vec3::new(x, y, z), s));
    }
    let mut act = w.actors.take(h).unwrap();
    act.as_any_mut().downcast_mut::<ObjLift>().unwrap().spawn_fragments(&mut w);
    w.actors.put_back(h, act);
    assert_eq!(w.rand, r, "the Rand calls, in the C's order");
    let slot = w.object_ctx.get_index(OBJECT_D_LIFT).expect("object_d_lift in a bank") as i16;
    for (i, &(pos, velocity, tumble, scale, pitch, yaw)) in want.iter().enumerate() {
        let e = &w.effect_ss.table[i];
        // EffectSsKakera_Spawn(pos, velocity, &world.pos, -256, tumble, 15, 15, 0, scale, 0, 32,
        // 50, KAKERA_COLOR_NONE, OBJECT_D_LIFT, gCollapsingPlatformDL).
        assert_eq!((e.ty, e.life, e.pos, e.velocity, e.vec), (EFFECT_SS_KAKERA, 50, pos, velocity, at), "piece {i}");
        let rg = &e.regs;
        assert_eq!((rg[k::R_GRAVITY], rg[k::R_REG4], rg[k::R_REG5], rg[k::R_REG6], rg[k::R_REG0]), (-256, tumble, 15, 15, 0), "piece {i}");
        assert_eq!((rg[k::R_SCALE], rg[k::R_REG8], rg[k::R_REG9], rg[k::R_PITCH], rg[k::R_YAW]), (scale, 0, 32, pitch, yaw), "piece {i}");
        assert_eq!((rg[k::R_OBJ_ID], rg[k::R_OBJECT_SLOT], rg[k::R_COLOR_IDX]), (OBJECT_D_LIFT, slot, KAKERA_COLOR_NONE));
    }
    for (j, &(pos, s)) in puffs.iter().enumerate() {
        let e = &w.effect_ss.table[9 + j];
        assert_eq!((e.ty, e.pos, e.regs[oot_game::effect::dust::R_SCALE]), (EFFECT_SS_DUST, pos, s), "puff {j}");
    }
}
