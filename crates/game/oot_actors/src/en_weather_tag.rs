//! `En_Weather_Tag` (`ovl_En_Weather_Tag/z_en_weather_tag.c`): an invisible tag that changes the
//! weather while Link is near it. Within its range (across, `(params >> 8) × 100`) it asks the sky
//! and the lights for its weather (`WeatherTag_CheckEnableWeatherEffect`: a skybox change and a
//! light config change over a duration, `gWeatherMode`), with snow or rain and lightning and the
//! storm's nature ambience for some; 100 past its range it changes them back
//! (`WeatherTag_CheckRestoreWeather`). Its eight types:
//!
//! - 0, the market (cloudy, light config 3, 60 frames);
//! - 1, Lon Lon Ranch (cloudy, light config 2, 100 frames; gone once Epona is Link's);
//! - 2, Zora's Domain (snow, 64 flakes; gone after `EVENTCHKINF_4A`);
//! - 3, Lake Hylia (rain, 25 drops, the storm's ambience; gone after `EVENTCHKINF_4A`);
//! - 4, Death Mountain (cloudy, light config 2, 60 frames; gone after `EVENTCHKINF_49`);
//! - 5, Kakariko's thunderstorm (heavy rain, light config 4: 30 drops, lightning, the ambience;
//!   only once the three spiritual stones' events (`EVENTCHKINF_48`, `_49`, `_4A`) are set and
//!   before the Shadow Medallion);
//! - 6, the desert's sandstorm thickening (the fog nearer, by up to 80, and the far plane by up to
//!   2000, eased; eased back outside the range);
//! - 7, the graveyard's thunderstorm: rain (25 drops), lightning and the ambience on within the
//!   range, whatever the sky; 10 past it, off again (the last lightning, the rain down to 10).
//!
//! While the sky is between two of its textures by the time of day (`gSkyboxIsChanging`, in
//! time-based lights) a weather change waits, and the day runs 20 a frame faster
//! (`gSaveContext.save.dayTime += 20`) to get it over with. Where the place's weather takes over,
//! the Song of Storms' storm (`En_Okarina_Effect`) is cut short: `gInterruptSongOfStorms` is
//! set, and cleared again once the tag's change goes through (this ROM, PAL 1.0 and later, clears
//! it whether or not the weather mode changes).
//!
//! The whole overlay is ported. The debug display (`BREG(0)`) isn't drawn: `BREG(0)` is 0. The
//! skybox change itself is kept in the environment's state, not drawn (`oot_game::env`).
//!
//! Params: `type` bits 0 to 3 (`EnWeatherTagType`, 0 to 7 as listed above); bits 4 to 7 unused;
//! `WEATHER_TAG_RANGE100` bits 8 to 15, signed (the range in hundreds of units across; the
//! graveyard's 0x0607 is 600; below 1 never in range).
//! Flags: reads `EVENTCHKINF_EPONA_OBTAINED` (0x18; type 1 gone), `EVENTCHKINF_48`, `_49`, `_4A`
//! (types 2 to 5) and the Shadow Medallion (`QUEST_MEDALLION_SHADOW`, type 5); sets and clears
//! `gInterruptSongOfStorms` (set while its weather change waits).

use eng_math::smooth_step_to_s;
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::env::{CHANGE_SKYBOX_REQUESTED, LIGHT_MODE_TIME, LIGHTNING_LAST, LIGHTNING_ON, PRECIP_RAIN_CUR, PRECIP_RAIN_MAX, PRECIP_SNOW_MAX, STORM_REQUEST_NONE, WEATHER_MODE_CLEAR, WEATHER_MODE_HEAVY_RAIN, WEATHER_MODE_RAIN};
use oot_game::event_chk_inf;
use oot_game::play::PlayState;
use oot_game::skybox::SKYBOX_NORMAL_SKY;

/// `ACTOR_EN_WEATHER_TAG` (`actor_table.h`: 0x0165).
pub const ACTOR_EN_WEATHER_TAG: i16 = 0x0165;

