//! GAME-08 milestone 3 against the C: the Guays (`z_en_crow.c`) over Lon Lon Ranch by night
//! (`spot20` layer 1: fifteen `En_Crow` 0x0000 in room 0, 259 to 438 up).
//!
//! Child Link enters the ranch (`ENTR_LON_LON_RANCH_0`) on a new save at 22:00; its entrance
//! cutscene is waited out. The Guays that act are spawned by Link (in view, so they update every
//! frame), each with a fresh init (`sDeathCount` reset). Expected values are worked out from the C
//! in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use eng_math::approach_s;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_crow::*;
use oot_actors::en_item00::{EnItem00, ITEM00_RUPEE_RED};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_HOSTILE, ACTOR_FLAG_UPDATE_CULLING_DISABLED, COLORFILTER_COLORFLAG_RED};
use oot_game::actor_ctx::ActorHandle;
use oot_game::collision_check::{AC_HIT, AC_ON};
use oot_game::effect::EFFECT_SS_DEAD_DB;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::target::{pitch_to, yaw_to};

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Child Link entered at the ranch at `hour`:00 (the sound log on), its entrance cutscene over.
fn ranch(a: &Arc<GameAssets>, hour: i32) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_LON_LON_RANCH_0").expect("entrance");
    let save = SaveContext::new(e, false, oot_game::env::clock_time(hour, 0) as u16);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    for _ in 0..600 {
        if !w.player_in_cs_mode() {
            break;
        }
        idle(&mut w, 1);
    }
    assert!(!w.player_in_cs_mode(), "the entrance's cutscene never ended");
    w
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn guays(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnCrow>(h).is_some_and(|r| !r.actor.killed)).collect()
}

fn guay(w: &PlayState, h: ActorHandle) -> &EnCrow {
    w.actors.downcast::<EnCrow>(h).expect("En_Crow")
}

fn guay_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnCrow {
    w.actors.downcast_mut::<EnCrow>(h).expect("En_Crow")
}

fn sfx_frames(w: &PlayState, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, _)| f).collect()
}

fn items(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnItem00>(h).is_some_and(|e| !e.actor.killed)).collect()
}

/// Where Link stands: the ranch's field under the lowest Guay's home (-727, 259, 789).
const LINK: Vec3 = Vec3::new(-727.0, 0.0, 789.0);

/// Link at `LINK` facing +z, and a new Guay `up` above him, `ahead` along +z, facing +z (yaw 0).
fn spawn_guay(w: &mut PlayState, ahead: f32, up: f32) -> ActorHandle {
    let (floor, _) = w.col.entity_raycast_down(LINK + Vec3::Y * 50.0);
    let link = Vec3::new(LINK.x, floor, LINK.z);
    w.place_player(link, 0);
    idle(w, 1);
    w.actor_spawn(ACTOR_EN_CROW, link + Vec3::new(0.0, up, ahead), [0, 0, 0], 0).expect("spawned")
}

/// A hit as `CollisionCheck_AC` leaves it for the next update: `AC_HIT`, the table's damage and
/// reaction in `colChkInfo`.
fn hit(w: &mut PlayState, h: ActorHandle, damage: u8, reaction: u8) {
    let g = guay_mut(w, h);
    g.collider.base.ac_flags |= AC_HIT;
    g.actor.col_chk_info.damage = damage;
    g.actor.col_chk_info.damage_reaction = reaction;
}

/// Frames until `cond`, at most `max`.
fn until(w: &mut PlayState, max: usize, mut cond: impl FnMut(&PlayState) -> bool) -> usize {
    for n in 0..max {
        if cond(w) {
            return n;
        }
        idle(w, 1);
    }
    panic!("not within {max} frames");
}

