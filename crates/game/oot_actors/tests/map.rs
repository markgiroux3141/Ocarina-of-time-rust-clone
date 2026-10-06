//! The map and the compass as data (`z_map_exp.c`, `oot_game::map`), against the C: what
//! `Map_Init` sets entering the Deku Tree and Kokiri Forest, the rooms visited as
//! `Room_FinishRoomChange` marks them, the floor `Map_Update` finds from Link's height (and its
//! floor switches), and the map's and the compass's chests (`En_Box` params 0x0823 in room 0:
//! `GI_DUNGEON_MAP`, treasure flag 3; 0x0801 in room 2: `GI_COMPASS`, flag 1) giving them through
//! the get-item flow.
//!
//! Expected values come from `z_map_data.c`'s tables for the Deku Tree (`mapIndex` 0):
//! - `floorCoordY[0]` = { 9999, 9999, 9999, 760, 360, -40, -1000, -2000 };
//! - `floorTexIndexOffset[0]` = { 0, 0, 0, 0, 2, 4, 6, 8 }, `floorTexIndexOffset[1][0]` = 0;
//! - `dgnCompassInfo[0]` = { 3, 3, 1070, -690 }, `dgnTexIndexBase[0]` = 0;
//! - `roomCompassOffsetX[0]` = { 1090, 1390, 1560, ..., 1110 (11), 1040 (12) }, `...Y[0]` =
//!   { -660, -570, -410, ..., -630 (11), -660 (12) };
//! - `roomPalette[0]` = { 10, 1, 2, ... }; `paletteRoom[0][4]` = { 0, 1, 2, 0xFF... },
//!   `paletteRoom[0][5]` = { 0, 0xFF... }, floors 0..2 all 0xFF;
//! - the switches (`switchFromRoom`, `switchFromFloor`, `switchToRoom`): (11, 3) → 12,
//!   (0, 4) → 11, (0, 3) → 12, (12, 4) → 11, (11, 5) → 0.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use eng_math::{cos_s, sin_s};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_box::EnBox;
use oot_actors::player::Action;
use oot_game::actor_ctx::ActorHandle;
use oot_game::map::{DUNGEON_COMPASS, DUNGEON_MAP, MapSegment, check_dungeon_item};
use oot_game::message::MSGMODE_NONE;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

const NONE: PadState = PadState { button: 0, stick_x: 0, stick_y: 0 };
const A: PadState = PadState { button: BTN_A, stick_x: 0, stick_y: 0 };

fn frames(w: &mut PlayState, prev: &mut PadState, pad: PadState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(*prev, pad));
        *prev = pad;
    }
}

/// `Play_Init` on `entrance` with a new save (and `preset`).
fn enter(a: &Arc<GameAssets>, entrance: &str, preset: Option<&str>) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    if let Some(p) = preset {
        save.apply_preset(p).unwrap();
    }
    PlayState::play_init(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init")
}

/// `Map_SavePlayerInitialInfo`'s values for Link at `pos` facing `yaw`.
fn initial_info(pos: Vec3, yaw: i16) -> (i16, i16, i16) {
    (pos.x as i16, pos.z as i16, ((0x7FFF - yaw as i32) / 0x400) as i16)
}

fn saved_initial_info(w: &PlayState) -> (i16, i16, i16) {
    (w.map.player_initial_pos_x, w.map.player_initial_pos_z, w.map.player_initial_direction)
}

