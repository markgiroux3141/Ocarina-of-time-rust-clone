//! `Obj_Tsubo` (`ovl_Obj_Tsubo/z_obj_tsubo.c`): a pot. `params` bits 0..4 the drop (`ITEM00_*`,
//! dropped as it breaks), bit 8 the kind (0 `gameplay_dangeon_keep`'s `gPotDL`, 1 `object_tsubo`'s
//! `object_tsubo_DL_0017C0`), bits 9..14 the drop's collectible flag.
//!
//! Without its object in the scene's list it is gone at once ("Bank danger!": Hyrule Castle's two
//! pots ask for `object_tsubo`, which the castle doesn't load); it waits for its object to load,
//! then stands on the floor below it. A sword, an arrow, a hammer, a boomerang or an explosion
//! breaks it into 15 pieces (`EffectSsKakera`, its kind's fragment list) and dust, with
//! `NA_SE_EV_POT_BROKEN`; deeper than 15 in water it breaks with a splash. Link facing it within
//! 100 is offered it to lift (`Actor_OfferGetItem` with `GI_NONE`); lifted it follows him, and
//! thrown it tumbles until it hits the ground, a wall or an actor (its `AT`), or water.
//!
//! The whole overlay is ported. Player doesn't lift yet (BACKLOG #4), so the lifted and thrown
//! states aren't reached.
//!
//! Params: `dropParams` bits 0 to 4 (the collectible dropped as it breaks, `ITEM00_*`; 0x1A and
//! up none); bit 5 unused (the data writes "no drop" as 0x3F, six bits); bits 6 and 7 unused; bit 8 the pot (`sObjectIds`: 0 `gameplay_dangeon_keep`'s, 1
//! `object_tsubo`'s); bits 9 to 14 the drop's collectible flag; bit 15 unused.
//! Flags: none itself; the `En_Item00` it drops reads and sets collectible flag bits 9 to 14.

use eng_collision::math3d::Cylinder16;
use eng_math::{cos_s, sin_s, step_to_s};
use glam::Vec3;
use oot_game::actor::{
    ACTOR_FLAG_THROW_ONLY, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, BGCHECKFLAG_GROUND, BGCHECKFLAG_GROUND_TOUCH, BGCHECKFLAG_WALL, BGCHECKFLAG_WATER, BGCHECKFLAG_WATER_TOUCH,
    UPDBGCHECKINFO_FLAG_0, UPDBGCHECKINFO_FLAG_2, UPDBGCHECKINFO_FLAG_7,
};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile, func_80033480, player_play_sfx};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, ObjectNeed};
use oot_game::collision_check::*;
use oot_game::effect::kakera::KAKERA_COLOR_NONE;
use oot_game::item::GI_NONE;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

/// `ACTOR_OBJ_TSUBO` (`actor_table.h`: 0x0111).
pub const ACTOR_OBJ_TSUBO: i16 = 0x0111;

/// `Obj_Tsubo_Profile`.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_OBJ_TSUBO, name: "Obj_Tsubo", category: ACTORCAT_PROP, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_THROW_ONLY, object: "gameplay_keep", schema: &SCHEMA };

/// `PARAMS_GET_U(params, 0, 5)`: the drop.
pub const DROP: Field = Field::params("dropParams", 0, 5, Kind::Drop, "the collectible it drops as it breaks (0x1A and up: none)");
/// Bit 5: the game's data writes "no drop" as 0x3F; the C reads five bits.
pub const DROP_BIT_5: Field = Field::params("dropParams bit 5", 5, 1, Kind::Unused, "set with a drop of 0x3F (none) in the data; read by nothing");
/// `PARAMS_GET_U(params, 8, 1)`: the pot, an index into `sObjectIds`.
pub const OBJECT_INDEX: Field =
    Field::params("objectIndex", 8, 1, Kind::Named(&[(0, "gameplay_dangeon_keep's pot"), (1, "object_tsubo's pot")]), "which pot, by its object");
