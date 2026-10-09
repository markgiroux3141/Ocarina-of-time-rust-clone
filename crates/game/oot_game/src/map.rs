//! The map and the compass (`z_map_exp.c`, with `z_map_data.c`'s and `z_map_mark.c`'s data): what
//! they record as Link goes through a dungeon, without the draws.
//!
//! - **The tables** (`table/map`, read from the C by the importer): `gMapDataTable`
//!   (`z_map_data.c`: the floors' heights, the rooms' palettes and compass offsets, the
//!   floor-to-room switches, the overworld minimaps) and `gMapMarkDataTable`
//!   (`ovl_map_mark_data`'s `z_map_mark_data_mq.c`: the chests' and the boss's marks per room's
//!   minimap); for the pause menu's map page, `gPauseMapMarkDataTable` (`ovl_kaleido_scope`'s
//!   `z_lmap_mark_data_mq.c`: the marks per floor's map) and `map_48x85_static`'s bytes (the
//!   floors' room maps, which the menu copies into `mapSegment` and recolours: `crate::kaleido`).
//! - **The state** (`MapState`): what `z_map_exp.c` keeps in `interfaceCtx` (`mapRoomNum`,
//!   `mapPalette`, `mapPaletteIndex`, which minimap texture `mapSegment` holds), the REGs it sets
//!   (`R_MAP_INDEX`, `R_MAP_TEX_INDEX`, the compass's scale and offset, `VREG(30)`'s floor) and
//!   its statics (`sPlayerInitialPos*`). The save holds `mapIndex` and each dungeon's visited
//!   rooms and floors (`sceneFlags[mapIndex].rooms`, `.floors`).
//! - **The calls**, where the C makes them: `Map_Init` in `Interface_Init` (`Play_Init`, before
//!   Player), `Map_SavePlayerInitialInfo` at the end of `Player_Init`, `Map_Update` in
//!   `Interface_Update`, `Map_InitRoomData` and `Map_SavePlayerInitialInfo` in
//!   `Room_FinishRoomChange`, `Map_Destroy` in `Interface_Destroy` (`Play_Destroy`).
//!
//! **Not ported:** the draws, milestone 5's (`Minimap_Draw` in `Interface_Draw`,
//! `Minimap_DrawCompassIcons`, `MapMark_Draw`, and the pause menu's map in `z_kaleido_map.c`);
//! the Sun's Song (`sunsSongState`, which `Map_InitRoomData` and `Map_Update` reset);
//! `Message_LoadItemIcon`'s `mapPalette[30]`, `[31]` = -1 for `ITEM_DUNGEON_MAP` (in the ten
//! dungeons `Map_Update` writes both again the same frame); the debug prints.
//!
//! A play state starts with `Regs_InitData`'s REGs and the statics' initial values. The C keeps
//! the REGs and the statics `Regs_InitData` doesn't reset (`R_MAP_TEX_INDEX`, `R_OW_MINIMAP_X`,
//! `sPlayerInitialPosX`...) from the last play state, but they're only read in the scenes whose
//! `Map_Init` sets them, so a fresh state is the same.

use crate::play::PlayState;
use crate::save::SaveContext;

/// `MapData`'s arrays' row sizes (`map.h`, `z_map_data.c`): a dungeon's 8 floors, 32 rooms'
/// palettes, 14 palettes a floor, 44 rooms' compass offsets, 51 floor switches.
pub const MAP_FLOORS: usize = 8;
pub const MAP_ROOM_PALETTES: usize = 32;
pub const MAP_FLOOR_PALETTES: usize = 14;
pub const MAP_COMPASS_ROOMS: usize = 44;
pub const MAP_SWITCHES: usize = 51;

