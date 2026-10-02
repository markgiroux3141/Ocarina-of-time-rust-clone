//! The environment across frames (`z_kankyo.c`, GAME-04b milestone 4): the cutscenes' misc
//! actions on `envCtx` (the nightmare's rain and lightning), a light setting override's blend,
//! and a lightning strike's flash. Expected values come from `Environment_UpdateRain`,
//! `Environment_DrawLightning`'s bolt states, `Environment_Update`'s light blend and
//! `Environment_UpdateLightningStrike`.

mod common;

use std::sync::Arc;

use eng_input::pad::PadState;
use oot_game::env::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn enter(a: &Arc<GameAssets>, save: SaveContext) -> Option<PlayState> {
    Some(oot_actors::play_entrance(a.clone(), common::data()?, common::rules()?, save).expect("Play_Init"))
}

fn tick(w: &mut PlayState) {
    w.tick_with(scripted_input(PadState::default(), PadState::default()));
}

#[test]
fn the_nightmares_rain_and_lightning() {
    let Some(a) = assets() else { return };
    // Hyrule Field's layer 4 (the nightmare), as the narration's terminator enters it.
    let mut save = SaveContext::file_select_new();
    save.entrance_index = a.scenes.entrance_index("ENTR_SPOT00_0").unwrap();
    save.cutscene_index = 0xFFF0;
    let Some(mut w) = enter(&a, save) else { return };
    assert_eq!(w.save.scene_layer, 4);
    // Misc 1 at the script's frame 0 (run with frame 1 on its first tick): 20 drops at most;
    // Environment_UpdateRain then adds 2 on every frame that's a multiple of 8.
    tick(&mut w);
    assert_eq!(w.env_ctx.precipitation[PRECIP_RAIN_MAX], 20);
    let mut last = w.env_ctx.precipitation[PRECIP_RAIN_CUR];
    while w.cs_ctx.frames < 19 {
        tick(&mut w);
        let cur = w.env_ctx.precipitation[PRECIP_RAIN_CUR];
        let want = if w.gameplay_frames % 8 == 0 && last < 20 { last + 2 } else { last };
        assert_eq!(cur, want, "frame {}", w.gameplay_frames);
        last = cur;
    }
    // Misc 2 at 20: three bolts started (Environment_AddLightningBolts(play, 3)); the strike's
    // state START, but lightningState is off here (no En_Weather_Tag), so no flash runs.
    tick(&mut w);
    assert_eq!(w.cs_ctx.frames, 20);
    let started = w.gameplay_frames;
    assert!(w.env_statics.lightning_bolts.iter().all(|b| b.state == LIGHTNING_BOLT_WAIT));
    assert_eq!(w.env_statics.lightning_strike.state, LIGHTNING_STRIKE_START);
    let eye = w.view.eye;
    for (i, b) in w.env_statics.lightning_bolts.iter().enumerate() {
        // 9500 ahead of the view's eye (along the view's xz), 4000 to 5000 up, waiting 3 frames
        // per slot.
        let ahead = b.pos - eye;
        assert!(((ahead.x * ahead.x + ahead.z * ahead.z).sqrt() - 9500.0).abs() < 1.0);
        assert!((4000.0..=5000.0).contains(&b.pos.y));
        assert_eq!(b.delay_timer, 3 * (i as u8 + 1));
    }
    // Bolt i is drawn from frame started + 3 (i + 1), its 8 textures, one a frame. The misc 2
    // at 24 finds no free slot.
    let mut drawn: Vec<Vec<(u32, u8)>> = vec![Vec::new(); 3];
    for _ in 0..20 {
        tick(&mut w);
        let f = w.gameplay_frames;
        for b in &w.lightning_bolts {
            let i = w.env_statics.lightning_bolts.iter().position(|x| x.pos == b.pos).unwrap();
            drawn[i].push((f, b.texture_index));
        }
    }
    for (i, d) in drawn.iter().enumerate() {
        let first = started + 3 * (i as u32 + 1);
        let want: Vec<(u32, u8)> = (0..8).map(|t| (first + t as u32, t)).collect();
        assert_eq!(d, &want, "bolt {i}");
    }
}

#[test]
fn a_light_setting_override_blends_in() {
    let Some(a) = assets() else { return };
    // The Deku Tree (LIGHT_MODE_SETTINGS, setting 0; its intro seen).
    let mut save = SaveContext::new(a.scenes.entrance_index("ENTR_YDAN_0").unwrap(), false, oot_game::env::clock_time(10, 0) as u16);
    save.set_event_chk_inf(0xA8);
    let Some(mut w) = enter(&a, save) else { return };
    tick(&mut w);
    let list = w.scene.as_ref().unwrap().layer_data().light_settings.clone();
    assert_eq!(w.env_ctx.light_mode, LIGHT_MODE_SETTINGS);
    assert!(list.len() > 1);
    let base = w.scene.as_ref().unwrap().lights;
    assert_eq!(base.ambient, list[0].ambient);
    // Cutscene_Command_SetLighting's override (setting 1): the next Environment_Update switches
    // (prevLightSetting 0, lightBlend 0) and blends by the new setting's rate (fogNear >> 10,
    // times 4, at least 1) / 255 a frame.
    w.env_ctx.light_setting_override = 1;
    w.env_ctx.light_blend = 1.0;
    let rate = (((list[1].fog_near_raw >> 10) * 4) as u8).max(1) as f32 / 255.0;
    let mut blend = 0.0f32;
    for _ in 0..4 {
        tick(&mut w);
        blend = (blend + rate).min(1.0);
        let l = w.scene.as_ref().unwrap().lights;
        let want = [0, 1, 2].map(|j| ((list[1].ambient[j] as f32 - list[0].ambient[j] as f32) * blend + list[0].ambient[j] as f32) as u8);
        assert_eq!((w.env_ctx.prev_light_setting, w.env_ctx.light_setting), (0, 1));
        assert_eq!(l.ambient, want, "blend {blend}");
    }
}

