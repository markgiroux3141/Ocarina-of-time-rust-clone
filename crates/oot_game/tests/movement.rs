//! Scripted-input checks of Player movement against values derived from the decomp:
//! `sBootData` REG values, `sAgeProperties`, and the formulas in the ported functions.
//! Needs `oot.toml` (ROM + decomp); tests are skipped without it.

mod common;

use common::*;
use glam::Vec3;
use oot_game::input::{BTN_A, PadState};
use oot_game::math::{UPDATE_SCALE, cos_s};

/// `R_RUN_SPEED_LIMIT / 100` for adult Link with Kokiri boots (sBootData[0][9] = 600).
const RUN_LIMIT: f32 = 6.0;
/// `REG(19) / 100` (sBootData[0][0] = 200): run acceleration per frame.
const RUN_ACCEL: f32 = 2.0;

fn find(frames: &[Frame], action: &str) -> Option<usize> {
    frames.iter().position(|f| f.action == action)
}

/// `func_80836FAC` start curve (arg4 = 0.018) for a stick magnitude, before the run limit.
fn start_curve(mag: f32) -> f32 {
    let m = mag - 20.0;
    if m < 0.0 {
        return 0.0;
    }
    let t = 1.0 - cos_s((m * 450.0) as i16);
    ((t * t) * 30.0 + 7.0) * 0.14
}

#[test]
fn regs_come_from_boot_data() {
    let Some(d) = data() else { return };
    let r = &d.regs[0];
    assert_eq!(r.reg(19), 200);
    assert_eq!(r.reg(27), 2000);
    assert_eq!(r.reg(43), 800);
    assert_eq!(r.reg(45), 600);
    assert_eq!(r.reg(48), 370);
    assert_eq!(r.reg(68), -100);
    assert_eq!((r.ireg(66), r.ireg(67), r.ireg(68), r.ireg(69)), (590, 750, 125, 200));
    assert_eq!(d.ages[0].wall_radius, 18.0);
    assert_eq!(d.ages[1].translation_scale, 11.0 / 17.0);
    // The shipped tables equal trunc(sin(i·π/2046)·32767) and round(atan(i/1024)·0x8000/π).
    let (sin_diff, atan_diff, max) = d.table_mismatches;
    println!("sintable entries differing from the formula: {sin_diff}, atan: {atan_diff}, max |d| {max}");
    assert_eq!((sin_diff, atan_diff, max), (0, 0, 0));
}

#[test]
fn full_stick_accelerates_to_the_run_limit() {
    let Some(mut w) = world_at(Vec3::new(-300.0, 0.0, 900.0), -0x8000) else { return };
    let f = run(&mut w, &repeat(stick(0, 80), 40));
    // Frame 1 switches StandingStill → Run (func_8083C8DC); speed then rises by REG(19)/100.
    assert_eq!(f[0].action, "Run");
    let speeds: Vec<f32> = f.iter().take(6).map(|x| x.speed).collect();
    assert_eq!(speeds, vec![0.0, RUN_ACCEL, 2.0 * RUN_ACCEL, RUN_LIMIT, RUN_LIMIT, RUN_LIMIT]);
    // The stick curve (47 × 0.14 = 6.58) is above the limit, so the limit wins.
    assert!(start_curve(60.0) > RUN_LIMIT);
    // Position integrates velocity × R_UPDATE_RATE / 2 (func_8002D7EC), one frame behind.
    for k in 6..40 {
        let d = f[k - 1].pos.z - f[k].pos.z;
        assert!((d - RUN_LIMIT * UPDATE_SCALE).abs() < 1e-3, "frame {k}: moved {d}");
    }
    let dist = f[0].pos.z - f[39].pos.z;
    let expected = (RUN_ACCEL + 2.0 * RUN_ACCEL + 36.0 * RUN_LIMIT) * UPDATE_SCALE;
    assert!((dist - expected).abs() < 1e-2, "40 frames covered {dist}, expected {expected}");
    assert!(f.iter().all(|x| x.grounded && x.pos.y == 0.0));
}

