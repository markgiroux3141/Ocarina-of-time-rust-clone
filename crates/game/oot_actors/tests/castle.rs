//! GAME-07 milestone 2 against the C: Hyrule Castle's grounds. Talon (`z_en_ta.c`) asleep by the
//! crates and woken by the Cucco Player shows him (`Player_ActionHandler_13`'s exchange), his run
//! home; the milk crates (`z_bg_spot15_rrbox.c`), the sentries (`z_en_heishi3.c`) and their
//! throw-out (`TransitionCircle`, `z_fbdemo_circle.c`), the gate guard and Lon Lon's fence
//! (`z_en_heishi2.c`, `z_bg_spot15_saku.c`); Kakariko's gate and its guard (`z_bg_gate_shutter.c`),
//! the pots (`z_obj_tsubo.c`), the torch (`z_en_light.c`); the moat's water camera
//! (`Camera_UpdateWater`, `Camera_Jump3`), `Player_UpdateBottleHeld`; then the exit's runs
//! (`Route::Castle`, `Route::CastleCaught`).
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_CLEFT, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_gate_shutter::{self, BgGateShutter};
use oot_actors::bg_spot15_rrbox::{self, BgSpot15Rrbox};
use oot_actors::bg_spot15_saku::{self, BgSpot15Saku};
use oot_actors::en_heishi2::EnHeishi2;
use oot_actors::en_heishi3::{self, EnHeishi3};
use oot_actors::en_light::{self, EnLight};
use oot_actors::en_ta::{self, EnTa};
use oot_actors::obj_tsubo::ObjTsubo;
use oot_actors::player::{Action as PA, Player};
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::transition::{TCA_STARBURST, TCC_WHITE, TCS_FAST, trans_type_circle};

/// `SCENE_HYRULE_CASTLE`, `SCENE_CASTLE_COURTYARD_GUARDS_DAY` (`scene_table.h`).
const SCENE_HYRULE_CASTLE: u16 = 0x5F;
const SCENE_CASTLE_COURTYARD_GUARDS_DAY: u16 = 0x45;

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

/// The castle's road by day with the Weird Egg (`castle-egg`).
fn castle(a: &Arc<GameAssets>, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    enter_with(a, "ENTR_HYRULE_CASTLE_0", 10, "castle-egg", edit)
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

/// The `T` placed at (`x`, `z`).
fn at<T: 'static + ActorImpl>(w: &PlayState, x: f32, z: f32) -> ActorHandle {
    let v: Vec<_> = all::<T>(w).into_iter().filter(|&h| w.actors.actor(h).is_some_and(|a| (a.home_pos.x - x).abs() < 1.0 && (a.home_pos.z - z).abs() < 1.0)).collect();
    assert_eq!(v.len(), 1, "one {} at ({x}, {z})", std::any::type_name::<T>());
    v[0]
}

/// Updated wherever the camera looks (the tests' actors are out of its culling volume).
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

// Talon.

#[test]
fn talon_sleeps_by_the_crates_until_woken_and_is_gone_once_home() {
    let Some(a) = assets() else { return };
    // EnTa_Init, "Other Talon" (params -1) in SCENE_HYRULE_CASTLE: asleep (gTalonSleepAnim, eyes
    // closed, the shadow 54) in EnTa_IdleAsleepInCastle; scale 0.01, ATTENTION_RANGE_6.
    let w = castle(&a, |_| {});
    let h = one::<EnTa>(&w);
    let t = w.actors.downcast::<EnTa>(h).unwrap();
    assert_eq!(t.actor.params, en_ta::ENTA_CHILD_ERA);
    assert_eq!(t.action, en_ta::Action::IdleAsleepInCastle);
    assert_eq!(t.eye_index, en_ta::TALON_EYE_INDEX_CLOSED);
    assert!(t.skel.is("gTalonSleepAnim"));
    assert_eq!(t.actor.scale, Vec3::splat(0.01));
    assert_eq!(t.actor.target_mode, 6);
    // Woken (EVENTCHKINF_TALON_WOKEN_IN_CASTLE): standing (gTalonStandAnim), eyes open.
    let w = castle(&a, |s| s.set_event_chk_inf(en_ta::EVENTCHKINF_TALON_WOKEN_IN_CASTLE));
    let t = w.actors.downcast::<EnTa>(one::<EnTa>(&w)).unwrap();
    assert_eq!(t.action, en_ta::Action::IdleAwakeInCastle);
    assert_eq!(t.eye_index, en_ta::TALON_EYE_INDEX_OPEN);
    assert!(t.skel.is("gTalonStandAnim"));
    // Gone home (EVENTCHKINF_TALON_RETURNED_FROM_CASTLE): Actor_Kill.
    let w = castle(&a, |s| s.set_event_chk_inf(en_ta::EVENTCHKINF_TALON_RETURNED_FROM_CASTLE));
    assert!(all::<EnTa>(&w).is_empty());
}

