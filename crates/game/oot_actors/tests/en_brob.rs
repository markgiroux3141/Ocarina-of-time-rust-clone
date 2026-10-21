//! GAME-10 milestone 4 against the C: the Flobbery (`z_en_brob.c`). Master Quest's Jabu-Jabu
//! (`bdan`) room 1 places three at half size (params 0x01FF) at (-172, -340, -1553), (-233, -330,
//! -1707) and (-177, -340, -1858); room 2 and room 4 full-size ones (0x00FF).
//!
//! Child Link enters at `ENTR_JABU_JABU_0` on a `jabu-fish` save and the room changes to room 1
//! (`Room_RequestNewRoom`, as the game's `--room`). The tests run one Flobbery's update alone
//! (`Actor_UpdateAll`'s call), Link's distance and the hits set as `Actor_UpdateAll` and the
//! collision check would, so no fight is scripted and nothing waits on `Rand`. Expected values are
//! worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_collision::dyna::{BG_ACTOR_MAX, DYNA_INTERACT_PLAYER_ON_TOP};
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_brob::*;
use oot_game::actor::*;
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::sfx::{NA_SE_EN_GOMA_JR_FREEZE, SfxPos};
use oot_game::collision_check::*;
use oot_game::effect::EFFECT_SS_LIGHTNING;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::shadow::ShadowDraw;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Room 1's three (`ootx scene-info --scene bdan --layer 0`).
const HOMES: [Vec3; 3] = [Vec3::new(-172.0, -340.0, -1553.0), Vec3::new(-233.0, -330.0, -1707.0), Vec3::new(-177.0, -340.0, -1858.0)];

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// Child Link in Jabu-Jabu (`jabu-fish`), the sound log on, in room 1, its objects in.
fn room_1(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_JABU_JABU_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("jabu-fish").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    assert!(w.room_request(1));
    idle(&mut w, 1);
    w.room_change_done();
    assert_eq!(w.room_ctx.cur.num, 1);
    for _ in 0..30 {
        if brobs(&w).len() == 3 {
            break;
        }
        idle(&mut w, 1);
    }
    w
}

fn brobs(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnBrob>(h).is_some_and(|b| !b.actor.killed)).collect()
}

fn br(w: &PlayState, h: ActorHandle) -> &EnBrob {
    w.actors.downcast::<EnBrob>(h).expect("En_Brob")
}

fn br_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnBrob {
    w.actors.downcast_mut::<EnBrob>(h).expect("En_Brob")
}

fn at_home(w: &PlayState, home: Vec3) -> ActorHandle {
    *brobs(w).iter().find(|&&h| br(w, h).actor.home_pos == home).expect("the Flobbery")
}

/// The actor's update alone (`Actor_UpdateAll`'s call), as the updating actor.
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// Its draw's writes (`Actor_DrawAll`'s call).
fn draw_update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.draw_update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// The live soft sprites of a type.
fn effects(w: &PlayState, ty: u8) -> usize {
    w.effect_ss.table.iter().filter(|e| e.ty == ty && e.life >= 0).count()
}

/// The times a sound was asked for at `h`.
fn sfx(w: &PlayState, h: ActorHandle, id: u16) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, p)| s == id && p == SfxPos::Actor(h)).count()
}

/// Its colliders registered for AT, AC and OC since the lists were last cleared.
fn registered(w: &PlayState, h: ActorHandle) -> (usize, usize, usize) {
    let n = |l: &[ColliderRef]| l.iter().filter(|r| r.actor == h).count();
    (n(&w.col_chk.col_at), n(&w.col_chk.col_ac), n(&w.col_chk.col_oc))
}

fn clear_registrations(w: &mut PlayState) {
    w.col_chk.col_at.clear();
    w.col_chk.col_ac.clear();
    w.col_chk.col_oc.clear();
}

