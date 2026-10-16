//! GAME-09 milestone 3 against the C: `Bg_Dodoago` (`z_bg_dodoago.c`), the skull in Dodongo's
//! Cavern's lobby (`ddan` room 0, Master Quest: params 0x001A at (0, 196, -1750), facing +z): its
//! eyes lit by blasts, the jaw (`gDodongoLowerJawCol`), one-points 3065 and 3380, the bomb
//! catcher under the bridge's holes, `roomCtx.drawParams` for `SDC_DODONGOS_CAVERN`.
//!
//! Entered by `ENTR_DODONGOS_CAVERN_0` on a `cavern-open` save, with the entrance wall broken
//! (`EVENTCHKINF_B0`, switch 7) unless a test says otherwise; Link on the island in front of the
//! skull. Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_dodoago::*;
use oot_actors::en_bom::{ACTOR_EN_BOM, EnBom};
use oot_actors::en_bombf::{ACTOR_EN_BOMBF, EnBombf};
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::sfx::{NA_SE_EV_STONE_BOUND, NA_SE_SY_CORRECT_CHIME, NA_SE_SY_ERROR};
use oot_game::camera::CAM_ID_MAIN;
use oot_game::effect::{EFFECT_SS_DUST, EFFECT_SS_KIRAKIRA};
use oot_game::env::clock_time;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// `SCENE_DODONGOS_CAVERN` (`scene_table.h`: 0x01).
const SCENE_DODONGOS_CAVERN: usize = 0x01;
/// The skull's switch flag (params 0x001A) and placement.
const SKULL_FLAG: i32 = 0x1A;
const SKULL: Vec3 = Vec3::new(0.0, 196.0, -1750.0);
/// On the skull's head under the bridge's holes (x ±90, z -1500 to -1580), a bit above it: a
/// bomb here lands on the head (y 308 there) by the eye on its side.
const BY_RIGHT_EYE: Vec3 = Vec3::new(90.0, 330.0, -1535.0);
const BY_LEFT_EYE: Vec3 = Vec3::new(-90.0, 330.0, -1535.0);
/// Above the bridge's holes (the bridge at y 591).
const HOLE_RIGHT: Vec3 = Vec3::new(90.0, 600.0, -1540.0);
const HOLE_LEFT: Vec3 = Vec3::new(-90.0, 600.0, -1540.0);
/// The jaw's display list as the pack names it.
const JAW_MESH: &str = "mesh/object_ddan_objects/gDodongoLowerJawDL";

/// Child Link on the island in front of the skull (`cavern-open`, Navi's word about the pit heard:
/// `Elf_Msg` 0x0C15's flag 0x0C), with the entrance wall broken (`EVENTCHKINF_B0`, switch 7) if
/// `wall_broken`, `prepare` run on the save first; three frames on, the audio logged.
fn cavern_with(a: &Arc<GameAssets>, wall_broken: bool, prepare: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DODONGOS_CAVERN_0").expect("entrance");
    let mut save = SaveContext::new(e, false, clock_time(10, 0) as u16);
    save.apply_preset("cavern-open").unwrap();
    if wall_broken {
        save.set_event_chk_inf(EVENTCHKINF_B0);
        save.scene_flags[SCENE_DODONGOS_CAVERN].swch |= 1 << 7;
    }
    save.scene_flags[SCENE_DODONGOS_CAVERN].swch |= 1 << 0x0C;
    prepare(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    w.audio.log = Some(Default::default());
    w.place_player(Vec3::new(0.0, 0.0, -1100.0), i16::MIN);
    idle(&mut w, 3);
    w
}

fn cavern(a: &Arc<GameAssets>) -> PlayState {
    cavern_with(a, true, |_| {})
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn skull_h(w: &PlayState) -> ActorHandle {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<BgDodoago>(h).is_some()).expect("the skull")
}

fn skull(w: &PlayState) -> &BgDodoago {
    w.actors.downcast::<BgDodoago>(skull_h(w)).unwrap()
}

fn eyes(w: &PlayState) -> [i16; 2] {
    w.scene.as_ref().unwrap().draw.room_draw_params
}

fn sfx_frames(w: &PlayState, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, _)| f).collect()
}

