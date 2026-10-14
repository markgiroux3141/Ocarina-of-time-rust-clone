//! GAME-07 milestone 6 against the C: the graveyard's thunderstorm (`En_Weather_Tag`,
//! `z_en_weather_tag.c`) and the shield grave's bombable wall (`Bg_Breakwall`,
//! `z_bg_breakwall.c`).
//!
//! - The graveyard (`ootx scene-info --scene spot02_scene`, layers 0 and 1, the child's): one tag,
//!   params 0x0607 at (728, 181, 80): type 7, range `(0x0607 >> 8) × 100` = 600.
//! - The ruined market entrance (`enrui_scene`, the adult's): one tag, params 0x0200 at (0, 0,
//!   3060): type 0 (the cloudy market), range 200.
//! - The shield grave (`hakaana2_scene` room 0): `Bg_Breakwall` params 0xA001 at (-28, 20, 386),
//!   turned 0: type `(0xA001 >> 13) & 3` = 1 (a wall), switch flag 1, bit 15 set (the chime).
//!
//! Link has no bombs yet: the tests put a hit on the wall's quad, as `CollisionCheck_AC` leaves
//! it, or bring in an explosion's cylinder (an injected actor's AT) for the collision check to
//! find. Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_breakwall::{self as bw, ACTOR_BG_BREAKWALL, BgBreakwall, NA_SE_EV_EXPLOSION};
use oot_actors::en_a_obj::{A_OBJ_BOULDER_FRAGMENT, EnAObj};
use oot_actors::en_weather_tag::{self as wt, ACTOR_EN_WEATHER_TAG, EnWeatherTag};
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTORCAT_EXPLOSIVE, ActorHandle, ActorImpl};
use oot_game::audio::bgm::seq_cmd8;
use oot_game::audio::sfx::{NA_SE_EV_WALL_BROKEN, NA_SE_SY_CORRECT_CHIME, NA_SE_SY_TRE_BOX_APPEAR, SfxPos};
use oot_game::audio::{CHANNEL_IO_PORT_1, NATURE_CHANNEL_LIGHTNING, NATURE_CHANNEL_RAIN, SEQ_PLAYER_BGM_MAIN};
use oot_game::collision_check::{self as cc, ColliderCylinder, ColliderCylinderInit, ColliderMut};
use oot_game::effect::EFFECT_SS_DUST;
use oot_game::env::*;
use oot_game::play::{PlayState, Rand, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::sys_matrix::MtxF;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// Link (a child, or an adult with `adult`) entered at `entrance` at `hour`:00, `edit` applied
/// to the save last, the sound log on; `frames` frames on.
fn enter(a: &Arc<GameAssets>, entrance: &str, adult: bool, hour: i32, frames: usize, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, adult, oot_game::env::clock_time(hour, 0) as u16);
    edit(&mut save);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, frames);
    w
}

fn all<T: 'static + ActorImpl>(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<T>(h).is_some_and(|a| !a.base().killed)).collect()
}

fn one<T: 'static + ActorImpl>(w: &PlayState) -> ActorHandle {
    let v = all::<T>(w);
    assert_eq!(v.len(), 1, "one {}", std::any::type_name::<T>());
    v[0]
}

/// Link on the floor below `pos` (from 100 above), facing `yaw`.
fn place_link(w: &mut PlayState, pos: Vec3, yaw: i16) {
    let (y, _) = w.col.entity_raycast_down(pos + Vec3::Y * 100.0);
    assert!(y > -1000.0, "a floor under {pos:?}");
    w.place_player(Vec3::new(pos.x, y, pos.z), yaw);
}

/// `h`'s update run on its own (`Actor_UpdateAll`'s call), the actor the updating one.
fn update_alone(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).unwrap();
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

fn seq_cmds_on(w: &PlayState, frame: u32) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().seq_cmds.iter().filter(|&&(f, _)| f == frame).map(|&(_, c)| c).collect()
}

fn sfx_on(w: &PlayState, frame: u32) -> Vec<(u16, SfxPos)> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(f, _, _)| f == frame).map(|&(_, s, p)| (s, p)).collect()
}

// ---------------------------------------------------------------------------------------------
// En_Weather_Tag
// ---------------------------------------------------------------------------------------------

const GRAVEYARD_TAG: Vec3 = Vec3::new(728.0, 181.0, 80.0);
/// `QUEST_MEDALLION_SHADOW` (`item.h`: 0x04).
const QUEST_MEDALLION_SHADOW: u32 = 0x04;

