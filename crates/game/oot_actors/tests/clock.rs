//! GAME-06 milestone 3 against the C: the clock (`z_kankyo.c`'s `Environment_Update`,
//! `z_scene.c`'s `Scene_CommandTimeSettings`, `z_play.c`'s `Play_Init`, `z_parameter.c`'s Sun's
//! Song), the time-based music, the sun and the moon (`Environment_DrawSunAndMoon`), the skybox
//! filters (`Environment_DrawSkyboxFilters`), the lens flare (`Environment_DrawLensFlare`), the
//! drawbridge's night (`z_bg_spot00_hanebasi.c`); then the exit's run (`Route::Dusk`): Hyrule
//! Field from 17:00 to the night.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::bg_spot00_hanebasi::{BgSpot00Hanebasi, DT_DRAWBRIDGE};
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::scene::*;
use oot_game::audio::sfx::{NA_SE_EV_CHICKEN_CRY_M, NA_SE_EV_DOG_CRY_EVENING};
use oot_game::clock::*;
use oot_game::env::clock_time;
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

fn t(h: i32, m: i32) -> u16 {
    clock_time(h, m) as u16
}

/// A child's play state (`deku-tree-dead`) entered at `entrance` at `time`, before its first frame;
/// the audio side on (the field's spec change).
fn world(a: &Arc<GameAssets>, entrance: &str, time: u16, prep: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, false, time);
    save.apply_preset("deku-tree-dead").unwrap();
    prep(&mut save);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    w.audio_side = Some(Box::new(oot_game::audio::offline::OfflineAudio::new(&a.pack.audio_data().unwrap(), false)));
    w
}

/// Frames until the transition is over (time doesn't pass during it).
fn settle(w: &mut PlayState) {
    for _ in 0..200 {
        if w.transition.mode == oot_game::transition::TRANS_MODE_OFF && w.transition.trigger == oot_game::transition::TRANS_TRIGGER_OFF {
            return;
        }
        idle(w, 1);
    }
    panic!("the transition never ended");
}

fn sfx_on(w: &PlayState, id: u16) -> bool {
    let f = w.audio.frames;
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(fr, s, _)| fr == f && s == id)
}

#[test]
fn the_fields_room_sets_the_time_speed_and_the_forests_stops_it() {
    let Some(a) = assets() else { return };
    // spot00_room_0: SCENE_CMD_TIME_SETTINGS(0xFF, 0xFF, 10): the time kept, sceneTimeSpeed 10,
    // gTimeSpeed 10 (sunsSongState SUNSSONG_INACTIVE); sunPos by the time; no snap (the speed
    // isn't 0).
    let w = world(&a, "ENTR_HYRULE_FIELD_0", t(17, 0), |_| {});
    assert_eq!((w.env_ctx.scene_time_speed, w.env_statics.time_speed), (10, 10));
    assert_eq!(w.save.day_time, t(17, 0));
    // SaveContext_Init's skyboxTime (0) until the first Environment_Update.
    assert_eq!(w.save.skybox_time, 0);
    assert_eq!(w.env_ctx.sun_pos, sun_pos(t(17, 0)));
    // -(sin(0x3555) * 120) * 25, cos(..) * 120 * 25, cos(..) * 20 * 25 (Math_SinS's table).
    let s = (0x3555i32 as f32 / 32768.0 * std::f32::consts::PI).sin();
    assert!((w.env_ctx.sun_pos.x + s * 3000.0).abs() < 4.0, "{:?}", w.env_ctx.sun_pos);

    // spot04's rooms: SCENE_CMD_TIME_SETTINGS(0xFF, 0xFF, 0): time stands; outside a cutscene the
    // sky's time snapped out of the dusk: 16:30 is in CLOCK_TIME(16, 0) .. CLOCK_TIME(17, 0), so
    // CLOCK_TIME(17, 0) + 1.
    let mut w = world(&a, "ENTR_KOKIRI_FOREST_0", t(16, 30), |_| {});
    assert_eq!((w.env_ctx.scene_time_speed, w.env_statics.time_speed), (0, 0));
    assert_eq!((w.save.day_time, w.save.skybox_time), (t(16, 30), t(17, 0) + 1));
    idle(&mut w, 60);
    // gTimeSpeed 0: nothing moves; layer 0 (< 5) with no speed: skyboxTime stays.
    assert_eq!((w.save.day_time, w.save.skybox_time, w.save.night_flag), (t(16, 30), t(17, 0) + 1, false));
}

