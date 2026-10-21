//! GAME-10 milestone 4 against the C: the Stinger in water (`z_en_weiyer.c`). Jabu-Jabu's Belly
//! (Master Quest) places one in room 1 at (-119, -1246, -1691), over the pit's water (surface
//! -1248, floor -1553), and two in room 13 at (-987, -1203, -2604) and (-871, -1163, -2316)
//! (surface -1153, floor -1233).
//!
//! Entered by `ENTR_JABU_JABU_0` on a `jabu-fish` save, the room loaded with
//! `Room_RequestNewRoom`, Link placed out of view (the Stingers aren't updated by the frames).
//! Each test runs one Stinger's update alone (`Actor_UpdateAll`'s call), Link's distances and
//! place set as `Actor_UpdateAll` would leave them, its hits put on its collider as the collision
//! check leaves them, so no fight is scripted and nothing waits on `Rand`. Expected values are
//! worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_eiyer::{ANIM_HIT, ANIM_POP_OUT, NA_SE_EN_EIER_ATTACK, NA_SE_EN_EIER_DEAD, NAVI_ENEMY_STINGER};
use oot_actors::en_okuta::{NA_SE_EN_OCTAROCK_JUMP, NA_SE_EN_OCTAROCK_SINK};
use oot_actors::en_weiyer::*;
use oot_game::actor::*;
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::audio::sfx::{NA_SE_EN_GOMA_JR_FREEZE, SfxPos};
use oot_game::collision_check::*;
use oot_game::effect::EFFECT_SS_G_SPLASH;
use oot_game::env::clock_time;
use oot_game::play::{DrawOut, PlayState, ViewInfo, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::shadow::ShadowDraw;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// The placements (`ootx scene-info --scene bdan`, layers 0 to 3).
const ROOM1: Vec3 = Vec3::new(-119.0, -1246.0, -1691.0);
const ROOM13: [Vec3; 2] = [Vec3::new(-987.0, -1203.0, -2604.0), Vec3::new(-871.0, -1163.0, -2316.0)];
/// The water's surface over them (room 1's pit, room 13's pool).
const SURFACE1: f32 = -1248.0;
const SURFACE13: f32 = -1153.0;

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// Child Link in `room` (`jabu-fish`), out of view of the Stingers, three frames on, the audio
/// logged.
fn jabu(a: &Arc<GameAssets>, room: i8) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_JABU_JABU_0").expect("entrance");
    let mut save = SaveContext::new(e, false, clock_time(10, 0) as u16);
    save.apply_preset("jabu-fish").unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    w.audio.log = Some(Default::default());
    idle(&mut w, 2);
    assert!(w.room_request(room));
    idle(&mut w, 1);
    w.room_change_done();
    w.place_player(Vec3::ZERO, 0);
    idle(&mut w, 3);
    assert_eq!(w.room_ctx.cur.num, room);
    w
}

fn weiyers(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnWeiyer>(h).is_some_and(|o| !o.actor.killed)).collect()
}

fn at_home(w: &PlayState, home: Vec3) -> ActorHandle {
    *weiyers(w).iter().find(|&&h| wy(w, h).actor.home_pos.x == home.x && wy(w, h).actor.home_pos.z == home.z).expect("the Stinger")
}

fn wy(w: &PlayState, h: ActorHandle) -> &EnWeiyer {
    w.actors.downcast::<EnWeiyer>(h).expect("En_Weiyer")
}

fn wy_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnWeiyer {
    w.actors.downcast_mut::<EnWeiyer>(h).expect("En_Weiyer")
}

fn gone(w: &PlayState, h: ActorHandle) -> bool {
    w.actors.downcast::<EnWeiyer>(h).is_none_or(|o| o.actor.killed)
}

/// The actor's update alone (`Actor_UpdateAll`'s call), as the updating actor.
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// `Actor_UpdateAll`'s distances: Link `dist` across at `yaw`, `y` above it.
fn set_link(w: &mut PlayState, h: ActorHandle, dist: f32, y: f32, yaw: i16) {
    let o = wy_mut(w, h);
    o.actor.xz_dist_to_player = dist;
    o.actor.y_dist_to_player = y;
    o.actor.yaw_towards_player = yaw;
}

