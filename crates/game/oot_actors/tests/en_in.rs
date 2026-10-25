//! GAME-08 milestone 3 against the C: the child's Ingo (`z_en_in.c`): which Ingo lives where for a
//! child (`EnIn_GetStartMode`), his texts and the flags they set (`EnIn_GetTextIdChild`,
//! `EnIn_UpdateTalkStateOnClosing`), his tracking, `sIsObedient`'s reset of the horses' `eventInf`
//! word; and the adult's Ingos (GAME-12 milestone 2): who stays by the race's state, and the
//! rental's minute running out. (The rental and the race: `ingo_runs.rs`.) Adult Malon's init is
//! `en_ma2.rs`'s.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_in::{self, EnIn};
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::message::{MSGMODE_TEXT_AWAIT_INPUT, TEXT_STATE_AWAITING_NEXT, TEXT_STATE_CHOICE, TEXT_STATE_DONE, TEXT_STATE_DONE_HAS_NEXT, TEXT_STATE_NONE};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

/// `EVENTCHKINF_B4` (`save.h`): the ranch's intro seen (`sEntranceCutsceneTable`, both ages).
const EVENTCHKINF_B4: u16 = 0xB4;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Link (`adult` or a child) entered at `entrance` at `hour`:00 with `preset`, the ranch's intro
/// seen, `edit` applied to the save last; four frames on (the room's actors are in on the third,
/// and each Ingo's first update, `EnIn_WaitForObject`, is the fourth).
fn enter_age(a: &Arc<GameAssets>, entrance: &str, adult: bool, hour: i32, preset: &str, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, adult, oot_game::env::clock_time(hour, 0) as u16);
    save.apply_preset(preset).unwrap();
    save.set_event_chk_inf(EVENTCHKINF_B4);
    edit(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    idle(&mut w, 4);
    w
}

/// Before Talon's return (`castle-egg`) or after it (`castle-courtyard`:
/// `EVENTCHKINF_TALON_RETURNED_FROM_CASTLE`).
fn preset(talon_back: bool) -> &'static str {
    if talon_back { "castle-courtyard" } else { "castle-egg" }
}

fn ranch(a: &Arc<GameAssets>, hour: i32, talon_back: bool) -> PlayState {
    enter_age(a, "ENTR_LON_LON_RANCH_0", false, hour, preset(talon_back), |_| {})
}

fn stable(a: &Arc<GameAssets>, hour: i32, talon_back: bool) -> PlayState {
    enter_age(a, "ENTR_STABLE_0", false, hour, preset(talon_back), |_| {})
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn all<T: 'static + ActorImpl>(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<T>(h).is_some_and(|a| !a.base().killed)).collect()
}

fn one<T: 'static + ActorImpl>(w: &PlayState) -> ActorHandle {
    let v = all::<T>(w);
    assert_eq!(v.len(), 1, "one {}", std::any::type_name::<T>());
    v[0]
}

/// The live placeholders standing in for actor `id`, by where they were placed.
fn placeholders(w: &PlayState, id: i16) -> Vec<Vec3> {
    w.actors.all().into_iter().filter(|&h| w.actors.get(h).is_some_and(|a| a.name() == "Placeholder" && a.base().id == id && !a.base().killed)).map(|h| w.actors.actor(h).unwrap().home_pos).collect()
}

/// Any live actor of `id`, ported or a placeholder.
fn any_live(w: &PlayState, id: i16) -> usize {
    w.actors.all().into_iter().filter(|&h| w.actors.actor(h).is_some_and(|a| a.id == id && !a.killed)).count()
}

fn base(w: &PlayState, h: ActorHandle) -> &Actor {
    w.actors.actor(h).unwrap()
}

fn ingo(w: &PlayState, h: ActorHandle) -> &EnIn {
    w.actors.downcast::<EnIn>(h).unwrap()
}

/// Link `d` in front of `h` (along its facing), facing it.
fn before(w: &mut PlayState, h: ActorHandle, d: f32) {
    let a = base(w, h);
    let (p, yaw) = (a.world_pos, a.shape_rot.y);
    let r = yaw as f32 * std::f32::consts::PI / 32768.0;
    w.place_player(Vec3::new(p.x + d * r.sin(), p.y, p.z + d * r.cos()), yaw.wrapping_add(i16::MIN));
}

/// Link `d` in front of `h` until it's the actor Player would talk to.
fn offered(w: &mut PlayState, h: ActorHandle, d: f32) {
    for _ in 0..30 {
        before(w, h, d);
        idle(w, 1);
        if w.player().target_actor == Some(h) {
            return;
        }
    }
    panic!("{} never offered to talk", w.actors.get(h).unwrap().name());
}

