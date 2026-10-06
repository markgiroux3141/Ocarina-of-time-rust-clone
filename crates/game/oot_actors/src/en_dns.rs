//! `En_Dns` (`ovl_En_Dns/z_en_dns.c`): the Business Scrub's sale phase, the salesman a caught
//! `En_Shopnuts` turns into. He offers his item for rupees (a choice); bought or not, he
//! burrows down, spinning, in a cloud of dust, and is gone (leaving three recovery hearts after
//! a sale).
//!
//! Params (`DNS_GET_TYPE`, `EnDnsType`): what he sells (`sItemEntries`: the price, the item, the
//! checks and the payment), from 0, Deku Nuts (5 for 20), to 10, the Deku Nut upgrade; arrows
//! are Deku Seeds for a child. The Deku Tree's is 4, the Deku Shield for 50.
//!
//! - **Idle** (`EnDns_SetupIdle`, then `EnDns_Idle`): turning to Link; within 130 he offers to
//!   talk (`Actor_OfferTalkNearColChkInfoCylinder`), taken at once when Link touches him or locks
//!   on. His text (`sStartingTextIds`) ends in a choice.
//! - **The choice** (`EnDns_Talk`): "Yes" checks the sale (`EnDns_CanBuy*`): not enough rupees
//!   (0x10A5), the item at capacity or owned (0x10A6), not yet usable (0x10DE: seeds without the
//!   slingshot, bombs without the Goron's Ruby, arrows without the bow) — he burrows without a
//!   sale; else 0x10A7, and the item is offered (`Actor_OfferGetItem`, 130 by 100) until Link
//!   takes it; then he's paid (`EnDns_Pay*`, flags for the piece of heart and the upgrades) and
//!   burrows. "No" is 0x10A4 and he burrows.
//! - **Burrowing** (`EnDns_Burrow`, `EnDns_PostBurrow`): the animation, `NA_SE_EN_AKINDONUTS_HIDE`;
//!   then down through the floor (no floor check, gravity -1), spinning by 0x2000, dust every 4
//!   frames (`func_80028990`); 400 down, gone.
//!
//! The whole overlay is ported. Not ported: the circle shadow (`ActorShadow_DrawCircle`, 35),
//! not ported for any actor.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::smooth_step_to_s;
use glam::Vec3;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile, PLAYER_STATE1_10, audio_play_actor_sfx2};
use oot_game::audio::sfx::*;
use oot_game::collision_check::*;
use oot_game::item::*;
use oot_game::message::{TEXT_STATE_CHOICE, TEXT_STATE_DONE, TEXT_STATE_EVENT};
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::save::SaveContext;
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE, Anim, SkelAnimeStd};

use crate::en_shopnuts::{NAVI_ENEMY_BUSINESS_SCRUB, OBJECT};

/// `ACTOR_EN_DNS` (`actor_table.h`).
pub const ACTOR_EN_DNS: i16 = 0x011A;

/// `En_Dns_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_DNS, name: "En_Dns", category: ACTORCAT_BG, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY, object: OBJECT };

/// `EnDnsType`.
pub const DNS_TYPE_DEKU_NUTS_5: i16 = 0;
pub const DNS_TYPE_DEKU_STICKS_1: i16 = 1;
pub const DNS_TYPE_HEART_PIECE: i16 = 2;
pub const DNS_TYPE_DEKU_SEEDS_30: i16 = 3;
pub const DNS_TYPE_DEKU_SHIELD: i16 = 4;
pub const DNS_TYPE_BOMBS_5: i16 = 5;
pub const DNS_TYPE_ARROWS_30: i16 = 6;
pub const DNS_TYPE_RED_POTION: i16 = 7;
pub const DNS_TYPE_GREEN_POTION: i16 = 8;
pub const DNS_TYPE_DEKU_STICK_UPGRADE: i16 = 9;
pub const DNS_TYPE_DEKU_NUT_UPGRADE: i16 = 10;

/// `EnDnsCanBuyResult`.
pub const DNS_CANBUY_RESULT_NEED_RUPEES: u32 = 0;
pub const DNS_CANBUY_RESULT_CAPACITY_FULL: u32 = 1;
pub const DNS_CANBUY_RESULT_SUCCESS_NEW_ITEM: u32 = 2;
pub const DNS_CANBUY_RESULT_CANT_GET_NOW: u32 = 3;
pub const DNS_CANBUY_RESULT_SUCCESS: u32 = 4;

