//! The base `Actor` (`z64actor.h`) every actor embeds, and the parts of `z_actor.c` that act on
//! it: velocity from speed and yaw, gravity, position integration, and
//! `Actor_UpdateBgCheckInfo`. The actor system (`crate::actor_ctx`) owns the actors and fills in
//! the fields it manages (id, category, distances to Player).

use eng_collision::bgcheck::{BGCHECK_Y_MIN, CollisionContext, IGNORE_ENTITY, PolyId};
use eng_math::{UPDATE_SCALE, atan2_s, cos_s, sin_s};
use glam::Vec3;

use crate::collision_check::CollisionCheckInfo;

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

// `ACTOR_FLAG_*` (`z64actor.h`), with what they mean where the decomp knows.
/// Targetable.
pub const ACTOR_FLAG_0: u32 = 1 << 0;
/// Hostile (with `ACTOR_FLAG_0`: Z-targeting locks on).
pub const ACTOR_FLAG_2: u32 = 1 << 2;
/// Talked to (`func_800343CC`'s NPCs; with `ACTOR_FLAG_0` Z-targeting treats the actor as
/// friendly).
pub const ACTOR_FLAG_3: u32 = 1 << 3;
/// Updates even when not in view (`Actor_UpdateAll` updates actors with flag 4 or 6).
pub const ACTOR_FLAG_4: u32 = 1 << 4;
/// Drawn even when not in view.
pub const ACTOR_FLAG_5: u32 = 1 << 5;
/// In view this frame (set by `func_800314D4`'s culling).
pub const ACTOR_FLAG_6: u32 = 1 << 6;
/// Seen only with the Lens of Truth (drawn in `Actor_DrawAll`'s lens pass: not ported).
pub const ACTOR_FLAG_7: u32 = 1 << 7;
/// A talk request was made (`Actor_ProcessTalkRequest` answers it).
pub const ACTOR_FLAG_8: u32 = 1 << 8;
/// Talking starts without A (`func_8083B644`: the actor's offer is taken at once).
pub const ACTOR_FLAG_16: u32 = 1 << 16;
/// With `ACTOR_FLAG_0`: Navi can be asked about it with C-Up (`func_8083B644`).
pub const ACTOR_FLAG_18: u32 = 1 << 18;
/// `ACTOR_FLAG_19`, `_20`, `_21`, `_28`: how `Actor_DrawAll` plays `actor->sfx`
/// (`func_80030ED8`).
pub const ACTOR_FLAG_19: u32 = 1 << 19;
pub const ACTOR_FLAG_20: u32 = 1 << 20;
pub const ACTOR_FLAG_21: u32 = 1 << 21;
pub const ACTOR_FLAG_28: u32 = 1 << 28;
/// `Actor_Draw` binds no point lights for it (`Lights_BindAll` with no position).
pub const ACTOR_FLAG_22: u32 = 1 << 22;
pub const ACTOR_FLAG_23: u32 = 1 << 23;
pub const ACTOR_FLAG_24: u32 = 1 << 24;
pub const ACTOR_FLAG_25: u32 = 1 << 25;
pub const ACTOR_FLAG_26: u32 = 1 << 26;
/// Can't be targeted by the arrow-pointed search (`func_8002EFC0`).
pub const ACTOR_FLAG_27: u32 = 1 << 27;

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
    /// `id` (`ACTOR_*`) and `category` (`ACTORCAT_*`), from the actor's profile.
    pub id: i16,
    pub category: usize,
    pub flags: u32,
    pub params: i16,
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
    /// `floorBgId`: `BGCHECK_SCENE` or the bg actor Player stands on.
    pub floor_bg_id: u16,
    pub wall_poly: Option<PolyId>,
    /// Yaw added by a rotating platform this frame (`func_800432A0`; Player adds it to
    /// `currentYaw`).
    pub carried_yaw: i16,
    pub wall_yaw: i16,
    pub y_dist_to_water: f32,
    /// `shape.yOffset`: model offset along y, in model units (× `scale.y` when drawn).
    pub shape_y_offset: f32,
    /// `room`: the room the actor belongs to (the current room when it spawned), or -1 for
    /// none (Player, doors): a room change deletes actors of other rooms (`func_80031B14`).
    pub room: i8,
    /// `home.rot`.
    pub home_rot: Rot,
    /// `objBankIndex`: the object bank the actor's assets are in (`None` for actors built
    /// directly rather than spawned by id).
    pub obj_bank_index: Option<usize>,
    /// `colChkInfo`: collision properties, and this frame's pushes and damage.
    pub col_chk_info: CollisionCheckInfo,
    /// `focus.pos`: where targeting aims (Player: the head).
    pub focus_pos: Vec3,
    /// `targetArrowOffset`: how far (times `scale.y`) above the focus the target arrow floats.
    /// No ported actor sets it, so it stays at the zeroed actor memory's 0.
    pub target_arrow_offset: f32,
    /// `targetMode`: the targeting range class (`D_80115FF8`).
    pub target_mode: u8,
    /// `targetPriority` (0 = normal).
    pub target_priority: u8,
    /// `isTargeted`: Player's current lock-on target.
    pub is_targeted: bool,
    /// `freezeTimer`: frames the actor skips updating.
    pub freeze_timer: u16,
    /// Distances and yaw to Player, refreshed by `Actor_UpdateAll` before each update.
    pub xz_dist_to_player: f32,
    pub y_dist_to_player: f32,
    pub xyz_dist_to_player_sq: f32,
    pub yaw_towards_player: i16,
    /// `Actor_Kill` ran: `update` and `draw` are NULL, and `Actor_UpdateAll` deletes it.
    pub killed: bool,
    /// `parent`, `child`: set by `Actor_SpawnAsChild` (and by Player when it holds or rides).
    pub parent: Option<crate::actor_ctx::ActorHandle>,
    pub child: Option<crate::actor_ctx::ActorHandle>,
    /// `textId`: what the actor says when talked to.
    pub text_id: u16,
    /// Moved without passing in between (spawned, respawned, a scene change): the renderer
    /// doesn't blend from the last frame.
    pub teleported: bool,
    /// `projectedPos`, `projectedW`: the position through `play->viewProjectionMtxF`, as
    /// `Actor_DrawAll` sets it every frame (the sound effects are positioned by it).
    pub projected_pos: Vec3,
    pub projected_w: f32,
    /// `sfx`: a sound the actor asks `Actor_DrawAll` to play this frame (`func_8002F8F0` and
    /// the rest, `func_80030ED8`); `Actor_UpdateAll` clears it first.
    pub sfx: u16,
}

