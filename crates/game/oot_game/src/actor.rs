//! The base `Actor` (`actor.h`) every actor embeds, and the parts of `z_actor.c` that act on
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

// `ACTOR_FLAG_*` (`actor.h`), with what they mean where the decomp knows.
/// Targetable.
pub const ACTOR_FLAG_ATTENTION_ENABLED: u32 = 1 << 0;
/// Hostile (with `ACTOR_FLAG_ATTENTION_ENABLED`: Z-targeting locks on).
pub const ACTOR_FLAG_HOSTILE: u32 = 1 << 2;
/// Talked to (`Npc_UpdateTalking`'s NPCs; with `ACTOR_FLAG_ATTENTION_ENABLED` Z-targeting treats the actor as
/// friendly).
pub const ACTOR_FLAG_FRIENDLY: u32 = 1 << 3;
/// Updates even when not in view (`Actor_UpdateAll` updates actors with flag 4 or 6).
pub const ACTOR_FLAG_UPDATE_CULLING_DISABLED: u32 = 1 << 4;
/// Drawn even when not in view.
pub const ACTOR_FLAG_DRAW_CULLING_DISABLED: u32 = 1 << 5;
/// In view this frame (set by `Actor_CullingVolumeTest`'s culling).
pub const ACTOR_FLAG_INSIDE_CULLING_VOLUME: u32 = 1 << 6;
/// Seen only with the Lens of Truth (drawn in `Actor_DrawAll`'s lens pass: not ported).
pub const ACTOR_FLAG_REACT_TO_LENS: u32 = 1 << 7;
/// A talk request was made (`Actor_TalkOfferAccepted` answers it).
pub const ACTOR_FLAG_TALK: u32 = 1 << 8;
/// Talking starts without A (`Player_ActionHandler_Talk`: the actor's offer is taken at once).
pub const ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED: u32 = 1 << 16;
/// With `ACTOR_FLAG_ATTENTION_ENABLED`: Navi can be asked about it with C-Up (`Player_ActionHandler_Talk`).
pub const ACTOR_FLAG_TALK_WITH_C_UP: u32 = 1 << 18;
/// `ACTOR_FLAG_SFX_ACTOR_POS_2`, `_20`, `_21`, `_28`: how `Actor_DrawAll` plays `actor->sfx`
/// (`Actor_UpdateFlaggedAudio`).
pub const ACTOR_FLAG_SFX_ACTOR_POS_2: u32 = 1 << 19;
pub const ACTOR_AUDIO_FLAG_SFX_CENTERED_1: u32 = 1 << 20;
pub const ACTOR_AUDIO_FLAG_SFX_CENTERED_2: u32 = 1 << 21;
pub const ACTOR_FLAG_SFX_TIMER: u32 = 1 << 28;
/// `Actor_Draw` binds no point lights for it (`Lights_BindAll` with no position).
pub const ACTOR_FLAG_IGNORE_POINT_LIGHTS: u32 = 1 << 22;
pub const ACTOR_FLAG_THROW_ONLY: u32 = 1 << 23;
pub const ACTOR_FLAG_SFX_FOR_PLAYER_BODY_HIT: u32 = 1 << 24;
pub const ACTOR_FLAG_UPDATE_DURING_OCARINA: u32 = 1 << 25;
pub const ACTOR_FLAG_CAN_PRESS_SWITCHES: u32 = 1 << 26;
/// Can't be targeted by the arrow-pointed search (`Attention_WeightedDistToPlayerSq`).
pub const ACTOR_FLAG_LOCK_ON_DISABLED: u32 = 1 << 27;

// `colorFilterParams` (`actor.h`): `Actor_SetColorFilter`'s colour, intensity, list and duration.
pub const COLORFILTER_COLORFLAG_GRAY: u16 = 0x8000;
pub const COLORFILTER_COLORFLAG_RED: u16 = 0x4000;
pub const COLORFILTER_COLORFLAG_BLUE: u16 = 0x0000;
pub const COLORFILTER_INTENSITY_FLAG: i16 = 0x8000u16 as i16;
pub const COLORFILTER_BUFFLAG_XLU: u16 = 0x2000;
pub const COLORFILTER_BUFFLAG_OPA: u16 = 0x0000;

/// `COLORFILTER_GET_COLORINTENSITY`.
pub fn colorfilter_get_colorintensity(params: u16) -> u8 {
    ((params & 0x1F00) >> 5) as u8
}

/// `COLORFILTER_GET_DURATION`.
pub fn colorfilter_get_duration(params: u16) -> u16 {
    params & 0xFF
}

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
    /// Yaw added by a rotating platform this frame (`DynaPolyActor_UpdateCarriedActorRotY`; Player adds it to
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
    /// `targetMode`: the targeting range class (`sAttentionRanges`).
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
    /// `sfx`: a sound the actor asks `Actor_DrawAll` to play this frame (`Actor_PlaySfx_Flagged2` and
    /// the rest, `Actor_UpdateFlaggedAudio`); `Actor_UpdateAll` clears it first.
    pub sfx: u16,
    /// `colorFilterParams`, `colorFilterTimer`: `Actor_SetColorFilter`'s tint (`Actor_Draw`'s fog),
    /// counted down by `Actor_UpdateAll` before each update.
    pub color_filter_params: u16,
    pub color_filter_timer: u8,
    /// `dropFlag`: what the last hit was (`Actor_SetDropFlag`): an arrow's or magic's kind, which
    /// an enemy's drop can depend on.
    pub drop_flag: u8,
    /// `naviEnemyId` (`NAVI_ENEMY_*`): what Navi says about the actor (`NAVI_ENEMY_NONE`, 0xFF, for
    /// none).
    pub navi_enemy_id: u8,
}

