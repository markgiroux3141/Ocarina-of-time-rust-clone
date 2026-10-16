//! Scene draw configs (`Scene_DrawConfig*` in `z_scene_table.c`), ported: what each scene's
//! draw config puts in the dynamic segments every frame (texture scroll tile sizes, env and
//! prim colours), which the renderer applies to the room meshes' dynamic materials.
//!
//! The meshes themselves are built at import time with the draw config's output for the
//! layer (segment bindings to static data, and the geometry of any display list it calls).
//! At runtime only the per-frame values matter; `segment_values` returns them for the ported
//! configs, and `None` for the others, whose materials then keep their import-time (frame 0)
//! values. `oot_import`'s tests check each port against the C interpreter (`drawcfg`).
//!
//! The display lists are built with the `z_rcp.c` helpers the configs call (`Gfx_TexScroll`,
//! `Gfx_TwoTexScroll`) and read back with `SegmentValues::read`, the same way the import-time
//! values were.
//!
//! Some configs point a segment at one of two textures by the time of day
//! (`gSPSegment(.., SEGMENTED_TO_VIRTUAL(sKakarikoWindowTextures[nightFlag]))`). The meshes are
//! baked with the layer's choice (day for the day layers, night for the night ones), but the C
//! changes it on the frame `nightFlag` does, inside the layer: `segment_textures` says which
//! texture each such segment holds this frame, and the room draw puts it in place of what the
//! mesh loaded from that segment (`DrawParams::texture_images`, by the load's source address).

use std::collections::BTreeMap;

use eng_gfx::SegmentValues;
use eng_math::step_to_s;

use crate::env::clock_time;

/// What a draw config reads: the frame counter and save state, and `roomCtx.drawParams`, which
/// some configs keep their own state in across frames.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DrawConfigState {
    /// `play->gameplayFrames`.
    pub gameplay_frames: u32,
    /// `LINK_IS_CHILD`.
    pub child: bool,
    /// `gSaveContext.save.nightFlag`.
    pub night: bool,
    /// `gSaveContext.sceneLayer`.
    pub scene_layer: usize,
    /// `gSaveContext.save.dayTime`.
    pub day_time: u16,
    /// `play->roomCtx.drawParams` ("context-specific data used by the current scene draw config",
    /// `z64.h`). Zeroed on scene load (`Room_Init` clears the room context).
    pub room_draw_params: [i16; 2],
    /// `GET_EVENTCHKINF(EVENTCHKINF_07)`.
    pub event_chk_inf_07: bool,
    /// `INV_CONTENT(ITEM_COJIRO) == ITEM_COJIRO`.
    pub cojiro: bool,
    /// Out: the Lost Woods' config asked for `Player_PlaySfx(NA_SE_EV_CHICKEN_CRY_M)` this
    /// frame (the caller plays it and clears this).
    pub chicken_cry: bool,
    /// `play->sceneId`.
    pub scene_id: u16,
    /// Out: the Temple of Time's config on layer 5 turned the custom lens flare on
    /// (`gCustomLensFlareOn`, `gCustomLensFlarePos`, `gLensFlareScale`,
    /// `gLensFlareColorIntensity`, `gLensFlareGlareStrength`). Nothing draws it yet
    /// (`Environment_DrawCustomLensFlare` isn't ported, `crate::env_draw`).
    pub custom_lens_flare: Option<CustomLensFlare>,
}

/// The custom lens flare's globals (`z_kankyo.c`), as a draw config sets them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CustomLensFlare {
    pub pos: [f32; 3],
    pub scale: i16,
    pub color_intensity: f32,
    pub glare_strength: i16,
}

/// `SCENE_CASTLE_COURTYARD_GUARDS_DAY` (`scene_table.h`).
const SCENE_CASTLE_COURTYARD_GUARDS_DAY: u16 = 0x45;

impl DrawConfigState {
    /// `IS_CUTSCENE_LAYER` (`macros.h`): `gSaveContext.sceneLayer > 3`.
    fn is_cutscene_layer(&self) -> bool {
        self.scene_layer > 3
    }
}

/// A display list under construction (`Graph_Alloc`'d `Gfx`s).
type Dl = Vec<(u32, u32)>;

/// The segments a draw config set in the OPA and XLU buffers to display lists it built.
/// (Segments set to pointers into the scene's data don't change per frame and aren't here.)
#[derive(Default)]
struct Buffers {
    opa: BTreeMap<u8, Dl>,
    xlu: BTreeMap<u8, Dl>,
}

impl Buffers {
    fn values(&self) -> [SegmentValues; 2] {
        [&self.opa, &self.xlu].map(|b| {
            let mut v = SegmentValues::default();
            for (&s, dl) in b {
                v.read(s, dl);
            }
            v
        })
    }
}

const G_TX_RENDERTILE: u32 = 0;

