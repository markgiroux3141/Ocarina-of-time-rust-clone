//! `Bg_Bdan_Objects` (`ovl_Bg_Bdan_Objects/z_bg_bdan_objects.c`): Lord Jabu-Jabu's moving parts
//! (GAME-10 milestone 4), of `object_bdan_objects`, by type (`params & 0xFF`; the switch flag in
//! bits 8 to 13, kept in `var` before the params are cut to the type):
//!
//! - **Big Octo's platform** (type 0, `gJabuObjectsLargeRotatingSpikePlatformDL`,
//!   `gJabuBigOctoPlatformCol`; room 6): 79 below its home, the switch flag set, it waits for Ruto
//!   (`En_Ru1`) to set its camera setting to `CAM_SET_NORMAL0`; then, Link within 250 across, it's
//!   `CAM_SET_NORMAL1`, one-point 3070 starts on it, Link is put at (-1130, -1025, -3300), and 20
//!   frames on it rises (`Math_SmoothStepToF` to 500 above home, `NA_SE_EV_BUYOSTAND_RISING`) to
//!   `NA_SE_EV_BUYOSTAND_STOP_A` and `CAM_SET_DUNGEON0`; 30 frames on a quake (type 1, speed 15000,
//!   1 across and 250 of the view's angle, 10 frames); when Ruto sets `CAM_SET_DUNGEON1` it spawns
//!   **Big Octo** (`En_Bigokuta` params 0, 140 above it, facing back) as its child, sets
//!   `INFTABLE_RUTO_ABDUCTED`, ends the camera, clears the flag 10 frames on and 40 after descends
//!   (0.5 a frame faster each frame, `NA_SE_EV_BUYOSTAND_FALL`, Big Octo 140 above it) to 70 below
//!   home: `NA_SE_EV_BUYOSTAND_STOP_U`, one-point 3080 on Big Octo, Link put at (-1130, -1025,
//!   -3500) facing 0x7530. Big Octo's params 0 it sets to 1; at 2 the battle is on: its spikes hurt
//!   (a cylinder 187 by 80, 4, `AT_TYPE_ENEMY`) and it turns by `world.rot.y` (Big Octo's), rolling
//!   (`NA_SE_EV_ROLL_STAND`, faster with the turn). The room cleared, the flag is set, it turns to
//!   the nearest quarter and sinks 3 a frame to 125 below home; Link on it within 120 across,
//!   one-point 3090 and it rises (to 965 above home, `NA_SE_EV_BUYOSTAND_STOP_A`) and stays. With
//!   the room cleared it starts sunk; with `INFTABLE_RUTO_ABDUCTED` (Big Octo met), Big Octo (params
//!   3) is spawned at once, 263 along z, and the battle's on. Big Octo is `En_Bigokuta` (GAME-10
//!   milestone 8), who writes the platform's turn (`world.rot.y`) through his parent.
//! - **Room 1's elevator** (type 1, `gJabuElevatorPlatformDL`, `gJabuElevatorCol`): rises and sinks
//!   by `sin`, 471.24 (150π) round its home over 512 frames, for good; Link on it turns the main
//!   camera to `CAM_SET_NORMAL2` without the bg's settings (`CAM_STATE_CHECK_BG` off), the setting
//!   it had kept and put back 10 frames after he steps off.
//! - **Room 14's water** (type 2, `gJabuWaterDL`, no collision): it holds the scene's
//!   `waterBoxes[7]` at its height; on its switch flag it rises 0.5 a frame to 75 above home
//!   (`NA_SE_EV_WATER_LEVEL_DOWN`), stays 100 frames (the timer's ticks), sinks back and clears the
//!   flag.
//! - **Room 1's falling platform** (type 3, `gJabuFallingPlatformDL`, `gJabuLoweringPlatformCol`):
//!   Link on it sets the flag, one-point 3100 (the camera below it looking up, Link held) and it
//!   falls 400 over 50 frames (by `cos`, `NA_SE_EV_BUYOSTAND_FALL`, `NA_SE_EV_BUYOSTAND_STOP_U`),
//!   the main camera copied from the active one; with the flag set it starts down.
//!
//! The whole overlay is ported but the rumble (`Rumble_Request`). `BgBdanObjects_GetProperty` and
//! `_SetProperty` are `En_Ru1`'s (milestone 5).
//!
//! Jabu-Jabu (Master Quest) places four: 0x1F00 (room 6, the platform, flag 0x1F), 0xFF01 (room 1,
//! the elevator), 0x1D03 (room 1, the falling platform, flag 0x1D), 0x3902 (room 14, the water, on
//! the yellow switch 0x39).
//!
//! Params: `type` bits 0 to 7 (`BgBdanObjectsType`: 0 Big Octo's platform, 1 the elevator, 2 the
//! water, 3 the falling platform); `switchFlag` bits 8 to 13.
//! Flags: the platform sets and clears switch flag `switchFlag`, reads the room's clear flag, and
//! reads and sets `INFTABLE_RUTO_ABDUCTED` (for Ruto); the water reads and clears it; the falling
//! platform reads and sets it.

