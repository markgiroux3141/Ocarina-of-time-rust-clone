//! `En_Dy_Extra` (`ovl_En_Dy_Extra/z_en_dy_extra.c`): the spiral beam a Great Fairy pours over
//! Link when she refills him on a revisit (`Bg_Dy_Yoseizo`'s `BgDyYoseizo_Revisit_RefillPlayerEnergy`,
//! spawned as her child 200 above him).
//!
//! It sinks slowly (its gravity -0.2 easing to 0, `Math_ApproachF` by 0.005 a frame; held once
//! below y -55), the beam's hum `NA_SE_PL_SPIRAL_HEAL_BEAM` every frame, for at least 60 frames
//! (`EnDyExtra_Wait`); when the fairy sets `startDisappearing` after them, it fades
//! (`alphaFac` to 0 by `Math_ApproachZeroF(0.03, 0.05)`) and is gone once under 0.02 or 200
//! frames on (`EnDyExtra_Disappear`).
//!
//! Drawn (`EnDyExtra_Draw`) as `gGreatFairySpiralBeamDL` (`object_dy_obj`) at the actor's matrix
//! (its scale 0.025, 0.039, 0.025), its textures scrolling (`Gfx_TwoTexScroll` on segment 8), the
//! prim and env colours by type; `gGreatFairySpiralBeamVtx`'s alphas written every draw by
//! `sVerticesAlphaType` (0 kept, 1 `alphaFac × 240`, 2 `alphaFac × 255`): the bake's vertex colours
//! with those alphas (`oceff_wipe::FrustumVertices`, ADR 0053). The C's `(s8)` casts of the alphas
//! (`#ifndef AVOID_UB`) land on the same bytes as `(u8)` for 0 to 255.
//!
//! The whole overlay is ported.
//!
//! Params: `params` whole, the type (0: the magic fountain's, a pink env colour; 1: a spell
//! fountain's, cyan); spawned only, by `Bg_Dy_Yoseizo`.
//! Flags: none.

use std::sync::Arc;

use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{approach_f, approach_zero_f};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::audio::sfx::SFX_FLAG;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::scene_table::gfx_two_tex_scroll;

use crate::oceff_wipe::FrustumVertices;

/// `ACTOR_EN_DY_EXTRA` (`actor_table.h`: 0x0184).
pub const ACTOR_EN_DY_EXTRA: i16 = 0x0184;
/// `OBJECT_DY_OBJ` (`object_table.h`: 0x000A).
pub const OBJECT: &str = "object_dy_obj";

/// `En_Dy_Extra_Profile`: `ACTORCAT_PROP`, `ACTOR_FLAG_UPDATE_CULLING_DISABLED |
/// ACTOR_FLAG_DRAW_CULLING_DISABLED`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_DY_EXTRA,
    name: "En_Dy_Extra",
    category: ACTORCAT_PROP,
    flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `this->type = this->actor.params`: which fountain's colours (`sPrimColors`, `sEnvColors`).
pub const TYPE: Field = Field::whole("params", Kind::Named(&[(0, "the magic fountain's beam (pink)"), (1, "a spell fountain's beam (cyan)")]), "whose beam: its colours");

/// En_Dy_Extra's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[TYPE], flags: &[], todo: false, objects: &[], texts: &[] };

/// `NA_SE_PL_SPIRAL_HEAL_BEAM` (`playerbank_table.h`: 0x08C4).
pub const NA_SE_PL_SPIRAL_HEAL_BEAM: u16 = 0x08C4;

/// `sPrimColors`, `sEnvColors` (`EnDyExtra_Draw`), by type.
const PRIM_COLORS: [[u8; 4]; 2] = [[255, 255, 170, 255], [255, 255, 170, 255]];
const ENV_COLORS: [[u8; 4]; 2] = [[255, 100, 255, 255], [100, 255, 255, 255]];
/// `sVerticesAlphaType`: each of `gGreatFairySpiralBeamVtx`'s 27 vertices' alpha (0: as it is, 1:
/// `alphaFac × 240`, 2: `alphaFac × 255`).
pub const VERTICES_ALPHA_TYPE: [u8; 27] = [2, 1, 1, 2, 0, 0, 2, 1, 0, 2, 1, 0, 2, 1, 0, 2, 1, 0, 2, 1, 0, 2, 1, 0, 1, 2, 0];

pub const BAKE: &str = "En_Dy_Extra/beam";
/// The draw's segments: the scroll (`gSPSegment(0x08, Gfx_TwoTexScroll(...))`, called by the list)
/// and the colours (`gDPSetPrimColor`, `gDPSetEnvColor` before it).
const SEG_SCROLL: u8 = 0x08;
const SEG_COLOR: u8 = 0x0A;

/// `Gfx_TwoTexScroll(G_TX_RENDERTILE, frames * 2, 0, 0x20, 0x40, 1, frames, frames * -8, 0x10, 0x10)`.
fn scroll(frames: u32) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, frames.wrapping_mul(2), 0, 0x20, 0x40, 1, frames, frames.wrapping_mul((-8i32) as u32), 0x10, 0x10)
}

