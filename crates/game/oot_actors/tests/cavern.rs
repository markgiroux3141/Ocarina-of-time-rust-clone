//! GAME-09 milestone 3's runs in Dodongo's Cavern (Master Quest), each a short debug start of one
//! of the way's moments: `Route::BombBag` (the lobby's big chest from its east corridor on
//! `cavern-loop`: `GI_BOMB_BAG_20`). Each run's end that a later one starts from is a preset,
//! checked here against the run.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use oot_actors::PlayExt;
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Runs `route` from its start; the play state at its end, its run and its start's save.
fn run(a: &Arc<GameAssets>, route: Route) -> (PlayState, Playthrough, SaveContext) {
    let e = a.scenes.entrance_index(route.entrance()).unwrap();
    let start = route.save(e);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), start.clone()).unwrap();
    route.debug_start(&mut w);
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

#[test]
fn the_lobby_s_big_chest_holds_the_bomb_bag() {
    let Some(a) = assets() else { return };
    let (mut w, run, start) = run(&a, Route::BombBag);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::BombBag]);
    // En_Box 0x0644: GI_BOMB_BAG_20 (item.h 0x32), its get-item text (sGetItemTable's
    // GET_ITEM(ITEM_BOMB_BAG_20, OBJECT_GI_BOMBPOUCH, GID_BOMB_BAG_20, 0x58, ...)); Item_Give:
    // the bag (UPG_BOMB_BAG 1) and 20 bombs; the chest's treasure flag 4.
    assert!(run.texts.contains(&0x58), "{:x?}", run.texts);
    assert_eq!(w.save.inventory.ammo[oot_game::item::slot(oot_game::item::ITEM_BOMB)], 20);
    assert!(w.flags.get_treasure(4));
    assert_eq!(w.save.health, start.health);
    ends_as_preset(&mut w, "cavern-bomb-bag", |_| {});
}

#[test]
fn the_gravestone_off_its_switch_opens_the_way_down_to_king_dodongo() {
    let Some(a) = assets() else { return };
    let (mut w, run, start) = run(&a, Route::Gravestone);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::StonePulled, Step::BossPathOpen, Step::DoorOpened, Step::BossFall]);
    // ddan_boss's spawn 0 (ENTR_DODONGOS_CAVERN_BOSS_0): the fall's end.
    assert_eq!(w.scene_id, 0x12);
    // A Baby Dodongo (room 7's En_Dodojr at (-203, 16, -2929)) surfaces as Link crosses to door
    // 20 and jumps at him: 8 (half a heart). The run doesn't fight.
    assert_eq!(w.save.health, start.health - 8);
    // Switch 0x20 is a temporary flag (0x20 to 0x3F): lost with the scene. The run's end is
    // `cavern-boss`, milestone 4's start.
    ends_as_preset(&mut w, "cavern-boss", |_| {});
}

#[test]
fn the_island_s_flower_blows_the_pit_boulder_and_the_pillar_takes_link_up() {
    let Some(a) = assets() else { return };
    let (mut w, run, start) = run(&a, Route::Pillar);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    use Step::*;
    assert_eq!(steps, vec![FlowerLifted, Thrown, PitBoulderBlown, PillarSwitch, LadderUp, PillarReady, OnPillar, PillarReady, UpperLevel]);
    // Obj_Bombiwa 0x000E's flag, then Obj_Switch 0x0A00's: Bg_Ddan_Jd 0x000A, the shortcut
    // pillar, shuttles (+140 and +700 at ySpeed 5).
    assert!(w.flags.get_switch(0x0E) && w.flags.get_switch(0x0A));
    // Off the pillar's top (y 560) east: the mid-air ledge grab (func_8083A5C4) on the upper
    // level's edge, the climb onto it (y 531).
    let p = w.player().actor.world_pos;
    assert!(p.y == 531.0 && p.x > 640.0, "{p:?}");
    // Carrying, the run-off jump off the island falls short into the pit's lava (y -180,
    // FLOOR_TYPE_2: Player's hot floor, a quarter heart) and the throw is made from there; the
    // flower lands 66 from the boulder, Link out of the blast's reach.
    assert_eq!(w.save.health, start.health - 4);
    ends_as_preset(&mut w, "cavern-upper", |_| {});
}

#[test]
fn bombs_through_the_bridge_s_holes_light_the_skull_s_eyes_and_open_its_jaw() {
    use oot_actors::bg_dodoago::{Action, BgDodoago, ONEPOINT_EYE, ONEPOINT_JAW};
    let Some(a) = assets() else { return };
    let route = Route::Skull;
    let e = a.scenes.entrance_index(route.entrance()).unwrap();
    let start = route.save(e);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), start.clone()).unwrap();
    route.debug_start(&mut w);
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    let (mut cutscenes, mut caught) = (Vec::new(), Vec::new());
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
        let cs = w.camera(w.active_cam_id).filter(|_| w.active_cam_id != oot_game::camera::CAM_ID_MAIN).map_or(0, |c| c.cs_id);
        if cs != 0 && cutscenes.last() != Some(&cs) {
            cutscenes.push(cs);
        }
        // The bomb catcher: a bomb (params 0) falling through a hole's cylinder, its fuse set to 50
        // in the skull's turn and counted down in its own (49 at the frame's end), the skull's
        // parent.
        let skull = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<BgDodoago>(h));
        if let Some(b) = skull.and_then(|s| s.actor.parent).and_then(|h| w.actors.downcast::<oot_actors::en_bom::EnBom>(h))
            && b.timer == 49
        {
            caught.push(b.actor.world_pos.x.signum());
        }
    }
    assert!(run.failure.is_none(), "{:?}", run.failure);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    use Step::*;
    assert_eq!(steps, vec![MenuOpened, CursorOnItem, ItemEquipped, MenuClosed, BombOut, PutDown, EyeLit, BombOut, PutDown, JawOpen]);
    // Each put down at a hole's edge falls onto the head by that side's eye. The first through the
    // right catcher (fuse 50); its blast, the first explosive, is the skull's parent with sTimer
    // 50, let go only past 140: the second comes sooner and falls through the left one on its own
    // fuse. One-point 3065 for the first eye, 3380 for the second (the jaw).
    assert_eq!(caught, vec![1.0]);
    assert_eq!(cutscenes, vec![ONEPOINT_EYE, ONEPOINT_JAW]);
    assert!(w.flags.get_switch(0x1A));
    let skull = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<BgDodoago>(h)).unwrap();
    assert_eq!(skull.action, Action::DoNothing);
    assert_eq!(w.scene.as_ref().unwrap().draw.room_draw_params, [255, 255]);
    // The blasts on the head, 280 below the bridge: Link unhurt.
    assert_eq!(w.save.health, start.health);
    assert_eq!(w.save.inventory.ammo[oot_game::item::SLOT_BOMB], 18);
    ends_as_preset(&mut w, "cavern-jaw", |_| {});
}
