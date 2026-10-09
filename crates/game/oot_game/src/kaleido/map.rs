//! `z_kaleido_map.c`: the dungeon map page (`KaleidoScope_DrawDungeonMap`): its cursor over the
//! floors' column and the dungeon items' column (input in the draw, as the item page's), the
//! dungeon's title, the boss key, compass and map owned, the floors' buttons, Link's head at his
//! floor, the boss's skull with the compass, the Gold Skulltula icon, and the viewed floor's two
//! room maps, their current room pulsing. The world map (`KaleidoScope_DrawWorldMap`) logs
//! (`super::scope`).

use super::gfx::{Cc, KTex};
use super::scope::{AREA_GS_FLAGS, move_cursor_to_special_pos};
use super::*;
use crate::audio::sfx::NA_SE_SY_CURSOR;
use crate::map::{DUNGEON_BOSS_KEY, DUNGEON_COMPASS, DUNGEON_MAP, MAP_48X85_TEX_SIZE, MAP_FLOORS, check_dungeon_item};

/// `dungeonItemTexs` (`icon_item_24_static`): by `DungeonItem`.
pub(super) const DUNGEON_ITEM_TEXS: [&str; 3] = ["gQuestIconDungeonBossKeyTex", "gQuestIconDungeonCompassTex", "gQuestIconDungeonMapTex"];
/// `dungeonTitleTexs` (`icon_item_nes_static`, IA8 96x16): by `mapIndex`.
pub(super) const DUNGEON_TITLE_TEXS: [&str; 10] = [
    "gPauseDekuTitleENGTex",
    "gPauseDodongoTitleENGTex",
    "gPauseJabuTitleENGTex",
    "gPauseForestTitleENGTex",
    "gPauseFireTitleENGTex",
    "gPauseWaterTitleENGTex",
    "gPauseSpiritTitleENGTex",
    "gPauseShadowTitleENGTex",
    "gPauseBotWTitleENGTex",
    "gPauseIceCavernTitleENGTex",
];
/// `gPauseDekuTitleENGTex`'s width (the titles' quad, `KaleidoScope_DrawDungeonMap`'s 96).
pub(super) const DUNGEON_TITLE_WIDTH: u32 = 96;
/// `floorIconTexs` (`icon_item_dungeon_static`, IA8 24x16): by `FloorID` (`F_NA` the blank one).
pub(super) const FLOOR_ICON_TEXS: [&str; 17] = [
    "gDungeonMapBlankFloorButtonTex",
    "gDungeonMap8FButtonTex",
    "gDungeonMap7FButtonTex",
    "gDungeonMap6FButtonTex",
    "gDungeonMap5FButtonTex",
    "gDungeonMap4FButtonTex",
    "gDungeonMap3FButtonTex",
    "gDungeonMap2FButtonTex",
    "gDungeonMap1FButtonTex",
    "gDungeonMapB1ButtonTex",
    "gDungeonMapB2ButtonTex",
    "gDungeonMapB3ButtonTex",
    "gDungeonMapB4ButtonTex",
    "gDungeonMapB5ButtonTex",
    "gDungeonMapB6ButtonTex",
    "gDungeonMapB7ButtonTex",
    "gDungeonMapB8ButtonTex",
];
/// `gDungeonMapLinkHeadTex`, `gDungeonMapSkullTex` (RGBA16 16x16).
pub(super) const LINK_HEAD_TEX: &str = "gDungeonMapLinkHeadTex";
pub(super) const SKULL_TEX: &str = "gDungeonMapSkullTex";
/// `gQuestIconGoldSkulltulaTex` (`icon_item_24_static`, `QUEST_ICON_WIDTH` by `_HEIGHT`, 24).
pub(super) const GOLD_SKULLTULA_TEX: &str = "gQuestIconGoldSkulltulaTex";

/// `mapBgPulseColors`: the current room's two colours, in 5 bits (`/ 8`).
const MAP_BG_PULSE_COLORS: [[i16; 3]; 2] = [[0 / 8, 80 / 8, 255 / 8], [0 / 8, 200 / 8, 140 / 8]];
/// `ITEM_DUNGEON_BOSS_KEY` (`item.h`): the items' column names the boss key, compass and map.
const ITEM_DUNGEON_BOSS_KEY: u16 = 0x74;
/// `SCENE_TREASURE_BOX_SHOP` (0x10): `SCENE_DEKU_TREE` (0) up to it are the scenes the pulse runs
/// in (`KaleidoScope_UpdateDungeonMap`'s range too).
pub(super) const SCENE_TREASURE_BOX_SHOP: u16 = 0x10;
/// The floor buttons' quads (`VTX_PAGE_MAP_DUNGEON`'s 6 to 13, `mapPageVtx[84]`), and the cursor
/// slots' first (`(cursorSlot + 18) * 4`: slot 3, the top floor, is quad 21).
const MAP_VTX_CURSOR_QUAD_OFFSET: usize = 18;

