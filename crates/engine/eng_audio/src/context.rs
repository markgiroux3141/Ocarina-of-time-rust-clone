//! `gAudioCtx` (`AudioContext`, `audio.h`) and the structs it holds.
//!
//! The port keeps the C's layout of ownership: the notes, layers, channels and players are
//! arrays, and what points at them points by index (`NoteId`, `LayerId`, `ChanId`); the lists
//! they move between (`AudioListItem`, `NotePool`) are an arena of nodes with the C's links.
//! What points at data the game loads (fonts, sequences, samples, envelopes, the RSP's state)
//! is a RAM address, as in the C (`ram.rs`): the structs that live in RAM there are read and
//! written in RAM here (`Sample`, `Instrument`, `Drum`, `AdpcmLoop`, `EnvelopePoint`, ...).

use std::collections::VecDeque;
use std::sync::Arc;

use crate::data::AudioTables;
use crate::ram::{Cart, Ram};
use crate::rsp::{Acmd, Rsp};

pub type NoteId = usize;
pub type LayerId = usize;
/// A channel: `channels[0]` is `sequenceChannelNone`, the others each player's.
pub type ChanId = usize;
pub type PoolId = usize;
pub type NodeId = usize;

/// `&gAudioCtx.sequenceChannelNone`.
pub const CHANNEL_NONE: ChanId = 0;

// `audio.h`.
pub const SEQTICKS_PER_BEAT: i32 = 48;
pub const SEQ_NUM_CHANNELS: usize = 16;
pub const MUTE_BEHAVIOR_3: u8 = 1 << 3;
pub const MUTE_BEHAVIOR_4: u8 = 1 << 4;
pub const MUTE_BEHAVIOR_SOFTEN: u8 = 1 << 5;
pub const MUTE_BEHAVIOR_STOP_NOTES: u8 = 1 << 6;
pub const MUTE_BEHAVIOR_STOP_SCRIPT: u8 = 1 << 7;
pub const ADSR_DISABLE: i16 = 0;
pub const ADSR_HANG: i16 = -1;
pub const ADSR_GOTO: i16 = -2;
pub const ADSR_RESTART: i16 = -3;
pub const SAMPLE_SIZE: i32 = 2;
pub const SAMPLES_PER_FRAME: i32 = 16;
pub const DMEM_1CH_SIZE: i32 = 13 * SAMPLES_PER_FRAME * SAMPLE_SIZE;
pub const DMEM_2CH_SIZE: i32 = 2 * DMEM_1CH_SIZE;
pub const AIBUF_LEN: i32 = 88 * SAMPLES_PER_FRAME;
pub const AIBUF_SIZE: i32 = AIBUF_LEN * SAMPLE_SIZE;
pub const WAVE_SAMPLE_COUNT: i32 = 64;

pub const SOUND_OUTPUT_STEREO: i8 = 0;
pub const SOUND_OUTPUT_HEADSET: i8 = 1;
pub const SOUND_OUTPUT_SURROUND: i8 = 2;
pub const SOUND_OUTPUT_MONO: i8 = 3;

pub const ADSR_STATE_DISABLED: u8 = 0;
pub const ADSR_STATE_INITIAL: u8 = 1;
pub const ADSR_STATE_START_LOOP: u8 = 2;
pub const ADSR_STATE_LOOP: u8 = 3;
pub const ADSR_STATE_FADE: u8 = 4;
pub const ADSR_STATE_HANG: u8 = 5;
pub const ADSR_STATE_DECAY: u8 = 6;
pub const ADSR_STATE_RELEASE: u8 = 7;
pub const ADSR_STATE_SUSTAIN: u8 = 8;

pub const MEDIUM_RAM: u8 = 0;
pub const MEDIUM_UNK: u8 = 1;
pub const MEDIUM_CART: u8 = 2;
pub const MEDIUM_DISK_DRIVE: u8 = 3;

pub const CODEC_ADPCM: u8 = 0;
pub const CODEC_S8: u8 = 1;
pub const CODEC_S16_INMEMORY: u8 = 2;
pub const CODEC_SMALL_ADPCM: u8 = 3;
pub const CODEC_REVERB: u8 = 4;
pub const CODEC_S16: u8 = 5;

pub const SEQUENCE_TABLE: i32 = 0;
pub const FONT_TABLE: i32 = 1;
pub const SAMPLE_TABLE: i32 = 2;

pub const CACHE_TEMPORARY: i32 = 0;
pub const CACHE_PERSISTENT: i32 = 1;
pub const CACHE_EITHER: i32 = 2;
pub const CACHE_PERMANENT: i32 = 3;

pub const LOAD_STATUS_NOT_LOADED: u8 = 0;
pub const LOAD_STATUS_IN_PROGRESS: u8 = 1;
pub const LOAD_STATUS_COMPLETE: u8 = 2;
pub const LOAD_STATUS_DISCARDABLE: u8 = 3;
pub const LOAD_STATUS_MAYBE_DISCARDABLE: u8 = 4;
pub const LOAD_STATUS_PERMANENTLY_LOADED: u8 = 5;

