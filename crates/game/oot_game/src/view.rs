//! `z_view.c`'s distortion: `play->view`'s projection scaled (and turned) by an amount the camera
//! (`Camera_UpdateDistortion`: under water, a hot room) or a scene's draw config (Jabu-Jabu's
//! breathing) sets, eased towards at `distortionSpeed` each frame `View_ApplyPerspective` runs
//! (`View_StepDistortion`, once per `Play_Draw`).
//!
//! The rest of `View` (the viewport, the eye and at, the perspective) is the camera's and the
//! renderer's (`crate::play::PlayState::camera_view_proj`).

use glam::{Mat4, Vec3};

/// `View`'s distortion fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewDistortion {
    /// `distortionOrientation`, `distortionScale`: the target.
    pub orientation: Vec3,
    pub scale: Vec3,
    /// `curDistortionOrientation`, `curDistortionScale`: where it is.
    pub cur_orientation: Vec3,
    pub cur_scale: Vec3,
    /// `distortionSpeed`: 0 none (the projection as it is), 1 at once.
    pub speed: f32,
    /// Whether the last `View_StepDistortion` changed the projection (`distortionSpeed` wasn't 0).
    pub applied: bool,
}

impl Default for ViewDistortion {
    fn default() -> ViewDistortion {
        ViewDistortion::init()
    }
}

/// `F32_LERPIMP(v, target, t)`: `v + (target - v) * t`.
fn lerp_imp(v: f32, target: f32, t: f32) -> f32 {
    v + (target - v) * t
}

impl ViewDistortion {
    /// `View_InitDistortion`: no turn, unit scale, speed 0.
    pub fn init() -> ViewDistortion {
        ViewDistortion { orientation: Vec3::ZERO, scale: Vec3::ONE, cur_orientation: Vec3::ZERO, cur_scale: Vec3::ONE, speed: 0.0, applied: false }
    }

    /// `View_SetDistortionOrientation`.
    pub fn set_orientation(&mut self, rot: Vec3) {
        self.orientation = rot;
    }

    /// `View_SetDistortionScale`.
    pub fn set_scale(&mut self, scale: Vec3) {
        self.scale = scale;
    }

    /// `View_SetDistortionSpeed`.
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }

    /// `View_SetDistortion`.
    pub fn set(&mut self, orientation: Vec3, scale: Vec3, speed: f32) {
        self.orientation = orientation;
        self.scale = scale;
        self.speed = speed;
    }

    /// `View_ClearDistortion`: back to none at once (speed 1: the next step lands there, then
    /// stops).
    pub fn clear(&mut self) {
        self.orientation = Vec3::ZERO;
        self.scale = Vec3::ONE;
        self.speed = 1.0;
    }

    /// `View_StepDistortion`: with speed 0, nothing (the projection undistorted); 1, the target at
    /// once and the speed back to 0; else eased towards the target by the speed. Returns whether
    /// the projection is distorted this frame.
    pub fn step(&mut self) -> bool {
        if self.speed == 0.0 {
            self.applied = false;
            return false;
        } else if self.speed == 1.0 {
            self.cur_orientation = self.orientation;
            self.cur_scale = self.scale;
            self.speed = 0.0;
        } else {
            let t = self.speed;
            self.cur_orientation = Vec3::new(lerp_imp(self.cur_orientation.x, self.orientation.x, t), lerp_imp(self.cur_orientation.y, self.orientation.y, t), lerp_imp(self.cur_orientation.z, self.orientation.z, t));
            self.cur_scale = Vec3::new(lerp_imp(self.cur_scale.x, self.scale.x, t), lerp_imp(self.cur_scale.y, self.scale.y, t), lerp_imp(self.cur_scale.z, self.scale.z, t));
        }
        self.applied = true;
        true
    }

    /// What `View_StepDistortion` multiplies the projection by (on the right): turned by the
    /// orientation (x, y, z, radians), scaled, turned back (z, y, x); the identity when this
    /// frame's step didn't apply.
    pub fn matrix(&self) -> Mat4 {
        if !self.applied {
            return Mat4::IDENTITY;
        }
        let o = self.cur_orientation;
        let r = Mat4::from_rotation_x(o.x) * Mat4::from_rotation_y(o.y) * Mat4::from_rotation_z(o.z);
        let back = Mat4::from_rotation_z(-o.z) * Mat4::from_rotation_y(-o.y) * Mat4::from_rotation_x(-o.x);
        r * Mat4::from_scale(self.cur_scale) * back
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_step_eases_to_the_target_and_speed_1_lands_at_once() {
        let mut v = ViewDistortion::init();
        assert!(!v.step());
        assert_eq!(v.matrix(), Mat4::IDENTITY);
        v.set_scale(Vec3::new(1.09, 0.91, 1.0));
        v.set_speed(0.5);
        assert!(v.step());
        assert!((v.cur_scale.x - 1.045).abs() < 1e-6 && (v.cur_scale.y - 0.955).abs() < 1e-6);
        let m = v.matrix();
        assert!((m.x_axis.x - 1.045).abs() < 1e-6 && (m.y_axis.y - 0.955).abs() < 1e-6);
        // View_ClearDistortion: one step to unit scale, then off.
        v.clear();
        assert!(v.step());
        assert_eq!(v.cur_scale, Vec3::ONE);
        assert_eq!(v.speed, 0.0);
        assert!(!v.step());
        assert_eq!(v.matrix(), Mat4::IDENTITY);
    }
}
