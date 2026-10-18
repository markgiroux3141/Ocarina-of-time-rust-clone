//! `En_Kusa` (`ovl_En_Kusa/z_en_kusa.c`): a bush. The sword cuts it (`AC_HIT`): a type-0 bush
//! goes, types 1 and 2 stay as a stump, and type 1 grows back after 120 frames.
//!
//! A cut bush scatters its leaves (`EnKusa_SpawnFragments`: eight `EffectSsKakera`) and drops
//! from its table (`EnKusa_DropCollectible`: `Item_DropCollectibleRandom`, `crate::en_item00`).
//!
//! Within 100 it offers itself to Player's lift (`Actor_OfferCarry`); Link pulls it up only with
//! a strength upgrade (a child without the Goron's Bracelet tugs at it). Lifted
//! (`EnKusa_LiftedUp`) and let go, it falls tumbling (`EnKusa_Fall`) and breaks where it lands:
//! types 0 and 2 go, type 1 leaves its root, 9 down, which rises from frame 120 and regrows from
//! 170 (`EnKusa_UprootedWaitRegrow`).
//!
//! Not ported: the splash and ripples where it falls in water (`EffectSsGSplash`,
//! `EffectSsGRipple`). The bugs spawn as `En_Insect` (a placeholder).
//!
//! Params: `type` bits 0 and 1 (`EnKusaType`: 0 `ENKUSA_TYPE_0`, `gFieldBushDL` from
//! `gameplay_field_keep`, gone when cut; 1 `ENKUSA_TYPE_1`, `object_kusa`'s bush, regrowing; 2
//! `ENKUSA_TYPE_2`, `object_kusa`'s, staying a stump); `bugs` bit 4 (1: bugs hide in it);
//! `dropParams` bits 8 to 11 (types 0 and 2's collectible drop table, 13 and up read as 0; type 1
//! drops seeds or a heart); bits 12 to 15 unused (the data writes some drops as 0xFF, the C reads
//! four bits).
//! Flags: none.

use eng_collision::math3d::Cylinder16;
use glam::Vec3;
use oot_game::actor::{
    ACTOR_FLAG_THROW_ONLY, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, BGCHECKFLAG_GROUND, BGCHECKFLAG_GROUND_TOUCH, BGCHECKFLAG_WALL, BGCHECKFLAG_WATER, BGCHECKFLAG_WATER_TOUCH, UPDBGCHECKINFO_FLAG_0,
    UPDBGCHECKINFO_FLAG_2, UPDBGCHECKINFO_FLAG_7,
};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Kind, ObjectNeed};
use oot_game::collision_check::*;
use oot_game::effect::kakera::{KAKERA_COLOR_NONE, OBJECT_GAMEPLAY_KEEP};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

pub const ACTOR_EN_KUSA: i16 = 0x0125;
/// `ACTOR_EN_INSECT`, `INSECT_TYPE_SPAWNED`.
const ACTOR_EN_INSECT: i16 = 0x0020;
const INSECT_TYPE_SPAWNED: i16 = 1;
/// `ACTOR_FLAG_GRASS_DESTROYED` (`actor.h`: bit 11).
pub const ACTOR_FLAG_GRASS_DESTROYED: u32 = 1 << 11;

/// `En_Kusa_Profile` (no draw until its object is in: `EnKusa_WaitForObject` sets it).
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_KUSA, name: "En_Kusa", category: ACTORCAT_PROP, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_THROW_ONLY, object: "gameplay_keep", schema: &SCHEMA };

/// `PARAMS_GET_U(this->actor.params, 0, 2)`: the bush.
pub const TYPE: Field = Field::params(
    "type",
    0,
    2,
    Kind::Named(&[(0, "ENKUSA_TYPE_0: gone when cut"), (1, "ENKUSA_TYPE_1: regrows"), (2, "ENKUSA_TYPE_2: stays a stump")]),
    "which bush, and what cutting it leaves",
);
/// `PARAMS_GET_U(this->actor.params, 4, 1)`: bugs in it.
pub const BUGS: Field = Field::params("bugs", 4, 1, Kind::Named(&[(0, "no bugs"), (1, "bugs hide in it")]), "three bugs let out when it's cut or lifted");
/// `PARAMS_GET_U(this->actor.params, 8, 4)`: the drop table.
pub const DROP_TABLE: Field = Field::params("dropParams", 8, 4, Kind::Range(0, 15), "types 0 and 2's collectible drop table (13 and up: table 0)");
/// Bits 12 to 15: the game's data writes some drops as 0xFF; the C reads four bits.
pub const DROP_HIGH: Field = Field::params("dropParams bits 4 to 7", 12, 4, Kind::Unused, "0xF where the data writes a drop of 0xFF; read by nothing");