/// The `sizeof`s the heap's allocations take (`audio.h`'s size comments, `abi.h`).
pub const SIZEOF_NOTE: u32 = 0xE0;
pub const SIZEOF_SEQUENCE_CHANNEL: u32 = 0xD4;
pub const SIZEOF_NOTE_SUB_EU: u32 = 0x20;
pub const SIZEOF_ACMD: u32 = 8;
pub const SIZEOF_NOTE_SYNTHESIS_BUFFERS: u32 = 0x1E0;
pub const SIZEOF_RESAMPLE_STATE: u32 = 0x20;
pub const SIZEOF_SAMPLE_DMA: u32 = 0x10;
pub const SIZEOF_SOUND_FONT: u32 = 0x14;
/// Where `NoteAttributes.filterBuf` is in a `Note` (`playbackState` 0x30 + `attributes` 0x1C +
/// `filterBuf` 0x14).
pub const NOTE_FILTER_BUF_OFFSET: u32 = 0x60;

/// `NoteSynthesisBuffers`' members.
pub const NSB_ADPCMDEC_STATE: u32 = 0x000;
pub const NSB_FINAL_RESAMPLE_STATE: u32 = 0x020;
pub const NSB_MIX_ENVELOPE_STATE: u32 = 0x040;
pub const NSB_HAAS_EFFECT_DELAY_STATE: u32 = 0x0A0;
pub const NSB_UNK_STATE: u32 = 0x0E0;

/// `ALIGN16` and friends.
pub const fn align16(v: u32) -> u32 {
    (v + 0xF) & !0xF
}
pub const fn align8(v: i32) -> i32 {
    (v + 7) & !7
}
pub const fn align64(v: i32) -> i32 {
    (v + 0x3F) & !0x3F
}
pub const fn align256(v: i32) -> i32 {
    (v + 0xFF) & !0xFF
}

/// `SeqScriptState`: a script's counter and call stack, as RAM addresses into the sequence.
///
/// Kept as the struct's 0x1C bytes (`pc` 0x00, `stack[4]` 0x04, `remLoopIters[4]` 0x14, `depth`
/// 0x18, `value` 0x19), because the C indexes its arrays by `depth` unchecked: a fifth nested
/// call writes `stack[4]`, which is `remLoopIters` (`@bug (game)`, `AudioSeq_SequenceChannelProcessScript`'s
/// 0xE4). Indexes past the struct are dropped.
#[derive(Debug, Clone, Copy, Default)]
pub struct SeqScriptState {
    bytes: [u8; 0x1C],
}

impl SeqScriptState {
    fn get32(&self, o: usize) -> u32 {
        match self.bytes.get(o..o + 4) {
            Some(b) => u32::from_be_bytes([b[0], b[1], b[2], b[3]]),
            None => 0,
        }
    }
    fn set32(&mut self, o: usize, v: u32) {
        if let Some(b) = self.bytes.get_mut(o..o + 4) {
            b.copy_from_slice(&v.to_be_bytes());
        }
    }
    pub fn pc(&self) -> u32 {
        self.get32(0)
    }
    pub fn set_pc(&mut self, v: u32) {
        self.set32(0, v)
    }
    /// `stack[i]` (`i` may run past the array, as the C's does).
    pub fn stack(&self, i: i32) -> u32 {
        if i < 0 { 0 } else { self.get32(4 + 4 * i as usize) }
    }
    pub fn set_stack(&mut self, i: i32, v: u32) {
        if i >= 0 {
            self.set32(4 + 4 * i as usize, v)
        }
    }
    /// `remLoopIters[i]`.
    pub fn rem_loop_iters(&self, i: i32) -> u8 {
        let o = 0x14 + i as isize;
        if (0..0x1C).contains(&o) { self.bytes[o as usize] } else { 0 }
    }
    pub fn set_rem_loop_iters(&mut self, i: i32, v: u8) {
        let o = 0x14 + i as isize;
        if (0..0x1C).contains(&o) {
            self.bytes[o as usize] = v;
        }
    }
    pub fn depth(&self) -> u8 {
        self.bytes[0x18]
    }
    pub fn set_depth(&mut self, v: u8) {
        self.bytes[0x18] = v;
    }
    pub fn value(&self) -> i8 {
        self.bytes[0x19] as i8
    }
    pub fn set_value(&mut self, v: i8) {
        self.bytes[0x19] = v as u8;
    }
}

/// `AdsrSettings`.
#[derive(Debug, Clone, Copy, Default)]
pub struct AdsrSettings {
    pub decay_index: u8,
    pub sustain: u8,
    /// `EnvelopePoint*`.
    pub envelope: u32,
}

/// `AdsrState`. `action` is the byte of its bitfield union: `unused` (0x80), `hang` (0x40),
/// `decay` (0x20), `release` (0x10) and `state` (the low four bits).
#[derive(Debug, Clone, Copy, Default)]
pub struct AdsrState {
    pub action: u8,
    pub env_index: u8,
    pub delay: i16,
    pub sustain: f32,
    pub velocity: f32,
    pub fade_out_vel: f32,
    pub current: f32,
    pub target: f32,
    pub envelope: u32,
}

