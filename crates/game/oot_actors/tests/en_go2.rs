//! GAME-09 milestone 2 against the C: the Gorons (`z_en_go2.c`) of Death Mountain Trail, Goron City
//! and the bazaar: which placements live (`EnGo2_Init`), the child's texts and flags
//! (`EnGo2_GetTextId*`, `EnGo2_UpdateTalkState*`), waking and talking (`EnGo2_IsWakingUp`,
//! `func_80A46B40`, `Npc_UpdateTalking`), the bomb-flower Goron's walk, the trail's rolling Goron
//! (`EnGo2_ContinueRolling`, `EnGo2_SlowRolling`, `func_80A44AB0`, `EnGo2_StopRolling`'s bomb) and the
//! city's big one stopped by a bomb and his bomb bag.
//!
//! Entered from `ranch-epona` (the phase's start) with the trail's and the city's intros seen
//! (`EVENTCHKINF_A1`, `EVENTCHKINF_A6`). Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_bom::EnBom;
use oot_actors::en_go2::{self, Action, EnGo2};
use oot_actors::player::Action as PlayerAction;
use oot_game::actor_ctx::ActorHandle;
use oot_game::item::{ITEM_BOMB, ITEM_BOMB_BAG_20, ITEM_STRENGTH_GORONS_BRACELET, QUEST_GORON_RUBY, UPG_BOMB_BAG, item_give, slot};
use oot_game::message::{MSGMODE_TEXT_AWAIT_INPUT, TEXT_STATE_AWAITING_NEXT, TEXT_STATE_CHOICE, TEXT_STATE_DONE, TEXT_STATE_DONE_HAS_NEXT, TEXT_STATE_EVENT, TEXT_STATE_NONE};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

