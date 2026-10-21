//! GAME-10 milestone 3 against the C: `Item_Etcetera` (`z_item_etcetera.c`). Ruto's letter on Lake
//! Hylia's bottom (its init's kill, its offer and bubbles, and the dive that takes it: Player's
//! `Player_ActionHandler_2` at the bottom and `Player_Action_8084E1EC` surfacing with it held up),
//! the fire arrows falling outside a cutscene, and the treasure chest game's items gone with their
//! chest.
//!
//! Entered from `fairy-magic` (the phase's start) with the lake's intro seen (`EVENTCHKINF_B1`).
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::item_etcetera::{self, Action, DrawFn, ItemEtcetera};
use oot_game::actor_ctx::ActorHandle;
use oot_game::item::{GI_ARROW_FIRE, GI_BOTTLE_RUTOS_LETTER, ITEM_BOTTLE_RUTOS_LETTER, SLOT_BOTTLE_1};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

/// `EVENTCHKINF_B1` (`save.h`): Lake Hylia's intro (`gLakeHyliaIntroCs`) seen.
const EVENTCHKINF_B1: u16 = 0xB1;
/// `ITEM_SCALE_SILVER` (`item.h`: 0x53).
const ITEM_SCALE_SILVER: u8 = 0x53;
/// Ruto's letter's placement (`spot06` room 0: `Item_Etcetera` 0x0001).
const LETTER_POS: Vec3 = Vec3::new(-916.0, -1580.0, 4297.0);

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Lake Hylia (`ENTR_LAKE_HYLIA_0`) as a child or an adult at 10:00 from `fairy-magic`, its intro
/// seen, `tweak` applied last.
fn lake(a: &Arc<GameAssets>, adult: bool, tweak: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_LAKE_HYLIA_0").unwrap();
    let mut save = SaveContext::new(e, adult, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("fairy-magic").unwrap();
    save.adult = adult;
    save.set_event_chk_inf(EVENTCHKINF_B1);
    tweak(&mut save);
    oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap()
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

fn action(w: &PlayState) -> &'static str {
    w.player().action.decomp_name()
}

/// Every `Item_Etcetera` in play (killed or not, until deleted).
fn items(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<ItemEtcetera>(h).is_some()).collect()
}

fn item(w: &PlayState, h: ActorHandle) -> &ItemEtcetera {
    w.actors.downcast::<ItemEtcetera>(h).unwrap()
}

#[test]
fn ruto_s_letter_lies_on_the_lake_s_bottom() {
    let Some(a) = assets() else { return };
    let mut w = lake(&a, false, |_| {});
    idle(&mut w, 8);
    let v = items(&w);
    assert_eq!(v.len(), 1);
    let l = item(&w, v[0]);
    // ITEM_ETC_LETTER: scale 0.5, its object (object_gi_bottle_letter, in the room's list) in, so
    // drawn and on to func_80B858B4, offering GI_BOTTLE_RUTOS_LETTER.
    assert_eq!(l.actor.world_pos, LETTER_POS);
    assert_eq!(l.actor.scale, Vec3::splat(0.5));
    assert_eq!((l.action, l.draw, l.get_item_id), (Action::OfferLetter, Some(DrawFn::Draw), GI_BOTTLE_RUTOS_LETTER));
    assert!(!l.actor.killed);
    // Its bubbles: EffectSsBubble_Spawn on the frames where gameplayFrames & 0xD is 0.
    let bubbles = |w: &PlayState| w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == oot_game::effect::EFFECT_SS_BUBBLE && (e.pos.x - LETTER_POS.x).abs() < 30.0).count();
    let mut seen = 0;
    for _ in 0..32 {
        let before = bubbles(&w);
        idle(&mut w, 1);
        if bubbles(&w) > before {
            seen += 1;
        }
    }
    // Two of every 16 frames (gameplayFrames 0, 2, 16, 18, ... mod 16): four in any 32.
    assert_eq!(seen, 4);

    // Taken once (EVENTCHKINF_31): gone at init.
    let mut w = lake(&a, false, |s| s.set_event_chk_inf(item_etcetera::EVENTCHKINF_31));
    idle(&mut w, 8);
    assert!(items(&w).iter().all(|&h| item(&w, h).actor.killed) || items(&w).is_empty());
}

