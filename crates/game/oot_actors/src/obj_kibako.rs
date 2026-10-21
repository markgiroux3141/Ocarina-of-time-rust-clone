//! `Obj_Kibako` (`ovl_Obj_Kibako/z_obj_kibako.c`): the small wooden crate of the dungeons
//! (`gameplay_dangeon_keep`'s `gSmallWoodenBoxDL`), which Link lifts, carries and throws, and which
//! weighs down a floor switch (`ACTOR_FLAG_CAN_PRESS_SWITCHES`: Jabu-Jabu's blue ones,
//! `Bg_Bdan_Switch`).
//!
//! Master Quest's Jabu-Jabu (`bdan`) places four, all params 0xFFFF (no drop), the same in layers
//! 0 to 3: room 4's at (-1341, -12, -2116) and (-1381, -12, -2115), room 7's at (-666, -340, -4671)
//! and (-512, -340, -4592) (room 7's blue switch, flag 0x0D, is at (1, -341, -5178)).
//!
//! - **Standing** (`ObjKibako_Idle`): falling (gravity -1.2, at most 13 a frame) onto the floor
//!   (`Actor_MoveXZGravity`, the floor and walls 19 up and 20 out); within 600 across hittable
//!   (12 across, 27 high, `0x4FC00748`: a sword, the hammer, an explosion, arrows, the slingshot,
//!   a Deku stick...) and within 180 pushing (its OC, mass `MASS_HEAVY`; Link's push off until
//!   he's 28 away after it's put down); within 100 it offers itself to Player's lift
//!   (`Actor_OfferCarry`). Hit, it **breaks** (`ObjKibako_AirBreak`); over 19 deep in water it
//!   breaks in the water (`ObjKibako_WaterBreak`).
//! - **Held** (`ObjKibako_Held`, `actor.room` -1: it goes where Link goes;
//!   `NA_SE_PL_PULL_UP_WOODBOX`): let go (`Actor_HasNoParent`) into the room he's in; put down
//!   (speed under 0.1, `NA_SE_EV_PUT_DOWN_WOODBOX`) it stands again; thrown it **flies**.
//! - **Thrown** (`ObjKibako_Thrown`, mass 240): along its yaw at its speed, falling; hurting
//!   (its AT: 1, `DMG_DEKU_STICK`) and pushing. It breaks on the ground, a wall or anything its
//!   AT touches (in the air), or on touching water (in the water).
//! - **Breaking**: in the air twelve splinters (`EffectSsKakera` of `gSmallWoodenBoxFragmentDL`, a
//!   turn of 0x4E20 apart, 16 out, a tenth big, six tenths medium, 60 frames) and dust
//!   (`func_80033480`, 40 across, four puffs); in water a splash (`EffectSsGSplash`, 500, at the
//!   surface) and twelve slower sinking splinters (70 frames); `NA_SE_EV_WOODBOX_BREAK` where it
//!   was (20 frames), its drop (`Item_DropCollectible`), and it's gone.
//!
//! The whole overlay is ported. Its draw (`ObjKibako_Draw`) is `Gfx_DrawDListOpa` of
//! `gSmallWoodenBoxDL`.
//!
//! Params: `collectible` bits 0 to 4 (the drop as it breaks, `ITEM00_*`; 0x1A and up none);
//! bits 5 to 7 unused (set in the data's 0xFFFF); `collectibleFlag` bits 8 to 13 (the drop's
//! collectible flag); bits 14 and 15 unused.
//! Flags: none itself; the `En_Item00` it drops reads and sets collectible flag bits 8 to 13.

use eng_collision::math3d::Cylinder16;
use eng_math::{cos_s, sin_s};
use glam::Vec3;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile, audio_play_actor_sfx2, func_80033480, player_play_sfx};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::collision_check::*;
use oot_game::effect::kakera::KAKERA_COLOR_NONE;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

/// `ACTOR_OBJ_KIBAKO` (`actor_table.h`: 0x0110).
pub const ACTOR_OBJ_KIBAKO: i16 = 0x0110;
/// `OBJECT_GAMEPLAY_DANGEON_KEEP` (`object_table.h`: 0x0003).
pub const OBJECT: &str = "gameplay_dangeon_keep";
pub const OBJECT_GAMEPLAY_DANGEON_KEEP: i16 = 0x0003;
/// `gSmallWoodenBoxDL`, `gSmallWoodenBoxFragmentDL` (`gameplay_dangeon_keep.xml`).
pub const DL: &str = "gSmallWoodenBoxDL";
pub const FRAGMENT_DL: &str = "gSmallWoodenBoxFragmentDL";