/// `En_Weather_Tag_Profile`: `ACTORCAT_PROP`, `ACTOR_FLAG_UPDATE_CULLING_DISABLED`, `gameplay_keep`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_WEATHER_TAG, name: "En_Weather_Tag", category: ACTORCAT_PROP, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: "gameplay_keep", schema: &SCHEMA };

/// `PARAMS_GET_U(params, 0, 4)`: the type.
pub const TYPE: Field = Field::params(
    "type",
    0,
    4,
    Kind::Named(&[
        (0, "EN_WEATHER_TAG_TYPE_CLOUDY_MARKET"),
        (1, "EN_WEATHER_TAG_TYPE_CLOUDY_LON_LON_RANCH"),
        (2, "EN_WEATHER_TAG_TYPE_SNOW_ZORAS_DOMAIN"),
        (3, "EN_WEATHER_TAG_TYPE_RAIN_LAKE_HYLIA"),
        (4, "EN_WEATHER_TAG_TYPE_CLOUDY_DEATH_MOUNTAIN"),
        (5, "EN_WEATHER_TAG_TYPE_THUNDERSTORM_KAKARIKO"),
        (6, "EN_WEATHER_TAG_TYPE_SANDSTORM_INTENSITY"),
        (7, "EN_WEATHER_TAG_TYPE_THUNDERSTORM_GRAVEYARD"),
    ]),
    "which weather, and when it's gone",
);
/// `WEATHER_TAG_RANGE100(params)`: `(params >> 8) * 100.0f`, an arithmetic shift of the `s16`.
pub const RANGE: Field = Field::params("WEATHER_TAG_RANGE100", 8, 8, Kind::Range(-128, 127), "its range across, in hundreds of units (below 1: never in range)").signed();

/// En_Weather_Tag's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, RANGE],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_EPONA_OBTAINED)).reads().note("type 1: gone once Epona is Link's"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_48)).reads().note("type 5: the storm needs it"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_49)).reads().note("type 4: gone once set; type 5: the storm needs it"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_4A)).reads().note("types 2 and 3: gone once set; type 5: the storm needs it"),
        FlagUse::new(Flag::Other("gInterruptSongOfStorms")).sets().clears().note("set while its weather change waits, cleared once it goes through"),
    ],
    todo: false,
    objects: &[],
};

/// `EnWeatherTagType` (`z_en_weather_tag.h`).
pub const EN_WEATHER_TAG_TYPE_CLOUDY_MARKET: i16 = 0;
pub const EN_WEATHER_TAG_TYPE_CLOUDY_LON_LON_RANCH: i16 = 1;
pub const EN_WEATHER_TAG_TYPE_SNOW_ZORAS_DOMAIN: i16 = 2;
pub const EN_WEATHER_TAG_TYPE_RAIN_LAKE_HYLIA: i16 = 3;
pub const EN_WEATHER_TAG_TYPE_CLOUDY_DEATH_MOUNTAIN: i16 = 4;
pub const EN_WEATHER_TAG_TYPE_THUNDERSTORM_KAKARIKO: i16 = 5;
pub const EN_WEATHER_TAG_TYPE_SANDSTORM_INTENSITY: i16 = 6;
pub const EN_WEATHER_TAG_TYPE_THUNDERSTORM_GRAVEYARD: i16 = 7;

/// `WEATHER_MODE_CLOUDY_CONFIG3`, `WEATHER_MODE_CLOUDY_CONFIG2`, `WEATHER_MODE_SNOW`
/// (`environment.h`: 1, 2, 3; the port's `oot_game::env` has the others).
pub const WEATHER_MODE_CLOUDY_CONFIG3: u8 = 1;
pub const WEATHER_MODE_CLOUDY_CONFIG2: u8 = 2;
pub const WEATHER_MODE_SNOW: u8 = 3;

