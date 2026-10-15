//! `En_Horse_Normal` (`z_en_horse_normal.c`): Lon Lon Ranch's horses as a child by day (spot20
//! layer 0: eight 0x000F with rot.z 1 or 3, two of them at the clones' places (-730, 0, -1100) and
//! (880, 0, -1170); two 0x0010 at path 0's first point (736, 1, 429), rot.z 1 and 3), and the
//! stable's two by night (`malon_stable` layer 1: 0x000F at (355, 0, -245) and (238, 0, -245),
//! rot.z 170).
//!
//! Distances move 1.5 times the speed a frame (`Actor_UpdatePos`: `R_UPDATE_RATE * 0.5`).
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::{Mat4, Vec3};
use oot_actors::PlayExt;
use oot_actors::en_horse_normal::*;
use oot_game::actor::{ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED};
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::sfx::{NA_SE_EV_HORSE_GROAN, NA_SE_EV_HORSE_NEIGH};
use oot_game::collision_check::{MASS_HEAVY, MASS_IMMOVABLE};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::skin::SkinSkeleton;

/// `UPDATE_SCALE` (`R_UPDATE_RATE * 0.5`).
const RATE: f32 = 1.5;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// `entrance` entered as a child (an adult with `adult`) at `hour`:00 with the sound log, Talon
/// back from the castle (`castle-courtyard`: `EVENTCHKINF_TALON_RETURNED_FROM_CASTLE`) if
/// `talon`; two frames on (the actors spawned, not yet updated).
fn enter(a: &Arc<GameAssets>, entrance: &str, adult: bool, hour: i32, talon: bool) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, adult, oot_game::env::clock_time(hour, 0) as u16);
    if talon {
        save.apply_preset("castle-courtyard").unwrap();
    }
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    w
}

fn horses(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnHorseNormal>(h).is_some_and(|x| !x.actor.killed)).collect()
}

fn horse(w: &PlayState, h: ActorHandle) -> &EnHorseNormal {
    w.actors.downcast::<EnHorseNormal>(h).unwrap()
}

/// The live horses' places (x, z) and actions, sorted.
fn census(w: &PlayState) -> Vec<((i32, i32), Action)> {
    let mut v: Vec<_> = horses(w)
        .into_iter()
        .map(|h| {
            let x = horse(w, h);
            ((x.actor.home_pos.x as i32, x.actor.home_pos.z as i32), x.action)
        })
        .collect();
    v.sort_by_key(|(p, _)| *p);
    v
}

fn sfx_at(w: &PlayState, id: u16, frame: u32) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(f, s, _)| s == id && f == frame).count()
}

#[test]
fn the_ranchs_horses_by_talons_return_and_the_day() {
    let Some(a) = assets() else { return };
    use Action::*;
    // A child before Talon's return: the rot.z 1 ones (EnHorseNormal_Init); the two at the
    // clones' places wait with their clones (func_80A6C6B0), the 0x0010 gallops path 0
    // (func_80A6B91C), the rest wander (func_80A6BC48).
    let w = enter(&a, "ENTR_LON_LON_RANCH_0", false, 10, false);
    assert_eq!(
        census(&w),
        vec![((-1316, -557), Wander), ((-730, -1100), WaitClone), ((124, -1343), Wander), ((736, 429), FollowPath), ((880, -1170), WaitClone), ((1257, -735), Wander)]
    );
    // After it: the rot.z 3 ones.
    let w = enter(&a, "ENTR_LON_LON_RANCH_0", false, 10, true);
    assert_eq!(census(&w), vec![((-780, -1175), Wander), ((736, 429), FollowPath), ((750, -1076), Wander), ((821, -283), Wander)]);
    for h in horses(&w) {
        let x = horse(&w, h);
        // The ranch's rot.z zeroed; scale 0.01, gravity -3.5; sInitChain's culling volume.
        assert_eq!((x.actor.home_rot.z, x.actor.world_rot.z, x.actor.shape_rot.z), (0, 0, 0));
        assert_eq!((x.actor.scale, x.actor.gravity), (Vec3::splat(0.01), -3.5));
        assert_eq!((x.actor.culling_volume_scale, x.actor.culling_volume_downward), (1200.0, 300.0));
    }
    // By night the ranch's night layer places none.
    let w = enter(&a, "ENTR_LON_LON_RANCH_0", false, 22, true);
    assert!(census(&w).is_empty());

    // The init's other checks, by spawning one in the day's ranch: rot.z 0 never; not by night;
    // an adult's 5 before Epona, 7 after.
    let mut w = enter(&a, "ENTR_LON_LON_RANCH_0", false, 10, true);
    let pos = Vec3::new(0.0, 0.0, -900.0);
    let lives = |w: &mut PlayState, rot_z: i16| {
        let h = w.actor_spawn(ACTOR_EN_HORSE_NORMAL, pos, [0, 0, rot_z], 0x000F).expect("spawn");
        let alive = !horse(w, h).actor.killed;
        w.actors.actor_mut(h).unwrap().kill();
        alive
    };
    assert_eq!((lives(&mut w, 0), lives(&mut w, 1), lives(&mut w, 3)), (false, false, true));
    w.save.night_flag = true;
    assert!(!lives(&mut w, 3), "IS_DAY");
    w.save.night_flag = false;
    w.save.adult = true;
    assert_eq!((lives(&mut w, 3), lives(&mut w, 5), lives(&mut w, 7)), (false, true, false));
    w.save.set_event_chk_inf(EVENTCHKINF_EPONA_OBTAINED);
    assert_eq!((lives(&mut w, 5), lives(&mut w, 7)), (false, true));
}

