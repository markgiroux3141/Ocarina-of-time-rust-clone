//! GAME-06 milestone 1b against the C: `Demo_Kankyo` (`z_demo_kankyo.c`: the rain, the rocks,
//! the clouds, the Door of Time, the warp sparkles and their leave, the sparkles),
//! `Bg_Spot09_Obj` (`z_bg_spot09_obj.c`), `Bg_Spot16_Doughnut` (`z_bg_spot16_doughnut.c`), the
//! normal sky's loads (`Environment_UpdateSkybox`, `z_kankyo.c`); then the chain's run
//! (`Route::Creation`): the creation from the cutscene map to Kokiri Forest's layer 6.
//!
//! The actors are spawned by hand in Link's house (the scene they check set on `play->sceneId`)
//! and each updated alone, as `Actor_UpdateAll` would, its draw-time changes made as
//! `Actor_DrawAll` would. Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::{IVec3, Vec3};
use oot_actors::bg_spot09_obj::*;
use oot_actors::bg_spot16_doughnut::*;
use oot_actors::demo_kankyo::*;
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::actor::ACTOR_FLAG_UPDATE_DURING_OCARINA;
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorHandle};
use oot_game::audio::sfx::*;
use oot_game::cutscene::CsCmdActorCue;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, scripted_input};
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

/// `object_table.h`.
const OBJECT_TOKI_OBJECTS: i16 = 0x005E;
const OBJECT_EFC_STAR_FIELD: i16 = 0x0092;
const OBJECT_SPOT09_OBJ: i16 = 0x00AE;
const OBJECT_EFC_DOUGHNUT: i16 = 0x017A;

/// `scene_table.h`.
const SCENE_CUTSCENE_MAP: u16 = 0x47;
const SCENE_GERUDO_VALLEY: u16 = 0x5A;
const SCENE_DEATH_MOUNTAIN_TRAIL: u16 = 0x60;
const SCENE_KAKARIKO_VILLAGE: u16 = 0x52;

/// Link's house as a child after the tree's death (or an adult), three frames run, the
/// objects these tests use loaded, the sound log on.
fn house(a: &Arc<GameAssets>, adult: bool) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_LINKS_HOUSE_0").expect("entrance");
    let mut save = SaveContext::new(e, adult, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-dead").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 3);
    for o in [OBJECT_TOKI_OBJECTS, OBJECT_EFC_STAR_FIELD, OBJECT_SPOT09_OBJ, OBJECT_EFC_DOUGHNUT] {
        if w.object_ctx.get_index(o).is_none() {
            w.object_ctx.spawn(o);
        }
    }
    w
}

fn spawn(w: &mut PlayState, id: i16, params: i16) -> ActorHandle {
    w.actor_spawn(id, Vec3::new(10.0, 20.0, 30.0), [0; 3], params).expect("spawned")
}

/// The actor's update alone (`Actor_UpdateAll`'s call), as the updating actor.
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// The draw's changes to the actor, as `Actor_DrawAll` makes them, then its sounds.
fn draw_update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.draw_update(w);
    a.draw_sfx(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

fn draws(w: &PlayState, h: ActorHandle) -> DrawOut {
    let a = w.actors.get(h).unwrap();
    let mut out = DrawOut::default();
    let view = ViewInfo { eye: Vec3::new(1.0, 2.0, 3.0), billboard: glam::Mat4::IDENTITY };
    a.draw(&RenderState::of(a.base()), w, &view, &mut out);
    out
}

fn kankyo(w: &PlayState, h: ActorHandle) -> &DemoKankyo {
    w.actors.downcast::<DemoKankyo>(h).expect("a Demo_Kankyo")
}

fn kankyo_mut(w: &mut PlayState, h: ActorHandle) -> &mut DemoKankyo {
    w.actors.downcast_mut::<DemoKankyo>(h).expect("a Demo_Kankyo")
}

fn sfx_heard(w: &PlayState, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(_, s, _)| s == id)
}

