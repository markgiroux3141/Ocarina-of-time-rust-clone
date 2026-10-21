//! GAME-10 milestone 1 against the C: the bugs (`z_en_insect.c`). Each type's init, the crawling
//! spells and the run from Link, the Gold Skulltula soil in the graveyard (`spot02`'s
//! `Obj_Makekinsuta` 0x5101 at (-715, 120, -340)), Zora's River's water, the bottle's offer and a
//! set `parent`, the dropped bugs, and `Obj_Mure`'s bug groups in a grotto (`kakusiana` room 8:
//! two of 0x5323, five permanent bugs each).
//!
//! Where the C draws random numbers the expected values replay them on a copy of the game's
//! `Rand` (the C's order), never on a lucky outcome. Expected values are worked out from the C in
//! the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_insect::*;
use oot_actors::en_sw::EnSw;
use oot_actors::obj_makekinsuta::ObjMakekinsuta;
use oot_actors::obj_mure::ObjMure;
use oot_game::actor::*;
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::audio::sfx::SfxPos;
use oot_game::item::GI_MAX;
use oot_game::play::{PlayState, Rand, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// `spot02` layers 0 and 1: `Obj_Makekinsuta` params 0x5101 (gsFlags index 0x11 - 1 = 0x10,
/// flag 1), over `Obj_Bean`'s soft soil.
const SOIL_POS: Vec3 = Vec3::new(-715.0, 120.0, -340.0);
/// `EVENTCHKINF_B6` (`save.h`): the graveyard's intro (`gGraveyardIntroCs`) seen.
const EVENTCHKINF_B6: u16 = 0xB6;
/// Zora's River (`spot03` room 0): a blue Tektite's home, on the water.
const RIVER: Vec3 = Vec3::new(-122.0, 180.0, -1252.0);

/// Enters `entrance` as a child at `hour`:00 with `edit` applied to the save, the sound log on,
/// two frames run.
fn enter(a: &Arc<GameAssets>, entrance: &str, hour: i32, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(hour, 0) as u16);
    save.apply_preset("deku-tree-dead").unwrap();
    save.adult = false;
    edit(&mut save);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    w
}

/// The graveyard (Link at spawn 0, (-1411, 0, 330), far from the soil), at 10:00; 20 frames
/// more, past the scene's first frames, where `Bg_Spot02_Objects` changing its category walks the
/// BG update loop on into the ITEMACTION list (an actor spawned then updates twice).
fn graveyard(a: &Arc<GameAssets>, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    let mut w = enter(a, "ENTR_GRAVEYARD_0", 10, |s| {
        s.set_event_chk_inf(EVENTCHKINF_B6);
        edit(s);
    });
    idle(&mut w, 20);
    w
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn all<T: 'static + ActorImpl>(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<T>(h).is_some_and(|a| !a.base().killed)).collect()
}

fn bug(w: &PlayState, h: ActorHandle) -> &EnInsect {
    w.actors.downcast::<EnInsect>(h).expect("En_Insect")
}

fn bug_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnInsect {
    w.actors.downcast_mut::<EnInsect>(h).expect("En_Insect")
}

fn spawn(w: &mut PlayState, pos: Vec3, rot: [i16; 3], params: i16) -> ActorHandle {
    w.actor_spawn(ACTOR_EN_INSECT, pos, rot, params).expect("En_Insect spawned")
}

/// The frames a sound was asked for (at `pos`, or anywhere).
fn sfx_frames(w: &PlayState, id: u16, pos: Option<SfxPos>) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, p)| s == id && pos.is_none_or(|q| q == p)).map(|&(f, _, _)| f).collect()
}

/// Runs until `done` or `max` frames; the frames run.
fn run_until(w: &mut PlayState, max: usize, mut done: impl FnMut(&PlayState) -> bool) -> usize {
    for n in 0..max {
        if done(w) {
            return n;
        }
        idle(w, 1);
    }
    panic!("not reached in {max} frames");
}

/// `EnInsect_SetupDropped`'s draws: `unk_328`, `unk_316`, `world.rot.y`.
fn setup_dropped_draws(r: &mut Rand) -> (i16, i16, i16) {
    let unk_328 = (r.zero_one() * (0xFFFF as f32 + 0.5)) as i32 as i16;
    let unk_316 = ((r.zero_one() - 0.5) * 1500.0) as i32 as i16;
    let yaw = (r.zero_one() * (0xFFFF as f32 + 0.5)) as i32 as i16;
    (unk_328, unk_316, yaw)
}

