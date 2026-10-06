//! The Deku Tree's first enemies (GAME-05 milestone 3a) against the C: `En_Karebaba`
//! (`z_en_karebaba.c`) and `En_Firefly` (`z_en_firefly.c`), their states and damage tables,
//! on the Deku Tree's ground floor (room 0): the withered Deku Baba at (-88, 0, -363) and the
//! Keese perched at (-54, 262, -397) (both `params` as the MQ scene places them).
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_B, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_firefly::{Action as KeeseAction, EnFirefly};
use oot_actors::en_item00::EnItem00;
use oot_actors::en_karebaba::{Action as BabaAction, EnKarebaba};
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ACTORCAT_MISC, ActorHandle};
use oot_game::audio::sfx::*;
use oot_game::collision_check as cc;
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

/// The actor of type `T` whose home is nearest `pos`.
fn nearest<T: oot_game::actor_ctx::ActorImpl>(w: &PlayState, pos: Vec3) -> ActorHandle {
    w.actors
        .all()
        .into_iter()
        .filter(|&h| w.actors.downcast::<T>(h).is_some())
        .min_by(|&a, &b| {
            let d = |h| w.actors.actor(h).map(|x| x.home_pos.distance(pos)).unwrap_or(f32::MAX);
            d(a).total_cmp(&d(b))
        })
        .expect("the actor")
}

fn sfx_on(w: &PlayState, frame: u32, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == frame && s == id)
}

const BABA_HOME: Vec3 = Vec3::new(-88.0, 0.0, -363.0);
const KEESE_HOME: Vec3 = Vec3::new(-54.0, 262.0, -397.0);

fn baba(w: &PlayState, h: ActorHandle) -> &EnKarebaba {
    w.actors.downcast::<EnKarebaba>(h).expect("En_Karebaba")
}

/// `(DMG_SWORD | DMG_BOOMERANG) & ~DMG_JUMP_MASTER` (`collision_check.h`: `DMG_SWORD` the
/// slashes, spins and jump slashes of the three swords, `DMG_BOOMERANG` 1 << 4).
const CHILD_UPRIGHT_DMG: u32 = ((1 << 8 | 1 << 9 | 1 << 10 | 1 << 22 | 1 << 23 | 1 << 24 | 1 << 25 | 1 << 26 | 1 << 27) | 1 << 4) & !(1 << 27);

