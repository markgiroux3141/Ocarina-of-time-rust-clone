//! `En_Bombf` (`ovl_En_Bombf/z_en_bombf.c`): a Bomb Flower, and the bomb it grows. Every
//! placement is the flower (params -1); it spawns its bomb (params 0) as a separate actor, which
//! explodes itself (params 1) rather than through an `En_Bom`.
//!
//! The flower, grown, offers its bomb to be picked up (`Actor_OfferCarry`). Link without strength
//! only tugs at it (`Player_GetStrength`, `func_8083A0F4`); with the Goron's Bracelet he lifts it:
//! the flower, finding itself held, spawns its bomb in its place and hands Link's carry on to it
//! (`func_8002F5C4`), and grows a new one (180 frames, then a twentieth of its size a frame). A hit
//! (an explosion, an arrow, Din's Fire; not from a boss) spawns a bomb that blows at once, and a
//! burning Deku Stick at it one with 100 frames on its fuse.
//!
//! The bomb is an explosive (`Actor_ChangeCategory` in its init): a fuse of 140 frames, sparks
//! (`Effect_Ss_G_Spk`) and smoke under 127, a green-tinted flash quickening at 70, 50, 30 and 3;
//! carried, thrown or put down like any; set off at once by a hit or an enemy's touch, put out by
//! deep water. At 0 the blast as `En_Bom`'s (scale step 19), then ten frames of a
//! `DMG_EXPLOSIVE` sphere (damage 8) growing 8 a frame round where it is.
//!
//! The circle shadow (12) is the shape's (`ActorShape_Init`'s, drawn by `Actor_Draw`).
//!
//! Not ported: `Rumble_Request` (BACKLOG #60), `DREG(6)` (a debug register set round its floor
//! checks).
//!
//! Params: `type` (`EnBombfType`: -1 `BOMBFLOWER_FLOWER`, every placement; 0 `BOMBFLOWER_BODY`,
//! the bomb it spawns; 1 `BOMBFLOWER_EXPLOSION` once that blows). `shape.rot.x` tilts a flower
//! on a wall or a ceiling (Dodongo's Cavern's), as a rotation.
//! Flags: none.

use glam::{Mat4, Vec3};
use oot_game::actor::{
    ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, BGCHECKFLAG_CEILING, BGCHECKFLAG_GROUND, BGCHECKFLAG_GROUND_TOUCH, BGCHECKFLAG_WALL,
    BGCHECKFLAG_WATER_TOUCH, UPDBGCHECKINFO_FLAG_0, UPDBGCHECKINFO_FLAG_1, UPDBGCHECKINFO_FLAG_2, UPDBGCHECKINFO_FLAG_3, UPDBGCHECKINFO_FLAG_4,
};
use oot_game::actor_ctx::{ACTORCAT_BG, ACTORCAT_BOSS, ACTORCAT_ENEMY, ACTORCAT_EXPLOSIVE, ACTORCAT_PROP, ActorImpl, ActorProfile, actor_find_nearby, actor_play_sfx_surface_bomb, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Kind, Source};
use oot_game::audio::sfx::{NA_SE_EV_BOMB_BOUND, NA_SE_EV_BOMB_DROP_WATER, NA_SE_IT_BOMB_EXPLOSION, NA_SE_IT_BOMB_IGNIT, NA_SE_IT_BOMB_UNEXPLOSION, NA_SE_PL_PULL_UP_ROCK, SFX_FLAG};
use oot_game::collision_check::*;
use oot_game::effect::dead_sound::DEADSOUND_REPEAT_MODE_OFF;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

use crate::en_bom::{fade_explosion_light, flash_explosion_light, release_from_player, ss_actor};

pub const ACTOR_EN_BOMBF: i16 = 0x004C;
/// `ACTOR_BG_DDAN_KD` (`actor_table.h`: 0x005C): Dodongo's Cavern's falling stairs.
const ACTOR_BG_DDAN_KD: i16 = 0x005C;
/// `OBJECT_BOMBF`'s file.
const OBJECT: &str = "object_bombf";