/// `EnInsect_Init`'s last draw for types 0 and 1 and its setup's timer: (the action, the timer).
fn first_spell(r: &mut Rand) -> (Action, i16) {
    let rand = r.zero_one();
    if rand < 0.3 {
        (Action::SlowDown, r.s16_offset(5, 35))
    } else if rand < 0.4 {
        (Action::Crawl, r.s16_offset(10, 45))
    } else {
        (Action::RunFromPlayer, r.s16_offset(10, 40))
    }
}

#[test]
fn each_type_initialises_as_the_c_does() {
    let Some(a) = assets() else { return };
    let mut w = graveyard(&a, |_| {});
    let at = Vec3::new(-1200.0, 0.0, 330.0);

    // INSECT_TYPE_PERMANENT: sInitInsectFlags[0] (none), so no gravity and no life; one draw
    // picks the spell, its setup another.
    let mut r = w.rand;
    let (spell, timer) = first_spell(&mut r);
    let h = spawn(&mut w, at, [0, 0x1000, 0], INSECT_TYPE_PERMANENT);
    let b = bug(&w, h);
    assert_eq!(w.rand, r, "the C's draws, in its order");
    assert_eq!((b.action, b.action_timer, b.life_timer, b.insect_flags), (spell, timer, 0, INSECT_FLAG_CRAWLING));
    assert_eq!((b.actor.gravity, b.actor.min_velocity_y, b.actor.col_chk_info.mass), (0.0, -20.0, 30));
    assert_eq!((b.actor.scale, b.actor.culling_volume_distance, b.actor.culling_volume_scale, b.actor.culling_volume_downward), (Vec3::splat(0.01), 700.0, 20.0, 600.0));
    assert_eq!(b.actor.flags & ACTOR_FLAG_UPDATE_CULLING_DISABLED, 0);
    assert_eq!(b.actor.shape_shadow_draw, oot_game::shadow::ShadowDraw::None, "no ActorShape_Init: no shadow");
    assert!(b.drawn);

    // INSECT_TYPE_SPAWNED (from a rock or a bush): INSECT_FLAG_0 and IS_SHORT_LIVED; gravity
    // -0.2 down to -2, a life of Rand_S16Offset(200, 40), updating out of view.
    let mut r = w.rand;
    let life = r.s16_offset(200, 40);
    let (spell, timer) = first_spell(&mut r);
    let h = spawn(&mut w, at, [0, 0, 0], INSECT_TYPE_SPAWNED);
    let b = bug(&w, h);
    assert_eq!(w.rand, r);
    assert!((200..240).contains(&life));
    assert_eq!((b.action, b.action_timer, b.life_timer), (spell, timer, life));
    assert_eq!(b.insect_flags, INSECT_FLAG_0 | INSECT_FLAG_IS_SHORT_LIVED | INSECT_FLAG_CRAWLING);
    assert_eq!((b.actor.gravity, b.actor.min_velocity_y), (-0.2, -2.0));
    assert_ne!(b.actor.flags & ACTOR_FLAG_UPDATE_CULLING_DISABLED, 0);
    assert_eq!(overlay_statics(&w).dropped_count, 0);

    // INSECT_TYPE_FIRST_DROPPED, as the bottle lets it out (rotation (0x4000, yaw, 0); here a z
    // of 5, which its init zeroes): its life drawn, then two INSECT_TYPE_EXTRA_DROPPED spawned at
    // its position and shape rotation (each: its life, no soil within 80, EnInsect_SetupDropped),
    // then its own EnInsect_SetupDropped; sDroppedCount 3.
    let before = all::<EnInsect>(&w);
    let mut r = w.rand;
    let life = r.s16_offset(200, 40);
    let mut extras = Vec::new();
    for _ in 0..2 {
        let l = r.s16_offset(200, 40);
        extras.push((l, setup_dropped_draws(&mut r)));
    }
    let (unk_328, unk_316, yaw) = setup_dropped_draws(&mut r);
    let h = spawn(&mut w, at, [0x4000, 0x2000, 5], INSECT_TYPE_FIRST_DROPPED);
    assert_eq!(w.rand, r);
    let b = bug(&w, h);
    assert_eq!((b.action, b.action_timer, b.life_timer, b.unk_324, b.unk_328, b.unk_316), (Action::Dropped, 100, life, 1.5, unk_328, unk_316));
    assert_eq!((b.actor.world_rot.y, b.actor.world_rot.x, b.actor.world_rot.z, b.actor.shape_rot.z), (yaw, 0x4000, 0, 0));
    assert_eq!(b.actor.scale, Vec3::splat(0.003));
    assert_eq!(b.insect_flags, INSECT_FLAG_0 | INSECT_FLAG_1 | INSECT_FLAG_IS_SHORT_LIVED | INSECT_FLAG_CRAWLING);
    assert_eq!(b.soil_actor, None);
    let new: Vec<ActorHandle> = all::<EnInsect>(&w).into_iter().filter(|x| !before.contains(x) && *x != h).collect();
    assert_eq!(new.len(), 2);
    let mut got: Vec<(i16, (i16, i16, i16))> = new.iter().map(|&x| (bug(&w, x).life_timer, (bug(&w, x).unk_328, bug(&w, x).unk_316, bug(&w, x).actor.world_rot.y))).collect();
    got.sort();
    extras.sort();
    assert_eq!(got, extras);
    for &x in &new {
        let e = bug(&w, x);
        assert_eq!((e.actor.params, e.action, e.actor.world_pos, e.actor.shape_rot.x, e.actor.shape_rot.z, e.actor.parent), (INSECT_TYPE_EXTRA_DROPPED, Action::Dropped, at, 0x4000, 0, None));
        assert_eq!(e.insect_flags, INSECT_FLAG_0 | INSECT_FLAG_1 | INSECT_FLAG_IS_SHORT_LIVED | INSECT_FLAG_CRAWLING);
    }
    assert_eq!(overlay_statics(&w).dropped_count, 3);
}

