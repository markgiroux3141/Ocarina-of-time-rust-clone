//! The pause menu's tables and rules that need no play state, against the C. The menu itself is
//! `oot_actors --test pause` (with the pack).

use super::item::item_equip_write;
use super::scope::{PAGE_SWITCH_NEXT_PAGE_INDEX, page_switch_eye_d};
use super::*;
use crate::item::*;
use crate::save::SaveContext;

#[test]
fn the_grey_is_r_plus_2g_plus_b_over_7_with_the_alpha_kept() {
    // KaleidoScope_GrayOutTextureRGBA32: ((r) + ((rgb & 0xFF00) >> 7) + (b)) / 7.
    // 0xFF8040C0: (255 + 256 + 64) / 7 = 82. Black (no colour bits) is left alone; green 1 alone
    // is 2 / 7 = 0.
    let mut t = vec![0xFF, 0x80, 0x40, 0xC0, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x01, 0x00, 0x80];
    gray_out_texture_rgba32(&mut t);
    assert_eq!(t, vec![82, 82, 82, 0xC0, 0, 0, 0, 0xFF, 0, 0, 0, 0x80]);
}

#[test]
fn the_age_requirements() {
    // gSlotAgeReqs: the slingshot's and the sticks' slots are the child's, the bow's the
    // adult's, the nuts' either's.
    assert!(check_age_req_slot(SLOT_SLINGSHOT, false));
    assert!(!check_age_req_slot(SLOT_SLINGSHOT, true));
    assert!(!check_age_req_slot(SLOT_BOW, false));
    assert!(check_age_req_slot(SLOT_DEKU_NUT, false) && check_age_req_slot(SLOT_DEKU_NUT, true));
    // gItemAgeReqs: the hookshot the adult's, the Weird Egg and the masks the child's, the
    // bottles either's; the last entry is the Giant's Knife (the adult's).
    assert!(!check_age_req_item(ITEM_HOOKSHOT as usize, false));
    assert!(!check_age_req_item(ITEM_WEIRD_EGG as usize, true));
    assert!(check_age_req_item(ITEM_BOTTLE_EMPTY as usize, false) && check_age_req_item(ITEM_BOTTLE_EMPTY as usize, true));
    assert_eq!(ITEM_AGE_REQS[ITEM_SOLD_OUT as usize], AGE_REQ_CHILD);
    assert_eq!(ITEM_AGE_REQS[ITEM_GIANTS_KNIFE as usize], AGE_REQ_ADULT);
}

#[test]
fn a_page_turn_moves_the_eye_in_16_steps_round_the_box() {
    // sPageSwitchEyeDx/Dz: -PAUSE_EYE_DIST * (next - this) / 16 by nextPageMode (page * 2, + 1
    // for left). The item page right to the map: (1 - 0, 0 - -1) → (-4, -4); left to the
    // equipment page: (-1 - 0, 0 - -1) → (4, -4); the map right to the quest page: (0 - 1, 1 - 0)
    // → (4, -4); the equipment page right to the item page: (0 - -1, -1 - 0) → (-4, 4).
    assert_eq!(page_switch_eye_d(0), (-4.0, -4.0));
    assert_eq!(page_switch_eye_d(1), (4.0, -4.0));
    assert_eq!(page_switch_eye_d(2), (4.0, -4.0));
    assert_eq!(page_switch_eye_d(6), (-4.0, 4.0));
    // sPageSwitchNextPageIndex: right goes item, map, quest, equipment.
    assert_eq!(PAGE_SWITCH_NEXT_PAGE_INDEX, [PAUSE_MAP, PAUSE_EQUIP, PAUSE_QUEST, PAUSE_ITEM, PAUSE_EQUIP, PAUSE_MAP, PAUSE_ITEM, PAUSE_QUEST]);
}

fn buttons(s: &SaveContext) -> ([u8; 3], [u8; 3]) {
    let b = s.equips.button_items;
    ([b[1], b[2], b[3]], s.equips.c_button_slots)
}