/// `PARAMS_GET_U(params, 9, 6)`: the drop's collectible flag.
pub const COLLECTIBLE_FLAG: Field = Field::params("collectibleFlag", 9, 6, Kind::CollectibleFlag, "the drop's collectible flag");

/// Obj_Tsubo's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[DROP, DROP_BIT_5, OBJECT_INDEX, COLLECTIBLE_FLAG],
    flags: &[FlagUse::new(Flag::Collectible("collectibleFlag")).reads().sets().indirect("by the En_Item00 it drops")],
    todo: false,
    // `sObjectIds[objectIndex]`: without it the pot is gone at once
    objects: &[
        ObjectNeed::new("gameplay_dangeon_keep").when("objectIndex", &[0]).note("the dungeons' pot"),
        ObjectNeed::new("object_tsubo").when("objectIndex", &[1]).note("its own pot"),
    ],
    texts: &[],
};

/// `sObjectIds`: `OBJECT_GAMEPLAY_DANGEON_KEEP`, `OBJECT_TSUBO` (`object_table.h`: 0x0003, 0x012C),
/// with their files.
pub const OBJECT_IDS: [(i16, &str); 2] = [(0x0003, "gameplay_dangeon_keep"), (0x012C, "object_tsubo")];
/// `D_80BA1B84`: the pots.
pub const POT_DLS: [&str; 2] = ["gPotDL", "object_tsubo_DL_0017C0"];
/// `D_80BA1B8C`: their fragments.
pub const FRAGMENT_DLS: [&str; 2] = ["gPotFragmentDL", "object_tsubo_DL_001960"];

/// `NA_SE_EV_POT_BROKEN` (`environmentbank_table.h`: 0x2887), `NA_SE_PL_PULL_UP_POT`
/// (`playerbank_table.h`: 0x086F).
pub const NA_SE_EV_POT_BROKEN: u16 = 0x2887;
pub const NA_SE_PL_PULL_UP_POT: u16 = 0x086F;
/// `ITEM00_MAX` (`z_en_item00.h`: 0x1A).
const ITEM00_MAX: i16 = 0x1A;

/// `sCylinderInit`.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit {
        col_type: COL_MATERIAL_HARD,
        at_flags: AT_ON | AT_TYPE_PLAYER,
        ac_flags: AC_ON | AC_TYPE_PLAYER,
        oc_flags1: OC1_ON | OC1_TYPE_ALL,
        oc_flags2: OC2_TYPE_2,
        shape: COLSHAPE_CYLINDER,
    },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0002, hit_special_effect: 0, damage: 0x01 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x4FC1_FFFE, hit_backlash: 0, defense: 0 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 9, height: 26, y_shift: 0, pos: [0; 3] },
};

/// `sColChkInfoInit`.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 0, cyl_radius: 12, cyl_height: 60, mass: MASS_IMMOVABLE };

/// The overlay's statics: a thrown pot's tumble (`D_80BA1B50`, `_54`, `_58`, `_5C`: the target and
/// current spin about x and y).
#[derive(Debug, Default)]
pub struct Statics {
    pub d_80ba1b50: i16,
    pub d_80ba1b54: i16,
    pub d_80ba1b58: i16,
    pub d_80ba1b5c: i16,
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    WaitForObject,
    Idle,
    LiftedUp,
    Thrown,
}

pub struct ObjTsubo {
    pub actor: Actor,
    pub action: Action,
    pub collider: ColliderCylinder,
    /// `requiredObjectSlot`.
    pub required_object_slot: Option<usize>,
    /// `actor.draw`: set once the object is loaded.
    pub drawn: bool,
}

impl ObjTsubo {
    /// `PARAMS_GET_U(params, 8, 1)`: the kind.
    fn kind(&self) -> usize {
        OBJECT_INDEX.get(self.actor.params) as usize
    }

