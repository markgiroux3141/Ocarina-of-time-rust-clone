//! `En_Goroiwa` (`ovl_En_Goroiwa/z_en_goroiwa.c`): a rolling boulder that follows one of the
//! scene's paths (`play->setupPathList[params & 0xFF]`), knocking Link down when it runs into
//! him.
//!
//! Params: bits 0..7 the path, bits 8..9 the loop mode (`ENGOROIWA_LOOPMODE_*`: 0 round the
//! path and back to its start, 1 the same with the break's fragments at the ends, 3 there and
//! back), bit 10 whether it falls with gravity and its floor check (1) or keeps to the path's
//! heights (0, climbing and dropping between points). `home.rot.z & 1`: after hitting Link,
//! 50 frames without colliding, and a short wait.
//!
//! Kokiri Forest's training area has one: params 0x0C02, `rot.z` 1. Path 2 is a loop of five
//! points round the corridors' block; loop mode 0, gravity (bit 10), so `EnGoroiwa_MoveAndFall`
//! and `EnGoroiwa_SetupMoveAndFallToGround` on a hit, at `R_EN_GOROIWA_SPEED` 920 (9.2 a frame).
//!
//! The whole overlay is ported, with what Kokiri Forest's boulder never reaches: the climbs and
//! drops between points of different heights (bit 10 clear: `EnGoroiwa_MoveUp`,
//! `EnGoroiwa_MoveDown`), the round trip and the breaking loop. Not ported: the
//! quake of a drop (`Quake_Add`), the dust, splashes, ripples and fragments (the effects; their
//! `Rand_ZeroOne` calls in the overlay are made), and the circle shadow
//! (`ActorShadow_DrawCircle`).

use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_4, Actor, BGCHECKFLAG_GROUND, UPDBGCHECKINFO_FLAG_2, UPDBGCHECKINFO_FLAG_3, UPDBGCHECKINFO_FLAG_4};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile, audio_play_actor_sfx2, func_8002f6d4, func_8002f7dc};
use oot_game::audio::sfx::{NA_SE_EV_BIGBALL_ROLL, NA_SE_PL_BODY_HIT, SFX_FLAG};
use oot_game::collision_check::*;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::sys_matrix::{MtxF, binang_to_rad};

pub const ACTOR_EN_GOROIWA: i16 = 0x0130;
const OBJECT: &str = "object_goroiwa";

/// `En_Goroiwa_InitVars`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_GOROIWA, name: "En_Goroiwa", category: ACTORCAT_PROP, flags: ACTOR_FLAG_4, object: OBJECT };

/// `stateFlags`.
pub const ENGOROIWA_ENABLE_AT: u8 = 1 << 0;
pub const ENGOROIWA_ENABLE_OC: u8 = 1 << 1;
pub const ENGOROIWA_PLAYER_IN_THE_WAY: u8 = 1 << 2;
pub const ENGOROIWA_RETAIN_ROT_SPEED: u8 = 1 << 3;
pub const ENGOROIWA_IN_WATER: u8 = 1 << 4;

/// The loop modes (`(params >> 8) & 3`).
pub const ENGOROIWA_LOOPMODE_ONEWAY: i16 = 0;
pub const ENGOROIWA_LOOPMODE_ONEWAY_BREAK: i16 = 1;
pub const ENGOROIWA_LOOPMODE_ROUNDTRIP: i16 = 3;

/// `SCENE_SPOT04`: Kokiri Forest's boulders are slower.
const SCENE_SPOT04: u16 = 0x55;

