//! The scene's music: `Scene_CommandSoundSettings` (`z_scene.c`), and `z_kankyo.c`'s
//! `Environment_PlaySceneSequence`, which `Play_Init` runs once the first room is in, and
//! `Environment_PlayTimeBasedSequence`, which `Environment_Update` runs every frame.

use super::*;
use crate::env::clock_time;
use crate::play::PlayState;

/// `TimeBasedSeqState` (`environment.h`).
pub const TIMESEQ_DAY_BGM: u8 = 0x00;
pub const TIMESEQ_FADE_DAY_BGM: u8 = 0x01;
pub const TIMESEQ_NIGHT_BEGIN_SFX: u8 = 0x02;
pub const TIMESEQ_EARLY_NIGHT_CRITTERS: u8 = 0x03;
pub const TIMESEQ_NIGHT_DELAY: u8 = 0x04;
pub const TIMESEQ_NIGHT_CRITTERS: u8 = 0x05;
pub const TIMESEQ_DAY_BEGIN_SFX: u8 = 0x06;
pub const TIMESEQ_MORNING_CRITTERS: u8 = 0x07;
pub const TIMESEQ_DAY_DELAY: u8 = 0x08;
pub const TIMESEQ_DISABLED: u8 = 0xFF;

/// `ENTR_LOST_WOODS_8`, `ENTR_LOST_WOODS_9` (`entrance_table.h`): the Lost Woods' two exits on the
/// bridge between Kokiri Forest and Hyrule Field, looked up in the pack's entrance table.
pub const LOST_WOODS_BRIDGE_ENTRANCES: [&str; 2] = ["ENTR_LOST_WOODS_8", "ENTR_LOST_WOODS_9"];

/// `SceneSequences` (`z64.h`): the scene's music and ambience.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SceneSequences {
    pub seq_id: u8,
    pub nature_ambience_id: u8,
}

impl PlayState {
    /// `Scene_CommandSoundSettings`: the scene's music, and the spec change if nothing plays
    /// (`gSaveContext.seqId` disabled: the last scene's music faded out, or the game just
    /// started).
    pub fn scene_command_sound_settings(&mut self, s: crate::scene::SoundSettings) {
        self.sequence_ctx.seq_id = s.seq_id;
        self.sequence_ctx.nature_ambience_id = s.nature_ambience_id;
        if self.save.seq_id == NA_BGM_DISABLED as u8 {
            self.audio.queue_seq_cmd(s.spec_id as u32 | 0xF000_0000);
        }
    }

    /// `Environment_IsForcedSequenceDisabled`.
    pub fn environment_is_forced_sequence_disabled(&self) -> bool {
        self.save.forced_seq_id == NA_BGM_DISABLED
    }

    /// `Environment_ForcePlaySequence`: the next scene plays `seq_id` instead of its own.
    pub fn environment_force_play_sequence(&mut self, seq_id: u16) {
        self.save.forced_seq_id = seq_id;
    }

    /// `Environment_PlaySceneSequence`: the scene's music, or its nature ambience by the time
    /// of day, unless what plays already is it.
    pub fn environment_play_scene_sequence(&mut self) {
        self.time_seq_state = TIMESEQ_DISABLED;
        let (seq_id, nature) = (self.sequence_ctx.seq_id, self.sequence_ctx.nature_ambience_id);
        let day_time = self.save.day_time as i32;

        // both lost woods exits on the bridge from kokiri to hyrule field
        let entr = self.save.entrance_index;
        let bridge = self.assets.as_ref().is_some_and(|a| LOST_WOODS_BRIDGE_ENTRANCES.iter().any(|n| a.scenes.entrance_index(n) == Some(entr)));
        if bridge {
            self.audio.play_nature_ambience_sequence(NATURE_ID_KOKIRI_REGION);
        } else if self.save.forced_seq_id != NA_BGM_GENERAL_SFX {
            if !self.environment_is_forced_sequence_disabled() {
                self.audio.queue_seq_cmd(((SEQ_PLAYER_BGM_MAIN as u32) << 24) | self.save.forced_seq_id as u32);
            }
            self.save.forced_seq_id = NA_BGM_GENERAL_SFX;
        } else if seq_id as u16 == NA_BGM_NO_MUSIC {
            if nature == NATURE_ID_NONE {
                return;
            }
            if self.save.nature_ambience_id != nature {
                self.audio.play_nature_ambience_sequence(nature);
            }
        } else if nature == NATURE_ID_NONE {
            log::debug!("BGM設定 game_play->sound_info.BGM=[{seq_id}] old_bgm=[{}]", self.save.seq_id);
            if self.save.seq_id != seq_id {
                self.audio.audio_play_scene_sequence(seq_id as u16);
            }
        } else if day_time >= clock_time(7, 0) && day_time <= clock_time(17, 10) {
            if self.save.seq_id != seq_id {
                self.audio.audio_play_scene_sequence(seq_id as u16);
            }
            self.time_seq_state = TIMESEQ_FADE_DAY_BGM;
        } else {
            if self.save.nature_ambience_id != nature {
                self.audio.play_nature_ambience_sequence(nature);
            }
            if day_time > clock_time(17, 10) && day_time <= clock_time(19, 0) {
                self.time_seq_state = TIMESEQ_EARLY_NIGHT_CRITTERS;
            } else if day_time > clock_time(19, 0) + 1 || day_time < clock_time(6, 30) {
                self.time_seq_state = TIMESEQ_NIGHT_CRITTERS;
            } else {
                self.time_seq_state = TIMESEQ_MORNING_CRITTERS;
            }
        }
        log::debug!("強制BGM=[{}] BGM=[{seq_id}] エンブ=[{nature}] status=[{}]", self.save.forced_seq_id, self.time_seq_state);
        let echo = self.room_ctx.cur.echo as i8;
        self.audio.set_env_reverb(echo);
    }