/// `EnDnsAnimation`.
const DNS_ANIM_IDLE: u8 = 0;
const DNS_ANIM_BURROW: u8 = 1;

/// `ITEMGETINF_DEKU_HEART_PIECE`, `INFTABLE_HAS_DEKU_STICK_UPGRADE`,
/// `INFTABLE_HAS_DEKU_NUT_UPGRADE` (`save.h`).
pub const ITEMGETINF_DEKU_HEART_PIECE: u16 = 0x0B;
pub const INFTABLE_HAS_DEKU_STICK_UPGRADE: u16 = 0x192;
pub const INFTABLE_HAS_DEKU_NUT_UPGRADE: u16 = 0x193;

/// `GI_*` (`item.h`) the salesmen sell that `oot_game::item` doesn't name.
const GI_BOMBS_5: i16 = 0x01;
const GI_BOTTLE_POTION_RED: i16 = 0x10;
const GI_BOTTLE_POTION_GREEN: i16 = 0x11;
const GI_ARROWS_30: i16 = 0x4B;
const GI_DEKU_NUTS_5_2: i16 = 0x63;
const GI_DEKU_SEEDS_30: i16 = 0x69;
const GI_DEKU_STICK_UPGRADE_20: i16 = 0x77;
const GI_DEKU_STICK_UPGRADE_30: i16 = 0x78;
const GI_DEKU_NUT_UPGRADE_30: i16 = 0x79;
const GI_DEKU_NUT_UPGRADE_40: i16 = 0x7A;

/// `ITEM00_RECOVERY_HEART` (`z_en_item00.h`).
const ITEM00_RECOVERY_HEART: i16 = 0x03;

/// `sCylinderInit` (`ColliderCylinderInitType1`: `Collider_SetCylinderType1` gives `OC2_TYPE_1`);
/// only its OC is ever set. 18 by 32.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 18, height: 32, y_shift: 0, pos: [0, 0, 0] },
};

/// `sStartingTextIds`.
pub const S_STARTING_TEXT_IDS: [u16; 11] = [0x10A0, 0x10A1, 0x10A2, 0x10CA, 0x10CB, 0x10CC, 0x10CD, 0x10CE, 0x10CF, 0x10DC, 0x10DD];

/// `sItemDebugTxt` (the debug build's names, for the log).
const S_ITEM_DEBUG_TXT: [&str; 11] =
    ["Deku Nuts", "Deku Sticks", "Piece of Heart", "Deku Seeds", "Deku Shield", "Bombs", "Arrows", "Red Potion", "Green Potion", "Deku Stick Upgrade", "Deku Nut Upgrade"];

/// `DnsItemEntry.canBuy`: which `EnDns_CanBuy*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanBuy {
    Price,
    DekuNuts,
    DekuSticks,
    DekuSeeds,
    DekuShield,
    Bombs,
    Arrows,
    Bottle,
}

/// `DnsItemEntry.payment`: which `EnDns_Pay*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Payment {
    Price,
    DekuNuts,
    HeartPiece,
    Bombs,
    Arrows,
    DekuStickUpgrade,
    DekuNutUpgrade,
}

/// `DnsItemEntry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DnsItemEntry {
    pub item_price: i16,
    pub item_amount: u16,
    pub get_item_id: i16,
    pub can_buy: CanBuy,
    pub payment: Payment,
}

