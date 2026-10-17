//! `Demo_Go` (GAME-09 milestone 4) against the C (`z_demo_go.c`): each Goron's init and cue
//! channel, the fall on cue 2 (the lerp, the sound for the first two), the landing (dust, the
//! thud), cue 3's animation and the walk (the turn, on the spot for 8 frames, then 1.2 a frame,
//! the footsteps), the turn's long way round, the draw (`En_Go2`'s bakes).
//!
//! The cues are put on the cutscene's channels by hand (`play->csCtx.actorCues`), each Goron
//! spawned on Death Mountain Trail's normal layer and updated alone, as `Actor_UpdateAll` would.
//! Expected values are worked out from the C in the comments. The Ruby's cutscene run whole is in
//! `tests/demo_du.rs`.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::{IVec3, Vec3};
use oot_actors::PlayExt;
use oot_actors::demo_go::*;
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::audio::sfx::NA_SE_EN_DODO_M_GND;
use oot_game::cutscene::CsCmdActorCue;
use oot_game::effect::EFFECT_SS_DUST;
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

/// `CS_STATE_RUN` (`cutscene.h`).
const CS_STATE_RUN: u8 = 2;
/// `OBJECT_OF1D_MAP` (`object_table.h`: 0x00C9).
const OBJECT_OF1D_MAP: i16 = 0x00C9;
/// Link by the cavern's mouth, where the cutscene holds him (its Player cues' position).
const LINK_AT: Vec3 = Vec3::new(-1656.0, 688.0, -519.0);

/// Death Mountain Trail (`ENTR_DEATH_MOUNTAIN_TRAIL_0`, its normal layer) after the cavern, Link by
/// the cavern's mouth, the Gorons' object in, the sound log on.
fn trail(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DEATH_MOUNTAIN_TRAIL_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("cavern-boss").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 3);
    w.place_player(LINK_AT, 0);
    idle(&mut w, 1);
    if w.object_ctx.get_index(OBJECT_OF1D_MAP).is_none() {
        w.object_ctx.spawn(OBJECT_OF1D_MAP);
    }
    w
}

fn spawn(w: &mut PlayState, params: i16, pos: Vec3) -> ActorHandle {
    w.actor_spawn(ACTOR_DEMO_GO, pos, [0; 3], params).expect("Demo_Go spawned")
}

fn go(w: &PlayState, h: ActorHandle) -> &DemoGo {
    w.actors.downcast::<DemoGo>(h).expect("a Demo_Go")
}

/// The actor's update alone (`Actor_UpdateAll`'s call), as the updating actor.
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// A cue on `channel`, from `start` to `end` over the frames `from..to`, turned `yaw`, the
/// cutscene running at frame `frame`.
#[allow(clippy::too_many_arguments)]
fn cue(w: &mut PlayState, channel: usize, action: u16, from: u16, to: u16, yaw: i16, start: IVec3, end: IVec3, frame: u16) {
    w.cs_ctx.state = CS_STATE_RUN;
    w.cs_ctx.frames = frame;
    w.cs_ctx.npc_actions[channel] = Some(CsCmdActorCue { action, start_frame: from, end_frame: to, rot: [0, yaw, 0], start_pos: start, end_pos: end, normal: IVec3::ZERO });
}

fn sfx_count(w: &PlayState) -> usize {
    w.audio.log.as_ref().unwrap().sfx.len()
}

/// The sounds asked for since the log held `n`.
fn sfx_since(w: &PlayState, n: usize) -> Vec<u16> {
    w.audio.log.as_ref().unwrap().sfx[n..].iter().map(|&(_, s, _)| s).collect()
}

fn dust(w: &PlayState) -> usize {
    w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == EFFECT_SS_DUST).count()
}

