//! GAME-09 milestone 5 against the C: the Beamos (`z_en_vm.c`) and its two effects,
//! `Effect_Ss_Dead_Dd` (the eye's flash) and `Effect_Ss_Dead_Ds` (the beam's scorch). Dodongo's
//! Cavern's room 2 (Master Quest, `ddan_room_2`) places two small ones (params 0x0401) at
//! (-2277, 0, -1362) and (-1645, 0, -1362), facing +z (rot 0).
//!
//! Child Link enters the cavern at room 0 (`ENTR_DODONGOS_CAVERN_0`) on a `cavern-bombs` save and
//! room 2 is loaded (`Room_RequestNewRoom`); the tests use the east Beamos, Link placed 120 from
//! it facing it (the camera behind him, so it's drawn: `EnVm_PostLimbDraw` runs). Expected values
//! are worked out from the C in the comments. The numbers they use:
//! - `gBeamosAnim`'s last frame is 7; the eye (`beamPos1`, limb 2's origin) is 60 above the
//!   small one's base, so Link's feet 120 away are `atan(60 / 120)` (0x12E4) below it: in sight
//!   (0x0E38 to 0x1B91) and steep enough to fire at (0x0AAA);
//! - `Math_SmoothStepToS` with scale 1 lands on its target at once (the step's at least 0xFA0).

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_bom::{ACTOR_EN_BOM, EnBom};
use oot_actors::en_vm::*;
use oot_game::actor::*;
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::sfx::{NA_SE_EN_GOMA_JR_FREEZE, SFX_FLAG, SfxPos};
use oot_game::collision_check::{AC_HIT, AT_ON};
use oot_game::effect::{EFFECT_SS_DEAD_DD, EFFECT_SS_DEAD_DS, EffectSs};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Room 2's two (`ootx scene-info --scene ddan --layer 0`).
const WEST: Vec3 = Vec3::new(-2277.0, 0.0, -1362.0);
const EAST: Vec3 = Vec3::new(-1645.0, 0.0, -1362.0);
/// The room's middle, out of both's sight (over 300 from each).
const MIDDLE: Vec3 = Vec3::new(-1960.0, 0.0, -1362.0);

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// Child Link in room 2 (`cavern-bombs`, the sound log on), entered at room 0 and the room
/// changed; Link in the room's middle, out of sight, two frames on (both drawn once).
fn room2(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DODONGOS_CAVERN_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("cavern-bombs").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    assert!(w.room_request(2));
    idle(&mut w, 1);
    w.room_change_done();
    idle(&mut w, 3);
    w.place_player(MIDDLE, 0x4000);
    idle(&mut w, 2);
    w
}

fn all_vm(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnVm>(h).is_some_and(|a| !a.actor.killed)).collect()
}

fn at_home(w: &PlayState, home: Vec3) -> ActorHandle {
    *all_vm(w).iter().find(|&&h| vm(w, h).actor.home_pos == home).expect("the Beamos")
}

fn vm(w: &PlayState, h: ActorHandle) -> &EnVm {
    w.actors.downcast::<EnVm>(h).expect("En_Vm")
}

fn vm_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnVm {
    w.actors.downcast_mut::<EnVm>(h).expect("En_Vm")
}

/// The frames a sound was asked for at `h` (`Actor_PlaySfx`).
fn sfx_frames(w: &PlayState, h: ActorHandle, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, p)| s == id && p == SfxPos::Actor(h)).map(|&(f, _, _)| f).collect()
}

/// Runs until `done` or `max` frames; the frames run.
fn run_until(w: &mut PlayState, max: usize, mut done: impl FnMut(&PlayState) -> bool) -> usize {
    for n in 0..max {
        if done(w) {
            return n;
        }
        idle(w, 1);
    }
    panic!("not done in {max} frames");
}

/// The live effects of a type, newest slot last.
fn effects(w: &PlayState, ty: u8) -> Vec<usize> {
    w.effect_ss.table.iter().enumerate().filter(|(_, e)| e.ty == ty && e.life >= 0).map(|(i, _)| i).collect()
}

/// The east Beamos with its head straight (`headRotY` 0) and ten frames to aim, Link 120 in front
/// of it facing it (`EnVm_Wait` sees him from the next frame).
fn east_with_link_in_front(w: &mut PlayState) -> ActorHandle {
    let h = at_home(w, EAST);
    let x = vm_mut(w, h);
    x.head_rot_y = 0;
    x.timer = 10;
    w.place_player(EAST + Vec3::new(0.0, 0.0, 120.0), -0x8000);
    h
}