#[test]
fn each_types_init_kills_it_by_its_flags() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_GRAVEYARD_0", false, 10, 2, |_| {});
    let at = Vec3::new(-1411.0, 0.0, 330.0);
    let spawn = |w: &mut PlayState, ty: i16| -> (bool, wt::Action) {
        let h = w.actor_spawn(ACTOR_EN_WEATHER_TAG, at, [0, 0, 0], 0x0100 | ty).expect("En_Weather_Tag");
        let t = w.actors.downcast::<EnWeatherTag>(h).unwrap();
        // EnWeatherTag_Init: ACTOR_FLAG_ATTENTION_ENABLED off, whatever the type.
        assert_eq!(t.actor.flags & oot_game::actor::ACTOR_FLAG_ATTENTION_ENABLED, 0);
        (t.actor.killed, t.action)
    };
    // No flags: the market's, Zora's Domain's, Lake Hylia's, Death Mountain's, the sandstorm's
    // and the graveyard's stay; the ranch's stays (Epona not Link's); Kakariko's goes (it needs
    // EVENTCHKINF_48, _49 and _4A all set).
    use wt::Action::*;
    assert_eq!(spawn(&mut w, 0), (false, DisabledCloudyHyruleMarket));
    assert_eq!(spawn(&mut w, 1), (false, DisabledCloudyLonLonRanch));
    assert_eq!(spawn(&mut w, 2), (false, DisabledCloudySnow));
    assert_eq!(spawn(&mut w, 3), (false, DisabledRainLakeHylia));
    assert_eq!(spawn(&mut w, 4), (false, DisabledCloudyDeathMountain));
    assert_eq!(spawn(&mut w, 5), (true, DisabledCloudyRainThunderKakariko));
    assert_eq!(spawn(&mut w, 6), (false, SetSandstormIntensity));
    assert_eq!(spawn(&mut w, 7), (false, DisabledRainThunder));
    // EVENTCHKINF_EPONA_OBTAINED (0x18): the ranch's goes.
    w.save.set_event_chk_inf(wt::EVENTCHKINF_EPONA_OBTAINED);
    assert!(spawn(&mut w, 1).0);
    // EVENTCHKINF_49: Death Mountain's goes; Kakariko's still (48 and 4A clear).
    w.save.set_event_chk_inf(wt::EVENTCHKINF_49);
    assert!(spawn(&mut w, 4).0);
    assert!(spawn(&mut w, 5).0);
    // EVENTCHKINF_4A: Zora's Domain's and Lake Hylia's go; Kakariko's still (48 clear).
    w.save.set_event_chk_inf(wt::EVENTCHKINF_4A);
    assert!(spawn(&mut w, 2).0);
    assert!(spawn(&mut w, 3).0);
    assert!(spawn(&mut w, 5).0);
    // All three: Kakariko's storm stays, until the Shadow Medallion.
    w.save.set_event_chk_inf(wt::EVENTCHKINF_48);
    assert!(!spawn(&mut w, 5).0);
    w.save.inventory.quest_items |= 1 << QUEST_MEDALLION_SHADOW;
    assert!(spawn(&mut w, 5).0);
    // The market's, the sandstorm's and the graveyard's read no flag.
    assert!(!spawn(&mut w, 0).0);
    assert!(!spawn(&mut w, 6).0);
    assert!(!spawn(&mut w, 7).0);
}

/// The point `d` across from the graveyard's tag towards spawn 5 (152, 120, 281), whose floor the
/// path keeps.
fn from_tag(d: f32) -> Vec3 {
    let dir = (Vec3::new(152.0, 0.0, 281.0) - Vec3::new(GRAVEYARD_TAG.x, 0.0, GRAVEYARD_TAG.z)).normalize();
    GRAVEYARD_TAG + dir * d
}

#[test]
fn the_graveyards_tag_rains_within_600_and_stops_past_610() {
    let Some(a) = assets() else { return };
    // The child's graveyard by day (layer 0), from Kakariko (spawn 0, (-1411, 0, 330): 2153
    // across from the tag).
    let mut w = enter(&a, "ENTR_GRAVEYARD_0", false, 10, 3, |_| {});
    assert_eq!(w.save.scene_layer, 0);
    let h = one::<EnWeatherTag>(&w);
    let t = w.actors.downcast::<EnWeatherTag>(h).unwrap();
    assert_eq!((t.actor.params, t.actor.home_pos), (0x0607, GRAVEYARD_TAG));
    // Type 7, WEATHER_TAG_RANGE100(0x0607) = 6 × 100.
    assert_eq!((t.ty(), t.range(), t.action), (wt::EN_WEATHER_TAG_TYPE_THUNDERSTORM_GRAVEYARD, 600.0, wt::Action::DisabledRainThunder));
    assert_eq!((w.env_ctx.precipitation[PRECIP_RAIN_MAX], w.env_ctx.lightning_state), (0, LIGHTNING_OFF));

    // 605 across: outside 600, nothing yet.
    place_link(&mut w, from_tag(605.0), 0);
    idle(&mut w, 1);
    assert_eq!(w.actors.downcast::<EnWeatherTag>(h).unwrap().action, wt::Action::DisabledRainThunder);
    assert_eq!(w.env_ctx.precipitation[PRECIP_RAIN_MAX], 0);

    // 595 across: EnWeatherTag_DisabledRainThunder, whatever the sky:
    // Environment_PlayStormNatureAmbience (the scene's own ambience, NATURE_ID 0x02, restarted;
    // its rain and lightning channels on), LIGHTNING_ON, precipitation[PRECIP_RAIN_MAX] 25.
    place_link(&mut w, from_tag(595.0), 0);
    idle(&mut w, 1);
    let frame = w.audio.frames;
    assert_eq!(w.actors.downcast::<EnWeatherTag>(h).unwrap().action, wt::Action::EnabledRainThunder);
    assert_eq!((w.env_ctx.precipitation[PRECIP_RAIN_MAX], w.env_ctx.lightning_state), (25, LIGHTNING_ON));
    let cmds = seq_cmds_on(&w, frame);
    let rain_on = seq_cmd8(SEQ_PLAYER_BGM_MAIN, CHANNEL_IO_PORT_1, NATURE_CHANNEL_RAIN, 1);
    let lightning_on = seq_cmd8(SEQ_PLAYER_BGM_MAIN, CHANNEL_IO_PORT_1, NATURE_CHANNEL_LIGHTNING, 1);
    let rain_at = cmds.iter().position(|&c| c == rain_on).expect("the rain channel on");
    assert_eq!(cmds.get(rain_at + 1), Some(&lightning_on), "then the lightning channel");
    // The weather tag doesn't touch the weather mode or the Song of Storms' interruption.
    assert_eq!((w.env_statics.weather_mode, w.env_statics.interrupt_song_of_storms), (WEATHER_MODE_CLEAR, false));

    // The rain comes in: Environment_UpdateRain, 2 drops every 8 frames towards 25.
    idle(&mut w, 40);
    let cur = w.env_ctx.precipitation[PRECIP_RAIN_CUR];
    assert!((8..=12).contains(&cur), "{cur} drops after 41 frames");

    // 605 across again: within 600 + 10, still raining.
    place_link(&mut w, from_tag(605.0), 0);
    idle(&mut w, 1);
    assert_eq!(w.actors.downcast::<EnWeatherTag>(h).unwrap().action, wt::Action::EnabledRainThunder);
    assert_eq!(w.env_ctx.precipitation[PRECIP_RAIN_MAX], 25);

    // 615 across: EnWeatherTag_EnabledRainThunder: Environment_StopStormNatureAmbience (the
    // channels off), LIGHTNING_LAST, precipitation[PRECIP_RAIN_MAX] 0 and [PRECIP_RAIN_CUR] 10
    // (then Environment_UpdateRain, later in the frame, takes 2 off on a frame that's a multiple
    // of 8).
    place_link(&mut w, from_tag(615.0), 0);
    idle(&mut w, 1);
    let frame = w.audio.frames;
    assert_eq!(w.actors.downcast::<EnWeatherTag>(h).unwrap().action, wt::Action::DisabledRainThunder);
    assert_eq!(w.env_ctx.precipitation[PRECIP_RAIN_MAX], 0);
    let expect_cur = if w.gameplay_frames % 8 == 0 { 8 } else { 10 };
    assert_eq!(w.env_ctx.precipitation[PRECIP_RAIN_CUR], expect_cur);
    // LIGHTNING_LAST lasts until a strike under way ends (Environment_UpdateLightningStrike).
    let ls = w.env_ctx.lightning_state;
    assert!(ls == LIGHTNING_LAST || (ls == LIGHTNING_OFF && w.env_statics.lightning_strike.state == LIGHTNING_STRIKE_WAIT), "lightning {ls}");
    let cmds = seq_cmds_on(&w, frame);
    assert!(cmds.contains(&seq_cmd8(SEQ_PLAYER_BGM_MAIN, CHANNEL_IO_PORT_1, NATURE_CHANNEL_RAIN, 0)), "the rain channel off");
    assert!(cmds.contains(&seq_cmd8(SEQ_PLAYER_BGM_MAIN, CHANNEL_IO_PORT_1, NATURE_CHANNEL_LIGHTNING, 0)), "the lightning channel off");
    println!("graveyard tag: rain on at 595, off at 615 (cur {expect_cur})");
}