#[test]
fn partial_stick_walks_at_the_curve_speed() {
    let Some(mut w) = world_at(Vec3::new(-300.0, 0.0, 900.0), -0x8000) else { return };
    // Raw 30 → rel 23 (dead zone 7) → func_80836FAC's start curve.
    let f = run(&mut w, &repeat(stick(0, 30), 20));
    let target = start_curve(23.0);
    assert!((f[19].speed - target).abs() < 1e-5, "walk speed {} vs {target}", f[19].speed);
    assert!(target < 1.0 && target > 0.95);
}

#[test]
fn releasing_the_stick_brakes_then_stands() {
    let Some(mut w) = world_at(Vec3::new(-300.0, 0.0, 900.0), -0x8000) else { return };
    let mut s = repeat(stick(0, 80), 12);
    s.extend(repeat(stick(0, 0), 20));
    let f = run(&mut w, &s);
    // func_8083DF68 decelerates by 1.5 per frame, then func_8083C0B8 plays a walk-end.
    let tail: Vec<f32> = f[12..16].iter().map(|x| x.speed).collect();
    assert_eq!(tail, vec![4.5, 3.0, 1.5, 0.0]);
    assert_eq!(f[15].action, "StandingStill");
    assert!(f[15].anim.starts_with("link_normal_walk_end"), "{}", f[15].anim);
}

#[test]
fn a_while_running_rolls_for_the_decomp_duration() {
    let Some(mut w) = world_at(Vec3::new(-300.0, 0.0, 900.0), -0x8000) else { return };
    let mut s = repeat(stick(0, 80), 10);
    s.push(with(stick(0, 80), BTN_A));
    s.extend(repeat(stick(0, 80), 20));
    let f = run(&mut w, &s);
    let start = find(&f, "Roll").expect("rolled");
    assert_eq!(start, 10, "roll starts on the A frame");
    let len = f[start..].iter().take_while(|x| x.action == "Roll").count();
    // The roll ends once curFrame ≥ 20 (func_80844708); it plays at 1.25 × 1.5 frames per
    // game frame (func_8083BC04, LinkAnimation_Once).
    let expected = (20.0f32 / (1.25 * UPDATE_SCALE)).ceil() as usize;
    assert_eq!(len, expected, "roll lasted {len} frames");
    // Speed target is 1.5 × the run limit (func_80844708), reached at REG(19)/100 per frame.
    let top = f[start..start + len].iter().map(|x| x.speed).fold(0.0, f32::max);
    assert_eq!(top, RUN_LIMIT * 1.5);
    assert_eq!(f[start + 1].speed, RUN_LIMIT + RUN_ACCEL);
    let dist = f[start - 1].pos.z - f[start + len - 1].pos.z;
    println!("roll: {len} frames, {dist:.1} units, top speed {top}");
    assert!(f[start + len].action == "StandingStill" || f[start + len].action == "Run");
}

#[test]
fn a_rolls_only_with_the_stick_forward() {
    let Some(mut w) = world_at(Vec3::new(-300.0, 0.0, 900.0), -0x8000) else { return };
    // Idle + A: func_8083BC7C needs unk_84B == 0 (stick ≥ 55, pointing forward).
    let mut s = repeat(stick(0, 0), 3);
    s.push(with(stick(0, 0), BTN_A));
    s.extend(repeat(stick(0, 0), 3));
    // Stick fully sideways + A on the first frame: direction 1 or 3 → no roll.
    s.push(with(stick(80, 0), BTN_A));
    s.extend(repeat(stick(80, 0), 3));
    let f = run(&mut w, &s);
    assert!(find(&f, "Roll").is_none(), "{:?}", f.iter().map(|x| x.action.clone()).collect::<Vec<_>>());
}

