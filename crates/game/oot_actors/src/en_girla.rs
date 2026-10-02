//! `En_GirlA` (`ovl_En_GirlA/z_en_girla.c`): an item on a shop's shelf. `params` is the item
//! (`EnGirlAShopItem`, `SI_*`), an index into `sShopItemEntries`: its object, its get-item model
//! (`GetItem_Draw`), its price and count, its two texts (the description while browsing and the
//! buy prompt), what Link gets, and the three functions the shopkeeper calls (`canBuyFunc`,
//! `itemGiveFunc`, `buyEventFunc`).
//!
//! `En_Ossan` spawns them on its shelves (`EnOssan_SpawnItemsOnShelves`), moves the one Link
//! chooses towards him and back (`EnOssan_PositionSelectedItem`), marks it selected (it spins:
//! `EnGirlA_Update2`), and when Link buys it asks whether he can (`canBuyFunc`): a fanfare item
//! is offered through the get-item flow and charged afterwards (`buyEventFunc`); anything else
//! is given at once and charged (`itemGiveFunc`). A bought item hides (`EnGirlA_SetItemOutOfStock`)
//! until the shopkeeper puts it back (`EnGirlA_UpdateStockedItem`), sold out for good for the
//! bombchu packs.
//!
//! The whole overlay is ported. Left out: the items' highlight setup (`hiliteFunc`:
//! `func_8002EBCC`, `func_8002ED80`, `func_80A3C498`), kept as data but not drawn, as for
//! `En_Item00`'s rupees; the prints.

use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile};
use oot_game::item::*;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::save::SaveContext;

/// `ACTOR_EN_GIRLA` (`actor_table.h`).
pub const ACTOR_EN_GIRLA: i16 = 0x0004;

/// `En_GirlA_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_GIRLA, name: "En_GirlA", category: ACTORCAT_PROP, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: "gameplay_keep" };

// `EnGirlAShopItem` (`z_en_girla.h`).
pub const SI_DEKU_NUTS_5: i16 = 0x00;
pub const SI_ARROWS_30: i16 = 0x01;
pub const SI_ARROWS_50: i16 = 0x02;
pub const SI_BOMBS_5_R25: i16 = 0x03;
pub const SI_DEKU_NUTS_10: i16 = 0x04;
pub const SI_DEKU_STICK: i16 = 0x05;
pub const SI_BOMBS_10: i16 = 0x06;
pub const SI_FISH: i16 = 0x07;
pub const SI_RED_POTION_R30: i16 = 0x08;
pub const SI_GREEN_POTION: i16 = 0x09;
pub const SI_BLUE_POTION: i16 = 0x0A;
pub const SI_LONGSWORD: i16 = 0x0B;
pub const SI_HYLIAN_SHIELD: i16 = 0x0C;
pub const SI_DEKU_SHIELD: i16 = 0x0D;
pub const SI_GORON_TUNIC: i16 = 0x0E;
pub const SI_ZORA_TUNIC: i16 = 0x0F;
pub const SI_RECOVERY_HEART: i16 = 0x10;
pub const SI_MILK_BOTTLE: i16 = 0x11;
pub const SI_WEIRD_EGG: i16 = 0x12;
pub const SI_19: i16 = 0x13;
pub const SI_20: i16 = 0x14;
pub const SI_BOMBCHU_10_1: i16 = 0x15;
pub const SI_BOMBCHU_20_1: i16 = 0x16;
pub const SI_BOMBCHU_20_2: i16 = 0x17;
pub const SI_BOMBCHU_10_2: i16 = 0x18;
pub const SI_BOMBCHU_10_3: i16 = 0x19;
pub const SI_BOMBCHU_20_3: i16 = 0x1A;
pub const SI_BOMBCHU_20_4: i16 = 0x1B;
pub const SI_BOMBCHU_10_4: i16 = 0x1C;
pub const SI_DEKU_SEEDS_30: i16 = 0x1D;
pub const SI_KEATON_MASK: i16 = 0x1E;
pub const SI_SPOOKY_MASK: i16 = 0x1F;
pub const SI_SKULL_MASK: i16 = 0x20;
pub const SI_BUNNY_HOOD: i16 = 0x21;
pub const SI_MASK_OF_TRUTH: i16 = 0x22;
pub const SI_ZORA_MASK: i16 = 0x23;
pub const SI_GORON_MASK: i16 = 0x24;
pub const SI_GERUDO_MASK: i16 = 0x25;
pub const SI_SOLD_OUT: i16 = 0x26;
pub const SI_BLUE_FIRE: i16 = 0x27;
pub const SI_BUGS: i16 = 0x28;
pub const SI_BIG_POE: i16 = 0x29;
pub const SI_POE: i16 = 0x2A;
pub const SI_FAIRY: i16 = 0x2B;
pub const SI_ARROWS_10: i16 = 0x2C;
pub const SI_BOMBS_20: i16 = 0x2D;
pub const SI_BOMBS_30: i16 = 0x2E;
pub const SI_BOMBS_5_R35: i16 = 0x2F;
pub const SI_RED_POTION_R40: i16 = 0x30;
pub const SI_RED_POTION_R50: i16 = 0x31;
pub const SI_MAX: i16 = 0x32;

// `EnGirlACanBuyResult`.
pub const CANBUY_RESULT_SUCCESS_FANFARE: i32 = 0;
pub const CANBUY_RESULT_SUCCESS: i32 = 1;
pub const CANBUY_RESULT_CANT_GET_NOW: i32 = 2;
pub const CANBUY_RESULT_NEED_BOTTLE: i32 = 3;
pub const CANBUY_RESULT_NEED_RUPEES: i32 = 4;
pub const CANBUY_RESULT_CANT_GET_NOW_5: i32 = 5;

// `OBJECT_*` (`object_table.h`).
const OBJECT_GI_HEART: i16 = 0x00B7;
const OBJECT_GI_NUTS: i16 = 0x00BB;
const OBJECT_GI_STICK: i16 = 0x00C7;
const OBJECT_GI_SHIELD_1: i16 = 0x00CB;
const OBJECT_GI_BOMB_1: i16 = 0x00CE;
const OBJECT_GI_ARROW: i16 = 0x00D8;
const OBJECT_GI_BOMB_2: i16 = 0x00D9;
const OBJECT_GI_EGG: i16 = 0x00DA;
const OBJECT_GI_SHIELD_2: i16 = 0x00DC;
const OBJECT_GI_MILK: i16 = 0x00DF;
const OBJECT_GI_LIQUID: i16 = 0x00EB;
const OBJECT_GI_CLOTHES: i16 = 0x00F2;
const OBJECT_GI_FISH: i16 = 0x00F4;
const OBJECT_GI_LONGSWORD: i16 = 0x00F8;
const OBJECT_GI_SEED: i16 = 0x0119;
const OBJECT_GI_KI_TAN_MASK: i16 = 0x0134;
const OBJECT_GI_REDEAD_MASK: i16 = 0x0135;
const OBJECT_GI_SKJ_MASK: i16 = 0x0136;
const OBJECT_GI_RABIT_MASK: i16 = 0x0137;
const OBJECT_GI_TRUTH_MASK: i16 = 0x0138;
const OBJECT_GI_SOLDOUT: i16 = 0x0148;
const OBJECT_GI_GOLONMASK: i16 = 0x0150;
const OBJECT_GI_ZORAMASK: i16 = 0x0151;
const OBJECT_GI_GERUDOMASK: i16 = 0x0152;
const OBJECT_GI_FIRE: i16 = 0x0173;
const OBJECT_GI_INSECT: i16 = 0x0174;
const OBJECT_GI_GHOST: i16 = 0x0176;
const OBJECT_GI_SOUL: i16 = 0x0177;

