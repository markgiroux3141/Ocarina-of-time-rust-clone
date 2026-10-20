//! GAME-09 milestone 5's runs in Dodongo's Cavern (Master Quest), each a short debug start of one
//! of the rest of the cavern's moments, no fights (the Dodongos, the Beamos, the larvae, the Poe,
//! the Skulltulas: the hand test's): `Route::FallingStairs` (room 2's Bomb Flowers, the stairs,
//! one-point 3050), `Route::SilverRupees` (the last of room 2's five, door 17), `Route::ShieldChest`
//! (room 14's gravestone and chest), `Route::TimeBlocks` (room 15's Song of Time). Each run's end
//! that a preset holds is checked here against the run; and the six rooms' placements, all
//! ported.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Runs `route` from its start; the play state at its end, its run and its start's save. The
/// ocarina's route gets the audio side (`needs_audio`), as the sandbox gives it (tests/summit.rs's).
fn run(a: &Arc<GameAssets>, route: Route) -> (PlayState, Playthrough, SaveContext) {
    let e = a.scenes.entrance_index(route.entrance()).unwrap();
    let start = route.save(e);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), start.clone()).unwrap();
    w.audio.log = Some(Default::default());
    route.debug_start(&mut w);
    // The ocarina's routes: AudioOcarina_Update runs in Audio_Update, on the audio side.
    if route.needs_audio() {
        w.audio_side = Some(Box::new(oot_game::audio::offline::OfflineAudio::new(&a.pack.audio_data().unwrap(), false)));
    }
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
    }
    eprintln!("steps {:?}", run.steps);
    eprintln!("texts {:x?}", run.texts);
    eprintln!("at {:?} health {}", w.player().actor.world_pos, w.save.health);
    assert!(run.failure.is_none(), "{:?}", run.failure);
    (w, run, start)
}

/// The run's save is the preset's in its flags, inventory and equipment, with `extra` applied to
/// the run's (what the run doesn't do that the preset has).
fn ends_as_preset(w: &mut PlayState, preset: &str, extra: impl Fn(&mut SaveContext)) {
    // The scene's flags as leaving it saves them (Play_SaveSceneFlags).
    w.save_scene_flags();
    let mut s = w.save.clone();
    extra(&mut s);
    let mut p = SaveContext::new(s.entrance_index, false, s.day_time);
    p.apply_preset(preset).unwrap();
    assert_eq!((&p.event_chk_inf, &p.inf_table, &p.item_get_inf, &p.event_inf), (&s.event_chk_inf, &s.inf_table, &s.item_get_inf, &s.event_inf));
    assert_eq!((&p.inventory.items, &p.inventory.ammo, p.inventory.upgrades, p.inventory.quest_items), (&s.inventory.items, &s.inventory.ammo, s.inventory.upgrades, s.inventory.quest_items));
    assert_eq!((p.rupees, &p.equips.button_items), (s.rupees + s.rupee_accumulator, &s.equips.button_items));
    for (i, (a, b)) in p.scene_flags.iter().zip(s.scene_flags.iter()).enumerate() {
        assert_eq!(format!("{a:?}"), format!("{b:?}"), "scene {i:#x}'s flags");
    }
}

fn steps(run: &Playthrough) -> Vec<Step> {
    run.steps.iter().map(|s| s.0).collect()
}

fn sfx_count(w: &PlayState, id: u16) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).count()
}

#[test]
fn room_2_s_bomb_flowers_drop_the_stairs() {
    let Some(a) = assets() else { return };
    let (mut w, run, start) = run(&a, Route::FallingStairs);
    assert_eq!(steps(&run), vec![Step::BombOut, Step::PutDown, Step::StairsFallen]);
    // Bg_Ddan_Kd 0x000B down 220 (y 260), its flag set; one-point 3050 played (its chime).
    let s = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<oot_actors::bg_ddan_kd::BgDdanKd>(h)).unwrap();
    assert_eq!((s.action, s.actor.world_pos.y), (oot_actors::bg_ddan_kd::Action::DoNothing, 260.0));
    assert!(w.flags.get_switch(0x0B));
    assert!(sfx_count(&w, oot_game::audio::sfx::NA_SE_SY_CORRECT_CHIME) >= 1);
    assert_eq!(w.save.health, start.health);
    ends_as_preset(&mut w, "cavern-stairs", |_| {});
}