/// The meshes the last frame draws.
fn drawn(w: &PlayState) -> Vec<String> {
    let f = w.current_frame();
    let mut out = oot_game::play::DrawOut::default();
    w.draw(&f, &oot_game::play::ViewInfo::new(f.view.eye, glam::Mat4::IDENTITY), &mut out);
    out.opa.iter().chain(out.xlu.iter()).map(|c| c.mesh.name.clone()).collect()
}

/// The floor under (x, z), from 250 up.
fn floor(w: &PlayState, x: f32, z: f32) -> f32 {
    w.col.entity_raycast_down(Vec3::new(x, 250.0, z)).0
}

/// A bomb (`En_Bom`, params 0) at `pos` with `fuse` frames to go.
fn bomb_at(w: &mut PlayState, pos: Vec3, fuse: i16) -> ActorHandle {
    let h = w.actor_spawn(ACTOR_EN_BOM, pos, [0; 3], 0).unwrap();
    w.actors.downcast_mut::<EnBom>(h).unwrap().timer = fuse;
    h
}

/// Ticks until the skull's action is `a`; the frames it took.
fn until_action(w: &mut PlayState, a: Action, max: usize) -> usize {
    for n in 1..=max {
        idle(w, 1);
        if skull(w).action == a {
            return n;
        }
    }
    panic!("not {a:?} within {max} frames (still {:?}, eyes {:?})", skull(w).action, eyes(w));
}

/// Dust puffs (`EFFECT_SS_DUST`) under the skull's mouth (`dustOffsets`: 200 below it, 290 to 430
/// in front), alive.
fn mouth_dust(w: &PlayState) -> usize {
    w.effect_ss
        .table
        .iter()
        .filter(|e| e.life > -1 && e.ty == EFFECT_SS_DUST && (e.pos.y - (SKULL.y - 200.0)).abs() < 3.0 && e.pos.z > SKULL.z + 280.0 && e.pos.z < SKULL.z + 440.0)
        .count()
}

#[test]
fn before_the_entrance_wall_breaks_the_skull_has_no_jaw_drawn_and_no_colliders() {
    let Some(a) = assets() else { return };
    let mut w = cavern_with(&a, false, |_| {});
    // BgDodoago_Init: scale 0.1, the culling volume (5000, 1000, 800); its flag 0x1A unset:
    // waiting for explosives, the eyes dark.
    let s = skull(&w);
    assert_eq!((s.switch_flag(), s.action, s.actor.scale.x), (SKULL_FLAG, Action::WaitExplosives, 0.1));
    assert_eq!((s.actor.culling_volume_distance, s.actor.culling_volume_scale, s.actor.culling_volume_downward), (5000.0, 1000.0, 800.0));
    assert_eq!(eyes(&w), [0, 0]);
    // No EVENTCHKINF_B0: BgDodoago_WaitExplosives never places its cylinders (Collider_
    // UpdateCylinder) nor registers them; BgDodoago_Draw draws nothing.
    assert_eq!((s.main_collider.dim.pos, s.left_collider.dim.pos, s.right_collider.dim.pos), ([0; 3], [0; 3], [0; 3]));
    assert!(!drawn(&w).iter().any(|m| m == JAW_MESH));
    // The jaw's DynaPoly collision is registered at init whatever the flag (DynaPoly_SetBgActor):
    // closed, its lip stands 204 high in front of the island (z -1300).
    assert!((floor(&w, 0.0, -1300.0) - 204.0).abs() < 0.5, "{}", floor(&w, 0.0, -1300.0));
    // A blast by the right eye lights nothing.
    bomb_at(&mut w, BY_RIGHT_EYE, 4);
    idle(&mut w, 20);
    assert_eq!((eyes(&w), skull(&w).action), ([0, 0], Action::WaitExplosives));
    // The wall broken: placed the next frame, 200 in front for the main one; 215 in front and 90
    // either side for the catcher's, which are 280 up.
    w.save.set_event_chk_inf(EVENTCHKINF_B0);
    idle(&mut w, 1);
    let s = skull(&w);
    assert_eq!((s.main_collider.dim.pos, s.main_collider.dim.radius, s.main_collider.dim.height, s.main_collider.dim.y_shift), ([0, 196, -1550], 80, 30, 80));
    assert_eq!((s.left_collider.dim.pos, s.right_collider.dim.pos), ([90, 196, -1535], [-90, 196, -1535]));
    assert_eq!((s.left_collider.dim.radius, s.left_collider.dim.height, s.left_collider.dim.y_shift), (50, 60, 280));
    assert!(drawn(&w).iter().any(|m| m == JAW_MESH), "{:?}", drawn(&w));
}