// `GetItemDrawID` (`item.h`).
const GID_RECOVERY_HEART: i16 = 0x08;
const GID_DEKU_NUTS: i16 = 0x11;
const GID_DEKU_STICK: i16 = 0x1A;
const GID_SHIELD_DEKU: i16 = 0x1C;
const GID_BOMB: i16 = 0x1F;
const GID_ARROWS_5: i16 = 0x24;
const GID_ARROWS_10: i16 = 0x25;
const GID_ARROWS_30: i16 = 0x26;
const GID_BOMBCHU: i16 = 0x27;
const GID_EGG: i16 = 0x28;
const GID_SHIELD_HYLIAN: i16 = 0x2B;
const GID_BOTTLE_MILK_FULL: i16 = 0x2F;
const GID_MASK_KEATON: i16 = 0x30;
const GID_MASK_SPOOKY: i16 = 0x31;
const GID_BOTTLE_POTION_GREEN: i16 = 0x36;
const GID_BOTTLE_POTION_RED: i16 = 0x37;
const GID_BOTTLE_POTION_BLUE: i16 = 0x38;
const GID_TUNIC_GORON: i16 = 0x3B;
const GID_TUNIC_ZORA: i16 = 0x3C;
const GID_FISH: i16 = 0x3E;
const GID_SWORD_BIGGORON: i16 = 0x42;
const GID_DEKU_SEEDS: i16 = 0x47;
const GID_MASK_SKULL: i16 = 0x4E;
const GID_MASK_BUNNY_HOOD: i16 = 0x4F;
const GID_MASK_TRUTH: i16 = 0x50;
const GID_SOLDOUT: i16 = 0x58;
const GID_MASK_GORON: i16 = 0x5A;
const GID_MASK_ZORA: i16 = 0x5B;
const GID_MASK_GERUDO: i16 = 0x5C;
const GID_BLUE_FIRE: i16 = 0x66;
const GID_BUG: i16 = 0x67;
const GID_POE: i16 = 0x69;
const GID_FAIRY: i16 = 0x6A;
const GID_BIG_POE: i16 = 0x6F;

// `GetItemID` (`item.h`).
const GI_BOMBS_5: i16 = 0x01;
const GI_BOMBCHUS_10: i16 = 0x03;
const GI_BOTTLE_POTION_RED: i16 = 0x10;
const GI_BOTTLE_POTION_GREEN: i16 = 0x11;
const GI_BOTTLE_POTION_BLUE: i16 = 0x12;
const GI_BOTTLE_FAIRY: i16 = 0x13;
const GI_BOTTLE_MILK_FULL: i16 = 0x14;
const GI_MASK_SKULL: i16 = 0x17;
const GI_MASK_SPOOKY: i16 = 0x18;
const GI_MASK_KEATON: i16 = 0x1A;
const GI_MASK_BUNNY_HOOD: i16 = 0x1B;
const GI_MASK_TRUTH: i16 = 0x1C;
const GI_SWORD_KNIFE: i16 = 0x28;
const GI_SHIELD_HYLIAN: i16 = 0x2A;
const GI_TUNIC_GORON: i16 = 0x2C;
const GI_TUNIC_ZORA: i16 = 0x2D;
const GI_WEIRD_EGG: i16 = 0x47;
const GI_ARROWS_5: i16 = 0x49;
const GI_ARROWS_10: i16 = 0x4A;
const GI_ARROWS_30: i16 = 0x4B;
const GI_MASK_GORON: i16 = 0x51;
const GI_MASK_ZORA: i16 = 0x52;
const GI_MASK_GERUDO: i16 = 0x53;
const GI_DEKU_NUTS_5_2: i16 = 0x63;
const GI_DEKU_NUTS_10: i16 = 0x64;
const GI_BOMBS_10: i16 = 0x66;
const GI_BOMBS_20: i16 = 0x67;
const GI_BOMBS_30: i16 = 0x68;
const GI_DEKU_SEEDS_30: i16 = 0x69;
const GI_BOMBCHUS_20: i16 = 0x6B;
const GI_BOTTLE_FISH: i16 = 0x6C;
const GI_BOTTLE_BUGS: i16 = 0x6D;
const GI_BOTTLE_BLUE_FIRE: i16 = 0x6E;
const GI_BOTTLE_POE: i16 = 0x6F;
const GI_BOTTLE_BIG_POE: i16 = 0x70;

// `ITEM_*` (`item.h`) the port's item module doesn't name.
const ITEM_BOMBS_10: u8 = 0x8F;
const ITEM_BOMBS_20: u8 = 0x90;
const ITEM_TUNIC_GORON: u8 = 0x42;
const ITEM_MASK_SPOOKY: u8 = 0x25;
const ITEM_MASK_SKULL: u8 = 0x26;
const ITEM_MASK_BUNNY_HOOD: u8 = 0x27;
const ITEM_MASK_TRUTH: u8 = 0x28;
const ITEM_MASK_GORON: u8 = 0x29;
const ITEM_MASK_ZORA: u8 = 0x2A;
const ITEM_MASK_GERUDO: u8 = 0x2B;

// `ITEMGETINF_*` (`save.h`): the bit numbers.
const ITEMGETINF_TALON_BOTTLE: u16 = 0x02;
const ITEMGETINF_03: u16 = 0x03;
const ITEMGETINF_04: u16 = 0x04;
const ITEMGETINF_05: u16 = 0x05;
const ITEMGETINF_06: u16 = 0x06;
const ITEMGETINF_07: u16 = 0x07;
const ITEMGETINF_08: u16 = 0x08;
const ITEMGETINF_09: u16 = 0x09;
const ITEMGETINF_0A: u16 = 0x0A;
pub const ITEMGETINF_38: u16 = 0x38;
pub const ITEMGETINF_39: u16 = 0x39;
pub const ITEMGETINF_3A: u16 = 0x3A;
pub const ITEMGETINF_3B: u16 = 0x3B;
/// `INFTABLE_76`: Zelda's letter shown to the Kakariko guard (the Hylian Shield's discount).
pub const INFTABLE_76: u16 = 0x76;
/// `QUEST_GORON_RUBY`.
const QUEST_GORON_RUBY: u32 = 0x13;

/// `hiliteFunc`: the highlight setup the item draws after (not drawn by the port).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hilite {
    None,
    /// `func_8002EBCC`.
    Opa,
    /// `func_8002ED80`.
    Xlu,
    /// `func_80A3C498`: both.
    Both,
}

pub type CanBuyFunc = fn(&SaveContext, &EnGirlA) -> i32;
pub type GiveFunc = fn(&mut PlayState, &EnGirlA);

