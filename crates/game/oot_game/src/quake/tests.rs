//! `z_quake.c` frame by frame. The camera looks level along +z from (0, 50, -100), so
//! `OLib_Vec3fDiffToVecGeo(eye, at)` is { 100, pitch -1, yaw 0 } (`Math_FAtan2F(100, 0)` is 90
//! degrees: sph pitch 0x4000, geo pitch 0x3FFF - 0x4000), and the shakes point along the world's
//! axes: the y shake's geo pitch is -1 + 0x4000 = 0x3FFF, which `OLib_VecGeoToVec3f` turns back
//! into sph pitch 0 (straight up: (0, r, 0)); the x shake's is -1 at yaw 0x4000, sph pitch
//! 0x4000 (along +x: (r sin² 0x4000, r cos 0x4000, r sin 0x4000 cos 0x4000)). Expected values
//! are the C's arithmetic; the sines are `Math_SinS`'s.

use super::*;
use crate::camera::{CAM_ID_MAIN, CAM_MODE_NORMAL, CamFrame, CamView, CameraData, CameraGlobals, GameCamera, PlayerView, calc_up, cam_binang_to_deg};
use crate::letterbox::Letterbox;
use crate::onepoint::OnePointStatics;
use eng_collision::bgcheck::CollisionContext;
use eng_input::pad::Input;
use eng_math::{cos_s, sin_s};

const EYE: Vec3 = Vec3::new(0.0, 50.0, -100.0);
const AT: Vec3 = Vec3::new(0.0, 50.0, 0.0);

/// `play->cameraPtrs` with the main camera only.
fn main_only() -> [Option<(Vec3, Vec3)>; NUM_CAMS] {
    [Some((EYE, AT)), None, None, None]
}

/// The tag `Quake_RequestImpl` puts on the request it makes with `rand` as it is:
/// `(s16)(Rand_ZeroOne() * 0x10000) & ~3`.
fn tag(rand: &Rand) -> i16 {
    let mut r = *rand;
    ((r.zero_one() * 65536.0) as i32 as i16) & !3
}

fn close(a: Vec3, b: Vec3) -> bool {
    (a - b).length() < 1e-5
}

#[test]
fn quake_door_shutter_slam_type_3_frame_by_frame() {
    // Door_Shutter's slam (z_door_shutter.c): Quake_Request(Play_GetCamera(play, CAM_ID_MAIN), 3),
    // Quake_SetSpeed(i, -32536), Quake_SetPerturbations(i, 2, 0, 0, 0), Quake_SetDuration(i, 10).
    let mut q = QuakeStatics::default();
    let mut rand = Rand::default();
    let want_index = tag(&rand);
    let i = q.quake_request(CAM_ID_MAIN, QUAKE_TYPE_3, &mut rand);
    // The first free slot is 0, so the index is the tag alone.
    assert_eq!(i, want_index);
    assert!(q.quake_set_speed(i, -32536));
    assert!(q.quake_set_perturbations(i, 2, 0, 0, 0));
    assert!(q.quake_set_duration(i, 10));
    assert_eq!(q.request_count, 1);
    assert_eq!(q.quake_get_time_left(i), 10);
    let rand_after_request = rand;
    // Math_SinS(req->speed * req->timer), the int product passed as an s16: -32536 is 33000
    // mod 0x10000, so (s16)(33000 t) for t = 10 down to 1. Even t lands just past 0, odd t just
    // short of -0x8000: the shake flips up and down as it decays by t / 10.
    let angles: [i16; 10] = [2320, -30680, 1856, -31144, 1392, -31608, 928, -32072, 464, -32536];
    for (k, &a) in angles.iter().enumerate() {
        let t = 10 - k as i16;
        let mut shake = ShakeInfo::default();
        let n = q.quake_update(CAM_ID_MAIN, &main_only(), &mut rand, &mut shake);
        if t == 1 {
            // The 10th frame's callback takes the timer to 0: the request is removed and its
            // offsets (computed) aren't applied.
            assert_eq!(n, 0);
            assert_eq!(shake, ShakeInfo::default());
            break;
        }
        assert_eq!(n, 1, "t {t}");
        // Quake_CallbackType3: xyOffset = Math_SinS(angle) * ((f32)timer / duration), x and y
        // alike; the y perturbation 2 straight up, the x perturbation 0.
        let xy = sin_s(a) * (t as f32 / 10.0);
        assert!(close(shake.eye_offset, Vec3::new(0.0, 2.0 * xy, 0.0)), "t {t}: {:?}", shake.eye_offset);
        assert_eq!(shake.at_offset, shake.eye_offset);
        // No roll and no fov: upPitchOffset and fovOffset are 0, not above the merge's 0, so
        // upYawOffset (0x8000 * y) isn't taken either.
        assert_eq!((shake.up_pitch_offset, shake.up_yaw_offset, shake.fov_offset), (0, 0, 0));
        // maxOffset: the offset's length times |speed| / 0x8000.
        assert!((shake.max_offset - (2.0 * xy).abs() * (32536.0 / 32768.0)).abs() < 1e-5);
        // The timer counts down once per Quake_Update.
        assert_eq!(q.quake_get_time_left(i), t - 1);
    }
    // Gone: the setters and Quake_GetTimeLeft find nothing; no Rand was drawn after the request.
    assert_eq!(q.request_count, 0);
    assert_eq!(q.quake_get_time_left(i), 0);
    assert!(!q.quake_set_speed(i, 1));
    assert!(!q.quake_remove_request(i));
    assert_eq!(q.requests[0].type_, QUAKE_TYPE_NONE);
    assert_eq!(q.requests[0].timer, -1);
    assert_eq!(rand, rand_after_request);
}