/// `NA_SE_EV_WOODBOX_BREAK`, `NA_SE_EV_PUT_DOWN_WOODBOX` (`environmentbank_table.h`: 0x28AA,
/// 0x28AB), `NA_SE_PL_PULL_UP_WOODBOX` (`playerbank_table.h`: 0x878).
pub const NA_SE_EV_WOODBOX_BREAK: u16 = 0x28AA;
pub const NA_SE_EV_PUT_DOWN_WOODBOX: u16 = 0x28AB;
pub const NA_SE_PL_PULL_UP_WOODBOX: u16 = 0x0878;
/// `ITEM00_MAX` (`z_en_item00.h`: 0x1A).
const ITEM00_MAX: i16 = 0x1A;

/// `Obj_Kibako_Profile`: `ACTORCAT_PROP`, always updating, presses switches,
/// `OBJECT_GAMEPLAY_DANGEON_KEEP`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_OBJ_KIBAKO,
    name: "Obj_Kibako",
    category: ACTORCAT_PROP,
    flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_CAN_PRESS_SWITCHES,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `PARAMS_GET_U(params, 0, 5)`: the drop.
pub const COLLECTIBLE: Field = Field::params("collectible", 0, 5, Kind::Drop, "the collectible it drops as it breaks (0x1A and up: none)");
/// Bits 5 to 7: set in the data (0xFFFF), read by nothing.
pub const UNUSED_LOW: Field = Field::params("bits 5 to 7", 5, 3, Kind::Unused, "set in the data's 0xFFFF; read by nothing");
/// `PARAMS_GET_U(params, 8, 6)`: the drop's collectible flag.
pub const COLLECTIBLE_FLAG: Field = Field::params("collectibleFlag", 8, 6, Kind::CollectibleFlag, "the drop's collectible flag");
/// Bits 14 and 15: set in the data (0xFFFF), read by nothing.
pub const UNUSED_HIGH: Field = Field::params("bits 14 and 15", 14, 2, Kind::Unused, "set in the data's 0xFFFF; read by nothing");

/// Obj_Kibako's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[COLLECTIBLE, UNUSED_LOW, COLLECTIBLE_FLAG, UNUSED_HIGH],
    flags: &[FlagUse::new(Flag::Collectible("collectibleFlag")).reads().sets().indirect("by the En_Item00 it drops")],
    todo: false,
    objects: &[],
    texts: &[],
};

/// `sCylinderInit`: hurting 1 with `DMG_DEKU_STICK` (thrown), hit by `0x4FC00748`, pushing as
/// type 2; 12 by 27.
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_ON | AT_TYPE_PLAYER, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0002, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x01 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x4FC0_0748, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 12, height: 27, y_shift: 0, pos: [0; 3] },
};

/// `sCCInfoInit`: health 0, 12 by 60, `MASS_HEAVY`.
pub const COL_CHK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 0, cyl_radius: 12, cyl_height: 60, mass: MASS_HEAVY };

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Idle,
    Held,
    Thrown,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::Idle => "ObjKibako_Idle",
            Action::Held => "ObjKibako_Held",
            Action::Thrown => "ObjKibako_Thrown",
        }
    }
}

pub struct ObjKibako {
    pub actor: Actor,
    pub action: Action,
    pub collider: ColliderCylinder,
}

impl ObjKibako {
    /// `ObjKibako_Init`: scale 0.1, the culling volume (`sInitChain`), gravity -1.2 to -13, the
    /// cylinder, standing.
    pub fn init(mut actor: Actor, _play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain.
        actor.scale = Vec3::splat(0.1);
        actor.culling_volume_distance = 1000.0;
        actor.culling_volume_scale = 60.0;
        actor.culling_volume_downward = 1000.0;
        actor.gravity = -1.2;
        actor.min_velocity_y = -13.0;
        // ObjKibako_InitCollider.
        let mut collider = ColliderCylinder::new(&CYLINDER_INIT);
        collider.update(&actor);
        actor.col_chk_info.set_info(None, &COL_CHK_INFO_INIT);
        let mut this = ObjKibako { actor, action: Action::Idle, collider };
        this.setup_idle();
        log::debug!("(dungeon keep wooden box)(arg_data 0x{:04x})", this.actor.params); // schema: printed
        Box::new(this)
    }