/// `sJntSphElementsInit`: one sphere of 58, AT `0x20000000` with 4 damage.
const JNT_SPH_ELEMENTS: [ColliderJntSphElementInit; 1] = [ColliderJntSphElementInit {
    info: ColliderInfoInit {
        elem_type: ELEMTYPE_UNK0,
        toucher: ColliderTouch { dmg_flags: 0x2000_0000, effect: 0x00, damage: 0x04 },
        bumper: ColliderBumpInit { dmg_flags: 0, effect: 0, defense: 0 },
        toucher_flags: TOUCH_ON | TOUCH_SFX_NORMAL,
        bumper_flags: BUMP_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    limb: 0,
    model_sphere: eng_collision::math3d::Sphere16 { center: [0, 0, 0], radius: 58 },
    scale: 100,
}];

/// `sJntSphInit`.
const JNT_SPH_INIT: ColliderInit =
    ColliderInit { col_type: COLTYPE_NONE, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_JNTSPH };

/// `sColChkInfoInit`.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 0, cyl_radius: 12, cyl_height: 60, mass: MASS_HEAVY };

/// `EnGoroiwa_UpdateCollider`'s and `EnGoroiwa_SpawnFragments`' `yOffsets`, by bit 10: the
/// sphere's centre above the position.
const Y_OFFSETS: [f32; 2] = [0.0, 59.5];
/// `EnGoroiwa_Init`'s `yOffsets`: `shape.yOffset`.
const SHAPE_Y_OFFSETS: [f32; 2] = [0.0, 595.0];
/// `EnGoroiwa_SetupWait`'s `waitDurations`, by `home.rot.z & 1`.
const WAIT_DURATIONS: [i16; 2] = [20, 6];

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `EnGoroiwa_Roll`.
    Roll,
    /// `EnGoroiwa_MoveAndFallToGround`.
    MoveAndFallToGround,
    /// `EnGoroiwa_Wait`.
    Wait,
    /// `EnGoroiwa_MoveUp`.
    MoveUp,
    /// `EnGoroiwa_MoveDown`.
    MoveDown,
}

pub struct EnGoroiwa {
    pub actor: Actor,
    pub action: Action,
    pub collider: ColliderJntSph,
    pub prev_unit_roll_axis: Vec3,
    pub prev_roll_angle_diff: f32,
    pub roll_rot_speed: f32,
    pub wait_timer: i16,
    pub bounce_count: i16,
    pub collision_disabled_timer: i16,
    pub end_waypoint: i16,
    pub current_waypoint: i16,
    pub next_waypoint: i16,
    pub path_direction: i16,
    pub is_in_kokiri: bool,
    pub state_flags: u8,
    /// `R_EN_GOROIWA_SPEED` (`mREG(12)`), which `EnGoroiwa_SetSpeed` sets at init: a debug
    /// register the C shares between all boulders (every boulder of a scene sets the same).
    pub speed_reg: i16,
}

impl EnGoroiwa {
    /// `(params >> 10) & 1`: gravity and the floor check.
    fn bgc(&self) -> usize {
        ((self.actor.params >> 10) & 1) as usize
    }

    fn loop_mode(&self) -> i16 {
        (self.actor.params >> 8) & 3
    }

    fn rot_z_bit(&self) -> bool {
        self.actor.home_rot.z & 1 == 1
    }

    /// `play->setupPathList[params & 0xFF]`'s point `i`.
    fn point(&self, play: &PlayState, i: i16) -> Vec3 {
        play.setup_path_list()[(self.actor.params & 0xFF) as usize].point(i as usize)
    }