impl Actor {
    /// A spawned actor as `Actor_Spawn` + `Actor_Init` set it up before the actor's own init:
    /// home = world = `pos`, shape rotation = world rotation, focus at the position, scale 0.01,
    /// `targetMode` 3, `minVelocityY` -20, `xyzDistToPlayerSq` `FLT_MAX`, and
    /// `CollisionCheck_InitInfo`.
    pub fn new(pos: Vec3, yaw: i16) -> Actor {
        let rot = Rot { x: 0, y: yaw, z: 0 };
        Actor {
            id: 0,
            category: 0,
            flags: 0,
            params: 0,
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
            floor_bg_id: eng_collision::bgcheck::BGCHECK_SCENE,
            wall_poly: None,
            carried_yaw: 0,
            wall_yaw: 0,
            y_dist_to_water: BGCHECK_Y_MIN,
            shape_y_offset: 0.0,
            room: 0,
            home_rot: rot,
            obj_bank_index: None,
            col_chk_info: CollisionCheckInfo::new(),
            focus_pos: pos,
            target_arrow_offset: 0.0,
            target_mode: 3,
            target_priority: 0,
            is_targeted: false,
            freeze_timer: 0,
            xz_dist_to_player: f32::MAX,
            y_dist_to_player: 0.0,
            xyz_dist_to_player_sq: f32::MAX,
            yaw_towards_player: 0,
            killed: false,
            parent: None,
            child: None,
            text_id: 0,
            teleported: true,
            projected_pos: Vec3::ZERO,
            projected_w: 0.0,
            sfx: 0,
        }
    }

