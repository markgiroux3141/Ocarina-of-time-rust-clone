//! GAME-10 milestone 4 against the C: the Shabom (`z_en_bubble.c`). Master Quest's Jabu-Jabu
//! (`bdan`) room 0 places three (params 0xFFFF) at (-219, -240, -408), (-265, -99, -359) and
//! (-251, -190, -267).
//!
//! Child Link enters at `ENTR_JABU_JABU_0` (room 0) on a `jabu-fish` save. The tests spawn a
//! Shabom of their own (`Actor_Spawn`, `object_bubble` is room 0's) and run its update alone
//! (`Actor_UpdateAll`'s call), its state and hits set as the collision check would, so no fight is
//! scripted and nothing waits on `Rand`. Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_bubble::*;
use oot_game::actor::*;
use oot_game::actor_ctx::{ActorHandle, PLAYER_STATE1_28, PLAYER_STATE1_29};
use oot_game::audio::sfx::SfxPos;
use oot_game::collision_check::*;
use oot_game::effect::EFFECT_SS_DT_BUBBLE;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::shadow::ShadowDraw;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Room 0's three (`ootx scene-info --scene bdan --layer 0`).
const HOMES: [Vec3; 3] = [Vec3::new(-219.0, -240.0, -408.0), Vec3::new(-265.0, -99.0, -359.0), Vec3::new(-251.0, -190.0, -267.0)];

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// Child Link in Jabu-Jabu's room 0 (`jabu-fish`), the sound log on, two frames on.
fn jabu(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_JABU_JABU_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("jabu-fish").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    assert_eq!(w.room_ctx.cur.num, 0);
    // Walked in: until Link is his own again (no PLAYER_STATE1_28 or _29, no cutscene mode), so
    // play->damagePlayer reaches him.
    for _ in 0..200 {
        if w.player().state1 & (PLAYER_STATE1_28 | PLAYER_STATE1_29) == 0 && w.player().cs_mode == 0 {
            break;
        }
        idle(&mut w, 1);
    }
    assert_eq!(w.player().state1 & (PLAYER_STATE1_28 | PLAYER_STATE1_29), 0);
    w
}

fn bubbles(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnBubble>(h).is_some_and(|b| !b.actor.killed)).collect()
}

fn bb(w: &PlayState, h: ActorHandle) -> &EnBubble {
    w.actors.downcast::<EnBubble>(h).expect("En_Bubble")
}

fn bb_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnBubble {
    w.actors.downcast_mut::<EnBubble>(h).expect("En_Bubble")
}

fn gone(w: &PlayState, h: ActorHandle) -> bool {
    w.actors.downcast::<EnBubble>(h).is_none_or(|b| b.actor.killed)
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

/// The times a sound was asked for anywhere.
fn sfx_any(w: &PlayState, id: u16) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).count()
}

/// A Shabom of the test's own at `pos`, still: no bounce, no push, no sinking yet.
fn still_at(w: &mut PlayState, pos: Vec3) -> ActorHandle {
    let h = w.actor_spawn(ACTOR_EN_BUBBLE, pos, [0, 0, 0], -1).expect("the Shabom");
    let b = bb_mut(w, h);
    b.velocity_from_bounce = Vec3::ZERO;
    b.velocity_from_attack = Vec3::ZERO;
    b.actor.velocity = Vec3::ZERO;
    h
}

/// Open air near the homes: the first spot (by a fixed search around them, 40 apart) with
/// nothing within 30 of its middle up, down or sideways (the line `EnBubble_Fly` looks along).
fn open_air(w: &PlayState) -> Vec3 {
    for home in HOMES {
        for dy in [0.0, 40.0, 80.0] {
            for dx in [0.0, -40.0, 40.0, -80.0, 80.0] {
                for dz in [0.0, -40.0, 40.0, -80.0, 80.0] {
                    let p = home + Vec3::new(dx, dy, dz);
                    let mid = p + Vec3::Y * 16.0;
                    let open = [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z].iter().all(|&d| w.col.entity_line_test(mid, mid + d * 30.0, true, true, true, false).is_none());
                    if open && w.col.entity_raycast_down(p).0 > -10000.0 {
                        return p;
                    }
                }
            }
        }
    }
    panic!("no open air near the homes");
}

