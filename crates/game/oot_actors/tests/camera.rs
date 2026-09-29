//! Milestone 2 checks: `z_camera.c`'s Normal camera. Expected values come from
//! `z_camera.c` / `z_camera_data.c` formulas and constants, not from the port.

mod common;

use common::*;
use oot_actors::PlayExt;
use glam::Vec3;
use oot_game::camera::CameraKind;
use oot_game::camera::{self, VecSph, diff_to_sph_geo, f_atan2f, lerp_ceil_s, sph_geo_add};

#[test]
fn camera_data_read_from_decomp() {
    let Some(d) = data() else { return };
    let c = &d.camera;
    // sOREGInit: OREG(2..4) = 5, R_CAM_MAX_PITCH 14500, OREG(6) 20, R_CAM_DEFAULT_PITCH_UPDATE_RATE_INV 16,
    // OREG(8) 150, R_CAM_YOFFSET_NORM -10, OREG(50) = OREG(51) = 20, 53 entries.
    assert_eq!(c.oreg.len(), 53);
    assert_eq!((c.oreg(2), c.oreg(5), c.oreg(6), c.oreg(7), c.oreg(8), c.oreg(46), c.oreg(50), c.oreg(51)), (5, 14500, 20, 16, 150, -10, 20, 20));
    // sCameraSettings: CAM_SET_MAX (0x42) entries, named by z64camera.h's enum; entry 0
    // (CAM_SET_NONE) is { { 0x00000000 }, NULL }.
    assert_eq!(c.settings.len(), 0x42);
    assert_eq!((c.settings[0].name.as_str(), c.settings[0].flags, c.settings[0].modes.len()), ("CAM_SET_NONE", 0, 0));
    let s = &c.settings[camera::CAM_SET_NORMAL0 as usize];
    // sCamSetNormal0Modes: CAM_MODE_MAX (21) entries, every one valid ({ { 0x051FFFFF }, ...}).
    assert_eq!((s.name.as_str(), s.flags, s.modes.len()), ("CAM_SET_NORMAL0", 0x051F_FFFF, 21));
    let mode = |set: i16, m: i16| c.mode(set, m).unwrap();
    // sSetNormal0ModeNormalData = CAM_FUNCDATA_NORM1(-20, 200, 300, 10, 12, 10, 35, 60, 60, 0x0003).
    let m = mode(camera::CAM_SET_NORMAL0, camera::CAM_MODE_NORMAL);
    assert_eq!((m.func.as_str(), m.data.as_str()), ("CAM_FUNC_NORM1", "sSetNormal0ModeNormalData"));
    assert_eq!(m.values, [-20, 200, 300, 10, 12, 10, 35, 60, 60, 3]);
    // sSetNormal0ModeTargetData = CAM_FUNCDATA_PARA1(-20, 250, 0, 0, 5, 5, 45, 50, 0x200A, -40, 20).
    let m = mode(camera::CAM_SET_NORMAL0, camera::CAM_MODE_TARGET);
    assert_eq!(m.func, "CAM_FUNC_PARA1");
    assert_eq!(m.values, [-20, 250, 0, 0, 5, 5, 45, 50, 0x200A, -40, 20]);
    // sSetNormal0ModeFollowTargetData = CAM_FUNCDATA_KEEP1(-20, 120, 140, 25, 45, -5, 15, 15, 45,
    // 50, 0x2001, -50, 30).
    let m = mode(camera::CAM_SET_NORMAL0, camera::CAM_MODE_FOLLOWTARGET);
    assert_eq!(m.func, "CAM_FUNC_KEEP1");
    assert_eq!(m.values, [-20, 120, 140, 25, 45, -5, 15, 15, 45, 50, 0x2001, -50, 30]);
    // STILL is Normal1 too; BATTLE is Camera_Battle1 (not ported).
    assert_eq!(mode(camera::CAM_SET_NORMAL0, camera::CAM_MODE_STILL).func, "CAM_FUNC_NORM1");
    assert_eq!(mode(camera::CAM_SET_NORMAL0, camera::CAM_MODE_BATTLE).func, "CAM_FUNC_BATT1");

    // The prerendered rooms' settings. sCamSetPreRendFixedModes = { FIXD3 sDataOnlyNullFlags,
    // { CAM_FUNC_NONE, 0, NULL }, FIXD3 sSetPrerendFixedModeFollowTargetData (FLAGS 0x2000) x2 },
    // valid 0x8C00000D.
    let s = &c.settings[camera::CAM_SET_PREREND_FIXED as usize];
    assert_eq!((s.name.as_str(), s.flags, s.modes.len()), ("CAM_SET_PREREND_FIXED", 0x8C00_000D, 4));
    assert!(s.modes[1].is_none());
    assert_eq!((mode(camera::CAM_SET_PREREND_FIXED, 0).func.as_str(), mode(camera::CAM_SET_PREREND_FIXED, 0).values.as_slice()), ("CAM_FUNC_FIXD3", &[0][..]));
    assert_eq!(mode(camera::CAM_SET_PREREND_FIXED, camera::CAM_MODE_FOLLOWTARGET).values, [0x2000]);
    // sCamSetPreRendPivotModes: UNIQ7 CAM_FUNCDATA_UNIQ7(60, 0x0000), none, UNIQ7 (60, 0x2000),
    // KEEP0 CAM_FUNCDATA_KEEP0(30, 0, 4, 0x3500).
    let m = mode(camera::CAM_SET_PREREND_PIVOT, camera::CAM_MODE_NORMAL);
    assert_eq!((m.func.as_str(), m.values.as_slice()), ("CAM_FUNC_UNIQ7", &[60, 0][..]));
    assert_eq!(mode(camera::CAM_SET_PREREND_PIVOT, camera::CAM_MODE_TALK).values, [30, 0, 4, 0x3500]);
    // Doors, exits and FREE0: CAM_FUNCDATA_SPEC9(-5, 60, 0x3202), CAM_FUNCDATA_UNIQ2(-20, 150,
    // 60, 0x0210), CAM_FUNCDATA_FLAGS(0xFF00); PIVOT_SHOP_BROWSING CAM_FUNCDATA_DATA4(-40, 60,
    // 0x3F00); PIVOT_IN_FRONT CAM_FUNCDATA_FIXD4(-40, 50, 80, 60, 0x0004).
    let m = mode(camera::CAM_SET_DOORC, camera::CAM_MODE_NORMAL);
    assert_eq!((m.func.as_str(), m.values.as_slice()), ("CAM_FUNC_SPEC9", &[-5, 60, 0x3202][..]));
    let m = mode(camera::CAM_SET_SCENE_TRANSITION, camera::CAM_MODE_NORMAL);
    assert_eq!((m.func.as_str(), m.values.as_slice()), ("CAM_FUNC_UNIQ2", &[-20, 150, 60, 0x0210][..]));
    assert_eq!(c.settings[camera::CAM_SET_SCENE_TRANSITION as usize].flags, 0x4500_0001);
    let m = mode(camera::CAM_SET_FREE0, camera::CAM_MODE_NORMAL);
    assert_eq!((m.func.as_str(), m.values.as_slice()), ("CAM_FUNC_UNIQ6", &[0xFF00u16 as i16][..]));
    let m = mode(camera::CAM_SET_PIVOT_SHOP_BROWSING, camera::CAM_MODE_NORMAL);
    assert_eq!((m.func.as_str(), m.values.as_slice()), ("CAM_FUNC_DATA4", &[-40, 60, 0x3F00][..]));
    let m = mode(camera::CAM_SET_PIVOT_IN_FRONT, camera::CAM_MODE_NORMAL);
    assert_eq!((m.func.as_str(), m.values.as_slice()), ("CAM_FUNC_FIXD4", &[-40, 50, 80, 60, 4][..]));
    assert_eq!(c.setting_id("CAM_SET_PREREND_FIXED"), Some(camera::CAM_SET_PREREND_FIXED));
}

