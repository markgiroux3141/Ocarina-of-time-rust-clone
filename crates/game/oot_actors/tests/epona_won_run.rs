//! GAME-12 milestone 3's run: Epona won and out (`Route::EponaWon`): the second race won, Ingo's
//! gate scene, the jump over the shut gate past him, the field's jump cutscene. Its end is
//! `epona-won`, Phase 14's start.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use oot_actors::PlayExt;
use oot_actors::en_horse::*;
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn run(a: &Arc<GameAssets>, route: Route, mut watch: impl FnMut(&PlayState)) -> (PlayState, Playthrough) {
    let e = a.scenes.entrance_index(route.entrance()).unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), route.save(e)).unwrap();
    w.audio.log = Some(Default::default());
    route.debug_start(&mut w);
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
        watch(&w);
    }
    eprintln!("steps {:?}", run.steps);
    eprintln!("texts {:x?}", run.texts);
    eprintln!("at {:?} scene {:#x} layer {} entrance {:#x}", w.player().actor.world_pos, w.scene_id, w.save.scene_layer, w.save.entrance_index);
    assert!(run.failure.is_none(), "{:?}", run.failure);
    (w, run)
}

fn steps(run: &Playthrough) -> Vec<Step> {
    run.steps.iter().map(|s| s.0).collect()
}

#[test]
fn epona_won_and_out() {
    let Some(a) = assets() else { return };
    // Epona's actions and Link's riding through the run.
    let mut actions: Vec<(u16, i32)> = Vec::new();
    let mut jumped = false;
    let (mut w, run) = run(&a, Route::EponaWon, |w| {
        if let Some(e) = w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<EnHorse>(h)).find(|h| h.ty == HORSE_EPONA && !h.actor.killed) {
            if actions.last() != Some(&(w.scene_id, e.action)) {
                actions.push((w.scene_id, e.action));
            }
            jumped |= w.scene_id == 0x63 && e.action == ENHORSE_ACT_HIGH_JUMP;
        }
    });
    eprintln!("{actions:?}");
    assert_eq!(steps(&run), vec![Step::GateShut, Step::JumpedOut, Step::EponaWon]);
    // EnIn_PlayerWonSecondRace: 0x203B, then 0x203C (EnIn_ClosingGate).
    assert_eq!(run.texts, vec![0x203B, 0x203C]);
    // Over the shut gate (80 high: EnHorse_UpdateBgCheckInfo's high jump past 13.8, spurred).
    assert!(jumped, "{actions:?}");
    // In the field: Horse_SetupInGameplay's fence jump (ENTR_HYRULE_FIELD_15, no respawn) spawns
    // her HORSE_PTYPE_7 under Link, mounted; the cutscene drives her (ENHORSE_ACT_CS_UPDATE).
    assert!(actions.contains(&(0x51, ENHORSE_ACT_CS_UPDATE)), "{actions:?}");
    // gHyruleFieldGateEponaJumpCs's row of sEntranceCutsceneTable: EVENTCHKINF_EPONA_OBTAINED.
    assert_eq!(w.save.entrance_index, a.scenes.entrance_index("ENTR_HYRULE_FIELD_15").unwrap());
    assert!(w.save.get_event_chk_inf(0x18));
    // Ends as epona-won.
    w.save_scene_flags();
    let s = w.save.clone();
    let mut p = SaveContext::new(s.entrance_index, false, s.day_time);
    p.apply_preset("epona-won").unwrap();
    assert_eq!((&p.event_chk_inf, &p.inf_table, &p.item_get_inf), (&s.event_chk_inf, &s.inf_table, &s.item_get_inf));
    assert_eq!((&p.inventory.items, p.inventory.quest_items, p.inventory.equipment), (&s.inventory.items, s.inventory.quest_items, s.inventory.equipment));
}

#[test]
fn the_four_fence_jumps() {
    let Some(a) = assets() else { return };
    // sEntranceCutsceneTable's four rows (ENTR_HYRULE_FIELD_11 south, _13 east, _12 west, _15 the
    // gate) with EVENTCHKINF_EPONA_OBTAINED: entered without it (ingo-second-win) and no respawn,
    // Horse_SetupInGameplay spawns Epona HORSE_PTYPE_7 under Link, mounted, and the jump's cutscene
    // plays; its end leaves Epona his (the flag the table sets as it starts the cutscene) and
    // horseData's scene the field's.
    for name in ["ENTR_HYRULE_FIELD_11", "ENTR_HYRULE_FIELD_12", "ENTR_HYRULE_FIELD_13", "ENTR_HYRULE_FIELD_15"] {
        let e = a.scenes.entrance_index(name).unwrap();
        let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
        save.apply_preset("ingo-second-win").unwrap();
        let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap();
        let mut started = false;
        for _ in 0..2000 {
            w.tick_with(scripted_input(PadState::default(), PadState::default()));
            started |= w.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE;
            if started && w.cs_ctx.state == oot_game::cutscene::CS_STATE_IDLE {
                break;
            }
        }
        assert!(started, "{name}: no cutscene");
        assert_eq!(w.cs_ctx.state, oot_game::cutscene::CS_STATE_IDLE, "{name}: the cutscene never ended");
        assert!(w.save.get_event_chk_inf(0x18), "{name}");
        assert_ne!(w.player().state1 & oot_actors::player::STATE1_23, 0, "{name}: not on Epona");
        assert_eq!(w.save.horse_data.scene_id, 0x51, "{name}");
        // Play_TriggerRespawn's Play_LoadToLastEntrance (OOT_VERSION >= PAL_1_1): a fall now goes
        // to ENTR_HYRULE_FIELD_6, not into the jump again.
        w.trigger_respawn();
        assert_eq!(w.transition.next_entrance_index, a.scenes.entrance_index("ENTR_HYRULE_FIELD_6").unwrap(), "{name}");
    }
}
