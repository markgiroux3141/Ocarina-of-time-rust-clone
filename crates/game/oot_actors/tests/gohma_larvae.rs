//! Gohma's eggs and larvae (GAME-05 milestone 3b) against the C: `En_Goma` (`z_en_goma.c`)
//! with its effects `Effect_Ss_Sibuki` and `Effect_Ss_K_Fire`, in the Deku Tree's first room
//! (room 0): its two eggs as the MQ scene places them (params 6 on a ledge at (402, 360, 197),
//! params 8 on the ceiling at (321, 981, 285)), and eggs spawned on the ground floor for the
//! larva's states.
//!
//! Expected values are worked out from the C in the comments. Where `Rand` decides what happens
//! (the stand's length, the fragments' frames), the test reads the generator or drives the
//! state directly.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_B, PadState};
use eng_math::{approach_f, smooth_step_to_f};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_goma::{ACTOR_BOSS_GOMA, ACTOR_EN_GOMA, Action, ENGOMA_EGG, ENGOMA_HATCH_DEBRIS, ENGOMA_NORMAL, EnGoma};
use oot_actors::en_item00::EnItem00;
use oot_game::actor::ACTOR_FLAG_ATTENTION_ENABLED;
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::sfx::*;
use oot_game::collision_check as cc;
use oot_game::effect::{EFFECT_SS_HAHEN, EFFECT_SS_K_FIRE, EFFECT_SS_SIBUKI};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn tick(w: &mut PlayState, p: PadState, prev: &mut PadState) {
    w.tick_with(scripted_input(*prev, p));
    *prev = p;
}

/// Inside the Deku Tree (`deku-tree-inside`: the Kokiri Sword and the Deku Shield), the sound
/// log on, the entrance's walk in over.
fn deku_tree(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-inside").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 60);
    w
}

fn sfx_on(w: &PlayState, frame: u32, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == frame && s == id)
}

fn goma(w: &PlayState, h: ActorHandle) -> &EnGoma {
    w.actors.downcast::<EnGoma>(h).expect("En_Goma")
}

fn goma_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnGoma {
    w.actors.downcast_mut::<EnGoma>(h).expect("En_Goma")
}

/// The En_Goma whose home is `pos`.
fn goma_at(w: &PlayState, pos: Vec3) -> ActorHandle {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnGoma>(h).is_some_and(|g| g.actor.home_pos == pos)).expect("the egg")
}

/// The En_Goma debris (params 10 to 24) whose parent is `h`.
fn debris_of(w: &PlayState, h: ActorHandle) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&d| w.actors.downcast::<EnGoma>(d).is_some_and(|g| g.goma_type == ENGOMA_HATCH_DEBRIS && g.actor.parent == Some(h))).collect()
}

const LEDGE_EGG: Vec3 = Vec3::new(402.0, 360.0, 197.0);
const CEILING_EGG: Vec3 = Vec3::new(321.0, 981.0, 285.0);
/// A spot on the ground floor (floor 0) away from the withered Deku Baba and the Keese.
const GROUND_EGG: Vec3 = Vec3::new(220.0, 0.0, -150.0);
/// Link 200 off the ground egg in z: not seen (`EnGoma_Egg` wants |dx| and |dz| under 100).
const LINK_AWAY: Vec3 = Vec3::new(220.0, 0.0, 50.0);
/// Link 90 off it in z: seen.
const LINK_NEAR: Vec3 = Vec3::new(220.0, 0.0, -60.0);

/// `Actor_Spawn(ACTOR_EN_GOMA, pos, params)`, as a room's list would.
fn spawn_egg(w: &mut PlayState, pos: Vec3, params: i16) -> ActorHandle {
    w.actor_spawn(ACTOR_EN_GOMA, pos, [0, 0, 0], params).expect("spawned")
}

/// An AC hit on the larva's or egg's `colliderCylinder2` with `dmg_flags`, as the collision
/// check leaves it (`AC_HIT`, `acHitElem`), for its next update.
fn hit(w: &mut PlayState, h: ActorHandle, dmg_flags: u32) {
    let player = w.player.unwrap();
    let g = goma_mut(w, h);
    g.collider_cylinder2.base.ac_flags |= cc::AC_HIT;
    g.collider_cylinder2.info.ac_hit_elem = Some(cc::HitElem {
        elem: cc::ElemRef { col: cc::ColliderRef { actor: player, id: 0 }, elem: 0 },
        at_dmg_info: cc::ColliderElementDamageInfoAT { dmg_flags, hit_special_effect: cc::HIT_SPECIAL_EFFECT_NONE, damage: 1 },
        ac_dmg_info: Default::default(),
        elem_material: 0,
    });
}