/// En_Kusa's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, BUGS, DROP_TABLE, DROP_HIGH],
    flags: &[],
    todo: false,
    // `sObjectIds[type]`: without it the bush is gone at once ("Bank danger!")
    objects: &[
        ObjectNeed::new("gameplay_field_keep").when("type", &[0]).note("the field's bush"),
        ObjectNeed::new("object_kusa").when("type", &[1, 2]).note("its own bush"),
    ],
};

/// `OBJECT_GAMEPLAY_FIELD_KEEP`, `OBJECT_KUSA` (`object_table.h`).
const OBJECT_GAMEPLAY_FIELD_KEEP: i16 = 0x0002;
const OBJECT_KUSA: i16 = 0x012B;
/// `sObjectIds`.
const OBJECT_IDS: [i16; 3] = [OBJECT_GAMEPLAY_FIELD_KEEP, OBJECT_KUSA, OBJECT_KUSA];

/// `sUnitDirections`: the four leaves' directions out of the bush.
const UNIT_DIRECTIONS: [Vec3; 4] = [Vec3::new(0.0, 0.7071, 0.7071), Vec3::new(0.7071, 0.7071, 0.0), Vec3::new(0.0, 0.7071, -0.7071), Vec3::new(-0.7071, 0.7071, 0.0)];

/// `sFragmentScales`.
pub const FRAGMENT_SCALES: [i16; 8] = [108, 102, 96, 84, 66, 55, 42, 38];

/// `ENKUSA_TYPE_*`.
const ENKUSA_TYPE_0: i16 = 0;
const ENKUSA_TYPE_1: i16 = 1;
const ENKUSA_TYPE_2: i16 = 2;

/// `sCylinderInit`.
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit {
        col_type: COL_MATERIAL_NONE,
        at_flags: AT_NONE,
        ac_flags: AC_ON | AC_TYPE_PLAYER,
        oc_flags1: OC1_ON | OC1_TYPE_PLAYER | OC1_TYPE_2,
        oc_flags2: OC2_TYPE_2,
        shape: COLSHAPE_CYLINDER,
    },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: 0, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x4FC0_0758, hit_backlash: 0, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 12, height: 44, y_shift: 0, pos: [0; 3] },
};

/// `sColChkInfoInit`.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 0, cyl_radius: 12, cyl_height: 30, mass: MASS_IMMOVABLE };

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `EnKusa_WaitForObject`.
    WaitObject,
    /// `EnKusa_Main`.
    Main,
    /// `EnKusa_CutWaitRegrow`.
    CutWaitRegrow,
    /// `EnKusa_DoNothing`.
    DoNothing,
    /// `EnKusa_Regrow`.
    Regrow,
    /// `EnKusa_LiftedUp`.
    LiftedUp,
    /// `EnKusa_Fall`.
    Fall,
    /// `EnKusa_UprootedWaitRegrow`.
    UprootedWaitRegrow,
}

/// `rotSpeedXtarget`, `rotSpeedX`, `rotSpeedYtarget`, `rotSpeedY`: the falling bush's tumble, the
/// overlay's statics.
#[derive(Debug, Default)]
pub struct Statics {
    pub rot_speed_x_target: i16,
    pub rot_speed_x: i16,
    pub rot_speed_y_target: i16,
    pub rot_speed_y: i16,
}

/// `NA_SE_PL_PULL_UP_PLANT` (`playerbank_table.h`: 0x869), `NA_SE_EV_DIVE_INTO_WATER_L`
/// (`environmentbank_table.h`: 0x28C5).
const NA_SE_PL_PULL_UP_PLANT: u16 = 0x0869;
const NA_SE_EV_DIVE_INTO_WATER_L: u16 = 0x28C5;
/// `Actor_UpdateBgCheckInfo`'s flags for the falling bush (`UPDBGCHECKINFO_FLAG_6`: no splash).
const FALL_BGCHECK: u32 = UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | (1 << 6) | UPDBGCHECKINFO_FLAG_7;