#[test]
fn a_withered_deku_baba_springs_up_sways_and_spins() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a);
    let h = nearest::<EnKarebaba>(&w, BABA_HOME);
    assert_eq!(w.actors.actor(h).unwrap().home_pos, BABA_HOME);
    // EnKarebaba_SetupIdle (params 1): scale 0.005, head pitched down (-0x4000), 14 up.
    let b = baba(&w, h);
    assert_eq!((b.action, b.actor.scale.x, b.actor.shape_rot.x, b.actor.world_pos.y), (BabaAction::Idle, 0.005, -0x4000, 14.0));
    // DamageTable_Get(1), sColCheckInfoInit { 1, 15, 80, MASS_HEAVY }.
    assert_eq!(b.actor.col_chk_info.damage_table.unwrap().table, oot_actors::en_karebaba::S_DAMAGE_TABLE_PRESET_1.table);
    assert_eq!(b.actor.col_chk_info.health, 1);
    // Link 250 off: idle (EnKarebaba_Idle wants xzDistToPlayer < 200, |yDistToPlayer| < 30).
    w.place_player(BABA_HOME + Vec3::new(0.0, 0.0, 250.0), -0x8000);
    idle(&mut w, 5);
    assert_eq!(baba(&w, h).action, BabaAction::Idle);
    // 150 off: EnKarebaba_SetupAwaken, NA_SE_EN_DUMMY482.
    w.place_player(BABA_HOME + Vec3::new(0.0, 0.0, 150.0), -0x8000);
    let mut awoke = None;
    for i in 0..3 {
        idle(&mut w, 1);
        if baba(&w, h).action == BabaAction::Awaken {
            awoke = Some(i);
            break;
        }
    }
    assert!(awoke.is_some());
    assert!(sfx_on(&w, w.audio.frames, NA_SE_EN_DUMMY482));
    // EnKarebaba_Awaken: up from 14 to 60 by 5 a frame (Math_StepToF: 9 steps short, the 10th
    // there), the scale to 0.01 by 0.0005, the head spinning by 0x1999, a fragment from home
    // each frame (EffectSsHahen_SpawnBurst, count 1); then EnKarebaba_SetupUpright.
    for k in 1..=10 {
        let rot0 = baba(&w, h).actor.shape_rot.y;
        idle(&mut w, 1);
        let b = baba(&w, h);
        assert_eq!(b.actor.world_pos.y, (14.0 + 5.0 * k as f32).min(60.0), "frame {k}");
        assert_eq!(b.actor.shape_rot.y, rot0.wrapping_add(0x1999));
        if k < 10 {
            assert_eq!(b.action, BabaAction::Awaken, "frame {k}");
        }
    }
    // EnKarebaba_SetupUpright: full size, the body soft (COL_MATERIAL_HIT6, not AC_HARD) and
    // only the sword and the boomerang hurt it (the child's flags without DMG_JUMP_MASTER), 15
    // by 80, the head 80 high, params 40.
    let b = baba(&w, h);
    assert_eq!(b.action, BabaAction::Upright);
    assert_eq!(b.actor.scale.x, 0.01);
    assert_eq!(b.body_collider.base.col_type, cc::COL_MATERIAL_HIT6);
    assert_eq!(b.body_collider.base.ac_flags & cc::AC_HARD, 0);
    assert_eq!(b.body_collider.info.ac_dmg_info.dmg_flags, CHILD_UPRIGHT_DMG);
    assert_eq!((b.body_collider.dim.radius, b.body_collider.dim.height, b.head_collider.dim.height), (15, 80, 80));
    assert_eq!(b.actor.params, 40);
    // EnKarebaba_Upright: params counts down to 0, then EnKarebaba_SetupSpin (params 40).
    for k in 1..=40 {
        idle(&mut w, 1);
        let b = baba(&w, h);
        if k < 40 {
            assert_eq!((b.action, b.actor.params), (BabaAction::Upright, 40 - k), "frame {k}");
        } else {
            assert_eq!((b.action, b.actor.params), (BabaAction::Spin, 40));
        }
    }
    // EnKarebaba_Spin: params down from 40; value = min(20 - |20 - params|, 10): the head
    // collider 4 + 2 value wide, pitched 0xC000 - value * 0x100, turning by value * 0x2C0, 60
    // from home along its pitch and yaw.
    for k in 1..=40 {
        let rot_y0 = baba(&w, h).actor.shape_rot.y;
        idle(&mut w, 1);
        let b = baba(&w, h);
        if k == 40 {
            // Back to EnKarebaba_SetupUpright (already full size: params 40 only).
            assert_eq!((b.action, b.actor.params), (BabaAction::Upright, 40));
            break;
        }
        let params = 40 - k;
        let value = (20 - (20 - params as i32).abs()).min(10);
        assert_eq!(b.actor.params, params as i16);
        assert_eq!(b.head_collider.dim.radius, 4 + (value * 2) as i16, "frame {k}");
        let rot_x = (0xC000u16 as i16).wrapping_sub((value * 0x100) as i16);
        assert_eq!(b.actor.shape_rot.x, rot_x);
        assert_eq!(b.actor.shape_rot.y, rot_y0.wrapping_add((value * 0x2C0) as i16));
        let cos60 = eng_math::cos_s(rot_x) * 60.0;
        let want = Vec3::new(eng_math::sin_s(b.actor.shape_rot.y) * cos60 + BABA_HOME.x, eng_math::sin_s(rot_x) * -60.0 + BABA_HOME.y, eng_math::cos_s(b.actor.shape_rot.y) * cos60 + BABA_HOME.z);
        assert_eq!(b.actor.world_pos, want, "frame {k}");
    }
    // Link 250 from home (Math_Vec3f_DistXZ > 240): EnKarebaba_SetupRetract, down to 14 by 5
    // (from 60: 10 frames) and the scale to 0.005, then idle again with the hard body.
    w.place_player(BABA_HOME + Vec3::new(0.0, 0.0, 250.0), -0x8000);
    idle(&mut w, 1);
    assert_eq!(baba(&w, h).action, BabaAction::Retract);
    let mut n = 0;
    while baba(&w, h).action == BabaAction::Retract {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 20);
    }
    let b = baba(&w, h);
    assert_eq!(n, 10);
    assert_eq!((b.action, b.actor.world_pos.y, b.actor.scale.x), (BabaAction::Idle, 14.0, 0.005));
    // EnKarebaba_ResetCollider.
    assert_eq!(b.body_collider.base.col_type, cc::COL_MATERIAL_HARD);
    assert_ne!(b.body_collider.base.ac_flags & cc::AC_HARD, 0);
    assert_eq!((b.body_collider.dim.radius, b.body_collider.dim.height, b.head_collider.dim.height), (7, 25, 25));
}

