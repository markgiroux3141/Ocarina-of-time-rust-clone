//! `Bg_Ddan_Jd` (`ovl_Bg_Ddan_Jd/z_bg_ddan_jd.c`): the rising stone pillars of Dodongo's Cavern,
//! DynaPoly actors of `object_ddan_objects` (`gDodongoRisingPlatformCol`, `DYNA_TRANSFORM_POS`:
//! they carry Link) drawn with `gDodongoRisingPlatformDL`.
//!
//! A pillar idles 100 frames (`BgDdanJd_Idle`), then moves (`BgDdanJd_Move`) at its speed
//! (`ySpeed`, 1) between its home and 140 above it (`MOVE_HEIGHT_MIDDLE`): up, idle, down, idle,
//! by its `state` (`BgDdanJdState`). Arrived, `NA_SE_EV_PILLAR_MOVE_STOP`. While it moves, dust
//! rises at its base (`BgDdanJd_MoveEffects`: two puffs a frame, `func_80033480`, 65 out on x on
//! odd `gameplayFrames`, on z on even ones, at its home's height).
//!
//! **The shortcut:** with its switch flag set at init it's the shortcut already (speed 5); set
//! while it idles or moves (only for a flag under 0x40), it jumps to 140 up, speed 5, and starts
//! one-point cutscene 3060 (for its `-99`, the cutscene's own length) on itself. As the shortcut
//! it goes from the middle up to 700 above its home (`MOVE_HEIGHT_TOP`) and back to the middle,
//! never to the bottom, with `NA_SE_EV_ELEVATOR_MOVE` while it moves. Master Quest's lobby has
//! three: 0x000A (switch 0x0A, under `Obj_Bombiwa` 0x000E's boulder) is the shortcut to the upper
//! level; the two 0x00FF are plain pillars.
//!
//! The whole overlay is ported. @bug (game) (the C's own comment): the init reads the flag for any
//! params, the idle and the move only for params under 0x40, so a pillar of params 0x40 or more
//! becomes the shortcut only once it's spawned again. And its params read as a switch flag whole:
//! 0xFF reads temp switch flag 0x3F (`Flags_GetSwitch`'s shift masked to 5 bits on the console,
//! `1 << (0xFF - 0x20)` is `1 << 0x1F`), so 0x00FF becomes the shortcut at init if 0x3F is set.
//!
//! Params: `switchFlag` the whole params (`Flags_GetSwitch(play, params)`: set, the shortcut; under
//! 0x40 also watched while it runs; 0xFF reads 0x3F through the console's masked shift, and isn't
//! watched).
//! Flags: reads switch flag `switchFlag`.

use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource, DYNA_TRANSFORM_POS};
use eng_math::step_to_f;
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile, audio_play_actor_sfx2, func_80033480};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::SFX_FLAG;
use oot_game::camera::CAM_ID_MAIN;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

/// `ACTOR_BG_DDAN_JD` (`actor_table.h`: 0x0058).
pub const ACTOR_BG_DDAN_JD: i16 = 0x0058;
/// `OBJECT_DDAN_OBJECTS` (`object_table.h`: 0x002B).
pub const OBJECT: &str = "object_ddan_objects";
/// `gDodongoRisingPlatformCol`, `gDodongoRisingPlatformDL` (`object_ddan_objects.xml`).
pub const COLLISION: &str = "gDodongoRisingPlatformCol";
pub const DL: &str = "gDodongoRisingPlatformDL";

/// `NA_SE_EV_ELEVATOR_MOVE` (`environmentbank_table.h`: 0x2824).
pub const NA_SE_EV_ELEVATOR_MOVE: u16 = 0x2824;
/// `NA_SE_EV_PILLAR_MOVE_STOP` (`environmentbank_table.h`: 0x28F1).
pub const NA_SE_EV_PILLAR_MOVE_STOP: u16 = 0x28F1;

/// The one-point cutscene of the shortcut rising (`OnePointCutscene_Init(play, 3060, -99, ..)`).
pub const ONEPOINT_SHORTCUT: i16 = 3060;

/// `Bg_Ddan_Jd_Profile`: `ACTORCAT_BG`, `ACTOR_FLAG_UPDATE_CULLING_DISABLED |
/// ACTOR_FLAG_DRAW_CULLING_DISABLED`, `OBJECT_DDAN_OBJECTS`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_BG_DDAN_JD,
    name: "Bg_Ddan_Jd",
    category: ACTORCAT_BG,
    flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `this->dyna.actor.params` whole, read as a switch flag.
