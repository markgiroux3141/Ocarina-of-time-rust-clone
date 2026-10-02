//! Kokiri Forest's crawlspace (GAME-03 milestone 2): Player's crawl (`Player_TryEnteringCrawlspace`,
//! `Player_Action_8084C760`, `Player_TryLeavingCrawlspace`, `Player_Action_8084C81C` in `z_player.c`) and the crawlspace's
//! camera (`Camera_Subj4`, `CAM_SET_CRAWLSPACE`).
//!
//! Expected values come from the scene data and the C:
//! - the mouth is two triangles of wall at z 1059, x -801..-769, y 120..144, facing -z, wall
//!   type 5 (`WALL_FLAG_CRAWLSPACE_1`): their middle is x -785, `wallYaw` 0x8000;
//! - inside, the tunnel's floor (x -801..-769, z 1059..1379) names bg camera 9, `CAM_SET_CRAWLSPACE`
//!   with six points along x -784 at y 132, z 1037 (points 0..2) and 1401 (3..5); the walls at
//!   z 1079 (facing +z) and z 1359 (facing -z) are `WALL_FLAG_CRAWLSPACE_1` too;
//! - `En_Holl` 0 (params 0x013F, kind 4) at z 1219 joins rooms 0 and 2;
//! - past the far end, room 2's floor names bg camera 14 (`CAM_SET_DUNGEON0`).

mod common;

use std::sync::Arc;

