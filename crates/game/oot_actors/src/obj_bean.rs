//! `Obj_Bean` (`ovl_Obj_Bean/z_obj_bean.c`): the soft soil a magic bean is planted in, and the
//! beanstalk lift an adult rides from it. Its params: the switch flag set once a bean is planted
//! (bits 0 to 5), the lift's path (bits 8 to 12).
//!
//! Ported (GAME-06 milestone 2, as decided): what a child with no bean planted meets, the soft
//! soil (`ObjBean_SetupWaitForBean`, `ObjBean_WaitForBean`: its patch drawn, its talk offered for
//! the magic bean, text 0x2F). Planting a bean (`func_80B8FE00` on), the watering and the leaves
//! (`ObjBean_SetupWaitForWater` on), and the adult's lift (its path, its flight, its DynaPoly)
//! are logged: their actor stands and draws nothing.
//!
//! Params: `switchFlag` bits 0 to 5 (set once a bean is planted: a child's plant waits for water,
//! an adult's lift is there; clear, an adult's is gone); `path` bits 8 to 12 (the adult's lift's
//! path; 0x1F: none, killed). `home.rot.z & 3`: the lift's speed (`sBeanSpeeds`: 0 and 3 slow, 1
//! fast, 2 fastest); rot.z is zeroed at init. The lift isn't ported: the path and speed are only
//! logged.
//! Flags: reads switch flag `switchFlag`, and sets it as a bean is planted.

use eng_gfx::{DrawCmd, MeshKey};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_IGNORE_POINT_LIGHTS, Actor};
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, Source, TextLine};
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::sys_matrix::binang_to_rad;

/// `ACTOR_OBJ_BEAN` (`actor_table.h`: 0x0126).
pub const ACTOR_OBJ_BEAN: i16 = 0x0126;
pub const OBJECT: &str = "object_mamenoki";

/// `Obj_Bean_Profile`: `ACTORCAT_BG`, `ACTOR_FLAG_IGNORE_POINT_LIGHTS`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_OBJ_BEAN, name: "Obj_Bean", category: ACTORCAT_BG, flags: ACTOR_FLAG_IGNORE_POINT_LIGHTS, object: OBJECT, schema: &SCHEMA };

/// `PARAMS_GET_U(params, 0, 6)`: the switch flag.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 0, 6, Kind::SwitchFlag, "set once a bean is planted (an adult's lift there only then)");
/// `PARAMS_GET_U(params, 8, 5)`: the adult's lift's path.
pub const PATH: Field = Field::params("path", 8, 5, Kind::PathIndex, "the adult's beanstalk lift's path (0x1F: none, killed; under 3 points: killed)");
/// `home.rot.z & 3` (`unk_1F6`): an index into `sBeanSpeeds`.
pub const SPEED: Field = Field::rot(
    "home.rot.z & 3",
    Source::RotZ,
    0,
    2,
    Kind::Named(&[(0, "slow (3, 0.3)"), (1, "fast (10, 0.5)"), (2, "fastest (30, 0.5)"), (3, "slow (3, 0.3)")]),
    "the adult's lift's speed and acceleration (`sBeanSpeeds`)",
);

/// Obj_Bean's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[SWITCH_FLAG, PATH, SPEED],
    flags: &[FlagUse::new(Flag::Switch("switchFlag")).reads().sets().note("set as a bean is planted")],
    todo: false,
    objects: &[],
    texts: &[TextLine::new(0x002F).cond("no bean planted yet").note("the soft soil")],
};

/// `BEAN_STATE_*`.
pub const BEAN_STATE_DRAW_LEAVES: u8 = 1 << 0;
pub const BEAN_STATE_DRAW_SOIL: u8 = 1 << 1;
pub const BEAN_STATE_DRAW_PLANT: u8 = 1 << 2;
pub const BEAN_STATE_DRAW_STALK: u8 = 1 << 3;

/// `EXCH_ITEM_MAGIC_BEAN` (`item.h`: 4).
pub const EXCH_ITEM_MAGIC_BEAN: u8 = 0x04;
/// The soft soil's text (`dyna.actor.textId`).
pub const TEXT_SOFT_SOIL: u16 = 0x2F;

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `ObjBean_WaitForBean`.
    WaitForBean,
    /// What isn't ported: planting, watering, the lift.
    NotPorted,
}

pub struct ObjBean {
    /// `dyna.actor`.
    pub actor: Actor,
    pub action: Action,
    pub timer: i16,
    pub state_flags: u8,
}

