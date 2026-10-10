//! Items: the inventory's tables (`z_inventory.c`), giving items to Link (`Item_Give`,
//! `Item_CheckObtainability`, `Health_ChangeBy`, `Rupees_ChangeBy`, `Inventory_ChangeAmmo` in
//! `z_parameter.c`), the get-item table Player reads (`sGetItemTable` in `z_player.c`) and the
//! get-item models' draw table (`sDrawItemTable` in `z_draw.c`), both read by the importer into
//! `table/items`, and the tables random drops pick from (`sItemDropIds`, `sDropQuantities` in
//! `z_en_item00.c`, in `table/item_drops`).
//!
//! `Item_Give` is ported whole. What it can't do here: `Interface_LoadItemIcon1/2` load
//! nothing (every button icon is a bake, picked by the item when the HUD draws), the water
//! medallion's `Horse_FixLakeHyliaPosition` (the Water Temple's water level) and the magic jars'
//! `Magic_Fill`/`Magic_RequestChange` (no magic meter) are logged.
//!
//! The audio both take is the play state's (`None` where there's none to play on: the debug
//! save presets, `Play_Init`'s triggers, tests).

use crate::audio::GameAudio;
use crate::save::SaveContext;

// `ItemID` (item.h).
pub const ITEM_DEKU_STICK: u8 = 0x00;
pub const ITEM_DEKU_NUT: u8 = 0x01;
pub const ITEM_BOMB: u8 = 0x02;
pub const ITEM_BOW: u8 = 0x03;
pub const ITEM_ARROW_FIRE: u8 = 0x04;
pub const ITEM_DINS_FIRE: u8 = 0x05;
pub const ITEM_SLINGSHOT: u8 = 0x06;
pub const ITEM_OCARINA_FAIRY: u8 = 0x07;
pub const ITEM_OCARINA_OF_TIME: u8 = 0x08;
pub const ITEM_BOMBCHU: u8 = 0x09;
pub const ITEM_HOOKSHOT: u8 = 0x0A;
pub const ITEM_LONGSHOT: u8 = 0x0B;
pub const ITEM_ARROW_ICE: u8 = 0x0C;
pub const ITEM_FARORES_WIND: u8 = 0x0D;
pub const ITEM_BOOMERANG: u8 = 0x0E;
pub const ITEM_LENS_OF_TRUTH: u8 = 0x0F;
pub const ITEM_MAGIC_BEAN: u8 = 0x10;
pub const ITEM_HAMMER: u8 = 0x11;
pub const ITEM_ARROW_LIGHT: u8 = 0x12;
pub const ITEM_NAYRUS_LOVE: u8 = 0x13;
pub const ITEM_BOTTLE_EMPTY: u8 = 0x14;
pub const ITEM_BOTTLE_POTION_RED: u8 = 0x15;
pub const ITEM_BOTTLE_POTION_GREEN: u8 = 0x16;
pub const ITEM_BOTTLE_POTION_BLUE: u8 = 0x17;
pub const ITEM_BOTTLE_FAIRY: u8 = 0x18;
pub const ITEM_BOTTLE_FISH: u8 = 0x19;
pub const ITEM_BOTTLE_MILK_FULL: u8 = 0x1A;
pub const ITEM_BOTTLE_RUTOS_LETTER: u8 = 0x1B;
pub const ITEM_BOTTLE_BLUE_FIRE: u8 = 0x1C;
pub const ITEM_BOTTLE_BUG: u8 = 0x1D;
pub const ITEM_BOTTLE_BIG_POE: u8 = 0x1E;
pub const ITEM_BOTTLE_MILK_HALF: u8 = 0x1F;
pub const ITEM_BOTTLE_POE: u8 = 0x20;
pub const ITEM_WEIRD_EGG: u8 = 0x21;
pub const ITEM_CHICKEN: u8 = 0x22;
pub const ITEM_ZELDAS_LETTER: u8 = 0x23;
pub const ITEM_MASK_KEATON: u8 = 0x24;
pub const ITEM_SOLD_OUT: u8 = 0x2C;
pub const ITEM_POCKET_EGG: u8 = 0x2D;
pub const ITEM_POCKET_CUCCO: u8 = 0x2E;
pub const ITEM_POACHERS_SAW: u8 = 0x32;
pub const ITEM_CLAIM_CHECK: u8 = 0x37;
pub const ITEM_BOW_FIRE: u8 = 0x38;
pub const ITEM_BOW_LIGHT: u8 = 0x3A;
pub const ITEM_SWORD_KOKIRI: u8 = 0x3B;
pub const ITEM_SWORD_MASTER: u8 = 0x3C;
pub const ITEM_SWORD_BIGGORON: u8 = 0x3D;
pub const ITEM_SHIELD_DEKU: u8 = 0x3E;
pub const ITEM_SHIELD_HYLIAN: u8 = 0x3F;
pub const ITEM_SHIELD_MIRROR: u8 = 0x40;
pub const ITEM_TUNIC_KOKIRI: u8 = 0x41;
pub const ITEM_TUNIC_ZORA: u8 = 0x43;
pub const ITEM_BOOTS_KOKIRI: u8 = 0x44;
pub const ITEM_BOOTS_HOVER: u8 = 0x46;
pub const ITEM_BULLET_BAG_30: u8 = 0x47;
pub const ITEM_BULLET_BAG_40: u8 = 0x48;
pub const ITEM_BULLET_BAG_50: u8 = 0x49;
pub const ITEM_QUIVER_30: u8 = 0x4A;
pub const ITEM_QUIVER_40: u8 = 0x4B;
pub const ITEM_QUIVER_50: u8 = 0x4C;
pub const ITEM_BOMB_BAG_20: u8 = 0x4D;
pub const ITEM_BOMB_BAG_30: u8 = 0x4E;
pub const ITEM_BOMB_BAG_40: u8 = 0x4F;
pub const ITEM_STRENGTH_GORONS_BRACELET: u8 = 0x50;
pub const ITEM_STRENGTH_SILVER_GAUNTLETS: u8 = 0x51;
pub const ITEM_STRENGTH_GOLD_GAUNTLETS: u8 = 0x52;
pub const ITEM_SCALE_SILVER: u8 = 0x53;
pub const ITEM_SCALE_GOLDEN: u8 = 0x54;
pub const ITEM_GIANTS_KNIFE: u8 = 0x55;
pub const ITEM_ADULTS_WALLET: u8 = 0x56;
pub const ITEM_GIANTS_WALLET: u8 = 0x57;
pub const ITEM_DEKU_SEEDS: u8 = 0x58;
pub const ITEM_FISHING_POLE: u8 = 0x59;
pub const ITEM_SONG_MINUET: u8 = 0x5A;
pub const ITEM_SONG_STORMS: u8 = 0x65;
pub const ITEM_MEDALLION_FOREST: u8 = 0x66;
pub const ITEM_MEDALLION_WATER: u8 = 0x68;
pub const ITEM_MEDALLION_LIGHT: u8 = 0x6B;
pub const ITEM_KOKIRI_EMERALD: u8 = 0x6C;
pub const ITEM_ZORA_SAPPHIRE: u8 = 0x6E;
pub const ITEM_STONE_OF_AGONY: u8 = 0x6F;
pub const ITEM_GERUDOS_CARD: u8 = 0x70;
pub const ITEM_SKULL_TOKEN: u8 = 0x71;
pub const ITEM_HEART_CONTAINER: u8 = 0x72;
pub const ITEM_HEART_PIECE: u8 = 0x73;
pub const ITEM_DUNGEON_BOSS_KEY: u8 = 0x74;
pub const ITEM_DUNGEON_COMPASS: u8 = 0x75;
pub const ITEM_DUNGEON_MAP: u8 = 0x76;
pub const ITEM_SMALL_KEY: u8 = 0x77;
pub const ITEM_MAGIC_JAR_SMALL: u8 = 0x78;
pub const ITEM_MAGIC_JAR_BIG: u8 = 0x79;
pub const ITEM_HEART_PIECE_2: u8 = 0x7A;
pub const ITEM_MILK: u8 = 0x82;
pub const ITEM_RECOVERY_HEART: u8 = 0x83;
pub const ITEM_RUPEE_GREEN: u8 = 0x84;
pub const ITEM_RUPEE_BLUE: u8 = 0x85;
pub const ITEM_RUPEE_RED: u8 = 0x86;
pub const ITEM_RUPEE_PURPLE: u8 = 0x87;
pub const ITEM_RUPEE_GOLD: u8 = 0x88;
pub const ITEM_INVALID_8: u8 = 0x89;
pub const ITEM_DEKU_STICKS_5: u8 = 0x8A;
pub const ITEM_DEKU_STICKS_10: u8 = 0x8B;
pub const ITEM_DEKU_NUTS_5: u8 = 0x8C;
pub const ITEM_DEKU_NUTS_10: u8 = 0x8D;
pub const ITEM_BOMBS_5: u8 = 0x8E;
pub const ITEM_BOMBS_30: u8 = 0x91;
pub const ITEM_ARROWS_5: u8 = 0x92;
pub const ITEM_ARROWS_10: u8 = 0x93;
pub const ITEM_ARROWS_30: u8 = 0x94;
pub const ITEM_DEKU_SEEDS_30: u8 = 0x95;
pub const ITEM_BOMBCHUS_5: u8 = 0x96;
pub const ITEM_BOMBCHUS_20: u8 = 0x97;
pub const ITEM_DEKU_STICK_UPGRADE_20: u8 = 0x98;
pub const ITEM_DEKU_STICK_UPGRADE_30: u8 = 0x99;
pub const ITEM_DEKU_NUT_UPGRADE_30: u8 = 0x9A;
pub const ITEM_DEKU_NUT_UPGRADE_40: u8 = 0x9B;
pub const ITEM_NONE_FE: u8 = 0xFE;
pub const ITEM_NONE: u8 = 0xFF;

