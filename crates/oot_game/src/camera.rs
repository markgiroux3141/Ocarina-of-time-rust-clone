//! The game camera from `z_camera.c`: `Camera_Init`, `Camera_InitPlayerSettings`, the
//! player-following part of `Camera_Update`, and `Camera_Normal1` for `CAM_SET_NORMAL0` /
//! `CAM_MODE_NORMAL`, with the helpers it uses (`Camera_CalcAtDefault`, `Camera_ClampDist`,
//! `Camera_CalcDefaultYaw`/`Pitch`, `Camera_GetPitchAdjFromFloorHeightDiffs`, the swing
//! `func_80046E20` / `func_80045508`) and the camera bgcheck (`Camera_BGCheckInfo`,
//! `Camera_BGCheckCorner`, `Camera_GetFloorYLayer`). Vector-sphere maths is `z_olib.c`;
//! `Math_FAtan2F` is the Taylor-series version from `code_800FCE80.c`.
//!
//! `OREG` values (`sOREGInit`) and the NORMAL0 mode data (`sSetNormal0ModeNormalData`) are read
//! from `z_camera_data.c` at runtime.
//!
//! Not modelled: other settings and modes (targeting, jumping, climbing...), bg-camera
//! setting changes from the floor poly (`Camera_ChangeBgCamIndex`), water and hot-room checks,
//! quakes, the low-health wiggle, the debug camera.

use std::path::Path;

use anyhow::{Context, Result};
use glam::Vec3;
use oot_core::csrc::{find_initializer, strip_comments};

use crate::bgcheck::{self, PolyId, StaticCollision};
use crate::math::{cos_s, is_zero, sin_s};

/// `BGCHECK_SCENE`: the bgId of static collision.
const BGCHECK_SCENE: i32 = 50;

/// Values read from `z_camera_data.c`.
#[derive(Debug, Clone)]
pub struct CameraData {
    /// `sOREGInit`: `OREG(0)`..
    pub oreg: Vec<i16>,
    /// `sSetNormal0ModeNormalData`: `CAM_FUNCDATA_NORM1(yOffset, eyeDist, eyeDistNext,
    /// pitchTarget, yawUpdateRateTarget, xzUpdateRateTarget, maxYawUpdate, fov, atLerpStepScale, flags)`.
    pub normal0: [i16; 10],
}

impl CameraData {
    pub fn load(decomp: &Path) -> Result<CameraData> {
        let p = decomp.join("src/code/z_camera_data.c");
        let src = strip_comments(&std::fs::read_to_string(&p).with_context(|| format!("reading {}", p.display()))?);
        let oreg = find_initializer(&src, "sOREGInit")?
            .list()
            .iter()
            .map(|i| i.as_int().map(|v| v as i16))
            .collect::<Option<Vec<_>>>()
            .context("sOREGInit: non-integer entry")?;
        let norm = find_initializer(&src, "sSetNormal0ModeNormalData")?;
        let call = norm.flatten().join(",");
        let args = call
            .strip_prefix("CAM_FUNCDATA_NORM1(")
            .and_then(|s| s.strip_suffix(')'))
            .context("sSetNormal0ModeNormalData is not a CAM_FUNCDATA_NORM1")?;
        let v: Vec<i16> = args
            .split(',')
            .map(|a| {
                let a = a.trim();
                match a.strip_prefix("0x") {
                    Some(h) => i16::from_str_radix(h, 16).ok(),
                    None => a.parse().ok(),
                }
            })
            .collect::<Option<_>>()
            .context("CAM_FUNCDATA_NORM1 arguments")?;
        let normal0 = v.try_into().map_err(|_| anyhow::anyhow!("CAM_FUNCDATA_NORM1 wants 10 arguments"))?;
        Ok(CameraData { oreg, normal0 })
    }

    pub fn oreg(&self, n: usize) -> i16 {
        self.oreg.get(n).copied().unwrap_or(0)
    }
    /// `CAM_DATA_SCALED(OREG(n))`.
    fn oreg_s(&self, n: usize) -> f32 {
        self.oreg(n) as f32 * 0.01
    }
}

// Named OREGs (regs.h).
const R_CAM_MAX_PITCH: usize = 5;
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
    ro: Norm1Ro,
    rw: Norm1Rw,
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
            ro: Norm1Ro::default(),
            rw: Norm1Rw::default(),
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

    /// `Camera_Update` for the main camera following Player. `frames` is
    /// `play->state.frames`.
    pub fn update(&mut self, d: &CameraData, col: &StaticCollision, p: &PlayerView, frames: u32) {
        self.update_direction = false;
        let cur = p.pos;
        self.xz_speed = dist_xz(cur, self.player_pos);
        // func_8002DCE4: R_RUN_SPEED_LIMIT / 100 when not riding or swimming.
        self.speed_ratio = clamp_max_dist(self.xz_speed / ((p.run_speed_limit as f32 / 100.0) * d.oreg_s(8)), 1.0);
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
            self.normal1(d, col, p, frames);
        } else {
            let e = diff_to_sph_geo(self.at, self.eye);
            self.calc_at_default(d, &e, 0.0, false, p);
        }

        let angle = diff_to_sph_geo(self.eye, self.at);
        self.up = calc_up(angle.pitch, angle.yaw, self.roll);
        if !self.update_direction {
            self.input_dir = [angle.pitch, angle.yaw, 0];
        }
    }

    // ---- bgcheck ----------------------------------------------------------------------

    /// `Camera_BGCheckInfo`: returns 0 when nothing is between `from` and `to.pos`.
    fn bg_check_info(col: &StaticCollision, from: Vec3, to: &mut ColChk) -> i32 {
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
    fn bg_check(col: &StaticCollision, from: Vec3, to: &mut Vec3) -> bool {
        let mut c = ColChk { pos: *to, ..Default::default() };
        let r = Self::bg_check_info(col, from, &mut c);
        *to = c.pos;
        r != 0
    }

    /// `Camera_GetFloorYLayer`.
    fn floor_y_layer(&self, col: &StaticCollision, pos: &mut Vec3) -> f32 {
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
    fn bg_check_corner(col: &StaticCollision, a: Vec3, b: Vec3, ca: &ColChk, cb: &ColChk) -> Vec3 {
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
    fn pitch_adj_from_floor(&mut self, d: &CameraData, col: &StaticCollision, p: &PlayerView, view_yaw: i16, init: bool, frames: u32) -> i16 {
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
    fn col_between(&mut self, col: &StaticCollision, diff: &VecSph, check_eye: bool) -> i32 {
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
    fn swing(&mut self, d: &CameraData, col: &StaticCollision, adj: &VecSph, min_dist: f32, arg3: f32, arg4: &mut f32) {
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

    /// `Camera_Normal1` for NORMAL0 / NORMAL mode.
    fn normal1(&mut self, d: &CameraData, col: &StaticCollision, p: &PlayerView, frames: u32) {
        let rate = 0.1f32;
        let player_height = p.height();
        // RELOAD_PARAMS: animState 0, 10 or 20.
        if matches!(self.anim_state, 0 | 10 | 20) {
            let v = d.normal0.map(|x| x as f32);
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
                interface_flags: d.normal0[9],
            };
        }
        let ro = self.ro;
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