/// `EVENTCHKINF_A1` (the trail's intro, `gDMTIntroCs`), `EVENTCHKINF_A6` (Goron City's,
/// `gGoronCityIntroCs`) (`save.h`; `sEntranceCutsceneTable`).
const EVENTCHKINF_A1: u16 = 0xA1;
const EVENTCHKINF_A6: u16 = 0xA6;
/// `QUEST_MEDALLION_FIRE` (`item.h`).
const QUEST_MEDALLION_FIRE: u32 = 0x01;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Link (`adult` or a child) entered at `entrance` at 10:00 from `ranch-epona`, the intros seen,
/// `edit` applied last; eight frames on (the room's actors are in on the third, each initialised
/// once its object has loaded).
fn enter(a: &Arc<GameAssets>, entrance: &str, adult: bool, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, adult, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("ranch-epona").unwrap();
    save.set_event_chk_inf(EVENTCHKINF_A1);
    save.set_event_chk_inf(EVENTCHKINF_A6);
    edit(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    idle(&mut w, 8);
    w
}

fn trail(a: &Arc<GameAssets>, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    enter(a, "ENTR_DEATH_MOUNTAIN_TRAIL_0", false, edit)
}

fn city(a: &Arc<GameAssets>, adult: bool, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    enter(a, "ENTR_GORON_CITY_0", adult, edit)
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// The Gorons in the arena, killed or not (until deleted).
fn gorons(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnGo2>(h).is_some()).collect()
}

/// The live Gorons' types, sorted.
fn live_types(w: &PlayState) -> Vec<i16> {
    let mut v: Vec<i16> = gorons(w).into_iter().map(|h| go(w, h)).filter(|g| !g.actor.killed).map(|g| en_go2::TYPE.get(g.actor.params)).collect();
    v.sort();
    v
}

/// The live placeholders standing in for `En_Go2`, by their types.
fn placeholder_types(w: &PlayState) -> Vec<i16> {
    w.actors
        .all()
        .into_iter()
        .filter_map(|h| w.actors.get(h))
        .filter(|a| a.name() == "Placeholder" && a.base().id == en_go2::ACTOR_EN_GO2 && !a.base().killed)
        .map(|a| en_go2::TYPE.get(a.base().params))
        .collect()
}

fn go(w: &PlayState, h: ActorHandle) -> &EnGo2 {
    w.actors.downcast::<EnGo2>(h).expect("En_Go2")
}

fn go_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnGo2 {
    w.actors.downcast_mut::<EnGo2>(h).expect("En_Go2")
}

/// The one live Goron of type `ty`.
fn goron(w: &PlayState, ty: i16) -> ActorHandle {
    let v: Vec<ActorHandle> = gorons(w).into_iter().filter(|&h| !go(w, h).actor.killed && en_go2::TYPE.get(go(w, h).actor.params) == ty).collect();
    assert_eq!(v.len(), 1, "one Goron of type {ty:#x}");
    v[0]
}

/// Link `d` in front of `h` (along its facing), facing it.
fn before(w: &mut PlayState, h: ActorHandle, d: f32) {
    let a = &go(w, h).actor;
    let (p, yaw) = (a.world_pos, a.shape_rot.y);
    let r = yaw as f32 * std::f32::consts::PI / 32768.0;
    w.place_player(Vec3::new(p.x + d * r.sin(), p.y, p.z + d * r.cos()), yaw.wrapping_add(i16::MIN));
}

/// Link `d` in front of a curled Goron until he's woken up, sat up and offers to talk (Player's
/// `talkActor`); the frames it took.
fn woken_and_offered(w: &mut PlayState, h: ActorHandle, d: f32) -> usize {
    for n in 0..200 {
        before(w, h, d);
        idle(w, 1);
        if go(w, h).is_awake && w.player().target_actor == Some(h) {
            return n;
        }
    }
    panic!("the Goron never offered to talk (action {:?})", go(w, h).action);
}

/// A until the talk starts (unless it has), then A whenever the box waits (once each wait),
/// choosing `choice` at a choice, until it closes. The texts shown.
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
    for _ in 0..2000 {
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
        let waits = matches!(st, TEXT_STATE_AWAITING_NEXT | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_CHOICE | TEXT_STATE_EVENT) || mode == MSGMODE_TEXT_AWAIT_INPUT;
        let pad = if waits && prev.button == 0 && pressed_at.is_none() {
            if st == TEXT_STATE_CHOICE {
                w.msg_ctx.choice_index = choice;
            }
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

/// Wakes the Goron `h` with Link `d` in front, talks (answering `choice`), and lets two frames
/// go by; the texts.
fn wake_and_talk(w: &mut PlayState, h: ActorHandle, d: f32, choice: u8) -> Vec<u16> {
    woken_and_offered(w, h, d);
    let t = talk(w, choice);
    idle(w, 2);
    t
}

// Which Gorons live.

#[test]
fn every_placed_child_goron_lives() {
    let Some(a) = assets() else { return };
    // spot16 layer 0 (a child by day): 0x1404 (type 4, path 0), 0xFC25 (type 5, path 1), 0xFFE6
    // (type 6), 0xFFEC (type 0xC). EnGo2_Init kills none of them for a child.
    let w = trail(&a, |_| {});
    assert_eq!(live_types(&w), [4, 5, 6, 0xC]);
    // The rolling one starts rolling (EnGo2_SetupRolling: speed 6 × 2, 1800 up, ten frames), the
    // others curled (ENGO2_ANIM_0, gGoronAnim_004930 at frame 0 still: drawn as gGoronDL_00BD80);
    // the trail's cylinders 30 across (68 high from EnGo2_SetColliderDim until their first
    // EnGo2_CurledUp: their update is culled out of view).
    let r = go(&w, goron(&w, en_go2::GORON_DMT_ROLLING_SMALL));
    assert_eq!((r.action, r.actor.speed_xz, r.actor.shape_y_offset, r.anim_timer), (Action::ContinueRolling, 12.0, 1800.0, 10));
    let c = go(&w, goron(&w, en_go2::GORON_DMT_DC_ENTRANCE));
    assert_eq!((c.action, c.skel.cur_frame, c.skel.play_speed, c.draw_mode()), (Action::CurledUp, 0.0, 0.0, 1));
    assert_eq!(c.collider.dim.radius, 30);
    assert_eq!((c.actor.scale.x, c.interact_range), (0.01, 60.0));
    // spot18 layer 0 (a child): types 0xA, 0xB, 9, 8, 7 and 0 (path 0), all in room 3.
    let w = city(&a, false, |_| {});
    assert_eq!(live_types(&w), [0, 7, 8, 9, 0xA, 0xB]);
    let big = go(&w, goron(&w, en_go2::GORON_CITY_ROLLING_BIG));
    // D_80A4816C[0]: 68 across, 148 × 0.6 = 88 high; D_80A481F8[0]: scale 0.026, range 60 + 68;
    // EnGo2_SetupRolling without INFTABLE_11E: 3.6 × 2; his cylinder takes hits.
    assert_eq!((big.collider.dim.radius, big.collider.dim.height, big.actor.scale.x, big.interact_range), (68, 88, 0.026, 128.0));
    assert!((big.actor.speed_xz - 7.200_000_3).abs() < 1e-6, "{}", big.actor.speed_xz);
    assert_eq!(big.collider.info.ac_elem_flags, oot_game::collision_check::ACELEM_ON);
}

#[test]
fn the_city_s_gorons_die_for_an_adult_without_the_fire_medallion() {
    let Some(a) = assets() else { return };
    // spot18 layer 2 (an adult by day): types 0xA, 0xB, 1 (0x1C01, Link the Goron: path 0, rot.z
    // 10), 9, 8, 7. Without the Fire Medallion types 7 to 0xB are killed; Link the Goron is the
    // placeholder (Phase 15).
    let w = city(&a, true, |_| {});
    assert_eq!(live_types(&w), Vec::<i16>::new());
    assert_eq!(placeholder_types(&w), [1]);
    // With it they live (and say 0x3043).
    let w = city(&a, true, |s| s.inventory.quest_items |= 1 << QUEST_MEDALLION_FIRE);
    assert_eq!(live_types(&w), [7, 8, 9, 0xA, 0xB]);
}

#[test]
fn the_bazaar_s_goron_needs_the_ruby_and_a_child() {
    let Some(a) = assets() else { return };
    // shop1: 0xFFED (type 0xD) at (54, 0, 73). Killed without the Goron's Ruby.
    let w = enter(&a, "ENTR_BAZAAR_0", false, |_| {});
    assert_eq!(live_types(&w), Vec::<i16>::new());
    // With it: EnGo2_GetItemAnimation: awake at once (ENGO2_ANIM_1 held at its end, unk_211).
    let mut w = enter(&a, "ENTR_BAZAAR_0", false, |s| s.inventory.quest_items |= 1 << QUEST_GORON_RUBY);
    let h = goron(&w, en_go2::GORON_MARKET_BAZAAR);
    let g = go(&w, h);
    assert_eq!((g.action, g.unk_211, g.skel.play_speed), (Action::Func80A46B40, true, 0.0));
    assert_eq!(g.skel.cur_frame, g.skel.end_frame);
    // EnGo2_GetTextIdGoronMarketBazaar: 0x7122.
    assert_eq!(wake_and_talk(&mut w, h, 40.0, 0), [0x7122]);
    // An adult (the Kakariko bazaar is the same scene): killed.
    let w = enter(&a, "ENTR_BAZAAR_0", true, |s| s.inventory.quest_items |= 1 << QUEST_GORON_RUBY);
    assert_eq!(live_types(&w), Vec::<i16>::new());
}

// Waking and the texts.

#[test]
fn the_cavern_s_goron_wakes_talks_and_curls_up() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a, |_| {});
    let h = goron(&w, en_go2::GORON_DMT_DC_ENTRANCE);
    // Link 300 in front: beyond 200, he sleeps; seen, he updates: EnGo2_CurledUp's cylinder 68 × 0.6
    // = 40 high at frame 0.
    before(&mut w, h, 300.0);
    idle(&mut w, 10);
    assert_eq!((go(&w, h).action, go(&w, h).collider.dim.height), (Action::CurledUp, 40));
    // 100 in front: EnGo2_IsWakingUp; EnGo2_WakeUp (gGoronAnim_004930 forwards), func_80A46B40.
    before(&mut w, h, 100.0);
    idle(&mut w, 1);
    let g = go(&w, h);
    assert_eq!((g.action, g.skel.play_speed, g.is_awake, g.unk_211), (Action::Func80A46B40, 1.0, false, false));
    // At the animation's end func_80A454CC: type 6 stands (ENGO2_ANIM_9, gGoronAnim_0021D0); the
    // cylinder at its full 68; then awake (EnGo2_DefaultWakingUp).
    let mut n = 0;
    while !go(&w, h).unk_211 {
        before(&mut w, h, 100.0);
        idle(&mut w, 1);
        n += 1;
        assert!(n < 100);
    }
    let g = go(&w, h);
    assert!(g.skel.is("gGoronAnim_0021D0"));
    assert_eq!(g.collider.dim.height, 68);
    // The texts: 0x3008 the first time (the message goes on to 0x302A itself; INFTABLE_E0 as it
    // closes, the actor's textId still 0x3008), then 0x302A.
    assert_eq!(wake_and_talk(&mut w, h, 40.0, 0), [0x3008, 0x302A]);
    assert!(w.save.get_inf_table(en_go2::INFTABLE_E0));
    assert_eq!(wake_and_talk(&mut w, h, 40.0, 0), [0x302A]);
    // Link leaves: not EnGo2_IsCameraModified for type 6: EnGo2_RollingAnimation (backwards from
    // the end), and curled at frame 0 again (the dust as it ends).
    before(&mut w, h, 400.0);
    idle(&mut w, 1);
    let g = go(&w, h);
    assert_eq!((g.action, g.skel.play_speed, g.is_awake), (Action::CurledUp, -1.0, false));
    let mut n = 0;
    while go(&w, h).draw_mode() != 1 {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 100, "curled up again");
    }
    assert!(go(&w, h).effects.iter().any(|e| e.type_ == 1), "EnGo2_GetDustData(this, 1)");
    // With the boulder blown (EVENTCHKINF_23): 0x3021.
    let mut w = trail(&a, |s| s.set_event_chk_inf(en_go2::EVENTCHKINF_23));
    let h = goron(&w, en_go2::GORON_DMT_DC_ENTRANCE);
    assert_eq!(wake_and_talk(&mut w, h, 40.0, 0), [0x3021]);
}