// `GetItemID` (item.h): the ones the ported code names.
pub const GI_NONE: i16 = 0x00;
pub const GI_DEKU_NUTS_5: i16 = 0x02;
pub const GI_DEKU_STICKS_1: i16 = 0x07;
pub const GI_SWORD_KOKIRI: i16 = 0x27;
pub const GI_SHIELD_DEKU: i16 = 0x29;
pub const GI_SHIELD_HYLIAN: i16 = 0x2A;
pub const GI_TUNIC_GORON: i16 = 0x2C;
pub const GI_TUNIC_ZORA: i16 = 0x2D;
pub const GI_SILVER_GAUNTLETS: i16 = 0x35;
pub const GI_DEKU_SEEDS_5: i16 = 0x3C;
pub const GI_HEART_CONTAINER: i16 = 0x3D;
pub const GI_HEART_PIECE: i16 = 0x3E;
pub const GI_SMALL_KEY: i16 = 0x42;
pub const GI_MAGIC_JAR_SMALL: i16 = 0x43;
pub const GI_MAGIC_JAR_LARGE: i16 = 0x44;
pub const GI_RECOVERY_HEART: i16 = 0x48;
pub const GI_RUPEE_GREEN: i16 = 0x4C;
pub const GI_RUPEE_BLUE: i16 = 0x4D;
pub const GI_RUPEE_RED: i16 = 0x4E;
pub const GI_HEART_CONTAINER_2: i16 = 0x4F;
pub const GI_RUPEE_PURPLE: i16 = 0x55;
pub const GI_RUPEE_GOLD: i16 = 0x56;
pub const GI_RUPEE_GREEN_LOSE: i16 = 0x72;
pub const GI_RUPEE_PURPLE_LOSE: i16 = 0x75;
pub const GI_ICE_TRAP: i16 = 0x7C;
pub const GI_MAX: i16 = 0x7E;

// `GetItemDrawID` (item.h): the ones the ported code names.
pub const GID_RECOVERY_HEART: i16 = 0x08;
pub const GID_SHIELD_DEKU: i16 = 0x1C;
pub const GID_SHIELD_HYLIAN: i16 = 0x2B;
pub const GID_TUNIC_GORON: i16 = 0x3B;
pub const GID_TUNIC_ZORA: i16 = 0x3C;

// `EquipmentType`, `EquipValue*`, `EquipInv*` (item.h).
pub const EQUIP_TYPE_SWORD: usize = 0;
pub const EQUIP_TYPE_SHIELD: usize = 1;
pub const EQUIP_TYPE_TUNIC: usize = 2;
pub const EQUIP_TYPE_BOOTS: usize = 3;
pub const EQUIP_VALUE_SWORD_NONE: u16 = 0;
pub const EQUIP_VALUE_SWORD_KOKIRI: u16 = 1;
pub const EQUIP_VALUE_SWORD_MASTER: u16 = 2;
pub const EQUIP_VALUE_SHIELD_NONE: u16 = 0;
pub const EQUIP_VALUE_SHIELD_DEKU: u16 = 1;
pub const EQUIP_VALUE_SHIELD_HYLIAN: u16 = 2;
pub const EQUIP_VALUE_SHIELD_MIRROR: u16 = 3;
pub const EQUIP_VALUE_TUNIC_KOKIRI: u16 = 1;
pub const EQUIP_VALUE_BOOTS_KOKIRI: u16 = 1;
pub const EQUIP_INV_SWORD_KOKIRI: u16 = 0;
pub const EQUIP_INV_SWORD_MASTER: u16 = 1;
pub const EQUIP_INV_SWORD_BIGGORON: u16 = 2;
pub const EQUIP_INV_SWORD_BROKENGIANTKNIFE: u16 = 3;
pub const EQUIP_INV_SHIELD_DEKU: u16 = 0;
pub const EQUIP_INV_SHIELD_HYLIAN: u16 = 1;
pub const EQUIP_INV_SHIELD_MIRROR: u16 = 2;
pub const EQUIP_INV_TUNIC_KOKIRI: u16 = 0;
pub const EQUIP_INV_TUNIC_GORON: u16 = 1;
pub const EQUIP_INV_TUNIC_ZORA: u16 = 2;
pub const EQUIP_INV_BOOTS_KOKIRI: u16 = 0;
pub const EQUIP_INV_BOOTS_IRON: u16 = 1;
pub const EQUIP_INV_BOOTS_HOVER: u16 = 2;

// `UpgradeType` (item.h).
pub const UPG_QUIVER: usize = 0;
pub const UPG_BOMB_BAG: usize = 1;
pub const UPG_STRENGTH: usize = 2;
pub const UPG_SCALE: usize = 3;
pub const UPG_WALLET: usize = 4;
pub const UPG_BULLET_BAG: usize = 5;
pub const UPG_DEKU_STICKS: usize = 6;
pub const UPG_DEKU_NUTS: usize = 7;

/// `QUEST_*` (item.h).
pub const QUEST_MEDALLION_FOREST: u32 = 0x00;
pub const QUEST_SONG_MINUET: u32 = 0x06;
pub const QUEST_SONG_LULLABY: u32 = 0x0C;
pub const QUEST_SONG_SARIA: u32 = 0x0E;
pub const QUEST_KOKIRI_EMERALD: u32 = 0x12;
pub const QUEST_GORON_RUBY: u32 = 0x13;
pub const QUEST_ZORA_SAPPHIRE: u32 = 0x14;
pub const QUEST_STONE_OF_AGONY: u32 = 0x15;
pub const QUEST_SKULL_TOKEN: u32 = 0x17;
/// `QUEST_HEART_PIECE_COUNT`: the pieces are counted in `questItems`' top four bits.
pub const QUEST_HEART_PIECE_COUNT: u32 = 0x1C;

