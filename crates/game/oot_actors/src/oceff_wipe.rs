//! `Oceff_Wipe` (`ovl_Oceff_Wipe/z_oceff_wipe.c`): Zelda's Lullaby's and the Song of Time's
//! effect (`MSGMODE_SONG_PLAYED` spawns it at Link: params `OCEFF_WIPE_ZL` 0, `OCEFF_WIPE_SOT`
//! 1). A frustum of light in front of the camera's eye, facing the camera, rushing out over 32
//! frames (`Math_SinS(timer << 9) * 1400`) and fading from frame 80; gone at 100. The lullaby's
//! is pink and violet, the Song of Time's blue.
//!
//! The draw writes the frustum's vertices' alphas every frame (`sFrustumVtx[i].v.cn[3]` from
//! `sAlphaIndices`): the bake keeps each vertex's source address, and the draw rebuilds the
//! colours from them (`FrustumVertices`, as `Demo_Effect`'s, ADR 0053). The other wipes
//! (`oceff_wipe2`, `_3`, `_4`) share this file's helpers.
//!
//! The whole overlay is ported, its destroy's `Magic_Reset` too; Nayru's Love's restore never
//! applies (the spells aren't ported: `nayrusLoveTimer` stays 0).
//!
//! Params: `params` whole (`OceffWipeType`: 0 `OCEFF_WIPE_ZL` Zelda's Lullaby's colours, any other
//! `OCEFF_WIPE_SOT`'s, the Song of Time's); only spawned, by `Message_DrawMain`
//! (`sOcarinaEffectActorParams`: 0 for the lullaby, 1 for the Song of Time).
//! Flags: none.

use std::sync::Arc;

use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::sin_s;
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_UPDATE_DURING_OCARINA, Actor};
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, SEGMENT_CALL, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::scene_table::gfx_two_tex_scroll;

use crate::demo_effect::BakeVertices;

/// `ACTOR_OCEFF_WIPE` (`actor_table.h`: 0x018A).
pub const ACTOR_OCEFF_WIPE: i16 = 0x018A;

/// `Oceff_Wipe_Profile`.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_OCEFF_WIPE, name: "Oceff_Wipe", category: ACTORCAT_ITEMACTION, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA, object: "gameplay_keep", schema: &SCHEMA };

/// `OceffWipeType` (`z_oceff_wipe.h`).
pub const OCEFF_WIPE_ZL: i16 = 0;
pub const OCEFF_WIPE_SOT: i16 = 1;

/// `this->actor.params != OCEFF_WIPE_ZL`: whose colours.
pub const TYPE: Field = Field::whole("params", Kind::Named(&[(0, "OCEFF_WIPE_ZL"), (1, "OCEFF_WIPE_SOT")]), "Zelda's Lullaby's colours (0) or the Song of Time's (any other)");

/// Oceff_Wipe's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[TYPE], flags: &[], todo: false };

pub const BAKE: &str = "Oceff_Wipe/frustum";
/// The draw's segments: the colours (`gDPSetPrimColor`, `gDPSetEnvColor` before the material) and
/// the scroll (`Gfx_TwoTexScroll` between the material and the frustum).
pub const SEG_COLOR: u8 = 0x0A;
pub const SEG_SCROLL: u8 = 0x08;

/// `sAlphaIndices`: each of the 20 vertex pairs' alphas, an index into the frame's table for the
/// first (high nibble) and second (low nibble).
const ALPHA_INDICES: [u8; 20] = [0x01, 0x10, 0x22, 0x01, 0x20, 0x12, 0x01, 0x20, 0x12, 0x01, 0x10, 0x22, 0x01, 0x20, 0x12, 0x01, 0x12, 0x21, 0x01, 0x02];

/// A wipe's bake: `Gfx_SetupDL_25Xlu` (the bake's start), the prelude's commands, `material`,
/// the scroll (segment 8), then `body` lists.
pub fn wipe_bake(name: &str, file: &str, prelude: Vec<(u8, BakeSegment)>, material: &[&str], body: &[&str], scroll: Vec<(u32, u32)>) -> MeshBake {
    let order: Vec<u8> = prelude.iter().map(|(s, _)| *s).collect();
    let mut segments = prelude;
    segments.push((SEG_SCROLL, BakeSegment::Dynamic(scroll)));
    let mut lists: Vec<(String, String)> = material.iter().map(|m| (file.to_string(), m.to_string())).collect();
    lists.push((SEGMENT_CALL.into(), format!("{SEG_SCROLL:x}")));
    lists.extend(body.iter().map(|b| (file.to_string(), b.to_string())));
    MeshBake { name: name.into(), object: "gameplay_keep".into(), segments, prelude: order, body: BakeBody::DLists(lists) }
}

pub fn bakes() -> Vec<MeshBake> {
    vec![wipe_bake(BAKE, "ovl_Oceff_Wipe", vec![(SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true })], &["sMaterialDL"], &["sFrustumDL"], scroll(0))]
}