/// `ShopItemEntry`.
#[derive(Clone, Copy)]
pub struct ShopItemEntry {
    pub obj_id: i16,
    pub gi_draw_id: i16,
    pub hilite: Hilite,
    pub price: i16,
    pub count: i16,
    pub item_desc_text_id: u16,
    pub item_buy_prompt_text_id: u16,
    pub get_item_id: i16,
    pub can_buy: CanBuyFunc,
    pub item_give: Option<GiveFunc>,
    pub buy_event: Option<GiveFunc>,
}

#[allow(clippy::too_many_arguments)]
const fn e(
    obj_id: i16,
    gi_draw_id: i16,
    hilite: Hilite,
    price: i16,
    count: i16,
    desc: u16,
    prompt: u16,
    gi: i16,
    can_buy: CanBuyFunc,
    give: Option<GiveFunc>,
    event: Option<GiveFunc>,
) -> ShopItemEntry {
    ShopItemEntry { obj_id, gi_draw_id, hilite, price, count, item_desc_text_id: desc, item_buy_prompt_text_id: prompt, get_item_id: gi, can_buy, item_give: give, buy_event: event }
}

use Hilite::{Both as C498, None as NOHL, Opa as EBCC, Xlu as ED80};
const SHIELD_DISCOUNT: Option<GiveFunc> = Some(buy_event_shield_discount);

