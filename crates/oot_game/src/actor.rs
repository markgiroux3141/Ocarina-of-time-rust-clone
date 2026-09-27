//! The parts of `Actor` (`z64actor.h`) and `z_actor.c` that Player's movement depends on:
//! velocity from speed and yaw, gravity, position integration, and `Actor_UpdateBgCheckInfo`.

use glam::Vec3;

use crate::bgcheck::{BGCHECK_Y_MIN, IGNORE_ENTITY, PolyId, StaticCollision};
use crate::math::{UPDATE_SCALE, atan2_s, cos_s, sin_s};

pub const BGCHECKFLAG_GROUND: u16 = 1 << 0;
pub const BGCHECKFLAG_GROUND_TOUCH: u16 = 1 << 1;
pub const BGCHECKFLAG_GROUND_LEAVE: u16 = 1 << 2;
pub const BGCHECKFLAG_WALL: u16 = 1 << 3;
pub const BGCHECKFLAG_CEILING: u16 = 1 << 4;
pub const BGCHECKFLAG_WATER: u16 = 1 << 5;
pub const BGCHECKFLAG_WATER_TOUCH: u16 = 1 << 6;
pub const BGCHECKFLAG_GROUND_STRICT: u16 = 1 << 7;
pub const BGCHECKFLAG_CRUSHED: u16 = 1 << 8;
pub const BGCHECKFLAG_PLAYER_WALL_INTERACT: u16 = 1 << 9;

pub const UPDBGCHECKINFO_FLAG_0: u32 = 1 << 0; // walls
pub const UPDBGCHECKINFO_FLAG_1: u32 = 1 << 1; // ceiling
pub const UPDBGCHECKINFO_FLAG_2: u32 = 1 << 2; // floor and water
pub const UPDBGCHECKINFO_FLAG_3: u32 = 1 << 3;
pub const UPDBGCHECKINFO_FLAG_4: u32 = 1 << 4;
pub const UPDBGCHECKINFO_FLAG_5: u32 = 1 << 5;
pub const UPDBGCHECKINFO_FLAG_7: u32 = 1 << 7;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rot {
    pub x: i16,
    pub y: i16,
    pub z: i16,
}

#[derive(Debug, Clone)]
pub struct Actor {
    pub world_pos: Vec3,
    pub world_rot: Rot,
    pub prev_pos: Vec3,
    pub home_pos: Vec3,
    pub shape_rot: Rot,
    pub focus_rot: Rot,
    pub scale: Vec3,
    pub velocity: Vec3,
    pub speed_xz: f32,
    pub gravity: f32,
    pub min_velocity_y: f32,
    pub bg_check_flags: u16,
    pub floor_height: f32,
    pub floor_poly: Option<PolyId>,
    pub wall_poly: Option<PolyId>,
    pub wall_yaw: i16,
    pub y_dist_to_water: f32,
    /// `colChkInfo.displacement` (pushes from other actors' colliders; always zero here).
    pub displacement: Vec3,
}

impl Actor {
    pub fn new(pos: Vec3, yaw: i16) -> Actor {
        let rot = Rot { x: 0, y: yaw, z: 0 };
        Actor {
            world_pos: pos,
            world_rot: rot,
            prev_pos: pos,
            home_pos: pos,
            shape_rot: rot,
            focus_rot: rot,
            scale: Vec3::splat(0.01),
            velocity: Vec3::ZERO,
            speed_xz: 0.0,
            gravity: 0.0,
            min_velocity_y: -20.0,
            bg_check_flags: 0,
            floor_height: 0.0,
            floor_poly: None,
            wall_poly: None,
            wall_yaw: 0,
            y_dist_to_water: BGCHECK_Y_MIN,
            displacement: Vec3::ZERO,
        }
    }

    /// `func_8002D868` (`Actor_UpdateVelocityXZGravity`).
    pub fn update_velocity(&mut self) {
        self.velocity.x = sin_s(self.world_rot.y) * self.speed_xz;
        self.velocity.z = cos_s(self.world_rot.y) * self.speed_xz;
        self.velocity.y += self.gravity;
        if self.velocity.y < self.min_velocity_y {
            self.velocity.y = self.min_velocity_y;
        }
    }

    /// `func_8002D7EC` (`Actor_UpdatePos`).
    pub fn update_pos(&mut self) {
        self.world_pos += self.velocity * UPDATE_SCALE + self.displacement;
    }

    /// `Actor_MoveForward`.
    pub fn move_forward(&mut self) {
        self.update_velocity();
        self.update_pos();
    }