#[test]
fn setting_changes_follow_camera_change_setting_flags() {
    let Some(d) = data() else { return };
    let c = &d.camera;
    let pv = camera::PlayerView { pos: Vec3::ZERO, shape_yaw: 0, shape_pitch: 0, world_yaw: 0, adult: false, run_speed_limit: 550, gravity: 0.0, climbing: false, state1: 0 };
    // A collision with two bg cameras: 0 PREREND_FIXED, 1 PREREND_PIVOT.
    let mut h = eng_collision::collision::CollisionHeader::default();
    h.bg_cams.push(eng_collision::collision::BgCamInfo { setting: camera::CAM_SET_PREREND_FIXED as u16, count: 3, data: vec![[0, 300, 0], [0x3000, 0, 0], [5000, -1, -1]] });
    h.bg_cams.push(eng_collision::collision::BgCamInfo { setting: camera::CAM_SET_PREREND_PIVOT as u16, count: 3, data: vec![[0, 40, 0], [0, 0, 0], [6000, -1, -1]] });
    let col = eng_collision::bgcheck::CollisionContext::new(h);
    let mut cam = camera::GameCamera::new(c, &pv);
    // func_80057FC4 for a prerendered room: CAM_SET_FREE0, bgCamIndex -1.
    cam.func_80057fc4(camera::CamRoom { image: true, behavior_type1: 0 });
    assert_eq!((cam.setting, cam.prev_setting, cam.bg_cam_index), (camera::CAM_SET_FREE0, camera::CAM_SET_FREE0, -1));
    // Camera_ChangeBgCamIndex(1): the bg camera's setting with flags 5 (the index kept),
    // unk_14A 0x40 | 0x10 | 4 (and 1: flags without 2); prevSetting FREE0.
    assert_eq!(cam.change_bg_cam_index(c, &col, 1), (0x8000_0000u32 | 1) as i32);
    assert_eq!((cam.setting, cam.prev_setting, cam.bg_cam_index), (camera::CAM_SET_PREREND_PIVOT, camera::CAM_SET_FREE0, 1));
    assert_eq!(cam.unk_14a & 0x55, 0x55);
    // Once a frame: a second index this frame changes nothing (unk_14A & 0x40).
    cam.change_bg_cam_index(c, &col, 0);
    assert_eq!((cam.setting, cam.bg_cam_index), (camera::CAM_SET_PREREND_PIVOT, 1));
    // After a change this frame (unk_14A & 1), a setting of the same or lower priority: -2
    // (sCameraSettings: PREREND_PIVOT 0x8C00000D is priority 0xC, DOORC 0xC5000003 priority
    // 5), checked before the same setting's -1.
    assert_eq!(cam.change_setting(c, camera::CAM_SET_PREREND_PIVOT), -2);
    assert_eq!(cam.change_setting(c, camera::CAM_SET_DOORC), -2);
    cam.unk_14a = 0;
    assert_eq!(cam.change_setting(c, camera::CAM_SET_PREREND_PIVOT), -1);
    // Not a setting: -99.
    cam.unk_14a = 0;
    assert_eq!(cam.change_setting(c, camera::CAM_SET_NONE), -99);
    // A new frame (Camera_Update clears unk_14A): DOORC through Camera_ChangeDoorCam(-1), which
    // also stores the door's timers; the bg camera goes to prevBgCamIndex (flags 0).
    cam.unk_14a = 0;
    assert_eq!(cam.change_door_cam(c, &col, None, -1, 38, 26, 10), -1);
    assert_eq!((cam.setting, cam.prev_setting, cam.bg_cam_index, cam.prev_bg_cam_index), (camera::CAM_SET_DOORC, camera::CAM_SET_PREREND_PIVOT, -1, 1));
    assert_eq!((cam.door_params.timer1, cam.door_params.timer2, cam.door_params.timer3), (38, 26, 10));
    // DOORC takes no other door camera until it ends.
    assert_eq!(cam.change_door_cam(c, &col, None, 0, 1, 1, 1), 0);
}

