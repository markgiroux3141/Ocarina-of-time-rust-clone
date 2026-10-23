//! `En_Rl` (GAME-11 milestone 2) against the C (`z_en_rl.c`): Rauru. Type 0 in the Chamber of the
//! Sages' layer 4 (`gChamberOfSagesLightMedallionCs`): shown with Link turned to him, his raise on
//! cue 3, and on cue 2 of channel 6 the Light Medallion (`Demo_Effect` 0xE over Link,
//! `Item_Give(ITEM_MEDALLION_LIGHT)`) once. Type 2 (the sages'): the fade in on cue 4, out on
//! another (his light ball, `Demo_6K` 5, once), the raise on cue 3, the debug reset at the
//! script's end.
//!
//! The Chamber is entered as the Ganondorf script's terminator does (`ENTR_CHAMBER_OF_THE_SAGES_0`
//! with `CS_INDEX_0`, layer 4), an adult. Rauru's update runs alone (`Actor_UpdateAll`'s call),
//! his cues put on the script's channels by hand (`play->csCtx.actorCues`). Expected values are
//! worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use glam::{IVec3, Vec3};
use oot_actors::PlayExt;
use oot_actors::demo_effect::{ACTOR_DEMO_EFFECT, DEMO_EFFECT_MEDAL_LIGHT};
use oot_actors::en_rl::*;
use oot_game::actor_ctx::ActorHandle;
use oot_game::cutscene::{CS_STATE_IDLE, CsCmdActorCue};
use oot_game::item::QUEST_MEDALLION_FOREST;
use oot_game::play::PlayState;
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// `QUEST_MEDALLION_LIGHT` (`z64save.h`: 5, after the Forest's 0).
const QUEST_MEDALLION_LIGHT: u32 = QUEST_MEDALLION_FOREST + 5;
/// `ACTOR_DEMO_6K` (`actor_table.h`: 0x00F5).
const ACTOR_DEMO_6K: i16 = 0x00F5;
/// `CS_STATE_RUN` (`cutscene.h`).
const CS_STATE_RUN: u8 = 2;