impl AdsrState {
    pub fn state(&self) -> u8 {
        self.action & 0xF
    }
    pub fn set_state(&mut self, s: u8) {
        self.action = (self.action & 0xF0) | (s & 0xF);
    }
    pub fn hang(&self) -> bool {
        self.action & 0x40 != 0
    }
    pub fn decay(&self) -> bool {
        self.action & 0x20 != 0
    }
    pub fn set_decay(&mut self, v: bool) {
        self.action = (self.action & !0x20) | if v { 0x20 } else { 0 };
    }
    pub fn release(&self) -> bool {
        self.action & 0x10 != 0
    }
    pub fn set_release(&mut self, v: bool) {
        self.action = (self.action & !0x10) | if v { 0x10 } else { 0 };
    }
}

/// `Portamento`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Portamento {
    pub mode: u8,
    pub cur: u16,
    pub speed: u16,
    pub extent: f32,
}

/// `Stereo`'s byte: `unused` (bits 7-6), `bit2` (5-4), `strongRight` (3), `strongLeft` (2),
/// `stereoHeadsetEffects` (1), `usesHeadsetPanEffects` (0).
pub mod stereo {
    pub fn bit2(b: u8) -> u8 {
        (b >> 4) & 3
    }
    pub fn strong_right(b: u8) -> bool {
        b & 0x08 != 0
    }
    pub fn strong_left(b: u8) -> bool {
        b & 0x04 != 0
    }
    pub fn stereo_headset_effects(b: u8) -> bool {
        b & 0x02 != 0
    }
    pub fn uses_headset_pan_effects(b: u8) -> bool {
        b & 0x01 != 0
    }
}

/// `SequencePlayer`.
#[derive(Debug, Clone, Default)]
pub struct SequencePlayer {
    pub enabled: bool,
    pub finished: bool,
    pub muted: bool,
    pub seq_dma_in_progress: bool,
    pub font_dma_in_progress: bool,
    pub recalculate_volume: bool,
    pub stop_script: bool,
    pub apply_bend: bool,
    pub state: u8,
    pub note_alloc_policy: u8,
    pub mute_behavior: u8,
    pub seq_id: u8,
    pub default_font: u8,
    pub player_idx: i8,
    pub tempo: u16,
    pub tempo_acc: u16,
    pub tempo_change: u16,
    pub transposition: i16,
    pub delay: u16,
    pub fade_timer: u16,
    pub fade_timer_unk_eu: u16,
    pub seq_data: u32,
    pub fade_volume: f32,
    pub fade_velocity: f32,
    pub volume: f32,
    pub mute_volume_scale: f32,
    pub fade_volume_scale: f32,
    pub applied_fade_volume: f32,
    pub bend: f32,
    pub channels: [ChanId; 16],
    pub script_state: SeqScriptState,
    pub short_note_velocity_table: u32,
    pub short_note_gate_time_table: u32,
    pub note_pool: PoolId,
    pub skip_ticks: i32,
    pub script_counter: u32,
    pub sound_script_io: [i8; 8],
}

/// `SequenceChannel`.
#[derive(Debug, Clone, Default)]
pub struct SequenceChannel {
    pub enabled: bool,
    pub finished: bool,
    pub stop_script: bool,
    pub stop_something2: bool,
    pub has_instrument: bool,
    pub stereo_headset_effects: bool,
    pub large_notes: bool,
    pub unused: bool,
    /// `changes`: `freqScale` (0x80), `volume` (0x40), `pan` (0x20).
    pub changes: u8,
    pub note_alloc_policy: u8,
    pub mute_behavior: u8,
    pub reverb: u8,
    pub note_priority: u8,
    pub some_other_priority: u8,
    pub font_id: u8,
    pub reverb_index: u8,
    pub book_offset: u8,
    pub new_pan: u8,
    pub pan_channel_weight: u8,
    pub gain: u8,
    pub velocity_random_variance: u8,
    pub gate_time_random_variance: u8,
    pub comb_filter_size: u8,
    pub vibrato_rate_start: u16,
    pub vibrato_extent_start: u16,
    pub vibrato_rate_target: u16,
    pub vibrato_extent_target: u16,
    pub vibrato_rate_change_delay: u16,
    pub vibrato_extent_change_delay: u16,
    pub vibrato_delay: u16,
    pub delay: u16,
    pub comb_filter_gain: u16,
    pub unk_22: u16,
    pub inst_or_wave: i16,
    pub transposition: i16,
    pub volume_scale: f32,
    pub volume: f32,
    pub pan: i32,
    pub applied_volume: f32,
    pub freq_scale: f32,
    /// `u8 (*dynTable)[][2]`, in the sequence.
    pub dyn_table: u32,
    pub note_unused: Option<NoteId>,
    pub layer_unused: Option<LayerId>,
    /// `Instrument*`: an instrument in a font, or the C's `(Instrument*)1` (drums) and
    /// `(Instrument*)2` (sound effects).
    pub instrument: u32,
    /// `seqPlayer`: none for `sequenceChannelNone`.
    pub seq_player: Option<usize>,
    pub layers: [Option<LayerId>; 4],
    pub script_state: SeqScriptState,
    pub adsr: AdsrSettings,
    pub note_pool: PoolId,
    pub sound_script_io: [i8; 8],
    /// `s16* filter`, in the sequence.
    pub filter: u32,
    pub stereo: u8,
}

pub const CHANGES_FREQ_SCALE: u8 = 0x80;
pub const CHANGES_VOLUME: u8 = 0x40;
pub const CHANGES_PAN: u8 = 0x20;

