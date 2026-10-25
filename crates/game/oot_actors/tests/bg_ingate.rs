//! `Bg_Ingate` (GAME-12 milestone 2) against the C (`z_bg_ingate.c`): Lon Lon Ranch's gates, which
//! are there for an adult, and with Epona had only on the cutscene layer; the way out's pair
//! swinging shut as Ingo traps Link after the race for Epona.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use oot_actors::bg_ingate::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Lon Lon Ranch as an adult with `preset` (and `edit` on the save), entered at `entrance` at
/// 10:00, `ticks` frames on.
fn ranch(a: &Arc<GameAssets>, entrance: &str, preset: &str, edit: impl FnOnce(&mut SaveContext), ticks: usize) -> PlayState {
    let e = a.scenes.entrance_index(entrance).unwrap();
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset(preset).unwrap();
    edit(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap();
    for _ in 0..ticks {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
    w
}

/// The live gates: (handle, params, action, shape.rot.y).
fn gates(w: &PlayState) -> Vec<(oot_game::actor_ctx::ActorHandle, i16, Action, i16)> {
    let mut v: Vec<_> = w
        .actors
        .all()
        .into_iter()
        .filter_map(|h| w.actors.downcast::<BgInGate>(h).map(|g| (h, g)))
        .filter(|(_, g)| !g.actor.killed)
        .map(|(h, g)| (h, g.actor.params, g.action, g.actor.shape_rot.y))
        .collect();
    v.sort_by_key(|g| g.1);
    v
}

#[test]
fn the_gates_are_an_adults_and_go_with_epona() {
    let Some(a) = assets() else { return };
    // The adult's day layer places four: the corral's pair (params 0) and the way out's (1 and 3).
    // BgInGate_Init: on the ranch as an adult, without Epona, they stay, scale 0.1, doing nothing
    // (the race's state isn't INGO_RACE_STATE_TRAPPED_WIN_EPONA).
    let w = ranch(&a, "ENTR_LON_LON_RANCH_0", "adult-awake", |_| {}, 3);
    let g = gates(&w);
    assert_eq!(g.iter().map(|g| g.1).collect::<Vec<_>>(), vec![0, 0, 1, 3]);
    assert!(g.iter().all(|g| g.2 == Action::DoNothing));
    let h = g[0].0;
    assert_eq!(w.actors.actor(h).unwrap().scale.x, 0.1);
    // BgInGate_Draw's gIngoGateDL is in the pack.
    assert!(a.pack.mesh(OBJECT, "gIngoGateDL").is_ok());
    // With Epona had (EVENTCHKINF_EPONA_OBTAINED) and not on the cutscene layer: all killed.
    let w = ranch(&a, "ENTR_LON_LON_RANCH_0", "epona", |_| {}, 3);
    assert!(gates(&w).is_empty());
    // A gate spawned for a child is killed (!LINK_IS_ADULT).
    let mut w = ranch(&a, "ENTR_LON_LON_RANCH_0", "adult-awake", |_| {}, 3);
    w.save.adult = false;
    let h = w.actor_spawn(ACTOR_BG_INGATE, glam::Vec3::ZERO, [0; 3], 0).unwrap();
    assert!(w.actors.actor(h).unwrap().killed);
}

#[test]
fn the_way_outs_gates_swing_shut_when_ingo_traps_link() {
    let Some(a) = assets() else { return };
    // INGO_RACE_STATE_TRAPPED_WIN_EPONA (6) in eventInf[0]'s low bits: params bit 0's gates (1 and
    // 3, placed turned 0x4000) take func_80892890, the cutscene's frame rewound to 0; the corral's
    // (0) do nothing. Entered as the race ends (ENTR_LON_LON_RANCH_7: at the gateway, Ingo's init
    // would clear the race's word, sIsObedient). (The gates' object comes in on the first frames:
    // their inits run then.)
    let mut w = ranch(&a, "ENTR_LON_LON_RANCH_7", "adult-awake", |s| s.event_inf[0] = (s.event_inf[0] & !0xF) | 6, 3);
    let g = gates(&w);
    assert_eq!(g.iter().map(|g| (g.1, g.2)).collect::<Vec<_>>(), vec![(0, Action::DoNothing), (0, Action::DoNothing), (1, Action::Swing), (3, Action::Swing)]);
    assert_eq!(w.cs_ctx.frames, 0);
    let (h1, h3) = (g[2].0, g[3].0);
    let step = |w: &mut PlayState, frame: u16| {
        w.cs_ctx.frames = frame;
        for h in [h1, h3] {
            let mut x = w.actors.take(h).unwrap();
            x.update(w);
            w.actors.put_back(h, x);
        }
        let g = gates(w);
        (g[2].3, g[3].3, g[2].2)
    };
    // Before frame 10: as placed (0x4000).
    assert_eq!(step(&mut w, 9), (0x4000, 0x4000, Action::Swing));
    // Frame 10: Math_SinS(0) * 0x4000 = 0, still as placed.
    assert_eq!(step(&mut w, 10), (0x4000, 0x4000, Action::Swing));
    // Frame 30: Math_SinS(20 * 400 = 8000) * 16384 = sin(43.95 degrees) * 16384, about 11370:
    // params 1 (bit 1 clear) turns by minus that, params 3 by plus.
    let (r1, r3, act) = step(&mut w, 30);
    assert_eq!(act, Action::Swing);
    let d = 0x4000 - r1 as i32;
    assert!((11300..11450).contains(&d), "{d}");
    assert_eq!(r3 as i32 - 0x4000, d);
    // Frame 50: a quarter turn each way, 0x4000 - 0x4000 = 0 and 0x4000 + 0x4000 = 0x8000; done.
    assert_eq!(step(&mut w, 50), (0, i16::MIN, Action::DoNothing));
    // Then it stands whatever the frame.
    assert_eq!(step(&mut w, 20), (0, i16::MIN, Action::DoNothing));
}