#[test]
fn the_last_silver_rupee_opens_door_17() {
    let Some(a) = assets() else { return };
    let (w, run, start) = run(&a, Route::SilverRupees);
    assert_eq!(steps(&run), vec![Step::BombOut, Step::PutDown, Step::CrateBroken, Step::SilverRupees]);
    // The tracker's flag 0x25; door 17 (SHUTTER_FRONT_SWITCH on it) unbarred.
    assert!(w.flags.get_switch(0x25));
    let door = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<oot_actors::door_shutter::DoorShutter>(h).filter(|d| d.actor.params & 0x3F == 0x25)).expect("door 17");
    assert!(door.bars_closed_amount < 1.0, "{}", door.bars_closed_amount);
    // The rupee's 5.
    assert_eq!(w.save.rupees + w.save.rupee_accumulator, start.rupees + 5);
    assert_eq!(w.save.health, start.health);
}

#[test]
fn room_14_s_gravestone_hides_a_chest() {
    let Some(a) = assets() else { return };
    let (mut w, run, start) = run(&a, Route::ShieldChest);
    assert_eq!(steps(&run), vec![Step::StonePulled, Step::ShieldChest]);
    // En_Box 0x5541: treasure flag 1; GI_SHIELD_HYLIAN (item.h 0x2A) to a Link who has the shield:
    // a blue rupee's text.
    assert!(w.flags.get_treasure(1));
    assert_eq!(w.save.health, start.health);
    // Room 14's clear flag is the debug start's (its Poe not spawned), not the run's.
    ends_as_preset(&mut w, "cavern-rooms", |s| s.scene_flags[0x01].clear &= !(1 << 14));
}

#[test]
fn the_song_of_time_takes_room_15_s_nearest_block() {
    let Some(a) = assets() else { return };
    let (w, run, start) = run(&a, Route::TimeBlocks);
    assert_eq!(steps(&run), vec![Step::OcarinaOut, Step::TimeSongPlayed, Step::TimeBlockGone]);
    // Only the block within 100 of Link (sRanges[1]) heard the song: the other 14 stand.
    let blocks: Vec<_> = w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<oot_actors::obj_timeblock::ObjTimeblock>(h)).collect();
    assert_eq!(blocks.len(), 15);
    assert_eq!(blocks.iter().filter(|b| !b.is_visible).count(), 1);
    assert_eq!(w.save.health, start.health);
}

/// The six rooms this milestone finishes: every placement ported (no `Placeholder`).
#[test]
fn rooms_2_5_6_13_14_15_are_all_ported() {
    let Some(a) = assets() else { return };
    // Link inside each room (else the room he stands in is loaded back and the room's objects never
    // come in), where game-cavern-rooms.bat's debug starts put him: a floor under him.
    for (room, at) in [
        (2i8, Vec3::new(-1960.0, 0.0, -1150.0)),
        (5, Vec3::new(-1960.0, 531.0, -650.0)),
        (6, Vec3::new(1840.0, 411.0, -1550.0)),
        (13, Vec3::new(2433.0, 100.0, -2960.0)),
        (14, Vec3::new(1473.0, 76.0, -4300.0)),
        (15, Vec3::new(-1958.0, 0.0, -650.0)),
    ] {
        let e = a.scenes.entrance_index("ENTR_DODONGOS_CAVERN_0").unwrap();
        let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
        save.apply_preset("fairy-magic").unwrap();
        let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap();
        assert!(w.room_request(room));
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
        w.room_change_done();
        w.place_player(at, 0);
        let (floor, _) = w.col.entity_raycast_down(at + Vec3::new(0.0, 50.0, 0.0));
        assert!((floor - at.y).abs() < 1.0, "room {room}: the start's floor at {floor}");
        // Until every actor's object is in and its init has run.
        let in_room = |w: &PlayState| -> Vec<oot_game::actor_ctx::ActorHandle> { w.actors.all().into_iter().filter(|&h| w.actors.actor(h).is_some_and(|a| a.room == room)).collect() };
        for _ in 0..120 {
            w.tick_with(scripted_input(PadState::default(), PadState::default()));
            if in_room(&w).into_iter().all(|h| w.actors.get(h).is_some_and(|i| i.name() != "(not initialised)")) {
                break;
            }
        }
        let waiting: Vec<i16> = in_room(&w).into_iter().filter(|&h| w.actors.get(h).is_none_or(|i| i.name() == "(not initialised)")).map(|h| w.actors.actor(h).unwrap().id).collect();
        assert!(waiting.is_empty(), "room {room}: never initialised: {waiting:x?}");
        let unported: Vec<String> = w
            .actors
            .all()
            .into_iter()
            .filter(|&h| w.actors.actor(h).is_some_and(|a| a.room == room))
            .filter_map(|h| w.actors.get(h).filter(|i| i.name() == "Placeholder").map(|_| format!("{:#06x}", w.actors.actor(h).unwrap().id)))
            .collect();
        assert!(unported.is_empty(), "room {room}: {unported:?}");
    }
}

