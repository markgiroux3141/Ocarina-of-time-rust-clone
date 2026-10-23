//! `Door_Toki` (`ovl_Door_Toki/z_door_toki.c`): the Door of Time's collision, a DynaPoly actor of
//! `object_toki_objects` (`gDoorTokiCol`, `DynaPolyActor_Init(0)`) with no draw: the door itself
//! is `Demo_Kankyo` type 0xD, which spawns this as its child.
//!
//! Each frame it blocks the doorway while the door is shut, and lets Link through once it's open
//! (`EVENTCHKINF_OPENED_DOOR_OF_TIME`, which `Demo_Kankyo` sets as the door finishes opening):
//! `DynaPoly_EnableCollision` / `DynaPoly_DisableCollision`.
//!
//! The whole overlay is ported.
//!
//! Params: none read (`Demo_Kankyo` spawns it with 0).
//! Flags: reads EVENTCHKINF_OPENED_DOOR_OF_TIME (the collision off once the door is open).

use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use glam::Vec3;
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, FlagUse};
use oot_game::event_chk_inf;
use oot_game::play::PlayState;

/// `ACTOR_DOOR_TOKI` (`actor_table.h`: 0x0070).
pub const ACTOR_DOOR_TOKI: i16 = 0x0070;
pub const OBJECT: &str = "object_toki_objects";
const COLLISION: &str = "gDoorTokiCol";

/// `EVENTCHKINF_OPENED_DOOR_OF_TIME` (`save.h`: 0x4B).
pub const EVENTCHKINF_OPENED_DOOR_OF_TIME: u16 = 0x4B;

/// `Door_Toki_Profile`: `FLAGS` 0, no draw.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_DOOR_TOKI, name: "Door_Toki", category: ACTORCAT_BG, flags: 0, object: OBJECT, schema: &SCHEMA };

/// Door_Toki's configuration (docs/adr/0066-the-actor-schema.md): no params (never placed;
/// `Demo_Kankyo` spawns it).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[],
    flags: &[FlagUse::new(event_chk_inf!(EVENTCHKINF_OPENED_DOOR_OF_TIME)).reads().note("the doorway's collision goes once the door is open")],
    todo: false,
    objects: &[],
    texts: &[],
};

pub struct DoorToki {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    /// The collision's state last set (the port asks the DynaPoly only on a change: its
    /// `set_collision_disabled` rebuilds the lookup each call, the C's flag write doesn't).
    pub disabled: Option<bool>,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

impl DoorToki {
    /// `DoorToki_Init`: scale 1 (`sInitChain`), `DynaPolyActor_Init(0)`, `gDoorTokiCol`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: ICHAIN_VEC3F_DIV1000(scale, 1000).
        actor.scale = Vec3::ONE;
        let mut this = DoorToki { actor, bg: BG_ACTOR_MAX, disabled: None };
        if let Some(h) = crate::obj_kibako2::load_collision(play, OBJECT, COLLISION) {
            this.bg = play.col.dyna.set_bg_actor(h, source(&this.actor), 0);
        }
        Box::new(this)
    }
}

impl ActorImpl for DoorToki {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `DoorToki_Update`: the collision off with the door open, else on.
    fn update(&mut self, play: &mut PlayState) {
        let open = play.save.get_event_chk_inf(EVENTCHKINF_OPENED_DOOR_OF_TIME);
        if self.disabled != Some(open) {
            play.col.dyna.set_collision_disabled(self.bg, open);
            self.disabled = Some(open);
        }
    }

    /// `DoorToki_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
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
