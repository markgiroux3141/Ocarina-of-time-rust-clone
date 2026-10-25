//! `Bg_Ingate` (`ovl_Bg_Ingate/z_bg_ingate.c`): Ingo's gates, Lon Lon Ranch's wooden gates (the
//! corral's two and the two at the ranch's way out), a DynaPoly actor of `object_ingate`
//! (`DynaPolyActor_Init(0)`, `gIngoGateCol`) drawn with its display list (`gIngoGateDL`,
//! `Gfx_SetupDL_25Opa`).
//!
//! Only for an adult on the ranch, and with Epona had only on the cutscene layer (`CS_INDEX_0`);
//! otherwise killed. A gate with params bit 0, while Ingo has shut Link in the ranch after the race
//! for Epona (`INGO_RACE_STATE_TRAPPED_WIN_EPONA`), rewinds the cutscene's frame
//! (`csCtx.curFrame = 0`) and swings 90 degrees by it (`func_80892890`): from frame 10 by
//! `Math_SinS((frame - 10) * 400) * 0x4000` (a quarter turn by frame 49), a quarter turn at 50, then
//! stands. Params bit 1 turns it the other way (left with it, right without). Every other gate
//! stands as placed (`BgInGate_DoNothing`).
//!
//! Lon Lon Ranch places them on its adult day layer 2 (the corral's pair at (-302, 0, -1504) and
//! (-142, 0, -1504), params 0; the way out's at (820, -40, -2420) params 1 and (980, -40, -2420)
//! params 3, both turned 0x4000: they swing shut, towards each other), its night layer 3 and its
//! cutscene layer 4 (all params 0).
//!
//! The whole overlay is ported.
//!
//! Params: bit 0 swings shut when Ingo traps Link (`INGO_RACE_STATE_TRAPPED_WIN_EPONA`), bit 1 the
//! way it swings (`+0x4000` with it, `-0x4000` without); 0, 1 and 3 placed.
//! Flags: reads EVENTCHKINF_EPONA_OBTAINED (killed with it, unless on the cutscene layer
//! `CS_INDEX_0`) and `eventInf[EVENTINF_INDEX_HORSES]`'s race state (bit 0's swing).

use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use eng_math::sin_s;
use glam::Vec3;
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::event_chk_inf;
use oot_game::horse::{CS_INDEX_0, EVENTCHKINF_EPONA_OBTAINED, INGO_RACE_STATE_TRAPPED_WIN_EPONA, SCENE_LON_LON_RANCH, ingo_race_state};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

/// `ACTOR_BG_INGATE` (`actor_table.h`: 0x0140).
pub const ACTOR_BG_INGATE: i16 = 0x0140;
/// `OBJECT_INGATE` (`object_table.h`: 0x0129).
pub const OBJECT: &str = "object_ingate";
const COLLISION: &str = "gIngoGateCol";
const DL: &str = "gIngoGateDL";

/// `Bg_Ingate_Profile`: `ACTORCAT_PROP`, `FLAGS` 0.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_INGATE, name: "Bg_Ingate", category: ACTORCAT_PROP, flags: 0, object: OBJECT, schema: &SCHEMA };

/// `PARAMS_GET_U(params, 0, 1)`: swings shut when Ingo traps Link.
pub const SWINGS: Field = Field::params(
    "PARAMS_GET_U(params, 0, 1)",
    0,
    1,
    Kind::Named(&[(0, "stands as placed"), (1, "swings shut when Ingo traps Link in the ranch (INGO_RACE_STATE_TRAPPED_WIN_EPONA)")]),
    "whether it swings shut as Ingo traps Link after the race for Epona",
);
/// `PARAMS_GET_NOSHIFT(params, 1, 1)`: which way it swings.
pub const WAY: Field = Field::params(
    "PARAMS_GET_NOSHIFT(params, 1, 1)",
    1,
    1,
    Kind::Named(&[(0, "a quarter turn clockwise (-0x4000)"), (1, "a quarter turn anticlockwise (+0x4000)")]),
    "which way it swings (with bit 0)",
);

