//! GAME-10 milestone 8 against the C: Big Octo (`z_en_bigokuta.c`) on Jabu-Jabu's rotating platform
//! (`Bg_Bdan_Objects` 0x1F00, room 6, home (-1360, -1025, -3343)).
//!
//! Child Link enters at `ENTR_JABU_JABU_0` on a `jabu-fish` save and the room changes to 6. Big Octo
//! comes from the real platform: spawned as its child as Ruto's cutscene has it (params 0), or at
//! its init with `INFTABLE_RUTO_ABDUCTED` set (params 3). Each test runs one actor's update alone
//! (`Actor_UpdateAll`'s call), Link's distance and yaw set as `Actor_UpdateAll` would, so no fight
//! is scripted and nothing waits on `Rand`. Expected values are worked out from the C in the
//! comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use eng_math::{cos_s, scaled_step_to_s, sin_s, vec3f_yaw};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_bdan_objects::{self as objects, BgBdanObjects};
use oot_actors::en_bigokuta::*;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ACTORCAT_PROP, ActorHandle, ActorImpl};
use oot_game::audio::bgm::start_seq;
use oot_game::audio::sfx::{SFX_FLAG, SfxPos};
use oot_game::camera::{CAM_ID_MAIN, CAM_SET_DUNGEON0};
use oot_game::collision_check::*;
use oot_game::effect::{EFFECT_SS_DUST, EFFECT_SS_G_RIPPLE, EFFECT_SS_G_SPLASH};
use oot_game::play::{DrawOut, PlayState, ViewInfo, scripted_input};
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

/// The platform's home (`ootx scene-info --scene bdan_scene`).
const PLATFORM_HOME: Vec3 = Vec3::new(-1360.0, -1025.0, -3343.0);

/// Child Link in Jabu-Jabu (`jabu-fish`, `save` changed first), the sound log on, in room 6.
fn jabu(a: &Arc<GameAssets>, save_fn: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_JABU_JABU_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("jabu-fish").unwrap();
    save_fn(&mut save);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    assert!(w.room_request(6));
    idle(&mut w, 1);
    w.room_change_done();
    assert_eq!(w.room_ctx.cur.num, 6);
    idle(&mut w, 2);
    w
}

fn platform(w: &PlayState) -> ActorHandle {
    *w.actors
        .all()
        .iter()
        .find(|&&h| w.actors.downcast::<BgBdanObjects>(h).is_some_and(|o| !o.actor.killed && o.actor.params == objects::JABU_OBJECTS_TYPE_BIG_OCTO_PLATFORM && o.actor.home_pos == PLATFORM_HOME))
        .expect("Big Octo's platform")
}

fn ob(w: &mut PlayState, h: ActorHandle) -> &mut BgBdanObjects {
    w.actors.downcast_mut::<BgBdanObjects>(h).expect("Bg_Bdan_Objects")
}

fn bo(w: &PlayState, h: ActorHandle) -> &EnBigokuta {
    w.actors.downcast::<EnBigokuta>(h).expect("En_Bigokuta")
}

fn bo_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnBigokuta {
    w.actors.downcast_mut::<EnBigokuta>(h).expect("En_Bigokuta")
}

fn gone(w: &PlayState, h: ActorHandle) -> bool {
    w.actors.downcast::<EnBigokuta>(h).is_none_or(|o| o.actor.killed)
}

/// The actor's update alone (`Actor_UpdateAll`'s call), as the updating actor.
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

fn draw_update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.draw_update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// The times a sound was asked for at `h`.
fn sfx(w: &PlayState, h: ActorHandle, id: u16) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, p)| s == id && p == SfxPos::Actor(h)).count()
}

fn sfx_any(w: &PlayState, id: u16) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).count()
}

fn seq_cmds(w: &PlayState, cmd: u32) -> usize {
    w.audio.log.as_ref().unwrap().seq_cmds.iter().filter(|&&(_, c)| c == cmd).count()
}

/// The live soft sprites of a type.
fn effects(w: &PlayState, ty: u8) -> usize {
    w.effect_ss.table.iter().filter(|e| e.ty == ty && e.life >= 0).count()
}

fn camera(w: &PlayState) -> (i16, i16) {
    let c = w.camera(CAM_ID_MAIN).expect("the main camera");
    (c.setting, c.state_flags & CAM_STATE_CHECK_BG)
}

/// A new frame for the main camera (`Camera_Update` clears its requests of the frame).
fn camera_frame(w: &mut PlayState) {
    w.camera_mut(CAM_ID_MAIN).unwrap().behavior_flags = 0;
}

/// Ruto's cue (`CAM_SET_DUNGEON1`, `En_Ru1`'s): the platform spawns Big Octo (params 0) 140 over
/// it as its child.
fn abducted(w: &mut PlayState) -> (ActorHandle, ActorHandle) {
    let p = platform(w);
    ob(w, p).action = objects::Action::OctoPlatformWaitForRutoToAdvanceCutscene;
    ob(w, p).timer = 0;
    ob(w, p).camera_setting = objects::CAM_SET_DUNGEON1;
    update(w, p);
    let b = ob(w, p).actor.child.expect("Big Octo spawned as the platform's child");
    (p, b)
}

/// Met before: the platform's init spawned him (params 3); on the rim, circling.
fn met(a: &Arc<GameAssets>) -> (PlayState, ActorHandle, ActorHandle) {
    let mut w = jabu(a, |s| s.set_inf_table(objects::INFTABLE_RUTO_ABDUCTED));
    let p = platform(&w);
    let b = ob(&mut w, p).actor.child.expect("Big Octo, params 3");
    update(&mut w, b);
    assert_eq!(bo(&w, b).action, Action::Circle);
    (w, p, b)
}

/// Circling `dir` round at shape yaw `s`, on the rim (`func_809BCE3C`), the counts fresh.
fn place(w: &mut PlayState, b: ActorHandle, s: i16, dir: i8) {
    let o = bo_mut(w, b);
    o.action = Action::Circle;
    o.unk_194 = dir;
    o.unk_195 = false;
    o.unk_196 = 350;
    o.unk_198 = 80;
    o.actor.shape_rot.y = s;
    o.actor.world_rot.y = (s as i32 + dir as i32 * -0x4000) as i16;
    let (w_y, home) = (o.actor.world_rot.y, o.actor.home_pos);
    o.actor.world_pos = Vec3::new(sin_s(w_y) * RIM + home.x, home.y, cos_s(w_y) * RIM + home.z);
}