/// `EnDyExtra_Draw`'s mesh: `Gfx_SetupDL_25Xlu`, the colours, `gGreatFairySpiralBeamDL` (which calls
/// segment 8 for the scroll).
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: BAKE.into(),
        object: OBJECT.into(),
        segments: vec![(SEG_SCROLL, BakeSegment::Dynamic(scroll(0))), (SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true })],
        prelude: vec![SEG_COLOR],
        body: BakeBody::DLists(vec![(OBJECT.into(), "gGreatFairySpiralBeamDL".into())]),
    }]
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Wait,
    Disappear,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::Wait => "EnDyExtra_Wait",
            Action::Disappear => "EnDyExtra_Disappear",
        }
    }
}

pub struct EnDyExtra {
    pub actor: Actor,
    pub action: Action,
    /// `type`.
    pub ty: i16,
    /// `startDisappearing`: set by the fairy.
    pub start_disappearing: i16,
    pub timer: i16,
    /// `alphaFac`.
    pub alpha_fac: f32,
    /// `scale`: copied to the actor's every update.
    pub scale: Vec3,
    /// `homePos`.
    pub home_pos: Vec3,
    vertices: Option<Arc<FrustumVertices>>,
}

impl EnDyExtra {
    /// `EnDyExtra_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        log::debug!("☆☆☆☆☆ Big fairy effect ☆☆☆☆☆ {}", TYPE.get(actor.params));
        let mut this = EnDyExtra {
            ty: TYPE.get(actor.params),
            start_disappearing: 0,
            timer: 60,
            alpha_fac: 1.0,
            scale: Vec3::new(0.025, 0.039, 0.025),
            home_pos: actor.world_pos,
            action: Action::Wait,
            vertices: FrustumVertices::load(play, BAKE),
            actor,
        };
        this.actor.gravity = -0.2;
        Box::new(this)
    }

    /// `EnDyExtra_Wait`: the sinking eases off, held below -55; after the 60 frames, the fairy's
    /// word: 200 frames to fade.
    fn wait(&mut self) {
        approach_f(&mut self.actor.gravity, 0.0, 0.1, 0.005);
        if self.actor.world_pos.y < -55.0 {
            self.actor.velocity.y = 0.0;
        }
        if self.timer == 0 && self.start_disappearing != 0 {
            self.timer = 200;
            self.action = Action::Disappear;
        }
    }

    /// `EnDyExtra_Disappear`.
    fn disappear(&mut self) {
        approach_f(&mut self.actor.gravity, 0.0, 0.1, 0.005);
        if self.timer == 0 || self.alpha_fac < 0.02 {
            self.actor.kill();
            return;
        }
        approach_zero_f(&mut self.alpha_fac, 0.03, 0.05);
        if self.actor.world_pos.y < -55.0 {
            self.actor.velocity.y = 0.0;
        }
    }

    /// `alphaVals`: 0, `(s8)(alphaFac × 240)`, `(s8)(alphaFac × 255)` as the vertices' `u8`.
    pub fn alpha_vals(alpha_fac: f32) -> [u8; 3] {
        [0, (alpha_fac * 240.0) as i32 as u8, (alpha_fac * 255.0) as i32 as u8]
    }
}

impl ActorImpl for EnDyExtra {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnDyExtra_Update`: the timer down (`DECR`), the scale, the hum, the action, then
    /// `Actor_MoveXZGravity`.
    fn update(&mut self, play: &mut PlayState) {
        if self.timer != 0 {
            self.timer -= 1;
        }
        self.actor.scale = self.scale;
        audio_play_actor_sfx2(play, NA_SE_PL_SPIRAL_HEAL_BEAM - SFX_FLAG);
        match self.action {
            Action::Wait => self.wait(),
            Action::Disappear => self.disappear(),
        }
        self.actor.move_forward();
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.values = vec![self.alpha_fac];
        rs.switches = vec![self.ty as u16 as u32];
        rs
    }

    /// `EnDyExtra_Draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(&alpha_fac), Some(&ty)) = (rs.values.first(), rs.switches.first()) else { return };
        let ty = ty as usize;
        // sPrimColors[this->type]: only 0 and 1 are spawned (a larger type reads past the tables).
        let (Some(prim), Some(env)) = (PRIM_COLORS.get(ty), ENV_COLORS.get(ty)) else { return };
        let alpha_vals = Self::alpha_vals(alpha_fac);
        let vertex_colors = self.vertices.as_ref().map(|v| v.with_alphas(|i| VERTICES_ALPHA_TYPE.get(i).filter(|&&t| t != 0).map(|&t| alpha_vals[t as usize])));
        let mut sv = SegmentValues::default();
        sv.read(SEG_SCROLL, &scroll(play.state_frames));
        // gDPSetPrimColor(.., 255) and gDPSetEnvColor(.., 128).
        sv.prim[SEG_COLOR as usize] = Some([prim[0], prim[1], prim[2], 255]);
        sv.env[SEG_COLOR as usize] = Some([env[0], env[1], env[2], 128]);
        out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE)), transform: actor_draw_matrix(rs), bones: Vec::new(), params: DrawParams { segments: Some(sv), vertex_colors, ..Default::default() } });
    }

    /// `EnDyExtra_Destroy`: nothing.
    fn destroy(&mut self, _play: &mut PlayState) {}

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