#[test]
fn room_0s_three_and_a_new_one_as_init_sets_them() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a);
    let placed = bubbles(&w);
    assert_eq!(placed.len(), 3);
    for home in HOMES {
        assert!(placed.iter().any(|&h| bb(&w, h).actor.home_pos == home), "{home:?}");
    }
    let h = w.actor_spawn(ACTOR_EN_BUBBLE, HOMES[1], [0, 0, 0], -1).expect("the Shabom");
    let b = bb(&w, h);
    // ACTOR_FLAG_ATTENTION_ENABLED only (not hostile); NAVI_ENEMY_SHABOM.
    assert_eq!(b.actor.flags & (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE), ACTOR_FLAG_ATTENTION_ENABLED);
    assert_eq!(b.actor.navi_enemy_id, NAVI_ENEMY_SHABOM);
    // ActorShape_Init(16, ActorShadow_DrawCircle, 0.2); EnBubble_SetDimensions(0): scale 1, the
    // wobble 16 and 0.08, no growth; unk_218 and unk_21C 1.
    assert_eq!((b.actor.shape_shadow_draw, b.actor.shape_shadow_scale, b.actor.shape_y_offset), (ShadowDraw::Circle, 0.2, 16.0));
    assert_eq!(b.actor.scale, Vec3::ONE);
    assert_eq!((b.graphic_rot_speed, b.graphic_eccentricity, b.expansion_width, b.expansion_height), (16.0, 0.08, 0.0, 0.0));
    assert_eq!((b.unk_218, b.unk_21c), (1.0, 1.0));
    // A random direction made 1 long (all three parts 0 to 1), set off at 3.
    let d = b.bounce_direction;
    assert!((d.length() - 1.0).abs() < 1e-5 && d.x >= 0.0 && d.y >= 0.0 && d.z >= 0.0, "{d:?}");
    assert_eq!(b.velocity_from_bounce, d * 3.0);
    assert_eq!((b.action, b.sink_speed, b.bounce_count, b.unk_208), (Action::Wait, 0.0, 0, 0));
    // sColChkInfoInit2 (1, 2, 25, 25, MASS_IMMOVABLE); DamageTable_Get(9).
    let c = &b.actor.col_chk_info;
    assert_eq!((c.health, c.cyl_radius, c.cyl_height, c.cyl_y_shift, c.mass), (1, 2, 25, 25, MASS_IMMOVABLE));
    assert_eq!(c.damage_table.map(|t| t.table), Some(DAMAGE_TABLE_9.table));
    // sJntSphInit: two spheres of 16; element 0's AT 4 but off, element 1 pushed only.
    let e = &b.collider.elements;
    assert_eq!((e[0].dim.model_sphere.radius, e[1].dim.model_sphere.radius), (16, 16));
    assert_eq!((e[0].info.at_elem_flags, e[0].info.at_dmg_info.damage), (ATELEM_NONE, 4));
    assert_eq!((e[0].info.ac_dmg_info.dmg_flags, e[1].info.ac_dmg_info.dmg_flags), (0xFFCF_D753, 0x0000_2824));
    assert_eq!(e[1].info.oc_elem_flags, OCELEM_NONE);
}

#[test]
fn drifting_it_sinks_faster_each_frame_and_is_hittable() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a);
    let p = open_air(&w);
    let h = still_at(&mut w, p);
    bb_mut(&mut w, h).velocity_from_bounce = Vec3::new(1.0, 0.0, 0.0);
    let (at, ac, oc) = (w.col_chk.col_at.len(), w.col_chk.col_ac.len(), w.col_chk.col_oc.len());
    for k in 1..=3 {
        update(&mut w, h);
        let b = bb(&w, h);
        // EnBubble_Fly: sinkSpeed 0.1 lower a frame; the velocity the bounce's plus the sinking.
        let sink = -0.1 * k as f32;
        assert!((b.sink_speed - sink).abs() < 1e-6, "frame {k}: {}", b.sink_speed);
        assert_eq!(b.actor.velocity, Vec3::new(1.0, b.sink_speed, 0.0));
        assert_eq!((b.action, b.actor.shape_y_offset), (Action::Wait, 16.0));
        // Actor_SetFocus(shape.yOffset).
        assert_eq!(b.actor.focus_pos, b.actor.world_pos + Vec3::Y * 16.0);
    }
    // EnBubble_Wait: AC and OC each frame, never AT.
    assert_eq!((w.col_chk.col_at.len(), w.col_chk.col_ac.len() - ac, w.col_chk.col_oc.len() - oc), (at, 3, 3));
    // minVelocityY (-20) holds the sinking.
    bb_mut(&mut w, h).sink_speed = -19.95;
    update(&mut w, h);
    assert_eq!(bb(&w, h).sink_speed, -20.0);
}

