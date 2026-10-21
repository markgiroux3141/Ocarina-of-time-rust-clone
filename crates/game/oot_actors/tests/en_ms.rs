//! The Magic Bean Salesman (`z_en_ms.c`) in Zora's River (`ootx scene-info --scene spot03`: room
//! 0, params 0xFFFF at (-717, 100, -312), turned -0x4000), against the C.
//!
//! Link (a child, the `zora-diving` preset's save) enters at the river's foot
//! (`ENTR_ZORAS_RIVER_0`) and is placed in front of him. Expected values are worked out from the C
//! in the comments: `sPrices` 10, 20, ... 100 and `sOfferTextIDs` 0x405E.. by `BEANS_BOUGHT`
//! (`AMMO(ITEM_MAGIC_BEAN + 1)`), which `Item_Give(ITEM_MAGIC_BEAN)` raises with the bean count.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_ms::{self, Action, EnMs};
use oot_game::actor_ctx::ActorHandle;
use oot_game::item::{ITEM_MAGIC_BEAN, item_give};
use oot_game::message::{MSGMODE_TEXT_AWAIT_INPUT, TEXT_STATE_AWAITING_NEXT, TEXT_STATE_CHOICE, TEXT_STATE_DONE, TEXT_STATE_DONE_HAS_NEXT, TEXT_STATE_EVENT, TEXT_STATE_NONE};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// A child at the river's foot at 10:00 on the `zora-diving` save with `rupees` and `edit`
/// applied, settled.
fn river(a: &Arc<GameAssets>, rupees: i16, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_ZORAS_RIVER_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("zora-diving").unwrap();
    save.rupees = rupees;
    edit(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    idle(&mut w, 20);
    w
}

fn salesman(w: &PlayState) -> ActorHandle {
    let v: Vec<ActorHandle> = w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnMs>(h).is_some_and(|m| !m.actor.killed)).collect();
    assert_eq!(v.len(), 1, "one bean salesman");
    v[0]
}

fn ms(w: &PlayState, h: ActorHandle) -> &EnMs {
    w.actors.downcast::<EnMs>(h).unwrap()
}

/// Link 50 in front of him, facing him, until he's the actor Player would talk to (`EnMs_Wait`:
/// within 90 and 0x2000 of his facing).
fn offered(w: &mut PlayState, h: ActorHandle) {
    for _ in 0..30 {
        let a = &ms(w, h).actor;
        let (p, yaw) = (a.world_pos, a.shape_rot.y);
        w.place_player(p + Vec3::new(eng_math::sin_s(yaw) * 50.0, 0.0, eng_math::cos_s(yaw) * 50.0), yaw.wrapping_add(i16::MIN));
        idle(w, 1);
        if w.player().target_actor == Some(h) {
            return;
        }
    }
    panic!("the salesman never offered to talk");
}

/// A until the talk starts, then A whenever the box waits (on a choice: `choice` first) until it
/// closes. The texts shown.
fn talk(w: &mut PlayState, choice: u8) -> Vec<u16> {
    let a = PadState { button: BTN_A, ..Default::default() };
    let mut prev = PadState::default();
    let mut n = 0;
    while w.message_state() == TEXT_STATE_NONE {
        let pad = if prev.button == 0 { a } else { PadState::default() };
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        n += 1;
        assert!(n < 40, "the talk never started");
    }
    let mut texts = Vec::new();
    let mut pressed_at = None;
    for _ in 0..3000 {
        let (st, mode) = (w.message_state(), w.msg_ctx.msg_mode);
        if st != TEXT_STATE_NONE && texts.last() != Some(&w.msg_ctx.text_id) {
            texts.push(w.msg_ctx.text_id);
        }
        if st == TEXT_STATE_NONE && mode == 0 {
            return texts;
        }
        if pressed_at.is_some_and(|p| p != (st, mode)) {
            pressed_at = None;
        }
        if st == TEXT_STATE_CHOICE {
            w.msg_ctx.choice_index = choice;
        }
        let waits = matches!(st, TEXT_STATE_AWAITING_NEXT | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_CHOICE | TEXT_STATE_EVENT) || mode == MSGMODE_TEXT_AWAIT_INPUT;
        let pad = if waits && prev.button == 0 && pressed_at.is_none() {
            pressed_at = Some((st, mode));
            a
        } else {
            PadState::default()
        };
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
    }
    panic!("the talk didn't end: {texts:x?}");
}

/// Up to `n` frames for a box to open on its own (the get-item's), then A through it.
fn next_box(w: &mut PlayState, n: usize) -> Vec<u16> {
    for _ in 0..n {
        if w.message_state() != TEXT_STATE_NONE {
            return talk(w, 0);
        }
        idle(w, 1);
    }
    Vec::new()
}

fn beans(w: &PlayState) -> (u8, i8, i8) {
    (w.save.inv_content(ITEM_MAGIC_BEAN), w.save.ammo(ITEM_MAGIC_BEAN), en_ms::beans_bought(w))
}