/// `sItemEntries` (`sItemDekuNuts` to `sItemDekuNutUpgrade`).
pub const S_ITEM_ENTRIES: [DnsItemEntry; 11] = [
    DnsItemEntry { item_price: 20, item_amount: 5, get_item_id: GI_DEKU_NUTS_5_2, can_buy: CanBuy::DekuNuts, payment: Payment::DekuNuts },
    DnsItemEntry { item_price: 15, item_amount: 1, get_item_id: GI_DEKU_STICKS_1, can_buy: CanBuy::DekuSticks, payment: Payment::Price },
    DnsItemEntry { item_price: 10, item_amount: 1, get_item_id: GI_HEART_PIECE, can_buy: CanBuy::Price, payment: Payment::HeartPiece },
    DnsItemEntry { item_price: 40, item_amount: 30, get_item_id: GI_DEKU_SEEDS_30, can_buy: CanBuy::DekuSeeds, payment: Payment::Price },
    DnsItemEntry { item_price: 50, item_amount: 1, get_item_id: GI_SHIELD_DEKU, can_buy: CanBuy::DekuShield, payment: Payment::Price },
    DnsItemEntry { item_price: 40, item_amount: 5, get_item_id: GI_BOMBS_5, can_buy: CanBuy::Bombs, payment: Payment::Bombs },
    DnsItemEntry { item_price: 70, item_amount: 20, get_item_id: GI_ARROWS_30, can_buy: CanBuy::Arrows, payment: Payment::Arrows },
    DnsItemEntry { item_price: 40, item_amount: 1, get_item_id: GI_BOTTLE_POTION_RED, can_buy: CanBuy::Bottle, payment: Payment::Price },
    DnsItemEntry { item_price: 40, item_amount: 1, get_item_id: GI_BOTTLE_POTION_GREEN, can_buy: CanBuy::Bottle, payment: Payment::Price },
    DnsItemEntry { item_price: 40, item_amount: 1, get_item_id: GI_DEKU_STICK_UPGRADE_20, can_buy: CanBuy::Price, payment: Payment::DekuStickUpgrade },
    DnsItemEntry { item_price: 40, item_amount: 1, get_item_id: GI_DEKU_NUT_UPGRADE_30, can_buy: CanBuy::Price, payment: Payment::DekuNutUpgrade },
];

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    SetupIdle,
    Idle,
    Talk,
    SetupSale,
    Sale,
    SetupBurrow,
    SetupNoSaleBurrow,
    Burrow,
    PostBurrow,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::SetupIdle => "EnDns_SetupIdle",
            Action::Idle => "EnDns_Idle",
            Action::Talk => "EnDns_Talk",
            Action::SetupSale => "EnDns_SetupSale",
            Action::Sale => "EnDns_Sale",
            Action::SetupBurrow => "EnDns_SetupBurrow",
            Action::SetupNoSaleBurrow => "EnDns_SetupNoSaleBurrow",
            Action::Burrow => "EnDns_Burrow",
            Action::PostBurrow => "EnDns_PostBurrow",
        }
    }
}

/// `sAnimationInfo`: nervous idle (loop), leaving by burrowing (once), the nervous transition
/// (once).
struct Anims([(Anim, u8); 3]);

impl Anims {
    fn load(play: &PlayState) -> Option<Anims> {
        let a = play.assets.clone()?;
        let get = |s: &str| a.animation(OBJECT, s).map_err(|e| log::error!("En_Dns: {e:#}")).ok();
        Some(Anims([(get("gBusinessScrubNervousIdleAnim")?, ANIMMODE_LOOP), (get("gBusinessScrubLeaveBurrowAnim")?, ANIMMODE_ONCE), (get("gBusinessScrubNervousTransitionAnim")?, ANIMMODE_ONCE)]))
    }
}

pub struct EnDns {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: Option<Anims>,
    pub action: Action,
    pub collider: ColliderCylinder,
    /// `dustTimer`.
    pub dust_timer: i16,
    /// `animIndex` ("set but not read").
    pub anim_index: u8,
    /// `isColliderEnabled`, `standOnGround`, `dropCollectible`.
    pub is_collider_enabled: bool,
    pub stand_on_ground: bool,
    pub drop_collectible: bool,
    /// `dnsItemEntry`.
    pub dns_item_entry: DnsItemEntry,
    /// `yInitPos`.
    pub y_init_pos: f32,
}