#[test]
fn a_blast_by_each_eye_lights_it_and_the_second_opens_the_jaw() {
    let Some(a) = assets() else { return };
    let mut w = cavern(&a);
    let me = w.player;
    // A bomb by the right eye: its blast (Actor_GetCollidedExplosive) on the main cylinder. The
    // yaw from the skull to it is past its facing (0): BGDODOAGO_EYE_RIGHT. The eye dark:
    // one-point 3065 for 40, the chime, lighting it; the first explosive: the skull's parent,
    // sTimer 50.
    let b1 = bomb_at(&mut w, BY_RIGHT_EYE, 4);
    until_action(&mut w, Action::LightOneEye, 20);
    let lit_at = w.audio.frames;
    let s = skull(&w);
    assert_eq!((s.state, s.actor.parent), (BGDODOAGO_EYE_RIGHT as i16, Some(b1)));
    let st = statics(&mut w);
    assert_eq!((st.first_explosive_flag, st.timer), (1, 50));
    let sub = w.active_cam_id;
    assert_ne!(sub, CAM_ID_MAIN);
    assert_eq!(w.camera(sub).unwrap().cs_id, ONEPOINT_EYE);
    assert!((38..=40).contains(&w.camera(sub).unwrap().timer));
    assert_eq!(sfx_frames(&w, NA_SE_SY_CORRECT_CHIME).len(), 1);
    // The cutscene holds Link (PLAYER_CSACTION_8).
    idle(&mut w, 1);
    assert_eq!(w.player().cs_mode, 8);
    // BgDodoago_LightOneEye from the next frame: the glow 5 a frame to 255 (51 frames), then
    // waiting again. sTimer up 1 a frame meanwhile (BgDodoago_Update: a parent).
    assert_eq!(eyes(&w), [0, 5]);
    let n = until_action(&mut w, Action::WaitExplosives, 60);
    assert_eq!((n, eyes(&w)), (50, [0, 255]));
    let since = (w.audio.frames - lit_at) as i32;
    assert_eq!(statics(&mut w).timer, 50 + since);
    // The same side again: 3065 for 20, NA_SE_SY_ERROR, sTimer 30 more, still waiting. The blast's
    // sphere stays on the cylinder for some frames: an error every other frame (the error's
    // `return` skips registering the cylinders that frame, so the next hit is a frame later).
    let b2 = bomb_at(&mut w, BY_RIGHT_EYE, 4);
    let mut errored = false;
    for _ in 0..20 {
        idle(&mut w, 1);
        if !sfx_frames(&w, NA_SE_SY_ERROR).is_empty() {
            errored = true;
            break;
        }
    }
    assert!(errored, "NA_SE_SY_ERROR");
    assert_eq!(w.camera(w.active_cam_id).unwrap().cs_id, ONEPOINT_EYE);
    assert_eq!((skull(&w).action, eyes(&w)), (Action::WaitExplosives, [0, 255]));
    while w.actors.actor(b2).is_some_and(|b| !b.killed) {
        idle(&mut w, 1);
    }
    idle(&mut w, 2);
    let errors = sfx_frames(&w, NA_SE_SY_ERROR);
    assert!(errors.windows(2).all(|p| p[1] == p[0] + 2), "{errors:?}");
    // The parent let go once sTimer passed 140 (the flag unset); sTimer stays where it is after.
    assert_eq!(skull(&w).actor.parent, None);
    let t = statics(&mut w).timer;
    assert!(t > 140, "{t}");
    // Wait for the cutscenes to end.
    while w.active_cam_id != CAM_ID_MAIN {
        idle(&mut w, 1);
    }
    assert_eq!(statics(&mut w).timer, t);
    // The other side: switch flag 0x1A, state 0, the chime, opening the jaw, one-point 3380 for
    // 160.
    bomb_at(&mut w, BY_LEFT_EYE, 4);
    let n = until_action(&mut w, Action::OpenJaw, 20);
    assert!(n <= 10, "{n}");
    let blast = w.audio.frames;
    assert!(w.flags.get_switch(SKULL_FLAG));
    assert_eq!(skull(&w).state, 0);
    assert_eq!(sfx_frames(&w, NA_SE_SY_CORRECT_CHIME).last(), Some(&blast));
    assert_eq!(w.camera(w.active_cam_id).unwrap().cs_id, ONEPOINT_JAW);
    // BgDodoago_OpenJaw: the left eye up 5 a frame from the next (51 frames to 255), sTimer down 1
    // a frame meanwhile (no parent: nothing counts it up).
    let n = until_action_eyes(&mut w, [255, 255], 60);
    assert_eq!(n, 51);
    assert_eq!(statics(&mut w).timer, t - 50);
    // Then the jaw: Math_SmoothStepToS(rot.x, 0x1333, 110 - state, 1000, 50), state up 3 a frame
    // to 100 (the scale 107 down to 10); the frame its step starts from 0x1333 it's done,
    // NA_SE_EV_STONE_BOUND. (The frames from the C's numbers: `jaw_frames`.)
    assert_eq!(skull(&w).actor.shape_rot.x, jaw_rot_after(1));
    let n = until_action(&mut w, Action::DoNothing, 200);
    assert_eq!(n + 1, jaw_frames());
    assert_eq!(skull(&w).actor.shape_rot.x, JAW_OPEN);
    assert_eq!(sfx_frames(&w, NA_SE_EV_STONE_BOUND), vec![w.audio.frames]);
    // sTimer frozen (no parent) at its value when the eyes were both lit, not 108: no dust under
    // the mouth (@bug (game)).
    assert_ne!(t - 50, 108);
    assert_eq!(mouth_dust(&w), 0);
    let open_at = w.audio.frames;
    eprintln!("jaw open {} frames after the second eye's blast", open_at - blast);
    assert_eq!((open_at - blast) as usize, 51 + jaw_frames() - 1);
    // Link free when 3380's camera ends (160 frames on).
    let mut free_at = None;
    for _ in 0..200 {
        idle(&mut w, 1);
        if w.active_cam_id == CAM_ID_MAIN && w.player().cs_mode != 8 {
            free_at = Some(w.audio.frames);
            break;
        }
    }
    let free_at = free_at.expect("Link let go");
    eprintln!("Link free {} frames after the second eye's blast", free_at - blast);
    assert!((155..=165).contains(&(free_at - blast)), "{}", free_at - blast);
    // The jaw open, its collision: a ramp from the island's floor (4 at z -1375) up to 75 (z
    // -1600), level to the door's floor (76, door 7 at (0, 76, -1970)): no step over 10 a 25.
    let mut prev = floor(&w, 0.0, -1350.0);
    assert_eq!(prev, 0.0);
    let mut z = -1375.0;
    while z >= -1975.0 {
        let f = floor(&w, 0.0, z);
        assert!((f - prev).abs() <= 10.0, "z {z}: {prev} to {f}");
        prev = f;
        z -= 25.0;
    }
    assert_eq!(prev, 76.0);
    let _ = me;
}

