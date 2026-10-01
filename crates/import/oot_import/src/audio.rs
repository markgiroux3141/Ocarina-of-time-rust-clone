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

/// The game's audio tables (`oot_game::audio::AudioGameTables`), read from the C:
/// `code_800EC960.c`'s `sSeqFlags` (with its `SEQ_FLAG_*` defines), `sSpecReverbs` and
/// `sNatureAmbienceDataIO` (its `NATURE_IO_*` macros and enums from `sequence.h`), and
/// `audio_external_data.c`'s `gSoundModeList` (`SoundMode`, `z64audio.h`).
pub fn audio_game_tables(decomp: &Path) -> Result<oot_game::audio::AudioGameTables> {
    let src = read_c(decomp, "src/code/code_800EC960.c")?;
    let seq_h = read_c(decomp, "include/sequence.h")?;
    let audio_h = read_c(decomp, "include/z64audio.h")?;
    let ext = read_c(decomp, "src/code/audio_external_data.c")?;

    // Names to values: the enums, and the object-like #defines with a number or a shift.
    let mut names: HashMap<String, i64> = HashMap::new();
    for member in ["NATURE_CHANNEL_STREAM_0", "CHANNEL_IO_PORT_0", "NATURE_STREAM_RUSHING_WATER", "NATURE_CRITTER_BIRD_CHIRP_1", "NATURE_ID_GENERAL_NIGHT"] {
        let e = crate::csrc::parse_enum(&seq_h, member);
        if e.is_empty() {
            bail!("include/sequence.h: no enum with {member}");
        }
        names.extend(e.into_iter().map(|(v, n)| (n, v)));
    }
    let modes = crate::csrc::parse_enum(&audio_h, "SOUNDMODE_STEREO");
    if modes.is_empty() {
        bail!("include/z64audio.h: no SoundMode enum");
    }
    names.extend(modes.into_iter().map(|(v, n)| (n, v)));
    for text in [&src, &seq_h] {
        for l in text.lines() {
            let mut it = l.split_whitespace();
            if it.next() != Some("#define") {
                continue;
            }
            let Some(name) = it.next() else { continue };
            if name.contains('(') {
                continue;
            }
            let body: String = it.collect::<Vec<_>>().join(" ");
            if let Some(v) = shift_or_int(&body) {
                names.insert(name.to_string(), v);
            }
        }
    }
    let value = |a: &str| -> Result<i64> {
        a.split('|')
            .map(|t| {
                let t = t.trim();
                crate::csrc::parse_int(t).or_else(|| names.get(t).copied()).or_else(|| shift_or_int(t)).with_context(|| format!("can't evaluate {t}"))
            })
            .try_fold(0i64, |acc, v| v.map(|v| acc | v))
    };

    // The function-like NATURE_IO_* macros: `#define NAME(arg) a, b, arg`.
    let mut macros: HashMap<String, (String, Vec<String>)> = HashMap::new();
    for l in seq_h.lines() {
        let Some(rest) = l.trim().strip_prefix("#define ") else { continue };
        let Some(open) = rest.find('(') else { continue };
        let name = &rest[..open];
        if name.contains(char::is_whitespace) || !name.starts_with("NATURE_IO_") {
            continue;
        }
        let Some(close) = rest.find(')') else { continue };
        let param = rest[open + 1..close].trim().to_string();
        let body = rest[close + 1..].split(',').map(|s| s.trim().to_string()).collect();
        macros.insert(name.to_string(), (param, body));
    }
    let expand = |atom: &str| -> Result<Vec<u8>> {
        if let Some(open) = atom.find('(') {
            let name = atom[..open].trim();
            let arg = atom[open + 1..].strip_suffix(')').with_context(|| format!("{atom}"))?.trim();
            let (param, body) = macros.get(name).with_context(|| format!("include/sequence.h: no macro {name}"))?;
            body.iter().map(|b| value(if b == param { arg } else { b }).map(|v| v as u8)).collect()
        } else {
            Ok(vec![value(atom)? as u8])
        }
    };

    let seq_flags: Vec<u8> = find_initializer(&src, "sSeqFlags")?.flatten().iter().map(|a| value(a).map(|v| v as u8)).collect::<Result<_>>()?;
    let spec_reverbs: Vec<i8> = find_initializer(&src, "sSpecReverbs")?.flatten().iter().map(|a| value(a).map(|v| v as i8)).collect::<Result<_>>()?;
    let mut nature_ambience = Vec::new();
    for e in find_initializer(&src, "sNatureAmbienceDataIO")?.list() {
        let f = e.list();
        if f.len() != 3 {
            bail!("sNatureAmbienceDataIO: an entry with {} fields", f.len());
        }
        let num = |i: &Init| -> Result<u16> { value(i.atom().with_context(|| format!("{i:?}"))?).map(|v| v as u16) };
        let mut channel_io = Vec::new();
        for a in f[2].flatten() {
            channel_io.extend(expand(&a)?);
        }
        // channelIO[3 * 33 + 1], the rest zero.
        if channel_io.len() > 100 {
            bail!("sNatureAmbienceDataIO: {} channel IO bytes, more than 100", channel_io.len());
        }
        channel_io.resize(100, 0);
        nature_ambience.push(oot_game::audio::NatureAmbienceDataIO { player_io: num(&f[0])?, channel_mask: num(&f[1])?, channel_io });
    }
    let sound_mode_list: Vec<u8> = find_initializer(&ext, "gSoundModeList")?.flatten().iter().map(|a| value(a).map(|v| v as u8)).collect::<Result<_>>()?;

    let bytes = |name: &str| -> Result<Vec<u8>> { find_initializer(&src, name)?.flatten().iter().map(|a| value(a).map(|v| v as u8)).collect() };
    let rows = |name: &str| -> Result<Vec<Vec<u8>>> {
        find_initializer(&src, name)?.list().iter().map(|r| r.flatten().iter().map(|a| value(a).map(|v| v as u8)).collect()).collect()
    };
    let float_list = |name: &str| -> Result<Vec<f32>> { floats(&src, name) };
    let ganons_tower_levels_vol = bytes("sGanonsTowerLevelsVol")?;
    let is_large_sfx_bank = bytes("gIsLargeSfxBank")?;
    let channels_per_bank = rows("gChannelsPerBank")?;
    let used_channels_per_bank = rows("gUsedChannelsPerBank")?;
    let behind_screen_z = float_list("sBehindScreenZ")?;
    let charge_freq_scales = float_list("D_801305E4")?;

    // gSfxBanks' arrays (audio_external_data.c), sized where code_800F7260.c declares them.
    let banks_c = read_c(decomp, "src/code/code_800F7260.c")?;
    let mut sfx_bank_sizes = Vec::new();
    for name in find_initializer(&ext, "gSfxBanks")?.flatten() {
        let decl = format!("SfxBankEntry {name}[");
        let at = banks_c.find(&decl).with_context(|| format!("code_800F7260.c: no {decl}"))?;
        let rest = &banks_c[at + decl.len()..];
        let n = rest.split(']').next().and_then(crate::csrc::parse_int).with_context(|| format!("{name}'s size"))?;
        sfx_bank_sizes.push(n as u8);
    }

    // gSfxParams (audio_sfx_params.c): DEFINE_SFX(enum, importance, distParam, randParam, flags)
    // as { importance, ((distParam << SFX_PARAM_01_SHIFT) & SFX_PARAM_01_MASK) |
    // ((randParam << SFX_PARAM_67_SHIFT) & SFX_PARAM_67_MASK) | flags }, the banks in its order.
    let sfx_h = read_c(decomp, "include/sfx.h")?;
    let mut sfx_names: HashMap<String, i64> = HashMap::new();
    for l in sfx_h.lines() {
        let mut it = l.split_whitespace();
        if it.next() != Some("#define") {
            continue;
        }
        let Some(name) = it.next() else { continue };
        if name.contains('(') || !name.starts_with("SFX_") {
            continue;
        }
        let body: String = it.collect::<Vec<_>>().join(" ");
        if let Some(v) = eval_int(&body, &sfx_names) {
            sfx_names.insert(name.to_string(), v);
        }
    }
    let params_c = read_c(decomp, "src/code/audio_sfx_params.c")?;
    let order = find_initializer(&params_c, "gSfxParams")?.flatten();
    // Each sXBankParams' #include.
    let mut sfx_params = Vec::new();
    for bank in &order {
        let at = params_c.find(&format!("SfxParams {bank}[]")).with_context(|| format!("audio_sfx_params.c: no {bank}"))?;
        let inc = params_c[at..].split("#include \"").nth(1).and_then(|r| r.split('"').next()).with_context(|| format!("{bank}'s table"))?;
        let table = std::fs::read_to_string(decomp.join("include").join(inc)).with_context(|| inc.to_string())?;
        let mut rows = Vec::new();
        for (m, args) in crate::csrc::define_rows_nested(&table) {
            if m != "DEFINE_SFX" {
                continue;
            }
            if args.len() != 5 {
                bail!("{inc}: DEFINE_SFX with {} arguments", args.len());
            }
            let num = |a: &str| eval_int(a, &sfx_names).with_context(|| format!("{inc}: {a}"));
            let (importance, dist, rand, flags) = (num(&args[1])?, num(&args[2])?, num(&args[3])?, num(&args[4])?);
            let m01 = sfx_names["SFX_PARAM_01_MASK"];
            let m67 = sfx_names["SFX_PARAM_67_MASK"];
            let params = ((dist << sfx_names["SFX_PARAM_01_SHIFT"]) & m01) | ((rand << sfx_names["SFX_PARAM_67_SHIFT"]) & m67) | flags;
            rows.push(oot_game::audio::SfxParams { name: args[0].clone(), importance: importance as u8, params: params as u16 });
        }
        sfx_params.push(rows);
    }

    // z_bgcheck.c's D_80119E10: NA_SE_PL_WALK_* - SFX_FLAG.
    let bgcheck = read_c(decomp, "src/code/z_bgcheck.c")?;
    let sfx_id_of = |name: &str| -> Option<i64> {
        sfx_params.iter().enumerate().find_map(|(b, bank)| bank.iter().position(|p| p.name == name).map(|i| ((b as i64) << 12) + 0x800 + i as i64))
    };
    let floor_sfx: Vec<u16> = find_initializer(&bgcheck, "D_80119E10")?
        .flatten()
        .iter()
        .map(|a| {
            let (name, minus) = match a.split_once('-') {
                Some((n, m)) => (n.trim(), m.trim()),
                None => (a.trim(), ""),
            };
            let id = sfx_id_of(name).with_context(|| format!("z_bgcheck.c: D_80119E10: {name}"))?;
            let sub = if minus.is_empty() { 0 } else { eval_int(minus, &sfx_names).with_context(|| format!("z_bgcheck.c: {minus}"))? };
            Ok((id - sub) as u16)
        })
        .collect::<Result<_>>()?;

    // z_player.c's struct_80832924 tables: { sfxId, field } rows.
    let player_c = read_c(decomp, "src/overlays/actors/ovl_player_actor/z_player.c")?;
    let mut player_anim_sfx = Vec::new();
    let sfx_value = |a: &str| -> Result<u16> {
        let (name, minus) = match a.split_once('-') {
            Some((n, m)) => (n.trim(), m.trim()),
            None => (a.trim(), ""),
        };
        let base = match crate::csrc::parse_int(name) {
            Some(v) => v,
            None => sfx_id_of(name).with_context(|| format!("z_player.c: no sound {name}"))?,
        };
        let sub = if minus.is_empty() { 0 } else { eval_int(minus, &sfx_names).with_context(|| format!("z_player.c: {minus}"))? };
        Ok((base - sub) as u16)
    };
    let pair = |r: &Init| -> Result<(u16, i16)> {
        let f = r.flatten();
        if f.len() != 2 {
            bail!("z_player.c: a struct_80832924 with {} fields", f.len());
        }
        Ok((sfx_value(&f[0])?, eval_int(&f[1], &sfx_names).with_context(|| format!("z_player.c: {}", f[1]))? as i16))
    };
    let mut from = 0;
    while let Some(rel) = player_c[from..].find("static struct_80832924 ") {
        let at = from + rel + "static struct_80832924 ".len();
        from = at;
        let name: String = player_c[at..].chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        let rest = &player_c[at + name.len()..];
        if rest.starts_with("[][2]") {
            for (i, row) in find_initializer(&player_c, &name)?.list().iter().enumerate() {
                player_anim_sfx.push((format!("{name}[{i}]"), row.list().iter().map(pair).collect::<Result<Vec<_>>>()?));
            }
        } else if rest.starts_with("[]") {
            player_anim_sfx.push((name.clone(), find_initializer(&player_c, &name)?.list().iter().map(pair).collect::<Result<Vec<_>>>()?));
        }
    }

    for (name, n, want) in [
        ("D_80119E10", floor_sfx.len(), 14),
        ("sSeqFlags", seq_flags.len(), 0x6E),
        ("sSpecReverbs", spec_reverbs.len(), 20),
        ("sNatureAmbienceDataIO", nature_ambience.len(), 20),
        ("gSoundModeList", sound_mode_list.len(), 4),
        ("sGanonsTowerLevelsVol", ganons_tower_levels_vol.len(), 8),
        ("gSfxParams", sfx_params.len(), 7),
        ("gSfxBanks", sfx_bank_sizes.len(), 7),
        ("gIsLargeSfxBank", is_large_sfx_bank.len(), 7),
        ("gChannelsPerBank", channels_per_bank.len(), 4),
        ("gUsedChannelsPerBank", used_channels_per_bank.len(), 4),
        ("sBehindScreenZ", behind_screen_z.len(), 2),
        ("D_801305E4", charge_freq_scales.len(), 4),
    ] {
        if n != want {
            bail!("{name}: {n} entries, not {want}");
        }
    }
    Ok(oot_game::audio::AudioGameTables {
        seq_flags,
        spec_reverbs,
        nature_ambience,
        sound_mode_list,
        ganons_tower_levels_vol,
        sfx_params,
        sfx_bank_sizes,
        is_large_sfx_bank,
        channels_per_bank,
        used_channels_per_bank,
        behind_screen_z,
        charge_freq_scales,
        floor_sfx,
        player_anim_sfx,
    })
}