#[test]
fn fifteen_guays_by_night_and_none_by_day() {
    let Some(a) = assets() else { return };
    let w = ranch(&a, 22);
    // The night layer (1): fifteen En_Crow 0x0000 in room 0, 259 to 438 up.
    assert_eq!(w.save.scene_layer, 1);
    let hs = guays(&w);
    assert_eq!(hs.len(), 15);
    let mut heights: Vec<f32> = hs.iter().map(|&h| guay(&w, h).actor.home_pos.y).collect();
    heights.sort_by(f32::total_cmp);
    assert_eq!((heights[0], heights[14]), (259.0, 438.0));
    for &h in &hs {
        let g = guay(&w, h);
        assert_eq!((g.actor.params, g.actor.room), (0, 0));
        // sInitChain: cullingVolumeScale 3000, NAVI_ENEMY_GUAY (0x58), gravity -0.2,
        // lockOnArrowOffset 2000; ActorShape_Init: yOffset 2000; Actor_Init's scale 0.01.
        assert_eq!((g.actor.culling_volume_scale, g.actor.navi_enemy_id, g.actor.gravity, g.actor.target_arrow_offset), (3000.0, 0x58, -0.2, 2000.0));
        assert_eq!((g.actor.shape_y_offset, g.actor.scale), (2000.0, Vec3::splat(0.01)));
        // FLAGS: ATTENTION_ENABLED | HOSTILE | IGNORE_QUAKE (1 << 12) | CAN_ATTACH_TO_ARROW (1 << 14)
        // (and Actor_DrawAll's INSIDE_CULLING_VOLUME, 1 << 6, left out).
        assert_eq!(g.actor.flags & 0xFFBF, ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | (1 << 12) | (1 << 14));
        // sColChkInfoInit: 1 health, 15 by 30, mass 30; sJntSphInit: radius 20 (its world sphere
        // set at init), 8 damage.
        let c = &g.actor.col_chk_info;
        assert_eq!((c.health, c.cyl_radius, c.cyl_height, c.mass), (1, 15, 30, 30));
        let e = &g.collider.elements[0];
        assert_eq!((e.dim.model_sphere.radius, e.dim.world_sphere.radius, e.info.at_dmg_info.damage), (20, 20, 8));
    }
    // By day (layer 0) no Guay.
    let w = ranch(&a, 12);
    assert_eq!(w.save.scene_layer, 0);
    assert!(guays(&w).is_empty());
}

#[test]
fn the_damage_table_is_the_cs() {
    // sDamageTable: (damage, reaction) per DMG_* bit.
    let table: [(u8, u8); 32] = [
        (0, 1),
        (2, 0),
        (1, 0),
        (2, 0),
        (1, 0),
        (2, 0),
        (2, 0),
        (2, 0),
        (1, 0),
        (2, 0),
        (4, 0),
        (4, 2),
        (2, 3),
        (2, 0),
        (4, 0),
        (2, 0),
        (2, 0),
        (4, 2),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (1, 0),
        (4, 0),
        (2, 0),
        (2, 0),
        (8, 0),
        (4, 0),
        (0, 0),
        (0, 0),
        (4, 0),
        (0, 0),
    ];
    for (i, &(damage, reaction)) in table.iter().enumerate() {
        assert_eq!(S_DAMAGE_TABLE.table[i], (reaction << 4) | damage, "entry {i}");
    }
}

