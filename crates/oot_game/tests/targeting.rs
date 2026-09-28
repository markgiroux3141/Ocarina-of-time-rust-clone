//! Milestone 5a checks: Z-targeting (`func_80836BEC`), parallel mode, the lock-on and parallel
//! movement actions, side hops and the backflip. Expected values come from `z_player.c` /
//! `z_actor.c` (quoted per test), not from the port.

mod common;

use common::*;
use glam::Vec3;
use oot_game::input::{BTN_A, BTN_Z, PadState};
use oot_game::player::{STATE1_17, STATE1_4, STATE2_19};
use oot_game::target::TargetActor;
use oot_game::world::World;

/// Adult Link at the course spawn (0, 0, 0) facing -z, with a dummy target `dist` ahead.
fn target_world(dist: f32) -> Option<World> {
    let mut w = world()?;
    w.targets.push(TargetActor::dummy(Vec3::new(0.0, 0.0, -dist)));
    Some(w)
}

fn z(p: PadState) -> PadState {
    with(p, BTN_Z)
}

fn yaw_to_target(w: &World) -> i16 {
    oot_game::target::yaw_to(w.player.actor.world_pos, w.targets[0].focus)
}

#[test]
fn z_with_nothing_to_target_is_parallel_mode() {
    let Some(mut w) = world() else { return };
    let yaw0 = w.player.actor.shape_rot.y;
    let f = run(&mut w, &[stick(0, 0), z(stick(0, 0)), z(stick(0, 0))]);
    // func_808355DC: PLAYER_STATE1_17 and targetYaw = the facing; standing → func_80839F30.
    assert!(w.player.state1 & STATE1_17 != 0);
    assert_eq!(w.player.target_yaw, yaw0);
    assert_eq!(f[2].action, "ParallelIdle");
    // Stick left (x -80) while holding Z: walk sideways (func_8083CB94 → func_80840DE4), facing
    // kept on targetYaw.
    let f = run(&mut w, &repeat(z(stick(-80, 0)), 20));
    assert!(f.iter().any(|x| x.action == "ParallelWalk" || x.action == "Sidestep"), "{:?}", f.iter().map(|x| &x.action).collect::<Vec<_>>());
    assert_eq!(w.player.actor.shape_rot.y, yaw0, "facing held");
    assert!(w.player.actor.world_pos.x < -20.0, "moved left: {:?}", w.player.actor.world_pos);
    // Stick back: walk backwards (func_8083CB2C → func_808414F8), still facing -z.
    let mut w2 = world().unwrap();
    let f = run(&mut w2, &[stick(0, 0), z(stick(0, 0))]);
    let _ = f;
    let f = run(&mut w2, &repeat(z(stick(0, -80)), 20));
    assert!(f.iter().any(|x| x.action == "ParallelBackwalk"), "{:?}", f.iter().map(|x| &x.action).collect::<Vec<_>>());
    assert_eq!(w2.player.actor.shape_rot.y, yaw0);
    assert!(w2.player.actor.world_pos.z > 20.0, "moved backwards (+z): {:?}", w2.player.actor.world_pos);
    // Releasing Z ends parallel mode (unk_66C → 0, func_8008EE08).
    run(&mut w2, &repeat(stick(0, 0), 3));
    assert!(w2.player.state1 & STATE1_17 == 0);
}

#[test]
fn z_locks_on_and_keeps_the_target_until_pressed_again() {
    let Some(mut w) = target_world(150.0) else { return };
    // Frame 1: the target context finds the dummy (arrowPointedActor). Frame 2: Z.
    let f = run(&mut w, &[stick(0, 0), z(stick(0, 0)), stick(0, 0)]);
    assert_eq!(w.player.unk_664, Some(0));
    assert!(w.player.state1 & STATE1_4 != 0, "hostile target → PLAYER_STATE1_4");
    // func_8083CEAC: func_80840450 with PLAYER_ANIMGROUP_7.
    assert_eq!(f[1].action, "TargetIdle");
    let enter = w.data.anim_name(w.data.player_anim(7, 0)).to_string();
    assert_eq!(f[1].anim, enter);
    // "Switch" Z-targeting (zTargetSetting 0): released Z keeps the lock (PLAYER_STATE2_13).
    run(&mut w, &repeat(stick(0, 0), 20));
    assert_eq!(w.player.unk_664, Some(0));
    // Z again with no other candidate drops it.
    run(&mut w, &[z(stick(0, 0)), stick(0, 0), stick(0, 0)]);
    assert_eq!(w.player.unk_664, None);
    assert!(w.player.state1 & STATE1_4 == 0);
}