/// `sShopItemEntries`, by `SI_*`.
pub const SHOP_ITEM_ENTRIES: [ShopItemEntry; SI_MAX as usize] = [
    e(OBJECT_GI_NUTS, GID_DEKU_NUTS, ED80, 15, 5, 0x00B2, 0x007F, GI_DEKU_NUTS_5_2, can_buy_deku_nuts, Some(item_give_deku_nuts), SHIELD_DISCOUNT),
    e(OBJECT_GI_ARROW, GID_ARROWS_10, EBCC, 60, 30, 0x00C1, 0x009B, GI_ARROWS_10, can_buy_arrows, Some(item_give_arrows), SHIELD_DISCOUNT),
    e(OBJECT_GI_ARROW, GID_ARROWS_30, EBCC, 90, 50, 0x00B0, 0x007D, GI_ARROWS_30, can_buy_arrows, Some(item_give_arrows), SHIELD_DISCOUNT),
    e(OBJECT_GI_BOMB_1, GID_BOMB, EBCC, 25, 5, 0x00A3, 0x008B, GI_BOMBS_5, can_buy_bombs, Some(item_give_bombs), SHIELD_DISCOUNT),
    e(OBJECT_GI_NUTS, GID_DEKU_NUTS, ED80, 30, 10, 0x00A2, 0x0087, GI_DEKU_NUTS_10, can_buy_deku_nuts, Some(item_give_deku_nuts), SHIELD_DISCOUNT),
    e(OBJECT_GI_STICK, GID_DEKU_STICK, NOHL, 10, 1, 0x00A1, 0x0088, GI_DEKU_STICKS_1, can_buy_deku_sticks, Some(item_give_deku_sticks), SHIELD_DISCOUNT),
    e(OBJECT_GI_BOMB_1, GID_BOMB, EBCC, 50, 10, 0x00B1, 0x007C, GI_BOMBS_10, can_buy_bombs, Some(item_give_bombs), SHIELD_DISCOUNT),
    e(OBJECT_GI_FISH, GID_FISH, ED80, 200, 1, 0x00B3, 0x007E, GI_BOTTLE_FISH, can_buy_fish, None, SHIELD_DISCOUNT),
    e(OBJECT_GI_LIQUID, GID_BOTTLE_POTION_RED, EBCC, 30, 1, 0x00A5, 0x008E, GI_BOTTLE_POTION_RED, can_buy_red_potion, Some(item_give_bottled_item), SHIELD_DISCOUNT),
    e(OBJECT_GI_LIQUID, GID_BOTTLE_POTION_GREEN, EBCC, 30, 1, 0x00A6, 0x008F, GI_BOTTLE_POTION_GREEN, can_buy_green_potion, Some(item_give_bottled_item), SHIELD_DISCOUNT),
    e(OBJECT_GI_LIQUID, GID_BOTTLE_POTION_BLUE, EBCC, 60, 1, 0x00A7, 0x0090, GI_BOTTLE_POTION_BLUE, can_buy_blue_potion, Some(item_give_bottled_item), SHIELD_DISCOUNT),
    e(OBJECT_GI_LONGSWORD, GID_SWORD_BIGGORON, EBCC, 1000, 1, 0x00A8, 0x0091, GI_SWORD_KNIFE, can_buy_longsword, Some(item_give_longsword), SHIELD_DISCOUNT),
    e(OBJECT_GI_SHIELD_2, GID_SHIELD_HYLIAN, EBCC, 80, 1, 0x00A9, 0x0092, GI_SHIELD_HYLIAN, can_buy_hylian_shield, Some(item_give_hylian_shield), SHIELD_DISCOUNT),
    e(OBJECT_GI_SHIELD_1, GID_SHIELD_DEKU, EBCC, 40, 1, 0x009F, 0x0089, GI_SHIELD_DEKU, can_buy_deku_shield, Some(item_give_deku_shield), SHIELD_DISCOUNT),
    e(OBJECT_GI_CLOTHES, GID_TUNIC_GORON, NOHL, 200, 1, 0x00AA, 0x0093, GI_TUNIC_GORON, can_buy_goron_tunic, Some(item_give_goron_tunic), Some(buy_event_goron_tunic)),
    e(OBJECT_GI_CLOTHES, GID_TUNIC_ZORA, NOHL, 300, 1, 0x00AB, 0x0094, GI_TUNIC_ZORA, can_buy_zora_tunic, Some(item_give_zora_tunic), Some(buy_event_zora_tunic)),
    e(OBJECT_GI_HEART, GID_RECOVERY_HEART, NOHL, 10, 16, 0x00AC, 0x0095, GI_RECOVERY_HEART, can_buy_recovery_heart, Some(item_give_health), SHIELD_DISCOUNT),
    e(OBJECT_GI_MILK, GID_BOTTLE_MILK_FULL, C498, 100, 1, 0x00AD, 0x0097, GI_BOTTLE_MILK_FULL, can_buy_milk_bottle, Some(item_give_milk_bottle), SHIELD_DISCOUNT),
    e(OBJECT_GI_EGG, GID_EGG, EBCC, 100, 1, 0x00AE, 0x0099, GI_WEIRD_EGG, can_buy_weird_egg, Some(item_give_weird_egg), SHIELD_DISCOUNT),
    e(OBJECT_GI_MILK, GID_BOTTLE_MILK_FULL, C498, 10000, 1, 0x00B4, 0x0085, GI_NONE, can_buy_unk19, Some(item_give_unk19), SHIELD_DISCOUNT),
    e(OBJECT_GI_EGG, GID_EGG, EBCC, 10000, 1, 0x00B5, 0x0085, GI_NONE, can_buy_unk20, Some(item_give_unk20), SHIELD_DISCOUNT),
    e(OBJECT_GI_BOMB_2, GID_BOMBCHU, EBCC, 100, 10, 0x00BC, 0x008C, GI_BOMBCHUS_10, can_buy_bombchus, None, Some(buy_event_obtain_bombchu_pack)),
    e(OBJECT_GI_BOMB_2, GID_BOMBCHU, EBCC, 180, 20, 0x0061, 0x002A, GI_BOMBCHUS_20, can_buy_bombchus, None, Some(buy_event_obtain_bombchu_pack)),
    e(OBJECT_GI_BOMB_2, GID_BOMBCHU, EBCC, 180, 20, 0x0061, 0x002A, GI_BOMBCHUS_20, can_buy_bombchus, None, Some(buy_event_obtain_bombchu_pack)),
    e(OBJECT_GI_BOMB_2, GID_BOMBCHU, EBCC, 100, 10, 0x00BC, 0x008C, GI_BOMBCHUS_10, can_buy_bombchus, None, Some(buy_event_obtain_bombchu_pack)),
    e(OBJECT_GI_BOMB_2, GID_BOMBCHU, EBCC, 100, 10, 0x00BC, 0x008C, GI_BOMBCHUS_10, can_buy_bombchus, None, Some(buy_event_obtain_bombchu_pack)),
    e(OBJECT_GI_BOMB_2, GID_BOMBCHU, EBCC, 180, 20, 0x0061, 0x002A, GI_BOMBCHUS_20, can_buy_bombchus, None, Some(buy_event_obtain_bombchu_pack)),
    e(OBJECT_GI_BOMB_2, GID_BOMBCHU, EBCC, 180, 20, 0x0061, 0x002A, GI_BOMBCHUS_20, can_buy_bombchus, None, Some(buy_event_obtain_bombchu_pack)),
    e(OBJECT_GI_BOMB_2, GID_BOMBCHU, EBCC, 100, 10, 0x00BC, 0x008C, GI_BOMBCHUS_10, can_buy_bombchus, None, Some(buy_event_obtain_bombchu_pack)),
    e(OBJECT_GI_SEED, GID_DEKU_SEEDS, EBCC, 30, 30, 0x00DF, 0x00DE, GI_DEKU_SEEDS_30, can_buy_deku_seeds, Some(item_give_deku_seeds), SHIELD_DISCOUNT),
    // The masks: the Deku seeds' functions (the Happy Mask Shop's own states sell them).
    e(OBJECT_GI_KI_TAN_MASK, GID_MASK_KEATON, EBCC, 0, 1, 0x70B2, 0x70BE, GI_MASK_KEATON, can_buy_deku_seeds, Some(item_give_deku_seeds), SHIELD_DISCOUNT),
    e(OBJECT_GI_REDEAD_MASK, GID_MASK_SPOOKY, EBCC, 0, 1, 0x70B1, 0x70BD, GI_MASK_SPOOKY, can_buy_deku_seeds, Some(item_give_deku_seeds), SHIELD_DISCOUNT),
    e(OBJECT_GI_SKJ_MASK, GID_MASK_SKULL, EBCC, 0, 1, 0x70B0, 0x70BC, GI_MASK_SKULL, can_buy_deku_seeds, Some(item_give_deku_seeds), SHIELD_DISCOUNT),
    e(OBJECT_GI_RABIT_MASK, GID_MASK_BUNNY_HOOD, EBCC, 0, 1, 0x70B3, 0x70BF, GI_MASK_BUNNY_HOOD, can_buy_deku_seeds, Some(item_give_deku_seeds), SHIELD_DISCOUNT),
    e(OBJECT_GI_TRUTH_MASK, GID_MASK_TRUTH, C498, 0, 1, 0x70AF, 0x70C3, GI_MASK_TRUTH, can_buy_deku_seeds, Some(item_give_deku_seeds), SHIELD_DISCOUNT),
    e(OBJECT_GI_ZORAMASK, GID_MASK_ZORA, NOHL, 0, 1, 0x70B9, 0x70C1, GI_MASK_ZORA, can_buy_deku_seeds, Some(item_give_deku_seeds), SHIELD_DISCOUNT),
    e(OBJECT_GI_GOLONMASK, GID_MASK_GORON, NOHL, 0, 1, 0x70B8, 0x70C0, GI_MASK_GORON, can_buy_deku_seeds, Some(item_give_deku_seeds), SHIELD_DISCOUNT),
    e(OBJECT_GI_GERUDOMASK, GID_MASK_GERUDO, NOHL, 0, 1, 0x70BA, 0x70C2, GI_MASK_GERUDO, can_buy_deku_seeds, Some(item_give_deku_seeds), SHIELD_DISCOUNT),
    // SI_SOLD_OUT: GI_MASK_GERUDO and the Gerudo Mask's prompt, as in the C.
    e(OBJECT_GI_SOLDOUT, GID_SOLDOUT, EBCC, 0, 0, 0x00BD, 0x70C2, GI_MASK_GERUDO, can_buy_sold_out, None, None),
    e(OBJECT_GI_FIRE, GID_BLUE_FIRE, EBCC, 300, 1, 0x00B9, 0x00B8, GI_BOTTLE_BLUE_FIRE, can_buy_blue_fire, Some(item_give_bottled_item), SHIELD_DISCOUNT),
    e(OBJECT_GI_INSECT, GID_BUG, C498, 50, 1, 0x00BB, 0x00BA, GI_BOTTLE_BUGS, can_buy_bugs, Some(item_give_bottled_item), SHIELD_DISCOUNT),
    e(OBJECT_GI_GHOST, GID_BIG_POE, C498, 50, 1, 0x506F, 0x5070, GI_BOTTLE_BIG_POE, can_buy_poe, Some(item_give_bottled_item), SHIELD_DISCOUNT),
    e(OBJECT_GI_GHOST, GID_POE, C498, 30, 1, 0x506D, 0x506E, GI_BOTTLE_POE, can_buy_poe, Some(item_give_bottled_item), SHIELD_DISCOUNT),
    e(OBJECT_GI_SOUL, GID_FAIRY, C498, 50, 1, 0x00B7, 0x00B6, GI_BOTTLE_FAIRY, can_buy_fairy, Some(item_give_bottled_item), SHIELD_DISCOUNT),
    e(OBJECT_GI_ARROW, GID_ARROWS_5, EBCC, 20, 10, 0x00A0, 0x008A, GI_ARROWS_5, can_buy_arrows, Some(item_give_arrows), SHIELD_DISCOUNT),
    e(OBJECT_GI_BOMB_1, GID_BOMB, EBCC, 80, 20, 0x001C, 0x0006, GI_BOMBS_20, can_buy_bombs, Some(item_give_bombs), SHIELD_DISCOUNT),
    e(OBJECT_GI_BOMB_1, GID_BOMB, EBCC, 120, 30, 0x001D, 0x001E, GI_BOMBS_30, can_buy_bombs, Some(item_give_bombs), SHIELD_DISCOUNT),
    e(OBJECT_GI_BOMB_1, GID_BOMB, EBCC, 35, 5, 0x00CB, 0x00CA, GI_BOMBS_5, can_buy_bombs, Some(item_give_bombs), SHIELD_DISCOUNT),
    e(OBJECT_GI_LIQUID, GID_BOTTLE_POTION_RED, EBCC, 40, 1, 0x0064, 0x0062, GI_BOTTLE_POTION_RED, can_buy_red_potion, Some(item_give_bottled_item), SHIELD_DISCOUNT),
    e(OBJECT_GI_LIQUID, GID_BOTTLE_POTION_RED, EBCC, 50, 1, 0x0065, 0x0063, GI_BOTTLE_POTION_RED, can_buy_red_potion, Some(item_give_bottled_item), SHIELD_DISCOUNT),
];