#[test]
fn a_lightning_strike_flashes_the_ambient_light() {
    let Some(a) = assets() else { return };
    let save = SaveContext::new(a.scenes.entrance_index("ENTR_SPOT04_0").unwrap(), false, oot_game::env::clock_time(10, 0) as u16);
    let Some(mut w) = enter(&a, save) else { return };
    tick(&mut w);
    let base = w.scene.as_ref().unwrap().lights.ambient;
    // With lightning on (an En_Weather_Tag's), a strike started: the flash fades in by 100 a
    // frame to 200, the ambient light up by (80, 80, 100) each of those frames; then down by 10
    // a frame (by 10 on the first two channels while the first is positive) to 0.
    w.env_ctx.lightning_state = 1;
    w.env_statics.lightning_strike.state = LIGHTNING_STRIKE_START;
    w.env_statics.lightning_strike.flash_alpha_target = 200;
    tick(&mut w);
    assert_eq!(w.env_ctx.adj_ambient_color, [80, 80, 100]);
    assert_eq!(w.lightning_flash, Some([200, 200, 255, 100]));
    tick(&mut w);
    assert_eq!((w.env_ctx.adj_ambient_color, w.env_statics.lightning_strike.state), ([160, 160, 200], LIGHTNING_STRIKE_END));
    // The lights this frame were computed before the strike's update (Environment_Update
    // comes first): the ambient raised by the first frame's (80, 80, 100), clamped to 255.
    let l = w.scene.as_ref().unwrap().lights.ambient;
    assert_eq!(l, [0, 1, 2].map(|j| (base[j] as i32 + [80, 80, 100][j]).min(255) as u8));
    let mut n = 0;
    while w.env_statics.lightning_strike.state != LIGHTNING_STRIKE_WAIT {
        tick(&mut w);
        n += 1;
        assert!(n < 40);
    }
    // 200 down to 0 by 10: 20 frames; the adjustments back to 0.
    assert_eq!(n, 20);
    assert_eq!(w.env_ctx.adj_ambient_color, [0, 0, 0]);
    assert_eq!(w.lightning_flash, None);
}

#[test]
fn entering_kokiri_forest_shows_its_place_name() {
    let Some(a) = assets() else { return };
    // An entrance into Kokiri Forest (SCENE_SPOT04) whose entrance table row shows the title card
    // (ENTRANCE_INFO_DISPLAY_TITLE_CARD_FLAG).
    let e = a.scenes.entrances.iter().position(|e| e.scene == 0x55 && e.title_card).expect("an entrance with the title card") as u16;
    let save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    let Some(mut w) = enter(&a, save) else { return };
    // Player_Init: TitleCard_InitPlaceName(play, .., 160, 120, 144, 24, 20) with Kokiri Forest's
    // title file (scene_table.h: g_pn_31).
    assert_eq!((w.title_ctx.texture.as_deref(), w.title_ctx.delay_timer, w.title_ctx.duration_timer), (Some("title/g_pn_31"), 20, 80));
    // TitleCard_Update once a frame: 20 frames of delay (the 20th already counts the duration),
    // then in by 10 (to 255) for 79 frames, then out by 30.
    let mut alphas = Vec::new();
    for _ in 0..120 {
        tick(&mut w);
        alphas.push(w.title_ctx.alpha);
    }
    let mut want = Vec::new();
    let mut a_ = 0i16;
    for k in 1..=120 {
        if k >= 20 && k < 20 + 79 {
            a_ = (a_ + 10).min(255);
        } else if k >= 20 + 79 {
            a_ = (a_ - 30).max(0);
        }
        want.push(a_);
    }
    assert_eq!(alphas, want);
    // Drawn while its alpha isn't 0: gSPTextureRectangle(160 * 4 - 144 * 2, 120 * 4 - 24 * 2,
    // + 144 * 4 - 4, + 24 * 4 - 1): (88, 108) to (231, 131.75), grey at the intensity.
    let mut s = Vec::new();
    w.title_ctx.alpha = 100;
    w.title_ctx.intensity = 40;
    w.title_ctx.draw(&mut s);
    assert_eq!(s.len(), 1);
    assert_eq!((s[0].name.as_str(), s[0].prim), ("title/g_pn_31", Some([40, 40, 40, 100])));
    assert_eq!(s[0].transform, oot_game::sprite::rect_transform(88.0, 108.0, 231.0, 131.75));
}