/// `MapData` (`map.h`), `gMapDataTable` as `z_map_data.c` initialises it. Each array is flat in
/// the C's row order (`floorTexIndexOffset[10][8]` is 80 values), its rows zero-filled as C
/// fills them, so an index past a row reads the next row as the game does.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MapData {
    /// `floorTexIndexOffset[10][8]`: each floor's pause map texture, from the dungeon's base.
    pub floor_tex_index_offset: Vec<i16>,
    /// `bossFloor[8]`: the boss room's floor.
    pub boss_floor: Vec<i16>,
    /// `roomPalette[10][32]`: each room's palette on the pause map.
    pub room_palette: Vec<i16>,
    /// `maxPaletteCount[10]`: the most palettes on one floor.
    pub max_palette_count: Vec<i16>,
    /// `paletteRoom[10][8][14]`: each floor's palettes' rooms (0xFF none).
    pub palette_room: Vec<i16>,
    /// `roomCompassOffsetX[10][44]`, `roomCompassOffsetY[10][44]`: the compass icons' offset by room.
    pub room_compass_offset_x: Vec<i16>,
    pub room_compass_offset_y: Vec<i16>,
    /// `dgnMinimapCount[12]`: the rooms' minimaps (`MapMark_DrawForDungeon`'s bound).
    pub dgn_minimap_count: Vec<u8>,
    /// `dgnMinimapTexIndexOffset[10]`: the dungeon's first minimap in `map_i_static`.
    pub dgn_minimap_tex_index_offset: Vec<u16>,
    /// `owMinimapTexSize[24]`, `owMinimapTexOffset[24]`: the overworld minimaps in `map_grand_static`.
    pub ow_minimap_tex_size: Vec<u16>,
    pub ow_minimap_tex_offset: Vec<u16>,
    /// `owMinimapPosX[24]`, `owMinimapPosY[24]`.
    pub ow_minimap_pos_x: Vec<i16>,
    pub ow_minimap_pos_y: Vec<i16>,
    /// `owCompassInfo[24][4]`: X scale, Y scale, X offset, Y offset.
    pub ow_compass_info: Vec<i16>,
    /// `dgnTexIndexBase[10]`: the dungeon's first pause map texture.
    pub dgn_tex_index_base: Vec<i16>,
    /// `dgnCompassInfo[10][4]`.
    pub dgn_compass_info: Vec<i16>,
    /// `owMinimapWidth[24]`, `owMinimapHeight[24]`.
    pub ow_minimap_width: Vec<i16>,
    pub ow_minimap_height: Vec<i16>,
    /// `owEntranceIconPosX[24]`, `owEntranceIconPosY[24]`: the dungeon entrance icon.
    pub ow_entrance_icon_pos_x: Vec<i16>,
    pub ow_entrance_icon_pos_y: Vec<i16>,
    /// `owEntranceFlag[20]`: the `infTable[INFTABLE_INDEX_1AX]` bit that shows it (0xFFFF always).
    pub ow_entrance_flag: Vec<u16>,
    /// `floorCoordY[10][8]`: Link is on the first floor whose height he's above.
    pub floor_coord_y: Vec<f32>,
    /// `switchEntryCount[10]`, and `switchFromRoom`, `switchFromFloor`, `switchToRoom`
    /// (`[10][51]`): a room that spans floors shows another room's minimap on another floor.
    pub switch_entry_count: Vec<u16>,
    pub switch_from_room: Vec<u8>,
    pub switch_from_floor: Vec<u8>,
    pub switch_to_room: Vec<u8>,
    /// `floorID[10][8]` (`FloorID`: `F_1F` 8, `F_B1` 9...; 0 none).
    pub floor_id: Vec<u8>,
    /// `skullFloorIconY[10]`: the pause map's boss skull, -99 for none.
    pub skull_floor_icon_y: Vec<i16>,
}

/// `MAP_MARK_NONE`, `MAP_MARK_CHEST`, `MAP_MARK_BOSS` (`map_mark.h`).
pub const MAP_MARK_NONE: i8 = -1;
pub const MAP_MARK_CHEST: i8 = 0;
pub const MAP_MARK_BOSS: i8 = 1;

/// `MapMarkPoint` (`map_mark.h`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MapMarkPoint {
    /// `chestFlag`: a chest's icon shows until this treasure flag is set.
    pub chest_flag: i8,
    /// `x`, `y`: the icon's top left, from the minimap's.
    pub x: u8,
    pub y: u8,
}

/// `MapMarkIconData` (`map_mark.h`): `count` icons of one kind; the list ends at
/// `MAP_MARK_NONE`.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MapMarkIconData {
    pub mark_type: i8,
    pub count: u8,
    /// `points[12]`, zero-filled as the C fills them.
    pub points: Vec<MapMarkPoint>,
}