/// `sShieldDiscounts`: what the Hylian Shield's discount takes off.
const SHIELD_DISCOUNTS: [i16; 8] = [5, 10, 15, 20, 25, 30, 35, 40];
/// `sMaskShopItems`.
const MASK_SHOP_ITEMS: [u8; 8] = [ITEM_MASK_KEATON, ITEM_MASK_SPOOKY, ITEM_MASK_SKULL, ITEM_MASK_BUNNY_HOOD, ITEM_MASK_TRUTH, ITEM_MASK_ZORA, ITEM_MASK_GORON, ITEM_MASK_GERUDO];
/// `sMaskShopFreeToBorrowTextIds`.
const MASK_SHOP_FREE_TO_BORROW_TEXT_IDS: [u16; 5] = [0x70B6, 0x70B5, 0x70B4, 0x70B7, 0x70BB];

/// `actionFunc2`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `EnGirlA_WaitForObject`: waiting for the item's object.
    Initialize,
    /// `EnGirlA_Update2`.
    Update2,
}

pub struct EnGirlA {
    pub actor: Actor,
    pub obj_bank_index: Option<usize>,
    pub action: Action,
    pub is_initialized: bool,
    pub item_buy_prompt_text_id: u16,
    pub get_item_id: i16,
    pub is_invisible: bool,
    /// `actor.draw != NULL`: out of stock takes the draw away.
    pub drawn: bool,
    pub is_selected: bool,
    pub y_rotation_init: i16,
    pub y_rotation: i16,
    pub can_buy_func: Option<CanBuyFunc>,
    pub item_give_func: Option<GiveFunc>,
    pub buy_event_func: Option<GiveFunc>,
    pub base_price: i16,
    pub item_count: i16,
    pub gi_draw_id: i16,
    pub hilite: Hilite,
}

