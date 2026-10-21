//! GAME-10 milestone 4's runs, each a short debug start in Jabu-Jabu's Belly (Master Quest), no
//! fights (the rooms marked cleared): `Route::JabuCow` (room 0's west cow shot, door 1),
//! `Route::JabuElevator` (room 1's elevator down, the water, the yellow switch 0x19) and
//! `Route::JabuBoomerang` (room 14's chest). Each run's end that a preset holds is checked
//! against the run.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn run(a: &Arc<GameAssets>, route: Route) -> (PlayState, Playthrough) {
    let e = a.scenes.entrance_index(route.entrance()).unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), route.save(e)).unwrap();
    w.audio.log = Some(Default::default());
    route.debug_start(&mut w);
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    let trace = std::env::var("JABU_TRACE").is_ok();
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
        if trace && w.gameplay_frames % 20 == 0 {
            let pl = oot_actors::PlayExt::player(&w);
            eprintln!("f {} pos {} speed {} action {:?} state1 {:x} stick {} {} item {} fire {} door {}", w.gameplay_frames, pl.actor.world_pos, pl.linear_velocity, pl.action, pl.state1, p.stick_x, p.stick_y, pl.held_item_ap, pl.unk_860, pl.door_type);
        }
    }
    eprintln!("steps {:?}", run.steps);
    eprintln!("texts {:x?}", run.texts);
    eprintln!("link {} room {}", w.player_pos(), w.room_ctx.cur.num);
    assert!(run.failure.is_none(), "{:?}", run.failure);
    (w, run)
}

/// The run's save is the preset's in its flags, inventory and equipment, with `extra` applied to
/// the run's (what the run doesn't do that the preset has).
fn ends_as_preset(w: &mut PlayState, preset: &str, extra: impl Fn(&mut SaveContext)) {
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

trait PlayerPos {
    fn player_pos(&self) -> glam::Vec3;
}
impl PlayerPos for PlayState {
    fn player_pos(&self) -> glam::Vec3 {
        self.player.and_then(|h| self.actors.actor(h)).map(|a| a.world_pos).unwrap_or_default()
    }
}

#[test]
fn room_0s_west_cow_shot_opens_door_1() {
    let Some(a) = assets() else { return };
    let (mut w, run) = run(&a, Route::JabuCow);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::MenuOpened, Step::CursorOnItem, Step::ItemEquipped, Step::MenuClosed, Step::SlingshotDrawn, Step::CowShot, Step::DoorOpened, Step::ThroughDoor]);
    // En_Wonder_Item 0x1915 (WONDERITEM_INTERACT_SWITCH): switch flag 0x15; through door 1 in room 1.
    assert!(w.flags.get_switch(0x15));
    assert_eq!(w.room_ctx.cur.num, 1);
    // Less the debug start's room 0 clear (Flags_SetClear: the save keeps the scene's).
    ends_as_preset(&mut w, "jabu-room-1", |s| s.scene_flags[0x02].clear &= !(1 << 0));
}

#[test]
fn room_1s_elevator_down_and_the_yellow_switch() {
    let Some(a) = assets() else { return };
    let (mut w, run) = run(&a, Route::JabuElevator);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::ElevatorLevel, Step::OnElevator, Step::ElevatorDown, Step::Dived, Step::YellowSwitch]);
    assert!(w.flags.get_switch(0x19));
    ends_as_preset(&mut w, "jabu-room-14", |s| s.scene_flags[0x02].clear &= !(1 << 1));
}

#[test]
fn room_14s_chest_holds_the_boomerang() {
    let Some(a) = assets() else { return };
    let (mut w, run) = run(&a, Route::JabuBoomerang);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::BoomerangChest]);
    // En_Box 0x10C6: GI_BOOMERANG, treasure flag 6; Item_Give(ITEM_BOOMERANG) (item.h: 0x0E).
    assert!(w.flags.get_treasure(6));
    // SLOT_BOOMERANG (item.h: 0x0C).
    assert_eq!(w.save.inventory.items[0x0C], oot_game::item::ITEM_BOOMERANG);
    // The boomerang's get-item text (sGetItemTable's GI_BOOMERANG: 0x35).
    assert!(run.texts.contains(&0x35), "{:x?}", run.texts);
    ends_as_preset(&mut w, "jabu-boomerang", |s| s.scene_flags[0x02].clear &= !(1 << 14));
}

