//! `Bg_Spot18_Basket` (`ovl_Bg_Spot18_Basket/z_bg_spot18_basket.c`): the big urn at the bottom of
//! Goron City, a DynaPoly actor of `object_spot18_obj` (`gGoronCityVaseCol`, carrying what stands
//! on it along and round: `DYNA_TRANSFORM_POS | DYNA_TRANSFORM_ROT_Y`) drawn with
//! `gGoronCityVaseDL`.
//!
//! It waits under its lid (`Bg_Spot18_Futa`, spawned as its child, turned 0x1555) for its switch
//! flag, which the bottom floor's four timed torches set. The flag set: one-point cutscene 4220
//! for 80 frames, and 21 frames on it lets go of the lid (which shrinks away) and starts turning
//! (`BgSpot18Basket_Spinning`): its spin up to 0x1F4 a frame, swinging round its home on a circle
//! growing to 50, with `NA_SE_EV_ELEVATOR_MOVE`. Its collider (`sJntSphInit`) is two spheres on
//! the model: the body (0, 2040, 0) 54 across, which only pushes Link (OC), and the mouth (0, 1400,
//! 0) 13 across, which takes `DMG_EXPLOSIVE` (AC). An explosion hitting the mouth from an actor
//! 120 to 200 above the urn and within 32 of it across (a bomb dropped into it) starts one-point
//! cutscene 4210 for 240 frames, takes its collision away and spins it wild
//! (`BgSpot18Basket_ExplosionCs`: up to 0xBB8 a frame, then down to 0x3E8 after 120 frames; the
//! circle out to 100 and back to 0; a wobble of 0x5DC turning 0xBB8 a frame; dust
//! (`func_800286CC`); `NA_SE_EV_WALL_MOVE_SP` pitched by the spin). After 140 frames where it
//! points decides the prize (thirds of the turn: `BgSpot18Basket_SetupStopping`), and it slows
//! (`BgSpot18Basket_Stopping`) to stop at the prize's angle (`sGoronVaseStoppedRotY`) once the
//! wobble is gone, its collision back. The next frame the prize drops 170 above it
//! (`BgSpot18Basket_GivingPrize`): three bomb drops (`ITEM00_BOMBS_A`), three green rupees, or the
//! piece of heart (its collectible flag, params bits 0 to 5) with a red and a blue rupee (a purple
//! one in its place once the piece is given or its flag set); a frame later the chime
//! (`NA_SE_SY_CORRECT_CHIME`, the piece's third) or `NA_SE_SY_TRE_BOX_APPEAR`; 200 frames on it
//! turns again, waiting for the next bomb.
//!
//! The colliders' spheres are placed in its draw (`BgSpot18Basket_Draw`'s
//! `Collider_UpdateSpheres`): the port's `draw_update`, through the actor's model matrix.
//!
//! The whole overlay is ported. Its circle shadow (15) is the shape's (`ActorShape_Init`'s, drawn
//! by `Actor_Draw` on the floor its update finds).
//!
//! Params: `collectibleFlag` bits 0 to 5 (`PARAMS_GET_U(params, 0, 6)`: the piece of heart's;
//! set, a purple rupee in its place); `switchFlag` bits 8 to 13 (`PARAMS_GET_U(params, 8, 6)`:
//! set, it turns; set at init, it turns from the start with no lid). Goron City's one is 0x1C1F:
//! switch flag 0x1C (the four torches' 0x111C), collectible flag 0x1F.
//! Flags: reads switch flag `switchFlag` (init and while it waits); reads collectible flag
//! `collectibleFlag` (the piece of heart given), which the piece (`En_Item00`) sets when it's
//! taken.

use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource, DPM_ROTATE, DYNA_TRANSFORM_POS};
use eng_collision::math3d::Sphere16;
use eng_math::{cos_s, sin_s, step_to_f, step_to_s};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::{NA_SE_SY_CORRECT_CHIME, NA_SE_SY_TRE_BOX_APPEAR, SFX_FLAG, SfxPos};
use oot_game::camera::CAM_ID_MAIN;
use oot_game::collision_check::*;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};

