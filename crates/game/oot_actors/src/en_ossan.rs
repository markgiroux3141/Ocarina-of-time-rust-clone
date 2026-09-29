//! `En_Ossan` (`ovl_En_Ossan/z_en_ossan.c`): a shopkeeper. `params` is the shop
//! (`OssanType`); the Kokiri shop's (`OSSAN_TYPE_KOKIRI`, 0) is the one ported.
//!
//! **The Kokiri shopkeeper** stands behind the counter (33 in front of his placement) until
//! Link talks to him (within 100: `func_8002F2CC`). Then:
//! 1. Link is hidden (`PLAYER_STATE2_29`), the shop's viewpoint turns to its browsing camera
//!    (`Play_SetShopBrowsingViewpoint`: bg camera 1, `PIVOT_SHOP_BROWSING`), and the
//!    shopkeeper drives the message box (`YREG(31)`): 0x9E "Welcome!", then 0x83, "Talk to the
//!    owner" or "Quit", with the stick prompts either side (`EnOssan_State_FacingShopkeeper`).
//! 2. The stick left or right turns the camera 30 degrees to that shelf
//!    (`EnOssan_UpdateCameraDirection`: `Camera_SetCameraData`'s `data2`, which `Camera_Data4`
//!    adds to the bg camera's yaw) and puts the cursor on its first item; the stick moves it,
//!    the item's description shows (`EnOssan_State_BrowseLeftShelf`, `_RightShelf`), and past
//!    the counter's end the camera turns back.
//! 3. A takes the item off the shelf towards Link (`EnOssan_TakeItemOffShelf`) with its buy
//!    prompt; "Buy" asks the item (`canBuyFunc`, `EnOssan_HandleCanBuyItem`): not enough
//!    rupees (0x85), can't get it now (0x86), a quick buy ("Thanks a lot!", 0x84), or, the
//!    first of a kind, the get-item flow: the shopkeeper offers it within 120
//!    (`func_8002F434`), closes the box, and Player takes it when his talk ends; after its
//!    text the item's price is charged (`buyEventFunc`) and 0x6B asks whether Link wants
//!    something else.
//! 4. B, "Quit" or "No" end it (`EnOssan_EndInteraction`).
//!
//! The shop's items are `En_GirlA`s on the `En_Tana` shelves (`EnOssan_SpawnItemsOnShelves`).
//!
//! Every state is ported (the milk, egg, bomb, mask and discount ones too), but only the
//! Kokiri shopkeeper's init, objects and draw: the other types log that they aren't ported and
//! stand as placeholders (`EnOssan_InitPotionShopkeeper`, `_BombchuShopkeeper`,
//! `_BazaarShopkeeper`, `_ZoraShopkeeper`, `_GoronShopkeeper`, `_HappyMaskShopkeeper` and their
//! draws). Left out: the sounds (`func_80078884`), the prints, and the unused collider.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{approach_f, step_to_s};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_0, ACTOR_FLAG_3, ACTOR_FLAG_4, Actor, UPDBGCHECKINFO_FLAG_0, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ACTORCAT_PROP, ActorHandle, ActorImpl, ActorProfile};
use oot_game::collision_check::MASS_IMMOVABLE;
use oot_game::gbi::{G_IM_FMT_IA, G_IM_SIZ_4B, G_IM_SIZ_8B, G_TX_MIRROR, G_TX_NOMASK, G_TX_WRAP, setup_dl};
use oot_game::interface::{DO_ACTION_DECIDE, DO_ACTION_NEXT, change_alpha};
use oot_game::message::*;
use oot_game::pack::{BakeBody, BakeSegment, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, VIEWPOINT_LOCKED, VIEWPOINT_PIVOT, ViewInfo};
use oot_game::skelanime_std::*;
use oot_game::sprite::{Load, Quad, Sprite, SpriteBake, TexSrc};

use crate::en_girla::*;
use crate::en_tana::ACTOR_EN_TANA;

/// `ACTOR_EN_OSSAN` (`actor_table.h`).
pub const ACTOR_EN_OSSAN: i16 = 0x003D;
/// `ACTOR_EN_ELF`: the Kokiri shopkeeper's fairy (`FAIRY_KOKIRI`, 3).
const ACTOR_EN_ELF: i16 = 0x0018;
const FAIRY_KOKIRI: i16 = 3;

/// `En_Ossan_InitVars` (no draw until the shopkeeper's init sets one).
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_OSSAN, name: "En_Ossan", category: ACTORCAT_NPC, flags: ACTOR_FLAG_0 | ACTOR_FLAG_3 | ACTOR_FLAG_4, object: "gameplay_keep" };

// `OssanType`.
pub const OSSAN_TYPE_KOKIRI: i16 = 0;
pub const OSSAN_TYPE_KAKARIKO_POTION: i16 = 1;
pub const OSSAN_TYPE_BOMBCHUS: i16 = 2;
pub const OSSAN_TYPE_MARKET_POTION: i16 = 3;
pub const OSSAN_TYPE_BAZAAR: i16 = 4;
pub const OSSAN_TYPE_ADULT: i16 = 5;
pub const OSSAN_TYPE_TALON: i16 = 6;
pub const OSSAN_TYPE_ZORA: i16 = 7;
pub const OSSAN_TYPE_GORON: i16 = 8;
pub const OSSAN_TYPE_INGO: i16 = 9;
pub const OSSAN_TYPE_MASK: i16 = 10;

/// `sShopkeeperPrintName`'s English: the shops, for the log.
const SHOPKEEPER_NAMES: [&str; 11] =
    ["Kokiri Shop", "Potion Shop", "Night Shop", "Back Alley Shop", "Shield Shop", "Adult Shop", "Talon Shop", "Zora Shop", "Goron Night Shop", "Ingo Store", "Mask Shop"];

// `EnOssanState`.
pub const OSSAN_STATE_IDLE: u8 = 0;
pub const OSSAN_STATE_START_CONVERSATION: u8 = 1;
pub const OSSAN_STATE_FACING_SHOPKEEPER: u8 = 2;
pub const OSSAN_STATE_TALKING_TO_SHOPKEEPER: u8 = 3;
pub const OSSAN_STATE_LOOK_SHELF_LEFT: u8 = 4;
pub const OSSAN_STATE_LOOK_SHELF_RIGHT: u8 = 5;
pub const OSSAN_STATE_BROWSE_LEFT_SHELF: u8 = 6;
pub const OSSAN_STATE_BROWSE_RIGHT_SHELF: u8 = 7;
pub const OSSAN_STATE_LOOK_SHOPKEEPER: u8 = 8;
pub const OSSAN_STATE_SELECT_ITEM: u8 = 9;
pub const OSSAN_STATE_SELECT_ITEM_MILK_BOTTLE: u8 = 10;
pub const OSSAN_STATE_SELECT_ITEM_WEIRD_EGG: u8 = 11;
pub const OSSAN_STATE_SELECT_ITEM_UNIMPLEMENTED: u8 = 12;
pub const OSSAN_STATE_SELECT_ITEM_BOMBS: u8 = 13;
pub const OSSAN_STATE_CANT_GET_ITEM: u8 = 14;
pub const OSSAN_STATE_GIVE_ITEM_FANFARE: u8 = 15;
pub const OSSAN_STATE_ITEM_PURCHASED: u8 = 16;
pub const OSSAN_STATE_CONTINUE_SHOPPING_PROMPT: u8 = 17;
pub const OSSAN_STATE_GIVE_LON_LON_MILK: u8 = 18;
pub const OSSAN_STATE_DISPLAY_ONLY_BOMB_DIALOG: u8 = 19;
pub const OSSAN_STATE_WAIT_FOR_DISPLAY_ONLY_BOMB_DIALOG: u8 = 20;
pub const OSSAN_STATE_21: u8 = 21;
pub const OSSAN_STATE_22: u8 = 22;
pub const OSSAN_STATE_QUICK_BUY: u8 = 23;
pub const OSSAN_STATE_SELECT_ITEM_MASK: u8 = 24;
pub const OSSAN_STATE_LEND_MASK_OF_TRUTH: u8 = 25;
pub const OSSAN_STATE_DISCOUNT_DIALOG: u8 = 26;

// `EnOssanHappyMaskState`.
const OSSAN_HAPPY_STATE_REQUEST_PAYMENT_KEATON_MASK: u8 = 0;
const OSSAN_HAPPY_STATE_REQUEST_PAYMENT_SPOOKY_MASK: u8 = 1;
const OSSAN_HAPPY_STATE_REQUEST_PAYMENT_SKULL_MASK: u8 = 2;
const OSSAN_HAPPY_STATE_REQUEST_PAYMENT_BUNNY_HOOD: u8 = 3;
const OSSAN_HAPPY_STATE_BORROWED_FIRST_MASK: u8 = 4;
const OSSAN_HAPPY_STATE_ANGRY: u8 = 5;
const OSSAN_HAPPY_STATE_ALL_MASKS_SOLD: u8 = 6;
const OSSAN_HAPPY_STATE_NONE: u8 = 8;

/// `CURSOR_INVALID`.
const CURSOR_INVALID: u8 = 0xFF;

// `OBJECT_*` (`object_table.h`): `sShopkeeperObjectIds`' objects; `OBJECT_ID_MAX` is none.
const OBJECT_OSSAN: i16 = 0x005B;
const OBJECT_OF1D_MAP: i16 = 0x00C9;
const OBJECT_KM1: i16 = 0x00FC;
const OBJECT_ZO: i16 = 0x00FE;
const OBJECT_MASTERKOKIRI: i16 = 0x0101;
const OBJECT_MASTERKOKIRIHEAD: i16 = 0x0102;
const OBJECT_MASTERGOLON: i16 = 0x0103;
const OBJECT_MASTERZOORA: i16 = 0x0104;
const OBJECT_OS: i16 = 0x013E;
const OBJECT_DS2: i16 = 0x0159;
const OBJECT_RS: i16 = 0x0165;
const OBJECT_ID_MAX: i16 = 0x0192;

/// `sShopkeeperObjectIds`: the skeleton's object, then the Kokiri head's and the animation's.
const SHOPKEEPER_OBJECT_IDS: [[i16; 3]; 11] = [
    [OBJECT_KM1, OBJECT_MASTERKOKIRIHEAD, OBJECT_MASTERKOKIRI],
    [OBJECT_DS2, OBJECT_ID_MAX, OBJECT_ID_MAX],
    [OBJECT_RS, OBJECT_ID_MAX, OBJECT_ID_MAX],
    [OBJECT_DS2, OBJECT_ID_MAX, OBJECT_ID_MAX],
    [OBJECT_OSSAN, OBJECT_ID_MAX, OBJECT_ID_MAX],
    [OBJECT_OSSAN, OBJECT_ID_MAX, OBJECT_ID_MAX],
    [OBJECT_OSSAN, OBJECT_ID_MAX, OBJECT_ID_MAX],
    [OBJECT_ZO, OBJECT_ID_MAX, OBJECT_MASTERZOORA],
    [OBJECT_OF1D_MAP, OBJECT_ID_MAX, OBJECT_MASTERGOLON],
    [OBJECT_OSSAN, OBJECT_ID_MAX, OBJECT_ID_MAX],
    [OBJECT_OS, OBJECT_ID_MAX, OBJECT_ID_MAX],
];

/// `sShopkeeperScale`.
const SHOPKEEPER_SCALE: [f32; 11] = [0.01, 0.011, 0.0105, 0.011, 0.01, 0.01, 0.01, 0.01, 0.01, 0.01, 0.01];

/// `sShopkeeperPositionOffsets`.
const SHOPKEEPER_POSITION_OFFSETS: [[f32; 3]; 11] =
    [[0.0, 0.0, 33.0], [0.0, 0.0, 31.0], [0.0, 0.0, 31.0], [0.0, 0.0, 31.0], [0.0; 3], [0.0; 3], [0.0; 3], [0.0, 0.0, 36.0], [0.0, 0.0, 15.0], [0.0; 3], [0.0, 0.0, 26.0]];

/// `sItemShelfRot`: each slot's yaw on the shelves.
const ITEM_SHELF_ROT: [u16; 8] = [0xEAAC, 0xEAAC, 0xEAAC, 0xEAAC, 0x1554, 0x1554, 0x1554, 0x1554];

/// `sSelectedItemPosition`: where a chosen item moves to (right shelf, left shelf).
const SELECTED_ITEM_POSITION: [[f32; 3]; 2] = [[17.0, 58.0, 30.0], [-17.0, 58.0, 30.0]];