/// A ground egg hatched by Link coming near (`LINK_NEAR`): its handle once it's a larva.
fn hatched_larva(w: &mut PlayState) -> ActorHandle {
    let h = spawn_egg(w, GROUND_EGG, 7);
    w.place_player(LINK_NEAR, -0x8000);
    let mut n = 0;
    while goma(w, h).goma_type != ENGOMA_NORMAL {
        idle(w, 1);
        n += 1;
        assert!(n < 20, "it never hatched");
    }
    h
}

#[test]
fn the_eggs_init_as_the_c_says() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a);
    let ledge = goma_at(&w, LEDGE_EGG);
    let ceiling = goma_at(&w, CEILING_EGG);
    // En_Goma_Profile's id is ACTOR_BOSS_GOMA: Actor_Spawn gives every En_Goma that id.
    for h in [ledge, ceiling] {
        let g = goma(&w, h);
        assert_eq!(g.actor.id, ACTOR_BOSS_GOMA);
        // An egg: health 2, ENGOMA_EGG, EnGoma_Egg, eggScale 1; sInitChain's ATTENTION_RANGE_3
        // and lockOnArrowOffset 20.
        assert_eq!((g.goma_type, g.action, g.actor.col_chk_info.health, g.egg_scale), (ENGOMA_EGG, Action::Egg, 2, 1.0));
        assert_eq!((g.actor.target_mode, g.actor.target_arrow_offset), (3, 20.0));
        assert_ne!(g.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    }
    // eggYOffset: 1500 on the floor, -1500 on the ceiling (params >= 8).
    assert_eq!(goma(&w, ledge).egg_y_offset, 1500.0);
    assert_eq!(goma(&w, ceiling).egg_y_offset, -1500.0);
    // sSpawnNum++ for params 6 and 8, in init order: the room's actors wait for object_gol, and
    // Actor_UpdateAll initialises them newest first (the ceiling egg, listed after the ledge's,
    // is at the head of ACTORCAT_ENEMY).
    assert_eq!((goma(&w, ceiling).spawn_num, goma(&w, ledge).spawn_num), (0, 1));
    // A new egg's Rand calls: eggTimer = Rand_ZeroOne() * 200, then eggSquishAngle =
    // Rand_ZeroOne() * 1000; actionTimer 50. Params 7 takes no sSpawnNum, a 6 the next (2).
    let mut r = w.rand;
    let (timer, angle) = ((r.zero_one() * 200.0) as i16, r.zero_one() * 1000.0);
    let h7 = spawn_egg(&mut w, GROUND_EGG, 7);
    let g = goma(&w, h7);
    assert_eq!((g.egg_timer, g.egg_squish_angle, g.action_timer, g.spawn_num, g.egg_y_offset), (timer, angle, 50, 0, 1500.0));
    assert_eq!(w.rand, r);
    let h6 = spawn_egg(&mut w, GROUND_EGG + Vec3::new(-40.0, 0.0, 0.0), 6);
    assert_eq!(goma(&w, h6).spawn_num, 2);
    // A piece of debris (params 10 to 24): eggTimer, then velocity.y = Rand * 5 + 5, speed =
    // Rand * 2.3 + 1.5, each scale Rand * 0.005 + 0.01; gravity -1.3, actionTimer 30, not
    // targetable.
    let mut r = w.rand;
    let timer = (r.zero_one() * 200.0) as i16;
    let vy = r.zero_one() * 5.0 + 5.0;
    let speed = r.zero_one() * 2.3 + 1.5;
    let scale = Vec3::new(r.zero_one() * 0.005 + 0.01, r.zero_one() * 0.005 + 0.01, r.zero_one() * 0.005 + 0.01);
    let d = spawn_egg(&mut w, GROUND_EGG + Vec3::Y * 15.0, 10);
    let g = goma(&w, d);
    assert_eq!((g.goma_type, g.action, g.egg_timer), (ENGOMA_HATCH_DEBRIS, Action::Debris, timer));
    assert_eq!((g.actor.velocity.y, g.actor.speed_xz, g.actor.scale, g.actor.gravity, g.action_timer), (vy, speed, scale, -1.3, 30));
    assert_eq!(g.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
}

#[test]
fn an_egg_squishes_and_sheds_fragments_every_16_frames() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a);
    w.place_player(LINK_AWAY, -0x8000);
    let h = spawn_egg(&mut w, GROUND_EGG, 7);
    idle(&mut w, 1);
    let mut events = 0;
    for _ in 0..512 {
        let (angle0, amount0, timer0) = (goma(&w, h).egg_squish_angle, goma(&w, h).egg_squish_amount, goma(&w, h).egg_timer);
        idle(&mut w, 1);
        let g = goma(&w, h);
        // EnGoma_Egg: eggSquishAngle + 1, eggSquishAmount to 0.1 by 0.005 (Math_ApproachF(.., 1,
        // 0.005)); EnGoma_Update: eggTimer + 1. Link 200 off: not seen.
        let mut amount = amount0;
        approach_f(&mut amount, 0.1, 1.0, 0.005);
        assert_eq!((g.egg_squish_angle, g.egg_squish_amount, g.egg_timer), (angle0 + 1.0, amount, timer0.wrapping_add(1)));
        assert_eq!((g.action, g.player_detection_timer), (Action::Egg, 0));
        // The fragments of this frame (EffectSsHahen_Spawn, then EffectSs_UpdateAll's first
        // step: life 199, a fall of 0.5): only when eggTimer & 0xF was 0, two of them, 10 to 14
        // big, within 15 across and 30 up.
        let p = g.actor.world_pos;
        let new: Vec<_> = w.effect_ss.table.iter().filter(|e| e.ty == EFFECT_SS_HAHEN && e.life == 199 && (e.pos.x - p.x).abs() <= 15.0 && (e.pos.z - p.z).abs() <= 15.0).collect();
        if !new.is_empty() {
            events += 1;
            assert_eq!(timer0 & 0xF, 0, "fragments at eggTimer {timer0}");
            assert_eq!(new.len(), 2);
            for e in new {
                assert!((10..=14).contains(&e.regs[3]), "scale {}", e.regs[3]);
                assert!(e.pos.y >= p.y - 0.5 && e.pos.y < p.y + 30.0);
                assert_eq!((e.velocity, e.accel), (Vec3::new(0.0, -0.5, 0.0), Vec3::new(0.0, -0.5, 0.0)));
            }
        }
    }
    // 32 chances (a half each) in 512 frames. (128 frames' eight chances all missed once room 0's
    // torches joined the Rand calls in milestone 4a: how many come depends on Rand's stream.)
    assert!(events > 0);
}