#[test]
fn a_sword_slash_kills_the_withered_deku_baba_which_leaves_a_stick_and_regrows() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a);
    let h = nearest::<EnKarebaba>(&w, BABA_HOME);
    // Link 40 in front of it (+z), facing it (yaw 0x8000: -z).
    w.place_player(BABA_HOME + Vec3::new(0.0, 0.0, 40.0), -0x8000);
    let mut prev = PadState::default();
    let mut n = 0;
    while baba(&w, h).action != BabaAction::Upright {
        tick(&mut w, PadState::default(), &mut prev);
        n += 1;
        assert!(n < 30);
    }
    // B: the Kokiri Sword's slash (DMG_SLASH_KOKIRI) on the body (COL_MATERIAL_HIT6).
    let b_press = with(PadState::default(), BTN_B);
    let mut died = None;
    for i in 0..30 {
        tick(&mut w, if i < 2 { b_press } else { PadState::default() }, &mut prev);
        if baba(&w, h).action == BabaAction::Dying {
            died = Some(w.audio.frames);
            break;
        }
    }
    let f = died.expect("the slash killed it");
    // EnKarebaba_SetupDying: params 0, gravity -0.8, velocity.y 4 (its first fall this frame),
    // flying back from where its head faced (world.rot.y = shape.rot.y + 0x8000) at 3,
    // NA_SE_EN_DEKU_JR_DEAD; Enemy_StartFinishingBlow.
    let b = baba(&w, h);
    assert_eq!(b.actor.params, 0);
    assert_eq!(b.actor.gravity, -0.8);
    assert_eq!(b.actor.world_rot.y, b.actor.shape_rot.y.wrapping_add(-0x8000i16));
    assert!(sfx_on(&w, f, NA_SE_EN_DEKU_JR_DEAD));
    // EnKarebaba_Dying until it lands (BGCHECKFLAG_GROUND_TOUCH): NA_SE_EN_DODO_M_GND, scale 0
    // (it was over 0.005), not targetable or hostile, a 15-fragment burst; params 1, and the
    // next frame dust along the stem (func_800286CC, 4 and one at home) and
    // EnKarebaba_SetupDeadItemDrop: scale 0.03, ACTORCAT_MISC, params 200.
    let mut n = 0;
    while baba(&w, h).action == BabaAction::Dying {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 120, "it never landed");
    }
    let b = baba(&w, h);
    assert_eq!(b.action, BabaAction::DeadItemDrop);
    assert_eq!(b.actor.scale.x, 0.03);
    assert_eq!(b.actor.category, ACTORCAT_MISC);
    assert!(w.actors.category(ACTORCAT_MISC).contains(&h));
    // (EnKarebaba_SetupDeadItemDrop ran in ACTORCAT_ENEMY's turn and put it at the head of
    // ACTORCAT_MISC's list, so Actor_UpdateAll updates it again in MISC's turn: its first
    // EnKarebaba_DeadItemDrop, params 199.)
    assert_eq!(b.actor.params, 199);
    assert_eq!(b.actor.flags & (oot_game::actor::ACTOR_FLAG_ATTENTION_ENABLED | oot_game::actor::ACTOR_FLAG_HOSTILE), 0);
    assert!(w.audio.log.as_ref().unwrap().sfx.iter().any(|&(fr, s, _)| fr > f && s == NA_SE_EN_DODO_M_GND));
    // EnKarebaba_DeadItemDrop: params down 1 a frame (offering GI_DEKU_STICKS_1 nearby, not
    // taken here), to 0: EnKarebaba_SetupDead, back home (params 200).
    for _ in 0..198 {
        idle(&mut w, 1);
        assert_eq!(baba(&w, h).action, BabaAction::DeadItemDrop);
    }
    idle(&mut w, 1);
    let b = baba(&w, h);
    assert_eq!((b.action, b.actor.params, b.actor.world_pos), (BabaAction::Dead, 200, BABA_HOME));
    // EnKarebaba_Dead 200 frames, then EnKarebaba_Regrow over 20 (scale 0.005 * params * 0.05),
    // then idle and an enemy again.
    idle(&mut w, 200);
    assert_eq!(baba(&w, h).action, BabaAction::Regrow);
    for k in 1..20 {
        idle(&mut w, 1);
        let b = baba(&w, h);
        assert_eq!(b.actor.scale.x, 0.005 * (k as f32 * 0.05), "frame {k}");
    }
    idle(&mut w, 1);
    let b = baba(&w, h);
    assert_eq!(b.action, BabaAction::Idle);
    assert_eq!(b.actor.category, ACTORCAT_ENEMY);
    assert_ne!(b.actor.flags & oot_game::actor::ACTOR_FLAG_HOSTILE, 0);
}

