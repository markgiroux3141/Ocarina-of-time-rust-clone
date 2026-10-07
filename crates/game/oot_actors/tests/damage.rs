//! Damage, death and the game over (GAME-05 milestone 2) against the C:
//! - each hit response of `func_80837C0C` (`z_player.c`) through the sandbox's hurting dummy:
//!   the stagger, the knockdown, frozen (`Player_Action_8084FB10`), electrified
//!   (`Player_Action_8084FBF4`), the swimming hit (`Player_Action_8084E30C`), and burning
//!   (`func_8083821C`, `Player_UpdateBodyBurn`, the Deku Shield's `func_8083819C`), with their timers;
//! - the hit flash (`Player_Draw`'s `Gfx_SetFog2`);
//! - death (`func_80836448`, `Player_Action_80843CEC`, `func_80843AE8`), `GameOver_Update`
//!   (`z_game_over.c`) and the game over menu's stand-in (`KaleidoScope_Update`'s game over
//!   states) frame by frame to "Continue" and the respawn; and the revival by a bottled fairy;
//! - `En_Dekubaba`'s damage tables (`z_en_dekubaba.c`);
//! - the exit: the dummy hits Link, and a Deku Baba bites him (`Route::DekuBaba`, the
//!   `deku-baba` script's run, also a golden trace).
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_dekubaba::{self, Action as BabaAction, EnDekubaba};
use oot_actors::player::{Action, STATE1_26, STATE1_27, STATE1_7};
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::actor_ctx::ActorHandle;
use oot_game::camera::{CAM_ID_MAIN, CAM_SET_TURN_AROUND, CAM_STAT_ACTIVE, CAM_STAT_UNK3};
use oot_game::collision_check::*;
use oot_game::game_over::*;
use oot_game::kaleido::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn tick(w: &mut PlayState, p: PadState, prev: &mut PadState) {
    w.tick_with(scripted_input(*prev, p));
    *prev = p;
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// Child Link standing on the course at (0, 0, 0) facing -z, a hurting dummy with `effect` 20
/// in front of him: returns the play state on the frame its touch hit him (his
/// `invincibilityTimer` 20), the dummy killed then so it hits once, and his health before.
fn hit_by(effect: u8) -> Option<(PlayState, i16)> {
    let mut w = child_on_course(Vec3::ZERO, -0x8000)?;
    idle(&mut w, 5);
    let health = w.save.health;
    let h = w.spawn_hurting_target(Vec3::new(0.0, 0.0, -20.0), effect);
    for _ in 0..5 {
        idle(&mut w, 1);
        if w.player().invincibility_timer != 0 {
            kill(&mut w, h);
            return Some((w, health));
        }
    }
    panic!("the dummy didn't hit Link");
}

fn child_on_course(pos: Vec3, yaw: i16) -> Option<PlayState> {
    let d = data()?;
    let c = oot_game::course::build();
    Some(new_world(d, eng_collision::bgcheck::CollisionContext::new(c.collision), false, pos, yaw))
}

fn kill(w: &mut PlayState, h: ActorHandle) {
    if let Some(a) = w.actors.actor_mut(h) {
        a.kill();
    }
}

#[test]
fn a_plain_hit_staggers_link_back() {
    let Some((w, health)) = hit_by(HIT_SPECIAL_EFFECT_NONE) else { return };
    let p = w.player();
    // func_80837C0C, PLAYER_HIT_RESPONSE_NONE on the ground: the dummy's 8 off
    // (func_80837B18), 20 frames of invincibility (Player_SetIntangibility), and the
    // stagger (Player_Action_8084370C): damage 8 >= 5, so speedXZ 23 and D_808544B0[4 + ...];
    // PLAYER_STATE1_26. Player_UpdateCommon runs the new action the same frame, whose
    // Player_DecelerateToZero takes R_DECELERATE_RATE (REG(43)) / 100 off.
    assert_eq!(w.save.health, health - 8);
    assert_eq!(p.invincibility_timer, 20);
    assert_eq!(p.action, Action::Damaged);
    assert_eq!(p.linear_velocity, 23.0 - p.regs.reg(43) as f32 / 100.0);
    assert!(p.state1 & STATE1_26 != 0);
    // Facing the dummy (yaw -z, hit from in front: |yRot| > 0x4000 after the turn), not locked
    // on: D_808544B0[4], gPlayerAnim_link_normal_front_hit.
    assert_eq!(w.data.anim_name(p.skel.animation), "link_normal_front_hit");
    assert!(!p.body_is_burning && p.body_shock_timer == 0);
}

#[test]
fn the_invincibility_counts_down_and_the_flash_follows_it() {
    let Some((mut w, _)) = hit_by(HIT_SPECIAL_EFFECT_NONE) else { return };
    // Player_Draw (each frame while invincibilityTimer > 0): damageFlickerAnimCounter +=
    // CLAMP(50 - invincibilityTimer, 8, 40), and the red fog's far plane is
    // 4000 - (s32)(Math_CosS(damageFlickerAnimCounter * 256) * 2000). The counter was reset by
    // Player_SetIntangibility and stepped by the hit frame's draw (timer 20: by 30).
    let mut counter: u8 = 30;
    assert_eq!(w.player().damage_flicker_anim_counter, counter);
    for k in 1..=20 {
        idle(&mut w, 1);
        let p = w.player();
        // The timer counts down at the top of each update.
        let timer = 20 - k;
        assert_eq!(p.invincibility_timer, timer);
        if timer > 0 {
            counter = counter.wrapping_add((50 - timer as i32).clamp(8, 40) as u8);
            assert_eq!(p.damage_flicker_anim_counter, counter);
            let far = 4000 - (eng_math::cos_s((counter as i32 * 256) as i16) * 2000.0) as i32;
            assert_eq!(p.damage_flash_far, Some(far), "frame {k}");
        } else {
            assert_eq!(p.damage_flash_far, None);
        }
    }
}

#[test]
fn a_knockback_hit_knocks_link_down() {
    let Some((w, health)) = hit_by(HIT_SPECIAL_EFFECT_KNOCKBACK) else { return };
    let p = w.player();
    // HIT_SPECIAL_EFFECT_KNOCKBACK → PLAYER_HIT_RESPONSE_KNOCKBACK_LARGE at func_808382DC's
    // 4.0, 5.0: Player_Action_8084377C, speed 4, velocity.y 5, off the ground, PLAYER_STATE3_1.
    assert_eq!(p.action, Action::KnockedDown);
    assert_eq!((p.linear_velocity, p.actor.velocity.y), (4.0, 5.0));
    assert!(!p.grounded());
    // From in front (|yRot| > 0x4000): gPlayerAnim_link_normal_front_downA.
    assert_eq!(w.data.anim_name(p.skel.animation), "link_normal_front_downA");
    assert_eq!(w.save.health, health - 8);
    // Player_UpdateCamAndSeqModes: knocked down (Player_Action_8084377C), the main camera is asked
    // for CAM_MODE_STILL (BACKLOG #18), which the course's setting has.
    assert_eq!(p.update_cam_and_seq_modes(), Some((oot_game::camera::CAM_MODE_STILL, None)));
    assert_eq!(w.game_camera.mode, oot_game::camera::CAM_MODE_STILL);
}

#[test]
fn an_ice_hit_freezes_link_until_he_struggles_free() {
    let Some((mut w, _)) = hit_by(HIT_SPECIAL_EFFECT_ICE) else { return };
    // PLAYER_HIT_RESPONSE_FROZEN: Player_Action_8084FB10, gPlayerAnim_link_normal_ice_down,
    // speed 0 (func_80832224).
    assert_eq!(w.player().action, Action::Frozen);
    assert_eq!(w.data.anim_name(w.player().skel.animation), "link_normal_ice_down");
    assert_eq!(w.player().linear_velocity, 0.0);
    // Each frame: actionVar1 up to 6; func_80832594(1, 100): actionVar2 += 1 + (stick spin:
    // none here) + 5 with A pressed; past 100 the ice breaks (actionVar1 -1). A quarter heart off
    // when gameplayFrames % 4 == 0 (Player_InflictDamage: func_80837B18 takes nothing while the
    // hit's invincibility lasts). With A every other frame: +6, +1, ... 7 every two frames. The
    // hit frame's run of the action already counted 1.
    let mut prev = PadState::default();
    let mut var2 = w.player().action_var2 as i32;
    let mut health = w.save.health;
    let mut broke_at = None;
    for k in 0..60 {
        let pad = if k % 2 == 0 { with(stick(0, 0), BTN_A) } else { stick(0, 0) };
        tick(&mut w, pad, &mut prev);
        let p = w.player();
        if broke_at.is_none() {
            var2 += 1 + if pad.button & BTN_A != 0 { 5 } else { 0 };
            if w.gameplay_frames % 4 == 0 && p.invincibility_timer == 0 {
                health -= 1;
            }
            assert_eq!(w.save.health, health, "frame {k}");
            if var2 > 100 {
                broke_at = Some(k);
                assert_eq!(p.action_var1, -1);
                // (STATE2_14, which keeps the colliders unregistered, is set while frozen.)
            } else {
                assert_eq!(p.action_var2 as i32, var2, "frame {k}");
                assert_eq!(p.action_var1, (k as i8 + 2).min(6));
                assert!(p.state2 & oot_actors::player::STATE2_14 != 0);
            }
        } else if p.action != Action::Frozen {
            // Then the ice-down animation plays out and Link stands (func_80839F90),
            // invulnerable for 20 frames (Player_SetInvulnerability(-20)).
            assert_eq!(p.action, Action::StandingStill);
            assert_eq!(p.invincibility_timer, -20);
            return;
        }
    }
    panic!("Link didn't stand after the ice (broke at {broke_at:?})");
}

#[test]
fn an_electric_hit_shocks_link_for_twenty_frames() {
    let Some((mut w, _)) = hit_by(HIT_SPECIAL_EFFECT_ELECTRIC) else { return };
    // PLAYER_HIT_RESPONSE_ELECTRIFIED: Player_Action_8084FBF4 with actionVar2 20, which runs the
    // same frame (19). Each frame: actionVar2 % 25 != 0, so DECR it (func_80837B18's quarter
    // heart only at a multiple of 25: never from 20 down); at 0, func_80839F90. bodyShockTimer
    // 40 every frame of it.
    assert_eq!(w.player().action, Action::Electrified);
    assert_eq!(w.player().action_var2, 19);
    let health = w.save.health;
    for k in 1..=19 {
        idle(&mut w, 1);
        let p = w.player();
        assert_eq!(w.save.health, health, "frame {k}");
        if k < 19 {
            assert_eq!((p.action, p.action_var2), (Action::Electrified, 19 - k as i16), "frame {k}");
            // Player_UpdateBodyShock took one off this frame's 40 (the action sets 40 after).
            assert_eq!(p.body_shock_timer, 40);
        } else {
            assert_eq!(p.action, Action::StandingStill, "frame {k}");
        }
    }
    // Then Player_UpdateBodyShock counts the shock down, one a frame, its sparks going.
    for k in 1..=40 {
        idle(&mut w, 1);
        assert_eq!(w.player().body_shock_timer, 40 - k as u8);
    }
}

#[test]
fn a_fire_hit_sets_link_burning_and_the_flames_burn_down() {
    let Some((mut w, _)) = hit_by(HIT_SPECIAL_EFFECT_FIRE) else { return };
    let p = w.player();
    // func_80838280 with HIT_SPECIAL_EFFECT_FIRE: func_8083821C, each body part's flame
    // Rand_S16Offset(0, 200), bodyIsBurning; the stagger as for a plain hit.
    assert!(p.body_is_burning);
    assert_eq!(p.action, Action::Damaged);
    assert!(p.body_flame_timers.iter().all(|&t| t < 200) && p.body_flame_timers.iter().any(|&t| t > 0));
    // The child's Kokiri Shield is the Deku Shield (the course's map select save): it burns
    // away at the first Player_UpdateBodyBurn (func_8083819C), off and out of the inventory.
    let mut timers = p.body_flame_timers;
    let mut health = w.save.health;
    // The speed the next frame's Player_UpdateBodyBurn reads: the stagger's after this frame.
    let mut speed = p.linear_velocity;
    idle(&mut w, 1);
    assert_eq!(w.player().current_shield, 0);
    assert_eq!(w.save.cur_equip_value(oot_game::item::EQUIP_TYPE_SHIELD), 0);
    // Then each frame: every flame down by (s32)(speedXZ * 0.4) + 1 (the Kokiri Tunic, no
    // PLAYER_STATE2_3), to 0 once at or under it; a quarter heart off when (7 &
    // gameplayFrames) == 0 while any burns; bodyIsBurning off once none does.
    let step = |t: &mut [u8; 18], speed: f32| -> bool {
        let s = (speed * 0.4) as i32 + 1;
        let mut any = false;
        for v in t.iter_mut() {
            if *v as i32 <= s {
                *v = 0;
            } else {
                any = true;
                *v = (*v as i32 - s) as u8;
            }
        }
        any
    };
    let burning = step(&mut timers, speed);
    if burning && w.gameplay_frames & 7 == 0 && w.player().invincibility_timer == 0 {
        health -= 1;
    }
    assert_eq!(w.player().body_flame_timers, timers);
    assert_eq!(w.save.health, health);
    for k in 0..300 {
        speed = w.player().linear_velocity;
        idle(&mut w, 1);
        let any = step(&mut timers, speed);
        if any && w.gameplay_frames & 7 == 0 && w.player().invincibility_timer == 0 {
            health -= 1;
        }
        assert_eq!(w.player().body_flame_timers, timers, "frame {k}");
        assert_eq!(w.save.health, health, "frame {k}");
        if !any {
            assert!(!w.player().body_is_burning);
            return;
        }
    }
    panic!("the flames didn't burn out");
}

#[test]
fn a_hit_while_swimming_is_the_swimming_hit() {
    // The course's channel: Link walks off its bank and treads water (as water.rs).
    let Some(mut w) = (|| {
        let d = data()?;
        let c = oot_game::course::build();
        Some(new_world(d, eng_collision::bgcheck::CollisionContext::new(c.collision), true, Vec3::new(-880.0, 0.0, 800.0), 0x4000))
    })() else {
        return;
    };
    idle(&mut w, 120);
    assert!(w.player().state1 & STATE1_27 != 0 && w.player().action == Action::Swim);
    let pos = w.player().actor.world_pos;
    let h = w.spawn_hurting_target(pos + Vec3::new(0.0, -30.0, 10.0), HIT_SPECIAL_EFFECT_ICE);
    for _ in 0..5 {
        idle(&mut w, 1);
        if w.player().invincibility_timer != 0 {
            break;
        }
    }
    kill(&mut w, h);
    let p = w.player();
    // In water func_808382DC answers every body hit with PLAYER_HIT_RESPONSE_NONE (no freezing),
    // and func_80837C0C with PLAYER_STATE1_27: Player_Action_8084E30C, speedXZ 4, velocity.y 0,
    // gPlayerAnim_link_swimer_swim_hit. The action runs the same frame: func_8084B000's
    // buoyancy below unk_28 adds 0 + 0.1.
    assert_eq!(p.action, Action::SwimDamaged);
    assert_eq!(p.actor.velocity.y, 0.1);
    assert_eq!(w.data.anim_name(p.skel.animation), "link_swimer_swim_hit");
    // At the animation's end, treading water again (func_80838F18).
    for _ in 0..60 {
        idle(&mut w, 1);
        if w.player().action != Action::SwimDamaged {
            break;
        }
    }
    assert_eq!(w.player().action, Action::Swim);
}

/// Inside the Deku Tree on `preset`, Link standing on the top floor away from the Deku Babas.
fn deku_tree(a: &Arc<GameAssets>, preset: &str) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset(preset).unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    // The entrance's walk in (PLAYER_STATE1_29: Player_InBlockingCsMode, no damage) over, Link
    // standing on the entrance's floor: no enemy near.
    idle(&mut w, 60);
    assert_eq!(w.player().action, Action::StandingStill);
    w
}