#[test]
fn link_near_for_10_frames_hatches_the_egg_into_a_larva_and_its_shell() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a);
    w.place_player(LINK_AWAY, -0x8000);
    let h = spawn_egg(&mut w, GROUND_EGG, 7);
    idle(&mut w, 2);
    assert_ne!(goma(&w, h).actor.bg_check_flags & oot_game::actor::BGCHECKFLAG_GROUND, 0);
    // Link within 100 in x and z: playerDetectionTimer 1, 2, ...; past 9 (its 10th frame),
    // EnGoma_EggFallToGround.
    w.place_player(LINK_NEAR, -0x8000);
    for k in 1..=10 {
        idle(&mut w, 1);
        let g = goma(&w, h);
        assert_eq!(g.player_detection_timer, k);
        assert_eq!(g.action, if k < 10 { Action::Egg } else { Action::EggFallToGround }, "frame {k}");
    }
    // The next frame, on the ground (hatchState 0, params > 5): NA_SE_EN_GOMA_EGG1, then
    // EnGoma_SetupHatch: ENGOMA_NORMAL, EnGoma_Hatch for 5 frames, facing Link, speed 0,
    // Actor_SetScale(0.005) (then EnGoma_Update's Math_SmoothStepToF towards 0.01: +0.00075);
    // EnGoma_SpawnHatchDebris: NA_SE_EN_GOMA_EGG2, 15 pieces.
    idle(&mut w, 1);
    let f = w.audio.frames;
    assert!(sfx_on(&w, f, NA_SE_EN_GOMA_EGG1));
    assert!(sfx_on(&w, f, NA_SE_EN_GOMA_EGG2));
    let g = goma(&w, h);
    let mut scale = 0.005f32;
    smooth_step_to_f(&mut scale, 0.01, 0.5, 0.00075, 0.000001);
    assert_eq!((g.goma_type, g.action, g.action_timer, g.actor.scale.x, g.actor.speed_xz), (ENGOMA_NORMAL, Action::Hatch, 5, scale, 0.0));
    let egg_pos = g.actor.world_pos;
    assert_eq!(g.actor.shape_rot.y, eng_math::vec3f_yaw(egg_pos, w.player().actor.world_pos));
    // The pieces: params 10 to 24 (i + 10), its children, spawned at (x, y + 15, z) ±5
    // (Rand_CenteredFloat(10)), each still at its init (they update from the next frame).
    let debris = debris_of(&w, h);
    assert_eq!(debris.len(), 15);
    let mut params: Vec<i16> = debris.iter().map(|&d| goma(&w, d).actor.params).collect();
    params.sort();
    assert_eq!(params, (10..25).collect::<Vec<_>>());
    for &d in &debris {
        let g = goma(&w, d);
        assert_eq!((g.actor.id, g.action, g.action_timer), (ACTOR_BOSS_GOMA, Action::Debris, 30));
        let p = g.actor.world_pos - egg_pos;
        assert!(p.x.abs() <= 5.0 && p.z.abs() <= 5.0 && (p.y - 15.0).abs() <= 5.0, "{p:?}");
    }
    // Its 5 frames hatching, then EnGoma_SetupStand (Rand_S16Offset(10, 30) frames).
    for k in 1..=5 {
        idle(&mut w, 1);
        let g = goma(&w, h);
        if k < 5 {
            assert_eq!((g.action, g.action_timer), (Action::Hatch, 5 - k), "frame {k}");
        } else {
            assert_eq!(g.action, Action::Stand);
            assert!((10..40).contains(&g.action_timer), "{}", g.action_timer);
        }
    }
    // The pieces spin (EnGoma_Debris: shape.rot.y + 2500, x + 3500) and fall, and go on their
    // 30th update (actionTimer 30 down to 0: Actor_Kill).
    idle(&mut w, 24);
    assert!(debris.iter().all(|&d| w.actors.downcast::<EnGoma>(d).is_some_and(|g| !g.actor.killed)));
    idle(&mut w, 1);
    assert!(debris.iter().all(|&d| w.actors.downcast::<EnGoma>(d).is_none_or(|g| g.actor.killed)));
}