#[test]
fn the_cucco_shown_wakes_talon() {
    let Some(a) = assets() else { return };
    // castle-cucco: the Cucco (ITEM_CHICKEN) on C-Left. Link 70 in front of Talon (he faces +x),
    // facing him: EnTa_IdleAsleepInCastle offers a talk for EXCH_ITEM_CHICKEN within 100
    // (Actor_OfferTalkExchange), Player takes him as talkActor.
    let mut w = enter_with(&a, "ENTR_HYRULE_CASTLE_0", 10, "castle-cucco", |_| {});
    let h = one::<EnTa>(&w);
    always_update(&mut w, h);
    let talon = base(&w, h).world_pos;
    w.place_player(Vec3::new(talon.x + 70.0, 1561.0, talon.z), -0x4000);
    idle(&mut w, 10);
    assert_eq!(player(&w).target_actor, Some(h));
    // C-Left: Player_ActionHandler_13's exchange (the item's exchange id, EXCH_ITEM_CHICKEN 3, is
    // the one offered: the talk accepted). EnTa_IdleAsleepInCastle: Player's text 0x702B, EnTa_WakeUp
    // with the timer at 40.
    press(&mut w, PadState { button: BTN_CLEFT, ..Default::default() });
    let mut frames = 0;
    while w.actors.downcast::<EnTa>(h).unwrap().action == en_ta::Action::IdleAsleepInCastle {
        idle(&mut w, 1);
        frames += 1;
        assert!(frames < 60, "Talon never took the Cucco");
    }
    let t = w.actors.downcast::<EnTa>(h).unwrap();
    assert_eq!(t.action, en_ta::Action::WakeUp);
    assert_eq!(player(&w).action, PA::ExchangeItem);
    assert_eq!(player(&w).actor.text_id, 0x702B);
    // The timer counts down at the end of each EnTa_Update: 40 updates on he wakes
    // (EnTa_TalkWakingUp1, gTalonWakeUpAnim, three quick blinks, the timer at 60).
    assert_eq!(t.timer, 39);
    idle(&mut w, 40);
    let t = w.actors.downcast::<EnTa>(h).unwrap();
    assert_eq!(t.action, en_ta::Action::TalkWakingUp1);
    assert!(t.skel.is("gTalonWakeUpAnim"));
    assert_eq!(t.rapid_blinks, 3);
}

#[test]
fn talons_run_home_takes_205_frames() {
    let Some(a) = assets() else { return };
    // EnTa_TalkAwakeInCastle on the text's event: EnTa_RunAwayStart with the timer at 5 (4 once
    // the update's countdown has run). Each state changes when its timer reads 0: RunAwayStart 5,
    // RunSouth 65, TurnWest 5, RunWest 65, TurnTowardsGate 5, RunOutOfGate 60: Actor_Kill 205
    // updates after the event's.
    let mut w = castle(&a, |s| s.set_event_chk_inf(en_ta::EVENTCHKINF_TALON_WOKEN_IN_CASTLE));
    let h = one::<EnTa>(&w);
    always_update(&mut w, h);
    {
        let t = w.actors.downcast_mut::<EnTa>(h).unwrap();
        t.action = en_ta::Action::RunAwayStart;
        t.timer = 4;
    }
    let mut states = Vec::new();
    let mut n = 0;
    loop {
        idle(&mut w, 1);
        n += 1;
        match w.actors.downcast::<EnTa>(h) {
            Some(t) if !t.actor.killed => {
                if states.last().map(|s: &(en_ta::Action, i32)| s.0) != Some(t.action) {
                    states.push((t.action, n));
                }
            }
            _ => break,
        }
        assert!(n < 400);
    }
    use en_ta::Action::*;
    assert_eq!(states, vec![(RunAwayStart, 1), (RunAwayRunSouth, 5), (RunAwayTurnWest, 70), (RunAwayRunWest, 75), (RunAwayTurnTowardsGate, 140), (RunAwayRunOutOfGate, 145)]);
    // Killed on update 205 (gone from the list the frame after).
    assert!(n == 205 || n == 206, "{n}");
}

