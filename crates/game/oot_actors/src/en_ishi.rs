//! `En_Ishi` (`ovl_En_Ishi/z_en_ishi.c`): a small grey rock (`params & 1` 0, `gFieldKakeraDL`)
//! or a silver boulder (1, `gSilverRockDL`). The sword bounces off it (`AC_HARD`); a hammer or an
//! explosion breaks a small one.
//!
//! A broken small rock drops from its table (`EnIshi_DropCollectible`), and breaks into pieces
//! (`sFragmentSpawnFuncs`: `EffectSsKakera`) and dust (`sDustSpawnFuncs`: `func_80033480`).
//!
//! Within 90 it offers itself to Player's lift (`Actor_OfferGetItem` with `GI_NONE`: 50 across
//! and 10 up for a small rock, 80 and 20 for a boulder, which Player lifts only with the silver
//! gauntlets). Lifted (`EnIshi_LiftedUp`) it plays its pull-up sound and, with bugs under it,
//! lets three out (`En_Insect`, a placeholder); thrown (`EnIshi_Fly`) it tumbles and breaks where
//! it lands or hits a wall, a boulder setting its switch flag and shaking the camera.
//!
//! Not ported: the splash and ripples where it falls in water (`EffectSsGSplash`,
//! `EffectSsGRipple`) and the boulder's rumble (`Rumble_Request`).
//!
//! Params: `type` bit 0 (`EnIshiType`: 0 `ROCK_SMALL`, 1 `ROCK_LARGE`, a silver boulder); `bugs`
//! bit 4 (1: bugs under it, three let out when lifted); `noSnapToFloor` bit 5 (1: not set on the
//! floor at init); `switchFlagLower` bits 6 and 7 and `switchFlagUpper` bits 12 to 15 (the
//! boulder's switch flag's low two and high four bits, `ISHI_GET_SWITCH_FLAG_LOWER` and
//! `_UPPER`, which `ISHI_GET_SWITCH_FLAG` puts together); `dropParams` bits 8 to 11 (the small
//! rock's collectible drop table, 13 and up read as 0; the boulders' unread, 0xF in the data).
//! Flags: the boulder reads its switch flag `ISHI_GET_SWITCH_FLAG` at init (set: gone) and sets it
//! when thrown.

use eng_collision::math3d::Cylinder16;
use glam::Vec3;
use oot_game::actor::{
    ACTOR_FLAG_THROW_ONLY, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, BGCHECKFLAG_GROUND, BGCHECKFLAG_WALL, BGCHECKFLAG_WATER, BGCHECKFLAG_WATER_TOUCH, UPDBGCHECKINFO_FLAG_0, UPDBGCHECKINFO_FLAG_2,
    UPDBGCHECKINFO_FLAG_7,
};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile, func_80033480};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::collision_check::*;
use oot_game::effect::kakera::{KAKERA_COLOR_NONE, KAKERA_COLOR_WHITE, OBJECT_GAMEPLAY_FIELD_KEEP};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

pub const ACTOR_EN_ISHI: i16 = 0x014E;

/// `En_Ishi_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_ISHI, name: "En_Ishi", category: ACTORCAT_PROP, flags: ACTOR_FLAG_THROW_ONLY, object: "gameplay_field_keep", schema: &SCHEMA };

/// `PARAMS_GET_U(this->actor.params, 0, 1)`: the rock.
pub const TYPE: Field = Field::params("type", 0, 1, Kind::Named(&[(0, "ROCK_SMALL"), (1, "ROCK_LARGE")]), "a small rock or a silver boulder");
/// `PARAMS_GET_U(this->actor.params, 4, 1)`: bugs under it.
pub const BUGS: Field = Field::params("bugs", 4, 1, Kind::Named(&[(0, "no bugs"), (1, "bugs under it")]), "three bugs let out when it's lifted");
/// `PARAMS_GET_U(this->actor.params, 5, 1)`: not snapped to the floor.
pub const NO_SNAP: Field =
    Field::params("noSnapToFloor", 5, 1, Kind::Named(&[(0, "set on the floor below (gone with none)"), (1, "stays where placed")]), "whether init sets it on the floor");
