//! `Oceff_Storm` (`ovl_Oceff_Storm/z_oceff_storm.c`): the Song of Storms' effect
//! (`MSGMODE_SONG_PLAYED` spawns it at Link). A whirl of cloud round Link (`sCylinderModelDL`,
//! its vertices' alphas rising to 210 from frame 10 and falling from 65), with a screen of rain
//! over everything (`OceffStorm_Draw2`: a texture rectangle over the screen, its alpha 0 to 100
//! and back), rising off from frame 60; gone at 100. It spawns the storm itself
//! (`En_Okarina_Effect`, 30 below Link). Params 1 is only the screen of rain, which never ends
//! (`@bug (game)`: its action never kills it).
//!
//! The whole overlay is ported. The screen's `gSPTextureRectangle` is a quad over the 320x240
//! screen in the screen's space (`DrawParams::screen`), its texture coordinates the rectangle's
//! (0.13671875 a pixel, t falling); `SETUPDL_64` turns texturing off for the triangles it doesn't
//! draw, so the quad's commands turn it on (`gSPTexture`), as the rectangle is textured
//! regardless. Its destroy's `Magic_Reset` is ported; Nayru's Love's restore never applies (no
//! spells).
//!
//! Params: `params` whole (1: only the screen of rain, forever; any other: the storm, its
//! `En_Okarina_Effect` spawned); `Message_DrawMain` spawns it with 0.
//! Flags: none.

use std::sync::Arc;

use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use oot_game::actor::{ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_UPDATE_DURING_OCARINA, Actor};
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::gbi::{Dl, push_vtx, setup_dl};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, SEGMENT_CALL, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::scene_table::gfx_two_tex_scroll;

use crate::oceff_wipe::{FrustumVertices, SEG_SCROLL, destroy_magic_reset, wipe_bake};

/// `ACTOR_OCEFF_STORM` (`actor_table.h`: 0x018B).
pub const ACTOR_OCEFF_STORM: i16 = 0x018B;

/// `Oceff_Storm_Profile`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_OCEFF_STORM,
    name: "Oceff_Storm",
    category: ACTORCAT_ITEMACTION,
    flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA,
    object: "gameplay_keep",
    schema: &SCHEMA,
};

/// `this->actor.params == 1`: only the screen of rain.
pub const TYPE: Field = Field::whole("params", Kind::Named(&[(0, "the storm"), (1, "only the screen of rain (never ends)")]), "the storm, or only its screen of rain (1)");

/// Oceff_Storm's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[TYPE], flags: &[], todo: false, objects: &[], texts: &[] };

/// `ACTOR_EN_OKARINA_EFFECT` (`actor_table.h`: 0x0170).
const ACTOR_EN_OKARINA_EFFECT: i16 = 0x0170;

pub const BAKE_CYLINDER: &str = "Oceff_Storm/cylinder";
pub const BAKE_SCREEN: &str = "Oceff_Storm/screen";
/// The cylinder's fixed colours, the screen's setup and dynamic colour, its quad.
const SEG_COLOR: u8 = 0x0A;
const SEG_SETUP: u8 = 0x0B;
const SEG_VTX: u8 = 0x09;
const SEG_QUAD: u8 = 0x0D;

/// `gDPSetAlphaDither(G_AD_NOISE)`, `gDPSetColorDither(G_CD_NOISE)`.
fn dither_noise(d: &mut Dl) {
    d.othermode_h(4, 2, 2 << 4);
    d.color_dither(2 << 6);
}