/// Bg_Ingate's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[SWINGS, WAY],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_EPONA_OBTAINED)).reads().note("killed with it, unless on the cutscene layer (CS_INDEX_0)"),
        FlagUse::new(Flag::Other("eventInf[EVENTINF_INDEX_HORSES]'s race state (EVENTINF_INGO_RACE_STATE_MASK)")).reads().note("INGO_RACE_STATE_TRAPPED_WIN_EPONA: bit 0's gates swing shut"),
    ],
    todo: false,
    objects: &[],
    texts: &[],
};

/// `BgInGateActionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_80892890`: swinging shut by the cutscene's frame.
    Swing,
    /// `BgInGate_DoNothing`.
    DoNothing,
}

pub struct BgInGate {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub action: Action,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

impl BgInGate {
    /// `BgInGate_Init`: `DynaPolyActor_Init(0)`, the gate's collision; killed but for an adult on
    /// the ranch (and with Epona had, but on the cutscene layer); scale 0.1; bit 0's gate swings
    /// while Ingo traps Link (the cutscene's frame rewound), else does nothing.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = BgInGate { actor, bg: BG_ACTOR_MAX, action: Action::DoNothing };
        if let Some(h) = crate::obj_kibako2::load_collision(play, OBJECT, COLLISION) {
            this.bg = play.col.dyna.set_bg_actor(h, source(&this.actor), 0);
        }
        if (play.scene_id != SCENE_LON_LON_RANCH || !play.save.adult) || (play.save.get_event_chk_inf(EVENTCHKINF_EPONA_OBTAINED) && play.save.cutscene_index != CS_INDEX_0) {
            this.actor.kill();
            return Box::new(this);
        }
        // Actor_SetScale(0.1).
        this.actor.scale = Vec3::splat(0.1);
        if SWINGS.get(this.actor.params) != 0 && ingo_race_state(&play.save) == INGO_RACE_STATE_TRAPPED_WIN_EPONA {
            play.cs_ctx.frames = 0;
            this.action = Action::Swing;
        } else {
            this.action = Action::DoNothing;
        }
        play.col.dyna.set_source(this.bg, source(&this.actor));
        Box::new(this)
    }

    /// `func_80892890`: from the cutscene's frame 50 a quarter turn (bit 1's way) and done; from
    /// frame 10 `Math_SinS((frame - 10) * 400) * 0x4000` of one.
    fn swing(&mut self, play: &mut PlayState) {
        let cur_frame = play.cs_ctx.frames as i32;
        if cur_frame >= 50 {
            let phi0: i32 = if WAY.get(self.actor.params) == 0 { -0x4000 } else { 0x4000 };
            self.actor.shape_rot.y = (self.actor.world_rot.y as i32 + phi0) as i16;
            self.action = Action::DoNothing;
        } else if cur_frame >= 10 {
            // s16 csFrames = (curFrame - 10) * 400, capped at 0x4000.
            let mut cs_frames = ((cur_frame - 10) as i16).wrapping_mul(400);
            if cs_frames > 0x4000 {
                cs_frames = 0x4000;
            }
            cs_frames = (sin_s(cs_frames) * 16384.0) as i16;
            let mut phi1 = cs_frames;
            if WAY.get(self.actor.params) == 0 {
                phi1 = phi1.wrapping_neg();
            }
            self.actor.shape_rot.y = self.actor.world_rot.y.wrapping_add(phi1);
        }
    }
}

impl ActorImpl for BgInGate {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgInGate_Update`: the action (the collision follows the gate's turn).
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Swing => self.swing(play),
            Action::DoNothing => {}
        }
        play.col.dyna.set_source(self.bg, source(&self.actor));
    }

    /// `BgInGate_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }

    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }

    /// `BgInGate_Draw`: `Gfx_SetupDL_25Opa`, `gIngoGateDL`.
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
