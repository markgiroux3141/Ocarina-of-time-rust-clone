//! Scene lighting and fog from `z_kankyo.c`: the light-settings part of `Environment_Update`
//! (time-based blending for `LIGHT_MODE_TIME`, a fixed setting for `LIGHT_MODE_SETTINGS`),
//! the sun/moon light directions, and the clamps that produce `LightContext`'s ambient colour,
//! fog colour, fog near/far and the two directional lights (`dirLight1`, `dirLight2`).
//!
//! `update` is the lights a scene starts with (`SceneState::load`); `EnvCtx` is
//! `play->envCtx` across frames (GAME-04b milestone 4): the light-setting override and its
//! blend, a time-based config's change, the `adj*` adjustments, the rain's drops, and with
//! `EnvStatics` the lightning (`gLightningStrike`, `sLightningBolts`). The time the lights read
//! is the save's (`dayTime`, `skyboxTime`), which `crate::clock` advances (GAME-06 milestone 3).
//! Not modelled: the weather's light configs at `Play_Init` (`retainWeatherMode`), and the debug
//! register overrides (`R_ENV_DISABLE_DBG` is true, so the computed values are used).

use eng_math::{cos_s, sin_s};

use crate::scene::EnvLightSettings;

/// `LIGHT_MODE_TIME` / `LIGHT_MODE_SETTINGS` (`environment.h`).
pub const LIGHT_MODE_TIME: u8 = 0;
pub const LIGHT_MODE_SETTINGS: u8 = 1;

/// `CLOCK_TIME` (`macros.h`): `(s32)(((hr) * 60 + (min)) * (f32)0x10000 / (24 * 60) + 0.5f)`.
pub fn clock_time(hr: i32, min: i32) -> i32 {
    ((((hr * 60 + min) as f32) * 65536.0 / 1440.0) + 0.5) as i32
}

/// `TimeBasedLightEntry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TimeBasedLightEntry {
    pub start: u16,
    pub end: u16,
    pub light_setting: u8,
    pub next_light_setting: u8,
}

/// `TimeBasedSkyboxEntry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TimeBasedSkyboxEntry {
    pub start_time: u16,
    pub end_time: u16,
    pub change_skybox: bool,
    pub skybox1_index: u8,
    pub skybox2_index: u8,
}

/// `sTimeBasedLightConfigs[][7]`, `gTimeBasedSkyboxConfigs[][9]` and `gNormalSkyFiles` (each sky's
/// texture and palette files) from `z_kankyo.c` (read by `oot_import::tables`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EnvTables {
    pub time_based: Vec<Vec<TimeBasedLightEntry>>,
    pub skybox_configs: Vec<Vec<TimeBasedSkyboxEntry>>,
    pub normal_sky_files: Vec<(String, String)>,
}

impl EnvTables {
    /// The entry of `gTimeBasedSkyboxConfigs[config]` holding `skybox_time` (`startTime <= time <
    /// endTime`, or an `endTime` of 0xFFFF), and its index.
    pub fn skybox_entry(&self, config: usize, skybox_time: u16) -> Option<(usize, TimeBasedSkyboxEntry)> {
        self.skybox_configs.get(config)?.iter().copied().enumerate().find(|(_, e)| skybox_time >= e.start_time && (skybox_time < e.end_time || e.end_time == 0xFFFF))
    }
}

/// `Environment_LerpWeight(max, min, val)`.
pub fn lerp_weight(max: u16, min: u16, val: u16) -> f32 {
    let diff = max as f32 - min as f32;
    if diff != 0.0 {
        let ret = 1.0 - (max as f32 - val as f32) / diff;
        if !(ret >= 1.0) {
            return ret;
        }
    }
    1.0
}

/// `LERP` into a `u8` (the C assigns the float result to a `u8`, truncating).
fn lerp_u8(x: u8, y: u8, scale: f32) -> u8 {
    ((y as f32 - x as f32) * scale + x as f32) as u8
}

/// `LERP16`: `((s16)(((y) - (x)) * (scale)) + (x))`.
fn lerp16(x: i16, y: i16, scale: f32) -> i16 {
    (((y as f32 - x as f32) * scale) as i16).wrapping_add(x)
}

/// What `Environment_Update` leaves in `LightContext` and the environment's two lights.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnvLights {
    pub ambient: [u8; 3],
    pub light1_dir: [i8; 3],
    pub light1_color: [u8; 3],
    pub light2_dir: [i8; 3],
    pub light2_color: [u8; 3],
    pub fog_color: [u8; 3],
    pub fog_near: i16,
    pub fog_far: i16,
}

/// The inputs `Environment_Update` reads for the light settings.
#[derive(Debug, Clone, Copy)]
pub struct EnvState {
    pub light_mode: u8,
    /// `envCtx->lightConfig` (= `changeLightNextConfig` unless weather changes it).
    pub light_config: usize,
    /// `envCtx->lightSetting` for `LIGHT_MODE_SETTINGS`.
    pub light_setting: usize,
    pub day_time: u16,
    pub skybox_time: u16,
}

