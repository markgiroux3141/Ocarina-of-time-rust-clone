//! GAME-09 milestone 3 against the C: `Bg_Ddan_Jd` (`z_bg_ddan_jd.c`), Dodongo's Cavern's rising
//! pillars. Master Quest's lobby (`ddan` room 0) has three: 0x00FF at (-420, -180, -1180) and (0,
//! -180, -780), and the shortcut 0x000A at (420, -180, -1180).
//!
//! Entered by `ENTR_DODONGOS_CAVERN_0` on a `cavern-open` save with the entrance wall broken
//! (`EVENTCHKINF_B0`, switch 7). Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_ddan_jd::*;
use oot_game::actor_ctx::ActorHandle;
use oot_game::camera::CAM_ID_MAIN;
use oot_game::effect::EFFECT_SS_DUST;
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
/// The shortcut's switch flag (its params 0x000A) and its home.
const SHORTCUT_FLAG: i32 = 0x0A;
const SHORTCUT_HOME: Vec3 = Vec3::new(420.0, -180.0, -1180.0);
const PLAIN_HOME: Vec3 = Vec3::new(-420.0, -180.0, -1180.0);

/// Child Link in the cavern's lobby (`cavern-open`, the entrance wall broken, Navi's word about
/// the pit heard), `prepare` run on the save first, on the lobby's floor by the skull, three frames
/// on, the audio logged.
fn cavern_with(a: &Arc<GameAssets>, prepare: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DODONGOS_CAVERN_0").expect("entrance");
    let mut save = SaveContext::new(e, false, clock_time(10, 0) as u16);
    save.apply_preset("cavern-open").unwrap();
    save.set_event_chk_inf(oot_actors::bg_dodoago::EVENTCHKINF_B0);
    save.scene_flags[SCENE_DODONGOS_CAVERN].swch |= 1 << 7;
    // Navi's word about the pit (Elf_Msg 0x0C15 over it) heard: its switch flag 0x0C.
    save.scene_flags[SCENE_DODONGOS_CAVERN].swch |= 1 << 0x0C;
    prepare(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    w.audio.log = Some(Default::default());
    w.place_player(Vec3::new(0.0, 0.0, -1100.0), i16::MIN);
    idle(&mut w, 3);
    w
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn pillar_at(w: &PlayState, home: Vec3) -> ActorHandle {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<BgDdanJd>(h).is_some_and(|p| p.actor.home_pos.distance(home) < 1.0)).expect("the pillar")
}

fn pillar(w: &PlayState, h: ActorHandle) -> &BgDdanJd {
    w.actors.downcast::<BgDdanJd>(h).unwrap()
}

fn sfx_frames(w: &PlayState, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, _)| f).collect()
}

fn dust(w: &PlayState) -> usize {
    w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == EFFECT_SS_DUST).count()
}

/// Ticks until the pillar's action is `a` (at most `max` frames); the frames it took.
fn until(w: &mut PlayState, h: ActorHandle, max: usize, f: impl Fn(&BgDdanJd) -> bool) -> usize {
    for n in 1..=max {
        idle(w, 1);
        if f(pillar(w, h)) {
            return n;
        }
    }
    panic!("not within {max} frames: {:?} y {}", pillar(w, h).action, pillar(w, h).actor.world_pos.y);
}

