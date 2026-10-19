//! `Bg_Dodoago` (`ovl_Bg_Dodoago/z_bg_dodoago.c`): the giant Dodongo skull in Dodongo's Cavern's
//! lobby, its lower jaw a DynaPoly actor of `object_ddan_objects` (`gDodongoLowerJawCol`,
//! `DynaPolyActor_Init(0)`: it doesn't carry Link) drawn with `gDodongoLowerJawDL`.
//!
//! - **The eyes** (`BgDodoago_WaitExplosives`): an explosive's hit (`Actor_GetCollidedExplosive`,
//!   any `ACTORCAT_EXPLOSIVE`: a bomb's blast or a Bomb Flower's) on its main cylinder (80 across,
//!   30 high, 80 up, 200 in front) lights the eye on its side (`Math_Vec3f_Yaw` to the explosive
//!   at or past its facing: `BGDODOAGO_EYE_RIGHT`, else `_LEFT`): one-point 3065 for 40, the
//!   chime, and the eye's glow (`roomCtx.drawParams[eye]`, the scene's draw config's env alpha,
//!   `SDC_DODONGOS_CAVERN`) up 5 a frame to 255 (`BgDodoago_LightOneEye`). That side again: 3065
//!   for 20, `NA_SE_SY_ERROR`, `sTimer` up 30. The other side with one lit: its switch flag, the
//!   chime, one-point 3380 for 160 and the jaw opens.
//! - **The jaw** (`BgDodoago_OpenJaw`): the unlit eye's glow up 5 a frame to 255 (`sTimer` down a
//!   frame meanwhile), then: eleven puffs of dust under its mouth on the frame `sTimer` is 108
//!   (`func_80033480`), four sparkles from each eye a frame (`EffectSsKiraKira_SpawnSmall`), the
//!   rumble (`Rumble_Request`, logged), and the jaw down to 0x1333 (`Math_SmoothStepToS`, the scale
//!   from 110 shrinking by 3 a frame to 10) with `NA_SE_EV_STONE_STATUE_OPEN`; done,
//!   `NA_SE_EV_STONE_BOUND`. With its flag set at init it's open (0x1333), both eyes at 255.
//! - **Before the entrance wall breaks** (`EVENTCHKINF_B0`) its colliders aren't registered and
//!   the jaw isn't drawn (its DynaPoly collision is there all the same: the C registers it at
//!   init whatever the flag).
//! - **The bomb catcher** (`BgDodoago_Update`): two OC cylinders (50 across, 60 high, 280 up, 215
//!   in front, 90 either side), under the holes of the bridge over the skull. While the skull has
//!   no `parent`, a bomb (`ACTOR_EN_BOM` of params 0: a Bomb Flower's isn't one) touching either
//!   has its fuse set to 50 and its speed to 0, becomes the skull's `parent` (the catcher off) and
//!   `sTimer` 0. With a `parent`, `sTimer` counts up; past 140 the `parent` is let go, unless the
//!   switch flag is set, which turns the catcher off for good (`sDisableBombCatcher`). The first
//!   explosive to light an eye becomes the `parent` too, with `sTimer` 50
//!   (`sFirstExplosiveFlag`, never set back; the statics reset only when the overlay is loaded
//!   again: `Actor_FreeOverlay` as the last skull goes).
//!
//! The whole overlay is ported. Not ported: `Rumble_Request` (BACKLOG #60, logged). @bug (game):
//! the dust under the jaw needs `sTimer` to pass 108 while the jaw opens, which it does only while
//! the skull has a `parent` (a bomb caught or the first explosive, `sTimer` counting up); with
//! the catcher let go first (`sTimer` frozen past 108) the jaw opens with no dust. @bug (game): a
//! blast by an eye already lit is the error every other frame its sphere stays on the main
//! cylinder (the error's `return` skips registering the cylinders that frame), each time 30 more
//! on `sTimer` and 3065 again.
//!
//! Params: `switchFlag` bits 0 to 5 (`PARAMS_GET_U(params, 0, 6)`: set when both eyes are lit;
//! set at init, the jaw's open). Master Quest's lobby's is 0x001A.
//! Flags: reads and sets switch flag `switchFlag`; reads EVENTCHKINF_B0 (the entrance wall broken:
//! its colliders and its jaw's draw).