/// Hits Link with a dummy at a quarter heart left; returns the frame (gameplayFrames) his
/// health reached 0.
fn hit_to_zero(w: &mut PlayState) -> u32 {
    w.save.health = 4;
    let pos = w.player().actor.world_pos;
    let yaw = w.player().actor.shape_rot.y;
    let ahead = pos + Vec3::new(eng_math::sin_s(yaw), 0.0, eng_math::cos_s(yaw)) * 20.0;
    let h = w.spawn_hurting_target(ahead, HIT_SPECIAL_EFFECT_NONE);
    for _ in 0..5 {
        idle(w, 1);
        if w.save.health == 0 {
            kill(w, h);
            return w.gameplay_frames;
        }
    }
    panic!("Link wasn't hit");
}

#[test]
fn death_runs_the_game_over_to_continue_and_the_respawn() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, "deku-tree-inside");
    let scene_changes = w.scene_changes;
    let deaths = w.save.deaths;
    hit_to_zero(&mut w);
    // The hit frame: func_80837B18 false, so no reaction (PLAYER_STATE2_7 off; on the ground,
    // nothing else); no game over yet.
    assert_eq!(w.game_over_ctx.state, GAMEOVER_INACTIVE);
    assert_ne!(w.player().action, Action::Dying);
    // The next frame Player_UpdateCommon sees health 0 on the ground: func_80836448 with
    // gPlayerAnim_link_derth_rebirth (its end frame set to 84), PLAYER_STATE1_DEAD; no fairy:
    // GAMEOVER_DEATH_START, which GameOver_Update (after the actors) turns into
    // GAMEOVER_DEATH_WAIT_GROUND with sGameOverTimer 20; the music and ambience off.
    idle(&mut w, 1);
    let p = w.player();
    assert_eq!(p.action, Action::Dying);
    assert!(p.state1 & STATE1_7 != 0);
    assert_eq!(w.data.anim_name(p.skel.animation), "link_derth_rebirth");
    assert_eq!(p.skel.end_frame, 84.0);
    assert_eq!((w.game_over_ctx.state, w.game_over_timer), (GAMEOVER_DEATH_WAIT_GROUND, 20));
    assert_eq!(w.save.seq_id, oot_game::audio::NA_BGM_DISABLED as u8);
    // Enemies and the rest freeze while Link is dead (sCategoryFreezeMasks' PLAYER_STATE1_DEAD).
    // Waiting for the ground: until the animation ends (curFrame 84), when func_80843AE8 moves
    // the game over to GAMEOVER_DEATH_DELAY_MENU, and GameOver_Update the same frame counts the
    // timer to 19.
    let mut frames = 0;
    while w.game_over_ctx.state == GAMEOVER_DEATH_WAIT_GROUND {
        idle(&mut w, 1);
        frames += 1;
        assert!(frames < 200, "the death's animation didn't end");
    }
    assert_eq!(w.player().skel.cur_frame, 84.0);
    assert_eq!((w.game_over_ctx.state, w.game_over_timer), (GAMEOVER_DEATH_DELAY_MENU, 19));
    // Then the frames each pause state is left in at the frame's end, from the C:
    // - 18 more frames of the delay (timer 18 to 1); the 19th, at 0, opens the menu
    //   (PAUSE_STATE_GAME_OVER_START, GAMEOVER_DEATH_MENU);
    // - KaleidoScopeCall_Update: START → WAIT_BG_PRERENDER (the prerender's SETUP); Play_Draw:
    //   SETUP → PROCESS, then PROCESS → READY: 2 frames in WAIT_BG_PRERENDER; READY → INIT;
    // - INIT → SHOW_MESSAGE (D_8082B260 30); 30 frames of it, the last ending in WINDOW_DELAY
    //   (D_8082B260 40); 40 of that, the last in SHOW_WINDOW; promptPitch -434 down by
    //   160 / R_PAUSE_UI_ANIMS_DURATION (WREG(6) 8) = 20 a frame, past -628 on its 10th frame:
    //   SAVE_PROMPT, deaths + 1. (A state's count includes the frame that entered it.)
    let mut runs: Vec<(u16, usize)> = Vec::new();
    while w.pause_ctx.state != PAUSE_STATE_GAME_OVER_SAVE_PROMPT {
        idle(&mut w, 1);
        let s = w.pause_ctx.state;
        match runs.last_mut() {
            Some((x, n)) if *x == s => *n += 1,
            _ => runs.push((s, 1)),
        }
        assert!(runs.iter().map(|r| r.1).sum::<usize>() < 300);
    }
    assert_eq!(
        runs,
        vec![
            (PAUSE_STATE_OFF, 18),
            (PAUSE_STATE_GAME_OVER_START, 1),
            (PAUSE_STATE_GAME_OVER_WAIT_BG_PRERENDER, 2),
            (PAUSE_STATE_GAME_OVER_INIT, 1),
            (PAUSE_STATE_GAME_OVER_SHOW_MESSAGE, 30),
            (PAUSE_STATE_GAME_OVER_WINDOW_DELAY, 40),
            (PAUSE_STATE_GAME_OVER_SHOW_WINDOW, 10),
            (PAUSE_STATE_GAME_OVER_SAVE_PROMPT, 1),
        ]
    );
    assert_eq!(w.game_over_ctx.state, GAMEOVER_DEATH_MENU);
    assert_eq!(w.save.deaths, deaths + 1);
    assert_eq!(w.pause_ctx.prompt_pitch, -628.0);
    // While paused, nothing updates: Link stays as he fell.
    let link = w.player().actor.world_pos;
    idle(&mut w, 5);
    assert_eq!(w.player().actor.world_pos, link);
    // "Save?": the stick right moves to No (KaleidoScope_UpdatePrompt, stick x >= 30), A takes it:
    // CONTINUE_PROMPT at once, gameOverCtx.state + 1.
    let mut prev = PadState::default();
    tick(&mut w, stick(60, 0), &mut prev);
    assert_eq!(w.pause_ctx.prompt_choice, 4);
    tick(&mut w, with(stick(0, 0), BTN_A), &mut prev);
    assert_eq!(w.pause_ctx.state, PAUSE_STATE_GAME_OVER_CONTINUE_PROMPT);
    assert_eq!(w.pause_ctx.prompt_choice, 0);
    assert_eq!(w.game_over_ctx.state, GAMEOVER_DEATH_MENU + 1);
    // "Continue?": A on Yes: FINISH; the black (interfaceCtx.unk_244) up 10 a frame to 255 (26
    // frames), then the respawn: Play_TriggerRespawn, respawnFlag -2, 3 hearts.
    tick(&mut w, stick(0, 0), &mut prev);
    tick(&mut w, with(stick(0, 0), BTN_A), &mut prev);
    assert_eq!(w.pause_ctx.state, PAUSE_STATE_GAME_OVER_FINISH);
    let mut n = 0;
    while w.pause_ctx.state == PAUSE_STATE_GAME_OVER_FINISH {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 40);
    }
    assert_eq!(n, 26);
    assert_eq!(w.interface_ctx.unk_244, 255);
    assert_eq!((w.save.respawn_flag, w.save.health), (-2, 0x30));
    assert_eq!(w.transition.trigger, oot_game::transition::TRANS_TRIGGER_START);
    // The fade out, then Play_Init at the Deku Tree's entrance: Link alive, the game over and
    // the black gone.
    for _ in 0..200 {
        idle(&mut w, 1);
        if w.scene_changes > scene_changes {
            break;
        }
    }
    assert!(w.scene_changes > scene_changes, "no respawn");
    idle(&mut w, 30);
    let p = w.player();
    assert!(p.state1 & STATE1_7 == 0);
    assert_eq!(w.save.health, 0x30);
    assert_eq!((w.game_over_ctx.state, w.pause_ctx.state, w.interface_ctx.unk_244), (GAMEOVER_INACTIVE, PAUSE_STATE_OFF, 0));
    let spawn = a.scenes.entrance_index("ENTR_DEKU_TREE_0").unwrap();
    assert_eq!(w.save.entrance_index, spawn);
}

