//! GAME-09 milestone 5 against the C: `En_Dodongo` (`z_en_dodongo.c`), the Dodongo, and its fire
//! (`Effect_Ss_D_Fire`, `z_eff_ss_d_fire.c`). Dodongo's Cavern's room 5 (Master Quest) places
//! three (params 0xFFFF) on its floor (y 531).
//!
//! Entered by `ENTR_DODONGOS_CAVERN_0` on a `cavern-open` save, room 5 loaded with
//! `Room_RequestNewRoom`; each test keeps one Dodongo (the others killed) and puts Link where it
//! needs him. Hits are put on its colliders as the collision check leaves them (`AC_HIT` with the
//! damage table's damage and reaction); nothing is fought by script. Random values (its idle and
//! walk timers, the fire's pause) are read back, never assumed. Expected values are worked out from
//! the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_bom::{ACTOR_EN_BOM, EnBom};
use oot_actors::en_dodongo::*;
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, COLORFILTER_COLORFLAG_BLUE};
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::sfx::{NA_SE_EN_GOMA_JR_FREEZE, NA_SE_IT_BOMB_EXPLOSION, SFX_FLAG};
use oot_game::collision_check::*;
use oot_game::effect::{EFFECT_SS_D_FIRE, EFFECT_SS_EN_ICE, d_fire};
use oot_game::env::clock_time;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Room 5's three (`ootx scene-info --scene ddan`): west, north-east, south-east.
const WEST: Vec3 = Vec3::new(-2168.0, 531.0, -360.0);
const NORTH_EAST: Vec3 = Vec3::new(-1823.0, 531.0, -129.0);
const SOUTH_EAST: Vec3 = Vec3::new(-1817.0, 531.0, -448.0);
/// The room's north-east corner: over 400 from the west one's home.
const CORNER: Vec3 = Vec3::new(-1740.0, 531.0, -60.0);
const SKEL_MESH: &str = "mesh/object_dodongo/gDodongoSkel";

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// Child Link in room 5 (`cavern-open`), in the corner, the audio logged; only the Dodongo at
/// `keep` left.
fn room5(a: &Arc<GameAssets>, keep: Vec3) -> (PlayState, ActorHandle) {
    let e = a.scenes.entrance_index("ENTR_DODONGOS_CAVERN_0").expect("entrance");
    let mut save = SaveContext::new(e, false, clock_time(10, 0) as u16);
    save.apply_preset("cavern-open").unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    w.audio.log = Some(Default::default());
    idle(&mut w, 2);
    assert!(w.room_request(5));
    idle(&mut w, 1);
    w.room_change_done();
    w.place_player(CORNER, i16::MIN);
    // The room's actors spawn once object_dodongo is loaded.
    for _ in 0..30 {
        if w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnDodongo>(h).is_some()).count() == 3 {
            break;
        }
        idle(&mut w, 1);
    }
    for home in [WEST, NORTH_EAST, SOUTH_EAST] {
        if home != keep {
            let h = dodongo_at(&w, home);
            w.actors.actor_mut(h).unwrap().kill();
        }
    }
    idle(&mut w, 1);
    let h = dodongo_at(&w, keep);
    (w, h)
}

fn dodongo_at(w: &PlayState, home: Vec3) -> ActorHandle {
    w.actors
        .all()
        .into_iter()
        .find(|&h| w.actors.downcast::<EnDodongo>(h).is_some_and(|d| !d.actor.killed && (d.actor.home_pos.x - home.x).abs() < 1.0 && (d.actor.home_pos.z - home.z).abs() < 1.0))
        .expect("the Dodongo")
}

fn dodongo(w: &PlayState, h: ActorHandle) -> &EnDodongo {
    w.actors.downcast::<EnDodongo>(h).unwrap()
}

fn dodongo_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnDodongo {
    w.actors.downcast_mut::<EnDodongo>(h).unwrap()
}

fn anim_name(d: &EnDodongo) -> &str {
    d.skel.animation.as_ref().map(|a| a.name.as_str()).unwrap_or("")
}

fn sfx_frames(w: &PlayState, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, _)| f).collect()
}

/// The meshes the last frame draws.
fn drawn(w: &PlayState) -> Vec<String> {
    let f = w.current_frame();
    let mut out = oot_game::play::DrawOut::default();
    w.draw(&f, &oot_game::play::ViewInfo::new(f.view.eye, glam::Mat4::IDENTITY), &mut out);
    out.opa.iter().chain(out.xlu.iter()).map(|c| c.mesh.name.clone()).collect()
}

/// Ticks until `f` holds of it; the frames it took.
fn until(w: &mut PlayState, h: ActorHandle, max: usize, f: impl Fn(&EnDodongo) -> bool) -> usize {
    for n in 1..=max {
        idle(w, 1);
        if f(dodongo(w, h)) {
            return n;
        }
    }
    let d = dodongo(w, h);
    panic!("not within {max} frames: {:?} at {} (frame {})", d.action, d.actor.world_pos, d.skel.cur_frame);
}

