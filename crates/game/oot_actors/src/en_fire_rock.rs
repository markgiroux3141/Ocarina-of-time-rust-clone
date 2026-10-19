//! `En_Fire_Rock` (`ovl_En_Fire_Rock/z_en_fire_rock.c`): the burning rocks (GAME-09 milestone 6).
//! Its kinds, by params (`EnFireRockType`):
//!
//! - **The falling rocks** (`FIRE_ROCK_SPAWNED_FALLING1`, `_FALLING2`): `En_Encount2`'s, from the
//!   sky over Death Mountain Trail's summit and Ganon's Tower's collapse, and the ceiling spawner's.
//!   Scale 0.02 to 0.04, falling under `-0.3 - scale × 7`, tumbling (15 to 25 degrees a frame on
//!   each axis), a flame (`EffectSsEnFire`, 100) on them every frame; an attack on Link
//!   (`D_80A12CA0`: 8, `HIT_SPECIAL_EFFECT_9`, cylinder 30 by 30, 10 down). `_FALLING1` (the child's
//!   near-Link rocks) homes on Link at 10 a frame while he's below it. Landing (and its 10
//!   frames' wait over): a quake (5, 2 frames), a ring of two dust clouds, five flames (300) on the
//!   floor, and it breaks the next frame (`EnFireRock_SpawnMoreBrokenPieces`) into two
//!   `_BROKEN_PIECE1` and `NA_SE_EN_VALVAISA_ROCK`. Turned away by a shield (`AT_BOUNCED`):
//!   `NA_SE_EV_BRIDGE_OPEN_STOP`, stopped, broken the same way. Gone with no floor under it or
//!   200 below Link.
//! - **The pieces**: `_BROKEN_PIECE1` jumps (4 to 7 up, 3 to 6 along a random yaw, gravity -1.5,
//!   scale 0.02 to 0.03) with the same attack, rumbling every 4th frame; landing, the dust and
//!   flames again, and two `_BROKEN_PIECE2`. Those (2 to 5 along, gravity -1.2, scale 0.01 to
//!   0.012, no collider) land in a ring of four dust clouds and `NA_SE_EV_EXPLOSION` (40 frames
//!   at the place), and are gone.
//! - **The ceiling spawner** (`FIRE_ROCK_CEILING_SPOT_SPAWNER`, 5; not drawn): Link coming within
//!   200 (across), a `_FALLING2` 10 above it (±1.5 across), which can't land for 10 frames;
//!   again only after Link has left and come back, 30 frames on at least.
//! - **The rock on the floor** (`FIRE_ROCK_ON_FLOOR`, 6): a prop (`Actor_ChangeCategory`) of
//!   scale 0.03, immovable, burning (a flame of 100 to 160 every 3 to 5 frames), its attack fire
//!   (`D_80A12CCC`: 8, `HIT_SPECIAL_EFFECT_FIRE`, cylinder 23 by 37, 10 down); having hit Link,
//!   a small knockback (2 back along his facing, 3 up, 4 damage) unless he's hurt already
//!   (`PLAYER_STATE1_26`).
//!
//! Its draw (all but the ceiling spawner's): `object_efc_star_field_DL_000DE0`, 3 up, tumbled by
//! `rockRotation` (degrees), at its scale, `Gfx_SetupDL_25Opa`, prim (255, 155, 55), env (155,
//! 255, 55): `Demo_Kankyo`'s rock bake (`demo_kankyo::BAKE_ROCK`, the same list and colours). The
//! circle shadow is the shape's (`ActorShape_Init`'s, drawn by `Actor_Draw`), its scale `10 +
//! scale × 300` clamped to 10..20 each frame.
//!
//! **@bug (game)**, kept: `_FALLING1`'s area check (`(x > -700) || (x < 100) || (z > -1290) || (z
//! < -3880)`) holds for every x, so it homes on Link wherever he is (below it).
//!
//! The whole overlay is ported. The debug display (`BREG(0)`, the ceiling spawner's) isn't drawn;
//! the rumble isn't ported (`Actor_RequestQuakeAndRumble`'s quake is).
//!
//! Params: `type` the whole params (`EnFireRockType`: 0 `FIRE_ROCK_SPAWNED_FALLING1`, 1
//! `_BROKEN_PIECE1`, 2 `_BROKEN_PIECE2`, 3 `_SPAWNED_FALLING2`, 5 `_CEILING_SPOT_SPAWNER`, 6
//! `_ON_FLOOR`; any other: killed at init). Placed: 5 and 6, in Ganon's Tower's collapse
//! (`ganon_sonogo`, `ganontikasonogo`); 0 to 3 are spawned.
//! Flags: none.