#[test]
fn a_bottled_fairy_revives_link() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, "deku-tree-inside-fairy");
    hit_to_zero(&mut w);
    idle(&mut w, 1);
    // func_80836448: Inventory_ConsumeFairy empties the bottle, GAMEOVER_REVIVE_START, actionVar1
    // 1; GameOver_Update the same frame: GAMEOVER_REVIVE_RUMBLE, the lights, the letterbox to 32.
    assert_eq!(w.player().action, Action::Dying);
    assert_eq!(w.save.inventory.items[oot_game::item::SLOT_BOTTLE_1], oot_game::item::ITEM_BOTTLE_EMPTY);
    assert_eq!(w.game_over_ctx.state, GAMEOVER_REVIVE_RUMBLE);
    // func_80836448's OnePointCutscene_Init(9806, 120, &this->actor, CAM_ID_MAIN): a sub camera on
    // CAM_SET_TURN_AROUND (Camera_KeepOn4, data2 CAM_ITEM_TYPE_12) about Link, until ended
    // (timer -99); the main camera waits (CAM_STAT_UNK3).
    let link = w.player().actor.world_pos;
    let cam = |w: &PlayState, cs: i16| w.sub_cameras.iter().flatten().find(|c| c.cs_id == cs).cloned();
    let c = cam(&w, 9806).expect("9806");
    assert_eq!((w.active_cam_id, c.setting, c.timer, c.data2), (c.cam_id, CAM_SET_TURN_AROUND, -99, 0xC));
    assert_eq!(w.game_camera.status, CAM_STAT_UNK3);
    idle(&mut w, 30);
    let c = cam(&w, 9806).expect("9806");
    assert!(c.eye.distance(link) < 150.0, "the death camera about Link, not {:?}", c.eye);
    // The states' lengths: RUMBLE 1 frame (sGameOverTimer 50), WAIT_GROUND 50 (64), WAIT_FAIRY 64
    // (50), FADE_OUT 50, then INACTIVE: each state is left in on its first frame (the one that
    // set its timer), then on all but the last of its timer's frames.
    let mut runs: Vec<(u16, usize)> = Vec::new();
    let mut fairy = false;
    while w.game_over_ctx.state != GAMEOVER_INACTIVE {
        idle(&mut w, 1);
        let s = w.game_over_ctx.state;
        match runs.last_mut() {
            Some((x, n)) if *x == s => *n += 1,
            _ => runs.push((s, 1)),
        }
        let spawned = w.actors.all().into_iter().any(|h| w.actors.downcast::<oot_actors::en_elf::EnElf>(h).is_some_and(|e| e.actor.params == oot_actors::en_elf::FAIRY_REVIVE_DEATH));
        if spawned && !fairy {
            // func_80843AE8's OnePointCutscene_Init(9908, 125, ...): in front of 9806, which has
            // the lower priority (98 < 99) and is removed; 9908 active on CAM_SET_CS_C
            // (Camera_Unique9 on D_801231B4's keyframes).
            assert!(cam(&w, 9806).is_none());
            let c = cam(&w, 9908).expect("9908");
            assert_eq!((w.active_cam_id, c.setting, c.status), (c.cam_id, oot_game::onepoint::CAM_SET_CS_C, CAM_STAT_ACTIVE));
        }
        fairy |= spawned;
        assert!(runs.iter().map(|r| r.1).sum::<usize>() < 400);
    }
    assert_eq!(runs, vec![(GAMEOVER_REVIVE_WAIT_GROUND, 20), (GAMEOVER_REVIVE_WAIT_FAIRY, 64), (GAMEOVER_REVIVE_FADE_OUT, 50), (GAMEOVER_INACTIVE, 1)]);
    // func_80843AE8 spawned the fairy (Player_SpawnFairy, FAIRY_REVIVE_DEATH); 60 frames later
    // Link got up with 0x140 to count in (Interface_Update, 4 a frame, up to the 3 hearts' capacity),
    // and once counted he stands: PLAYER_STATE1_DEAD off, unk_A87 20, invulnerable for 20.
    assert!(fairy);
    for _ in 0..100 {
        if w.player().state1 & STATE1_7 == 0 {
            break;
        }
        idle(&mut w, 1);
    }
    assert!(w.player().state1 & STATE1_7 == 0);
    assert_eq!(w.save.health, 0x30);
    assert_eq!(w.pause_ctx.state, PAUSE_STATE_OFF);
    // 9908's timer out: back to the main camera, active again.
    for _ in 0..100 {
        if w.active_cam_id == CAM_ID_MAIN {
            break;
        }
        idle(&mut w, 1);
    }
    assert_eq!((w.active_cam_id, w.game_camera.status), (CAM_ID_MAIN, CAM_STAT_ACTIVE));
    assert!(w.sub_cameras.iter().all(|c| c.is_none()));
}

