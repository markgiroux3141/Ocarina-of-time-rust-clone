//! `z_kaleido_scope.c`: the menu's states (`KaleidoScope_Update`), its frame (the four pages'
//! box and their turns, the cursor, the name and info panels: `KaleidoScope_Draw`), the dungeon
//! map page's loads (`KaleidoScope_LoadDungeonMap`, `_UpdateDungeonMap`,
//! `_OverridePalIndexCI4`), and the game over's states and screens (`KaleidoScope_DrawGameOver`,
//! the prompt page). The item page is `super::item`, the dungeon map `super::map` and
//! `super::lmap_mark`; the other pages' contents log (see the module's notes).

use glam::{Mat4, Vec3};

use super::gfx::{Cc, GameOverPart, KTex, KaleidoGfx, Vtx};
use super::map::{MAP_SEGMENT_SECOND, SCENE_TREASURE_BOX_SHOP};
use super::*;
use crate::map::{MAP_48X85_TEX_SIZE, MapSegment};
use crate::audio::sfx::{NA_SE_SY_CURSOR, NA_SE_SY_DECIDE, NA_SE_SY_OCARINA_ERROR, NA_SE_SY_PIECE_OF_HEART, NA_SE_SY_TRE_BOX_APPEAR, NA_SE_SY_WIN_SCROLL_LEFT, NA_SE_SY_WIN_SCROLL_RIGHT};
use crate::interface::{BTN_DISABLED, BTN_ENABLED, DO_ACTION_DECIDE, DO_ACTION_NONE, DO_ACTION_SAVE, HUD_VISIBILITY_NOTHING};
use crate::item::{ITEM_NONE, ITEM_SOLD_OUT};
use eng_input::pad::{BTN_A, BTN_B, BTN_L, BTN_R, BTN_START, BTN_Z};

/// `HUD_VISIBILITY_NO_CHANGE`, `HUD_VISIBILITY_ALL` (`save.h`).
const HUD_VISIBILITY_NO_CHANGE: u16 = 0;
const HUD_VISIBILITY_ALL: u16 = 50;

// `VtxPageInit`, and the extra quads each page's vertices have.
const VTX_PAGE_ITEM: usize = 0;
const VTX_PAGE_EQUIP: usize = 1;
const VTX_PAGE_MAP_DUNGEON: usize = 2;
const VTX_PAGE_QUEST: usize = 3;
const VTX_PAGE_MAP_WORLD: usize = 4;
const VTX_PAGE_PROMPT: usize = 5;
const VTX_PAGE_MAP_DUNGEON_QUADS: usize = 17;
const VTX_PAGE_MAP_WORLD_QUADS: usize = 32;
const VTX_PAGE_PROMPT_QUADS: usize = 5;

/// `PAGE_BG_COLS`, `PAGE_BG_ROWS`, `PAGE_BG_QUADS`, `PAGE_BG_QUAD_WIDTH`, `_HEIGHT`
/// (`z_kaleido_scope.h`).
const PAGE_BG_COLS: usize = 3;
const PAGE_BG_ROWS: usize = 5;
pub(super) const PAGE_BG_QUADS: usize = PAGE_BG_COLS * PAGE_BG_ROWS;
const PAGE_BG_QUAD_WIDTH: i16 = 80;
const PAGE_BG_QUAD_HEIGHT: i16 = 32;

/// `WORLD_MAP_IMAGE_WIDTH`, `_HEIGHT`, `_FRAG_HEIGHT` (`(TMEM_SIZE / 2) / 216` = 9), `_FRAG_NUM`.
const WORLD_MAP_IMAGE_WIDTH: i16 = 216;
const WORLD_MAP_IMAGE_HEIGHT: i16 = 128;
const WORLD_MAP_IMAGE_FRAG_HEIGHT: i16 = 9;
const WORLD_MAP_IMAGE_FRAG_NUM: usize = 15;
/// `WORLD_MAP_QUAD_POINT_FIRST`, `WORLD_MAP_QUAD_TRADE_QUEST_MARKER`.
const WORLD_MAP_QUAD_POINT_FIRST: usize = 16;
const WORLD_MAP_QUAD_TRADE_QUEST_MARKER: usize = 29;
/// `TRADE_QUEST_MARKER_NONE`.
pub const TRADE_QUEST_MARKER_NONE: u8 = 0xFF;

/// `ITEM_QUAD_MAX`, `EQUIP_QUAD_MAX`, `QUEST_QUAD_MAX`, `UI_OVERLAY_QUAD_MAX`,
/// `PAUSE_CURSOR_QUAD_MAX`.
pub(super) const ITEM_QUAD_MAX: usize = 41;
const EQUIP_QUAD_MAX: usize = 28;
const QUEST_QUAD_MAX: usize = 47;
const UI_OVERLAY_QUAD_MAX: usize = 7;
const PAUSE_CURSOR_QUAD_MAX: usize = 5;

/// `PAUSE_EQUIP_PLAYER_WIDTH`, `_HEIGHT`, `_FRAG_HEIGHT` (`TMEM_SIZE / (64 * 2)` = 32).
const PAUSE_EQUIP_PLAYER_WIDTH: i16 = 64;
const PAUSE_EQUIP_PLAYER_HEIGHT: i16 = 112;
const PAUSE_EQUIP_PLAYER_FRAG_HEIGHT: i16 = 32;

/// `UI_OVERLAY_QUAD_BUTTON_LR_WIDTH`, `_HEIGHT`, `_TEX_WIDTH`, `_TEX_HEIGHT`.
const UI_OVERLAY_QUAD_BUTTON_LR_WIDTH: i16 = 24;
const UI_OVERLAY_QUAD_BUTTON_LR_HEIGHT: i16 = 32;
/// `gABtnSymbolTex_WIDTH`, `gCBtnSymbolsTex_WIDTH`, `QUEST_ICON_WIDTH`.
const A_BTN_SYMBOL_WIDTH: i16 = 24;
const C_BTN_SYMBOLS_WIDTH: i16 = 48;
/// `gPauseToEquipENGTex_WIDTH`, `gPauseToDecideENGTex_WIDTH`, `gPauseToPlayMelodyENGTex_WIDTH`,
/// `TO_PAGE_LABEL_TEX_WIDTH` (`icon_item_nes_static`).
const TO_EQUIP_WIDTH: i16 = 56;
const TO_DECIDE_WIDTH: i16 = 64;
const TO_PLAY_MELODY_WIDTH: i16 = 80;
const TO_PAGE_LABEL_TEX_WIDTH: i16 = 128;

/// `PAGE_SWITCH_NSTEPS`.
const PAGE_SWITCH_NSTEPS: u16 = 16;
/// `PAGE_SWITCH_PT_LEFT`, `PAGE_SWITCH_PT_RIGHT`.
const PAGE_SWITCH_PT_LEFT: u8 = 0;
const PAGE_SWITCH_PT_RIGHT: u8 = 2;

/// The pages' positions about the eye (`PAUSE_*_X`, `PAUSE_*_Z`, `pause.h`): item, map, quest,
/// equipment.
const PAGE_XZ: [(f32, f32); 4] = [(0.0, -1.0), (1.0, 0.0), (0.0, 1.0), (-1.0, 0.0)];

/// `sPageSwitchEyeDx`, `sPageSwitchEyeDz`: by `nextPageMode` (each page right, then left).
pub(super) fn page_switch_eye_d(mode: u16) -> (f32, f32) {
    let page = (mode / 2) as usize;
    let to = if mode % 2 == 0 { (page + 1) % 4 } else { (page + 3) % 4 };
    let n = PAGE_SWITCH_NSTEPS as f32;
    (-PAUSE_EYE_DIST * (PAGE_XZ[to].0 - PAGE_XZ[page].0) / n, -PAUSE_EYE_DIST * (PAGE_XZ[to].1 - PAGE_XZ[page].1) / n)
}

/// `sPageSwitchNextPageIndex`.
pub(super) const PAGE_SWITCH_NEXT_PAGE_INDEX: [u16; 8] = [PAUSE_MAP, PAUSE_EQUIP, PAUSE_QUEST, PAUSE_ITEM, PAUSE_EQUIP, PAUSE_MAP, PAUSE_ITEM, PAUSE_QUEST];

/// `gPageSwitchNextButtonStatus`: the page switched to's buttons, by `pageIndex + pt`.
const PAGE_SWITCH_NEXT_BUTTON_STATUS: [[u8; 5]; 6] = {
    const E: u8 = BTN_ENABLED;
    const D: u8 = BTN_DISABLED;
    [
        [E, D, D, D, E], // -> PAUSE_EQUIP
        [E, E, E, E, D], // -> PAUSE_ITEM
        [E, D, D, D, D], // -> PAUSE_MAP
        [E, D, D, D, E], // -> PAUSE_QUEST
        [E, D, D, D, E], // -> PAUSE_EQUIP
        [E, E, E, E, D], // -> PAUSE_ITEM
    ]
};

/// The English pages' background tiles (`sItemPageBgQuadsENGTexs` .. `sQuestPageBgQuadsENGTexs`,
/// `SELECT_ITEM_TEXS(LANGUAGE_ENG)`), by column then row.
const ITEM_PAGE_BG: [&str; 15] = [
    "gPauseSelectItem00ENGTex",
    "gPauseSelectItem01Tex",
    "gPauseSelectItem02Tex",
    "gPauseSelectItem03Tex",
    "gPauseSelectItem04Tex",
    "gPauseSelectItem10ENGTex",
    "gPauseSelectItem11Tex",
    "gPauseSelectItem12Tex",
    "gPauseSelectItem13Tex",
    "gPauseSelectItem14Tex",
    "gPauseSelectItem20ENGTex",
    "gPauseSelectItem21Tex",
    "gPauseSelectItem22Tex",
    "gPauseSelectItem23Tex",
    "gPauseSelectItem24Tex",
];
const EQUIP_PAGE_BG: [&str; 15] = [
    "gPauseEquipment00Tex",
    "gPauseEquipment01Tex",
    "gPauseEquipment02Tex",
    "gPauseEquipment03Tex",
    "gPauseEquipment04Tex",
    "gPauseEquipment10ENGTex",
    "gPauseEquipment11Tex",
    "gPauseEquipment12Tex",
    "gPauseEquipment13Tex",
    "gPauseEquipment14Tex",
    "gPauseEquipment20Tex",
    "gPauseEquipment21Tex",
    "gPauseEquipment22Tex",
    "gPauseEquipment23Tex",
    "gPauseEquipment24Tex",
];
const MAP_PAGE_BG: [&str; 15] = [
    "gPauseMap00Tex",
    "gPauseMap01Tex",
    "gPauseMap02Tex",
    "gPauseMap03Tex",
    "gPauseMap04Tex",
    "gPauseMap10ENGTex",
    "gPauseMap11Tex",
    "gPauseMap12Tex",
    "gPauseMap13Tex",
    "gPauseMap14Tex",
    "gPauseMap20Tex",
    "gPauseMap21Tex",
    "gPauseMap22Tex",
    "gPauseMap23Tex",
    "gPauseMap24Tex",
];
const QUEST_PAGE_BG: [&str; 15] = [
    "gPauseQuestStatus00ENGTex",
    "gPauseQuestStatus01Tex",
    "gPauseQuestStatus02Tex",
    "gPauseQuestStatus03Tex",
    "gPauseQuestStatus04Tex",
    "gPauseQuestStatus10ENGTex",
    "gPauseQuestStatus11Tex",
    "gPauseQuestStatus12Tex",
    "gPauseQuestStatus13Tex",
    "gPauseQuestStatus14Tex",
    "gPauseQuestStatus20ENGTex",
    "gPauseQuestStatus21Tex",
    "gPauseQuestStatus22Tex",
    "gPauseQuestStatus23Tex",
    "gPauseQuestStatus24Tex",
];

/// `sGameOverTexs`: the game over's prompt page.
const GAME_OVER_PAGE_BG: [&str; 15] = [
    "gPauseSave00Tex",
    "gPauseSave01Tex",
    "gPauseSave02Tex",
    "gPauseSave03Tex",
    "gPauseSave04Tex",
    "gPauseGameOver10Tex",
    "gPauseSave11Tex",
    "gPauseSave12Tex",
    "gPauseSave13Tex",
    "gPauseSave14Tex",
    "gPauseSave20Tex",
    "gPauseSave21Tex",
    "gPauseSave22Tex",
    "gPauseSave23Tex",
    "gPauseSave24Tex",
];

/// Every page's background tiles, for the bakes: the four pages and the game over's prompt.
pub(super) const PAGE_BGS: [&[&str; 15]; 5] = [&ITEM_PAGE_BG, &EQUIP_PAGE_BG, &MAP_PAGE_BG, &QUEST_PAGE_BG, &GAME_OVER_PAGE_BG];

/// `sSavePromptMessageTexs[LANGUAGE_ENG]` (IA8 152x16), `sPromptChoiceTexs[LANGUAGE_ENG]` (IA8
/// 48x16): the prompts' labels (`icon_item_nes_static`).
pub(super) const SAVE_PROMPT_MESSAGE: (&str, u32) = ("gPauseSavePromptENGTex", 152);
pub(super) const PROMPT_CHOICES: [(&str, u32); 2] = [("gPauseYesENGTex", 48), ("gPauseNoENGTex", 48)];
/// `KALEIDO_PROMPT_CURSOR_R`, `_G`, `_B` (`PLATFORM_GC`: 100, 255, 100).
const KALEIDO_PROMPT_CURSOR: [i16; 3] = [100, 255, 100];
/// `PROMPT_QUAD_MESSAGE`, `_CURSOR_LEFT`, `_CURSOR_RIGHT`, `_CHOICE_YES`, `_CHOICE_NO`, times 4.
const PROMPT_QUAD_MESSAGE: usize = 0;
const PROMPT_QUAD_CURSOR_LEFT: usize = 4;
const PROMPT_QUAD_CURSOR_RIGHT: usize = 8;
const PROMPT_QUAD_CHOICE_YES: usize = 12;
const PROMPT_QUAD_CHOICE_NO: usize = 16;

/// `gAreaGsFlags`: by `mapIndex`, the Gold Skulltulas flags of an area all found.
pub(super) const AREA_GS_FLAGS: [u8; 22] = [0x0F, 0x1F, 0x0F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x07, 0x07, 0x03, 0x0F, 0x07, 0x0F, 0x0F, 0xFF, 0xFF, 0xFF, 0x1F, 0x0F, 0x03, 0x0F];