// `InventorySlot` (item.h).
pub const SLOT_DEKU_STICK: usize = 0x00;
pub const SLOT_DEKU_NUT: usize = 0x01;
pub const SLOT_BOMB: usize = 0x02;
pub const SLOT_BOW: usize = 0x03;
pub const SLOT_SLINGSHOT: usize = 0x06;
pub const SLOT_OCARINA: usize = 0x07;
pub const SLOT_BOMBCHU: usize = 0x08;
pub const SLOT_BOTTLE_1: usize = 0x12;
pub const SLOT_TRADE_ADULT: usize = 0x16;
pub const SLOT_TRADE_CHILD: usize = 0x17;
pub const SLOT_NONE: u8 = 0xFF;

/// `gEquipMasks`, `gEquipNegMasks`, `gEquipShifts` (`z_inventory.c`): a nibble per equipment
/// type.
pub const EQUIP_MASKS: [u16; 4] = [0xF, 0xF << 4, 0xF << 8, 0xF << 12];
pub const EQUIP_NEG_MASKS: [u16; 4] = [!0xF, !(0xF << 4), !(0xF << 8), !(0xF << 12)];
pub const EQUIP_SHIFTS: [u8; 4] = [0, 4, 8, 12];
/// `gUpgradeMasks`, `gUpgradeShifts`, `gUpgradeCapacities` (`z_inventory.c`).
pub const UPGRADE_MASKS: [u32; 8] = [0x0000_0007, 0x0000_0038, 0x0000_01C0, 0x0000_0E00, 0x0000_3000, 0x0001_C000, 0x000E_0000, 0x0070_0000];
pub const UPGRADE_SHIFTS: [u8; 8] = [0, 3, 6, 9, 12, 14, 17, 20];
pub const UPGRADE_CAPACITIES: [[u16; 4]; 8] = [
    [0, 30, 40, 50],     // UPG_QUIVER
    [0, 20, 30, 40],     // UPG_BOMB_BAG
    [0, 0, 0, 0],        // UPG_STRENGTH (unused)
    [0, 0, 0, 0],        // UPG_SCALE (unused)
    [99, 200, 500, 500], // UPG_WALLET
    [0, 30, 40, 50],     // UPG_BULLET_BAG
    [0, 10, 20, 30],     // UPG_DEKU_STICKS
    [0, 20, 30, 40],     // UPG_DEKU_NUTS
];

/// `gItemSlots` (`z_inventory.c`): each item's inventory slot, up to `ITEM_CLAIM_CHECK`.
pub const ITEM_SLOTS: [u8; 56] = {
    const T: u8 = SLOT_TRADE_CHILD as u8;
    const A: u8 = SLOT_TRADE_ADULT as u8;
    const B: u8 = SLOT_BOTTLE_1 as u8;
    [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, // stick, nut, bomb, bow, fire arrow, Din's fire
        0x06, 0x07, 0x07, 0x08, 0x09, 0x09, // slingshot, ocarinas, bombchu, hookshot, longshot
        0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, // ice arrow, Farore's, boomerang, lens, bean, hammer
        0x10, 0x11, B, B, B, B, // light arrow, Nayru's, bottle, the potions
        B, B, B, B, B, B, // fairy, fish, milk, Ruto's letter, blue fire, bug
        B, B, B, T, T, T, // big poe, half milk, poe, weird egg, chicken, Zelda's letter
        T, T, T, T, T, T, // the masks
        T, T, T, A, A, A, // the masks, sold out, pocket egg, pocket cucco, Cojiro
        A, A, A, A, A, A, // odd mushroom .. eyeball frog
        A, A, // eye drops, claim check
    ]
};

/// `sExtraItemBases` (`z_parameter.c`): the item whose slot a refill (from `ITEM_DEKU_STICKS_5`)
/// counts in.
const EXTRA_ITEM_BASES: [u8; 18] = [
    ITEM_DEKU_STICK, ITEM_DEKU_STICK, ITEM_DEKU_NUT, ITEM_DEKU_NUT, ITEM_BOMB, ITEM_BOMB, ITEM_BOMB, ITEM_BOMB, ITEM_BOW, ITEM_BOW, ITEM_BOW, ITEM_DEKU_SEEDS, ITEM_BOMBCHU, ITEM_BOMBCHU, ITEM_DEKU_STICK, ITEM_DEKU_STICK, ITEM_DEKU_NUT,
    ITEM_DEKU_NUT,
];

/// `SLOT(item)`. `gItemSlots` has entries up to `ITEM_CLAIM_CHECK`; the C reads past it for
/// later items (only to compute a slot `Item_Give` doesn't use), which gives `SLOT_NONE` here.
pub fn slot(item: u8) -> usize {
    ITEM_SLOTS.get(item as usize).map(|&s| s as usize).unwrap_or(SLOT_NONE as usize)
}

/// `sItemDropIds` and `sDropQuantities` (16 per drop table), as `Item_DropCollectibleRandom`
/// indexes them.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ItemDropTables {
    pub ids: Vec<u8>,
    pub quantities: Vec<u8>,
}

/// `GetItemEntry` (`z_player.c`): what a get-item id gives.
#[derive(Debug, Clone, Copy, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GetItemEntry {
    /// `itemId` (`ITEM_*`).
    pub item_id: u8,
    /// `field`: bits 0..4 the drop's `Item00Type` (`func_8083E4C4`), 0x20 a blue rupee if
    /// already had, 0x40 a blue rupee if not obtainable, 0x80 given without a drop.
    pub field: u8,
    /// `gi`: the draw id plus one, negative for the short chest animation (`CHEST_ANIM_SHORT`).
    pub gi: i8,
    /// `textId`.
    pub text_id: u8,
    /// `objectId` (`OBJECT_*`, `OBJECT_INVALID` 0 for none).
    pub object_id: i16,
}

/// `DrawItemTableEntry` (`z_draw.c`): a get-item model's draw function (its name,
/// `GetItem_Draw*`) and display lists (`gGi*DL` symbols).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DrawItemEntry {
    pub func: String,
    pub dlists: Vec<String>,
}

/// `table/items`: `sGetItemTable` (index `getItemId - 1`) and `sDrawItemTable` (by
/// `GetItemDrawID`).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ItemTables {
    pub get_items: Vec<GetItemEntry>,
    pub draw_items: Vec<DrawItemEntry>,
}

impl ItemTables {
    /// `sGetItemTable[getItemId - 1]`.
    pub fn get_item(&self, get_item_id: i16) -> Option<&GetItemEntry> {
        usize::try_from(get_item_id - 1).ok().and_then(|i| self.get_items.get(i))
    }
}

/// `Health_ChangeBy`: false once Link is out of health. A gain plays the recovery sound;
/// damage is halved with double defence (`isDoubleDefenseAcquired`).
pub fn health_change_by(save: &mut SaveContext, audio: Option<&mut GameAudio>, mut amount: i16) -> bool {
    if amount > 0 {
        if let Some(a) = audio {
            a.play_sfx_centered(crate::audio::sfx::NA_SE_SY_HP_RECOVER);
        }
    } else if save.is_double_defense_acquired && amount < 0 {
        amount >>= 1;
    }
    save.health += amount;
    if save.health > save.health_capacity {
        save.health = save.health_capacity;
    }
    if save.health <= 0 {
        save.health = 0;
        return false;
    }
    true
}