#[test]
fn entering_the_deku_tree_marks_room_0() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_DEKU_TREE_0", Some("deku-tree-inside"));
    // Map_Init (Interface_Init): SCENE_DEKU_TREE is mapIndex 0; dgnCompassInfo[0]'s scale, and
    // R_MAP_TEX_INDEX = R_MAP_TEX_INDEX_BASE = dgnTexIndexBase[0]; interfaceCtx.unk_258 -1.
    let m = &w.map;
    assert!(m.loaded);
    assert_eq!((w.save.map_index, m.r_map_index, m.unk_258), (0, 0, -1));
    assert_eq!((m.r_compass_scale_x, m.r_compass_scale_y), (3, 3));
    assert_eq!((m.r_map_tex_index_base, m.r_map_tex_index), (0, 0));
    // Map_InitRoomData(curRoom.num 0): room 0 visited, its minimap (map_i_static's 0 + 0) and
    // compass offset (roomCompassOffsetX/Y[0][0], over dgnCompassInfo's 1070, -690), unk_25A 0.
    assert_eq!(w.save.scene_flags[0].rooms, 1);
    assert_eq!((m.map_room_num, m.unk_25a), (0, 0));
    assert_eq!(m.map_segment, Some(MapSegment::Dungeon { index: 0 }));
    assert_eq!((m.r_compass_offset_x, m.r_compass_offset_y), (1090, -660));
    // Map_SetPaletteData(0): mapPaletteIndex = roomPalette[0][0] = 10 (and palette entries 20, 21
    // set), then Map_InitData's Map_SetFloorPalettesData(VREG(30) = 0, Regs_InitData's) clears the
    // palette, and floor 0 has no rooms.
    assert_eq!(m.map_palette_index, 10);
    assert_eq!(m.map_palette, [0; 32]);
    // The end of Player_Init: Map_SavePlayerInitialInfo at the spawn.
    assert_eq!(saved_initial_info(&w), initial_info(w.spawn.0, w.spawn.1));
    assert_eq!(saved_initial_info(&w), (-4, 603, 63), "(0x7FFF + 0x8000) / 0x400");
    assert_eq!(w.save.scene_flags[0].floors, 0);

    // The first frame's Map_Update (Interface_Update): no map, so mapPalette[31] 0; Link at y 0
    // is above floorCoordY[0][5] (-40): floor 5, visited; R_MAP_TEX_INDEX = 0 +
    // floorTexIndexOffset[0][5] (4). No switch is from room 0 on floor 5.
    let mut prev = NONE;
    frames(&mut w, &mut prev, NONE, 1);
    let m = &w.map;
    assert_eq!((m.floor, m.r_map_tex_index), (5, 4));
    assert_eq!(w.save.scene_flags[0].floors, 1 << 5);
    assert_eq!((m.map_palette[30], m.map_palette[31]), (0, 0));
    assert_eq!((m.map_room_num, m.vreg_10), (0, 0));
    assert!(!check_dungeon_item(&w.save, DUNGEON_MAP, 0) && !check_dungeon_item(&w.save, DUNGEON_COMPASS, 0));
}

#[test]
fn a_room_change_marks_the_new_room_when_it_finishes() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_DEKU_TREE_0", Some("deku-tree-inside"));
    let mut prev = NONE;
    frames(&mut w, &mut prev, NONE, 20);
    let floor = w.map.floor;
    assert_eq!(floor, 5);
    // Room_RequestNewRoom and the load (Room_ProcessRoomRequest) mark nothing...
    assert!(w.room_request(1));
    frames(&mut w, &mut prev, NONE, 1);
    assert_eq!(w.save.scene_flags[0].rooms, 1);
    assert_eq!(w.map.map_room_num, 0);
    // ...Room_FinishRoomChange does: Map_InitRoomData(1) sets bit 1; mapRoomNum 1;
    // Map_SetPaletteData(1): mapPaletteIndex = roomPalette[0][1] = 1; Map_InitData(1): minimap 1,
    // roomCompassOffsetX/Y[0][1]; Map_SetFloorPalettesData(VREG(30) = 5) clears the palette and
    // sets paletteRoom[0][5]'s visited room 0's colour (roomPalette[0][0] = 10: entries 20, 21).
    w.room_change_done();
    let m = &w.map;
    assert_eq!(w.save.scene_flags[0].rooms, 0b11);
    assert_eq!(m.map_room_num, 1);
    assert_eq!(m.map_palette_index, 1);
    assert_eq!(m.map_segment, Some(MapSegment::Dungeon { index: 1 }));
    assert_eq!((m.r_compass_offset_x, m.r_compass_offset_y), (1390, -570));
    let mut want = [0u8; 32];
    (want[20], want[21]) = (2, 0xBF);
    assert_eq!(m.map_palette, want);
    // Not an overworld scene: Map_SavePlayerInitialInfo, where Link is now.
    let p = w.player().actor.clone();
    assert_eq!(saved_initial_info(&w), initial_info(p.world_pos, p.shape_rot.y));
    // A room already visited changes nothing in the flags.
    assert!(w.room_request(0));
    frames(&mut w, &mut prev, NONE, 1);
    w.room_change_done();
    assert_eq!(w.save.scene_flags[0].rooms, 0b11);
    assert_eq!((w.map.map_room_num, w.map.map_palette_index), (0, 10));
}