/// From `east_with_link_in_front` to the first frame of `EnVm_Attack` (19 frames: 10 to aim, 9
/// of the animation).
fn to_attack(w: &mut PlayState) -> ActorHandle {
    let h = east_with_link_in_front(w);
    let n = run_until(w, 40, |w| vm(w, h).action == Action::Attack);
    assert_eq!(n, 19);
    h
}

#[test]
fn room_2s_two_small_beamos_wait_for_link() {
    let Some(a) = assets() else { return };
    let w = room2(&a);
    let hs = all_vm(&w);
    assert_eq!(hs.len(), 2);
    for home in [WEST, EAST] {
        let x = vm(&w, at_home(&w, home));
        // EnVm_Init: params 0x0401: beamSightRange (0x0401 >> 8) × 40 = 160, params cut to 0x01
        // (not BEAMOS_LARGE): health 1, scale 0.01; NAVI_ENEMY_BEAMOS; no shadow
        // (ActorShape_Init(.., NULL, ..)).
        assert_eq!((x.actor.params, x.beam_sight_range), (BEAMOS_SMALL, 160.0));
        assert_eq!((x.actor.col_chk_info.health, x.actor.scale), (1, Vec3::splat(0.01)));
        assert_eq!((x.actor.navi_enemy_id, x.actor.shape_shadow_draw), (NAVI_ENEMY_BEAMOS, oot_game::shadow::ShadowDraw::None));
        assert_eq!(x.actor.flags & (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_UPDATE_CULLING_DISABLED), ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_UPDATE_CULLING_DISABLED);
        // EnVm_SetupWait: gBeamosAnim held at its last frame (7), unk_25E, unk_260, unk_21C 0;
        // Link out of sight: the timer still 10.
        assert_eq!((x.action, x.unk_25e, x.unk_260, x.unk_21c, x.timer), (Action::Wait, 0, 0, 0, 10));
        assert_eq!((x.skel.cur_frame, x.skel.end_frame), (7.0, 7.0));
        // sCylinderInit: 25 across, 70 high.
        assert_eq!((x.collider_cylinder.dim.radius, x.collider_cylinder.dim.height), (25, 70));
        // The focus: (6500 + yOffset 0) × 0.01 = 65 up.
        assert_eq!(x.actor.focus_pos, home + Vec3::new(0.0, 65.0, 0.0));
        // Drawn: the eye (limb 2's origin) 60 up, the flash's place 1600 (16) out along it.
        assert!(x.beam_pos1.distance(home + Vec3::new(0.0, 60.0, 0.0)) < 0.01, "{:?}", x.beam_pos1);
        assert!((x.beam_pos2.distance(x.beam_pos1) - 16.0).abs() < 0.01);
    }
}

#[test]
fn the_head_turns_500_a_frame_until_link_is_in_sight_then_ten_frames_to_aim() {
    let Some(a) = assets() else { return };
    let mut w = room2(&a);
    let h = at_home(&w, EAST);
    vm_mut(&mut w, h).head_rot_y = 0;
    // Link 120 north of it (behind its head, yaw 0x8000), facing it: out of sight by 0x8000. Each
    // frame `headRot` (yaw - headRotY) is checked, then headRotY -= 0x1F4; in sight once
    // |0x8000 - headRotY| <= 0x2710, headRotY <= -22768: the 47th frame (46 turns, -23000).
    w.place_player(EAST + Vec3::new(0.0, 0.0, -120.0), 0);
    for k in 1..=46 {
        idle(&mut w, 1);
        let x = vm(&w, h);
        assert_eq!((x.head_rot_y, x.timer), (-500 * k as i16, 10), "frame {k}");
    }
    idle(&mut w, 1);
    // In sight: the head towards him (yawTowardsPlayer - shape.rot.y, 0x8000) at scale 1, at most
    // (|(s16)((160 - 120) × 180)| / 3) + 0xFA0 = 6400 a frame; the difference (-0x8000 + 23000,
    // -9768) under 5460: the timer down. The next frame the rest (-3368): there.
    let x = vm(&w, h);
    assert_eq!((x.head_rot_y, x.timer), (-23000 - 6400, 9));
    idle(&mut w, 1);
    let x = vm(&w, h);
    assert_eq!((x.head_rot_y, x.timer), (-0x8000, 8));
    // Eight more frames in sight: the last aims (unk_25E 1, the animation from 0 at 2 a frame:
    // the frame's own update, 2), NA_SE_EN_BIMOS_AIM.
    idle(&mut w, 7);
    assert_eq!((vm(&w, h).timer, vm(&w, h).unk_25e), (1, 0));
    idle(&mut w, 1);
    let x = vm(&w, h);
    assert_eq!((x.timer, x.unk_25e, x.skel.cur_frame, x.skel.play_speed), (0, 1, 2.0, 2.0));
    assert_eq!(sfx_frames(&w, h, NA_SE_EN_BIMOS_AIM), vec![w.audio.frames]);
    // The eye tipped down at him (a tenth of the way, at most 0xFA0, each frame in sight).
    assert!(x.beam_rot[0] > 0);
    // NA_SE_EN_BIMOS_ROLL_HEAD every frame it's alive and not stunned.
    assert!(sfx_frames(&w, h, NA_SE_EN_BIMOS_ROLL_HEAD - SFX_FLAG).contains(&w.audio.frames));
}

