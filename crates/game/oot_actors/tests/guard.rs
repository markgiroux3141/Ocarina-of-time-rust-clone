//! Player's guard (GAME-05 milestone 3a) against the C (`z_player.c`, `z_player_lib.c`):
//! - R raises the shield (`Player_ActionHandler_11`), held in the right hand
//!   (`Player_SetModelsForHoldingShield`), the guard holds (`Player_Action_80843188`) with its
//!   collider registered from the hand (`Player_UpdateShieldCollider`), and letting go lowers it;
//! - a blow on the shield is blocked (`func_808382DC`'s `AC_BOUNCED` path,
//!   `Player_Action_808435C4`): no damage, the recoil, then the guard again;
//! - a Deku Scrub's nut (`En_Nutsball`) bounces off the Deku Shield and flies back as Link's
//!   attack, and without the guard it hurts him and breaks.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_R, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_nutsball::{self, EnNutsball};
use oot_actors::player::{Action, COLLIDER_SHIELD, STATE1_22};
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::sfx::*;
use oot_game::collision_check as cc;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn tick(w: &mut PlayState, p: PadState, prev: &mut PadState) {
    w.tick_with(scripted_input(*prev, p));
    *prev = p;
}

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Inside the Deku Tree with the Kokiri Sword and the Deku Shield (`deku-tree-inside`), the
/// sound log on, Link standing on the ground floor at (-88, 0, -250) facing +z.
fn deku_tree(a: &Arc<GameAssets>) -> Option<PlayState> {
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-inside").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data()?, rules()?, save, audio).expect("Play_Init");
    let mut prev = PadState::default();
    for _ in 0..60 {
        tick(&mut w, PadState::default(), &mut prev);
    }
    w.place_player(Vec3::new(-88.0, 0.0, -250.0), 0);
    for _ in 0..10 {
        tick(&mut w, PadState::default(), &mut prev);
    }
    Some(w)
}

fn sfx_on(w: &PlayState, frame: u32, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == frame && s == id)
}

fn shield_registered(w: &PlayState) -> bool {
    let h = w.player.unwrap();
    w.col_chk.col_ac.iter().any(|r| r.actor == h && r.id == COLLIDER_SHIELD) && w.col_chk.col_at.iter().any(|r| r.actor == h && r.id == COLLIDER_SHIELD)
}

