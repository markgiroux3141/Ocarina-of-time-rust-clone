//! `En_Okarina_Effect` (`ovl_En_Okarina_Effect/z_en_okarina_effect.c`): the Song of Storms' storm,
//! spawned by `Oceff_Storm`. Unless it already rains, its first update starts the storm: rain up
//! to 20 drops (`precipitation[PRECIP_SOS_MAX]`), the storm's request (the sky darkening, its
//! state on in bad weather or a configured sky), lightning, and the nature ambience's rain and
//! thunder. It lasts 400 frames, counted while the game runs normally (or past 250 regardless),
//! not while the light settings are the scene's own (`lightConfig` 1 outside time-based lights);
//! at 308 it sets cutscene flag 5 (a magic bean's sprout grows). Then the rain stops, the
//! ambience's channels go quiet, and the storm stops (clear weather) or is forgotten.
//!
//! The whole overlay is ported. `gInterruptSongOfStorms` (set by `En_Weather_Tag` near a place's
//! own weather) cuts it short; `FrameAdvance_IsEnabled` is false.
//!
//! Params: none (`Oceff_Storm` spawns it with 1; nothing reads them).
//! Flags: clears cutscene flag 5 every frame of the storm and sets it at 308 (`CutsceneFlags_Set`:
//! a magic bean's sprout grows); reads `gInterruptSongOfStorms` (the storm cut short).

use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_UPDATE_DURING_OCARINA, Actor};
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Flag, FlagUse};
use oot_game::audio::{CHANNEL_IO_PORT_1, NA_BGM_NATURE_AMBIENCE, NATURE_CHANNEL_LIGHTNING, NATURE_CHANNEL_RAIN, SEQ_PLAYER_BGM_MAIN};
use oot_game::env::{
    LIGHT_MODE_TIME, LIGHTNING_LAST, LIGHTNING_ON, PRECIP_RAIN_CUR, PRECIP_SOS_MAX, STORM_REQUEST_NONE, STORM_REQUEST_START, STORM_REQUEST_STOP, STORM_STATE_OFF, STORM_STATE_ON, WEATHER_MODE_CLEAR,
    WEATHER_MODE_HEAVY_RAIN, WEATHER_MODE_RAIN,
};
use oot_game::play::PlayState;

/// `ACTOR_EN_OKARINA_EFFECT` (`actor_table.h`: 0x0170).
pub const ACTOR_EN_OKARINA_EFFECT: i16 = 0x0170;

/// `En_Okarina_Effect_Profile`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_OKARINA_EFFECT,
    name: "En_Okarina_Effect",
    category: ACTORCAT_ITEMACTION,
    flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA,
    object: "gameplay_keep",
    schema: &SCHEMA,
};

/// En_Okarina_Effect's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[],
    flags: &[
        FlagUse::new(Flag::Other("CutsceneFlags 5")).sets().clears().note("set at 308: a magic bean's sprout grows"),
        FlagUse::new(Flag::Other("gInterruptSongOfStorms")).reads().note("set: the storm ends at once"),
    ],
    todo: false,
    objects: &[],
};

/// `PAUSE_STATE_OFF`, `GAMEOVER_INACTIVE`, `TRANS_MODE_OFF`, `GAMEMODE_NORMAL`, `CS_STATE_IDLE`.
const GAMEMODE_NORMAL: u8 = 0;

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    TriggerStorm,
    ManageStorm,
}

pub struct EnOkarinaEffect {
    pub actor: Actor,
    pub timer: u16,
    pub action: Action,
}

