//! `En_Horse_Link_Child` (`z_en_horse_link_child.c`): young Epona in Lon Lon Ranch's paddock
//! (spot20 layer 0's placement: params 0xFFFF at (-4, 0, -572) facing 0x4000, rot.z 3), as a child
//! by day. Entered by `ENTR_LON_LON_RANCH_0` the scene's actors first update on the third frame:
//! the tests place Link after two.
//!
//! Distances move 1.5 times the speed a frame (`Actor_UpdatePos`: `R_UPDATE_RATE * 0.5`), and the
//! animations 1.5 times their play speed. Expected values are worked out from the C in the
//! comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::{Mat4, Vec3};
use oot_actors::PlayExt;
use oot_actors::en_horse_link_child::*;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_UPDATE_DURING_OCARINA};
use oot_game::actor_ctx::ActorHandle;
use oot_game::collision_check::MASS_HEAVY;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::skin::SkinSkeleton;
use oot_game::target::yaw_to;

/// The placement's home.
const HOME: Vec3 = Vec3::new(-4.0, 0.0, -572.0);
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

/// The ranch entered by `entrance` as a child at 10:00, with the sound log, Talon back from the
/// castle (the `castle-courtyard` preset: `EVENTCHKINF_TALON_RETURNED_FROM_CASTLE`) if `talon`,
/// `edit` applied last; two frames on (the actors spawned, not yet updated).
fn ranch(a: &Arc<GameAssets>, entrance: &str, talon: bool, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    if talon {
        save.apply_preset("castle-courtyard").unwrap();
    }
    edit(&mut save);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    w
}

fn foals(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnHorseLinkChild>(h).is_some_and(|f| !f.actor.killed)).collect()
}

fn foal_handle(w: &PlayState) -> ActorHandle {
    let v = foals(w);
    assert_eq!(v.len(), 1, "one live En_Horse_Link_Child");
    v[0]
}

fn foal(w: &PlayState) -> &EnHorseLinkChild {
    w.actors.downcast::<EnHorseLinkChild>(foal_handle(w)).unwrap()
}

fn foal_mut(w: &mut PlayState) -> &mut EnHorseLinkChild {
    let h = foal_handle(w);
    w.actors.downcast_mut::<EnHorseLinkChild>(h).unwrap()
}

fn sfx_count(w: &PlayState, id: u16, since: u32) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(f, s, _)| s == id && f > since).count()
}

