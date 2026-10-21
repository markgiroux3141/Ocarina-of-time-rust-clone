//! `Bg_Spot03_Taki` (`ovl_Bg_Spot03_Taki/z_bg_spot03_taki.c`): the waterfall in front of Zora's
//! Domain at the top of Zora's River (GAME-10 milestone 2), a DynaPoly actor of
//! `object_spot03_object` (`object_spot03_object_Col_000C98`: the wall that keeps Link out) at a
//! tenth of its size. The river's room 1 places one (params 0x0038) at (4226, 660, -1402) in every
//! layer; the song spot above it (`En_Okarina_Tag` 0x07F8, type 1) sets its switch flag 0x38 when
//! Zelda's Lullaby is played there.
//!
//! - **Closed** (`WATERFALL_CLOSED`): it waits for its switch flag. Set, one-point cutscene 4100
//!   starts (`OnePointCutscene_Init(play, 4100, -99, NULL, CAM_ID_MAIN)`: Link turned east and held,
//!   the camera's five keyframes on the falls) and it **opens** (`WATERFALL_OPENING_ANIMATED`): the
//!   middle of the falls fades out, its alpha (`openingAlpha`) down 5 a frame from 255. At 0 its
//!   collision goes (`DynaPoly_DisableCollision`) for 400 frames (`WATERFALL_OPENED`); then it
//!   **closes** (`WATERFALL_CLOSING`), the alpha up 5 a frame; at 255 the collision is back, the
//!   switch flag cleared and it's closed again: the song is played on every visit.
//! - **The fade** (`BgSpot03Taki_ApplyOpeningAlpha`): the alpha is written into the object's own
//!   vertices, the five of the falls' middle (10 to 14) of one of its two copies of the falls'
//!   vertex array (`object_spot03_object_Vtx_000800`, `_000990`, 25 each): the one the next draw
//!   loads (`bufferIndex`, which the draw flips after loading it: the RSP reads last frame's copy
//!   while this frame's is written). Init writes both at 255.
//! - **The sound** (`Audio_PlaySfxWaterfall`, from its draw): the falls' roar at frequency 0.5
//!   while it's open or opening, 1.0 otherwise.
//!
//! The whole overlay is ported. The draw (`BgSpot03Taki_Draw`, XLU after `Gfx_SetupDL_25Xlu`):
//! `object_spot03_object_DL_000B20` (the material, its texture scrolling on segment 8,
//! `Gfx_TwoTexScroll(.., 0, gameplayFrames * 5, 64, 64, 1, 0, gameplayFrames * 5, 64, 64)`), the
//! copy's vertices (`gSPVertex(.., 25, 0)`) and `object_spot03_object_DL_000BC0` (the falls drawn
//! from them); then the spray at the falls' foot, `object_spot03_object_DL_001580` with segment 8
//! set again (`Gfx_TwoTexScroll(.., gameplayFrames, gameplayFrames * 3, 64, 64, 1, -gameplayFrames,
//! gameplayFrames * 3, 64, 64)`). The vertex load is the bake's own segment call between the two
//! lists (a `BakeSegment::Commands` segment: nothing new), one bake for each copy; the alphas go on
//! through the bake's vertex sources (`FrustumVertices`, ADR 0053). The spray sets its own material
//! in full, so it's a second bake. `draw_update` flips the buffer (the draw's write); `draw_sfx`
//! plays the roar. The `WATERFALL_OPENING_IDLE` state (a 40-frame wait) is never entered: the
//! switch's 40 (`timer`) is set and never read.
//!
//! Params: `switchFlag` bits 0 to 5 (`PARAMS_GET_U(params, 0, 6)`: the flag that opens it; 0x38 in
//! the river).
//! Flags: reads switch flag `switchFlag` (set: it opens); clears it once it has closed again.

use std::sync::Arc;

use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::SfxPos;
use oot_game::camera::CAM_ID_MAIN;
use oot_game::gbi::Dl;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, SEGMENT_CALL, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::scene_table::gfx_two_tex_scroll;

use crate::oceff_wipe::FrustumVertices;