/// `En_Bombf_Profile`.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_EN_BOMBF, name: "En_Bombf", category: ACTORCAT_PROP, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `EnBombfType`: the whole params.
pub const TYPE: Field = Field::whole(
    "type",
    Kind::Named(&[(-1, "BOMBFLOWER_FLOWER"), (0, "BOMBFLOWER_BODY"), (1, "BOMBFLOWER_EXPLOSION")]),
    "the flower (every placement); the bomb it spawns; that bomb blowing",
);

/// `shape.rot.x`: a flower on a wall (0x4000) or a ceiling (0x8000), drawn tilted.
pub const ROT_X: Field = Field::rot("shape.rot.x", Source::RotX, 0, 16, Kind::Rotation, "the flower's tilt: on a wall (0x4000) or a ceiling (0x8000)");

/// En_Bombf's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[TYPE, ROT_X], flags: &[], todo: false };

/// `EnBombfType` (`BOMBFLOWER_FLOWER` is -1, every other value below 0 drawn as it).
const BOMBFLOWER_BODY: i16 = 0;
const BOMBFLOWER_EXPLOSION: i16 = 1;

/// `sCylinderInit`: what sets it off (`0x0003F828`, as a bomb's).
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER | AC_TYPE_OTHER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK2,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0003_F828, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: eng_collision::math3d::Cylinder16 { radius: 9, height: 18, y_shift: 10, pos: [0; 3] },
};

/// `sJntSphElementsInit`: the explosion, `DMG_EXPLOSIVE` damage 8.
const JNT_SPH_ELEMENTS: [ColliderJntSphElementInit; 1] = [ColliderJntSphElementInit {
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0008, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x08 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_NONE,
    },
    limb: 0,
    model_sphere: eng_collision::math3d::Sphere16 { center: [0, 0, 0], radius: 0 },
    scale: 100,
}];

/// `sJntSphInit`.
const JNT_SPH_INIT: ColliderInit = ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_ON | AT_TYPE_ALL, ac_flags: AC_NONE, oc_flags1: OC1_NONE, oc_flags2: OC2_NONE, shape: COLSHAPE_JNTSPH };

/// The colliders' ids (`collider_mut`).
const COL_BOMB: u8 = 0;
const COL_EXPLOSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    /// `EnBombf_GrowBomb`.
    GrowBomb,
    /// `EnBombf_Move`.
    Move,
    /// `EnBombf_WaitForRelease`.
    WaitForRelease,
    /// `EnBombf_Explode`.
    Explode,
}

pub struct EnBombf {
    pub actor: Actor,
    pub bomb_collider: ColliderCylinder,
    pub explosion_collider: ColliderJntSph,
    /// `timer`: the flower's regrowth, the bomb's fuse, then its explosion's ten frames.
    pub timer: i16,
    action: Action,
    /// `isFuseEnabled`: the timer counts down.
    pub is_fuse_enabled: bool,
    pub collider_set_oc: bool,
    pub flash_speed_scale: i16,
    pub flash_intensity: f32,
    /// `flowerBombScale`: the flower's bomb's size (1 grown).
    pub flower_bomb_scale: f32,
}

