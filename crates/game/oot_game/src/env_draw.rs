//! The sky's draws around the rooms (`z_kankyo.c`, GAME-06 milestone 3,
//! docs/adr/0056-the-clock.md): what `Play_Draw` draws of the environment besides the sky
//! (`crate::skybox`) and the weather (`crate::weather`).
//!
//! - `Environment_DrawSunAndMoon`, after the sky: the sun by the time of day (`gSunDL` after
//!   `SETUPDL_54`, tinted and faded as it sets, 10 to 12 big) and the moon opposite it (`gMoonDL`
//!   after `SETUPDL_51`, faded in below the horizon); the sun's place eased in a cutscene.
//! - `Environment_DrawSkyboxFilters`: the fog's colour over the sky (opaque from a `fogNear` of
//!   950 down, none from 980; always over `SKYBOX_UNSET_1D`, the forests' missing sky), and a
//!   custom fill. With the lightning's flash (`Environment_DrawLightningFlash`) they're
//!   `gDPFillRectangle`s after `SETUPDL_57`, drawn here as one screen-space quad bake
//!   (`FILL_BAKE`) in the list where the C draws them: under the rooms.
//! - `Environment_DrawSunLensFlare` after the actors: ten circles and a ring along the line from
//!   the sun through the view (`gLensFlareCircleDL`, `gLensFlareRingDL` on the billboard after
//!   `SETUPDL_65`), and the glare over the screen, both eased in and out
//!   (`lensFlareAlphaScale`, `glareAlpha`) and hidden when the sun is behind something: the
//!   depth read at the sun after the last frame (`Environment_GraphCallback`'s
//!   `sSunScreenDepth`) isn't the far plane's. The renderer reads it back
//!   (`eng_gfx::DrawLists::depth_probe`); `PlayState::environment_graph_callback` takes it.
//!
//! The state is updated once per game frame at `Play_Draw`'s time (`environment_draw_update`),
//! the draws made from it with the render's eye (`draw_environment`, `draw_lens_flares`).
//! Not drawn: `Environment_DrawCustomLensFlare` (`gCustomLensFlareOn`: Ganon's tower's and the
//! like, none ported).

use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use glam::{Mat4, Vec3};

use crate::clock::sun_pos;
use crate::env::{PRECIP_RAIN_CUR, clock_time, lerp_weight};
use crate::gbi::{Dl, push_vtx, seg, setup_dl};
use crate::pack::{BakeBody, BakeSegment, MeshBake, keys};
use crate::play::{DrawOut, PlayState, ViewInfo};

const SUN_BAKE: &str = "Environment/sun";
const MOON_BAKE: &str = "Environment/moon";
const FLARE_CIRCLE_BAKE: &str = "Environment/lens_flare_circle";
const FLARE_RING_BAKE: &str = "Environment/lens_flare_ring";
/// A screen fill: `SETUPDL_57`, the prim colour, a quad over the screen.
const FILL_BAKE: &str = "Environment/fill";

const SEG_VTX: u8 = 0x09;
const SEG_SETUP: u8 = 0x0A;
/// The dynamic prim and env colours (`SegmentValues::prim[0x0B]`, `env[0x0B]`).
const SEG_COLOR: u8 = 0x0B;
const SEG_DRAW: u8 = 0x0D;
/// `D_01000000`: the billboard matrix (`play->billboardMtx`).
const SEG_BILLBOARD: u8 = 0x01;

/// `GPACK_ZDZ(G_MAXFBZ, 0)`: the depth of a pixel nothing was drawn over.
pub const ZBUF_MAX: u16 = 0xFFFC;
/// `ENV_FOGNEAR_MAX` (`light.h`).
const ENV_FOGNEAR_MAX: f32 = 996.0;
/// `SCREEN_WIDTH`, `SCREEN_HEIGHT`.
const SCREEN_WIDTH: f32 = 320.0;
const SCREEN_HEIGHT: f32 = 240.0;