#[test]
fn the_bomb_flower_goron_s_choice_without_the_bracelet() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a, |_| {});
    let h = goron(&w, en_go2::GORON_DMT_BOMB_FLOWER);
    // 0x300A's choice: "yes" (0) without the bracelet: 0x300C.
    assert_eq!(wake_and_talk(&mut w, h, 50.0, 0), [0x300A, 0x300C]);
    // "No" (1): 0x300D. INFTABLE_EB never set: he stays.
    assert_eq!(wake_and_talk(&mut w, h, 50.0, 1), [0x300A, 0x300D]);
    assert!(!w.save.get_inf_table(en_go2::INFTABLE_EB));
    assert_eq!(go(&w, h).action, Action::Func80A46B40);
}

#[test]
fn the_bomb_flower_goron_with_the_bracelet_steps_aside_next_time() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a, |s| {
        item_give(s, None, ITEM_STRENGTH_GORONS_BRACELET);
    });
    let h = goron(&w, en_go2::GORON_DMT_BOMB_FLOWER);
    // Placed on path 0's first point (-1278, 1411, -552), facing 15656.
    let start = go(&w, h).actor.world_pos;
    assert_eq!((start.x, start.z, go(&w, h).actor.shape_rot.y), (-1278.0, -552.0, 15656));
    woken_and_offered(&mut w, h, 50.0);
    // "Yes" with the bracelet: 0x300B; as it closes INFTABLE_EB and NPC_TALK_STATE_ACTION.
    let texts = talk(&mut w, 0);
    assert_eq!(texts, [0x300A, 0x300B]);
    assert!(w.save.get_inf_table(en_go2::INFTABLE_EB));
    // The next update: EnGo2_IsGoronDmtBombFlower: ENGO2_ANIM_3 (gGoronAnim_010590), not awake,
    // walking (EnGo2_GoronDmtBombFlowerAnimation).
    w.place_player(Vec3::new(start.x + 300.0, start.y, start.z), 0);
    let mut walked = 0;
    let mut frames = 0;
    while go(&w, h).action != Action::GoronDmtBombFlowerAnimation {
        idle(&mut w, 1);
        frames += 1;
        assert!(frames < 10, "the walk");
    }
    assert!(go(&w, h).skel.is("gGoronAnim_010590"));
    assert!(!go(&w, h).is_awake);
    // He aims for waypoint 0, his own point: Path_OrientAndGetDistSq's 0 isn't over 0, so the
    // waypoint stays until his first step (a sin of the walk's frame, under 1) takes him off it;
    // then within 30, EnGo2_UpdateWaypoint: 1, not under the count less one (1): back to 0, and
    // EnGo2_GetItemAnimation: he sits again (ENGO2_ANIM_1 at its end, unk_211, speed 0).
    while go(&w, h).action == Action::GoronDmtBombFlowerAnimation {
        idle(&mut w, 1);
        walked += 1;
        assert!(walked < 10, "a walk that never ends");
    }
    let g = go(&w, h);
    assert_eq!((g.action, g.unk_211, g.skel.play_speed, g.actor.speed_xz, g.waypoint), (Action::Func80A46B40, true, 0.0, 0.0, 0));
    let moved = Vec3::new(g.actor.world_pos.x - start.x, 0.0, g.actor.world_pos.z - start.z).length();
    eprintln!("the walk: {walked} frames, {moved} moved, world yaw {}", g.actor.world_rot.y);
    assert!(moved < 1.0, "he barely moves: {moved}");
    assert_eq!(g.actor.shape_rot.y, 15656, "his facing isn't turned");
    // Next time (INFTABLE_EB at init): at the path's last point (-1263, 1412, -730).
    let w = trail(&a, |s| {
        item_give(s, None, ITEM_STRENGTH_GORONS_BRACELET);
        s.set_inf_table(en_go2::INFTABLE_EB);
    });
    let g = go(&w, goron(&w, en_go2::GORON_DMT_BOMB_FLOWER));
    assert_eq!((g.actor.world_pos.x, g.actor.world_pos.z), (-1263.0, -730.0));
    assert_eq!((g.actor.home_pos.x, g.actor.home_pos.z), (-1263.0, -730.0));
    // The Bomb Flower at (-1397, 1408, -646): 152 from his first place, 158 from this one.
    let flower = Vec3::new(-1397.0, 1408.0, -646.0);
    eprintln!("from the flower: {} then {}", Vec3::new(start.x - flower.x, 0.0, start.z - flower.z).length(), Vec3::new(-1263.0 - flower.x, 0.0, -730.0 - flower.z).length());
}

