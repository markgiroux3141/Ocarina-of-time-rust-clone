//! The mode functions GAME-04b milestone 2 ported (`Camera_Jump1`, `Camera_Jump2`,
//! `Camera_Unique1`), on the pack's camera data with no collision: a child Link standing at the
//! origin facing +z, the main camera put where each test says. Expected values are the C's
//! arithmetic on the modes' data (`sSetNormal0Mode*Data`) and the `OREG`s.

use super::*;
use crate::letterbox::Letterbox;
use crate::onepoint::OnePointStatics;
use glam::Vec2;

fn data() -> Option<CameraData> {
    let pack = crate::pack::GamePack::open_default().ok()?;
    Some(pack.game_data().ok()?.camera)
}

fn player() -> PlayerView {
    PlayerView { pos: Vec3::ZERO, shape_yaw: 0, shape_pitch: 0, world_yaw: 0, adult: false, run_speed_limit: 550, gravity: 0.0, climbing: false, state1: 0, iron_boots: false }
}

/// One `Camera_Update` of `c` with Link as `p`, in an empty world.
fn update(c: &mut GameCamera, d: &CameraData, p: &PlayerView, waist: Vec3) {
    let col = CollisionContext::new(Default::default());
    let oc = crate::collision_check::OcLines::default();
    let f = CamFrame {
        col: &col,
        player: *p,
        target_focus: None,
        door: None,
        transitioning: false,
        frames: 100,
        input: Input::default(),
        player_actor: None,
        oc_lines: &oc,
        cs_active: false,
        target_pos_rot: None,
        target: None,
        player_actor_info: None,
        view: CamView { eye: c.eye, at: c.at, fov: c.fov },
        main_player_pos_rot: (c.player_pos, c.player_rot_y),
        player_waist: waist,
    };
    let mut g = CameraGlobals::main_init();
    g.scene_init_letterbox_timer = 0;
    let mut op = OnePointStatics::new(&d.onepoint);
    let mut rand = crate::play::Rand::default();
    c.update(d, &f, &mut Letterbox::new(), &mut g, &mut op, &mut rand);
}

/// The main camera behind Link in `mode`, its eye (and `eyeNext`) moved to `eye`.
fn camera(d: &CameraData, mode: i16, eye: Vec3) -> GameCamera {
    let p = player();
    let mut c = GameCamera::new(d, &p);
    c.change_mode_flags(d, mode, 0);
    c.sfx.clear();
    c.eye = eye;
    c.eye_next = eye;
    c
}

/// `1 + R_CAM_YOFFSET_NORM / 100 * (1 - 68 / height)`: the modes' `yNormal`.
fn y_normal(d: &CameraData, h: f32) -> f32 {
    1.0 + d.oreg(46) as f32 * 0.01 - d.oreg(46) as f32 * 0.01 * (68.0 / h)
}

/// An angle measured back from a point the camera placed by it: `OLib_Vec3fDiffToVecGeo`'s
/// `Math_FAtan2F` (a Taylor series) against `OLib_VecGeoToVec3f`'s sines differ by up to about
/// 0x10 at these angles.
const ROUND_TRIP: i32 = 0x10;

fn yaw_close(a: i16, b: i16, tol: i32) -> bool {
    (a.wrapping_sub(b) as i32).abs() <= tol
}