#[test]
fn the_stable_by_night_has_two_horses_with_clones() {
    let Some(a) = assets() else { return };
    // By day: Actor_Kill.
    let w = enter(&a, "ENTR_STABLE_0", false, 10, true);
    assert!(census(&w).is_empty());
    let mut w = enter(&a, "ENTR_STABLE_0", false, 22, true);
    assert_eq!(census(&w), vec![((238, -245), Action::WaitClone), ((355, -245), Action::WaitClone)]);
    idle(&mut w, 5);
    for h in horses(&w) {
        let x = horse(&w, h);
        // func_80A6C6B0: never culled; speed 0 (immovable).
        let f = ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED;
        assert_eq!(x.actor.flags & f, f);
        assert_eq!((x.actor.speed_xz, x.actor.col_chk_info.mass), (0.0, MASS_IMMOVABLE));
        // The stable's rot.z (170) isn't zeroed: drawn as its roll.
        assert_eq!(x.actor.shape_rot.z, 170);
        // func_80A6C8E0 on the flat floor: atan2(0, 30), no pitch.
        assert_eq!((x.unk_220, x.actor.shape_rot.x), (0.0, 0));
        // The clone's cylinder at its place (60 by 100): (355, 0, -245)'s at (235, 0, 100),
        // (238, 0, -245)'s at (478, 0, 100).
        let want = if x.actor.world_pos.x == 355.0 { [235, 0, 100] } else { [478, 0, 100] };
        let c = &x.clone_collider.dim;
        assert_eq!((c.pos, c.radius, c.height), (want, 60, 100));
        assert_eq!(EnHorseNormal::clone_place(SCENE_STABLE, x.actor.world_pos).1, 0x7FFF);
    }
    // EnHorseNormal_WaitClone: at each animation's end the idle (0.4), the whinny (0.4) or the
    // rearing; for the whinny and the rearing their voice now (at unk_204) and the clone's
    // (unk_1E4 0x20) from the same frame's draw (func_80A6CC88), which clears 0x20.
    let mut voices = 0;
    let mut last: Vec<i32> = horses(&w).iter().map(|&h| horse(&w, h).animation_idx).collect();
    for _ in 0..1500 {
        idle(&mut w, 1);
        let frame = w.audio.frames;
        let hs = horses(&w);
        for (i, &h) in hs.iter().enumerate() {
            let x = horse(&w, h);
            assert!(matches!(x.animation_idx, 0 | 1 | 3), "{}", x.animation_idx);
            assert_eq!(x.unk_1e4 & 0x20, 0, "the draw played the clone's voice");
            if x.animation_idx != last[i] && x.animation_idx != 0 {
                voices += 1;
                let sfx = if x.animation_idx == 1 { NA_SE_EV_HORSE_GROAN } else { NA_SE_EV_HORSE_NEIGH };
                assert!(sfx_at(&w, sfx, frame) >= 2, "the horse's and the clone's");
            }
            last[i] = x.animation_idx;
        }
    }
    assert!(voices > 0);
}