impl EnDns {
    /// `EnDns_Init`: a negative type is an error (gone); arrows are seeds for a child; the
    /// skeleton (`gBusinessScrubNervousTransitionAnim`), the collider, his text, immovable,
    /// falling by 1, his item.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let collider = ColliderCylinder::new(&CYLINDER_INIT);
        let mut n = EnDns {
            actor: Actor::new(Vec3::ZERO, 0),
            skel: SkelAnimeStd::init_flex(0, None),
            skeleton: None,
            anims: None,
            action: Action::SetupIdle,
            collider,
            dust_timer: 0,
            anim_index: 0,
            is_collider_enabled: false,
            stand_on_ground: false,
            drop_collectible: false,
            dns_item_entry: S_ITEM_ENTRIES[0],
            y_init_pos: 0.0,
        };
        if actor.params < 0 {
            log::debug!("Argument error (selling nuts) [ arg_data = {} ]", actor.params);
            actor.kill();
            n.actor = actor;
            return Box::new(n);
        }
        // Sell Seeds instead of Arrows if Link is child (LINK_AGE_IN_YEARS == YEARS_CHILD).
        if actor.params == DNS_TYPE_ARROWS_30 && !play.save.adult {
            actor.params = DNS_TYPE_DEKU_SEEDS_30;
        }
        let ty = (actor.params as usize).min(S_ITEM_ENTRIES.len() - 1);
        log::debug!("Selling nuts: {}", S_ITEM_DEBUG_TXT[ty]);
        // sInitChain: NAVI_ENEMY_BUSINESS_SCRUB, ATTENTION_RANGE_2, lockOnArrowOffset 30.
        actor.navi_enemy_id = NAVI_ENEMY_BUSINESS_SCRUB;
        actor.target_mode = 2;
        actor.target_arrow_offset = 30.0;
        let skeleton = play.assets.clone().and_then(|a| a.skeleton(OBJECT, "gBusinessScrubSkel").map_err(|e| log::error!("En_Dns: {e:#}")).ok());
        let anims = Anims::load(play);
        // SkelAnime_InitFlex(&gBusinessScrubSkel, &gBusinessScrubNervousTransitionAnim).
        let limbs = skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(17);
        n.skel = SkelAnimeStd::init_flex(limbs, anims.as_ref().map(|a| a.0[2].0.clone()));
        n.skeleton = skeleton;
        n.anims = anims;
        // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 35): the circle shadow isn't ported.
        actor.shape_y_offset = 0.0;
        actor.text_id = S_STARTING_TEXT_IDS[ty];
        actor.scale = Vec3::splat(0.01);
        actor.col_chk_info.mass = MASS_IMMOVABLE;
        n.is_collider_enabled = true;
        n.stand_on_ground = true;
        n.drop_collectible = false;
        actor.speed_xz = 0.0;
        actor.velocity.y = 0.0;
        actor.gravity = -1.0;
        n.dns_item_entry = S_ITEM_ENTRIES[ty];
        n.actor = actor;
        n.action = Action::SetupIdle;
        Box::new(n)
    }

    /// `DNS_GET_TYPE`.
    fn ty(&self) -> usize {
        (self.actor.params as usize).min(S_ITEM_ENTRIES.len() - 1)
    }

    /// `EnDns_ChangeAnim`.
    fn change_anim(&mut self, index: u8) {
        self.anim_index = index;
        if let Some((a, mode)) = self.anims.as_ref().map(|a| a.0[index as usize].clone()) {
            // Animation_GetLastFrame as an s16.
            let frame_count = a.last_frame() as i16 as f32;
            self.skel.change(a, 1.0, 0.0, frame_count, mode, 0.0);
        }
    }

    /// `EnDns_CanBuy*` for his item.
    fn can_buy(&self, s: &SaveContext) -> u32 {
        let price = self.dns_item_entry.item_price;
        let rupees = s.rupees;
        match self.dns_item_entry.can_buy {
            // EnDns_CanBuyPrice.
            CanBuy::Price => {
                if rupees < price {
                    return DNS_CANBUY_RESULT_NEED_RUPEES;
                }
                DNS_CANBUY_RESULT_SUCCESS
            }
            // EnDns_CanBuyDekuNuts.
            CanBuy::DekuNuts => {
                if s.cur_capacity(UPG_DEKU_NUTS) != 0 && s.ammo(ITEM_DEKU_NUT) as i32 >= s.cur_capacity(UPG_DEKU_NUTS) as i32 {
                    return DNS_CANBUY_RESULT_CAPACITY_FULL;
                }
                if rupees < price {
                    return DNS_CANBUY_RESULT_NEED_RUPEES;
                }
                if item_check_obtainability(s, ITEM_DEKU_NUT) == ITEM_NONE {
                    return DNS_CANBUY_RESULT_SUCCESS_NEW_ITEM;
                }
                DNS_CANBUY_RESULT_SUCCESS
            }
            // EnDns_CanBuyDekuSticks.
            CanBuy::DekuSticks => {
                if s.cur_capacity(UPG_DEKU_STICKS) != 0 && s.ammo(ITEM_DEKU_STICK) as i32 >= s.cur_capacity(UPG_DEKU_STICKS) as i32 {
                    return DNS_CANBUY_RESULT_CAPACITY_FULL;
                }
                if rupees < price {
                    return DNS_CANBUY_RESULT_NEED_RUPEES;
                }
                if item_check_obtainability(s, ITEM_DEKU_STICK) == ITEM_NONE {
                    return DNS_CANBUY_RESULT_SUCCESS_NEW_ITEM;
                }
                DNS_CANBUY_RESULT_SUCCESS
            }
            // EnDns_CanBuyDekuSeeds.
            CanBuy::DekuSeeds => {
                if s.inv_content(ITEM_SLINGSHOT) == ITEM_NONE {
                    return DNS_CANBUY_RESULT_CANT_GET_NOW;
                }
                if s.ammo(ITEM_SLINGSHOT) as i32 >= s.cur_capacity(UPG_BULLET_BAG) as i32 {
                    return DNS_CANBUY_RESULT_CAPACITY_FULL;
                }
                if rupees < price {
                    return DNS_CANBUY_RESULT_NEED_RUPEES;
                }
                if item_check_obtainability(s, ITEM_DEKU_SEEDS) == ITEM_NONE {
                    return DNS_CANBUY_RESULT_SUCCESS_NEW_ITEM;
                }
                DNS_CANBUY_RESULT_SUCCESS
            }
            // EnDns_CanBuyDekuShield (CHECK_OWNED_EQUIP_ALT).
            CanBuy::DekuShield => {
                if s.check_owned_equip(EQUIP_TYPE_SHIELD, EQUIP_INV_SHIELD_DEKU) {
                    return DNS_CANBUY_RESULT_CAPACITY_FULL;
                }
                if rupees < price {
                    return DNS_CANBUY_RESULT_NEED_RUPEES;
                }
                DNS_CANBUY_RESULT_SUCCESS
            }
            // EnDns_CanBuyBombs.
            CanBuy::Bombs => {
                if !s.check_quest_item(QUEST_GORON_RUBY) {
                    return DNS_CANBUY_RESULT_CANT_GET_NOW;
                }
                if s.ammo(ITEM_BOMB) as i32 >= s.cur_capacity(UPG_BOMB_BAG) as i32 {
                    return DNS_CANBUY_RESULT_CAPACITY_FULL;
                }
                if rupees < price {
                    return DNS_CANBUY_RESULT_NEED_RUPEES;
                }
                DNS_CANBUY_RESULT_SUCCESS
            }
            // EnDns_CanBuyArrows.
            CanBuy::Arrows => {
                if item_check_obtainability(s, ITEM_BOW) == ITEM_NONE {
                    return DNS_CANBUY_RESULT_CANT_GET_NOW;
                }
                if s.ammo(ITEM_BOW) as i32 >= s.cur_capacity(UPG_QUIVER) as i32 {
                    return DNS_CANBUY_RESULT_CAPACITY_FULL;
                }
                if rupees < price {
                    return DNS_CANBUY_RESULT_NEED_RUPEES;
                }
                DNS_CANBUY_RESULT_SUCCESS
            }
            // EnDns_CanBuyBottle.
            CanBuy::Bottle => {
                if !inventory_has_empty_bottle(s) {
                    return DNS_CANBUY_RESULT_CAPACITY_FULL;
                }
                if rupees < price {
                    return DNS_CANBUY_RESULT_NEED_RUPEES;
                }
                DNS_CANBUY_RESULT_SUCCESS
            }
        }
    }

    /// `EnDns_Pay*`: the price (`Rupees_ChangeBy`), and the flag of a piece of heart or an
    /// upgrade.
    fn payment(&self, s: &mut SaveContext) {
        match self.dns_item_entry.payment {
            Payment::Price | Payment::DekuNuts | Payment::Bombs | Payment::Arrows => {}
            Payment::HeartPiece => s.set_item_get_inf(ITEMGETINF_DEKU_HEART_PIECE),
            Payment::DekuStickUpgrade => s.set_inf_table(INFTABLE_HAS_DEKU_STICK_UPGRADE),
            Payment::DekuNutUpgrade => s.set_inf_table(INFTABLE_HAS_DEKU_NUT_UPGRADE),
        }
        rupees_change_by(s, -self.dns_item_entry.item_price);
    }

    /// `EnDns_SetupIdle`: at the transition's last frame, the nervous idle.
    fn setup_idle(&mut self) {
        if self.skel.cur_frame == self.skel.end_frame {
            self.action = Action::Idle;
            self.change_anim(DNS_ANIM_IDLE);
        }
    }

    /// `EnDns_Idle`: turning to Link; a talk taken, talking; else within 130 the offer, taken at
    /// once when he touches Link or is locked on.
    fn idle(&mut self, play: &mut PlayState) {
        smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 3, 2000, 0);
        self.actor.world_rot.y = self.actor.shape_rot.y;
        if oot_game::npc::process_talk_request(&mut self.actor) {
            self.action = Action::Talk;
        } else {
            if self.collider.base.oc_flags1 & OC1_HIT != 0 || self.actor.is_targeted {
                self.actor.flags |= ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
            } else {
                self.actor.flags &= !ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
            }
            if self.actor.xz_dist_to_player < 130.0 {
                let a = self.actor.clone();
                oot_game::npc::offer_talk_default(play, &a);
            }
        }
    }

    /// `EnDns_Talk`: the choice made: "OK" checks the sale, "No" (0x10A4) burrows.
    fn talk(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_CHOICE && play.message_should_advance() {
            match play.msg_ctx.choice_index {
                0 => match self.can_buy(&play.save) {
                    DNS_CANBUY_RESULT_NEED_RUPEES => {
                        play.continue_textbox(0x10A5);
                        self.action = Action::SetupNoSaleBurrow;
                    }
                    DNS_CANBUY_RESULT_CAPACITY_FULL => {
                        play.continue_textbox(0x10A6);
                        self.action = Action::SetupNoSaleBurrow;
                    }
                    DNS_CANBUY_RESULT_CANT_GET_NOW => {
                        play.continue_textbox(0x10DE);
                        self.action = Action::SetupNoSaleBurrow;
                    }
                    DNS_CANBUY_RESULT_SUCCESS_NEW_ITEM | DNS_CANBUY_RESULT_SUCCESS => {
                        play.continue_textbox(0x10A7);
                        self.action = Action::SetupSale;
                    }
                    _ => {}
                },
                1 => {
                    play.continue_textbox(0x10A4);
                    self.action = Action::SetupNoSaleBurrow;
                }
                _ => {}
            }
        }
    }

    /// `EnDns_OfferSaleItem`: the upgrades by the current level, else his item; within 130 by
    /// 100.
    fn offer_sale_item(&self, play: &mut PlayState) {
        let ty = self.actor.params;
        let gi = if ty == DNS_TYPE_DEKU_STICK_UPGRADE {
            if play.save.cur_upg_value(UPG_DEKU_STICKS) < 2 { GI_DEKU_STICK_UPGRADE_20 } else { GI_DEKU_STICK_UPGRADE_30 }
        } else if ty == DNS_TYPE_DEKU_NUT_UPGRADE {
            if play.save.cur_upg_value(UPG_DEKU_NUTS) < 2 { GI_DEKU_NUT_UPGRADE_30 } else { GI_DEKU_NUT_UPGRADE_40 }
        } else {
            self.dns_item_entry.get_item_id
        };
        let a = self.actor.clone();
        oot_game::get_item::offer_get_item_range(play, &a, gi, 130.0, 100.0);
    }

    /// `EnDns_SetupSale`: 0x10A7's event advanced: the text closed and the item offered.
    fn setup_sale(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            play.with_msg(|m, f| m.close_textbox(f.audio));
            self.offer_sale_item(play);
            self.action = Action::Sale;
        }
    }

    /// `EnDns_Sale`: offering it until Link takes it.
    fn sale(&mut self, play: &mut PlayState) {
        if oot_game::get_item::actor_has_parent(&self.actor) {
            self.actor.parent = None;
            self.action = Action::SetupBurrow;
        } else {
            self.offer_sale_item(play);
        }
    }

    /// The end of the sale: paid, the hearts to come, the collider off, untargetable, burrowing.
    fn pay_and_burrow(&mut self, play: &mut PlayState) {
        self.payment(&mut play.save);
        self.drop_collectible = true;
        self.is_collider_enabled = false;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.change_anim(DNS_ANIM_BURROW);
        self.action = Action::Burrow;
    }

    /// `EnDns_SetupBurrow`: with Link holding up the item (`PLAYER_STATE1_10`), once its text is
    /// done and advanced; else at once.
    fn setup_burrow(&mut self, play: &mut PlayState) {
        let state1 = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map(|p| p.state_flags1()).unwrap_or(0);
        if state1 & PLAYER_STATE1_10 != 0 {
            if play.message_state() == TEXT_STATE_DONE && play.message_should_advance() {
                self.pay_and_burrow(play);
            }
        } else {
            self.pay_and_burrow(play);
        }
    }

    /// `EnDns_SetupNoSaleBurrow`: the text done and advanced: burrowing, unpaid.
    fn setup_no_sale_burrow(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_DONE && play.message_should_advance() {
            self.is_collider_enabled = false;
            self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
            self.change_anim(DNS_ANIM_BURROW);
            self.action = Action::Burrow;
        }
    }

    /// `EnDns_Burrow`: at the animation's last frame, `NA_SE_EN_AKINDONUTS_HIDE`, and down.
    fn burrow(&mut self, play: &mut PlayState) {
        let frame_count = self.anims.as_ref().map(|a| a.0[DNS_ANIM_BURROW as usize].0.last_frame()).unwrap_or(0.0);
        if self.skel.cur_frame == frame_count {
            audio_play_actor_sfx2(play, NA_SE_EN_AKINDONUTS_HIDE);
            self.action = Action::PostBurrow;
            self.stand_on_ground = false;
            self.y_init_pos = self.actor.world_pos.y;
        }
    }

    /// `EnDns_PostBurrow`: dust every 4 frames where he went down, spinning; 400 down, three
    /// recovery hearts after a sale, and gone.
    fn post_burrow(&mut self, play: &mut PlayState) {
        let depth_in_ground = self.y_init_pos - self.actor.world_pos.y;
        if self.dust_timer & 3 == 0 {
            let init_pos = Vec3::new(self.actor.world_pos.x, self.y_init_pos, self.actor.world_pos.z);
            play.with_ss(|s| s.func_80028990(20.0, init_pos));
        }
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x2000);
        if depth_in_ground > 400.0 {
            if self.drop_collectible {
                let init_pos = Vec3::new(self.actor.world_pos.x, self.y_init_pos, self.actor.world_pos.z);
                for _ in 0..3 {
                    crate::en_item00::item_drop_collectible(play, init_pos, ITEM00_RECOVERY_HEART);
                }
            }
            self.actor.kill();
        }
    }
}