fn g_dp_pipe_sync() -> (u32, u32) {
    (0xE700_0000, 0)
}
pub(crate) fn g_dp_tile_sync() -> (u32, u32) {
    (0xE800_0000, 0)
}
pub(crate) fn g_sp_end_display_list() -> (u32, u32) {
    (0xDF00_0000, 0)
}
/// `gDPSetTileSize` (`gbi.h`: `G_SETTILESIZE`, 12-bit fields).
pub(crate) fn g_dp_set_tile_size(tile: u32, uls: u32, ult: u32, lrs: u32, lrt: u32) -> (u32, u32) {
    (0xF200_0000 | ((uls & 0xFFF) << 12) | (ult & 0xFFF), ((tile & 7) << 24) | ((lrs & 0xFFF) << 12) | (lrt & 0xFFF))
}
/// `gDPSetEnvColor`: each component through `_SHIFTL(v, s, 8)`, i.e. `(u32)v & 0xFF`.
fn g_dp_set_env_color(r: u32, g: u32, b: u32, a: u32) -> (u32, u32) {
    (0xFB00_0000, ((r & 0xFF) << 24) | ((g & 0xFF) << 16) | ((b & 0xFF) << 8) | (a & 0xFF))
}
/// `gDPSetPrimColor(pkt, m, l, r, g, b, a)`.
fn g_dp_set_prim_color(m: u32, l: u32, r: u32, g: u32, b: u32, a: u32) -> (u32, u32) {
    (0xFA00_0000 | ((m & 0xFF) << 8) | (l & 0xFF), ((r & 0xFF) << 24) | ((g & 0xFF) << 16) | ((b & 0xFF) << 8) | (a & 0xFF))
}

/// `Gfx_TexScroll` (`z_rcp.c`).
pub fn gfx_tex_scroll(x: u32, y: u32, width: i32, height: i32) -> Dl {
    let (x, y) = (x % (512 << 2), y % (512 << 2));
    vec![
        g_dp_tile_sync(),
        g_dp_set_tile_size(G_TX_RENDERTILE, x, y, x.wrapping_add(((width - 1) << 2) as u32), y.wrapping_add(((height - 1) << 2) as u32)),
        g_sp_end_display_list(),
    ]
}

/// `Gfx_TwoTexScroll` (`z_rcp.c`).
#[allow(clippy::too_many_arguments)]
pub fn gfx_two_tex_scroll(tile1: u32, x1: u32, y1: u32, width1: i32, height1: i32, tile2: u32, x2: u32, y2: u32, width2: i32, height2: i32) -> Dl {
    let (x1, y1, x2, y2) = (x1 % (512 << 2), y1 % (512 << 2), x2 % (512 << 2), y2 % (512 << 2));
    vec![
        g_dp_tile_sync(),
        g_dp_set_tile_size(tile1, x1, y1, x1.wrapping_add(((width1 - 1) << 2) as u32), y1.wrapping_add(((height1 - 1) << 2) as u32)),
        g_dp_tile_sync(),
        g_dp_set_tile_size(tile2, x2, y2, x2.wrapping_add(((width2 - 1) << 2) as u32), y2.wrapping_add(((height2 - 1) << 2) as u32)),
        g_sp_end_display_list(),
    ]
}

/// `Scene_DrawConfigDefault`: `sDefaultDisplayList` into both buffers, no segments.
fn draw_config_default(_st: &mut DrawConfigState, _b: &mut Buffers) {}

/// `Scene_DrawConfigHyruleField` (Hyrule Field): the river's scrolling textures on segments 8 and
/// 9, and at night a display list on segment 0xA that fades the lit-window overlay
/// (`spot00_room_0DL_012B20`) in with `roomCtx.drawParams[0]` as its prim alpha.
fn draw_config_spot00(st: &mut DrawConfigState, b: &mut Buffers) {
    let f = st.gameplay_frames;
    b.xlu.insert(0x08, gfx_two_tex_scroll(G_TX_RENDERTILE, 127 - f % 128, f.wrapping_mul(3) % 128, 32, 32, 1, f % 128, f.wrapping_mul(3) % 128, 32, 32));
    b.xlu.insert(0x09, gfx_two_tex_scroll(G_TX_RENDERTILE, 127 - f % 128, f.wrapping_mul(10) % 128, 32, 32, 1, f % 128, f.wrapping_mul(10) % 128, 32, 32));
    // gDPSetEnvColor(POLY_OPA_DISP / POLY_XLU_DISP, 128, 128, 128, 128): straight into the
    // buffers, so already in the meshes.
    let t = st.day_time as i32;
    let dl = if t > clock_time(7, 0) && t <= clock_time(18, 30) {
        vec![g_sp_end_display_list()]
    } else {
        if t > clock_time(18, 30) {
            if st.room_draw_params[0] != 255 {
                step_to_s(&mut st.room_draw_params[0], 255, 5);
            }
        } else if t >= clock_time(6, 0) && st.room_draw_params[0] != 0 {
            step_to_s(&mut st.room_draw_params[0], 0, 10);
        }
        // The gSPDisplayList(spot00_room_0DL_012B20) between them draws geometry that is in
        // the meshes; SegmentValues::read skips it.
        vec![g_dp_set_prim_color(0, 0, 255, 255, 255, st.room_draw_params[0] as u32), (0xDE00_0000, 0), g_sp_end_display_list()]
    };
    b.xlu.insert(0x0A, dl);
}

