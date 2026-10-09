//! `gSaveContext` (`save.h`): the save file's contents that play reads and writes (the
//! inventory, the equipment, the story flags, each scene's flags), and what one `Play_Init`
//! passes to the next (where it enters, Link's age, the time of day, the respawn points, the
//! entrance speed, the transition type). It outlives each `PlayState`: a scene change builds a
//! new one from it, as `Play_Init` does.
//!
//! Two saves are built as the game builds them (docs/adr/0019-inventory-and-saves.md):
//! - `SaveContext::new`: a new file, `Sram_InitNewSave` (`z_sram.c`): three hearts, no rupees,
//!   no sword, no shield, the Kokiri tunic and boots.
//! - `SaveContext::debug`: the map select's file, `Sram_InitDebugSave` with `fileNum` 0xFF
//!   (`z_select.c`): most of the inventory, ten hearts, and the child with the Kokiri Sword on B
//!   and the Deku Shield.

use glam::Vec3;

use crate::item::*;

/// `RESPAWN_MODE_*`.
pub const RESPAWN_MODE_DOWN: usize = 0;
pub const RESPAWN_MODE_RETURN: usize = 1;
pub const RESPAWN_MODE_TOP: usize = 2;

/// `TRANS_NEXT_TYPE_DEFAULT`: take the transition type from the entrance table.
pub const TRANS_NEXT_TYPE_DEFAULT: u8 = 0xFF;

pub use crate::item::{ITEM_NONE, ITEM_SWORD_KOKIRI, ITEM_SWORD_MASTER};

/// `RespawnData`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RespawnData {
    pub pos: Vec3,
    pub yaw: i16,
    pub player_params: i16,
    pub entrance_index: u16,
    pub room_index: u8,
    pub data: i8,
    pub temp_swch_flags: u32,
    pub temp_collect_flags: u32,
}

/// `ItemEquips`: the buttons' items and the equipment worn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemEquips {
    /// `buttonItems`: B, C-left, C-down, C-right (`ITEM_NONE` when empty).
    pub button_items: [u8; 4],
    /// `cButtonSlots`: the inventory slot each C button shows (`SLOT_NONE`).
    pub c_button_slots: [u8; 3],
    /// `equipment`: a nibble per `EquipmentType`, each an `EquipValue*` (0 for none).
    pub equipment: u16,
}

impl ItemEquips {
    /// Nothing on the buttons, nothing worn.
    pub const NONE: ItemEquips = ItemEquips { button_items: [ITEM_NONE; 4], c_button_slots: [SLOT_NONE; 3], equipment: 0 };
}

/// `Inventory`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Inventory {
    /// `items`: each slot's item (`ITEM_NONE` when empty).
    pub items: [u8; 24],
    /// `ammo`, by slot.
    pub ammo: [i8; 16],
    /// `equipment`: a nibble per `EquipmentType`, a bit per owned piece (`EquipInv*`).
    pub equipment: u16,
    /// `upgrades`: `gUpgradeMasks`' fields.
    pub upgrades: u32,
    /// `questItems` (`QUEST_*` bits, the heart pieces in the top four).
    pub quest_items: u32,
    pub dungeon_items: [u8; 20],
    pub dungeon_keys: [i8; 19],
    pub defense_hearts: i8,
    pub gs_tokens: i16,
}

/// `SavedSceneFlags`: a scene's flags, kept between visits (`Play_SaveSceneFlags` when play
/// ends, and `Actor_InitContext` loads them).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SavedSceneFlags {
    pub chest: u32,
    pub swch: u32,
    pub clear: u32,
    pub collect: u32,
    pub unk: u32,
    pub rooms: u32,
    pub floors: u32,
}

/// `sceneFlags`' size.
pub const SCENE_FLAGS_COUNT: usize = 124;

/// `FaroresWindData`: Farore's Wind's warp point (not ported: kept as the save holds it).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FaroresWindData {
    pub pos: [i32; 3],
    pub yaw: i32,
    pub player_params: i32,
    pub entrance_index: i32,
    pub room_index: i32,
    pub set: i32,
    pub temp_swch_flags: i32,
    pub temp_collect_flags: i32,
}