/// An integer expression of numbers and `names`, with `|`, `<<` and parentheses (a
/// `#define`'s body, a table's flags).
fn eval_int(s: &str, names: &HashMap<String, i64>) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(rest) = s.strip_prefix('-') {
        return eval_int(rest, names).map(|v| -v);
    }
    // Strip parentheses around the whole.
    if s.starts_with('(') && s.ends_with(')') {
        let mut depth = 0;
        let whole = s.char_indices().all(|(i, c)| {
            match c {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
            depth > 0 || i == s.len() - 1
        });
        if whole {
            return eval_int(&s[1..s.len() - 1], names);
        }
    }
    // The loosest operator at the top level: `|`, then `<<`.
    for op in ["|", "<<"] {
        let mut depth = 0;
        let b = s.as_bytes();
        let mut i = b.len();
        while i > 0 {
            i -= 1;
            match b[i] {
                b')' => depth += 1,
                b'(' => depth -= 1,
                _ => {}
            }
            if depth == 0 && s[i..].starts_with(op) && !(op == "|" && s[..i].ends_with('|')) {
                let (l, r) = (eval_int(&s[..i], names)?, eval_int(&s[i + op.len()..], names)?);
                return Some(if op == "|" { l | r } else { l << r });
            }
        }
    }
    crate::csrc::parse_int(s).or_else(|| names.get(s).copied())
}

/// `(1 << n)`, `1 << n`, or a number.
fn shift_or_int(s: &str) -> Option<i64> {
    let t = s.trim().trim_start_matches('(').trim_end_matches(')').trim();
    if let Some((a, b)) = t.split_once("<<") {
        return Some(crate::csrc::parse_int(a.trim())? << crate::csrc::parse_int(b.trim())?);
    }
    crate::csrc::parse_int(t)
}
