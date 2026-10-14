//! `Oceff_Wipe3` (`ovl_Oceff_Wipe3/z_oceff_wipe3.c`): Saria's Song's effect
//! (`MSGMODE_SONG_PLAYED`'s), green. A frustum of light in front of the camera's eye, as
//! `Oceff_Wipe`'s (its helpers), rushing out over 32 frames (`Math_SinS(counter << 9) * 1330`) and
//! fading from frame 80; gone at 100.
//!
//! The whole overlay is ported. `Magic_Reset` and Nayru's Love's restore aren't (no magic, no
//! spells). Its init prints "WIPE2" (sic).
//!
//! Params: none (`Message_DrawMain` spawns it with 0 for Saria's Song; its init only prints them).
//! Flags: none.

use std::sync::Arc;

use eng_gfx::SegmentValues;
use eng_math::sin_s;
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_UPDATE_DURING_OCARINA, Actor};
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorImpl, ActorProfile};
use oot_game::pack::{BakeSegment, MeshBake};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::scene_table::gfx_two_tex_scroll;

use crate::oceff_wipe::{FrustumVertices, SEG_SCROLL, destroy_magic_reset, wipe_bake, wipe_cmd, wipe_matrix};

/// `ACTOR_OCEFF_WIPE3` (`actor_table.h`: 0x0199).
pub const ACTOR_OCEFF_WIPE3: i16 = 0x0199;

/// `Oceff_Wipe3_Profile`.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_OCEFF_WIPE3, name: "Oceff_Wipe3", category: ACTORCAT_ITEMACTION, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA, object: "gameplay_keep", schema: &oot_game::actor_schema::ActorSchema::NONE };

pub const BAKE: &str = "Oceff_Wipe3/frustum";

/// The draw's fixed colours (`gDPSetPrimColor`, `gDPSetEnvColor` before the material).
#[allow(dead_code)]
const SEG_COLOR: u8 = 0x0A;

pub fn bakes() -> Vec<MeshBake> {
    vec![wipe_bake(
        BAKE,
        "ovl_Oceff_Wipe3",
        vec![(SEG_COLOR, BakeSegment::Commands(vec![(0xFA00_0000, 0xFFFF_AAFF), (0xFB00_0000, 0x64C8_0080), (0xDF00_0000, 0)]))],
        &["sMaterialDL"],
        &["sFrustumDL"],
        scroll(0),
    )]
}

/// `Gfx_TwoTexScroll(.., G_TX_RENDERTILE, scroll * 12, scroll * -12, 64, 64, 1, scroll * 8, scroll * -8, 64, 64)`.
fn scroll(scroll: u32) -> Vec<(u32, u32)> {
    let m = |k: i32| scroll.wrapping_mul(k as u32);
    gfx_two_tex_scroll(0, m(12), m(-12), 64, 64, 1, m(8), m(-8), 64, 64)
}

/// The frustum's vertices whose alpha the draw writes (the odd ones, 1 to 21).
const ALPHA_VERTICES: [usize; 11] = [1, 3, 5, 7, 9, 11, 13, 15, 17, 19, 21];

pub struct OceffWipe3 {
    pub actor: Actor,
    pub counter: i16,
    pub vertices: Option<Arc<FrustumVertices>>,
}

impl OceffWipe3 {
    /// `OceffWipe3_Init`: a tenth the size, at the active camera's eye.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let bake = BAKE;
        let mut this = OceffWipe3 { actor, counter: 0, vertices: FrustumVertices::load(play, bake) };
        this.actor.scale = Vec3::splat(0.1);
        this.actor.world_pos = play.active_camera().eye;
        // schema: printed
        log::debug!(" WIPE2 arg_data = {}", this.actor.params);
        Box::new(this)
    }
}

impl ActorImpl for OceffWipe3 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `OceffWipe3_Update`: kept at the eye, 100 frames.
    fn update(&mut self, play: &mut PlayState) {
        self.actor.world_pos = play.active_camera().eye;
        if self.counter < 100 {
            self.counter += 1;
        } else {
            self.actor.kill();
        }
    }
    /// `OceffWipe3_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        destroy_magic_reset(play, PROFILE.name);
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.switches = vec![self.counter as u32];
        rs
    }
    /// `OceffWipe3_Draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        let Some(&t) = rs.switches.first() else { return };
        let t = t as i16;
        let frames = play.state_frames & 0xFFF;
        let z = if t < 32 { sin_s(t << 9) * 1330.0 } else { 1330.0 };
        let alpha: u8 = if t >= 80 { (12 * (100 - t)) as u8 } else { 255 };
        let vertex_colors = self.vertices.as_ref().map(|v| v.with_alphas(|i| ALPHA_VERTICES.contains(&i).then_some(alpha)));
        let mut sv = SegmentValues::default();
        sv.read(SEG_SCROLL, &scroll(frames));
        out.xlu.push(wipe_cmd(BAKE, wipe_matrix(view, z), sv, vertex_colors));
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
