//! GAME-05 milestone 3b's exit (`Route::Scrub`, the `scrub` script's run, also a golden trace):
//! inside the Deku Tree, from a debug start in room 4, Link guards with the Deku Shield until a
//! Mad Scrub's nut bounces back off it and knocks the scrub out of its flower, then locks on, runs
//! it down and slashes it; it dies with its effects, and its drop, if any, is picked up.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use oot_actors::PlayExt;
use oot_actors::en_dekunuts::{Action as NutsAction, EnDekunuts};
use oot_actors::playthrough::{Playthrough, Route, SCRUB_HOME, Step};
use oot_game::effect::{EFFECT_SS_DEAD_DB, EFFECT_SS_HAHEN};
use oot_game::play::scripted_input;
use oot_game::play_scene::GameAssets;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

#[test]
fn exit_link_bounces_a_mad_scrubs_nut_back_catches_it_and_kills_it() {
    let Some(a) = assets() else { return };
    let route = Route::Scrub;
    let e = a.scenes.entrance_index(route.entrance()).expect("entrance");
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), route.save(e)).expect("Play_Init");
    route.debug_start(&mut w);
    assert_eq!(w.room_ctx.cur.num, 4);
    let health = w.save.health;
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    // What the run shows: the effects spawned (each slot's life restarting).
    let (mut dead_db, mut fragments) = (0, 0);
    let mut lives: Vec<i16> = w.effect_ss.table.iter().map(|e| e.life).collect();
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
        for (i, e) in w.effect_ss.table.iter().enumerate() {
            if e.life > lives[i] || (lives[i] < 0 && e.life >= 0) {
                match e.ty {
                    EFFECT_SS_DEAD_DB => dead_db += 1,
                    EFFECT_SS_HAHEN => fragments += 1,
                    _ => {}
                }
            }
            lives[i] = e.life;
        }
        if let Some(s) = run.take_done()
            && s == Step::NutBounced
        {
            // Its own nut, bounced off the shield as Link's attack, hit it in its flower: out of
            // the ground (EnDekunuts_SetupBeginRun: mass 50), and Link unhurt.
            let n = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<EnDekunuts>(h).filter(|n| n.actor.home_pos == SCRUB_HOME && n.actor.params == 0)).expect("the scrub");
            assert_eq!((n.action, n.actor.col_chk_info.mass), (NutsAction::BeginRun, 50));
            assert_eq!(w.save.health, health);
        }
    }
    assert!(run.failure.is_none(), "{:?}", run.failure);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::NutBounced, Step::ScrubCaught, Step::ScrubKilled]);
    eprintln!("steps {:?}; {dead_db} white puffs, {fragments} fragments; the drop {:?}", run.steps, run.drop);
    // EnDekunuts_Die: the puff (EffectSsDeadDb) and the burst of 15 fragments (and the nut's own
    // 15 when it broke on the scrub).
    assert_eq!(dead_db, 1);
    assert!(fragments >= 15, "{fragments}");
    // The drop, if any, is one of table 3's (and was picked up), after func_8001F404.
    if let Some(d) = run.drop {
        let table = &a.item_drops.ids[3 * 16..3 * 16 + 16];
        assert!(table.contains(&(d as u8)) || (d == 0 && table.contains(&3)), "{d} from {table:?}");
    }
    assert!(w.player().actor.world_pos.distance(SCRUB_HOME) < 600.0);
}
