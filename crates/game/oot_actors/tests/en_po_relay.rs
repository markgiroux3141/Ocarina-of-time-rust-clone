//! GAME-13 milestone 1 against the C: Dampé's ghost (`z_en_po_relay.c`) in his grave
//! (`hakasitarelay_scene` layer 2, the adult's day), entered at `ENTR_WINDMILL_AND_DAMPES_GRAVE_0`
//! on `epona-won`. No race is run: the race's states are set up directly (his point, his place),
//! as the user's rule for long routes asks; only the wait from the start to the first door is
//! counted, Link standing where the talk left him.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_relay_objects::{self, BgRelayObjects};
use oot_actors::en_box::{self, EnBox};
use oot_actors::en_honotrap::{self, EnHonotrap};
use oot_actors::en_item00::{self, EnItem00, ITEM00_HEART_PIECE, ITEM00_RUPEE_RED};
use oot_actors::en_po_relay::{self, Action, EnPoRelay};
use oot_game::actor::*;
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::item::ITEM_HOOKSHOT;
use oot_game::message::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::{SaveContext, TIMER_STATE_OFF};

/// `HIGH_SCORE(HS_DAMPE_RACE)`.
const HS: usize = HS_DAMPE_RACE as usize;
/// Dampé's placement (room 0: `En_Po_Relay` 0x0007 at (-63, -530, -2177)).
const HOME: Vec3 = Vec3::new(-63.0, -530.0, -2177.0);
/// Room 0's floors: where Dampé waits, and the landing at the stairs' foot north of him (to 80
/// short of him).
const FLOOR_0: f32 = -530.0;
const FLOOR_LANDING: f32 = -470.0;
/// Room 4's floor.
const FLOOR_4: f32 = -360.0;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// An adult at `ENTR_WINDMILL_AND_DAMPES_GRAVE_0` (spawn 0, room 0) at 10:00 with `epona-won`,
/// `edit` applied to the save last; the sounds logged; four frames on.
fn enter(a: &Arc<GameAssets>, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_WINDMILL_AND_DAMPES_GRAVE_0").expect("entrance");
    let mut save = SaveContext::new(e, true, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("epona-won").unwrap();
    edit(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    w.audio.log = Some(Default::default());
    idle(&mut w, 4);
    w
}

fn all<T: 'static + ActorImpl>(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<T>(h).is_some_and(|a| !a.base().killed)).collect()
}

fn get<T: 'static + ActorImpl>(w: &PlayState, h: ActorHandle) -> &T {
    w.actors.downcast::<T>(h).unwrap()
}

fn get_mut<T: 'static + ActorImpl>(w: &mut PlayState, h: ActorHandle) -> &mut T {
    w.actors.downcast_mut::<T>(h).unwrap()
}

fn dampe(w: &PlayState) -> ActorHandle {
    let v = all::<EnPoRelay>(w);
    assert_eq!(v.len(), 1, "one En_Po_Relay");
    v[0]
}

fn sfx_since(w: &PlayState, from: usize, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx[from..].iter().any(|&(_, s, _)| s == id)
}

fn sfx_len(w: &PlayState) -> usize {
    w.audio.log.as_ref().unwrap().sfx.len()
}