/// `HorseData`: where Epona was left (not ported: kept as the save holds it).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HorseData {
    pub scene_id: i16,
    pub pos: [i16; 3],
    pub angle: i16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SaveContext {
    /// `entranceIndex`: the `gEntranceTable` row the next `Play_Init` enters by (the group's
    /// first row; the scene layer is added).
    pub entrance_index: u16,
    /// `sceneLayer`.
    pub scene_layer: usize,
    /// `linkAge == LINK_AGE_ADULT`.
    pub adult: bool,
    /// `dayTime` (`CLOCK_TIME`), and `nightFlag` as `Play_Init` sets it.
    pub day_time: u16,
    pub night_flag: bool,
    /// `respawn[RESPAWN_MODE_MAX]` and `respawnFlag` (0: a normal entrance; 1..3: respawn from
    /// that mode + 1; negative: the last entrance).
    pub respawn: [RespawnData; 3],
    pub respawn_flag: i32,
    /// `entranceSpeed`: Player's speed through the exit, for the walk-in on the other side.
    pub entrance_speed: f32,
    /// `entranceSound`: what Player plays at its init on the other side (a door's).
    pub entrance_sound: u16,
    /// `nextTransitionType` (`TRANS_NEXT_TYPE_DEFAULT` for the entrance table's).
    pub next_transition_type: u8,
    /// `transFadeDuration`, `transWipeSpeed`.
    pub trans_fade_duration: u16,
    pub trans_wipe_speed: u8,
    /// `retainWeatherMode`, `showTitleCard`.
    pub retain_weather_mode: bool,
    pub show_title_card: bool,
    /// `seqId`, `natureAmbienceId`: the music and the nature ambience the last scene started
    /// (`Play_Init`), `NA_BGM_DISABLED` and `NATURE_ID_DISABLED` once they've faded out (a
    /// scene left without `ENTRANCE_INFO_CONTINUE_BGM_FLAG`, a new game). `forcedSeqId`: what
    /// the next scene plays instead of its own (`Environment_ForcePlaySequence`),
    /// `NA_BGM_GENERAL_SFX` for none.
    pub seq_id: u8,
    pub nature_ambience_id: u8,
    pub forced_seq_id: u16,
    /// `cutsceneIndex`: 0 for none, 0xFFF0 and up for a scene's cutscene layer
    /// (`SCENE_LAYER_CUTSCENE_FIRST + (cutsceneIndex & 0xF)`), 0xFFFD while a script plays. The
    /// file select's new file enters with 0xFFF1, the opening on Link's house's layer 5
    /// ([`SaveContext::file_select_new`]); `SaveContext::new` enters with 0.
    pub cutscene_index: u16,
    /// `nextCutsceneIndex` (0xFFEF: none), `cutsceneTrigger` (1 an actor's script, 2 an
    /// entrance's), `cutsceneTransitionControl`.
    pub next_cutscene_index: u16,
    pub cutscene_trigger: u8,
    pub cutscene_transition_control: u8,
    /// `gameMode` (`GAMEMODE_NORMAL` 0), `fileNum` (0xFF for the map select's file).
    pub game_mode: u8,
    pub file_num: i32,
    /// `eventChkInf`, `itemGetInf`, `infTable`: the story and conversation flags.
    pub event_chk_inf: [u16; 14],
    pub item_get_inf: [u16; 4],
    pub inf_table: [u16; 30],
    /// `eventInf`: the flags of an event in progress (a minigame, the marathon), which a game
    /// over clears.
    pub event_inf: [u16; 4],
    /// `playerName`, in the file select's character codes.
    pub player_name: [u8; 8],
    /// `deaths`.
    pub deaths: u16,
    /// `naviTimer`: counts play frames outside cutscenes (`func_80A053F0`, to 25800); between 600
    /// and 3000 Navi has her C-Up text (`QuestHint_GetNaviTextId`), and talking to her with it sets
    /// 3001.
    pub navi_timer: u16,
    /// `healthCapacity`, `health` (16 a heart), `healthAccumulator` (health still to add).
    pub health_capacity: i16,
    pub health: i16,
    pub health_accumulator: i16,
    /// `magicLevel`, `magic` (the meter isn't ported; they're kept as the saves set them).
    pub magic_level: i8,
    pub magic: i8,
    /// `isMagicAcquired`, `isDoubleMagicAcquired`, `isDoubleDefenseAcquired`.
    pub is_magic_acquired: bool,
    pub is_double_magic_acquired: bool,
    pub is_double_defense_acquired: bool,
    /// `rupees`, `rupeeAccumulator` (rupees still to add or take).
    pub rupees: i16,
    pub rupee_accumulator: i16,
    /// `swordHealth` (Biggoron's Sword), `bgsFlag`.
    pub sword_health: u16,
    pub bgs_flag: bool,
    /// `childEquips`, `adultEquips` (what `Inventory_SwapAgeEquipment` swaps in), `equips`.
    pub child_equips: ItemEquips,
    pub adult_equips: ItemEquips,
    pub equips: ItemEquips,
    /// `inventory`.
    pub inventory: Inventory,
    /// `sceneFlags`, by scene id.
    pub scene_flags: Vec<SavedSceneFlags>,
    /// `gsFlags`: the Gold Skulltulas' tokens taken, a byte of flags per index (a scene's
    /// number + 1 in the placed ones' params), four to a word (`GET_GS_FLAGS`, `SET_GS_FLAGS`).
    pub gs_flags: [i32; 6],
    /// `savedSceneId`.
    pub saved_scene_id: u16,
    /// `mapIndex`: the dungeon whose items `Item_Give` counts, or the overworld area's minimap
    /// (`Map_Init`, `crate::map`; other scenes keep the last one's).
    pub map_index: u16,
    /// `buttonStatus` (B, the C buttons, A; `BTN_ENABLED` 0, `BTN_DISABLED` 0xFF).
    pub button_status: [u8; 5],
    /// `forceRisingButtonAlphas`, `nextHudVisibilityMode`, `hudVisibilityMode`, `hudVisibilityModeTimer`, `prevHudVisibilityMode`: the interface's alpha
    /// type (`Interface_ChangeHudVisibilityMode`), its fade step and the type to go back to.
    pub force_rising_button_alphas: u8,
    /// `envHazardTextTriggerFlags` (`ENV_HAZARD_TEXT_TRIGGER_*`): the hot room's and the iron
    /// boots' underwater texts shown once (`Player_GetEnvironmentalHazard`).
    pub env_hazard_text_trigger_flags: u8,
    pub next_hud_visibility_mode: u16,
    pub hud_visibility_mode: u16,
    pub hud_visibility_mode_timer: u16,
    pub prev_hud_visibility_mode: u16,
    /// `language`: English.
    pub language: u8,
    /// `soundSetting`, `zTargetSetting`: the SRAM header's (`Sram_InitSram`); stereo and
    /// "Switch", which a fresh header gives, are what the port plays with.
    pub sound_setting: u8,
    pub z_target_setting: u8,
    // The rest of `Save` (`save.h`), which nothing ported reads: kept so that a slot read back
    // (`Sram_OpenSave`) is written back the same (`Sram_WriteSave`, crate::sram).
    /// `totalDays`, `bgsDayCount` (`Environment_Update`'s clock isn't ported).
    pub total_days: i32,
    pub bgs_day_count: i32,
    /// `playerData.newf`: "ZELDAZ" in a file that exists (`Sram_InitSave`).
    pub newf: [u8; 6],
    /// `playerData.n64ddFlag`, `unk_3B` (`sNewSavePlayerData`'s `unk_1F`),
    /// `ocarinaGameRoundNum`, `unk_54` (`unk_38`), `unk_58` (`unk_3C`).
    pub n64dd_flag: i16,
    pub unk_3b: u8,
    pub ocarina_game_round_num: u8,
    pub unk_54: u32,
    pub unk_58: [u8; 0x0E],
    /// `fw`.
    pub fw: FaroresWindData,
    /// `unk_E8C`, `unk_EB4`.
    pub unk_e8c: [u8; 0x10],
    pub unk_eb4: [u8; 0x4],
    /// `highScores` (`HS_*`).
    pub high_scores: [i32; 7],
    /// `unk_F34`, `worldMapAreaData` ("area_arrival"), `unk_F3C`.
    pub unk_f34: [u8; 0x4],
    pub world_map_area_data: u32,
    pub unk_f3c: [u8; 0x4],
    /// `scarecrowLongSongSet`, `scarecrowLongSong`, `unk_12A1`, `scarecrowSpawnSongSet`,
    /// `scarecrowSpawnSong`, `unk_1346` (the ocarina isn't ported).
    pub scarecrow_long_song_set: u8,
    pub scarecrow_long_song: [u8; 0x360],
    pub unk_12a1: [u8; 0x24],
    pub scarecrow_spawn_song_set: u8,
    pub scarecrow_spawn_song: [u8; 0x80],
    pub unk_1346: [u8; 0x2],
    /// `horseData`.
    pub horse_data: HorseData,
    /// `checksum` (`Sram_WriteSave`).
    pub checksum: u16,
}

/// `EVENTCHKINF_A8` (`save.h`): the Deku Tree's intro seen.
pub const EVENTCHKINF_A8: u16 = 0xA8;

/// `gGsFlagsMasks`, `gGsFlagsShifts` (`z_inventory.c`).
pub const GS_FLAGS_MASKS: [u32; 4] = [0x0000_00FF, 0x0000_FF00, 0x00FF_0000, 0xFF00_0000];
pub const GS_FLAGS_SHIFTS: [u32; 4] = [0, 8, 16, 24];

/// `GAMEMODE_NORMAL` (`save.h`).
pub const GAMEMODE_NORMAL: u8 = 0;

/// `LANGUAGE_ENG`.
pub const LANGUAGE_ENG: u8 = 0;

/// `EVENTCHKINF_04` (`save.h`: 0x04): Mido has let Link through to the Deku Tree. Set by
/// `EnMd_BlockPath` (`z_en_md.c:743`) when his text 0x1033 (Link wearing the Kokiri Sword and the
/// Deku Shield) closes; from then on he stands at his path's last point (`EnMd_SetMovedPos`).
pub const EVENTCHKINF_04: u16 = 0x04;
/// `EVENTCHKINF_05` (`save.h`: 0x05): the Deku Tree has opened his mouth. Set by
/// `func_808BC9EC` (`z_bg_treemouth.c`) when Link answers the tree's question with the first
/// choice; from then on `func_808BC8B8` holds the mouth open (`unk_168` 1).
pub const EVENTCHKINF_05: u16 = 0x05;
/// `EVENTCHKINF_07` (0x07): the Deku Tree is dead. Set with `EVENTCHKINF_09` and the Kokiri
/// Emerald by `Door_Warp1`'s blue warp out of Gohma's room (`z_door_warp1.c`, `SCENE_DEKU_TREE_BOSS`).
/// The tree and his mouth are drawn with env alpha 2150 instead of 500
/// (`Scene_DrawConfigKokiriForest`, `BgTreemouth_Draw`).
pub const EVENTCHKINF_07: u16 = 0x07;
/// `EVENTCHKINF_09` (0x09): set with `EVENTCHKINF_07` by the same blue warp.
pub const EVENTCHKINF_09: u16 = 0x09;
/// `EVENTCHKINF_0C` (0x0C): Link has met the Deku Tree. Set by `func_808BC8B8` as it starts the
/// first talk's cutscene (`gDekuTreeMeetingCs`).
pub const EVENTCHKINF_0C: u16 = 0x0C;
/// `EVENTCHKINF_OBTAINED_ZELDAS_LETTER`: Zelda's letter obtained (`(4 << 4) | 0`).
pub const EVENTCHKINF_OBTAINED_ZELDAS_LETTER: u16 = 0x40;
/// `EVENTCHKINF_ZELDA_FLED_CASTLE`, `EVENTCHKINF_C4` (`save.h`), set by `Sram_InitDebugSave`.
pub const EVENTCHKINF_ZELDA_FLED_CASTLE: u16 = 0x80;
pub const EVENTCHKINF_C4: u16 = 0xC4;
/// `INFTABLE_INDEX_1DX` (29): `infTable[29]` is 1 while the B button has had no sword since
/// the file began or lost it (`Sram_InitNewSave`, `Inventory_DeleteEquipment`); the pause
/// menu's equipment screen clears it when it equips a sword. `func_80083108` only gives B
/// back its stored item while it's 0.
pub const INFTABLE_INDEX_1DX: usize = 29;
/// `ENTR_LINKS_HOUSE_0` (`entrance_table.h`, row 0x0BB): Link's house, by his bed.
pub const ENTR_LINKS_HOUSE_0: u16 = 0x0BB;
/// `ENTR_LOAD_OPENING` (`scene.h`: -1): `Play_Init` goes to the title screen instead.
pub const ENTR_LOAD_OPENING: u16 = 0xFFFF;
/// `SCENE_LINKS_HOUSE` (`scene_table.h`).
pub const SCENE_LINKS_HOUSE: u16 = 0x34;
/// `MAGIC_NORMAL_METER`.
pub const MAGIC_NORMAL_METER: i8 = 0x30;
/// `ENTR_HYRULE_FIELD_0` (`entrance_table.h`, row 0x0CD): where `Sram_InitDebugSave` enters.
pub const ENTR_HYRULE_FIELD_0: u16 = 0x0CD;
/// `SCENE_WATER_TEMPLE` (`scene_table.h`, 0x05): the scene whose switch 0x1E the new saves set
/// (the water at its lowest).
pub const SCENE_WATER_TEMPLE: usize = 0x05;
/// `FILENAME_SPACE` (`message.h`, `!OOT_NTSC`).
pub const FILENAME_SPACE: u8 = 0x3E;
/// `Sram_InitNewSave`'s and `Sram_InitDebugSave`'s Epona: Hyrule Field, (-1840, 72, 5497),
/// facing -0x6AD9.
pub const SRAM_HORSE_DATA: HorseData = HorseData { scene_id: crate::play_scene::SCENE_HYRULE_FIELD as i16, pos: [-1840, 72, 5497], angle: -0x6AD9 };