#[test]
fn camera_jump2_turns_behind_link_on_the_ladder() {
    let Some(d) = data() else { return };
    let p = player();
    // Link faces +z: behind him is yaw 0 - 0x7FFF. The eye 100 out at yaw 0x3000 past that.
    let behind = 0i16.wrapping_sub(0x7FFF);
    let start_yaw = behind.wrapping_add(0x3000);
    let at0 = Vec3::new(0.0, p.height(), 0.0);
    let eye0 = sph_geo_add(at0, VecSphGeo { r: 100.0, pitch: 0x400, yaw: start_yaw });
    let mut c = camera(&d, CAM_MODE_WALL_CLIMB, eye0);
    assert_eq!(d.mode(c.setting, CAM_MODE_WALL_CLIMB).unwrap().func, "CAM_FUNC_JUMP2");
    update(&mut c, &d, &p, Vec3::ZERO);
    // While animTimer (R_CAM_DEFAULT_ANIM_TIME) runs, the yaw halves the way to behind him:
    // Camera_LERPCeilS(behind, atToEyeNext.yaw, 0.5, 0xA).
    // (atToEyeNextDir is measured from the at before it moves, through OLib's arctangent.)
    let s = diff_to_sph_geo(c.at, c.eye);
    let want = lerp_ceil_s(behind, diff_to_sph_geo(at0, eye0).yaw, 0.5, 0xA);
    assert!(yaw_close(s.yaw, want, ROUND_TRIP), "{:#x} {:#x}", s.yaw, want);
    // The distance from the moved at to the old eye, clamped to [minDist (1 - f), maxDist
    // (1 + f)] (the CLIMB data's values 1 to 3, scaled by Link's height and yNormal); no floor
    // anywhere, so the pitch stays the moved at's and the update rates go to 100.
    let v = |i: usize| d.value((c.setting, CAM_MODE_WALL_CLIMB), i) as f32;
    let (h, yn) = (p.height(), y_normal(&d, p.height()));
    let (min, max, fac) = (v(1) * 0.01 * h * yn, v(2) * 0.01 * h * yn, v(3) * 0.01);
    let r = c.at.distance(eye0).clamp(min - min * fac, max + max * fac);
    assert!((c.dist - r).abs() < 1e-3, "{} {r}", c.dist);
    assert_eq!((c.pitch_update_rate_inv, c.r_update_rate_inv), (100.0, 100.0));
    assert!(yaw_close(s.pitch, diff_to_sph_geo(c.at, eye0).pitch, ROUND_TRIP));
    assert!(c.eye.distance(c.eye_next) < 1e-3);
    // Once the timer's out, within yawAdj (0xA: the CLIMB data's flags & 2, else 0x2710) of
    // behind him.
    let yaw_adj = if d.value((c.setting, CAM_MODE_WALL_CLIMB), 8) & 2 != 0 { 0xA } else { 0x2710 };
    for _ in 0..d.oreg(23) + 5 {
        update(&mut c, &d, &p, Vec3::ZERO);
    }
    let s = diff_to_sph_geo(c.at, c.eye);
    assert!(yaw_close(s.yaw, behind, yaw_adj + 0xA), "{:#x}", s.yaw);
    assert_eq!(c.roll, 0);
}

#[test]
fn camera_jump1_keeps_its_distance_in_the_air() {
    let Some(d) = data() else { return };
    let p = player();
    // The eye 20 from the at and steep (0x3800): the distance clamps to distMin, the pitch to
    // R_CAM_MAX_PITCH (OREG(5)).
    let at0 = Vec3::new(0.0, p.height(), 0.0);
    let eye0 = sph_geo_add(at0, VecSphGeo { r: 20.0, pitch: 0x3800, yaw: 0i16.wrapping_sub(0x7FFF) });
    let mut c = camera(&d, CAM_MODE_JUMP, eye0);
    assert_eq!(d.mode(c.setting, CAM_MODE_JUMP).unwrap().func, "CAM_FUNC_JUMP1");
    update(&mut c, &d, &p, Vec3::ZERO);
    let v = |i: usize| d.value((c.setting, CAM_MODE_JUMP), i) as f32;
    let dist_min = v(1) * 0.01 * p.height() * y_normal(&d, p.height());
    // eyeDiffSph: the r eased from the old eye's towards the moved at's (OREG(29)), the pitch
    // likewise, then clamped.
    let old = diff_to_sph_geo(c.at, eye0);
    let pitch = lerp_ceil_s(old.pitch, 0x3800, d.oreg(29) as f32 * 0.01, 0xA).clamp(d.oreg(35), d.oreg(5));
    assert_eq!(pitch, d.oreg(5));
    // eyeNext's x and z at that distance and pitch; its y eased from the old eye's by OREG(31).
    let xz = Vec2::new(c.eye_next.x - c.at.x, c.eye_next.z - c.at.z).length();
    let want = dist_min * cos_s(pitch);
    assert!((xz - want).abs() < 0.05, "{xz} {want}");
    let new_y = c.at.y + dist_min * sin_s(pitch);
    let want_y = eye0.y + (new_y - eye0.y) * d.oreg(31) as f32 * 0.01;
    assert!((c.eye_next.y - want_y).abs() < 0.05, "{} {want_y}", c.eye_next.y);
    // xzOffsetUpdateRate from RELOAD_PARAMS' 1/10000 towards OREG(2) by OREG(25).
    let want = lerp_ceil_f(d.oreg(2) as f32 * 0.01, 1.0 / 10000.0, d.oreg(25) as f32 * 0.01, 0.1);
    assert_eq!(c.xz_offset_update_rate, want);
}

