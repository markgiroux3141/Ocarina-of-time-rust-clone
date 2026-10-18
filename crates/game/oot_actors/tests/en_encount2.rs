//! GAME-09 milestone 6 against the C: the falling rocks of Death Mountain Trail's summit,
//! `En_Encount2` (`z_en_encount2.c`) and `En_Fire_Rock` (`z_en_fire_rock.c`).
//!
//! Child Link comes out of the Great Fairy's fountain (`ENTR_DEATH_MOUNTAIN_TRAIL_4`, spawn 4 at
//! (-249, 3314, -4497): out of the spawner's area, z below -3600) with `goron-ruby`, and is put
//! on the trail inside it. Nothing here hangs on `Rand`: the checks hold for any draw (Link
//! stands still, so every rock starts within 5 of him across; the counts are bounded), or set the
//! rock's state by hand. Expected values are worked out from the C in the comments.

mod common;

use std::collections::BTreeSet;
use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_encount2::{ACTOR_EN_ENCOUNT2, Action as EncAction, EVENTCHKINF_49, EnEncount2, EnEncount2Effect};
use oot_actors::en_fire_rock::{Action as RockAction, *};
use oot_actors::player::Action as PlayerAction;
use oot_game::actor::BGCHECKFLAG_GROUND;
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorHandle, ActorImpl};
use oot_game::audio::sfx::{NA_SE_EN_VALVAISA_ROCK, NA_SE_EV_BRIDGE_OPEN_STOP, NA_SE_EV_EXPLOSION, NA_SE_EV_VOLCANO, SFX_FLAG, SfxPos};
use oot_game::collision_check::{AT_BOUNCED, MASS_IMMOVABLE};
use oot_game::effect::{EFFECT_SS_DUST, EFFECT_SS_EN_FIRE};
use oot_game::play::{DrawOut, PlayState, ViewInfo, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::quake::{QUAKE_TYPE_1, QUAKE_TYPE_3};
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

/// On the trail inside the area (the floor at (-300, -3000) is 2012: `entity_raycast_down`), by
/// the spawner's place (-199, 2032, -3060).
const LINK_AT: Vec3 = Vec3::new(-300.0, 2012.0, -3000.0);
/// Lower down the trail, inside the area (floor 1776), and a flat spot 200 up the trail from
/// him (floor 1844) where the rocks are dropped.
const LINK_LOW: Vec3 = Vec3::new(-300.0, 1776.0, -2300.0);
const ROCK_SPOT: Vec3 = Vec3::new(-300.0, 1844.0, -2500.0);

/// Child Link out of the Great Fairy's fountain with `goron-ruby`, the sound log on, ten frames
/// run (still at spawn 4, out of the area): by then the Skullwalltulas' object is in and their
/// inits have moved them to the enemies (`Actor_ChangeCategory` in an init run from
/// `Actor_UpdateAll` walks on into the enemies' list: the spawner updates twice on those frames,
/// as in the game).
fn summit(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DEATH_MOUNTAIN_TRAIL_4").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("goron-ruby").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 10);
    w
}

fn all<T: 'static + ActorImpl>(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<T>(h).is_some_and(|a| !a.base().killed)).collect()
}

fn spawner(w: &PlayState) -> ActorHandle {
    *all::<EnEncount2>(w).first().expect("the trail's En_Encount2")
}

fn enc(w: &PlayState) -> &EnEncount2 {
    w.actors.downcast::<EnEncount2>(spawner(w)).unwrap()
}

fn rock(w: &PlayState, h: ActorHandle) -> &EnFireRock {
    w.actors.downcast::<EnFireRock>(h).expect("En_Fire_Rock")
}

fn rocks_of(w: &PlayState, ty: i16) -> Vec<ActorHandle> {
    all::<EnFireRock>(w).into_iter().filter(|&h| rock(w, h).ty == ty).collect()
}

/// The rocks in the arena that `s` spawned, killed or not (a killed one counts until its
/// destroy, at its deletion).
fn spawned_by(w: &PlayState, s: ActorHandle) -> usize {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnFireRock>(h).is_some_and(|r| r.spawner == Some(s))).count()
}

fn link_pos(w: &PlayState) -> Vec3 {
    w.player().actor.world_pos
}

/// Link put at `at`, two frames to land.
fn put_link(w: &mut PlayState, at: Vec3) {
    w.place_player(at, 0);
    idle(w, 2);
}

fn kill_spawner(w: &mut PlayState) {
    let h = spawner(w);
    w.actors.actor_mut(h).unwrap().kill();
    idle(w, 1);
}

