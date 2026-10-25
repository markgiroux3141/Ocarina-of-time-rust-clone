//! `Bg_Umajump` (`ovl_Bg_Umajump/z_bg_umajump.c`): a fence a horse can jump, a DynaPoly actor of
//! `object_umajump` (`DynaPolyActor_Init(0)`, `gJumpableHorseFenceCol`) drawn with its display list
//! (`gJumpableHorseFenceDL`, `Gfx_DrawDListOpa`). It stands still. `En_Horse` jumps it, as it jumps
//! any low wall, by its own line test (`EnHorse_UpdateBgCheckInfo`): on Malon's obstacle course
//! (`HORSE_PTYPE_6` with Epona had, `ENHORSE_FLAG_26`) only a wall of a `Bg_Umajump` can be jumped.
//!
//! Lon Lon Ranch's layers place type 0 (two in the corral, on the adult's day and night layers) and
//! type 1 (the course's eight on its layer 4, `CS_INDEX_0`); Hyrule Field's unused cutscene layer 10
//! places two of 0xFFFF.
//!
//! The whole overlay is ported.
//!
//! Params: `params` whole, 1 the obstacle course's fence (killed until Epona is had; then never
//! culled), any other a fence that's always there (0 and 0xFFFF placed).
//! Flags: reads EVENTCHKINF_EPONA_OBTAINED (type 1: killed without it) and the debug
//! `R_DEBUG_FORCE_EPONA_OBTAINED` (0).

use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, Source};
use oot_game::event_chk_inf;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

/// `ACTOR_BG_UMAJUMP` (`actor_table.h`: 0x0108).
pub const ACTOR_BG_UMAJUMP: i16 = 0x0108;
pub const OBJECT: &str = "object_umajump";
const COLLISION: &str = "gJumpableHorseFenceCol";
const DL: &str = "gJumpableHorseFenceDL";

/// `EVENTCHKINF_EPONA_OBTAINED` (`save.h`).
pub const EVENTCHKINF_EPONA_OBTAINED: u16 = 0x18;
/// `R_DEBUG_FORCE_EPONA_OBTAINED` (`regs.h`: `DREG(1)`): the debug registers start zeroed.
const R_DEBUG_FORCE_EPONA_OBTAINED: i16 = 0;

/// `Bg_Umajump_Profile`: `ACTORCAT_PROP`, `FLAGS` 0.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_UMAJUMP, name: "Bg_Umajump", category: ACTORCAT_PROP, flags: 0, object: OBJECT, schema: &SCHEMA };

/// `this->dyna.actor.params == 1`: the obstacle course's fence.
pub const TYPE: Field = Field::whole(
    "params",
    Kind::Named(&[(-1, "a fence that's always there (0xFFFF placed: Hyrule Field's)"), (0, "a fence that's always there (any value but 1)"), (1, "the obstacle course's fence: only with Epona had, never culled")]),
    "which fence: 1 the obstacle course's (killed until Epona is had), any other always there",
);

/// `shape.rot.z`: collided and drawn as placed (Hyrule Field's leaning fence).
pub const ROT_Z: Field = Field::rot("shape.rot.z", Source::RotZ, 0, 16, Kind::Rotation, "its roll, collided and drawn as placed (one of Hyrule Field's fences leans)");

/// Bg_Umajump's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, ROT_Z],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_EPONA_OBTAINED)).reads().note("type 1: killed without it"),
        FlagUse::new(Flag::Other("R_DEBUG_FORCE_EPONA_OBTAINED")).reads().note("the debug register DREG(1): Epona's as obtained (0)"),
    ],
    todo: false,
    objects: &[],
    texts: &[],
};

pub struct BgUmajump {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

impl BgUmajump {
    /// `BgUmaJump_Init`: `sInitChain` (scale 0.1), `DynaPolyActor_Init(0)`, the fence's collision;
    /// type 1 killed without Epona, else never culled.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: ICHAIN_VEC3F_DIV1000(scale, 100).
        actor.scale = Vec3::splat(0.1);
        let mut this = BgUmajump { actor, bg: BG_ACTOR_MAX };
        if let Some(h) = crate::obj_kibako2::load_collision(play, OBJECT, COLLISION) {
            this.bg = play.col.dyna.set_bg_actor(h, source(&this.actor), 0);
        }
        if TYPE.get(this.actor.params) == 1 {
            if !(play.save.get_event_chk_inf(EVENTCHKINF_EPONA_OBTAINED) || R_DEBUG_FORCE_EPONA_OBTAINED != 0) {
                this.actor.kill();
                return Box::new(this);
            }
            this.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED;
        }
        Box::new(this)
    }
}

impl ActorImpl for BgUmajump {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgUmaJump_Update`: nothing.
    fn update(&mut self, _play: &mut PlayState) {}

    /// `BgUmaJump_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }

    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }

    /// `BgUmaJump_Draw`: `Gfx_DrawDListOpa(play, gJumpableHorseFenceDL)`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        crate::gfx_draw_dlist_opa(out, OBJECT, DL, rs);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
