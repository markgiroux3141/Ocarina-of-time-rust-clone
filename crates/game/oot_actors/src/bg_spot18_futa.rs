//! `Bg_Spot18_Futa` (`ovl_Bg_Spot18_Futa/z_bg_spot18_futa.c`): the lid on Goron City's spinning
//! urn, a DynaPoly actor of `object_spot18_obj` (`gGoronCityVaseLidCol`, `DynaPolyActor_Init(0)`)
//! drawn with `gGoronCityVaseLidDL`.
//!
//! Only `Bg_Spot18_Basket` spawns it, as its child (`Actor_SpawnAsChild`, turned 0x1555 from the
//! urn), while the urn waits for its switch flag. When the urn lets go of it (its `parent`
//! cleared, `BgSpot18Basket_Activation`) it shrinks away, 0.005 a frame from its scale of 0.1, and
//! is gone at 0.
//!
//! The whole overlay is ported.
//!
//! Params: none read (the urn spawns it with -1).
//! Flags: none.

use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use glam::Vec3;
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile};
use oot_game::actor_schema::ActorSchema;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

/// `ACTOR_BG_SPOT18_FUTA` (`actor_table.h`: 0x01C3).
pub const ACTOR_BG_SPOT18_FUTA: i16 = 0x01C3;
pub const OBJECT: &str = "object_spot18_obj";
/// `OBJECT_SPOT18_OBJ` (`object_table.h`: 0x00AF).
pub const OBJECT_SPOT18_OBJ: i16 = 0x00AF;
const COLLISION: &str = "gGoronCityVaseLidCol";
const DL: &str = "gGoronCityVaseLidDL";

/// `Bg_Spot18_Futa_Profile`: `FLAGS` 0.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_SPOT18_FUTA, name: "Bg_Spot18_Futa", category: ACTORCAT_PROP, flags: 0, object: OBJECT, schema: &SCHEMA };

/// Bg_Spot18_Futa's configuration (docs/adr/0066-the-actor-schema.md): nothing (never placed;
/// the urn spawns it with params -1).
pub const SCHEMA: ActorSchema = ActorSchema::NONE;

pub struct BgSpot18Futa {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

impl BgSpot18Futa {
    /// `BgSpot18Futa_Init`: `DynaPolyActor_Init(0)`, `gGoronCityVaseLidCol`, then `sInitChain`
    /// (scale 0.1; culling volume 1000, 500, 1000).
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = BgSpot18Futa { actor, bg: BG_ACTOR_MAX };
        if let Some(h) = crate::obj_kibako2::load_collision(play, OBJECT, COLLISION) {
            this.bg = play.col.dyna.set_bg_actor(h, source(&this.actor), 0);
        }
        // sInitChain: ICHAIN_VEC3F_DIV1000(scale, 100); cullingVolumeDistance 1000,
        // cullingVolumeScale 500, cullingVolumeDownward 1000.
        this.actor.scale = Vec3::splat(0.1);
        this.actor.culling_volume_distance = 1000.0;
        this.actor.culling_volume_scale = 500.0;
        this.actor.culling_volume_downward = 1000.0;
        play.col.dyna.set_source(this.bg, source(&this.actor));
        Box::new(this)
    }
}

impl ActorImpl for BgSpot18Futa {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgSpot18Futa_Update`: let go by the urn (no `parent`), its scale steps to 0 by 0.005 a
    /// frame (`scale.x`, then y and z follow), and it's gone once there.
    fn update(&mut self, play: &mut PlayState) {
        if self.actor.parent.is_none() {
            if eng_math::step_to_f(&mut self.actor.scale.x, 0.0, 0.005) {
                self.actor.kill();
            } else {
                self.actor.scale.z = self.actor.scale.x;
                self.actor.scale.y = self.actor.scale.x;
            }
        }
        play.col.dyna.set_source(self.bg, source(&self.actor));
    }

    /// `BgSpot18Futa_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }

    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }

    /// `BgSpot18Futa_Draw`: `Gfx_DrawDListOpa(gGoronCityVaseLidDL)`.
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