pub fn bakes() -> Vec<MeshBake> {
    // OceffStorm_Draw: Gfx_SetupDL_25Xlu (the bake's start), gDPSetPrimColor(0x80, 0x80, 255, 255,
    // 200, 255), gDPSetEnvColor(150, 150, 0, 128), the dithers; the material, the scroll, the model.
    let mut colors = Dl::default();
    colors.0.push((0xFA00_8080, 0xFFFF_C8FF));
    colors.env_color([150, 150, 0, 128]);
    dither_noise(&mut colors);
    colors.end();
    let cylinder = wipe_bake(BAKE_CYLINDER, "ovl_Oceff_Storm", vec![(SEG_COLOR, BakeSegment::Commands(colors.0))], &["sCylinderMaterialDL"], &["sCylinderModelDL"], cylinder_scroll(0));
    // OceffStorm_Draw2: Gfx_SetupDL_64, the dithers, gDPSetPrimColor(0x80, 0x80, 200, 200, 150,
    // primColorAlpha) (dynamic), sMaterialDL, the scroll, the rectangle.
    let mut setup = setup_dl::setup_dl_64();
    dither_noise(&mut setup);
    setup.end();
    let mut vtx = Vec::new();
    // The unit quad from (0, 0) to (1, -1) (crate::sprite's way), its texture coordinates the
    // rectangle's over 320x240: s from 0 by 0.13671875 a pixel (43.75 texels, 1400 in S10.5), t
    // from 0 by -0.13671875 (-1050).
    for (ob, tc) in [([0, 0, 0], [0, 0]), ([1, 0, 0], [1400, 0]), ([1, -1, 0], [1400, -1050]), ([0, -1, 0], [0, -1050])] {
        push_vtx(&mut vtx, ob, tc, [255, 255, 255, 255]);
    }
    let mut quad = Dl::default();
    quad.texture_on(true);
    quad.vertex((SEG_VTX as u32) << 24, 4, 0);
    quad.quad0(0, 1, 2, 3);
    quad.end();
    let screen = MeshBake {
        name: BAKE_SCREEN.into(),
        object: "gameplay_keep".into(),
        segments: vec![
            (SEG_SETUP, BakeSegment::Commands(setup.0)),
            (SEG_COLOR, BakeSegment::DynamicColor { env: false, prim: true }),
            (SEG_SCROLL, BakeSegment::Dynamic(screen_scroll(0))),
            (SEG_VTX, BakeSegment::Bytes(vtx)),
            (SEG_QUAD, BakeSegment::Commands(quad.0)),
        ],
        prelude: vec![SEG_SETUP, SEG_COLOR],
        body: BakeBody::DLists(vec![("ovl_Oceff_Storm".into(), "sMaterialDL".into()), (SEGMENT_CALL.into(), format!("{SEG_SCROLL:x}")), (SEGMENT_CALL.into(), format!("{SEG_QUAD:x}"))]),
    };
    vec![cylinder, screen]
}

/// `Gfx_TwoTexScroll(.., G_TX_RENDERTILE, scroll * 4, (0 - scroll) * 8, 32, 32, 1, scroll * 8,
/// (0 - scroll) * 12, 32, 32)`.
fn cylinder_scroll(scroll: u32) -> Vec<(u32, u32)> {
    let neg = 0u32.wrapping_sub(scroll);
    gfx_two_tex_scroll(0, scroll.wrapping_mul(4), neg.wrapping_mul(8), 32, 32, 1, scroll.wrapping_mul(8), neg.wrapping_mul(12), 32, 32)
}

/// `Gfx_TwoTexScroll(.., G_TX_RENDERTILE, scroll * 8, scroll * 4, 64, 64, 1, scroll * 4, scroll * 4,
/// 64, 64)`.
fn screen_scroll(scroll: u32) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, scroll.wrapping_mul(8), scroll.wrapping_mul(4), 64, 64, 1, scroll.wrapping_mul(4), scroll.wrapping_mul(4), 64, 64)
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Default,
    Unk,
}

pub struct OceffStorm {
    pub actor: Actor,
    pub counter: u16,
    pub prim_color_alpha: u8,
    pub vtx_alpha: u8,
    pub pos_y_off: i16,
    pub pos_y_off_add: i16,
    pub action: Action,
    /// `actor.draw`: `OceffStorm_Draw2` (only the screen) for params 1.
    pub draw_screen_only: bool,
    pub vertices: Option<Arc<FrustumVertices>>,
}