pub fn update(tables: &EnvTables, list: &[EnvLightSettings], st: &EnvState) -> EnvLights {
    let get = |i: usize| list.get(i).copied().unwrap_or(EnvLightSettings {
        ambient: [0; 3],
        light1_dir: [0; 3],
        light1_color: [0; 3],
        light2_dir: [0; 3],
        light2_color: [0; 3],
        fog_color: [0; 3],
        fog_near_raw: 0,
        fog_far: 0,
    });
    let mut s = get(st.light_setting);
    let mut fog_near = s.fog_near();
    let mut fog_far = s.fog_far;
    let cfg = tables.time_based.get(st.light_config);
    if st.light_mode == LIGHT_MODE_TIME
        && let Some(cfg) = cfg
        && let Some(e) = cfg.iter().find(|e| st.skybox_time >= e.start && (st.skybox_time < e.end || e.end == 0xFFFF))
    {
        // lightConfig == changeLightNextConfig, so both blend8 terms are equal and the
        // changeLight blend (sp88) drops out.
        let w = lerp_weight(e.end, e.start, st.skybox_time);
        let (a, b) = (get(e.light_setting as usize), get(e.next_light_setting as usize));
        let l3 = |x: [u8; 3], y: [u8; 3]| [0, 1, 2].map(|j| lerp_u8(x[j], y[j], w));
        s.ambient = l3(a.ambient, b.ambient);
        s.light1_color = l3(a.light1_color, b.light1_color);
        s.light2_color = l3(a.light2_color, b.light2_color);
        s.fog_color = l3(a.fog_color, b.fog_color);
        // Sun direction; the moon is the opposite.
        let t = st.day_time.wrapping_sub(clock_time(12, 0) as u16) as i16;
        s.light1_dir = [(-(sin_s(t) * 120.0)) as i8, (cos_s(t) * 120.0) as i8, (cos_s(t) * 20.0) as i8];
        s.light2_dir = s.light1_dir.map(|v| v.wrapping_neg());
        fog_near = lerp16(a.fog_near(), b.fog_near(), w);
        fog_far = lerp16(a.fog_far, b.fog_far, w);
    }
    // The adjustments are 0; clamp as the C does. dirLight2 adds adjLight1Color (sic).
    EnvLights {
        ambient: s.ambient,
        light1_dir: s.light1_dir,
        light1_color: s.light1_color,
        light2_dir: s.light2_dir,
        light2_color: s.light2_color,
        fog_color: s.fog_color,
        fog_near: fog_near.min(996),
        fog_far: fog_far.min(12800),
    }
}

/// `Scene_CommandTimeSettings`: the time the scene starts at, and the `skyboxTime` snapping
/// applied when time is stopped (`sceneTimeSpeed == 0`) outside cutscenes.
pub fn scene_times(day_time: u16, time_cmd: Option<[u8; 3]>) -> (u16, u16, u8) {
    let mut day = day_time;
    let mut speed = 0u8;
    if let Some([h, m, sp]) = time_cmd {
        if h != 0xFF && m != 0xFF {
            day = (((h as f32 + m as f32 / 60.0) * 60.0) / (1440.0 / 65536.0)) as i32 as u16;
        }
        speed = if sp != 0xFF { sp } else { 0 };
    }
    let mut sky = day;
    if speed == 0 {
        let c = |h, m| clock_time(h, m) as u16;
        if sky > c(4, 0) && sky < c(6, 30) {
            sky = c(5, 0) + 1;
        } else if sky >= c(6, 30) && sky <= c(8, 0) {
            sky = c(8, 0) + 1;
        } else if sky >= c(16, 0) && sky <= c(17, 0) {
            sky = c(17, 0) + 1;
        } else if sky >= c(18, 0) + 1 && sky <= c(19, 0) {
            sky = c(19, 0) + 1;
        }
    }
    (day, sky, speed)
}

/// `gSPFogPosition` / `Gfx_SetFog`'s special cases: the (multiplier, offset) pair loaded into
/// the RSP for fog from `near` to `far` (in 0..1000 of the depth range).
pub fn fog_factor(near: i32, far: i32) -> (i16, i16) {
    let far = if far == near { far + 1 } else { far };
    if near >= 1000 {
        (0, 0)
    } else if near >= 997 {
        (0x7FFF, 0x8100u16 as i16)
    } else if near < 0 {
        (0, 255)
    } else {
        ((128000 / (far - near)) as i16, ((500 - near) * 256 / (far - near)) as i16)
    }
}

// ---------------------------------------------------------------------------------------------
// The environment context across frames (`play->envCtx`, GAME-04b milestone 4)
// ---------------------------------------------------------------------------------------------

/// `LIGHT_SETTING_MAX`, `LIGHT_SETTING_OVERRIDE_NONE`, `LIGHT_SETTING_OVERRIDE_FULL_CONTROL`
/// (`environment.h`).
pub const LIGHT_SETTING_MAX: u8 = 31;
pub const LIGHT_SETTING_OVERRIDE_NONE: u8 = 0xFF;
pub const LIGHT_SETTING_OVERRIDE_FULL_CONTROL: u8 = 0xFE;
/// `LIGHT_BLENDRATE_OVERRIDE_NONE`, `LIGHT_BLEND_OVERRIDE_NONE`, `_FULL_CONTROL`.
pub const LIGHT_BLENDRATE_OVERRIDE_NONE: u16 = 0xFFFF;
pub const LIGHT_BLEND_OVERRIDE_NONE: u8 = 0;
pub const LIGHT_BLEND_OVERRIDE_FULL_CONTROL: u8 = 2;
/// `PRECIP_*`: `precipitation`'s indices.
pub const PRECIP_RAIN_MAX: usize = 0;
pub const PRECIP_RAIN_CUR: usize = 1;
pub const PRECIP_SNOW_CUR: usize = 2;
pub const PRECIP_SNOW_MAX: usize = 3;
pub const PRECIP_SOS_MAX: usize = 4;
/// `LIGHTNING_*` (`lightningState`), `LIGHTNING_STRIKE_*`, `LIGHTNING_BOLT_*`.
pub const LIGHTNING_OFF: u8 = 0;
pub const LIGHTNING_LAST: u8 = 2;
pub const LIGHTNING_STRIKE_WAIT: u8 = 0;
pub const LIGHTNING_STRIKE_START: u8 = 1;
pub const LIGHTNING_STRIKE_END: u8 = 2;
pub const LIGHTNING_BOLT_START: u8 = 0;
pub const LIGHTNING_BOLT_WAIT: u8 = 1;
pub const LIGHTNING_BOLT_DRAW: u8 = 2;
pub const LIGHTNING_BOLT_INACTIVE: u8 = 0xFF;
/// `STORM_REQUEST_*`, `CHANGE_SKYBOX_*`, `STORM_STATE_*`, `SANDSTORM_FILL`.
pub const STORM_REQUEST_NONE: u8 = 0;
pub const STORM_REQUEST_START: u8 = 1;
pub const STORM_REQUEST_STOP: u8 = 2;
pub const CHANGE_SKYBOX_INACTIVE: u8 = 0;
pub const CHANGE_SKYBOX_REQUESTED: u8 = 1;
pub const CHANGE_SKYBOX_ACTIVE: u8 = 3;
pub const STORM_STATE_OFF: u8 = 0;
pub const STORM_STATE_ON: u8 = 1;
pub const SANDSTORM_FILL: u8 = 1;
/// `SKYBOX_DMA_*` (`environment.h`).
pub const SKYBOX_DMA_INACTIVE: u8 = 0;
pub const SKYBOX_DMA_TEXTURE1_START: u8 = 1;
pub const SKYBOX_DMA_TEXTURE1_DONE: u8 = 2;
pub const SKYBOX_DMA_TLUT1_START: u8 = 3;
pub const SKYBOX_DMA_TEXTURE2_START: u8 = 11;
pub const SKYBOX_DMA_TEXTURE2_DONE: u8 = 12;
pub const SKYBOX_DMA_TLUT2_START: u8 = 13;
/// `WEATHER_MODE_*` (`environment.h`).
pub const WEATHER_MODE_CLEAR: u8 = 0;
pub const WEATHER_MODE_HEAVY_RAIN: u8 = 5;

