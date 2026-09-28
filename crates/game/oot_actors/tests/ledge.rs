//! Milestone 4 checks: ledge grabs, hanging, climbing up and climbing onto ledges
//! (`func_8083A6AC`, `func_8083A5C4`, `func_8084BBE4`, `func_8084BDFC`, `func_80838A14`,
//! `func_80845668` and the mid-air grab in `func_8084411C`). Expected values come from the C
//! and `sAgeProperties` (adult: unk_1C 41, unk_18 59, unk_14 79.4, unk_0C 111, unk_34 70).

mod common;

use common::*;
use oot_actors::PlayExt;
use oot_game::play::PlayState;
use eng_collision::bgcheck::CollisionContext;
use eng_collision::collision::CollisionBuilder;
use eng_input::pad::BTN_A;
use glam::Vec3;

/// A floor at y = 0 for z in [0, 400] and a block of height `h` for z in [-400, 0], its face at
/// z = 0 facing +z. Adult Link starts at z = 150 facing the block (-z).
fn block_world(h: f32) -> Option<PlayState> {
    let d = data()?;
    let mut b = CollisionBuilder::new();
    let s = b.surface(0, 0);
    b.quad(Vec3::new(-400.0, 0.0, 400.0), Vec3::new(400.0, 0.0, 400.0), Vec3::new(400.0, 0.0, 0.0), Vec3::new(-400.0, 0.0, 0.0), s);
    b.quad(Vec3::new(-400.0, h, 0.0), Vec3::new(400.0, h, 0.0), Vec3::new(400.0, h, -400.0), Vec3::new(-400.0, h, -400.0), s);
    b.quad(Vec3::new(-400.0, 0.0, 0.0), Vec3::new(400.0, 0.0, 0.0), Vec3::new(400.0, h, 0.0), Vec3::new(-400.0, h, 0.0), s);
    Some(new_world(d, CollisionContext::new(b.finish()), true, Vec3::new(0.0, 0.0, 150.0), -0x8000))
}

fn first(f: &[Frame], action: &str) -> Option<usize> {
    f.iter().position(|x| x.action == action)
}

#[test]
fn class_1_steps_are_hopped() {
    let Some(mut w) = block_world(30.0) else { return };
    let f = run(&mut w, &repeat(stick(0, 80), 60));
    // func_80838A14: unk_88C == 1 (18 <= h < unk_1C = 41) for 3 frames → func_808389E8(link_normal_jump,
    // wallHeight * 0.08 + 5.5), linearVelocity 2.5.
    let j = first(&f, "Midair").expect("hopped");
    assert_eq!(f[j].anim, "link_normal_jump");
    assert!((f[j].vy - (30.0 * 0.08 + 5.5)).abs() < 1e-4, "vy {}", f[j].vy);
    assert!(f.last().unwrap().pos.y == 30.0, "ended on top: {:?}", f.last().unwrap().pos);
}

#[test]
fn class_2_ledges_step_up_100() {
    let Some(mut w) = block_world(50.0) else { return };
    let f = run(&mut w, &repeat(stick(0, 80), 80));
    let c = first(&f, "ClimbLedge").expect("climbed");
    assert_eq!(f[c].anim, "link_normal_100step_up");
    // Position jumps to the top at once; the model is pulled down by (h - 41·unk_08)·100 and eased
    // back at 150 per frame (func_80845668 starts easing on the next frame for the 100 step).
    assert_eq!(f[c].pos.y, 50.0);
    assert!((f[c].y_offset - (-(50.0 - 41.0) * 100.0)).abs() < 1e-3, "y offset {}", f[c].y_offset);
    assert!((f[c + 1].y_offset - (-(50.0 - 41.0) * 100.0 + 150.0)).abs() < 1e-3, "y offset {}", f[c + 1].y_offset);
    // x/z: moved onto the wall plane + 0.5 (wallDistance + 0.5 along the normal).
    assert!(f[c].pos.z <= 0.0 && f[c].pos.z > -1.0, "z {}", f[c].pos.z);
    // Facing the wall: wallYaw + 0x8000.
    assert_eq!(f[c].facing, -0x8000);
    let end = f[c..].iter().position(|x| x.action != "ClimbLedge").map(|k| k + c).expect("finished");
    assert_eq!(f[end].y_offset, 0.0);
    assert_eq!(f[end].pos.y, 50.0);
}

#[test]
fn class_3_ledges_step_up_150() {
    let Some(mut w) = block_world(70.0) else { return };
    let f = run(&mut w, &repeat(stick(0, 80), 80));
    let c = first(&f, "ClimbLedge").expect("climbed");
    assert_eq!(f[c].anim, "link_normal_150step_up");
    assert_eq!(f[c].pos.y, 70.0);
    // The 150 step only eases the offset after frame 5: it starts at -(70 - 59)·100.
    assert!((f[c].y_offset - (-(70.0 - 59.0) * 100.0)).abs() < 1e-3, "y offset {}", f[c].y_offset);
}