/// `Inventory_ConsumeFairy` (`z_parameter.c`): a bottled fairy is used up (the first bottle
/// holding one; the first C button holding one first, whose bottle then empties). True if
/// there was one. (`Interface_LoadItemIcon1`: the icons are drawn from the items.)
pub fn inventory_consume_fairy(save: &mut SaveContext) -> bool {
    let mut bottle_slot = slot(ITEM_BOTTLE_FAIRY);
    for mut i in 0..4 {
        if save.inventory.items[bottle_slot + i] == ITEM_BOTTLE_FAIRY {
            for j in 1..4 {
                if save.equips.button_items[j] == ITEM_BOTTLE_FAIRY {
                    save.equips.button_items[j] = ITEM_BOTTLE_EMPTY;
                    i = 0;
                    bottle_slot = save.equips.c_button_slots[j - 1] as usize;
                    break;
                }
            }
            save.inventory.items[bottle_slot + i] = ITEM_BOTTLE_EMPTY;
            return true;
        }
    }
    false
}

/// `Inventory_DeleteEquipment` (`z_inventory.c`): the piece of `equipment` worn is taken off and
/// out of the inventory (the Kokiri tunic is put on in a tunic's place; a sword's loss empties
/// B). Returns the piece's value (0 for none). The caller runs `Player_SetEquipmentData`, as
/// the C does here. (`pauseCtx.cursorSpecialPos`: no pause menu.)
pub fn inventory_delete_equipment(save: &mut SaveContext, equipment: usize) -> u16 {
    let mut equip_value = save.equips.equipment & EQUIP_MASKS[equipment];
    if equip_value != 0 {
        equip_value >>= EQUIP_SHIFTS[equipment];
        save.equips.equipment &= EQUIP_NEG_MASKS[equipment];
        save.inventory.equipment ^= owned_equip_flag(equipment, equip_value - 1);
        if equipment == EQUIP_TYPE_TUNIC {
            save.equips.equipment |= EQUIP_VALUE_TUNIC_KOKIRI << (EQUIP_TYPE_TUNIC * 4);
        }
        if equipment == EQUIP_TYPE_SWORD {
            save.equips.button_items[0] = ITEM_NONE;
            save.inf_table[crate::save::INFTABLE_INDEX_1DX] = 1;
        }
    }
    equip_value
}

/// `Rupees_ChangeBy`: counted in by `Interface_Update`.
pub fn rupees_change_by(save: &mut SaveContext, rupee_change: i16) {
    save.rupee_accumulator += rupee_change;
}

/// `Inventory_ChangeAmmo`.
pub fn inventory_change_ammo(save: &mut SaveContext, item: u8, ammo_change: i16) {
    let (upg, clamp_low) = match item {
        ITEM_DEKU_STICK => (Some(UPG_DEKU_STICKS), true),
        ITEM_DEKU_NUT => (Some(UPG_DEKU_NUTS), true),
        ITEM_BOMBCHU => (None, true),
        ITEM_BOW => (Some(UPG_QUIVER), true),
        ITEM_SLINGSHOT => (Some(UPG_BULLET_BAG), true),
        ITEM_BOMB => (Some(UPG_BOMB_BAG), true),
        ITEM_MAGIC_BEAN => (None, false),
        _ => return,
    };
    let s = slot(item);
    let mut v = save.inventory.ammo[s] as i16 + ammo_change;
    match upg {
        Some(u) => {
            let cap = save.cur_capacity(u) as i16;
            if v >= cap {
                v = cap;
            } else if clamp_low && v < 0 {
                v = 0;
            }
        }
        None if item == ITEM_BOMBCHU => {
            if v >= 50 {
                v = 50;
            } else if v < 0 {
                v = 0;
            }
        }
        None => {}
    }
    save.inventory.ammo[s] = v as i8;
    if item == ITEM_MAGIC_BEAN {
        // BEANS_BOUGHT (AMMO(ITEM_MAGIC_BEAN + 1)) is left alone.
    }
}