pub struct EnKusa {
    pub actor: Actor,
    pub collider: ColliderCylinder,
    pub timer: i16,
    /// `objBankIndex`: the bank of the bush's own object (`sObjectIds`).
    pub obj_bank: Option<usize>,
    pub action: Action,
    /// `actor.draw` is set once the object is in.
    drawn: bool,
}

impl EnKusa {
    fn ty(&self) -> i16 {
        TYPE.get(self.actor.params)
    }

    /// `EnKusa_SetupAction`.
    fn setup_action(&mut self, a: Action) {
        self.timer = 0;
        self.action = a;
    }

    /// `EnKusa_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: scale 0.4, gravity -3.2, minVelocityY -17; cullingVolumeDistance 1200,
        // cullingVolumeScale 100, cullingVolumeDownward 120 (a thousand further in a cutscene).
        actor.scale = Vec3::splat(0.4);
        actor.gravity = -3.2;
        actor.min_velocity_y = -17.0;
        actor.culling_volume_distance = 1200.0;
        actor.culling_volume_scale = 100.0;
        actor.culling_volume_downward = 120.0;
        if play.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE {
            actor.culling_volume_distance += 1000.0;
        }
        // EnKusa_InitCollider.
        let mut collider = ColliderCylinder::new(&CYLINDER_INIT);
        collider.update(&actor);
        actor.col_chk_info.set_info(None, &COL_CHK_INFO_INIT);
        if actor.shape_rot.y == 0 {
            let r = play.rand.zero_float(65536.0) as i32 as i16;
            actor.world_rot.y = r;
            actor.home_rot.y = r;
            actor.shape_rot.y = r;
        }
        let mut k = EnKusa { actor, collider, timer: 0, obj_bank: None, action: Action::WaitObject, drawn: false };
        if !crate::en_ishi::snap_to_floor(&mut k.actor, play, 0.0) {
            k.actor.kill();
            return Box::new(k);
        }
        let ty = TYPE.get(k.actor.params) as usize;
        k.obj_bank = OBJECT_IDS.get(ty).and_then(|&o| play.object_ctx.get_index(o));
        if k.obj_bank.is_none() {
            // "Bank danger!"
            // schema: printed
            log::debug!("En_Kusa {:#06x}: its object isn't in a bank", k.actor.params);
            k.actor.kill();
            return Box::new(k);
        }
        k.setup_action(Action::WaitObject);
        Box::new(k)
    }

    /// `EnKusa_WaitForObject`.
    fn wait_object(&mut self, play: &mut PlayState) {
        if self.obj_bank.is_some_and(|b| play.object_ctx.is_loaded(b)) {
            if self.actor.flags & ACTOR_FLAG_GRASS_DESTROYED != 0 {
                self.setup_cut();
            } else {
                self.setup_main();
            }
            self.drawn = true;
            self.actor.obj_bank_index = self.obj_bank;
            self.actor.flags &= !ACTOR_FLAG_UPDATE_CULLING_DISABLED;
        }
    }

    /// `EnKusa_DropCollectible`: types 0 and 2 from their table (bits 8..11, 0 past 0xC), type
    /// 1 seeds or a heart.
    fn drop_collectible(&self, play: &mut PlayState) {
        use crate::en_item00::*;
        let pos = self.actor.world_pos;
        match self.ty() {
            ENKUSA_TYPE_1 => {
                let id = if play.rand.zero_one() < 0.5 { ITEM00_SEEDS } else { ITEM00_RECOVERY_HEART };
                item_drop_collectible(play, pos, id);
            }
            _ => {
                let mut drop_params = DROP_TABLE.get(self.actor.params);
                if drop_params >= 0xD {
                    drop_params = 0;
                }
                item_drop_collectible_random(play, None, pos, drop_params << 4);
            }
        }
    }

    /// `EnKusa_SpawnFragments`: for each of `sUnitDirections`, a stalk (`gCuttableShrubStalkDL`)
    /// 20 out (scaled) and 10 up, flying ±4 across and up to 10 up, its scale
    /// `sFragmentScales[(s32)(Rand × 111.1) & 7]`; then a tip (`gCuttableShrubTipDL`) 40 out, ±3
    /// across, its scale index `% 7` (the last scale never comes up for a tip). Gravity -100/256,
    /// tumbling at medium speed, light drag (40, 3), 80 frames, falling 600 past Player's floor.
    pub fn spawn_fragments(&self, play: &mut PlayState) {
        let (wp, sc) = (self.actor.world_pos, self.actor.scale);
        play.with_ss(|ss| {
            for dir in &UNIT_DIRECTIONS {
                let pos = Vec3::new(wp.x + (dir.x * sc.x * 20.0), wp.y + (dir.y * sc.y * 20.0) + 10.0, wp.z + (dir.z * sc.z * 20.0));
                let vx = (ss.rand.zero_one() - 0.5) * 8.0;
                let vy = ss.rand.zero_one() * 10.0;
                let vz = (ss.rand.zero_one() - 0.5) * 8.0;
                let scale_index = ((ss.rand.zero_one() * 111.1) as i32 & 7) as usize;
                let (velocity, dl) = (Vec3::new(vx, vy, vz), Some(("gameplay_keep", "gCuttableShrubStalkDL")));
                ss.kakera_spawn(pos, velocity, pos, -100, 64, 40, 3, 0, FRAGMENT_SCALES[scale_index], 0, 0, 80, KAKERA_COLOR_NONE, OBJECT_GAMEPLAY_KEEP, dl);
                let pos = Vec3::new(wp.x + (dir.x * sc.x * 40.0), wp.y + (dir.y * sc.y * 40.0) + 10.0, wp.z + (dir.z * sc.z * 40.0));
                let vx = (ss.rand.zero_one() - 0.5) * 6.0;
                let vy = ss.rand.zero_one() * 10.0;
                let vz = (ss.rand.zero_one() - 0.5) * 6.0;
                let scale_index = ((ss.rand.zero_one() * 111.1) as i32 % 7) as usize;
                let (velocity, dl) = (Vec3::new(vx, vy, vz), Some(("gameplay_keep", "gCuttableShrubTipDL")));
                ss.kakera_spawn(pos, velocity, pos, -100, 64, 40, 3, 0, FRAGMENT_SCALES[scale_index], 0, 0, 80, KAKERA_COLOR_NONE, OBJECT_GAMEPLAY_KEEP, dl);
            }
        });
    }

    /// `EnKusa_SetupMain`.
    fn setup_main(&mut self) {
        self.setup_action(Action::Main);
        self.actor.flags &= !ACTOR_FLAG_UPDATE_CULLING_DISABLED;
    }

    /// `EnKusa_Main`.
    fn main(&mut self, play: &mut PlayState) {
        if self.actor.parent.is_some() {
            self.setup_lifted_up();
            play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, 20, NA_SE_PL_PULL_UP_PLANT);
        } else if self.collider.base.ac_flags & AC_HIT != 0 {
            self.collider.base.ac_flags &= !AC_HIT;
            self.spawn_fragments(play);
            self.drop_collectible(play);
            play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, 20, oot_game::audio::sfx::NA_SE_EV_PLANT_BROKEN);
            if BUGS.get(self.actor.params) != 0 {
                self.spawn_bugs(play);
            }
            if self.ty() == ENKUSA_TYPE_0 {
                self.actor.kill();
                return;
            }
            self.setup_cut();
            self.actor.flags |= ACTOR_FLAG_GRASS_DESTROYED;
        } else {
            if self.collider.base.oc_flags1 & OC1_TYPE_PLAYER == 0 && self.actor.xz_dist_to_player > 12.0 {
                self.collider.base.oc_flags1 |= OC1_TYPE_PLAYER;
            }
            if self.actor.xz_dist_to_player < 600.0 {
                self.collider.update(&self.actor);
                play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
                if self.actor.xz_dist_to_player < 400.0 {
                    play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
                    if self.actor.xz_dist_to_player < 100.0 {
                        let a = self.actor.clone();
                        oot_game::get_item::offer_pick_up(play, &a);
                    }
                }
            }
        }
    }

    /// `EnKusa_SetupLiftedUp`: in no room while carried, and updated off screen.
    fn setup_lifted_up(&mut self) {
        self.setup_action(Action::LiftedUp);
        self.actor.room = -1;
        self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
    }

    /// `EnKusa_UpdateVelY`: gravity, down to `minVelocityY`.
    fn update_vel_y(&mut self) {
        self.actor.velocity.y += self.actor.gravity;
        if self.actor.velocity.y < self.actor.min_velocity_y {
            self.actor.velocity.y = self.actor.min_velocity_y;
        }
    }

    /// `EnKusa_RandScaleVecToZero`: `vec` slowed by `scale`, give or take a tenth of it.
    fn rand_scale_vec_to_zero(vec: &mut Vec3, mut scale: f32, play: &mut PlayState) {
        scale += ((play.rand.zero_one() * 0.2) - 0.1) * scale;
        vec.x -= vec.x * scale;
        vec.y -= vec.y * scale;
        vec.z -= vec.z * scale;
    }

    /// `EnKusa_LiftedUp`: let go, it's back in the room and falls from where it is, along its yaw
    /// at its speed (the first frame's gravity -0.1, then -3.2), with the mass of a thrown thing.
    fn lifted_up(&mut self, play: &mut PlayState) {
        if self.actor.parent.is_none() {
            self.actor.room = play.room_ctx.cur.num;
            self.setup_fall(play);
            self.actor.velocity.x = self.actor.speed_xz * eng_math::sin_s(self.actor.world_rot.y);
            self.actor.velocity.z = self.actor.speed_xz * eng_math::cos_s(self.actor.world_rot.y);
            self.actor.col_chk_info.mass = 240;
            self.actor.gravity = -0.1;
            self.update_vel_y();
            Self::rand_scale_vec_to_zero(&mut self.actor.velocity, 0.005, play);
            self.actor.update_pos();
            self.actor.update_bg_check_info(&play.col, 7.5, 35.0, 0.0, FALL_BGCHECK);
            self.actor.gravity = -3.2;
        }
    }

    /// `EnKusa_SetupFall`: a tumble towards -0xBB8 on x and a random spin on y.
    fn setup_fall(&mut self, play: &mut PlayState) {
        self.setup_action(Action::Fall);
        let y_target = ((play.rand.zero_one() - 0.5) * 1600.0) as i16;
        let s = play.overlay_static::<Statics>(ACTOR_EN_KUSA);
        s.rot_speed_x_target = -0xBB8;
        s.rot_speed_y_target = y_target;
        s.rot_speed_x = 0;
        s.rot_speed_y = 0;
    }

    /// `EnKusa_Fall`: landing or hitting a wall it breaks (its sound out of water, its leaves, its
    /// drop): types 0 and 2 go, type 1 waits to regrow. Entering water it slows (a splash and
    /// ripples in the C) and tumbles half as fast. Otherwise it falls, tumbling, slowed by the air.
    fn fall(&mut self, play: &mut PlayState) {
        if self.actor.bg_check_flags & (BGCHECKFLAG_GROUND | BGCHECKFLAG_GROUND_TOUCH | BGCHECKFLAG_WALL) != 0 {
            if self.actor.bg_check_flags & BGCHECKFLAG_WATER == 0 {
                play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, 20, oot_game::audio::sfx::NA_SE_EV_PLANT_BROKEN);
            }
            self.spawn_fragments(play);
            self.drop_collectible(play);
            match self.ty() {
                ENKUSA_TYPE_0 | ENKUSA_TYPE_2 => self.actor.kill(),
                ENKUSA_TYPE_1 => self.setup_uprooted_wait_regrow(),
                _ => {}
            }
            return;
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_WATER_TOUCH != 0 {
            // EffectSsGSplash_Spawn and three EffectSsGRipple_Spawn at the surface: not ported.
            self.actor.min_velocity_y = -3.0;
            let s = play.overlay_static::<Statics>(ACTOR_EN_KUSA);
            s.rot_speed_x >>= 1;
            s.rot_speed_x_target >>= 1;
            s.rot_speed_y >>= 1;
            s.rot_speed_y_target >>= 1;
            self.actor.bg_check_flags &= !BGCHECKFLAG_WATER_TOUCH;
            play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, 40, NA_SE_EV_DIVE_INTO_WATER_L);
        }
        self.update_vel_y();
        let s = play.overlay_static::<Statics>(ACTOR_EN_KUSA);
        eng_math::step_to_s(&mut s.rot_speed_x, s.rot_speed_x_target, 0x1F4);
        eng_math::step_to_s(&mut s.rot_speed_y, s.rot_speed_y_target, 0xAA);
        let (rx, ry) = (s.rot_speed_x, s.rot_speed_y);
        self.actor.shape_rot.x = self.actor.shape_rot.x.wrapping_add(rx);
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(ry);
        Self::rand_scale_vec_to_zero(&mut self.actor.velocity, 0.05, play);
        self.actor.update_pos();
        self.actor.update_bg_check_info(&play.col, 7.5, 35.0, 0.0, FALL_BGCHECK);
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
    }

    /// `EnKusa_SetupUprootedWaitRegrow`: its root back home, 9 down, small and unturned.
    fn setup_uprooted_wait_regrow(&mut self) {
        self.actor.world_pos.x = self.actor.home_pos.x;
        self.actor.world_pos.y = self.actor.home_pos.y - 9.0;
        self.actor.world_pos.z = self.actor.home_pos.z;
        self.set_scale_small();
        self.actor.shape_rot = self.actor.home_rot;
        self.setup_action(Action::UprootedWaitRegrow);
    }

    /// `EnKusa_UprootedWaitRegrow`: from frame 121 it rises 0.6 a frame home, and regrows from
    /// frame 170 once there.
    fn uprooted_wait_regrow(&mut self) {
        if self.timer > 120 && eng_math::step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y, 0.6) && self.timer >= 170 {
            self.setup_regrow();
        }
    }

    /// `EnKusa_SpawnBugs`: three `En_Insect`s.
    fn spawn_bugs(&mut self, play: &mut PlayState) {
        for _ in 0..3 {
            let yaw = (play.rand.zero_one() * 65535.0) as i32 as i16;
            if play.actor_spawn(ACTOR_EN_INSECT, self.actor.world_pos, [0, yaw, 0], INSECT_TYPE_SPAWNED).is_err() {
                break;
            }
        }
    }

    /// `EnKusa_SetupCut`.
    fn setup_cut(&mut self) {
        match self.ty() {
            ENKUSA_TYPE_2 => self.setup_action(Action::DoNothing),
            ENKUSA_TYPE_1 => self.setup_action(Action::CutWaitRegrow),
            _ => {}
        }
    }

    /// `EnKusa_SetScaleSmall`.
    fn set_scale_small(&mut self) {
        self.actor.scale = Vec3::new(0.120000005, 0.16000001, 0.120000005);
    }

    /// `EnKusa_SetupRegrow`.
    fn setup_regrow(&mut self) {
        self.setup_action(Action::Regrow);
        self.set_scale_small();
        self.actor.shape_rot = self.actor.home_rot;
        self.actor.flags &= !ACTOR_FLAG_GRASS_DESTROYED;
    }

    /// `EnKusa_Regrow`.
    fn regrow(&mut self) {
        let mut grown = eng_math::step_to_f(&mut self.actor.scale.y, 0.4, 0.014);
        grown &= eng_math::step_to_f(&mut self.actor.scale.x, 0.4, 0.011);
        self.actor.scale.z = self.actor.scale.x;
        if grown {
            self.actor.scale = Vec3::splat(0.4);
            self.setup_main();
            self.collider.base.oc_flags1 &= !OC1_TYPE_PLAYER;
        }
    }
}