#[test]
fn class_4_ledges_jump_grab_and_climb() {
    let Some(mut w) = block_world(100.0) else { return };
    let f = run(&mut w, &repeat(stick(0, 80), 120));
    let c = first(&f, "ClimbLedge").expect("started the tall-ledge jump");
    assert_eq!(f[c].anim, "link_normal_250jump_start");
    let j = first(&f[c..], "Midair").map(|k| k + c).expect("jumped");
    // Frame 8 of 250jump_start: func_80838940 with min(wallHeight, unk_0C) * 0.072 = 7.2.
    assert!((f[j].vy - 100.0 * 0.072).abs() < 1e-3, "vy {}", f[j].vy);
    let h = first(&f[j..], "Hang").map(|k| k + j).expect("grabbed the ledge in the air");
    assert_eq!(f[h].anim, "link_normal_jump_climb_hold_free");
    // func_8084411C: pos.y += wallHeight → on the ledge top; facing the wall.
    assert_eq!(f[h].pos.y, 100.0, "{:?}", f[h].pos);
    assert_eq!(f[h].facing, -0x8000);
    // The stick is held, so the hang climbs straight up (group 41 after a mid-air grab).
    let u = first(&f[h..], "ClimbUp").map(|k| k + h).expect("climbed up");
    assert_eq!(f[u].anim, "link_normal_jump_climb_up_free");
    let s = f[u..].iter().position(|x| x.action != "ClimbUp").map(|k| k + u).expect("finished climbing");
    assert_eq!(f[s].pos.y, 100.0);
    assert!(f[s].pos.z < 0.0, "stands on the block: {:?}", f[s].pos);
}

#[test]
fn walking_slowly_off_a_high_ledge_grabs_it() {
    // The plateau (y = 150) ends at x = -500; walk east off it slowly (speed ≤ 3: no auto-jump).
    let Some(mut w) = world_at(Vec3::new(-540.0, 150.0, -700.0), 0x4000) else { return };
    let mut f = run(&mut w, &repeat(stick(0, 30), 40));
    let h = first(&f, "Hang").expect("grabbed the ledge");
    // func_8083A6AC → func_8083A5C4(link_normal_fall): 1 unit past the face onto the top,
    // facing the face's normal (+x, 0x4000). The frame's fall is undone by the next bg check.
    assert_eq!(f[h].anim, "link_normal_fall");
    assert!((f[h].pos.x - -501.0).abs() < 1e-3, "{:?}", f[h].pos);
    assert_eq!(f[h].facing, 0x4000);
    assert_eq!(f[h + 1].pos.y, 150.0);
    // A stick of 30 is below the 55 of unk_847, so Link keeps hanging.
    assert!(f[h..].iter().all(|x| x.action == "Hang"));
    // A full stick climbs up (group 38 after a walk-off grab) and ends on the plateau.
    f = run(&mut w, &repeat(stick(0, 80), 80));
    let h = 0;
    let u = first(&f[h..], "ClimbUp").map(|k| k + h).expect("climbed");
    assert_eq!(f[u].anim, "link_normal_fall_up_free");
    let s = f[u..].iter().position(|x| x.action != "ClimbUp").map(|k| k + u).expect("finished");
    assert_eq!(f[s].pos.y, 150.0);
    assert!(f[s].pos.x < -500.0, "back on the plateau: {:?}", f[s].pos);
}

#[test]
fn a_lets_go_of_the_ledge() {
    let Some(mut w) = world_at(Vec3::new(-540.0, 150.0, -700.0), 0x4000) else { return };
    // Walk off slowly until the grab, then release the stick so the hang doesn't climb.
    let mut s = Vec::new();
    let mut f = Vec::new();
    for _ in 0..80 {
        f.extend(run(&mut w, &[stick(0, 30)]));
        if w.player().action == oot_actors::player::Action::Hang {
            break;
        }
    }
    assert!(first(&f, "Hang").is_some(), "grabbed");
    s.extend(repeat(stick(0, 0), 20));
    s.push(with(stick(0, 0), BTN_A));
    s.extend(repeat(stick(0, 0), 40));
    let f = run(&mut w, &s);
    assert_eq!(f[19].action, "Hang", "still hanging before A");
    let m = first(&f, "Midair").expect("let go");
    assert_eq!(f[m].anim, "link_normal_landing_wait");
    // func_80837B60 bakes the hanging body's root offset in, so Link drops from below the edge.
    assert!(f[m].pos.y < 150.0 && f[m].pos.x > -500.0, "{:?}", f[m].pos);
    assert!(f.iter().any(|x| x.grounded && x.pos.y == 0.0), "landed below");
}

#[test]
fn walking_off_a_low_ledge_falls() {
    // Off the 40-high block (top z in [150, 350]): drop 40 <= unk_34 (70) → no grab check.
    let Some(mut w) = world_at(Vec3::new(500.0, 40.0, 250.0), 0) else { return };
    let f = run(&mut w, &repeat(stick(0, 30), 120));
    let j = first(&f, "Midair").expect("walked off");
    assert_eq!(f[j].anim, "link_normal_landing_wait");
    assert!(first(&f, "Hang").is_none());
    assert!(f.iter().any(|x| x.grounded && x.pos.y == 0.0));
}