#[test]
fn quake_obj_lift_type_1_frame_by_frame() {
    // Obj_Lift's shake (z_obj_lift.c): Quake_Request(GET_ACTIVE_CAM(play), QUAKE_TYPE_1), speed
    // 10000, perturbations (2, 0, 0, 0), duration 20.
    let mut q = QuakeStatics::default();
    let mut rand = Rand::default();
    let i = q.quake_request(CAM_ID_MAIN, QUAKE_TYPE_1, &mut rand);
    q.quake_set_speed(i, 10000);
    q.quake_set_perturbations(i, 2, 0, 0, 0);
    q.quake_set_duration(i, 20);
    // (s16)(10000 t) for t = 20 down to 1.
    let angles: [i16; 20] = [3392, -6608, -16608, -26608, 28928, 18928, 8928, -1072, -11072, -21072, -31072, 24464, 14464, 4464, -5536, -15536, -25536, 30000, 20000, 10000];
    let mut reference = rand;
    for (k, &a) in angles.iter().enumerate() {
        let t = 20 - k as i16;
        let mut shake = ShakeInfo::default();
        let n = q.quake_update(CAM_ID_MAIN, &main_only(), &mut rand, &mut shake);
        // Quake_CallbackType1 draws one Rand_ZeroOne a frame (the x shake's scale), whatever x is.
        reference.zero_one();
        assert_eq!(rand, reference, "t {t}");
        if t == 1 {
            assert_eq!(n, 0);
            break;
        }
        assert_eq!(n, 1);
        // Sustaining: xyOffset = Math_SinS(angle), no decay; the x shake is 0 * Rand * xy.
        let xy = sin_s(a);
        assert!(close(shake.eye_offset, Vec3::new(0.0, 2.0 * xy, 0.0)), "t {t}: {:?}", shake.eye_offset);
    }
    assert_eq!(q.request_count, 0);
}