#[test]
fn the_larva_chases_link_at_10_thirds_and_jumps_at_him_within_150() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a);
    let h = hatched_larva(&mut w);
    // Link away (300 off): it stands out its timer, then EnGoma_SetupChasePlayer.
    w.place_player(GROUND_EGG + Vec3::new(0.0, 0.0, 300.0), -0x8000);
    let mut n = 0;
    while goma(&w, h).action != Action::ChasePlayer {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 50, "{:?}", goma(&w, h).action);
    }
    // EnGoma_ChasePlayer: speed to 10/3 (Math_ApproachF(.., 0.5, 2)).
    for k in 0..6 {
        let s0 = goma(&w, h).actor.speed_xz;
        idle(&mut w, 1);
        let g = goma(&w, h);
        assert_eq!(g.action, Action::ChasePlayer);
        let mut s = s0;
        approach_f(&mut s, 10.0 / 3.0, 0.5, 2.0);
        assert_eq!(g.actor.speed_xz, s, "frame {k}");
    }
    // Link within 150 across: EnGoma_SetupPrepareJump, 30 frames (eyes going red), then
    // EnGoma_SetupJump: velocity.y 8 (less the frame's gravity, -1.3), NA_SE_EN_GOMA_JR_CRY.
    let p = goma(&w, h).actor.world_pos;
    w.place_player(p + Vec3::new(0.0, 0.0, 120.0), -0x8000);
    idle(&mut w, 1);
    assert_eq!((goma(&w, h).action, goma(&w, h).action_timer), (Action::PrepareJump, 30));
    for k in 1..30 {
        idle(&mut w, 1);
        assert_eq!((goma(&w, h).action, goma(&w, h).action_timer), (Action::PrepareJump, 30 - k), "frame {k}");
    }
    // sTargetEyeEnvColors[..][0]: (255, 17, 0).
    let e = goma(&w, h).eye_env_color;
    assert!(e[0] > 250.0 && e[1] < 20.0 && e[2] < 5.0, "{e:?}");
    idle(&mut w, 1);
    let g = goma(&w, h);
    assert_eq!(g.action, Action::Jump);
    assert_eq!(g.actor.velocity.y, 8.0 - 1.3);
    assert!(sfx_on(&w, w.audio.frames, NA_SE_EN_GOMA_JR_CRY));
    // EnGoma_Jump: on at up to 10 (Math_ApproachF(.., 0.5, 5)) until it lands: EnGoma_SetupLand
    // (10 frames; or its bite landing on Link: the same, stopped), then standing.
    let mut n = 0;
    while goma(&w, h).action == Action::Jump {
        let s0 = goma(&w, h).actor.speed_xz;
        idle(&mut w, 1);
        if goma(&w, h).action == Action::Jump {
            let mut s = s0;
            approach_f(&mut s, 10.0, 0.5, 5.0);
            assert_eq!(goma(&w, h).actor.speed_xz, s);
        }
        n += 1;
        assert!(n < 40);
    }
    assert_eq!((goma(&w, h).action, goma(&w, h).action_timer), (Action::Land, 10));
    idle(&mut w, 10);
    assert_eq!(goma(&w, h).action, Action::Stand);
}

/// Holds the larva standing 36 in front of Link, as his sword reaches.
fn hold_in_front(w: &mut PlayState, h: ActorHandle) {
    let link = w.player().actor.world_pos;
    let g = goma_mut(w, h);
    g.actor.world_pos = link + Vec3::new(0.0, 0.0, -36.0);
    g.actor.prev_pos = g.actor.world_pos;
    g.actor.speed_xz = 0.0;
    g.action = Action::Stand;
    g.action_timer = 100;
}

