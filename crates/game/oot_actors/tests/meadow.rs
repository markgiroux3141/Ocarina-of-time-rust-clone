//! GAME-08 milestone 1: the Sacred Forest Meadow's gate and blocks (`Bg_Spot05_Soko`), and the
//! milestone's exit (`Route::Meadow`): from the Lost Woods' entrance with Zelda's letter, through
//! the woods' right turns, the Wolfos killed and its gate down, the maze's scrubs, and Saria's
//! lesson: Saria's Song learned.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use oot_actors::bg_spot05_soko::{Action, BgSpot05Soko};
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// `QUEST_SONG_SARIA` (`item.h`: 0x0E), `Item_Give(ITEM_SONG_SARIA)`'s bit.
const QUEST_SONG_SARIA: u32 = 0x0E;
/// The gray Wolfos's and the gate's switch flag (`En_Wf` 0x1F00, `Bg_Spot05_Soko` 0x1F01).
const GATE_FLAG: i32 = 0x1F;

/// The meadow from the Lost Woods (`ENTR_SACRED_FOREST_MEADOW_0`) with the route's save (a child's);
/// `f` changes the save first.
fn meadow(a: &Arc<GameAssets>, f: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_SACRED_FOREST_MEADOW_0").unwrap();
    let mut save = Route::Meadow.save(e);
    f(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap();
    for _ in 0..3 {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
    w
}

fn sokos(w: &PlayState) -> Vec<&BgSpot05Soko> {
    w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<BgSpot05Soko>(h)).collect()
}

#[test]
fn the_blocks_stand_for_a_child_only_and_the_gate_with_its_flag_clear() {
    let Some(a) = assets() else { return };
    // A child: BgSpot05Soko_Init keeps the four type 0 blocks (func_808AE5A8) and the gate (type 1,
    // its flag 0x1F clear: func_808AE5B4, updating wherever Link is).
    let w = meadow(&a, |_| {});
    let s = sokos(&w);
    assert_eq!(s.iter().filter(|b| b.action == Action::Block).count(), 4);
    let gate: Vec<_> = s.iter().filter(|b| b.action == Action::Wait).collect();
    assert_eq!(gate.len(), 1);
    assert_eq!(gate[0].switch_flag, GATE_FLAG);
    // init writes the type over the params (params &= 0xFF).
    assert_eq!(gate[0].actor.params, 1);
    assert_ne!(gate[0].actor.flags & oot_game::actor::ACTOR_FLAG_UPDATE_CULLING_DISABLED, 0);
    // (The init kills the blocks for an adult, LINK_IS_ADULT; but only the child's layers, 0 and 1,
    // place them.)
    // The flag already set: the gate is gone at init.
    let w = meadow(&a, |s| s.scene_flags[0x56].swch |= 1 << GATE_FLAG);
    assert!(sokos(&w).iter().all(|b| b.action == Action::Block));
}

#[test]
fn the_gate_sinks_120_faster_each_frame_when_its_flag_is_set() {
    let Some(a) = assets() else { return };
    let mut w = meadow(&a, |_| {});
    let home = sokos(&w).into_iter().find(|b| b.action == Action::Wait).unwrap().actor.home_pos;
    w.flags.set_switch(GATE_FLAG);
    let idle = PadState::default();
    // func_808AE5B4: on the flag, the attention cutscene (OnePointCutscene_Attention: a sub camera)
    // and speed 0.5; then func_808AE630 multiplies it by 1.5 before each step down: 0.75, 1.125, ...
    w.tick_with(scripted_input(idle, idle));
    let g = sokos(&w).into_iter().find(|b| b.action == Action::Sink).expect("sinking");
    assert_eq!(g.actor.speed_xz, 0.5);
    assert_eq!(g.actor.world_pos.y, home.y);
    let mut speed = 0.5f32;
    let mut y = home.y;
    let mut frames = 0;
    loop {
        w.tick_with(scripted_input(idle, idle));
        frames += 1;
        speed *= 1.5;
        let target = home.y - 120.0;
        let g = sokos(&w).into_iter().find(|b| b.action == Action::Sink).expect("sinking");
        // Math_StepToF: down by the speed, clamped at the bottom; there Actor_Kill.
        y = (y - speed).max(target);
        assert_eq!(g.actor.world_pos.y, y, "frame {frames}");
        if y == target {
            assert!(g.actor.killed);
            break;
        }
        assert!(frames < 20);
    }
    // The steps 0.5 × 1.5^k (k from 1) sum to 1.5^(n+1) - 1.5 after n: past 120 at n = 11.
    assert_eq!(frames, 11);
}

#[test]
fn exit_the_woods_the_wolfos_the_maze_and_sarias_song() {
    let Some(a) = assets() else { return };
    let route = Route::Meadow;
    let e = a.scenes.entrance_index(route.entrance()).unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), route.save(e)).unwrap();
    route.debug_start(&mut w);
    w.audio_side = Some(Box::new(oot_game::audio::offline::OfflineAudio::new(&a.pack.audio_data().unwrap(), false)));
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
    }
    assert!(run.failure.is_none(), "{:?}", run.failure);
    eprintln!("steps {:?}", run.steps);
    eprintln!("texts {:x?}", run.texts);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    use Step::*;
    assert_eq!(
        steps,
        vec![
            Owl, MeadowEntered, WolfosKilled, NutBounced, ScrubCaught, ScrubKilled, NutBounced, ScrubCaught, ScrubKilled, NutBounced, ScrubCaught, ScrubKilled, SariasSongLesson, SariasSongPlayed,
            SariasSongLearned
        ]
    );
    // Room 2's owl: 0x10C0 to 0x10C3; Saria's lesson: her 0x10A8 and 0x10AA, the song's staff and
    // the playback's texts, 0x10AC; then her 0x10AD.
    for t in [0x10C0, 0x10C1, 0x10A8, 0x10AC] {
        assert!(run.texts.contains(&t), "text {t:#x}: {:x?}", run.texts);
    }
    // The meadow (SCENE_SACRED_FOREST_MEADOW 0x56), the gate's flag, the song.
    assert_eq!(w.scene_id, 0x56);
    assert!(w.flags.get_switch(GATE_FLAG));
    assert!(w.save.check_quest_item(QUEST_SONG_SARIA));
    assert!(w.save.health > 0);
    // Saria plays on her stump (SARIA_TYPE_MEADOW_PLAYING's idle after EnSa_BecomeIdle).
    assert!(w.actors.all().into_iter().any(|h| w.actors.downcast::<oot_actors::en_sa::EnSa>(h).is_some()));
}