/// Link's place and his depth in water (`depthInWater`).
fn place_link(w: &mut PlayState, pos: Vec3, depth: f32) {
    let p = w.player.unwrap();
    let a = w.actors.actor_mut(p).unwrap();
    a.world_pos = pos;
    a.y_dist_to_water = depth;
}

/// A hit as the collision check leaves it: `AC_HIT`, the damage table's damage and reaction.
fn hit(w: &mut PlayState, h: ActorHandle, damage: u8, reaction: u8) {
    let o = wy_mut(w, h);
    o.collider.base.ac_flags |= AC_HIT;
    o.actor.col_chk_info.damage = damage;
    o.actor.col_chk_info.damage_reaction = reaction;
}

fn effects(w: &PlayState, ty: u8) -> usize {
    w.effect_ss.table.iter().filter(|e| e.ty == ty && e.life >= 0).count()
}

fn sfx(w: &PlayState, h: ActorHandle, id: u16) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, p)| s == id && p == SfxPos::Actor(h)).count()
}

/// Its draw: the meshes, opaque and translucent.
fn draw(w: &PlayState, h: ActorHandle) -> (Vec<String>, Vec<String>) {
    let o = wy(w, h);
    let rs = o.render_state();
    let mut out = DrawOut::default();
    o.draw(&rs, w, &ViewInfo::new(Vec3::ZERO, glam::Mat4::IDENTITY), &mut out);
    (out.opa.iter().map(|c| c.mesh.name.clone()).collect(), out.xlu.iter().map(|c| c.mesh.name.clone()).collect())
}

fn until_not(w: &mut PlayState, h: ActorHandle, action: Action, max: usize) -> usize {
    for n in 1..=max {
        update(w, h);
        if gone(w, h) || wy(w, h).action != action {
            return n;
        }
    }
    panic!("still {action:?} after {max}");
}

/// Room 1's, placed (its first update) and swimming, Link far.
fn swimming(w: &mut PlayState) -> ActorHandle {
    let h = at_home(w, ROOM1);
    set_link(w, h, 1000.0, 0.0, 0);
    update(w, h);
    assert_eq!(wy(w, h).action, Action::FreeSwim);
    h
}

#[test]
fn placed_in_the_water_halfway_down_they_swim() {
    let Some(a) = assets() else { return };
    for (room, homes, surface) in [(1, vec![ROOM1], SURFACE1), (13, ROOM13.to_vec(), SURFACE13)] {
        let mut w = jabu(&a, room);
        assert_eq!(weiyers(&w).len(), homes.len());
        for home in homes {
            let h = at_home(&w, home);
            let o = wy(&w, h);
            // sInitChain: NAVI_ENEMY_STINGER, scale 3 / 1000, lockOnArrowOffset 2500; the circle
            // shadow 65 at 1000; sColChkInfoInit 2, 45, 15, 100; sCylinderInit 16 by 10, 6 down,
            // AT off, AC on.
            assert_eq!((o.actor.params, o.action), (0, Action::InitInsideWaterBox));
            assert_eq!((o.actor.navi_enemy_id, o.actor.scale, o.actor.target_arrow_offset), (NAVI_ENEMY_STINGER, Vec3::splat(0.003), 2500.0));
            assert_eq!((o.actor.shape_shadow_draw, o.actor.shape_shadow_scale, o.actor.shape_y_offset), (ShadowDraw::Circle, 65.0, 1000.0));
            let c = &o.actor.col_chk_info;
            assert_eq!((c.health, c.cyl_radius, c.cyl_height, c.mass), (2, 45, 15, 100));
            assert_eq!((o.collider.dim.radius, o.collider.dim.height, o.collider.dim.y_shift), (16, 10, -6));
            assert_eq!((o.collider.base.at_flags & AT_ON, o.collider.base.ac_flags & AC_ON), (0, AC_ON));
            // EnWeiyer_InitInsideWaterBox: the floor under it (half a unit up), home 5 under the
            // surface over it, halfway between.
            update(&mut w, h);
            let o = wy(&w, h);
            let floor = w.col.entity_raycast_down(Vec3::new(home.x, home.y + 0.5, home.z)).0;
            assert_eq!(o.actor.floor_height, floor);
            assert!(floor < surface - 5.0);
            assert_eq!(o.actor.home_pos.y, surface - 5.0);
            assert_eq!(o.actor.world_pos.y, (surface - 5.0 + floor) / 2.0);
            // EnWeiyer_SetupFreeSwim: gStingerHitAnim, 30 frames, its swim height 3 over
            // (cosf(-M_PI / 8) × 3), aiming for the floor.
            assert_eq!((o.action, o.timer, o.target_yaw), (Action::FreeSwim, 30, o.actor.shape_rot.y));
            assert!(o.skel.is(ANIM_HIT));
            assert_eq!(o.swim_height, ((-std::f32::consts::PI / 8.0).cos() * 3.0) + o.actor.world_pos.y);
            assert_eq!(o.target_swim_height, floor);
            assert_eq!(draw(&w, h), (vec!["bake/En_Eiyer/opa".to_string()], vec![]));
            // Its update keeps its home at the surface less 5 (its depth from the last update).
            update(&mut w, h);
            assert_eq!(wy(&w, h).actor.home_pos.y, surface - 5.0);
        }
    }
}