pub use crate::item::{QUEST_KOKIRI_EMERALD, QUEST_MEDALLION_FOREST};

/// A debug save preset: story flags and items set on a new save, standing in for events that
/// aren't playable yet (the Deku Tree's talk is a cutscene, and there are no cutscenes until
/// Phase 4; there's no pause menu to equip with). Not in the game.
pub struct SavePreset {
    pub name: &'static str,
    pub about: &'static str,
    pub apply: fn(&mut SaveContext),
}

/// What a save that has met the Deku Tree has from the story before it: the Kokiri Sword (from
/// its chest, `Item_Give(ITEM_SWORD_KOKIRI)`) and the Deku Shield (from the shop,
/// `Item_Give(ITEM_SHIELD_DEKU)`), both owned (`OWNED_EQUIP_FLAG`: `inventory.equipment` bits
/// 0 and 4) and equipped as the pause menu's equipment screen equips them
/// (`z_kaleido_equipment.c`: `Inventory_ChangeEquipment(EQUIP_TYPE_SWORD,
/// EQUIP_VALUE_SWORD_KOKIRI)`, `infTable[INFTABLE_INDEX_1DX] = 0`, `buttonItems[0] =
/// ITEM_SWORD_KOKIRI`; and `Inventory_ChangeEquipment(EQUIP_TYPE_SHIELD,
/// EQUIP_VALUE_SHIELD_DEKU)`): `equips.equipment` nibbles 0 and 1 are 1. Mido lets Link past
/// with both worn (`z_en_md.c`: `CUR_EQUIP_VALUE(EQUIP_TYPE_SHIELD) == EQUIP_VALUE_SHIELD_DEKU &&
/// CUR_EQUIP_VALUE(EQUIP_TYPE_SWORD) == EQUIP_VALUE_SWORD_KOKIRI`).
/// `gEquipAgeReqs` (`z_kaleido_scope.c`): who can wear each equipment type's pieces
/// (column 0 is the upgrades'), 9 either age, else `LINK_AGE_ADULT` 0 or `LINK_AGE_CHILD` 1.
pub const EQUIP_AGE_REQS: [[u8; 4]; 4] = [[0, 1, 0, 0], [9, 1, 9, 0], [0, 9, 0, 0], [9, 9, 0, 0]];
/// `LINK_AGE_ADULT`, `LINK_AGE_CHILD`.
pub const LINK_AGE_ADULT: u8 = 0;
pub const LINK_AGE_CHILD: u8 = 1;

pub fn kokiri_sword_and_deku_shield(s: &mut SaveContext) {
    item_give(s, None, ITEM_SWORD_KOKIRI);
    item_give(s, None, ITEM_SHIELD_DEKU);
    s.equip_from_pause_menu(EQUIP_TYPE_SWORD, EQUIP_VALUE_SWORD_KOKIRI);
    s.equip_from_pause_menu(EQUIP_TYPE_SHIELD, EQUIP_VALUE_SHIELD_DEKU);
}

