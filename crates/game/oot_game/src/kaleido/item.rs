//! `z_kaleido_item.c`: the item page (`KaleidoScope_DrawItemSelect`: the cursor over the 6x4
//! grid, C-Left, C-Down or C-Right starting an equip, the icons, the outlines on the equipped
//! slots and the ammo) and the equip's flight to its C button (`KaleidoScope_UpdateItemEquip`).
//! The flying icon is drawn by the HUD (`Interface_Draw`, `crate::interface`).

use super::gfx::{Cc, KTex};
use super::scope::move_cursor_to_special_pos;
use super::*;
use crate::audio::sfx::{NA_SE_SY_CURSOR, NA_SE_SY_DECIDE, NA_SE_SY_ERROR, NA_SE_SY_SET_FIRE_ARROW, NA_SE_SY_SYNTH_MAGIC_ARROW};
use crate::item::*;
use crate::save::SaveContext;
use eng_input::pad::{BTN_CDOWN, BTN_CLEFT, BTN_CRIGHT};

/// `ITEM_GRID_ROWS`, `ITEM_GRID_COLS`, `ITEM_GRID_QUAD_WIDTH`, `_HEIGHT`, `_ENLARGE_OFFSET`.
const ITEM_GRID_ROWS: i16 = 4;
const ITEM_GRID_COLS: i16 = 6;
const ITEM_GRID_QUAD_WIDTH: i16 = 28;
const ITEM_GRID_QUAD_HEIGHT: i16 = 28;
const ITEM_GRID_QUAD_ENLARGE_OFFSET: i16 = 2;
/// `ITEM_QUAD_GRID_SELECTED_C_LEFT`, `ITEM_QUAD_AMMO_FIRST`.
const ITEM_QUAD_GRID_SELECTED_C_LEFT: usize = 24;
const ITEM_QUAD_AMMO_FIRST: usize = 27;

/// `gAmmoItems`: the item whose ammo a slot shows, by slot (the first 15).
pub const AMMO_ITEMS: [u8; 16] = [
    ITEM_DEKU_STICK,
    ITEM_DEKU_NUT,
    ITEM_BOMB,
    ITEM_BOW,
    ITEM_NONE,
    ITEM_NONE,
    ITEM_SLINGSHOT,
    ITEM_NONE,
    ITEM_BOMBCHU,
    ITEM_NONE,
    ITEM_NONE,
    ITEM_NONE,
    ITEM_NONE,
    ITEM_NONE,
    ITEM_MAGIC_BEAN,
    ITEM_NONE,
];

/// `sAmmoVtxOffset`: by item, its ammo's quads after `ITEM_QUAD_AMMO_FIRST` (99 none).
const AMMO_VTX_OFFSET: [usize; 17] = [0, 2, 4, 6, 99, 99, 8, 99, 99, 10, 99, 99, 99, 99, 99, 99, 12];

/// `magicArrowEffectsR`, `G`, `B`: fire, ice, light.
pub const MAGIC_ARROW_EFFECTS: [[i16; 3]; 3] = [[255, 0, 0], [100, 100, 255], [255, 255, 100]];

/// `sCButtonPosX`, `sCButtonPosY`: the C buttons' places for the flying icon (tenths).
const C_BUTTON_POS_X: [i16; 3] = [660, 900, 1140];
const C_BUTTON_POS_Y: [i16; 3] = [1100, 920, 1100];

/// `gSaveContext.save.info.inventory.items[i]` as the C reads it: past the 24 slots, the bytes
/// that follow in `Inventory` (`ammo[]`).
pub fn inventory_byte(save: &SaveContext, i: usize) -> u8 {
    match save.inventory.items.get(i) {
        Some(&v) => v,
        None => save.inventory.ammo.get(i - 24).map(|&a| a as u8).unwrap_or(ITEM_NONE),
    }
}

