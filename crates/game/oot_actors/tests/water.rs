//! Milestone 6 checks: water boxes (`BgCheck_GetWaterSurface`), `yDistToWater`, entering and
//! leaving water (`func_8083D53C`), buoyancy (`func_8084B000`), swim strokes (`func_8084AEEC`),
//! diving (`Player_Action_8084DC48`) and surfacing (`func_8083D12C`, `Player_Action_8084E1EC`).
//! Expected values come from `z_player.c` / `z_bgcheck.c` and `sAgeProperties`, not the port.
//! The course's pool: water surface y=-20 over x∈[-950,-500], z∈[650,950]; floor -150 over
//! x∈[-950,-800], then a ramp up to the ground at x=-500.

mod common;

use common::*;
use oot_actors::PlayExt;
use oot_game::play::PlayState;
use eng_collision::bgcheck::CollisionContext;
use eng_collision::collision::CollisionBuilder;
use eng_input::pad::{BTN_A, BTN_Z};
use glam::Vec3;
use oot_actors::player::{Action, STATE1_17, STATE1_27, STATE2_10};
use oot_game::course;

const SURFACE: f32 = -20.0;

fn y_dist(f: &Frame) -> f32 {
    SURFACE - f.pos.y
}

fn child_at(pos: Vec3, yaw: i16) -> Option<PlayState> {
    let d = data()?;
    let c = course::build();
    Some(new_world(d, CollisionContext::new(c.collision), false, pos, yaw))
}

/// Records whether Player is in water (`PLAYER_STATE1_27`) alongside each frame.
fn run_wet(w: &mut PlayState, script: &[eng_input::pad::PadState]) -> Vec<(Frame, bool)> {
    let mut out = Vec::new();
    for p in script {
        let f = run(w, std::slice::from_ref(p)).remove(0);
        out.push((f, w.player().state1 & STATE1_27 != 0));
    }
    out
}

#[test]
fn water_box_surface_query() {
    // BgCheck_GetWaterSurface: room ((properties >> 13) & 0x3F) must match, or be 0x3F; boxes
    // with bit 19 set are skipped; the x/z test is strict.
    let mut b = CollisionBuilder::new();
    let s = b.surface(0, 0);
    b.quad(Vec3::new(-500.0, -100.0, 500.0), Vec3::new(500.0, -100.0, 500.0), Vec3::new(500.0, -100.0, -500.0), Vec3::new(-500.0, -100.0, -500.0), s);
    b.water_box(0, 0, 100, 100, -10, 2);
    b.water_box(-200, -200, 100, 100, 30, 0x3F);
    let mut h = b.finish();
    h.water_boxes.push(eng_collision::collision::WaterBox { x_min: 200, y_surface: 5, z_min: 200, x_length: 50, z_length: 50, properties: (0x3F << 13) | (1 << 19) });
    let col = CollisionContext::new(h);
    assert_eq!(col.water_surface(50.0, 50.0, 2), Some(-10.0));
    assert_eq!(col.water_surface(50.0, 50.0, 1), None, "room-bound box");
    assert_eq!(col.water_surface(0.0, 50.0, 2), None, "x_min is exclusive");
    assert_eq!(col.water_surface(100.0, 50.0, 2), None, "x_min + x_length is exclusive");
    assert_eq!(col.water_surface(-150.0, -150.0, 7), Some(30.0), "WATERBOX_ROOM_ALL");
    assert_eq!(col.water_surface(225.0, 225.0, 0), None, "bit 19 boxes are ignored");
}

#[test]
fn age_thresholds_come_from_age_properties() {
    let Some(d) = data() else { return };
    // sAgeProperties[0] (adult) / [1] (child): unk_24, unk_28, unk_2C, unk_30.
    let a = &d.ages[0];
    assert_eq!((a.unk_24, a.unk_28, a.unk_2C, a.unk_30), (36.0, 44.8, 56.0, 68.0));
    let c = &d.ages[1];
    assert_eq!((c.unk_24, c.unk_28, c.unk_2C, c.unk_30), (22.0, 29.6, 32.0, 48.0));
}