#[test]
fn the_child_graveyards_night_layer_has_the_same_tag() {
    let Some(a) = assets() else { return };
    let w = enter(&a, "ENTR_GRAVEYARD_0", false, 22, 3, |_| {});
    assert_eq!(w.save.scene_layer, 1);
    let h = one::<EnWeatherTag>(&w);
    let t = w.actors.downcast::<EnWeatherTag>(h).unwrap();
    assert_eq!((t.actor.params, t.actor.home_pos), (0x0607, GRAVEYARD_TAG));
}

#[test]
fn the_ruined_markets_tag_asks_for_the_cloudy_sky_and_back() {
    let Some(a) = assets() else { return };
    // The adult's market entrance (ENTR_MARKET_ENTRANCE_DAY_0 + 2: the ruins), spawn 0 at (-1, 0,
    // 2189): 871 across from the tag at (0, 0, 3060), range 2 × 100.
    let mut w = enter(&a, "ENTR_MARKET_ENTRANCE_DAY_0", true, 10, 3, |_| {});
    let h = one::<EnWeatherTag>(&w);
    let tag = w.actors.downcast::<EnWeatherTag>(h).unwrap();
    assert_eq!((tag.actor.params, tag.ty(), tag.range()), (0x0200, wt::EN_WEATHER_TAG_TYPE_CLOUDY_MARKET, 200.0));
    let tag_pos = tag.actor.world_pos;
    assert_eq!(w.env_statics.weather_mode, WEATHER_MODE_CLEAR);

    // Link 150 in, with the lights free (WeatherTag_CheckEnableWeatherEffect's conditions: no
    // storm asked for, light config not 1 and no change under way). The tag's update on its own:
    // weather mode WEATHER_MODE_CLOUDY_CONFIG3, the skybox change asked for (config 0 to 1) and
    // the light config change (0 to 3), both over 60 frames; gLightConfigAfterUnderwater 3;
    // gInterruptSongOfStorms set, then cleared again (OOT_VERSION >= PAL_1_0).
    w.place_player(tag_pos + Vec3::new(0.0, 0.0, -150.0), 0);
    w.env_ctx.storm_request = STORM_REQUEST_NONE;
    w.env_ctx.light_config = 0;
    w.env_ctx.change_light_next_config = 0;
    w.env_ctx.change_light_enabled = false;
    w.env_ctx.change_skybox_state = CHANGE_SKYBOX_INACTIVE;
    update_alone(&mut w, h);
    let e = &w.env_ctx;
    assert_eq!(w.env_statics.weather_mode, wt::WEATHER_MODE_CLOUDY_CONFIG3);
    assert_eq!((e.change_skybox_state, e.skybox_config, e.change_skybox_next_config, e.change_skybox_timer), (CHANGE_SKYBOX_REQUESTED, 0, 1, 60));
    assert_eq!((e.change_light_enabled, e.light_config, e.change_light_next_config, e.change_duration, e.change_light_timer), (true, 0, 3, 60, 60));
    assert_eq!(w.env_statics.light_config_after_underwater, 3);
    assert!(!w.env_statics.interrupt_song_of_storms);
    assert_eq!(w.actors.downcast::<EnWeatherTag>(h).unwrap().action, wt::Action::EnabledCloudyHyruleMarket);

    // Link 250 across: not past 200 + 100, nothing.
    w.place_player(tag_pos + Vec3::new(0.0, 0.0, -250.0), 0);
    w.env_ctx.change_light_enabled = false;
    let before = w.env_ctx.clone();
    update_alone(&mut w, h);
    assert_eq!(w.env_ctx, before);

    // 350 across, the light change done: WeatherTag_CheckRestoreWeather: clear weather, the sky
    // back (1 to 0) and the lights (3 to 0) over 60; back to the disabled action.
    w.place_player(tag_pos + Vec3::new(0.0, 0.0, -350.0), 0);
    w.env_ctx.light_config = 3;
    w.env_ctx.change_light_next_config = 3;
    update_alone(&mut w, h);
    let e = &w.env_ctx;
    assert_eq!(w.env_statics.weather_mode, WEATHER_MODE_CLEAR);
    assert_eq!((e.change_skybox_state, e.skybox_config, e.change_skybox_next_config, e.change_skybox_timer), (CHANGE_SKYBOX_REQUESTED, 1, 0, 60));
    assert_eq!((e.change_light_enabled, e.light_config, e.change_light_next_config, e.change_duration, e.change_light_timer), (true, 3, 0, 60, 60));
    assert_eq!(w.env_statics.light_config_after_underwater, 0);
    assert_eq!(w.actors.downcast::<EnWeatherTag>(h).unwrap().action, wt::Action::DisabledCloudyHyruleMarket);

    // The real frames from here, and Link back in: the cloudy sky asked for again.
    idle(&mut w, 70);
    // The ruins' lights aren't time-based (LIGHT_MODE_SETTINGS): Environment_UpdateLights never
    // runs the change (changeLightEnabled stays), and WeatherTag_CheckEnableWeatherEffect's
    // `lightMode != LIGHT_MODE_TIME` lets the tag change the weather regardless.
    assert_eq!((w.env_ctx.light_mode, w.env_ctx.change_light_enabled), (LIGHT_MODE_SETTINGS, true));
    w.place_player(tag_pos + Vec3::new(0.0, 0.0, -150.0), 0);
    idle(&mut w, 1);
    assert_eq!(w.env_statics.weather_mode, wt::WEATHER_MODE_CLOUDY_CONFIG3);
    assert_eq!(w.actors.downcast::<EnWeatherTag>(h).unwrap().action, wt::Action::EnabledCloudyHyruleMarket);
}