/// B until the larva's health changes (the sword's hit read by `EnGoma_UpdateHit`), the larva held
/// in front of Link. Returns the frame.
fn slash(w: &mut PlayState, h: ActorHandle, prev: &mut PadState) -> u32 {
    let health = goma(w, h).actor.col_chk_info.health;
    let b = with(PadState::default(), BTN_B);
    for i in 0..40 {
        hold_in_front(w, h);
        tick(w, if i < 2 { b } else { PadState::default() }, prev);
        if goma(w, h).actor.col_chk_info.health != health {
            return w.audio.frames;
        }
    }
    panic!("the slash never hit");
}

#[test]
fn two_kokiri_sword_slashes_kill_a_larva_which_burns_away_and_drops_from_table_3() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a);
    let h = hatched_larva(&mut w);
    let items_before: Vec<ActorHandle> = w.actors.all().into_iter().filter(|&i| w.actors.downcast::<EnItem00>(i).is_some()).collect();
    let mut prev = PadState::default();
    // The first slash (DMG_SLASH_KOKIRI): CollisionCheck_GetSwordDamage 1, so health 2 - 1;
    // the bubbles (EffectSsSibuki_SpawnBurst at the focus); EnGoma_SetupHurt: 10 frames,
    // thrown back at 20 from Link, NA_SE_EN_GOMA_JR_DAM1; red for 5 frames; hurtTimer 13.
    let f = slash(&mut w, h, &mut prev);
    let g = goma(&w, h);
    assert_eq!(g.actor.col_chk_info.health, 1);
    assert_eq!((g.action, g.action_timer, g.hurt_timer), (Action::Hurt, 10, 13));
    assert!(sfx_on(&w, f, NA_SE_EN_GOMA_JR_DAM1));
    assert_eq!(g.actor.world_rot.y, g.actor.yaw_towards_player.wrapping_add(i16::MIN));
    assert_eq!(oot_game::actor::colorfilter_get_duration(g.actor.color_filter_params), 5);
    // EnGoma_OverrideLimbDraw while hurtTimer runs: the body's env colour three Rand_ZeroOne()
    // × 255, drawn once this game frame.
    assert!(g.hurt_body_env.is_some());
    // The burst: 30 bubbles, one side (Rand_ZeroOne() * 1.99), 40 big, moveDelay i / 6 + 1;
    // EffectSs_UpdateAll's first step took one off each delay (the first six are thrown).
    let bubbles: Vec<_> = w.effect_ss.table.iter().filter(|e| e.ty == EFFECT_SS_SIBUKI && e.life >= 0).collect();
    assert_eq!(bubbles.len(), 30);
    let dir = bubbles[0].regs[9];
    assert!(dir == 0 || dir == 1);
    let mut delays = [0; 5];
    for b in &bubbles {
        assert_eq!((b.regs[9], b.regs[10]), (dir, 40));
        // EffectSsSibuki_Init: life (s16)(Rand_ZeroOne() * 500 * 0.01) + 10, one taken.
        assert!((9..=13).contains(&b.life), "{}", b.life);
        delays[b.regs[8] as usize] += 1;
        if b.regs[8] == 0 {
            // EffectSsSibuki_Update, its delay out: along the camera's input yaw at 2 + 0.1 ×
            // Rand × 20 (to one side by `direction`), up at 7 + 0.1 × Rand × 20, falling by 1.
            let xz = (b.velocity.x * b.velocity.x + b.velocity.z * b.velocity.z).sqrt();
            assert!((2.0..4.01).contains(&xz) && (7.0..9.0).contains(&b.velocity.y), "{:?}", b.velocity);
            assert_eq!(b.accel.y, -1.0);
        }
    }
    assert_eq!(delays, [6; 5]);
    // 10 updates hurt (slowing on the ground), then EnGoma_SetupFlee (20 frames,
    // NA_SE_EN_GOMA_JR_DAM2). (The sword's hit stops play for a frame or two, the larva's
    // updates with it: the timers count its updates.)
    let mut timers = vec![goma(&w, h).action_timer];
    while goma(&w, h).action == Action::Hurt {
        tick(&mut w, PadState::default(), &mut prev);
        let g = goma(&w, h);
        if g.action == Action::Hurt && timers.last() != Some(&g.action_timer) {
            timers.push(g.action_timer);
        }
        assert!(w.audio.frames < f + 20);
    }
    assert_eq!(timers, (1..=10).rev().collect::<Vec<i16>>());
    assert_eq!((goma(&w, h).action, goma(&w, h).action_timer), (Action::Flee, 20));
    assert!(sfx_on(&w, w.audio.frames, NA_SE_EN_GOMA_JR_DAM2));
    // hurtTimer counts down from 13 (no hits meanwhile); then the second slash: health 0,
    // EnGoma_SetupHurt's 5 frames and Enemy_StartFinishingBlow (NA_SE_EN_LAST_DAMAGE).
    while goma(&w, h).hurt_timer != 0 {
        assert!(goma(&w, h).hurt_body_env.is_some());
        idle(&mut w, 1);
        assert!(w.audio.frames < f + 30);
    }
    assert_eq!(goma(&w, h).hurt_body_env, None);
    let f = slash(&mut w, h, &mut prev);
    let g = goma(&w, h);
    assert_eq!(g.actor.col_chk_info.health, 0);
    assert_eq!((g.action, g.action_timer), (Action::Hurt, 5));
    assert!(sfx_on(&w, f, NA_SE_EN_LAST_DAMAGE));
    // Then EnGoma_SetupDie: 30 frames, NA_SE_EN_GOMA_JR_DEAD, invincible for 100, not
    // targetable.
    let mut n = 0;
    while goma(&w, h).action == Action::Hurt {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 20);
    }
    let g = goma(&w, h);
    assert_eq!((g.action, g.action_timer, g.invincibility_timer), (Action::Die, 30, 100));
    assert_eq!(g.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert!(sfx_on(&w, w.audio.frames, NA_SE_EN_GOMA_JR_DEAD));
    // EnGoma_Die: at 17 left NA_SE_EN_GOMA_JR_LAND; at 0, EnGoma_SetupDead (actionTimer 3).
    while goma(&w, h).action_timer != 17 {
        idle(&mut w, 1);
        assert!(w.audio.frames < f + 60);
    }
    assert!(sfx_on(&w, w.audio.frames, NA_SE_EN_GOMA_JR_LAND));
    idle(&mut w, 17);
    assert_eq!((goma(&w, h).action, goma(&w, h).action_timer), (Action::Dead, 3));
    // EnGoma_Dead at 2 left: EffectSsKFire_Spawn(pos 5 down, 40, type 0): 100 frames, opaque,
    // scrolling by Rand_ZeroFloat(5) - 25.
    idle(&mut w, 1);
    let fire: Vec<_> = w.effect_ss.table.iter().filter(|e| e.ty == EFFECT_SS_K_FIRE && e.life >= 0).collect();
    assert_eq!(fire.len(), 1);
    let p = goma(&w, h).actor.world_pos;
    // EffectSsKFire_Update's first step: rXZScale and rYScale 4 (towards rScaleMax 40).
    assert_eq!((fire[0].life, fire[0].regs[0], fire[0].regs[6], fire[0].regs[3], fire[0].regs[5], fire[0].regs[4]), (99, 255, 40, 0, 4, 4));
    assert!((-25..=-21).contains(&fire[0].regs[2]));
    assert!((fire[0].pos - Vec3::new(p.x, p.y - 5.0 + 0.03, p.z)).length() < 30.0);
    let fire_slot = w.effect_ss.table.iter().position(|e| e.ty == EFFECT_SS_K_FIRE && e.life >= 0).unwrap();
    // From 0 left, the height shrinks (Math_SmoothStepToF(.., 0, 0.5, 0.00225, 0.00001), with
    // EnGoma_Update growing it back by up to 0.00075) to 0.001 or less: NA_SE_EN_EXTINCT,
    // Actor_Kill, Item_DropCollectibleRandom(table 3).
    idle(&mut w, 2);
    let mut n = 0;
    let mut last = Vec3::ZERO;
    while w.actors.downcast::<EnGoma>(h).is_some_and(|g| !g.actor.killed) {
        let g = goma(&w, h);
        last = g.actor.world_pos;
        let mut y = g.actor.scale.y;
        let left = smooth_step_to_f(&mut y, 0.0, 0.5, 0.00225, 0.00001);
        idle(&mut w, 1);
        n += 1;
        if left > 0.001 {
            let mut grown = y;
            smooth_step_to_f(&mut grown, 0.01, 0.5, 0.00075, 0.000001);
            assert_eq!(goma(&w, h).actor.scale.y, grown);
        }
        assert!(n < 60);
    }
    assert!(sfx_on(&w, w.audio.frames, NA_SE_EN_EXTINCT));
    // What table 3 drops (sItemDropIds[3 * 16 + Rand_ZeroOne() * 16]): one of the table's, or
    // for ITEM00_FLEXIBLE (16) what Link needs most (at full health, a table of rupees); as many
    // as sDropQuantities says, all the same, at the larva.
    let ids = &a.item_drops.ids[3 * 16..3 * 16 + 16];
    let new_items: Vec<&EnItem00> = w.actors.all().into_iter().filter(|i| !items_before.contains(i)).filter_map(|i| w.actors.downcast::<EnItem00>(i)).collect();
    for it in &new_items {
        assert!(it.actor.home_pos.distance(last) < 1.0, "{:?} vs {last:?}", it.actor.home_pos);
        assert_eq!(it.actor.params, new_items[0].actor.params);
        assert!(ids.contains(&(it.actor.params as u8)) || ids.contains(&16), "{} in {ids:?}", it.actor.params);
    }
    eprintln!("table 3: {ids:?}; dropped {:?}; shrank over {n} frames", new_items.iter().map(|i| i.actor.params).collect::<Vec<_>>());
    // The flame: 4 wider and taller a frame up to 40 (its 10th frame), then 10 less opaque a
    // frame from 255: gone at 0, 26 frames on (36 in all).
    let fire_frames = 3 + n as i16;
    let fire = &w.effect_ss.table[fire_slot];
    assert_eq!(fire.ty, EFFECT_SS_K_FIRE);
    if fire_frames >= 10 {
        assert_eq!((fire.regs[5], fire.regs[4], fire.regs[0]), (40, 40, 255 - 10 * (fire_frames - 10)));
    }
    idle(&mut w, (36 - fire_frames) as usize - 1);
    assert_eq!((w.effect_ss.table[fire_slot].ty, w.effect_ss.table[fire_slot].regs[0]), (EFFECT_SS_K_FIRE, 5));
    idle(&mut w, 1);
    assert_eq!(w.effect_ss.table[fire_slot].regs[0], 0);
    idle(&mut w, 1);
    assert_eq!(w.effect_ss.table[fire_slot].life, -1);
}

