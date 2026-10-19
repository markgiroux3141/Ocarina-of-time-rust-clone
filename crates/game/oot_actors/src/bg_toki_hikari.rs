//! `Bg_Toki_Hikari` (`ovl_Bg_Toki_Hikari/z_bg_toki_hikari.c`): the Temple of Time's windows.
//! `params` 0: the windows (`func_808BA018`), a child's with the light streaming through them
//! (two scrolling layers); 1: the beam of light when the Door of Time opens (`func_808BA2CC`),
//! growing once the room's `drawParams[1]` is set (`func_808BA22C`) and fading (`func_808BA274`).
//!
//! Drawn from baked meshes (docs/adr/0012-actor-bakes.md).
//!
//! Params: `params` whole (0: the windows; 1: the Door of Time's beam; any other has no action and
//! draws nothing).
//! Flags: reads EVENTCHKINF_OPENED_DOOR_OF_TIME (type 1: gone once the door is open).

use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_DRAW_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind};
use oot_game::event_chk_inf;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::scene_table::{gfx_tex_scroll, gfx_two_tex_scroll};

/// `ACTOR_BG_TOKI_HIKARI` (`actor_table.h`: 0x006A).
pub const ACTOR_BG_TOKI_HIKARI: i16 = 0x006A;
pub const OBJECT: &str = "object_toki_objects";

/// `Bg_Toki_Hikari_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_TOKI_HIKARI, name: "Bg_Toki_Hikari", category: ACTORCAT_BG, flags: ACTOR_FLAG_DRAW_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `EVENTCHKINF_OPENED_DOOR_OF_TIME` (`save.h`: 0x4B).
const EVENTCHKINF_OPENED_DOOR_OF_TIME: u16 = 0x4B;

/// `switch (this->actor.params)`: what it is.
pub const TYPE: Field = Field::whole("params", Kind::Named(&[(0, "the windows"), (1, "the Door of Time's beam of light")]), "the windows, or the beam when the Door of Time opens");

/// Bg_Toki_Hikari's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE],
    flags: &[FlagUse::new(event_chk_inf!(EVENTCHKINF_OPENED_DOOR_OF_TIME)).reads().note("type 1: gone once the door is open")],
    todo: false,
    objects: &[],
    texts: &[],
};

/// The windows: the adult's (`object_toki_objects_DL_008190`), the child's
/// (`object_toki_objects_DL_007E20`) and the child's light (`object_toki_objects_DL_007EE0`,
/// segments 8 and 9 scrolling).
pub const ADULT_BAKE: &str = "Bg_Toki_Hikari/adult";
pub const CHILD_BAKE: &str = "Bg_Toki_Hikari/child";
pub const LIGHT_BAKE: &str = "Bg_Toki_Hikari/light";
/// The beam: `object_toki_objects_DL_000880` (segment 8 scrolling) and `_DL_0009C0` twice (the
/// second from its eleventh command), each under its colours (segment 10).
pub const BEAM_BAKES: [&str; 3] = ["Bg_Toki_Hikari/beam0", "Bg_Toki_Hikari/beam1", "Bg_Toki_Hikari/beam2"];
const SEG_COLOR: u8 = 0x0A;

/// `Gfx_TexScroll(.., 0, gameplayFrames % 128, 64, 32)`.
fn light_scroll(frames: u32) -> Vec<(u32, u32)> {
    gfx_tex_scroll(0, frames % 128, 64, 32)
}

/// `Gfx_TwoTexScroll(.., G_TX_RENDERTILE, -2 * (frames & 0x7F), 0, 0x20, 0x40, 1, (frames & 0x7F) * 4, 0, 0x20, 0x40)`.
fn beam_scroll(frames: u32) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, (frames & 0x7F).wrapping_mul(2).wrapping_neg(), 0, 0x20, 0x40, 1, (frames & 0x7F) * 4, 0, 0x20, 0x40)
}

/// The beam's colours for `unk_14C`, by piece: (prim's `l`, prim, env).
fn beam_colors(unk_14c: f32) -> [(u8, [u8; 4], [u8; 4]); 3] {
    let f = |x: f32| x as i32 as u8;
    let c0 = (0x80, [f(unk_14c * 255.0), f(155.0 * unk_14c).wrapping_add(100), f(unk_14c * 255.0), f(unk_14c * 255.0)], [f(unk_14c * 155.0).wrapping_add(100), f(255.0 * unk_14c), 0, 128]);
    let c1 = (0, [255, 255, 255, f(unk_14c * 200.0)], [f(unk_14c * 255.0), f(unk_14c * 255.0), f(unk_14c * 255.0), f(200.0 * unk_14c)]);
    [c0, c1, c1]
}

