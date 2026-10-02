//! Z-targeting polish: the camera modes Player asks for (`Player_UpdateCamAndSeqModes`),
//! `Camera_RequestModeImpl`, `Camera_Parallel1`, `Camera_KeepOn1`, the letterbox
//! (`Camera_UpdateInterface`, `Letterbox_Update`), the reticle (`Attention_Draw`), and
//! placeholders never being targets. Expected values come from `z_camera.c`,
//! `z_camera_data.inc.c`, `shrink_window.c` and `z_actor.c`, quoted per test.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_Z, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_ko::{self, EnKo};
use oot_game::actor::ACTOR_FLAG_ATTENTION_ENABLED;
use oot_game::camera::{CAM_MODE_Z_TARGET_FRIENDLY, CAM_MODE_NORMAL, CAM_MODE_Z_PARALLEL, diff_to_sph_geo};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn z(p: PadState) -> PadState {
    with(p, BTN_Z)
}

/// One frame with `pad` (the previous frame's pad held for edges).
fn frame(w: &mut PlayState, prev: &mut PadState, pad: PadState) {
    w.tick_with(scripted_input(*prev, pad));
    *prev = pad;
}

fn cam_yaw(w: &PlayState) -> i16 {
    diff_to_sph_geo(w.game_camera.at, w.game_camera.eye).yaw
}

#[test]
fn z_with_nothing_to_target_swings_the_camera_behind_and_letterboxes() {
    let Some(mut w) = world() else { return };
    let t = w.data.camera.oreg(23);
    // Camera_Init's sSceneInitLetterboxTimer: the main camera's first three updates hold the interface at
    // 0x3200 (the letterbox's target 32); then Normal1's flags (0x0003) take it back to 0.
    let mut prev = PadState::default();
    for _ in 0..3 {
        frame(&mut w, &mut prev, stick(0, 0));
        assert_eq!(w.letterbox.size_target, 32);
    }
    frame(&mut w, &mut prev, stick(0, 0));
    assert_eq!((w.cam_globals.scene_init_letterbox_timer, w.letterbox.size_target), (0, 0));
    // R_CAM_DEFAULT_ANIM_TIME (sOREGInit).
    assert!(t > 1);
    // Link turned to face +x with the camera still looking down -z.
    frame(&mut w, &mut prev, stick(0, 0));
    let p = w.player_mut();
    p.actor.shape_rot.y = 0x4000;
    p.actor.world_rot.y = 0x4000;
    p.current_yaw = 0x4000;
    let behind = 0x4000i16.wrapping_sub(0x7FFF);
    assert!((cam_yaw(&w).wrapping_sub(behind) as i32).abs() > 0x3000);
    // Z: PLAYER_STATE1_PARALLEL → CAM_MODE_Z_PARALLEL (CAM_FUNC_PARA1). Camera_Parallel1 animates for
    // animTimer = R_CAM_DEFAULT_ANIM_TIME frames (interfaceFlags 0x200A has no 4), turning
    // towards yawTarget = behind Player + roData->yawTarget (0) since flags & 2, and refusing
    // mode changes meanwhile (stateFlags & 0x20).
    for i in 0..t {
        frame(&mut w, &mut prev, z(stick(0, 0)));
        assert_eq!(w.game_camera.mode, CAM_MODE_Z_PARALLEL, "frame {i}");
        assert_ne!(w.game_camera.state_flags & 0x20, 0, "frame {i}");
        // The interface flags are only set once the animation is over: still Normal1's 0x0003.
        assert_eq!(w.letterbox.size_target, 0, "frame {i}");
    }
    // The triangular schedule ((yawTarget - yaw) / (T(T+1)/2) * T, ...) lands behind Player.
    assert!((cam_yaw(&w).wrapping_sub(behind) as i32).abs() < 0x200, "yaw {:#x}, behind {:#x}", cam_yaw(&w), behind);
    // animTimer 0: sCameraInterfaceField = 0x200A, so Camera_UpdateInterface asks for
    // letterbox size 27 (0x2000), which Letterbox_Update reaches 10 rows a frame
    // (R_UPDATE_RATE 3).
    frame(&mut w, &mut prev, z(stick(0, 0)));
    assert_eq!(w.game_camera.state_flags & 0x20, 0);
    assert_eq!(w.game_camera.interface_flags, 0x200A);
    assert_eq!(w.letterbox.size_target, 27);
    let mut sizes = Vec::new();
    for _ in 0..4 {
        frame(&mut w, &mut prev, z(stick(0, 0)));
        sizes.push(w.letterbox.size);
    }
    assert_eq!(sizes, vec![10, 20, 27, 27]);
    // pitchTarget 0 and distTarget 250 (scaled by Player's height, R_CAM_YOFFSET_NORM) once
    // settled.
    for _ in 0..40 {
        frame(&mut w, &mut prev, z(stick(0, 0)));
    }
    let e = diff_to_sph_geo(w.game_camera.at, w.game_camera.eye);
    let h = 68.0f32;
    let y_normal = 1.0 + (-10.0 * 0.01) - (-10.0 * 0.01) * (68.0 / h);
    assert!((e.r - 2.5 * h * y_normal).abs() < 3.0, "distance {}", e.r);
    assert!((e.pitch as i32).abs() < 0x100, "pitch {:#x}", e.pitch);
    // Letting go: back to NORMAL (Normal1, flags 0x0003), and the bars go.
    for _ in 0..6 {
        frame(&mut w, &mut prev, stick(0, 0));
    }
    assert_eq!(w.game_camera.mode, CAM_MODE_NORMAL);
    assert_eq!(w.letterbox.size, 0);
}