pub const SWITCH_FLAG: Field = Field::whole(
    "switchFlag",
    Kind::SwitchFlag,
    "set: the shortcut (to 700 up, speed 5); under 0x40 also watched while it runs (0xFF reads flag 0x3F through the console's masked shift, and isn't watched)",
);

/// Bg_Ddan_Jd's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[SWITCH_FLAG],
    flags: &[FlagUse::new(Flag::Switch("switchFlag")).reads().note("set (at init, or while it runs for a flag under 0x40): the shortcut, one-point 3060")],
    todo: false,
};

/// `BgDdanJdState`.
pub const STATE_GO_BOTTOM: u8 = 0;
pub const STATE_GO_MIDDLE_FROM_BOTTOM: u8 = 1;
pub const STATE_GO_MIDDLE_FROM_TOP: u8 = 2;
pub const STATE_GO_TOP: u8 = 3;

/// `MOVE_HEIGHT_MIDDLE`, `MOVE_HEIGHT_TOP`.
pub const MOVE_HEIGHT_MIDDLE: f32 = 140.0;
pub const MOVE_HEIGHT_TOP: f32 = 700.0;
/// `IDLE_FRAMES`.
pub const IDLE_FRAMES: i16 = 100;
/// `DEFAULT_Y_SPEED`, `SHORTCUT_Y_SPEED`.
pub const DEFAULT_Y_SPEED: u8 = 1;
pub const SHORTCUT_Y_SPEED: u8 = 5;

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `BgDdanJd_Idle`.
    Idle,
    /// `BgDdanJd_Move`.
    Move,
}

pub struct BgDdanJd {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub action: Action,
    /// `state` (`BgDdanJdState`).
    pub state: u8,
    /// `ySpeed`: 1, or 5 as the shortcut.
    pub y_speed: u8,
    pub idle_timer: i16,
    pub target_y: f32,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

impl BgDdanJd {
    /// `BgDdanJd_Init`: scale 0.1 (`sInitChain`), the collision (`DYNA_TRANSFORM_POS`), 100
    /// frames to idle at the bottom; the shortcut's speed if its flag is set (any params: the C's
    /// missing check), idling.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: ICHAIN_VEC3F_DIV1000(scale, 100).
        actor.scale = Vec3::splat(0.1);
        let bg = match crate::obj_kibako2::load_collision(play, OBJECT, COLLISION) {
            Some(h) => play.col.dyna.set_bg_actor(h, source(&actor), DYNA_TRANSFORM_POS),
            None => BG_ACTOR_MAX,
        };
        let mut this = BgDdanJd { actor, bg, action: Action::Idle, state: STATE_GO_BOTTOM, y_speed: DEFAULT_Y_SPEED, idle_timer: IDLE_FRAMES, target_y: 0.0 };
        // @bug (game) (the C's comment): no check for params < 0x40 here.
        this.y_speed = if play.flags.get_switch(this.switch_flag() as i32) { SHORTCUT_Y_SPEED } else { DEFAULT_Y_SPEED };
        this.action = Action::Idle;
        Box::new(this)
    }

    /// `this->dyna.actor.params`: the switch flag.
    pub fn switch_flag(&self) -> i16 {
        SWITCH_FLAG.get(self.actor.params)
    }

    /// The switch flag just set on a plain pillar of a flag under 0x40 (`BgDdanJd_Idle`'s and
    /// `BgDdanJd_Move`'s check).
    fn shortcut_turned_on(&self, play: &PlayState) -> bool {
        self.y_speed == DEFAULT_Y_SPEED && self.switch_flag() < 0x40 && play.flags.get_switch(self.switch_flag() as i32)
    }

    /// The shortcut's start: speed 5, 140 up at once, rising from the middle with no wait,
    /// one-point 3060 (`-99`) on it.
    fn start_shortcut(&mut self, play: &mut PlayState) {
        self.y_speed = SHORTCUT_Y_SPEED;
        self.state = STATE_GO_MIDDLE_FROM_BOTTOM;
        self.idle_timer = 0;
        self.actor.world_pos.y = self.actor.home_pos.y + MOVE_HEIGHT_MIDDLE;
        let me = play.cur_actor.map(|h| play.cam_actor_of(h, &self.actor));
        play.onepoint_cutscene_init(ONEPOINT_SHORTCUT, -99, me, CAM_ID_MAIN);
    }