#[test]
fn the_clock_runs_by_ten_by_day_twenty_by_night_and_waits_for_messages() {
    let Some(a) = assets() else { return };
    let mut w = world(&a, "ENTR_HYRULE_FIELD_0", t(17, 59), |s| s.set_event_chk_inf(0xA0));
    settle(&mut w);
    // Environment_Update: dayTime += gTimeSpeed while IS_DAY (nightFlag from the last frame),
    // gTimeSpeed * 2 after; skyboxTime follows (gTimeSpeed != 0, dayTime > skyboxTime); nightFlag
    // once dayTime > CLOCK_TIME(18, 0).
    let mut seen_night = false;
    for _ in 0..40 {
        let (before, night_before) = (w.save.day_time, w.save.night_flag);
        idle(&mut w, 1);
        let step = if night_before { 20 } else { 10 };
        assert_eq!(w.save.day_time, before.wrapping_add(step));
        assert_eq!(w.save.skybox_time, w.save.day_time);
        assert_eq!(w.save.night_flag, w.save.day_time > t(18, 0) || w.save.day_time < t(6, 30));
        seen_night |= w.save.night_flag;
    }
    assert!(seen_night);
    // A message open (msgLength, msgMode): the time waits.
    w.start_textbox(0x2064, None);
    idle(&mut w, 1);
    let held = w.save.day_time;
    idle(&mut w, 10);
    assert_eq!(w.save.day_time, held);
}

#[test]
fn a_new_day_counts_hatches_the_egg_and_crows_fifteen_frames_on() {
    let Some(a) = assets() else { return };
    // Play_Init with nextDayTime NEXT_TIME_DAY: dayTime = skyboxTime = 0x8001; after
    // Interface_Init: totalDays and bgsDayCount + 1, dogIsLost, the Weird Egg a Cucco (text
    // 0x3066), nextDayTime NEXT_TIME_DAY_SET.
    let mut w = world(&a, "ENTR_KOKIRI_FOREST_0", t(23, 0), |s| {
        s.next_day_time = NEXT_TIME_DAY;
        s.total_days = 3;
        s.bgs_day_count = 1;
        s.dog_is_lost = false;
        s.inventory.items[oot_game::item::slot(oot_game::item::ITEM_WEIRD_EGG)] = oot_game::item::ITEM_WEIRD_EGG;
    });
    assert_eq!((w.save.day_time, w.save.skybox_time, w.save.night_flag), (NEXT_TIME_DAY, NEXT_TIME_DAY, false));
    assert_eq!((w.save.total_days, w.save.bgs_day_count, w.save.dog_is_lost), (4, 2, true));
    assert_eq!(w.save.inventory.items[oot_game::item::slot(oot_game::item::ITEM_WEIRD_EGG)], oot_game::item::ITEM_CHICKEN);
    assert_eq!(w.msg_ctx.text_id, 0x3066);
    assert_eq!(w.save.next_day_time, NEXT_TIME_DAY_SET);
    // Environment_Update: 0x10 off each frame from 0xFFFE; at 0xFFFE - 15 * 0x10 the cock crows
    // (Sfx_PlaySfxCentered) and nextDayTime is NEXT_TIME_NONE.
    for k in 1..=15u16 {
        idle(&mut w, 1);
        if k < 15 {
            assert_eq!(w.save.next_day_time, NEXT_TIME_DAY_SET - 0x10 * k);
        }
    }
    assert_eq!(w.save.next_day_time, NEXT_TIME_NONE);
    assert!(sfx_on(&w, NA_SE_EV_CHICKEN_CRY_M));

    // A night (NEXT_TIME_NIGHT): dayTime 0, nightFlag, NEXT_TIME_NIGHT_SET; the dog howls 15 frames
    // on (Sfx_PlaySfxCentered2).
    let mut w = world(&a, "ENTR_KOKIRI_FOREST_0", t(12, 0), |s| s.next_day_time = NEXT_TIME_NIGHT);
    assert_eq!((w.save.day_time, w.save.night_flag, w.save.next_day_time), (0, true, NEXT_TIME_NIGHT_SET));
    idle(&mut w, 15);
    assert_eq!(w.save.next_day_time, NEXT_TIME_NONE);
    assert!(sfx_on(&w, NA_SE_EV_DOG_CRY_EVENING));
}