impl PlayState {
    /// `KaleidoScope_DrawAmmoCount`: an item's ammo as two 8x8 digits (no tens digit under 10):
    /// grey for the wrong age, darker grey at 0, green at capacity.
    fn kaleido_scope_draw_ammo_count(&mut self, item: u8) {
        let s = &self.save;
        let p = &mut self.pause_ctx;
        let alpha = p.alpha as i16;
        let mut ammo = s.ammo(item) as i16;
        if !check_age_req_slot(slot(item), s.adult) {
            p.gfx.prim_color(100, 100, 100, alpha);
        } else {
            p.gfx.prim_color(255, 255, 255, alpha);
            let full = |upg: usize| s.ammo(item) as i16 == s.cur_capacity(upg) as i16;
            if ammo == 0 {
                p.gfx.prim_color(130, 130, 130, alpha);
            } else if (item == ITEM_BOMB && full(UPG_BOMB_BAG))
                || (item == ITEM_BOW && full(UPG_QUIVER))
                || (item == ITEM_SLINGSHOT && full(UPG_BULLET_BAG))
                || (item == ITEM_DEKU_STICK && full(UPG_DEKU_STICKS))
                || (item == ITEM_DEKU_NUT && full(UPG_DEKU_NUTS))
                || (item == ITEM_BOMBCHU && ammo == 50)
                || (item == ITEM_MAGIC_BEAN && ammo == 15)
            {
                p.gfx.prim_color(120, 255, 0, alpha);
            }
        }
        let mut tens = 0;
        while ammo >= 10 {
            ammo -= 10;
            tens += 1;
        }
        let first = (ITEM_QUAD_AMMO_FIRST + AMMO_VTX_OFFSET[item as usize]) * 4;
        if tens != 0 {
            p.gfx.vertex(&p.item_vtx[first..], 4, 0);
            p.gfx.quad(KTex::AmmoDigit(tens as u8), 0);
        }
        p.gfx.vertex(&p.item_vtx[first + 4..], 4, 0);
        p.gfx.quad(KTex::AmmoDigit(ammo as u8), 0);
    }