#[test]
fn link_in_its_water_it_attacks_and_backs_off_when_he_climbs_out() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a, 1);
    let h = swimming(&mut w);
    let home_y = SURFACE1 - 5.0;
    // Link 300 across on its level, but 20 or more over the surface (-1253 + 20): it swims on.
    place_link(&mut w, Vec3::new(ROOM1.x, home_y + 20.0, ROOM1.z), 0.0);
    set_link(&mut w, h, 300.0, 0.0, 0x1234);
    update(&mut w, h);
    assert_eq!(wy(&w, h).action, Action::FreeSwim);
    // 400 or more across, or 250 or more up or down: on too.
    place_link(&mut w, Vec3::new(ROOM1.x, -1400.0, ROOM1.z), 100.0);
    for (d, y) in [(400.0, 0.0), (300.0, 250.0), (300.0, -250.0)] {
        set_link(&mut w, h, d, y, 0x1234);
        update(&mut w, h);
        assert_eq!(wy(&w, h).action, Action::FreeSwim, "{d} {y}");
    }
    // In its water within: EnWeiyer_SetupAttack (200 frames, AT on, the animation at 3).
    set_link(&mut w, h, 300.0, 0.0, 0x1234);
    update(&mut w, h);
    let o = wy(&w, h);
    assert_eq!((o.action, o.timer, o.skel.play_speed), (Action::Attack, 200, 3.0));
    assert_ne!(o.collider.base.at_flags & AT_ON, 0);
    // EnWeiyer_Attack: turning to him (a half, 0x200 at most, 0x80 at least), speeding to 1.3
    // not facing him.
    let (yaw, speed) = (o.actor.shape_rot.y, o.actor.speed_xz);
    update(&mut w, h);
    let o = wy(&w, h);
    let (mut y, mut s) = (yaw, speed);
    eng_math::smooth_step_to_s(&mut y, 0x1234, 2, 0x200, 0x80);
    eng_math::step_to_f(&mut s, if (0x1234i16.wrapping_sub(yaw) as i32).abs() < 0x2800 { 4.0 } else { 1.3 }, 0.2);
    assert_eq!((o.timer, o.actor.shape_rot.y, o.actor.world_rot.y, o.actor.speed_xz), (199, y, y, s));
    // Link climbs out (20 over the surface): it backs off (EnWeiyer_SetupInactive: 200 frames
    // away from him, AT off, the animation at 1).
    place_link(&mut w, Vec3::new(ROOM1.x, home_y + 20.0, ROOM1.z), 0.0);
    set_link(&mut w, h, 300.0, 0.0, 0x1234);
    let y = wy(&w, h).actor.world_pos.y;
    update(&mut w, h);
    let o = wy(&w, h);
    assert_eq!((o.action, o.target_yaw, o.skel.play_speed), (Action::Inactive, 0x1234i16.wrapping_add(-0x8000i16), 1.0));
    assert_eq!(o.collider.base.at_flags & AT_ON, 0);
    // (Its height at the setup, before the update's move.)
    assert_eq!((o.timer, o.swim_height), (200, y));
    // EnWeiyer_Inactive: toward the lower quarter off the floor at 1 a frame; 200 frames, then
    // swimming again.
    let o = wy(&w, h);
    let (sh, goal) = (o.swim_height, (home_y - o.actor.floor_height) / 4.0 + o.actor.floor_height);
    update(&mut w, h);
    let mut e = sh;
    eng_math::step_to_f(&mut e, goal, 1.0);
    assert_eq!(wy(&w, h).swim_height, e);
    wy_mut(&mut w, h).timer = 1;
    update(&mut w, h);
    assert_eq!(wy(&w, h).action, Action::FreeSwim);
    // A hit on Link ends an attack too, and plays NA_SE_EN_EIER_ATTACK with AT off.
    place_link(&mut w, Vec3::new(ROOM1.x, -1400.0, ROOM1.z), 100.0);
    wy_mut(&mut w, h).setup_attack();
    wy_mut(&mut w, h).collider.base.at_flags |= AT_HIT;
    update(&mut w, h);
    let o = wy(&w, h);
    assert_eq!(o.action, Action::Inactive);
    assert_eq!(o.collider.base.at_flags & (AT_ON | AT_HIT), 0);
    assert_eq!(sfx(&w, h, NA_SE_EN_EIER_ATTACK), 1);
}