    /// `func_8002F8F0`: `sfx` played at the actor (`Audio_PlaySfxGeneral` at `projectedPos`).
    pub fn func_8002f8f0(&mut self, sfx_id: u16) {
        self.sfx = sfx_id;
        self.flags |= ACTOR_FLAG_19;
        self.flags &= !(ACTOR_FLAG_20 | ACTOR_FLAG_21 | ACTOR_FLAG_28);
    }

    /// `func_8002F91C`: `sfx` with no position (`func_80078884`).
    pub fn func_8002f91c(&mut self, sfx_id: u16) {
        self.sfx = sfx_id;
        self.flags |= ACTOR_FLAG_20;
        self.flags &= !(ACTOR_FLAG_19 | ACTOR_FLAG_21 | ACTOR_FLAG_28);
    }

    /// `func_8002F948`: `sfx` with no position (`func_800788CC`).
    pub fn func_8002f948(&mut self, sfx_id: u16) {
        self.sfx = sfx_id;
        self.flags |= ACTOR_FLAG_21;
        self.flags &= !(ACTOR_FLAG_19 | ACTOR_FLAG_20 | ACTOR_FLAG_28);
    }

    /// `func_8002F974`: `sfx` at the actor (`func_80078914`).
    pub fn func_8002f974(&mut self, sfx_id: u16) {
        self.flags &= !(ACTOR_FLAG_19 | ACTOR_FLAG_20 | ACTOR_FLAG_21 | ACTOR_FLAG_28);
        self.sfx = sfx_id;
    }

    /// Targetable and hostile: Z-targeting locks on (`ACTOR_FLAG_0 | ACTOR_FLAG_2`).
    pub fn is_hostile(&self) -> bool {
        self.flags & (ACTOR_FLAG_0 | ACTOR_FLAG_2) == ACTOR_FLAG_0 | ACTOR_FLAG_2
    }

    /// `Actor_Kill`.
    /// `func_8002DBD0` (`Actor_WorldToActorCoords`): `pos` in the actor's frame (its shape yaw,
    /// from its position).
    pub fn world_to_actor_coords(&self, pos: Vec3) -> Vec3 {
        let (c, s) = (cos_s(self.shape_rot.y), sin_s(self.shape_rot.y));
        let (dx, dz) = (pos.x - self.world_pos.x, pos.z - self.world_pos.z);
        Vec3::new(dx * c - dz * s, pos.y - self.world_pos.y, dx * s + dz * c)
    }

    /// `Player_IsFacingActor`: Player (facing `player_yaw`) faces this actor within `max_angle`.
    pub fn player_is_facing(&self, player_yaw: i16, max_angle: i16) -> bool {
        let d = self.yaw_towards_player.wrapping_add(i16::MIN).wrapping_sub(player_yaw);
        (d as i32).abs() < max_angle as i32
    }

    pub fn kill(&mut self) {
        self.killed = true;
        self.flags &= !ACTOR_FLAG_0;
    }

    /// `Actor_SetFocus`: the focus `y_offset` above the position, facing as the actor moves.
    pub fn set_focus(&mut self, y_offset: f32) {
        self.focus_pos = self.world_pos + Vec3::Y * y_offset;
        self.focus_rot = self.world_rot;
    }

