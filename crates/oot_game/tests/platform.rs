//! Milestone 7 checks: DynaPoly collision with one moving platform, `Bg_Ydan_Hasi`'s floating
//! block (`gDTSlidingPlatformCol`, read from `object_ydan_objects`). Expected values follow
//! from `z_bgcheck.c` (`DynaPoly_AddBgActorToLookup`, the `BgCheck_*Dyna*` checks),
//! `code_800430A0.c` (carrying) and `z_bg_ydan_hasi.c`, not from the port.
//! The course's channel: water y -20 over x∈[350,950], z∈[-300,-100]; the platform's home is
//! (650, 0, -200) facing +x (0x4000).

mod common;

use std::f32::consts::PI;
use std::sync::Arc;

use common::*;
use glam::Vec3;
use oot_core::collision::CollisionHeader;
use oot_game::bgcheck::{CHECK_ALL, IGNORE_CAMERA, IGNORE_ENTITY, IGNORE_NONE};
use oot_game::course::{CHANNEL_WATER, PLATFORM_HOME, PLATFORM_YAW};
use oot_game::world::World;

fn header() -> Option<Arc<CollisionHeader>> {
    let p = oot_core::project::Project::open_default().ok()?;
    Some(oot_game::bg_ydan_hasi::load_collision(&p).expect("gDTSlidingPlatformCol"))
}

fn world_with_platform(pos: Vec3, yaw: i16) -> Option<World> {
    let mut w = world_at(pos, yaw)?;
    w.spawn_platform(header()?, PLATFORM_HOME, PLATFORM_YAW, CHANNEL_WATER);
    Some(w)
}

#[test]
fn the_collision_is_a_1000_unit_box() {
    // object_ydan_objects' gDTSlidingPlatformCol: 8 vertices ±500 in x/z, y -400..0; 12 polys
    // (2 floor, 8 wall, 2 ceiling); every poly carries COLPOLY_IGNORE_CAMERA (bit 13).
    let Some(h) = header() else { return };
    assert_eq!((h.vertices.len(), h.polys.len()), (8, 12));
    assert_eq!((h.min_bounds, h.max_bounds), ([-500, -400, -500], [500, 0, 500]));
    assert!(h.polys.iter().all(|p| p.vtx[0] & 0x2000 != 0));
}

#[test]
fn expand_srt_transforms_and_bounds() {
    // BgYdanHasi_Init: scale (0.15, 0.1, 0.15), y = water + 20 = 0. The expanded vertex
    // (-500, 0, -500) → RotY(0x4000) · S = (-75, 0, 75) → + (650, 0, -200) = (575, 0, -125)
    // (SkinMatrix_SetRotateYXZ: x' = x·cosY + z·sinY, z' = -x·sinY + z·cosY), truncated to s16.
    let Some(w) = world_with_platform(Vec3::new(0.0, 0.0, 0.0), 0) else { return };
    let d = &w.col.dyna;
    let a = &d.actors[0];
    assert!(d.verts_s[..8].contains(&[575, 0, -125]), "{:?}", &d.verts_s[..8]);
    // The sphere: the vertices' mean, radius sqrt(max dist²) · 1.1 truncated:
    // √(75² + 20² + 75²) · 1.1 = 118.73 → 118.
    assert_eq!(a.sphere_center, [650, -20, -200]);
    assert_eq!(a.sphere_radius, 118);
    assert_eq!((a.min_y, a.max_y), (-40.0, 0.0));
    // Lists: floors ny > 0.5, ceilings ny < -0.8, the rest walls; each inserted at the head,
    // so visited from the last poly to the first.
    assert_eq!(a.floor, vec![1, 0]);
    assert_eq!(a.ceiling, vec![11, 10]);
    assert_eq!(a.wall, vec![9, 8, 7, 6, 5, 4, 3, 2]);
}

#[test]
fn the_platform_slides_and_bobs() {
    // BgYdanHasi_UpdateFloatingBlock: x = home.x + sinS(0x4000) · sin((frames & 0xFF)·π/128) · 165;
    // y = water + 20 + 2·sin(timer·π/25), the timer counting 50, 49, ... 1, 50, ...
    let Some(mut w) = world_with_platform(Vec3::new(0.0, 0.0, 0.0), 0) else { return };
    let mut timer = 0i32;
    for n in 1..=300u32 {
        run(&mut w, &[stick(0, 0)]);
        timer = if timer == 0 { 50 } else { timer - 1 };
        if timer == 0 {
            timer = 50;
        }
        let p = &w.platforms[0];
        let x = 650.0 + ((n & 0xFF) as f32 * (PI / 128.0)).sin() * 165.0;
        let y = CHANNEL_WATER + 20.0 + 2.0 * (timer as f32 * (PI / 25.0)).sin();
        assert!((p.pos.x - x).abs() < 1e-3 && (p.pos.y - y).abs() < 1e-4, "frame {n}: {} {} vs {x} {y}", p.pos.x, p.pos.y);
        assert!((p.pos.z - -200.0).abs() < 1e-2);
    }
}

