//! The Zora shop (`zoora_scene`, `ootx scene-info --scene zoora`): `En_Ossan` params 7
//! (`OSSAN_TYPE_ZORA`) at (0, 0, -180), the `En_Tana` shelves at (0, 0, -140), on a child's
//! `zora-diving` save (50 rupees) entering by `ENTR_ZORA_SHOP_0`.
//!
//! Expected values from the C:
//! - `EnOssan_InitActionFunc`: `sShopkeeperPositionOffsets[7]` (0, 0, 36) added: (0, 0, -144);
//!   `sShopkeeperScale[7]` 0.01; text 0x9E; `EnOssan_InitZoraShopkeeper`: `gZoraSkel` playing
//!   `gZoraShopkeeperAnim` looped;
//! - `EnOssan_SpawnItemsOnShelves`: `sShopkeeperStores[7]`'s items at their offsets from the
//!   shelves, with `sShopItemEntries`' prices and texts;
//! - "Talk to the owner": `EnOssan_TalkZoraShopkeeper`, a child's 0x403A;
//! - the stick left: slot 4, the Deku nuts (5 for 15); owned, a quick buy (`Item_Give`, 15
//!   rupees, 0x84); the stick right: slot 0, the Zora Tunic, which a child can't get now (0x86).

mod common;

use std::sync::Arc;

use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_girla::*;
use oot_actors::en_ossan::*;
use oot_actors::en_tana::EnTana;
use oot_game::actor_ctx::ActorHandle;
use oot_game::item::*;
use oot_game::message::{TEXT_STATE_CHOICE, TEXT_STATE_EVENT};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

const A: PadState = PadState { button: BTN_A, stick_x: 0, stick_y: 0 };
const NONE: PadState = PadState { button: 0, stick_x: 0, stick_y: 0 };
const LEFT: PadState = PadState { button: 0, stick_x: -60, stick_y: 0 };
const RIGHT: PadState = PadState { button: 0, stick_x: 60, stick_y: 0 };

struct Shop {
    w: PlayState,
    prev: PadState,
}