#[test]
fn link_at_the_surface_it_jumps_out_after_him() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a, 1);
    let h = swimming(&mut w);
    let (splashes, jumps) = (effects(&w, EFFECT_SS_G_SPLASH), sfx(&w, h, NA_SE_EN_OCTAROCK_JUMP));
    // Link under 50 deep, it under 20 and facing him (within 0x2000): EnWeiyer_SetupOutOfWater
    // (pitched up 0x2000, at 5).
    let y = wy(&w, h).actor.world_pos.y;
    place_link(&mut w, Vec3::new(ROOM1.x, y - 100.0, ROOM1.z), 10.0);
    wy_mut(&mut w, h).setup_attack();
    let yaw = wy(&w, h).actor.shape_rot.y;
    set_link(&mut w, h, 300.0, 0.0, yaw);
    wy_mut(&mut w, h).actor.y_dist_to_water = 10.0;
    update(&mut w, h);
    let o = wy(&w, h);
    assert_eq!((o.action, o.timer, o.actor.speed_xz), (Action::OutOfWater, -1, 5.0));
    assert_eq!((o.actor.shape_rot.x, o.actor.world_rot.x), (-0x2000, 0x2000));
    // Out of the water (depthInWater under 0): a splash (400) and NA_SE_EN_OCTAROCK_JUMP, ten
    // frames.
    wy_mut(&mut w, h).actor.y_dist_to_water = -1.0;
    let y = wy(&w, h).actor.world_pos.y;
    place_link(&mut w, Vec3::new(ROOM1.x, y - 100.0, ROOM1.z), 10.0);
    update(&mut w, h);
    assert_eq!(wy(&w, h).timer, 10);
    assert_eq!(effects(&w, EFFECT_SS_G_SPLASH), splashes + 1);
    assert_eq!(sfx(&w, h, NA_SE_EN_OCTAROCK_JUMP), jumps + 1);
    // Link out of the water (20 over the surface): no more waiting, it pitches down at 0x1800
    // (0x400 steps); pitched down in the water, a splash, NA_SE_EN_OCTAROCK_SINK, backing off.
    place_link(&mut w, Vec3::new(ROOM1.x, 0.0, ROOM1.z), 0.0);
    let x = wy(&w, h).actor.shape_rot.x;
    update(&mut w, h);
    let mut e = x;
    eng_math::scaled_step_to_s(&mut e, 0x1800, 0x400);
    assert_eq!((wy(&w, h).timer, wy(&w, h).actor.shape_rot.x), (0, e));
    for _ in 0..20 {
        let o = wy_mut(&mut w, h);
        if o.action != Action::OutOfWater {
            break;
        }
        o.actor.bg_check_flags = BGCHECKFLAG_WATER;
        update(&mut w, h);
    }
    let o = wy(&w, h);
    assert_eq!(o.action, Action::Inactive);
    assert!(o.actor.shape_rot.x > 0);
    assert_eq!(effects(&w, EFFECT_SS_G_SPLASH), splashes + 2);
    assert_eq!(sfx(&w, h, NA_SE_EN_OCTAROCK_SINK), 1);
}

