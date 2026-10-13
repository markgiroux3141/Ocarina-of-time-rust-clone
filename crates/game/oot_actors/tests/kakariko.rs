//! GAME-07 milestone 5 against the C: Kakariko. Player's lift, carry and throw (`z_player.c`:
//! `Player_ActionHandler_2`'s lift, `func_8083A0F4`, `Player_Action_80846050`,
//! `Player_UpperAction_CarryActor` with the cucco's glide, `Player_ActionHandler_9`,
//! `Player_Action_80846578`, `_808464B0`, `_80846408`) with the cuccos (`z_en_niw.c`), the pots
//! (`z_obj_tsubo.c`), the rocks (`z_en_ishi.c`) and the bushes (`z_en_kusa.c`).
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_niw::EnNiw;
use oot_actors::player::{Action as PA, Player, STATE1_11, UpperAction};
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

/// `EVENTCHKINF_A3` (`save.h`): Kakariko's intro seen.
const EVENTCHKINF_A3: u16 = 0xA3;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Child Link entered at `entrance` at `hour`:00 with `preset`, `edit` applied to the save last;
/// two frames on (the room's actors and objects in).
fn enter_with(a: &Arc<GameAssets>, entrance: &str, hour: i32, preset: &str, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(hour, 0) as u16);
    save.apply_preset(preset).unwrap();
    edit(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    idle(&mut w, 2);
    w
}

/// Kakariko by day from its entrance, its intro seen.
fn kakariko(a: &Arc<GameAssets>, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    enter_with(a, "ENTR_KAKARIKO_VILLAGE_0", 10, "deku-tree-dead", |s| {
        s.set_event_chk_inf(EVENTCHKINF_A3);
        edit(s);
    })
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn all<T: 'static + ActorImpl>(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<T>(h).is_some_and(|a| !a.base().killed)).collect()
}

/// The `T` placed at (`x`, `z`).
fn at<T: 'static + ActorImpl>(w: &PlayState, x: f32, z: f32) -> ActorHandle {
    let v: Vec<_> = all::<T>(w).into_iter().filter(|&h| w.actors.actor(h).is_some_and(|a| (a.home_pos.x - x).abs() < 1.0 && (a.home_pos.z - z).abs() < 1.0)).collect();
    assert_eq!(v.len(), 1, "one {} at ({x}, {z})", std::any::type_name::<T>());
    v[0]
}

fn always_update(w: &mut PlayState, h: ActorHandle) {
    w.actors.actor_mut(h).unwrap().flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
}

fn base(w: &PlayState, h: ActorHandle) -> &Actor {
    w.actors.actor(h).unwrap()
}

fn player(w: &PlayState) -> &Player {
    w.actors.downcast::<Player>(w.player.unwrap()).unwrap()
}

/// One frame with `pad`, then one released.
fn press(w: &mut PlayState, pad: PadState) {
    w.tick_with(scripted_input(PadState::default(), pad));
    w.tick_with(scripted_input(pad, PadState::default()));
}

fn a_button() -> PadState {
    PadState { button: BTN_A, ..Default::default() }
}

/// Link 20 from `h`, facing it, until it offers itself to his lift (`GI_NONE`).
fn offered(w: &mut PlayState, h: ActorHandle) {
    always_update(w, h);
    for _ in 0..40 {
        let c = base(w, h).world_pos;
        // Facing it a little off-axis: dead on, the C's ABS of 0x8000 wraps to -0x8000 in an s16
        // and no offer is taken.
        w.place_player(Vec3::new(c.x + 2.0, c.y, c.z - 20.0), -0x200);
        idle(w, 1);
        if player(w).interact_range_actor == Some(h) {
            return;
        }
    }
    panic!("{} never offered itself", w.actors.get(h).unwrap().name());
}

