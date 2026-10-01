//! The game's side of the audio library: what plays when (docs/adr/0026-the-games-audio.md).
//!
//! The library itself (`eng_audio`) runs on the audio thread and knows nothing of Zelda. The
//! game talks to it from its own thread:
//! - `seqcmd` (`code_800F9280.c`): the sequence commands (`Audio_QueueSeqCmd`,
//!   `Audio_ProcessSeqCmd`), each player's volume, tempo and channel fades
//!   (`D_8016E750`, `func_800FA3DC`), the player queues;
//! - `bgm` (`code_800EC960.c`'s sequence parts): the scene's music (`func_800F5550`), the
//!   sequence modes (`Audio_SetSequenceMode`), fanfares and the restored music, the nature
//!   ambience and its channels' IO ports, and `Audio_Update` (`func_800F3054`);
//! - `scene` (`z_kankyo.c`, `z_scene.c`): the scene's sound settings, `Environment_PlaySceneSequence`
//!   and the time of day's music.
//!
//! `GameAudio` holds the statics of those files (they're the code segment's, so they outlive
//! a play state), what the game's thread does to the library this frame (`GameOp`s, which the
//! driver hands to the audio side), and the last `AudioView` the audio side gave back.

pub mod bgm;
pub mod offline;
pub mod scene;
pub mod seqcmd;
pub mod sfx;

use std::sync::Arc;

use eng_audio::{AudioTables, AudioView, GameOp};

pub use seqcmd::{ActiveSeq, Unk50};

/// `SequencePlayerId` (`sequence.h`).
pub const SEQ_PLAYER_BGM_MAIN: u8 = 0;
pub const SEQ_PLAYER_FANFARE: u8 = 1;
pub const SEQ_PLAYER_SFX: u8 = 2;
pub const SEQ_PLAYER_BGM_SUB: u8 = 3;

/// `SequenceMode` (`sequence.h`).
pub const SEQ_MODE_DEFAULT: u8 = 0;
pub const SEQ_MODE_ENEMY: u8 = 1;
pub const SEQ_MODE_STILL: u8 = 2;
pub const SEQ_MODE_IGNORE: u8 = 3;

/// `NA_BGM_*` (`sequence.h`), the ones the ported code names.
pub const NA_BGM_GENERAL_SFX: u16 = 0x00;
pub const NA_BGM_NATURE_AMBIENCE: u16 = 0x01;
pub const NA_BGM_FIELD_LOGIC: u16 = 0x02;
pub const NA_BGM_ENEMY: u16 = 0x1A;
pub const NA_BGM_GAME_OVER: u16 = 0x20;
pub const NA_BGM_ITEM_GET: u16 = 0x22;
pub const NA_BGM_HEART_GET: u16 = 0x24;
pub const NA_BGM_OPEN_TRE_BOX: u16 = 0x2B;
pub const NA_BGM_SMALL_ITEM_GET: u16 = 0x39;
pub const NA_BGM_GANON_TOWER: u16 = 0x2E;
pub const NA_BGM_LONLON: u16 = 0x2F;
pub const NA_BGM_KOKIRI: u16 = 0x3C;
pub const NA_BGM_GREAT_FAIRY: u16 = 0x28;
pub const NA_BGM_WINDMILL: u16 = 0x4C;
pub const NA_BGM_ESCAPE: u16 = 0x62;
pub const NA_BGM_TIMED_MINI_GAME: u16 = 0x6C;
pub const NA_BGM_CUTSCENE_EFFECTS: u16 = 0x6D;
/// `NA_BGM_NO_MUSIC`, `NA_BGM_NATURE_SFX_RAIN`, `NA_BGM_DISABLED` (`sequence.h`).
pub const NA_BGM_NO_MUSIC: u16 = 0x7F;
pub const NA_BGM_NATURE_SFX_RAIN: u16 = 0x80;
pub const NA_BGM_DISABLED: u16 = 0xFFFF;

/// `NatureAmbienceId` (`sequence.h`), the ones the ported code names.
pub const NATURE_ID_KOKIRI_REGION: u8 = 0x04;
pub const NATURE_ID_MARKET_NIGHT: u8 = 0x05;
pub const NATURE_ID_NONE: u8 = 0x13;
pub const NATURE_ID_DISABLED: u8 = 0xFF;

