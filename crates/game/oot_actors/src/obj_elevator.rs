//! `Obj_Elevator` (`ovl_Obj_Elevator/z_obj_elevator.c`): the small stone lifts of Dodongo's
//! Cavern, DynaPoly actors of `object_d_elevator` (`object_d_elevator_Col_000360`,
//! `DYNA_TRANSFORM_POS`: they carry Link) drawn with `object_d_elevator_DL_000180`.
//!
//! It waits (`func_80B92C80`) until Link steps onto it (`DYNA_INTERACT_PLAYER_ON_TOP` this frame
//! and not the last, `unk_170`): at home (within 0.1) it heads for its rise (`unk_168`: 80 a step
//! of its `rise` above home), else back home. Then it moves (`func_80B92D44`:
//! `Math_SmoothStepToF` at most `unk_16C`, twice its `speed`, a frame) with
//! `NA_SE_EV_STONE_STATUE_OPEN`, and arrived (`NA_SE_EV_FOOT_SWITCH`) waits again. So each time
//! Link steps on it, it goes the other way; it doesn't come back down by itself.
//!
//! Master Quest's cavern places three: room 1's 0x0001 (small, rise 0, speed 0: it never moves,
//! the step-on only sounds the switch) and 0x1201 (small, 80 up at 4 a frame), room 2's 0x2200
//! (large, 160 up at 4 a frame).
//!
//! The whole overlay is ported (the debug build's `PRINTF`s logged).
//!
//! Params: `size` bit 0 (`PARAMS_GET_U(params, 0, 1)`: 0 large, scale 0.1; 1 small, 0.05);
//! `speed` bits 8 to 11 (`PARAMS_GET_U(params, 8, 4)`: twice it the most it moves a frame; 0
//! doesn't move); `rise` bits 12 to 15 (`PARAMS_GET_U(params, 12, 4)`: how far above its home it
//! goes, 80 a unit).
//! Flags: none.

use eng_collision::dyna::{
    BG_ACTOR_MAX, BgActorSource, DYNA_INTERACT_ACTOR_ON_TOP, DYNA_INTERACT_ACTOR_SWITCH_PRESSED, DYNA_INTERACT_PLAYER_ABOVE, DYNA_INTERACT_PLAYER_ON_TOP, DYNA_TRANSFORM_POS,
};
use eng_math::smooth_step_to_f;
use glam::Vec3;
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::audio::sfx::{NA_SE_EV_STONE_STATUE_OPEN, SFX_FLAG};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

/// `ACTOR_OBJ_ELEVATOR` (`actor_table.h`: 0x012B).
pub const ACTOR_OBJ_ELEVATOR: i16 = 0x012B;
/// `OBJECT_D_ELEVATOR` (`object_table.h`: 0x011B).
pub const OBJECT: &str = "object_d_elevator";
/// `object_d_elevator_Col_000360`, `object_d_elevator_DL_000180` (`object_d_elevator.xml`).
pub const COLLISION: &str = "object_d_elevator_Col_000360";
pub const DL: &str = "object_d_elevator_DL_000180";

/// `NA_SE_EV_FOOT_SWITCH` (`environmentbank_table.h`: 0x2815).
pub const NA_SE_EV_FOOT_SWITCH: u16 = 0x2815;

/// `Obj_Elevator_Profile`: `ACTORCAT_BG`, `FLAGS` 0, `OBJECT_D_ELEVATOR`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_OBJ_ELEVATOR, name: "Obj_Elevator", category: ACTORCAT_BG, flags: 0, object: OBJECT, schema: &SCHEMA };

/// `PARAMS_GET_U(thisx->params, 0, 1)`: an index into `sScales`.
pub const SIZE: Field = Field::params("size", 0, 1, Kind::Named(&[(0, "large (scale 0.1)"), (1, "small (scale 0.05)")]), "the lift's size");
/// `PARAMS_GET_U(thisx->params, 8, 4)`: `unk_16C` is twice it.
pub const SPEED: Field = Field::params("speed", 8, 4, Kind::Range(0, 15), "twice it the most it moves a frame (0: it doesn't move)");
/// `PARAMS_GET_U(thisx->params, 12, 4)`: the rise, in 80s.
pub const RISE: Field = Field::params("rise", 12, 4, Kind::Range(0, 15), "how far above its home it goes, 80 a unit");

/// Obj_Elevator's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[SIZE, SPEED, RISE], flags: &[], todo: false, objects: &[] };