/// `sLensFlareScales`.
const LENS_FLARE_SCALES: [f32; 10] = [23.0, 12.0, 7.0, 5.0, 3.0, 10.0, 6.0, 2.0, 3.0, 1.0];
/// `Environment_DrawLensFlare`'s `lensFlareColors`.
const LENS_FLARE_COLORS: [[u8; 3]; 10] =
    [[155, 205, 255], [255, 255, 205], [255, 255, 205], [255, 255, 205], [155, 255, 205], [205, 255, 255], [155, 155, 255], [205, 175, 255], [175, 255, 205], [255, 155, 235]];
/// `lensFlareAlphas`.
const LENS_FLARE_ALPHAS: [u32; 10] = [50, 10, 25, 40, 70, 30, 50, 70, 50, 40];
/// `lensFlareTypes`: `LENS_FLARE_RING` first, `LENS_FLARE_CIRCLE1` after.
const LENS_FLARE_RING: [bool; 10] = [true, false, false, false, false, false, false, false, false, false];

/// The environment's meshes.
pub fn bakes() -> Vec<MeshBake> {
    let keep = "gameplay_keep";
    let color = |env: bool| (SEG_COLOR, BakeSegment::DynamicColor { env, prim: true });
    let setup = |mut d: Dl| {
        d.end();
        (SEG_SETUP, BakeSegment::Commands(d.0))
    };
    // func_800947AC (SETUPDL_65, gDPSetColorDither(G_CD_DISABLE)), then
    // gDPSetCombineLERP(0, 0, 0, PRIMITIVE, TEXEL0, 0, PRIMITIVE, 0, ..) in both cycles,
    // gDPSetAlphaDither(G_AD_DISABLE), gDPSetColorDither(G_CD_DISABLE).
    let flare_setup = || {
        use crate::gbi::{ac, cc_ab, cc_c, cc_d};
        let mut d = setup_dl::setup_dl_65();
        d.color_dither(crate::gbi::G_CD_DISABLE);
        let c = [cc_ab::ZERO, cc_ab::ZERO, cc_c::ZERO, cc_d::PRIMITIVE, ac::TEXEL0, ac::ZERO, ac::PRIMITIVE, ac::ZERO];
        d.combine_lerp(c, c);
        // gDPSetAlphaDither: G_SETOTHERMODE_H from G_MDSFT_ALPHADITHER (4), 2 bits, G_AD_DISABLE.
        d.othermode_h(4, 2, 0x30);
        d.color_dither(crate::gbi::G_CD_DISABLE);
        setup(d)
    };
    // The unit quad from (0, 0) to (1, -1), placed by the draw (crate::sprite's way).
    let mut vtx = Vec::new();
    for ob in [[0, 0, 0], [1, 0, 0], [1, -1, 0], [0, -1, 0]] {
        push_vtx(&mut vtx, ob, [0, 0], [255, 255, 255, 255]);
    }
    let mut quad = Dl::default();
    quad.vertex(seg(SEG_VTX as u32, 0), 4, 0);
    quad.quad0(0, 1, 2, 3);
    quad.end();
    vec![
        MeshBake {
            name: SUN_BAKE.into(),
            object: keep.into(),
            // gSunDL's own gSPMatrix(D_01000000): its vertices on the billboard (bone 0).
            segments: vec![color(true), setup(setup_dl::setup_dl_54()), (SEG_BILLBOARD, BakeSegment::Matrix(0))],
            prelude: vec![SEG_COLOR, SEG_SETUP],
            body: BakeBody::DLists(vec![(keep.into(), "gSunDL".into())]),
        },
        MeshBake {
            name: MOON_BAKE.into(),
            object: keep.into(),
            // gMoonDL's own gSPMatrix(D_01000000), as the sun's.
            segments: vec![setup(setup_dl::setup_dl_51()), color(true), (SEG_BILLBOARD, BakeSegment::Matrix(0))],
            prelude: vec![SEG_SETUP, SEG_COLOR],
            body: BakeBody::DLists(vec![(keep.into(), "gMoonDL".into())]),
        },
        MeshBake {
            name: FLARE_CIRCLE_BAKE.into(),
            object: keep.into(),
            segments: vec![flare_setup(), color(false)],
            prelude: vec![SEG_SETUP, SEG_COLOR],
            body: BakeBody::DLists(vec![(keep.into(), "gLensFlareCircleDL".into())]),
        },
        MeshBake {
            name: FLARE_RING_BAKE.into(),
            object: keep.into(),
            segments: vec![flare_setup(), color(false)],
            prelude: vec![SEG_SETUP, SEG_COLOR],
            body: BakeBody::DLists(vec![(keep.into(), "gLensFlareRingDL".into())]),
        },
        MeshBake {
            name: FILL_BAKE.into(),
            object: keep.into(),
            segments: vec![setup(setup_dl::setup_dl_57()), color(false), (SEG_VTX, BakeSegment::Bytes(vtx)), (SEG_DRAW, BakeSegment::Commands(quad.0))],
            prelude: vec![SEG_SETUP, SEG_COLOR, SEG_DRAW],
            body: BakeBody::DLists(Vec::new()),
        },
    ]
}

