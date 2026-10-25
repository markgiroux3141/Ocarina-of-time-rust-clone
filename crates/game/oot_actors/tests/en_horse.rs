//! `En_Horse` and `Bg_Umajump` (GAME-12 milestone 1) against the C (`z_en_horse.c`, `z_horse.c`,
//! `z_bg_umajump.c`): Lon Lon Ranch as an adult, before Epona and after; Epona fleeing until
//! Epona's Song; the rental horse's day; the obstacle course's fence. (Riding: `riding_runs.rs`.)
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::en_horse::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn horses(w: &PlayState) -> Vec<&EnHorse> {
    w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<EnHorse>(h)).filter(|h| !h.actor.killed).collect()
}

fn epona(w: &PlayState) -> Option<&EnHorse> {
    horses(w).into_iter().find(|h| h.ty == HORSE_EPONA)
}

/// Lon Lon Ranch as an adult with `preset`, entered at its gateway (`ENTR_LON_LON_RANCH_0`), at
/// `hour`:00, a few frames on.
fn ranch(a: &Arc<GameAssets>, preset: &str, hour: i32) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_LON_LON_RANCH_0").unwrap();
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(hour, 0) as u16);
    save.apply_preset(preset).unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap();
    for _ in 0..3 {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
    w
}

#[test]
fn epona_flees_until_her_song_then_comes() {
    let Some(a) = assets() else { return };
    let mut w = ranch(&a, "adult-awake", 10);
    // Horse_SetupInGameplay without Epona had: she's spawned at (0, 0, -500) (the ranch's
    // sHorseSpawns entry); EnHorse_Init: the ranch before Epona, by day, EnHorse_InitFleePlayer
    // (ENHORSE_UNRIDEABLE).
    let e = epona(&w).expect("Epona by day");
    assert_eq!(e.action, ENHORSE_ACT_FLEE_PLAYER);
    assert_ne!(e.state_flags & ENHORSE_UNRIDEABLE, 0);
    // Epona's Song (R_EPONAS_SONG_PLAYED): EnHorse_FleePlayer starts EnHorse_StartIdleRidable
    // (rideable), then EnHorse_Idle takes the song (clearing it) and follows Link.
    w.msg_ctx.r_eponas_song_played = true;
    let mut actions = vec![ENHORSE_ACT_FLEE_PLAYER];
    for _ in 0..30 {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
        let e = epona(&w).unwrap();
        if actions.last() != Some(&e.action) {
            actions.push(e.action);
        }
    }
    assert_eq!(&actions[..3], &[ENHORSE_ACT_FLEE_PLAYER, ENHORSE_ACT_IDLE, ENHORSE_ACT_FOLLOW_PLAYER]);
    assert!(!w.msg_ctx.r_eponas_song_played);
    assert_eq!(epona(&w).unwrap().state_flags & ENHORSE_UNRIDEABLE, 0);
}

#[test]
fn the_rental_horse_and_epona_by_day_and_night() {
    let Some(a) = assets() else { return };
    // The ranch's placed En_Horse (params 0xFFFF: type 1, HORSE_PTYPE_1; rot.z 5) at (-297, 0,
    // -1288): by day before Epona it stays (EnHorse_Init's ranch checks), rideable from the start
    // (EnHorse_FleePlayer: a rental horse goes to EnHorse_StartIdleRidable).
    let w = ranch(&a, "adult-awake", 10);
    let hni: Vec<_> = horses(&w).into_iter().filter(|h| h.ty == HORSE_HNI).collect();
    assert_eq!(hni.len(), 1);
    assert_eq!(hni[0].actor.world_pos, Vec3::new(-297.0, 0.0, -1288.0));
    assert_eq!(hni[0].action, ENHORSE_ACT_IDLE);
    // At night: the rental horse goes (`!IS_DAY`), and so does Epona before she's had.
    let w = ranch(&a, "adult-awake", 22);
    assert!(horses(&w).is_empty(), "{:?}", horses(&w).iter().map(|h| h.ty).collect::<Vec<_>>());
    // With Epona had (EVENTCHKINF_EPONA_OBTAINED): no rental horse; Epona inactive (called by her
    // song), by day or night.
    for hour in [10, 22] {
        let w = ranch(&a, "epona", hour);
        let hs = horses(&w);
        assert_eq!(hs.len(), 1, "{hour}:00");
        assert_eq!((hs[0].ty, hs[0].action), (HORSE_EPONA, ENHORSE_ACT_INACTIVE), "{hour}:00");
    }
}

#[test]
fn the_obstacle_courses_fence_needs_epona() {
    let Some(a) = assets() else { return };
    use oot_actors::bg_umajump::{ACTOR_BG_UMAJUMP, BgUmajump};
    // BgUmaJump_Init: params 1 is killed without EVENTCHKINF_EPONA_OBTAINED; with it, never
    // culled (ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED). Any other
    // params is always there (the corral's two are 0).
    for (preset, there) in [("adult-awake", false), ("epona", true)] {
        let mut w = ranch(&a, preset, 10);
        let corral = w.actors.all().into_iter().filter(|&h| w.actors.downcast::<BgUmajump>(h).is_some()).count();
        assert_eq!(corral, 2, "{preset}");
        let h = w.actor_spawn(ACTOR_BG_UMAJUMP, Vec3::new(0.0, 0.0, -500.0), [0; 3], 1).unwrap();
        let fence = w.actors.actor(h).unwrap();
        assert_eq!(!fence.killed, there, "{preset}");
        if there {
            assert_eq!(fence.flags & 0x30, 0x30);
        }
    }
}
