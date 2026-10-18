//! `Bg_Spot05_Soko` (`ovl_Bg_Spot05_Soko/z_bg_spot05_soko.c`): the Sacred Forest Meadow's
//! child-only blocks and its gate (GAME-08 milestone 1), DynaPoly actors of
//! `object_spot05_objects`.
//!
//! - **Type 0** (`object_spot05_objects_Col_000918`, `_DL_000840`): one of the four blocks in the
//!   meadow's maze that stand only for a child (killed for an adult at init); it does nothing.
//! - **Type 1** (`_Col_0012C0`, `_DL_001190`): the gate by the entrance, gone at init if its
//!   switch flag is set. When the flag is set (the gray Wolfos's death: `En_Wf` 0x1F00 sets 0x1F)
//!   it sounds (`NA_SE_EV_METALDOOR_CLOSE`, 30 frames at its position), the camera turns to it
//!   (`OnePointCutscene_Attention`, its focus 50 up) and it sinks, its speed 0.5 growing ×1.5 a
//!   frame, to 120 below its home, then it's gone; it updates wherever Link is.
//!
//! The whole overlay is ported.
//!
//! Params: `type` bits 0 to 7 (0 a child's block, 1 the gate; init writes it over the params);
//! `switchFlag` bits 8 to 15 (the gate's; 0x1F in the meadow; the blocks' 0).
//! Flags: the gate reads switch flag `switchFlag` (set: gone at init, else it sinks when set).

use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::NA_SE_EV_METALDOOR_CLOSE;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

/// `ACTOR_BG_SPOT05_SOKO` (`actor_table.h`: 0x018D).
pub const ACTOR_BG_SPOT05_SOKO: i16 = 0x018D;
/// `OBJECT_SPOT05_OBJECTS` (`object_table.h`: 0x0161).
pub const OBJECT: &str = "object_spot05_objects";

/// `Bg_Spot05_Soko_Profile`: `ACTORCAT_PROP`, `FLAGS` 0.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_SPOT05_SOKO, name: "Bg_Spot05_Soko", category: ACTORCAT_PROP, flags: 0, object: OBJECT, schema: &SCHEMA };

/// `thisx->params &= 0xFF`: the type.
pub const TYPE: Field = Field::params("type", 0, 8, Kind::Named(&[(0, "a child's block"), (1, "the gate")]), "a block that stands for a child, or the gate");
/// `PARAMS_GET_U(thisx->params, 8, 8)`: the gate's switch flag.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 8, 8, Kind::SwitchFlag, "set: the gate sinks (gone at init if already set)").when("type", &[1]);
/// The blocks' bits 8 to 15: 0 in the data, read by nothing.
pub const UNUSED: Field = Field::params("switchFlag (unused)", 8, 8, Kind::Unused, "0 in the data; the blocks don't read it").when("type", &[0]);

/// Bg_Spot05_Soko's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema =
    ActorSchema { fields: &[TYPE, SWITCH_FLAG, UNUSED], flags: &[FlagUse::new(Flag::Switch("switchFlag")).reads().note("the gate: set, it sinks")], todo: false, objects: &[] };

/// `sDLists`.
const DLISTS: [&str; 2] = ["object_spot05_objects_DL_000840", "object_spot05_objects_DL_001190"];
/// The collisions by type.
const COLLISIONS: [&str; 2] = ["object_spot05_objects_Col_000918", "object_spot05_objects_Col_0012C0"];

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_808AE5A8`: a block: nothing.
    Block,
    /// `func_808AE5B4`: the gate, waiting for its flag.
    Wait,
    /// `func_808AE630`: the gate sinking.
    Sink,
}

pub struct BgSpot05Soko {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub action: Action,
    /// `switchFlag`.
    pub switch_flag: i32,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

impl BgSpot05Soko {
    /// `BgSpot05Soko_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = BgSpot05Soko { actor, bg: BG_ACTOR_MAX, action: Action::Block, switch_flag: 0 };
        // sInitChain: ICHAIN_VEC3F_DIV1000(scale, 100).
        this.actor.scale = Vec3::splat(0.1);
        this.switch_flag = SWITCH_FLAG.get(this.actor.params) as i32;
        this.actor.params = TYPE.get(this.actor.params);
        // DynaPolyActor_Init(&this->dyna, 0).
        let ty = TYPE.get(this.actor.params) as usize;
        if ty == 0 {
            if play.save.adult {
                this.actor.kill();
            } else {
                this.action = Action::Block;
            }
        } else if play.flags.get_switch(this.switch_flag) {
            this.actor.kill();
        } else {
            this.action = Action::Wait;
            this.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
        }
        // DynaPoly_SetBgActor (after the kill too, as the C does; the destroy deletes it).
        if let Some(h) = crate::obj_kibako2::load_collision(play, OBJECT, COLLISIONS[ty.min(1)]) {
            this.bg = play.col.dyna.set_bg_actor(h, source(&this.actor), 0);
        }
        Box::new(this)
    }

    /// `func_808AE5B4`: the flag set, the sound, the attention cutscene, and sinking.
    fn wait(&mut self, play: &mut PlayState) {
        if play.flags.get_switch(self.switch_flag) {
            play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, 30, NA_SE_EV_METALDOOR_CLOSE);
            self.actor.set_focus(50.0);
            if let Some(me) = play.cur_actor.map(|h| play.cam_actor_of(h, &self.actor)) {
                play.onepoint_attention(me);
            }
            self.action = Action::Sink;
            self.actor.speed_xz = 0.5;
        }
    }

    /// `func_808AE630`: down, faster each frame, to 120 below home; then gone.
    fn sink(&mut self) {
        self.actor.speed_xz *= 1.5;
        if eng_math::step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y - 120.0, self.actor.speed_xz) {
            self.actor.kill();
        }
    }
}

impl ActorImpl for BgSpot05Soko {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `BgSpot05Soko_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Block => {}
            Action::Wait => self.wait(play),
            Action::Sink => self.sink(),
        }
        play.col.dyna.set_source(self.bg, source(&self.actor));
    }
    /// `BgSpot05Soko_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }
    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }
    fn render_state(&self) -> RenderState {
        RenderState::of(&self.actor)
    }
    /// `BgSpot05Soko_Draw`: `Gfx_DrawDListOpa(sDLists[params])`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        crate::gfx_draw_dlist_opa(out, OBJECT, DLISTS[(TYPE.get(self.actor.params) as usize).min(1)], rs);
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