/// A until the talk starts, then A whenever the box waits (once each wait) until it closes. The
/// texts shown.
fn talk(w: &mut PlayState) -> Vec<u16> {
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
        let waits = matches!(st, TEXT_STATE_AWAITING_NEXT | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_CHOICE) || mode == MSGMODE_TEXT_AWAIT_INPUT;
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

/// The one live Ingo, checked as `EnIn_WaitForObject` sets up a working one: `gIngoWorkingAnim`
/// (ENIN_ANIM_WORKING), `EnIn_Idle`, scale 0.01, ATTENTION_RANGE_6, rot.z `rot_z`; his sway
/// still (`EnIn_Fidget`'s `f` 0 while working).
fn working_ingo(w: &PlayState, rot_z: i16) -> ActorHandle {
    let h = one::<EnIn>(w);
    let i = ingo(w, h);
    assert_eq!(i.action, en_in::Action::Idle);
    assert!(i.skel.is(en_in::WORKING_ANIM), "{:?}", i.skel.animation.as_ref().map(|a| a.name.clone()));
    assert_eq!((i.actor.shape_rot.z, i.actor.scale.x, i.actor.target_mode, i.actor.params), (rot_z, 0.01, 6, -1));
    assert!(i.fidget_y.iter().chain(i.fidget_z.iter()).all(|&v| v == 0));
    h
}

// Where he is.

#[test]
fn ingo_works_in_the_ranch_by_day_until_talon_is_back() {
    let Some(a) = assets() else { return };
    // spot20 layer 0 (a child by day): En_In -1 at (670, 0, -1976), rot.z 1. EnIn_GetStartMode:
    // SCENE_LON_LON_RANCH, a child, by day, rot.z 1, no EVENTCHKINF_TALON_RETURNED_FROM_CASTLE:
    // ENIN_START_MODE_WORKING.
    let w = ranch(&a, 10, false);
    let h = working_ingo(&w, 1);
    assert_eq!((base(&w, h).home_pos.x, base(&w, h).home_pos.z), (670.0, -1976.0));
    // After Talon's return: ENIN_START_MODE_NULL, killed on his first update.
    let w = ranch(&a, 10, true);
    assert!(all::<EnIn>(&w).is_empty());
    assert_eq!(any_live(&w, en_in::ACTOR_EN_IN), 0);
    // By night (layer 1 has no Ingo).
    let w = ranch(&a, 22, false);
    assert_eq!(any_live(&w, en_in::ACTOR_EN_IN), 0);
}

#[test]
fn ingo_works_in_the_stable_by_day_after_talons_return_and_by_night() {
    let Some(a) = assets() else { return };
    // malon_stable: three Ingos -1 at (-35, 0, 95), rot.z 2, 3 and 4. A child by day after
    // Talon's return: rot.z 3 works; 2 and 4 (night's) are killed.
    let w = stable(&a, 10, true);
    working_ingo(&w, 3);
    // By day before it: none.
    let w = stable(&a, 10, false);
    assert_eq!(any_live(&w, en_in::ACTOR_EN_IN), 0);
    // By night: rot.z 2 before Talon's return, 4 after.
    let w = stable(&a, 22, false);
    working_ingo(&w, 2);
    let w = stable(&a, 22, true);
    working_ingo(&w, 4);
}

#[test]
fn no_ingo_in_talons_house_for_a_child() {
    let Some(a) = assets() else { return };
    // souko room 2: En_In -1 at (1568, 164, -140), rot.z 8 and 6 (the adult's asleep Ingo):
    // EnIn_GetStartMode needs an adult by night there, so ENIN_START_MODE_NULL for a child. Room 2
    // is behind transition 2's door (from room 1, at (1220, 140, 150)).
    for (hour, talon_back) in [(10, true), (22, true), (22, false)] {
        let mut w = enter_age(&a, "ENTR_LON_LON_BUILDINGS_2", false, hour, preset(talon_back), |_| {});
        idle(&mut w, 30);
        w.place_player(Vec3::new(1190.0, 140.0, 150.0), 0x4000);
        idle(&mut w, 1);
        let a_btn = PadState { button: BTN_A, ..Default::default() };
        w.tick_with(scripted_input(PadState::default(), a_btn));
        let mut spawned = false;
        for _ in 0..120 {
            idle(&mut w, 1);
            if w.room_ctx.cur.num == 2 {
                spawned = w.actors.all().into_iter().any(|h| w.actors.actor(h).is_some_and(|x| x.id == en_in::ACTOR_EN_IN));
                if spawned {
                    break;
                }
            }
        }
        assert!(spawned, "room 2's Ingos never spawned (hour {hour})");
        // Their init waits for room 2's object_in to load (Actor_UpdateAll's deferred init); their
        // first update (EnIn_WaitForObject) kills them.
        let pending = |w: &PlayState| {
            w.actors.all().into_iter().filter_map(|h| w.actors.get(h)).any(|x| x.base().id == en_in::ACTOR_EN_IN && !x.base().killed && x.as_any().downcast_ref::<EnIn>().is_none_or(|i| i.action == en_in::Action::WaitForObject))
        };
        let mut frames = 0;
        while pending(&w) {
            idle(&mut w, 1);
            frames += 1;
            assert!(frames < 120, "hour {hour}: still waiting");
        }
        assert_eq!(any_live(&w, en_in::ACTOR_EN_IN), 0, "hour {hour}");
    }
}

#[test]
fn an_adults_ranch_by_day_has_the_rentals_ingo() {
    let Some(a) = assets() else { return };
    // spot20 layer 2 (an adult by day): rot.z 7 -1 at (854, -2429); rot.z 5 params 2, 3 and 4 at
    // (-158, -1596), (-154, -1439), (899, -2536). No Epona, race state 0 (OFFER_RENTAL):
    // ENIN_START_MODE_RUNNING_RANCH keeps params 2 (EnIn_StartRentalPeriod,
    // ENIN_ANIM_STANDING_FOOT_FORWARD); 3 and 4 are killed; rot.z 7 is ENIN_START_MODE_NULL, killed.
    let w = enter_age(&a, "ENTR_LON_LON_RANCH_0", true, 10, "deku-tree-dead", |_| {});
    let h = one::<EnIn>(&w);
    let i = ingo(&w, h);
    assert_eq!((i.actor.home_pos.x, i.actor.home_pos.z, i.actor.params), (-158.0, -1596.0, 2));
    assert_eq!(i.action, en_in::Action::StartRentalPeriod);
    assert!(i.skel.is("gIngoStandingFootForwardAnim"));
    assert!(placeholders(&w, en_in::ACTOR_EN_IN).is_empty());
    // The rental (race state 1, in the corral: not by spawn 0, whose respawn point resets the word):
    // params 3 (EnIn_RentalPeriod, ATTENTION_RANGE_3, the timer at 60).
    let w = enter_age(&a, "ENTR_LON_LON_RANCH_6", true, 10, "ingo-rental", |s| s.event_inf[0] = en_in::INGO_RACE_STATE_HORSE_RENTAL_PERIOD);
    let h = one::<EnIn>(&w);
    let i = ingo(&w, h);
    assert_eq!((i.actor.home_pos.x, i.actor.home_pos.z, i.actor.params, i.actor.target_mode), (-154.0, -1439.0, 3, 3));
    assert_eq!(i.action, en_in::Action::RentalPeriod);
    assert_eq!(w.save.timer_state, oot_game::save::TIMER_STATE_DOWN_INIT);
    // Trapped by his loss (race state 6): params 4 (EnIn_PlayerWonSecondRace, losing).
    let w = enter_age(&a, "ENTR_LON_LON_RANCH_7", true, 10, "ingo-rental", |s| s.event_inf[0] = en_in::INGO_RACE_STATE_TRAPPED_WIN_EPONA);
    let h = one::<EnIn>(&w);
    assert_eq!((base(&w, h).home_pos.x, base(&w, h).params), (899.0, 4));
    // With Epona: rot.z 7 is ENIN_START_MODE_OBEDIENT (ENIN_ANIM_OBEDIENT, his eyes shut for good:
    // 3); the rot.z 5 ones need no Epona: ENIN_START_MODE_NULL.
    let w = enter_age(&a, "ENTR_LON_LON_RANCH_0", true, 10, "deku-tree-dead", |s| s.set_event_chk_inf(en_in::EVENTCHKINF_EPONA_OBTAINED));
    let h = one::<EnIn>(&w);
    let i = ingo(&w, h);
    assert_eq!((i.actor.home_pos.x, i.actor.home_pos.z), (854.0, -2429.0));
    assert_eq!((i.action, i.eye_index), (en_in::Action::Idle, 3));
    assert!(i.skel.is("gIngoObedientAnim"));
}

#[test]
fn the_rentals_minute_runs_out() {
    let Some(a) = assets() else { return };
    // The rental (race state 1, in the corral at ENTR_LON_LON_RANCH_6), Link having ridden
    // (INFTABLE_AB) on his first rental (no EVENTCHKINF_1B).
    let mut w = enter_age(&a, "ENTR_LON_LON_RANCH_6", true, 10, "adult-awake", |s| {
        s.event_inf[0] = en_in::INGO_RACE_STATE_HORSE_RENTAL_PERIOD;
        s.set_inf_table(en_in::INFTABLE_AB);
    });
    let h = one::<EnIn>(&w);
    assert_eq!(ingo(&w, h).action, en_in::Action::RentalPeriod);
    // The timer out (TIMER_STATE_STOP): EnIn_RentalTimeExpired: a sub camera on him, 0x2025, Link
    // put 100 before him and held (freezeTimer 10), the HUD gone; EnIn_EndRentalPeriod.
    w.save.timer_state = oot_game::save::TIMER_STATE_STOP;
    idle(&mut w, 1);
    let i = ingo(&w, h);
    assert_eq!(i.action, en_in::Action::EndRentalPeriod);
    assert_eq!(w.save.timer_state, 0);
    assert_eq!(w.msg_ctx.text_id, 0x2025);
    assert_ne!(w.active_cam_id, oot_game::camera::CAM_ID_MAIN);
    // His yaw then is the one to the camera's eye, straight along +z (the eye 40 out in z over
    // him; his tracking turns him again later in the update): Link 100 out in +z.
    let p = i.actor.world_pos;
    let link = w.player().actor.world_pos;
    assert!((link.x - p.x).abs() < 1.0 && (link.z - (p.z + 100.0)).abs() < 1.0, "{link} {p}");
    assert_eq!(i.actor.flags & oot_game::actor::ACTOR_FLAG_ATTENTION_ENABLED, 0);
    // 0x2025 read: back to the gate (EnIn_StartTransition 0: ENTR_LON_LON_RANCH_8), the race's
    // state OFFER_RENTAL, and the first ridden rental's EVENTCHKINF_1B.
    let a_btn = PadState { button: BTN_A, ..Default::default() };
    let mut prev = PadState::default();
    for _ in 0..600 {
        // TRANS_TRIGGER_START (the corral's own fade-in is still TRANS_TRIGGER_END).
        if w.transition.trigger == oot_game::transition::TRANS_TRIGGER_START {
            break;
        }
        let pad = if prev.button == 0 && w.message_state() != TEXT_STATE_NONE { a_btn } else { PadState::default() };
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
    }
    assert_eq!(w.transition.next_entrance_index, a.scenes.entrance_index("ENTR_LON_LON_RANCH_8").unwrap());
    assert_eq!(en_in::race_state(&w), en_in::INGO_RACE_STATE_OFFER_RENTAL);
    assert!(w.save.get_event_chk_inf(en_in::EVENTCHKINF_1B));
    assert!(!w.save.r_exited_scene_riding_horse);
}

// The horses' eventInf word.

#[test]
fn entering_the_ranch_by_its_entrance_resets_the_horses_event_inf() {
    let Some(a) = assets() else { return };
    // Player_Init sets respawn[RESPAWN_MODE_DOWN] to spawn 0's (1107, 0, -3740); EnIn_Init, the
    // first time (sIsObedient), clears eventInf[EVENTINF_INDEX_HORSES]. The other words stay.
    let w = enter_age(&a, "ENTR_LON_LON_RANCH_0", false, 10, preset(false), |s| s.event_inf = [0x1234, 0x0055, 0, 0]);
    let r = w.save.respawn[oot_game::save::RESPAWN_MODE_DOWN].pos;
    assert_eq!((r.x, r.y, r.z), (1107.0, 0.0, -3740.0));
    assert_eq!(w.save.event_inf, [0, 0x0055, 0, 0]);
    // By night (layer 1) the ranch has no Ingo, so no init: no reset.
    let w = enter_age(&a, "ENTR_LON_LON_RANCH_0", false, 22, preset(false), |s| s.event_inf[0] = 0x1234);
    assert_eq!(w.save.event_inf[0], 0x1234);
    // An adult by day (layer 2: four Ingos, the first init resets it; three are then killed, as
    // below).
    let w = enter_age(&a, "ENTR_LON_LON_RANCH_0", true, 10, "deku-tree-dead", |s| s.event_inf[0] = 0x1234);
    assert_eq!(w.save.event_inf[0], 0);
    // In the stable (its own spawn): no reset.
    let w = enter_age(&a, "ENTR_STABLE_0", false, 10, preset(true), |s| s.event_inf[0] = 0x1234);
    assert_eq!(w.save.event_inf[0], 0x1234);
}

// The texts.

#[test]
fn ingo_before_talons_return_says_0x203f_then_0x2040() {
    let Some(a) = assets() else { return };
    // EnIn_GetTextIdChild: no EVENTCHKINF_TALON_RETURNED_FROM_CASTLE, no
    // INFTABLE_INGO_TALKED_TO_CHILD_LINK_BEFORE_TALON_RETURNED: 0x203F. Its closing
    // (EnIn_UpdateTalkStateOnClosing) sets EVENTCHKINF_11 and the INFTABLE flag; then 0x2040.
    let mut w = ranch(&a, 10, false);
    let h = one::<EnIn>(&w);
    offered(&mut w, h, 50.0);
    assert_eq!(base(&w, h).text_id, 0x203F);
    assert!(!w.save.get_inf_table(en_in::INFTABLE_INGO_TALKED_TO_CHILD_LINK_BEFORE_TALON_RETURNED));
    assert!(!w.save.get_event_chk_inf(en_in::EVENTCHKINF_11));
    // 0x203F goes on to 0x2040 in the same box (the message's own next text); the actor's textId
    // stays 0x203F, whose closing sets the flags.
    assert_eq!(talk(&mut w), vec![0x203F, 0x2040]);
    assert!(w.save.get_inf_table(en_in::INFTABLE_INGO_TALKED_TO_CHILD_LINK_BEFORE_TALON_RETURNED));
    assert!(w.save.get_event_chk_inf(en_in::EVENTCHKINF_11));
    // The talk over: NPC_TALK_STATE_IDLE again.
    assert_eq!(ingo(&w, h).interact_info.talk_state, en_in::NPC_TALK_STATE_IDLE);
    offered(&mut w, h, 50.0);
    assert_eq!(base(&w, h).text_id, 0x2040);
    assert_eq!(talk(&mut w), vec![0x2040]);
    // 0x2040 sets nothing more.
    assert!(!w.save.get_inf_table(en_in::INFTABLE_INGO_TALKED_TO_CHILD_LINK_AFTER_TALON_RETURNED));
}

#[test]
fn ingo_after_talons_return_says_0x2045_then_0x2046() {
    let Some(a) = assets() else { return };
    // EnIn_GetTextIdChild: EVENTCHKINF_TALON_RETURNED_FROM_CASTLE, no
    // INFTABLE_INGO_TALKED_TO_CHILD_LINK_AFTER_TALON_RETURNED: 0x2045, whose closing sets it;
    // then 0x2046. In the stable by day (rot.z 3).
    let mut w = stable(&a, 10, true);
    let h = one::<EnIn>(&w);
    offered(&mut w, h, 50.0);
    assert_eq!(base(&w, h).text_id, 0x2045);
    // 0x2045 goes on to 0x2046 in the same box; its closing (textId 0x2045) sets the flag.
    assert_eq!(talk(&mut w), vec![0x2045, 0x2046]);
    assert!(w.save.get_inf_table(en_in::INFTABLE_INGO_TALKED_TO_CHILD_LINK_AFTER_TALON_RETURNED));
    assert!(!w.save.get_event_chk_inf(en_in::EVENTCHKINF_11));
    offered(&mut w, h, 50.0);
    assert_eq!(base(&w, h).text_id, 0x2046);
    assert_eq!(talk(&mut w), vec![0x2046]);
}

#[test]
fn ingo_turns_his_head_to_link_while_talking() {
    let Some(a) = assets() else { return };
    // EnIn_UpdateTracking: preset 1 and NPC_TRACKING_PLAYER_AUTO_TURN; talking
    // (Npc_UpdateAutoTurn: talkState != 0) is mode 4 (the whole body). Link off to his side, the
    // head turns towards him.
    let mut w = stable(&a, 10, true);
    let h = one::<EnIn>(&w);
    offered(&mut w, h, 50.0);
    let a_btn = PadState { button: BTN_A, ..Default::default() };
    w.tick_with(scripted_input(PadState::default(), a_btn));
    idle(&mut w, 10);
    let i = ingo(&w, h);
    assert_eq!(i.interact_info.talk_state, en_in::NPC_TALK_STATE_TALKING);
    assert_eq!(i.interact_info.mode, 4);
    assert_eq!(i.interact_info.eye_height, 16.0);
}
