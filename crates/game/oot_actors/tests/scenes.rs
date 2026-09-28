//! Scenes from the asset pack through `Play_Init` (`oot_game::play_scene`): Kokiri Forest's
//! placements (each a ported actor or a placeholder), its rooms and `En_Holl` room changes, and
//! a scripted walk into Link's house and back out through the exits.
//!
//! The walks steer the stick towards a point each frame, as a player would. Everything they
//! check comes from the decomp: the exit lists and entrance table (`ENTR_LINK_HOME_1` from
//! Kokiri Forest's exit 4, `ENTR_SPOT04_3` from the house's exit 2), the spawns the entrances
//! name, the start modes in their params (`0x0EFF`: walk in 120 units, `0x0DFF`: stand), and
//! the transitions' timing (`TRANS_TYPE_FADE_BLACK_FAST`: `transFadeDuration` 20, in steps of
//! `R_UPDATE_RATE` 3).

mod common;

use std::sync::Arc;

use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_holl::EnHoll;
use oot_actors::script::{exit_to, stick_towards};
use oot_game::actor_ctx::ACTOR_PLAYER;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::spawn::{Placeholder, Uninit};
use oot_game::transition::{TRANS_MODE_OFF, TRANS_TRIGGER_OFF};

const SCENE_SPOT04: u16 = 0x55;
const SCENE_LINK_HOME: u16 = 0x34;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Play entering by `entrance` (an `ENTR_*` name), child Link, 10:00.
fn enter(entrance: &str) -> Option<PlayState> {
    let a = assets()?;
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    Some(oot_actors::play_entrance(a, common::data()?, common::rules()?, save).expect("Play_Init"))
}

/// One frame with `pad` (held, no edges).
fn frame(w: &mut PlayState, prev: &mut PadState, pad: PadState) {
    w.tick_with(scripted_input(*prev, pad));
    *prev = pad;
}

#[test]
fn kokiri_forest_spawns_every_placement_or_a_placeholder() {
    let Some(mut w) = enter("ENTR_SPOT04_0") else { return };
    assert_eq!(w.scene_id, SCENE_SPOT04);
    assert_eq!(w.room_ctx.cur.num, 0);
    assert!(w.room_ctx.cur.loaded);
    let room0 = w.scene.as_ref().unwrap().rooms[0].clone();
    // Play_Init: Player, Navi (a placeholder) and the transition actors touching room 0 (both
    // En_Holl planes) exist; the room's actor list waits for the first Actor_UpdateAll.
    assert_eq!(w.setup_actors.len(), room0.actors.len());
    let holls = w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnHoll>(h).is_some()).count();
    assert_eq!(holls, 2, "both En_Holl touch room 0");
    let mut prev = PadState::default();
    frame(&mut w, &mut prev, PadState::default());
    assert!(w.setup_actors.is_empty());
    // Everything placed spawned: room 0's objects are swapped in on load, so the actors whose
    // object is one of them wait (`Uninit`) until Object_UpdateBank finishes it a frame later.
    let count = |w: &PlayState| {
        let mut n = (0, 0, 0);
        for h in w.actors.all() {
            let a = w.actors.get(h).unwrap();
            if a.as_any().is::<Uninit>() {
                n.0 += 1;
            } else if a.as_any().is::<Placeholder>() {
                n.1 += 1;
            } else {
                n.2 += 1;
            }
        }
        n
    };
    let (waiting, placeholders, ported) = count(&w);
    // Player + 2 En_Holl ported; Navi and every placement a placeholder (or still waiting).
    assert_eq!(ported, 3);
    assert_eq!(waiting + placeholders, room0.actors.len() + 1);
    assert!(waiting > 0, "actors whose object loads with the room wait for it");
    for _ in 0..2 {
        frame(&mut w, &mut prev, PadState::default());
    }
    let (waiting, placeholders, _) = count(&w);
    assert_eq!(waiting, 0, "every object is loaded two frames after the room");
    assert_eq!(placeholders, room0.actors.len() + 1);
    // Each placement is where the room's actor list puts it, in its category and room.
    let at = &w.assets.as_ref().unwrap().actors;
    for e in &room0.actors {
        let pos = Vec3::new(e.pos[0] as f32, e.pos[1] as f32, e.pos[2] as f32);
        let found = w.actors.all().into_iter().filter_map(|h| w.actors.actor(h)).any(|a| {
            a.id == e.id && a.params == e.params && a.home_pos == pos && a.room == 0 && a.category == at.get(e.id).unwrap().init.as_ref().unwrap().category as usize
        });
        assert!(found, "{} at {:?}", at.name(e.id), e.pos);
    }
    assert_eq!(w.player().actor.room, -1, "Player_Init: thisx->room = -1");
}