/// A load `Environment_UpdateSkybox` started (`DMA_REQUEST_ASYNC`): where its bytes go and
/// which of `gNormalSkyFiles` they're from. It completes by the next call, whose
/// `osRecvMesg(OS_MESG_NOBLOCK)` then succeeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkyboxDma {
    /// `skyboxCtx->staticSegments[n]` gets file `i`'s texture.
    Texture(usize, u8),
    /// `skyboxCtx->palettes`' half `n` gets file `i`'s palette.
    Palette(usize, u8),
}

/// `play->envCtx`: what the port keeps of `EnvironmentContext`, from `Environment_Init`.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvCtx {
    pub light_mode: u8,
    /// `lightConfig`, `changeLightNextConfig`, `changeLightEnabled`, `changeLightTimer`,
    /// `changeDuration`: a time-based config changing to another over a duration.
    pub light_config: usize,
    pub change_light_next_config: usize,
    pub change_light_enabled: bool,
    pub change_light_timer: u16,
    pub change_duration: u16,
    /// `skyboxConfig`, `changeSkyboxNextConfig`, `changeSkyboxState`, `changeSkyboxTimer`
    /// (kept; the skybox change isn't drawn).
    pub skybox_config: u8,
    pub change_skybox_next_config: u8,
    pub change_skybox_state: u8,
    pub change_skybox_timer: u16,
    /// `lightSetting`, `prevLightSetting`, `lightBlend`, `lightBlendEnabled`,
    /// `lightSettingOverride`, `lightBlendOverride`, `lightBlendRateOverride`.
    pub light_setting: usize,
    pub prev_light_setting: usize,
    pub light_blend: f32,
    pub light_blend_enabled: bool,
    pub light_setting_override: u8,
    pub light_blend_override: u8,
    pub light_blend_rate_override: u16,
    /// `lightSettings`: the "live" lights before the adjustments.
    pub light_settings: EnvLightSettings,
    /// `adjAmbientColor`, `adjLight1Color`, `adjFogColor`, `adjFogNear`, `adjFogFar`.
    pub adj_ambient_color: [i16; 3],
    pub adj_light1_color: [i16; 3],
    pub adj_fog_color: [i16; 3],
    pub adj_fog_near: i16,
    pub adj_fog_far: i16,
    /// `precipitation[PRECIP_MAX]`, `stormRequest`, `lightningState`, `sandstormState`.
    pub precipitation: [u8; 5],
    pub storm_request: u8,
    /// `windDirection` (`Environment_Init`: 80 on each axis).
    pub wind_direction: [i16; 3],
    pub lightning_state: u8,
    pub sandstorm_state: u8,
    /// `sceneTimeSpeed`: the room's time speed (`Scene_CommandTimeSettings`), `gTimeSpeed`'s
    /// unless the Sun's Song runs.
    pub scene_time_speed: u8,
    /// `sunPos`: the sun from the eye (`crate::clock::sun_pos`; the moon is opposite).
    pub sun_pos: glam::Vec3,
    /// `glareAlpha`, `lensFlareAlphaScale`: the sun's glare and lens flare, eased in and out
    /// (`Environment_DrawLensFlare`).
    pub glare_alpha: f32,
    pub lens_flare_alpha_scale: f32,
    /// `customSkyboxFilter`, `skyboxFilterColor`: a second fill over the sky.
    pub custom_skybox_filter: bool,
    pub skybox_filter_color: [u8; 4],
    /// `skyboxDisabled`, `sunMoonDisabled` (`Scene_CommandSkyboxDisables`, the room's header).
    pub skybox_disabled: bool,
    pub sun_moon_disabled: bool,
    /// `skybox1Index`, `skybox2Index`, `skyboxBlend`, `skyboxDmaState`, `stormState`; `dmaRequest`
    /// (the load in flight, `None` once received).
    pub skybox1_index: u8,
    pub skybox2_index: u8,
    pub skybox_blend: u8,
    pub skybox_dma_state: u8,
    pub storm_state: u8,
    pub skybox_dma: Option<SkyboxDma>,
}