fn xz_dist(a: Vec3, b: Vec3) -> f32 {
    ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

#[test]
fn she_is_there_only_after_talons_return() {
    let Some(a) = assets() else { return };
    // EnHorseLinkChild_Init on the ranch, not a cutscene layer: Actor_Kill without
    // EVENTCHKINF_TALON_RETURNED_FROM_CASTLE.
    let w = ranch(&a, "ENTR_LON_LON_RANCH_0", false, |_| {});
    assert!(foals(&w).is_empty(), "no foal before Talon is back");

    let w = ranch(&a, "ENTR_LON_LON_RANCH_0", true, |_| {});
    let f = foal(&w);
    // func_80A69EC0: action 3, the idle, speed 0; rot.z zeroed everywhere; scale 0.005, gravity
    // -3.5; the profile's flags; CollisionCheck_SetInfo's mass.
    assert_eq!((f.action, f.animation_idx, f.actor.speed_xz), (ACTION_RANCH, 0, 0.0));
    assert_eq!((f.actor.home_rot.z, f.actor.world_rot.z, f.actor.shape_rot.z), (0, 0, 0));
    assert_eq!((f.actor.world_pos, f.actor.world_rot.y), (HOME, 0x4000));
    assert_eq!((f.actor.scale, f.actor.gravity), (Vec3::splat(0.005), -3.5));
    assert_eq!(f.actor.flags & (ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA), ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA);
    assert_eq!((f.actor.col_chk_info.mass, f.actor.col_chk_info.cyl_radius, f.actor.col_chk_info.cyl_height), (MASS_HEAVY, 35, 100));
    assert!(!f.unk_2a0, "EVENTCHKINF_CAN_LEARN_EPONAS_SONG unset");
    assert!(f.skel.is("gChildEponaIdleAnim"));
}

#[test]
fn before_malons_talk_she_stands_idling_and_whinnying() {
    let Some(a) = assets() else { return };
    let mut w = ranch(&a, "ENTR_LON_LON_RANCH_0", true, |_| {});
    // Link 150 east of her, facing her.
    w.place_player(HOME + Vec3::new(150.0, 0.0, 0.0), -0x4000);
    let mut anims = vec![0];
    let mut eyes = vec![];
    for _ in 0..400 {
        idle(&mut w, 1);
        let f = foal(&w);
        // func_80A6A068 without EVENTCHKINF_TALKED_TO_CHILD_MALON_AT_RANCH: speed 0, the idle (0)
        // and the whinny (1) in turn, each to its end.
        assert_eq!((f.action, f.actor.speed_xz), (ACTION_RANCH, 0.0));
        assert!(xz_dist(f.actor.world_pos, HOME) < 1e-3, "she stays: {}", f.actor.world_pos);
        if *anims.last().unwrap() != f.animation_idx {
            anims.push(f.animation_idx);
        }
        eyes.push(f.eye_tex_index);
    }
    // The idle and the whinny are 70 frames, played at 1 (1.5 a frame): about 47 frames each.
    assert!(anims.len() >= 6, "{anims:?}");
    assert!(anims.iter().enumerate().all(|(i, &x)| x == (i % 2) as i32), "idle, whinny, idle, ...: {anims:?}");
    // The blink: once started, 1, 2, 3 (sEyeIndexOrder's 1, 2, 1), then 0, a frame each.
    for w2 in eyes.windows(2) {
        if w2[0] > 0 {
            assert_eq!(w2[1], (w2[0] + 1) % 4, "{eyes:?}");
        }
    }
    assert!(eyes.iter().any(|&e| e > 0), "a 1 in 40 chance a frame: she blinks in 400 frames");
}

#[test]
fn after_malons_talk_she_runs_from_link_near_her_home() {
    let Some(a) = assets() else { return };
    let mut w = ranch(&a, "ENTR_LON_LON_RANCH_0", true, |s| s.set_event_chk_inf(EVENTCHKINF_TALKED_TO_CHILD_MALON_AT_RANCH));
    // Malon (En_Ma1) sings 68 east of her home: the foal turned west, away from her, and Link 150
    // north of her (and of her home), facing north (0).
    foal_mut(&mut w).actor.world_rot.y = -0x4000;
    let link = HOME + Vec3::new(0.0, 0.0, 150.0);
    w.place_player(link, 0);
    idle(&mut w, 1);
    // The idle (0) is checked every frame: Link within 250 of her home and within 200 of her:
    // the gallop (4) at 6. (func_80A69F5C ran first, while she stood: no turn yet.)
    let f = foal(&w);
    assert_eq!((f.action, f.animation_idx, f.actor.speed_xz, f.actor.world_rot.y), (ACTION_RANCH, 4, 6.0, -0x4000));
    assert!(f.skel.is("gChildEponaGallopingAnim"));
    // The gallop played at speed / 5 * 1.5 (D_80A6AF64[4]).
    assert!((f.skel.play_speed - 6.0 / 5.0 * 1.5).abs() < 1e-6);
    // func_80A69F5C while she moves, Link within 250 of her home: yawDiff = Link's facing (0)
    // + 0x4000 when the yaw from her to Link is positive, - 0x4000 otherwise (the @bug: her own
    // yaw isn't taken from it), so she turns 0x12C a frame one way or the other, whichever way
    // she faces. She moves 6 * 1.5 a frame.
    let mut frames = 0;
    for _ in 0..30 {
        let f = foal(&w);
        if f.animation_idx != 4 {
            break;
        }
        let (pos, yaw) = (f.actor.world_pos, f.actor.world_rot.y);
        let toward = yaw_to(pos, w.player().actor.world_pos);
        idle(&mut w, 1);
        let f = foal(&w);
        let want = if toward > 0 { yaw.wrapping_add(0x12C) } else { yaw.wrapping_sub(0x12C) };
        assert_eq!(f.actor.world_rot.y, want, "the yaw towards Link {toward:#x}");
        assert_eq!(f.actor.shape_rot.y, f.actor.world_rot.y);
        // (Nothing in her way for her first ten frames: later the walls may push her.)
        if frames < 10 {
            assert!((xz_dist(f.actor.world_pos, pos) - 6.0 * RATE).abs() < 0.01, "{} to {}", pos, f.actor.world_pos);
        }
        frames += 1;
    }
    assert!(frames >= 10, "{frames} frames of the gallop");

    // Link 240 from her (and her home): the trot (3) at 4.
    let mut w = ranch(&a, "ENTR_LON_LON_RANCH_0", true, |s| s.set_event_chk_inf(EVENTCHKINF_TALKED_TO_CHILD_MALON_AT_RANCH));
    w.place_player(HOME + Vec3::new(0.0, 0.0, 240.0), 0);
    idle(&mut w, 1);
    let f = foal(&w);
    assert_eq!((f.animation_idx, f.actor.speed_xz), (3, 4.0));
    // The trot played at speed / 3 * 1.5.
    assert!((f.skel.play_speed - 4.0 / 3.0 * 1.5).abs() < 1e-6);
}

#[test]
fn after_malons_talk_with_link_away_she_goes_home() {
    let Some(a) = assets() else { return };
    let mut w = ranch(&a, "ENTR_LON_LON_RANCH_0", true, |s| s.set_event_chk_inf(EVENTCHKINF_TALKED_TO_CHILD_MALON_AT_RANCH));
    // Link at the entrance's spawn, thousands away. She stands at home: within 70 of it.
    idle(&mut w, 10);
    assert_eq!((foal(&w).actor.speed_xz, xz_dist(foal(&w).actor.world_pos, HOME) < 1e-3), (0.0, true));
    // Moved 400 north of her home, facing east: the gallop at 6 (func_80A69F5C ran first, while
    // she stood), then turning (0x12C a frame) to her home (yaw 0x8000 from there).
    let start = HOME + Vec3::new(0.0, 0.0, 400.0);
    foal_mut(&mut w).actor.world_pos = start;
    idle(&mut w, 1);
    let f = foal(&w);
    assert_eq!((f.animation_idx, f.actor.speed_xz, f.actor.world_rot.y), (4, 6.0, 0x4000));
    idle(&mut w, 1);
    assert_eq!(foal(&w).actor.world_rot.y, 0x4000 + 0x12C);
    // Her pace by her distance from home at each animation's end; within 70 she stands.
    let mut nearest = f32::MAX;
    for _ in 0..1500 {
        idle(&mut w, 1);
        let f = foal(&w);
        nearest = nearest.min(xz_dist(f.actor.world_pos, HOME));
        if f.actor.speed_xz == 0.0 {
            break;
        }
    }
    let f = foal(&w);
    assert_eq!((f.action, f.actor.speed_xz), (ACTION_RANCH, 0.0), "home");
    assert!(xz_dist(f.actor.world_pos, HOME) < 70.0, "{} (nearest {nearest})", f.actor.world_pos);
    assert!(matches!(f.animation_idx, 0 | 1));
}

#[test]
fn eponas_song_turns_her_to_link_and_again_brings_her_to_him() {
    let Some(a) = assets() else { return };
    // Without EVENTCHKINF_CAN_LEARN_EPONAS_SONG the song is nothing to her.
    let mut w = ranch(&a, "ENTR_LON_LON_RANCH_0", true, |_| {});
    w.msg_ctx.r_eponas_song_played = true;
    idle(&mut w, 1);
    assert_eq!(foal(&w).action, ACTION_RANCH);

    let mut w = ranch(&a, "ENTR_LON_LON_RANCH_0", true, |s| s.set_event_chk_inf(EVENTCHKINF_CAN_LEARN_EPONAS_SONG));
    // Link 150 west of her, behind her (she faces east).
    let link = HOME - Vec3::new(150.0, 0.0, 0.0);
    w.place_player(link, 0x4000);
    w.msg_ctx.r_eponas_song_played = true;
    idle(&mut w, 1);
    // func_80A6A4DC: action 5, the idle or the whinny, R_EPONAS_SONG_PLAYED cleared.
    let f = foal(&w);
    assert_eq!(f.action, ACTION_FACE_LINK);
    assert!(matches!(f.animation_idx, 0 | 1));
    assert!(!w.msg_ctx.r_eponas_song_played);
    // func_80A6A5A4: in place (speed 0); at the animation's end, Link behind her (cos < 0): the
    // walk (at D_80A6AF64[2], 1.5), turning to him (Horse_RotateToPoint, 300 a frame) while he's
    // more than 45 degrees off; once he isn't behind her, idling or whinnying again.
    let mut walked = false;
    for _ in 0..400 {
        let yaw = foal(&w).actor.world_rot.y;
        idle(&mut w, 1);
        let f = foal(&w);
        assert_eq!((f.action, f.actor.speed_xz), (ACTION_FACE_LINK, 0.0));
        assert!(xz_dist(f.actor.world_pos, HOME) < 1e-3, "in place");
        assert!((f.actor.world_rot.y.wrapping_sub(yaw) as i32).abs() <= 300);
        if f.animation_idx == 2 {
            walked = true;
            assert!(f.skel.is("gChildEponaWalkingAnim") && f.skel.play_speed == 1.5);
        }
    }
    assert!(walked);
    let f = foal(&w);
    let off = yaw_to(f.actor.world_pos, link).wrapping_sub(f.actor.world_rot.y);
    assert!(eng_math::cos_s(off) >= 0.0, "she's turned to Link: {off:#x} off");
    assert!(matches!(f.animation_idx, 0 | 1));

    // Played again: a neigh, and func_80A6A724: to Link (action 4), the walk at 2.
    let frame = w.audio.frames;
    w.msg_ctx.r_eponas_song_played = true;
    idle(&mut w, 1);
    let f = foal(&w);
    assert_eq!((f.action, f.animation_idx, f.actor.speed_xz, f.timer), (ACTION_COME_TO_LINK, 2, 2.0, 0));
    assert!(!w.msg_ctx.r_eponas_song_played);
    assert_eq!(sfx_count(&w, NA_SE_EV_KID_HORSE_NEIGH, frame), 1);
}

#[test]
fn the_ranchs_cs_index_1_turns_her_to_link() {
    let Some(a) = assets() else { return };
    let mut w = ranch(&a, "ENTR_LON_LON_RANCH_0", true, |_| {});
    // On the ranch with cutsceneIndex CS_INDEX_1 (the layer chosen already): func_80A6A4DC.
    w.save.cutscene_index = CS_INDEX_1;
    idle(&mut w, 1);
    assert_eq!(foal(&w).action, ACTION_FACE_LINK);
}

#[test]
fn entered_by_eponas_songs_end_she_comes_to_link() {
    let Some(a) = assets() else { return };
    let mut w = ranch(&a, "ENTR_LON_LON_RANCH_1", true, |_| {});
    // Link 500 east of her home.
    let link = HOME + Vec3::new(500.0, 0.0, 0.0);
    w.place_player(link, -0x4000);
    let frame = w.audio.frames;
    idle(&mut w, 1);
    // func_80A6A068 with entranceIndex ENTR_LON_LON_RANCH_1: the neigh, func_80A6A724 (action 4,
    // the walk at 2, timer 0), and nothing else this frame (it returns).
    let f = foal(&w);
    assert_eq!((f.action, f.animation_idx, f.actor.speed_xz, f.timer, f.unk_1e8), (ACTION_COME_TO_LINK, 2, 2.0, 0, false));
    assert_eq!(sfx_count(&w, NA_SE_EV_KID_HORSE_NEIGH, frame), 1);
    // The walk's first loop: its hooves past frames 1 and 19 (func_80A693D0), twice.
    let frame = w.audio.frames;
    let mut last = foal(&w).skel.cur_frame;
    loop {
        idle(&mut w, 1);
        let cur = foal(&w).skel.cur_frame;
        if cur < last || foal(&w).animation_idx != 2 {
            break;
        }
        last = cur;
    }
    assert_eq!(sfx_count(&w, NA_SE_EV_KID_HORSE_WALK, frame), 2);
    // func_80A6A7D0: to Link (Horse_RotateToPoint) for 300 frames, by his distance at each
    // animation's end (galloping from 300, trotting from 150, else walking: never standing).
    let mut nearest = f32::MAX;
    while foal(&w).timer < 300 {
        idle(&mut w, 1);
        let f = foal(&w);
        nearest = nearest.min(xz_dist(f.actor.world_pos, link));
        assert!(!f.unk_1e8);
        assert!(matches!(f.animation_idx, 2..=4) && f.actor.speed_xz > 0.0);
    }
    assert!(nearest < 150.0, "she came to Link: {nearest}");
    // Frame 301: home-wards; within 70 of home at an animation's end, func_80A6A4DC (action 5).
    idle(&mut w, 1);
    assert!(foal(&w).unk_1e8);
    for _ in 0..1500 {
        if foal(&w).action != ACTION_COME_TO_LINK {
            break;
        }
        idle(&mut w, 1);
    }
    let f = foal(&w);
    assert_eq!(f.action, ACTION_FACE_LINK);
    assert!(f.actor.world_pos.distance(HOME) < 70.0 + 6.0 * RATE, "{}", f.actor.world_pos);
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
fn her_head_sphere_is_on_limb_13() {
    let Some(a) = assets() else { return };
    let mut w = ranch(&a, "ENTR_LON_LON_RANCH_0", true, |_| {});
    // Link south of her, facing her: she's drawn (the post-draw runs from the draw).
    w.place_player(HOME - Vec3::new(0.0, 0.0, 200.0), 0);
    idle(&mut w, 20);
    let f = foal(&w);
    assert!(f.actor.is_drawn);
    // sJntSphElementsInit: limb 13, centre (0, 0, 0), radius 10, scale 100 (1.0); the world
    // centre is Skin_GetLimbPos's (truncated to s16).
    let e = &f.head_collider.elements[0];
    let want = limb_world(&f.skeleton, &f.skel.joint_table, f.actor.world_pos, [f.actor.shape_rot.x, f.actor.shape_rot.y, f.actor.shape_rot.z], f.actor.scale, 13, Vec3::ZERO);
    let got = Vec3::new(e.dim.world_sphere.center[0] as f32, e.dim.world_sphere.center[1] as f32, e.dim.world_sphere.center[2] as f32);
    assert!((got - want).abs().max_element() <= 1.0, "{got} vs {want}");
    assert_eq!((e.dim.limb, e.dim.world_sphere.radius), (13, 10));
    // Her head is ahead of her (she faces +x) and above her back.
    assert!(got.x > f.actor.world_pos.x + 10.0 && got.y > 20.0, "{got}");
    // The body's cylinder at her place (Collider_UpdateCylinder), 20 by 100.
    let c = &f.body_collider.dim;
    assert_eq!((c.pos, c.radius, c.height), ([HOME.x as i16, HOME.y as i16, HOME.z as i16], 20, 100));
}
