//! Milestone 2 checks: `z_camera.c`'s Normal camera. Expected values come from
//! `z_camera.c` / `z_camera_data.c` formulas and constants, not from the port.

mod common;

use common::*;
use glam::Vec3;
use oot_game::camera::{self, VecSph, diff_to_sph_geo, f_atan2f, lerp_ceil_s, sph_geo_add};
use oot_game::world::CameraKind;

#[test]
fn camera_data_read_from_decomp() {
    let Some(d) = data() else { return };
    let c = &d.camera;
    // sOREGInit: OREG(2..4) = 5, R_CAM_MAX_PITCH 14500, OREG(6) 20, R_CAM_DEFAULT_PITCH_UPDATE_RATE_INV 16,
    // OREG(8) 150, R_CAM_YOFFSET_NORM -10, OREG(50) = OREG(51) = 20, 53 entries.
    assert_eq!(c.oreg.len(), 53);
    assert_eq!((c.oreg(2), c.oreg(5), c.oreg(6), c.oreg(7), c.oreg(8), c.oreg(46), c.oreg(50), c.oreg(51)), (5, 14500, 20, 16, 150, -10, 20, 20));
    // sSetNormal0ModeNormalData = CAM_FUNCDATA_NORM1(-20, 200, 300, 10, 12, 10, 35, 60, 60, 0x0003).
    assert_eq!(c.normal0, [-20, 200, 300, 10, 12, 10, 35, 60, 60, 3]);
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
    let p = w.player.actor.world_pos;
    // Camera_InitPlayerSettings: at = pos + height (68 adult); eye at r 180, pitch 0x71C,
    // yaw = shape.rot.y - 0x7FFF; inputDir.y = shape.rot.y.
    assert_eq!(c.at, p + Vec3::Y * 68.0);
    let e = diff_to_sph_geo(c.at, c.eye);
    assert!((e.r - 180.0).abs() < 0.01);
    // Math_SinS/CosS use libultra's table, which runs 1023/1024 slow, so the round trip
    // through a vector lands within ~0.1 degrees.
    assert!((e.pitch as i32 - 0x71C).abs() <= 20, "{e:?} eye {} at {}", c.eye, c.at);
    assert!((e.yaw.wrapping_sub(w.player.actor.shape_rot.y.wrapping_sub(0x7FFF)) as i32).abs() <= 2);
    assert_eq!(c.input_dir_yaw(), w.player.actor.shape_rot.y);
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
    let at_target = w.player.actor.world_pos.y + 68.0 + y_off;
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
    let facing = w.player.actor.shape_rot.y;
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
    assert!(c.eye.z > w.player.actor.world_pos.z, "eye {}", c.eye);
}