/// Link `d` from the platform's middle along Big Octo's side (`d` 263 is his place) and 20 back
/// from his face's line (his frame's z -20); his distance and yaw as `Actor_UpdateAll` has them.
fn link_along(w: &mut PlayState, b: ActorHandle, d: f32, xz_dist: f32, yaw: i16) {
    let o = bo(w, b);
    let (w_y, s, home) = (o.actor.world_rot.y, o.actor.shape_rot.y, o.actor.home_pos);
    let ph = w.player.unwrap();
    w.actors.actor_mut(ph).unwrap().world_pos = Vec3::new(sin_s(w_y) * d - sin_s(s) * 20.0 + home.x, home.y, cos_s(w_y) * d - cos_s(s) * 20.0 + home.z);
    let o = bo_mut(w, b);
    o.actor.xz_dist_to_player = xz_dist;
    o.actor.yaw_towards_player = yaw;
}

/// His back hit with `damage` and `reaction` (`CollisionCheck_SetInfo`'s table read), the hit
/// element's `dmg_flags`.
fn hit(w: &mut PlayState, b: ActorHandle, damage: u8, reaction: u8, dmg_flags: u32) {
    let ph = w.player.unwrap();
    let o = bo_mut(w, b);
    o.collider.base.ac_flags |= AC_HIT;
    o.collider.elements[0].info.ac_hit_elem = Some(HitElem {
        elem: ElemRef { col: ColliderRef { actor: ph, id: 0 }, elem: 0 },
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags, hit_special_effect: 0, damage },
        ac_dmg_info: Default::default(),
        elem_material: 0,
    });
    o.actor.col_chk_info.damage = damage;
    o.actor.col_chk_info.damage_reaction = reaction;
}