/// `sMaskPaymentPrice`: what the Happy Mask Shop asks back.
const MASK_PAYMENT_PRICE: [i16; 4] = [10, 30, 20, 50];

/// `ShopItem`: the item and its offset from the shelves.
#[derive(Debug, Clone, Copy)]
pub struct ShopItem {
    pub shop_item_index: i16,
    pub x: i16,
    pub y: i16,
    pub z: i16,
}

const fn si(shop_item_index: i16, x: i16, y: i16, z: i16) -> ShopItem {
    ShopItem { shop_item_index, x, y, z }
}

/// `sShopkeeperStores`: each shop's eight slots, laid out
/// ```text
/// 7 5  3 1
/// 6 4  2 0
/// ```
/// (the right shelf is slots 0 to 3, the bottom row the even slots).
pub const SHOPKEEPER_STORES: [[ShopItem; 8]; 11] = [
    [
        si(SI_DEKU_SHIELD, 50, 52, -20),
        si(SI_DEKU_NUTS_5, 50, 76, -20),
        si(SI_DEKU_NUTS_10, 80, 52, -3),
        si(SI_DEKU_STICK, 80, 76, -3),
        si(SI_DEKU_SEEDS_30, -50, 52, -20),
        si(SI_ARROWS_10, -50, 76, -20),
        si(SI_ARROWS_30, -80, 52, -3),
        si(SI_RECOVERY_HEART, -80, 76, -3),
    ],
    [
        si(SI_GREEN_POTION, 50, 52, -20),
        si(SI_BLUE_FIRE, 50, 76, -20),
        si(SI_RED_POTION_R30, 80, 52, -3),
        si(SI_FAIRY, 80, 76, -3),
        si(SI_DEKU_NUTS_5, -50, 52, -20),
        si(SI_BUGS, -50, 76, -20),
        si(SI_POE, -80, 52, -3),
        si(SI_FISH, -80, 76, -3),
    ],
    [
        si(SI_BOMBCHU_10_2, 50, 52, -20),
        si(SI_BOMBCHU_10_4, 50, 76, -20),
        si(SI_BOMBCHU_10_3, 80, 52, -3),
        si(SI_BOMBCHU_10_1, 80, 76, -3),
        si(SI_BOMBCHU_20_3, -50, 52, -20),
        si(SI_BOMBCHU_20_1, -50, 76, -20),
        si(SI_BOMBCHU_20_4, -80, 52, -3),
        si(SI_BOMBCHU_20_2, -80, 76, -3),
    ],
    [
        si(SI_GREEN_POTION, 50, 52, -20),
        si(SI_BLUE_FIRE, 50, 76, -20),
        si(SI_RED_POTION_R30, 80, 52, -3),
        si(SI_FAIRY, 80, 76, -3),
        si(SI_DEKU_NUTS_5, -50, 52, -20),
        si(SI_BUGS, -50, 76, -20),
        si(SI_POE, -80, 52, -3),
        si(SI_FISH, -80, 76, -3),
    ],
    [
        si(SI_HYLIAN_SHIELD, 50, 52, -20),
        si(SI_BOMBS_5_R35, 50, 76, -20),
        si(SI_DEKU_NUTS_5, 80, 52, -3),
        si(SI_RECOVERY_HEART, 80, 76, -3),
        si(SI_ARROWS_10, -50, 52, -20),
        si(SI_ARROWS_50, -50, 76, -20),
        si(SI_DEKU_STICK, -80, 52, -3),
        si(SI_ARROWS_30, -80, 76, -3),
    ],
    [
        si(SI_HYLIAN_SHIELD, 50, 52, -20),
        si(SI_BOMBS_5_R25, 50, 76, -20),
        si(SI_DEKU_NUTS_5, 80, 52, -3),
        si(SI_RECOVERY_HEART, 80, 76, -3),
        si(SI_ARROWS_10, -50, 52, -20),
        si(SI_ARROWS_50, -50, 76, -20),
        si(SI_DEKU_STICK, -80, 52, -3),
        si(SI_ARROWS_30, -80, 76, -3),
    ],
    [
        si(SI_MILK_BOTTLE, 50, 52, -20),
        si(SI_DEKU_NUTS_5, 50, 76, -20),
        si(SI_DEKU_NUTS_10, 80, 52, -3),
        si(SI_RECOVERY_HEART, 80, 76, -3),
        si(SI_WEIRD_EGG, -50, 52, -20),
        si(SI_DEKU_STICK, -50, 76, -20),
        si(SI_RECOVERY_HEART, -80, 52, -3),
        si(SI_RECOVERY_HEART, -80, 76, -3),
    ],
    [
        si(SI_ZORA_TUNIC, 50, 52, -20),
        si(SI_ARROWS_10, 50, 76, -20),
        si(SI_RECOVERY_HEART, 80, 52, -3),
        si(SI_ARROWS_30, 80, 76, -3),
        si(SI_DEKU_NUTS_5, -50, 52, -20),
        si(SI_ARROWS_50, -50, 76, -20),
        si(SI_FISH, -80, 52, -3),
        si(SI_RED_POTION_R50, -80, 76, -3),
    ],
    [
        si(SI_BOMBS_5_R25, 50, 52, -20),
        si(SI_BOMBS_10, 50, 76, -20),
        si(SI_BOMBS_20, 80, 52, -3),
        si(SI_BOMBS_30, 80, 76, -3),
        si(SI_GORON_TUNIC, -50, 52, -20),
        si(SI_RECOVERY_HEART, -50, 76, -20),
        si(SI_RED_POTION_R40, -80, 52, -3),
        si(SI_RECOVERY_HEART, -80, 76, -3),
    ],
    [si(SI_19, 50, 52, -20), si(SI_19, 50, 76, -20), si(SI_19, 80, 52, -3), si(SI_19, 80, 76, -3), si(SI_20, -50, 52, -20), si(SI_20, -50, 76, -20), si(SI_20, -80, 52, -3), si(SI_20, -80, 76, -3)],
    [
        si(SI_GERUDO_MASK, 50, 52, -20),
        si(SI_ZORA_MASK, 50, 76, -20),
        si(SI_MASK_OF_TRUTH, 80, 52, -3),
        si(SI_GORON_MASK, 80, 76, -3),
        si(SI_SKULL_MASK, -50, 52, -20),
        si(SI_KEATON_MASK, -50, 76, -20),
        si(SI_BUNNY_HOOD, -80, 52, -3),
        si(SI_SPOOKY_MASK, -80, 76, -3),
    ],
];

// `ITEMGETINF_*`, `EVENTCHKINF_*`, `INFTABLE_*` (`z64save.h`) the other shops read.
const ITEMGETINF_23: u16 = 0x23;
const ITEMGETINF_24: u16 = 0x24;
const ITEMGETINF_25: u16 = 0x25;
const ITEMGETINF_26: u16 = 0x26;
const ITEMGETINF_2A: u16 = 0x2A;
const ITEMGETINF_3F: u16 = 0x3F;
const EVENTCHKINF_25: u16 = 0x25;
const EVENTCHKINF_8C: u16 = 0x8C;
const EVENTCHKINF_8D: u16 = 0x8D;
const EVENTCHKINF_8E: u16 = 0x8E;
const EVENTCHKINF_8F: u16 = 0x8F;
const INFTABLE_FC: u16 = 0xFC;
/// `QUEST_MEDALLION_FIRE`.
const QUEST_MEDALLION_FIRE: u32 = 0x01;
/// `ENTR_MARKET_DAY_9` (`entrance_table.h`: 0x1D1), and
/// `TRANS_TYPE_CIRCLE(TCA_STARBURST, TCC_WHITE, TCS_FAST)` (0x20 | 1 << 3 | 3 << 1 | 0).
const ENTR_MARKET_DAY_9: u16 = 0x01D1;
const TRANS_TYPE_CIRCLE_STARBURST_WHITE_FAST: u8 = 0x2E;

/// `sShopItemReplaceFunc` (`ShopItemDisp_*`): -1 for a mask the Happy Mask Shop hasn't
/// unlocked yet.
fn shop_item_disp(v: i16, s: &oot_game::save::SaveContext) -> i16 {
    let needs = match v {
        // ShopItemDisp_SpookyMask: the Skull Mask sold.
        SI_SPOOKY_MASK => Some(ITEMGETINF_39),
        // ShopItemDisp_SkullMask: the Keaton Mask sold.
        SI_SKULL_MASK => Some(ITEMGETINF_38),
        // ShopItemDisp_BunnyHood: the Spooky Mask sold.
        SI_BUNNY_HOOD => Some(ITEMGETINF_3A),
        // ShopItemDisp_ZoraMask, _GoronMask, _GerudoMask: the Mask of Truth got.
        SI_ZORA_MASK | SI_GORON_MASK | SI_GERUDO_MASK => Some(ITEMGETINF_3F),
        _ => None,
    };
    match needs {
        Some(f) if !s.get_item_get_inf(f) => -1,
        _ => v,
    }
}

// The Kokiri shopkeeper's draw (`EnOssan_DrawKokiriShopkeeper`).
/// `sKokiriShopkeeperEyeTextures` (`object_masterkokirihead`), on segment 0x0A.
const KOKIRI_EYES: [&str; 3] = ["gKokiriShopkeeperEyeDefaultTex", "gKokiriShopkeeperEyeHalfTex", "gKokiriShopkeeperEyeOpenTex"];
/// `gSPSegment(0x08, EnOssan_SetEnvColor(0, 130, 70, 255))`, `(0x09, ...(110, 170, 20, 255))`,
/// `(0x0C, EnOssan_EmptyDList)`, the eyes on 0x0A, and the direct `gDPSetEnvColor(0, 0, 0,
/// 255)` run first (0x0E).
const SEG_TUNIC: u8 = 0x08;
const SEG_BOOTS: u8 = 0x09;
const SEG_EYES: u8 = 0x0A;
const SEG_EMPTY: u8 = 0x0C;
const SEG_ENV: u8 = 0x0E;

/// `gsDPSetEnvColor(r, g, b, a)`.
const fn set_env_color(r: u8, g: u8, b: u8, a: u8) -> (u32, u32) {
    (0xFB00_0000, ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | a as u32)
}

fn kokiri_bake(eye: usize) -> String {
    format!("En_Ossan/kokiri_eye{eye}")
}

/// The cursor and the stick prompts' sprites (`EnOssan_DrawCursor`,
/// `EnOssan_DrawStickDirectionPrompts`).
pub const CURSOR_SPRITE: &str = "En_Ossan/cursor";
pub const ARROW_SPRITE: &str = "En_Ossan/arrow";
pub const STICK_SPRITE: &str = "En_Ossan/stick";

/// The meshes: the Kokiri shopkeeper with each eye (`object_km1`'s `gKm1Skel`, his head
/// `gKokiriShopkeeperHeadDL` from `object_masterkokirihead` on limb 15), and the cursor's and
/// prompts' sprites.
pub fn bakes() -> Vec<MeshBake> {
    let mut v: Vec<MeshBake> = KOKIRI_EYES
        .iter()
        .enumerate()
        .map(|(eye, tex)| MeshBake {
            name: kokiri_bake(eye),
            object: "object_km1".into(),
            segments: vec![
                (SEG_TUNIC, BakeSegment::Commands(vec![set_env_color(0, 130, 70, 255)])),
                (SEG_BOOTS, BakeSegment::Commands(vec![set_env_color(110, 170, 20, 255)])),
                (SEG_EMPTY, BakeSegment::Commands(Vec::new())),
                (SEG_EYES, BakeSegment::Texture { file: "object_masterkokirihead".into(), symbol: (*tex).into() }),
                (SEG_ENV, BakeSegment::Commands(vec![set_env_color(0, 0, 0, 255)])),
            ],
            prelude: vec![SEG_ENV],
            body: BakeBody::Skeleton {
                file: "object_km1".into(),
                symbol: "gKm1Skel".into(),
                limbs: vec![LimbOverride { limb: 14, file: "object_masterkokirihead".into(), symbol: "gKokiriShopkeeperHeadDL".into() }],
            },
        })
        .collect();
    v.extend(sprite_bakes().iter().map(|b| b.mesh_bake()));
    v
}