use eng_gfx::{DrawCmd, MeshKey};
use eng_collision::math3d::Cylinder16;
use eng_math::approach_f;
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ACTORCAT_PROP, ActorHandle, ActorImpl, ActorProfile, actor_set_player_knockback_small, actor_spawn_floor_dust_ring, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::audio::sfx::{NA_SE_EN_VALVAISA_ROCK, NA_SE_EV_BRIDGE_OPEN_STOP, NA_SE_EV_EXPLOSION};
use oot_game::collision_check::*;
use oot_game::effect::SsActor;
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

use crate::en_encount2::EnEncount2;

/// `ACTOR_EN_FIRE_ROCK` (`actor_table.h`: 0x00B5).
pub const ACTOR_EN_FIRE_ROCK: i16 = 0x00B5;
/// `OBJECT_EFC_STAR_FIELD` (`object_table.h`: 0x0092).
pub const OBJECT_EFC_STAR_FIELD: i16 = 0x0092;
pub const OBJECT: &str = "object_efc_star_field";

/// `En_Fire_Rock_Profile`: `ACTORCAT_ENEMY`, `ACTOR_FLAG_UPDATE_CULLING_DISABLED |
/// ACTOR_FLAG_DRAW_CULLING_DISABLED`, `OBJECT_EFC_STAR_FIELD`.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_EN_FIRE_ROCK, name: "En_Fire_Rock", category: ACTORCAT_ENEMY, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `EnFireRockType` (`z_en_fire_rock.h`).
pub const FIRE_ROCK_SPAWNED_FALLING1: i16 = 0;
pub const FIRE_ROCK_BROKEN_PIECE1: i16 = 1;
pub const FIRE_ROCK_BROKEN_PIECE2: i16 = 2;
pub const FIRE_ROCK_SPAWNED_FALLING2: i16 = 3;
pub const FIRE_ROCK_CEILING_SPOT_SPAWNER: i16 = 5;
pub const FIRE_ROCK_ON_FLOOR: i16 = 6;

/// `this->type = this->actor.params`.
pub const TYPE: Field = Field::whole(
    "type",
    Kind::Named(&[
        (0, "FIRE_ROCK_SPAWNED_FALLING1"),
        (1, "FIRE_ROCK_BROKEN_PIECE1"),
        (2, "FIRE_ROCK_BROKEN_PIECE2"),
        (3, "FIRE_ROCK_SPAWNED_FALLING2"),
        (5, "FIRE_ROCK_CEILING_SPOT_SPAWNER"),
        (6, "FIRE_ROCK_ON_FLOOR"),
    ]),
    "which rock: 0 and 3 falling (En_Encount2's), 1 and 2 the pieces, 5 a spawner of falling rocks over Link, 6 a burning rock on the floor; any other killed at init",
);

/// En_Fire_Rock's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[TYPE], flags: &[], todo: false, objects: &[], texts: &[] };

/// `PLAYER_STATE1_26`: hurt.
const PLAYER_STATE1_26: u32 = crate::player::STATE1_26;

