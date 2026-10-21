//! GAME-10 milestone 2 against the C: `Bg_Spot03_Taki` (`z_bg_spot03_taki.c`), the waterfall in
//! front of Zora's Domain, and one-point cutscene 4100 (`z_onepointdemo.c`). Zora's River's room 1
//! places it (params 0x0038) at (4226, 660, -1402); the song spot (`En_Okarina_Tag` 0x07F8) stands
//! at (4096, 921, -1400) on the ledge before it.
//!
//! Entered by `ENTR_ZORAS_RIVER_1` (spawn 1, room 1) on a `fairy-magic` save, Link put on the
//! ledge; the switch flag is set as the song spot sets it. Expected values are worked out from the
//! C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_spot03_taki::*;
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::sfx::{NA_SE_EV_WATER_WALL_BIG, SFX_FLAG, SfxPos};
use oot_game::camera::CAM_ID_MAIN;
use oot_game::env::clock_time;
use oot_game::onepoint::CAM_SET_CS_C;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// The falls' switch flag (their params 0x0038) and home.
const FLAG: i32 = 0x38;
const HOME: Vec3 = Vec3::new(4226.0, 660.0, -1402.0);
/// The song spot's ledge, where Link plays the lullaby.
const LEDGE: Vec3 = Vec3::new(4096.0, 921.0, -1400.0);

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// Child Link in room 1 (`fairy-magic`, the sound log on), on the ledge facing the falls (east),
/// three frames on.
fn room1(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_ZORAS_RIVER_1").expect("entrance");
    let mut save = SaveContext::new(e, false, clock_time(10, 0) as u16);
    save.apply_preset("fairy-magic").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    assert_eq!(w.room_ctx.cur.num, 1);
    w.place_player(LEDGE, 0x4000);
    idle(&mut w, 3);
    w
}

fn taki_h(w: &PlayState) -> ActorHandle {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<BgSpot03Taki>(h).is_some()).expect("the falls")
}

fn taki(w: &PlayState) -> &BgSpot03Taki {
    w.actors.downcast::<BgSpot03Taki>(taki_h(w)).unwrap()
}

fn collision_off(w: &PlayState) -> bool {
    w.col.dyna.actors[taki(w).bg as usize].collision_disabled
}

/// The frames the roar was asked for at the falls.
fn roars(w: &PlayState, h: ActorHandle) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, p)| s == NA_SE_EV_WATER_WALL_BIG - SFX_FLAG && p == SfxPos::Actor(h)).map(|&(f, _, _)| f).collect()
}

#[test]
fn the_falls_wait_closed_for_their_switch() {
    let Some(a) = assets() else { return };
    let mut w = room1(&a);
    let t = taki(&w);
    // BgSpot03Taki_Init: PARAMS_GET_U(0x0038, 0, 6) = 0x38; the init chain's 0.1; closed, the
    // alpha 255 in both copies; its collision (object_spot03_object_Col_000C98) on.
    assert_eq!((t.switch_flag, t.actor.home_pos, t.actor.scale), (0x38, HOME, Vec3::splat(0.1)));
    assert_eq!((t.state, t.opening_alpha, t.vtx_alpha), (WATERFALL_CLOSED, 255.0, [255, 255]));
    assert!(t.bg < eng_collision::dyna::BG_ACTOR_MAX && !collision_off(&w));
    // Each copy's bake loads its own array: vertices 10 to 14 among those drawn.
    for (i, v) in t.vertices.iter().enumerate() {
        let v = v.as_ref().expect("the falls' bake");
        assert_eq!(v.base, VTX[i]);
        let opening: Vec<_> = v.v.sources.iter().filter(|&&s| s.checked_sub(v.base).is_some_and(|o| OPENING_VTX.contains(&(o as usize / 16)))).collect();
        assert!(!opening.is_empty(), "copy {i}");
        // Alpha 0 on them alone.
        let colors = v.with_alphas(|k| OPENING_VTX.contains(&k).then_some(0));
        let changed = colors.iter().zip(&v.v.colors).filter(|(a, b)| a != b).count();
        assert!(changed > 0 && changed <= opening.len());
    }
    // Nothing happens without the flag; the draw takes the copies in turn.
    let mut drawn = Vec::new();
    for _ in 0..4 {
        idle(&mut w, 1);
        drawn.push(taki(&w).drawn_buffer);
    }
    assert_eq!(taki(&w).state, WATERFALL_CLOSED);
    assert!(drawn == [0, 1, 0, 1] || drawn == [1, 0, 1, 0], "{drawn:?}");
    // Closed, the roar at 1.0 every frame (Audio_PlaySfxWaterfall from its draw).
    let h = taki_h(&w);
    let f = w.audio.frames;
    assert!(roars(&w, h).ends_with(&[f - 3, f - 2, f - 1, f]), "{:?}", roars(&w, h));
    let l = &w.audio.waterfall_freq_scale_lerp;
    assert!(l.value == 1.0 || l.target == 1.0);
}