/// `NatureChannelIdx` (`sequence.h`).
pub const NATURE_CHANNEL_STREAM_0: u8 = 0;
pub const NATURE_CHANNEL_CRITTER_0: u8 = 1;
pub const NATURE_CHANNEL_CRITTER_1: u8 = 2;
pub const NATURE_CHANNEL_CRITTER_2: u8 = 3;
pub const NATURE_CHANNEL_CRITTER_3: u8 = 4;
pub const NATURE_CHANNEL_CRITTER_4: u8 = 5;
pub const NATURE_CHANNEL_CRITTER_5: u8 = 6;
pub const NATURE_CHANNEL_UNK: u8 = 13;
pub const NATURE_CHANNEL_RAIN: u8 = 14;
pub const NATURE_CHANNEL_LIGHTNING: u8 = 15;

/// `ChannelIOPort` (`sequence.h`).
pub const CHANNEL_IO_PORT_0: u8 = 0;
pub const CHANNEL_IO_PORT_1: u8 = 1;
pub const CHANNEL_IO_PORT_7: u8 = 7;

/// `SoundMode` (`z64audio.h`).
pub const SOUNDMODE_STEREO: i8 = 0;
pub const SOUNDMODE_HEADSET: i8 = 1;
pub const SOUNDMODE_SURROUND: i8 = 2;
pub const SOUNDMODE_MONO: i8 = 3;

/// `gSaveContext.audioSetting` 0: stereo (`func_800F6700`).
pub const AUDIO_SETTING_STEREO: i8 = 0;

/// `SFX_CHANNEL_SYSTEM0`, `SFX_CHANNEL_SYSTEM1`, `SFX_CHANNEL_OCARINA` (`code_800EC960.c`'s
/// `SfxChannelIndex`).
pub const SFX_CHANNEL_SYSTEM0: u8 = 0xB;
pub const SFX_CHANNEL_SYSTEM1: u8 = 0xC;
pub const SFX_CHANNEL_OCARINA: u8 = 0xD;

/// `SEQUENCE_TABLE`, `FONT_TABLE`, `SAMPLE_TABLE` (`z64audio.h`).
pub const SEQUENCE_TABLE: i32 = 0;
pub const FONT_TABLE: i32 = 1;
pub const SAMPLE_TABLE: i32 = 2;

/// `SEQ_FLAG_*` (`code_800EC960.c`): what `sSeqFlags` says of a sequence.
pub const SEQ_FLAG_ENEMY: u8 = 1 << 0;
pub const SEQ_FLAG_FANFARE: u8 = 1 << 1;
pub const SEQ_FLAG_FANFARE_GANON: u8 = 1 << 2;
pub const SEQ_FLAG_RESTORE: u8 = 1 << 3;
pub const SEQ_FLAG_4: u8 = 1 << 4;
pub const SEQ_FLAG_5: u8 = 1 << 5;
pub const SEQ_FLAG_6: u8 = 1 << 6;
pub const SEQ_FLAG_NO_AMBIENCE: u8 = 1 << 7;

/// `NatureAmbienceDataIO`: the player's IO ports' data, the channels it leaves alone, and the
/// (channel, port, value) triples that set the channels up, ending at 0xFF.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NatureAmbienceDataIO {
    pub player_io: u16,
    pub channel_mask: u16,
    /// `channelIO[3 * 33 + 1]`, zero past what the C lists.
    pub channel_io: Vec<u8>,
}

/// `SfxParams` (`sfx.h`), one row of a bank's table (`DEFINE_SFX`), with its enum's name.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SfxParams {
    /// `NA_SE_*`: the id is the bank's base (`NA_SE_PL_BASE`...) plus one plus the row.
    pub name: String,
    pub importance: u8,
    /// The distance and random parameters packed with the flags (`SFX_PARAM_01_MASK`,
    /// `SFX_PARAM_67_MASK`, `SFX_FLAG_*`).
    pub params: u16,
}