/// Child Link in `room` at `at` (`fairy-magic`), the room's actors in.
fn in_room(a: &Arc<GameAssets>, room: i8, at: Vec3) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DODONGOS_CAVERN_0").unwrap();
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("fairy-magic").unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap();
    assert!(w.room_request(room));
    w.tick_with(scripted_input(PadState::default(), PadState::default()));
    w.room_change_done();
    w.place_player(at, 0);
    for _ in 0..60 {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
    w
}

/// The room's enemies killed (`Actor_Kill`: the hand test fights them), then until the room-clear
/// chest whose home is `chest` has appeared and Link is free; the frames it took.
fn clear_and_wait_chest(w: &mut PlayState, room: i8, chest: Vec3) -> usize {
    use oot_actors::en_box::{Action, EnBox};
    let chest_action = |w: &PlayState| w.actors.all().into_iter().find_map(|h| w.actors.downcast::<EnBox>(h).filter(|b| b.actor.home_pos.distance(chest) < 1.0).map(|b| b.action)).expect("the chest");
    assert_eq!(chest_action(w), Action::AppearOnRoomClear);
    assert!(!w.flags.get_clear(room));
    let enemies: Vec<_> = w.actors.category(oot_game::actor_ctx::ACTORCAT_ENEMY).to_vec();
    assert!(!enemies.is_empty());
    for h in enemies {
        if w.actors.actor(h).is_some_and(|a| a.room == room) {
            w.actors.actor_mut(h).unwrap().kill();
        }
    }
    for n in 1..=600 {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
        if chest_action(w) == Action::WaitOpen && !w.player_in_cs_mode() && w.active_cam_id == oot_game::camera::CAM_ID_MAIN {
            assert!(w.flags.get_clear(room));
            return n;
        }
    }
    panic!("the chest never appeared ({:?})", chest_action(w));
}

#[test]
fn room_5_s_compass_chest_comes_when_the_dodongos_are_gone() {
    let Some(a) = assets() else { return };
    let mut w = in_room(&a, 5, Vec3::new(-1960.0, 531.0, -650.0));
    // En_Box 0x1805: ENBOX_TYPE_ROOM_CLEAR_BIG (type 1), GI_COMPASS (item.h 0x40), treasure flag 5.
    let n = clear_and_wait_chest(&mut w, 5, Vec3::new(-1960.0, 531.0, -28.0));
    assert!(n > 1);
}

#[test]
fn room_6_s_deku_shield_chest_comes_when_the_larvae_are_gone() {
    let Some(a) = assets() else { return };
    let mut w = in_room(&a, 6, Vec3::new(1840.0, 411.0, -1550.0));
    // En_Box 0x7522: ENBOX_TYPE_ROOM_CLEAR_SMALL (type 7), GI_SHIELD_DEKU (item.h 0x29), treasure
    // flag 2; the Gold Skulltula (En_Sw 0x8210) isn't an enemy to the clear (its init makes it
    // ACTORCAT_NPC).
    let n = clear_and_wait_chest(&mut w, 6, Vec3::new(1807.0, 411.0, -1710.0));
    assert!(n > 1);
}