// The milk crates.

/// Pushes the crate with Player's force (`func_8002DFA4`: `unk_150`, `unk_158`) each frame for `n`
/// frames.
fn push(w: &mut PlayState, h: ActorHandle, yaw: i16, n: usize) {
    for _ in 0..n {
        let bg = w.actors.downcast::<BgSpot15Rrbox>(h).unwrap().bg;
        w.col.dyna.func_8002DFA4(bg, 2.0, yaw);
        idle(w, 1);
    }
}

#[test]
fn a_crate_by_talons_place_goes_west_or_south_only_once_he_has_gone() {
    let Some(a) = assets() else { return };
    // func_808B3AAC: with x <= 930 and z >= -360 (by Talon), a push whose yaw is between -0x6000
    // and 0x2000 (west; yaw 0 is +z, south) needs EVENTCHKINF_TALON_RETURNED_FROM_CASTLE; others go.
    for (flag, yaw, moves) in [(false, -0x4000i16, false), (false, 0, false), (true, -0x4000, true), (true, 0, true), (false, 0x4000, true), (false, i16::MIN, true)] {
        let mut w = castle(&a, |s| {
            if flag {
                s.set_event_chk_inf(bg_spot15_rrbox::EVENTCHKINF_TALON_RETURNED_FROM_CASTLE);
            }
        });
        let h = at::<BgSpot15Rrbox>(&w, 910.0, -519.0);
        always_update(&mut w, h);
        // Moved by Talon's place, as a push south would leave it (for a push south, 56 short of
        // it: from -299 a wall south of his spot stops it, func_800435D8).
        let z0 = if yaw == 0 { -355.0 } else { -299.0 };
        {
            let c = w.actors.downcast_mut::<BgSpot15Rrbox>(h).unwrap();
            c.actor.world_pos.z = z0;
            c.actor.home_pos.z = z0;
        }
        idle(&mut w, 1);
        // One update takes the push (func_808B4178), then the slide: 0.5 faster a frame up to 2
        // (0.5, 1, 1.5, 2, ...: 20 on the 12th), then at rest again with a 10-frame wait.
        push(&mut w, h, yaw, 1);
        idle(&mut w, 12);
        let c = w.actors.downcast::<BgSpot15Rrbox>(h).unwrap();
        // 20 along the push (Math_SinS, Math_CosS of the yaw).
        let r = eng_math::binang_to_rad(yaw);
        let (dx, dz) = ((r.sin() * 20.0).round(), (r.cos() * 20.0).round());
        let (x, z) = (c.actor.world_pos.x.round(), c.actor.world_pos.z.round());
        if moves {
            assert_eq!((x, z), (910.0 + dx, z0 + dz), "flag {flag} yaw {yaw:#x}");
            assert_eq!(c.action, bg_spot15_rrbox::Action::Rest);
            assert_eq!(c.unk_168, 10);
        } else {
            assert_eq!((x, z), (910.0, z0), "flag {flag} yaw {yaw:#x}");
            // The push dropped: unk_150 zeroed.
            assert_eq!(w.col.dyna.unk_150(c.bg), 0.0);
        }
    }
}

#[test]
fn crates_with_their_switch_flag_start_stacked_in_the_moat() {
    let Some(a) = assets() else { return };
    // BgSpot15Rrbox_Init: with the switch flag (params & 0x3F: both crates' 4) set, D_808B45C4[n]
    // for the n-th (770, 1490, -299), (770, 1550, -299), pushes ignored (func_808B44CC).
    let w = castle(&a, |s| s.scene_flags[SCENE_HYRULE_CASTLE as usize].swch |= 1 << 4);
    let mut crates: Vec<_> = all::<BgSpot15Rrbox>(&w).into_iter().map(|h| w.actors.downcast::<BgSpot15Rrbox>(h).unwrap()).collect();
    assert_eq!(crates.len(), 2);
    crates.sort_by(|a, b| a.actor.world_pos.y.total_cmp(&b.actor.world_pos.y));
    for (c, p) in crates.iter().zip(bg_spot15_rrbox::STACKED) {
        assert_eq!(c.actor.params & 0x3F, 4);
        assert_eq!(c.action, bg_spot15_rrbox::Action::Stuck);
        assert_eq!(c.actor.world_pos, p);
    }
    // Without it, where they're placed.
    let w = castle(&a, |_| {});
    assert_eq!(base(&w, at::<BgSpot15Rrbox>(&w, 950.0, -459.0)).world_pos, Vec3::new(950.0, 1561.0, -459.0));
    assert_eq!(base(&w, at::<BgSpot15Rrbox>(&w, 910.0, -519.0)).world_pos, Vec3::new(910.0, 1561.0, -519.0));
}