#[test]
fn a_bug_crawls_home_turns_and_runs_from_link() {
    let Some(a) = assets() else { return };
    let mut w = graveyard(&a, |_| {});
    // A permanent bug (no gravity: it keeps its height) 100 east of its home, Link far.
    let home = Vec3::new(-1100.0, 0.0, 500.0);
    let h = spawn(&mut w, home, [0, 0, 0], INSECT_TYPE_PERMANENT);
    // Not updating until a draw finds it in its culling volume (no
    // ACTOR_FLAG_UPDATE_CULLING_DISABLED for a permanent bug).
    idle(&mut w, 1);
    {
        let b = bug_mut(&mut w, h);
        b.actor.home_pos = home;
        b.actor.world_pos = home + Vec3::new(100.0, 0.0, 0.0);
        b.action = Action::Crawl;
        b.action_timer = 30;
        b.actor.world_rot.y = 0;
        b.actor.speed_xz = 0.0;
        b.crawl_sound_delay = 0;
    }
    let start = bug(&w, h).actor.world_pos;
    idle(&mut w, 1);
    // EnInsect_Crawl: Math_SmoothStepToF(speed, 1.5, 0.1, 0.5, 0): 0.15. Beyond 40 of home
    // (1600 squared), it turns to it (west, -0x4000) by Math_ScaledStepToS's 2000 (× 1.5, the
    // update rate: 3000); the shape follows. playSpeed CLAMP(0.15 × 1.4, 0.7, 1.9) = 0.7; the
    // walk sound at once, then 3 / 0.7 = 4 frames to the next.
    let b = bug(&w, h);
    assert_eq!((b.action, b.action_timer), (Action::Crawl, 29));
    assert!((b.actor.speed_xz - 0.15).abs() < 1e-6);
    assert_eq!((b.actor.world_rot.y, b.actor.shape_rot.y), (-3000, -3000));
    assert_eq!(b.skel.play_speed, 0.7);
    assert_eq!(b.crawl_sound_delay, 4);
    assert_eq!(sfx_frames(&w, NA_SE_EN_MUSI_WALK, Some(SfxPos::Actor(h))).len(), 1);
    // Actor_MoveXZGravity along the new heading (× 1.5); no gravity.
    let moved = bug(&w, h).actor.world_pos - start;
    let (vx, vz) = (eng_math::sin_s(-3000) * 0.15 * 1.5, eng_math::cos_s(-3000) * 0.15 * 1.5);
    assert!((moved.x - vx).abs() < 1e-4 && (moved.z - vz).abs() < 1e-4 && moved.y == 0.0, "{moved}");
    // The delay counted down 4 frames, then on the fifth the next sound: the speed then 0.7028
    // (smooth-stepped 10% a frame), playSpeed 0.984, 3 frames to the next.
    idle(&mut w, 4);
    assert_eq!((bug(&w, h).crawl_sound_delay, sfx_frames(&w, NA_SE_EN_MUSI_WALK, Some(SfxPos::Actor(h))).len()), (0, 1));
    // -3000 a frame: facing home on the sixth (-15000, then the yaw to home from where it was).
    assert_eq!(bug(&w, h).actor.world_rot.y, -15000);
    let from = bug(&w, h).actor.world_pos;
    idle(&mut w, 1);
    assert_eq!(bug(&w, h).actor.world_rot.y, eng_math::vec3f_yaw(from, home));
    assert_eq!((bug(&w, h).crawl_sound_delay, sfx_frames(&w, NA_SE_EN_MUSI_WALK, Some(SfxPos::Actor(h))).len()), (3, 2));

    // Link 30 from it: the next update's test (within 40) hands it to EnInsect_RunFromPlayer.
    let pos = bug(&w, h).actor.world_pos;
    w.place_player(pos + Vec3::new(0.0, 0.0, 30.0), -0x8000);
    idle(&mut w, 1);
    let b = bug(&w, h);
    assert_eq!(b.action, Action::RunFromPlayer);
    // Its timer: EnInsect_SetupRunFromPlayer's Rand_S16Offset(10, 40).
    assert!((10..50).contains(&b.action_timer));
    // Running: away from Link (yawTowardsPlayer + 0x8000), +0x2000 when state.frames has bits 4
    // and 5, -0x2000 with only bit 5, 2000 a frame (yawTowardsPlayer as Actor_UpdateAll set it
    // before the update; state.frames as the update saw it, before GameState_Update's ++).
    let mut checked = 0;
    for _ in 0..40 {
        let before = bug(&w, h).actor.world_rot.y;
        let f = w.state_frames as i16;
        idle(&mut w, 1);
        let b = bug(&w, h);
        if b.action != Action::RunFromPlayer {
            break;
        }
        if b.action_timer < 4 || xz_distance_squared(b.actor.world_pos, b.actor.home_pos) > 25600.0 {
            continue;
        }
        let mut yaw = b.actor.yaw_towards_player.wrapping_add(i16::MIN);
        if f & 0x10 != 0 {
            if f & 0x20 != 0 {
                yaw = yaw.wrapping_add(0x2000);
            }
        } else if f & 0x20 != 0 {
            yaw = yaw.wrapping_sub(0x2000);
        }
        let mut expect = before;
        eng_math::scaled_step_to_s(&mut expect, yaw, 2000);
        assert_eq!(b.actor.world_rot.y, expect);
        assert_eq!(b.actor.shape_rot.y, expect);
        checked += 1;
    }
    assert!(checked > 0);
}