#[test]
fn wading_in_starts_swimming_past_unk_2c() {
    // Down the ramp towards -x. func_8083D53C runs after the move and bgcheck, so the frame's
    // recorded position is the one it tested: swimming starts on the first frame with
    // yDistToWater > unk_2C (56), and Player_UpdateCommon then scales speeds by 0.5 (sWaterSpeedFactor).
    let Some(mut w) = world_at(Vec3::new(-450.0, 0.0, 800.0), -0x4000) else { return };
    let f = run_wet(&mut w, &repeat(stick(0, 80), 40));
    let k = f.iter().position(|(_, wet)| *wet).expect("entered the water");
    assert!(y_dist(&f[k].0) > 56.0, "{}", y_dist(&f[k].0));
    assert!(y_dist(&f[k - 1].0) <= 56.0, "{}", y_dist(&f[k - 1].0));
    assert!(matches!(f[k].0.action.as_str(), "Swim" | "SwimMove"), "{}", f[k].0.action);
    assert!(w.player().state2 & STATE2_10 != 0 || w.player().action != Action::Dive);
    assert_eq!(w.player().s.speed_scale, 0.5);
    // Before that, shallow water only sets the bgcheck water flags.
    assert!(f[..k].iter().all(|(x, _)| x.action == "Run"));
}

#[test]
fn treading_water_floats_at_unk_28() {
    // Dropped over the deep end: sinks, bobs up (func_8084B000 pushes up by 0.1 + 0.3·|vy| when
    // deeper than unk_28, down by 0.1 + vy/2 above it, capped at +2 / -5) and settles about
    // unk_28 = 44.8 below the surface.
    let Some(mut w) = world_at(Vec3::new(-880.0, 0.0, 800.0), 0x4000) else { return };
    let f = run_wet(&mut w, &repeat(stick(0, 0), 200));
    let k = f.iter().position(|(_, wet)| *wet).expect("fell in");
    assert!(y_dist(&f[k].0) > 56.0);
    // Falling in with Player's STATE2_10 set: the surfacing animation, then treading water.
    assert!(f[k..].iter().any(|(x, _)| x.anim == "link_swimer_swim_deep_end"));
    assert_eq!(f.last().unwrap().0.action, "Swim");
    let tail: Vec<f32> = f[140..].iter().map(|(x, _)| y_dist(x)).collect();
    let mean = tail.iter().sum::<f32>() / tail.len() as f32;
    assert!((mean - 44.8).abs() < 1.5, "mean depth {mean}");
    assert!(tail.iter().all(|d| (d - 44.8).abs() < 3.5), "{tail:?}");
    for (x, _) in &f[k + 1..] {
        assert!(x.vy <= 2.0 + 1e-4 && x.vy >= -5.0 - 1e-4, "vy {}", x.vy);
    }
}

#[test]
fn swim_strokes_accelerate_only_mid_stroke() {
    // func_8084AEEC: the speed is capped at R_RUN_SPEED_LIMIT/100 · 0.8 and steps towards
    // stick speed · 0.8 by (frame - 10) · 6 only while the stroke's frame is in (10, 20);
    // otherwise it steps towards 0 by |v|·0.02 + 0.05.
    let Some(mut w) = world_at(Vec3::new(-880.0, 0.0, 800.0), 0x4000) else { return };
    run(&mut w, &repeat(stick(0, 0), 120));
    let f = run(&mut w, &repeat(stick(0, 80), 60));
    let cap = w.data.regs[0].reg(45) as f32 / 100.0 * 0.8;
    assert!((cap - 4.8).abs() < 1e-6);
    let f: Vec<Frame> = f.into_iter().take_while(|x| x.action.starts_with("Swim")).collect();
    assert!(f.len() > 40, "{}", f.len());
    let max = f.iter().map(|x| x.speed).fold(0.0f32, f32::max);
    assert!((max - cap).abs() < 1e-4, "max {max}");
    let mut decels = 0;
    for p in f.windows(2) {
        let (a, b) = (p[0].speed, p[1].speed);
        if b < a && p[1].action == "SwimMove" {
            let expect = a - (a.abs() * 0.02 + 0.05);
            assert!((b - expect).abs() < 1e-4, "{a} -> {b}, expected {expect}");
            decels += 1;
        }
    }
    assert!(decels > 10);
    // Moving at half the land speed: it covers ground, but slower than running (6).
    assert!(f.last().unwrap().pos.x > -880.0 + 100.0);
}