/// `SequenceLayer`.
#[derive(Debug, Clone, Default)]
pub struct SequenceLayer {
    pub enabled: bool,
    pub finished: bool,
    pub stop_something: bool,
    pub continuous_notes: bool,
    pub bit3: bool,
    pub ignore_drum_pan: bool,
    pub bit1: bool,
    pub note_properties_need_init: bool,
    pub stereo: u8,
    pub inst_or_wave: u8,
    pub gate_time: u8,
    pub semitone: u8,
    pub portamento_target_note: u8,
    pub pan: u8,
    pub note_pan: u8,
    pub delay: i16,
    pub gate_delay: i16,
    pub delay2: i16,
    pub portamento_time: u16,
    pub transposition: i16,
    pub short_note_default_delay: i16,
    pub last_delay: i16,
    pub adsr: AdsrSettings,
    pub portamento: Portamento,
    pub note: Option<NoteId>,
    pub freq_scale: f32,
    pub bend: f32,
    pub velocity_square2: f32,
    pub velocity_square: f32,
    pub note_velocity: f32,
    pub note_freq_scale: f32,
    /// `Instrument*`.
    pub instrument: u32,
    /// `TunedSample*`.
    pub tuned_sample: u32,
    pub channel: Option<ChanId>,
    pub script_state: SeqScriptState,
}

/// `NoteSynthesisState`.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoteSynthesisState {
    pub restart: u8,
    pub sample_dma_index: u8,
    pub prev_haas_effect_left_delay_size: u8,
    pub prev_haas_effect_right_delay_size: u8,
    pub reverb_vol: u8,
    pub num_parts: u8,
    pub sample_pos_frac: u16,
    pub sample_pos_int: i32,
    /// `NoteSynthesisBuffers*`.
    pub synthesis_buffers: u32,
    pub cur_vol_left: u16,
    pub cur_vol_right: u16,
    pub comb_filter_needs_init: u8,
}

/// `VibratoState`.
#[derive(Debug, Clone, Copy, Default)]
pub struct VibratoState {
    pub channel: Option<ChanId>,
    pub time: u32,
    /// `s16* curve`.
    pub curve: u32,
    pub extent: f32,
    pub rate: f32,
    pub active: u8,
    pub rate_change_timer: u16,
    pub extent_change_timer: u16,
    pub delay: u16,
}

/// `NoteAttributes`. `filterBuf` lives in the note in RAM (`NOTE_FILTER_BUF_OFFSET`).
#[derive(Debug, Clone, Copy, Default)]
pub struct NoteAttributes {
    pub reverb: u8,
    pub gain: u8,
    pub pan: u8,
    pub stereo: u8,
    pub comb_filter_size: u8,
    pub comb_filter_gain: u16,
    pub freq_scale: f32,
    pub velocity: f32,
    pub filter: u32,
}

/// `NotePlaybackState`. The layer links: `None` is `NO_LAYER`.
#[derive(Debug, Clone, Copy, Default)]
pub struct NotePlaybackState {
    pub priority: u8,
    pub wave_id: u8,
    pub harmonic_index: u8,
    pub font_id: u8,
    pub unk_04: u8,
    pub stereo_headset_effects: u8,
    pub adsr_vol_scale_unused: i16,
    pub portamento_freq_scale: f32,
    pub vibrato_freq_scale: f32,
    pub prev_parent_layer: Option<LayerId>,
    pub parent_layer: Option<LayerId>,
    pub wanted_parent_layer: Option<LayerId>,
    pub attributes: NoteAttributes,
    pub adsr: AdsrState,
    pub portamento: Portamento,
    pub vibrato_state: VibratoState,
}

/// `NoteSampleState`: what one update hands the synthesis for a note.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct NoteSampleState {
    pub enabled: bool,
    pub needs_init: bool,
    pub finished: bool,
    pub unused: bool,
    pub stereo_strong_right: bool,
    pub stereo_strong_left: bool,
    pub stereo_headset_effects: bool,
    pub uses_headset_pan_effects: bool,
    /// 3 bits.
    pub reverb_index: u8,
    /// 2 bits.
    pub book_offset: u8,
    pub is_synthetic_wave: bool,
    pub has_two_parts: bool,
    pub use_haas_effect: bool,
    pub gain: u8,
    pub haas_effect_left_delay_size: u8,
    pub haas_effect_right_delay_size: u8,
    pub reverb_vol: u8,
    pub harmonic_index_cur_and_prev: u8,
    pub comb_filter_size: u8,
    pub target_vol_left: u16,
    pub target_vol_right: u16,
    pub resampling_rate_fixed_point: u16,
    pub comb_filter_gain: u16,
    /// The union of `TunedSample* tunedSample` and `s16* waveSampleAddr`.
    pub tuned_sample: u32,
    /// `s16* filter`.
    pub filter: u32,
}

