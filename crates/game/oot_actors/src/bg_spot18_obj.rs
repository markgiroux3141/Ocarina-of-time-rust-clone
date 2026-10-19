//! `Bg_Spot18_Obj` (`ovl_Bg_Spot18_Obj/z_bg_spot18_obj.c`): the Goron statue in Darunia's room
//! (type 0) and the spear the child's statue holds (type 1), DynaPoly actors of
//! `object_spot18_obj` (`gGoronCityStatueCol` / `gGoronCityStatueSpearCol`, drawn with
//! `gGoronCityStatueDL` / `gGoronCityStatueSpearDL`) at a tenth of their size.
//!
//! Its init runs `D_808B910C`'s four checks in order (`func_808B8B38`), killed at the first that
//! fails: the collision (`func_808B8A98`), whether this one is here for Link's age
//! (`func_808B8910`, `D_808B90F0[type][age]`, age 0 a child and 1 an adult: the statue for both,
//! the spear for a child only), the scale 0.1 (`func_808B8A5C`), the flags (`func_808B8B08`, none
//! added). Then the type's own setup (`func_808B8CC8`):
//! - **the statue** (`func_808B8BB4`, `sInitChain1`: gravity -4, `minVelocityY` -10): for a child
//!   it stays (`func_808B9040`: a push on it dropped); for an adult with its switch flag set it
//!   starts 80 along its yaw from home; else it waits to be pulled (`func_808B8E7C`: the push held
//!   at under -0.001 for 20 frames, Player's `PLAYER_STATE2_4` cleared each frame), then slides
//!   (`func_808B8F08`) along +z (`world.rot.y` 0) at up to 1.2 (0.1 more a frame) under gravity,
//!   `NA_SE_EV_ROCK_SLIDE` each frame; 80 from home it stops at home + 80 along its yaw, sets the
//!   switch flag, the chime (`NA_SE_SY_CORRECT_CHIME`) and `NA_SE_EV_BLOCK_BOUND`.
//! - **the spear** (`func_808B8C90`, `sInitChain2`): nothing (`func_808B8DD0`).
//!
//! The whole overlay is ported. @bug (game): a type of 2 or more reads past `D_808B90FC` (the
//! collision: here none) and `D_808B90F0` (the bytes of `D_808B90F4`'s 0.1f: 0x3D, 0xCC, 0xCD,
//! not 0 to 2), so its age check prints its error and it's killed; none is placed.
//!
//! Params: `type` bits 0 to 3 (0: the statue; 1: the child's spear); `switchFlag` bits 8 to 13
//! (the statue: set, an adult's statue has been pulled aside; set by the pull; 0x3F in the child
//! layers, 0x11 in the adult's); bits 14 and 15 unused (3 in the child layers' statue, 0xFF00).
//! The adult's statue is placed at yaw 0, the child's at -182 (its pulled place, never used by a
//! child, would be off its +z slide's end by `sin(-182) * 80`, -0.44 in x).
//! Flags: reads and sets the switch flag `switchFlag` (the statue, an adult).

use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use eng_math::{cos_s, sin_s, step_to_f};
use glam::Vec3;
use oot_game::actor::{Actor, UPDBGCHECKINFO_FLAG_2, UPDBGCHECKINFO_FLAG_3, UPDBGCHECKINFO_FLAG_4};
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::{NA_SE_SY_CORRECT_CHIME, SFX_FLAG};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

use crate::obj_oshihiki::{NA_SE_EV_BLOCK_BOUND, NA_SE_EV_ROCK_SLIDE};

/// `ACTOR_BG_SPOT18_OBJ` (`actor_table.h`: 0x00B9).
pub const ACTOR_BG_SPOT18_OBJ: i16 = 0x00B9;
/// `OBJECT_SPOT18_OBJ` (`object_table.h`: 0x00AF).
pub const OBJECT: &str = "object_spot18_obj";

/// `Bg_Spot18_Obj_Profile`: `ACTORCAT_BG`, `FLAGS` 0.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_SPOT18_OBJ, name: "Bg_Spot18_Obj", category: ACTORCAT_BG, flags: 0, object: OBJECT, schema: &SCHEMA };