/// `ACTOR_BG_SPOT03_TAKI` (`actor_table.h`: 0x011F).
pub const ACTOR_BG_SPOT03_TAKI: i16 = 0x011F;
/// `OBJECT_SPOT03_OBJECT` (`object_table.h`: 0x0112).
pub const OBJECT: &str = "object_spot03_object";
/// `object_spot03_object_Col_000C98` (`object_spot03_object.xml`).
pub const COLLISION: &str = "object_spot03_object_Col_000C98";
/// `object_spot03_object_DL_000B20`: the falls' material, calling segment 8.
const DL_MATERIAL: &str = "object_spot03_object_DL_000B20";
/// `object_spot03_object_DL_000BC0`: the falls' triangles, on the vertices the draw loaded.
const DL_FALLS: &str = "object_spot03_object_DL_000BC0";
/// `object_spot03_object_DL_001580`: the spray at the falls' foot (its own material and
/// vertices, calling segment 8).
const DL_SPRAY: &str = "object_spot03_object_DL_001580";
/// `object_spot03_object_Vtx_000800`, `object_spot03_object_Vtx_000990` (the XML's offsets 0x800,
/// 0x990): the falls' vertex arrays, 25 each, by `bufferIndex`.
pub const VTX: [u32; 2] = [0x0600_0800, 0x0600_0990];
/// The vertices' count (`gSPVertex(.., 25, 0)`).
const VTX_COUNT: u32 = 25;
/// The vertices whose alpha the opening writes (`vtx[i + 10]`, `i` 0 to 4).
pub const OPENING_VTX: std::ops::Range<usize> = 10..15;

/// The one-point cutscene of the falls opening (`OnePointCutscene_Init(play, 4100, -99, ..)`).
pub const ONEPOINT_OPENING: i16 = 4100;

/// `Bg_Spot03_Taki_Profile`: `ACTORCAT_BG`, always updating and drawn, `OBJECT_SPOT03_OBJECT`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_BG_SPOT03_TAKI,
    name: "Bg_Spot03_Taki",
    category: ACTORCAT_BG,
    flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `PARAMS_GET_U(params, 0, 6)`: the switch flag that opens it.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 0, 6, Kind::SwitchFlag, "set: the falls open for 400 frames (one-point 4100), then close and clear it");

/// Bg_Spot03_Taki's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[SWITCH_FLAG],
    flags: &[FlagUse::new(Flag::Switch("switchFlag")).reads().clears().note("set (by the song spot's Zelda's Lullaby): it opens; cleared once it has closed again")],
    todo: false,
    objects: &[],
    texts: &[],
};

/// `BgSpot03TakiState` (`z_bg_spot03_taki.h`).
pub const WATERFALL_CLOSED: u8 = 0;
pub const WATERFALL_OPENING_IDLE: u8 = 1;
pub const WATERFALL_OPENING_ANIMATED: u8 = 2;
pub const WATERFALL_OPENED: u8 = 3;
pub const WATERFALL_CLOSING: u8 = 4;

/// The draw's segment 8 (`Gfx_TwoTexScroll` before each list), and the bakes' segment holding the
/// vertex load (`gSPVertex` between the material and the falls).
const SEG_SCROLL: u8 = 0x08;
const SEG_VTX: u8 = 0x09;

/// The bakes: the falls loading each copy, and the spray.
pub const BAKE_FALLS: [&str; 2] = ["Bg_Spot03_Taki/falls0", "Bg_Spot03_Taki/falls1"];
pub const BAKE_SPRAY: &str = "Bg_Spot03_Taki/spray";

/// The falls' scroll: `Gfx_TwoTexScroll(.., G_TX_RENDERTILE, 0, gameplayFrames * 5, 64, 64, 1, 0,
/// gameplayFrames * 5, 64, 64)`.
fn falls_scroll(frames: u32) -> Vec<(u32, u32)> {
    let y = frames.wrapping_mul(5);
    gfx_two_tex_scroll(0, 0, y, 64, 64, 1, 0, y, 64, 64)
}

/// The spray's scroll: `Gfx_TwoTexScroll(.., G_TX_RENDERTILE, gameplayFrames * 1, gameplayFrames *
/// 3, 64, 64, 1, -gameplayFrames, gameplayFrames * 3, 64, 64)`.
fn spray_scroll(frames: u32) -> Vec<(u32, u32)> {
    let y = frames.wrapping_mul(3);
    gfx_two_tex_scroll(0, frames, y, 64, 64, 1, frames.wrapping_neg(), y, 64, 64)
}