#[test]
fn small_stick_turns_in_place() {
    let Some(mut w) = world_at(Vec3::new(-300.0, 0.0, 900.0), -0x8000) else { return };
    // Raw -26 → rel -19: magnitude < 20 gives no speed on the start curve, so Player turns
    // in place (func_8083CD54) at unk_87E = 1200 × 1.5 per frame.
    let f = run(&mut w, &repeat(stick(-26, 0), 20));
    assert_eq!(f[0].action, "Turn");
    assert!(f.iter().all(|x| x.speed == 0.0 && x.pos == f[0].pos));
    let done = f.iter().position(|x| x.action == "StandingStill").expect("turn finished");
    // Quarter turn (0x4000) at 1800 per frame.
    assert_eq!(done, (0x4000 as f32 / 1800.0).ceil() as usize);
}

#[test]
fn running_off_a_ledge_auto_jumps() {
    let Some(mut w) = world_at(Vec3::new(-700.0, 150.0, -700.0), 0x4000) else { return };
    let mut s = repeat(stick(0, 80), 30);
    s.extend(repeat(stick(0, 0), 40));
    let f = run(&mut w, &s);
    let j = find(&f, "Midair").expect("left the plateau");
    // func_8083A4A8: speed 6 > IREG(66)/100 = 5.9 → vy = IREG(67)/100 = 7.5, run jump.
    assert_eq!(f[j].vy, 7.5);
    assert_eq!(f[j].anim, "link_normal_run_jump");
    let peak = f.iter().map(|x| x.pos.y).fold(f32::MIN, f32::max);
    // Integrate the same way: v += gravity (REG(68)/100 = -1) then y += v × 1.5.
    let (mut y, mut v) = (f[j].pos.y, 7.5f32);
    let mut best = y;
    while v > 0.0 {
        v -= 1.0;
        y += v * UPDATE_SCALE;
        best = best.max(y);
    }
    assert!((peak - best).abs() < 1e-3, "peak {peak} vs {best}");
    let land = f[j..].iter().position(|x| x.grounded).map(|k| k + j).expect("landed");
    assert_eq!(f[land].pos.y, 0.0);
    assert_eq!(f[land].action, "StandingStill");
}

#[test]
fn walking_off_a_ledge_does_not_jump() {
    let Some(mut w) = world_at(Vec3::new(-540.0, 150.0, -700.0), 0x4000) else { return };
    let f = run(&mut w, &repeat(stick(0, 30), 80));
    // Speed ≤ 3 → no auto-jump. The drop (150) exceeds unk_34 (70), so func_8083A6AC grabs the
    // ledge (spike 04; spike 03 fell here because the grab wasn't ported). See tests/ledge.rs.
    assert!(f.iter().all(|x| !x.anim.contains("jump")));
    assert!(find(&f, "Hang").is_some(), "grabbed the ledge");
}

#[test]
fn landing_with_the_stick_forward_rolls() {
    let Some(mut w) = world_at(Vec3::new(-700.0, 150.0, -700.0), 0x4000) else { return };
    let f = run(&mut w, &repeat(stick(0, 80), 60));
    let j = find(&f, "Midair").unwrap();
    let land = f[j..].iter().position(|x| x.action != "Midair").map(|k| k + j).unwrap();
    // func_8084411C: 80 < fallDistance (150) < 800 with the stick forward → func_8083BC04.
    assert_eq!(f[land].action, "Roll");
}

#[test]
fn a_long_fall_staggers_on_landing() {
    let Some(mut w) = world_at(Vec3::new(600.0, 0.0, -300.0), -0x8000) else { return };
    let mut s = repeat(stick(0, 80), 14);
    s.extend(repeat(stick(0, 0), 60));
    let f = run(&mut w, &s);
    let land = f.iter().position(|x| x.grounded && x.pos.y == -450.0).expect("reached the pit floor");
    // fallDistance ≥ 400 → func_80843E64 returns 1 → endFrame 8, unk_850 = 10 stagger.
    assert!(w.player.notes.iter().any(|n| n.starts_with("fall damage -8")), "{:?}", w.player.notes);
    assert_eq!(f[land].action, "StandingStill");
    // Velocity is capped at minVelocityY = -20 (func_8083D6EC).
    assert!(f.iter().all(|x| x.vy >= -20.0));
}