use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use eng_collision::math3d::Cylinder16;
use eng_math::{smooth_step_to_s, step_to_s, vec3f_yaw};
use glam::Vec3;
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTORCAT_BG, ACTORCAT_EXPLOSIVE, ActorImpl, ActorProfile, actor_get_collided_explosive, audio_play_actor_sfx2, func_80033480};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::{NA_SE_EV_STONE_BOUND, NA_SE_EV_STONE_STATUE_OPEN, NA_SE_SY_CORRECT_CHIME, NA_SE_SY_ERROR, SFX_FLAG};
use oot_game::camera::CAM_ID_MAIN;
use oot_game::collision_check::*;
use oot_game::event_chk_inf;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
pub use oot_game::scene_table::{BGDODOAGO_EYE_LEFT, BGDODOAGO_EYE_RIGHT};

use crate::en_bom::{ACTOR_EN_BOM, EnBom};

/// `ACTOR_BG_DODOAGO` (`actor_table.h`: 0x003F).
pub const ACTOR_BG_DODOAGO: i16 = 0x003F;
/// `OBJECT_DDAN_OBJECTS` (`object_table.h`: 0x002B).
pub const OBJECT: &str = "object_ddan_objects";
/// `gDodongoLowerJawCol`, `gDodongoLowerJawDL` (`object_ddan_objects.xml`).
pub const COLLISION: &str = "gDodongoLowerJawCol";
pub const DL: &str = "gDodongoLowerJawDL";

/// `EVENTCHKINF_B0` (`save.h`: 0xB0): Dodongo's Cavern's entrance wall broken.
pub const EVENTCHKINF_B0: u16 = 0xB0;

/// The one-point cutscenes: an eye lit (or lit already), the jaw opening.
pub const ONEPOINT_EYE: i16 = 3065;
pub const ONEPOINT_JAW: i16 = 3380;

/// The jaw open (`shape.rot.x`).
pub const JAW_OPEN: i16 = 0x1333;

/// `Bg_Dodoago_Profile`: `ACTORCAT_BG`, `FLAGS` 0, `OBJECT_DDAN_OBJECTS`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_DODOAGO, name: "Bg_Dodoago", category: ACTORCAT_BG, flags: 0, object: OBJECT, schema: &SCHEMA };

/// `PARAMS_GET_U(this->dyna.actor.params, 0, 6)`: the switch flag.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 0, 6, Kind::SwitchFlag, "set when both eyes are lit (the jaw opens); set at init, the jaw's open");

/// Bg_Dodoago's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[SWITCH_FLAG],
    flags: &[
        FlagUse::new(Flag::Switch("switchFlag")).reads().sets().note("set when both eyes are lit; set at init, the jaw's open; set, the bomb catcher stays off"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_B0)).reads().note("the entrance wall broken: the colliders registered, the jaw drawn"),
    ],
    todo: false,
    objects: &[],
    texts: &[],
};

/// `sMainColliderCylinderInit`: hit by anything but the shield and the mirror's ray
/// (`0xFFCFFFFF`), 80 across, 30 high, 80 up.
pub const MAIN_CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_ALL, oc_flags1: OC1_NONE, oc_flags2: OC2_NONE, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK2,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_NONE,
    },
    dim: Cylinder16 { radius: 80, height: 30, y_shift: 80, pos: [0, 0, 0] },
};

/// `sLeftRightColliderCylinderInit`: OC only, not pushing (`OC1_NO_PUSH`), 50 across, 60 high,
/// 280 up.
pub const LEFT_RIGHT_CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_NO_PUSH | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK2,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0000, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 50, height: 60, y_shift: 280, pos: [0, 0, 0] },
};