/// `Item_Give`: returns what the C returns (`ITEM_NONE` when the item was taken in full, the
/// item or what its slot held otherwise).
pub fn item_give(save: &mut SaveContext, audio: Option<&mut GameAudio>, mut item: u8) -> u8 {
    /// `sAmmoRefillCounts` (sticks, nuts, bombs), `sArrowRefillCounts`,
    /// `sBombchuRefillCounts`, `sRupeeRefillCounts`.
    const AMMO_REFILL_COUNTS: [i16; 4] = [5, 10, 20, 30];
    const ARROW_REFILL_COUNTS: [i16; 3] = [5, 10, 30];
    const BOMBCHU_REFILL_COUNTS: [i16; 2] = [5, 20];
    const RUPEE_REFILL_COUNTS: [i16; 6] = [1, 5, 20, 50, 200, 10];

    let mut slot_i = slot(item);
    if item >= ITEM_DEKU_STICKS_5 {
        if let Some(&base) = EXTRA_ITEM_BASES.get((item - ITEM_DEKU_STICKS_5) as usize) {
            slot_i = slot(base);
        }
    }
    let inv = |s: &SaveContext, i: usize| s.inventory.items.get(i).copied().unwrap_or(ITEM_NONE);
    fn ammo(s: &mut SaveContext, it: u8) -> &mut i8 {
        &mut s.inventory.ammo[slot(it)]
    }

    if (ITEM_MEDALLION_FOREST..=ITEM_MEDALLION_LIGHT).contains(&item) {
        save.inventory.quest_items |= 1 << (item - ITEM_MEDALLION_FOREST) as u32 + QUEST_MEDALLION_FOREST;
        if item == ITEM_MEDALLION_WATER {
            log::debug!("Item_Give: Horse_FixLakeHyliaPosition (the Water Temple's water) isn't ported");
        }
        return ITEM_NONE;
    } else if (ITEM_SONG_MINUET..=ITEM_SONG_STORMS).contains(&item) {
        save.inventory.quest_items |= 1 << ((item - ITEM_SONG_MINUET) as u32 + QUEST_SONG_MINUET);
        return ITEM_NONE;
    } else if (ITEM_KOKIRI_EMERALD..=ITEM_ZORA_SAPPHIRE).contains(&item) {
        save.inventory.quest_items |= 1 << ((item - ITEM_KOKIRI_EMERALD) as u32 + QUEST_KOKIRI_EMERALD);
        return ITEM_NONE;
    } else if item == ITEM_STONE_OF_AGONY || item == ITEM_GERUDOS_CARD {
        save.inventory.quest_items |= 1 << ((item - ITEM_STONE_OF_AGONY) as u32 + QUEST_STONE_OF_AGONY);
        return ITEM_NONE;
    } else if item == ITEM_SKULL_TOKEN {
        save.inventory.quest_items |= 1 << QUEST_SKULL_TOKEN;
        save.inventory.gs_tokens += 1;
        return ITEM_NONE;
    } else if (ITEM_SWORD_KOKIRI..=ITEM_SWORD_BIGGORON).contains(&item) {
        save.inventory.equipment |= owned_equip_flag(EQUIP_TYPE_SWORD, (item - ITEM_SWORD_KOKIRI) as u16 + EQUIP_INV_SWORD_KOKIRI);
        if item == ITEM_SWORD_BIGGORON {
            save.sword_health = 8;
            let all = (1 << EQUIP_INV_SWORD_KOKIRI) | (1 << EQUIP_INV_SWORD_MASTER) | (1 << EQUIP_INV_SWORD_BIGGORON) | (1 << EQUIP_INV_SWORD_BROKENGIANTKNIFE);
            if save.all_equip_value(EQUIP_TYPE_SWORD) == all {
                save.inventory.equipment ^= (1 << EQUIP_INV_SWORD_BROKENGIANTKNIFE) << EQUIP_SHIFTS[EQUIP_TYPE_SWORD];
                if save.equips.button_items[0] == ITEM_GIANTS_KNIFE {
                    save.equips.button_items[0] = ITEM_SWORD_BIGGORON;
                }
            }
        } else if item == ITEM_SWORD_MASTER {
            save.equips.button_items[0] = ITEM_SWORD_MASTER;
            save.equips.equipment &= !(0xF << (EQUIP_TYPE_SWORD * 4));
            save.equips.equipment |= EQUIP_VALUE_SWORD_MASTER << (EQUIP_TYPE_SWORD * 4);
        }
        return ITEM_NONE;
    } else if (ITEM_SHIELD_DEKU..=ITEM_SHIELD_MIRROR).contains(&item) {
        save.inventory.equipment |= owned_equip_flag(EQUIP_TYPE_SHIELD, (item - ITEM_SHIELD_DEKU) as u16);
        return ITEM_NONE;
    } else if (ITEM_TUNIC_KOKIRI..=ITEM_TUNIC_ZORA).contains(&item) {
        save.inventory.equipment |= owned_equip_flag(EQUIP_TYPE_TUNIC, (item - ITEM_TUNIC_KOKIRI) as u16);
        return ITEM_NONE;
    } else if (ITEM_BOOTS_KOKIRI..=ITEM_BOOTS_HOVER).contains(&item) {
        save.inventory.equipment |= owned_equip_flag(EQUIP_TYPE_BOOTS, (item - ITEM_BOOTS_KOKIRI) as u16);
        return ITEM_NONE;
    } else if item == ITEM_DUNGEON_BOSS_KEY || item == ITEM_DUNGEON_COMPASS || item == ITEM_DUNGEON_MAP {
        let m = save.map_index as usize;
        if let Some(d) = save.inventory.dungeon_items.get_mut(m) {
            *d |= 1 << (item - ITEM_DUNGEON_BOSS_KEY);
        }
        return ITEM_NONE;
    } else if item == ITEM_SMALL_KEY {
        let m = save.map_index as usize;
        if let Some(k) = save.inventory.dungeon_keys.get_mut(m) {
            if *k < 0 {
                *k = 1;
            } else {
                *k += 1;
            }
        }
        return ITEM_NONE;
    } else if item == ITEM_QUIVER_30 || item == ITEM_BOW {
        if save.cur_upg_value(UPG_QUIVER) == 0 {
            save.inventory_change_upgrade(UPG_QUIVER, 1);
            save.set_inv_content(ITEM_BOW, ITEM_BOW);
            *ammo(save, ITEM_BOW) = capacity(UPG_QUIVER, 1) as i8;
            return ITEM_NONE;
        } else {
            *ammo(save, ITEM_BOW) += 1;
            let cap = save.cur_capacity(UPG_QUIVER) as i8;
            if *ammo(save, ITEM_BOW) > cap {
                *ammo(save, ITEM_BOW) = cap;
            }
        }
    } else if item == ITEM_QUIVER_40 || item == ITEM_QUIVER_50 {
        let v = if item == ITEM_QUIVER_40 { 2 } else { 3 };
        save.inventory_change_upgrade(UPG_QUIVER, v);
        *ammo(save, ITEM_BOW) = capacity(UPG_QUIVER, v) as i8;
        return ITEM_NONE;
    } else if item == ITEM_BULLET_BAG_40 || item == ITEM_BULLET_BAG_50 {
        let v = if item == ITEM_BULLET_BAG_40 { 2 } else { 3 };
        save.inventory_change_upgrade(UPG_BULLET_BAG, v);
        *ammo(save, ITEM_SLINGSHOT) = capacity(UPG_BULLET_BAG, v) as i8;
        return ITEM_NONE;
    } else if item == ITEM_BOMB_BAG_20 {
        if save.cur_upg_value(UPG_BOMB_BAG) == 0 {
            save.inventory_change_upgrade(UPG_BOMB_BAG, 1);
            save.set_inv_content(ITEM_BOMB, ITEM_BOMB);
            *ammo(save, ITEM_BOMB) = capacity(UPG_BOMB_BAG, 1) as i8;
            return ITEM_NONE;
        } else {
            *ammo(save, ITEM_BOMB) += 1;
            let cap = save.cur_capacity(UPG_BOMB_BAG) as i8;
            if *ammo(save, ITEM_BOMB) > cap {
                *ammo(save, ITEM_BOMB) = cap;
            }
        }
    } else if item == ITEM_BOMB_BAG_30 || item == ITEM_BOMB_BAG_40 {
        let v = if item == ITEM_BOMB_BAG_30 { 2 } else { 3 };
        save.inventory_change_upgrade(UPG_BOMB_BAG, v);
        *ammo(save, ITEM_BOMB) = capacity(UPG_BOMB_BAG, v) as i8;
        return ITEM_NONE;
    } else if (ITEM_STRENGTH_GORONS_BRACELET..=ITEM_STRENGTH_GOLD_GAUNTLETS).contains(&item) {
        save.inventory_change_upgrade(UPG_STRENGTH, (item - ITEM_STRENGTH_GORONS_BRACELET + 1) as u32);
        return ITEM_NONE;
    } else if item == ITEM_SCALE_SILVER || item == ITEM_SCALE_GOLDEN {
        save.inventory_change_upgrade(UPG_SCALE, (item - ITEM_SCALE_SILVER + 1) as u32);
        return ITEM_NONE;
    } else if item == ITEM_ADULTS_WALLET || item == ITEM_GIANTS_WALLET {
        save.inventory_change_upgrade(UPG_WALLET, (item - ITEM_ADULTS_WALLET + 1) as u32);
        return ITEM_NONE;
    } else if item == ITEM_DEKU_STICK_UPGRADE_20 || item == ITEM_DEKU_STICK_UPGRADE_30 {
        if inv(save, slot_i) == ITEM_NONE {
            save.set_inv_content(ITEM_DEKU_STICK, ITEM_DEKU_STICK);
        }
        let v = if item == ITEM_DEKU_STICK_UPGRADE_20 { 2 } else { 3 };
        save.inventory_change_upgrade(UPG_DEKU_STICKS, v);
        *ammo(save, ITEM_DEKU_STICK) = capacity(UPG_DEKU_STICKS, v) as i8;
        return ITEM_NONE;
    } else if item == ITEM_DEKU_NUT_UPGRADE_30 || item == ITEM_DEKU_NUT_UPGRADE_40 {
        if inv(save, slot_i) == ITEM_NONE {
            save.set_inv_content(ITEM_DEKU_NUT, ITEM_DEKU_NUT);
        }
        let v = if item == ITEM_DEKU_NUT_UPGRADE_30 { 2 } else { 3 };
        save.inventory_change_upgrade(UPG_DEKU_NUTS, v);
        *ammo(save, ITEM_DEKU_NUT) = capacity(UPG_DEKU_NUTS, v) as i8;
        return ITEM_NONE;
    } else if item == ITEM_LONGSHOT {
        save.set_inv_content(item, item);
        for b in &mut save.equips.button_items[1..] {
            if *b == ITEM_HOOKSHOT {
                *b = ITEM_LONGSHOT;
            }
        }
        return ITEM_NONE;
    } else if item == ITEM_DEKU_STICK {
        if inv(save, slot_i) == ITEM_NONE {
            save.inventory_change_upgrade(UPG_DEKU_STICKS, 1);
            *ammo(save, ITEM_DEKU_STICK) = 1;
        } else {
            *ammo(save, ITEM_DEKU_STICK) += 1;
            let cap = save.cur_capacity(UPG_DEKU_STICKS) as i8;
            if *ammo(save, ITEM_DEKU_STICK) > cap {
                *ammo(save, ITEM_DEKU_STICK) = cap;
            }
        }
    } else if item == ITEM_DEKU_STICKS_5 || item == ITEM_DEKU_STICKS_10 {
        let n = AMMO_REFILL_COUNTS[(item - ITEM_DEKU_STICKS_5) as usize] as i8;
        if inv(save, slot_i) == ITEM_NONE {
            save.inventory_change_upgrade(UPG_DEKU_STICKS, 1);
            *ammo(save, ITEM_DEKU_STICK) = n;
        } else {
            *ammo(save, ITEM_DEKU_STICK) += n;
            let cap = save.cur_capacity(UPG_DEKU_STICKS) as i8;
            if *ammo(save, ITEM_DEKU_STICK) > cap {
                *ammo(save, ITEM_DEKU_STICK) = cap;
            }
        }
        item = ITEM_DEKU_STICK;
    } else if item == ITEM_DEKU_NUT {
        if inv(save, slot_i) == ITEM_NONE {
            save.inventory_change_upgrade(UPG_DEKU_NUTS, 1);
            // @bug (game): AMMO(ITEM_DEKU_NUT) = ITEM_DEKU_NUT, which happens to be the one nut.
            *ammo(save, ITEM_DEKU_NUT) = ITEM_DEKU_NUT as i8;
        } else {
            *ammo(save, ITEM_DEKU_NUT) += 1;
            let cap = save.cur_capacity(UPG_DEKU_NUTS) as i8;
            if *ammo(save, ITEM_DEKU_NUT) > cap {
                *ammo(save, ITEM_DEKU_NUT) = cap;
            }
        }
    } else if item == ITEM_DEKU_NUTS_5 || item == ITEM_DEKU_NUTS_10 {
        // sAmmoRefillCounts[item - ITEM_DEKU_NUTS_5]: 5 and 10.
        let n = AMMO_REFILL_COUNTS[(item - ITEM_DEKU_NUTS_5) as usize] as i8;
        if inv(save, slot_i) == ITEM_NONE {
            save.inventory_change_upgrade(UPG_DEKU_NUTS, 1);
            *ammo(save, ITEM_DEKU_NUT) += n;
        } else {
            *ammo(save, ITEM_DEKU_NUT) += n;
            let cap = save.cur_capacity(UPG_DEKU_NUTS) as i8;
            if *ammo(save, ITEM_DEKU_NUT) > cap {
                *ammo(save, ITEM_DEKU_NUT) = cap;
            }
        }
        item = ITEM_DEKU_NUT;
    } else if item == ITEM_BOMB {
        *ammo(save, ITEM_BOMB) += 1;
        let cap = save.cur_capacity(UPG_BOMB_BAG) as i8;
        if *ammo(save, ITEM_BOMB) > cap {
            *ammo(save, ITEM_BOMB) = cap;
        }
        return ITEM_NONE;
    } else if (ITEM_BOMBS_5..=ITEM_BOMBS_30).contains(&item) {
        *ammo(save, ITEM_BOMB) += AMMO_REFILL_COUNTS[(item - ITEM_BOMBS_5) as usize] as i8;
        let cap = save.cur_capacity(UPG_BOMB_BAG) as i8;
        if *ammo(save, ITEM_BOMB) > cap {
            *ammo(save, ITEM_BOMB) = cap;
        }
        return ITEM_NONE;
    } else if item == ITEM_BOMBCHU {
        if inv(save, slot_i) == ITEM_NONE {
            save.set_inv_content(ITEM_BOMBCHU, ITEM_BOMBCHU);
            *ammo(save, ITEM_BOMBCHU) = 10;
        } else {
            *ammo(save, ITEM_BOMBCHU) += 10;
            if *ammo(save, ITEM_BOMBCHU) > 50 {
                *ammo(save, ITEM_BOMBCHU) = 50;
            }
        }
        return ITEM_NONE;
    } else if item == ITEM_BOMBCHUS_5 || item == ITEM_BOMBCHUS_20 {
        let n = BOMBCHU_REFILL_COUNTS[(item - ITEM_BOMBCHUS_5) as usize] as i8;
        if inv(save, slot_i) == ITEM_NONE {
            save.set_inv_content(ITEM_BOMBCHU, ITEM_BOMBCHU);
            *ammo(save, ITEM_BOMBCHU) += n;
        } else {
            *ammo(save, ITEM_BOMBCHU) += n;
            if *ammo(save, ITEM_BOMBCHU) > 50 {
                *ammo(save, ITEM_BOMBCHU) = 50;
            }
        }
        return ITEM_NONE;
    } else if (ITEM_ARROWS_5..=ITEM_ARROWS_30).contains(&item) {
        *ammo(save, ITEM_BOW) = ammo(save, ITEM_BOW).wrapping_add(ARROW_REFILL_COUNTS[(item - ITEM_ARROWS_5) as usize] as i8);
        let cap = save.cur_capacity(UPG_QUIVER) as i8;
        if *ammo(save, ITEM_BOW) >= cap || *ammo(save, ITEM_BOW) < 0 {
            *ammo(save, ITEM_BOW) = cap;
        }
        return ITEM_BOW;
    } else if item == ITEM_SLINGSHOT {
        save.inventory_change_upgrade(UPG_BULLET_BAG, 1);
        save.set_inv_content(ITEM_SLINGSHOT, ITEM_SLINGSHOT);
        *ammo(save, ITEM_SLINGSHOT) = 30;
        return ITEM_NONE;
    } else if item == ITEM_DEKU_SEEDS || item == ITEM_DEKU_SEEDS_30 {
        *ammo(save, ITEM_SLINGSHOT) += if item == ITEM_DEKU_SEEDS { 5 } else { 30 };
        let cap = save.cur_capacity(UPG_BULLET_BAG) as i8;
        if *ammo(save, ITEM_SLINGSHOT) >= cap {
            *ammo(save, ITEM_SLINGSHOT) = cap;
        }
        if !save.get_item_get_inf(ITEMGETINF_13) {
            save.set_item_get_inf(ITEMGETINF_13);
            return ITEM_NONE;
        }
        return ITEM_DEKU_SEEDS;
    } else if item == ITEM_OCARINA_FAIRY {
        save.set_inv_content(ITEM_OCARINA_FAIRY, ITEM_OCARINA_FAIRY);
        return ITEM_NONE;
    } else if item == ITEM_OCARINA_OF_TIME {
        save.set_inv_content(ITEM_OCARINA_OF_TIME, ITEM_OCARINA_OF_TIME);
        for b in &mut save.equips.button_items[1..] {
            if *b == ITEM_OCARINA_FAIRY {
                *b = ITEM_OCARINA_OF_TIME;
            }
        }
        return ITEM_NONE;
    } else if item == ITEM_MAGIC_BEAN {
        if inv(save, slot_i) == ITEM_NONE {
            save.set_inv_content(item, item);
            *ammo(save, ITEM_MAGIC_BEAN) = 1;
            // BEANS_BOUGHT: AMMO(ITEM_MAGIC_BEAN + 1).
            *ammo(save, ITEM_MAGIC_BEAN + 1) = 1;
        } else {
            *ammo(save, ITEM_MAGIC_BEAN) += 1;
            *ammo(save, ITEM_MAGIC_BEAN + 1) += 1;
        }
        return ITEM_NONE;
    } else if item == ITEM_HEART_PIECE_2 || item == ITEM_HEART_PIECE {
        save.inventory.quest_items = save.inventory.quest_items.wrapping_add(1 << QUEST_HEART_PIECE_COUNT);
        return ITEM_NONE;
    } else if item == ITEM_HEART_CONTAINER {
        save.health_capacity += 0x10;
        save.health += 0x10;
        return ITEM_NONE;
    } else if item == ITEM_RECOVERY_HEART {
        health_change_by(save, audio, 0x10);
        return item;
    } else if item == ITEM_MAGIC_JAR_SMALL || item == ITEM_MAGIC_JAR_BIG {
        // Magic_Fill / Magic_RequestChange(12 or 24, MAGIC_ADD): no magic meter.
        log::debug!("Item_Give({item:#04x}): the magic meter isn't ported");
        if !save.get_inf_table(INFTABLE_198) {
            save.set_inf_table(INFTABLE_198);
            return ITEM_NONE;
        }
        return item;
    } else if (ITEM_RUPEE_GREEN..=ITEM_INVALID_8).contains(&item) {
        rupees_change_by(save, RUPEE_REFILL_COUNTS[(item - ITEM_RUPEE_GREEN) as usize]);
        return ITEM_NONE;
    } else if item == ITEM_BOTTLE_EMPTY {
        let t = slot(item);
        for i in 0..4 {
            if save.inventory.items[t + i] == ITEM_NONE {
                save.inventory.items[t + i] = item;
                return ITEM_NONE;
            }
        }
    } else if (ITEM_BOTTLE_POTION_RED..=ITEM_BOTTLE_POE).contains(&item) || item == ITEM_MILK {
        let mut t = slot(item);
        if item != ITEM_BOTTLE_MILK_FULL && item != ITEM_BOTTLE_RUTOS_LETTER {
            if item == ITEM_MILK {
                item = ITEM_BOTTLE_MILK_FULL;
                t = slot(item);
            }
            for i in 0..4 {
                if save.inventory.items[t + i] == ITEM_BOTTLE_EMPTY {
                    // The C button showing that bottle gets the new contents.
                    for b in 0..3 {
                        if (t + i) as u8 == save.equips.c_button_slots[b] {
                            save.equips.button_items[b + 1] = item;
                            save.button_status[b + 1] = crate::interface::BTN_ENABLED;
                            break;
                        }
                    }
                    save.inventory.items[t + i] = item;
                    return ITEM_NONE;
                }
            }
        } else {
            for i in 0..4 {
                if save.inventory.items[t + i] == ITEM_NONE {
                    save.inventory.items[t + i] = item;
                    return ITEM_NONE;
                }
            }
        }
    } else if (ITEM_WEIRD_EGG..=ITEM_CLAIM_CHECK).contains(&item) {
        if item == ITEM_POACHERS_SAW {
            save.set_item_get_inf(ITEMGETINF_FOREST_STAGE_NUT_UPGRADE);
        }
        let temp = save.inv_content(item);
        save.set_inv_content(item, item);
        if temp != ITEM_NONE {
            for b in &mut save.equips.button_items[1..] {
                if temp == *b {
                    *b = if item != ITEM_SOLD_OUT { item } else { ITEM_NONE };
                    return ITEM_NONE;
                }
            }
        }
        return ITEM_NONE;
    }
    let temp = inv(save, slot_i);
    save.set_inv_content(item, item);
    temp
}