#[test]
fn the_fields_music_stops_after_ten_past_five_and_the_night_comes_in() {
    let Some(a) = assets() else { return };
    let mut w = world(&a, "ENTR_HYRULE_FIELD_0", t(17, 0), |s| s.set_event_chk_inf(0xA0));
    // Environment_PlaySceneSequence at 17:00 (7:00 .. 17:10): the day's music, TIMESEQ_FADE_DAY_BGM.
    assert_eq!(w.time_seq_state, TIMESEQ_FADE_DAY_BGM);
    // Environment_PlayTimeBasedSequence runs before the clock in Environment_Update: a state's
    // change is seen on the frame after the one that passed its time.
    let stop = (1u32 << 28) | ((oot_game::audio::SEQ_PLAYER_BGM_MAIN as u32) << 24) | (240 << 16) | 0xFF;
    let mut log = Vec::new();
    for _ in 0..600 {
        let (before, state) = (w.save.day_time, w.time_seq_state);
        let cmds = w.audio.log.as_ref().unwrap().seq_cmds.len();
        idle(&mut w, 1);
        if w.time_seq_state != state {
            log.push((state, w.time_seq_state, before));
            if state == TIMESEQ_FADE_DAY_BGM {
                // SEQCMD_STOP_SEQUENCE(SEQ_PLAYER_BGM_MAIN, 240).
                assert!(w.audio.log.as_ref().unwrap().seq_cmds[cmds..].iter().any(|&(_, c)| c == stop));
            }
            if state == TIMESEQ_NIGHT_BEGIN_SFX {
                assert!(sfx_on(&w, NA_SE_EV_DOG_CRY_EVENING));
            }
        }
        if w.time_seq_state == TIMESEQ_DAY_BEGIN_SFX {
            break;
        }
    }
    // Each change, with the time it was made at: past 17:10, past 18:00, the next frame, past
    // 19:00, the next frame.
    let states: Vec<(u8, u8)> = log.iter().map(|&(a, b, _)| (a, b)).collect();
    assert_eq!(
        states,
        vec![
            (TIMESEQ_FADE_DAY_BGM, TIMESEQ_NIGHT_BEGIN_SFX),
            (TIMESEQ_NIGHT_BEGIN_SFX, TIMESEQ_EARLY_NIGHT_CRITTERS),
            (TIMESEQ_EARLY_NIGHT_CRITTERS, TIMESEQ_NIGHT_DELAY),
            (TIMESEQ_NIGHT_DELAY, TIMESEQ_NIGHT_CRITTERS),
            (TIMESEQ_NIGHT_CRITTERS, TIMESEQ_DAY_BEGIN_SFX)
        ]
    );
    assert!(log[0].2 > t(17, 10) && log[0].2 - 10 <= t(17, 10));
    assert!(log[1].2 > t(18, 0) && log[1].2 - 10 <= t(18, 0));
    assert!(log[3].2 > t(19, 0) && log[3].2 - 20 <= t(19, 0));
}

