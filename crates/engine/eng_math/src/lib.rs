//! Fixed-point angle maths and the step helpers the game logic is built on.
//!
//! - `sins`/`coss` (libultra `gu/sins.c`, `gu/coss.c`) with the 1024-entry `sintable`
//! - `Math_Atan2S` (`sys_math_atan.c`) with the 1025-entry `sAtan2Tbl`
//! - `Math_StepToF`, `Math_AsymStepToF`, `Math_ScaledStepToS`, ... (`z_lib.c`)
//! - `guPerspective` (libultra `gu/perspective.c`)
//!
//! The tables are installed by the caller (`install`): the importer reads them from the
//! decomp source (`oot_import::tables`). Without that, they are generated from the formulas
//! they match; `Tables::compare` checks the two agree.

use std::sync::OnceLock;

use glam::Mat4;

/// `R_UPDATE_RATE` during gameplay (`SREG(30) = 3`, set in `z_play.c`): the game runs one
/// logic frame per three 60 Hz vertical retraces.
pub const R_UPDATE_RATE: i32 = 3;
/// `R_UPDATE_RATE * 0.5f`, the scale several z_lib and SkelAnime functions apply.
pub const UPDATE_SCALE: f32 = R_UPDATE_RATE as f32 * 0.5;
/// Game logic rate in Hz.
pub const GAME_HZ: f32 = 60.0 / R_UPDATE_RATE as f32;

/// `SHT_MINV` (`libc/math.h`).
pub const SHT_MINV: f32 = 1.0 / 32767.0;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Tables {
    pub sin: Vec<i16>,
    pub atan: Vec<u16>,
    pub from_decomp: bool,
}

static TABLES: OnceLock<Tables> = OnceLock::new();

impl Tables {
    /// The formulas the shipped tables match: `trunc(sin(i·π/2046)·32767)` (entry 1023 is
    /// exactly 90°, so `sins` runs 1023/1024 slow) and `round(atan(i/1024)·0x8000/π)`.
    pub fn computed() -> Tables {
        let sin = (0..0x400).map(|i| ((i as f64 * std::f64::consts::PI / 2046.0).sin() * 32767.0) as i16).collect();
        let atan = (0..=0x400)
            .map(|i| ((i as f64 / 1024.0).atan() * 32768.0 / std::f64::consts::PI).round() as u16)
            .collect();
        Tables { sin, atan, from_decomp: false }
    }

    /// Number of entries that differ between two table sets (sin, atan) and the largest
    /// difference seen.
    pub fn compare(&self, other: &Tables) -> (usize, usize, i64) {
        let d = |a: &[i64], b: &[i64]| a.iter().zip(b).filter(|(x, y)| x != y).count();
        let m = |a: &[i64], b: &[i64]| a.iter().zip(b).map(|(x, y)| (x - y).abs()).max().unwrap_or(0);
        let s = |t: &Tables| t.sin.iter().map(|&v| v as i64).collect::<Vec<_>>();
        let a = |t: &Tables| t.atan.iter().map(|&v| v as i64).collect::<Vec<_>>();
        let (ss, so, aa, ao) = (s(self), s(other), a(self), a(other));
        (d(&ss, &so), d(&aa, &ao), m(&ss, &so).max(m(&aa, &ao)))
    }
}

/// Installs the tables used by the free functions below. The first call wins; later calls
/// are ignored. Functions used before any call fall back to `Tables::computed()`.
pub fn install(t: Tables) {
    let _ = TABLES.set(t);
}

pub fn tables() -> &'static Tables {
    TABLES.get_or_init(Tables::computed)
}

/// libultra `sins`.
pub fn sins(x: u16) -> i16 {
    let t = &tables().sin;
    let x = x >> 4;
    let value = if x & 0x400 != 0 { t[0x3FF - (x & 0x3FF) as usize] } else { t[(x & 0x3FF) as usize] };
    if x & 0x800 != 0 { -value } else { value }
}

/// libultra `coss`.
pub fn coss(x: u16) -> i16 {
    sins(x.wrapping_add(0x4000))
}

/// `Math_SinS`.
pub fn sin_s(angle: i16) -> f32 {
    sins(angle as u16) as f32 * SHT_MINV
}

/// `Math_CosS`.
pub fn cos_s(angle: i16) -> f32 {
    coss(angle as u16) as f32 * SHT_MINV
}