impl NoteSampleState {
    /// `bitField0` and `bitField1` (`sub->bitField0 = note->sampleState.bitField0`).
    pub fn copy_bitfields_from(&mut self, o: &NoteSampleState) {
        self.enabled = o.enabled;
        self.needs_init = o.needs_init;
        self.finished = o.finished;
        self.unused = o.unused;
        self.stereo_strong_right = o.stereo_strong_right;
        self.stereo_strong_left = o.stereo_strong_left;
        self.stereo_headset_effects = o.stereo_headset_effects;
        self.uses_headset_pan_effects = o.uses_headset_pan_effects;
        self.reverb_index = o.reverb_index;
        self.book_offset = o.book_offset;
        self.is_synthetic_wave = o.is_synthetic_wave;
        self.has_two_parts = o.has_two_parts;
        self.use_haas_effect = o.use_haas_effect;
    }
}

/// `Note`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Note {
    pub synthesis_state: NoteSynthesisState,
    pub playback_state: NotePlaybackState,
    pub start_sample_pos: u32,
    pub note_sub_eu: NoteSampleState,
}

/// `NoteSampleStateAttributes`.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoteSampleStateAttributes {
    pub reverb_vol: u8,
    pub gain: u8,
    pub pan: u8,
    pub stereo: u8,
    pub frequency: f32,
    pub velocity: f32,
    pub filter: u32,
    pub comb_filter_size: u8,
    pub comb_filter_gain: u16,
}

/// `ReverbRingBufferItem`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ReverbRingBufferItem {
    pub num_samples_after_downsampling: i16,
    pub chunk_len: i16,
    pub to_downsample_left: u32,
    pub to_downsample_right: u32,
    pub start_pos: i32,
    pub length_a: i16,
    pub length_b: i16,
    pub unk_14: u16,
    pub unk_16: u16,
    pub unk_18: u16,
}

/// `SynthesisReverb`. `tunedSample`, `sample` and `loop` are in RAM (`tuned_sample_addr`):
/// a layer can play the reverb's ring buffer as an instrument (`instOrWave` 0xC0 on).
#[derive(Debug, Clone, Copy, Default)]
pub struct SynthesisReverb {
    pub resample_flags: u8,
    pub use_reverb: u8,
    pub frames_to_ignore: u8,
    pub cur_frame: u8,
    pub downsample_rate: u8,
    pub unk_05: i8,
    pub window_size: u16,
    pub unk_08: i16,
    pub volume: i16,
    pub decay_ratio: u16,
    pub unk_0e: u16,
    pub leak_rtl: i16,
    pub leak_ltr: i16,
    pub unk_14: u16,
    pub unk_16: i16,
    pub unk_18: u8,
    pub next_ring_buf_pos: i32,
    pub unk_20: i32,
    pub buf_size_per_chan: i32,
    pub left_ring_buf: u32,
    pub right_ring_buf: u32,
    pub unk_30: u32,
    pub unk_34: u32,
    pub unk_38: u32,
    pub unk_3c: u32,
    pub items: [[ReverbRingBufferItem; 5]; 2],
    pub items2: [[ReverbRingBufferItem; 5]; 2],
    pub filter_left: u32,
    pub filter_right: u32,
    pub filter_left_state: u32,
    pub filter_right_state: u32,
    /// `&reverb->tunedSample` (then `sample` at +8 and `loop` at +0x18).
    pub tuned_sample_addr: u32,
}

/// `SoundFont`: a font's counts and, once relocated, its lists (RAM addresses).
#[derive(Debug, Clone, Copy, Default)]
pub struct SoundFont {
    pub num_instruments: u8,
    pub num_drums: u8,
    pub sample_bank_id1: u8,
    pub sample_bank_id2: u8,
    pub num_sfx: u16,
    pub instruments: u32,
    pub drums: u32,
    pub sound_effects: u32,
}

/// `AudioTableEntry`.
#[derive(Debug, Clone, Copy, Default)]
pub struct AudioTableEntry {
    pub rom_addr: u32,
    pub size: u32,
    pub medium: i8,
    pub cache_policy: i8,
    pub short_data1: i16,
    pub short_data2: i16,
    pub short_data3: i16,
}

/// `AudioTable`.
#[derive(Debug, Clone, Default)]
pub struct AudioTable {
    pub num_entries: i16,
    pub unk_medium_param: i16,
    pub rom_addr: u32,
    pub entries: Vec<AudioTableEntry>,
}

impl AudioTable {
    /// Reads the ROM's table (`s16 numEntries, s16 unkMediumParam, u32 romAddr, pad[8]`, then
    /// 16-byte entries).
    pub fn parse(b: &[u8]) -> AudioTable {
        let be16 = |o: usize| -> u16 { ((b.get(o).copied().unwrap_or(0) as u16) << 8) | b.get(o + 1).copied().unwrap_or(0) as u16 };
        let be32 = |o: usize| -> u32 { ((be16(o) as u32) << 16) | be16(o + 2) as u32 };
        let n = be16(0) as i16;
        let entries = (0..n.max(0) as usize)
            .map(|i| {
                let e = 16 + i * 16;
                AudioTableEntry {
                    rom_addr: be32(e),
                    size: be32(e + 4),
                    medium: b.get(e + 8).copied().unwrap_or(0) as i8,
                    cache_policy: b.get(e + 9).copied().unwrap_or(0) as i8,
                    short_data1: be16(e + 10) as i16,
                    short_data2: be16(e + 12) as i16,
                    short_data3: be16(e + 14) as i16,
                }
            })
            .collect();
        AudioTable { num_entries: n, unk_medium_param: be16(2) as i16, rom_addr: be32(4), entries }
    }
}