#[test]
fn the_sun_and_the_moon_by_the_time_eased_in_cutscenes() {
    let Some(a) = assets() else { return };
    let mut w = world(&a, "ENTR_HYRULE_FIELD_0", t(17, 0), |s| s.set_event_chk_inf(0xA0));
    idle(&mut w, 1);
    let day = w.save.day_time;
    w.environment_draw_sun_and_moon();
    // Environment_DrawSunAndMoon: y = sunPos.y / 25, temp = y / 80; alpha 255 - clamp(temp * 255);
    // color clamp(temp, 0, 1); prim (255, color * 75 + 180, color * 155 + 100, 255), env (255,
    // color * 255, color * 255, alpha); scale color * 2 + 10.
    let p = sun_pos(day);
    let temp = p.y / 25.0 / 80.0;
    let c = temp.clamp(0.0, 1.0);
    let sun = w.env_draw.sun.expect("the sun");
    assert_eq!(sun.pos, p);
    assert_eq!(sun.scale, c * 2.0 + 10.0);
    assert_eq!(sun.prim, [255, (c * 75.0) as u8 + 180, (c * 155.0) as u8 + 100, 255]);
    assert_eq!(sun.env, [255, (c * 255.0) as u8, (c * 255.0) as u8, (255.0 - (temp * 255.0).clamp(0.0, 255.0)) as u8]);
    // The moon (-y / 80 * 255) is under the horizon by day: not drawn.
    assert!(p.y > 0.0 && w.env_draw.moon.is_none());

    // At 23:00 the moon: alpha min(-y / 80, 1) * 255, scale -15 * max(-y / 120, 0) + 25, prim
    // (240, 255, 180, alpha), env (80, 70, 20, alpha), opposite the sun.
    w.save.day_time = t(23, 0);
    w.environment_draw_sun_and_moon();
    let p = sun_pos(t(23, 0));
    let y = p.y / 25.0;
    let alpha = ((-y / 80.0).min(1.0) * 255.0) as u8;
    let moon = w.env_draw.moon.expect("the moon");
    assert_eq!((moon.pos, moon.scale), (-p, -15.0 * (-y / 120.0).max(0.0) + 25.0));
    assert_eq!((moon.prim, moon.env), ([240, 255, 180, alpha], [80, 70, 20, alpha]));

    // In a cutscene the sun eases: Math_SmoothStepToF(x, .., 1, 0.8, 0.8), the same for y, then
    // @bug (game) y again towards z's place: from (0, 0, 0), x and y move 0.8 (y twice), z stays.
    w.cs_ctx.state = 2;
    w.env_ctx.sun_pos = Vec3::ZERO;
    w.save.day_time = t(17, 0);
    w.environment_draw_sun_and_moon();
    let target = sun_pos(t(17, 0));
    let sp = w.env_ctx.sun_pos;
    assert_eq!(sp.x, 0.8 * target.x.signum());
    // y: 0 -> 0.8 (towards y's 776), -> 1.6 (towards z's 129).
    assert_eq!((sp.y, sp.z), (1.6, 0.0));
}

