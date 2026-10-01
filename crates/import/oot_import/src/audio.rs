//! The audio data for the pack (`eng_audio::AudioData`, docs/adr/0024-audio-data.md):
//! - the ROM's `Audiobank`, `Audioseq` and `Audiotable` files as they are, with their ROM
//!   addresses (the fonts, sequences and sample banks; nothing decoded);
//! - `gSoundFontTable`, `gSequenceFontTable`, `gSequenceTable` and `gSampleBankTable` from the
//!   ROM, where the decomp's `data/audio_tables.rodata.s` says they are;
//! - the tables of `src/code/audio_data.c` and `audio_init_params.c` (the audio specs and their
//!   reverbs), `gAudioHeap`'s size (`src/buffers/heaps.c`), read from the C;
//! - the microcode's resampler filters, from the ROM's `aspMainData` (`data/rsp.rodata.s`);
//! - the code `gWaveSamples[8]` reads as noise (`func_800E4FE0` on), from the ROM's `code`.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use eng_audio::data::{AudioData, AudioSpec, AudioTables, HeapSizes, ReverbSettings, RomFile};

use crate::csrc::{Init, find_initializer, strip_comments};
use crate::project::Project;

/// Where the resampler's 64 × 4 filters are in the microcode's data: `aspMainData + 0xE0`,
/// 0x200 bytes (found by matching mupen64plus-rsp-hle's `RESAMPLE_LUT` against this ROM's
/// `aspMainData`; docs/adr/0025-audio-mixer.md).
pub const RESAMPLE_LUT_OFFSET: usize = 0xE0;
/// `code`'s VRAM start in gc-eu-mq-dbg (as `crate::room`'s `CODE_VRAM`).
const CODE_VRAM: u32 = 0x8001_CE60;
/// `func_800E4FE0`, where `gWaveSamples[8]` starts (`code_800E4FE0.c`'s `func_800E5000`).
const FUNC_800E4FE0: u32 = 0x800E_4FE0;
/// How much of `code` the noise reads: the random offset (up to 0xFFF0) plus what a frame's
/// updates load from there.
const NOISE_LEN: usize = 0x11000;

/// `glabel <label>` followed by `.incbin "baserom.z64", <offset>, <size>` in a decomp `.s` file.
pub fn incbin(decomp: &Path, rel: &str, label: &str) -> Result<(usize, usize)> {
    let p = decomp.join(rel);
    let text = std::fs::read_to_string(&p).with_context(|| p.display().to_string())?;
    let mut lines = text.lines();
    while let Some(l) = lines.next() {
        if l.trim() == format!("glabel {label}") {
            let inc = lines.next().unwrap_or("").trim();
            let args = inc.strip_prefix(".incbin").with_context(|| format!("{rel}: {label} isn't followed by .incbin"))?;
            let parts: Vec<&str> = args.split(',').map(|s| s.trim()).collect();
            if parts.len() != 3 {
                bail!("{rel}: {label}: unexpected .incbin {args}");
            }
            let num = |s: &str| crate::csrc::parse_int(s).with_context(|| format!("{rel}: {label}: {s}"));
            return Ok((num(parts[1])? as usize, num(parts[2])? as usize));
        }
    }
    bail!("{rel}: no glabel {label}")
}

fn read_c(decomp: &Path, rel: &str) -> Result<String> {
    let p = decomp.join(rel);
    Ok(strip_comments(&std::fs::read_to_string(&p).with_context(|| p.display().to_string())?))
}

/// `#define NAME value` (the value as an integer).
fn define_int(src: &str, name: &str) -> Result<i64> {
    for l in src.lines() {
        let mut it = l.split_whitespace();
        if it.next() == Some("#define") && it.next() == Some(name) {
            let v: String = it.collect::<Vec<_>>().join(" ");
            return crate::csrc::parse_int(&v).with_context(|| format!("#define {name} {v}"));
        }
    }
    bail!("no #define {name}")
}

