//! GAME-07 milestone 5 against the C: Kakariko's carpenters (`z_en_daiku_kakariko.c`), their boss
//! Mutoh (`z_en_toryo.c`) and the carpenter's son by night (`z_en_hs2.c`): where each is kept by
//! scene, age and time, the runners' paths, their texts and the flags the talks set.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_daiku_kakariko::{self as daiku, Action as DA, EnDaikuKakariko};
use oot_actors::en_hs2::{self, EnHs2};
use oot_actors::en_toryo::{self, EnToryo};
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::message::{MSGMODE_TEXT_AWAIT_INPUT, TEXT_STATE_AWAITING_NEXT, TEXT_STATE_CHOICE, TEXT_STATE_DONE, TEXT_STATE_DONE_HAS_NEXT, TEXT_STATE_NONE};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

/// `EVENTCHKINF_A3` (`save.h`): Kakariko's intro seen.
const EVENTCHKINF_A3: u16 = 0xA3;
/// `EVENTCHKINF_B2` (`save.h`): Gerudo Valley's intro seen (`sEntranceCutsceneTable`).
const EVENTCHKINF_B2: u16 = 0xB2;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Link (`adult` or a child) entered at `entrance` at `hour`:00 with `preset`, `edit` applied to
/// the save last; two frames on (the room's actors and objects in).
fn enter_age(a: &Arc<GameAssets>, entrance: &str, adult: bool, hour: i32, preset: &str, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, adult, oot_game::env::clock_time(hour, 0) as u16);
    save.apply_preset(preset).unwrap();
    edit(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    idle(&mut w, 2);
    w
}

/// Child Link entered at `entrance` at `hour`:00.
fn enter(a: &Arc<GameAssets>, entrance: &str, hour: i32, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    enter_age(a, entrance, false, hour, "deku-tree-dead", edit)
}

/// Kakariko at `hour`:00 from its entrance, its intro seen.
fn kakariko(a: &Arc<GameAssets>, hour: i32, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    enter(a, "ENTR_KAKARIKO_VILLAGE_0", hour, |s| {
        s.set_event_chk_inf(EVENTCHKINF_A3);
        edit(s);
    })
}

/// Gerudo Valley by day, its intro seen.
fn gerudo_valley(a: &Arc<GameAssets>, adult: bool) -> PlayState {
    enter_age(a, "ENTR_GERUDO_VALLEY_0", adult, 10, "deku-tree-dead", |s| s.set_event_chk_inf(EVENTCHKINF_B2))
}

fn guest_house(a: &Arc<GameAssets>, hour: i32) -> PlayState {
    enter(a, "ENTR_KAKARIKO_CENTER_GUEST_HOUSE_0", hour, |_| {})
}