/// A until the talk starts (or it starts by itself), then A whenever the box waits (once each
/// wait) until it closes. The texts shown.
fn talk(w: &mut PlayState) -> Vec<u16> {
    let a = PadState { button: BTN_A, ..Default::default() };
    let mut prev = PadState::default();
    let mut n = 0;
    while w.message_state() == TEXT_STATE_NONE {
        let pad = if prev.button == 0 { a } else { PadState::default() };
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        n += 1;
        assert!(n < 60, "the talk never started");
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

/// Link 200 north of Dampé's home (on the landing), facing him: within his 250 (`EnPoRelay_Idle`), his greeting
/// talked through: the race begun.
fn start_race(w: &mut PlayState) -> ActorHandle {
    w.place_player(Vec3::new(HOME.x, FLOOR_LANDING, HOME.z + 200.0), i16::MIN);
    let texts = talk(w);
    assert_eq!(texts, vec![0x5041], "his greeting");
    let h = dampe(w);
    assert_eq!(get::<EnPoRelay>(w, h).action, Action::Race);
    h
}

/// Link in room `room` at `pos` facing `yaw`, the room loaded as walking into it would.
fn into_room(w: &mut PlayState, room: i8, pos: Vec3, yaw: i16) {
    w.place_player(pos, yaw);
    assert!(w.room_request(room));
    idle(w, 1);
    w.room_change_done();
    idle(w, 2);
}

/// Dampé put 20 short of his point `i` (on the way from the one before), making for it.
fn at_point(w: &mut PlayState, h: ActorHandle, i: i16) {
    let p = en_po_relay::path_point(i);
    let prev = en_po_relay::path_point(i - 1);
    let d = (prev - p) * Vec3::new(1.0, 0.0, 1.0);
    let pos = p + d.normalize() * 20.0;
    let d = get_mut::<EnPoRelay>(w, h);
    d.path_point = i;
    d.actor.world_pos = Vec3::new(pos.x, p.y + 45.0, pos.z);
    d.actor.home_pos.y = p.y + 45.0;
}

/// The race's door with switch flag `flag`.
fn door(w: &PlayState, flag: u8) -> ActorHandle {
    all::<BgRelayObjects>(w).into_iter().find(|&h| get::<BgRelayObjects>(w, h).switch_flag == flag).expect("the door")
}

#[test]
fn dampe_waits_in_room_0() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, |_| {});
    let h = dampe(&w);
    let d = get::<EnPoRelay>(&w, h);
    // EnPoRelay_Init: the text 65 with the grave's prefix 0x5000; params & 0x3F (7);
    // EnPoRelay_SetupIdle: no room, facing world -0x8000, heavy.
    assert_eq!(d.action, Action::Idle);
    assert_eq!(d.text_id, 0x5041);
    assert_eq!(d.actor.text_id, 0x5041);
    assert_eq!(d.actor.params, 7);
    assert_eq!(d.actor.room, -1);
    assert_eq!(d.actor.world_rot.y, -0x8000);
    assert_eq!((d.actor.home_pos.x, d.actor.home_pos.z), (HOME.x, HOME.z));
    assert_eq!(d.actor.navi_enemy_id, en_po_relay::NAVI_ENEMY_DAMPES_GHOST);
    assert_eq!(d.actor.col_chk_info.mass, oot_game::collision_check::MASS_HEAVY);
    assert!(d.light_node.is_some());
    // He bobs 45 above his point 0's height (-530): home.y stepped 2 a frame from -530.
    assert!(d.actor.home_pos.y > HOME.y && d.actor.home_pos.y < HOME.y + 45.0);
    // Link at the spawn, 503 away: no offer, no talk.
    idle(&mut w, 30);
    assert_eq!(w.message_state(), TEXT_STATE_NONE);
    let d = get::<EnPoRelay>(&w, h);
    assert_eq!(d.action, Action::Idle);
    assert_eq!(d.actor.home_pos.y, -485.0);
    // sAlreadySpawned: a second is killed at init.
    let h2 = w.actor_spawn(en_po_relay::ACTOR_EN_PO_RELAY, HOME + Vec3::X * 100.0, [0; 3], 7).expect("spawned");
    assert!(w.actors.actor(h2).is_none_or(|a| a.killed));
    idle(&mut w, 2);
    assert_eq!(dampe(&w), h);
}