/// `NAVI_ENEMY_NONE`.
pub const NAVI_ENEMY_NONE: u8 = 0xFF;

impl Actor {
    /// A spawned actor as `Actor_Spawn` + `Actor_Init` set it up before the actor's own init:
    /// home = world = `pos`, shape rotation = world rotation, focus at the position, scale 0.01,
    /// `targetMode` 3, `minVelocityY` -20, `xyzDistToPlayerSq` `MAXFLOAT`, and
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
            color_filter_params: 0,
            color_filter_timer: 0,
            drop_flag: 0,
            // Actor_Init: actor->naviEnemyId = NAVI_ENEMY_NONE.
            navi_enemy_id: NAVI_ENEMY_NONE,
        }
    }

    /// `Actor_ApplyDamage`: this frame's damage off `colChkInfo.health`, down to 0. Returns the
    /// health left.
    pub fn apply_damage(&mut self) -> u8 {
        let c = &mut self.col_chk_info;
        if c.health <= c.damage {
            c.health = 0;
        } else {
            c.health -= c.damage;
        }
        c.health
    }

    /// `Actor_SetColorFilter`: tint the actor `color_flag` (`COLORFILTER_COLORFLAG_*`) at up to
    /// `color_intensity_max`, in the OPA or XLU list (`buf_flag`), for `duration` frames.
    ///
    /// @bug (game): the gray check compares the s16 `colorFlag` with 0x8000, which an s16 can't
    /// hold, so the light arrow's sound never plays.
    pub fn set_color_filter(&mut self, color_flag: u16, color_intensity_max: i16, buf_flag: u16, duration: u16) {
        self.color_filter_params = color_flag | buf_flag | (((color_intensity_max & 0xF8) as u16) << 5) | duration;
        self.color_filter_timer = duration as u8;
    }

    /// `Actor_SetDropFlagJntSph`: `dropFlag` from every sphere's hit, the last element first (an
    /// ice or fire magic hit with `freeze_flag` freezes the actor for the hit's damage in frames).
    pub fn set_drop_flag_jnt_sph(&mut self, jnt_sph: &crate::collision_check::ColliderJntSph, freeze_flag: bool) {
        self.drop_flag = 0;
        for e in jnt_sph.elements.iter().rev() {
            let flag = match e.info.ac_hit_elem {
                None => 0,
                Some(h) => self.drop_flag_of(&h.at_dmg_info, freeze_flag),
            };
            self.drop_flag |= flag;
        }
    }

    /// `Actor_SetDropFlag`: `dropFlag` from one element's hit.
    pub fn set_drop_flag(&mut self, elem: &crate::collision_check::ColliderElement, freeze_flag: bool) {
        self.drop_flag = match elem.ac_hit_elem {
            None => 0,
            Some(h) => self.drop_flag_of(&h.at_dmg_info, freeze_flag),
        };
    }

    /// `Actor_SetDropFlag`'s and `Actor_SetDropFlagJntSph`'s flag for one hit.
    fn drop_flag_of(&mut self, at: &crate::collision_check::ColliderElementDamageInfoAT, freeze_flag: bool) -> u8 {
        use crate::collision_check::*;
        let f = at.dmg_flags;
        if freeze_flag && f & (DMG_UNKNOWN_1 | DMG_MAGIC_ICE | DMG_MAGIC_FIRE) != 0 {
            self.freeze_timer = at.damage as u16;
            0x00
        } else if f & DMG_ARROW_FIRE != 0 {
            0x01
        } else if f & DMG_ARROW_ICE != 0 {
            0x02
        } else if f & DMG_ARROW_UNK1 != 0 {
            0x04
        } else if f & DMG_ARROW_UNK2 != 0 {
            0x08
        } else if f & DMG_ARROW_UNK3 != 0 {
            0x10
        } else if f & DMG_ARROW_LIGHT != 0 {
            0x20
        } else if f & DMG_MAGIC_LIGHT != 0 {
            if freeze_flag {
                self.freeze_timer = at.damage as u16;
            }
            0x40
        } else {
            0x00
        }
    }

    /// `Actor_PlaySfx_Flagged2`: `sfx` played at the actor (`Audio_PlaySfxGeneral` at `projectedPos`).
    pub fn play_sfx_flagged2(&mut self, sfx_id: u16) {
        self.sfx = sfx_id;
        self.flags |= ACTOR_FLAG_SFX_ACTOR_POS_2;
        self.flags &= !(ACTOR_AUDIO_FLAG_SFX_CENTERED_1 | ACTOR_AUDIO_FLAG_SFX_CENTERED_2 | ACTOR_FLAG_SFX_TIMER);
    }

    /// `Actor_PlaySfx_FlaggedCentered1`: `sfx` with no position (`Sfx_PlaySfxCentered`).
    pub fn play_sfx_flagged_centered1(&mut self, sfx_id: u16) {
        self.sfx = sfx_id;
        self.flags |= ACTOR_AUDIO_FLAG_SFX_CENTERED_1;
        self.flags &= !(ACTOR_FLAG_SFX_ACTOR_POS_2 | ACTOR_AUDIO_FLAG_SFX_CENTERED_2 | ACTOR_FLAG_SFX_TIMER);
    }

    /// `Actor_PlaySfx_FlaggedCentered2`: `sfx` with no position (`Sfx_PlaySfxCentered2`).
    pub fn play_sfx_flagged_centered2(&mut self, sfx_id: u16) {
        self.sfx = sfx_id;
        self.flags |= ACTOR_AUDIO_FLAG_SFX_CENTERED_2;
        self.flags &= !(ACTOR_FLAG_SFX_ACTOR_POS_2 | ACTOR_AUDIO_FLAG_SFX_CENTERED_1 | ACTOR_FLAG_SFX_TIMER);
    }

    /// `Actor_PlaySfx_Flagged`: `sfx` at the actor (`Sfx_PlaySfxAtPos`).
    pub fn play_sfx_flagged(&mut self, sfx_id: u16) {
        self.flags &= !(ACTOR_FLAG_SFX_ACTOR_POS_2 | ACTOR_AUDIO_FLAG_SFX_CENTERED_1 | ACTOR_AUDIO_FLAG_SFX_CENTERED_2 | ACTOR_FLAG_SFX_TIMER);
        self.sfx = sfx_id;
    }

    /// `Actor_PlaySfx_FlaggedTimer`: the timer's tick (`Actor_DrawAll` plays `NA_SE_SY_TIMER` through
    /// `func_800F4C58` with `sfx - 1`): the C uses the ids as numbers, `NA_SE_PL_WALK_DIRT -
    /// SFX_FLAG` (3) under 40, `_CONCRETE`'s (2) under 100, else `_SAND`'s (1).
    pub fn play_sfx_flagged_timer(&mut self, arg1: i32) {
        use crate::audio::sfx::{NA_SE_PL_WALK_CONCRETE, NA_SE_PL_WALK_DIRT, NA_SE_PL_WALK_SAND, SFX_FLAG};
        self.flags |= ACTOR_FLAG_SFX_TIMER;
        self.flags &= !(ACTOR_FLAG_SFX_ACTOR_POS_2 | ACTOR_AUDIO_FLAG_SFX_CENTERED_1 | ACTOR_AUDIO_FLAG_SFX_CENTERED_2);
        self.sfx = if arg1 < 40 {
            NA_SE_PL_WALK_DIRT - SFX_FLAG
        } else if arg1 < 100 {
            NA_SE_PL_WALK_CONCRETE - SFX_FLAG
        } else {
            NA_SE_PL_WALK_SAND - SFX_FLAG
        };
    }

    /// Targetable and hostile: Z-targeting locks on (`ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE`).
    pub fn is_hostile(&self) -> bool {
        self.flags & (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE) == ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE
    }

    /// `Actor_Kill`.
    /// `Actor_WorldToActorCoords`: `pos` in the actor's frame (its shape yaw,
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
        self.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
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

    /// `Actor_UpdateVelocityXZGravity`.
    pub fn update_velocity(&mut self) {
        self.velocity.x = sin_s(self.world_rot.y) * self.speed_xz;
        self.velocity.z = cos_s(self.world_rot.y) * self.speed_xz;
        self.velocity.y += self.gravity;
        if self.velocity.y < self.min_velocity_y {
            self.velocity.y = self.min_velocity_y;
        }
    }

    /// `Actor_UpdatePos`.
    pub fn update_pos(&mut self) {
        self.world_pos += self.velocity * UPDATE_SCALE + self.col_chk_info.displacement;
    }

    /// `Actor_MoveXZGravity`.
    pub fn move_forward(&mut self) {
        self.update_velocity();
        self.update_pos();
    }

    /// `Actor_UpdateVelocityXYZ`: `speed` along the world yaw and pitch (a positive pitch up).
    pub fn update_velocity_xyz(&mut self) {
        let speed_xz = self.speed_xz * cos_s(self.world_rot.x);
        self.velocity.x = speed_xz * sin_s(self.world_rot.y);
        self.velocity.y = self.speed_xz * sin_s(self.world_rot.x);
        self.velocity.z = speed_xz * cos_s(self.world_rot.y);
    }

    /// `Actor_MoveXYZ`.
    pub fn move_xyz(&mut self) {
        self.update_velocity_xyz();
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
        // DynaPolyActor_TransformCarriedActor: ride the platform we're standing on.
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
            // BgCheck_GetWaterSurfaceAllHack (ripples not modelled).
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