#[test]
fn a_places_weather_interrupts_the_song_of_storms_and_hurries_a_changing_sky() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_MARKET_ENTRANCE_DAY_0", true, 10, 3, |_| {});
    let h = one::<EnWeatherTag>(&w);
    let tag_pos = w.actors.actor(h).unwrap().world_pos;
    w.place_player(tag_pos + Vec3::new(0.0, 0.0, -150.0), 0);

    // The Song of Storms' storm asked for (En_Okarina_Effect: STORM_REQUEST_START): the sky can
    // change (not time-based lights), so gInterruptSongOfStorms is set; the storm keeps the
    // lights, so no change, and it stays set (the storm's En_Okarina_Effect then ends at once).
    w.env_ctx.light_mode = LIGHT_MODE_SETTINGS;
    w.env_ctx.storm_request = STORM_REQUEST_START;
    w.env_statics.interrupt_song_of_storms = false;
    update_alone(&mut w, h);
    assert!(w.env_statics.interrupt_song_of_storms);
    assert_eq!(w.env_statics.weather_mode, WEATHER_MODE_CLEAR);
    assert_eq!(w.actors.downcast::<EnWeatherTag>(h).unwrap().action, wt::Action::DisabledCloudyHyruleMarket);

    // The storm over: cleared, and the weather changes.
    w.env_ctx.storm_request = STORM_REQUEST_NONE;
    update_alone(&mut w, h);
    assert!(!w.env_statics.interrupt_song_of_storms);
    assert_eq!(w.env_statics.weather_mode, wt::WEATHER_MODE_CLOUDY_CONFIG3);

    // Back to the disabled action, with time-based lights whose normal sky is between two
    // textures (gSkyboxIsChanging), and the lights' config not the one changed to: no change;
    // with gTimeSpeed not 0, the day 20 further on (gSaveContext.save.dayTime += 20).
    let mut w = enter(&a, "ENTR_MARKET_ENTRANCE_DAY_0", true, 10, 3, |_| {});
    let h = one::<EnWeatherTag>(&w);
    w.place_player(tag_pos + Vec3::new(0.0, 0.0, -150.0), 0);
    w.env_ctx.light_mode = LIGHT_MODE_TIME;
    w.env_statics.skybox_is_changing = true;
    w.skybox_ctx.skybox_id = oot_game::skybox::SKYBOX_NORMAL_SKY;
    w.env_statics.time_speed = 10;
    let day = w.save.day_time;
    update_alone(&mut w, h);
    assert_eq!(w.save.day_time, day.wrapping_add(20));
    assert_eq!(w.env_statics.weather_mode, WEATHER_MODE_CLEAR);
    // gTimeSpeed 0: the day stands.
    w.env_statics.time_speed = 0;
    update_alone(&mut w, h);
    assert_eq!(w.save.day_time, day.wrapping_add(20));
}

#[test]
fn the_sandstorm_tag_eases_the_fog_in_and_out() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_GRAVEYARD_0", false, 10, 2, |_| {});
    let p = w.player().actor.world_pos;
    let h = w.actor_spawn(ACTOR_EN_WEATHER_TAG, p + Vec3::new(50.0, 0.0, 0.0), [0, 0, 0], 0x0106).expect("En_Weather_Tag");
    w.env_ctx.adj_fog_near = 0;
    w.env_ctx.adj_fog_far = 0;
    // Within 100: Math_SmoothStepToS(adjFogNear, -80, 1, 2, 1): a step of -80 / 1 capped at 2;
    // adjZFar towards -2000 by 50.
    update_alone(&mut w, h);
    assert_eq!((w.env_ctx.adj_fog_near, w.env_ctx.adj_fog_far), (-2, -50));
    for _ in 0..50 {
        update_alone(&mut w, h);
    }
    // 51 frames: the fog near at -80 after 40, the far plane at -2000 after 40.
    assert_eq!((w.env_ctx.adj_fog_near, w.env_ctx.adj_fog_far), (-80, -2000));
    // Outside: back by 1 and 25.
    w.actors.actor_mut(h).unwrap().world_pos = p + Vec3::new(500.0, 0.0, 0.0);
    update_alone(&mut w, h);
    assert_eq!((w.env_ctx.adj_fog_near, w.env_ctx.adj_fog_far), (-79, -1975));
}

// ---------------------------------------------------------------------------------------------
// Bg_Breakwall
// ---------------------------------------------------------------------------------------------