/// `Item_CheckObtainability`: `ITEM_NONE` if giving the item would give something new, else
/// what the C returns (the item, 0, or the slot's contents).
pub fn item_check_obtainability(save: &SaveContext, item: u8) -> u8 {
    let mut slot_i = slot(item);
    if item >= ITEM_DEKU_STICKS_5 {
        if let Some(&base) = EXTRA_ITEM_BASES.get((item - ITEM_DEKU_STICKS_5) as usize) {
            slot_i = slot(base);
        }
    }
    let owned = |t: usize, v: u16| save.check_owned_equip(t, v);
    if (ITEM_MEDALLION_FOREST..=ITEM_MEDALLION_LIGHT).contains(&item) {
        return ITEM_NONE;
    } else if (ITEM_KOKIRI_EMERALD..=ITEM_SKULL_TOKEN).contains(&item) {
        return ITEM_NONE;
    } else if (ITEM_SWORD_KOKIRI..=ITEM_SWORD_BIGGORON).contains(&item) {
        if item == ITEM_SWORD_BIGGORON {
            return ITEM_NONE;
        }
        return if owned(EQUIP_TYPE_SWORD, (item - ITEM_SWORD_KOKIRI) as u16 + EQUIP_INV_SWORD_KOKIRI) { item } else { ITEM_NONE };
    } else if (ITEM_SHIELD_DEKU..=ITEM_SHIELD_MIRROR).contains(&item) {
        return if owned(EQUIP_TYPE_SHIELD, (item - ITEM_SHIELD_DEKU) as u16 + EQUIP_INV_SHIELD_DEKU) { item } else { ITEM_NONE };
    } else if (ITEM_TUNIC_KOKIRI..=ITEM_TUNIC_ZORA).contains(&item) {
        return if owned(EQUIP_TYPE_TUNIC, (item - ITEM_TUNIC_KOKIRI) as u16 + EQUIP_INV_TUNIC_KOKIRI) { item } else { ITEM_NONE };
    } else if (ITEM_BOOTS_KOKIRI..=ITEM_BOOTS_HOVER).contains(&item) {
        return if owned(EQUIP_TYPE_BOOTS, (item - ITEM_BOOTS_KOKIRI) as u16 + EQUIP_INV_BOOTS_KOKIRI) { item } else { ITEM_NONE };
    } else if item == ITEM_DUNGEON_BOSS_KEY || item == ITEM_DUNGEON_COMPASS || item == ITEM_DUNGEON_MAP || item == ITEM_SMALL_KEY {
        return ITEM_NONE;
    } else if (ITEM_SLINGSHOT..=ITEM_BOMBCHU).contains(&item) || item == ITEM_BOMBCHUS_5 || item == ITEM_BOMBCHUS_20 {
        return ITEM_NONE;
    } else if item == ITEM_QUIVER_30 || item == ITEM_BOW {
        return if save.cur_upg_value(UPG_QUIVER) == 0 { ITEM_NONE } else { 0 };
    } else if item == ITEM_QUIVER_40 || item == ITEM_QUIVER_50 || item == ITEM_BULLET_BAG_40 || item == ITEM_BULLET_BAG_50 {
        return ITEM_NONE;
    } else if item == ITEM_BOMB_BAG_20 || item == ITEM_BOMB {
        return if save.cur_upg_value(UPG_BOMB_BAG) == 0 { ITEM_NONE } else { 0 };
    } else if (ITEM_DEKU_STICK_UPGRADE_20..=ITEM_DEKU_NUT_UPGRADE_40).contains(&item) || (ITEM_BOMB_BAG_30..=ITEM_GIANTS_WALLET).contains(&item) || item == ITEM_LONGSHOT {
        return ITEM_NONE;
    } else if item == ITEM_DEKU_SEEDS || item == ITEM_DEKU_SEEDS_30 {
        return if !save.get_item_get_inf(ITEMGETINF_13) { ITEM_NONE } else { ITEM_DEKU_SEEDS };
    } else if item == ITEM_MAGIC_BEAN || item == ITEM_HEART_PIECE_2 || item == ITEM_HEART_PIECE || item == ITEM_HEART_CONTAINER {
        return ITEM_NONE;
    } else if item == ITEM_RECOVERY_HEART {
        return ITEM_RECOVERY_HEART;
    } else if item == ITEM_MAGIC_JAR_SMALL || item == ITEM_MAGIC_JAR_BIG {
        return if !save.get_inf_table(INFTABLE_198) { ITEM_NONE } else { item };
    } else if (ITEM_RUPEE_GREEN..=ITEM_INVALID_8).contains(&item) || item == ITEM_BOTTLE_EMPTY {
        return ITEM_NONE;
    } else if (ITEM_BOTTLE_POTION_RED..=ITEM_BOTTLE_POE).contains(&item) || item == ITEM_MILK {
        let (mut it, mut t) = (item, slot(item));
        if it != ITEM_BOTTLE_MILK_FULL && it != ITEM_BOTTLE_RUTOS_LETTER {
            if it == ITEM_MILK {
                it = ITEM_BOTTLE_MILK_FULL;
                t = slot(it);
            }
            let _ = it;
            if (0..4).any(|i| save.inventory.items[t + i] == ITEM_BOTTLE_EMPTY) {
                return ITEM_NONE;
            }
        } else if (0..4).any(|i| save.inventory.items[t + i] == ITEM_NONE) {
            return ITEM_NONE;
        }
    } else if (ITEM_WEIRD_EGG..=ITEM_CLAIM_CHECK).contains(&item) {
        return ITEM_NONE;
    }
    save.inventory.items.get(slot_i).copied().unwrap_or(ITEM_NONE)
}