    /// `KaleidoScope_DrawItemSelect`.
    pub(super) fn kaleido_scope_draw_item_select(&mut self) {
        let press = self.input.press;
        let adult = self.save.adult;
        // Gfx_SetupDL_42Opa, G_CC_MODULATEIA_PRIM.
        self.pause_ctx.gfx.combine(Cc::ModulateIaPrim);
        let p = &mut self.pause_ctx;
        p.cursor_color_set = 0;
        p.name_color_set = 0;
        // @bug (game): `cursorSlot` is read uninitialised when the first branch below doesn't
        // run (the equip's flight, the opening and closing): the stack's last value, which that
        // branch last left as `pauseCtx->cursorSlot[PAUSE_ITEM]`.
        let mut cursor_slot = p.cursor_slot[PAUSE_ITEM as usize];
        const IDX: usize = PAUSE_ITEM as usize;
        if p.state == PAUSE_STATE_MAIN && p.main_state == PAUSE_MAIN_STATE_IDLE && p.page_index == PAUSE_ITEM {
            let mut cursor_move_result = 0;
            let old_cursor_point = p.cursor_point[IDX];
            let mut cursor_item = p.cursor_item[IDX];
            // The inventory as the cursor reads it (25 bytes: one past the slots, see
            // KaleidoScope_SetDefaultCursor).
            let inv: [u8; 25] = std::array::from_fn(|i| inventory_byte(&self.save, i));
            let items = |i: i16| inv.get(i as usize).copied().unwrap_or(ITEM_NONE);
            let audio = &mut self.audio;

            if p.cursor_special_pos == 0 {
                p.cursor_color_set = 4;
                if cursor_item == PAUSE_ITEM_NONE {
                    p.stick_adj_x = 40;
                }
                if p.stick_adj_x.wrapping_abs() > 30 {
                    let cursor_point = p.cursor_point[IDX];
                    let cursor_x = p.cursor_x[IDX];
                    let cursor_y = p.cursor_y[IDX];
                    loop {
                        if p.stick_adj_x < -30 {
                            if p.cursor_x[IDX] != 0 {
                                // Left; an item there stops it.
                                p.cursor_x[IDX] -= 1;
                                p.cursor_point[IDX] -= 1;
                                if items(p.cursor_point[IDX]) != ITEM_NONE {
                                    cursor_move_result = 1;
                                }
                            } else {
                                // Back to the starting column, the next row down.
                                p.cursor_x[IDX] = cursor_x;
                                p.cursor_y[IDX] += 1;
                                if p.cursor_y[IDX] >= ITEM_GRID_ROWS {
                                    p.cursor_y[IDX] = 0;
                                }
                                p.cursor_point[IDX] = p.cursor_x[IDX] + p.cursor_y[IDX] * ITEM_GRID_COLS;
                                if p.cursor_point[IDX] >= ITEM_GRID_ROWS * ITEM_GRID_COLS {
                                    p.cursor_point[IDX] = p.cursor_x[IDX];
                                }
                                if cursor_y == p.cursor_y[IDX] {
                                    // No item left of the start on any row: the left arrow.
                                    p.cursor_x[IDX] = cursor_x;
                                    p.cursor_point[IDX] = cursor_point;
                                    move_cursor_to_special_pos(p, audio, PAUSE_CURSOR_PAGE_LEFT);
                                    cursor_move_result = 2;
                                }
                            }
                        } else if p.stick_adj_x > 30 {
                            if p.cursor_x[IDX] < ITEM_GRID_COLS - 1 {
                                p.cursor_x[IDX] += 1;
                                p.cursor_point[IDX] += 1;
                                if items(p.cursor_point[IDX]) != ITEM_NONE {
                                    cursor_move_result = 1;
                                }
                            } else {
                                p.cursor_x[IDX] = cursor_x;
                                p.cursor_y[IDX] += 1;
                                if p.cursor_y[IDX] >= ITEM_GRID_ROWS {
                                    p.cursor_y[IDX] = 0;
                                }
                                p.cursor_point[IDX] = p.cursor_x[IDX] + p.cursor_y[IDX] * ITEM_GRID_COLS;
                                if p.cursor_point[IDX] >= ITEM_GRID_ROWS * ITEM_GRID_COLS {
                                    p.cursor_point[IDX] = p.cursor_x[IDX];
                                }
                                if cursor_y == p.cursor_y[IDX] {
                                    p.cursor_x[IDX] = cursor_x;
                                    p.cursor_point[IDX] = cursor_point;
                                    move_cursor_to_special_pos(p, audio, PAUSE_CURSOR_PAGE_RIGHT);
                                    cursor_move_result = 2;
                                }
                            }
                        }
                        if cursor_move_result != 0 {
                            break;
                        }
                    }
                    if cursor_move_result == 1 {
                        cursor_item = items(p.cursor_point[IDX]) as u16;
                    }
                }
            } else if p.cursor_special_pos == PAUSE_CURSOR_PAGE_LEFT {
                if p.stick_adj_x > 30 {
                    p.name_display_timer = 0;
                    p.cursor_special_pos = 0;
                    audio.play_sfx_centered(NA_SE_SY_CURSOR);
                    // Column by column from the top left: the first item.
                    let (mut cursor_point, mut cursor_x, mut cursor_y) = (0i16, 0i16, 0i16);
                    loop {
                        if items(cursor_point) != ITEM_NONE {
                            p.cursor_point[IDX] = cursor_point;
                            p.cursor_x[IDX] = cursor_x;
                            p.cursor_y[IDX] = cursor_y;
                            cursor_move_result = 1;
                            break;
                        }
                        cursor_y += 1;
                        cursor_point += ITEM_GRID_COLS;
                        if cursor_y >= ITEM_GRID_ROWS {
                            cursor_y = 0;
                            cursor_point = cursor_x + 1;
                            cursor_x = cursor_point;
                            if cursor_x >= ITEM_GRID_COLS {
                                move_cursor_to_special_pos(p, audio, PAUSE_CURSOR_PAGE_RIGHT);
                                break;
                            }
                        }
                    }
                }
            } else if p.stick_adj_x < -30 {
                // On the right arrow, the stick left: column by column from the top right.
                p.name_display_timer = 0;
                p.cursor_special_pos = 0;
                audio.play_sfx_centered(NA_SE_SY_CURSOR);
                let (mut cursor_point, mut cursor_x, mut cursor_y) = (ITEM_GRID_COLS - 1, ITEM_GRID_COLS - 1, 0i16);
                loop {
                    if items(cursor_point) != ITEM_NONE {
                        p.cursor_point[IDX] = cursor_point;
                        p.cursor_x[IDX] = cursor_x;
                        p.cursor_y[IDX] = cursor_y;
                        cursor_move_result = 1;
                        break;
                    }
                    cursor_y += 1;
                    cursor_point += ITEM_GRID_COLS;
                    if cursor_y >= ITEM_GRID_ROWS {
                        cursor_y = 0;
                        cursor_point = cursor_x - 1;
                        cursor_x = cursor_point;
                        if cursor_x < 0 {
                            move_cursor_to_special_pos(p, audio, PAUSE_CURSOR_PAGE_LEFT);
                            break;
                        }
                    }
                }
            }

            if p.cursor_special_pos == 0 {
                if cursor_item != PAUSE_ITEM_NONE && p.stick_adj_y.wrapping_abs() > 30 {
                    cursor_move_result = 0;
                    let cursor_point = p.cursor_point[IDX];
                    let cursor_y = p.cursor_y[IDX];
                    loop {
                        if p.stick_adj_y > 30 {
                            if p.cursor_y[IDX] != 0 {
                                p.cursor_y[IDX] -= 1;
                                p.cursor_point[IDX] -= ITEM_GRID_COLS;
                                if items(p.cursor_point[IDX]) != ITEM_NONE {
                                    cursor_move_result = 1;
                                }
                            } else {
                                p.cursor_y[IDX] = cursor_y;
                                p.cursor_point[IDX] = cursor_point;
                                cursor_move_result = 2;
                            }
                        } else if p.stick_adj_y < -30 {
                            if p.cursor_y[IDX] < ITEM_GRID_ROWS - 1 {
                                p.cursor_y[IDX] += 1;
                                p.cursor_point[IDX] += ITEM_GRID_COLS;
                                if items(p.cursor_point[IDX]) != ITEM_NONE {
                                    cursor_move_result = 1;
                                }
                            } else {
                                p.cursor_y[IDX] = cursor_y;
                                p.cursor_point[IDX] = cursor_point;
                                cursor_move_result = 2;
                            }
                        }
                        if cursor_move_result != 0 {
                            break;
                        }
                    }
                }

                cursor_slot = p.cursor_point[IDX] as u16;
                p.cursor_color_set = 4;
                if cursor_move_result == 1 || cursor_move_result != 2 {
                    cursor_item = items(p.cursor_point[IDX]) as u16;
                }
                p.cursor_item[IDX] = cursor_item;
                p.cursor_slot[IDX] = cursor_slot;
                if !check_age_req_slot(cursor_slot as usize, adult) {
                    p.name_color_set = 1;
                }
                if cursor_item != PAUSE_ITEM_NONE {
                    let index = cursor_slot as usize * 4;
                    // KaleidoScope_SetCursorPos.
                    p.cursor_vtx[0].ob[0] = p.item_vtx[index].ob[0];
                    p.cursor_vtx[0].ob[1] = p.item_vtx[index].ob[1];
                    if p.debug_state == PAUSE_DEBUG_STATE_CLOSED && p.state == PAUSE_STATE_MAIN && p.main_state == PAUSE_MAIN_STATE_IDLE && press.button & (BTN_CLEFT | BTN_CDOWN | BTN_CRIGHT) != 0 {
                        if check_age_req_slot(cursor_slot as usize, adult) && cursor_item != ITEM_SOLD_OUT as u16 {
                            if press.held(BTN_CLEFT) {
                                p.equip_target_c_btn = 0;
                            } else if press.held(BTN_CDOWN) {
                                p.equip_target_c_btn = 1;
                            } else if press.held(BTN_CRIGHT) {
                                p.equip_target_c_btn = 2;
                            }
                            p.equip_target_item = cursor_item;
                            p.equip_target_slot = cursor_slot;
                            p.main_state = PAUSE_MAIN_STATE_3;
                            p.equip_anim_x = p.item_vtx[index].ob[0].wrapping_mul(10);
                            p.equip_anim_y = p.item_vtx[index].ob[1].wrapping_mul(10);
                            p.equip_anim_alpha = 255;
                            p.statics.equip_anim_timer = 0;
                            p.statics.equip_state = 3;
                            p.statics.equip_move_timer = 10;
                            let t = p.equip_target_item as u8;
                            if t == ITEM_ARROW_FIRE || t == ITEM_ARROW_ICE || t == ITEM_ARROW_LIGHT {
                                let index = if t == ITEM_ARROW_ICE {
                                    1
                                } else if t == ITEM_ARROW_LIGHT {
                                    2
                                } else {
                                    0
                                };
                                audio.play_sfx_centered(NA_SE_SY_SET_FIRE_ARROW + index);
                                p.equip_target_item = 0xBF + index;
                                p.statics.equip_state = 0;
                                p.equip_anim_alpha = 0;
                                p.statics.equip_move_timer = 6;
                            } else {
                                audio.play_sfx_centered(NA_SE_SY_DECIDE);
                            }
                        } else {
                            audio.play_sfx_centered(NA_SE_SY_ERROR);
                        }
                    }
                } else {
                    // No item: the cursor out of view.
                    for k in 0..4 {
                        p.cursor_vtx[k].ob[0] = 0;
                        p.cursor_vtx[k].ob[1] = -200;
                    }
                }
            } else {
                p.cursor_item[IDX] = PAUSE_ITEM_NONE;
            }
            if old_cursor_point != p.cursor_point[IDX] {
                audio.play_sfx_centered(NA_SE_SY_CURSOR);
            }
        } else if p.main_state == PAUSE_MAIN_STATE_3 && p.page_index == PAUSE_ITEM {
            // KaleidoScope_SetCursorPos at the uninitialised cursorSlot (see above).
            let index = cursor_slot as usize * 4;
            if let Some(v) = p.item_vtx.get(index).copied() {
                p.cursor_vtx[0].ob[0] = v.ob[0];
                p.cursor_vtx[0].ob[1] = v.ob[1];
            }
            p.cursor_color_set = 4;
        }

        let p = &mut self.pause_ctx;
        // @bug (game): this combiner goes to OVERLAY_DISP (which the HUD sets again): the
        // outlines draw under G_CC_MODULATEIA_PRIM.
        p.gfx.prim_color(255, 255, 255, p.alpha as i16);
        p.gfx.env_color(0, 0, 0, 0);
        for i in 0..3 {
            if self.save.equips.button_items[i + 1] != ITEM_NONE {
                let j = (ITEM_QUAD_GRID_SELECTED_C_LEFT + i) * 4;
                p.gfx.vertex(&p.item_vtx[j..], 4, 0);
                p.gfx.quad(KTex::EquippedOutline, 0);
            }
        }

        p.gfx.combine(Cc::ModulateIaPrim);
        for i in 0..(ITEM_GRID_ROWS * ITEM_GRID_COLS) as usize {
            let j = i * 4;
            p.gfx.prim_color(255, 255, 255, p.alpha as i16);
            let item = self.save.inventory.items[i];
            if item != ITEM_NONE {
                if p.main_state == PAUSE_MAIN_STATE_IDLE && p.page_index == PAUSE_ITEM && p.cursor_special_pos == 0 && check_age_req_slot(i, adult) {
                    let enlarge = |v: &mut [Vtx]| {
                        let x0 = v[0].ob[0] - ITEM_GRID_QUAD_ENLARGE_OFFSET;
                        let y0 = v[0].ob[1] + ITEM_GRID_QUAD_ENLARGE_OFFSET;
                        v[0].ob[0] = x0;
                        v[2].ob[0] = x0;
                        v[1].ob[0] = x0 + ITEM_GRID_QUAD_WIDTH + ITEM_GRID_QUAD_ENLARGE_OFFSET * 2;
                        v[3].ob[0] = v[1].ob[0];
                        v[0].ob[1] = y0;
                        v[1].ob[1] = y0;
                        v[2].ob[1] = y0 - (ITEM_GRID_QUAD_HEIGHT + ITEM_GRID_QUAD_ENLARGE_OFFSET * 2);
                        v[3].ob[1] = v[2].ob[1];
                    };
                    if p.statics.equip_state == 2 && i == 3 {
                        let c = MAGIC_ARROW_EFFECTS[(p.equip_target_item as usize).wrapping_sub(0xBF).min(2)];
                        p.gfx.prim_color(c[0], c[1], c[2], p.alpha as i16);
                        enlarge(&mut p.item_vtx[j..j + 4]);
                    } else if i == cursor_slot as usize {
                        // The item under the cursor, enlarged.
                        enlarge(&mut p.item_vtx[j..j + 4]);
                    }
                }
                p.gfx.vertex(&p.item_vtx[j..], 4, 0);
                // The icons the menu greyed when it opened (KaleidoScope_GrayOutTextureRGBA32 on
                // !CHECK_AGE_REQ_ITEM).
                let tex = if check_age_req_item(item as usize, adult) { KTex::ItemIcon(item) } else { KTex::ItemIconGray(item) };
                p.gfx.quad(tex, 0);
            }
        }

        if p.cursor_special_pos == 0 {
            self.kaleido_scope_draw_cursor(PAUSE_ITEM);
        }

        self.pause_ctx.gfx.combine(Cc::PrimEnvTexel);
        for i in 0..15 {
            let item = self.save.inventory.items[i];
            if AMMO_ITEMS[i] != ITEM_NONE && item != ITEM_NONE {
                self.kaleido_scope_draw_ammo_count(item);
            }
        }
    }