#[test]
fn each_goron_starts_hidden_with_its_cue_channel_and_animation_set() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    for (params, channel) in [(0, 3), (1, 4), (2, 5), (7, 5)] {
        let h = spawn(&mut w, params, Vec3::new(-1808.0, 684.0, -544.0));
        let g = go(&w, h);
        // DemoGo_Init: Actor_Init's 0.01 (no Actor_SetScale), ActorShape_Init (yOffset 0, the
        // circle shadow 30), gGoronAnim_004930 once from 0, action 0, drawConfig 0 (nothing).
        assert_eq!((g.actor.scale, g.actor.shape_y_offset), (Vec3::splat(0.01), 0.0));
        assert!(g.skel.is(ANIM_004930));
        assert_eq!((g.skel.mode, g.skel.cur_frame, g.skel.play_speed), (oot_game::skelanime_std::ANIMMODE_ONCE, 0.0, 1.0));
        assert_eq!((g.action, g.draw_config, g.unk_190), (0, 0, 0));
        // DemoGo_GetCueChannel: 3, 4, else 5.
        assert_eq!(g.get_cue_channel(), channel, "params {params}");
        // func_8097CFDC: straight to action 1 (func_8097CE20, waiting for cue 2).
        update(&mut w, h);
        assert_eq!(go(&w, h).action, 1);
    }
}

#[test]
fn cue_2_shows_it_falling_along_the_cue_and_the_first_two_are_heard() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    // gDMTGoronRubyCs' first Goron: cue 2 over 1270..1286 from (-1546, 1072, -591) to
    // (-1546, 691, -591), yaw -10013.
    let (start, end) = (IVec3::new(-1546, 1072, -591), IVec3::new(-1546, 691, -591));
    for (params, channel, heard) in [(0, 3, true), (1, 4, true), (2, 5, false)] {
        let h = spawn(&mut w, params, Vec3::new(-1808.0, 684.0, -544.0));
        update(&mut w, h);
        // Cue 1 holds it in action 1.
        cue(&mut w, channel, 1, 0, 1270, -10013, start, start, 1000);
        update(&mut w, h);
        assert_eq!((go(&w, h).action, go(&w, h).draw_config), (1, 0));
        // Cue 2 at frame 1274: action 2, drawn; Environment_LerpWeight(1286, 1270, 1274) = 0.25:
        // y = (691 - 1072) × 0.25 + 1072 = 976.75; turned to the cue's yaw. func_8097C8A8 for
        // params 0 and 1: NA_SE_EV_OBJECT_FALL from a fixed point.
        let n = sfx_count(&w);
        cue(&mut w, channel, 2, 1270, 1286, -10013, start, end, 1274);
        update(&mut w, h);
        let g = go(&w, h);
        assert_eq!((g.action, g.draw_config), (2, 1));
        assert_eq!(g.actor.world_pos, Vec3::new(-1546.0, 976.75, -591.0));
        assert_eq!((g.actor.world_rot.y, g.actor.shape_rot.y), (-10013, -10013));
        assert_eq!(sfx_since(&w, n).contains(&NA_SE_EV_OBJECT_FALL), heard, "params {params}");
        w.actors.remove(h);
    }
}