const fn cylinder_init(hit_special_effect: u8) -> ColliderCylinderInit {
    ColliderCylinderInit {
        base: ColliderInit {
            col_type: COL_MATERIAL_HARD,
            at_flags: AT_ON | AT_TYPE_ENEMY,
            ac_flags: AC_ON | AC_TYPE_PLAYER,
            oc_flags1: OC1_NONE,
            oc_flags2: OC2_TYPE_2,
            shape: COLSHAPE_CYLINDER,
        },
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect, damage: 0x08 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
            at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
            ac_elem_flags: ACELEM_ON,
            oc_elem_flags: OCELEM_NONE,
        },
        dim: Cylinder16 { radius: 30, height: 30, y_shift: -10, pos: [0; 3] },
    }
}

/// `D_80A12CA0`: the falling rocks' and the big pieces'.
pub const D_80A12CA0: ColliderCylinderInit = cylinder_init(HIT_SPECIAL_EFFECT_9);
/// `D_80A12CCC`: the rock on the floor's (its dimensions set after: 23, 37, -10).
pub const D_80A12CCC: ColliderCylinderInit = cylinder_init(HIT_SPECIAL_EFFECT_FIRE);

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `FireRock_WaitSpawnRocksFromCeiling`.
    WaitSpawnRocksFromCeiling,
    /// `FireRock_WaitOnFloor`.
    WaitOnFloor,
    /// `EnFireRock_Fall`.
    Fall,
    /// `EnFireRock_SpawnMoreBrokenPieces`.
    SpawnMoreBrokenPieces,
    /// None set (an unknown type, killed at init).
    None,
}

pub struct EnFireRock {
    pub actor: Actor,
    pub angular_velocity: Vec3,
    /// `rockRotation`: degrees.
    pub rock_rotation: Vec3,
    pub action: Action,
    /// `scale`: the rock's own (set again by the rock it broke from: its less 0.01).
    pub scale: f32,
    /// `relativePos`: the draw's offset (3 up once updated).
    pub relative_pos: Vec3,
    /// `unk_17C`: set (`_FALLING1`), read by nothing.
    pub unk_17c: Vec3,
    pub timer: i16,
    pub timer2: i16,
    pub ty: i16,
    pub player_nearby: u8,
    /// `spawner`: the `En_Encount2` that spawned it.
    pub spawner: Option<ActorHandle>,
    pub collider: ColliderCylinder,
    /// `actor.draw` (NULL for the ceiling spawner).
    pub drawn: bool,
}

/// `DEG_TO_RAD` (a double `M_PI`).
fn deg_to_rad(d: f32) -> f32 {
    (d as f64 * (std::f64::consts::PI / 180.0)) as f32
}

/// What these functions read of Player: his position, facing, `stateFlags1` and handle.
fn player_info(play: &PlayState) -> Option<(Vec3, i16, u32, ActorHandle)> {
    let h = play.player?;
    let a = play.actors.get(h)?;
    let state1 = a.as_player().map(|p| p.state_flags1()).unwrap_or(0);
    Some((a.base().world_pos, a.base().world_rot.y, state1, h))
}

