//! GAME-10 milestone 2 against the C: the Octorok (`z_en_okuta.c`) and its rock. Zora's River's
//! room 0 places two for a child (params 0xFF00) at (-133, 60, -53) and (2928, 500, -557).
//!
//! Child Link enters at `ENTR_ZORAS_RIVER_0` (room 0) on a `fairy-magic` save, over 480 from both.
//! Most tests run one Octorok's update alone (`Actor_UpdateAll`'s call), Link's distance, height
//! and yaw set as `Actor_UpdateAll` would, so no fight is scripted and nothing waits on `Rand`.
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_okuta::*;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorHandle, PLAYER_STATE1_28, PLAYER_STATE1_29};
use oot_game::audio::sfx::{NA_SE_EN_NUTS_THROW, SfxPos};
use oot_game::collision_check::*;
use oot_game::effect::{EFFECT_SS_BUBBLE, EFFECT_SS_DT_BUBBLE, EFFECT_SS_DUST, EFFECT_SS_EN_ICE, EFFECT_SS_G_RIPPLE, EFFECT_SS_G_SPLASH, EFFECT_SS_HAHEN};
use oot_game::env::clock_time;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::shadow::ShadowDraw;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Room 0's two (`ootx scene-info --scene spot03 --layer 0`).
const WEST: Vec3 = Vec3::new(-133.0, 60.0, -53.0);
const EAST: Vec3 = Vec3::new(2928.0, 500.0, -557.0);

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// Child Link at the river's mouth (`fairy-magic`, the sound log on), two frames on.
fn river(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_ZORAS_RIVER_0").expect("entrance");
    let mut save = SaveContext::new(e, false, clock_time(10, 0) as u16);
    save.apply_preset("fairy-magic").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    assert_eq!(w.room_ctx.cur.num, 0);
    // Walked in from the field: until Link is his own again (no PLAYER_STATE1_28 or _29, which
    // hold the enemies, EnOkuta_Update's own check too).
    for _ in 0..200 {
        if w.player().state1 & (PLAYER_STATE1_28 | PLAYER_STATE1_29) == 0 {
            break;
        }
        idle(&mut w, 1);
    }
    assert_eq!(w.player().state1 & (PLAYER_STATE1_28 | PLAYER_STATE1_29), 0);
    idle(&mut w, 1);
    w
}

fn okutas(w: &PlayState, ty: i16) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnOkuta>(h).is_some_and(|o| !o.actor.killed && o.actor.params == ty)).collect()
}

fn at_home(w: &PlayState, x: f32, z: f32) -> ActorHandle {
    *okutas(w, TYPE_OCTOROK).iter().find(|&&h| ok(w, h).actor.home_pos.x == x && ok(w, h).actor.home_pos.z == z).expect("the Octorok")
}

fn ok(w: &PlayState, h: ActorHandle) -> &EnOkuta {
    w.actors.downcast::<EnOkuta>(h).expect("En_Okuta")
}

fn ok_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnOkuta {
    w.actors.downcast_mut::<EnOkuta>(h).expect("En_Okuta")
}

fn gone(w: &PlayState, h: ActorHandle) -> bool {
    w.actors.downcast::<EnOkuta>(h).is_none_or(|o| o.actor.killed)
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
    let o = ok_mut(w, h);
    o.actor.xz_dist_to_player = dist;
    o.actor.y_dist_to_player = y;
    o.actor.yaw_towards_player = yaw;
}

/// The live soft sprites of a type.
fn effects(w: &PlayState, ty: u8) -> usize {
    w.effect_ss.table.iter().filter(|e| e.ty == ty && e.life >= 0).count()
}

/// The times a sound was asked for at `h`.
fn sfx(w: &PlayState, h: ActorHandle, id: u16) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, p)| s == id && p == SfxPos::Actor(h)).count()
}

/// The times a sound was asked for anywhere.
fn sfx_any(w: &PlayState, id: u16) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).count()
}