#[test]
fn a_dropped_bug_on_the_soil_digs_in_and_makes_the_gold_skulltula() {
    let Some(a) = assets() else { return };
    let mut w = graveyard(&a, |_| {});
    let soil = all::<ObjMakekinsuta>(&w)[0];
    assert_eq!(w.actors.actor(soil).unwrap().world_pos, SOIL_POS);
    // An extra dropped bug right on the soil: EnInsect_TryFindNearbySoil finds it (0 across,
    // same room), INSECT_FLAG_FOUND_SOIL, D_80A7DEB0 zeroed.
    let h = spawn(&mut w, SOIL_POS, [0, 0, 0], INSECT_TYPE_EXTRA_DROPPED);
    let b = bug(&w, h);
    assert_eq!(b.soil_actor, Some(soil));
    assert_ne!(b.insect_flags & INSECT_FLAG_FOUND_SOIL, 0);
    assert_eq!(overlay_statics(&w).d_80a7deb0, 0.0);
    // Its first update: EnInsect_Dropped, the soil within 3 (its distance taken before it
    // moves): EnInsect_SetupDig: 60 frames, NA_SE_EN_MUSI_SINK, home where it is, uncatchable,
    // not crawling.
    idle(&mut w, 1);
    let b = bug(&w, h);
    assert_eq!((b.action, b.action_timer), (Action::Dig, 60));
    assert_ne!(b.insect_flags & INSECT_FLAG_UNCATCHABLE, 0);
    assert_eq!(b.insect_flags & INSECT_FLAG_CRAWLING, 0);
    assert_eq!(sfx_frames(&w, NA_SE_EN_MUSI_SINK, Some(SfxPos::Actor(h))).len(), 1);
    let home = b.actor.home_pos;
    // Its first update was in the air (no ground yet): shape.rot.x down 2000 then.
    let x0 = b.actor.shape_rot.x;
    assert_eq!(x0, -2000);
    // EnInsect_Dig: tipping down 352 a frame to 10922, sinking 0.8 a frame, shrinking 0.0002 a
    // frame to 0.001 (from 0.0038), jittering within 0.5 of where it began (then moved by what's
    // left of its speed).
    for n in 1..=59 {
        idle(&mut w, 1);
        let b = bug(&w, h);
        assert_eq!(b.action_timer, 60 - n);
        assert_eq!(b.actor.shape_rot.x, (x0 as i32 + 352 * n as i32).min(10922) as i16);
        assert!((b.actor.shape_y_offset + 0.8 * n as f32).abs() < 1e-3);
        let slack = 0.5 + b.actor.speed_xz.abs() * 1.5 + 1e-3;
        assert!((b.actor.world_pos.x - home.x).abs() <= slack && (b.actor.world_pos.z - home.z).abs() <= slack, "{} {}", b.actor.world_pos, home);
    }
    assert!((bug(&w, h).actor.scale.x - 0.001).abs() < 1e-7);
    assert_eq!(w.actors.downcast::<ObjMakekinsuta>(soil).unwrap().unk_152, 0);
    // The 60th: within 8 of the soil, soilActor->unk_152 = 1; killed (deleted, and
    // EnInsect_Destroy's sDroppedCount down, at the next update pass).
    idle(&mut w, 1);
    assert!(w.actors.actor(h).is_none_or(|a| a.killed));
    assert_eq!(w.actors.downcast::<ObjMakekinsuta>(soil).unwrap().unk_152, 1);
    idle(&mut w, 1);
    assert!(!w.actors.exists(h));
    assert_eq!(overlay_statics(&w).dropped_count, 0);
    // Obj_Makekinsuta: 60 frames on, its Gold Skulltula.
    run_until(&mut w, 70, |w| !all::<EnSw>(w).is_empty());
}