#[test]
fn the_floor_from_links_height_and_the_floor_switches() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_DEKU_TREE_0", Some("deku-tree-inside"));
    // Map_Update on Link's height (place him, then run it as Interface_Update would): the first
    // floor whose floorCoordY[0] he's strictly above.
    let at = |w: &mut PlayState, y: f32| {
        w.place_player(Vec3::new(100.0, y, 200.0), 0x4000);
        w.map_update();
        (w.map.floor, w.map.r_map_tex_index, w.map.map_room_num)
    };
    // Floor 5 (above -40), texture 4.
    assert_eq!(at(&mut w, 0.0), (5, 4, 0));
    // Exactly 360 isn't above floor 4's 360 (the map chest's ledge is at 360).
    assert_eq!(at(&mut w, 360.0), (5, 4, 0));
    // Floor 4, texture 2; room 0 on floor 4 switches to room 11 (switch 1): Map_InitData(11)
    // (minimap 11, roomCompassOffsetX/Y[0][11]), its floor palettes for VREG(30) = 4 (room 0
    // visited: roomPalette[0][0] = 10), and Map_SavePlayerInitialInfo.
    assert_eq!(at(&mut w, 360.5), (4, 2, 11));
    let m = &w.map;
    assert_eq!(m.map_segment, Some(MapSegment::Dungeon { index: 11 }));
    assert_eq!((m.r_compass_offset_x, m.r_compass_offset_y), (1110, -630));
    assert_eq!((m.map_palette[20], m.map_palette[21], m.map_palette_index), (2, 0xBF, 10));
    assert_eq!(m.vreg_10, 11);
    assert_eq!(saved_initial_info(&w), initial_info(Vec3::new(100.0, 360.5, 200.0), 0x4000));
    // Floor 3 (above 760), texture 0: room 11 on floor 3 switches to 12 (switch 0); the later
    // switches don't match room 12 on floor 3.
    assert_eq!(at(&mut w, 800.0), (3, 0, 12));
    assert_eq!((w.map.r_compass_offset_x, w.map.r_compass_offset_y), (1040, -660));
    // Floor 4 again: room 12 → 11 (switch 3); floor 5: room 11 → 0 (switch 4).
    assert_eq!(at(&mut w, 500.0), (4, 2, 11));
    assert_eq!(at(&mut w, 100.0), (5, 4, 0));
    // Floor 7 (above -2000), texture 8; nothing switches from room 0 there.
    assert_eq!(at(&mut w, -1500.0), (7, 8, 0));
    // @bug (game): below every floor the loop ends at 8: floors' bit 8, and
    // floorTexIndexOffset[0][8], the next dungeon's floor 0 (0).
    assert_eq!(at(&mut w, -2500.0), (8, 0, 0));
    assert_eq!(w.save.scene_flags[0].floors, (1 << 3) | (1 << 4) | (1 << 5) | (1 << 7) | (1 << 8));
    // Paused, Map_Update does nothing.
    w.pause_ctx.state = 1;
    assert_eq!(at(&mut w, 0.0), (8, 0, 0));
}

#[test]
fn kokiri_forest_gets_its_minimap_and_no_rooms() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_KOKIRI_FOREST_0", None);
    // Map_Init: SCENE_KOKIRI_FOREST - SCENE_HYRULE_FIELD = 4; owCompassInfo[4] = { 8, 8, 660, -730 };
    // Map_InitData(4): no special case, so sEntranceIconMapIndex 4 and map_grand_static's
    // minimap at owMinimapTexOffset[4] (0x2660), owMinimapTexSize[4] (2976) bytes; unk_258 4;
    // R_OW_MINIMAP_X/Y = owMinimapPosX/Y[4] (202, 160). No room data: unk_25A stays -1.
    let m = &w.map;
    assert_eq!((w.save.map_index, m.r_map_index, m.unk_258, m.unk_25a), (4, 4, 4, -1));
    assert_eq!((m.r_compass_scale_x, m.r_compass_scale_y, m.r_compass_offset_x, m.r_compass_offset_y), (8, 8, 660, -730));
    assert_eq!(m.entrance_icon_map_index, 4);
    assert_eq!(m.map_segment, Some(MapSegment::Overworld { offset: 0x2660, size: 2976 }));
    assert_eq!((m.r_ow_minimap_x, m.r_ow_minimap_y), (202, 160));
    let initial = initial_info(w.spawn.0, w.spawn.1);
    assert_eq!(saved_initial_info(&w), initial);
    // Map_Update has no overworld case; a room change in an overworld scene marks no room
    // (Map_InitRoomData's switch has no case for it) and, in SCENE_HYRULE_FIELD ..
    // SCENE_LON_LON_RANCH, doesn't save Link's position.
    let mut prev = NONE;
    frames(&mut w, &mut prev, NONE, 20);
    assert!(w.room_request(2));
    frames(&mut w, &mut prev, NONE, 1);
    w.room_change_done();
    assert_eq!(saved_initial_info(&w), initial);
    assert!(w.save.scene_flags.iter().all(|f| f.rooms == 0 && f.floors == 0));
    assert_eq!((w.map.floor, w.map.map_room_num, w.map.r_map_tex_index), (0, 0, 0));
}