impl EnBombf {
    /// `EnBombf_Init`: scale 0.01, the fuse on; drawn culled but near `Bg_Ddan_Kd` (the falling
    /// stairs' fuse of flowers); 10 across and 10 high, the nearest attention range. The bomb: its
    /// model 1000 up, a fuse of 140, gravity -1.5, an explosive, mass 200, not a target. The
    /// flower: immovable, its bomb grown. Both seen from 31000 further.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        actor.scale = Vec3::splat(0.01);
        let ty = TYPE.get(actor.params);
        let bomb_collider = ColliderCylinder::new(&CYLINDER_INIT);
        let explosion_collider = ColliderJntSph::new(&JNT_SPH_INIT, &JNT_SPH_ELEMENTS);
        // ActorShape_Init(&shape, shapeUnk10, ActorShadow_DrawCircle, 12): shapeUnk10 1000 for the bomb.
        actor.shape_init(if ty == BOMBFLOWER_BODY { 1000.0 } else { 0.0 }, oot_game::shadow::ShadowDraw::Circle, 12.0);
        actor.focus_pos = actor.world_pos;
        if actor_find_nearby(play, &actor, None, ACTOR_BG_DDAN_KD, ACTORCAT_BG, 10000.0).is_some() {
            actor.flags |= ACTOR_FLAG_DRAW_CULLING_DISABLED;
        }
        actor.col_chk_info.cyl_radius = 10;
        actor.col_chk_info.cyl_height = 10;
        // ATTENTION_RANGE_0.
        actor.target_mode = 0;
        let mut this = EnBombf {
            actor,
            bomb_collider,
            explosion_collider,
            timer: 0,
            action: Action::GrowBomb,
            is_fuse_enabled: true,
            collider_set_oc: false,
            flash_speed_scale: 0,
            flash_intensity: 0.0,
            flower_bomb_scale: 0.0,
        };
        if ty == BOMBFLOWER_BODY {
            this.timer = 140;
            this.flash_speed_scale = 15;
            this.actor.gravity = -1.5;
            // Actor_ChangeCategory(ACTORCAT_EXPLOSIVE): spawned into its new list.
            this.actor.category = ACTORCAT_EXPLOSIVE;
            this.actor.col_chk_info.mass = 200;
            this.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
            this.action = Action::Move;
        } else {
            this.actor.col_chk_info.mass = MASS_IMMOVABLE;
            this.collider_set_oc = true;
            this.flower_bomb_scale = 1.0;
            this.action = Action::GrowBomb;
        }
        this.actor.culling_volume_scale += 31000.0;
        this.actor.culling_volume_distance += 31000.0;
        Box::new(this)
    }

    fn ty(&self) -> i16 {
        TYPE.get(self.actor.params)
    }

    /// `Actor_Spawn(ACTOR_EN_BOMBF, world.pos, 0, 0, 0, BOMBFLOWER_BODY)`: the flower's bomb.
    fn spawn_bomb(&self, play: &mut PlayState) -> Option<oot_game::actor_ctx::ActorHandle> {
        play.actor_spawn(ACTOR_EN_BOMBF, self.actor.world_pos, [0; 3], BOMBFLOWER_BODY).map_err(|e| log::debug!("En_Bombf: no bomb: {e:?}")).ok()
    }

    /// `EnBombf_GrowBomb`: the flower.
    /// - Grown and held (Link lifted it): its bomb spawned and Link's carry handed on to it
    ///   (`func_8002F5C4`), a new one to grow (180 frames), `NA_SE_PL_PULL_UP_ROCK`, not a target;
    ///   if the spawn fails, Link lets go and carries nothing.
    /// - Grown and hit (but by a boss: King Dodongo's roll): a bomb that blows at once.
    /// - Grown, with a burning stick at it: a bomb with 100 on its fuse. Else, not held, it offers
    ///   itself to be picked up; held, Link lets go and it's back home.
    /// - Growing: from the timer's end a twentieth a frame (a target again once grown); held,
    ///   Link lets go and it's back home.
    fn grow_bomb(&mut self, play: &mut PlayState) {
        if self.flower_bomb_scale >= 1.0 {
            if self.actor.parent.is_some() {
                if let Some(h) = self.spawn_bomb(play) {
                    oot_game::actor_ctx::func_8002f5c4(play, &mut self.actor, h);
                    self.timer = 180;
                    self.flower_bomb_scale = 0.0;
                    audio_play_actor_sfx2(play, NA_SE_PL_PULL_UP_ROCK);
                    self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
                } else {
                    self.let_go_of_player(play);
                }
            } else if self.bomb_collider.base.ac_flags & AC_HIT != 0 {
                self.bomb_collider.base.ac_flags &= !AC_HIT;
                // A deleted attacker reads as not a boss.
                let by_boss = self.bomb_collider.base.ac.and_then(|h| play.actors.actor(h)).is_some_and(|a| a.category == ACTORCAT_BOSS);
                if !by_boss && let Some(h) = self.spawn_bomb(play) {
                    if let Some(b) = play.actors.downcast_mut::<EnBombf>(h) {
                        b.is_fuse_enabled = true;
                        b.timer = 0;
                    }
                    self.timer = 180;
                    self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
                    self.flower_bomb_scale = 0.0;
                }
            } else if crate::player_is_burning_stick_in_range(play, self.actor.world_pos, 30.0, 50.0) {
                if let Some(h) = self.spawn_bomb(play) {
                    if let Some(b) = play.actors.downcast_mut::<EnBombf>(h) {
                        b.timer = 100;
                    }
                    self.timer = 180;
                    self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
                    self.flower_bomb_scale = 0.0;
                }
            } else if self.actor.parent.is_none() {
                let a = self.actor.clone();
                oot_game::get_item::offer_pick_up(play, &a);
            } else {
                self.let_go_of_player(play);
                self.actor.world_pos = self.actor.home_pos;
            }
        } else {
            if self.timer == 0 {
                self.flower_bomb_scale += 0.05;
                if self.flower_bomb_scale >= 1.0 {
                    self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
                }
            }
            if self.actor.parent.is_some() {
                self.let_go_of_player(play);
                self.actor.world_pos = self.actor.home_pos;
            }
        }
    }

    /// The flower's own letting go of Link (`EnBombf_GrowBomb`): his `actor.child`, `heldActor`
    /// and `interactRangeActor` cleared, its `parent`, his carrying flag off, whatever he held.
    fn let_go_of_player(&mut self, play: &mut PlayState) {
        if let Some(pi) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
            pi.clear_carried();
        }
        self.actor.parent = None;
    }

    /// `EnBombf_Move`: the bomb. Taken up it waits to be let go, in no room (its flower size 0,
    /// unused). Otherwise its size 1 (unused); it slows (by up to 0.025 in the air, 1.5 on the
    /// ground); landing faster than 6 it bounces (`vy × -0.5`, the surface's sound), else on the
    /// ground with 4 or more on the fuse it offers itself to be picked up.
    fn move_(&mut self, play: &mut PlayState) {
        if self.actor.parent.is_some() {
            self.flower_bomb_scale = 0.0;
            self.action = Action::WaitForRelease;
            self.actor.room = -1;
            return;
        }
        self.flower_bomb_scale = 1.0;
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND == 0 {
            eng_math::smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 1.0, 0.025, 0.0);
        } else {
            eng_math::smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 1.0, 1.5, 0.0);
            if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 && self.actor.velocity.y < -6.0 {
                actor_play_sfx_surface_bomb(play, &self.actor);
                self.actor.velocity.y *= -0.5;
            } else if self.timer >= 4 {
                let a = self.actor.clone();
                oot_game::get_item::offer_pick_up(play, &a);
            }
        }
    }

    /// `EnBombf_WaitForRelease`: let go, it moves on this frame; held, it doesn't fall.
    fn wait_for_release(&mut self, play: &mut PlayState) {
        if self.actor.parent.is_none() {
            self.action = Action::Move;
            self.move_(play);
        } else {
            self.actor.velocity.y = 0.0;
        }
    }

    /// `EnBombf_Explode`: the sphere grows 8 a frame (its first frame turns draw culling off; the
    /// rumble isn't ported), its hits registered while it's an explosion; the light flash fades;
    /// at 0 it lets go of Link if he holds it, and is gone.
    fn explode(&mut self, play: &mut PlayState) {
        let dim = &mut self.explosion_collider.elements[0].dim;
        if dim.model_sphere.radius == 0 {
            self.actor.flags |= ACTOR_FLAG_DRAW_CULLING_DISABLED;
            // Rumble_Request(xzDistToPlayer, 255, 20, 150): not ported (BACKLOG #60).
        }
        dim.model_sphere.radius = dim.model_sphere.radius.wrapping_add(8);
        dim.world_sphere.radius = dim.model_sphere.radius;
        if self.ty() == BOMBFLOWER_EXPLOSION {
            play.collision_check_set_at(&self.actor, COL_EXPLOSION, &mut self.explosion_collider);
        }
        fade_explosion_light(play);
        if self.timer == 0 {
            let me = play.cur_actor;
            release_from_player(play, me);
            self.actor.kill();
        }
    }

    /// The bomb rebounding off a wall it moves into (`EnBombf_Update`): the yaw mirrored off the
    /// wall's when it hit more than square on, `NA_SE_EV_BOMB_BOUND`, a step and its floor checks
    /// again, at 0.7 the speed.
    fn rebound(&mut self, play: &mut PlayState) {
        if (self.actor.wall_yaw.wrapping_sub(self.actor.world_rot.y) as i32).abs() > 0x4000 {
            self.actor.world_rot.y = self.actor.wall_yaw.wrapping_sub(self.actor.world_rot.y).wrapping_add(self.actor.wall_yaw).wrapping_sub(0x8000u16 as i16);
        }
        audio_play_actor_sfx2(play, NA_SE_EV_BOMB_BOUND);
        self.actor.move_forward();
        self.update_bg_check(play);
        self.actor.speed_xz *= 0.7;
        self.actor.bg_check_flags &= !BGCHECKFLAG_WALL;
    }

    /// `Actor_UpdateBgCheckInfo(5, 10, 0, walls, ceiling, floor and water, 3, 4)` (with `DREG(6)`
    /// set round it, a debug register).
    fn update_bg_check(&mut self, play: &PlayState) {
        self.actor.update_bg_check_info(&play.col, 5.0, 10.0, 0.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_1 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
    }
}

