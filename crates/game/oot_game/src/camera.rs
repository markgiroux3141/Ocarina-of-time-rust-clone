//! The game camera from `z_camera.c`: `Camera_Init`, `Camera_InitPlayerSettings`, the
//! player-following part of `Camera_Update`, mode changes (`Camera_ChangeModeFlags`) and the
//! `CAM_SET_NORMAL0` mode functions:
//!
//! - `Camera_Normal1` (NORMAL and STILL), with `Camera_CalcAtDefault`, `Camera_ClampDist`,
//!   `Camera_CalcDefaultYaw`/`Pitch`, `Camera_GetPitchAdjFromFloorHeightDiffs` and the swing
//!   `func_80046E20` / `func_80045508`;
//! - `Camera_Parallel1` (TARGET: Z held with nothing to lock on to, and PUSHPULL), with
//!   `Camera_CalcAtForParallel` and `func_800458D4`;
//! - `Camera_KeepOn1` (FOLLOWTARGET: locked on to a non-enemy), with `Camera_CalcAtForLockOn`;
//! - the camera bgcheck (`Camera_BGCheckInfo`, `Camera_BGCheckCorner`, `Camera_GetFloorYLayer`);
//! - `Camera_UpdateInterface`'s letterbox half, driving `crate::letterbox`.
//!
//! Vector-sphere maths is `z_olib.c`; `Math_FAtan2F` is the Taylor-series version from
//! `code_800FCE80.c`. `OREG` values (`sOREGInit`) and each NORMAL0 mode's function and data
//! (`sCamSetNormal0Modes`) come from `z_camera_data.c`, through the asset pack (`CameraData`).
//! `PREG(75)` and `PREG(76)` are 0 (only the debug register editor sets them), so the
//! at-calculations skip their slope adjustment and take the fov-based off-ground branch.
//!
//! Modes whose function isn't ported (BATTLE's `Camera_Battle1`, TALK's `Camera_KeepOn3`,
//! JUMP, CLIMB, HANG...) run `Camera_Normal1` on NORMAL's data; `camera->mode` still changes
//! as in the game. Not modelled: other settings, bg-camera setting changes from the floor poly
//! (`Camera_ChangeBgCamIndex`), water and hot-room checks, quakes, the low-health wiggle, the
//! debug camera, the mode-change sounds, `func_80043F94` (scenes with the skybox disabled) and
//! the interface alpha.

use eng_collision::bgcheck::{self, CollisionContext, PolyId};
use eng_input::pad::{BTN_CLEFT, BTN_CRIGHT, Input};
use eng_math::{binang_to_rad, cos_s, is_zero, rad_to_binang, sin_s};
use glam::Vec3;

use crate::actor_ctx::ActorHandle;
use crate::letterbox::Letterbox;
use crate::surface::SurfaceType;

/// `BGCHECK_SCENE`: the bgId of static collision.
const BGCHECK_SCENE: i32 = 50;

/// Values from `z_camera_data.c` (read by `oot_import::tables`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CameraData {
    /// `sOREGInit`: `OREG(0)`..
    pub oreg: Vec<i16>,
    /// `sCamSetNormal0Modes`, indexed by `CAM_MODE_*`: each mode's function and data.
    pub normal0_modes: Vec<CamModeData>,
    /// `sCameraSettings[CAM_SET_NORMAL0].unk_00`: bit `n` set when mode `n` is valid.
    pub normal0_valid_modes: u32,
}

/// A `CameraMode` entry (`CAM_SETTING_MODE_ENTRY(func, data)`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CamModeData {
    /// `CAM_FUNC_*`, e.g. `CAM_FUNC_PARA1`.
    pub func: String,
    /// The data's symbol, e.g. `sSetNormal0ModeTargetData`.
    pub data: String,
    /// The `CAM_FUNCDATA_*` arguments in order (`values[i].val`).
    pub values: Vec<i16>,
}

impl CameraData {
    pub fn oreg(&self, n: usize) -> i16 {
        self.oreg.get(n).copied().unwrap_or(0)
    }
    /// `CAM_DATA_SCALED(OREG(n))`.
    fn oreg_s(&self, n: usize) -> f32 {
        self.oreg(n) as f32 * 0.01
    }
    /// NORMAL0's data for `mode`.
    pub fn mode(&self, mode: i16) -> Option<&CamModeData> {
        self.normal0_modes.get(mode as usize)
    }
    /// The value `i` of `mode`'s data (`GET_NEXT_RO_DATA` in order).
    fn value(&self, mode: i16, i: usize) -> i16 {
        self.mode(mode).and_then(|m| m.values.get(i)).copied().unwrap_or(0)
    }
}

// CAM_MODE_* (z64camera.h).
pub const CAM_MODE_NORMAL: i16 = 0;
pub const CAM_MODE_TARGET: i16 = 1;
pub const CAM_MODE_FOLLOWTARGET: i16 = 2;
pub const CAM_MODE_TALK: i16 = 3;
pub const CAM_MODE_BATTLE: i16 = 4;
pub const CAM_MODE_CLIMB: i16 = 5;
pub const CAM_MODE_FIRSTPERSON: i16 = 6;
pub const CAM_MODE_BOWARROW: i16 = 7;
pub const CAM_MODE_BOWARROWZ: i16 = 8;
pub const CAM_MODE_HOOKSHOT: i16 = 9;
pub const CAM_MODE_BOOMERANG: i16 = 10;
pub const CAM_MODE_SLINGSHOT: i16 = 11;
pub const CAM_MODE_CLIMBZ: i16 = 12;
pub const CAM_MODE_JUMP: i16 = 13;
pub const CAM_MODE_HANG: i16 = 14;
pub const CAM_MODE_HANGZ: i16 = 15;
pub const CAM_MODE_FREEFALL: i16 = 16;
pub const CAM_MODE_CHARGE: i16 = 17;
pub const CAM_MODE_STILL: i16 = 18;
pub const CAM_MODE_PUSHPULL: i16 = 19;
pub const CAM_MODE_FOLLOWBOOMERANG: i16 = 20;

// Named OREGs (regs.h).
const R_CAM_MAX_PITCH: usize = 5;
const R_CAM_DEFAULT_ANIM_TIME: usize = 23;
const R_CAM_MIN_PITCH_1: usize = 34;
const R_CAM_DEFAULT_PITCH_UPDATE_RATE_INV: usize = 7;
const R_CAM_PITCH_FLOOR_CHECK_NEAR_DIST_FAC: usize = 17;
const R_CAM_PITCH_FLOOR_CHECK_FAR_DIST_FAC: usize = 18;
const R_CAM_PITCH_FLOOR_CHECK_OFFSET_Y_FAC: usize = 19;
const R_CAM_PITCH_FLOOR_CHECK_NEAR_WEIGHT: usize = 20;
const R_CAM_AT_LERP_STEP_SCALE_MIN: usize = 41;
const R_CAM_AT_LERP_STEP_SCALE_FAC: usize = 42;
const R_CAM_YOFFSET_NORM: usize = 46;

// ---------------------------------------------------------------------------------------------
// Maths (z_olib.c, code_800FCE80.c, z_camera.c helpers)
// ---------------------------------------------------------------------------------------------

/// `VecSph`: radius, pitch, yaw. "Geo" variants measure pitch from the horizon.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct VecSph {
    pub r: f32,
    pub pitch: i16,
    pub yaw: i16,
}

fn atan_taylor_qf(x: f32) -> f32 {
    const COEFFS: [f32; 9] = [-1.0 / 3.0, 1.0 / 5.0, -1.0 / 7.0, 1.0 / 9.0, -1.0 / 11.0, 1.0 / 13.0, -1.0 / 15.0, 1.0 / 17.0, 0.0];
    let mut poly = x;
    let sq = x * x;
    let mut exp = x * sq;
    for c in COEFFS {
        let term = c * exp;
        if poly + term == poly {
            break;
        }
        poly += term;
        exp *= sq;
    }
    poly
}

/// `Math_FAtanTaylorF`.
fn atan_taylor_f(x: f32) -> f32 {
    let t = x.abs();
    if x == 0.0 {
        return 0.0;
    }
    if x.is_nan() {
        return f32::NAN;
    }
    let s2 = std::f32::consts::SQRT_2;
    if t <= s2 - 1.0 {
        return atan_taylor_qf(x);
    }
    let q = if t >= s2 + 1.0 {
        std::f32::consts::FRAC_PI_2 - atan_taylor_qf(1.0 / t)
    } else {
        std::f32::consts::FRAC_PI_4 - atan_taylor_qf((1.0 - t) / (1.0 + t))
    };
    if x > 0.0 { q } else { -q }
}

/// `Math_FAtan2F(y, x)` (`gUseAtanContFrac` is 0, so the Taylor series).
pub fn f_atan2f(y: f32, x: f32) -> f32 {
    use std::f32::consts::{FRAC_PI_2, PI};
    if x == 0.0 {
        if y == 0.0 {
            0.0
        } else if y > 0.0 {
            FRAC_PI_2
        } else {
            -FRAC_PI_2
        }
    } else if x >= 0.0 {
        atan_taylor_f(y / x)
    } else if y < 0.0 {
        atan_taylor_f(y / x) - PI
    } else {
        PI - atan_taylor_f(-(y / x))
    }
}

/// `RAD_TO_DEG`.
fn rad_to_deg(r: f32) -> f32 {
    r * (180.0 / std::f32::consts::PI)
}

/// `CAM_DEG_TO_BINANG`: `(s16)((degrees) * 182.04167f + .5f)`.
pub fn cam_deg_to_binang(d: f32) -> i16 {
    (d * 182.04167 + 0.5) as i32 as i16
}

/// `CAM_BINANG_TO_DEG` (the C's constants as written).
#[allow(clippy::excessive_precision)]
fn cam_binang_to_deg(b: i16) -> f32 {
    b as f32 * (360.0001525 / 65535.0)
}

/// `DEG_TO_RAD`.
fn deg_to_rad(d: f32) -> f32 {
    d * (std::f32::consts::PI / 180.0)
}

/// `OLib_Vec3fToVecSph`.
pub fn vec3_to_sph(v: Vec3) -> VecSph {
    let dist_sq = v.x * v.x + v.z * v.z;
    let dist = dist_sq.sqrt();
    let pitch = if dist == 0.0 && v.y == 0.0 { 0 } else { cam_deg_to_binang(rad_to_deg(f_atan2f(dist, v.y))) };
    let r = (v.y * v.y + dist_sq).sqrt();
    let yaw = if v.x == 0.0 && v.z == 0.0 { 0 } else { cam_deg_to_binang(rad_to_deg(f_atan2f(v.x, v.z))) };
    VecSph { r, pitch, yaw }
}

/// `OLib_Vec3fToVecSphGeo`.
pub fn vec3_to_sph_geo(v: Vec3) -> VecSph {
    let mut s = vec3_to_sph(v);
    s.pitch = 0x3FFFi16.wrapping_sub(s.pitch);
    s
}

/// `OLib_Vec3fDiffToVecSphGeo(a, b)`: geographic coordinates of `b - a`.
pub fn diff_to_sph_geo(a: Vec3, b: Vec3) -> VecSph {
    vec3_to_sph_geo(b - a)
}