#[test]
fn the_lowest_floor_goron_reads_darunia_s_door() {
    let Some(a) = assets() else { return };
    // Darunia's door shut (switch 0x1B clear): 0x3017.
    let mut w = city(&a, false, |_| {});
    let h = goron(&w, en_go2::GORON_CITY_LOWEST_FLOOR);
    assert_eq!(wake_and_talk(&mut w, h, 40.0, 0), [0x3017]);
    // Open: 0x3018 (the message goes on to 0x3019 and 0x302C itself; INFTABLE_F8 as it closes),
    // then 0x3019.
    w.flags.set_switch(0x1B);
    assert_eq!(wake_and_talk(&mut w, h, 40.0, 0), [0x3018, 0x3019, 0x302C]);
    assert!(w.save.get_inf_table(en_go2::INFTABLE_F8));
    assert_eq!(wake_and_talk(&mut w, h, 40.0, 0), [0x3019, 0x302C]);
    // With the bracelet (CUR_UPG_VALUE(UPG_STRENGTH) != 0): 0x302C.
    let mut w = city(&a, false, |s| {
        item_give(s, None, ITEM_STRENGTH_GORONS_BRACELET);
    });
    let h = goron(&w, en_go2::GORON_CITY_LOWEST_FLOOR);
    assert_eq!(wake_and_talk(&mut w, h, 40.0, 0), [0x302C]);
}