#[test]
fn ruto_abducted_he_comes_down_with_the_platform_and_jumps_to_its_rim_on_its_cue() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a, |_| {});
    let (p, b) = abducted(&mut w);
    let pa = ob(&mut w, p).actor.clone();
    // EnBigokuta_Init (params 0): a prop, floating, 121 frames, his home at the surface (-1025);
    // sInitChain: lockOnArrowOffset 2000, ATTENTION_RANGE_2, gravity -1, NAVI_ENEMY_BIGOCTO,
    // scale 0.033; sColChkInfoInit 4, 130, 120, 200; the sphere 75 (its world radius set at init),
    // the cylinders 50 by 100.
    let o = bo(&w, b);
    assert_eq!((o.actor.params, o.actor.category, o.action, o.unk_196, o.unk_194), (PARAMS_INTRO, ACTORCAT_PROP, Action::IntroWait, 121, 1));
    assert!(o.skel.is(ANIM_FLOAT));
    assert_eq!(o.actor.parent, Some(p));
    assert_eq!(o.actor.world_pos, pa.world_pos + Vec3::Y * 140.0);
    assert_eq!(o.actor.shape_rot.y, pa.shape_rot.y.wrapping_add(-0x8000i16));
    assert_eq!((o.actor.home_pos.x, o.actor.home_pos.y, o.actor.home_pos.z), (pa.world_pos.x, -1025.0, pa.world_pos.z));
    assert_eq!((o.actor.target_arrow_offset, o.actor.target_mode, o.actor.gravity, o.actor.navi_enemy_id, o.actor.scale), (2000.0, 2, -1.0, NAVI_ENEMY_BIGOCTO, Vec3::splat(0.033)));
    let c = &o.actor.col_chk_info;
    assert_eq!((c.health, c.cyl_radius, c.cyl_height, c.mass), (4, 130, 120, 200));
    assert_eq!((o.collider.elements[0].dim.world_sphere.radius, o.collider.elements[0].dim.model_sphere.center), (75, [0, 45, -30]));
    assert_eq!(o.collider_cylinders.iter().map(|c| (c.dim.radius, c.dim.height)).collect::<Vec<_>>(), vec![(50, 100), (50, 100)]);
    assert_eq!(o.actor.flags & (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED), ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED);
    assert_eq!(o.actor.shape_shadow_draw, oot_game::shadow::ShadowDraw::None);
    // func_809BD84C: his voice at 13 (108 updates) and -20 (141), the mini-boss music at 1 (120).
    let music = start_seq(0, 0, NA_BGM_MINI_BOSS);
    for n in 1..=141 {
        update(&mut w, b);
        let voices = sfx(&w, b, NA_SE_EN_DAIOCTA_VOICE);
        assert_eq!(voices, (n >= 108) as usize + (n >= 141) as usize, "update {n}");
        assert_eq!(seq_cmds(&w, music), (n >= 120) as usize, "update {n}");
    }
    assert_eq!((bo(&w, b).action, bo(&w, b).unk_196), (Action::IntroWait, -20));
    // Every update: CAM_SET_BIG_OCTO without the bg's settings.
    assert_eq!(camera(&w), (CAM_SET_BIG_OCTO, 0));
    // The platform landed, 60 frames on: his params 0 made 1 (BgBdanObjects' cue).
    ob(&mut w, p).action = objects::Action::OctoPlatformWaitForBigOctoToStartBattle;
    ob(&mut w, p).timer = 1;
    update(&mut w, p);
    assert_eq!(bo(&w, b).actor.params, PARAMS_JUMP);
    // func_809BD370: the jump, 21 frames, NA_SE_EN_STAL_JUMP.
    update(&mut w, b);
    assert_eq!((bo(&w, b).action, bo(&w, b).unk_196), (Action::IntroJump, 21));
    assert_eq!(sfx(&w, b, NA_SE_EN_STAL_JUMP), 1);
    // func_809BD8DC: out 263/21 a frame along x; up sinf((t - 7) π/14) × 130 over 70 above home to
    // 14, sinf(t π/28) × 200 below; at 0 two splashes, the landing's sounds.
    let o = bo(&w, b);
    let (home, x0) = (o.actor.home_pos, o.actor.world_pos.x);
    let splashes = effects(&w, EFFECT_SS_G_SPLASH);
    use std::f32::consts::PI;
    for t in (0..=20i16).rev() {
        update(&mut w, b);
        let o = bo(&w, b);
        assert_eq!(o.unk_196, t);
        let want_y = if t < 14 { ((t as f32 * (PI / 28.0)).sin() * 200.0) + home.y } else { ((((t - 7) as f32) * (PI / 14.0)).sin() * 130.0) + (home.y + 70.0) };
        assert_eq!(o.actor.world_pos.y, want_y, "t {t}");
        let want_x = (x0 + (21 - t) as f32 * (RIM / 21.0)).min(home.x + RIM);
        assert!((o.actor.world_pos.x - want_x).abs() < 0.01, "t {t}: {} {want_x}", o.actor.world_pos.x);
    }
    assert_eq!(effects(&w, EFFECT_SS_G_SPLASH), splashes + 2);
    assert_eq!((sfx(&w, b, NA_SE_EN_DAIOCTA_LAND_WATER), sfx(&w, b, NA_SE_EN_GOLON_LAND_BIG)), (1, 1));
    // -1 nothing; -2 to -10 a bob of 20 (home - sinf((t + 1) π/10) × 20); at -10 on the rim (263
    // along x) at the surface, turning, 10 frames.
    update(&mut w, b);
    assert_eq!((bo(&w, b).unk_196, bo(&w, b).actor.world_pos.y), (-1, home.y));
    for t in (-9..=-2i16).rev() {
        update(&mut w, b);
        assert_eq!(bo(&w, b).unk_196, t);
        assert_eq!(bo(&w, b).actor.world_pos.y, home.y - ((((t + 1) as f32) * (PI / 10.0)).sin() * 20.0), "t {t}");
    }
    update(&mut w, b);
    let o = bo(&w, b);
    assert_eq!((o.action, o.unk_196, o.actor.world_pos.x, o.actor.world_pos.y), (Action::IntroTurn, 10, home.x + RIM, home.y));
    // func_809BDAE8: turned to home.rot.y + 0x4000 (0x400 a frame, scaled), 10 frames on: on the
    // rim facing along it (func_809BCE3C), an enemy, params 2, his voice, 40 frames to wait.
    let target = o.actor.home_rot.y.wrapping_add(0x4000);
    let voices = sfx(&w, b, NA_SE_EN_DAIOCTA_VOICE);
    let mut turned_at = None;
    for n in 1..=60 {
        update(&mut w, b);
        let o = bo(&w, b);
        if turned_at.is_none() && o.actor.shape_rot.y == target {
            turned_at = Some(n);
        }
        if o.action != Action::IntroTurn {
            // The frame it got there counts the first of the 10.
            assert_eq!(n - turned_at.unwrap(), 9);
            break;
        }
    }
    let o = bo(&w, b);
    assert_eq!((o.action, o.unk_196, o.actor.params, o.actor.category), (Action::WaitToFight, 40, PARAMS_BATTLE, ACTORCAT_ENEMY));
    assert_eq!(o.actor.world_rot.y, o.actor.home_rot.y);
    let wy = o.actor.world_rot.y;
    assert_eq!((o.actor.world_pos.x, o.actor.world_pos.z), (sin_s(wy) * RIM + home.x, cos_s(wy) * RIM + home.z));
    assert_eq!(o.actor.home_pos.y, home.y);
    assert_eq!(sfx(&w, b, NA_SE_EN_DAIOCTA_VOICE), voices + 1);
    // The platform sees params 2: the battle's on.
    ob(&mut w, p).timer = 0;
    update(&mut w, p);
    assert_eq!(ob(&mut w, p).action, objects::Action::OctoPlatformBattleInProgress);
    // func_809BDB90: 40 frames (no music again for params 2), circling: 350, 80, the platform
    // 0x200 against him, his front hurting, his back hittable.
    for _ in 0..40 {
        update(&mut w, b);
    }
    let o = bo(&w, b);
    assert_eq!((o.action, o.unk_196, o.unk_198, o.unk_19a), (Action::Circle, 350, 80, -0x200));
    assert!(o.skel.is(ANIM_SWIM));
    assert_ne!(o.collider_cylinders[0].base.at_flags & AT_ON, 0);
    assert_ne!(o.collider.base.ac_flags & AC_ON, 0);
    assert_eq!(seq_cmds(&w, music), 1);
}

#[test]
fn met_before_the_platform_spawns_him_on_its_rim_and_he_fights_at_once() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a, |s| s.set_inf_table(objects::INFTABLE_RUTO_ABDUCTED));
    let p = platform(&w);
    // BgBdanObjects_Init: Big Octo (params 3) at its home facing back, 263 along z, its child;
    // the battle on.
    assert_eq!(ob(&mut w, p).action, objects::Action::OctoPlatformBattleInProgress);
    let b = ob(&mut w, p).actor.child.expect("Big Octo");
    let o = bo(&w, b);
    // EnBigokuta_Init (params 3): an enemy, waiting 1 frame (func_809BD3E0's 40 overwritten), his
    // home at the surface.
    assert_eq!((o.actor.params, o.actor.category, o.action, o.unk_196, o.unk_19a), (PARAMS_MET, ACTORCAT_ENEMY, Action::WaitToFight, 1, 0));
    assert_eq!(o.actor.parent, Some(p));
    assert_eq!((o.actor.home_pos, o.actor.world_pos.z), (Vec3::new(PLATFORM_HOME.x, -1025.0, PLATFORM_HOME.z), PLATFORM_HOME.z + 263.0));
    // func_809BDB90: params 3: the mini-boss music, circling.
    let music = start_seq(0, 0, NA_BGM_MINI_BOSS);
    let before = seq_cmds(&w, music);
    update(&mut w, b);
    assert_eq!(seq_cmds(&w, music), before + 1);
    let o = bo(&w, b);
    assert_eq!((o.action, o.unk_196, o.unk_198, o.unk_19a), (Action::Circle, 350, 80, -0x200));
}

