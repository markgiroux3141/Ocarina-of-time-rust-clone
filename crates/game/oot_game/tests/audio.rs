//! The audio library (`eng_audio`) on the pack's audio data, against the C and against the
//! samples decoded on their own (`eng_audio::font`, the reference decoder):
//! - the microcode's ADPCM decoder (`aADPCMdec`) gives every sample of every font bit for bit
//!   what the reference decoder gives, and after a loop's restart from its stored predictor
//!   state (`aSetLoop`), what follows that state;
//! - a note of a Kokiri Forest instrument (font 15, instrument 4, tuning 1.0) played by a
//!   sequence: its volume each update is the C's ADSR (`Audio_AdsrUpdate` on the instrument's
//!   envelope, `updatesPerFrameScaled` 0.75) times `gDefaultPanVolume` at pan 64, as
//!   `Audio_InitNoteSub` makes it; at C4 (`gPitchFrequencies[39]` = 1.0) the output follows the
//!   sample one for one, at C3 (0.5) at half speed; it decays with `adsrDecayTable[239]` once
//!   the layer ends;
//! - a drum plays its sample at its tuning;
//! - with reverb, the note rings on after it ends: reverb 0 of spec 0 (`sReverbSettings[0]`'s
//!   first, `DEFAULT_REVERB_SETTINGS`) brings its ring buffer back every 0x30 * 64 samples,
//!   mixed into the output at `volume` 0x7FFF, then scaled by `decayRatio` 0x3000 / 0x8000
//!   (`AudioSynth_DoOneAudioUpdate`), so each echo is 0.375 of the one before;
//! - Kokiri Forest's sequence (60) strikes as many notes before its loop (5951 ticks) as the
//!   extractor's interpreter reads (597), and sounds.

use eng_audio::adpcm::CODEC_SMALL_ADPCM;
use eng_audio::context::*;
use eng_audio::font::{FontReader, SampleInfo};
use eng_audio::ram::Ram;
use eng_audio::renderer::{Renderer, note_script, note_script_reverb};
use eng_audio::rsp::abi::*;
use eng_audio::rsp::{A_CONTINUE, A_INIT, A_LOOP, Rsp};
use eng_audio::{AudioContext, AudioData};

fn data() -> Option<AudioData> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(pack.audio_data().expect("the pack's audio data"))
}

/// Decodes `n` samples starting at frame `first_frame` with the microcode, 8 frames per
/// command, as `AudioSynth_ProcessNote` lays DMEM out: the compressed frames below
/// `DMEM_COMPRESSED_ADPCM_DATA`, the output from `DMEM_UNCOMPRESSED_NOTE`, its first 16 samples
/// the frame before (the state, or the loop's predictor state with `A_LOOP`). Returns the
/// samples after those 16.
fn microcode_decode(rsp: &mut Rsp, ram: &mut Ram, s: &SampleInfo, data: &[u8], first_frame: usize, n: usize, first_flags: u32, loop_state: Option<[i16; 16]>) -> Vec<i16> {
    const BOOK: u32 = 0x8010_0000;
    const STATE: u32 = 0x8010_1000;
    const LOOP: u32 = 0x8010_1100;
    const DATA: u32 = 0x8011_0000;
    let book = s.book.as_ref().unwrap();
    for (i, c) in book.coefs.iter().enumerate() {
        ram.set_s16(BOOK + 2 * i as u32, *c);
    }
    if let Some(st) = loop_state {
        for (i, v) in st.iter().enumerate() {
            ram.set_s16(LOOP + 0x10 + 2 * i as u32, *v);
        }
    }
    let fsize = if s.codec == CODEC_SMALL_ADPCM { 5 } else { 9 };
    let small = if s.codec == CODEC_SMALL_ADPCM { 4 } else { 0 };
    ram.write(DATA, data);
    rsp.command(a_load_adpcm((16 * book.order * book.npred) as u32, BOOK), ram);
    if loop_state.is_some() {
        rsp.command(a_set_loop(LOOP + 0x10), ram);
    }
    let mut out = Vec::new();
    let mut frame = first_frame;
    let mut flags = first_flags;
    while out.len() < n {
        let frames = ((n - out.len()).div_ceil(16)).min(8);
        let src = DATA + (frame * fsize) as u32;
        let pad = src & 0xF;
        let aligned = ((frames * fsize + 16 + 0xF) & !0xF) as u32;
        let addr = 0x940 - aligned;
        rsp.command(a_load_buffer(src - pad, addr, aligned), ram);
        rsp.command(a_set_buffer(0, addr + pad, 0x580, (frames * 32) as u32), ram);
        rsp.command(a_adpcm_dec(flags | small, STATE), ram);
        for i in 0..frames * 16 {
            out.push(rsp.s16(0x580 + 32 + 2 * i as u32));
        }
        frame += frames;
        flags = A_CONTINUE;
    }
    out.truncate(n);
    out
}