/// An array's atoms as integers, with `consts` substituted (`ADSR_HANG`, `SAMPLE_SIZE`).
fn ints(src: &str, name: &str, consts: &HashMap<&str, i64>) -> Result<Vec<i64>> {
    find_initializer(src, name)?
        .flatten()
        .iter()
        .map(|a| {
            let mut e = a.clone();
            for (k, v) in consts {
                e = e.replace(k, &v.to_string());
            }
            Init::Atom(e.clone()).as_int().or_else(|| crate::csrc::eval_expr(&e).map(|f| f as i64)).with_context(|| format!("{name}: {a}"))
        })
        .collect()
}

fn floats(src: &str, name: &str) -> Result<Vec<f32>> {
    find_initializer(src, name)?.flatten().iter().map(|a| Init::Atom(a.clone()).as_f32().with_context(|| format!("{name}: {a}"))).collect()
}

/// Loads the audio data: `AudioData::load(p)` with this trait in scope.
pub trait LoadAudioData: Sized {
    fn load(p: &Project) -> Result<Self>;
}

impl LoadAudioData for AudioData {
    fn load(p: &Project) -> Result<AudioData> {
        let decomp = &p.config.decomp;
        let rom = &p.rom;

        let file = |name: &str| -> Result<RomFile> {
            let i = rom.index_of(name).with_context(|| format!("no {name} in the ROM"))?;
            Ok(RomFile { name: name.to_string(), rom_start: rom.files[i].rom_start, bytes: rom.file(i)?.to_vec() })
        };
        let rom_bytes = |rel: &str, label: &str| -> Result<Vec<u8>> {
            let (o, n) = incbin(decomp, rel, label)?;
            Ok(rom.data.get(o..o + n).with_context(|| format!("{label} (0x{o:X}+0x{n:X}) is outside the ROM"))?.to_vec())
        };

        let tables_s = "data/audio_tables.rodata.s";
        let mut t = AudioTables {
            sound_font_table: rom_bytes(tables_s, "gSoundFontTable")?,
            sequence_font_table: rom_bytes(tables_s, "gSequenceFontTable")?,
            sequence_table: rom_bytes(tables_s, "gSequenceTable")?,
            sample_bank_table: rom_bytes(tables_s, "gSampleBankTable")?,
            ..Default::default()
        };

        // audio_data.c
        let data_c = read_c(decomp, "src/code/audio_data.c")?;
        let header = read_c(decomp, "include/z64audio.h")?;
        let mut consts: HashMap<&str, i64> = HashMap::new();
        for name in ["ADSR_DISABLE", "ADSR_HANG", "ADSR_GOTO", "ADSR_RESTART"] {
            consts.insert(name, define_int(&header, name)?);
        }
        // SAMPLE_SIZE is sizeof(s16).
        consts.insert("SAMPLE_SIZE", 2);
        let wave_names = find_initializer(&data_c, "gWaveSamples")?.flatten();
        let mut distinct: Vec<String> = Vec::new();
        for w in &wave_names {
            if !distinct.contains(w) {
                distinct.push(w.clone());
            }
        }
        for w in &distinct {
            let v = ints(&data_c, w, &consts)?;
            if v.len() != 4 * 64 {
                bail!("{w}: {} samples, not 4 harmonics of 64", v.len());
            }
            t.wave_samples.push(v.into_iter().map(|x| x as i16).collect());
        }
        t.wave_sample_index = wave_names.iter().map(|w| distinct.iter().position(|d| d == w).unwrap() as u8).collect();
        t.bend_pitch_one_octave_frequencies = floats(&data_c, "gBendPitchOneOctaveFrequencies")?;
        t.bend_pitch_two_semitones_frequencies = floats(&data_c, "gBendPitchTwoSemitonesFrequencies")?;
        t.pitch_frequencies = floats(&data_c, "gPitchFrequencies")?;
        t.default_short_note_velocity_table = ints(&data_c, "gDefaultShortNoteVelocityTable", &consts)?.into_iter().map(|v| v as u8).collect();
        t.default_short_note_gate_time_table = ints(&data_c, "gDefaultShortNoteGateTimeTable", &consts)?.into_iter().map(|v| v as u8).collect();
        let env = ints(&data_c, "gDefaultEnvelope", &consts)?;
        t.default_envelope = env.chunks(2).map(|c| (c[0] as i16, c[1] as i16)).collect();
        t.haas_effect_delay_sizes = ints(&data_c, "gHaasEffectDelaySizes", &consts)?.into_iter().map(|v| v as u16).collect();
        t.d_8012fba8 = ints(&data_c, "D_8012FBA8", &consts)?.into_iter().map(|v| v as i16).collect();
        t.headset_pan_volume = floats(&data_c, "gHeadsetPanVolume")?;
        t.stereo_pan_volume = floats(&data_c, "gStereoPanVolume")?;
        t.default_pan_volume = floats(&data_c, "gDefaultPanVolume")?;
        t.low_pass_filter_data = ints(&data_c, "gLowPassFilterData", &consts)?.into_iter().map(|v| v as i16).collect();
        t.high_pass_filter_data = ints(&data_c, "gHighPassFilterData", &consts)?.into_iter().map(|v| v as i16).collect();
        for (name, n, want) in [
            ("gPitchFrequencies", t.pitch_frequencies.len(), 128),
            ("gBendPitchOneOctaveFrequencies", t.bend_pitch_one_octave_frequencies.len(), 256),
            ("gBendPitchTwoSemitonesFrequencies", t.bend_pitch_two_semitones_frequencies.len(), 256),
            ("gHeadsetPanVolume", t.headset_pan_volume.len(), 128),
            ("gStereoPanVolume", t.stereo_pan_volume.len(), 128),
            ("gDefaultPanVolume", t.default_pan_volume.len(), 128),
            ("gHaasEffectDelaySizes", t.haas_effect_delay_sizes.len(), 64),
            ("gLowPassFilterData", t.low_pass_filter_data.len(), 16 * 8),
            ("gHighPassFilterData", t.high_pass_filter_data.len(), 15 * 8),
        ] {
            if n != want {
                bail!("{name}: {n} entries, not {want}");
            }
        }

        // audio_init_params.c
        let params = read_c(decomp, "src/code/audio_init_params.c")?;
        let default_reverb = {
            // `#define DEFAULT_REVERB_SETTINGS \` and the next line.
            let raw = std::fs::read_to_string(decomp.join("src/code/audio_init_params.c"))?;
            let at = raw.find("#define DEFAULT_REVERB_SETTINGS").context("no DEFAULT_REVERB_SETTINGS")?;
            let rest = &raw[at..];
            let open = rest.find('{').context("DEFAULT_REVERB_SETTINGS")?;
            let close = rest.find('}').context("DEFAULT_REVERB_SETTINGS")?;
            rest[open..=close].to_string()
        };
        let params = params.replace("DEFAULT_REVERB_SETTINGS", &default_reverb);
        let reverbs = find_initializer(&params, "sReverbSettings")?;
        let num = |i: &Init| -> Result<i64> { i.as_int().with_context(|| format!("{i:?}")) };
        let reverb_rows: Vec<Vec<ReverbSettings>> = reverbs
            .list()
            .iter()
            .map(|row| {
                row.list()
                    .iter()
                    .map(|r| {
                        let f: Vec<i64> = r.list().iter().map(num).collect::<Result<_>>()?;
                        if f.len() != 12 {
                            bail!("a ReverbSettings with {} fields", f.len());
                        }
                        Ok(ReverbSettings {
                            downsample_rate: f[0] as u8,
                            window_size: f[1] as u16,
                            decay_ratio: f[2] as u16,
                            unk_6: f[3] as u16,
                            unk_8: f[4] as u16,
                            volume: f[5] as u16,
                            leak_rtl: f[6] as u16,
                            leak_ltr: f[7] as u16,
                            unk_10: f[8] as u8 as i8,
                            unk_12: f[9] as u16,
                            low_pass_filter_cutoff_left: f[10] as i16,
                            low_pass_filter_cutoff_right: f[11] as i16,
                        })
                    })
                    .collect()
            })
            .collect::<Result<_>>()?;
        let specs = find_initializer(&params, "gAudioSpecs")?;
        for s in specs.list() {
            let f = s.list();
            if f.len() != 19 {
                bail!("an AudioSpec with {} fields", f.len());
            }
            let row = f[7]
                .atom()
                .and_then(|a| a.strip_prefix("sReverbSettings["))
                .and_then(|a| a.strip_suffix(']'))
                .and_then(crate::csrc::parse_int)
                .with_context(|| format!("AudioSpec reverbs: {:?}", f[7]))?;
            let n = |i: usize| num(&f[i]);
            let num_reverbs = n(6)? as u8;
            let mut rs = reverb_rows.get(row as usize).cloned().with_context(|| format!("sReverbSettings[{row}]"))?;
            // A row lists its reverbs; the C reads `numReverbs` of them (rows are 3 long).
            rs.resize(3, ReverbSettings::default());
            t.specs.push(AudioSpec {
                sampling_frequency: n(0)? as u32,
                unk_04: n(1)? as u8,
                num_notes: n(2)? as u8,
                num_sequence_players: n(3)? as u8,
                unk_07: n(4)? as u8,
                unk_08: n(5)? as u8,
                num_reverbs,
                reverb_settings: rs,
                sample_dma_buf_size1: n(8)? as u16,
                sample_dma_buf_size2: n(9)? as u16,
                unk_14: n(10)? as u16,
                persistent_seq_cache_size: n(11)? as u32,
                persistent_font_cache_size: n(12)? as u32,
                persistent_sample_bank_cache_size: n(13)? as u32,
                temporary_seq_cache_size: n(14)? as u32,
                temporary_font_cache_size: n(15)? as u32,
                temporary_sample_bank_cache_size: n(16)? as u32,
                persistent_sample_cache_size: n(17)? as i32,
                temporary_sample_cache_size: n(18)? as i32,
            });
        }
        let heaps = read_c(decomp, "src/buffers/heaps.c")?;
        let heap_size =
            heaps.lines().find_map(|l| l.trim().strip_prefix("u8 gAudioHeap[").and_then(|r| r.split(']').next()).and_then(crate::csrc::parse_int)).context("src/buffers/heaps.c: no gAudioHeap")?;
        t.heap_sizes = HeapSizes {
            audio_heap: heap_size as u32,
            num_soundfonts: define_int(&params, "NUM_SOUNDFONTS")? as u32,
            sfx_seq_size: define_int(&params, "SFX_SEQ_SIZE")? as u32,
            sfx_soundfont_1_size: define_int(&params, "SFX_SOUNDFONT_1_SIZE")? as u32,
            sfx_soundfont_2_size: define_int(&params, "SFX_SOUNDFONT_2_SIZE")? as u32,
        };
        // D_8014A6C0[1] is gTatumsPerBeat.
        let tatums = ints(&params, "D_8014A6C0", &consts)?;
        t.tatums_per_beat = *tatums.get(1).context("D_8014A6C0[1]")? as i16;

        // The microcode's resampler filters.
        let asp = rom_bytes("data/rsp.rodata.s", "aspMainDataStart")?;
        let lut = asp.get(RESAMPLE_LUT_OFFSET..RESAMPLE_LUT_OFFSET + 0x200).context("aspMainData is too short for the resampler's filters")?;
        t.resample_lut = lut.chunks(2).map(|c| i16::from_be_bytes([c[0], c[1]])).collect();
        // The 64 phases interpolate: each row's taps sum to about 1.0 (0x8000).
        for (i, r) in t.resample_lut.chunks(4).enumerate() {
            let sum: i32 = r.iter().map(|&v| v as i32).sum();
            if !(0x7F00..=0x8100).contains(&sum) {
                bail!("aspMainData + 0x{RESAMPLE_LUT_OFFSET:X}: row {i}'s taps sum to 0x{sum:X}, not about 0x8000 (not the resampler's table)");
            }
        }

        // The noise: code from func_800E4FE0 on.
        let code = rom.file_by_name("code")?;
        let at = (FUNC_800E4FE0 - CODE_VRAM) as usize;
        t.noise_code = code.get(at..(at + NOISE_LEN).min(code.len())).context("code is too short for func_800E4FE0")?.to_vec();
        t.noise_code_vram = FUNC_800E4FE0;

        Ok(AudioData { tables: t, audiobank: file("Audiobank")?, audioseq: file("Audioseq")?, audiotable: file("Audiotable")? })
    }
}