#[test]
fn the_talk_starts_the_race() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, |_| {});
    let from = sfx_len(&w);
    let h = start_race(&mut w);
    let d = get::<EnPoRelay>(&w, h);
    // EnPoRelay_Talk: the text 0x2F with the prefix; EnPoRelay_SetupRace: the timer counting up
    // from 0, no hookshot on epona-won, no lock-on, his laugh.
    assert_eq!(d.text_id, 0x502F);
    assert!(!d.hookshot_slot_full);
    assert!(d.actor.flags & ACTOR_FLAG_LOCK_ON_DISABLED != 0);
    assert!(d.actor.flags & ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED == 0, "cleared as the talk was taken");
    assert!(w.save.timer_state >= oot_game::save::TIMER_STATE_UP_INIT && w.save.timer_state <= oot_game::save::TIMER_STATE_UP_TICK);
    assert!(sfx_since(&w, from, en_po_relay::NA_SE_EN_PO_LAUGH));
    assert!((0..32).contains(&d.action_timer));
    // His first move: towards his point 0 (-60, -530, -2950), from (-63, -2177); Link 200 off
    // (under 300: Link's speed 0 * 0.8, plus 1 and 0.02 * 50): 2 at most.
    let z0 = d.actor.world_pos.z;
    idle(&mut w, 10);
    let d = get::<EnPoRelay>(&w, h);
    assert_eq!(d.action, Action::Race);
    assert!(d.actor.speed_xz > 0.0);
    assert!(d.actor.world_pos.z < z0, "making for his point 0");
    assert_eq!(d.path_point, 0);
    assert_eq!(d.yaw_towards_path_point, eng_math::vec3f_yaw(d.actor.world_pos, en_po_relay::path_point(0)));
    assert!(w.save.timer_state != TIMER_STATE_OFF);
}

#[test]
fn the_doors_open_at_his_points() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, |_| {});
    let h = start_race(&mut w);
    let d35 = door(&w, 0x35);
    let y0 = get::<BgRelayObjects>(&w, d35).actor.world_pos.y;
    assert!(!w.flags.get_switch(0x35));
    // Within 40 of his point 8: on to 9, the switch 0x35 set (the first door).
    at_point(&mut w, h, 8);
    idle(&mut w, 1);
    assert_eq!(get::<EnPoRelay>(&w, h).path_point, 9);
    assert!(w.flags.get_switch(0x35));
    assert!(!w.flags.get_switch(0x36));
    // func_808A90F4: the door rises 12 a frame.
    idle(&mut w, 3);
    let door = get::<BgRelayObjects>(&w, d35);
    assert_eq!(door.action, bg_relay_objects::Action::Func808a90f4);
    assert!(door.actor.world_pos.y > y0);
    // Points 16 and 24: 0x36 and 0x37.
    at_point(&mut w, h, 16);
    idle(&mut w, 1);
    assert_eq!(get::<EnPoRelay>(&w, h).path_point, 17);
    assert!(w.flags.get_switch(0x36));
    at_point(&mut w, h, 24);
    idle(&mut w, 1);
    assert_eq!(get::<EnPoRelay>(&w, h).path_point, 25);
    assert!(w.flags.get_switch(0x37));
}

#[test]
fn the_wait_to_the_first_door() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, |_| {});
    let h = start_race(&mut w);
    // Link standing 200 north of his home: Dampé alone (over 300 off, 3.5 + 1) along points 0
    // to 8; the switch 0x35 as he reaches point 8.
    let mut flames = std::collections::BTreeSet::new();
    let mut frames = 0;
    while !w.flags.get_switch(0x35) {
        idle(&mut w, 1);
        frames += 1;
        assert!(frames < 2000, "no door after {frames} frames");
        flames.extend(all::<EnHonotrap>(&w));
    }
    eprintln!("the first door's switch {frames} frames after the greeting closed; {} flames dropped; timer {} s", flames.len(), w.save.timer_seconds);
    assert_eq!(get::<EnPoRelay>(&w, h).path_point, 9);
    // About 4300 along his way at 4.5 a frame (6.75 a frame moved: Actor_UpdatePos's 1.5).
    assert!((600..720).contains(&frames), "{frames}");
    assert!(!flames.is_empty(), "his flames (3% a frame once 32 frames have passed)");
    for f in flames.iter().filter_map(|&f| w.actors.downcast::<EnHonotrap>(f)) {
        assert_eq!(f.actor.params, en_honotrap::HONOTRAP_TYPE_FLAME_DROP);
    }
}