#[test]
fn the_microcode_decodes_every_sample_as_the_reference_decoder() {
    let Some(data) = data() else { return };
    let reader = FontReader::new(&data);
    let mut rsp = Rsp::new(&data.tables.resample_lut);
    let mut ram = Ram::new();
    let mut seen = std::collections::HashSet::new();
    let (mut samples, mut looped, mut loop_exact) = (0, 0, 0);
    let mut mismatched_states = Vec::new();
    let mut past_book = Vec::new();
    for font_id in 0..reader.fonts.entries.len() {
        if reader.fonts.entries[font_id].size == 0 {
            continue;
        }
        let f = reader.font(font_id).expect("font");
        let tuned = f.instruments.iter().flatten().flat_map(|i| [&i.low, &i.normal, &i.high]).chain(f.drums.iter().flatten().map(|d| &d.sound)).chain(f.sfx.iter()).flatten();
        for t in tuned {
            let s = &t.sample;
            if !seen.insert((s.bank, s.addr, s.size)) {
                continue;
            }
            let reference = reader.decode(s).expect("decodes");
            let bytes = reader.sample_data(s).expect("data");
            // A frame whose predictor is past the book reads, on the RSP, whatever coefficients
            // an earlier `aLoadADPCM` left in DMEM; the reference decoder uses predictor 0.
            // Compare up to the first one.
            let fsize = if s.codec == CODEC_SMALL_ADPCM { 5 } else { 9 };
            let npred = s.book.as_ref().unwrap().npred;
            let frames = reference.len().div_ceil(16);
            let bad = (0..frames).find(|&f| bytes.get(f * fsize).is_some_and(|&b| (b & 0xF) as usize >= npred));
            if let Some(f) = bad {
                past_book.push(format!("font {font_id}: bank {} 0x{:X} from frame {f} of {frames}", s.bank, s.addr));
            }
            let n = bad.map(|f| f * 16).unwrap_or(reference.len());
            let reference = reference[..n].to_vec();
            if n == 0 {
                continue;
            }
            let got = microcode_decode(&mut rsp, &mut ram, s, &bytes[..bytes.len().min(0x8_0000)], 0, n, A_INIT, None);
            if let Some(i) = (0..n).find(|&i| got[i] != reference[i]) {
                panic!(
                    "font {font_id}: the sample at bank {} 0x{:X} (codec {}, size 0x{:X}, {n} samples, book {}x{}) decodes differently from sample {i}: {:?} against {:?}",
                    s.bank,
                    s.addr,
                    s.codec,
                    s.size,
                    s.book.as_ref().unwrap().order,
                    s.book.as_ref().unwrap().npred,
                    &got[i.saturating_sub(2)..(i + 6).min(n)],
                    &reference[i.saturating_sub(2)..(i + 6).min(n)]
                );
            }
            samples += 1;

            if let Some(state) = s.lp.state.filter(|_| bad.is_none()) {
                looped += 1;
                let fs = (s.lp.start as usize) & !15;
                if reference[fs..fs + 16] != state {
                    mismatched_states.push(format!("bank {} 0x{:X} (loop {}..{}): stored state != the decoded frame", s.bank, s.addr, s.lp.start, s.lp.end));
                    continue;
                }
                // `AudioSynth_ProcessNote` on a restart: the frame holding the loop start from
                // the state, decoding on from the next one.
                let after = n - (fs + 16);
                if after == 0 {
                    continue;
                }
                let got = microcode_decode(&mut rsp, &mut ram, s, &bytes[..bytes.len().min(0x8_0000)], fs / 16 + 1, after, A_LOOP, Some(state));
                assert_eq!(got, reference[fs + 16..], "font {font_id}: after the loop's restart, bank {} 0x{:X}", s.bank, s.addr);
                loop_exact += 1;
            }
        }
    }
    println!("{samples} samples decoded alike; {looped} looped, {loop_exact} restart alike; stored states that differ: {mismatched_states:?}; predictors past the book: {past_book:?}");
    assert!(samples > 400, "every font's samples: {samples}");
    assert!(mismatched_states.len() <= 1, "{mismatched_states:?}");
}