// The sentries.

/// A grounds sentry at `guard` (its placement) sees Link put `dist` from it at `angle` off its
/// facing, at `hour`:00?
fn sentry_sees(a: &Arc<GameAssets>, hour: i32, guard: Vec3, angle: i16, dist: f32) -> bool {
    let mut w = enter_with(a, "ENTR_HYRULE_CASTLE_0", hour, "castle-egg", |_| {});
    let h = at::<EnHeishi3>(&w, guard.x, guard.z);
    always_update(&mut w, h);
    // The others gone (one catch is all: sPlayerCaught).
    for o in all::<EnHeishi3>(&w) {
        if o != h {
            w.actors.actor_mut(o).unwrap().kill();
        }
    }
    let yaw = base(&w, h).shape_rot.y.wrapping_add(angle);
    let r = eng_math::binang_to_rad(yaw);
    let p = guard + Vec3::new(r.sin(), 0.0, r.cos()) * dist;
    let (y, _) = w.col.entity_raycast_down(Vec3::new(p.x, guard.y + 150.0, p.z));
    // On the guard's level (EnHeishi3_StandSentinelInGrounds also wants within 100 up or down).
    assert!((y - guard.y).abs() < 60.0, "the floor at {p} is {y}");
    w.place_player(Vec3::new(p.x, y, p.z), yaw.wrapping_add(i16::MIN));
    idle(&mut w, 3);
    let caught = w.actors.downcast::<EnHeishi3>(h).unwrap().action != en_heishi3::Action::StandSentinelInGrounds;
    if caught {
        // "Hey you!", Link held (PLAYER_CSACTION_1).
        assert_eq!(w.msg_ctx.text_id, 0x702D);
    }
    caught
}

#[test]
fn a_sentry_sees_further_in_front_and_by_day() {
    let Some(a) = assets() else { return };
    // EnHeishi3_StandSentinelInGrounds: within 250 in front (|yaw to Link - facing| < 0x4300) by
    // day, 200 by night; 150 and 100 behind. The sentry on the field's east slope (level all
    // round).
    let g = Vec3::new(1554.0, 1393.0, 2271.0);
    assert!(sentry_sees(&a, 10, g, 0, 240.0));
    assert!(!sentry_sees(&a, 10, g, 0, 260.0));
    assert!(sentry_sees(&a, 10, g, i16::MIN, 140.0));
    assert!(!sentry_sees(&a, 10, g, i16::MIN, 160.0));
    assert!(sentry_sees(&a, 22, g, 0, 190.0));
    assert!(!sentry_sees(&a, 22, g, 0, 210.0));
    assert!(sentry_sees(&a, 22, g, i16::MIN, 90.0));
    assert!(!sentry_sees(&a, 22, g, i16::MIN, 110.0));
}

#[test]
fn the_castle_doors_guards_watch_their_halves_of_the_box() {
    let Some(a) = assets() else { return };
    // EnHeishi3_Init: params > 0 are the castle door's (unk_278 1 east of x -290, 2 west);
    // EnHeishi3_StandSentinelInCastle catches Link in x -380..-190, z 700..1020 on its half.
    let w = castle(&a, |_| {});
    let east = at::<EnHeishi3>(&w, -215.0, 915.0);
    let west = at::<EnHeishi3>(&w, -354.0, 915.0);
    assert_eq!(w.actors.downcast::<EnHeishi3>(east).unwrap().unk_278, 1);
    assert_eq!(w.actors.downcast::<EnHeishi3>(west).unwrap().unk_278, 2);
    assert_eq!(w.actors.downcast::<EnHeishi3>(at::<EnHeishi3>(&w, -339.0, 1324.0)).unwrap().unk_278, 0);
    for (p, catcher) in [(Vec3::new(-250.0, 1571.0, 800.0), Some(east)), (Vec3::new(-330.0, 1571.0, 800.0), Some(west)), (Vec3::new(-250.0, 1571.0, 1040.0), None)] {
        let mut w = castle(&a, |_| {});
        always_update(&mut w, east);
        always_update(&mut w, west);
        w.place_player(p, 0);
        idle(&mut w, 3);
        let caught = |h| w.actors.downcast::<EnHeishi3>(h).unwrap().action != en_heishi3::Action::StandSentinelInCastle;
        assert_eq!(caught(east), catcher == Some(east), "at {p}");
        assert_eq!(caught(west), catcher == Some(west), "at {p}");
    }
}