#[test]
fn it_aims_flashes_and_fires_until_the_beam_hits_link() {
    let Some(a) = assets() else { return };
    let mut w = room2(&a);
    let h = east_with_link_in_front(&mut w);
    // Ten frames in sight: aimed (unk_25E 1). gBeamosAnim at 2 a frame: 2, 4, 6, 7 (its end),
    // then the update that finds it ended (unk_260 1, frame 0) and again: unk_260 2 on the 9th
    // frame after aiming.
    idle(&mut w, 10);
    assert_eq!(vm(&w, h).unk_25e, 1);
    let mut seen = Vec::new();
    for _ in 0..8 {
        idle(&mut w, 1);
        let x = vm(&w, h);
        seen.push((x.unk_260, x.skel.cur_frame));
    }
    assert_eq!(seen, vec![(0, 4.0), (0, 6.0), (0, 7.0), (1, 0.0), (1, 2.0), (1, 4.0), (1, 6.0), (1, 7.0)]);
    // The flash's place as the last draw left it.
    let flash_pos = vm(&w, h).beam_pos2;
    idle(&mut w, 1);
    // unk_260 2: aimed at him (beamRot.y his yaw, 0; the pitch 0x12E4 from the eye), steep enough:
    // the flash at beamPos2 and EnVm_SetupAttack: frames 3 to 7 at 3 a frame (from 3), 305 frames,
    // the beam 0.6 wide, 40 a frame, unk_21C 1.
    let x = vm(&w, h);
    assert_eq!((x.action, x.unk_21c, x.timer, x.beam_scale.x, x.beam_speed), (Action::Attack, 1, 305, 0.6, 40.0));
    assert_eq!((x.skel.cur_frame, x.skel.start_frame, x.skel.end_frame, x.skel.play_speed), (3.0, 3.0, 7.0, 3.0));
    let pitch = eng_math::atan2_s(120.0, 60.0);
    assert_eq!(x.beam_rot, [pitch, 0, 0]);
    // EffectSsDeadDd_Spawn(beamPos2, 0, 0, 150, -25, 0, 0, 255, 0, 255, 255, 255, 16, 20): its
    // first update the same frame (EffectSs_UpdateAll after the actors): life 19, scale 125,
    // alpha 0 + 16 (rAlphaMode 16: rising).
    let dd = effects(&w, EFFECT_SS_DEAD_DD);
    assert_eq!(dd.len(), 1);
    let e = &w.effect_ss.table[dd[0]];
    assert_eq!((e.pos, e.life, e.priority), (flash_pos, 19, 120));
    assert_eq!(e.regs[..12], [125, 0, 0, 0, 255, 16, 255, 255, 255, -25, 16, 16]);
    // Four frames still (timer 304 to 301: `--timer > 300` returns before the animation).
    for t in [304, 303, 302, 301] {
        idle(&mut w, 1);
        let x = vm(&w, h);
        assert_eq!((x.timer, x.beam_scale.x, x.beam_scale.z, x.skel.cur_frame, x.unk_260), (t, 0.6, 0.0, 3.0, 2));
    }
    // Timer 300: firing. The beam 0.6 - 0.12 wide, 40 long, NA_SE_EN_BIMOS_LAZER, unk_260 3 (no
    // AT yet: it was 2). The draw's line from the eye to 500 past its end (540 along the pitch)
    // meets the floor at Link's feet: 5 short of there (the eye to his feet: sqrt(120² + 60²)),
    // unk_260 4, beamPos3 the hit.
    idle(&mut w, 1);
    let x = vm(&w, h);
    let feet = EAST + Vec3::new(0.0, 0.0, 120.0);
    let eye = x.beam_pos1;
    assert_eq!((x.timer, x.unk_260), (300, 4));
    assert!((x.beam_scale.x - 0.48).abs() < 1e-6);
    assert!(x.beam_pos3.distance(feet) < 0.1, "{:?}", x.beam_pos3);
    assert!((x.beam_scale.z - (eye.distance(x.beam_pos3) - 5.0)).abs() < 1e-3);
    assert!((x.beam_scale.z - (134.164 - 5.0)).abs() < 0.01, "{}", x.beam_scale.z);
    assert!(sfx_frames(&w, h, NA_SE_EN_BIMOS_LAZER - SFX_FLAG).contains(&w.audio.frames));
    // The beam's quad: 5 either side of the eye (±500 at 0.01) to as far along its length
    // (`DistXYZ(beamPos1, beamPos3) × 100` in model units).
    let q = x.collider_quad1.dim.quad;
    let len = eye.distance(x.beam_pos3);
    assert!((q[3].distance(q[2]) - 10.0).abs() < 1e-3 && (q[1].distance(q[0]) - 10.0).abs() < 1e-3);
    assert!((((q[3] + q[2]) * 0.5).distance(eye)).abs() < 1e-3);
    assert!((((q[1] + q[0]) * 0.5).distance(eye) - len).abs() < 1e-2);
    let pos3 = x.beam_pos3;
    let health = w.save.health;
    // The next frame: the scorch where it touched (unk_260 4 at the update's start:
    // EffectSsDeadDs_SpawnStationary(beamPos3, 20, -1, 255, 20), priority 100;
    // NA_SE_EN_BIMOS_LAZER_GND); the AT on (unk_260 > 2): the beam hits Link (16).
    idle(&mut w, 1);
    assert!(sfx_frames(&w, h, NA_SE_EN_BIMOS_LAZER_GND - SFX_FLAG).contains(&w.audio.frames));
    let ds = effects(&w, EFFECT_SS_DEAD_DS);
    assert_eq!(ds.len(), 1);
    let e = &w.effect_ss.table[ds[0]];
    // EffectSsDeadDs_Init: rHalfOfLife 10, the alpha's step 255 / 10 = 25; life 19 after its
    // first update (no change before half its life); its first draw on the floor (1.5 above).
    assert_eq!((e.life, e.priority), (19, 100));
    assert_eq!((e.regs[0], e.regs[1], e.regs[5], e.regs[9], e.regs[10], e.regs[11]), (20, 1, 255, -1, 25, 10));
    assert!((e.pos.x - pos3.x).abs() < 1e-3 && (e.pos.z - pos3.z).abs() < 1e-3);
    assert!((e.pos.y - 1.5).abs() < 1e-3, "{:?}", e.pos);
    // The hit (AT_HIT) the frame after: the timer 0, the width held to 0.1 then narrowing 0.03 a
    // frame (unk_260 0): 0.07, 0.04, 0.01, 0; at 0 EnVm_SetupWait (the length 0).
    idle(&mut w, 1);
    let x = vm(&w, h);
    assert_eq!((x.timer, x.unk_260), (0, 0));
    // Player takes the hit in his update: 16 (a heart).
    assert_eq!(w.save.health, health - 16);
    assert!((x.beam_scale.x - 0.07).abs() < 1e-6);
    let mut widths = Vec::new();
    for _ in 0..3 {
        idle(&mut w, 1);
        widths.push(vm(&w, h).beam_scale.x);
    }
    assert!((widths[0] - 0.04).abs() < 1e-6 && (widths[1] - 0.01).abs() < 1e-6 && widths[2] == 0.0, "{widths:?}");
    let x = vm(&w, h);
    assert_eq!((x.action, x.timer, x.beam_scale), (Action::Wait, 10, Vec3::ZERO));
}