/// `Scene_DrawConfigKokiriForest` (Kokiri Forest): the stream and waterfall scroll on segments 8
/// and 9, env colours on 0xA (alpha `spA3`) and 0xB (alpha `spA0 * 0.1`), and a scroll by
/// `roomCtx.drawParams[0]` on 0xC.
fn draw_config_spot04(st: &mut DrawConfigState, b: &mut Buffers) {
    let mut sp_a3: u8 = 128;
    let mut sp_a0: u16 = 500;
    let f = st.gameplay_frames;
    b.xlu.insert(0x09, gfx_two_tex_scroll(G_TX_RENDERTILE, 127 - f % 128, f % 128, 32, 32, 1, f % 128, f % 128, 32, 32));
    b.xlu.insert(0x08, gfx_two_tex_scroll(G_TX_RENDERTILE, 127 - f % 128, f.wrapping_mul(10) % 128, 32, 32, 1, f % 128, f.wrapping_mul(10) % 128, 32, 32));
    if st.scene_layer == 4 {
        sp_a3 = 255u8.wrapping_sub(st.room_draw_params[0] as u8);
    } else if st.scene_layer == 6 {
        sp_a0 = (st.room_draw_params[0] as i32 + 500) as u16;
    } else if (!st.is_cutscene_layer() || !st.child) && st.event_chk_inf_07 {
        sp_a0 = 2150;
    }
    let seg_a = vec![g_dp_pipe_sync(), g_dp_set_env_color(128, 128, 128, sp_a3 as u32), g_sp_end_display_list()];
    // The float `spA0 * 0.1f` goes through `_SHIFTL`'s `(u32)` cast.
    let seg_b = vec![g_dp_pipe_sync(), g_dp_set_env_color(128, 128, 128, (sp_a0 as f32 * 0.1) as u32), g_sp_end_display_list()];
    b.opa.insert(0x0A, seg_a);
    b.xlu.insert(0x0B, seg_b.clone());
    b.opa.insert(0x0B, seg_b);
    let scroll = (-(st.room_draw_params[0] as f32) * 0.02) as i16 as u32;
    b.opa.insert(0x0C, gfx_two_tex_scroll(G_TX_RENDERTILE, 0, scroll, 32, 16, 1, 0, scroll, 32, 16));
}

/// `Scene_DrawConfigDekuTree` (the Deku Tree): a scroll on segment 9. Segment 8 points at the
/// day or night texture (`sDekuTreeEntranceTextures[nightFlag]`), which is static data in the meshes.
fn draw_config_ydan(st: &mut DrawConfigState, b: &mut Buffers) {
    let f = st.gameplay_frames;
    b.xlu.insert(0x09, gfx_two_tex_scroll(G_TX_RENDERTILE, 127 - (f % 128), f % 128, 32, 32, 1, f % 128, f % 128, 32, 32));
}

/// `Scene_DrawConfigGerudoValley` (Gerudo Valley): the river and the waterfalls' scrolls on
/// segments 8 to 0xD (0xB in the OPA buffer).
fn draw_config_spot09(st: &mut DrawConfigState, b: &mut Buffers) {
    let f = st.gameplay_frames;
    b.xlu.insert(0x08, gfx_two_tex_scroll(G_TX_RENDERTILE, 0, f.wrapping_mul(3) % 1024, 32, 256, 1, 0, f.wrapping_mul(3) % 1024, 32, 256));
    b.xlu.insert(0x09, gfx_two_tex_scroll(G_TX_RENDERTILE, 0, f % 256, 64, 64, 1, 0, f % 256, 64, 64));
    b.xlu.insert(0x0A, gfx_two_tex_scroll(G_TX_RENDERTILE, 0, f.wrapping_mul(2) % 128, 32, 32, 1, 0, f.wrapping_mul(2) % 128, 32, 32));
    b.opa.insert(0x0B, gfx_two_tex_scroll(G_TX_RENDERTILE, 0, 0, 32, 32, 1, 0, 127 - f.wrapping_mul(3) % 128, 32, 32));
    b.xlu.insert(0x0C, gfx_two_tex_scroll(G_TX_RENDERTILE, 0, f % 128, 32, 32, 1, 0, f % 128, 32, 32));
    b.xlu.insert(0x0D, gfx_two_tex_scroll(G_TX_RENDERTILE, 0, f % 64, 16, 16, 1, 0, f % 64, 16, 16));
    // gDPSetEnvColor(POLY_OPA_DISP / POLY_XLU_DISP, 128, 128, 128, 128): straight into the
    // buffers, so already in the meshes.
}