#[test]
fn locked_on_player_turns_to_face_the_target() {
    // Target off to the side (still inside 0x2AAA of the facing and 350 units; on the -x side
    // ramp B would block the line of sight, BgCheck_CameraLineTest1 in func_800328D4).
    let Some(mut w) = world() else { return };
    w.targets.push(TargetActor::dummy(Vec3::new(120.0, 0.0, -200.0)));
    run(&mut w, &[stick(0, 0), z(stick(0, 0))]);
    assert_eq!(w.player.unk_664, Some(0));
    run(&mut w, &repeat(stick(0, 0), 40));
    // func_80837268: once the reticle has locked (unk_4B != 0) the idle yaw is the yaw to the
    // target's focus, and func_80840450 turns towards it.
    let d = w.player.actor.shape_rot.y.wrapping_sub(yaw_to_target(&w)) as i32;
    assert!(d.abs() < 0x200, "facing {:#x} vs {:#x}", w.player.actor.shape_rot.y, yaw_to_target(&w));
}

#[test]
fn stick_sideways_sidesteps_around_the_target() {
    let Some(mut w) = target_world(200.0) else { return };
    run(&mut w, &[stick(0, 0), z(stick(0, 0))]);
    run(&mut w, &repeat(stick(0, 0), 10));
    let f = run(&mut w, &repeat(stick(-80, 0), 30));
    // func_8083FC68: speed ≤ (t²·50 + 6) and ≤ ((1 - t)·10 + 6.8) sideways → 0 → func_8083CC9C.
    assert!(f.iter().any(|x| x.action == "Sidestep"), "{:?}", f.iter().map(|x| &x.action).collect::<Vec<_>>());
    // Still facing the target while circling it.
    let d = w.player.actor.shape_rot.y.wrapping_sub(yaw_to_target(&w)) as i32;
    assert!(d.abs() < 0x800, "facing {:#x} vs {:#x}", w.player.actor.shape_rot.y, yaw_to_target(&w));
    assert!(w.player.actor.world_pos.x < -40.0, "moved: {:?}", w.player.actor.world_pos);
}

#[test]
fn stick_back_walks_backwards_facing_the_target() {
    let Some(mut w) = target_world(200.0) else { return };
    run(&mut w, &[stick(0, 0), z(stick(0, 0))]);
    run(&mut w, &repeat(stick(0, 0), 10));
    let f = run(&mut w, &repeat(stick(0, -80), 20));
    // func_8083FC68 only returns -1 (func_8083CBF0's step back) above (1 - t)·10 + 6.8 = 6.8 for
    // straight back, and returns 1 (run) above 6; the stick speed is clamped to unk_880 = 6.0, so
    // all lock-on movement goes through the sidestep action (func_8083CC9C, speed > 4).
    assert!(f.iter().all(|x| x.action != "TargetBackwalk"));
    assert!(f.iter().any(|x| x.action == "Sidestep"));
    assert!(w.player.actor.world_pos.z > 20.0, "moved away: {:?}", w.player.actor.world_pos);
    let d = w.player.actor.shape_rot.y.wrapping_sub(yaw_to_target(&w)) as i32;
    assert!(d.abs() < 0x200, "still facing the target");
}