/// The sprites: `gSelectionCursorTex` (IA4 16x16, mirrored and wrapped, masks 4) after
/// `Gfx_SetupDL_39Overlay`; `gArrowCursorTex` (IA8 16x24) and `gControlStickTex` (IA8 16x16),
/// wrapped across (mask 4), after it with `G_CC_MODULATEIA_PRIM`. Each rectangle of
/// `EnOssan_DrawTextRec` is 16x24 at one texel a pixel, so the stick's reads 24 rows of its 16
/// (no mask down: past its end, what TMEM holds; the bake's texture repeats).
pub fn sprite_bakes() -> Vec<SpriteBake> {
    let keep = |symbol: &str| TexSrc::Symbol { file: "gameplay_keep".into(), symbol: symbol.into() };
    let mirror_wrap = G_TX_MIRROR | G_TX_WRAP;
    let mut modulate = setup_dl::setup_dl_39();
    modulate.combine_lerp(setup_dl::MODULATEIA_PRIM, setup_dl::MODULATEIA_PRIM);
    vec![
        SpriteBake {
            name: CURSOR_SPRITE.into(),
            tex: keep("gSelectionCursorTex"),
            load: Load { fmt: G_IM_FMT_IA, siz: G_IM_SIZ_4B, width: 16, height: 16, cms: mirror_wrap, cmt: mirror_wrap, masks: 4, maskt: 4 },
            setup: setup_dl::setup_dl_39(),
            prim: true,
            env: false,
            quad: Quad::Rect { s: 32, t: 32 },
        },
        SpriteBake {
            name: ARROW_SPRITE.into(),
            tex: keep("gArrowCursorTex"),
            load: Load { fmt: G_IM_FMT_IA, siz: G_IM_SIZ_8B, width: 16, height: 24, cms: G_TX_WRAP, cmt: G_TX_WRAP, masks: 4, maskt: G_TX_NOMASK },
            setup: modulate.clone(),
            prim: true,
            env: false,
            quad: Quad::Rect { s: 16, t: 24 },
        },
        SpriteBake {
            name: STICK_SPRITE.into(),
            tex: keep("gControlStickTex"),
            load: Load { fmt: G_IM_FMT_IA, siz: G_IM_SIZ_8B, width: 16, height: 16, cms: G_TX_WRAP, cmt: G_TX_WRAP, masks: 4, maskt: G_TX_NOMASK },
            setup: modulate,
            prim: true,
            env: false,
            quad: Quad::Rect { s: 16, t: 24 },
        },
    ]
}

/// `StickDirectionPrompt`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StickDirectionPrompt {
    pub stick_color: [u32; 4],
    pub stick_tex_x: f32,
    pub stick_tex_y: f32,
    pub arrow_color: [u32; 4],
    pub arrow_tex_x: f32,
    pub arrow_tex_y: f32,
    pub z: f32,
    pub is_enabled: bool,
}

impl StickDirectionPrompt {
    /// `EnOssan_InitActionFunc`'s setup, with the stick's and the arrow's x.
    fn new(stick_tex_x: f32, arrow_tex_x: f32) -> Self {
        StickDirectionPrompt { stick_color: [200, 200, 200, 180], stick_tex_x, stick_tex_y: 95.0, arrow_color: [255, 255, 0, 200], arrow_tex_x, arrow_tex_y: 91.0, z: 1.0, is_enabled: false }
    }
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `EnOssan_InitActionFunc`: waiting for the objects and the shelves.
    Init,
    /// `EnOssan_MainActionFunc`.
    Main,
}

/// `blinkFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blink {
    /// `EnOssan_WaitForBlink`.
    Wait,
    /// `EnOssan_Blink`.
    Blink,
}

pub struct EnOssan {
    pub actor: Actor,
    pub skel: Option<SkelAnimeStd>,
    pub skeleton: Option<Arc<Skeleton>>,
    pub action: Action,
    pub timer: i16,
    pub delay_timer: i16,
    pub obj_bank_index1: Option<usize>,
    pub obj_bank_index2: Option<usize>,
    pub obj_bank_index3: Option<usize>,
    pub happy_mask_shop_state: u8,
    pub happy_mask_shopkeeper_eye_idx: u8,
    pub head_rot: i16,
    pub head_target_rot: i16,
    pub eye_texture_idx: i16,
    pub blink_timer: i16,
    pub blink: Blink,
    pub state_flag: u8,
    pub temp_state_flag: u8,
    pub shelf_slots: [Option<ActorHandle>; 8],
    pub shelves: Option<ActorHandle>,
    pub stick_accum_x: i32,
    pub stick_accum_y: i32,
    pub move_horizontal: bool,
    pub move_vertical: bool,
    pub cursor_x: f32,
    pub cursor_y: f32,
    pub cursor_z: f32,
    pub cursor_color: [u32; 4],
    pub cursor_anim_tween: f32,
    pub cursor_anim_state: u8,
    pub draw_cursor: u8,
    pub cursor_index: u8,
    pub stick_left_prompt: StickDirectionPrompt,
    pub stick_right_prompt: StickDirectionPrompt,
    pub arrow_anim_tween: f32,
    pub stick_anim_tween: f32,
    pub arrow_anim_state: u8,
    pub stick_anim_state: u8,
    pub shop_item_selected_tween: f32,
    /// `cameraFaceAngle`, in degrees.
    pub camera_face_angle: f32,
}

/// Runs `f` on shelf item `h` with the play state (the item is taken out of the actor context
/// meanwhile, as the C calls its functions with both).
fn with_item<R>(play: &mut PlayState, h: Option<ActorHandle>, f: impl FnOnce(&mut EnGirlA, &mut PlayState) -> R) -> Option<R> {
    let h = h?;
    let mut a = play.actors.take(h)?;
    let r = a.as_any_mut().downcast_mut::<EnGirlA>().map(|g| f(g, play));
    play.actors.put_back(h, a);
    r
}

impl EnOssan {
    fn ty(&self) -> usize {
        self.actor.params as usize
    }