#[test]
fn circling_he_swims_round_the_rim_and_turns_the_platform_against_him() {
    let Some(a) = assets() else { return };
    let (mut w, p, b) = met(&a);
    let s: i16 = 0x1234;
    // Link halfway between the middle and him (his frame's x 131.5, z -20: 133 off the middle),
    // 300 off, facing him: (133 - 180) × 8/15 = -25, + 0x130: 279 his way.
    place(&mut w, b, s, 1);
    link_along(&mut w, b, 131.5, 300.0, s);
    ob(&mut w, p).actor.world_rot.y = 0;
    let (splashes, ripples) = (effects(&w, EFFECT_SS_G_SPLASH), effects(&w, EFFECT_SS_G_RIPPLE));
    w.gameplay_frames = 2;
    camera_frame(&mut w);
    update(&mut w, b);
    let o = bo(&w, b);
    let s1 = s.wrapping_add(279);
    assert_eq!((o.actor.shape_rot.y, o.unk_196, o.unk_198), (s1, 350, 80));
    let wy = s1.wrapping_sub(0x4000);
    let home = o.actor.home_pos;
    assert_eq!(o.actor.world_rot.y, wy);
    assert_eq!((o.actor.world_pos.x, o.actor.world_pos.z), (sin_s(wy) * RIM + home.x, cos_s(wy) * RIM + home.z));
    // func_809BE568: the sphere 30 behind, 45 up; the cylinders (±30, 12) in his frame.
    let (sn, cs) = (sin_s(s1), cos_s(s1));
    let wp = o.actor.world_pos;
    let c = o.collider.elements[0].dim.world_sphere.center;
    assert_eq!(c, [(wp.x + -30.0 * sn) as i16, (wp.y + 45.0) as i16, (wp.z + -30.0 * cs) as i16]);
    assert_eq!(o.collider_cylinders[0].dim.pos, [(wp.x + 12.0 * sn + 30.0 * cs) as i16, wp.y as i16, (wp.z + 12.0 * cs - 30.0 * sn) as i16]);
    assert_eq!(o.collider_cylinders[1].dim.pos, [(wp.x + 12.0 * sn - 30.0 * cs) as i16, wp.y as i16, (wp.z + 12.0 * cs + 30.0 * sn) as i16]);
    // The wake (frame bit 1: a ripple too) and NA_SE_EN_DAIOCTA_SPLASH; the platform's
    // world.rot.y to -0x200 by 0x10 (scaled); the camera asked for CAM_SET_BIG_OCTO.
    assert_eq!((effects(&w, EFFECT_SS_G_SPLASH), effects(&w, EFFECT_SS_G_RIPPLE)), (splashes + 1, ripples + 1));
    assert_eq!(o.actor.sfx, NA_SE_EN_DAIOCTA_SPLASH - SFX_FLAG);
    let mut want = 0i16;
    scaled_step_to_s(&mut want, -0x200, 0x10);
    assert_eq!(ob(&mut w, p).actor.world_rot.y, want);
    assert_eq!(camera(&w), (CAM_SET_BIG_OCTO, 0));
    // Frame 7: the wake's ripple (bit 1) and func_809BCEBC's (at the surface); frame 1 neither.
    for (frames, more) in [(7u32, 2), (1, 0)] {
        let ripples = effects(&w, EFFECT_SS_G_RIPPLE);
        w.gameplay_frames = frames;
        update(&mut w, b);
        assert_eq!(effects(&w, EFFECT_SS_G_RIPPLE), ripples + more, "frame {frames}");
    }
    // Link 610 out beyond him (his x -347, off his side of the platform): 229 - 0x80 + 0x130 =
    // 405, his count down; 347 off: the sink count back to 80.
    place(&mut w, b, s, 1);
    bo_mut(&mut w, b).unk_198 = 50;
    link_along(&mut w, b, 610.0, 347.0, s);
    update(&mut w, b);
    assert_eq!((bo(&w, b).actor.shape_rot.y, bo(&w, b).unk_196, bo(&w, b).unk_198), (s.wrapping_add(405), 349, 80));
    // Link near (150) behind him: the sink count down, 0x20 faster: -25 + 0x20 + 0x130 = 311.
    place(&mut w, b, s, 1);
    link_along(&mut w, b, 131.5, 150.0, s.wrapping_add(-0x8000i16));
    update(&mut w, b);
    assert_eq!((bo(&w, b).actor.shape_rot.y, bo(&w, b).unk_196, bo(&w, b).unk_198), (s.wrapping_add(311), 350, 79));
    // On his last health a tenth faster: (-25 + 0x130) × 1.1 = 306; the other way round, minus.
    place(&mut w, b, s, -1);
    bo_mut(&mut w, b).actor.col_chk_info.health = 1;
    link_along(&mut w, b, 131.5, 300.0, s);
    update(&mut w, b);
    assert_eq!(bo(&w, b).actor.shape_rot.y, s.wrapping_sub(306));
    assert_eq!(bo(&w, b).unk_19a, -0x200);
}

#[test]
fn his_count_out_he_turns_about_and_circles_the_other_way() {
    let Some(a) = assets() else { return };
    let (mut w, p, b) = met(&a);
    let s: i16 = -0x3000;
    place(&mut w, b, s, 1);
    bo_mut(&mut w, b).unk_196 = 1;
    // Off his side: the count to 0: func_809BD4A4 (to his back's yaw, kept in world.rot.x), the
    // platform 0x200 his way, his back closed.
    link_along(&mut w, b, 610.0, 347.0, s);
    update(&mut w, b);
    let o = bo(&w, b);
    let s1 = s.wrapping_add(405);
    assert_eq!((o.action, o.unk_195, o.unk_196, o.unk_19a), (Action::TurnAround, true, 0, 0x200));
    assert_eq!(o.actor.world_rot.x, s1.wrapping_add(-0x8000i16));
    assert_eq!(o.collider.base.ac_flags & AC_ON, 0);
    assert!(o.skel.is(ANIM_FLOAT));
    // func_809BDFC8: 0x800 a frame (scaled) to his back; there the other way round, circling.
    for _ in 0..40 {
        update(&mut w, b);
        if bo(&w, b).action != Action::TurnAround {
            break;
        }
    }
    let o = bo(&w, b);
    assert_eq!((o.action, o.unk_194, o.unk_19a, o.unk_196), (Action::Circle, -1, 0x200, 350));
    let _ = p;
}