#[test]
fn en_holl_changes_rooms_and_deletes_the_old_rooms_actors() {
    let Some(mut w) = enter("ENTR_SPOT04_0") else { return };
    let mut prev = PadState::default();
    // Spawn 0's entrance walk (start mode 0xF) first.
    while format!("{:?}", w.player().action) == "ExitWalk" {
        frame(&mut w, &mut prev, PadState::default());
    }
    let room0_actors = w.actors.all().into_iter().filter(|&h| w.actors.actor(h).unwrap().room == 0).count();
    assert!(room0_actors > 50);
    // Transition 1 is the plane on the path to the Deku Tree's meadow, between room 1 (front,
    // local z < 0) and room 0 (back), at (2160, -1, -148) facing +z. (Transition 0 is in the
    // crawlspace to the Kokiri Sword's maze, which needs crawling.) Walk Link through it from
    // the village.
    let t = w.transi_actors[1];
    assert_eq!((t.sides[0].0, t.sides[1].0), (1, 0));
    let plane = Vec3::new(t.pos[0] as f32, t.pos[1] as f32, t.pos[2] as f32);
    {
        let p = w.player_mut();
        p.actor.world_pos = plane + Vec3::new(0.0, 1.0, 150.0);
        p.actor.prev_pos = p.actor.world_pos;
        p.actor.home_pos = p.actor.world_pos;
        p.actor.shape_rot.y = -0x8000;
        p.current_yaw = -0x8000;
    }
    let target = plane + Vec3::new(0.0, 0.0, -300.0);
    let mut loaded_at = None;
    let mut done_at = None;
    for i in 0..120 {
        let s = stick_towards(&w, target, 60.0);
        frame(&mut w, &mut prev, s);
        if std::env::var("DBG").is_ok() {
            let p = w.player();
            println!("{i} {:?} {:?} room {:?}/{:?} st {} holl {:?}", p.actor.world_pos, p.action, w.room_ctx.cur, w.room_ctx.prev.num, w.room_ctx.status, s);
        }
        if loaded_at.is_none() && w.room_ctx.status == 1 {
            // func_8009728C: the load is requested from En_Holl's update...
            assert_eq!((w.room_ctx.cur.num, w.room_ctx.prev.num), (1, 0));
        }
        if loaded_at.is_none() && w.room_ctx.cur.num == 1 && w.room_ctx.cur.loaded {
            // ...and done by the next frame's func_800973FC; En_Holl lets room 0 go once the
            // load is in (EnHoll_NextAction), the same frame.
            loaded_at = Some(i);
        }
        if loaded_at.is_some() && w.room_ctx.prev.num < 0 {
            done_at = Some(i);
            break;
        }
    }
    assert!(loaded_at.is_some(), "room 1 loaded");
    assert!(done_at.is_some(), "the old room let go");
    assert_eq!(w.room_ctx.cur.num, 1);
    // func_80031B14: room 0's actors are gone, room 1's are spawning.
    assert_eq!(w.actors.all().into_iter().filter(|&h| w.actors.actor(h).unwrap().room == 0).count(), 0);
    for _ in 0..3 {
        frame(&mut w, &mut prev, PadState::default());
    }
    let room1 = w.scene.as_ref().unwrap().rooms[1].clone();
    assert_eq!(w.actors.all().into_iter().filter(|&h| w.actors.actor(h).unwrap().room == 1).count(), room1.actors.len() + 1);
    // The En_Holl Link went through stays (it belongs to room 1 now); the one between rooms 0
    // and 2 was deleted with room 0, and its entry can spawn again.
    let holls: Vec<_> = w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<EnHoll>(h)).map(|h| h.actor.params >> 10).collect();
    assert_eq!(holls, vec![1]);
    assert!(w.transi_actors[0].id > 0);
}