/// `OLib_VecSphToVec3f`.
pub fn sph_to_vec3(s: VecSph) -> Vec3 {
    let (sp, cp, sy, cy) = (sin_s(s.pitch), cos_s(s.pitch), sin_s(s.yaw), cos_s(s.yaw));
    Vec3::new(s.r * sp * sy, s.r * cp, s.r * sp * cy)
}

/// `OLib_VecSphGeoToVec3f`.
pub fn sph_geo_to_vec3(s: VecSph) -> Vec3 {
    sph_to_vec3(VecSph { r: s.r, pitch: 0x3FFFi16.wrapping_sub(s.pitch), yaw: s.yaw })
}

/// `Camera_Vec3fVecSphGeoAdd`.
pub fn sph_geo_add(a: Vec3, s: VecSph) -> Vec3 {
    a + sph_geo_to_vec3(s)
}

fn dist_xz(a: Vec3, b: Vec3) -> f32 {
    ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

/// `OLib_ClampMinDist`.
fn clamp_min_dist(v: f32, min: f32) -> f32 {
    if min <= v.abs() { v } else if v >= 0.0 { min } else { -min }
}

/// `OLib_ClampMaxDist`.
fn clamp_max_dist(v: f32, max: f32) -> f32 {
    if v.abs() <= max { v } else if v >= 0.0 { max } else { -max }
}

/// `OLib_Vec3fDistNormalize(a, b)`: unit vector from `a` to `b`.
fn dist_normalize(a: Vec3, b: Vec3) -> Vec3 {
    let v = b - a;
    v / clamp_min_dist(v.length(), 0.01)
}

/// `Camera_LERPCeilF`.
pub fn lerp_ceil_f(target: f32, cur: f32, step_scale: f32, min_diff: f32) -> f32 {
    let diff = target - cur;
    if diff.abs() >= min_diff { cur + diff * step_scale } else { target }
}

/// `Camera_LERPCeilS`.
pub fn lerp_ceil_s(target: i16, cur: i16, step_scale: f32, min_diff: i16) -> i16 {
    let diff = target.wrapping_sub(cur);
    if (diff as i32).abs() >= min_diff as i32 {
        let step = (diff as f32 * step_scale + 0.5) as i32 as i16;
        (cur as i32 + step as i32) as i16
    } else {
        target
    }
}

/// `Camera_LERPCeilVec3f`.
fn lerp_ceil_vec3(target: Vec3, cur: &mut Vec3, y_step: f32, xz_step: f32, min_diff: f32) {
    cur.x = lerp_ceil_f(target.x, cur.x, xz_step, min_diff);
    cur.y = lerp_ceil_f(target.y, cur.y, y_step, min_diff);
    cur.z = lerp_ceil_f(target.z, cur.z, xz_step, min_diff);
}

/// `Camera_InterpolateCurve`.
pub fn interpolate_curve(a: f32, b: f32) -> f32 {
    let t = 0.4f32;
    let abs_b = b.abs();
    if a < abs_b {
        1.0
    } else {
        let t2 = 1.0 - t;
        if a * t2 > abs_b {
            (b * b * (1.0 - t)) / (a * t2).powi(2)
        } else {
            1.0 - ((a - abs_b).powi(2) * t) / (0.4 * a).powi(2)
        }
    }
}

/// `Math3D_Cos`.
fn math3d_cos(a: Vec3, b: Vec3) -> f32 {
    let m = a.length() * b.length();
    if is_zero(m) { 0.0 } else { a.dot(b) / m }
}

/// `Math3D_PlaneVsPlaneNewLine`: a point and direction of the planes' intersection.
fn plane_vs_plane_line(a: Vec3, a_dist: f32, b: Vec3, b_dist: f32) -> Option<(Vec3, Vec3)> {
    let dir = a.cross(b);
    if is_zero(dir.x) && is_zero(dir.y) && is_zero(dir.z) {
        return None;
    }
    // Math3D_FindPointOnPlaneIntersect(a1, a2, b1, b2, axis3Dir, aDist, bDist).
    let find = |a1: f32, a2: f32, b1: f32, b2: f32, d: f32| ((a2 * b_dist - b2 * a_dist) / d, (b1 * a_dist - a1 * b_dist) / d);
    let (dx, dy, dz) = (dir.x.abs(), dir.y.abs(), dir.z.abs());
    let point = if dx >= dy && dx >= dz {
        let (y, z) = find(a.y, a.z, b.y, b.z, dir.x);
        Vec3::new(0.0, y, z)
    } else if dy >= dx && dy >= dz {
        let (z, x) = find(a.z, a.x, b.z, b.x, dir.y);
        Vec3::new(x, 0.0, z)
    } else {
        let (x, y) = find(a.x, a.y, b.x, b.y, dir.z);
        Vec3::new(x, y, 0.0)
    };
    Some((point, dir))
}

/// `Math3D_LineVsLineClosestTwoPoints`: the point on line A closest to line B.
fn line_vs_line_closest(a0: Vec3, a1: Vec3, b0: Vec3, b1: Vec3) -> Option<Vec3> {
    let la = a1 - a0;
    let lb = b1 - b0;
    let sq = lb.length_squared();
    if is_zero(sq) {
        return None;
    }
    let scale = 1.0 / sq;
    let comp_a = la.dot(lb) * scale;
    let comp_ba = lb.dot(a0 - b0) * scale;
    let a_perp = la - lb * comp_a;
    let sq = a_perp.length_squared();
    if is_zero(sq) {
        return None;
    }
    let ba_perp = (a0 - b0) - lb * comp_ba;
    let t = -a_perp.dot(ba_perp) / sq;
    Some(la * t + a0)
}

// ---------------------------------------------------------------------------------------------
// The camera
// ---------------------------------------------------------------------------------------------

/// `CamColChk`.
#[derive(Debug, Clone, Copy, Default)]
struct ColChk {
    pos: Vec3,
    norm: Vec3,
    poly: Option<PolyId>,
    sph_norm: VecSph,
}

/// `SwingAnimation`.
#[derive(Debug, Clone, Copy, Default)]
struct Swing {
    collision_close_point: Vec3,
    at_eye_poly: Option<PolyId>,
    swing_update_rate: f32,
    unk_14: i16,
    unk_16: i16,
    unk_18: i16,
    swing_update_rate_timer: i16,
}

/// `Normal1ReadOnlyData`.
#[derive(Debug, Clone, Copy, Default)]
struct Norm1Ro {
    y_offset: f32,
    dist_min: f32,
    dist_max: f32,
    unk_0c: f32,
    unk_10: f32,
    unk_14: f32,
    fov_target: f32,
    at_lerp_scale_max: f32,
    pitch_target: i16,
    interface_flags: i16,
}

/// `Normal1ReadWriteData`.
#[derive(Debug, Clone, Copy, Default)]
struct Norm1Rw {
    swing: Swing,
    y_offset: f32,
    unk_20: f32,
    slope_pitch_adj: i16,
    swing_yaw_target: i16,
    unk_28: i16,
    start_swing_timer: i16,
}

/// `Parallel1ReadOnlyData`.
#[derive(Debug, Clone, Copy, Default)]
struct Para1Ro {
    y_offset: f32,
    dist_target: f32,
    pitch_target: i16,
    yaw_target: i16,
    unk_08: f32,
    unk_0c: f32,
    fov_target: f32,
    unk_14: f32,
    interface_flags: i16,
    unk_18: f32,
    unk_1c: f32,
}

/// `Parallel1ReadWriteData`.
#[derive(Debug, Clone, Copy, Default)]
struct Para1Rw {
    unk_00_x: f32,
    y_target: f32,
    unk_10: i16,
    yaw_target: i16,
    pitch_target: i16,
    unk_16: i16,
    anim_timer: i16,
}

/// `KeepOn1ReadOnlyData`.
#[derive(Debug, Clone, Copy, Default)]
struct Keep1Ro {
    unk_00: f32,
    unk_04: f32,
    unk_08: f32,
    unk_0c: f32,
    unk_10: f32,
    unk_14: f32,
    unk_18: f32,
    unk_1c: f32,
    unk_20: f32,
    unk_24: f32,
    interface_flags: i16,
    unk_28: f32,
    unk_2c: f32,
}

/// `KeepOn1ReadWriteData`.
#[derive(Debug, Clone, Copy, Default)]
struct Keep1Rw {
    unk_00: f32,
    unk_04: f32,
    unk_08: f32,
    unk_0c: Option<ActorHandle>,
    unk_10: i16,
    unk_12: i16,
    unk_14: i16,
    unk_16: i16,
}

/// What `Camera_Update` is given each frame besides the camera data.
pub struct CamFrame<'a> {
    pub col: &'a CollisionContext,
    pub player: PlayerView,
    /// `Actor_GetFocus(camera->target)`, or `None` when there's no target or it was killed
    /// (`target->update == NULL`).
    pub target_focus: Option<Vec3>,
    /// `play->transitionMode != TRANS_MODE_OFF`.
    pub transitioning: bool,
    /// `play->state.frames`.
    pub frames: u32,
}

/// What `Camera_Update` reads from Player each frame.
#[derive(Debug, Clone, Copy)]
pub struct PlayerView {
    /// `actor.world.pos`.
    pub pos: Vec3,
    /// `actor.shape.rot.y`.
    pub shape_yaw: i16,
    pub adult: bool,
    /// `R_RUN_SPEED_LIMIT` (for `func_8002DCE4`).
    pub run_speed_limit: i16,
    /// `actor.gravity`.
    pub gravity: f32,
    /// `stateFlags1 & PLAYER_STATE1_21` (climbing).
    pub climbing: bool,
}

impl PlayerView {
    /// `Player_GetHeight` (not riding Epona).
    pub fn height(&self) -> f32 {
        if self.adult { 68.0 } else { 44.0 }
    }
}

#[derive(Debug, Clone)]
pub struct GameCamera {
    pub eye: Vec3,
    pub at: Vec3,
    pub eye_next: Vec3,
    pub fov: f32,
    pub roll: i16,
    pub dist: f32,
    /// `inputDir` (pitch, yaw, roll): what `Camera_GetInputDirYaw` returns (`.1`).
    pub input_dir: [i16; 3],
    pub up: Vec3,
    player_pos: Vec3,
    player_rot_y: i16,
    xz_speed: f32,
    speed_ratio: f32,
    player_ground_y: f32,
    floor_norm: Vec3,
    player_floor_poly: Option<PolyId>,
    pos_offset: Vec3,
    at_lerp_step_scale: f32,
    yaw_update_rate_inv: f32,
    pitch_update_rate_inv: f32,
    r_update_rate_inv: f32,
    xz_offset_update_rate: f32,
    y_offset_update_rate: f32,
    fov_update_rate: f32,
    anim_state: i16,
    /// `mode`: a `CAM_MODE_*`.
    pub mode: i16,
    /// `unk_14A`: bits 0x20 and 2 mark a mode request this frame.
    pub unk_14a: i16,
    /// `unk_14C`: 0x20 while `Camera_Parallel1` animates (mode requests are then refused).
    pub unk_14c: i16,
    /// `paramFlags` (`Camera_SetParam`): 8 once `target` is set.
    pub param_flags: i16,
    /// `target` (`Camera_SetParam(camera, 8, actor)`): what KEEPON looks at.
    pub target: Option<ActorHandle>,
    /// `targetPosRot.pos`.
    target_pos: Vec3,
    /// `playerPosDelta`.
    player_pos_delta: Vec3,
    /// `sCameraInterfaceFlags`: the letterbox (`0xF000`) and interface alpha (`0x0F00`) bits.
    pub interface_flags: i16,
    ro: Norm1Ro,
    rw: Norm1Rw,
    para1_ro: Para1Ro,
    para1_rw: Para1Rw,
    keep1_ro: Keep1Ro,
    keep1_rw: Keep1Rw,
    update_direction: bool,
    oob_timer: u32,
    // Statics of Camera_GetPitchAdjFromFloorHeightDiffs and func_80046E20.
    floor_y_near: f32,
    floor_y_far: f32,
    far_col_chk: ColChk,
    at_eye_col_chk: ColChk,
    eye_at_col_chk: ColChk,
    new_eye_col_chk: ColChk,
}