#[test]
fn his_front_knocks_link_away_and_turns_him_about() {
    let Some(a) = assets() else { return };
    let (mut w, _p, b) = met(&a);
    let s: i16 = 0x2000;
    place(&mut w, b, s, 1);
    link_along(&mut w, b, 131.5, 300.0, s);
    // func_809BE798: a cylinder's AT landed, Link 0x5000 off his course (> 0x4000): knocked away at
    // his course + 0x4000, 10, 5 up (Actor_SetPlayerKnockbackLargeNoDamage); turning about, 40
    // frames (39 after this update's own).
    let course = bo(&w, b).actor.world_rot.y;
    bo_mut(&mut w, b).actor.yaw_towards_player = course.wrapping_add(0x5000);
    bo_mut(&mut w, b).collider_cylinders[1].base.at_flags |= AT_HIT;
    update(&mut w, b);
    let o = bo(&w, b);
    assert_eq!((o.action, o.unk_196), (Action::TurnAround, 39));
    assert_eq!(o.collider_cylinders[1].base.at_flags & AT_HIT, 0);
    let pl = w.player();
    assert_eq!((pl.knockback_type, pl.knockback_damage, pl.knockback_rot, pl.knockback_speed, pl.knockback_y_velocity), (2, 0, course.wrapping_add(0x4000), 10.0, 5.0));
    // His voice at 20.
    for _ in 0..19 {
        update(&mut w, b);
    }
    assert_eq!((bo(&w, b).unk_196, sfx(&w, b, NA_SE_EN_DAIOCTA_VOICE)), (20, 1));
    // Spinning, knocked towards his way round (0 < yawDiff: 0x6000): his way flips, 4 more frames.
    {
        let o = bo_mut(&mut w, b);
        o.action = Action::Spin;
        o.unk_194 = 1;
        o.unk_196 = 10;
        o.actor.yaw_towards_player = o.actor.world_rot.y.wrapping_add(0x100);
        o.collider.base.at_flags |= AT_HIT;
    }
    update(&mut w, b);
    assert_eq!((bo(&w, b).unk_194, bo(&w, b).unk_196), (-1, 13));
}

#[test]
fn the_damage_table_only_his_back_is_hurt_nuts_stun_him_bombs_throw_him() {
    let Some(a) = assets() else { return };
    let (mut w, _p, b) = met(&a);
    // sDamageTable: (damage, reaction).
    let t = &S_DAMAGE_TABLE.table;
    let e = |i: usize| (t[i] & 0xF, t[i] >> 4);
    assert_eq!([e(0), e(1), e(2), e(3), e(4), e(8), e(9), e(22), e(25), e(27)], [(0, 1), (2, 0), (0, 0), (0, 0xF), (0, 1), (1, 0), (0, 0), (1, 0), (2, 0), (4, 0)]);
    // His front's cylinders are hard (AC_HARD: Link's blows bounce off), his back's sphere not.
    let o = bo(&w, b);
    assert_ne!(o.collider_cylinders[0].base.ac_flags & AC_HARD, 0);
    assert_eq!(o.collider.base.ac_flags & AC_HARD, 0);
    assert_eq!((o.collider_cylinders[0].info.ac_dmg_info.dmg_flags, o.collider.elements[0].info.ac_dmg_info.dmg_flags), (0xFFCF_FFE7, 0xFFCF_FFFF));
    let s: i16 = 0x4000;
    // The Kokiri Sword (1) with Link before him (within 0x4000 of his face): nothing.
    place(&mut w, b, s, 1);
    link_along(&mut w, b, 131.5, 300.0, s.wrapping_add(0x3000));
    hit(&mut w, b, 1, 0, DMG_SLASH_KOKIRI);
    update(&mut w, b);
    assert_eq!((bo(&w, b).action, bo(&w, b).actor.col_chk_info.health), (Action::Circle, 4));
    assert_eq!(bo(&w, b).collider.base.ac_flags & AC_HIT, 0);
    // The Master Sword (0, 0) from behind: nothing.
    place(&mut w, b, s, 1);
    link_along(&mut w, b, 131.5, 300.0, s.wrapping_add(-0x8000i16));
    hit(&mut w, b, 0, 0, DMG_SLASH_MASTER);
    update(&mut w, b);
    assert_eq!((bo(&w, b).action, bo(&w, b).actor.col_chk_info.health), (Action::Circle, 4));
    // The Kokiri Sword from behind: 3 left, NA_SE_EN_DAIOCTA_DAMAGE, hurt: red 24 frames, his
    // front harmless, the platform still; the count 23 after this update.
    place(&mut w, b, s, 1);
    hit(&mut w, b, 1, 0, DMG_SLASH_KOKIRI);
    update(&mut w, b);
    let o = bo(&w, b);
    assert_eq!((o.action, o.actor.col_chk_info.health, o.unk_196, o.unk_19a), (Action::Hurt, 3, 23, 0));
    assert!(o.skel.is(ANIM_HURT));
    assert_eq!(o.collider_cylinders[0].base.at_flags & AT_ON, 0);
    assert_eq!((o.actor.color_filter_params & COLORFILTER_COLORFLAG_RED, o.actor.color_filter_timer), (COLORFILTER_COLORFLAG_RED, 24));
    assert_eq!(sfx(&w, b, NA_SE_EN_DAIOCTA_DAMAGE), 1);
    // A Deku nut (reaction 1, DMG_DEKU_NUT): stunned 20 frames, a short spin after, blue as long,
    // NA_SE_EN_DAIOCTA_MAHI; before him or behind.
    place(&mut w, b, s, 1);
    link_along(&mut w, b, 131.5, 300.0, s);
    hit(&mut w, b, 0, 1, DMG_DEKU_NUT);
    update(&mut w, b);
    let o = bo(&w, b);
    assert_eq!((o.action, o.unk_195, o.unk_196, o.unk_19a), (Action::Stunned, true, 19, 0));
    assert!(o.skel.is(ANIM_STUN));
    assert_eq!((o.actor.color_filter_params & (COLORFILTER_COLORFLAG_RED | COLORFILTER_COLORFLAG_GRAY), o.actor.color_filter_timer), (COLORFILTER_COLORFLAG_BLUE, 20));
    assert_eq!(sfx(&w, b, NA_SE_EN_DAIOCTA_MAHI), 1);
    // Stunned, another reaction 1 does nothing.
    hit(&mut w, b, 0, 1, DMG_BOOMERANG);
    update(&mut w, b);
    assert_eq!((bo(&w, b).unk_196, bo(&w, b).unk_195), (18, true));
    // The boomerang: 80 frames, the long spin.
    place(&mut w, b, s, 1);
    hit(&mut w, b, 0, 1, DMG_BOOMERANG);
    update(&mut w, b);
    assert_eq!((bo(&w, b).action, bo(&w, b).unk_195, bo(&w, b).unk_196), (Action::Stunned, false, 79));
    // A bomb (reaction 0xF): thrown up 16 frames, his back closed, the colour filter cut.
    place(&mut w, b, s, 1);
    bo_mut(&mut w, b).actor.color_filter_timer = 5;
    hit(&mut w, b, 0, 0xF, DMG_EXPLOSIVE);
    update(&mut w, b);
    let o = bo(&w, b);
    assert_eq!((o.action, o.unk_196, o.actor.color_filter_timer), (Action::Bounce, 15, 0));
    assert_eq!(o.collider.base.ac_flags & AC_ON, 0);
}