impl PlayState {
    /// `CHECK_DUNGEON_ITEM(item, gSaveContext.mapIndex)`.
    fn check_dungeon_item(&self, item: u32) -> bool {
        check_dungeon_item(&self.save, item, self.save.map_index)
    }

    /// `R_MAP_TEX_INDEX = R_MAP_TEX_INDEX_BASE + gMapData->floorTexIndexOffset[mapIndex][floor]`.
    fn map_tex_index_for(&self, floor: i16) -> i16 {
        let Some(a) = self.assets.as_ref() else { return self.map.r_map_tex_index };
        self.map.r_map_tex_index_base + a.map.data.floor_tex_index_offset(self.save.map_index as usize, floor.max(0) as usize)
    }

    /// `gMapData->floorID[interfaceCtx->unk_25A][i]`, read on past the row as the C does (the
    /// next dungeon's floors; none past the table).
    fn floor_id(&self, i: usize) -> u8 {
        let Some(a) = self.assets.as_ref() else { return 0 };
        let row = self.map.unk_25a.max(0) as usize;
        a.map.data.floor_id.get(row * MAP_FLOORS + i).copied().unwrap_or(0)
    }

    /// A floor the floors' column can move to: visited, or any of the dungeon's with the map.
    fn floor_shown(&self, i: usize) -> bool {
        let floors = self.save.scene_flags(self.save.map_index).floors;
        floors & (1u32 << (i & 31)) != 0 || (self.check_dungeon_item(DUNGEON_MAP) && self.floor_id(i) != 0)
    }

    /// `KaleidoScope_SetCursorPos(pauseCtx, index, pauseCtx->mapPageVtx)`: the cursor's top left
    /// at the vertex.
    fn set_cursor_pos_map(&mut self, index: usize) {
        let p = &mut self.pause_ctx;
        if let Some(v) = p.map_page_vtx.get(index).copied() {
            p.cursor_vtx[0].ob[0] = v.ob[0];
            p.cursor_vtx[0].ob[1] = v.ob[1];
        }
    }