#[test]
fn the_city_s_other_gorons_texts() {
    let Some(a) = assets() else { return };
    let mut w = city(&a, false, |_| {});
    // The entrance's: 0x3014 (going on to 0x3015; INFTABLE_F0), then 0x3015.
    let h = goron(&w, en_go2::GORON_CITY_ENTRANCE);
    assert_eq!(wake_and_talk(&mut w, h, 40.0, 0), [0x3014, 0x3015]);
    assert!(w.save.get_inf_table(en_go2::INFTABLE_F0));
    assert_eq!(wake_and_talk(&mut w, h, 40.0, 0), [0x3015]);
    // The Lost Woods': 0x3024 (going on to 0x3025; INFTABLE_E6), then 0x3025; with switch 0x1C
    // 0x302F.
    let h = goron(&w, en_go2::GORON_CITY_LOST_WOODS);
    assert_eq!(wake_and_talk(&mut w, h, 40.0, 0), [0x3024, 0x3025]);
    assert!(w.save.get_inf_table(en_go2::INFTABLE_E6));
    w.flags.set_switch(0x1C);
    // The urn (Bg_Spot18_Basket) answers the torches' switch with one-point 4220 (80 frames),
    // Link held (PLAYER_CSACTION_1): over before the talk.
    idle(&mut w, 120);
    assert_eq!(wake_and_talk(&mut w, h, 40.0, 0), [0x302F]);
    // With the Goron's Ruby the entrance's says 0x3027.
    let mut w = city(&a, false, |s| s.inventory.quest_items |= 1 << QUEST_GORON_RUBY);
    let h = goron(&w, en_go2::GORON_CITY_ENTRANCE);
    assert_eq!(wake_and_talk(&mut w, h, 40.0, 0), [0x3027]);
}

// The trail's rolling Goron.