#[test]
fn stunned_he_pushes_link_then_spins_out_of_it() {
    let Some(a) = assets() else { return };
    let (mut w, _p, b) = met(&a);
    let s: i16 = 0x1000;
    place(&mut w, b, s, 1);
    // A nut with Link 0x6000 off his face (`ABS(shape.rot.y - yawTowardsPlayer)` 0x6000): spinning
    // 4 frames after.
    link_along(&mut w, b, 131.5, 300.0, s.wrapping_add(0x6000));
    hit(&mut w, b, 0, 1, DMG_DEKU_NUT);
    update(&mut w, b);
    assert_eq!((bo(&w, b).action, bo(&w, b).unk_196), (Action::Stunned, 19));
    // func_809BE058: touching his back (its OC), Link (speed 0: 1) pushed back along his yaw;
    // only a cylinder: towards him.
    let ph = w.player.unwrap();
    w.actors.actor_mut(ph).unwrap().speed_xz = 0.0;
    for (sphere, sign) in [(true, 1.0f32), (false, -1.0)] {
        let before = w.actors.actor(ph).unwrap().world_pos;
        {
            let o = bo_mut(&mut w, b);
            if sphere {
                o.collider.base.oc_flags1 |= OC1_HIT;
            } else {
                o.collider.base.oc_flags1 &= !OC1_HIT;
                o.collider_cylinders[0].base.oc_flags1 |= OC1_HIT;
            }
        }
        update(&mut w, b);
        let after = w.actors.actor(ph).unwrap().world_pos;
        let r = bo(&w, b).actor.shape_rot.y;
        assert_eq!((after.x, after.z), (before.x - sign * sin_s(r), before.z - sign * cos_s(r)), "sphere {sphere}");
        assert_eq!(bo(&w, b).collider_cylinders[0].base.oc_flags1 & OC1_HIT, 0);
    }
    bo_mut(&mut w, b).collider.base.oc_flags1 &= !OC1_HIT;
    // 20 frames, then func_809BD6B8: 4 frames' spin, the platform still, his back closed.
    while bo(&w, b).action == Action::Stunned {
        update(&mut w, b);
    }
    let o = bo(&w, b);
    assert_eq!((o.action, o.unk_196, o.unk_19a, o.actor.shape_rot.y), (Action::Spin, 4, 0, s));
    assert_eq!(o.collider.base.ac_flags & AC_ON, 0);
    // func_809BE3E4: 0x2000 a frame; out (the 4th), his way round the way he faces off his course
    // ((s16)(shape - world) = 0x8000 + 0x4000: negative: -1), circling, no dizziness after a nut.
    for _ in 0..3 {
        update(&mut w, b);
        assert_eq!(bo(&w, b).action, Action::Spin);
    }
    update(&mut w, b);
    assert_eq!(bo(&w, b).actor.shape_rot.y, s.wrapping_add(-0x8000i16));
    let o = bo(&w, b);
    assert_eq!((o.action, o.unk_194, o.unk_196), (Action::Circle, -1, 350));
    // After anything else (unk_195 false) dizzy: 40 frames of 0x200 his way, then 350.
    {
        let o = bo_mut(&mut w, b);
        o.action = Action::Spin;
        o.unk_195 = false;
        o.unk_196 = 1;
    }
    update(&mut w, b);
    assert_eq!((bo(&w, b).action, bo(&w, b).unk_196), (Action::Circle, -40));
    let r0 = bo(&w, b).actor.shape_rot.y;
    let dir = bo(&w, b).unk_194 as i32;
    for _ in 0..40 {
        update(&mut w, b);
    }
    assert_eq!(bo(&w, b).unk_196, 350);
    assert_eq!(bo(&w, b).actor.shape_rot.y, (r0 as i32 + 40 * dir * 0x200) as i16);
}

