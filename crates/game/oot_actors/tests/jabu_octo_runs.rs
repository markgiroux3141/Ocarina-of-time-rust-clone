//! GAME-10 milestone 8's runs, short debug starts in Jabu-Jabu's Belly (Master Quest), no fights
//! (the rooms marked cleared): `Route::JabuSwitch` (room 3: Ruto talked to, lifted and carried
//! onto the blue switch 0x3C: door 12) and `Route::JabuOcto` (Ruto carried into room 6 and thrown
//! onto Big Octo's platform: her two scripts, the platform's rise, Big Octo's arrival and the
//! descent to the fight's start). Each run's end that a preset holds is checked against the run.
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
fn room_3_ruto_carried_onto_the_blue_switch_opens_door_12() {
    let Some(a) = assets() else { return };
    let (w, run) = run(&a, Route::JabuSwitch);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::RutoExplained, Step::RutoSeated, Step::RutoLifted, Step::YellowSwitch, Step::SpoutUp, Step::RutoPutDown, Step::RutoOnSwitch]);
    // Bg_Bdan_Switch 0x3C00 (blue): held by Ruto, switch flag 0x3C (door 12).
    assert!(w.flags.get_switch(0x3C));
}

#[test]
fn ruto_carried_to_room_6_and_thrown_onto_big_octos_platform() {
    use oot_actors::bg_bdan_objects::{Action, BgBdanObjects};
    let Some(a) = assets() else { return };
    let (mut w, run) = run(&a, Route::JabuOcto);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(
        steps,
        vec![
            Step::RutoExplained,
            Step::RutoSeated,
            Step::RutoLifted,
            Step::YellowSwitch,
            Step::SpoutUp,
            Step::DoorOpened,
            Step::ThroughDoor,
            Step::RutoFoundSapphire,
            Step::RutoThrown,
            Step::RutoHasSapphire,
            Step::PlatformRising,
            Step::BigOctoArrived,
            Step::BigOctoBattle,
        ]
    );
    // Her talks (0x404C, 0x404D), gRutoFoundSapphireCs's 0x404F, gRutoObtainingSapphireCs's
    // 0x4050 to 0x4052, and EnRu1_RespondToAbduction's 0x4048.
    assert_eq!(run.texts, vec![0x404C, 0x404D, 0x404F, 0x4050, 0x4051, 0x4052, 0x4048]);
    assert_eq!(w.room_ctx.cur.num, 6);
    // BgBdanObjects_OctoPlatform_DescendWithBigOcto put Link at (-1130, -1025, -3500); Big Octo
    // (his params 2) on the platform's rim, the battle on.
    assert_eq!(w.player_pos(), glam::Vec3::new(-1130.0, -1025.0, -3500.0));
    let platform = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<BgBdanObjects>(h).filter(|o| o.action == Action::OctoPlatformBattleInProgress).map(|o| o.actor.child)).expect("the battle on");
    let octo = platform.and_then(|h| w.actors.actor(h)).expect("Big Octo");
    assert_eq!((octo.id, octo.params), (oot_actors::en_bigokuta::ACTOR_EN_BIGOKUTA, 2));
    // Less the debug start's room 3 clear; with room 10 visited (the green tentacle's, whose flag
    // 0x0C the debug start sets).
    ends_as_preset(&mut w, "jabu-big-octo", |s| {
        s.scene_flags[0x02].clear &= !(1 << 3);
        s.scene_flags[0x02].rooms |= 1 << 10;
    });
}