impl OceffStorm {
    /// `OceffStorm_Init`: no size; params 1 the screen alone, else the storm spawned
    /// (`En_Okarina_Effect`, 30 below).
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = OceffStorm {
            actor,
            counter: 0,
            prim_color_alpha: 0,
            vtx_alpha: 0,
            pos_y_off: 0,
            pos_y_off_add: 0,
            action: Action::Default,
            draw_screen_only: false,
            vertices: FrustumVertices::load(play, BAKE_CYLINDER),
        };
        this.actor.scale = glam::Vec3::ZERO;
        this.pos_y_off = this.pos_y_off_add;
        if TYPE.get(this.actor.params) == 1 {
            this.action = Action::Unk;
            this.draw_screen_only = true;
        } else {
            let p = this.actor.world_pos;
            if let Err(e) = play.actor_spawn(ACTOR_EN_OKARINA_EFFECT, glam::Vec3::new(p.x, p.y - 30.0, p.z), [0, 0, 0], 1) {
                log::debug!("Oceff_Storm: En_Okarina_Effect: {e:?}");
            }
        }
        Box::new(this)
    }

    /// `OceffStorm_DefaultAction`.
    fn default_action(&mut self) {
        let c = self.counter;
        self.prim_color_alpha = if c < 20 {
            (c as f32 * 5.0) as i8 as u8
        } else if c > 80 {
            ((100 - c as i32) as f32 * 5.0) as i8 as u8
        } else {
            100
        };
        if c < 10 || c >= 90 {
            self.vtx_alpha = 0;
        } else if c <= 65 {
            if self.vtx_alpha <= 200 {
                self.vtx_alpha += 10;
            }
            self.actor.scale.x = 0.4;
            self.actor.scale.z = 0.4;
            self.actor.scale.y = 0.3;
        } else {
            self.vtx_alpha = ((90 - c as i32) * 10) as u8;
        }
        if c > 60 {
            self.actor.world_pos.y += self.pos_y_off as f32 * 0.01;
            self.pos_y_off = self.pos_y_off.wrapping_add(self.pos_y_off_add);
            self.pos_y_off_add = self.pos_y_off_add.wrapping_add(10);
        }
        if self.counter < 100 {
            self.counter += 1;
        } else {
            self.actor.kill();
        }
    }

    /// `OceffStorm_UnkAction`: the screen's alpha up to 100. @bug (game): never killed.
    fn unk_action(&mut self) {
        if self.prim_color_alpha < 100 {
            self.prim_color_alpha += 5;
        }
    }

    /// `OceffStorm_Draw2`: the screen of rain.
    fn draw_screen(prim_alpha: u8, frames: u32, out: &mut DrawOut) {
        let mut sv = SegmentValues::default();
        sv.prim[SEG_COLOR as usize] = Some([200, 200, 150, prim_alpha]);
        sv.read(SEG_SCROLL, &screen_scroll(frames));
        let params = DrawParams { segments: Some(sv), screen: true, ..Default::default() };
        out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE_SCREEN)), transform: oot_game::sprite::rect_transform(0.0, 0.0, 320.0, 240.0), bones: Vec::new(), params });
    }
}

impl ActorImpl for OceffStorm {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `OceffStorm_Update`: at Link, turned to the active camera's direction.
    fn update(&mut self, play: &mut PlayState) {
        if let Some(p) = play.player.and_then(|h| play.actors.actor(h)) {
            self.actor.world_pos = p.world_pos;
        }
        self.actor.shape_rot.y = play.cam_dir_yaw();
        match self.action {
            Action::Default => self.default_action(),
            Action::Unk => self.unk_action(),
        }
    }
    /// `OceffStorm_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        destroy_magic_reset(play, PROFILE.name);
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.switches = vec![self.prim_color_alpha as u32, self.vtx_alpha as u32];
        rs
    }
    /// `OceffStorm_Draw` (the cylinder, then `OceffStorm_Draw2`), or `OceffStorm_Draw2` alone.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(&prim_alpha), Some(&vtx_alpha)) = (rs.switches.first(), rs.switches.get(1)) else { return };
        let frames = play.state_frames & 0xFFF;
        if !self.draw_screen_only {
            let a = vtx_alpha as u8;
            let vertex_colors = self.vertices.as_ref().map(|v| {
                v.with_alphas(|i| match i {
                    0 | 6 | 16 | 25 => Some(a >> 1),
                    10 | 22 => Some(a),
                    _ => None,
                })
            });
            let mut sv = SegmentValues::default();
            sv.read(SEG_SCROLL, &cylinder_scroll(frames));
            out.xlu.push(DrawCmd {
                mesh: MeshKey::named(keys::bake(BAKE_CYLINDER)),
                transform: actor_draw_matrix(rs),
                bones: Vec::new(),
                params: DrawParams { segments: Some(sv), vertex_colors, ..Default::default() },
            });
        }
        Self::draw_screen(prim_alpha as u8, frames, out);
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