/// `sScrollLeftLabels`, `sScrollRightLabels` (English), by page.
const SCROLL_LEFT_LABELS: [&str; 4] = ["gPauseToEquipmentENGTex", "gPauseToSelectItemENGTex", "gPauseToMapENGTex", "gPauseToQuestStatusENGTex"];
const SCROLL_RIGHT_LABELS: [&str; 4] = ["gPauseToMapENGTex", "gPauseToQuestStatusENGTex", "gPauseToEquipmentENGTex", "gPauseToSelectItemENGTex"];

/// The labels the info panel draws (`icon_item_nes_static`), for the bakes.
pub(super) const LABELS: [(&str, i16); 7] = [
    ("gPauseToEquipENGTex", TO_EQUIP_WIDTH),
    ("gPauseToDecideENGTex", TO_DECIDE_WIDTH),
    ("gPauseToPlayMelodyENGTex", TO_PLAY_MELODY_WIDTH),
    ("gPauseToSelectItemENGTex", TO_PAGE_LABEL_TEX_WIDTH),
    ("gPauseToMapENGTex", TO_PAGE_LABEL_TEX_WIDTH),
    ("gPauseToQuestStatusENGTex", TO_PAGE_LABEL_TEX_WIDTH),
    ("gPauseToEquipmentENGTex", TO_PAGE_LABEL_TEX_WIDTH),
];

fn label(symbol: &'static str) -> KTex {
    let w = LABELS.iter().find(|l| l.0 == symbol).map(|l| l.1).unwrap_or(128);
    KTex::Label(symbol, w as u32)
}

/// `sPageBgColorRed`, `_Green`, `_Blue`: by `VtxPageInit`, the columns' edges.
const PAGE_BG_COLOR_RED: [[u8; 4]; 6] = [[10, 70, 70, 10], [10, 90, 90, 10], [80, 140, 140, 80], [80, 120, 120, 80], [80, 140, 140, 80], [50, 110, 110, 50]];
const PAGE_BG_COLOR_GREEN: [[u8; 4]; 6] = [[50, 100, 100, 50], [50, 100, 100, 50], [40, 60, 60, 40], [80, 120, 120, 80], [40, 60, 60, 40], [50, 110, 110, 50]];
const PAGE_BG_COLOR_BLUE: [[u8; 4]; 6] = [[80, 130, 130, 80], [40, 60, 60, 40], [30, 60, 60, 30], [50, 70, 70, 50], [30, 60, 60, 30], [50, 110, 110, 50]];

/// `sVtxPageMapDungeonQuads*`, `sVtxPageMapWorldQuads*` (`gVtxPageMapWorldQuadsWidth`,
/// `_Height`), `sVtxPagePromptQuads*`: the pages' extra quads.
const MAP_DUNGEON_QUADS: [[i16; 4]; VTX_PAGE_MAP_DUNGEON_QUADS] = [
    // x, width, y, height
    [-36, 48, 28, 85],
    [12, 48, 28, 85],
    [-18, 96, 46, 16],
    [70, 24, 28, 24],
    [70, 24, -2, 24],
    [70, 24, -32, 24],
    [-88, 24, 50, 16],
    [-88, 24, 36, 16],
    [-88, 24, 22, 16],
    [-88, 24, 8, 16],
    [-88, 24, -6, 16],
    [-88, 24, -20, 16],
    [-88, 24, -34, 16],
    [-88, 24, -48, 16],
    [-106, 16, 18, 16],
    [-62, 16, 18, 16],
    [-40, 24, 50, 24],
];
const MAP_WORLD_QUADS_X: [i16; VTX_PAGE_MAP_WORLD_QUADS] = [47, -49, -17, -15, -9, 24, 43, 14, 9, 38, 82, 71, -76, -87, -108, -54, -93, -67, -56, -33, -10, 1, 14, 24, 35, 58, 74, 89, 0, -58, 19, 28];
const MAP_WORLD_QUADS_WIDTH: [i16; VTX_PAGE_MAP_WORLD_QUADS] = [32, 112, 32, 48, 32, 32, 32, 48, 32, 64, 32, 48, 48, 48, 48, 64, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 80, 64];
const MAP_WORLD_QUADS_Y: [i16; VTX_PAGE_MAP_WORLD_QUADS] = [15, 40, 11, 45, 52, 37, 36, 57, 54, 33, 31, 45, 32, 42, 49, -10, 31, 27, 15, -49, 8, 38, 7, 47, 30, 1, -9, 25, 0, 1, -32, -26];
const MAP_WORLD_QUADS_HEIGHT: [i16; VTX_PAGE_MAP_WORLD_QUADS] = [24, 72, 13, 22, 19, 20, 19, 27, 14, 26, 22, 21, 49, 32, 45, 60, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 16, 32, 8];
const PROMPT_QUADS: [[i16; 4]; VTX_PAGE_PROMPT_QUADS] = [[-76, 152, 36, 16], [-58, 48, 10, 48], [10, 48, 10, 48], [-58, 48, -6, 16], [10, 48, -6, 16]];

/// `sVtxMapWorldAreaX`, `_Width`, `_Y`, `_Height`: by `WorldMapArea`.
const MAP_WORLD_AREA: [[i16; 4]; 21] = [
    [-58, 89, 1, 36],
    [11, 20, 15, 15],
    [30, 14, 20, 16],
    [30, 35, 9, 23],
    [15, 32, -30, 23],
    [38, 17, -17, 16],
    [-62, 50, -34, 24],
    [60, 16, 15, 13],
    [61, 21, 30, 17],
    [-78, 20, 1, 18],
    [-300, -1, -300, 1],
    [-86, 32, 42, 25],
    [-65, 16, 7, 13],
    [-300, -1, -300, 1],
    [-300, -1, -300, 1],
    [-21, 19, 24, 13],
    [14, 19, 36, 21],
    [13, 21, 53, 15],
    [20, 16, 37, 13],
    [-34, 20, -13, 12],
    [-300, -1, -300, 1],
];

/// `sItemVtxQuadsWithAmmo`: the ammo items' slots (`SLOT_*`), times 4.
const ITEM_VTX_QUADS_WITH_AMMO: [usize; 7] = [0, 4, 8, 12, 24, 32, 56];
/// `sEquipColumnsX`, `sEquipQuadsFirstByEquipType`.
const EQUIP_COLUMNS_X: [i16; 4] = [-114, 12, 44, 76];
const EQUIP_QUADS_FIRST_BY_EQUIP_TYPE: [usize; 4] = [1, 5, 9, 13];
/// `sQuestQuadsX`, `sQuestQuadsY`, `sQuestQuadsSize`.
const QUEST_QUADS_X: [i16; QUEST_QUAD_MAX] = [
    74, 74, 46, 18, 18, 46, -108, -90, -72, -54, -36, -18, -108, -90, -72, -54, -36, -18, 20, 46, 72, -110, -86, -110, -54, -98, -86, -74, -62, -50, -38, -26, -14, -98, -86, -74, -62, -50, -38, -26,
    -14, -88, -81, -72, -90, -83, -74,
];
const QUEST_QUADS_Y: [i16; QUEST_QUAD_MAX] = [
    38, 6, -12, 6, 38, 56, -20, -20, -20, -20, -20, -20, 2, 2, 2, 2, 2, 2, -46, -46, -46, 58, 58, 34, 58, -52, -52, -52, -52, -52, -52, -52, -52, -52, -52, -52, -52, -52, -52, -52, -52, 34, 34, 34,
    36, 36, 36,
];
const QUEST_QUADS_SIZE: [i16; QUEST_QUAD_MAX] =
    [24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 48, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16];
// `QuestItem`'s and `QuestQuad`'s indices.
const QUEST_SONG_MINUET: usize = 6;
const QUEST_KOKIRI_EMERALD: usize = 18;
const QUEST_SKULL_TOKEN: usize = 23;
const QUEST_HEART_PIECE: usize = 24;
const QUEST_QUAD_SKULL_TOKENS_DIGIT1_SHADOW: usize = 41;

/// `D_8082ACF4`: the cursor's env colours, by `cursorColorSet` plus the phase.
const CURSOR_ENV_COLORS: [[i16; 3]; 12] = [[0, 0, 0], [0, 0, 0], [0, 0, 0], [0, 0, 0], [255, 255, 0], [0, 0, 0], [0, 0, 0], [255, 255, 0], [0, 255, 50], [0, 0, 0], [0, 0, 0], [0, 255, 50]];
/// `sCursorColors` (`KALEIDO_COLOR_CURSOR_UNK` on GameCube: (0, 255, 50)).
const CURSOR_COLORS: [[i16; 3]; 3] = [[255, 255, 255], [255, 255, 0], [0, 255, 50]];
/// `sLRSelectedPrimColors`.
const LR_SELECTED_PRIM_COLORS: [[i16; 4]; 2] = [[180, 210, 255, 220], [100, 100, 150, 220]];

/// `ABS(a) / b` on the C's s16s.
fn step(a: i16, b: i16) -> i16 {
    if b == 0 {
        return 0;
    }
    a.wrapping_abs() / b
}

/// A colour component stepped towards `target` by `ABS(c - target) / timer`.
fn approach(c: &mut i16, target: i16, timer: i16) {
    let s = step(c.wrapping_sub(target), timer);
    if *c >= target {
        *c -= s;
    } else {
        *c += s;
    }
}

/// The page matrices of `KaleidoScope_DrawPages`: translated `depth` out on the page's side
/// (`R_PAUSE_DEPTH_OFFSET / 100`) and down to `R_PAUSE_PAGES_Y_ORIGIN_2 / 100`, scaled 0.78, then
/// turned by its pitch.
fn page_matrix(page: u16, depth: f32, y: f32, pitch: f32) -> Mat4 {
    let t = |x: f32, z: f32| Mat4::from_translation(Vec3::new(x, y, z)) * Mat4::from_scale(Vec3::splat(0.78));
    match page {
        PAUSE_ITEM => t(0.0, -depth) * Mat4::from_rotation_x(-pitch / 100.0),
        PAUSE_EQUIP => t(-depth, 0.0) * Mat4::from_rotation_z(pitch / 100.0) * Mat4::from_rotation_y(1.57),
        PAUSE_QUEST => t(0.0, depth) * Mat4::from_rotation_x(pitch / 100.0) * Mat4::from_rotation_y(3.14),
        _ => t(depth, 0.0) * Mat4::from_rotation_z(-pitch / 100.0) * Mat4::from_rotation_y(-1.57),
    }
}