impl EnOkarinaEffect {
    /// `EnOkarinaEffect_Init`: killed if it rains already.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        log::debug!("Ocarina storm effect sparkle sparkle sparkle");
        let mut this = EnOkarinaEffect { actor, timer: 0, action: Action::TriggerStorm };
        if play.env_ctx.precipitation[PRECIP_RAIN_CUR] != 0 {
            this.actor.kill();
        }
        Box::new(this)
    }

    /// `EnOkarinaEffect_TriggerStorm`.
    fn trigger_storm(&mut self, play: &mut PlayState) {
        self.timer = 400; // 20 seconds
        play.env_ctx.precipitation[PRECIP_SOS_MAX] = 20;
        play.env_ctx.storm_request = STORM_REQUEST_START;
        if play.env_statics.weather_mode != WEATHER_MODE_CLEAR || play.env_ctx.skybox_config != 0 {
            play.env_ctx.storm_state = STORM_STATE_ON;
        }
        play.env_ctx.lightning_state = LIGHTNING_ON;
        play.environment_play_storm_nature_ambience();
        self.action = Action::ManageStorm;
    }

    /// `EnOkarinaEffect_ManageStorm`.
    fn manage_storm(&mut self, play: &mut PlayState) {
        play.flags_unset_env(5);
        let normal = play.pause_ctx.state == 0
            && play.game_over_ctx.state == oot_game::game_over::GAMEOVER_INACTIVE
            && play.msg_ctx.msg_length == 0
            && (play.transition.mode == oot_game::transition::TRANS_MODE_OFF || play.save.game_mode != GAMEMODE_NORMAL);
        if normal || self.timer >= 250 {
            if play.env_ctx.light_mode != LIGHT_MODE_TIME || play.env_ctx.light_config != 1 {
                self.timer = self.timer.wrapping_sub(1);
            }
            log::trace!("this->timer=[{}]", self.timer);
            if self.timer == 308 {
                log::debug!("Beans, grow!");
                play.flags_set_env(5);
            }
        }
        if play.env_statics.interrupt_song_of_storms {
            self.timer = 0;
        }
        if self.timer == 0 {
            play.env_ctx.precipitation[PRECIP_SOS_MAX] = 0;
            if play.cs_ctx.state == oot_game::cutscene::CS_STATE_IDLE {
                play.environment_stop_storm_nature_ambience();
            } else if play.audio.audio_get_active_seq_id(SEQ_PLAYER_BGM_MAIN) == NA_BGM_NATURE_AMBIENCE {
                play.audio.set_nature_ambience_channel_io(NATURE_CHANNEL_LIGHTNING, CHANNEL_IO_PORT_1, 0);
                play.audio.set_nature_ambience_channel_io(NATURE_CHANNEL_RAIN, CHANNEL_IO_PORT_1, 0);
            }
            log::debug!("E_wether_flg=[{}] rain_evt_trg=[{}]", play.env_statics.weather_mode, play.env_ctx.storm_request);
            if play.env_statics.weather_mode == WEATHER_MODE_CLEAR && play.env_ctx.storm_request == STORM_REQUEST_START {
                play.env_ctx.storm_request = STORM_REQUEST_STOP;
            } else {
                play.env_ctx.storm_request = STORM_REQUEST_NONE;
                play.env_ctx.storm_state = STORM_STATE_OFF;
            }
            play.env_ctx.lightning_state = LIGHTNING_LAST;
            self.actor.kill();
        }
    }
}

impl ActorImpl for EnOkarinaEffect {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnOkarinaEffect_Update` (the debug display aside: `BREG(0)` 0).
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::TriggerStorm => self.trigger_storm(play),
            Action::ManageStorm => self.manage_storm(play),
        }
    }
    /// `EnOkarinaEffect_Destroy`: the storm's rain off; outside rainy weather a started storm
    /// stops (not `OOT_VERSION < PAL_1_0`: either kind of rain keeps it); one last strike.
    fn destroy(&mut self, play: &mut PlayState) {
        play.env_ctx.precipitation[PRECIP_SOS_MAX] = 0;
        let w = play.env_statics.weather_mode;
        if w != WEATHER_MODE_RAIN && w != WEATHER_MODE_HEAVY_RAIN && play.env_ctx.storm_request == STORM_REQUEST_START {
            play.env_ctx.storm_request = STORM_REQUEST_STOP;
            play.environment_stop_storm_nature_ambience();
        }
        play.env_ctx.lightning_state = LIGHTNING_LAST;
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