    /// `Actor_UpdateAll`'s per-actor distances to Player: `Actor_WorldDistXZToActor`,
    /// `Actor_HeightDiff`, their squares, and `Actor_WorldYawTowardActor`.
    pub fn update_distances(&mut self, player_pos: Vec3) {
        let d = player_pos - self.world_pos;
        self.xz_dist_to_player = (d.x * d.x + d.z * d.z).sqrt();
        self.y_dist_to_player = player_pos.y - self.world_pos.y;
        self.xyz_dist_to_player_sq = self.xz_dist_to_player * self.xz_dist_to_player + self.y_dist_to_player * self.y_dist_to_player;
        self.yaw_towards_player = atan2_s(d.z, d.x);
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
        self.world_pos += self.velocity * UPDATE_SCALE + self.col_chk_info.displacement;
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
    fn update_floor(&mut self, col: &CollisionContext, mut pos: Vec3, flags: u32, ceiling_bg: Option<u16>) -> bool {
        pos.y += 50.0;
        let (floor, poly) = col.entity_raycast_down(pos);
        self.floor_height = floor;
        self.floor_poly = poly;
        self.bg_check_flags &= !(BGCHECKFLAG_GROUND_TOUCH | BGCHECKFLAG_GROUND_LEAVE | BGCHECKFLAG_GROUND_STRICT);
        if self.floor_height <= BGCHECK_Y_MIN {
            return self.check_leave_ground(BGCHECK_Y_MIN, flags);
        }
        let diff = self.floor_height - self.world_pos.y;
        if let Some(p) = poly {
            self.floor_bg_id = p.bg;
        }
        if diff >= 0.0 {
            self.bg_check_flags |= BGCHECKFLAG_GROUND_STRICT;
            if self.bg_check_flags & BGCHECKFLAG_CEILING != 0
                && let Some(cb) = ceiling_bg
            {
                if self.floor_bg_id != cb {
                    // Floor and ceiling from different meshes closing in: crushed.
                    if diff > 15.0 {
                        self.bg_check_flags |= BGCHECKFLAG_CRUSHED;
                    }
                } else {
                    // Floor and ceiling from the same mesh: undo the x/z move.
                    self.world_pos.x = self.prev_pos.x;
                    self.world_pos.z = self.prev_pos.z;
                }
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

    /// `Actor_UpdateBgCheckInfo`.
    pub fn update_bg_check_info(&mut self, col: &CollisionContext, wall_check_height: f32, wall_check_radius: f32, ceiling_check_height: f32, flags: u32) {
        let dy = self.world_pos.y - self.prev_pos.y;
        // func_800433A4: ride the platform we're standing on.
        self.carried_yaw = 0;
        if self.floor_bg_id != eng_collision::bgcheck::BGCHECK_SCENE
            && self.bg_check_flags & BGCHECKFLAG_GROUND != 0
            && let Some((pos, dyaw)) = col.dyna.carry(self.floor_bg_id, self.world_pos)
        {
            self.world_pos = pos;
            self.shape_rot.y = self.shape_rot.y.wrapping_add(dyaw);
            self.world_rot.y = self.world_rot.y.wrapping_add(dyaw);
            self.carried_yaw = dyaw;
        }
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
        let mut ceiling_bg = None;
        if flags & UPDBGCHECKINFO_FLAG_1 != 0 {
            let p = Vec3::new(self.world_pos.x, self.prev_pos.y + 10.0, self.world_pos.z);
            if let Some((y, poly)) = col.check_ceiling(IGNORE_ENTITY, p, (ceiling_check_height + dy) - 10.0) {
                self.bg_check_flags |= BGCHECKFLAG_CEILING;
                self.world_pos.y = (y + dy) - 10.0;
                ceiling_bg = Some(poly.bg);
            } else {
                self.bg_check_flags &= !BGCHECKFLAG_CEILING;
            }
        }
        if flags & UPDBGCHECKINFO_FLAG_2 != 0 {
            let p = Vec3::new(self.world_pos.x, self.prev_pos.y, self.world_pos.z);
            self.update_floor(col, p, flags, ceiling_bg);
            // WaterBox_GetSurface1 (ripples not modelled).
            match col.water_surface(self.world_pos.x, self.world_pos.z, col.water_room) {
                Some(y) => {
                    self.y_dist_to_water = y - self.world_pos.y;
                    if self.y_dist_to_water < 0.0 {
                        self.bg_check_flags &= !(BGCHECKFLAG_WATER | BGCHECKFLAG_WATER_TOUCH);
                    } else {
                        if self.bg_check_flags & BGCHECKFLAG_WATER == 0 {
                            self.bg_check_flags |= BGCHECKFLAG_WATER_TOUCH;
                        }
                        self.bg_check_flags |= BGCHECKFLAG_WATER;
                    }
                }
                None => {
                    self.bg_check_flags &= !(BGCHECKFLAG_WATER | BGCHECKFLAG_WATER_TOUCH);
                    self.y_dist_to_water = BGCHECK_Y_MIN;
                }
            }
        }
    }
}
