//! GAME-10 milestone 5 against the C: the electrified tentacle barriers (`z_en_bx.c`) of
//! Jabu-Jabu's Belly, as Master Quest places them (`ootx scene-info --scene bdan_scene`): room 7's
//! two gray ones (0x0A82: colour 2, the quad, flag 0x0A) at (-655, -340, -4955) and (-840, -340,
//! -4685) turned 16384.
//!
//! Child Link enters at `ENTR_JABU_JABU_0` on a `jabu-boomerang` save, and the room changes to room
//! 7 (`Room_RequestNewRoom`). The tests run a barrier's update alone (`Actor_UpdateAll`'s call),
//! Link's distance set as `Actor_UpdateAll` would; its quad's touch goes through the collision
//! check (`CollisionCheck_AT`) with Link's cylinder put inside it. Expected values are worked out
//! from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::en_bx::*;
use oot_actors::player::Player;
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::sfx::{SFX_FLAG, SfxPos};
use oot_game::collision_check::*;
use oot_game::effect::EFFECT_SS_LIGHTNING;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::shadow::ShadowDraw;
use oot_game::sys_matrix::MtxF;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// Child Link in Jabu-Jabu (`jabu-boomerang`), the sound log on, `flags` set, in room 7, its actors
/// in, Link his own (no cutscene's hold).
fn room_7(a: &Arc<GameAssets>, flags: &[i32]) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_JABU_JABU_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("jabu-boomerang").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    for &f in flags {
        w.flags.set_switch(f);
    }
    assert!(w.room_request(7));
    idle(&mut w, 1);
    w.room_change_done();
    assert_eq!(w.room_ctx.cur.num, 7);
    idle(&mut w, 20);
    for _ in 0..400 {
        if !w.player_in_cs_mode() {
            break;
        }
        idle(&mut w, 1);
    }
    w
}

const WEST: Vec3 = Vec3::new(-840.0, -340.0, -4685.0);
const SOUTH: Vec3 = Vec3::new(-655.0, -340.0, -4955.0);

fn barriers(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnBx>(h).is_some_and(|b| !b.actor.killed)).collect()
}

fn at_home(w: &PlayState, home: Vec3) -> ActorHandle {
    *barriers(w).iter().find(|&&h| bx(w, h).actor.home_pos == home).expect("the barrier")
}

fn bx(w: &PlayState, h: ActorHandle) -> &EnBx {
    w.actors.downcast::<EnBx>(h).expect("En_Bx")
}

fn bx_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnBx {
    w.actors.downcast_mut::<EnBx>(h).expect("En_Bx")
}

fn player(w: &PlayState) -> &Player {
    w.actors.downcast::<Player>(w.player.unwrap()).unwrap()
}

/// The actor's update alone (`Actor_UpdateAll`'s call), as the updating actor.
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

fn draw_update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.draw_update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// The times a sound was asked for at `h`.
fn sfx(w: &PlayState, h: ActorHandle, id: u16) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, p)| s == id && p == SfxPos::Actor(h)).count()
}

fn bolts(w: &PlayState) -> usize {
    w.effect_ss.table.iter().filter(|e| e.ty == EFFECT_SS_LIGHTNING && e.life >= 0).count()
}

/// Nothing touching it: its colliders' hits cleared, Link `dist` across.
fn alone(w: &mut PlayState, h: ActorHandle, dist: f32) {
    let b = bx_mut(w, h);
    b.actor.xz_dist_to_player = dist;
    b.actor.yaw_towards_player = 0x1234;
    for base in [&mut b.collider.base, &mut b.collider_quad.base] {
        base.at_flags &= !AT_HIT;
        base.ac_flags &= !AC_HIT;
        base.at = None;
        base.ac = None;
    }
}