    /// `EnOssan_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut o = EnOssan {
            actor,
            skel: None,
            skeleton: None,
            action: Action::Init,
            timer: 0,
            delay_timer: 0,
            obj_bank_index1: None,
            obj_bank_index2: None,
            obj_bank_index3: None,
            happy_mask_shop_state: 0,
            happy_mask_shopkeeper_eye_idx: 0,
            head_rot: 0,
            head_target_rot: 0,
            eye_texture_idx: 0,
            blink_timer: 0,
            blink: Blink::Wait,
            state_flag: OSSAN_STATE_IDLE,
            temp_state_flag: 0,
            shelf_slots: [None; 8],
            shelves: None,
            stick_accum_x: 0,
            stick_accum_y: 0,
            move_horizontal: false,
            move_vertical: false,
            cursor_x: 0.0,
            cursor_y: 0.0,
            cursor_z: 0.0,
            cursor_color: [0; 4],
            cursor_anim_tween: 0.0,
            cursor_anim_state: 0,
            draw_cursor: 0,
            cursor_index: 0,
            stick_left_prompt: StickDirectionPrompt::default(),
            stick_right_prompt: StickDirectionPrompt::default(),
            arrow_anim_tween: 0.0,
            stick_anim_tween: 0.0,
            arrow_anim_state: 0,
            stick_anim_state: 0,
            shop_item_selected_tween: 0.0,
            camera_face_angle: 0.0,
        };
        let s = &play.save;
        if o.actor.params == OSSAN_TYPE_TALON && s.adult {
            o.actor.params = OSSAN_TYPE_INGO;
        }
        // @bug (game): `params > OSSAN_TYPE_MASK && params < OSSAN_TYPE_KOKIRI` is never true
        // (it should be `||`), so a bad params indexes past the tables. The port kills it.
        if !(OSSAN_TYPE_KOKIRI..=OSSAN_TYPE_MASK).contains(&o.actor.params) {
            log::error!("En_Ossan: bad params {} (arg_data)", o.actor.params);
            o.actor.kill();
            return Box::new(o);
        }
        // The Happy Mask Shop opens once Zelda's letter has been shown in Kakariko.
        if o.actor.params == OSSAN_TYPE_MASK && !s.get_inf_table(INFTABLE_76) {
            o.actor.kill();
            return Box::new(o);
        }
        if o.actor.params == OSSAN_TYPE_KAKARIKO_POTION && !s.adult {
            o.actor.kill();
            return Box::new(o);
        }
        // Dodongo's Cavern done.
        if o.actor.params == OSSAN_TYPE_BOMBCHUS && !s.get_event_chk_inf(EVENTCHKINF_25) {
            o.actor.kill();
            return Box::new(o);
        }
        let ids = SHOPKEEPER_OBJECT_IDS[o.ty()];
        o.obj_bank_index1 = play.object_ctx.get_index(ids[0]);
        if o.obj_bank_index1.is_none() {
            // "No bank!!"
            log::warn!("En_Ossan: no bank ({})", SHOPKEEPER_NAMES[o.ty()]);
            o.actor.kill();
            return Box::new(o);
        }
        if !o.try_get_obj_bank_indices(play, ids) {
            // "No spare bank!!"
            log::warn!("En_Ossan: no spare bank ({})", SHOPKEEPER_NAMES[o.ty()]);
            o.actor.kill();
            return Box::new(o);
        }
        if o.actor.params != OSSAN_TYPE_KOKIRI {
            log::info!("En_Ossan: the {} shopkeeper (type {}) isn't ported: a placeholder", SHOPKEEPER_NAMES[o.ty()], o.actor.params);
            return oot_game::spawn::Placeholder::init(o.actor, play);
        }
        // Actor_ProcessInitChain: targetMode 2, targetArrowOffset 500.
        o.actor.target_mode = 2;
        o.actor.target_arrow_offset = 500.0;
        o.action = Action::Init;
        Box::new(o)
    }

    /// `EnOssan_TryGetObjBankIndices`.
    fn try_get_obj_bank_indices(&mut self, play: &PlayState, ids: [i16; 3]) -> bool {
        self.obj_bank_index2 = None;
        if ids[1] != OBJECT_ID_MAX {
            self.obj_bank_index2 = play.object_ctx.get_index(ids[1]);
            if self.obj_bank_index2.is_none() {
                return false;
            }
        }
        self.obj_bank_index3 = None;
        if ids[2] != OBJECT_ID_MAX {
            self.obj_bank_index3 = play.object_ctx.get_index(ids[2]);
            if self.obj_bank_index3.is_none() {
                return false;
            }
        }
        true
    }

    /// `EnOssan_AreShopkeeperObjectsLoaded`.
    fn are_shopkeeper_objects_loaded(&self, play: &PlayState) -> bool {
        let loaded = |b: Option<usize>| b.is_none_or(|b| play.object_ctx.is_loaded(b));
        self.obj_bank_index1.is_some_and(|b| play.object_ctx.is_loaded(b)) && loaded(self.obj_bank_index2) && loaded(self.obj_bank_index3)
    }

    /// `EnOssan_InitActionFunc`: once the objects are in and the shelves found, the shopkeeper
    /// set up behind the counter, his items spawned, and `EnOssan_MainActionFunc`.
    fn init_action_func(&mut self, play: &mut PlayState) {
        if !self.are_shopkeeper_objects_loaded(play) {
            return;
        }
        self.actor.flags &= !ACTOR_FLAG_4;
        self.actor.obj_bank_index = self.obj_bank_index1;
        self.shelves = play.actors.find(ACTOR_EN_TANA, ACTORCAT_PROP);
        if self.shelves.is_none() {
            // "Warning!! There are no shelves!!"
            log::warn!("En_Ossan: there are no shelves (En_Tana)");
            return;
        }
        let off = SHOPKEEPER_POSITION_OFFSETS[self.ty()];
        self.actor.world_pos += Vec3::from(off);
        // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 20).
        self.actor.shape_y_offset = 0.0;
        if !self.init_kokiri_shopkeeper(play) {
            return;
        }
        self.actor.text_id = self.setup_hello_dialog(play);
        self.cursor_x = 100.0;
        self.cursor_y = 100.0;
        self.actor.col_chk_info.mass = MASS_IMMOVABLE;
        self.actor.col_chk_info.cyl_radius = 50;
        self.state_flag = OSSAN_STATE_IDLE;
        self.stick_accum_x = 0;
        self.stick_accum_y = 0;
        self.cursor_index = 0;
        self.cursor_z = 1.5;
        self.cursor_color = [0, 255, 80, 255];
        self.cursor_anim_tween = 0.0;
        self.cursor_anim_state = 0;
        self.draw_cursor = 0;
        self.happy_mask_shopkeeper_eye_idx = 0;
        self.stick_left_prompt = StickDirectionPrompt::new(49.0, 33.0);
        self.stick_right_prompt = StickDirectionPrompt::new(274.0, 290.0);
        self.arrow_anim_state = 0;
        self.stick_anim_state = 0;
        self.arrow_anim_tween = 0.0;
        self.stick_anim_tween = 0.0;
        self.shop_item_selected_tween = 0.0;
        self.actor.scale = Vec3::splat(SHOPKEEPER_SCALE[self.ty()]);
        self.spawn_items_on_shelves(play);
        self.head_rot = 0;
        self.head_target_rot = 0;
        self.blink_timer = 20;
        self.eye_texture_idx = 0;
        self.blink = Blink::Wait;
        self.actor.flags &= !ACTOR_FLAG_0;
        self.action = Action::Main;
    }

    /// `EnOssan_InitKokiriShopkeeper`: `object_km1`'s skeleton playing
    /// `object_masterkokiri_Anim_0004A8` looped, and his fairy.
    fn init_kokiri_shopkeeper(&mut self, play: &mut PlayState) -> bool {
        let Some(assets) = play.assets.clone() else { return false };
        let loaded = assets.skeleton("object_km1", "gKm1Skel").and_then(|s| Ok((s, assets.animation("object_masterkokiri", "object_masterkokiri_Anim_0004A8")?)));
        let (skeleton, anim) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Ossan: {e:#}");
                self.actor.kill();
                return false;
            }
        };
        let mut skel = SkelAnimeStd::init_flex(skeleton.limbs.len(), None);
        let last = anim.last_frame();
        skel.change(anim, 1.0, 0.0, last, ANIMMODE_LOOP, 0.0);
        self.skel = Some(skel);
        self.skeleton = Some(skeleton);
        let pos = self.actor.world_pos;
        if let Err(e) = play.actor_spawn_as_child(&mut self.actor, ACTOR_EN_ELF, pos, [0; 3], FAIRY_KOKIRI) {
            log::debug!("En_Ossan's fairy: {e:?}");
        }
        true
    }

    /// `EnOssan_SpawnItemsOnShelves`: each slot's `En_GirlA` at its offset from the shelves,
    /// turned to face out.
    fn spawn_items_on_shelves(&mut self, play: &mut PlayState) {
        let Some(shelves) = self.shelves.and_then(|h| play.actors.actor(h)).map(|a| (a.world_pos, a.shape_rot)) else { return };
        let (pos, rot) = shelves;
        for (i, item) in SHOPKEEPER_STORES[self.ty()].iter().enumerate() {
            self.shelf_slots[i] = None;
            if item.shop_item_index < 0 {
                continue;
            }
            let params = shop_item_disp(item.shop_item_index, &play.save);
            if params < 0 {
                continue;
            }
            let at = pos + Vec3::new(item.x as f32, item.y as f32, item.z as f32);
            match play.actor_spawn(ACTOR_EN_GIRLA, at, [rot.x, rot.y.wrapping_add(ITEM_SHELF_ROT[i] as i16), rot.z], params) {
                Ok(h) => self.shelf_slots[i] = Some(h),
                Err(e) => log::warn!("En_Ossan: shelf item {i} ({params}): {e:?}"),
            }
        }
    }

    /// `EnOssan_UpdateShopOfferings`: the Happy Mask Shop's newly unlocked masks.
    fn update_shop_offerings(&mut self, play: &mut PlayState) {
        if self.actor.params != OSSAN_TYPE_MASK {
            return;
        }
        let Some((pos, rot)) = self.shelves.and_then(|h| play.actors.actor(h)).map(|a| (a.world_pos, a.shape_rot)) else { return };
        for (i, item) in SHOPKEEPER_STORES[self.ty()].iter().enumerate() {
            if item.shop_item_index >= 0 && self.shelf_slots[i].is_none() {
                let params = shop_item_disp(item.shop_item_index, &play.save);
                if params >= 0 {
                    let at = pos + Vec3::new(item.x as f32, item.y as f32, item.z as f32);
                    self.shelf_slots[i] = play.actor_spawn(ACTOR_EN_GIRLA, at, [rot.x, rot.y.wrapping_add(ITEM_SHELF_ROT[i] as i16), rot.z], params).ok();
                }
            }
        }
    }

    /// `sShopkeeperTalkOwner`: what the shopkeeper says when Link talks to him
    /// (`EnOssan_TalkKokiriShopkeeper` and the rest).
    fn talk_owner(&self, play: &mut PlayState) {
        let s = &play.save;
        let text = match self.actor.params {
            OSSAN_TYPE_KOKIRI => 0x10BA,
            OSSAN_TYPE_KAKARIKO_POTION => {
                if play.cur_spawn == 0 {
                    0x5046
                } else {
                    0x504E
                }
            }
            OSSAN_TYPE_BOMBCHUS => 0x7076,
            OSSAN_TYPE_MARKET_POTION => 0x504E,
            OSSAN_TYPE_BAZAAR => {
                if play.cur_spawn == 0 {
                    0x9D
                } else {
                    0x9C
                }
            }
            OSSAN_TYPE_ZORA => {
                if !s.adult {
                    0x403A
                } else {
                    0x403B
                }
            }
            OSSAN_TYPE_GORON => {
                if !s.adult {
                    if s.get_event_chk_inf(EVENTCHKINF_25) {
                        0x3028
                    } else if s.cur_upg_value(oot_game::item::UPG_STRENGTH) != 0 {
                        0x302D
                    } else {
                        0x300F
                    }
                } else if !s.check_quest_item(QUEST_MEDALLION_FIRE) {
                    0x3057
                } else {
                    0x305B
                }
            }
            OSSAN_TYPE_MASK => {
                if [ITEMGETINF_38, ITEMGETINF_39, ITEMGETINF_3A, ITEMGETINF_3B].iter().all(|&f| s.get_item_get_inf(f)) {
                    0x70AE
                } else {
                    match play.msg_ctx.choice_index {
                        1 => 0x70A4,
                        0 => 0x70A3,
                        _ => return,
                    }
                }
            }
            // EnOssan_TalkDefaultShopkeeper.
            _ => 0x9E,
        };
        play.continue_textbox(text);
    }

    /// `EnOssan_UpdateCameraDirection`: the browsing camera's turn (`Camera_SetCameraData(cam,
    /// 0xC, NULL, NULL, cameraFaceAngle, 0, 0)`: `data2` the angle as an s16, `data3` 0).
    fn update_camera_direction(&mut self, play: &mut PlayState, camera_face_angle: f32) {
        self.camera_face_angle = camera_face_angle;
        play.game_camera.set_camera_data(0xC, camera_face_angle as i32 as i16, 0);
    }

    /// `EnOssan_UpdateCursorPos`: the cursor on the selected item's screen position.
    fn update_cursor_pos(&mut self, play: &PlayState) {
        if let Some(a) = self.slot_actor(play, self.cursor_index) {
            let (x, y) = oot_game::target::actor_screen_pos(play.view_proj, a);
            self.cursor_x = x as f32;
            self.cursor_y = y as f32;
        }
    }

    fn slot(&self, i: u8) -> Option<ActorHandle> {
        self.shelf_slots.get(i as usize).copied().flatten()
    }

    fn slot_actor<'a>(&self, play: &'a PlayState, i: u8) -> Option<&'a Actor> {
        self.slot(i).and_then(|h| play.actors.actor(h))
    }

    fn slot_item<'a>(&self, play: &'a PlayState, i: u8) -> Option<&'a EnGirlA> {
        self.slot(i).and_then(|h| play.actors.downcast::<EnGirlA>(h))
    }

    /// The selected item's description (`shelfSlots[cursorIndex]->actor.textId`).
    fn selected_text(&self, play: &PlayState) -> u16 {
        self.slot_actor(play, self.cursor_index).map(|a| a.text_id).unwrap_or(0)
    }

    fn set_player_hidden(play: &mut PlayState, hidden: bool) {
        /// `PLAYER_STATE2_29`.
        const STATE2_29: u32 = 1 << 29;
        if let Some(p) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
            if hidden { p.change_state_flags2(STATE2_29, 0) } else { p.change_state_flags2(0, STATE2_29) }
        }
    }

    /// `EnOssan_EndInteraction`: the box closes, Link is back, the camera on its fixed view.
    fn end_interaction(&mut self, play: &mut PlayState) {
        // "End of conversation!"
        play.msg_ctx.yreg_31 = 0;
        oot_game::npc::process_talk_request(&mut self.actor);
        play.msg_ctx.msg_mode = MSGMODE_TEXT_CLOSING;
        play.msg_ctx.state_timer = 4;
        Self::set_player_hidden(play, false);
        play.set_viewpoint(VIEWPOINT_LOCKED);
        change_alpha(&mut play.save, 50);
        self.draw_cursor = 0;
        self.stick_left_prompt.is_enabled = false;
        self.stick_right_prompt.is_enabled = false;
        self.update_camera_direction(play, 0.0);
        self.actor.text_id = self.setup_hello_dialog(play);
        self.state_flag = OSSAN_STATE_IDLE;
    }

    /// `EnOssan_TestEndInteraction`: B ends it.
    fn test_end_interaction(&mut self, play: &mut PlayState) -> bool {
        if play.input.press.held(eng_input::pad::BTN_B) {
            self.end_interaction(play);
            return true;
        }
        false
    }

    /// `EnOssan_TestCancelOption`: B goes back to the item's description.
    fn test_cancel_option(&mut self, play: &mut PlayState) -> bool {
        if play.input.press.held(eng_input::pad::BTN_B) {
            self.state_flag = self.temp_state_flag;
            let t = self.selected_text(play);
            play.continue_textbox(t);
            return true;
        }
        false
    }

    /// `EnOssan_SetStateStartShopping`.
    fn set_state_start_shopping(&mut self, play: &mut PlayState, skip_hello_state: bool) {
        play.msg_ctx.yreg_31 = 1;
        self.head_rot = 0;
        self.head_target_rot = 0;
        play.interface_ctx.set_do_action(DO_ACTION_NEXT);
        self.update_camera_direction(play, 0.0);
        if !skip_hello_state {
            self.state_flag = OSSAN_STATE_START_CONVERSATION;
        } else {
            self.start_shopping(play);
        }
    }

    /// `EnOssan_StartShopping`: facing the shopkeeper, 0x83's choice with both prompts.
    fn start_shopping(&mut self, play: &mut PlayState) {
        self.state_flag = OSSAN_STATE_FACING_SHOPKEEPER;
        if self.actor.params == OSSAN_TYPE_MASK {
            // All masks sold: the Mask of Truth can be asked about.
            let s = &play.save;
            if [ITEMGETINF_38, ITEMGETINF_39, ITEMGETINF_3A, ITEMGETINF_3B].iter().all(|&f| s.get_item_get_inf(f)) {
                play.continue_textbox(0x70AD);
            } else {
                play.continue_textbox(0x70A2);
            }
        } else {
            play.continue_textbox(0x83);
        }
        play.interface_ctx.set_do_action(DO_ACTION_DECIDE);
        self.stick_right_prompt.is_enabled = true;
        self.stick_left_prompt.is_enabled = true;
        self.update_camera_direction(play, 0.0);
    }

    /// `EnOssan_ChooseTalkToOwner`.
    fn choose_talk_to_owner(&mut self, play: &mut PlayState) {
        self.state_flag = OSSAN_STATE_TALKING_TO_SHOPKEEPER;
        self.talk_owner(play);
        play.interface_ctx.set_do_action(DO_ACTION_DECIDE);
        self.stick_left_prompt.is_enabled = false;
        self.stick_right_prompt.is_enabled = false;
    }

    /// `EnOssan_SetLookToShopkeeperFromShelf`.
    fn set_look_to_shopkeeper_from_shelf(&mut self) {
        // NA_SE_SY_CURSOR.
        self.draw_cursor = 0;
        self.state_flag = OSSAN_STATE_LOOK_SHOPKEEPER;
    }

    /// `EnOssan_State_Idle`: facing Link, offering to talk within 100.
    fn state_idle(&mut self, play: &mut PlayState) {
        self.head_target_rot = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
        if oot_game::npc::process_talk_request(&mut self.actor) {
            // "Start conversation!!"
            Self::set_player_hidden(play, true);
            // Play_SetShopBrowsingViewpoint.
            if play.scene_cam_type == oot_game::scene::SCENE_CAM_TYPE_FIXED_SHOP_VIEWPOINT {
                play.viewpoint = VIEWPOINT_PIVOT;
            }
            self.set_state_start_shopping(play, false);
        } else if self.actor.xz_dist_to_player < 100.0 {
            let a = self.actor.clone();
            oot_game::npc::offer_talk(play, &a, 100.0);
        }
    }

    /// `EnOssan_UpdateJoystickInputState`: the stick's tilt past 30 accumulated per direction
    /// (up to 2000), a move on a fresh tilt or a reversal.
    fn update_joystick_input_state(&mut self, play: &PlayState) {
        let (stick_x, stick_y) = (play.input.rel.stick_x as i32, play.input.rel.stick_y as i32);
        self.move_horizontal = false;
        self.move_vertical = false;
        let axis = |accum: &mut i32, stick: i32, moved: &mut bool| {
            if *accum == 0 {
                if !(-30..=30).contains(&stick) {
                    *accum = stick;
                    *moved = true;
                }
            } else if (-30..=30).contains(&stick) {
                *accum = 0;
            } else if *accum * stick < 0 {
                *accum = stick;
                *moved = true;
            } else {
                *accum = (*accum + stick).clamp(-2000, 2000);
            }
        };
        axis(&mut self.stick_accum_x, stick_x, &mut self.move_horizontal);
        axis(&mut self.stick_accum_y, stick_y, &mut self.move_vertical);
    }

    /// `EnOssan_SetCursorIndexFromNeutral`: the first item of a shelf (`shelf_offset` 0 the
    /// right, 4 the left), on the cursor's row first.
    fn set_cursor_index_from_neutral(&self, shelf_offset: u8) -> u8 {
        let top = (shelf_offset + 1..shelf_offset + 4).step_by(2);
        let bottom = (shelf_offset..shelf_offset + 4).step_by(2);
        let order: Vec<u8> = if self.cursor_index & 1 != 0 { top.chain(bottom).collect() } else { bottom.chain(top).collect() };
        order.into_iter().find(|&i| self.slot(i).is_some()).unwrap_or(CURSOR_INVALID)
    }

    /// `EnOssan_CursorRight`: towards the shelf's lower slots, while inside it.
    fn cursor_right(&self, mut cursor_index: u8, shelf_slot_min: u8) -> u8 {
        let c = shelf_slot_min + 4;
        while cursor_index >= shelf_slot_min && cursor_index < c {
            cursor_index = cursor_index.wrapping_sub(2);
            if cursor_index >= shelf_slot_min && cursor_index < c && self.slot(cursor_index).is_some() {
                return cursor_index;
            }
        }
        CURSOR_INVALID
    }

    /// `EnOssan_CursorLeft`: towards the higher slots, below `shelf_slot_max`.
    fn cursor_left(&self, mut cursor_index: u8, shelf_slot_max: u8) -> u8 {
        while cursor_index < shelf_slot_max {
            cursor_index = cursor_index.wrapping_add(2);
            if cursor_index < shelf_slot_max && self.slot(cursor_index).is_some() {
                return cursor_index;
            }
        }
        CURSOR_INVALID
    }

    /// `EnOssan_TryPaybackMask`.
    fn try_payback_mask(&mut self, play: &mut PlayState) {
        let price = MASK_PAYMENT_PRICE[(self.happy_mask_shop_state as usize).min(3)];
        if play.save.rupees < price {
            play.continue_textbox(0x70A8);
            self.happy_mask_shopkeeper_eye_idx = 1;
            self.happy_mask_shop_state = OSSAN_HAPPY_STATE_ANGRY;
        } else {
            oot_game::item::rupees_change_by(&mut play.save, -price);
            if self.happy_mask_shop_state == OSSAN_HAPPY_STATE_REQUEST_PAYMENT_BUNNY_HOOD {
                play.save.set_event_chk_inf(EVENTCHKINF_8F);
                play.continue_textbox(0x70A9);
                self.happy_mask_shop_state = OSSAN_HAPPY_STATE_ALL_MASKS_SOLD;
                return;
            }
            match self.happy_mask_shop_state {
                OSSAN_HAPPY_STATE_REQUEST_PAYMENT_KEATON_MASK => play.save.set_event_chk_inf(EVENTCHKINF_8C),
                OSSAN_HAPPY_STATE_REQUEST_PAYMENT_SPOOKY_MASK => play.save.set_event_chk_inf(EVENTCHKINF_8E),
                OSSAN_HAPPY_STATE_REQUEST_PAYMENT_SKULL_MASK => play.save.set_event_chk_inf(EVENTCHKINF_8D),
                _ => {}
            }
            play.continue_textbox(0x70A7);
            self.happy_mask_shop_state = OSSAN_HAPPY_STATE_NONE;
        }
        self.state_flag = OSSAN_STATE_START_CONVERSATION;
    }

    /// `EnOssan_State_StartConversation`: after the hello's A, shopping starts.
    fn state_start_conversation(&mut self, play: &mut PlayState) {
        let dialog_state = play.message_state();
        if self.actor.params == OSSAN_TYPE_MASK && dialog_state == TEXT_STATE_CHOICE {
            if !self.test_end_interaction(play) && should_advance(&play.input) {
                match play.msg_ctx.choice_index {
                    0 => self.start_shopping(play),
                    1 => self.end_interaction(play),
                    _ => {}
                }
            }
        } else if dialog_state == TEXT_STATE_EVENT && should_advance(&play.input) {
            // NA_SE_SY_MESSAGE_PASS.
            match self.happy_mask_shop_state {
                OSSAN_HAPPY_STATE_ALL_MASKS_SOLD => {
                    play.continue_textbox(0x70AA);
                    self.state_flag = OSSAN_STATE_LEND_MASK_OF_TRUTH;
                    return;
                }
                OSSAN_HAPPY_STATE_BORROWED_FIRST_MASK => {
                    self.end_interaction(play);
                    return;
                }
                OSSAN_HAPPY_STATE_REQUEST_PAYMENT_KEATON_MASK..=OSSAN_HAPPY_STATE_REQUEST_PAYMENT_BUNNY_HOOD => {
                    self.try_payback_mask(play);
                    return;
                }
                OSSAN_HAPPY_STATE_ANGRY => {
                    play.transition.next_entrance_index = ENTR_MARKET_DAY_9;
                    play.transition.trigger = oot_game::transition::TRANS_TRIGGER_START;
                    play.transition.ty = TRANS_TYPE_CIRCLE_STARBURST_WHITE_FAST;
                    return;
                }
                _ => {}
            }
            if !self.test_end_interaction(play) {
                // "Shop around by moving the stick left and right".
                self.start_shopping(play);
            }
        }
    }

    /// `EnOssan_FacingShopkeeperDialogResult`.
    fn facing_shopkeeper_dialog_result(&mut self, play: &mut PlayState) -> bool {
        match play.msg_ctx.choice_index {
            0 => {
                self.choose_talk_to_owner(play);
                true
            }
            1 => {
                self.end_interaction(play);
                true
            }
            _ => false,
        }
    }

    /// `EnOssan_State_FacingShopkeeper`: 0x83's choice, or the stick to a shelf.
    fn state_facing_shopkeeper(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_CHOICE && !self.test_end_interaction(play) {
            if should_advance(&play.input) && self.facing_shopkeeper_dialog_result(play) {
                // NA_SE_SY_DECIDE.
                return;
            }
            if self.stick_accum_x < 0 {
                let next = self.set_cursor_index_from_neutral(4);
                if next != CURSOR_INVALID {
                    self.cursor_index = next;
                    self.state_flag = OSSAN_STATE_LOOK_SHELF_LEFT;
                    play.interface_ctx.set_do_action(DO_ACTION_DECIDE);
                    self.stick_left_prompt.is_enabled = false;
                }
            } else if self.stick_accum_x > 0 {
                let next = self.set_cursor_index_from_neutral(0);
                if next != CURSOR_INVALID {
                    self.cursor_index = next;
                    self.state_flag = OSSAN_STATE_LOOK_SHELF_RIGHT;
                    play.interface_ctx.set_do_action(DO_ACTION_DECIDE);
                    self.stick_right_prompt.is_enabled = false;
                }
            }
        }
    }

    /// `EnOssan_State_TalkingToShopkeeper`.
    fn state_talking_to_shopkeeper(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_EVENT && should_advance(&play.input) {
            self.start_shopping(play);
        }
    }

    /// `EnOssan_State_LookToLeftShelf` (`target` 30) and `_RightShelf` (-30): the camera turns
    /// (`Math_ApproachF` by half, at most 10 a frame), then the shelf's first description.
    fn state_look_to_shelf(&mut self, play: &mut PlayState, target: f32, browse: u8) {
        approach_f(&mut self.camera_face_angle, target, 0.5, 10.0);
        let reached = |a: f32| if target > 0.0 { a > 29.5 } else { a < -29.5 };
        if reached(self.camera_face_angle) {
            self.update_camera_direction(play, target);
        }
        let a = self.camera_face_angle;
        self.update_camera_direction(play, a);
        let done = if target > 0.0 { self.camera_face_angle >= target } else { self.camera_face_angle <= target };
        if done {
            self.update_camera_direction(play, target);
            self.update_cursor_pos(play);
            self.state_flag = browse;
            let t = self.selected_text(play);
            play.continue_textbox(t);
        } else {
            self.stick_accum_x = 0;
        }
    }

    /// `EnOssan_CursorUpDown`: the stick up or down to the other row, or along the shelf to the
    /// next item on that row.
    fn cursor_up_down(&mut self) {
        let mut cur_temp = self.cursor_index;
        if self.stick_accum_y == 0 {
            return;
        }
        // Down: the bottom row (even slots), scanning from 0 or 4; up: the top row, from 1 or 5.
        let (right_wrap, left_wrap) = if self.stick_accum_y < 0 {
            cur_temp &= 0xFE;
            (0u8, 4u8)
        } else {
            cur_temp |= 1;
            (1u8, 5u8)
        };
        if self.slot(cur_temp).is_some() {
            self.cursor_index = cur_temp;
            return;
        }
        let (limit, wrap) = if cur_temp < 4 { (4u8, right_wrap) } else { (8u8, left_wrap) };
        let mut scan = cur_temp + 2;
        if scan >= limit {
            scan = wrap;
        }
        while scan != cur_temp {
            if self.slot(scan).is_some() {
                self.cursor_index = scan;
                return;
            }
            scan += 2;
            if scan >= limit {
                scan = wrap;
            }
        }
    }

    /// `EnOssan_HasPlayerSelectedItem`: B ends; A (or any advance) on an item in stock takes it
    /// with its buy prompt, in the state its kind needs.
    fn has_player_selected_item(&mut self, play: &mut PlayState) -> bool {
        if self.test_end_interaction(play) {
            return true;
        }
        if should_advance(&play.input) {
            let Some((params, invisible, prompt)) = self.slot_item(play, self.cursor_index).map(|g| (g.actor.params, g.is_invisible, g.item_buy_prompt_text_id)) else {
                return true;
            };
            if params != SI_SOLD_OUT && !invisible {
                self.temp_state_flag = self.state_flag;
                play.continue_textbox(prompt);
                self.stick_left_prompt.is_enabled = false;
                self.stick_right_prompt.is_enabled = false;
                // NA_SE_SY_DECIDE (NA_SE_SY_ERROR for SI_19 and SI_20).
                self.draw_cursor = 0;
                self.state_flag = match params {
                    SI_KEATON_MASK | SI_SPOOKY_MASK | SI_SKULL_MASK | SI_BUNNY_HOOD | SI_MASK_OF_TRUTH | SI_ZORA_MASK | SI_GORON_MASK | SI_GERUDO_MASK => OSSAN_STATE_SELECT_ITEM_MASK,
                    SI_MILK_BOTTLE => OSSAN_STATE_SELECT_ITEM_MILK_BOTTLE,
                    SI_WEIRD_EGG => OSSAN_STATE_SELECT_ITEM_WEIRD_EGG,
                    SI_19 | SI_20 => OSSAN_STATE_SELECT_ITEM_UNIMPLEMENTED,
                    SI_BOMBS_5_R25 | SI_BOMBS_10 | SI_BOMBS_20 | SI_BOMBS_30 | SI_BOMBS_5_R35 => OSSAN_STATE_SELECT_ITEM_BOMBS,
                    _ => OSSAN_STATE_SELECT_ITEM,
                };
                return true;
            }
            // NA_SE_SY_ERROR.
            return true;
        }
        false
    }

    /// `EnOssan_State_BrowseLeftShelf` (`left`) and `_RightShelf`: once the last item is back
    /// on the shelf and 3 frames have passed, the cursor on the item; the stick moves it along
    /// the shelf (a fresh tilt, or held past 500), off its outer end back to the shopkeeper,
    /// and up or down; A chooses.
    fn state_browse_shelf(&mut self, play: &mut PlayState, left: bool) {
        let prev_index = self.cursor_index;
        if !self.return_item_to_shelf(play) {
            // "Zooming!!"
            self.delay_timer = 3;
            return;
        }
        if self.delay_timer != 0 {
            self.delay_timer -= 1;
            return;
        }
        self.draw_cursor = 0xFF;
        if left {
            self.stick_right_prompt.is_enabled = true;
        } else {
            self.stick_left_prompt.is_enabled = true;
        }
        self.update_cursor_pos(play);
        if play.message_state() == TEXT_STATE_EVENT && !self.has_player_selected_item(play) {
            // Towards the shopkeeper: right on the left shelf, left on the right one.
            let (toward, away) = if left { (self.stick_accum_x > 0, self.stick_accum_x < 0) } else { (self.stick_accum_x < 0, self.stick_accum_x > 0) };
            let far = self.stick_accum_x.abs() > 500;
            let (min, max) = if left { (4, 8) } else { (0, 4) };
            if toward && (self.move_horizontal || far) {
                let a = self.cursor_right(self.cursor_index, min);
                if a != CURSOR_INVALID {
                    self.cursor_index = a;
                } else {
                    self.set_look_to_shopkeeper_from_shelf();
                    return;
                }
            } else if away && (self.move_horizontal || far) {
                let b = self.cursor_left(self.cursor_index, max);
                if b != CURSOR_INVALID {
                    self.cursor_index = b;
                }
            }
            self.cursor_up_down();
            if self.cursor_index != prev_index {
                let t = self.selected_text(play);
                play.continue_textbox(t);
                // NA_SE_SY_CURSOR.
            }
        }
    }

    /// `EnOssan_State_LookFromShelfToShopkeeper`: the camera back to 0, then 0x83 again.
    fn state_look_from_shelf_to_shopkeeper(&mut self, play: &mut PlayState) {
        approach_f(&mut self.camera_face_angle, 0.0, 0.5, 10.0);
        if self.camera_face_angle < 0.5 && self.camera_face_angle > -0.5 {
            self.update_camera_direction(play, 0.0);
        }
        let a = self.camera_face_angle;
        self.update_camera_direction(play, a);
        if self.camera_face_angle == 0.0 {
            self.start_shopping(play);
        }
    }

    /// `EnOssan_State_DisplayOnlyBombDialog`.
    fn state_display_only_bomb_dialog(&mut self, play: &mut PlayState) {
        if !self.return_item_to_shelf(play) {
            return;
        }
        approach_f(&mut self.camera_face_angle, 0.0, 0.5, 10.0);
        if self.camera_face_angle < 0.5 && self.camera_face_angle > -0.5 {
            self.update_camera_direction(play, 0.0);
        }
        let a = self.camera_face_angle;
        self.update_camera_direction(play, a);
        if self.camera_face_angle == 0.0 {
            play.continue_textbox(0x3010);
            self.state_flag = OSSAN_STATE_WAIT_FOR_DISPLAY_ONLY_BOMB_DIALOG;
        }
    }

    /// `EnOssan_GiveItemWithFanfare`: the item offered through the get-item flow (within 120),
    /// the box closed so Player's talk ends and he takes it, Link shown, the fixed view.
    fn give_item_with_fanfare(&mut self, play: &mut PlayState) {
        // "Obtained for the first time!!"
        let gi = self.slot_item(play, self.cursor_index).map(|g| g.get_item_id).unwrap_or(0);
        let a = self.actor.clone();
        oot_game::get_item::offer_get_item_range(play, &a, gi, 120.0, 120.0);
        play.msg_ctx.msg_mode = MSGMODE_TEXT_CLOSING;
        play.msg_ctx.state_timer = 4;
        Self::set_player_hidden(play, false);
        play.set_viewpoint(VIEWPOINT_LOCKED);
        change_alpha(&mut play.save, 50);
        self.draw_cursor = 0;
        self.update_camera_direction(play, 0.0);
        self.state_flag = OSSAN_STATE_GIVE_ITEM_FANFARE;
        // "Start lifting!!"
    }

    /// `EnOssan_SetStateCantGetItem`.
    fn set_state_cant_get_item(&mut self, play: &mut PlayState, text_id: u16) {
        play.continue_textbox(text_id);
        self.state_flag = OSSAN_STATE_CANT_GET_ITEM;
    }

    /// `EnOssan_SetStateQuickBuyDialog`.
    fn set_state_quick_buy_dialog(&mut self, play: &mut PlayState, text_id: u16) {
        play.continue_textbox(text_id);
        self.state_flag = OSSAN_STATE_QUICK_BUY;
    }

    fn selected_can_buy(&self, play: &PlayState) -> Option<(i32, i16)> {
        self.slot_item(play, self.cursor_index).map(|g| (g.can_buy(&play.save), g.actor.params))
    }

    fn selected_give(&self, play: &mut PlayState) {
        with_item(play, self.slot(self.cursor_index), |g, play| {
            if let Some(f) = g.item_give_func {
                f(play, g);
            }
        });
    }

    fn selected_out_of_stock(&self, play: &mut PlayState) {
        with_item(play, self.slot(self.cursor_index), |g, _| g.set_item_out_of_stock());
    }

    fn selected_restock(&self, play: &mut PlayState) {
        with_item(play, self.slot(self.cursor_index), |g, play| g.update_stocked_item(play));
    }

    /// `EnOssan_HandleCanBuyItem`: what "Buy" does, by the item's answer.
    fn handle_can_buy_item(&mut self, play: &mut PlayState) {
        let Some((result, params)) = self.selected_can_buy(play) else { return };
        match result {
            CANBUY_RESULT_SUCCESS_FANFARE => {
                if params == SI_HYLIAN_SHIELD && play.save.get_inf_table(INFTABLE_76) {
                    self.set_state_give_discount_dialog(play);
                } else {
                    self.give_item_with_fanfare(play);
                    self.draw_cursor = 0;
                    self.shop_item_selected_tween = 0.0;
                    self.selected_out_of_stock(play);
                }
            }
            CANBUY_RESULT_SUCCESS => {
                self.selected_give(play);
                self.set_state_quick_buy_dialog(play, 0x84);
                self.draw_cursor = 0;
                self.shop_item_selected_tween = 0.0;
                self.selected_out_of_stock(play);
            }
            CANBUY_RESULT_CANT_GET_NOW | CANBUY_RESULT_CANT_GET_NOW_5 => self.set_state_cant_get_item(play, 0x86),
            CANBUY_RESULT_NEED_BOTTLE => self.set_state_cant_get_item(play, 0x96),
            CANBUY_RESULT_NEED_RUPEES => self.set_state_cant_get_item(play, 0x85),
            _ => {}
        }
    }

    /// `EnOssan_HandleCanBuyLonLonMilk`.
    fn handle_can_buy_lon_lon_milk(&mut self, play: &mut PlayState) {
        let Some((result, _)) = self.selected_can_buy(play) else { return };
        match result {
            CANBUY_RESULT_SUCCESS_FANFARE => {
                play.continue_textbox(0x9C);
                self.state_flag = OSSAN_STATE_GIVE_LON_LON_MILK;
                self.draw_cursor = 0;
            }
            CANBUY_RESULT_SUCCESS => {
                self.selected_give(play);
                self.set_state_quick_buy_dialog(play, 0x98);
                self.draw_cursor = 0;
                self.shop_item_selected_tween = 0.0;
                self.selected_out_of_stock(play);
            }
            CANBUY_RESULT_NEED_BOTTLE => self.set_state_cant_get_item(play, 0x96),
            CANBUY_RESULT_NEED_RUPEES => self.set_state_cant_get_item(play, 0x85),
            _ => {}
        }
    }

    /// `EnOssan_HandleCanBuyWeirdEgg`.
    fn handle_can_buy_weird_egg(&mut self, play: &mut PlayState) {
        let Some((result, _)) = self.selected_can_buy(play) else { return };
        match result {
            CANBUY_RESULT_SUCCESS_FANFARE => {
                self.give_item_with_fanfare(play);
                self.draw_cursor = 0;
                self.shop_item_selected_tween = 0.0;
                self.selected_out_of_stock(play);
            }
            CANBUY_RESULT_SUCCESS => {
                self.selected_give(play);
                self.set_state_quick_buy_dialog(play, 0x9A);
                self.draw_cursor = 0;
                self.shop_item_selected_tween = 0.0;
                self.selected_out_of_stock(play);
            }
            CANBUY_RESULT_CANT_GET_NOW => self.set_state_cant_get_item(play, 0x9D),
            CANBUY_RESULT_NEED_RUPEES => self.set_state_cant_get_item(play, 0x85),
            _ => {}
        }
    }

    /// `EnOssan_HandleCanBuyBombs`.
    fn handle_can_buy_bombs(&mut self, play: &mut PlayState) {
        let Some((result, _)) = self.selected_can_buy(play) else { return };
        match result {
            CANBUY_RESULT_SUCCESS_FANFARE | CANBUY_RESULT_SUCCESS => {
                self.selected_give(play);
                self.set_state_quick_buy_dialog(play, 0x84);
                self.draw_cursor = 0;
                self.shop_item_selected_tween = 0.0;
                self.selected_out_of_stock(play);
            }
            CANBUY_RESULT_CANT_GET_NOW => self.set_state_cant_get_item(play, 0x86),
            CANBUY_RESULT_NEED_RUPEES => self.set_state_cant_get_item(play, 0x85),
            _ => {}
        }
    }

    /// `EnOssan_BuyGoronCityBombs`.
    fn buy_goron_city_bombs(&mut self, play: &mut PlayState) {
        if !play.save.adult && !play.save.get_event_chk_inf(EVENTCHKINF_25) {
            if play.save.get_inf_table(INFTABLE_FC) {
                self.set_state_cant_get_item(play, 0x302E);
            } else {
                self.stick_left_prompt.is_enabled = false;
                self.stick_right_prompt.is_enabled = false;
                self.draw_cursor = 0;
                self.state_flag = OSSAN_STATE_DISPLAY_ONLY_BOMB_DIALOG;
            }
        } else {
            self.handle_can_buy_bombs(play);
        }
    }

    /// The buy prompt's two answers: "Buy" does `buy`, "Don't buy" (or B) goes back to the
    /// description. The shape of `EnOssan_State_ItemSelected`, `_SelectMilkBottle`,
    /// `_SelectWeirdEgg`, and `_SelectBombs` for the Goron shop.
    fn buy_prompt(&mut self, play: &mut PlayState, buy: fn(&mut Self, &mut PlayState)) {
        if !self.take_item_off_shelf(play) {
            // "Zooming!!"
            return;
        }
        if play.message_state() == TEXT_STATE_CHOICE && !self.test_cancel_option(play) && should_advance(&play.input) {
            match play.msg_ctx.choice_index {
                0 => buy(self, play),
                1 => {
                    self.state_flag = self.temp_state_flag;
                    let t = self.selected_text(play);
                    play.continue_textbox(t);
                }
                _ => {}
            }
        }
    }

    /// `EnOssan_State_SelectUnimplementedItem`.
    fn state_select_unimplemented_item(&mut self, play: &mut PlayState) {
        if !self.take_item_off_shelf(play) {
            return;
        }
        if play.message_state() == TEXT_STATE_EVENT && should_advance(&play.input) {
            self.state_flag = self.temp_state_flag;
            let t = self.selected_text(play);
            play.continue_textbox(t);
        }
    }

    /// `EnOssan_State_SelectBombs`: the Goron shop's own answers, else `_ItemSelected`.
    fn state_select_bombs(&mut self, play: &mut PlayState) {
        if !self.take_item_off_shelf(play) {
            return;
        }
        if self.actor.params != OSSAN_TYPE_GORON {
            self.buy_prompt(play, Self::handle_can_buy_item);
            return;
        }
        self.buy_prompt(play, Self::buy_goron_city_bombs);
    }

    /// `EnOssan_State_SelectMaskItem`.
    fn state_select_mask_item(&mut self, play: &mut PlayState) {
        let talk_state = play.message_state();
        if !self.take_item_off_shelf(play) {
            return;
        }
        if talk_state == TEXT_STATE_EVENT {
            if should_advance(&play.input) {
                self.state_flag = self.temp_state_flag;
                let t = self.selected_text(play);
                play.continue_textbox(t);
            }
        } else if talk_state == TEXT_STATE_CHOICE && !self.test_cancel_option(play) && should_advance(&play.input) {
            match play.msg_ctx.choice_index {
                0 => {
                    let flag = match self.slot_item(play, self.cursor_index).map(|g| g.actor.params) {
                        Some(SI_KEATON_MASK) => Some(ITEMGETINF_23),
                        Some(SI_SPOOKY_MASK) => Some(ITEMGETINF_25),
                        Some(SI_SKULL_MASK) => Some(ITEMGETINF_24),
                        Some(SI_BUNNY_HOOD) => Some(ITEMGETINF_26),
                        _ => None,
                    };
                    if let Some(f) = flag {
                        play.save.set_item_get_inf(f);
                    }
                    self.give_item_with_fanfare(play);
                    self.draw_cursor = 0;
                    self.shop_item_selected_tween = 0.0;
                    self.selected_out_of_stock(play);
                }
                1 => {
                    self.state_flag = self.temp_state_flag;
                    let t = self.selected_text(play);
                    play.continue_textbox(t);
                }
                _ => {}
            }
        }
    }

    /// `EnOssan_State_CantGetItem`.
    fn state_cant_get_item(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_EVENT && should_advance(&play.input) {
            self.state_flag = self.temp_state_flag;
            let t = self.selected_text(play);
            play.continue_textbox(t);
        }
    }

    /// `EnOssan_State_QuickBuyDialog`: after "Thanks a lot!", the item back on the shelf.
    fn state_quick_buy_dialog(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_EVENT && should_advance(&play.input) {
            self.shop_item_selected_tween = 0.0;
            self.reset_item_position(play);
            self.selected_restock(play);
            self.state_flag = self.temp_state_flag;
            let t = self.selected_text(play);
            play.continue_textbox(t);
        }
    }

    /// `EnOssan_State_GiveItemWithFanfare`: once Player has taken it (`Actor_HasParent`), on to
    /// its text; until then the offer stands.
    fn state_give_item_with_fanfare(&mut self, play: &mut PlayState) {
        // Player sets itself as the parent when it takes the offer.
        if oot_game::get_item::actor_has_parent(&self.actor) {
            self.actor.parent = None;
            self.state_flag = OSSAN_STATE_ITEM_PURCHASED;
            return;
        }
        let gi = self.slot_item(play, self.cursor_index).map(|g| g.get_item_id).unwrap_or(0);
        let a = self.actor.clone();
        oot_game::get_item::offer_get_item_range(play, &a, gi, 120.0, 120.0);
    }

    /// `EnOssan_State_ItemPurchased`: at the item text's end, the price (`buyEventFunc`) and
    /// 0x6B, "anything else?".
    fn state_item_purchased(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_DONE && should_advance(&play.input) {
            if self.actor.params == OSSAN_TYPE_MASK {
                let temp_params = self.slot_item(play, self.cursor_index).map(|g| g.actor.params);
                self.reset_item_position(play);
                self.selected_restock(play);
                if temp_params == Some(SI_MASK_OF_TRUTH) && !play.save.get_item_get_inf(ITEMGETINF_3F) {
                    play.save.set_item_get_inf(ITEMGETINF_3F);
                    play.continue_textbox(0x70AB);
                    self.happy_mask_shop_state = OSSAN_HAPPY_STATE_BORROWED_FIRST_MASK;
                    self.update_shop_offerings(play);
                    self.state_flag = OSSAN_STATE_START_CONVERSATION;
                } else {
                    self.end_interaction(play);
                }
                return;
            }
            with_item(play, self.slot(self.cursor_index), |g, play| {
                if let Some(f) = g.buy_event_func {
                    f(play, g);
                }
            });
            self.state_flag = OSSAN_STATE_CONTINUE_SHOPPING_PROMPT;
            play.continue_textbox(0x6B);
        }
    }

    /// The way back to shopping from 0x6B's "Yes" (or its event end): Link turned back to the
    /// shopkeeper and hidden, the browsing camera, the hello text afresh, and a talk offer with
    /// exchange item -1 so Player's get-item action ends into the talk (`func_8084E6D4`).
    fn continue_shopping(&mut self, play: &mut PlayState) {
        // "Continuing!!"
        if let Some(a) = play.player.and_then(|h| play.actors.actor_mut(h)) {
            a.shape_rot.y = a.shape_rot.y.wrapping_add(i16::MIN);
        }
        Self::set_player_hidden(play, true);
        play.set_viewpoint(VIEWPOINT_PIVOT);
        let (text, me) = (self.actor.text_id, play.cur_actor);
        play.start_textbox(text, me);
        self.set_state_start_shopping(play, true);
        let a = self.actor.clone();
        // func_8002F298(&this->actor, play, 100.0f, -1).
        oot_game::npc::offer_talk_exchange(play, &a, 100.0, 0xFF);
    }

    /// `EnOssan_State_ContinueShoppingPrompt`.
    fn state_continue_shopping_prompt(&mut self, play: &mut PlayState) {
        let talk_state = play.message_state();
        if talk_state == TEXT_STATE_CHOICE {
            if should_advance(&play.input) {
                self.reset_item_position(play);
                self.selected_restock(play);
                if !self.test_end_interaction(play) {
                    match play.msg_ctx.choice_index {
                        0 => self.continue_shopping(play),
                        // "Quitting!!"
                        _ => self.end_interaction(play),
                    }
                }
            }
        } else if talk_state == TEXT_STATE_EVENT && should_advance(&play.input) {
            self.reset_item_position(play);
            self.selected_restock(play);
            self.continue_shopping(play);
        }
    }

    /// `EnOssan_State_WaitForDisplayOnlyBombDialog`.
    fn state_wait_for_display_only_bomb_dialog(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_EVENT && should_advance(&play.input) {
            play.save.set_inf_table(INFTABLE_FC);
            self.start_shopping(play);
        }
    }

    /// `EnOssan_State_21` (unreachable).
    fn state_21(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_DONE_HAS_NEXT && should_advance(&play.input) {
            self.state_flag = OSSAN_STATE_22;
            play.continue_textbox(0x3012);
            play.save.set_inf_table(INFTABLE_FC);
        }
    }

    /// `EnOssan_State_22` (unreachable).
    fn state_22(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_EVENT && should_advance(&play.input) {
            self.start_shopping(play);
        }
    }

    /// `EnOssan_State_GiveLonLonMilk`.
    fn state_give_lon_lon_milk(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_EVENT && should_advance(&play.input) {
            self.give_item_with_fanfare(play);
        }
    }

    /// `EnOssan_State_LendMaskOfTruth`.
    fn state_lend_mask_of_truth(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_EVENT && should_advance(&play.input) {
            play.save.set_item_get_inf(ITEMGETINF_2A);
            self.cursor_index = 2;
            self.give_item_with_fanfare(play);
        }
    }

    /// `EnOssan_SetStateGiveDiscountDialog`.
    fn set_state_give_discount_dialog(&mut self, play: &mut PlayState) {
        play.continue_textbox(0x71B2);
        self.state_flag = OSSAN_STATE_DISCOUNT_DIALOG;
    }

    /// `EnOssan_State_GiveDiscountDialog`.
    fn state_give_discount_dialog(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_DONE && should_advance(&play.input) {
            self.give_item_with_fanfare(play);
            self.draw_cursor = 0;
            self.shop_item_selected_tween = 0.0;
            self.selected_out_of_stock(play);
        }
    }

    /// `EnOssan_PositionSelectedItem`: the item between its slot and in front of Link, by the
    /// tween.
    fn position_selected_item(&self, play: &mut PlayState) {
        let i = self.cursor_index as usize;
        let Some(shop_item) = SHOPKEEPER_STORES[self.ty()].get(i).copied() else { return };
        let Some(shelves) = self.shelves.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos) else { return };
        let target = SELECTED_ITEM_POSITION[i >> 2];
        let t = self.shop_item_selected_tween;
        let off = [shop_item.x as f32, shop_item.y as f32, shop_item.z as f32];
        let tx = (target[0] - off[0]) * t + off[0];
        let ty = (target[1] - off[1]) * t + off[1];
        let tz = (target[2] - off[2]) * t + off[2];
        if let Some(a) = self.slot(self.cursor_index).and_then(|h| play.actors.actor_mut(h)) {
            a.world_pos = shelves + Vec3::new(tx, ty, tz);
        }
    }

    /// `EnOssan_ResetItemPosition`.
    fn reset_item_position(&mut self, play: &mut PlayState) {
        self.shop_item_selected_tween = 0.0;
        self.position_selected_item(play);
    }

    /// `EnOssan_TakeItemOffShelf`: true once it's all the way out.
    fn take_item_off_shelf(&mut self, play: &mut PlayState) -> bool {
        approach_f(&mut self.shop_item_selected_tween, 1.0, 1.0, 0.15);
        if self.shop_item_selected_tween >= 0.85 {
            self.shop_item_selected_tween = 1.0;
        }
        self.position_selected_item(play);
        self.shop_item_selected_tween == 1.0
    }

    /// `EnOssan_ReturnItemToShelf`: true once it's back.
    fn return_item_to_shelf(&mut self, play: &mut PlayState) -> bool {
        approach_f(&mut self.shop_item_selected_tween, 0.0, 1.0, 0.15);
        if self.shop_item_selected_tween <= 0.15 {
            self.shop_item_selected_tween = 0.0;
        }
        self.position_selected_item(play);
        self.shop_item_selected_tween == 0.0
    }

    /// `EnOssan_UpdateItemSelectedProperty`: the item under the cursor spins, while choosing or
    /// with the cursor drawn.
    fn update_item_selected_property(&self, play: &mut PlayState) {
        let choosing = matches!(
            self.state_flag,
            OSSAN_STATE_SELECT_ITEM
                | OSSAN_STATE_SELECT_ITEM_MILK_BOTTLE
                | OSSAN_STATE_SELECT_ITEM_WEIRD_EGG
                | OSSAN_STATE_SELECT_ITEM_UNIMPLEMENTED
                | OSSAN_STATE_SELECT_ITEM_BOMBS
                | OSSAN_STATE_SELECT_ITEM_MASK
                | OSSAN_STATE_CANT_GET_ITEM
        );
        for i in 0..8u8 {
            let selected = (choosing || self.draw_cursor != 0) && self.cursor_index == i;
            if let Some(g) = self.slot(i).and_then(|h| play.actors.downcast_mut::<EnGirlA>(h)) {
                g.is_selected = selected;
            }
        }
    }

    /// `EnOssan_UpdateCursorAnim`: the cursor's green pulses (`ColChanMix`).
    fn update_cursor_anim(&mut self) {
        let mut t = self.cursor_anim_tween;
        if self.cursor_anim_state == 0 {
            t += 0.05;
            if t >= 1.0 {
                t = 1.0;
                self.cursor_anim_state = 1;
            }
        } else {
            t -= 0.05;
            if t <= 0.0 {
                t = 0.0;
                self.cursor_anim_state = 0;
            }
        }
        // ColChanMix(c1, c2, m): (c1 - (s32)(c2 * m)) & 0xFF.
        let mix = |c1: i32, c2: f32| ((c1 - (c2 * t) as i32) & 0xFF) as u32;
        self.cursor_color = [mix(0, 0.0), mix(255, 80.0), mix(80, 0.0), mix(255, 0.0)];
        self.cursor_anim_tween = t;
    }

    /// `EnOssan_UpdateStickDirectionPromptAnim`: the arrows' yellow pulses, the sticks nudge
    /// outwards.
    fn update_stick_direction_prompt_anim(&mut self) {
        let mut arrow = self.arrow_anim_tween;
        let mut stick = self.stick_anim_tween;
        if self.arrow_anim_state == 0 {
            arrow += 0.05;
            if arrow > 1.0 {
                arrow = 1.0;
                self.arrow_anim_state = 1;
            }
        } else {
            arrow -= 0.05;
            if arrow < 0.0 {
                arrow = 0.0;
                self.arrow_anim_state = 0;
            }
        }
        self.arrow_anim_tween = arrow;
        if self.stick_anim_state == 0 {
            stick += 0.1;
            if stick > 1.0 {
                stick = 1.0;
                self.stick_anim_state = 1;
            }
        } else {
            stick = 0.0;
            self.stick_anim_state = 0;
        }
        self.stick_anim_tween = stick;
        let u8c = |v: i32| (v as u8) as u32;
        let new_var3 = 155.0 * arrow;
        let rg = u8c(255 - (155.0 * arrow) as i32);
        let b = u8c(-((-100.0 * arrow) as i32));
        let a = u8c(200 - (50.0 * arrow) as i32);
        self.stick_left_prompt.arrow_color = [rg, rg, b, a];
        self.stick_right_prompt.arrow_color = [u8c(255 - new_var3 as i32), u8c(255 - new_var3 as i32), b, a];
        self.stick_right_prompt.arrow_tex_x = 290.0;
        self.stick_left_prompt.arrow_tex_x = 33.0;
        self.stick_right_prompt.stick_tex_x = 274.0 + 8.0 * stick;
        self.stick_left_prompt.stick_tex_x = 49.0 - 8.0 * stick;
        self.stick_left_prompt.arrow_tex_y = 91.0;
        self.stick_right_prompt.arrow_tex_y = 91.0;
        self.stick_left_prompt.stick_tex_y = 95.0;
        self.stick_right_prompt.stick_tex_y = 95.0;
    }

    /// `EnOssan_WaitForBlink`, `EnOssan_Blink`.
    fn blink(&mut self, play: &mut PlayState) {
        let decr = self.blink_timer.wrapping_sub(1);
        match self.blink {
            Blink::Wait => {
                if decr != 0 {
                    self.blink_timer = decr;
                } else {
                    self.blink = Blink::Blink;
                }
            }
            Blink::Blink => {
                if decr != 0 {
                    self.blink_timer = decr;
                    return;
                }
                let next = self.eye_texture_idx + 1;
                if next > 2 {
                    self.eye_texture_idx = 0;
                    self.blink_timer = (play.rand.zero_one() * 60.0) as i32 as i16 + 20;
                    self.blink = Blink::Wait;
                } else {
                    self.eye_texture_idx = next;
                    self.blink_timer = 1;
                }
            }
        }
    }

    /// `EnOssan_SetupHelloDialog`: 0x9E, or the Happy Mask Shop's text for the masks' state.
    fn setup_hello_dialog(&mut self, play: &PlayState) -> u16 {
        self.happy_mask_shop_state = OSSAN_HAPPY_STATE_NONE;
        if self.actor.params == OSSAN_TYPE_MASK {
            let s = &play.save;
            let g = |f: u16| s.get_item_get_inf(f);
            let e = |f: u16| s.get_event_chk_inf(f);
            if s.inv_content(oot_game::item::ITEM_MASK_KEATON) == oot_game::item::ITEM_SOLD_OUT {
                for (sold, paid, state, text) in [
                    (ITEMGETINF_3B, EVENTCHKINF_8F, OSSAN_HAPPY_STATE_REQUEST_PAYMENT_BUNNY_HOOD, 0x70C6),
                    (ITEMGETINF_3A, EVENTCHKINF_8E, OSSAN_HAPPY_STATE_REQUEST_PAYMENT_SPOOKY_MASK, 0x70C5),
                    (ITEMGETINF_39, EVENTCHKINF_8D, OSSAN_HAPPY_STATE_REQUEST_PAYMENT_SKULL_MASK, 0x70C4),
                    (ITEMGETINF_38, EVENTCHKINF_8C, OSSAN_HAPPY_STATE_REQUEST_PAYMENT_KEATON_MASK, 0x70A5),
                ] {
                    if g(sold) {
                        if !e(paid) {
                            self.happy_mask_shop_state = state;
                            return text;
                        }
                        return 0x70AC;
                    }
                }
            } else if g(ITEMGETINF_3B) {
                return 0x70AC;
            } else if !g(ITEMGETINF_3A) && !g(ITEMGETINF_24) && !g(ITEMGETINF_38) {
                if !g(ITEMGETINF_23) {
                    return 0x70A1;
                }
                self.happy_mask_shop_state = OSSAN_HAPPY_STATE_BORROWED_FIRST_MASK;
                return 0x70A6;
            } else {
                return 0x70C7;
            }
        }
        0x9E
    }

    /// `EnOssan_MainActionFunc`.
    fn main_action_func(&mut self, play: &mut PlayState) {
        self.blink(play);
        self.update_joystick_input_state(play);
        self.update_item_selected_property(play);
        self.update_stick_direction_prompt_anim();
        self.update_cursor_anim();
        step_to_s(&mut self.head_rot, self.head_target_rot, 0x190);
        if play.player.is_some() {
            match self.state_flag {
                OSSAN_STATE_IDLE => self.state_idle(play),
                OSSAN_STATE_START_CONVERSATION => self.state_start_conversation(play),
                OSSAN_STATE_FACING_SHOPKEEPER => self.state_facing_shopkeeper(play),
                OSSAN_STATE_TALKING_TO_SHOPKEEPER => self.state_talking_to_shopkeeper(play),
                OSSAN_STATE_LOOK_SHELF_LEFT => self.state_look_to_shelf(play, 30.0, OSSAN_STATE_BROWSE_LEFT_SHELF),
                OSSAN_STATE_LOOK_SHELF_RIGHT => self.state_look_to_shelf(play, -30.0, OSSAN_STATE_BROWSE_RIGHT_SHELF),
                OSSAN_STATE_BROWSE_LEFT_SHELF => self.state_browse_shelf(play, true),
                OSSAN_STATE_BROWSE_RIGHT_SHELF => self.state_browse_shelf(play, false),
                OSSAN_STATE_LOOK_SHOPKEEPER => self.state_look_from_shelf_to_shopkeeper(play),
                OSSAN_STATE_SELECT_ITEM => self.buy_prompt(play, Self::handle_can_buy_item),
                OSSAN_STATE_SELECT_ITEM_MILK_BOTTLE => self.buy_prompt(play, Self::handle_can_buy_lon_lon_milk),
                OSSAN_STATE_SELECT_ITEM_WEIRD_EGG => self.buy_prompt(play, Self::handle_can_buy_weird_egg),
                OSSAN_STATE_SELECT_ITEM_UNIMPLEMENTED => self.state_select_unimplemented_item(play),
                OSSAN_STATE_SELECT_ITEM_BOMBS => self.state_select_bombs(play),
                OSSAN_STATE_CANT_GET_ITEM => self.state_cant_get_item(play),
                OSSAN_STATE_GIVE_ITEM_FANFARE => self.state_give_item_with_fanfare(play),
                OSSAN_STATE_ITEM_PURCHASED => self.state_item_purchased(play),
                OSSAN_STATE_CONTINUE_SHOPPING_PROMPT => self.state_continue_shopping_prompt(play),
                OSSAN_STATE_GIVE_LON_LON_MILK => self.state_give_lon_lon_milk(play),
                OSSAN_STATE_DISPLAY_ONLY_BOMB_DIALOG => self.state_display_only_bomb_dialog(play),
                OSSAN_STATE_WAIT_FOR_DISPLAY_ONLY_BOMB_DIALOG => self.state_wait_for_display_only_bomb_dialog(play),
                OSSAN_STATE_21 => self.state_21(play),
                OSSAN_STATE_22 => self.state_22(play),
                OSSAN_STATE_QUICK_BUY => self.state_quick_buy_dialog(play),
                OSSAN_STATE_SELECT_ITEM_MASK => self.state_select_mask_item(play),
                OSSAN_STATE_LEND_MASK_OF_TRUTH => self.state_lend_mask_of_truth(play),
                OSSAN_STATE_DISCOUNT_DIALOG => self.state_give_discount_dialog(play),
                _ => {}
            }
        }
        self.actor.move_forward();
        self.actor.update_bg_check_info(&play.col, 26.0, 10.0, 0.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
        self.actor.set_focus(90.0);
        self.actor.scale = Vec3::splat(SHOPKEEPER_SCALE[self.ty()]);
        // obj3ToSeg6Func: the animation's object on segment 6 (the pack's animations need none).
        if let Some(s) = &mut self.skel {
            s.update();
        }
    }
}