#[test]
fn a_plain_pillar_idles_100_frames_and_moves_140_up_and_down_at_1() {
    let Some(a) = assets() else { return };
    let mut w = cavern_with(&a, |_| {});
    let h = pillar_at(&w, PLAIN_HOME);
    let p = pillar(&w, h);
    // BgDdanJd_Init: scale 0.1, idleTimer 100, STATE_GO_BOTTOM, ySpeed 1 (flag 0xFF reads 0x3F,
    // unset); idling. Updated once a frame since: its timer counts down.
    assert_eq!((p.actor.scale.x, p.state, p.y_speed, p.action), (0.1, STATE_GO_BOTTOM, DEFAULT_Y_SPEED, Action::Idle));
    assert_eq!(p.actor.world_pos.y, -180.0);
    let left = p.idle_timer as usize;
    assert!(left > 0 && left < 100);
    // BgDdanJd_Idle: the timer reaches 0 on its `left`th update: 100 again, STATE_GO_MIDDLE_FROM_
    // BOTTOM, the target home + 140, moving.
    let n = until(&mut w, h, 200, |p| p.action == Action::Move);
    assert_eq!(n, left);
    let p = pillar(&w, h);
    assert_eq!((p.state, p.idle_timer, p.target_y), (STATE_GO_MIDDLE_FROM_BOTTOM, IDLE_FRAMES, -40.0));
    // BgDdanJd_Move: 1 a frame (Math_StepToF): there on the 140th, NA_SE_EV_PILLAR_MOVE_STOP,
    // idling. Its dust (two puffs a frame at its home's height) meanwhile.
    idle(&mut w, 1);
    assert_eq!(pillar(&w, h).actor.world_pos.y, -179.0);
    assert!(dust(&w) >= 2);
    let start = w.audio.frames;
    let n = until(&mut w, h, 300, |p| p.action == Action::Idle);
    assert_eq!(n, 139);
    assert_eq!(pillar(&w, h).actor.world_pos.y, -40.0);
    // (All three pillars started together: three stops on the one frame.)
    let stops: Vec<u32> = sfx_frames(&w, NA_SE_EV_PILLAR_MOVE_STOP).into_iter().filter(|&f| f > start).collect();
    assert_eq!(stops.len(), 3);
    assert!(stops.iter().all(|&f| f == stops[0]));
    // No NA_SE_EV_ELEVATOR_MOVE (ySpeed 1).
    assert_eq!(pillar(&w, h).actor.sfx, 0);
    // 100 frames at the middle, then down to its home at 1 (140 frames), STATE_GO_BOTTOM.
    let n = until(&mut w, h, 200, |p| p.action == Action::Move);
    assert_eq!(n, 100);
    assert_eq!((pillar(&w, h).state, pillar(&w, h).target_y), (STATE_GO_BOTTOM, -180.0));
    let n = until(&mut w, h, 300, |p| p.action == Action::Idle);
    assert_eq!((n, pillar(&w, h).actor.world_pos.y), (140, -180.0));
    // Its collision carries what stands on it (DYNA_TRANSFORM_POS): the floor over it is its
    // top, 40 above it.
    let (f, _) = w.col.entity_raycast_down(PLAIN_HOME + Vec3::new(0.0, 300.0, 0.0));
    assert_eq!(f, -140.0);
}

#[test]
fn flag_0x3f_makes_a_0x00ff_pillar_the_shortcut_only_at_init() {
    let Some(a) = assets() else { return };
    let mut w = cavern_with(&a, |_| {});
    let h = pillar_at(&w, PLAIN_HOME);
    // Flags_GetSwitch(play, 0xFF): 1 << (0xFF - 0x20), the shift masked to 5 bits on the console:
    // temp switch flag 0x3F. Set while it runs, nothing (BgDdanJd_Idle and _Move check
    // params < 0x40).
    w.flags.set_switch(0x3F);
    assert!(w.flags.get_switch(0xFF));
    idle(&mut w, 2);
    assert_eq!(pillar(&w, h).y_speed, DEFAULT_Y_SPEED);
    assert_eq!(w.active_cam_id, CAM_ID_MAIN);
    // Spawned with it set (BgDdanJd_Init's missing check): the shortcut's speed.
    let n = w.actor_spawn(ACTOR_BG_DDAN_JD, Vec3::new(-420.0, -180.0, -900.0), [0; 3], 0x00FF).unwrap();
    assert_eq!(pillar(&w, n).y_speed, SHORTCUT_Y_SPEED);
}