/// Path 1's points (spot16 layer 0).
const PATH1: [[f32; 3]; 9] = [
    [-311.0, 1500.0, -393.0],
    [-464.0, 1465.0, -129.0],
    [-437.0, 1418.0, 133.0],
    [-577.0, 1371.0, 313.0],
    [-626.0, 1247.0, 593.0],
    [-899.0, 1164.0, 848.0],
    [-1286.0, 1130.0, 1098.0],
    [-1548.0, 1048.0, 1235.0],
    [-1680.0, 1045.0, 1208.0],
];
/// Where Link is far from everything: the trail's spawn 0, at its foot.
const FOOT: Vec3 = Vec3::new(-1344.0, 118.0, 2221.0);

fn p(i: usize) -> Vec3 {
    Vec3::from(PATH1[i])
}

/// The trail with Link at its foot, the rolling Goron waiting; then Link at path 1's point 3
/// (765 from him), facing him, until he rolls. The frames from Link's coming within 1000.
fn rolling_started(a: &Arc<GameAssets>) -> (PlayState, ActorHandle, usize) {
    let mut w = trail(a, |_| {});
    w.place_player(FOOT, 0);
    let h = goron(&w, en_go2::GORON_DMT_ROLLING_SMALL);
    // Far away he waits (and, his update culled like the other trail Gorons' as EnGo2_Init clears
    // ACTOR_FLAG_UPDATE_CULLING_DISABLED for types 3 to 0xD, doesn't even update out of view).
    idle(&mut w, 30);
    let g = go(&w, h);
    assert_eq!((g.action, g.anim_timer, g.actor.world_pos.x, g.actor.world_pos.z), (Action::ContinueRolling, 10, -311.0, -393.0));
    w.place_player(p(3), oot_game::target::yaw_to(p(3), p(0)));
    let mut n = 0;
    while go(&w, h).action == Action::ContinueRolling {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 20);
    }
    // Ten frames' DECR(animTimer) (10 to 0) once within 1000 (and seen), then EnGo2_SlowRolling at
    // half speed (6); the dust of EnGo2_ContinueRolling meanwhile (EnGo2_GetDustData(this, 2)).
    assert!((10..=11).contains(&n), "{n}");
    let g = go(&w, h);
    assert_eq!((g.action, g.anim_timer, g.actor.speed_xz), (Action::SlowRolling, 0, 6.0));
    assert!(g.effects.iter().any(|e| e.type_ == 1));
    (w, h, n)
}

/// One frame with Link following the rolling Goron `h`: where he was 30 frames ago (`trail`),
/// facing him, so the camera keeps him in view (his update is culled out of it).
fn follow(w: &mut PlayState, h: ActorHandle, trail: &mut std::collections::VecDeque<Vec3>) {
    let at = go(w, h).actor.world_pos;
    trail.push_back(at);
    let behind = if trail.len() > 30 { trail.pop_front().unwrap() } else { trail[0] };
    let behind = Vec3::new(behind.x, behind.y + 5.0, behind.z);
    w.place_player(behind, oot_game::target::yaw_to(behind, at));
    idle(w, 1);
}

#[test]
fn the_trail_s_rolling_goron_rolls_his_path_and_drops_a_bomb() {
    let Some(a) = assets() else { return };
    let (mut w, h, mut frames) = rolling_started(&a);
    let mut max_waypoint = 0;
    let mut stop_at = None;
    let mut trail = std::collections::VecDeque::new();
    while stop_at.is_none() {
        follow(&mut w, h, &mut trail);
        frames += 1;
        assert!(frames < 1500, "never stopped (waypoint {})", go(&w, h).waypoint);
        let g = go(&w, h);
        max_waypoint = max_waypoint.max(g.waypoint);
        if g.action != Action::SlowRolling {
            stop_at = Some(g.actor.world_pos);
        }
    }
    let stop = stop_at.unwrap();
    // EnGo2_UpdateWaypoint: waypoints 0 to 7 (count 9 less two), then 0 again: EnGo2_StopRolling
    // within 30 of point 7, the path's second-to-last (the last, 8, is never aimed for).
    let g = go(&w, h);
    assert_eq!((g.action, g.waypoint, g.actor.speed_xz, g.actor.shape_y_offset), (Action::GroundRolling, 0, 0.0, 0.0));
    assert_eq!(max_waypoint, 7);
    let d7 = Vec3::new(stop.x - p(7).x, 0.0, stop.z - p(7).z).length();
    assert!(d7 < 30.0, "{stop:?}: {d7} from point 7");
    // His bomb (En_Bom, timer 0) where he stopped: it blows on its first update.
    let bombs: Vec<ActorHandle> = w.actors.all().into_iter().filter(|&b| w.actors.downcast::<EnBom>(b).is_some()).collect();
    assert_eq!(bombs.len(), 1);
    let b = w.actors.downcast::<EnBom>(bombs[0]).unwrap();
    assert_eq!((b.timer, b.actor.home_pos), (0, stop));
    eprintln!("from Link within 1000 to the bomb: {frames} frames; the bomb at {stop:?} ({d7} from point 7)");
    idle(&mut w, 1);
    assert_eq!(w.actors.downcast::<EnBom>(bombs[0]).unwrap().actor.params, 1, "BOMB_EXPLOSION");
    // EnGo2_GroundRolling: four bounces (8, 6, 4, 2 up), then curled (EnGo2_CurledUp, ENGO2_ANIM_0
    // still at 0: the rock).
    let mut n = 0;
    while go(&w, h).action == Action::GroundRolling {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 200);
    }
    assert_eq!((go(&w, h).action, go(&w, h).draw_mode()), (Action::CurledUp, 1));
    // EnGo2_GetTextIdGoronDmtRollingSmall: 0x3009 (0x3026 once the boulder's blown).
    assert_eq!(wake_and_talk(&mut w, h, 40.0, 0), [0x3009]);
}