impl ActorImpl for EnKusa {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnKusa_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.timer = self.timer.wrapping_add(1);
        match self.action {
            Action::WaitObject => self.wait_object(play),
            Action::Main => self.main(play),
            Action::CutWaitRegrow => {
                if self.timer >= 120 {
                    self.setup_regrow();
                }
            }
            Action::DoNothing => {}
            Action::Regrow => self.regrow(),
            Action::LiftedUp => self.lifted_up(play),
            Action::Fall => self.fall(play),
            Action::UprootedWaitRegrow => self.uprooted_wait_regrow(),
        }
        self.actor.shape_y_offset = if self.actor.flags & ACTOR_FLAG_GRASS_DESTROYED != 0 { -6.25 } else { 0.0 };
    }
    /// `EnKusa_Draw`: the stump when cut, else the type's bush.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if !self.drawn {
            return;
        }
        let (file, dl) = if self.actor.flags & ACTOR_FLAG_GRASS_DESTROYED != 0 {
            ("object_kusa", "object_kusa_DL_0002E0")
        } else if self.ty() == ENKUSA_TYPE_0 {
            ("gameplay_field_keep", "gFieldBushDL")
        } else {
            ("object_kusa", "object_kusa_DL_000140")
        };
        crate::gfx_draw_dlist_opa(out, file, dl, rs);
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