/// The game's audio tables, read from the C by the importer (`keys::AUDIO_GAME`).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AudioGameTables {
    /// `sSeqFlags` (`code_800EC960.c`), one per sequence (0x6E).
    pub seq_flags: Vec<u8>,
    /// `sSpecReverbs` (`code_800EC960.c`), one per audio spec (20).
    pub spec_reverbs: Vec<i8>,
    /// `sNatureAmbienceDataIO` (`code_800EC960.c`), by `NatureAmbienceId` (20).
    pub nature_ambience: Vec<NatureAmbienceDataIO>,
    /// `gSoundModeList` (`audio_external_data.c`).
    pub sound_mode_list: Vec<u8>,
    /// `sGanonsTowerLevelsVol` (`code_800EC960.c`).
    pub ganons_tower_levels_vol: Vec<u8>,
    /// `gSfxParams` (`audio_sfx_params.c`): the seven banks' tables, by `SfxBankType`.
    pub sfx_params: Vec<Vec<SfxParams>>,
    /// `gSfxBankSizes` (`audio_external_data.c`): the banks' entry arrays' sizes
    /// (`D_8016BAD0[9]`... in `code_800F7260.c`).
    pub sfx_bank_sizes: Vec<u8>,
    /// `gIsLargeSfxBank`, `gChannelsPerBank`, `gUsedChannelsPerBank` (`code_800EC960.c`): by
    /// bank; the last two by `gSfxChannelLayout` first.
    pub is_large_sfx_bank: Vec<u8>,
    pub channels_per_bank: Vec<Vec<u8>>,
    pub used_channels_per_bank: Vec<Vec<u8>>,
    /// `sBehindScreenZ` (`code_800EC960.c`).
    pub behind_screen_z: Vec<f32>,
    /// `D_801305E4` (`code_800EC960.c`): the sword charge's frequency by level.
    pub charge_freq_scales: Vec<f32>,
    /// `D_80119E10` (`z_bgcheck.c`): the footstep sound by `SURFACE_SFX_TYPE_*`
    /// (`SurfaceType_GetSfxId`), each `NA_SE_PL_WALK_* - SFX_FLAG`.
    pub floor_sfx: Vec<u16>,
    /// `z_player.c`'s `struct_80832924` tables (`D_808545DC`...; a two-dimensional one's rows
    /// as `D_80854A8C[i]`): the sounds `func_80832924` plays on an animation's frames, as
    /// (`sfxId`, `field`).
    pub player_anim_sfx: Vec<(String, Vec<(u16, i16)>)>,
}

impl AudioGameTables {
    /// `sSeqFlags[seq_id]`; 0 past the table (the C reads past it: the next statics).
    pub fn seq_flags(&self, seq_id: u8) -> u8 {
        self.seq_flags.get(seq_id as usize).copied().unwrap_or(0)
    }

    /// `gSfxParams[SFX_BANK_SHIFT(sfx_id)][SFX_INDEX(sfx_id)]`; zero past the tables.
    pub fn sfx_params(&self, sfx_id: u16) -> (u8, u16) {
        self.sfx_params.get(((sfx_id >> 12) & 0xFF) as usize).and_then(|b| b.get((sfx_id & 0x1FF) as usize)).map(|p| (p.importance, p.params)).unwrap_or((0, 0))
    }

    /// `SurfaceType_GetSfxId`'s table lookup: past it, `NA_SE_PL_WALK_GROUND - SFX_FLAG` (0).
    pub fn surface_sfx_id(&self, sfx_type: u32) -> u16 {
        self.floor_sfx.get(sfx_type as usize).copied().unwrap_or(0)
    }

    /// A `struct_80832924` table of `z_player.c` by name; empty if the pack has none.
    pub fn player_anim_sfx(&self, name: &str) -> &[(u16, i16)] {
        self.player_anim_sfx.iter().find(|(n, _)| n == name).map(|(_, t)| t.as_slice()).unwrap_or(&[])
    }

    /// The `NA_SE_*` name of `sfx_id` (its 0x800 bit aside; `None` for 0 or past the tables).
    pub fn sfx_name(&self, sfx_id: u16) -> Option<&str> {
        if sfx_id == 0 {
            return None;
        }
        self.sfx_params.get(((sfx_id >> 12) & 0xFF) as usize).and_then(|b| b.get((sfx_id & 0x1FF) as usize)).map(|p| p.name.as_str())
    }

    /// The `NA_SE_*` id named `name`.
    pub fn sfx_id(&self, name: &str) -> Option<u16> {
        self.sfx_params.iter().enumerate().find_map(|(b, bank)| bank.iter().position(|p| p.name == name).map(|i| ((b as u16) << 12) + 0x800 + i as u16))
    }
}

