//! The Kokiri shop (`kokiri_shop_scene`): `En_Ossan` (params 0, `OSSAN_TYPE_KOKIRI`), its
//! `En_GirlA` shelf items on the `En_Tana` shelves, and the pause menu's equipping stand-in in
//! the play frame (`KaleidoSetup_Update`).
//!
//! Expected values from the C:
//! - `EnOssan_InitActionFunc`: the shopkeeper 33 in front of his placement (0, 0, -59), scale
//!   0.01, not targetable, text 0x9E; `EnOssan_SpawnItemsOnShelves`: `sShopkeeperStores[0]`'s
//!   items at their offsets from the shelves (0, 0, -20), turned `sItemShelfRot`;
//! - `EnGirlA_WaitForObject`: scale 0.25, 24 up, the description text, the price;
//! - talking: `PLAYER_STATE2_29`, `Play_SetShopBrowsingViewpoint` (bg camera 1,
//!   `CAM_SET_PIVOT_SHOP_BROWSING`), `YREG(31)` 1; 0x9E, then 0x83;
//! - the stick right: `EnOssan_State_LookToRightShelf` approaches -30 degrees
//!   (`Math_ApproachF(.., 0.5, 10)`), `Camera_SetCameraData`'s `data2`; the cursor on slot 0,
//!   0x9F;
//! - A: the buy prompt 0x89 and `EnOssan_TakeItemOffShelf` (0.15 a frame, to
//!   `sSelectedItemPosition[0]` (17, 58, 30) from the shelves);
//! - "Buy": `EnGirlA_CanBuy_DekuShield` (owned: 0x86; under 40 rupees: 0x85), or the get-item
//!   flow (`Actor_OfferGetItem` within 120), `Item_Give` owns it, and the 40 are charged after its
//!   text (`EnGirlA_BuyEvent_ShieldDiscount`); 0x6B; B ends it (`EnOssan_EndInteraction`).

mod common;

use std::sync::Arc;