impl EnvCtx {
    /// `Environment_Init`'s values, for a scene with `light_mode`, entered at `day_time` (the
    /// sun's place; the room's time settings come later, `crate::clock`).
    pub fn init(light_mode: u8, day_time: u16) -> EnvCtx {
        EnvCtx {
            light_mode,
            light_config: 0,
            change_light_next_config: 0,
            change_light_enabled: false,
            change_light_timer: 0,
            change_duration: 0,
            skybox_config: 0,
            change_skybox_next_config: 0,
            change_skybox_state: 0,
            change_skybox_timer: 0,
            light_setting: 0,
            prev_light_setting: 0,
            light_blend: 1.0,
            light_blend_enabled: false,
            light_setting_override: LIGHT_SETTING_OVERRIDE_NONE,
            light_blend_override: LIGHT_BLEND_OVERRIDE_NONE,
            light_blend_rate_override: LIGHT_BLENDRATE_OVERRIDE_NONE,
            light_settings: EnvLightSettings { ambient: [0; 3], light1_dir: [0; 3], light1_color: [0; 3], light2_dir: [0; 3], light2_color: [0; 3], fog_color: [0; 3], fog_near_raw: 0, fog_far: 0 },
            adj_ambient_color: [0; 3],
            adj_light1_color: [0; 3],
            adj_fog_color: [0; 3],
            adj_fog_near: 0,
            adj_fog_far: 0,
            precipitation: [0; 5],
            storm_request: STORM_REQUEST_NONE,
            wind_direction: [80, 80, 80],
            lightning_state: LIGHTNING_OFF,
            sandstorm_state: 0,
            // Environment_Init: sceneTimeSpeed 0, the sun by the time, no glare or lens flare, no
            // custom filter. (The disables are the room's: zeroed with the play state.)
            scene_time_speed: 0,
            sun_pos: crate::clock::sun_pos(day_time),
            glare_alpha: 0.0,
            lens_flare_alpha_scale: 0.0,
            custom_skybox_filter: false,
            skybox_filter_color: [0; 4],
            skybox_disabled: false,
            sun_moon_disabled: false,
            // Environment_Init: 99 (none loaded by the environment yet), the load idle.
            skybox1_index: 99,
            skybox2_index: 99,
            skybox_blend: 0,
            skybox_dma_state: SKYBOX_DMA_INACTIVE,
            storm_state: STORM_STATE_OFF,
            skybox_dma: None,
        }
    }

    /// `Environment_UpdateStorm`: a storm requested starts the change to the rainy sky and
    /// lights (configs 1 and 2) over 100 frames, once the sky isn't between two of its textures;
    /// a stop changes back and clears the weather.
    pub fn update_storm(&mut self, statics: &mut EnvStatics) {
        if self.storm_request == STORM_REQUEST_NONE {
            return;
        }
        match self.storm_state {
            STORM_STATE_OFF => {
                if self.storm_request == STORM_REQUEST_START && !statics.skybox_is_changing {
                    self.change_skybox_state = CHANGE_SKYBOX_REQUESTED;
                    self.skybox_config = 0;
                    self.change_skybox_next_config = 1;
                    self.change_skybox_timer = 100;
                    self.change_light_enabled = true;
                    self.light_config = 0;
                    self.change_light_next_config = 2;
                    statics.light_config_after_underwater = 2;
                    self.change_light_timer = 100;
                    self.change_duration = 100;
                    self.storm_state += 1;
                }
            }
            STORM_STATE_ON => {
                if !statics.skybox_is_changing && self.storm_request == STORM_REQUEST_STOP {
                    statics.weather_mode = WEATHER_MODE_CLEAR;
                    self.change_skybox_state = CHANGE_SKYBOX_REQUESTED;
                    self.skybox_config = 1;
                    self.change_skybox_next_config = 0;
                    self.change_skybox_timer = 100;
                    self.change_light_enabled = true;
                    self.light_config = 2;
                    self.change_light_next_config = 0;
                    statics.light_config_after_underwater = 0;
                    self.change_light_timer = 100;
                    self.change_duration = 100;
                    self.precipitation[PRECIP_RAIN_MAX] = 0;
                    self.storm_request = STORM_REQUEST_NONE;
                    self.storm_state = STORM_STATE_OFF;
                }
            }
            _ => {}
        }
    }

    /// `Environment_UpdateSkybox`, at `Play_Draw`: the cutscene map's blend (config 3) by
    /// `skyboxTime`; the normal sky's pair of textures and their blend by `skyboxTime` (a
    /// changing entry blends across it; a steady one shows one texture: 255 under the entry's
    /// half, 0 from it), a weather change's blend over its duration, and the loads of a new
    /// texture and its palette (into the half its index's parity picks), one step a frame.
    pub fn update_skybox(&mut self, tables: &EnvTables, skybox_id: u8, sky: &mut crate::skybox::SkyboxContext, statics: &mut EnvStatics, skybox_disabled: bool, skybox_time: u16) {
        use crate::skybox::{SKYBOX_CUTSCENE_MAP, SKYBOX_NORMAL_SKY};
        let time = skybox_time;
        if skybox_id == SKYBOX_CUTSCENE_MAP {
            self.skybox_config = 3;
            if let Some((_, e)) = tables.skybox_entry(3, time) {
                self.skybox_blend = if e.change_skybox { (lerp_weight(e.end_time, e.start_time, time) * 255.0) as u8 } else { 0 };
            }
            return;
        }
        if skybox_id != SKYBOX_NORMAL_SKY || skybox_disabled {
            return;
        }
        let (mut new1, mut new2, mut blend) = (0xFFu8, 0xFFu8, 0u8);
        let found = tables.skybox_entry(self.skybox_config as usize, time);
        if let Some((_, e)) = found {
            new1 = e.skybox1_index;
            new2 = e.skybox2_index;
            statics.skybox_is_changing = e.change_skybox;
            let w = (lerp_weight(e.end_time, e.start_time, time) * 255.0) as u8;
            if e.change_skybox {
                blend = w;
            } else {
                blend = if w < 128 { 255 } else { 0 };
                if self.change_skybox_state != CHANGE_SKYBOX_INACTIVE && self.change_skybox_state < CHANGE_SKYBOX_ACTIVE {
                    self.change_skybox_state += 1;
                    blend = 0;
                }
            }
        }
        self.update_storm(statics);
        if self.change_skybox_state >= CHANGE_SKYBOX_ACTIVE {
            // `i` as the loop left it: the entry found (or the end of the list).
            let i = found.map_or(tables.skybox_configs.get(self.skybox_config as usize).map_or(0, |c| c.len()), |(i, _)| i);
            let entry = |config: u8| tables.skybox_configs.get(config as usize).and_then(|c| c.get(i)).copied();
            if let (Some(a), Some(b)) = (entry(self.skybox_config), entry(self.change_skybox_next_config)) {
                new1 = a.skybox1_index;
                new2 = b.skybox2_index;
            }
            blend = ((self.change_duration as f32 - self.change_skybox_timer as f32) / self.change_duration as f32 * 255.0) as u8;
            self.change_skybox_timer = self.change_skybox_timer.wrapping_sub(1);
            if self.change_skybox_timer as i16 <= 0 {
                self.change_skybox_state = CHANGE_SKYBOX_INACTIVE;
                self.skybox_config = self.change_skybox_next_config;
            }
        }
        if new1 == 0xFF {
            log::warn!("Environment VR data acquisition failed! Report to Sasaki!");
        }
        // A palette's half: the first for an index whose bits 0 and 2 differ, else the second.
        let half = |i: u8| if ((i & 1) ^ ((i & 4) >> 2)) != 0 { 0 } else { 1 };
        // A load started in this call isn't received in it (the DMA is still running); one
        // started in the last call is (osRecvMesg(OS_MESG_NOBLOCK) succeeds).
        let mut started = false;
        if self.skybox1_index != new1 && self.skybox_dma_state == SKYBOX_DMA_INACTIVE {
            self.skybox_dma_state = SKYBOX_DMA_TEXTURE1_START;
            self.skybox_dma = Some(SkyboxDma::Texture(0, new1));
            self.skybox1_index = new1;
            started = true;
        }
        if self.skybox2_index != new2 && self.skybox_dma_state == SKYBOX_DMA_INACTIVE {
            self.skybox_dma_state = SKYBOX_DMA_TEXTURE2_START;
            self.skybox_dma = Some(SkyboxDma::Texture(1, new2));
            self.skybox2_index = new2;
            started = true;
        }
        if self.skybox_dma_state == SKYBOX_DMA_TEXTURE1_DONE {
            self.skybox_dma_state = SKYBOX_DMA_TLUT1_START;
            self.skybox_dma = Some(SkyboxDma::Palette(half(new1), new1));
            started = true;
        }
        if self.skybox_dma_state == SKYBOX_DMA_TEXTURE2_DONE {
            self.skybox_dma_state = SKYBOX_DMA_TLUT2_START;
            self.skybox_dma = Some(SkyboxDma::Palette(half(new2), new2));
            started = true;
        }
        if self.skybox_dma_state == SKYBOX_DMA_TEXTURE1_START || self.skybox_dma_state == SKYBOX_DMA_TEXTURE2_START {
            if !started {
                if let Some(d) = self.skybox_dma.take() {
                    sky.receive(d);
                }
                self.skybox_dma_state += 1;
            }
        } else if self.skybox_dma_state >= SKYBOX_DMA_TEXTURE1_DONE && !started {
            if let Some(d) = self.skybox_dma.take() {
                sky.receive(d);
            }
            self.skybox_dma_state = SKYBOX_DMA_INACTIVE;
        }
        self.skybox_blend = blend;
    }