#[test]
fn the_beam_runs_305_frames_and_narrows_when_nothing_stops_it() {
    let Some(a) = assets() else { return };
    let mut w = room2(&a);
    let h = to_attack(&mut w);
    // The beam's quad never registered (its AT off): Link isn't hit.
    vm_mut(&mut w, h).collider_quad1.base.at_flags &= !AT_ON;
    idle(&mut w, 4);
    // Firing from timer 300: the width 0.6 to 0.1 by 0.12 a frame.
    let mut widths = Vec::new();
    for _ in 0..5 {
        idle(&mut w, 1);
        widths.push(vm(&w, h).beam_scale.x);
    }
    for (w_, e) in widths.iter().zip([0.48, 0.36, 0.24, 0.12, 0.1]) {
        assert!((w_ - e).abs() < 1e-6, "{widths:?}");
    }
    assert_eq!(vm(&w, h).timer, 296);
    // On to 0 (296 more frames firing, the scorch every frame), then the narrowing: 0.1 to 0 in
    // four frames, looking again.
    for t in (0..296).rev() {
        idle(&mut w, 1);
        let x = vm(&w, h);
        assert_eq!((x.action, x.timer, x.unk_260), (Action::Attack, t, 4), "timer {t}");
    }
    let n = run_until(&mut w, 10, |w| vm(w, h).action == Action::Wait);
    assert_eq!(n, 4);
    // A scorch each update that starts with unk_260 4: every firing frame's draw set it, the last
    // read by the first narrowing frame (3 frames ago); each lives 20 frames (19 after its first
    // update): those of the last 17 such frames are left.
    let n = effects(&w, EFFECT_SS_DEAD_DS).len();
    assert_eq!(n, 17, "{n} scorches");
}