#[test]
fn the_skybox_filters_cover_the_sky_with_fog() {
    let Some(a) = assets() else { return };
    let mut w = world(&a, "ENTR_HYRULE_FIELD_0", t(12, 0), |s| s.set_event_chk_inf(0xA0));
    idle(&mut w, 1);
    let fog = w.scene.as_ref().unwrap().lights.fog_color;
    let filters = |w: &mut PlayState, near: i16| {
        w.scene.as_mut().unwrap().lights.fog_near = near;
        w.environment_draw_skybox_filters();
        w.env_draw.skybox_filters.clone()
    };
    // The normal sky (1): from fogNear 980 no fill; under it alpha (1000 - fogNear) * 0.02, at
    // most 1, times 255.
    assert_eq!(filters(&mut w, 985), Vec::<[u8; 4]>::new());
    assert_eq!(filters(&mut w, 960), vec![[fog[0], fog[1], fog[2], (255.0 * (40.0 * 0.02f32)) as u8]]);
    assert_eq!(filters(&mut w, 900), vec![[fog[0], fog[1], fog[2], 255]]);
    // customSkyboxFilter: its colour after.
    w.env_ctx.custom_skybox_filter = true;
    w.env_ctx.skybox_filter_color = [10, 20, 30, 40];
    assert_eq!(filters(&mut w, 985), vec![[10, 20, 30, 40]]);

    // Kokiri Forest (SKYBOX_UNSET_1D): opaque fog whatever fogNear.
    let mut w = world(&a, "ENTR_KOKIRI_FOREST_0", t(12, 0), |_| {});
    idle(&mut w, 1);
    let fog = w.scene.as_ref().unwrap().lights.fog_color;
    assert_eq!(filters(&mut w, 996), vec![[fog[0], fog[1], fog[2], 255]]);
}

#[test]
fn the_lens_flare_eases_in_at_the_sun_and_out_when_hidden() {
    let Some(a) = assets() else { return };
    let mut w = world(&a, "ENTR_HYRULE_FIELD_0", t(17, 0), |s| s.set_event_chk_inf(0xA0));
    idle(&mut w, 1);
    let day = w.save.day_time;
    w.environment_draw_sun_and_moon();
    // Looking straight at the sun: lookDir and posDir the same, cosAngle 1; the target 1 (3.5
    // clamped).
    let eye = Vec3::new(0.0, 100.0, 2000.0);
    w.view = oot_game::camera::CamView { eye, at: eye + w.env_ctx.sun_pos, fov: 60.0 };
    w.env_ctx.lens_flare_alpha_scale = 0.0;
    w.env_ctx.glare_alpha = 0.0;
    w.env_statics.sun_screen_depth = oot_game::env_draw::ZBUF_MAX;
    w.environment_draw_lens_flares_update();
    let fl = w.env_draw.lens_flares.clone();
    assert_eq!(fl.len(), 10);
    // The pixel read next: the sun at the screen's centre, (160, (s16)120 - 5).
    assert_eq!(w.env_statics.sun_depth_test, [160, 115]);
    // colorIntensity cos(dayTime - 12:00) * 120 over 10, at most 1; times lensFlareAlphas[i]; times
    // 1 - (996 - fogNear) / 50 (at most 1). The scale eases from 0 by 0.05 (0.5 of the gap, at most
    // 0.05) once per flare: 0.05 (i + 1) for the i-th's alpha.
    let ci = eng_math::cos_s(day.wrapping_sub(t(12, 0)) as i16) * 120.0;
    let fog_near = w.scene.as_ref().unwrap().lights.fog_near as f32;
    let fi = ((996.0 - fog_near) / 50.0).min(1.0);
    let alphas = [50u32, 10, 25, 40, 70, 30, 50, 70, 50, 40];
    for (i, f) in fl.iter().enumerate() {
        let alpha = ((ci / 10.0).min(1.0) * alphas[i] as f32).max(0.0) * (1.0 - fi);
        let mut s = 0.0f32;
        for _ in 0..=i {
            eng_math::smooth_step_to_f(&mut s, 1.0, 0.5, 0.05, 0.001);
        }
        assert_eq!(f.color[3], (alpha * s) as u8, "flare {i}");
        // Along the line back from the sun: i twelfths of the distance.
        let dist = w.env_ctx.sun_pos.length() / 12.0;
        assert!((f.offset - (w.env_ctx.sun_pos - w.env_ctx.sun_pos.normalize() * i as f32 * dist)).length() < 0.5);
    }
    assert!(fl[0].ring && fl[1..].iter().all(|f| !f.ring));
    // The glare: glareAlphaScale = cos - (1.5 - cos) = 0.5; its alpha min(ci / 10, 1) * 400 * (1 -
    // fogInfluence) * 0.5, eased by 0.5 of the gap at most 50 from 0.
    let target = (ci / 10.0).min(1.0) * 400.0 * (1.0 - fi) * 0.5;
    let mut g = 0.0f32;
    eng_math::smooth_step_to_f(&mut g, target, 0.5, 50.0, 0.1);
    assert_eq!(w.env_draw.glare.unwrap()[3], g as u8);

    // Hidden (the depth read isn't the far plane's): the scale and the glare ease back to 0.
    w.env_statics.sun_screen_depth = 0;
    let before = w.env_ctx.lens_flare_alpha_scale;
    w.environment_draw_lens_flares_update();
    assert!(w.env_ctx.lens_flare_alpha_scale < before);
    // Looking away (cosAngle < 0): nothing drawn, nothing eased.
    w.view.at = eye - w.env_ctx.sun_pos;
    let (s, g) = (w.env_ctx.lens_flare_alpha_scale, w.env_ctx.glare_alpha);
    w.environment_draw_lens_flares_update();
    assert!(w.env_draw.lens_flares.is_empty() && w.env_draw.glare.is_none());
    assert_eq!((w.env_ctx.lens_flare_alpha_scale, w.env_ctx.glare_alpha), (s, g));
}

