//! `Bg_Spot00_Break` (`ovl_Bg_Spot00_Break/z_bg_spot00_break.c`): the adult's Hyrule Field's
//! ruins, a DynaPoly actor of `object_spot00_break` (`DynaPolyActor_Init(0)`): the broken
//! drawbridge in front of the market's entrance (by the field's spawn 7, where the adult comes out
//! of the ruined market) and a barbed wire fence (by the Lost Woods' bridge). Each is drawn
//! with its display list (`Gfx_DrawDListOpa`) and stands still; a child never sees them
//! (`BgSpot00Break_Init` kills it), and the field's child layers don't place them.
//!
//! The whole overlay is ported.
//!
//! Params: `params` whole, what it is (1 the barbed wire fence, `gBarbedWireFenceCol` and
//! `gBarbedWireFenceDL`; any other, 0 placed: the broken drawbridge, `gBrokenDrawbridgeCol` and
//! `gBrokenDrawbridgeDL`). `shape.rot.z`: its roll (two fences lean); with the yaw it turns the
//! collision and the model.
//! Flags: none.

use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use glam::Vec3;
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Kind, Source};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

/// `ACTOR_BG_SPOT00_BREAK` (`actor_table.h`: 0x0194).
pub const ACTOR_BG_SPOT00_BREAK: i16 = 0x0194;
pub const OBJECT: &str = "object_spot00_break";
const FENCE_COLLISION: &str = "gBarbedWireFenceCol";
const FENCE_DL: &str = "gBarbedWireFenceDL";
const DRAWBRIDGE_COLLISION: &str = "gBrokenDrawbridgeCol";
const DRAWBRIDGE_DL: &str = "gBrokenDrawbridgeDL";

/// `Bg_Spot00_Break_Profile`: `ACTORCAT_PROP`, `FLAGS` 0.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_SPOT00_BREAK, name: "Bg_Spot00_Break", category: ACTORCAT_PROP, flags: 0, object: OBJECT, schema: &SCHEMA };

/// `this->dyna.actor.params` (`BgSpot00Break_Init`'s and `_Draw`'s `params == 1`): which piece.
pub const TYPE: Field = Field::whole(
    "params",
    Kind::Named(&[(0, "the broken drawbridge (any value but 1)"), (1, "the barbed wire fence")]),
    "which piece of the ruins it is (1 the fence; any other the broken drawbridge)",
);

/// `shape.rot.z`: the piece's roll, drawn and collided as placed (two of the fences lean, 0x04FA and
/// -0x0444).
pub const ROT_Z: Field = Field::rot("shape.rot.z", Source::RotZ, 0, 16, Kind::Rotation, "its roll, drawn and collided as placed (the leaning fences)");

/// Bg_Spot00_Break's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[TYPE, ROT_Z], flags: &[], todo: false, objects: &[], texts: &[] };

pub struct BgSpot00Break {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

impl BgSpot00Break {
    /// Whether it's the barbed wire fence (`params == 1`).
    pub fn is_fence(&self) -> bool {
        TYPE.get(self.actor.params) == 1
    }

    /// `BgSpot00Break_Init`: `sInitChain` (the culling volume 1200 across, 1200 down, 2000 out;
    /// scale 1), `DynaPolyActor_Init(0)`, the piece's collision; killed for a child.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: ICHAIN_F32(cullingVolumeScale, 1200), ICHAIN_F32(cullingVolumeDownward,
        // 1200), ICHAIN_F32(cullingVolumeDistance, 2000), ICHAIN_VEC3F_DIV1000(scale, 1000).
        actor.culling_volume_scale = 1200.0;
        actor.culling_volume_downward = 1200.0;
        actor.culling_volume_distance = 2000.0;
        actor.scale = Vec3::ONE;
        let mut this = BgSpot00Break { actor, bg: BG_ACTOR_MAX };
        let collision = if this.is_fence() { FENCE_COLLISION } else { DRAWBRIDGE_COLLISION };
        if let Some(h) = crate::obj_kibako2::load_collision(play, OBJECT, collision) {
            this.bg = play.col.dyna.set_bg_actor(h, source(&this.actor), 0);
        }
        // !LINK_IS_ADULT.
        if !play.save.adult {
            this.actor.kill();
        }
        Box::new(this)
    }
}

impl ActorImpl for BgSpot00Break {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgSpot00Break_Update`: nothing.
    fn update(&mut self, _play: &mut PlayState) {}

    /// `BgSpot00Break_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }

    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }

    /// `BgSpot00Break_Draw`: `Gfx_DrawDListOpa` of the piece's list.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        crate::gfx_draw_dlist_opa(out, OBJECT, if self.is_fence() { FENCE_DL } else { DRAWBRIDGE_DL }, rs);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
