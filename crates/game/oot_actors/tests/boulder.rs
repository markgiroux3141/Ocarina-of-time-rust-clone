//! The training area's rolling boulder (`ovl_En_Goroiwa/z_en_goroiwa.c`) and what it does to
//! Link: the body hit and the knockdown (`func_808382DC`, `func_80837C0C`, `Player_Action_8084370C`,
//! `Player_Action_8084377C`, `Player_Action_80843954`, `Player_Action_80843A38` in `z_player.c`).
//!
//! Expected values come from the scene data and the C:
//! - room 2's `En_Goroiwa`: params 0x0C02 (path 2, loop mode 0, bit 10: gravity and the floor
//!   check), `rot.z` 1;
//! - path 2: (-247, 120, 1869), (-247, 120, 1538), (-575, 120, 1538), (-575, 120, 1869),
//!   (-247, 120, 1869);
//! - `EnGoroiwa_SetSpeed`: `R_EN_GOROIWA_SPEED` 920 in `SCENE_KOKIRI_FOREST`, so 9.2 a frame, reached
//!   by `Math_StepToF` steps of 0.3;
//! - its sphere: radius 58, 59.5 up (bit 10), AT `0x20000000` with 4 damage; `MASS_HEAVY`.

mod common;

use std::sync::Arc;

use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_goroiwa::{self, EnGoroiwa};
use oot_actors::player::Action;
use oot_game::actor_ctx::ActorHandle;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::sys_matrix::{MtxF, binang_to_rad};

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn tick(w: &mut PlayState) {
    w.tick_with(scripted_input(PadState::default(), PadState::default()));
}

/// Kokiri Forest on a new save with room 2 loaded and Link placed at `link`, facing `yaw`;
/// returns once the boulder has spawned (its object loads with the room).
fn room2(a: &Arc<GameAssets>, link: Vec3, yaw: i16) -> Option<(PlayState, ActorHandle)> {
    let e = a.scenes.entrance_index("ENTR_KOKIRI_FOREST_0").expect("entrance");
    let save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    let mut w = oot_actors::play_entrance(a.clone(), common::data()?, common::rules()?, save).expect("Play_Init");
    for _ in 0..20 {
        tick(&mut w);
    }
    w.room_request(2);
    tick(&mut w);
    w.room_change_done();
    w.place_player(link, yaw);
    for _ in 0..10 {
        if let Some(h) = boulder(&w) {
            return Some((w, h));
        }
        tick(&mut w);
    }
    panic!("no boulder in room 2");
}

fn boulder(w: &PlayState) -> Option<ActorHandle> {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnGoroiwa>(h).is_some())
}

fn b(w: &PlayState, h: ActorHandle) -> &EnGoroiwa {
    w.actors.downcast::<EnGoroiwa>(h).unwrap()
}

/// The rotation a YXZ angle triple stands for (`Matrix_RotateY`, then X, then Z).
fn yxz(r: [i16; 3]) -> MtxF {
    let mut m = MtxF::IDENTITY;
    m.rotate_y(binang_to_rad(r[1]));
    m.rotate_x(binang_to_rad(r[0]));
    m.rotate_z(binang_to_rad(r[2]));
    m
}

/// The angle of the rotation between two orientations (from the trace of `a`ᵀ·`b`).
fn angle_between(a: &MtxF, b: &MtxF) -> f32 {
    let col = |m: &MtxF, i: usize| match i {
        0 => Vec3::new(m.xx, m.yx, m.zx),
        1 => Vec3::new(m.xy, m.yy, m.zy),
        _ => Vec3::new(m.xz, m.yz, m.zz),
    };
    let trace: f32 = (0..3).map(|i| col(a, i).dot(col(b, i))).sum();
    ((trace - 1.0) / 2.0).clamp(-1.0, 1.0).acos()
}

