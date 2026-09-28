//! The parts of `gSaveContext` (`z64save.h`) that play reads and writes: where the next
//! `Play_Init` enters, Link's age, the time of day, the respawn points, and what one scene
//! passes to the next (the entrance speed, the transition type). It outlives each
//! `PlayState`: a scene change builds a new one from it, as `Play_Init` does.

use glam::Vec3;

/// `RESPAWN_MODE_*`.
pub const RESPAWN_MODE_DOWN: usize = 0;
pub const RESPAWN_MODE_RETURN: usize = 1;
pub const RESPAWN_MODE_TOP: usize = 2;

/// `TRANS_NEXT_TYPE_DEFAULT`: take the transition type from the entrance table.
pub const TRANS_NEXT_TYPE_DEFAULT: u8 = 0xFF;

/// `RespawnData`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RespawnData {
    pub pos: Vec3,
    pub yaw: i16,
    pub player_params: i16,
    pub entrance_index: u16,
    pub room_index: u8,
    pub data: i8,
    pub temp_swch_flags: u32,
    pub temp_collect_flags: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SaveContext {
    /// `entranceIndex`: the `gEntranceTable` row the next `Play_Init` enters by (the group's
    /// first row; the scene layer is added).
    pub entrance_index: u16,
    /// `sceneLayer`.
    pub scene_layer: usize,
    /// `linkAge == LINK_AGE_ADULT`.
    pub adult: bool,
    /// `dayTime` (`CLOCK_TIME`), and `nightFlag` as `Play_Init` sets it.
    pub day_time: u16,
    pub night_flag: bool,
    /// `respawn[RESPAWN_MODE_MAX]` and `respawnFlag` (0: a normal entrance; 1..3: respawn from
    /// that mode + 1; negative: the last entrance).
    pub respawn: [RespawnData; 3],
    pub respawn_flag: i32,
    /// `entranceSpeed`: Player's speed through the exit, for the walk-in on the other side.
    pub entrance_speed: f32,
    /// `nextTransitionType` (`TRANS_NEXT_TYPE_DEFAULT` for the entrance table's).
    pub next_transition_type: u8,
    /// `transFadeDuration`, `transWipeSpeed`.
    pub trans_fade_duration: u16,
    pub trans_wipe_speed: u8,
    /// `retainWeatherMode`, `showTitleCard`.
    pub retain_weather_mode: bool,
    pub show_title_card: bool,
    /// `cutsceneIndex`: never a cutscene here (0).
    pub cutscene_index: u16,
}

impl Default for SaveContext {
    fn default() -> SaveContext {
        SaveContext::new(0, false, 0)
    }
}

impl SaveContext {
    /// A save entering by `entrance_index`, Link `adult` or not, at `day_time`.
    pub fn new(entrance_index: u16, adult: bool, day_time: u16) -> SaveContext {
        SaveContext {
            entrance_index,
            scene_layer: 0,
            adult,
            day_time,
            night_flag: false,
            respawn: [RespawnData::default(); 3],
            respawn_flag: 0,
            entrance_speed: 0.0,
            next_transition_type: TRANS_NEXT_TYPE_DEFAULT,
            trans_fade_duration: 0,
            trans_wipe_speed: 0,
            retain_weather_mode: false,
            show_title_card: true,
            cutscene_index: 0,
        }
    }

    /// `IS_DAY`.
    pub fn is_day(&self) -> bool {
        !self.night_flag
    }
}