impl GameCamera {
    /// `Camera_Init` + `Camera_InitPlayerSettings` (+ `func_80057FC4` picking NORMAL0 for a
    /// room with `behaviorType1` 0, and `Camera_CopyDataToRegs` setting `animState` 0).
    pub fn new(d: &CameraData, p: &PlayerView) -> GameCamera {
        let h = p.height();
        let yaw = p.shape_yaw.wrapping_sub(0x7FFF);
        let at = p.pos + Vec3::Y * h;
        let off = VecSph { r: 180.0, pitch: 0x71C, yaw };
        let eye_next = sph_geo_add(at, off);
        GameCamera {
            eye: eye_next,
            at,
            eye_next,
            fov: 60.0,
            roll: 0,
            dist: 180.0,
            input_dir: [0x71C, p.shape_yaw, 0],
            up: Vec3::Y,
            player_pos: p.pos,
            player_rot_y: p.shape_yaw,
            xz_speed: 0.0,
            speed_ratio: 0.0,
            player_ground_y: 0.0,
            floor_norm: Vec3::Y,
            player_floor_poly: None,
            pos_offset: Vec3::new(0.0, h, 0.0),
            at_lerp_step_scale: 1.0,
            yaw_update_rate_inv: 10.0,
            pitch_update_rate_inv: d.oreg(R_CAM_DEFAULT_PITCH_UPDATE_RATE_INV) as f32,
            r_update_rate_inv: 10.0,
            xz_offset_update_rate: d.oreg_s(2),
            y_offset_update_rate: d.oreg_s(3),
            fov_update_rate: d.oreg_s(4),
            anim_state: 0,
            mode: CAM_MODE_NORMAL,
            unk_14a: 0,
            // Camera_Init: 0x4000; Camera_InitPlayerSettings: |= 4.
            unk_14c: 0x4000 | 4,
            param_flags: 0,
            target: None,
            target_pos: Vec3::ZERO,
            player_pos_delta: Vec3::ZERO,
            // Camera_InitPlayerSettings, for the main camera.
            interface_flags: 0xB200u16 as i16,
            ro: Norm1Ro::default(),
            rw: Norm1Rw::default(),
            para1_ro: Para1Ro::default(),
            para1_rw: Para1Rw::default(),
            keep1_ro: Keep1Ro::default(),
            keep1_rw: Keep1Rw::default(),
            update_direction: false,
            oob_timer: 0,
            floor_y_near: 0.0,
            floor_y_far: 0.0,
            far_col_chk: ColChk::default(),
            at_eye_col_chk: ColChk::default(),
            eye_at_col_chk: ColChk::default(),
            new_eye_col_chk: ColChk::default(),
        }
    }

    /// `Camera_GetInputDirYaw`.
    pub fn input_dir_yaw(&self) -> i16 {
        self.input_dir[1]
    }

    /// `Camera_SetParam(camera, 8, actor)`: the actor KEEPON and BATTLE look at.
    pub fn set_target(&mut self, actor: ActorHandle) {
        self.target = Some(actor);
        self.param_flags &= !(0x10 | 0x8 | 0x1);
        self.param_flags |= 8;
    }

    /// `Camera_ChangeMode`: `Camera_ChangeModeFlags(camera, mode, 0)`.
    pub fn change_mode(&mut self, d: &CameraData, mode: i16) -> i32 {
        self.change_mode_flags(d, mode, 0)
    }

    /// `Camera_ChangeModeFlags`. Returns -1 when the request is refused or changes nothing,
    /// `0x80000000 | mode` for a change. The sound effects aren't modelled.
    pub fn change_mode_flags(&mut self, d: &CameraData, mode: i16, flags: u8) -> i32 {
        if self.unk_14c & 0x20 != 0 && flags == 0 {
            self.unk_14a |= 0x20;
            return -1;
        }
        if (d.normal0_valid_modes & 0x3FFF_FFFF) & (1u32 << mode) == 0 {
            if self.mode != CAM_MODE_NORMAL {
                self.mode = CAM_MODE_NORMAL;
                self.copy_data_to_regs();
                self.func_8005a02c();
                return (0xC000_0000u32 | mode as u32) as i32;
            }
            self.unk_14a |= 0x20 | 2;
            return 0;
        }
        if mode == self.mode && flags == 0 {
            self.unk_14a |= 0x20 | 2;
            return -1;
        }
        self.unk_14a |= 0x20 | 2;
        self.copy_data_to_regs();
        let mut mode_change_flags = match mode {
            CAM_MODE_FIRSTPERSON => 0x20,
            CAM_MODE_BATTLE => 4,
            // The boomerang (ACTOR_EN_BOOM) isn't ported.
            CAM_MODE_FOLLOWTARGET if self.target.is_some() => 8,
            CAM_MODE_TARGET | CAM_MODE_TALK | CAM_MODE_BOWARROWZ | CAM_MODE_HANGZ | CAM_MODE_PUSHPULL => 2,
            _ => 0,
        };
        match self.mode {
            CAM_MODE_FIRSTPERSON => {
                if mode_change_flags & 0x20 != 0 {
                    self.anim_state = 10;
                }
            }
            CAM_MODE_TARGET => {
                if mode_change_flags & 0x10 != 0 {
                    self.anim_state = 10;
                }
                mode_change_flags |= 1;
            }
            CAM_MODE_CHARGE => mode_change_flags |= 1,
            CAM_MODE_FOLLOWTARGET => {
                if mode_change_flags & 8 != 0 {
                    self.anim_state = 10;
                }
                mode_change_flags |= 1;
            }
            CAM_MODE_BATTLE => {
                if mode_change_flags & 4 != 0 {
                    self.anim_state = 10;
                }
                mode_change_flags |= 1;
            }
            CAM_MODE_BOWARROWZ | CAM_MODE_HANGZ | CAM_MODE_PUSHPULL => mode_change_flags |= 1,
            CAM_MODE_NORMAL => {
                if mode_change_flags & 0x10 != 0 {
                    self.anim_state = 10;
                }
            }
            _ => {}
        }
        // With CAM_STAT_ACTIVE, modeChangeFlags (1, 2, 4 or 8) picks a sound: not modelled.
        let _ = mode_change_flags;
        self.func_8005a02c();
        self.mode = mode;
        (0x8000_0000u32 | mode as u32) as i32
    }

    /// `Camera_CopyDataToRegs`: the `PREG` copy only feeds the debug register editor.
    fn copy_data_to_regs(&mut self) {
        self.anim_state = 0;
    }

    /// `func_8005A02C`.
    fn func_8005a02c(&mut self) {
        self.unk_14c |= 0xC;
        self.unk_14c &= !(0x1000 | 0x8);
    }

    /// `Camera_Update` for the main camera following Player, with `Camera_UpdateInterface`'s
    /// letterbox target at the end.
    pub fn update(&mut self, d: &CameraData, f: &CamFrame, letterbox: &mut Letterbox) {
        let (col, p, frames) = (f.col, &f.player, f.frames);
        self.update_direction = false;
        let cur = p.pos;
        self.xz_speed = dist_xz(cur, self.player_pos);
        // func_8002DCE4: R_RUN_SPEED_LIMIT / 100 when not riding or swimming.
        self.speed_ratio = clamp_max_dist(self.xz_speed / ((p.run_speed_limit as f32 / 100.0) * d.oreg_s(8)), 1.0);
        self.player_pos_delta = cur - self.player_pos;
        let pos = cur + Vec3::Y * p.height();
        // BgCheck_EntityRaycastDown5.
        let (ground, poly) = col.entity_raycast_down(pos);
        if ground != bgcheck::BGCHECK_Y_MIN {
            self.oob_timer = 0;
            self.floor_norm = poly.map(|id| col.poly_normal(id)).unwrap_or(Vec3::Y);
            self.player_ground_y = ground;
            self.player_floor_poly = poly;
        } else {
            self.oob_timer += 1;
            self.floor_norm = Vec3::Y;
        }
        self.player_pos = cur;
        self.player_rot_y = p.shape_yaw;

        self.unk_14a = 0;
        self.unk_14c &= !(0x400 | 0x20);
        self.unk_14c |= 0x10;
        if self.oob_timer < 200 {
            // sCameraFunctions[sCameraSettings[setting].cameraModes[mode].funcIdx].
            match d.mode(self.mode).map(|m| m.func.as_str()) {
                Some("CAM_FUNC_NORM1") => self.normal1(d, col, p, self.mode, frames),
                Some("CAM_FUNC_PARA1") => self.parallel1(d, col, p, frames),
                Some("CAM_FUNC_KEEP1") => self.keep_on1(d, col, p, f.target_focus),
                _ => self.normal1(d, col, p, CAM_MODE_NORMAL, frames),
            }
        } else {
            let e = diff_to_sph_geo(self.at, self.eye);
            self.calc_at_default(d, &e, 0.0, false, p);
        }

        // CAM_STAT_ACTIVE: a running transition holds the interface (0xF200).
        if f.transitioning {
            self.interface_flags = 0xF200u16 as i16;
        }
        self.update_interface(letterbox);

        let angle = diff_to_sph_geo(self.eye, self.at);
        self.up = calc_up(angle.pitch, angle.yaw, self.roll);
        if !self.update_direction {
            self.input_dir = [angle.pitch, angle.yaw, 0];
        }
    }

    /// `Camera_UpdateInterface(sCameraInterfaceFlags)`, the letterbox part: `flags & 0x7000`
    /// picks the size (`sCameraLetterboxSize`), `0x8000` sets it at once; all of `0xF000` set
    /// leaves the letterbox alone.
    fn update_interface(&self, letterbox: &mut Letterbox) {
        let flags = self.interface_flags as u16;
        if flags & 0xF000 != 0xF000 {
            let size = match flags & 0x7000 {
                0x1000 => 26,
                0x2000 => 27,
                0x3000 => 32,
                _ => 0,
            };
            if flags & 0x8000 != 0 {
                letterbox.set_size(size);
            } else {
                letterbox.set_size_target(size);
            }
        }
    }

    // ---- bgcheck ----------------------------------------------------------------------