/// The bake names: the flower's leaves, and its bomb with its spark (the spark facing the camera
/// through the matrix on segment 8, the bomb's flash dynamic).
const BAKE_FLOWER: &str = "En_Bombf/flower";
const BAKE_BOMB: &str = "En_Bombf/bomb";
/// The bomb's segments: 8 the list `EnBombf_NewMtxDList` builds (`gSPMatrix` of the billboarded
/// matrix, then the end), 9 that matrix, 0x0B the colours.
const SEG_MTX_DL: u8 = 0x08;
const SEG_MTX: u8 = 0x09;
const SEG_COLOR: u8 = 0x0B;

/// `EnBombf_Draw`'s lists after `Gfx_SetupDL_25Opa`: `gBombFlowerLeavesDL` and
/// `gBombFlowerBaseLeavesDL`; `gBombFlowerBombAndSparkDL` with segment 8 a list loading the matrix
/// on bone 0 (`gSPMatrix(G_MTX_NOPUSH | G_MTX_LOAD | G_MTX_MODELVIEW)`: 0xDA380003) and its colours
/// dynamic.
pub fn bakes() -> Vec<MeshBake> {
    vec![
        MeshBake {
            name: BAKE_FLOWER.into(),
            object: OBJECT.into(),
            segments: vec![],
            prelude: vec![],
            body: BakeBody::DLists(vec![(OBJECT.into(), "gBombFlowerLeavesDL".into()), (OBJECT.into(), "gBombFlowerBaseLeavesDL".into())]),
        },
        MeshBake {
            name: BAKE_BOMB.into(),
            object: OBJECT.into(),
            segments: vec![
                (SEG_MTX_DL, BakeSegment::Commands(vec![(0xDA38_0003, (SEG_MTX as u32) << 24), (0xDF00_0000, 0)])),
                (SEG_MTX, BakeSegment::Matrix(0)),
                (SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true }),
            ],
            prelude: vec![SEG_COLOR],
            body: BakeBody::DLists(vec![(OBJECT.into(), "gBombFlowerBombAndSparkDL".into())]),
        },
    ]
}

