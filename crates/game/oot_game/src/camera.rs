//! The game camera from `z_camera.c`: `Camera_Init`, `Camera_InitPlayerSettings`, the
//! player-following part of `Camera_Update` (with the floor's bg camera), the mode and setting
//! changes (`Camera_ChangeModeFlags`, `Camera_ChangeSettingFlags`, `Camera_ChangeBgCamIndex`,
//! `Camera_ChangeDoorCam`, `func_80057FC4`) and these mode functions
//! (docs/adr/0013-camera-modes-and-screen.md, docs/adr/0015-camera-settings-and-bg-cameras.md):
//!
//! - `Camera_Normal1` (NORMAL and STILL), with `Camera_CalcAtDefault`, `Camera_ClampDist`,
//!   `Camera_CalcDefaultYaw`/`Pitch`, `Camera_GetPitchAdjFromFloorHeightDiffs` and the swing
//!   `func_80046E20` / `func_80045508`;
//! - `Camera_Parallel1` (TARGET: Z held with nothing to lock on to, and PUSHPULL), with
//!   `Camera_CalcAtForParallel` and `func_800458D4`;
//! - `Camera_KeepOn1` (FOLLOWTARGET: locked on to a non-enemy), with `Camera_CalcAtForLockOn`;
//! - the scene cameras: `Camera_Fixed2` (`PIVOT_CRAWLSPACE`), `Camera_Fixed3`
//!   (`PREREND_FIXED`), `Camera_Fixed4` (`PIVOT_IN_FRONT`), `Camera_Data4`
//!   (`PIVOT_SHOP_BROWSING`), `Camera_Unique0` (`START1`), `Camera_Unique2`
//!   (`SCENE_TRANSITION`), `Camera_Unique3` (`DOOR0`), `Camera_Unique6` (`FREE0`),
//!   `Camera_Unique7` (`PREREND_PIVOT`) and `Camera_Special9` (`DOORC`);
//! - the camera bgcheck (`Camera_BGCheckInfo`, `Camera_BGCheckCorner`, `Camera_GetFloorYLayer`,
//!   `Camera_CheckOOB`);
//! - `Camera_UpdateInterface`, driving `crate::letterbox` and the interface's alpha type.
//!
//! Vector-sphere maths is `z_olib.c`; `Math_FAtan2F` is the Taylor-series version from
//! `code_800FCE80.c`. `OREG` values (`sOREGInit`) and every setting's modes, functions and
//! data (`sCameraSettings`) come from `z_camera_data.c`, through the asset pack
//! (`CameraData`); the scene's bg cameras from its collision header (`BgCamInfo`).
//! `PREG(75)` and `PREG(76)` are 0 (only the debug register editor sets them), so the
//! at-calculations skip their slope adjustment and take the fov-based off-ground branch.
//!
//! A mode whose function isn't ported (BATTLE's `Camera_Battle1`, TALK's `Camera_KeepOn3` and
//! `Camera_KeepOn0`, JUMP, CLIMB, HANG...) runs its setting's NORMAL function if that one is
//! ported, else `Camera_Normal1` on NORMAL0's NORMAL data; `camera->mode` still changes as in
//! the game. Not modelled: water and hot-room checks, quakes, the low-health wiggle, the debug
//! camera, the mode-change sounds, `func_80043F94` (scenes with the skybox disabled) and the
//! interface alpha.

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
    /// `sCameraSettings`, indexed by `CAM_SET_*` (entry 0, `CAM_SET_NONE`, has no modes).
    pub settings: Vec<CamSettingData>,
}

/// A `CameraSetting` (`sCameraSettings[setting]`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CamSettingData {
    /// `CAM_SET_NORMAL0`.
    pub name: String,
    /// `unk_00`: the valid modes in bits 0..29 (`validModes`), the priority in bits 24..27
    /// (`Camera_ChangeSettingFlags`), 0x40000000 (don't become `prevSetting`) and 0x80000000
    /// (`Camera_ChangeBgCamIndex` keeps the index even when the setting change is refused).
    pub flags: u32,
    /// The `sCamSet*Modes` array, indexed by `CAM_MODE_*`; `{ CAM_FUNC_NONE, 0, NULL }`
    /// entries are `None`, and the array can be shorter than `CAM_MODE_MAX`.
    pub modes: Vec<Option<CamModeData>>,
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
    /// `sCameraSettings[setting]`.
    pub fn setting(&self, setting: i16) -> Option<&CamSettingData> {
        usize::try_from(setting).ok().and_then(|s| self.settings.get(s))
    }
    /// `sCameraSettings[setting].unk_00` (0 for a setting past the table).
    pub fn setting_flags(&self, setting: i16) -> u32 {
        self.setting(setting).map(|s| s.flags).unwrap_or(0)
    }
    /// `sCameraSettings[setting].cameraModes[mode]`.
    pub fn mode(&self, setting: i16, mode: i16) -> Option<&CamModeData> {
        self.setting(setting)?.modes.get(usize::try_from(mode).ok()?)?.as_ref()
    }
    /// The value `i` of a mode's data (`GET_NEXT_RO_DATA` in order).
    fn value(&self, (setting, mode): (i16, i16), i: usize) -> i16 {
        self.mode(setting, mode).and_then(|m| m.values.get(i)).copied().unwrap_or(0)
    }
    /// The `CAM_SET_*` value of a setting name.
    pub fn setting_id(&self, name: &str) -> Option<i16> {
        self.settings.iter().position(|s| s.name == name).map(|i| i as i16)
    }
}

// CAM_SET_* (z64camera.h) the camera and the ported actors name.
pub const CAM_SET_NONE: i16 = 0x00;
pub const CAM_SET_NORMAL0: i16 = 0x01;
pub const CAM_SET_DUNGEON0: i16 = 0x03;
pub const CAM_SET_PIVOT_CRAWLSPACE: i16 = 0x16;
pub const CAM_SET_PIVOT_SHOP_BROWSING: i16 = 0x17;
pub const CAM_SET_PIVOT_IN_FRONT: i16 = 0x18;
pub const CAM_SET_PREREND_FIXED: i16 = 0x19;
pub const CAM_SET_PREREND_PIVOT: i16 = 0x1A;
pub const CAM_SET_DOOR0: i16 = 0x1C;
pub const CAM_SET_DOORC: i16 = 0x1D;
pub const CAM_SET_CRAWLSPACE: i16 = 0x1E;
pub const CAM_SET_FREE0: i16 = 0x21;
/// `CAM_SET_CS_0` ("DEMO0"): a cutscene script's camera (`Camera_Demo1`).
pub const CAM_SET_CS_0: i16 = 0x25;
/// `CAM_SET_SLOW_CHEST_CS` ("ITEM0"): a big chest opening on a major item.
pub const CAM_SET_SLOW_CHEST_CS: i16 = 0x28;
pub const CAM_SET_CS_ATTENTION: i16 = 0x2B;
pub const CAM_SET_SCENE_TRANSITION: i16 = 0x2F;
pub const CAM_SET_MEADOW_BIRDS_EYE: i16 = 0x35;
pub const CAM_SET_MEADOW_UNUSED: i16 = 0x36;
pub const CAM_SET_TURN_AROUND: i16 = 0x38;
pub const CAM_SET_MAX: i16 = 0x42;

/// The mode functions ported (`CAM_FUNC_*`); the others fall back (`GameCamera::update`).
const PORTED: &[&str] = &[
    "CAM_FUNC_NORM1",
    "CAM_FUNC_PARA1",
    "CAM_FUNC_KEEP1",
    "CAM_FUNC_KEEP3",
    "CAM_FUNC_KEEP0",
    "CAM_FUNC_FIXD2",
    "CAM_FUNC_FIXD3",
    "CAM_FUNC_FIXD4",
    "CAM_FUNC_DATA4",
    "CAM_FUNC_UNIQ0",
    "CAM_FUNC_UNIQ2",
    "CAM_FUNC_UNIQ3",
    "CAM_FUNC_UNIQ6",
    "CAM_FUNC_UNIQ7",
    "CAM_FUNC_SPEC9",
    "CAM_FUNC_KEEP4",
    "CAM_FUNC_DEMO3",
    "CAM_FUNC_SUBJ4",
    "CAM_FUNC_DEMO1",
];

// CAM_STAT_* (z64camera.h).
pub const CAM_STAT_CUT: i16 = 0;
pub const CAM_STAT_WAIT: i16 = 1;
pub const CAM_STAT_UNK3: i16 = 3;
pub const CAM_STAT_ACTIVE: i16 = 7;
pub const CAM_STAT_UNK100: i16 = 0x100;

// Camera ids (z64camera.h).
pub const NUM_CAMS: usize = 4;
pub const CAM_ID_MAIN: i16 = 0;
pub const CAM_ID_SUB_FIRST: i16 = 1;
pub const CAM_ID_NONE: i16 = -1;

/// `z_camera.c`'s file-scope state that every camera shares: what `Camera_Init` resets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CameraGlobals {
    /// `sCameraInterfaceFlags`: the letterbox (`0xF000`) and interface alpha (`0x0F00`) bits the
    /// last mode function asked for.
    pub interface_flags: i16,
    /// `sCameraInterfaceAlpha`: the alpha type last asked for.
    pub interface_alpha: u16,
    /// `D_8011D3F0`: after a `Camera_Init`, the main camera's first three active updates hold the
    /// interface at 0x3200 (the letterbox's target 32, alpha type 2).
    pub d_8011d3f0: i16,
    /// `Camera_Update`'s `sOOBTimer`: frames the camera's player has been over no floor.
    pub oob_timer: u32,
    /// `sNextUID`.
    pub next_uid: i16,
}

impl CameraGlobals {
    /// As `Camera_Init` then `Camera_InitPlayerSettings` leave them for the main camera.
    pub fn main_init() -> CameraGlobals {
        let mut g = CameraGlobals { interface_flags: 0, interface_alpha: 0, d_8011d3f0: 0, oob_timer: 0, next_uid: 0 };
        g.camera_init();
        // Camera_InitPlayerSettings, for the main camera.
        g.interface_flags = 0xB200u16 as i16;
        g
    }

    /// `Camera_Init`'s writes to the shared state (`sCameraLetterboxSize` 32 isn't kept: only
    /// `Camera_UpdateInterface` sets it before reading it). Returns the new camera's uid
    /// (`sNextUID`, skipping 0).
    pub fn camera_init(&mut self) -> i16 {
        let mut uid = self.next_uid;
        self.next_uid = self.next_uid.wrapping_add(1);
        if uid == 0 {
            uid = self.next_uid;
            self.next_uid = self.next_uid.wrapping_add(1);
        }
        self.interface_alpha = 0;
        self.interface_flags = 0xFF00u16 as i16;
        self.d_8011d3f0 = 3;
        uid
    }
}

/// `func_800BB0A0` (`code_800BB0A0.c`): the cubic B-spline through four points at `u`: position,
/// roll and view angle.
fn func_800bb0a0(u: f32, p: [[f32; 5]; 4]) -> (Vec3, f32, f32) {
    let u = u.min(1.0);
    let coeff = [(1.0 - u) * (1.0 - u) * (1.0 - u) / 6.0, u * u * u / 2.0 - u * u + 2.0 / 3.0, -u * u * u / 2.0 + u * u / 2.0 + u / 2.0 + 1.0 / 6.0, u * u * u / 6.0];
    let c = |k: usize| coeff[0] * p[0][k] + coeff[1] * p[1][k] + coeff[2] * p[2][k] + coeff[3] * p[3][k];
    (Vec3::new(c(0), c(1), c(2)), c(3), c(4))
}

/// `func_800BB2B4` (`code_800BB0A0.c`): the point on the spline of `points` at `keyframe` +
/// `cur_frame`, then the step to the next frame, by the next two points' `nextPointFrame`s.
/// Returns the position, roll and fov (`None` for each left as it was, when the spline is over
/// before it starts), and true when the spline is over.
///
/// The fov isn't written when the spline returns early (a point `CS_CMD_STOP` within the next
/// three), as in the C.
pub fn func_800bb2b4(points: &[crate::cutscene::CutsceneCameraPoint], keyframe: &mut i16, cur_frame: &mut f32, fov: &mut f32) -> (bool, Option<(Vec3, f32)>) {
    use crate::cutscene::CS_CMD_STOP;
    let mut progress = *cur_frame;
    let key = *keyframe as i32;
    if key < 0 {
        progress = 0.0;
    }
    let pt = |i: i32| points.get(usize::try_from(i).unwrap_or(usize::MAX)).copied().unwrap_or_default();
    if pt(key).continue_flag == CS_CMD_STOP || pt(key + 1).continue_flag == CS_CMD_STOP || pt(key + 2).continue_flag == CS_CMD_STOP {
        return (true, None);
    }
    let mut data = [[0.0f32; 5]; 4];
    for (i, d) in data.iter_mut().enumerate() {
        let p = pt(key + i as i32);
        *d = [p.pos[0] as f32, p.pos[1] as f32, p.pos[2] as f32, p.camera_roll as f32, p.view_angle];
    }
    let (pos, roll, view_angle) = func_800bb0a0(progress, data);
    *fov = view_angle;
    let mut speed1 = 0.0;
    let mut speed2 = 0.0;
    if pt(*keyframe as i32 + 1).next_point_frame != 0 {
        speed1 = 1.0 / pt(*keyframe as i32 + 1).next_point_frame as f32;
    }
    if pt(*keyframe as i32 + 2).next_point_frame != 0 {
        speed2 = 1.0 / pt(*keyframe as i32 + 2).next_point_frame as f32;
    }
    let mut advance = (*cur_frame * (speed2 - speed1)) + speed1;
    if advance < 0.0 {
        advance = 0.0;
    }
    *cur_frame += advance;
    let mut ret = false;
    if *cur_frame >= 1.0 {
        *keyframe += 1;
        if pt(*keyframe as i32 + 3).continue_flag == CS_CMD_STOP {
            *keyframe = 0;
            ret = true;
        }
        *cur_frame -= 1.0;
    }
    (ret, Some((pos, roll)))
}