/// Ticks until the eyes' glow is `want`; the frames it took.
fn until_action_eyes(w: &mut PlayState, want: [i16; 2], max: usize) -> usize {
    for n in 1..=max {
        idle(w, 1);
        if eyes(w) == want {
            return n;
        }
    }
    panic!("eyes {:?}", eyes(w));
}

/// `BgDodoago_OpenJaw`'s jaw after `n` frames of it with both eyes lit (from rot.x 0, state 0).
fn jaw_rot_after(n: usize) -> i16 {
    let (mut rot, mut state) = (0i16, 0i16);
    for _ in 0..n {
        eng_math::step_to_s(&mut state, 100, 3);
        eng_math::smooth_step_to_s(&mut rot, JAW_OPEN, 110 - state, 0x3E8, 0x32);
    }
    rot
}

/// The frames of `BgDodoago_OpenJaw` with both eyes lit until it's done (its step's start at
/// 0x1333: `Math_SmoothStepToS` returns 0).
fn jaw_frames() -> usize {
    let (mut rot, mut state) = (0i16, 0i16);
    for n in 1.. {
        eng_math::step_to_s(&mut state, 100, 3);
        if eng_math::smooth_step_to_s(&mut rot, JAW_OPEN, 110 - state, 0x3E8, 0x32) == 0 {
            return n;
        }
    }
    unreachable!()
}