fn keese(w: &PlayState, h: ActorHandle) -> &EnFirefly {
    w.actors.downcast::<EnFirefly>(h).expect("En_Firefly")
}

#[test]
fn a_perched_keese_dives_at_link_then_flies_about() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a);
    let h = nearest::<EnFirefly>(&w, KEESE_HOME);
    // EN_FIREFLY_TYPE_NORMAL_PERCHED (3): EnFirefly_Perched, homeY 100 above home, sDamageTable,
    // sColChkInfoInit { 1, 10, 10, 30 }.
    let k = keese(&w, h);
    assert_eq!((k.actor.params, k.action, k.home_y), (3, KeeseAction::Perched, KEESE_HOME.y + 100.0));
    assert_eq!(k.actor.col_chk_info.damage_table.unwrap().table, oot_actors::en_firefly::S_DAMAGE_TABLE.table);
    assert_eq!(k.actor.col_chk_info.health, 1);
    // Link 150 across: still perched (EnFirefly_Perched wants xzDistToPlayer < 120).
    w.place_player(KEESE_HOME * Vec3::new(1.0, 0.0, 1.0) + Vec3::new(0.0, 0.0, 150.0), -0x8000);
    idle(&mut w, 10);
    assert_eq!(keese(&w, h).action, KeeseAction::Perched);
    // 100 across: EnFirefly_SetupAttackFromPerched: pitch 0x1554, facing Link, timer 50,
    // speed 3, the wings at 3 times speed; its body's AT on.
    w.place_player(KEESE_HOME * Vec3::new(1.0, 0.0, 1.0) + Vec3::new(0.0, 0.0, 100.0), -0x8000);
    idle(&mut w, 1);
    let k = keese(&w, h);
    assert_eq!(k.action, KeeseAction::AttackFromPerched);
    assert_eq!((k.timer, k.skel.play_speed), (50, 3.0));
    let at_registered = |w: &PlayState| w.col_chk.col_at.iter().any(|r| r.actor == h);
    assert!(at_registered(&w));
    // 50 frames of it (timer to 0), then EnFirefly_SetupIdle: timer Rand_S16Offset(70, 100),
    // speed 1.5 to 3.
    for i in 1..50 {
        idle(&mut w, 1);
        if keese(&w, h).action != KeeseAction::AttackFromPerched {
            panic!("left the dive on frame {i}: {:?}", keese(&w, h).action);
        }
        assert_eq!(keese(&w, h).timer, 50 - i);
    }
    idle(&mut w, 1);
    let k = keese(&w, h);
    assert_eq!(k.action, KeeseAction::Idle);
    assert!((70..170).contains(&k.timer) || k.timer == 69, "timer {}", k.timer);
    assert!(k.actor.speed_xz >= 1.5 && k.actor.speed_xz < 3.0);
}