#[test]
fn a_hit_on_the_eye_stuns_it_for_180_frames() {
    let Some(a) = assets() else { return };
    let mut w = room2(&a);
    let h = to_attack(&mut w);
    idle(&mut w, 5);
    // As CollisionCheck_AC leaves the eye's quad hit by Player for the next update.
    vm_mut(&mut w, h).collider_quad2.base.ac_flags |= AC_HIT;
    idle(&mut w, 1);
    // EnVm_CheckHealth: health 1 left: red 8 frames (255), EnVm_SetupStun: the animation from
    // its last frame back at -1, timer 180 (179 after the frame's EnVm_Stun), the beam's length
    // and unk_260 0, unk_21C 2, NA_SE_EN_GOMA_JR_FREEZE.
    let x = vm(&w, h);
    assert_eq!((x.action, x.unk_21c, x.timer, x.unk_25e, x.unk_260, x.beam_scale.z), (Action::Stun, 2, 179, 0, 0, 0.0));
    assert_eq!(x.actor.color_filter_params & 0xC000, COLORFILTER_COLORFLAG_RED);
    assert_eq!(x.actor.col_chk_info.health, 1);
    assert!(sfx_frames(&w, h, NA_SE_EN_GOMA_JR_FREEZE).contains(&w.audio.frames));
    let frame = w.audio.frames;
    // Stunned, no NA_SE_EN_BIMOS_ROLL_HEAD; another hit on the eye isn't taken.
    vm_mut(&mut w, h).collider_quad2.base.ac_flags |= AC_HIT;
    idle(&mut w, 1);
    assert!(!sfx_frames(&w, h, NA_SE_EN_BIMOS_ROLL_HEAD - SFX_FLAG).contains(&w.audio.frames));
    assert_eq!(vm(&w, h).timer, 178);
    // 178 more frames to 0; then the animation (at 0 for long) ends: unk_25E 1, forwards 0 to 7
    // (7 frames, the 8th finds it ended): unk_25E 2, 10 frames at 2 a frame; the 11th finds it
    // ended: unk_25E 3, EnVm_SetupWait.
    idle(&mut w, 178);
    assert_eq!((vm(&w, h).timer, vm(&w, h).unk_25e), (0, 0));
    idle(&mut w, 1);
    assert_eq!((vm(&w, h).unk_25e, vm(&w, h).skel.cur_frame, vm(&w, h).skel.play_speed), (1, 0.0, 1.0));
    idle(&mut w, 7);
    assert_eq!((vm(&w, h).unk_25e, vm(&w, h).skel.cur_frame), (1, 7.0));
    idle(&mut w, 1);
    assert_eq!((vm(&w, h).unk_25e, vm(&w, h).timer), (2, 10));
    idle(&mut w, 10);
    assert_eq!((vm(&w, h).action, vm(&w, h).timer), (Action::Stun, 0));
    idle(&mut w, 1);
    let x = vm(&w, h);
    assert_eq!((x.action, x.unk_21c, x.timer), (Action::Wait, 0, 10));
    assert_eq!(w.audio.frames - frame, 1 + 178 + 1 + 7 + 1 + 10 + 1);
}