/// Not in the C: how many `GameOp`s wait for an audio side before the oldest go.
const MAX_UNTAKEN_OPS: usize = 1 << 16;

/// `FreqLerp` (`code_800EC960.c`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FreqLerp {
    pub value: f32,
    pub target: f32,
    pub step: f32,
    pub remaining_frames: i32,
}

/// Not in the C: an audio side a play state hands its frames to by itself
/// (`PlayState::audio_side`; headless runs and tests use `offline::OfflineAudio`).
pub trait AudioSide {
    /// After a game frame: the game's ops to the audio side (`GameAudio::take_ops`), its
    /// retraces, and the view back (`GameAudio::set_view`).
    fn hand_over(&mut self, audio: &mut GameAudio);
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Not in the C: what the game's thread did, logged for tests and traces.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AudioLog {
    /// `Audio_QueueSeqCmd`'s commands as queued, with the game frame (`GameAudio::frames`).
    pub seq_cmds: Vec<(u32, u32)>,
    /// Every `GameOp`, with the game frame.
    pub ops: Vec<(u32, GameOp)>,
    /// `Audio_PlaySfxGeneral`'s ids as asked for (muted banks included), with the game frame.
    pub sfx: Vec<(u32, u16)>,
}

/// The statics of `code_800F9280.c`, `code_800EC960.c` (its sequence parts) and
/// `audio_external_data.c`, and the hand-over to the audio side.
#[derive(Debug, Clone)]
pub struct GameAudio {
    pub tables: Arc<AudioGameTables>,
    /// The audio library's tables (`gSequenceFontTable` for `func_800E5E84`).
    pub audio_tables: Arc<AudioTables>,
    /// What the audio side gave back last (`AudioContext::view`).
    pub view: AudioView,
    /// What this frame did to the library, for the audio side.
    ops: Vec<GameOp>,
    /// Not in the C: the game frames run (`PlayState::tick_with` counts them), the log's frame
    /// numbers: 0 is the boot and the first `Play_Init`.
    pub frames: u32,
    /// Not in the C: the log, if asked for.
    pub log: Option<AudioLog>,

    // audio_external_data.c
    pub seq_cmd_wr_pos: u8,
    pub seq_cmd_rd_pos: u8,
    /// `D_80133408`: sequences other than the sfx player's don't start (seq command `0xE01`).
    pub d_80133408: u8,
    pub audio_spec_id: u8,
    /// `D_80133418`: a spec change waits for the audio side's reset.
    pub d_80133418: u8,
    pub sfx_channel_layout: u8,

    // code_800F9280.c
    /// `D_8016E320`: each player's queue of sequences (`unk_0` the id, `unk_1` the priority).
    pub d_8016e320: [[(u8, u8); 5]; 4],
    /// `D_8016E348`: how many each queue holds.
    pub d_8016e348: [u8; 4],
    /// `sAudioSeqCmds`.
    pub seq_cmds: Box<[u32; 0x100]>,
    /// `D_8016E750`.
    pub active: [ActiveSeq; 4],

    // code_800EC960.c
    pub sound_mode: i8,
    pub d_80130608: i8,
    pub audio_cutscene_flag: i8,
    pub spec_reverb: i8,
    pub audio_env_reverb: i8,
    pub audio_code_reverb: i8,
    pub prev_seq_mode: u8,
    pub audio_enemy_dist: f32,
    pub audio_enemy_vol: i8,
    pub prev_main_bgm_seq_id: u16,
    pub d_8013062c: u8,
    pub d_80130630: u8,
    pub num_frames_still: u32,
    pub num_frames_moving: u32,
    pub audio_base_filter: u8,
    pub audio_extra_filter: u8,
    pub audio_base_filter2: u8,
    pub audio_extra_filter2: u8,
    /// `sSariaBgmPtr`: whether it's set (the C keeps the actor's position's address).
    pub saria_bgm_set: bool,
    pub seq_mode_input: u8,
    pub enter_ganons_tower_timer: u8,
    pub d_8016b7a8: f32,
    pub d_8016b7b0: f32,
    pub river_freq_scale_lerp: FreqLerp,
    pub waterfall_freq_scale_lerp: FreqLerp,
    pub d_8016b7d8: f32,
    pub river_sound_main_bgm_vol: u8,
    pub river_sound_main_bgm_current_vol: u8,
    pub river_sound_main_bgm_lower: bool,
    pub river_sound_main_bgm_restore: bool,
    pub ganons_tower_vol: u8,
    pub d_8016b9d8: u8,
    pub d_8016b9f2: u8,
    pub d_8016b9f3: u8,
    pub d_8016b9f4: u8,
    pub d_8016b9f6: u16,
    /// `sSfxChannelState`.
    pub sfx_channel_state: [bgm::SfxPlayerState; 16],