#[test]
fn link_walks_from_the_island_up_the_open_jaw_to_door_7() {
    let Some(a) = assets() else { return };
    // The skull's flag saved: open at init (rot.x 0x1333), both eyes 255, doing nothing.
    let mut w = cavern_with(&a, true, |s| s.scene_flags[SCENE_DODONGOS_CAVERN].swch |= 1 << SKULL_FLAG);
    let s = skull(&w);
    assert_eq!((s.action, s.actor.shape_rot.x), (Action::DoNothing, JAW_OPEN));
    assert_eq!(eyes(&w), [255, 255]);
    // From the island's south end (y 0) north up the jaw to the door's floor.
    w.place_player(Vec3::new(0.0, 0.0, -1300.0), i16::MIN);
    idle(&mut w, 2);
    let target = Vec3::new(0.0, 76.0, -1940.0);
    let mut prev = PadState::default();
    let mut reached = None;
    for n in 0..200 {
        let pad = oot_actors::script::stick_towards(&w, target, 60.0);
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        let p = w.player().actor.world_pos;
        assert!(p.y > -20.0, "fell at {p}");
        if p.z < -1900.0 {
            reached = Some((n, p));
            break;
        }
    }
    let (n, p) = reached.expect("at the door");
    eprintln!("at the door's floor after {n} frames: {p}");
    assert!((p.y - 76.0).abs() < 1.0, "{p}");
}

#[test]
fn the_bomb_catcher_stops_a_bomb_dropped_through_a_hole_with_fuse_50() {
    let Some(a) = assets() else { return };
    let mut w = cavern(&a);
    // A bomb (params 0) dropped through the bridge's right hole, still moving along: falling
    // through the right catcher cylinder (x 90, y 476 to 536): BgDodoago_Update with no parent:
    // fuse 50, speed 0, the skull's parent, sTimer 0.
    let b = bomb_at(&mut w, HOLE_RIGHT, 70);
    {
        let bomb = w.actors.downcast_mut::<EnBom>(b).unwrap();
        bomb.actor.speed_xz = 3.0;
        bomb.actor.world_rot.y = 0x4000;
    }
    let mut caught = false;
    for _ in 0..30 {
        idle(&mut w, 1);
        if skull(&w).actor.parent == Some(b) {
            caught = true;
            break;
        }
    }
    assert!(caught, "caught");
    let bomb = w.actors.downcast::<EnBom>(b).unwrap();
    let y = bomb.actor.world_pos.y;
    assert!(y < 536.0 + 15.0 && y > 400.0, "y {y}");
    // Set in the skull's turn (ACTORCAT_BG), before the bomb's own (ACTORCAT_EXPLOSIVE) counts it
    // down: 49 at the frame's end.
    assert_eq!((bomb.timer, bomb.actor.speed_xz), (49, 0.0));
    assert_eq!(statics(&mut w).timer, 0);
    idle(&mut w, 1);
    assert_eq!(w.actors.downcast::<EnBom>(b).unwrap().timer, 48);
    assert_eq!(statics(&mut w).timer, 1);
    // It lands on the skull's head by the right eye and blows: the eye lit, the bomb the first
    // explosive (the parent already, sTimer 50).
    until_action(&mut w, Action::LightOneEye, 60);
    assert_eq!(skull(&w).state, BGDODOAGO_EYE_RIGHT as i16);
    assert_eq!(statics(&mut w).timer, 50);
}

