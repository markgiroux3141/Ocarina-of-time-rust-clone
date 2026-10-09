//! GAME-05 milestone 6a's exit (`Route::Gohma`, the `gohma` script's run, also a golden trace):
//! in Queen Gohma's room, from the entrance's spawn with the sword, the shield and the slingshot,
//! Link walks in and her intro plays; he looks up at her with the slingshot until she notices,
//! and her drop and the boss's title card follow; in the fight a seed into her red eye stuns her
//! and jump slashes hurt her, a seed into her eye on the ceiling knocks her down; at no health her
//! death plays, she shrinks away, and Link takes the heart container.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use oot_actors::PlayExt;
use oot_actors::boss_goma::BossGoma;
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::play::scripted_input;
use oot_game::play_scene::GameAssets;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn gohma(w: &oot_game::play::PlayState) -> Option<&BossGoma> {
    w.actors.all().into_iter().find_map(|h| w.actors.downcast::<BossGoma>(h))
}

#[test]
fn exit_link_beats_queen_gohma_and_takes_the_heart_container() {
    let Some(a) = assets() else { return };
    let route = Route::Gohma;
    let e = a.scenes.entrance_index(route.entrance()).expect("entrance");
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), route.save(e)).expect("Play_Init");
    route.debug_start(&mut w);
    let capacity = w.save.health_capacity;
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    let log = std::env::var("GOHMA_LOG").is_ok();
    let mut frame = 0;
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
        frame += 1;
        if log && frame % 5 == 0 {
            let pl = w.player();
            match gohma(&w) {
                Some(g) => eprintln!(
                    "{frame} {:?}/{} hp {} eye {} lid {} inv {} pat {} pos {:.0} | Link {:?} {:.0} 6AD {} hp {} | {}",
                    g.action, g.action_state, g.actor.col_chk_info.health as i8, g.eye_closed_timer, g.eye_lid_bottom_rot_x, g.invincibility_frames, g.patience_timer, g.actor.world_pos,
                    pl.action, pl.actor.world_pos, pl.unk_6AD, w.save.health, run.at()
                ),
                None => eprintln!("{frame} gone | Link {:?} {:.0} | {}", pl.action, pl.actor.world_pos, run.at()),
            }
        }
    }
    assert!(run.failure.is_none(), "{:?}", run.failure);
    eprintln!("steps {:?}", run.steps);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    for s in [Step::GohmaEntered, Step::GohmaWaiting, Step::GohmaLookedAt, Step::GohmaBattle, Step::GohmaStunned, Step::GohmaHit, Step::GohmaDefeated, Step::GohmaGone, Step::HeartTaken] {
        assert!(steps.contains(&s), "{s:?} in {steps:?}");
    }
    // BossGoma_Encounter: EVENTCHKINF_BEGAN_GOHMA_BATTLE; BossGoma_Defeated: the room cleared;
    // Item_B_Heart: its flag and a heart more (GI_HEART_CONTAINER_2).
    assert!(w.save.get_event_chk_inf(oot_actors::boss_goma::EVENTCHKINF_BEGAN_GOHMA_BATTLE));
    assert!(w.flags.get_clear(w.room_ctx.cur.num));
    assert!(w.flags.get_collectible(0x1F));
    assert_eq!(w.save.health_capacity, capacity + 0x10);
}