#[test]
fn it_lands_at_the_cues_end_in_dust_with_a_thud_then_waits_for_cue_3() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    let (start, end) = (IVec3::new(-1731, 1074, -574), IVec3::new(-1731, 692, -574));
    let h = spawn(&mut w, 1, Vec3::new(-1707.0, 693.0, -544.0));
    update(&mut w, h);
    cue(&mut w, 4, 2, 1250, 1272, 9102, start, end, 1260);
    update(&mut w, h);
    assert_eq!(go(&w, h).action, 2);
    // Action 2 (func_8097D01C) before the end frame: still falling.
    cue(&mut w, 4, 2, 1250, 1272, 9102, start, end, 1271);
    update(&mut w, h);
    assert_eq!(go(&w, h).action, 2);
    // At 1272 (curFrame >= endFrame): at the cue's end; func_8097CA78: func_80033480 with 10 + 1
    // puffs, NA_SE_EN_DODO_M_GND; action 3.
    let (n, d) = (sfx_count(&w), dust(&w));
    cue(&mut w, 4, 2, 1250, 1272, 9102, start, end, 1272);
    update(&mut w, h);
    let g = go(&w, h);
    assert_eq!(g.action, 3);
    assert_eq!((g.actor.world_pos.x, g.actor.world_pos.z), (-1731.0, -574.0));
    assert!(g.actor.world_pos.y >= 692.0);
    assert_eq!(dust(&w) - d, 11);
    assert!(sfx_since(&w, n).contains(&NA_SE_EN_DODO_M_GND));
    // The floor found (Actor_UpdateBgCheckInfo): the circle shadow drawn with it.
    assert!(g.actor.floor_poly.is_some());
    // Still cue 2 (the hold, 1272..1296): action 3 stays; cue 3: action 4.
    cue(&mut w, 4, 2, 1272, 1296, 9102, end, end, 1280);
    update(&mut w, h);
    assert_eq!(go(&w, h).action, 3);
    cue(&mut w, 4, 3, 1296, 1300, 9102, end, end, 1296);
    update(&mut w, h);
    assert_eq!(go(&w, h).action, 4);
}

#[test]
fn after_its_animation_it_turns_on_the_spot_for_8_frames_then_walks_at_1_2() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    let p = IVec3::new(-1546, 691, -591);
    let h = spawn(&mut w, 0, Vec3::new(-1808.0, 684.0, -544.0));
    update(&mut w, h);
    cue(&mut w, 3, 2, 1270, 1286, -10013, p, p, 1286);
    update(&mut w, h);
    update(&mut w, h);
    cue(&mut w, 3, 3, 1296, 1321, -10013, p, p, 1296);
    update(&mut w, h);
    assert_eq!(go(&w, h).action, 4);
    // Action 4 (func_8097D088): gGoronAnim_004930 once, blinking; finished, the walk
    // (gGoronAnim_0029A8, looped, morph -8), action 5, unk_19C 0.
    let last = go(&w, h).skel.end_frame;
    let mut frames = 0;
    while go(&w, h).action == 4 {
        update(&mut w, h);
        frames += 1;
        assert!(frames < 400);
    }
    // SkelAnime_Update's once: done the update its frame reaches the end.
    assert_eq!(frames as f32, last + 1.0);
    let g = go(&w, h);
    assert!(g.skel.is(ANIM_0029A8));
    assert_eq!((g.action, g.unk_19c, g.skel.mode), (5, 0.0, oot_game::skelanime_std::ANIMMODE_LOOP));
    // The walk's cue (4) turned to yaw 0 from -10013: action 5 turns 0x96 a frame on the spot
    // (speed 0) for 8 frames, then action 6.
    cue(&mut w, 3, 4, 1321, 1379, 0, p, IVec3::new(-1559, 690, -539), 1321);
    let at = go(&w, h).actor.world_pos;
    for k in 1..=8 {
        update(&mut w, h);
        let g = go(&w, h);
        assert_eq!(g.actor.world_rot.y as i32, -10013 + 0x96 * k);
        assert_eq!(g.actor.shape_rot.y, g.actor.world_rot.y);
        assert_eq!((g.actor.world_pos.x, g.actor.world_pos.z), (at.x, at.z));
    }
    assert_eq!(go(&w, h).action, 6);
    // Action 6: func_8097CC08 (unk_19C 8: 1.2), Actor_MoveXZGravity along its yaw, which
    // Actor_UpdatePos scales by R_UPDATE_RATE × 0.5 (3 × 0.5): 1.8 a frame.
    let before = go(&w, h).actor.world_pos;
    let yaw_before = go(&w, h).actor.world_rot.y;
    update(&mut w, h);
    let g = go(&w, h);
    assert_eq!(g.actor.speed_xz, 1.2);
    let yaw = yaw_before.wrapping_add(0x96);
    assert_eq!(g.actor.world_rot.y, yaw);
    let moved = g.actor.world_pos - before;
    let expect = Vec3::new(eng_math::sin_s(yaw), 0.0, eng_math::cos_s(yaw)) * 1.2 * 1.5;
    assert!((moved.x - expect.x).abs() < 1e-3 && (moved.z - expect.z).abs() < 1e-3, "{moved} {expect}");
}