use std::sync::Arc;

use eng_collision::collision::CollisionHeader;
use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource, DYNA_INTERACT_PLAYER_ON_TOP, DYNA_TRANSFORM_POS};
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{scaled_step_to_s, smooth_step_to_f, step_to_f};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::{SFX_FLAG, SfxPos};
use oot_game::camera::{CAM_ID_MAIN, CAM_SET_DUNGEON0, CAM_SET_NORMAL0};
use oot_game::collision_check::*;
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};

/// `ACTOR_BG_BDAN_OBJECTS` (`actor_table.h`: 0x00C8).
pub const ACTOR_BG_BDAN_OBJECTS: i16 = 0x00C8;
/// `ACTOR_EN_BIGOKUTA` (`actor_table.h`: 0x00C6).
pub const ACTOR_EN_BIGOKUTA: i16 = 0x00C6;
/// `OBJECT_BDAN_OBJECTS` (`object_table.h`: 0x0096).
pub const OBJECT: &str = "object_bdan_objects";

/// `sDLists`, by type (`object_bdan_objects.xml`).
pub const DLISTS: [&str; 4] = ["gJabuObjectsLargeRotatingSpikePlatformDL", "gJabuElevatorPlatformDL", "gJabuWaterDL", "gJabuFallingPlatformDL"];
/// The collision of the platform, the elevator and the falling platform.
pub const BIG_OCTO_PLATFORM_COL: &str = "gJabuBigOctoPlatformCol";
pub const ELEVATOR_COL: &str = "gJabuElevatorCol";
pub const LOWERING_PLATFORM_COL: &str = "gJabuLoweringPlatformCol";

/// `BgBdanObjectsType` (`z_bg_bdan_objects.h`).
pub const JABU_OBJECTS_TYPE_BIG_OCTO_PLATFORM: i16 = 0;
pub const JABU_OBJECTS_TYPE_SMALL_AUTO_ELEVATOR: i16 = 1;
pub const JABU_OBJECTS_TYPE_WATERBOX_HEIGHT_CHANGER: i16 = 2;
pub const JABU_OBJECTS_TYPE_FALLING_PLATFORM: i16 = 3;

/// `BgBdanObjectsPropertyGetter` and `BgBdanObjectsPropertySetter` (`En_Ru1`'s).
pub const JABU_OBJECTS_GET_PROP_CAM_SETTING_NORMAL0: i32 = 0;
pub const JABU_OBJECTS_GET_PROP_CAM_SETTING_DUNGEON1: i32 = 3;
pub const JABU_OBJECTS_GET_PROP_WATCHED_BIGOCTO_INTRO_CUTSCENE: i32 = 4;
pub const JABU_OBJECTS_SET_PROP_CAM_SETTING_NORMAL1: i32 = 1;
pub const JABU_OBJECTS_SET_PROP_CAM_SETTING_DUNGEON0: i32 = 2;
pub const JABU_OBJECTS_SET_PROP_WATCHED_BIGOCTO_INTRO_CUTSCENE: i32 = 4;

/// `CAM_SET_NORMAL1`, `CAM_SET_DUNGEON1`, `CAM_SET_PIVOT_VERTICAL`, `CAM_SET_NORMAL2`
/// (`camera.h`).
pub const CAM_SET_NORMAL1: i16 = 0x02;
pub const CAM_SET_DUNGEON1: i16 = 0x04;
pub use oot_game::camera::CAM_SET_PIVOT_VERTICAL;
pub const CAM_SET_NORMAL2: i16 = 0x3A;
/// `CAM_STATE_CHECK_BG` (`camera.h`: 1 << 2).
const CAM_STATE_CHECK_BG: i16 = 1 << 2;
/// `INFTABLE_RUTO_ABDUCTED` (`save.h`: 0x146).
pub const INFTABLE_RUTO_ABDUCTED: u16 = 0x146;
/// The scene's water box the water holds: `play->colCtx.colHeader->waterBoxes[7]`.
const WATER_BOX: usize = 7;

// The sounds (`include/tables/sfx/environmentbank_table.h`).
/// `NA_SE_EV_WATER_LEVEL_DOWN` (0x285E).
pub const NA_SE_EV_WATER_LEVEL_DOWN: u16 = 0x285E;
/// `NA_SE_EV_ROLL_STAND` (0x2863).
pub const NA_SE_EV_ROLL_STAND: u16 = 0x2863;
/// `NA_SE_EV_BUYOSTAND_RISING` (0x288F).
pub const NA_SE_EV_BUYOSTAND_RISING: u16 = 0x288F;
/// `NA_SE_EV_BUYOSTAND_FALL` (0x2890).
pub const NA_SE_EV_BUYOSTAND_FALL: u16 = 0x2890;
/// `NA_SE_EV_BUYOSTAND_STOP_A` (0x289E).
pub const NA_SE_EV_BUYOSTAND_STOP_A: u16 = 0x289E;
/// `NA_SE_EV_BUYOSTAND_STOP_U` (0x289F).
pub const NA_SE_EV_BUYOSTAND_STOP_U: u16 = 0x289F;