#[test]
fn a_flame_drop_falls_and_goes() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, |_| {});
    // As Dampé drops one: 45 up, thrown the way it faces (0x4000: +x).
    let pos = Vec3::new(HOME.x, FLOOR_0 + 45.0, HOME.z - 300.0);
    let from = sfx_len(&w);
    let h = w.actor_spawn(en_honotrap::ACTOR_EN_HONOTRAP, pos, [0, 0x4000, 0], en_honotrap::HONOTRAP_TYPE_FLAME_DROP).expect("spawned");
    let f = get::<EnHonotrap>(&w, h);
    // EnHonotrap_InitFlame: tiny, no room's, its cylinder 12 by 30, lit.
    assert_eq!(f.action, en_honotrap::Action::FlameGrow);
    assert_eq!(f.actor.scale.x, 0.0001);
    assert_eq!(f.actor.room, -1);
    assert_eq!((f.cyl.dim.radius, f.cyl.dim.height), (12, 30));
    assert_eq!(f.actor.shape_shadow_alpha, 128);
    assert!(sfx_since(&w, from, oot_game::audio::sfx::NA_SE_EV_FLAME_IGNITION));
    // EnHonotrap_FlameGrow: 0.0006 a frame to 0.0048: 8 frames.
    idle(&mut w, 7);
    assert_eq!(get::<EnHonotrap>(&w, h).action, en_honotrap::Action::FlameGrow);
    idle(&mut w, 1);
    let f = get::<EnHonotrap>(&w, h);
    assert_eq!(f.action, en_honotrap::Action::FlameDrop);
    assert_eq!(f.actor.scale.x, 0.0048);
    assert_eq!(f.timer, 40);
    // EnHonotrap_FlameDrop: 2 a frame along +x while it falls 1, 2, 3 ... to 1 above the floor
    // (44 down: 9 frames), then still.
    let x0 = f.actor.world_pos.x;
    idle(&mut w, 9);
    let f = get::<EnHonotrap>(&w, h);
    assert_eq!(f.actor.world_pos.y, FLOOR_0 + 1.0);
    assert_eq!(f.actor.velocity.y, 0.0);
    assert!((f.actor.world_pos.x - (x0 + 18.0)).abs() < 0.01, "{} {}", f.actor.world_pos.x, x0);
    assert_eq!(f.col_chk_flags, en_honotrap::HONOTRAP_AT_ACTIVE | en_honotrap::HONOTRAP_AC_ACTIVE | en_honotrap::HONOTRAP_OC_ACTIVE);
    let x1 = f.actor.world_pos.x;
    idle(&mut w, 5);
    assert_eq!(get::<EnHonotrap>(&w, h).actor.world_pos.x, x1);
    // Its 40 frames out: fading (0.00015 a frame to 0.0001), then gone.
    idle(&mut w, 40 - 14);
    assert_eq!(get::<EnHonotrap>(&w, h).action, en_honotrap::Action::FlameVanish);
    idle(&mut w, 32);
    assert!(w.actors.actor(h).is_none_or(|a| a.killed));
}