/// The C's `Audio_AdsrUpdate` while the note is held (`ADSR_STATE_LOOP`/`FADE`, no `GOTO` or
/// `RESTART` in these envelopes): the value each update returns.
fn adsr_values(envelope: &[(i16, i16)], updates_per_frame_scaled: f32, n: usize) -> Vec<f32> {
    let (mut current, mut velocity, mut delay, mut index, mut fading, mut hang) = (0.0f32, 0.0f32, 0i16, 0usize, false, false);
    let mut out = Vec::new();
    for _ in 0..n {
        if !hang && !fading {
            let (d, arg) = envelope[index];
            if d == ADSR_HANG {
                hang = true;
            } else {
                delay = (d as f32 * updates_per_frame_scaled) as i32 as i16;
                if delay == 0 {
                    delay = 1;
                }
                let mut target = arg as f32 / 32767.0f32;
                target *= target;
                velocity = (target - current) / delay as f32;
                index += 1;
                fading = true;
            }
        }
        if fading {
            current += velocity;
            delay -= 1;
            if delay <= 0 {
                fading = false;
            }
        }
        out.push(current.clamp(0.0, 1.0));
    }
    out
}

/// Runs `r` for `retraces`, collecting each update's `targetVolLeft` for the (one) note.
fn run_collecting(r: &mut Renderer, retraces: u64) -> Vec<u16> {
    let mut vols = Vec::new();
    for _ in 0..retraces {
        r.retrace();
        let c = &r.ctx;
        let n = c.num_notes as usize;
        for u in 0..c.audio_buffer_parameters.updates_per_frame as usize {
            if let Some(s) = (0..n).map(|i| c.note_subs_eu[u * n + i]).find(|s| s.enabled) {
                vols.push(s.target_vol_left);
            }
        }
    }
    vols
}

/// Normalized cross-correlation of `a` against `b`, at the lag (of `b` behind `a`) that's best.
fn best_correlation(a: &[f64], b: &[f64], max_lag: usize) -> (f64, usize) {
    let mut best = (f64::MIN, 0);
    let n = a.len().min(b.len().saturating_sub(max_lag));
    for lag in 0..max_lag {
        let (mut sab, mut saa, mut sbb) = (0.0, 0.0, 0.0);
        for i in 0..n {
            let (x, y) = (a[i], b[i + lag]);
            sab += x * y;
            saa += x * x;
            sbb += y * y;
        }
        let c = sab / (saa.sqrt() * sbb.sqrt()).max(1e-9);
        if c > best.0 {
            best = (c, lag);
        }
    }
    best
}