/// The debug save presets (`--preset` in the game and the sandbox).
pub const SAVE_PRESETS: &[SavePreset] = &[
    SavePreset {
        name: "deku-tree-open",
        about: "the Kokiri Sword and the Deku Shield owned and equipped, Mido stepped aside (EVENTCHKINF_04), the Deku Tree met and his mouth open (EVENTCHKINF_0C, EVENTCHKINF_05), as after his first talk's cutscenes",
        apply: |s| {
            kokiri_sword_and_deku_shield(s);
            s.set_event_chk_inf(EVENTCHKINF_04);
            s.set_event_chk_inf(EVENTCHKINF_0C);
            s.set_event_chk_inf(EVENTCHKINF_05);
        },
    },
    SavePreset {
        name: "deku-tree-dead",
        about: "deku-tree-open, and the Deku Tree dead with the Kokiri Emerald (EVENTCHKINF_07, EVENTCHKINF_09, QUEST_KOKIRI_EMERALD), as after Gohma's blue warp",
        apply: |s| {
            kokiri_sword_and_deku_shield(s);
            s.set_event_chk_inf(EVENTCHKINF_04);
            s.set_event_chk_inf(EVENTCHKINF_0C);
            s.set_event_chk_inf(EVENTCHKINF_05);
            // Door_Warp1 (SCENE_DEKU_TREE_BOSS): Flags_SetEventChkInf(EVENTCHKINF_07) and (_09),
            // Item_Give(play, ITEM_KOKIRI_EMERALD).
            s.set_event_chk_inf(EVENTCHKINF_07);
            s.set_event_chk_inf(EVENTCHKINF_09);
            item_give(s, None, ITEM_KOKIRI_EMERALD);
        },
    },
    SavePreset {
        name: "deku-tree-inside",
        about: "deku-tree-open, and the Deku Tree's intro seen (EVENTCHKINF_A8, sEntranceCutsceneTable's flag for ENTR_DEKU_TREE_0), so entering it plays no cutscene",
        apply: |s| {
            kokiri_sword_and_deku_shield(s);
            s.set_event_chk_inf(EVENTCHKINF_04);
            s.set_event_chk_inf(EVENTCHKINF_0C);
            s.set_event_chk_inf(EVENTCHKINF_05);
            s.set_event_chk_inf(EVENTCHKINF_A8);
        },
    },
    SavePreset {
        name: "deku-tree-inside-fairy",
        about: "deku-tree-inside, and a fairy in the first bottle (ITEM_BOTTLE_FAIRY in SLOT_BOTTLE_1), which revives Link once (Inventory_ConsumeFairy)",
        apply: |s| {
            kokiri_sword_and_deku_shield(s);
            s.set_event_chk_inf(EVENTCHKINF_04);
            s.set_event_chk_inf(EVENTCHKINF_0C);
            s.set_event_chk_inf(EVENTCHKINF_05);
            s.set_event_chk_inf(EVENTCHKINF_A8);
            s.inventory.items[crate::item::SLOT_BOTTLE_1] = crate::item::ITEM_BOTTLE_FAIRY;
        },
    },
    SavePreset {
        name: "deku-tree-sticks",
        about: "deku-tree-inside, and ten Deku Sticks (Item_Give(ITEM_DEKU_STICKS_10)) on C-Left, as the pause menu equips them (GAME-05 milestone 4b)",
        apply: |s| {
            kokiri_sword_and_deku_shield(s);
            s.set_event_chk_inf(EVENTCHKINF_04);
            s.set_event_chk_inf(EVENTCHKINF_0C);
            s.set_event_chk_inf(EVENTCHKINF_05);
            s.set_event_chk_inf(EVENTCHKINF_A8);
            item_give(s, None, ITEM_DEKU_STICKS_10);
            s.equip_item_on_c_left(ITEM_DEKU_STICK);
        },
    },
    SavePreset {
        name: "deku-tree-slingshot",
        about: "deku-tree-sticks, and ten Deku nuts (Item_Give(ITEM_DEKU_NUTS_10)) on C-Down and the Fairy Slingshot with 30 seeds (Item_Give(ITEM_SLINGSHOT)) on C-Right, as the pause menu equips them (GAME-05 milestone 5a): ready to use",
        apply: |s| {
            kokiri_sword_and_deku_shield(s);
            s.set_event_chk_inf(EVENTCHKINF_04);
            s.set_event_chk_inf(EVENTCHKINF_0C);
            s.set_event_chk_inf(EVENTCHKINF_05);
            s.set_event_chk_inf(EVENTCHKINF_A8);
            item_give(s, None, ITEM_DEKU_STICKS_10);
            s.equip_item_on_c(0, ITEM_DEKU_STICK);
            item_give(s, None, ITEM_DEKU_NUTS_10);
            s.equip_item_on_c(1, ITEM_DEKU_NUT);
            item_give(s, None, ITEM_SLINGSHOT);
            s.equip_item_on_c(2, ITEM_SLINGSHOT);
        },
    },
    SavePreset {
        name: "deku-tree-gohma",
        about: "deku-tree-slingshot, for Queen Gohma's room (ENTR_DEKU_TREE_BOSS_0): the Kokiri Sword and Deku Shield, sticks on C-Left, nuts on C-Down, the slingshot on C-Right, three hearts; her battle not begun, so her intro plays whole (GAME-05 milestone 6a)",
        apply: |s| {
            kokiri_sword_and_deku_shield(s);
            s.set_event_chk_inf(EVENTCHKINF_04);
            s.set_event_chk_inf(EVENTCHKINF_0C);
            s.set_event_chk_inf(EVENTCHKINF_05);
            s.set_event_chk_inf(EVENTCHKINF_A8);
            item_give(s, None, ITEM_DEKU_STICKS_10);
            s.equip_item_on_c(0, ITEM_DEKU_STICK);
            item_give(s, None, ITEM_DEKU_NUTS_10);
            s.equip_item_on_c(1, ITEM_DEKU_NUT);
            item_give(s, None, ITEM_SLINGSHOT);
            s.equip_item_on_c(2, ITEM_SLINGSHOT);
        },
    },
    SavePreset {
        name: "deku-tree-gohma-again",
        about: "deku-tree-gohma with her battle begun (EVENTCHKINF_BEGAN_GOHMA_BATTLE), as after a game over in her room: her intro skips the slab's fall and the look at her, and shows no title card (GAME-05 milestone 6a)",
        apply: |s| {
            kokiri_sword_and_deku_shield(s);
            s.set_event_chk_inf(EVENTCHKINF_04);
            s.set_event_chk_inf(EVENTCHKINF_0C);
            s.set_event_chk_inf(EVENTCHKINF_05);
            s.set_event_chk_inf(EVENTCHKINF_A8);
            item_give(s, None, ITEM_DEKU_STICKS_10);
            s.equip_item_on_c(0, ITEM_DEKU_STICK);
            item_give(s, None, ITEM_DEKU_NUTS_10);
            s.equip_item_on_c(1, ITEM_DEKU_NUT);
            item_give(s, None, ITEM_SLINGSHOT);
            s.equip_item_on_c(2, ITEM_SLINGSHOT);
            // EVENTCHKINF_BEGAN_GOHMA_BATTLE (save.h).
            s.set_event_chk_inf(0x70);
        },
    },
    SavePreset {
        name: "deku-tree-gohma-cleared",
        about: "deku-tree-gohma-again, and Queen Gohma beaten: her room (SCENE_DEKU_TREE_BOSS, room 1) cleared and its heart container taken (collectible flag 0x1F, four hearts), so her init leaves the blue warp (GAME-05 milestone 6b)",
        apply: |s| {
            kokiri_sword_and_deku_shield(s);
            s.set_event_chk_inf(EVENTCHKINF_04);
            s.set_event_chk_inf(EVENTCHKINF_0C);
            s.set_event_chk_inf(EVENTCHKINF_05);
            s.set_event_chk_inf(EVENTCHKINF_A8);
            item_give(s, None, ITEM_DEKU_STICKS_10);
            s.equip_item_on_c(0, ITEM_DEKU_STICK);
            item_give(s, None, ITEM_DEKU_NUTS_10);
            s.equip_item_on_c(1, ITEM_DEKU_NUT);
            item_give(s, None, ITEM_SLINGSHOT);
            s.equip_item_on_c(2, ITEM_SLINGSHOT);
            // EVENTCHKINF_BEGAN_GOHMA_BATTLE (save.h).
            s.set_event_chk_inf(0x70);
            // SCENE_DEKU_TREE_BOSS (0x11): BossGoma_Defeated's Flags_SetClear(room 1) and
            // Item_B_Heart's Flags_SetCollectible(0x1F), as Play_SaveSceneFlags keeps them.
            s.scene_flags[0x11].clear |= 1 << 1;
            s.scene_flags[0x11].collect |= 1 << 0x1F;
            // GI_HEART_CONTAINER_2: Item_Give(ITEM_HEART_CONTAINER).
            item_give(s, None, crate::item::ITEM_HEART_CONTAINER);
        },
    },
    SavePreset {
        name: "deku-tree-slingshot-owned",
        about: "deku-tree-sticks, ten Deku nuts on C-Down, and the Fairy Slingshot with 30 seeds owned but on no button, as room 10's chest leaves it: the pause menu equips it (GAME-05 milestone 5b)",
        apply: |s| {
            kokiri_sword_and_deku_shield(s);
            s.set_event_chk_inf(EVENTCHKINF_04);
            s.set_event_chk_inf(EVENTCHKINF_0C);
            s.set_event_chk_inf(EVENTCHKINF_05);
            s.set_event_chk_inf(EVENTCHKINF_A8);
            item_give(s, None, ITEM_DEKU_STICKS_10);
            s.equip_item_on_c(0, ITEM_DEKU_STICK);
            item_give(s, None, ITEM_DEKU_NUTS_10);
            s.equip_item_on_c(1, ITEM_DEKU_NUT);
            item_give(s, None, ITEM_SLINGSHOT);
        },
    },
    SavePreset {
        name: "deku-tree-compass",
        about: "deku-tree-inside, the Deku Tree's compass (Item_Give(ITEM_DUNGEON_COMPASS) at mapIndex 0), and its 3F, 2F and 1F visited with rooms 0, 1 and 2 (sceneFlags[0].floors, .rooms), as after a climb to the second floor's rooms: the pause menu's map page shows those floors and the chests' marks (GAME-05 milestone 5b-2)",
        apply: |s| {
            kokiri_sword_and_deku_shield(s);
            s.set_event_chk_inf(EVENTCHKINF_04);
            s.set_event_chk_inf(EVENTCHKINF_0C);
            s.set_event_chk_inf(EVENTCHKINF_05);
            s.set_event_chk_inf(EVENTCHKINF_A8);
            item_give(s, None, crate::item::ITEM_DUNGEON_COMPASS);
            // The Deku Tree's floors 3 (3F), 4 (2F) and 5 (1F) (`sFloorID`), as Map_Update marks
            // them; rooms 0 (the central room, on 3F to 1F), 1 and 2 (2F), as Map_InitRoomData.
            let f = &mut s.scene_flags[0];
            f.floors |= (1 << 3) | (1 << 4) | (1 << 5);
            f.rooms |= (1 << 0) | (1 << 1) | (1 << 2);
        },
    },
    SavePreset {
        name: "deku-tree-quarter-heart",
        about: "deku-tree-inside with a quarter heart left (health 4): the next hit kills Link and the game over follows (GAME-05 milestone 5b-2)",
        apply: |s| {
            kokiri_sword_and_deku_shield(s);
            s.set_event_chk_inf(EVENTCHKINF_04);
            s.set_event_chk_inf(EVENTCHKINF_0C);
            s.set_event_chk_inf(EVENTCHKINF_05);
            s.set_event_chk_inf(EVENTCHKINF_A8);
            s.health = 4;
        },
    },
    SavePreset {
        name: "deku-tree-save",
        about: "deku-tree-inside, the Fairy Slingshot with 30 seeds owned on no button, and two hearts (health 0x20): the pause menu equips it and saves, and the file loads back with three (Sram_OpenSave) (GAME-05 milestone 5c)",
        apply: |s| {
            kokiri_sword_and_deku_shield(s);
            s.set_event_chk_inf(EVENTCHKINF_04);
            s.set_event_chk_inf(EVENTCHKINF_0C);
            s.set_event_chk_inf(EVENTCHKINF_05);
            s.set_event_chk_inf(EVENTCHKINF_A8);
            item_give(s, None, ITEM_SLINGSHOT);
            s.health = 0x20;
        },
    },
    SavePreset {
        name: "sword-and-40-rupees",
        about: "the Kokiri Sword owned and worn and 40 rupees, what a new save has on its way to the Kokiri shop (GAME-03 milestone 3); no shield, Mido still blocking",
        apply: |s| {
            item_give(s, None, ITEM_SWORD_KOKIRI);
            s.equip_from_pause_menu(EQUIP_TYPE_SWORD, EQUIP_VALUE_SWORD_KOKIRI);
            s.rupees = 40;
        },
    },
];