#[test]
fn it_rolls_round_path_2() {
    let Some(a) = assets() else { return };
    // Link out of its way, in the tunnel's mouth on room 2's side.
    let Some((mut w, h)) = room2(&a, Vec3::new(-785.0, 120.0, 1420.0), 0) else { return };
    let g = b(&w, h);
    // EnGoroiwa_Init: path 2's point 0, facing point 1 (south, 0x8000); 0.1 scale, the model
    // 595 up (bit 10), the sphere 58; rolling (EnGoroiwa_SetupRoll: AT and OC on).
    assert_eq!(g.actor.home_pos, Vec3::new(-245.0, 120.0, 1870.0), "placed at the actor list's position");
    assert_eq!((g.end_waypoint, g.current_waypoint, g.next_waypoint, g.path_direction), (4, 0, 1, 1));
    assert_eq!(g.actor.world_rot.y, -0x8000);
    assert_eq!((g.speed_reg, g.is_in_kokiri), (920, true));
    assert_eq!((g.actor.scale, g.actor.shape_y_offset), (Vec3::splat(0.1), 595.0));
    assert_eq!(g.collider.elements[0].dim.world_sphere.radius, 58);
    assert_eq!(g.state_flags, en_goroiwa::ENGOROIWA_ENABLE_AT | en_goroiwa::ENGOROIWA_ENABLE_OC);
    assert_eq!(g.actor.col_chk_info.mass, oot_game::collision_check::MASS_HEAVY);
    // Teleported to point 0 at init (the first frame may already have moved it).
    assert_eq!((g.actor.world_pos.x, g.actor.world_pos.y), (-247.0, 120.0));

    // Math_StepToF(&speedXZ, 9.2, 0.3): 0.3 more a frame until 9.2.
    let mut speeds = Vec::new();
    for _ in 0..40 {
        tick(&mut w);
        speeds.push(b(&w, h).actor.speed_xz);
    }
    let k = speeds.iter().position(|&s| s > 0.0).unwrap();
    for (i, s) in speeds[k..].iter().enumerate() {
        let want = (0.3 * (i + 1) as f32).min(9.2);
        assert!((s - want).abs() < 1e-3, "frame {i}: {s} vs {want}");
    }

    // Round the loop: at 9.2, each side (331 and 328 long) takes 36 frames, the last step
    // snapping onto the point (Math_StepToF); at point 4, EnGoroiw_CheckEndOfPath goes back to
    // 0 → 1 (ONEWAY), EnGoroiwa_TeleportToWaypoint onto point 0 (the same place).
    let mut changes: Vec<(usize, i16, i16)> = Vec::new();
    let mut last = (b(&w, h).current_waypoint, b(&w, h).next_waypoint);
    for f in 0..400 {
        tick(&mut w);
        let g = b(&w, h);
        if (g.current_waypoint, g.next_waypoint) != last {
            last = (g.current_waypoint, g.next_waypoint);
            changes.push((f, last.0, last.1));
            let p = a_point(&w, last.0);
            assert_eq!(g.actor.world_pos.x, p.x, "on point {} when it turns", last.0);
            assert_eq!(g.actor.world_pos.z, p.z);
        }
        assert_eq!(g.action, en_goroiwa::Action::Roll);
        assert_eq!(g.actor.world_pos.y, 120.0, "on the floor (UPDBGCHECKINFO_FLAG_2)");
    }
    let order: Vec<(i16, i16)> = changes.iter().map(|c| (c.1, c.2)).collect();
    assert_eq!(&order[..6], &[(1, 2), (2, 3), (3, 4), (0, 1), (1, 2), (2, 3)]);
    for w2 in changes.windows(2).skip(1) {
        assert_eq!(w2[1].0 - w2[0].0, 36, "a side in 36 frames: {changes:?}");
    }
}

fn a_point(w: &PlayState, i: i16) -> Vec3 {
    w.setup_path_list()[2].point(i as usize)
}

#[test]
fn it_rolls_by_the_distance_over_its_radius() {
    let Some(a) = assets() else { return };
    let Some((mut w, h)) = room2(&a, Vec3::new(-785.0, 120.0, 1420.0), 0) else { return };
    for _ in 0..60 {
        tick(&mut w);
    }
    // EnGoroiwa_UpdateRotation: the distance moved over 59.5, about the horizontal axis
    // across the velocity, on top of the shape's rotation.
    for _ in 0..20 {
        let before = b(&w, h).actor.shape_rot;
        let from = b(&w, h).actor.world_pos;
        tick(&mut w);
        let g = b(&w, h);
        let moved = g.actor.world_pos.distance(from);
        let turned = angle_between(&yxz([before.x, before.y, before.z]), &yxz([g.actor.shape_rot.x, g.actor.shape_rot.y, g.actor.shape_rot.z]));
        assert!((turned - moved / 59.5).abs() < 0.002, "{turned} vs {}", moved / 59.5);
    }
}