/// A bomb lit to blow in two frames, 30 in front of `pos`.
fn bomb_by(w: &mut PlayState, pos: Vec3) -> ActorHandle {
    let b = w.actor_spawn(ACTOR_EN_BOM, pos + Vec3::new(0.0, 0.0, 30.0), [0, 0, 0], 0).expect("bomb");
    w.actors.downcast_mut::<EnBom>(b).unwrap().timer = 2;
    b
}

fn bombs(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&b| w.actors.downcast::<EnBom>(b).is_some_and(|x| !x.actor.killed)).collect()
}

#[test]
fn a_bomb_blows_the_small_beamos_head_off() {
    let Some(a) = assets() else { return };
    let mut w = room2(&a);
    let h = at_home(&w, EAST);
    let first = bomb_by(&mut w, EAST);
    // The blast (an explosive's AT on the metal cylinder, Actor_GetCollidedExplosive): health 1
    // to 0: a bomb with no fuse 20 up (BOMB_BODY, shape.rot.z 0x601: EnBom_Init keeps its low
    // byte, 1), EnVm_SetupDie.
    run_until(&mut w, 20, |w| vm(w, h).action == Action::Die);
    let x = vm(&w, h);
    assert_eq!((x.actor.col_chk_info.health, x.unk_21c, x.unk_25e, x.unk_260), (0, 3, 0, 0));
    let new: Vec<ActorHandle> = bombs(&w).into_iter().filter(|&b| b != first).collect();
    assert_eq!(new.len(), 1);
    let b = w.actors.downcast::<EnBom>(new[0]).unwrap();
    assert_eq!((b.actor.params, b.actor.shape_rot.z, b.timer), (0, 1, 0));
    assert!(b.actor.home_pos.distance(EAST + Vec3::new(0.0, 20.0, 0.0)) < 1e-3, "{:?}", b.actor.home_pos);
    // EnVm_SetupDie: the model 5000 down, the actor 5000 × 0.01 up; up at 8, gravity -0.5; the
    // frame's EnVm_Die: timer 32, spun (beamRot.x 0x5DC, headRotY 0x9C4 a frame), moved
    // (Actor_MoveXZGravity: vy 7.5, × 1.5).
    assert_eq!((x.timer, x.actor.shape_y_offset, x.actor.gravity, x.actor.velocity.y), (32, -5000.0, -0.5, 7.5));
    assert!((1.0..2.0).contains(&x.actor.speed_xz));
    assert!((x.actor.world_pos.y - (50.0 + 7.5 * 1.5)).abs() < 1e-3);
    // The focus (6500 - 5000) × 0.01 = 15 up.
    assert!((x.actor.focus_pos.y - x.actor.world_pos.y - 15.0).abs() < 1e-3);
    // No more hits taken (health 0), no NA_SE_EN_BIMOS_ROLL_HEAD.
    let (rot_x, head) = (x.beam_rot[0], x.head_rot_y);
    idle(&mut w, 1);
    let x = vm(&w, h);
    assert_eq!((x.beam_rot[0], x.head_rot_y), (rot_x.wrapping_add(0x5DC), head.wrapping_add(0x9C4)));
    assert!(!sfx_frames(&w, h, NA_SE_EN_BIMOS_ROLL_HEAD - SFX_FLAG).contains(&w.audio.frames));
    // 31 frames more to timer 0: the second bomb (shape.rot.z 0x6FF: 0xFF, -1) where it is, the
    // drop (table 10), gone. Its height: 50 + 1.5 × Σ(8 - 0.5k), k 1 to 33 = 50 - 24.75.
    let before: Vec<ActorHandle> = bombs(&w);
    idle(&mut w, 30);
    assert_eq!(vm(&w, h).timer, 1);
    idle(&mut w, 1);
    assert!(w.actors.get(h).is_none_or(|a| a.base().killed), "the Beamos is gone");
    let new: Vec<ActorHandle> = bombs(&w).into_iter().filter(|b| !before.contains(b)).collect();
    assert_eq!(new.len(), 1);
    let b = w.actors.downcast::<EnBom>(new[0]).unwrap();
    assert_eq!((b.actor.shape_rot.z, b.timer), (-1, 0));
    assert!((b.actor.home_pos.y - 25.25).abs() < 1e-3, "{:?}", b.actor.home_pos);
}

