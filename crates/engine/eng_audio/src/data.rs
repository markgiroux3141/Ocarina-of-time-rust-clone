//! What the audio library needs from the game, as the importer writes it into the pack: the
//! ROM's three audio files, the tables the `code` file holds, and the tables of
//! `audio/internal/data.c` and `session_config.c` (read from the decomp's C).
//!
//! Nothing here is parsed ahead of time: the fonts, sequences and samples stay the ROM's bytes,
//! and the library reads them as the game does (`AudioLoad_RelocateFont`, the sequence
//! player, the microcode's ADPCM decoder; docs/adr/0024-audio-data.md).

use serde::{Deserialize, Serialize};

/// A ROM file and where it is in the ROM (`_AudiobankSegmentRomStart` and the others): the
/// audio tables' addresses are relative to it until `AudioLoad_InitTable` adds it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RomFile {
    pub name: String,
    pub rom_start: u32,
    pub bytes: Vec<u8>,
}

/// `ReverbSettings` (`audio.h`), one row of `sReverbSettings`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ReverbSettings {
    pub downsample_rate: u8,
    pub window_size: u16,
    pub decay_ratio: u16,
    pub unk_6: u16,
    pub unk_8: u16,
    pub volume: u16,
    pub leak_rtl: u16,
    pub leak_ltr: u16,
    pub unk_10: i8,
    pub unk_12: u16,
    pub low_pass_filter_cutoff_left: i16,
    pub low_pass_filter_cutoff_right: i16,
}

/// `AudioSpec` (`audio.h`), one row of `gAudioSpecs`, with its reverbs' settings.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AudioSpec {
    pub sampling_frequency: u32,
    pub unk_04: u8,
    pub num_notes: u8,
    pub num_sequence_players: u8,
    pub unk_07: u8,
    pub unk_08: u8,
    pub num_reverbs: u8,
    /// `reverbSettings`: the row of `sReverbSettings` it points at (`num_reverbs` used).
    pub reverb_settings: Vec<ReverbSettings>,
    pub sample_dma_buf_size1: u16,
    pub sample_dma_buf_size2: u16,
    pub unk_14: u16,
    pub persistent_seq_cache_size: u32,
    pub persistent_font_cache_size: u32,
    pub persistent_sample_bank_cache_size: u32,
    pub temporary_seq_cache_size: u32,
    pub temporary_font_cache_size: u32,
    pub temporary_sample_bank_cache_size: u32,
    pub persistent_sample_cache_size: i32,
    pub temporary_sample_cache_size: i32,
}

/// The sizes `gAudioHeapInitSizes` is computed from (`session_config.c`, `heaps.c`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct HeapSizes {
    /// `sizeof(gAudioHeap)` (`src/buffers/heaps.c`).
    pub audio_heap: u32,
    /// `NUM_SOUNDFONTS`.
    pub num_soundfonts: u32,
    /// `SFX_SEQ_SIZE`, `Soundfont_0_SIZE`, `Soundfont_1_SIZE`.
    pub sfx_seq_size: u32,
    pub sfx_soundfont_1_size: u32,
    pub sfx_soundfont_2_size: u32,
}

/// Everything but the ROM files (`keys::AUDIO_TABLES`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AudioTables {
    /// `gSequenceTable`, `gSoundFontTable`, `gSampleBankTable` (`AudioTable`s) and
    /// `gSequenceFontTable`, as the ROM holds them (at `baseroms/gc-eu-mq-dbg/config.yml`'s `gSequenceFontTable`).
    pub sequence_table: Vec<u8>,
    pub sound_font_table: Vec<u8>,
    pub sample_bank_table: Vec<u8>,
    pub sequence_font_table: Vec<u8>,

    /// `audio/internal/data.c`'s wave samples, in `gWaveSamples`' order: sawtooth, triangle, sine,
    /// square, white noise, `D_8012EA90`, eighth pulse, quarter pulse (each 4 harmonics × 64).
    /// `gWaveSamples[8]` points into code (`noise_code`).
    pub wave_samples: Vec<Vec<i16>>,
    /// `gWaveSamples`: which of `wave_samples` each of the first eight entries is.
    pub wave_sample_index: Vec<u8>,
    pub bend_pitch_one_octave_frequencies: Vec<f32>,
    pub bend_pitch_two_semitones_frequencies: Vec<f32>,
    pub pitch_frequencies: Vec<f32>,
    pub default_short_note_velocity_table: Vec<u8>,
    pub default_short_note_gate_time_table: Vec<u8>,
    /// `gDefaultEnvelope`: (delay, arg) points.
    pub default_envelope: Vec<(i16, i16)>,
    pub haas_effect_delay_sizes: Vec<u16>,
    /// `D_8012FBA8`: the book `bookOffset` 1 loads (from its second entry).
    pub d_8012fba8: Vec<i16>,
    pub headset_pan_volume: Vec<f32>,
    pub stereo_pan_volume: Vec<f32>,
    pub default_pan_volume: Vec<f32>,
    pub low_pass_filter_data: Vec<i16>,
    pub high_pass_filter_data: Vec<i16>,

    /// `gAudioSpecs`.
    pub specs: Vec<AudioSpec>,
    pub heap_sizes: HeapSizes,
    /// `gTempoData.seqTicksPerBeat`.
    pub tatums_per_beat: i16,

    /// The microcode's resampler filters: 64 phases × 4 taps (`aspMainData + 0xE0`).
    pub resample_lut: Vec<i16>,
    /// The code `gWaveSamples[8]` reads as noise: `AudioThread_Update` and the 0x10400 bytes from it
    /// (its VRAM address is `noise_code_vram`).
    pub noise_code: Vec<u8>,
    pub noise_code_vram: u32,
}

impl AudioTables {
    /// `AudioLoad_GetFontsForSequence`'s list: the fonts sequence `seq_id` uses, as
    /// `gSequenceFontTable` lists them (a big-endian offset per sequence, then a count and the
    /// font ids). Empty for a sequence outside the table.
    pub fn fonts_for_sequence(&self, seq_id: u32) -> &[u8] {
        let t = &self.sequence_font_table;
        let at = 2 * seq_id as usize;
        let Some(o) = t.get(at..at + 2).map(|b| u16::from_be_bytes([b[0], b[1]]) as usize) else { return &[] };
        let n = t.get(o).copied().unwrap_or(0) as usize;
        t.get(o + 1..o + 1 + n).unwrap_or(&[])
    }
}

/// The whole of it: the tables and the three ROM files.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AudioData {
    pub tables: AudioTables,
    pub audiobank: RomFile,
    pub audioseq: RomFile,
    pub audiotable: RomFile,
}