impl EnFireRock {
    /// `EnFireRock_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let player_pos = player_info(play).map(|p| p.0).unwrap_or(Vec3::ZERO);
        let ty = TYPE.get(actor.params);
        let mut this = EnFireRock {
            actor,
            angular_velocity: Vec3::ZERO,
            rock_rotation: Vec3::ZERO,
            action: Action::None,
            scale: 0.0,
            relative_pos: Vec3::ZERO,
            unk_17c: Vec3::ZERO,
            timer: 0,
            timer2: 0,
            ty,
            player_nearby: 0,
            spawner: None,
            collider: ColliderCylinder::default(),
            drawn: true,
        };
        if ty != FIRE_ROCK_CEILING_SPOT_SPAWNER {
            this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 15.0);
            if ty != FIRE_ROCK_ON_FLOOR {
                this.angular_velocity.x = play.rand.zero_float(10.0) + 15.0;
                this.angular_velocity.y = play.rand.zero_float(10.0) + 15.0;
                this.angular_velocity.z = play.rand.zero_float(10.0) + 15.0;
            }
        }
        match ty {
            FIRE_ROCK_CEILING_SPOT_SPAWNER => {
                this.drawn = false;
                log::debug!("En_Fire_Rock: ceiling waiting rock");
                this.action = Action::WaitSpawnRocksFromCeiling;
            }
            FIRE_ROCK_ON_FLOOR => {
                this.actor.scale = Vec3::splat(0.03);
                this.collider = ColliderCylinder::new(&D_80A12CCC);
                log::debug!("En_Fire_Rock: floor rock");
                this.collider.dim.radius = 23;
                this.collider.dim.height = 37;
                this.collider.dim.y_shift = -10;
                // Actor_ChangeCategory(.., ACTORCAT_PROP): the spawn files it there.
                this.actor.category = ACTORCAT_PROP;
                this.actor.col_chk_info.mass = MASS_IMMOVABLE;
                this.action = Action::WaitOnFloor;
            }
            FIRE_ROCK_SPAWNED_FALLING1 | FIRE_ROCK_SPAWNED_FALLING2 => {
                if ty == FIRE_ROCK_SPAWNED_FALLING1 {
                    // Sets unused vars (the C's comment).
                    this.unk_17c.x = play.rand.centered_float(50.0) + player_pos.x;
                    this.unk_17c.z = play.rand.centered_float(50.0) + player_pos.z;
                }
                this.scale = (play.rand.zero_float(2.0) / 100.0) + 0.02;
                this.actor.scale = Vec3::splat(this.scale);
                this.collider = ColliderCylinder::new(&D_80A12CA0);
                let yaw = play.rand.centered_float(65535.0) as i16;
                this.actor.world_rot.y = yaw;
                this.actor.shape_rot.y = yaw;
                this.action = Action::Fall;
                this.actor.shape_shadow_scale = 15.0;
            }
            FIRE_ROCK_BROKEN_PIECE1 => {
                this.actor.velocity.y = play.rand.zero_float(3.0) + 4.0;
                this.actor.speed_xz = play.rand.zero_float(3.0) + 3.0;
                this.scale = (play.rand.zero_float(1.0) / 100.0) + 0.02;
                this.actor.scale = Vec3::splat(this.scale);
                this.actor.gravity = -1.5;
                this.collider = ColliderCylinder::new(&D_80A12CA0);
                this.actor.shape_shadow_scale = 10.0;
                let yaw = play.rand.centered_float(65535.0) as i16;
                this.actor.world_rot.y = yaw;
                this.actor.shape_rot.y = yaw;
                this.action = Action::Fall;
            }
            FIRE_ROCK_BROKEN_PIECE2 => {
                this.actor.velocity.y = play.rand.zero_float(3.0) + 4.0;
                this.actor.speed_xz = play.rand.zero_float(3.0) + 2.0;
                this.scale = (play.rand.zero_float(1.0) / 500.0) + 0.01;
                this.actor.scale = Vec3::splat(this.scale);
                this.actor.gravity = -1.2;
                this.actor.shape_shadow_scale = 5.0;
                let yaw = play.rand.centered_float(65535.0) as i16;
                this.actor.world_rot.y = yaw;
                this.actor.shape_rot.y = yaw;
                this.action = Action::Fall;
            }
            _ => {
                log::error!("En_Fire_Rock: No such rock! ERR!!!!!! (type {ty})");
                this.actor.kill();
            }
        }
        Box::new(this)
    }

    fn ss_actor(&self, play: &PlayState) -> Option<SsActor> {
        let r = self.actor.shape_rot;
        play.cur_actor.map(|h| SsActor { handle: h, world_pos: self.actor.world_pos, shape_rot: [r.x, r.y, r.z] })
    }

    /// `EffectSsEnFire_SpawnVec3f(play, &this->actor, pos, scale, 0, 0, -1)`.
    fn flame(&self, play: &mut PlayState, pos: Vec3, scale: i16) {
        let me = self.ss_actor(play);
        play.effect_ss_en_fire_spawn_vec3f(me, pos, scale, 0, 0, -1);
    }

    /// `EnFireRock_Fall`: gone with no floor or 200 below Link; flames on the falling rocks
    /// (`_FALLING1` homing on Link below it), the big piece's rumble; landing (once `timer` is
    /// out), the quake for the falling ones, the dust and flames and the break, or for the small
    /// pieces the dust and the explosion's sound.
    fn fall(&mut self, play: &mut PlayState) {
        let Some((player_pos, ..)) = player_info(play) else { return };
        if self.actor.floor_height == eng_collision::bgcheck::BGCHECK_Y_MIN || self.actor.world_pos.y < player_pos.y - 200.0 {
            self.actor.kill();
            return;
        }
        match self.ty {
            FIRE_ROCK_SPAWNED_FALLING1 | FIRE_ROCK_SPAWNED_FALLING2 => {
                if self.ty == FIRE_ROCK_SPAWNED_FALLING1 && player_pos.y < self.actor.world_pos.y {
                    // @bug (game): always true (any x is above -700 or below 100).
                    if player_pos.x > -700.0 || player_pos.x < 100.0 || player_pos.z > -1290.0 || player_pos.z < -3880.0 {
                        approach_f(&mut self.actor.world_pos.x, player_pos.x, 1.0, 10.0);
                        approach_f(&mut self.actor.world_pos.z, player_pos.z, 1.0, 10.0);
                    }
                }
                let x = play.rand.centered_float(20.0) + self.actor.world_pos.x;
                let y = play.rand.centered_float(20.0) + self.actor.world_pos.y;
                let z = play.rand.centered_float(20.0) + self.actor.world_pos.z;
                self.flame(play, Vec3::new(x, y, z), 100);
            }
            FIRE_ROCK_BROKEN_PIECE1 => {
                if play.gameplay_frames & 3 == 0 {
                    audio_play_actor_sfx2(play, NA_SE_EN_VALVAISA_ROCK);
                }
            }
            _ => {}
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 && self.timer == 0 {
            match self.ty {
                FIRE_ROCK_SPAWNED_FALLING1 | FIRE_ROCK_SPAWNED_FALLING2 | FIRE_ROCK_BROKEN_PIECE1 => {
                    if self.ty != FIRE_ROCK_BROKEN_PIECE1 {
                        play.actor_request_quake_and_rumble(5, 2);
                    }
                    actor_spawn_floor_dust_ring(play, &self.actor, self.actor.world_pos, self.actor.shape_shadow_scale, 1, 8.0, 500, 10, false);
                    for _ in 0..5 {
                        let x = play.rand.centered_float(20.0) + self.actor.world_pos.x;
                        let y = self.actor.floor_height;
                        let z = play.rand.centered_float(20.0) + self.actor.world_pos.z;
                        self.flame(play, Vec3::new(x, y, z), 300);
                    }
                    self.action = Action::SpawnMoreBrokenPieces;
                }
                _ => {
                    actor_spawn_floor_dust_ring(play, &self.actor, self.actor.world_pos, self.actor.shape_shadow_scale, 3, 8.0, 200, 10, false);
                    play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, 40, NA_SE_EV_EXPLOSION);
                    self.actor.kill();
                }
            }
        }
    }

    /// `EnFireRock_SpawnMoreBrokenPieces`: after the rock has already hit the ground, two more
    /// (the next smaller kind, 10 up, ±1.5 each way) along its way, the first turned as it was,
    /// its scale less 0.01; the rock gone. The small pieces (and the kinds that don't fall)
    /// spawn none.
    fn spawn_more_broken_pieces(&mut self, play: &mut PlayState) {
        let next_rock_type = match self.ty {
            FIRE_ROCK_SPAWNED_FALLING1 | FIRE_ROCK_SPAWNED_FALLING2 => FIRE_ROCK_BROKEN_PIECE1,
            FIRE_ROCK_BROKEN_PIECE1 => FIRE_ROCK_BROKEN_PIECE2,
            _ => FIRE_ROCK_SPAWNED_FALLING1,
        };
        if next_rock_type != FIRE_ROCK_SPAWNED_FALLING1 {
            for i in 0..2 {
                let p = self.actor.world_pos;
                let x = play.rand.centered_float(3.0) + p.x;
                let y = play.rand.centered_float(3.0) + (p.y + 10.0);
                let z = play.rand.centered_float(3.0) + p.z;
                match play.actor_spawn(ACTOR_EN_FIRE_ROCK, Vec3::new(x, y, z), [0; 3], next_rock_type) {
                    Ok(h) => {
                        if let Some(r) = play.actors.downcast_mut::<EnFireRock>(h) {
                            r.actor.world_rot.y = self.actor.world_rot.y;
                            if i == 0 {
                                r.actor.shape_rot.y = self.actor.shape_rot.y;
                            }
                            r.scale = self.scale - 0.01;
                        }
                    }
                    Err(e) => log::debug!("En_Fire_Rock: It's full ({e:?})"),
                }
            }
            audio_play_actor_sfx2(play, NA_SE_EN_VALVAISA_ROCK);
        }
        self.actor.kill();
    }

    /// `FireRock_WaitSpawnRocksFromCeiling`: Link within 200 across, not near last frame, and its
    /// 30 frames out: a falling rock 10 up (±1.5 across) that can't land for 10 frames.
    fn wait_spawn_rocks_from_ceiling(&mut self, play: &mut PlayState) {
        if self.actor.xz_dist_to_player < 200.0 {
            if self.player_nearby == 0 && self.timer2 == 0 {
                self.timer2 = 30;
                let x = play.rand.centered_float(3.0) + self.actor.world_pos.x;
                let y = self.actor.world_pos.y + 10.0;
                let z = play.rand.centered_float(3.0) + self.actor.world_pos.z;
                match play.actor_spawn(ACTOR_EN_FIRE_ROCK, Vec3::new(x, y, z), [0; 3], FIRE_ROCK_SPAWNED_FALLING2) {
                    Ok(h) => {
                        if let Some(r) = play.actors.downcast_mut::<EnFireRock>(h) {
                            r.timer = 10;
                        }
                    }
                    Err(e) => log::debug!("En_Fire_Rock: It's full ({e:?})"),
                }
            }
            self.player_nearby = 1;
        } else {
            self.player_nearby = 0;
        }
        // DEBUG_FEATURES && BREG(0): DebugDisplay_AddObject isn't drawn.
    }

    /// `FireRock_WaitOnFloor`: a flame (130 ± 30) within 10 of it every 3 to 5 frames.
    fn wait_on_floor(&mut self, play: &mut PlayState) {
        if self.timer2 == 0 {
            let x = play.rand.centered_float(20.0) + self.actor.world_pos.x;
            let y = play.rand.centered_float(20.0) + self.actor.world_pos.y;
            let z = play.rand.centered_float(20.0) + self.actor.world_pos.z;
            let scale = 130i16.wrapping_add(play.rand.centered_float(60.0) as i16);
            self.timer2 = 3 + play.rand.zero_float(3.0) as i16;
            self.flame(play, Vec3::new(x, y, z), scale);
        }
    }
}