/// `PAUSE_MAP_MARK_NONE`, `PAUSE_MAP_MARK_CHEST`, `PAUSE_MAP_MARK_BOSS` (`pause.h`).
pub const PAUSE_MAP_MARK_NONE: i16 = -1;
pub const PAUSE_MAP_MARK_CHEST: i16 = 0;
pub const PAUSE_MAP_MARK_BOSS: i16 = 1;

/// `PauseMapMarkPoint` (`pause.h`).
#[derive(Debug, Clone, Copy, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PauseMapMarkPoint {
    /// `chestFlag`: a chest's mark shows until this treasure flag is set (-1 none).
    pub chest_flag: i16,
    /// `x`, `y`: where the mark goes on the map.
    pub x: f32,
    pub y: f32,
}

/// `PauseMapMarkData` (`pause.h`): `count` marks of one kind, drawn with `vtx`; a floor's list
/// ends at `PAUSE_MAP_MARK_NONE`.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PauseMapMarkData {
    pub mark_type: i16,
    /// `unk_04` (23 in every entry; nothing reads it).
    pub unk_04: i32,
    /// The `Vtx` array `vtx` points at (`sMarkChestVtx`, `sMarkBossVtx`; none for a list's end).
    pub vtx: Vec<crate::kaleido::gfx::Vtx>,
    pub vtx_count: i32,
    pub count: i32,
    /// `points[12]`, zero-filled as the C fills them.
    pub points: Vec<PauseMapMarkPoint>,
}

/// `map_48x85_static`'s textures: `MAP_48x85_TEX_WIDTH`, `_HEIGHT`, `MAP_48x85_TEX_SIZE`
/// (`map.h`: a 48x85 CI4).
pub const MAP_48X85_TEX_WIDTH: u32 = 48;
pub const MAP_48X85_TEX_HEIGHT: u32 = 85;
pub const MAP_48X85_TEX_SIZE: usize = (MAP_48X85_TEX_WIDTH * MAP_48X85_TEX_HEIGHT / 2) as usize;

/// `table/map`: `gMapDataTable`, `gMapMarkDataTable` (by dungeon, then by the room's minimap:
/// its three `MapMarkIconData`, `MapMarkData`), and the pause map's `gPauseMapMarkDataTable` (by
/// floor's map, `R_MAP_TEX_INDEX >> 1`: its three `PauseMapMarkData`) and `map_48x85_static`.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MapTables {
    pub data: MapData,
    pub marks: Vec<Vec<[MapMarkIconData; 3]>>,
    pub pause_marks: Vec<[PauseMapMarkData; 3]>,
    /// `map_48x85_static` as it is in the ROM (`MAP_48X85_TEX_SIZE` a map).
    pub map_48x85_static: Vec<u8>,
}

/// `DUNGEON_BOSS_KEY`, `DUNGEON_COMPASS`, `DUNGEON_MAP` (`DungeonItem`, `item.h`).
pub const DUNGEON_BOSS_KEY: u32 = 0;
pub const DUNGEON_COMPASS: u32 = 1;
pub const DUNGEON_MAP: u32 = 2;

/// `CHECK_DUNGEON_ITEM(item, dungeonIndex)` (`save.h`): `dungeonItems[dungeonIndex] &
/// gBitFlags[item]`.
pub fn check_dungeon_item(save: &SaveContext, item: u32, dungeon_index: u16) -> bool {
    save.inventory.dungeon_items.get(dungeon_index as usize).is_some_and(|&d| d as u32 & (1 << item) != 0)
}

/// Which texture `interfaceCtx.mapSegment` holds (`Map_InitData`'s DMA), for the draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapSegment {
    /// `map_grand_static` from `owMinimapTexOffset[extendedMapIndex]`, `owMinimapTexSize[mapIndex]`
    /// bytes: an overworld minimap (IA4, `owMinimapWidth` by `owMinimapHeight`).
    Overworld { offset: u32, size: u32 },
    /// `map_i_static`'s texture `dgnMinimapTexIndexOffset[mapIndex] + room` (`MAP_I_TEX_SIZE`
    /// each, a 96x85 I4): a dungeon room's minimap.
    Dungeon { index: u32 },
    /// `map_48x85_static`'s textures `index` and `index + 1` (at 0 and
    /// `ALIGN16(MAP_48x85_TEX_SIZE)`): the pause map's floor (`KaleidoScope_LoadDungeonMap`),
    /// their bytes in `MapState::segment`.
    PauseMap { index: u32 },
}