const WALL: Vec3 = Vec3::new(-28.0, 20.0, 386.0);
/// `SCENE_GRAVE_WITH_FAIRYS_FOUNTAIN` (`scene_table.h`: 0x40).
const SCENE_GRAVE_WITH_FAIRYS_FOUNTAIN: usize = 0x40;

/// The shield grave (`ENTR_GRAVE_WITH_FAIRYS_FOUNTAIN_0`, spawn 0 at (-23, 73, 1080)), as a
/// child; 6 frames on (the room's objects load, then `BgBreakwall_WaitForObject`).
fn grave(a: &Arc<GameAssets>, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    enter(a, "ENTR_GRAVE_WITH_FAIRYS_FOUNTAIN_0", false, 10, 6, edit)
}

fn wall(w: &PlayState, h: ActorHandle) -> &BgBreakwall {
    w.actors.downcast::<BgBreakwall>(h).expect("Bg_Breakwall")
}

fn near(a: Vec3, b: Vec3) -> bool {
    (a - b).abs().max_element() < 0.01
}

#[test]
fn the_shield_graves_wall_stands_and_blocks_the_way() {
    let Some(a) = assets() else { return };
    let mut w = grave(&a, |_| {});
    let h = one::<BgBreakwall>(&w);
    let b = wall(&w, h);
    // BgBreakwall_Init: 0xA001: type (0xA001 >> 13) & 3 = 1 (BWALL_WALL), switch flag 0xA001 & 0x3F
    // = 1, not set; scale 0.1; object_bwall_DL_000040, colType 0 (rot.x stays 0); its quad.
    assert_eq!((b.actor.params as u16, b.wall_type(), b.switch_flag()), (0xA001, bw::BWALL_WALL, 1));
    assert_eq!((b.actor.world_pos, b.actor.scale), (WALL, Vec3::splat(0.1)));
    assert_eq!((b.bombable_wall_dlist, b.col_type, b.actor.world_rot.x), (Some("object_bwall_DL_000040"), 0, 0));
    // BgBreakwall_WaitForObject has run (object_bwall is in the room's list): drawn, culled
    // (ACTOR_FLAG_UPDATE_CULLING_DISABLED off), object_bwall_Col_000118 a bg actor, waiting.
    assert_eq!(b.action, bw::Action::Wait);
    assert!(b.drawn);
    assert_eq!(b.actor.flags & oot_game::actor::ACTOR_FLAG_UPDATE_CULLING_DISABLED, 0);
    assert!(w.col.dyna.is_bg_actor(b.bg));
    // sQuadInit: AC_ON | AC_TYPE_PLAYER | AC_TYPE_OTHER, taking 0x48.
    let c = &b.collider;
    assert_eq!(c.base.ac_flags & !cc::AC_HIT, cc::AC_ON | cc::AC_TYPE_PLAYER | cc::AC_TYPE_OTHER);
    assert_eq!((c.base.at_flags, c.base.oc_flags1, c.base.oc_flags2), (cc::AT_NONE, cc::OC1_NONE, cc::OC2_TYPE_2));
    assert_eq!(c.info.ac_dmg_info.dmg_flags, 0x48);
    // BgBreakwall_Draw's quad: sColQuadList[0] through the model matrix (at its position, turned
    // 0, scale 0.1): (±800, 1600 or 0, 100) / 10 from (-28, 20, 386).
    let q = c.dim.quad;
    let expect = [Vec3::new(52.0, 180.0, 396.0), Vec3::new(-108.0, 180.0, 396.0), Vec3::new(52.0, 20.0, 396.0), Vec3::new(-108.0, 20.0, 396.0)];
    for i in 0..4 {
        assert!(near(q[i], expect[i]), "quad {i}: {:?}", q[i]);
    }

    // Its collision: a line through it from in front stops at its face, on its bg actor.
    let (hit, poly) = w.col.entity_line_test(Vec3::new(-28.0, 60.0, 450.0), Vec3::new(-28.0, 60.0, 300.0), true, false, false, true).expect("the wall");
    assert_eq!(poly.bg, b.bg);
    assert!(hit.z > 380.0 && hit.z < 400.0, "the face at {hit:?}");
    println!("the wall's face at z {}", hit.z);

    // Link walking into it from 90 in front (between the chest and the wall) stops at it.
    place_link(&mut w, Vec3::new(-28.0, 20.0, 476.0), i16::MIN);
    idle(&mut w, 2);
    let mut prev = PadState::default();
    let mut min_z = f32::MAX;
    for _ in 0..60 {
        let s = oot_actors::script::stick_towards(&w, Vec3::new(-28.0, 20.0, 200.0), 60.0);
        w.tick_with(scripted_input(prev, s));
        prev = s;
        min_z = min_z.min(w.player().actor.world_pos.z);
    }
    // Never behind its face. (On the frame he runs into it, from 400.2 at 4 a frame, he's left on
    // the face's plane, z 386, for one frame, then put 14 out: not worked out against the C's
    // BgCheck_CheckWallImpl / BgCheck_SphVsDynaWall yet, noted for the lead.)
    assert!(min_z >= hit.z, "Link held at the wall (got to z {min_z})");
    // Then held 14 in front of it (child Link's wall check radius, ageProperties->wallCheckRadius).
    assert_eq!(w.player().actor.world_pos.z, hit.z + 14.0, "Link against the wall");
    // 60 frames standing at it: it stands (no bombs).
    assert_eq!(all::<BgBreakwall>(&w), vec![h]);
    assert!(!w.flags.get_switch(1));
}

#[test]
fn a_broken_walls_flag_keeps_it_away() {
    let Some(a) = assets() else { return };
    // Switch flag 1 saved for the shield grave: BgBreakwall_Init kills it (Flags_GetSwitch) before
    // its collider and object slot.
    let w = grave(&a, |s| s.scene_flags[SCENE_GRAVE_WITH_FAIRYS_FOUNTAIN].swch |= 1 << 1);
    assert!(all::<BgBreakwall>(&w).is_empty());
    let hit = w.col.entity_line_test(Vec3::new(-28.0, 60.0, 450.0), Vec3::new(-28.0, 60.0, 300.0), true, false, false, true);
    assert!(hit.is_none_or(|(_, p)| p.bg == eng_collision::dyna::BGCHECK_SCENE), "no wall's bg actor in the way: {hit:?}");
}