/// `BgDodoago_OpenJaw`'s `dustOffsets`: under the mouth, 200 down.
pub const DUST_OFFSETS: [Vec3; 11] = [
    Vec3::new(0.0, -200.0, 430.0),
    Vec3::new(20.0, -200.0, 420.0),
    Vec3::new(-20.0, -200.0, 420.0),
    Vec3::new(40.0, -200.0, 380.0),
    Vec3::new(-40.0, -200.0, 380.0),
    Vec3::new(50.0, -200.0, 350.0),
    Vec3::new(-50.0, -200.0, 350.0),
    Vec3::new(60.0, -200.0, 320.0),
    Vec3::new(-60.0, -200.0, 320.0),
    Vec3::new(70.0, -200.0, 290.0),
    Vec3::new(-70.0, -200.0, 290.0),
];

/// The colliders' ids (`collider_mut`).
pub const COL_MAIN: u8 = 0;
pub const COL_LEFT: u8 = 1;
pub const COL_RIGHT: u8 = 2;

/// The overlay's statics (docs/adr/0059-castle-towns-actors.md): `sFirstExplosiveFlag`,
/// `sDisableBombCatcher`, `sTimer` (`sUnused` isn't read).
#[derive(Debug, Default)]
pub struct Statics {
    pub first_explosive_flag: i16,
    pub disable_bomb_catcher: u8,
    pub timer: i32,
}

/// The statics, zeroed with a fresh overlay (`static s16 sFirstExplosiveFlag = false`; the
/// others in `.bss`).
pub fn statics(play: &mut PlayState) -> &mut Statics {
    play.overlay_static::<Statics>(ACTOR_BG_DODOAGO)
}

// shared: Actor_FreeOverlay
/// `Actor_FreeOverlay` (`z_actor.c`, from `Actor_Delete`): when the last Bg_Dodoago goes the
/// overlay unloads (`ACTOROVL_ALLOC_NORMAL`) and its statics come back with the next load
/// (`sFirstExplosiveFlag` false, the `.bss` zeroed).
fn actor_free_overlay(play: &mut PlayState) {
    let others = play.actors.all().into_iter().any(|h| play.actors.actor(h).is_some_and(|a| a.id == ACTOR_BG_DODOAGO));
    if !others {
        play.overlay_statics.remove(&ACTOR_BG_DODOAGO);
    }
}

// shared: roomCtx.drawParams (read)
/// `play->roomCtx.drawParams[i]` (0 with no scene loaded).
pub fn draw_param(play: &PlayState, i: usize) -> i16 {
    play.scene.as_ref().map_or(0, |s| s.draw.room_draw_params[i])
}

// shared: roomCtx.drawParams (written)
/// `play->roomCtx.drawParams[i] = v`.
fn set_draw_param(play: &mut PlayState, i: usize, v: i16) {
    match &mut play.scene {
        Some(s) => s.draw.room_draw_params[i] = v,
        None => log::debug!("Bg_Dodoago: roomCtx.drawParams[{i}] = {v} with no scene"),
    }
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `BgDodoago_WaitExplosives`.
    WaitExplosives,
    /// `BgDodoago_OpenJaw`.
    OpenJaw,
    /// `BgDodoago_DoNothing`.
    DoNothing,
    /// `BgDodoago_LightOneEye`.
    LightOneEye,
}