#[test]
fn the_throw_out_is_a_white_starburst_circle() {
    // TRANS_TYPE_CIRCLE(TCA_STARBURST, TCC_WHITE, TCS_FAST) = 0x20 | 1 << 3 | 3 << 1 | 0 = 46,
    // and ENTR_HYRULE_CASTLE_4 0x47E (entrance_table.h).
    assert_eq!(en_heishi3::TRANS_TYPE_CAUGHT, 46);
    assert_eq!(trans_type_circle(TCA_STARBURST, TCC_WHITE, TCS_FAST), 46);
    assert_eq!(en_heishi3::ENTR_HYRULE_CASTLE_4, 0x47E);
}

// The gate guard and the fence.

/// The castle gate guard's (type 2) text after a few updates.
fn gate_guard_text(a: &Arc<GameAssets>, hour: i32, preset: &str) -> u16 {
    let mut w = enter_with(a, "ENTR_HYRULE_CASTLE_0", hour, preset, |_| {});
    let h = one::<EnHeishi2>(&w);
    assert_eq!(w.actors.downcast::<EnHeishi2>(h).unwrap().ty, 2);
    always_update(&mut w, h);
    idle(&mut w, 3);
    base(&w, h).text_id
}

#[test]
fn the_gate_guards_text_follows_the_egg_and_the_time() {
    let Some(a) = assets() else { return };
    // func_80A53278: without EVENTCHKINF_RECEIVED_WEIRD_EGG 0x7029 ("won't pass"); with it the
    // bribe 0x7071 (unk_30E zeroed first: always the first); by night 0x7002.
    assert_eq!(gate_guard_text(&a, 10, "deku-tree-dead"), 0x7029);
    assert_eq!(gate_guard_text(&a, 10, "castle-egg"), 0x7071);
    assert_eq!(gate_guard_text(&a, 22, "castle-egg"), 0x7002);
}

#[test]
fn lon_lons_fence_slides_open_on_the_guards_call() {
    let Some(a) = assets() else { return };
    // func_808B4930: on unk_168 (the guard's call) the timer at 2; func_808B4978 moves it 2 a
    // frame once the timer is 0 (the third update on) until z < 2660: 2919 - 2 * 130 = 2659 on the
    // 132nd, then 30 frames open (func_808B4A04) and back to waiting on the 162nd.
    let mut w = castle(&a, |_| {});
    let h = one::<BgSpot15Saku>(&w);
    always_update(&mut w, h);
    assert_eq!(base(&w, h).world_pos.z, 2919.0);
    w.actors.downcast_mut::<BgSpot15Saku>(h).unwrap().unk_168 = 1;
    for n in 1..=162 {
        idle(&mut w, 1);
        let f = w.actors.downcast::<BgSpot15Saku>(h).unwrap();
        let z = f.actor.world_pos.z;
        match n {
            1 | 2 => assert_eq!((z, f.action), (2919.0, bg_spot15_saku::Action::Open), "update {n}"),
            3..=131 => assert_eq!((z, f.action), (2919.0 - 2.0 * (n - 2) as f32, bg_spot15_saku::Action::Open), "update {n}"),
            132..=161 => assert_eq!((z, f.action), (2659.0, bg_spot15_saku::Action::Opened), "update {n}"),
            _ => assert_eq!((z, f.action, f.unk_168), (2659.0, bg_spot15_saku::Action::Wait, 0), "update {n}"),
        }
    }
    // INFTABLE_71: placed open, and it stays (func_808B4930 wants the flag clear).
    let mut w = castle(&a, |s| s.set_inf_table(bg_spot15_saku::INFTABLE_71));
    let h = one::<BgSpot15Saku>(&w);
    always_update(&mut w, h);
    assert_eq!(base(&w, h).world_pos.z, 2659.0);
    w.actors.downcast_mut::<BgSpot15Saku>(h).unwrap().unk_168 = 1;
    idle(&mut w, 5);
    assert_eq!(w.actors.downcast::<BgSpot15Saku>(h).unwrap().action, bg_spot15_saku::Action::Wait);
}