use eng_input::pad::{BTN_A, BTN_B, BTN_START, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_girla::*;
use oot_actors::en_ossan::*;
use oot_actors::en_tana::EnTana;
use oot_actors::player::{Action, STATE2_29};
use oot_game::actor::ACTOR_FLAG_ATTENTION_ENABLED;
use oot_game::actor_ctx::ActorHandle;
use oot_game::camera::CAM_SET_PIVOT_SHOP_BROWSING;
use oot_game::item::*;
use oot_game::message::{TEXT_STATE_CHOICE, TEXT_STATE_DONE, TEXT_STATE_EVENT};
use oot_game::play::{PlayState, VIEWPOINT_LOCKED, VIEWPOINT_PIVOT, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

const A: PadState = PadState { button: BTN_A, stick_x: 0, stick_y: 0 };
const B: PadState = PadState { button: BTN_B, stick_x: 0, stick_y: 0 };
const NONE: PadState = PadState { button: 0, stick_x: 0, stick_y: 0 };
const RIGHT: PadState = PadState { button: 0, stick_x: 60, stick_y: 0 };

struct Shop {
    w: PlayState,
    prev: PadState,
}

impl Shop {
    /// The shop on a new save with `rupees`, changed by `f`, settled.
    fn enter(rupees: i16, f: impl FnOnce(&mut SaveContext)) -> Option<Shop> {
        let a = assets()?;
        let e = a.scenes.entrance_index("ENTR_KOKIRI_SHOP_0").unwrap();
        let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
        save.rupees = rupees;
        f(&mut save);
        let w = oot_actors::play_entrance(a, common::data()?, common::rules()?, save).expect("Play_Init");
        let mut s = Shop { w, prev: NONE };
        s.frames(NONE, 40);
        Some(s)
    }

    fn frames(&mut self, pad: PadState, n: usize) {
        for _ in 0..n {
            self.w.tick_with(scripted_input(self.prev, pad));
            self.prev = pad;
        }
    }

    /// One press (and a release after it).
    fn press(&mut self, pad: PadState) {
        self.frames(pad, 1);
        self.frames(NONE, 1);
    }

    fn ossan(&self) -> &EnOssan {
        self.w.actors.all().into_iter().find_map(|h| self.w.actors.downcast::<EnOssan>(h)).expect("En_Ossan")
    }

    fn item(&self, h: ActorHandle) -> &EnGirlA {
        self.w.actors.downcast::<EnGirlA>(h).expect("En_GirlA")
    }

    /// Runs idle frames until `done`, at most `n`.
    fn until(&mut self, n: usize, done: impl Fn(&Shop) -> bool) {
        for _ in 0..n {
            if done(self) {
                return;
            }
            self.frames(NONE, 1);
        }
        panic!("not done: shopkeeper state {}, text {:#x}, message state {}", self.ossan().state_flag, self.w.msg_ctx.text_id, self.w.message_state());
    }

    /// From the counter: talk, A at "Welcome!", on to 0x83's choice.
    fn talk(&mut self) {
        self.w.place_player(Vec3::new(20.0, 0.0, 40.0), i16::MIN);
        self.frames(NONE, 2);
        let h = self.w.actors.all().into_iter().find(|&h| self.w.actors.downcast::<EnOssan>(h).is_some()).unwrap();
        assert_eq!(self.w.player().target_actor, Some(h), "the shopkeeper offers to talk within 100");
        self.press(A);
        self.until(60, |s| s.ossan().state_flag == OSSAN_STATE_START_CONVERSATION && s.w.message_state() == TEXT_STATE_EVENT);
        self.press(A);
        self.until(60, |s| s.ossan().state_flag == OSSAN_STATE_FACING_SHOPKEEPER && s.w.message_state() == TEXT_STATE_CHOICE);
    }

    /// The stick right, to the right shelf's first item.
    fn browse_right(&mut self) {
        self.press(RIGHT);
        self.until(60, |s| s.ossan().state_flag == OSSAN_STATE_BROWSE_RIGHT_SHELF && s.ossan().draw_cursor != 0 && s.w.message_state() == TEXT_STATE_EVENT);
    }

    /// A on the item, then A on "Buy" once the item is off the shelf and the choice is up.
    fn buy(&mut self) {
        self.press(A);
        self.until(60, |s| s.w.message_state() == TEXT_STATE_CHOICE && s.ossan().shop_item_selected_tween == 1.0);
        assert_eq!(self.w.msg_ctx.choice_index, 0);
        self.press(A);
    }
}

#[test]
fn the_kokiri_shop_sets_up_its_shelves() {
    let Some(s) = Shop::enter(0, |_| {}) else { return };
    let w = &s.w;
    let o = s.ossan();
    // EnOssan_InitActionFunc: sShopkeeperPositionOffsets[0] (0, 0, 33), sShopkeeperScale[0].
    assert_eq!(o.action, oot_actors::en_ossan::Action::Main);
    assert_eq!((o.actor.world_pos, o.actor.scale), (Vec3::new(0.0, 0.0, -26.0), Vec3::splat(0.01)));
    assert_eq!((o.actor.text_id, o.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, o.actor.target_mode), (0x9E, 0, 2));
    assert_eq!((o.state_flag, o.cursor_z, o.cursor_color), (OSSAN_STATE_IDLE, 1.5, [0, 255 - (80.0 * o.cursor_anim_tween) as u32, 80, 255]));
    let shelves = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<EnTana>(h)).expect("En_Tana");
    assert_eq!(shelves.actor.world_pos, Vec3::new(0.0, 0.0, -20.0));
    // sShopkeeperStores[OSSAN_TYPE_KOKIRI], with each item's sShopItemEntries row.
    let store = [
        (SI_DEKU_SHIELD, [50, 52, -20], 40, 0x9F, 0x89),
        (SI_DEKU_NUTS_5, [50, 76, -20], 15, 0xB2, 0x7F),
        (SI_DEKU_NUTS_10, [80, 52, -3], 30, 0xA2, 0x87),
        (SI_DEKU_STICK, [80, 76, -3], 10, 0xA1, 0x88),
        (SI_DEKU_SEEDS_30, [-50, 52, -20], 30, 0xDF, 0xDE),
        (SI_ARROWS_10, [-50, 76, -20], 20, 0xA0, 0x8A),
        (SI_ARROWS_30, [-80, 52, -3], 60, 0xC1, 0x9B),
        (SI_RECOVERY_HEART, [-80, 76, -3], 10, 0xAC, 0x95),
    ];
    for (i, &(si, off, price, desc, prompt)) in store.iter().enumerate() {
        let h = o.shelf_slots[i].expect("every slot filled");
        let g = s.item(h);
        assert_eq!(g.actor.params, si, "slot {i}");
        assert_eq!(g.actor.world_pos, Vec3::new(off[0] as f32, off[1] as f32, off[2] as f32 - 20.0), "slot {i}");
        // sItemShelfRot: the right shelf 0xEAAC, the left 0x1554.
        assert_eq!(g.actor.shape_rot.y as u16, if i < 4 { 0xEAAC } else { 0x1554 });
        assert_eq!((g.action, g.actor.scale, g.actor.shape_y_offset), (oot_actors::en_girla::Action::Update2, Vec3::splat(0.25), 24.0));
        assert_eq!((g.base_price, g.actor.text_id, g.item_buy_prompt_text_id), (price, desc, prompt), "slot {i}");
        assert!(g.drawn && !g.is_invisible && !g.is_selected);
    }
    let shield = s.item(o.shelf_slots[0].unwrap());
    assert_eq!((shield.get_item_id, shield.gi_draw_id), (GI_SHIELD_DEKU, 0x1C));
}

#[test]
fn browsing_and_buying_the_deku_shield() {
    let Some(mut s) = Shop::enter(40, |_| {}) else { return };
    s.talk();
    // Link hidden, the browsing camera (viewpoint 2: bg camera 1), YREG(31) 1, 0x83.
    assert_ne!(s.w.player().state2 & STATE2_29, 0);
    assert_eq!((s.w.viewpoint, s.w.game_camera.bg_cam_index, s.w.game_camera.setting), (VIEWPOINT_PIVOT, 1, CAM_SET_PIVOT_SHOP_BROWSING));
    assert_eq!((s.w.msg_ctx.yreg_31, s.w.msg_ctx.text_id), (1, 0x83));
    assert!(s.ossan().stick_left_prompt.is_enabled && s.ossan().stick_right_prompt.is_enabled);

    // The stick right: the camera turns to -30 (0.5 of the way, 10 at most, a frame), then the
    // cursor on slot 0 (EnOssan_SetCursorIndexFromNeutral(0): the bottom row first).
    s.frames(RIGHT, 1);
    assert_eq!(s.ossan().state_flag, OSSAN_STATE_LOOK_SHELF_RIGHT);
    s.frames(NONE, 1);
    let mut angles = vec![s.ossan().camera_face_angle];
    for _ in 0..8 {
        s.frames(NONE, 1);
        angles.push(s.ossan().camera_face_angle);
    }
    assert_eq!(&angles[..4], &[-10.0, -20.0, -25.0, -27.5]);
    s.until(30, |s| s.ossan().state_flag == OSSAN_STATE_BROWSE_RIGHT_SHELF && s.ossan().draw_cursor != 0);
    assert_eq!((s.ossan().cursor_index, s.w.game_camera.data2), (0, -30));
    s.until(30, |s| s.w.message_state() == TEXT_STATE_EVENT);
    assert_eq!(s.w.msg_ctx.text_id, 0x9F);
    let shield = s.ossan().shelf_slots[0].unwrap();
    assert!(s.item(shield).is_selected, "the item under the cursor spins");

    // A: the buy prompt, and the shield off the shelf towards Link.
    s.press(A);
    assert_eq!((s.ossan().state_flag, s.w.msg_ctx.text_id), (OSSAN_STATE_SELECT_ITEM, 0x89));
    s.until(30, |s| s.ossan().shop_item_selected_tween == 1.0);
    // sSelectedItemPosition[0] (17, 58, 30) from the shelves.
    assert_eq!(s.w.actors.actor(shield).unwrap().world_pos, Vec3::new(17.0, 58.0, 10.0));
    s.until(60, |s| s.w.message_state() == TEXT_STATE_CHOICE);
    s.press(A);
    // CANBUY_RESULT_SUCCESS_FANFARE: the offer, the box closing, Link shown, the fixed view,
    // and the shield off the shelf for good (EnGirlA_SetItemOutOfStock).
    assert_eq!(s.ossan().state_flag, OSSAN_STATE_GIVE_ITEM_FANFARE);
    assert_eq!(s.w.player().get_item_id, GI_SHIELD_DEKU);
    assert_eq!(s.w.viewpoint, VIEWPOINT_LOCKED);
    assert_eq!(s.w.player().state2 & STATE2_29, 0);
    assert!(s.item(shield).is_invisible && !s.item(shield).drawn);
    // Player takes it when his talk ends: the get-item action, the shopkeeper its parent's
    // child no more (Actor_HasParent), and Item_Give owns it (not worn); nothing charged yet.
    s.until(80, |s| s.w.player().action == Action::GetItem && s.ossan().state_flag == OSSAN_STATE_ITEM_PURCHASED);
    s.until(200, |s| s.w.msg_ctx.text_id == 0x4C);
    assert!(s.w.save.check_owned_equip(EQUIP_TYPE_SHIELD, EQUIP_INV_SHIELD_DEKU));
    assert_eq!(s.w.save.cur_equip_value(EQUIP_TYPE_SHIELD), EQUIP_VALUE_SHIELD_NONE);
    assert_eq!((s.w.save.rupees, s.w.save.rupee_accumulator), (40, 0));
    // Through the text's two boxes; at its end A: the price, and 0x6B.
    for _ in 0..400 {
        if s.ossan().state_flag == OSSAN_STATE_CONTINUE_SHOPPING_PROMPT {
            break;
        }
        let st = s.w.message_state();
        if matches!(st, TEXT_STATE_DONE | oot_game::message::TEXT_STATE_AWAITING_NEXT) {
            s.press(A);
        } else {
            s.frames(NONE, 1);
        }
    }
    assert_eq!((s.ossan().state_flag, s.w.msg_ctx.text_id), (OSSAN_STATE_CONTINUE_SHOPPING_PROMPT, 0x6B));
    assert_eq!(s.w.save.rupees + s.w.save.rupee_accumulator, 0);
    s.until(60, |s| s.w.message_state() == TEXT_STATE_CHOICE);
    // B: EnOssan_EndInteraction.
    s.press(B);
    assert_eq!((s.ossan().state_flag, s.w.msg_ctx.yreg_31), (OSSAN_STATE_IDLE, 0));
    s.until(80, |s| s.w.msg_ctx.msg_mode == 0 && s.w.player().action == Action::StandingStill);
    // The shield back on the shelf (updateStockedItemFunc); the 40 still counting down
    // (Interface_Update: one a frame).
    assert!(!s.item(shield).is_invisible);
    assert_eq!(s.w.save.rupees + s.w.save.rupee_accumulator, 0);
}

#[test]
fn without_the_rupees_you_dont_have_enough() {
    let Some(mut s) = Shop::enter(39, |_| {}) else { return };
    s.talk();
    s.browse_right();
    s.buy();
    // CANBUY_RESULT_NEED_RUPEES: 0x85, then A back to the description.
    assert_eq!((s.ossan().state_flag, s.w.msg_ctx.text_id), (OSSAN_STATE_CANT_GET_ITEM, 0x85));
    s.until(60, |s| s.w.message_state() == TEXT_STATE_EVENT);
    s.press(A);
    assert_eq!((s.ossan().state_flag, s.w.msg_ctx.text_id), (OSSAN_STATE_BROWSE_RIGHT_SHELF, 0x9F));
    assert_eq!(s.w.save.rupees, 39);
    assert!(!s.w.save.check_owned_equip(EQUIP_TYPE_SHIELD, EQUIP_INV_SHIELD_DEKU));
}

#[test]
fn a_second_deku_shield_cant_be_got_now() {
    let Some(mut s) = Shop::enter(99, |sv| {
        item_give(sv, None, ITEM_SHIELD_DEKU);
    }) else {
        return;
    };
    s.talk();
    s.browse_right();
    s.buy();
    // EnGirlA_CanBuy_DekuShield: owned, CANBUY_RESULT_CANT_GET_NOW: 0x86.
    assert_eq!((s.ossan().state_flag, s.w.msg_ctx.text_id), (OSSAN_STATE_CANT_GET_ITEM, 0x86));
}

#[test]
fn a_quick_buy_of_deku_nuts_once_owned() {
    // With nuts owned (the first give sets the upgrade: 20), SI_DEKU_NUTS_5 is a quick buy:
    // itemGiveFunc (Item_Give(ITEM_DEKU_NUTS_5), 15 rupees), "Thanks a lot!" (0x84).
    let Some(mut s) = Shop::enter(20, |sv| {
        item_give(sv, None, ITEM_DEKU_NUTS_5);
    }) else {
        return;
    };
    let nuts = s.w.save.ammo(ITEM_DEKU_NUT);
    s.talk();
    s.browse_right();
    // The stick up to slot 1 (EnOssan_CursorUpDown: the top row).
    s.frames(PadState { stick_y: 60, ..NONE }, 1);
    s.frames(NONE, 2);
    assert_eq!((s.ossan().cursor_index, s.w.msg_ctx.text_id), (1, 0xB2));
    s.until(30, |s| s.w.message_state() == TEXT_STATE_EVENT);
    s.buy();
    assert_eq!((s.ossan().state_flag, s.w.msg_ctx.text_id), (OSSAN_STATE_QUICK_BUY, 0x84));
    assert_eq!(s.w.save.ammo(ITEM_DEKU_NUT), nuts + 5);
    assert_eq!(s.w.save.rupees + s.w.save.rupee_accumulator, 5);
    s.until(60, |s| s.w.message_state() == TEXT_STATE_EVENT);
    s.press(A);
    assert_eq!((s.ossan().state_flag, s.w.msg_ctx.text_id), (OSSAN_STATE_BROWSE_RIGHT_SHELF, 0xB2));
}

#[test]
fn the_left_shelf_and_back() {
    let Some(mut s) = Shop::enter(99, |_| {}) else { return };
    s.talk();
    // The stick left: to +30, the cursor on slot 4 (the seeds), which a child without a
    // slingshot can't get now (EnGirlA_CanBuy_DekuSeeds: 0 of 0 bullets).
    s.press(PadState { stick_x: -60, ..NONE });
    s.until(60, |s| s.ossan().state_flag == OSSAN_STATE_BROWSE_LEFT_SHELF && s.ossan().draw_cursor != 0 && s.w.message_state() == TEXT_STATE_EVENT);
    assert_eq!((s.ossan().cursor_index, s.w.game_camera.data2, s.w.msg_ctx.text_id), (4, 30, 0xDF));
    s.buy();
    assert_eq!((s.ossan().state_flag, s.w.msg_ctx.text_id), (OSSAN_STATE_CANT_GET_ITEM, 0x86));
    s.until(60, |s| s.w.message_state() == TEXT_STATE_EVENT);
    s.press(A);
    s.until(30, |s| s.ossan().state_flag == OSSAN_STATE_BROWSE_LEFT_SHELF && s.ossan().draw_cursor != 0);
    // The stick right from the shelf's inner end (slot 4): EnOssan_CursorRight finds nothing
    // lower, so back to the shopkeeper (the camera to 0, 0x83 again).
    s.press(PadState { stick_x: 60, ..NONE });
    assert_eq!(s.ossan().state_flag, OSSAN_STATE_LOOK_SHOPKEEPER);
    s.until(30, |s| s.ossan().state_flag == OSSAN_STATE_FACING_SHOPKEEPER);
    assert_eq!((s.ossan().camera_face_angle, s.w.game_camera.data2), (0.0, 0));
    s.until(30, |s| s.w.message_state() == TEXT_STATE_CHOICE);
    assert_eq!(s.w.msg_ctx.text_id, 0x83);
    // "Talk to the owner": EnOssan_TalkKokiriShopkeeper's 0x10BA.
    s.press(A);
    assert_eq!((s.ossan().state_flag, s.w.msg_ctx.text_id), (OSSAN_STATE_TALKING_TO_SHOPKEEPER, 0x10BA));
}

#[test]
fn start_equips_in_the_play_frame_but_not_with_a_message_up() {
    // KaleidoSetup_Update, only with msgMode MSGMODE_NONE (z_play.c): the stand-in.
    let Some(mut s) = Shop::enter(0, |sv| {
        item_give(sv, None, ITEM_SHIELD_DEKU);
        item_give(sv, None, ITEM_SWORD_KOKIRI);
    }) else {
        return;
    };
    s.talk();
    s.press(PadState { button: BTN_START, ..NONE });
    assert_eq!(s.w.save.cur_equip_value(EQUIP_TYPE_SHIELD), EQUIP_VALUE_SHIELD_NONE, "not while talking");
    s.press(B);
    s.until(80, |s| s.w.msg_ctx.msg_mode == 0 && s.w.player().action == Action::StandingStill);
    s.press(PadState { button: BTN_START, ..NONE });
    assert_eq!(s.w.save.cur_equip_value(EQUIP_TYPE_SHIELD), EQUIP_VALUE_SHIELD_DEKU);
    assert_eq!(s.w.save.cur_equip_value(EQUIP_TYPE_SWORD), EQUIP_VALUE_SWORD_KOKIRI);
    assert_eq!(s.w.save.equips.button_items[0], ITEM_SWORD_KOKIRI);
    assert_eq!(s.w.player().current_shield, 1);
}

#[test]
fn en_girla_can_buy_the_deku_shield() {
    // EnGirlA_CanBuy_DekuShield on a new save's shield row.
    let e = SHOP_ITEM_ENTRIES[SI_DEKU_SHIELD as usize];
    assert_eq!((e.price, e.count, e.get_item_id, e.item_desc_text_id, e.item_buy_prompt_text_id), (40, 1, GI_SHIELD_DEKU, 0x9F, 0x89));
    let g = |price: i16| EnGirlA {
        actor: oot_game::actor::Actor::new(Vec3::ZERO, 0),
        obj_bank_index: None,
        action: oot_actors::en_girla::Action::Update2,
        is_initialized: true,
        item_buy_prompt_text_id: 0,
        get_item_id: 0,
        is_invisible: false,
        drawn: true,
        is_selected: false,
        y_rotation_init: 0,
        y_rotation: 0,
        can_buy_func: Some(e.can_buy),
        item_give_func: e.item_give,
        buy_event_func: e.buy_event,
        base_price: price,
        item_count: 1,
        gi_draw_id: 0,
        hilite: Hilite::None,
    };
    let mut s = SaveContext::new(0, false, 0);
    s.rupees = 39;
    assert_eq!(g(40).can_buy(&s), CANBUY_RESULT_NEED_RUPEES);
    s.rupees = 40;
    assert_eq!(g(40).can_buy(&s), CANBUY_RESULT_SUCCESS_FANFARE);
    item_give(&mut s, None, ITEM_SHIELD_DEKU);
    assert_eq!(g(40).can_buy(&s), CANBUY_RESULT_CANT_GET_NOW);
}