#[test]
fn a_with_the_stick_sideways_side_hops() {
    let Some(mut w) = target_world(200.0) else { return };
    run(&mut w, &[stick(0, 0), z(stick(0, 0))]);
    run(&mut w, &repeat(stick(0, 0), 10));
    let yaw = w.player.actor.shape_rot.y;
    let mut s = vec![stick(-80, 0), with(stick(-80, 0), BTN_A)];
    s.extend(repeat(stick(0, 0), 30));
    let f = run(&mut w, &s);
    let h = f.iter().position(|x| x.action == "Midair").expect("hopped");
    // func_8083BCD0(1): D_80853D4C[1][0], vy 3.5, speed 8.5, currentYaw = facing + 0x4000.
    assert_eq!(f[h].anim, "link_fighter_Lside_jump");
    assert!((f[h].vy - 3.5).abs() < 1e-4, "vy {}", f[h].vy);
    assert!((f[h].speed - 8.5).abs() < 0.2, "speed {}", f[h].speed);
    assert_eq!(f[h].yaw, yaw.wrapping_add(0x4000));
    // Landing locked on: D_80853D4C[1][2].
    let l = f[h..].iter().position(|x| x.action != "Midair").map(|k| k + h).expect("landed");
    assert_eq!(f[l].anim, "link_fighter_Lside_jump_endL");
}

#[test]
fn a_with_the_stick_back_backflips() {
    let Some(mut w) = target_world(200.0) else { return };
    run(&mut w, &[stick(0, 0), z(stick(0, 0))]);
    run(&mut w, &repeat(stick(0, 0), 10));
    let yaw = w.player.actor.shape_rot.y;
    let mut s = vec![stick(0, -80), with(stick(0, -80), BTN_A)];
    s.extend(repeat(stick(0, 0), 30));
    let f = run(&mut w, &s);
    let h = f.iter().position(|x| x.action == "Midair").expect("flipped");
    // func_8083BCD0(2): D_80853D4C[2][0], vy 5.8, speed 6, currentYaw = facing + 0x8000.
    assert_eq!(f[h].anim, "link_fighter_backturn_jump");
    assert!((f[h].vy - 5.8).abs() < 1e-4, "vy {}", f[h].vy);
    assert_eq!(f[h].yaw, yaw.wrapping_add(i16::MIN));
    assert!(w.player.state2 & STATE2_19 == 0, "flag cleared after landing");
    let l = f[h..].iter().position(|x| x.action != "Midair").map(|k| k + h).expect("landed");
    assert_eq!(f[l].anim, "link_fighter_backturn_jump_endR");
}

#[test]
fn target_out_of_leash_range_is_lost() {
    // targetMode 3: TARGET_RANGE(350, 525) → rangeSq 350², leashScale 350/525. func_8002F0C8
    // loses the target once leashScale · dist² ≥ rangeSq, i.e. dist ≥ sqrt(350 · 525) ≈ 428.7,
    // and only while unk_66C < 6 (the first frames after locking can't lose it).
    let Some(d) = data() else { return };
    assert_eq!(d.target_ranges[3], (350.0 * 350.0, 350.0 / 525.0));
    let limit = (350.0f32 * 525.0).sqrt();
    let Some(mut w) = target_world(300.0) else { return };
    run(&mut w, &[stick(0, 0), z(stick(0, 0))]);
    assert_eq!(w.player.unk_664, Some(0));
    run(&mut w, &repeat(stick(0, 0), 15));
    assert_eq!(w.player.unk_664, Some(0));
    // Carry the target away (as if it walked off), 10 units a frame.
    let mut seen = Vec::new();
    for k in 0..40 {
        // Player sees the distances Actor_UpdateAll computed at the end of the last frame.
        seen.push(w.targets[0].xyz_dist_to_player_sq.sqrt());
        w.targets[0].pos.z = -300.0 - 10.0 * k as f32;
        w.targets[0].focus = w.targets[0].pos + Vec3::Y * 40.0;
        run(&mut w, &[stick(0, 0)]);
        if w.player.unk_664.is_none() {
            let (now, before) = (seen[seen.len() - 1], seen[seen.len() - 2]);
            assert!(now >= limit && before < limit, "lost seeing {now} (before {before}), limit {limit}");
            return;
        }
    }
    panic!("never lost the target");
}