#[test]
fn the_first_dropped_bug_chimes_by_a_soil_unless_its_token_is_taken() {
    let Some(a) = assets() else { return };
    for taken in [false, true] {
        let mut w = graveyard(&a, |s| {
            if taken {
                // GET_GS_FLAGS(0x11 - 1) & 1.
                s.set_gs_flags(0x10, 1);
            }
        });
        // 40 from the soil: it can't come within 3 in 16 frames.
        let h = spawn(&mut w, SOIL_POS + Vec3::new(40.0, 0.0, 0.0), [0x4000, 0, 0], INSECT_TYPE_FIRST_DROPPED);
        assert_ne!(bug(&w, h).insect_flags & INSECT_FLAG_FOUND_SOIL, 0);
        idle(&mut w, 15);
        assert_eq!(bug(&w, h).unk_32a, 15);
        assert!(sfx_frames(&w, NA_SE_SY_TRE_BOX_APPEAR, None).is_empty());
        idle(&mut w, 1);
        assert_ne!(bug(&w, h).insect_flags & INSECT_FLAG_7, 0);
        assert_eq!(sfx_frames(&w, NA_SE_SY_TRE_BOX_APPEAR, None).len(), if taken { 0 } else { 1 });
        idle(&mut w, 30);
        assert_eq!(sfx_frames(&w, NA_SE_SY_TRE_BOX_APPEAR, None).len(), if taken { 0 } else { 1 }, "once");
    }
}