    /// `KaleidoScope_UpdateItemEquip` (`PAUSE_MAIN_STATE_3`): the icon flies from its slot to the
    /// C button (10 frames, shrinking: `WREG(90)` less `WREG(87) / sEquipMoveTimer` a frame), a
    /// magic arrow first fading in and joining the bow; then the C buttons' swap and the equip.
    pub(super) fn kaleido_scope_update_item_equip(&mut self) {
        let p = &mut self.pause_ctx;
        let s = &mut p.statics;
        if s.equip_state == 0 {
            p.equip_anim_alpha += 14;
            if p.equip_anim_alpha > 255 {
                p.equip_anim_alpha = 254;
                s.equip_state += 1;
            }
            s.equip_anim_timer = 5;
            return;
        }
        if s.equip_state == 2 {
            s.d_8082a488 -= 1;
            if s.d_8082a488 == 0 {
                // The arrow becomes the bow with it.
                p.equip_target_item -= 0xBF - ITEM_BOW_FIRE as u16;
                p.equip_target_slot = SLOT_BOW as u16;
                s.equip_move_timer = 6;
                p.regs.wreg90 = 320;
                p.regs.wreg87 = p.regs.wreg91;
                s.equip_state += 1;
                self.audio.play_sfx_centered(NA_SE_SY_SYNTH_MAGIC_ARROW);
            }
            return;
        }
        let (target_x, target_y) = if s.equip_state == 1 {
            // To the bow's slot.
            (p.item_vtx[12].ob[0] as i32 * 10, p.item_vtx[12].ob[1] as i32 * 10)
        } else {
            let b = p.equip_target_c_btn as usize;
            (C_BUTTON_POS_X[b] as i32, C_BUTTON_POS_Y[b] as i32)
        };
        let mt = s.equip_move_timer as i32;
        let offset_x = ((p.equip_anim_x as i32 - target_x).abs() / mt) as u16;
        let offset_y = ((p.equip_anim_y as i32 - target_y).abs() / mt) as u16;
        if p.equip_target_item >= 0xBF && p.equip_anim_alpha < 254 {
            p.equip_anim_alpha += 14;
            if p.equip_anim_alpha > 255 {
                p.equip_anim_alpha = 254;
            }
            s.equip_anim_timer = 5;
            return;
        }
        if s.equip_anim_timer == 0 {
            let r = &mut p.regs;
            r.wreg90 -= r.wreg87 / s.equip_move_timer;
            r.wreg87 -= r.wreg87 / s.equip_move_timer;
            if p.equip_anim_x as i32 >= target_x {
                p.equip_anim_x = (p.equip_anim_x as i32 - offset_x as i32) as i16;
            } else {
                p.equip_anim_x = (p.equip_anim_x as i32 + offset_x as i32) as i16;
            }
            if p.equip_anim_y as i32 >= target_y {
                p.equip_anim_y = (p.equip_anim_y as i32 - offset_y as i32) as i16;
            } else {
                p.equip_anim_y = (p.equip_anim_y as i32 + offset_y as i32) as i16;
            }
            s.equip_move_timer -= 1;
            if s.equip_move_timer == 0 {
                if s.equip_state == 1 {
                    s.equip_state += 1;
                    s.d_8082a488 = 4;
                    return;
                }
                let (item, slot) = (p.equip_target_item, p.equip_target_slot);
                let (item, slot) = item_equip_write(&mut self.save, p.equip_target_c_btn as usize, item, slot);
                let p = &mut self.pause_ctx;
                p.equip_target_item = item;
                p.equip_target_slot = slot;
                p.main_state = PAUSE_MAIN_STATE_IDLE;
                p.statics.equip_move_timer = 10;
                p.regs.wreg90 = 320;
                p.regs.wreg87 = p.regs.wreg91;
            }
        } else {
            s.equip_anim_timer -= 1;
            if s.equip_anim_timer == 0 {
                p.equip_anim_alpha = 255;
            }
        }
    }
}