    /// `Camera_BGCheckInfo`: returns 0 when nothing is between `from` and `to.pos`.
    fn bg_check_info(col: &CollisionContext, from: Vec3, to: &mut ColChk) -> i32 {
        let mut off = diff_to_sph_geo(from, to.pos);
        off.r += 8.0;
        let to_point = sph_geo_add(from, off);
        // BgCheck_CameraLineTest1(.., chkWall 1, chkFloor 1, chkCeil 1, chkOneFace -1, ..): dyna included.
        let bcc = bgcheck::CHECK_WALL | bgcheck::CHECK_FLOOR | bgcheck::CHECK_CEILING | bgcheck::CHECK_ONE_FACE | bgcheck::CHECK_DYNA;
        let new_pos;
        match col.check_line(bgcheck::IGNORE_CAMERA, bgcheck::IGNORE_NONE, from, to_point, 1.0, bcc) {
            Some((hit, poly)) => {
                new_pos = hit;
                to.poly = Some(poly);
            }
            None => {
                let n = -dist_normalize(from, to.pos);
                to.norm = n;
                let mut np = to.pos;
                np.y += 5.0;
                // BgCheck_CameraRaycastDown2.
                let (fy, fpoly) = col.raycast_down(np, bgcheck::IGNORE_CAMERA, bgcheck::DOWN_CHECK_WALLS | bgcheck::DOWN_CHECK_FLOORS, 1.0);
                if to.pos.y - fy > 5.0 || fpoly.is_none() {
                    to.pos += n;
                    return 0;
                }
                to.poly = fpoly;
                np.y = fy + 1.0;
                new_pos = np;
            }
        }
        to.norm = to.poly.map(|id| col.poly_normal(id)).unwrap_or(Vec3::Y);
        to.pos = to.norm + new_pos;
        BGCHECK_SCENE + 1
    }

    /// `Camera_BGCheck`.
    fn bg_check(col: &CollisionContext, from: Vec3, to: &mut Vec3) -> bool {
        let mut c = ColChk { pos: *to, ..Default::default() };
        let r = Self::bg_check_info(col, from, &mut c);
        *to = c.pos;
        r != 0
    }

    /// `Camera_GetFloorYLayer`.
    fn floor_y_layer(&self, col: &CollisionContext, pos: &mut Vec3) -> f32 {
        let mut floor_y = bgcheck::BGCHECK_Y_MIN;
        for _ in 0..3 {
            let (fy, poly) = col.raycast_down(*pos, bgcheck::IGNORE_CAMERA, bgcheck::DOWN_CHECK_WALLS | bgcheck::DOWN_CHECK_FLOORS, 1.0);
            floor_y = fy;
            let Some(poly) = poly.filter(|_| fy != bgcheck::BGCHECK_Y_MIN) else {
                floor_y = bgcheck::BGCHECK_Y_MIN;
                break;
            };
            if self.player_ground_y < fy && !(col.poly_normal(poly).y > 0.5) {
                floor_y = bgcheck::BGCHECK_Y_MIN;
                break;
            } else if col.floor_type(poly) == 1 {
                // FLOOR_TYPE_1: not solid for the camera, look below.
                pos.y = fy - 10.0;
                continue;
            } else {
                break;
            }
        }
        floor_y
    }

    /// `Camera_BGCheckCorner` (`func_800427B4` → `Math3D_PlaneVsLineSegClosestPoint`).
    fn bg_check_corner(col: &CollisionContext, a: Vec3, b: Vec3, ca: &ColChk, cb: &ColChk) -> Vec3 {
        let plane = |c: &ColChk| c.poly.map(|id| (col.poly_normal(id), col.poly(id).dist as f32));
        if let (Some((na, da)), Some((nb, db))) = (plane(ca), plane(cb))
            && let Some((point, dir)) = plane_vs_plane_line(na, da, nb, db)
            && let Some(p) = line_vs_line_closest(point, dir * 100.0 + point, a, b)
        {
            return p;
        }
        ca.pos
    }

    // ---- Camera_Normal1 helpers --------------------------------------------------------

    /// `Camera_GetPitchAdjFromFloorHeightDiffs`.
    fn pitch_adj_from_floor(&mut self, d: &CameraData, col: &CollisionContext, p: &PlayerView, view_yaw: i16, init: bool, frames: u32) -> i16 {
        let (fx, fz) = (sin_s(view_yaw), cos_s(view_yaw));
        let h = p.height();
        let check_y = d.oreg_s(R_CAM_PITCH_FLOOR_CHECK_OFFSET_Y_FAC) * h;
        let mut near_dist = d.oreg_s(R_CAM_PITCH_FLOOR_CHECK_NEAR_DIST_FAC) * h;
        let mut far_dist = d.oreg_s(R_CAM_PITCH_FLOOR_CHECK_FAR_DIST_FAC) * h;
        let player = Vec3::new(self.player_pos.x, self.player_ground_y + check_y, self.player_pos.z);
        let mut near = Vec3::new(player.x + near_dist * fx, player.y, player.z + near_dist * fz);
        if init || frames % 2 == 0 {
            self.far_col_chk.pos = Vec3::new(player.x + far_dist * fx, player.y, player.z + far_dist * fz);
            Self::bg_check_info(col, player, &mut self.far_col_chk);
            if init {
                self.floor_y_near = self.player_ground_y;
                self.floor_y_far = self.player_ground_y;
            }
        } else {
            far_dist = dist_xz(player, self.far_col_chk.pos);
            self.far_col_chk.pos += self.far_col_chk.norm * 5.0;
            let mut far_pos = self.far_col_chk.pos;
            if near_dist > far_dist {
                near_dist = far_dist;
                let y = self.floor_y_layer(col, &mut far_pos);
                self.floor_y_near = y;
                self.floor_y_far = y;
            } else {
                self.floor_y_near = self.floor_y_layer(col, &mut near);
                self.floor_y_far = self.floor_y_layer(col, &mut far_pos);
            }
            self.far_col_chk.pos = far_pos;
            if self.floor_y_near == bgcheck::BGCHECK_Y_MIN {
                self.floor_y_near = self.player_ground_y;
            }
            if self.floor_y_far == bgcheck::BGCHECK_Y_MIN {
                self.floor_y_far = self.floor_y_near;
            }
        }
        let w = d.oreg_s(R_CAM_PITCH_FLOOR_CHECK_NEAR_WEIGHT);
        let near_diff = w * (self.floor_y_near - self.player_ground_y);
        let far_diff = (1.0 - w) * (self.floor_y_far - self.player_ground_y);
        let pn = cam_deg_to_binang(rad_to_deg(f_atan2f(near_diff, near_dist)));
        let pf = cam_deg_to_binang(rad_to_deg(f_atan2f(far_diff, far_dist)));
        pn.wrapping_add(pf)
    }

    /// `Camera_CalcAtDefault`.
    fn calc_at_default(&mut self, d: &CameraData, eye_at_dir: &VecSph, extra_y: f32, calc_slope: bool, p: &PlayerView) {
        let h = p.height();
        let mut target = Vec3::new(0.0, h + extra_y, 0.0);
        if calc_slope {
            target.y -= clamp_max_dist(calc_slope_y_adj(self.floor_norm, self.player_rot_y, eye_at_dir.yaw, d.oreg(9) as f32), h);
        }
        let mut off = self.pos_offset;
        lerp_ceil_vec3(target, &mut off, self.y_offset_update_rate, self.xz_offset_update_rate, 0.1);
        self.pos_offset = off;
        let at_target = self.player_pos + self.pos_offset;
        let mut at = self.at;
        lerp_ceil_vec3(at_target, &mut at, self.at_lerp_step_scale, self.at_lerp_step_scale, 0.2);
        self.at = at;
    }

    /// `Camera_ClampDist`.
    fn clamp_dist(&mut self, d: &CameraData, dist: f32, min: f32, max: f32, timer: i16) -> f32 {
        let o6 = d.oreg(6) as f32;
        let (target, r_target) = if dist < min {
            (min, if timer != 0 { o6 * 0.5 } else { o6 })
        } else if max < dist {
            (max, if timer != 0 { o6 * 0.5 } else { o6 })
        } else {
            (dist, if timer != 0 { o6 } else { 1.0 })
        };
        self.r_update_rate_inv = lerp_ceil_f(r_target, self.r_update_rate_inv, d.oreg_s(25), 0.1);
        lerp_ceil_f(target, self.dist, 1.0 / self.r_update_rate_inv, 0.2)
    }

    /// `Camera_CalcDefaultPitch`.
    fn calc_default_pitch(&self, d: &CameraData, cur: i16, target: i16, slope: i16) -> i16 {
        let abs_cur = (cur as i32).abs() as i16;
        let phi = if slope > 0 { (cos_s(slope) * slope as f32) as i16 } else { slope };
        let target = target.wrapping_sub(phi);
        let step = if ((target as i32).abs() as i16) < abs_cur {
            (1.0 / self.pitch_update_rate_inv) * 3.0
        } else {
            let t = abs_cur as f32 * (1.0 / d.oreg(R_CAM_MAX_PITCH) as f32);
            (1.0 / self.pitch_update_rate_inv) * interpolate_curve(0.8, 1.0 - t)
        };
        lerp_ceil_s(target, cur, step, 0xA)
    }

    /// `Camera_CalcDefaultYaw`.
    fn calc_default_yaw(&self, d: &CameraData, cur: i16, target: i16, arg3: f32, accel: f32) -> i16 {
        let ang_delta = target.wrapping_sub(cur.wrapping_sub(0x7FFF));
        let speed_t = if self.xz_speed > 0.001 {
            // COLPOLY_GET_NORMAL((s16)(angDelta - 0x7FFF)).
            ang_delta.wrapping_sub(0x7FFF) as f32 * (1.0 / 32767.0)
        } else {
            d.oreg_s(48)
        };
        let upd = interpolate_curve(arg3, speed_t);
        let velocity = (upd + (1.0 - upd) * accel).max(0.0);
        let vel_factor = interpolate_curve(0.5, self.speed_ratio);
        let rate = 1.0 / self.yaw_update_rate_inv;
        cur.wrapping_add((ang_delta as f32 * velocity * vel_factor * rate) as i32 as i16)
    }

    /// `func_80045508`: what lies between `at` and `eyeNext`.
    fn col_between(&mut self, col: &CollisionContext, diff: &VecSph, check_eye: bool) -> i32 {
        // The C passes the static CamColChks, so fields a check doesn't write carry over.
        let mut eye_chk = self.at_eye_col_chk;
        eye_chk.pos = self.eye_next;
        let mut at_chk = self.eye_at_col_chk;
        let mut ret = 0;
        let at_eye_bg = Self::bg_check_info(col, self.at, &mut eye_chk);
        if at_eye_bg != 0 {
            at_chk.pos = self.at;
            eye_chk.sph_norm = vec3_to_sph_geo(eye_chk.norm);
            if eye_chk.sph_norm.pitch >= 0x2EE1 {
                eye_chk.sph_norm.yaw = diff.yaw;
            }
            let eye_at_bg = Self::bg_check_info(col, self.eye_next, &mut at_chk);
            let early = if eye_at_bg == 0 {
                if check_eye {
                    at_chk.pos = self.at;
                    let eye = self.eye;
                    if Self::bg_check_info(col, eye, &mut at_chk) == 0 || eye_chk.poly == at_chk.poly { Some(3) } else { None }
                } else {
                    Some(3)
                }
            } else if eye_chk.poly == at_chk.poly {
                Some(3)
            } else {
                None
            };
            if let Some(r) = early {
                self.at_eye_col_chk = eye_chk;
                self.eye_at_col_chk = at_chk;
                return r;
            }
            at_chk.sph_norm = vec3_to_sph_geo(at_chk.norm);
            if at_chk.sph_norm.pitch >= 0x2EE1 {
                at_chk.sph_norm.yaw = diff.yaw.wrapping_sub(0x7FFF);
            }
            ret = if at_eye_bg != eye_at_bg {
                3
            } else {
                let c = math3d_cos(eye_chk.norm, at_chk.norm);
                if c < -0.5 {
                    6
                } else if c > 0.5 {
                    3
                } else {
                    2
                }
            };
        }
        self.at_eye_col_chk = eye_chk;
        self.eye_at_col_chk = at_chk;
        ret
    }