#[test]
fn walls_stop_at_the_player_radius() {
    // The course's +z boundary wall at z = 1000 (300 high: no climb class). Spike 03 used the
    // 40-high block, which Link now hops onto (func_80838A14, class 1; see tests/ledge.rs).
    let Some(mut w) = world_at(Vec3::new(0.0, 0.0, 900.0), 0) else { return };
    let f = run(&mut w, &repeat(stick(0, 80), 40));
    // Player's wall radius is sAgeProperties.unk_38 = 18.
    assert_eq!(f[39].pos.z, 1000.0 - 18.0);
    assert!(f[39].speed <= 0.1 + 1e-6, "unk_880 drops to 0.1 running straight into a wall");
}

#[test]
fn walls_at_an_angle_slide() {
    let Some(mut w) = world_at(Vec3::new(-300.0, 0.0, 600.0), -0x8000) else { return };
    let f = run(&mut w, &repeat(stick(0, 80), 60));
    let last = f[59].clone();
    let first_contact = f.iter().position(|x| x.pos.x != -300.0).unwrap();
    assert!(last.pos.x < -340.0, "slid along the wall: {:?}", last.pos);
    // unk_880 = R_RUN_SPEED_LIMIT/100 × |yaw − wall normal| × 0.00008 (func_80847BA0).
    // Running along -z into a wall whose normal is 30° off gives ≈ 30° = 5461.
    let k = 5461.0 * 0.00008;
    assert!((last.speed - RUN_LIMIT * k).abs() < 0.06, "speed {} vs {}", last.speed, RUN_LIMIT * k);
    assert!(first_contact > 10);
}

#[test]
fn ramps_slow_the_run() {
    let Some(mut w) = world_at(Vec3::new(-700.0, 0.0, 100.0), -0x8000) else { return };
    let f = run(&mut w, &repeat(stick(0, 80), 60));
    let on: Vec<_> = f.iter().filter(|x| x.pos.y > 20.0 && x.pos.y < 100.0).collect();
    assert!(!on.is_empty());
    // func_80836FAC: start curve − 8·sin²(slope), slope = atan(150/400) along the move.
    let slope = (150.0f32 / 400.0).atan();
    let target = start_curve(60.0) - 8.0 * slope.sin().min(0.6).powi(2);
    for x in on {
        assert!((x.speed - target).abs() < 0.06, "speed {} on the ramp, expected ≈{target}", x.speed);
    }
    assert!(f[59].pos.y > 100.0, "climbed: {:?}", f[59].pos);
}

#[test]
fn stairs_of_15_are_walkable() {
    let Some(mut w) = world_at(Vec3::new(150.0, 0.0, -300.0), -0x8000) else { return };
    let f = run(&mut w, &repeat(stick(0, 80), 60));
    let top = f.iter().map(|x| x.pos.y).fold(f32::MIN, f32::max);
    assert_eq!(top, 120.0, "reached the top step");
}