#[test]
fn a_sword_slash_kills_a_keese_and_it_disappears_dropping_from_table_14() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a);
    let h = nearest::<EnFirefly>(&w, KEESE_HOME);
    let link = KEESE_HOME * Vec3::new(1.0, 0.0, 1.0) + Vec3::new(0.0, 0.0, 100.0);
    w.place_player(link, -0x8000);
    let mut prev = PadState::default();
    tick(&mut w, PadState::default(), &mut prev);
    assert_eq!(keese(&w, h).action, KeeseAction::AttackFromPerched);
    let items_before: Vec<ActorHandle> = w.actors.all().into_iter().filter(|&i| w.actors.downcast::<EnItem00>(i).is_some()).collect();
    // B (drawing the Kokiri Sword, then its slash), the Keese held off over Link's head while
    // he draws, then in front of him (36 ahead, 20 up) until the swing hits it.
    let b_press = with(PadState::default(), BTN_B);
    let mut died = None;
    for i in 0..40 {
        let swinging = w.player().action == oot_actors::player::Action::Attack;
        let hold = w.player().actor.world_pos + if !swinging { Vec3::new(0.0, 60.0, -80.0) } else { Vec3::new(0.0, 20.0, -36.0) };
        if let Some(k) = w.actors.actor_mut(h) {
            k.world_pos = hold;
            k.prev_pos = hold;
        }
        tick(&mut w, if i == 0 { b_press } else { PadState::default() }, &mut prev);
        if keese(&w, h).action == KeeseAction::Die {
            died = Some(w.audio.frames);
            break;
        }
    }
    let f = died.expect("the slash killed it");
    // EnFirefly_CheckCollide: Actor_SetDropFlag (a sword: dropFlag 0), the Kokiri Sword's
    // DMG_ENTRY(1, NONE): Actor_ApplyDamage to 0, Enemy_StartFinishingBlow, not targetable;
    // EnFirefly_SetupDie: timer 40, NA_SE_EN_FFLY_DEAD, the red flash 40; and EnFirefly_Update
    // runs the new EnFirefly_Die the same frame (after EnFirefly_CheckCollide): timer 39.
    let k = keese(&w, h);
    assert_eq!(k.actor.col_chk_info.health, 0);
    assert_eq!(k.actor.drop_flag, 0);
    assert_eq!(k.actor.flags & oot_game::actor::ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!(k.timer, 39);
    assert_eq!(k.actor.color_filter_timer, 40);
    assert!(sfx_on(&w, f, NA_SE_EN_FFLY_DEAD));
    // EnFirefly_Die until it lands or 40 frames pass, then EnFirefly_SetupDisappear: 15 frames
    // shrinking (by 0.00034), then Item_DropCollectibleRandom(table 14) and Actor_Kill.
    let mut n = 0;
    while keese(&w, h).action == KeeseAction::Die {
        idle(&mut w, 1);
        n += 1;
        assert!(n <= 40);
    }
    assert_eq!(keese(&w, h).action, KeeseAction::Disappear);
    assert_eq!(keese(&w, h).timer, 15);
    let mut last = Vec3::ZERO;
    for _ in 0..15 {
        last = keese(&w, h).actor.world_pos;
        idle(&mut w, 1);
    }
    let gone = w.actors.downcast::<EnFirefly>(h).is_none() || w.actors.actor(h).is_some_and(|a| a.killed);
    assert!(gone, "the Keese is gone");
    // What table 14 drops (sItemDropIds[14 * 16 + Rand_ZeroOne() * 16]): at most one, at the
    // Keese, one of the table's (after func_8001F404: a recovery heart, 3, at full health is a
    // green rupee, 0).
    let ids = &a.item_drops.ids[14 * 16..14 * 16 + 16];
    let new_items: Vec<&EnItem00> = w.actors.all().into_iter().filter(|i| !items_before.contains(i)).filter_map(|i| w.actors.downcast::<EnItem00>(i)).collect();
    assert!(new_items.len() <= 1);
    for it in &new_items {
        assert!(it.actor.home_pos.distance(last) < 1.0, "{:?} vs {last:?}", it.actor.home_pos);
        assert!(ids.contains(&(it.actor.params as u8)) || (it.actor.params == 0 && ids.contains(&3)), "{} in {ids:?}", it.actor.params);
    }
    eprintln!("table 14: {ids:?}; dropped {:?}", new_items.iter().map(|i| i.actor.params).collect::<Vec<_>>());
}