/// `ISHI_GET_SWITCH_FLAG_LOWER`: `PARAMS_GET_U(params, 6, 2)`, the switch flag's bits 0 and 1.
pub const SWITCH_FLAG_LOWER: Field =
    Field::params("switchFlagLower", 6, 2, Kind::Range(0, 3), "the boulder's switch flag's low two bits").when("type", &[1]);
/// `PARAMS_GET_U(this->actor.params, 8, 4)`: the drop table.
pub const DROP_TABLE: Field =
    Field::params("dropParams", 8, 4, Kind::Range(0, 15), "the small rock's collectible drop table (13 and up: table 0; the boulder's unread, 0xF in the data)");
/// `ISHI_GET_SWITCH_FLAG_UPPER`: `(params >> (12 - 2)) & (NBITS_TO_MASK(4) << 2)`, the switch
/// flag's bits 2 to 5.
pub const SWITCH_FLAG_UPPER: Field =
    Field::params("switchFlagUpper", 12, 4, Kind::Range(0, 15), "the boulder's switch flag's high four bits").when("type", &[1]);

/// En_Ishi's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, BUGS, NO_SNAP, SWITCH_FLAG_LOWER, DROP_TABLE, SWITCH_FLAG_UPPER],
    flags: &[FlagUse::new(Flag::Switch("switchFlagUpper << 2 | switchFlagLower")).reads().sets().note("the boulder: read at init (set: gone), set when thrown")],
    todo: false,
};

/// `ROCK_SMALL`, `ROCK_LARGE`.
const ROCK_SMALL: usize = 0;
const ROCK_LARGE: usize = 1;

/// `sRockScales`, `D_80A7FA20` (the model's `yOffset`).
const ROCK_SCALES: [f32; 2] = [0.1, 0.4];
const Y_OFFSETS: [f32; 2] = [58.0, 80.0];

/// `sInitChains[type]`: gravity, `minVelocityY`, and the culling volume (distance, scale,
/// downward: 1200, 150, 400 for the small rocks; 2000, 250, 500 for the boulders).
const GRAVITY: [f32; 2] = [-1.2, -2.5];
const MIN_VELOCITY_Y: f32 = -20.0;
const CULLING_VOLUMES: [(f32, f32, f32); 2] = [(1200.0, 150.0, 400.0), (2000.0, 250.0, 500.0)];

/// `sCylinderInits`.
const CYLINDER_INITS: [ColliderCylinderInit; 2] = [
    ColliderCylinderInit {
        base: ColliderInit { col_type: COL_MATERIAL_HARD, at_flags: AT_NONE, ac_flags: AC_ON | AC_HARD | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: 0, damage: 0 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x4FC1_FFFE, hit_backlash: 0, defense: 0 },
            at_elem_flags: ATELEM_NONE,
            ac_elem_flags: ACELEM_ON,
            oc_elem_flags: OCELEM_ON,
        },
        dim: Cylinder16 { radius: 10, height: 18, y_shift: -2, pos: [0; 3] },
    },
    ColliderCylinderInit {
        base: ColliderInit { col_type: COL_MATERIAL_HARD, at_flags: AT_NONE, ac_flags: AC_ON | AC_HARD | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: 0, damage: 0 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x4FC1_FFF6, hit_backlash: 0, defense: 0 },
            at_elem_flags: ATELEM_NONE,
            ac_elem_flags: ACELEM_ON,
            oc_elem_flags: OCELEM_ON,
        },
        dim: Cylinder16 { radius: 55, height: 70, y_shift: 0, pos: [0; 3] },
    },
];

/// `sColChkInfoInit`.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 0, cyl_radius: 12, cyl_height: 60, mass: MASS_IMMOVABLE };

/// `DMG_HAMMER | DMG_EXPLOSIVE`: what breaks a small rock.
const DMG_BREAKS_ROCK: u32 = DMG_HAMMER_SWING | DMG_HAMMER_JUMP | DMG_EXPLOSIVE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    /// `EnIshi_Wait`.
    Wait,
    /// `EnIshi_LiftedUp`.
    LiftedUp,
    /// `EnIshi_Fly`.
    Fly,
}