#[test]
fn a_bomb_throws_him_up_and_he_lands_with_a_splash() {
    let Some(a) = assets() else { return };
    let (mut w, _p, b) = met(&a);
    place(&mut w, b, 0, 1);
    hit(&mut w, b, 0, 0xF, DMG_EXPLOSIVE);
    update(&mut w, b);
    // func_809BDF34: home + sinf(t π/16) × 100.
    let home = bo(&w, b).actor.home_pos;
    assert_eq!(bo(&w, b).actor.world_pos.y, ((15.0 * (std::f32::consts::PI / 16.0)).sin() * 100.0) + home.y);
    let splashes = effects(&w, EFFECT_SS_G_SPLASH);
    for _ in 0..15 {
        update(&mut w, b);
    }
    // func_809BD1C8: four splashes 60 out, the landing's sounds; circling.
    let o = bo(&w, b);
    assert_eq!((o.action, o.actor.world_pos.y), (Action::Circle, home.y));
    assert_eq!(effects(&w, EFFECT_SS_G_SPLASH), splashes + 4);
    assert_eq!((sfx(&w, b, NA_SE_EN_DAIOCTA_LAND_WATER), sfx(&w, b, NA_SE_EN_GOLON_LAND_BIG)), (1, 1));
}

#[test]
fn hurt_he_flies_up_spinning_lands_and_turns_about() {
    let Some(a) = assets() else { return };
    let (mut w, _p, b) = met(&a);
    let s: i16 = 0x4000;
    place(&mut w, b, s, 1);
    link_along(&mut w, b, 131.5, 300.0, s.wrapping_add(-0x8000i16));
    hit(&mut w, b, 2, 0, DMG_DEKU_STICK);
    update(&mut w, b);
    // func_809BE180: to 8, 0x200 a frame his way and up sinf((t - 8) π/16) × 100; at 8 landing.
    let home = bo(&w, b).actor.home_pos;
    let splashes = effects(&w, EFFECT_SS_G_SPLASH);
    for t in (1..=22i16).rev() {
        update(&mut w, b);
        let o = bo(&w, b);
        assert_eq!(o.unk_196, t);
        if t >= 8 {
            assert_eq!(o.actor.world_pos.y, ((((t - 8) as f32) * (std::f32::consts::PI / 16.0)).sin() * 100.0) + home.y, "t {t}");
        }
    }
    assert_eq!(bo(&w, b).actor.shape_rot.y, s.wrapping_add(16 * 0x200));
    assert_eq!(effects(&w, EFFECT_SS_G_SPLASH) - splashes, 4);
    // Health left (2): turning about.
    update(&mut w, b);
    assert_eq!((bo(&w, b).action, bo(&w, b).actor.col_chk_info.health), (Action::TurnAround, 2));
}

#[test]
fn his_last_hit_kills_him_the_room_is_cleared_and_the_platform_sinks() {
    let Some(a) = assets() else { return };
    let (mut w, p, b) = met(&a);
    let s: i16 = 0x4000;
    place(&mut w, b, s, 1);
    bo_mut(&mut w, b).actor.col_chk_info.health = 1;
    link_along(&mut w, b, 131.5, 300.0, s.wrapping_add(-0x8000i16));
    hit(&mut w, b, 1, 0, DMG_SLASH_KOKIRI);
    w.actors.freeze_flash_timer = 0;
    update(&mut w, b);
    // No health: NA_SE_EN_DAIOCTA_DEAD, the finishing blow; hurt, then (the count out) dying.
    assert_eq!((bo(&w, b).action, bo(&w, b).actor.col_chk_info.health), (Action::Hurt, 0));
    assert_eq!(sfx(&w, b, NA_SE_EN_DAIOCTA_DEAD), 1);
    assert_eq!(w.actors.freeze_flash_timer, 5);
    while bo(&w, b).action == Action::Hurt {
        update(&mut w, b);
    }
    // func_809BD658: object_bigokuta_Anim_000A74, NA_SE_EN_DAIOCTA_DEAD2, 38 and 10.
    let o = bo(&w, b);
    assert_eq!((o.action, o.unk_196, o.unk_198), (Action::Die, 38, 10));
    assert!(o.skel.is(ANIM_DIE));
    assert_eq!(sfx(&w, b, NA_SE_EN_DAIOCTA_DEAD2), 1);
    // func_809BE26C: 38 frames, 0x2000 a frame to 10; limb 10's env count × 255/38.
    let r0 = o.actor.shape_rot.y;
    for t in (0..38i16).rev() {
        update(&mut w, b);
        let o = bo(&w, b);
        assert_eq!(o.unk_196, t);
        assert_eq!(o.limb_envs(), ((t as f32 * (255.0f32 / 38.0)) as u8, None));
    }
    assert_eq!(bo(&w, b).actor.shape_rot.y, r0.wrapping_add((28 * 0x2000) as i16));
    // The animation, then 10 frames (SkelAnime_Update done): at 6 a puff 150 up and
    // NA_SE_EN_OCTAROCK_DEAD2; limb 17 swelling.
    let dust = effects(&w, EFFECT_SS_DUST);
    let mut n = 0;
    while bo(&w, b).unk_198 != 0 {
        update(&mut w, b);
        n += 1;
        assert!(n < 200);
        if bo(&w, b).unk_198 == 6 {
            assert_eq!(effects(&w, EFFECT_SS_DUST), dust + 1);
            assert_eq!(sfx(&w, b, NA_SE_EN_OCTAROCK_DEAD2), 1);
        }
    }
    assert_eq!(bo(&w, b).swell_scale(), Some(Vec3::new(1.0, 1.0, 1.0)));
    assert!(bo(&w, b).drawn_xlu());
    // Then scale.y 0.001 a frame to nothing: the room cleared, CAM_SET_DUNGEON0 with the bg's
    // settings, NA_SE_EN_OCTAROCK_BUBLE where he was, gone. The same update's own
    // CAM_SET_BIG_OCTO (priority 5, as DUNGEON0's) is turned down, and CAM_STATE_CHECK_BG ends off.
    let bubles = sfx_any(&w, NA_SE_EN_OCTAROCK_BUBLE);
    let mut n = 0;
    while !gone(&w, b) {
        assert!(!w.flags.get_clear(6));
        camera_frame(&mut w);
        update(&mut w, b);
        n += 1;
        assert!(n < 40);
    }
    // 0.033 down 0.001 a frame: 33 steps, the first in the update that ended the count.
    assert_eq!(n, 32);
    assert!(w.flags.get_clear(6));
    assert_eq!(camera(&w), (CAM_SET_DUNGEON0, 0));
    assert!(sfx_any(&w, NA_SE_EN_OCTAROCK_BUBLE) > bubles);
    // The platform: the room cleared, it sinks.
    update(&mut w, p);
    assert_eq!(ob(&mut w, p).action, objects::Action::SinkToFloorHeight);
}