#[test]
fn the_walk_steps_on_frames_12_and_25() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    let p = IVec3::new(-1546, 691, -591);
    let h = spawn(&mut w, 2, Vec3::new(-1631.0, 696.0, -523.0));
    update(&mut w, h);
    cue(&mut w, 5, 2, 1425, 1438, 0, p, p, 1438);
    update(&mut w, h);
    update(&mut w, h);
    cue(&mut w, 5, 3, 1438, 1760, 0, p, p, 1438);
    update(&mut w, h);
    while go(&w, h).action == 4 {
        update(&mut w, h);
    }
    // func_8097C9DC after SkelAnime_Update: NA_SE_EN_MORIBLIN_WALK as the walk's frame is 12 or 25.
    let mut steps = Vec::new();
    for _ in 0..60 {
        let n = sfx_count(&w);
        update(&mut w, h);
        let f = go(&w, h).skel.cur_frame;
        if sfx_since(&w, n).contains(&NA_SE_EN_MORIBLIN_WALK) {
            steps.push(f);
        }
    }
    assert!(!steps.is_empty());
    assert!(steps.iter().all(|&f| f == 12.0 || f == 25.0), "{steps:?}");
}

#[test]
fn a_turn_across_the_half_turn_goes_the_long_way_round() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    let p = IVec3::new(-1546, 691, -591);
    let h = spawn(&mut w, 0, Vec3::new(-1808.0, 684.0, -544.0));
    update(&mut w, h);
    cue(&mut w, 3, 2, 1270, 1286, -0x7000, p, p, 1286);
    update(&mut w, h);
    update(&mut w, h);
    cue(&mut w, 3, 3, 1296, 1321, -0x7000, p, p, 1296);
    update(&mut w, h);
    while go(&w, h).action == 4 {
        update(&mut w, h);
    }
    assert_eq!(go(&w, h).actor.world_rot.y, -0x7000);
    // To 0x7000: (s16)0x7000 - (-0x7000) = 0xE000 in 32 bits, positive: +0x96 a frame, up through
    // 0 (the short way would be down through 0x8000).
    cue(&mut w, 3, 4, 1321, 1379, 0x7000, p, p, 1321);
    update(&mut w, h);
    assert_eq!(go(&w, h).actor.world_rot.y, -0x7000 + 0x96);
}

#[test]
fn it_draws_en_go2s_skeleton_with_its_eye_and_the_smile() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    let h = spawn(&mut w, 0, Vec3::new(-1808.0, 684.0, -544.0));
    let view = ViewInfo { eye: Vec3::ZERO, billboard: glam::Mat4::IDENTITY };
    let draws = |w: &PlayState| {
        let g = go(w, h);
        let mut out = DrawOut::default();
        g.draw(&g.render_state(), w, &view, &mut out);
        out
    };
    // drawConfig 0 (func_8097D290): nothing.
    assert!(draws(&w).opa.is_empty());
    // func_8097D29C: sEyeTextures[unk_190] on 8 and gGoronCsMouthSmileTex on 9: En_Go2's eye
    // index + 1 (its 0 is gGoronCsEyeClosed2Tex) and mouth 1.
    w.actors.downcast_mut::<DemoGo>(h).unwrap().draw_config = 1;
    for eye in 0..3 {
        w.actors.downcast_mut::<DemoGo>(h).unwrap().unk_190 = eye;
        let out = draws(&w);
        assert_eq!(out.opa.len(), 1);
        assert_eq!(out.opa[0].mesh, eng_gfx::MeshKey::named(oot_game::pack::keys::bake(&oot_actors::en_go2::bake_name(eye as usize + 1, 1))));
    }
    assert_eq!(bake_name(0), oot_actors::en_go2::bake_name(1, 1));
}