    // code_800F7260.c
    /// `gSfxBankMuted`.
    pub sfx_bank_muted: [u8; 7],
    /// The rest of the sound effects' statics (`sfx`).
    pub sfx: sfx::SfxStatics,
}

impl Default for GameAudio {
    fn default() -> GameAudio {
        GameAudio::new(Arc::default(), Arc::default())
    }
}

impl GameAudio {
    /// The statics as the ROM loads them (their initialisers, BSS zero).
    pub fn new(tables: Arc<AudioGameTables>, audio_tables: Arc<AudioTables>) -> GameAudio {
        let sfx = sfx::SfxStatics::new(&tables.sfx_bank_sizes);
        GameAudio {
            sfx,
            tables,
            audio_tables,
            view: AudioView::default(),
            ops: Vec::new(),
            frames: 0,
            log: None,
            seq_cmd_wr_pos: 0,
            seq_cmd_rd_pos: 0,
            d_80133408: 0,
            audio_spec_id: 0,
            d_80133418: 0,
            sfx_channel_layout: 0,
            d_8016e320: [[(0, 0); 5]; 4],
            d_8016e348: [0; 4],
            seq_cmds: Box::new([0; 0x100]),
            active: [ActiveSeq::default(); 4],
            sound_mode: SOUNDMODE_SURROUND,
            d_80130608: 0,
            audio_cutscene_flag: 0,
            spec_reverb: 0,
            audio_env_reverb: 0,
            audio_code_reverb: 0,
            prev_seq_mode: 0,
            audio_enemy_dist: 0.0,
            audio_enemy_vol: 127,
            prev_main_bgm_seq_id: NA_BGM_DISABLED,
            d_8013062c: 0,
            d_80130630: NA_BGM_GENERAL_SFX as u8,
            num_frames_still: 0,
            num_frames_moving: 0,
            audio_base_filter: 0,
            audio_extra_filter: 0,
            audio_base_filter2: 0,
            audio_extra_filter2: 0,
            saria_bgm_set: false,
            seq_mode_input: 0,
            enter_ganons_tower_timer: 0,
            d_8016b7a8: 0.0,
            d_8016b7b0: 0.0,
            river_freq_scale_lerp: FreqLerp::default(),
            waterfall_freq_scale_lerp: FreqLerp::default(),
            d_8016b7d8: 0.0,
            river_sound_main_bgm_vol: 0,
            river_sound_main_bgm_current_vol: 0,
            river_sound_main_bgm_lower: false,
            river_sound_main_bgm_restore: false,
            ganons_tower_vol: 0,
            d_8016b9d8: 0,
            d_8016b9f2: 0,
            d_8016b9f3: 0,
            d_8016b9f4: 0,
            d_8016b9f6: 0,
            sfx_channel_state: [bgm::SfxPlayerState::default(); 16],
            sfx_bank_muted: [0; 7],
        }
    }

    /// The audio manager's start (`AudioMgr_ThreadEntry`): `Audio_Init` is the audio side's
    /// (`AudioContext::new`; what the game reads of it then is `AudioView::boot`), then
    /// `Audio_InitSound`; then what the title screen leaves (its game state isn't ported):
    /// `Sram_InitSram`'s `func_800F6700(gSaveContext.audioSetting)`, the SRAM header's sound
    /// setting, `SRAM_HEADER_SOUND`, 0 (stereo) as `sZeldaMagic` writes a fresh one.
    pub fn boot(tables: Arc<AudioGameTables>, audio_tables: Arc<AudioTables>) -> GameAudio {
        Self::boot_logged(tables, audio_tables, false)
    }

    /// `boot`, logging from the start if `log` (`GameAudio::log`).
    pub fn boot_logged(tables: Arc<AudioGameTables>, audio_tables: Arc<AudioTables>, log: bool) -> GameAudio {
        let mut a = GameAudio::new(tables, audio_tables);
        if log {
            a.log = Some(AudioLog::default());
        }
        a.view = AudioView::boot(&a.audio_tables);
        a.audio_init_sound();
        a.func_800f6700(AUDIO_SETTING_STEREO);
        a
    }

