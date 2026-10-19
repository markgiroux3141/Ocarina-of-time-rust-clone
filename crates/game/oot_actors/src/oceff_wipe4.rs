//! `Oceff_Wipe4` (`ovl_Oceff_Wipe4/z_oceff_wipe4.c`): the Scarecrow's Song's effect
//! (`MSGMODE_SONG_PLAYED`'s; `OCEFF_WIPE4_UNUSED` another material), drawn twice: unscrolled
//! (`sMaterial2DL`), then scrolled (`&sMaterial2DL[11]`). A frustum of light in front of the
//! camera's eye, as `Oceff_Wipe`'s (its helpers), rushing out over 16 frames
//! (`Math_SinS(timer * 1024) * 1330`) and fading from frame 30; gone at 50.
//!
//! The whole overlay is ported, its destroy's `Magic_Reset` too. @bug (game): unlike the other
//! effects, its destroy doesn't restore Nayru's Love.
//!
//! Params: `params` whole (`OceffWipe4Type`: 1 `OCEFF_WIPE4_UNUSED` the other material, any other
//! `OCEFF_WIPE4_SCARECROWS`); `Message_DrawMain` spawns it with 0 for the Scarecrow's Song,
//! `En_Zl3` with 1 (`func_80B57754`).
//! Flags: none.

use std::sync::Arc;

use eng_gfx::SegmentValues;
use eng_math::sin_s;
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_UPDATE_DURING_OCARINA, Actor};
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::pack::MeshBake;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::scene_table::gfx_two_tex_scroll;

use crate::oceff_wipe::{FrustumVertices, SEG_SCROLL, destroy_magic_reset, wipe_bake, wipe_cmd, wipe_matrix};

/// `ACTOR_OCEFF_WIPE4` (`actor_table.h`: 0x01CB).
pub const ACTOR_OCEFF_WIPE4: i16 = 0x01CB;

/// `Oceff_Wipe4_Profile`.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_OCEFF_WIPE4, name: "Oceff_Wipe4", category: ACTORCAT_ITEMACTION, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA, object: "gameplay_keep", schema: &SCHEMA };

/// `OceffWipe4Type` (`z_oceff_wipe4.h`).
pub const OCEFF_WIPE4_SCARECROWS: i16 = 0;
pub const OCEFF_WIPE4_UNUSED: i16 = 1;

/// `this->actor.params == OCEFF_WIPE4_UNUSED`: which material.
pub const TYPE: Field =
    Field::whole("params", Kind::Named(&[(0, "OCEFF_WIPE4_SCARECROWS"), (1, "OCEFF_WIPE4_UNUSED")]), "the Scarecrow's Song's material, or the other one (1, En_Zl3's)");

/// Oceff_Wipe4's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[TYPE], flags: &[], todo: false, objects: &[], texts: &[] };

pub const BAKE: &str = "Oceff_Wipe4/frustum";
pub const BAKE_UNUSED: &str = "Oceff_Wipe4/frustum_unused";

/// The draw's fixed colours (`gDPSetPrimColor`, `gDPSetEnvColor` before the material).
#[allow(dead_code)]
const SEG_COLOR: u8 = 0x0A;

pub fn bakes() -> Vec<MeshBake> {
    let b = |name: &str, material: &str| wipe_bake(name, "ovl_Oceff_Wipe4", Vec::new(), &[material, "sMaterial2DL"], &["sMaterial2DL+0x58"], scroll(0));
    vec![b(BAKE, "sMaterialDL"), b(BAKE_UNUSED, "sUnusedMaterialDL")]
}

/// `Gfx_TwoTexScroll(.., G_TX_RENDERTILE, scroll * 2, scroll * -2, 32, 64, 1, scroll * -1, scroll, 32, 32)`.
fn scroll(scroll: u32) -> Vec<(u32, u32)> {
    let m = |k: i32| scroll.wrapping_mul(k as u32);
    gfx_two_tex_scroll(0, m(2), m(-2), 32, 64, 1, m(-1), scroll, 32, 32)
}

/// The frustum's vertices whose alpha the draw writes (the odd ones, 1 to 21).
const ALPHA_VERTICES: [usize; 11] = [1, 3, 5, 7, 9, 11, 13, 15, 17, 19, 21];

pub struct OceffWipe4 {
    pub actor: Actor,
    pub timer: i16,
    pub vertices: Option<Arc<FrustumVertices>>,
}

impl OceffWipe4 {
    /// `OceffWipe4_Init`: a tenth the size, at the active camera's eye.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let bake = if TYPE.get(actor.params) == OCEFF_WIPE4_UNUSED { BAKE_UNUSED } else { BAKE };
        let mut this = OceffWipe4 { actor, timer: 0, vertices: FrustumVertices::load(play, bake) };
        this.actor.scale = Vec3::splat(0.1);
        this.actor.world_pos = play.active_camera().eye;
        // schema: printed
        log::debug!(" WIPE4 arg_data = {}", this.actor.params);
        Box::new(this)
    }
}

impl ActorImpl for OceffWipe4 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `OceffWipe4_Update`: kept at the eye, 50 frames.
    fn update(&mut self, play: &mut PlayState) {
        self.actor.world_pos = play.active_camera().eye;
        if self.timer < 50 {
            self.timer += 1;
        } else {
            self.actor.kill();
        }
    }
    /// `OceffWipe4_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        destroy_magic_reset(play, PROFILE.name);
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.switches = vec![self.timer as u32];
        rs
    }
    /// `OceffWipe4_Draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        let Some(&t) = rs.switches.first() else { return };
        let t = t as i16;
        let frames = play.state_frames & 0xFFF;
        let z = if t < 16 { sin_s(t.wrapping_mul(1024)) * 1330.0 } else { 1330.0 };
        let alpha: u8 = if t >= 30 { (12 * (50 - t)) as u8 } else { 255 };
        let vertex_colors = self.vertices.as_ref().map(|v| v.with_alphas(|i| ALPHA_VERTICES.contains(&i).then_some(alpha)));
        let mut sv = SegmentValues::default();
        sv.read(SEG_SCROLL, &scroll(frames));
        out.xlu.push(wipe_cmd(if TYPE.get(self.actor.params) == OCEFF_WIPE4_UNUSED { BAKE_UNUSED } else { BAKE }, wipe_matrix(view, z), sv, vertex_colors));
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