/// `Bg_Bdan_Objects_Profile`: `ACTORCAT_BG`, `ACTOR_FLAG_UPDATE_CULLING_DISABLED`,
/// `OBJECT_BDAN_OBJECTS`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_BDAN_OBJECTS, name: "Bg_Bdan_Objects", category: ACTORCAT_BG, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `params & 0xFF` (cut to it at init): `BgBdanObjectsType`.
pub const TYPE: Field = Field::params(
    "type",
    0,
    8,
    Kind::Named(&[(0, "Big Octo's platform"), (1, "the elevator (up and down for good)"), (2, "the water (waterBoxes[7], up 75 on the flag)"), (3, "the falling platform")]),
    "which of Jabu-Jabu's moving parts",
);
/// `PARAMS_GET_U(params, 8, 6)`: `var.switchFlag`.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 8, 6, Kind::SwitchFlag, "the platform's (set waiting, cleared as Big Octo comes), the water's (raises it), the falling platform's (set as it falls)");
/// Bits 14 and 15: set in the elevator's 0xFF01, read by nothing.
pub const UNUSED_14: Field = Field::params("params bits 14 and 15", 14, 2, Kind::Unused, "set in the elevator's 0xFF01; read by nothing");

/// Bg_Bdan_Objects' configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, SWITCH_FLAG, UNUSED_14],
    flags: &[
        FlagUse::new(Flag::Switch("switchFlag")).reads().sets().clears().note("the platform sets it and clears it as Big Octo comes; the water reads and clears it; the falling platform reads and sets it"),
        FlagUse::new(Flag::Clear).reads().note("the platform: sunk, Big Octo beaten"),
        FlagUse::new(Flag::InfTable("INFTABLE_RUTO_ABDUCTED", INFTABLE_RUTO_ABDUCTED)).reads().sets().note("the platform: Big Octo met"),
    ],
    todo: false,
    objects: &[],
    texts: &[],
};

/// `sCylinderInit`: the platform's spikes, hurting 4 (`AT_TYPE_ENEMY`), 187 by 80.
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_NONE, oc_flags1: OC1_NONE, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x04 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0000, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_HARD,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_NONE,
    },
    dim: Cylinder16 { radius: 187, height: 80, y_shift: 0, pos: [0; 3] },
};

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    OctoPlatformWaitForRutoToStartCutscene,
    OctoPlatformRaiseToUpperPosition,
    OctoPlatformWaitForRutoToAdvanceCutscene,
    OctoPlatformPauseBeforeDescending,
    OctoPlatformDescendWithBigOcto,
    OctoPlatformWaitForBigOctoToStartBattle,
    OctoPlatformBattleInProgress,
    SinkToFloorHeight,
    WaitForPlayerInRange,
    RaiseToUpperPosition,
    DoNothing,
    ElevatorOscillate,
    WaitForSwitch,
    ChangeWaterBoxLevel,
    WaitForTimerExpired,
    WaitForPlayerOnTop,
    FallToLowerPos,
}

pub struct BgBdanObjects {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub action: Action,
    /// `var`: `switchFlag` (types 0, 2, 3) and `camChangeTimer` (the elevator) share it.
    pub var: u8,
    pub timer: i16,
    /// `collider` (the platform's).
    pub collider: ColliderCylinder,
    /// `cameraSetting`: the setting the elevator put back; the platform's state as Ruto sets it.
    pub camera_setting: i16,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

fn load_collision(play: &PlayState, symbol: &str) -> anyhow::Result<Arc<CollisionHeader>> {
    let assets = play.assets.as_ref().ok_or_else(|| anyhow::anyhow!("no asset pack"))?;
    Ok(Arc::new(assets.pack.collision(&keys::collision(OBJECT, symbol))?))
}

impl BgBdanObjects {
    fn ty(&self) -> i16 {
        TYPE.get(self.actor.params)
    }

    fn switch_flag(&self) -> i32 {
        self.var as i32
    }

    /// `DynaPolyActor_IsPlayerOnTop`.
    fn player_on_top(&self, play: &PlayState) -> bool {
        self.bg != BG_ACTOR_MAX && play.col.dyna.interact_flag(self.bg, DYNA_INTERACT_PLAYER_ON_TOP)
    }

    /// The actor as the one-point cutscenes read it (it's out of the arena in its update).
    fn cam_actor(&self, play: &PlayState) -> Option<oot_game::camera::CamActor> {
        play.cur_actor.map(|h| play.cam_actor_of(h, &self.actor))
    }

