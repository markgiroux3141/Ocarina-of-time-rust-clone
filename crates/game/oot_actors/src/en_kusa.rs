//! `En_Kusa` (`ovl_En_Kusa/z_en_kusa.c`): a bush. The sword cuts it (`AC_HIT`): a type-0 bush
//! goes, types 1 and 2 stay as a stump, and type 1 grows back after 120 frames.
//!
//! Params: bits 0..1 the type (0 `gFieldBushDL` from `gameplay_field_keep`, 1 and 2
//! `object_kusa`'s bush), bit 4 bugs hide in it, bits 8..11 the drop table.
//!
//! A cut bush scatters its leaves (`EnKusa_SpawnFragments`: eight `EffectSsKakera`) and drops
//! from its table (`EnKusa_DropCollectible`: `Item_DropCollectibleRandom`, `crate::en_item00`).
//!
//! Not ported: lifting and throwing (Player can't lift yet: `Actor_HasParent` is never true, so
//! `EnKusa_LiftedUp`, `EnKusa_Fall` and `EnKusa_UprootedWaitRegrow` aren't reached). The bugs
//! spawn as `En_Insect` (a placeholder).

use eng_collision::math3d::Cylinder16;
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_THROW_ONLY, Actor};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile};
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
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_KUSA, name: "En_Kusa", category: ACTORCAT_PROP, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_THROW_ONLY, object: "gameplay_keep" };

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
}

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
        self.actor.params & 3
    }

    /// `EnKusa_SetupAction`.
    fn setup_action(&mut self, a: Action) {
        self.timer = 0;
        self.action = a;
    }

    /// `EnKusa_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: scale 0.4, gravity -3.2, minVelocityY -17 (the cull zone isn't ported).
        actor.scale = Vec3::splat(0.4);
        actor.gravity = -3.2;
        actor.min_velocity_y = -17.0;
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
        let ty = (k.actor.params & 3) as usize;
        k.obj_bank = OBJECT_IDS.get(ty).and_then(|&o| play.object_ctx.get_index(o));
        if k.obj_bank.is_none() {
            // "Bank danger!"
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
                let mut drop_params = (self.actor.params >> 8) & 0xF;
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
        // Actor_HasParent (lifted): Player doesn't lift things yet.
        if self.collider.base.ac_flags & AC_HIT != 0 {
            self.collider.base.ac_flags &= !AC_HIT;
            self.spawn_fragments(play);
            self.drop_collectible(play);
            play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, 20, oot_game::audio::sfx::NA_SE_EV_PLANT_BROKEN);
            if (self.actor.params >> 4) & 1 != 0 {
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
                    // < 100: Actor_OfferCarry offers the bush to Player's lift (not ported).
                }
            }
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