#[test]
fn a_note_of_a_kokiri_instrument_plays_its_sample() {
    let Some(data) = data() else { return };
    let reader = FontReader::new(&data);
    let font = reader.font(15).expect("Kokiri Forest's font");
    let inst = font.instruments[4].clone().expect("instrument 4");
    let tuned = inst.normal.clone().unwrap();
    assert_eq!(tuned.tuning, 1.0, "the instrument plays its sample at 32 kHz at C4");
    let decoded = reader.decode(&tuned.sample).expect("decodes");

    // C4 for 96 tatums at 120 bpm (one second), held, then the layer ends.
    let mut r = Renderer::new(&data);
    assert_eq!(r.output_rate(), 32006, "osAiSetFrequency(32000) on NTSC");
    let c = &r.ctx;
    assert_eq!(c.audio_buffer_parameters.updates_per_frame, 3, "(544 + 16) / 0xD0 + 1");
    assert_eq!(c.tempo_internal_to_external, 10770, "3 * 2880000 / 48 / 16.713");
    r.play_script(0, 15, &note_script(4, 39, 127, 96, 300, 120));
    let vols = run_collecting(&mut r, 120);

    // Its volume: the ADSR times gDefaultPanVolume[64] (pan 0x40 at the channel's full weight).
    let pan = data.tables.default_pan_volume[64];
    let adsr = adsr_values(&inst.envelope, r.ctx.audio_buffer_parameters.updates_per_frame_scaled, vols.len());
    let held = 96 * 10770 / 5760; // updates the 96 tatums take (a tick every 10770 / 5760 updates)
    for (k, v) in vols.iter().enumerate().take(held - 2) {
        let want = ((1.0f32 * adsr[k] * pan) * (0x1000 as f32 - 0.001f32)) as i32 as u16;
        assert_eq!(*v, want, "update {k}: targetVolLeft");
    }

    // C4: the output follows the sample one for one, through the resampler's filter.
    let left: Vec<f64> = r.out.chunks(2).map(|f| f[0] as f64).collect();
    let start = 2000;
    let a: Vec<f64> = decoded[start..start + 8192].iter().map(|&v| v as f64).collect();
    let (corr, lag) = best_correlation(&a, &left[start..], 2400);
    println!("C4: correlation {corr:.5} at a lag of {lag} samples");
    assert!(corr > 0.995, "C4 output against the sample: {corr} at {lag}");

    // Silent once the decay has run: adsrDecayTable[239] = (1/3) / 12 per update.
    let decay = r.ctx.adsr_decay_table[inst.adsr_decay_index as usize];
    assert!((decay - (1.0 / 3.0) / 12.0).abs() < 1e-6, "adsrDecayTable[239]: {decay}");
    let end = left.len() - 32006 / 4;
    assert!(left[end..].iter().all(|&v| v == 0.0), "the note decayed to silence");
    assert!(left[32006 / 2..32006].iter().map(|v| v.abs()).fold(0.0, f64::max) > 5000.0, "the note sounds while held");

    // C3: half speed (gPitchFrequencies[27] = 0.5): the sample, stretched twice as long.
    let mut r = Renderer::new(&data);
    r.play_script(0, 15, &note_script(4, 27, 127, 96, 300, 120));
    r.run(90);
    let left: Vec<f64> = r.out.chunks(2).map(|f| f[0] as f64).collect();
    let stretched: Vec<f64> = (0..8192)
        .map(|i| {
            let x = (start / 2) as f64 + i as f64 * 0.5;
            let (k, f) = (x.floor() as usize, x.fract());
            decoded[k] as f64 * (1.0 - f) + decoded[k + 1] as f64 * f
        })
        .collect();
    let (corr, lag) = best_correlation(&stretched, &left[start..], 2400);
    println!("C3: correlation {corr:.5} at a lag of {lag} samples");
    assert!(corr > 0.98, "C3 output against the sample at half speed: {corr} at {lag}");
}