/// `EVENTCHKINF_EPONA_OBTAINED`, `EVENTCHKINF_48`, `_49`, `_4A` (`save.h`: 0x18, 0x48, 0x49,
/// 0x4A).
pub const EVENTCHKINF_EPONA_OBTAINED: u16 = 0x18;
pub const EVENTCHKINF_48: u16 = 0x48;
pub const EVENTCHKINF_49: u16 = 0x49;
pub const EVENTCHKINF_4A: u16 = 0x4A;
/// `QUEST_MEDALLION_SHADOW` (`item.h`: 0x04).
pub const QUEST_MEDALLION_SHADOW: u32 = 0x04;

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    DisabledCloudyHyruleMarket,
    EnabledCloudyHyruleMarket,
    DisabledCloudyLonLonRanch,
    EnabledCloudyLonLonRanch,
    DisabledCloudySnow,
    EnabledCloudySnow,
    DisabledRainLakeHylia,
    EnabledRainLakeHylia,
    DisabledCloudyDeathMountain,
    EnabledCloudyDeathMountain,
    DisabledCloudyRainThunderKakariko,
    EnabledCloudyRainThunderKakariko,
    SetSandstormIntensity,
    DisabledRainThunder,
    EnabledRainThunder,
    /// No action set (a type over 7: the C's `actionFunc` stays NULL, and its update would call
    /// it; never placed).
    None,
}

pub struct EnWeatherTag {
    pub actor: Actor,
    pub action: Action,
}

impl EnWeatherTag {
    /// `PARAMS_GET_U(params, 0, 4)`: the type.
    pub fn ty(&self) -> i16 {
        TYPE.get(self.actor.params)
    }

    /// `WEATHER_TAG_RANGE100(params)`: `(params >> 8) * 100.0f` (an arithmetic shift of the s16).
    pub fn range(&self) -> f32 {
        RANGE.get(self.actor.params) as f32 * 100.0
    }

