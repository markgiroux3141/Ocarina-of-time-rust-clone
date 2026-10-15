//! `z_horse.c`'s helpers that the horses' overlays call. Only `Horse_RotateToPoint` so far: the
//! rest (`Horse_CanSpawn`, `Horse_SetupInGameplay`, `Horse_InitPlayerHorse` and the like) are the
//! adult's Epona's (BACKLOG #51).

use glam::Vec3;

use crate::actor::Actor;
use crate::target::yaw_to;

/// `Horse_RotateToPoint`: turns the actor's `world.rot.y` towards `pos` by at most `turn_amount`
/// (straight to it when nearer), and its shape with it.
pub fn horse_rotate_to_point(actor: &mut Actor, pos: Vec3, turn_amount: i16) {
    let x = yaw_to(actor.world_pos, pos).wrapping_sub(actor.world_rot.y);
    if x > turn_amount {
        actor.world_rot.y = actor.world_rot.y.wrapping_add(turn_amount);
    } else if x < turn_amount.wrapping_neg() {
        actor.world_rot.y = actor.world_rot.y.wrapping_sub(turn_amount);
    } else {
        actor.world_rot.y = actor.world_rot.y.wrapping_add(x);
    }
    actor.shape_rot.y = actor.world_rot.y;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turns_by_at_most_the_amount() {
        let mut a = Actor::new(Vec3::ZERO, 0);
        // The point due +x: Math_Vec3f_Yaw is atan2(dz, dx) = 0x4000.
        horse_rotate_to_point(&mut a, Vec3::new(100.0, 0.0, 0.0), 300);
        assert_eq!((a.world_rot.y, a.shape_rot.y), (300, 300));
        // Near enough: straight to it.
        a.world_rot.y = 0x4000 - 100;
        horse_rotate_to_point(&mut a, Vec3::new(100.0, 0.0, 0.0), 300);
        assert_eq!(a.world_rot.y, 0x4000);
        // The other way.
        a.world_rot.y = 0x4000 + 1000;
        horse_rotate_to_point(&mut a, Vec3::new(100.0, 0.0, 0.0), 300);
        assert_eq!(a.world_rot.y, 0x4000 + 700);
    }
}