/// `BgSpot03Taki_Draw`'s meshes (after `Gfx_SetupDL_25Xlu`): the material, the copy's vertices,
/// the falls (one bake for each copy); the spray.
pub fn bakes() -> Vec<MeshBake> {
    let mut v: Vec<MeshBake> = VTX
        .iter()
        .zip(BAKE_FALLS)
        .map(|(&vtx, name)| {
            let mut load = Dl::default();
            load.vertex(vtx, VTX_COUNT, 0);
            load.end();
            MeshBake {
                name: name.into(),
                object: OBJECT.into(),
                segments: vec![(SEG_SCROLL, BakeSegment::Dynamic(falls_scroll(0))), (SEG_VTX, BakeSegment::Commands(load.0))],
                prelude: Vec::new(),
                body: BakeBody::DLists(vec![(OBJECT.into(), DL_MATERIAL.into()), (SEGMENT_CALL.into(), format!("{SEG_VTX:x}")), (OBJECT.into(), DL_FALLS.into())]),
            }
        })
        .collect();
    v.push(MeshBake {
        name: BAKE_SPRAY.into(),
        object: OBJECT.into(),
        segments: vec![(SEG_SCROLL, BakeSegment::Dynamic(spray_scroll(0)))],
        prelude: Vec::new(),
        body: BakeBody::DLists(vec![(OBJECT.into(), DL_SPRAY.into())]),
    });
    v
}

/// `actionFunc`: only `BgSpot03Taki_HandleWaterfallState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    HandleWaterfallState,
}

pub struct BgSpot03Taki {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub action: Action,
    pub timer: i16,
    /// `state` (`BgSpot03TakiState`).
    pub state: u8,
    /// `switchFlag`.
    pub switch_flag: u16,
    /// `openingAlpha`.
    pub opening_alpha: f32,
    /// `bufferIndex`: the copy of the vertices the next draw loads.
    pub buffer_index: u8,
    /// The alpha of vertices 10 to 14 in each copy (`vtx[i + 10].v.cn[3]`), as written.
    pub vtx_alpha: [u8; 2],
    /// The copy the last draw loaded (its `bufferIndex` before the flip).
    pub drawn_buffer: u8,
    /// Each bake's vertices (their sources), for the alphas.
    pub vertices: [Option<Arc<FrustumVertices>>; 2],
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

impl BgSpot03Taki {
    /// `BgSpot03Taki_Init`: the switch flag; `DynaPolyActor_Init(0)` and the collision (at the
    /// spawn's scale; the next `DynaPoly_UpdateContext` takes the init chain's); scale 0.1
    /// (`sInitChain`); copy 0 next, the alpha 255 in both copies; waiting.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let switch_flag = SWITCH_FLAG.get(actor.params) as u16;
        let bg = match crate::obj_kibako2::load_collision(play, OBJECT, COLLISION) {
            Some(h) => play.col.dyna.set_bg_actor(h, source(&actor), 0),
            None => BG_ACTOR_MAX,
        };
        let vertices = BAKE_FALLS.map(|b| FrustumVertices::load(play, b));
        let mut this = BgSpot03Taki {
            actor,
            bg,
            action: Action::HandleWaterfallState,
            timer: 0,
            state: WATERFALL_CLOSED,
            switch_flag,
            opening_alpha: 255.0,
            buffer_index: 0,
            vtx_alpha: [0; 2],
            drawn_buffer: 0,
            vertices,
        };
        // sInitChain: ICHAIN_VEC3F_DIV1000(scale, 100).
        this.actor.scale = Vec3::splat(0.1);
        this.buffer_index = 0;
        this.opening_alpha = 255.0;
        this.apply_opening_alpha(0);
        this.apply_opening_alpha(1);
        this.action = Action::HandleWaterfallState;
        Box::new(this)
    }

    /// `BgSpot03Taki_ApplyOpeningAlpha`: the alpha (`(u8)openingAlpha`) into vertices 10 to 14 of
    /// copy `buffer_index`.
    pub fn apply_opening_alpha(&mut self, buffer_index: u8) {
        self.vtx_alpha[(buffer_index != 0) as usize] = self.opening_alpha as u8;
    }