fn effects_of(w: &PlayState, ty: u8) -> usize {
    w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == ty).count()
}

fn sfx_count(w: &PlayState, id: u16) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).count()
}

fn sfx_pos(w: &PlayState, id: u16) -> Vec<SfxPos> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(_, _, p)| p).collect()
}

fn spawn_rock(w: &mut PlayState, ty: i16, pos: Vec3) -> ActorHandle {
    w.actor_spawn(ACTOR_EN_FIRE_ROCK, pos, [0; 3], ty).expect("En_Fire_Rock spawned")
}

/// The quake requests of `ty` with `y` up and down.
fn quakes(w: &PlayState, ty: u32, y: i16) -> Vec<oot_game::quake::QuakeRequest> {
    w.quake.requests.iter().filter(|r| r.type_ == ty && r.y == y && r.timer > 0).copied().collect()
}

#[test]
fn the_trail_spawner_is_killed_only_for_an_adult_after_the_fire_temple() {
    let Some(a) = assets() else { return };
    let mut w = summit(&a);
    // Placed on the trail (spot16 room 0, every child layer): params 5 at (-199, 2032, -3060).
    let s = enc(&w);
    assert_eq!(s.actor.world_pos, Vec3::new(-199.0, 2032.0, -3060.0));
    assert!(!s.is_not_death_mountain);
    assert_eq!(s.action, EncAction::Wait);
    // EnEncount2_Init: LINK_IS_ADULT && GET_EVENTCHKINF(EVENTCHKINF_49) kills it.
    let pos = Vec3::new(-199.0, 2032.0, -3060.0);
    for (adult, flag, killed) in [(false, false, false), (false, true, false), (true, false, false), (true, true, true)] {
        w.save.adult = adult;
        if flag {
            w.save.set_event_chk_inf(EVENTCHKINF_49);
        } else {
            w.save.clear_event_chk_inf(EVENTCHKINF_49);
        }
        let h = w.actor_spawn(ACTOR_EN_ENCOUNT2, pos, [0; 3], 5).expect("En_Encount2 spawned");
        assert_eq!(w.actors.actor(h).unwrap().killed, killed, "adult {adult}, EVENTCHKINF_49 {flag}");
        w.actors.actor_mut(h).unwrap().kill();
    }
}

#[test]
fn out_of_its_area_the_spawner_waits() {
    let Some(a) = assets() else { return };
    let mut w = summit(&a);
    // Spawn 4 is at z -4497, below EnEncount2_Wait's -3600.
    assert!(link_pos(&w).z < -3600.0);
    idle(&mut w, 30);
    let s = enc(&w);
    assert_eq!(s.action, EncAction::Wait);
    assert!(!s.is_quaking);
    assert_eq!((s.env_effects_timer, s.num_spawned_rocks), (0, 0));
    assert!(s.effects.iter().all(|e| !e.is_alive));
    assert!(all::<EnFireRock>(&w).is_empty());
    assert_eq!(sfx_count(&w, NA_SE_EV_VOLCANO - SFX_FLAG), 0);
    // The darkening at 0: (s16)(0 * -50) * -1.5 and the rest 0.
    assert_eq!(w.env_ctx.adj_ambient_color, [0, 0, 0]);
    assert_eq!(w.env_ctx.adj_light1_color, [0, 0, 0]);
    assert_eq!(w.env_ctx.adj_fog_near, 0);
}