#[test]
fn room_7s_gray_barriers_stand_without_their_flag() {
    let Some(a) = assets() else { return };
    let w = room_7(&a, &[]);
    assert!(!w.flags.get_switch(0x0A));
    assert_eq!(barriers(&w).len(), 2);
    for (home, yaw) in [(WEST, 16384), (SOUTH, 0)] {
        let b = bx(&w, at_home(&w, home));
        // EnBx_Init: params &= 0xFF (colour 2, the quad); 100 under its place; the segments 140
        // apart up from there; lockOnArrowOffset 5300; the circle shadow 48; immovable; culled
        // 2000 down; the cylinder 60 by 100, 100 up.
        assert_eq!((b.actor.params, b.has_quad(), b.actor.world_rot.y), (0x82, true, yaw));
        assert_eq!(b.actor.world_pos, home - Vec3::Y * 100.0);
        for i in 0..4 {
            assert_eq!(b.unk_154[i], Vec3::new(home.x, home.y - 100.0 + (i as f32 + 1.0) * 140.0, home.z));
        }
        assert_eq!(b.actor.target_arrow_offset, 5300.0);
        assert_eq!((b.actor.shape_shadow_draw, b.actor.shape_shadow_scale), (ShadowDraw::Circle, 48.0));
        assert_eq!((b.actor.col_chk_info.mass, b.actor.culling_volume_downward), (MASS_IMMOVABLE, 2000.0));
        assert_eq!((b.collider.dim.radius, b.collider.dim.height, b.collider.dim.y_shift), (60, 100, 100));
        assert_eq!(b.actor.scale.y, 0.03);
    }
}

#[test]
fn with_the_gray_tentacles_flag_they_are_gone() {
    let Some(a) = assets() else { return };
    // EnBx_Init: Flags_GetSwitch(PARAMS_GET_U(params, 8, 8)): Actor_Kill.
    let w = room_7(&a, &[0x0A]);
    assert!(barriers(&w).is_empty());
}

#[test]
fn its_draw_pulses_and_sets_its_quad() {
    let Some(a) = assets() else { return };
    let mut w = room_7(&a, &[]);
    let h = at_home(&w, WEST);
    {
        let b = bx_mut(&mut w, h);
        b.unk_14e = 0;
        b.actor.scale.x = 0.0225;
        b.actor.scale.z = 0.0225;
    }
    draw_update(&mut w, h);
    let b = bx(&w, h);
    // func_809D1D0C under Actor_Draw's matrix (the frame's scale): (∓8000, 10000 and 15000, 2500).
    let r = b.actor.shape_rot;
    let mut m = MtxF::set_translate_rotate_yxz(b.actor.world_pos.x, b.actor.world_pos.y, b.actor.world_pos.z, [r.x, r.y, r.z]);
    m.scale(0.0225, 0.03, 0.0225);
    let q = b.collider_quad.dim.quad;
    assert_eq!(q[0], m.mult_vec3f(Vec3::new(-8000.0, 10000.0, 2500.0)));
    assert_eq!(q[1], m.mult_vec3f(Vec3::new(-8000.0, 15000.0, 2500.0)));
    assert_eq!(q[2], m.mult_vec3f(Vec3::new(8000.0, 10000.0, 2500.0)));
    assert_eq!(q[3], m.mult_vec3f(Vec3::new(8000.0, 15000.0, 2500.0)));
    assert_eq!(b.drawn_scale.x, 0.0225);
    // Then unk_14E -0xBB8: the width 0.015 + 0.0075 cos, each segment's 0x2000 later.
    assert_eq!(b.unk_14e, -0xBB8);
    assert_eq!(b.actor.scale.x, eng_math::cos_s(-0xBB8) * 0.0075 + 0.015);
    for i in 0..4 {
        assert_eq!(b.unk_184[i].x, eng_math::cos_s((-0xBB8 + 0x2000 * i as i32) as i16) * 0.0075 + 0.015);
        assert_eq!(b.unk_1b4[i], [r.x, r.y, r.z]);
    }
}