impl Default for SaveContext {
    fn default() -> SaveContext {
        SaveContext::new(0, false, 0)
    }
}

impl SaveContext {
    /// The fields play sets, around a file's contents.
    pub(crate) fn blank(entrance_index: u16, adult: bool, day_time: u16) -> SaveContext {
        SaveContext {
            entrance_index,
            scene_layer: 0,
            adult,
            day_time,
            night_flag: false,
            respawn: [RespawnData::default(); 3],
            respawn_flag: 0,
            entrance_speed: 0.0,
            entrance_sound: 0,
            next_transition_type: TRANS_NEXT_TYPE_DEFAULT,
            trans_fade_duration: 0,
            trans_wipe_speed: 0,
            retain_weather_mode: false,
            show_title_card: true,
            // SaveContext_Init (z_common_data.c); the file select (z_file_choose.c) and the map
            // select (z_select.c) set the same.
            seq_id: crate::audio::NA_BGM_DISABLED as u8,
            nature_ambience_id: crate::audio::NATURE_ID_DISABLED,
            forced_seq_id: crate::audio::NA_BGM_GENERAL_SFX,
            cutscene_index: 0,
            // SaveContext_Init (z_common_data.c).
            next_cutscene_index: 0xFFEF,
            cutscene_trigger: 0,
            cutscene_transition_control: 0,
            game_mode: GAMEMODE_NORMAL,
            file_num: 0,
            event_chk_inf: [0; 14],
            item_get_inf: [0; 4],
            inf_table: [0; 30],
            event_inf: [0; 4],
            player_name: [FILENAME_SPACE; 8],
            deaths: 0,
            navi_timer: 0,
            health_capacity: 0,
            health: 0,
            health_accumulator: 0,
            magic_level: 0,
            magic: 0,
            is_magic_acquired: false,
            is_double_magic_acquired: false,
            is_double_defense_acquired: false,
            rupees: 0,
            rupee_accumulator: 0,
            sword_health: 0,
            bgs_flag: false,
            child_equips: ItemEquips::NONE,
            adult_equips: ItemEquips::NONE,
            equips: ItemEquips::NONE,
            inventory: Inventory {
                items: [ITEM_NONE; 24],
                ammo: [0; 16],
                equipment: 0,
                upgrades: 0,
                quest_items: 0,
                dungeon_items: [0; 20],
                dungeon_keys: [0; 19],
                defense_hearts: 0,
                gs_tokens: 0,
            },
            scene_flags: vec![SavedSceneFlags::default(); SCENE_FLAGS_COUNT],
            gs_flags: [0; 6],
            saved_scene_id: 0,
            map_index: 0,
            button_status: [0; 5],
            force_rising_button_alphas: 0,
            env_hazard_text_trigger_flags: 0,
            next_hud_visibility_mode: 0,
            hud_visibility_mode: 0,
            hud_visibility_mode_timer: 0,
            prev_hud_visibility_mode: 0,
            language: LANGUAGE_ENG,
            sound_setting: 0,
            z_target_setting: 0,
            total_days: 0,
            bgs_day_count: 0,
            newf: [0; 6],
            n64dd_flag: 0,
            unk_3b: 0,
            ocarina_game_round_num: 0,
            unk_54: 0,
            unk_58: [0; 0x0E],
            fw: FaroresWindData::default(),
            unk_e8c: [0; 0x10],
            unk_eb4: [0; 0x4],
            high_scores: [0; 7],
            unk_f34: [0; 0x4],
            world_map_area_data: 0,
            unk_f3c: [0; 0x4],
            scarecrow_long_song_set: 0,
            scarecrow_long_song: [0; 0x360],
            unk_12a1: [0; 0x24],
            scarecrow_spawn_song_set: 0,
            scarecrow_spawn_song: [0; 0x80],
            unk_1346: [0; 0x2],
            horse_data: HorseData::default(),
            checksum: 0,
        }
    }

    /// `bzero(&gSaveContext.save.info, sizeof(SaveInfo))`: every `SaveInfo` field zero.
    pub fn clear_info(&mut self) {
        // SaveInfo is playerData, equips, inventory and the rest: everything from deaths to the
        // checksum, but not Save's own words (entranceIndex .. bgsDayCount) nor the runtime part.
        self.newf = [0; 6];
        self.deaths = 0;
        self.player_name = [0; 8];
        self.n64dd_flag = 0;
        self.health_capacity = 0;
        self.health = 0;
        self.magic_level = 0;
        self.magic = 0;
        self.rupees = 0;
        self.sword_health = 0;
        self.navi_timer = 0;
        self.is_magic_acquired = false;
        self.unk_3b = 0;
        self.is_double_magic_acquired = false;
        self.is_double_defense_acquired = false;
        self.bgs_flag = false;
        self.ocarina_game_round_num = 0;
        self.child_equips = ItemEquips { button_items: [0; 4], c_button_slots: [0; 3], equipment: 0 };
        self.adult_equips = self.child_equips;
        self.unk_54 = 0;
        self.unk_58 = [0; 0x0E];
        self.saved_scene_id = 0;
        self.equips = self.child_equips;
        self.inventory = Inventory { items: [0; 24], ammo: [0; 16], equipment: 0, upgrades: 0, quest_items: 0, dungeon_items: [0; 20], dungeon_keys: [0; 19], defense_hearts: 0, gs_tokens: 0 };
        self.scene_flags = vec![SavedSceneFlags::default(); SCENE_FLAGS_COUNT];
        self.fw = FaroresWindData::default();
        self.unk_e8c = [0; 0x10];
        self.gs_flags = [0; 6];
        self.unk_eb4 = [0; 0x4];
        self.high_scores = [0; 7];
        self.event_chk_inf = [0; 14];
        self.item_get_inf = [0; 4];
        self.inf_table = [0; 30];
        self.unk_f34 = [0; 0x4];
        self.world_map_area_data = 0;
        self.unk_f3c = [0; 0x4];
        self.scarecrow_long_song_set = 0;
        self.scarecrow_long_song = [0; 0x360];
        self.unk_12a1 = [0; 0x24];
        self.scarecrow_spawn_song_set = 0;
        self.scarecrow_spawn_song = [0; 0x80];
        self.unk_1346 = [0; 0x2];
        self.horse_data = HorseData::default();
        self.checksum = 0;
    }