/// `rs.values`: `flashIntensity`, `flowerBombScale`; `rs.switches`: the type plus 1.
mod rs {
    pub const FLASH: usize = 0;
    pub const BOMB_SCALE: usize = 1;
    pub const TYPE: usize = 0;
}

impl ActorImpl for EnBombf {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnBombf_Update`.
    fn update(&mut self, play: &mut PlayState) {
        let eff_velocity = Vec3::ZERO;
        let bomb2_accel = Vec3::new(0.0, 0.1, 0.0);
        let eff_accel = Vec3::ZERO;
        let mut dust_accel = Vec3::new(0.0, 0.6, 0.0);
        let dust_color = [255, 255, 255, 255];
        if self.is_fuse_enabled && self.timer != 0 {
            self.timer -= 1;
        }
        if !self.collider_set_oc && self.actor.parent.is_none() && (self.actor.xz_dist_to_player >= 20.0 || self.actor.y_dist_to_player.abs() >= 80.0) {
            self.collider_set_oc = true;
        }
        match self.action {
            Action::GrowBomb => self.grow_bomb(play),
            Action::Move => self.move_(play),
            Action::WaitForRelease => self.wait_for_release(play),
            Action::Explode => self.explode(play),
        }
        if self.ty() == BOMBFLOWER_BODY {
            self.actor.move_forward();
        }
        if self.actor.gravity != 0.0 {
            self.update_bg_check(play);
        }
        if self.ty() == BOMBFLOWER_BODY {
            if self.actor.velocity.y > 0.0 && self.actor.bg_check_flags & BGCHECKFLAG_CEILING != 0 {
                self.actor.velocity.y = -self.actor.velocity.y;
            }
            if self.actor.speed_xz != 0.0 && self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
                self.rebound(play);
            }
            let enemy_touch = self.bomb_collider.base.oc_flags1 & OC1_HIT != 0 && self.bomb_collider.base.oc.and_then(|h| play.actors.actor(h)).is_some_and(|a| a.category == ACTORCAT_ENEMY);
            if self.bomb_collider.base.ac_flags & AC_HIT != 0 || enemy_touch {
                self.is_fuse_enabled = true;
                self.timer = 0;
            } else if self.timer > 100 && crate::player_is_burning_stick_in_range(play, self.actor.world_pos, 30.0, 50.0) {
                self.timer = 100;
            }
            if self.is_fuse_enabled {
                dust_accel.y = 0.2;
                let mut eff_pos = self.actor.world_pos;
                eff_pos.y += 25.0;
                if self.timer < 127 {
                    // The spark on even frames, 25 up.
                    if play.gameplay_frames % 2 == 0 {
                        let a = ss_actor(play, &self.actor);
                        play.with_ss(|s| s.g_spk_spawn_fuse(a, eff_pos, eff_velocity, eff_accel));
                    }
                    audio_play_actor_sfx2(play, NA_SE_IT_BOMB_IGNIT - SFX_FLAG);
                    eff_pos.y += 3.0;
                    play.with_ss(|s| s.func_8002829c(eff_pos, eff_velocity, dust_accel, dust_color, dust_color, 50, 5));
                }
                if self.timer == 3 || self.timer == 30 || self.timer == 50 || self.timer == 70 {
                    self.flash_speed_scale >>= 1;
                }
                // 150 / flashSpeedScale: infinite once it's halved to 0, as the C's float division.
                let step = 150.0 / self.flash_speed_scale as f32;
                if self.timer < 100 && (self.timer & (self.flash_speed_scale + 1)) != 0 {
                    eng_math::smooth_step_to_f(&mut self.flash_intensity, 150.0, 1.0, step, 0.0);
                } else {
                    eng_math::smooth_step_to_f(&mut self.flash_intensity, 0.0, 1.0, step, 0.0);
                }
                if self.timer < 3 {
                    self.actor.scale = Vec3::splat(self.actor.scale.x + 0.002);
                }
                if self.timer == 0 {
                    let mut eff_pos = self.actor.world_pos;
                    eff_pos.y += 10.0;
                    if self.actor.parent.is_some() {
                        eff_pos.y += 30.0;
                    }
                    play.with_ss(|s| s.bomb2_spawn_layered(eff_pos, eff_velocity, bomb2_accel, 100, 19));
                    eff_pos.y = self.actor.floor_height;
                    if self.actor.floor_height > eng_collision::bgcheck::BGCHECK_Y_MIN {
                        play.with_ss(|s| s.blast_spawn_white_shockwave(eff_pos, eff_velocity, eff_accel));
                    }
                    audio_play_actor_sfx2(play, NA_SE_IT_BOMB_EXPLOSION);
                    flash_explosion_light(play);
                    play.camera_request_quake(oot_game::camera::CAM_ID_MAIN, 11, 8);
                    self.actor.params = BOMBFLOWER_EXPLOSION; // schema: the C's write, the bomb blowing
                    self.timer = 10;
                    self.actor.flags |= ACTOR_FLAG_DRAW_CULLING_DISABLED;
                    self.action = Action::Explode;
                }
            }
        }
        self.actor.focus_pos = self.actor.world_pos + Vec3::Y * 10.0;
        if self.ty() <= BOMBFLOWER_BODY {
            self.bomb_collider.update(&self.actor);
            if self.flower_bomb_scale >= 1.0 && self.collider_set_oc {
                play.collision_check_set_oc(&self.actor, COL_BOMB, &mut self.bomb_collider);
            }
            play.collision_check_set_ac(&self.actor, COL_BOMB, &mut self.bomb_collider);
        }
        if self.actor.scale.x >= 0.01 && self.ty() != BOMBFLOWER_EXPLOSION {
            if self.actor.y_dist_to_water >= 20.0 {
                let pos = self.actor.projected_pos;
                play.with_ss(|s| s.dead_sound_spawn_stationary(pos, NA_SE_IT_BOMB_UNEXPLOSION, 1, DEADSOUND_REPEAT_MODE_OFF, 10));
                self.actor.kill();
                return;
            }
            if self.actor.bg_check_flags & BGCHECKFLAG_WATER_TOUCH != 0 {
                self.actor.bg_check_flags &= !BGCHECKFLAG_WATER_TOUCH;
                audio_play_actor_sfx2(play, NA_SE_EV_BOMB_DROP_WATER);
            }
        }
    }

    /// `EnBombf_Draw`'s `Collider_UpdateSpheres(0, &explosionCollider)` once it's an explosion:
    /// the sphere round the draw's matrix (its translation; the model sphere's centre is 0), its
    /// radius the model's.
    fn draw_update(&mut self, _play: &mut PlayState) {
        if self.ty() > BOMBFLOWER_BODY {
            let m = oot_game::play::actor_draw_matrix(&RenderState::of(&self.actor));
            self.explosion_collider.update_spheres(0, &m);
        }
    }

    fn render_state(&self) -> RenderState {
        let mut r = RenderState::of(&self.actor);
        r.values = vec![self.flash_intensity, self.flower_bomb_scale];
        r.switches = vec![(self.ty() as i32 + 1) as u32];
        r
    }

    /// `EnBombf_Draw`: the flower's leaves (`gBombFlowerLeavesDL`, `gBombFlowerBaseLeavesDL`) and
    /// its bomb 1000 up at its size; or the bomb alone; `gBombFlowerBombAndSparkDL` with env
    /// `((s16)flashIntensity, 20, 10, 0)` (its own prim set in the list), the spark facing the
    /// camera (`EnBombf_NewMtxDList`: `Matrix_ReplaceRotation(&play->billboardMtxF)`). Nothing once
    /// it's an explosion.
    fn draw(&self, rs: &RenderState, _play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        let ty = rs.switches.get(rs::TYPE).copied().unwrap_or(0) as i32 - 1;
        if ty > BOMBFLOWER_BODY as i32 {
            return;
        }
        let mut m = oot_game::play::actor_draw_matrix(rs);
        if ty != BOMBFLOWER_BODY as i32 {
            out.opa.push(eng_gfx::DrawCmd::new(eng_gfx::MeshKey::named(keys::bake(BAKE_FLOWER)), m));
            let s = rs.values.get(rs::BOMB_SCALE).copied().unwrap_or(0.0);
            // At size 0 the bomb and its spark collapse to a point: nothing drawn.
            if s == 0.0 {
                return;
            }
            m = m * Mat4::from_translation(Vec3::new(0.0, 1000.0, 0.0)) * Mat4::from_scale(Vec3::splat(s));
        }
        let billboarded = oot_game::draw::replace_rotation(m, view.billboard);
        let flash = rs.values.get(rs::FLASH).copied().unwrap_or(0.0) as i16 as u8;
        let mut sv = eng_gfx::SegmentValues::default();
        sv.prim[SEG_COLOR as usize] = Some([200, 255, 200, 255]);
        sv.env[SEG_COLOR as usize] = Some([flash, 20, 10, 0]);
        out.opa.push(eng_gfx::DrawCmd {
            mesh: eng_gfx::MeshKey::named(keys::bake(BAKE_BOMB)),
            transform: m,
            bones: vec![m.inverse() * billboarded],
            params: eng_gfx::DrawParams { segments: Some(sv), ..Default::default() },
        });
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            COL_BOMB => Some(ColliderMut::Cylinder(&mut self.bomb_collider)),
            COL_EXPLOSION => Some(ColliderMut::JntSph(&mut self.explosion_collider)),
            _ => None,
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placements_are_flowers() {
        assert_eq!(TYPE.get(0xFFFFu16 as i16), -1);
        assert_eq!(TYPE.get(0), BOMBFLOWER_BODY);
    }
}