#[test]
fn in_its_area_a_round_starts_at_once_with_the_quake_and_two_rocks_four_frames_apart() {
    let Some(a) = assets() else { return };
    let mut w = summit(&a);
    w.save.health = w.save.health_capacity;
    w.place_player(LINK_AT, 0);
    idle(&mut w, 1);
    let sh = spawner(&w);
    // EnEncount2_Wait, Link in the area: no quake running, so Quake_Request (type 1) with speed
    // 0x7FFF, 50 up and down, 300 frames; envEffectsTimer 0 -> 1; its timer at 0: 200, the
    // count 0, EnEncount2_SpawnRocks.
    let s = enc(&w);
    assert_eq!(s.action, EncAction::SpawnRocks);
    assert_eq!(s.death_mountain_spawner_timer, 200);
    assert!(s.is_quaking);
    assert_eq!(s.env_effects_timer, 1);
    let q = quakes(&w, QUAKE_TYPE_1, 50);
    assert_eq!(q.len(), 1);
    assert_eq!(q[0].speed, 0x7FFF);
    assert_eq!(q[0].duration, 300);
    assert!((299..=300).contains(&q[0].timer), "{:?}", q[0]);
    assert!(all::<EnFireRock>(&w).is_empty());

    // The next frame (EnEncount2_SpawnRocks, its timer 199): the volcano's sound, an ember, and a
    // rock spawned as its child: within 5 of Link across (a child's FALLING1 by
    // Rand_ZeroFloat(1.99) < 1, else FALLING2, which also starts by Link while he stands still),
    // 400 above the point 160 ahead of the camera by its pitch.
    let eye_y = w.view.eye.y;
    idle(&mut w, 1);
    let lp = link_pos(&w);
    let s = enc(&w);
    assert_eq!(s.death_mountain_spawner_timer, 199);
    assert_eq!(s.num_spawned_rocks, 1);
    assert_eq!(s.timer_between_rock_spawns, 4);
    assert_eq!(s.env_effects_timer, 2);
    assert_eq!(s.effects.iter().filter(|e| e.is_alive).count(), 1);
    assert_eq!(sfx_count(&w, NA_SE_EV_VOLCANO - SFX_FLAG), 1);
    let first = all::<EnFireRock>(&w);
    assert_eq!(first.len(), 1);
    let r = rock(&w, first[0]);
    assert!(r.ty == FIRE_ROCK_SPAWNED_FALLING1 || r.ty == FIRE_ROCK_SPAWNED_FALLING2);
    assert_eq!(r.spawner, Some(sh));
    assert_eq!(r.actor.parent, Some(sh));
    assert!((r.actor.home_pos.x - lp.x).abs() <= 5.0 && (r.actor.home_pos.z - lp.z).abs() <= 5.0, "{:?} vs Link {lp:?}", r.actor.home_pos);
    // eye.y + dir.y × 160 + 400, dir.y in -1..1.
    assert!((r.actor.home_pos.y - (eye_y + 400.0)).abs() <= 160.0 + 1.0, "{} vs eye {eye_y}", r.actor.home_pos.y);

    // timerBetweenRockSpawns 4: counted down 3, 2, 1, then 0 on the fourth frame on: the second.
    for k in 1..=3 {
        idle(&mut w, 1);
        assert_eq!(enc(&w).timer_between_rock_spawns, 4 - k);
        assert_eq!(spawned_by(&w, sh), 1, "frame {k}");
    }
    idle(&mut w, 1);
    assert_eq!(enc(&w).num_spawned_rocks, 2);
    assert_eq!(spawned_by(&w, sh), 2);
    // Two at most at once; every rock it spawned counts itself off at its destroy, so the count
    // is always the rocks of its still in the arena; as they break, others come.
    let mut seen: BTreeSet<ActorHandle> = BTreeSet::new();
    for k in 0..120 {
        idle(&mut w, 1);
        w.save.health = w.save.health_capacity;
        let s = enc(&w);
        assert!(s.num_spawned_rocks <= 2, "frame {k}");
        assert_eq!(s.num_spawned_rocks as usize, spawned_by(&w, sh), "frame {k}");
        for h in all::<EnFireRock>(&w) {
            if rock(&w, h).spawner == Some(sh) {
                seen.insert(h);
            }
        }
    }
    assert!(seen.len() > 2, "more rocks after the first two broke: {}", seen.len());
}

/// Link in the area with the rocks killed as they come (so they don't hurt him): `n` frames.
fn run_killing_rocks(w: &mut PlayState, n: usize, mut each: impl FnMut(&PlayState, usize)) {
    for k in 0..n {
        for h in all::<EnFireRock>(w) {
            w.actors.actor_mut(h).unwrap().kill();
        }
        idle(w, 1);
        each(w, k);
    }
}

#[test]
fn a_round_lasts_200_frames_and_the_wait_100_with_the_quake_again() {
    let Some(a) = assets() else { return };
    let mut w = summit(&a);
    w.place_player(LINK_AT, 0);
    idle(&mut w, 1);
    assert_eq!(enc(&w).action, EncAction::SpawnRocks);
    // EnEncount2_SpawnRocks with its timer from 199 to 1 (199 frames); at 0 (the 200th) the
    // wait, 100, without spawning.
    run_killing_rocks(&mut w, 199, |w, k| assert_eq!(enc(w).action, EncAction::SpawnRocks, "frame {k}"));
    assert_eq!(enc(&w).death_mountain_spawner_timer, 1);
    run_killing_rocks(&mut w, 1, |_, _| {});
    assert_eq!(enc(&w).action, EncAction::Wait);
    assert_eq!(enc(&w).death_mountain_spawner_timer, 100);
    // EnEncount2_Wait: the timer 99 down; at 1 (the 99th frame) the quake is asked for again
    // (the first, 300 frames, all but out); at 0 a round of 200.
    run_killing_rocks(&mut w, 98, |w, k| assert_eq!(enc(w).action, EncAction::Wait, "frame {k}"));
    assert_eq!(enc(&w).death_mountain_spawner_timer, 2);
    assert!(quakes(&w, QUAKE_TYPE_1, 50).iter().all(|r| r.timer < 10), "{:?}", quakes(&w, QUAKE_TYPE_1, 50));
    run_killing_rocks(&mut w, 1, |_, _| {});
    assert_eq!(enc(&w).action, EncAction::Wait);
    assert!(quakes(&w, QUAKE_TYPE_1, 50).iter().any(|r| r.duration == 300 && r.timer >= 299), "a fresh quake: {:?}", quakes(&w, QUAKE_TYPE_1, 50));
    run_killing_rocks(&mut w, 1, |_, _| {});
    assert_eq!(enc(&w).action, EncAction::SpawnRocks);
    assert_eq!(enc(&w).death_mountain_spawner_timer, 200);
}