/// `OWNED_EQUIP_FLAG(equip, value)`: `gBitFlags[value] << gEquipShifts[equip]`.
/// `Inventory_ReplaceItem` (`z_parameter.c`): the first `old_item` in the inventory becomes
/// `new_item`, on the first C button holding it too. True if one was there.
/// (`Interface_LoadItemIcon1`: the icons are drawn from the items.)
pub fn inventory_replace_item(save: &mut SaveContext, old_item: u8, new_item: u8) -> bool {
    let Some(i) = save.inventory.items.iter().position(|&it| it == old_item) else { return false };
    save.inventory.items[i] = new_item;
    log::debug!("Item Purge ({i})");
    if let Some(b) = (1..4).find(|&b| save.equips.button_items[b] == old_item) {
        save.equips.button_items[b] = new_item;
    }
    true
}

/// `Inventory_HasEmptyBottle` (`z_parameter.c`): an empty bottle in one of the four bottle slots.
pub fn inventory_has_empty_bottle(save: &SaveContext) -> bool {
    (0..4).any(|i| save.inventory.items.get(SLOT_BOTTLE_1 + i).copied() == Some(ITEM_BOTTLE_EMPTY))
}

/// `func_800849EC` (`z_parameter.c`): the Giant's Knife bought (`EnGirlA_ItemGive_Longsword`):
/// owned, the broken knife's bit toggled, and B gets whichever the bits say. (The B icon's
/// reload, `Interface_LoadItemIcon1`, is the HUD's own each frame.)
pub fn func_800849ec(save: &mut SaveContext) {
    save.inventory.equipment |= owned_equip_flag(EQUIP_TYPE_SWORD, EQUIP_INV_SWORD_BIGGORON);
    save.inventory.equipment ^= (1 << EQUIP_INV_SWORD_BROKENGIANTKNIFE) << EQUIP_SHIFTS[EQUIP_TYPE_SWORD];
    save.equips.button_items[0] = if save.check_owned_equip(EQUIP_TYPE_SWORD, EQUIP_INV_SWORD_BROKENGIANTKNIFE) { ITEM_GIANTS_KNIFE } else { ITEM_SWORD_BIGGORON };
}