#[test]
fn the_deku_babas_tables_give_the_swords_damage() {
    let (n, nc, b) = (&en_dekubaba::S_DAMAGE_TABLE_NORMAL, &en_dekubaba::S_DAMAGE_TABLE_NORMAL_CHILD, &en_dekubaba::S_DAMAGE_TABLE_BIG);
    // sDamageTableNormal: the Kokiri Sword's slash DMG_ENTRY(1, EN_DEKUBABA_DMG_REACT_SWORD),
    // the Master Sword's (2, SWORD), a Deku Nut (0, STUN), the boomerang (2, BOOMERANG).
    assert_eq!(n.lookup(DMG_SLASH_KOKIRI), 0xF1);
    assert_eq!(n.lookup(DMG_SLASH_MASTER), 0xF2);
    assert_eq!(n.lookup(DMG_DEKU_NUT), 0x10);
    assert_eq!(n.lookup(DMG_BOOMERANG), 0xE2);
    // The Master Sword's jump attack, entry 27: DMG_ENTRY(4, SWORD) as written, DMG_ENTRY(4,
    // NONE) once a child's EnDekubaba_Init has rewritten it.
    assert_eq!(n.lookup(DMG_JUMP_MASTER), 0xF4);
    assert_eq!(nc.lookup(DMG_JUMP_MASTER), 0x04);
    // The big one's: the hookshot stuns (0, STUN) where the normal one's takes 2.
    assert_eq!((n.lookup(DMG_HOOKSHOT), b.lookup(DMG_HOOKSHOT)), (0x02, 0x10));
}