impl ActorImpl for EnDns {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnDns_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.dust_timer = self.dust_timer.wrapping_add(1);
        self.actor.text_id = S_STARTING_TEXT_IDS[self.ty()];
        self.actor.set_focus(60.0);
        self.actor.scale = Vec3::splat(0.01);
        self.skel.update();
        self.actor.move_forward();
        match self.action {
            Action::SetupIdle => self.setup_idle(),
            Action::Idle => self.idle(play),
            Action::Talk => self.talk(play),
            Action::SetupSale => self.setup_sale(play),
            Action::Sale => self.sale(play),
            Action::SetupBurrow => self.setup_burrow(play),
            Action::SetupNoSaleBurrow => self.setup_no_sale_burrow(play),
            Action::Burrow => self.burrow(play),
            Action::PostBurrow => self.post_burrow(play),
        }
        if self.stand_on_ground {
            self.actor.update_bg_check_info(&play.col, 20.0, 20.0, 20.0, UPDBGCHECKINFO_FLAG_2);
        }
        if self.is_collider_enabled {
            self.collider.update(&self.actor);
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs
    }

    /// `EnDns_Draw`: the skeleton (`Gfx_SetupDL_25Opa`, `SkelAnime_DrawFlexOpa`).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(joints), Some(skeleton)) = (&rs.joints, &self.skeleton) else { return };
        let bones = skeleton.pose(joints);
        let model = oot_game::play::actor_draw_matrix(rs);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::mesh(OBJECT, "gBusinessScrubSkel")), transform: model, bones, params: Default::default() });
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0).then_some(ColliderMut::Cylinder(&mut self.collider))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