/// A cue on `channel`, from `start` to `end` over the frames `from..to`, the cutscene running at
/// frame `frame`.
fn cue(w: &mut PlayState, channel: usize, action: u16, from: u16, to: u16, start: IVec3, end: IVec3, frame: u16) {
    w.cs_ctx.state = CS_STATE_RUN;
    w.cs_ctx.frames = frame;
    w.cs_ctx.npc_actions[channel] = Some(CsCmdActorCue { action, start_frame: from, end_frame: to, rot: [0; 3], start_pos: start, end_pos: end, normal: IVec3::ZERO });
}

#[test]
fn the_cutscene_maps_rain_hides_the_room_and_falls_in_front_of_the_eye() {
    let Some(a) = assets() else { return };
    let mut w = house(&a, false);
    w.scene_id = SCENE_CUTSCENE_MAP;
    w.save.entrance_index = a.scenes.entrance_index("ENTR_CUTSCENE_MAP_0").unwrap();
    assert!(w.room_ctx.cur.loaded);
    let h = spawn(&mut w, ACTOR_DEMO_KANKYO, DEMOKANKYO_BLUE_RAIN);
    // SCENE_CUTSCENE_MAP: roomCtx.curRoom.segment = NULL, D_8098CF80 10, sRainScale 8.
    assert!(!w.room_ctx.cur.loaded);
    let s = statics(&mut w);
    assert_eq!((s.d_8098cf80, s.rain_scale), (10, 8));
    // Its object (object_efc_star_field) isn't its slot yet: the first draw doesn't draw, then
    // takes the slot.
    w.view.eye = Vec3::ZERO;
    w.view.at = Vec3::new(0.0, 0.0, -100.0);
    update(&mut w, h);
    draw_update(&mut w, h);
    assert!(!kankyo(&w, h).drawn && kankyo(&w, h).rain.is_empty());
    draw_update(&mut w, h);
    let k = kankyo(&w, h);
    assert!(k.drawn);
    // 30 streaks of 5 drops; each 350 ahead of the eye along the view (80 up), its start
    // (func_80989B54: x and z in ±250, speed 10 + [0, 40)) then y in [0, 500) at ENTR_CUTSCENE_MAP_0.
    assert_eq!(k.rain.len(), 150);
    assert_eq!(k.rain_colors, ([200, 255, 255, 255], [0, 150, 255, 255]));
    for (i, e) in k.unk_150.iter().enumerate() {
        assert_eq!(e.unk_22, 1);
        assert_eq!(e.unk_c, Vec3::new(0.0, 0.0, -350.0));
        assert!(e.unk_0.x.abs() <= 250.0 && e.unk_0.z.abs() <= 250.0);
        assert!((0.0..500.0).contains(&e.unk_0.y));
        assert!((10.0..50.0).contains(&e.unk_18));
        // The first drop: translated to unk_C + unk_0, scaled 8 × 0.001 (no turn in the map).
        let m = k.rain[i * 5];
        assert!((m.w_axis.truncate() - (e.unk_c + e.unk_0)).length() < 1e-3);
        assert!((m.x_axis.x - 0.008).abs() < 1e-6);
        // The next drop: 1500 back along x and z (towards the middle), 4000 up (j = 1 is odd).
        let n = k.rain[i * 5 + 1];
        let sx = if e.unk_0.x >= 0.0 { -1500.0 } else { 1500.0 };
        let sz = if e.unk_0.z >= 0.0 { -1500.0 } else { 1500.0 };
        assert!((n.w_axis.truncate() - (m.w_axis.truncate() + Vec3::new(sx, 4000.0, sz) * 0.008)).length() < 1e-2);
    }
    // Falling: y less its speed each draw; below 300 under the eye's line (eye.y + 150 × dy = 0)
    // the streak restarts (case 2: func_80989B54's y of 500, back to case 1).
    let (y0, speed) = (k.unk_150[0].unk_0.y, k.unk_150[0].unk_18);
    draw_update(&mut w, h);
    assert_eq!(kankyo(&w, h).unk_150[0].unk_0.y, y0 - speed);
    kankyo_mut(&mut w, h).unk_150[0].unk_0.y = -300.0;
    draw_update(&mut w, h);
    assert_eq!(kankyo(&w, h).unk_150[0].unk_22, 2);
    draw_update(&mut w, h);
    let e = kankyo(&w, h).unk_150[0];
    assert_eq!((e.unk_22, e.unk_0.y), (1, 500.0));
    assert_eq!(draws(&w, h).xlu.len(), 150);
}