// The scenes z_map_exp.c's switches name, by their ids (include/tables/scene_table.h).
/// `SCENE_DEKU_TREE` (0x00) .. `SCENE_ICE_CAVERN` (0x09): the ten dungeons with maps.
const SCENE_ICE_CAVERN: u16 = 0x09;
/// `SCENE_DEKU_TREE_BOSS` (0x11) .. `SCENE_SHADOW_TEMPLE_BOSS` (0x18).
const SCENE_DEKU_TREE_BOSS: u16 = 0x11;
const SCENE_SHADOW_TEMPLE_BOSS: u16 = 0x18;
/// `SCENE_HYRULE_FIELD` (0x51) .. `SCENE_OUTSIDE_GANONS_CASTLE` (0x64): the overworld with
/// minimaps, every scene between them.
const SCENE_HYRULE_FIELD: u16 = 0x51;
const SCENE_GRAVEYARD: u16 = 0x53;
const SCENE_LAKE_HYLIA: u16 = 0x57;
const SCENE_GERUDO_VALLEY: u16 = 0x5A;
const SCENE_GERUDOS_FORTRESS: u16 = 0x5D;
const SCENE_LON_LON_RANCH: u16 = 0x63;
const SCENE_OUTSIDE_GANONS_CASTLE: u16 = 0x64;

/// `QUEST_MEDALLION_WATER`, `QUEST_SONG_NOCTURNE` (`QuestItem`, `item.h`).
const QUEST_MEDALLION_WATER: u32 = 0x02;
const QUEST_SONG_NOCTURNE: u32 = 0x0A;
/// `EVENTCHKINF_INDEX_CARPENTERS_RESCUED` and `EVENTCHKINF_CARPENTERS_ALL_RESCUED_MASK`
/// (`save.h`: `EVENTCHKINF_CARPENTER_0_RESCUED` 0x90 .. `_3_` 0x93).
const EVENTCHKINF_INDEX_CARPENTERS_RESCUED: usize = 0x9;
const EVENTCHKINF_CARPENTERS_ALL_RESCUED_MASK: u16 = 0xF;

fn is_overworld(scene_id: u16) -> bool {
    (SCENE_HYRULE_FIELD..=SCENE_OUTSIDE_GANONS_CASTLE).contains(&scene_id)
}

/// The ten dungeons and their boss rooms (`Map_InitData`'s, `Map_InitRoomData`'s and
/// `Map_SetFloorPalettesData`'s cases, and the pause menu's `sInDungeonScene`).
pub(crate) fn is_dungeon_or_boss(scene_id: u16) -> bool {
    scene_id <= SCENE_ICE_CAVERN || (SCENE_DEKU_TREE_BOSS..=SCENE_SHADOW_TEMPLE_BOSS).contains(&scene_id)
}

impl MapData {
    /// `Map_GetFloorTextIndexOffset`: `floorTexIndexOffset[mapIndex][floor]`.
    ///
    /// @bug (game): `Map_Update` asks for floor 8 when Link is below the lowest floor's height,
    /// which reads the next dungeon's floor 0 (0), and past the last dungeon `sBossFloor[0]`,
    /// the array after it in `z_map_data.c`'s data.
    pub fn floor_tex_index_offset(&self, map_index: usize, floor: usize) -> i16 {
        let i = map_index * MAP_FLOORS + floor;
        let n = self.floor_tex_index_offset.len();
        if i < n { self.floor_tex_index_offset[i] } else { self.boss_floor.get(i - n).copied().unwrap_or(0) }
    }

    /// `paletteRoom[mapIndex][floor][i]`; floor 8 reads the next dungeon's floor 0 as the game
    /// does, and past the table no room (the game would read `sRoomCompassOffsetX`: only the
    /// Ice Cavern below its lowest floor gets there).
    fn palette_room(&self, map_index: usize, floor: usize, i: usize) -> i16 {
        self.palette_room.get((map_index * MAP_FLOORS + floor) * MAP_FLOOR_PALETTES + i).copied().unwrap_or(0xFF)
    }
}