#[test]
fn a_hit_breaks_an_egg_before_it_hatches() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a);
    w.place_player(LINK_AWAY, -0x8000);
    let h = spawn_egg(&mut w, GROUND_EGG, 7);
    idle(&mut w, 2);
    // EnGoma_UpdateHit on an egg (not ENGOMA_NORMAL): EnGoma_SpawnHatchDebris
    // (NA_SE_EN_GOMA_EGG2, 15 pieces) and Actor_Kill; params 7 isn't Queen Gohma's.
    hit(&mut w, h, cc::DMG_SLASH_KOKIRI);
    idle(&mut w, 1);
    assert!(goma(&w, h).actor.killed);
    assert!(sfx_on(&w, w.audio.frames, NA_SE_EN_GOMA_EGG2));
    assert_eq!(debris_of(&w, h).len(), 15);
}

#[test]
fn a_deku_nut_stuns_a_larva_for_100_frames() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a);
    let h = hatched_larva(&mut w);
    w.place_player(GROUND_EGG + Vec3::new(0.0, 0.0, 300.0), -0x8000);
    idle(&mut w, 1);
    // DMG_DEKU_NUT: EnGoma_SetupStunned: stunTimer 100, its animation running (actionTimer)
    // (s16)Rand_ZeroFloat(15) + 3 frames, NA_SE_EN_GOMA_JR_FREEZE; hurtTimer 8.
    hit(&mut w, h, cc::DMG_DEKU_NUT);
    idle(&mut w, 1);
    let g = goma(&w, h);
    assert_eq!((g.action, g.stun_timer, g.hurt_timer, g.actor.col_chk_info.health), (Action::Stunned, 100, 8, 2));
    assert!((3..=17).contains(&g.action_timer));
    assert!(sfx_on(&w, w.audio.frames, NA_SE_EN_GOMA_JR_FREEZE));
    // EnGoma_Stunned: blue (Actor_SetColorFilter(BLUE, 180, OPA, 2)), the eyes grey; stunTimer
    // down by 1; under 30, shaking: +1.5 in x and z on odd, -1.5 on even.
    for k in 1..=100 {
        let x0 = goma(&w, h).actor.world_pos.x;
        idle(&mut w, 1);
        let g = goma(&w, h);
        assert_eq!((g.action, g.stun_timer), (Action::Stunned, 100 - k), "frame {k}");
        assert_eq!(oot_game::actor::colorfilter_get_duration(g.actor.color_filter_params), 2);
        if k >= 71 && g.actor.speed_xz == 0.0 {
            let dx = g.actor.world_pos.x - x0;
            assert_eq!(dx, if g.stun_timer & 1 != 0 { 1.5 } else { -1.5 }, "frame {k}");
        }
    }
    let e = goma(&w, h).eye_env_color;
    assert!(e.iter().all(|c| (c - 50.0).abs() < 1.0), "{e:?}");
    // stunTimer 0: EnGoma_SetupStand.
    idle(&mut w, 1);
    assert_eq!(goma(&w, h).action, Action::Stand);
}