/// A hit by `flags` on collider `i`.
fn hit(w: &mut PlayState, h: ActorHandle, i: usize, flags: u32) {
    let link = w.player.expect("Link");
    let c = &mut br_mut(w, h).colliders[i];
    c.base.ac_flags |= AC_HIT;
    c.base.ac = Some(link);
    c.info.ac_hit_elem = Some(HitElem {
        elem: ElemRef { col: ColliderRef { actor: link, id: 0 }, elem: 0 },
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: flags, hit_special_effect: 0, damage: 1 },
        ac_dmg_info: Default::default(),
        elem_material: 0,
    });
}

/// The middle one up and wobbling (`EnBrob_SetupWobble`), Link 400 away.
fn wobbling(w: &mut PlayState) -> ActorHandle {
    let h = at_home(w, HOMES[1]);
    br_mut(w, h).setup_wobble();
    br_mut(w, h).actor.xz_dist_to_player = 400.0;
    h
}

#[test]
fn room_1s_three_half_size_and_the_full_size_ones() {
    let Some(a) = assets() else { return };
    let mut w = room_1(&a);
    assert_eq!(brobs(&w).len(), 3);
    for home in HOMES {
        let b = br(&w, at_home(&w, home));
        // EnBrob_Init: params 0x01FF: half size (0.005), its own height; params cut to 0xFF.
        assert_eq!((b.actor.scale, b.actor.params), (Vec3::splat(0.005), 0xFF));
        // sCylinderInit scaled: 8000 × 0.005 across; the body 12000 × 0.005 high from the floor,
        // the top 11000 × 0.005 from 5000 × 0.005 down.
        let [c0, c1] = &b.colliders;
        assert_eq!((c0.dim.radius, c0.dim.height, c0.dim.y_shift), (40, 60, 0));
        assert_eq!((c1.dim.radius, c1.dim.height, c1.dim.y_shift), (40, 55, -25));
        // Lock-on off, hostile; down (EnBrob_SetupIdle: no wait, the model at home); its
        // collision registered (DynaPoly_SetBgActor); no shadow (no ActorShape_Init).
        assert_eq!(b.actor.flags & (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE), ACTOR_FLAG_HOSTILE);
        assert_eq!((b.action, b.timer, b.model_offset_y), (Some(Action::Idle), 0, 0));
        assert_ne!(b.bg, BG_ACTOR_MAX);
        assert_eq!(b.actor.shape_shadow_draw, ShadowDraw::None);
        // sColChkInfoInit: 0, 60, 120, MASS_IMMOVABLE.
        let c = &b.actor.col_chk_info;
        assert_eq!((c.health, c.cyl_radius, c.cyl_height, c.mass), (0, 60, 120, MASS_IMMOVABLE));
    }
    // 0x00FF (rooms 2 and 4): full size; 0x0014 (the unused layer's room 5): 20/30 of its height.
    let p = HOMES[1] + Vec3::new(0.0, 0.0, 300.0);
    let h = w.actor_spawn(ACTOR_EN_BROB, p, [0, 0, 0], 0x00FF).expect("the Flobbery");
    let [c0, c1] = &br(&w, h).colliders;
    assert_eq!(br(&w, h).actor.scale, Vec3::splat(0.01));
    assert_eq!((c0.dim.radius, c0.dim.height, c0.dim.y_shift, c1.dim.height, c1.dim.y_shift), (80, 120, 0, 110, -50));
    let h = w.actor_spawn(ACTOR_EN_BROB, p + Vec3::X * 300.0, [0, 0, 0], 0x0014).expect("the Flobbery");
    let b = br(&w, h);
    let sy = 0.01f32 * (20.0 * (1.0 / 30.0));
    assert_eq!((b.actor.scale.x, b.actor.scale.y, b.actor.params), (0.01, sy, 0x14));
    assert_eq!((b.colliders[0].dim.height, b.colliders[1].dim.height), ((sy * 12000.0) as i16, (11000.0 * sy) as i16));
}