/// An explosion as the collision check sees one: a cylinder AT (`AT_ON | AT_TYPE_PLAYER`, taking
/// `0x08`, `DMG_EXPLOSIVE`, as a bomb's blast deals), registered each frame.
struct Blast {
    actor: Actor,
    collider: ColliderCylinder,
}

const BLAST_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: cc::ColliderInit { col_type: cc::COL_MATERIAL_HIT0, at_flags: cc::AT_ON | cc::AT_TYPE_PLAYER, ac_flags: cc::AC_NONE, oc_flags1: cc::OC1_NONE, oc_flags2: cc::OC2_NONE, shape: cc::COLSHAPE_CYLINDER },
    info: cc::ColliderElementInit {
        elem_material: cc::ELEM_MATERIAL_UNK0,
        at_dmg_info: cc::ColliderElementDamageInfoAT { dmg_flags: 0x0000_0008, hit_special_effect: cc::HIT_SPECIAL_EFFECT_NONE, damage: 0x08 },
        ac_dmg_info: cc::ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: cc::HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: cc::ATELEM_ON | cc::ATELEM_SFX_NONE,
        ac_elem_flags: cc::ACELEM_NONE,
        oc_elem_flags: cc::OCELEM_NONE,
    },
    dim: eng_collision::math3d::Cylinder16 { radius: 60, height: 80, y_shift: -20, pos: [0, 0, 0] },
};

impl ActorImpl for Blast {
    fn name(&self) -> &'static str {
        "Injected explosion"
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    fn update(&mut self, play: &mut PlayState) {
        self.collider.update(&self.actor);
        play.collision_check_set_at(&self.actor, 0, &mut self.collider);
    }
    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0).then_some(ColliderMut::Cylinder(&mut self.collider))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// `ACTOR_EN_BOM` (`actor_table.h`: 0x0010), the bomb.
const ACTOR_EN_BOM: i16 = 0x0010;

fn fragments(w: &PlayState, parent: ActorHandle) -> Vec<ActorHandle> {
    let mut v: Vec<ActorHandle> = w
        .actors
        .category(oot_game::actor_ctx::ACTORCAT_PROP)
        .iter()
        .copied()
        .filter(|&h| w.actors.downcast::<EnAObj>(h).is_some_and(|f| f.actor.params == A_OBJ_BOULDER_FRAGMENT && f.actor.parent == Some(parent)))
        .collect();
    v.reverse();
    v
}

#[test]
fn an_explosion_breaks_the_wall() {
    let Some(a) = assets() else { return };
    let mut w = grave(&a, |_| {});
    let h = one::<BgBreakwall>(&w);
    let bg = wall(&w, h).bg;
    // Link 120 in front, facing it (the camera on it: its draw, which sets the quad, runs).
    place_link(&mut w, Vec3::new(-28.0, 20.0, 506.0), i16::MIN);
    idle(&mut w, 3);
    assert!(w.actors.actor(h).unwrap().is_drawn);
    // A blast 30 in front of the wall's foot, its cylinder (60 across, 20 below to 60 above)
    // through the quad 10 in front of the face.
    let mut b = Actor::new(Vec3::new(-28.0, 20.0, 416.0), 0);
    b.id = ACTOR_EN_BOM;
    b.category = ACTORCAT_EXPLOSIVE;
    b.flags = oot_game::actor::ACTOR_FLAG_UPDATE_CULLING_DISABLED;
    w.effect_ss = Default::default();
    w.spawn(Box::new(Blast { actor: b, collider: ColliderCylinder::new(&BLAST_INIT) })).expect("spawn");
    // Its update registers the AT; the next frame's CollisionCheck_AT finds the quad, and the
    // wall's BgBreakwall_Wait breaks it.
    idle(&mut w, 1);
    assert!(!w.actors.downcast::<BgBreakwall>(h).unwrap().actor.killed);
    idle(&mut w, 1);
    let frame = w.audio.frames;
    let b = wall(&w, h);
    assert!(b.actor.killed);
    // DynaPoly_DeleteBgActor; Flags_SetSwitch(1).
    assert!(!w.col.dyna.is_bg_actor(bg));
    assert!(w.flags.get_switch(1));
    // BgBreakwall_SpawnFragments(effectPos (0, 55, 0), velocity 0, scaleY 6.4, scaleX 5, count 1,
    // accel 2): one ring of 4 × 4 En_A_Obj (params 0x000B), the wall their parent; on rows 2 and
    // 0, func_80033480's 2 puffs (amountMinusOne 1) for each of 4 points: 16 dust.
    assert_eq!(fragments(&w, h).len(), 16);
    assert_eq!(w.effect_ss.table.iter().filter(|e| e.ty == EFFECT_SS_DUST).count(), 16);
    // Actor_PlaySfx(NA_SE_EV_WALL_BROKEN) at the wall (type 1, not the floor's explosion); params
    // 0xA001 < 0: SFX_PLAY_CENTERED(NA_SE_SY_TRE_BOX_APPEAR). No opening (type 1).
    let s = sfx_on(&w, frame);
    assert!(s.contains(&(NA_SE_EV_WALL_BROKEN, SfxPos::Actor(h))), "{s:?}");
    assert!(s.iter().any(|&(id, _)| id == NA_SE_SY_TRE_BOX_APPEAR), "{s:?}");
    assert!(!s.iter().any(|&(id, _)| id == NA_SE_EV_EXPLOSION || id == NA_SE_SY_CORRECT_CHIME), "{s:?}");
    assert!(!w.save.get_event_chk_inf(bw::EVENTCHKINF_B0));
    // Gone the next frame, and the way open.
    idle(&mut w, 1);
    assert!(w.actors.downcast::<BgBreakwall>(h).is_none());
    let blocked = w.col.entity_line_test(Vec3::new(-28.0, 60.0, 450.0), Vec3::new(-28.0, 60.0, 300.0), true, false, false, true);
    assert!(blocked.is_none_or(|(_, p)| p.bg != bg), "the way open");
}

