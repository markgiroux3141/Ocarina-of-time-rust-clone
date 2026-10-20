//! GAME-09 milestone 5 against the C: `Bg_Ddan_Kd` (`z_bg_ddan_kd.c`), Dodongo's Cavern's falling
//! stairs, and one-point cutscene 3050 (`z_onepointdemo.c`). Master Quest's room 2 has one, 0x000B
//! at (-1960, 480, -1900), facing -x.
//!
//! Entered by `ENTR_DODONGOS_CAVERN_0` on a `fairy-magic` save, then room 2 as walking into it
//! would (the debug start's `--room`). Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_ddan_kd::*;
use oot_actors::en_bom::{ACTOR_EN_BOM, EnBom};
use oot_actors::en_bombf::EnBombf;
use oot_game::actor_ctx::ActorHandle;
use oot_game::camera::CAM_ID_MAIN;
use oot_game::env::clock_time;
use oot_game::onepoint::CAM_SET_CS_3;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// `SCENE_DODONGOS_CAVERN` (`scene_table.h`: 0x01).
const SCENE_DODONGOS_CAVERN: usize = 0x01;
/// The stairs' switch flag (their params 0x000B) and home.
const STAIRS_FLAG: i32 = 0x0B;
const STAIRS_HOME: Vec3 = Vec3::new(-1960.0, 480.0, -1900.0);

/// Child Link in room 2 (`fairy-magic`, the cavern's entrance wall broken), `prepare` run on the
/// save first, by the room's south door (-1960, 0, -1150), three frames on, the audio logged.
fn room2_with(a: &Arc<GameAssets>, prepare: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DODONGOS_CAVERN_0").expect("entrance");
    let mut save = SaveContext::new(e, false, clock_time(10, 0) as u16);
    save.apply_preset("fairy-magic").unwrap();
    prepare(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    // Room_RequestNewRoom, a frame for it to load, Room_FinishRoomChange (the debug start's --room).
    assert!(w.room_request(2));
    idle(&mut w, 1);
    w.room_change_done();
    w.audio.log = Some(Default::default());
    w.place_player(Vec3::new(-1960.0, 0.0, -1150.0), i16::MIN);
    idle(&mut w, 3);
    w
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn stairs_h(w: &PlayState) -> ActorHandle {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<BgDdanKd>(h).is_some()).expect("the stairs")
}

fn stairs(w: &PlayState) -> &BgDdanKd {
    w.actors.downcast::<BgDdanKd>(stairs_h(w)).unwrap()
}

/// Room 2's Bomb Flowers taken away (their chain would answer the test bombs' blasts).
fn without_flowers(w: &mut PlayState) {
    for h in w.actors.all() {
        if w.actors.downcast::<EnBombf>(h).is_some() {
            w.actors.actor_mut(h).unwrap().kill();
        }
    }
    idle(w, 1);
}

fn bomb_at(w: &mut PlayState, pos: Vec3, fuse: i16) -> ActorHandle {
    let h = w.actor_spawn(ACTOR_EN_BOM, pos, [0; 3], 0).unwrap();
    w.actors.downcast_mut::<EnBom>(h).unwrap().timer = fuse;
    h
}

fn sfx_frames(w: &PlayState, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, _)| f).collect()
}

/// Ticks until the stairs' action is `a` (at most `max` frames); the frames it took.
fn until(w: &mut PlayState, a: Action, max: usize) -> usize {
    for n in 1..=max {
        idle(w, 1);
        if stairs(w).action == a {
            return n;
        }
    }
    panic!("not {a:?} within {max} frames (still {:?})", stairs(w).action);
}

/// `Math_SmoothStepToF` as the C writes it (`z_lib.c`): the expected values' own model.
fn c_smooth_step_to_f(v: &mut f32, target: f32, fraction: f32, step: f32, min_step: f32) -> f32 {
    if *v != target {
        let mut step_size = (target - *v) * fraction;
        if step_size >= min_step || step_size <= -min_step {
            step_size = step_size.clamp(-step, step);
            *v += step_size;
        } else {
            // Both small-step branches, as the C writes them (the second can undo the first).
            if step_size < min_step {
                *v += min_step;
                step_size = min_step;
                if target < *v {
                    *v = target;
                }
            }
            if step_size > -min_step {
                *v += -min_step;
                if *v < target {
                    *v = target;
                }
            }
        }
    }
    (target - *v).abs()
}

