//! `sys_matrix.c`'s functions on an `MtxF`, ported as written for the actors whose game state
//! depends on a matrix: `En_Goroiwa`'s roll (built and read back as angles), `En_Dekubaba`'s
//! collider spheres (placed by the matrices its draw builds). The field names are the C's:
//! `MtxF` stores `xx, yx, zx, wx, xy, ...` in that order, and a column vector `v` transforms as
//! `x' = xx·vx + xy·vy + xz·vz + xw`. The `s16` angles go through `Math_SinS` and `Math_CosS`.

use crate::camera::f_atan2f;
use eng_math::{cos_s, sin_s};
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

    /// `Matrix_Translate(x, y, z, MTXMODE_NEW)` (`SkinMatrix_SetTranslate`).
    pub fn set_translate(x: f32, y: f32, z: f32) -> MtxF {
        MtxF { xw: x, yw: y, zw: z, ..MtxF::IDENTITY }
    }

    /// `Matrix_Translate(x, y, z, MTXMODE_APPLY)`.
    pub fn translate(&mut self, x: f32, y: f32, z: f32) {
        self.xw += self.xx * x + self.xy * y + self.xz * z;
        self.yw += self.yx * x + self.yy * y + self.yz * z;
        self.zw += self.zx * x + self.zy * y + self.zz * z;
        self.ww += self.wx * x + self.wy * y + self.wz * z;
    }

    /// `Matrix_Scale(x, y, z, MTXMODE_APPLY)`.
    pub fn scale(&mut self, x: f32, y: f32, z: f32) {
        self.xx *= x;
        self.yx *= x;
        self.zx *= x;
        self.xy *= y;
        self.yy *= y;
        self.zy *= y;
        self.xz *= z;
        self.yz *= z;
        self.zz *= z;
        self.wx *= x;
        self.wy *= y;
        self.wz *= z;
    }

    /// The rotations `Matrix_RotateZYX` and `Matrix_TranslateRotateZYX` apply: about z (always,
    /// even by 0), then y, then x (each skipped at 0).
    fn rotate_zyx_parts(&mut self, x: i16, y: i16, z: i16) {
        let (sin, cos) = (sin_s(z), cos_s(z));
        let rz = |a: &mut f32, b: &mut f32| {
            let (t1, t2) = (*a, *b);
            *a = t1 * cos + t2 * sin;
            *b = t2 * cos - t1 * sin;
        };
        rz(&mut self.xx, &mut self.xy);
        rz(&mut self.yx, &mut self.yy);
        rz(&mut self.zx, &mut self.zy);
        rz(&mut self.wx, &mut self.wy);
        if y != 0 {
            let (sin, cos) = (sin_s(y), cos_s(y));
            let ry = |a: &mut f32, b: &mut f32| {
                let (t1, t2) = (*a, *b);
                *a = t1 * cos - t2 * sin;
                *b = t1 * sin + t2 * cos;
            };
            ry(&mut self.xx, &mut self.xz);
            ry(&mut self.yx, &mut self.yz);
            ry(&mut self.zx, &mut self.zz);
            ry(&mut self.wx, &mut self.wz);
        }
        if x != 0 {
            let (sin, cos) = (sin_s(x), cos_s(x));
            let rx = |a: &mut f32, b: &mut f32| {
                let (t1, t2) = (*a, *b);
                *a = t1 * cos + t2 * sin;
                *b = t2 * cos - t1 * sin;
            };
            rx(&mut self.xy, &mut self.xz);
            rx(&mut self.yy, &mut self.yz);
            rx(&mut self.zy, &mut self.zz);
            rx(&mut self.wy, &mut self.wz);
        }
    }

    /// `Matrix_RotateZYX(x, y, z, MTXMODE_APPLY)`.
    pub fn rotate_zyx(&mut self, x: i16, y: i16, z: i16) {
        self.rotate_zyx_parts(x, y, z);
    }

    /// `Matrix_TranslateRotateZYX(&translation, &rotation)`: a limb's place in its parent (the
    /// translation in the current frame, then the rotation).
    pub fn translate_rotate_zyx(&mut self, t: Vec3, r: [i16; 3]) {
        self.xw += self.xx * t.x + self.xy * t.y + self.xz * t.z;
        self.yw += self.yx * t.x + self.yy * t.y + self.yz * t.z;
        self.zw += self.zx * t.x + self.zy * t.y + self.zz * t.z;
        self.ww += self.wx * t.x + self.wy * t.y + self.wz * t.z;
        self.rotate_zyx_parts(r[0], r[1], r[2]);
    }

    /// `Matrix_SetTranslateRotateYXZ(tx, ty, tz, &rot)`: `Actor_Draw`'s model matrix before its
    /// scale.
    pub fn set_translate_rotate_yxz(tx: f32, ty: f32, tz: f32, rot: [i16; 3]) -> MtxF {
        let mut m = MtxF::IDENTITY;
        let (temp1, temp2) = (sin_s(rot[1]), cos_s(rot[1]));
        m.xx = temp2;
        m.zx = -temp1;
        m.xw = tx;
        m.yw = ty;
        m.zw = tz;
        m.wx = 0.0;
        m.wy = 0.0;
        m.wz = 0.0;
        m.ww = 1.0;
        if rot[0] != 0 {
            let (sin, cos) = (sin_s(rot[0]), cos_s(rot[0]));
            m.zz = temp2 * cos;
            m.zy = temp2 * sin;
            m.xz = temp1 * cos;
            m.xy = temp1 * sin;
            m.yz = -sin;
            m.yy = cos;
        } else {
            m.zz = temp2;
            m.xz = temp1;
            m.yz = 0.0;
            m.zy = 0.0;
            m.xy = 0.0;
            m.yy = 1.0;
        }
        if rot[2] != 0 {
            let (sin, cos) = (sin_s(rot[2]), cos_s(rot[2]));
            let (t1, t2) = (m.xx, m.xy);
            m.xx = t1 * cos + t2 * sin;
            m.xy = t2 * cos - t1 * sin;
            let (t1, t2) = (m.zx, m.zy);
            m.zx = t1 * cos + t2 * sin;
            m.zy = t2 * cos - t1 * sin;
            let t2 = m.yy;
            m.yx = t2 * sin;
            m.yy = t2 * cos;
        } else {
            m.yx = 0.0;
        }
        m
    }

    /// `Matrix_MultVec3f`.
    pub fn mult_vec3f(&self, v: Vec3) -> Vec3 {
        Vec3::new(
            self.xw + (self.xx * v.x + self.xy * v.y + self.xz * v.z),
            self.yw + (self.yx * v.x + self.yy * v.y + self.yz * v.z),
            self.zw + (self.zx * v.x + self.zy * v.y + self.zz * v.z),
        )
    }

    /// From glam's (column-major: `x_axis` is `(xx, yx, zx, wx)`).
    pub fn from_mat4(m: glam::Mat4) -> MtxF {
        let [xx, yx, zx, wx, xy, yy, zy, wy, xz, yz, zz, wz, xw, yw, zw, ww] = m.to_cols_array();
        MtxF { xx, yx, zx, wx, xy, yy, zy, wy, xz, yz, zz, wz, xw, yw, zw, ww }
    }

    /// The matrix as glam's (column-major: `x_axis` is `(xx, yx, zx, wx)`).
    pub fn to_mat4(&self) -> glam::Mat4 {
        glam::Mat4::from_cols(
            glam::Vec4::new(self.xx, self.yx, self.zx, self.wx),
            glam::Vec4::new(self.xy, self.yy, self.zy, self.wy),
            glam::Vec4::new(self.xz, self.yz, self.zz, self.wz),
            glam::Vec4::new(self.xw, self.yw, self.zw, self.ww),
        )
    }

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