/// `Gfx_TwoTexScroll(.., G_TX_RENDERTILE, 0 - scroll, scroll * -2, 32, 32, 1, 0 - scroll, scroll * -2,
/// 32, 32)`.
fn scroll(scroll: u32) -> Vec<(u32, u32)> {
    let (a, b) = (0u32.wrapping_sub(scroll), scroll.wrapping_mul((-2i32) as u32));
    gfx_two_tex_scroll(0, a, b, 32, 32, 1, a, b, 32, 32)
}

/// A bake's vertices whose alphas the draw writes: the colours as baked and where each vertex
/// was loaded from, the array's start the lowest of them (the frustum lists load the arrays from
/// their first vertex).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FrustumVertices {
    pub v: BakeVertices,
    pub base: u32,
}

impl FrustumVertices {
    pub fn load(play: &PlayState, bake: &str) -> Option<Arc<FrustumVertices>> {
        let v = BakeVertices::load(play, bake)?;
        let base = v.sources.iter().copied().filter(|&s| s != 0).min()?;
        Some(Arc::new(FrustumVertices { v: (*v).clone(), base }))
    }

    /// The colours with the alphas `alpha_of(i)` gives the array's vertex `i` (`None`: as baked).
    pub fn with_alphas(&self, alpha_of: impl Fn(usize) -> Option<u8>) -> Vec<[u8; 4]> {
        self.v
            .colors
            .iter()
            .zip(self.v.sources.iter().copied().map(Some).chain(std::iter::repeat(None)))
            .map(|(&c, s)| match s.and_then(|s| s.checked_sub(self.base)).and_then(|o| alpha_of(o as usize / 16)) {
                Some(a) => [c[0], c[1], c[2], a],
                None => c,
            })
            .collect()
    }
}

/// The wipes' matrix: at the eye (`Matrix_Translate(eye + quakeOffset)`: the view's eye has the
/// quake in it), a tenth the size, facing the camera (`Matrix_ReplaceRotation(&billboardMtxF)`,
/// which keeps the scale), `z` ahead.
pub fn wipe_matrix(view: &ViewInfo, z: f32) -> Mat4 {
    Mat4::from_translation(view.eye) * view.billboard * Mat4::from_scale(Vec3::splat(0.1)) * Mat4::from_translation(Vec3::new(0.0, 0.0, -z))
}

/// A wipe's draw command: its bake, the matrix, the segments' values and the vertices' colours.
pub fn wipe_cmd(bake: &str, m: Mat4, sv: SegmentValues, vertex_colors: Option<Vec<[u8; 4]>>) -> DrawCmd {
    DrawCmd { mesh: MeshKey::named(keys::bake(bake)), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), vertex_colors, ..Default::default() } }
}

/// The song effects' destroy: `Magic_Reset`, and Nayru's Love's restore while its timer runs
/// (`PLAYER_STATE3_RESTORE_NAYRUS_LOVE`). The spells aren't ported: `nayrusLoveTimer` is never
/// set, so the restore never applies.
pub fn destroy_magic_reset(play: &mut PlayState, _name: &str) {
    play.magic_reset();
}

pub struct OceffWipe {
    pub actor: Actor,
    pub timer: i16,
    pub vertices: Option<Arc<FrustumVertices>>,
}

impl OceffWipe {
    /// `OceffWipe_Init`: a tenth the size, at the active camera's eye.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = OceffWipe { actor, timer: 0, vertices: FrustumVertices::load(play, BAKE) };
        this.actor.scale = Vec3::splat(0.1);
        this.actor.world_pos = play.active_camera().eye;
        // schema: printed
        log::debug!(" WIPE arg_data = {}", this.actor.params);
        Box::new(this)
    }
}

impl ActorImpl for OceffWipe {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `OceffWipe_Update`: kept at the eye, 100 frames.
    fn update(&mut self, play: &mut PlayState) {
        self.actor.world_pos = play.active_camera().eye;
        if self.timer < 100 {
            self.timer += 1;
        } else {
            self.actor.kill();
        }
    }
    /// `OceffWipe_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        destroy_magic_reset(play, PROFILE.name);
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.switches = vec![self.timer as u32];
        rs
    }
    /// `OceffWipe_Draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        let Some(&timer) = rs.switches.first() else { return };
        let timer = timer as i16;
        let frames = play.state_frames & 0xFF;
        let z = if timer < 32 { sin_s(timer << 9) * 1400.0 } else { 1400.0 };
        let alphas: [u8; 3] = if timer >= 80 { [0, ((0x64 - timer) * 8) as u8, ((0x64 - timer) * 12) as u8] } else { [0, 0xA0, 0xFF] };
        let vertex_colors = self.vertices.as_ref().map(|v| {
            v.with_alphas(|i| {
                let pair = ALPHA_INDICES.get(i / 2)?;
                Some(if i % 2 == 0 { alphas[((pair & 0xF0) >> 4) as usize] } else { alphas[(pair & 0xF) as usize] })
            })
        });
        let mut sv = SegmentValues::default();
        let (prim, env) = if TYPE.get(self.actor.params) != OCEFF_WIPE_ZL { ([170, 255, 255, 255], [0, 150, 255, 128]) } else { ([255, 255, 200, 255], [100, 0, 255, 128]) };
        sv.prim[SEG_COLOR as usize] = Some(prim);
        sv.env[SEG_COLOR as usize] = Some(env);
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