#[test]
fn he_sets_up_by_the_fence() {
    let Some(a) = assets() else { return };
    let w = river(&a, 0, |_| {});
    let h = salesman(&w);
    let m = ms(&w, h);
    // EnMs_Init: ATTENTION_RANGE_2, lockOnArrowOffset 500, scale 0.015, immovable, gravity -1;
    // EnMs_SetOfferText with none bought: sOfferTextIDs[0], 0x405E; EnMs_Wait; eating, looped.
    assert_eq!((m.actor.target_mode, m.actor.target_arrow_offset, m.actor.scale), (2, 500.0, Vec3::splat(0.015)));
    assert_eq!((m.actor.col_chk_info.mass, m.actor.gravity), (oot_game::collision_check::MASS_IMMOVABLE, -1.0));
    assert_eq!((m.actor.text_id, m.action), (0x405E, Action::Wait));
    assert!(m.skel_anime.is("gBeanSalesmanEatingAnim"));
    // Nothing moves him outside the ending (no Actor_MoveXZGravity): still where he's placed.
    assert_eq!(m.actor.world_pos, Vec3::new(-717.0, 100.0, -312.0));
    assert_eq!((m.collider.dim.radius, m.collider.dim.height), (22, 37));
}

#[test]
fn the_first_bean_costs_10() {
    let Some(a) = assets() else { return };
    let mut w = river(&a, 50, |_| {});
    let h = salesman(&w);
    offered(&mut w, h);
    // "Yes" with 50: Actor_OfferGetItem(GI_MAGIC_BEAN, 90, 10) (EnMs_Sell).
    assert_eq!(talk(&mut w, 0), vec![0x405E]);
    assert_eq!(ms(&w, h).action, Action::Sell);
    // Player takes it (his parent): the 10 charged at once (sPrices[0], before the get's
    // Item_Give); the get's text, then 0x406C (EnMs_TalkAfterPurchase), to its end: EnMs_Wait.
    let texts = next_box(&mut w, 120);
    assert_eq!(texts.last(), Some(&0x406C), "{texts:x?}");
    assert_eq!(ms(&w, h).action, Action::Wait);
    // Item_Give(ITEM_MAGIC_BEAN), the first: the slot, one bean, BEANS_BOUGHT 1.
    assert_eq!(beans(&w), (ITEM_MAGIC_BEAN, 1, 1));
    assert_eq!(w.save.rupees + w.save.rupee_accumulator, 40);
    // The next offer: sOfferTextIDs[1], 0x405F (20 rupees).
    idle(&mut w, 1);
    assert_eq!(ms(&w, h).actor.text_id, 0x405F);
}

#[test]
fn the_second_bean_costs_20() {
    let Some(a) = assets() else { return };
    let mut w = river(&a, 25, |s| {
        item_give(s, None, ITEM_MAGIC_BEAN);
    });
    assert_eq!(beans(&w), (ITEM_MAGIC_BEAN, 1, 1));
    let h = salesman(&w);
    assert_eq!(ms(&w, h).actor.text_id, 0x405F);
    offered(&mut w, h);
    assert_eq!(talk(&mut w, 0), vec![0x405F]);
    let texts = next_box(&mut w, 120);
    assert_eq!(texts.last(), Some(&0x406C), "{texts:x?}");
    // 25 - sPrices[1] (20) = 5; two beans, BEANS_BOUGHT 2: the offer 0x4060 (30 rupees).
    assert_eq!(beans(&w), (ITEM_MAGIC_BEAN, 2, 2));
    assert_eq!(w.save.rupees + w.save.rupee_accumulator, 5);
    idle(&mut w, 1);
    assert_eq!(ms(&w, h).actor.text_id, 0x4060);
}

#[test]
fn under_the_price_or_no_sells_nothing() {
    let Some(a) = assets() else { return };
    // One bought: 20 asked; 19 rupees: 0x4069, nothing given.
    let mut w = river(&a, 19, |s| {
        item_give(s, None, ITEM_MAGIC_BEAN);
    });
    let h = salesman(&w);
    offered(&mut w, h);
    assert_eq!(talk(&mut w, 0), vec![0x405F, 0x4069]);
    assert_eq!(ms(&w, h).action, Action::Wait);
    assert_eq!((beans(&w), w.save.rupees), ((ITEM_MAGIC_BEAN, 1, 1), 19));
    // "No": 0x4068.
    offered(&mut w, h);
    assert_eq!(talk(&mut w, 1), vec![0x405F, 0x4068]);
    assert_eq!((beans(&w), w.save.rupees), ((ITEM_MAGIC_BEAN, 1, 1), 19));
}

#[test]
fn ten_bought_is_sold_out() {
    let Some(a) = assets() else { return };
    // BEANS_BOUGHT >= 10: 0x406B, whatever the rupees.
    let w = river(&a, 500, |s| {
        for _ in 0..10 {
            item_give(s, None, ITEM_MAGIC_BEAN);
        }
    });
    assert_eq!(en_ms::beans_bought(&w), 10);
    let h = salesman(&w);
    assert_eq!(ms(&w, h).actor.text_id, 0x406B);
}

#[test]
fn the_prices_and_offers() {
    // sPrices and sOfferTextIDs.
    assert_eq!(en_ms::PRICES, [10, 20, 30, 40, 50, 60, 70, 80, 90, 100]);
    assert_eq!(en_ms::OFFER_TEXT_IDS[0], 0x405E);
    assert_eq!(en_ms::OFFER_TEXT_IDS[9], 0x4067);
}