/// `Scene_DrawConfigLostWoods` (the Lost Woods): the water's scrolls on segments 8 and 9; with
/// Cojiro owned, once, 50 frames in, his cry (`roomCtx.drawParams[0]` set when it's done,
/// `drawParams[1]` the frames).
fn draw_config_spot10(st: &mut DrawConfigState, b: &mut Buffers) {
    let f = st.gameplay_frames;
    b.xlu.insert(0x08, gfx_two_tex_scroll(G_TX_RENDERTILE, f % 128, 0, 32, 16, 1, f % 128, 0, 32, 16));
    b.xlu.insert(0x09, gfx_two_tex_scroll(G_TX_RENDERTILE, 127 - f % 128, f % 128, 32, 32, 1, f % 128, f % 128, 32, 32));
    // gDPSetEnvColor(POLY_XLU_DISP / POLY_OPA_DISP, 128, 128, 128, 128): straight into the
    // buffers, so already in the meshes.
    if st.room_draw_params[0] == 0 && st.cojiro {
        if st.room_draw_params[1] == 50 {
            st.chicken_cry = true;
            st.room_draw_params[0] = 1;
        }
        st.room_draw_params[1] = st.room_draw_params[1].wrapping_add(1);
    }
}

/// `Scene_DrawConfigDeathMountainTrail` (Death Mountain Trail): at night (from 18:00 to 7:00)
/// a display list on segment 8 that fades the lit overlay (`spot16_room_0DL_00AA48`) in with
/// `roomCtx.drawParams[0]` as its prim alpha (out again from 6:00).
fn draw_config_spot16(st: &mut DrawConfigState, b: &mut Buffers) {
    let t = st.day_time as i32;
    let dl = if t > clock_time(7, 0) && t <= clock_time(18, 0) {
        vec![g_sp_end_display_list()]
    } else {
        if t > clock_time(18, 0) {
            if st.room_draw_params[0] != 255 {
                step_to_s(&mut st.room_draw_params[0], 255, 5);
            }
        } else if t >= clock_time(6, 0) && st.room_draw_params[0] != 0 {
            step_to_s(&mut st.room_draw_params[0], 0, 10);
        }
        // The gSPDisplayList(spot16_room_0DL_00AA48) draws geometry that is in the meshes.
        vec![g_dp_set_prim_color(0, 0, 255, 255, 255, st.room_draw_params[0] as u32), (0xDE00_0000, 0), g_sp_end_display_list()]
    };
    b.xlu.insert(0x08, dl);
}

/// `Scene_DrawConfigHyruleCastle` (Hyrule Castle's grounds): the moat's and the drain's
/// waterfall's scrolls on segments 8 and 9.
fn draw_config_spot15(st: &mut DrawConfigState, b: &mut Buffers) {
    let f = st.gameplay_frames;
    b.xlu.insert(0x08, gfx_two_tex_scroll(G_TX_RENDERTILE, 127 - f % 128, f.wrapping_mul(10) % 128, 32, 32, 1, f % 128, f.wrapping_mul(10) % 128, 32, 32));
    b.xlu.insert(0x09, gfx_two_tex_scroll(G_TX_RENDERTILE, 127 - f % 128, f.wrapping_mul(3) % 128, 32, 32, 1, f % 128, f.wrapping_mul(3) % 128, 32, 32));
    // gDPSetEnvColor(POLY_OPA_DISP / POLY_XLU_DISP, 128, 128, 128, 128): straight into the
    // buffers, so already in the meshes.
}

/// `(u8)(f)` of a float: MIPS `trunc.w.s`, then the low byte (Rust's `as u8` saturates).
fn u8_of(f: f32) -> u32 {
    (f as i32) as u8 as u32
}

/// `Scene_DrawConfigTempleOfTime` (the Temple of Time): prim and env colours on segments 8 to
/// 0xD in both buffers, faded by `roomCtx.drawParams[0]` (the light from the Door of Time,
/// `CS_MISC_*` cues) and `drawParams[1]` (the Triforce's flash, `CS_MISC_TRIFORCE_FLASH`); on
/// layer 5 the custom lens flare at the window.
fn draw_config_tokinoma(st: &mut DrawConfigState, b: &mut Buffers) {
    let temp = st.room_draw_params[0] as f32 / 255.0;
    let p0 = st.room_draw_params[0] as u32;
    let p1 = st.room_draw_params[1] as u32;
    // Each component is an int expression; gDPSetPrimColor's _SHIFTL keeps its low byte.
    let seg8 = vec![g_dp_set_prim_color(0, 0, 255 - u8_of(185.0 * temp), 255 - u8_of(145.0 * temp), 255 - u8_of(105.0 * temp), 255), g_sp_end_display_list()];
    let seg9 = vec![g_dp_set_prim_color(0, 0, 76 + u8_of(6.0 * temp), 76 + u8_of(34.0 * temp), 76 + u8_of(74.0 * temp), 255), g_sp_end_display_list()];
    let seg_a = vec![g_dp_pipe_sync(), g_dp_set_env_color(0, 0, 0, p0), g_sp_end_display_list()];
    let grey = 89 + u8_of(166.0 * temp);
    let seg_b = vec![g_dp_set_prim_color(0, 0, grey, grey, grey, 255), g_dp_pipe_sync(), g_dp_set_env_color(0, 0, 0, p0), g_sp_end_display_list()];
    let white = 255 + u8_of(179.0 * temp);
    let seg_c = vec![g_dp_set_prim_color(0, 0, white, white, white, 255), g_dp_pipe_sync(), g_dp_set_env_color(0, 0, 0, p0), g_sp_end_display_list()];
    let seg_d = vec![g_dp_pipe_sync(), g_dp_set_env_color(0, 0, 0, p1), g_sp_end_display_list()];
    for (s, dl) in [(0x08, seg8), (0x09, seg9), (0x0A, seg_a), (0x0B, seg_b), (0x0C, seg_c), (0x0D, seg_d)] {
        b.opa.insert(s, dl.clone());
        b.xlu.insert(s, dl);
    }
    if st.scene_layer == 5 {
        st.custom_lens_flare = Some(CustomLensFlare { pos: [-20.0, 1220.0, -684.0], scale: 10, color_intensity: 8.0, glare_strength: 200 });
    }
}