/// `z_map_exp.c`'s state: `interfaceCtx`'s map fields, the REGs it sets, its statics, and
/// whether `gMapData` is set.
#[derive(Debug, Clone, PartialEq)]
pub struct MapState {
    /// `gMapData != NULL`: `Map_Init` ran, `Map_Destroy` hasn't.
    pub loaded: bool,
    /// `interfaceCtx.mapSegment`'s texture.
    pub map_segment: Option<MapSegment>,
    /// `mapSegment`'s bytes where the port reads them: the pause map's two room maps, which the
    /// menu recolours in place (`KaleidoScope_OverridePalIndexCI4`). The minimaps aren't drawn,
    /// so their loads only set `map_segment`.
    pub segment: Vec<u8>,
    /// `interfaceCtx.mapPalette[32]`: the pause map's 16-colour palette (two bytes a colour): the
    /// visited rooms' colours (2, 0xBF), and in 30..31 the map's own (0, 1 with the map).
    pub map_palette: [u8; 32],
    /// `interfaceCtx.unk_258`: the overworld's `mapIndex` (-1 elsewhere); `unk_25A`: the
    /// dungeon's (the pause map's floor icons read it).
    pub unk_258: i16,
    pub unk_25a: i16,
    /// `interfaceCtx.mapRoomNum`: the room whose minimap shows (a switch can show another
    /// room's on another floor).
    pub map_room_num: i16,
    /// `interfaceCtx.mapPaletteIndex` (`map_palete_no`): the current room's palette.
    pub map_palette_index: i16,
    /// `R_MAP_INDEX` (`VREG(11)`), `R_MAP_TEX_INDEX_BASE` (`VREG(12)`), `R_MAP_TEX_INDEX`
    /// (`VREG(13)`): the pause map's texture for Link's floor.
    pub r_map_index: i16,
    pub r_map_tex_index_base: i16,
    pub r_map_tex_index: i16,
    /// `R_COMPASS_SCALE_X`, `_Y`, `R_COMPASS_OFFSET_X`, `_Y` (`VREG(14..17)`).
    pub r_compass_scale_x: i16,
    pub r_compass_scale_y: i16,
    pub r_compass_offset_x: i16,
    pub r_compass_offset_y: i16,
    /// `R_OW_MINIMAP_X`, `R_OW_MINIMAP_Y` (`WREG(29)`, `WREG(30)`).
    pub r_ow_minimap_x: i16,
    pub r_ow_minimap_y: i16,
    /// `VREG(30)`: the floor Link is on (the pause map's cursor starts there), 0 to 8.
    pub floor: i16,
    /// `VREG(10)`: `mapRoomNum` as `Map_Update` leaves it.
    pub vreg_10: i16,
    /// `sPlayerInitialPosX`, `sPlayerInitialPosZ`, `sPlayerInitialDirection`: where Link came
    /// into the room (the compass's red arrow).
    pub player_initial_pos_x: i16,
    pub player_initial_pos_z: i16,
    pub player_initial_direction: i16,
    /// `sEntranceIconMapIndex`: the overworld minimap shown (`Map_InitData`'s extended index).
    pub entrance_icon_map_index: i16,
    /// `Map_Update`'s `sLastRoomNum` (it only gates a print).
    pub last_room_num: i16,
}

impl Default for MapState {
    /// What `Regs_InitData` sets (`z_construct.c`, `gameMode` `GAMEMODE_NORMAL`), and the statics'
    /// initial values.
    fn default() -> MapState {
        MapState {
            loaded: false,
            map_segment: None,
            segment: Vec::new(),
            map_palette: [0; 32],
            unk_258: 0,
            unk_25a: 0,
            map_room_num: 0,
            map_palette_index: 0,
            r_map_index: 0,
            r_map_tex_index_base: 0,
            r_map_tex_index: 0,
            r_compass_scale_x: 32,
            r_compass_scale_y: 32,
            r_compass_offset_x: 110,
            r_compass_offset_y: -740,
            r_ow_minimap_x: 0,
            r_ow_minimap_y: 0,
            floor: 0,
            vreg_10: 0,
            player_initial_pos_x: 0,
            player_initial_pos_z: 0,
            player_initial_direction: 0,
            entrance_icon_map_index: 0,
            last_room_num: 99,
        }
    }
}