/// `Demo1ReadOnlyData`, `Demo1ReadWriteData`.
#[derive(Debug, Clone, Copy, Default)]
struct Demo1 {
    interface_flags: i16,
    cur_frame: f32,
    keyframe: i16,
    /// `Camera_Demo1`'s `csEyeUpdate`, `csAtUpdate` and `newRoll`, as last written.
    eye_update: Vec3,
    at_update: Vec3,
    roll: f32,
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

/// `Camera_LERPFloorS`: like `Camera_LERPCeilS`, but within `minDiff` it stays at `cur`.
pub fn lerp_floor_s(target: i16, cur: i16, step_scale: f32, min_diff: i16) -> i16 {
    let diff = target.wrapping_sub(cur);
    if (diff as i32).abs() >= min_diff as i32 {
        let step = (diff as f32 * step_scale + 0.5) as i32 as i16;
        (cur as i32 + step as i32) as i16
    } else {
        cur
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

/// `KeepOn3ReadOnlyData`.
#[derive(Debug, Clone, Copy, Default)]
struct Keep3Ro {
    y_offset: f32,
    min_dist: f32,
    max_dist: f32,
    swing_yaw_initial: f32,
    swing_yaw_final: f32,
    swing_pitch_initial: f32,
    swing_pitch_final: f32,
    swing_pitch_adj: f32,
    fov_target: f32,
    at_lerp_scale_max: f32,
    init_timer: i16,
    flags: i16,
}

/// `KeepOn3ReadWriteData` (`eyeToAtTarget` is (r, yaw, pitch) steps as x, y, z).
#[derive(Debug, Clone, Copy, Default)]
struct Keep3Rw {
    eye_to_at_target: Vec3,
    target: Option<ActorHandle>,
    at_target: Vec3,
    anim_timer: i16,
}

/// `KeepOn0ReadOnlyData`, `KeepOn0ReadWriteData`.
#[derive(Debug, Clone, Copy, Default)]
struct Keep0 {
    fov_scale: f32,
    yaw_scale: f32,
    timer_init: i16,
    interface_flags: i16,
    fov_target: f32,
    anim_timer: i16,
}

/// `D_8011D3B0`, `D_8011D3CC` (`z_camera_data.c`): the yaw and pitch offsets `Camera_KeepOn3`
/// tries in turn when the line to its eye is blocked.
const D_8011D3B0: [u16; 14] = [0x0AAA, 0xF556, 0x1555, 0xEAAB, 0x2AAA, 0xD556, 0x3FFF, 0xC001, 0x5555, 0xAAAB, 0x6AAA, 0x9556, 0x7FFF, 0x0000];
const D_8011D3CC: [u16; 14] = [0x0000, 0x02C6, 0x058C, 0x0000, 0x0000, 0xFD3A, 0x0000, 0x0852, 0x0000, 0x0000, 0x0B18, 0x02C6, 0xFA74, 0x0000];

/// `KeepOn4ReadOnlyData` (`unk_00` .. `unk_1E`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Keep4Ro {
    unk_00: f32,
    unk_04: f32,
    unk_08: f32,
    unk_0c: f32,
    unk_10: f32,
    unk_14: f32,
    unk_18: f32,
    unk_1c: i16,
    unk_1e: i16,
}

/// `KeepOn4ReadWriteData`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Keep4Rw {
    unk_00: f32,
    unk_04: f32,
    unk_08: f32,
    unk_0c: i16,
    unk_0e: i16,
    unk_10: i16,
    unk_12: i16,
    unk_14: i16,
}

/// `Demo3ReadOnlyData`, `Demo3ReadWriteData`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Demo3 {
    fov: f32,
    interface_flags: i16,
    initial_at: Vec3,
    unk_0c: f32,
    anim_frame: i16,
    yaw_dir: i16,
}

/// `D_8011D658` (`z_camera_data.c`): `Camera_Demo3`'s eye offsets (r, pitch, yaw) at its four
/// key frames.
const D_8011D658: [VecSph; 4] = [
    VecSph { r: 50.0, pitch: 0xEE3Au16 as i16, yaw: 0xD558u16 as i16 },
    VecSph { r: 75.0, pitch: 0x0000, yaw: 0x8008u16 as i16 },
    VecSph { r: 80.0, pitch: 0xEE3Au16 as i16, yaw: 0x8008u16 as i16 },
    VecSph { r: 15.0, pitch: 0xEE3Au16 as i16, yaw: 0x8008u16 as i16 },
];
/// `D_8011D678`: its `at` offsets from where the chest opening started.
const D_8011D678: [Vec3; 4] = [Vec3::new(0.0, 40.0, 20.0), Vec3::new(0.0, 40.0, 0.0), Vec3::new(0.0, 3.0, -3.0), Vec3::new(0.0, 3.0, -3.0)];

/// `Subj4ReadOnlyData` (`interfaceFlags`, the only value it reads) and `Subj4ReadWriteData`
/// (`unk_00` is an `InfiniteLine`: the crawlspace's line through `line_point` along
/// `line_dir`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Subj4 {
    interface_flags: i16,
    line_point: Vec3,
    line_dir: Vec3,
    unk_24: f32,
    unk_28: f32,
    unk_2c: i16,
    unk_2e: bool,
    unk_30: i16,
    unk_32: i16,
}

/// `BINANG_LERPIMPINV(v0, v1, t)`: `v0 + (s16)(v1 - v0) / t`, an integer division.
fn binang_lerpimpinv(v0: i16, v1: i16, t: i16) -> i16 {
    (v0 as i32 + v1.wrapping_sub(v0) as i32 / t as i32) as i16
}

/// `Camera_XZAngle(to, from)`.
fn camera_xz_angle(to: Vec3, from: Vec3) -> i16 {
    cam_deg_to_binang(rad_to_deg(f_atan2f(from.x - to.x, from.z - to.z)))
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

/// The fixed and data cameras' `paramData` (`Fixed2`..`Fixed4`, `Data4`; `z64camera.h`).
#[derive(Debug, Clone, Copy, Default)]
struct FixedData {
    // Fixed2ReadOnlyData / Fixed2ReadWriteData.
    fixd2_y_offset: f32,
    fixd2_eye_step_scale: f32,
    fixd2_pos_step_scale: f32,
    fixd2_fov: f32,
    fixd2_interface_flags: i16,
    fixd2_eye: Vec3,
    fixd2_rw_fov: i16,
    // Fixed3ReadOnlyData / Fixed3ReadWriteData.
    fixd3_interface_flags: i16,
    fixd3_rot: [i16; 3],
    fixd3_fov: i16,
    fixd3_upd_dir_timer: i16,
    fixd3_room_image_override_bg_cam_index: i16,
    // Fixed4ReadOnlyData / Fixed4ReadWriteData.
    fixd4_y_offset: f32,
    fixd4_speed_to_eye_pos: f32,
    fixd4_follow_speed: f32,
    fixd4_fov: f32,
    fixd4_interface_flags: i16,
    fixd4_eye_target: Vec3,
    fixd4_rw_follow_speed: f32,
    // Data4ReadOnlyData / Data4ReadWriteData.
    data4_y_offset: f32,
    data4_fov: f32,
    data4_interface_flags: i16,
    data4_eye_pos: Vec3,
    data4_eye_rot: [i16; 3],
    data4_rw_fov: i16,
    data4_flags: i16,
}

/// The unique and special cameras' `paramData` (`Unique0`, `Unique2`, `Unique3`, `Unique6`,
/// `Unique7`, `Special9`).
#[derive(Debug, Clone, Copy, Default)]
struct UniqueData {
    // Unique0: roData.interfaceFlags; rwData.initalPos, animTimer, eyeAndDirection.
    uniq0_interface_flags: i16,
    uniq0_inital_pos: Vec3,
    uniq0_anim_timer: i16,
    uniq0_eye_point: Vec3,
    uniq0_eye_dir: Vec3,
    // Unique2ReadOnlyData / Unique2ReadWriteData.
    uniq2_y_offset: f32,
    uniq2_dist_target: f32,
    uniq2_fov_target: f32,
    uniq2_interface_flags: i16,
    uniq2_unk_00: f32,
    uniq2_unk_04: i16,
    // Unique3ReadOnlyData / Unique3ReadWriteData.
    uniq3_y_offset: f32,
    uniq3_fov: f32,
    uniq3_interface_flags: i16,
    uniq3_initial_fov: f32,
    uniq3_initial_dist: f32,
    // Unique6ReadOnlyData.
    uniq6_interface_flags: i16,
    // Unique7ReadOnlyData / Unique7ReadWriteData.
    uniq7_fov: f32,
    uniq7_interface_flags: i16,
    uniq7_unk_00_x: i16,
    // Special9ReadOnlyData / Special9ReadWriteData.
    spec9_y_offset: f32,
    spec9_unk_04: f32,
    spec9_interface_flags: i16,
    spec9_target_yaw: i16,
}

/// What `Camera_Update` is given each frame besides the camera data.
pub struct CamFrame<'a> {
    pub col: &'a CollisionContext,
    pub player: PlayerView,
    /// `Actor_GetFocus(camera->target)`, or `None` when there's no target or it was killed
    /// (`target->update == NULL`).
    pub target_focus: Option<Vec3>,
    /// `Actor_GetWorldPosShapeRot(doorParams.doorActor)`: the door's position and shape
    /// rotation, if the door camera has one.
    pub door: Option<(Vec3, [i16; 3])>,
    /// `play->transitionMode != TRANS_MODE_OFF`.
    pub transitioning: bool,
    /// `play->state.frames`.
    pub frames: u32,
    /// `D_8015BD7C->state.input[0]`: the modes that end on a button press read it.
    pub input: Input,
    /// Player (`camera->player`), and the OC colliders registered this frame, for
    /// `CollisionCheck_LineOCCheck`.
    pub player_actor: Option<ActorHandle>,
    pub oc_lines: &'a crate::collision_check::OcLines,
    /// `play->csCtx.state != CS_STATE_IDLE`.
    pub cs_active: bool,
    /// `camera->target`'s world position and shape rotation (`Actor_GetWorldPosShapeRot`), for
    /// `Camera_KeepOn4`'s target-relative item cameras.
    pub target_pos_rot: Option<(Vec3, [i16; 3])>,
}

/// `paramData.doorParams` (`Camera_ChangeDoorCam`): what a door camera reads. In the C it's
/// in the union with the mode functions' data, so a function's reload overwrites it; here it
/// stays until the next `Camera_ChangeDoorCam` (the only difference is for a `START1` camera
/// whose own timer is -1, reached some other way than a spawn or a door).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DoorParams {
    pub door_actor: Option<ActorHandle>,
    pub bg_cam_index: i16,
    pub timer1: i16,
    pub timer2: i16,
    pub timer3: i16,
}

/// What `func_80057FC4` reads of the current room.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CamRoom {
    /// `roomShape->base.type == ROOM_SHAPE_TYPE_IMAGE`.
    pub image: bool,
    /// `behaviorType1` (`ROOM_BEHAVIOR_TYPE1_*`).
    pub behavior_type1: u8,
}

/// `BgCamFuncData` (`z64bgcheck.h`): a bg camera's `Vec3s` data as position, rotation, fov and
/// the field that's `roomImageOverrideBgCamIndex`, `timer` or `flags` by setting.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BgCamFuncData {
    pub pos: [i16; 3],
    pub rot: [i16; 3],
    pub fov: i16,
    pub flags: i16,
    pub unk_10: i16,
}

impl BgCamFuncData {
    fn pos_f(&self) -> Vec3 {
        Vec3::new(self.pos[0] as f32, self.pos[1] as f32, self.pos[2] as f32)
    }
}

/// `BgCheck_GetBgCamSettingImpl(colCtx, bgCamIndex, BGCHECK_SCENE)`. An index past the list is
/// `CAM_SET_NONE` here (the C reads past the array).
pub fn bg_cam_setting(col: &CollisionContext, bg_cam_index: i32) -> i16 {
    usize::try_from(bg_cam_index).ok().and_then(|i| col.header.bg_cams.get(i)).map(|c| c.setting as i16).unwrap_or(CAM_SET_NONE)
}

/// `BgCheck_GetBgCamFuncDataImpl(colCtx, bgCamIndex, BGCHECK_SCENE)` as a `BgCamFuncData`
/// (`None` for a NULL pointer or an index past the list; missing entries read as 0).
pub fn bg_cam_func_data(col: &CollisionContext, bg_cam_index: i32) -> Option<BgCamFuncData> {
    let c = col.header.bg_cams.get(usize::try_from(bg_cam_index).ok()?)?;
    if c.data.is_empty() {
        return None;
    }
    let v = |i: usize| c.data.get(i).copied().unwrap_or([0; 3]);
    let (pos, rot, rest) = (v(0), v(1), v(2));
    Some(BgCamFuncData { pos, rot, fov: rest[0], flags: rest[1], unk_10: rest[2] })
}

/// What `Camera_Update` reads from Player each frame.
#[derive(Debug, Clone, Copy)]
pub struct PlayerView {
    /// `actor.world.pos`.
    pub pos: Vec3,
    /// `actor.shape.rot.y`, `actor.shape.rot.x`.
    pub shape_yaw: i16,
    pub shape_pitch: i16,
    /// `actor.world.rot.y` (`Actor_GetWorld`).
    pub world_yaw: i16,
    pub adult: bool,
    /// `R_RUN_SPEED_LIMIT` (for `func_8002DCE4`).
    pub run_speed_limit: i16,
    /// `actor.gravity`.
    pub gravity: f32,
    /// `stateFlags1 & PLAYER_STATE1_21` (climbing).
    pub climbing: bool,
    /// `stateFlags1` (`Camera_Unique0` reads `PLAYER_STATE1_29`).
    pub state1: u32,
}