#[test]
fn after_the_chest_door_14_leads_from_room_1_into_room_14() {
    let Some(a) = assets() else { return };
    let (w, run) = run(&a, Route::JabuDoor14);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::DoorOpened, Step::ThroughDoor]);
    // En_Siofuki 0x15C6 with its chest flag 6 set: at -45 for good (EnSiofuki_Init).
    assert_eq!(w.room_ctx.cur.num, 14);
}

#[test]
fn the_hand_tests_starts_stand_on_a_floor_in_their_room() {
    use oot_actors::PlayExt;
    let Some(a) = assets() else { return };
    let e = a.scenes.entrance_index("ENTR_JABU_JABU_0").unwrap();
    for (name, preset, room, pos, yaw, clears, switches) in oot_actors::playthrough::JABU_STARTS {
        let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
        save.apply_preset(preset).unwrap();
        let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap();
        for &c in clears {
            w.flags.set_clear(c);
        }
        if room != 0 {
            assert!(w.room_request(room), "{name}");
            w.tick_with(scripted_input(PadState::default(), PadState::default()));
            w.room_change_done();
        }
        for &s in switches {
            w.flags.set_switch(s);
        }
        w.place_player(pos, yaw);
        for _ in 0..10 {
            w.tick_with(scripted_input(PadState::default(), PadState::default()));
        }
        let p = w.player_pos();
        // Room 7's floor conveyors (speed 2) drift Link towards its middle: 5 or so in 10 frames.
        let drift = if room == 7 { 10.0 } else { 3.0 };
        assert!((p.y - pos.y).abs() < 3.0 && (p.x - pos.x).abs() < drift && (p.z - pos.z).abs() < drift, "{name}: Link at {p} from {pos}");
        assert_eq!(w.room_ctx.cur.num, room, "{name}");
        assert!(w.player().state1 & oot_game::play::PLAYER_STATE1_27 == 0, "{name}: swimming");
    }
}

#[test]
fn room_5s_crystal_door_0_and_the_fall_to_barinade() {
    let Some(a) = assets() else { return };
    let (mut w, run) = run(&a, Route::JabuCrystal);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::DoorOpened, Step::ThroughDoor, Step::SlingshotDrawn, Step::CrystalShot, Step::DoorOpened, Step::BarinadesRoom]);
    // SCENE_JABU_JABU_BOSS (scene_table.h: 0x13).
    assert_eq!(w.scene_id, 0x13);
    // Less the debug start's room 1 and 5 clears.
    ends_as_preset(&mut w, "jabu-barinade", |s| s.scene_flags[0x02].clear &= !((1 << 1) | (1 << 5)));
}

#[test]
fn room_11s_stick_lit_and_carried_to_room_7s_web() {
    let Some(a) = assets() else { return };
    let (mut w, run) = run(&a, Route::JabuWeb);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::StickChest, Step::MenuOpened, Step::CursorOnItem, Step::ItemEquipped, Step::MenuClosed, Step::StickOut, Step::StickLit, Step::DoorOpened, Step::ThroughDoor, Step::WebBurnt]);
    // Bg_Ydan_Sp 0x1FD6: its isDestroyedSwitchFlag 0x16.
    assert!(w.flags.get_switch(0x16));
    assert_eq!(w.room_ctx.cur.num, 7);
    ends_as_preset(&mut w, "jabu-web", |s| s.scene_flags[0x02].clear &= !(1 << 11));
}

#[test]
fn room_2_ruto_met_and_the_cow_freed_open_door_4() {
    let Some(a) = assets() else { return };
    let (mut w, run) = run(&a, Route::JabuRuto);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(
        steps,
        vec![Step::RutoMet, Step::MenuOpened, Step::CursorOnItem, Step::ItemEquipped, Step::MenuClosed, Step::BombOut, Step::FuseShort, Step::BombThrown, Step::BouldersBroken, Step::SlingshotDrawn, Step::CowShot, Step::DoorOpened, Step::ThroughDoor]
    );
    // Her texts in gRutoFirstMeetingCs: 0x404B, 0x401E.
    assert!(run.texts.contains(&0x404B) && run.texts.contains(&0x401E), "{:x?}", run.texts);
    assert_eq!(w.room_ctx.cur.num, 7);
    ends_as_preset(&mut w, "jabu-room-7", |s| s.scene_flags[0x02].clear &= !(1 << 2));
}