/// Link at the surface over the letter, A held: the dive (with or without the silver scale).
/// Returns the play state after `frames`, the frame the letter was taken (if it was), and the
/// actions Player went through.
fn dive_for_the_letter(a: &Arc<GameAssets>, scale: bool, frames: u32) -> (PlayState, Option<u32>, Vec<&'static str>) {
    let mut w = lake(a, false, |s| {
        if scale {
            oot_game::item::item_give(s, None, ITEM_SCALE_SILVER);
        }
    });
    idle(&mut w, 20);
    // Treading at WATER_LEVEL_RAISED (-1313), 267 above the letter, facing it from 14 short.
    w.place_player(Vec3::new(LETTER_POS.x, -1313.0, LETTER_POS.z - 14.0), 0);
    idle(&mut w, 60);
    assert!(w.player().state1 & oot_actors::player::STATE1_27 != 0, "swimming: {}", action(&w));
    let letter = items(&w)[0];
    let mut prev = PadState::default();
    let mut a_pad = PadState::default();
    a_pad.button |= BTN_A;
    let mut taken = None;
    let mut actions: Vec<&'static str> = Vec::new();
    for f in 0..frames {
        step(&mut w, &mut prev, a_pad);
        if taken.is_none() && w.actors.actor(letter).is_none_or(|l| l.killed) {
            taken = Some(f);
        }
        let s = action(&w);
        if actions.last() != Some(&s) {
            actions.push(s);
        }
    }
    (w, taken, actions)
}

#[test]
fn a_dive_with_the_silver_scale_takes_ruto_s_letter() {
    let Some(a) = assets() else { return };
    let (mut w, taken, actions) = dive_for_the_letter(&a, true, 400);
    eprintln!("taken at {taken:?}: {actions:?}");
    // The dive turns at D_80854784[1] = 240 deep (y -1553): 27 above the letter, within the offer's
    // 50 up or down and 30 across (func_80B858B4's Actor_OfferGetItem). Player_ActionHandler_2 in
    // Player_Action_8084DC48 takes it (its parent set: func_80B858B4 sets EVENTCHKINF_31 and switch
    // 0xB and kills it) and he surfaces holding it up: Player_Action_8084E1EC with PLAYER_STATE1_10,
    // whose func_8084DFF4 shows its text (sGetItemTable's 0x99) and Item_Give's it.
    assert!(taken.is_some(), "never taken: {actions:?}");
    assert!(w.save.get_event_chk_inf(item_etcetera::EVENTCHKINF_31));
    assert!(w.flags.get_switch(item_etcetera::LETTER_SWITCH_FLAG));
    assert!(actions.contains(&"Player_Action_8084E1EC"), "{actions:?}");
    // Held up at the surface until its text closes.
    let mut texts = Vec::new();
    let mut held_up = false;
    let mut prev = PadState::default();
    let a_pad = PadState { button: BTN_A, ..Default::default() };
    for f in 0..600 {
        let p = w.player();
        // func_8083D12C surfacing: PLAYER_STATE1_10 | _CARRYING_ACTOR | _29 and the letter held up
        // (link_swimer_swim_get, not swim_deep_end).
        if action(&w) == "Player_Action_8084E1EC" && p.state1 & oot_actors::player::STATE1_10 != 0 {
            assert!(p.state1 & oot_actors::player::STATE1_11 != 0, "carrying");
            if p.skel.animation == w.data.anim("link_swimer_swim_get") {
                held_up = true;
            }
        }
        if w.message_state() != oot_game::message::TEXT_STATE_NONE && texts.last() != Some(&w.msg_ctx.text_id) {
            texts.push(w.msg_ctx.text_id);
        }
        // A now and then to close the box once it waits.
        let pad = if f % 20 == 19 { a_pad } else { PadState::default() };
        step(&mut w, &mut prev, pad);
        if !texts.is_empty() && w.message_state() == oot_game::message::TEXT_STATE_NONE && action(&w) != "Player_Action_8084E1EC" {
            break;
        }
    }
    eprintln!("texts {texts:x?}, now {}", action(&w));
    assert!(held_up, "never held it up");
    assert_eq!(texts.first(), Some(&0x99));
    // Item_Give(ITEM_BOTTLE_RUTOS_LETTER): the first empty bottle slot.
    let bottles = &w.save.inventory.items[SLOT_BOTTLE_1..SLOT_BOTTLE_1 + 4];
    assert!(bottles.contains(&ITEM_BOTTLE_RUTOS_LETTER), "{bottles:x?}");
}