use eng_input::pad::{BTN_A, PadState};
use glam::{Mat4, Vec3};
use oot_actors::PlayExt;
use oot_actors::player::{Action, STATE2_16, STATE2_18};
use oot_game::camera::{CAM_SET_CRAWLSPACE, CAM_SET_DUNGEON0, CAM_SET_NORMAL0};
use oot_game::interface::DO_ACTION_ENTER;
use oot_game::play::{DrawOut, PlayState, ViewInfo, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Kokiri Forest (`ENTR_KOKIRI_FOREST_0`) on a new save, `adult` or not, with Link placed in front of
/// the crawlspace's mouth at `at`, facing +z.
fn at_the_mouth(a: &Arc<GameAssets>, adult: bool, at: Vec3) -> Option<(PlayState, PadState)> {
    let e = a.scenes.entrance_index("ENTR_KOKIRI_FOREST_0").expect("entrance");
    let save = SaveContext::new(e, adult, oot_game::env::clock_time(10, 0) as u16);
    let mut w = oot_actors::play_entrance(a.clone(), common::data()?, common::rules()?, save).expect("Play_Init");
    let mut prev = PadState::default();
    for _ in 0..20 {
        tick(&mut w, &mut prev, PadState::default());
    }
    w.place_player(at, 0);
    Some((w, prev))
}

fn tick(w: &mut PlayState, prev: &mut PadState, pad: PadState) {
    w.tick_with(scripted_input(*prev, pad));
    *prev = pad;
}

const FORWARD: PadState = PadState { button: 0, stick_x: 0, stick_y: 60 };
const BACK: PadState = PadState { button: 0, stick_x: 0, stick_y: -60 };
const A_FORWARD: PadState = PadState { button: BTN_A, stick_x: 0, stick_y: 60 };

/// Forward into the mouth until "Enter" shows, then A (with the stick still forward, so Link
/// stays in `Player_Action_80842180`, whose interrupts include `Player_ActionHandler_5`).
fn enter(w: &mut PlayState, prev: &mut PadState) {
    for _ in 0..60 {
        if w.player().state2 & STATE2_16 != 0 {
            break;
        }
        tick(w, prev, FORWARD);
    }
    assert!(w.player().state2 & STATE2_16 != 0, "PLAYER_STATE2_DO_ACTION_ENTER at the mouth");
    tick(w, prev, A_FORWARD);
}

/// Whether the last frame drew Link's model.
fn link_drawn(w: &PlayState) -> bool {
    let f = w.current_frame();
    let mut out = DrawOut::default();
    w.draw(&f, &ViewInfo::new(f.view.eye, Mat4::IDENTITY), &mut out);
    out.opa.iter().any(|c| c.mesh.name.starts_with("player/"))
}

#[test]
fn into_the_crawlspace_through_it_and_out() {
    let Some(a) = assets() else { return };
    let Some((mut w, mut prev)) = at_the_mouth(&a, false, Vec3::new(-785.0, 120.0, 1000.0)) else { return };
    // Up to the mouth: Player_ActionHandler_5 sees the wall (facing it, BGCHECKFLAG_PLAYER_WALL_INTERACT)
    // and Player_TryEnteringCrawlspace finds Link within 8 of its triangles' middle: "Enter" on A.
    for _ in 0..60 {
        if w.player().state2 & STATE2_16 != 0 {
            break;
        }
        tick(&mut w, &mut prev, FORWARD);
    }
    assert!(w.player().state2 & STATE2_16 != 0);
    tick(&mut w, &mut prev, FORWARD);
    assert_eq!(w.interface_ctx.unk_1f0, DO_ACTION_ENTER, "A says Enter");
    let wall_distance = w.player().wall_distance;
    // A: Player_SetupWaitForPutAway(func_8083A40C) with PLAYER_STATE2_CRAWLING, Link at the middle plus
    // wallDistance along the wall's normal (0, 0, -1), facing wallYaw + 0x8000 (0: +z),
    // gPlayerAnim_link_child_tunnel_start with moveFlags 0x9D.
    tick(&mut w, &mut prev, A_FORWARD);
    let p = w.player();
    assert_eq!(p.action, Action::ItemPutAway);
    assert!(p.state2 & STATE2_18 != 0);
    assert_eq!(p.actor.world_pos.x, -785.0);
    assert!((p.actor.world_pos.z - (1059.0 - wall_distance)).abs() < 0.01, "{} {}", p.actor.world_pos, wall_distance);
    assert_eq!((p.actor.shape_rot.y, p.current_yaw), (0, 0));
    assert_eq!(w.data.anim_name(p.skel.animation), "link_child_tunnel_start");
    // 0x9D, less 0x10 (no translation on the first frame), which SkelAnime_UpdateTranslation
    // clears in the frame's AnimTaskQueue_Update.
    assert_eq!(p.skel.move_flags, 0x9D & !0x10);
    // Player_Action_WaitForPutAway: no item to put away, so the crawl (func_8083A40C) next frame.
    tick(&mut w, &mut prev, FORWARD);
    assert_eq!(w.player().action, Action::Crawl);
    // tunnel_start's root motion carries Link in, and the stick does nothing until it's over.
    let mut frames = 0;
    while w.player().linear_velocity == 0.0 {
        tick(&mut w, &mut prev, FORWARD);
        frames += 1;
        assert!(frames < 120, "tunnel_start ends");
    }
    let p = w.player();
    assert_eq!(p.skel.move_flags, 0, "cleared at the animation's end");
    assert!(p.actor.world_pos.z > 1079.0, "inside, past the mouth's inner wall: {}", p.actor.world_pos);
    // On the tunnel's floor: bg camera 9, CAM_SET_CRAWLSPACE (Camera_Subj4).
    assert_eq!((w.game_camera.setting, w.game_camera.bg_cam_index), (CAM_SET_CRAWLSPACE, 9));
    // Player_Action_8084C760: linearVelocity = rel.stick_y * 0.03 (stick 60, 53 past the dead zone).
    assert!((p.linear_velocity - 53.0 * 0.03).abs() < 1e-5);

    // Crawling: Camera_Subj4 puts Link on its line (x -784, the points' x) at the ground,
    // facing along it: Camera_XZAngle(point 4, point 1) - 0x7FFF = 0x8000 - 0x7FFF = 1. And
    // Player_OverrideLimbDrawGameplayCrawling draws none of him with the camera inside him.
    let mut saw_room_2_at = None;
    loop {
        tick(&mut w, &mut prev, FORWARD);
        let p = w.player();
        if p.action != Action::Crawl {
            break;
        }
        assert_eq!(p.actor.world_pos.x, -784.0);
        assert_eq!(p.actor.world_pos.y, 120.0);
        assert_eq!(p.actor.shape_rot.y, 1);
        assert!(!link_drawn(&w), "Link isn't drawn in the crawlspace");
        if saw_room_2_at.is_none() && w.room_ctx.cur.num == 2 {
            saw_room_2_at = Some(p.actor.world_pos.z);
        }
    }
    // En_Holl 0 (kind 4, 200 wide): room 2 loads as Link comes within 50 of its plane.
    let z = saw_room_2_at.expect("room 2 loaded on the way");
    assert!((1219.0..1300.0).contains(&z), "room 2 at z {z}");
    // Player_TryLeavingCrawlspace at the far wall (z 1359, head first): out with tunnel_end, facing
    // wallYaw + 0x8000 (the wall faces -z, wallYaw 0x8000: Link faces 0, +z).
    // (The exit's one-point cutscene, 9601, puts the main camera back on its previous setting
    // in Player's update, so Camera_Subj4 doesn't run this frame to write its yaw over
    // shape.rot.y.)
    let p = w.player();
    assert_eq!(p.action, Action::CrawlExit);
    assert_eq!(w.data.anim_name(p.skel.animation), "link_child_tunnel_end");
    assert_eq!((p.current_yaw, p.linear_velocity, p.actor.shape_rot.y), (0, 0.0, 0));
    assert!(p.actor.world_pos.z > 1340.0 && p.actor.world_pos.z < 1359.0, "at the far wall: {}", p.actor.world_pos);
    // Player_Action_8084C81C: standing at the animation's end (func_8083C0E8), the crawl over; room 2's
    // floor takes the camera (bg camera 14).
    let mut frames = 0;
    while w.player().action == Action::CrawlExit {
        tick(&mut w, &mut prev, PadState::default());
        frames += 1;
        assert!(frames < 120);
    }
    let p = w.player();
    assert_eq!(p.action, Action::StandingStill);
    assert_eq!(p.state2 & STATE2_18, 0);
    assert!(p.actor.world_pos.z > 1379.0, "out: {}", p.actor.world_pos);
    assert_eq!(w.room_ctx.cur.num, 2);
    assert_eq!((w.game_camera.setting, w.game_camera.bg_cam_index), (CAM_SET_DUNGEON0, 14));
    assert!(link_drawn(&w));
}

#[test]
fn the_way_out_is_a_one_point_cutscene() {
    // Player_TryLeavingCrawlspace's OnePointCutscene_Init(play, 9601, 999, NULL, CAM_ID_MAIN), and
    // OnePointCutscene_SetInfo's 9601: CAM_SET_CS_3 (Camera_Demo9) on the sub camera, the main
    // camera back on its prevSetting, the splines sCrawlspaceAtPoints (at) and sCrawlspaceForwardsEyePoints (eye) around
    // the main camera's Player (actionParameters sCrawlspaceActionParam = 1, | 0x1000: copied to the main
    // camera at the end) for sCrawlspaceTimer = 90 frames.
    use oot_game::camera::{CAM_ID_MAIN, CAM_STAT_ACTIVE, CAM_STAT_UNK3, VecSphGeo, sph_geo_add, vec3_to_sph_geo};
    use oot_game::onepoint::CAM_SET_CS_3;
    let Some(a) = assets() else { return };
    let Some((mut w, mut prev)) = at_the_mouth(&a, false, Vec3::new(-785.0, 120.0, 1000.0)) else { return };
    enter(&mut w, &mut prev);
    let mut frames = 0;
    while w.player().action != Action::CrawlExit {
        tick(&mut w, &mut prev, FORWARD);
        frames += 1;
        assert!(frames < 400);
    }
    assert_eq!(w.active_cam_id, 1, "the one-point cutscene's sub camera is active");
    let sub = w.camera(1).expect("sub camera 1").clone();
    assert_eq!((sub.cs_id, sub.setting, sub.parent_cam_id, sub.child_cam_id), (9601, CAM_SET_CS_3, CAM_ID_MAIN, CAM_ID_MAIN));
    assert_eq!(w.game_camera.child_cam_id, 1);
    assert_eq!(w.game_camera.status, CAM_STAT_UNK3);
    assert_eq!(w.game_camera.setting, CAM_SET_NORMAL0, "the main camera's prevSetting: the field's");
    let op = &w.onepoint;
    assert_eq!(sub.one_point_cam_data.at_points, op.point_list("sCrawlspaceAtPoints"));
    assert_eq!(sub.one_point_cam_data.eye_points, op.point_list("sCrawlspaceForwardsEyePoints"));
    // The finishing action is taken off actionParameters by Camera_Demo9's first frame.
    assert_eq!((sub.one_point_cam_data.action_parameters, sub.one_point_cam_data.init_timer), (1, 90));
    // Its first frame: the splines at keyframe 0, u 0, ((p0 + 4 p1 + p2) / 6, from the
    // tables' first points: the eye (0, 9, 45), (0, 8, 50), (0, 17, 58), the at (0, 4, 0) twice
    // then (0, 9, 0)), turned by the main camera's Player's yaw round his position.
    let link = w.player().actor.world_pos;
    let yaw = w.player().actor.shape_rot.y;
    let turn = |v: Vec3| {
        let mut s: VecSphGeo = vec3_to_sph_geo(v);
        s.yaw = s.yaw.wrapping_add(yaw);
        sph_geo_add(link, s)
    };
    let eye = turn(Vec3::new(0.0, (9.0 + 4.0 * 8.0 + 17.0) / 6.0, (45.0 + 4.0 * 50.0 + 58.0) / 6.0));
    let at = turn(Vec3::new(0.0, (4.0 + 4.0 * 4.0 + 9.0) / 6.0, 0.0));
    assert!(sub.eye.distance(eye) < 0.01 && sub.at.distance(at) < 0.01, "eye {} at {} want {eye} {at}", sub.eye, sub.at);
    // The fov from the at points' (the second call's): (40 + 4 x 40.000004 + 50) / 6.
    assert!((sub.fov - (40.0 + 4.0 * 40.000004 + 50.0) / 6.0).abs() < 1e-4, "{}", sub.fov);
    // 92 updates of the sub camera: the splines' and then the wait, each counting animTimer
    // down from 90 to -1, and the one that finishes (timer 0, Camera_Copy to the main camera);
    // Camera_Finish then gives the main camera back. While Link is still on the crawlspace's
    // floor, the main camera (CAM_STAT_UNK3, still updated) takes its bg camera again the frame
    // after 9601 left it (Camera_RequestBgCam: refused in the frame of the change by the
    // setting's priority, behaviorFlags & 1), and its Camera_Subj4 asks for view.unk_124: Play_Draw
    // then updates the active camera, the sub camera, a second time.
    let mut frames = 1;
    let mut doubled = 0;
    let mut last = sub;
    while w.active_cam_id == 1 {
        last = w.camera(1).unwrap().clone();
        tick(&mut w, &mut prev, PadState::default());
        frames += 1;
        if w.game_camera.setting == CAM_SET_CRAWLSPACE {
            doubled += 1;
        }
        assert!(frames < 200);
    }
    assert!(doubled > 0, "the crawlspace's camera doubles some frames");
    // (The frame 9601 starts in has one: the main camera's floor change is refused then.)
    assert_eq!(frames + doubled, 92, "{frames} frames, {doubled} of them with two updates");
    assert!(w.camera(1).is_none(), "the sub camera cleared");
    assert_eq!((w.game_camera.status, w.game_camera.child_cam_id, w.game_camera.parent_cam_id), (CAM_STAT_ACTIVE, CAM_ID_MAIN, CAM_ID_MAIN));
    // No jump: the main camera starts from where the cutscene ended (Camera_Copy), then follows
    // room 2's floor (bg camera 14).
    let (e, t) = (w.game_camera.eye, w.game_camera.at);
    assert!(e.distance(last.eye) < 40.0 && t.distance(last.at) < 40.0, "main eye {e} at {t}, the cutscene's last {} {}", last.eye, last.at);
    assert_eq!((w.game_camera.setting, w.game_camera.bg_cam_index), (CAM_SET_DUNGEON0, 14));
}

#[test]
fn camera_subj4_eases_in_then_rides_the_line() {
    use oot_game::camera::{VecSphGeo, sph_geo_add};
    let Some(a) = assets() else { return };
    let Some((mut w, mut prev)) = at_the_mouth(&a, false, Vec3::new(-785.0, 120.0, 1000.0)) else { return };
    enter(&mut w, &mut prev);
    // Until the crawlspace's setting takes the camera.
    let mut frames = 0;
    while w.game_camera.setting != CAM_SET_CRAWLSPACE {
        tick(&mut w, &mut prev, FORWARD);
        frames += 1;
        assert!(frames < 120);
    }
    // Its first call each frame asks for the second at the end of Play_Draw (view.unk_124);
    // the second eases in for 10 frames (zoomTimer: at += (target - at) / (zoomTimer + 1)), with the
    // eye at a shrinking distance.
    let d0 = w.game_camera.at.distance(w.game_camera.eye);
    for _ in 0..9 {
        tick(&mut w, &mut prev, FORWARD);
    }
    let d9 = w.game_camera.at.distance(w.game_camera.eye);
    assert!(d9 < d0, "the eye closes in: {d0} -> {d9}");
    // Once Link moves (the first call's xzSpeed ≥ 0.5), the eye is on the line at Link (its
    // closest point, x -784, y 132, Link's z), bobbing towards 5 ahead-and-up with
    // |cos(eyeLerpPhase)| (eyeLerpPhase += 0xBB8 a frame), and `at` 10 ahead, swaying by
    // 240 · cos(eyeLerpPhase) · xzSpeed · 0.416667.
    while w.player().linear_velocity == 0.0 {
        tick(&mut w, &mut prev, FORWARD);
    }
    for _ in 0..30 {
        tick(&mut w, &mut prev, FORWARD);
        let (c, p) = (&w.game_camera, w.player());
        let on_line = Vec3::new(-784.0, 132.0, p.actor.world_pos.z);
        let ahead = sph_geo_add(on_line, VecSphGeo { r: 5.0, pitch: 0x238C, yaw: 1 });
        // eye = on_line + (ahead - on_line)·|cos|: on the segment between them.
        let t = (c.eye - on_line).length() / (ahead - on_line).length();
        assert!(t <= 1.0 + 1e-4 && (c.eye - (on_line + (ahead - on_line) * t)).length() < 0.01, "eye {:?} line {on_line} ahead {ahead}", c.eye);
        assert!((c.at.y - c.eye.y).abs() < 1e-4 && (c.at.distance(c.eye) - 10.0).abs() < 0.01, "at {:?} eye {:?}", c.at, c.eye);
    }
}

#[test]
fn backwards_out_of_the_mouth() {
    let Some(a) = assets() else { return };
    let Some((mut w, mut prev)) = at_the_mouth(&a, false, Vec3::new(-785.0, 120.0, 1000.0)) else { return };
    enter(&mut w, &mut prev);
    while w.player().linear_velocity == 0.0 {
        tick(&mut w, &mut prev, FORWARD);
    }
    // The stick back: backwards (rel.stick_y -53 × 0.03), into the mouth's inner wall (z 1079,
    // facing +z, wallYaw 0): shape.rot.y (1) - wallYaw, plus 0x8000 going backwards, is more
    // than a quarter turn, so Player_TryLeavingCrawlspace's other branch: facing wallYaw (+z) and
    // tunnel_start played backwards (LinkAnimation_Change with speed -1 from its last frame).
    let mut frames = 0;
    while w.player().action == Action::Crawl {
        tick(&mut w, &mut prev, BACK);
        frames += 1;
        assert!(frames < 200);
    }
    let p = w.player();
    assert_eq!(p.action, Action::CrawlExit);
    assert_eq!(w.data.anim_name(p.skel.animation), "link_child_tunnel_start");
    assert_eq!(p.skel.play_speed, -1.0);
    // (9602 takes the main camera off CRAWLSPACE in Player's update, as 9601 at the far end:
    // Camera_Subj4 doesn't write shape.rot.y this frame.)
    assert_eq!((p.current_yaw, p.actor.shape_rot.y), (0, 0));
    while w.player().action == Action::CrawlExit {
        tick(&mut w, &mut prev, PadState::default());
    }
    // Out where Link went in, in room 0 and on its NORMAL0 floor.
    let p = w.player();
    assert_eq!(p.action, Action::StandingStill);
    assert_eq!(p.state2 & STATE2_18, 0);
    assert!(p.actor.world_pos.z < 1059.0, "back out of the mouth: {}", p.actor.world_pos);
    assert_eq!(w.room_ctx.cur.num, 0);
    assert_eq!(w.game_camera.setting, CAM_SET_NORMAL0);
}

#[test]
fn off_the_middle_there_is_no_enter() {
    let Some(a) = assets() else { return };
    // 12 off the triangles' middle (x -785): |dx · n.z - dz · n.x| = 12 is not under 8.
    let Some((mut w, mut prev)) = at_the_mouth(&a, false, Vec3::new(-797.0, 120.0, 1000.0)) else { return };
    let mut touched = false;
    for _ in 0..60 {
        tick(&mut w, &mut prev, FORWARD);
        touched |= w.player().actor.world_pos.z > 1040.0;
        assert_eq!(w.player().state2 & STATE2_16, 0, "no Enter off the middle");
    }
    assert!(touched, "Link reached the mouth");
    // A there: Player_ActionHandler_5 doesn't take it; with the stick forward, the roll's interrupt
    // (Player_ActionHandler_Roll) does.
    tick(&mut w, &mut prev, A_FORWARD);
    assert_eq!(w.player().state2 & STATE2_18, 0);
    assert_ne!(w.player().action, Action::ItemPutAway);
}

#[test]
fn adult_link_cannot_crawl() {
    let Some(a) = assets() else { return };
    // Player_TryEnteringCrawlspace: !LINK_IS_ADULT.
    let Some((mut w, mut prev)) = at_the_mouth(&a, true, Vec3::new(-785.0, 120.0, 1000.0)) else { return };
    for _ in 0..60 {
        tick(&mut w, &mut prev, FORWARD);
        assert_eq!(w.player().state2 & STATE2_16, 0);
    }
    tick(&mut w, &mut prev, A_FORWARD);
    tick(&mut w, &mut prev, FORWARD);
    assert_eq!(w.player().state2 & STATE2_18, 0);
    assert_ne!(w.player().action, Action::Crawl);
}