use crate::en_item00::{ITEM00_BOMBS_A, ITEM00_HEART_PIECE, ITEM00_RUPEE_BLUE, ITEM00_RUPEE_GREEN, ITEM00_RUPEE_PURPLE, ITEM00_RUPEE_RED, item_drop_collectible};

/// `ACTOR_BG_SPOT18_BASKET` (`actor_table.h`: 0x015C).
pub const ACTOR_BG_SPOT18_BASKET: i16 = 0x015C;
pub use crate::bg_spot18_futa::{ACTOR_BG_SPOT18_FUTA, OBJECT, OBJECT_SPOT18_OBJ};
const COLLISION: &str = "gGoronCityVaseCol";
const DL: &str = "gGoronCityVaseDL";

/// `Bg_Spot18_Basket_Profile`: `ACTOR_FLAG_UPDATE_CULLING_DISABLED`.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_BG_SPOT18_BASKET, name: "Bg_Spot18_Basket", category: ACTORCAT_PROP, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `PARAMS_GET_U(params, 0, 6)`: the piece of heart's collectible flag.
pub const COLLECTIBLE_FLAG: Field = Field::params("collectibleFlag", 0, 6, Kind::CollectibleFlag, "the piece of heart's; set, a purple rupee in its place");
/// `PARAMS_GET_U(params, 8, 6)`: the switch flag that starts it turning.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 8, 6, Kind::SwitchFlag, "set: the lid off and the urn turning (set at init: turning, no lid)");

/// Bg_Spot18_Basket's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[COLLECTIBLE_FLAG, SWITCH_FLAG],
    flags: &[
        FlagUse::new(Flag::Switch("switchFlag")).reads().note("set: it turns"),
        FlagUse::new(Flag::Collectible("collectibleFlag")).reads().note("set: the piece of heart given, a purple rupee instead"),
        FlagUse::new(Flag::Collectible("collectibleFlag")).sets().note("the piece of heart taken").indirect("the dropped piece's En_Item00 sets it when Link takes it"),
    ],
    todo: false,
    objects: &[],
    texts: &[],
};

/// `NA_SE_EV_ELEVATOR_MOVE` (`environmentbank_table.h`: 0x2824).
pub const NA_SE_EV_ELEVATOR_MOVE: u16 = 0x2824;
/// `NA_SE_EV_WALL_MOVE_SP` (`playerbank_table.h`: 0x08CB; in the player's bank, as the C has it).
pub const NA_SE_EV_WALL_MOVE_SP: u16 = 0x08CB;

/// `sJntSphElementsInit`: the body (limb 0: (0, 2040, 0), 54; OC only) and the mouth (limb 1:
/// (0, 1400, 0), 13; hit by `0x00000008`, `DMG_EXPLOSIVE`).
const JNT_SPH_ELEMENTS: [ColliderJntSphElementInit; 2] = [
    ColliderJntSphElementInit {
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0000, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
            at_elem_flags: ATELEM_NONE,
            ac_elem_flags: ACELEM_NONE,
            oc_elem_flags: OCELEM_ON,
        },
        limb: 0,
        model_sphere: Sphere16 { center: [0, 2040, 0], radius: 54 },
        scale: 100,
    },
    ColliderJntSphElementInit {
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0008, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
            at_elem_flags: ATELEM_NONE,
            ac_elem_flags: ACELEM_ON,
            oc_elem_flags: OCELEM_NONE,
        },
        limb: 1,
        model_sphere: Sphere16 { center: [0, 1400, 0], radius: 13 },
        scale: 100,
    },
];

/// `sJntSphInit`: hit by Player's side (`AC_TYPE_PLAYER`: his and his explosives' attacks),
/// pushing Link (`OC1_TYPE_PLAYER`).
const JNT_SPH_INIT: ColliderInit =
    ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_PLAYER, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_JNTSPH };

/// `sGoronVaseStoppedRotY`: where it stops, by prize (the C's fourth, 0, is never used).
pub const STOPPED_ROT_Y: [i16; 4] = [0x8000u16 as i16, 0x2AAA, 0xD555u16 as i16, 0x0000];
/// `sGoronVaseCollectibleRotY`: the drops' yaws.
pub const COLLECTIBLE_ROT_Y: [i16; 3] = [-0x0FA0, 0x0320, 0x0FA0];

