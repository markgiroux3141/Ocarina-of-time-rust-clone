//! `z_path.c`: a scene path by index, an actor turned towards one of its points, its last point.

use glam::Vec3;

use crate::actor::Actor;
use crate::scene::Path;

/// `Path_GetByIndex`: the scene's path `index`, unless it's `max` (the params' "no path").
pub fn path_get_by_index(paths: &[Path], index: i16, max: i16) -> Option<usize> {
    if index != max && index >= 0 && (index as usize) < paths.len() {
        return Some(index as usize);
    }
    None
}

/// `RAD_TO_BINANG` (`z_math.h`): `(s16)(s32)(radians * (0x8000 / M_PI))`, the product a double.
pub fn rad_to_binang_trunc(radians: f32) -> i16 {
    (radians as f64 * (32768.0 / std::f64::consts::PI)) as i32 as i16
}

/// `Path_OrientAndGetDistSq`: the yaw from the actor to the path's `waypoint` (`Math_FAtan2F`) and
/// the squared distance to it across the ground; -1 without a path.
pub fn path_orient_and_get_dist_sq(actor: &Actor, path: Option<&Path>, waypoint: i16) -> (f32, Option<i16>) {
    let Some(path) = path else { return (-1.0, None) };
    let p = path.point(waypoint as usize);
    let dx = p.x - actor.world_pos.x;
    let dz = p.z - actor.world_pos.z;
    let yaw = rad_to_binang_trunc(crate::camera::f_atan2f(dx, dz));
    (dx * dx + dz * dz, Some(yaw))
}

/// `Path_CopyLastPoint`.
pub fn path_copy_last_point(path: Option<&Path>, dest: &mut Vec3) {
    if let Some(path) = path {
        *dest = path.point(path.count() - 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_yaw_truncates_towards_zero() {
        // atan2(1, 1) = PI / 4: 0x2000 exactly; a hair under it truncates down.
        assert_eq!(rad_to_binang_trunc(std::f32::consts::FRAC_PI_4), 0x2000);
        assert_eq!(rad_to_binang_trunc(-0.000_01), 0);
        assert_eq!(rad_to_binang_trunc(0.000_2), 2);
    }

    #[test]
    fn no_path_is_minus_one() {
        let a = Actor::new(Vec3::ZERO, 0);
        assert_eq!(path_orient_and_get_dist_sq(&a, None, 0), (-1.0, None));
        let p = Path { points: vec![[30, 0, 40]] };
        let (d, yaw) = path_orient_and_get_dist_sq(&a, Some(&p), 0);
        assert_eq!(d, 2500.0);
        assert!(yaw.is_some());
        assert_eq!(path_get_by_index(&[p.clone()], 15, 15), None);
        assert_eq!(path_get_by_index(&[p], 0, 15), Some(0));
    }
}