    /// `BgBdanObjects_GetProperty` (`En_Ru1`'s): the camera setting Ruto set, or Big Octo met.
    pub fn get_property(&self, play: &PlayState, arg1: i32) -> i32 {
        match arg1 {
            JABU_OBJECTS_GET_PROP_CAM_SETTING_NORMAL0 => (self.camera_setting == CAM_SET_NORMAL0) as i32,
            JABU_OBJECTS_GET_PROP_WATCHED_BIGOCTO_INTRO_CUTSCENE => play.save.get_inf_table(INFTABLE_RUTO_ABDUCTED) as i32,
            JABU_OBJECTS_GET_PROP_CAM_SETTING_DUNGEON1 => (self.camera_setting == CAM_SET_DUNGEON1) as i32,
            _ => {
                log::warn!("Bg_Bdan_Objects_Get_Contact_Ru1\nThere is no such receiving mode {arg1}!!!!!!!!");
                -1
            }
        }
    }

    /// `BgBdanObjects_SetProperty` (`En_Ru1`'s).
    pub fn set_property(&mut self, play: &mut PlayState, arg1: i32) {
        match arg1 {
            JABU_OBJECTS_SET_PROP_CAM_SETTING_NORMAL1 => self.camera_setting = CAM_SET_NORMAL1,
            JABU_OBJECTS_SET_PROP_CAM_SETTING_DUNGEON0 => self.camera_setting = CAM_SET_DUNGEON0,
            JABU_OBJECTS_SET_PROP_WATCHED_BIGOCTO_INTRO_CUTSCENE => play.save.set_inf_table(INFTABLE_RUTO_ABDUCTED),
            _ => log::warn!("Bg_Bdan_Objects_Set_Contact_Ru1\nThere is no such transmission mode {arg1}!!!!!!!!"),
        }
    }

    /// `BgBdanObjects_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = BgBdanObjects { actor, bg: BG_ACTOR_MAX, action: Action::DoNothing, var: 0, timer: 0, collider: ColliderCylinder::default(), camera_setting: 0 };
        // sInitChain: ICHAIN_VEC3F_DIV1000(scale, 100).
        this.actor.scale = Vec3::splat(0.1);
        // DynaPolyActor_Init(DYNA_TRANSFORM_POS): the bg actor below, as each type has it.
        this.var = SWITCH_FLAG.get(this.actor.params) as u8;
        this.actor.params &= 0xFF;
        let ty = this.ty();
        if ty == JABU_OBJECTS_TYPE_WATERBOX_HEIGHT_CHANGER {
            this.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED;
            this.set_water_box(play);
            this.action = Action::WaitForSwitch;
            return Box::new(this);
        }
        let col = if ty == JABU_OBJECTS_TYPE_BIG_OCTO_PLATFORM {
            this.collider = ColliderCylinder::new(&CYLINDER_INIT);
            this.actor.world_pos.y += -79.0;
            if play.flags.get_clear(this.actor.room) {
                play.flags.set_switch(this.switch_flag());
                this.action = Action::SinkToFloorHeight;
            } else if this.get_property(play, JABU_OBJECTS_GET_PROP_WATCHED_BIGOCTO_INTRO_CUTSCENE) != 0 {
                let (home, rot_y) = (this.actor.home_pos, this.actor.shape_rot.y);
                if let Ok(child) = play.actor_spawn_as_child(&mut this.actor, ACTOR_EN_BIGOKUTA, home, [0, rot_y.wrapping_add(-0x8000), 0], 3)
                    && let Some(c) = play.actors.actor_mut(child)
                {
                    c.world_pos.z = c.home_pos.z + 263.0;
                }
                this.actor.world_rot.y = 0;
                this.action = Action::OctoPlatformBattleInProgress;
                this.actor.world_pos.y = this.actor.home_pos.y + -70.0;
            } else {
                play.flags.set_switch(this.switch_flag());
                this.timer = 0;
                this.action = Action::OctoPlatformWaitForRutoToStartCutscene;
            }
            BIG_OCTO_PLATFORM_COL
        } else if ty == JABU_OBJECTS_TYPE_SMALL_AUTO_ELEVATOR {
            this.timer = 512;
            this.var = 0;
            this.action = Action::ElevatorOscillate;
            ELEVATOR_COL
        } else {
            // JABU_OBJECTS_TYPE_FALLING_PLATFORM (and any other type).
            if play.flags.get_switch(this.switch_flag()) {
                this.action = Action::DoNothing;
                this.actor.world_pos.y = this.actor.home_pos.y - 400.0;
            } else {
                this.action = Action::WaitForPlayerOnTop;
            }
            LOWERING_PLATFORM_COL
        };
        this.bg = match load_collision(play, col) {
            Ok(h) => play.col.dyna.set_bg_actor(h, source(&this.actor), DYNA_TRANSFORM_POS),
            Err(e) => {
                log::error!("Bg_Bdan_Objects: {e:#}");
                BG_ACTOR_MAX
            }
        };
        Box::new(this)
    }