/// `sScales`, by size.
pub const SCALES: [f32; 2] = [0.1, 0.05];

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_80B92C80`: waiting for Link to step on.
    Wait,
    /// `func_80B92D44`: moving to `unk_168`.
    Move,
}

pub struct ObjElevator {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub action: Action,
    /// `unk_168`: where it's going.
    pub unk_168: f32,
    /// `unk_16C`: the most it moves a frame.
    pub unk_16c: f32,
    /// `unk_170`: last frame's `dyna.interactFlags`.
    pub unk_170: u8,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

// shared: dyna.interactFlags (the whole byte, `DynaPolyActor`'s field)
/// Bg actor `bg`'s `interactFlags`, from its four `DYNA_INTERACT_*` bits.
fn interact_flags(play: &PlayState, bg: u16) -> u8 {
    [DYNA_INTERACT_ACTOR_ON_TOP, DYNA_INTERACT_PLAYER_ON_TOP, DYNA_INTERACT_PLAYER_ABOVE, DYNA_INTERACT_ACTOR_SWITCH_PRESSED]
        .into_iter()
        .filter(|&f| play.col.dyna.interact_flag(bg, f))
        .fold(0, |a, f| a | f)
}

impl ObjElevator {
    /// `ObjElevator_Init`: the collision (`func_80B92B08`, `DYNA_TRANSFORM_POS`), the size's
    /// scale, the culling volume (`sInitChain`), twice the speed, waiting.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let bg = Self::func_80b92b08(&actor, play, COLLISION, DYNA_TRANSFORM_POS);
        // Actor_SetScale(thisx, sScales[PARAMS_GET_U(thisx->params, 0, 1)]).
        actor.scale = Vec3::splat(SCALES[SIZE.get(actor.params) as usize]);
        // sInitChain: cullingVolumeDistance 2000, cullingVolumeScale 600, cullingVolumeDownward
        // 2000.
        actor.culling_volume_distance = 2000.0;
        actor.culling_volume_scale = 600.0;
        actor.culling_volume_downward = 2000.0;
        let temp_f0 = SPEED.get(actor.params) as f32;
        let mut this = ObjElevator { actor, bg, action: Action::Wait, unk_168: 0.0, unk_16c: temp_f0 + temp_f0, unk_170: 0 };
        this.func_80b92c5c();
        // schema: the whole params printed.
        log::debug!("(Dungeon Elevator)(arg_data 0x{:04x})", this.actor.params);
        play.col.dyna.set_source(this.bg, source(&this.actor));
        Box::new(this)
    }

    /// `func_80B92B08`: `DynaPolyActor_Init(flag)` and the bg actor; the debug build's warning when
    /// there's no room for it.
    fn func_80b92b08(actor: &Actor, play: &mut PlayState, collision: &str, flag: u32) -> u16 {
        let bg = match crate::obj_kibako2::load_collision(play, OBJECT, collision) {
            Some(h) => play.col.dyna.set_bg_actor(h, source(actor), flag),
            None => BG_ACTOR_MAX,
        };
        if bg == BG_ACTOR_MAX {
            // schema: the whole params printed.
            log::warn!("Warning : move BG registration failed (../z_obj_elevator.c 136)(name {})(arg_data 0x{:04x})", actor.id, actor.params);
        }
        bg
    }

    /// `func_80B92C5C`: waiting.
    fn func_80b92c5c(&mut self) {
        self.action = Action::Wait;
    }

    /// `func_80B92C80`: Link just stepped on: at home (within 0.1), to its rise (80 a unit) above
    /// it, else home; moving.
    fn func_80b92c80(&mut self, play: &mut PlayState) {
        if play.col.dyna.interact_flag(self.bg, DYNA_INTERACT_PLAYER_ON_TOP) && self.unk_170 & DYNA_INTERACT_PLAYER_ON_TOP == 0 {
            let a = &self.actor;
            let sub = a.world_pos.y - a.home_pos.y;
            if sub.abs() < 0.1 {
                self.unk_168 = a.home_pos.y + RISE.get(a.params) as f32 * 80.0;
            } else {
                self.unk_168 = a.home_pos.y;
            }
            self.func_80b92d20();
        }
    }

    /// `func_80B92D20`: moving.
    fn func_80b92d20(&mut self) {
        self.action = Action::Move;
    }

    /// `func_80B92D44`: a step towards `unk_168` (`Math_SmoothStepToF`, all of the gap, at most
    /// `unk_16C`); arrived, `NA_SE_EV_FOOT_SWITCH` and waiting; else `NA_SE_EV_STONE_STATUE_OPEN`.
    fn func_80b92d44(&mut self, play: &mut PlayState) {
        if smooth_step_to_f(&mut self.actor.world_pos.y, self.unk_168, 1.0, self.unk_16c, 0.0).abs() < 0.001 {
            audio_play_actor_sfx2(play, NA_SE_EV_FOOT_SWITCH);
            self.func_80b92c5c();
        } else {
            audio_play_actor_sfx2(play, NA_SE_EV_STONE_STATUE_OPEN - SFX_FLAG);
        }
    }
}

impl ActorImpl for ObjElevator {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `ObjElevator_Update`: the action, then this frame's `interactFlags` kept (`unk_170`); the
    /// new transform for `DynaPoly_UpdateContext`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Wait => self.func_80b92c80(play),
            Action::Move => self.func_80b92d44(play),
        }
        self.unk_170 = interact_flags(play, self.bg);
        play.col.dyna.set_source(self.bg, source(&self.actor));
    }

    /// `ObjElevator_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }

    /// `DynaPolyActor_IsPlayerOnTop` reads the interact flags `Actor_UpdateAll` clears after it.
    fn dyna_bg_id(&self) -> Option<u16> {
        Some(self.bg)
    }

    /// `ObjElevator_Draw`: `Gfx_DrawDListOpa(object_d_elevator_DL_000180)`.
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