// Kakariko.

#[test]
fn kakarikos_gate_and_its_guard() {
    let Some(a) = assets() else { return };
    // spot01's placements: the gate (91, 400, -1350), rot -1638; its guard (En_Heishi2 type 5,
    // params 0xFF05) at (-30, 400, -1298). Not shown Zelda's letter: 0x200E.
    let mut w = enter_with(&a, "ENTR_KAKARIKO_VILLAGE_0", 10, "deku-tree-dead", |_| {});
    let g = one::<BgGateShutter>(&w);
    assert_eq!(base(&w, g).world_pos, Vec3::new(91.0, 400.0, -1350.0));
    assert_eq!(base(&w, g).shape_rot.y, -1638);
    let guard = at::<EnHeishi2>(&w, -30.0, -1298.0);
    assert_eq!(w.actors.downcast::<EnHeishi2>(guard).unwrap().ty, 5);
    always_update(&mut w, guard);
    idle(&mut w, 3);
    assert_eq!(base(&w, guard).text_id, 0x200E);
    // openingState 1 (the guard's, the letter shown): func_80878300 from the third update, 2 a
    // frame west (z to -1375 by 0.8) until x < -89: 91 - 2 * 91 = -91 on the 93rd; 30 frames done.
    always_update(&mut w, g);
    w.actors.downcast_mut::<BgGateShutter>(g).unwrap().opening_state = 1;
    idle(&mut w, 92);
    let s = w.actors.downcast::<BgGateShutter>(g).unwrap();
    assert_eq!((s.actor.world_pos.x, s.action), (-89.0, bg_gate_shutter::Action::Open));
    idle(&mut w, 1);
    let s = w.actors.downcast::<BgGateShutter>(g).unwrap();
    assert_eq!((s.actor.world_pos.x, s.actor.world_pos.z, s.action), (-91.0, -1375.0, bg_gate_shutter::Action::Done));
    // INFTABLE_76 (the gate opened before): placed open.
    let w = enter_with(&a, "ENTR_KAKARIKO_VILLAGE_0", 10, "deku-tree-dead", |s| s.set_inf_table(bg_gate_shutter::INFTABLE_76));
    let g = one::<BgGateShutter>(&w);
    assert_eq!(base(&w, g).world_pos, Vec3::new(-89.0, 400.0, -1375.0));
}

#[test]
fn pots_live_where_their_object_is_loaded() {
    let Some(a) = assets() else { return };
    // ObjTsubo_Init: kind (params >> 8) & 1 asks for object_tsubo (kind 1) or
    // gameplay_dangeon_keep; "Bank danger!" and Actor_Kill when the room hasn't loaded it. The
    // castle's two (params 0x7F00, 0x7F01: kind 1) find no object_tsubo; Kakariko's eleven (0x4103
    // to 0x5503) do.
    let w = castle(&a, |_| {});
    assert!(all::<ObjTsubo>(&w).is_empty());
    let w = enter_with(&a, "ENTR_KAKARIKO_VILLAGE_0", 10, "deku-tree-dead", |_| {});
    let pots = all::<ObjTsubo>(&w);
    assert_eq!(pots.len(), 11);
    for h in pots {
        let p = w.actors.downcast::<ObjTsubo>(h).unwrap();
        assert_eq!((p.actor.params >> 8) & 1, 1);
        assert!(p.required_object_slot.is_some());
    }
}

#[test]
fn the_castle_roads_torch() {
    let Some(a) = assets() else { return };
    // The torch by the gate (params 0x03F5): D_80A9E840[5] (scale 75: 0.0075); EnLight_Update
    // (bit 10 clear): the glow 40 above, radius 300, its colour (255, 200, 0) at 0.5 to 1.
    let mut w = castle(&a, |_| {});
    let h = one::<EnLight>(&w);
    always_update(&mut w, h);
    idle(&mut w, 1);
    let l = w.actors.downcast::<EnLight>(h).unwrap();
    assert_eq!(l.actor.params, 0x03F5);
    assert_eq!(l.actor.scale, Vec3::splat(75.0 * 0.0001));
    assert!(!l.update_switch);
    let info = w.light_ctx.info(l.light_node.unwrap()).unwrap();
    assert_eq!((info.x, info.y, info.z, info.radius), (1138, 1210 + 40, 3218, 300));
    let f = en_light::FLAME_PARAMS[5];
    for i in 0..3 {
        assert!(info.color[i] as f32 >= f.prim[i] as f32 * 0.5 - 1.0 && info.color[i] <= f.prim[i], "{:?}", info.color);
    }
}