#[test]
fn the_switch_opens_them_with_one_point_4100_for_400_frames_then_they_close() {
    let Some(a) = assets() else { return };
    let mut w = room1(&a);
    w.flags.set_switch(FLAG);
    idle(&mut w, 1);
    // The flag seen: WATERFALL_OPENING_ANIMATED, timer 40 (never read), the alpha untouched this
    // frame; OnePointCutscene_Init(4100, -99, NULL, CAM_ID_MAIN).
    let t = taki(&w);
    assert_eq!((t.state, t.timer, t.opening_alpha), (WATERFALL_OPENING_ANIMATED, 40, 255.0));
    let sub = w.active_cam_id;
    assert_ne!(sub, CAM_ID_MAIN);
    let cam = w.camera(sub).unwrap();
    // 4100: D_801225D4's five keyframes on CAM_SET_CS_C; Link turned to 0x3FFC (shape and
    // world) and held (PLAYER_CSACTION_8, its first keyframe's too).
    assert_eq!((cam.cs_id, cam.setting, cam.cs_info.key_frame_cnt), (ONEPOINT_OPENING, CAM_SET_CS_C, 5));
    assert_eq!(cam.cs_info.key_frames.map(|k| (k.table, k.start)), Some((w.onepoint.table("D_801225D4"), 0)));
    assert_eq!(w.onepoint.len(w.onepoint.table("D_801225D4")), 5);
    assert_eq!((w.player().actor.shape_rot.y, w.player().actor.world_rot.y), (0x3FFC, 0x3FFC));
    assert_eq!(w.player().cs_mode, 8);
    // The fade: 5 a frame, into the copy each draw loads (ApplyOpeningAlpha before the draw).
    for k in 1..=50 {
        idle(&mut w, 1);
        let t = taki(&w);
        let want = 255.0 - 5.0 * k as f32;
        assert_eq!((t.state, t.opening_alpha), (WATERFALL_OPENING_ANIMATED, want), "frame {k}");
        assert_eq!(t.vtx_alpha[t.drawn_buffer as usize], want as u8, "frame {k}");
        assert!(!collision_off(&w));
    }
    // The 51st: 0, the collision off (DynaPoly_DisableCollision), open 400 frames; the roar at 0.5.
    idle(&mut w, 1);
    let t = taki(&w);
    assert_eq!((t.state, t.timer, t.opening_alpha), (WATERFALL_OPENED, 400, 0.0));
    assert_eq!(t.vtx_alpha[t.drawn_buffer as usize], 0);
    assert!(collision_off(&w));
    let l = &w.audio.waterfall_freq_scale_lerp;
    assert!(l.value == 0.5 || l.target == 0.5);
    // 400 down to 0, then -1: closing.
    idle(&mut w, 400);
    assert_eq!((taki(&w).state, taki(&w).timer), (WATERFALL_OPENED, 0));
    assert!(w.flags.get_switch(FLAG));
    idle(&mut w, 1);
    assert_eq!((taki(&w).state, taki(&w).timer, taki(&w).opening_alpha), (WATERFALL_CLOSING, -1, 0.0));
    // Back up 5 a frame: at 255 the collision on, closed, the flag cleared.
    for k in 1..=50 {
        idle(&mut w, 1);
        assert_eq!((taki(&w).state, taki(&w).opening_alpha), (WATERFALL_CLOSING, 5.0 * k as f32), "frame {k}");
        assert!(collision_off(&w));
    }
    idle(&mut w, 1);
    let t = taki(&w);
    assert_eq!((t.state, t.opening_alpha, t.vtx_alpha[t.drawn_buffer as usize]), (WATERFALL_CLOSED, 255.0, 255));
    assert!(!collision_off(&w));
    assert!(!w.flags.get_switch(FLAG));
    // The cutscene long over: the main camera.
    assert_eq!(w.active_cam_id, CAM_ID_MAIN);
}
