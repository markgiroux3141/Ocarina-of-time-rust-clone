//! GAME-10 milestone 3 against the C: the lakeside professor (`z_en_mk.c`). His talk
//! (`EnMk_Wait`), the dive counted in his tank (`EnMk_Update`'s `swimFlag` from Link's
//! `depthInWater`), its texts (0x406C plus the count), and at the deepest the piece of heart
//! (0x4074, `GI_HEART_PIECE`, `ITEMGETINF_10`).
//!
//! Entered from `fairy-magic` (the phase's start). Expected values are worked out from the C in the
//! comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use eng_math::{cos_s, sin_s};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_mk::{self, Action, EnMk};
use oot_game::actor_ctx::ActorHandle;
use oot_game::message::{MSGMODE_TEXT_AWAIT_INPUT, TEXT_STATE_AWAITING_NEXT, TEXT_STATE_CHOICE, TEXT_STATE_DONE, TEXT_STATE_DONE_HAS_NEXT, TEXT_STATE_EVENT, TEXT_STATE_NONE};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

/// `ITEM_SCALE_SILVER`, `ITEM_SCALE_GOLDEN` (`item.h`: 0x53, 0x54).
const ITEM_SCALE_SILVER: u8 = 0x53;
const ITEM_SCALE_GOLDEN: u8 = 0x54;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// The lab (`ENTR_LAKESIDE_LABORATORY_0`) at 10:00 from `fairy-magic`, `tweak` last; eight frames
/// on.
fn lab(a: &Arc<GameAssets>, tweak: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_LAKESIDE_LABORATORY_0").unwrap();
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("fairy-magic").unwrap();
    tweak(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap();
    idle(&mut w, 8);
    w
}

fn step(w: &mut PlayState, prev: &mut PadState, pad: PadState) {
    w.tick_with(scripted_input(*prev, pad));
    *prev = pad;
}

fn idle(w: &mut PlayState, n: u32) {
    let mut prev = PadState::default();
    for _ in 0..n {
        step(w, &mut prev, PadState::default());
    }
}

fn professor(w: &PlayState) -> (ActorHandle, &EnMk) {
    let h = w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnMk>(h).is_some()).expect("the professor");
    (h, w.actors.downcast::<EnMk>(h).unwrap())
}

/// Link 60 in front of the professor, facing him.
fn before_him(w: &mut PlayState) {
    let (_, mk) = professor(w);
    let (pos, yaw) = (mk.actor.world_pos, mk.actor.shape_rot.y);
    let at = pos + Vec3::new(sin_s(yaw) * 60.0, 0.0, cos_s(yaw) * 60.0);
    w.place_player(at, yaw.wrapping_add(i16::MIN));
    idle(w, 3);
}

/// A until a talk starts, then A whenever a box waits, until no box is open for 60 frames: every
/// text shown (the get-item text after a gift too, which comes once Link holds it up).
fn talk(w: &mut PlayState) -> Vec<u16> {
    let a = PadState { button: BTN_A, ..Default::default() };
    let mut prev = PadState::default();
    let mut n = 0;
    while w.message_state() == TEXT_STATE_NONE {
        let pad = if prev.button == 0 { a } else { PadState::default() };
        step(w, &mut prev, pad);
        n += 1;
        assert!(n < 40, "the talk never started");
    }
    let mut texts = Vec::new();
    let mut pressed_at = None;
    let mut quiet = 0;
    for _ in 0..3000 {
        let (st, mode) = (w.message_state(), w.msg_ctx.msg_mode);
        if st != TEXT_STATE_NONE && texts.last() != Some(&w.msg_ctx.text_id) {
            texts.push(w.msg_ctx.text_id);
        }
        if st == TEXT_STATE_NONE && mode == 0 {
            quiet += 1;
            if quiet > 60 {
                return texts;
            }
        } else {
            quiet = 0;
        }
        if pressed_at.is_some_and(|p| p != (st, mode)) {
            pressed_at = None;
        }
        let waits = matches!(st, TEXT_STATE_AWAITING_NEXT | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_CHOICE | TEXT_STATE_EVENT) || mode == MSGMODE_TEXT_AWAIT_INPUT;
        let pad = if waits && prev.button == 0 && pressed_at.is_none() {
            pressed_at = Some((st, mode));
            a
        } else {
            PadState::default()
        };
        step(w, &mut prev, pad);
    }
    panic!("the talk didn't end: {texts:x?}");
}

/// The lab's tank: its deepest water box's middle and surface.
fn tank(w: &PlayState) -> (Vec3, f32) {
    let boxes = &w.col.header.water_boxes;
    eprintln!("water boxes {boxes:?}");
    let b = boxes.iter().max_by_key(|b| b.x_length as i32 * b.z_length as i32).expect("a water box");
    let mid = Vec3::new(b.x_min as f32 + b.x_length as f32 / 2.0, b.y_surface as f32, b.z_min as f32 + b.z_length as f32 / 2.0);
    (mid, b.y_surface as f32)
}