    /// `play->colCtx.colHeader->waterBoxes[7].ySurface = world.pos.y` (an s16).
    fn set_water_box(&self, play: &mut PlayState) {
        if !play.col.set_water_box_surface(WATER_BOX, self.actor.world_pos.y as i32 as i16) {
            log::error!("Bg_Bdan_Objects: the scene has no waterBoxes[{WATER_BOX}]");
        }
    }

    /// Link's base actor (`GET_PLAYER(play)->actor`), for the platform's placing of him.
    fn place_player(play: &mut PlayState, pos: Vec3, rot_y: Option<i16>) {
        if let Some(a) = play.player.and_then(|h| play.actors.actor_mut(h)) {
            a.world_pos = pos;
            if let Some(y) = rot_y {
                a.shape_rot.y = y;
                a.world_rot.y = y;
            }
        }
    }

    /// `BgBdanObjects_OctoPlatform_WaitForRutoToStartCutscene`: Ruto set `CAM_SET_NORMAL0`
    /// (`En_Ru1`) and Link within 250 across: `CAM_SET_NORMAL1`, one-point 3070, Link put in place,
    /// rising 20 frames on. 79 below home, 5 lower while a cutscene plays or Ruto's set it.
    fn octo_platform_wait_for_ruto_to_start_cutscene(&mut self, play: &mut PlayState) {
        if self.get_property(play, JABU_OBJECTS_GET_PROP_CAM_SETTING_NORMAL0) != 0 {
            if self.actor.xz_dist_to_player < 250.0 {
                self.set_property(play, JABU_OBJECTS_SET_PROP_CAM_SETTING_NORMAL1);
                self.timer = 20;
                let me = self.cam_actor(play);
                play.onepoint_cutscene_init(3070, -99, me, CAM_ID_MAIN);
                Self::place_player(play, Vec3::new(-1130.0, -1025.0, -3300.0), None);
                // not ported: Rumble_Request(0.0f, 255, 20, 150).
            }
        } else if self.timer != 0 {
            self.timer -= 1;
            if self.timer == 0 {
                self.action = Action::OctoPlatformRaiseToUpperPosition;
            }
        }
        if !play.play_in_cs_mode() && self.get_property(play, JABU_OBJECTS_GET_PROP_CAM_SETTING_NORMAL0) == 0 {
            self.actor.world_pos.y = self.actor.home_pos.y + -79.0;
        } else {
            self.actor.world_pos.y = (self.actor.home_pos.y + -79.0) - 5.0;
        }
    }