#[test]
fn the_shield_knocks_a_larva_back_or_out_of_its_jump() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a);
    let h = hatched_larva(&mut w);
    w.place_player(GROUND_EGG + Vec3::new(0.0, 0.0, 300.0), -0x8000);
    idle(&mut w, 1);
    // Standing: DMG_SHIELD knocks it back along Link's facing (sShieldKnockbackVel (0, 0, 20)
    // turned by Matrix_RotateY(BINANG_TO_RAD_ALT(shape.rot.y))), invincible 5 frames.
    {
        let g = goma_mut(&mut w, h);
        g.action = Action::Stand;
        g.action_timer = 100;
        g.actor.speed_xz = 0.0;
    }
    hit(&mut w, h, cc::DMG_SHIELD);
    idle(&mut w, 1);
    let yaw = w.player().actor.shape_rot.y;
    let mut m = oot_game::sys_matrix::MtxF::IDENTITY;
    m.rotate_y(((yaw as f32 / 32768.0) as f64 * std::f64::consts::PI) as f32);
    let vel = m.mult_vec3f(Vec3::new(0.0, 0.0, 20.0));
    let g = goma(&w, h);
    assert_eq!((g.shield_knockback_vel, g.invincibility_timer, g.actor.col_chk_info.health), (vel, 5, 2));
    // EnGoma_Update: the position takes it, then Math_ApproachZeroF(.., 1, 3) on x and z.
    let p0 = g.actor.world_pos;
    idle(&mut w, 1);
    let g = goma(&w, h);
    let d = g.actor.world_pos - p0;
    assert!((d.z - vel.z).abs() < 0.5 && (d.x - vel.x).abs() < 0.5, "{d:?} vs {vel:?}");
    let mut k = vel;
    eng_math::approach_zero_f(&mut k.x, 1.0, 3.0);
    eng_math::approach_zero_f(&mut k.z, 1.0, 3.0);
    assert_eq!((g.shield_knockback_vel.x, g.shield_knockback_vel.z), (k.x, k.z));
    // Jumping (just up at 8, so EnGoma_Jump doesn't land it first): EnGoma_SetupLand, stopped
    // in the air, pushed back at 5.
    idle(&mut w, 5);
    {
        let g = goma_mut(&mut w, h);
        g.action = Action::Jump;
        g.actor.velocity.y = 8.0;
    }
    hit(&mut w, h, cc::DMG_SHIELD);
    idle(&mut w, 1);
    let g = goma(&w, h);
    assert_eq!((g.action, g.action_timer, g.actor.speed_xz, g.actor.velocity.y), (Action::Land, 10, -5.0, 0.0));
}