#[test]
fn it_idles_100_frames_then_dives_at_link_and_bites_him() {
    let Some(a) = assets() else { return };
    let mut w = ranch(&a, 22);
    // A Guay 150 above Link and 250 behind his front (it flies +z at 3 to 4.5).
    let h = spawn_guay(&mut w, -250.0, 150.0);
    let health = w.save.health;
    // EnCrow_SetupFlyIdle at init: timer 100. The spawn's frame doesn't update it (not yet in its
    // culling volume: Actor_DrawAll tests it after the updates); then 99 updates: still idle, 1
    // left, at 3 to 4.5.
    idle(&mut w, 100);
    let g = guay(&w, h);
    assert_eq!((g.action, g.timer), (Action::FlyIdle, 1));
    assert!((3.0..4.5).contains(&g.actor.speed_xz), "{}", g.actor.speed_xz);
    assert!(g.actor.xz_dist_to_player < 300.0, "{}", g.actor.xz_dist_to_player);
    // The 100th: Link within 300, not riding, the Guay out of water (no water: -32000): the dive
    // (EnCrow_SetupDiveAttack: 300 frames at 4, flapping at 2).
    idle(&mut w, 1);
    let g = guay(&w, h);
    assert_eq!((g.action, g.timer, g.actor.speed_xz, g.skel.play_speed), (Action::DiveAttack, 300, 4.0, 2.0));
    // Each frame of the dive, from the frame before: facing Link (|yawTowardsPlayer - rot.y| <
    // 0x2800), Math_ApproachS(rot.x, pitch at Link 20 up (at most 0x3000), 2, 0x400), else
    // Math_ApproachS(rot.x, -0x1000, 2, 0x100); facing or over 80 across,
    // Math_ApproachS(rot.y, yawTowardsPlayer, 4, 0xC00). Link stands still.
    let link = w.player().actor.world_pos;
    let mut bit = None;
    for n in 0..300 {
        let g = guay(&w, h);
        let (pos, mut rx, mut ry) = (g.actor.world_pos, g.actor.shape_rot.x, g.actor.shape_rot.y);
        let yaw_to_link = yaw_to(pos, link);
        let facing = (yaw_to_link.wrapping_sub(ry) as i32).abs() < 0x2800;
        if facing {
            approach_s(&mut rx, pitch_to(pos, link + Vec3::Y * 20.0).min(0x3000), 2, 0x400);
        } else {
            approach_s(&mut rx, -0x1000, 2, 0x100);
        }
        let xz = ((link.x - pos.x).powi(2) + (link.z - pos.z).powi(2)).sqrt();
        if facing || xz > 80.0 {
            approach_s(&mut ry, yaw_to_link, 4, 0xC00);
        }
        idle(&mut w, 1);
        if w.save.health < health {
            bit = Some(n);
            break;
        }
        let g = guay(&w, h);
        assert_eq!(g.action, Action::DiveAttack, "frame {n}");
        assert_eq!((g.actor.shape_rot.x, g.actor.shape_rot.y), (rx, ry), "frame {n}");
        // Its move: world.rot = (-rot.x, rot.y), Actor_MoveXYZ at 4.
        assert_eq!((g.actor.world_rot.x, g.actor.world_rot.y), (rx.wrapping_neg(), ry));
    }
    let n = bit.expect("it never bit Link");
    // Its sphere's AT (8, sJntSphElementsInit) on Link: 8 off his health; the next update sees
    // AT_HIT: NA_SE_EN_KAICHO_ATTACK and idle again (timer 100). Player updates first, so both
    // in the same frame.
    assert_eq!(health - w.save.health, 8, "after {n} frames");
    let g = guay(&w, h);
    assert_eq!((g.action, g.timer, g.skel.play_speed), (Action::FlyIdle, 100, 1.0));
    assert_eq!(sfx_frames(&w, NA_SE_EN_KAICHO_ATTACK), [w.audio.frames]);
}