/// Walks towards `to` until the transition starts; returns the frames it took.
fn walk_into_exit(w: &mut PlayState, prev: &mut PadState, to: Vec3, max: usize) -> usize {
    for i in 0..max {
        let s = stick_towards(w, to, 60.0);
        frame(w, prev, s);
        if w.transition.trigger != TRANS_TRIGGER_OFF {
            return i + 1;
        }
    }
    panic!("no transition after {max} frames, Link at {:?}", w.player().actor.world_pos);
}

/// Runs frames until the scene changes; returns the frames it took.
fn until_scene_change(w: &mut PlayState, prev: &mut PadState, max: usize) -> usize {
    let n0 = w.scene_changes;
    for i in 0..max {
        frame(w, prev, PadState::default());
        if w.scene_changes != n0 {
            return i + 1;
        }
    }
    panic!("no scene change after {max} frames");
}

#[test]
fn link_walks_into_his_house_and_back_out() {
    let Some(a) = assets() else { return };
    let Some(mut w) = enter("ENTR_SPOT04_3") else { return };
    let mut prev = PadState::default();
    // Arriving on the porch (spawn 3, start mode 0xD: standing), the fade in runs.
    assert_eq!(w.scene_id, SCENE_SPOT04);
    let spawn3 = Vec3::new(-31.0, 100.0, 1073.0);
    assert_eq!(w.player().actor.world_pos, spawn3);
    assert!(w.transition.trigger != TRANS_TRIGGER_OFF);
    // FADE_BLACK_FAST: set up on the first frame, then 7 updates of 3 (21 >= 20), then done.
    let mut fade_frames = 0;
    while w.transition.mode != TRANS_MODE_OFF || w.transition.trigger != TRANS_TRIGGER_OFF {
        frame(&mut w, &mut prev, PadState::default());
        fade_frames += 1;
        assert!(fade_frames < 30);
    }
    assert_eq!(fade_frames, 9);
    // The door is behind Link: Kokiri Forest's exit to ENTR_LINK_HOME_1.
    let (exit, door) = exit_to(&w, "ENTR_LINK_HOME_1").unwrap();
    assert_eq!(exit, 4);
    let n = walk_into_exit(&mut w, &mut prev, door, 200);
    assert!(n < 100, "walked to the door in {n} frames");
    assert_eq!(w.transition.next_entrance_index, a.scenes.entrance_index("ENTR_LINK_HOME_1").unwrap());
    assert_eq!(format!("{:?}", w.player().action), "ExitWalk");
    // The fade out, then Play_Init for the house.
    let n = until_scene_change(&mut w, &mut prev, 30);
    assert_eq!(n, 9);
    assert_eq!(w.scene_id, SCENE_LINK_HOME);
    assert_eq!(w.player().actor.id, ACTOR_PLAYER);
    // Spawn 1 (-4, 0, -114), params 0x0EFF: func_8083CA54 walks in at speed 2 for 15 frames.
    let spawn1 = Vec3::new(-4.0, 0.0, -114.0);
    assert_eq!(w.player().actor.world_pos, spawn1);
    assert_eq!(w.player().linear_velocity, 2.0);
    for _ in 0..20 {
        frame(&mut w, &mut prev, PadState::default());
    }
    let inside = w.player().actor.world_pos;
    assert!(inside.z > spawn1.z + 20.0, "walked in: {inside:?}");
    assert_ne!(format!("{:?}", w.player().action), "ExitWalk");
    // And back out: the house's exit 2 leads to ENTR_SPOT04_3.
    // (Exit 1 leads there too, but no floor uses it.)
    let (exit, door) = exit_to(&w, "ENTR_SPOT04_3").unwrap();
    assert_eq!(exit, 2);
    walk_into_exit(&mut w, &mut prev, door, 200);
    assert_eq!(w.transition.next_entrance_index, a.scenes.entrance_index("ENTR_SPOT04_3").unwrap());
    until_scene_change(&mut w, &mut prev, 30);
    assert_eq!(w.scene_id, SCENE_SPOT04);
    assert_eq!(w.player().actor.world_pos, spawn3);
    assert_eq!(w.scene_changes, 2);
    // The void-out point is the entrance Link came in by.
    assert_eq!(w.save.respawn[0].entrance_index, a.scenes.entrance_index("ENTR_SPOT04_3").unwrap());
}

