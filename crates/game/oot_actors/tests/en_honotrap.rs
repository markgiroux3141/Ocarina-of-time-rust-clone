//! `En_Honotrap` (`z_en_honotrap.c`) against the C: the Shadow Temple's eyes (`HAKAdan_scene`
//! room 0, every layer: `HONOTRAP_TYPE_EYE` at (-1775, 5, 95) and (-1335, 5, 95), facing -0x8000)
//! and their flames (`HONOTRAP_TYPE_FLAME_MOVE`), an adult on `epona-won` put in room 0. Dampé's
//! flames (`HONOTRAP_TYPE_FLAME_DROP`) are tests/en_po_relay.rs's.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_honotrap::*;
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::collision_check::AC_HIT;
use oot_game::effect::EFFECT_SS_BOMB2;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;

/// The west eye, and room 0's floor.
const EYE: Vec3 = Vec3::new(-1775.0, 5.0, 95.0);
const FLOOR: f32 = -63.0;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// An adult entered at `ENTR_SHADOW_TEMPLE_0` on `epona-won` at 10:00, put in room 0 at `link`
/// facing the eyes (north, +z), the room loaded as walking into it would; the sounds logged.
fn room0(a: &Arc<GameAssets>, link: Vec3) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_SHADOW_TEMPLE_0").expect("entrance");
    let mut save = oot_game::save::SaveContext::new(e, true, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("epona-won").unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    w.audio.log = Some(Default::default());
    idle(&mut w, 2);
    let (floor, _) = w.col.entity_raycast_down(link + Vec3::Y * 50.0);
    assert_eq!(floor, FLOOR, "room 0's floor at {link}");
    w.place_player(link, 0);
    assert!(w.room_request(0));
    idle(&mut w, 1);
    w.room_change_done();
    w
}

fn traps(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnHonotrap>(h).is_some_and(|a| !a.base().killed)).collect()
}

fn eye(w: &PlayState) -> ActorHandle {
    traps(w).into_iter().find(|&h| get(w, h).actor.home_pos == EYE).expect("the west eye")
}

fn get(w: &PlayState, h: ActorHandle) -> &EnHonotrap {
    w.actors.downcast::<EnHonotrap>(h).unwrap()
}

fn get_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnHonotrap {
    w.actors.downcast_mut::<EnHonotrap>(h).unwrap()
}

fn sfx_since(w: &PlayState, from: usize, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx[from..].iter().any(|&(_, s, _)| s == id)
}

#[test]
fn the_eyes_are_placed_shut() {
    let Some(a) = assets() else { return };
    // Link far off (the room's east end, no eye sees him there: over 750 from the west one).
    let w = room0(&a, Vec3::new(-700.0, FLOOR, -200.0));
    let all = traps(&w);
    assert_eq!(all.len(), 2, "room 0's two eyes");
    let e = get(&w, eye(&w));
    // EnHonotrap_InitEye: scale 0.1, shut (EnHonotrap_SetupEyeIdle); its triangles 8.5 out in
    // front of it (facing -0x8000: towards -z).
    assert_eq!(e.actor.params, HONOTRAP_TYPE_EYE);
    assert_eq!(e.actor.scale, Vec3::splat(0.1));
    assert_eq!(e.action, Action::EyeIdle);
    assert_eq!(e.eye_state, HONOTRAP_EYE_SHUT);
    let v = e.tris.elements[0].dim.vtx;
    assert!((v[0].z - (EYE.z - 8.5)).abs() < 0.01 && (v[0].y - (EYE.y + 23.0)).abs() < 0.01, "{v:?}");
    // sin(-0x8000) is -0 and cos -1: (-23, 0, 8.5) turned is (23, 0, -8.5) about it.
    assert!((v[1].x - (EYE.x + 23.0)).abs() < 0.01, "{v:?}");
}

#[test]
fn the_eye_opens_shoots_and_shuts() {
    let Some(a) = assets() else { return };
    // Link 300 off, 68 below, a little west of straight in front of it (a pit's edge is 50 west
    // of it): an s32 angle of about 0x660 from its facing.
    let link = Vec3::new(EYE.x - 15.0, FLOOR, EYE.z - 300.0);
    let mut w = room0(&a, link);
    let from = 0;
    let h = eye(&w);
    // EnHonotrap_EyeIdle at its first update (the room's loading frame): its timer 0, Link
    // seen: EnHonotrap_SetupEyeOpen (red 40 frames, 30 frames' stare, NA_SE_EV_RED_EYE).
    let e = get(&w, h);
    assert_eq!(e.action, Action::EyeOpen);
    assert_eq!(e.timer, 30);
    assert_eq!(e.eye_state, HONOTRAP_EYE_SHUT);
    assert!(e.actor.color_filter_timer > 0);
    assert!(sfx_since(&w, from, NA_SE_EV_RED_EYE));
    // EnHonotrap_EyeOpen: shut (3) to open (0) a step a frame; open, its flame, its child, 12
    // out (at -z) and 10 down.
    idle(&mut w, 2);
    assert_eq!(get(&w, h).eye_state, HONOTRAP_EYE_HALF);
    assert!(get(&w, h).actor.child.is_none());
    idle(&mut w, 1);
    let e = get(&w, h);
    assert_eq!(e.action, Action::EyeAttack);
    assert_eq!(e.eye_state, HONOTRAP_EYE_OPEN);
    let flame = e.actor.child.expect("its flame");
    let f = get(&w, flame);
    assert_eq!(f.actor.params, HONOTRAP_TYPE_FLAME_MOVE);
    assert_eq!(f.action, Action::FlameGrow);
    assert!((f.actor.world_pos - Vec3::new(EYE.x, EYE.y - 10.0, EYE.z - 12.0)).length() < 0.01, "{}", f.actor.world_pos);
    // At Link, 10 up, as it was made.
    assert!((f.target_pos - (link + Vec3::Y * 10.0)).length() < 1.0, "{}", f.target_pos);
    assert_eq!(f.actor.room, 0, "the eye's room (Actor_SpawnAsChild)");
    // EnHonotrap_EyeAttack: 30 frames from its opening (3 gone), then EnHonotrap_EyeClose, a step
    // a frame to shut: EnHonotrap_EyeIdle with 200 frames' wait, kept at 200 while its flame
    // lives.
    idle(&mut w, 26);
    assert_eq!(get(&w, h).action, Action::EyeAttack);
    idle(&mut w, 1);
    assert_eq!(get(&w, h).action, Action::EyeClose);
    idle(&mut w, 3);
    let e = get(&w, h);
    assert_eq!(e.action, Action::EyeIdle);
    assert_eq!(e.eye_state, HONOTRAP_EYE_SHUT);
    assert!(e.timer >= 199);
    // Its flame gone (it met Link, a wall or its time), the 200 frames run down; Link still in
    // front: it opens again.
    let mut n = 0;
    while w.actors.actor(flame).is_some_and(|a| !a.killed) {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 300, "the flame lives on");
    }
    idle(&mut w, 1);
    assert!(get(&w, h).actor.child.is_none());
    w.place_player(link, 0);
    let mut n = 0;
    while get(&w, h).action == Action::EyeIdle {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 260, "never opened again");
    }
    assert!((195..=202).contains(&n), "{n}");
}