/// `AudioBufferParameters`.
#[derive(Debug, Clone, Copy, Default)]
pub struct AudioBufferParameters {
    pub spec_unk4: i16,
    pub sampling_frequency: u16,
    pub ai_sampling_frequency: u16,
    pub samples_per_frame_target: i16,
    pub max_ai_buffer_length: i16,
    pub min_ai_buffer_length: i16,
    pub updates_per_frame: i16,
    pub samples_per_update: i16,
    pub samples_per_update_max: i16,
    pub samples_per_update_min: i16,
    pub num_sequence_players: i16,
    pub resample_rate: f32,
    pub updates_per_frame_inv: f32,
    pub updates_per_frame_inv_scaled: f32,
    pub updates_per_frame_scaled: f32,
}

/// `AudioAllocPool`.
#[derive(Debug, Clone, Copy, Default)]
pub struct AudioAllocPool {
    pub start_ram_addr: u32,
    pub cur_ram_addr: u32,
    pub size: i32,
    pub num_entries: i32,
}

/// `AudioCacheEntry`.
#[derive(Debug, Clone, Copy, Default)]
pub struct AudioCacheEntry {
    pub ram_addr: u32,
    pub size: u32,
    pub table_type: i16,
    pub id: i16,
}

/// `AudioPersistentCache`.
#[derive(Debug, Clone, Copy, Default)]
pub struct AudioPersistentCache {
    pub num_entries: u32,
    pub pool: AudioAllocPool,
    pub entries: [AudioCacheEntry; 16],
}

/// `AudioTemporaryCache`.
#[derive(Debug, Clone, Copy, Default)]
pub struct AudioTemporaryCache {
    pub next_side: u32,
    pub pool: AudioAllocPool,
    pub entries: [AudioCacheEntry; 2],
}

/// `AudioCache`.
#[derive(Debug, Clone, Copy, Default)]
pub struct AudioCache {
    pub persistent: AudioPersistentCache,
    pub temporary: AudioTemporaryCache,
}

/// `SampleCacheEntry`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SampleCacheEntry {
    pub in_use: bool,
    pub orig_medium: i8,
    pub sample_bank_id: i8,
    pub allocated_addr: u32,
    pub sample_addr: u32,
    pub size: u32,
}

/// `AudioSampleCache`.
#[derive(Debug, Clone, Copy, Default)]
pub struct AudioSampleCache {
    pub pool: AudioAllocPool,
    pub entries: [SampleCacheEntry; 32],
    pub num_entries: i32,
}

/// `AudioCachePoolSplit`, `AudioCommonPoolSplit`, `AudioSessionPoolSplit`.
#[derive(Debug, Clone, Copy, Default)]
pub struct AudioCachePoolSplit {
    pub persistent_common_pool_size: u32,
    pub temporary_common_pool_size: u32,
}
#[derive(Debug, Clone, Copy, Default)]
pub struct AudioCommonPoolSplit {
    pub seq_cache_size: u32,
    pub font_cache_size: u32,
    pub sample_bank_cache_size: u32,
}
#[derive(Debug, Clone, Copy, Default)]
pub struct AudioSessionPoolSplit {
    pub misc_pool_size: u32,
    pub cache_pool_size: u32,
}

/// `AudioPreloadReq`.
#[derive(Debug, Clone, Copy, Default)]
pub struct AudioPreloadReq {
    pub end_and_medium_key: u32,
    /// `Sample*`.
    pub sample: u32,
    pub ram_addr: u32,
    pub encoded_info: u32,
    pub is_free: bool,
}

/// Where an async load's completion message goes (`retQueue`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RetQueue {
    #[default]
    External,
    PreloadSample,
    ScriptLoad,
}

/// `AudioAsyncLoad`. Its one-message DMA queue is `dma_done`.
#[derive(Debug, Clone, Copy, Default)]
pub struct AudioAsyncLoad {
    pub status: i8,
    pub delay: i8,
    pub medium: i8,
    pub ram_addr: u32,
    pub cur_dev_addr: u32,
    pub cur_ram_addr: u32,
    pub bytes_remaining: u32,
    pub chunk_size: u32,
    pub unk_medium_param: i32,
    pub ret_msg: u32,
    pub ret_queue: RetQueue,
    pub dma_done: bool,
}

/// `AudioSlowLoad`. `sample` is the copy it keeps; `status` the `s8*` it reports to (an IO
/// port of a channel or player).
#[derive(Debug, Clone, Copy, Default)]
pub struct AudioSlowLoad {
    pub medium: u8,
    pub seq_or_font_id: u8,
    pub inst_id: u16,
    pub unk_medium_param: i32,
    pub cur_dev_addr: u32,
    pub cur_ram_addr: u32,
    pub ram_addr: u32,
    pub state: i32,
    pub bytes_remaining: i32,
    pub status: Option<IoPort>,
    /// The copied `Sample`'s `sampleAddr` (zero for a sequence's load).
    pub sample_addr: u32,
    pub dma_done: bool,
}

/// An `s8*` the loads report to: a sequence player's or a channel's IO port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoPort {
    Player(usize, usize),
    Channel(ChanId, usize),
}