/// `BgBreakwall_SpawnFragments` the C's way from `r`: for the ring, `j` and `k` from 3 down, the
/// point `actorPosList[j][k]` through the matrix, the spawn's three `Rand_CenteredFloat(20)`; on
/// even rows `func_80033480`'s two puffs (four `Rand` each, and the dust's own: draw flags 5);
/// then speed, `velocity.y`, the yaw's scatter, rot x and z, the three scales.
struct Expected {
    pos: Vec3,
    rot: [i16; 3],
    speed: f32,
    velocity_y: f32,
    scale: Vec3,
}

fn expected_fragments(r: &mut Rand, wpos: Vec3, wrot: [i16; 3], effect_pos: Vec3, scale_y: f32, scale_x: f32, accel: f32) -> Vec<Expected> {
    let rot_list: [[i16; 3]; 4] = [[0, 0, 0], [0, 0, 0x4000], [0, 0, -0x4000], [0, 0, 0]];
    let mut scales = [Vec3::splat(0.004); 4];
    for k in 0..4 {
        if k == 0 || k == 3 {
            scales[k] = Vec3::new(0.004 * scale_x, 0.004 * scale_y, 0.004 * scale_y);
        } else {
            scales[k] = Vec3::new(0.004 * scale_y, 0.004 * scale_x, 0.004 * scale_x);
        }
    }
    let pos_list: [[Vec3; 4]; 4] = [
        [Vec3::new(40.0, 15.0, 0.0), Vec3::new(30.0, 57.0, 0.0), Vec3::new(50.0, 57.0, 0.0), Vec3::new(40.0, 70.0, 0.0)],
        [Vec3::new(55.0, -15.0, 0.0), Vec3::new(30.0, -32.0, 0.0), Vec3::new(50.0, -32.0, 0.0), Vec3::new(20.0, -10.0, 0.0)],
        [Vec3::new(-40.0, 14.0, 0.0), Vec3::new(-50.0, 57.0, 0.0), Vec3::new(-30.0, 57.0, 0.0), Vec3::new(-40.0, 70.0, 0.0)],
        [Vec3::new(-55.0, -15.0, 0.0), Vec3::new(-55.0, -32.0, 0.0), Vec3::new(-30.0, -32.0, 0.0), Vec3::new(-20.0, -10.0, 0.0)],
    ];
    let angle1 = (wrot[1] as i32).abs() as i16;
    let mut m = MtxF::set_translate(wpos.x, wpos.y, wpos.z);
    m.rotate_zyx(wrot[0], wrot[1], wrot[2]);
    m.translate(effect_pos.x, effect_pos.y, effect_pos.z);
    let mut v = Vec::new();
    for j in (0..4).rev() {
        for k in (0..4).rev() {
            let p = m.mult_vec3f(pos_list[j][k]);
            let x = r.centered_float(20.0) + p.x;
            let y = r.centered_float(20.0) + p.y;
            let z = r.centered_float(20.0) + p.z;
            if j & 1 == 0 {
                for _ in 0..2 {
                    for _ in 0..5 {
                        r.zero_one();
                    }
                }
            }
            let speed = r.zero_one() + accel * 0.6;
            let velocity_y = r.zero_one() + accel * 0.6;
            let d_yaw = ((r.zero_one() - 0.5) * 3000.0) as i16;
            let rx = ((r.zero_one() * 3500.0) as i16).wrapping_add(2000);
            let rz = ((r.zero_one() * 3500.0) as i16).wrapping_add(2000);
            let sx = scales[k].x + r.centered_float(0.001);
            let sy = scales[k].y + r.centered_float(0.001);
            let sz = scales[k].z + r.centered_float(0.001);
            v.push(Expected { pos: Vec3::new(x, y, z), rot: [rx, rot_list[k][1].wrapping_add(angle1).wrapping_add(d_yaw), rz], speed, velocity_y, scale: Vec3::new(sx, sy, sz) });
        }
    }
    v
}

#[test]
fn the_fragments_fly_as_the_cs() {
    let Some(a) = assets() else { return };
    let mut w = grave(&a, |_| {});
    let h = one::<BgBreakwall>(&w);
    w.effect_ss = Default::default();
    let mut r = w.rand;
    let exp = expected_fragments(&mut r, WALL, [0, 0, 0], Vec3::new(0.0, 55.0, 0.0), 6.4, 5.0, 2.0);
    let act = w.actors.take(h).unwrap();
    w.cur_actor = Some(h);
    act.as_any().downcast_ref::<BgBreakwall>().unwrap().spawn_fragments(&mut w, Vec3::new(0.0, 55.0, 0.0), 0.0, 6.4, 5.0, 1, 2.0);
    w.cur_actor = None;
    w.actors.put_back(h, act);
    assert_eq!(w.rand, r, "the Rand calls, in the C's order");
    let f = fragments(&w, h);
    assert_eq!(f.len(), 16);
    for (i, (&fh, e)) in f.iter().zip(&exp).enumerate() {
        let a = w.actors.actor(fh).unwrap();
        assert_eq!(a.world_pos, e.pos, "fragment {i}");
        assert_eq!([a.world_rot.x, a.world_rot.y, a.world_rot.z], e.rot, "fragment {i}");
        assert_eq!((a.speed_xz, a.velocity.y, a.scale), (e.speed, e.velocity_y, e.scale), "fragment {i}");
        // Actor_Spawn's rotation: shape.rot keeps actorRotList[k] + angle1 (rot.z ±0x4000 for
        // k 1 and 2).
        let k = 3 - (i % 4);
        assert_eq!(a.shape_rot.z, [0, 0x4000, -0x4000, 0][k], "fragment {i}");
    }
    // The dust: func_80033480 at each even row's points, 0 across (velocity 0), 650 to 780 big.
    let dust: Vec<_> = w.effect_ss.table.iter().filter(|e| e.ty == EFFECT_SS_DUST).collect();
    assert_eq!(dust.len(), 16);
}