#[test]
fn it_knocks_link_down() {
    let Some(a) = assets() else { return };
    // Link on the bottom side of the path (z 1538), facing east, where the boulder comes west.
    let Some((mut w, h)) = room2(&a, Vec3::new(-400.0, 120.0, 1538.0), 0x4000) else { return };
    let health = w.save.health;
    let mut frames = 0;
    while w.player().action == Action::StandingStill {
        tick(&mut w);
        frames += 1;
        assert!(frames < 200, "the boulder reaches Link");
    }
    // The collision check of the frame before: its AT sphere on Link's AC cylinder. Player's
    // update: func_808382DC's AC branch, func_80837C0C(0, 4.0, 5.0, yaw from the boulder to
    // Link, 20): Health_ChangeBy(-4) (the sphere's toucher damage, no damage table), 20
    // frames of invincibility, and with Link still (speed under 4), the stagger
    // (Player_Action_8084370C) from the front: D_808544B0[0].
    let p = w.player();
    assert_eq!(p.action, Action::Damaged);
    assert_eq!(w.save.health, health - 4, "a quarter heart");
    assert_eq!(p.invincibility_timer, 20);
    assert_eq!(w.data.anim_name(p.skel.animation), "link_normal_front_shit");
    // The boulder's update that frame: AT_HIT with Link ahead (yawTowardsPlayer within a
    // quarter turn of its way), so EnGoroiwa_ReverseDirection; Actor_SetPlayerKnockbackLarge(2.0,
    // yawTowardsPlayer, 0.0, 0); EnGoroiwa_SetupMoveAndFallToGround (a hop: 5 up, 0.15 of its
    // speed); 50 frames without colliding (rot.z bit 0).
    let g = b(&w, h);
    assert_eq!(g.action, en_goroiwa::Action::MoveAndFallToGround);
    assert_eq!(g.path_direction, -1);
    assert_eq!((g.current_waypoint, g.next_waypoint), (2, 1));
    assert_eq!(g.actor.velocity.y, 5.0);
    assert!((g.actor.speed_xz - 9.2 * 0.15).abs() < 1e-4);
    assert_eq!(g.collision_disabled_timer, 50);
    assert!(g.state_flags & en_goroiwa::ENGOROIWA_PLAYER_IN_THE_WAY != 0);
    let away = g.actor.yaw_towards_player;

    // Next frame, the knockback (knockbackType 2 → sp5C[1] = 1): func_80837C0C(1, 2.0, 0.0, away):
    // invincible, so no more damage; knocked down (Player_Action_8084377C) backwards at 2, facing the
    // boulder (the hit came from the front: shape.rot.y turned by 0x8000 back).
    tick(&mut w);
    let p = w.player();
    assert_eq!(p.action, Action::KnockedDown);
    assert_eq!(w.save.health, health - 4);
    assert_eq!(w.data.anim_name(p.skel.animation), "link_normal_front_downA");
    assert_eq!((p.current_yaw, p.linear_velocity), (away, 2.0));
    assert_eq!(p.actor.shape_rot.y, away.wrapping_add(i16::MIN));
    assert_eq!(p.knockback_type, 0, "cleared at the end of Player_UpdateCommon");

    // Down (Player_Action_80843954, front_downB), up (Player_Action_80843A38, front_down_wake), standing; all
    // the while invincible (func_808382BC keeps it at 20), then counting down.
    let mut seen = vec![Action::KnockedDown];
    while w.player().action != Action::StandingStill {
        tick(&mut w);
        let p = w.player();
        if seen.last() != Some(&p.action) {
            seen.push(p.action);
        }
        if p.action != Action::StandingStill {
            assert_eq!(p.invincibility_timer, 20);
        }
        assert!(seen.len() < 6);
    }
    assert_eq!(seen, [Action::KnockedDown, Action::Down, Action::GetUp, Action::StandingStill]);
    let t = w.player().invincibility_timer;
    tick(&mut w);
    assert_eq!(w.player().invincibility_timer, t - 1);

    // The boulder, on the ground again: back on its way (EnGoroiwa_MoveAndFallToGround,
    // Link was ahead), a 6-frame wait (rot.z bit 0), rolling again.
    let mut frames = 0;
    while b(&w, h).action != en_goroiwa::Action::Roll {
        tick(&mut w);
        frames += 1;
        assert!(frames < 100);
    }
    let g = b(&w, h);
    assert_eq!((g.path_direction, g.current_waypoint, g.next_waypoint), (1, 1, 2));
}

#[test]
fn it_needs_a_path() {
    let Some(a) = assets() else { return };
    let Some((mut w, _)) = room2(&a, Vec3::new(-785.0, 120.0, 1420.0), 0) else { return };
    // EnGoroiwa_Init: path 0xFF is "arg_data" invalid, and it goes.
    let h = w.actor_spawn(en_goroiwa::ACTOR_EN_GOROIWA, Vec3::new(-400.0, 120.0, 1700.0), [0; 3], 0x0CFF).expect("spawn");
    tick(&mut w);
    assert!(w.actors.get(h).is_none_or(|a| a.base().killed));
}