#[test]
fn a_cucco_is_lifted_carried_and_thrown() {
    let Some(a) = assets() else { return };
    // The cucco by the bazaar (params -1: kind 0, wandering about home, func_80AB6570) offers
    // itself to Player's lift (Actor_OfferCarry: Actor_OfferGetItemNearby with GI_NONE).
    let mut w = kakariko(&a, |_| {});
    let h = at::<EnNiw>(&w, -247.0, 854.0);
    offered(&mut w, h);
    // A: Player_ActionHandler_2's lift: PLAYER_STATE1_CARRYING_ACTOR, the item put away
    // (Player_SetupWaitForPutAway, nothing in hand: func_8083A0F4 at once), then
    // Player_Action_80846050 with PLAYER_ANIMGROUP_carryB.
    press(&mut w, a_button());
    let p = player(&w);
    assert!(p.state1 & STATE1_11 != 0);
    assert_eq!(p.action, PA::Lift, "lifting");
    // On frame 4 Player takes it: heldActor, the cucco's parent.
    let mut n = 0;
    while player(&w).held_actor.is_none() {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 10, "never took the cucco");
    }
    assert_eq!(player(&w).held_actor, Some(h));
    assert_eq!(base(&w, h).parent, w.player);
    // At the animation's end: standing (func_80839F90), carrying (Player_UpperAction_CarryActor).
    n = 0;
    while player(&w).action == PA::Lift {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 60);
    }
    assert_eq!(player(&w).upper, UpperAction::Carry);
    // Held between the hands (sGetItemRefPos), over Link's head.
    idle(&mut w, 2);
    let (cucco, link) = (base(&w, h).world_pos, player(&w).actor.world_pos);
    assert!(cucco.y > link.y + 20.0, "the cucco at {cucco}, Link at {link}");
    // A: the cucco is throw-only (ACTOR_FLAG_THROW_ONLY): Player_Action_80846578. On frame 3
    // func_8084409C: along Link's facing, speed his + 8, 12 up; let go.
    let yaw = player(&w).actor.shape_rot.y;
    press(&mut w, a_button());
    assert_eq!(player(&w).action, PA::Throw);
    n = 0;
    while base(&w, h).parent.is_some() {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 10, "never thrown");
    }
    assert_eq!(player(&w).held_actor, None);
    assert!(player(&w).state1 & STATE1_11 == 0);
    assert_eq!(base(&w, h).world_rot.y, yaw);
}

/// Lifts `h` (offered, A) and waits for the lift's end, carrying it.
fn lift(w: &mut PlayState, h: ActorHandle) {
    offered(w, h);
    press(w, a_button());
    let mut n = 0;
    while player(w).action != PA::StandingStill || player(w).held_actor.is_none() {
        idle(w, 1);
        n += 1;
        assert!(n < 80, "never carried it: {:?}", player(w).action);
    }
}

#[test]
fn a_cucco_carried_slows_the_fall() {
    let Some(a) = assets() else { return };
    // Player_UpperAction_CarryActor: a cucco held while velocity.y <= 0: minVelocityY -2, gravity
    // -0.5, and fallStartHeight follows Link down (so no fall damage).
    let mut w = kakariko(&a, |_| {});
    let h = at::<EnNiw>(&w, -247.0, 854.0);
    lift(&mut w, h);
    let p = player(&w).actor.world_pos;
    w.place_player(Vec3::new(p.x, p.y + 300.0, p.z), player(&w).actor.shape_rot.y);
    idle(&mut w, 30);
    let pl = player(&w);
    assert_eq!(pl.action, PA::Midair);
    assert_eq!(pl.actor.velocity.y, -2.0);
    assert_eq!(pl.actor.gravity, -0.5);
    assert_eq!(pl.fall_start_height, pl.actor.world_pos.y as i16);
    let health = w.save.health;
    let mut n = 0;
    while !player(&w).grounded() {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 400);
    }
    idle(&mut w, 5);
    assert_eq!(w.save.health, health, "no fall damage");
    assert_eq!(player(&w).held_actor, Some(h));
}

#[test]
fn a_small_rock_is_lifted_and_breaks_where_it_lands() {
    let Some(a) = assets() else { return };
    // En_Ishi 0x0200 by the gate (220, 400, -1236): a small rock (type 0), drop table 2. Within 90
    // EnIshi_Wait offers it (50 across, 10 up). Player_GetStrength: no strength, but a small rock
    // needs none: Player_Action_80846050.
    let mut w = kakariko(&a, |_| {});
    let h = at::<oot_actors::en_ishi::EnIshi>(&w, 220.0, -1236.0);
    always_update(&mut w, h);
    lift(&mut w, h);
    // EnIshi_Wait saw its parent: EnIshi_SetupLiftedUp (room -1).
    assert_eq!(base(&w, h).room, -1);
    // Carried west, away from the cliff east of it (thrown within 35 of a wall it breaks at once:
    // Actor_UpdateBgCheckInfo's wall radius), and thrown (throw-only): EnIshi_LiftedUp,
    // EnIshi_SetupFly, EnIshi_Fly until the ground or a wall: its pieces, its sound and dust,
    // Actor_Kill.
    w.place_player(Vec3::new(120.0, 400.0, -1236.0), -0x4000);
    idle(&mut w, 3);
    press(&mut w, a_button());
    let mut n = 0;
    while w.actors.actor(h).is_some_and(|a| !a.killed) {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 60, "the rock never landed");
    }
    assert!(n > 3, "it flew a few frames");
}