    /// `func_80046E20`: slide the eye round walls between it and `at`.
    fn swing(&mut self, d: &CameraData, col: &CollisionContext, adj: &VecSph, min_dist: f32, arg3: f32, arg4: &mut f32) {
        let mut anim = self.rw.swing;
        let kind = self.col_between(col, adj, anim.unk_18 == 0);
        let mut fallthrough = false;
        match kind {
            1 | 2 => {
                let (ae, ea) = (self.at_eye_col_chk, self.eye_at_col_chk);
                anim.collision_close_point = Self::bg_check_corner(col, self.at, self.eye_next, &ae, &ea);
                let peek = anim.collision_close_point + (ae.norm + ea.norm);
                let t = (self.at - ae.pos).length();
                *arg4 = if t > min_dist { 1.0 } else { t / min_dist };
                anim.swing_update_rate = d.oreg_s(10);
                anim.unk_18 = 1;
                anim.at_eye_poly = ea.poly;
                let mut ne = diff_to_sph_geo(self.at, peek);
                ne.r = adj.r;
                self.eye = sph_geo_add(self.at, ne);
                self.new_eye_col_chk.pos = self.eye;
                let mut nc = self.new_eye_col_chk;
                let hit = Self::bg_check_info(col, self.at, &mut nc);
                self.new_eye_col_chk = nc;
                if hit == 0 {
                    ne.yaw = ne.yaw.wrapping_add(adj.yaw.wrapping_sub(ne.yaw) >> 1);
                    ne.pitch = ne.pitch.wrapping_add(adj.pitch.wrapping_sub(ne.pitch) >> 1);
                    self.eye = sph_geo_add(self.at, ne);
                    if ae.sph_norm.pitch < 0x2AA8 {
                        anim.unk_16 = ne.yaw;
                        anim.unk_14 = ne.pitch;
                    } else {
                        anim.unk_16 = adj.yaw;
                        anim.unk_14 = adj.pitch;
                    }
                    let peek = anim.collision_close_point - (ae.norm + ea.norm);
                    let mut ne = diff_to_sph_geo(self.at, peek);
                    ne.r = adj.r;
                    self.eye_next = sph_geo_add(self.at, ne);
                } else {
                    self.eye = self.new_eye_col_chk.pos;
                    self.at_eye_col_chk = self.new_eye_col_chk;
                    fallthrough = true;
                }
            }
            3 | 6 => fallthrough = true,
            _ => {
                if anim.unk_18 != 0 {
                    anim.swing_update_rate_timer = d.oreg(52);
                    self.eye_next = self.eye;
                    anim.unk_18 = 0;
                }
                anim.swing_update_rate = arg3;
                anim.at_eye_poly = None;
                self.eye = self.at_eye_col_chk.pos + self.at_eye_col_chk.norm;
            }
        }
        if fallthrough {
            if anim.unk_18 != 0 {
                anim.swing_update_rate_timer = d.oreg(52);
                anim.unk_18 = 0;
                self.eye_next = self.eye;
            }
            let ae = self.at_eye_col_chk;
            let t = (self.at - ae.pos).length();
            *arg4 = if t > min_dist { 1.0 } else { t / min_dist };
            anim.swing_update_rate = *arg4 * arg3;
            self.eye = ae.pos + ae.norm * 1.0;
            anim.at_eye_poly = None;
            if t < d.oreg(21) as f32 {
                let s = VecSph {
                    yaw: adj.yaw,
                    pitch: (sin_s(ae.sph_norm.pitch.wrapping_add(0x3FFF)) * 16380.0) as i16,
                    r: (d.oreg(21) as f32 - t) * d.oreg_s(22),
                };
                self.eye = sph_geo_add(self.eye, s);
            }
        }
        self.rw.swing = anim;
    }

    /// `Camera_Normal1`, reading `data_mode`'s values.
    fn normal1(&mut self, d: &CameraData, col: &CollisionContext, p: &PlayerView, data_mode: i16, frames: u32) {
        let rate = 0.1f32;
        let player_height = p.height();
        // RELOAD_PARAMS: animState 0, 10 or 20.
        if matches!(self.anim_state, 0 | 10 | 20) {
            let v: [f32; 10] = std::array::from_fn(|i| d.value(data_mode, i) as f32);
            let y_normal = 1.0 + d.oreg_s(R_CAM_YOFFSET_NORM) - d.oreg_s(R_CAM_YOFFSET_NORM) * (68.0 / player_height);
            let sp94 = y_normal * (player_height * 0.01);
            self.ro = Norm1Ro {
                y_offset: v[0] * sp94,
                dist_min: v[1] * sp94,
                dist_max: v[2] * sp94,
                pitch_target: cam_deg_to_binang(v[3]),
                unk_0c: v[4],
                unk_10: v[5],
                unk_14: v[6] * 0.01,
                fov_target: v[7],
                at_lerp_scale_max: v[8] * 0.01,
                interface_flags: d.value(data_mode, 9),
            };
        }
        let ro = self.ro;
        self.interface_flags = ro.interface_flags;
        let at_eye_geo = diff_to_sph_geo(self.at, self.eye);
        let at_eye_next_geo = diff_to_sph_geo(self.at, self.eye_next);

        if matches!(self.anim_state, 20 | 0 | 10 | 25) {
            if self.anim_state == 20 {
                self.yaw_update_rate_inv = d.oreg(27) as f32;
                self.pitch_update_rate_inv = d.oreg(27) as f32;
            }
            let rw = &mut self.rw;
            rw.swing.at_eye_poly = None;
            rw.slope_pitch_adj = 0;
            rw.unk_28 = 0xA;
            rw.swing.unk_16 = 0;
            rw.swing.unk_14 = 0;
            rw.swing.unk_18 = 0;
            rw.swing.swing_update_rate = ro.unk_0c;
            rw.y_offset = self.player_pos.y;
            rw.unk_20 = self.xz_speed;
            rw.swing.swing_update_rate_timer = 0;
            rw.swing_yaw_target = at_eye_geo.yaw;
            rw.start_swing_timer = d.oreg(50) + d.oreg(51);
        }
        self.anim_state = 1;
        self.update_direction = true;

        if self.rw.unk_28 != 0 {
            self.rw.unk_28 -= 1;
        }
        if self.xz_speed > 0.001 {
            self.rw.start_swing_timer = d.oreg(50) + d.oreg(51);
        } else if self.rw.start_swing_timer > 0 {
            if self.rw.start_swing_timer > d.oreg(50) {
                let a = self.player_rot_y.wrapping_sub(0x7FFF).wrapping_sub(at_eye_geo.yaw);
                self.rw.swing_yaw_target = at_eye_geo.yaw.wrapping_add(a / self.rw.start_swing_timer);
            }
            self.rw.start_swing_timer -= 1;
        }

        let spa0 = self.speed_ratio * d.oreg_s(25);
        let mut sp9c = self.speed_ratio * d.oreg_s(26);
        let mut sp98 = if self.rw.swing.unk_18 != 0 { d.oreg_s(25) } else { spa0 };
        let mut sp94 = (self.xz_speed - self.rw.unk_20) * 0.333333;
        if sp94 > 1.0 {
            sp94 = 1.0;
        }
        // As shipped: `if (sp94 > -1.0f) sp94 = -1.0f;`, so sp94 is at most -1.
        if sp94 > -1.0 {
            sp94 = -1.0;
        }
        self.rw.unk_20 = self.xz_speed;

        let pitch_default = d.oreg(R_CAM_DEFAULT_PITCH_UPDATE_RATE_INV) as f32;
        if self.rw.swing.swing_update_rate_timer != 0 {
            let t = self.rw.swing.swing_update_rate_timer as f32 * 2.0;
            self.yaw_update_rate_inv = lerp_ceil_f(self.rw.swing.swing_update_rate + t, self.yaw_update_rate_inv, sp98, rate);
            self.pitch_update_rate_inv = lerp_ceil_f(pitch_default + t, self.pitch_update_rate_inv, sp9c, rate);
            self.rw.swing.swing_update_rate_timer -= 1;
        } else {
            let s = self.rw.swing.swing_update_rate;
            self.yaw_update_rate_inv = lerp_ceil_f(s - (d.oreg(49) as f32 * 0.01) * s * sp94, self.yaw_update_rate_inv, sp98, rate);
            self.pitch_update_rate_inv = lerp_ceil_f(pitch_default, self.pitch_update_rate_inv, sp9c, rate);
        }
        self.pitch_update_rate_inv = lerp_ceil_f(pitch_default, self.pitch_update_rate_inv, sp9c, rate);
        self.xz_offset_update_rate = lerp_ceil_f(d.oreg_s(2), self.xz_offset_update_rate, spa0, rate);
        self.y_offset_update_rate = lerp_ceil_f(d.oreg_s(3), self.y_offset_update_rate, sp9c, rate);
        // As shipped: the fov rate lerps from yOffsetUpdateRate.
        self.fov_update_rate = lerp_ceil_f(d.oreg_s(4), self.y_offset_update_rate, self.speed_ratio * 0.05, rate);

        if ro.interface_flags & 1 != 0 {
            let t = self.pitch_adj_from_floor(d, col, p, at_eye_geo.yaw.wrapping_sub(0x7FFF), false, frames);
            sp9c = ((1.0 / ro.unk_10) * 0.5) * (1.0 - self.speed_ratio);
            self.rw.slope_pitch_adj = lerp_ceil_s(t, self.rw.slope_pitch_adj, ((1.0 / ro.unk_10) * 0.5) + sp9c, 0xF);
        } else {
            self.rw.slope_pitch_adj = 0;
            if self.player_ground_y == self.player_pos.y {
                self.rw.y_offset = self.player_pos.y;
            }
        }

        let y_off = if self.rw.swing.unk_18 != 0 && ro.y_offset > -40.0 {
            let s = sin_s(self.rw.swing.unk_14);
            -40.0 * s + ro.y_offset * (1.0 - s)
        } else {
            ro.y_offset
        };
        // Flags 0x80 / 0x20 pick func_800458D4 / func_80045B08; NORMAL0 has 0x3, so the default.
        self.calc_at_default(d, &at_eye_next_geo, y_off, ro.interface_flags & 1 != 0, p);

        let mut adj = diff_to_sph_geo(self.at, self.eye_next);
        self.dist = self.clamp_dist(d, adj.r, ro.dist_min, ro.dist_max, self.rw.unk_28);
        adj.r = self.dist;

        if self.rw.start_swing_timer <= 0 {
            adj.pitch = at_eye_next_geo.pitch;
            adj.yaw = lerp_ceil_s(self.rw.swing_yaw_target, at_eye_next_geo.yaw, 1.0 / self.yaw_update_rate_inv, 0xA);
        } else if self.rw.swing.unk_18 != 0 {
            adj.yaw = lerp_ceil_s(self.rw.swing.unk_16, at_eye_next_geo.yaw, 1.0 / self.yaw_update_rate_inv, 0xA);
            adj.pitch = lerp_ceil_s(self.rw.swing.unk_14, at_eye_next_geo.pitch, 1.0 / self.yaw_update_rate_inv, 0xA);
        } else {
            // Rotate yaw to follow Player.
            adj.yaw = self.calc_default_yaw(d, at_eye_next_geo.yaw, self.player_rot_y, ro.unk_14, sp94);
            adj.pitch = self.calc_default_pitch(d, at_eye_next_geo.pitch, ro.pitch_target, self.rw.slope_pitch_adj);
        }
        // 79.65 to -85 degrees.
        adj.pitch = adj.pitch.clamp(-0x3C8C, 0x38A4);

        self.eye_next = sph_geo_add(self.at, adj);
        // CAM_STAT_ACTIVE and not interfaceFlags & 0x10.
        if ro.interface_flags & 0x10 == 0 {
            self.rw.swing_yaw_target = self.player_rot_y.wrapping_sub(0x7FFF);
            if self.rw.start_swing_timer > 0 {
                let (min, u0c) = (ro.dist_min, ro.unk_0c);
                self.swing(d, col, &adj, min, u0c, &mut sp98);
            } else {
                let mut sp88 = self.eye_next;
                self.rw.swing.swing_update_rate = ro.unk_0c * 2.0;
                self.yaw_update_rate_inv = ro.unk_0c * 2.0;
                if Self::bg_check(col, self.at, &mut sp88) {
                    self.rw.swing_yaw_target = at_eye_next_geo.yaw;
                    self.rw.start_swing_timer = -1;
                } else {
                    self.eye = self.eye_next;
                }
                self.rw.swing.unk_18 = 0;
            }
            if self.rw.swing.unk_18 != 0 {
                let target = self.input_dir[1].wrapping_add(self.rw.swing.unk_16.wrapping_sub(0x7FFF).wrapping_sub(self.input_dir[1]));
                self.input_dir[1] = lerp_ceil_s(target, self.input_dir[1], 1.0 - 0.99 * sp98, 0xA);
            }
            if ro.interface_flags & 4 != 0 {
                self.input_dir = [at_eye_geo.pitch.wrapping_neg(), at_eye_geo.yaw.wrapping_sub(0x7FFF), 0];
            } else {
                let e = diff_to_sph_geo(self.eye, self.at);
                self.input_dir = [e.pitch, e.yaw, 0];
            }
        } else {
            self.rw.swing.swing_update_rate = ro.unk_0c;
            self.rw.swing.unk_18 = 0;
            self.update_direction = false;
            self.eye = self.eye_next;
        }

        // Full health: no 0.8 fov squeeze.
        self.fov = lerp_ceil_f(ro.fov_target, self.fov, self.fov_update_rate, 1.0);
        self.roll = lerp_ceil_s(0, self.roll, 0.5, 0xA);
        self.at_lerp_step_scale = self.clamp_lerp_scale(d, ro.at_lerp_scale_max);
    }