#[test]
fn the_trail_s_rolling_goron_knocks_link_down() {
    let Some(a) = assets() else { return };
    let (mut w, h, _) = rolling_started(&a);
    // Link following him until he aims for point 5; then on point 5, facing him.
    let mut trail = std::collections::VecDeque::new();
    while go(&w, h).waypoint < 5 {
        follow(&mut w, h, &mut trail);
    }
    let at = go(&w, h).actor.world_pos;
    w.place_player(p(5), oot_game::target::yaw_to(p(5), at));
    let health = w.save.health;
    let mut n = 0;
    while w.save.health == health {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 600, "he never reached Link (at {:?})", go(&w, h).actor.world_pos);
    }
    // func_80A44AB0: OC2_HIT_PLAYER: play->damagePlayer(play, -4), Actor_SetPlayerKnockbackLargeNoDamage(
    // speed × 1.5, yawTowardsPlayer, 6), NA_SE_PL_BODY_HIT; Player's next update knocks him down.
    assert_eq!(w.save.health, health - 4);
    assert_eq!(go(&w, h).action, Action::SlowRolling, "he can't be stopped");
    idle(&mut w, 1);
    assert_eq!(w.player().action, PlayerAction::KnockedDown);
}

// The city's big rolling Goron.

/// Path 0's points (spot18 layer 0).
const CITY_PATH: [[f32; 3]; 14] = [
    [520.0, 399.0, 565.0],
    [45.0, 400.0, 767.0],
    [-259.0, 397.0, 556.0],
    [-495.0, 400.0, 552.0],
    [-435.0, 397.0, 230.0],
    [-568.0, 400.0, -22.0],
    [-434.0, 397.0, -194.0],
    [-478.0, 400.0, -662.0],
    [-185.0, 399.0, -662.0],
    [-69.0, 401.0, -874.0],
    [401.0, 397.0, -530.0],
    [570.0, 399.0, 72.0],
    [428.0, 397.0, 298.0],
    [520.0, 399.0, 565.0],
];

/// Goron City with the 20 bomb bag (if `bag`); the big Goron rolling, put on point `from` aiming
/// for waypoint `from + 1`; a bomb (timer 2) 40 ahead of him. Until he's stopped, settled and
/// woken: his handle.
fn big_one_stopped(a: &Arc<GameAssets>, bag: bool, from: usize) -> (PlayState, ActorHandle) {
    let mut w = city(a, false, |s| {
        if bag {
            item_give(s, None, ITEM_BOMB_BAG_20);
        }
    });
    let h = goron(&w, en_go2::GORON_CITY_ROLLING_BIG);
    while go(&w, h).action != Action::SlowRolling {
        idle(&mut w, 1);
    }
    let at = Vec3::from(CITY_PATH[from]);
    let next = Vec3::from(CITY_PATH[from + 1]);
    {
        let g = go_mut(&mut w, h);
        g.actor.world_pos = at;
        g.actor.prev_pos = at;
        g.waypoint = (from + 1) as i8;
        g.actor.world_rot.y = oot_game::target::yaw_to(at, next);
        g.actor.teleported = true;
    }
    idle(&mut w, 1);
    let pos = go(&w, h).actor.world_pos;
    let ahead = pos + (next - at).normalize() * 40.0;
    let b = w.actor_spawn(oot_actors::en_bom::ACTOR_EN_BOM, Vec3::new(ahead.x, pos.y, ahead.z), [0; 3], 0).unwrap();
    w.actors.downcast_mut::<EnBom>(b).unwrap().timer = 2;
    // The blast's DMG_EXPLOSIVE on his cylinder (ACELEM_ON while rolling): func_80A44AB0's AC_HIT:
    // NA_SE_SY_CORRECT_CHIME, EnGo2_StopRolling (ACELEM_NONE), EnGo2_GroundRolling.
    let mut n = 0;
    while go(&w, h).action == Action::SlowRolling {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 15, "the blast never stopped him");
    }
    let g = go(&w, h);
    assert_eq!((g.action, g.actor.speed_xz, g.collider.info.ac_elem_flags), (Action::GroundRolling, 0.0, oot_game::collision_check::ACELEM_NONE));
    assert_eq!(g.waypoint as usize, from + 1);
    // The bounces, then EnGo2_WakeUp (not curled up: type 0), and awake.
    let mut n = 0;
    while !go(&w, h).is_awake {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 300, "never woke ({:?})", go(&w, h).action);
    }
    (w, h)
}