    /// `func_8002E234`: leaving the ground when the floor drops away by more than 11.
    fn check_leave_ground(&mut self, floor_diff: f32, flags: u32) -> bool {
        if self.bg_check_flags & BGCHECKFLAG_GROUND != 0 && floor_diff < -11.0 {
            self.bg_check_flags &= !BGCHECKFLAG_GROUND;
            self.bg_check_flags |= BGCHECKFLAG_GROUND_LEAVE;
            if self.velocity.y < 0.0 && flags & UPDBGCHECKINFO_FLAG_4 != 0 {
                self.velocity.y = 0.0;
            }
            return false;
        }
        true
    }

    /// `func_8002E2AC`: floor raycast from 50 above `pos`, snapping to the floor.
    fn update_floor(&mut self, col: &StaticCollision, mut pos: Vec3, flags: u32, ceiling_poly_hit: bool) -> bool {
        pos.y += 50.0;
        let (floor, poly) = col.entity_raycast_down(pos);
        self.floor_height = floor;
        self.floor_poly = poly;
        self.bg_check_flags &= !(BGCHECKFLAG_GROUND_TOUCH | BGCHECKFLAG_GROUND_LEAVE | BGCHECKFLAG_GROUND_STRICT);
        if self.floor_height <= BGCHECK_Y_MIN {
            return self.check_leave_ground(BGCHECK_Y_MIN, flags);
        }
        let diff = self.floor_height - self.world_pos.y;
        if diff >= 0.0 {
            self.bg_check_flags |= BGCHECKFLAG_GROUND_STRICT;
            if self.bg_check_flags & BGCHECKFLAG_CEILING != 0 && ceiling_poly_hit {
                // Floor and ceiling from the same (scene) bg: undo the x/z move.
                self.world_pos.x = self.prev_pos.x;
                self.world_pos.z = self.prev_pos.z;
            }
            self.world_pos.y = self.floor_height;
            if self.velocity.y <= 0.0 {
                if self.bg_check_flags & BGCHECKFLAG_GROUND == 0 {
                    self.bg_check_flags |= BGCHECKFLAG_GROUND_TOUCH;
                } else if flags & UPDBGCHECKINFO_FLAG_3 != 0 && self.gravity < 0.0 {
                    self.velocity.y = -4.0;
                } else {
                    self.velocity.y = 0.0;
                }
                self.bg_check_flags |= BGCHECKFLAG_GROUND;
            }
        } else {
            return self.check_leave_ground(diff, flags);
        }
        true
    }

    /// `Actor_UpdateBgCheckInfo` (static collision, no water boxes).
    pub fn update_bg_check_info(&mut self, col: &StaticCollision, wall_check_height: f32, wall_check_radius: f32, ceiling_check_height: f32, flags: u32) {
        let dy = self.world_pos.y - self.prev_pos.y;
        if flags & UPDBGCHECKINFO_FLAG_0 != 0 {
            let arg_a = if flags & UPDBGCHECKINFO_FLAG_7 != 0 { 1 } else { 0 };
            let (hit, pos, poly) = col.check_wall(IGNORE_ENTITY, self.world_pos, self.prev_pos, wall_check_radius, wall_check_height, arg_a);
            if hit {
                self.wall_poly = poly;
                self.world_pos = pos;
                if let Some(p) = poly {
                    let n = col.poly(p).normal;
                    self.wall_yaw = atan2_s(n[2] as f32, n[0] as f32);
                }
                self.bg_check_flags |= BGCHECKFLAG_WALL;
            } else {
                self.bg_check_flags &= !BGCHECKFLAG_WALL;
            }
        }
        let mut ceiling_hit = false;
        if flags & UPDBGCHECKINFO_FLAG_1 != 0 {
            let p = Vec3::new(self.world_pos.x, self.prev_pos.y + 10.0, self.world_pos.z);
            if let Some((y, _)) = col.check_ceiling(IGNORE_ENTITY, p, (ceiling_check_height + dy) - 10.0) {
                self.bg_check_flags |= BGCHECKFLAG_CEILING;
                self.world_pos.y = (y + dy) - 10.0;
                ceiling_hit = true;
            } else {
                self.bg_check_flags &= !BGCHECKFLAG_CEILING;
            }
        }
        if flags & UPDBGCHECKINFO_FLAG_2 != 0 {
            let p = Vec3::new(self.world_pos.x, self.prev_pos.y, self.world_pos.z);
            self.update_floor(col, p, flags, ceiling_hit);
            // No water boxes in the test course.
            self.bg_check_flags &= !(BGCHECKFLAG_WATER | BGCHECKFLAG_WATER_TOUCH);
            self.y_dist_to_water = BGCHECK_Y_MIN;
        }
    }
}