/// The Chamber of the Sages' layer 4 (`ENTR_CHAMBER_OF_THE_SAGES_0`, `CS_INDEX_0`), an adult from
/// `ocarina-of-time`; just after `Play_Init`.
fn chamber(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_CHAMBER_OF_THE_SAGES_0").expect("entrance");
    let mut save = SaveContext::new(e, true, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("ocarina-of-time").unwrap();
    save.adult = true;
    save.cutscene_index = 0xFFF0;
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    settle(&mut w, ACTOR_EN_RL);
    assert_eq!(w.save.scene_layer, 4);
    w
}

fn rl(w: &PlayState, h: ActorHandle) -> &EnRl {
    w.actors.downcast::<EnRl>(h).expect("an En_Rl")
}

/// The actor's update alone (`Actor_UpdateAll`'s call), as the updating actor.
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// A cue `action` on `channel` (or none), the script running.
fn cue(w: &mut PlayState, channel: usize, action: Option<u16>) {
    w.cs_ctx.state = CS_STATE_RUN;
    w.cs_ctx.npc_actions[channel] = action.map(|action| CsCmdActorCue { action, start_frame: 0, end_frame: 100, rot: [0; 3], start_pos: IVec3::ZERO, end_pos: IVec3::ZERO, normal: IVec3::ZERO });
}

fn count(w: &PlayState, id: i16, params: i16) -> usize {
    w.actors.all().into_iter().filter(|&h| w.actors.actor(h).is_some_and(|a| a.id == id && a.params == params)).count()
}

/// Updates until `action` is `to` (at most `limit`); how many it took.
fn until_action(w: &mut PlayState, h: ActorHandle, to: i32, limit: usize) -> usize {
    for k in 1..=limit {
        update(w, h);
        if rl(w, h).action == to {
            return k;
        }
    }
    panic!("not in action {to} after {limit} updates (in {})", rl(w, h).action);
}


/// Ticks until the room's actors of `id` are in and initialised (`Actor_UpdateAll` runs an init
/// once its object is loaded, without the update that frame); stops on that tick.
fn settle(w: &mut PlayState, id: i16) {
    for _ in 0..60 {
        let mine: Vec<&dyn oot_game::actor_ctx::ActorImpl> = w.actors.all().into_iter().filter_map(|h| w.actors.get(h)).filter(|a| a.base().id == id).collect();
        if !mine.is_empty() && mine.iter().all(|a| a.name() != "(not initialised)") {
            return;
        }
        w.tick_with(oot_game::play::scripted_input(eng_input::pad::PadState::default(), eng_input::pad::PadState::default()));
    }
    panic!("actor {id:#x} not initialised");
}

#[test]
fn type_0_gives_the_light_medallion_on_its_cue() {
    let Some(a) = assets() else { return };
    let mut w = chamber(&a);
    let h = w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnRl>(h).is_some()).expect("Rauru on layer 4");
    // EnRl_Init (params 0): func_80AE7544; action 0, drawn as nothing; at (-191, 6, 2), yaw 0x4000.
    assert_eq!(rl(&w, h).action, 0);
    assert_eq!(rl(&w, h).draw_config, DRAW_NOTHING);
    assert_eq!(rl(&w, h).actor.world_rot.y, 0x4000);
    assert!(rl(&w, h).skel.is(ANIM_IDLE));
    // func_80AE7668: drawn, action 1; Link turned to Rauru's yaw + 0x8000 (0xC000).
    update(&mut w, h);
    assert_eq!((rl(&w, h).action, rl(&w, h).draw_config), (1, DRAW_OPA));
    let p = w.player();
    assert_eq!((p.actor.world_rot.y, p.actor.shape_rot.y), (-0x4000, -0x4000));
    // func_80AE7698: no cue 3 on channel 0, still action 1.
    cue(&mut w, 0, Some(2));
    update(&mut w, h);
    assert_eq!(rl(&w, h).action, 1);
    // Cue 3: object_rl_Anim_00040C once, action 2; then, once it's done, _000830 looped, action 3.
    cue(&mut w, 0, Some(3));
    update(&mut w, h);
    assert_eq!(rl(&w, h).action, 2);
    assert!(rl(&w, h).skel.is(ANIM_RAISE));
    until_action(&mut w, h, 3, 200);
    assert!(rl(&w, h).skel.is(ANIM_RAISED));
    // func_80AE7590: channel 6 not cue 2: nothing.
    let before = count(&w, ACTOR_DEMO_EFFECT, DEMO_EFFECT_MEDAL_LIGHT as i16);
    cue(&mut w, 6, Some(1));
    update(&mut w, h);
    assert_eq!(count(&w, ACTOR_DEMO_EFFECT, DEMO_EFFECT_MEDAL_LIGHT as i16), before);
    assert!(!w.save.check_quest_item(QUEST_MEDALLION_LIGHT));
    // Cue 2 on channel 6 (layer 4, the Chamber, a script): Demo_Effect 0xE 80 above Link, the
    // Light Medallion given, once.
    let link = w.player().actor.world_pos;
    cue(&mut w, 6, Some(2));
    update(&mut w, h);
    assert!(rl(&w, h).light_medallion_given);
    assert!(w.save.check_quest_item(QUEST_MEDALLION_LIGHT));
    let medals: Vec<ActorHandle> = w.actors.all().into_iter().filter(|&m| w.actors.actor(m).is_some_and(|x| x.id == ACTOR_DEMO_EFFECT && x.params == DEMO_EFFECT_MEDAL_LIGHT as i16)).collect();
    assert_eq!(medals.len(), before + 1);
    let m = *medals.last().unwrap();
    assert_eq!(w.actors.actor(m).unwrap().home_pos, link + Vec3::new(0.0, 80.0, 0.0));
    update(&mut w, h);
    assert_eq!(count(&w, ACTOR_DEMO_EFFECT, DEMO_EFFECT_MEDAL_LIGHT as i16), before + 1, "once");
}