#[test]
fn link_near_behind_him_he_sinks_and_rises_across_the_platform() {
    let Some(a) = assets() else { return };
    let (mut w, _p, b) = met(&a);
    let s: i16 = 0x0800;
    place(&mut w, b, s, 1);
    bo_mut(&mut w, b).unk_198 = 1;
    link_along(&mut w, b, 131.5, 150.0, s.wrapping_add(-0x8000i16));
    update(&mut w, b);
    // func_809BD768: no lock-on, his front harmless, NA_SE_EN_DAIOCTA_SINK, the platform still.
    let o = bo(&w, b);
    assert_eq!((o.action, o.unk_19a), (Action::Sink, 0));
    assert_eq!(o.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!(o.collider_cylinders[0].base.at_flags & AT_ON, 0);
    assert_eq!(sfx(&w, b, NA_SE_EN_DAIOCTA_SINK), 1);
    // func_809BE4A4: 10 a frame, 0x2000 spin; under 200 (21 frames), func_809BD7F0: on the rim
    // across the middle from Link (the yaw from him to the middle).
    let home = o.actor.home_pos;
    for _ in 0..20 {
        update(&mut w, b);
        assert_eq!(bo(&w, b).action, Action::Sink);
    }
    update(&mut w, b);
    let o = bo(&w, b);
    assert_eq!((o.action, o.actor.world_pos.y), (Action::Rise, home.y - 210.0));
    let link = w.actors.actor(w.player.unwrap()).unwrap().world_pos;
    let yaw = vec3f_yaw(link, home);
    assert_eq!(o.actor.world_rot.y, yaw);
    assert_eq!((o.actor.world_pos.x, o.actor.world_pos.z), (sin_s(yaw) * RIM + home.x, cos_s(yaw) * RIM + home.z));
    // func_809BE518: 10 a frame to the surface: lock-on, circling.
    for _ in 0..21 {
        update(&mut w, b);
    }
    let o = bo(&w, b);
    assert_eq!((o.action, o.actor.world_pos.y), (Action::Circle, home.y));
    assert_ne!(o.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
}

#[test]
fn drawn_in_three_bakes_split_at_the_overrides_env_colours_or_translucent_at_the_end() {
    let Some(a) = assets() else { return };
    let (mut w, _p, b) = met(&a);
    let view = ViewInfo { eye: Vec3::ZERO, billboard: glam::Mat4::IDENTITY };
    let draws = |w: &PlayState| {
        let o = bo(w, b);
        let mut out = DrawOut::default();
        o.draw(&o.render_state(), w, &view, &mut out);
        out
    };
    let key = |s: &str| eng_gfx::MeshKey::named(oot_game::pack::keys::bake(s));
    let env = |c: &eng_gfx::DrawCmd| c.params.segments.as_ref().unwrap().env[0x0E];
    // Circling: the draw's white, limb 10's 255 (and on to limbs 16-19).
    let out = draws(&w);
    assert!(out.xlu.is_empty());
    assert_eq!(out.opa.iter().map(|c| (c.mesh.clone(), env(c))).collect::<Vec<_>>(), vec![(key(BAKE_FRONT), Some([255; 4])), (key(BAKE_BODY), Some([255; 4])), (key(BAKE_BACK), Some([255; 4]))]);
    // Stunned at count 2 (of a 12-frame cycle): limb 15's sin(2 π/16) × 125 + 130 on limbs 16-19.
    {
        let o = bo_mut(&mut w, b);
        o.action = Action::Stunned;
        o.unk_196 = 2;
    }
    let v = (((2.0 * (std::f32::consts::PI / 16.0)).sin() * 125.0) + 130.0) as u8;
    assert_eq!(env(&draws(&w).opa[2]), Some([v, v, v, 255]));
    // Hurt at an odd count of 8 or more: the squash (two Rand_ZeroOne) drawn; at 7 not.
    {
        let o = bo_mut(&mut w, b);
        o.action = Action::Hurt;
        o.unk_196 = 9;
    }
    draw_update(&mut w, b);
    assert!(bo(&w, b).squash.is_some());
    bo_mut(&mut w, b).unk_196 = 7;
    draw_update(&mut w, b);
    assert!(bo(&w, b).squash.is_none());
    // Dying, both counts out: translucent, env (0, 0, 0, scale.y × 255/0.033).
    {
        let o = bo_mut(&mut w, b);
        o.action = Action::Die;
        o.unk_196 = 0;
        o.unk_198 = 0;
        o.actor.scale.y = 0.0165;
    }
    let out = draws(&w);
    assert!(out.opa.is_empty());
    assert_eq!(out.xlu.len(), 1);
    assert_eq!((out.xlu[0].mesh.clone(), env(&out.xlu[0])), (key(BAKE_XLU), Some([0, 0, 0, (0.0165f32 * (255.0f32 / 0.033)) as u8])));
    // The bakes: the three opaque ones the whole skeleton between them, nothing unresolved; limb
    // 10's and limb 16's lists take the env colour in their colour (ENV × TEXEL0).
    let get = |s: &str| -> eng_gfx::DrawList { a.pack.assets.get(&oot_game::pack::keys::bake(s)).unwrap_or_else(|_| panic!("{s} baked")) };
    let (f, m, k, x) = (get(BAKE_FRONT), get(BAKE_BODY), get(BAKE_BACK), get(BAKE_XLU));
    for d in [&f, &m, &k, &x] {
        assert!(d.stats.unresolved_addresses.is_empty());
        assert!(d.triangle_count() > 0);
    }
    assert_eq!(f.triangle_count() + m.triangle_count() + k.triangle_count(), x.triangle_count());
    assert!(m.materials.iter().any(|mat| mat.env_dyn == Some(0x0E)));
    assert!(k.materials.iter().any(|mat| mat.env_dyn == Some(0x0E)));
    assert_eq!(bakes().len(), 4);
}