    /// `BgBdanObjects_OctoPlatform_RaiseToUpperPosition`: to 500 above home; there,
    /// `CAM_SET_DUNGEON0` and Ruto's turn.
    fn octo_platform_raise_to_upper_position(&mut self, play: &mut PlayState) {
        if smooth_step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y + 500.0, 0.5, 7.5, 1.0) < 0.1 {
            audio_play_actor_sfx2(play, NA_SE_EV_BUYOSTAND_STOP_A);
            self.action = Action::OctoPlatformWaitForRutoToAdvanceCutscene;
            self.timer = 30;
            self.set_property(play, JABU_OBJECTS_SET_PROP_CAM_SETTING_DUNGEON0);
            // not ported: Rumble_Request(0.0f, 255, 20, 150).
        } else {
            if self.timer != 0 {
                self.timer -= 1;
            }
            if self.timer == 0 {
                // not ported: Rumble_Request(0.0f, 120, 20, 10).
                self.timer = 11;
            }
            self.actor.play_sfx_flagged(NA_SE_EV_BUYOSTAND_RISING - SFX_FLAG);
        }
    }

    /// `BgBdanObjects_OctoPlatform_WaitForRutoToAdvanceCutscene`: 30 frames on a quake; when Ruto
    /// sets `CAM_SET_DUNGEON1`, Big Octo spawned on it, `INFTABLE_RUTO_ABDUCTED`, the camera done.
    fn octo_platform_wait_for_ruto_to_advance_cutscene(&mut self, play: &mut PlayState) {
        if self.timer != 0 {
            self.timer -= 1;
            if self.timer == 0 {
                let active = play.active_cam_id;
                let i = play.quake_request(active, oot_game::quake::QUAKE_TYPE_1);
                play.quake_set_speed(i, 0x3A98);
                play.quake_set_perturbations(i, 0, 1, 250, 1);
                play.quake_set_duration(i, 10);
            }
        }
        if self.get_property(play, JABU_OBJECTS_GET_PROP_CAM_SETTING_DUNGEON1) != 0 {
            let pos = self.actor.world_pos + Vec3::Y * 140.0;
            let rot_y = self.actor.shape_rot.y.wrapping_add(-0x8000);
            if let Err(e) = play.actor_spawn_as_child(&mut self.actor, ACTOR_EN_BIGOKUTA, pos, [0, rot_y, 0], 0) {
                log::warn!("Bg_Bdan_Objects: En_Bigokuta not spawned: {e:?}");
            }
            self.set_property(play, JABU_OBJECTS_SET_PROP_WATCHED_BIGOCTO_INTRO_CUTSCENE);
            self.timer = 10;
            self.action = Action::OctoPlatformPauseBeforeDescending;
            let active = play.active_cam_id;
            play.camera_set_finished_flag(active);
        }
    }

    /// `BgBdanObjects_OctoPlatform_DescendWithBigOcto`: falling, 0.5 a frame faster each frame,
    /// Big Octo 140 above it, to 70 below home; there one-point 3080 on Big Octo, Link put in place.
    fn octo_platform_descend_with_big_octo(&mut self, play: &mut PlayState) {
        self.actor.velocity.y += 0.5;
        if step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y + -70.0, self.actor.velocity.y) {
            self.actor.world_rot.y = 0;
            self.timer = 60;
            audio_play_actor_sfx2(play, NA_SE_EV_BUYOSTAND_STOP_U);
            let y = self.actor.world_pos.y + 140.0;
            let child = self.actor.child;
            let mut child_cam = None;
            if let Some(c) = child.and_then(|h| play.actors.actor_mut(h)) {
                c.world_pos.y = y;
            }
            if let Some(h) = child
                && let Some(c) = play.actors.actor(h)
            {
                child_cam = Some(play.cam_actor_of(h, c));
            }
            self.action = Action::OctoPlatformWaitForBigOctoToStartBattle;
            play.onepoint_cutscene_init(3080, -99, child_cam, CAM_ID_MAIN);
            Self::place_player(play, Vec3::new(-1130.0, -1025.0, -3500.0), Some(0x7530));
            // not ported: Rumble_Request(0.0f, 255, 30, 150).
        } else {
            self.actor.play_sfx_flagged(NA_SE_EV_BUYOSTAND_FALL - SFX_FLAG);
            if self.timer != 0 {
                self.timer -= 1;
            }
            if self.timer == 0 {
                // not ported: Rumble_Request(0.0f, 120, 20, 10).
                self.timer = 11;
            }
            let y = self.actor.world_pos.y + 140.0;
            if let Some(c) = self.actor.child.and_then(|h| play.actors.actor_mut(h)) {
                c.world_pos.y = y;
            }
        }
    }

    /// `BgBdanObjects_OctoPlatform_PauseBeforeDescending`: the flag cleared at once, falling 40
    /// frames later (the timer through 0 to -40).
    fn octo_platform_pause_before_descending(&mut self, play: &mut PlayState) {
        self.timer -= 1;
        if self.timer == 0 {
            play.flags.unset_switch(self.switch_flag());
        } else if self.timer == -40 {
            self.timer = 0;
            self.action = Action::OctoPlatformDescendWithBigOcto;
        }
    }

    /// `BgBdanObjects_OctoPlatform_WaitForBigOctoToStartBattle`: 60 frames on, Big Octo's params 0
    /// made 1; at 2 the battle's on.
    fn octo_platform_wait_for_big_octo_to_start_battle(&mut self, play: &mut PlayState) {
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer == 0
            && let Some(c) = self.actor.child.and_then(|h| play.actors.actor_mut(h))
        {
            // schema: another actor's params read whole (Big Octo's state).
            let child_params = c.params;
            if child_params == 2 {
                self.action = Action::OctoPlatformBattleInProgress;
            } else if child_params == 0 {
                // schema: another actor's params written whole.
                c.params = 1;
            }
        }
    }

    /// `BgBdanObjects_OctoPlatform_BattleInProgress`: the spikes' AT; the room cleared, the flag set
    /// and sinking to the nearest quarter turn; else turning by `world.rot.y`, rolling.
    fn octo_platform_battle_in_progress(&mut self, play: &mut PlayState) {
        self.collider.update(&self.actor);
        play.collision_check_set_at(&self.actor, 0, &mut self.collider);
        if play.flags.get_clear(self.actor.room) {
            play.flags.set_switch(self.switch_flag());
            self.actor.home_rot.y = (self.actor.shape_rot.y.wrapping_add(0x2000) as u16 & 0xC000) as i16;
            self.action = Action::SinkToFloorHeight;
        } else {
            self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(self.actor.world_rot.y);
            if let Some(h) = play.cur_actor {
                play.audio.func_800f436c(SfxPos::Actor(h), NA_SE_EV_ROLL_STAND - SFX_FLAG, (self.actor.world_rot.y as i32).abs() as f32 / 512.0);
            }
        }
    }

    /// `BgBdanObjects_SinkToFloorHeight`: turning to `home.rot.y` (0x200 a frame, scaled) and down 3
    /// a frame to 125 below home; both done, waiting for Link.
    fn sink_to_floor_height(&mut self) {
        let cond = scaled_step_to_s(&mut self.actor.shape_rot.y, self.actor.home_rot.y, 0x200);
        if step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y + -125.0, 3.0) && cond {
            self.action = Action::WaitForPlayerInRange;
        }
    }

    /// `BgBdanObjects_WaitForPlayerInRange`: Link on it within 120 across: one-point 3090, rising.
    fn wait_for_player_in_range(&mut self, play: &mut PlayState) {
        if self.player_on_top(play) && self.actor.xz_dist_to_player < 120.0 {
            self.action = Action::RaiseToUpperPosition;
            let me = self.cam_actor(play);
            play.onepoint_cutscene_init(3090, -99, me, CAM_ID_MAIN);
        }
    }

    /// `BgBdanObjects_RaiseToUpperPosition`: to 965 above home, `NA_SE_EV_BUYOSTAND_STOP_A`.
    fn raise_to_upper_position(&mut self, play: &mut PlayState) {
        if smooth_step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y + 965.0, 0.5, 15.0, 0.2) < 0.01 {
            audio_play_actor_sfx2(play, NA_SE_EV_BUYOSTAND_STOP_A);
            self.action = Action::DoNothing;
        } else {
            self.actor.play_sfx_flagged(NA_SE_EV_BUYOSTAND_RISING - SFX_FLAG);
        }
    }

    /// `BgBdanObjects_ElevatorOscillate`: the main camera `CAM_SET_NORMAL2` while Link's on it (its
    /// setting kept, put back with the bg's settings 10 frames after he's off); its height
    /// `home - sin(timer × π/256) × 471.24`, the timer 512 down to 0 and round again.
    fn elevator_oscillate(&mut self, play: &mut PlayState) {
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.var == 0 {
            if self.player_on_top(play) {
                self.camera_setting = play.camera(CAM_ID_MAIN).map_or(0, |c| c.setting);
                play.camera_change_setting(CAM_ID_MAIN, CAM_SET_NORMAL2);
                if let Some(c) = play.camera_mut(CAM_ID_MAIN) {
                    c.state_flags &= !CAM_STATE_CHECK_BG;
                }
                self.var = 10;
            }
        } else {
            play.camera_change_setting(CAM_ID_MAIN, CAM_SET_NORMAL2);
            if !self.player_on_top(play) && self.var != 0 {
                self.var -= 1;
            }
            if self.var == 0 {
                play.camera_change_setting(CAM_ID_MAIN, self.camera_setting);
                if let Some(c) = play.camera_mut(CAM_ID_MAIN) {
                    c.state_flags |= CAM_STATE_CHECK_BG;
                }
            }
        }
        // sinf(this->timer * (M_PI / 256.0f)) * 471.24f: pi * 150.
        self.actor.world_pos.y = self.actor.home_pos.y - ((self.timer as f32 * (std::f32::consts::PI / 256.0)).sin() * 471.24);
        if self.timer == 0 {
            self.timer = 512;
        }
    }

    /// `BgBdanObjects_WaitForSwitch`: the flag set: rising, held 100 frames.
    fn wait_for_switch(&mut self, play: &mut PlayState) {
        if play.flags.get_switch(self.switch_flag()) {
            self.timer = 100;
            self.action = Action::ChangeWaterBoxLevel;
        }
    }

    /// `BgBdanObjects_ChangeWaterBoxLevel`: with time left, up 0.5 a frame to 75 above home; with
    /// none, down to home and the flag cleared; `NA_SE_EV_WATER_LEVEL_DOWN` either way; the water box
    /// at its height.
    fn change_water_box_level(&mut self, play: &mut PlayState) {
        if self.timer == 0 {
            if step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y, 0.5) {
                play.flags.unset_switch(self.switch_flag());
                self.action = Action::WaitForSwitch;
            }
        } else if step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y + 75.0, 0.5) {
            self.action = Action::WaitForTimerExpired;
        }
        self.actor.play_sfx_flagged_centered2(NA_SE_EV_WATER_LEVEL_DOWN - SFX_FLAG);
        self.set_water_box(play);
    }

    /// `BgBdanObjects_WaitForTimerExpired`: the timer's ticks down to 0, then sinking.
    fn wait_for_timer_expired(&mut self) {
        if self.timer != 0 {
            self.timer -= 1;
        }
        self.actor.play_sfx_flagged_timer(self.timer as i32);
        if self.timer == 0 {
            self.action = Action::ChangeWaterBoxLevel;
        }
    }

    /// `BgBdanObjects_WaitForPlayerOnTop`: Link on it: the flag set, falling 50 frames to 200 below
    /// its home moved 200 down, one-point 3100 (51 frames).
    fn wait_for_player_on_top(&mut self, play: &mut PlayState) {
        if self.player_on_top(play) {
            play.flags.set_switch(self.switch_flag());
            self.timer = 50;
            self.action = Action::FallToLowerPos;
            self.actor.home_pos.y -= 200.0;
            let me = self.cam_actor(play);
            play.onepoint_cutscene_init(3100, 51, me, CAM_ID_MAIN);
        }
    }

    /// `BgBdanObjects_FallToLowerPos`: `home - cos(timer × π/50) × 200`; at 0
    /// `NA_SE_EV_BUYOSTAND_STOP_U`, done, the main camera copied from the active one
    /// (`Play_CopyCamera(play, CAM_ID_MAIN, CAM_ID_NONE)`).
    fn fall_to_lower_pos(&mut self, play: &mut PlayState) {
        if self.timer != 0 {
            self.timer -= 1;
        }
        self.actor.world_pos.y = self.actor.home_pos.y - ((self.timer as f32 * (std::f32::consts::PI / 50.0)).cos() * 200.0);
        if self.timer == 0 {
            audio_play_actor_sfx2(play, NA_SE_EV_BUYOSTAND_STOP_U);
            self.action = Action::DoNothing;
            let active = play.active_cam_id;
            play.copy_camera(CAM_ID_MAIN, active);
        } else {
            self.actor.play_sfx_flagged(NA_SE_EV_BUYOSTAND_FALL - SFX_FLAG);
        }
    }
}