#[test]
fn dropped_bugs_land_turn_upright_and_crawl_on() {
    let Some(a) = assets() else { return };
    let mut w = graveyard(&a, |_| {});
    // Far from the soil (no soil within 80), 20 above the ground near Link's spawn.
    let at = Vec3::new(-1300.0, 20.0, 330.0);
    let h = spawn(&mut w, at, [0x4000, 0, 0], INSECT_TYPE_FIRST_DROPPED);
    let bugs = all::<EnInsect>(&w);
    assert_eq!(bugs.len(), 3);
    // Falling (gravity -0.2, at most -2), each lands once (NA_SE_EN_MUSI_LAND,
    // INSECT_FLAG_DROPPED_HAS_LANDED), grows to 0.01 (0.0008 a frame from 0.003: 9 frames), turns
    // upright (world.rot.x 0x4000 to 0 by 2000) and becomes a crawling bug (EnInsect_SetupSlowDown).
    run_until(&mut w, 200, |w| bugs.iter().all(|&x| bug(w, x).action != Action::Dropped));
    for &x in &bugs {
        let b = bug(&w, x);
        assert_ne!(b.insect_flags & INSECT_FLAG_DROPPED_HAS_LANDED, 0);
        assert_eq!(sfx_frames(&w, NA_SE_EN_MUSI_LAND, Some(SfxPos::Actor(x))).len(), 1);
        assert_eq!((b.actor.world_rot.x, b.actor.shape_rot.x), (0, 0));
        assert_eq!(b.actor.scale, Vec3::splat(0.01));
        assert!(matches!(b.action, Action::SlowDown | Action::Crawl | Action::RunFromPlayer), "{:?}", b.action);
        assert_ne!(b.actor.bg_check_flags & BGCHECKFLAG_GROUND, 0);
    }
    // Short-lived: when its life is out it digs in (60 frames) and is gone; sDroppedCount down
    // to 0, and the overlay's statics with the last bug.
    let _ = h;
    run_until(&mut w, 400, |w| all::<EnInsect>(w).is_empty());
    assert_eq!(sfx_frames(&w, NA_SE_EN_MUSI_SINK, None).len(), 3);
    idle(&mut w, 1);
    assert!(w.overlay_statics.get(&ACTOR_EN_INSECT).is_none(), "the overlay unloaded");
}

#[test]
fn four_dropped_bugs_on_the_ground_dig_in() {
    let Some(a) = assets() else { return };
    let mut w = graveyard(&a, |_| {});
    // Two bottles' worth: sDroppedCount 6. Upright on the ground (EnInsect_SlowDown and the
    // rest), a dropped bug digs in at once while four or more are about.
    spawn(&mut w, Vec3::new(-1300.0, 20.0, 330.0), [0x4000, 0, 0], INSECT_TYPE_FIRST_DROPPED);
    spawn(&mut w, Vec3::new(-1300.0, 20.0, 250.0), [0x4000, 0, 0], INSECT_TYPE_FIRST_DROPPED);
    assert_eq!(overlay_statics(&w).dropped_count, 6);
    let bugs = all::<EnInsect>(&w);
    assert_eq!(bugs.len(), 6);
    // Each, out of EnInsect_Dropped (upright: EnInsect_SetupSlowDown), digs in at its next
    // update (EnInsect_SlowDown's test: on the ground, sDroppedCount at least 4), never crawling.
    let mut dug = [false; 6];
    for _ in 0..200 {
        idle(&mut w, 1);
        for (i, &x) in bugs.iter().enumerate() {
            let Some(b) = w.actors.downcast::<EnInsect>(x) else { continue };
            assert!(!matches!(b.action, Action::Crawl | Action::RunFromPlayer | Action::WalkOnWater), "{:?}", b.action);
            dug[i] |= b.action == Action::Dig;
        }
        if dug.iter().all(|&d| d) {
            break;
        }
    }
    assert!(dug.iter().all(|&d| d), "{dug:?}");
}