/// The prizes (`prize`).
pub const PRIZE_BOMBS: i16 = 0;
pub const PRIZE_RUPEES: i16 = 1;
pub const PRIZE_HEART_PIECE: i16 = 2;

/// `BgSpot18Basket_SpawnDustClouds`'s `static s16 spawnPos`.
#[derive(Debug, Default)]
pub struct Statics {
    pub spawn_pos: i16,
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `BgSpot18Basket_Inactive`: waiting for the switch flag.
    Inactive,
    /// `BgSpot18Basket_Activation`: the lid let go after 20 frames.
    Activation,
    /// `BgSpot18Basket_Spinning`.
    Spinning,
    /// `BgSpot18Basket_ExplosionCs`: the wild spin.
    ExplosionCs,
    /// `BgSpot18Basket_Stopping`.
    Stopping,
    /// `BgSpot18Basket_GivingPrize`.
    GivingPrize,
}

pub struct BgSpot18Basket {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub collider_jnt_sph: ColliderJntSph,
    pub action: Action,
    pub circle_radius: f32,
    pub circle_rate: i16,
    pub circle_move_angle: i16,
    pub spin_rate: i16,
    pub pivot_azimuth: i16,
    pub pivot_altitude: i16,
    pub timer: i16,
    pub prize: i16,
    pub is_heart_piece_given: bool,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

/// The pitch `func_800F436C` gets: `(spinRate - 0x1F4) * 0.0006`, clamped to 0 to 1.5.
pub fn freq_scale(spin_rate: i16) -> f32 {
    let freq_scale = (spin_rate as i32 - 0x1F4) as f32 * 0.0006;
    freq_scale.clamp(0.0, 1.5)
}

/// `BgSpot18Basket_SetupStopping`'s choice: the prize by where it points (`shape.rot.y`).
pub fn prize_for(shape_rot_y: i16) -> i16 {
    if shape_rot_y < -0x2E93 || shape_rot_y >= 0x7C19 {
        PRIZE_HEART_PIECE
    } else if shape_rot_y < 0x26C2 {
        PRIZE_RUPEES
    } else {
        PRIZE_BOMBS
    }
}

impl BgSpot18Basket {
    /// `PARAMS_GET_U(params, 8, 6)`.
    fn switch_flag(&self) -> i32 {
        SWITCH_FLAG.get(self.actor.params) as i32
    }

    /// `BgSpot18Basket_Init`: `DynaPolyActor_Init(DYNA_TRANSFORM_POS | DYNA_TRANSFORM_ROT_Y)`, its
    /// spheres (`BgSpot18Basket_InitColliderJntSph`: immovable), `gGoronCityVaseCol`, `sInitChain`
    /// (scale 0.1; culling volume 1000, 500, 1000), the circle shadow (15), 0.01 up. The flag set:
    /// turning at once. Else waiting, with its lid spawned as its child (gone without it).
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = BgSpot18Basket {
            actor,
            bg: BG_ACTOR_MAX,
            collider_jnt_sph: ColliderJntSph::new(&JNT_SPH_INIT, &JNT_SPH_ELEMENTS),
            action: Action::Inactive,
            circle_radius: 0.0,
            circle_rate: 0,
            circle_move_angle: 0,
            spin_rate: 0,
            pivot_azimuth: 0,
            pivot_altitude: 0,
            timer: 0,
            prize: 0,
            is_heart_piece_given: false,
        };
        this.actor.col_chk_info.mass = MASS_IMMOVABLE;
        if let Some(h) = crate::obj_kibako2::load_collision(play, OBJECT, COLLISION) {
            this.bg = play.col.dyna.set_bg_actor(h, source(&this.actor), DYNA_TRANSFORM_POS | DPM_ROTATE);
        }
        // sInitChain: ICHAIN_VEC3F_DIV1000(scale, 100); cullingVolumeDistance 1000,
        // cullingVolumeScale 500, cullingVolumeDownward 1000.
        this.actor.scale = Vec3::splat(0.1);
        this.actor.culling_volume_distance = 1000.0;
        this.actor.culling_volume_scale = 500.0;
        this.actor.culling_volume_downward = 1000.0;
        this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 15.0);
        this.actor.home_pos.y += 0.01;
        this.actor.world_pos.y = this.actor.home_pos.y;