    /// `KaleidoScope_DrawDungeonMap`.
    pub(super) fn kaleido_scope_draw_dungeon_map(&mut self) {
        const M: usize = PAUSE_MAP as usize;
        let p = &self.pause_ctx;
        if p.state == PAUSE_STATE_MAIN && p.main_state == PAUSE_MAIN_STATE_IDLE && p.page_index == PAUSE_MAP {
            self.pause_ctx.cursor_color_set = 0;
            let old_cursor_point = self.pause_ctx.cursor_point[M];
            let (stick_x, stick_y) = (self.pause_ctx.stick_adj_x, self.pause_ctx.stick_adj_y);
            if self.pause_ctx.cursor_special_pos == 0 {
                if stick_x > 30 {
                    if self.pause_ctx.cursor_x[M] != 0 {
                        move_cursor_to_special_pos(&mut self.pause_ctx, &mut self.audio, PAUSE_CURSOR_PAGE_RIGHT);
                    } else {
                        // To the items' column, its first item owned.
                        self.pause_ctx.cursor_x[M] = 1;
                        self.pause_ctx.cursor_point[M] = 0;
                        if !self.check_dungeon_item(DUNGEON_BOSS_KEY) {
                            self.pause_ctx.cursor_point[M] += 1;
                            if !self.check_dungeon_item(DUNGEON_COMPASS) {
                                self.pause_ctx.cursor_point[M] += 1;
                                if !self.check_dungeon_item(DUNGEON_MAP) {
                                    move_cursor_to_special_pos(&mut self.pause_ctx, &mut self.audio, PAUSE_CURSOR_PAGE_RIGHT);
                                }
                            }
                        }
                    }
                } else if stick_x < -30 {
                    if self.pause_ctx.cursor_x[M] == 0 {
                        move_cursor_to_special_pos(&mut self.pause_ctx, &mut self.audio, PAUSE_CURSOR_PAGE_LEFT);
                    } else {
                        // Back to the floors' column, on the floor viewed.
                        self.pause_ctx.cursor_x[M] = 0;
                        self.pause_ctx.cursor_point[M] = self.pause_ctx.dungeon_map_slot;
                        self.map.r_map_tex_index = self.map_tex_index_for(self.pause_ctx.cursor_point[M] - 3);
                        self.kaleido_scope_update_dungeon_map();
                    }
                }

                let cursor_point = self.pause_ctx.cursor_point[M];
                if cursor_point < 3 {
                    // The items' column: up and down to the next item owned.
                    if stick_y > 30 {
                        if cursor_point != 0 {
                            for i in (0..cursor_point).rev() {
                                if self.check_dungeon_item(i as u32) {
                                    self.pause_ctx.cursor_point[M] = i;
                                    break;
                                }
                            }
                        }
                    } else if stick_y < -30 && cursor_point != 2 {
                        for i in cursor_point + 1..3 {
                            if self.check_dungeon_item(i as u32) {
                                self.pause_ctx.cursor_point[M] = i;
                                break;
                            }
                        }
                    }
                } else {
                    // The floors' column: up and down to the next floor shown.
                    if stick_y > 30 {
                        if cursor_point >= 4 {
                            for i in (0..=cursor_point - 3 - 1).rev() {
                                if self.floor_shown(i as usize) {
                                    self.pause_ctx.cursor_point[M] = i + 3;
                                    break;
                                }
                            }
                        }
                    } else if stick_y < -30 && cursor_point != 10 {
                        // @bug (game): `i < 11` tries floors 8 to 10, past the dungeon's 8: no
                        // floor bit is set there, and `floorID` reads the next dungeon's first
                        // floors (none of the dungeons has any), so it never stops on one.
                        for i in cursor_point - 3 + 1..11 {
                            if self.floor_shown(i as usize) {
                                self.pause_ctx.cursor_point[M] = i + 3;
                                break;
                            }
                        }
                    }
                    let i = self.map.r_map_tex_index;
                    self.map.r_map_tex_index = self.map_tex_index_for(self.pause_ctx.cursor_point[M] - 3);
                    self.pause_ctx.dungeon_map_slot = self.pause_ctx.cursor_point[M];
                    if i != self.map.r_map_tex_index {
                        self.kaleido_scope_update_dungeon_map();
                    }
                }
            } else if self.pause_ctx.cursor_special_pos == PAUSE_CURSOR_PAGE_LEFT {
                if stick_x > 30 {
                    let p = &mut self.pause_ctx;
                    p.name_display_timer = 0;
                    p.cursor_special_pos = 0;
                    p.cursor_point[M] = p.dungeon_map_slot;
                    p.cursor_slot[M] = p.dungeon_map_slot as u16;
                    p.cursor_x[M] = 0;
                    let j = (p.cursor_slot[M] as usize + MAP_VTX_CURSOR_QUAD_OFFSET) * 4;
                    self.set_cursor_pos_map(j);
                    self.audio.play_sfx_centered(NA_SE_SY_CURSOR);
                }
            } else if stick_x < -30 {
                // From the right arrow: the items' column's first item owned, else the floors'.
                let p = &mut self.pause_ctx;
                p.name_display_timer = 0;
                p.cursor_special_pos = 0;
                p.cursor_x[M] = 1;
                p.cursor_point[M] = 0;
                if !self.check_dungeon_item(DUNGEON_BOSS_KEY) {
                    self.pause_ctx.cursor_point[M] += 1;
                    if !self.check_dungeon_item(DUNGEON_COMPASS) {
                        self.pause_ctx.cursor_point[M] += 1;
                        if !self.check_dungeon_item(DUNGEON_MAP) {
                            let p = &mut self.pause_ctx;
                            p.cursor_x[M] = 0;
                            p.cursor_point[M] = p.dungeon_map_slot;
                            p.cursor_slot[M] = p.dungeon_map_slot as u16;
                            self.map.r_map_tex_index = self.map_tex_index_for(self.pause_ctx.cursor_point[M] - 3);
                            self.kaleido_scope_update_dungeon_map();
                        }
                    }
                } else {
                    self.pause_ctx.cursor_slot[M] = self.pause_ctx.cursor_point[M] as u16;
                }
                // (The compass's or the map's slot isn't set here: the cursor's position below.)
                let j = (self.pause_ctx.cursor_slot[M] as usize + MAP_VTX_CURSOR_QUAD_OFFSET) * 4;
                self.set_cursor_pos_map(j);
                self.audio.play_sfx_centered(NA_SE_SY_CURSOR);
            }

            if old_cursor_point != self.pause_ctx.cursor_point[M] {
                self.audio.play_sfx_centered(NA_SE_SY_CURSOR);
            }
        }

        if self.pause_ctx.cursor_special_pos == 0 {
            let p = &mut self.pause_ctx;
            p.cursor_item[M] = if p.cursor_point[M] < 3 { ITEM_DUNGEON_BOSS_KEY + p.cursor_point[M] as u16 } else { PAUSE_ITEM_NONE };
            p.cursor_slot[M] = p.cursor_point[M] as u16;
            let j = (p.cursor_slot[M] as usize + MAP_VTX_CURSOR_QUAD_OFFSET) * 4;
            self.set_cursor_pos_map(j);
            let p = &mut self.pause_ctx;
            if p.cursor_x[M] == 0 {
                // The floor's button under the cursor, 2 bigger each side (4 to the right and below).
                let v = &mut p.map_page_vtx;
                if v.len() >= j + 4 {
                    let x0 = v[j].ob[0] - 2;
                    v[j].ob[0] = x0;
                    v[j + 2].ob[0] = x0;
                    let x1 = v[j + 1].ob[0] + 4;
                    v[j + 1].ob[0] = x1;
                    v[j + 3].ob[0] = x1;
                    let y0 = v[j].ob[1] + 2;
                    v[j].ob[1] = y0;
                    v[j + 1].ob[1] = y0;
                    let y1 = v[j + 2].ob[1] - 4;
                    v[j + 2].ob[1] = y1;
                    v[j + 3].ob[1] = y1;
                }
            }
        }

        let map_index = self.save.map_index;
        let dungeon_items: [bool; 3] = std::array::from_fn(|i| self.check_dungeon_item(i as u32));
        let floors = self.save.scene_flags(map_index).floors;
        let floor_ids: [u8; 8] = std::array::from_fn(|i| self.floor_id(i));
        let viewed = (self.pause_ctx.dungeon_map_slot - 3).max(0);
        let viewed_id = self.floor_id(viewed as usize);
        let gs = self.save.get_gs_flags(map_index as i32) == AREA_GS_FLAGS.get(map_index as usize).copied().unwrap_or(0) as u32;
        let skull_y = self.assets.as_ref().and_then(|a| a.map.data.skull_floor_icon_y.get(map_index as usize).copied()).unwrap_or(-99);
        let floor = self.map.floor;
        let in_pulse_scenes = self.scene_id <= SCENE_TREASURE_BOX_SHOP;
        let p = &mut self.pause_ctx;
        let alpha = p.alpha as i16;
        let g = &mut p.gfx;

        // The dungeon's title, coloured by its quad's vertices.
        g.prim_color(255, 255, 255, alpha);
        g.combine(Cc::ModulateIa);
        g.vertex(&p.map_page_vtx[68..], 16, 0);
        let title = DUNGEON_TITLE_TEXS.get(map_index as usize).copied().unwrap_or(DUNGEON_TITLE_TEXS[0]);
        g.quad(KTex::Label(title, DUNGEON_TITLE_WIDTH), 0);

        // The boss key, compass and map owned.
        g.combine(Cc::ModulateIaPrim);
        for (i, j) in (0..3).zip((4..).step_by(4)) {
            if dungeon_items[i] {
                g.quad(KTex::Icon24(DUNGEON_ITEM_TEXS[i]), j);
            }
        }

        // The floors' buttons: the visited ones (all with the map), but the one viewed.
        g.combine(Cc::ModulateIaPrim);
        g.prim_color(255, 255, 200, alpha);
        g.vertex(&p.map_page_vtx[84..], 32, 0);
        for (i, j) in (0..8usize).zip((0..).step_by(4)) {
            if (floors & (1 << i) != 0 || dungeon_items[DUNGEON_MAP as usize]) && i as i16 != p.dungeon_map_slot - 3 {
                g.quad(KTex::DungeonMap(FLOOR_ICON_TEXS[floor_ids[i] as usize % FLOOR_ICON_TEXS.len()]), j);
            }
        }
        // The one viewed, in blue (below the lowest floor, `VREG(30)` 8, past the loaded
        // vertices: nothing to see).
        let j = viewed as usize * 4;
        g.prim_color(150, 150, 255, alpha);
        g.quad(KTex::DungeonMap(FLOOR_ICON_TEXS[viewed_id as usize % FLOOR_ICON_TEXS.len()]), j);

        // The Gold Skulltula icon's quad: 19 square, 2 in from its corner.
        let v = &mut p.map_page_vtx;
        let x0 = v[124].ob[0] + 2;
        v[124].ob[0] = x0;
        v[126].ob[0] = x0;
        let x1 = v[124].ob[0] + 19;
        v[125].ob[0] = x1;
        v[127].ob[0] = x1;
        let y0 = v[124].ob[1] - 2;
        v[124].ob[1] = y0;
        v[125].ob[1] = y0;
        let y1 = v[124].ob[1] - 19;
        v[126].ob[1] = y1;
        v[127].ob[1] = y1;

        // Link's head at his floor, and the boss's skull with the compass. The C moves their quads
        // after loading them (`gSPVertex(&mapPageVtx[116], 12, 0)`); the RSP reads the vertices
        // once the frame's list is done, so it sees them moved: they're moved first here.
        let y = p.pages_y_origin1 + 50 - (floor * 14) - 1;
        v[116].ob[1] = y;
        v[117].ob[1] = y;
        v[118].ob[1] = y - 16;
        v[119].ob[1] = y - 16;
        let skull = dungeon_items[DUNGEON_COMPASS as usize] && skull_y != -99;
        if skull {
            let y = skull_y + p.pages_y_origin1;
            v[120].ob[1] = y;
            v[121].ob[1] = y;
            v[122].ob[1] = y - 16;
            v[123].ob[1] = y - 16;
        }
        g.vertex(&p.map_page_vtx[116..], 12, 0);
        g.prim_color(255, 255, 255, alpha);
        g.quad(KTex::DungeonMap(LINK_HEAD_TEX), 0);
        if skull {
            g.quad(KTex::DungeonMap(SKULL_TEX), 4);
        }

        g.prim_color(255, 255, 255, alpha);
        if gs {
            // KaleidoScope_DrawQuadTextureRGBA32(gQuestIconGoldSkulltulaTex, 24, 24, 8).
            g.quad(KTex::Icon24(GOLD_SKULLTULA_TEX), 8);
        }

        // The current room's colour (palette entry 14) steps between the two colours over 20
        // frames each.
        if in_pulse_scenes {
            let s = &mut p.statics;
            let target = MAP_BG_PULSE_COLORS[s.map_bg_pulse_stage as usize];
            let timer = s.map_bg_pulse_timer as i32;
            let step_r = ((s.map_bg_pulse_r as i32 - target[0] as i32) / timer) as i16;
            let step_g = ((s.map_bg_pulse_g as i32 - target[1] as i32) / timer) as i16;
            let step_b = ((s.map_bg_pulse_b as i32 - target[2] as i32) / timer) as i16;
            s.map_bg_pulse_r -= step_r;
            s.map_bg_pulse_g -= step_g;
            s.map_bg_pulse_b -= step_b;
            let rgba16 = (((s.map_bg_pulse_r & 0x1F) as u16) << 11) | (((s.map_bg_pulse_g & 0x1F) as u16) << 6) | (((s.map_bg_pulse_b & 0x1F) as u16) << 1) | 1;
            self.map.map_palette[28] = (rgba16 >> 8) as u8;
            self.map.map_palette[29] = rgba16 as u8;
            s.map_bg_pulse_timer -= 1;
            if s.map_bg_pulse_timer == 0 {
                s.map_bg_pulse_stage ^= 1;
                s.map_bg_pulse_timer = 20;
            }
        }

        // The viewed floor's room maps, point sampled (G_TF_POINT: the bake's), through the
        // palette.
        g.prim_color(255, 255, 255, alpha);
        g.load_tlut_pal16(&self.map.map_palette);
        g.vertex(&p.map_page_vtx[60..], 8, 0);
        let seg = &self.map.segment;
        if let Some(t) = seg.get(..MAP_48X85_TEX_SIZE) {
            g.quad_ci4(KTex::RoomMap, 0, t);
        }
        if let Some(t) = seg.get(MAP_SEGMENT_SECOND..MAP_SEGMENT_SECOND + MAP_48X85_TEX_SIZE) {
            g.quad_ci4(KTex::RoomMap, 4, t);
        }
        // (G_TF_BILERP again.)
    }
}

/// `ALIGN16(MAP_48x85_TEX_SIZE)`: the second room map's offset in `mapSegment`.
pub(super) const MAP_SEGMENT_SECOND: usize = (MAP_48X85_TEX_SIZE + 15) & !15;