pub fn owned_equip_flag(equip: usize, value: u16) -> u16 {
    (1u16 << value) << EQUIP_SHIFTS[equip]
}

/// `CAPACITY(upg, value)`.
pub fn capacity(upg: usize, value: u32) -> u16 {
    UPGRADE_CAPACITIES[upg][value.min(3) as usize]
}

/// `ITEMGETINF_13` (`save.h`: the first Deku seeds).
pub const ITEMGETINF_13: u16 = 0x13;
/// `ITEMGETINF_FOREST_STAGE_NUT_UPGRADE`: the Poacher's Saw.
pub const ITEMGETINF_FOREST_STAGE_NUT_UPGRADE: u16 = 0x1F;
/// `INFTABLE_198` (`save.h`): the first magic jar.
pub const INFTABLE_198: u16 = 0x198;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rupees_and_hearts() {
        let mut s = SaveContext::new(0, false, 0);
        // A red rupee is 20 (sRupeeRefillCounts[2]), counted in later.
        assert_eq!(item_give(&mut s, None, ITEM_RUPEE_RED), ITEM_NONE);
        assert_eq!((s.rupees, s.rupee_accumulator), (0, 20));
        // A recovery heart at full health changes nothing but is still "given" (returns it).
        assert_eq!(item_give(&mut s, None, ITEM_RECOVERY_HEART), ITEM_RECOVERY_HEART);
        assert_eq!(s.health, 0x30);
        s.health = 0x18;
        item_give(&mut s, None, ITEM_RECOVERY_HEART);
        assert_eq!(s.health, 0x28);
        // Four pieces count in questItems' top bits.
        item_give(&mut s, None, ITEM_HEART_PIECE);
        assert_eq!(s.inventory.quest_items >> QUEST_HEART_PIECE_COUNT, 1);
    }

    #[test]
    fn the_kokiri_swords_equipment() {
        let mut s = SaveContext::new(0, false, 0);
        // A new save owns the Kokiri tunic and boots only (sNewSaveInventory).
        assert_eq!(item_check_obtainability(&s, ITEM_SWORD_KOKIRI), ITEM_NONE);
        assert_eq!(item_give(&mut s, None, ITEM_SWORD_KOKIRI), ITEM_NONE);
        // OWNED_EQUIP_FLAG(EQUIP_TYPE_SWORD, EQUIP_INV_SWORD_KOKIRI): bit 0. Not put on B.
        assert_eq!(s.inventory.equipment & 0xF, 1);
        assert_eq!(s.equips.button_items[0], ITEM_NONE);
        assert_eq!(item_check_obtainability(&s, ITEM_SWORD_KOKIRI), ITEM_SWORD_KOKIRI);
        // The Deku Shield: OWNED_EQUIP_FLAG(EQUIP_TYPE_SHIELD, 0), bit 4.
        item_give(&mut s, None, ITEM_SHIELD_DEKU);
        assert_eq!(s.inventory.equipment & 0xF0, 0x10);
        assert_eq!(item_check_obtainability(&s, ITEM_SHIELD_DEKU), ITEM_SHIELD_DEKU);
    }

    #[test]
    fn sticks_and_nuts() {
        let mut s = SaveContext::new(0, false, 0);
        // The first stick: the stick upgrade 1 (10), one stick, and the slot takes it.
        assert_eq!(item_give(&mut s, None, ITEM_DEKU_STICK), ITEM_NONE);
        assert_eq!((s.cur_upg_value(UPG_DEKU_STICKS), s.ammo(ITEM_DEKU_STICK), s.inv_content(ITEM_DEKU_STICK)), (1, 1, ITEM_DEKU_STICK));
        // Five more (sAmmoRefillCounts[0]), capped at 10.
        item_give(&mut s, None, ITEM_DEKU_STICKS_5);
        item_give(&mut s, None, ITEM_DEKU_STICKS_5);
        assert_eq!(s.ammo(ITEM_DEKU_STICK), 10);
        // Nuts: ITEM_DEKU_NUTS_5 without nuts adds 5 to nothing.
        assert_eq!(item_give(&mut s, None, ITEM_DEKU_NUTS_5), ITEM_NONE);
        assert_eq!((s.cur_upg_value(UPG_DEKU_NUTS), s.ammo(ITEM_DEKU_NUT), s.inv_content(ITEM_DEKU_NUT)), (1, 5, ITEM_DEKU_NUT));
        assert_eq!(item_check_obtainability(&s, ITEM_DEKU_NUTS_5), ITEM_DEKU_NUT);
    }
}