impl PlayState {
    /// `gMapData`'s tables (none without the pack's).
    fn map_tables(&self) -> Option<std::sync::Arc<crate::play_scene::GameAssets>> {
        self.assets.clone().filter(|_| self.map.loaded)
    }

    /// `Map_SavePlayerInitialInfo`: where Link is and which way he faces, for the compass.
    pub fn map_save_player_initial_info(&mut self) {
        let Some(p) = self.player.and_then(|h| self.actors.actor(h)) else { return };
        self.map.player_initial_pos_x = p.world_pos.x as i16;
        self.map.player_initial_pos_z = p.world_pos.z as i16;
        self.map.player_initial_direction = ((0x7FFF - p.shape_rot.y as i32) / 0x400) as i16;
    }

    /// `Map_SetPaletteData`: `room`'s palette on the pause map is the visited colour, and the
    /// current one if `room` is the minimap's.
    pub fn map_set_palette_data(&mut self, room: i16) {
        let Some(a) = self.map_tables() else { return };
        let map_index = self.save.map_index as usize;
        let palette_index = a.map.data.room_palette.get(map_index * MAP_ROOM_PALETTES + room as usize).copied().unwrap_or(0);
        if self.map.map_room_num == room {
            self.map.map_palette_index = palette_index;
        }
        let i = palette_index as usize * 2;
        self.map.map_palette[i] = 2;
        self.map.map_palette[i + 1] = 0xBF;
    }

    /// `Map_SetFloorPalettesData`: the palette cleared, the map's colour if Link has the map, and
    /// in the dungeons each visited room of `floor` in its colour.
    pub fn map_set_floor_palettes_data(&mut self, floor: i16) {
        let Some(a) = self.map_tables() else { return };
        let map_index = self.save.map_index;
        self.map.map_palette = [0; 32];
        if check_dungeon_item(&self.save, DUNGEON_MAP, map_index) {
            self.map.map_palette[30] = 0;
            self.map.map_palette[31] = 1;
        }
        if is_dungeon_or_boss(self.scene_id) {
            let d = &a.map.data;
            let rooms = self.save.scene_flags(map_index).rooms;
            for i in 0..d.max_palette_count.get(map_index as usize).copied().unwrap_or(0).max(0) as usize {
                let room = d.palette_room(map_index as usize, floor as usize, i);
                if room != 0xFF && rooms & (1u32 << (room & 31)) != 0 {
                    self.map_set_palette_data(room);
                }
            }
        }
    }

    /// `Map_InitData`: the minimap's texture (`mapSegment`), and in the dungeons the room's
    /// compass offset and the floor's palettes.
    pub fn map_init_data(&mut self, room: i16) {
        let Some(a) = self.map_tables() else { return };
        let d = &a.map.data;
        let map_index = self.save.map_index as usize;
        if is_overworld(self.scene_id) {
            let mut extended_map_index = map_index as i16;
            if self.scene_id == SCENE_GRAVEYARD {
                if self.save.check_quest_item(QUEST_SONG_NOCTURNE) {
                    extended_map_index = 0x14;
                }
            } else if self.scene_id == SCENE_LAKE_HYLIA {
                if self.save.adult && !self.save.check_quest_item(QUEST_MEDALLION_WATER) {
                    extended_map_index = 0x15;
                }
            } else if self.scene_id == SCENE_GERUDO_VALLEY {
                if self.save.adult && !carpenters_all_rescued(&self.save) {
                    extended_map_index = 0x16;
                }
            } else if self.scene_id == SCENE_GERUDOS_FORTRESS && carpenters_all_rescued(&self.save) {
                extended_map_index = 0x17;
            }
            self.map.entrance_icon_map_index = extended_map_index;
            // DMA_REQUEST_SYNC(mapSegment, _map_grand_staticSegmentRomStart + owMinimapTexOffset[extendedMapIndex],
            // owMinimapTexSize[mapIndex]).
            let offset = d.ow_minimap_tex_offset.get(extended_map_index as usize).copied().unwrap_or(0) as u32;
            let size = d.ow_minimap_tex_size.get(map_index).copied().unwrap_or(0) as u32;
            self.map.map_segment = Some(MapSegment::Overworld { offset, size });
            self.map.unk_258 = map_index as i16;
        } else if is_dungeon_or_boss(self.scene_id) {
            // DMA_REQUEST_SYNC(mapSegment, _map_i_staticSegmentRomStart + (dgnMinimapTexIndexOffset[mapIndex] + room) *
            // MAP_I_TEX_SIZE, MAP_I_TEX_SIZE).
            let base = d.dgn_minimap_tex_index_offset.get(map_index).copied().unwrap_or(0) as i32;
            self.map.map_segment = Some(MapSegment::Dungeon { index: (base + room as i32) as u32 });
            let at = map_index * MAP_COMPASS_ROOMS + room as usize;
            self.map.r_compass_offset_x = d.room_compass_offset_x.get(at).copied().unwrap_or(0);
            self.map.r_compass_offset_y = d.room_compass_offset_y.get(at).copied().unwrap_or(0);
            let floor = self.map.floor;
            self.map_set_floor_palettes_data(floor);
        }
    }