/// `sRotSpeedX`, `sRotSpeedY`: the thrown rock's tumble, the overlay's statics.
#[derive(Debug, Default)]
pub struct Statics {
    pub rot_speed_x: i16,
    pub rot_speed_y: i16,
}

/// `D_80A7FA28`: the air's drag on a thrown rock, by type.
const D_80A7FA28: [f32; 2] = [0.0, 0.005];
/// `liftSfxIds`: `NA_SE_PL_PULL_UP_ROCK`, `NA_SE_PL_PULL_UP_BIGROCK` (`playerbank_table.h`:
/// 0x86A, 0x86C).
const LIFT_SFX_IDS: [u16; 2] = [0x086A, 0x086C];
/// `NA_SE_EV_DIVE_INTO_WATER_L` (`environmentbank_table.h`: 0x28C5).
const NA_SE_EV_DIVE_INTO_WATER_L: u16 = 0x28C5;
/// `ACTOR_EN_INSECT` (`actor_table.h`: 0x0020), `INSECT_TYPE_SPAWNED` (`z_en_insect.h`).
const ACTOR_EN_INSECT: i16 = 0x0020;
const INSECT_TYPE_SPAWNED: i16 = 1;
/// `UPDBGCHECKINFO_FLAG_6` (`actor.h`): no splash on entering water (the port draws none).
const UPDBGCHECKINFO_FLAG_6: u32 = 1 << 6;
/// `Actor_UpdateBgCheckInfo`'s flags for the lifted and thrown rock.
const FLY_BGCHECK: u32 = UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_6 | UPDBGCHECKINFO_FLAG_7;

pub struct EnIshi {
    pub actor: Actor,
    pub collider: ColliderCylinder,
    action: Action,
}

impl EnIshi {
    fn ty(&self) -> usize {
        TYPE.get(self.actor.params) as usize
    }