#[test]
fn over_the_surface_it_turns_over_and_on_the_ground_it_flops() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a, 1);
    let h = swimming(&mut w);
    // EnWeiyer_SetupTurnAround: AT on, at 5. EnWeiyer_TurnAround: pitching up to 0x4000 (0x800
    // steps), rolled twice its pitch; there, turned round, unrolled.
    wy_mut(&mut w, h).setup_turn_around();
    let o = wy(&w, h);
    assert_ne!(o.collider.base.at_flags & AT_ON, 0);
    let yaw = o.actor.shape_rot.y;
    let mut x = 0i16;
    loop {
        update(&mut w, h);
        let o = wy(&w, h);
        let done = eng_math::scaled_step_to_s(&mut x, -0x4000, 0x800);
        assert_eq!(o.actor.shape_rot.x, x);
        if done {
            assert_eq!((o.timer, o.actor.shape_rot.z, o.actor.shape_rot.y), (1, 0, yaw.wrapping_add(-0x8000i16)));
            break;
        }
        assert_eq!((o.timer, o.actor.shape_rot.z), (0, (x as i32 * 2) as i16));
    }
    // Then pitching down to 0x1800; in the water but still pitched up: backing off, no splash.
    let splashes = effects(&w, EFFECT_SS_G_SPLASH);
    update(&mut w, h);
    assert_eq!((wy(&w, h).action, effects(&w, EFFECT_SS_G_SPLASH)), (Action::Inactive, splashes));
    // Pitched down: a splash and NA_SE_EN_OCTAROCK_SINK.
    wy_mut(&mut w, h).setup_turn_around();
    let o = wy_mut(&mut w, h);
    o.timer = 1;
    o.actor.shape_rot.x = 0x1000;
    update(&mut w, h);
    assert_eq!((wy(&w, h).action, effects(&w, EFFECT_SS_G_SPLASH), sfx(&w, h, NA_SE_EN_OCTAROCK_SINK)), (Action::Inactive, splashes + 1, 1));
    // Out of the water on the ground: stuck (EnWeiyer_SetupStuckOnFloor: gStingerPopOutAnim at
    // 2, 40 frames, AT on).
    wy_mut(&mut w, h).setup_turn_around();
    let o = wy_mut(&mut w, h);
    o.timer = 1;
    o.actor.y_dist_to_water = -10.0;
    o.actor.bg_check_flags = BGCHECKFLAG_GROUND;
    update(&mut w, h);
    let o = wy(&w, h);
    assert_eq!((o.action, o.timer, o.skel.play_speed), (Action::StuckOnFloor, 40, 2.0));
    assert!(o.skel.is(ANIM_POP_OUT));
    assert_ne!(o.collider.base.at_flags & AT_ON, 0);
    // EnWeiyer_StuckOnFloor: the 40 frames out, it turns over again; back in the water, it swims.
    let o = wy_mut(&mut w, h);
    o.timer = 1;
    o.actor.y_dist_to_water = -10.0;
    update(&mut w, h);
    assert_eq!(wy(&w, h).action, Action::TurnAround);
    wy_mut(&mut w, h).setup_stuck_on_floor();
    wy_mut(&mut w, h).actor.y_dist_to_water = 50.0;
    update(&mut w, h);
    assert_eq!(wy(&w, h).action, Action::FreeSwim);
}