#[test]
fn standing_on_it_carries_link() {
    // Dropped onto the platform: floorBgId becomes its bg id, and each frame func_800430A0
    // moves Link by cur · prev⁻¹, so his offset from the platform stays fixed. His height is
    // the top face from the s16 vertex list, i.e. the platform y truncated.
    let Some(mut w) = world_with_platform(Vec3::new(650.0, 10.0, -200.0), 0) else { return };
    run(&mut w, &repeat(stick(0, 0), 10));
    assert_eq!(w.player.actor.floor_bg_id, 0);
    assert!(w.player.grounded());
    let off = w.player.actor.world_pos - w.platforms[0].pos;
    let mut max_dx = 0.0f32;
    for _ in 0..200 {
        let f = run(&mut w, &[stick(0, 0)]);
        let p = &w.platforms[0];
        let d = f[0].pos - p.pos;
        max_dx = max_dx.max((d.x - off.x).abs()).max((d.z - off.z).abs());
        assert_eq!(f[0].pos.y, p.pos.y as i16 as f32, "top face at the truncated platform y");
        assert_eq!(f[0].action, "StandingStill");
    }
    assert!(max_dx < 0.01, "drift {max_dx}");
}

#[test]
fn walking_off_the_platform_returns_to_the_scene_floor() {
    // Run off the far end towards the bank: Link jumps the gap (the edge is >11 above the
    // water floor) and lands on the static ground, floorBgId back to BGCHECK_SCENE (50).
    let Some(mut w) = world_with_platform(Vec3::new(650.0, 10.0, -200.0), 0x4000) else { return };
    run(&mut w, &repeat(stick(0, 0), 44));
    let f = run(&mut w, &repeat(stick(0, 80), 30));
    assert!(f.iter().any(|x| x.action == "Midair"));
    assert_eq!(w.player.actor.floor_bg_id, oot_game::bgcheck::BGCHECK_SCENE);
    assert!(w.player.actor.world_pos.x > 950.0 && w.player.actor.world_pos.y == 0.0);
}

#[test]
fn raycast_prefers_the_higher_dyna_floor() {
    // BgCheck_RaycastDownImpl: the static floor below (channel bed, -150) is replaced by the
    // dyna floor when higher; with nothing dynamic under the point, the static floor stays.
    let Some(w) = world_with_platform(Vec3::ZERO, 0) else { return };
    let (y, poly) = w.col.entity_raycast_down(Vec3::new(650.0, 50.0, -200.0));
    assert_eq!(y, 0.0);
    assert_eq!(poly.unwrap().bg, 0);
    let (y, poly) = w.col.entity_raycast_down(Vec3::new(900.0, 50.0, -200.0));
    assert_eq!(y, -150.0);
    assert!(poly.unwrap().is_scene());
    // Below the platform's bottom (minY -40) it's skipped.
    let (y, _) = w.col.entity_raycast_down(Vec3::new(650.0, -60.0, -200.0));
    assert_eq!(y, -150.0);
}

#[test]
fn walls_ceilings_and_lines_see_the_platform() {
    let Some(w) = world_with_platform(Vec3::ZERO, 0) else { return };
    let c = &w.col;
    // BgCheck_SphVsDynaWall: a radius-18 sphere (centre y -30 + 10) moving into the -x side (x = 575)
    // is pushed back to 575 - 18.
    let (hit, p, poly) = c.check_wall(IGNORE_ENTITY, Vec3::new(570.0, -30.0, -200.0), Vec3::new(540.0, -30.0, -200.0), 18.0, 10.0, 0);
    assert!(hit);
    assert_eq!(poly.unwrap().bg, 0);
    assert!((p.x - (575.0 - 18.0)).abs() < 0.01, "{p}");
    // BgCheck_CheckDynaCeiling: under the bottom face (y -40, facing down) with checkHeight 30:
    // y = -1 · 30 + (-40).
    let (y, poly) = c.check_ceiling(IGNORE_ENTITY, Vec3::new(650.0, -60.0, -200.0), 30.0).expect("ceiling");
    assert_eq!(y, -70.0);
    assert_eq!(poly.bg, 0);
    // BgCheck_CheckLineAgainstDyna: an entity line down through it stops at the top; a camera
    // line (COLPOLY_IGNORE_CAMERA) passes through to the channel bed; without CHECK_DYNA the
    // platform isn't tested at all.
    let (a, b) = (Vec3::new(650.0, 100.0, -200.0), Vec3::new(650.0, -200.0, -200.0));
    let (hit, poly) = c.check_line(IGNORE_ENTITY, IGNORE_NONE, a, b, 1.0, CHECK_ALL).unwrap();
    assert!((hit.y - 0.0).abs() < 1e-3 && poly.bg == 0);
    let (hit, poly) = c.check_line(IGNORE_CAMERA, IGNORE_NONE, a, b, 1.0, CHECK_ALL).unwrap();
    assert!((hit.y + 150.0).abs() < 1e-3 && poly.is_scene());
    let (hit, _) = c.check_line(IGNORE_ENTITY, IGNORE_NONE, a, b, 1.0, CHECK_ALL & !oot_game::bgcheck::CHECK_DYNA).unwrap();
    assert!((hit.y + 150.0).abs() < 1e-3);
}

#[test]
fn surface_types_come_from_the_platform_header() {
    // SurfaceType_Get*(colCtx, poly, bgId) reads the bg actor's own header: the platform's
    // walls use type 1 (data0 0x200000: wall type 1 → WALL_FLAG_0, D_80119D90), its top type 0.
    let Some(w) = world_with_platform(Vec3::ZERO, 0) else { return };
    let (_, floor) = w.col.entity_raycast_down(Vec3::new(650.0, 50.0, -200.0));
    assert_eq!(w.col.wall_type(floor.unwrap()), 0);
    let (_, _, wall) = w.col.check_wall(IGNORE_ENTITY, Vec3::new(570.0, -30.0, -200.0), Vec3::new(540.0, -30.0, -200.0), 18.0, 10.0, 0);
    assert_eq!(w.col.wall_type(wall.unwrap()), 1);
    assert_eq!(w.col.wall_flags(wall.unwrap()), oot_game::bgcheck::WALL_FLAG_0);
}