#[test]
fn rain_elsewhere_than_its_three_scenes_is_killed() {
    let Some(a) = assets() else { return };
    let mut w = house(&a, false);
    let h = spawn(&mut w, ACTOR_DEMO_KANKYO, DEMOKANKYO_BLUE_RAIN_2);
    assert!(kankyo(&w, h).actor.killed);
    // Link's house's room is still drawn.
    assert!(w.room_ctx.cur.loaded);
}

#[test]
fn dins_rocks_hide_the_room_follow_their_cue_and_tumble() {
    let Some(a) = assets() else { return };
    let mut w = house(&a, false);
    w.scene_id = SCENE_GERUDO_VALLEY;
    let h = spawn(&mut w, ACTOR_DEMO_KANKYO, DEMOKANKYO_ROCK_2);
    assert!(!w.room_ctx.cur.loaded);
    let k = kankyo(&w, h);
    // Scale 0.5 + [0, 0.5); the tumble's steps 1 + [0, 3) degrees a frame on each axis.
    assert!((0.5..1.0).contains(&k.actor.scale.x) && k.actor.scale.x == k.actor.scale.z);
    let step = k.unk_150[0].unk_0;
    for v in [step.x, step.y, step.z] {
        assert!((1.0..4.0).contains(&v));
    }
    // DemoKankyo_SetupType waits for the slot (taken at the first draw), then UpdateRock.
    update(&mut w, h);
    draw_update(&mut w, h);
    update(&mut w, h);
    assert_eq!(kankyo(&w, h).action, Action::UpdateRock);
    // ROCK_2's cue is channel 1: halfway through frames 0..10 at frame 5.
    cue(&mut w, 1, 1, 0, 10, IVec3::new(0, 100, 0), IVec3::new(200, 300, -400), 5);
    update(&mut w, h);
    let k = kankyo(&w, h);
    assert_eq!(k.actor.world_pos, Vec3::new(100.0, 200.0, -200.0));
    assert_eq!(k.unk_150[0].unk_c, step);
    update(&mut w, h);
    assert_eq!(kankyo(&w, h).unk_150[0].unk_c, step + step);
    draw_update(&mut w, h);
    assert_eq!(draws(&w, h).opa.len(), 1);
}

#[test]
fn the_clouds_circle_by_their_own_speed() {
    let Some(a) = assets() else { return };
    let mut w = house(&a, false);
    let h = spawn(&mut w, ACTOR_DEMO_KANKYO, DEMOKANKYO_CLOUDS);
    let speeds: Vec<f32> = kankyo(&w, h).unk_150.iter().map(|e| e.unk_18).collect();
    assert!(speeds.iter().all(|s| (60.0..160.0).contains(s)));
    // gameplay_keep is its slot from the start: SetupType goes straight to UpdateClouds.
    update(&mut w, h);
    assert_eq!(kankyo(&w, h).action, Action::UpdateClouds);
    let before: Vec<i16> = kankyo(&w, h).unk_150.iter().map(|e| e.unk_20).collect();
    update(&mut w, h);
    for (i, e) in kankyo(&w, h).unk_150.iter().enumerate() {
        assert_eq!(e.unk_20, before[i].wrapping_add(speeds[i] as i16));
    }
    draw_update(&mut w, h);
    assert_eq!(draws(&w, h).xlu.len(), 30);
}