/// `Scene_DrawConfigCastleCourtyardGuards` (the guards' courtyard, by day and by night): the
/// water's scroll on segment 8, and by day the fountain's on 9.
fn draw_config_hairal_niwa(st: &mut DrawConfigState, b: &mut Buffers) {
    let f = st.gameplay_frames;
    b.xlu.insert(0x08, gfx_two_tex_scroll(G_TX_RENDERTILE, 127 - f % 128, f.wrapping_mul(3) % 128, 32, 32, 1, f % 128, f.wrapping_mul(3) % 128, 32, 32));
    if st.scene_id == SCENE_CASTLE_COURTYARD_GUARDS_DAY {
        b.xlu.insert(0x09, gfx_tex_scroll(0, f.wrapping_mul(10) % 256, 32, 64));
    }
    // gDPSetEnvColor(POLY_OPA_DISP / POLY_XLU_DISP, 128, 128, 128, 128): straight into the
    // buffers, so already in the meshes.
}

/// `Scene_DrawConfigCalmWater` (Zelda's courtyard and other still water): the water's slow
/// scroll on segment 8.
fn draw_config_calm_water(st: &mut DrawConfigState, b: &mut Buffers) {
    let f = st.gameplay_frames;
    b.xlu.insert(0x08, gfx_two_tex_scroll(G_TX_RENDERTILE, 127 - f % 128, f % 128, 32, 32, 1, f % 128, f % 128, 32, 32));
    // gDPSetEnvColor(POLY_OPA_DISP / POLY_XLU_DISP, 128, 128, 128, 128): straight into the
    // buffers, so already in the meshes.
}

/// `Scene_DrawConfigGrottos` (the grottos): scrolls on segments 8, 9 and 0xC in the XLU buffer,
/// and 0xA, 0xB and 0xD in the OPA buffer.
fn draw_config_kakusiana(st: &mut DrawConfigState, b: &mut Buffers) {
    let f = st.gameplay_frames;
    b.xlu.insert(0x08, gfx_tex_scroll(0, f % 64, 256, 16));
    b.xlu.insert(0x09, gfx_two_tex_scroll(G_TX_RENDERTILE, 127 - (f % 128), f % 128, 32, 32, 1, f % 128, f % 128, 32, 32));
    b.opa.insert(0x0A, gfx_two_tex_scroll(G_TX_RENDERTILE, 0, 0, 32, 32, 1, 0, 127 - f % 128, 32, 32));
    b.opa.insert(0x0B, gfx_tex_scroll(0, f % 128, 32, 32));
    b.xlu.insert(0x0C, gfx_two_tex_scroll(G_TX_RENDERTILE, 0, f.wrapping_mul(50) % 2048, 8, 512, 1, 0, f.wrapping_mul(60) % 2048, 8, 512));
    b.opa.insert(0x0D, gfx_two_tex_scroll(G_TX_RENDERTILE, 0, 0, 32, 64, 1, 0, f % 128, 32, 32));
    // gDPSetEnvColor(POLY_XLU_DISP / POLY_OPA_DISP, 128, 128, 128, 128): straight into the
    // buffers, so already in the meshes.
}

/// `Scene_DrawConfigGraveExitLightShining` (the ReDead grave, the windmill and Dampé's grave):
/// the exit's light, a scroll on segment 8 in the XLU buffer.
fn draw_config_grave_exit_light_shining(st: &mut DrawConfigState, b: &mut Buffers) {
    let f = st.gameplay_frames;
    b.xlu.insert(0x08, gfx_tex_scroll(0, f % 64, 256, 16));
    // gDPSetEnvColor(POLY_XLU_DISP / POLY_OPA_DISP, 128, 128, 128, 128): straight into the
    // buffers, so already in the meshes.
}

/// `Scene_DrawConfigFairysFountain` (the fairy fountains, the shield grave): the water's scroll
/// on segment 8 and the exit's light on segment 9, both in the XLU buffer.
fn draw_config_fairys_fountain(st: &mut DrawConfigState, b: &mut Buffers) {
    let f = st.gameplay_frames;
    b.xlu.insert(0x08, gfx_two_tex_scroll(G_TX_RENDERTILE, 127 - f % 128, f.wrapping_mul(3) % 128, 32, 32, 1, f % 128, f.wrapping_mul(3) % 128, 32, 32));
    b.xlu.insert(0x09, gfx_tex_scroll(0, f % 64, 256, 16));
    // gDPSetEnvColor(POLY_OPA_DISP / POLY_XLU_DISP, 128, 128, 128, 128): straight into the
    // buffers, so already in the meshes.
}