#[test]
fn the_trail_darkens_over_60_frames_and_lifts_in_the_wait_out_of_the_area() {
    let Some(a) = assets() else { return };
    let mut w = summit(&a);
    w.place_player(LINK_AT, 0);
    // envEffectsTimer up 1 a frame (Wait's first frame, then SpawnRocks') to 60.
    run_killing_rocks(&mut w, 70, |w, k| assert_eq!(enc(w).env_effects_timer, (k as i16 + 1).min(60), "frame {k}"));
    // EnEncount2_Update at 60: unk_17C 1; unk_160 -50: adjAmbientColor[0] (s16)-50 × -1.5 = 75,
    // [1], [2] -50; unk_168 -20: adjLight1Color (30, -20, -20); adjFogNear -50; adjFogColor
    // (u8)((160 - fogColor) × 1), (u8)((150 - fogColor[2]) × 1).
    assert_eq!(w.env_ctx.adj_ambient_color, [75, -50, -50]);
    assert_eq!(w.env_ctx.adj_light1_color, [30, -20, -20]);
    assert_eq!(w.env_ctx.adj_fog_near, -50);
    let fog = w.env_ctx.light_settings.fog_color;
    let want = [160.0, 160.0, 150.0];
    for i in 0..3 {
        assert_eq!(w.env_ctx.adj_fog_color[i], (((want[i] - fog[i] as f32) * 1.0) as i32 as u8) as i16, "fog {i}");
    }
    // Link out of the area: the round goes on (SpawnRocks still counts envEffectsTimer up, at
    // most 60) until its end; then the wait counts it down 1 a frame and drops the quake flag.
    w.place_player(Vec3::new(-249.0, 3314.0, -4497.0), 0);
    let mut k = 0;
    while enc(&w).action == EncAction::SpawnRocks {
        run_killing_rocks(&mut w, 1, |_, _| {});
        assert_eq!(enc(&w).env_effects_timer, 60);
        k += 1;
        assert!(k < 200);
    }
    // The frame the round ended (the timer at 0) it counted up first: still 60.
    for n in 1..=60 {
        run_killing_rocks(&mut w, 1, |_, _| {});
        assert_eq!(enc(&w).env_effects_timer, 60 - n);
        assert!(!enc(&w).is_quaking);
    }
    run_killing_rocks(&mut w, 2, |_, _| {});
    assert_eq!(enc(&w).env_effects_timer, 0);
    assert_eq!(w.env_ctx.adj_ambient_color, [0, 0, 0]);
    assert_eq!(w.env_ctx.adj_fog_near, 0);
}