#[test]
fn the_door_of_time_slides_open_with_cutscene_flag_2() {
    let Some(a) = assets() else { return };
    let mut w = house(&a, true);
    let h = spawn(&mut w, ACTOR_DEMO_KANKYO, DEMOKANKYO_DOOR_OF_TIME);
    // Door_Toki (its collision) spawned as its child.
    let child = kankyo(&w, h).actor.child.expect("Door_Toki");
    update(&mut w, h);
    draw_update(&mut w, h);
    update(&mut w, h);
    assert_eq!(kankyo(&w, h).action, Action::SetupType);
    w.flags_set_env(2);
    update(&mut w, h);
    assert_eq!(kankyo(&w, h).action, Action::UpdateDoorOfTime);
    // A unit a frame; at 102 the flag, the stop's sound and the child gone.
    for n in 1..=101 {
        update(&mut w, h);
        assert_eq!(kankyo(&w, h).unk_150[0].unk_18, n as f32);
    }
    assert!(!w.save.get_event_chk_inf(0x4B));
    update(&mut w, h);
    assert!(w.save.get_event_chk_inf(0x4B));
    assert!(sfx_heard(&w, NA_SE_EV_STONEDOOR_STOP));
    assert!(w.actors.get(child).is_none_or(|c| c.base().killed));
    assert_eq!(kankyo(&w, h).action, Action::KillDoorOfTimeCollision);
    // Opened, it isn't there: roomCtx.drawParams[1] 0xFF.
    let h2 = spawn(&mut w, ACTOR_DEMO_KANKYO, DEMOKANKYO_DOOR_OF_TIME);
    assert!(kankyo(&w, h2).actor.killed);
    assert_eq!(w.scene.as_ref().unwrap().draw.room_draw_params[1], 0xFF);
}

#[test]
fn the_warp_out_whites_the_screen_then_starts_the_warp_in_cutscene() {
    let Some(a) = assets() else { return };
    let mut w = house(&a, false);
    let h = spawn(&mut w, ACTOR_DEMO_KANKYO, DEMOKANKYO_WARP_OUT);
    let k = kankyo(&w, h);
    assert_eq!((k.actor.category, k.actor.room, k.warp_timer), (ACTORCAT_ITEMACTION, -1, 35));
    assert!(k.actor.flags & ACTOR_FLAG_UPDATE_DURING_OCARINA != 0);
    assert!(sfx_heard(&w, NA_SE_EV_SARIA_MELODY));
    // Update n sees the timer at 36 - n before its decrement: from 20 to 15 the white fades
    // in (255 - 255 × (t - 15) / 5), from 14 to 4 out (255 × (t - 4) / 10).
    let mut fills = Vec::new();
    for _ in 0..33 {
        update(&mut w, h);
        fills.push(w.transition.screen_fill.map(|f| f[3]));
    }
    let at = |t: usize| fills[35 - t];
    assert_eq!(at(21), None);
    assert_eq!(at(20), Some(0));
    assert_eq!(at(17), Some(153));
    assert_eq!(at(15), Some(255));
    assert_eq!(at(14), Some(255));
    assert_eq!(at(10), Some(153));
    assert_eq!(at(4), Some(0));
    assert_eq!(at(3), None);
    assert!(fills.iter().flatten().count() == 17);
    // The 34th: the timer to 1, gChildWarpInCS (D_8098CF84 32), the trigger (the camera isn't
    // fixed in the house? Play_CamIsNotFixed), DoNothing.
    assert!(w.cs_ctx.segment.is_none());
    update(&mut w, h);
    assert_eq!(kankyo(&w, h).warp_timer, 1);
    assert!(w.cs_ctx.segment.is_some());
    assert_eq!(statics(&mut w).d_8098cf84, 32);
    assert_eq!(w.save.cutscene_trigger == 1, w.cam_is_not_fixed());
    assert_eq!(kankyo(&w, h).action, Action::DoNothing);
}