#[test]
fn the_guard_comes_up_holds_and_comes_down() {
    let Some(a) = assets() else { return };
    let Some(mut w) = deku_tree(&a) else { return };
    let d = w.data.clone();
    let mut prev = PadState::default();
    let r = with(PadState::default(), BTN_R);
    assert!(!shield_registered(&w));
    assert!(!w.player().holding_shield);

    // Player_ActionHandler_11 (from the standing action's handler list): no lock-on, a shield
    // worn. Player_SetupAction(Player_Action_80843188), PLAYER_STATE1_SHIELDING,
    // Player_SetModelsForHoldingShield (the right hand RH_SHIELD, modelAnimType
    // PLAYER_ANIMTYPE_2, itemAction -1), then the defense animation (GET_PLAYER_ANIM(defense,
    // PLAYER_ANIMTYPE_2) = link_normal_defense) changed to at its last frame, and
    // NA_SE_IT_SHIELD_POSTURE.
    tick(&mut w, r, &mut prev);
    let f0 = w.audio.frames;
    let p = w.player();
    let defense = d.anim("link_normal_defense");
    assert_eq!(p.action, Action::Guard);
    assert_ne!(p.state1 & STATE1_22, 0);
    assert!(p.holding_shield);
    assert_eq!((p.model_anim_type, p.item_ap), (2, -1));
    assert_eq!(p.skel.animation, defense);
    assert_eq!(p.skel.cur_frame, d.anims[defense].last_frame());
    assert!(sfx_on(&w, f0, NA_SE_IT_SHIELD_POSTURE));
    // Player_PostLimbDrawGameplay, PLAYER_LIMB_R_HAND (rightHandType RH_SHIELD): shieldMf is the
    // hand's matrix, and Player_UpdateShieldCollider (shielding) puts the quad there, the Deku
    // Shield's COL_MATERIAL_WOOD, set for AC and AT.
    assert!(shield_registered(&w));
    let p = w.player();
    assert_eq!(p.shield_quad.base.col_type, cc::COL_MATERIAL_WOOD);
    let v = [Vec3::new(-4500.0, -3000.0, -600.0), Vec3::new(1500.0, -3000.0, -600.0), Vec3::new(-4500.0, 3000.0, -600.0), Vec3::new(1500.0, 3000.0, -600.0)];
    let want = v.map(|v| p.shield_mf.transform_point3(v));
    let got = p.shield_quad.dim.quad;
    for i in 0..4 {
        assert!(got[i].distance(want[i]) < 1e-3, "vertex {i}: {:?} vs {:?}", got[i], want[i]);
    }
    // Its centre is (-1500, 0, -600) in the hand's space, 0.01 to the unit: 16.16 from the
    // hand (bodyPartsPos[PLAYER_BODYPART_R_HAND], the limb's origin), in front of Link (+z).
    let c = (want[0] + want[3]) * 0.5;
    let hand = p.body_parts_pos[oot_actors::player::BODYPART_R_HAND];
    assert!((c.distance(hand) - Vec3::new(1500.0, 0.0, 600.0).length() * 0.01).abs() < 0.01, "the shield's centre {c:?}, the hand {hand:?}");
    assert!(c.z > p.actor.world_pos.z);

    // Player_Action_80843188's next frame: LinkAnimation_Update ends the one-frame animation:
    // the defense's wait loops (link_normal_defense_wait), actionVar2 1.
    tick(&mut w, r, &mut prev);
    let p = w.player();
    assert_eq!(p.skel.animation, d.anim("link_normal_defense_wait"));
    assert_eq!(p.action_var2, 1);
    // Held: the guard stays, the shield up and registered every frame.
    for _ in 0..20 {
        tick(&mut w, r, &mut prev);
        let p = w.player();
        assert_eq!(p.action, Action::Guard);
        assert!(p.holding_shield && p.state1 & STATE1_22 != 0);
        assert!(shield_registered(&w));
    }

    // R let go: Player_ActionHandler_11 not taken, so PLAYER_STATE1_SHIELDING off, func_8008EC70
    // (itemAction < 0: the held item's models back: nothing in hand, so
    // PLAYER_MODELGROUP_DEFAULT and PLAYER_ANIMTYPE_0), func_8083A098 with
    // GET_PLAYER_ANIM(defense_end, PLAYER_ANIMTYPE_0) = link_normal_defense_end_free,
    // NA_SE_IT_SHIELD_REMOVE.
    tick(&mut w, PadState::default(), &mut prev);
    let f1 = w.audio.frames;
    let p = w.player();
    assert_eq!(p.state1 & STATE1_22, 0);
    assert!(!p.holding_shield);
    assert_eq!(p.item_ap, p.held_item_ap);
    assert_eq!(p.model_anim_type, 0);
    assert_eq!(d.anim_name(p.skel.animation), "link_normal_defense_end_free");
    assert_ne!(p.action, Action::Guard);
    assert!(sfx_on(&w, f1, NA_SE_IT_SHIELD_REMOVE));
    assert!(!shield_registered(&w));
}

/// Child Link on the course at (0, 0, 0) facing -z, with the Deku Shield.
fn child_on_course() -> Option<PlayState> {
    let d = data()?;
    let c = oot_game::course::build();
    Some(new_world(d, eng_collision::bgcheck::CollisionContext::new(c.collision), false, Vec3::ZERO, -0x8000))
}