#[test]
fn an_ember_a_frame_falling_16_a_frame_gone_below_1500() {
    let Some(a) = assets() else { return };
    let mut w = summit(&a);
    w.place_player(LINK_AT, 0);
    idle(&mut w, 1);
    // From SpawnRocks' first frame one ember a frame (the trail's), into the first free of 50.
    // Each: 400 above the camera's ±25, so above Link (y > 1500) by more than 160 at least: none
    // goes below 1500 in its first 10 frames (16 a frame).
    let mut prev: Option<EnEncount2Effect> = None;
    run_killing_rocks(&mut w, 10, |w, k| {
        let s = enc(w);
        assert_eq!(s.effects.iter().filter(|e| e.is_alive).count(), k + 1);
        let e0 = s.effects[0];
        assert!(e0.is_alive);
        if let Some(p) = prev {
            // Math_ApproachF(pos.y, pos.y - 20, 0.8, 250): 16 down; x and z 0.3 of moveDirection
            // (±10); moveDirection.y stays -20.
            assert!((e0.pos.y - (p.pos.y - 16.0)).abs() < 1e-2, "{} vs {}", e0.pos.y, p.pos.y);
            assert!((e0.pos.x - (p.pos.x + 0.3 * p.move_direction.x)).abs() < 1e-2);
            assert_eq!(e0.move_direction.y, -20.0);
            assert!((0.0045..0.0095).contains(&e0.scale));
        }
        prev = Some(e0);
    });
    // An ember put at 1510 is gone after its next step (1494 < 1500: SCENE_DEATH_MOUNTAIN_TRAIL).
    let h = spawner(&w);
    {
        let s = w.actors.downcast_mut::<EnEncount2>(h).unwrap();
        s.effects[49] = EnEncount2Effect { pos: Vec3::new(-300.0, 1510.0, -3000.0), scale: 0.007, is_alive: true, move_direction: Vec3::new(0.0, -20.0, 0.0), rot: Vec3::ZERO };
    }
    run_killing_rocks(&mut w, 1, |_, _| {});
    assert!(!enc(&w).effects[49].is_alive);
    // The draw: one rock (Demo_Kankyo's bake) per live ember.
    let s = enc(&w);
    let live = s.effects.iter().filter(|e| e.is_alive).count();
    let mut out = DrawOut::default();
    s.draw(&s.render_state(), &w, &ViewInfo::new(Vec3::ZERO, glam::Mat4::IDENTITY), &mut out);
    assert_eq!(out.opa.len(), live);
}

#[test]
fn a_falling_rock_falls_burning_and_falling1_homes_on_link() {
    let Some(a) = assets() else { return };
    let mut w = summit(&a);
    kill_spawner(&mut w);
    put_link(&mut w, LINK_LOW);
    let lp = link_pos(&w);
    let fires = effects_of(&w, EFFECT_SS_EN_FIRE);
    // FALLING2 100 across from Link, 200 up.
    let start = lp + Vec3::new(100.0, 200.0, 0.0);
    let h = spawn_rock(&mut w, FIRE_ROCK_SPAWNED_FALLING2, start);
    let r = rock(&w, h);
    // EnFireRock_Init: scale 0.02 to 0.04, its collider D_80A12CA0, the circle shadow 15, the
    // tumble 15 to 25 a frame on each axis.
    assert!((0.02..0.04).contains(&r.scale) && r.actor.scale == Vec3::splat(r.scale));
    assert_eq!(r.action, RockAction::Fall);
    assert_eq!((r.collider.dim.radius, r.collider.dim.height, r.collider.dim.y_shift), (30, 30, -10));
    assert_eq!(r.collider.info.at_dmg_info.damage, 8);
    for v in [r.angular_velocity.x, r.angular_velocity.y, r.angular_velocity.z] {
        assert!((15.0..25.0).contains(&v));
    }
    let (scale, av) = (r.scale, r.angular_velocity);
    idle(&mut w, 1);
    let r = rock(&w, h);
    // EnFireRock_Update: gravity -0.3 - scale × 7, Actor_MoveXZGravity from rest: velocity.y the
    // gravity, y + gravity × 1.5 (Actor_UpdatePos' R_UPDATE_RATE × 0.5);
    // across it stays (FALLING2); the shadow 10 + scale × 300 clamped to 10..20; rockRotation
    // one step; relativePos.y 3; a flame (EffectSsEnFire, 100) on it.
    let g = -0.3 - scale * 7.0;
    assert_eq!(r.actor.gravity, g);
    assert_eq!(r.actor.velocity.y, g);
    assert!((r.actor.world_pos.y - (start.y + g * 1.5)).abs() < 1e-3, "{:?} from {start:?}, g {g}", r.actor.world_pos);
    assert_eq!((r.actor.world_pos.x, r.actor.world_pos.z), (start.x, start.z));
    assert_eq!(r.actor.shape_shadow_scale, (10.0 + scale * 300.0).clamp(10.0, 20.0));
    assert_eq!(r.rock_rotation, av);
    assert_eq!(r.relative_pos.y, 3.0);
    assert_eq!(effects_of(&w, EFFECT_SS_EN_FIRE), fires + 1);
    // Its draw: the rock 3 up, tumbled, at its scale; the circle shadow is Actor_Draw's (the render
    // state's, its floor 200 below, within 500).
    let mut out = DrawOut::default();
    let rs = r.render_state();
    r.draw(&rs, &w, &ViewInfo::new(Vec3::ZERO, glam::Mat4::IDENTITY), &mut out);
    assert_eq!(out.opa.len(), 1);
    assert_eq!(out.opa[0].transform.w_axis.truncate(), r.actor.world_pos + Vec3::new(0.0, 3.0, 0.0));
    assert!(out.xlu.is_empty());
    let s = rs.shadow.expect("the shape's shadow");
    assert_eq!((s.draw, s.scale, s.floor_poly.is_some()), (oot_game::shadow::ShadowDraw::Circle, r.actor.shape_shadow_scale, true));

    // FALLING1 above Link homes on him across at 10 a frame (Math_ApproachF(.., 1.0, 10)):
    // @bug (game) the area check holds wherever he is.
    let start = lp + Vec3::new(100.0, 200.0, 0.0);
    let h1 = spawn_rock(&mut w, FIRE_ROCK_SPAWNED_FALLING1, start);
    idle(&mut w, 1);
    let r1 = rock(&w, h1);
    assert!((r1.actor.world_pos.x - (start.x - 10.0)).abs() < 1e-3, "{}", r1.actor.world_pos.x);
    assert!((r1.actor.world_pos.z - lp.z).abs() < 1e-3);
    idle(&mut w, 2);
    assert!((rock(&w, h1).actor.world_pos.x - (start.x - 30.0)).abs() < 1e-3);
}