        if play.flags.get_switch(this.switch_flag()) {
            this.setup_spinning();
            play.col.dyna.set_source(this.bg, source(&this.actor));
            return Box::new(this);
        }

        this.action = Action::Inactive;
        let a = &this.actor;
        let (pos, rot) = (a.world_pos, [a.shape_rot.x, a.shape_rot.y.wrapping_add(0x1555), a.shape_rot.z]);
        // Actor_SpawnAsChild(.., ACTOR_BG_SPOT18_FUTA, .., -1).
        let _ = play.actor_spawn_as_child(&mut this.actor, ACTOR_BG_SPOT18_FUTA, pos, rot, -1);
        if this.actor.child.is_none() {
            log::error!("Error : Failed to spawn the change pot cover (../z_bg_spot18_basket.c 351)");
            this.actor.kill();
        }
        play.col.dyna.set_source(this.bg, source(&this.actor));
        Box::new(this)
    }

    /// `BgSpot18Basket_SpawnDustClouds`: twice, unless a random draw is over `spawn_rate`, a puff
    /// of brown dust (`func_800286CC`) 35 to 70 out at the overlay's turning angle (0x7530 on
    /// each time), 10 up, drifting out and rising.
    fn spawn_dust_clouds(&self, play: &mut PlayState, spawn_rate: f32) {
        for _ in 0..2 {
            if spawn_rate < play.rand.zero_one() {
                continue;
            }
            let s = play.overlay_static::<Statics>(ACTOR_BG_SPOT18_BASKET);
            s.spawn_pos = s.spawn_pos.wrapping_add(0x7530);
            let spawn_pos = s.spawn_pos;
            let sin_value = sin_s(spawn_pos);
            let cos_value = cos_s(spawn_pos);
            let random_value = (play.rand.zero_one() * 35.0) + 35.0;
            let position = Vec3::new((random_value * sin_value) + self.actor.world_pos.x, self.actor.world_pos.y + 10.0, (random_value * cos_value) + self.actor.world_pos.z);
            let velocity = Vec3::new(sin_value, 0.0, cos_value);
            let acceleration = Vec3::new(0.0, 0.5, 0.0);
            // The s16 arguments: the floats truncated, left to right.
            let scale = ((play.rand.zero_one() * 16.0) + 80.0) as i16;
            let scale_step = ((play.rand.zero_one() * 30.0) + 80.0) as i16;
            play.with_ss(|ss| ss.func_800286cc(position, velocity, acceleration, scale, scale_step));
        }
    }

    /// `BgSpot18Basket_Inactive`: the flag set, one-point cutscene 4220 for 80 frames on it.
    fn inactive(&mut self, play: &mut PlayState) {
        if play.flags.get_switch(self.switch_flag()) {
            let me = play.cur_actor.map(|h| play.cam_actor_of(h, &self.actor));
            play.onepoint_cutscene_init(4220, 80, me, CAM_ID_MAIN);
            self.setup_activation();
        }
    }

    /// `BgSpot18Basket_SetupActivation`.
    fn setup_activation(&mut self) {
        self.action = Action::Activation;
        self.timer = 0;
    }

    /// `BgSpot18Basket_Activation`: after 20 frames it turns, and lets go of its lid (the lid's
    /// `parent` and its `child` cleared).
    fn activation(&mut self, play: &mut PlayState) {
        if self.timer > 20 {
            self.setup_spinning();
            if let Some(lid) = self.actor.child.and_then(|h| play.actors.actor_mut(h)) {
                lid.parent = None;
            }
            self.actor.child = None;
        }
    }

    /// `BgSpot18Basket_SetupSpinning`.
    pub fn setup_spinning(&mut self) {
        self.action = Action::Spinning;
        self.circle_rate = 0;
        self.spin_rate = 0;
    }

    /// The circle round its home: `world.pos.xz` at `circleMoveAngle`, `circleRadius` out.
    fn move_on_circle(&mut self) {
        self.actor.world_pos.x = (sin_s(self.circle_move_angle) * self.circle_radius) + self.actor.home_pos.x;
        self.actor.world_pos.z = (cos_s(self.circle_move_angle) * self.circle_radius) + self.actor.home_pos.z;
    }

    /// `BgSpot18Basket_Spinning`: the spin up to 0x1F4 by 0x1E, the circle out to 50 by 1.5 at up
    /// to 400 by 15 a frame. An explosion on the mouth last frame from an actor 120 to 200 above
    /// and within 32 across: one-point cutscene 4210 for 240 frames, the wild spin, no collision.
    /// `NA_SE_EV_ELEVATOR_MOVE` every frame.
    fn spinning(&mut self, play: &mut PlayState) {
        step_to_s(&mut self.spin_rate, 0x1F4, 0x1E);
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(self.spin_rate);
        step_to_f(&mut self.circle_radius, 50.0, 1.5);
        step_to_s(&mut self.circle_rate, 400, 15);
        self.circle_move_angle = self.circle_move_angle.wrapping_add(self.circle_rate);
        self.move_on_circle();

        if self.collider_jnt_sph.base.ac_flags & AC_HIT != 0 {
            // colliderJntSph.base.ac (a pointer in the C; an actor gone since reads as none here).
            if let Some(ac) = self.collider_jnt_sph.base.ac.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos) {
                let position_diff = ac.y - self.actor.world_pos.y;
                if position_diff > 120.0 && position_diff < 200.0 {
                    // Math3D_Dist2DSq(ac.z, ac.x, this.z, this.x).
                    let (dz, dx) = (ac.z - self.actor.world_pos.z, ac.x - self.actor.world_pos.x);
                    if dz * dz + dx * dx < 32.0 * 32.0 {
                        let me = play.cur_actor.map(|h| play.cam_actor_of(h, &self.actor));
                        play.onepoint_cutscene_init(4210, 240, me, CAM_ID_MAIN);
                        self.setup_explosion_cs();
                        play.col.dyna.set_collision_disabled(self.bg, true);
                    }
                }
            }
        }
        self.actor.play_sfx_flagged(NA_SE_EV_ELEVATOR_MOVE - SFX_FLAG);
    }

    /// `BgSpot18Basket_SetupExplosionCs`.
    fn setup_explosion_cs(&mut self) {
        self.action = Action::ExplosionCs;
        self.timer = 0;
        self.pivot_altitude = 0;
    }

    /// The wobble: `shape.rot.x` and `.z` from the pivot's azimuth and altitude.
    fn tilt(&mut self) {
        self.actor.shape_rot.x = (cos_s(self.pivot_azimuth) * self.pivot_altitude as f32) as i32 as i16;
        self.actor.shape_rot.z = (-sin_s(self.pivot_azimuth) * self.pivot_altitude as f32) as i32 as i16;
    }

    /// `func_800F436C(&projectedPos, NA_SE_EV_WALL_MOVE_SP - SFX_FLAG, ...)`: the grinding,
    /// pitched by the spin.
    fn grind(&self, play: &mut PlayState) {
        let clamped_freq_scale = freq_scale(self.spin_rate);
        if let Some(h) = play.cur_actor {
            play.audio.func_800f436c(SfxPos::Actor(h), NA_SE_EV_WALL_MOVE_SP - SFX_FLAG, clamped_freq_scale);
        }
    }

    /// `BgSpot18Basket_ExplosionCs`: the spin to 0xBB8 by 0x64 (after 120 frames to 0x3E8 by
    /// 0x32); the circle out to 100 by 2 (after 70 frames back to 0), at up to 1000 by 20; the
    /// wobble turning 0xBB8 a frame, leaning to 0x5DC by 0x1E; after 140 frames,
    /// `BgSpot18Basket_SetupStopping`; dust (all of it for 80 frames, then 0.8); the grinding.
    fn explosion_cs(&mut self, play: &mut PlayState) {
        if self.timer > 120 {
            step_to_s(&mut self.spin_rate, 0x3E8, 0x32);
        } else {
            step_to_s(&mut self.spin_rate, 0xBB8, 0x64);
        }
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(self.spin_rate);
        if self.timer < 70 {
            step_to_f(&mut self.circle_radius, 100.0, 2.0);
        } else {
            step_to_f(&mut self.circle_radius, 0.0, 2.0);
        }
        step_to_s(&mut self.circle_rate, 1000, 20);
        self.circle_move_angle = self.circle_move_angle.wrapping_add(self.circle_rate);
        self.move_on_circle();
        self.pivot_azimuth = self.pivot_azimuth.wrapping_add(0xBB8);
        step_to_s(&mut self.pivot_altitude, 0x5DC, 0x1E);
        self.tilt();
        if self.timer > 140 {
            self.setup_stopping();
        }
        if self.timer < 80 {
            self.spawn_dust_clouds(play, 1.0);
        } else {
            self.spawn_dust_clouds(play, 0.8);
        }
        self.grind(play);
    }

    /// `BgSpot18Basket_SetupStopping`: the prize by where it points now (`prize_for`).
    pub fn setup_stopping(&mut self) {
        let shape_rot_y = self.actor.shape_rot.y;
        self.action = Action::Stopping;
        self.prize = prize_for(shape_rot_y);
        self.timer = 0;
    }

    /// `BgSpot18Basket_Stopping`: the wobble turning on, easing (55 a frame) from 13 frames on;
    /// the spin down to 0x1F4 by 0xA. With the wobble gone, it stops on passing the prize's angle
    /// (`sGoronVaseStoppedRotY`), its collision back, and gives the prize. Dust (0.5 for 30 frames,
    /// then 0.3), the grinding.
    fn stopping(&mut self, play: &mut PlayState) {
        self.pivot_azimuth = self.pivot_azimuth.wrapping_add(0xBB8);
        let temp_altitude = if self.timer >= 13 { step_to_s(&mut self.pivot_altitude, 0, 55) } else { false };
        self.tilt();
        step_to_s(&mut self.spin_rate, 0x1F4, 0xA);
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(self.spin_rate);
        if temp_altitude {
            let stopped_rot_y = STOPPED_ROT_Y[self.prize as usize];
            if self.actor.shape_rot.y.wrapping_sub(stopped_rot_y) >= 0 {
                self.actor.shape_rot.y = stopped_rot_y;
                self.setup_giving_prize();
                play.col.dyna.set_collision_disabled(self.bg, false);
            }
        }
        if self.timer < 30 {
            self.spawn_dust_clouds(play, 0.5);
        } else {
            self.spawn_dust_clouds(play, 0.3);
        }
        self.grind(play);
    }

    /// `BgSpot18Basket_SetupGivingPrize`.
    fn setup_giving_prize(&mut self) {
        self.action = Action::GivingPrize;
        self.timer = 0;
    }

    /// `Item_DropCollectible` 170 above it, thrown up at 11 at `yaw`.
    fn drop(play: &mut PlayState, spawn_pos: Vec3, params: i16, yaw: i16) -> bool {
        let Some(h) = item_drop_collectible(play, spawn_pos, params) else { return false };
        if let Some(c) = play.actors.actor_mut(h) {
            c.velocity.y = 11.0;
            c.world_rot.y = yaw;
        }
        true
    }

    /// `BgSpot18Basket_GivingPrize`: at frame 1 the prize, 170 above it; at 2 the chime for the
    /// piece's third, else `NA_SE_SY_TRE_BOX_APPEAR`; at 200 it turns again.
    fn giving_prize(&mut self, play: &mut PlayState) {
        if self.timer == 1 {
            let mut spawn_pos = self.actor.world_pos;
            spawn_pos.y += 170.0;
            if self.prize == PRIZE_BOMBS {
                for &yaw in &COLLECTIBLE_ROT_Y {
                    Self::drop(play, spawn_pos, ITEM00_BOMBS_A, yaw);
                }
            } else if self.prize == PRIZE_RUPEES {
                for &yaw in &COLLECTIBLE_ROT_Y {
                    Self::drop(play, spawn_pos, ITEM00_RUPEE_GREEN, yaw);
                }
            } else if self.prize == PRIZE_HEART_PIECE {
                let collectible_flag = COLLECTIBLE_FLAG.get(self.actor.params);
                if self.is_heart_piece_given || play.flags.get_collectible(collectible_flag as i32) {
                    Self::drop(play, spawn_pos, ITEM00_RUPEE_PURPLE, COLLECTIBLE_ROT_Y[1]);
                } else if Self::drop(play, spawn_pos, (collectible_flag << 8) | ITEM00_HEART_PIECE, COLLECTIBLE_ROT_Y[1]) {
                    self.is_heart_piece_given = true;
                }
                Self::drop(play, spawn_pos, ITEM00_RUPEE_RED, COLLECTIBLE_ROT_Y[0]);
                Self::drop(play, spawn_pos, ITEM00_RUPEE_BLUE, COLLECTIBLE_ROT_Y[2]);
            }
        } else if self.timer == 2 {
            if self.prize == PRIZE_HEART_PIECE {
                play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
            } else {
                play.audio.play_sfx_centered(NA_SE_SY_TRE_BOX_APPEAR);
            }
        } else if self.timer == 200 {
            self.setup_spinning();
        }
    }
}