    /// `EnIshi_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let ty = TYPE.get(actor.params) as usize;
        actor.gravity = GRAVITY[ty];
        actor.min_velocity_y = MIN_VELOCITY_Y;
        (actor.culling_volume_distance, actor.culling_volume_scale, actor.culling_volume_downward) = CULLING_VOLUMES[ty];
        if play.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE {
            actor.culling_volume_distance += 1000.0;
        }
        if actor.shape_rot.y == 0 {
            let r = play.rand.zero_float(65536.0) as i32 as i16;
            actor.shape_rot.y = r;
            actor.world_rot.y = r;
        }
        actor.scale = Vec3::splat(ROCK_SCALES[ty]);
        // EnIshi_InitCollider.
        let mut collider = ColliderCylinder::new(&CYLINDER_INITS[ty]);
        collider.update(&actor);
        if ty == ROCK_LARGE && play.flags.get_switch(switch_flag(&actor)) {
            actor.kill();
            return Box::new(EnIshi { actor, collider, action: Action::Wait });
        }
        actor.col_chk_info.set_info(None, &COL_CHK_INFO_INIT);
        actor.shape_y_offset = Y_OFFSETS[ty];
        if NO_SNAP.get(actor.params) == 0 && !snap_to_floor(&mut actor, play, 0.0) {
            actor.kill();
        }
        Box::new(EnIshi { actor, collider, action: Action::Wait })
    }

    /// `EnIshi_DropCollectible`: a small rock drops from its table (13 and up read as 0).
    fn drop_collectible(&self, play: &mut PlayState) {
        if self.ty() == ROCK_SMALL {
            let mut drop_params = DROP_TABLE.get(self.actor.params);
            if drop_params >= 0xD {
                drop_params = 0;
            }
            // COLLECTIBLE_DROP_RANDOM_PARAMS(dropParams, false).
            crate::en_item00::item_drop_collectible_random(play, None, self.actor.world_pos, drop_params << 4);
        }
    }

    /// `EnIshi_Fall`: gravity, down to `minVelocityY`.
    fn fall(&mut self) {
        self.actor.velocity.y += self.actor.gravity;
        if self.actor.velocity.y < self.actor.min_velocity_y {
            self.actor.velocity.y = self.actor.min_velocity_y;
        }
    }

    /// `func_80A7ED94`: the velocity slowed by `drag`, give or take a tenth of it at random.
    fn func_80a7ed94(velocity: &mut Vec3, mut drag: f32, play: &mut PlayState) {
        drag += ((play.rand.zero_one() * 0.2) - 0.1) * drag;
        velocity.x -= velocity.x * drag;
        velocity.y -= velocity.y * drag;
        velocity.z -= velocity.z * drag;
    }

    /// `EnIshi_SpawnBugs`: three bugs (`En_Insect`, `INSECT_TYPE_SPAWNED`) at a random yaw, until
    /// one can't spawn.
    fn spawn_bugs(&self, play: &mut PlayState) {
        for _ in 0..3 {
            let yaw = (play.rand.zero_one() * 65535.0) as i32 as i16;
            if play.actor_spawn(ACTOR_EN_INSECT, self.actor.world_pos, [0, yaw, 0], INSECT_TYPE_SPAWNED).is_err() {
                break;
            }
        }
    }

    /// `EnIshi_Wait`: lifted (Player its parent): its pull-up sound, and its bugs let out. A small
    /// rock hit by a hammer or an explosion breaks. Within 600 its collider takes hits, within
    /// 400 pushes, and within 90 it offers itself to Player's lift.
    fn wait(&mut self, play: &mut PlayState) {
        let ty = self.ty();
        let hit = self.collider.base.ac_flags & AC_HIT != 0;
        if self.actor.parent.is_some() {
            self.setup_lifted_up();
            play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, 20, LIFT_SFX_IDS[ty]);
            if BUGS.get(self.actor.params) != 0 {
                self.spawn_bugs(play);
            }
        } else if hit && ty == ROCK_SMALL && self.collider.info.ac_hit_elem.is_some_and(|h| h.at_dmg_info.dmg_flags & DMG_BREAKS_ROCK != 0) {
            self.drop_collectible(play);
            play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, BREAK_SFX_DURATIONS[ty], BREAK_SFX_IDS[ty]);
            self.spawn_fragments(play, ty);
            self.spawn_dust(play, ty);
            self.actor.kill();
        } else if self.actor.xz_dist_to_player < 600.0 {
            self.collider.update(&self.actor);
            self.collider.base.ac_flags &= !AC_HIT;
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
            if self.actor.xz_dist_to_player < 400.0 {
                play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
                if self.actor.xz_dist_to_player < 90.0 {
                    // GI_NONE in these cases allows the player to lift the actor.
                    let a = self.actor.clone();
                    if ty == ROCK_LARGE {
                        oot_game::get_item::offer_get_item_range(play, &a, oot_game::item::GI_NONE, 80.0, 20.0);
                    } else {
                        oot_game::get_item::offer_get_item_range(play, &a, oot_game::item::GI_NONE, 50.0, 10.0);
                    }
                }
            }
        }
    }

    /// `EnIshi_SetupLiftedUp`: in no room while carried, and updated off screen.
    fn setup_lifted_up(&mut self) {
        self.action = Action::LiftedUp;
        self.actor.room = -1;
        self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
    }

    /// `EnIshi_LiftedUp`: let go, it's back in the room (a boulder sets its switch flag) and flies
    /// from where it is.
    fn lifted_up(&mut self, play: &mut PlayState) {
        if self.actor.parent.is_none() {
            self.actor.room = play.room_ctx.cur.num;
            if self.ty() == ROCK_LARGE {
                play.flags.set_switch(switch_flag(&self.actor));
            }
            self.setup_fly(play);
            self.fall();
            let drag = D_80A7FA28[self.ty()];
            Self::func_80a7ed94(&mut self.actor.velocity, drag, play);
            self.actor.update_pos();
            self.actor.update_bg_check_info(&play.col, 7.5, 35.0, 0.0, FLY_BGCHECK);
        }
    }

    /// `EnIshi_SetupFly`: its velocity along its yaw at its speed; a random tumble (a small rock's
    /// faster), and the mass of a thrown thing (240).
    fn setup_fly(&mut self, play: &mut PlayState) {
        self.actor.velocity.x = eng_math::sin_s(self.actor.world_rot.y) * self.actor.speed_xz;
        self.actor.velocity.z = eng_math::cos_s(self.actor.world_rot.y) * self.actor.speed_xz;
        let (x, y) = if self.ty() == ROCK_SMALL { (16000.0, 2400.0) } else { (8000.0, 1600.0) };
        let rx = ((play.rand.zero_one() - 0.5) * x) as i16;
        let ry = ((play.rand.zero_one() - 0.5) * y) as i16;
        let s = play.overlay_static::<Statics>(ACTOR_EN_ISHI);
        s.rot_speed_x = rx;
        s.rot_speed_y = ry;
        self.actor.col_chk_info.mass = 240;
        self.action = Action::Fly;
    }

    /// `EnIshi_Fly`: landing or hitting a wall it breaks (its drop, its pieces, and out of water
    /// its sound and dust; a boulder shakes the camera); entering water it slows (a splash and
    /// ripples in the C) and tumbles a quarter as fast. Otherwise it falls, slowed by the air,
    /// tumbling, its model's lift easing to 0.
    fn fly(&mut self, play: &mut PlayState) {
        let ty = self.ty();
        if self.actor.bg_check_flags & (BGCHECKFLAG_GROUND | BGCHECKFLAG_WALL) != 0 {
            self.drop_collectible(play);
            self.spawn_fragments(play, ty);
            if self.actor.bg_check_flags & BGCHECKFLAG_WATER == 0 {
                play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, BREAK_SFX_DURATIONS[ty], BREAK_SFX_IDS[ty]);
                self.spawn_dust(play, ty);
            }
            if ty == ROCK_LARGE {
                let cam = play.active_cam_id;
                let quake_index = play.quake_request(cam, oot_game::quake::QUAKE_TYPE_3);
                play.quake_set_speed(quake_index, -0x3CB0);
                play.quake_set_perturbations(quake_index, 3, 0, 0, 0);
                play.quake_set_duration(quake_index, 7);
                // (Rumble_Request(xyzDistToPlayerSq, 255, 20, 150) isn't ported.)
            }
            self.actor.kill();
            return;
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_WATER_TOUCH != 0 {
            // EffectSsGSplash_Spawn and three EffectSsGRipple_Spawn at the surface: not ported.
            self.actor.min_velocity_y = -6.0;
            let s = play.overlay_static::<Statics>(ACTOR_EN_ISHI);
            s.rot_speed_x >>= 2;
            s.rot_speed_y >>= 2;
            play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, 40, NA_SE_EV_DIVE_INTO_WATER_L);
            self.actor.bg_check_flags &= !BGCHECKFLAG_WATER_TOUCH;
        }
        eng_math::step_to_f(&mut self.actor.shape_y_offset, 0.0, 2.0);
        self.fall();
        Self::func_80a7ed94(&mut self.actor.velocity, D_80A7FA28[ty], play);
        self.actor.update_pos();
        let s = play.overlay_static::<Statics>(ACTOR_EN_ISHI);
        let (rx, ry) = (s.rot_speed_x, s.rot_speed_y);
        self.actor.shape_rot.x = self.actor.shape_rot.x.wrapping_add(rx);
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(ry);
        self.actor.update_bg_check_info(&play.col, 7.5, 35.0, 0.0, FLY_BGCHECK);
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
    }
}