#[test]
fn the_bomb_catcher_lets_a_bomb_flower_s_bomb_fall() {
    let Some(a) = assets() else { return };
    let mut w = cavern(&a);
    // A Bomb Flower's bomb (En_Bombf params 0) through the left hole: ACTORCAT_EXPLOSIVE but not
    // ACTOR_EN_BOM: not caught, its fuse runs on.
    let h = w.actor_spawn(ACTOR_EN_BOMBF, HOLE_LEFT, [0; 3], 0).unwrap();
    let mut fuses = Vec::new();
    for _ in 0..30 {
        idle(&mut w, 1);
        fuses.push(w.actors.downcast::<EnBombf>(h).unwrap().timer);
    }
    assert!(fuses.windows(2).all(|p| p[1] == p[0] - 1), "{fuses:?}");
    assert_eq!(skull(&w).actor.parent, None);
    // It lies on the skull's head by the left eye, and its own blast lights it: any
    // ACTORCAT_EXPLOSIVE does.
    let y = w.actors.actor(h).unwrap().world_pos.y;
    assert!((y - 308.0).abs() < 20.0, "on the head: {y}");
    w.actors.downcast_mut::<EnBombf>(h).unwrap().timer = 2;
    until_action(&mut w, Action::LightOneEye, 30);
    assert_eq!(skull(&w).state, BGDODOAGO_EYE_LEFT as i16);
}

#[test]
fn the_catcher_s_parent_is_let_go_past_140_and_kept_for_good_once_the_jaw_opens() {
    let Some(a) = assets() else { return };
    let mut w = cavern(&a);
    let b = bomb_at(&mut w, HOLE_RIGHT, 70);
    for _ in 0..30 {
        idle(&mut w, 1);
        if skull(&w).actor.parent.is_some() {
            break;
        }
    }
    assert_eq!(skull(&w).actor.parent, Some(b));
    // Its fuse runs out and it lights the right eye (sTimer 50 again, the first explosive); 91
    // frames on from there sTimer passes 140: the flag unset, the parent let go, the catcher on
    // again (sDisableBombCatcher still 0).
    until_action(&mut w, Action::LightOneEye, 60);
    let n = {
        let mut n = 0;
        while skull(&w).actor.parent.is_some() {
            idle(&mut w, 1);
            n += 1;
        }
        n
    };
    assert_eq!(n, 91);
    let st = statics(&mut w);
    assert_eq!((st.timer, st.disable_bomb_catcher), (141, 0));
}

#[test]
fn the_dust_under_the_mouth_comes_on_the_frame_s_timer_is_108() {
    let Some(a) = assets() else { return };
    let mut w = cavern(&a);
    // The right eye lit already (drawParams[1] 255), the first explosive seen, the skull holding a
    // parent with sTimer 100: the left eye's blast opens the jaw.
    w.scene.as_mut().unwrap().draw.room_draw_params[BGDODOAGO_EYE_RIGHT] = 255;
    {
        let st = statics(&mut w);
        st.first_explosive_flag = 1;
        st.timer = 99;
    }
    let me = w.player;
    let h = skull_h(&w);
    w.actors.downcast_mut::<BgDodoago>(h).unwrap().actor.parent = me;
    bomb_at(&mut w, BY_LEFT_EYE, 4);
    until_action(&mut w, Action::OpenJaw, 20);
    let t0 = statics(&mut w).timer;
    // While the left eye fills, BgDodoago_Update counts sTimer up and BgDodoago_OpenJaw down:
    // it holds; then it counts up, and on 108 eleven groups of four puffs (func_80033480) under
    // the mouth, and sparkles from both eyes every frame (EffectSsKiraKira_SpawnSmall, 4 each).
    until_action_eyes(&mut w, [255, 255], 60);
    let t = statics(&mut w).timer;
    assert_eq!(t, t0 + 1);
    assert!(w.effect_ss.table.iter().any(|e| e.life > -1 && e.ty == EFFECT_SS_KIRAKIRA));
    assert!(t < 108);
    let mut dust_at = None;
    for _ in 0..20 {
        let before = mouth_dust(&w);
        idle(&mut w, 1);
        if mouth_dust(&w) > before {
            dust_at = Some(statics(&mut w).timer);
            assert_eq!(mouth_dust(&w) - before, 44);
            break;
        }
    }
    assert_eq!(dust_at, Some(108));
    // Past 140 with the flag set: sDisableBombCatcher, the parent kept for good.
    while statics(&mut w).timer <= 141 {
        idle(&mut w, 1);
    }
    assert_eq!(statics(&mut w).disable_bomb_catcher, 1);
    assert_eq!(skull(&w).actor.parent, me);
}