#[test]
fn a_flame_drop_on_link_knocks_him_down() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, |_| {});
    // Dampé gone: within his 250 Link would be talking (and not hit).
    let d = dampe(&w);
    get_mut::<EnPoRelay>(&mut w, d).actor.kill();
    let link = Vec3::new(HOME.x, FLOOR_0, HOME.z - 200.0);
    w.place_player(link, 0);
    idle(&mut w, 40);
    let health = w.save.health;
    let h = w.actor_spawn(en_honotrap::ACTOR_EN_HONOTRAP, link + Vec3::Y * 20.0, [0, 0, 0], en_honotrap::HONOTRAP_TYPE_FLAME_DROP).expect("spawned");
    // Grown in 8 frames, then on Link: its AT hits (4, fire), it fades at once.
    let mut n = 0;
    while get::<EnHonotrap>(&w, h).action != en_honotrap::Action::FlameVanish {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 40, "never hit Link");
    }
    assert!(n < 40, "hit before its 40 frames: {n}");
    // Not bounced (no shield up): EnHonotrap_FlameDrop's knockback (large, no damage of its
    // own), still where it is.
    let at = get::<EnHonotrap>(&w, h).cyl.base.at_flags;
    assert!(at & oot_game::collision_check::AT_HIT != 0 && at & oot_game::collision_check::AT_BOUNCED == 0);
    let p = w.player.and_then(|p| w.actors.downcast::<oot_actors::player::Player>(p)).unwrap();
    assert_eq!(p.knockback_type, 2, "PLAYER_KNOCKBACK_LARGE");
    let p0 = p.actor.world_pos;
    idle(&mut w, 6);
    let p = w.player.and_then(|p| w.actors.downcast::<oot_actors::player::Player>(p)).unwrap();
    // Player_UpdateDamage: the knockback, with the cylinder's hit's damage (4: a quarter heart).
    assert!(p.actor.world_pos.distance(p0) > 10.0, "Link knocked back");
    assert_eq!(w.save.health, health - 4, "Link hurt");
    assert_eq!(get::<EnHonotrap>(&w, h).actor.velocity, Vec3::ZERO);
}

/// The race just ended (his point 27 reached) with Link in room 4 10 west of the end, `seconds`
/// on the timer; his time's texts talked through. The texts.
fn end_race(w: &mut PlayState, h: ActorHandle, seconds: i16) -> Vec<u16> {
    into_room(w, 4, Vec3::new(2890.0, FLOOR_4, -1540.0), 0x4000);
    at_point(w, h, 27);
    idle(w, 1);
    assert_eq!(get::<EnPoRelay>(w, h).action, Action::EndRace);
    assert_eq!(get::<EnPoRelay>(w, h).actor.speed_xz, 0.0);
    w.save.timer_seconds = seconds;
    // EnPoRelay_EndRace: turned to -0x4000 (west, Link's way), 0x800 a frame.
    idle(w, 20);
    assert_eq!(get::<EnPoRelay>(w, h).actor.shape_rot.y, -0x4000);
    // Link 100 west of him, facing him: within 150 and 0x3000 of his facing.
    let p = get::<EnPoRelay>(w, h).actor.world_pos;
    w.place_player(Vec3::new(p.x - 100.0, FLOOR_4, p.z), 0x4000);
    talk(w)
}

#[test]
fn the_first_race_ends_with_the_chest() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, |_| {});
    let h = start_race(&mut w);
    let texts = end_race(&mut w, h, 75);
    // EnPoRelay_Talk2: 0x502F's event goes on to 0x502D (no hookshot at the start).
    assert_eq!(texts, vec![0x502F, 0x502D]);
    let d = get::<EnPoRelay>(&w, h);
    assert_eq!(d.action, Action::DisappearAndReward);
    assert_eq!(w.save.timer_state, TIMER_STATE_OFF);
    let chest = all::<EnBox>(&w).into_iter().find(|&c| get::<EnBox>(&w, c).actor.params as u16 == 0x1100).expect("room 4's chest");
    assert_eq!(get::<EnBox>(&w, chest).action, en_box::Action::AppearOnRoomClear);
    assert!(!w.flags.get_temp_clear(4));
    // EnPoRelay_DisappearAndReward: from 0.01 by 0.001 a frame: gone on his 10th frame (the
    // talk's closing frame took one), room 4's temporary clear flag and the high score set.
    let mut n = 0;
    while w.actors.actor(h).is_some_and(|a| !a.killed) {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 15, "still there");
    }
    assert!(w.flags.get_temp_clear(4));
    assert_eq!(w.save.high_scores[HS], 75);
    // The chest appears (EnBox_AppearOnRoomClear: the room's clear flag, then its appearing).
    idle(&mut w, 3);
    assert_ne!(get::<EnBox>(&w, chest).action, en_box::Action::AppearOnRoomClear);
    assert!(w.flags.get_clear(4));
}