    /// `Camera_ClampLERPScale`.
    fn clamp_lerp_scale(&self, d: &CameraData, max: f32) -> f32 {
        let min = d.oreg_s(R_CAM_AT_LERP_STEP_SCALE_MIN);
        if self.at_lerp_step_scale < min {
            min
        } else if self.at_lerp_step_scale >= max {
            max
        } else {
            d.oreg_s(R_CAM_AT_LERP_STEP_SCALE_FAC) * self.at_lerp_step_scale
        }
    }

    // ---- Camera_Parallel1 and Camera_KeepOn1 ---------------------------------------------

    /// The at-calculations' ground test: `playerGroundY == pos.y`, `gravity > -0.1`, or
    /// climbing (`PLAYER_STATE1_21`).
    fn player_grounded(&self, p: &PlayerView) -> bool {
        self.player_ground_y == self.player_pos.y || p.gravity > -0.1 || p.climbing
    }

    /// The off-ground branch shared by `Camera_CalcAtForParallel` and `Camera_CalcAtForLockOn`
    /// with `PREG(75)` 0 (or without `FLG_OFFGROUND`): keep Player within `fov * 0.4` of the view
    /// axis, moving `y_target` by what's left over. Returns the height to take off the offset.
    fn off_ground_fov_adj(&self, y_target: &mut f32) -> f32 {
        let mut dy = self.player_pos.y - *y_target;
        let dist = dist_xz(self.at, self.eye);
        let a = deg_to_rad(self.fov * 0.4);
        let t = (a.sin() / a.cos()) * dist;
        if t < dy {
            *y_target += dy - t;
            dy = t;
        } else if dy < -t {
            *y_target += dy + t;
            dy = -t;
        }
        dy
    }

    /// The other off-ground branch (`func_800458D4`, `PREG(75)`, `FLG_OFFGROUND`): the height
    /// difference scaled down outside `OREG(32)`..`OREG(33)` degrees of pitch.
    fn off_ground_pitch_adj(&self, d: &CameraData, y_target: f32) -> f32 {
        let dy = self.player_pos.y - y_target;
        let angle = f_atan2f(dy, dist_xz(self.at, self.eye));
        let (hi, lo) = (deg_to_rad(d.oreg(32) as f32), deg_to_rad(d.oreg(33) as f32));
        let f = if angle > hi {
            1.0 - (angle - hi).sin()
        } else if angle < lo {
            1.0 - (lo - angle).sin()
        } else {
            1.0
        };
        dy * f
    }

    /// `Camera_CalcAtForParallel`.
    fn calc_at_for_parallel(&mut self, d: &CameraData, p: &PlayerView, y_offset: f32, y_target: &mut f32) {
        let mut target = Vec3::new(0.0, p.height() + y_offset, 0.0);
        if self.player_grounded(p) {
            *y_target = lerp_ceil_f(self.player_pos.y, *y_target, d.oreg_s(43), 0.1);
            target.y -= self.player_pos.y - *y_target;
            let mut off = self.pos_offset;
            lerp_ceil_vec3(target, &mut off, self.y_offset_update_rate, self.xz_offset_update_rate, 0.1);
            self.pos_offset = off;
        } else {
            target.y -= self.off_ground_fov_adj(y_target);
            let mut off = self.pos_offset;
            lerp_ceil_vec3(target, &mut off, d.oreg_s(29), d.oreg_s(30), 0.1);
            self.pos_offset = off;
            self.y_offset_update_rate = d.oreg_s(29);
            self.xz_offset_update_rate = d.oreg_s(30);
        }
        let at_target = self.player_pos + self.pos_offset;
        let mut at = self.at;
        lerp_ceil_vec3(at_target, &mut at, self.at_lerp_step_scale, self.at_lerp_step_scale, 0.2);
        self.at = at;
    }

    /// `func_800458D4`: the at for Parallel1 in the air.
    fn func_800458d4(&mut self, d: &CameraData, p: &PlayerView, eye_at_dir: &VecSph, y_offset: f32, y_target: f32, slope: bool) {
        let mut target = Vec3::new(0.0, p.height() + y_offset, 0.0);
        if slope {
            target.y -= calc_slope_y_adj(self.floor_norm, self.player_rot_y, eye_at_dir.yaw, d.oreg(9) as f32);
        }
        target.y -= self.off_ground_pitch_adj(d, y_target);
        let mut off = self.pos_offset;
        lerp_ceil_vec3(target, &mut off, d.oreg_s(29), d.oreg_s(30), 0.1);
        self.pos_offset = off;
        let at_target = self.player_pos + self.pos_offset;
        let mut at = self.at;
        lerp_ceil_vec3(at_target, &mut at, self.at_lerp_step_scale, self.at_lerp_step_scale, 0.2);
        self.at = at;
    }