    /// Takes what the audio side gave back after its retraces (before the game's next frame).
    pub fn set_view(&mut self, view: AudioView) {
        self.view.update(view);
    }

    /// Takes what the game's thread did since the last call, for the audio side.
    pub fn take_ops(&mut self) -> Vec<GameOp> {
        std::mem::take(&mut self.ops)
    }

    pub(crate) fn op(&mut self, op: GameOp) {
        if let Some(l) = &mut self.log {
            l.ops.push((self.frames, op));
        }
        // With no audio side taking them (headless runs without sound, `--no-audio`), only the
        // latest are kept.
        if self.ops.len() >= MAX_UNTAKEN_OPS {
            self.ops.drain(..MAX_UNTAKEN_OPS / 2);
        }
        self.ops.push(op);
    }

    /// `Audio_QueueCmd`.
    pub fn queue_cmd(&mut self, op_args: u32, data: u32) {
        self.op(GameOp::Cmd(op_args, data));
    }
    /// `Audio_QueueCmdF32`.
    pub fn queue_cmd_f32(&mut self, op_args: u32, data: f32) {
        self.queue_cmd(op_args, data.to_bits());
    }
    /// `Audio_QueueCmdS32`.
    pub fn queue_cmd_s32(&mut self, op_args: u32, data: i32) {
        self.queue_cmd(op_args, data as u32);
    }
    /// `Audio_QueueCmdS8`.
    pub fn queue_cmd_s8(&mut self, op_args: u32, data: i8) {
        self.queue_cmd(op_args, ((data as i32) << 0x18) as u32);
    }
    /// `Audio_QueueCmdU16`.
    pub fn queue_cmd_u16(&mut self, op_args: u32, data: u16) {
        self.queue_cmd(op_args, (data as u32) << 0x10);
    }
    /// `Audio_ScheduleProcessCmds`.
    pub fn schedule_process_cmds(&mut self) {
        self.op(GameOp::Schedule);
    }

    /// `func_800E5E20`: a message from `externalLoadQueue`: its top byte, and the rest in
    /// `out`; 0 if none.
    pub fn func_800e5e20(&mut self, out: &mut u32) -> u32 {
        if self.view.external_load_msgs.is_empty() {
            *out = 0;
            return 0;
        }
        let m = self.view.external_load_msgs.remove(0);
        *out = m & 0xFF_FFFF;
        m >> 0x18
    }

    /// `func_800E5E84` (`AudioLoad_GetFontsForSequence`): the fonts sequence `seq_id` uses.
    pub fn func_800e5e84(&self, seq_id: u8) -> &[u8] {
        self.audio_tables.fonts_for_sequence(seq_id as u32)
    }

    /// `func_800E5EDC`: a message from `audioResetQueue`: 1 if it's the reset to the spec
    /// asked for, -1 if another, 0 if none.
    pub fn func_800e5edc(&mut self) -> i32 {
        if self.view.reset_msgs.is_empty() {
            return 0;
        }
        let m = self.view.reset_msgs.remove(0);
        if self.view.audio_reset_spec_id_to_load as u32 != m { -1 } else { 1 }
    }

    /// `func_800E5F88`: the spec change. The game's thread empties `audioResetQueue` (its copy
    /// of the messages, `func_800E5F34`); the rest runs on the audio side
    /// (`AudioContext::func_800e5f88`).
    pub fn func_800e5f88(&mut self, reset_preload_id: u8) {
        self.view.reset_msgs.clear();
        self.op(GameOp::ResetSpec(reset_preload_id));
    }

    /// `func_800E6070`: a channel's IO port, -1 with its player off.
    pub fn func_800e6070(&self, player_idx: u8, channel_idx: u8, script_idx: u8) -> i8 {
        let p = &self.view.players[player_idx as usize & 3];
        if p.enabled { p.channels[channel_idx as usize & 0xF].sound_script_io[script_idx as usize & 7] } else { -1 }
    }

    /// `func_800E60C4`: a player's IO port.
    pub fn func_800e60c4(&self, player_idx: u8, port: u8) -> i8 {
        self.view.players[player_idx as usize & 3].sound_script_io[port as usize & 7]
    }
}