#[test]
fn quake_type_3_in_full_with_x_fov_and_roll() {
    // Every perturbation of a decaying quake: z_onepointdemo.c 3070's speed 22000 and duration
    // 10, with y 2, x 1, fov 200 and roll 0x3C.
    let mut q = QuakeStatics::default();
    let mut rand = Rand::default();
    let i = q.quake_request(CAM_ID_MAIN, QUAKE_TYPE_3, &mut rand);
    q.quake_set_speed(i, 22000);
    q.quake_set_perturbations(i, 2, 1, 200, 0x3C);
    q.quake_set_duration(i, 10);
    let (s, c) = (sin_s(0x4000), cos_s(0x4000));
    for t in (2..=10i16).rev() {
        let mut shake = ShakeInfo::default();
        assert_eq!(q.quake_update(CAM_ID_MAIN, &main_only(), &mut rand, &mut shake), 1);
        let a = (22000 * t as i32) as i16;
        let xy = sin_s(a) * (t as f32 / 10.0);
        // The y shake (r = 2 xy) straight up, then the x shake (r = 1 xy) along +x.
        let (yr, xr) = (2.0 * xy, 1.0 * xy);
        let want = Vec3::new(xr * s * s, yr + xr * c, xr * s * c);
        assert!(close(shake.eye_offset, want), "t {t}: {:?} {want:?}", shake.eye_offset);
        // fovOffset = (s16)(fov * y), upPitchOffset = (s16)(roll * y), upYawOffset =
        // (s16)(0x8000 * y); the merge takes each only above 0 (signed), so a frame shaking down
        // has neither.
        let (fov, pitch, yaw) = ((200.0 * xy) as i32 as i16, (60.0 * xy) as i32 as i16, (32768.0 * xy) as i32 as i16);
        assert_eq!(shake.fov_offset, fov.max(0), "t {t}");
        if pitch > 0 {
            assert_eq!((shake.up_pitch_offset, shake.up_yaw_offset), (pitch, yaw), "t {t}");
        } else {
            assert_eq!((shake.up_pitch_offset, shake.up_yaw_offset), (0, 0), "t {t}");
        }
    }
    // t = 10: (s16)220000 = 23392, Math_SinS of it about 0.78 (128.5 degrees): up, a fov of
    // (s16)(200 * 0.78) and a roll.
    let mut q = QuakeStatics::default();
    let i = q.quake_request(CAM_ID_MAIN, QUAKE_TYPE_3, &mut rand);
    q.quake_set_speed(i, 22000);
    q.quake_set_perturbations(i, 2, 1, 200, 0x3C);
    q.quake_set_duration(i, 10);
    let mut shake = ShakeInfo::default();
    q.quake_update(CAM_ID_MAIN, &main_only(), &mut rand, &mut shake);
    assert!((sin_s(23392) - 0.7826).abs() < 1e-3);
    assert_eq!(shake.fov_offset, (200.0 * sin_s(23392)) as i32 as i16);
    assert!(shake.fov_offset > 150 && shake.up_pitch_offset > 40);
}

#[test]
fn quake_world_relative_shake_and_orientation() {
    // Quake_SetOrientation with isRelativeToScreen 0: y straight up whatever the camera, x along
    // the orientation's pitch and yaw (here yaw 0x4000, pitch 0: geo pitch 0 is sph 0x3FFF).
    let mut q = QuakeStatics::default();
    let mut rand = Rand::default();
    let i = q.quake_request(CAM_ID_MAIN, QUAKE_TYPE_5, &mut rand);
    q.quake_set_speed(i, 400);
    q.quake_set_perturbations(i, 4, 5, 0, 0);
    q.quake_set_duration(i, 100);
    assert!(q.quake_set_orientation(i, 0, [0, 0x4000, 0]));
    let mut shake = ShakeInfo::default();
    // A camera looking down, which a screen-relative shake would follow.
    let cams = [Some((Vec3::new(0.0, 500.0, 0.0), Vec3::ZERO)), None, None, None];
    q.quake_update(CAM_ID_MAIN, &cams, &mut rand, &mut shake);
    // Quake_CallbackType5 at t = 100: (s16)40000 = -25536.
    let xy = sin_s(-25536);
    let (sp, cp, sy, cy) = (sin_s(0x3FFF), cos_s(0x3FFF), sin_s(0x4000), cos_s(0x4000));
    let xr = 5.0 * xy;
    let want = Vec3::new(xr * sp * sy, 4.0 * xy + xr * cp, xr * sp * cy);
    assert!(close(shake.eye_offset, want), "{:?} {want:?}", shake.eye_offset);
}