fn save_with(c: [(u8, u8); 3]) -> SaveContext {
    let mut s = SaveContext::default();
    for (i, (item, slot)) in c.into_iter().enumerate() {
        s.equips.button_items[i + 1] = item;
        s.equips.c_button_slots[i] = slot;
    }
    s
}

#[test]
fn an_item_on_another_c_button_swaps_with_the_one_equipped_on() {
    let none = (ITEM_NONE, SLOT_NONE);
    let sling = (ITEM_SLINGSHOT, SLOT_SLINGSHOT as u8);
    let nut = (ITEM_DEKU_NUT, SLOT_DEKU_NUT as u8);
    // The slingshot from C-Left onto C-Right, which has nuts: C-Left gets the nuts.
    let mut s = save_with([sling, none, nut]);
    let (item, slot) = item_equip_write(&mut s, 2, ITEM_SLINGSHOT as u16, SLOT_SLINGSHOT as u16);
    assert_eq!((item, slot), (ITEM_SLINGSHOT as u16, SLOT_SLINGSHOT as u16));
    assert_eq!(buttons(&s), ([ITEM_DEKU_NUT, ITEM_NONE, ITEM_SLINGSHOT], [SLOT_DEKU_NUT as u8, SLOT_NONE, SLOT_SLINGSHOT as u8]));
    // Onto an empty C-Right: C-Left is emptied.
    let mut s = save_with([sling, none, none]);
    item_equip_write(&mut s, 2, ITEM_SLINGSHOT as u16, SLOT_SLINGSHOT as u16);
    assert_eq!(buttons(&s), ([ITEM_NONE, ITEM_NONE, ITEM_SLINGSHOT], [SLOT_NONE, SLOT_NONE, SLOT_SLINGSHOT as u8]));
    // C-Down's equip checks C-Left first, then C-Right.
    let mut s = save_with([nut, none, sling]);
    item_equip_write(&mut s, 1, ITEM_SLINGSHOT as u16, SLOT_SLINGSHOT as u16);
    assert_eq!(buttons(&s), ([ITEM_DEKU_NUT, ITEM_SLINGSHOT, ITEM_NONE], [SLOT_DEKU_NUT as u8, SLOT_SLINGSHOT as u8, SLOT_NONE]));
}

#[test]
fn a_magic_arrow_onto_a_bow_becomes_the_bow_with_it() {
    let bow = (ITEM_BOW, SLOT_BOW as u8);
    // 0xBF (fire) onto C-Left, which has the bow: ITEM_BOW_FIRE in the bow's slot.
    let mut s = save_with([bow, (ITEM_NONE, SLOT_NONE), (ITEM_NONE, SLOT_NONE)]);
    let (item, slot) = item_equip_write(&mut s, 0, 0xBF, 4);
    assert_eq!((item, slot), (ITEM_BOW_FIRE as u16, SLOT_BOW as u16));
    assert_eq!(s.equips.button_items[1], ITEM_BOW_FIRE);
}

#[test]
fn the_bow_beside_a_bow_with_arrows_and_the_slot_only_c_left_copies() {
    let nut = (ITEM_DEKU_NUT, SLOT_DEKU_NUT as u8);
    let bow_fire = (ITEM_BOW_FIRE, SLOT_BOW as u8);
    // The bow onto C-Left with the fire bow on C-Down (which has the bow's slot: the first
    // branch swaps it): C-Down gets C-Left's nuts and their slot.
    let mut s = save_with([nut, bow_fire, (ITEM_NONE, SLOT_NONE)]);
    item_equip_write(&mut s, 0, ITEM_BOW as u16, SLOT_BOW as u16);
    assert_eq!(buttons(&s), ([ITEM_BOW, ITEM_DEKU_NUT, ITEM_NONE], [SLOT_BOW as u8, SLOT_DEKU_NUT as u8, SLOT_NONE]));
    // @bug (game): onto C-Right with the fire bow on C-Down, but in another slot than the bow's
    // (not swapped by the first branch): C-Down gets C-Right's item, its slot left as it was.
    let mut s = save_with([(ITEM_NONE, SLOT_NONE), (ITEM_BOW_FIRE, 4), nut]);
    item_equip_write(&mut s, 2, ITEM_BOW as u16, SLOT_BOW as u16);
    assert_eq!(buttons(&s), ([ITEM_NONE, ITEM_DEKU_NUT, ITEM_BOW], [SLOT_NONE, 4, SLOT_BOW as u8]));
}