/// Opens the chest `c` (at `pos`, facing `yaw`) from in front, through the get-item flow, to
/// Link standing again.
fn open_chest(w: &mut PlayState, prev: &mut PadState, c: ActorHandle, pos: Vec3, yaw: i16, gi: i16) {
    let front = Vec3::new(sin_s(yaw), 0.0, cos_s(yaw));
    w.place_player(pos - front * 30.0, yaw);
    frames(w, prev, NONE, 2);
    // EnBox_WaitOpen: Actor_OfferGetItemNearby(-(params >> 5 & 0x7F)).
    assert_eq!(w.player().interact_range_actor, Some(c));
    assert_eq!(w.player().get_item_id, -gi);
    frames(w, prev, A, 1);
    for i in 0..900 {
        if i > 20 && w.msg_ctx.msg_mode == MSGMODE_NONE && w.player().action == Action::StandingStill && w.player().get_item_id == 0 {
            return;
        }
        frames(w, prev, if i % 10 == 0 { A } else { NONE }, 1);
    }
    panic!("the chest's get-item didn't end: {:?}", w.player().action);
}

fn chest(w: &PlayState, params: i16) -> Option<(ActorHandle, Vec3, i16)> {
    w.actors.all().into_iter().find_map(|h| w.actors.downcast::<EnBox>(h).filter(|b| b.actor.params == params).map(|b| (h, b.actor.home_pos, b.actor.shape_rot.y)))
}

/// `GI_COMPASS`, `GI_DUNGEON_MAP` (`GetItemID`, `item.h`).
const GI_COMPASS: i16 = 0x40;
const GI_DUNGEON_MAP: i16 = 0x41;

#[test]
fn the_map_and_compass_chests_give_them() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_DEKU_TREE_0", Some("deku-tree-inside"));
    let mut prev = NONE;
    frames(&mut w, &mut prev, NONE, 20);
    // sGetItemTable: GET_ITEM(ITEM_DUNGEON_MAP, OBJECT_GI_MAP, GID_DUNGEON_MAP, 0x66, 0x80,
    // CHEST_ANIM_LONG), GET_ITEM(ITEM_DUNGEON_COMPASS, OBJECT_GI_COMPASS, GID_COMPASS, 0x67, ...).
    assert_eq!(a.items.get_item(GI_DUNGEON_MAP).unwrap().item_id, oot_game::item::ITEM_DUNGEON_MAP);
    assert_eq!(a.items.get_item(GI_COMPASS).unwrap().item_id, oot_game::item::ITEM_DUNGEON_COMPASS);

    // Room 0's chest, 0x0823: ENBOX_TYPE_BIG, GI_DUNGEON_MAP (0x0823 >> 5 & 0x7F), flag 3.
    let (c, pos, yaw) = chest(&w, 0x0823).expect("room 0's map chest");
    open_chest(&mut w, &mut prev, c, pos, yaw, GI_DUNGEON_MAP);
    // Item_Give(ITEM_DUNGEON_MAP): dungeonItems[mapIndex 0] |= gBitFlags[ITEM_DUNGEON_MAP -
    // ITEM_DUNGEON_BOSS_KEY] (DUNGEON_MAP, 2); En_Box's Flags_SetTreasure(3).
    assert_eq!(w.save.inventory.dungeon_items[0], 1 << DUNGEON_MAP);
    assert!(check_dungeon_item(&w.save, DUNGEON_MAP, 0));
    assert!(w.flags.get_treasure(3));
    // Map_Update with the map: mapPalette[31] = 1. A room change's Map_SetFloorPalettesData
    // sets 30, 31 to 0, 1 too.
    assert_eq!((w.map.map_palette[30], w.map.map_palette[31]), (0, 1));
    assert!(w.room_request(2));
    frames(&mut w, &mut prev, NONE, 1);
    w.room_change_done();
    assert_eq!((w.map.map_palette[30], w.map.map_palette[31]), (0, 1));
    assert_eq!(w.save.scene_flags[0].rooms, 0b101);

    // Room 2's chest, 0x0801: GI_COMPASS (0x0801 >> 5 & 0x7F), flag 1.
    for _ in 0..20 {
        if chest(&w, 0x0801).is_some() {
            break;
        }
        frames(&mut w, &mut prev, NONE, 1);
    }
    let (c, pos, yaw) = chest(&w, 0x0801).expect("room 2's compass chest");
    open_chest(&mut w, &mut prev, c, pos, yaw, GI_COMPASS);
    assert_eq!(w.save.inventory.dungeon_items[0], (1 << DUNGEON_MAP) | (1 << DUNGEON_COMPASS));
    assert!(check_dungeon_item(&w.save, DUNGEON_COMPASS, 0));
    assert!(w.flags.get_treasure(1));
}