/// `PARAMS_GET_U(this->dyna.actor.params, 0, 4)`: which one.
pub const TYPE: Field = Field::params(
    "type",
    0,
    4,
    Kind::Named(&[(0, "the Goron statue (a child's: still; an adult's: pulled aside)"), (1, "the statue's spear (a child's only)")]),
    "the statue or its spear",
);
/// `PARAMS_GET_U(this->dyna.actor.params, 8, 6)`: the statue's switch flag.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 8, 6, Kind::SwitchFlag, "the statue, an adult: set, it starts pulled aside; set by the pull");
/// Bits 14 and 15: 3 in the child layers' statue (0xFF00), read by nothing.
pub const BITS_14_15: Field = Field::params("params bits 14 and 15", 14, 2, Kind::Unused, "3 in the child layers' statue (0xFF00); read by nothing");

/// Bg_Spot18_Obj's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, SWITCH_FLAG, BITS_14_15],
    flags: &[FlagUse::new(Flag::Switch("switchFlag")).reads().sets().note("the statue, an adult: set, pulled aside (starts there); set as the pull ends")],
    todo: false,
    objects: &[],
    texts: &[],
};

/// `D_808B90F0[type][age]` (age 0 a child, 1 an adult): whether it's here.
const D_808B90F0: [[u8; 2]; 2] = [[0x01, 0x01], [0x01, 0x00]];
/// `D_808B90F4`: each type's scale.
const D_808B90F4: [f32; 2] = [0.1, 0.1];
/// `D_808B90FC`: each type's collision.
const D_808B90FC: [&str; 2] = ["gGoronCityStatueCol", "gGoronCityStatueSpearCol"];
/// `D_808B9104`: each type's added actor flags.
const D_808B9104: [u32; 2] = [0, 0];
/// `sDlists`.
const S_DLISTS: [&str; 2] = ["gGoronCityStatueDL", "gGoronCityStatueSpearDL"];

/// How far the statue slides (`func_808B8F08`), and its speed.
pub const PULL: f32 = 80.0;
pub const PULL_SPEED: f32 = 1.2;