#[test]
fn quake_type_6_lasts_until_removed() {
    // CS_MISC_QUAKE_START (z_demo.c): type 6 on the active camera, speed 0x7FFF, (4, 0, 1000, 0),
    // duration 800; CS_MISC_QUAKE_STOP removes it.
    let mut q = QuakeStatics::default();
    let mut rand = Rand::default();
    let i = q.quake_request(CAM_ID_MAIN, QUAKE_TYPE_6, &mut rand);
    q.quake_set_speed(i, 0x7FFF);
    q.quake_set_perturbations(i, 4, 0, 1000, 0);
    q.quake_set_duration(i, 800);
    let mut shake = ShakeInfo::default();
    q.quake_update(CAM_ID_MAIN, &main_only(), &mut rand, &mut shake);
    // Quake_CallbackType6 counts down first: timer 799, (799 & 0xF) + 500 = 515, and
    // (s16)(0x7FFF * 515) = 16875005 - 257 * 0x10000 = 32253.
    let xy = sin_s(32253);
    assert!(close(shake.eye_offset, Vec3::new(0.0, 4.0 * xy, 0.0)));
    assert_eq!(shake.fov_offset, (1000.0 * xy) as i32 as i16);
    // Past its duration it goes on (the timer runs negative, and wraps), and it repeats every 16
    // frames.
    let mut ys = Vec::new();
    for _ in 0..1000 {
        let n = q.quake_update(CAM_ID_MAIN, &main_only(), &mut rand, &mut shake);
        assert_eq!(n, 1);
        ys.push(shake.eye_offset.y);
    }
    assert_eq!(q.requests[0].timer, 799 - 1000);
    assert_eq!(ys[900], ys[900 - 16]);
    assert!(q.quake_remove_request(i));
    assert_eq!(q.request_count, 0);
    assert_eq!(q.quake_update(CAM_ID_MAIN, &main_only(), &mut rand, &mut shake), 0);
}

#[test]
fn quake_types_2_and_4_draw_two_rands() {
    // Quake_CallbackType2: xyOffset = Rand_ZeroOne(), then x = Rand_ZeroOne() * xyOffset;
    // Quake_CallbackType4: xyOffset = Rand_ZeroOne() * timer / duration, then the same.
    for (ty, decay) in [(QUAKE_TYPE_2, false), (QUAKE_TYPE_4, true)] {
        let mut q = QuakeStatics::default();
        let mut rand = Rand::default();
        let i = q.quake_request(CAM_ID_MAIN, ty, &mut rand);
        q.quake_set_perturbations(i, 4, 1, 0, 0);
        q.quake_set_duration(i, 4);
        let mut reference = rand;
        let mut shake = ShakeInfo::default();
        q.quake_update(CAM_ID_MAIN, &main_only(), &mut rand, &mut shake);
        let xy = reference.zero_one() * if decay { 4.0 / 4.0 } else { 1.0 };
        let x = reference.zero_one() * xy;
        assert_eq!(rand, reference);
        let (s, c) = (sin_s(0x4000), cos_s(0x4000));
        let want = Vec3::new(x * s * s, 4.0 * xy + x * c, x * s * c);
        assert!(close(shake.eye_offset, want), "type {ty}");
    }
}

#[test]
fn quake_too_many_requests_replace_the_one_ending_soonest() {
    let mut q = QuakeStatics::default();
    let mut rand = Rand::default();
    let mut idx = Vec::new();
    for d in [30, 10, 20, 10] {
        let i = q.quake_request(CAM_ID_MAIN, QUAKE_TYPE_3, &mut rand);
        q.quake_set_duration(i, d);
        idx.push(i);
    }
    // Each request took the first free slot: the index's low two bits.
    assert_eq!(idx.iter().map(|i| i & 3).collect::<Vec<_>>(), [0, 1, 2, 3]);
    assert_eq!(q.request_count, 4);
    // Quake_GetFreeIndex with no slot free: the least timer, the first of equals (10 in slot 1;
    // the strict `timerMin > timer` keeps slot 1 over slot 3).
    assert_eq!(q.quake_get_free_index(), 1);
    let fifth = q.quake_request(CAM_ID_MAIN, QUAKE_TYPE_5, &mut rand);
    assert_eq!(fifth & 3, 1);
    assert_ne!(fifth, idx[1]);
    // The replaced request's index no longer finds it (another tag), the others still do.
    assert!(!q.quake_set_speed(idx[1], 100));
    assert!(q.quake_set_speed(fifth, 100));
    assert_eq!(q.quake_get_time_left(idx[0]), 30);
    // @bug (game): the count went to 5 with four slots, so it ends at 1 once they're all gone.
    assert_eq!(q.request_count, 5);
    for i in [idx[0], fifth, idx[2], idx[3]] {
        assert!(q.quake_remove_request(i));
    }
    assert_eq!(q.request_count, 1);
    // A free slot again: the first.
    assert_eq!(q.quake_get_free_index(), 0);
}