#[test]
fn link_within_300_brings_it_up_then_it_wobbles() {
    let Some(a) = assets() else { return };
    let mut w = room_1(&a);
    let h = at_home(&w, HOMES[1]);
    // 300 or more: down, nothing registered.
    br_mut(&mut w, h).actor.xz_dist_to_player = 300.0;
    clear_registrations(&mut w);
    update(&mut w, h);
    assert_eq!(br(&w, h).action, Some(Action::Idle));
    assert_eq!(registered(&w, h), (0, 0, 0));
    br_mut(&mut w, h).actor.xz_dist_to_player = 299.0;
    update(&mut w, h);
    // EnBrob_SetupMoveUp: gBrobMoveUpAnim once, the model 1000 up; hurting and pushing (not
    // hittable) from this frame.
    let b = br(&w, h);
    assert_eq!((b.action, b.model_offset_y), (Some(Action::MoveUp), 1000));
    assert!(b.skel.is(ANIM_MOVE_UP));
    assert_eq!(registered(&w, h), (2, 0, 2));
    // EnBrob_MoveUp: up 1000 a frame to frame 8, 250 to 12, down 250 after.
    let mut offset = 1000i32;
    loop {
        update(&mut w, h);
        let b = br(&w, h);
        if b.action != Some(Action::MoveUp) {
            break;
        }
        let f = b.skel.cur_frame;
        offset += if f < 8.0 {
            1000
        } else if f < 12.0 {
            250
        } else {
            -250
        };
        assert_eq!(b.model_offset_y as i32, offset, "frame {f}");
    }
    // EnBrob_SetupWobble: gBrobWobbleAnim (a 5-frame morph), 8000 up, 1200 frames; hurting,
    // hittable and pushing.
    let b = br(&w, h);
    assert_eq!((b.action, b.model_offset_y, b.timer), (Some(Action::Wobble), 8000, 1200));
    assert!(b.skel.is(ANIM_WOBBLE));
    clear_registrations(&mut w);
    update(&mut w, h);
    assert_eq!(registered(&w, h), (2, 2, 2));
}

#[test]
fn standing_on_it_throws_link_off_as_it_rises() {
    let Some(a) = assets() else { return };
    let mut w = room_1(&a);
    let h = at_home(&w, HOMES[0]);
    let bg = br(&w, h).bg;
    {
        let b = br_mut(&mut w, h);
        b.actor.xz_dist_to_player = 1000.0;
        b.actor.yaw_towards_player = 0x1000;
    }
    w.player_mut().knockback_type = 0;
    // DynaPolyActor_IsPlayerOnTop: Actor_SetPlayerKnockbackLargeNoDamage(5, yawTowardsPlayer, 1).
    w.col.dyna.set_interact_flag(bg, DYNA_INTERACT_PLAYER_ON_TOP);
    update(&mut w, h);
    let p = w.player();
    assert_eq!((p.knockback_type, p.knockback_rot, p.knockback_speed, p.knockback_y_velocity, p.knockback_damage), (2, 0x1000, 5.0, 1.0, 0));
    assert_eq!(br(&w, h).action, Some(Action::MoveUp));
}