/// `ISHI_GET_SWITCH_FLAG`.
fn switch_flag(actor: &Actor) -> i32 {
    ((SWITCH_FLAG_UPPER.get(actor.params) << 2) | SWITCH_FLAG_LOWER.get(actor.params)) as i32
}

/// `EnIshi_SpawnFragmentsSmall`'s `scales`.
pub const FRAGMENT_SCALES_SMALL: [i16; 6] = [16, 13, 11, 9, 7, 5];
/// `EnIshi_SpawnFragmentsLarge`'s `scales`.
pub const FRAGMENT_SCALES_LARGE: [i16; 9] = [145, 135, 120, 100, 70, 50, 45, 40, 35];

impl EnIshi {
    /// `sFragmentSpawnFuncs[type]`.
    pub fn spawn_fragments(&self, play: &mut PlayState, ty: usize) {
        if ty == ROCK_SMALL { self.spawn_fragments_small(play) } else { self.spawn_fragments_large(play) }
    }

    /// `sDustSpawnFuncs[type]`.
    pub fn spawn_dust(&self, play: &mut PlayState, ty: usize) {
        if ty == ROCK_SMALL { self.spawn_dust_small(play) } else { self.spawn_dust_large(play) }
    }

    /// The rock's velocity bounced off the ground (`BGCHECKFLAG_GROUND`: across `xz`, up `-y`)
    /// or a wall (`BGCHECKFLAG_WALL`: back `-xz`, `y`).
    fn bounced_velocity(&self, xz: f32, y: f32) -> Vec3 {
        let mut velocity = self.actor.velocity;
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            velocity.x *= xz;
            velocity.y *= -y;
            velocity.z *= xz;
        } else if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
            velocity.x *= -xz;
            velocity.y *= y;
            velocity.z *= -xz;
        }
        velocity
    }

    /// `EnIshi_SpawnFragmentsSmall`: six chips of `gFieldKakeraDL` (`scales`), within 4 across
    /// and 5 to 10 up, with the rock's bounced velocity ±5.5 across and up to 6 up; bouncing
    /// (collision mode 1, three bounces, radius 10), tumbling (`0x41` or `0x21`), gravity
    /// -420/256, 40 frames.
    fn spawn_fragments_small(&self, play: &mut PlayState) {
        let wp = self.actor.world_pos;
        let bounced = self.bounced_velocity(0.8, 0.8);
        play.with_ss(|ss| {
            for &scale in &FRAGMENT_SCALES_SMALL {
                let x = wp.x + (ss.rand.zero_one() - 0.5) * 8.0;
                let y = wp.y + (ss.rand.zero_one() * 5.0) + 5.0;
                let z = wp.z + (ss.rand.zero_one() - 0.5) * 8.0;
                let pos = Vec3::new(x, y, z);
                let mut velocity = bounced;
                velocity.x += (ss.rand.zero_one() - 0.5) * 11.0;
                velocity.y += ss.rand.zero_one() * 6.0;
                velocity.z += (ss.rand.zero_one() - 0.5) * 11.0;
                let phi_v0 = if ss.rand.zero_one() < 0.5 { 65 } else { 33 };
                ss.kakera_spawn(pos, velocity, pos, -420, phi_v0, 30, 5, 0, scale, 3, 10, 40, KAKERA_COLOR_NONE, OBJECT_GAMEPLAY_FIELD_KEEP, Some(("gameplay_field_keep", "gFieldKakeraDL")));
            }
        });
    }

    /// `EnIshi_SpawnFragmentsLarge`: nine pieces of `gSilverRockFragmentsDL` (`scales`), white,
    /// round the rock a turn of 0x4E20 apart from 0x1000 + 0x4E20 (up to 10 out, 5 to 45 up),
    /// with its bounced velocity plus up to 10 out and some up; the first big and heavy, the
    /// next three medium, the rest light; five bounces, 70 frames.
    fn spawn_fragments_large(&self, play: &mut PlayState) {
        let wp = self.actor.world_pos;
        let bounced = self.bounced_velocity(0.9, 0.8);
        play.with_ss(|ss| {
            let mut angle: i16 = 0x1000;
            for (i, &scale) in FRAGMENT_SCALES_LARGE.iter().enumerate() {
                angle = angle.wrapping_add(0x4E20);
                let rand = ss.rand.zero_one() * 10.0;
                let x = wp.x + (eng_math::sin_s(angle) * rand);
                let y = wp.y + (ss.rand.zero_one() * 40.0) + 5.0;
                let z = wp.z + (eng_math::cos_s(angle) * rand);
                let pos = Vec3::new(x, y, z);
                let mut velocity = bounced;
                let rand = ss.rand.zero_one() * 10.0;
                velocity.x += rand * eng_math::sin_s(angle);
                // (Rand_ZeroOne() * 4.0f) + ((Rand_ZeroOne() * i) * 0.7f): left to right, as IDO
                // evaluates the operands.
                let r1 = ss.rand.zero_one() * 4.0;
                let r2 = (ss.rand.zero_one() * i as f32) * 0.7;
                velocity.y += r1 + r2;
                velocity.z += rand * eng_math::cos_s(angle);
                let (phi_v0, phi_v1) = if i == 0 {
                    (41, -450)
                } else if i < 4 {
                    (37, -380)
                } else {
                    (69, -320)
                };
                ss.kakera_spawn(pos, velocity, wp, phi_v1, phi_v0, 30, 5, 0, scale, 5, 2, 70, KAKERA_COLOR_WHITE, OBJECT_GAMEPLAY_FIELD_KEEP, Some(("gameplay_field_keep", "gSilverRockFragmentsDL")));
            }
        });
    }

    /// The dust's centre: the rock's position, moved by twice its velocity off the ground or a
    /// wall.
    fn dust_pos(&self) -> Vec3 {
        let mut pos = self.actor.world_pos;
        let v = self.actor.velocity;
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            pos.x += 2.0 * v.x;
            pos.y -= 2.0 * v.y;
            pos.z += 2.0 * v.z;
        } else if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
            pos.x -= 2.0 * v.x;
            pos.y += 2.0 * v.y;
            pos.z -= 2.0 * v.z;
        }
        pos
    }

    /// `EnIshi_SpawnDustSmall`: four puffs 60 across, 80 big (`func_80033480`, lit).
    fn spawn_dust_small(&self, play: &mut PlayState) {
        func_80033480(play, self.dust_pos(), 60.0, 3, 0x50, 0x3C, 1);
    }

    /// `EnIshi_SpawnDustLarge`: eleven puffs 140 across, 180 big.
    fn spawn_dust_large(&self, play: &mut PlayState) {
        func_80033480(play, self.dust_pos(), 140.0, 0xA, 0xB4, 0x5A, 1);
    }
}

