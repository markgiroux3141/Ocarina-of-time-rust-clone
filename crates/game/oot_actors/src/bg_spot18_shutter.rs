//! `Bg_Spot18_Shutter` (`ovl_Bg_Spot18_Shutter/z_bg_spot18_shutter.c`): Darunia's door in Goron
//! City, a DynaPoly actor of `object_spot18_obj` (`gGoronCityDoorCol`, drawn with
//! `gGoronCityDoorDL`) at a tenth of its size.
//!
//! Bit 8 of its params picks the door:
//! - **0, the child's door** (`BgSpot18Shutter_Init`): for a child it waits for its switch flag
//!   (`func_808B95B8`; Zelda's Lullaby at the song spot `En_Okarina_Tag` 0x1C9B sets 0x1B), then
//!   its focus 70 up, `OnePointCutscene_Attention` on it, and it rises 180 above its home at 1.44 a
//!   frame (`func_808B9698`: `NA_SE_EV_STONE_STATUE_OPEN` each frame, `NA_SE_EV_STONEDOOR_STOP` at
//!   the top). With the flag already set it starts 180 up. For an adult the same door waits for
//!   `INFTABLE_109` instead (`func_808B9618`) and rises without the attention cutscene; with it set
//!   it starts up.
//! - **1, the adult's sideways door**: with `INFTABLE_109` it starts 125 along its local x
//!   (`world.pos.x += 125 cos(rot.y)`, `z -= 125 sin(rot.y)`); without, it waits for it
//!   (`func_808B9618`), then one-point cutscene 4221 (140 frames) and it slides there
//!   (`func_808B971C`: x by `|cos|` and z by `|sin|` a frame, with the same sounds).
//!
//! The whole overlay is ported.
//!
//! Params: `type` bit 8 (0: the child's door, rising on the switch flag; 1: the adult's, sliding on
//! INFTABLE_109); `switchFlag` bits 0 to 5 (type 0, for a child: set, the door opens; no none
//! value); bits 6 and 7 unused (3 in the adult layers' 0x00FF).
//! Flags: reads the switch flag `switchFlag` (type 0, a child); reads INFTABLE_109 (an adult's
//! type 0, and type 1: open).

use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use eng_math::{cos_s, sin_s, step_to_f};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::{NA_SE_EV_STONE_STATUE_OPEN, NA_SE_EV_STONEDOOR_STOP, SFX_FLAG};
use oot_game::camera::CAM_ID_MAIN;
use oot_game::inf_table;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

/// `ACTOR_BG_SPOT18_SHUTTER` (`actor_table.h`: 0x01C4).
pub const ACTOR_BG_SPOT18_SHUTTER: i16 = 0x01C4;
/// `OBJECT_SPOT18_OBJ` (`object_table.h`: 0x00AF).
pub const OBJECT: &str = "object_spot18_obj";

/// `Bg_Spot18_Shutter_Profile`: `ACTORCAT_PROP`, `FLAGS` (`ACTOR_FLAG_UPDATE_CULLING_DISABLED |
/// ACTOR_FLAG_DRAW_CULLING_DISABLED`).
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_BG_SPOT18_SHUTTER,
    name: "Bg_Spot18_Shutter",
    category: ACTORCAT_PROP,
    flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `PARAMS_GET_U(this->dyna.actor.params, 8, 1)`: which door.
pub const TYPE: Field = Field::params(
    "type",
    8,
    1,
    Kind::Named(&[(0, "the child's door: rises 180 on its switch flag (an adult's: on INFTABLE_109)"), (1, "the adult's door: slides 125 aside on INFTABLE_109 (one-point 4221)")]),
    "which door",
);
/// `PARAMS_GET_U(this->dyna.actor.params, 0, 6)`: the switch flag.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 0, 6, Kind::SwitchFlag, "type 0, for a child: set, the door rises (no none value)");
/// Bits 6 and 7: 3 in the adult layers' 0x00FF, read by nothing.
pub const BITS_6_7: Field = Field::params("params bits 6 and 7", 6, 2, Kind::Unused, "3 in the adult layers' 0x00FF; read by nothing");

/// `INFTABLE_109` (`save.h`: 0x109): Darunia's door open for an adult.
pub const INFTABLE_109: u16 = 0x109;

/// Bg_Spot18_Shutter's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, SWITCH_FLAG, BITS_6_7],
    flags: &[
        FlagUse::new(Flag::Switch("switchFlag")).reads().note("type 0, a child: set, the door rises (starts up when set)"),
        FlagUse::new(inf_table!(INFTABLE_109)).reads().note("an adult's type 0 and type 1: the door opens (starts open when set)"),
    ],
    todo: false,
    objects: &[],
};

