//! `Bg_Spot15_Saku` (`ovl_Bg_Spot15_Saku/z_bg_spot15_saku.c`): the fence by Hyrule Castle's gate
//! guard, a DynaPoly actor of `object_spot15_obj` (`gLonLonCorralFenceCol`, drawn translucent with
//! `gLonLonCorralFenceDL`). The guard (`En_Heishi2` type 2) sets `unk_168` when he has been bribed:
//! two frames on the fence slides 2 a frame along -z (`NA_SE_EV_METALGATE_OPEN`) until it's past
//! z 2660 (`NA_SE_EV_BRIDGE_OPEN_STOP`), then 30 frames on it's ready again. With `INFTABLE_71`
//! (set only by a legacy text handler) it starts open at z 2659 and the guard's call does nothing.
//!
//! The whole overlay is ported.
//!
//! Params: `params` whole unused (0xFFFF in the data).
//! Flags: reads INFTABLE_71 (set: open from the start, the guard's call ignored).

use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use eng_gfx::{DrawCmd, MeshKey};
use glam::Vec3;
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind};
use oot_game::inf_table;
use oot_game::audio::sfx::SFX_FLAG;
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

/// `ACTOR_BG_SPOT15_SAKU` (`actor_table.h`: 0x00F8).
pub const ACTOR_BG_SPOT15_SAKU: i16 = 0x00F8;
pub const OBJECT: &str = "object_spot15_obj";

/// `Bg_Spot15_Saku_Profile`: `FLAGS` 0.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_SPOT15_SAKU, name: "Bg_Spot15_Saku", category: ACTORCAT_ITEMACTION, flags: 0, object: OBJECT, schema: &SCHEMA };

/// `INFTABLE_71` (`save.h`: 0x71).
pub const INFTABLE_71: u16 = 0x71;

/// The whole params: 0xFFFF in the game's data, read by nothing.
pub const PARAMS: Field = Field::whole("params", Kind::Unused, "0xFFFF in the data; read by nothing");

/// Bg_Spot15_Saku's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema =
    ActorSchema { fields: &[PARAMS], flags: &[FlagUse::new(inf_table!(INFTABLE_71)).reads().note("open from the start")], todo: false, objects: &[] };
/// `NA_SE_EV_METALGATE_OPEN`, `NA_SE_EV_BRIDGE_OPEN_STOP` (`environmentbank_table.h`: 0x2867,
/// 0x280E).
pub const NA_SE_EV_METALGATE_OPEN: u16 = 0x2867;
pub const NA_SE_EV_BRIDGE_OPEN_STOP: u16 = 0x280E;

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_808B4930`: shut, waiting for the guard's call.
    Wait,
    /// `func_808B4978`: sliding open.
    Open,
    /// `func_808B4A04`: open, 30 frames.
    Opened,
}

pub struct BgSpot15Saku {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub action: Action,
    /// `unk_168`: the guard's call to open.
    pub unk_168: u64,
    /// `unk_170`: where it was placed.
    pub unk_170: Vec3,
    pub timer: i16,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

impl BgSpot15Saku {
    /// `BgSpot15Saku_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = BgSpot15Saku { actor, bg: BG_ACTOR_MAX, action: Action::Wait, unk_168: 0, unk_170: Vec3::ZERO, timer: 0 };
        // DynaPolyActor_Init(&this->dyna, 0), the collision.
        this.actor.scale = Vec3::splat(0.1);
        if let Some(h) = crate::obj_kibako2::load_collision(play, OBJECT, "gLonLonCorralFenceCol") {
            this.bg = play.col.dyna.set_bg_actor(h, source(&this.actor), 0);
        }
        this.unk_170 = this.actor.world_pos;
        if play.save.get_inf_table(INFTABLE_71) {
            this.actor.world_pos.z = 2659.0;
        }
        play.col.dyna.set_source(this.bg, source(&this.actor));
        Box::new(this)
    }

    /// `func_808B4930`.
    fn wait(&mut self, play: &PlayState) {
        if self.unk_168 != 0 && !play.save.get_inf_table(INFTABLE_71) {
            self.timer = 2;
            self.action = Action::Open;
        }
    }

    /// `func_808B4978`.
    fn open(&mut self, play: &mut PlayState) {
        if self.timer == 0 {
            audio_play_actor_sfx2(play, NA_SE_EV_METALGATE_OPEN - SFX_FLAG);
            self.actor.world_pos.z -= 2.0;
            if self.actor.world_pos.z < 2660.0 {
                audio_play_actor_sfx2(play, NA_SE_EV_BRIDGE_OPEN_STOP);
                self.timer = 30;
                self.action = Action::Opened;
            }
        }
    }

    /// `func_808B4A04`.
    fn opened(&mut self) {
        if self.timer == 0 {
            self.unk_168 = 0;
            self.action = Action::Wait;
        }
    }
}

impl ActorImpl for BgSpot15Saku {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `BgSpot15Saku_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if self.timer != 0 {
            self.timer -= 1;
        }
        match self.action {
            Action::Wait => self.wait(play),
            Action::Open => self.open(play),
            Action::Opened => self.opened(),
        }
        play.col.dyna.set_source(self.bg, source(&self.actor));
    }
    /// `BgSpot15Saku_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }
    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }
    fn render_state(&self) -> RenderState {
        RenderState::of(&self.actor)
    }
    /// `BgSpot15Saku_Draw`: `Gfx_SetupDL_25Xlu`, `gLonLonCorralFenceDL` in `POLY_XLU_DISP`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        out.xlu.push(DrawCmd::new(MeshKey::named(keys::mesh(OBJECT, "gLonLonCorralFenceDL")), oot_game::play::actor_draw_matrix(rs)));
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