#[test]
fn a_mode_request_during_the_parallel_swing_is_refused() {
    let Some(mut w) = world() else { return };
    let t = w.data.camera.oreg(23) as usize;
    let mut prev = PadState::default();
    frame(&mut w, &mut prev, stick(0, 0));
    // Z for two frames, then let go: Player asks for NORMAL, but Camera_RequestModeImpl
    // refuses (stateFlags & 0x20, flags 0) until Camera_Parallel1's animTimer runs out.
    frame(&mut w, &mut prev, z(stick(0, 0)));
    frame(&mut w, &mut prev, z(stick(0, 0)));
    let mut modes = Vec::new();
    for _ in 0..t + 3 {
        frame(&mut w, &mut prev, stick(0, 0));
        modes.push(w.game_camera.mode);
    }
    // Frames 1..T animate (2 already done). The frame after the last animated one, Player's
    // request still meets stateFlags & 0x20 from the frame before; the next one goes through.
    let first_normal = modes.iter().position(|&m| m == CAM_MODE_NORMAL).expect("back to NORMAL");
    assert!(modes[..first_normal].iter().all(|&m| m == CAM_MODE_Z_PARALLEL));
    assert_eq!(first_normal, t - 2 + 1, "{modes:?}");
}

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn kokiri_forest() -> Option<PlayState> {
    let a = assets()?;
    let e = a.scenes.entrance_index("ENTR_KOKIRI_FOREST_3").expect("entrance");
    let save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    Some(oot_actors::play_entrance(a, data()?, rules()?, save).expect("Play_Init"))
}

#[test]
fn locking_on_a_kokiri_child_uses_keepon1_and_the_npc_colour() {
    let Some(mut w) = kokiri_forest() else { return };
    let mut prev = PadState::default();
    for _ in 0..4 {
        frame(&mut w, &mut prev, stick(0, 0));
    }
    let h = w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnKo>(h).is_some_and(|k| k.actor.params & 0xFF == en_ko::ENKO_TYPE_CHILD_1 as i16)).expect("child 1");
    let home = w.actors.actor(h).unwrap().home_pos;
    // Link 80 in front of it, facing it (-z), so it fades in and is the Z candidate.
    w.place_player(home + Vec3::new(0.0, 0.0, 80.0), i16::MIN);
    for _ in 0..20 {
        frame(&mut w, &mut prev, stick(0, 0));
    }
    assert_eq!(w.target_ctx.arrow_pointed, Some(h));
    frame(&mut w, &mut prev, z(stick(0, 0)));
    frame(&mut w, &mut prev, stick(0, 0));
    assert_eq!(w.player().focus_actor, Some(h));
    // Not hostile: PLAYER_STATE1_FRIENDLY_ACTOR_FOCUS → CAM_MODE_Z_TARGET_FRIENDLY (CAM_FUNC_KEEP1), with the child
    // as camera->target (Camera_SetViewParam 8).
    assert_eq!(w.game_camera.mode, CAM_MODE_Z_TARGET_FRIENDLY);
    assert_eq!(w.game_camera.target, Some(h));
    // Camera_KeepOn1 sets sCameraInterfaceField = 0x2001 every frame: letterbox 27.
    assert_eq!(w.game_camera.interface_flags, 0x2001);
    for _ in 0..4 {
        frame(&mut w, &mut prev, stick(0, 0));
    }
    assert_eq!(w.letterbox.size, 27);
    // Attention_InitReticle: the reticle takes sAttentionColors[ACTORCAT_NPC].inner (150, 150, 255).
    assert!(w.target_ctx.arr_50.iter().all(|e| e.color == [150, 150, 255]));
    assert!(w.target_ctx.reticle.is_some());
    // The camera looks at both: `at` lies between Link's head and the child's focus.
    let focus = w.actors.actor(h).unwrap().focus_pos;
    let head = w.player().actor.world_pos + Vec3::Y * 44.0;
    let at = w.game_camera.at;
    assert!(at.z < head.z && at.z > focus.z - 10.0, "at {at:?}, head {head:?}, focus {focus:?}");
    // Z again (no other candidate) lets go: KeepOn1 → NORMAL, no bars.
    frame(&mut w, &mut prev, z(stick(0, 0)));
    for _ in 0..6 {
        frame(&mut w, &mut prev, stick(0, 0));
    }
    assert_eq!(w.player().focus_actor, None);
    assert_eq!(w.game_camera.mode, CAM_MODE_NORMAL);
    assert_eq!(w.letterbox.size, 0);
}