#[test]
fn on_the_floor_it_bounces_off_at_3_or_3_6() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a);
    let p = open_air(&w);
    let (floor, _) = w.col.entity_raycast_down(p);
    // 2 over the floor, going straight down (the bounce's 3 down, sinking 0.1).
    let pos = Vec3::new(p.x, floor + 2.0, p.z);
    let h = still_at(&mut w, pos);
    bb_mut(&mut w, h).velocity_from_bounce = Vec3::new(0.0, -3.0, 0.0);
    // BgCheck_EntityLineTest1 from its middle (16 up) 24 down: the floor's poly.
    let mid = pos + Vec3::Y * 16.0;
    let (_, poly) = w.col.entity_line_test(mid, mid - Vec3::Y * 24.0, true, true, true, false).expect("the floor");
    let want = vec3f_normalized_reflect(Vec3::new(0.0, -1.0, 0.0), w.col.poly_normal(poly));
    let bounds = sfx(&w, h, NA_SE_EN_AWA_BOUND);
    update(&mut w, h);
    let b = bb(&w, h);
    // Reflected off it, made 1 long; the count up to 1, back to 0 past a random 0 to 9: 3.6
    // when it's 0, else 3; the push and the sinking gone; NA_SE_EN_AWA_BOUND; the wobble.
    assert_eq!(b.bounce_direction, want);
    assert!(want.y > 0.9, "{want:?}");
    let speed = if b.bounce_count == 0 { 3.6000001 } else { 3.0 };
    assert!(b.bounce_count <= 1);
    assert_eq!(b.velocity_from_bounce, want * speed);
    assert_eq!((b.sink_speed, b.velocity_from_attack), (0.0, Vec3::ZERO));
    assert_eq!(b.actor.velocity, b.velocity_from_bounce);
    assert_eq!((b.graphic_rot_speed, b.graphic_eccentricity), (128.0, 0.48));
    assert_eq!(sfx(&w, h, NA_SE_EN_AWA_BOUND), bounds + 1);
    // Its draw eases the wobble back by a fifth (Math_SmoothStepToF, 0.2): 128 + (16 - 128) × 0.2.
    draw_update(&mut w, h);
    let b = bb(&w, h);
    assert!((b.graphic_rot_speed - 105.6).abs() < 1e-4, "{}", b.graphic_rot_speed);
    assert!((b.graphic_eccentricity - (0.48 + (0.08 - 0.48) * 0.2)).abs() < 1e-6);
}

#[test]
fn a_slingshot_seed_or_arrow_pushes_it_along_and_it_doesnt_burst() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a);
    let p = open_air(&w);
    let h = still_at(&mut w, p);
    // Element 1 hit (0x2824: ACELEM_NO_DAMAGE) by an attacker going +z at 5.
    let link = w.player.expect("Link");
    w.player_mut().actor.velocity = Vec3::new(0.0, 0.0, 5.0);
    {
        let b = bb_mut(&mut w, h);
        b.collider.base.ac_flags |= AC_HIT;
        b.collider.base.ac = Some(link);
        b.collider.elements[1].info.ac_elem_flags |= ACELEM_HIT;
    }
    update(&mut w, h);
    let b = bb(&w, h);
    // func_809CC648: the hit forgotten, a tenth of the attacker's velocity (unk_1F0), the wobble,
    // no burst. EnBubble_Fly: its velocity made 1 long, times 3, added to the push; then the push
    // eases 0.3 of the way to 0, 0.1 at most.
    assert_eq!((b.action, b.collider.base.ac_flags & AC_HIT, b.unk_208), (Action::Wait, 0, 0));
    assert_eq!(b.unk_1f0, Vec3::new(0.0, 0.0, 0.5));
    assert_eq!((b.graphic_rot_speed, b.graphic_eccentricity), (128.0, 0.48));
    assert_eq!(b.normalized_attacker_velocity, Vec3::new(0.0, 0.0, 1.0));
    assert_eq!(b.actor.velocity, Vec3::new(0.0, -0.1, 3.0));
    assert!((b.velocity_from_attack.z - 2.9).abs() < 1e-6, "{:?}", b.velocity_from_attack);
}

#[test]
fn touching_link_it_hurts_him_and_bursts() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a);
    let p = open_air(&w);
    let h = still_at(&mut w, p);
    w.player_mut().invincibility_timer = 0;
    let health = w.save.health;
    {
        let b = bb_mut(&mut w, h);
        b.collider.base.oc_flags2 |= OC2_HIT_PLAYER;
        b.actor.yaw_towards_player = 0x1234;
    }
    update(&mut w, h);
    let b = bb(&w, h);
    // EnBubble_DamagePlayer: play->damagePlayer(play, -4) (element 0's AT damage) and
    // Actor_SetPlayerKnockbackSmallNoDamage(6, yawTowardsPlayer, 6); 8 frames' wait;
    // func_809CBCBC: element 0 an explosive 4, on; stopped; six frames to EnBubble_Pop.
    assert_eq!(w.save.health, health - 4);
    let pl = w.player();
    assert_eq!((pl.knockback_type, pl.knockback_rot, pl.knockback_speed, pl.knockback_y_velocity, pl.knockback_damage), (1, 0x1234, 6.0, 6.0, 0));
    assert_eq!((b.action, b.explosion_countdown, b.unk_208), (Action::Pop, 6, 8));
    assert_eq!(b.collider.base.oc_flags2 & OC2_HIT_PLAYER, 0);
    let e = &b.collider.elements[0].info;
    assert_eq!((e.at_dmg_info.dmg_flags, e.at_dmg_info.damage, e.at_elem_flags), (DMG_EXPLOSIVE, 4, ATELEM_ON));
    assert_eq!(b.actor.velocity.y, 0.0);
    burst(&mut w, h);
}