impl Shop {
    /// The shop on the `zora-diving` save, changed by `f`, settled.
    fn enter(f: impl FnOnce(&mut SaveContext)) -> Option<Shop> {
        let a = assets()?;
        let e = a.scenes.entrance_index("ENTR_ZORA_SHOP_0").unwrap();
        let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
        save.apply_preset("zora-diving").unwrap();
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

    fn press(&mut self, pad: PadState) {
        self.frames(pad, 1);
        self.frames(NONE, 1);
    }

    fn handle(&self) -> ActorHandle {
        self.w.actors.all().into_iter().find(|&h| self.w.actors.downcast::<EnOssan>(h).is_some()).expect("En_Ossan, not a placeholder")
    }

    fn ossan(&self) -> &EnOssan {
        self.w.actors.downcast::<EnOssan>(self.handle()).unwrap()
    }

    fn item(&self, h: ActorHandle) -> &EnGirlA {
        self.w.actors.downcast::<EnGirlA>(h).expect("En_GirlA")
    }

    fn until(&mut self, n: usize, done: impl Fn(&Shop) -> bool) {
        for _ in 0..n {
            if done(self) {
                return;
            }
            self.frames(NONE, 1);
        }
        panic!("not done: shopkeeper state {}, text {:#x}, message state {}", self.ossan().state_flag, self.w.msg_ctx.text_id, self.w.message_state());
    }

    /// In front of the counter (54 from the shopkeeper, facing him): talk, A at "Welcome!", on
    /// to 0x83's choice.
    fn talk(&mut self) {
        self.w.place_player(Vec3::new(0.0, 0.0, -90.0), i16::MIN);
        self.frames(NONE, 2);
        let h = self.handle();
        assert_eq!(self.w.player().target_actor, Some(h), "the shopkeeper offers to talk within 100");
        self.press(A);
        self.until(60, |s| s.ossan().state_flag == OSSAN_STATE_START_CONVERSATION && s.w.message_state() == TEXT_STATE_EVENT);
        assert_eq!(self.w.msg_ctx.text_id, 0x9E);
        self.press(A);
        self.until(60, |s| s.ossan().state_flag == OSSAN_STATE_FACING_SHOPKEEPER && s.w.message_state() == TEXT_STATE_CHOICE);
        assert_eq!(self.w.msg_ctx.text_id, 0x83);
    }

    /// The stick to a shelf, until its first item's description is up.
    fn browse(&mut self, pad: PadState, state: u8) {
        self.press(pad);
        self.until(60, |s| s.ossan().state_flag == state && s.ossan().draw_cursor != 0 && s.w.message_state() == TEXT_STATE_EVENT);
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
fn the_zora_shopkeeper_sets_up_his_shelves() {
    let Some(s) = Shop::enter(|_| {}) else { return };
    let w = &s.w;
    let o = s.ossan();
    assert_eq!(o.actor.params, OSSAN_TYPE_ZORA);
    assert_eq!(o.action, oot_actors::en_ossan::Action::Main);
    assert_eq!((o.actor.world_pos, o.actor.scale), (Vec3::new(0.0, 0.0, -144.0), Vec3::splat(0.01)));
    assert_eq!(o.actor.text_id, 0x9E);
    let skel = o.skel.as_ref().expect("his skeleton");
    assert!(skel.is("gZoraShopkeeperAnim"));
    assert_eq!(o.skeleton.as_ref().unwrap().limbs.len(), 19, "gZoraSkel: 19 limbs, ZORA_LIMB_MAX 20 joints");
    let shelves = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<EnTana>(h)).expect("En_Tana");
    assert_eq!(shelves.actor.world_pos, Vec3::new(0.0, 0.0, -140.0));
    // sShopkeeperStores[OSSAN_TYPE_ZORA], with each item's sShopItemEntries row (price,
    // description, buy prompt).
    let store = [
        (SI_ZORA_TUNIC, [50, 52, -20], 300, 0xAB, 0x94),
        (SI_ARROWS_10, [50, 76, -20], 20, 0xA0, 0x8A),
        (SI_RECOVERY_HEART, [80, 52, -3], 10, 0xAC, 0x95),
        (SI_ARROWS_30, [80, 76, -3], 60, 0xC1, 0x9B),
        (SI_DEKU_NUTS_5, [-50, 52, -20], 15, 0xB2, 0x7F),
        (SI_ARROWS_50, [-50, 76, -20], 90, 0xB0, 0x7D),
        (SI_FISH, [-80, 52, -3], 200, 0xB3, 0x7E),
        (SI_RED_POTION_R50, [-80, 76, -3], 50, 0x65, 0x63),
    ];
    for (i, &(si, off, price, desc, prompt)) in store.iter().enumerate() {
        let h = o.shelf_slots[i].expect("every slot filled");
        let g = s.item(h);
        assert_eq!(g.actor.params, si, "slot {i}");
        assert_eq!(g.actor.world_pos, Vec3::new(off[0] as f32, off[1] as f32, off[2] as f32 - 140.0), "slot {i}");
        assert_eq!(g.actor.shape_rot.y as u16, if i < 4 { 0xEAAC } else { 0x1554 });
        assert_eq!(g.action, oot_actors::en_girla::Action::Update2, "slot {i}: its object loaded");
        assert_eq!((g.base_price, g.actor.text_id, g.item_buy_prompt_text_id), (price, desc, prompt), "slot {i}");
        assert!(g.drawn && !g.is_invisible);
    }
    // GI_TUNIC_ZORA (item.h: 0x2D), drawn as GID_TUNIC_ZORA (0x3C).
    let tunic = s.item(o.shelf_slots[0].unwrap());
    assert_eq!((tunic.get_item_id, tunic.gi_draw_id), (0x2D, 0x3C));
}

#[test]
fn talking_to_the_zora_shopkeeper() {
    let Some(mut s) = Shop::enter(|_| {}) else { return };
    s.talk();
    // "Talk to the owner" (choice 0): EnOssan_TalkZoraShopkeeper, a child: 0x403A.
    s.press(A);
    assert_eq!((s.ossan().state_flag, s.w.msg_ctx.text_id), (OSSAN_STATE_TALKING_TO_SHOPKEEPER, 0x403A));
}

#[test]
fn a_quick_buy_of_deku_nuts() {
    // Nuts owned (5 of 20): SI_DEKU_NUTS_5 is a quick buy: Item_Give(ITEM_DEKU_NUTS_5), 15
    // rupees, "Thanks a lot!" (0x84).
    let Some(mut s) = Shop::enter(|sv| {
        item_give(sv, None, ITEM_DEKU_NUTS_5);
        sv.inventory.ammo[slot(ITEM_DEKU_NUT)] = 5;
    }) else {
        return;
    };
    assert_eq!(s.w.save.rupees, 50);
    s.talk();
    // The stick left: the left shelf, the cursor on slot 4 (its bottom row's inner item).
    s.browse(LEFT, OSSAN_STATE_BROWSE_LEFT_SHELF);
    assert_eq!((s.ossan().cursor_index, s.w.msg_ctx.text_id), (4, 0xB2));
    s.buy();
    assert_eq!((s.ossan().state_flag, s.w.msg_ctx.text_id), (OSSAN_STATE_QUICK_BUY, 0x84));
    assert_eq!(s.w.save.ammo(ITEM_DEKU_NUT), 10);
    assert_eq!(s.w.save.rupees + s.w.save.rupee_accumulator, 35);
}

#[test]
fn a_child_cant_get_the_zora_tunic() {
    let Some(mut s) = Shop::enter(|sv| sv.rupees = 300) else { return };
    s.talk();
    // The stick right: slot 0, the tunic (0xAB); EnGirlA_CanBuy_ZoraTunic: a child,
    // CANBUY_RESULT_CANT_GET_NOW: 0x86.
    s.browse(RIGHT, OSSAN_STATE_BROWSE_RIGHT_SHELF);
    assert_eq!((s.ossan().cursor_index, s.w.msg_ctx.text_id), (0, 0xAB));
    s.buy();
    assert_eq!((s.ossan().state_flag, s.w.msg_ctx.text_id), (OSSAN_STATE_CANT_GET_ITEM, 0x86));
    assert_eq!(s.w.save.rupees, 300);
}