#[test]
fn a_bomb_flower_s_blast_lights_an_eye_too() {
    let Some(a) = assets() else { return };
    let mut w = cavern(&a);
    // A Bomb Flower's bomb (En_Bombf params 0) about to blow by the right eye.
    let h = w.actor_spawn(ACTOR_EN_BOMBF, BY_RIGHT_EYE, [0; 3], 0).unwrap();
    idle(&mut w, 5);
    w.actors.downcast_mut::<EnBombf>(h).unwrap().timer = 2;
    until_action(&mut w, Action::LightOneEye, 30);
    assert_eq!(skull(&w).state, BGDODOAGO_EYE_RIGHT as i16);
    assert_eq!(skull(&w).actor.parent, Some(h));
}

#[test]
fn the_statics_go_with_the_last_skull() {
    let Some(a) = assets() else { return };
    let mut w = cavern(&a);
    bomb_at(&mut w, BY_RIGHT_EYE, 4);
    until_action(&mut w, Action::LightOneEye, 20);
    assert_eq!(statics(&mut w).first_explosive_flag, 1);
    // Room 9 (the upper level's east room): room 0's actors go; the skull's overlay unloads
    // (Actor_FreeOverlay) and comes back with sFirstExplosiveFlag false.
    assert!(w.room_request(9));
    idle(&mut w, 1);
    w.room_change_done();
    idle(&mut w, 3);
    assert!(w.actors.all().into_iter().all(|h| w.actors.downcast::<BgDodoago>(h).is_none()));
    assert!(!w.overlay_statics.contains_key(&ACTOR_BG_DODOAGO));
}

#[test]
fn the_upper_level_s_bomb_flower_can_t_be_carried_to_a_hole_in_its_fuse() {
    use eng_input::pad::BTN_A;
    use oot_actors::player::Action as PA;
    let Some(a) = assets() else { return };
    // The three boulders round it (Obj_Bombiwa 0x0015) gone: their flag 0x15 saved.
    let mut w = cavern_with(&a, true, |s| s.scene_flags[SCENE_DODONGOS_CAVERN].swch |= 1 << 0x15);
    let flower = Vec3::new(850.0, 531.0, -923.0);
    w.place_player(flower + Vec3::new(32.0, 0.0, 0.0), -0x4000);
    idle(&mut w, 2);
    w.tick_with(scripted_input(PadState::default(), PadState { button: BTN_A, ..Default::default() }));
    let mut lift = 1;
    let body = loop {
        idle(&mut w, 1);
        lift += 1;
        let held = w.player().held_actor.filter(|&h| w.actors.downcast::<EnBombf>(h).is_some_and(|b| b.actor.params == 0));
        if let Some(h) = held
            && w.player().action == PA::StandingStill
        {
            break h;
        }
        assert!(lift < 80, "no bomb in hand");
    };
    // EnBombf: its bomb's fuse 140 from its spawn in the lift.
    let fuse_at_run = w.actors.downcast::<EnBombf>(body).unwrap().timer;
    // Its run carrying it, west along the upper level (the boulders' places) for 30 frames.
    let start = w.player().actor.world_pos;
    let toward = Vec3::new(600.0, 531.0, -923.0);
    let mut prev = PadState::default();
    let mut track = Vec::new();
    for _ in 0..30 {
        let pad = oot_actors::script::stick_towards(&w, toward, 60.0);
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        track.push(w.player().actor.world_pos);
    }
    assert_eq!(w.player().held_actor, Some(body), "still carrying");
    // Its top speed carrying it over the last 15 (`speed` 4.05, moved 1.5 times that a frame by
    // Actor_UpdatePos at R_UPDATE_RATE 3).
    let speed = track[29].distance(track[14]) / 15.0;
    assert!((speed - w.player().actor.speed_xz * 1.5).abs() < 0.01, "{speed}");
    // A lower bound for the carry: from where it starts running to the right hole's nearest
    // corner (100, -1500) as the crow flies, ignoring the pit, the walls and the climb to the
    // bridge (y 591), at that speed.
    let way = Vec3::new(start.x, 0.0, start.z).distance(Vec3::new(100.0, 0.0, -1500.0));
    let frames = way / speed;
    eprintln!("lift {lift} frames, fuse {fuse_at_run} at the run, run speed {speed:.2}, way {way:.0}: {frames:.0} frames at least");
    assert!(frames > fuse_at_run as f32, "it could: {frames} frames, fuse {fuse_at_run}");
}