/// The last frame of one of `object_okuta`'s animations.
fn last_frame(a: &Arc<GameAssets>, name: &str) -> f32 {
    a.animation(OBJECT, name).unwrap().last_frame()
}

/// Updates until the action changes; the updates run.
fn until_not(w: &mut PlayState, h: ActorHandle, action: Action, max: usize) -> usize {
    for n in 1..=max {
        update(w, h);
        if gone(w, h) || ok(w, h).action != action {
            return n;
        }
    }
    panic!("still {action:?} after {max}");
}

/// The west Octorok up and floating, Link 300 across at yaw 0 on its level.
fn floating(w: &mut PlayState) -> ActorHandle {
    let h = at_home(w, WEST.x, WEST.z);
    set_link(w, h, 300.0, 0.0, 0);
    update(w, h);
    assert_eq!(ok(w, h).action, Action::Appear);
    until_not(w, h, Action::Appear, 100);
    assert_eq!(ok(w, h).action, Action::WaitToShoot);
    h
}

#[test]
fn the_rivers_two_octoroks_wait_under_the_surface() {
    let Some(a) = assets() else { return };
    let w = river(&a);
    assert_eq!(okutas(&w, TYPE_OCTOROK).len(), 2);
    for home in [WEST, EAST] {
        let h = at_home(&w, home.x, home.z);
        let o = ok(&w, h);
        // EnOkuta_Init: numShots PARAMS_GET_U(0xFF00, 8, 8) = 0xFF: one; params cut to 0; the
        // floor under it (BgCheck_EntityRaycastDown4) below the water's surface there, its home's
        // height the surface's.
        assert_eq!((o.actor.params, o.num_shots), (TYPE_OCTOROK, 1));
        let surface = w.col.water_surface(home.x, home.z, w.col.water_room).expect("water");
        assert!(surface > o.actor.floor_height);
        assert_eq!(o.actor.home_pos.y, surface);
        // EnOkuta_SetupWaitToAppear: not drawn, no lock-on, at the surface; every frame there.
        assert_eq!((o.action, o.drawn, o.actor.world_pos.y), (Action::WaitToAppear, false, surface));
        assert_eq!(o.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
        assert_ne!(o.actor.flags & ACTOR_FLAG_HOSTILE, 0);
        // sInitChain: NAVI_ENEMY_OCTOROK, lockOnArrowOffset 6500; sColChkInfoInit 1, 15, 60, 100.
        assert_eq!((o.actor.navi_enemy_id, o.actor.target_arrow_offset), (NAVI_ENEMY_OCTOROK, 6500.0));
        let c = &o.actor.col_chk_info;
        assert_eq!((c.health, c.cyl_radius, c.cyl_height, c.mass), (1, 15, 60, 100));
        // No ActorShape_Init: no shadow. sOctorockColliderInit: 20 across by 40, 30 down.
        assert_eq!(o.actor.shape_shadow_draw, ShadowDraw::None);
        assert_eq!((o.collider.dim.radius, o.collider.dim.y_shift, o.collider.dim.height), (20, -30, 40));
    }
    // Its first update (Actor_UpdateAll runs it only inside its culling volume, or here alone):
    // the head 1 (EnOkuta_UpdateHeadScale), the cylinder (40 × 1 + 30) × 0.01 × 100 = 70.
    let mut w = w;
    let h = at_home(&w, WEST.x, WEST.z);
    update(&mut w, h);
    let o = ok(&w, h);
    assert_eq!((o.action, o.head_scale, o.collider.dim.height), (Action::WaitToAppear, Vec3::ONE, 70));
}

#[test]
fn link_between_200_and_480_brings_it_up_then_it_floats() {
    let Some(a) = assets() else { return };
    let mut w = river(&a);
    let h = at_home(&w, WEST.x, WEST.z);
    // 480 or more across, or 200 or less: it stays down.
    for d in [480.0, 600.0, 200.0, 100.0] {
        set_link(&mut w, h, d, 0.0, 0x1234);
        update(&mut w, h);
        assert_eq!(ok(&w, h).action, Action::WaitToAppear, "{d}");
    }
    let bubbles = effects(&w, EFFECT_SS_BUBBLE);
    set_link(&mut w, h, 300.0, 0.0, 0x1234);
    update(&mut w, h);
    // EnOkuta_SetupAppear: drawn, facing Link, lock-on on, gOctorokAppearAnim, ten bubbles.
    let o = ok(&w, h);
    assert_eq!((o.action, o.drawn, o.actor.shape_rot.y), (Action::Appear, true, 0x1234));
    assert_ne!(o.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert!(o.skel.is(ANIM_APPEAR));
    assert_eq!(effects(&w, EFFECT_SS_BUBBLE), bubbles + 10);
    // EnOkuta_Appear: growing a quarter of its size a frame to frame 4, its size at 5; the jump's
    // sound at frame 2, the landing's at 12, a splash at 3 and 15 (at the surface, its home).
    let last = last_frame(&a, ANIM_APPEAR);
    let splashes = effects(&w, EFFECT_SS_G_SPLASH);
    let mut k = 0.0;
    while ok(&w, h).action == Action::Appear {
        update(&mut w, h);
        k += 1.0;
        let o = ok(&w, h);
        if k <= 4.0 {
            assert_eq!(o.actor.scale, Vec3::splat(k * 0.25 * 0.01), "frame {k}");
        } else if k <= last {
            assert_eq!(o.actor.scale, Vec3::splat(0.01), "frame {k}");
        }
        assert!(k <= last + 1.0);
    }
    assert_eq!(sfx(&w, h, NA_SE_EN_OCTAROCK_JUMP), 1);
    assert_eq!(sfx(&w, h, NA_SE_EN_OCTAROCK_LAND), 1);
    assert_eq!(effects(&w, EFFECT_SS_G_SPLASH), splashes + [3.0, 15.0].iter().filter(|&&f| f <= last).count());
    // Ended with Link 160 or more across: floating, gOctorokFloatAnim looped, no wait.
    let o = ok(&w, h);
    assert_eq!((o.action, o.timer), (Action::WaitToShoot, 0));
    assert!(o.skel.is(ANIM_FLOAT));
    // Its head bobs (EnOkuta_UpdateHeadScale: y 1 + sinf((M_PI / 16) × frame) × 0.2) and the
    // cylinder with it; the snout twice as much (EnOkuta_GetSnoutScale).
    let f = o.skel.cur_frame;
    let hy = (((std::f32::consts::PI / 16.0) * f).sin() * 0.2) + 1.0;
    assert_eq!(o.head_scale, Vec3::new(1.0, hy, 1.0));
    assert_eq!(o.collider.dim.height, ((40.0 * hy + 30.0) * 0.01 * 100.0) as i16);
    let (head, snout) = o.limb_scales();
    assert_eq!(head.is_some(), hy != 1.0);
    assert_eq!(snout, Some(Vec3::new(1.0, (((std::f32::consts::PI / 16.0) * f).sin() * 0.4) + 1.0, 1.0)));
}

#[test]
fn appearing_with_link_near_it_goes_straight_back_under() {
    let Some(a) = assets() else { return };
    let mut w = river(&a);
    let h = at_home(&w, WEST.x, WEST.z);
    set_link(&mut w, h, 300.0, 0.0, 0);
    update(&mut w, h);
    // Link comes within 160 before the animation ends: EnOkuta_SetupHide.
    set_link(&mut w, h, 150.0, 0.0, 0);
    until_not(&mut w, h, Action::Appear, 100);
    let o = ok(&w, h);
    assert_eq!(o.action, Action::Hide);
    assert!(o.skel.is(ANIM_HIDE));
    // EnOkuta_Hide: back to the surface; shrinking from frame 4 ((6 - frame) × 0.5 of its size);
    // the sinking's sound at 2, a splash at 4; done, NA_SE_EN_OCTAROCK_BUBLE, ten bubbles, under.
    let (bubbles, splashes) = (effects(&w, EFFECT_SS_BUBBLE), effects(&w, EFFECT_SS_G_SPLASH));
    let last = last_frame(&a, ANIM_HIDE);
    let mut k = 0.0;
    while ok(&w, h).action == Action::Hide {
        update(&mut w, h);
        k += 1.0;
        let o = ok(&w, h);
        if (4.0..=last).contains(&k) && o.action == Action::Hide {
            assert_eq!(o.actor.scale, Vec3::splat((6.0 - k) * 0.5 * 0.01), "frame {k}");
        }
        assert!(k <= last + 1.0);
    }
    let o = ok(&w, h);
    assert_eq!((o.action, o.drawn), (Action::WaitToAppear, false));
    assert_eq!(o.actor.world_pos.y, o.actor.home_pos.y);
    assert_eq!(o.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!(sfx(&w, h, NA_SE_EN_OCTAROCK_SINK), 1);
    assert_eq!(sfx(&w, h, NA_SE_EN_OCTAROCK_BUBLE), 1);
    assert_eq!(effects(&w, EFFECT_SS_BUBBLE), bubbles + 10);
    assert_eq!(effects(&w, EFFECT_SS_G_SPLASH), splashes + 1);
}

#[test]
fn facing_link_it_shoots_its_rocks_then_waits_two_loops() {
    let Some(a) = assets() else { return };
    let mut w = river(&a);
    let h = floating(&mut w);
    // Facing Link (yaw 0, its shape's 0): EnOkuta_SetupShoot, numShots (1) rocks; the jump Link's
    // height plus 20, at least 10: 20, no splash.
    let (splashes, dust) = (effects(&w, EFFECT_SS_G_SPLASH), effects(&w, EFFECT_SS_DUST));
    set_link(&mut w, h, 300.0, 0.0, 0);
    ok_mut(&mut w, h).actor.shape_rot.y = 0;
    update(&mut w, h);
    let o = ok(&w, h);
    assert_eq!((o.action, o.timer, o.jump_height), (Action::Shoot, 1, 20.0));
    assert!(o.skel.is(ANIM_SHOOT));
    assert_eq!(effects(&w, EFFECT_SS_G_SPLASH), splashes);
    // EnOkuta_Shoot: up sinf(0.08333 × M_PI × frame) × 20 over the surface to frame 12; the rock
    // at frame 6, 25 ahead (+z) and 6 down, its dust 40 ahead; NA_SE_EN_NUTS_THROW.
    let home_y = o.actor.home_pos.y;
    let last = last_frame(&a, ANIM_SHOOT);
    let mut rock = None;
    for k in 1..=(last as usize) {
        update(&mut w, h);
        let o = ok(&w, h);
        let f = k as f32;
        assert_eq!(o.action, Action::Shoot);
        if f < 13.0 {
            assert_eq!(o.actor.world_pos.y, (((0.08333f32 * std::f32::consts::PI) * f).sin() * 20.0) + home_y, "frame {k}");
        }
        if k == 6 {
            let rocks = okutas(&w, TYPE_PROJECTILE);
            assert_eq!(rocks.len(), 1);
            let r = ok(&w, rocks[0]);
            let p = o.actor.world_pos;
            assert_eq!(r.actor.world_pos, Vec3::new(p.x, p.y - 6.0, p.z + 25.0));
            assert_eq!(effects(&w, EFFECT_SS_DUST), dust + 1);
            assert_eq!(sfx(&w, h, NA_SE_EN_NUTS_THROW), 1);
            rock = Some(rocks[0]);
        }
    }
    assert!(rock.is_some());
    // The animation's end: the count (1) out, floating, two loops to wait.
    update(&mut w, h);
    let o = ok(&w, h);
    assert_eq!((o.action, o.timer), (Action::WaitToShoot, 2));
    // Facing Link it waits out the loops, then shoots again.
    let float_len = last_frame(&a, ANIM_FLOAT) + 1.0;
    let n = until_not(&mut w, h, Action::WaitToShoot, 200);
    assert_eq!(ok(&w, h).action, Action::Shoot);
    assert!(n as f32 >= float_len, "{n}");
}

#[test]
fn three_shots_in_a_row_and_a_high_jump_splashes() {
    let Some(a) = assets() else { return };
    let mut w = river(&a);
    let h = floating(&mut w);
    ok_mut(&mut w, h).num_shots = 3;
    // Link 40 above: a jump of 60, over 50: a splash and the jump's sound at each start, a splash
    // and the landing's sound at frame 13.
    let splashes = effects(&w, EFFECT_SS_G_SPLASH);
    let jumps = sfx(&w, h, NA_SE_EN_OCTAROCK_JUMP);
    set_link(&mut w, h, 300.0, 40.0, 0);
    ok_mut(&mut w, h).actor.shape_rot.y = 0;
    update(&mut w, h);
    assert_eq!((ok(&w, h).action, ok(&w, h).timer, ok(&w, h).jump_height), (Action::Shoot, 3, 60.0));
    assert_eq!(effects(&w, EFFECT_SS_G_SPLASH), splashes + 1);
    assert_eq!(sfx(&w, h, NA_SE_EN_OCTAROCK_JUMP), jumps + 1);
    let mut starts = 1;
    while ok(&w, h).action == Action::Shoot {
        let timer = ok(&w, h).timer;
        update(&mut w, h);
        if ok(&w, h).action == Action::Shoot && ok(&w, h).timer != timer {
            starts += 1;
        }
    }
    // Three shoots (the count down each end), three rocks.
    assert_eq!(starts, 3);
    assert_eq!(okutas(&w, TYPE_PROJECTILE).len(), 3);
    assert_eq!(sfx(&w, h, NA_SE_EN_OCTAROCK_JUMP), jumps + 3);
    assert_eq!(sfx(&w, h, NA_SE_EN_OCTAROCK_LAND) >= 3, true);
    assert_eq!((ok(&w, h).action, ok(&w, h).timer), (Action::WaitToShoot, 2));
}

#[test]
fn link_too_near_or_far_sends_it_under() {
    let Some(a) = assets() else { return };
    let mut w = river(&a);
    for d in [150.0, 600.0] {
        let h = floating(&mut w);
        set_link(&mut w, h, d, 0.0, 0);
        update(&mut w, h);
        assert_eq!(ok(&w, h).action, Action::Hide, "{d}");
        until_not(&mut w, h, Action::Hide, 100);
        assert_eq!(ok(&w, h).action, Action::WaitToAppear);
    }
}

#[test]
fn any_hit_with_damage_kills_it_a_deku_nut_does_nothing() {
    let Some(a) = assets() else { return };
    let mut w = river(&a);
    let h = floating(&mut w);
    // Link off to its side (it turns, not shooting this frame).
    set_link(&mut w, h, 300.0, 0.0, 0x4000);
    // A Deku nut: damage 0, reaction 0: the hit forgotten, nothing else.
    {
        let o = ok_mut(&mut w, h);
        o.collider.base.ac_flags |= AC_HIT;
        o.actor.col_chk_info.damage = 0;
        o.actor.col_chk_info.damage_reaction = 0;
    }
    update(&mut w, h);
    let o = ok(&w, h);
    assert_eq!((o.action, o.actor.col_chk_info.health, o.collider.base.ac_flags & AC_HIT), (Action::WaitToShoot, 1, 0));
    // The Kokiri Sword: 1. Enemy_StartFinishingBlow (freezeFlashTimer 5), health 0, lock-on off;
    // EnOkuta_SetupWaitToDie: gOctorokHitAnim, red 11 frames, its size, NA_SE_EN_OCTAROCK_DEAD1.
    {
        let o = ok_mut(&mut w, h);
        o.collider.base.ac_flags |= AC_HIT;
        o.actor.col_chk_info.damage = 1;
    }
    w.actors.freeze_flash_timer = 0;
    update(&mut w, h);
    let o = ok(&w, h);
    assert_eq!((o.action, o.actor.col_chk_info.health, o.actor.scale), (Action::WaitToDie, 0, Vec3::splat(0.01)));
    assert!(o.skel.is(ANIM_HIT));
    assert_eq!(o.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!(o.actor.color_filter_params & COLORFILTER_COLORFLAG_RED, COLORFILTER_COLORFLAG_RED);
    assert_eq!(o.actor.color_filter_timer, 11);
    assert_eq!(w.actors.freeze_flash_timer, 5);
    assert_eq!(sfx(&w, h, NA_SE_EN_OCTAROCK_DEAD1), 1);
    // Dying it isn't hit again: no AC (EnOkuta_Update).
    until_not(&mut w, h, Action::WaitToDie, 100);
    assert_eq!(ok(&w, h).action, Action::Die);
    assert!(ok(&w, h).skel.is(ANIM_DIE));
    // EnOkuta_Die: the animation, then a frame count: 1.0, 1.25, 1.5 of its size, down 0.2333 to
    // 0.8, up 0.04 to 1.0; the dust and NA_SE_EN_OCTAROCK_DEAD2 at 5; shrinking from 11.
    // The animation (a 3-frame morph into it, then once) with the count at 0; the count's first
    // frame is the one after its last.
    let die_last = last_frame(&a, ANIM_DIE);
    let mut n = 0;
    while ok(&w, h).skel.cur_frame < die_last || ok(&w, h).skel.morph_weight != 0.0 {
        update(&mut w, h);
        assert_eq!(ok(&w, h).timer, 0);
        n += 1;
        assert!(n < 100);
    }
    let dust = effects(&w, EFFECT_SS_DUST);
    for t in 1..=10i16 {
        update(&mut w, h);
        let o = ok(&w, h);
        assert_eq!(o.timer, t);
        let tf = t as f32;
        let want = if t < 3 { ((tf * 0.25) + 1.0) * 0.01 } else if t < 6 { (1.5 - ((tf - 2.0) * 0.2333)) * 0.01 } else { (((tf - 5.0) * 0.04) + 0.8) * 0.01 };
        assert_eq!(o.actor.scale, Vec3::splat(want), "count {t}");
        if t == 5 {
            assert_eq!(effects(&w, EFFECT_SS_DUST), dust + 1);
            assert_eq!(sfx(&w, h, NA_SE_EN_OCTAROCK_DEAD2), 1);
        }
    }
    // 0.0005 a frame from 0.01: 20 frames; then NA_SE_EN_OCTAROCK_BUBLE where it was, the drop,
    // twenty bubbles, gone.
    let bubbles = effects(&w, EFFECT_SS_DT_BUBBLE);
    for k in 1..=19 {
        update(&mut w, h);
        let s = ok(&w, h).actor.scale;
        assert!(s.x > 0.0 && s.x == s.y && s.y == s.z, "frame {k}: {s:?}");
    }
    assert!(!gone(&w, h));
    update(&mut w, h);
    assert!(gone(&w, h));
    assert_eq!(effects(&w, EFFECT_SS_DT_BUBBLE), bubbles + 20);
}

#[test]
fn an_ice_arrow_freezes_it_in_ice_then_it_dies() {
    let Some(a) = assets() else { return };
    let mut w = river(&a);
    let h = floating(&mut w);
    {
        let o = ok_mut(&mut w, h);
        o.collider.base.ac_flags |= AC_HIT;
        o.actor.col_chk_info.damage = 4;
        o.actor.col_chk_info.damage_reaction = 3;
    }
    let ice = effects(&w, EFFECT_SS_EN_ICE);
    update(&mut w, h);
    // EnOkuta_SetupFreeze: 80 frames, blue.
    // EnOkuta_ColliderCheck runs before the action: Freeze runs the same frame (79).
    let o = ok(&w, h);
    assert_eq!((o.action, o.timer, o.actor.col_chk_info.health), (Action::Freeze, 79, 0));
    assert_eq!(o.actor.color_filter_params & (COLORFILTER_COLORFLAG_RED | COLORFILTER_COLORFLAG_GRAY), COLORFILTER_COLORFLAG_BLUE);
    assert_eq!(o.actor.color_filter_timer, 80);
    // EnOkuta_Freeze: a chunk of ice on the odd counts from 79 to 65 (eight), dying at 0.
    for t in (0..79i16).rev() {
        update(&mut w, h);
        if t == 0 {
            break;
        }
        assert_eq!((ok(&w, h).action, ok(&w, h).timer), (Action::Freeze, t));
    }
    assert_eq!(effects(&w, EFFECT_SS_EN_ICE), ice + 8);
    assert_eq!((ok(&w, h).action, ok(&w, h).timer), (Action::Die, 0));
}

#[test]
fn out_of_the_water_alive_it_is_gone() {
    let Some(a) = assets() else { return };
    let mut w = river(&a);
    let h = at_home(&w, WEST.x, WEST.z);
    // Its x and z where no water box is: Actor_Kill.
    ok_mut(&mut w, h).actor.world_pos.x = 20000.0;
    update(&mut w, h);
    assert!(gone(&w, h));
    // Dying (no health), it stays.
    let h = at_home(&w, EAST.x, EAST.z);
    {
        let o = ok_mut(&mut w, h);
        o.actor.col_chk_info.health = 0;
        o.actor.world_pos.x = 20000.0;
    }
    update(&mut w, h);
    assert!(!gone(&w, h));
}

#[test]
fn drawn_it_ripples_the_surface_every_seventh_frame() {
    let Some(a) = assets() else { return };
    let mut w = river(&a);
    let h = floating(&mut w);
    set_link(&mut w, h, 300.0, 0.0, 0x4000);
    for frames in [700u32, 701, 706, 707] {
        w.gameplay_frames = frames;
        let ripples = effects(&w, EFFECT_SS_G_RIPPLE);
        update(&mut w, h);
        assert_eq!(effects(&w, EFFECT_SS_G_RIPPLE), ripples + (frames % 7 == 0) as usize, "{frames}");
    }
}

#[test]
fn its_rock_flies_on_and_breaks_on_the_ground() {
    let Some(a) = assets() else { return };
    let mut w = river(&a);
    // 100 over the water where the west one waits.
    let o = ok(&w, at_home(&w, WEST.x, WEST.z));
    let p = Vec3::new(WEST.x, o.actor.home_pos.y + 100.0, WEST.z);
    let r = w.actor_spawn(ACTOR_EN_OKUTA, p, [0, 0x4000, 0], TYPE_PROJECTILE).expect("the rock");
    // EnOkuta_Init (not 0): the circle shadow 18 at 1100 up, no lock-on, always updating, a prop,
    // facing nowhere (shape.rot.y 0), 10 a frame, 30 frames; sProjectileColliderInit: 13 by 20,
    // hurting 8.
    let o = ok(&w, r);
    assert_eq!((o.actor.category, o.action, o.timer, o.actor.speed_xz), (ACTORCAT_PROP, Action::ProjectileFly, 30, 10.0));
    assert_eq!((o.actor.shape_shadow_draw, o.actor.shape_shadow_scale, o.actor.shape_y_offset), (ShadowDraw::Circle, 18.0, 1100.0));
    assert_eq!(o.actor.flags & (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_UPDATE_CULLING_DISABLED), ACTOR_FLAG_UPDATE_CULLING_DISABLED);
    assert_eq!((o.actor.shape_rot.y, o.actor.world_rot.y), (0, 0x4000));
    assert_eq!((o.collider.dim.radius, o.collider.dim.height, o.collider.info.at_dmg_info.damage), (13, 20, 8));
    assert_eq!(o.collider.base.at_flags & (AT_ON | AT_TYPE_ENEMY), AT_ON | AT_TYPE_ENEMY);
    // EnOkuta_ProjectileFly: on at 10 along +x (15 a frame: Actor_UpdatePos's R_UPDATE_RATE × 0.5),
    // level for its 30 frames, spinning 0x1554 a frame.
    update(&mut w, r);
    let o = ok(&w, r);
    assert_eq!((o.timer, o.actor.home_rot.z), (29, 0x1554));
    assert!(o.actor.world_pos.distance(p + Vec3::new(15.0, 0.0, 0.0)) < 0.01, "{:?}", o.actor.world_pos);
    assert_eq!(o.actor.flags & ACTOR_FLAG_SFX_FOR_PLAYER_BODY_HIT, ACTOR_FLAG_SFX_FOR_PLAYER_BODY_HIT);
    // Put on the ground: the next update finds BGCHECKFLAG_GROUND and it breaks: fifteen pieces
    // (EffectSsHahen_SpawnBurst), NA_SE_EN_OCTAROCK_ROCK, gone.
    let pieces = effects(&w, EFFECT_SS_HAHEN);
    let rocks = sfx_any(&w, oot_game::audio::sfx::NA_SE_EN_OCTAROCK_ROCK);
    {
        let o = ok_mut(&mut w, r);
        o.actor.bg_check_flags |= BGCHECKFLAG_GROUND;
    }
    w.player_mut().current_shield = 0;
    update(&mut w, r);
    assert!(gone(&w, r));
    assert_eq!(effects(&w, EFFECT_SS_HAHEN), pieces + 15);
    assert!(sfx_any(&w, oot_game::audio::sfx::NA_SE_EN_OCTAROCK_ROCK) > rocks);
}

#[test]
fn bounced_off_the_deku_shield_the_rock_flies_back_as_links() {
    let Some(a) = assets() else { return };
    let mut w = river(&a);
    let p = w.player().actor.world_pos + Vec3::new(0.0, 30.0, 100.0);
    let r = w.actor_spawn(ACTOR_EN_OKUTA, p, [0, -0x8000, 0], TYPE_PROJECTILE).expect("the rock");
    // The Deku Shield (PLAYER_SHIELD_DEKU), its hit bounced (AT_HIT, AT_BOUNCED, an enemy's).
    w.player_mut().current_shield = 1;
    let shield_yaw = oot_game::sys_matrix::MtxF::from_mat4(w.player().shield_mf).to_yxz_rot_s(false)[1];
    {
        let o = ok_mut(&mut w, r);
        o.collider.base.at_flags |= AT_HIT | AT_BOUNCED;
    }
    update(&mut w, r);
    let o = ok(&w, r);
    assert!(!gone(&w, r));
    // Link's attack now (AT_TYPE_PLAYER, DMG_DEKU_STICK), back the way the shield faces
    // (Matrix_MtxFToYXZRotS's y + 0x8000), 30 frames more.
    assert_eq!(o.collider.base.at_flags & (AT_HIT | AT_BOUNCED | AT_TYPE_ENEMY | AT_TYPE_PLAYER), AT_TYPE_PLAYER);
    assert_eq!(o.collider.info.at_dmg_info.dmg_flags, 1 << 1);
    assert_eq!((o.actor.world_rot.y, o.timer), (shield_yaw.wrapping_add(-0x8000i16), 30));
}