#[test]
fn step_up_limit() {
    use oot_core::collision::CollisionBuilder;
    use oot_game::bgcheck::StaticCollision;
    use oot_game::world::World;
    let Some(d) = data() else { return };
    let climbs = |h: f32| -> (bool, &'static str) {
        let mut b = CollisionBuilder::new();
        let s = b.surface(0, 0);
        b.quad(Vec3::new(-300.0, 0.0, 300.0), Vec3::new(300.0, 0.0, 300.0), Vec3::new(300.0, 0.0, 0.0), Vec3::new(-300.0, 0.0, 0.0), s);
        b.quad(Vec3::new(-300.0, h, 0.0), Vec3::new(300.0, h, 0.0), Vec3::new(300.0, h, -300.0), Vec3::new(-300.0, h, -300.0), s);
        b.quad(Vec3::new(-300.0, 0.0, 0.0), Vec3::new(300.0, 0.0, 0.0), Vec3::new(300.0, h, 0.0), Vec3::new(-300.0, h, 0.0), s);
        let mut w = World::new(d.clone(), StaticCollision::new(b.finish()), true, Vec3::new(0.0, 0.0, 150.0), -0x8000);
        // 40 frames: long enough to reach and climb the step, short of running off the far end.
        let f = run(&mut w, &repeat(stick(0, 80), 40));
        let how = if f.iter().any(|x| x.action == "ClimbLedge") {
            "climb"
        } else if f.iter().any(|x| x.action == "Midair") {
            "hop"
        } else {
            "walk"
        };
        (f.last().unwrap().pos.y == h, how)
    };
    let mut walk = 0;
    let mut hop = 0;
    for h in 1..=45 {
        match climbs(h as f32) {
            (true, "walk") => walk = h,
            (true, "hop") => hop = h,
            (true, "climb") => {
                // func_80838A14's class 2 starts at sAgeProperties.unk_1C = 41 for adult Link.
                assert_eq!(h, 41, "first climbed step");
                break;
            }
            other => panic!("step {h}: {other:?}"),
        }
    }
    println!("walked up to {walk}, hopped up to {hop}");
    // Walking: the wall line test ~18.5 above the feet stops taller risers (spike 03).
    assert!((15..20).contains(&walk), "walk limit {walk}");
    // Hopping (unk_88C == 1): from the 18 of the wall-height check up to unk_1C.
    assert_eq!(hop, 40);
}

#[test]
fn spec_script_forward_40_then_a() {
    // The example from the spike brief: full stick forward for 40 frames, then A.
    let Some(mut w) = world_at(Vec3::new(-300.0, 0.0, 900.0), -0x8000) else { return };
    let mut s: Vec<PadState> = repeat(stick(0, 80), 40);
    s.push(with(stick(0, 80), BTN_A));
    s.extend(repeat(stick(0, 0), 30));
    let f = run(&mut w, &s);
    let roll = find(&f, "Roll").unwrap();
    assert_eq!(roll, 40);
    assert!(f.iter().all(|x| x.grounded));
    let end = f.last().unwrap();
    assert_eq!(end.action, "StandingStill");
    assert_eq!(end.speed, 0.0);
    println!("after 40 run + roll + 30 idle: travelled {:.1} units", 900.0 - end.pos.z);
}

#[test]
fn root_motion_moves_the_actor() {
    use oot_game::skelanime::{ANIM_FLAG_UPDATEXZ, ANIM_FLAG_UPDATEY};
    // SkelAnime_UpdateTranslation + AnimationContext_MoveActor: the root's movement since the
    // last frame, rotated by the facing yaw and scaled by the actor scale (0.01).
    let Some(mut w) = world_at(Vec3::new(0.0, 0.0, 0.0), 0x4000) else { return };
    let p = &mut w.player;
    let base = p.skel.base_transl;
    p.skel.move_flags = ANIM_FLAG_UPDATEXZ | ANIM_FLAG_UPDATEY | 8;
    p.skel.prev_transl = base;
    p.skel.prev_rot = p.actor.shape_rot.y;
    // Root moved +1000 along the model's z (forward) and +500 up this frame.
    p.skel.joint[0] = [base[0], base[1] + 500, base[2] + 1000];
    p.skel.request_move_actor(1.0);
    let before = p.actor.world_pos;
    p.finish_frame();
    let d = p.actor.world_pos - before;
    // Facing +x (yaw 0x4000): model +z maps to world +x.
    assert!((d.x - 10.0).abs() < 1e-3 && d.z.abs() < 1e-3 && (d.y - 5.0).abs() < 1e-3, "{d}");
    // The root is reset to the base translation for drawing.
    assert_eq!(p.skel.joint[0], base);
}