#[test]
fn quake_on_another_camera_counts_down_but_doesnt_shake_this_one() {
    let mut q = QuakeStatics::default();
    let mut rand = Rand::default();
    let i = q.quake_request(1, QUAKE_TYPE_5, &mut rand);
    q.quake_set_speed(i, 400);
    q.quake_set_perturbations(i, 2, 0, 0, 0);
    q.quake_set_duration(i, 5);
    let cams = [Some((EYE, AT)), Some((EYE, AT)), None, None];
    let mut shake = ShakeInfo::default();
    // The main camera's update runs the callback (the timer counts down) but merges nothing.
    assert_eq!(q.quake_update(CAM_ID_MAIN, &cams, &mut rand, &mut shake), 0);
    assert_eq!(shake, ShakeInfo::default());
    assert_eq!(q.quake_get_time_left(i), 4);
    // Then the sub camera's own update applies it, and counts it down again.
    assert_eq!(q.quake_update(1, &cams, &mut rand, &mut shake), 1);
    assert_eq!(q.quake_get_time_left(i), 3);
    // Its camera cleared (cameraPtrs[1] NULL): removed at the next Quake_Update.
    assert_eq!(q.quake_update(CAM_ID_MAIN, &main_only(), &mut rand, &mut shake), 0);
    assert_eq!(q.request_count, 0);
    assert_eq!(q.quake_get_time_left(i), 0);
}

#[test]
fn quake_init_clears_types_and_timers_only() {
    let mut q = QuakeStatics::default();
    let mut rand = Rand::default();
    let i = q.quake_request(CAM_ID_MAIN, QUAKE_TYPE_3, &mut rand);
    q.quake_set_speed(i, 1234);
    q.quake_set_duration(i, 9);
    q.quake_init();
    assert_eq!(q.request_count, 0);
    assert_eq!((q.requests[0].type_, q.requests[0].timer, q.requests[0].speed, q.requests[0].index), (QUAKE_TYPE_NONE, 0, 1234, i));
    assert_eq!(q.quake_get_request(i), None);
    // A negative duration never ends a decaying quake (@bug (game)): its callback returns the
    // timer unchanged.
    let i = q.quake_request(CAM_ID_MAIN, QUAKE_TYPE_3, &mut rand);
    q.quake_set_duration(i, -3);
    let mut shake = ShakeInfo::default();
    for _ in 0..5 {
        assert_eq!(q.quake_update(CAM_ID_MAIN, &main_only(), &mut rand, &mut shake), 1);
    }
    assert_eq!(q.quake_get_time_left(i), -3);
}

#[test]
fn quake_set_value_sets_one_field() {
    let mut q = QuakeStatics::default();
    let mut rand = Rand::default();
    let i = q.quake_request(CAM_ID_MAIN, QUAKE_TYPE_1, &mut rand);
    assert_eq!(q.quake_set_value(i, QUAKE_DURATION, 7), Some(0));
    assert_eq!((q.requests[0].timer, q.requests[0].duration), (7, 7));
    q.quake_set_value(i, QUAKE_ROLL, 9);
    q.quake_set_value(i, QUAKE_ORIENTATION_YAW, 0x100);
    q.quake_set_value(i, QUAKE_IS_RELATIVE_TO_SCREEN, 0);
    assert_eq!((q.requests[0].up_pitch_offset, q.requests[0].orientation, q.requests[0].is_relative_to_screen), (9, [0, 0x100, 0], 0));
    assert_eq!(q.quake_set_value(i.wrapping_add(4), QUAKE_SPEED, 1), None);
}

fn data() -> Option<CameraData> {
    let pack = crate::pack::GamePack::open_default().ok()?;
    Some(pack.game_data().ok()?.camera)
}

/// One `Camera_Update` of the main camera `c` with a child Link at the origin facing +z, in an
/// empty world, with the quakes `q`.
fn update(c: &mut GameCamera, d: &CameraData, q: &mut QuakeStatics, rand: &mut Rand) {
    let p = PlayerView { pos: Vec3::ZERO, shape_yaw: 0, shape_pitch: 0, world_yaw: 0, adult: false, run_speed_limit: 550, gravity: 0.0, climbing: false, state1: 0, iron_boots: false, focus_pos: Vec3::ZERO, focus_rot: [0; 3] };
    let col = CollisionContext::new(Default::default());
    let oc = crate::collision_check::OcLines::default();
    let f = CamFrame {
        col: &col,
        player: p,
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
        main_player_pos_rot: c.main_player_pos_rot(),
        player_waist: Vec3::ZERO,
        player_melee_weapon_active: false,
        health: 0x30,
        skybox_disabled: false,
        cameras: [Some((c.eye, c.at)), None, None, None],
    };
    let mut g = CameraGlobals::main_init();
    g.scene_init_letterbox_timer = 0;
    let mut op = OnePointStatics::new(&d.onepoint);
    c.update(d, &f, &mut Letterbox::new(), &mut g, &mut op, rand, q);
}