impl EnGirlA {
    /// `EnGirlA_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut g = EnGirlA {
            actor,
            obj_bank_index: None,
            action: Action::Initialize,
            is_initialized: false,
            item_buy_prompt_text_id: 0,
            get_item_id: 0,
            is_invisible: false,
            drawn: false,
            is_selected: false,
            y_rotation_init: 0,
            y_rotation: 0,
            can_buy_func: None,
            item_give_func: None,
            buy_event_func: None,
            base_price: 0,
            item_count: 0,
            gi_draw_id: 0,
            hilite: Hilite::None,
        };
        g.try_change_shop_item(&play.save);
        g.init_item(play);
        Box::new(g)
    }

    /// The item's `sShopItemEntries` row.
    pub fn entry(&self) -> Option<&'static ShopItemEntry> {
        usize::try_from(self.actor.params).ok().and_then(|i| SHOP_ITEM_ENTRIES.get(i))
    }

    /// `EnGirlA_TryChangeShopItem`: the milk becomes a recovery heart once got, and a bombchu
    /// pack bought is sold out.
    fn try_change_shop_item(&mut self, s: &SaveContext) -> bool {
        let (flag, to) = match self.actor.params {
            SI_MILK_BOTTLE => (ITEMGETINF_TALON_BOTTLE, SI_RECOVERY_HEART),
            SI_BOMBCHU_10_2 => (ITEMGETINF_06, SI_SOLD_OUT),
            SI_BOMBCHU_10_3 => (ITEMGETINF_07, SI_SOLD_OUT),
            SI_BOMBCHU_20_3 => (ITEMGETINF_08, SI_SOLD_OUT),
            SI_BOMBCHU_20_4 => (ITEMGETINF_09, SI_SOLD_OUT),
            SI_BOMBCHU_10_4 => (ITEMGETINF_0A, SI_SOLD_OUT),
            SI_BOMBCHU_10_1 => (ITEMGETINF_03, SI_SOLD_OUT),
            SI_BOMBCHU_20_1 => (ITEMGETINF_04, SI_SOLD_OUT),
            SI_BOMBCHU_20_2 => (ITEMGETINF_05, SI_SOLD_OUT),
            _ => return false,
        };
        if s.get_item_get_inf(flag) {
            self.actor.params = to;
            return true;
        }
        false
    }

    /// `EnGirlA_InitItem`: the item's object, then `EnGirlA_WaitForObject`.
    fn init_item(&mut self, play: &PlayState) {
        let params = self.actor.params;
        // @bug (game): `(params >= SI_MAX) && (params < 0)` is never true; an item past the
        // table reads past it. The port kills it.
        let Some(entry) = self.entry() else {
            log::error!("En_GirlA: params {params} is past sShopItemEntries");
            self.actor.kill();
            return;
        };
        self.obj_bank_index = play.object_ctx.get_index(entry.obj_id);
        if self.obj_bank_index.is_none() {
            // "No bank!!"
            log::warn!("En_GirlA {params}: object {:#06x} isn't loaded in this room", entry.obj_id);
            self.actor.kill();
            return;
        }
        self.action = Action::Initialize;
    }

    /// `EnGirlA_SetItemDescription`: the description text (a mask the shop lends has its own),
    /// and drawn again.
    fn set_item_description(&mut self, s: &SaveContext) {
        let Some(tmp) = self.entry() else { return };
        let params = self.actor.params;
        if (SI_KEATON_MASK..=SI_MASK_OF_TRUTH).contains(&params) {
            let mask_id = (params - SI_KEATON_MASK) as usize;
            let free = match params {
                SI_KEATON_MASK => s.get_item_get_inf(ITEMGETINF_38),
                SI_SPOOKY_MASK => s.get_item_get_inf(ITEMGETINF_3A),
                SI_SKULL_MASK => s.get_item_get_inf(ITEMGETINF_39),
                SI_BUNNY_HOOD | SI_MASK_OF_TRUTH => s.get_item_get_inf(ITEMGETINF_3B),
                _ => false,
            };
            self.actor.text_id = if free { MASK_SHOP_FREE_TO_BORROW_TEXT_IDS[mask_id] } else { tmp.item_desc_text_id };
        } else {
            self.actor.text_id = tmp.item_desc_text_id;
        }
        self.is_invisible = false;
        self.drawn = true;
    }

    /// `EnGirlA_SetItemOutOfStock` (`setOutOfStockFunc`): hidden; a mask says "sold out"
    /// (0xBD).
    pub fn set_item_out_of_stock(&mut self) {
        self.is_invisible = true;
        self.drawn = false;
        if (SI_KEATON_MASK..=SI_GERUDO_MASK).contains(&self.actor.params) {
            self.actor.text_id = 0xBD;
        }
    }

    /// `EnGirlA_UpdateStockedItem` (`updateStockedItemFunc`): back on the shelf, or changed
    /// (sold out) and set up again.
    pub fn update_stocked_item(&mut self, play: &PlayState) {
        if self.try_change_shop_item(&play.save) {
            self.init_item(play);
            if let Some(e) = self.entry() {
                self.actor.text_id = e.item_desc_text_id;
            }
        } else {
            self.is_invisible = false;
            self.drawn = true;
        }
    }

    /// `EnGirlA_TrySetMaskItemDescription`: a mask Link holds is out of stock.
    fn try_set_mask_item_description(&mut self, s: &SaveContext) -> bool {
        let params = self.actor.params;
        if (SI_KEATON_MASK..=SI_GERUDO_MASK).contains(&params) {
            if s.inv_content(ITEM_MASK_KEATON) == MASK_SHOP_ITEMS[(params - SI_KEATON_MASK) as usize] {
                self.set_item_out_of_stock();
            } else {
                self.set_item_description(s);
            }
            return true;
        }
        false
    }

    /// `EnGirlA_WaitForObject`: once the item's object is loaded, its texts, functions,
    /// price and model, its size and height (0.25, 24 up), and `EnGirlA_Update2`.
    fn initialize_item_action(&mut self, play: &PlayState) {
        let Some(bank) = self.obj_bank_index else { return };
        if !play.object_ctx.is_loaded(bank) {
            return;
        }
        let Some(entry) = self.entry() else { return };
        let s = &play.save;
        self.actor.flags &= !ACTOR_FLAG_UPDATE_CULLING_DISABLED;
        self.actor.obj_bank_index = Some(bank);
        let (text, prompt) = match self.actor.params {
            SI_KEATON_MASK => (if s.get_item_get_inf(ITEMGETINF_38) { 0x70B6 } else { entry.item_desc_text_id }, entry.item_buy_prompt_text_id),
            SI_SPOOKY_MASK => (if s.get_item_get_inf(ITEMGETINF_3A) { 0x70B5 } else { entry.item_desc_text_id }, entry.item_buy_prompt_text_id),
            SI_SKULL_MASK => (if s.get_item_get_inf(ITEMGETINF_39) { 0x70B4 } else { entry.item_desc_text_id }, entry.item_buy_prompt_text_id),
            SI_BUNNY_HOOD => (if s.get_item_get_inf(ITEMGETINF_3B) { 0x70B7 } else { entry.item_desc_text_id }, entry.item_buy_prompt_text_id),
            SI_MASK_OF_TRUTH => {
                if s.get_item_get_inf(ITEMGETINF_3B) {
                    (0x70BB, entry.item_buy_prompt_text_id)
                } else {
                    (entry.item_desc_text_id, 0xEB)
                }
            }
            _ => (entry.item_desc_text_id, entry.item_buy_prompt_text_id),
        };
        self.actor.text_id = text;
        self.item_buy_prompt_text_id = prompt;
        if !self.try_set_mask_item_description(s) {
            self.set_item_description(s);
        }
        self.get_item_id = entry.get_item_id;
        self.can_buy_func = Some(entry.can_buy);
        self.item_give_func = entry.item_give;
        self.buy_event_func = entry.buy_event;
        self.base_price = entry.price;
        self.item_count = entry.count;
        self.hilite = entry.hilite;
        self.gi_draw_id = entry.gi_draw_id;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.actor.scale = Vec3::splat(0.25);
        self.actor.shape_y_offset = 24.0;
        // shape.shadowScale = 4.
        self.actor.floor_height = self.actor.home_pos.y;
        self.actor.gravity = 0.0;
        // EnGirlA_SetupAction(this, EnGirlA_Noop).
        self.is_initialized = true;
        self.action = Action::Update2;
        self.is_selected = false;
        self.y_rotation = 0;
        self.y_rotation_init = self.actor.shape_rot.y;
    }

    /// `EnGirlA_Update2`: spinning while selected, easing back when not.
    fn update2(&mut self, play: &PlayState) {
        self.actor.scale = Vec3::splat(0.25);
        self.actor.shape_y_offset = 24.0;
        self.try_set_mask_item_description(&play.save);
        // actionFunc: EnGirlA_Noop.
        self.actor.set_focus(5.0);
        self.actor.shape_rot.x = 0;
        if self.actor.params != SI_SOLD_OUT {
            if self.is_selected {
                self.y_rotation = self.y_rotation.wrapping_add(0x1F4);
            } else {
                eng_math::smooth_step_to_s(&mut self.y_rotation, 0, 10, 0x7D0, 0);
            }
        }
    }

    /// `canBuyFunc`.
    pub fn can_buy(&self, s: &SaveContext) -> i32 {
        self.can_buy_func.map(|f| f(s, self)).unwrap_or(CANBUY_RESULT_CANT_GET_NOW_5)
    }
}

// The `EnGirlA_CanBuy_*` functions.

fn can_buy_arrows(s: &SaveContext, g: &EnGirlA) -> i32 {
    if item_check_obtainability(s, ITEM_BOW) == ITEM_NONE {
        return CANBUY_RESULT_CANT_GET_NOW_5;
    }
    if s.ammo(ITEM_BOW) as i32 >= s.cur_capacity(UPG_QUIVER) as i32 {
        return CANBUY_RESULT_CANT_GET_NOW;
    }
    if s.rupees < g.base_price {
        return CANBUY_RESULT_NEED_RUPEES;
    }
    CANBUY_RESULT_SUCCESS
}

fn can_buy_bombs(s: &SaveContext, g: &EnGirlA) -> i32 {
    if !s.check_quest_item(QUEST_GORON_RUBY) {
        return CANBUY_RESULT_CANT_GET_NOW;
    }
    if s.ammo(ITEM_BOMB) as i32 >= s.cur_capacity(UPG_BOMB_BAG) as i32 {
        return CANBUY_RESULT_CANT_GET_NOW;
    }
    if s.rupees < g.base_price {
        return CANBUY_RESULT_NEED_RUPEES;
    }
    CANBUY_RESULT_SUCCESS
}

/// The shape most of the checks share: full, then the rupees, then whether it's new
/// (`Item_CheckObtainability`: a fanfare for the first).
fn fanfare_if_new(s: &SaveContext, g: &EnGirlA, item: u8) -> i32 {
    if s.rupees < g.base_price {
        return CANBUY_RESULT_NEED_RUPEES;
    }
    if item_check_obtainability(s, item) == ITEM_NONE {
        return CANBUY_RESULT_SUCCESS_FANFARE;
    }
    CANBUY_RESULT_SUCCESS
}

fn can_buy_deku_nuts(s: &SaveContext, g: &EnGirlA) -> i32 {
    if s.cur_capacity(UPG_DEKU_NUTS) != 0 && s.ammo(ITEM_DEKU_NUT) as i32 >= s.cur_capacity(UPG_DEKU_NUTS) as i32 {
        return CANBUY_RESULT_CANT_GET_NOW;
    }
    fanfare_if_new(s, g, ITEM_DEKU_NUT)
}