#[test]
fn the_warp_sparkles_climb_their_spline_and_the_first_leaves() {
    let Some(a) = assets() else { return };
    let mut w = house(&a, false);
    w.save.respawn[oot_game::save::RESPAWN_MODE_RETURN].entrance_index = 0x053;
    let h = spawn(&mut w, ACTOR_DEMO_KANKYO, DEMOKANKYO_WARP_OUT);
    w.msg_ctx.last_played_song = 2;
    draw_update(&mut w, h);
    // Two sparkles a frame to 30, each at Link (D_8098CF98) plus its offset, minuet to prelude's
    // colours by the song played (serenade's).
    let k = kankyo(&w, h);
    assert_eq!(k.sparkle_counter, 2);
    assert!(k.sparkles.iter().all(|s| s.env == [0, 150, 255]));
    for _ in 0..20 {
        draw_update(&mut w, h);
    }
    assert_eq!(kankyo(&w, h).sparkle_counter, 30);
    // Sparkle 0 rides sWarpOutCameraPoints to its end (keyframe 9, 8 or 5 frames a point),
    // then Environment_WarpSongLeave: out to the return entrance in a white fade, its flag.
    let mut n = 0;
    while w.transition.trigger != oot_game::transition::TRANS_TRIGGER_START && n < 400 {
        draw_update(&mut w, h);
        n += 1;
    }
    assert!(n < 400, "never left");
    assert_eq!(w.transition.next_entrance_index, 0x053);
    assert_eq!(w.transition.ty, oot_game::transition::TRANS_TYPE_FADE_WHITE);
    assert_eq!((w.save.respawn_flag, w.save.cutscene_index), (-3, 0));
    // ENTR_TEMPLE_OF_TIME_0: EVENTCHKINF_A7.
    assert!(w.save.get_event_chk_inf(0xA7));
    assert_eq!(kankyo(&w, h).unk_150[0].unk_22, 3);
}

#[test]
fn the_sparkles_grow_to_twenty_and_go_once_the_cutscene_is_over() {
    let Some(a) = assets() else { return };
    let mut w = house(&a, false);
    let h = spawn(&mut w, ACTOR_DEMO_KANKYO, DEMOKANKYO_SPARKLES);
    for n in 1..=20 {
        draw_update(&mut w, h);
        assert_eq!(kankyo(&w, h).sparkle_counter, n);
    }
    draw_update(&mut w, h);
    assert_eq!(kankyo(&w, h).sparkle_counter, 20);
    let drawn = kankyo(&w, h).sparkles.len();
    assert_eq!(draws(&w, h).xlu.len(), drawn);
    // The colour: sSparkleEnvColors[3].
    assert!(kankyo(&w, h).sparkles.iter().all(|s| s.env == [255, 150, 0]));
    let mut n = 0;
    while !kankyo(&w, h).actor.killed && n < 600 {
        draw_update(&mut w, h);
        n += 1;
    }
    assert!(kankyo(&w, h).actor.killed, "the last sparkle never ended");
}

#[test]
fn add_pos_rot_turns_the_offset_by_the_yaw() {
    // OLib_Vec3fToVecGeo, the yaw plus 0x4000, OLib_VecGeoToVec3f: +z turned to +x.
    let v = vec3f_add_pos_rot((Vec3::new(100.0, 5.0, 0.0), 0x4000), Vec3::new(0.0, 0.0, 10.0));
    assert!((v - Vec3::new(110.0, 5.0, 0.0)).length() < 0.05, "{v}");
}

