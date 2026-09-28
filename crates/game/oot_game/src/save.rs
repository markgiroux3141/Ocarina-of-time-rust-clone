//! The parts of `gSaveContext` (`z64save.h`) that play reads and writes: where the next
//! `Play_Init` enters, Link's age, the time of day, the respawn points, and what one scene
//! passes to the next (the entrance speed, the transition type). It outlives each
//! `PlayState`: a scene change builds a new one from it, as `Play_Init` does.

use glam::Vec3;

/// `RESPAWN_MODE_*`.
pub const RESPAWN_MODE_DOWN: usize = 0;
pub const RESPAWN_MODE_RETURN: usize = 1;
pub const RESPAWN_MODE_TOP: usize = 2;

/// `TRANS_NEXT_TYPE_DEFAULT`: take the transition type from the entrance table.
pub const TRANS_NEXT_TYPE_DEFAULT: u8 = 0xFF;

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
    /// `nextTransitionType` (`TRANS_NEXT_TYPE_DEFAULT` for the entrance table's).
    pub next_transition_type: u8,
    /// `transFadeDuration`, `transWipeSpeed`.
    pub trans_fade_duration: u16,
    pub trans_wipe_speed: u8,
    /// `retainWeatherMode`, `showTitleCard`.
    pub retain_weather_mode: bool,
    pub show_title_card: bool,
    /// `cutsceneIndex`: never a cutscene here (0).
    pub cutscene_index: u16,
    /// `eventChkInf`, `itemGetInf`, `infTable`: the story and conversation flags (all clear
    /// on a new save).
    pub event_chk_inf: [u16; 14],
    pub item_get_inf: [u16; 4],
    pub inf_table: [u16; 30],
    /// `inventory.questItems` (`QUEST_*` bits).
    pub quest_items: u32,
    /// `playerName`, in the file select's character codes.
    pub player_name: [u8; 8],
    /// `healthCapacity`, `health` (16 a heart), `healthAccumulator` (health still to add).
    pub health_capacity: i16,
    pub health: i16,
    pub health_accumulator: i16,
    /// `inventory.defenseHearts`.
    pub defense_hearts: u8,
    /// `rupees`, `rupeeAccumulator` (rupees still to add or take).
    pub rupees: i16,
    pub rupee_accumulator: i16,
    /// `inventory.upgrades`' wallet (`CUR_UPG_VALUE(UPG_WALLET)`).
    pub wallet_upgrade: u8,
    /// `equips.buttonItems` (B, C-left, C-down, C-right; `ITEM_NONE` when empty).
    pub button_items: [u8; 4],
    /// `buttonStatus` (B, the C buttons, A; `BTN_ENABLED` 0, `BTN_DISABLED` 0xFF).
    pub button_status: [u8; 5],
    /// `unk_13E7`, `unk_13E8`, `unk_13EA`, `unk_13EC`, `unk_13EE`: the interface's alpha
    /// type (`Interface_ChangeAlpha`), its fade step and the type to go back to.
    pub unk_13e7: u8,
    pub unk_13e8: u16,
    pub unk_13ea: u16,
    pub unk_13ec: u16,
    pub unk_13ee: u16,
    /// `inventory.collectibles`-style per-scene flags aren't kept; `language` is English.
    pub language: u8,
}

/// `ITEM_SWORD_KOKIRI`, `ITEM_SWORD_MASTER`, `ITEM_NONE` (`z64item.h`).
pub const ITEM_SWORD_KOKIRI: u8 = 0x3B;
pub const ITEM_SWORD_MASTER: u8 = 0x3C;
pub const ITEM_NONE: u8 = 0xFF;
/// `LANGUAGE_ENG`.
pub const LANGUAGE_ENG: u8 = 0;

/// `EVENTCHKINF_40`: Zelda's letter obtained (`(4 << 4) | 0`).
pub const EVENTCHKINF_40: u16 = 0x40;
/// `QUEST_MEDALLION_FOREST`, `QUEST_KOKIRI_EMERALD` (`z64item.h`).
pub const QUEST_MEDALLION_FOREST: u32 = 0x00;
pub const QUEST_KOKIRI_EMERALD: u32 = 0x12;

impl Default for SaveContext {
    fn default() -> SaveContext {
        SaveContext::new(0, false, 0)
    }
}

impl SaveContext {
    /// A save entering by `entrance_index`, Link `adult` or not, at `day_time`.
    pub fn new(entrance_index: u16, adult: bool, day_time: u16) -> SaveContext {
        SaveContext {
            entrance_index,
            scene_layer: 0,
            adult,
            day_time,
            night_flag: false,
            respawn: [RespawnData::default(); 3],
            respawn_flag: 0,
            entrance_speed: 0.0,
            next_transition_type: TRANS_NEXT_TYPE_DEFAULT,
            trans_fade_duration: 0,
            trans_wipe_speed: 0,
            retain_weather_mode: false,
            show_title_card: true,
            cutscene_index: 0,
            event_chk_inf: [0; 14],
            item_get_inf: [0; 4],
            inf_table: [0; 30],
            quest_items: 0,
            // Sram_InitNewSave's sNewSavePlayerData: three hearts, no rupees, the child's
            // wallet. The name is the debug save's "LINK".
            player_name: [0x15, 0x12, 0x17, 0x14, 0x3E, 0x3E, 0x3E, 0x3E],
            health_capacity: 0x30,
            health: 0x30,
            health_accumulator: 0,
            defense_hearts: 0,
            rupees: 0,
            rupee_accumulator: 0,
            wallet_upgrade: 0,
            // The sword Player always carries (Sram_InitDebugSave gives the child the Kokiri
            // Sword on B; the adult has the Master Sword).
            button_items: [if adult { ITEM_SWORD_MASTER } else { ITEM_SWORD_KOKIRI }, ITEM_NONE, ITEM_NONE, ITEM_NONE],
            button_status: [0; 5],
            unk_13e7: 0,
            unk_13e8: 0,
            unk_13ea: 0,
            unk_13ec: 0,
            unk_13ee: 0,
            language: LANGUAGE_ENG,
        }
    }

    /// `CUR_CAPACITY(UPG_WALLET)` (`gUpgradeCapacities`, `code_80097A00.c`).
    pub fn wallet_capacity(&self) -> i16 {
        [99, 200, 500, 500][self.wallet_upgrade.min(3) as usize]
    }

    /// `GET_EVENTCHKINF`.
    pub fn get_event_chk_inf(&self, flag: u16) -> bool {
        self.event_chk_inf[(flag >> 4) as usize] & (1 << (flag & 0xF)) != 0
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
        self.quest_items & (1 << item) != 0
    }

    /// `IS_DAY`.
    pub fn is_day(&self) -> bool {
        !self.night_flag
    }
}