// Player.

#[test]
fn an_item_shown_to_nobody_is_looked_at() {
    let Some(a) = assets() else { return };
    // Player_ActionHandler_13 with no talkActor: Link takes the Cucco out and looks at it
    // (actionVar1 2, link_normal_take_out), textId 0xCF, the turn-around camera.
    let mut w = enter_with(&a, "ENTR_HYRULE_CASTLE_0", 10, "castle-cucco", |_| {});
    idle(&mut w, 40);
    press(&mut w, PadState { button: BTN_CLEFT, ..Default::default() });
    let mut n = 0;
    while player(&w).action != PA::ExchangeItem {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 30, "never took the Cucco out ({:?})", player(&w).action);
    }
    assert_eq!(player(&w).actor.text_id, 0xCF);
    assert_eq!(player(&w).exchange_item_id, en_ta::EXCH_ITEM_CHICKEN);
    let mut n = 0;
    while w.msg_ctx.text_id != 0xCF {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 120, "0xCF never shown");
    }
}

#[test]
fn a_bottle_held_takes_what_it_now_holds() {
    let Some(a) = assets() else { return };
    // Player_UpdateBottleHeld: Inventory_UpdateBottleItem on the held item's button (a full Lon
    // Lon Milk emptied only by half: ITEM_BOTTLE_MILK_HALF), the pause cursor's item; an empty
    // bottle leaves heldItemId alone, the item action follows.
    use oot_game::item::{ITEM_BOTTLE_EMPTY, ITEM_BOTTLE_MILK_FULL, ITEM_BOTTLE_MILK_HALF, SLOT_BOTTLE_1};
    let mut w = castle(&a, |s| {
        s.inventory.items[SLOT_BOTTLE_1] = ITEM_BOTTLE_MILK_FULL;
        s.equip_item_on_c_left(ITEM_BOTTLE_MILK_FULL);
    });
    let ph = w.player.unwrap();
    {
        let p = w.actors.downcast_mut::<Player>(ph).unwrap();
        p.held_item_button = 1;
        p.held_item_id = ITEM_BOTTLE_MILK_FULL;
    }
    oot_actors::player::player_update_bottle_held(&mut w, ITEM_BOTTLE_EMPTY, 0x1E);
    assert_eq!(w.save.inventory.items[SLOT_BOTTLE_1], ITEM_BOTTLE_MILK_HALF);
    assert_eq!(w.save.equips.button_items[1], ITEM_BOTTLE_MILK_HALF);
    assert_eq!(w.pause_ctx.cursor_item[oot_game::kaleido::PAUSE_ITEM as usize], ITEM_BOTTLE_MILK_HALF as u16);
    let p = w.actors.downcast::<Player>(ph).unwrap();
    assert_eq!((p.held_item_id, p.item_ap), (ITEM_BOTTLE_MILK_FULL, 0x1E));
    // A half emptied: empty.
    oot_actors::player::player_update_bottle_held(&mut w, ITEM_BOTTLE_EMPTY, 0x1E);
    assert_eq!(w.save.inventory.items[SLOT_BOTTLE_1], ITEM_BOTTLE_EMPTY);
}

#[test]
fn the_weird_egg_hatches_at_dawn() {
    let Some(a) = assets() else { return };
    // Environment_UpdateTimeBasedSequence's TIMESEQ_DAY_BEGIN_SFX once dayTime passes 6:30 (the
    // field's time sequence): the cock's crow, Inventory_ReplaceItem(ITEM_WEIRD_EGG, ITEM_CHICKEN)
    // (the C-Left button too) and 0x3066. (Play_Init's hatch on a new day: tests/clock.rs.)
    use oot_game::item::{ITEM_CHICKEN, ITEM_WEIRD_EGG, slot};
    let mut w = enter_with(&a, "ENTR_HYRULE_FIELD_0", 6, "castle-egg", |s| s.day_time = oot_game::env::clock_time(6, 25) as u16);
    assert_eq!(w.save.inventory.items[slot(ITEM_WEIRD_EGG)], ITEM_WEIRD_EGG);
    let mut n = 0;
    while w.msg_ctx.text_id != 0x3066 {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 600, "no hatch by {:#x}", w.save.day_time);
    }
    assert!(w.save.day_time > oot_game::env::clock_time(6, 30) as u16);
    assert_eq!(w.save.inventory.items[slot(ITEM_WEIRD_EGG)], ITEM_CHICKEN);
    assert_eq!(w.save.equips.button_items[1], ITEM_CHICKEN);
}