fn drawbridge(w: &PlayState) -> ActorHandle {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<BgSpot00Hanebasi>(h).is_some_and(|b| b.actor.params == DT_DRAWBRIDGE)).expect("the drawbridge")
}

#[test]
fn the_drawbridge_rises_at_nightfall_and_layer_5_sets_the_time_speed() {
    let Some(a) = assets() else { return };
    let mut w = world(&a, "ENTR_HYRULE_FIELD_0", t(17, 55), |s| s.set_event_chk_inf(0xA0));
    settle(&mut w);
    let h = drawbridge(&w);
    // BgSpot00Hanebasi_DrawbridgeWait: down by day; a child's night raises it to -0x4000 by
    // Math_ScaledStepToS's 80: 80 * R_UPDATE_RATE (3) * 0.5 = 120 a frame, 137 frames.
    while !w.save.night_flag {
        assert_eq!(w.actors.downcast::<BgSpot00Hanebasi>(h).unwrap().actor.shape_rot.x, 0);
        idle(&mut w, 1);
    }
    let mut n = 0;
    while w.actors.downcast::<BgSpot00Hanebasi>(h).unwrap().actor.shape_rot.x != -0x4000 {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 300);
    }
    // The night seen on frame k is acted on at frame k + 1's update; then 137 steps.
    assert_eq!(n, 1 + 137);

    // Layer 5: a speed of 50 becomes (CLOCK_TIME(20, 0) + 1 - dayTime) / 350; 4:00 .. 4:30 stops it.
    w.save.scene_layer = 5;
    w.save.day_time = t(17, 0);
    w.env_statics.time_speed = 50;
    let mut b = w.actors.take(h).unwrap();
    b.update(&mut w);
    assert_eq!(w.env_statics.time_speed, ((clock_time(20, 0) + 1 - t(17, 0) as i32) as f32 * (1.0 / 350.0)) as u16);
    w.save.day_time = t(4, 10);
    b.update(&mut w);
    assert_eq!(w.env_statics.time_speed, 0);
    w.actors.put_back(h, b);
}