    /// `Environment_UpdateRain`: the rain's drops towards the larger of its two maximums, by 2
    /// every 8 frames.
    pub fn update_rain(&mut self, frames: u32) {
        let max = self.precipitation[PRECIP_RAIN_MAX].max(self.precipitation[PRECIP_SOS_MAX]);
        let cur = &mut self.precipitation[PRECIP_RAIN_CUR];
        if *cur != max && frames % 8 == 0 {
            if *cur < max {
                *cur = cur.wrapping_add(2);
            } else {
                *cur = cur.wrapping_sub(2);
            }
        }
    }

    /// `Environment_Update`'s lights, after the clock (`crate::clock`) at the save's `day_time`
    /// and `skybox_time`: the setting override, the time-based blend (with a config change's),
    /// the settings' blend, then the adjustments, into what `LightContext` and the two
    /// directional lights get (a light with no direction pointing along x).
    pub fn update_lights(&mut self, tables: &EnvTables, list: &[EnvLightSettings], day_time: u16, skybox_time: u16) -> EnvLights {
        let zero = EnvLightSettings { ambient: [0; 3], light1_dir: [0; 3], light1_color: [0; 3], light2_dir: [0; 3], light2_color: [0; 3], fog_color: [0; 3], fog_near_raw: 0, fog_far: 0 };
        let get = |i: usize| list.get(i).copied().unwrap_or(zero);
        if self.light_setting_override != LIGHT_SETTING_OVERRIDE_NONE
            && self.light_blend_override != LIGHT_BLEND_OVERRIDE_FULL_CONTROL
            && self.light_setting != self.light_setting_override as usize
            && self.light_blend >= 1.0
            && self.light_setting_override <= LIGHT_SETTING_MAX
        {
            self.prev_light_setting = self.light_setting;
            self.light_setting = self.light_setting_override as usize;
            self.light_blend = 0.0;
        }
        if self.light_setting_override != LIGHT_SETTING_OVERRIDE_FULL_CONTROL {
            if self.light_mode == LIGHT_MODE_TIME && self.light_setting_override == LIGHT_SETTING_OVERRIDE_NONE {
                let n = tables.time_based.get(self.light_config).map(|c| c.len()).unwrap_or(0);
                for i in 0..n {
                    let e = tables.time_based[self.light_config][i];
                    if !(skybox_time >= e.start && (skybox_time < e.end || e.end == 0xFFFF)) {
                        continue;
                    }
                    let sp8c = lerp_weight(e.end, e.start, skybox_time);
                    let mut sp88 = 0.0;
                    if self.change_light_enabled {
                        sp88 = (self.change_duration as f32 - self.change_light_timer as f32) / self.change_duration as f32;
                        self.change_light_timer = self.change_light_timer.wrapping_sub(1);
                        if self.change_light_timer == 0 {
                            self.change_light_enabled = false;
                            self.light_config = self.change_light_next_config;
                        }
                    }
                    let entry = |cfg: usize| tables.time_based.get(cfg).and_then(|c| c.get(i)).copied().unwrap_or(e);
                    let (e0, e1) = (entry(self.light_config), entry(self.change_light_next_config));
                    let (a0, b0, a1, b1) = (get(e0.light_setting as usize), get(e0.next_light_setting as usize), get(e1.light_setting as usize), get(e1.next_light_setting as usize));
                    let blend = |f: fn(&EnvLightSettings) -> [u8; 3]| -> [u8; 3] {
                        [0, 1, 2].map(|j| {
                            let x = lerp_u8(f(&a0)[j], f(&b0)[j], sp8c);
                            let y = lerp_u8(f(&a1)[j], f(&b1)[j], sp8c);
                            lerp_u8(x, y, sp88)
                        })
                    };
                    let ls = &mut self.light_settings;
                    ls.ambient = blend(|s| s.ambient);
                    // The sun's direction; the moon's is the opposite.
                    let t = day_time.wrapping_sub(clock_time(12, 0) as u16) as i16;
                    ls.light1_dir = [(-(sin_s(t) * 120.0)) as i8, (cos_s(t) * 120.0) as i8, (cos_s(t) * 20.0) as i8];
                    ls.light2_dir = ls.light1_dir.map(|v| v.wrapping_neg());
                    ls.light1_color = blend(|s| s.light1_color);
                    ls.light2_color = blend(|s| s.light2_color);
                    ls.fog_color = blend(|s| s.fog_color);
                    let near = lerp16(lerp16(a0.fog_near(), b0.fog_near(), sp8c), lerp16(a1.fog_near(), b1.fog_near(), sp8c), sp88);
                    ls.fog_near_raw = near as u16;
                    ls.fog_far = lerp16(lerp16(a0.fog_far, b0.fog_far, sp8c), lerp16(a1.fog_far, b1.fog_far, sp8c), sp88);
                    break;
                }
            } else if !self.light_blend_enabled {
                let mut s = get(self.light_setting);
                s.fog_near_raw = s.fog_near() as u16;
                self.light_settings = s;
                self.light_blend = 1.0;
            } else {
                let (p, c) = (get(self.prev_light_setting), get(self.light_setting));
                let mut blend_rate = ((c.fog_near_raw >> 10) * 4) as u8;
                if blend_rate == 0 {
                    blend_rate += 1;
                }
                if self.light_blend_rate_override != LIGHT_BLENDRATE_OVERRIDE_NONE {
                    blend_rate = self.light_blend_rate_override as u8;
                }
                if self.light_blend_override == LIGHT_BLEND_OVERRIDE_NONE {
                    self.light_blend += blend_rate as f32 / 255.0;
                }
                if self.light_blend > 1.0 {
                    self.light_blend = 1.0;
                }
                let b = self.light_blend;
                let l8 = |x: [u8; 3], y: [u8; 3]| [0, 1, 2].map(|j| lerp_u8(x[j], y[j], b));
                let ld = |x: [i8; 3], y: [i8; 3]| [0, 1, 2].map(|j| lerp16(x[j] as i16, y[j] as i16, b) as i8);
                self.light_settings = EnvLightSettings {
                    ambient: l8(p.ambient, c.ambient),
                    light1_dir: ld(p.light1_dir, c.light1_dir),
                    light1_color: l8(p.light1_color, c.light1_color),
                    light2_dir: ld(p.light2_dir, c.light2_dir),
                    light2_color: l8(p.light2_color, c.light2_color),
                    fog_color: l8(p.fog_color, c.fog_color),
                    fog_near_raw: lerp16(p.fog_near(), c.fog_near(), b) as u16,
                    fog_far: lerp16(p.fog_far, c.fog_far, b),
                };
            }
        }
        self.light_blend_enabled = true;
        // The adjustments, each sum taken as an s16 and clamped to 0..255. dirLight2 adds
        // adjLight1Color too.
        let adj = |v: u8, a: i16| -> u8 {
            let s = (v as i32 + a as i32) as i16;
            s.clamp(0, 255) as u8
        };
        let ls = self.light_settings;
        let near = ls.fog_near_raw as i16 as i32 + self.adj_fog_near as i32;
        let far = ls.fog_far as i32 + self.adj_fog_far as i32;
        // A light with no direction gets x 1.
        let dir = |d: [i8; 3]| if d == [0; 3] { [1, 0, 0] } else { d };
        EnvLights {
            ambient: [0, 1, 2].map(|j| adj(ls.ambient[j], self.adj_ambient_color[j])),
            light1_dir: dir(ls.light1_dir),
            light1_color: [0, 1, 2].map(|j| adj(ls.light1_color[j], self.adj_light1_color[j])),
            light2_dir: dir(ls.light2_dir),
            light2_color: [0, 1, 2].map(|j| adj(ls.light2_color[j], self.adj_light1_color[j])),
            fog_color: [0, 1, 2].map(|j| adj(ls.fog_color[j], self.adj_fog_color[j])),
            fog_near: if near <= 996 { near as i16 } else { 996 },
            fog_far: if far <= 12800 { far as i16 } else { 12800 },
        }
    }
}