    /// `EnGoroiwa_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: gravity -0.86, minVelocityY -15, scale 0.1 (the cull zone isn't ported).
        actor.gravity = -0.86;
        actor.min_velocity_y = -15.0;
        actor.scale = Vec3::splat(0.1);
        let mut this = EnGoroiwa {
            // EnGoroiwa_InitCollider.
            collider: ColliderJntSph::new(&JNT_SPH_INIT, &JNT_SPH_ELEMENTS),
            action: Action::Roll,
            prev_unit_roll_axis: Vec3::ZERO,
            prev_roll_angle_diff: 0.0,
            roll_rot_speed: 0.0,
            wait_timer: 0,
            bounce_count: 0,
            collision_disabled_timer: 0,
            end_waypoint: 0,
            current_waypoint: 0,
            next_waypoint: 0,
            path_direction: 0,
            is_in_kokiri: false,
            state_flags: 0,
            speed_reg: 0,
            actor,
        };
        this.update_collider();
        this.collider.elements[0].dim.world_sphere.radius = 58;
        let path_idx = (this.actor.params & 0xFF) as usize;
        if path_idx == 0xFF {
            log::warn!("En_Goroiwa: arg_data 0x{:04x} is invalid", this.actor.params);
            this.actor.kill();
            return Box::new(this);
        }
        if play.setup_path_list().get(path_idx).is_none_or(|p| p.count() < 2) {
            // (A path past the list reads past it in the C.)
            log::warn!("En_Goroiwa: invalid path data (path {path_idx})");
            this.actor.kill();
            return Box::new(this);
        }
        this.actor.col_chk_info.set_info(None, &COL_CHK_INFO_INIT);
        // ActorShape_Init(&shape, yOffsets[bgc], ActorShadow_DrawCircle, 9.4f), shadowAlpha 200.
        this.actor.shape_y_offset = SHAPE_Y_OFFSETS[this.bgc()];
        this.set_speed(play);
        this.init_path(play);
        this.teleport_to_waypoint(play, 0);
        this.init_rotation();
        this.face_next_waypoint(play);
        this.setup_roll();
        Box::new(this)
    }

    /// `EnGoroiwa_UpdateCollider`: the sphere at the position, `yOffsets[bgc]` up.
    fn update_collider(&mut self) {
        let s = &mut self.collider.elements[0].dim.world_sphere;
        let p = self.actor.world_pos;
        s.center = [p.x as i16, (p.y + Y_OFFSETS[((self.actor.params >> 10) & 1) as usize]) as i16, p.z as i16];
    }

    /// `EnGoroiwa_UpdateFlags`.
    fn update_flags(&mut self, set_flags: u8) {
        self.state_flags &= !(ENGOROIWA_ENABLE_AT | ENGOROIWA_ENABLE_OC);
        self.state_flags |= set_flags;
    }

    /// `EnGoroiwa_SetSpeed`.
    fn set_speed(&mut self, play: &PlayState) {
        if play.scene_id == SCENE_SPOT04 {
            self.is_in_kokiri = true;
            self.speed_reg = 920;
        } else {
            self.is_in_kokiri = false;
            self.speed_reg = 1000;
        }
    }

    /// `R_EN_GOROIWA_SPEED * 0.01f`.
    fn speed(&self) -> f32 {
        self.speed_reg as f32 * 0.01
    }

    /// `EnGoroiwa_FaceNextWaypoint`.
    fn face_next_waypoint(&mut self, play: &PlayState) {
        let next = self.point(play, self.next_waypoint);
        self.actor.world_rot.y = eng_math::vec3f_yaw(self.actor.world_pos, next);
    }

    /// `EnGoroiw_CheckEndOfPath`.
    fn check_end_of_path(&mut self) {
        let loop_mode = self.loop_mode();
        let one_way = loop_mode == ENGOROIWA_LOOPMODE_ONEWAY || loop_mode == ENGOROIWA_LOOPMODE_ONEWAY_BREAK;
        if self.next_waypoint < 0 {
            if one_way {
                self.current_waypoint = self.end_waypoint;
                self.next_waypoint = self.end_waypoint - 1;
                self.path_direction = -1;
            } else if loop_mode == ENGOROIWA_LOOPMODE_ROUNDTRIP {
                self.current_waypoint = 0;
                self.next_waypoint = 1;
                self.path_direction = 1;
            }
        } else if self.next_waypoint > self.end_waypoint {
            if one_way {
                self.current_waypoint = 0;
                self.next_waypoint = 1;
                self.path_direction = 1;
            } else if loop_mode == ENGOROIWA_LOOPMODE_ROUNDTRIP {
                self.current_waypoint = self.end_waypoint;
                self.next_waypoint = self.end_waypoint - 1;
                self.path_direction = -1;
            }
        }
    }

    /// `EnGoroiwa_SetNextWaypoint`.
    fn set_next_waypoint(&mut self) {
        self.current_waypoint = self.next_waypoint;
        self.next_waypoint += self.path_direction;
        self.check_end_of_path();
    }

    /// `EnGoroiwa_ReverseDirection`.
    fn reverse_direction(&mut self) {
        self.path_direction *= -1;
        self.current_waypoint = self.next_waypoint;
        self.next_waypoint += self.path_direction;
    }

    /// `EnGoroiwa_InitPath`.
    fn init_path(&mut self, play: &PlayState) {
        self.end_waypoint = play.setup_path_list()[(self.actor.params & 0xFF) as usize].count() as i16 - 1;
        self.current_waypoint = 0;
        self.next_waypoint = 1;
        self.path_direction = 1;
    }

    /// `EnGoroiwa_TeleportToWaypoint`.
    fn teleport_to_waypoint(&mut self, play: &PlayState, waypoint: i16) {
        self.actor.world_pos = self.point(play, waypoint);
    }

    /// `EnGoroiwa_InitRotation`.
    fn init_rotation(&mut self) {
        self.prev_unit_roll_axis.x = 1.0;
        self.roll_rot_speed = 1.0;
    }

    /// `EnGoroiwa_GetAscendDirection`: 1 when the next point is straight above, -1 below, 0
    /// otherwise.
    fn get_ascend_direction(&self, play: &PlayState) -> i32 {
        let next = self.point(play, self.next_waypoint);
        let cur = self.point(play, self.current_waypoint);
        if next.x == cur.x && next.z == cur.z {
            if next.y == cur.y {
                log::warn!("En_Goroiwa: invalid path data (points overlap), arg_data 0x{:04x}", self.actor.params);
            }
            return if next.y > cur.y { 1 } else { -1 };
        }
        0
    }

    /// `EnGoroiwa_SpawnDust`: the dust of a drop (`func_800286CC`, not ported); its
    /// `Rand_ZeroOne` calls are made.
    fn spawn_dust(play: &mut PlayState) {
        for _ in 0..8 {
            for _ in 0..5 {
                play.rand.zero_one();
            }
        }
    }

    /// `EnGoroiwa_MoveAndFall`: towards the next point across at the speed along the yaw,
    /// falling with gravity. True when across at the point.
    fn move_and_fall(&mut self, play: &PlayState) -> bool {
        let target = self.speed();
        eng_math::step_to_f(&mut self.actor.speed_xz, target, 0.3);
        // func_8002D868.
        self.actor.update_velocity();
        let next = self.point(play, self.next_waypoint);
        let mut result = true;
        result &= eng_math::step_to_f(&mut self.actor.world_pos.x, next.x, self.actor.velocity.x.abs());
        result &= eng_math::step_to_f(&mut self.actor.world_pos.z, next.z, self.actor.velocity.z.abs());
        self.actor.world_pos.y += self.actor.velocity.y;
        result
    }

    /// `EnGoroiwa_Move`: straight at the next point (along the path's segment, or at the point
    /// once within 5), at the speed. True at the point.
    fn move_(&mut self, play: &PlayState) -> bool {
        let next = self.point(play, self.next_waypoint);
        let cur = self.point(play, self.current_waypoint);
        let target = self.speed();
        eng_math::step_to_f(&mut self.actor.speed_xz, target, 0.3);
        let pos_diff = if next.distance_squared(self.actor.world_pos) < 5.0 * 5.0 { next - self.actor.world_pos } else { next - cur };
        // EnGoroiwa_Vec3fNormalize leaves the velocity as it was for a zero difference.
        let mag = pos_diff.length();
        if mag >= 0.001 {
            self.actor.velocity = pos_diff * (1.0 / mag);
        }
        self.actor.velocity *= self.actor.speed_xz;
        let v = self.actor.velocity;
        let mut reached = true;
        reached &= eng_math::step_to_f(&mut self.actor.world_pos.x, next.x, v.x.abs());
        reached &= eng_math::step_to_f(&mut self.actor.world_pos.y, next.y, v.y.abs());
        reached &= eng_math::step_to_f(&mut self.actor.world_pos.z, next.z, v.z.abs());
        reached
    }

    /// `EnGoroiwa_MoveUpToNextWaypoint`.
    fn move_up_to_next_waypoint(&mut self, play: &PlayState) -> bool {
        let next = self.point(play, self.next_waypoint);
        let target = self.speed() * 0.5;
        eng_math::step_to_f(&mut self.actor.velocity.y, target, 0.18);
        self.actor.world_pos.x = next.x;
        self.actor.world_pos.z = next.z;
        eng_math::step_to_f(&mut self.actor.world_pos.y, next.y, self.actor.velocity.y.abs())
    }

    /// `EnGoroiwa_MoveDownToNextWaypoint`: dropping to the next point, bouncing once at 0.3;
    /// into water, slowed. (The quake, the dust and the splashes aren't ported.)
    fn move_down_to_next_waypoint(&mut self, play: &mut PlayState) -> bool {
        let next = self.point(play, self.next_waypoint);
        let next_y = next.y;
        eng_math::step_to_f(&mut self.actor.velocity.y, -14.0, 1.0);
        self.actor.world_pos.x = next.x;
        self.actor.world_pos.z = next.z;
        let this_y = self.actor.world_pos.y;
        self.actor.world_pos.y += self.actor.velocity.y;
        if self.actor.velocity.y < 0.0 && self.actor.world_pos.y <= next_y {
            if self.bounce_count == 0 {
                // Quake_Add(GET_ACTIVE_CAM(play), 3) within 600 of Link: not ported.
                self.roll_rot_speed = 0.0;
                if self.state_flags & ENGOROIWA_IN_WATER == 0 {
                    // BgCheck_EntityRaycastDown5 from 50 up.
                    let (floor_y, _) = play.col.entity_raycast_down(self.actor.world_pos + Vec3::Y * 50.0);
                    let y_dist_to_floor = floor_y - (self.actor.world_pos.y - 59.5);
                    if y_dist_to_floor.abs() < 15.0 {
                        Self::spawn_dust(play);
                    }
                }
            }
            if self.bounce_count >= 1 {
                return true;
            }
            self.bounce_count += 1;
            self.actor.velocity.y *= -0.3;
            self.actor.world_pos.y = next_y - ((self.actor.world_pos.y - next_y) * 0.3);
        }
        if self.bounce_count == 0
            && let Some(y_surface) = play.col.water_surface(self.actor.world_pos.x, self.actor.world_pos.z, play.col.water_room)
            && self.actor.world_pos.y <= y_surface
        {
            self.state_flags |= ENGOROIWA_IN_WATER;
            if y_surface < this_y {
                // EnGoroiwa_SpawnWaterEffects: the splashes and ripples (not ported).
                self.actor.velocity.y *= 0.2;
            }
            if self.actor.velocity.y < -8.0 {
                self.actor.velocity.y = -8.0;
            }
        }
        false
    }

    /// `EnGoroiwa_UpdateRotation`: rolls by the distance moved over the radius (59.5) about the
    /// horizontal axis across the velocity (the last one when it's still), added to the shape's
    /// rotation. While `ENGOROIWA_RETAIN_ROT_SPEED`, the last step's angle again.
    /// (`EnGoroiwa_GetPrevWaypointDiff`, which the C calls then, has no effect.)
    fn update_rotation(&mut self) {
        if self.state_flags & ENGOROIWA_RETAIN_ROT_SPEED == 0 {
            // Math3D_Vec3f_DistXYZ.
            self.prev_roll_angle_diff = self.actor.world_pos.distance(self.actor.prev_pos) * (1.0 / 59.5);
        }
        let roll_angle_diff = self.prev_roll_angle_diff * self.roll_rot_speed;
        // Math3D_Vec3f_Cross(&unitY, &velocity).
        let v = self.actor.velocity;
        let roll_axis = Vec3::new(v.z, 0.0, -v.x);
        let mag = roll_axis.length();
        let unit_roll_axis = if mag >= 0.001 {
            let u = roll_axis * (1.0 / mag);
            self.prev_unit_roll_axis = u;
            u
        } else {
            self.prev_unit_roll_axis
        };
        let mut m = MtxF::rotate_axis(roll_angle_diff, unit_roll_axis);
        let r = self.actor.shape_rot;
        m.rotate_y(binang_to_rad(r.y));
        m.rotate_x(binang_to_rad(r.x));
        m.rotate_z(binang_to_rad(r.z));
        let [x, y, z] = m.to_yxz_rot_s(false);
        self.actor.shape_rot.x = x;
        self.actor.shape_rot.y = y;
        self.actor.shape_rot.z = z;
    }

    /// `EnGoroiwa_NextWaypoint`: on to the next point; round the loop, back at the start.
    fn next_waypoint(&mut self, play: &PlayState) {
        let loop_mode = self.loop_mode();
        self.set_next_waypoint();
        if (loop_mode == ENGOROIWA_LOOPMODE_ONEWAY || loop_mode == ENGOROIWA_LOOPMODE_ONEWAY_BREAK) && (self.current_waypoint == 0 || self.current_waypoint == self.end_waypoint) {
            self.teleport_to_waypoint(play, self.current_waypoint);
        }
        self.face_next_waypoint(play);
    }

    /// `EnGoroiwa_SpawnFragments`: the break at the path's ends (`EffectSsKakera_Spawn`,
    /// `func_80033480`: not ported); the overlay's `Rand_ZeroOne` calls are made.
    fn spawn_fragments(play: &mut PlayState) {
        for _ in 0..16 {
            for _ in 0..6 {
                play.rand.zero_one();
            }
        }
    }

    /// `EnGoroiwa_SetupRoll`.
    fn setup_roll(&mut self) {
        self.action = Action::Roll;
        self.update_flags(ENGOROIWA_ENABLE_AT | ENGOROIWA_ENABLE_OC);
        self.roll_rot_speed = 1.0;
    }

    /// `func_8002F6D4(play, &this->actor, 2.0f, this->actor.yawTowardsPlayer, 0.0f, damage)`.
    fn knock_down_player(&mut self, play: &mut PlayState, damage: u8) {
        func_8002f6d4(play, 2.0, self.actor.yaw_towards_player, 0.0, damage);
    }

    /// `func_8002F7DC(&GET_PLAYER(play)->actor, NA_SE_PL_BODY_HIT)`: at Link.
    fn body_hit_sfx(play: &mut PlayState) {
        if let Some(ph) = play.player {
            func_8002f7dc(play, ph, NA_SE_PL_BODY_HIT);
        }
    }

    /// `EnGoroiwa_Roll`. On a hit on Link: if he's ahead (within a quarter turn of its way),
    /// back the way it came (always with gravity, else without `home.rot.z` bit 0); Link
    /// knocked down; then a fall to the ground (gravity) or a wait. Otherwise on along the
    /// path, and at each point on to the next; rolling all the while.
    fn roll(&mut self, play: &mut PlayState) {
        if self.collider.base.at_flags & AT_HIT != 0 {
            self.collider.base.at_flags &= !AT_HIT;
            self.state_flags &= !ENGOROIWA_PLAYER_IN_THE_WAY;
            let yaw_diff = self.actor.yaw_towards_player.wrapping_sub(self.actor.world_rot.y);
            if yaw_diff > -0x4000 && yaw_diff < 0x4000 {
                self.state_flags |= ENGOROIWA_PLAYER_IN_THE_WAY;
                if self.bgc() == 1 || !self.rot_z_bit() {
                    self.reverse_direction();
                    self.face_next_waypoint(play);
                }
            }
            // "Player knocked down".
            self.knock_down_player(play, 0);
            if self.bgc() == 1 {
                self.setup_move_and_fall_to_ground();
            } else {
                self.setup_wait();
            }
            Self::body_hit_sfx(play);
            if self.rot_z_bit() {
                self.collision_disabled_timer = 50;
            }
        } else if if self.bgc() == 1 { self.move_and_fall(play) } else { self.move_(play) } {
            let loop_mode = self.loop_mode();
            if loop_mode == ENGOROIWA_LOOPMODE_ONEWAY_BREAK && (self.next_waypoint == 0 || self.next_waypoint == self.end_waypoint) {
                Self::spawn_fragments(play);
            }
            self.next_waypoint(play);
            if loop_mode == ENGOROIWA_LOOPMODE_ROUNDTRIP && (self.current_waypoint == 0 || self.current_waypoint == self.end_waypoint) {
                self.setup_wait();
            } else if self.bgc() == 0 && self.current_waypoint != 0 && self.current_waypoint != self.end_waypoint {
                let ascend = self.get_ascend_direction(play);
                if ascend > 0 {
                    self.setup_move_up();
                } else if ascend < 0 {
                    self.setup_move_down();
                } else {
                    self.setup_roll();
                }
            } else {
                self.setup_roll();
            }
        }
        audio_play_actor_sfx2(play, NA_SE_EV_BIGBALL_ROLL - SFX_FLAG);
    }

    /// `EnGoroiwa_SetupMoveAndFallToGround`: a hop back, at 0.15 of the speed.
    fn setup_move_and_fall_to_ground(&mut self) {
        self.action = Action::MoveAndFallToGround;
        self.update_flags(ENGOROIWA_ENABLE_OC);
        self.actor.gravity = -0.86;
        self.actor.min_velocity_y = -15.0;
        self.actor.speed_xz *= 0.15;
        self.actor.velocity.y = 5.0;
        self.roll_rot_speed = 1.0;
    }

    /// `EnGoroiwa_MoveAndFallToGround`: on the ground again, back on its way if Link was ahead
    /// (for `home.rot.z` bit 0), then the wait.
    fn move_and_fall_to_ground(&mut self, play: &PlayState) {
        self.move_and_fall(play);
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 && self.actor.velocity.y < 0.0 {
            if self.state_flags & ENGOROIWA_PLAYER_IN_THE_WAY != 0 && self.rot_z_bit() {
                self.reverse_direction();
                self.face_next_waypoint(play);
            }
            self.setup_wait();
        }
    }

    /// `EnGoroiwa_SetupWait`.
    fn setup_wait(&mut self) {
        self.action = Action::Wait;
        self.actor.speed_xz = 0.0;
        self.update_flags(ENGOROIWA_ENABLE_OC);
        self.wait_timer = WAIT_DURATIONS[(self.actor.home_rot.z & 1) as usize];
        self.roll_rot_speed = 0.0;
    }

    /// `EnGoroiwa_Wait`.
    fn wait(&mut self) {
        if self.wait_timer > 0 {
            self.wait_timer -= 1;
        } else {
            self.collider.base.at_flags &= !AT_HIT;
            self.setup_roll();
        }
    }

    /// `EnGoroiwa_SetupMoveUp`.
    fn setup_move_up(&mut self) {
        self.action = Action::MoveUp;
        self.update_flags(ENGOROIWA_ENABLE_AT | ENGOROIWA_ENABLE_OC);
        self.roll_rot_speed = 0.0;
        self.actor.velocity.y = self.actor.speed_xz.abs() * 0.1;
    }

    /// `EnGoroiwa_MoveUp`.
    fn move_up(&mut self, play: &mut PlayState) {
        if self.collider.base.at_flags & AT_HIT != 0 {
            self.collider.base.at_flags &= !AT_HIT;
            self.knock_down_player(play, 4);
            Self::body_hit_sfx(play);
            if self.rot_z_bit() {
                self.collision_disabled_timer = 50;
            }
        } else if self.move_up_to_next_waypoint(play) {
            self.next_waypoint(play);
            self.setup_roll();
            self.actor.speed_xz = 0.0;
        }
    }

    /// `EnGoroiwa_SetupMoveDown`.
    fn setup_move_down(&mut self) {
        self.action = Action::MoveDown;
        self.update_flags(ENGOROIWA_ENABLE_AT | ENGOROIWA_ENABLE_OC);
        self.roll_rot_speed = 0.3;
        self.bounce_count = 0;
        self.actor.velocity.y = self.actor.speed_xz.abs() * -0.3;
        self.state_flags |= ENGOROIWA_RETAIN_ROT_SPEED;
        self.state_flags &= !ENGOROIWA_IN_WATER;
    }

    /// `EnGoroiwa_MoveDown`.
    fn move_down(&mut self, play: &mut PlayState) {
        if self.collider.base.at_flags & AT_HIT != 0 {
            self.collider.base.at_flags &= !AT_HIT;
            self.knock_down_player(play, 4);
            Self::body_hit_sfx(play);
            if self.rot_z_bit() {
                self.collision_disabled_timer = 50;
            }
        } else if self.move_down_to_next_waypoint(play) {
            self.next_waypoint(play);
            self.setup_roll();
            self.state_flags &= !ENGOROIWA_RETAIN_ROT_SPEED;
            self.actor.speed_xz = 0.0;
        }
    }
}