    /// `Sram_InitNewSave` (`z_sram.c`): the save's info cleared, then `sNewSavePlayerData`,
    /// `sNewSaveEquips`, `sNewSaveInventory`, `sNewSaveChecksum` (0), Epona in Hyrule Field,
    /// `magicLevel` 0, `infTable[INFTABLE_INDEX_1DX]` 1 and the Water Temple's switch 0x1E.
    /// `entranceIndex`, `linkAge`, `cutsceneIndex`, `dayTime` and `nightFlag` are left as they
    /// were.
    pub fn init_new_save(&mut self) {
        self.clear_info();
        self.total_days = 0;
        self.bgs_day_count = 0;
        // sNewSavePlayerData: no newf, no deaths, the name all FILENAME_SPACE, three hearts, the
        // normal meter (level 0), no rupees, both ages' equips empty, Link's house.
        self.player_name = [FILENAME_SPACE; 8];
        self.health_capacity = 0x30;
        self.health = 0x30;
        self.magic = MAGIC_NORMAL_METER;
        self.child_equips = ItemEquips::NONE;
        self.adult_equips = ItemEquips::NONE;
        self.saved_scene_id = SCENE_LINKS_HOUSE;
        // sNewSaveEquips: nothing on the buttons, the Kokiri tunic and boots worn.
        self.equips = ItemEquips { button_items: [ITEM_NONE; 4], c_button_slots: [SLOT_NONE; 3], equipment: 0x1100 };
        // sNewSaveInventory: no items, the Kokiri tunic and boots owned, no keys (-1).
        self.inventory = Inventory {
            items: [ITEM_NONE; 24],
            ammo: [0; 16],
            equipment: owned_equip_flag(EQUIP_TYPE_TUNIC, EQUIP_INV_TUNIC_KOKIRI) | owned_equip_flag(EQUIP_TYPE_BOOTS, EQUIP_INV_BOOTS_KOKIRI),
            upgrades: 0,
            quest_items: 0,
            dungeon_items: [0; 20],
            dungeon_keys: [-1; 19],
            defense_hearts: 0,
            gs_tokens: 0,
        };
        self.checksum = 0;
        self.horse_data = SRAM_HORSE_DATA;
        self.magic_level = 0;
        self.inf_table[INFTABLE_INDEX_1DX] = 1;
        self.scene_flags[SCENE_WATER_TEMPLE].swch = 0x4000_0000;
    }