    /// `BgDdanJd_Idle`: the timer down; the flag just set, the shortcut starts; with the timer
    /// out, 100 again and the next leg: bottom to the middle, the middle to the top (the
    /// shortcut) or the bottom, the top to the middle; moving.
    fn idle(&mut self, play: &mut PlayState) {
        if self.idle_timer != 0 {
            self.idle_timer -= 1;
        }
        // If this is the platform that rises all the way to the top, and the switch state has
        // just changed to on.
        if self.shortcut_turned_on(play) {
            self.start_shortcut(play);
        }
        if self.idle_timer == 0 {
            self.idle_timer = IDLE_FRAMES;
            let home_y = self.actor.home_pos.y;
            if self.state == STATE_GO_BOTTOM {
                self.state = STATE_GO_MIDDLE_FROM_BOTTOM;
                self.target_y = home_y + MOVE_HEIGHT_MIDDLE;
            } else if self.state == STATE_GO_MIDDLE_FROM_BOTTOM || self.state == STATE_GO_MIDDLE_FROM_TOP {
                // If the platform has been activated as a shortcut.
                if self.y_speed != DEFAULT_Y_SPEED {
                    self.state = STATE_GO_TOP;
                    self.target_y = home_y + MOVE_HEIGHT_TOP;
                } else {
                    self.state = STATE_GO_BOTTOM;
                    self.target_y = home_y;
                }
            } else if self.state == STATE_GO_TOP {
                self.state = STATE_GO_MIDDLE_FROM_TOP;
                self.target_y = home_y + MOVE_HEIGHT_MIDDLE;
            }
            self.action = Action::Move;
        }
    }

    /// `BgDdanJd_MoveEffects`: dust at its base (its home's height), two puffs: 65 out either
    /// side on x, anywhere 110 along z, on odd `gameplayFrames`; on z, along x, on even ones;
    /// `NA_SE_EV_ELEVATOR_MOVE` as the shortcut.
    fn move_effects(&mut self, play: &mut PlayState) {
        let mut dust_pos = Vec3::ZERO;
        // Generate random dust particles at the platform's base.
        dust_pos.y = self.actor.home_pos.y;
        let p = self.actor.world_pos;
        if play.gameplay_frames & 1 != 0 {
            dust_pos.x = p.x + 65.0;
            dust_pos.z = play.rand.centered_float(110.0) + p.z;
            func_80033480(play, dust_pos, 5.0, 1, 20, 60, 1);
            dust_pos.x = p.x - 65.0;
            dust_pos.z = play.rand.centered_float(110.0) + p.z;
            func_80033480(play, dust_pos, 5.0, 1, 20, 60, 1);
        } else {
            dust_pos.x = play.rand.centered_float(110.0) + p.x;
            dust_pos.z = p.z + 65.0;
            func_80033480(play, dust_pos, 5.0, 1, 20, 60, 1);
            dust_pos.x = play.rand.centered_float(110.0) + p.x;
            dust_pos.z = p.z - 65.0;
            func_80033480(play, dust_pos, 5.0, 1, 20, 60, 1);
        }
        if self.y_speed == SHORTCUT_Y_SPEED {
            self.actor.play_sfx_flagged(NA_SE_EV_ELEVATOR_MOVE - SFX_FLAG);
        }
    }

    /// `BgDdanJd_Move`: the flag just set, the shortcut starts (idling, no wait); else a step of
    /// `ySpeed` to the target, arriving with `NA_SE_EV_PILLAR_MOVE_STOP` to idle. The dust either
    /// way.
    fn move_(&mut self, play: &mut PlayState) {
        // If this is the platform that rises all the way to the top, and the switch state has
        // just changed to on.
        if self.shortcut_turned_on(play) {
            self.y_speed = SHORTCUT_Y_SPEED;
            self.state = STATE_GO_MIDDLE_FROM_BOTTOM;
            self.actor.world_pos.y = self.actor.home_pos.y + MOVE_HEIGHT_MIDDLE;
            self.idle_timer = 0;
            self.action = Action::Idle;
            let me = play.cur_actor.map(|h| play.cam_actor_of(h, &self.actor));
            play.onepoint_cutscene_init(ONEPOINT_SHORTCUT, -99, me, CAM_ID_MAIN);
        } else if step_to_f(&mut self.actor.world_pos.y, self.target_y, self.y_speed as f32) {
            audio_play_actor_sfx2(play, NA_SE_EV_PILLAR_MOVE_STOP);
            self.action = Action::Idle;
        }
        self.move_effects(play);
    }
}

impl ActorImpl for BgDdanJd {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgDdanJd_Update`: the action; the new transform for `DynaPoly_UpdateContext`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Idle => self.idle(play),
            Action::Move => self.move_(play),
        }
        play.col.dyna.set_source(self.bg, source(&self.actor));
    }

    /// `BgDdanJd_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }

    fn dyna_bg_id(&self) -> Option<u16> {
        Some(self.bg)
    }

    /// `BgDdanJd_Draw`: `Gfx_DrawDListOpa(gDodongoRisingPlatformDL)`.
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