fn atan2_tbl(x: f32, y: f32) -> u16 {
    let t = &tables().atan;
    if y == 0.0 {
        t[0]
    } else {
        let idx = ((x / y) * 1024.0 + 0.5) as i32;
        if idx < 0 || idx as usize >= t.len() { t[0] } else { t[idx as usize] }
    }
}

/// `Math_Atan2S(x, y)`. Note the argument order: this is the conventional atan2(y, x).
/// Yaw in the game is `Math_Atan2S(dz, dx)`: 0 faces +z, 0x4000 faces +x.
pub fn atan2_s(x: f32, y: f32) -> i16 {
    let ret: i32 = if y >= 0.0 {
        if x >= 0.0 {
            if y <= x { atan2_tbl(y, x) as i32 } else { 0x4000 - atan2_tbl(x, y) as i32 }
        } else if -x < y {
            atan2_tbl(-x, y) as i32 + 0x4000
        } else {
            0x8000 - atan2_tbl(y, -x) as i32
        }
    } else if x < 0.0 {
        if -y <= -x { atan2_tbl(-y, -x) as i32 + 0x8000 } else { 0xC000 - atan2_tbl(-x, -y) as i32 }
    } else if x < -y {
        atan2_tbl(x, -y) as i32 + 0xC000
    } else {
        -(atan2_tbl(-y, x) as i32)
    };
    ret as i16
}

/// Yaw from `from` to `to` (`Math_Vec3f_Yaw`).
pub fn vec3f_yaw(from: glam::Vec3, to: glam::Vec3) -> i16 {
    atan2_s(to.z - from.z, to.x - from.x)
}

/// `Math_SmoothStepToF` (`z_lib.c`): step `v` towards `target` by `fraction` of the gap,
/// clamped to `step`, at least `min_step`. Returns the gap left.
///
/// @bug (game) Below `min_step`, both of the C's small-step branches can run, and the second
/// undoes the first: kept as written.
pub fn smooth_step_to_f(v: &mut f32, target: f32, fraction: f32, step: f32, min_step: f32) -> f32 {
    if *v != target {
        let mut step_size = (target - *v) * fraction;
        if step_size >= min_step || step_size <= -min_step {
            step_size = step_size.clamp(-step, step);
            *v += step_size;
        } else {
            if step_size < min_step {
                *v += min_step;
                step_size = min_step;
                if target < *v {
                    *v = target;
                }
            }
            if step_size > -min_step {
                *v += -min_step;
                if *v < target {
                    *v = target;
                }
            }
        }
    }
    (target - *v).abs()
}

/// `Math_StepToF`.
pub fn step_to_f(v: &mut f32, target: f32, step: f32) -> bool {
    if step != 0.0 {
        let step = if target < *v { -step } else { step };
        *v += step;
        if (*v - target) * step >= 0.0 {
            *v = target;
            return true;
        }
    } else if target == *v {
        return true;
    }
    false
}

/// `Math_StepToS`.
pub fn step_to_s(v: &mut i16, target: i16, step: i16) -> bool {
    if step != 0 {
        let step = if target < *v { step.wrapping_neg() } else { step };
        *v = v.wrapping_add(step);
        if (*v as i32 - target as i32) * step as i32 >= 0 {
            *v = target;
            return true;
        }
    } else if target == *v {
        return true;
    }
    false
}

/// `Math_AsymStepToF`: `incr` when rising towards the target, `decr` when falling.
pub fn asym_step_to_f(v: &mut f32, target: f32, incr: f32, decr: f32) -> bool {
    let step = if target >= *v { incr } else { decr };
    if step != 0.0 {
        let step = if target < *v { -step } else { step };
        *v += step;
        if (*v - target) * step >= 0.0 {
            *v = target;
            return true;
        }
    } else if target == *v {
        return true;
    }
    false
}

/// `Math_ScaledStepToS`: steps an angle towards `target` by `step * R_UPDATE_RATE * 0.5`.
pub fn scaled_step_to_s(v: &mut i16, target: i16, step: i16) -> bool {
    if step != 0 {
        let step = if v.wrapping_sub(target) > 0 { step.wrapping_neg() } else { step };
        *v = v.wrapping_add((step as f32 * UPDATE_SCALE) as i16);
        if (v.wrapping_sub(target) as i32) * (step as i32) >= 0 {
            *v = target;
            return true;
        }
    } else if target == *v {
        return true;
    }
    false
}