/// `LightningStrike` (`gLightningStrike`) and `sLightningFlashAlpha`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LightningStrike {
    pub state: u8,
    pub flash_red: u8,
    pub flash_green: u8,
    pub flash_blue: u8,
    pub flash_alpha_target: u8,
    pub delay_timer: f32,
    /// `sLightningFlashAlpha`.
    pub flash_alpha: i16,
}

/// `LightningBolt` (`sLightningBolts[3]`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightningBolt {
    pub state: u8,
    pub pos: glam::Vec3,
    pub offset: glam::Vec3,
    pub texture_index: u8,
    /// Degrees.
    pub pitch: i8,
    pub roll: i8,
    pub delay_timer: u8,
}

impl Default for LightningBolt {
    fn default() -> LightningBolt {
        // sLightningBolts' initial state (z_kankyo.c): inactive.
        LightningBolt { state: LIGHTNING_BOLT_INACTIVE, pos: glam::Vec3::ZERO, offset: glam::Vec3::ZERO, texture_index: 0, pitch: 0, roll: 0, delay_timer: 0 }
    }
}

/// `z_kankyo.c`'s statics that outlive a scene: `gWeatherMode`, `gLightningStrike` (reset by
/// `Environment_Init`) and `sLightningBolts`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EnvStatics {
    pub weather_mode: u8,
    /// `gSkyboxIsChanging`: the normal sky's current entry blends between two textures.
    pub skybox_is_changing: bool,
    /// `gLightConfigAfterUnderwater`.
    pub light_config_after_underwater: u8,
    pub lightning_strike: LightningStrike,
    pub lightning_bolts: [LightningBolt; 3],
    /// `sGameOverLightsIntensity`, and the two game over lights' nodes (`sNGameOverLightNode`,
    /// `sSGameOverLightNode`; `None` once removed: the C's nodes point at the static infos,
    /// which later writes then change to no effect).
    pub game_over_lights_intensity: u8,
    pub n_game_over_light_node: Option<crate::lights::LightNode>,
    pub s_game_over_light_node: Option<crate::lights::LightNode>,
    /// `gTimeSpeed` (`crate::clock`).
    pub time_speed: u16,
    /// `z_parameter.c`'s `sPrevTimeSpeed` and `D_80125B60` (the Sun's Song played by day).
    pub prev_time_speed: u16,
    pub suns_song_from_day: bool,
    /// `sSunDepthTestX`, `sSunDepthTestY`: the pixel under the sun the next depth read is at
    /// (`Environment_DrawLensFlare`); `sSunScreenDepth`: the depth read there after the frame
    /// (`Environment_GraphCallback`, crate::env_draw).
    pub sun_depth_test: [i16; 2],
    pub sun_screen_depth: u16,
}