/// `Scene_DrawConfigRoyalFamilysTomb` (the royal tomb): the exit's light on segment 8, two tall
/// scrolls on segments 9 and 0xB (XLU), and the water's on segment 0xA (OPA).
fn draw_config_royal_familys_tomb(st: &mut DrawConfigState, b: &mut Buffers) {
    let f = st.gameplay_frames;
    b.xlu.insert(0x08, gfx_tex_scroll(0, f % 64, 256, 16));
    b.xlu.insert(0x09, gfx_two_tex_scroll(G_TX_RENDERTILE, 0, f.wrapping_mul(60) % 2048, 8, 512, 1, 0, f.wrapping_mul(50) % 2048, 8, 512));
    b.opa.insert(0x0A, gfx_two_tex_scroll(G_TX_RENDERTILE, 127 - f % 128, 0, 32, 32, 1, f % 128, 0, 32, 32));
    b.xlu.insert(0x0B, gfx_two_tex_scroll(G_TX_RENDERTILE, 0, 1023 - f.wrapping_mul(6) % 1024, 16, 256, 1, 0, 1023 - f.wrapping_mul(3) % 1024, 16, 256));
    // gDPSetEnvColor(POLY_OPA_DISP / POLY_XLU_DISP, 128, 128, 128, 128): straight into the
    // buffers, so already in the meshes.
}

/// `Scene_DrawConfigMarketGuardHouse` (the market's guard house): its two window views on
/// segments 8 and 9 (`segment_textures`); no display lists.
fn draw_config_miharigoya(_st: &mut DrawConfigState, _b: &mut Buffers) {
    // gDPSetEnvColor(POLY_OPA_DISP / POLY_XLU_DISP, 128, 128, 128, 128): straight into the
    // buffers, so already in the meshes.
}

/// `Scene_DrawConfigKakarikoVillage` (Kakariko): its windows on segment 8
/// (`segment_textures`); no display lists.
fn draw_config_spot01(_st: &mut DrawConfigState, _b: &mut Buffers) {
    // gDPSetEnvColor(POLY_OPA_DISP / POLY_XLU_DISP, 128, 128, 128, 128): straight into the
    // buffers, so already in the meshes.
}

/// `Scene_DrawConfigLonLonRanch` (Lon Lon Ranch): its windows on segment 8 (`segment_textures`);
/// no display lists.
fn draw_config_spot20(_st: &mut DrawConfigState, _b: &mut Buffers) {
    // gDPSetEnvColor(POLY_OPA_DISP / POLY_XLU_DISP, 128, 128, 128, 128): straight into the
    // buffers, so already in the meshes.
}

/// `Scene_DrawConfigLonLonBuildings` (Talon's house, `souko`): its entrance on segment 8
/// (`segment_textures`); no display lists.
fn draw_config_souko(_st: &mut DrawConfigState, _b: &mut Buffers) {
    // gDPSetEnvColor(POLY_OPA_DISP / POLY_XLU_DISP, 128, 128, 128, 128): straight into the
    // buffers, so already in the meshes.
}

/// `Scene_DrawConfigGoronCity` (Goron City): a scroll on segment 8 in the OPA buffer, and its
/// entrance on segment 8 in the XLU buffer (`segment_textures`).
fn draw_config_spot18(st: &mut DrawConfigState, b: &mut Buffers) {
    let f = st.gameplay_frames;
    b.opa.insert(0x08, gfx_two_tex_scroll(G_TX_RENDERTILE, 0, 127 - f % 128, 32, 32, 1, f % 128, 0, 32, 32));
    // gDPSetEnvColor(POLY_OPA_DISP / POLY_XLU_DISP, 128, 128, 128, 128): straight into the
    // buffers, so already in the meshes.
}

/// `Scene_DrawConfigDodongosCavern` (Dodongo's Cavern): a scroll on segment 9 in the XLU buffer
/// and on 0xA in the OPA one; the skull's eyes' env colours on 0xB and 0xC, their alpha
/// `roomCtx.drawParams[BGDODOAGO_EYE_LEFT]` and `[BGDODOAGO_EYE_RIGHT]` (which `Bg_Dodoago`
/// raises as each eye lights). The entrance by `nightFlag` on segment 8 and the lava floor's
/// eight frames on segment 9 (OPA) are `segment_textures`'.
fn draw_config_ddan(st: &mut DrawConfigState, b: &mut Buffers) {
    let f = st.gameplay_frames;
    b.xlu.insert(0x09, gfx_two_tex_scroll(G_TX_RENDERTILE, f % 256, 0, 64, 32, 1, 0, f % 128, 64, 32));
    b.opa.insert(0x0A, gfx_two_tex_scroll(G_TX_RENDERTILE, 0, f % 128, 32, 32, 1, 0, f.wrapping_mul(2) % 128, 32, 32));
    // gDPSetEnvColor(POLY_OPA_DISP / POLY_XLU_DISP, 128, 128, 128, 128): straight into the
    // buffers, so already in the meshes.
    for (seg, eye) in [(0x0B, BGDODOAGO_EYE_LEFT), (0x0C, BGDODOAGO_EYE_RIGHT)] {
        // gDPSetEnvColor's alpha through _SHIFTL(a, 0, 8): the s16's low byte.
        b.opa.insert(seg, vec![g_dp_pipe_sync(), g_dp_set_env_color(255, 255, 255, st.room_draw_params[eye] as u32), g_sp_end_display_list()]);
    }
}