#[test]
fn camera_unique1_hangs_at_its_pitch_target() {
    let Some(d) = data() else { return };
    let p = player();
    let behind = 0i16.wrapping_sub(0x7FFF);
    let at0 = Vec3::new(0.0, p.height(), 0.0);
    let eye0 = sph_geo_add(at0, VecSphGeo { r: 120.0, pitch: 0x800, yaw: behind });
    let mut c = camera(&d, CAM_MODE_LEDGE_HANG, eye0);
    assert_eq!(d.mode(c.setting, CAM_MODE_LEDGE_HANG).unwrap().func, "CAM_FUNC_UNIQ1");
    // Link's waist off to his right (+x, yaw 0x4000), more than 0x3A98 from the eye's yaw: the
    // yaw target moves by ((diff / R_CAM_DEFAULT_ANIM_TIME) / 4) * 3 a frame for that many
    // frames.
    let waist = Vec3::new(30.0, 20.0, 0.0);
    let pitch_rate0 = c.pitch_update_rate_inv;
    update(&mut c, &d, &p, waist);
    let timer = d.oreg(23);
    let diff = diff_to_sph_geo(Vec3::ZERO, waist).yaw.wrapping_sub(diff_to_sph_geo(at0, eye0).yaw);
    assert!((diff as i32).abs() >= 0x3A98);
    let adj = (((diff as i32 / timer as i32) / 4) * 3) as i16;
    // The pitch: pitchUpdateRateInv eased towards 100 by OREG(25), then the pitch eased from
    // eyeNext's towards pitchTarget (the HANG data's value 3, degrees) by its inverse.
    let pitch_target = cam_deg_to_binang(d.value((c.setting, CAM_MODE_LEDGE_HANG), 3) as f32);
    let rate = lerp_ceil_f(100.0, pitch_rate0, d.oreg(25) as f32 * 0.01, 0.1);
    assert_eq!(c.pitch_update_rate_inv, rate);
    // (eyeNextAtOffset is measured from the at before it moves.)
    let s = diff_to_sph_geo(c.at, c.eye);
    let old = diff_to_sph_geo(at0, eye0);
    let want = lerp_ceil_s(pitch_target, old.pitch, 1.0 / rate, 0xA).clamp(d.oreg(5).wrapping_neg(), d.oreg(5));
    assert!(yaw_close(s.pitch, want, ROUND_TRIP), "{:#x} {:#x}", s.pitch, want);
    // The yaw: Camera_LERPFloorS(yawTarget, eyeNext's yaw, 0.5, 0x2710) each frame.
    let mut yaw_target = old.yaw.wrapping_add(adj);
    let mut yaw = lerp_floor_s(yaw_target, old.yaw, 0.5, 0x2710);
    assert!(yaw_close(s.yaw, yaw, ROUND_TRIP), "{:#x} {:#x}", s.yaw, yaw);
    for i in 1..timer + 5 {
        update(&mut c, &d, &p, waist);
        let cur = diff_to_sph_geo(c.at, c.eye_next).yaw;
        let prev = yaw;
        if i < timer {
            yaw_target = yaw_target.wrapping_add(adj);
        }
        yaw = lerp_floor_s(yaw_target, prev, 0.5, 0x2710);
        assert!(yaw_close(cur, yaw, ROUND_TRIP), "frame {i}: {cur:#x} {yaw:#x}");
        yaw = cur;
    }
    assert_eq!(c.roll, 0);
}