#[test]
fn a_large_beamos_takes_two_blasts() {
    let Some(a) = assets() else { return };
    let mut w = room2(&a);
    // params 0x0300: BEAMOS_LARGE, sight 3 × 40.
    let pos = Vec3::new(-1960.0, 0.0, -1500.0);
    let h = w.actor_spawn(ACTOR_EN_VM, pos, [0, 0, 0], 0x0300).expect("a Beamos");
    idle(&mut w, 1);
    let x = vm(&w, h);
    assert_eq!((x.actor.params, x.beam_sight_range, x.actor.col_chk_info.health, x.actor.scale), (BEAMOS_LARGE, 120.0, 2, Vec3::splat(0.014)));
    // The first blast: health 1, red 8 frames and stunned (as by the eye).
    bomb_by(&mut w, pos);
    run_until(&mut w, 20, |w| vm(w, h).action == Action::Stun);
    let x = vm(&w, h);
    assert_eq!((x.actor.col_chk_info.health, x.unk_21c), (1, 2));
    assert_eq!(x.actor.color_filter_params & 0xC000, COLORFILTER_COLORFLAG_RED);
    // Its cylinder takes hits again once the red's gone (colorFilterTimer 0): the second blast,
    // stunned or not, takes the last: the head flies off.
    run_until(&mut w, 20, |w| vm(w, h).actor.color_filter_timer == 0);
    run_until(&mut w, 30, |w| bombs(w).is_empty());
    bomb_by(&mut w, pos);
    run_until(&mut w, 20, |w| vm(w, h).action == Action::Die);
    assert_eq!(vm(&w, h).actor.col_chk_info.health, 0);
}

// The effects alone, on the test course.

/// The live effect at `i`.
fn ss(w: &PlayState, i: usize) -> &EffectSs {
    &w.effect_ss.table[i]
}

#[test]
fn the_eyes_flash_shrinks_25_and_brightens_16_a_frame() {
    let Some(mut w) = world() else { return };
    idle(&mut w, 1);
    let pos = Vec3::new(0.0, 60.0, -100.0);
    // EnVm_Wait's: scale 150 step -25, prim (0, 0, 255) alpha 0, env (255, 255, 255), alpha
    // step 16 (rising), 20 frames.
    w.with_ss(|s| s.dead_dd_spawn(pos, Vec3::ZERO, Vec3::ZERO, 150, -25, [0, 0, 255], 0, [255, 255, 255], 16, 20));
    let i = effects(&w, EFFECT_SS_DEAD_DD)[0];
    assert_eq!((ss(&w, i).life, ss(&w, i).regs[0], ss(&w, i).regs[5]), (20, 150, 0));
    let name = oot_game::pack::keys::bake("EffectSs/dead_dd");
    for life in (0..20).rev() {
        idle(&mut w, 1);
        let e = ss(&w, i);
        let k = 20 - life;
        let scale = (150 - 25 * k as i16).max(0);
        let alpha = (16 * k as i16).min(255);
        assert_eq!((e.life, e.regs[0], e.regs[5]), (life, scale, alpha), "life {life}");
        // EffectSsDeadDd_Draw: at its position, rScale × 0.01 across (the billboard after), prim
        // and env at rAlpha.
        let d = w.effect_draws.xlu.iter().find(|d| d.mesh.name == name).expect("drawn");
        let sv = d.params.segments.as_ref().unwrap();
        assert_eq!(sv.prim[0x0E], Some([0, 0, 255, alpha as u8]));
        assert_eq!(sv.env[0x0E], Some([255, 255, 255, alpha as u8]));
        assert!((d.transform.x_axis.truncate().length() - scale as f32 * 0.01).abs() < 1e-4);
        assert_eq!(d.transform.w_axis.truncate(), pos);
    }
    idle(&mut w, 1);
    assert_eq!(ss(&w, i).life, -1);
}