/// `Math_SmoothStepToS`.
pub fn smooth_step_to_s(v: &mut i16, target: i16, scale: i16, step: i16, min_step: i16) -> i16 {
    let diff = target.wrapping_sub(*v);
    let mut step_size = (diff as i32 / scale as i32) as i16;
    if *v != target {
        if step_size > min_step || step_size < -min_step {
            step_size = step_size.clamp(-step, step);
            *v = v.wrapping_add(step_size);
        } else if diff >= 0 {
            *v = v.wrapping_add(min_step);
            if target.wrapping_sub(*v) <= 0 {
                *v = target;
            }
        } else {
            *v = v.wrapping_sub(min_step);
            if target.wrapping_sub(*v) >= 0 {
                *v = target;
            }
        }
    }
    diff
}

/// `Math_ApproachS`: a `scale`-th of the way to `target`, at most `step`.
pub fn approach_s(v: &mut i16, target: i16, scale: i16, step: i16) {
    let diff = (target.wrapping_sub(*v) as i32 / scale as i32) as i16;
    if diff > step {
        *v = v.wrapping_add(step);
    } else if (diff as i32) < -(step as i32) {
        *v = v.wrapping_sub(step);
    } else {
        *v = v.wrapping_add(diff);
    }
}

/// `Math_ApproachF`: `fraction` of the way to `target`, at most `step`.
pub fn approach_f(v: &mut f32, target: f32, fraction: f32, step: f32) {
    if *v != target {
        let mut s = (target - *v) * fraction;
        if s > step {
            s = step;
        } else if s < -step {
            s = -step;
        }
        *v += s;
    }
}

/// `Math_ApproachZeroF`.
pub fn approach_zero_f(v: &mut f32, fraction: f32, step: f32) {
    let mut s = *v * fraction;
    if s > step {
        s = step;
    } else if s < -step {
        s = -step;
    }
    *v -= s;
}

/// Converts a C `f32 → s16` cast (truncation towards zero).
pub fn f2s(v: f32) -> i16 {
    v as i32 as i16
}

pub fn binang_to_rad(a: i16) -> f32 {
    a as f32 * (std::f32::consts::PI / 32768.0)
}

pub fn rad_to_binang(r: f32) -> i16 {
    (r * (32768.0 / std::f32::consts::PI)).round() as i32 as i16
}

/// `IS_ZERO` (`z_math.h`).
pub fn is_zero(f: f32) -> bool {
    f.abs() < 0.008
}

/// `guPerspective` (libultra `gu/perspective.c`) as a column-vector matrix, for clip-space z
/// the way the game computes it (OpenGL-style depth, -w..w).
pub fn gu_perspective(fovy_deg: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let cot = 1.0 / (fovy_deg.to_radians() / 2.0).tan();
    Mat4::from_cols_array(&[
        cot / aspect, 0.0, 0.0, 0.0,
        0.0, cot, 0.0, 0.0,
        0.0, 0.0, (near + far) / (near - far), -1.0,
        0.0, 0.0, 2.0 * near * far / (near - far), 0.0,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn angles_follow_the_game_convention() {
        assert_eq!(atan2_s(1.0, 0.0), 0); // +z
        assert_eq!(atan2_s(0.0, 1.0), 0x4000); // +x
        assert_eq!(atan2_s(1.0, 1.0), 0x2000);
        assert_eq!(atan2_s(-1.0, 0.0), -0x8000);
        assert!((sin_s(0x4000) - 1.0).abs() < 1e-4);
        assert!((cos_s(0) - 1.0).abs() < 1e-6);
        assert!(cos_s(-0x8000) < -0.999);
    }

    #[test]
    fn scaled_step_uses_update_rate() {
        let mut a = 0i16;
        assert!(!scaled_step_to_s(&mut a, 10000, 1000));
        assert_eq!(a, 1500);
        let mut b = 0i16;
        assert!(scaled_step_to_s(&mut b, -500, 1000));
        assert_eq!(b, -500);
    }

    #[test]
    fn asym_step() {
        let mut v = 0.0;
        asym_step_to_f(&mut v, 6.0, 2.0, 1.5);
        assert_eq!(v, 2.0);
        asym_step_to_f(&mut v, 0.0, 2.0, 1.5);
        assert_eq!(v, 0.5);
    }
}