/// The sun or the moon this frame: where from the eye, how big, its colours.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SunMoonDraw {
    pub pos: Vec3,
    pub scale: f32,
    pub prim: [u8; 4],
    pub env: [u8; 4],
}

/// One of the lens flare's ten: where from the eye, how big, its colour, a ring or a circle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlareDraw {
    pub offset: Vec3,
    pub scale: f32,
    pub color: [u8; 4],
    pub ring: bool,
}

/// What `Play_Draw` draws of the environment this frame (`PlayState::env_draw`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EnvDraw {
    pub sun: Option<SunMoonDraw>,
    pub moon: Option<SunMoonDraw>,
    /// `Environment_DrawSkyboxFilters`' fills, in order.
    pub skybox_filters: Vec<[u8; 4]>,
    pub lens_flares: Vec<FlareDraw>,
    /// The glare's fill over the screen.
    pub glare: Option<[u8; 4]>,
}

/// A fill over the whole screen (`gDPFillRectangle(0, 0, SCREEN_WIDTH - 1, SCREEN_HEIGHT - 1)`),
/// in the screen's space, wide enough for a wider target.
pub fn fill_cmd(color: [u8; 4]) -> DrawCmd {
    let mut sv = SegmentValues::default();
    sv.prim[SEG_COLOR as usize] = Some(color);
    let params = DrawParams { segments: Some(sv), screen: true, ..Default::default() };
    DrawCmd { mesh: MeshKey::named(keys::bake(FILL_BAKE)), transform: crate::sprite::rect_transform(-10000.0, 0.0, 10320.0, SCREEN_HEIGHT), bones: Vec::new(), params }
}

/// The sun's or the moon's draw: their lists load the billboard themselves
/// (`gSPMatrix(D_01000000, G_MTX_MUL)`), so the whole matrix is bone 0's.
fn billboarded(bake: &str, m: Mat4, prim: [u8; 4], env: [u8; 4]) -> DrawCmd {
    let mut c = colored(bake, Mat4::IDENTITY, prim, Some(env));
    c.bones = vec![m];
    c
}

fn colored(bake: &str, transform: Mat4, prim: [u8; 4], env: Option<[u8; 4]>) -> DrawCmd {
    let mut sv = SegmentValues::default();
    sv.prim[SEG_COLOR as usize] = Some(prim);
    sv.env[SEG_COLOR as usize] = env;
    DrawCmd { mesh: MeshKey::named(keys::bake(bake)), transform, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } }
}

impl PlayState {
    /// `Play_Draw`'s environment before the rooms, once per game frame: the sun and the moon
    /// (unless the room disables them), the skybox filters.
    pub fn environment_draw_update(&mut self) {
        if !self.env_ctx.sun_moon_disabled {
            self.environment_draw_sun_and_moon();
        } else {
            self.env_draw.sun = None;
            self.env_draw.moon = None;
        }
        self.environment_draw_skybox_filters();
    }

