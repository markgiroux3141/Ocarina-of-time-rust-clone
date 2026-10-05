//! GAME-05 milestone 3a's exit (`Route::Combat`, the `combat` script's run, also a golden
//! trace): inside the Deku Tree, Link kills a withered Deku Baba and picks up its Deku Stick,
//! blocks a Keese's dive with the shield and kills the Keese, with the battle camera
//! (`Camera_Battle1`) on while he's locked on; the kills show their effects and leave their
//! drops.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use oot_actors::PlayExt;
use oot_actors::player::Action;
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::camera::CAM_MODE_Z_TARGET_UNFRIENDLY;
use oot_game::effect::{EFFECT_SS_DUST, EFFECT_SS_HAHEN, EFFECT_SS_HITMARK};
use oot_game::play::scripted_input;
use oot_game::play_scene::GameAssets;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

#[test]
fn exit_link_fights_a_withered_deku_baba_and_a_keese_and_blocks_with_the_shield() {
    let Some(a) = assets() else { return };
    let route = Route::Combat;
    let e = a.scenes.entrance_index(route.entrance()).expect("entrance");
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), route.save(e)).expect("Play_Init");
    let (pos, yaw) = route.start().unwrap();
    w.place_player(pos, yaw);
    let sticks = w.save.ammo(oot_game::item::ITEM_DEKU_STICK);
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    // What the run shows along the way: the frames on the battle camera (NORMAL0's BATTLE,
    // Camera_Battle1), the effects spawned (each slot's life restarting), and Link's state at the
    // block.
    let (mut battle_frames, mut hit_marks, mut fragments, mut dust) = (0, 0, 0, 0);
    let mut lives: Vec<i16> = w.effect_ss.table.iter().map(|e| e.life).collect();
    let mut health = w.save.health;
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
        let c = &w.game_camera;
        if c.mode == CAM_MODE_Z_TARGET_UNFRIENDLY && w.data.camera.mode(c.setting, c.mode).is_some_and(|m| m.func == "CAM_FUNC_BATT1") {
            battle_frames += 1;
        }
        for (i, e) in w.effect_ss.table.iter().enumerate() {
            if e.life > lives[i] || (lives[i] < 0 && e.life >= 0) {
                match e.ty {
                    EFFECT_SS_HITMARK => hit_marks += 1,
                    EFFECT_SS_HAHEN => fragments += 1,
                    EFFECT_SS_DUST => dust += 1,
                    _ => {}
                }
            }
            lives[i] = e.life;
        }
        if let Some(s) = run.take_done()
            && s == Step::Blocked
        {
            // The dive on the shield (func_808382DC's AC_BOUNCED path): no damage, the upper
            // body's recoil (the shield was up on the lock-on, not in the guard: actionVar1 0).
            let pl = w.player();
            assert_eq!(pl.action, Action::GuardHit);
            assert_eq!(pl.action_var1, 0);
            assert_eq!(w.save.health, health);
        }
        health = w.save.health;
    }
    assert!(run.failure.is_none(), "{:?}", run.failure);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::KarebabaKilled, Step::StickTaken, Step::Blocked, Step::KeeseKilled]);
    eprintln!("steps {:?}; battle camera {battle_frames} frames; {hit_marks} hit marks, {fragments} fragments, {dust} dust; the Keese's drop {:?}", run.steps, run.drop);
    // Locked on to both enemies: the battle camera ran.
    assert!(battle_frames > 30, "{battle_frames}");
    // The sword's hits flash (CollisionCheck's EffectSsHitMark); the withered Deku Baba throws
    // fragments as it springs and dies (EffectSsHahen_SpawnBurst) and dust along its stem
    // (func_800286CC).
    assert!(hit_marks >= 2, "{hit_marks}");
    assert!(fragments > 15, "{fragments}");
    assert!(dust >= 5, "{dust}");
    // Its Deku Stick taken (GI_DEKU_STICKS_1).
    assert_eq!(w.save.ammo(oot_game::item::ITEM_DEKU_STICK), sticks + 1);
    // The Keese's drop, if any, is one of table 14's (and was picked up), after func_8001F404: a
    // recovery heart (ITEM00_RECOVERY_HEART, 3) at full health is a green rupee (0).
    if let Some(d) = run.drop {
        let table = &a.item_drops.ids[14 * 16..14 * 16 + 16];
        assert!(table.contains(&(d as u8)) || (d == 0 && table.contains(&3)), "{d} from {table:?}");
    }
}