#[test]
fn a_drum_plays_its_sample() {
    let Some(data) = data() else { return };
    let reader = FontReader::new(&data);
    // Font 3 (the first with a full drum kit): its first drum with a sample.
    let font = reader.font(3).expect("font 3");
    let (d, drum) = font.drums.iter().enumerate().find_map(|(i, d)| d.as_ref().filter(|d| d.sound.is_some()).map(|d| (i, d.clone()))).expect("a drum");
    let tuned = drum.sound.unwrap();
    let decoded = reader.decode(&tuned.sample).expect("decodes");
    let mut r = Renderer::new(&data);
    r.play_script(0, 3, &note_script(0x7F, d as u8, 127, 48, 200, 120));
    r.run(60);
    let left: Vec<f64> = r.out.chunks(2).map(|f| f[0] as f64).collect();
    let peak = left.iter().map(|v| v.abs()).fold(0.0, f64::max);
    assert!(peak > 1000.0, "drum {d} sounds: peak {peak}");
    // Its sample at its tuning (drums play at the tuning, whatever the note).
    let len = (decoded.len() as f64 / tuned.tuning as f64).min(4096.0) as usize;
    let resampled: Vec<f64> = (0..len)
        .map(|i| {
            let x = i as f64 * tuned.tuning as f64;
            let (k, f) = (x.floor() as usize, x.fract());
            decoded[k.min(decoded.len() - 2)] as f64 * (1.0 - f) + decoded[(k + 1).min(decoded.len() - 1)] as f64 * f
        })
        .collect();
    let (corr, lag) = best_correlation(&resampled, &left, 2400);
    println!("drum {d} (tuning {}): correlation {corr:.4} at {lag}", tuned.tuning);
    assert!(corr > 0.9, "drum {d} against its sample: {corr} at {lag}");
}

#[test]
fn kokiri_forests_sequence_plays_as_the_extractor_reads_it() {
    let Some(data) = data() else { return };
    let mut r = Renderer::new(&data);
    r.start_sequence(0, 60);
    let mut at_5951 = None;
    while r.ctx.stats.seq_ticks[0] < 5952 {
        if r.ctx.stats.seq_ticks[0] == 5951 && at_5951.is_none() {
            at_5951 = Some(r.ctx.stats.notes_struck);
        }
        r.retrace();
        assert!(r.retraces < 60 * 120, "5952 ticks take about 51 s");
    }
    let c: &AudioContext = &r.ctx;
    assert!(c.seq_players[0].enabled, "the sequence still plays (it loops)");
    assert_eq!(c.seq_players[0].default_font, 15, "the sequence's font (gSequenceFontTable)");
    println!("5952 ticks in {} retraces ({:.2} s), {} notes ({at_5951:?} at tick 5951)", r.retraces, r.retraces as f64 / 60.0, c.stats.notes_struck);
    // The extractor's interpreter (oot_extract::audio) counts 597 notes before the loop, in the
    // ticks before the one that jumps back (it drops that tick's second pass).
    assert_eq!(at_5951, Some(597), "notes struck in the first 5951 ticks");
    let peak = r.out.iter().map(|v| (*v as i32).abs()).max().unwrap();
    let clipped = r.out.iter().filter(|&&v| v == i16::MAX || v == i16::MIN).count();
    assert!(peak > 5000 && clipped == 0, "it sounds without clipping: peak {peak}, {clipped} clipped");
}