#[test]
fn dive_holds_then_rises_and_surfaces() {
    let Some(mut w) = world_at(Vec3::new(-880.0, 0.0, 800.0), 0x4000) else { return };
    run(&mut w, &repeat(stick(0, 0), 120));
    assert_eq!(w.player().action, Action::Swim);
    assert!(w.player().state2 & STATE2_10 == 0, "func_80832340 cleared it after surfacing");
    // func_8083D12C: A with |unk_6C2| < 12000 → Player_Action_8084DC48, velocity.y = 0, STATE2_10|11.
    let mut s = vec![with(stick(0, 0), BTN_A)];
    s.extend(repeat(with(stick(0, 0), BTN_A), 20));
    let f = run(&mut w, &s);
    assert_eq!(f[0].action, "Dive");
    assert_eq!(f[0].anim, "link_swimer_swim_deep_start");
    // velocity.y stays 0 (no buoyancy in the first phase) until the animation's frame 20,
    // where it's set to -2.
    let k20 = f.iter().position(|x| x.vy != 0.0).expect("started sinking");
    assert!(f[k20].vy == -2.0 && f[k20].anim_frame >= 20.0, "{} @{}", f[k20].vy, f[k20].anim_frame);
    assert!(f[..k20].iter().all(|x| (x.pos.y - f[0].pos.y).abs() < 1e-3));
    // func_8083D330 once the start animation ends with A held: unk_6C2 = 16000 (pitched down).
    let mut depth = 0.0f32;
    let mut pitched = false;
    let mut script = repeat(with(stick(0, 0), BTN_A), 60);
    script.extend(repeat(stick(0, 0), 90));
    let mut frames = Vec::new();
    for p in &script {
        let x = run(&mut w, std::slice::from_ref(p)).remove(0);
        pitched |= w.player().unk_6C2 == 16000;
        depth = depth.max(y_dist(&x));
        frames.push(x);
    }
    assert!(pitched);
    // Holding A keeps swimming down only while yDistToWater < 120 (D_80854784[0], no scale).
    assert!(depth > 110.0 && depth < 130.0, "max depth {depth}");
    // Rising (phase 2) at up to min(depth·0.018 + 4, 8), and func_8083D12C surfaces once
    // rising with yDistToWater < unk_30 (68).
    let s = frames.iter().position(|x| x.action == "Surface").expect("surfaced");
    assert!(y_dist(&frames[s]) < 68.0 && frames[s].vy > 0.0);
    assert_eq!(frames[s].anim, "link_swimer_swim_deep_end");
    let up = (depth * 0.018 + 4.0).min(8.0);
    assert!(frames.iter().all(|x| x.vy <= up + 1e-3), "vy capped at {up}");
    assert_eq!(frames.last().unwrap().action, "Swim");
}

#[test]
fn wading_out_leaves_water_below_unk_24() {
    // Swim up the ramp (+x): func_8083D53C calls func_8083D0A8 once yDistToWater < unk_24 (36),
    // clearing PLAYER_STATE1_27; Link runs out.
    let Some(mut w) = world_at(Vec3::new(-880.0, 0.0, 800.0), 0x4000) else { return };
    run(&mut w, &repeat(stick(0, 0), 120));
    let f = run_wet(&mut w, &repeat(stick(0, 80), 120));
    let k = f.iter().position(|(_, wet)| !*wet).expect("left the water");
    assert!(y_dist(&f[k].0) < 36.0, "{}", y_dist(&f[k].0));
    assert!(y_dist(&f[k - 1].0) >= 36.0, "{}", y_dist(&f[k - 1].0));
    assert_eq!(w.player().s.speed_scale, 1.0);
    assert!(f.last().unwrap().0.pos.y == 0.0 && f.last().unwrap().0.action == "Run");
}

