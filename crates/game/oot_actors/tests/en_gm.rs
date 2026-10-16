//! GAME-09 milestone 2 against the C: Medigoron (`z_en_gm.c`) in Goron City's room 2 (`ootx
//! scene-info --scene spot18`: params 0x0006 at (-951, 400, 1317), turned 14746; behind
//! `Bg_Breakwall` 0x2005 and 0xA006, whose flag 6 is his).
//!
//! Link enters at the bottom (`ENTR_GORON_CITY_2`, room 3) and room 2 is loaded as the debug
//! starts load rooms; he's placed past the walls. Expected values are worked out from the C in the
//! comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_gm::{self, Action, EnGm, UpdateFunc};
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::ActorHandle;
use oot_game::item::{EQUIP_INV_SWORD_BIGGORON, EQUIP_INV_SWORD_BROKENGIANTKNIFE, EQUIP_TYPE_SWORD};
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

/// Medigoron's switch flag (his params): the inner wall's, `Bg_Breakwall` 0xA006.
const WALL_FLAG: i32 = 6;

/// Link (`adult` or a child) at Goron City's bottom at 10:00 with `edit` applied to the save,
/// then room 2 loaded (its objects two frames, the actors' inits, Medigoron's wait for
/// `object_gm`).
fn room_2(a: &Arc<GameAssets>, adult: bool, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_GORON_CITY_2").expect("entrance");
    let mut save = SaveContext::new(e, adult, oot_game::env::clock_time(10, 0) as u16);
    if !adult {
        save.apply_preset("cavern-bombs").unwrap();
    }
    edit(&mut save);
    let mut w = PlayState::play_init_with(
        a.clone(),
        data().unwrap(),
        rules().unwrap(),
        save,
        oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true),
    )
    .expect("Play_Init");
    idle(&mut w, 4);
    assert!(w.room_request(2));
    idle(&mut w, 1);
    w.room_change_done();
    idle(&mut w, 10);
    w
}

fn medigoron(w: &PlayState) -> ActorHandle {
    let v: Vec<ActorHandle> = w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnGm>(h).is_some_and(|g| !g.actor.killed)).collect();
    assert_eq!(v.len(), 1, "one Medigoron");
    v[0]
}

fn gm(w: &PlayState, h: ActorHandle) -> &EnGm {
    w.actors.downcast::<EnGm>(h).unwrap()
}

fn base(w: &PlayState, h: ActorHandle) -> &Actor {
    w.actors.actor(h).unwrap()
}

/// Link at his talk point (4300 model units, 215, in front of him), facing him, until he's the
/// actor Player would talk to.
fn offered(w: &mut PlayState, h: ActorHandle) {
    for _ in 0..30 {
        let a = base(w, h);
        let (p, yaw) = (a.world_pos, a.shape_rot.y);
        let r = yaw as f32 * std::f32::consts::PI / 32768.0;
        w.place_player(Vec3::new(p.x + 200.0 * r.sin(), p.y, p.z + 200.0 * r.cos()), yaw.wrapping_add(i16::MIN));
        idle(w, 1);
        if w.player().target_actor == Some(h) {
            return;
        }
    }
    panic!("Medigoron never offered to talk");
}