#[test]
fn a_child_without_strength_only_tugs_at_a_bush() {
    let Some(a) = assets() else { return };
    // En_Kusa 0x0200 at (-887, 0, 467): EnKusa_Main offers it within 100 (Actor_OfferCarry).
    // func_8083A0F4: En_Kusa with Player_GetStrength() <= PLAYER_STR_NONE: Player_Action_80846408,
    // Link 20 from it facing it (yawTowardsPlayer + 0x8000), link_normal_nocarry_free; then the
    // wait for 15 frames, link_normal_nocarry_free_end and PLAYER_STATE1_CARRYING_ACTOR off.
    let mut w = kakariko(&a, |_| {});
    let h = at::<oot_actors::en_kusa::EnKusa>(&w, -887.0, 467.0);
    always_update(&mut w, h);
    offered(&mut w, h);
    press(&mut w, a_button());
    assert_eq!(player(&w).action, PA::NoCarry);
    let (bush, link) = (base(&w, h).world_pos, player(&w).actor.world_pos);
    assert!(((bush.x - link.x).hypot(bush.z - link.z) - 20.0).abs() < 0.5, "20 from the bush");
    let mut n = 0;
    while player(&w).state1 & STATE1_11 != 0 {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 120);
    }
    assert!(base(&w, h).parent.is_none(), "still in the ground");
}

#[test]
fn with_the_bracelet_a_bush_is_pulled_up_and_thrown() {
    let Some(a) = assets() else { return };
    // A strength upgrade (the Goron's Bracelet): PLAYER_STR_BRACELET for a child, so the bush is
    // lifted (Player_Action_80846050): EnKusa_Main sees its parent (EnKusa_SetupLiftedUp,
    // NA_SE_PL_PULL_UP_PLANT); thrown, EnKusa_LiftedUp then EnKusa_Fall until it lands: type 0
    // goes (Actor_Kill).
    let mut w = kakariko(&a, |s| s.inventory_change_upgrade(oot_game::item::UPG_STRENGTH, 1));
    let h = at::<oot_actors::en_kusa::EnKusa>(&w, -887.0, 467.0);
    always_update(&mut w, h);
    lift(&mut w, h);
    assert_eq!(base(&w, h).room, -1);
    press(&mut w, a_button());
    let mut n = 0;
    while w.actors.actor(h).is_some_and(|a| !a.killed) {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 80, "the bush never landed");
    }
}

// The exit's run.

#[test]
fn exit_three_cuccos_penned_and_the_gate_opened_by_the_letter() {
    use oot_actors::playthrough::{Playthrough, Route, Step};
    let Some(a) = assets() else { return };
    let route = Route::Kakariko;
    let e = a.scenes.entrance_index(route.entrance()).expect("entrance");
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), route.save(e)).expect("Play_Init");
    route.debug_start(&mut w);
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
    }
    assert!(run.failure.is_none(), "{:?}", run.failure);
    eprintln!("steps {:?}", run.steps);
    use Step::*;
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![CuccoLifted, CuccoPenned, CuccoLifted, CuccoPenned, CuccoLifted, Glided, CuccoPenned, LetterShown, GateOpen]);
    // Kakariko's intro played on the arrival (sEntranceCutsceneTable: EVENTCHKINF_A3).
    assert!(w.save.get_event_chk_inf(EVENTCHKINF_A3));
    // Anju counted them in her pen (func_80ABA244): sKakarikoPosList's 0 (by the way in), 2 (east
    // of the pen) and 1 (the landing below the gate's stairs, glided down off the low roof), so
    // INFTABLE_199, _19B and _19A (D_80ABB3B4) are set, the others not.
    for (flag, set) in [(0x199u16, true), (0x19A, true), (0x19B, true), (0x19C, false), (0x19D, false), (0x19E, false), (0x19F, false)] {
        assert_eq!(w.save.get_inf_table(flag), set, "INFTABLE_{flag:X}");
    }
    // The gate guard took the letter (EnHeishi2's 0x2010, then 0x2015): INFTABLE_76, and the gate
    // slid open to x -89 (func_80878300).
    assert_eq!(run.texts, vec![0x2010, 0x2015]);
    assert!(w.save.get_inf_table(oot_actors::bg_gate_shutter::INFTABLE_76));
    let gate = all::<oot_actors::bg_gate_shutter::BgGateShutter>(&w)[0];
    assert!(base(&w, gate).world_pos.x <= -89.0);
}