#[test]
fn gerudo_valleys_bridges_and_tent_by_age_rescue_and_layer() {
    let Some(a) = assets() else { return };
    let alive = |w: &mut PlayState, p: i16| {
        let h = spawn(w, ACTOR_BG_SPOT09_OBJ, p);
        let o = w.actors.downcast::<BgSpot09Obj>(h).unwrap();
        (!o.actor.killed).then_some((o.actor.scale.x, o.bg != eng_collision::dyna::BG_ACTOR_MAX))
    };
    // A child: the child's bridge only, at 1, with gValleyObjects2Col.
    let mut w = house(&a, false);
    let got: Vec<_> = (0..5).map(|p| alive(&mut w, p)).collect();
    assert_eq!(got, vec![None, None, Some((1.0, true)), None, None]);
    // A cutscene layer: the sides only, with no collision.
    w.save.scene_layer = 4;
    let got: Vec<_> = (0..5).map(|p| alive(&mut w, p)).collect();
    assert_eq!(got, vec![Some((1.0, false)), None, None, None, None]);
    // An adult: the broken bridge and the tent (at 0.1); with the four carpenters rescued, the
    // repaired bridge instead.
    let mut w = house(&a, true);
    let got: Vec<_> = (0..5).map(|p| alive(&mut w, p)).collect();
    assert_eq!(got, vec![None, Some((1.0, true)), None, Some((0.1, true)), None]);
    for f in 0x90..=0x93 {
        w.save.set_event_chk_inf(f);
    }
    let got: Vec<_> = (0..5).map(|p| alive(&mut w, p)).collect();
    assert_eq!(got, vec![None, None, None, Some((0.1, true)), Some((1.0, true))]);
}

#[test]
fn death_mountains_cloud_ring_turns_and_the_fiery_one_fades_on_its_cue() {
    let Some(a) = assets() else { return };
    fn ring(w: &PlayState, h: ActorHandle) -> &BgSpot16Doughnut {
        w.actors.downcast::<BgSpot16Doughnut>(h).unwrap()
    }
    // A child: white, 0.1 (elsewhere than Kakariko and the Temple of Time's outside), turning
    // 0x20 a frame.
    let mut w = house(&a, false);
    w.scene_id = SCENE_DEATH_MOUNTAIN_TRAIL;
    let h = spawn(&mut w, ACTOR_BG_SPOT16_DOUGHNUT, -1);
    assert_eq!((ring(&w, h).fire_flag, ring(&w, h).actor.scale.x, ring(&w, h).env_color_alpha), (0, 0.1, 255));
    update(&mut w, h);
    assert_eq!(ring(&w, h).actor.shape_rot.y, -0x20);
    w.scene_id = SCENE_KAKARIKO_VILLAGE;
    let k = spawn(&mut w, ACTOR_BG_SPOT16_DOUGHNUT, -1);
    assert_eq!(ring(&w, k).actor.scale.x, 0.04);
    // An adult before EVENTCHKINF_2F: fiery, still; channel 2's cue 2 fades it 5 a frame, then
    // it's white.
    let mut w = house(&a, true);
    let h = spawn(&mut w, ACTOR_BG_SPOT16_DOUGHNUT, -1);
    assert_eq!(ring(&w, h).fire_flag, 1);
    update(&mut w, h);
    assert_eq!((ring(&w, h).actor.shape_rot.y, ring(&w, h).env_color_alpha), (0, 255));
    cue(&mut w, 2, 2, 0, 100, IVec3::ZERO, IVec3::ZERO, 1);
    for _ in 0..50 {
        update(&mut w, h);
    }
    assert_eq!((ring(&w, h).env_color_alpha, ring(&w, h).fire_flag), (5, 1));
    update(&mut w, h);
    assert_eq!((ring(&w, h).env_color_alpha, ring(&w, h).fire_flag), (0, 0));
    update(&mut w, h);
    assert_eq!(ring(&w, h).env_color_alpha, 5);
    // Expanding (params 2): 70 × 1e-4, growing 0.002 a frame, gone when its alpha is under 6.
    let e = spawn(&mut w, ACTOR_BG_SPOT16_DOUGHNUT, 2);
    assert!((ring(&w, e).actor.scale.x - 0.007).abs() < 1e-7);
    for n in 1..=50 {
        update(&mut w, e);
        assert_eq!(ring(&w, e).env_color_alpha, 255 - 5 * n as u8);
    }
    assert!(!ring(&w, e).actor.killed);
    update(&mut w, e);
    assert!(ring(&w, e).actor.killed);
    assert!((ring(&w, e).actor.scale.x - (0.007 + 51.0 * 0.0019999998)).abs() < 1e-5);
}