#[test]
fn switch_0x0a_makes_the_pillar_the_shortcut_to_700_up() {
    let Some(a) = assets() else { return };
    let mut w = cavern_with(&a, |_| {});
    let h = pillar_at(&w, SHORTCUT_HOME);
    assert_eq!((pillar(&w, h).y_speed, pillar(&w, h).action), (DEFAULT_Y_SPEED, Action::Idle));
    // The boulder's floor switch (0x0A) pressed while it idles at the bottom: BgDdanJd_Idle, ySpeed
    // 5, 140 up at once, STATE_GO_MIDDLE_FROM_BOTTOM with no wait; one-point 3060 (-99) on it; the
    // same update, the wait over, STATE_GO_TOP to home + 700 (520), moving.
    w.flags.set_switch(SHORTCUT_FLAG);
    idle(&mut w, 1);
    let p = pillar(&w, h);
    assert_eq!((p.y_speed, p.state, p.target_y, p.idle_timer, p.action), (SHORTCUT_Y_SPEED, STATE_GO_TOP, 520.0, IDLE_FRAMES, Action::Move));
    assert_eq!(p.actor.world_pos.y, -40.0);
    let sub = w.active_cam_id;
    assert_ne!(sub, CAM_ID_MAIN);
    assert_eq!(w.camera(sub).unwrap().cs_id, ONEPOINT_SHORTCUT);
    // Rising at 5 with NA_SE_EV_ELEVATOR_MOVE (Actor_PlaySfx_Flagged) and its dust: 560 in 112
    // frames.
    idle(&mut w, 1);
    assert_eq!(pillar(&w, h).actor.world_pos.y, -35.0);
    assert_eq!(pillar(&w, h).actor.sfx, NA_SE_EV_ELEVATOR_MOVE - oot_game::audio::sfx::SFX_FLAG);
    let n = until(&mut w, h, 200, |p| p.action == Action::Idle);
    assert_eq!((n, pillar(&w, h).actor.world_pos.y), (111, 520.0));
    // At the top 100 frames, then down to the middle (STATE_GO_MIDDLE_FROM_TOP, 112 frames), 100
    // there, and up again (STATE_GO_TOP): never to the bottom. The cycle: 424 frames, 100 of them
    // idling at the top.
    let n = until(&mut w, h, 200, |p| p.action == Action::Move);
    assert_eq!((n, pillar(&w, h).state, pillar(&w, h).target_y), (100, STATE_GO_MIDDLE_FROM_TOP, -40.0));
    let n = until(&mut w, h, 200, |p| p.action == Action::Idle);
    assert_eq!((n, pillar(&w, h).actor.world_pos.y), (112, -40.0));
    let n = until(&mut w, h, 200, |p| p.action == Action::Move);
    assert_eq!((n, pillar(&w, h).state, pillar(&w, h).target_y), (100, STATE_GO_TOP, 520.0));
    let n = until(&mut w, h, 200, |p| p.action == Action::Idle);
    assert_eq!((n, pillar(&w, h).actor.world_pos.y), (112, 520.0));
    // At the top its floor is 40 above it: 560.
    let (f, _) = w.col.entity_raycast_down(SHORTCUT_HOME + Vec3::new(0.0, 800.0, 0.0));
    assert_eq!(f, 560.0);
}

#[test]
fn switch_0x0a_set_while_it_moves_jumps_it_to_the_middle() {
    let Some(a) = assets() else { return };
    let mut w = cavern_with(&a, |_| {});
    let h = pillar_at(&w, SHORTCUT_HOME);
    until(&mut w, h, 200, |p| p.action == Action::Move);
    idle(&mut w, 10);
    assert_eq!(pillar(&w, h).actor.world_pos.y, -170.0);
    // BgDdanJd_Move: ySpeed 5, STATE_GO_MIDDLE_FROM_BOTTOM, 140 up at once, no wait, idling,
    // one-point 3060; the next update's idle sends it to the top.
    w.flags.set_switch(SHORTCUT_FLAG);
    idle(&mut w, 1);
    let p = pillar(&w, h);
    assert_eq!((p.y_speed, p.state, p.idle_timer, p.action, p.actor.world_pos.y), (SHORTCUT_Y_SPEED, STATE_GO_MIDDLE_FROM_BOTTOM, 0, Action::Idle, -40.0));
    assert_eq!(w.camera(w.active_cam_id).unwrap().cs_id, ONEPOINT_SHORTCUT);
    idle(&mut w, 1);
    let p = pillar(&w, h);
    assert_eq!((p.state, p.target_y, p.action), (STATE_GO_TOP, 520.0, Action::Move));
}