    /// `Map_InitRoomData`: in the dungeons, `room` is visited (`sceneFlags[mapIndex].rooms`), its
    /// minimap shows, and its palette is set.
    pub fn map_init_room_data(&mut self, room: i16) {
        if !self.map.loaded {
            return;
        }
        let map_index = self.save.map_index;
        if room >= 0 {
            if is_dungeon_or_boss(self.scene_id) {
                if let Some(f) = self.save.scene_flags.get_mut(map_index as usize) {
                    f.rooms |= 1u32 << (room & 31);
                }
                self.map.map_room_num = room;
                self.map.unk_25a = map_index as i16;
                self.map_set_palette_data(room);
                self.map_init_data(room);
            }
        } else {
            self.map.map_room_num = 0;
        }
        // (gSaveContext.sunsSongState: the Sun's Song isn't ported.)
    }

    /// `Map_Destroy` (`MapMark_ClearPointers`: the marks are the pack's).
    pub fn map_destroy(&mut self) {
        self.map.loaded = false;
    }

    /// `Map_Init`: `gSaveContext.mapIndex` for the overworld's and the dungeons' scenes, the
    /// compass's scale and offset, and the first room's data (`MapMark_Init` loads
    /// `ovl_map_mark_data`: the marks are the pack's).
    pub fn map_init(&mut self) {
        if self.assets.is_none() {
            return;
        }
        self.map.loaded = true;
        self.map.unk_258 = -1;
        self.map.unk_25a = -1;
        let Some(a) = self.map_tables() else { return };
        let d = &a.map.data;
        let scene_id = self.scene_id;
        if is_overworld(scene_id) {
            let map_index = (scene_id - SCENE_HYRULE_FIELD) as usize;
            self.save.map_index = map_index as u16;
            self.map.r_map_index = map_index as i16;
            let c = |k: usize| d.ow_compass_info.get(map_index * 4 + k).copied().unwrap_or(0);
            (self.map.r_compass_scale_x, self.map.r_compass_scale_y, self.map.r_compass_offset_x, self.map.r_compass_offset_y) = (c(0), c(1), c(2), c(3));
            self.map_init_data(map_index as i16);
            self.map.r_ow_minimap_x = d.ow_minimap_pos_x.get(map_index).copied().unwrap_or(0);
            self.map.r_ow_minimap_y = d.ow_minimap_pos_y.get(map_index).copied().unwrap_or(0);
        } else if scene_id <= SCENE_SHADOW_TEMPLE_BOSS {
            // The dungeons, SCENE_GANONS_TOWER (0x0A) .. SCENE_TREASURE_BOX_SHOP (0x10), and the boss
            // rooms (the dungeon's index).
            let map_index = if scene_id >= SCENE_DEKU_TREE_BOSS { scene_id - SCENE_DEKU_TREE_BOSS } else { scene_id } as usize;
            self.save.map_index = map_index as u16;
            self.map.r_map_index = map_index as i16;
            if scene_id <= SCENE_ICE_CAVERN || scene_id >= SCENE_DEKU_TREE_BOSS {
                let c = |k: usize| d.dgn_compass_info.get(map_index * 4 + k).copied().unwrap_or(0);
                (self.map.r_compass_scale_x, self.map.r_compass_scale_y, self.map.r_compass_offset_x, self.map.r_compass_offset_y) = (c(0), c(1), c(2), c(3));
                let base = d.dgn_tex_index_base.get(map_index).copied().unwrap_or(0);
                self.map.r_map_tex_index_base = base;
                self.map.r_map_tex_index = base;
                let room = self.room_ctx.cur.num as i16;
                self.map_init_room_data(room);
            }
        }
    }