fn can_buy_deku_sticks(s: &SaveContext, g: &EnGirlA) -> i32 {
    if s.cur_capacity(UPG_DEKU_STICKS) != 0 && s.ammo(ITEM_DEKU_STICK) as i32 >= s.cur_capacity(UPG_DEKU_STICKS) as i32 {
        return CANBUY_RESULT_CANT_GET_NOW;
    }
    fanfare_if_new(s, g, ITEM_DEKU_STICK)
}

fn bottled(s: &SaveContext, g: &EnGirlA, item: u8) -> i32 {
    if !inventory_has_empty_bottle(s) {
        return CANBUY_RESULT_NEED_BOTTLE;
    }
    fanfare_if_new(s, g, item)
}

fn can_buy_fish(s: &SaveContext, g: &EnGirlA) -> i32 {
    bottled(s, g, ITEM_BOTTLE_FISH)
}

fn can_buy_red_potion(s: &SaveContext, g: &EnGirlA) -> i32 {
    bottled(s, g, ITEM_BOTTLE_POTION_RED)
}

fn can_buy_green_potion(s: &SaveContext, g: &EnGirlA) -> i32 {
    bottled(s, g, ITEM_BOTTLE_POTION_GREEN)
}

fn can_buy_blue_potion(s: &SaveContext, g: &EnGirlA) -> i32 {
    bottled(s, g, ITEM_BOTTLE_POTION_BLUE)
}

fn can_buy_longsword(s: &SaveContext, g: &EnGirlA) -> i32 {
    if s.check_owned_equip(EQUIP_TYPE_SWORD, EQUIP_INV_SWORD_BIGGORON) && !s.check_owned_equip(EQUIP_TYPE_SWORD, EQUIP_INV_SWORD_BROKENGIANTKNIFE) {
        return CANBUY_RESULT_CANT_GET_NOW;
    }
    fanfare_if_new(s, g, ITEM_SWORD_BIGGORON)
}

fn can_buy_hylian_shield(s: &SaveContext, g: &EnGirlA) -> i32 {
    if s.check_owned_equip(EQUIP_TYPE_SHIELD, EQUIP_INV_SHIELD_HYLIAN) {
        return CANBUY_RESULT_CANT_GET_NOW;
    }
    fanfare_if_new(s, g, ITEM_SHIELD_HYLIAN)
}

/// `EnGirlA_CanBuy_DekuShield`: not if one is owned; 40 rupees; the first one's fanfare.
pub fn can_buy_deku_shield(s: &SaveContext, g: &EnGirlA) -> i32 {
    if s.check_owned_equip(EQUIP_TYPE_SHIELD, EQUIP_INV_SHIELD_DEKU) {
        return CANBUY_RESULT_CANT_GET_NOW;
    }
    fanfare_if_new(s, g, ITEM_SHIELD_DEKU)
}

fn can_buy_goron_tunic(s: &SaveContext, g: &EnGirlA) -> i32 {
    if !s.adult || s.check_owned_equip(EQUIP_TYPE_TUNIC, EQUIP_INV_TUNIC_GORON) {
        return CANBUY_RESULT_CANT_GET_NOW;
    }
    fanfare_if_new(s, g, ITEM_TUNIC_GORON)
}

fn can_buy_zora_tunic(s: &SaveContext, g: &EnGirlA) -> i32 {
    if !s.adult || s.check_owned_equip(EQUIP_TYPE_TUNIC, EQUIP_INV_TUNIC_ZORA) {
        return CANBUY_RESULT_CANT_GET_NOW;
    }
    fanfare_if_new(s, g, ITEM_TUNIC_ZORA)
}

fn can_buy_recovery_heart(s: &SaveContext, g: &EnGirlA) -> i32 {
    if s.health_capacity == s.health {
        return CANBUY_RESULT_CANT_GET_NOW;
    }
    if s.rupees < g.base_price {
        return CANBUY_RESULT_NEED_RUPEES;
    }
    CANBUY_RESULT_SUCCESS
}

fn can_buy_milk_bottle(s: &SaveContext, g: &EnGirlA) -> i32 {
    fanfare_if_new(s, g, ITEM_BOTTLE_MILK_FULL)
}

/// `EnGirlA_CanBuy_WeirdEgg`: it asks about Zelda's letter, which shares the child trade slot
/// with the egg (`Item_CheckObtainability(ITEM_ZELDAS_LETTER)`).
fn can_buy_weird_egg(s: &SaveContext, g: &EnGirlA) -> i32 {
    fanfare_if_new(s, g, ITEM_ZELDAS_LETTER)
}

fn can_buy_unk19(_s: &SaveContext, _g: &EnGirlA) -> i32 {
    CANBUY_RESULT_NEED_RUPEES
}

fn can_buy_unk20(_s: &SaveContext, _g: &EnGirlA) -> i32 {
    CANBUY_RESULT_NEED_RUPEES
}

fn can_buy_bombchus(s: &SaveContext, g: &EnGirlA) -> i32 {
    if s.ammo(ITEM_BOMBCHU) >= 50 {
        return CANBUY_RESULT_CANT_GET_NOW;
    }
    fanfare_if_new(s, g, ITEM_BOMBCHU)
}

fn can_buy_deku_seeds(s: &SaveContext, g: &EnGirlA) -> i32 {
    if s.ammo(ITEM_SLINGSHOT) as i32 >= s.cur_capacity(UPG_BULLET_BAG) as i32 {
        return CANBUY_RESULT_CANT_GET_NOW;
    }
    fanfare_if_new(s, g, ITEM_DEKU_SEEDS)
}

fn can_buy_sold_out(_s: &SaveContext, _g: &EnGirlA) -> i32 {
    CANBUY_RESULT_CANT_GET_NOW_5
}

fn can_buy_blue_fire(s: &SaveContext, g: &EnGirlA) -> i32 {
    bottled(s, g, ITEM_BOTTLE_BLUE_FIRE)
}

fn can_buy_bugs(s: &SaveContext, g: &EnGirlA) -> i32 {
    bottled(s, g, ITEM_BOTTLE_BUG)
}

fn can_buy_poe(s: &SaveContext, g: &EnGirlA) -> i32 {
    bottled(s, g, ITEM_BOTTLE_POE)
}

fn can_buy_fairy(s: &SaveContext, g: &EnGirlA) -> i32 {
    bottled(s, g, ITEM_BOTTLE_FAIRY)
}

// The `EnGirlA_ItemGive_*` functions: the item, then the price.

fn pay(play: &mut PlayState, g: &EnGirlA) {
    rupees_change_by(&mut play.save, -g.base_price);
}

fn item_give_arrows(play: &mut PlayState, g: &EnGirlA) {
    inventory_change_ammo(&mut play.save, ITEM_BOW, g.item_count);
    pay(play, g);
}

fn item_give_bombs(play: &mut PlayState, g: &EnGirlA) {
    match g.item_count {
        5 => {
            item_give(&mut play.save, Some(&mut play.audio), ITEM_BOMBS_5);
        }
        10 => {
            item_give(&mut play.save, Some(&mut play.audio), ITEM_BOMBS_10);
        }
        20 => {
            item_give(&mut play.save, Some(&mut play.audio), ITEM_BOMBS_20);
        }
        30 => {
            item_give(&mut play.save, Some(&mut play.audio), ITEM_BOMBS_30);
        }
        _ => {}
    }
    pay(play, g);
}