#[test]
fn the_reticle_flies_in_from_the_centre_and_fades_when_lost() {
    let Some(mut w) = world() else { return };
    w.spawn_target(Vec3::new(0.0, 0.0, -150.0));
    let mut prev = PadState::default();
    frame(&mut w, &mut prev, stick(0, 0));
    frame(&mut w, &mut prev, z(stick(0, 0)));
    // Attention_InitReticle on the new target: reticleRadius 500 → Attention_Update steps it to 80 (30..100 a
    // frame). Attention_Draw scales the projected position by var1 = (500 - reticleRadius) / 420, so
    // the first entry sits near the screen's centre and later ones reach the target.
    let first = w.target_ctx.arr_50[w.target_ctx.cur_reticle as usize];
    let mut last = first;
    for _ in 0..12 {
        frame(&mut w, &mut prev, stick(0, 0));
        last = w.target_ctx.arr_50[w.target_ctx.cur_reticle as usize];
    }
    assert!(first.pos.truncate().length() < last.pos.truncate().length() || last.pos.truncate().length() < 1.0);
    assert!(w.target_ctx.reticle_spin_counter != 0);
    // Locked: reticleRadius 120 each frame, one entry drawn at full alpha, and the enemy colour
    // (sAttentionColors[ACTORCAT_ENEMY].inner = 255, 255, 0).
    assert_eq!(last.radius, 120.0);
    assert_eq!(w.target_ctx.reticle.map(|r| (r.alpha, r.count)), Some((0xFF, 1)));
    assert_eq!(last.color, [255, 255, 0]);
    // Lost (Z again): reticleFadeAlphaControl drops 120 a frame from 0x100: 136, 16, then 0 and nothing drawn.
    frame(&mut w, &mut prev, z(stick(0, 0)));
    let mut alphas = Vec::new();
    for _ in 0..4 {
        frame(&mut w, &mut prev, stick(0, 0));
        alphas.push(w.target_ctx.reticle.map(|r| r.alpha));
    }
    assert_eq!(w.player().focus_actor, None);
    assert!(alphas.contains(&Some(16)) && alphas.last() == Some(&None), "{alphas:?}");
}

#[test]
fn placeholders_are_never_targets() {
    let Some(mut w) = kokiri_forest() else { return };
    let mut prev = PadState::default();
    for _ in 0..4 {
        frame(&mut w, &mut prev, stick(0, 0));
    }
    let placeholders: Vec<_> = w.actors.all().into_iter().filter(|&h| w.actors.get(h).is_some_and(|a| a.name() == "Placeholder")).collect();
    assert!(!placeholders.is_empty());
    // Actor_Spawn gives a placeholder its profile's flags without ACTOR_FLAG_ATTENTION_ENABLED.
    for &h in &placeholders {
        assert_eq!(w.actors.actor(h).unwrap().flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    }
    // Stand next to each one facing it: Attention_FindActorInCategory skips actors without ACTOR_FLAG_ATTENTION_ENABLED, so
    // it's never the Z candidate, and Z can't lock on to it.
    for &h in placeholders.iter().take(12) {
        let Some(a) = w.actors.actor(h) else { continue };
        let pos = a.world_pos;
        w.place_player(pos + Vec3::new(0.0, 0.0, 60.0), i16::MIN);
        frame(&mut w, &mut prev, stick(0, 0));
        frame(&mut w, &mut prev, stick(0, 0));
        assert_ne!(w.target_ctx.arrow_pointed, Some(h));
        frame(&mut w, &mut prev, z(stick(0, 0)));
        assert_ne!(w.player().focus_actor, Some(h));
        frame(&mut w, &mut prev, stick(0, 0));
    }
}