#[test]
fn audio_heap_init_lays_out_the_heap_as_the_c() {
    let Some(data) = data() else { return };
    let c = AudioContext::new(&data);
    let p = c.audio_buffer_parameters;
    // AudioHeap_Init on spec 0 at 32 kHz, NTSC (refreshRate 60): ALIGN16(32000 / 60) = 544.
    assert_eq!((p.samples_per_frame_target, p.min_ai_buffer_length, p.max_ai_buffer_length), (544, 528, 560));
    assert_eq!((p.updates_per_frame, p.samples_per_update, p.samples_per_update_max, p.samples_per_update_min), (3, 176, 184, 168));
    assert_eq!((p.resample_rate, p.updates_per_frame_scaled), (1.0, 0.75));
    assert_eq!(p.ai_sampling_frequency, 32006, "48681812 / (u32)(48681812 / 32000.0f + 0.5f)");
    assert_eq!(c.num_notes, 24);
    // The notes, their synthesis buffers and the reverbs' ring buffers fit in the misc pool
    // (0x21E90 bytes); of the sample DMA buffers the C asks for (3 per note of 0x300 bytes, then
    // 1 per note of 0x200), the 72 short-lived ones fit and only 5 long-lived ones: the
    // second loop stops when the pool is full (`AudioLoad_InitSampleDmaBuffers`' `break`).
    assert!(c.notes_addr != 0 && c.notes.iter().all(|n| n.synthesis_state.synthesis_buffers != 0));
    assert_eq!(c.num_synthesis_reverbs, 2);
    for (i, ws) in [(0, 0x30 * 64), (1, 0x20 * 64)] {
        let r = &c.synthesis_reverbs[i];
        assert!(r.left_ring_buf != 0 && r.right_ring_buf != 0, "reverb {i}'s ring buffers");
        assert_eq!((r.window_size as i32, r.decay_ratio, r.frames_to_ignore, r.use_reverb), (ws, if i == 0 { 0x3000 } else { 0x800 }, 2, 8));
    }
    assert_eq!(c.misc_pool.size, 0x21E90);
    assert_eq!((c.sample_dma_count, c.sample_dma_list_size1), (72 + 5, 72));
    assert_eq!(c.sample_dmas.iter().map(|d| d.size as u32).sum::<u32>(), 72 * 0x300 + 5 * 0x200);
    assert!(c.misc_pool.cur_ram_addr <= c.misc_pool.start_ram_addr + c.misc_pool.size as u32, "the misc pool holds it all");
    // AudioHeap_InitAdsrDecayTable: 255 is 1 / 0.25 of updatesPerFrameInv; 0 never decays.
    let inv = 1.0f32 / 3.0;
    assert!((c.adsr_decay_table[255] - inv / 0.25).abs() < 1e-6 && c.adsr_decay_table[0] == 0.0);
    assert!((c.adsr_decay_table[100] - inv / (4.0 * 43.0)).abs() < 1e-7, "4 * (143 - i)");
}

#[test]
fn a_note_with_reverb_rings_on_decaying_by_the_decay_ratio() {
    let Some(data) = data() else { return };
    let render = |reverb: u8| {
        let mut r = Renderer::new(&data);
        r.play_script(0, 15, &note_script_reverb(4, 39, 127, 48, 400, 120, reverb));
        r.run(200);
        r.out.chunks(2).map(|f| f[0] as f64).collect::<Vec<f64>>()
    };
    let dry = render(0);
    let wet = render(0x7F);
    let end = dry.iter().rposition(|&v| v != 0.0).expect("the note sounds");
    assert!(wet[end + 1..].iter().any(|&v| v != 0.0), "the reverb rings on after the note");
    // Once the note and its first trip round the ring are over, each window of 3072 samples
    // is the one before at 0.375.
    let window = 0x30 * 64;
    let rms = |a: &[f64]| (a.iter().map(|v| v * v).sum::<f64>() / a.len() as f64).sqrt();
    let start = end + window;
    let levels: Vec<f64> = (0..4).map(|k| rms(&wet[start + k * window..start + (k + 1) * window])).collect();
    println!("reverb tail RMS per window: {levels:?}");
    for k in 0..3 {
        let ratio = levels[k + 1] / levels[k];
        assert!((ratio - 0.375).abs() < 0.01, "window {k} to {}: {ratio}", k + 1);
    }
}