    /// `Map_Update` (in `Interface_Update`), unless paused: in the dungeons, the map's colour, the
    /// floor Link is on (visited in `sceneFlags[mapIndex].floors`) and its pause map texture, and
    /// the floor switches (a room across floors shows another room's minimap); in a boss room,
    /// its floor.
    pub fn map_update(&mut self) {
        let Some(a) = self.map_tables() else { return };
        if self.pause_ctx.is_paused() {
            return;
        }
        let d = &a.map.data;
        let map_index = self.save.map_index as usize;
        if self.scene_id <= SCENE_ICE_CAVERN {
            self.map.map_palette[30] = 0;
            self.map.map_palette[31] = check_dungeon_item(&self.save, DUNGEON_MAP, map_index as u16) as u8;
            let Some(y) = self.player.and_then(|h| self.actors.actor(h)).map(|p| p.world_pos.y) else { return };
            let mut floor = 0;
            while floor < MAP_FLOORS {
                if y > d.floor_coord_y.get(map_index * MAP_FLOORS + floor).copied().unwrap_or(f32::MIN) {
                    break;
                }
                floor += 1;
            }
            if let Some(f) = self.save.scene_flags.get_mut(map_index) {
                f.floors |= 1u32 << floor;
            }
            self.map.floor = floor as i16;
            let tex = self.map.r_map_tex_index_base + d.floor_tex_index_offset(map_index, floor);
            if self.map.r_map_tex_index != tex {
                self.map.r_map_tex_index = tex;
            }
            if self.map.map_room_num != self.map.last_room_num {
                self.map.last_room_num = self.map.map_room_num;
            }
            let count = d.switch_entry_count.get(map_index).copied().unwrap_or(0) as usize;
            for i in 0..count {
                let at = map_index * MAP_SWITCHES + i;
                let (from_room, from_floor, to_room) = (d.switch_from_room[at], d.switch_from_floor[at], d.switch_to_room[at]);
                if self.map.map_room_num == from_room as i16 && floor == from_floor as usize {
                    self.map.map_room_num = to_room as i16;
                    let room = self.map.map_room_num;
                    self.map_init_data(room);
                    // (gSaveContext.sunsSongState = SUNSSONG_INACTIVE.)
                    self.map_save_player_initial_info();
                }
            }
            self.map.vreg_10 = self.map.map_room_num;
        } else if (SCENE_DEKU_TREE_BOSS..=SCENE_SHADOW_TEMPLE_BOSS).contains(&self.scene_id) {
            let boss = (self.scene_id - SCENE_DEKU_TREE_BOSS) as usize;
            self.map.floor = d.boss_floor.get(boss).copied().unwrap_or(0);
            self.map.r_map_tex_index = self.map.r_map_tex_index_base + d.floor_tex_index_offset(boss, self.map.floor as usize);
        }
    }
}

/// `GET_EVENTCHKINF_CARPENTERS_ALL_RESCUED()` (`save.h`).
fn carpenters_all_rescued(save: &SaveContext) -> bool {
    save.event_chk_inf[EVENTCHKINF_INDEX_CARPENTERS_RESCUED] & EVENTCHKINF_CARPENTERS_ALL_RESCUED_MASK == EVENTCHKINF_CARPENTERS_ALL_RESCUED_MASK
}

/// Whether `Room_FinishRoomChange` saves where Link came in (`Map_SavePlayerInitialInfo`): not in
/// `SCENE_HYRULE_FIELD` .. `SCENE_LON_LON_RANCH`.
pub fn room_change_saves_initial_info(scene_id: u16) -> bool {
    !(SCENE_HYRULE_FIELD..=SCENE_LON_LON_RANCH).contains(&scene_id)
}