impl ObjBean {
    /// The switch flag: `PARAMS_GET_U(params, 0, 6)`.
    fn switch_flag(&self) -> i32 {
        SWITCH_FLAG.get(self.actor.params) as i32
    }

    /// `ObjBean_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: ICHAIN_VEC3F_DIV1000(scale, 100); cullingVolumeDistance 2000,
        // cullingVolumeScale 200, cullingVolumeDownward 1600.
        actor.scale = Vec3::splat(0.1);
        actor.culling_volume_distance = 2000.0;
        actor.culling_volume_scale = 200.0;
        actor.culling_volume_downward = 1600.0;
        let mut this = ObjBean { actor, action: Action::NotPorted, timer: 0, state_flags: 0 };
        let flag = play.flags.get_switch(this.switch_flag());
        if play.save.adult {
            if flag {
                let (path, speed) = (PATH.get(this.actor.params), SPEED.get(this.actor.home_rot.z));
                log::warn!("Obj_Bean: the adult's beanstalk lift (path {path}, speed {speed}, ObjBean_SetupWaitForPlayer) isn't ported");
            } else {
                this.actor.kill();
                return Box::new(this);
            }
        } else if flag {
            log::warn!("Obj_Bean: a planted bean waiting for water (ObjBean_SetupWaitForWater) isn't ported");
        } else {
            this.setup_wait_for_bean();
        }
        this.actor.world_rot.z = 0;
        this.actor.home_rot.z = 0;
        this.actor.shape_rot.z = 0;
        log::debug!("(Magic beanstalk lift)(arg_data {:#06x})", this.actor.params); // schema: printed

        Box::new(this)
    }

    /// `ObjBean_SetDrawMode`.
    fn set_draw_mode(&mut self, draw_flag: u8) {
        self.state_flags &= !(BEAN_STATE_DRAW_LEAVES | BEAN_STATE_DRAW_PLANT | BEAN_STATE_DRAW_STALK | BEAN_STATE_DRAW_SOIL);
        self.state_flags |= draw_flag;
    }

    /// `ObjBean_SetupWaitForBean`: the patch, text 0x2F.
    fn setup_wait_for_bean(&mut self) {
        self.action = Action::WaitForBean;
        self.set_draw_mode(BEAN_STATE_DRAW_LEAVES);
        self.actor.text_id = TEXT_SOFT_SOIL;
    }

    /// `ObjBean_WaitForBean`: its talk offered within 40 for the magic bean; a bean given plants
    /// (not ported).
    fn wait_for_bean(&mut self, play: &mut PlayState) {
        if oot_game::npc::process_talk_request(&mut self.actor) {
            let exchange = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map(|p| p.exchange_item_id()).unwrap_or(0);
            if exchange == EXCH_ITEM_MAGIC_BEAN {
                log::warn!("Obj_Bean: planting a magic bean (func_80B8FE00) isn't ported");
                play.flags.set_switch(self.switch_flag());
                self.action = Action::NotPorted;
            }
        } else {
            let a = self.actor.clone();
            oot_game::npc::offer_talk_exchange(play, &a, 40.0, EXCH_ITEM_MAGIC_BEAN);
        }
    }
}

impl ActorImpl for ObjBean {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `ObjBean_Update`: the timer, the action; the plant's movement and shadow aren't reached
    /// (`BEAN_STATE_DRAW_PLANT` is never set: `shadowDraw` NULL); `Actor_SetFocus(6)`.
    fn update(&mut self, play: &mut PlayState) {
        if self.timer > 0 {
            self.timer -= 1;
        }
        match self.action {
            Action::WaitForBean => self.wait_for_bean(play),
            Action::NotPorted => {}
        }
        // Not BEAN_STATE_DRAW_PLANT.
        self.actor.shape_shadow_draw = oot_game::shadow::ShadowDraw::None;
        self.actor.set_focus(6.0);
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.switches = vec![self.state_flags as u32];
        rs
    }

    /// `ObjBean_Draw`: the soft soil's patch (`BEAN_STATE_DRAW_LEAVES`, `ObjBean_DrawSoftSoilSpot`)
    /// at its home, turned by its home's yaw, at 0.1.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let flags = rs.switches.first().copied().unwrap_or(0) as u8;
        if flags & BEAN_STATE_DRAW_LEAVES != 0 {
            let h = self.actor.home_pos;
            let m = Mat4::from_translation(h) * Mat4::from_rotation_y(binang_to_rad(self.actor.home_rot.y)) * Mat4::from_scale(Vec3::splat(0.1));
            out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh(OBJECT, "gMagicBeanSoftSoilDL")), m));
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