/// The render state's values: `rockRotation` (3), `relativePos` (3).
const RS_VALUES: usize = 6;

impl ActorImpl for EnFireRock {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnFireRock_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if self.timer2 != 0 {
            self.timer2 -= 1;
        }
        if self.timer != 0 {
            self.timer -= 1;
        }
        match self.action {
            Action::WaitSpawnRocksFromCeiling => self.wait_spawn_rocks_from_ceiling(play),
            Action::WaitOnFloor => self.wait_on_floor(play),
            Action::Fall => self.fall(play),
            Action::SpawnMoreBrokenPieces => self.spawn_more_broken_pieces(play),
            Action::None => {}
        }
        if self.ty == FIRE_ROCK_CEILING_SPOT_SPAWNER {
            return;
        }
        self.rock_rotation += self.angular_velocity;
        self.relative_pos.y = 3.0;
        self.actor.shape_shadow_scale = (10.0 + (self.scale * 300.0)).clamp(10.0, 20.0);
        if self.ty == FIRE_ROCK_SPAWNED_FALLING1 || self.ty == FIRE_ROCK_SPAWNED_FALLING2 {
            self.actor.gravity = -0.3 - (self.scale * 7.0);
        }
        if self.ty != FIRE_ROCK_ON_FLOOR {
            self.actor.move_forward();
            self.actor.update_bg_check_info(&play.col, 50.0, 50.0, 100.0, UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
        }
        let mut set_collision = false;
        if self.action != Action::SpawnMoreBrokenPieces && matches!(self.ty, FIRE_ROCK_SPAWNED_FALLING1 | FIRE_ROCK_SPAWNED_FALLING2 | FIRE_ROCK_BROKEN_PIECE1) {
            if self.collider.base.at_flags & AT_BOUNCED != 0 {
                self.collider.base.at_flags &= !AT_BOUNCED;
                audio_play_actor_sfx2(play, NA_SE_EV_BRIDGE_OPEN_STOP);
                self.actor.velocity.y = 0.0;
                self.actor.speed_xz = 0.0;
                self.action = Action::SpawnMoreBrokenPieces;
                log::debug!("En_Fire_Rock: Shield Defense Lv1");
                return;
            }
            set_collision = true;
        }
        if self.ty == FIRE_ROCK_ON_FLOOR {
            if self.collider.base.at_flags & AT_HIT != 0 {
                self.collider.base.at_flags &= !AT_HIT;
                if let Some((_, rot_y, state1, ph)) = player_info(play)
                    && self.collider.base.at == Some(ph)
                {
                    if state1 & PLAYER_STATE1_26 == 0 {
                        actor_set_player_knockback_small(play, 2.0, rot_y.wrapping_neg(), 3.0, 4);
                    }
                    return;
                }
            }
            set_collision = true;
        }
        if set_collision {
            self.collider.update(&self.actor);
            play.collision_check_set_at(&self.actor, 0, &mut self.collider);
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
        }
    }