pub struct BgDodoago {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    /// `state`: the eye (`BgDodoagoEye`), or the jaw's step counted to 100.
    pub state: i16,
    pub main_collider: ColliderCylinder,
    pub left_collider: ColliderCylinder,
    pub right_collider: ColliderCylinder,
    pub action: Action,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

impl BgDodoago {
    /// `BgDodoago_Init`: scale 0.1 and the culling volume (`sInitChain`), the jaw's collision
    /// (`DynaPolyActor_Init(0)`); with its flag set the jaw open and both eyes lit, doing
    /// nothing; else its three cylinders, waiting for explosives, the catcher on.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: ICHAIN_VEC3F_DIV1000(scale, 100), cullingVolumeDistance 5000,
        // cullingVolumeScale 1000, cullingVolumeDownward 800.
        actor.scale = Vec3::splat(0.1);
        actor.culling_volume_distance = 5000.0;
        actor.culling_volume_scale = 1000.0;
        actor.culling_volume_downward = 800.0;
        let bg = match crate::obj_kibako2::load_collision(play, OBJECT, COLLISION) {
            Some(h) => play.col.dyna.set_bg_actor(h, source(&actor), 0),
            None => BG_ACTOR_MAX,
        };
        actor.shape_init(0.0, oot_game::shadow::ShadowDraw::None, 0.0);
        let mut this = BgDodoago {
            actor,
            bg,
            state: 0,
            main_collider: ColliderCylinder::default(),
            left_collider: ColliderCylinder::default(),
            right_collider: ColliderCylinder::default(),
            action: Action::WaitExplosives,
        };
        if play.flags.get_switch(this.switch_flag()) {
            this.action = Action::DoNothing;
            this.actor.shape_rot.x = JAW_OPEN;
            set_draw_param(play, BGDODOAGO_EYE_LEFT, 255);
            set_draw_param(play, BGDODOAGO_EYE_RIGHT, 255);
            play.col.dyna.set_source(this.bg, source(&this.actor));
            return Box::new(this);
        }
        this.main_collider = ColliderCylinder::new(&MAIN_CYLINDER_INIT);
        this.left_collider = ColliderCylinder::new(&LEFT_RIGHT_CYLINDER_INIT);
        this.right_collider = ColliderCylinder::new(&LEFT_RIGHT_CYLINDER_INIT);
        this.action = Action::WaitExplosives;
        statics(play).disable_bomb_catcher = 0;
        play.col.dyna.set_source(this.bg, source(&this.actor));
        Box::new(this)
    }

    /// `PARAMS_GET_U(this->dyna.actor.params, 0, 6)`.
    pub fn switch_flag(&self) -> i32 {
        SWITCH_FLAG.get(self.actor.params) as i32
    }

    /// `BgDodoago_SpawnSparkles`: four small sparkles (`EffectSsKiraKira_SpawnSmall`, grey) within
    /// 20 across and 10 up of `mean_pos`, falling at 1.5 and gathering 0.2.
    fn spawn_sparkles(mean_pos: Vec3, play: &mut PlayState) {
        let prim_color = [100, 100, 100, 0];
        let env_color = [40, 40, 40, 0];
        let velocity = Vec3::new(0.0, -1.5, 0.0);
        let acceleration = Vec3::new(0.0, -0.2, 0.0);
        play.with_ss(|ss| {
            for _ in 0..4 {
                let x = ss.rand.centered_float(20.0) + mean_pos.x;
                let y = ss.rand.centered_float(10.0) + mean_pos.y;
                let z = ss.rand.centered_float(20.0) + mean_pos.z;
                ss.kirakira_spawn_small(Vec3::new(x, y, z), velocity, acceleration, prim_color, env_color);
            }
        });
    }