#[test]
fn a_spawned_bug_walks_on_the_water_then_drowns() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_ZORAS_RIVER_0", 10, |_| {});
    let room = w.col.water_room;
    let surface = w.col.water_surface(RIVER.x, RIVER.z, room).expect("the river's water");
    // A bug from under a rock, 10 above the water: it falls (gravity -0.2), touches the water
    // (BGCHECKFLAG_WATER_TOUCH), and its next spell's test hands it to EnInsect_WalkOnWater.
    let h = spawn(&mut w, Vec3::new(RIVER.x, surface + 10.0, RIVER.z), [0, 0, 0], INSECT_TYPE_SPAWNED);
    run_until(&mut w, 60, |w| bug(w, h).action == Action::WalkOnWater);
    let b = bug(&w, h);
    assert!((120..170).contains(&b.action_timer), "Rand_S16Offset(120, 50)");
    assert_eq!(b.insect_flags & INSECT_FLAG_CRAWLING, 0);
    idle(&mut w, 2);
    // On the surface: its depth added back each update, then Actor_MoveXZGravity's gravity (-0.2,
    // velocity.y zeroed first; × 1.5, the update rate) leaves it 0.3 under.
    let b = bug(&w, h);
    assert!((b.actor.world_pos.y - (surface - 0.3)).abs() < 1e-3, "{} vs {}", b.actor.world_pos.y, surface);
    assert_ne!(b.actor.bg_check_flags & BGCHECKFLAG_WATER_TOUCH, 0);
    // Its time out: EnInsect_SetupDrown: 100 frames, sinking at -0.04 to -0.8, no more floor or
    // wall checks (INSECT_FLAG_0 and _1 off), uncatchable.
    bug_mut(&mut w, h).action_timer = 1;
    idle(&mut w, 1);
    let b = bug(&w, h);
    assert_eq!((b.action, b.action_timer), (Action::Drown, 100));
    assert_eq!((b.actor.gravity, b.actor.min_velocity_y), (-0.04, -0.8));
    assert_eq!(b.insect_flags & (INSECT_FLAG_0 | INSECT_FLAG_1), 0);
    assert_ne!(b.insect_flags & INSECT_FLAG_UNCATCHABLE, 0);
    let (rx, ry, s, y) = (b.actor.shape_rot.x, b.actor.shape_rot.y, b.actor.scale.x, b.actor.world_pos.y);
    idle(&mut w, 1);
    // EnInsect_Drown: shape.rot.x -500, .y +200, scale -0.00005; sinking.
    let b = bug(&w, h);
    assert_eq!((b.actor.shape_rot.x, b.actor.shape_rot.y), (rx.wrapping_sub(500), ry.wrapping_add(200)));
    assert!((b.actor.scale.x - (s - 0.00005)).abs() < 1e-7);
    assert!(b.actor.world_pos.y < y);
    // Gone when its 100 frames are out.
    idle(&mut w, 98);
    assert!(w.actors.actor(h).is_some_and(|a| !a.killed));
    idle(&mut w, 1);
    assert!(w.actors.actor(h).is_none_or(|a| a.killed));
}

#[test]
fn a_bug_offers_itself_to_the_bottle_and_a_set_parent_catches_it() {
    let Some(a) = assets() else { return };
    let mut w = graveyard(&a, |_| {});
    idle(&mut w, 30);
    let link = w.player().actor.world_pos;
    let yaw = w.player().actor.shape_rot.y;
    // A permanent bug 20 in front of Link: within 32 (EnInsect_InBottleRange), catchable, and
    // within Actor_OfferGetItem's 60 across and 30 up or down.
    let at = link + Vec3::new(eng_math::sin_s(yaw) * 20.0, 0.0, eng_math::cos_s(yaw) * 20.0);
    let h = spawn(&mut w, at, [0, 0, 0], INSECT_TYPE_PERMANENT);
    idle(&mut w, 1);
    {
        let b = bug_mut(&mut w, h);
        b.actor.world_pos = at;
        b.action = Action::SlowDown;
        b.action_timer = 40;
    }
    idle(&mut w, 1);
    // Player's interactRangeActor is the bug, offering GI_MAX (only a bottle's swing takes it).
    assert_eq!(w.player().interact_range_actor, Some(h));
    assert_eq!(w.player().get_item_id, GI_MAX);
    // sCaughtCount: up by the offer, zeroed by any drawn bug's draw.
    if w.actors.actor(h).unwrap().is_drawn {
        assert_eq!(overlay_statics(&w).caught_count, 0);
    }

    // Player_Action_SwingBottle's catch: interactRangeActor->parent = Player. The bug's next
    // update (Actor_HasParent) clears it and is caught: undrawn, 0.001, still, 200 frames.
    let ph = w.player;
    w.actors.actor_mut(h).unwrap().parent = ph;
    idle(&mut w, 1);
    let b = bug(&w, h);
    assert_eq!((b.action, b.action_timer, b.actor.parent, b.drawn, b.actor.speed_xz), (Action::Caught, 200, None, false, 0.0));
    assert_eq!(b.actor.scale, Vec3::splat(0.001));
    assert_eq!(b.skel.play_speed, 0.3);
    assert_eq!(b.insect_flags & INSECT_FLAG_CRAWLING, 0);
    // A permanent one comes back: drawn again at 20, growing 0.001 a frame from 19 (to 0.01),
    // and at 0 a crawling bug again.
    idle(&mut w, 179);
    let b = bug(&w, h);
    assert_eq!((b.action_timer, b.drawn), (21, false));
    idle(&mut w, 1);
    assert_eq!((bug(&w, h).action_timer, bug(&w, h).drawn, bug(&w, h).actor.scale.x), (20, true, 0.001));
    idle(&mut w, 1);
    assert!((bug(&w, h).actor.scale.x - 0.002).abs() < 1e-7);
    idle(&mut w, 19);
    let b = bug(&w, h);
    assert_eq!((b.action, b.actor.scale), (Action::SlowDown, Vec3::splat(0.01)));

    // A short-lived bug caught is gone at the end of its 200 frames, never drawn again.
    let h = spawn(&mut w, at, [0, 0, 0], INSECT_TYPE_SPAWNED);
    w.actors.actor_mut(h).unwrap().parent = ph;
    idle(&mut w, 1);
    assert_eq!(bug(&w, h).action, Action::Caught);
    idle(&mut w, 199);
    assert!(w.actors.actor(h).is_some_and(|a| !a.killed) && !bug(&w, h).drawn);
    idle(&mut w, 1);
    assert!(w.actors.actor(h).is_none_or(|a| a.killed));

    // A dropped one with a parent is simply gone.
    let h = spawn(&mut w, at + Vec3::Y * 10.0, [0x4000, 0, 0], INSECT_TYPE_EXTRA_DROPPED);
    w.actors.actor_mut(h).unwrap().parent = ph;
    idle(&mut w, 1);
    assert!(w.actors.actor(h).is_none_or(|a| a.killed));
}