#[test]
fn the_stairs_wait_up_and_start_down_with_their_flag() {
    let Some(a) = assets() else { return };
    let w = room2_with(&a, |_| {});
    // BgDdanKd_Init: scale 0.1, waiting for explosions at their home; the cylinder 245 across,
    // 180 high, 400 down (from y 80 to 260).
    let s = stairs(&w);
    assert_eq!((s.actor.params, s.action, s.actor.world_pos), (0x000B, Action::CheckForExplosions, STAIRS_HOME));
    assert_eq!(s.actor.scale, Vec3::splat(0.1));
    assert_eq!((s.collider.dim.radius, s.collider.dim.height, s.collider.dim.y_shift), (245, 180, -400));
    assert_eq!(s.collider.dim.pos, [-1960, 480, -1900]);
    // Its top is a floor: 480 at (-1960, -1650) (its collision is the dyna's, carried down).
    let (f, p) = w.col.entity_raycast_down(Vec3::new(-1960.0, 700.0, -1650.0));
    assert_eq!((f, p.map(|p| p.bg)), (480.0, Some(s.bg)));
    // With flag 0x0B set: 220 down already, doing nothing; no one-point.
    let w = room2_with(&a, |s| s.scene_flags[SCENE_DODONGOS_CAVERN].swch |= 1 << STAIRS_FLAG);
    let s = stairs(&w);
    assert_eq!((s.action, s.actor.world_pos.y), (Action::DoNothing, 480.0 - 220.0));
    assert_eq!(w.active_cam_id, CAM_ID_MAIN);
}

#[test]
fn one_blast_is_kept_13_frames_and_quenched() {
    let Some(a) = assets() else { return };
    let mut w = room2_with(&a, |_| {});
    without_flowers(&mut w);
    // A bomb on the ledge by the stairs (y 20; under them is the pit they drop into): its blast's sphere reaches the cylinder; BgDdanKd_CheckForExplosions
    // gives it params 2 (its AT stops: EnBom_Explode sets it only with params 1), keeps it for 13
    // frames with where it was; nothing falls.
    let b = bomb_at(&mut w, Vec3::new(-2060.0, 20.0, -1700.0), 2);
    let mut kept_at = None;
    for n in 1..=40 {
        idle(&mut w, 1);
        if stairs(&w).prev_explosive.is_some() {
            kept_at = Some(n);
            break;
        }
    }
    kept_at.expect("the blast caught");
    assert_eq!(stairs(&w).prev_explosive, Some(b));
    assert_eq!(stairs(&w).timer, 13);
    assert_eq!(w.actors.actor(b).unwrap().params, 2);
    assert_eq!(stairs(&w).prev_explosive_pos.x, -2060.0);
    // 13 frames counting down, the 14th keeps the frame's explosive: none.
    idle(&mut w, 13);
    assert_eq!((stairs(&w).timer, stairs(&w).prev_explosive), (0, Some(b)));
    idle(&mut w, 1);
    assert_eq!(stairs(&w).prev_explosive, None);
    assert_eq!(stairs(&w).action, Action::CheckForExplosions);
}