#[test]
fn without_the_scale_the_dive_falls_short() {
    let Some(a) = assets() else { return };
    // D_80854784[0] = 120: the dive turns at y -1433, 147 above the letter, out of the offer's 50.
    let (w, taken, actions) = dive_for_the_letter(&a, false, 300);
    assert_eq!(taken, None, "{actions:?}");
    assert!(!w.save.get_event_chk_inf(item_etcetera::EVENTCHKINF_31));
}

#[test]
fn the_fire_arrows_fall_outside_a_cutscene() {
    let Some(a) = assets() else { return };
    // An adult's lake (layer 2) loads object_gi_m_arrow; Shot_Sun's spawn without its cutscene.
    let mut w = lake(&a, true, |_| {});
    idle(&mut w, 8);
    let pos = Vec3::new(700.0, -800.0, 7261.0);
    let h = w.actor_spawn(item_etcetera::ACTOR_ITEM_ETCETERA, pos, [0, 0, 0], item_etcetera::ITEM_ETC_ARROW_FIRE).unwrap();
    let f = item(&w, h);
    // ITEM_ETC_ARROW_FIRE: scale 0.5, shape.yOffset 50, no draw yet.
    assert_eq!((f.actor.scale, f.actor.shape_y_offset, f.draw, f.action), (Vec3::splat(0.5), 50.0, None, Action::WaitForObject));
    idle(&mut w, 1);
    // The object in: drawn (drawFunc), on to ItemEtcetera_UpdateFireArrow.
    assert_eq!((item(&w, h).draw, item(&w, h).action), (Some(DrawFn::Draw), Action::UpdateFireArrow));
    idle(&mut w, 1);
    // No cutscene: gravity -0.1, down to -4, falling from the next frame.
    let f = item(&w, h);
    assert_eq!((f.action, f.actor.gravity, f.actor.min_velocity_y), (Action::MoveFireArrowDown, -0.1, -4.0));
    let (y0, rot0) = (f.actor.world_pos.y, f.actor.shape_rot.y);
    idle(&mut w, 10);
    let f = item(&w, h);
    // Actor_MoveXZGravity: velocity.y -0.1 a frame (10 frames: -1.0), 0x400 a frame of spin.
    assert!((f.actor.velocity.y + 1.0).abs() < 1e-4, "{}", f.actor.velocity.y);
    assert!(f.actor.world_pos.y < y0);
    assert_eq!(f.actor.shape_rot.y, rot0.wrapping_add(10 * 0x400));
    assert_eq!(f.get_item_id, GI_ARROW_FIRE);
}

#[test]
fn the_chest_game_s_items_go_with_their_chest() {
    let Some(a) = assets() else { return };
    let mut w = lake(&a, false, |_| {});
    idle(&mut w, 8);
    // En_Changer's spawn of a green rupee over chest 3: ((3 & 0x1F) << 8) + type.
    w.object_ctx.spawn(item_etcetera::OBJECT_GI_RUPY);
    let pos = Vec3::new(-1000.0, -1200.0, 3000.0);
    let h = w.actor_spawn(item_etcetera::ACTOR_ITEM_ETCETERA, pos, [0, 0, 0], 0x0300 | item_etcetera::ITEM_ETC_RUPEE_GREEN_CHEST_GAME).unwrap();
    let r = item(&w, h);
    // Scale 0.5, 15 higher, offered nothing, drawn only through the lens.
    assert_eq!((r.actor.scale, r.actor.world_pos.y, r.draw_func), (Vec3::splat(0.5), pos.y + 15.0, DrawFn::DrawThroughLens));
    idle(&mut w, 2);
    assert_eq!(item(&w, h).action, Action::ChestGame);
    assert!(!item(&w, h).actor.killed);
    w.flags.set_treasure(3);
    idle(&mut w, 1);
    assert!(w.actors.actor(h).is_none_or(|r| r.killed));
}