impl EnvStatics {
    /// `Environment_Init`'s part: the lightning strike waiting, no flash, the bolts inactive.
    pub fn init(&mut self) {
        self.lightning_strike.state = LIGHTNING_STRIKE_WAIT;
        self.lightning_strike.flash_red = 0;
        self.lightning_strike.flash_green = 0;
        self.lightning_strike.flash_blue = 0;
        self.lightning_strike.flash_alpha = 0;
        for b in self.lightning_bolts.iter_mut() {
            b.state = LIGHTNING_BOLT_INACTIVE;
        }
    }

    /// `Environment_DrawLightning`'s update of the bolts (in `Play_Draw`, with its `Rand_ZeroOne`
    /// calls): a started bolt is put 9500 ahead of the view's eye, 4000 to 5000 up, scattered
    /// and tilted at random, then waits 3 frames per slot, then steps through its eight textures.
    /// Returns the bolts to draw this frame.
    pub fn update_lightning_bolts(&mut self, view_eye: glam::Vec3, view_at: glam::Vec3, rand: &mut crate::play::Rand) -> Vec<LightningBolt> {
        let mut out = Vec::new();
        for (i, b) in self.lightning_bolts.iter_mut().enumerate() {
            match b.state {
                LIGHTNING_BOLT_START => {
                    let (dx, dz) = (view_at.x - view_eye.x, view_at.z - view_eye.z);
                    let len = (dx * dx + dz * dz).sqrt();
                    let (x, z) = (dx / len, dz / len);
                    b.pos.x = view_eye.x + x * 9500.0;
                    b.pos.y = rand.zero_one() * 1000.0 + 4000.0;
                    b.pos.z = view_eye.z + z * 9500.0;
                    b.offset.x = (rand.zero_one() - 0.5) * 5000.0;
                    b.offset.y = 0.0;
                    b.offset.z = (rand.zero_one() - 0.5) * 5000.0;
                    b.texture_index = 0;
                    b.pitch = ((rand.zero_one() - 0.5) * 40.0) as i32 as i8;
                    b.roll = ((rand.zero_one() - 0.5) * 40.0) as i32 as i8;
                    b.delay_timer = 3 * (i as u8 + 1);
                    b.state += 1;
                }
                LIGHTNING_BOLT_WAIT => {
                    b.delay_timer = b.delay_timer.wrapping_sub(1);
                    if b.delay_timer == 0 {
                        b.state += 1;
                    }
                }
                LIGHTNING_BOLT_DRAW => {
                    if b.texture_index < 7 {
                        b.texture_index += 1;
                    } else {
                        b.state = LIGHTNING_BOLT_INACTIVE;
                    }
                }
                _ => {}
            }
            if b.state == LIGHTNING_BOLT_DRAW {
                out.push(*b);
            }
        }
        out
    }

    /// `Environment_AddLightningBolts`: up to `num` inactive bolts started.
    pub fn add_lightning_bolts(&mut self, num: u8) {
        let mut added = 0;
        for b in self.lightning_bolts.iter_mut() {
            if b.state == LIGHTNING_BOLT_INACTIVE {
                b.state = LIGHTNING_BOLT_START;
                added += 1;
                if added >= num {
                    break;
                }
            }
        }
    }
}

impl crate::play::PlayState {
    /// `Environment_Update`'s time and lights into the scene's (`LightContext`'s and the two
    /// directional lights, which the renderer reads).
    pub fn environment_update_lights(&mut self) {
        let Some(assets) = self.assets.clone() else { return };
        let (day_time, skybox_time) = (self.save.day_time, self.save.skybox_time);
        if let Some(sc) = self.scene.as_mut() {
            let list = sc.layer_data().light_settings.clone();
            sc.lights = self.env_ctx.update_lights(&assets.env, &list, day_time, skybox_time);
        }
    }

    /// `Environment_UpdateLightningStrike`: with lightning on, a strike every so often (a 10%
    /// chance a frame of 50 more on its timer, plus up to 1, to 500), its bolts, the flash
    /// fading in by 100 with the ambient light raised and the thunder (the nature ambience's
    /// lightning channel), then fading out by 10. The flash is drawn while it runs.
    pub fn environment_update_lightning_strike(&mut self) {
        use crate::audio::NATURE_CHANNEL_LIGHTNING;
        if self.env_ctx.lightning_state != LIGHTNING_OFF {
            let st = &mut self.env_statics.lightning_strike;
            match st.state {
                LIGHTNING_STRIKE_WAIT => {
                    // Every frame a 10% chance of the timer advancing 50.
                    if self.rand.zero_one() < 0.1 {
                        st.delay_timer += 50.0;
                    }
                    st.delay_timer += self.rand.zero_one();
                    if st.delay_timer > 500.0 {
                        st.flash_red = 200;
                        st.flash_green = 200;
                        st.flash_blue = 255;
                        st.flash_alpha_target = 200;
                        st.delay_timer = 0.0;
                        let n = (self.rand.zero_one() * (3.0 - 0.1)) as u8 + 1;
                        self.env_statics.add_lightning_bolts(n);
                        let st = &mut self.env_statics.lightning_strike;
                        st.flash_alpha = 0;
                        st.state += 1;
                    }
                }
                LIGHTNING_STRIKE_START => {
                    st.flash_red = 200;
                    st.flash_green = 200;
                    st.flash_blue = 255;
                    let a = &mut self.env_ctx.adj_ambient_color;
                    a[0] = a[0].wrapping_add(80);
                    a[1] = a[1].wrapping_add(80);
                    a[2] = a[2].wrapping_add(100);
                    st.flash_alpha = st.flash_alpha.wrapping_add(100);
                    if st.flash_alpha >= st.flash_alpha_target as i16 {
                        st.state += 1;
                        st.flash_alpha_target = 0;
                        self.audio.set_nature_ambience_channel_io(NATURE_CHANNEL_LIGHTNING, 0, 0);
                    }
                }
                LIGHTNING_STRIKE_END => {
                    let a = &mut self.env_ctx.adj_ambient_color;
                    if a[0] > 0 {
                        a[0] -= 10;
                        a[1] -= 10;
                    }
                    if a[2] > 0 {
                        a[2] -= 10;
                    }
                    st.flash_alpha = st.flash_alpha.wrapping_sub(10);
                    if st.flash_alpha <= st.flash_alpha_target as i16 {
                        *a = [0; 3];
                        st.state = LIGHTNING_STRIKE_WAIT;
                        if self.env_ctx.lightning_state == LIGHTNING_LAST {
                            self.env_ctx.lightning_state = LIGHTNING_OFF;
                        }
                    }
                }
                _ => {}
            }
        }
        let st = self.env_statics.lightning_strike;
        self.lightning_flash = (st.state != LIGHTNING_STRIKE_WAIT).then_some([st.flash_red, st.flash_green, st.flash_blue, st.flash_alpha as u8]);
    }