#[test]
fn quake_shakes_the_cameras_view() {
    let Some(d) = data() else { return };
    let p = PlayerView { pos: Vec3::ZERO, shape_yaw: 0, shape_pitch: 0, world_yaw: 0, adult: false, run_speed_limit: 550, gravity: 0.0, climbing: false, state1: 0, iron_boots: false, focus_pos: Vec3::ZERO, focus_rot: [0; 3] };
    let mut c = GameCamera::new(&d, &p);
    c.change_mode_flags(&d, CAM_MODE_NORMAL, 0);
    let mut rand = Rand::default();
    // No quake: the view is the camera's.
    let mut q = QuakeStatics::default();
    update(&mut c, &d, &mut q, &mut rand);
    let v = c.shaken_view();
    assert_eq!((v.eye, v.at, v.fov, c.quake_offset), (c.eye, c.at, c.fov, Vec3::ZERO));
    // 3070's quake with a roll: at t = 10, (s16)220000 = 23392.
    let i = q.quake_request(CAM_ID_MAIN, QUAKE_TYPE_3, &mut rand);
    q.quake_set_speed(i, 22000);
    q.quake_set_perturbations(i, 2, 1, 200, 0x3C);
    q.quake_set_duration(i, 10);
    update(&mut c, &d, &mut q, &mut rand);
    // Quake_UpdateShakeInfo on the eye and at Camera_Normal1 left.
    let xy = sin_s(23392);
    let g = crate::camera::diff_to_sph_geo(c.eye, c.at);
    let yv = crate::camera::sph_geo_to_vec3(VecSphGeo { r: 2.0 * xy, pitch: g.pitch.wrapping_add(0x4000), yaw: g.yaw });
    let xv = crate::camera::sph_geo_to_vec3(VecSphGeo { r: 1.0 * xy, pitch: g.pitch, yaw: g.yaw.wrapping_add(0x4000) });
    let offset = (Vec3::ZERO + yv) + xv;
    assert!(close(c.quake_offset, offset), "{:?} {offset:?}", c.quake_offset);
    // Camera_Update: viewAt = at + atOffset, viewEye = eye + eyeOffset, viewFov = fov +
    // CAM_BINANG_TO_DEG(fovOffset); the up vector from the shaken eye and at with
    // upPitchOffset and upYawOffset added; camDir and inputDir are the shaken view's.
    let v = c.shaken_view();
    assert!(close(v.eye, c.eye + offset) && close(v.at, c.at + offset));
    let fov_offset = (200.0 * xy) as i32 as i16;
    assert!((v.fov - (c.fov + cam_binang_to_deg(fov_offset))).abs() < 1e-5);
    let a = crate::camera::diff_to_sph_geo(v.eye, v.at);
    let (up_pitch, up_yaw) = ((60.0 * xy) as i32 as i16, (32768.0 * xy) as i32 as i16);
    assert!(close(c.up, calc_up(a.pitch.wrapping_add(up_pitch), a.yaw.wrapping_add(up_yaw), c.roll)));
    assert_eq!(c.cam_dir, [a.pitch, a.yaw, 0]);
    assert_eq!(c.input_dir, [a.pitch, a.yaw, 0]);
    assert_ne!(c.up, crate::camera::calc_up(a.pitch, a.yaw, c.roll));
    // Nine frames shake; the tenth ends it, and the view is the camera's again.
    for _ in 0..9 {
        update(&mut c, &d, &mut q, &mut rand);
    }
    assert_eq!(q.request_count, 0);
    let v = c.shaken_view();
    assert_eq!((v.eye, v.at, v.fov, c.quake_offset), (c.eye, c.at, c.fov, Vec3::ZERO));
}