/// The meshes `BgTokiHikari_Draw` draws.
pub fn bakes() -> Vec<MeshBake> {
    let dl = |name: &str, symbol: &str, segments: Vec<(u8, BakeSegment)>, prelude: Vec<u8>| MeshBake {
        name: name.into(),
        object: OBJECT.into(),
        segments,
        prelude,
        body: BakeBody::DLists(vec![(OBJECT.into(), symbol.into())]),
    };
    let colors = |l: u8| BakeSegment::Dynamic(vec![(0xFA00_0000 | l as u32, 0x0000_00FF), (0xFB00_0000, 0x0000_00FF), (0xDF00_0000, 0)]);
    vec![
        dl(ADULT_BAKE, "object_toki_objects_DL_008190", Vec::new(), Vec::new()),
        dl(CHILD_BAKE, "object_toki_objects_DL_007E20", Vec::new(), Vec::new()),
        dl(LIGHT_BAKE, "object_toki_objects_DL_007EE0", vec![(0x08, BakeSegment::Dynamic(light_scroll(0))), (0x09, BakeSegment::Dynamic(light_scroll(0)))], Vec::new()),
        dl(BEAM_BAKES[0], "object_toki_objects_DL_000880", vec![(SEG_COLOR, colors(0x80)), (0x08, BakeSegment::Dynamic(beam_scroll(0)))], vec![SEG_COLOR]),
        dl(BEAM_BAKES[1], "object_toki_objects_DL_0009C0", vec![(SEG_COLOR, colors(0))], vec![SEG_COLOR]),
        // &object_toki_objects_DL_0009C0[10]: 10 commands of 8 bytes in.
        dl(BEAM_BAKES[2], "object_toki_objects_DL_0009C0+0x50", vec![(SEG_COLOR, colors(0))], vec![SEG_COLOR]),
    ]
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    DoNothing,
    /// `func_808BA204`: waiting for the door.
    Func808BA204,
    /// `func_808BA22C`: the beam growing.
    Func808BA22C,
    /// `func_808BA274`: the beam fading.
    Func808BA274,
}

pub struct BgTokiHikari {
    pub actor: Actor,
    /// `unk_14C`: the beam's length and brightness.
    pub unk_14c: f32,
    pub action: Action,
}

impl BgTokiHikari {
    /// `BgTokiHikari_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = BgTokiHikari { actor, unk_14c: 0.0, action: Action::DoNothing };
        match TYPE.get(this.actor.params) {
            0 => {
                // sInitChain: ICHAIN_VEC3F_DIV1000(scale, 1000).
                this.actor.scale = Vec3::ONE;
                this.action = Action::DoNothing;
            }
            1 => {
                if !play.save.get_event_chk_inf(EVENTCHKINF_OPENED_DOOR_OF_TIME) {
                    this.action = Action::Func808BA204;
                    this.unk_14c = 0.0;
                } else {
                    this.actor.kill();
                }
            }
            // Any other params keeps actionFunc NULL: the update would jump to address 0.
            p => log::warn!("Bg_Toki_Hikari: params {p} has no action (actionFunc NULL)"),
        }
        Box::new(this)
    }

    /// `func_808BA204`.
    fn func_808ba204(&mut self, play: &PlayState) {
        if play.scene.as_ref().is_some_and(|s| s.draw.room_draw_params[1] != 0) {
            self.action = Action::Func808BA22C;
        }
    }

    /// `func_808BA22C`.
    fn func_808ba22c(&mut self) {
        if self.unk_14c < 1.0 {
            self.unk_14c += 0.05;
        } else {
            self.unk_14c = 1.0;
            self.action = Action::Func808BA274;
        }
    }

    /// `func_808BA274`.
    fn func_808ba274(&mut self) {
        if self.unk_14c > 0.2 {
            self.unk_14c -= 0.025;
        } else {
            self.unk_14c = 0.0;
            self.actor.kill();
        }
    }
}

impl ActorImpl for BgTokiHikari {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `BgTokiHikari_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::DoNothing => {}
            Action::Func808BA204 => self.func_808ba204(play),
            Action::Func808BA22C => self.func_808ba22c(),
            Action::Func808BA274 => self.func_808ba274(),
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.values = vec![self.unk_14c];
        rs
    }
    /// `BgTokiHikari_Draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let frames = play.gameplay_frames;
        match TYPE.get(self.actor.params) {
            0 => {
                // func_808BA018.
                let m = oot_game::play::actor_draw_matrix(rs);
                if play.save.adult {
                    out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(ADULT_BAKE)), m));
                } else {
                    out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(CHILD_BAKE)), m));
                    let mut sv = SegmentValues::default();
                    sv.read(0x08, &light_scroll(frames));
                    sv.read(0x09, &light_scroll(frames));
                    out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(LIGHT_BAKE)), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
                }
            }
            1 => {
                // func_808BA2CC: Translate(0, 276, 1122) (MTXMODE_NEW), Scale(0.32, 0.32, unk_14C * 7), RotateZ(M_PI).
                let unk_14c = rs.values.first().copied().unwrap_or(self.unk_14c);
                let m = Mat4::from_translation(Vec3::new(0.0, 276.0, 1122.0)) * Mat4::from_scale(Vec3::new(0.32, 0.32, unk_14c * 7.0)) * Mat4::from_rotation_z(std::f32::consts::PI);
                for (i, (_, prim, env)) in beam_colors(unk_14c).into_iter().enumerate() {
                    let mut sv = SegmentValues::default();
                    sv.prim[SEG_COLOR as usize] = Some(prim);
                    sv.env[SEG_COLOR as usize] = Some(env);
                    if i == 0 {
                        sv.read(0x08, &beam_scroll(frames));
                    }
                    out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(BEAM_BAKES[i])), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
                }
            }
            _ => {}
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