#[test]
fn olib_maths() {
    // Math_FAtan2F is a Taylor series; it should agree with atan2 to float precision.
    for &(y, x) in &[(1.0f32, 2.0f32), (-3.0, 0.5), (0.2, -4.0), (-1.0, -1.0), (5.0, 0.0), (1e-3, 7.0)] {
        assert!((f_atan2f(y, x) - y.atan2(x)).abs() < 2e-6, "{y} {x}");
    }
    // Geographic coordinates: pitch from the horizon, yaw 0 = +z.
    let v = diff_to_sph_geo(Vec3::ZERO, Vec3::new(0.0, 100.0, 100.0));
    assert_eq!(v.yaw, 0);
    assert!((v.pitch as i32 - 0x2000).abs() <= 2, "{v:?}");
    let back = sph_geo_add(Vec3::ZERO, VecSph { r: 100.0, pitch: 0, yaw: 0x4000 });
    assert!((back - Vec3::new(100.0, 0.0, 0.0)).length() < 0.05, "{back}");
    // Camera_LERPCeilS: step = diff * scale + 0.5 truncated; within minDiff snaps to target.
    assert_eq!(lerp_ceil_s(1000, 0, 0.25, 10), 250);
    assert_eq!(lerp_ceil_s(5, 0, 0.25, 10), 5);
    assert_eq!(lerp_ceil_s(-0x7000, 0x7000, 0.5, 10), 0x7000i16.wrapping_add(((0x2000 as f32) * 0.5 + 0.5) as i16));
}

/// Normal1's read-only data for Player height `h` (Camera_Normal1, RELOAD_PARAMS).
fn normal0_limits(h: f32) -> (f32, f32, f32) {
    let y_normal = 1.0 + (-10.0 * 0.01) - (-10.0 * 0.01) * (68.0 / h);
    let s = y_normal * (h * 0.01);
    (-20.0 * s, 200.0 * s, 300.0 * s)
}