    /// `Sram_InitDebugSave` (`z_sram.c`): the save's info cleared, then `sDebugSavePlayerData`
    /// ("ZELDAZ", "LINK", 14 hearts, 150 rupees, `swordHealth` 8, magic acquired, Hyrule
    /// Field), `sDebugSaveEquips` (B the Master Sword, C the bow, bombs and the Fairy Ocarina;
    /// the Master Sword, Hylian Shield, Kokiri tunic and boots worn), `sDebugSaveInventory`
    /// (most items with ammo, every sword but the broken knife, every shield, tunic and boots,
    /// upgrades 0x125249, quest items 0x1E3FFFF, the first ten dungeons' items and 8 keys
    /// everywhere), Epona in Hyrule Field, the story flags; a child (`LINK_AGE_IN_YEARS`, read
    /// from `linkAge` as it stands) gets the Kokiri Sword on B and, with `fileNum` 0xFF (the map
    /// select's), the slingshot on C-Left and the Deku Shield. Then `ENTR_HYRULE_FIELD_0`,
    /// `magicLevel` 0 and the Water Temple's switch.
    pub fn init_debug_save(&mut self) {
        self.clear_info();
        self.total_days = 0;
        self.bgs_day_count = 0;
        self.newf = *b"ZELDAZ";
        // FILENAME_UPPERCASE('L'), 'I', 'N', 'K' (OOT_VERSION >= PAL_1_0), then FILENAME_SPACE.
        self.player_name = [0x15, 0x12, 0x17, 0x14, FILENAME_SPACE, FILENAME_SPACE, FILENAME_SPACE, FILENAME_SPACE];
        self.health_capacity = 0xE0;
        self.health = 0xE0;
        self.magic = MAGIC_NORMAL_METER;
        self.rupees = 150;
        self.sword_health = 8;
        self.is_magic_acquired = true;
        self.child_equips = ItemEquips::NONE;
        self.adult_equips = ItemEquips::NONE;
        self.saved_scene_id = crate::play_scene::SCENE_HYRULE_FIELD;
        self.equips = ItemEquips {
            button_items: [ITEM_SWORD_MASTER, ITEM_BOW, ITEM_BOMB, ITEM_OCARINA_FAIRY],
            c_button_slots: [SLOT_BOW as u8, SLOT_BOMB as u8, SLOT_OCARINA as u8],
            equipment: (EQUIP_VALUE_SWORD_MASTER << 0) | (EQUIP_VALUE_SHIELD_HYLIAN << 4) | (EQUIP_VALUE_TUNIC_KOKIRI << 8) | (EQUIP_VALUE_BOOTS_KOKIRI << 12),
        };
        self.inventory = Inventory {
            items: [
                ITEM_DEKU_STICK,
                ITEM_DEKU_NUT,
                ITEM_BOMB,
                ITEM_BOW,
                ITEM_ARROW_FIRE,
                ITEM_DINS_FIRE,
                ITEM_SLINGSHOT,
                ITEM_OCARINA_FAIRY,
                ITEM_BOMBCHU,
                ITEM_HOOKSHOT,
                ITEM_ARROW_ICE,
                ITEM_FARORES_WIND,
                ITEM_BOOMERANG,
                ITEM_LENS_OF_TRUTH,
                ITEM_MAGIC_BEAN,
                ITEM_HAMMER,
                ITEM_ARROW_LIGHT,
                ITEM_NAYRUS_LOVE,
                ITEM_BOTTLE_EMPTY,
                ITEM_BOTTLE_POTION_RED,
                ITEM_BOTTLE_POTION_GREEN,
                ITEM_BOTTLE_POTION_BLUE,
                ITEM_POCKET_EGG,
                ITEM_WEIRD_EGG,
            ],
            ammo: [50, 50, 10, 30, 1, 1, 30, 1, 50, 1, 1, 1, 1, 1, 1, 1],
            // Every sword but the broken knife, every shield, tunic and boots.
            equipment: 0x7 | (0x7 << 4) | (0x7 << 8) | (0x7 << 12),
            upgrades: 0x125249,
            quest_items: 0x1E3FFFF,
            dungeon_items: [7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            dungeon_keys: [8; 19],
            defense_hearts: 0,
            gs_tokens: 0,
        };
        self.checksum = 0;
        self.horse_data = SRAM_HORSE_DATA;
        // INFTABLE_SARIA_GREETED_LINK, _SARIA_WAS_TOLD_ABOUT_MIDO, INFTABLE_0C, INFTABLE_0E.
        self.inf_table[0] |= 0x5009;
        // EVENTCHKINF_00_UNUSED, _01_UNUSED, _MIDO_DENIED_DEKU_TREE_ACCESS,
        // _SARIA_WAS_TOLD_ABOUT_MIDO, _04, _05, _09, _0C.
        self.event_chk_inf[0] |= 0x123F;
        self.set_event_chk_inf(EVENTCHKINF_ZELDA_FLED_CASTLE);
        self.set_event_chk_inf(EVENTCHKINF_C4);
        if !self.adult {
            self.equips.button_items[0] = ITEM_SWORD_KOKIRI;
            self.inventory_change_equipment(EQUIP_TYPE_SWORD, EQUIP_VALUE_SWORD_KOKIRI);
            if self.file_num == 0xFF {
                self.equips.button_items[1] = ITEM_SLINGSHOT;
                self.equips.c_button_slots[0] = SLOT_SLINGSHOT as u8;
                self.inventory_change_equipment(EQUIP_TYPE_SHIELD, EQUIP_VALUE_SHIELD_DEKU);
            }
        }
        self.entrance_index = ENTR_HYRULE_FIELD_0;
        self.magic_level = 0;
        self.scene_flags[SCENE_WATER_TEMPLE].swch = 0x4000_0000;
    }

    /// A new file entering by `entrance_index`, Link `adult` or not, at `day_time`:
    /// `Sram_InitNewSave` (`z_sram.c`).
    /// - `sNewSavePlayerData`: three hearts, no rupees, the normal magic meter (level 0), both
    ///   ages' equips empty, `savedSceneId` `SCENE_LINKS_HOUSE`;
    /// - `sNewSaveEquips`: nothing on the buttons, equipment 0x1100 (the Kokiri tunic and boots
    ///   worn, no sword, no shield);
    /// - `sNewSaveInventory`: no items or ammo, the Kokiri tunic and boots owned, the dungeon
    ///   keys at -1;
    /// - `infTable[INFTABLE_INDEX_1DX] = 1`, `sceneFlags[5].swch = 0x40000000`.
    ///
    /// The name is "LINK": the file select's name entry (which would write it) isn't ported.
    pub fn new(entrance_index: u16, adult: bool, day_time: u16) -> SaveContext {
        let mut s = SaveContext::blank(entrance_index, adult, day_time);
        s.init_new_save();
        // FILENAME_UPPERCASE('L'), 'I', 'N', 'K': what the name entry would write.
        s.player_name = [0x15, 0x12, 0x17, 0x14, FILENAME_SPACE, FILENAME_SPACE, FILENAME_SPACE, FILENAME_SPACE];
        s
    }

    /// A new file as the file select starts it: file 2 (`fileNum` 1; the first file is the map
    /// select's in this debug ROM, `FS_BTN_SELECT_FILE_1`) made on a fresh SRAM as the name
    /// entry makes it ("LINK", `Sram_InitSave`), then loaded (`FileSelect_LoadGame`): see
    /// [`crate::file_select::new_game`].
    /// - `Sram_InitSave`: `Sram_InitNewSave`, then `ENTR_LINKS_HOUSE_0`, child, 10:00 and
    ///   `cutsceneIndex` 0xFFF1, the opening on Link's house's layer 5; "ZELDAZ";
    /// - `FileSelect_LoadGame`: `Sram_OpenSave` (Link's house from `savedSceneId`), `fileNum` 1,
    ///   `GAMEMODE_NORMAL`, `respawn[RESPAWN_MODE_DOWN]`'s entrance `ENTR_LOAD_OPENING` (-1),
    ///   `respawnFlag` 0, `showTitleCard`, the next transition, cutscene and trigger cleared,
    ///   every button enabled, the interface's alpha types, `magic` and `magicLevel` 0 (the meter
    ///   grows back), `naviTimer` 0, and the sword's equip with nothing on B.
    pub fn file_select_new() -> SaveContext {
        let mut sram = crate::sram::Sram::default();
        crate::file_select::new_game(&mut sram, 2, None).expect("a fresh SRAM's file 2 is empty")
    }

    /// The map select's file (`MapSelect_LoadGame` with `fileNum` 0xFF): `Sram_InitDebugSave`
    /// ([`Self::init_debug_save`]) with Link `adult` or not, the map select's entrance, then its
    /// magic reset and enabled buttons.
    pub fn debug(entrance_index: u16, adult: bool, day_time: u16) -> SaveContext {
        let mut s = SaveContext::blank(entrance_index, adult, day_time);
        s.file_num = 0xFF;
        s.init_debug_save();
        // MapSelect_LoadGame: the entrance picked; magicCapacity, magicLevel and magic to 0 (to
        // grow back), every button enabled, the interface's alpha types cleared.
        s.entrance_index = entrance_index;
        s.magic = 0;
        s.button_status = [crate::interface::BTN_ENABLED; 5];
        s
    }

    /// `CUR_CAPACITY(UPG_WALLET)`.
    pub fn wallet_capacity(&self) -> i16 {
        self.cur_capacity(UPG_WALLET) as i16
    }

    /// `CUR_UPG_VALUE(upg)`.
    pub fn cur_upg_value(&self, upg: usize) -> u32 {
        (self.inventory.upgrades & UPGRADE_MASKS[upg]) >> UPGRADE_SHIFTS[upg]
    }

    /// `CUR_CAPACITY(upg)`.
    pub fn cur_capacity(&self, upg: usize) -> u16 {
        capacity(upg, self.cur_upg_value(upg))
    }

    /// `Inventory_ChangeUpgrade`.
    pub fn inventory_change_upgrade(&mut self, upg: usize, value: u32) {
        self.inventory.upgrades &= !UPGRADE_MASKS[upg];
        self.inventory.upgrades |= value << UPGRADE_SHIFTS[upg];
    }

    /// `CUR_EQUIP_VALUE(equip)`.
    pub fn cur_equip_value(&self, equip: usize) -> u16 {
        (self.equips.equipment & EQUIP_MASKS[equip]) >> EQUIP_SHIFTS[equip]
    }

    /// `ALL_EQUIP_VALUE(equip)`: the owned pieces' bits.
    pub fn all_equip_value(&self, equip: usize) -> u16 {
        (self.inventory.equipment & EQUIP_MASKS[equip]) >> EQUIP_SHIFTS[equip]
    }

    /// `CHECK_OWNED_EQUIP(equip, value)`.
    pub fn check_owned_equip(&self, equip: usize, value: u16) -> bool {
        owned_equip_flag(equip, value) & self.inventory.equipment != 0
    }

    /// `Inventory_ChangeEquipment`.
    pub fn inventory_change_equipment(&mut self, equip: usize, value: u16) {
        self.equips.equipment &= EQUIP_NEG_MASKS[equip];
        self.equips.equipment |= value << EQUIP_SHIFTS[equip];
    }

    /// What the pause menu's equipment page does when A equips an owned piece
    /// (`KaleidoScope_DrawEquipment`, `z_kaleido_equipment.c`): `Inventory_ChangeEquipment`,
    /// and for a sword `infTable[INFTABLE_INDEX_1DX] = 0` and the sword on B
    /// (`gEquipAgeReqs`' age check, Biggoron's Sword and the broken knife's cases aside).
    /// The equipment page's contents aren't ported: this is its effect, for the presets and the
    /// stand-in (docs/adr/0047). `Player_SetEquipmentData` runs when the menu closes (Player reads
    /// the save).
    pub fn equip_from_pause_menu(&mut self, equip: usize, value: u16) {
        self.inventory_change_equipment(equip, value);
        if equip == EQUIP_TYPE_SWORD {
            self.inf_table[INFTABLE_INDEX_1DX] = 0;
            // pauseCtx->cursorItem[PAUSE_EQUIP]: ITEM_SWORD_KOKIRI + value - 1.
            self.equips.button_items[0] = ITEM_SWORD_KOKIRI + value as u8 - 1;
        }
    }

    /// The equipment page's stand-in, run as the pause menu resumes the game: every equipment
    /// type with nothing worn gets its first owned piece that Link's age can wear
    /// (`z_kaleido_equipment.c`: `CHECK_OWNED_EQUIP` and `gEquipAgeReqs`), as
    /// [`Self::equip_from_pause_menu`] equips it. Returns whether anything was equipped.
    /// (Pieces already worn are kept: the menu's choice between two owned pieces isn't made
    /// for the player.)
    pub fn equip_owned_unworn(&mut self) -> bool {
        let age = if self.adult { LINK_AGE_ADULT } else { LINK_AGE_CHILD };
        let mut any = false;
        for equip in [EQUIP_TYPE_SWORD, EQUIP_TYPE_SHIELD, EQUIP_TYPE_TUNIC, EQUIP_TYPE_BOOTS] {
            if self.cur_equip_value(equip) != 0 {
                continue;
            }
            let wearable = |v: u16| EQUIP_AGE_REQS[equip][v as usize] == 9 || EQUIP_AGE_REQS[equip][v as usize] == age;
            // CHECK_OWNED_EQUIP(cursorY, cursorX - 1): the flag is the value's, less one.
            if let Some(v) = (1..4u16).find(|&v| self.check_owned_equip(equip, v - 1) && wearable(v)) {
                self.equip_from_pause_menu(equip, v);
                any = true;
            }
        }
        any
    }

    /// What the pause menu's item screen does when C-Left equips `item` from its slot: see
    /// [`Self::equip_item_on_c`].
    pub fn equip_item_on_c_left(&mut self, item: u8) {
        self.equip_item_on_c(0, item);
    }

    /// What the pause menu's item page does when C button `target` (0 C-Left, 1 C-Down, 2
    /// C-Right) equips `item` from its slot, for the presets: the equip's end
    /// (`crate::kaleido::item::item_equip_write`, `KaleidoScope_UpdateItemEquip`) without the
    /// icon's flight.
    pub fn equip_item_on_c(&mut self, target: usize, item: u8) {
        crate::kaleido::item::item_equip_write(self, target, item as u16, slot(item) as u16);
    }

    /// `INV_CONTENT(item)`.
    pub fn inv_content(&self, item: u8) -> u8 {
        self.inventory.items.get(slot(item)).copied().unwrap_or(ITEM_NONE)
    }

    pub fn set_inv_content(&mut self, item: u8, v: u8) {
        if let Some(s) = self.inventory.items.get_mut(slot(item)) {
            *s = v;
        }
    }

    /// `AMMO(item)`.
    pub fn ammo(&self, item: u8) -> i8 {
        self.inventory.ammo.get(slot(item)).copied().unwrap_or(0)
    }

    /// `B_BTN_ITEM`: B's item, `ITEM_NONE` while B is disabled (`buttonStatus[0]` holds
    /// `ITEM_NONE` then), the broken knife as Biggoron's Sword.
    pub fn b_btn_item(&self) -> u8 {
        if self.button_status[0] == ITEM_NONE {
            ITEM_NONE
        } else if self.equips.button_items[0] == ITEM_GIANTS_KNIFE {
            ITEM_SWORD_BIGGORON
        } else {
            self.equips.button_items[0]
        }
    }

    /// `C_BTN_ITEM(button)` (0 C-left, 1 C-down, 2 C-right).
    pub fn c_btn_item(&self, button: usize) -> u8 {
        if self.button_status[button + 1] != crate::interface::BTN_DISABLED { self.equips.button_items[button + 1] } else { ITEM_NONE }
    }

    /// `GET_EVENTCHKINF` (`Flags_GetEventChkInf`).
    pub fn get_event_chk_inf(&self, flag: u16) -> bool {
        self.event_chk_inf[(flag >> 4) as usize] & (1 << (flag & 0xF)) != 0
    }

    /// `SET_EVENTCHKINF` (`Flags_SetEventChkInf`).
    pub fn set_event_chk_inf(&mut self, flag: u16) {
        self.event_chk_inf[(flag >> 4) as usize] |= 1 << (flag & 0xF);
    }

    /// `GET_ITEMGETINF`, `SET_ITEMGETINF`.
    pub fn get_item_get_inf(&self, flag: u16) -> bool {
        self.item_get_inf[(flag >> 4) as usize] & (1 << (flag & 0xF)) != 0
    }
    pub fn set_item_get_inf(&mut self, flag: u16) {
        self.item_get_inf[(flag >> 4) as usize] |= 1 << (flag & 0xF);
    }

    /// Applies the debug save preset `name` (see `SAVE_PRESETS`).
    pub fn apply_preset(&mut self, name: &str) -> Result<(), String> {
        let p = SAVE_PRESETS.iter().find(|p| p.name == name).ok_or_else(|| {
            let names: Vec<_> = SAVE_PRESETS.iter().map(|p| p.name).collect();
            format!("no save preset {name} (there are: {})", names.join(", "))
        })?;
        (p.apply)(self);
        Ok(())
    }

    /// `GET_INFTABLE`, `SET_INFTABLE`.
    pub fn get_inf_table(&self, flag: u16) -> bool {
        self.inf_table[(flag >> 4) as usize] & (1 << (flag & 0xF)) != 0
    }
    pub fn set_inf_table(&mut self, flag: u16) {
        self.inf_table[(flag >> 4) as usize] |= 1 << (flag & 0xF);
    }

    /// `CHECK_QUEST_ITEM` (`gBitFlags[item] & questItems`).
    pub fn check_quest_item(&self, item: u32) -> bool {
        self.inventory.quest_items & (1 << item) != 0
    }

    /// `sceneFlags[scene]` (a zeroed entry past the table).
    pub fn scene_flags(&self, scene: u16) -> SavedSceneFlags {
        self.scene_flags.get(scene as usize).copied().unwrap_or_default()
    }

    /// `IS_DAY`.
    pub fn is_day(&self) -> bool {
        !self.night_flag
    }

    /// `GET_GS_FLAGS(index)` (`save.h`): `gsFlags[index >> 2]`'s byte `index & 3`
    /// (`gGsFlagsMasks`, `gGsFlagsShifts`).
    pub fn get_gs_flags(&self, index: i32) -> u32 {
        (self.gs_flags[(index >> 2) as usize] as u32 & GS_FLAGS_MASKS[(index & 3) as usize]) >> GS_FLAGS_SHIFTS[(index & 3) as usize]
    }

    /// `SET_GS_FLAGS(index, value)` (`save.h`): `value` or'ed into that byte.
    pub fn set_gs_flags(&mut self, index: i32, value: i32) {
        self.gs_flags[(index >> 2) as usize] |= value << GS_FLAGS_SHIFTS[(index & 3) as usize];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_file_has_nothing_to_fight_with() {
        let s = SaveContext::new(0, false, 0);
        // sNewSaveEquips: equipment 0x1100, nothing on B.
        assert_eq!((s.equips.equipment, s.equips.button_items[0], s.b_btn_item()), (0x1100, ITEM_NONE, ITEM_NONE));
        assert_eq!((s.cur_equip_value(EQUIP_TYPE_SWORD), s.cur_equip_value(EQUIP_TYPE_SHIELD)), (0, 0));
        assert_eq!((s.cur_equip_value(EQUIP_TYPE_TUNIC), s.cur_equip_value(EQUIP_TYPE_BOOTS)), (1, 1));
        // sNewSaveInventory: the Kokiri tunic and boots owned (bits 8 and 12).
        assert_eq!(s.inventory.equipment, 0x1100);
        assert_eq!((s.health, s.rupees, s.inf_table[INFTABLE_INDEX_1DX]), (0x30, 0, 1));
    }

    #[test]
    fn the_file_selects_new_file_starts_the_opening() {
        let s = SaveContext::file_select_new();
        // Sram_InitSave (file 2): ENTR_LINKS_HOUSE_0, child, CLOCK_TIME(10, 0), 0xFFF1.
        assert_eq!((s.entrance_index, s.adult, s.day_time, s.cutscene_index), (ENTR_LINKS_HOUSE_0, false, 0x6AAB, 0xFFF1));
        // Sram_InitNewSave's file, then FileSelect_LoadGame's resets.
        let n = SaveContext::new(ENTR_LINKS_HOUSE_0, false, 0x6AAB);
        assert_eq!((s.equips, s.inventory.clone(), s.health), (n.equips, n.inventory.clone(), n.health));
        assert_eq!((s.file_num, s.game_mode, s.respawn[RESPAWN_MODE_DOWN].entrance_index, s.respawn_flag), (1, GAMEMODE_NORMAL, ENTR_LOAD_OPENING, 0));
        assert_eq!((s.next_cutscene_index, s.cutscene_trigger, s.navi_timer, s.magic, s.prev_hud_visibility_mode), (0xFFEF, 0, 0, 0, 0x32));
        assert_eq!(s.button_status, [crate::interface::BTN_ENABLED; 5]);
    }

    #[test]
    fn the_map_selects_child_has_the_kokiri_sword_and_the_deku_shield() {
        let s = SaveContext::debug(0, false, 0);
        assert_eq!(s.b_btn_item(), ITEM_SWORD_KOKIRI);
        assert_eq!((s.cur_equip_value(EQUIP_TYPE_SWORD), s.cur_equip_value(EQUIP_TYPE_SHIELD)), (EQUIP_VALUE_SWORD_KOKIRI, EQUIP_VALUE_SHIELD_DEKU));
        assert_eq!((s.equips.button_items[1], s.c_btn_item(0)), (ITEM_SLINGSHOT, ITEM_SLINGSHOT));
        let a = SaveContext::debug(0, true, 0);
        assert_eq!((a.b_btn_item(), a.cur_equip_value(EQUIP_TYPE_SHIELD)), (ITEM_SWORD_MASTER, EQUIP_VALUE_SHIELD_HYLIAN));
        // eventChkInf[0] |= 0x123F: EVENTCHKINF_05 and EVENTCHKINF_0C among them.
        assert!(s.get_event_chk_inf(EVENTCHKINF_05) && s.get_event_chk_inf(EVENTCHKINF_0C));
    }

    #[test]
    fn the_deku_tree_presets_own_and_wear_the_sword_and_shield() {
        let mut s = SaveContext::new(0, false, 0);
        s.apply_preset("deku-tree-open").unwrap();
        assert_eq!(s.inventory.equipment & 0xFF, 0x11);
        assert_eq!(s.equips.equipment, 0x1111);
        assert_eq!((s.equips.button_items[0], s.inf_table[INFTABLE_INDEX_1DX]), (ITEM_SWORD_KOKIRI, 0));
    }
}