/// `EnBubble_Pop` from its first frame: growing a tenth of the way to 5 times its size each
/// frame; at the sixth twenty bubbles, lock-on off, `NA_SE_EN_AWA_BREAK`, gone.
fn burst(w: &mut PlayState, h: ActorHandle) {
    let dt = effects(w, EFFECT_SS_DT_BUBBLE);
    let breaks = sfx_any(w, NA_SE_EN_AWA_BREAK);
    let mut width = bb(w, h).expansion_width;
    for k in 1..=5 {
        update(w, h);
        let b = bb(w, h);
        // Math_SmoothStepToF(&expansionWidth, 4, 0.1, 1000, 0).
        width += (4.0 - width) * 0.1;
        assert!((b.expansion_width - width).abs() < 1e-5, "frame {k}");
        assert_eq!(b.actor.shape_y_offset, (b.expansion_height + 1.0) * 16.0);
        assert_eq!((b.action, b.explosion_countdown), (Action::Pop, 6 - k));
        assert_eq!(effects(w, EFFECT_SS_DT_BUBBLE), dt);
    }
    update(w, h);
    assert!(gone(w, h));
    assert_eq!(bb(w, h).actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!(effects(w, EFFECT_SS_DT_BUBBLE), dt + 20);
    assert_eq!(sfx_any(w, NA_SE_EN_AWA_BREAK), breaks + 1);
}

#[test]
fn a_sword_or_anything_on_element_0_bursts_it_without_hurting_link() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a);
    let p = open_air(&w);
    let h = still_at(&mut w, p);
    let health = w.save.health;
    {
        let b = bb_mut(&mut w, h);
        b.collider.base.ac_flags |= AC_HIT;
        b.collider.elements[0].info.ac_elem_flags |= ACELEM_HIT;
    }
    update(&mut w, h);
    // func_809CC648: element 1 not hit: 8 frames' wait and the burst.
    let b = bb(&w, h);
    assert_eq!((b.action, b.explosion_countdown, b.unk_208), (Action::Pop, 6, 8));
    assert_eq!(w.save.health, health);
    // Bursting, hits aren't looked at (EnBubble_DetectPop) and it isn't hittable.
    let ac = w.col_chk.col_ac.len();
    burst(&mut w, h);
    assert_eq!(w.col_chk.col_ac.len(), ac);
}

#[test]
fn its_draw_puts_both_spheres_at_its_middle_and_scales_the_shadow() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a);
    let p = open_air(&w);
    let h = still_at(&mut w, p);
    {
        let b = bb_mut(&mut w, h);
        b.expansion_width = 0.5;
        b.actor.world_pos = Vec3::new(10.7, 20.2, -30.9);
    }
    draw_update(&mut w, h);
    // func_809CC774: the matrix's translation (16 up) truncated, 16 × 1.5 across; element 1 a
    // copy. The shadow (1.5 × 0.2).
    let b = bb(&w, h);
    for e in &b.collider.elements {
        assert_eq!((e.dim.world_sphere.center, e.dim.world_sphere.radius), ([10, 36, -30], 24));
    }
    assert!((b.actor.shape_shadow_scale - 0.3).abs() < 1e-6);
}

#[test]
fn the_unused_disappear_and_regrow_bring_it_back_from_nothing() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a);
    let p = open_air(&w);
    let h = still_at(&mut w, p);
    {
        let b = bb_mut(&mut w, h);
        b.action = Action::Disappear;
        b.explosion_countdown = 2;
    }
    // EnBubble_Disappear: func_809CBFD4's countdown, then func_809CBCEC (SetDimensions(-1), 12).
    update(&mut w, h);
    assert_eq!((bb(&w, h).action, bb(&w, h).explosion_countdown), (Action::Disappear, 1));
    update(&mut w, h);
    let b = bb(&w, h);
    assert_eq!((b.action, b.explosion_countdown, b.expansion_width), (Action::Regrow, 12, -1.0));
    assert_eq!(b.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_ATTENTION_ENABLED);
    // EnBubble_Regrow: a twelfth a frame (func_809CC020), drifting again after 12.
    let mut width = -1.0f32;
    for k in 1..=12 {
        update(&mut w, h);
        width += 1.0 / 12.0;
        let b = bb(&w, h);
        assert_eq!(b.expansion_width, width);
        assert_eq!(b.action, if k == 12 { Action::Wait } else { Action::Regrow }, "frame {k}");
    }
}
