//! `Bg_Spot01_Idomizu` (`ovl_Bg_Spot01_Idomizu/z_bg_spot01_idomizu.c`): the water in Kakariko's
//! well (`gKakarikoWellWaterDL` in `object_spot01_objects`).
//!
//! It stands at y 52, or -550 once the well is drained (`EVENTCHKINF_DRAINED_WELL`, set by the
//! Song of Storms' cutscene: `CS_MISC_SET_FLAG_DRAINED_WELL`) and always for an adult. Each frame
//! it puts the scene's `waterBoxes[0]` at its height and moves 2 a frame towards its target,
//! sounding `NA_SE_EV_WATER_LEVEL_DOWN` while above it (`BgSpot01Idomizu_UpdateWaterLevel`).
//!
//! Drawn translucent (`Gfx_SetupDL_25Xlu`) with `Gfx_TwoTexScroll` on segment 8 by
//! `play->state.frames` (a bake, docs/adr/0012-actor-bakes.md).
//!
//! The whole overlay is ported.
//!
//! Params: none (Kakariko places it with 0xFFFF).
//! Flags: reads EVENTCHKINF_DRAINED_WELL.

use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::approach_f;
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_DRAW_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile};
use oot_game::audio::sfx::SFX_FLAG;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::scene_table::gfx_two_tex_scroll;

/// `ACTOR_BG_SPOT01_IDOMIZU` (`actor_table.h`: 0x0104).
pub const ACTOR_BG_SPOT01_IDOMIZU: i16 = 0x0104;
/// `OBJECT_SPOT01_OBJECTS` (`object_table.h`: 0x00F9).
pub const OBJECT: &str = "object_spot01_objects";

/// `Bg_Spot01_Idomizu_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_SPOT01_IDOMIZU, name: "Bg_Spot01_Idomizu", category: ACTORCAT_BG, flags: ACTOR_FLAG_DRAW_CULLING_DISABLED, object: OBJECT };

/// `EVENTCHKINF_DRAINED_WELL` (`save.h`: 0x67).
pub const EVENTCHKINF_DRAINED_WELL: u16 = 0x67;
/// `NA_SE_EV_WATER_LEVEL_DOWN` (`environmentbank_table.h`: 0x285E).
pub const NA_SE_EV_WATER_LEVEL_DOWN: u16 = 0x285E;
/// The water box the C writes: `play->colCtx.colHeader->waterBoxes[0]`.
pub const WATER_BOX: usize = 0;
/// The heights: full and drained.
pub const WATER_FULL: f32 = 52.0;
pub const WATER_DRAINED: f32 = -550.0;

/// The water's bake: `gKakarikoWellWaterDL` after `Gfx_SetupDL_25Xlu` (the bake's start), the
/// scroll on segment 8.
pub const BAKE: &str = "Bg_Spot01_Idomizu/water";
const SEG_SCROLL: u8 = 0x08;

/// `BgSpot01Idomizu_Draw`'s scroll at `play->state.frames` (a `u32`):
/// `Gfx_TwoTexScroll(.., G_TX_RENDERTILE, 127 - frames % 128, frames & 0x7F, 32, 32, 1,
/// frames % 128, frames & 0x7F, 32, 32)`.
pub fn water_scroll(frames: u32) -> Vec<(u32, u32)> {
    let f = frames % 128;
    gfx_two_tex_scroll(0, 127 - f, frames & 0x7F, 32, 32, 1, f, frames & 0x7F, 32, 32)
}

pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: BAKE.into(),
        object: OBJECT.into(),
        segments: vec![(SEG_SCROLL, BakeSegment::Dynamic(water_scroll(0)))],
        prelude: vec![],
        body: BakeBody::DLists(vec![(OBJECT.into(), "gKakarikoWellWaterDL".into())]),
    }]
}

pub struct BgSpot01Idomizu {
    pub actor: Actor,
    /// `waterHeight`: the height it moves to.
    pub water_height: f32,
}

impl BgSpot01Idomizu {
    /// `BgSpot01Idomizu_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: scale 0.1.
        actor.scale = Vec3::splat(0.1);
        // GET_EVENTCHKINF(EVENTCHKINF_DRAINED_WELL) || LINK_AGE_IN_YEARS == YEARS_ADULT.
        let water_height = if play.save.get_event_chk_inf(EVENTCHKINF_DRAINED_WELL) || play.save.adult { WATER_DRAINED } else { WATER_FULL };
        actor.world_pos.y = water_height;
        Box::new(BgSpot01Idomizu { actor, water_height })
    }

    /// `BgSpot01Idomizu_UpdateWaterLevel`.
    fn update_water_level(&mut self, play: &mut PlayState) {
        if play.save.get_event_chk_inf(EVENTCHKINF_DRAINED_WELL) {
            self.water_height = WATER_DRAINED;
        }
        // waterBoxes[0].ySurface = world.pos.y (an s16).
        if !play.col.set_water_box_surface(WATER_BOX, self.actor.world_pos.y as i32 as i16) {
            log::error!("Bg_Spot01_Idomizu: the scene has no waterBoxes[{WATER_BOX}]");
        }
        if self.water_height < self.actor.world_pos.y {
            play.audio.play_sfx_centered(NA_SE_EV_WATER_LEVEL_DOWN - SFX_FLAG);
        }
        approach_f(&mut self.actor.world_pos.y, self.water_height, 1.0, 2.0);
    }
}

impl ActorImpl for BgSpot01Idomizu {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `BgSpot01Idomizu_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.update_water_level(play);
    }
    fn render_state(&self) -> RenderState {
        RenderState::of(&self.actor)
    }
    /// `BgSpot01Idomizu_Draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let mut sv = SegmentValues::default();
        sv.read(SEG_SCROLL, &water_scroll(play.state_frames));
        out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE)), transform: actor_draw_matrix(rs), bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