/// Ticks until it's gone; the frames it took.
fn until_gone(w: &mut PlayState, h: ActorHandle, max: usize) -> usize {
    for n in 1..=max {
        idle(w, 1);
        if w.actors.actor(h).is_none_or(|a| a.killed) {
            return n;
        }
    }
    panic!("still there");
}

/// A hit on its body as the collision check leaves one: `AC_HIT`, the table's damage and
/// reaction.
fn hit(w: &mut PlayState, h: ActorHandle, damage: u8, reaction: u8) {
    let d = dodongo_mut(w, h);
    d.body_collider.base.ac_flags |= AC_HIT;
    d.actor.col_chk_info.damage = damage;
    d.actor.col_chk_info.damage_reaction = reaction;
}

/// Link `dist` from it along `yaw_off` from its facing, facing it.
fn link_at(w: &mut PlayState, h: ActorHandle, dist: f32, yaw_off: i16) {
    let d = dodongo(w, h);
    let yaw = d.actor.shape_rot.y.wrapping_add(yaw_off);
    let p = d.actor.world_pos + Vec3::new(eng_math::sin_s(yaw), 0.0, eng_math::cos_s(yaw)) * dist;
    w.place_player(p, yaw.wrapping_add(i16::MIN));
}

/// `Effect_GetByIndex` of a trail.
fn blure(w: &PlayState, idx: usize) -> Option<&oot_game::effect::blure::EffectBlure> {
    idx.checked_sub(oot_game::effect::SPARK_COUNT).and_then(|i| w.effect_ctx.blures.get(i)).and_then(|b| b.as_ref())
}

/// The live D_Fire puffs.
fn fires(w: &PlayState) -> Vec<&oot_game::effect::EffectSs> {
    w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == EFFECT_SS_D_FIRE).collect()
}