#[test]
fn a_rock_on_the_floor_breaks_in_two_and_those_in_two_more() {
    let Some(a) = assets() else { return };
    let mut w = summit(&a);
    kill_spawner(&mut w);
    put_link(&mut w, LINK_LOW);
    // FALLING2 30 above the flat spot, 200 up the trail from Link.
    let h = spawn_rock(&mut w, FIRE_ROCK_SPAWNED_FALLING2, ROCK_SPOT + Vec3::new(0.0, 30.0, 0.0));
    let mut n = 0;
    while rock(&w, h).action == RockAction::Fall {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 40, "it lands");
    }
    // Landing (BGCHECKFLAG_GROUND, timer 0): Actor_RequestQuakeAndRumble(5, 2) (type 3, 5 up and
    // down), a ring of two dust clouds, five flames (300) on the floor, then
    // EnFireRock_SpawnMoreBrokenPieces; no collider registered that frame.
    let r = rock(&w, h);
    assert!(r.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0);
    assert!((r.actor.floor_height - ROCK_SPOT.y).abs() < 2.0);
    assert_eq!(r.action, RockAction::SpawnMoreBrokenPieces);
    assert_eq!(quakes(&w, QUAKE_TYPE_3, 5).len(), 1);
    assert!(effects_of(&w, EFFECT_SS_DUST) >= 2);
    let (pos, rot_y, shape_y, scale) = (r.actor.world_pos, r.actor.world_rot.y, r.actor.shape_rot.y, r.scale);
    let valvaisa = sfx_count(&w, NA_SE_EN_VALVAISA_ROCK);
    idle(&mut w, 1);
    // The next frame: two FIRE_ROCK_BROKEN_PIECE1, 10 up (±1.5), along its way (the first turned
    // as it was), its scale less 0.01 (the field), NA_SE_EN_VALVAISA_ROCK; the rock gone.
    assert!(w.actors.actor(h).is_none_or(|a| a.killed));
    assert!(sfx_count(&w, NA_SE_EN_VALVAISA_ROCK) > valvaisa);
    let pieces = rocks_of(&w, FIRE_ROCK_BROKEN_PIECE1);
    assert_eq!(pieces.len(), 2);
    for (i, &p) in pieces.iter().enumerate() {
        let r = rock(&w, p);
        assert_eq!(r.actor.world_rot.y, rot_y);
        assert_eq!(r.scale, scale - 0.01);
        assert!((r.actor.home_pos - (pos + Vec3::new(0.0, 10.0, 0.0))).abs().max_element() <= 1.5);
        assert_eq!(r.actor.gravity, -1.5);
        assert!((3.0..6.0).contains(&r.actor.speed_xz));
        assert_eq!(r.spawner, None);
        let _ = i;
    }
    assert!(pieces.iter().any(|&p| rock(&w, p).actor.shape_rot.y == shape_y));
    // Everything breaks or falls away in the end (the small pieces land and are gone).
    let mut n = 0;
    while !all::<EnFireRock>(&w).is_empty() {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 200, "the pieces are gone");
    }
}

