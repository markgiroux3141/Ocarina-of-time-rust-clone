//! GAME-08 milestone 3 against the C: adult Malon's init (`z_en_ma2.c`, `EnMa2_GetSpawnIndex`):
//! killed for a child, a placeholder where an adult would meet her.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::en_ma2;
use oot_game::actor::Actor;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

/// `EVENTCHKINF_B4` (`save.h`): the ranch's intro seen.
const EVENTCHKINF_B4: u16 = 0xB4;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Link (`adult` or a child) entered at `entrance` at `hour`:00 with `preset`, `edit` applied to
/// the save last; four frames on (the room's actors are in on the third).
fn enter_age(a: &Arc<GameAssets>, entrance: &str, adult: bool, hour: i32, preset: &str, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, adult, oot_game::env::clock_time(hour, 0) as u16);
    save.apply_preset(preset).unwrap();
    save.set_event_chk_inf(EVENTCHKINF_B4);
    edit(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    for _ in 0..4 {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
    w
}

/// A child in the stable after Talon's return (`castle-courtyard`).
fn stable(a: &Arc<GameAssets>, hour: i32, _talon_back: bool) -> PlayState {
    enter_age(a, "ENTR_STABLE_0", false, hour, "castle-courtyard", |_| {})
}

/// The live placeholders standing in for actor `id`.
fn placeholders(w: &PlayState, id: i16) -> Vec<Vec3> {
    w.actors.all().into_iter().filter(|&h| w.actors.get(h).is_some_and(|a| a.name() == "Placeholder" && a.base().id == id && !a.base().killed)).map(|h| w.actors.actor(h).unwrap().home_pos).collect()
}

/// Any live actor of `id`, ported or a placeholder.
fn any_live(w: &PlayState, id: i16) -> usize {
    w.actors.all().into_iter().filter(|&h| w.actors.actor(h).is_some_and(|a| a.id == id && !a.killed)).count()
}

#[test]
fn adult_malon_is_killed_for_a_child_and_a_placeholder_for_an_adult() {
    let Some(a) = assets() else { return };
    // malon_stable: En_Ma2 -1 at (120, 0, 0), rot.z 5. EnMa2_GetSpawnIndex: a child,
    // MALON_ADULT_SPAWN_NONE (killed).
    for hour in [10, 22] {
        let w = stable(&a, hour, true);
        assert_eq!(any_live(&w, en_ma2::ACTOR_EN_MA2), 0);
    }
    // An adult by day without Epona: MALON_ADULT_SPAWN_DAY_STABLES_NO_EPONA, a placeholder.
    let w = enter_age(&a, "ENTR_STABLE_0", true, 10, "deku-tree-dead", |_| {});
    assert_eq!(placeholders(&w, en_ma2::ACTOR_EN_MA2).len(), 1);
    // By night, or with Epona: MALON_ADULT_SPAWN_NONE (the stable is only her day's before Epona).
    let w = enter_age(&a, "ENTR_STABLE_0", true, 22, "deku-tree-dead", |_| {});
    assert_eq!(any_live(&w, en_ma2::ACTOR_EN_MA2), 0);
    let w = enter_age(&a, "ENTR_STABLE_0", true, 10, "deku-tree-dead", |s| s.set_event_chk_inf(en_ma2::EVENTCHKINF_EPONA_OBTAINED));
    assert_eq!(any_live(&w, en_ma2::ACTOR_EN_MA2), 0);
}

#[test]
fn adult_malons_spawn_index_follows_the_c() {
    let Some(a) = assets() else { return };
    use en_ma2::*;
    // EnMa2_GetSpawnIndex over the scene, the time, rot.z and Epona, for an adult.
    let mut w = enter_age(&a, "ENTR_STABLE_0", true, 10, "deku-tree-dead", |_| {});
    let mut actor = Actor::new(Vec3::ZERO, 0);
    let mut idx = |w: &mut PlayState, scene: u16, day: bool, rot_z: i16, epona: bool| {
        w.scene_id = scene;
        w.save.night_flag = !day;
        if epona {
            w.save.set_event_chk_inf(EVENTCHKINF_EPONA_OBTAINED);
        } else {
            w.save.clear_event_chk_inf(EVENTCHKINF_EPONA_OBTAINED);
        }
        actor.shape_rot.z = rot_z;
        EnMa2::get_spawn_index(&actor, w)
    };
    const STABLE: u16 = 0x36;
    const RANCH: u16 = 0x63;
    assert_eq!(idx(&mut w, STABLE, true, 5, false), MALON_ADULT_SPAWN_DAY_STABLES_NO_EPONA);
    assert_eq!(idx(&mut w, STABLE, true, 5, true), MALON_ADULT_SPAWN_NONE);
    assert_eq!(idx(&mut w, RANCH, false, 6, false), MALON_ADULT_SPAWN_NIGHT_RANCH_NO_EPONA);
    assert_eq!(idx(&mut w, RANCH, true, 6, false), MALON_ADULT_SPAWN_NONE);
    assert_eq!(idx(&mut w, RANCH, true, 7, true), MALON_ADULT_SPAWN_ALWAYS);
    assert_eq!(idx(&mut w, RANCH, false, 7, true), MALON_ADULT_SPAWN_NONE);
    assert_eq!(idx(&mut w, RANCH, false, 8, true), MALON_ADULT_SPAWN_ALWAYS);
    assert_eq!(idx(&mut w, RANCH, true, 8, true), MALON_ADULT_SPAWN_NONE);
    assert_eq!(idx(&mut w, STABLE, true, 7, true), MALON_ADULT_SPAWN_NONE);
    // A child: none, whatever else.
    w.save.adult = false;
    assert_eq!(idx(&mut w, STABLE, true, 5, false), MALON_ADULT_SPAWN_NONE);
}

