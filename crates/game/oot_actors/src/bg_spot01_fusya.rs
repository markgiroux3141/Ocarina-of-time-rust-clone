//! `Bg_Spot01_Fusya` (`ovl_Bg_Spot01_Fusya/z_bg_spot01_fusya.c`): the windmill's sails in
//! Kakariko (`gKakarikoWindmillSailsDL` in `object_spot01_objects`).
//!
//! They turn about z by `unk_154` a frame (100 at first), which approaches `unk_158` by half the
//! difference, at most 100 a frame (`Math_ApproachF`): `unk_158` is 100 until `EVENTCHKINF_65`
//! (set as the windmill man teaches the Song of Storms, `En_Fu`, and by a cutscene's
//! `CS_MISC_SET_FLAG_FAST_WINDMILL`), then 1800. Outside a cutscene layer the init clears
//! `EVENTCHKINF_65`, so the fast turn lasts only as long as the scene. The windmill's sound plays
//! every frame, its frequency
//! `(unk_154 - 100) / 1700 + 1` (`func_800F436C`).
//!
//! The whole overlay is ported.
//!
//! Params: none (Kakariko places it with 0xFFFF).
//! Flags: reads EVENTCHKINF_65 (the fast turn); clears it outside a cutscene layer.

use eng_math::approach_f;
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile};
use oot_game::audio::sfx::{SFX_FLAG, SfxPos};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

/// `ACTOR_BG_SPOT01_FUSYA` (`actor_table.h`: 0x0102).
pub const ACTOR_BG_SPOT01_FUSYA: i16 = 0x0102;
/// `OBJECT_SPOT01_OBJECTS` (`object_table.h`: 0x00F9).
pub const OBJECT: &str = "object_spot01_objects";

/// `Bg_Spot01_Fusya_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_SPOT01_FUSYA, name: "Bg_Spot01_Fusya", category: ACTORCAT_BG, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT };

/// `EVENTCHKINF_65` (`save.h`: 0x65): the sails' fast turn.
pub const EVENTCHKINF_65: u16 = 0x65;
/// `NA_SE_EV_WINDMILL_LEVEL` (`environmentbank_table.h`: 0x2885).
pub const NA_SE_EV_WINDMILL_LEVEL: u16 = 0x2885;

pub struct BgSpot01Fusya {
    pub actor: Actor,
    /// `unk_154`: the turn a frame.
    pub unk_154: f32,
    /// `unk_158`: the turn it approaches.
    pub unk_158: f32,
    /// `unk_15C`: `Math_ApproachF`'s fraction.
    pub unk_15c: f32,
}

impl BgSpot01Fusya {
    /// `BgSpot01Fusya_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: scale 0.1, cullingVolumeDistance 12800, cullingVolumeScale 1300,
        // cullingVolumeDownward 1300.
        actor.scale = Vec3::splat(0.1);
        actor.culling_volume_distance = 12800.0;
        actor.culling_volume_scale = 1300.0;
        actor.culling_volume_downward = 1300.0;
        if !play.is_cutscene_layer() {
            play.save.clear_event_chk_inf(EVENTCHKINF_65);
        }
        Box::new(BgSpot01Fusya { actor, unk_154: 100.0, unk_158: 100.0, unk_15c: 0.5 })
    }

    /// `func_808AAA50`: the turn, the sound, the turn's approach.
    fn func_808aaa50(&mut self, play: &mut PlayState) {
        if play.save.get_event_chk_inf(EVENTCHKINF_65) {
            self.unk_158 = 1800.0;
        }
        // shape.rot.z += unk_154: the s16 widened to a float and the sum truncated back.
        let r = &mut self.actor.shape_rot.z;
        *r = (*r as f32 + self.unk_154) as i32 as i16;
        let temp = ((self.unk_154 - 100.0) / 1700.0) + 1.0;
        if let Some(h) = play.cur_actor {
            play.audio.func_800f436c(SfxPos::Actor(h), NA_SE_EV_WINDMILL_LEVEL - SFX_FLAG, temp);
        }
        approach_f(&mut self.unk_154, self.unk_158, self.unk_15c, 100.0);
    }
}

impl ActorImpl for BgSpot01Fusya {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `BgSpot01Fusya_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.func_808aaa50(play);
    }
    fn render_state(&self) -> RenderState {
        RenderState::of(&self.actor)
    }
    /// `BgSpot01Fusya_Draw`: `Gfx_SetupDL_25Opa`, `gKakarikoWindmillSailsDL`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        crate::gfx_draw_dlist_opa(out, OBJECT, "gKakarikoWindmillSailsDL", rs);
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