    /// `EnWeatherTag_Init`: not targetable; the type's kill check and first action.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        let mut this = EnWeatherTag { actor, action: Action::None };
        let save = &play.save;
        match this.ty() {
            EN_WEATHER_TAG_TYPE_CLOUDY_MARKET => {
                log::debug!("☆☆☆☆☆ (;o;) About that much ☆☆☆☆☆");
                this.action = Action::DisabledCloudyHyruleMarket;
            }
            EN_WEATHER_TAG_TYPE_CLOUDY_LON_LON_RANCH => {
                log::debug!("☆☆☆☆☆ Cloudy (._.) Ah Melancholy ☆☆☆☆☆");
                if save.get_event_chk_inf(EVENTCHKINF_EPONA_OBTAINED) {
                    this.actor.kill();
                }
                this.action = Action::DisabledCloudyLonLonRanch;
            }
            EN_WEATHER_TAG_TYPE_SNOW_ZORAS_DOMAIN => {
                log::debug!("☆☆☆☆☆ The snow is falling... You're not coming... (._.) ☆☆☆☆☆");
                if save.get_event_chk_inf(EVENTCHKINF_4A) {
                    this.actor.kill();
                }
                this.action = Action::DisabledCloudySnow;
            }
            EN_WEATHER_TAG_TYPE_RAIN_LAKE_HYLIA => {
                log::debug!("☆☆☆☆☆ Waaaa- Na-ga-sa-ki wa- (^o^) ☆☆☆☆☆");
                if save.get_event_chk_inf(EVENTCHKINF_4A) {
                    this.actor.kill();
                }
                this.action = Action::DisabledRainLakeHylia;
            }
            EN_WEATHER_TAG_TYPE_CLOUDY_DEATH_MOUNTAIN => {
                log::debug!("☆☆☆☆☆ Cloudy (._.) Ah Melancholy ☆☆☆☆☆");
                if save.get_event_chk_inf(EVENTCHKINF_49) {
                    this.actor.kill();
                }
                this.action = Action::DisabledCloudyDeathMountain;
            }
            EN_WEATHER_TAG_TYPE_THUNDERSTORM_KAKARIKO => {
                log::debug!("☆☆☆☆☆ Cloudy with rain and thunder (;O;) Wow, I'm depressed ☆☆☆☆☆");
                if !save.get_event_chk_inf(EVENTCHKINF_48) || !save.get_event_chk_inf(EVENTCHKINF_49) || !save.get_event_chk_inf(EVENTCHKINF_4A) || save.check_quest_item(QUEST_MEDALLION_SHADOW) {
                    this.actor.kill();
                }
                this.action = Action::DisabledCloudyRainThunderKakariko;
            }
            EN_WEATHER_TAG_TYPE_SANDSTORM_INTENSITY => {
                log::debug!("☆☆☆☆☆ The desert becomes thicker ☆☆☆☆☆");
                this.action = Action::SetSandstormIntensity;
            }
            EN_WEATHER_TAG_TYPE_THUNDERSTORM_GRAVEYARD => {
                log::debug!("☆☆☆☆☆ Waaaa- Na-ga-sa-ki wa- (^o^) ☆☆☆☆☆");
                this.action = Action::DisabledRainThunder;
            }
            t => log::warn!("En_Weather_Tag: type {t} has no action (the C's actionFunc stays NULL)"),
        }
        Box::new(this)
    }

    /// `Actor_WorldDistXZToActor(&player->actor, &this->actor)`.
    fn player_dist_xz(&self, play: &PlayState) -> f32 {
        let p = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(Vec3::splat(f32::MAX));
        let (dx, dz) = (self.actor.world_pos.x - p.x, self.actor.world_pos.z - p.z);
        (dx * dx + dz * dz).sqrt()
    }

    /// The test both checks share: the weather can change now (not while time-based lights'
    /// sky blends between two textures, unless a non-normal sky already has its next config).
    fn can_change(play: &PlayState) -> bool {
        let e = &play.env_ctx;
        e.light_mode != LIGHT_MODE_TIME || !play.env_statics.skybox_is_changing || (play.skybox_ctx.skybox_id != SKYBOX_NORMAL_SKY && e.light_config == e.change_light_next_config)
    }

    /// No storm asked for, and the lights free to change (outside time-based lights, or not on
    /// config 1 and no change under way).
    fn lights_free(play: &PlayState) -> bool {
        let e = &play.env_ctx;
        e.storm_request == STORM_REQUEST_NONE && (e.light_mode != LIGHT_MODE_TIME || (e.light_config != 1 && !e.change_light_enabled))
    }

    /// Both checks' request: the skybox change from `skybox_config` to `change_skybox_next_config`
    /// and the light config change from `light_config` to `change_light_next_config`, over
    /// `change_duration` frames.
    fn request_change(play: &mut PlayState, skybox_config: u8, change_skybox_next_config: u8, light_config: u8, change_light_next_config: u8, change_duration: u16) {
        let e = &mut play.env_ctx;
        e.change_skybox_state = CHANGE_SKYBOX_REQUESTED;
        e.skybox_config = skybox_config;
        e.change_skybox_next_config = change_skybox_next_config;
        e.change_skybox_timer = change_duration;
        e.change_light_enabled = true;
        e.light_config = light_config as usize;
        e.change_light_next_config = change_light_next_config as usize;
        play.env_statics.light_config_after_underwater = change_light_next_config;
        e.change_duration = change_duration;
        e.change_light_timer = e.change_duration;
    }

    /// `if (gTimeSpeed != 0) gSaveContext.save.dayTime += 20;`: the sky's blend hurried along.
    fn hurry_day_time(play: &mut PlayState) {
        if play.env_statics.time_speed != 0 {
            play.save.day_time = play.save.day_time.wrapping_add(20);
        }
    }

    /// `WeatherTag_CheckEnableWeatherEffect`: Link within range and the sky free to change: the
    /// Song of Storms interrupted; with no storm asked for and the lights free, not interrupted
    /// after all (`OOT_VERSION >= PAL_1_0`), and unless the weather is `weather_mode` already, the
    /// change asked for; true. The sky not free: the day hurried.
    #[allow(clippy::too_many_arguments)]
    pub fn check_enable_weather_effect(&self, play: &mut PlayState, skybox_config: u8, change_skybox_next_config: u8, light_config: u8, change_light_next_config: u8, change_duration: u16, weather_mode: u8) -> bool {
        let mut ret = false;
        if self.player_dist_xz(play) < self.range() {
            if Self::can_change(play) {
                play.env_statics.interrupt_song_of_storms = true;
                if Self::lights_free(play) {
                    // OOT_VERSION >= PAL_1_0 (this ROM): cleared before the weather mode's check.
                    play.env_statics.interrupt_song_of_storms = false;
                    if play.env_statics.weather_mode != weather_mode {
                        play.env_statics.weather_mode = weather_mode;
                        if play.env_ctx.storm_request == STORM_REQUEST_NONE {
                            Self::request_change(play, skybox_config, change_skybox_next_config, light_config, change_light_next_config, change_duration);
                        }
                    }
                    ret = true;
                }
            } else {
                Self::hurry_day_time(play);
            }
        }
        ret
    }

    /// `WeatherTag_CheckRestoreWeather`: Link over 100 past the range and the sky free to
    /// change: the Song of Storms interrupted; with no storm asked for and the lights free, not
    /// after all, the weather clear and the change back asked for; true. The sky not free: the
    /// day hurried.
    pub fn check_restore_weather(&self, play: &mut PlayState, skybox_config: u8, change_skybox_next_config: u8, light_config: u8, change_light_next_config: u8, change_duration: u16) -> bool {
        let mut ret = false;
        if self.range() + 100.0 < self.player_dist_xz(play) {
            if Self::can_change(play) {
                play.env_statics.interrupt_song_of_storms = true;
                if Self::lights_free(play) {
                    play.env_statics.interrupt_song_of_storms = false;
                    play.env_statics.weather_mode = WEATHER_MODE_CLEAR;
                    Self::request_change(play, skybox_config, change_skybox_next_config, light_config, change_light_next_config, change_duration);
                    ret = true;
                }
            } else {
                Self::hurry_day_time(play);
            }
        }
        ret
    }

    /// `EnWeatherTag_DisabledCloudyHyruleMarket`: sky 0 to 1, lights 0 to 3, over 60.
    fn disabled_cloudy_hyrule_market(&mut self, play: &mut PlayState) {
        if self.check_enable_weather_effect(play, 0, 1, 0, 3, 60, WEATHER_MODE_CLOUDY_CONFIG3) {
            self.action = Action::EnabledCloudyHyruleMarket;
        }
    }

    /// `EnWeatherTag_EnabledCloudyHyruleMarket`: back, sky 1 to 0, lights 3 to 0, over 60.
    fn enabled_cloudy_hyrule_market(&mut self, play: &mut PlayState) {
        if self.check_restore_weather(play, 1, 0, 3, 0, 60) {
            self.action = Action::DisabledCloudyHyruleMarket;
        }
    }

    /// `EnWeatherTag_DisabledCloudyLonLonRanch`: sky 0 to 1, lights 0 to 2, over 100.
    fn disabled_cloudy_lon_lon_ranch(&mut self, play: &mut PlayState) {
        if self.check_enable_weather_effect(play, 0, 1, 0, 2, 100, WEATHER_MODE_CLOUDY_CONFIG2) {
            self.action = Action::EnabledCloudyLonLonRanch;
        }
    }

    /// `EnWeatherTag_EnabledCloudyLonLonRanch`: back over 100.
    fn enabled_cloudy_lon_lon_ranch(&mut self, play: &mut PlayState) {
        if self.check_restore_weather(play, 1, 0, 2, 0, 100) {
            self.action = Action::DisabledCloudyLonLonRanch;
        }
    }

    /// `EnWeatherTag_DisabledCloudyDeathMountain`: sky 0 to 1, lights 0 to 2, over 60.
    fn disabled_cloudy_death_mountain(&mut self, play: &mut PlayState) {
        if self.check_enable_weather_effect(play, 0, 1, 0, 2, 60, WEATHER_MODE_CLOUDY_CONFIG2) {
            self.action = Action::EnabledCloudyDeathMountain;
        }
    }

    /// `EnWeatherTag_EnabledCloudyDeathMountain`: back over 60, then
    /// `EnWeatherTag_DisabledCloudyLonLonRanch`. @bug (game): the C goes back to the ranch's
    /// disabled action, not the mountain's own: from then on the tag uses the ranch's 100-frame
    /// changes (the same configs and weather mode, so only the duration differs).
    fn enabled_cloudy_death_mountain(&mut self, play: &mut PlayState) {
        if self.check_restore_weather(play, 1, 0, 2, 0, 60) {
            self.action = Action::DisabledCloudyLonLonRanch;
        }
    }

    /// `EnWeatherTag_DisabledCloudySnow`: sky 0 to 1, lights 0 to 2, over 60; 64 snowflakes.
    fn disabled_cloudy_snow(&mut self, play: &mut PlayState) {
        if self.check_enable_weather_effect(play, 0, 1, 0, 2, 60, WEATHER_MODE_SNOW) {
            play.env_ctx.precipitation[PRECIP_SNOW_MAX] = 64;
            self.action = Action::EnabledCloudySnow;
        }
    }

    /// `EnWeatherTag_EnabledCloudySnow`: back over 60; no snow.
    fn enabled_cloudy_snow(&mut self, play: &mut PlayState) {
        if self.check_restore_weather(play, 1, 0, 2, 0, 60) {
            play.env_ctx.precipitation[PRECIP_SNOW_MAX] = 0;
            self.action = Action::DisabledCloudySnow;
        }
    }

    /// `EnWeatherTag_DisabledRainLakeHylia`: sky 0 to 1, lights 0 to 2, over 100; the storm's
    /// ambience, 25 raindrops.
    fn disabled_rain_lake_hylia(&mut self, play: &mut PlayState) {
        if self.check_enable_weather_effect(play, 0, 1, 0, 2, 100, WEATHER_MODE_RAIN) {
            play.environment_play_storm_nature_ambience();
            play.env_ctx.precipitation[PRECIP_RAIN_MAX] = 25;
            self.action = Action::EnabledRainLakeHylia;
        }
    }

    /// `EnWeatherTag_EnabledRainLakeHylia`: back over 100; the ambience stopped, no rain.
    fn enabled_rain_lake_hylia(&mut self, play: &mut PlayState) {
        if self.check_restore_weather(play, 1, 0, 2, 0, 100) {
            play.environment_stop_storm_nature_ambience();
            play.env_ctx.precipitation[PRECIP_RAIN_MAX] = 0;
            self.action = Action::DisabledRainLakeHylia;
        }
    }

    /// `EnWeatherTag_DisabledCloudyRainThunderKakariko`: sky 0 to 1, lights 0 to 4, over 100;
    /// the ambience, lightning, 30 raindrops.
    fn disabled_cloudy_rain_thunder_kakariko(&mut self, play: &mut PlayState) {
        if self.check_enable_weather_effect(play, 0, 1, 0, 4, 100, WEATHER_MODE_HEAVY_RAIN) {
            play.environment_play_storm_nature_ambience();
            play.env_ctx.lightning_state = LIGHTNING_ON;
            play.env_ctx.precipitation[PRECIP_RAIN_MAX] = 30;
            self.action = Action::EnabledCloudyRainThunderKakariko;
        }
    }

    /// `EnWeatherTag_EnabledCloudyRainThunderKakariko`: back over 100; the ambience stopped, the
    /// last lightning, no rain.
    fn enabled_cloudy_rain_thunder_kakariko(&mut self, play: &mut PlayState) {
        if self.check_restore_weather(play, 1, 0, 4, 0, 100) {
            play.environment_stop_storm_nature_ambience();
            play.env_ctx.lightning_state = LIGHTNING_LAST;
            play.env_ctx.precipitation[PRECIP_RAIN_MAX] = 0;
            self.action = Action::DisabledCloudyRainThunderKakariko;
        }
    }

    /// `EnWeatherTag_SetSandstormIntensity`: within range, `adjFogNear` eased to -80 (steps of
    /// up to 2) and `adjZFar` to -2000 (up to 50); outside, both eased back to 0 (up to 1, 25).
    fn set_sandstorm_intensity(&mut self, play: &mut PlayState) {
        let within = self.player_dist_xz(play) < self.range();
        // adjZFar is the port's adj_fog_far.
        let e = &mut play.env_ctx;
        if within {
            smooth_step_to_s(&mut e.adj_fog_near, -80, 1, 2, 1);
            smooth_step_to_s(&mut e.adj_fog_far, -2000, 1, 50, 1);
        } else {
            smooth_step_to_s(&mut e.adj_fog_near, 0, 1, 1, 1);
            smooth_step_to_s(&mut e.adj_fog_far, 0, 1, 25, 1);
        }
    }

    /// `EnWeatherTag_DisabledRainThunder`: within range, whatever the sky: the ambience,
    /// lightning, 25 raindrops.
    fn disabled_rain_thunder(&mut self, play: &mut PlayState) {
        if self.player_dist_xz(play) < self.range() {
            play.environment_play_storm_nature_ambience();
            play.env_ctx.lightning_state = LIGHTNING_ON;
            play.env_ctx.precipitation[PRECIP_RAIN_MAX] = 25;
            self.action = Action::EnabledRainThunder;
        }
    }

    /// `EnWeatherTag_EnabledRainThunder`: more than 10 past the range: the ambience stopped, the
    /// last lightning, no more rain and 10 drops left falling.
    fn enabled_rain_thunder(&mut self, play: &mut PlayState) {
        if self.range() + 10.0 < self.player_dist_xz(play) {
            play.environment_stop_storm_nature_ambience();
            play.env_ctx.lightning_state = LIGHTNING_LAST;
            play.env_ctx.precipitation[PRECIP_RAIN_MAX] = 0;
            play.env_ctx.precipitation[PRECIP_RAIN_CUR] = 10;
            self.action = Action::DisabledRainThunder;
        }
    }
}