#[test]
fn the_small_piece_lands_with_the_explosion_and_is_gone() {
    let Some(a) = assets() else { return };
    let mut w = summit(&a);
    kill_spawner(&mut w);
    put_link(&mut w, LINK_LOW);
    let h = spawn_rock(&mut w, FIRE_ROCK_BROKEN_PIECE2, ROCK_SPOT + Vec3::new(0.0, 5.0, 0.0));
    let r = rock(&w, h);
    // EnFireRock_Init: 4 to 7 up, 2 to 5 along, gravity -1.2, scale 0.01 to 0.012, no collider.
    assert!((4.0..7.0).contains(&r.actor.velocity.y) && (2.0..5.0).contains(&r.actor.speed_xz));
    assert_eq!(r.actor.gravity, -1.2);
    assert!((0.01..0.012).contains(&r.scale));
    let mut n = 0;
    while w.actors.actor(h).is_some_and(|a| !a.killed) {
        assert_eq!(sfx_count(&w, NA_SE_EV_EXPLOSION), 0);
        idle(&mut w, 1);
        n += 1;
        assert!(n < 40, "it lands");
    }
    // The default landing: a ring of four dust clouds, NA_SE_EV_EXPLOSION at its place for 40
    // frames (SfxSource_PlaySfxAtFixedWorldPos), killed; no pieces.
    assert_eq!(sfx_count(&w, NA_SE_EV_EXPLOSION), 1);
    assert!(matches!(sfx_pos(&w, NA_SE_EV_EXPLOSION)[0], SfxPos::Source(_)));
    assert!(effects_of(&w, EFFECT_SS_DUST) >= 4);
    assert!(all::<EnFireRock>(&w).is_empty());
}

#[test]
fn a_falling_rock_on_link_hits_him_for_8() {
    let Some(a) = assets() else { return };
    let mut w = summit(&a);
    kill_spawner(&mut w);
    put_link(&mut w, LINK_LOW);
    idle(&mut w, 5);
    let health = w.save.health;
    let lp = link_pos(&w);
    // FALLING2 right on Link: its cylinder (30 round, 30 high, 10 down) on his.
    let h = spawn_rock(&mut w, FIRE_ROCK_SPAWNED_FALLING2, lp + Vec3::new(0.0, 20.0, 0.0));
    let mut n = 0;
    while w.player().invincibility_timer == 0 {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 4, "the rock hits Link");
    }
    // D_80A12CA0's attack: 8 (half a heart), HIT_SPECIAL_EFFECT_9 (as none): Link staggers
    // (func_80837C0C), 20 frames' invincibility.
    assert_eq!(w.save.health, health - 8);
    assert_eq!(w.player().invincibility_timer, 20);
    assert_eq!(w.player().action, PlayerAction::Damaged);
    assert!(!w.player().body_is_burning);
    // The rock falls on (its AT_HIT is reset by its next CollisionCheck_SetAT).
    assert_eq!(rock(&w, h).action, RockAction::Fall);
}

#[test]
fn a_rock_turned_by_a_shield_stops_and_breaks() {
    let Some(a) = assets() else { return };
    let mut w = summit(&a);
    kill_spawner(&mut w);
    put_link(&mut w, LINK_LOW);
    let h = spawn_rock(&mut w, FIRE_ROCK_SPAWNED_FALLING2, ROCK_SPOT + Vec3::new(0.0, 100.0, 0.0));
    idle(&mut w, 3);
    assert_eq!(rock(&w, h).action, RockAction::Fall);
    // AT_BOUNCED (the shield's AC_HARD) as CollisionCheck_AT leaves it: in the next update
    // NA_SE_EV_BRIDGE_OPEN_STOP, velocity.y and speed 0, EnFireRock_SpawnMoreBrokenPieces.
    w.actors.downcast_mut::<EnFireRock>(h).unwrap().collider.base.at_flags |= AT_BOUNCED;
    idle(&mut w, 1);
    let r = rock(&w, h);
    assert_eq!(r.action, RockAction::SpawnMoreBrokenPieces);
    assert_eq!((r.actor.velocity.y, r.actor.speed_xz), (0.0, 0.0));
    assert_eq!(r.collider.base.at_flags & AT_BOUNCED, 0);
    assert_eq!(sfx_count(&w, NA_SE_EV_BRIDGE_OPEN_STOP), 1);
    idle(&mut w, 1);
    assert_eq!(rocks_of(&w, FIRE_ROCK_BROKEN_PIECE1).len(), 2);
}