/// A later race: the hookshot had at its start, `seconds` on the timer, the best `best`, the
/// heart piece's flag (7) set or not. The high score after, and the drop's type.
fn later_race(a: &Arc<GameAssets>, seconds: i16, best: i32, heart_taken: bool) -> (i32, i16) {
    let mut w = enter(a, |s| {
        oot_game::item::item_give(s, None, ITEM_HOOKSHOT);
        s.high_scores[HS] = best;
    });
    if heart_taken {
        w.flags.set_collectible(7);
    }
    let h = start_race(&mut w);
    assert!(get::<EnPoRelay>(&w, h).hookshot_slot_full);
    let texts = end_race(&mut w, h, seconds);
    // 0x502F, then 0x502E (the hookshot at the start).
    assert_eq!(texts, vec![0x502F, 0x502E]);
    let before = all::<EnItem00>(&w);
    let mut n = 0;
    while w.actors.actor(h).is_some_and(|a| !a.killed) {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 15, "still there");
    }
    assert!(!w.flags.get_temp_clear(4), "a later race leaves the chest alone");
    let new: Vec<_> = all::<EnItem00>(&w).into_iter().filter(|i| !before.contains(i)).collect();
    assert_eq!(new.len(), 1, "one drop");
    let item = get::<EnItem00>(&w, new[0]);
    // At his place on the floor (room 4's -360).
    assert!((item.actor.world_pos.y - FLOOR_4).abs() < 1.0, "{}", item.actor.world_pos.y);
    let ty = en_item00::TYPE.get(item.actor.params);
    if ty == ITEM00_HEART_PIECE {
        // Item_DropCollectible2((7 << 8) + (0x4000 | ITEM00_HEART_PIECE)): his collectible flag,
        // hovering (no gravity, still).
        assert_eq!(item.collectible_flag, 7);
        assert_eq!(item.actor.gravity, 0.0);
    }
    (w.save.high_scores[HS], ty)
}

#[test]
fn a_later_race_under_60_seconds_drops_the_heart_piece() {
    let Some(a) = assets() else { return };
    // 55 seconds, the best 70: the best 55, the piece of heart.
    assert_eq!(later_race(&a, 55, 70, false), (55, ITEM00_HEART_PIECE));
    // 61 seconds: a red rupee; 61 is better than 70.
    assert_eq!(later_race(&a, 61, 70, false), (61, ITEM00_RUPEE_RED));
    // 50 seconds, the best 40, the piece of heart already taken: a red rupee, the best kept.
    assert_eq!(later_race(&a, 50, 40, true), (40, ITEM00_RUPEE_RED));
}

#[test]
fn link_past_him_in_room_5_and_he_is_gone() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, |_| {});
    let h = start_race(&mut w);
    into_room(&mut w, 4, Vec3::new(2890.0, FLOOR_4, -1540.0), 0);
    at_point(&mut w, h, 27);
    idle(&mut w, 1);
    assert_eq!(get::<EnPoRelay>(&w, h).action, Action::EndRace);
    // Room 5 (past the last door): EnPoRelay_EndRace kills him, the timer off.
    into_room(&mut w, 5, Vec3::new(2892.0, FLOOR_4, -1149.0), 0);
    assert!(w.actors.actor(h).is_none_or(|a| a.killed));
    assert_eq!(w.save.timer_state, TIMER_STATE_OFF);
}