#[test]
fn the_cavern_entrances_wall_plays_the_opening_once() {
    let Some(a) = assets() else { return };
    let mut w = grave(&a, |_| {});
    // A type 0 wall (BWALL_DC_ENTRANCE, flag 5) spawned between Link and the chest, on
    // object_bwall (in view: once its object's in, it updates only inside its culling volume).
    let at = Vec3::new(-23.0, 20.0, 800.0);
    let h = w.actor_spawn(ACTOR_BG_BREAKWALL, at, [0, 0, 0], 0x0005).expect("Bg_Breakwall");
    idle(&mut w, 1);
    assert_eq!(wall(&w, h).action, bw::Action::Wait);
    // The hit as CollisionCheck_AC leaves it.
    w.actors.downcast_mut::<BgBreakwall>(h).unwrap().collider.base.ac_flags |= cc::AC_HIT;
    idle(&mut w, 1);
    let frame = w.audio.frames;
    assert!(wall(&w, h).actor.killed);
    assert!(w.flags.get_switch(5));
    // EVENTCHKINF_B0 not set: set; Cutscene_SetScript(gDcOpeningCs), cutsceneTrigger 1 (taken by
    // the cutscene's start, Cutscene_HandleConditionalTriggers), the chime; params 5 > 0: no
    // treasure chime.
    assert!(w.save.get_event_chk_inf(bw::EVENTCHKINF_B0));
    assert!(w.cs_ctx.segment.as_ref().is_some_and(|s| s.name == "gDcOpeningCs"));
    let s = sfx_on(&w, frame);
    assert!(s.iter().any(|&(id, _)| id == NA_SE_SY_CORRECT_CHIME), "{s:?}");
    assert!(s.contains(&(NA_SE_EV_WALL_BROKEN, SfxPos::Actor(h))), "{s:?}");
    assert!(!s.iter().any(|&(id, _)| id == NA_SE_SY_TRE_BOX_APPEAR), "{s:?}");
}

#[test]
fn king_dodongos_floor_and_lava_cover() {
    let Some(a) = assets() else { return };
    let mut w = grave(&a, |_| {});
    // Without object_kingdodongo in a slot, types 2 and 3 go at init (Object_GetSlot < 0).
    let at = Vec3::new(-23.0, 20.0, 800.0);
    let floor = w.actor_spawn(ACTOR_BG_BREAKWALL, at, [0, 0, 0], 0x4002).expect("Bg_Breakwall");
    assert!(wall(&w, floor).actor.killed);
    let cover = w.actor_spawn(ACTOR_BG_BREAKWALL, at, [0, 0, 0], 0x6000).expect("Bg_Breakwall");
    assert!(wall(&w, cover).actor.killed);
    // With it loaded (King Dodongo's room's object list).
    w.object_ctx.spawn(bw::OBJECT_KINGDODONGO);
    let floor = w.actor_spawn(ACTOR_BG_BREAKWALL, at, [0, 0, 0], 0x4002).expect("Bg_Breakwall");
    let cover = w.actor_spawn(ACTOR_BG_BREAKWALL, at + Vec3::new(0.0, 0.0, 100.0), [0, 0, 0], 0x6000).expect("Bg_Breakwall");
    {
        // The floor: colType 1, world.rot.x 0x4000 (shape.rot left 0), its list.
        let f = wall(&w, floor);
        assert_eq!((f.wall_type(), f.col_type, f.actor.world_rot.x, f.actor.shape_rot.x), (bw::BWALL_KD_FLOOR, 1, 0x4000, 0));
        assert_eq!(f.bombable_wall_dlist, Some("object_kingdodongo_DL_025BD0"));
        // The lava cover: no list, no quad; 40 down.
        let c = wall(&w, cover);
        assert_eq!((c.wall_type(), c.col_type, c.bombable_wall_dlist), (bw::BWALL_KD_LAVA_COVER, -1, None));
        assert_eq!(c.actor.world_pos.y, 20.0 - 40.0);
        assert_eq!(c.quad(), None);
    }
    idle(&mut w, 1);
    assert_eq!(wall(&w, floor).action, bw::Action::Wait);
    assert_eq!(wall(&w, cover).action, bw::Action::LavaCoverMove);
    // The floor's quad: sColQuadList[1], the 20 by 20 square at its feet (shape.rot 0), scale 0.1.
    let q = wall(&w, floor).quad().unwrap();
    assert!(near(q[0], at + Vec3::new(1.0, 0.0, 1.0)) && near(q[3], at + Vec3::new(-1.0, 0.0, -1.0)), "{q:?}");
    // The cover rises 1 a frame to its home (Math_StepToF, KREG(80) 0): from -20 at its update
    // after WaitForObject's.
    let y0 = wall(&w, cover).actor.world_pos.y;
    idle(&mut w, 10);
    assert_eq!(wall(&w, cover).actor.world_pos.y, y0 + 10.0);
    idle(&mut w, 40);
    assert_eq!(wall(&w, cover).actor.world_pos.y, 20.0);
    // The floor broken: NA_SE_EV_EXPLOSION, the fragments from 25 in front of it, 10 down in its
    // frame (world.rot.x not 0).
    w.actors.downcast_mut::<BgBreakwall>(floor).unwrap().collider.base.ac_flags |= cc::AC_HIT;
    idle(&mut w, 1);
    let frame = w.audio.frames;
    assert!(wall(&w, floor).actor.killed);
    assert!(sfx_on(&w, frame).contains(&(NA_SE_EV_EXPLOSION, SfxPos::Actor(floor))));
    assert!(w.flags.get_switch(2));
    assert_eq!(fragments(&w, floor).len(), 16);
}