impl ActorImpl for BgBdanObjects {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgBdanObjects_Update`: the focus 50 up, the action.
    fn update(&mut self, play: &mut PlayState) {
        self.actor.set_focus(50.0);
        match self.action {
            Action::OctoPlatformWaitForRutoToStartCutscene => self.octo_platform_wait_for_ruto_to_start_cutscene(play),
            Action::OctoPlatformRaiseToUpperPosition => self.octo_platform_raise_to_upper_position(play),
            Action::OctoPlatformWaitForRutoToAdvanceCutscene => self.octo_platform_wait_for_ruto_to_advance_cutscene(play),
            Action::OctoPlatformPauseBeforeDescending => self.octo_platform_pause_before_descending(play),
            Action::OctoPlatformDescendWithBigOcto => self.octo_platform_descend_with_big_octo(play),
            Action::OctoPlatformWaitForBigOctoToStartBattle => self.octo_platform_wait_for_big_octo_to_start_battle(play),
            Action::OctoPlatformBattleInProgress => self.octo_platform_battle_in_progress(play),
            Action::SinkToFloorHeight => self.sink_to_floor_height(),
            Action::WaitForPlayerInRange => self.wait_for_player_in_range(play),
            Action::RaiseToUpperPosition => self.raise_to_upper_position(play),
            Action::DoNothing => {}
            Action::ElevatorOscillate => self.elevator_oscillate(play),
            Action::WaitForSwitch => self.wait_for_switch(play),
            Action::ChangeWaterBoxLevel => self.change_water_box_level(play),
            Action::WaitForTimerExpired => self.wait_for_timer_expired(),
            Action::WaitForPlayerOnTop => self.wait_for_player_on_top(play),
            Action::FallToLowerPos => self.fall_to_lower_pos(play),
        }
        if self.bg != BG_ACTOR_MAX {
            play.col.dyna.set_source(self.bg, source(&self.actor));
        }
    }

    /// `BgBdanObjects_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        if self.bg != BG_ACTOR_MAX {
            play.col.dyna.delete_bg_actor(self.bg);
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        // BgBdanObjects_Draw's lowered platform: waiting for Ruto, above its low place.
        let a = &self.actor;
        let lowered = self.ty() == JABU_OBJECTS_TYPE_BIG_OCTO_PLATFORM && self.action == Action::OctoPlatformWaitForRutoToStartCutscene && ((a.home_pos.y + -79.0) - 5.0) < a.world_pos.y;
        rs.switches = vec![lowered as u32];
        rs
    }

    /// `BgBdanObjects_Draw`: the water translucent, the others opaque; the platform waiting for Ruto
    /// above its low place drawn 50 (model units) lower.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let mut m = actor_draw_matrix(rs);
        if rs.switches.first() == Some(&1) {
            m *= Mat4::from_translation(Vec3::new(0.0, -50.0, 0.0));
        }
        let Some(dl) = usize::try_from(self.ty()).ok().and_then(|t| DLISTS.get(t)) else { return };
        let cmd = DrawCmd::new(MeshKey::named(keys::mesh(OBJECT, dl)), m);
        if self.ty() == JABU_OBJECTS_TYPE_WATERBOX_HEIGHT_CHANGER {
            out.xlu.push(cmd);
        } else {
            out.opa.push(cmd);
        }
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0).then_some(ColliderMut::Cylinder(&mut self.collider))
    }

    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