#[test]
fn the_suns_song_speeds_the_field_and_reloads_the_forest_at_midnight() {
    let Some(a) = assets() else { return };
    // Where time passes: SUNSSONG_SPEED_TIME, gTimeSpeed 400 (from 10, kept), played by day
    // (6:30 .. 18:00 + 1); the ocarina's mode 4 (ocarinaAction isn't CHECK_NOWARP_DONE); 400 a
    // frame (not doubled: >= 400) until past 18:00 + 1, then back to 10.
    let mut w = world(&a, "ENTR_HYRULE_FIELD_0", t(17, 0), |s| s.set_event_chk_inf(0xA0));
    settle(&mut w);
    w.save.suns_song_state = SUNSSONG_START;
    let t0 = w.save.day_time;
    idle(&mut w, 1);
    assert_eq!((w.save.suns_song_state, w.env_statics.time_speed, w.env_statics.prev_time_speed, w.msg_ctx.ocarina_mode), (SUNSSONG_SPEED_TIME, 400, 10, 4));
    assert_eq!(w.save.day_time, t0 + 400);
    let mut n = 1;
    while w.save.suns_song_state == SUNSSONG_SPEED_TIME {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 50);
    }
    // Over once a frame starts past CLOCK_TIME(18, 0) + 1: the frames to get there at 400.
    let frames = (t(18, 0) as i32 + 1 - t0 as i32) / 400 + 1;
    assert_eq!(n, frames + 1);
    assert_eq!(w.env_statics.time_speed, 10);

    // Where it stands (Kokiri Forest, sceneTimeSpeed 0, not a dungeon room, sunsSong 0 in its
    // restrictions): by day, the next midnight (NEXT_TIME_NIGHT), the black fade, the scene again
    // (respawnFlag -2), the music off.
    let mut w = world(&a, "ENTR_KOKIRI_FOREST_0", t(12, 0), |_| {});
    settle(&mut w);
    w.save.suns_song_state = SUNSSONG_START;
    idle(&mut w, 1);
    assert_eq!(w.save.next_day_time, NEXT_TIME_NIGHT);
    assert_eq!(w.save.next_transition_type, oot_game::transition::TRANS_TYPE_FADE_BLACK);
    assert!(w.halt_all_actors);
    assert_eq!(w.transition.next_entrance_index, w.save.entrance_index);
    assert_eq!((w.save.suns_song_state, w.save.seq_id), (SUNSSONG_INACTIVE, oot_game::audio::NA_BGM_DISABLED as u8));
    // Play_Init: midnight, night, the dog's howl due.
    for _ in 0..200 {
        idle(&mut w, 1);
        if w.save.next_day_time == NEXT_TIME_NIGHT_SET {
            break;
        }
    }
    assert_eq!(w.save.next_day_time, NEXT_TIME_NIGHT_SET);
    assert!(w.save.night_flag);
}

#[test]
fn exit_hyrule_field_from_dusk_to_night() {
    let Some(a) = assets() else { return };
    let route = Route::Dusk;
    let e = a.scenes.entrance_index(route.entrance()).expect("entrance");
    let save = route.save(e);
    assert_eq!(save.day_time, t(17, 0));
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    w.audio_side = Some(Box::new(oot_game::audio::offline::OfflineAudio::new(&a.pack.audio_data().unwrap(), false)));
    route.debug_start(&mut w);
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    let mut times = Vec::new();
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
        times.push(w.save.day_time);
    }
    assert!(run.failure.is_none(), "{:?}", run.failure);
    eprintln!("steps {:?}", run.steps);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::MusicFaded, Step::Night, Step::BridgeRaised, Step::NightCritters]);
    // The night's step on the first frame past 18:00; the critters' past 19:00.
    let at = |s: Step| run.steps.iter().find(|x| x.0 == s).unwrap().1;
    let night = at(Step::Night);
    assert!(times[night - 1] > t(18, 0) && times[night - 2] <= t(18, 0));
    assert!(times[at(Step::NightCritters) - 1] > t(19, 0));
    // The drawbridge raised; the music stopped; the field's sky the night's.
    assert!(w.save.night_flag);
    assert_eq!(w.time_seq_state, TIMESEQ_DAY_BEGIN_SFX);
}