/// `BgDodoagoEye` (`z_bg_dodoago.h`): the index of each eye's glow in `roomCtx.drawParams`.
pub const BGDODOAGO_EYE_LEFT: usize = 0;
pub const BGDODOAGO_EYE_RIGHT: usize = 1;

/// A texture a draw config points a segment at this frame
/// (`gSPSegment(.., SEGMENTED_TO_VIRTUAL(texture))`): in the OPA buffer, or the XLU one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentTexture {
    pub segment: u8,
    pub xlu: bool,
    /// The texture's file and symbol in the pack (`keys::texture`).
    pub file: &'static str,
    pub symbol: &'static str,
}

/// `sKakarikoWindowTextures` (by `nightFlag`).
const KAKARIKO_WINDOW_TEXTURES: [&str; 2] = ["gKakarikoVillageDayWindowTex", "gKakarikoVillageNightWindowTex"];
/// `sGuardHouseView1Textures`, `sGuardHouseView2Textures` (day, night).
const GUARD_HOUSE_VIEW1_TEXTURES: [&str; 2] = ["gGuardHouseOutSideView1DayTex", "gGuardHouseOutSideView1NightTex"];
const GUARD_HOUSE_VIEW2_TEXTURES: [&str; 2] = ["gGuardHouseOutSideView2DayTex", "gGuardHouseOutSideView2NightTex"];

/// `Scene_DrawConfigKakarikoVillage`'s segment 8: the windows lit by night.
fn textures_spot01(st: &DrawConfigState) -> Vec<SegmentTexture> {
    vec![SegmentTexture { segment: 0x08, xlu: false, file: "spot01_scene", symbol: KAKARIKO_WINDOW_TEXTURES[st.night as usize] }]
}

/// `Scene_DrawConfigMarketGuardHouse`'s segments 8 (view 2) and 9 (view 1): the outside by
/// `nightFlag` for a child, always the night's for an adult (Castle Town is ruined).
fn textures_miharigoya(st: &DrawConfigState) -> Vec<SegmentTexture> {
    let var = if !st.child { 1 } else { st.night as usize };
    vec![
        SegmentTexture { segment: 0x08, xlu: false, file: "miharigoya_scene", symbol: GUARD_HOUSE_VIEW2_TEXTURES[var] },
        SegmentTexture { segment: 0x09, xlu: false, file: "miharigoya_scene", symbol: GUARD_HOUSE_VIEW1_TEXTURES[var] },
    ]
}

/// `sLonLonRanchWindowTextures` (by `nightFlag`; the C's names, `Range` and `Windows` theirs).
const LON_LON_RANCH_WINDOW_TEXTURES: [&str; 2] = ["gLonLonRanchDayWindowTex", "gLonLonRangeNightWindowsTex"];
/// `sLonLonHouseEntranceTextures` (by `nightFlag`).
const LON_LON_HOUSE_ENTRANCE_TEXTURES: [&str; 2] = ["gLonLonHouseDayEntranceTex", "gLonLonHouseNightEntranceTex"];

/// `Scene_DrawConfigLonLonRanch`'s segment 8 (OPA): the windows lit by night.
fn textures_spot20(st: &DrawConfigState) -> Vec<SegmentTexture> {
    vec![SegmentTexture { segment: 0x08, xlu: false, file: "spot20_scene", symbol: LON_LON_RANCH_WINDOW_TEXTURES[st.night as usize] }]
}

/// `Scene_DrawConfigLonLonBuildings`' segment 8 (XLU): Talon's house's entrance, the light
/// through it by `nightFlag`.
fn textures_souko(st: &DrawConfigState) -> Vec<SegmentTexture> {
    vec![SegmentTexture { segment: 0x08, xlu: true, file: "souko_scene", symbol: LON_LON_HOUSE_ENTRANCE_TEXTURES[st.night as usize] }]
}

/// `sGoronCityEntranceTextures` (by `nightFlag`).
const GORON_CITY_ENTRANCE_TEXTURES: [&str; 2] = ["gGoronCityDayEntranceTex", "gGoronCityNightEntranceTex"];

/// `Scene_DrawConfigGoronCity`'s segment 8 (XLU): the city's entrance, the light through it by
/// `nightFlag`.
fn textures_spot18(st: &DrawConfigState) -> Vec<SegmentTexture> {
    vec![SegmentTexture { segment: 0x08, xlu: true, file: "spot18_scene", symbol: GORON_CITY_ENTRANCE_TEXTURES[st.night as usize] }]
}