/// `SampleDma`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SampleDma {
    pub ram_addr: u32,
    pub dev_addr: u32,
    pub size_unused: u16,
    pub size: u16,
    pub unused: u8,
    pub reuse_index: u8,
    pub ttl: u8,
}

/// `AudioCmd`.
#[derive(Debug, Clone, Copy, Default)]
pub struct AudioCmd {
    pub op: u8,
    pub arg0: u8,
    pub arg1: u8,
    pub arg2: u8,
    /// The data word (`asInt`, `asFloat`'s bits, ...).
    pub data: u32,
}

impl AudioCmd {
    pub fn op_args(&self) -> u32 {
        ((self.op as u32) << 24) | ((self.arg0 as u32) << 16) | ((self.arg1 as u32) << 8) | self.arg2 as u32
    }
    pub fn as_float(&self) -> f32 {
        f32::from_bits(self.data)
    }
    pub fn as_int(&self) -> i32 {
        self.data as i32
    }
    /// `asUShort`/`asSbyte`/`asUbyte`: the union's first bytes (big-endian).
    pub fn as_ushort(&self) -> u16 {
        (self.data >> 16) as u16
    }
    pub fn as_sbyte(&self) -> i8 {
        (self.data >> 24) as u8 as i8
    }
    pub fn as_ubyte(&self) -> u8 {
        (self.data >> 24) as u8
    }
}

/// What an `AudioListItem` holds: a note, a layer, or nothing (a list's head, whose `count` is
/// the list's length).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Note(NoteId),
    Layer(LayerId),
    Head,
}

/// `AudioListItem`.
#[derive(Debug, Clone, Copy)]
pub struct ListNode {
    /// `None`: detached (`prev == NULL`).
    pub prev: Option<NodeId>,
    pub next: NodeId,
    pub count: i32,
    pub pool: Option<PoolId>,
    pub value: Item,
}

/// `NotePool`: its four lists' heads.
#[derive(Debug, Clone, Copy, Default)]
pub struct NotePool {
    pub disabled: NodeId,
    pub decaying: NodeId,
    pub releasing: NodeId,
    pub active: NodeId,
}

/// The RAM addresses of what the C has as static data (`code`'s data and BSS): the tables the
/// microcode and the scripts read from memory.
#[derive(Debug, Clone, Default)]
pub struct Statics {
    /// `gWaveSamples[0..8]`; `[8]` is `wave8` (moves every frame).
    pub wave_samples: [u32; 8],
    pub default_envelope: u32,
    pub default_short_note_velocity_table: u32,
    pub default_short_note_gate_time_table: u32,
    pub d_8012fba8: u32,
    pub sequence_font_table: u32,
    /// Each reverb's `tunedSample`, `sample` and `loop`.
    pub reverb_tuned_samples: [u32; 4],
    /// `AudioThread_Update`'s address: where `gWaveSamples[8]` starts each frame.
    pub noise_base: u32,
}

/// The audio thread's output: what one call of `AudioThread_UpdateImpl` produced.
#[derive(Debug, Clone, Default)]
pub struct AudioFrame {
    /// Interleaved stereo samples (L, R), `ai_buf_lengths[index]` frames of them.
    pub samples: Vec<i16>,
}

/// `gAudioCtx`, and the statics of the audio files that belong to it.
pub struct AudioContext {
    pub ram: Ram,
    pub cart: Cart,
    pub rsp: Rsp,
    pub tables: Arc<AudioTables>,
    pub statics: Statics,
    /// `_AudioseqSegmentRomStart`, `_AudiobankSegmentRomStart`, `_AudiotableSegmentRomStart`.
    pub segment_rom_starts: [u32; 3],