/// `KaleidoScope_DrawPageSections`: the 15 background quads (32 vertices, then 28), each its
/// IA8 80x32 tile.
fn draw_page_sections(g: &mut KaleidoGfx, vertices: &[Vtx], textures: &[&'static str; 15]) {
    g.vertex(vertices, 32, 0);
    let mut i = 0;
    let mut j = 0;
    while j < 32 {
        g.quad(KTex::PageBg(textures[i]), j);
        j += 4;
        i += 1;
    }
    g.vertex(&vertices[32..], 28, 0);
    j = 0;
    while j < 28 {
        g.quad(KTex::PageBg(textures[i]), j);
        j += 4;
        i += 1;
    }
}

/// A quad's four vertices' positions: x0 for 0 and 2, x1 for 1 and 3, y0 for 0 and 1, y1 for 2
/// and 3.
fn set_quad_ob(v: &mut [Vtx], x0: i16, x1: i16, y0: i16, y1: i16) {
    v[0].ob[0] = x0;
    v[2].ob[0] = x0;
    v[1].ob[0] = x1;
    v[3].ob[0] = x1;
    v[0].ob[1] = y0;
    v[1].ob[1] = y0;
    v[2].ob[1] = y1;
    v[3].ob[1] = y1;
}

/// A quad's z (0), texture coordinates (0 to `s`, `t` texels) and colour.
fn set_quad_tc_cn(v: &mut [Vtx], s: i16, t: i16, cn: [u8; 4]) {
    for k in 0..4 {
        v[k].ob[2] = 0;
        v[k].cn = cn;
    }
    v[0].tc = [0, 0];
    v[1].tc = [s << 5, 0];
    v[2].tc = [0, t << 5];
    v[3].tc = [s << 5, t << 5];
}

impl PlayState {
    /// `KaleidoScope_SetDefaultCursor`: on the item page, an empty slot under the cursor moves it
    /// to the next item, or to none.
    pub(super) fn kaleido_scope_set_default_cursor(&mut self) {
        let p = &mut self.pause_ctx;
        if p.page_index == PAUSE_ITEM {
            let s = p.cursor_slot[PAUSE_ITEM as usize] as usize;
            let items = |i: usize| item::inventory_byte(&self.save, i);
            if items(s) == ITEM_NONE {
                // @bug (game): the first slot looked at is s + 1, 24 when s is the last: past
                // `items[24]`, the save's next bytes (`ammo[0]`).
                let mut i = s + 1;
                loop {
                    if items(i) != ITEM_NONE {
                        break;
                    }
                    i += 1;
                    if i >= 24 {
                        i = 0;
                    }
                    if i == s {
                        p.cursor_item[PAUSE_ITEM as usize] = PAUSE_ITEM_NONE;
                        p.named_item = PAUSE_ITEM_NONE;
                        return;
                    }
                }
                p.cursor_item[PAUSE_ITEM as usize] = items(i) as u16;
                p.cursor_slot[PAUSE_ITEM as usize] = i as u16;
            }
        }
    }

    /// `KaleidoScope_SetupPageSwitch`: the scroll to the page left (`pt` 0) or right (2), the
    /// cursor onto the arrow it comes in by, the next page's buttons.
    fn kaleido_scope_setup_page_switch(&mut self, pt: u8) {
        let p = &mut self.pause_ctx;
        p.main_state = PAUSE_MAIN_STATE_SWITCHING_PAGE;
        p.page_switch_timer = 0;
        if pt == PAGE_SWITCH_PT_LEFT {
            p.next_page_mode = p.page_index * 2 + 1;
            self.audio.play_sfx_centered(NA_SE_SY_WIN_SCROLL_LEFT);
            p.cursor_special_pos = PAUSE_CURSOR_PAGE_RIGHT;
        } else {
            p.next_page_mode = p.page_index * 2;
            self.audio.play_sfx_centered(NA_SE_SY_WIN_SCROLL_RIGHT);
            p.cursor_special_pos = PAUSE_CURSOR_PAGE_LEFT;
        }
        let status = PAGE_SWITCH_NEXT_BUTTON_STATUS[(p.page_index + pt as u16) as usize];
        // (PLATFORM_N64 || OOT_NTSC: buttonStatus[0] too. Not this version.)
        self.save.button_status[1..5].copy_from_slice(&status[1..5]);
        self.save.hud_visibility_mode = HUD_VISIBILITY_NO_CHANGE;
        crate::interface::change_alpha(&mut self.save, HUD_VISIBILITY_ALL);
    }

    /// `KaleidoScope_HandlePageToggles`: R right, Z left; on an arrow, the stick held outward
    /// turns the page after 10 frames (at once after a release).
    fn kaleido_scope_handle_page_toggles(&mut self) {
        let press = self.input.press;
        let p = &mut self.pause_ctx;
        if p.debug_state == PAUSE_DEBUG_STATE_CLOSED && press.held(BTN_L) {
            // DEBUG_FEATURES: PAUSE_DEBUG_STATE_INVENTORY_EDITOR_OPENING. The inventory editor
            // (z_kaleido_debug.c) isn't ported: the menu stays as it is (docs/adr/0047).
            log::info!("pause menu: L opens the inventory editor (KaleidoScope_DrawInventoryEditor), which isn't ported");
            return;
        }
        if press.held(BTN_R) {
            self.kaleido_scope_setup_page_switch(PAGE_SWITCH_PT_RIGHT);
            return;
        }
        if press.held(BTN_Z) {
            self.kaleido_scope_setup_page_switch(PAGE_SWITCH_PT_LEFT);
            return;
        }
        if p.cursor_special_pos == PAUSE_CURSOR_PAGE_LEFT {
            if p.stick_adj_x < -30 {
                p.page_switch_input_timer = p.page_switch_input_timer.wrapping_add(1);
                if p.page_switch_input_timer >= 10 || p.page_switch_input_timer == 0 {
                    self.kaleido_scope_setup_page_switch(PAGE_SWITCH_PT_LEFT);
                }
            } else {
                p.page_switch_input_timer = -1;
            }
        } else if p.cursor_special_pos == PAUSE_CURSOR_PAGE_RIGHT {
            if p.stick_adj_x > 30 {
                p.page_switch_input_timer = p.page_switch_input_timer.wrapping_add(1);
                if p.page_switch_input_timer >= 10 || p.page_switch_input_timer == 0 {
                    self.kaleido_scope_setup_page_switch(PAGE_SWITCH_PT_RIGHT);
                }
            } else {
                p.page_switch_input_timer = -1;
            }
        }
    }

    /// `KaleidoScope_UpdateNamePanel`: a new item under the cursor loads its name and restarts
    /// the panel's timer (the name shows for `WREG(89)` of every `WREG(88)` frames).
    fn kaleido_scope_update_name_panel(&mut self) {
        let p = &mut self.pause_ctx;
        let page = p.page_index as usize;
        if p.named_item != p.cursor_item[page] || (p.page_index == PAUSE_MAP && p.cursor_special_pos != 0) {
            p.named_item = p.cursor_item[page];
            if p.named_item != PAUSE_ITEM_NONE {
                // The name's DMA into nameSegment (item_name_static, or map_name_static's point
                // names on the world map; English: no language offset). The draw reads
                // `namedItem`.
                p.name_display_timer = 0;
            }
        } else if p.name_color_set == 0 {
            let quest_song = p.page_index == PAUSE_QUEST && (6..=0x11).contains(&p.cursor_slot[PAUSE_QUEST as usize]) && p.main_state == PAUSE_MAIN_STATE_IDLE_CURSOR_ON_SONG;
            let equip = p.page_index == PAUSE_EQUIP && p.cursor_x[PAUSE_EQUIP as usize] != 0;
            if quest_song || p.page_index == PAUSE_ITEM || equip {
                if p.named_item != ITEM_SOLD_OUT as u16 {
                    p.name_display_timer = p.name_display_timer.wrapping_add(1);
                    if p.name_display_timer > p.regs.wreg88 as u16 {
                        p.name_display_timer = 0;
                    }
                }
            } else {
                p.name_display_timer = 0;
            }
        } else {
            p.name_display_timer = 0;
        }
    }

    /// `KaleidoScope_UpdatePageSwitch`: the eye's 16 steps round to the next page, the L and R
    /// buttons out for the first half and back.
    fn kaleido_scope_update_page_switch(&mut self) {
        let p = &mut self.pause_ctx;
        // R_PAUSE_PAGE_SWITCH_FRAME_ADVANCE_ON: a debug frame advance on L.
        if p.regs.page_switch_frame_advance_on && !self.input.press.held(BTN_L) {
            return;
        }
        let (dx, dz) = page_switch_eye_d(p.next_page_mode);
        p.eye.x += dx;
        p.eye.z += dz;
        let r = &mut p.regs;
        if p.page_switch_timer < (4 * PAGE_SWITCH_NSTEPS) / 2 {
            r.button_left_x -= r.button_left_move_offset_x / r.ui_anims_duration;
            r.button_right_x -= r.button_right_move_offset_x / r.ui_anims_duration;
        } else {
            r.button_left_x += r.button_left_move_offset_x / r.ui_anims_duration;
            r.button_right_x += r.button_right_move_offset_x / r.ui_anims_duration;
        }
        p.page_switch_timer += 4;
        if p.page_switch_timer == 4 * PAGE_SWITCH_NSTEPS {
            p.page_switch_timer = 0;
            p.page_index = PAGE_SWITCH_NEXT_PAGE_INDEX[p.next_page_mode as usize];
            p.main_state = PAUSE_MAIN_STATE_IDLE;
        }
    }

    /// `KaleidoScope_UpdateOpening`: the opening's scroll onto the page; at its end the page's
    /// buttons, `PAUSE_STATE_MAIN`, and "SAVE" on B.
    fn kaleido_scope_update_opening(&mut self) {
        let p = &mut self.pause_ctx;
        let (dx, dz) = page_switch_eye_d(p.next_page_mode);
        let z46 = p.regs.zreg46;
        p.eye.x += dx * z46 as f32;
        p.eye.z += dz * z46 as f32;
        p.page_switch_timer = p.page_switch_timer.wrapping_add((4 * z46) as u16);
        if p.page_switch_timer == 4 * PAGE_SWITCH_NSTEPS * p.regs.zreg47 as u16 {
            // Finished opening.
            crate::interface::func_80084bf4(&mut self.save);
            let status = PAGE_SWITCH_NEXT_BUTTON_STATUS[(p.page_index + PAGE_SWITCH_PT_LEFT as u16) as usize];
            self.save.button_status = status;
            p.page_index = PAGE_SWITCH_NEXT_PAGE_INDEX[p.next_page_mode as usize];
            p.main_state = PAUSE_MAIN_STATE_IDLE;
            p.state += 1;
            p.alpha = 255;
            self.interface_ctx.load_action_label_b(DO_ACTION_SAVE);
        } else if p.page_switch_timer == 4 * PAGE_SWITCH_NSTEPS {
            // ZREG(47) is 1, so this never happens.
            p.page_index = PAGE_SWITCH_NEXT_PAGE_INDEX[p.next_page_mode as usize];
            p.next_page_mode = (p.page_index << 1) + 1;
        }
    }

    /// `KaleidoScope_UpdateCursorVtx`: the cursor's four corners from its top left
    /// (`cursorVtx[0]`) by the page's and slot's offsets.
    pub(super) fn kaleido_scope_update_cursor_vtx(&mut self) {
        let p = &mut self.pause_ctx;
        let (mut tl_x, mut tl_y, mut right_x, mut bottom_y);
        if p.cursor_special_pos == 0 {
            tl_x = -1;
            tl_y = 1;
            right_x = 14;
            bottom_y = 14;
            let slot = p.cursor_slot[p.page_index.min(3) as usize] as usize;
            if p.page_index == PAUSE_MAP {
                if !p.statics.in_dungeon_scene {
                    tl_x = -6;
                    tl_y = 6;
                    right_x = 4;
                    bottom_y = 4;
                } else if slot >= 3 {
                    tl_x = -6;
                    tl_y = 5;
                    bottom_y = 7;
                    right_x = 19;
                } else {
                    tl_x = -3;
                    tl_y = 3;
                    right_x = 13;
                    bottom_y = 13;
                }
            } else if p.page_index == PAUSE_QUEST {
                tl_x = -4;
                tl_y = 4;
                right_x = 12;
                bottom_y = 12;
                if slot == QUEST_HEART_PIECE {
                    tl_x = -2;
                    tl_y = 2;
                    right_x = 32;
                    bottom_y = 32;
                } else if slot == QUEST_SKULL_TOKEN {
                    tl_x = -4;
                    tl_y = 4;
                    bottom_y = 13;
                    right_x = 34;
                } else if slot < QUEST_SONG_MINUET {
                    tl_x = -1;
                    tl_y = 1;
                    right_x = 10;
                    bottom_y = 10;
                } else if (QUEST_SONG_MINUET..QUEST_KOKIRI_EMERALD).contains(&slot) {
                    tl_x = -5;
                    tl_y = 3;
                    right_x = 8;
                    bottom_y = 8;
                }
            }
        } else {
            tl_x = -4;
            tl_y = 4;
            right_x = 16;
            bottom_y = 16;
        }
        let c = &mut p.cursor_vtx;
        let x0 = c[0].ob[0].wrapping_add(tl_x);
        let y0 = c[0].ob[1].wrapping_add(tl_y);
        // PAUSE_CURSOR_QUAD_TL, TR, BL, BR.
        set_quad_ob(&mut c[0..4], x0, x0 + 16, y0, y0 - 16);
        set_quad_ob(&mut c[4..8], x0 + right_x, x0 + right_x + 16, y0, y0 - 16);
        set_quad_ob(&mut c[8..12], x0, x0 + 16, y0 - bottom_y, y0 - bottom_y - 16);
        set_quad_ob(&mut c[12..16], x0 + right_x, x0 + right_x + 16, y0 - bottom_y, y0 - bottom_y - 16);
    }

    /// `KaleidoScope_Update`.
    pub(super) fn kaleido_scope_update(&mut self) {
        let input = self.input;
        let in_range = |s: u16| (PAUSE_STATE_OPENING_1..=PAUSE_STATE_SAVE_PROMPT).contains(&s) || (PAUSE_STATE_GAME_OVER_INIT..=PAUSE_STATE_CLOSING).contains(&s);
        if self.pause_ctx.bg_prerender_state >= PAUSE_BG_PRERENDER_READY && in_range(self.pause_ctx.state) {
            let p = &mut self.pause_ctx;
            if (p.main_state == PAUSE_MAIN_STATE_IDLE || p.main_state == PAUSE_MAIN_STATE_IDLE_CURSOR_ON_SONG) && p.state == PAUSE_STATE_MAIN {
                p.stick_adj_x = input.rel.stick_x as i16;
                p.stick_adj_y = input.rel.stick_y as i16;
                // KaleidoScope_UpdateCursorVtx: the port spreads the cursor at the end of the
                // last frame's draw, whose vertices these are (see gfx.rs).
                self.kaleido_scope_handle_page_toggles();
            } else if p.page_index == PAUSE_QUEST && (p.main_state < PAUSE_MAIN_STATE_3 || p.main_state == PAUSE_MAIN_STATE_SONG_PROMPT) {
                // KaleidoScope_UpdateCursorVtx: as above.
            }
            if self.pause_ctx.state == PAUSE_STATE_MAIN {
                self.kaleido_scope_update_name_panel();
            }
        }
        match self.pause_ctx.state {
            PAUSE_STATE_INIT => self.kaleido_scope_update_init(),
            PAUSE_STATE_OPENING_1 => {
                if self.pause_ctx.item_page_pitch == 160.0 {
                    // The first frame in this state.
                    self.kaleido_scope_set_default_cursor();
                    // KaleidoScope_ProcessPlayerPreRender: the equipment page's Link (logged).
                }
                let p = &mut self.pause_ctx;
                let d = p.regs.ui_anims_duration;
                let pitch = p.quest_page_pitch - 160.0 / d as f32;
                (p.item_page_pitch, p.equip_page_pitch, p.map_page_pitch, p.quest_page_pitch) = (pitch, pitch, pitch, pitch);
                p.info_panel_offset_y += 40 / d;
                self.interface_ctx.start_alpha += 255 / d;
                let r = &mut p.regs;
                r.button_left_x += r.button_left_move_offset_x / d;
                r.button_right_x += r.button_right_move_offset_x / d;
                r.xreg5 += 150 / d;
                p.alpha = p.alpha.wrapping_add((255 / (d + r.wreg4)) as u16);
                if p.item_page_pitch == 0.0 {
                    self.interface_ctx.start_alpha = 255;
                    p.regs.pages_y_origin_2 = 0;
                    p.state = PAUSE_STATE_OPENING_2;
                }
                self.kaleido_scope_update_opening();
            }
            PAUSE_STATE_OPENING_2 => {
                let p = &mut self.pause_ctx;
                p.alpha = p.alpha.wrapping_add((255 / (p.regs.ui_anims_duration + p.regs.wreg4)) as u16);
                self.kaleido_scope_update_opening();
                if self.pause_ctx.state == PAUSE_STATE_MAIN {
                    self.kaleido_scope_update_name_panel();
                }
            }
            PAUSE_STATE_MAIN => self.kaleido_scope_update_main(),
            PAUSE_STATE_SAVE_PROMPT => {
                // The save prompt is milestone 5c's: B logs instead of opening it (see the main
                // state), so this state isn't reached.
            }
            PAUSE_STATE_GAME_OVER_INIT..=PAUSE_STATE_GAME_OVER_FINISH => self.kaleido_scope_update_game_over(),
            PAUSE_STATE_CLOSING => {
                let p = &mut self.pause_ctx;
                if p.item_page_pitch != 160.0 {
                    let d = p.regs.ui_anims_duration;
                    let pitch = p.quest_page_pitch + 160.0 / d as f32;
                    (p.item_page_pitch, p.equip_page_pitch, p.map_page_pitch, p.quest_page_pitch) = (pitch, pitch, pitch, pitch);
                    p.info_panel_offset_y -= 40 / d;
                    self.interface_ctx.start_alpha -= 255 / d;
                    let r = &mut p.regs;
                    r.button_left_x -= r.button_left_move_offset_x / d;
                    r.button_right_x -= r.button_right_move_offset_x / d;
                    r.xreg5 -= 150 / d;
                    p.alpha = p.alpha.wrapping_sub((255 / d) as u16);
                    if p.item_page_pitch == 160.0 {
                        p.alpha = 0;
                    }
                } else {
                    p.debug_state = PAUSE_DEBUG_STATE_CLOSED;
                    p.state = PAUSE_STATE_RESUME_GAMEPLAY;
                    (p.quest_page_pitch, p.map_page_pitch, p.equip_page_pitch, p.item_page_pitch) = (160.0, 160.0, 160.0, 160.0);
                    p.named_item = PAUSE_ITEM_NONE;
                    self.interface_ctx.start_alpha = 0;
                }
            }
            PAUSE_STATE_RESUME_GAMEPLAY => self.kaleido_scope_update_resume_gameplay(),
            _ => {}
        }
    }

    /// `KaleidoScope_Update`'s `PAUSE_STATE_INIT`: the buttons saved, the map's cursor on Link's
    /// floor, the textures loaded (the wrong age's icons greyed: their bakes), the A button's
    /// "DECIDE", the equipment page's Link, the world map's points.
    fn kaleido_scope_update_init(&mut self) {
        let p = &mut self.pause_ctx;
        p.statics.saved_button_status = self.save.button_status;
        p.cursor_x[PAUSE_MAP as usize] = 0;
        let floor_slot = self.map.floor + 3;
        p.cursor_slot[PAUSE_MAP as usize] = floor_slot as u16;
        p.cursor_point[PAUSE_MAP as usize] = floor_slot;
        p.dungeon_map_slot = floor_slot;
        p.regs.button_left_x = -175;
        p.regs.button_right_x = 155;
        p.prompt_pitch = -314.0;
        // (playerSegment, Player_InitPauseDrawData, the icon_item_static DMA, the greying of the
        // icons !CHECK_AGE_REQ_ITEM: the draw picks the greyed bakes; icon_item_24_static.)
        // The ten dungeons and their boss rooms: icon_item_dungeon_static; else
        // icon_item_field_static.
        p.statics.in_dungeon_scene = crate::map::is_dungeon_or_boss(self.scene_id);
        if p.statics.in_dungeon_scene {
            // (icon_item_dungeon_static.)
            self.map.map_palette[28] = 6;
            self.map.map_palette[29] = 99;
            self.kaleido_scope_update_dungeon_map();
        }
        // (icon_item_field_static, icon_item_nes_static, nameSegment.)
        self.interface_set_do_action_paused(DO_ACTION_DECIDE);
        // (gSaveContext.worldMapArea's name into nameSegment: the world map's.)
        let p = &mut self.pause_ctx;
        p.log_once(1, "pause menu: the equipment page's Link (KaleidoScope_DrawPlayerWork, the player prerender) isn't ported");
        // The world map's points and the trade quest marker: the world map's contents.
        p.log_once(2, "pause menu: the world map's points (worldMapPoints) and the trade quest marker aren't ported");
        p.world_map_points = [0; 20];
        p.trade_quest_marker = TRADE_QUEST_MARKER_NONE;
        p.state = PAUSE_STATE_OPENING_1;
    }

    /// `Interface_SetDoAction` while paused (`pauseCtx->state != PAUSE_STATE_OFF`): the flip
    /// starts from its second half's label (`unk_1EC` 3).
    pub(crate) fn interface_set_do_action_paused(&mut self, action: u16) {
        if self.interface_ctx.unk_1f0 != action {
            self.interface_ctx.set_do_action(action);
            if self.pause_ctx.state != PAUSE_STATE_OFF {
                self.interface_ctx.unk_1ec = 3;
            }
        }
    }

    /// `KaleidoScope_Update`'s `PAUSE_STATE_MAIN`.
    fn kaleido_scope_update_main(&mut self) {
        let press = self.input.press;
        match self.pause_ctx.main_state {
            PAUSE_MAIN_STATE_IDLE => {
                if press.held(BTN_START) {
                    self.interface_set_do_action_paused(DO_ACTION_NONE);
                    self.pause_ctx.state = PAUSE_STATE_CLOSING;
                    self.pause_ctx.regs.pages_y_origin_2 = PAUSE_PAGES_Y_ORIGIN_2_LOWER;
                    self.audio.func_800f64e0(0);
                    // (PLATFORM_GC && OOT_NTSC: AudioOcarina_SetInstrument. Not this version.)
                } else if press.held(BTN_B) {
                    self.kaleido_scope_save_prompt();
                }
            }
            PAUSE_MAIN_STATE_SWITCHING_PAGE => self.kaleido_scope_update_page_switch(),
            PAUSE_MAIN_STATE_SONG_PLAYBACK => {
                // pauseCtx->ocarinaStaff = AudioOcarina_GetPlaybackStaff(): the ocarina isn't
                // ported; its playback's state reads 0, done.
                self.pause_ctx.main_state = PAUSE_MAIN_STATE_SONG_PROMPT_INIT;
                log::info!("pause menu: AudioOcarina_SetInstrument(OCARINA_INSTRUMENT_OFF) (the ocarina isn't ported)");
            }
            PAUSE_MAIN_STATE_3 => self.kaleido_scope_update_item_equip(),
            PAUSE_MAIN_STATE_SONG_PROMPT_INIT => {}
            PAUSE_MAIN_STATE_SONG_PROMPT => {
                // AudioOcarina_GetPlayingStaff: not ported (no state: neither the song's index
                // nor 0xFF).
                if press.held(BTN_START) {
                    log::info!("pause menu: AudioOcarina_SetInstrument(OCARINA_INSTRUMENT_OFF) (the ocarina isn't ported)");
                    self.interface_set_do_action_paused(DO_ACTION_NONE);
                    self.pause_ctx.state = PAUSE_STATE_CLOSING;
                    self.pause_ctx.regs.pages_y_origin_2 = PAUSE_PAGES_Y_ORIGIN_2_LOWER;
                    self.audio.func_800f64e0(0);
                    self.pause_ctx.main_state = PAUSE_MAIN_STATE_IDLE;
                } else if press.held(BTN_B) {
                    log::info!("pause menu: AudioOcarina_SetInstrument(OCARINA_INSTRUMENT_OFF) (the ocarina isn't ported)");
                    self.pause_ctx.main_state = PAUSE_MAIN_STATE_IDLE;
                    self.kaleido_scope_save_prompt();
                }
            }
            PAUSE_MAIN_STATE_SONG_PROMPT_DONE => {
                let p = &mut self.pause_ctx;
                p.statics.delay_timer -= 1;
                if p.statics.delay_timer == 0 {
                    p.main_state = p.statics.main_state_after_song_player_playing_done;
                    if p.main_state == PAUSE_MAIN_STATE_IDLE {
                        log::info!("pause menu: AudioOcarina_SetInstrument(OCARINA_INSTRUMENT_OFF) (the ocarina isn't ported)");
                    }
                }
            }
            PAUSE_MAIN_STATE_EQUIP_CHANGED => {}
            PAUSE_MAIN_STATE_IDLE_CURSOR_ON_SONG => {
                if press.held(BTN_START) {
                    log::info!("pause menu: AudioOcarina_SetInstrument(OCARINA_INSTRUMENT_OFF) (the ocarina isn't ported)");
                    self.interface_set_do_action_paused(DO_ACTION_NONE);
                    self.pause_ctx.state = PAUSE_STATE_CLOSING;
                    self.pause_ctx.regs.pages_y_origin_2 = PAUSE_PAGES_Y_ORIGIN_2_LOWER;
                    self.audio.func_800f64e0(0);
                    self.pause_ctx.main_state = PAUSE_MAIN_STATE_IDLE;
                } else if press.held(BTN_B) {
                    log::info!("pause menu: AudioOcarina_SetInstrument(OCARINA_INSTRUMENT_OFF) (the ocarina isn't ported)");
                    self.pause_ctx.main_state = PAUSE_MAIN_STATE_IDLE;
                    self.kaleido_scope_save_prompt();
                }
            }
            PAUSE_MAIN_STATE_SONG_PLAYBACK_START => {}
            _ => self.pause_ctx.main_state = PAUSE_MAIN_STATE_IDLE,
        }
        let _ = (NA_SE_SY_TRE_BOX_APPEAR, NA_SE_SY_OCARINA_ERROR);
    }

    /// B's save prompt (`nextPageMode` 0, `promptChoice` 0, `NA_SE_SY_DECIDE`, the buttons
    /// disabled but A, `PAUSE_STATE_SAVE_PROMPT`): milestone 5c's, logged; the menu stays.
    fn kaleido_scope_save_prompt(&mut self) {
        log::info!("pause menu: B opens the save prompt (PAUSE_STATE_SAVE_PROMPT), which is milestone 5c's: the menu stays");
    }

    /// `KaleidoScope_Update`'s `PAUSE_STATE_RESUME_GAMEPLAY`: the game resumes at 20 frames a
    /// second, the dungeon's minimap reloaded, the buttons and the HUD as they were, Player's
    /// equipment from the save.
    fn kaleido_scope_update_resume_gameplay(&mut self) {
        self.pause_ctx.state = PAUSE_STATE_OFF;
        self.r_update_rate = 3;
        self.pause_ctx.bg_prerender_state = PAUSE_BG_PRERENDER_OFF;
        // (Object_ReloadAll and func_800418D0: the menu borrowed no object or collision space.)
        if crate::map::is_dungeon_or_boss(self.scene_id) {
            let room = self.map.map_room_num;
            self.map_init_data(room);
        }
        self.save.button_status = self.pause_ctx.statics.saved_button_status;
        self.interface_ctx.unk_1fa = false;
        self.interface_ctx.unk_1fc = 0;
        self.save.hud_visibility_mode = HUD_VISIBILITY_NO_CHANGE;
        let prev = self.save.prev_hud_visibility_mode;
        crate::interface::change_alpha(&mut self.save, prev);
        // Not in the C: the equipment page's stand-in (its A button isn't ported): what's owned
        // and unworn goes on (docs/adr/0021, docs/adr/0047).
        if self.save.equip_owned_unworn() {
            let b = self.save.equips.button_items;
            log::info!("equipped (the equipment page's stand-in): equipment {:#06x}, B {:#04x}", self.save.equips.equipment, b[0]);
        }
        // player->talkActor = NULL; Player_SetEquipmentData.
        let (data, save) = (self.data.clone(), self.save.clone());
        if let Some(pl) = self.player.and_then(|h| self.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
            pl.clear_talk_actor();
            pl.set_equipment_data(&data, &save);
        }
    }

    /// `KaleidoScope_Draw`: the pages under the menu's view (`pauseCtx->eye`), then the UI
    /// overlay under the view from (0, 0, 64); the game over's message.
    pub(super) fn kaleido_scope_draw(&mut self) {
        let p = &mut self.pause_ctx;
        p.stick_adj_x = self.input.rel.stick_x as i16;
        p.stick_adj_y = self.input.rel.stick_y as i16;
        // (The segments: parameter_static, the player image, the icon files, the name, the
        // language file.)
        if p.debug_state == PAUSE_DEBUG_STATE_CLOSED {
            let eye = p.eye;
            p.gfx.set_view(eye);
            self.kaleido_scope_set_vertices();
            self.kaleido_scope_draw_pages();
            let p = &mut self.pause_ctx;
            p.gfx.combine(Cc::PrimEnvTexel);
            p.gfx.set_view(Vec3::new(0.0, 0.0, 64.0));
            if !p.is_game_over() {
                self.kaleido_scope_draw_ui_overlay();
            }
        }
        let p = &mut self.pause_ctx;
        if (PAUSE_STATE_GAME_OVER_SHOW_MESSAGE..=PAUSE_STATE_GAME_OVER_FINISH).contains(&p.state) {
            self.kaleido_scope_draw_game_over();
        }
        // (The inventory editor's draw: L doesn't open it, see KaleidoScope_HandlePageToggles.)
        // KaleidoScope_UpdateCursorVtx, as the next frame's update runs it on these vertices
        // before the RSP reads them (gfx.rs): under that update's condition.
        let p = &self.pause_ctx;
        let in_range = (PAUSE_STATE_OPENING_1..=PAUSE_STATE_SAVE_PROMPT).contains(&p.state) || (PAUSE_STATE_GAME_OVER_INIT..=PAUSE_STATE_CLOSING).contains(&p.state);
        let idle = (p.main_state == PAUSE_MAIN_STATE_IDLE || p.main_state == PAUSE_MAIN_STATE_IDLE_CURSOR_ON_SONG) && p.state == PAUSE_STATE_MAIN;
        let quest = p.page_index == PAUSE_QUEST && (p.main_state < PAUSE_MAIN_STATE_3 || p.main_state == PAUSE_MAIN_STATE_SONG_PROMPT);
        if p.bg_prerender_state >= PAUSE_BG_PRERENDER_READY && in_range && (idle || quest) {
            self.kaleido_scope_update_cursor_vtx();
        }
    }

    /// `KaleidoScope_DrawPages`: the cursor's colour cycle and the stick's repeat (input in the
    /// draw), the three other pages, the active page with its cursor, the prompt page.
    fn kaleido_scope_draw_pages(&mut self) {
        let p = &mut self.pause_ctx;
        if !p.is_game_over() {
            if p.state != PAUSE_STATE_SAVE_PROMPT {
                let s = &mut p.statics;
                let target = CURSOR_ENV_COLORS[(p.cursor_color_set + s.d_8082ad40) as usize];
                let timer = s.d_8082ad3c;
                for k in 0..3 {
                    approach(&mut s.d_8082ab8c[k], target[k], timer);
                }
                s.d_8082ad3c -= 1;
                if s.d_8082ad3c == 0 {
                    s.d_8082ab8c[0..3].copy_from_slice(&target);
                    s.d_8082ad3c = p.regs.zreg28[s.d_8082ad40 as usize];
                    s.d_8082ad40 += 1;
                    if s.d_8082ad40 >= 4 {
                        s.d_8082ad40 = 0;
                    }
                }
                // The stick's repeat: a first push passes, then again after
                // R_PAUSE_STICK_REPEAT_DELAY_FIRST frames and every R_PAUSE_STICK_REPEAT_DELAY + 1.
                let (first, delay) = (p.regs.stick_repeat_delay_first, p.regs.stick_repeat_delay);
                let repeat = |adj: &mut i16, timer: &mut i16, state: &mut i16| {
                    let dir = if *adj < -30 {
                        -1
                    } else if *adj > 30 {
                        1
                    } else {
                        *state = 0;
                        return;
                    };
                    if *state == dir {
                        *timer -= 1;
                        if *timer < 0 {
                            *timer = delay;
                        } else {
                            *adj = 0;
                        }
                    } else {
                        *timer = first;
                        *state = dir;
                    }
                };
                repeat(&mut p.stick_adj_x, &mut s.stick_x_repeat_timer, &mut s.stick_x_repeat_state);
                repeat(&mut p.stick_adj_y, &mut s.stick_y_repeat_timer, &mut s.stick_y_repeat_state);
            }

            // The pages not looked at.
            let depth = p.regs.depth_offset as f32 / 100.0;
            let y2 = p.regs.pages_y_origin_2 as f32 / 100.0;
            if p.page_index != PAUSE_ITEM {
                p.gfx.combine(Cc::ModulateIa);
                p.gfx.matrix(page_matrix(PAUSE_ITEM, depth, y2, p.item_page_pitch));
                draw_page_sections(&mut p.gfx, &p.item_page_vtx, &ITEM_PAGE_BG);
                self.kaleido_scope_draw_item_select();
            }
            let p = &mut self.pause_ctx;
            if p.page_index != PAUSE_EQUIP {
                p.gfx.combine(Cc::ModulateIa);
                p.gfx.matrix(page_matrix(PAUSE_EQUIP, depth, y2, p.equip_page_pitch));
                draw_page_sections(&mut p.gfx, &p.equip_page_vtx, &EQUIP_PAGE_BG);
                self.kaleido_scope_draw_equipment();
            }
            let p = &mut self.pause_ctx;
            if p.page_index != PAUSE_QUEST {
                // (gDPSetTextureFilter(G_TF_BILERP): every page is bilinear.)
                p.gfx.combine(Cc::ModulateIa);
                p.gfx.matrix(page_matrix(PAUSE_QUEST, depth, y2, p.quest_page_pitch));
                draw_page_sections(&mut p.gfx, &p.quest_page_vtx, &QUEST_PAGE_BG);
                self.kaleido_scope_draw_quest_status();
            }
            let p = &mut self.pause_ctx;
            if p.page_index != PAUSE_MAP {
                p.gfx.combine(Cc::ModulateIa);
                p.gfx.matrix(page_matrix(PAUSE_MAP, depth, y2, p.map_page_pitch));
                draw_page_sections(&mut p.gfx, &p.map_page_vtx, &MAP_PAGE_BG);
                self.kaleido_scope_draw_map_contents(false);
            }

            // The page looked at.
            let p = &mut self.pause_ctx;
            p.gfx.combine(Cc::ModulateIa);
            match p.page_index {
                PAUSE_ITEM => {
                    p.gfx.matrix(page_matrix(PAUSE_ITEM, depth, y2, p.item_page_pitch));
                    draw_page_sections(&mut p.gfx, &p.item_page_vtx, &ITEM_PAGE_BG);
                    self.kaleido_scope_draw_item_select();
                }
                PAUSE_MAP => {
                    p.gfx.matrix(page_matrix(PAUSE_MAP, depth, y2, p.map_page_pitch));
                    draw_page_sections(&mut p.gfx, &p.map_page_vtx, &MAP_PAGE_BG);
                    self.kaleido_scope_draw_map_contents(true);
                }
                PAUSE_QUEST => {
                    p.gfx.matrix(page_matrix(PAUSE_QUEST, depth, y2, p.quest_page_pitch));
                    draw_page_sections(&mut p.gfx, &p.quest_page_vtx, &QUEST_PAGE_BG);
                    self.kaleido_scope_draw_quest_status();
                    if self.pause_ctx.cursor_special_pos == 0 {
                        self.kaleido_scope_draw_cursor(PAUSE_QUEST);
                    }
                }
                _ => {
                    p.gfx.matrix(page_matrix(PAUSE_EQUIP, depth, y2, p.equip_page_pitch));
                    draw_page_sections(&mut p.gfx, &p.equip_page_vtx, &EQUIP_PAGE_BG);
                    self.kaleido_scope_draw_equipment();
                    if self.pause_ctx.cursor_special_pos == 0 {
                        self.kaleido_scope_draw_cursor(PAUSE_EQUIP);
                    }
                }
            }
        }

        // The prompt: the game over's; the save prompt's is milestone 5c's (B doesn't open it).
        let p = &mut self.pause_ctx;
        if p.state == PAUSE_STATE_SAVE_PROMPT || p.is_game_over() {
            self.kaleido_scope_update_prompt();
            if self.pause_ctx.is_game_over() {
                self.kaleido_scope_draw_game_over_prompt();
            } else {
                self.pause_ctx.log_once(4, "pause menu: the save prompt's page (KaleidoScope_DrawPages' save prompt) is milestone 5c's");
            }
        }
    }

    /// `KaleidoScope_DrawPages`' prompt page in a game over: under the page looked at's matrix,
    /// turned by `promptPitch` (the page itself turned half a turn on), the 15 tiles
    /// (`sGameOverTexs`), then "Would you like to save?" or "Continue playing?" with the cursor
    /// on the choice and "Yes", "No".
    fn kaleido_scope_draw_game_over_prompt(&mut self) {
        let p = &mut self.pause_ctx;
        // Gfx_SetupDL_42Opa, G_CC_MODULATEIA.
        p.gfx.combine(Cc::ModulateIa);
        let pitch = p.prompt_pitch;
        match p.page_index {
            PAUSE_ITEM => p.item_page_pitch = pitch + 314.0,
            PAUSE_MAP => p.map_page_pitch = pitch + 314.0,
            PAUSE_QUEST => p.quest_page_pitch = pitch + 314.0,
            _ => p.equip_page_pitch = pitch + 314.0,
        }
        let y2 = p.regs.pages_y_origin_2 as f32 / 100.0;
        p.gfx.matrix(page_matrix(p.page_index, p.prompt_depth_offset / 10.0, y2, pitch));
        draw_page_sections(&mut p.gfx, &p.prompt_page_vtx, &GAME_OVER_PAGE_BG);
        // @bug (game): loads 32 vertices where there are 20 (the 12 after are whatever follows
        // in the frame's memory; nothing draws with them).
        p.gfx.vertex(&p.prompt_page_vtx[PAGE_BG_QUADS * 4..], 32, 0);
        let message = match p.state {
            PAUSE_STATE_GAME_OVER_SAVE_PROMPT => Some(KTex::Label(SAVE_PROMPT_MESSAGE.0, SAVE_PROMPT_MESSAGE.1)),
            // PAUSE_STATE_GAME_OVER_SAVED: "Game saved." is !PLATFORM_GC's (sSaveConfirmationTexs).
            PAUSE_STATE_GAME_OVER_CONTINUE_PROMPT | PAUSE_STATE_GAME_OVER_FINISH => Some(KTex::ContinuePlaying),
            _ => None,
        };
        if let Some(message) = message {
            // KaleidoScope_QuadTextureIA8(message, 152, 16, PROMPT_QUAD_MESSAGE * 4).
            p.gfx.quad(message, PROMPT_QUAD_MESSAGE);
            p.gfx.combine(Cc::PrimTexelAlpha);
            let c = KALEIDO_PROMPT_CURSOR;
            p.gfx.prim_color(c[0], c[1], c[2], p.regs.prompt_cursor_alpha);
            // gPromptCursorLeftDL or gPromptCursorRightDL: gPausePromptCursorTex on the quad.
            p.gfx.quad(KTex::PromptCursor, if p.prompt_choice == 0 { PROMPT_QUAD_CURSOR_LEFT } else { PROMPT_QUAD_CURSOR_RIGHT });
            p.gfx.combine(Cc::ModulateIa);
            p.gfx.prim_color(255, 255, 255, p.alpha as i16);
            p.gfx.quad(KTex::Label(PROMPT_CHOICES[0].0, PROMPT_CHOICES[0].1), PROMPT_QUAD_CHOICE_YES);
            p.gfx.quad(KTex::Label(PROMPT_CHOICES[1].0, PROMPT_CHOICES[1].1), PROMPT_QUAD_CHOICE_NO);
        }
        p.gfx.combine(Cc::PrimEnvTexel);
        if p.state != PAUSE_STATE_GAME_OVER_CONTINUE_PROMPT && p.state != PAUSE_STATE_GAME_OVER_FINISH {
            p.gfx.prim_color(255, 255, 0, p.alpha as i16);
            p.gfx.env_color(0, 0, 0, 0);
        }
    }

    /// `KaleidoScope_DrawGameOver`: "GAME OVER" in three 64x32 parts at (`VREG(87)`,
    /// `VREG(88)`), each blended with the mask on tile 1 (`PRIM_LOD_FRAC` 80), the mask scrolling
    /// up 2 (10.2) a frame (`VREG(89)`), coloured from env to prim (`D_8082AB8C`..).
    fn kaleido_scope_draw_game_over(&mut self) {
        let p = &mut self.pause_ctx;
        // Gfx_SetupDL_39Opa, G_CYC_2CYCLE, G_RM_PASS and G_RM_XLU_SURF2, the combiner: the bakes'.
        let c = p.statics.d_8082ab8c;
        p.gfx.prim_color(c[0], c[1], c[2], c[3]);
        p.gfx.env_color(c[4], c[5], c[6], 255);
        p.regs.vreg89 = p.regs.vreg89.wrapping_sub(2);
        let ult = (p.regs.vreg89 & 0x7F) as u16;
        let (x, y) = (p.regs.vreg87, p.regs.vreg88);
        p.gfx.rect(GameOverPart::P1, x, y, x + 64, y + 32, ult);
        p.gfx.rect(GameOverPart::P2, x + 64, y, x + 128, y + 32, ult);
        p.gfx.rect(GameOverPart::P3, x + 128, y, x + 192, y + 32, ult);
    }

    /// `KaleidoScope_LoadDungeonMap`: the floor's two room maps (`R_MAP_TEX_INDEX` and the next,
    /// `map_48x85_static`'s) into `mapSegment`, at 0 and `ALIGN16(MAP_48x85_TEX_SIZE)`.
    fn kaleido_scope_load_dungeon_map(&mut self) {
        let Some(a) = self.assets.clone() else { return };
        let src = &a.map.map_48x85_static;
        let index = self.map.r_map_tex_index.max(0) as usize;
        // Map_Init's GAME_STATE_ALLOC(0x1000).
        let mut seg = std::mem::take(&mut self.map.segment);
        seg.resize(MAP_SEGMENT_SECOND * 2, 0);
        for (k, at) in [(0, 0), (1, MAP_SEGMENT_SECOND)] {
            let from = (index + k) * MAP_48X85_TEX_SIZE;
            if let Some(t) = src.get(from..from + MAP_48X85_TEX_SIZE) {
                seg[at..at + MAP_48X85_TEX_SIZE].copy_from_slice(t);
            }
        }
        self.map.segment = seg;
        self.map.map_segment = Some(MapSegment::PauseMap { index: index as u32 });
    }

    /// `KaleidoScope_UpdateDungeonMap`: the floor's room maps loaded, the floor's palette
    /// (`Map_SetFloorPalettesData`), and on Link's floor the current room's texels moved to
    /// palette entry 14, the one that pulses (`KaleidoScope_DrawDungeonMap`).
    pub(super) fn kaleido_scope_update_dungeon_map(&mut self) {
        // (PRINTF("MAP DMA = %d", mapPaletteIndex); PLATFORM_N64's 64DD hook.)
        self.kaleido_scope_load_dungeon_map();
        self.map_set_floor_palettes_data(self.pause_ctx.dungeon_map_slot - 3);
        let on_links_floor = self.map.floor == self.pause_ctx.cursor_point[PAUSE_MAP as usize] - 3;
        if self.scene_id <= SCENE_TREASURE_BOX_SHOP && on_links_floor {
            let target = self.map.map_palette_index as i32;
            override_pal_index_ci4(self.map.segment.get_mut(..MAP_48X85_TEX_SIZE), MAP_48X85_TEX_SIZE as i32, target, 14);
        }
        if self.scene_id <= SCENE_TREASURE_BOX_SHOP && on_links_floor {
            let target = self.map.map_palette_index as i32;
            override_pal_index_ci4(self.map.segment.get_mut(MAP_SEGMENT_SECOND..MAP_SEGMENT_SECOND + MAP_48X85_TEX_SIZE), MAP_48X85_TEX_SIZE as i32, target, 14);
        }
    }

    /// `KaleidoScope_DrawEquipment` (`z_kaleido_equipment.c`): logged (GAME-05 5b-1).
    fn kaleido_scope_draw_equipment(&mut self) {
        self.pause_ctx.log_once(5, "pause menu: the equipment page's contents (KaleidoScope_DrawEquipment: the 4x4 grid, its cursor, A equipping, Link's image) aren't ported");
    }

    /// `KaleidoScope_DrawQuestStatus` (`z_kaleido_collect.c`): logged.
    fn kaleido_scope_draw_quest_status(&mut self) {
        self.pause_ctx.log_once(6, "pause menu: the quest status page's contents (KaleidoScope_DrawQuestStatus: medallions, songs, stones, the song playback) aren't ported");
    }

    /// The map page's contents: in a dungeon `KaleidoScope_DrawDungeonMap`, then
    /// (`Gfx_SetupDL_42Opa`, `G_CC_MODULATEIA_PRIM`) the cursor on the page looked at and, with
    /// the compass, `PauseMapMark_Draw`; elsewhere `KaleidoScope_DrawWorldMap` (logged).
    fn kaleido_scope_draw_map_contents(&mut self, looked_at: bool) {
        if self.pause_ctx.statics.in_dungeon_scene {
            self.kaleido_scope_draw_dungeon_map();
            self.pause_ctx.gfx.combine(Cc::ModulateIaPrim);
            if looked_at && self.pause_ctx.cursor_special_pos == 0 {
                self.kaleido_scope_draw_cursor(PAUSE_MAP);
            }
            if crate::map::check_dungeon_item(&self.save, crate::map::DUNGEON_COMPASS, self.save.map_index) {
                self.pause_map_mark_draw();
            }
        } else {
            self.pause_ctx.log_once(8, "pause menu: the world map's contents (KaleidoScope_DrawWorldMap) aren't ported");
        }
    }

    /// `KaleidoScope_DrawCursor`: the four corners (`sCursorTexs`) at `cursorVtx` in the cursor's
    /// colour set, the env colour cycling; only on the page looked at, while it's idle (or the
    /// quest page's song states).
    pub(super) fn kaleido_scope_draw_cursor(&mut self, page_index: u16) {
        let p = &mut self.pause_ctx;
        let idle = (p.main_state == PAUSE_MAIN_STATE_IDLE || p.main_state == PAUSE_MAIN_STATE_IDLE_CURSOR_ON_SONG) && p.state == PAUSE_STATE_MAIN;
        let quest = p.page_index == PAUSE_QUEST && (p.main_state < PAUSE_MAIN_STATE_3 || p.main_state == PAUSE_MAIN_STATE_SONG_PROMPT || p.main_state == PAUSE_MAIN_STATE_IDLE_CURSOR_ON_SONG);
        if idle || quest {
            if p.page_index == page_index {
                p.gfx.combine(Cc::PrimEnvTexel);
                let c = CURSOR_COLORS[(p.cursor_color_set >> 2) as usize];
                p.gfx.prim_color(c[0], c[1], c[2], 255);
                let e = p.statics.d_8082ab8c;
                p.gfx.env_color(e[0], e[1], e[2], 255);
                p.gfx.vertex_cursor(0, 16, 0);
                for i in 0..4 {
                    p.gfx.quad(KTex::Cursor(i as u8), i * 4);
                }
            }
            p.gfx.env_color(0, 0, 0, 255);
        }
    }

    /// `KaleidoScope_DrawUIOverlay`: the info panel, the L and R buttons (the one the cursor's on
    /// pulsing), the cursor on an arrow, and the panel's name or prompt.
    fn kaleido_scope_draw_ui_overlay(&mut self) {
        let p = &mut self.pause_ctx;
        let s = &mut p.statics;
        let target = LR_SELECTED_PRIM_COLORS[s.lr_selected_prim_state as usize];
        let timer = s.lr_selected_prim_timer;
        for k in 0..4 {
            approach(&mut s.lr_selected_prim[k], target[k], timer);
        }
        s.lr_selected_prim_timer -= 1;
        if s.lr_selected_prim_timer == 0 {
            s.lr_selected_prim = target;
            s.lr_selected_prim_timer = p.regs.zreg28[0];
            s.lr_selected_prim_state ^= 1;
        }

        let alpha = p.alpha as u8;
        let mut y = p.info_panel_offset_y - 76;
        let ui = &mut p.ui_overlay_vtx;
        for j in (0..UI_OVERLAY_QUAD_MAX * 4).step_by(4) {
            set_quad_ob(&mut ui[j..j + 4], -72, 0, y, y - 24);
            set_quad_tc_cn(&mut ui[j..j + 4], 72, 24, [200, 200, 200, alpha]);
        }
        // UI_OVERLAY_QUAD_INFO_BG_RIGHT.
        let x = ui[0].ob[0] + 72;
        set_quad_ob(&mut ui[4..8], x, x + 72, y, y - 24);
        let r = &p.regs;
        let idle = p.main_state == PAUSE_MAIN_STATE_IDLE;
        // UI_OVERLAY_QUAD_BUTTON_LEFT, _RIGHT: full size when the cursor's on them.
        for (j, sel, x) in [(8, PAUSE_CURSOR_PAGE_LEFT, r.button_left_x), (12, PAUSE_CURSOR_PAGE_RIGHT, r.button_right_x)] {
            let ly = r.button_left_right_y;
            if p.cursor_special_pos == sel && idle {
                set_quad_ob(&mut ui[j..j + 4], x, x + UI_OVERLAY_QUAD_BUTTON_LR_WIDTH, ly, ly - UI_OVERLAY_QUAD_BUTTON_LR_HEIGHT);
            } else {
                set_quad_ob(&mut ui[j..j + 4], x + 3, x + 3 + (UI_OVERLAY_QUAD_BUTTON_LR_WIDTH - 2 * 3), ly - 3, ly - 3 - (UI_OVERLAY_QUAD_BUTTON_LR_HEIGHT - 2 * 3));
            }
            ui[j + 1].tc[0] = UI_OVERLAY_QUAD_BUTTON_LR_WIDTH << 5;
            ui[j + 3].tc[0] = UI_OVERLAY_QUAD_BUTTON_LR_WIDTH << 5;
            ui[j + 2].tc[1] = UI_OVERLAY_QUAD_BUTTON_LR_HEIGHT << 5;
            ui[j + 3].tc[1] = UI_OVERLAY_QUAD_BUTTON_LR_HEIGHT << 5;
        }

        let g = &mut p.gfx;
        g.combine(Cc::ModulateIaPrim);
        g.matrix(Mat4::from_translation(Vec3::new(0.0, 0.0, -144.0)));
        g.prim_color(90, 100, 130, 255);
        g.vertex(&p.ui_overlay_vtx, 16, 0);
        // gInfoPanelBgDL: the two halves on quads 0 and 1.
        g.quad(KTex::InfoPanelBg(0), 0);
        g.quad(KTex::InfoPanelBg(1), 4);
        let lr = p.statics.lr_selected_prim;
        if p.cursor_special_pos == PAUSE_CURSOR_PAGE_LEFT && idle {
            g.prim_color(lr[0], lr[1], lr[2], lr[3]);
        }
        // gLButtonIconDL.
        g.quad(KTex::LButton, 8);
        g.prim_color(180, 210, 255, 220);
        if p.cursor_special_pos == PAUSE_CURSOR_PAGE_RIGHT && idle {
            g.prim_color(lr[0], lr[1], lr[2], lr[3]);
        }
        // gRButtonIconDL.
        g.quad(KTex::RButton, 12);

        if p.cursor_special_pos != 0 {
            let j = ((p.cursor_special_pos - PAUSE_CURSOR_PAGE_LEFT) as usize + 2) * 4;
            p.cursor_vtx[0].ob[0] = p.ui_overlay_vtx[j].ob[0];
            p.cursor_vtx[0].ob[1] = p.ui_overlay_vtx[j].ob[1];
            let page = p.page_index;
            self.kaleido_scope_draw_cursor(page);
        }

        let p = &mut self.pause_ctx;
        // UI_OVERLAY_QUAD_INFO_ICON.
        y = p.info_panel_offset_y - 80;
        let ui = &mut p.ui_overlay_vtx;
        ui[16].ob[1] = y;
        ui[17].ob[1] = y;
        ui[18].ob[1] = y - 16;
        ui[19].ob[1] = y - 16;
        ui[18].tc[1] = 16 << 5;
        ui[19].tc[1] = 16 << 5;
        p.gfx.combine(Cc::PrimEnvTexel);
        p.gfx.env_color(20, 30, 40, 0);

        let ms = p.main_state;
        let shows_name = ms == PAUSE_MAIN_STATE_IDLE
            || ms == PAUSE_MAIN_STATE_SONG_PLAYBACK
            || (PAUSE_MAIN_STATE_SONG_PROMPT_INIT..=PAUSE_MAIN_STATE_EQUIP_CHANGED).contains(&ms)
            || ms == PAUSE_MAIN_STATE_IDLE_CURSOR_ON_SONG;
        if p.state == PAUSE_STATE_MAIN && p.named_item != PAUSE_ITEM_NONE && p.name_display_timer < p.regs.wreg89 as u16 && shows_name && p.cursor_special_pos == 0 {
            // UI_OVERLAY_QUAD_INFO_ICON: the name (ITEM_NAME_TEX_WIDTH 128).
            set_quad_ob(&mut ui[16..20], -63, -63 + 128, y, y - 16);
            ui[17].tc[0] = 128 << 5;
            ui[19].tc[0] = 128 << 5;
            p.gfx.vertex(&p.ui_overlay_vtx[16..], 4, 0);
            if p.name_color_set == 1 {
                p.gfx.prim_color(70, 70, 70, 255);
            } else {
                p.gfx.prim_color(255, 255, 255, 255);
            }
            // (The map name on the world map: map_name_static's point names, the world map's.)
            if p.page_index == PAUSE_MAP && !p.statics.in_dungeon_scene {
                p.log_once(9, "pause menu: the world map's point names (map_name_static) aren't drawn");
            } else {
                p.gfx.quad(KTex::ItemName(p.named_item), 0);
            }
            // (DEBUG_FEATURES: YREG(7)'s Gold Skulltula flags. The world map's "all Gold
            // Skulltulas" icon: the world map's.)
        } else if ms < PAUSE_MAIN_STATE_3 || ms == PAUSE_MAIN_STATE_EQUIP_CHANGED || ms == PAUSE_MAIN_STATE_IDLE_CURSOR_ON_SONG {
            // UI_OVERLAY_QUAD_INFO_TEXT.
            ui[20].ob[1] = y;
            ui[21].ob[1] = y;
            ui[22].ob[1] = y - 16;
            ui[23].ob[1] = y - 16;
            ui[22].tc[1] = 16 << 5;
            ui[23].tc[1] = 16 << 5;
            let r = p.regs.clone();
            // The icon (`w` wide at `x`) and the text (`tw` wide, `dx` from the icon).
            let icon_and_text = |p: &mut PauseContext, x: i16, w: i16, dx: i16, tw: i16| {
                let ui = &mut p.ui_overlay_vtx;
                ui[16].ob[0] = x;
                ui[18].ob[0] = x;
                ui[17].ob[0] = x + w;
                ui[19].ob[0] = x + w;
                ui[20].ob[0] = x + dx;
                ui[22].ob[0] = x + dx;
                ui[21].ob[0] = x + dx + tw;
                ui[23].ob[0] = x + dx + tw;
                ui[17].tc[0] = w << 5;
                ui[19].tc[0] = w << 5;
                ui[21].tc[0] = tw << 5;
                ui[23].tc[0] = tw << 5;
            };
            if p.state == PAUSE_STATE_SAVE_PROMPT {
                icon_and_text(p, r.info_panel_icon_save_prompt_x, A_BTN_SYMBOL_WIDTH, r.info_panel_text_x, TO_DECIDE_WIDTH);
                p.gfx.vertex(&p.ui_overlay_vtx[16..], 8, 0);
                // gAButtonIconDL: prim (0, 255, 100).
                p.gfx.prim_color(0, 255, 100, 255);
                p.gfx.quad(KTex::ABtnSymbol, 0);
                p.gfx.prim_color(255, 255, 255, 255);
                p.gfx.quad(label("gPauseToDecideENGTex"), 4);
            } else if p.cursor_special_pos != 0 {
                if p.state == PAUSE_STATE_MAIN && ms == PAUSE_MAIN_STATE_IDLE {
                    let ui = &mut p.ui_overlay_vtx;
                    ui[16].ob[0] = -63;
                    ui[18].ob[0] = -63;
                    ui[17].ob[0] = -63 + 128;
                    ui[19].ob[0] = -63 + 128;
                    ui[17].tc[0] = 128 << 5;
                    ui[19].tc[0] = 128 << 5;
                    p.gfx.vertex(&p.ui_overlay_vtx[16..], 8, 0);
                    p.gfx.prim_color(255, 200, 0, 255);
                    let page = p.page_index as usize;
                    let t = if p.cursor_special_pos == PAUSE_CURSOR_PAGE_LEFT { SCROLL_LEFT_LABELS[page] } else { SCROLL_RIGHT_LABELS[page] };
                    p.gfx.quad(label(t), 0);
                }
            } else if p.page_index == PAUSE_ITEM {
                icon_and_text(p, r.info_panel_icon_c_item_x, C_BTN_SYMBOLS_WIDTH, r.info_panel_text_c_item_x, TO_EQUIP_WIDTH);
                p.gfx.vertex(&p.ui_overlay_vtx[16..], 8, 0);
                // gCButtonIconsDL: prim (255, 150, 0).
                p.gfx.prim_color(255, 150, 0, 255);
                p.gfx.quad(KTex::CBtnSymbols, 0);
                p.gfx.prim_color(255, 255, 255, 255);
                p.gfx.quad(label("gPauseToEquipENGTex"), 4);
            } else if p.page_index == PAUSE_MAP && p.statics.in_dungeon_scene {
            } else if p.page_index == PAUSE_QUEST && (6..=0x11).contains(&p.cursor_slot[PAUSE_QUEST as usize]) {
                if p.named_item != PAUSE_ITEM_NONE {
                    // (OOT_PAL, German: the text 99 left of the icon. English here.)
                    icon_and_text(p, r.info_panel_icon_play_song_x, 24, r.info_panel_text_x, TO_PLAY_MELODY_WIDTH);
                    let ui = &mut p.ui_overlay_vtx;
                    ui[17].tc[0] = A_BTN_SYMBOL_WIDTH << 5;
                    ui[19].tc[0] = A_BTN_SYMBOL_WIDTH << 5;
                    p.gfx.vertex(&p.ui_overlay_vtx[16..], 8, 0);
                    p.gfx.prim_color(0, 255, 100, 255);
                    p.gfx.quad(KTex::ABtnSymbol, 0);
                    p.gfx.prim_color(255, 255, 255, 255);
                    p.gfx.quad(label("gPauseToPlayMelodyENGTex"), 4);
                }
            } else if p.page_index == PAUSE_EQUIP {
                icon_and_text(p, r.info_panel_icon_equip_x, 24, r.info_panel_text_x, TO_EQUIP_WIDTH);
                let ui = &mut p.ui_overlay_vtx;
                ui[17].tc[0] = A_BTN_SYMBOL_WIDTH << 5;
                ui[19].tc[0] = A_BTN_SYMBOL_WIDTH << 5;
                p.gfx.vertex(&p.ui_overlay_vtx[16..], 8, 0);
                p.gfx.prim_color(0, 255, 100, 255);
                p.gfx.quad(KTex::ABtnSymbol, 0);
                p.gfx.prim_color(255, 255, 255, 255);
                p.gfx.quad(label("gPauseToEquipENGTex"), 4);
            }
        }
    }

    /// `KaleidoScope_SetVertices`: every page's vertices for the frame (the pages turn about
    /// their lower edge while opening and closing: `pagesYOrigin1` 80).
    fn kaleido_scope_set_vertices(&mut self) {
        // gSaveContext.worldMapArea: the scene's misc settings' area, which the pack doesn't carry
        // (only the world map's trade marker reads it, and the world map isn't drawn): Play_Init's
        // WORLD_MAP_AREA_HYRULE_FIELD.
        let world_map_area = 0;
        let cbutton_slots = self.save.equips.c_button_slots;
        let cur_equip: [u16; 4] = std::array::from_fn(|j| self.save.cur_equip_value(j));
        let p = &mut self.pause_ctx;
        p.pages_y_origin1 = 0;
        let lower = p.state == PAUSE_STATE_OPENING_1
            || p.state >= PAUSE_STATE_CLOSING
            || (p.state == PAUSE_STATE_SAVE_PROMPT && (p.save_prompt_state == PAUSE_SAVE_PROMPT_STATE_CLOSING || p.save_prompt_state == PAUSE_SAVE_PROMPT_STATE_CLOSING_AFTER_SAVED))
            || (PAUSE_STATE_GAME_OVER_START..=PAUSE_STATE_GAME_OVER_SHOW_WINDOW).contains(&p.state);
        if lower {
            p.pages_y_origin1 = PAUSE_PAGES_Y_ORIGIN_1_LOWER;
        }
        let y1 = p.pages_y_origin1;
        let alpha = p.alpha as u8;

        p.item_page_vtx = vec![Vtx::default(); PAGE_BG_QUADS * 4];
        set_page_vertices(p, PageVtx::Item, VTX_PAGE_ITEM, 0, world_map_area);
        p.equip_page_vtx = vec![Vtx::default(); PAGE_BG_QUADS * 4];
        set_page_vertices(p, PageVtx::Equip, VTX_PAGE_EQUIP, 0, world_map_area);
        if !p.statics.in_dungeon_scene {
            // The world map's page: its quads, then the image's fragments.
            p.map_page_vtx = vec![Vtx::default(); (PAGE_BG_QUADS + VTX_PAGE_MAP_WORLD_QUADS + WORLD_MAP_IMAGE_FRAG_NUM) * 4];
            let mut j = set_page_vertices(p, PageVtx::Map, VTX_PAGE_MAP_WORLD, VTX_PAGE_MAP_WORLD_QUADS, world_map_area);
            let mut y = 58i16;
            let v = &mut p.map_page_vtx;
            for _ in 0..WORLD_MAP_IMAGE_FRAG_NUM {
                set_quad_ob(&mut v[j..j + 4], -(WORLD_MAP_IMAGE_WIDTH / 2), -(WORLD_MAP_IMAGE_WIDTH / 2) + WORLD_MAP_IMAGE_WIDTH, y + y1, y + y1 - WORLD_MAP_IMAGE_FRAG_HEIGHT);
                set_quad_tc_cn(&mut v[j..j + 4], WORLD_MAP_IMAGE_WIDTH, WORLD_MAP_IMAGE_FRAG_HEIGHT, [alpha; 4]);
                j += 4;
                y -= WORLD_MAP_IMAGE_FRAG_HEIGHT;
            }
            let last_y = v[j - 4].ob[1] - (WORLD_MAP_IMAGE_HEIGHT % WORLD_MAP_IMAGE_FRAG_HEIGHT);
            v[j - 2].ob[1] = last_y;
            v[j - 1].ob[1] = last_y;
            v[j - 2].tc[1] = (WORLD_MAP_IMAGE_HEIGHT % WORLD_MAP_IMAGE_FRAG_HEIGHT) << 5;
            v[j - 1].tc[1] = (WORLD_MAP_IMAGE_HEIGHT % WORLD_MAP_IMAGE_FRAG_HEIGHT) << 5;
        } else {
            p.map_page_vtx = vec![Vtx::default(); (PAGE_BG_QUADS + VTX_PAGE_MAP_DUNGEON_QUADS) * 4];
            set_page_vertices(p, PageVtx::Map, VTX_PAGE_MAP_DUNGEON, VTX_PAGE_MAP_DUNGEON_QUADS, world_map_area);
        }
        p.quest_page_vtx = vec![Vtx::default(); PAGE_BG_QUADS * 4];
        set_page_vertices(p, PageVtx::Quest, VTX_PAGE_QUEST, 0, world_map_area);

        // cursorVtx: all at 0, white; the corners' and PAUSE_CURSOR_QUAD_4's texture sizes.
        let c = &mut p.cursor_vtx;
        *c = vec![Vtx { ob: [0; 3], tc: [0; 2], cn: [255; 4] }; PAUSE_CURSOR_QUAD_MAX * 4];
        for q in 0..4 {
            let k = q * 4;
            c[k + 1].tc[0] = 16 << 5;
            c[k + 2].tc[1] = 16 << 5;
            c[k + 3].tc = [16 << 5, 16 << 5];
        }
        c[17].tc[0] = 32 << 5;
        c[18].tc[1] = 32 << 5;
        c[19].tc = [32 << 5, 32 << 5];

        // itemVtx: the grid (6 by 4 cells of 32, quads of 28), the C buttons' outlines, the ammo.
        let v = &mut p.item_vtx;
        *v = vec![Vtx::default(); ITEM_QUAD_MAX * 4];
        let mut i = 0;
        let mut y = (4 * 32) / 2 - 6;
        for _ in 0..4 {
            let mut x = -(6 * 32) / 2;
            for _ in 0..6 {
                let (x0, y0) = (x + 2, y + y1 - 2);
                set_quad_ob(&mut v[i..i + 4], x0, x0 + 28, y0, y0 - 28);
                set_quad_tc_cn(&mut v[i..i + 4], 32, 32, [255; 4]);
                i += 4;
                x += 32;
            }
            y -= 32;
        }
        for j in 1..4 {
            if cbutton_slots[j - 1] != ITEM_NONE {
                let k = cbutton_slots[j - 1] as usize * 4;
                let (x0, y0) = (v[k].ob[0] - 2, v[k].ob[1] + 2);
                set_quad_ob(&mut v[i..i + 4], x0, x0 + 32, y0, y0 - 32);
                set_quad_tc_cn(&mut v[i..i + 4], 32, 32, [255, 255, 255, alpha]);
            } else {
                // No item on the C button: the quad out of view (its texture coordinates and
                // colours as GRAPH_ALLOC left them; here zero).
                set_quad_ob(&mut v[i..i + 4], -300, -300 + 32, 300, 300 - 32);
            }
            i += 4;
        }
        let mut i = 27 * 4;
        for &k in &ITEM_VTX_QUADS_WITH_AMMO {
            // The tens, 22 below the slot's top; the ones 6 to its right.
            let (x0, y0) = (v[k].ob[0], v[k].ob[1] - 22);
            set_quad_ob(&mut v[i..i + 4], x0, x0 + 8, y0, y0 - 8);
            set_quad_ob(&mut v[i + 4..i + 8], x0 + 6, x0 + 6 + 8, y0, y0 - 8);
            for _ in 0..2 {
                set_quad_tc_cn(&mut v[i..i + 4], 8, 8, [255, 255, 255, alpha]);
                i += 4;
            }
        }

        // equipVtx: the 4x4 grid, the worn pieces' outlines, Link's image's strips.
        let v = &mut p.equip_vtx;
        *v = vec![Vtx::default(); EQUIP_QUAD_MAX * 4];
        let mut k = 0;
        let mut y = (4 * 32) / 2 - 6;
        for _ in 0..4 {
            for &cx in &EQUIP_COLUMNS_X {
                let (x0, y0) = (cx + 2, y + y1 - 2);
                set_quad_ob(&mut v[k..k + 4], x0, x0 + 28, y0, y0 - 28);
                set_quad_tc_cn(&mut v[k..k + 4], 32, 32, [255, 255, 255, alpha]);
                k += 4;
            }
            y -= 32;
        }
        for (j, &value) in cur_equip.iter().enumerate() {
            if value != 0 {
                let i = (value as usize + EQUIP_QUADS_FIRST_BY_EQUIP_TYPE[j] - 1) * 4;
                let (x0, y0) = (v[i].ob[0] - 2, v[i].ob[1] + 2);
                set_quad_ob(&mut v[k..k + 4], x0, x0 + 32, y0, y0 - 32);
                set_quad_tc_cn(&mut v[k..k + 4], 32, 32, [255, 255, 255, alpha]);
            }
            k += 4;
        }
        let mut x = PAUSE_EQUIP_PLAYER_HEIGHT;
        let mut y = 50i16;
        loop {
            set_quad_ob(&mut v[k..k + 4], -64, -64 + PAUSE_EQUIP_PLAYER_WIDTH, y + y1, y + y1 - PAUSE_EQUIP_PLAYER_FRAG_HEIGHT);
            set_quad_tc_cn(&mut v[k..k + 4], PAUSE_EQUIP_PLAYER_WIDTH, PAUSE_EQUIP_PLAYER_FRAG_HEIGHT, [255, 255, 255, alpha]);
            x -= PAUSE_EQUIP_PLAYER_FRAG_HEIGHT;
            if x < 0 {
                let ly = v[k].ob[1] - (PAUSE_EQUIP_PLAYER_HEIGHT % PAUSE_EQUIP_PLAYER_FRAG_HEIGHT);
                v[k + 2].ob[1] = ly;
                v[k + 3].ob[1] = ly;
                v[k + 2].tc[1] = (PAUSE_EQUIP_PLAYER_HEIGHT % PAUSE_EQUIP_PLAYER_FRAG_HEIGHT) << 5;
                v[k + 3].tc[1] = (PAUSE_EQUIP_PLAYER_HEIGHT % PAUSE_EQUIP_PLAYER_FRAG_HEIGHT) << 5;
                break;
            }
            y -= PAUSE_EQUIP_PLAYER_FRAG_HEIGHT;
            k += 4;
        }

        // questVtx.
        let v = &mut p.quest_vtx;
        *v = vec![Vtx::default(); QUEST_QUAD_MAX * 4];
        for j in 0..QUEST_QUAD_MAX {
            let k = j * 4;
            let mut quad_width = QUEST_QUADS_SIZE[j];
            let size = QUEST_QUADS_SIZE[j];
            if j < QUEST_SONG_MINUET || j >= QUEST_QUAD_SKULL_TOKENS_DIGIT1_SHADOW {
                let (x0, y0) = (QUEST_QUADS_X[j], QUEST_QUADS_Y[j] + y1);
                set_quad_ob(&mut v[k..k + 4], x0, x0 + size, y0, y0 - size);
                if j >= QUEST_QUAD_SKULL_TOKENS_DIGIT1_SHADOW {
                    set_quad_ob(&mut v[k..k + 4], x0, x0 + 8, y0 - 6, y0 - 6 - 16);
                    quad_width = 8;
                }
            } else {
                if (QUEST_SONG_MINUET..QUEST_KOKIRI_EMERALD).contains(&j) {
                    quad_width = 16;
                }
                let (x0, y0) = (QUEST_QUADS_X[j] + 2, QUEST_QUADS_Y[j] + y1 - 2);
                set_quad_ob(&mut v[k..k + 4], x0, x0 + (quad_width - 4), y0, y0 - (size - 4));
            }
            set_quad_tc_cn(&mut v[k..k + 4], quad_width, size, [255, 255, 255, alpha]);
        }

        // uiOverlayVtx: GRAPH_ALLOC'd, set by KaleidoScope_DrawUIOverlay.
        p.ui_overlay_vtx = vec![Vtx::default(); UI_OVERLAY_QUAD_MAX * 4];
        p.prompt_page_vtx = vec![Vtx::default(); (PAGE_BG_QUADS + VTX_PAGE_PROMPT_QUADS) * 4];
        set_page_vertices(p, PageVtx::Prompt, VTX_PAGE_PROMPT, VTX_PAGE_PROMPT_QUADS, world_map_area);
    }

    /// `KaleidoScope_UpdatePrompt` (`z_kaleido_prompt.c`): on a prompt, the stick right to No,
    /// left to Yes; the prompt cursor's alpha pulses (`R_KALEIDO_PROMPT_CURSOR_ALPHA`).
    pub(super) fn kaleido_scope_update_prompt(&mut self) {
        const ALPHA_VALS: [i16; 2] = [100, 255];
        let stick_adj_x = self.input.rel.stick_x;
        let p = &mut self.pause_ctx;
        if (p.state == PAUSE_STATE_SAVE_PROMPT && p.save_prompt_state == PAUSE_SAVE_PROMPT_STATE_WAIT_CHOICE)
            || p.state == PAUSE_STATE_GAME_OVER_SAVE_PROMPT
            || p.state == PAUSE_STATE_GAME_OVER_CONTINUE_PROMPT
        {
            if p.prompt_choice == 0 && stick_adj_x >= 30 {
                self.audio.play_sfx_centered(NA_SE_SY_CURSOR);
                p.prompt_choice = 4;
            } else if p.prompt_choice != 0 && stick_adj_x <= -30 {
                self.audio.play_sfx_centered(NA_SE_SY_CURSOR);
                p.prompt_choice = 0;
            }
            let r = &mut p.regs;
            let target = ALPHA_VALS[r.prompt_cursor_alpha_state as usize];
            approach(&mut r.prompt_cursor_alpha, target, r.prompt_cursor_alpha_timer);
            r.prompt_cursor_alpha_timer -= 1;
            if r.prompt_cursor_alpha_timer == 0 {
                r.prompt_cursor_alpha = target;
                r.prompt_cursor_alpha_timer = r.prompt_cursor_alpha_timer_base + r.prompt_cursor_alpha_state;
                r.prompt_cursor_alpha_state ^= 1;
            }
        }
    }

    /// `KaleidoScope_Update`'s game over states (`PAUSE_STATE_GAME_OVER_INIT` to `_FINISH`,
    /// docs/adr/0032): the message's colours, the window's turn, the prompts. The pages'
    /// pitches, panels and buttons that turn with the window are drawn in milestone 5b-2.
    fn kaleido_scope_update_game_over(&mut self) {
        let press_a = self.input.press.held(BTN_A);
        let press_start = self.input.press.held(BTN_START);
        match self.pause_ctx.state {
            PAUSE_STATE_GAME_OVER_INIT => {
                let floor_slot = self.map.floor + 3;
                let p = &mut self.pause_ctx;
                p.cursor_slot[PAUSE_MAP as usize] = floor_slot as u16;
                p.cursor_point[PAUSE_MAP as usize] = floor_slot;
                p.dungeon_map_slot = floor_slot;
                p.regs.button_left_x = -175;
                p.regs.button_right_x = 155;
                p.prompt_pitch = -434.0;
                crate::interface::change_alpha(&mut self.save, HUD_VISIBILITY_NOTHING);
                // (The icon files' DMA: icon_item_gameover_static.)
                p.statics.d_8082ab8c = [255, 130, 0, 0, 30, 0, 0];
                p.statics.d_8082b260 = 30;
                p.regs.vreg88 = 98;
                p.prompt_choice = 0;
                p.state += 1;
            }
            PAUSE_STATE_GAME_OVER_SHOW_MESSAGE => {
                // The message's colours step to (30, 0, 0, 255) and (255, 130, 0) over 30 frames.
                let s = &mut self.pause_ctx.statics;
                let n = s.d_8082b260;
                let targets = [30i16, 0, 0, 255, 255, 130, 0];
                for k in 0..7 {
                    approach(&mut s.d_8082ab8c[k], targets[k], n);
                }
                s.d_8082b260 -= 1;
                if s.d_8082b260 == 0 {
                    s.d_8082ab8c = targets;
                    s.d_8082b260 = 40;
                    self.pause_ctx.state += 1;
                }
            }
            PAUSE_STATE_GAME_OVER_WINDOW_DELAY => {
                let p = &mut self.pause_ctx;
                p.statics.d_8082b260 -= 1;
                if p.statics.d_8082b260 == 0 {
                    p.state = PAUSE_STATE_GAME_OVER_SHOW_WINDOW;
                }
            }
            PAUSE_STATE_GAME_OVER_SHOW_WINDOW => {
                // The window turns in, the pages with it, the message rising 3 a frame.
                let p = &mut self.pause_ctx;
                let d = p.regs.ui_anims_duration;
                p.prompt_pitch -= 160.0 / d as f32;
                let pitch = p.prompt_pitch;
                (p.item_page_pitch, p.equip_page_pitch, p.map_page_pitch, p.quest_page_pitch) = (pitch, pitch, pitch, pitch);
                p.info_panel_offset_y += 40 / d;
                self.interface_ctx.start_alpha += 255 / d;
                let r = &mut p.regs;
                r.vreg88 -= 3;
                r.button_left_x += r.button_left_move_offset_x / d;
                r.button_right_x += r.button_right_move_offset_x / d;
                r.xreg5 += 150 / d;
                p.alpha = p.alpha.wrapping_add((255 / (d + r.wreg4)) as u16);
                if p.prompt_pitch < -628.0 {
                    p.prompt_pitch = -628.0;
                    self.interface_ctx.start_alpha = 255;
                    p.regs.vreg88 = 66;
                    p.regs.pages_y_origin_2 = 0;
                    p.alpha = 255;
                    p.state = PAUSE_STATE_GAME_OVER_SAVE_PROMPT;
                    self.save.deaths += 1;
                    if self.save.deaths > 999 {
                        self.save.deaths = 999;
                    }
                    log::info!("game over: \"Save?\" (the pause menu's stand-in: A, the stick for No)");
                }
            }
            PAUSE_STATE_GAME_OVER_SAVE_PROMPT => {
                if press_a {
                    if self.pause_ctx.prompt_choice != 0 {
                        self.pause_ctx.prompt_choice = 0;
                        self.audio.play_sfx_centered(NA_SE_SY_DECIDE);
                        self.pause_ctx.state = PAUSE_STATE_GAME_OVER_CONTINUE_PROMPT;
                        self.game_over_ctx.state += 1;
                        log::info!("game over: \"Continue?\" (A or Start)");
                    } else {
                        self.audio.play_sfx_centered(NA_SE_SY_PIECE_OF_HEART);
                        self.pause_ctx.prompt_choice = 0;
                        self.save_scene_flags();
                        self.save.saved_scene_id = self.scene_id;
                        log::info!("game over: saved (Sram_WriteSave, the write to SRAM, isn't ported)");
                        self.pause_ctx.state = PAUSE_STATE_GAME_OVER_SAVED;
                        // (sDelayTimer 3 on the GameCube versions.)
                        self.pause_ctx.statics.delay_timer = 3;
                    }
                }
            }
            PAUSE_STATE_GAME_OVER_SAVED => {
                self.pause_ctx.statics.delay_timer -= 1;
                if self.pause_ctx.statics.delay_timer == 0 {
                    self.pause_ctx.state = PAUSE_STATE_GAME_OVER_CONTINUE_PROMPT;
                    self.game_over_ctx.state += 1;
                    log::info!("game over: \"Continue?\" (A or Start)");
                } else if self.pause_ctx.statics.delay_timer <= 80 && (press_a || press_start) {
                    self.pause_ctx.state = PAUSE_STATE_GAME_OVER_CONTINUE_PROMPT;
                    self.game_over_ctx.state += 1;
                    self.audio.func_800f64e0(0);
                }
            }
            PAUSE_STATE_GAME_OVER_CONTINUE_PROMPT => {
                if press_a || press_start {
                    if self.pause_ctx.prompt_choice == 0 {
                        self.audio.play_sfx_centered(NA_SE_SY_PIECE_OF_HEART);
                        self.save_scene_flags();
                        self.continue_entrance();
                    } else {
                        self.audio.play_sfx_centered(NA_SE_SY_DECIDE);
                    }
                    self.pause_ctx.state = PAUSE_STATE_GAME_OVER_FINISH;
                }
            }
            PAUSE_STATE_GAME_OVER_FINISH => {
                if self.interface_ctx.unk_244 != 255 {
                    self.interface_ctx.unk_244 += 10;
                    if self.interface_ctx.unk_244 >= 255 {
                        self.interface_ctx.unk_244 = 255;
                        self.pause_ctx.state = PAUSE_STATE_OFF;
                        self.r_update_rate = 3;
                        self.pause_ctx.bg_prerender_state = PAUSE_BG_PRERENDER_OFF;
                        // (Object_ReloadAll and func_800418D0: the menu borrowed no object or
                        // collision space.)
                        if self.pause_ctx.prompt_choice != 0 {
                            // play->state.running = false into TitleSetup_Init: the title screen
                            // isn't ported.
                            log::info!("game over: \"No\" goes back to the title screen, which isn't ported: continuing");
                        }
                        self.trigger_respawn();
                        self.save.respawn_flag = -2;
                        self.save.next_transition_type = crate::transition::TRANS_TYPE_FADE_BLACK;
                        self.save.health = 0x30;
                        // SEQCMD_RESET_AUDIO_HEAP(0, 10).
                        self.audio.queue_seq_cmd((0xF << 28) | 10);
                        self.save.health_accumulator = 0;
                        // (The magic meter's refill: magic isn't ported.)
                    }
                }
            }
            _ => {}
        }
    }

    /// The "Continue" choice's entrance: a boss room's is its dungeon's entrance (the
    /// dungeons' own entrances stay).
    fn continue_entrance(&mut self) {
        const BOSS_TO_DUNGEON: [(&str, &str); 9] = [
            ("ENTR_DEKU_TREE_BOSS_0", "ENTR_DEKU_TREE_0"),
            ("ENTR_DODONGOS_CAVERN_BOSS_0", "ENTR_DODONGOS_CAVERN_0"),
            ("ENTR_JABU_JABU_BOSS_0", "ENTR_JABU_JABU_0"),
            ("ENTR_FOREST_TEMPLE_BOSS_0", "ENTR_FOREST_TEMPLE_0"),
            ("ENTR_FIRE_TEMPLE_BOSS_0", "ENTR_FIRE_TEMPLE_0"),
            ("ENTR_WATER_TEMPLE_BOSS_0", "ENTR_WATER_TEMPLE_0"),
            ("ENTR_SPIRIT_TEMPLE_BOSS_0", "ENTR_SPIRIT_TEMPLE_0"),
            ("ENTR_SHADOW_TEMPLE_BOSS_0", "ENTR_SHADOW_TEMPLE_0"),
            ("ENTR_GANONDORF_BOSS_0", "ENTR_GANONS_TOWER_0"),
        ];
        let Some(a) = self.assets.clone() else { return };
        for (boss, dungeon) in BOSS_TO_DUNGEON {
            if a.scenes.entrance_index(boss) == Some(self.save.entrance_index)
                && let Some(e) = a.scenes.entrance_index(dungeon)
            {
                self.save.entrance_index = e;
                return;
            }
        }
    }
}

/// `KaleidoScope_OverridePalIndexCI4`: in `size` bytes of a CI4 texture, every texel of
/// `targetIndex` (its low four bits) becomes `newIndex`; nothing when they're the same, the size
/// 0 or no texture.
pub fn override_pal_index_ci4(texture: Option<&mut [u8]>, size: i32, target_index: i32, new_index: i32) {
    let target_index = target_index & 0xF;
    let new_index = new_index & 0xF;
    let Some(texture) = texture.filter(|_| size != 0 && target_index != new_index) else { return };
    for b in texture.iter_mut().take(size.max(0) as usize) {
        let mut index1 = (*b as i32 >> 4) & 0xF;
        let mut index2 = *b as i32 & 0xF;
        if index1 == target_index {
            index1 = new_index;
        }
        if index2 == target_index {
            index2 = new_index;
        }
        *b = ((index1 << 4) | index2) as u8;
    }
}

/// `KaleidoScope_MoveCursorToSpecialPos`: the cursor onto a page arrow.
pub(super) fn move_cursor_to_special_pos(p: &mut PauseContext, audio: &mut crate::audio::GameAudio, special_pos: i16) {
    p.cursor_special_pos = special_pos;
    p.page_switch_input_timer = 0;
    audio.play_sfx_centered(NA_SE_SY_DECIDE);
}

/// Which page's vertices `set_page_vertices` fills.
#[derive(Clone, Copy)]
enum PageVtx {
    Item,
    Equip,
    Map,
    Quest,
    Prompt,
}

/// `KaleidoScope_SetPageVertices`: a page's 15 background quads (the column colours, the
/// alpha), then its extra quads (and the world map's trade quest marker, bobbing). Returns the
/// vertices filled.
fn set_page_vertices(p: &mut PauseContext, page: PageVtx, vtx_page: usize, num_quads: usize, world_map_area: usize) -> usize {
    let y1 = p.pages_y_origin1;
    let alpha = p.alpha as u8;
    let game_over = p.is_game_over();
    let mut vtx = std::mem::take(match page {
        PageVtx::Item => &mut p.item_page_vtx,
        PageVtx::Equip => &mut p.equip_page_vtx,
        PageVtx::Map => &mut p.map_page_vtx,
        PageVtx::Quest => &mut p.quest_page_vtx,
        PageVtx::Prompt => &mut p.prompt_page_vtx,
    });
    let v = &mut vtx;
    let mut buf_i = 0;
    let mut x = -(PAGE_BG_COLS as i16 * PAGE_BG_QUAD_WIDTH) / 2 - PAGE_BG_QUAD_WIDTH;
    for j in 0..PAGE_BG_COLS {
        x += PAGE_BG_QUAD_WIDTH;
        let mut y = (PAGE_BG_ROWS as i16 * PAGE_BG_QUAD_HEIGHT) / 2;
        for _ in 0..PAGE_BG_ROWS {
            set_quad_ob(&mut v[buf_i..buf_i + 4], x, x + PAGE_BG_QUAD_WIDTH, y + y1, y + y1 - PAGE_BG_QUAD_HEIGHT);
            set_quad_tc_cn(&mut v[buf_i..buf_i + 4], 80, 32, [0; 4]);
            let left = [PAGE_BG_COLOR_RED[vtx_page][j], PAGE_BG_COLOR_GREEN[vtx_page][j], PAGE_BG_COLOR_BLUE[vtx_page][j], alpha];
            let right = [PAGE_BG_COLOR_RED[vtx_page][j + 1], PAGE_BG_COLOR_GREEN[vtx_page][j + 1], PAGE_BG_COLOR_BLUE[vtx_page][j + 1], alpha];
            v[buf_i].cn = left;
            v[buf_i + 2].cn = left;
            v[buf_i + 1].cn = right;
            v[buf_i + 3].cn = right;
            buf_i += 4;
            y -= PAGE_BG_QUAD_HEIGHT;
        }
    }
    let after_sections = buf_i;
    if num_quads != 0 {
        for j in 0..num_quads {
            let (qx, qw, qy, qh) = match vtx_page {
                VTX_PAGE_MAP_DUNGEON => (MAP_DUNGEON_QUADS[j][0], MAP_DUNGEON_QUADS[j][1], MAP_DUNGEON_QUADS[j][2], MAP_DUNGEON_QUADS[j][3]),
                VTX_PAGE_MAP_WORLD => (MAP_WORLD_QUADS_X[j], MAP_WORLD_QUADS_WIDTH[j], MAP_WORLD_QUADS_Y[j], MAP_WORLD_QUADS_HEIGHT[j]),
                _ => (PROMPT_QUADS[j][0], PROMPT_QUADS[j][1], PROMPT_QUADS[j][2], PROMPT_QUADS[j][3]),
            };
            // In a game over the quads' y is YREG(60 + j) (14, -2, -2, -18, -18 for the prompt's).
            const YREG_60: [i16; 17] = [14, -2, -2, -18, -18, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
            let y0 = if !game_over { qy + y1 } else { YREG_60.get(j).copied().unwrap_or(0) + y1 };
            set_quad_ob(&mut v[buf_i..buf_i + 4], qx, qx + qw, y0, y0 - qh);
            set_quad_tc_cn(&mut v[buf_i..buf_i + 4], qw, qh, [255, 255, 255, alpha]);
            buf_i += 4;
        }
        if vtx_page == VTX_PAGE_MAP_WORLD {
            // WORLD_MAP_QUAD_TRADE_QUEST_MARKER: first at the current area's box.
            // @bug (game): WORLD_MAP_AREA_GANONS_CASTLE or _MAX read past the area arrays (the
            // vertices are overwritten below or not drawn): here the last entry.
            let a = MAP_WORLD_AREA[world_map_area.min(MAP_WORLD_AREA.len() - 1)];
            let i = buf_i - (VTX_PAGE_MAP_WORLD_QUADS - WORLD_MAP_QUAD_TRADE_QUEST_MARKER) * 4;
            set_quad_ob(&mut v[i..i + 4], a[0], a[0] + a[1], a[2] + y1, a[2] + y1 - a[3]);
            if p.trade_quest_marker != TRADE_QUEST_MARKER_NONE {
                let s = &mut p.statics;
                if s.trade_quest_marker_bob_timer == 0 {
                    s.trade_quest_marker_bob_state += 1;
                    match s.trade_quest_marker_bob_state {
                        1 => {
                            s.trade_quest_marker_bob_y = 3;
                            s.trade_quest_marker_bob_timer = 8;
                        }
                        2 => {
                            s.trade_quest_marker_bob_state = 0;
                            s.trade_quest_marker_bob_y = 0;
                            s.trade_quest_marker_bob_timer = 6;
                        }
                        _ => {}
                    }
                } else {
                    s.trade_quest_marker_bob_timer -= 1;
                }
                let j = after_sections + (WORLD_MAP_QUAD_POINT_FIRST + p.trade_quest_marker as usize) * 4;
                let i = after_sections + WORLD_MAP_QUAD_TRADE_QUEST_MARKER * 4;
                let (x0, y0) = (v[j].ob[0], v[j].ob[1] + 10 - s.trade_quest_marker_bob_y);
                set_quad_ob(&mut v[i..i + 4], x0, x0 + 8, y0, y0 - 8);
                for k in 0..4 {
                    v[i + k].ob[2] = 0;
                    v[i + k].cn = [255, 255, 255, alpha];
                }
                // @bug (game): `vtx[bufI]` for `vtx[i + 0]`: the first image fragment's vertex,
                // which KaleidoScope_SetVertices sets again after.
                if let Some(o) = v.get_mut(buf_i) {
                    o.tc = [0, 0];
                }
                v[i + 1].tc = [8 << 5, 0];
                v[i + 2].tc = [0, 8 << 5];
                v[i + 3].tc = [8 << 5, 8 << 5];
            }
        }
    }
    match page {
        PageVtx::Item => p.item_page_vtx = vtx,
        PageVtx::Equip => p.equip_page_vtx = vtx,
        PageVtx::Map => p.map_page_vtx = vtx,
        PageVtx::Quest => p.quest_page_vtx = vtx,
        PageVtx::Prompt => p.prompt_page_vtx = vtx,
    }
    buf_i
}