#[test]
fn the_big_rolling_goron_stopped_between_waypoints_8_and_11_gives_the_bomb_bag() {
    let Some(a) = assets() else { return };
    let (mut w, h) = big_one_stopped(&a, true, 7);
    // EnGo2_GetTextIdGoronCityRollingBig: the 20 bag and waypoint 8: 0x3012. Its event: EnGo2_GetItem
    // (GI_BOMB_BAG_30: CUR_CAPACITY 20), Message_CloseTextbox, INFTABLE_11E.
    before(&mut w, h, 100.0);
    idle(&mut w, 1);
    assert_eq!(w.player().target_actor, Some(h));
    let texts = talk(&mut w, 0);
    assert_eq!(texts, [0x3012]);
    assert!(w.save.get_inf_table(en_go2::INFTABLE_11E));
    assert_eq!(go(&w, h).get_item_id, en_go2::GI_BOMB_BAG_30 as i32);
    // EnGo2_SetupGetItem offers it until Link takes it (Actor_HasParent), then the item's text
    // (EnGo2_SetGetItem): read, he curls up to roll on.
    let mut n = 0;
    while w.message_state() == TEXT_STATE_NONE {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 60, "the bomb bag's text");
    }
    talk(&mut w, 0);
    assert_eq!(w.save.cur_capacity(UPG_BOMB_BAG), 30);
    assert_eq!(w.save.inventory.ammo[slot(ITEM_BOMB)], 30);
    idle(&mut w, 2);
    assert_eq!(go(&w, h).action, Action::GoronRollingBigContinueRolling);
    // Curled (gGoronAnim_004930 back to 0), EnGo2_SetupRolling with INFTABLE_11E: 6 × 2.
    let mut n = 0;
    while go(&w, h).action != Action::ContinueRolling {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 100);
    }
    assert_eq!(go(&w, h).actor.speed_xz, 12.0);
    // Ten frames on, half that (6) in EnGo2_SlowRolling; then Math_ApproachF to
    // EnGo2_GetTargetXZSpeed's 3.6 all the same (0.4 of the way, at most 0.6 a frame).
    while go(&w, h).action == Action::ContinueRolling {
        idle(&mut w, 1);
    }
    assert_eq!(go(&w, h).actor.speed_xz, 6.0);
    idle(&mut w, 30);
    assert!((go(&w, h).actor.speed_xz - 3.6).abs() < 1e-3, "{}", go(&w, h).actor.speed_xz);
}

#[test]
fn the_big_rolling_goron_stopped_elsewhere_says_0x3011_and_rolls_on() {
    let Some(a) = assets() else { return };
    // The 20 bag, but stopped aiming for waypoint 3: 0x3011.
    let (mut w, h) = big_one_stopped(&a, true, 2);
    before(&mut w, h, 100.0);
    idle(&mut w, 1);
    assert_eq!(talk(&mut w, 0), [0x3011]);
    assert!(!w.save.get_inf_table(en_go2::INFTABLE_11E));
    // NPC_TALK_STATE_ACTION: EnGo2_IsGoronRollingBig: curled up, then rolling on at 3.6 × 2.
    let mut n = 0;
    while go(&w, h).action != Action::ContinueRolling {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 100, "{:?}", go(&w, h).action);
    }
    assert!((go(&w, h).actor.speed_xz - 7.200_000_3).abs() < 1e-6);
    assert_eq!(w.save.cur_capacity(UPG_BOMB_BAG), 20);
}