impl ActorImpl for BgSpot18Basket {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgSpot18Basket_Update`: the timer up, the action, the floor below
    /// (`BgCheck_EntityRaycastDown4`); once it's off its wait, its spheres push (OC), and once
    /// past the lid's letting go they take hits (AC, last frame's hit cleared first).
    fn update(&mut self, play: &mut PlayState) {
        self.timer = self.timer.wrapping_add(1);
        match self.action {
            Action::Inactive => self.inactive(play),
            Action::Activation => self.activation(play),
            Action::Spinning => self.spinning(play),
            Action::ExplosionCs => self.explosion_cs(play),
            Action::Stopping => self.stopping(play),
            Action::GivingPrize => self.giving_prize(play),
        }
        let (floor, poly) = play.col.entity_raycast_down_actor(self.actor.world_pos, self.bg);
        self.actor.floor_height = floor;
        self.actor.floor_poly = poly;
        if self.action != Action::Inactive {
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider_jnt_sph);
            if self.action != Action::Activation {
                self.collider_jnt_sph.base.ac_flags &= !AC_HIT;
                play.collision_check_set_ac(&self.actor, 0, &mut self.collider_jnt_sph);
            }
        }
        play.col.dyna.set_source(self.bg, source(&self.actor));
    }

    /// `BgSpot18Basket_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }

    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }

    /// `BgSpot18Basket_Draw`'s `Collider_UpdateSpheres(0, ...)` and `(1, ...)`: both spheres at the
    /// actor's model matrix.
    fn draw_update(&mut self, _play: &mut PlayState) {
        let m = actor_draw_matrix(&RenderState::of(&self.actor));
        self.collider_jnt_sph.update_spheres(0, &m);
        self.collider_jnt_sph.update_spheres(1, &m);
    }

    /// `BgSpot18Basket_Draw`: `Gfx_DrawDListOpa(gGoronCityVaseDL)`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        crate::gfx_draw_dlist_opa(out, OBJECT, DL, rs);
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0).then_some(ColliderMut::JntSph(&mut self.collider_jnt_sph))
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
    fn the_prizes_are_thirds_of_the_turn() {
        // BgSpot18Basket_SetupStopping's bounds.
        assert_eq!(prize_for(-0x2E94), PRIZE_HEART_PIECE);
        assert_eq!(prize_for(-0x2E93), PRIZE_RUPEES);
        assert_eq!(prize_for(0x26C1), PRIZE_RUPEES);
        assert_eq!(prize_for(0x26C2), PRIZE_BOMBS);
        assert_eq!(prize_for(0x7C18), PRIZE_BOMBS);
        assert_eq!(prize_for(0x7C19), PRIZE_HEART_PIECE);
        assert_eq!(prize_for(i16::MIN), PRIZE_HEART_PIECE);
    }

    #[test]
    fn the_grinding_pitch_is_clamped() {
        assert_eq!(freq_scale(0x1F4), 0.0);
        assert_eq!(freq_scale(0), 0.0);
        assert!((freq_scale(0x3E8) - 0.3).abs() < 1e-6);
        assert_eq!(freq_scale(0x7FFF), 1.5);
    }
}