#[test]
fn the_shield_blocks_a_hit() {
    let Some(mut w) = child_on_course() else { return };
    let d = w.data.clone();
    let mut prev = PadState::default();
    let r = with(PadState::default(), BTN_R);
    assert_eq!(w.player().current_shield, 1);
    for _ in 0..10 {
        tick(&mut w, r, &mut prev);
    }
    assert_eq!(w.player().action, Action::Guard);
    let health = w.save.health;
    // The sandbox's hurting dummy 25 ahead: its body (AT_TYPE_ENEMY, damage 8) reaches the
    // shield (AC_HARD: CollisionCheck_SetATvsAC bounces it, AC_BOUNCED on the shield).
    let h = w.spawn_hurting_target(Vec3::new(0.0, 0.0, -25.0), cc::HIT_SPECIAL_EFFECT_NONE);
    let mut blocked = None;
    for i in 0..5 {
        tick(&mut w, r, &mut prev);
        if w.player().action == Action::GuardHit {
            blocked = Some(i);
            break;
        }
    }
    assert!(blocked.is_some(), "the shield took the dummy's blow");
    kill(&mut w, h);
    // func_808382DC, the shield's AC_BOUNCED: no damage (it returns before the body's hit);
    // in the guard (sp54), Player_SetupAction(Player_Action_808435C4), actionVar1 = sp54 = 1,
    // the body's recoil D_808543C4[two-handed 0] = link_normal_defense_hit, and speedXZ -18
    // facing shape.rot.y; then Player_Action_808435C4's Player_DecelerateToZero steps it by
    // R_DECELERATE_RATE / 100.
    let p = w.player();
    assert_eq!(w.save.health, health);
    assert_eq!(p.invincibility_timer, 0);
    assert_eq!(p.action_var1, 1);
    assert_eq!(p.skel.animation, d.anim("link_normal_defense_hit"));
    assert_eq!(p.current_yaw, p.actor.shape_rot.y);
    let decel = p.regs.reg(43) as f32 / 100.0;
    assert_eq!(p.linear_velocity, -18.0 + decel);
    // The recoil plays out (LinkAnimation_Update), unhurt; then Player_Action_808435C4 sets the
    // guard up again: Player_Action_80843188, shielding, link_normal_defense at its last frame.
    let hit_frames = d.anims[d.anim("link_normal_defense_hit")].last_frame();
    let mut n = 0;
    while w.player().action == Action::GuardHit {
        tick(&mut w, r, &mut prev);
        n += 1;
        assert!(n < 40);
    }
    let p = w.player();
    assert_eq!(p.action, Action::Guard);
    assert_ne!(p.state1 & STATE1_22, 0);
    assert_eq!(p.skel.animation, d.anim("link_normal_defense"));
    assert_eq!(w.save.health, health);
    eprintln!("the recoil: {n} frames (link_normal_defense_hit's last frame {hit_frames})");
}

fn kill(w: &mut PlayState, h: ActorHandle) {
    if let Some(a) = w.actors.actor_mut(h) {
        a.kill();
    }
}

/// A Deku Scrub's nut (`EnNutsballType` 0) 150 ahead of Link flying at him: `object_dekunuts`
/// loaded (Object_SpawnPersistent), then Actor_Spawn with the scrub's yaw.
fn fire_nut(w: &mut PlayState) -> ActorHandle {
    if w.object_ctx.get_index(en_nutsball::OBJECT_IDS[0]).is_none() {
        w.object_ctx.spawn(en_nutsball::OBJECT_IDS[0]);
    }
    let p = w.player().actor.world_pos;
    w.actor_spawn(en_nutsball::ACTOR_EN_NUTSBALL, p + Vec3::new(0.0, 20.0, 150.0), [0, -0x8000, 0], 0).expect("En_Nutsball")
}