    /// The two game over lights at `intensity`: 10 to the north-west and 10 to the south-east
    /// of Player, 10 up (`Lights_PointNoGlowSetInfo` with the position truncated, then offset).
    fn game_over_light_infos(&self, intensity: u8) -> [crate::lights::LightInfo; 2] {
        use crate::lights::LightInfo;
        let p = self.player.and_then(|h| self.actors.actor(h)).map(|a| a.world_pos).unwrap_or_default();
        let x = |d: f32| ((p.x as i16) as f32 + d) as i16;
        let y = ((p.y as i16) as f32 + 10.0) as i16;
        let z = |d: f32| ((p.z as i16) as f32 + d) as i16;
        let c = [intensity; 3];
        [LightInfo::point_no_glow(x(-10.0), y, z(-10.0), c, 255), LightInfo::point_no_glow(x(10.0), y, z(10.0), c, 255)]
    }

    /// `Environment_InitGameOverLights`: two black point lights inserted at Player.
    pub fn environment_init_game_over_lights(&mut self) {
        self.env_statics.game_over_lights_intensity = 0;
        let [n, s] = self.game_over_light_infos(0);
        self.env_statics.n_game_over_light_node = self.light_ctx.insert_light(n);
        self.env_statics.s_game_over_light_node = self.light_ctx.insert_light(s);
    }

    /// `Environment_FadeInGameOverLights`: the lights follow Player and brighten by 2 a frame
    /// to 254; the scene darkens (the ambient and the first light down by 12 a frame to -255,
    /// the fog black, the far plane and the fog pulled in), or in a fixed-camera scene the
    /// screen fills with black at the lights' intensity.
    pub fn environment_fade_in_game_over_lights(&mut self) {
        let intensity = self.env_statics.game_over_lights_intensity;
        let [n, s] = self.game_over_light_infos(intensity);
        self.light_ctx.set_info(self.env_statics.n_game_over_light_node, n);
        self.light_ctx.set_info(self.env_statics.s_game_over_light_node, s);
        if intensity < 254 {
            self.env_statics.game_over_lights_intensity += 2;
        }
        if self.cam_is_not_fixed() {
            let e = &mut self.env_ctx;
            for i in 0..3 {
                if e.adj_ambient_color[i] > -255 {
                    e.adj_ambient_color[i] -= 12;
                    e.adj_light1_color[i] -= 12;
                }
                e.adj_fog_color[i] = -255;
            }
            if e.light_settings.fog_far as i32 + e.adj_fog_far as i32 > 900 {
                e.adj_fog_far -= 100;
            }
            if e.light_settings.fog_near_raw as i16 as i32 + e.adj_fog_near as i32 > 950 {
                e.adj_fog_near -= 10;
            }
        } else {
            self.transition.screen_fill = Some([0, 0, 0, self.env_statics.game_over_lights_intensity]);
        }
    }

    /// `Environment_FadeOutGameOverLights`: the lights dim by 3 a frame and go at intensity 1,
    /// and the scene's adjustments ease back to none (or the black fill fades out).
    ///
    /// @bug (game): the lights are removed only if the intensity passes through exactly 1. A
    /// fade out from an intensity that's 2 more than a multiple of 3 (the revival's, which
    /// fades in from 0 by 2) goes 2, then 0, and they stay, unlit.
    pub fn environment_fade_out_game_over_lights(&mut self) {
        let st = &mut self.env_statics;
        if st.game_over_lights_intensity >= 3 {
            st.game_over_lights_intensity -= 3;
        } else {
            st.game_over_lights_intensity = 0;
        }
        let intensity = st.game_over_lights_intensity;
        if intensity == 1 {
            let (n, s) = (st.n_game_over_light_node.take(), st.s_game_over_light_node.take());
            self.light_ctx.remove_light(n);
            self.light_ctx.remove_light(s);
        } else if intensity >= 2 {
            let [n, s] = self.game_over_light_infos(intensity);
            self.light_ctx.set_info(self.env_statics.n_game_over_light_node, n);
            self.light_ctx.set_info(self.env_statics.s_game_over_light_node, s);
        }
        if self.cam_is_not_fixed() {
            let e = &mut self.env_ctx;
            for i in 0..3 {
                eng_math::smooth_step_to_s(&mut e.adj_ambient_color[i], 0, 5, 12, 1);
                eng_math::smooth_step_to_s(&mut e.adj_light1_color[i], 0, 5, 12, 1);
                e.adj_fog_color[i] = 0;
            }
            e.adj_fog_far = 0;
            e.adj_fog_near = 0;
        } else {
            self.transition.screen_fill = if intensity == 0 { None } else { Some([0, 0, 0, intensity]) };
        }
    }
}