/// `PLAYER_STATE2_4` (`player.h`): pushing or pulling.
const PLAYER_STATE2_4: u32 = 1 << 4;

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_808B8DD0`: the spear: nothing.
    Still,
    /// `func_808B8E7C`: the adult's statue waiting to be pulled.
    WaitPull,
    /// `func_808B8F08`: sliding.
    Slide,
    /// `func_808B9040`: in place for good; a push on it dropped.
    Rest,
}

pub struct BgSpot18Obj {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub action: Action,
    /// `unk_168`: the pull's wait.
    pub unk_168: i16,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

/// `player->stateFlags2 &= ~PLAYER_STATE2_4`.
fn clear_player_state2_4(play: &mut PlayState) {
    if let Some(p) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
        p.change_state_flags2(0, PLAYER_STATE2_4);
    }
}

impl BgSpot18Obj {
    fn ty(&self) -> usize {
        TYPE.get(self.actor.params) as usize
    }

    /// `func_808B8910`: whether this type is here for Link's age (`D_808B90F0`).
    fn func_808b8910(&self, play: &PlayState) -> bool {
        // LINK_AGE_IN_YEARS is YEARS_ADULT or YEARS_CHILD (the "age unknown" error can't happen).
        let age = if play.save.adult { 1 } else { 0 };
        let Some(row) = D_808B90F0.get(self.ty()) else {
            // @bug (game): past the table the C reads D_808B90F4's bytes (0x3D 0xCC, 0xCC 0xCD):
            // neither 0, 1 nor 2.
            log::debug!("Error : Obj appearance check failed (../z_bg_spot18_obj.c 210)(arg_data 0x{:04x})", self.actor.params as u16); // schema: printed
            return false;
        };
        match row[age] {
            0 | 1 => {
                if row[age] == 0 {
                    log::debug!("Non-appearing Object (0x{:04x})", self.actor.params as u16); // schema: printed
                }
                row[age] != 0
            }
            _ => false,
        }
    }

    /// `func_808B8A5C`: the type's scale.
    fn func_808b8a5c(&mut self) -> bool {
        if let Some(&s) = D_808B90F4.get(self.ty()) {
            self.actor.scale = Vec3::splat(s);
        }
        true
    }

    /// `func_808B8A98`: `DynaPolyActor_Init(0)` and the type's collision.
    fn func_808b8a98(&mut self, play: &mut PlayState) -> bool {
        // @bug (game): a type past D_808B90FC reads D_808B9104's 0 as the header: none here.
        if let Some(col) = D_808B90FC.get(self.ty())
            && let Some(h) = crate::obj_kibako2::load_collision(play, OBJECT, col)
        {
            self.bg = play.col.dyna.set_bg_actor(h, source(&self.actor), 0);
        }
        true
    }

    /// `func_808B8B08`: the type's added flags (none).
    fn func_808b8b08(&mut self) -> bool {
        self.actor.flags |= D_808B9104.get(self.ty()).copied().unwrap_or(0);
        true
    }

    /// `func_808B8B38`: `D_808B910C`'s checks in order (`func_808B8A98`, `func_808B8910`,
    /// `func_808B8A5C`, `func_808B8B08`), false at the first that fails.
    fn func_808b8b38(&mut self, play: &mut PlayState) -> bool {
        // The collision is set before the check that kills the actor; the kill's Destroy takes it
        // away again.
        self.func_808b8a98(play) && self.func_808b8910(play) && self.func_808b8a5c() && self.func_808b8b08()
    }

    /// `func_808B8BB4`: the statue. `sInitChain1`; a child's stays, an adult's starts pulled aside
    /// with the flag set, else waits for the pull.
    fn func_808b8bb4(&mut self, play: &PlayState) -> bool {
        // sInitChain1.
        self.actor.min_velocity_y = -10.0;
        self.actor.gravity = -4.0;
        self.actor.culling_volume_distance = 1400.0;
        self.actor.culling_volume_scale = 500.0;
        self.actor.culling_volume_downward = 800.0;
        if !play.save.adult {
            self.func_808b9030();
        } else if play.flags.get_switch(SWITCH_FLAG.get(self.actor.params) as i32) {
            self.func_808b9030();
            self.set_pulled_pos();
        } else {
            self.func_808b8e64();
        }
        true
    }

    /// The statue 80 along its yaw from home (`world.pos.x = sin(rot.y) * 80 + home.x`, z by cos).
    fn set_pulled_pos(&mut self) {
        let a = &mut self.actor;
        a.world_pos.x = (sin_s(a.world_rot.y) * PULL) + a.home_pos.x;
        a.world_pos.z = (cos_s(a.world_rot.y) * PULL) + a.home_pos.z;
    }

    /// `func_808B8C90`: the spear. `sInitChain2`, nothing to do.
    fn func_808b8c90(&mut self) -> bool {
        // sInitChain2.
        self.actor.culling_volume_distance = 1200.0;
        self.actor.culling_volume_scale = 500.0;
        self.actor.culling_volume_downward = 700.0;
        self.func_808b8dc0();
        true
    }

    /// `func_808B8CC8`: `D_808B913C[type]`'s setup.
    fn func_808b8cc8(&mut self, play: &PlayState) -> bool {
        match self.ty() {
            0 => self.func_808b8bb4(play),
            1 => self.func_808b8c90(),
            _ => true,
        }
    }

    /// `BgSpot18Obj_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = BgSpot18Obj { actor, bg: BG_ACTOR_MAX, action: Action::Still, unk_168: 0 };
        log::debug!("Spot18 Object [arg_data : 0x{:04x}]", this.actor.params as u16); // schema: printed
        if !this.func_808b8b38(play) || !this.func_808b8cc8(play) {
            this.actor.kill();
        }
        play.col.dyna.set_source(this.bg, source(&this.actor));
        Box::new(this)
    }

    /// `func_808B8DC0`.
    fn func_808b8dc0(&mut self) {
        self.action = Action::Still;
    }

    /// `func_808B8DDC`: `Actor_UpdateBgCheckInfo` (walls 20 high, 46 out; floor and water; flags 3
    /// and 4), its own collision skipped.
    fn func_808b8ddc(&mut self, play: &PlayState) {
        self.actor.update_bg_check_info_of(&play.col, 20.0, 46.0, 0.0, UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4, Some(self.bg));
    }

    /// `func_808B8E20`: a push or pull on it dropped (and Player's `PLAYER_STATE2_4`).
    fn func_808b8e20(&mut self, play: &mut PlayState) {
        if play.col.dyna.unk_150(self.bg).abs() > 0.001 {
            play.col.dyna.set_unk_150(self.bg, 0.0);
            clear_player_state2_4(play);
        }
    }

    /// `func_808B8E64`: waiting for the pull, 20 frames.
    fn func_808b8e64(&mut self) {
        self.unk_168 = 20;
        self.action = Action::WaitPull;
    }

    /// `func_808B8E7C`: pulled (`dyna.unk_150` under -0.001) for 20 frames: the slide; anything
    /// else resets the wait. The push is dropped every frame.
    fn wait_pull(&mut self, play: &mut PlayState) {
        if play.col.dyna.unk_150(self.bg) < -0.001 {
            if self.unk_168 <= 0 {
                self.func_808b8ee0();
            }
        } else {
            self.unk_168 = 20;
        }
        self.func_808b8e20(play);
    }

    /// `func_808B8EE0`: the slide from rest, along +z (`world.rot.y` 0).
    fn func_808b8ee0(&mut self) {
        self.action = Action::Slide;
        self.actor.speed_xz = 0.0;
        self.actor.velocity = Vec3::ZERO;
        self.actor.world_rot.y = 0;
    }

    /// `func_808B8F08`: up to 1.2 (0.1 more a frame) along its yaw under gravity; 80 from home it
    /// stops there, the push dropped, the switch flag set, the chime and `NA_SE_EV_BLOCK_BOUND`;
    /// on the way `NA_SE_EV_ROCK_SLIDE`.
    fn slide(&mut self, play: &mut PlayState) {
        step_to_f(&mut self.actor.speed_xz, PULL_SPEED, 0.1);
        // Actor_MoveXZGravity.
        self.actor.move_forward();
        self.func_808b8ddc(play);
        let (dx, dz) = (self.actor.world_pos.x - self.actor.home_pos.x, self.actor.world_pos.z - self.actor.home_pos.z);
        // Math3D_Dist2DSq(...) >= SQ(80.0f).
        if dx * dx + dz * dz >= PULL * PULL {
            self.func_808b9030();
            self.set_pulled_pos();
            play.col.dyna.set_unk_150(self.bg, 0.0);
            clear_player_state2_4(play);
            play.flags.set_switch(SWITCH_FLAG.get(self.actor.params) as i32);
            play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
            audio_play_actor_sfx2(play, NA_SE_EV_BLOCK_BOUND);
        } else {
            self.actor.play_sfx_flagged(NA_SE_EV_ROCK_SLIDE - SFX_FLAG);
        }
    }

    /// `func_808B9030`.
    fn func_808b9030(&mut self) {
        self.action = Action::Rest;
    }
}

impl ActorImpl for BgSpot18Obj {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `BgSpot18Obj_Update`: the wait down, then the action; the new transform for
    /// `DynaPoly_UpdateContext`.
    fn update(&mut self, play: &mut PlayState) {
        if self.unk_168 > 0 {
            self.unk_168 -= 1;
        }
        match self.action {
            Action::Still => {}
            Action::WaitPull => self.wait_pull(play),
            Action::Slide => self.slide(play),
            // func_808B9040.
            Action::Rest => self.func_808b8e20(play),
        }
        play.col.dyna.set_source(self.bg, source(&self.actor));
    }
    /// `BgSpot18Obj_Destroy`: `DynaPoly_DeleteBgActor`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }
    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }
    fn render_state(&self) -> RenderState {
        RenderState::of(&self.actor)
    }
    /// `BgSpot18Obj_Draw`: `Gfx_DrawDListOpa` of `sDlists[type]`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if let Some(dl) = S_DLISTS.get(self.ty()) {
            crate::gfx_draw_dlist_opa(out, OBJECT, dl, rs);
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