    /// `BgSpot03Taki_HandleWaterfallState`: see the module's docs; the alpha into the copy the
    /// next draw loads.
    fn handle_waterfall_state(&mut self, play: &mut PlayState) {
        if self.state == WATERFALL_CLOSED {
            if play.flags.get_switch(self.switch_flag as i32) {
                self.state = WATERFALL_OPENING_ANIMATED;
                self.timer = 40;
                play.onepoint_cutscene_init(ONEPOINT_OPENING, -99, None, CAM_ID_MAIN);
            }
        } else if self.state == WATERFALL_OPENING_IDLE {
            self.timer -= 1;
            if self.timer < 0 {
                self.state = WATERFALL_OPENING_ANIMATED;
            }
        } else if self.state == WATERFALL_OPENING_ANIMATED {
            if self.opening_alpha > 0.0 {
                self.opening_alpha -= 5.0;
                if self.opening_alpha <= 0.0 {
                    play.col.dyna.set_collision_disabled(self.bg, true);
                    self.timer = 400;
                    self.state = WATERFALL_OPENED;
                    self.opening_alpha = 0.0;
                }
            }
        } else if self.state == WATERFALL_OPENED {
            self.timer -= 1;
            if self.timer < 0 {
                self.state = WATERFALL_CLOSING;
            }
        } else if self.state == WATERFALL_CLOSING && self.opening_alpha < 255.0 {
            self.opening_alpha += 5.0;
            if self.opening_alpha >= 255.0 {
                play.col.dyna.set_collision_disabled(self.bg, false);
                self.state = WATERFALL_CLOSED;
                self.opening_alpha = 255.0;
                play.flags.unset_switch(self.switch_flag as i32);
            }
        }
        self.apply_opening_alpha(self.buffer_index);
    }
}

impl ActorImpl for BgSpot03Taki {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgSpot03Taki_Update`: the action; the transform for `DynaPoly_UpdateContext` (the init
    /// chain's scale from the first frame).
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::HandleWaterfallState => self.handle_waterfall_state(play),
        }
        play.col.dyna.set_source(self.bg, source(&self.actor));
    }

    /// `BgSpot03Taki_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }

    fn dyna_bg_id(&self) -> Option<u16> {
        Some(self.bg)
    }

    /// `BgSpot03Taki_Draw`'s write: the copy it loads, then `bufferIndex = bufferIndex == 0`.
    fn draw_update(&mut self, _play: &mut PlayState) {
        self.drawn_buffer = self.buffer_index;
        self.buffer_index = (self.buffer_index == 0) as u8;
    }

    /// `BgSpot03Taki_Draw`'s sound: the roar at 0.5 while opening or open, else 1.0.
    fn draw_sfx(&mut self, play: &mut PlayState) {
        if self.actor.killed {
            return;
        }
        let pos = play.cur_actor.map(SfxPos::Actor).unwrap_or(SfxPos::Default);
        if self.state >= WATERFALL_OPENING_IDLE && self.state <= WATERFALL_OPENED {
            play.audio.play_sfx_waterfall(pos, 0.5);
        } else {
            play.audio.play_sfx_waterfall(pos, 1.0);
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        let b = self.drawn_buffer as usize & 1;
        rs.switches = vec![b as u32, self.vtx_alpha[b] as u32];
        rs
    }

    /// `BgSpot03Taki_Draw`: the falls from the copy the draw loaded (its alphas), then the spray.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let [b, alpha] = rs.switches.as_slice() else { return };
        let (b, alpha) = (*b as usize & 1, *alpha as u8);
        let frames = play.gameplay_frames;
        let m = actor_draw_matrix(rs);
        let vertex_colors = self.vertices[b].as_ref().map(|v| v.with_alphas(|i| OPENING_VTX.contains(&i).then_some(alpha)));
        let mut sv = SegmentValues::default();
        sv.read(SEG_SCROLL, &falls_scroll(frames));
        out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE_FALLS[b])), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), vertex_colors, ..Default::default() } });
        let mut sv = SegmentValues::default();
        sv.read(SEG_SCROLL, &spray_scroll(frames));
        out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE_SPRAY)), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