fn potion_shop(a: &Arc<GameAssets>, hour: i32) -> PlayState {
    enter(a, "ENTR_POTION_SHOP_KAKARIKO_0", hour, |_| {})
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

fn base(w: &PlayState, h: ActorHandle) -> &Actor {
    w.actors.actor(h).unwrap()
}

fn carpenter(w: &PlayState, h: ActorHandle) -> &EnDaikuKakariko {
    w.actors.downcast::<EnDaikuKakariko>(h).unwrap()
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

/// A until the talk starts, then A whenever the box waits (once each wait) until it closes;
/// `during` runs once the talk has started. The texts shown.
fn talk(w: &mut PlayState, mut during: impl FnMut(&mut PlayState)) -> Vec<u16> {
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
    during(w);
    let mut texts = Vec::new();
    // The box's (state, mode) when A was last pressed: not pressed again until it changes.
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

// Where they are.

#[test]
fn the_carpenters_run_kakariko_by_day_for_a_child() {
    let Some(a) = assets() else { return };
    // spot01 layer 0: 0x0002 (Jiro, path 0) at (1105, 80, 88), 0x0103 (Shiro, path 1) at
    // (1167, 260, 663). EnDaikuKakariko_Init, a child in SCENE_KAKARIKO_VILLAGE by day: flags 1
    // | initFlags[type] (Jiro 0x0070, Shiro 0x0470): 0x10 runs (ANIM_3, EnDaikuKakariko_Run),
    // 0x40 falls (gravity -1); run speed 3, ATTENTION_RANGE_6, culling distance 1200.
    let w = kakariko(&a, 10, |_| {});
    let jiro = carpenter(&w, at::<EnDaikuKakariko>(&w, 1105.0, 88.0));
    assert_eq!((jiro.ty(), jiro.path_index()), (daiku::CARPENTER_JIRO, 0));
    assert_eq!(jiro.flags & 0xFFF, 0x0071);
    assert_eq!((jiro.action, jiro.current_anim_index), (DA::Run, daiku::ENDAIKUKAKARIKO_ANIM_3));
    assert_eq!((jiro.actor.gravity, jiro.run_speed, jiro.actor.target_mode, jiro.actor.culling_volume_distance), (-1.0, 3.0, 6, 1200.0));
    assert!(jiro.skel.is("object_daiku_Anim_000C44"));
    let shiro = carpenter(&w, at::<EnDaikuKakariko>(&w, 1167.0, 663.0));
    assert_eq!((shiro.ty(), shiro.path_index()), (daiku::CARPENTER_SHIRO, 1));
    assert_eq!(shiro.flags & 0xFFF, 0x0471);
    assert_eq!(shiro.action, DA::Run);
    // Their texts (EnDaikuKakariko_HandleTalking, flags 1): Jiro 0x506A, Shiro 0x5077.
    assert_eq!((jiro.text_id(), shiro.text_id()), (0x506A, 0x5077));
    // Bg_Spot01_Objects2 0x0203 (type 3, the shooting gallery's frame) spawns Sabooro by day as
    // its child: params (path 2 << 8) + 1 at path 2's first point (497, 300, 831). initFlags
    // 0x00B0: runs (0x10), there and back (0x20), the bg check once (0x80); no gravity. His text
    // 0x502A.
    let mut w = w;
    let toryo = one::<EnToryo>(&w);
    let mut parent = base(&w, toryo).clone();
    let h = w.actor_spawn_as_child(&mut parent, daiku::ACTOR_EN_DAIKU_KAKARIKO, Vec3::new(497.0, 300.0, 831.0), [0; 3], 0x0201).expect("spawned");
    assert_eq!(parent.child, Some(h));
    let s = carpenter(&w, h);
    assert!(!s.actor.killed);
    assert_eq!((s.ty(), s.path_index(), s.flags & 0xFFF, s.action), (daiku::CARPENTER_SABOORO, 2, 0x00B1, DA::Run));
    assert_eq!((s.actor.gravity, s.text_id()), (0.0, 0x502A));
    // The bg check once: the first run frame moves 0x80 to runFlag, the next checks and clears it.
    idle(&mut w, 1);
    assert_eq!((carpenter(&w, h).flags & 0x80, carpenter(&w, h).run_flag), (0, 1));
    idle(&mut w, 1);
    assert_eq!(carpenter(&w, h).run_flag, 0);
}

#[test]
fn nobody_builds_kakariko_by_night() {
    let Some(a) = assets() else { return };
    // By night spot01's layer 1 has no carpenters and no Mutoh; the son sits under a tree.
    let w = kakariko(&a, 22, |_| {});
    assert!(all::<EnDaikuKakariko>(&w).is_empty());
    assert!(all::<EnToryo>(&w).is_empty());
    let h = at::<EnHs2>(&w, -901.0, 522.0);
    assert_eq!(base(&w, h).params, -1);
}

#[test]
fn the_guest_house_has_them_by_night_only() {
    let Some(a) = assets() else { return };
    // kakariko_scene: Mutoh (43, 0, -100); 0xFF01 Sabooro (-122, 0, 115), 0xFF02 Jiro
    // (-130, 0, 35), 0xFF03 Shiro (-146, 16, -124). By day: all killed.
    let w = guest_house(&a, 10);
    assert!(all::<EnDaikuKakariko>(&w).is_empty());
    assert!(all::<EnToryo>(&w).is_empty());
    // By night: flags 2 | 8; waiting (0x100). Sabooro and Shiro ANIM_5 (object_daiku_Anim_008164)
    // with 0x800; Jiro ANIM_1 (object_daiku_Anim_007DE0). No gravity.
    let w = guest_house(&a, 22);
    assert_eq!(all::<EnDaikuKakariko>(&w).len(), 3);
    let s = carpenter(&w, at::<EnDaikuKakariko>(&w, -122.0, 115.0));
    assert_eq!((s.ty(), s.flags & 0xFFF, s.action, s.current_anim_index), (daiku::CARPENTER_SABOORO, 0x090A, DA::Wait, daiku::ENDAIKUKAKARIKO_ANIM_5));
    assert!(s.skel.is("object_daiku_Anim_008164"));
    let j = carpenter(&w, at::<EnDaikuKakariko>(&w, -130.0, 35.0));
    assert_eq!((j.ty(), j.flags & 0xFFF, j.action, j.current_anim_index), (daiku::CARPENTER_JIRO, 0x010A, DA::Wait, daiku::ENDAIKUKAKARIKO_ANIM_1));
    assert!(j.skel.is("object_daiku_Anim_007DE0"));
    let sh = carpenter(&w, at::<EnDaikuKakariko>(&w, -146.0, -124.0));
    assert_eq!((sh.ty(), sh.flags & 0xFFF, sh.current_anim_index), (daiku::CARPENTER_SHIRO, 0x090A, daiku::ENDAIKUKAKARIKO_ANIM_5));
    assert_eq!(s.actor.gravity, 0.0);
    // Their texts without flags 1: Sabooro 0x5074, Jiro 0x506B, Shiro 0x5078.
    assert_eq!((s.text_id(), j.text_id(), sh.text_id()), (0x5074, 0x506B, 0x5078));
    // Mutoh: stateFlags 4 | 8.
    let t = w.actors.downcast::<EnToryo>(at::<EnToryo>(&w, 43.0, -100.0)).unwrap();
    assert_eq!(t.state_flags & 0xF, 0xC);
}

#[test]
fn ichiro_is_in_the_potion_shop_always_but_not_for_an_adult() {
    let Some(a) = assets() else { return };
    // drag_scene: 0xFF00 (Ichiro) at (-104, 0, 64). flags 4 (and 8 by night), waiting: ANIM_0
    // (object_daiku_Anim_001AB0) by day, ANIM_1 by night.
    let w = potion_shop(&a, 10);
    let mut w = w;
    idle(&mut w, 1);
    let c = carpenter(&w, at::<EnDaikuKakariko>(&w, -104.0, 64.0));
    assert_eq!((c.ty(), c.flags & 0xFFF, c.action, c.current_anim_index), (daiku::CARPENTER_ICHIRO, 0x0104, DA::Wait, daiku::ENDAIKUKAKARIKO_ANIM_0));
    assert!(c.skel.is("object_daiku_Anim_001AB0"));
    assert_eq!(c.text_id(), 0x5075);
    // EnDaikuKakariko_Update, flags 4: the collider 27 back on x and z, radius 63.
    let p = c.actor.world_pos;
    assert_eq!((c.collider.dim.pos[0], c.collider.dim.pos[2], c.collider.dim.radius), (p.x as i16 - 27, p.z as i16 - 27, 63));
    let w = potion_shop(&a, 22);
    let c = carpenter(&w, at::<EnDaikuKakariko>(&w, -104.0, 64.0));
    assert_eq!((c.flags & 0xFFF, c.current_anim_index, c.text_id()), (0x010C, daiku::ENDAIKUKAKARIKO_ANIM_1, 0x5076));
    // An adult: LINK_AGE_IN_YEARS isn't YEARS_CHILD: killed.
    let w = enter_age(&a, "ENTR_POTION_SHOP_KAKARIKO_0", true, 10, "deku-tree-dead", |_| {});
    assert!(all::<EnDaikuKakariko>(&w).is_empty());
}

// The runners.

#[test]
fn jiro_runs_his_path_turning_at_each_point() {
    let Some(a) = assets() else { return };
    let mut w = kakariko(&a, 10, |_| {});
    let h = at::<EnDaikuKakariko>(&w, 1105.0, 88.0);
    // Back at path 0's first point (1105, 80, 88), as placed.
    {
        let c = w.actors.downcast_mut::<EnDaikuKakariko>(h).unwrap();
        c.actor.world_pos = Vec3::new(1105.0, 80.0, 88.0);
        c.actor.shape_rot.y = -9466;
        c.actor.speed_xz = 0.0;
        (c.waypoint, c.path_continue, c.run) = (0, false, false);
    }
    // EnDaikuKakariko_Run: within 10 of point 0: waypoint 1, run; point 1 (759, 80, 360) is 440
    // away. The turn to it (Math_SmoothStepToS, scale 1, step 5000) is done at once; speed
    // Math_SmoothStepToF(0 to 3, 0.8): 2.4. Moved 2.4 along the new yaw, by
    // Actor_UpdatePos's rate (R_UPDATE_RATE * 0.5: 1.5).
    idle(&mut w, 1);
    let yaw = oot_game::path::rad_to_binang_trunc(oot_game::camera::f_atan2f(759.0 - 1105.0, 360.0 - 88.0));
    let c = carpenter(&w, h);
    assert_eq!((c.waypoint, c.run, c.actor.shape_rot.y, c.actor.world_rot.y), (1, true, yaw, yaw));
    assert_eq!(c.actor.speed_xz, 2.4);
    let r = yaw as f32 * std::f32::consts::PI / 32768.0;
    assert!((c.actor.world_pos.x - (1105.0 + 2.4 * eng_math::UPDATE_SCALE * r.sin())).abs() < 0.01, "{}", c.actor.world_pos);
    assert!((c.actor.world_pos.z - (88.0 + 2.4 * eng_math::UPDATE_SCALE * r.cos())).abs() < 0.01, "{}", c.actor.world_pos);
    // Then 2.4 + 0.6 * 0.8 = 2.88.
    idle(&mut w, 1);
    assert_eq!(carpenter(&w, h).actor.speed_xz, 2.4 + (3.0 - 2.4) * 0.8);
    // At the last point (7 points, 0x20): back along it from point 5, path_continue; not running
    // (run false): stopped (speed 0) until turned to point 5.
    {
        let c = w.actors.downcast_mut::<EnDaikuKakariko>(h).unwrap();
        c.actor.world_pos = Vec3::new(-675.0, 200.0, -417.0);
        c.waypoint = 6;
    }
    idle(&mut w, 1);
    let c = carpenter(&w, h);
    assert_eq!((c.waypoint, c.path_continue, c.run, c.action), (5, true, false, DA::Run));
    assert_eq!(c.actor.speed_xz, 0.0, "turning first");
    let mut n = 0;
    while !carpenter(&w, h).run {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 20);
    }
}

#[test]
fn shiro_stops_at_each_end() {
    let Some(a) = assets() else { return };
    let mut w = kakariko(&a, 10, |_| {});
    let h = at::<EnDaikuKakariko>(&w, 1167.0, 663.0);
    // Path 1's last point (1743, 189, 1447) of 6: 0x20 back from point 4; 0x400: timer 2, ANIM_0,
    // EnDaikuKakariko_StopRunning.
    {
        let c = w.actors.downcast_mut::<EnDaikuKakariko>(h).unwrap();
        c.actor.world_pos = Vec3::new(1743.0, 189.0, 1447.0);
        c.waypoint = 5;
    }
    idle(&mut w, 1);
    let c = carpenter(&w, h);
    assert_eq!((c.waypoint, c.path_continue, c.timer, c.action, c.current_anim_index), (4, true, 2, DA::StopRunning, daiku::ENDAIKUKAKARIKO_ANIM_0));
    // Two plays of the once-animation (the timer down to 0 at each end), then running again.
    let mut n = 0;
    while carpenter(&w, h).action == DA::StopRunning {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 400);
    }
    let c = carpenter(&w, h);
    assert_eq!((c.action, c.current_anim_index, c.timer), (DA::Run, daiku::ENDAIKUKAKARIKO_ANIM_3, 0));
    // The path's first end the same: back below 0, waypoint 1, forwards.
    {
        let c = w.actors.downcast_mut::<EnDaikuKakariko>(h).unwrap();
        c.actor.world_pos = Vec3::new(1167.0, 260.0, 663.0);
        c.waypoint = 0;
    }
    idle(&mut w, 1);
    let c = carpenter(&w, h);
    assert_eq!((c.waypoint, c.path_continue, c.timer, c.action), (1, false, 2, DA::StopRunning));
}

// Talking.

#[test]
fn talking_to_ichiro_turns_him_and_sets_no_flag() {
    let Some(a) = assets() else { return };
    let mut w = potion_shop(&a, 10);
    let h = at::<EnDaikuKakariko>(&w, -104.0, 64.0);
    offered(&mut w, h, 50.0);
    assert_eq!(base(&w, h).text_id, 0x5075);
    // A: EnDaikuKakariko_Wait sees talkState 2: ANIM_4 (by day), 0x200 for 0x100 (the whole body
    // to Link), EnDaikuKakariko_Talk.
    let mut seen = None;
    let texts = talk(&mut w, |w| {
        let c = carpenter(w, h);
        seen = Some((c.action, c.talk_state, c.flags & 0x300, c.current_anim_index));
    });
    assert_eq!(texts, vec![0x5075]);
    assert_eq!(seen, Some((DA::Talk, 2, 0x200, daiku::ENDAIKUKAKARIKO_ANIM_4)));
    // The text done (EnDaikuKakariko_GetTalkState): talkState 0, ANIM_0, 0x100 again, waiting.
    let c = carpenter(&w, h);
    assert_eq!((c.action, c.talk_state, c.flags & 0x300, c.current_anim_index), (DA::Wait, 0, 0x100, daiku::ENDAIKUKAKARIKO_ANIM_0));
    assert!(!w.save.get_inf_table(daiku::INFTABLE_176) && !w.save.get_inf_table(daiku::INFTABLE_178));
    // En_Daiku's leftover: had his textId been 0x6061 (or 0x6064) at the text's end,
    // INFTABLE_176 (or _178).
    offered(&mut w, h, 50.0);
    talk(&mut w, |w| w.actors.downcast_mut::<EnDaikuKakariko>(h).unwrap().actor.text_id = 0x6061);
    assert!(w.save.get_inf_table(daiku::INFTABLE_176));
    offered(&mut w, h, 50.0);
    talk(&mut w, |w| w.actors.downcast_mut::<EnDaikuKakariko>(h).unwrap().actor.text_id = 0x6064);
    assert!(w.save.get_inf_table(daiku::INFTABLE_178));
}

#[test]
fn a_guest_house_carpenter_talks_by_night() {
    let Some(a) = assets() else { return };
    // Jiro by night (flags 8, no 0x800): no ANIM_4 (still ANIM_1); 0x200 while talking; 0x506B.
    let mut w = guest_house(&a, 22);
    let h = at::<EnDaikuKakariko>(&w, -130.0, 35.0);
    offered(&mut w, h, 50.0);
    let mut seen = None;
    let texts = talk(&mut w, |w| {
        let c = carpenter(w, h);
        seen = Some((c.action, c.flags & 0x300, c.current_anim_index));
    });
    assert_eq!(texts, vec![0x506B]);
    assert_eq!(seen, Some((DA::Talk, 0x200, daiku::ENDAIKUKAKARIKO_ANIM_1)));
    assert_eq!(carpenter(&w, h).flags & 0x300, 0x100);
    // Shiro (0x800): his flags stay as they were, 0x100.
    let h = at::<EnDaikuKakariko>(&w, -146.0, -124.0);
    offered(&mut w, h, 50.0);
    let mut seen = None;
    let texts = talk(&mut w, |w| seen = Some(carpenter(w, h).flags & 0x300));
    assert_eq!((texts, seen), (vec![0x5078], Some(0x100)));
}

// Mutoh.

#[test]
fn mutohs_texts_in_kakariko_follow_inftable_172() {
    let Some(a) = assets() else { return };
    // spot01 layer 0: Mutoh (params -1) at (-932, 0, 521). A child by day: stateFlags 2 | 8.
    // EnToryo_GetTextId: 0x5028 until INFTABLE_172; its end sets it (EnToryo_TalkRespond).
    let mut w = kakariko(&a, 10, |_| {});
    let h = at::<EnToryo>(&w, -932.0, 521.0);
    assert_eq!(w.actors.downcast::<EnToryo>(h).unwrap().state_flags & 0xF, 0xA);
    offered(&mut w, h, 50.0);
    assert_eq!(base(&w, h).text_id, 0x5028);
    let mut talking = None;
    let texts = talk(&mut w, |w| {
        let t = w.actors.downcast::<EnToryo>(h).unwrap();
        talking = Some((t.message_state, t.state_flags & 0x10));
    });
    // 0x5028 goes on to 0x5029 in the same box (the message's own next text); the actor's textId
    // stays 0x5028, whose end sets the flag.
    assert_eq!(texts, vec![0x5028, 0x5029]);
    assert_eq!(talking, Some((1, 0x10)));
    assert!(w.save.get_inf_table(en_toryo::INFTABLE_172));
    let t = w.actors.downcast::<EnToryo>(h).unwrap();
    assert_eq!((t.message_state, t.state_flags & 0x10), (0, 0));
    // Then 0x5029.
    offered(&mut w, h, 50.0);
    assert_eq!(base(&w, h).text_id, 0x5029);
    assert_eq!(talk(&mut w, |_| {}), vec![0x5029]);
}

#[test]
fn mutohs_texts_by_night_and_for_an_adult() {
    let Some(a) = assets() else { return };
    // The guest house by night: 0x506C.
    let mut w = guest_house(&a, 22);
    let h = at::<EnToryo>(&w, 43.0, -100.0);
    offered(&mut w, h, 50.0);
    assert_eq!(talk(&mut w, |_| {}), vec![0x506C]);
    // Gerudo Valley for an adult (spot09 layer 2, (-855, 15, -391)): stateFlags 1 | 8; 0x606A,
    // which sets INFTABLE_170; then 0x606B; 0x606C once the four carpenters are rescued.
    let mut w = gerudo_valley(&a, true);
    let h = at::<EnToryo>(&w, -855.0, -391.0);
    let t = w.actors.downcast::<EnToryo>(h).unwrap();
    assert_eq!(t.state_flags & 0xF, 9);
    assert_eq!(t.get_text_id(&w), 0x606A);
    offered(&mut w, h, 50.0);
    // 0x606A goes on to 0x606B in the same box.
    assert_eq!(talk(&mut w, |_| {}), vec![0x606A, 0x606B]);
    assert!(w.save.get_inf_table(en_toryo::INFTABLE_170));
    let t = w.actors.downcast::<EnToryo>(h).unwrap();
    assert_eq!(t.get_text_id(&w), 0x606B);
    for f in [en_toryo::EVENTCHKINF_CARPENTER_0_RESCUED, en_toryo::EVENTCHKINF_CARPENTER_1_RESCUED, en_toryo::EVENTCHKINF_CARPENTER_2_RESCUED] {
        w.save.set_event_chk_inf(f);
    }
    let t = w.actors.downcast::<EnToryo>(h).unwrap();
    assert_eq!(t.get_text_id(&w), 0x606B, "three of four");
    w.save.set_event_chk_inf(en_toryo::EVENTCHKINF_CARPENTER_3_RESCUED);
    let t = w.actors.downcast::<EnToryo>(h).unwrap();
    assert_eq!(t.get_text_id(&w), 0x606C);
    // A child in Gerudo Valley: killed.
    let w = gerudo_valley(&a, false);
    assert!(all::<EnToryo>(&w).is_empty());
}

#[test]
fn mutoh_answers_the_saw() {
    let Some(a) = assets() else { return };
    // EnToryo_ReactToExchangeItem: the Poacher's Saw 0x606D (0x606E once INFTABLE_171 is set),
    // anything else 0x200F.
    let mut w = gerudo_valley(&a, true);
    let _ = at::<EnToryo>(&w, -855.0, -391.0);
    let react = |w: &mut PlayState, item: u8| EnToryo::react_to_exchange_item(item, w);
    assert_eq!(react(&mut w, en_toryo::EXCH_ITEM_POACHERS_SAW), 0x606D);
    assert_eq!(react(&mut w, 0x01), 0x200F);
    w.save.set_inf_table(en_toryo::INFTABLE_171);
    assert_eq!(react(&mut w, en_toryo::EXCH_ITEM_POACHERS_SAW), 0x606E);
}

// The son.

#[test]
fn the_carpenters_son_talks_by_night() {
    let Some(a) = assets() else { return };
    // spot01 layer 1: En_Hs2 (params -1) at (-901, 0, 522), facing -x. Scale 0.01,
    // ATTENTION_RANGE_6, sitting (gCarpenterSonSittingAnim).
    let mut w = kakariko(&a, 22, |_| {});
    let h = at::<EnHs2>(&w, -901.0, 522.0);
    let s = w.actors.downcast::<EnHs2>(h).unwrap();
    assert_eq!((s.actor.scale, s.actor.target_mode, s.action), (Vec3::splat(0.01), 6, en_hs2::Action::Idle));
    assert!(s.skel.is("gCarpenterSonSittingAnim"));
    // Link behind him: no offer (EnHs2_Interact wants him within 0x2151 of the son's front).
    let p = base(&w, h).world_pos;
    w.place_player(Vec3::new(p.x + 50.0, p.y, p.z), -0x4000);
    idle(&mut w, 5);
    assert_ne!(w.player().target_actor, Some(h));
    // In front within 100: 0x5069; talking (EnHs2_Talking) until the box closes, then idle.
    offered(&mut w, h, 50.0);
    assert_eq!(base(&w, h).text_id, 0x5069);
    let mut seen = None;
    let texts = talk(&mut w, |w| seen = Some(w.actors.downcast::<EnHs2>(h).unwrap().action));
    assert_eq!(texts, vec![0x5069]);
    assert_eq!(seen, Some(en_hs2::Action::Talking));
    assert_eq!(w.actors.downcast::<EnHs2>(h).unwrap().action, en_hs2::Action::Idle);
}

/// Adult Link shows Mutoh the Poacher's Saw (on C-Left) and answers his choice with `choice`;
/// A through every text. Mutoh's `messageState`s and the texts shown.
fn show_the_saw(a: &Arc<GameAssets>, choice: u8) -> (PlayState, Vec<i32>, Vec<u16>) {
    let saw = oot_game::item::ITEM_POACHERS_SAW;
    let mut w = enter_age(a, "ENTR_GERUDO_VALLEY_0", true, 10, "deku-tree-dead", |s| {
        s.set_event_chk_inf(EVENTCHKINF_B2);
        s.inventory.items[oot_game::item::slot(saw)] = saw;
        s.equip_item_on_c_left(saw);
    });
    let h = at::<EnToryo>(&w, -855.0, -391.0);
    offered(&mut w, h, 50.0);
    // C-Left: Player_ActionHandler_13's exchange. EnToryo_HandleTalking: exchangeItemId the saw;
    // EnToryo_ReactToExchangeItem: NA_SE_SY_TRE_BOX_APPEAR and 0x606D as Player's text and his.
    let c_left = PadState { button: eng_input::pad::BTN_CLEFT, ..Default::default() };
    w.tick_with(scripted_input(PadState::default(), c_left));
    w.tick_with(scripted_input(c_left, PadState::default()));
    let mut n = 0;
    while w.actors.downcast::<EnToryo>(h).unwrap().message_state == 0 {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 60, "Mutoh never took the saw");
    }
    let t = w.actors.downcast::<EnToryo>(h).unwrap();
    assert_eq!((t.exchange_item_id, t.actor.text_id, w.player().actor.text_id), (en_toryo::EXCH_ITEM_POACHERS_SAW, 0x606D, 0x606D));
    let a_button = PadState { button: BTN_A, ..Default::default() };
    let (mut prev, mut texts, mut states, mut pressed_at) = (PadState::default(), Vec::new(), Vec::new(), None);
    for _ in 0..1500 {
        let (st, mode) = (w.message_state(), w.msg_ctx.msg_mode);
        if st != TEXT_STATE_NONE && texts.last() != Some(&w.msg_ctx.text_id) {
            texts.push(w.msg_ctx.text_id);
        }
        let ms = w.actors.downcast::<EnToryo>(h).unwrap().message_state;
        if states.last() != Some(&ms) {
            states.push(ms);
        }
        if ms == 0 && st == TEXT_STATE_NONE && mode == 0 {
            return (w, states, texts);
        }
        if pressed_at.is_some_and(|p| p != (st, mode)) {
            pressed_at = None;
        }
        let mut pad = PadState::default();
        if st == TEXT_STATE_CHOICE && w.msg_ctx.choice_index != choice {
            if prev.stick_y == 0 {
                pad.stick_y = -60;
            }
        } else {
            let waits = matches!(st, TEXT_STATE_AWAITING_NEXT | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_CHOICE) || mode == MSGMODE_TEXT_AWAIT_INPUT;
            if waits && prev.button == 0 && pressed_at.is_none() {
                pressed_at = Some((st, mode));
                pad = a_button;
            }
        }
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
    }
    panic!("the talk didn't end: {states:?} {texts:x?}");
}

