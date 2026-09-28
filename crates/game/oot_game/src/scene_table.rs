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

use std::collections::BTreeMap;

use eng_gfx::SegmentValues;
use eng_math::step_to_s;

use crate::env::clock_time;

/// What a draw config reads: the frame counter and save state, and `roomCtx.unk_74`, which
/// some configs keep their own state in across frames.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DrawConfigState {
    /// `play->gameplayFrames`.
    pub gameplay_frames: u32,
    /// `LINK_IS_CHILD`.
    pub child: bool,
    /// `gSaveContext.nightFlag`.
    pub night: bool,
    /// `gSaveContext.sceneLayer`.
    pub scene_layer: usize,
    /// `gSaveContext.dayTime`.
    pub day_time: u16,
    /// `play->roomCtx.unk_74` ("context-specific data used by the current scene draw config",
    /// `z64.h`). Zeroed on scene load (`Room_Init` clears the room context).
    pub room_unk_74: [i16; 2],
    /// `GET_EVENTCHKINF(EVENTCHKINF_07)`.
    pub event_chk_inf_07: bool,
}

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
fn g_dp_tile_sync() -> (u32, u32) {
    (0xE800_0000, 0)
}
fn g_sp_end_display_list() -> (u32, u32) {
    (0xDF00_0000, 0)
}
/// `gDPSetTileSize` (`gbi.h`: `G_SETTILESIZE`, 12-bit fields).
fn g_dp_set_tile_size(tile: u32, uls: u32, ult: u32, lrs: u32, lrt: u32) -> (u32, u32) {
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
#[allow(dead_code)]
fn gfx_tex_scroll(x: u32, y: u32, width: i32, height: i32) -> Dl {
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

/// `Scene_DrawConfigSpot00` (Hyrule Field): the river's scrolling textures on segments 8 and
/// 9, and at night a display list on segment 0xA that fades the lit-window overlay
/// (`spot00_room_0DL_012B20`) in with `roomCtx.unk_74[0]` as its prim alpha.
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
            if st.room_unk_74[0] != 255 {
                step_to_s(&mut st.room_unk_74[0], 255, 5);
            }
        } else if t >= clock_time(6, 0) && st.room_unk_74[0] != 0 {
            step_to_s(&mut st.room_unk_74[0], 0, 10);
        }
        // The gSPDisplayList(spot00_room_0DL_012B20) between them draws geometry that is in
        // the meshes; SegmentValues::read skips it.
        vec![g_dp_set_prim_color(0, 0, 255, 255, 255, st.room_unk_74[0] as u32), (0xDE00_0000, 0), g_sp_end_display_list()]
    };
    b.xlu.insert(0x0A, dl);
}

/// `Scene_DrawConfigSpot04` (Kokiri Forest): the stream and waterfall scroll on segments 8
/// and 9, env colours on 0xA (alpha `spA3`) and 0xB (alpha `spA0 * 0.1`), and a scroll by
/// `roomCtx.unk_74[0]` on 0xC.
fn draw_config_spot04(st: &mut DrawConfigState, b: &mut Buffers) {
    let mut sp_a3: u8 = 128;
    let mut sp_a0: u16 = 500;
    let f = st.gameplay_frames;
    b.xlu.insert(0x09, gfx_two_tex_scroll(G_TX_RENDERTILE, 127 - f % 128, f % 128, 32, 32, 1, f % 128, f % 128, 32, 32));
    b.xlu.insert(0x08, gfx_two_tex_scroll(G_TX_RENDERTILE, 127 - f % 128, f.wrapping_mul(10) % 128, 32, 32, 1, f % 128, f.wrapping_mul(10) % 128, 32, 32));
    if st.scene_layer == 4 {
        sp_a3 = 255u8.wrapping_sub(st.room_unk_74[0] as u8);
    } else if st.scene_layer == 6 {
        sp_a0 = (st.room_unk_74[0] as i32 + 500) as u16;
    } else if (!st.is_cutscene_layer() || !st.child) && st.event_chk_inf_07 {
        sp_a0 = 2150;
    }
    let seg_a = vec![g_dp_pipe_sync(), g_dp_set_env_color(128, 128, 128, sp_a3 as u32), g_sp_end_display_list()];
    // The float `spA0 * 0.1f` goes through `_SHIFTL`'s `(u32)` cast.
    let seg_b = vec![g_dp_pipe_sync(), g_dp_set_env_color(128, 128, 128, (sp_a0 as f32 * 0.1) as u32), g_sp_end_display_list()];
    b.opa.insert(0x0A, seg_a);
    b.xlu.insert(0x0B, seg_b.clone());
    b.opa.insert(0x0B, seg_b);
    let scroll = (-(st.room_unk_74[0] as f32) * 0.02) as i16 as u32;
    b.opa.insert(0x0C, gfx_two_tex_scroll(G_TX_RENDERTILE, 0, scroll, 32, 16, 1, 0, scroll, 32, 16));
}

/// `Scene_DrawConfigYdan` (the Deku Tree): a scroll on segment 9. Segment 8 points at the
/// day or night texture (`D_8012A2F8[nightFlag]`), which is static data in the meshes.
fn draw_config_ydan(st: &mut DrawConfigState, b: &mut Buffers) {
    let f = st.gameplay_frames;
    b.xlu.insert(0x09, gfx_two_tex_scroll(G_TX_RENDERTILE, 127 - (f % 128), f % 128, 32, 32, 1, f % 128, f % 128, 32, 32));
}

type DrawConfigFn = fn(&mut DrawConfigState, &mut Buffers);

/// The ported draw configs, by `SDC_*` name.
const PORTED: &[(&str, DrawConfigFn)] = &[
    ("SDC_DEFAULT", draw_config_default),
    ("SDC_SPOT00", draw_config_spot00),
    ("SDC_SPOT04", draw_config_spot04),
    ("SDC_YDAN", draw_config_ydan),
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