#[test]
fn up_1200_frames_with_link_over_500_away_it_sinks() {
    let Some(a) = assets() else { return };
    let mut w = room_1(&a);
    let h = wobbling(&mut w);
    // NA_SE_EN_BROB_WAVE at frames 6 and 15 of each loop.
    for _ in 0..40 {
        update(&mut w, h);
    }
    assert!(sfx(&w, h, NA_SE_EN_BROB_WAVE) >= 2);
    // The count out with Link 500 away: still up; over 500: EnBrob_SetupMoveDown (the move-up
    // animation backward, 8250 up); not registered.
    br_mut(&mut w, h).timer = 1;
    br_mut(&mut w, h).actor.xz_dist_to_player = 500.0;
    update(&mut w, h);
    assert_eq!((br(&w, h).action, br(&w, h).timer), (Some(Action::Wobble), 0));
    br_mut(&mut w, h).actor.xz_dist_to_player = 501.0;
    clear_registrations(&mut w);
    update(&mut w, h);
    let b = br(&w, h);
    assert_eq!((b.action, b.model_offset_y, b.skel.play_speed), (Some(Action::MoveDown), 8250, -1.0));
    assert!(b.skel.is(ANIM_MOVE_UP));
    assert_eq!(registered(&w, h), (0, 0, 0));
    // EnBrob_MoveDown: down 1000 a frame to frame 8 (counting down), 250 to 12, up 250 after;
    // done, down (its collision on).
    let mut offset = 8250i32;
    loop {
        update(&mut w, h);
        let b = br(&w, h);
        if b.action != Some(Action::MoveDown) {
            break;
        }
        let f = b.skel.cur_frame;
        offset += if f < 8.0 {
            -1000
        } else if f < 12.0 {
            -250
        } else {
            250
        };
        assert_eq!(b.model_offset_y as i32, offset, "frame {f}");
    }
    let b = br(&w, h);
    assert_eq!((b.action, b.model_offset_y, b.timer), (Some(Action::Idle), 0, 0));
}

#[test]
fn touching_link_up_it_shocks_ten_loops_of_four_bolts() {
    let Some(a) = assets() else { return };
    let mut w = room_1(&a);
    let h = wobbling(&mut w);
    let bolts = effects(&w, EFFECT_SS_LIGHTNING);
    br_mut(&mut w, h).colliders[0].base.at_flags |= AT_HIT;
    update(&mut w, h);
    // EnBrob_SetupShock: gBrobShockAnim looped, ten loops; the hit forgotten; four bolts a frame.
    let b = br(&w, h);
    assert_eq!((b.action, b.timer), (Some(Action::Shock), 10));
    assert!(b.skel.is(ANIM_SHOCK));
    assert_eq!(b.colliders[0].base.at_flags & (AT_HIT | AT_BOUNCED), 0);
    assert_eq!(effects(&w, EFFECT_SS_LIGHTNING), bolts + 4);
    let mut n = 1;
    while br(&w, h).action == Some(Action::Shock) {
        update(&mut w, h);
        n += 1;
        assert!(n < 1000);
    }
    // Ten loops (a loop's frames each), then up again.
    let shock_len = a.animation(OBJECT, ANIM_SHOCK).unwrap().last_frame() + 1.0;
    assert!(n as f32 >= 9.0 * shock_len, "{n}");
    assert_eq!(br(&w, h).action, Some(Action::Wobble));
    // A Kokiri Sword's slash shocks too; a Master Sword's doesn't.
    hit(&mut w, h, 1, DMG_SLASH_MASTER);
    update(&mut w, h);
    assert_eq!(br(&w, h).action, Some(Action::Wobble));
    hit(&mut w, h, 1, DMG_SLASH_KOKIRI);
    update(&mut w, h);
    assert_eq!(br(&w, h).action, Some(Action::Shock));
}

#[test]
fn touching_link_as_it_rises_throws_him_off_unless_he_blocked() {
    let Some(a) = assets() else { return };
    let mut w = room_1(&a);
    let h = at_home(&w, HOMES[1]);
    br_mut(&mut w, h).actor.xz_dist_to_player = 100.0;
    br_mut(&mut w, h).actor.yaw_towards_player = -0x2000;
    update(&mut w, h);
    assert_eq!(br(&w, h).action, Some(Action::MoveUp));
    w.player_mut().knockback_type = 0;
    br_mut(&mut w, h).colliders[1].base.at_flags |= AT_HIT;
    update(&mut w, h);
    let p = w.player();
    assert_eq!((p.knockback_type, p.knockback_rot, p.knockback_speed, p.knockback_y_velocity), (2, -0x2000, 5.0, 1.0));
    assert_eq!(br(&w, h).action, Some(Action::MoveUp));
    // Blocked (AT_BOUNCED): neither thrown nor shocked; the flags forgotten.
    w.player_mut().knockback_type = 0;
    br_mut(&mut w, h).colliders[0].base.at_flags |= AT_HIT | AT_BOUNCED;
    update(&mut w, h);
    assert_eq!((w.player().knockback_type, br(&w, h).action), (0, Some(Action::MoveUp)));
    assert_eq!(br(&w, h).colliders[0].base.at_flags & (AT_HIT | AT_BOUNCED), 0);
}