#[test]
fn a_sword_kills_it_and_it_comes_back_300_frames_later() {
    let Some(a) = assets() else { return };
    let mut w = ranch(&a, 22);
    let h = spawn_guay(&mut w, 0.0, 150.0);
    idle(&mut w, 10);
    let before = items(&w);
    let speed = guay(&w, h).actor.speed_xz;
    let rot_x = guay(&w, h).actor.world_rot.x;
    let y = guay(&w, h).actor.world_pos.y;
    // The Kokiri Sword (1, reaction 0): EnCrow_UpdateDamage: Actor_ApplyDamage (1 to 0), no
    // targeting, Enemy_StartFinishingBlow (freezeFlashTimer 5), EnCrow_SetupDamaged: speed ×
    // cos(world.rot.x), velocity.y 0, 20 up, yOffset 0, lockOnArrowOffset 0,
    // NA_SE_EN_KAICHO_DEAD, red for 40, no AC, always updated.
    hit(&mut w, h, 1, 0);
    idle(&mut w, 1);
    let g = guay(&w, h);
    assert_eq!((g.action, g.actor.col_chk_info.health), (Action::Damaged, 0));
    assert_eq!(g.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_ne!(g.actor.flags & ACTOR_FLAG_UPDATE_CULLING_DISABLED, 0);
    assert_eq!(g.collider.base.ac_flags & AC_ON, 0);
    assert_eq!((g.actor.shape_y_offset, g.actor.target_arrow_offset), (0.0, 0.0));
    assert_ne!(g.actor.color_filter_params & COLORFILTER_COLORFLAG_RED, 0);
    assert!(w.actors.freeze_flash_timer > 0);
    assert_eq!(sfx_frames(&w, NA_SE_EN_KAICHO_DEAD), [w.audio.frames]);
    // Its first damaged frame: speed (× cos) less 0.5; 20 up then Actor_MoveXZGravity (health 0:
    // velocity.y -0.2, gravity's first step).
    let expect_speed = (speed * eng_math::cos_s(rot_x) - 0.5).max(0.0);
    assert!((g.actor.speed_xz - expect_speed).abs() < 1e-4, "{} vs {expect_speed}", g.actor.speed_xz);
    assert!((g.actor.world_pos.y - (y + 20.0 - 0.2)).abs() < 0.5, "{} from {y}", g.actor.world_pos.y);
    // EnCrow_Damaged: red, nose down (Math_ScaledStepToS to 0x4000 by 0x200 × R_UPDATE_RATE × 0.5:
    // 0x300) and spinning (rot.z + 0x1780 a frame).
    let (rx, rz) = (g.actor.shape_rot.x, g.actor.shape_rot.z);
    idle(&mut w, 1);
    let g = guay(&w, h);
    assert_eq!(g.actor.shape_rot.z, rz.wrapping_add(0x1780));
    assert!(g.actor.shape_rot.x == 0x4000 || g.actor.shape_rot.x == rx.wrapping_add(0x300), "{:#x} from {rx:#x}", g.actor.shape_rot.x);
    // Down on the ground: EffectSsDeadDb (scale 0.01 × 10000 = 100, scale step 0, white, red env,
    // life 9) and EnCrow_Die.
    until(&mut w, 200, |w| guay(w, h).action != Action::Damaged);
    let g = guay(&w, h);
    assert_eq!(g.action, Action::Die);
    let db: Vec<_> = w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == EFFECT_SS_DEAD_DB).collect();
    assert_eq!(db.len(), 1);
    let r = &db[0].regs;
    assert_eq!((r[0], r[9], r[11]), (100, 0, 9));
    assert!((db[0].pos - g.actor.world_pos).length() < 1.0);
    // EnCrow_Die from the next frame: the scale stepped from 0.01 to 0 by 0.002 a frame.
    let mut scale = g.actor.scale.x;
    assert_eq!(scale, 0.01);
    let mut n = 0;
    while guay(&w, h).action == Action::Die {
        idle(&mut w, 1);
        let g = guay(&w, h);
        if g.action == Action::Die {
            assert!((scale - g.actor.scale.x - 0.002).abs() < 1e-6, "{} after {scale}", g.actor.scale.x);
            assert_eq!((g.actor.scale.y, g.actor.scale.z), (g.actor.scale.x, g.actor.scale.x));
        }
        scale = g.actor.scale.x;
        n += 1;
        assert!(n < 10);
    }
    // At 0: sDeathCount 1, a drop from table 0 (Item_DropCollectibleRandom; none for some
    // entries), EnCrow_SetupRespawn: params 0, at home, flat, undrawn, timer 300.
    let g = guay(&w, h);
    assert_eq!((g.action, g.timer, g.actor.params, g.drawn, g.actor.scale.x), (Action::Respawn, 300, 0, false, 0.0));
    assert_eq!(g.actor.world_pos, g.actor.home_pos);
    assert_eq!((g.actor.shape_rot.x, g.actor.shape_rot.z, g.actor.shape_y_offset, g.actor.target_arrow_offset), (0, 0, 2000.0, 2000.0));
    assert_eq!(statics(&mut w).s_death_count, 1);
    let new: Vec<_> = items(&w).into_iter().filter(|i| !before.contains(i)).collect();
    let table = &a.item_drops.ids[0..16];
    println!("table 0 {table:?}; dropped {:?}", new.iter().map(|&i| w.actors.actor(i).unwrap().params).collect::<Vec<_>>());
    assert!(new.len() <= 1);
    if let Some(&i) = new.first() {
        // Table 0's item, or (func_8001F404) a recovery heart at full health a green rupee.
        let d = w.actors.actor(i).unwrap().params as u8;
        assert!(table.contains(&d) || (d == 0 && table.contains(&3)), "{d} from {table:?}");
    }
    // 300 frames at home undrawn (the 300th grows it): 299 still nothing.
    idle(&mut w, 299);
    let g = guay(&w, h);
    assert_eq!((g.action, g.timer, g.drawn, g.actor.scale.x), (Action::Respawn, 1, false, 0.0));
    // Then drawn, growing 0.001 a frame (a tenth of 0.01) to 0.01: targetable, culled again,
    // 1 health, idle.
    idle(&mut w, 1);
    let g = guay(&w, h);
    assert!(g.drawn);
    assert!((g.actor.scale.x - 0.001).abs() < 1e-6, "{}", g.actor.scale.x);
    let n = until(&mut w, 20, |w| guay(w, h).action != Action::Respawn);
    assert!((8..=10).contains(&n), "{n}");
    let g = guay(&w, h);
    assert_eq!((g.action, g.timer, g.actor.scale, g.actor.col_chk_info.health), (Action::FlyIdle, 100, Vec3::splat(0.01), 1));
    assert_ne!(g.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!(g.actor.flags & ACTOR_FLAG_UPDATE_CULLING_DISABLED, 0);
    assert_ne!(g.collider.base.ac_flags & AC_ON, 0);
}

#[test]
fn a_deku_nut_turns_it_away_for_100_frames() {
    let Some(a) = assets() else { return };
    let mut w = ranch(&a, 22);
    let h = spawn_guay(&mut w, 0.0, 150.0);
    idle(&mut w, 10);
    // A Deku Nut (0 damage, reaction 1): EnCrow_SetupTurnAway: timer 100, 3.5, climbing
    // (aimRotX -0x1000), away from Link, flapping at 2, blue for 5, NA_SE_EN_GOMA_JR_FREEZE; its
    // health kept. The same update's EnCrow_TurnAway counts the first frame.
    hit(&mut w, h, 0, 1);
    idle(&mut w, 1);
    let g = guay(&w, h);
    assert_eq!((g.action, g.timer, g.actor.speed_xz, g.aim_rot_x, g.skel.play_speed), (Action::TurnAway, 99, 3.5, -0x1000, 2.0));
    assert_eq!(g.actor.col_chk_info.health, 1);
    assert_eq!(g.actor.color_filter_params & COLORFILTER_COLORFLAG_RED, 0);
    assert_eq!(sfx_frames(&w, NA_SE_EN_GOMA_JR_FREEZE), [w.audio.frames]);
    // Turning away: Math_ApproachS(rot.y, yawTowardsPlayer + 0x8000, 3, 0xC00) each frame.
    for _ in 0..98 {
        let g = guay(&w, h);
        let mut ry = g.actor.shape_rot.y;
        let wall = g.actor.bg_check_flags & oot_game::actor::BGCHECKFLAG_WALL != 0;
        let away = yaw_to(g.actor.world_pos, w.player().actor.world_pos).wrapping_add(0x8000u16 as i16);
        approach_s(&mut ry, away, 3, 0xC00);
        idle(&mut w, 1);
        let g = guay(&w, h);
        assert_eq!(g.action, Action::TurnAway);
        if !wall {
            assert_eq!(g.actor.shape_rot.y, ry);
        }
    }
    assert_eq!(guay(&w, h).timer, 1);
    // Its hundredth frame: idle again (timer 100).
    idle(&mut w, 1);
    let g = guay(&w, h);
    assert_eq!((g.action, g.timer), (Action::FlyIdle, 100));
}

/// Kills `h` (a sword's hit) and runs until it's respawning.
fn kill(w: &mut PlayState, h: ActorHandle) {
    hit(w, h, 1, 0);
    idle(w, 1);
    until(w, 300, |w| guay(w, h).action == Action::Respawn);
}

#[test]
fn the_tenth_death_comes_back_big_and_drops_a_red_rupee() {
    let Some(a) = assets() else { return };
    let mut w = ranch(&a, 22);
    let h = spawn_guay(&mut w, 0.0, 150.0);
    idle(&mut w, 10);
    // Nine deaths counted (sDeathCount, every Guay's): the tenth (EnCrow_Die: 10) makes
    // EnCrow_SetupRespawn's Guay big: params 1, the count 0, its sphere 20 × 0.03f × 100.0f in
    // floats: 0.59999996 × 100 = 59.999996, 59 as an s16 (not 60).
    statics(&mut w).s_death_count = 9;
    kill(&mut w, h);
    let g = guay(&w, h);
    assert_eq!((g.actor.params, g.collider.elements[0].dim.world_sphere.radius), (1, 59));
    assert_eq!(statics(&mut w).s_death_count, 0);
    // It grows back to 0.03 by 0.003 a frame.
    until(&mut w, 400, |w| guay(w, h).action != Action::Respawn);
    let g = guay(&w, h);
    assert_eq!((g.action, g.actor.scale), (Action::FlyIdle, Vec3::splat(0.03)));
    // Its update scales by it: the sphere 20 × 3 above its position (scale × 100 × 20).
    idle(&mut w, 1);
    let g = guay(&w, h);
    let c = g.collider.elements[0].dim.world_sphere.center;
    assert_eq!(c[1], (g.actor.world_pos.y + 60.0) as i32 as i16);
    // Its death: shrinking by 0.006, not counted, a red rupee (Item_DropCollectible); then a
    // normal Guay again (params 0, its sphere 20).
    let before = items(&w);
    hit(&mut w, h, 1, 0);
    idle(&mut w, 1);
    until(&mut w, 300, |w| guay(w, h).action == Action::Die);
    let s0 = guay(&w, h).actor.scale.x;
    idle(&mut w, 1);
    if guay(&w, h).action == Action::Die {
        assert!((s0 - guay(&w, h).actor.scale.x - 0.006).abs() < 1e-6);
    }
    until(&mut w, 20, |w| guay(w, h).action == Action::Respawn);
    let g = guay(&w, h);
    assert_eq!((g.actor.params, g.collider.elements[0].dim.world_sphere.radius), (0, 20));
    assert_eq!(statics(&mut w).s_death_count, 0);
    let new: Vec<_> = items(&w).into_iter().filter(|i| !before.contains(i)).collect();
    assert_eq!(new.len(), 1);
    assert_eq!(w.actors.actor(new[0]).unwrap().params, ITEM00_RUPEE_RED);
}

#[test]
fn ten_guays_dying_together_make_one_big_one() {
    let Some(a) = assets() else { return };
    let mut w = ranch(&a, 22);
    // Ten new Guays (the last init resets the count) killed at once: each death counts one;
    // the respawn of the one counted tenth is big, the rest normal; the count back at 0.
    let hs: Vec<_> = (0..10).map(|i| spawn_guay(&mut w, i as f32 * 30.0 - 150.0, 150.0 + i as f32 * 10.0)).collect();
    idle(&mut w, 5);
    assert_eq!(statics(&mut w).s_death_count, 0);
    for &h in &hs {
        hit(&mut w, h, 1, 0);
    }
    idle(&mut w, 1);
    until(&mut w, 300, |w| hs.iter().all(|&h| guay(w, h).action == Action::Respawn));
    let big: Vec<_> = hs.iter().filter(|&&h| guay(&w, h).actor.params == 1).collect();
    assert_eq!(big.len(), 1);
    assert_eq!(statics(&mut w).s_death_count, 0);
}

#[test]
fn an_init_resets_the_death_count() {
    let Some(a) = assets() else { return };
    let mut w = ranch(&a, 22);
    let h = spawn_guay(&mut w, 0.0, 150.0);
    idle(&mut w, 5);
    kill(&mut w, h);
    assert_eq!(statics(&mut w).s_death_count, 1);
    // EnCrow_Init: sDeathCount = 0 (any Guay's, as a scene's Guays spawn).
    spawn_guay(&mut w, 100.0, 150.0);
    assert_eq!(statics(&mut w).s_death_count, 0);
    // A new scene: a fresh overlay (and its inits).
    let mut w = ranch(&a, 22);
    assert_eq!(statics(&mut w).s_death_count, 0);
}