    /// `Environment_DrawSunAndMoon`: the sun's place by the time (eased by 0.8 a frame in a
    /// cutscene; @bug (game): the third ease is `y`'s again, towards `z`'s place, so `z` stays);
    /// unless in Hyrule Field's layer 5 from `ENTR_HYRULE_FIELD_0`, the sun (yellower and less
    /// faded the higher it is, 10 to 12 big) and the moon opposite (fading in below the horizon,
    /// 25 down to 10 big).
    pub fn environment_draw_sun_and_moon(&mut self) {
        let target = sun_pos(self.save.day_time);
        let p = &mut self.env_ctx.sun_pos;
        if self.cs_ctx.state != crate::cutscene::CS_STATE_IDLE {
            eng_math::smooth_step_to_f(&mut p.x, target.x, 1.0, 0.8, 0.8);
            eng_math::smooth_step_to_f(&mut p.y, target.y, 1.0, 0.8, 0.8);
            eng_math::smooth_step_to_f(&mut p.y, target.z, 1.0, 0.8, 0.8);
        } else {
            *p = target;
        }
        let field_0 = self.assets.as_ref().and_then(|a| a.scenes.entrance_index("ENTR_HYRULE_FIELD_0"));
        if Some(self.save.entrance_index) == field_0 && self.save.scene_layer == 5 {
            self.env_draw.sun = None;
            self.env_draw.moon = None;
            return;
        }
        let sp = self.env_ctx.sun_pos;
        let y = sp.y / 25.0;
        let temp = y / 80.0;
        let alpha = 255.0 - (temp * 255.0).clamp(0.0, 255.0);
        let color = temp.clamp(0.0, 1.0);
        self.env_draw.sun = Some(SunMoonDraw {
            pos: sp,
            scale: color * 2.0 + 10.0,
            prim: [255, ((color * 75.0) as u8).wrapping_add(180), ((color * 155.0) as u8).wrapping_add(100), 255],
            env: [255, (color * 255.0) as u8, (color * 255.0) as u8, alpha as u8],
        });
        let color = (-y / 120.0).max(0.0);
        let scale = -15.0 * color + 25.0;
        let alpha = (-y / 80.0).min(1.0) * 255.0;
        self.env_draw.moon = (alpha > 0.0).then(|| SunMoonDraw { pos: -sp, scale, prim: [240, 255, 180, alpha as u8], env: [80, 70, 20, alpha as u8] });
    }

    /// `Environment_DrawSkyboxFilters`: the fog's colour over the sky with a sky and a `fogNear`
    /// under 980 (its alpha `(1000 - fogNear) * 0.02`, at most 1), or opaque over
    /// `SKYBOX_UNSET_1D`; then the custom filter.
    pub fn environment_draw_skybox_filters(&mut self) {
        use crate::skybox::{SKYBOX_NONE, SKYBOX_UNSET_1D};
        self.env_draw.skybox_filters.clear();
        let id = self.skybox_ctx.skybox_id;
        let Some(l) = self.scene.as_ref().map(|s| s.lights) else { return };
        if (id != SKYBOX_NONE && l.fog_near < 980) || id == SKYBOX_UNSET_1D {
            let mut alpha = (1000 - l.fog_near as i32) as f32 * 0.02;
            if id == SKYBOX_UNSET_1D {
                alpha = 1.0;
            }
            let alpha = alpha.min(1.0);
            self.env_draw.skybox_filters.push([l.fog_color[0], l.fog_color[1], l.fog_color[2], (255.0 * alpha) as u8]);
        }
        if self.env_ctx.custom_skybox_filter {
            self.env_draw.skybox_filters.push(self.env_ctx.skybox_filter_color);
        }
    }