#[test]
fn entered_with_switch_0x0a_it_rises_to_the_middle_then_shuttles_to_the_top() {
    let Some(a) = assets() else { return };
    let mut w = cavern_with(&a, |s| s.scene_flags[SCENE_DODONGOS_CAVERN].swch |= 1 << SHORTCUT_FLAG);
    let h = pillar_at(&w, SHORTCUT_HOME);
    // BgDdanJd_Init with its flag set: ySpeed 5, from the bottom: up to the middle (28 frames at
    // 5), 100 there, then to the top; no one-point (only the change starts it).
    assert_eq!((pillar(&w, h).y_speed, pillar(&w, h).state), (SHORTCUT_Y_SPEED, STATE_GO_BOTTOM));
    until(&mut w, h, 200, |p| p.action == Action::Move);
    assert_eq!(pillar(&w, h).target_y, -40.0);
    let n = until(&mut w, h, 200, |p| p.action == Action::Idle);
    assert_eq!(n, 28);
    until(&mut w, h, 200, |p| p.action == Action::Move);
    assert_eq!((pillar(&w, h).state, pillar(&w, h).target_y), (STATE_GO_TOP, 520.0));
    assert_eq!(w.active_cam_id, CAM_ID_MAIN);
}

#[test]
fn at_its_top_the_shortcut_is_29_above_the_upper_level_and_170_short_of_it() {
    let Some(a) = assets() else { return };
    let mut w = cavern_with(&a, |_| {});
    let h = pillar_at(&w, SHORTCUT_HOME);
    w.flags.set_switch(SHORTCUT_FLAG);
    until(&mut w, h, 200, |p| p.action == Action::Idle && p.actor.world_pos.y == 520.0);
    let floor = |w: &PlayState, x: f32, z: f32| w.col.entity_raycast_down(Vec3::new(x, 900.0, z)).0;
    // Its top (40 above it: 560) is 120 across, x 360 to 480 and z -1120 to -1240.
    for (x, z) in [(365.0, -1180.0), (475.0, -1180.0), (420.0, -1125.0), (420.0, -1235.0)] {
        assert_eq!(floor(&w, x, z), 560.0, "({x}, {z})");
    }
    assert_eq!(floor(&w, 490.0, -1180.0), -180.0);
    // East of it at z -1180: the pit (-180) to x 600, the lobby's floor (0, under the upper level's
    // overhang) from 605, the upper level (531) from 660: 29 below the top, 170 on from its edge.
    assert!(floor(&w, 595.0, -1180.0) < -150.0);
    assert_eq!(floor(&w, 640.0, -1180.0), 0.0);
    assert_eq!(floor(&w, 660.0, -1180.0), 531.0);
    // Link running east off it (its auto-jump) gets about 60 on before he's below 531: he lands on
    // the lobby's floor under the upper level.
    w.place_player(Vec3::new(470.0, 560.0, -1180.0), 0x4000);
    idle(&mut w, 2);
    let mut prev = PadState::default();
    for _ in 0..50 {
        let pad = oot_actors::script::stick_towards(&w, Vec3::new(800.0, 560.0, -1180.0), 60.0);
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
    }
    let p = w.player().actor.world_pos;
    assert_eq!(p.y, 0.0, "{p}");
    assert!(p.x > 650.0, "{p}");
}