#[test]
fn a_boomerang_stuns_it_down_for_200_frames() {
    let Some(a) = assets() else { return };
    let mut w = room_1(&a);
    let h = wobbling(&mut w);
    hit(&mut w, h, 1, DMG_BOOMERANG);
    clear_registrations(&mut w);
    update(&mut w, h);
    // EnBrob_SetupStunned: gBrobStunnedAnim (a 5-frame morph), 125 down, blue 80,
    // NA_SE_EN_GOMA_JR_FREEZE; EnBrob_Stunned's first frame 1250 down; pushing only.
    let b = br(&w, h);
    assert_eq!((b.action, b.model_offset_y), (Some(Action::Stunned), 8000 - 125 - 1250));
    assert!(b.skel.is(ANIM_STUNNED));
    assert_eq!((b.actor.color_filter_params & COLORFILTER_COLORFLAG_RED, b.actor.color_filter_timer), (0, 80));
    assert_eq!(sfx(&w, h, NA_SE_EN_GOMA_JR_FREEZE), 1);
    assert_eq!(registered(&w, h), (0, 0, 2));
    assert_eq!(b.colliders[1].base.ac_flags & AC_HIT, 0);
    // Done: down, 200 frames before it rises; blue while over 80 of them.
    let mut n = 0;
    while br(&w, h).action == Some(Action::Stunned) {
        update(&mut w, h);
        n += 1;
        assert!(n < 200);
    }
    let b = br(&w, h);
    assert_eq!((b.action, b.timer, b.model_offset_y), (Some(Action::Idle), 200, 0));
    br_mut(&mut w, h).actor.xz_dist_to_player = 100.0;
    br_mut(&mut w, h).actor.color_filter_timer = 0;
    update(&mut w, h);
    assert_eq!((br(&w, h).action, br(&w, h).timer, br(&w, h).actor.color_filter_timer), (Some(Action::Idle), 199, 80));
    br_mut(&mut w, h).timer = 81;
    br_mut(&mut w, h).actor.color_filter_timer = 0;
    update(&mut w, h);
    assert_eq!((br(&w, h).timer, br(&w, h).actor.color_filter_timer), (80, 0));
    br_mut(&mut w, h).timer = 1;
    update(&mut w, h);
    assert_eq!(br(&w, h).action, Some(Action::MoveUp));
}

#[test]
fn its_draw_puts_the_cylinders_on_limbs_3_and_8() {
    let Some(a) = assets() else { return };
    let mut w = room_1(&a);
    let h = wobbling(&mut w);
    for _ in 0..10 {
        update(&mut w, h);
    }
    draw_update(&mut w, h);
    let b = br(&w, h);
    let (body, top) = b.limb_positions().expect("the skeleton");
    // EnBrob_PostLimbDraw: the matrices' translations (Matrix_Get), the top's 7 up.
    assert_eq!(b.colliders[0].dim.pos, [body.x as i16, body.y as i16, body.z as i16]);
    assert_eq!(b.colliders[1].dim.pos, [top.x as i16, (top.y + 7.0) as i16, top.z as i16]);
    // Up, the top's well over the body's (whose cylinder starts under the floor: its base limb
    // stays sunk), both about its home.
    let p = b.actor.world_pos;
    assert!(top.y > body.y + 40.0, "{body:?} {top:?} {p:?}");
    assert!((body.x - p.x).abs() < 30.0 && (body.z - p.z).abs() < 30.0, "{body:?} {p:?}");
}