fn item_give_deku_nuts(play: &mut PlayState, g: &EnGirlA) {
    match g.item_count {
        5 => {
            item_give(&mut play.save, Some(&mut play.audio), ITEM_DEKU_NUTS_5);
        }
        10 => {
            item_give(&mut play.save, Some(&mut play.audio), ITEM_DEKU_NUTS_10);
        }
        _ => {}
    }
    pay(play, g);
}

fn item_give_deku_sticks(play: &mut PlayState, g: &EnGirlA) {
    item_give(&mut play.save, Some(&mut play.audio), ITEM_DEKU_STICK);
    pay(play, g);
}

fn item_give_longsword(play: &mut PlayState, g: &EnGirlA) {
    func_800849ec(&mut play.save);
    play.save.sword_health = 8;
    pay(play, g);
}

fn item_give_hylian_shield(play: &mut PlayState, g: &EnGirlA) {
    item_give(&mut play.save, Some(&mut play.audio), ITEM_SHIELD_HYLIAN);
    pay(play, g);
}

/// `EnGirlA_ItemGive_DekuShield`: owned (not worn), and paid for.
fn item_give_deku_shield(play: &mut PlayState, g: &EnGirlA) {
    item_give(&mut play.save, Some(&mut play.audio), ITEM_SHIELD_DEKU);
    pay(play, g);
}

fn item_give_goron_tunic(play: &mut PlayState, g: &EnGirlA) {
    item_give(&mut play.save, Some(&mut play.audio), ITEM_TUNIC_GORON);
    pay(play, g);
}

fn item_give_zora_tunic(play: &mut PlayState, g: &EnGirlA) {
    item_give(&mut play.save, Some(&mut play.audio), ITEM_TUNIC_ZORA);
    pay(play, g);
}

fn item_give_health(play: &mut PlayState, g: &EnGirlA) {
    health_change_by(&mut play.save, Some(&mut play.audio), g.item_count);
    pay(play, g);
}

fn item_give_milk_bottle(play: &mut PlayState, g: &EnGirlA) {
    item_give(&mut play.save, Some(&mut play.audio), ITEM_BOTTLE_MILK_FULL);
    pay(play, g);
}

fn item_give_weird_egg(play: &mut PlayState, g: &EnGirlA) {
    item_give(&mut play.save, Some(&mut play.audio), ITEM_WEIRD_EGG);
    pay(play, g);
}

fn item_give_unk19(play: &mut PlayState, g: &EnGirlA) {
    pay(play, g);
}

fn item_give_unk20(play: &mut PlayState, g: &EnGirlA) {
    pay(play, g);
}

fn item_give_deku_seeds(play: &mut PlayState, g: &EnGirlA) {
    item_give(&mut play.save, Some(&mut play.audio), ITEM_DEKU_SEEDS_30);
    pay(play, g);
}

fn item_give_bottled_item(play: &mut PlayState, g: &EnGirlA) {
    let item = match g.actor.params {
        SI_FISH => Some(ITEM_BOTTLE_FISH),
        SI_RED_POTION_R30 => Some(ITEM_BOTTLE_POTION_RED),
        SI_GREEN_POTION => Some(ITEM_BOTTLE_POTION_GREEN),
        SI_BLUE_POTION => Some(ITEM_BOTTLE_POTION_BLUE),
        SI_BLUE_FIRE => Some(ITEM_BOTTLE_BLUE_FIRE),
        SI_BUGS => Some(ITEM_BOTTLE_BUG),
        SI_BIG_POE => Some(ITEM_BOTTLE_BIG_POE),
        SI_POE => Some(ITEM_BOTTLE_POE),
        SI_FAIRY => Some(ITEM_BOTTLE_FAIRY),
        _ => None,
    };
    if let Some(i) = item {
        item_give(&mut play.save, Some(&mut play.audio), i);
    }
    pay(play, g);
}

// The `EnGirlA_BuyEvent_*` functions: the price after a fanfare item's get-item.

/// `EnGirlA_BuyEvent_ShieldDiscount`: the price, less a random discount for the Hylian Shield
/// once Zelda's letter has been shown in Kakariko.
fn buy_event_shield_discount(play: &mut PlayState, g: &EnGirlA) {
    if g.actor.params == SI_HYLIAN_SHIELD && play.save.get_inf_table(INFTABLE_76) {
        let i = play.rand.zero_float(7.9) as i32 as usize;
        rupees_change_by(&mut play.save, -(g.base_price - SHIELD_DISCOUNTS[i]));
        return;
    }
    pay(play, g);
}

fn buy_event_goron_tunic(play: &mut PlayState, g: &EnGirlA) {
    pay(play, g);
}

fn buy_event_zora_tunic(play: &mut PlayState, g: &EnGirlA) {
    pay(play, g);
}

fn buy_event_obtain_bombchu_pack(play: &mut PlayState, g: &EnGirlA) {
    let flag = match g.actor.params {
        SI_BOMBCHU_10_2 => Some(ITEMGETINF_06),
        SI_BOMBCHU_10_3 => Some(ITEMGETINF_07),
        SI_BOMBCHU_20_3 => Some(ITEMGETINF_08),
        SI_BOMBCHU_20_4 => Some(ITEMGETINF_09),
        SI_BOMBCHU_10_4 => Some(ITEMGETINF_0A),
        SI_BOMBCHU_10_1 => Some(ITEMGETINF_03),
        SI_BOMBCHU_20_1 => Some(ITEMGETINF_04),
        SI_BOMBCHU_20_2 => Some(ITEMGETINF_05),
        _ => None,
    };
    if let Some(f) = flag {
        play.save.set_item_get_inf(f);
    }
    pay(play, g);
}

/// Indices into the render state's extras.
mod rs {
    /// `switches`: drawn (`actor.draw != NULL`); `angles`: `yRotation`.
    pub const DRAWN: usize = 0;
    pub const Y_ROTATION: usize = 0;
}

impl ActorImpl for EnGirlA {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnGirlA_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Initialize => self.initialize_item_action(play),
            Action::Update2 => self.update2(play),
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.switches = vec![self.drawn as u32];
        rs.angles = vec![self.y_rotation];
        rs
    }
    /// `EnGirlA_Draw`: `GetItem_Draw` turned by `yRotation` (the highlight isn't drawn).
    fn draw(&self, rs: &RenderState, play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        if rs.switches.get(rs::DRAWN).copied().unwrap_or(0) == 0 {
            return;
        }
        let Some(a) = &play.assets else { return };
        let y = rs.angles.get(rs::Y_ROTATION).copied().unwrap_or(0);
        // Matrix_RotateY(DEG_TO_RAD((yRotation * 360.0f) / 65536.0f), MTXMODE_APPLY).
        let m = actor_draw_matrix(rs) * Mat4::from_rotation_y(((y as f32 * 360.0) / 65536.0) * (std::f32::consts::PI / 180.0));
        oot_game::draw::get_item_draw(&a.items, self.gi_draw_id, m, play.gameplay_frames, view, out);
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