/// How far the child's door rises (`func_808B9698`) and the adult's slides (`func_808B971C`).
pub const RISE: f32 = 180.0;
pub const RISE_STEP: f32 = 1.44;
pub const SLIDE: f32 = 125.0;

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_808B95AC`: open (or shut for good): nothing.
    Idle,
    /// `func_808B95B8`: the child's door waiting for its switch flag.
    WaitSwitch,
    /// `func_808B9618`: waiting for `INFTABLE_109`.
    WaitInfTable,
    /// `func_808B9698`: rising.
    Rise,
    /// `func_808B971C`: sliding aside.
    Slide,
}

pub struct BgSpot18Shutter {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub action: Action,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

impl BgSpot18Shutter {
    /// `BgSpot18Shutter_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = BgSpot18Shutter { actor, bg: BG_ACTOR_MAX, action: Action::Idle };
        let param = TYPE.get(this.actor.params);
        // DynaPolyActor_Init(&this->dyna, 0); sInitChain: ICHAIN_VEC3F_DIV1000(scale, 100).
        this.actor.scale = Vec3::splat(0.1);

        if param == 0 {
            // LINK_AGE_IN_YEARS == YEARS_ADULT.
            if play.save.adult {
                if play.save.get_inf_table(INFTABLE_109) {
                    this.action = Action::Idle;
                    this.actor.world_pos.y += RISE;
                } else {
                    this.action = Action::WaitInfTable;
                }
            } else if play.flags.get_switch(SWITCH_FLAG.get(this.actor.params) as i32) {
                this.action = Action::Idle;
                this.actor.world_pos.y += RISE;
            } else {
                this.action = Action::WaitSwitch;
            }
        } else if play.save.get_inf_table(INFTABLE_109) {
            let rot_y = this.actor.world_rot.y;
            this.actor.world_pos.x += SLIDE * cos_s(rot_y);
            this.actor.world_pos.z -= SLIDE * sin_s(rot_y);
            this.action = Action::Idle;
        } else {
            this.action = Action::WaitInfTable;
        }

        // CollisionHeader_GetVirtual(&gGoronCityDoorCol), DynaPoly_SetBgActor.
        if let Some(h) = crate::obj_kibako2::load_collision(play, OBJECT, "gGoronCityDoorCol") {
            this.bg = play.col.dyna.set_bg_actor(h, source(&this.actor), 0);
        }
        Box::new(this)
    }

    /// `func_808B95B8`: the switch flag set: the focus 70 up, the attention cutscene, rising.
    fn wait_switch(&mut self, play: &mut PlayState) {
        if play.flags.get_switch(SWITCH_FLAG.get(self.actor.params) as i32) {
            self.actor.set_focus(70.0);
            if let Some(me) = play.cur_actor.map(|h| play.cam_actor_of(h, &self.actor)) {
                play.onepoint_attention(me);
            }
            self.action = Action::Rise;
        }
    }

    /// `func_808B9618`: `INFTABLE_109` set: the focus 70 up; the child's door rises, the adult's
    /// slides aside under one-point cutscene 4221 (140 frames).
    fn wait_inf_table(&mut self, play: &mut PlayState) {
        if play.save.get_inf_table(INFTABLE_109) {
            self.actor.set_focus(70.0);
            if TYPE.get(self.actor.params) == 0 {
                self.action = Action::Rise;
            } else {
                self.action = Action::Slide;
                let me = play.cur_actor.map(|h| play.cam_actor_of(h, &self.actor));
                play.onepoint_cutscene_init(4221, 140, me, CAM_ID_MAIN);
            }
        }
    }

    /// `func_808B9698`: up to 180 above home at 1.44 a frame; at the top `NA_SE_EV_STONEDOOR_STOP`,
    /// on the way `NA_SE_EV_STONE_STATUE_OPEN`.
    fn rise(&mut self, play: &mut PlayState) {
        if step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y + RISE, RISE_STEP) {
            audio_play_actor_sfx2(play, NA_SE_EV_STONEDOOR_STOP);
            self.action = Action::Idle;
        } else {
            self.actor.play_sfx_flagged(NA_SE_EV_STONE_STATUE_OPEN - SFX_FLAG);
        }
    }

    /// `func_808B971C`: 125 along its local x from home, x by `|cos|` and z by `|sin|` a frame
    /// (both steps taken every frame); there, `NA_SE_EV_STONEDOOR_STOP`.
    fn slide(&mut self, play: &mut PlayState) {
        let sin = sin_s(self.actor.world_rot.y);
        let cos = cos_s(self.actor.world_rot.y);
        let mut flag = true;
        flag &= step_to_f(&mut self.actor.world_pos.x, self.actor.home_pos.x + (SLIDE * cos), cos.abs());
        flag &= step_to_f(&mut self.actor.world_pos.z, self.actor.home_pos.z - (SLIDE * sin), sin.abs());
        if flag {
            audio_play_actor_sfx2(play, NA_SE_EV_STONEDOOR_STOP);
            self.action = Action::Idle;
        } else {
            self.actor.play_sfx_flagged(NA_SE_EV_STONE_STATUE_OPEN - SFX_FLAG);
        }
    }
}

impl ActorImpl for BgSpot18Shutter {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `BgSpot18Shutter_Update`: the action; the new transform for `DynaPoly_UpdateContext`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Idle => {}
            Action::WaitSwitch => self.wait_switch(play),
            Action::WaitInfTable => self.wait_inf_table(play),
            Action::Rise => self.rise(play),
            Action::Slide => self.slide(play),
        }
        play.col.dyna.set_source(self.bg, source(&self.actor));
    }
    /// `BgSpot18Shutter_Destroy`: `DynaPoly_DeleteBgActor`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }
    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }
    fn render_state(&self) -> RenderState {
        RenderState::of(&self.actor)
    }
    /// `BgSpot18Shutter_Draw`: `Gfx_DrawDListOpa(play, gGoronCityDoorDL)`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        crate::gfx_draw_dlist_opa(out, OBJECT, "gGoronCityDoorDL", rs);
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