#[test]
fn type_2_fades_on_its_cues() {
    let Some(a) = assets() else { return };
    let mut w = chamber(&a);
    let h = w.actor_spawn(ACTOR_EN_RL, Vec3::new(-150.0, 6.0, 2.0), [0x2000, 0x4000, 0], 2).expect("En_Rl 2");
    // func_80AE7878: action 4, shadow alpha 0, hidden; the placement's tilt kept.
    assert_eq!((rl(&w, h).action, rl(&w, h).draw_config, rl(&w, h).actor.shape_shadow_alpha), (4, DRAW_NOTHING, 0));
    assert_eq!(rl(&w, h).actor.shape_rot.x, 0x2000);
    // func_80AE7954: no cue 4, hidden.
    cue(&mut w, 0, Some(1));
    update(&mut w, h);
    assert_eq!(rl(&w, h).action, 4);
    // Cue 4: action 5, translucent, alpha 0.
    cue(&mut w, 0, Some(4));
    update(&mut w, h);
    assert_eq!((rl(&w, h).action, rl(&w, h).draw_config, rl(&w, h).alpha), (5, DRAW_XLU, 0));
    // func_80AE79A4 under cue 4: unk_19C += 1, alpha (unk_19C / 10) * 255 truncated: 25, 51, 76.
    let mut alphas = Vec::new();
    for _ in 0..3 {
        update(&mut w, h);
        alphas.push(rl(&w, h).alpha);
    }
    assert_eq!(alphas, vec![25, 51, 76]);
    assert_eq!(rl(&w, h).actor.shape_shadow_alpha, 76);
    // At 10: solid, action 7.
    let k = until_action(&mut w, h, 7, 20);
    assert_eq!(k, 7);
    assert_eq!((rl(&w, h).draw_config, rl(&w, h).alpha, rl(&w, h).actor.shape_shadow_alpha), (DRAW_OPA, 255, 255));
    // func_80AE7AF8, another cue: fading out from 10; the light ball (Demo_6K 5) the first time.
    assert_eq!(count(&w, ACTOR_DEMO_6K, 5), 0);
    cue(&mut w, 0, Some(5));
    update(&mut w, h);
    assert_eq!((rl(&w, h).action, rl(&w, h).draw_config, rl(&w, h).alpha), (5, DRAW_XLU, 255));
    assert!(rl(&w, h).light_ball_spawned);
    assert_eq!(count(&w, ACTOR_DEMO_6K, 5), 1);
    let ball = w.actors.all().into_iter().find(|&b| w.actors.actor(b).is_some_and(|x| x.id == ACTOR_DEMO_6K)).unwrap();
    assert_eq!(w.actors.actor(ball).unwrap().home_pos, Vec3::new(-150.0, 6.0 + 22.0, 2.0));
    assert_eq!(w.actors.actor(ball).unwrap().parent, Some(h));
    // Out over 10: 229 first, hidden (action 4) at 0.
    update(&mut w, h);
    assert_eq!(rl(&w, h).alpha, 229);
    let k = until_action(&mut w, h, 4, 20);
    assert_eq!(k, 9);
    assert_eq!((rl(&w, h).draw_config, rl(&w, h).alpha), (DRAW_NOTHING, 0));
    // In again and out again: no second ball.
    cue(&mut w, 0, Some(4));
    until_action(&mut w, h, 7, 20);
    cue(&mut w, 0, Some(5));
    update(&mut w, h);
    assert_eq!(count(&w, ACTOR_DEMO_6K, 5), 1);
    // Solid again, then cue 3: object_rl_Anim_00040C (action 6), then _000830 (action 7).
    cue(&mut w, 0, Some(4));
    until_action(&mut w, h, 7, 20);
    cue(&mut w, 0, Some(3));
    update(&mut w, h);
    assert_eq!(rl(&w, h).action, 6);
    assert!(rl(&w, h).skel.is(ANIM_RAISE));
    until_action(&mut w, h, 7, 200);
    assert!(rl(&w, h).skel.is(ANIM_RAISED));
    // func_80AE73D8 (DEBUG_FEATURES): the script over: func_80AE7358, hidden, the ball forgotten.
    w.cs_ctx.state = CS_STATE_IDLE;
    update(&mut w, h);
    assert_eq!((rl(&w, h).action, rl(&w, h).draw_config, rl(&w, h).alpha), (4, DRAW_NOTHING, 0));
    assert!(!rl(&w, h).light_ball_spawned);
    assert!(rl(&w, h).skel.is(ANIM_IDLE));
}