#[test]
fn the_eye_does_not_see_one_side_of_its_front() {
    let Some(a) = assets() else { return };
    // @bug (game): Link a little east of straight in front of it, as much as the other test's
    // west, is at a yaw from it of about +0x79A0: the s32 angle (yaw + 0x8000) is past 0x4000.
    // Never seen.
    let mut w = room0(&a, Vec3::new(EYE.x + 15.0, FLOOR, EYE.z - 300.0));
    let h = eye(&w);
    idle(&mut w, 30);
    assert_eq!(get(&w, h).action, Action::EyeIdle);
    assert!(get(&w, h).actor.child.is_none());
}

#[test]
fn the_eyes_flame_flies_at_link() {
    let Some(a) = assets() else { return };
    let link = Vec3::new(EYE.x - 15.0, FLOOR, EYE.z - 300.0);
    let mut w = room0(&a, link);
    let h = eye(&w);
    idle(&mut w, 4);
    let flame = get(&w, h).actor.child.expect("its flame");
    // EnHonotrap_FlameGrow: 0.0006 a frame to 0.004 (7 frames; its first was its spawn's).
    idle(&mut w, 6);
    let f = get(&w, flame);
    assert_eq!(f.action, Action::FlameMove);
    assert_eq!(f.actor.scale.x, 0.004);
    assert_eq!(f.timer, 160);
    // EnHonotrap_SetupFlameMove: its velocity the way to the target, over the distance plus 1.
    let to = f.target_pos - f.actor.world_pos;
    let dir = to / (to.length() + 1.0);
    assert!((f.actor.velocity - dir).length() < 0.001, "{} {dir}", f.actor.velocity);
    // EnHonotrap_FlameMove: 0.5, 1, 1.5 ... times it a frame.
    let p0 = f.actor.world_pos;
    idle(&mut w, 2);
    let f = get(&w, flame);
    assert!(((f.actor.world_pos - p0).length() - 1.5 * dir.length()).abs() < 0.01, "{}", (f.actor.world_pos - p0).length());
    // On at Link: it hits him (and fades) or reaches the point and chases him.
    let mut seen = vec![];
    for _ in 0..200 {
        let Some(f) = w.actors.downcast::<EnHonotrap>(flame).filter(|f| !f.actor.killed) else { break };
        if seen.last() != Some(&f.action) {
            seen.push(f.action);
        }
        idle(&mut w, 1);
    }
    assert!(w.actors.actor(flame).is_none_or(|a| a.killed), "gone: {seen:?}");
    assert_eq!(seen.last(), Some(&Action::FlameVanish), "{seen:?}");
}

#[test]
fn the_eye_hit_bursts() {
    let Some(a) = assets() else { return };
    let link = Vec3::new(EYE.x - 15.0, FLOOR, EYE.z - 300.0);
    let mut w = room0(&a, link);
    let h = eye(&w);
    idle(&mut w, 2);
    // Open (not shut): its AC on. A weapon's hit (AC_HIT) at its next update: the burst
    // (EffectSsBomb2_SpawnLayered, 15 and 8) and gone.
    assert!(get(&w, h).eye_state < HONOTRAP_EYE_SHUT);
    let before = w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == EFFECT_SS_BOMB2).count();
    get_mut(&mut w, h).tris.base.ac_flags |= AC_HIT;
    idle(&mut w, 1);
    assert!(w.actors.actor(h).is_none_or(|a| a.killed));
    let after = w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == EFFECT_SS_BOMB2).count();
    assert!(after > before, "the burst");
}

#[test]
fn the_draws_bakes() {
    // The eye's three textures (Closed twice in sSilverEyeTextures) and the flame.
    let names: Vec<String> = bakes().into_iter().map(|b| b.name).collect();
    assert_eq!(names, vec![eye_bake("gEyeSwitchSilverOpenTex"), eye_bake("gEyeSwitchSilverHalfTex"), eye_bake("gEyeSwitchSilverClosedTex"), FLAME_BAKE.to_string()]);
}