#[test]
fn nuts_stun_it_swords_hurt_and_kill_it_and_dead_it_fades() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a, 1);
    let h = swimming(&mut w);
    // A Deku nut (reaction 1): stunned, no damage (EnWeiyer_SetupStunned: blue 80 frames, still,
    // falling, 15 taller, AT off, NA_SE_EN_GOMA_JR_FREEZE).
    hit(&mut w, h, 0, 1);
    update(&mut w, h);
    let o = wy(&w, h);
    assert_eq!((o.action, o.timer, o.actor.speed_xz, o.actor.gravity), (Action::Stunned, 79, 0.0, -1.0));
    assert_eq!((o.collider.dim.height, o.actor.col_chk_info.health), (25, 2));
    assert_eq!(sfx(&w, h, NA_SE_EN_GOMA_JR_FREEZE), 1);
    wy_mut(&mut w, h).timer = 1;
    update(&mut w, h);
    let o = wy(&w, h);
    assert_eq!((o.action, o.actor.gravity, o.collider.dim.height), (Action::FreeSwim, 0.0, 10));
    // The Kokiri sword (1): hurt (red 40 frames, AT and AC off, at 3).
    let yaw = wy(&w, h).actor.shape_rot.y;
    set_link(&mut w, h, 300.0, 0.0, yaw.wrapping_add(0x2000));
    hit(&mut w, h, 1, 0);
    update(&mut w, h);
    let o = wy(&w, h);
    assert_eq!((o.action, o.timer, o.actor.speed_xz, o.actor.col_chk_info.health), (Action::Hurt, 39, 3.0, 1));
    assert_eq!((o.collider.base.at_flags & AT_ON, o.collider.base.ac_flags & AC_ON), (0, 0));
    assert_ne!(o.actor.color_filter_params & COLORFILTER_COLORFLAG_RED, 0);
    // @bug (game): EnWeiyer_Hurt turns world.rot.y away from Link, but the update puts it back
    // to shape.rot.y: it keeps straight on.
    assert_eq!(o.target_yaw, yaw.wrapping_add(0x2000).wrapping_add(-0x8000i16));
    assert_eq!((o.actor.shape_rot.y, o.actor.world_rot.y), (yaw, yaw));
    // Rolling (sinf(timer × M_PI / 5) × 5120); out, AC on, swimming.
    assert_eq!(o.actor.shape_rot.z, ((39.0f32 * (std::f32::consts::PI / 5.0)).sin() * 5120.0) as i16);
    wy_mut(&mut w, h).timer = 1;
    update(&mut w, h);
    let o = wy(&w, h);
    assert_eq!((o.action, o.actor.shape_rot.z), (Action::FreeSwim, 0));
    assert_ne!(o.collider.base.ac_flags & AC_ON, 0);
    // The Master sword (2): no health left (the finishing blow, NA_SE_EN_EIER_DEAD, no lock-on;
    // red 40 frames, at 3, 20 frames).
    hit(&mut w, h, 2, 0);
    update(&mut w, h);
    let o = wy(&w, h);
    assert_eq!((o.action, o.timer, o.actor.speed_xz, o.actor.col_chk_info.health), (Action::Die, 19, 3.0, 0));
    assert_eq!(o.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!(sfx(&w, h, NA_SE_EN_EIER_DEAD), 1);
    // EnWeiyer_Die: 20 frames (or a ceiling), then dead: translucent at its shadow's alpha, 5
    // less and 2 lower a frame; at 0 gone (a drop from table 14).
    assert!(until_not(&mut w, h, Action::Die, 20) <= 19);
    let o = wy(&w, h);
    assert_eq!(o.action, Action::Dead);
    assert_ne!(o.actor.color_filter_params & 0x2000, 0);
    let (alpha, y) = (o.actor.shape_shadow_alpha, o.actor.world_pos.y);
    update(&mut w, h);
    let o = wy(&w, h);
    assert_eq!((o.actor.shape_shadow_alpha, o.actor.world_pos.y), (alpha - 5, y - 2.0));
    assert_eq!(draw(&w, h), (vec![], vec!["bake/En_Eiyer/xlu".to_string()]));
    assert_eq!(until_not(&mut w, h, Action::Dead, 60), (alpha as usize - 5) / 5);
    assert!(gone(&w, h));
}