/// Indices into the render state's extras.
mod rs {
    /// `values`: the cursor (x, y, z, then its RGBA), then each prompt (left, right): the
    /// stick's RGBA, x, y, the arrow's RGBA, x, y, and z.
    pub const CURSOR: usize = 0;
    pub const PROMPTS: usize = 7;
    pub const PROMPT_LEN: usize = 13;
    /// `switches`: the eye, `drawCursor`, the prompts enabled (left, right).
    pub const EYE: usize = 0;
    pub const DRAW_CURSOR: usize = 1;
    pub const LEFT_ENABLED: usize = 2;
    pub const RIGHT_ENABLED: usize = 3;
}

impl ActorImpl for EnOssan {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnOssan_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.timer = self.timer.wrapping_add(1);
        match self.action {
            Action::Init => self.init_action_func(play),
            Action::Main => self.main_action_func(play),
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        let Some(s) = &self.skel else { return rs };
        rs.joints = Some(eng_anim::anim::JointTable { rot: s.joint_table.clone(), face: 0 });
        let c = self.cursor_color;
        let mut values = vec![self.cursor_x, self.cursor_y, self.cursor_z, c[0] as f32, c[1] as f32, c[2] as f32, c[3] as f32];
        for p in [&self.stick_left_prompt, &self.stick_right_prompt] {
            values.extend(p.stick_color.iter().map(|&v| v as f32));
            values.extend([p.stick_tex_x, p.stick_tex_y]);
            values.extend(p.arrow_color.iter().map(|&v| v as f32));
            values.extend([p.arrow_tex_x, p.arrow_tex_y, p.z]);
        }
        rs.values = values;
        rs.switches = vec![self.eye_texture_idx.clamp(0, 2) as u32, self.draw_cursor as u32, self.stick_left_prompt.is_enabled as u32, self.stick_right_prompt.is_enabled as u32];
        rs
    }
    /// `EnOssan_DrawKokiriShopkeeper`: the shopkeeper, then the cursor and the stick prompts
    /// (`EnOssan_DrawCursor`, `EnOssan_DrawStickDirectionPrompts`) into the overlay.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(skeleton), Some(joints)) = (&self.skeleton, &rs.joints) else { return };
        if rs.switches.len() < 4 || rs.values.len() < rs::PROMPTS + 2 * rs::PROMPT_LEN {
            return;
        }
        let bones = skeleton.pose(&eng_anim::anim::JointTable { rot: joints.rot.clone(), face: 0 });
        let m = oot_game::play::actor_draw_matrix(rs);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&kokiri_bake(rs.switches[rs::EYE] as usize))), transform: m, bones, params: Default::default() });
        // ActorShadow_DrawCircle (shadowScale 20): the circle shadow's stand-in.
        let (floor, _) = play.col.entity_raycast_down(rs.pos + Vec3::Y * 20.0);
        let shadow = Mat4::from_translation(Vec3::new(rs.pos.x, floor + 0.3, rs.pos.z)) * Mat4::from_scale(Vec3::new(20.0, 1.0, 20.0));
        out.xlu.push(DrawCmd::new(MeshKey::named(oot_game::play::builtin::SHADOW), shadow));
        let v = &rs.values;
        let color = |i: usize| [v[i] as u8, v[i + 1] as u8, v[i + 2] as u8, v[i + 3] as u8];
        // EnOssan_DrawCursor: 16 * z either side, the IA4 cursor mirrored across 32 texels
        // (the rectangle's corners in quarter pixels, truncated).
        if rs.switches[rs::DRAW_CURSOR] != 0 {
            let (x, y, z) = (v[rs::CURSOR], v[rs::CURSOR + 1], v[rs::CURSOR + 2]);
            let w = 16.0 * z;
            let q = |f: f32| ((f * 4.0) as i32) as f32 / 4.0;
            let s = Sprite::rect(CURSOR_SPRITE, q(x - w), q(y - w), q(x + w), q(y + w), Some(color(rs::CURSOR + 3)), None);
            out.overlay_2d.push(s.draw_cmd());
        }
        // EnOssan_DrawStickDirectionPrompts: the arrows, then the sticks; the left pair drawn
        // flipped (dsdx -1).
        let enabled = [rs.switches[rs::LEFT_ENABLED] != 0, rs.switches[rs::RIGHT_ENABLED] != 0];
        if enabled[0] || enabled[1] {
            for (sprite, color_at, x_at) in [(ARROW_SPRITE, 6, 10), (STICK_SPRITE, 0, 4)] {
                for (k, on) in enabled.iter().enumerate() {
                    if !on {
                        continue;
                    }
                    let base = rs::PROMPTS + k * rs::PROMPT_LEN;
                    let (x, y, z) = (v[base + x_at], v[base + x_at + 1], v[base + 12]);
                    let (w, h) = (8.0 * z, 12.0 * z);
                    let q = |f: f32| ((f * 4.0) as i32) as f32 / 4.0;
                    let (x0, x1) = if k == 0 { (q(x + w), q(x - w)) } else { (q(x - w), q(x + w)) };
                    let s = Sprite::rect(sprite, x0, q(y - h), x1, q(y + h), Some(color(base + color_at)), None);
                    out.overlay_2d.push(s.draw_cmd());
                }
            }
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