#[test]
fn the_menus_bakes_are_unique_and_cover_the_pages() {
    let list = gfx::bake_list();
    let mut names: Vec<String> = list.iter().map(|&(t, c)| gfx::bake_name(t, c)).collect();
    let n = names.len();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), n, "a bake listed twice");
    // Four pages and the game over's prompt (sGameOverTexs) of 15 tiles, the 123 names, the 59
    // icons up to the bows with magic arrows.
    assert_eq!(list.iter().filter(|(t, _)| matches!(t, gfx::KTex::PageBg(_))).count(), 75);
    assert_eq!(list.iter().filter(|(t, _)| matches!(t, gfx::KTex::ItemName(_))).count(), 123);
    assert_eq!(list.iter().filter(|(t, _)| matches!(t, gfx::KTex::ItemIcon(_))).count(), 0x3B);
}

#[test]
fn override_pal_index_ci4_moves_one_index_in_both_nibbles() {
    use super::scope::override_pal_index_ci4;
    // KaleidoScope_OverridePalIndexCI4: each byte's two texels, (b >> 4) & 0xF and b & 0xF, the
    // target's become the new index; the indices are masked to 4 bits first.
    let mut t = vec![0xA1, 0x1A, 0xAA, 0x00, 0xEA];
    override_pal_index_ci4(Some(&mut t), 5, 10, 14);
    assert_eq!(t, vec![0xE1, 0x1E, 0xEE, 0x00, 0xEE]);
    let mut t = vec![0xA1, 0x1A];
    override_pal_index_ci4(Some(&mut t), 2, 10 + 16, 14 + 32);
    assert_eq!(t, vec![0xE1, 0x1E]);
    // Only `size` bytes; nothing when the indices are the same or the size is 0.
    let mut t = vec![0xAA, 0xAA];
    override_pal_index_ci4(Some(&mut t), 1, 10, 14);
    assert_eq!(t, vec![0xEE, 0xAA]);
    override_pal_index_ci4(Some(&mut t), 2, 10, 10);
    override_pal_index_ci4(Some(&mut t), 0, 10, 14);
    assert_eq!(t, vec![0xEE, 0xAA]);
    override_pal_index_ci4(None, 2, 10, 14);
}

#[test]
fn the_game_overs_message_bakes_two_textures_and_scrolls_tile_1() {
    use crate::pack::BakeSegment;
    let bakes = gfx::game_over_bakes();
    assert_eq!(bakes.iter().map(|b| b.name.as_str()).collect::<Vec<_>>(), vec!["kaleido/game_over/P1", "kaleido/game_over/P2", "kaleido/game_over/P3"]);
    for b in &bakes {
        // gDPSetPrimColor(0, 80, ...): dynamic, its LOD fraction baked; env dynamic.
        let colour = b.segments.iter().find(|(s, _)| *s == crate::sprite::SEG_COLOR).map(|(_, s)| s.clone());
        assert_eq!(colour, Some(BakeSegment::Dynamic(vec![(0xFA00_0050, 0xFF), (0xFB00_0000, 0xFF), (0xDF00_0000, 0)])));
        // gDPSetTileSize(1, 0, ult, 63 << 2, (31 << 2) + ult): the first frame's ult 0, dynamic.
        let tile = b.segments.iter().find(|(s, _)| *s == crate::sprite::SEG_TILE).map(|(_, s)| s.clone());
        assert_eq!(tile, Some(BakeSegment::Dynamic(vec![(0xF200_0000, 0x0100_0000 | ((63 << 2) << 12) | (31 << 2)), (0xDF00_0000, 0)])));
        // The mask, gGameOverMaskTex, in its own segment.
        assert!(b.segments.iter().any(|(_, s)| *s == BakeSegment::Texture { file: "icon_item_gameover_static".into(), symbol: "gGameOverMaskTex".into() }));
    }
}