/// Runs the Deku Baba route from its debug start; returns the play state and the run.
fn deku_baba_run(a: &Arc<GameAssets>) -> (PlayState, Playthrough) {
    let route = Route::DekuBaba;
    let e = a.scenes.entrance_index(route.entrance()).expect("entrance");
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), route.save(e)).expect("Play_Init");
    let (pos, yaw) = route.start().unwrap();
    w.place_player(pos, yaw);
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    while let Some(p) = run.next(&w) {
        tick(&mut w, p, &mut prev);
    }
    (w, run)
}

#[test]
fn exit_a_deku_baba_bites_link_and_its_stem_is_cut() {
    let Some(a) = assets() else { return };
    let (w, run) = deku_baba_run(&a);
    assert!(run.failure.is_none(), "{:?}", run.failure);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::Bitten, Step::BabaWeakened, Step::BabaCut]);
    // The bite: the head's AT sphere (0xFFCFFFFF for 8) on Link's body: half a heart, and
    // EnDekubaba_Update's AT_HIT sends it to EnDekubaba_SetupSoothe. The second bite misses from
    // where the stagger left him.
    assert_eq!(w.save.health, 0x30 - 8);
    // The slash while it was stuck (EnDekubaba_RecoverFromAttackMiss): weakened, its health 2 - 1;
    // the stem's cut while stretched out: 1 - 1 = 0, EnDekubaba_SetupDieDropStick; now a stick.
    let b = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<EnDekubaba>(h).filter(|b| b.actor.home_pos.distance(oot_actors::playthrough::DEKU_BABA_HOME) < 1.0)).expect("the stick");
    assert_eq!(b.action, BabaAction::DekuStick);
    assert_eq!(b.actor.col_chk_info.health, 0);
    assert_eq!(b.actor.category, oot_game::actor_ctx::ACTORCAT_MISC);
    // EnDekubaba_SetupDekuStick: scale 0.03, 200 frames to blink out.
    assert_eq!(b.actor.scale, Vec3::splat(0.03));
}