    /// `ObjTsubo_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = ObjTsubo { actor, action: Action::WaitForObject, collider: ColliderCylinder::new(&CYLINDER_INIT), required_object_slot: None, drawn: false };
        // sInitChain.
        this.actor.gravity = -1.2;
        this.actor.min_velocity_y = -20.0;
        this.actor.scale = Vec3::splat(0.15);
        this.actor.culling_volume_distance = 900.0;
        this.actor.culling_volume_scale = 100.0;
        this.actor.culling_volume_downward = 800.0;
        // ObjTsubo_InitCollider.
        this.collider.update(&this.actor);
        this.actor.col_chk_info.set_info(None, &COL_CHK_INFO_INIT);
        if !this.snap_to_floor(play) {
            this.actor.kill();
            return Box::new(this);
        }
        let (id, _) = OBJECT_IDS[this.kind()];
        this.required_object_slot = play.object_ctx.get_index(id);
        if this.required_object_slot.is_none() {
            log::debug!("Error : Bank danger! (arg_data 0x{:04x})(../z_obj_tsubo.c 410)", this.actor.params); // schema: printed
            this.actor.kill();
        } else {
            this.action = Action::WaitForObject;
            log::debug!("(dungeon keep pot)(arg_data 0x{:04x})", this.actor.params); // schema: printed
        }
        Box::new(this)
    }

    /// `ObjTsubo_SpawnCollectible`: the drop in bits 0..4 with the flag in bits 9..14.
    fn spawn_collectible(&self, play: &mut PlayState) {
        let drop_params = DROP.get(self.actor.params);
        if (0..ITEM00_MAX).contains(&drop_params) {
            crate::en_item00::item_drop_collectible(play, self.actor.world_pos, drop_params | (COLLECTIBLE_FLAG.get(self.actor.params) << 8));
        }
    }

    /// `ObjTsubo_ApplyGravity`.
    fn apply_gravity(&mut self) {
        self.actor.velocity.y += self.actor.gravity;
        if self.actor.velocity.y < self.actor.min_velocity_y {
            self.actor.velocity.y = self.actor.min_velocity_y;
        }
    }

    /// `ObjTsubo_SnapToFloor`: the floor from 20 above (`BgCheck_EntityRaycastDown4`), the home
    /// there.
    fn snap_to_floor(&mut self, play: &PlayState) -> bool {
        let (ground_y, _) = play.col.entity_raycast_down(self.actor.world_pos + Vec3::Y * 20.0);
        if ground_y > eng_collision::bgcheck::BGCHECK_Y_MIN {
            self.actor.world_pos.y = ground_y;
            self.actor.home_pos = self.actor.world_pos;
            true
        } else {
            log::debug!("Failed to attach to ground");
            false
        }
    }

    /// `ObjTsubo_AirBreak`: 15 pieces round the pot (a turn of 0x4E20 apart, 8 out, 2 to 7 up),
    /// thrown out and up, a fifth big, two fifths medium; gravity -240/256, 60 frames; then dust.
    fn air_break(&self, play: &mut PlayState) {
        let wp = self.actor.world_pos;
        let k = self.kind();
        let (obj_id, file) = OBJECT_IDS[k];
        play.with_ss(|ss| {
            let mut angle: i16 = 0;
            for _ in 0..15 {
                let sins = sin_s(angle);
                let coss = cos_s(angle);
                let mut pos = Vec3::new(sins * 8.0, (ss.rand.zero_one() * 5.0) + 2.0, coss * 8.0);
                let velocity = Vec3::new(pos.x * 0.23, (ss.rand.zero_one() * 5.0) + 2.0, pos.z * 0.23);
                pos += wp;
                let rand = ss.rand.zero_one();
                let arg5 = if rand < 0.2 {
                    96
                } else if rand < 0.6 {
                    64
                } else {
                    32
                };
                let scale = ((ss.rand.zero_one() * 95.0) + 15.0) as i16;
                ss.kakera_spawn(pos, velocity, wp, -240, arg5, 10, 10, 0, scale, 0, 32, 60, KAKERA_COLOR_NONE, obj_id, Some((file, FRAGMENT_DLS[k])));
                angle = angle.wrapping_add(0x4E20);
            }
        });
        func_80033480(play, wp, 30.0, 4, 20, 50, 1);
    }

    /// `ObjTsubo_WaterBreak`: the splash at the water's surface (400), and 15 pieces sinking
    /// (gravity -180/256, 70 frames).
    fn water_break(&self, play: &mut PlayState) {
        let wp = self.actor.world_pos;
        let mut splash = wp;
        splash.y += self.actor.y_dist_to_water;
        play.with_ss(|s| s.g_splash_spawn(splash, None, None, 0, 400));
        let k = self.kind();
        let (obj_id, file) = OBJECT_IDS[k];
        play.with_ss(|ss| {
            let mut angle: i16 = 0;
            for _ in 0..15 {
                let sins = sin_s(angle);
                let coss = cos_s(angle);
                let mut pos = Vec3::new(sins * 8.0, (ss.rand.zero_one() * 5.0) + 2.0, coss * 8.0);
                let velocity = Vec3::new(pos.x * 0.2, (ss.rand.zero_one() * 4.0) + 2.0, pos.z * 0.2);
                pos += wp;
                let phi_s0 = if ss.rand.zero_one() < 0.2 { 64 } else { 32 };
                let scale = ((ss.rand.zero_one() * 95.0) + 15.0) as i16;
                ss.kakera_spawn(pos, velocity, wp, -180, phi_s0, 30, 30, 0, scale, 0, 32, 70, KAKERA_COLOR_NONE, obj_id, Some((file, FRAGMENT_DLS[k])));
                angle = angle.wrapping_add(0x4E20);
            }
        });
    }

    /// `NA_SE_EV_POT_BROKEN` where it broke, for 20 frames.
    fn broken_sfx(&self, play: &mut PlayState) {
        play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, 20, NA_SE_EV_POT_BROKEN);
    }

    /// `ObjTsubo_WaitForObject`.
    fn wait_for_object(&mut self, play: &PlayState) {
        if self.required_object_slot.is_some_and(|s| play.object_ctx.is_loaded(s)) {
            self.drawn = true;
            self.actor.obj_bank_index = self.required_object_slot;
            self.action = Action::Idle;
            self.actor.flags &= !ACTOR_FLAG_UPDATE_CULLING_DISABLED;
        }
    }

    /// `ObjTsubo_Idle`.
    fn idle(&mut self, play: &mut PlayState) {
        if self.actor.parent.is_some() {
            self.setup_lifted_up(play);
        } else if self.actor.bg_check_flags & BGCHECKFLAG_WATER != 0 && self.actor.y_dist_to_water > 15.0 {
            self.water_break(play);
            self.broken_sfx(play);
            self.spawn_collectible(play);
            self.actor.kill();
        } else if self.collider.base.ac_flags & AC_HIT != 0
            && self.collider.info.ac_hit_elem.is_some_and(|h| h.at_dmg_info.dmg_flags & (DMG_SWORD | DMG_RANGED | DMG_HAMMER | DMG_BOOMERANG | DMG_EXPLOSIVE) != 0)
        {
            self.air_break(play);
            self.spawn_collectible(play);
            self.broken_sfx(play);
            self.actor.kill();
        } else {
            if self.actor.xz_dist_to_player < 600.0 {
                self.collider.update(&self.actor);
                self.collider.base.ac_flags &= !AC_HIT;
                play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
                if self.actor.xz_dist_to_player < 150.0 {
                    play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
                }
            }
            if self.actor.xz_dist_to_player < 100.0 {
                let player_yaw = play.player.and_then(|h| play.actors.actor(h)).map_or(0, |p| p.world_rot.y);
                let temp_v0 = self.actor.yaw_towards_player.wrapping_sub(player_yaw);
                if (temp_v0 as i32).abs() >= 0x5556 {
                    // GI_NONE in this case allows the player to lift the actor.
                    let a = self.actor.clone();
                    oot_game::get_item::offer_get_item_range(play, &a, GI_NONE, 30.0, 30.0);
                }
            }
        }
    }

    /// `ObjTsubo_SetupLiftedUp`. `@bug (game)`: `Player_PlaySfx` on the pot cast to Player (its
    /// own `projectedPos`, so the sound still plays).
    fn setup_lifted_up(&mut self, play: &mut PlayState) {
        self.action = Action::LiftedUp;
        self.actor.room = -1;
        if let Some(me) = play.cur_actor {
            player_play_sfx(play, me, NA_SE_PL_PULL_UP_POT);
        }
        self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
    }

    /// `ObjTsubo_LiftedUp`: let go, it falls from where it is.
    fn lifted_up(&mut self, play: &mut PlayState) {
        if self.actor.parent.is_none() {
            self.actor.room = play.room_ctx.cur.num;
            self.setup_thrown(play);
            self.apply_gravity();
            self.actor.update_pos();
            self.actor.update_bg_check_info(&play.col, 5.0, 15.0, 0.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_7);
        }
    }

    /// `ObjTsubo_SetupThrown`: its velocity along its yaw at its speed; a random tumble.
    fn setup_thrown(&mut self, play: &mut PlayState) {
        self.actor.velocity.x = sin_s(self.actor.world_rot.y) * self.actor.speed_xz;
        self.actor.velocity.z = cos_s(self.actor.world_rot.y) * self.actor.speed_xz;
        self.actor.col_chk_info.mass = 240;
        let a = ((play.rand.zero_one() - 0.7) * 2800.0) as i16;
        let b = ((play.rand.zero_one() - 0.5) * 2000.0) as i16;
        let s = play.overlay_static::<Statics>(ACTOR_OBJ_TSUBO);
        s.d_80ba1b50 = a;
        s.d_80ba1b58 = b;
        s.d_80ba1b54 = 0;
        s.d_80ba1b5c = 0;
        self.action = Action::Thrown;
    }

    /// `ObjTsubo_Thrown`.
    fn thrown(&mut self, play: &mut PlayState) {
        if self.actor.bg_check_flags & (BGCHECKFLAG_GROUND | BGCHECKFLAG_GROUND_TOUCH | BGCHECKFLAG_WALL) != 0 || self.collider.base.at_flags & AT_HIT != 0 {
            self.air_break(play);
            self.spawn_collectible(play);
            self.broken_sfx(play);
            self.actor.kill();
        } else if self.actor.bg_check_flags & BGCHECKFLAG_WATER_TOUCH != 0 {
            self.water_break(play);
            self.spawn_collectible(play);
            self.broken_sfx(play);
            self.actor.kill();
        } else {
            self.apply_gravity();
            self.actor.update_pos();
            let s = play.overlay_static::<Statics>(ACTOR_OBJ_TSUBO);
            step_to_s(&mut s.d_80ba1b54, s.d_80ba1b50, 0x64);
            step_to_s(&mut s.d_80ba1b5c, s.d_80ba1b58, 0x64);
            let (rx, ry) = (s.d_80ba1b54, s.d_80ba1b5c);
            self.actor.shape_rot.x = self.actor.shape_rot.x.wrapping_add(rx);
            self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(ry);
            self.actor.update_bg_check_info(&play.col, 5.0, 15.0, 0.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_7);
            self.collider.update(&self.actor);
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
            play.collision_check_set_at(&self.actor, 0, &mut self.collider);
        }
    }
}

impl ActorImpl for ObjTsubo {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `ObjTsubo_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::WaitForObject => self.wait_for_object(play),
            Action::Idle => self.idle(play),
            Action::LiftedUp => self.lifted_up(play),
            Action::Thrown => self.thrown(play),
        }
    }
    fn render_state(&self) -> RenderState {
        RenderState::of(&self.actor)
    }
    /// `ObjTsubo_Draw`: `Gfx_DrawDListOpa` of the kind's pot.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if !self.drawn {
            return;
        }
        let k = self.kind();
        crate::gfx_draw_dlist_opa(out, OBJECT_IDS[k].1, POT_DLS[k], rs);
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