#[test]
fn its_quad_hurts_link_and_knocks_him_back() {
    let Some(a) = assets() else { return };
    let mut w = room_7(&a, &[]);
    let h = at_home(&w, WEST);
    let ph = w.player.unwrap();
    // The quad as its draw sets it at its widest (8000 × 0.0225 = 180 each side).
    {
        let b = bx_mut(&mut w, h);
        b.actor.scale.x = 0.0225;
        b.actor.scale.z = 0.0225;
        b.set_quad();
    }
    // Link's cylinder 112 to the side in it (5000 along it, 12500 up, 2500 out): over 70 across
    // from the barrier.
    let b = bx(&w, h);
    let r = b.actor.shape_rot;
    let mut m = MtxF::set_translate_rotate_yxz(b.actor.world_pos.x, b.actor.world_pos.y, b.actor.world_pos.z, [r.x, r.y, r.z]);
    m.scale(0.0225, 0.03, 0.0225);
    let in_quad = m.mult_vec3f(Vec3::new(5000.0, 12500.0, 2500.0));
    let dxz = (in_quad.x - b.actor.world_pos.x).hypot(in_quad.z - b.actor.world_pos.z);
    assert!(dxz > 70.0, "{dxz}");
    w.col_chk.clear();
    {
        let mut p = w.actors.take(ph).unwrap();
        let pl = p.as_any_mut().downcast_mut::<Player>().unwrap();
        pl.cylinder.dim.pos = [in_quad.x as i16, (in_quad.y - 20.0) as i16, in_quad.z as i16];
        w.col_chk.set_ac(Some(ph), &pl.actor, 0, &mut pl.cylinder);
        w.actors.put_back(ph, p);
    }
    {
        let mut a = w.actors.take(h).unwrap();
        let b = a.as_any_mut().downcast_mut::<EnBx>().unwrap();
        w.col_chk.set_at(Some(h), &b.actor, COL_QUAD, &mut b.collider_quad);
        w.actors.put_back(h, a);
    }
    let _ = w.col_chk.check(&mut w.actors);
    w.col_chk.clear();
    // CollisionCheck_AT: the quad's 4 with an electric shock on Link's cylinder.
    let b = bx(&w, h);
    assert_ne!(b.collider_quad.base.at_flags & AT_HIT, 0);
    assert_eq!(b.collider_quad.base.at, Some(ph));
    let hit = player(&w).cylinder.info.ac_hit_elem.expect("Link hit");
    assert_eq!((hit.at_dmg_info.damage, hit.at_dmg_info.hit_special_effect), (4, HIT_SPECIAL_EFFECT_ELECTRIC));
    // EnBx_Update with its quad on Link: knocked back along its facing (the quad's: world.rot.y),
    // 6 out, 6 up, no damage of its own (the quad's is Link's own to take); the hits cleared; 20
    // frames of sparks (19 after this one: no bolt at 19, (19 + 2i) % 4 never 0) and
    // NA_SE_EN_BIRI_SPARK.
    let health = w.save.health;
    let bolts0 = bolts(&w);
    bx_mut(&mut w, h).actor.xz_dist_to_player = dxz;
    update(&mut w, h);
    let p = player(&w);
    assert_eq!((p.knockback_type, p.knockback_damage, p.knockback_speed, p.knockback_y_velocity, p.knockback_rot), (2, 0, 6.0, 6.0, 16384));
    assert_eq!(w.save.health, health);
    let b = bx(&w, h);
    assert_eq!((b.collider_quad.base.at_flags & AT_HIT, b.collider_quad.base.at, b.unk_14c), (0, None, 19));
    assert_eq!(bolts(&w), bolts0);
    assert_eq!(sfx(&w, h, NA_SE_EN_BIRI_SPARK - SFX_FLAG), 1);
    // The next frame (18): bolts at corners 1 and 3 ((18 + 2i) % 4 == 0).
    alone(&mut w, h, 1000.0);
    update(&mut w, h);
    assert_eq!(bx(&w, h).unk_14c, 18);
    assert_eq!(bolts(&w), bolts0 + 2);
}

#[test]
fn near_it_link_loses_4_unless_invincible() {
    let Some(a) = assets() else { return };
    let mut w = room_7(&a, &[]);
    let h = at_home(&w, SOUTH);
    // Within 70 across, nothing touching: invincibilityTimer zeroed, play->damagePlayer(-4) (a
    // quarter heart), knocked back along its facing, the timer put back.
    let health = w.save.health;
    alone(&mut w, h, 70.0);
    update(&mut w, h);
    assert_eq!(w.save.health, health - 4);
    let p = player(&w);
    assert_eq!((p.knockback_type, p.knockback_speed, p.knockback_rot, p.invincibility_timer), (2, 6.0, 0, 0));
    // Invincible (10): no damage, knocked back all the same, the timer kept.
    w.actors.downcast_mut::<Player>(w.player.unwrap()).unwrap().invincibility_timer = 10;
    alone(&mut w, h, 50.0);
    update(&mut w, h);
    assert_eq!(w.save.health, health - 4);
    assert_eq!(player(&w).invincibility_timer, 10);
    // At -40 or under: zeroed, no damage, put back.
    w.actors.downcast_mut::<Player>(w.player.unwrap()).unwrap().invincibility_timer = -40;
    alone(&mut w, h, 50.0);
    update(&mut w, h);
    assert_eq!(w.save.health, health - 4);
    assert_eq!(player(&w).invincibility_timer, -40);
    // Over 70 with nothing touching: nothing.
    w.actors.downcast_mut::<Player>(w.player.unwrap()).unwrap().knockback_type = 0;
    alone(&mut w, h, 71.0);
    update(&mut w, h);
    assert_eq!(player(&w).knockback_type, 0);
}