#[test]
fn the_unused_yellow_flash_fades_from_155() {
    let Some(mut w) = world() else { return };
    idle(&mut w, 1);
    let pos = Vec3::new(0.0, 60.0, -100.0);
    // EffectSsDeadDd_SpawnRandYellow(pos, 100, 5, 40, 3, 10): type 1: life 10, alpha 155 down
    // 155 / 10 = 15 a frame, prim (255, 255, 155), env (250, 180, 0); the position up to 20 off
    // each way (three tries, the last kept), the velocity up to 1 each way.
    w.with_ss(|s| s.dead_dd_spawn_rand_yellow(pos, 100, 5, 40.0, 3, 10));
    let i = effects(&w, EFFECT_SS_DEAD_DD)[0];
    let e = ss(&w, i);
    assert_eq!((e.life, e.regs[0], e.regs[2], e.regs[3], e.regs[4], e.regs[5]), (10, 100, 255, 255, 155, 155));
    assert_eq!((e.regs[6], e.regs[7], e.regs[8], e.regs[9], e.regs[10], e.regs[11]), (250, 180, 0, 5, 15, 0));
    assert!((e.pos - pos).abs().max_element() <= 20.0);
    assert!(e.velocity.abs().max_element() <= 1.0 && e.velocity == e.accel);
    idle(&mut w, 1);
    let e = ss(&w, i);
    assert_eq!((e.regs[0], e.regs[5]), (105, 140));
}

#[test]
fn the_scorch_lies_on_the_floor_or_the_wall_and_fades_its_second_half() {
    let Some(mut w) = world() else { return };
    idle(&mut w, 1);
    // On the course's floor (y 0) where a beam would meet it.
    let floor = Vec3::new(100.0, 0.0, 100.0);
    w.with_ss(|s| s.dead_ds_spawn_stationary(floor, 20, -1, 255, 20));
    // Against the pit's west wall (x 400, facing +x), 0.5 off it, 200 down.
    let wall = Vec3::new(400.5, -200.0, -600.0);
    w.with_ss(|s| s.dead_ds_spawn_stationary(wall, 20, -1, 255, 20));
    let ds = effects(&w, EFFECT_SS_DEAD_DS);
    assert_eq!(ds.len(), 2);
    let find = |w: &PlayState, at: Vec3| *ds.iter().find(|&&i| (ss(w, i).pos.x - at.x).abs() < 2.0 && (ss(w, i).pos.z - at.z).abs() < 2.0).unwrap();
    let (fi, wi) = (find(&w, floor), find(&w, wall));
    idle(&mut w, 1);
    // The first draw: the floor under it (from 1 up): 1.5 above it, flat (the mark's face up);
    // the wall: pushed out to 1.5 from it, the mark's face along the wall's normal.
    let e = ss(&w, fi);
    assert_eq!(e.regs[1], 1);
    assert!(e.pos.distance(Vec3::new(100.0, 1.5, 100.0)) < 1e-3, "{:?}", e.pos);
    let m = oot_game::effect::dead_ds::mark_matrix(e);
    assert!(m.transform_vector3(Vec3::Z).normalize().dot(Vec3::Y).abs() > 0.999);
    let e = ss(&w, wi);
    assert_eq!(e.regs[1], 1);
    assert!(e.pos.distance(Vec3::new(401.5, -200.0, -600.0)) < 1e-3, "{:?}", e.pos);
    let m = oot_game::effect::dead_ds::mark_matrix(e);
    assert!(m.transform_vector3(Vec3::Z).normalize().dot(Vec3::X).abs() > 0.999);
    // Its life 19 down: no change until life < 10, then the scale down 1 and the alpha 25 a frame.
    for life in (0..19).rev() {
        idle(&mut w, 1);
        let e = ss(&w, fi);
        let k = (10 - life).max(0) as i16;
        assert_eq!((e.life, e.regs[0], e.regs[5]), (life, 20 - k, 255 - 25 * k), "life {life}");
        let name = oot_game::pack::keys::bake("EffectSs/dead_ds");
        let d = w.effect_draws.xlu.iter().find(|d| d.mesh.name == name && d.transform.w_axis.truncate().distance(e.pos) < 1e-3).expect("drawn");
        let sv = d.params.segments.as_ref().unwrap();
        assert_eq!((sv.prim[0x0E], sv.env[0x0E]), (Some([0, 0, 0, (255 - 25 * k) as u8]), Some([0, 0, 0, 0])));
    }
}