#[test]
fn the_saw_shown_to_mutoh_gets_the_broken_gorons_sword() {
    let Some(a) = assets() else { return };
    // The choice's first answer (EnToryo_TalkRespond): the box closed, parent and Player's
    // exchangeItemId cleared, MSGMODE_PAUSED, 0x601B (messageState 3, then continued: 1); its
    // end: messageState 4, GI_BROKEN_GORONS_SWORD offered until Player takes it
    // (Actor_HasParent), then 5 until the item's text ends, then 0.
    let (w, states, texts) = show_the_saw(&a, 0);
    assert_eq!(states, vec![1, 3, 1, 4, 5, 0], "texts {texts:x?}");
    // 0x606D goes on to 0x606E (the choice); the sword's get-item text is 0x08 (sGetItemTable).
    assert_eq!(texts, vec![0x606D, 0x606E, 0x601B, 0x08]);
    // ITEM_BROKEN_GORONS_SWORD (`item.h`: 0x33) in the adult trade slot.
    assert_eq!(w.save.inv_content(oot_game::item::ITEM_POACHERS_SAW), 0x33);
    assert!(!w.save.get_inf_table(en_toryo::INFTABLE_171));
}

#[test]
fn the_saw_refused_sets_inftable_171() {
    let Some(a) = assets() else { return };
    // The second answer: 0x606F (messageState 2, continued at once: 1), whose end sets INFTABLE_171; the
    // saw kept. Shown again: 0x606E (EnToryo_ReactToExchangeItem).
    let (w, states, texts) = show_the_saw(&a, 1);
    assert_eq!(states, vec![1, 0], "texts {texts:x?}");
    assert_eq!(texts, vec![0x606D, 0x606E, 0x606F]);
    assert!(w.save.get_inf_table(en_toryo::INFTABLE_171));
    assert_eq!(w.save.inv_content(oot_game::item::ITEM_POACHERS_SAW), oot_game::item::ITEM_POACHERS_SAW);
}