    /// `ObjKibako_SpawnCollectible`: the drop in bits 0 to 4 with the flag in bits 8 to 13.
    pub fn spawn_collectible(&self, play: &mut PlayState) {
        let collectible = COLLECTIBLE.get(self.actor.params);
        if (0..ITEM00_MAX).contains(&collectible) {
            crate::en_item00::item_drop_collectible(play, self.actor.world_pos, collectible | (COLLECTIBLE_FLAG.get(self.actor.params) << 8));
        }
    }

    /// `ObjKibako_ApplyGravity`.
    fn apply_gravity(&mut self) {
        self.actor.velocity.y += self.actor.gravity;
        if self.actor.velocity.y < self.actor.min_velocity_y {
            self.actor.velocity.y = self.actor.min_velocity_y;
        }
    }

    /// `ObjKibako_AirBreak`: twelve splinters round it, then dust.
    pub fn air_break(&self, play: &mut PlayState) {
        let wp = self.actor.world_pos;
        play.with_ss(|ss| {
            let mut angle: i16 = 0;
            for _ in 0..12 {
                let sn = sin_s(angle);
                let cs = cos_s(angle);
                let mut pos = Vec3::new(sn * 16.0, (ss.rand.zero_one() * 5.0) + 2.0, cs * 16.0);
                let velocity = Vec3::new(pos.x * 0.2, (ss.rand.zero_one() * 6.0) + 2.0, pos.z * 0.2);
                pos += wp;
                let temp_rand = ss.rand.zero_one();
                let phi_s0 = if temp_rand < 0.1 {
                    0x60
                } else if temp_rand < 0.7 {
                    0x40
                } else {
                    0x20
                };
                let scale = ((ss.rand.zero_one() * 30.0) + 10.0) as i16;
                ss.kakera_spawn(pos, velocity, wp, -200, phi_s0, 10, 10, 0, scale, 0, 32, 60, KAKERA_COLOR_NONE, OBJECT_GAMEPLAY_DANGEON_KEEP, Some((OBJECT, FRAGMENT_DL)));
                angle = angle.wrapping_add(0x4E20);
            }
        });
        func_80033480(play, wp, 40.0, 3, 50, 140, 1);
    }

    /// `ObjKibako_WaterBreak`: the splash at the surface (500), twelve splinters sinking.
    pub fn water_break(&self, play: &mut PlayState) {
        let wp = self.actor.world_pos;
        let mut splash = wp;
        splash.y += self.actor.y_dist_to_water;
        play.with_ss(|s| s.g_splash_spawn(splash, None, None, 0, 500));
        play.with_ss(|ss| {
            let mut angle: i16 = 0;
            for _ in 0..12 {
                let sn = sin_s(angle);
                let cs = cos_s(angle);
                let mut pos = Vec3::new(sn * 16.0, (ss.rand.zero_one() * 5.0) + 2.0, cs * 16.0);
                let velocity = Vec3::new(pos.x * 0.18, (ss.rand.zero_one() * 4.0) + 2.0, pos.z * 0.18);
                pos += wp;
                let phi_s0 = if ss.rand.zero_one() < 0.2 { 0x40 } else { 0x20 };
                let scale = ((ss.rand.zero_one() * 30.0) + 10.0) as i16;
                ss.kakera_spawn(pos, velocity, wp, -180, phi_s0, 30, 30, 0, scale, 0, 32, 70, KAKERA_COLOR_NONE, OBJECT_GAMEPLAY_DANGEON_KEEP, Some((OBJECT, FRAGMENT_DL)));
                angle = angle.wrapping_add(0x4E20);
            }
        });
    }