/// `PLAYER_STATE1_29`: Player in a cutscene-like state (walking through an exit or a door).
const PLAYER_STATE1_29: u32 = 1 << 29;

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
    /// `camDir`: the eye-to-at direction the last `Camera_Update` ended on.
    pub cam_dir: [i16; 3],
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
    /// `setting`, `prevSetting`: `CAM_SET_*`.
    pub setting: i16,
    pub prev_setting: i16,
    /// `bgCamIndex`, `prevBgCamIndex`, `nextBgCamIndex`: the scene's bg camera in use (-1 for
    /// none), the one before, and the one the floor asks for.
    pub bg_cam_index: i16,
    pub prev_bg_cam_index: i16,
    pub next_bg_cam_index: i16,
    /// `paramData.doorParams`.
    pub door_params: DoorParams,
    /// `timer` (-1: none).
    pub timer: i16,
    /// `data2`, `data3` (`Camera_SetCameraData`).
    pub data2: i16,
    pub data3: i16,
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
    /// `play->view.unk_124`: non-zero when a mode function asks for another `Camera_Update`
    /// at the end of `Play_Draw` (`camId | 0x50`).
    pub view_unk_124: u16,
    /// `sCameraInterfaceAlpha` (0 from `Camera_Init`): the alpha type last asked for.
    interface_alpha: u16,
    /// The `Interface_ChangeAlpha` this frame's `Camera_UpdateInterface` asked for (PlayState
    /// applies it to the save).
    pub interface_alpha_change: Option<u16>,
    /// What `Camera_Subj4` wrote to `camera->player` this update: its `world.pos` and
    /// `shape.rot.y` (PlayState applies them to Player right after the update).
    pub player_write: Option<(Vec3, i16)>,
    /// `status` (`CAM_STAT_*`), `camId`, `uid`.
    pub status: i16,
    pub cam_id: i16,
    pub uid: i16,
    /// `camera->player != NULL`: the main camera follows Player; a sub camera only once a
    /// cutscene's points are relative to him (`Camera_SetCSParams`).
    pub has_player: bool,
    /// `data0`, `data1` (`Camera_SetCSParams`): a cutscene's at and eye points.
    pub cs_at_points: Vec<crate::cutscene::CutsceneCameraPoint>,
    pub cs_eye_points: Vec<crate::cutscene::CutsceneCameraPoint>,
    demo1: Demo1,
    ro: Norm1Ro,
    rw: Norm1Rw,
    para1_ro: Para1Ro,
    para1_rw: Para1Rw,
    keep1_ro: Keep1Ro,
    keep1_rw: Keep1Rw,
    keep3_ro: Keep3Ro,
    keep3_rw: Keep3Rw,
    keep0: Keep0,
    keep4_ro: Keep4Ro,
    keep4_rw: Keep4Rw,
    demo3: Demo3,
    subj4: Subj4,
    fixd: FixedData,
    uniq: UniqueData,
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
            cam_dir: [0x71C, p.shape_yaw, 0],
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
            // func_80057FC4 for a room with behaviorType1 0 (Camera_Init has FREE0).
            setting: CAM_SET_NORMAL0,
            prev_setting: CAM_SET_NORMAL0,
            bg_cam_index: -1,
            prev_bg_cam_index: -1,
            next_bg_cam_index: -1,
            // func_80057FC4: Camera_ChangeDoorCam(camera, NULL, -99, 0, 0, 18, 10).
            door_params: DoorParams { door_actor: None, bg_cam_index: -99, timer1: 0, timer2: 18, timer3: 10 },
            timer: -1,
            data2: 0,
            data3: 0,
            unk_14a: 0,
            // Camera_Init: 0x4000; Camera_InitPlayerSettings: |= 4. (Play_Init's func_8005AC48
            // is `play_init_settings`.)
            unk_14c: 0x4000 | 4,
            param_flags: 0,
            target: None,
            target_pos: Vec3::ZERO,
            player_pos_delta: Vec3::ZERO,
            // Camera_InitPlayerSettings, for the main camera.
            interface_flags: 0xB200u16 as i16,
            view_unk_124: 0,
            interface_alpha: 0,
            interface_alpha_change: None,
            player_write: None,
            // Play_Init: Camera_ChangeStatus(&mainCamera, CAM_STAT_ACTIVE).
            status: CAM_STAT_ACTIVE,
            cam_id: CAM_ID_MAIN,
            uid: 0,
            has_player: true,
            cs_at_points: Vec::new(),
            cs_eye_points: Vec::new(),
            demo1: Demo1::default(),
            ro: Norm1Ro::default(),
            rw: Norm1Rw::default(),
            para1_ro: Para1Ro::default(),
            para1_rw: Para1Rw::default(),
            keep1_ro: Keep1Ro::default(),
            keep1_rw: Keep1Rw::default(),
            keep3_ro: Keep3Ro::default(),
            keep3_rw: Keep3Rw::default(),
            keep0: Keep0::default(),
            keep4_ro: Keep4Ro::default(),
            keep4_rw: Keep4Rw::default(),
            demo3: Demo3::default(),
            subj4: Subj4::default(),
            fixd: FixedData::default(),
            uniq: UniqueData::default(),
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

    /// `Camera_Init` for a sub camera (`Play_CreateSubCamera`): no player, eye and at at the
    /// origin, `CAM_SET_FREE0`, `CAM_STAT_CUT` until made active.
    pub fn init_sub(d: &CameraData, g: &mut CameraGlobals, cam_id: i16) -> GameCamera {
        let origin = PlayerView { pos: Vec3::ZERO, shape_yaw: 0, shape_pitch: 0, world_yaw: 0, adult: false, run_speed_limit: 0, gravity: 0.0, climbing: false, state1: 0 };
        let mut c = GameCamera::new(d, &origin);
        let uid = g.camera_init();
        c.eye = Vec3::ZERO;
        c.at = Vec3::ZERO;
        c.eye_next = Vec3::ZERO;
        c.dist = 0.0;
        c.input_dir = [0, 0x3FFF, 0];
        c.cam_dir = c.input_dir;
        c.player_pos = Vec3::ZERO;
        c.player_rot_y = 0;
        c.pos_offset = Vec3::ZERO;
        c.mode = 0;
        c.setting = CAM_SET_FREE0;
        c.prev_setting = CAM_SET_FREE0;
        c.door_params = DoorParams::default();
        c.unk_14c = 0x4000;
        c.status = CAM_STAT_CUT;
        c.cam_id = cam_id;
        c.uid = uid;
        c.has_player = false;
        c
    }

    /// `Camera_ChangeStatus` (the `R_CAM_DATA` copy only feeds the debug register editor).
    pub fn change_status(&mut self, status: i16) -> i16 {
        self.status = status;
        self.status
    }

    /// `Camera_ResetAnim`.
    pub fn reset_anim(&mut self) {
        self.anim_state = 0;
    }

    /// `Camera_SetCSParams`: the cutscene's at and eye points (`data0`, `data1`), and whether
    /// they're relative to Player (`data2`), who then becomes the camera's player.
    pub fn set_cs_params(&mut self, at: Vec<crate::cutscene::CutsceneCameraPoint>, eye: Vec<crate::cutscene::CutsceneCameraPoint>, player: &PlayerView, relative_to_player: i16) {
        self.cs_at_points = at;
        self.cs_eye_points = eye;
        self.data2 = relative_to_player;
        if self.data2 != 0 {
            self.has_player = true;
            self.player_pos = player.pos;
            self.player_rot_y = player.shape_yaw;
            self.next_bg_cam_index = -1;
            self.xz_speed = 0.0;
            self.speed_ratio = 0.0;
        }
    }

    /// `Play_CameraSetAtEye`: `Camera_SetParam` 1 (at) and 2 (eye and eyeNext), the distance,
    /// the offset from Player (`camera->player`'s position, if any), and `atLERPStepScale` 0.01.
    pub fn set_at_eye(&mut self, at: Vec3, eye: Vec3, player_pos: Vec3) {
        self.param_flags &= !(0x10 | 0x8 | 0x1);
        self.at = at;
        self.param_flags |= 1;
        self.eye = eye;
        self.eye_next = eye;
        self.param_flags |= 2;
        self.dist = at.distance(eye);
        self.pos_offset = if self.has_player { at - player_pos } else { Vec3::ZERO };
        self.at_lerp_step_scale = 0.01;
    }

    /// `Camera_SetParam(camera, 0x20, &fov)`.
    pub fn set_fov(&mut self, fov: f32) {
        self.fov = fov;
        self.param_flags |= 0x20;
    }

    /// `Camera_SetParam(camera, 0x40, &roll)`: the roll in degrees.
    pub fn set_roll_deg(&mut self, roll: f32) {
        self.roll = cam_deg_to_binang(roll);
        self.param_flags |= 0x40;
    }

    /// `Camera_Copy(this, src)`: `src`'s at, eye, fov and roll, with the distance and offset
    /// from this camera's player.
    pub fn copy_from(&mut self, d: &CameraData, src: &GameCamera, player_pos: Vec3) {
        self.pos_offset = Vec3::ZERO;
        self.at_lerp_step_scale = 0.1;
        self.at = src.at;
        self.eye = src.eye;
        self.eye_next = src.eye;
        self.dist = self.at.distance(self.eye);
        self.fov = src.fov;
        self.roll = src.roll;
        self.func_80043b60(d);
        if self.has_player {
            // Actor_GetWorld(&playerPosRot, &player->actor).
            self.player_pos = player_pos;
            self.pos_offset = self.at - self.player_pos;
            self.dist = self.player_pos.distance(self.eye);
            self.xz_offset_update_rate = 1.0;
            self.y_offset_update_rate = 1.0;
        }
    }

    /// `Camera_Demo1` (`CAM_SET_CS_0`): the eye and at along a cutscene's splines
    /// (`func_800BB2B4`), relative to Player when `data2` says so (`Camera_RotateAroundPoint`
    /// by his world yaw).
    ///
    /// The eye's and at's splines share one keyframe and frame counter (`rwData`), each call
    /// stepping it: the scripts give the eye's points `nextPointFrame` 0, so only the at's
    /// points pace it.
    fn demo1(&mut self, d: &CameraData, p: &PlayerView) {
        if matches!(self.anim_state, 0 | 10 | 20) {
            self.demo1.interface_flags = d.value(self.cur(), 0);
        }
        self.interface_flags = self.demo1.interface_flags;
        if self.anim_state == 0 {
            self.demo1.keyframe = 0;
            self.demo1.cur_frame = 0.0;
            self.anim_state += 1;
        }
        if self.anim_state == 1 {
            let (mut keyframe, mut cur_frame, mut fov) = (self.demo1.keyframe, self.demo1.cur_frame, self.fov);
            let eye_points = std::mem::take(&mut self.cs_eye_points);
            let at_points = std::mem::take(&mut self.cs_at_points);
            // @bug (game): with `||`, when the eye's spline ends the at's isn't evaluated, and
            // `csAtUpdate` is whatever the stack held. The port keeps the last one; no script of
            // the ported scenes ends that way (the eye's points never advance the spline).
            let (eye_done, eye_upd) = func_800bb2b4(&eye_points, &mut keyframe, &mut cur_frame, &mut fov);
            let (done, at_upd) = if eye_done { (true, None) } else { func_800bb2b4(&at_points, &mut keyframe, &mut cur_frame, &mut fov) };
            self.cs_eye_points = eye_points;
            self.cs_at_points = at_points;
            self.demo1.keyframe = keyframe;
            self.demo1.cur_frame = cur_frame;
            self.fov = fov;
            if done {
                self.anim_state += 1;
            }
            // Each call that doesn't return early writes its position and the roll; what isn't
            // written keeps the last value (the stack slot's).
            if let Some((v, r)) = eye_upd {
                self.demo1.eye_update = v;
                self.demo1.roll = r;
            }
            if let Some((v, r)) = at_upd {
                self.demo1.at_update = v;
                self.demo1.roll = r;
            }
            let (eye_update, at_update, new_roll) = (self.demo1.eye_update, self.demo1.at_update, self.demo1.roll);
            if self.data2 != 0 {
                if self.has_player {
                    // Actor_GetWorld: the position and the world rotation.
                    let rotate = |pos: Vec3| {
                        let mut s = vec3_to_sph_geo(pos);
                        s.yaw = s.yaw.wrapping_add(p.world_yaw);
                        sph_geo_add(p.pos, s)
                    };
                    self.eye_next = rotate(eye_update);
                    self.at = rotate(at_update);
                } else {
                    log::warn!("camera: spline demo: owner dead");
                }
            } else {
                self.eye_next = eye_update;
                self.at = at_update;
            }
            self.eye = self.eye_next;
            self.roll = (new_roll * 256.0) as i16;
            self.dist = self.at.distance(self.eye);
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
        if (d.setting_flags(self.setting) & 0x3FFF_FFFF) & (1u32 << mode) == 0 {
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

    /// `Camera_CheckValidMode`: 0 if the setting has no such mode, -1 if it's the current one.
    pub fn check_valid_mode(&self, d: &CameraData, mode: i16) -> i32 {
        if d.setting_flags(self.setting) & (1u32 << mode) == 0 {
            0
        } else if mode == self.mode {
            -1
        } else {
            (0x8000_0000u32 | mode as u32) as i32
        }
    }

    /// `Play_Init`'s part of the main camera's start: `func_8005AC48(&mainCamera, 0xFF)` (which
    /// among other bits enables the floor's bg cameras, bit 1), then `Camera_InitPlayerSettings`'
    /// `unk_14C |= 4` and `func_80057FC4` for the first room.
    pub fn play_init_settings(&mut self, room: CamRoom) {
        self.unk_14c = 0xFF;
        self.unk_14c |= 4;
        self.func_80057fc4(room);
        self.copy_data_to_regs();
    }

    /// `func_80057FC4`: the main camera's setting for the room: `FREE0` in a prerendered room
    /// (the bg camera from the spawn or the viewpoint follows), else `DUNGEON0` for
    /// `ROOM_BEHAVIOR_TYPE1_1` and `NORMAL0` for the rest (keeping the door parameters of
    /// `Camera_ChangeDoorCam(camera, NULL, -99, 0, 0, 18, 10)`).
    pub fn func_80057fc4(&mut self, room: CamRoom) {
        if room.image {
            self.setting = CAM_SET_FREE0;
            self.prev_setting = CAM_SET_FREE0;
            self.unk_14c &= !0x4;
            return;
        }
        self.door_params = DoorParams { door_actor: None, bg_cam_index: -99, timer1: 0, timer2: 18, timer3: 10 };
        self.copy_data_to_regs();
        match room.behavior_type1 {
            1 => {
                self.setting = CAM_SET_DUNGEON0;
                self.prev_setting = CAM_SET_DUNGEON0;
            }
            0 => {
                self.setting = CAM_SET_NORMAL0;
                self.prev_setting = CAM_SET_NORMAL0;
            }
            _ => {
                self.setting = CAM_SET_NORMAL0;
                self.prev_setting = CAM_SET_NORMAL0;
                self.unk_14c |= 4;
            }
        }
    }

    /// `Camera_ChangeSettingFlags`. Returns the setting, or -1 (already it), -2 (a change of
    /// higher priority happened this frame), -99 (not a setting).
    ///
    /// Not modelled: `-5` for `CAM_SET_MEADOW_BIRDS_EYE` / `_UNUSED` for adult Link in
    /// `SCENE_SPOT05` (the Sacred Forest Meadow isn't played yet).
    pub fn change_setting_flags(&mut self, d: &CameraData, setting: i16, flags: i16) -> i16 {
        let priority = |s: i16| (d.setting_flags(s) & 0x0F00_0000) >> 24;
        if self.unk_14a & 1 != 0 && priority(self.setting) >= priority(setting) {
            self.unk_14a |= 0x10;
            return -2;
        }
        if setting == CAM_SET_NONE || setting >= CAM_SET_MAX {
            return -99;
        }
        if setting == self.setting && flags & 1 == 0 {
            self.unk_14a |= 0x10;
            if flags & 2 == 0 {
                self.unk_14a |= 1;
            }
            return -1;
        }
        self.unk_14a |= 0x10;
        if flags & 2 == 0 {
            self.unk_14a |= 1;
        }
        self.unk_14c |= 0xC;
        self.unk_14c &= !0x1008;
        if d.setting_flags(self.setting) & 0x4000_0000 == 0 {
            self.prev_setting = self.setting;
        }
        if flags & 8 != 0 {
            self.bg_cam_index = self.prev_bg_cam_index;
            self.prev_bg_cam_index = -1;
        } else if flags & 4 == 0 {
            if d.setting_flags(self.setting) & 0x4000_0000 == 0 {
                self.prev_bg_cam_index = self.bg_cam_index;
            }
            self.bg_cam_index = -1;
        }
        self.setting = setting;
        // (0x80000000 | mode) is negative: only the "no such mode, already NORMAL" 0 copies.
        if self.change_mode_flags(d, self.mode, 1) >= 0 {
            self.copy_data_to_regs();
        }
        setting
    }

    /// `Camera_ChangeSetting`.
    pub fn change_setting(&mut self, d: &CameraData, setting: i16) -> i16 {
        self.change_setting_flags(d, setting, 0)
    }

    /// `Camera_ChangeBgCamIndex`: the bg camera's setting, keeping the index. At most once a
    /// frame (`unk_14A & 0x40`).
    pub fn change_bg_cam_index(&mut self, d: &CameraData, col: &CollisionContext, bg_cam_index: i32) -> i32 {
        if bg_cam_index == -1 || bg_cam_index == self.bg_cam_index as i32 {
            self.unk_14a |= 0x40;
            return -1;
        }
        if self.unk_14a & 0x40 == 0 {
            let new_setting = bg_cam_setting(col, bg_cam_index);
            self.unk_14a |= 0x40;
            let ok = self.change_setting_flags(d, new_setting, 5) >= 0;
            if ok || d.setting_flags(self.setting) & 0x8000_0000 != 0 {
                self.bg_cam_index = bg_cam_index as i16;
                self.unk_14a |= 4;
                self.copy_data_to_regs();
            }
            return (0x8000_0000u32 | bg_cam_index as u32) as i32;
        }
        // @bug (game): the C falls off the end without a return value; no caller reads it.
        0
    }

    /// `Camera_ChangeDoorCam`: `bg_cam_index` -99 only stores the door's parameters, -1 uses
    /// `CAM_SET_DOORC`, anything else that bg camera's setting.
    #[allow(clippy::too_many_arguments)]
    pub fn change_door_cam(&mut self, d: &CameraData, col: &CollisionContext, door: Option<ActorHandle>, bg_cam_index: i16, timer1: i16, timer2: i16, timer3: i16) -> i32 {
        if self.setting == CAM_SET_CS_ATTENTION || self.setting == CAM_SET_DOORC {
            return 0;
        }
        self.door_params = DoorParams { door_actor: door, bg_cam_index, timer1, timer2, timer3 };
        if bg_cam_index == -99 {
            self.copy_data_to_regs();
            return -99;
        }
        if bg_cam_index == -1 {
            self.change_setting(d, CAM_SET_DOORC);
        } else {
            let setting = bg_cam_setting(col, bg_cam_index as i32);
            self.unk_14a |= 0x40;
            if self.change_setting(d, setting) >= 0 {
                self.bg_cam_index = bg_cam_index;
                self.unk_14a |= 4;
            }
        }
        self.copy_data_to_regs();
        -1
    }

    /// `func_8005B1A4`: tells a door or exit camera that Player is through (`unk_14C |= 8`).
    pub fn func_8005b1a4(&mut self) {
        self.unk_14c |= 0x8;
    }

    /// `Camera_SetCameraData(camera, 4 | 8, ...)`: `data2` and `data3`.
    pub fn set_camera_data(&mut self, flags: i16, data2: i16, data3: i16) {
        if flags & 4 != 0 {
            self.data2 = data2;
        }
        if flags & 8 != 0 {
            self.data3 = data3;
        }
    }

    /// `Camera_GetBgCamFuncData`: the current bg camera's data.
    fn bg_cam_data(&self, col: &CollisionContext) -> BgCamFuncData {
        bg_cam_func_data(col, self.bg_cam_index as i32).unwrap_or_default()
    }

    /// `(setting, mode)`: the key of the current mode's data.
    fn cur(&self) -> (i16, i16) {
        (self.setting, self.mode)
    }

    /// `Camera_Update`, with `Camera_UpdateInterface`'s letterbox target at the end: `g` is the
    /// state every camera shares (`CameraGlobals`).
    pub fn update(&mut self, d: &CameraData, f: &CamFrame, letterbox: &mut Letterbox, g: &mut CameraGlobals) {
        self.interface_flags = g.interface_flags;
        self.interface_alpha = g.interface_alpha;
        self.oob_timer = g.oob_timer;
        self.update_inner(d, f, letterbox, g);
        g.interface_flags = self.interface_flags;
        g.interface_alpha = self.interface_alpha;
        g.oob_timer = self.oob_timer;
    }

    fn update_inner(&mut self, d: &CameraData, f: &CamFrame, letterbox: &mut Letterbox, g: &mut CameraGlobals) {
        let (col, p, frames) = (f.col, &f.player, f.frames);
        if self.status == CAM_STAT_CUT {
            return;
        }
        self.update_direction = false;
        if self.has_player {
            self.update_player(d, f);
        }
        if self.status == CAM_STAT_WAIT {
            return;
        }
        self.unk_14a = 0;
        self.unk_14c &= !(0x400 | 0x20);
        self.unk_14c |= 0x10;
        if self.oob_timer < 200 {
            // sCameraFunctions[sCameraSettings[setting].cameraModes[mode].funcIdx]. A function
            // that isn't ported runs the setting's NORMAL function if that one is, else Normal1
            // on NORMAL0's NORMAL data.
            let func = |m: i16| d.mode(self.setting, m).map(|m| m.func.as_str()).filter(|f| PORTED.contains(f));
            match func(self.mode).or_else(|| func(CAM_MODE_NORMAL)) {
                Some("CAM_FUNC_NORM1") => {
                    let key = if d.mode(self.setting, self.mode).is_some_and(|m| m.func == "CAM_FUNC_NORM1") { self.cur() } else { (self.setting, CAM_MODE_NORMAL) };
                    self.normal1(d, col, p, key, frames)
                }
                Some("CAM_FUNC_PARA1") => self.parallel1(d, col, p, frames),
                Some("CAM_FUNC_KEEP1") => self.keep_on1(d, col, p, f.target_focus),
                Some("CAM_FUNC_KEEP3") => self.keep_on3(d, col, p, f),
                Some("CAM_FUNC_KEEP0") => self.keep_on0(d, col, f.target_focus),
                Some("CAM_FUNC_FIXD2") => self.fixed2(d, col, p),
                Some("CAM_FUNC_FIXD3") => self.fixed3(d, col),
                Some("CAM_FUNC_FIXD4") => self.fixed4(d, col, p),
                Some("CAM_FUNC_DATA4") => self.data4(d, col, p),
                Some("CAM_FUNC_UNIQ0") => self.unique0(d, col, p, &f.input),
                Some("CAM_FUNC_UNIQ2") => self.unique2(d, col, p),
                Some("CAM_FUNC_UNIQ3") => self.unique3(d, col, p, &f.input),
                Some("CAM_FUNC_UNIQ6") => self.unique6(d, p),
                Some("CAM_FUNC_UNIQ7") => self.unique7(d, col),
                Some("CAM_FUNC_SPEC9") => self.special9(d, col, p, f.door, frames, &f.input),
                Some("CAM_FUNC_KEEP4") => self.keep_on4(d, col, p, f),
                Some("CAM_FUNC_DEMO3") => self.demo3(d, col, p, frames, &f.input),
                Some("CAM_FUNC_SUBJ4") => {
                    self.subj4(d, col, p);
                }
                Some("CAM_FUNC_DEMO1") => self.demo1(d, p),
                _ => self.normal1(d, col, p, (CAM_SET_NORMAL0, CAM_MODE_NORMAL), frames),
            }
        } else if self.has_player {
            let e = diff_to_sph_geo(self.at, self.eye);
            self.calc_at_default(d, &e, 0.0, false, p);
        }

        if self.status == CAM_STAT_ACTIVE {
            // (gameMode is GAMEMODE_NORMAL.) After a Camera_Init, the main camera's first three
            // updates hold the interface at 0x3200; a running transition holds it at 0xF200 and
            // a cutscene at 0x3200.
            if g.d_8011d3f0 != 0 && self.cam_id == CAM_ID_MAIN {
                g.d_8011d3f0 -= 1;
                self.interface_flags = 0x3200;
            } else if f.transitioning {
                self.interface_flags = 0xF200u16 as i16;
            } else if f.cs_active {
                self.interface_flags = 0x3200;
            }
            self.update_interface(letterbox);
        }
        if self.status == CAM_STAT_UNK3 {
            return;
        }

        let angle = diff_to_sph_geo(self.eye, self.at);
        self.up = calc_up(angle.pitch, angle.yaw, self.roll);
        self.cam_dir = [angle.pitch, angle.yaw, 0];
        if !self.update_direction {
            self.input_dir = [angle.pitch, angle.yaw, 0];
        }
    }

    /// `Camera_Update`'s part for a camera with a player (`camera->player != NULL`): his speed
    /// and floor, and the floor's bg camera.
    fn update_player(&mut self, d: &CameraData, f: &CamFrame) {
        let (col, p) = (f.col, &f.player);
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

        if self.oob_timer < 200 {
            // (Camera_UpdateWater and Camera_UpdateHotRoom aren't ported.)
            if self.unk_14c & 4 == 0 {
                self.next_bg_cam_index = -1;
            }
            // The floor's bg camera (bit 0x200, underwater, is never set here; the iron boots
            // exception with it doesn't arise).
            if self.unk_14c & 1 != 0 && self.unk_14c & 4 != 0 && self.unk_14c & 0x400 == 0 && self.unk_14c & 0x200 == 0 && self.unk_14c as u16 & 0x8000 == 0 && ground != bgcheck::BGCHECK_Y_MIN
                && let Some(id) = poly
            {
                // Camera_GetBgCamIndex: -1 when the index's setting is CAM_SET_NONE. Only a
                // scene floor's index is taken (a DynaPoly's names its own actor's list).
                use crate::surface::SurfaceType;
                let idx = col.bg_cam_index(id) as i32;
                if id.is_scene() && bg_cam_setting(col, idx) != CAM_SET_NONE {
                    self.next_bg_cam_index = idx as i16;
                }
            }
            if self.next_bg_cam_index != -1 && (cur.y - ground).abs() < 2.0 && self.unk_14c & 0x200 == 0 {
                let next = self.next_bg_cam_index as i32;
                self.change_bg_cam_index(d, col, next);
                self.next_bg_cam_index = -1;
            }
        }
    }

    /// `Camera_UpdateInterface(sCameraInterfaceFlags)`: `flags & 0x7000` picks the letterbox's
    /// size (`sCameraLetterboxSize`), `0x8000` sets it at once, all of `0xF000` set leaves it
    /// alone; `flags & 0x0F00` is the interface's alpha type (0 is 50), all set leaves it alone,
    /// and a new one is `Interface_ChangeAlpha`'s (`interface_alpha_change`).
    fn update_interface(&mut self, letterbox: &mut Letterbox) {
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
        if flags & 0x0F00 != 0x0F00 {
            let mut interface_alpha = (flags & 0x0F00) >> 8;
            if interface_alpha == 0 {
                interface_alpha = 0x32;
            }
            if interface_alpha != self.interface_alpha {
                self.interface_alpha = interface_alpha;
                self.interface_alpha_change = Some(interface_alpha);
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

    /// `Camera_Normal1`, reading the values of `data_mode` (a setting and a mode).
    fn normal1(&mut self, d: &CameraData, col: &CollisionContext, p: &PlayerView, data_mode: (i16, i16), frames: u32) {
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
            let v = |i: usize| d.value(self.cur(), i) as f32;
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
                interface_flags: d.value(self.cur(), 8),
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
            let v = |i: usize| d.value(self.cur(), i) as f32;
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
                interface_flags: d.value(self.cur(), 10),
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

    /// `Camera_KeepOn3` (TALK in the normal settings): over `initTimer` frames the camera
    /// swings to frame Player and the one talking, from the side the eye already is on, trying
    /// other angles while the line to the new eye is blocked (by the scene or another actor's
    /// OC collider). It holds there until Player is through (`func_8005B1A4`) and moves or
    /// presses a button.
    ///
    /// Its first frame asks for another `Camera_Update` after `Play_Draw` (`view.unk_124`),
    /// where the swing is set up.
    fn keep_on3(&mut self, d: &CameraData, col: &CollisionContext, p: &PlayerView, f: &CamFrame) {
        let mut player_height = p.height();
        let Some(focus) = f.target_focus.filter(|_| self.target.is_some()) else {
            // "talk: target is not valid, change parallel".
            self.target = None;
            self.change_mode(d, CAM_MODE_TARGET);
            return;
        };
        let reload = matches!(self.anim_state, 0 | 10 | 20);
        if reload {
            if self.view_unk_124 == 0 {
                self.unk_14c |= 0x20;
                // camera->camId | 0x50: the main camera.
                self.view_unk_124 = 0x50;
                return;
            }
            self.unk_14c &= !0x20;
        }
        self.unk_14c &= !0x10;
        if reload {
            let v = |i: usize| d.value(self.cur(), i) as f32;
            let y_normal = 1.0 + d.oreg_s(R_CAM_YOFFSET_NORM) - d.oreg_s(R_CAM_YOFFSET_NORM) * (68.0 / player_height);
            self.keep3_ro = Keep3Ro {
                y_offset: v(0) * 0.01 * player_height * y_normal,
                min_dist: v(1),
                max_dist: v(2),
                swing_yaw_initial: v(3),
                swing_yaw_final: v(4),
                swing_pitch_initial: v(5),
                swing_pitch_final: v(6),
                swing_pitch_adj: v(7) * 0.01,
                fov_target: v(8),
                at_lerp_scale_max: v(9) * 0.01,
                init_timer: d.value(self.cur(), 10),
                flags: d.value(self.cur(), 11),
            };
        }
        let ro = self.keep3_ro;
        player_height += ro.y_offset;
        let at_to_eye_next_dir = diff_to_sph_geo(self.at, self.eye_next);
        self.target_pos = focus;
        let player_head = self.player_pos + Vec3::Y * player_height;
        let mut target_to_player_dir = diff_to_sph_geo(player_head, self.target_pos);
        self.interface_flags = ro.flags;
        if reload {
            self.anim_state += 1;
            let rw = &mut self.keep3_rw;
            rw.target = self.target;
            let temp_f0 = if ro.max_dist < target_to_player_dir.r { 1.0 } else { target_to_player_dir.r / ro.max_dist };
            rw.anim_timer = ro.init_timer;
            let sp_bc = ((1.0 - temp_f0) * target_to_player_dir.r) / rw.anim_timer as f32;
            let lerp = |a: f32, b: f32| a + (b - a) * temp_f0;
            let mut adj = VecSph::default();
            let swing = lerp(ro.swing_pitch_initial, ro.swing_pitch_final);
            adj.pitch = cam_deg_to_binang(swing).wrapping_add((-(target_to_player_dir.pitch as f32 * ro.swing_pitch_adj)) as i32 as i16);
            let swing = cam_deg_to_binang(lerp(ro.swing_yaw_initial, ro.swing_yaw_final));
            let rel = target_to_player_dir.yaw.wrapping_sub(at_to_eye_next_dir.yaw);
            let behind = target_to_player_dir.yaw.wrapping_sub(0x7FFF);
            adj.yaw = if ro.flags & 0x10 != 0 {
                if rel < 0 { target_to_player_dir.yaw.wrapping_add(swing) } else { target_to_player_dir.yaw.wrapping_sub(swing) }
            } else if ro.flags & 0x20 != 0 {
                if rel < 0 { behind.wrapping_sub(swing) } else { behind.wrapping_add(swing) }
            } else if (rel as i32).abs() < 0x3FFF {
                if rel < 0 { target_to_player_dir.yaw.wrapping_add(swing) } else { target_to_player_dir.yaw.wrapping_sub(swing) }
            } else if rel < 0 {
                behind.wrapping_sub(swing)
            } else {
                behind.wrapping_add(swing)
            };
            let prev_target_player_dist = target_to_player_dir.r;
            target_to_player_dir.r = (sp_bc * 0.6) + (prev_target_player_dist * (1.0 - 0.6));
            let (sp80, sp82) = (adj.yaw, adj.pitch);
            rw.at_target = sph_geo_add(player_head, target_to_player_dir);
            target_to_player_dir.r = prev_target_player_dist;
            adj.r = ro.min_dist + (target_to_player_dir.r * (1.0 - 0.5)) - at_to_eye_next_dir.r + at_to_eye_next_dir.r;
            let at_target = rw.at_target;
            let mut line_b = sph_geo_add(at_target, adj);
            if ro.flags & 0x80 == 0 {
                let exclusions = [self.target, f.player_actor];
                for i in 0..D_8011D3B0.len() {
                    if !f.oc_lines.line_oc_check(at_target, line_b, &exclusions) && !Self::bg_check(col, at_target, &mut line_b) {
                        break;
                    }
                    adj.yaw = sp80.wrapping_add(D_8011D3B0[i] as i16);
                    adj.pitch = sp82.wrapping_add(D_8011D3CC[i] as i16);
                    line_b = sph_geo_add(at_target, adj);
                }
            }
            self.unk_14c &= !0xC;
            let rw = &mut self.keep3_rw;
            let pad = (((rw.anim_timer as i32 + 1) * rw.anim_timer as i32) >> 1) as f32;
            rw.eye_to_at_target.y = adj.yaw.wrapping_sub(at_to_eye_next_dir.yaw) as f32 / pad;
            rw.eye_to_at_target.z = adj.pitch.wrapping_sub(at_to_eye_next_dir.pitch) as f32 / pad;
            rw.eye_to_at_target.x = (adj.r - at_to_eye_next_dir.r) / pad;
            return;
        }
        if self.keep3_rw.anim_timer != 0 {
            let rw = self.keep3_rw;
            let t = rw.anim_timer as f32;
            self.at += (rw.at_target - self.at) / t;
            let adj = VecSph {
                r: ((rw.eye_to_at_target.x * t) + at_to_eye_next_dir.r) + 1.0,
                yaw: at_to_eye_next_dir.yaw.wrapping_add((rw.eye_to_at_target.y * t) as i32 as i16),
                pitch: at_to_eye_next_dir.pitch.wrapping_add((rw.eye_to_at_target.z * t) as i32 as i16),
            };
            self.eye_next = sph_geo_add(self.at, adj);
            self.eye = self.eye_next;
            self.fov = lerp_ceil_f(ro.fov_target, self.fov, 0.5, 1.0);
            self.roll = lerp_ceil_s(0, self.roll, 0.5, 0xA);
            self.at_lerp_step_scale = self.clamp_lerp_scale(d, ro.at_lerp_scale_max);
            let mut eye = self.eye;
            Self::bg_check(col, self.at, &mut eye);
            self.eye = eye;
            self.keep3_rw.anim_timer -= 1;
        } else {
            self.unk_14c |= 0x410;
        }
        if self.unk_14c & 8 != 0 {
            self.interface_flags = 0;
            self.func_80043b60(d);
            self.at_lerp_step_scale = 0.0;
            use eng_input::pad::*;
            let pressed = [BTN_A, BTN_B, BTN_CLEFT, BTN_CDOWN, BTN_CUP, BTN_CRIGHT, BTN_R, BTN_Z].iter().any(|&b| f.input.press.held(b));
            if self.xz_speed > 0.001 || pressed {
                self.unk_14c |= 4;
                self.unk_14c &= !8;
            }
        }
    }

    /// `Camera_KeepOn0` (TALK in the prerendered rooms): the eye stays at the bg camera, and
    /// over `timerInit` frames `at` turns towards the one talking while the view narrows by
    /// `fovScale`.
    fn keep_on0(&mut self, d: &CameraData, col: &CollisionContext, target_focus: Option<Vec3>) {
        self.unk_14c &= !0x10;
        if matches!(self.anim_state, 0 | 10 | 20) {
            let cur = self.cur();
            let v = |i: usize| d.value(cur, i);
            self.keep0.fov_scale = v(0) as f32 * 0.01;
            self.keep0.yaw_scale = v(1) as f32 * 0.01;
            self.keep0.timer_init = v(2);
            self.keep0.interface_flags = v(3);
        }
        let b = self.bg_cam_data(col);
        self.eye_next = b.pos_f();
        self.eye = self.eye_next;
        let fov = if b.fov == -1 { 6000 } else { b.fov };
        let Some(focus) = target_focus.filter(|_| self.target.is_some()) else {
            // "talk: target is not valid, change normal camera".
            self.target = None;
            self.change_mode(d, CAM_MODE_NORMAL);
            return;
        };
        self.target_pos = focus;
        let mut eye_at_offset = diff_to_sph_geo(self.eye, self.at);
        let eye_target_pos_offset = diff_to_sph_geo(self.eye, self.target_pos);
        self.interface_flags = self.keep0.interface_flags;
        if self.anim_state == 0 {
            self.anim_state += 1;
            // CAM_DATA_SCALED.
            self.fov = fov as f32 * 0.01;
            self.roll = 0;
            self.at_lerp_step_scale = 0.0;
            self.keep0.anim_timer = self.keep0.timer_init;
            self.keep0.fov_target = self.fov - (self.fov * self.keep0.fov_scale);
        }
        if self.keep0.anim_timer != 0 {
            let step = (eye_target_pos_offset.yaw.wrapping_sub(eye_at_offset.yaw) / self.keep0.anim_timer) as f32 * self.keep0.yaw_scale;
            eye_at_offset.yaw = (eye_at_offset.yaw as f32 + step) as i32 as i16;
            self.at = sph_geo_add(self.eye, eye_at_offset);
            self.keep0.anim_timer -= 1;
        } else {
            self.unk_14c |= 0x400 | 0x10;
        }
        self.fov = lerp_ceil_f(self.keep0.fov_target, self.fov, 0.5, 10.0);
    }

    /// `Camera_KeepOn4` (`CAM_SET_TURN_AROUND`, "ITEM2"): the camera turns in front of Player to
    /// look at him holding an item up. `data2` (`Camera_SetCameraData`) is the kind of item and
    /// adjusts the data (9 for `func_8084E6D4`'s get-item). Over `unk_1E` frames the eye swings
    /// from where it was to the chosen pitch and yaw around Player's head (retrying other angles
    /// when a collider or a wall is in the way), then holds until Player is done (`unk_14C & 8`,
    /// `func_8005B1A4`) and the previous setting or bg camera comes back.
    fn keep_on4(&mut self, d: &CameraData, col: &CollisionContext, p: &PlayerView, f: &CamFrame) {
        let reload = matches!(self.anim_state, 0 | 10 | 20);
        if reload {
            if self.view_unk_124 == 0 {
                self.unk_14c |= 0x20;
                self.unk_14c &= !(0x4 | 0x2);
                self.view_unk_124 = 0x50;
                return;
            }
            self.keep4_rw.unk_14 = self.data2;
            self.unk_14c &= !0x20;
        }
        if self.keep4_rw.unk_14 != self.data2 {
            // "camera: item: item type changed".
            self.anim_state = 20;
            self.unk_14c |= 0x20;
            self.unk_14c &= !(0x4 | 0x2);
            self.view_unk_124 = 0x50;
            return;
        }
        let player_height = p.height();
        self.unk_14c &= !0x10;
        if reload {
            let t = -0.5f32;
            let y_normal = 1.0 + t - (68.0 / player_height * t);
            let cur = self.cur();
            let v = |i: usize| d.value(cur, i) as f32;
            let mut ro = Keep4Ro {
                unk_00: v(0) * 0.01 * player_height * y_normal,
                unk_04: v(1) * 0.01 * player_height * y_normal,
                unk_08: v(2),
                unk_0c: v(3),
                unk_10: v(4),
                unk_18: v(5),
                unk_1c: d.value(cur, 6),
                unk_14: v(7) * 0.01,
                unk_1e: d.value(cur, 8),
            };
            match self.data2 {
                1 => {
                    ro.unk_00 = player_height * -0.6 * y_normal;
                    ro.unk_04 = player_height * 2.0 * y_normal;
                    ro.unk_08 = 10.0;
                }
                2 | 3 => {
                    ro.unk_08 = -20.0;
                    ro.unk_18 = 80.0;
                }
                4 => {
                    ro.unk_00 = player_height * -0.2 * y_normal;
                    ro.unk_08 = 25.0;
                }
                8 => {
                    ro.unk_00 = player_height * -0.2 * y_normal;
                    ro.unk_04 = player_height * 0.8 * y_normal;
                    ro.unk_08 = 50.0;
                    ro.unk_18 = 70.0;
                }
                9 => {
                    ro.unk_00 = player_height * 0.1 * y_normal;
                    ro.unk_04 = player_height * 0.5 * y_normal;
                    ro.unk_08 = -20.0;
                    ro.unk_0c = 0.0;
                    ro.unk_1c = 0x2540;
                }
                5 => {
                    ro.unk_00 = player_height * -0.4 * y_normal;
                    ro.unk_08 = -10.0;
                    ro.unk_0c = 45.0;
                    ro.unk_1c = 0x2002;
                }
                10 => {
                    ro.unk_00 = player_height * -0.5 * y_normal;
                    ro.unk_04 = player_height * 1.5 * y_normal;
                    ro.unk_08 = -15.0;
                    ro.unk_0c = 175.0;
                    ro.unk_18 = 70.0;
                    ro.unk_1c = 0x2202;
                    ro.unk_1e = 0x3C;
                }
                12 => {
                    ro.unk_00 = player_height * -0.6 * y_normal;
                    ro.unk_04 = player_height * 1.6 * y_normal;
                    ro.unk_08 = -2.0;
                    ro.unk_0c = 120.0;
                    // PLAYER_STATE1_27: swimming.
                    ro.unk_10 = if p.state1 & (1 << 27) != 0 { 0.0 } else { 20.0 };
                    ro.unk_1c = 0x3212;
                    ro.unk_1e = 0x1E;
                    ro.unk_18 = 50.0;
                }
                0x5A => {
                    ro.unk_00 = player_height * -0.3 * y_normal;
                    ro.unk_18 = 45.0;
                    ro.unk_1c = 0x2F02;
                }
                0x5B => {
                    ro.unk_00 = player_height * -0.1 * y_normal;
                    ro.unk_04 = player_height * 1.5 * y_normal;
                    ro.unk_08 = -3.0;
                    ro.unk_0c = 10.0;
                    ro.unk_18 = 55.0;
                    ro.unk_1c = 0x2F08;
                }
                0x51 => {
                    ro.unk_00 = player_height * -0.3 * y_normal;
                    ro.unk_04 = player_height * 1.5 * y_normal;
                    ro.unk_08 = 2.0;
                    ro.unk_0c = 20.0;
                    ro.unk_10 = 20.0;
                    ro.unk_1c = 0x2280;
                    ro.unk_1e = 0x1E;
                    ro.unk_18 = 45.0;
                }
                11 => {
                    ro.unk_00 = player_height * -0.19 * y_normal;
                    ro.unk_04 = player_height * 0.7 * y_normal;
                    ro.unk_0c = 130.0;
                    ro.unk_10 = 10.0;
                    ro.unk_1c = 0x2522;
                }
                _ => {}
            }
            self.keep4_ro = ro;
        }
        let ro = self.keep4_ro;
        self.update_direction = true;
        self.interface_flags = ro.unk_1c;
        let spa8 = diff_to_sph_geo(self.at, self.eye_next);
        let mut d_8015bd50 = self.player_pos + Vec3::Y * player_height;
        // BgCheck_CameraRaycastDown2.
        let (ground, _) = col.raycast_down(d_8015bd50, bgcheck::IGNORE_CAMERA, bgcheck::DOWN_CHECK_WALLS | bgcheck::DOWN_CHECK_FLOORS, 1.0);
        if ground > ro.unk_00 + d_8015bd50.y {
            d_8015bd50.y = ground + 10.0;
        } else {
            d_8015bd50.y += ro.unk_00;
        }
        let mut spb8 = VecSph::default();
        match self.anim_state {
            0 | 20 => {
                let mut exclusions = vec![f.player_actor];
                self.func_80043abc(d);
                self.unk_14c &= !(0x4 | 0x2);
                self.keep4_rw.unk_10 = ro.unk_1e;
                self.keep4_rw.unk_08 = self.player_pos.y - self.player_pos_delta.y;
                let behind = self.player_rot_y.wrapping_sub(0x7FFF);
                let (spa2, spa0);
                if ro.unk_1c & 2 != 0 {
                    spa2 = cam_deg_to_binang(ro.unk_08);
                    let turn = cam_deg_to_binang(ro.unk_0c);
                    spa0 = if behind.wrapping_sub(spa8.yaw) > 0 { behind.wrapping_add(turn) } else { behind.wrapping_sub(turn) };
                } else if ro.unk_1c & 4 != 0 {
                    spa2 = cam_deg_to_binang(ro.unk_08);
                    spa0 = cam_deg_to_binang(ro.unk_0c);
                } else if ro.unk_1c & 8 != 0
                    && let Some((_, rot)) = f.target_pos_rot.filter(|_| self.target.is_some())
                {
                    spa2 = cam_deg_to_binang(ro.unk_08).wrapping_sub(rot[0]);
                    let tb = rot[1].wrapping_sub(0x7FFF);
                    let turn = cam_deg_to_binang(ro.unk_0c);
                    spa0 = if tb.wrapping_sub(spa8.yaw) > 0 { tb.wrapping_add(turn) } else { tb.wrapping_sub(turn) };
                    exclusions.push(self.target);
                } else if ro.unk_1c & 0x80 != 0
                    && let Some((pos, _)) = f.target_pos_rot.filter(|_| self.target.is_some())
                {
                    spa2 = cam_deg_to_binang(ro.unk_08);
                    // Camera_XZAngle(&target, &playerPos).
                    let sp9e = cam_deg_to_binang(rad_to_deg(f_atan2f(self.player_pos.x - pos.x, self.player_pos.z - pos.z)));
                    let turn = cam_deg_to_binang(ro.unk_0c);
                    spa0 = if sp9e.wrapping_sub(spa8.yaw) > 0 { sp9e.wrapping_add(turn) } else { sp9e.wrapping_sub(turn) };
                    exclusions.push(self.target);
                } else if ro.unk_1c & 0x40 != 0 {
                    spa2 = cam_deg_to_binang(ro.unk_08);
                    spa0 = spa8.yaw;
                } else {
                    spa2 = spa8.pitch;
                    spa0 = spa8.yaw;
                }
                spb8 = VecSph { r: ro.unk_04, pitch: spa2, yaw: spa0 };
                let mut d_8015bd70 = sph_geo_add(d_8015bd50, spb8);
                if ro.unk_1c & 1 == 0 {
                    for i in 0..D_8011D3B0.len() {
                        if !f.oc_lines.line_oc_check(d_8015bd50, d_8015bd70, &exclusions) && !Self::bg_check(col, d_8015bd50, &mut d_8015bd70) {
                            break;
                        }
                        spb8.yaw = (D_8011D3B0[i] as i16).wrapping_add(spa0);
                        spb8.pitch = (D_8011D3CC[i] as i16).wrapping_add(spa2);
                        d_8015bd70 = sph_geo_add(d_8015bd50, spb8);
                    }
                }
                let rw = &mut self.keep4_rw;
                rw.unk_04 = spb8.pitch.wrapping_sub(spa8.pitch) as f32 / rw.unk_10 as f32;
                rw.unk_00 = spb8.yaw.wrapping_sub(spa8.yaw) as f32 / rw.unk_10 as f32;
                rw.unk_0c = spa8.yaw;
                rw.unk_0e = spa8.pitch;
                self.anim_state += 1;
                rw.unk_12 = 1;
            }
            10 => self.keep4_rw.unk_08 = self.player_pos.y - self.player_pos_delta.y,
            _ => {}
        }
        self.xz_offset_update_rate = 0.25;
        self.y_offset_update_rate = 0.25;
        self.at_lerp_step_scale = 0.75;
        let mut at = self.at;
        lerp_ceil_vec3(d_8015bd50, &mut at, 0.5, 0.5, 0.2);
        self.at = at;
        if ro.unk_10 != 0.0 {
            spb8 = VecSph { r: ro.unk_10, pitch: 0, yaw: self.player_rot_y };
            self.at = sph_geo_add(self.at, spb8);
        }
        self.at_lerp_step_scale = 0.0;
        self.dist = lerp_ceil_f(ro.unk_04, self.dist, 0.25, 2.0);
        spb8.r = self.dist;
        if self.keep4_rw.unk_10 != 0 {
            self.unk_14c |= 0x20;
            let rw = &mut self.keep4_rw;
            rw.unk_0c = rw.unk_0c.wrapping_add(rw.unk_00 as i32 as i16);
            rw.unk_0e = rw.unk_0e.wrapping_add(rw.unk_04 as i32 as i16);
            rw.unk_10 -= 1;
        } else if ro.unk_1c & 0x10 != 0 {
            self.unk_14c |= 0x400 | 0x10;
            self.unk_14c |= 0x4 | 0x2;
            self.unk_14c &= !8;
            if self.timer > 0 {
                self.timer -= 1;
            }
        } else {
            self.unk_14c |= 0x400 | 0x10;
            if self.unk_14c & 8 != 0 || ro.unk_1c & 0x80 != 0 {
                self.interface_flags = 0;
                self.unk_14c |= 0x4 | 0x2;
                self.unk_14c &= !8;
                self.restore_setting(d, col);
            }
        }
        spb8.yaw = lerp_ceil_s(self.keep4_rw.unk_0c, spa8.yaw, ro.unk_14, 4);
        spb8.pitch = lerp_ceil_s(self.keep4_rw.unk_0e, spa8.pitch, ro.unk_14, 4);
        self.eye_next = sph_geo_add(self.at, spb8);
        self.eye = self.eye_next;
        let mut eye = self.eye;
        Self::bg_check(col, self.at, &mut eye);
        self.eye = eye;
        self.fov = lerp_ceil_f(ro.unk_18, self.fov, self.fov_update_rate, 1.0);
        self.roll = lerp_ceil_s(0, self.roll, 0.5, 0xA);
    }

    /// The end of an item or chest camera: back to the setting before
    /// (`Camera_ChangeSettingFlags(prevSetting, 2)`), or to the bg camera before.
    fn restore_setting(&mut self, d: &CameraData, col: &CollisionContext) {
        if self.prev_bg_cam_index < 0 {
            let prev = self.prev_setting;
            self.change_setting_flags(d, prev, 2);
        } else {
            let prev = self.prev_bg_cam_index as i32;
            self.change_bg_cam_index(d, col, prev);
            self.prev_bg_cam_index = -1;
        }
    }

    /// `Camera_Demo3` (`CAM_SET_SLOW_CHEST_CS`, "ITEM0"): a big chest opening on a major item.
    /// From where Player stands, the camera circles in from one side (the side picked by the
    /// frame's parity, the other if a wall is there) through `D_8011D658` / `D_8011D678`'s
    /// four key frames (frames 2, 148, 159, 168), holds, and 60 frames after the last (or on a
    /// press, once Player is done) pulls back 80 and gives the setting back.
    fn demo3(&mut self, d: &CameraData, col: &CollisionContext, p: &PlayerView, frames: u32, input: &Input) {
        let y_offset = p.height();
        self.unk_14c &= !0x10;
        if matches!(self.anim_state, 0 | 10 | 20) {
            let cur = self.cur();
            self.demo3.fov = d.value(cur, 0) as f32;
            // unk_04 (unused), then the interface flags.
            self.demo3.interface_flags = d.value(cur, 2);
        }
        let eye_at_offset = diff_to_sph_geo(self.at, self.eye);
        self.interface_flags = self.demo3.interface_flags;
        let rot_y = self.player_rot_y;
        let mut eye_offset = VecSph::default();
        let mut skip_update_eye = false;
        let lerp_f = |a: f32, b: f32, t: f32| a + (b - a) * t;
        let lerp_s = |a: i16, b: i16, t: f32| a.wrapping_add((b.wrapping_sub(a) as f32 * t) as i32 as i16);
        match self.anim_state {
            0 => {
                self.unk_14c &= !(0x8 | 0x4);
                self.func_80043b60(d);
                self.fov = self.demo3.fov;
                self.roll = 0;
                self.demo3.anim_frame = 0;
                let mut initial_at = self.player_pos;
                if self.player_ground_y != bgcheck::BGCHECK_Y_MIN {
                    initial_at.y = self.player_ground_y;
                }
                self.demo3.initial_at = initial_at;
                let mut angle = rot_y;
                let sp68 = Vec3::new(initial_at.x + sin_s(angle) * 40.0, initial_at.y + 40.0, initial_at.z + cos_s(angle) * 40.0);
                if frames & 1 != 0 {
                    angle = angle.wrapping_sub(0x3FFF);
                    self.demo3.yaw_dir = 1;
                } else {
                    angle = angle.wrapping_add(0x3FFF);
                    self.demo3.yaw_dir = -1;
                }
                let mut sp74 = Vec3::new(sp68.x + D_8011D658[1].r * sin_s(angle), initial_at.y + 5.0, sp68.z + D_8011D658[1].r * cos_s(angle));
                if Self::bg_check(col, sp68, &mut sp74) {
                    self.demo3.yaw_dir = -self.demo3.yaw_dir;
                }
                let mut at_offset = vec3_to_sph_geo(D_8011D678[0]);
                at_offset.yaw = at_offset.yaw.wrapping_add(rot_y);
                self.at = sph_geo_add(initial_at, at_offset);
                eye_offset = VecSph { r: D_8011D658[0].r, pitch: D_8011D658[0].pitch, yaw: D_8011D658[0].yaw.wrapping_mul(self.demo3.yaw_dir).wrapping_add(rot_y) };
                self.demo3.unk_0c = 1.0;
            }
            1 | 2 | 3 => {
                let (k, t) = match self.anim_state {
                    1 => (0, (self.demo3.anim_frame - 2) as f32 * (1.0 / 146.0)),
                    2 => (1, (self.demo3.anim_frame - 0x94) as f32 * 0.1),
                    _ => (2, (self.demo3.anim_frame - 0x9F) as f32 * (1.0 / 9.0)),
                };
                let (a, bb) = (D_8011D678[k], D_8011D678[k + 1]);
                let mut sp5c = Vec3::new(lerp_f(a.x, bb.x, t), 0.0, lerp_f(a.z, bb.z, t));
                sp5c.y = match self.anim_state {
                    1 => lerp_f(a.y, bb.y, t),
                    2 => lerp_f(a.y - y_offset, bb.y, t) + y_offset,
                    _ => lerp_f(a.y, bb.y, t) + y_offset,
                };
                let mut at_offset = vec3_to_sph_geo(sp5c);
                at_offset.yaw = at_offset.yaw.wrapping_mul(self.demo3.yaw_dir).wrapping_add(rot_y);
                self.at = sph_geo_add(self.demo3.initial_at, at_offset);
                let (e0, e1) = (D_8011D658[k], D_8011D658[k + 1]);
                let r = lerp_f(e0.r, e1.r, t);
                let pitch = lerp_s(e0.pitch, e1.pitch, t);
                let yaw = lerp_s(e0.yaw, e1.yaw, t);
                eye_offset = VecSph { r, pitch, yaw: yaw.wrapping_mul(self.demo3.yaw_dir).wrapping_add(rot_y) };
                self.demo3.unk_0c += match self.anim_state {
                    1 => -(1.0 / 365.0),
                    2 => -0.04,
                    _ => 4.0 / 45.0,
                };
            }
            30 | 10 | 20 => {
                if self.anim_state == 30 {
                    self.unk_14c |= 0x400;
                    if self.unk_14c & 8 != 0 {
                        self.anim_state = 4;
                    }
                }
                skip_update_eye = true;
            }
            4 => {
                eye_offset = VecSph { r: 80.0, pitch: 0, yaw: eye_at_offset.yaw };
                self.demo3.unk_0c = 0.1;
                self.interface_flags = 0x3400;
                let pressed = Self::any_button_pressed(input);
                if !((self.demo3.anim_frame < 0 || self.xz_speed > 0.001 || pressed) && self.unk_14c & 8 != 0) {
                    skip_update_eye = true;
                } else {
                    self.demo3_end(d, col);
                    skip_update_eye = true;
                }
            }
            _ => {
                self.demo3_end(d, col);
                skip_update_eye = true;
            }
        }
        self.demo3.anim_frame = self.demo3.anim_frame.wrapping_add(1);
        self.anim_state = match self.demo3.anim_frame {
            1 => 10,
            2 => 1,
            148 => 2,
            158 => 20,
            159 => 3,
            168 => 30,
            228 => 4,
            _ => self.anim_state,
        };
        if !skip_update_eye {
            let t = self.demo3.unk_0c;
            eye_offset.r = lerp_ceil_f(eye_offset.r, eye_at_offset.r, t, 2.0);
            eye_offset.pitch = lerp_ceil_s(eye_offset.pitch, eye_at_offset.pitch, t, 0xA);
            eye_offset.yaw = lerp_ceil_s(eye_offset.yaw, eye_at_offset.yaw, t, 0xA);
            self.eye_next = sph_geo_add(self.at, eye_offset);
            self.eye = self.eye_next;
        }
        self.dist = self.at.distance(self.eye);
        self.at_lerp_step_scale = 0.1;
        self.pos_offset = self.at - self.player_pos;
    }

    /// `Camera_Demo3`'s default case: done; the setting before comes back.
    fn demo3_end(&mut self, d: &CameraData, col: &CollisionContext) {
        self.unk_14c |= 0x14;
        self.unk_14c &= !8;
        self.restore_setting(d, col);
        self.interface_flags = 0;
    }

    // ---- The fixed, data, unique and special cameras ----------------------------------------

    /// `func_80043ABC`.
    fn func_80043abc(&mut self, d: &CameraData) {
        self.yaw_update_rate_inv = 100.0;
        self.pitch_update_rate_inv = d.oreg(R_CAM_DEFAULT_PITCH_UPDATE_RATE_INV) as f32;
        self.r_update_rate_inv = d.oreg(6) as f32;
        self.xz_offset_update_rate = d.oreg_s(2);
        self.y_offset_update_rate = d.oreg_s(3);
        self.fov_update_rate = d.oreg_s(4);
    }

    /// `func_80043B60`.
    fn func_80043b60(&mut self, d: &CameraData) {
        self.r_update_rate_inv = d.oreg(27) as f32;
        self.yaw_update_rate_inv = d.oreg(27) as f32;
        self.pitch_update_rate_inv = d.oreg(27) as f32;
        self.xz_offset_update_rate = 0.001;
        self.y_offset_update_rate = 0.001;
        self.fov_update_rate = 0.001;
    }

    /// `1 + R_CAM_YOFFSET_NORM - R_CAM_YOFFSET_NORM * (68 / playerHeight)`, the modes'
    /// `yNormal`.
    fn y_normal(d: &CameraData, player_height: f32) -> f32 {
        1.0 + d.oreg_s(R_CAM_YOFFSET_NORM) - d.oreg_s(R_CAM_YOFFSET_NORM) * (68.0 / player_height)
    }

    /// The pressed buttons the button-ended modes check (A, B, the C buttons, R, Z).
    fn any_button_pressed(input: &Input) -> bool {
        use eng_input::pad::{BTN_A, BTN_B, BTN_CDOWN, BTN_CUP, BTN_R, BTN_Z};
        [BTN_A, BTN_B, BTN_CLEFT, BTN_CDOWN, BTN_CUP, BTN_CRIGHT, BTN_R, BTN_Z].iter().any(|&b| input.press.held(b))
    }

    /// `Camera_CheckOOB`: whether a poly is between `from` and `to` with `from` behind it.
    fn check_oob(col: &CollisionContext, from: Vec3, to: Vec3) -> bool {
        // BgCheck_CameraLineTest1(.., chkWall 1, chkFloor 1, chkCeil 1, chkOneFace 0, ..).
        let bcc = bgcheck::CHECK_WALL | bgcheck::CHECK_FLOOR | bgcheck::CHECK_CEILING | bgcheck::CHECK_DYNA;
        match col.check_line(bgcheck::IGNORE_CAMERA, bgcheck::IGNORE_NONE, from, to, 1.0, bcc) {
            // CollisionPoly_GetPointDistanceFromPlane.
            Some((_, poly)) => bgcheck::dist_plane_to_pos(col.poly_normal(poly), col.poly(poly).dist as f32, from) < 0.0,
            None => false,
        }
    }

    /// `Camera_GetBgCamFuncDataUnderPlayer`: the Vec3s data of the bg camera the floor below
    /// Player names (`BgCheck_EntityRaycastDown3` from Player's height), with its count; `None`
    /// for no floor, or a floor with no data. Only the scene's floors: a DynaPoly's floor names
    /// its own actor's list, which no ported actor has.
    fn bg_cam_func_data_under_player(col: &CollisionContext, p: &PlayerView) -> Option<Vec<[i16; 3]>> {
        use crate::surface::SurfaceType;
        let (y, poly) = col.entity_raycast_down(p.pos + Vec3::Y * p.height());
        let poly = poly.filter(|id| y != bgcheck::BGCHECK_Y_MIN && id.is_scene())?;
        let cam = col.header.bg_cams.get(col.bg_cam_index(poly) as usize)?;
        (!cam.data.is_empty()).then(|| cam.data.clone())
    }

    /// `Camera_Subj4` (`CAM_SET_CRAWLSPACE`): the crawlspace's subjective camera. Its first
    /// call each frame only asks for the second one at the end of `Play_Draw` (`view.unk_124`)
    /// and keeps the frame's `xzSpeed`. The second eases in over 10 frames, then, while Player
    /// moves (`unk_24` ≥ 0.5), puts the eye on the crawlspace's line at Player, bobbing and
    /// swaying with the crawl, and moves Player: onto the line, to the ground, facing along it
    /// (`player_write`). The bg camera's points: the second is one end, the second to last the
    /// other. Returns the C's value. The crawl's sound (`func_800F4010`) isn't ported.
    fn subj4(&mut self, d: &CameraData, col: &CollisionContext, p: &PlayerView) -> bool {
        if matches!(self.anim_state, 0 | 10 | 20) {
            self.subj4.interface_flags = d.value(self.cur(), 0);
        }
        if self.view_unk_124 == 0 {
            // camera->camId | 0x50: the main camera.
            self.view_unk_124 = 0x50;
            self.subj4.unk_24 = self.xz_speed;
            return true;
        }
        // Actor_GetWorldPosShapeRot(&sp6C, &camera->player->actor).
        let (sp6c_pos, sp6c_rot_x, sp6c_rot_y) = (p.pos, p.shape_pitch, p.shape_yaw);
        let mut sp5c = diff_to_sph_geo(self.at, self.eye);
        self.interface_flags = self.subj4.interface_flags;
        if self.anim_state == 0 {
            let Some(points) = Self::bg_cam_func_data_under_player(col, p).filter(|v| v.len() >= 3) else {
                // The C reads through a NULL pointer here.
                log::warn!("Camera_Subj4: no crawlspace points under Player");
                return false;
            };
            let v = |i: usize| Vec3::new(points[i][0] as f32, points[i][1] as f32, points[i][2] as f32);
            let s = &mut self.subj4;
            s.line_point = v(1);
            let sp98 = v(points.len() - 2);
            // 0x238C ~ 50 degrees.
            let mut sp64 = VecSph { r: 10.0, pitch: 0x238C, yaw: camera_xz_angle(sp98, s.line_point) };
            let sp88 = self.player_pos.distance(s.line_point);
            if self.player_pos.distance(sp98) < sp88 {
                s.line_dir = s.line_point - sp98;
                s.line_point = sp98;
            } else {
                s.line_dir = sp98 - s.line_point;
                sp64.yaw = sp64.yaw.wrapping_sub(0x7FFF);
            }
            s.unk_30 = sp64.yaw;
            s.unk_32 = 0xA;
            s.unk_2c = 0;
            s.unk_2e = false;
            s.unk_28 = 0.0;
            self.anim_state += 1;
        }
        let s = self.subj4;
        if s.unk_32 != 0 {
            let sp64 = VecSph { r: 10.0, pitch: 0x238C, yaw: s.unk_30 };
            let sp8c = sph_geo_add(sp6c_pos, sp64);
            let sp88 = s.unk_32 as f32 + 1.0;
            self.at.x += (sp8c.x - self.at.x) / sp88;
            self.at.y += (sp8c.y - self.at.y) / sp88;
            self.at.z += (sp8c.z - self.at.z) / sp88;
            sp5c.r -= sp5c.r / sp88;
            sp5c.yaw = binang_lerpimpinv(sp5c.yaw, sp6c_rot_y.wrapping_sub(0x7FFF), s.unk_32);
            sp5c.pitch = binang_lerpimpinv(sp5c.pitch, sp6c_rot_x, s.unk_32);
            self.eye_next = sph_geo_add(self.at, sp5c);
            self.eye = self.eye_next;
            self.subj4.unk_32 -= 1;
            return false;
        } else if s.unk_24 < 0.5 {
            return false;
        }
        self.eye_next = eng_collision::math3d::line_closest_to_point(s.line_point, s.line_dir, sp6c_pos);
        self.at = self.eye_next + s.line_dir;
        self.eye = self.eye_next;
        let sp64 = VecSph { r: 5.0, pitch: 0x238C, yaw: s.unk_30 };
        let sp98 = sph_geo_add(self.eye_next, sp64);
        let s = &mut self.subj4;
        s.unk_2c = s.unk_2c.wrapping_add(0xBB8);
        let t = cos_s(s.unk_2c);
        self.eye.x += (sp98.x - self.eye.x) * t.abs();
        self.eye.y += (sp98.y - self.eye.y) * t.abs();
        self.eye.z += (sp98.z - self.eye.z) * t.abs();
        if s.unk_28 < t && !s.unk_2e {
            // func_800F4010(&player->actor.projectedPos, player->unk_89E + 0x8B0, 4.0f): the
            // crawl's sound, not ported.
            s.unk_2e = true;
        } else if s.unk_28 > t {
            s.unk_2e = false;
        }
        s.unk_28 = t;
        self.player_write = Some((Vec3::new(self.eye_next.x, self.player_ground_y, self.eye_next.z), sp64.yaw));
        let temp_f16 = (240.0 * t) * (s.unk_24 * 0.416667);
        let temp_a0 = (temp_f16 + s.unk_30 as f32) as i32 as i16;
        self.at = Vec3::new(self.eye.x + sin_s(temp_a0) * 10.0, self.eye.y, self.eye.z + cos_s(temp_a0) * 10.0);
        self.roll = lerp_ceil_s(0, self.roll, 0.5, 0xA);
        true
    }

    /// `Camera_Fixed2`: the eye eases to the bg camera's position, `at` follows Player
    /// (`PIVOT_CRAWLSPACE`).
    fn fixed2(&mut self, d: &CameraData, col: &CollisionContext, p: &PlayerView) {
        let player_height = p.height();
        if matches!(self.anim_state, 0 | 10 | 20) {
            let key = self.cur();
            let v = |i: usize| d.value(key, i);
            let y_normal = Self::y_normal(d, player_height);
            let f = &mut self.fixd;
            f.fixd2_y_offset = (v(0) as f32 * 0.01 * player_height) * y_normal;
            f.fixd2_eye_step_scale = v(1) as f32 * 0.01;
            f.fixd2_pos_step_scale = v(2) as f32 * 0.01;
            f.fixd2_fov = v(3) as f32;
            f.fixd2_interface_flags = v(4);
            f.fixd2_rw_fov = (f.fixd2_fov * 100.0) as i16;
            match bg_cam_func_data(col, self.bg_cam_index as i32) {
                Some(b) => {
                    f.fixd2_eye = b.pos_f();
                    if b.fov != -1 {
                        f.fixd2_rw_fov = b.fov;
                    }
                }
                None => f.fixd2_eye = self.eye,
            }
            if f.fixd2_rw_fov <= 360 {
                f.fixd2_rw_fov = f.fixd2_rw_fov.wrapping_mul(100);
            }
        }
        let f = self.fixd;
        self.interface_flags = f.fixd2_interface_flags;
        let pos_offset_target = Vec3::new(0.0, f.fixd2_y_offset + player_height, 0.0);
        let mut off = self.pos_offset;
        lerp_ceil_vec3(pos_offset_target, &mut off, f.fixd2_pos_step_scale, f.fixd2_pos_step_scale, 0.1);
        self.pos_offset = off;
        let at_target = self.player_pos + self.pos_offset;
        if self.anim_state == 0 {
            self.anim_state += 1;
            self.func_80043b60(d);
            if f.fixd2_interface_flags & 1 == 0 {
                self.eye = f.fixd2_eye;
                self.eye_next = f.fixd2_eye;
                self.at = at_target;
            }
        }
        let mut at = self.at;
        lerp_ceil_vec3(at_target, &mut at, f.fixd2_pos_step_scale, f.fixd2_pos_step_scale, 10.0);
        self.at = at;
        let mut eye_next = self.eye_next;
        lerp_ceil_vec3(f.fixd2_eye, &mut eye_next, f.fixd2_eye_step_scale, f.fixd2_eye_step_scale, 0.1);
        self.eye_next = eye_next;
        self.eye = self.eye_next;
        self.dist = self.at.distance(self.eye);
        self.roll = 0;
        self.xz_speed = 0.0;
        self.fov = f.fixd2_rw_fov as f32 * 0.01;
        self.at_lerp_step_scale = self.clamp_lerp_scale(d, 1.0);
        self.pos_offset = self.at - self.player_pos;
    }

    /// `Camera_Fixed3` (`PREREND_FIXED`): the eye at the bg camera's position, looking along its
    /// rotation; nothing moves. `R_CAM_DATA(CAM_DATA_FOV)` is only a copy of the fov (the debug
    /// register editor aside).
    fn fixed3(&mut self, d: &CameraData, col: &CollisionContext) {
        let b = self.bg_cam_data(col);
        if matches!(self.anim_state, 0 | 10 | 20) {
            let flags = d.value(self.cur(), 0);
            let f = &mut self.fixd;
            f.fixd3_interface_flags = flags;
            self.eye_next = b.pos_f();
            self.eye = self.eye_next;
            f.fixd3_rot = b.rot;
            f.fixd3_fov = b.fov;
            f.fixd3_room_image_override_bg_cam_index = b.flags;
            if f.fixd3_fov == -1 {
                f.fixd3_fov = 6000;
            }
            if f.fixd3_fov <= 360 {
                f.fixd3_fov = f.fixd3_fov.wrapping_mul(100);
            }
        }
        if self.anim_state == 0 {
            self.fixd.fixd3_upd_dir_timer = 5;
            self.anim_state += 1;
        }
        let f = &mut self.fixd;
        if b.flags != f.fixd3_room_image_override_bg_cam_index {
            // "camera: position change".
            f.fixd3_room_image_override_bg_cam_index = b.flags;
            f.fixd3_upd_dir_timer = 5;
        }
        if f.fixd3_upd_dir_timer > 0 {
            f.fixd3_upd_dir_timer -= 1;
            self.update_direction = true;
        } else {
            self.update_direction = false;
        }
        let at_sph = VecSph { r: 150.0, yaw: f.fixd3_rot[1], pitch: f.fixd3_rot[0].wrapping_neg() };
        self.at = sph_geo_add(self.eye, at_sph);
        self.interface_flags = f.fixd3_interface_flags;
        self.roll = 0;
        self.fov = f.fixd3_fov as f32 * 0.01;
        self.at_lerp_step_scale = 0.0;
    }

    /// `Camera_Fixed4` (`PIVOT_IN_FRONT`): the eye eases to the bg camera's position and `at`
    /// turns to follow Player.
    fn fixed4(&mut self, d: &CameraData, col: &CollisionContext, p: &PlayerView) {
        let player_height = p.height();
        if matches!(self.anim_state, 0 | 10 | 20) {
            let key = self.cur();
            let v = |i: usize| d.value(key, i);
            let y_normal = Self::y_normal(d, player_height);
            let f = &mut self.fixd;
            f.fixd4_y_offset = v(0) as f32 * 0.01 * player_height * y_normal;
            f.fixd4_speed_to_eye_pos = v(1) as f32 * 0.01;
            f.fixd4_follow_speed = v(2) as f32 * 0.01;
            f.fixd4_fov = v(3) as f32;
            f.fixd4_interface_flags = v(4);
            f.fixd4_eye_target = bg_cam_func_data(col, self.bg_cam_index as i32).map(|b| b.pos_f()).unwrap_or(self.eye);
        }
        self.interface_flags = self.fixd.fixd4_interface_flags;
        if self.anim_state == 0 {
            self.anim_state += 1;
            if self.fixd.fixd4_interface_flags & 4 == 0 {
                self.func_80043b60(d);
            }
            self.fixd.fixd4_rw_follow_speed = self.fixd.fixd4_follow_speed;
        }
        let f = self.fixd;
        self.eye_next += (f.fixd4_eye_target - self.eye_next) * f.fixd4_speed_to_eye_pos;
        self.eye = self.eye_next;
        let pos_offset_target = Vec3::new(0.0, f.fixd4_y_offset + player_height, 0.0);
        let mut off = self.pos_offset;
        lerp_ceil_vec3(pos_offset_target, &mut off, 0.1, 0.1, 0.1);
        self.pos_offset = off;
        let player_with_offset = self.player_pos + self.pos_offset;
        let at_target = self.at + (player_with_offset - self.at) * 0.5;
        let mut at_eye_next = diff_to_sph_geo(self.eye_next, self.at);
        let at_target_eye_next = diff_to_sph_geo(self.eye_next, at_target);
        at_eye_next.r += (at_target_eye_next.r - at_eye_next.r) * f.fixd4_rw_follow_speed;
        at_eye_next.pitch = lerp_ceil_s(at_target_eye_next.pitch, at_eye_next.pitch, f.fixd4_rw_follow_speed * self.speed_ratio, 0xA);
        at_eye_next.yaw = lerp_ceil_s(at_target_eye_next.yaw, at_eye_next.yaw, f.fixd4_rw_follow_speed * self.speed_ratio, 0xA);
        self.at = sph_geo_add(self.eye_next, at_eye_next);
        self.dist = self.at.distance(self.eye);
        self.roll = 0;
        self.fov = f.fixd4_fov;
        self.at_lerp_step_scale = self.clamp_lerp_scale(d, 1.0);
    }

    /// `Camera_Data4` (`PIVOT_SHOP_BROWSING`): the eye at the bg camera, `at` along the eye's
    /// rotation plus `data2` / `data3` degrees when its flags say so.
    fn data4(&mut self, d: &CameraData, col: &CollisionContext, p: &PlayerView) {
        let player_height = p.height();
        if matches!(self.anim_state, 0 | 10 | 20) {
            let key = self.cur();
            let v = |i: usize| d.value(key, i);
            let y_normal = Self::y_normal(d, player_height);
            let b = self.bg_cam_data(col);
            let f = &mut self.fixd;
            f.data4_y_offset = v(0) as f32 * 0.01 * player_height * y_normal;
            f.data4_fov = v(1) as f32;
            f.data4_interface_flags = v(2);
            f.data4_eye_pos = b.pos_f();
            f.data4_eye_rot = b.rot;
            f.data4_rw_fov = b.fov;
            if b.fov != -1 {
                f.data4_fov = if f.data4_rw_fov <= 360 { f.data4_rw_fov as f32 } else { f.data4_rw_fov as f32 * 0.01 };
            }
            f.data4_flags = b.flags;
            self.eye = f.data4_eye_pos;
        }
        let f = self.fixd;
        self.interface_flags = f.data4_interface_flags;
        if self.anim_state == 0 {
            self.anim_state += 1;
            self.func_80043b60(d);
        }
        let eye_next_at = diff_to_sph_geo(self.at, self.eye_next);
        self.calc_at_default(d, &eye_next_at, f.data4_y_offset, false, p);
        let eye_at = diff_to_sph_geo(self.eye, self.at);
        let at_offset = VecSph {
            r: eye_at.r,
            yaw: if f.data4_flags & 1 != 0 { cam_deg_to_binang(self.data2 as f32).wrapping_add(f.data4_eye_rot[1]) } else { eye_at.yaw },
            pitch: if f.data4_flags & 2 != 0 { cam_deg_to_binang(self.data3 as f32).wrapping_add(f.data4_eye_rot[0]) } else { eye_at.pitch },
        };
        self.at = sph_geo_add(self.eye, at_offset);
        let look_at = self.player_pos + Vec3::Y * player_height;
        self.dist = look_at.distance(self.eye);
        self.roll = 0;
        self.xz_speed = 0.0;
        self.fov = f.data4_fov;
        self.at_lerp_step_scale = 0.0;
    }

    /// `Camera_Unique0` (`START1`): the eye at the bg camera, `at` where its line of sight
    /// passes Player, until Player moves (or a button is pressed) after its timer; then back
    /// to the previous setting.
    fn unique0(&mut self, d: &CameraData, col: &CollisionContext, p: &PlayerView, input: &Input) {
        let y_offset = p.height();
        if matches!(self.anim_state, 0 | 10 | 20) {
            self.uniq.uniq0_interface_flags = d.value(self.cur(), 0);
        }
        let player_with_offset = self.player_pos + Vec3::Y * y_offset;
        self.interface_flags = self.uniq.uniq0_interface_flags;
        if self.anim_state == 0 {
            self.func_80043b60(d);
            self.unk_14c &= !4;
            let b = self.bg_cam_data(col);
            self.uniq.uniq0_eye_point = b.pos_f();
            self.eye = self.uniq.uniq0_eye_point;
            self.eye_next = self.eye;
            if b.fov != -1 {
                self.fov = if b.fov <= 360 { b.fov as f32 } else { b.fov as f32 * 0.01 };
            }
            // bgCamFuncData->timer, else the door parameters' timers.
            self.uniq.uniq0_anim_timer = if b.flags == -1 { self.door_params.timer1.wrapping_add(self.door_params.timer2) } else { b.flags };
            let at_player = VecSph { r: player_with_offset.distance(self.eye), yaw: b.rot[1], pitch: b.rot[0].wrapping_neg() };
            self.uniq.uniq0_eye_dir = sph_geo_to_vec3(at_player);
            self.at = eng_collision::math3d::line_closest_to_point(self.uniq.uniq0_eye_point, self.uniq.uniq0_eye_dir, self.player_pos);
            self.uniq.uniq0_inital_pos = self.player_pos;
            self.anim_state += 1;
        }
        let cutscene = p.state1 & PLAYER_STATE1_29 != 0;
        if cutscene {
            self.uniq.uniq0_inital_pos = self.player_pos;
        }
        let leave = |s: &mut Self| {
            s.dist = s.at.distance(s.eye);
            s.pos_offset = s.at - s.player_pos;
            s.at_lerp_step_scale = 0.0;
        };
        if self.uniq.uniq0_interface_flags & 1 != 0 {
            if self.uniq.uniq0_anim_timer > 0 {
                self.uniq.uniq0_anim_timer -= 1;
                self.uniq.uniq0_inital_pos = self.player_pos;
            } else if !cutscene && (dist_xz(self.player_pos, self.uniq.uniq0_inital_pos) >= 10.0 || Self::any_button_pressed(input)) {
                leave(self);
                self.unk_14c |= 4;
                let prev = self.prev_setting;
                self.change_setting_flags(d, prev, 2);
            }
        } else {
            if self.uniq.uniq0_anim_timer > 0 {
                self.uniq.uniq0_anim_timer -= 1;
                if self.uniq.uniq0_anim_timer == 0 {
                    self.interface_flags = 0;
                }
            } else {
                self.uniq.uniq0_inital_pos = self.player_pos;
            }
            if !cutscene && (0.001 < self.xz_speed || Self::any_button_pressed(input)) {
                leave(self);
                let prev = self.prev_setting;
                self.change_setting_flags(d, prev, 2);
                self.unk_14c |= 4;
            }
        }
    }

    /// `Camera_Unique2` (`SCENE_TRANSITION`, and hanging): `at` follows Player, the eye keeps
    /// `distTarget` away.
    fn unique2(&mut self, d: &CameraData, col: &CollisionContext, p: &PlayerView) {
        let player_height = p.height();
        let eye_at = diff_to_sph_geo(self.at, self.eye);
        if matches!(self.anim_state, 0 | 10 | 20) {
            let key = self.cur();
            let v = |i: usize| d.value(key, i);
            let y_normal = Self::y_normal(d, player_height);
            let u = &mut self.uniq;
            u.uniq2_y_offset = v(0) as f32 * 0.01 * player_height * y_normal;
            u.uniq2_dist_target = v(1) as f32;
            u.uniq2_fov_target = v(2) as f32;
            u.uniq2_interface_flags = v(3);
        }
        let u = self.uniq;
        self.interface_flags = u.uniq2_interface_flags;
        if self.anim_state == 0 || self.uniq.uniq2_unk_04 != u.uniq2_interface_flags {
            self.uniq.uniq2_unk_04 = u.uniq2_interface_flags;
        }
        if self.anim_state == 0 {
            self.anim_state = 1;
            self.func_80043b60(d);
            self.uniq.uniq2_unk_00 = 200.0;
            if u.uniq2_interface_flags & 0x10 != 0 {
                self.unk_14c &= !4;
            }
        }
        let player_pos = self.player_pos;
        let rate = if u.uniq2_interface_flags & 1 != 0 { 1.0 } else { self.speed_ratio };
        self.at.x += (player_pos.x - self.at.x) * (rate * 0.6);
        self.at.y += ((player_pos.y + player_height + u.uniq2_y_offset) - self.at.y) * 0.4;
        self.at.z += (player_pos.z - self.at.z) * (rate * 0.6);
        // unk_00: unused.
        self.uniq.uniq2_unk_00 += (2.0 - self.uniq.uniq2_unk_00) * 0.05;
        if u.uniq2_interface_flags & 1 != 0 {
            let mut eye_offset = diff_to_sph_geo(self.at, self.eye_next);
            eye_offset.r = u.uniq2_dist_target;
            let target = sph_geo_add(self.at, eye_offset);
            let mut eye = self.eye;
            lerp_ceil_vec3(target, &mut eye, 0.25, 0.25, 0.2);
            self.eye = eye;
        } else if u.uniq2_interface_flags & 2 != 0 {
            if dist_xz(self.at, self.eye_next) < u.uniq2_dist_target {
                let mut eye_offset = diff_to_sph_geo(self.at, self.eye_next);
                eye_offset.yaw = lerp_ceil_s(eye_offset.yaw, eye_at.yaw, 0.1, 0xA);
                eye_offset.r = u.uniq2_dist_target;
                eye_offset.pitch = 0;
                self.eye = sph_geo_add(self.at, eye_offset);
                self.eye.y = self.eye_next.y;
            } else {
                let mut eye = self.eye;
                lerp_ceil_vec3(self.eye_next, &mut eye, 0.25, 0.25, 0.2);
                self.eye = eye;
            }
        }
        let mut eye = self.eye;
        Self::bg_check(col, self.at, &mut eye);
        self.eye = eye;
        self.dist = self.at.distance(self.eye);
        self.roll = 0;
        self.fov = lerp_ceil_f(u.uniq2_fov_target, self.fov, 0.2, 0.1);
        self.at_lerp_step_scale = self.clamp_lerp_scale(d, 1.0);
    }

    /// `Camera_Unique3` (`DOOR0`): the bg camera's view while the door parameters' timers run,
    /// then back to the previous setting once Player is through (`unk_14C & 8`) and moves.
    fn unique3(&mut self, d: &CameraData, col: &CollisionContext, p: &PlayerView, input: &Input) {
        let player_height = p.height();
        self.unk_14c &= !0x10;
        if matches!(self.anim_state, 0 | 10 | 20) {
            let key = self.cur();
            let v = |i: usize| d.value(key, i);
            let y_normal = Self::y_normal(d, player_height);
            let u = &mut self.uniq;
            u.uniq3_y_offset = v(0) as f32 * 0.01 * player_height * y_normal;
            u.uniq3_fov = v(1) as f32;
            u.uniq3_interface_flags = v(2);
        }
        let u = self.uniq;
        self.interface_flags = u.uniq3_interface_flags;
        let mut state = self.anim_state;
        // The C's switch falls through from each case into the next.
        if state == 0 {
            self.func_80043b60(d);
            self.unk_14c &= !(0x8 | 0x4);
            self.uniq.uniq3_initial_fov = self.fov;
            self.uniq.uniq3_initial_dist = self.at.distance(self.eye);
            self.anim_state += 1;
            state = 1;
        }
        if state == 1 {
            let t = self.door_params.timer1;
            self.door_params.timer1 = t.wrapping_sub(1);
            if t > 0 {
                return;
            }
            let b = self.bg_cam_data(col);
            self.eye_next = b.pos_f();
            self.eye = self.eye_next;
            self.at = sph_geo_add(self.eye, VecSph { r: 100.0, yaw: b.rot[1], pitch: b.rot[0].wrapping_neg() });
            self.anim_state += 1;
            state = 2;
        }
        if state == 2 {
            if u.uniq3_interface_flags & 4 != 0 {
                self.at = self.player_pos + Vec3::Y * (player_height + u.uniq3_y_offset);
            }
            let t = self.door_params.timer2;
            self.door_params.timer2 = t.wrapping_sub(1);
            if t > 0 {
                return;
            }
            self.anim_state += 1;
            state = 3;
        }
        if state == 3 {
            self.unk_14c |= 0x400 | 0x10;
            if self.unk_14c & 8 == 0 {
                return;
            }
            self.anim_state += 1;
            state = 4;
        }
        if state == 4 {
            if u.uniq3_interface_flags & 2 != 0 {
                self.unk_14c |= 4;
                self.unk_14c &= !8;
                self.change_setting_flags(d, CAM_SET_PIVOT_IN_FRONT, 2);
                return;
            }
            self.door_params.timer3 = 5;
            if !(self.xz_speed > 0.001 || Self::any_button_pressed(input)) {
                return;
            }
            self.anim_state += 1;
            state = 5;
        }
        if state == 5 {
            self.fov = lerp_ceil_f(self.uniq.uniq3_initial_fov, self.fov, 0.4, 0.1);
            let mut sp60 = diff_to_sph_geo(self.at, self.eye);
            sp60.r = lerp_ceil_f(100.0, sp60.r, 0.4, 0.1);
            self.eye_next = sph_geo_add(self.at, sp60);
            self.eye = self.eye_next;
            let t = self.door_params.timer3;
            self.door_params.timer3 = t.wrapping_sub(1);
            if t > 0 {
                return;
            }
            self.anim_state += 1;
        }
        // default:
        self.unk_14c |= 4;
        self.unk_14c &= !8;
        self.fov = u.uniq3_fov;
        let prev = self.prev_setting;
        self.change_setting_flags(d, prev, 2);
        self.at_lerp_step_scale = 0.0;
        self.pos_offset = self.at - self.player_pos;
    }

    /// `Camera_Unique6` (`FREE0`): nothing moves the eye or `at` (actors set them with
    /// `Camera_SetParam`); only the distance and offset follow Player.
    fn unique6(&mut self, d: &CameraData, p: &PlayerView) {
        if matches!(self.anim_state, 0 | 10 | 20) {
            self.uniq.uniq6_interface_flags = d.value(self.cur(), 0);
        }
        self.interface_flags = self.uniq.uniq6_interface_flags;
        if self.anim_state == 0 {
            self.anim_state += 1;
            self.func_80043abc(d);
        }
        let head = self.player_pos + Vec3::Y * p.height();
        self.dist = head.distance(self.eye);
        self.pos_offset = self.at - self.player_pos;
        if self.uniq.uniq6_interface_flags & 1 != 0 && self.timer > 0 {
            self.timer -= 1;
        }
    }

    /// `Camera_Unique7` (`PREREND_PIVOT`): the eye at the bg camera's position, turning to
    /// look at Player.
    fn unique7(&mut self, d: &CameraData, col: &CollisionContext) {
        if matches!(self.anim_state, 0 | 10 | 20) {
            self.uniq.uniq7_fov = d.value(self.cur(), 0) as f32;
            self.uniq.uniq7_interface_flags = d.value(self.cur(), 1);
        }
        let b = self.bg_cam_data(col);
        self.eye_next = b.pos_f();
        self.eye = self.eye_next;
        let mut player_pos_eye_offset = diff_to_sph_geo(self.eye, self.player_pos);
        // The fov is set to 60 below whatever this gives.
        let mut fov = b.fov;
        if fov == -1 {
            fov = (self.uniq.uniq7_fov * 100.0) as i16;
        }
        if fov <= 360 {
            fov = fov.wrapping_mul(100);
        }
        self.interface_flags = self.uniq.uniq7_interface_flags;
        if self.anim_state == 0 {
            self.anim_state += 1;
            self.fov = fov as f32 * 0.01;
            self.at_lerp_step_scale = 0.0;
            self.roll = 0;
            self.uniq.uniq7_unk_00_x = player_pos_eye_offset.yaw;
        }
        self.fov = 60.0;
        // 0x7D0 ~ 10.98 degrees; rwData->unk_00.x is never read.
        self.uniq.uniq7_unk_00_x = lerp_floor_s(player_pos_eye_offset.yaw, self.uniq.uniq7_unk_00_x, 0.4, 0x7D0);
        player_pos_eye_offset.pitch = (b.rot[0].wrapping_neg() as f32 * cos_s(player_pos_eye_offset.yaw.wrapping_sub(b.rot[1]))) as i16;
        self.at = sph_geo_add(self.eye, player_pos_eye_offset);
        self.unk_14c |= 0x400;
    }

    /// `Camera_Special9` (`DOORC`): through a door. The eye jumps to one side of the door (or
    /// the bg camera) after `timer1`, follows Player through for `timer2` and `timer3`, then
    /// returns to the previous setting once Player moves or a button is pressed.
    fn special9(&mut self, d: &CameraData, col: &CollisionContext, p: &PlayerView, door: Option<(Vec3, [i16; 3])>, frames: u32, input: &Input) {
        let player_y_offset = p.height();
        self.unk_14c &= !0x10;
        let y_normal = Self::y_normal(d, player_y_offset);
        if matches!(self.anim_state, 0 | 10 | 20) {
            let key = self.cur();
            let v = |i: usize| d.value(key, i);
            let u = &mut self.uniq;
            u.spec9_y_offset = v(0) as f32 * 0.01 * player_y_offset * y_normal;
            u.spec9_unk_04 = v(1) as f32;
            u.spec9_interface_flags = v(2);
        }
        let u = self.uniq;
        // The door's position and shape rotation, else Player's raised by the offset, pitch 0.
        let (_adj_pos, adj_rot) = match door.filter(|_| self.door_params.door_actor.is_some()) {
            Some((pos, rot)) => (pos, rot),
            None => (self.player_pos + Vec3::Y * (player_y_offset + u.spec9_y_offset), [0, self.player_rot_y, 0]),
        };
        let at_eye_offset_geo = diff_to_sph_geo(self.at, self.eye);
        self.interface_flags = u.spec9_interface_flags;
        let mut state = self.anim_state;
        if state == 0 {
            self.unk_14c &= !(0x4 | 0x2);
            self.anim_state += 1;
            // ABS on the int difference of the two s16 yaws.
            self.uniq.spec9_target_yaw = if (self.player_rot_y as i32 - adj_rot[1] as i32).abs() >= 0x4000 { adj_rot[1].wrapping_sub(0x7FFF) } else { adj_rot[1] };
            state = 1;
        }
        if state == 1 {
            self.door_params.timer1 = self.door_params.timer1.wrapping_sub(1);
            if self.door_params.timer1 > 0 {
                return self.special9_end(player_y_offset);
            }
            self.anim_state += 1;
            if u.spec9_interface_flags & 1 != 0 {
                let b = self.bg_cam_data(col);
                self.eye_next = b.pos_f();
                self.eye = self.eye_next;
            } else {
                // 0xE38 ~ 20 degrees, 0xAAA ~ 15 degrees.
                let mut yaw: i16 = if frames & 1 != 0 { 0xAAA } else { -0xAAA };
                let mut eye_adjustment = VecSph { pitch: 0xE38, yaw: self.uniq.spec9_target_yaw.wrapping_add(yaw), r: 200.0 * y_normal };
                self.eye_next = sph_geo_add(self.at, eye_adjustment);
                self.eye = self.eye_next;
                if Self::check_oob(col, self.eye, self.player_pos) {
                    yaw = yaw.wrapping_neg();
                    eye_adjustment.yaw = self.uniq.spec9_target_yaw.wrapping_add(yaw);
                    self.eye_next = sph_geo_add(self.at, eye_adjustment);
                    self.eye = self.eye_next;
                }
            }
            state = 2;
        }
        if state == 2 {
            let sp_ac = self.player_pos + Vec3::Y * (player_y_offset + u.spec9_y_offset);
            let mut at = self.at;
            lerp_ceil_vec3(sp_ac, &mut at, 0.25, 0.25, 0.1);
            self.at = at;
            self.door_params.timer2 = self.door_params.timer2.wrapping_sub(1);
            if self.door_params.timer2 > 0 {
                return self.special9_end(player_y_offset);
            }
            self.anim_state += 1;
            self.uniq.spec9_target_yaw = self.uniq.spec9_target_yaw.wrapping_sub(0x7FFF);
            state = 3;
        }
        if state == 3 {
            let sp_ac = self.player_pos + Vec3::Y * (player_y_offset + u.spec9_y_offset);
            let mut at = self.at;
            lerp_ceil_vec3(sp_ac, &mut at, 0.5, 0.5, 0.1);
            self.at = at;
            let eye_adjustment = VecSph {
                pitch: lerp_ceil_s(0xAAA, at_eye_offset_geo.pitch, 0.3, 0xA),
                yaw: lerp_ceil_s(self.uniq.spec9_target_yaw, at_eye_offset_geo.yaw, 0.3, 0xA),
                r: lerp_ceil_f(60.0, at_eye_offset_geo.r, 0.3, 1.0),
            };
            self.eye_next = sph_geo_add(self.at, eye_adjustment);
            self.eye = self.eye_next;
            self.door_params.timer3 = self.door_params.timer3.wrapping_sub(1);
            if self.door_params.timer3 > 0 {
                return self.special9_end(player_y_offset);
            }
            self.anim_state += 1;
            state = 4;
        }
        if state == 4 {
            self.anim_state += 1;
        }
        // default:
        self.unk_14c |= 0x400 | 0x10;
        self.interface_flags = 0;
        if self.xz_speed > 0.001 || Self::any_button_pressed(input) || u.spec9_interface_flags & 0x8 != 0 {
            let prev = self.prev_setting;
            self.change_setting_flags(d, prev, 2);
            self.unk_14c |= 0x4 | 0x2;
        }
        self.special9_end(player_y_offset);
    }

    /// The end of `Camera_Special9`, after its switch.
    fn special9_end(&mut self, player_y_offset: f32) {
        let head = self.player_pos + Vec3::Y * player_y_offset;
        self.dist = head.distance(self.eye);
        self.pos_offset = self.at - self.player_pos;
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