/// The ported actors' profiles are their `ActorInit`s (from the pack's actor table, read from
/// the overlays' C by the importer).
#[test]
fn ported_profiles_match_the_actor_table() {
    let Some(a) = assets() else { return };
    for p in oot_actors::PROFILES.iter().filter(|p| p.id >= 0) {
        let info = a.actors.get(p.id).unwrap();
        let init = info.init.as_ref().unwrap();
        assert_eq!(info.name, p.name);
        assert_eq!((init.id, init.category as usize, init.flags), (p.id, p.category, p.flags), "{}", p.name);
        assert_eq!(a.scenes.objects[init.object_id as usize], p.object, "{}", p.name);
        assert!(a.overlays.is_ported(p.id) || p.id == oot_game::actor_ctx::ACTOR_BG_YDAN_HASI, "{} has a constructor", p.name);
    }
}

/// Every scene enters by `Play_Init` (its first entrance, both ages) and plays 30 frames:
/// the scene and room loads, the object banks, the spawns and Player's start modes hold up
/// everywhere, not just in Kokiri Forest.
#[test]
fn every_scene_enters_and_plays() {
    let Some(a) = assets() else { return };
    let (Some(data), Some(rules)) = (common::data(), common::rules()) else { return };
    let mut entered = 0;
    let mut failed = Vec::new();
    for s in &a.scenes.scenes {
        let Some(e) = a.scenes.entrances.iter().position(|e| e.scene == s.id) else { continue };
        if a.pack.scene(&s.file).is_err() {
            continue;
        }
        for adult in [false, true] {
            let save = SaveContext::new(e as u16, adult, oot_game::env::clock_time(10, 0) as u16);
            match oot_actors::play_entrance(a.clone(), data.clone(), rules.clone(), save) {
                Ok(mut w) => {
                    let mut prev = PadState::default();
                    for _ in 0..30 {
                        frame(&mut w, &mut prev, PadState::default());
                    }
                    assert!(w.player.is_some(), "{}: Player is there", s.file);
                    entered += 1;
                }
                Err(err) => failed.push(format!("{} ({}): {err:#}", s.file, if adult { "adult" } else { "child" })),
            }
        }
    }
    println!("{entered} entered; failed: {failed:#?}");
    assert!(entered >= 200, "{entered} entered");
    assert!(failed.is_empty(), "{failed:#?}");
}

/// `Play_TriggerVoidOut` goes back to the respawn point `Player_Init` set (`func_80845C68`:
/// where Link came in, with params `0xDFF`), through `Play_Init` with `respawnFlag` 1.
#[test]
fn a_void_out_returns_to_where_link_came_in() {
    let Some(mut w) = enter("ENTR_SPOT04_3") else { return };
    let mut prev = PadState::default();
    let start = w.player().actor.world_pos;
    assert_eq!(w.save.respawn[0].pos, start);
    assert_eq!(w.save.respawn[0].player_params, 0xDFF);
    // Somewhere else in the village, then void out.
    let elsewhere = Vec3::new(-68.0, -80.0, 941.0);
    {
        let a = &mut w.player_mut().actor;
        (a.world_pos, a.prev_pos, a.home_pos) = (elsewhere, elsewhere, elsewhere);
    }
    for _ in 0..30 {
        frame(&mut w, &mut prev, PadState::default());
    }
    assert!(w.player().actor.world_pos.distance(start) > 100.0);
    w.trigger_void_out();
    until_scene_change(&mut w, &mut prev, 60);
    assert_eq!(w.scene_id, SCENE_SPOT04);
    assert_eq!(w.player().actor.world_pos, start);
    assert_eq!(w.save.respawn_flag, 0, "Play_Init clears it");
}