/// A until the talk starts, then A whenever the box waits (on a choice: `choice` first) until it
/// closes. The texts shown.
fn talk(w: &mut PlayState, choice: u8) -> Vec<u16> {
    let a = PadState { button: BTN_A, ..Default::default() };
    let mut prev = PadState::default();
    let mut n = 0;
    while w.message_state() == TEXT_STATE_NONE {
        let pad = if prev.button == 0 { a } else { PadState::default() };
        // (A box already open: straight on.)
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

/// Up to `n` frames for a message box to open on its own (a get-item's), then A through it as
/// `talk` does. The texts shown (none if it didn't open).
fn next_box(w: &mut PlayState, n: usize) -> Vec<u16> {
    for _ in 0..n {
        if w.message_state() != TEXT_STATE_NONE {
            return talk(w, 0);
        }
        idle(w, 1);
    }
    Vec::new()
}

#[test]
fn medigoron_sets_up_once_object_gm_is_in() {
    let Some(a) = assets() else { return };
    let mut w = room_2(&a, false, |_| {});
    let h = medigoron(&w);
    // Link in front of him: he's drawn (his draw places his cylinder and talk point).
    offered(&mut w, h);
    let g = gm(&w, h);
    // EnGm_Init: sInitChain (ATTENTION_RANGE_5, lockOnArrowOffset 30), object_gm's slot (room 2
    // lists it). func_80A3D838 once it's loaded: culled from then on, scale 0.05, immovable, the
    // eyes open with 20 to the blink (one frame since: 19 or less), text 0x3049, func_80A3DB04.
    assert!(g.gm_object_slot.is_some());
    assert_eq!(g.update_func, UpdateFunc::Main);
    assert!(g.drawn);
    assert_eq!((g.actor.target_mode, g.actor.target_arrow_offset), (5, 30.0));
    assert_eq!(g.actor.flags & ACTOR_FLAG_UPDATE_CULLING_DISABLED, 0);
    assert_eq!(g.actor.scale, Vec3::splat(0.05));
    assert_eq!(g.actor.col_chk_info.mass, oot_game::collision_check::MASS_IMMOVABLE);
    assert_eq!((g.actor.text_id, g.action), (0x3049, Action::BeforeFlag));
    assert!(g.skel_anime.as_ref().unwrap().is("object_gm_Anim_0002B8"));
    // func_80A3E090 (his draw): the cylinder 2600 model units (130) in front of him, his talk
    // point 4300 (215), his focus 3800 (190) in front and 100 up.
    let (p, yaw) = (g.actor.world_pos, g.actor.shape_rot.y);
    let fwd = |d: f32| p + Vec3::new(eng_math::sin_s(yaw) * d, 0.0, eng_math::cos_s(yaw) * d);
    assert!(g.talk_pos.distance(fwd(215.0)) < 0.1, "{} vs {}", g.talk_pos, fwd(215.0));
    assert!(g.actor.focus_pos.distance(fwd(190.0) + Vec3::Y * 100.0) < 0.1);
    let c = fwd(130.0);
    assert_eq!(g.collider.dim.pos, [c.x as i16, c.y as i16, c.z as i16]);
    assert_eq!((g.collider.dim.radius, g.collider.dim.height), (100, 120));
}

#[test]
fn a_child_before_the_walls_flag_gets_0x3049() {
    let Some(a) = assets() else { return };
    let mut w = room_2(&a, false, |_| {});
    let h = medigoron(&w);
    // func_80A3DB04: no flag 6, text 0x3049; the talk (func_80A3DBF4) and back.
    offered(&mut w, h);
    assert_eq!(talk(&mut w, 0), vec![0x3049]);
    assert_eq!(gm(&w, h).action, Action::BeforeFlag);
    assert!(!w.save.get_inf_table(en_gm::INFTABLE_B0));
}

#[test]
fn a_child_after_the_walls_flag_gets_0x304a_then_0x304b() {
    let Some(a) = assets() else { return };
    // The inner wall broken (its flag 6, a permanent flag: in the scene's flags).
    let mut w = room_2(&a, false, |s| s.scene_flags[0x62].swch |= 1 << WALL_FLAG);
    let h = medigoron(&w);
    // (Culled once set up: he updates once Link is near, in his culling volume.)
    offered(&mut w, h);
    // func_80A3DB04: the flag set, EnGm_SetTextID (a child, no INFTABLE_B0: 0x304A),
    // func_80A3DC44.
    assert_eq!((base(&w, h).text_id, gm(&w, h).action), (0x304A, Action::AfterFlag));
    // func_80A3DC44's talk: a child sets INFTABLE_B0 (func_80A3DD7C to the text's end).
    // (0x304A goes on to 0x304B in the same box: the message's own next text.)
    assert_eq!(talk(&mut w, 0), vec![0x304A, 0x304B]);
    assert!(w.save.get_inf_table(en_gm::INFTABLE_B0));
    assert_eq!(gm(&w, h).action, Action::AfterFlag);
    idle(&mut w, 1);
    assert_eq!(base(&w, h).text_id, 0x304B);
    offered(&mut w, h);
    assert_eq!(talk(&mut w, 0), vec![0x304B]);
}

#[test]
fn an_adult_without_200_rupees_cant_buy_the_knife() {
    let Some(a) = assets() else { return };
    let mut w = room_2(&a, true, |s| {
        s.scene_flags[0x62].swch |= 1 << WALL_FLAG;
        s.rupees = 199;
    });
    let h = medigoron(&w);
    // An adult without the knife: 0x304C (no INFTABLE_B1), which the talk sets; the choice "yes"
    // with under 200 rupees: text 0xC8; nothing given.
    offered(&mut w, h);
    assert_eq!(base(&w, h).text_id, 0x304C);
    let texts = talk(&mut w, 0);
    assert_eq!(texts.first(), Some(&0x304C));
    assert_eq!(texts.last(), Some(&0xC8), "{texts:x?}");
    assert!(w.save.get_inf_table(en_gm::INFTABLE_B1));
    assert!(!w.save.check_owned_equip(EQUIP_TYPE_SWORD, EQUIP_INV_SWORD_BIGGORON));
    assert_eq!(w.save.rupees, 199);
    idle(&mut w, 1);
    assert_eq!(base(&w, h).text_id, 0x304F);
    // "No": text 0x3050.
    offered(&mut w, h);
    let texts = talk(&mut w, 1);
    assert_eq!((texts.first(), texts.last()), (Some(&0x304F), Some(&0x3050)), "{texts:x?}");
}

#[test]
fn an_adult_with_200_rupees_buys_the_knife() {
    let Some(a) = assets() else { return };
    let mut w = room_2(&a, true, |s| {
        s.scene_flags[0x62].swch |= 1 << WALL_FLAG;
        s.rupees = 250;
    });
    let h = medigoron(&w);
    offered(&mut w, h);
    // "Yes" with 200: Actor_OfferGetItem(GI_SWORD_KNIFE, 415, 10) until Link has it
    // (func_80A3DF00), then its text done, Rupees_ChangeBy(-200) (func_80A3DF60).
    let texts = talk(&mut w, 0);
    assert_eq!(texts, vec![0x304C, 0x304F]);
    assert_eq!(gm(&w, h).action, Action::OfferKnife);
    // The box closed, Player takes the knife (Player_ActionHandler_2): his parent; its text (the
    // get-item's) to its end.
    let got = next_box(&mut w, 120);
    assert!(!got.is_empty(), "the knife's text");
    assert!(w.save.check_owned_equip(EQUIP_TYPE_SWORD, EQUIP_INV_SWORD_BIGGORON), "the knife");
    assert_eq!(gm(&w, h).action, Action::AfterFlag);
    // The 200 counted out by Interface_Update.
    idle(&mut w, 250);
    assert_eq!(w.save.rupees, 50);
    // With the knife whole: 0x304D, just a talk.
    assert_eq!(base(&w, h).text_id, 0x304D);
}

#[test]
fn an_adults_text_follows_the_knife() {
    let Some(a) = assets() else { return };
    // func_80A3D7C8: the knife broken (EQUIP_INV_SWORD_BROKENGIANTKNIFE too): 2, text 0x304E.
    let owned = |s: &mut SaveContext, v: u16| s.inventory.equipment |= 1 << (v + EQUIP_TYPE_SWORD as u16 * 4);
    let w = room_2(&a, true, |s| {
        s.scene_flags[0x62].swch |= 1 << WALL_FLAG;
        owned(s, EQUIP_INV_SWORD_BIGGORON);
        owned(s, EQUIP_INV_SWORD_BROKENGIANTKNIFE);
    });
    assert_eq!(en_gm::func_80a3d7c8(&w), 2);
    let mut w = w;
    let h = medigoron(&w);
    offered(&mut w, h);
    assert_eq!(base(&w, h).text_id, 0x304E);
    // Biggoron's Sword (the bit alone): 3, text 0x304D.
    let w = room_2(&a, true, |s| {
        s.scene_flags[0x62].swch |= 1 << WALL_FLAG;
        owned(s, EQUIP_INV_SWORD_BIGGORON);
    });
    assert_eq!(en_gm::func_80a3d7c8(&w), 3);
    let mut w = w;
    let h = medigoron(&w);
    offered(&mut w, h);
    assert_eq!(base(&w, h).text_id, 0x304D);
}
