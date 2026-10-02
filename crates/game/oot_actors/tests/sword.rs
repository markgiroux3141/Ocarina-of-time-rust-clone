//! Milestone 5b checks: drawing the sword on B (the item change on `skelAnime2`), slashes
//! (`Player_ActionHandler_7` → `func_80837948` → `Player_Action_808502D0`), combos, and putting the sword away.
//! Expected values come from `z_player.c`'s tables (`sItemChangeInfo`, `sItemChangeTypes`, `D_80854190`,
//! `D_80854480`) and code, not from the port.

mod common;

use common::*;
use oot_actors::PlayExt;
use eng_input::pad::{BTN_A, BTN_B, BTN_Z, PadState};
use glam::Vec3;
use oot_actors::player::{Action, UpperAction};

fn b(p: PadState) -> PadState {
    with(p, BTN_B)
}

#[test]
fn b_draws_the_sword_and_slashes() {
    let Some(mut w) = world() else { return };
    let d = w.data.clone();
    run(&mut w, &repeat(stick(0, 0), 2));
    let f = run(&mut w, &[b(stick(0, 0))]);
    // Player_UseItem(PLAYER_IA_SWORD_MASTER): anim type 0 → 1 is sItemChangeTypes[0][1] = -5, so the
    // change plays sItemChangeInfo[5] (link_normal_fighter2free) backwards at -1.2 × 2 on skelAnime2.
    assert_eq!(d.items.change_matrix[0][1], -5);
    assert_eq!(w.player().upper, UpperAction::Change);
    assert_eq!(d.anim_name(w.player().skel2.animation), "link_normal_fighter2free");
    assert!((w.player().skel2.play_speed + 2.4).abs() < 1e-6);
    assert_eq!(f[0].action, "StandingStill");
    // The swap frame is sItemChangeInfo[5].changeFrame - 1 = 9 (playing backwards from the last frame at
    // 2.4 × 1.5 per game frame); the attack starts the same frame (sUseHeldItem set in Player_UpperAction_ChangeHeldItem).
    let last = d.anims[w.player().skel2.animation].last_frame();
    let mut frames_to_swap = 1;
    let mut cur = last - 3.6;
    while cur > 9.0 {
        cur -= 3.6;
        frames_to_swap += 1;
    }
    let f = run(&mut w, &repeat(stick(0, 0), 20));
    let a = f.iter().position(|x| x.action == "Attack").expect("slashed");
    assert_eq!(a + 1, frames_to_swap, "attack on the swap frame (last frame {last})");
    assert_eq!(w.player().held_item_ap, d.items.ap("SWORD_MASTER"));
    assert_eq!(w.player().model_group, d.items.model_group("SWORD_AND_SHIELD"));
    assert_eq!(w.player().model_anim_type, 1, "sword + shield animation type");
    // Stick neutral, not targeting: func_80837818 → PLAYER_MWA_RIGHT_SLASH_1H.
    assert_eq!(f[a].anim, "link_fighter_Lside_kiru");
    let att = &d.items.attacks[d.items.mwa("RIGHT_SLASH_1H")];
    // After the attack, the end animation (D_80854190.unk_04) into standing.
    let s = f[a..].iter().position(|x| x.action != "Attack").map(|k| k + a).expect("finished");
    assert_eq!(f[s].action, "StandingStill");
    assert_eq!(f[s].anim, d.anim_name(att.end));
}

#[test]
fn the_same_slash_three_times_is_a_combo() {
    let Some(mut w) = world() else { return };
    let d = w.data.clone();
    run(&mut w, &repeat(stick(0, 0), 2));
    let mut names = Vec::new();
    for _ in 0..3 {
        let mut s = vec![b(stick(0, 0))];
        s.extend(repeat(stick(0, 0), 14));
        let f = run(&mut w, &s);
        if let Some(a) = f.iter().position(|x| x.action == "Attack") {
            names.push(f[a].anim.clone());
        }
    }
    // func_80837948: unk_845 counts repeats of the same attack; the third adds 2
    // (RIGHT_SLASH_1H → RIGHT_COMBO_1H).
    assert_eq!(names.len(), 3, "{names:?}");
    assert_eq!(names[2], d.anim_name(d.items.attacks[d.items.mwa("RIGHT_COMBO_1H")].anim), "{names:?}");
    assert_eq!(names[0], names[1]);
}

#[test]
fn locked_on_stick_forward_stabs_with_a_lunge() {
    let Some(mut w) = world() else { return };
    let d = w.data.clone();
    w.spawn_target(Vec3::new(0.0, 0.0, -200.0));
    run(&mut w, &[stick(0, 0), with(stick(0, 0), BTN_Z)]);
    run(&mut w, &repeat(stick(0, 0), 10));
    // Draw the sword first (stick neutral → forward slash while targeting).
    let mut s = vec![b(stick(0, 0))];
    s.extend(repeat(stick(0, 0), 25));
    let f = run(&mut w, &s);
    let a = f.iter().position(|x| x.action == "Attack").expect("slashed");
    assert_eq!(f[a].anim, d.anim_name(d.items.attacks[d.items.mwa("FORWARD_SLASH_1H")].anim));
    // Now B with the stick forward: D_80854480[0] = STAB_1H with PLAYER_STATE2_30, which sets
    // linearVelocity 15 on the attack's frame 0.
    let mut s = vec![b(stick(0, 80))];
    s.extend(repeat(stick(0, 80), 3));
    let f = run(&mut w, &s);
    let a = f.iter().position(|x| x.action == "Attack").expect("stabbed");
    assert_eq!(f[a].anim, d.anim_name(d.items.attacks[d.items.mwa("STAB_1H")].anim));
    assert!(f[a..].iter().any(|x| x.speed > 9.0), "lunge: {:?}", f.iter().map(|x| x.speed).collect::<Vec<_>>());
}

#[test]
fn a_puts_the_sword_away() {
    let Some(mut w) = world() else { return };
    let d = w.data.clone();
    run(&mut w, &repeat(stick(0, 0), 2));
    let mut s = vec![b(stick(0, 0))];
    s.extend(repeat(stick(0, 0), 40));
    run(&mut w, &s);
    assert_eq!(w.player().held_item_ap, d.items.ap("SWORD_MASTER"));
    assert_eq!(w.player().action, Action::StandingStill);
    // Player_ActionHandler_Roll: A, not locked on, heldItemActionParam >= PLAYER_IA_SWORD_MASTER →
    // Player_UseItem(ITEM_NONE): sItemChangeTypes[1][0] = 5, forwards at 1.2 (no item → no × 2).
    // The press only queues the change (PLAYER_STATE1_START_CHANGING_HELD_ITEM, set in the interrupt loop after this
    // frame's Player_UpdateUpperBody); Player_UpperAction_Sword starts it on the next frame.
    run(&mut w, &[with(stick(0, 0), BTN_A)]);
    assert!(w.player().state1 & oot_actors::player::STATE1_8 != 0);
    run(&mut w, &[stick(0, 0)]);
    assert_eq!(w.player().upper, UpperAction::Change);
    assert!((w.player().skel2.play_speed - 1.2).abs() < 1e-6);
    run(&mut w, &repeat(stick(0, 0), 30));
    assert_eq!(w.player().held_item_ap, 0);
    assert_eq!(w.player().model_group, d.items.model_group("DEFAULT"));
    assert_eq!(w.player().model_anim_type, 0);
}