#[test]
fn exit_the_dummy_hits_link() {
    // Link runs at a hurting dummy on the course: its touch hits him (a stagger, half a heart),
    // and once his invincibility is over and he's back at it, again.
    let Some(mut w) = child_on_course(Vec3::ZERO, -0x8000) else { return };
    idle(&mut w, 5);
    w.spawn_hurting_target(Vec3::new(0.0, 0.0, -150.0), HIT_SPECIAL_EFFECT_NONE);
    let mut prev = PadState::default();
    let mut hits = 0;
    let mut last = w.save.health;
    for _ in 0..200 {
        tick(&mut w, stick(0, 60), &mut prev);
        if w.save.health < last {
            hits += 1;
            assert_eq!(last - w.save.health, 8);
            assert_eq!(w.player().invincibility_timer, 20);
            last = w.save.health;
        }
    }
    assert!(hits >= 2, "{hits} hits");
}

#[test]
fn links_sword_hurts_the_dummy_by_its_table() {
    // The dummy takes the Deku Baba's sDamageTableNormal: the Kokiri Sword's slash is
    // DMG_ENTRY(1, EN_DEKUBABA_DMG_REACT_SWORD): damage 1, reaction 0xF, health 8 → 7.
    let Some(mut w) = child_on_course(Vec3::ZERO, -0x8000) else { return };
    idle(&mut w, 5);
    let h = w.spawn_target(Vec3::new(0.0, 0.0, -35.0));
    let mut prev = PadState::default();
    tick(&mut w, with(stick(0, 0), eng_input::pad::BTN_B), &mut prev);
    for _ in 0..20 {
        tick(&mut w, stick(0, 0), &mut prev);
    }
    // (The dummy keeps its AC on, so the swing hits it on each frame the blade crosses it.)
    let d = w.actors.downcast::<oot_actors::dummy_target::DummyTarget>(h).unwrap();
    assert!(d.hits >= 1);
    assert_eq!((d.last_damage, d.last_reaction), (1, 0xF));
    assert_eq!(d.actor.col_chk_info.health, 8 - d.hits as u8);
}