/// `gDCEntranceTextures` (by `nightFlag`).
const DC_ENTRANCE_TEXTURES: [&str; 2] = ["gDCDayEntranceTex", "gDCNightEntranceTex"];
/// `sDCLavaFloorTextures`.
const DC_LAVA_FLOOR_TEXTURES: [&str; 8] =
    ["gDCLavaFloor1Tex", "gDCLavaFloor2Tex", "gDCLavaFloor3Tex", "gDCLavaFloor4Tex", "gDCLavaFloor5Tex", "gDCLavaFloor6Tex", "gDCLavaFloor7Tex", "gDCLavaFloor8Tex"];

/// `Scene_DrawConfigDodongosCavern`'s segment 8 (OPA): the entrance, the light through it by
/// `nightFlag`; segment 9 (OPA): the lava floor, a frame of eight every two
/// (`(gameplayFrames & 14) >> 1`).
fn textures_ddan(st: &DrawConfigState) -> Vec<SegmentTexture> {
    vec![
        SegmentTexture { segment: 0x08, xlu: false, file: "ddan_scene", symbol: DC_ENTRANCE_TEXTURES[st.night as usize] },
        SegmentTexture { segment: 0x09, xlu: false, file: "ddan_scene", symbol: DC_LAVA_FLOOR_TEXTURES[((st.gameplay_frames & 14) >> 1) as usize] },
    ]
}

type TexturesFn = fn(&DrawConfigState) -> Vec<SegmentTexture>;

/// The ported draw configs that point segments at textures by state.
const TEXTURES: &[(&str, TexturesFn)] = &[
    ("SDC_KAKARIKO_VILLAGE", textures_spot01),
    ("SDC_MARKET_GUARD_HOUSE", textures_miharigoya),
    ("SDC_LON_LON_RANCH", textures_spot20),
    ("SDC_LON_LON_BUILDINGS", textures_souko),
    ("SDC_GORON_CITY", textures_spot18),
    ("SDC_DODONGOS_CAVERN", textures_ddan),
];

/// The textures the draw config `sdc` points segments at this frame (empty for the others).
pub fn segment_textures(sdc: &str, st: &DrawConfigState) -> Vec<SegmentTexture> {
    TEXTURES.iter().find(|(n, _)| *n == sdc).map(|(_, f)| f(st)).unwrap_or_default()
}

type DrawConfigFn = fn(&mut DrawConfigState, &mut Buffers);

/// The ported draw configs, by `SDC_*` name.
const PORTED: &[(&str, DrawConfigFn)] = &[
    ("SDC_DEFAULT", draw_config_default),
    ("SDC_HYRULE_FIELD", draw_config_spot00),
    ("SDC_KOKIRI_FOREST", draw_config_spot04),
    ("SDC_DEKU_TREE", draw_config_ydan),
    ("SDC_GERUDO_VALLEY", draw_config_spot09),
    ("SDC_DEATH_MOUNTAIN_TRAIL", draw_config_spot16),
    ("SDC_LOST_WOODS", draw_config_spot10),
    ("SDC_HYRULE_CASTLE", draw_config_spot15),
    ("SDC_TEMPLE_OF_TIME", draw_config_tokinoma),
    ("SDC_CASTLE_COURTYARD_GUARDS", draw_config_hairal_niwa),
    ("SDC_CALM_WATER", draw_config_calm_water),
    ("SDC_KAKARIKO_VILLAGE", draw_config_spot01),
    ("SDC_GROTTOS", draw_config_kakusiana),
    ("SDC_MARKET_GUARD_HOUSE", draw_config_miharigoya),
    ("SDC_GRAVE_EXIT_LIGHT_SHINING", draw_config_grave_exit_light_shining),
    ("SDC_FAIRYS_FOUNTAIN", draw_config_fairys_fountain),
    ("SDC_ROYAL_FAMILYS_TOMB", draw_config_royal_familys_tomb),
    ("SDC_LON_LON_RANCH", draw_config_spot20),
    ("SDC_LON_LON_BUILDINGS", draw_config_souko),
    ("SDC_GORON_CITY", draw_config_spot18),
    ("SDC_DODONGOS_CAVERN", draw_config_ddan),
];

/// Whether the draw config `sdc` (an `SDC_*` name) is ported.
pub fn is_ported(sdc: &str) -> bool {
    PORTED.iter().any(|(n, _)| *n == sdc)
}

/// The names of the ported draw configs.
pub fn ported() -> impl Iterator<Item = &'static str> {
    PORTED.iter().map(|(n, _)| *n)
}

/// Runs the scene's draw config (`Scene_Draw` → `sSceneDrawConfigs[play->sceneDrawConfig]`)
/// for one frame: this frame's contents of its dynamic segments in the OPA and XLU buffers,
/// or `None` if `sdc` isn't ported. `st.room_unk_74` carries the config's state to the next
/// frame.
pub fn segment_values(sdc: &str, st: &mut DrawConfigState) -> Option<[SegmentValues; 2]> {
    let f = PORTED.iter().find(|(n, _)| *n == sdc)?.1;
    let mut b = Buffers::default();
    f(st, &mut b);
    Some(b.values())
}