#[test]
fn two_blasts_80_apart_drop_the_stairs_with_one_point_3050() {
    let Some(a) = assets() else { return };
    let mut w = room2_with(&a, |_| {});
    without_flowers(&mut w);
    // Two bombs 200 apart, the same fuse: the first frame both blasts reach the cylinder one is
    // returned (`Actor_GetCollidedExplosive`) and kept, its AT quenched; the next frame the other,
    // different and more than 80 away: the fall and OnePointCutscene_Init(3050, 999).
    bomb_at(&mut w, Vec3::new(-2060.0, 20.0, -1700.0), 2);
    bomb_at(&mut w, Vec3::new(-1860.0, 20.0, -1700.0), 2);
    until(&mut w, Action::LowerStairs, 40);
    let sub = w.active_cam_id;
    assert_ne!(sub, CAM_ID_MAIN);
    let cam = w.camera(sub).unwrap();
    assert_eq!((cam.cs_id, cam.setting), (ONEPOINT_FALL, CAM_SET_CS_3));
    // 3050: D_80120304 (0) | 0x2000 (1020 after it), D_80120300 (210) frames; the at's and eye's
    // splines D_8012013C and D_8012021C; the main camera at their points 11 ((-1974, 12, -1179),
    // (-2007, 10, -1004)); the chime (OOT_VERSION >= PAL_1_0); Player held (PLAYER_CSACTION_5).
    let op = &cam.one_point_cam_data;
    assert_eq!((op.action_parameters & 0xFFF, op.init_timer), (0, 210));
    assert_eq!(op.at_points, w.onepoint.point_list("D_8012013C"));
    assert_eq!(op.eye_points, w.onepoint.point_list("D_8012021C"));
    // The main camera, CAM_STAT_UNK3 under the cutscene, still runs its function (Camera_Update
    // returns early only for CAM_STAT_WAIT): it has moved on from there by the frame's end.
    assert!(w.game_camera.at.distance(Vec3::new(-1974.0, 12.0, -1179.0)) < 40.0, "{:?}", w.game_camera.at);
    assert!(!sfx_frames(&w, oot_game::audio::sfx::NA_SE_SY_CORRECT_CHIME).is_empty());
    // The fall: the speed up 0.025 a frame to 4 (Math_SmoothStepToF at 0.5, at most 0.025), the
    // stairs down at 0.075 of the way, at most the speed, at least 0.0075, to 260; then flag 0x0B.
    let (mut speed, mut y, mut frames) = (0.0f32, 480.0f32, 0);
    loop {
        frames += 1;
        c_smooth_step_to_f(&mut speed, 4.0, 0.5, 0.025, 0.0);
        if c_smooth_step_to_f(&mut y, 260.0, 0.075, speed, 0.0075) == 0.0 {
            break;
        }
    }
    let mut prev = stairs(&w).actor.world_pos.y;
    assert_eq!(prev, 480.0);
    idle(&mut w, 1);
    assert!(stairs(&w).actor.world_pos.y < prev);
    prev = stairs(&w).actor.world_pos.y;
    assert_eq!(prev, 480.0 - 0.025);
    assert!(sfx_frames(&w, NA_SE_EV_PILLAR_SINK - oot_game::audio::sfx::SFX_FLAG).len() == 1);
    let n = until(&mut w, Action::DoNothing, 1000);
    assert_eq!(n + 1, frames, "the fall's frames");
    assert_eq!(stairs(&w).actor.world_pos.y, 260.0);
    assert!(w.flags.get_switch(STAIRS_FLAG));
    // The dyna's floor came down with them.
    let (f, _) = w.col.entity_raycast_down(Vec3::new(-1960.0, 700.0, -1650.0));
    assert_eq!(f, 260.0);
    // 3050's 210 frames then 1020 back to Link (42 frames: 210 / 5), the main camera again.
    for _ in 0..400 {
        if w.active_cam_id == CAM_ID_MAIN {
            break;
        }
        idle(&mut w, 1);
    }
    assert_eq!(w.active_cam_id, CAM_ID_MAIN);
}

#[test]
fn two_blasts_close_together_or_apart_in_time_drop_nothing() {
    let Some(a) = assets() else { return };
    // 60 apart (not more than 80): the second isn't the fall's.
    let mut w = room2_with(&a, |_| {});
    without_flowers(&mut w);
    bomb_at(&mut w, Vec3::new(-1990.0, 20.0, -1700.0), 2);
    bomb_at(&mut w, Vec3::new(-1930.0, 20.0, -1700.0), 2);
    idle(&mut w, 60);
    assert_eq!(stairs(&w).action, Action::CheckForExplosions);
    assert_eq!(w.active_cam_id, CAM_ID_MAIN);
    // 200 apart, the second 20 frames after the first: the first is forgotten by then (13 frames).
    let mut w = room2_with(&a, |_| {});
    without_flowers(&mut w);
    bomb_at(&mut w, Vec3::new(-2060.0, 20.0, -1700.0), 2);
    bomb_at(&mut w, Vec3::new(-1860.0, 20.0, -1700.0), 22);
    idle(&mut w, 80);
    assert_eq!(stairs(&w).action, Action::CheckForExplosions);
    // 200 apart, 10 frames after: within the 13, the fall.
    let mut w = room2_with(&a, |_| {});
    without_flowers(&mut w);
    bomb_at(&mut w, Vec3::new(-2060.0, 20.0, -1700.0), 2);
    bomb_at(&mut w, Vec3::new(-1860.0, 20.0, -1700.0), 12);
    until(&mut w, Action::LowerStairs, 60);
}

#[test]
fn the_bomb_flower_rows_lit_at_their_junction_drop_the_stairs() {
    let Some(a) = assets() else { return };
    let mut w = room2_with(&a, |_| {});
    // Room 2's two rows (x -2070 and -1844, z -1305 to -1665) and their junction (-2015 and -1903
    // at z -1305): a bomb between the junction's two flowers sets both off; the rows burn down
    // together and their last flowers' blasts, 226 apart, reach the stairs' cylinder in the same
    // frames: the fall.
    let flowers = w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnBombf>(h).is_some()).count();
    assert_eq!(flowers, 17);
    bomb_at(&mut w, Vec3::new(-1959.0, 20.0, -1290.0), 2);
    let n = until(&mut w, Action::LowerStairs, 600);
    assert!(n > 60, "the chain took {n} frames");
}