#[test]
fn a_deku_nut_bounces_back_off_the_deku_shield() {
    let Some(a) = assets() else { return };
    let Some(mut w) = deku_tree(&a) else { return };
    let mut prev = PadState::default();
    let r = with(PadState::default(), BTN_R);
    for _ in 0..5 {
        tick(&mut w, r, &mut prev);
    }
    let health = w.save.health;
    let h = fire_nut(&mut w);
    let mut bounced = None;
    for i in 0..30 {
        // shieldMf as the nut reads it: from the last draw, before this frame's update.
        let shield_yaw = oot_game::sys_matrix::MtxF::from_mat4(w.player().shield_mf).to_yxz_rot_s(false)[1];
        tick(&mut w, r, &mut prev);
        let Some(n) = w.actors.downcast::<EnNutsball>(h) else { panic!("the nut broke on frame {i}") };
        if n.collider.base.at_flags & cc::AT_TYPE_PLAYER != 0 {
            // EnNutsball_Projectile: AT_HIT, AT_TYPE_ENEMY and AT_BOUNCED with the Deku Shield:
            // the attack is Link's now (AT_TYPE_PLAYER, DMG_DEKU_STICK = 1 << 1), flying the
            // way the shield faces (Matrix_MtxFToYXZRotS(&shieldMf).y + 0x8000), its timer 30
            // again.
            let n = w.actors.downcast::<EnNutsball>(h).unwrap();
            assert_eq!(n.collider.base.at_flags & (cc::AT_TYPE_ENEMY | cc::AT_BOUNCED | cc::AT_HIT), 0);
            assert_eq!(n.collider.info.at_dmg_info.dmg_flags, 1 << 1);
            assert_eq!(n.timer, 30);
            assert_eq!(n.actor.world_rot.y, shield_yaw.wrapping_add(-0x8000i16));
            bounced = Some(i);
            break;
        }
        assert_eq!(n.actor.world_rot.y as u16, 0x8000, "flying at Link");
    }
    let i = bounced.expect("the nut bounced off the shield");
    // Link blocked it (func_808382DC's shield path: the recoil, no damage).
    assert_eq!(w.player().action, Action::GuardHit);
    assert_eq!(w.save.health, health);
    // Back the way it came (yaw near 0, +z), away from Link.
    let z0 = w.actors.downcast::<EnNutsball>(h).unwrap().actor.world_pos.z;
    for _ in 0..3 {
        tick(&mut w, r, &mut prev);
    }
    let z1 = w.actors.downcast::<EnNutsball>(h).expect("still flying").actor.world_pos.z;
    assert!(z1 > z0 + 30.0, "{z0} -> {z1}");
    eprintln!("the nut bounced on frame {i}");
}

#[test]
fn a_deku_nut_hurts_link_without_the_guard_and_breaks() {
    let Some(a) = assets() else { return };
    let Some(mut w) = deku_tree(&a) else { return };
    let mut prev = PadState::default();
    let health = w.save.health;
    let h = fire_nut(&mut w);
    let mut last = Vec3::ZERO;
    for i in 0..30 {
        tick(&mut w, PadState::default(), &mut prev);
        if let Some(n) = w.actors.downcast::<EnNutsball>(h) {
            last = n.actor.world_pos;
        } else {
            // Its AT hit Link's body (damage 8, half a heart) and EnNutsball_Projectile broke it
            // (AT_HIT): Actor_Kill, a burst of 15 fragments (EffectSsHahen_SpawnBurst), and
            // NA_SE_EN_OCTAROCK_ROCK from a fixed source.
            assert_eq!(w.save.health, health - 8, "frame {i}");
            // EffectSsHahen_Init: life 200, counted down by EffectSs_UpdateAll that frame and the
            // next (the killed nut is gone from the actor list a frame after Actor_Kill), so 198;
            // all near where it broke, 4 above it.
            let burst: Vec<Vec3> = w.effect_ss.table.iter().filter(|e| e.life == 198 && e.ty == oot_game::effect::EFFECT_SS_HAHEN).map(|e| e.pos).collect();
            assert_eq!(burst.len(), 15);
            assert!(burst.iter().all(|p| p.distance(last + Vec3::Y * 4.0) < 20.0), "{burst:?} from {last:?}");
            assert!(w.sfx_sources.iter().any(|s| s.countdown != 0));
            return;
        }
    }
    panic!("the nut never broke");
}