#[test]
fn the_ceiling_egg_falls_when_link_is_under_it() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a);
    let h = goma_at(&w, CEILING_EGG);
    // Link on the ledge under it (within 100 in x and z): 10 frames, then
    // EnGoma_EggFallToGround.
    w.place_player(Vec3::new(CEILING_EGG.x + 60.0, 400.0, CEILING_EGG.z - 40.0), 0);
    let mut n = 0;
    while goma(&w, h).action == Action::Egg {
        idle(&mut w, 1);
        n += 1;
        assert!(n <= 12);
    }
    assert_eq!(n, 10);
    assert_eq!(goma(&w, h).action, Action::EggFallToGround);
    // EnGoma_EggFallToGround: gravity -1.3 (velocity.y down by 1.3 a frame), the squish's
    // acceleration up by 0.03, eggYOffset from -1500 to 1500 by 150 a frame (the egg turns over
    // as it falls); on the ground it hatches (params 8 > 5).
    let mut k = 0;
    while goma(&w, h).goma_type == ENGOMA_EGG {
        let g = goma(&w, h);
        let (y0, vy0, off0, acc0) = (g.actor.world_pos.y, g.actor.velocity.y, g.egg_y_offset, g.egg_squish_accel);
        idle(&mut w, 1);
        k += 1;
        let g = goma(&w, h);
        if g.goma_type != ENGOMA_EGG {
            break;
        }
        let mut off = off0;
        approach_f(&mut off, 1500.0, 1.0, 150.0);
        assert_eq!((g.egg_y_offset, g.egg_squish_accel), (off, acc0 + 0.03), "frame {k}");
        if g.actor.bg_check_flags & oot_game::actor::BGCHECKFLAG_GROUND == 0 {
            assert_eq!(g.actor.velocity.y, (vy0 - 1.3).max(-20.0), "frame {k}");
            assert!(g.actor.world_pos.y < y0);
        }
        assert!(k < 80, "it never landed");
    }
    let g = goma(&w, h);
    assert_eq!(g.action, Action::Hatch);
    assert!(g.actor.world_pos.y < 450.0, "landed at {}", g.actor.world_pos.y);
    assert!(sfx_on(&w, w.audio.frames, NA_SE_EN_GOMA_EGG1));
    eprintln!("fell for {k} frames to {}", g.actor.world_pos.y);
}
