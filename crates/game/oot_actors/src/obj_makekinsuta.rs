//! `Obj_Makekinsuta` (`ovl_Obj_Makekinsuta/z_obj_makekinsuta.c`): the soft soil where bugs make
//! a Gold Skulltula. Invisible (no draw). Once a released bug (`En_Insect`) has reached it and
//! set `unk_152`, it counts 60 frames, then (unless Link is dead, in `PLAYER_STATE1_29` or in a
//! cutscene action: `func_8002DEEC`) spawns a Gold Skulltula (`En_Sw`, params `params | 0x8000`:
//! type 3 after `EnSw_Init`'s conversion, which jumps out) at its position facing its yaw, and
//! does nothing more (`ObjMakekinsuta_DoNothing`). Until `unk_152` is set its timer stays 0.
//!
//! The whole overlay is ported. Nothing sets `unk_152` yet: the bugs (`En_Insect`,
//! `z_en_insect.c`'s `EnInsect_Update`) aren't ported.
//!
//! Params: `params` whole, the Gold Skulltula's params (spawned with 0x8000 set): bits 0 to 7 its
//! flag (its bit in `gsFlags`), bits 8 to 12 its `gsFlags` index plus 1, bits 13 and 14
//! (`PARAMS_GET_NOSHIFT(params, 13, 2)`) 0x4000 for a Gold Skulltula soil (`En_Sw`'s type 3);
//! any other value there only logs "Invalid Argument". The graveyard's is 0x5101 (index 0x10,
//! flag 1).
//! Flags: none (the Gold Skulltula reads its own).

use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::play::PlayState;

use crate::en_sw::ACTOR_EN_SW;
use crate::player::{STATE1_7, STATE1_29};

/// `ACTOR_OBJ_MAKEKINSUTA` (`actor_table.h`: 0x01CF).
pub const ACTOR_OBJ_MAKEKINSUTA: i16 = 0x01CF;
/// `OBJECT_GAMEPLAY_KEEP` (`object_table.h`: 0x0001).
pub const OBJECT: &str = "gameplay_keep";

/// `Obj_Makekinsuta_Profile`: `ACTOR_FLAG_UPDATE_CULLING_DISABLED`; no draw.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_OBJ_MAKEKINSUTA, name: "Obj_Makekinsuta", category: ACTORCAT_ITEMACTION, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `this->actor.params | 0x8000`: the whole params, the Gold Skulltula's (`En_Sw`).
pub const PARAMS: Field = Field::whole("params", Kind::Range(-0x8000, 0x7FFF), "the Gold Skulltula's params, spawned with 0x8000 set");
/// `PARAMS_GET_NOSHIFT(params, 13, 2) == 0x4000`.
pub const SOIL: Field = Field::params(
    "PARAMS_GET_NOSHIFT(params, 13, 2)",
    13,
    2,
    Kind::Named(&[(2, "0x4000: a Gold Skulltula soil (En_Sw's type 3, which jumps out)")]),
    "a Gold Skulltula soil; any other value only prints \"Invalid Argument\"",
)
.part_of("params");
/// `PARAMS_GET_S(params, 8, 5)` in `EnSw_Init`.
pub const GS_FLAGS_INDEX: Field =
    Field::params("PARAMS_GET_S(params, 8, 5)", 8, 5, Kind::Range(1, 31), "the Gold Skulltula's `gsFlags` index plus 1 (En_Sw takes 1 off)").part_of("params");
/// `PARAMS_GET_S(params, 0, 8)` in `EnSw_Init`.
pub const GS_FLAG: Field = Field::params("PARAMS_GET_S(params, 0, 8)", 0, 8, Kind::Range(0, 0xFF), "the Gold Skulltula's flag: its bit in `gsFlags`").part_of("params");

/// Obj_Makekinsuta's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[PARAMS, GS_FLAG, GS_FLAGS_INDEX, SOIL], flags: &[], todo: false, objects: &[], texts: &[] };

/// The frames `func_80B98320` counts once a bug is in.
pub const SPAWN_DELAY: i16 = 60;

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_80B98320`: waiting for a bug, then the count.
    Func80B98320,
    /// `ObjMakekinsuta_DoNothing`: the Gold Skulltula spawned.
    DoNothing,
}

pub struct ObjMakekinsuta {
    pub actor: Actor,
    pub action: Action,
    /// `timer`: frames since a bug came (0 while none).
    pub timer: i16,
    /// `unk_152`: a bug is in the soil. Written by `En_Insect` (`z_en_insect.c`): a released bug
    /// finds the nearest soil in its room within 80 (`EnInsect_TryFindNearbySoil`, its
    /// `soilActor`), walks to it and digs in (`EnInsect_Dig`); at the dig's end, within 8 of the
    /// soil, it sets `soilActor->unk_152 = 1`. The bugs aren't ported yet: nothing sets it but
    /// tests.
    pub unk_152: i16,
}

/// `func_8002DEEC` (`z_actor.c`): Link dead (`PLAYER_STATE1_DEAD`), in `PLAYER_STATE1_29`, or in
/// a cutscene action (`csAction != PLAYER_CSACTION_NONE`).
pub fn func_8002deec(play: &PlayState) -> bool {
    let Some(p) = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()) else { return false };
    p.state_flags1() & (STATE1_7 | STATE1_29) != 0 || p.cs_mode() != 0
}

impl ObjMakekinsuta {
    /// `ObjMakekinsuta_Init`.
    pub fn init(actor: Actor, _play: &mut PlayState) -> Box<dyn ActorImpl> {
        if SOIL.get(actor.params) == 2 {
            // schema: printed
            log::debug!("Obj_Makekinsuta: Gold Star Enemy(arg_data {:x})", actor.params);
        } else {
            // schema: printed
            log::warn!("Obj_Makekinsuta: Invalid Argument (arg_data {:x})(../z_obj_makekinsuta.c 119)", actor.params);
        }
        Box::new(ObjMakekinsuta { actor, action: Action::Func80B98320, timer: 0, unk_152: 0 })
    }

    /// `func_80B98320`.
    fn func_80b98320(&mut self, play: &mut PlayState) {
        if self.unk_152 != 0 {
            if self.timer >= SPAWN_DELAY && !func_8002deec(play) {
                let pos = self.actor.world_pos;
                let params = (PARAMS.get(self.actor.params) as u16 | 0x8000) as i16;
                if let Err(e) = play.actor_spawn(ACTOR_EN_SW, pos, [0, self.actor.shape_rot.y, 0], params) {
                    log::warn!("Obj_Makekinsuta: En_Sw not spawned: {e:?}");
                }
                self.action = Action::DoNothing;
            } else {
                self.timer = self.timer.wrapping_add(1);
            }
        } else {
            self.timer = 0;
        }
    }
}

impl ActorImpl for ObjMakekinsuta {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `ObjMakekinsuta_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Func80B98320 => self.func_80b98320(play),
            Action::DoNothing => {}
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
