//! `sys_matrix.c`'s rotation functions on an `MtxF`, ported as written for the actors that
//! build a rotation and read it back as angles (`En_Goroiwa`'s roll). The field names are the
//! C's: `MtxF` stores `xx, yx, zx, wx, xy, ...` in that order, and a column vector `v`
//! transforms as `x' = xx·vx + xy·vy + xz·vz + xw`.

use crate::camera::f_atan2f;
use glam::Vec3;

/// `MtxF`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MtxF {
    pub xx: f32,
    pub yx: f32,
    pub zx: f32,
    pub wx: f32,
    pub xy: f32,
    pub yy: f32,
    pub zy: f32,
    pub wy: f32,
    pub xz: f32,
    pub yz: f32,
    pub zz: f32,
    pub wz: f32,
    pub xw: f32,
    pub yw: f32,
    pub zw: f32,
    pub ww: f32,
}

/// `RAD_TO_BINANG`: `(s16)(radians * (0x8000 / M_PI))`, in double precision, truncated.
pub fn rad_to_binang(radians: f32) -> i16 {
    (radians as f64 * (32768.0 / std::f64::consts::PI)) as i32 as i16
}

/// `BINANG_TO_RAD`: `(f32)binang * (M_PI / 0x8000)`, in double precision.
pub fn binang_to_rad(binang: i16) -> f32 {
    (binang as f32 as f64 * (std::f64::consts::PI / 32768.0)) as f32
}

impl MtxF {
    /// `Matrix_RotateAxis(angle, axis, MTXMODE_NEW)`: the rotation by `angle` radians around the
    /// unit vector `axis`.
    pub fn rotate_axis(angle: f32, axis: Vec3) -> MtxF {
        let mut m = MtxF::IDENTITY;
        if angle != 0.0 {
            let sin = angle.sin();
            let cos = angle.cos();
            let r_cos = 1.0 - cos;
            m.xx = axis.x * axis.x * r_cos + cos;
            m.yy = axis.y * axis.y * r_cos + cos;
            m.zz = axis.z * axis.z * r_cos + cos;
            let t2 = axis.x * r_cos * axis.y;
            let t3 = axis.z * sin;
            m.yx = t2 + t3;
            m.xy = t2 - t3;
            let t2 = axis.x * r_cos * axis.z;
            let t3 = axis.y * sin;
            m.zx = t2 - t3;
            m.xz = t2 + t3;
            let t2 = axis.y * r_cos * axis.z;
            let t3 = axis.x * sin;
            m.zy = t2 + t3;
            m.yz = t2 - t3;
        }
        m
    }

    pub const IDENTITY: MtxF = MtxF { xx: 1.0, yx: 0.0, zx: 0.0, wx: 0.0, xy: 0.0, yy: 1.0, zy: 0.0, wy: 0.0, xz: 0.0, yz: 0.0, zz: 1.0, wz: 0.0, xw: 0.0, yw: 0.0, zw: 0.0, ww: 1.0 };

    /// `Matrix_RotateX(x, MTXMODE_APPLY)`.
    pub fn rotate_x(&mut self, x: f32) {
        if x == 0.0 {
            return;
        }
        let (sin, cos) = (x.sin(), x.cos());
        let r = |a: &mut f32, b: &mut f32| {
            let (t1, t2) = (*a, *b);
            *a = t1 * cos + t2 * sin;
            *b = t2 * cos - t1 * sin;
        };
        r(&mut self.xy, &mut self.xz);
        r(&mut self.yy, &mut self.yz);
        r(&mut self.zy, &mut self.zz);
        r(&mut self.wy, &mut self.wz);
    }

    /// `Matrix_RotateY(y, MTXMODE_APPLY)`.
    pub fn rotate_y(&mut self, y: f32) {
        if y == 0.0 {
            return;
        }
        let (sin, cos) = (y.sin(), y.cos());
        let r = |a: &mut f32, b: &mut f32| {
            let (t1, t2) = (*a, *b);
            *a = t1 * cos - t2 * sin;
            *b = t1 * sin + t2 * cos;
        };
        r(&mut self.xx, &mut self.xz);
        r(&mut self.yx, &mut self.yz);
        r(&mut self.zx, &mut self.zz);
        r(&mut self.wx, &mut self.wz);
    }

    /// `Matrix_RotateZ(z, MTXMODE_APPLY)`.
    pub fn rotate_z(&mut self, z: f32) {
        if z == 0.0 {
            return;
        }
        let (sin, cos) = (z.sin(), z.cos());
        let r = |a: &mut f32, b: &mut f32| {
            let (t1, t2) = (*a, *b);
            *a = t1 * cos + t2 * sin;
            *b = t2 * cos - t1 * sin;
        };
        r(&mut self.xx, &mut self.xy);
        r(&mut self.yx, &mut self.yy);
        r(&mut self.zx, &mut self.zy);
        r(&mut self.wx, &mut self.wy);
    }

    /// `Matrix_MtxFToYXZRotS(mf, rotDest, flag)`: the rotation as Tait-Bryan YXZ angles,
    /// `[x, y, z]`.
    pub fn to_yxz_rot_s(&self, flag: bool) -> [i16; 3] {
        let mut temp = self.xz;
        temp *= temp;
        temp += self.zz * self.zz;
        let x = rad_to_binang(f_atan2f(-self.yz, temp.sqrt()));
        if x == 0x4000 || x == -0x4000 {
            return [x, rad_to_binang(f_atan2f(-self.zx, self.xx)), 0];
        }
        let y = rad_to_binang(f_atan2f(self.xz, self.zz));
        let z = if !flag {
            rad_to_binang(f_atan2f(self.yx, self.yy))
        } else {
            let mut t = self.xx;
            let mut t2 = self.zx;
            let mut t3 = self.zy;
            t *= t;
            t += t2 * t2;
            t2 = self.yx;
            t += t2 * t2;
            t = t.sqrt();
            t = t2 / t;
            let mut u = self.xy;
            u *= u;
            u += t3 * t3;
            t3 = self.yy;
            u += t3 * t3;
            u = u.sqrt();
            u = t3 / u;
            rad_to_binang(f_atan2f(t, u))
        };
        [x, y, z]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_yaw_round_trips_through_yxz_angles() {
        // Matrix_RotateY(yaw) read back as angles.
        let mut m = MtxF::IDENTITY;
        m.rotate_y(binang_to_rad(0x2000));
        let r = m.to_yxz_rot_s(false);
        assert_eq!(r[0], 0);
        assert!((r[1] as i32 - 0x2000).abs() <= 1, "{r:?}");
        assert_eq!(r[2], 0);
    }

    #[test]
    fn a_rotation_about_x_is_a_pitch() {
        // Around the x axis by a quarter turn less a little: x = the angle, no yaw or roll.
        let a = binang_to_rad(0x3000);
        let m = MtxF::rotate_axis(a, Vec3::X);
        let r = m.to_yxz_rot_s(false);
        assert!((r[0] as i32 - 0x3000).abs() <= 1, "{r:?}");
        assert_eq!((r[1], r[2]), (0, 0));
    }

    #[test]
    fn rad_to_binang_truncates() {
        // 0x8000 / M_PI * 0.001 = 10.43: (s16) truncates to 10.
        assert_eq!(rad_to_binang(0.001), 10);
        assert_eq!(rad_to_binang(-0.001), -10);
    }
}