    /// `BgDodoago_WaitExplosives`: an explosive's hit on the main cylinder lights its side's eye,
    /// or with the other lit opens the jaw, or (lit already) is the error; the first such
    /// explosive becomes the skull's `parent` (`sTimer` 50). No hit: with `EVENTCHKINF_B0` the
    /// three cylinders placed (200 in front; 215 in front and 90 either side) and registered.
    fn wait_explosives(&mut self, play: &mut PlayState) {
        let explosive = actor_get_collided_explosive(play, &mut self.main_collider.base);
        if let Some(explosive) = explosive {
            let explosive_pos = play.actors.actor(explosive).map(|a| a.world_pos).unwrap_or_default();
            self.state = if vec3f_yaw(self.actor.world_pos, explosive_pos) >= self.actor.shape_rot.y { BGDODOAGO_EYE_RIGHT as i16 } else { BGDODOAGO_EYE_LEFT as i16 };
            let me = play.cur_actor.map(|h| play.cam_actor_of(h, &self.actor));
            if (draw_param(play, BGDODOAGO_EYE_LEFT) == 255 && self.state == BGDODOAGO_EYE_RIGHT as i16) || (draw_param(play, BGDODOAGO_EYE_RIGHT) == 255 && self.state == BGDODOAGO_EYE_LEFT as i16) {
                play.flags.set_switch(self.switch_flag());
                self.state = 0;
                play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
                self.action = Action::OpenJaw;
                play.onepoint_cutscene_init(ONEPOINT_JAW, 160, me, CAM_ID_MAIN);
            } else if draw_param(play, self.state as usize) == 0 {
                play.onepoint_cutscene_init(ONEPOINT_EYE, 40, me, CAM_ID_MAIN);
                self.action = Action::LightOneEye;
                play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
            } else {
                play.onepoint_cutscene_init(ONEPOINT_EYE, 20, me, CAM_ID_MAIN);
                play.audio.play_sfx_centered(NA_SE_SY_ERROR);
                let st = statics(play);
                st.timer = st.timer.wrapping_add(30);
                return;
            }
            // The flag is never set back to false, so this only runs once.
            let st = statics(play);
            if st.first_explosive_flag == 0 {
                // This disables the bomb catcher (see BgDodoago_Update) for a few seconds.
                st.first_explosive_flag = 1;
                st.timer = 50;
                self.actor.parent = Some(explosive);
            }
        } else if play.save.get_event_chk_inf(EVENTCHKINF_B0) {
            self.main_collider.update(&self.actor);
            self.left_collider.update(&self.actor);
            self.right_collider.update(&self.actor);
            self.main_collider.dim.pos[2] = self.main_collider.dim.pos[2].wrapping_add(200);
            self.left_collider.dim.pos[2] = self.left_collider.dim.pos[2].wrapping_add(215);
            self.left_collider.dim.pos[0] = self.left_collider.dim.pos[0].wrapping_add(90);
            self.right_collider.dim.pos[2] = self.right_collider.dim.pos[2].wrapping_add(215);
            self.right_collider.dim.pos[0] = self.right_collider.dim.pos[0].wrapping_sub(90);
            play.collision_check_set_ac(&self.actor, COL_MAIN, &mut self.main_collider);
            play.collision_check_set_oc(&self.actor, COL_LEFT, &mut self.left_collider);
            play.collision_check_set_oc(&self.actor, COL_RIGHT, &mut self.right_collider);
        }
    }

    /// `BgDodoago_OpenJaw`: both eyes up 5 a frame to 255 (`sTimer` down meanwhile); then the dust
    /// on `sTimer` 108, the sparkles from both eyes, the rumble, and the jaw down to 0x1333 (the
    /// smooth step's scale `110 - state`, `state` up 3 a frame to 100); open, doing nothing.
    fn open_jaw(&mut self, play: &mut PlayState) {
        // Make both eyes red (one already is).
        for eye in [BGDODOAGO_EYE_LEFT, BGDODOAGO_EYE_RIGHT] {
            let v = draw_param(play, eye);
            if v < 255 {
                set_draw_param(play, eye, v.wrapping_add(5));
            }
        }
        if draw_param(play, BGDODOAGO_EYE_LEFT) != 255 || draw_param(play, BGDODOAGO_EYE_RIGHT) != 255 {
            let st = statics(play);
            st.timer = st.timer.wrapping_sub(1);
            return;
        }
        let p = self.actor.world_pos;
        if statics(play).timer == 108 {
            for d in DUST_OFFSETS.iter().rev() {
                let pos = Vec3::new(d.x + p.x, d.y + p.y, d.z + p.z);
                func_80033480(play, pos, 2.0, 3, 200, 75, 1);
            }
        }
        Self::spawn_sparkles(Vec3::new(p.x + 200.0, p.y - 20.0, p.z + 100.0), play);
        Self::spawn_sparkles(Vec3::new(p.x - 200.0, p.y - 20.0, p.z + 100.0), play);
        step_to_s(&mut self.state, 100, 3);
        // Rumble_Request(500.0f, 120, 20, 10): no rumble (BACKLOG #60).
        log::trace!("Bg_Dodoago: Rumble_Request(500, 120, 20, 10) not ported");
        if smooth_step_to_s(&mut self.actor.shape_rot.x, JAW_OPEN, 110 - self.state, 0x3E8, 0x32) == 0 {
            self.action = Action::DoNothing;
            audio_play_actor_sfx2(play, NA_SE_EV_STONE_BOUND);
        } else {
            audio_play_actor_sfx2(play, NA_SE_EV_STONE_STATUE_OPEN - SFX_FLAG);
        }
    }

