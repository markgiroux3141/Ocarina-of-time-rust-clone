//! `Bg_Spot09_Obj` (`ovl_Bg_Spot09_Obj/z_bg_spot09_obj.c`): Gerudo Valley's bridges and the
//! carpenters' tent, DynaPoly actors of `object_spot09_obj`. Its params' low byte picks one
//! (`BgSpot09ObjType`): the bridge's sides (in the cutscene layers only, with no collision), the
//! broken bridge (adult, before the carpenters are rescued), the child's bridge, the tent (adult,
//! at a tenth of its size, its entrance translucent) and the repaired bridge (adult, after).
//!
//! Ported whole (GAME-06 milestone 1b): `func_808B1C70`'s three checks (`func_808B1AE0` whether
//! this one is here, `func_808B1BA0` its scale, `func_808B1BEC` its collision), then its culling
//! volume (`func_808B1D44`: not used, as for every actor).

use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use eng_gfx::{DrawCmd, MeshKey};
use glam::Vec3;
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile};
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};

/// `ACTOR_BG_SPOT09_OBJ` (`actor_table.h`: 0x00B8).
pub const ACTOR_BG_SPOT09_OBJ: i16 = 0x00B8;
pub const OBJECT: &str = "object_spot09_obj";

/// `Bg_Spot09_Obj_Profile`: `FLAGS` 0.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_SPOT09_OBJ, name: "Bg_Spot09_Obj", category: ACTORCAT_BG, flags: 0, object: OBJECT };

/// `BgSpot09ObjType`.
pub const BG_SPOT09_OBJ_BRIDGE_SIDES: i16 = 0;
pub const BG_SPOT09_OBJ_BRIDGE_BROKEN: i16 = 1;
pub const BG_SPOT09_OBJ_BRIDGE_CHILD: i16 = 2;
pub const BG_SPOT09_OBJ_TENT: i16 = 3;
pub const BG_SPOT09_OBJ_BRIDGE_REPAIRED: i16 = 4;
pub const BG_SPOT09_OBJ_MAX: i16 = 5;

/// `D_808B1F90`: each type's collision.
const D_808B1F90: [Option<&str>; 5] = [None, Some("gValleyObjects1Col"), Some("gValleyObjects2Col"), Some("gValleyObjects3Col"), Some("gValleyObjects4Col")];
/// `sDLists`.
const S_DLISTS: [&str; 5] = ["gValleyBridgeSidesDL", "gValleyBrokenBridgeDL", "gValleyBridgeChildDL", "gCarpentersTentDL", "gValleyRepairedBridgeDL"];

/// `EVENTCHKINF_CARPENTER_0_RESCUED` to `_3_RESCUED` (`save.h`: 0x90 to 0x93).
const EVENTCHKINF_CARPENTERS_RESCUED: [u16; 4] = [0x90, 0x91, 0x92, 0x93];

pub struct BgSpot09Obj {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

/// `GET_EVENTCHKINF_CARPENTERS_ALL_RESCUED()`.
fn carpenters_all_rescued(play: &PlayState) -> bool {
    EVENTCHKINF_CARPENTERS_RESCUED.iter().all(|&f| play.save.get_event_chk_inf(f))
}

impl BgSpot09Obj {
    /// `func_808B1AE0`: whether this one is here: in the cutscene layers the sides only; as an
    /// adult the tent and the bridge the carpenters' rescue picks; as a child the child's bridge.
    fn func_808b1ae0(&self, play: &PlayState) -> bool {
        let params = self.actor.params;
        if play.save.scene_layer >= 4 {
            return params == BG_SPOT09_OBJ_BRIDGE_SIDES;
        }
        let rescued = carpenters_all_rescued(play);
        if play.save.adult {
            match params {
                BG_SPOT09_OBJ_BRIDGE_SIDES => false,
                BG_SPOT09_OBJ_BRIDGE_BROKEN => !rescued,
                BG_SPOT09_OBJ_BRIDGE_REPAIRED => rescued,
                BG_SPOT09_OBJ_TENT => true,
                _ => false,
            }
        } else {
            params == BG_SPOT09_OBJ_BRIDGE_CHILD
        }
    }

    /// `func_808B1BA0`: the tent at 0.1, the rest at 1.
    fn func_808b1ba0(&mut self) -> bool {
        self.actor.scale = Vec3::splat(if self.actor.params == BG_SPOT09_OBJ_TENT { 0.1 } else { 1.0 });
        true
    }

    /// `func_808B1BEC`: `DynaPolyActor_Init(0)` and the type's collision, if it has one.
    fn func_808b1bec(&mut self, play: &mut PlayState) -> bool {
        if let Some(col) = D_808B1F90.get(self.actor.params as usize).copied().flatten() {
            self.bg = match crate::obj_kibako2::load_collision(play, OBJECT, col) {
                Some(h) => play.col.dyna.set_bg_actor(h, source(&self.actor), 0),
                None => BG_ACTOR_MAX,
            };
        }
        true
    }

    /// `func_808B1C70`: `D_808B1FA4`'s checks in order (`func_808B1BEC`, `func_808B1AE0`,
    /// `func_808B1BA0`), false at the first that fails.
    fn func_808b1c70(&mut self, play: &mut PlayState) -> bool {
        // @bug (game): the collision is set before the check that kills the actor; the kill's
        // Destroy takes it away again (but the sides', which have none).
        self.func_808b1bec(play) && self.func_808b1ae0(play) && self.func_808b1ba0()
    }

    /// `BgSpot09Obj_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        log::debug!("Spot09 Object [arg_data : {:#06x}]", actor.params);
        actor.params &= 0xFF;
        if actor.params < 0 || actor.params >= BG_SPOT09_OBJ_MAX {
            log::error!("Error : Spot 09 object arg_data cannot be determined (arg_data {:#06x})", actor.params);
        }
        let mut this = BgSpot09Obj { actor, bg: BG_ACTOR_MAX };
        // func_808B1D44 (the culling volume) always returns true.
        if !this.func_808b1c70(play) {
            this.actor.kill();
        }
        Box::new(this)
    }
}

impl ActorImpl for BgSpot09Obj {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgSpot09Obj_Update`: nothing.
    fn update(&mut self, _play: &mut PlayState) {}

    /// `BgSpot09Obj_Destroy`: the collision taken away (but the sides').
    fn destroy(&mut self, play: &mut PlayState) {
        if self.actor.params != BG_SPOT09_OBJ_BRIDGE_SIDES {
            play.col.dyna.delete_bg_actor(self.bg);
        }
    }

    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }

    /// `BgSpot09Obj_Draw`: `Gfx_DrawDListOpa` of its list; the tent's entrance translucent.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(dl) = S_DLISTS.get(self.actor.params as usize) else { return };
        let m = actor_draw_matrix(rs);
        out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh(OBJECT, dl)), m));
        if self.actor.params == BG_SPOT09_OBJ_TENT {
            out.xlu.push(DrawCmd::new(MeshKey::named(keys::mesh(OBJECT, "gCarpentersTentEntranceDL")), m));
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