    /// `Environment_PlayTimeBasedSequence`: the day's music fading at dusk, the night's
    /// critters, the morning's (and with it a new day's count and the eggs hatching), as the
    /// clock passes (crate::clock); the music and the critters only with no rain.
    pub fn environment_play_time_based_sequence(&mut self) {
        use crate::env::{PRECIP_RAIN_MAX, PRECIP_SOS_MAX};
        let day_time = self.save.day_time as i32;
        let no_rain = self.env_ctx.precipitation[PRECIP_RAIN_MAX] == 0 && self.env_ctx.precipitation[PRECIP_SOS_MAX] == 0;
        let nature = self.sequence_ctx.nature_ambience_id;
        match self.time_seq_state {
            TIMESEQ_DAY_BGM => {
                self.audio.set_nature_ambience_channel_io((NATURE_CHANNEL_CRITTER_4 << 4) | NATURE_CHANNEL_CRITTER_5, CHANNEL_IO_PORT_1, 0);
                if no_rain {
                    log::debug!("Na_StartMorinigBgm");
                    let s = self.sequence_ctx.seq_id as u16;
                    self.audio.audio_play_morning_scene_sequence(s);
                }
                self.time_seq_state += 1;
            }
            TIMESEQ_FADE_DAY_BGM => {
                if day_time > clock_time(17, 10) {
                    if no_rain {
                        self.audio.queue_seq_cmd((0x1 << 28) | ((SEQ_PLAYER_BGM_MAIN as u32) << 24) | 0xF0_00FF);
                    }
                    self.time_seq_state += 1;
                }
            }
            TIMESEQ_NIGHT_BEGIN_SFX => {
                if day_time > clock_time(18, 0) {
                    self.audio.play_sfx_centered2(super::sfx::NA_SE_EV_DOG_CRY_EVENING);
                    self.time_seq_state += 1;
                }
            }
            TIMESEQ_EARLY_NIGHT_CRITTERS => {
                if no_rain {
                    self.audio.play_nature_ambience_sequence(nature);
                    self.audio.set_nature_ambience_channel_io(NATURE_CHANNEL_CRITTER_0, CHANNEL_IO_PORT_1, 1);
                }
                self.time_seq_state += 1;
            }
            TIMESEQ_NIGHT_DELAY => {
                if day_time > clock_time(19, 0) {
                    self.time_seq_state += 1;
                }
            }
            TIMESEQ_NIGHT_CRITTERS => {
                self.audio.set_nature_ambience_channel_io(NATURE_CHANNEL_CRITTER_0, CHANNEL_IO_PORT_1, 0);
                if no_rain {
                    self.audio.set_nature_ambience_channel_io((NATURE_CHANNEL_CRITTER_1 << 4) | NATURE_CHANNEL_CRITTER_3, CHANNEL_IO_PORT_1, 1);
                }
                self.time_seq_state += 1;
            }
            TIMESEQ_DAY_BEGIN_SFX => {
                if day_time <= clock_time(19, 0) && day_time > clock_time(6, 30) {
                    use crate::item::{ITEM_CHICKEN, ITEM_POCKET_CUCCO, ITEM_POCKET_EGG, ITEM_WEIRD_EGG, inventory_replace_item};
                    self.save.total_days += 1;
                    self.save.bgs_day_count += 1;
                    self.save.dog_is_lost = true;
                    self.audio.play_sfx_centered(super::sfx::NA_SE_EV_CHICKEN_CRY_M);
                    if (inventory_replace_item(&mut self.save, ITEM_WEIRD_EGG, ITEM_CHICKEN) || inventory_replace_item(&mut self.save, ITEM_POCKET_EGG, ITEM_POCKET_CUCCO))
                        && self.cs_ctx.state == crate::cutscene::CS_STATE_IDLE
                        && !self.player_in_cs_mode()
                    {
                        self.start_textbox(0x3066, None);
                    }
                    self.time_seq_state += 1;
                }
            }
            TIMESEQ_MORNING_CRITTERS => {
                self.audio.set_nature_ambience_channel_io((NATURE_CHANNEL_CRITTER_1 << 4) | NATURE_CHANNEL_CRITTER_3, CHANNEL_IO_PORT_1, 0);
                if no_rain {
                    self.audio.set_nature_ambience_channel_io((NATURE_CHANNEL_CRITTER_4 << 4) | NATURE_CHANNEL_CRITTER_5, CHANNEL_IO_PORT_1, 1);
                }
                self.time_seq_state += 1;
            }
            TIMESEQ_DAY_DELAY => {
                if day_time > clock_time(7, 0) {
                    self.time_seq_state = 0;
                }
            }
            _ => {}
        }
    }
}