#[test]
fn the_ranchs_clones_stand_at_their_places() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_LON_LON_RANCH_0", false, 10, false);
    idle(&mut w, 3);
    for h in horses(&w) {
        let x = horse(&w, h);
        if x.action != Action::WaitClone {
            continue;
        }
        // (-730, 0, -1100)'s clone at (780, 0, -80), (880, 0, -1170)'s at (-1000, 0, -70), both
        // facing 0.
        let (want, yaw) = if x.actor.world_pos.x == -730.0 { ([780, 0, -80], 0) } else { ([-1000, 0, -70], 0) };
        assert_eq!(x.clone_collider.dim.pos, want);
        assert_eq!(EnHorseNormal::clone_place(SCENE_LON_LON_RANCH, x.actor.world_pos).1, yaw);
    }
}

#[test]
fn the_galloper_follows_path_0() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_LON_LON_RANCH_0", false, 10, true);
    let h = horses(&w).into_iter().find(|&h| horse(&w, h).action == Action::FollowPath).expect("the galloper");
    let points: Vec<Vec3> = {
        let p = &w.setup_path_list()[0];
        (0..p.count()).map(|i| p.point(i)).collect()
    };
    assert_eq!(points.len(), 16);
    assert_eq!(points[0], Vec3::new(736.0, 1.0, 429.0));
    // func_80A6B91C: the gallop (6) at 7, from waypoint 0, never update-culled.
    let x = horse(&w, h);
    assert_eq!((x.animation_idx, x.actor.speed_xz, x.waypoint), (6, 7.0, 0));
    assert!(x.skel.is("gHorseNormalGallopingAnim"));
    assert_ne!(x.actor.flags & ACTOR_FLAG_UPDATE_CULLING_DISABLED, 0);
    // EnHorseNormal_FollowPath: the next point once she's within sqrt(600) of it (across the
    // ground, from where the frame starts), in order, round and round.
    let mut reached = vec![];
    for _ in 0..6000 {
        let (before, wp) = (horse(&w, h).actor.world_pos, horse(&w, h).waypoint);
        idle(&mut w, 1);
        let x = horse(&w, h);
        assert_eq!(x.actor.speed_xz, 7.0);
        if x.waypoint != wp {
            assert_eq!(x.waypoint, (wp + 1) % 16);
            let p = points[wp as usize];
            let (dx, dz) = (p.x - before.x, p.z - before.z);
            assert!(dx * dx + dz * dz < 600.0, "point {wp} reached from {before}");
            reached.push(wp);
        } else {
            let moved = ((x.actor.world_pos.x - before.x).powi(2) + (x.actor.world_pos.z - before.z).powi(2)).sqrt();
            assert!(moved <= 7.0 * RATE + 0.01);
        }
        if reached.len() > 17 {
            break;
        }
    }
    // A lap: 0 to 15, then 0 again.
    assert!(reached.len() > 17, "{reached:?}");
    assert_eq!(&reached[..17], &(0..17).map(|i| i % 16).collect::<Vec<_>>()[..]);
}

#[test]
fn the_wanderers_keep_their_limits() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_LON_LON_RANCH_0", false, 10, true);
    let wanderers: Vec<ActorHandle> = horses(&w).into_iter().filter(|&h| horse(&w, h).action == Action::Wander).collect();
    assert_eq!(wanderers.len(), 3);
    // FLAGS 0: updated only inside their culling volume. Link 300 south of the one at
    // (821, 0, -283), facing it.
    let seen = wanderers.iter().copied().find(|&h| horse(&w, h).actor.home_pos.x == 821.0).unwrap();
    w.place_player(Vec3::new(821.0, 0.0, -583.0), 0);
    let mut moved = false;
    for _ in 0..2000 {
        idle(&mut w, 1);
        for &h in &wanderers {
            let x = horse(&w, h);
            // EnHorseNormal_Wander: speed within 0 and 8; the turn rate within 300, its drift
            // within 50; standing (the idle, the whinny) at 0; walking under 3, trotting from 3,
            // galloping from 6 (the walk taken at the end of a standing animation a frame before
            // its speed rises from 0).
            let s = x.actor.speed_xz;
            assert!((0.0..=8.0).contains(&s), "{s}");
            assert!(x.unk_21c.abs() <= 300 && x.unk_21e.abs() <= 50, "{} {}", x.unk_21c, x.unk_21e);
            match x.animation_idx {
                0 | 1 => assert_eq!(s, 0.0),
                4 => assert!(s < 3.0),
                5 => assert!((3.0..6.0).contains(&s)),
                6 => assert!(s >= 6.0),
                i => panic!("animation {i}"),
            }
            // EnHorseNormal_Update: immovable while it stands.
            assert_eq!(x.actor.col_chk_info.mass, if s == 0.0 { MASS_IMMOVABLE } else { MASS_HEAVY });
            moved |= h == seen && s > 0.0;
        }
    }
    assert!(moved, "a 1 in 4 chance at each standing animation's end to walk off");
}