    /// `BgDodoago_LightOneEye`: the eye's glow up 5; at 255, waiting again.
    fn light_one_eye(&mut self, play: &mut PlayState) {
        let eye = self.state as usize;
        let v = draw_param(play, eye).wrapping_add(5);
        set_draw_param(play, eye, v);
        if v == 255 {
            self.action = Action::WaitExplosives;
        }
    }

    /// `BgDodoago_Update`'s bomb catcher: with no `parent`, a bomb (`ACTOR_EN_BOM`, params 0)
    /// touching either OC cylinder gets fuse 50 and speed 0 and becomes the `parent`, `sTimer` 0.
    /// With one, `sTimer` up; past 140 (the catcher not off for good) the switch flag turns it off
    /// for good, else the `parent` is let go.
    fn bomb_catcher(&mut self, play: &mut PlayState) {
        if self.actor.parent.is_none() {
            // This is a "bomb catcher", it kills the XZ speed and sets the timer for bombs that
            // are dropped through the holes in the bridge above the skull.
            let (left_hit, right_hit) = (self.left_collider.base.oc_flags1 & OC1_HIT != 0, self.right_collider.base.oc_flags1 & OC1_HIT != 0);
            if left_hit || right_hit {
                let actor = if left_hit { self.left_collider.base.oc } else { self.right_collider.base.oc };
                self.left_collider.base.oc_flags1 &= !OC1_HIT;
                self.right_collider.base.oc_flags1 &= !OC1_HIT;
                let Some(h) = actor else { return };
                let is_bomb = play.actors.actor(h).is_some_and(|a| a.category == ACTORCAT_EXPLOSIVE && a.id == ACTOR_EN_BOM && crate::en_bom::TYPE.get(a.params) == 0);
                if is_bomb && let Some(bomb) = play.actors.downcast_mut::<EnBom>(h) {
                    // Disable the bomb catcher for a few seconds.
                    bomb.timer = 50;
                    bomb.actor.speed_xz = 0.0;
                    self.actor.parent = Some(h);
                    statics(play).timer = 0;
                }
            }
        } else {
            let st = statics(play);
            st.timer = st.timer.wrapping_add(1);
            // The C reads the flag here and drops the result.
            let _ = play.flags.get_switch(self.switch_flag());
            let st = statics(play);
            if st.disable_bomb_catcher == 0 && st.timer > 140 {
                if play.flags.get_switch(self.switch_flag()) {
                    // This prevents clearing the actor's parent pointer, effectively disabling
                    // the bomb catcher.
                    let st = statics(play);
                    st.disable_bomb_catcher = st.disable_bomb_catcher.wrapping_add(1);
                } else {
                    self.actor.parent = None;
                }
            }
        }
    }
}

impl ActorImpl for BgDodoago {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgDodoago_Update`: the bomb catcher, then the action; the jaw's transform for
    /// `DynaPoly_UpdateContext`.
    fn update(&mut self, play: &mut PlayState) {
        self.bomb_catcher(play);
        match self.action {
            Action::WaitExplosives => self.wait_explosives(play),
            Action::OpenJaw => self.open_jaw(play),
            Action::DoNothing => {}
            Action::LightOneEye => self.light_one_eye(play),
        }
        play.col.dyna.set_source(self.bg, source(&self.actor));
    }

    /// `BgDodoago_Destroy`: the jaw's collision (the cylinders: nothing to free); the overlay's
    /// statics go with the last skull.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
        actor_free_overlay(play);
    }

    /// `BgDodoago_Draw`: with `EVENTCHKINF_B0`, `Gfx_SetupDL_25Opa` and the jaw
    /// (`gDodongoLowerJawDL`) at its matrix.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if play.save.get_event_chk_inf(EVENTCHKINF_B0) {
            crate::gfx_draw_dlist_opa(out, OBJECT, DL, rs);
        }
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            COL_MAIN => Some(ColliderMut::Cylinder(&mut self.main_collider)),
            COL_LEFT => Some(ColliderMut::Cylinder(&mut self.left_collider)),
            COL_RIGHT => Some(ColliderMut::Cylinder(&mut self.right_collider)),
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