impl ActorImpl for EnWeatherTag {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnWeatherTag_Update`: the action (the debug display aside: `BREG(0)` 0).
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::DisabledCloudyHyruleMarket => self.disabled_cloudy_hyrule_market(play),
            Action::EnabledCloudyHyruleMarket => self.enabled_cloudy_hyrule_market(play),
            Action::DisabledCloudyLonLonRanch => self.disabled_cloudy_lon_lon_ranch(play),
            Action::EnabledCloudyLonLonRanch => self.enabled_cloudy_lon_lon_ranch(play),
            Action::DisabledCloudySnow => self.disabled_cloudy_snow(play),
            Action::EnabledCloudySnow => self.enabled_cloudy_snow(play),
            Action::DisabledRainLakeHylia => self.disabled_rain_lake_hylia(play),
            Action::EnabledRainLakeHylia => self.enabled_rain_lake_hylia(play),
            Action::DisabledCloudyDeathMountain => self.disabled_cloudy_death_mountain(play),
            Action::EnabledCloudyDeathMountain => self.enabled_cloudy_death_mountain(play),
            Action::DisabledCloudyRainThunderKakariko => self.disabled_cloudy_rain_thunder_kakariko(play),
            Action::EnabledCloudyRainThunderKakariko => self.enabled_cloudy_rain_thunder_kakariko(play),
            Action::SetSandstormIntensity => self.set_sandstorm_intensity(play),
            Action::DisabledRainThunder => self.disabled_rain_thunder(play),
            Action::EnabledRainThunder => self.enabled_rain_thunder(play),
            Action::None => {}
        }
    }

    /// `EnWeatherTag_Destroy`: nothing.
    fn destroy(&mut self, _play: &mut PlayState) {}

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