#[test]
fn the_normal_sky_loads_its_textures_then_their_palettes_a_call_later() {
    use oot_game::env::*;
    use oot_game::skybox::SKYBOX_NORMAL_SKY;
    let Some(a) = assets() else { return };
    let w = house(&a, false);
    let mut env = w.env_ctx.clone();
    let mut sky = w.skybox_ctx.clone();
    let mut statics = w.env_statics.clone();
    // Environment_Init's indices (99), the normal sky's first config at 10:00.
    env.skybox1_index = 99;
    env.skybox2_index = 99;
    env.skybox_dma_state = SKYBOX_DMA_INACTIVE;
    env.skybox_config = 0;
    env.change_skybox_state = CHANGE_SKYBOX_INACTIVE;
    let skybox_time = clock_time(10, 0) as u16;
    let (_, e) = a.env.skybox_entry(0, skybox_time).expect("an entry at 10:00");
    sky.textures = [0xEE, 0xEE];
    sky.palettes = [0xEE, 0xEE];
    // Each call: texture 1's DMA started (not received in the call that starts it), received,
    // its palette started, received; then texture 2's the same.
    let mut states = Vec::new();
    for _ in 0..8 {
        env.update_skybox(&a.env, SKYBOX_NORMAL_SKY, &mut sky, &mut statics, false, skybox_time);
        states.push((env.skybox_dma_state, sky.textures, sky.palettes));
    }
    let (t1, t2) = (e.skybox1_index, e.skybox2_index);
    // The palette's half: the first when the index's bits 0 and 2 differ.
    let half = |i: u8| if (i & 1) ^ ((i & 4) >> 2) != 0 { 0 } else { 1 };
    let mut p = [0xEE, 0xEE];
    let p1 = {
        p[half(t1)] = t1;
        p
    };
    let p2 = {
        p[half(t2)] = t2;
        p
    };
    assert_eq!(
        states,
        vec![
            (SKYBOX_DMA_TEXTURE1_START, [0xEE, 0xEE], [0xEE, 0xEE]),
            (SKYBOX_DMA_TEXTURE1_DONE, [t1, 0xEE], [0xEE, 0xEE]),
            (SKYBOX_DMA_TLUT1_START, [t1, 0xEE], [0xEE, 0xEE]),
            (SKYBOX_DMA_INACTIVE, [t1, 0xEE], p1),
            (SKYBOX_DMA_TEXTURE2_START, [t1, 0xEE], p1),
            (SKYBOX_DMA_TEXTURE2_DONE, [t1, t2], p1),
            (SKYBOX_DMA_TLUT2_START, [t1, t2], p1),
            (SKYBOX_DMA_INACTIVE, [t1, t2], p2),
        ]
    );
    // A blend of 255 to 0 by the entry's time, or (a still entry) 255 before its middle.
    let w8 = (lerp_weight(e.end_time, e.start_time, skybox_time) * 255.0) as u8;
    assert_eq!(env.skybox_blend, if e.change_skybox { w8 } else if w8 < 128 { 255 } else { 0 });
}