#[test]
fn the_ceiling_spawner_drops_a_rock_when_link_comes_near() {
    let Some(a) = assets() else { return };
    let mut w = summit(&a);
    kill_spawner(&mut w);
    put_link(&mut w, LINK_LOW);
    w.save.health = w.save.health_capacity;
    let lp = link_pos(&w);
    // FIRE_ROCK_CEILING_SPOT_SPAWNER (Ganon's Tower's collapse places them): not drawn, no shadow.
    let c = spawn_rock(&mut w, FIRE_ROCK_CEILING_SPOT_SPAWNER, lp + Vec3::new(0.0, 150.0, 120.0));
    assert_eq!(rock(&w, c).action, RockAction::WaitSpawnRocksFromCeiling);
    let mut out = DrawOut::default();
    let r = rock(&w, c);
    r.draw(&r.render_state(), &w, &ViewInfo::new(Vec3::ZERO, glam::Mat4::IDENTITY), &mut out);
    assert!(out.opa.is_empty() && out.xlu.is_empty());
    // Link 120 across (< 200), not near before, timer2 0: a FALLING2 10 up (±1.5 across) with
    // timer 10, timer2 30, playerNearby 1.
    idle(&mut w, 1);
    let made = rocks_of(&w, FIRE_ROCK_SPAWNED_FALLING2);
    assert_eq!(made.len(), 1);
    let r = rock(&w, made[0]);
    let cp = rock(&w, c).actor.world_pos;
    assert!((r.actor.home_pos.y - (cp.y + 10.0)).abs() < 1e-3);
    assert!((r.actor.home_pos.x - cp.x).abs() <= 1.5 && (r.actor.home_pos.z - cp.z).abs() <= 1.5);
    assert!((9..=10).contains(&r.timer), "{}", r.timer);
    assert_eq!((rock(&w, c).timer2, rock(&w, c).player_nearby), (30, 1));
    // While Link stays near, no more (40 frames: past timer2).
    let mut seen: BTreeSet<ActorHandle> = made.into_iter().collect();
    for _ in 0..40 {
        idle(&mut w, 1);
        w.save.health = w.save.health_capacity;
        seen.extend(rocks_of(&w, FIRE_ROCK_SPAWNED_FALLING2));
    }
    assert_eq!(seen.len(), 1);
    // Away (300 across) a frame: playerNearby 0; back: another.
    w.place_player(lp + Vec3::new(0.0, 0.0, -300.0), 0);
    idle(&mut w, 1);
    assert_eq!(rock(&w, c).player_nearby, 0);
    w.place_player(lp, 0);
    idle(&mut w, 1);
    seen.extend(rocks_of(&w, FIRE_ROCK_SPAWNED_FALLING2));
    assert_eq!(seen.len(), 2);
}

#[test]
fn the_rock_on_the_floor_burns_and_sets_link_on_fire() {
    let Some(a) = assets() else { return };
    let mut w = summit(&a);
    kill_spawner(&mut w);
    put_link(&mut w, LINK_LOW);
    let fires = effects_of(&w, EFFECT_SS_EN_FIRE);
    let h = spawn_rock(&mut w, FIRE_ROCK_ON_FLOOR, ROCK_SPOT);
    // EnFireRock_Init: scale 0.03, D_80A12CCC (fire) at 23 by 37, 10 down, a prop, immovable.
    let r = rock(&w, h);
    assert_eq!(r.actor.scale, Vec3::splat(0.03));
    assert_eq!(r.actor.category, ACTORCAT_PROP);
    assert_eq!(r.actor.col_chk_info.mass, MASS_IMMOVABLE);
    assert_eq!((r.collider.dim.radius, r.collider.dim.height, r.collider.dim.y_shift), (23, 37, -10));
    assert_eq!(r.collider.info.at_dmg_info.hit_special_effect, oot_game::collision_check::HIT_SPECIAL_EFFECT_FIRE);
    assert_eq!(r.action, RockAction::WaitOnFloor);
    // FireRock_WaitOnFloor: timer2 0, so a flame and timer2 3 + (s16)Rand_ZeroFloat(3): 3 to 5.
    idle(&mut w, 1);
    assert_eq!(effects_of(&w, EFFECT_SS_EN_FIRE), fires + 1);
    assert!((3..=5).contains(&rock(&w, h).timer2));
    // It doesn't move (no Actor_MoveXZGravity for it).
    idle(&mut w, 5);
    assert_eq!(rock(&w, h).actor.world_pos, ROCK_SPOT);
    // Link walked into it (put on it): its fire attack (8, HIT_SPECIAL_EFFECT_FIRE) sets him
    // burning.
    let health = w.save.health;
    w.place_player(ROCK_SPOT, 0);
    let mut n = 0;
    while w.player().invincibility_timer == 0 {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 5, "the rock hits Link");
    }
    assert!(w.save.health <= health - 8, "{} from {health}", w.save.health);
    assert!(w.player().body_is_burning);
}