    /// `Camera_Parallel1`: the camera swings behind Player over `R_CAM_DEFAULT_ANIM_TIME`
    /// frames (`animTimer`, refusing mode changes meanwhile), then holds `distTarget` and
    /// `pitchTarget` there. Only then does it set `sCameraInterfaceFlags` (the letterbox).
    fn parallel1(&mut self, d: &CameraData, col: &CollisionContext, p: &PlayerView, frames: u32) {
        let player_height = p.height();
        if matches!(self.anim_state, 0 | 10 | 20) {
            let v = |i: usize| d.value(self.mode, i) as f32;
            let y_normal = 1.0 + d.oreg_s(R_CAM_YOFFSET_NORM) - d.oreg_s(R_CAM_YOFFSET_NORM) * (68.0 / player_height);
            self.para1_ro = Para1Ro {
                y_offset: v(0) * 0.01 * player_height * y_normal,
                dist_target: v(1) * 0.01 * player_height * y_normal,
                pitch_target: cam_deg_to_binang(v(2)),
                yaw_target: cam_deg_to_binang(v(3)),
                unk_08: v(4),
                unk_0c: v(5),
                fov_target: v(6),
                unk_14: v(7) * 0.01,
                interface_flags: d.value(self.mode, 8),
                unk_18: v(9) * 0.01 * player_height * y_normal,
                unk_1c: v(10) * 0.01,
            };
        }
        let ro = self.para1_ro;
        let flags = ro.interface_flags;
        let at_to_eye = diff_to_sph_geo(self.at, self.eye);
        let at_to_eye_next = diff_to_sph_geo(self.at, self.eye_next);

        if matches!(self.anim_state, 0 | 10 | 20 | 25) {
            let rw = &mut self.para1_rw;
            rw.unk_16 = 0;
            rw.unk_10 = 0;
            rw.anim_timer = if flags & 4 != 0 { 20 } else { d.oreg(R_CAM_DEFAULT_ANIM_TIME) };
            rw.unk_00_x = 0.0;
            rw.y_target = self.player_pos.y - self.player_pos_delta.y;
            self.anim_state += 1;
        }
        let behind = self.player_rot_y.wrapping_sub(0x7FFF);
        if self.para1_rw.anim_timer != 0 {
            self.para1_rw.yaw_target = if flags & 2 != 0 {
                // roData->yawTarget degrees from behind Player.
                behind.wrapping_add(ro.yaw_target)
            } else if flags & 4 != 0 {
                ro.yaw_target
            } else {
                at_to_eye_next.yaw
            };
        } else {
            if flags & 0x20 != 0 {
                self.para1_rw.yaw_target = behind.wrapping_add(ro.yaw_target);
            }
            self.interface_flags = flags;
        }
        self.para1_rw.pitch_target = ro.pitch_target;
        if self.anim_state == 21 {
            self.para1_rw.unk_16 = 1;
            self.anim_state = 1;
        } else if self.anim_state == 11 {
            self.anim_state = 1;
        }

        let spb8 = d.oreg_s(25) * self.speed_ratio;
        let spb4 = d.oreg_s(26) * self.speed_ratio;
        self.r_update_rate_inv = lerp_ceil_f(d.oreg(6) as f32, self.r_update_rate_inv, spb8, 0.1);
        self.yaw_update_rate_inv = lerp_ceil_f(ro.unk_08, self.yaw_update_rate_inv, spb8, 0.1);
        self.pitch_update_rate_inv = lerp_ceil_f(2.0, self.pitch_update_rate_inv, spb4, 0.1);
        self.xz_offset_update_rate = lerp_ceil_f(d.oreg_s(2), self.xz_offset_update_rate, spb8, 0.1);
        self.y_offset_update_rate = lerp_ceil_f(d.oreg_s(3), self.y_offset_update_rate, spb4, 0.1);
        self.fov_update_rate = lerp_ceil_f(d.oreg_s(4), self.fov_update_rate, self.speed_ratio * 0.05, 0.1);

        if flags & 1 != 0 {
            let t = self.pitch_adj_from_floor(d, col, p, at_to_eye.yaw.wrapping_sub(0x7FFF), true, frames);
            let a = (1.0 / ro.unk_0c) * 0.3;
            let b = ((1.0 / ro.unk_0c) * 0.7) * (1.0 - self.speed_ratio);
            self.para1_rw.unk_10 = lerp_ceil_s(t, self.para1_rw.unk_10, a + b, 0xF);
        } else {
            self.para1_rw.unk_10 = 0;
        }

        let off_ground = !self.player_grounded(p);
        if !off_ground {
            self.para1_rw.y_target = self.player_pos.y;
        }
        let mut y_target = self.para1_rw.y_target;
        if flags & 0x80 == 0 && !off_ground {
            self.calc_at_for_parallel(d, p, ro.y_offset, &mut y_target);
        } else {
            self.func_800458d4(d, p, &at_to_eye_next, ro.unk_18, y_target, flags & 1 != 0);
        }
        self.para1_rw.y_target = y_target;

        let mut spa8;
        let rw = &mut self.para1_rw;
        if rw.anim_timer != 0 {
            self.unk_14c |= 0x20;
            let tangle = (((rw.anim_timer as i32 + 1) * rw.anim_timer as i32) >> 1) as i16;
            let step = (rw.yaw_target.wrapping_sub(at_to_eye.yaw) / tangle) as i32 * rw.anim_timer as i32;
            spa8 = VecSph { yaw: (at_to_eye.yaw as i32 + step) as i16, pitch: at_to_eye.pitch, r: at_to_eye.r };
            rw.anim_timer -= 1;
        } else {
            rw.unk_16 = 0;
            self.dist = lerp_ceil_f(ro.dist_target, self.dist, 1.0 / self.r_update_rate_inv, 2.0);
            spa8 = diff_to_sph_geo(self.at, self.eye_next);
            spa8.r = self.dist;
            spa8.yaw = lerp_ceil_s(rw.yaw_target, at_to_eye_next.yaw, if flags & 0x40 != 0 { 0.6 } else { 0.8 }, 0xA);
            let pitch = if flags & 1 != 0 { rw.pitch_target.wrapping_sub(rw.unk_10) } else { rw.pitch_target };
            spa8.pitch = lerp_ceil_s(pitch, at_to_eye_next.pitch, 1.0 / self.pitch_update_rate_inv, 4);
            if spa8.pitch > d.oreg(R_CAM_MAX_PITCH) {
                spa8.pitch = d.oreg(R_CAM_MAX_PITCH);
            }
            if spa8.pitch < d.oreg(R_CAM_MIN_PITCH_1) {
                spa8.pitch = d.oreg(R_CAM_MIN_PITCH_1);
            }
        }
        self.eye_next = sph_geo_add(self.at, spa8);
        // CAM_STAT_ACTIVE with the skybox shown: Camera_BGCheckInfo.
        let mut c = ColChk { pos: self.eye_next, ..Default::default() };
        Self::bg_check_info(col, self.at, &mut c);
        self.eye = c.pos;
        self.fov = lerp_ceil_f(ro.fov_target, self.fov, self.fov_update_rate, 1.0);
        self.roll = lerp_ceil_s(0, self.roll, 0.5, 0xA);
        self.at_lerp_step_scale = self.clamp_lerp_scale(d, if off_ground { ro.unk_1c } else { ro.unk_14 });
        // @bug (game): Camera_Parallel1 doesn't return a value; Camera_Update ignores it.
    }

    /// `Camera_CalcAtForLockOn`: `at` between Player's head and the target. Returns
    /// `outPlayerToTargetDir`.
    #[allow(clippy::too_many_arguments)]
    fn calc_at_for_lock_on(&mut self, d: &CameraData, p: &PlayerView, target_pos: Vec3, y_offset: f32, distance: f32, y_pos_offset: &mut f32, flags: i16) -> VecSph {
        let h = p.height();
        let mut tmp0 = Vec3::new(0.0, h + y_offset, 0.0);
        // Player's head.
        let head = self.player_pos + Vec3::Y * h;
        let out = diff_to_sph_geo(head, target_pos);
        let mut dir = out;
        if distance < dir.r {
            dir.r *= d.oreg_s(38);
        } else {
            // Player's height off the ground, over his height.
            let t = clamp_max_dist((self.player_pos.y - self.player_ground_y) / h, 1.0);
            dir.r = (dir.r * d.oreg_s(39)) - (((d.oreg_s(39) - d.oreg_s(38)) * dir.r) * (dir.r / distance));
            dir.r -= (dir.r * t) * t;
        }
        if flags & 0x80 != 0 {
            dir.r *= 0.2;
            self.xz_offset_update_rate = 0.01;
            self.y_offset_update_rate = 0.01;
        }
        tmp0 += sph_geo_to_vec3(dir);
        let mut off = self.pos_offset;
        if self.player_grounded(p) {
            *y_pos_offset = lerp_ceil_f(self.player_pos.y, *y_pos_offset, d.oreg_s(43), 0.1);
            tmp0.y -= self.player_pos.y - *y_pos_offset;
            lerp_ceil_vec3(tmp0, &mut off, self.y_offset_update_rate, self.xz_offset_update_rate, 0.1);
        } else {
            tmp0.y -= if flags & 0x80 == 0 { self.off_ground_fov_adj(y_pos_offset) } else { self.off_ground_pitch_adj(d, *y_pos_offset) };
            lerp_ceil_vec3(tmp0, &mut off, d.oreg_s(29), d.oreg_s(30), 0.1);
            self.y_offset_update_rate = d.oreg_s(29);
            self.xz_offset_update_rate = d.oreg_s(30);
        }
        self.pos_offset = off;
        let at_target = self.player_pos + self.pos_offset;
        let mut at = self.at;
        lerp_ceil_vec3(at_target, &mut at, self.at_lerp_step_scale, self.at_lerp_step_scale, 0.2);
        self.at = at;
        out
    }