    /// The break's end: `NA_SE_EV_WOODBOX_BREAK` where it was (20 frames), the drop, gone.
    fn broken(&mut self, play: &mut PlayState) {
        play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, 20, NA_SE_EV_WOODBOX_BREAK);
        self.spawn_collectible(play);
        self.actor.kill();
    }

    /// `ObjKibako_SetupIdle`.
    pub fn setup_idle(&mut self) {
        self.action = Action::Idle;
        self.actor.col_chk_info.mass = MASS_HEAVY;
    }

    /// `ObjKibako_Idle`: see the module's docs.
    fn idle(&mut self, play: &mut PlayState) {
        if oot_game::get_item::actor_has_parent(&self.actor) {
            self.setup_held(play);
        } else if self.actor.bg_check_flags & BGCHECKFLAG_WATER != 0 && self.actor.y_dist_to_water > 19.0 {
            self.water_break(play);
            self.broken(play);
        } else if self.collider.base.ac_flags & AC_HIT != 0 {
            self.air_break(play);
            self.broken(play);
        } else {
            self.actor.move_forward();
            self.actor.update_bg_check_info(&play.col, 19.0, 20.0, 0.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
            if self.collider.base.oc_flags1 & OC1_TYPE_PLAYER == 0 && self.actor.xz_dist_to_player > 28.0 {
                self.collider.base.oc_flags1 |= OC1_TYPE_PLAYER;
            }
            if self.actor.xz_dist_to_player < 600.0 {
                self.collider.update(&self.actor);
                play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
                if self.actor.xz_dist_to_player < 180.0 {
                    play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
                }
            }
            if self.actor.xz_dist_to_player < 100.0 {
                // Actor_OfferCarry.
                let a = self.actor.clone();
                oot_game::get_item::offer_pick_up(play, &a);
            }
        }
    }

    /// `ObjKibako_SetupHeld`: in no room. `@bug (game)`: `Player_PlaySfx` on the crate cast to
    /// Player (its own `projectedPos`: the sound still plays).
    fn setup_held(&mut self, play: &mut PlayState) {
        self.action = Action::Held;
        self.actor.room = -1;
        if let Some(me) = play.cur_actor {
            player_play_sfx(play, me, NA_SE_PL_PULL_UP_WOODBOX);
        }
    }

    /// `ObjKibako_Held`: let go, put down (standing) or thrown.
    fn held(&mut self, play: &mut PlayState) {
        if self.actor.parent.is_none() {
            self.actor.room = play.room_ctx.cur.num;
            if self.actor.speed_xz.abs() < 0.1 {
                audio_play_actor_sfx2(play, NA_SE_EV_PUT_DOWN_WOODBOX);
                self.setup_idle();
                self.collider.base.oc_flags1 &= !OC1_TYPE_PLAYER;
            } else {
                self.setup_thrown();
                self.apply_gravity();
                self.actor.update_pos();
            }
            self.actor.update_bg_check_info(&play.col, 19.0, 20.0, 0.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
        }
    }

    /// `ObjKibako_SetupThrown`: its velocity along its yaw at its speed, mass 240.
    pub fn setup_thrown(&mut self) {
        self.actor.velocity.x = sin_s(self.actor.world_rot.y) * self.actor.speed_xz;
        self.actor.velocity.z = cos_s(self.actor.world_rot.y) * self.actor.speed_xz;
        self.actor.col_chk_info.mass = 240;
        self.action = Action::Thrown;
    }

    /// `ObjKibako_Thrown`: see the module's docs.
    fn thrown(&mut self, play: &mut PlayState) {
        if self.actor.bg_check_flags & (BGCHECKFLAG_GROUND | BGCHECKFLAG_GROUND_TOUCH | BGCHECKFLAG_WALL) != 0 || self.collider.base.at_flags & AT_HIT != 0 {
            self.air_break(play);
            self.broken(play);
        } else if self.actor.bg_check_flags & BGCHECKFLAG_WATER_TOUCH != 0 {
            self.water_break(play);
            self.broken(play);
        } else {
            self.apply_gravity();
            self.actor.update_pos();
            self.actor.update_bg_check_info(&play.col, 19.0, 20.0, 0.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
            self.collider.update(&self.actor);
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
            play.collision_check_set_at(&self.actor, 0, &mut self.collider);
        }
    }
}

impl ActorImpl for ObjKibako {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `ObjKibako_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Idle => self.idle(play),
            Action::Held => self.held(play),
            Action::Thrown => self.thrown(play),
        }
    }
    /// `ObjKibako_Destroy`: the cylinder (nothing to free).
    fn destroy(&mut self, _play: &mut PlayState) {}
    fn render_state(&self) -> RenderState {
        RenderState::of(&self.actor)
    }
    /// `ObjKibako_Draw`: `Gfx_DrawDListOpa(gSmallWoodenBoxDL)`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        crate::gfx_draw_dlist_opa(out, OBJECT, DL, rs);
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
