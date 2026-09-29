//! Kokiri Forest's crawlspace (GAME-03 milestone 2): Player's crawl (`func_8083F0C8`,
//! `func_8084C760`, `func_8083F570`, `func_8084C81C` in `z_player.c`) and the crawlspace's
//! camera (`Camera_Subj4`, `CAM_SET_CRAWLSPACE`).
//!
//! Expected values come from the scene data and the C:
//! - the mouth is two triangles of wall at z 1059, x -801..-769, y 120..144, facing -z, wall
//!   type 5 (`WALL_FLAG_4`): their middle is x -785, `wallYaw` 0x8000;
//! - inside, the tunnel's floor (x -801..-769, z 1059..1379) names bg camera 9, `CAM_SET_CRAWLSPACE`
//!   with six points along x -784 at y 132, z 1037 (points 0..2) and 1401 (3..5); the walls at
//!   z 1079 (facing +z) and z 1359 (facing -z) are `WALL_FLAG_4` too;
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

/// Kokiri Forest (`ENTR_SPOT04_0`) on a new save, `adult` or not, with Link placed in front of
/// the crawlspace's mouth at `at`, facing +z.
fn at_the_mouth(a: &Arc<GameAssets>, adult: bool, at: Vec3) -> Option<(PlayState, PadState)> {
    let e = a.scenes.entrance_index("ENTR_SPOT04_0").expect("entrance");
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
/// stays in `func_80842180`, whose interrupts include `func_8083F7BC`).
fn enter(w: &mut PlayState, prev: &mut PadState) {
    for _ in 0..60 {
        if w.player().state2 & STATE2_16 != 0 {
            break;
        }
        tick(w, prev, FORWARD);
    }
    assert!(w.player().state2 & STATE2_16 != 0, "PLAYER_STATE2_16 at the mouth");
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
    // Up to the mouth: func_8083F7BC sees the wall (facing it, BGCHECKFLAG_PLAYER_WALL_INTERACT)
    // and func_8083F0C8 finds Link within 8 of its triangles' middle: "Enter" on A.
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
    // A: func_80836898(func_8083A40C) with PLAYER_STATE2_18, Link at the middle plus
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
    // clears in the frame's AnimationContext_Update.
    assert_eq!(p.skel.move_flags, 0x9D & !0x10);
    // func_808458D0: no item to put away, so the crawl (func_8083A40C) next frame.
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
    // func_8084C760: linearVelocity = rel.stick_y * 0.03 (stick 60, 53 past the dead zone).
    assert!((p.linear_velocity - 53.0 * 0.03).abs() < 1e-5);

    // Crawling: Camera_Subj4 puts Link on its line (x -784, the points' x) at the ground,
    // facing along it: Camera_XZAngle(point 4, point 1) - 0x7FFF = 0x8000 - 0x7FFF = 1. And
    // Player_OverrideLimbDrawGameplay_80090440 draws none of him with the camera inside him.
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
    // func_8083F570 at the far wall (z 1359, head first): out with tunnel_end, facing
    // wallYaw + 0x8000 (the wall faces -z, wallYaw 0x8000: Link faces 0, +z).
    // (Link moved this frame before the exit, so Camera_Subj4, still the camera's, writes its
    // yaw over shape.rot.y again at the end of Play_Draw.)
    let p = w.player();
    assert_eq!(p.action, Action::CrawlExit);
    assert_eq!(w.data.anim_name(p.skel.animation), "link_child_tunnel_end");
    assert_eq!((p.current_yaw, p.linear_velocity, p.actor.shape_rot.y), (0, 0.0, 1));
    assert!(p.actor.world_pos.z > 1340.0 && p.actor.world_pos.z < 1359.0, "at the far wall: {}", p.actor.world_pos);
    // func_8084C81C: standing at the animation's end (func_8083C0E8), the crawl over; room 2's
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
fn camera_subj4_eases_in_then_rides_the_line() {
    use oot_game::camera::{VecSph, sph_geo_add};
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
    // the second eases in for 10 frames (unk_32: at += (target - at) / (unk_32 + 1)), with the
    // eye at a shrinking distance.
    let d0 = w.game_camera.at.distance(w.game_camera.eye);
    for _ in 0..9 {
        tick(&mut w, &mut prev, FORWARD);
    }
    let d9 = w.game_camera.at.distance(w.game_camera.eye);
    assert!(d9 < d0, "the eye closes in: {d0} -> {d9}");
    // Once Link moves (the first call's xzSpeed ≥ 0.5), the eye is on the line at Link (its
    // closest point, x -784, y 132, Link's z), bobbing towards 5 ahead-and-up with
    // |cos(unk_2C)| (unk_2C += 0xBB8 a frame), and `at` 10 ahead, swaying by
    // 240 · cos(unk_2C) · unk_24 · 0.416667.
    while w.player().linear_velocity == 0.0 {
        tick(&mut w, &mut prev, FORWARD);
    }
    for _ in 0..30 {
        tick(&mut w, &mut prev, FORWARD);
        let (c, p) = (&w.game_camera, w.player());
        let on_line = Vec3::new(-784.0, 132.0, p.actor.world_pos.z);
        let ahead = sph_geo_add(on_line, VecSph { r: 5.0, pitch: 0x238C, yaw: 1 });
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
    // than a quarter turn, so func_8083F570's other branch: facing wallYaw (+z) and
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
    // (shape.rot.y is Camera_Subj4's again, 1, as on the way out at the far end.)
    assert_eq!((p.current_yaw, p.actor.shape_rot.y), (0, 1));
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
    // A there: func_8083F7BC doesn't take it; with the stick forward, the roll's interrupt
    // (func_8083C1DC) does.
    tick(&mut w, &mut prev, A_FORWARD);
    assert_eq!(w.player().state2 & STATE2_18, 0);
    assert_ne!(w.player().action, Action::ItemPutAway);
}

#[test]
fn adult_link_cannot_crawl() {
    let Some(a) = assets() else { return };
    // func_8083F0C8: !LINK_IS_ADULT.
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