    /// `Camera_KeepOn1`: locked on to a non-enemy, the camera frames Player and the target
    /// (`camera->target`, whose focus is `target_focus`), swinging round over
    /// `R_CAM_DEFAULT_ANIM_TIME` frames after `OREG(24)` still ones.
    fn keep_on1(&mut self, d: &CameraData, col: &CollisionContext, p: &PlayerView, target_focus: Option<Vec3>) {
        let mut sp88 = false;
        let mut player_height = p.height();
        let Some(focus) = target_focus.filter(|_| self.target.is_some()) else {
            // "keepon: target is not valid, change parallel".
            self.target = None;
            self.change_mode(d, CAM_MODE_TARGET);
            return;
        };
        let reload = matches!(self.anim_state, 0 | 10 | 20);
        if reload {
            let v = |i: usize| d.value(self.mode, i) as f32;
            let y_normal = 1.0 + d.oreg_s(R_CAM_YOFFSET_NORM) - d.oreg_s(R_CAM_YOFFSET_NORM) * (68.0 / player_height);
            self.keep1_ro = Keep1Ro {
                unk_00: v(0) * 0.01 * player_height * y_normal,
                unk_04: v(1),
                unk_08: v(2),
                unk_0c: v(3),
                unk_10: v(4),
                unk_14: v(5),
                unk_18: v(6),
                unk_1c: v(7) * 0.01,
                unk_20: v(8),
                unk_24: v(9) * 0.01,
                interface_flags: d.value(self.mode, 10),
                unk_28: v(11) * 0.01 * player_height * y_normal,
                unk_2c: v(12) * 0.01,
            };
        }
        let ro = self.keep1_ro;
        player_height += ro.unk_00;
        let spc0 = diff_to_sph_geo(self.at, self.eye);
        let spb8 = diff_to_sph_geo(self.at, self.eye_next);
        self.interface_flags = ro.interface_flags;
        if reload {
            self.anim_state += 1;
            let rw = &mut self.keep1_rw;
            rw.unk_10 = 0;
            rw.unk_04 = 0.0;
            rw.unk_0c = self.target;
            rw.unk_16 = d.oreg(R_CAM_DEFAULT_ANIM_TIME) + d.oreg(24);
            rw.unk_12 = spc0.yaw;
            rw.unk_14 = spc0.pitch;
            rw.unk_00 = spc0.r;
            rw.unk_08 = self.player_pos.y - self.player_pos_delta.y;
        }
        // CAM_STAT_ACTIVE.
        self.update_direction = true;
        self.input_dir = [spc0.pitch.wrapping_neg(), spc0.yaw.wrapping_sub(0x7FFF), 0];

        let mut sp104 = ro.unk_04;
        let mut sp84 = 1.0f32;
        // Uninitialised in the C on the default path, which Player never takes (it sets the
        // target, and paramFlags 8, whenever it asks for KEEPON).
        let mut sp80 = false;
        let mut spc8 = VecSph::default();
        match self.param_flags & 0x18 {
            flags @ (8 | 0x10) => {
                if flags == 8 {
                    // Player's interactRangeActor being the target (60 in front of Player's
                    // focus) isn't modelled; the C then overwrites it with the target's focus.
                    self.target_pos = focus;
                    if self.keep1_rw.unk_0c != self.target {
                        self.keep1_rw.unk_0c = self.target;
                        self.at_lerp_step_scale = 0.0;
                    }
                    self.xz_offset_update_rate = lerp_ceil_f(1.0, self.xz_offset_update_rate, d.oreg_s(25) * self.speed_ratio, 0.1);
                    self.y_offset_update_rate = lerp_ceil_f(1.0, self.y_offset_update_rate, d.oreg_s(26) * self.speed_ratio, 0.1);
                    self.fov_update_rate = lerp_ceil_f(d.oreg_s(4), self.fov_update_rate, self.speed_ratio * 0.05, 0.1);
                } else {
                    self.keep1_rw.unk_0c = None;
                }
                if self.player_grounded(p) {
                    self.keep1_rw.unk_08 = self.player_pos.y;
                    sp80 = false;
                } else {
                    sp80 = true;
                }
                let mut y = self.keep1_rw.unk_08;
                let (y_off, fl) = if sp80 { (ro.unk_28, 0x80 | ro.interface_flags) } else { (ro.unk_00, ro.interface_flags) };
                self.calc_at_for_lock_on(d, p, self.target_pos, y_off, sp104, &mut y, fl);
                self.keep1_rw.unk_08 = y;
                let head = self.player_pos + Vec3::Y * player_height;
                spc8 = diff_to_sph_geo(head, self.target_pos);
                sp84 = if spc8.r > sp104 { 1.0 } else { spc8.r / sp104 };
            }
            _ => {
                self.at = self.player_pos + Vec3::Y * player_height;
                self.keep1_rw.unk_0c = None;
            }
        }
        let mut spd8 = diff_to_sph_geo(self.at, self.eye_next);
        let mut spe8;
        if spd8.r < ro.unk_04 {
            sp104 = ro.unk_04;
            spe8 = d.oreg(6) as f32;
        } else if ro.unk_08 < spd8.r {
            sp104 = ro.unk_08;
            spe8 = d.oreg(6) as f32;
        } else {
            sp104 = spd8.r;
            spe8 = 1.0;
        }
        self.r_update_rate_inv = lerp_ceil_f(spe8, self.r_update_rate_inv, d.oreg_s(25), 0.1);
        self.dist = lerp_ceil_f(sp104, self.dist, 1.0 / self.r_update_rate_inv, 0.2);
        spe8 = self.dist;
        let mut spd0 = diff_to_sph_geo(self.at, self.target_pos);
        spd0.r = spe8 - ((if spd0.r <= spe8 { spd0.r } else { spe8 }) * 0.5);
        let spec = ro.unk_0c + ((ro.unk_10 - ro.unk_0c) * (1.1 - sp84));
        let spf0 = d.oreg(13) as f32 + spec;
        self.dist = lerp_ceil_f(spe8, self.dist, d.oreg_s(11), 2.0);
        spd8.r = self.dist;
        spd8.yaw = spb8.yaw;
        let spe2 = spd0.yaw.wrapping_sub(spb8.yaw.wrapping_sub(0x7FFF));
        let rw = &mut self.keep1_rw;
        if rw.unk_16 != 0 {
            if rw.unk_16 >= d.oreg(24) {
                let sp82 = rw.unk_16 - d.oreg(24);
                let yaw = spc8.yaw;
                spc8 = diff_to_sph_geo(self.at, self.eye);
                spc8.yaw = yaw.wrapping_sub(0x7FFF);
                let t2 = 1.0 / d.oreg(R_CAM_DEFAULT_ANIM_TIME) as f32;
                let dr = (rw.unk_00 - spc8.r) * t2;
                let dyaw = (rw.unk_12.wrapping_sub(spc8.yaw) as f32 * t2) as i32 as i16;
                let dpitch = (rw.unk_14.wrapping_sub(spc8.pitch) as f32 * t2) as i32 as i16;
                spd8.r = lerp_ceil_f(spc8.r + (dr * sp82 as f32), spc0.r, d.oreg_s(28), 1.0);
                spd8.yaw = lerp_ceil_s((spc8.yaw as i32 + dyaw as i32 * sp82 as i32) as i16, spc0.yaw, d.oreg_s(28), 0xA);
                spd8.pitch = lerp_ceil_s((spc8.pitch as i32 + dpitch as i32 * sp82 as i32) as i16, spc0.pitch, d.oreg_s(28), 0xA);
            } else {
                sp88 = true;
            }
            rw.unk_16 -= 1;
        } else if (spe2 as i32).abs() > cam_deg_to_binang(spec) as i32 {
            let spf4 = cam_binang_to_deg(spe2);
            let t2 = spec + (spf0 - spec) * (clamp_max_dist(spd0.r, spd8.r) / spd8.r);
            let temp_f12_2 = (t2 * t2 - 2.0) / (t2 - 360.0);
            let t1 = (temp_f12_2 * spf4) + (2.0 - (360.0 * temp_f12_2));
            let temp_f14 = spf4 * spf4 / t1;
            let spe0 = if spe2 >= 0 { cam_deg_to_binang(temp_f14) } else { cam_deg_to_binang(temp_f14).wrapping_neg() };
            spd8.yaw = spb8.yaw.wrapping_sub(0x7FFF).wrapping_add(spe0).wrapping_sub(0x7FFF);
        } else {
            let spf4 = (1.0 - self.speed_ratio) * 0.02;
            let spe0 = if spe2 >= 0 { cam_deg_to_binang(spec) } else { cam_deg_to_binang(spec).wrapping_neg() };
            spd8.yaw = spb8.yaw.wrapping_sub(((spe0 as i32 - spe2 as i32) as f32 * spf4) as i32 as i16);
        }

        if !sp88 {
            let mut pitch = cam_deg_to_binang(ro.unk_14 + ((ro.unk_18 - ro.unk_14) * sp84));
            pitch = pitch.wrapping_sub((spc8.pitch as f32 * (0.5 + (sp84 * 0.5))) as i32 as i16);
            pitch = pitch.wrapping_add((spd0.pitch as f32 * ro.unk_1c) as i32 as i16);
            pitch = pitch.clamp(-0x3200, 0x3200);
            spd8.pitch = lerp_ceil_s(pitch, spb8.pitch, d.oreg_s(12), 0xA);
            self.eye_next = sph_geo_add(self.at, spd8);
            // CAM_STAT_ACTIVE with the skybox shown: Camera_BGCheckInfo.
            let mut c = ColChk { pos: self.eye_next, ..Default::default() };
            Self::bg_check_info(col, self.at, &mut c);
            self.eye = c.pos;
            // Camera_Vec3fTranslateByUnitVector(eye, eye, eye→at, OREG(1)).
            self.eye += dist_normalize(self.eye, self.at) * d.oreg(1) as f32;
        }
        self.fov = lerp_ceil_f(ro.unk_20, self.fov, self.fov_update_rate, 1.0);
        self.roll = lerp_ceil_s(0, self.roll, 0.5, 0xA);
        self.at_lerp_step_scale = self.clamp_lerp_scale(d, if sp80 { ro.unk_2c } else { ro.unk_24 });
    }

    /// Normal1's distance limits for this Player (read-only data after a reload).
    pub fn dist_limits(&self) -> (f32, f32) {
        (self.ro.dist_min, self.ro.dist_max)
    }
}

/// `Camera_CalcSlopeYAdj`.
fn calc_slope_y_adj(floor_norm: Vec3, player_y_rot: i16, eye_at_yaw: i16, adj: f32) -> f32 {
    let s = vec3_to_sph_geo(floor_norm);
    let tmp = cos_s(s.pitch) * cos_s(player_y_rot.wrapping_sub(s.yaw));
    (tmp.abs() * adj) * cos_s(player_y_rot.wrapping_sub(eye_at_yaw))
}

/// `Camera_CalcUpFromPitchYawRoll`.
pub fn calc_up(pitch: i16, yaw: i16, roll: i16) -> Vec3 {
    let (sp, cp, sy, cy) = (sin_s(pitch), cos_s(pitch), sin_s(yaw), cos_s(yaw));
    let (sr, cr) = (sin_s(roll.wrapping_neg()), cos_s(roll.wrapping_neg()));
    let u = Vec3::new(cp * sy, sp, cp * cy);
    let r1 = Vec3::new((1.0 - u.x * u.x) * cr + u.x * u.x, u.x * u.y * (1.0 - cr) - u.z * sr, u.z * u.x * (1.0 - cr) + u.y * sr);
    let r2 = Vec3::new(u.x * u.y * (1.0 - cr) + u.z * sr, (1.0 - u.y * u.y) * cr + u.y * u.y, u.y * u.z * (1.0 - cr) - u.x * sr);
    let r3 = Vec3::new(u.z * u.x * (1.0 - cr) - u.y * sr, u.y * u.z * (1.0 - cr) + u.x * sr, (1.0 - u.z * u.z) * cr + u.z * u.z);
    let base = Vec3::new(-sp * sy, cp, -sp * cy);
    Vec3::new(base.dot(r1), base.dot(r2), base.dot(r3))
}

// ---------------------------------------------------------------------------------------------
// The spikes' follow camera (not a game camera; F3 in the sandbox)
// ---------------------------------------------------------------------------------------------

/// Simple third-person follow camera, rotated with C-left / C-right.
#[derive(Debug, Clone, Copy)]
pub struct FollowCamera {
    /// Point the camera looks at.
    pub at: Vec3,
    /// Orbit yaw: direction from `at` to the eye, radians (0 = eye on +z side).
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    /// Height of the look-at point above Player's feet.
    pub height: f32,
}

impl FollowCamera {
    pub fn behind(pos: Vec3, facing: i16, adult: bool) -> FollowCamera {
        let height = if adult { 45.0 } else { 30.0 };
        // Eye behind Player: opposite the facing direction.
        let yaw = binang_to_rad(facing) + std::f32::consts::PI;
        FollowCamera { at: pos + Vec3::Y * height, yaw, pitch: 0.28, distance: if adult { 260.0 } else { 200.0 }, height }
    }

    pub fn eye(&self) -> Vec3 {
        let dir = Vec3::new(self.pitch.cos() * self.yaw.sin(), self.pitch.sin(), self.pitch.cos() * self.yaw.cos());
        self.at + dir * self.distance
    }

    /// `Camera_GetInputDirYaw`: `inputDir.y = atEyeGeo.yaw - 0x7FFF`, i.e. the yaw the
    /// camera looks along.
    pub fn input_dir_yaw(&self) -> i16 {
        rad_to_binang(self.yaw).wrapping_sub(0x7FFF)
    }

    /// One game frame: C-left/right orbit at 6° per frame, the look-at point follows Player
    /// with lag, and the orbit drifts behind Player while running.
    pub fn update(&mut self, input: &Input, player_pos: Vec3, player_facing: i16, speed: f32) {
        let step = 6f32.to_radians();
        if input.cur.held(BTN_CLEFT) {
            self.yaw += step;
        }
        if input.cur.held(BTN_CRIGHT) {
            self.yaw -= step;
        }
        let target = player_pos + Vec3::Y * self.height;
        self.at += (target - self.at) * 0.35;
        if speed > 1.0 && !input.cur.held(BTN_CLEFT) && !input.cur.held(BTN_CRIGHT) {
            let behind = binang_to_rad(player_facing) + std::f32::consts::PI;
            let mut d = (behind - self.yaw) % std::f32::consts::TAU;
            if d > std::f32::consts::PI {
                d -= std::f32::consts::TAU;
            } else if d < -std::f32::consts::PI {
                d += std::f32::consts::TAU;
            }
            // Only swing when Player runs roughly away from or across the view, not at it.
            if d.abs() < 2.4 {
                self.yaw += d * 0.02 * (speed / 6.0).min(1.0);
            }
        }
        self.yaw %= std::f32::consts::TAU;
    }
}

/// Which camera drives Player and the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraKind {
    /// `z_camera.c` (`Camera_Normal1`).
    Game,
    /// Spike 03's follow camera (C-left / C-right orbit).
    Follow,
}

/// The view a camera produced this frame.
#[derive(Debug, Clone, Copy)]
pub struct CamView {
    pub eye: Vec3,
    pub at: Vec3,
    /// Vertical field of view, degrees.
    pub fov: f32,
}
