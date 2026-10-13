//! `Bg_Gate_Shutter` (`ovl_Bg_Gate_Shutter/z_bg_gate_shutter.c`): the gate to Death Mountain in
//! Kakariko, a DynaPoly actor of `object_spot01_matoyab` (`gKakarikoGuardGateCol`, drawn with
//! `gKakarikoGuardGateDL`). Its guard (`En_Heishi2` type 5) sets `openingState`: 1 (with
//! `INFTABLE_76` clear) or 2 slides it open, 2 a frame along -x to x -89 while z eases to -1375; a
//! negative one shuts it, 2 a frame along +x to 91 while z eases to -1350. Each ends with
//! `NA_SE_EV_BRIDGE_OPEN_STOP` and 30 frames before it listens again. Once the guard has taken
//! Zelda's letter (`INFTABLE_76`) or after the Master Sword it starts open, in Kakariko.
//!
//! The whole overlay is ported.

use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::approach_f;
use glam::Vec3;
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::audio::sfx::SFX_FLAG;
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

use crate::bg_spot15_saku::{NA_SE_EV_BRIDGE_OPEN_STOP, NA_SE_EV_METALGATE_OPEN};

/// `ACTOR_BG_GATE_SHUTTER` (`actor_table.h`: 0x0100).
pub const ACTOR_BG_GATE_SHUTTER: i16 = 0x0100;
pub const OBJECT: &str = "object_spot01_matoyab";

/// `Bg_Gate_Shutter_Profile`: `FLAGS` 0.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_GATE_SHUTTER, name: "Bg_Gate_Shutter", category: ACTORCAT_ITEMACTION, flags: 0, object: OBJECT };

/// `INFTABLE_76` (`save.h`: 0x76): the guard has taken Zelda's letter.
pub const INFTABLE_76: u16 = 0x76;
/// `EVENTCHKINF_OBTAINED_MASTER_SWORD` (`save.h`: 0x45).
const EVENTCHKINF_OBTAINED_MASTER_SWORD: u16 = 0x45;
/// `SCENE_KAKARIKO_VILLAGE` (`scene_table.h`: 0x52).
const SCENE_KAKARIKO_VILLAGE: u16 = 0x52;

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_8087828C`: waiting for the guard's call.
    Wait,
    /// `func_80878300`: opening.
    Open,
    /// `func_808783AC`: done, 30 frames.
    Done,
    /// `func_808783D4`: shutting.
    Shut,
}

pub struct BgGateShutter {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub action: Action,
    /// `openingState`: 1 or 2 open, negative shut (the guard's).
    pub opening_state: i16,
    /// `somePos`: where it was placed.
    pub some_pos: Vec3,
    pub unk_178: i16,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

impl BgGateShutter {
    /// `BgGateShutter_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = BgGateShutter { actor, bg: BG_ACTOR_MAX, action: Action::Wait, opening_state: 0, some_pos: Vec3::ZERO, unk_178: 0 };
        // DynaPolyActor_Init(&this->dyna, 0), the collision (registered with the placement's scale,
        // set to 1 after).
        if let Some(h) = crate::obj_kibako2::load_collision(play, OBJECT, "gKakarikoGuardGateCol") {
            this.bg = play.col.dyna.set_bg_actor(h, source(&this.actor), 0);
        }
        this.some_pos = this.actor.world_pos;
        if (play.save.get_inf_table(INFTABLE_76) || play.save.get_event_chk_inf(EVENTCHKINF_OBTAINED_MASTER_SWORD)) && play.scene_id == SCENE_KAKARIKO_VILLAGE {
            this.actor.world_pos.x = -89.0;
            this.actor.world_pos.z = -1375.0;
        }
        this.actor.scale = Vec3::ONE;
        log::debug!(" ☆☆☆☆☆ There's a fence ☆☆☆☆☆ ");
        play.col.dyna.set_source(this.bg, source(&this.actor));
        Box::new(this)
    }

    /// `func_8087828C`.
    fn wait(&mut self, play: &PlayState) {
        if self.opening_state == 1 && !play.save.get_inf_table(INFTABLE_76) {
            self.unk_178 = 2;
            self.action = Action::Open;
        } else if self.opening_state == 2 {
            self.unk_178 = 2;
            self.action = Action::Open;
        } else if self.opening_state < 0 {
            self.unk_178 = 2;
            self.action = Action::Shut;
        }
    }

    /// `func_80878300`.
    fn open(&mut self, play: &mut PlayState) {
        if self.unk_178 == 0 {
            audio_play_actor_sfx2(play, NA_SE_EV_METALGATE_OPEN - SFX_FLAG);
            self.actor.world_pos.x -= 2.0;
            approach_f(&mut self.actor.world_pos.z, -1375.0, 0.8, 0.3);
            if self.actor.world_pos.x < -89.0 {
                audio_play_actor_sfx2(play, NA_SE_EV_BRIDGE_OPEN_STOP);
                self.unk_178 = 0x1E;
                self.action = Action::Done;
            }
        }
    }

    /// `func_808783AC`.
    fn done(&mut self) {
        if self.unk_178 == 0 {
            self.opening_state = 0;
            self.action = Action::Wait;
        }
    }

    /// `func_808783D4`.
    fn shut(&mut self, play: &mut PlayState) {
        if self.unk_178 == 0 {
            audio_play_actor_sfx2(play, NA_SE_EV_METALGATE_OPEN - SFX_FLAG);
            self.actor.world_pos.x += 2.0;
            approach_f(&mut self.actor.world_pos.z, -1350.0, 0.8, 0.3);
            if self.actor.world_pos.x > 90.0 {
                self.actor.world_pos.x = 91.0;
                audio_play_actor_sfx2(play, NA_SE_EV_BRIDGE_OPEN_STOP);
                self.unk_178 = 30;
                self.action = Action::Done;
            }
        }
    }
}

impl ActorImpl for BgGateShutter {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `BgGateShutter_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if self.unk_178 != 0 {
            self.unk_178 -= 1;
        }
        match self.action {
            Action::Wait => self.wait(play),
            Action::Open => self.open(play),
            Action::Done => self.done(),
            Action::Shut => self.shut(play),
        }
        play.col.dyna.set_source(self.bg, source(&self.actor));
    }
    /// `BgGateShutter_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }
    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }
    fn render_state(&self) -> RenderState {
        RenderState::of(&self.actor)
    }
    /// `BgGateShutter_Draw`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh(OBJECT, "gKakarikoGuardGateDL")), oot_game::play::actor_draw_matrix(rs)));
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