/// `sBreakSfxIds`, `sBreakSfxDurations` (the C sizes both 0x2852, `NA_SE_EV_ROCK_BROKEN`'s id).
const BREAK_SFX_IDS: [u16; 2] = [oot_game::audio::sfx::NA_SE_EV_ROCK_BROKEN, oot_game::audio::sfx::NA_SE_EV_WALL_BROKEN];
const BREAK_SFX_DURATIONS: [u16; 2] = [20, 40];

/// `EnIshi_SnapToFloor` / `EnKusa_SnapToFloor`: the floor below (from 30 above), plus `y`; the
/// home position follows. False if there's no floor.
pub(crate) fn snap_to_floor(actor: &mut Actor, play: &PlayState, y: f32) -> bool {
    // BgCheck_EntityRaycastDown4.
    let (floor, _) = play.col.entity_raycast_down(actor.world_pos + Vec3::Y * 30.0);
    if floor > eng_collision::bgcheck::BGCHECK_Y_MIN {
        actor.world_pos.y = floor + y;
        actor.home_pos = actor.world_pos;
        true
    } else {
        log::debug!("failure attaching to ground: {} at {}", actor.id, actor.world_pos);
        false
    }
}

impl ActorImpl for EnIshi {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnIshi_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Wait => self.wait(play),
            Action::LiftedUp => self.lifted_up(play),
            Action::Fly => self.fly(play),
        }
    }
    /// `EnIshi_Draw`: `EnIshi_DrawSmall` (`Gfx_DrawDListOpa`) or `EnIshi_DrawLarge` (the
    /// same with prim 255, which `Gfx_SetupDL_25Opa` leaves).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let dl = if self.ty() == ROCK_SMALL { "gFieldKakeraDL" } else { "gSilverRockDL" };
        crate::gfx_draw_dlist_opa(out, "gameplay_field_keep", dl, rs);
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