#[test]
fn exit_the_creation_from_the_cutscene_map_to_the_emerald() {
    let Some(a) = assets() else { return };
    let route = Route::Creation;
    let e = a.scenes.entrance_index(route.entrance()).expect("entrance");
    let save = route.save(e);
    // CS_INDEX_1: the cutscene map's layer 5 (part 2, Ganondorf).
    assert_eq!(save.cutscene_index, 0xFFF1);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    assert_eq!(w.save.scene_layer, 5);
    route.debug_start(&mut w);
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    // What each layer has: the rain hiding the cutscene map (layers 4 and 6), Din's five rocks
    // hiding Gerudo Valley's layer 5, the bridge's sides alone in its layer 4, and no cloud ring
    // in Death Mountain Trail's layer 4 (its object isn't in the layer's list: Actor_Spawn's
    // "No data bank").
    let (mut rain, mut rocks, mut sides, mut ring) = (false, false, false, false);
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
        let kankyo: Vec<i16> = w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<DemoKankyo>(h)).filter(|k| !k.actor.killed).map(|k| k.actor.params).collect();
        match (w.scene_id, w.save.scene_layer) {
            (SCENE_CUTSCENE_MAP, 4 | 6) if kankyo == [DEMOKANKYO_BLUE_RAIN] && !w.room_ctx.cur.loaded => rain = true,
            (SCENE_GERUDO_VALLEY, 5) if kankyo.len() == 5 && !w.room_ctx.cur.loaded => rocks = true,
            (SCENE_GERUDO_VALLEY, 4) => {
                let objs: Vec<i16> = w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<BgSpot09Obj>(h)).filter(|o| !o.actor.killed).map(|o| o.actor.params).collect();
                if objs == [BG_SPOT09_OBJ_BRIDGE_SIDES] {
                    sides = true;
                }
            }
            (SCENE_DEATH_MOUNTAIN_TRAIL, 4) => ring |= w.actors.all().into_iter().any(|h| w.actors.downcast::<BgSpot16Doughnut>(h).is_some()),
            _ => {}
        }
    }
    assert!(run.failure.is_none(), "{:?}", run.failure);
    eprintln!("steps {:?}; texts {:x?}", run.steps, run.texts);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(
        steps,
        vec![Step::Ganondorf, Step::Goddesses, Step::Din, Step::Valley, Step::Nayru, Step::Farore, Step::Triforce, Step::EmeraldPart9, Step::Forest11]
    );
    // Parts 2 to 9's texts, in order (their scripts' TEXT_LISTs).
    let mut want = vec![0x107E, 0x107F, 0x1080, 0x1081, 0x1082, 0x1083, 0x1084, 0x1085, 0x1086, 0x1087, 0x1088, 0x1089, 0x108A, 0x108B, 0x108C, 0x108D];
    want.extend([0x1029, 0x1093, 0x0080, 0x102A, 0x102B, 0x102C]);
    assert_eq!(run.texts, want);
    assert!(rain && rocks && sides && !ring, "rain {rain} rocks {rocks} sides {sides} ring {ring}");
    // Part 9's terminator: ENTR_KOKIRI_FOREST_11.
    assert_eq!(Some(w.save.entrance_index), a.scenes.entrance_index("ENTR_KOKIRI_FOREST_11"));
}

#[test]
fn exit_the_whole_chain_from_the_blue_warp_to_the_emerald() {
    let Some(a) = assets() else { return };
    // Route::BlueWarp's start (Queen Gohma's room cleared), its run carried on by the creation's:
    // part 1 in Kokiri Forest's layer 5, then parts 2 to 9.
    let route = Route::BlueWarp;
    let e = a.scenes.entrance_index(route.entrance()).expect("entrance");
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), route.save(e), audio).expect("Play_Init");
    route.debug_start(&mut w);
    let mut run = Playthrough::for_routes(&[Route::BlueWarp, Route::Creation]);
    let mut prev = PadState::default();
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
    }
    assert!(run.failure.is_none(), "{:?}", run.failure);
    eprintln!("steps {:?}", run.steps);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    let mut rest = steps.iter();
    for s in [
        Step::WarpEntered,
        Step::WarpedOut,
        Step::EmeraldPart1Over,
        Step::Ganondorf,
        Step::Goddesses,
        Step::Din,
        Step::Valley,
        Step::Nayru,
        Step::Farore,
        Step::Triforce,
        Step::EmeraldPart9,
        Step::Forest11,
    ] {
        assert!(rest.any(|&t| t == s), "{s:?} in order in {steps:?}");
    }
    assert_eq!(Some(w.save.entrance_index), a.scenes.entrance_index("ENTR_KOKIRI_FOREST_11"));
    // The tree's death (part 9's CS_MISC_DEKU_TREE_DEATH): EVENTCHKINF_07 kept.
    assert!(w.save.get_event_chk_inf(oot_game::save::EVENTCHKINF_07));
}
