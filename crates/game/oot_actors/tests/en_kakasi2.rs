//! GAME-10 milestone 9 against the C: Pierre (`z_en_kakasi2.c`), who comes up where the adult
//! plays the scarecrow's song once Bonooru has heard it (`EVENTCHKINF_9C`). Lake Hylia's adult
//! layer places three; the one at (1257, -1295, 3333), params 0x053B (switch 0x3B, range 20),
//! `rot.z` 20. His update run alone, the ocarina's mode set by hand.
//!
//! Entered from `fairy-magic` with the lake's intro seen. Expected values are worked out from the
//! C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_kakasi2::{Action, EVENTCHKINF_9C, EnKakasi2};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_LOCK_ON_DISABLED};
use oot_game::actor_ctx::ActorHandle;
use oot_game::message::{OCARINA_MODE_04, OCARINA_MODE_0B};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

/// `EVENTCHKINF_B1` (`save.h`): Lake Hylia's intro seen.
const EVENTCHKINF_B1: u16 = 0xB1;
const HOME: Vec3 = Vec3::new(1257.0, -1295.0, 3333.0);
const SWITCH: i32 = 0x3B;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// An adult's Lake Hylia (`ENTR_LAKE_HYLIA_0`) at 10:00 from `fairy-magic`, `tweak` last; eight
/// frames on.
fn lake(a: &Arc<GameAssets>, tweak: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_LAKE_HYLIA_0").unwrap();
    let mut save = SaveContext::new(e, true, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("fairy-magic").unwrap();
    save.adult = true;
    save.set_event_chk_inf(EVENTCHKINF_B1);
    tweak(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap();
    for _ in 0..8 {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
    w
}

fn find(w: &PlayState) -> ActorHandle {
    *w.actors.all().iter().find(|&&h| w.actors.downcast::<EnKakasi2>(h).is_some_and(|k| k.actor.home_pos == HOME && !k.actor.killed)).expect("En_Kakasi2")
}

fn p(w: &mut PlayState, h: ActorHandle) -> &mut EnKakasi2 {
    w.actors.downcast_mut::<EnKakasi2>(h).expect("En_Kakasi2")
}

fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// Link `across` from him, at his height (`xzDistToPlayer` set, Link's `world.pos.y` his).
fn link_at(w: &mut PlayState, h: ActorHandle, across: f32) {
    let y = p(w, h).actor.world_pos.y;
    w.player_mut().actor.world_pos.y = y;
    p(w, h).actor.xz_dist_to_player = across;
}

#[test]
fn pierre_waits_hidden_for_the_song() {
    let Some(a) = assets() else { return };
    let mut w = lake(&a, |_| {});
    let h = find(&w);
    // EnKakasi2_Init: switch 0x3B; maxSpawnDistance x (0x053B >> 6 & 0xFF = 20) × 40 + 40 = 840,
    // y rot.z 20 × 40 + 40 = 840; hidden 8000 down, not drawn, attention range 4, his yaw kept.
    let s = p(&mut w, h);
    assert_eq!((s.switch_flag, s.max_spawn_distance, s.action), (SWITCH as i16, Vec3::new(840.0, 840.0, 0.0), Action::Func80a90264));
    assert_eq!((s.actor.shape_y_offset, s.drawn, s.actor.target_mode, s.unk_198), (-8000.0, false, 4, -16384));
    assert_eq!((s.actor.scale, s.height), (Vec3::splat(0.01), 60.0));
    // In range, the song played, but Bonooru hasn't heard it: nothing (the mode left as it is),
    // his count (never culled: counting since the lake's first frame) up one.
    link_at(&mut w, h, 100.0);
    w.msg_ctx.ocarina_mode = OCARINA_MODE_0B;
    let n = p(&mut w, h).unk_194;
    update(&mut w, h);
    assert_eq!((p(&mut w, h).action, p(&mut w, h).unk_194), (Action::Func80a90264, n + 1));
    assert_eq!(w.msg_ctx.ocarina_mode, OCARINA_MODE_0B);
    // Heard, in range, another mode: unk_194 back to 0, still hidden.
    w.save.set_event_chk_inf(EVENTCHKINF_9C);
    w.msg_ctx.ocarina_mode = OCARINA_MODE_04;
    update(&mut w, h);
    assert_eq!((p(&mut w, h).action, p(&mut w, h).unk_194), (Action::Func80a90264, 0));
    // Out of range across (840) or up (840): nothing.
    w.msg_ctx.ocarina_mode = OCARINA_MODE_0B;
    link_at(&mut w, h, 840.0);
    update(&mut w, h);
    assert_eq!(p(&mut w, h).action, Action::Func80a90264);
    link_at(&mut w, h, 100.0);
    w.player_mut().actor.world_pos.y += 840.0;
    update(&mut w, h);
    assert_eq!(p(&mut w, h).action, Action::Func80a90264);
    assert!(!w.flags.get_switch(SWITCH));
}

#[test]
fn the_song_brings_pierre_up() {
    let Some(a) = assets() else { return };
    let mut w = lake(&a, |s| s.set_event_chk_inf(EVENTCHKINF_9C));
    let h = find(&w);
    let yaw = p(&mut w, h).actor.shape_rot.y;
    link_at(&mut w, h, 100.0);
    w.msg_ctx.ocarina_mode = OCARINA_MODE_0B;
    update(&mut w, h);
    // The switch flag, the mode back to 4, drawn, the attention cutscene (5010, 30 frames for a
    // prop), lock-on still off.
    let s = p(&mut w, h);
    assert_eq!((s.action, s.drawn), (Action::Func80a904d8, true));
    assert_eq!(s.actor.flags & (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_LOCK_ON_DISABLED), ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_LOCK_ON_DISABLED);
    assert!(w.flags.get_switch(SWITCH));
    assert_eq!(w.msg_ctx.ocarina_mode, OCARINA_MODE_04);
    let cam = w.sub_cameras.iter().flatten().find(|c| c.cs_id == 5010).expect("the attention camera");
    assert_eq!(cam.target, Some(h));
    // func_80A904D8: the dance looped; then rising (Math_ApproachZeroF: half the way, at most
    // 500) and spinning 0x800 a frame: 500 a frame to -1000 (14), then -500, -250, -125, -62.5
    // (> -100: up, the 18th).
    update(&mut w, h);
    assert_eq!(p(&mut w, h).action, Action::Func80a90578);
    let mut offsets = Vec::new();
    for _ in 0..18 {
        update(&mut w, h);
        offsets.push(p(&mut w, h).actor.shape_y_offset);
    }
    let mut want: Vec<f32> = (1..=14).map(|i| -8000.0 + 500.0 * i as f32).collect();
    want.extend([-500.0, -250.0, -125.0, 0.0]);
    assert_eq!(offsets, want);
    let s = p(&mut w, h);
    assert_eq!(s.action, Action::Func80a9062c);
    assert_eq!(s.actor.shape_rot.y, yaw.wrapping_add((18 * 0x800u32) as u16 as i16));
    // Standing: the animation held (speed 0), turning back to his yaw (Math_SmoothStepToS: a
    // fifth of 0x7000, at most 0xBB8).
    update(&mut w, h);
    update(&mut w, h);
    let s = p(&mut w, h);
    assert_eq!((s.action, s.skel.play_speed), (Action::Func80a906c4, 0.0));
    assert_eq!(s.actor.shape_rot.y, yaw.wrapping_add((18 * 0x800u32) as u16 as i16).wrapping_add(0xBB8));
    for _ in 0..60 {
        update(&mut w, h);
    }
    // A fifth of what's left until under 5 (minStep 0: then it stays): -4 here.
    assert_eq!(p(&mut w, h).actor.shape_rot.y, yaw - 4);
}

#[test]
fn his_switch_flag_set_he_is_up_from_the_start() {
    let Some(a) = assets() else { return };
    let mut w = lake(&a, |_| {});
    w.flags.set_switch(SWITCH);
    let h = w.actor_spawn(oot_actors::en_kakasi2::ACTOR_EN_KAKASI2, HOME + Vec3::X * 50.0, [0, 0x4000, 20], 0x053B).expect("En_Kakasi2");
    let s = p(&mut w, h);
    assert_eq!((s.action, s.drawn, s.actor.shape_y_offset), (Action::Func80a9062c, true, 0.0));
    update(&mut w, h);
    assert_eq!(p(&mut w, h).action, Action::Func80a906c4);
}