/// Whether a C button's item is the bow or a bow with magic arrows.
fn is_bow(item: u8) -> bool {
    item == ITEM_BOW || (ITEM_BOW_FIRE..=ITEM_BOW_LIGHT).contains(&item)
}

/// `KaleidoScope_UpdateItemEquip`'s end: C button `btn` (0 C-Left, 1 C-Down, 2 C-Right) gets
/// `item` from `slot`. An item already on one of the other two swaps with `btn`'s (or leaves its
/// button empty); a magic arrow onto a bow becomes the bow with it; the bow onto a button beside
/// a bow with arrows gives that one `btn`'s item. (`Interface_LoadItemIcon1/2` load nothing: the
/// HUD draws the buttons' items.) Returns the item and slot as they end up.
pub fn item_equip_write(save: &mut SaveContext, btn: usize, mut item: u16, mut slot: u16) -> (u16, u16) {
    let e = &mut save.equips;
    let magic = (0xBF..=0xC1).contains(&item);
    let mine = btn + 1;
    // The other two, in the C's order.
    let others: [usize; 2] = match btn {
        0 => [1, 2],
        1 => [0, 2],
        _ => [0, 1],
    };
    if let Some(&o) = others.iter().find(|&&o| slot == e.c_button_slots[o] as u16) {
        if e.button_items[mine] != ITEM_NONE {
            if magic && is_bow(e.button_items[mine]) {
                item -= 0xBF - ITEM_BOW_FIRE as u16;
                slot = SLOT_BOW as u16;
            } else {
                e.button_items[o + 1] = e.button_items[mine];
                e.c_button_slots[o] = e.c_button_slots[btn];
            }
        } else {
            e.button_items[o + 1] = ITEM_NONE;
            e.c_button_slots[o] = SLOT_NONE;
        }
    }
    if (0xBF..=0xC1).contains(&item) {
        if is_bow(e.button_items[mine]) {
            item -= 0xBF - ITEM_BOW_FIRE as u16;
            slot = SLOT_BOW as u16;
        }
    } else if item == ITEM_BOW as u16 {
        // A bow with arrows on another button gets this button's item.
        for &o in &others {
            if (ITEM_BOW_FIRE..=ITEM_BOW_LIGHT).contains(&e.button_items[o + 1]) {
                e.button_items[o + 1] = e.button_items[mine];
                // @bug (game): only C-Left's equip copies the slot too (C-Down's and
                // C-Right's leave the other button's slot).
                if btn == 0 {
                    e.c_button_slots[o] = e.c_button_slots[btn];
                }
                break;
            }
        }
    }
    e.button_items[mine] = item as u8;
    e.c_button_slots[btn] = slot as u8;
    (item, slot)
}
