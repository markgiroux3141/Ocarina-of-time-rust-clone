//! Milestone 3 checks: `func_8008F87C` (foot IK). Expected values come from `z_player_lib.c`.

mod common;

use common::*;
use oot_actors::PlayExt;
use glam::Vec3;

#[test]
fn foot_ik_constants_read_from_decomp() {
    let Some(d) = data() else { return };
    let k = &d.foot_ik;
    // D_80126038 = { { 1304, 0, 0 }, { 695, 0, 0 } }, D_80126050 = { 1265, 826 },
    // D_80126058 = { SQ(13.04f), SQ(6.95f) }, D_80126060 = { 10.019104f, -19.925102f },
    // D_80126068 = { 5, 3 }, D_80126070 = { 0, -300, 0 }.
    assert_eq!(k.shin_offset, [Vec3::new(1304.0, 0.0, 0.0), Vec3::new(695.0, 0.0, 0.0)]);
    assert_eq!(k.foot_x, [1265.0, 826.0]);
    assert!((k.thigh_len_sq[0] - 13.04f32 * 13.04).abs() < 1e-4 && (k.thigh_len_sq[1] - 6.95f32 * 6.95).abs() < 1e-4);
    assert_eq!(k.len_diff, [10.019104, -19.925102]);
    assert_eq!(k.floor_offset, [5.0, 3.0]);
    assert_eq!(k.footprint, Vec3::new(0.0, -300.0, 0.0));
    // The hardcoded shin offsets are the skeletons' own shin joint positions.
    for (age, rig) in d.rigs.iter().enumerate() {
        let shin = d.limb("L_SHIN");
        assert_eq!(rig.joint_pos[shin][0] as f32, k.shin_offset[age].x, "age {age}");
    }
}

#[test]
fn flat_ground_leaves_standing_legs_alone() {
    let Some(mut w) = world() else { return };
    run(&mut w, &repeat(stick(0, 0), 10));
    let legs = w.player().legs.expect("IK ran");
    for l in legs {
        // Ankles at least D_80126068[adult] = 5 above the floor (y = 0): no correction.
        assert!(!l.adjusted, "{l:?}");
        assert!(l.ankle.y >= 5.0 - 1e-3, "{l:?}");
    }
}

#[test]
fn slope_puts_uphill_ankle_on_the_floor() {
    let Some(mut w) = world_at(Vec3::new(-700.0, 80.0, -300.0), 0x4000) else { return };
    run(&mut w, &repeat(stick(0, 0), 20));
    let legs = w.player().legs.expect("IK ran");
    eprintln!("slope legs: {legs:#?}");
    let up = legs.iter().filter(|l| l.adjusted).collect::<Vec<_>>();
    assert!(!up.is_empty(), "no leg adjusted on a 20.6° slope: {legs:?}");
    for l in up {
        // The solve moves the ankle to y = floor + 5 (sp80), keeping its x/z.
        assert!((l.ankle.y - l.floor).abs() < 1.0, "{l:?}");
    }
    // Neither ankle ends below its floor target.
    for l in legs {
        assert!(l.ankle.y > l.floor - 1.0, "{l:?}");
    }
}