    /// `Play_Draw`'s lens flares after `Actor_DrawAll`, once per game frame: the sun's
    /// (`Environment_DrawSunLensFlare`, unless the room disables the sun: with no rain and the
    /// first sky config, at the sun, its colour by the time, glare 400).
    pub fn environment_draw_lens_flares_update(&mut self) {
        self.env_draw.lens_flares.clear();
        self.env_draw.glare = None;
        if self.env_ctx.sun_moon_disabled {
            return;
        }
        if self.env_ctx.precipitation[PRECIP_RAIN_CUR] == 0 && self.env_ctx.skybox_config == 0 {
            let pos = self.view.eye + self.env_ctx.sun_pos;
            let t = self.save.day_time.wrapping_sub(clock_time(12, 0) as u16) as i16;
            self.environment_draw_lens_flare(pos, 370, eng_math::cos_s(t) * 120.0, 400, true);
        }
    }

    /// `Environment_DrawLensFlare` at `pos`: the flares along the line from `pos` back through a
    /// point half as far ahead of the eye, spaced by a twelfth of the distance, bigger and
    /// brighter the nearer `pos` is to the view's centre (`cosAngle`), dimmer in thick fog; for
    /// the sun, hidden while it's off screen or behind something (the depth read there after
    /// the last frame, `sSunScreenDepth`), with the pixel 5 above it read after this one. The
    /// alpha scale eases by 0.5 (at most 0.05) towards its target ten times a frame, once per
    /// flare; the glare (with `glare_strength`) by 0.5 (at most 50) once.
    pub fn environment_draw_lens_flare(&mut self, pos: Vec3, scale: i16, color_intensity: f32, glare_strength: i16, is_sun: bool) {
        let (eye, at) = (self.view.eye, self.view.at);
        // Math3D_Vec3f_DistXYZ.
        let dist = pos.distance(eye) / 12.0;
        let t = at - eye;
        let length = (t.x * t.x + t.y * t.y + t.z * t.z).sqrt();
        let look = t / length;
        let half = eye + look * (dist * 6.0);
        let t2 = pos - half;
        let length = (t2.x * t2.x + t2.y * t2.y + t2.z * t2.z).sqrt();
        let dir = t2 / length;
        let cos_angle = (look.x * dir.x + look.y * dir.y + look.z * dir.z) / ((look.x * look.x + look.y * look.y + look.z * look.z) * (dir.x * dir.x + dir.y * dir.y + dir.z * dir.z)).sqrt();
        let mut target = (cos_angle * 3.5).min(1.0);
        if !is_sun {
            target = cos_angle;
        }
        if cos_angle < 0.0 {
            // Nothing drawn, nothing eased.
            return;
        }
        let mut off_screen = false;
        if is_sun {
            let sp = self.play_get_screen_pos(pos);
            self.env_statics.sun_depth_test = [sp.x as i16, ((sp.y as i16) as f32 - 5.0) as i16];
            if self.env_statics.sun_screen_depth != ZBUF_MAX || sp.x < 0.0 || sp.y < 0.0 || sp.x > SCREEN_WIDTH || sp.y > SCREEN_HEIGHT {
                off_screen = true;
            }
        }
        let fog_near = self.scene.as_ref().map_or(0, |s| s.lights.fog_near) as f32;
        let fog_influence = ((ENV_FOGNEAR_MAX - fog_near) / 50.0).min(1.0);
        let fovy = self.view.fov as u16;
        for i in 0..LENS_FLARE_RING.len() {
            let offset = pos - eye - dir * (i as f32) * dist;
            let mut adj_scale = LENS_FLARE_SCALES[i] * cos_angle;
            if is_sun {
                let temp = lerp_weight(60, 15, fovy);
                adj_scale = (adj_scale as f64 * (0.001 * (scale as f32 + 630.0 * temp) as f64)) as f32;
            } else {
                adj_scale *= 0.0001 * scale as f32 * (2.0 * dist);
            }
            let alpha = ((color_intensity / 10.0).min(1.0) * LENS_FLARE_ALPHAS[i] as f32).max(0.0) * (1.0 - fog_influence);
            let s = &mut self.env_ctx.lens_flare_alpha_scale;
            eng_math::smooth_step_to_f(s, if off_screen { 0.0 } else { target }, 0.5, 0.05, 0.001);
            let c = LENS_FLARE_COLORS[i];
            self.env_draw.lens_flares.push(FlareDraw { offset, scale: adj_scale, color: [c[0], c[1], c[2], (alpha * *s) as u8], ring: LENS_FLARE_RING[i] });
        }
        let glare_alpha_scale = cos_angle - (1.5 - cos_angle);
        if glare_strength != 0 {
            if glare_alpha_scale > 0.0 {
                let alpha = ((color_intensity / 10.0).min(1.0) * glare_strength as f32).max(0.0) * (1.0 - fog_influence);
                let g = &mut self.env_ctx.glare_alpha;
                eng_math::smooth_step_to_f(g, if off_screen { 0.0 } else { alpha * glare_alpha_scale }, 0.5, 50.0, 0.1);
                let temp = (color_intensity / 120.0).max(0.0);
                self.env_draw.glare = Some([255, ((temp * 75.0) as u8).wrapping_add(180), ((temp * 155.0) as u8).wrapping_add(100), *g as u8]);
            } else {
                self.env_ctx.glare_alpha = 0.0;
            }
        }
    }