// The water camera.

#[test]
fn swimming_in_the_moat_the_camera_takes_the_waters_setting() {
    let Some(a) = assets() else { return };
    // Camera_UpdateWater: Player in a water box with a bg camera (the moat's: props 0x104, bg cam
    // 4) takes its setting, CAM_SET_NORMAL3 (Camera_Jump3), while swimming; back on land, the
    // floor's again.
    let mut w = castle(&a, |_| {});
    w.place_player(Vec3::new(200.0, 1510.0, 610.0), 0x4000);
    idle(&mut w, 40);
    assert!(w.player().actor.world_pos.y < 1510.0, "not in the water at {}", w.player().actor.world_pos);
    assert_eq!(w.active_camera().setting, oot_game::camera::CAM_SET_NORMAL3);
    w.place_player(Vec3::new(-339.0, 1454.0, 1500.0), 0);
    idle(&mut w, 40);
    assert_ne!(w.active_camera().setting, oot_game::camera::CAM_SET_NORMAL3);
}

// The exits' runs.

fn run_route(route: Route) -> (Playthrough, PlayState) {
    let a = assets().unwrap();
    let e = a.scenes.entrance_index(route.entrance()).expect("entrance");
    let save = route.save(e);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    route.debug_start(&mut w);
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
    }
    assert!(run.failure.is_none(), "{:?}", run.failure);
    eprintln!("steps {:?}", run.steps);
    (run, w)
}

#[test]
fn exit_hyrule_castles_grounds_past_talon_and_the_crates_to_the_courtyard() {
    if assets().is_none() {
        return;
    }
    let (run, w) = run_route(Route::Castle);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    use Step::*;
    assert_eq!(steps, vec![OwlTalk, OwlFlown, Vine, Vine, MoatBank, TalonWoken, TalonTalk, TalonGone, CrateInMoat, CrateInMoat, OnCrates, Crawlspace, Drain, Courtyard]);
    assert_eq!(w.scene_id, SCENE_CASTLE_COURTYARD_GUARDS_DAY);
    // Talon woken and gone home; the courtyard entered by CIRCLE(WAVE, BLACK, FAST).
    assert!(w.save.get_event_chk_inf(en_ta::EVENTCHKINF_TALON_WOKEN_IN_CASTLE));
    assert!(w.save.get_event_chk_inf(en_ta::EVENTCHKINF_TALON_RETURNED_FROM_CASTLE));
    // Talon's run home from his text's event: closed 50 updates in (RunSouth's timer at 20), gone
    // 155 later.
    let at = |s: Step| run.steps.iter().find(|x| x.0 == s).unwrap().1;
    let gone = at(TalonGone) - at(TalonTalk);
    assert!((150..=160).contains(&gone), "{gone}");
}

#[test]
fn caught_on_the_grounds_and_thrown_out_by_the_gate() {
    if assets().is_none() {
        return;
    }
    let (run, w) = run_route(Route::CastleCaught);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    use Step::*;
    assert_eq!(steps, vec![OwlTalk, OwlFlown, Vine, Caught, ThrownOut]);
    // func_80A55D00: EVENTCHKINF_CAUGHT_BY_CASTLE_GUARDS, out to ENTR_HYRULE_CASTLE_4 (the
    // castle's road by the gate) through the white starburst.
    assert_eq!(w.scene_id, SCENE_HYRULE_CASTLE);
    assert!(w.save.get_event_chk_inf(en_heishi3::EVENTCHKINF_CAUGHT_BY_CASTLE_GUARDS));
    assert_eq!(w.save.entrance_index, en_heishi3::ENTR_HYRULE_CASTLE_4);
    assert_eq!(w.transition.ty, en_heishi3::TRANS_TYPE_CAUGHT);
}