#[test]
fn a_fire_blow_on_the_deku_shield_burns_it_away() {
    use oot_actors::item_shield::{self, ItemShield};
    let Some(a) = assets() else { return };
    let Some(mut w) = deku_tree(&a) else { return };
    let mut prev = PadState::default();
    let r = with(PadState::default(), BTN_R);
    for _ in 0..5 {
        tick(&mut w, r, &mut prev);
    }
    let health = w.save.health;
    let link = w.player().actor.world_pos;
    // The hurting dummy with fire, in front of the shield (Link faces +z).
    let h = w.spawn_hurting_target(link + Vec3::new(0.0, 0.0, 25.0), cc::HIT_SPECIAL_EFFECT_FIRE);
    let mut shield_mf = w.player().shield_mf;
    let mut found = None;
    for i in 0..5 {
        shield_mf = w.player().shield_mf;
        tick(&mut w, r, &mut prev);
        if let Some(s) = w.actors.all().into_iter().find(|&s| w.actors.downcast::<ItemShield>(s).is_some()) {
            found = Some((i, s));
            break;
        }
    }
    let (i, s) = found.expect("Item_Shield");
    kill(&mut w, h);
    // func_808382DC's shield path: blocked (no damage), and the blow's
    // HIT_SPECIAL_EFFECT_FIRE burns the Deku Shield (func_8083819C): off and out of the
    // inventory, Actor_Spawn(ACTOR_ITEM_SHIELD, params 1).
    assert_eq!(w.save.health, health, "frame {i}");
    assert_eq!(w.player().current_shield, 0);
    let sh = w.actors.downcast::<ItemShield>(s).unwrap();
    assert_eq!(sh.actor.params, 1);
    assert_eq!(sh.actor.id, item_shield::ACTOR_ITEM_SHIELD);
    // Its first update (func_80B86F68): at shieldMf's translation, facing as the shield did
    // (Math_Atan2S(-zz, -xz), Math_Atan2S(-yz, sqrt(zz² + xz²))), then hopping (velocity.y 4,
    // gravity -0.8, minVelocityY -4) with its timer 70, shown; this frame's
    // func_80B86CA8 hasn't run yet.
    let mf = oot_game::sys_matrix::MtxF::from_mat4(shield_mf);
    // (Item_Shield is ACTORCAT_ITEMACTION, after Player's category: it updates the frame it's
    // spawned.)
    assert_eq!((sh.action, sh.timer, sh.actor.velocity.y, sh.actor.gravity, sh.actor.min_velocity_y), (item_shield::Action::Burning, 70, 4.0, -0.8, -4.0));
    assert_eq!(sh.actor.world_pos, Vec3::new(mf.xw, mf.yw, mf.zw));
    assert_eq!(sh.actor.shape_rot.y, eng_math::atan2_s(-mf.zz, -mf.xz));
    assert_eq!(sh.actor.shape_rot.x, eng_math::atan2_s(-mf.yz, (mf.zz * mf.zz + mf.xz * mf.xz).sqrt()));
    assert_eq!(sh.unk_19c & 2, 0);
    // Burning: each frame, eight flames (EffectSsFireTail_SpawnFlame, the shield their actor);
    // once on the ground the timer runs down from 70, the scale (timer - 8) * 0.000625 from
    // timer 23 to 8, and at 0 Actor_Kill.
    let mut frames = 0;
    let mut grounded_at = None;
    loop {
        tick(&mut w, PadState::default(), &mut prev);
        frames += 1;
        assert!(frames < 200, "the shield never burnt away");
        let Some(sh) = w.actors.downcast::<ItemShield>(s) else { break };
        assert_eq!(sh.action, item_shield::Action::Burning);
        // (Each lives one frame: EffectSsFireTail_SpawnFlame's life 1, 0 after EffectSs_UpdateAll.)
        let flames = w.effect_ss.table.iter().filter(|e| e.life == 0 && e.ty == oot_game::effect::EFFECT_SS_FIRE_TAIL && e.actor == Some(s)).count();
        assert_eq!(flames, 8, "flames on frame {frames}");
        if grounded_at.is_none() && sh.actor.bg_check_flags & oot_game::actor::BGCHECKFLAG_GROUND != 0 {
            grounded_at = Some((frames, sh.timer));
        }
        if (8..24).contains(&(sh.timer + 1)) && sh.actor.bg_check_flags & oot_game::actor::BGCHECKFLAG_GROUND != 0 {
            assert_eq!(sh.actor.scale.x, (sh.timer + 1 - 8) as f32 * 0.000625, "timer {}", sh.timer);
        }
    }
    let (g, t) = grounded_at.expect("it landed");
    // From the frame it landed (its timer counted down there to t), t more frames to 0, then
    // one more for the Actor_Kill at 0, and the frame it's gone from the list.
    eprintln!("landed on frame {g} (timer {t}), gone on frame {frames}");
    assert_eq!(frames, g + t as i32 + 2);
}