    /// `Play_GetScreenPos`: `pos` through `viewProjectionMtxF` (`play->view`'s), onto the 320x240
    /// screen (y down).
    pub fn play_get_screen_pos(&self, pos: Vec3) -> Vec3 {
        let v = self.view;
        let view_proj = eng_math::gu_perspective(v.fov, 4.0 / 3.0, 10.0, 12800.0) * glam::camera::rh::view::look_at_mat4(v.eye, v.at, Vec3::Y);
        let c = view_proj * pos.extend(1.0);
        Vec3::new(SCREEN_WIDTH / 2.0 + (c.x / c.w) * (SCREEN_WIDTH / 2.0), SCREEN_HEIGHT / 2.0 - (c.y / c.w) * (SCREEN_HEIGHT / 2.0), c.z)
    }

    /// `Environment_GraphCallback` (once the frame is drawn): `sSunScreenDepth` from the depth
    /// the renderer read at `sun_depth_test` (`Environment_GetPixelDepth`): the far plane's, or
    /// something drawn (a pixel off the screen reads as drawn over).
    pub fn environment_graph_callback(&mut self, depth: Option<f32>) {
        self.env_statics.sun_screen_depth = if depth.is_some_and(|d| d >= 1.0) { ZBUF_MAX } else { 0 };
    }

    /// The environment `Play_Draw` draws between the sky and the rooms (into `out.opa`): the sun
    /// and the moon at the eye (`view.eye`, the render's), the skybox filters, the lightning's
    /// flash.
    pub fn draw_environment(&self, view: &ViewInfo, out: &mut DrawOut) {
        let d = &self.env_draw;
        if let Some(s) = d.sun {
            out.opa.push(billboarded(SUN_BAKE, Mat4::from_translation(view.eye + s.pos) * Mat4::from_scale(Vec3::splat(s.scale)) * view.billboard, s.prim, s.env));
        }
        if let Some(m) = d.moon {
            out.opa.push(billboarded(MOON_BAKE, Mat4::from_translation(view.eye + m.pos) * Mat4::from_scale(Vec3::splat(m.scale)) * view.billboard, m.prim, m.env));
        }
        for &f in &d.skybox_filters {
            out.opa.push(fill_cmd(f));
        }
        if let Some(f) = self.lightning_flash {
            out.opa.push(fill_cmd(f));
        }
    }

    /// The lens flares after the actors (into `out.xlu`): each on the billboard
    /// (`gSPMatrix(&D_01000000)`), then the glare's fill.
    pub fn draw_lens_flares(&self, view: &ViewInfo, out: &mut DrawOut) {
        let d = &self.env_draw;
        for f in &d.lens_flares {
            let m = Mat4::from_translation(view.eye + f.offset) * Mat4::from_scale(Vec3::splat(f.scale)) * view.billboard;
            out.xlu.push(colored(if f.ring { FLARE_RING_BAKE } else { FLARE_CIRCLE_BAKE }, m, f.color, None));
        }
        if let Some(g) = d.glare {
            out.xlu.push(fill_cmd(g));
        }
    }
}