    /// `EnFireRock_Destroy`: a rock its `En_Encount2` spawned counts itself off.
    fn destroy(&mut self, play: &mut PlayState) {
        if let Some(parent) = self.actor.parent
            && Some(parent) == self.spawner
            && let Some(s) = play.actors.downcast_mut::<EnEncount2>(parent)
            && !s.actor.killed
            && s.num_spawned_rocks > 0
        {
            s.num_spawned_rocks -= 1;
            log::debug!("En_Fire_Rock: Number of spawned instances recovery {}", s.num_spawned_rocks);
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        let (r, p) = (self.rock_rotation, self.relative_pos);
        rs.values = vec![r.x, r.y, r.z, p.x, p.y, p.z];
        rs.switches = vec![self.drawn as u32];
        rs
    }

    /// `EnFireRock_Draw`: the rock at its position and `relativePos`, tumbled by `rockRotation`
    /// (X, Y, Z), at its scale.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let ([drawn], Some(v)) = (rs.switches.as_slice(), rs.values.get(..RS_VALUES)) else { return };
        if *drawn == 0 {
            return;
        }
        let m = Mat4::from_translation(rs.pos + Vec3::new(v[3], v[4], v[5]))
            * Mat4::from_rotation_x(deg_to_rad(v[0]))
            * Mat4::from_rotation_y(deg_to_rad(v[1]))
            * Mat4::from_rotation_z(deg_to_rad(v[2]))
            * Mat4::from_scale(rs.scale);
        out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(crate::demo_kankyo::BAKE_ROCK)), m));
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0).then_some(ColliderMut::Cylinder(&mut self.collider))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