/// `PLAYER_STATE1_6`, `_7`, `_28`, `_29`: while Player talks, dies, or is held by a cutscene or
/// a transition, the boulder stops.
const PLAYER_STATE1_HOLD: u32 = (1 << 6) | (1 << 7) | (1 << 28) | (1 << 29);

impl ActorImpl for EnGoroiwa {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnGoroiwa_Update`.
    fn update(&mut self, play: &mut PlayState) {
        let state1 = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map(|pi| pi.state_flags1()).unwrap_or(0);
        if state1 & PLAYER_STATE1_HOLD != 0 {
            return;
        }
        if self.collision_disabled_timer > 0 {
            self.collision_disabled_timer -= 1;
        }
        match self.action {
            Action::Roll => self.roll(play),
            Action::MoveAndFallToGround => self.move_and_fall_to_ground(play),
            Action::Wait => self.wait(),
            Action::MoveUp => self.move_up(play),
            Action::MoveDown => self.move_down(play),
        }
        match self.bgc() {
            1 => self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4),
            _ => {
                // BgCheck_EntityRaycastDown4.
                let (y, poly) = play.col.entity_raycast_down(self.actor.world_pos);
                self.actor.floor_height = y;
                self.actor.floor_poly = poly;
            }
        }
        self.update_rotation();
        if self.actor.xz_dist_to_player < 300.0 {
            self.update_collider();
            if self.state_flags & ENGOROIWA_ENABLE_AT != 0 && self.collision_disabled_timer <= 0 {
                play.collision_check_set_at(&self.actor, 0, &mut self.collider);
            }
            if self.state_flags & ENGOROIWA_ENABLE_OC != 0 && self.collision_disabled_timer <= 0 {
                play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
            }
        }
    }
    /// `EnGoroiwa_Draw`: `Gfx_DrawDListOpa(play, gRollingRockDL)`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        crate::gfx_draw_dlist_opa(out, OBJECT, "gRollingRockDL", rs);
    }
    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0).then_some(ColliderMut::JntSph(&mut self.collider))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