/// The limbs' parents (the skeleton's child and sibling links).
fn parents(s: &SkinSkeleton) -> Vec<Option<usize>> {
    let mut p = vec![None; s.limbs.len()];
    for (i, l) in s.limbs.iter().enumerate() {
        let mut c = l.child;
        while c != 0xFF {
            p[c as usize] = Some(i);
            c = s.limbs[c as usize].sibling;
        }
    }
    p
}

/// Limb `limb`'s point `offset` in the world, worked out with glam in the C's order:
/// `Skin_ApplyAnimTransformations` (each limb `T(jointPos) * Rz * Ry * Rx`, the root at the
/// joint table's translation, under its parent's) and the skin's matrix
/// (`T(pos + yOffset * scale.y) * Ry * Rx * Rz * S(scale)`).
fn limb_world(s: &SkinSkeleton, joints: &[[i16; 3]], pos: Vec3, rot: [i16; 3], scale: Vec3, limb: usize, offset: Vec3) -> Vec3 {
    let r = |a: i16| eng_math::binang_to_rad(a);
    let parents = parents(s);
    let local = |i: usize| {
        let t = if i == 0 { joints[0] } else { s.limbs[i].joint_pos };
        let j = joints[i + 1];
        Mat4::from_translation(Vec3::new(t[0] as f32, t[1] as f32, t[2] as f32)) * Mat4::from_rotation_z(r(j[2])) * Mat4::from_rotation_y(r(j[1])) * Mat4::from_rotation_x(r(j[0]))
    };
    let mut m = local(limb);
    let mut i = limb;
    while let Some(p) = parents[i] {
        m = local(p) * m;
        i = p;
    }
    let skin = Mat4::from_translation(pos) * Mat4::from_rotation_y(r(rot[1])) * Mat4::from_rotation_x(r(rot[0])) * Mat4::from_rotation_z(r(rot[2])) * Mat4::from_scale(scale);
    (skin * m).transform_point3(offset)
}

#[test]
fn the_head_sphere_is_on_limb_11() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_STABLE_0", false, 22, true);
    idle(&mut w, 30);
    for h in horses(&w) {
        let x = horse(&w, h);
        assert!(x.actor.is_drawn);
        // sJntSphElementsInit: limb 11, centre (0, 0, 0), radius 20, scale 100; the world centre
        // Skin_GetLimbPos's (truncated to s16), the horse pitched and rolled as drawn.
        let e = &x.head_collider.elements[0];
        let want = limb_world(&x.skeleton, &x.skel.joint_table, x.actor.world_pos, [x.actor.shape_rot.x, x.actor.shape_rot.y, x.actor.shape_rot.z], x.actor.scale, 11, Vec3::ZERO);
        let got = Vec3::new(e.dim.world_sphere.center[0] as f32, e.dim.world_sphere.center[1] as f32, e.dim.world_sphere.center[2] as f32);
        assert!((got - want).abs().max_element() <= 1.0, "{got} vs {want}");
        assert_eq!((e.dim.limb, e.dim.world_sphere.radius), (11, 20));
        // Facing +z (yaw 0): the head ahead of it, off the ground.
        assert!(got.z > x.actor.world_pos.z + 40.0 && got.y > 30.0, "{got}");
    }
}