#[test]
fn an_uncatchable_bug_makes_no_offer() {
    let Some(a) = assets() else { return };
    let mut w = graveyard(&a, |_| {});
    idle(&mut w, 30);
    let link = w.player().actor.world_pos;
    let yaw = w.player().actor.shape_rot.y;
    let at = link + Vec3::new(eng_math::sin_s(yaw) * 20.0, 0.0, eng_math::cos_s(yaw) * 20.0);
    let h = spawn(&mut w, at, [0, 0, 0], INSECT_TYPE_PERMANENT);
    idle(&mut w, 1);
    {
        let b = bug_mut(&mut w, h);
        b.actor.world_pos = at;
        b.action = Action::SlowDown;
        b.action_timer = 40;
        b.insect_flags |= INSECT_FLAG_UNCATCHABLE;
    }
    idle(&mut w, 1);
    assert_ne!(w.player().interact_range_actor, Some(h));
}

#[test]
fn a_grotto_s_bug_groups_spawn_permanent_bugs() {
    let Some(a) = assets() else { return };
    // ENTR_GROTTOS_9: spawn 9, room 8, with its two Obj_Mure 0x5323 (type 3, bugs; svNum 1;
    // ptn 3; chNum 5), each spawning its five while the camera is within 1000 of it.
    let mut w = enter(&a, "ENTR_GROTTOS_9", 10, |_| {});
    let mures = all::<ObjMure>(&w);
    assert_eq!(mures.len(), 2);
    run_until(&mut w, 30, |w| !all::<EnInsect>(w).is_empty());
    idle(&mut w, 1);
    let mut groups = 0;
    for &m in &mures {
        let mure = w.actors.downcast::<ObjMure>(m).unwrap();
        if mure.action != oot_actors::obj_mure::Action::ActiveState {
            continue;
        }
        groups += 1;
        let (pos, room) = (mure.actor.world_pos, mure.actor.room);
        for c in mure.children.iter().take(5) {
            let h = c.expect("a child");
            let b = bug(&w, h);
            // sSpawnParams[OBJMURE_TYPE_BUGS]: INSECT_TYPE_PERMANENT, at the group's position.
            assert_eq!((b.actor.params, b.actor.room, b.actor.home_pos), (INSECT_TYPE_PERMANENT, room, pos));
            assert_eq!(b.insect_flags & !INSECT_FLAG_CRAWLING, 0);
            assert_eq!((b.actor.gravity, b.life_timer), (0.0, 0));
            // No gravity and no floor check: at the group's height.
            assert_eq!(b.actor.world_pos.y, pos.y);
        }
    }
    assert!(groups >= 1);
    assert_eq!(all::<EnInsect>(&w).len(), 5 * groups);
    idle(&mut w, 60);
    for &m in &mures {
        let mure = w.actors.downcast::<ObjMure>(m).unwrap();
        for c in mure.children.iter().flatten() {
            if let Some(b) = w.actors.downcast::<EnInsect>(*c) {
                assert_eq!(b.actor.world_pos.y, mure.actor.world_pos.y);
            }
        }
    }
}