    pub num_synthesis_reverbs: i8,
    pub unk_2: u16,
    pub unk_4: u16,
    pub cur_loaded_book: u32,
    pub note_subs_eu: Vec<NoteSampleState>,
    pub synthesis_reverbs: [SynthesisReverb; 4],
    pub used_samples: Vec<u32>,
    pub preload_sample_stack: Vec<AudioPreloadReq>,
    pub num_used_samples: i32,
    pub preload_sample_stack_top: i32,
    pub async_loads: [AudioAsyncLoad; 16],
    pub cur_unk_medium_load: Option<usize>,
    pub slow_load_pos: u32,
    pub slow_loads: [AudioSlowLoad; 2],
    pub external_load_queue: VecDeque<u32>,
    pub preload_sample_queue: VecDeque<u32>,
    pub script_load_queue: VecDeque<u32>,
    pub script_load_done_pointers: [Option<IoPort>; 16],
    pub script_load_index: u32,
    pub sample_dmas: Vec<SampleDma>,
    pub sample_dma_count: u32,
    pub sample_dma_list_size1: u32,
    pub sample_dma_reuse_queue1: [u8; 0x100],
    pub sample_dma_reuse_queue2: [u8; 0x100],
    pub sample_dma_reuse_queue1_rd_pos: u8,
    pub sample_dma_reuse_queue2_rd_pos: u8,
    pub sample_dma_reuse_queue1_wr_pos: u8,
    pub sample_dma_reuse_queue2_wr_pos: u8,
    pub sequence_table: AudioTable,
    pub sound_font_table: AudioTable,
    pub sample_bank_table: AudioTable,
    pub num_sequences: u16,
    pub sound_font_list: Vec<SoundFont>,
    pub audio_buffer_parameters: AudioBufferParameters,
    pub unk_2870: f32,
    pub sample_dma_buf_size1: i32,
    pub sample_dma_buf_size2: i32,
    pub sample_dma_buf_size: i32,
    pub max_audio_cmds: i32,
    pub num_notes: i32,
    pub tempo_internal_to_external: i16,
    pub sound_mode: i8,
    pub total_task_count: i32,
    pub cur_audio_frame_dma_count: i32,
    pub rsp_task_index: i32,
    pub cur_ai_buf_index: i32,
    pub abi_cmd_bufs: [u32; 2],
    /// The command list being built (`curAbiCmdBuf`).
    pub cmds: Vec<Acmd>,
    pub max_tempo_tv_type_factors: f32,
    pub refresh_rate: i32,
    pub ai_buffers: [u32; 3],
    pub ai_buf_lengths: [i16; 3],
    pub audio_random: u32,
    pub audio_error_flags: i32,
    pub reset_timer: u32,
    pub session_pool: AudioAllocPool,
    pub external_pool: AudioAllocPool,
    pub init_pool: AudioAllocPool,
    pub misc_pool: AudioAllocPool,
    pub cache_pool: AudioAllocPool,
    pub persistent_common_pool: AudioAllocPool,
    pub temporary_common_pool: AudioAllocPool,
    pub seq_cache: AudioCache,
    pub font_cache: AudioCache,
    pub sample_bank_cache: AudioCache,
    pub permanent_pool: AudioAllocPool,
    pub permanent_cache: [AudioCacheEntry; 32],
    pub persistent_sample_cache: AudioSampleCache,
    pub temporary_sample_cache: AudioSampleCache,
    pub session_pool_split: AudioSessionPoolSplit,
    pub cache_pool_split: AudioCachePoolSplit,
    pub persistent_common_pool_split: AudioCommonPoolSplit,
    pub temporary_common_pool_split: AudioCommonPoolSplit,
    pub sample_font_load_status: [u8; 0x30],
    pub font_load_status: [u8; 0x30],
    pub seq_load_status: [u8; 0x80],
    pub reset_status: u8,
    pub audio_reset_spec_id_to_load: u8,
    pub audio_reset_fade_out_frames_left: i32,
    /// `adsrDecayTable`'s address on the heap, and its values (only the CPU reads it).
    pub adsr_decay_table_addr: u32,
    pub adsr_decay_table: Vec<f32>,
    pub audio_heap: u32,
    pub audio_heap_size: u32,
    pub notes_addr: u32,
    pub notes: Vec<Note>,
    pub seq_players: [SequencePlayer; 4],
    pub sequence_layers: Vec<SequenceLayer>,
    pub channels: Vec<SequenceChannel>,
    pub note_sub_eu_offset: i32,
    pub layer_free_list: NodeId,
    pub note_free_lists: PoolId,
    pub lists: Vec<ListNode>,
    pub pools: Vec<NotePool>,
    pub cmd_wr_pos: u8,
    pub cmd_rd_pos: u8,
    pub cmd_queue_finished: u8,
    pub thread_cmd_channel_mask: [u16; 4],
    pub cmd_buf: [AudioCmd; 0x100],
    /// `cmdProcQueue` (4 messages), `audioResetQueue` (1), `taskStartQueue` (1).
    pub cmd_proc_queue: VecDeque<u32>,
    pub audio_reset_queue: VecDeque<u32>,
    pub task_start_queue: VecDeque<u32>,

    // Statics of the audio files.
    /// `gWaveSamples[8]`.
    pub wave8: u32,
    /// `gAudioContextInitialized`.
    pub audio_context_initialized: bool,
    /// `AudioThread_UpdateImpl`'s `sMaxAbiCmdCnt` and `sWaitingAudioTask` (whether one waits).
    pub max_abi_cmd_cnt: i32,
    /// `AudioThread_ProcessCmds`' `sCurCmdRdPos`.
    pub cur_cmd_rd_pos: u8,
    /// `AudioThread_ScheduleProcessCmds`' `D_801304E8`.
    pub d_801304e8: i32,
    /// `AudioThread_NextRandom`'s `sAudioRandom`.
    pub aud_rand: u32,
    /// `Audio_GetVibratoFreqScale`'s `D_80130510` and `D_80130514`.
    pub d_80130510: f32,
    pub d_80130514: i32,
    /// `osGetCount()`: the CPU's cycle counter, which the random numbers mix in. This port
    /// advances it by a fixed amount per audio frame, so runs repeat (ADR 0025).
    pub os_count: u32,
    /// `gAudioCustomUpdateFunction`: never set by this port.
    pub osc_frames: u64,
    /// Not in the C: counts for tests and tools.
    pub stats: Stats,
}

/// Not in the C: what the library did, counted, for tests and tools.
#[derive(Debug, Clone, Copy, Default)]
pub struct Stats {
    /// Notes a layer struck (`AudioSeq_SeqLayerProcessScriptStep4` found their sample).
    pub notes_struck: u64,
    /// Ticks each player's script ran (its tempo's overflows).
    pub seq_ticks: [u64; 4],
}
