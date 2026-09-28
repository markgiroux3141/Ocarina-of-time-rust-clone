//! Scene lighting and fog from `z_kankyo.c`: the light-settings part of `Environment_Update`
//! (time-based blending for `LIGHT_MODE_TIME`, a fixed setting for `LIGHT_MODE_SETTINGS`),
//! the sun/moon light directions, and the clamps that produce `LightContext`'s ambient colour,
//! fog colour, fog near/far and the two directional lights (`dirLight1`, `dirLight2`).
//!
//! Not modelled: light-setting overrides and their blend timers (cutscenes, Sun's Song), the
//! `adj*` adjustments (always 0 outside cutscenes and weather), weather light configs and the
//! debug register overrides (`R_ENV_DISABLE_DBG` is true, so the computed values are used).

use eng_math::{cos_s, sin_s};

use crate::scene::EnvLightSettings;

/// `LIGHT_MODE_TIME` / `LIGHT_MODE_SETTINGS` (`z64environment.h`).
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

/// `sTimeBasedLightConfigs[][7]` from `z_kankyo.c` (read by `oot_import::tables`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EnvTables {
    pub time_based: Vec<Vec<TimeBasedLightEntry>>,
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