#[test]
fn camera_init_places_eye_behind_player() {
    let Some(w) = world() else { return };
    let c = &w.game_camera;
    let p = w.player().actor.world_pos;
    // Camera_InitPlayerSettings: at = pos + height (68 adult); eye at r 180, pitch 0x71C,
    // yaw = shape.rot.y - 0x7FFF; inputDir.y = shape.rot.y.
    assert_eq!(c.at, p + Vec3::Y * 68.0);
    let e = diff_to_sph_geo(c.at, c.eye);
    assert!((e.r - 180.0).abs() < 0.01);
    // Math_SinS/CosS use libultra's table, which runs 1023/1024 slow, so the round trip
    // through a vector lands within ~0.1 degrees.
    assert!((e.pitch as i32 - 0x71C).abs() <= 20, "{e:?} eye {} at {}", c.eye, c.at);
    assert!((e.yaw.wrapping_sub(w.player().actor.shape_rot.y.wrapping_sub(0x7FFF)) as i32).abs() <= 2);
    assert_eq!(c.input_dir_yaw(), w.player().actor.shape_rot.y);
}

#[test]
fn standing_camera_settles_at_normal0_distance_and_pitch() {
    let Some(mut w) = world() else { return };
    assert_eq!(w.camera_kind, CameraKind::Game);
    run(&mut w, &repeat(stick(0, 0), 200));
    let c = &w.game_camera;
    let (y_off, dmin, dmax) = normal0_limits(68.0);
    assert!((dmin - 136.0).abs() < 1e-3 && (dmax - 204.0).abs() < 1e-3);
    // Starts at r 180, inside [distMin, distMax], so Camera_ClampDist doesn't pull it to a limit;
    // it changes only as `at` settles.
    assert!(c.dist > dmin && c.dist < dmax, "dist {} eye {} at {}", c.dist, c.eye, c.at);
    // at converges to pos + height + yOffset on flat ground (no slope adjustment).
    let at_target = w.player().actor.world_pos.y + 68.0 + y_off;
    assert!((c.at.y - at_target).abs() < 0.25, "at.y {} vs {at_target}", c.at.y);
    // Pitch converges to CAM_DEG_TO_BINANG(10) (Camera_CalcDefaultPitch, minDiff 0xA snaps).
    let e = diff_to_sph_geo(c.at, c.eye);
    assert!((e.pitch as i32 - camera::cam_deg_to_binang(10.0) as i32).abs() <= 12, "pitch {:#x}", e.pitch);
}

#[test]
fn camera_recentres_behind_player_after_he_stops() {
    let Some(mut w) = world() else { return };
    // Run left of the camera for a while (Link and the camera circle each other), then let go.
    // Standing still, startSwingTimer (OREG(50) + OREG(51) = 40) walks swingYawTarget round to
    // behind Player; the camera holds still until the timer runs out, then its yaw LERPs onto
    // `rot.y - 0x7FFF` at 1 / yawUpdateRateInv per frame (unk_0C * 2 = 24, raised towards 40.8
    // by the OREG(49) term), so after ~200 frames about 2% of the error is left.
    let mut s = repeat(stick(-80, 0), 30);
    s.extend(repeat(stick(0, 0), 200));
    run(&mut w, &s);
    let c = &w.game_camera;
    let facing = w.player().actor.shape_rot.y;
    let e = diff_to_sph_geo(c.at, c.eye);
    let behind = facing.wrapping_sub(0x7FFF);
    let err = e.yaw.wrapping_sub(behind) as i32;
    assert!(err.abs() < 0x100, "camera yaw {:#x}, behind Player {behind:#x}, {e:?}", e.yaw);
    // The input yaw Player reads is the eye->at yaw.
    assert_eq!(c.input_dir_yaw(), diff_to_sph_geo(c.eye, c.at).yaw);
    // Running (speedRatio 1) keeps the distance within Normal0's limits.
    let (_, dmin, dmax) = normal0_limits(68.0);
    assert!(c.dist >= dmin - 0.5 && c.dist <= dmax + 0.5, "dist {}", c.dist);
}

#[test]
fn camera_stays_in_front_of_walls() {
    let Some(d) = data() else { return };
    // Stand 60 units from the course's +z boundary wall (z = 1000), facing away from it
    // (yaw 0x8000 = -z), so the camera wants to sit 180 units behind, beyond the wall.
    let Some(mut w) = world_at(Vec3::new(0.0, 0.0, 940.0), -0x8000) else { return };
    let _ = d;
    run(&mut w, &repeat(stick(0, 0), 40));
    let c = &w.game_camera;
    assert!(c.eye.z < 1000.0, "eye z {} is behind the wall", c.eye.z);
    // And it's still behind Player.
    assert!(c.eye.z > w.player().actor.world_pos.z, "eye {}", c.eye);
}