/// Link dropped at the tank's surface, then A held for `frames`: the deepest he went.
fn dive(w: &mut PlayState, frames: u32) -> f32 {
    let (mid, _) = tank(w);
    w.place_player(mid, 0);
    // In, under and up again: treading (Player_Action_8084D610) before the dive's A.
    let mut n = 0;
    while n < 40 || w.player().action.decomp_name() != "Player_Action_8084D610" {
        idle(w, 1);
        n += 1;
        assert!(n < 300, "never treading: {}", w.player().action.decomp_name());
    }
    let mut prev = PadState::default();
    let a = PadState { button: BTN_A, ..Default::default() };
    let mut deepest = 0.0f32;
    let mut actions: Vec<&'static str> = Vec::new();
    for _ in 0..frames {
        step(w, &mut prev, a);
        deepest = deepest.max(w.player().actor.y_dist_to_water);
        let s = w.player().action.decomp_name();
        if actions.last() != Some(&s) {
            actions.push(s);
        }
    }
    eprintln!("dive: {actions:?} at {:?}", w.player().actor.world_pos);
    idle(w, 120);
    deepest
}

#[test]
fn the_professor_talks_of_the_lake() {
    let Some(a) = assets() else { return };
    let mut w = lab(&a, |_| {});
    let (_, mk) = professor(&w);
    // EnMk_Init: scale 0.01, gravity -1, waiting, nothing dived.
    assert_eq!((mk.actor.scale, mk.action, mk.swim_flag, mk.flags), (Vec3::splat(0.01), Action::Wait, 0, 0));
    before_him(&mut w);
    // Nothing shown, nothing dived: 0x4018.
    assert_eq!(talk(&mut w), vec![0x4018]);
    assert_eq!(professor(&w).1.action, Action::Wait);
}

#[test]
fn the_tank_counts_how_deep_link_dives() {
    let Some(a) = assets() else { return };
    let mut w = lab(&a, |s| {
        oot_game::item::item_give(s, None, ITEM_SCALE_SILVER);
    });
    let deepest = dive(&mut w, 200);
    let mk = professor(&w).1;
    eprintln!("deepest {deepest}, swimFlag {}", mk.swim_flag);
    // The silver scale's dive turns at 240 (D_80854784[1]): swimFlag = (s16)(240 × 0.025) = 6,
    // no chime (under 8).
    assert!((240.0..250.0).contains(&deepest), "{deepest}");
    assert_eq!(mk.swim_flag, 6);
    assert_eq!(mk.flags & en_mk::MK_FLAG_CHIMED, 0);
    before_him(&mut w);
    // 0x406C + 6.
    assert_eq!(talk(&mut w), vec![0x4072]);
}

#[test]
fn the_deepest_dive_earns_a_piece_of_heart() {
    let Some(a) = assets() else { return };
    let mut w = lab(&a, |s| {
        oot_game::item::item_give(s, None, ITEM_SCALE_GOLDEN);
    });
    let (pieces_before, capacity_before) = (w.save.inventory.quest_items >> 28, w.save.health_capacity);
    let deepest = dive(&mut w, 300);
    let mk = professor(&w).1;
    eprintln!("deepest {deepest}, swimFlag {}", mk.swim_flag);
    // The golden scale's dive turns at 360 (D_80854784[2]): 355 or deeper, swimFlag 8 and the chime
    // (flags 4).
    assert!(deepest >= 355.0, "{deepest}");
    assert_eq!(mk.swim_flag, 8);
    assert_ne!(mk.flags & en_mk::MK_FLAG_CHIMED, 0);
    before_him(&mut w);
    // 0x4074; as it closes the piece of heart is offered from 10000 across (func_80AAD014) and
    // taken (func_80AACFA0: ITEMGETINF_10), with its get-item text; swimFlag back to 0.
    let texts = talk(&mut w);
    eprintln!("texts {texts:x?}");
    // The piece of heart's own text (sGetItemTable's GI_HEART_PIECE: 0xC4).
    assert_eq!(texts, vec![0x4074, 0xC4]);
    assert!(w.save.get_item_get_inf(en_mk::ITEMGETINF_10));
    // Item_Give(ITEM_HEART_PIECE): one more piece (or, the fourth, a container).
    assert!(w.save.inventory.quest_items >> 28 != pieces_before || w.save.health_capacity > capacity_before);
    let mk = professor(&w).1;
    assert_eq!((mk.swim_flag, mk.action), (0, Action::Wait));
    // Talked to again, nothing dived since: 0x4018.
    idle(&mut w, 10);
    before_him(&mut w);
    assert_eq!(talk(&mut w), vec![0x4018]);
}