#[test]
fn child_swims_in_shallower_water() {
    // Child: unk_2C 32 to swim, floats at unk_28 29.6.
    let Some(mut w) = child_at(Vec3::new(-450.0, 0.0, 800.0), -0x4000) else { return };
    let f = run_wet(&mut w, &repeat(stick(0, 80), 40));
    let k = f.iter().position(|(_, wet)| *wet).expect("entered the water");
    assert!(y_dist(&f[k].0) > 32.0 && y_dist(&f[k - 1].0) <= 32.0, "{} {}", y_dist(&f[k - 1].0), y_dist(&f[k].0));
    let f = run(&mut w, &repeat(stick(0, 0), 160));
    let tail: Vec<f32> = f[100..].iter().map(y_dist).collect();
    let mean = tail.iter().sum::<f32>() / tail.len() as f32;
    assert!((mean - 29.6).abs() < 1.5, "mean depth {mean}");
}

#[test]
fn z_in_water_strafes() {
    // Z with nothing to target → parallel mode; Player_Action_8084DAB4 → func_8084D980 picks the stroke
    // by direction via func_8083FD78: sideways is link_swimer_Rside_swim / Lside_swim.
    let Some(mut w) = world_at(Vec3::new(-880.0, 0.0, 800.0), 0x4000) else { return };
    run(&mut w, &repeat(stick(0, 0), 120));
    let z = |p| with(p, BTN_Z);
    let f = run(&mut w, &repeat(z(stick(80, 0)), 20));
    assert!(w.player().state1 & STATE1_17 != 0);
    assert!(f.iter().any(|x| x.action == "SwimTarget"), "{:?}", f.iter().map(|x| &x.action).collect::<Vec<_>>());
    let last = f.last().unwrap();
    assert!(last.anim == "link_swimer_Rside_swim" || last.anim == "link_swimer_Lside_swim", "{}", last.anim);
    // Facing held while strafing.
    assert_eq!(last.facing, 0x4000);
}

#[test]
fn kokiri_forest_stream() {
    // spot04's collision has one water box, surface y -12 over x 73..1473, z -588..452, for
    // room 0 (the scene's main header, from the asset pack). The stream bed is at most 48 below
    // it, less than the adult's unk_2C (56): adult Link wades, child Link (unk_2C 32) swims.
    let Some(d) = data() else { return };
    let pack = pack().unwrap();
    let scene = pack.scene("spot04").expect("spot04");
    let collision = pack.collision(&scene.layers[0].collision).expect("spot04 collision");
    let col = CollisionContext::new(collision.clone());
    assert_eq!(col.water_surface(700.0, 0.0, 0), Some(-12.0));
    assert_eq!(col.water_surface(700.0, 0.0, 1), None, "room 0 only");
    assert_eq!(col.water_surface(0.0, 0.0, 0), None);
    let mut deepest = (0.0f32, Vec3::ZERO);
    for i in 0..28 {
        for k in 0..21 {
            let (x, z) = (80.0 + 50.0 * i as f32, -580.0 + 50.0 * k as f32);
            let (y, poly) = col.entity_raycast_down(Vec3::new(x, 50.0, z));
            if poly.is_some() && y < deepest.0 {
                deepest = (y, Vec3::new(x, y, z));
            }
        }
    }
    let (bed, at) = deepest;
    assert!((bed + 60.0).abs() < 1.0, "deepest bed {bed} at {at}");
    // Adult: stands on the bed with the water flags set.
    let mut w = new_world(d.clone(), CollisionContext::new(collision.clone()), true, at, 0);
    run(&mut w, &repeat(stick(0, 0), 30));
    assert!(w.player().state1 & STATE1_27 == 0);
    assert!(w.player().actor.bg_check_flags & oot_game::actor::BGCHECKFLAG_WATER != 0);
    assert!((w.player().actor.y_dist_to_water - (-12.0 - bed)).abs() < 1.0);
    assert_eq!(w.player().action, Action::StandingStill);
    // Child: swims, floating unk_28 = 29.6 below the surface.
    let mut w = new_world(d, col, false, at, 0);
    let f = run(&mut w, &repeat(stick(0, 0), 160));
    assert!(w.player().state1 & STATE1_27 != 0, "swimming at {at}");
    let mean = f[100..].iter().map(|x| -12.0 - x.pos.y).sum::<f32>() / 60.0;
    assert!((mean - 29.6).abs() < 1.5, "{mean}");
}
