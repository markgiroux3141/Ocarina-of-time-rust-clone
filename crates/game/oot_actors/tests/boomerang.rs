//! GAME-10 milestone 4's start: Player's boomerang (`func_80835800` to `func_80835C08`) and
//! `En_Boom`. The C button raises it, letting go throws it on the throw's frame 6, it flies 20
//! frames, turns back, and is caught.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_CLEFT, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::player::{STATE1_24, STATE1_25, UpperAction};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Zora's Domain's shallows (the bottle run's start) on `zora-bottle`, the boomerang on C-Left.
fn domain_with_boomerang(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_ZORAS_DOMAIN_0").unwrap();
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("zora-bottle").unwrap();
    oot_game::item::item_give(&mut save, None, oot_game::item::ITEM_BOOMERANG);
    save.equip_item_on_c_left(oot_game::item::ITEM_BOOMERANG);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap();
    let mut prev = PadState::default();
    for _ in 0..20 {
        w.tick_with(scripted_input(prev, PadState::default()));
        prev = PadState::default();
    }
    w.place_player(Vec3::new(560.0, -20.0, -420.0), 0x4000);
    w
}

/// The En_Boom actors alive.
fn booms(w: &PlayState) -> Vec<oot_game::actor_ctx::ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.actor(h).is_some_and(|a| a.id == oot_actors::en_boom::ACTOR_EN_BOOM && !a.killed)).collect()
}

#[test]
fn the_boomerang_is_thrown_flies_back_and_is_caught() {
    let Some(a) = assets() else { return };
    let mut w = domain_with_boomerang(&a);
    let mut prev = PadState::default();
    let mut step = |w: &mut PlayState, pad: PadState| {
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
    };
    for _ in 0..10 {
        step(&mut w, PadState::default());
    }
    // C-Left held: the boomerang taken out (Player_InitBoomerangIA: PLAYER_STATE1_USING_BOOMERANG,
    // its upper action func_80835800), then raised while the button stays down (func_80834F2C,
    // func_80834D2C: func_80835884, unk_834 10, then func_808358F0's loop). A tap throws at once.
    let c = with(PadState::default(), BTN_CLEFT);
    let mut raised = false;
    for _ in 0..40 {
        step(&mut w, c);
        raised |= w.player().upper == UpperAction::BoomerangAim;
    }
    assert!(w.player().state1 & STATE1_24 != 0, "in hand");
    assert!(raised, "raised: {:?}", w.player().upper);
    assert!(booms(&w).is_empty());
    // Let go: thrown (func_808359FC); on its frame 6 En_Boom spawned and the flag set.
    let mut thrown_at = None;
    for f in 0..20 {
        step(&mut w, PadState::default());
        if thrown_at.is_none() && !booms(&w).is_empty() {
            thrown_at = Some(f);
            let p = w.player();
            assert!(p.state1 & STATE1_25 != 0);
            assert_eq!(p.unk_A73, 4);
            assert_eq!(p.boomerang_actor, Some(booms(&w)[0]));
        }
    }
    let thrown_at = thrown_at.expect("En_Boom spawned");
    eprintln!("thrown {thrown_at} frames after letting go");
    // Out 20 frames (returnTimer), then back to Player's focus: within 40 it's gone, the flag
    // cleared (EnBoom_Fly), and Player catches it (func_80835B60: link_boom_catch).
    let mut caught = false;
    for _ in 0..200 {
        step(&mut w, PadState::default());
        if booms(&w).is_empty() {
            assert!(w.player().state1 & STATE1_25 == 0);
            caught = true;
            break;
        }
    }
    assert!(caught, "the boomerang came back");
    let mut catch_seen = false;
    for _ in 0..4 {
        step(&mut w, PadState::default());
        catch_seen |= w.player().upper == UpperAction::BoomerangCatch;
    }
    assert!(catch_seen, "caught: {:?}", w.player().upper);
}