#[test]
fn init_matches_the_c() {
    let Some(a) = assets() else { return };
    let (w, h) = room5(&a, WEST);
    let d = dodongo(&w, h);
    // EnDodongo_Init: ATTENTION_RANGE_3, sInitChain (NAVI_ENEMY_DODONGO, gravity -1,
    // lockOnArrowOffset 2800), the circle shadow 48, scale 0.01875, 4 health, MASS_HEAVY, the
    // smoke's colours, bodyScale 1, the trail; idle (gDodongoWaitAnim, Rand_S16Offset(30, 50)).
    assert_eq!((d.actor.target_mode, d.actor.navi_enemy_id, d.actor.gravity, d.actor.target_arrow_offset), (3, NAVI_ENEMY_DODONGO, -1.0, 2800.0));
    assert_eq!((d.actor.shape_shadow_draw, d.actor.shape_shadow_scale), (oot_game::shadow::ShadowDraw::Circle, 48.0));
    assert_eq!((d.actor.scale.x, d.actor.col_chk_info.health, d.actor.col_chk_info.mass), (0.01875, 4, MASS_HEAVY));
    assert_eq!((d.bomb_smoke_prim_color, d.bomb_smoke_env_color, d.body_scale), ([255, 255, 0, 200], [255, 10, 0, 200], Vec3::ONE));
    assert_eq!((d.action, d.action_state, anim_name(d)), (Action::Idle, ActionState::Idle, ANIM_WAIT));
    assert!(d.timer < 80, "{}", d.timer);
    assert!(blure(&w, d.blure_idx).is_some());
    assert_ne!(d.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    // The colliders: the tail's spheres no AT yet, the metal head, the fire's quad.
    assert_eq!(d.body_collider.elements.len(), 6);
    assert!(d.body_collider.elements.iter().all(|e| e.info.at_elem_flags == ATELEM_NONE));
    assert_eq!((d.collider_hard.base.col_type, d.collider_hard.base.ac_flags & AC_HARD), (COL_MATERIAL_METAL, AC_HARD));
    assert_eq!((d.collider_at.info.at_dmg_info.hit_special_effect, d.collider_at.info.at_dmg_info.damage), (HIT_SPECIAL_EFFECT_FIRE, 0x10));
    // Drawn; the draw put its spheres and mouth on its body (the tail's sphere 0 is 17 across).
    assert!(drawn(&w).iter().any(|m| m == SKEL_MESH));
    assert_eq!(d.body_collider.elements[0].dim.world_sphere.radius, 17);
    assert!(d.mouth_pos.distance(d.actor.world_pos) < 80.0, "mouth {}", d.mouth_pos);
}

#[test]
fn it_idles_until_its_timer_is_out_at_the_waits_frame_0_then_walks() {
    let Some(a) = assets() else { return };
    let (mut w, h) = room5(&a, WEST);
    // EnDodongo_Idle: DECR(timer) each frame; out, at the loop's frame 0 (Animation_OnFrame(0):
    // the frame wrapping to 0), EnDodongo_SetupWalk. From (timer t, frame f): the first k >= t
    // with (f + k) a multiple of the loop's length.
    let d = dodongo(&w, h);
    assert_eq!(d.action, Action::Idle);
    let (t, f) = (d.timer as usize, d.skel.cur_frame as usize);
    let len = d.skel.anim_length as usize;
    let expect = (t.max(1)..).find(|k| (f + k) % len == 0).unwrap();
    let n = until(&mut w, h, 400, |d| d.action == Action::Walk);
    assert_eq!(n, expect, "timer {t}, frame {f}, length {len}");
    // EnDodongo_SetupWalk: the walk looped (speed 0 until the walk sets it), speed 1.5, the
    // right foot next, Rand_S16Offset(50, 70); the walk's first frame counted it once (Link is
    // over 400 from its home) and eased its speed (Math_SmoothStepToF to 1.5: there).
    let d = dodongo(&w, h);
    assert_eq!((anim_name(d), d.action_state, d.actor.speed_xz), (ANIM_WALK, ActionState::Walk, 1.5));
    assert!((49..=119).contains(&d.timer), "{}", d.timer);
    // The playback 0.75 of its speed (1.125).
    idle(&mut w, 1);
    assert_eq!(dodongo(&w, h).skel.play_speed, 1.125);
}

#[test]
fn walking_it_turns_to_link_within_400_of_its_home_else_it_wanders_untargetable() {
    let Some(a) = assets() else { return };
    let (mut w, h) = room5(&a, WEST);
    until(&mut w, h, 400, |d| d.action == Action::Walk);
    // Link in the corner (over 400 from home): untargetable, the timer counting.
    idle(&mut w, 1);
    let d = dodongo(&w, h);
    assert_eq!(d.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    let t = d.timer;
    idle(&mut w, 1);
    assert_eq!(dodongo(&w, h).timer, t - 1);
    // Link 250 from its home, behind it: targetable, turning to him at most 0x1F4 a frame
    // (Math_SmoothStepToS with scale 1), shape and world together; the timer held.
    link_at(&mut w, h, 250.0, 0x7000);
    for _ in 0..8 {
        let before = dodongo(&w, h).actor.world_rot.y;
        idle(&mut w, 1);
        let d = dodongo(&w, h);
        if d.action != Action::Walk {
            break;
        }
        let step = d.actor.world_rot.y.wrapping_sub(before);
        let to_link = d.actor.yaw_towards_player.wrapping_sub(before);
        assert!(step.unsigned_abs() <= 0x1F4 && (step == 0 || step.signum() == to_link.signum()), "step {step:#x}, to Link {to_link:#x}");
        assert_eq!(d.actor.shape_rot.y, d.actor.world_rot.y);
        assert_ne!(d.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
        assert_eq!(d.timer, t - 1);
    }
}

#[test]
fn it_breathes_fire_at_link_within_100_and_the_quad_burns_him() {
    let Some(a) = assets() else { return };
    let (mut w, h) = room5(&a, WEST);
    until(&mut w, h, 400, |d| d.action == Action::Walk);
    // Link 80 ahead, face to face: within 100, its face within 0x1388, not 60 above: the fire
    // (gDodongoBreatheFireAnim, morph 4), stopped.
    link_at(&mut w, h, 80.0, 0);
    let health = w.save.health;
    until(&mut w, h, 3, |d| d.action == Action::BreatheFire);
    let d = dodongo(&w, h);
    assert_eq!((anim_name(d), d.action_state, d.actor.speed_xz), (ANIM_BREATHE_FIRE, ActionState::BreatheFire, 0.0));
    // Frame by frame: a puff (EffectSsDFire_SpawnFixedScale) each frame from 29 to 43 (15), its
    // alpha 255 - 10 × (frame - 29) and fade delay frame - 26; the quad registered past 29 and
    // before 43, set by the draw (limb 2) then; NA_SE_EN_DODO_J_CRY at 24.
    let mut puffs = Vec::new();
    let mut quad_frames = Vec::new();
    loop {
        let before: Vec<i32> = fires(&w).iter().map(|e| e.life as i32).collect();
        let frame = dodongo(&w, h).skel.cur_frame;
        let quad = dodongo(&w, h).collider_at.dim.quad;
        idle(&mut w, 1);
        let d = dodongo(&w, h);
        if d.action != Action::BreatheFire {
            break;
        }
        // The spawned puff this frame: life 8 counted down once (7).
        let new: Vec<_> = fires(&w).into_iter().filter(|e| e.life == 7).map(d_fire::regs_of).collect();
        if (29.0..=43.0).contains(&frame) {
            assert_eq!(new.len(), 1, "frame {frame}: {before:?}");
            puffs.push((frame as i16, new[0]));
        } else {
            assert!(new.is_empty(), "frame {frame}");
        }
        if 29.0 < d.skel.cur_frame && d.skel.cur_frame < 43.0 && d.collider_at.dim.quad != quad {
            quad_frames.push(d.skel.cur_frame as i32);
        }
    }
    assert_eq!(puffs.len(), 15);
    for (f, (scale, _tex, alpha, fade)) in &puffs {
        let fire_frame = f - 29;
        // Its first update: scale 100 + 35; the alpha: life 7 above the fade delay, up 15.
        assert_eq!(*scale, 135);
        assert_eq!(*fade, fire_frame + 3);
        let a0 = 255 - fire_frame * 10;
        let expect = if fire_frame + 3 >= 7 { a0 - 5 } else { (a0 + 15).min(255) };
        assert_eq!(*alpha, expect, "frame {f}");
    }
    assert!(!quad_frames.is_empty());
    assert!(quad_frames.iter().all(|f| (30..=42).contains(f)), "{quad_frames:?}");
    assert_eq!(sfx_frames(&w, NA_SE_EN_DODO_J_CRY).len(), 1);
    assert_eq!(sfx_frames(&w, NA_SE_EN_DODO_J_FIRE - SFX_FLAG).len(), 15);
    // The fire hit Link (HIT_SPECIAL_EFFECT_FIRE, 16): hurt and burning.
    assert!(w.save.health < health, "{} of {}", w.save.health, health);
    // Then EnDodongo_EndBreatheFire (gDodongoAfterBreatheFireAnim once); done, idle for
    // Rand_S16Offset(10, 20).
    let d = dodongo(&w, h);
    assert_eq!((d.action, anim_name(d)), (Action::EndBreatheFire, ANIM_AFTER_BREATHE_FIRE));
    until(&mut w, h, 100, |d| d.action == Action::Idle);
    assert!((9..=29).contains(&dodongo(&w, h).timer), "{}", dodongo(&w, h).timer);
}

#[test]
fn link_burns_in_the_fire() {
    let Some(a) = assets() else { return };
    let (mut w, h) = room5(&a, WEST);
    until(&mut w, h, 400, |d| d.action == Action::Walk);
    link_at(&mut w, h, 80.0, 0);
    let mut burned = false;
    for _ in 0..60 {
        idle(&mut w, 1);
        burned |= w.player().body_is_burning;
    }
    assert!(burned);
}

#[test]
fn a_sword_hit_sweeps_its_tail_with_its_at_and_trail() {
    let Some(a) = assets() else { return };
    let (mut w, h) = room5(&a, WEST);
    // Link 200 behind it (0x8000 off its facing: ABS(yaw) 0x4000 or more).
    link_at(&mut w, h, 200.0, i16::MIN);
    // A Kokiri Sword's slash (the table's 1, reaction 0): red 8 frames, 3 health left,
    // EnDodongo_SetupSweepTail (gDodongoDamageAnim, NA_SE_EN_DODO_J_DAMAGE, timer 0).
    hit(&mut w, h, 1, 0);
    idle(&mut w, 1);
    let d = dodongo(&w, h);
    assert_eq!((d.action, d.action_state, d.actor.col_chk_info.health, anim_name(d)), (Action::SweepTail, ActionState::SweepTail, 3, ANIM_DAMAGE));
    assert_eq!(d.timer, 0);
    assert_eq!(sfx_frames(&w, NA_SE_EN_DODO_J_DAMAGE).len(), 1);
    // The damage animation done, Link behind: a sweep his way at 2 (NA_SE_EN_DODO_J_TAIL),
    // timer 18, tailSwipeSpeed (0xFFFF - |yaw|) / 15 (negative with him on the left), the tail's
    // AT on: the collider's AT_ON | AT_TYPE_ENEMY, its spheres 1 and 2 ATELEM_ON |
    // ATELEM_SFX_WOOD (the same bits), DMG_DEFAULT, 8.
    let mut yaw = 0i16;
    let mut shape = 0i16;
    until(&mut w, h, 100, |d| d.timer == 18);
    let d = dodongo(&w, h);
    assert_eq!(sfx_frames(&w, NA_SE_EN_DODO_J_TAIL).len(), 1);
    assert!(anim_name(d) == ANIM_SWEEP_TAIL_LEFT || anim_name(d) == ANIM_SWEEP_TAIL_RIGHT);
    assert_eq!(d.skel.play_speed, 2.0);
    assert_eq!(d.body_collider.base.at_flags & (AT_ON | AT_TYPE_ENEMY), AT_ON | AT_TYPE_ENEMY);
    for i in [1, 2] {
        let e = &d.body_collider.elements[i].info;
        assert_eq!(e.at_elem_flags & !ATELEM_HIT, ATELEM_ON | ATELEM_SFX_WOOD);
        assert_eq!((e.at_dmg_info.dmg_flags, e.at_dmg_info.damage), (DMG_DEFAULT, 8));
    }
    for i in [0, 3, 4, 5] {
        assert_eq!(d.body_collider.elements[i].info.at_elem_flags, ATELEM_NONE);
    }
    assert!(d.tail_swipe_speed != 0);
    let left = anim_name(d) == ANIM_SWEEP_TAIL_LEFT;
    assert_eq!(d.tail_swipe_speed < 0, left);
    let speed = d.tail_swipe_speed;
    let end_frame = d.skel.end_frame;
    // Sweeping: timer down a frame, turning tailSwipeSpeed; the trail gets vertices (limb 15,
    // timer 2 and up).
    let mut turns = 0;
    loop {
        let before = dodongo(&w, h).actor.world_rot.y;
        idle(&mut w, 1);
        let d = dodongo(&w, h);
        if d.action != Action::SweepTail {
            break;
        }
        if d.actor.world_rot.y != before {
            assert_eq!(d.actor.world_rot.y, before.wrapping_add(speed));
            assert_eq!(d.actor.shape_rot.y, d.actor.world_rot.y);
            turns += 1;
        }
        yaw = d.actor.world_rot.y;
        shape = d.actor.shape_rot.y;
        if d.timer >= 2 {
            assert!(blure(&w, d.blure_idx).is_some_and(|b| b.num_elements > 0));
        }
    }
    assert_eq!(yaw, shape);
    // A turn each frame until timer 1 (17) or the sweep's animation ends first (at 2 a frame to
    // its last frame: then the next update says it's done).
    assert_eq!(turns, 17.min((end_frame / 2.0).ceil() as i32), "end frame {end_frame}");
    // The sweep done (timer 1): the tail's AT off and the fire.
    let d = dodongo(&w, h);
    assert_eq!(d.action, Action::BreatheFire);
    assert_eq!(d.body_collider.base.at_flags & AT_ON, 0);
    for i in [1, 2] {
        let e = &d.body_collider.elements[i].info;
        assert_eq!((e.at_elem_flags, e.at_dmg_info.dmg_flags, e.at_dmg_info.damage), (ATELEM_NONE, 0, 0));
    }
}

#[test]
fn facing_link_after_a_hit_it_breathes_fire_instead() {
    let Some(a) = assets() else { return };
    let (mut w, h) = room5(&a, WEST);
    link_at(&mut w, h, 200.0, 0);
    hit(&mut w, h, 1, 0);
    idle(&mut w, 1);
    // The damage animation done with Link within 0x4000 of its facing: no sweep, the fire.
    until(&mut w, h, 100, |d| d.action != Action::SweepTail);
    assert_eq!(dodongo(&w, h).action, Action::BreatheFire);
    assert!(sfx_frames(&w, NA_SE_EN_DODO_J_TAIL).is_empty());
}

#[test]
fn the_metal_head_turns_a_hit_away() {
    let Some(a) = assets() else { return };
    let (mut w, h) = room5(&a, WEST);
    // The head's triangles bounced it (AC_BOUNCED) the same frame as a body hit: the body's hit
    // cancelled, the bounce cleared.
    hit(&mut w, h, 1, 0);
    dodongo_mut(&mut w, h).collider_hard.base.ac_flags |= AC_BOUNCED;
    idle(&mut w, 1);
    let d = dodongo(&w, h);
    assert_eq!((d.action, d.actor.col_chk_info.health), (Action::Idle, 4));
    assert_eq!(d.collider_hard.base.ac_flags & AC_BOUNCED, 0);
}

#[test]
fn a_deku_nut_stuns_it_for_80_frames() {
    let Some(a) = assets() else { return };
    let (mut w, h) = room5(&a, WEST);
    // Reaction 1 (the table's Deku Nut, 0 damage): blue 80 frames, EnDodongo_SetupStunned (the
    // fire held at 25, NA_SE_EN_GOMA_JR_FREEZE), no ice.
    hit(&mut w, h, 0, 1);
    idle(&mut w, 1);
    let d = dodongo(&w, h);
    assert_eq!((d.action, d.action_state, d.actor.col_chk_info.health, d.ice_timer), (Action::Stunned, ActionState::Stunned, 4, 0));
    assert_eq!((anim_name(d), d.skel.play_speed, d.skel.cur_frame), (ANIM_BREATHE_FIRE, 0.0, 25.0));
    assert_eq!(d.actor.color_filter_params & COLORFILTER_COLORFLAG_BLUE, COLORFILTER_COLORFLAG_BLUE);
    assert_eq!(sfx_frames(&w, NA_SE_EN_GOMA_JR_FREEZE).len(), 1);
    // Stunned, another nut does nothing.
    let timer = d.actor.color_filter_timer;
    hit(&mut w, h, 0, 1);
    idle(&mut w, 1);
    assert_eq!(dodongo(&w, h).actor.color_filter_timer, timer - 1);
    assert_eq!(sfx_frames(&w, NA_SE_EN_GOMA_JR_FREEZE).len(), 1);
    // The colour out (counted down before each update): idle again, health kept.
    let n = until(&mut w, h, 100, |d| d.action != Action::Stunned);
    assert_eq!(n, timer as usize - 1);
    assert_eq!((dodongo(&w, h).action, dodongo(&w, h).actor.col_chk_info.health), (Action::Idle, 4));
}

#[test]
fn ice_freezes_it_and_at_no_health_it_dies_when_it_thaws() {
    let Some(a) = assets() else { return };
    let (mut w, h) = room5(&a, WEST);
    // An ice arrow (4, 0xF): blue, the 4 damage (no health), stunned with 36 frames of ice: the
    // draw holds its colour (colorFilterTimer up one as the update takes one) and every 4 frames
    // a chunk (EffectSsEnIce_SpawnFlyingVec3f) at icePos[iceTimer >> 2]: 9.
    hit(&mut w, h, 4, 0xF);
    idle(&mut w, 1);
    let d = dodongo(&w, h);
    assert_eq!((d.action, d.actor.col_chk_info.health, d.damage_reaction), (Action::Stunned, 0, 0xF));
    let filter = d.actor.color_filter_timer;
    let mut chunks = 0;
    let mut seen = std::collections::HashSet::new();
    while dodongo(&w, h).ice_timer != 0 {
        idle(&mut w, 1);
        assert_eq!(dodongo(&w, h).actor.color_filter_timer, filter);
        for (i, e) in w.effect_ss.table.iter().enumerate() {
            if e.life > -1 && e.ty == EFFECT_SS_EN_ICE && seen.insert(i) {
                chunks += 1;
            }
        }
    }
    assert_eq!(chunks, 9);
    // Thawed, the colour out: no health, dead (gDodongoDieAnim, NA_SE_EN_DODO_J_DEAD,
    // untargetable).
    until(&mut w, h, 100, |d| d.action == Action::Death);
    let d = dodongo(&w, h);
    assert_eq!((anim_name(d), d.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED), (ANIM_DIE, 0));
    assert_eq!(sfx_frames(&w, NA_SE_EN_DODO_J_DEAD).len(), 1);
}

/// A light arrow's hit (`DMG_ARROW_LIGHT`) on its body sphere 5: `Actor_SetDropFlagJntSph` reads
/// its `acHitElem`.
fn light_arrow_hit(w: &mut PlayState, h: ActorHandle, damage: u8) {
    let player = w.player.unwrap();
    hit(w, h, damage, 0);
    let d = dodongo_mut(w, h);
    let at = ColliderElementDamageInfoAT { dmg_flags: DMG_ARROW_LIGHT, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 2 };
    let ac = d.body_collider.elements[5].info.ac_dmg_info;
    d.body_collider.elements[5].info.ac_hit_elem = Some(HitElem { elem: ElemRef { col: ColliderRef { actor: player, id: 0 }, elem: 0 }, at_dmg_info: at, ac_dmg_info: ac, elem_material: ELEM_MATERIAL_UNK0 });
}

#[test]
fn dead_it_blows_up_in_its_own_bomb_and_drops_from_table_4() {
    let Some(a) = assets() else { return };
    let (mut w, h) = room5(&a, WEST);
    // The Giant's Knife's 4 (no health left): red 8, EnDodongo_SetupDeath.
    hit(&mut w, h, 4, 0);
    idle(&mut w, 1);
    let d = dodongo(&w, h);
    assert_eq!((d.action, d.action_state, d.timer, d.actor.speed_xz), (Action::Death, ActionState::Death, 0, 0.0));
    assert_eq!(sfx_frames(&w, NA_SE_EN_DODO_J_DEAD).len(), 1);
    // Params -1: no smoke. NA_SE_EN_RIZA_DOWN at 52; the animation done, an En_Bom where it lies
    // (BOMB_BODY, shape.rot.z 6: its blast's growth) with its fuse at 0; timer 8.
    until(&mut w, h, 200, |d| d.timer == 7);
    assert_eq!(sfx_frames(&w, NA_SE_EN_RIZA_DOWN).len(), 1);
    let pos = dodongo(&w, h).actor.world_pos;
    let bombs: Vec<ActorHandle> = w.actors.all().into_iter().filter(|&b| w.actors.downcast::<EnBom>(b).is_some()).collect();
    assert_eq!(bombs.len(), 1);
    let b = w.actors.actor(bombs[0]).unwrap();
    assert!(b.world_pos.distance(pos) < 0.01);
    assert_eq!(b.shape_rot.z, 6);
    assert_eq!((b.params, w.actors.downcast::<EnBom>(bombs[0]).unwrap().timer), (BOMB_BODY, 0));
    // (Spawned after the explosives' updates: the fuse at 0 blows it in its next one,
    // BOMB_EXPLOSION.)
    idle(&mut w, 1);
    assert_eq!(w.actors.actor(bombs[0]).unwrap().params, 1);
    // 7 frames after the bomb's, the drop (table 4: any item an En_Item00 is of that table, or
    // none) and gone.
    let n = until_gone(&mut w, h, 20);
    assert_eq!(n, 6);
    let tables = &a.item_drops;
    let table4: Vec<i16> = tables.ids[DROP_TABLE_4 as usize..DROP_TABLE_4 as usize + 16].iter().map(|&i| i as i16).collect();
    for i in w.actors.all() {
        if let Some(item) = w.actors.actor(i).filter(|x| x.id == oot_actors::en_item00::ACTOR_EN_ITEM00)
            && item.home_pos.distance(pos) < 1.0
        {
            assert!(table4.contains(&item.params) || table4.contains(&(oot_actors::en_item00::ITEM00_FLEXIBLE)), "{:#x} not of table 4 {table4:?}", item.params);
        }
    }
}

#[test]
fn killed_by_a_light_arrow_it_drops_the_purple_rupee() {
    let Some(a) = assets() else { return };
    let (mut w, h) = room5(&a, WEST);
    // A light arrow's 2 twice: dropFlag 0x20 (Actor_SetDropFlagJntSph), so its
    // Item_DropCollectibleRandom (given the Dodongo) drops ITEM00_RUPEE_PURPLE.
    dodongo_mut(&mut w, h).actor.col_chk_info.health = 2;
    light_arrow_hit(&mut w, h, 2);
    idle(&mut w, 1);
    assert_eq!(dodongo(&w, h).action, Action::Death);
    assert_eq!(dodongo(&w, h).actor.drop_flag, 0x20);
    let pos = dodongo(&w, h).actor.world_pos;
    until_gone(&mut w, h, 300);
    let rupees = w.actors.all().into_iter().filter(|&i| w.actors.actor(i).is_some_and(|x| x.id == oot_actors::en_item00::ACTOR_EN_ITEM00 && x.params == oot_actors::en_item00::ITEM00_RUPEE_PURPLE && x.home_pos.distance(pos) < 1.0)).count();
    assert_eq!(rupees, 1);
}

#[test]
fn a_bomb_at_its_mouth_is_swallowed_and_blows_up_inside() {
    let Some(a) = assets() else { return };
    let (mut w, h) = room5(&a, WEST);
    // A bomb held by Link (his child: parent set) at its mouth isn't eaten
    // (EnDodongo_AteBomb skips an explosive with a parent).
    // (Its fuse is left at 70: the Dodongo holds it, and kills it at frame 28.)
    let mouth = dodongo(&w, h).mouth_pos;
    let b = w.actor_spawn(ACTOR_EN_BOM, mouth, [0; 3], 0).unwrap();
    let player = w.player;
    w.actors.actor_mut(b).unwrap().parent = player;
    idle(&mut w, 1);
    w.actors.actor_mut(b).unwrap().world_pos = dodongo(&w, h).mouth_pos;
    idle(&mut w, 1);
    assert_eq!(dodongo(&w, h).action, Action::Idle);
    // Thrown (no parent), at its mouth: eaten. An En_Bom: the Dodongo's child, its parent the
    // Dodongo; EnDodongo_SetupSwallowBomb (the fire backwards from 35, 25 frames).
    // (12 on from its mouth, inside the mouth's box: not overlapping its body's sphere 5 (35
    // across, about 35 behind the mouth): an enemy's OC sets a free bomb off, EnBom_Update.)
    {
        let d = dodongo(&w, h);
        let yaw = d.actor.shape_rot.y;
        let mouth = d.mouth_pos + Vec3::new(eng_math::sin_s(yaw), 0.0, eng_math::cos_s(yaw)) * 12.0;
        let bomb = w.actors.actor_mut(b).unwrap();
        bomb.parent = None;
        bomb.world_pos = mouth;
        bomb.velocity = Vec3::ZERO;
    }
    idle(&mut w, 1);
    let d = dodongo(&w, h);
    assert_eq!((d.action, d.action_state, d.timer, d.skel.cur_frame), (Action::SwallowBomb, ActionState::SwallowBomb, 25, 35.0));
    assert_eq!((d.actor.child, d.actor.parent), (Some(b), None));
    assert_eq!(w.actors.actor(b).unwrap().parent, Some(h));
    // EnDodongo_SwallowBomb: the bomb at its mouth, its fuse held (up one as its update takes one).
    // (mouthPos as the last draw left it: the draw after moves it.)
    let fuse = w.actors.downcast::<EnBom>(b).unwrap().timer;
    let mouth = dodongo(&w, h).mouth_pos;
    idle(&mut w, 1);
    let bomb = w.actors.downcast::<EnBom>(b).unwrap();
    assert_eq!(bomb.timer, fuse);
    assert_eq!(bomb.actor.world_pos, mouth);
    // Frame 28: NA_SE_EN_DODO_J_EAT, the bomb gone, its child cleared.
    until(&mut w, h, 20, |d| d.actor.child.is_none());
    assert!(w.actors.actor(b).is_none_or(|a| a.killed));
    assert_eq!(sfx_frames(&w, NA_SE_EN_DODO_J_EAT).len(), 1);
    // Held at 24 while timer counts down; at 10 the blast inside: NA_SE_IT_BOMB_EXPLOSION, red 8,
    // its body swelling with the red (bodyScale y, z 1 + sin(timer × 0x1000) / 2, x 1 + sin).
    until(&mut w, h, 40, |d| d.timer == 10);
    assert_eq!(dodongo(&w, h).skel.cur_frame as i32, 24);
    assert_eq!(sfx_frames(&w, NA_SE_IT_BOMB_EXPLOSION).len(), 1);
    idle(&mut w, 1);
    let d = dodongo(&w, h);
    let s = eng_math::sin_s((d.actor.color_filter_timer as i32 * 0x1000) as i16);
    assert_eq!((d.body_scale.x, d.body_scale.y, d.body_scale.z), (s + 1.0, s * 0.5 + 1.0, s * 0.5 + 1.0));
    assert_ne!(d.body_scale, Vec3::ONE);
    // timer 0: dead.
    let n = until(&mut w, h, 20, |d| d.action == Action::Death);
    assert_eq!(n, 9);
    // Its own bomb after the death animation, then gone.
    until_gone(&mut w, h, 200);
    let bombs = w.actors.all().into_iter().filter(|&x| w.actors.downcast::<EnBom>(x).is_some_and(|b| !b.actor.killed)).count();
    assert_eq!(bombs, 1);
}

#[test]
fn d_fire_update_and_draw() {
    let Some(a) = assets() else { return };
    let (mut w, _h) = room5(&a, WEST);
    // EffectSsDFire_SpawnFixedScale: scale 100, step 35, life 8; the texture
    // (state.frames % 4) ^ 3; prim (255, 255, 50, alpha).
    let pos = Vec3::new(-2000.0, 600.0, -300.0);
    let accel = Vec3::new(0.0, 0.0, 2.5);
    let frames = w.state_frames;
    w.with_ss(|s| s.d_fire_spawn_fixed_scale(pos, Vec3::ZERO, accel, 200, 5));
    let i = w.effect_ss.table.iter().position(|e| e.life > -1 && e.ty == EFFECT_SS_D_FIRE).unwrap();
    let e = &w.effect_ss.table[i];
    assert_eq!((e.life, e.priority), (8, 128));
    assert_eq!(d_fire::regs_of(e), (100, ((frames % 4) as i16) ^ 3, 200, 5));
    assert_eq!(e.gfx, Some(("object_dodongo", "gDodongoFireDL")));
    // EffectSsDFire_Update each frame (after the life's count and the move): the next texture,
    // scale + 35; life above the fade delay (5): alpha + 15 (to 255), else - 5.
    let mut tex = ((frames % 4) as i16) ^ 3;
    let mut alpha = 200i16;
    for k in 1..=9i16 {
        idle(&mut w, 1);
        let e = &w.effect_ss.table[i];
        if k == 9 {
            // Life 8 counted below 0: deleted (EffectSs_UpdateAll), after 9 updates.
            assert_eq!(e.life, -1);
            break;
        }
        tex = (tex + 1) & 3;
        alpha = if 5 >= 8 - k { (alpha - 5).max(0) } else { (alpha + 15).min(255) };
        assert_eq!(e.life, 8 - k);
        assert_eq!(d_fire::regs_of(e), (100 + 35 * k, tex, alpha, 5), "k {k}");
        // The draw: the bake of that texture at translate × billboard × scale (s, s, 1), its prim
        // and env (255, 0, 0, 0) colours dynamic.
        let name = oot_game::pack::keys::bake(&d_fire::bake_name(tex as usize));
        let cmd = w.effect_draws.xlu.iter().find(|c| c.mesh.name == name).expect("the puff's draw");
        let m = d_fire::draw_mtx(e, w.billboard_mtx());
        assert!(cmd.transform.abs_diff_eq(m, 1e-3));
        let sv = cmd.params.segments.as_ref().unwrap();
        assert_eq!(sv.prim[0x0E], Some([255, 255, 50, alpha as u8]));
        assert_eq!(sv.env[0x0E], Some([255, 0, 0, 0]));
    }
    // Without OBJECT_DODONGO (no slot), nothing spawns.
    let mut stops = Vec::new();
    let before = w.effect_ss.table.iter().filter(|e| e.life > -1).count();
    let mut s = oot_game::effect::SsSpawn { info: &mut w.effect_ss, rand: &mut w.rand, objects: None, stops: &mut stops, state_frames: 0 };
    s.d_fire_spawn_fixed_scale(pos, Vec3::ZERO, accel, 255, 3);
    assert_eq!(w.effect_ss.table.iter().filter(|e| e.life > -1).count(), before);
}
