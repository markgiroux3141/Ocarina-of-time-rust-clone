//! Audio: soundfonts -> JSON, samples (VADPCM) -> WAV, sequences -> raw `.seq` + JSON + MIDI.
//!
//! Layouts follow the decomp (`include/z64audio.h`, `src/code/audio_load.c`,
//! `audio_synthesis.c`, `audio_seqplayer.c`):
//!
//! * `AudioTable`: `s16 numEntries, s16 unkMediumParam, u32 romAddr, pad[8]`, then 16-byte
//!   `AudioTableEntry {u32 romAddr, u32 size, s8 medium, s8 cachePolicy, s16 shortData1..3}`.
//!   An entry with `size == 0` is an alias: `romAddr` holds the real index
//!   (`AudioLoad_GetRealTableIndex`). Addresses are relative to the Audiobank/Audioseq/Audiotable file.
//! * Soundfont entry: `shortData1 = bank1 << 8 | bank2` (0xFF = none),
//!   `shortData2 = numInstruments << 8 | numDrums`, `shortData3 = numSfx`.
//! * Font data (offsets relative to the font start): `[0]` offset of a `u32` array of drum offsets,
//!   `[1]` offset of an inline `SoundEffect[numSfx]` array (8 bytes each), `[2..]` instrument offsets.
//! * `Sample` header: `codec:4, medium:2, unk_bit26:1, isRelocated:1, size:24`, then `sampleAddr`
//!   (relative to sample bank `medium == 0 ? bank1 : bank2`), `loop` and `book` (relative to the font).
//!
//! Samples are decoded like the RSP's aADPCMdec (checked bit for bit against the predictor
//! state stored with each loop). The sequence -> MIDI conversion is a tick-accurate
//! re-implementation of the sequence player (sequence, channel and layer scripts, including
//! loops, calls, branches, dynamic tables and self-modifying writes), recording notes, program,
//! volume, pan, pitch bend and tempo. Not converted: envelopes/release, vibrato, portamento
//! (the start note plays), per-layer bend, reverb, filters, random velocity/gate variance,
//! continuous (legato) notes (re-struck), and game-driven IO ports (left at -1).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use anyhow::{Context, Result, bail};
use oot_import::project::Project;
use serde_json::{Value, json};

use crate::{sanitize, write_json};

// ROM offsets (gc-eu-mq-dbg, uncompressed) from the decomp's data/audio_tables.rodata.s.
const SOUNDFONT_TABLE: (usize, usize) = (0xBCC270, 0x270);
const SEQUENCE_FONT_TABLE: (usize, usize) = (0xBCC4E0, 0x1C0);
const SEQUENCE_TABLE: (usize, usize) = (0xBCC6A0, 0x6F0);
const SAMPLE_BANK_TABLE: (usize, usize) = (0xBCCD90, 0x80);

/// The synthesis output rate; a tuning of 1.0 plays a sample at this rate.
const OUTPUT_RATE: f64 = 32000.0;
/// Sequence ticks per beat (TATUMS_PER_BEAT); used as the MIDI division.
const TATUMS_PER_BEAT: u32 = 48;
/// Hard cap for one MIDI conversion (seconds of music).
const MIDI_MAX_SECONDS: f64 = 15.0 * 60.0;

const CODEC_ADPCM: u8 = 0;
const CODEC_S8: u8 = 1;
const CODEC_S16_INMEMORY: u8 = 2;
const CODEC_SMALL_ADPCM: u8 = 3;
const CODEC_S16: u8 = 5;

fn codec_name(c: u8) -> &'static str {
    match c {
        0 => "ADPCM",
        1 => "S8",
        2 => "S16_INMEMORY",
        3 => "SMALL_ADPCM",
        4 => "REVERB",
        5 => "S16",
        _ => "UNKNOWN",
    }
}

// ---------------------------------------------------------------------------------------------
// Byte helpers

fn rd_u8(d: &[u8], o: usize) -> Result<u8> {
    d.get(o).copied().with_context(|| format!("read out of bounds at 0x{o:X} (len 0x{:X})", d.len()))
}
fn rd_u16(d: &[u8], o: usize) -> Result<u16> {
    let b = d.get(o..o + 2).with_context(|| format!("read out of bounds at 0x{o:X} (len 0x{:X})", d.len()))?;
    Ok(u16::from_be_bytes([b[0], b[1]]))
}
fn rd_u32(d: &[u8], o: usize) -> Result<u32> {
    let b = d.get(o..o + 4).with_context(|| format!("read out of bounds at 0x{o:X} (len 0x{:X})", d.len()))?;
    Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}
fn rd_f32(d: &[u8], o: usize) -> Result<f32> {
    Ok(f32::from_bits(rd_u32(d, o)?))
}

// ---------------------------------------------------------------------------------------------
// Tables

#[derive(Clone, Copy, Debug)]
struct Entry {
    addr: u32,
    size: u32,
    medium: i8,
    cache: i8,
    d1: u16,
    d2: u16,
    d3: u16,
}

fn parse_table(rom: &[u8], (off, len): (usize, usize)) -> Result<Vec<Entry>> {
    let t = rom.get(off..off + len).context("audio table outside ROM")?;
    let n = rd_u16(t, 0)? as usize;
    if 16 + n * 16 > len {
        bail!("audio table at 0x{off:X} claims {n} entries but is only 0x{len:X} bytes");
    }
    (0..n)
        .map(|i| {
            let e = 16 + i * 16;
            Ok(Entry {
                addr: rd_u32(t, e)?,
                size: rd_u32(t, e + 4)?,
                medium: t[e + 8] as i8,
                cache: t[e + 9] as i8,
                d1: rd_u16(t, e + 10)?,
                d2: rd_u16(t, e + 12)?,
                d3: rd_u16(t, e + 14)?,
            })
        })
        .collect()
}

/// `AudioLoad_GetRealTableIndex`: a size-0 entry's address is the index of the real entry.
fn real_id(t: &[Entry], id: usize) -> usize {
    match t.get(id) {
        Some(e) if e.size == 0 => e.addr as usize,
        _ => id,
    }
}

fn entry_slice<'a>(file: &'a [u8], e: &Entry, what: &str) -> Result<&'a [u8]> {
    let (a, s) = (e.addr as usize, e.size as usize);
    file.get(a..a + s).with_context(|| format!("{what}: 0x{a:X}+0x{s:X} outside file (0x{:X})", file.len()))
}

// ---------------------------------------------------------------------------------------------
// Soundfont structures

#[derive(Clone, Debug)]
struct Book {
    order: usize,
    npred: usize,
    coefs: Vec<i16>, // [pred][order][8]
}

#[derive(Clone, Debug, PartialEq)]
struct Loop {
    start: u32,
    end: u32,
    count: u32,
    state: Option<[i16; 16]>,
}

#[derive(Clone, Debug)]
struct SampleHdr {
    codec: u8,
    medium: u8,
    unk_bit26: bool,
    size: u32,
    addr: u32,
    loop_off: u32,
    book_off: u32,
    lp: Loop,
    book: Option<Book>,
}

fn parse_sample(font: &[u8], off: u32) -> Result<SampleHdr> {
    let o = off as usize;
    let w = rd_u32(font, o)?;
    let codec = (w >> 28) as u8;
    let medium = ((w >> 26) & 3) as u8;
    let unk_bit26 = (w >> 25) & 1 != 0;
    let size = w & 0xFF_FFFF;
    let addr = rd_u32(font, o + 4)?;
    let loop_off = rd_u32(font, o + 8)?;
    let book_off = rd_u32(font, o + 12)?;
    let lo = loop_off as usize;
    let (start, end, count) = (rd_u32(font, lo)?, rd_u32(font, lo + 4)?, rd_u32(font, lo + 8)?);
    let state = if count != 0 {
        let mut s = [0i16; 16];
        for (i, v) in s.iter_mut().enumerate() {
            *v = rd_u16(font, lo + 16 + i * 2)? as i16;
        }
        Some(s)
    } else {
        None
    };
    let book = if codec == CODEC_ADPCM || codec == CODEC_SMALL_ADPCM {
        let bo = book_off as usize;
        let order = rd_u32(font, bo)? as i32;
        let npred = rd_u32(font, bo + 4)? as i32;
        if !(1..=8).contains(&order) || !(1..=16).contains(&npred) {
            bail!("sample at 0x{off:X}: implausible ADPCM book order {order} predictors {npred}");
        }
        let (order, npred) = (order as usize, npred as usize);
        let coefs = (0..8 * order * npred).map(|i| rd_u16(font, bo + 8 + i * 2).map(|v| v as i16)).collect::<Result<_>>()?;
        Some(Book { order, npred, coefs })
    } else {
        None
    };
    Ok(SampleHdr { codec, medium, unk_bit26, size, addr, loop_off, book_off, lp: Loop { start, end, count, state }, book })
}

#[derive(Clone, Debug)]
struct TunedRef {
    hdr_off: u32,
    tuning: f32,
    sample: Option<usize>, // registry index
    error: Option<String>,
}

fn parse_tuned(font: &[u8], o: usize) -> Result<Option<TunedRef>> {
    let hdr_off = rd_u32(font, o)?;
    let tuning = rd_f32(font, o + 4)?;
    if hdr_off == 0 {
        return Ok(None);
    }
    Ok(Some(TunedRef { hdr_off, tuning, sample: None, error: None }))
}

fn parse_envelope(font: &[u8], off: u32) -> Vec<(i16, i16)> {
    let mut v = Vec::new();
    for i in 0..48 {
        let o = off as usize + i * 4;
        let (Ok(d), Ok(a)) = (rd_u16(font, o), rd_u16(font, o + 2)) else { break };
        let (d, a) = (d as i16, a as i16);
        v.push((d, a));
        if d <= 0 {
            break; // ADSR_DISABLE / HANG / GOTO / RESTART end the script
        }
    }
    v
}

#[derive(Clone, Debug)]
struct Instrument {
    off: u32,
    lo: u8,
    hi: u8,
    release: u8,
    env_off: u32,
    envelope: Vec<(i16, i16)>,
    low: Option<TunedRef>,
    normal: Option<TunedRef>,
    high: Option<TunedRef>,
}

#[derive(Clone, Debug)]
struct Drum {
    off: u32,
    release: u8,
    pan: u8,
    env_off: u32,
    envelope: Vec<(i16, i16)>,
    sound: Option<TunedRef>,
}

#[derive(Clone, Debug, Default)]
struct Font {
    bank1: u8,
    bank2: u8,
    instruments: Vec<Option<Instrument>>,
    drums: Vec<Option<Drum>>,
    sfx: Vec<Option<TunedRef>>,
    errors: Vec<String>,
}

fn parse_font(data: &[u8], e: &Entry) -> Font {
    let mut f = Font { bank1: (e.d1 >> 8) as u8, bank2: e.d1 as u8, ..Default::default() };
    let (ninst, ndrums, nsfx) = (((e.d2 >> 8) & 0xFF) as usize, (e.d2 & 0xFF) as usize, e.d3 as usize);
    let err = |f: &mut Font, s: String| f.errors.push(s);

    // Drums: fontData[0] -> u32 drum offsets
    match rd_u32(data, 0) {
        Ok(list) if list != 0 && ndrums != 0 => {
            for i in 0..ndrums {
                let r = (|| -> Result<Option<Drum>> {
                    let off = rd_u32(data, list as usize + i * 4)?;
                    if off == 0 {
                        return Ok(None);
                    }
                    let o = off as usize;
                    let env_off = rd_u32(data, o + 12)?;
                    Ok(Some(Drum {
                        off,
                        release: rd_u8(data, o)?,
                        pan: rd_u8(data, o + 1)?,
                        env_off,
                        envelope: parse_envelope(data, env_off),
                        sound: parse_tuned(data, o + 4)?,
                    }))
                })();
                match r {
                    Ok(d) => f.drums.push(d),
                    Err(x) => {
                        err(&mut f, format!("drum {i}: {x:#}"));
                        f.drums.push(None)
                    }
                }
            }
        }
        Ok(_) => {}
        Err(x) => err(&mut f, format!("drum list: {x:#}")),
    }

    // Sound effects: fontData[1] -> inline SoundEffect[numSfx]
    match rd_u32(data, 4) {
        Ok(list) if list != 0 && nsfx != 0 => {
            for i in 0..nsfx {
                match parse_tuned(data, list as usize + i * 8) {
                    Ok(t) => f.sfx.push(t),
                    Err(x) => {
                        err(&mut f, format!("sfx {i}: {x:#}"));
                        f.sfx.push(None)
                    }
                }
            }
        }
        Ok(_) => {}
        Err(x) => err(&mut f, format!("sfx list: {x:#}")),
    }

    // Instruments: fontData[2 + i]; ids >= 126 are reserved (AudioLoad_RelocateFont clamps to 126)
    for i in 0..ninst.min(126) {
        let r = (|| -> Result<Option<Instrument>> {
            let off = rd_u32(data, 8 + i * 4)?;
            if off == 0 {
                return Ok(None);
            }
            let o = off as usize;
            let lo = rd_u8(data, o + 1)?;
            let hi = rd_u8(data, o + 2)?;
            let env_off = rd_u32(data, o + 4)?;
            Ok(Some(Instrument {
                off,
                lo,
                hi,
                release: rd_u8(data, o + 3)?,
                env_off,
                envelope: parse_envelope(data, env_off),
                // RelocateFont only touches the low/high samples when the ranges say they exist
                low: if lo != 0 { parse_tuned(data, o + 8)? } else { None },
                normal: parse_tuned(data, o + 0x10)?,
                high: if hi != 0x7F { parse_tuned(data, o + 0x18)? } else { None },
            }))
        })();
        match r {
            Ok(x) => f.instruments.push(x),
            Err(x) => {
                err(&mut f, format!("instrument {i}: {x:#}"));
                f.instruments.push(None)
            }
        }
    }
    f
}

// ---------------------------------------------------------------------------------------------
// VADPCM decoding (the RSP's aADPCMdec, as in N64 audio HLE)

/// Decodes `n` samples. Returns the PCM and an error string if the data ran out early.
fn decode_sample(codec: u8, data: &[u8], n: usize, book: Option<&Book>) -> (Vec<i16>, Vec<String>) {
    let mut out: Vec<i16> = Vec::with_capacity(n);
    let mut errs = Vec::new();
    match codec {
        CODEC_ADPCM | CODEC_SMALL_ADPCM => {
            let Some(book) = book else {
                errs.push("ADPCM sample without book".into());
                return (out, errs);
            };
            let small = codec == CODEC_SMALL_ADPCM;
            let fsize = if small { 5 } else { 9 };
            let order = book.order;
            let mut bad_pred = 0;
            let mut f = 0;
            while out.len() < n {
                let Some(fr) = data.get(f * fsize..f * fsize + fsize) else {
                    errs.push(format!("data ends after {} frames, {} of {n} samples decoded", f, out.len()));
                    break;
                };
                let scale = (fr[0] >> 4) as u32;
                let mut pred = (fr[0] & 0xF) as usize;
                if pred >= book.npred {
                    bad_pred += 1;
                    pred = 0;
                }
                let mut ins = [0i32; 16];
                if small {
                    let rshift = 14u32.saturating_sub(scale);
                    for i in 0..4 {
                        let b = fr[1 + i] as u16;
                        for (j, (mask, lsh)) in [(0xC0u16, 8u32), (0x30, 10), (0x0C, 12), (0x03, 14)].iter().enumerate() {
                            ins[i * 4 + j] = ((((b & mask) << lsh) as i16) >> rshift) as i32;
                        }
                    }
                } else {
                    let rshift = 12u32.saturating_sub(scale);
                    for i in 0..8 {
                        let b = fr[1 + i] as u16;
                        ins[i * 2] = ((((b & 0xF0) << 8) as i16) >> rshift) as i32;
                        ins[i * 2 + 1] = ((((b & 0x0F) << 12) as i16) >> rshift) as i32;
                    }
                }
                let tbl = &book.coefs[pred * order * 8..(pred + 1) * order * 8];
                let mut frame = [0i16; 16];
                for h in 0..2 {
                    // history: the `order` samples before this 8-sample group, oldest first
                    let mut hist = [0i64; 8];
                    for (k, hv) in hist.iter_mut().enumerate().take(order) {
                        let back = order - k; // 1..=order samples back
                        let idx = h as isize * 8 - back as isize;
                        *hv = if idx >= 0 {
                            frame[idx as usize] as i64
                        } else {
                            let p = out.len() as isize + idx;
                            if p >= 0 { out[p as usize] as i64 } else { 0 }
                        };
                    }
                    for i in 0..8 {
                        let mut acc: i64 = (ins[h * 8 + i] as i64) << 11;
                        for (k, hv) in hist.iter().enumerate().take(order) {
                            acc += tbl[k * 8 + i] as i64 * hv;
                        }
                        let last = &tbl[(order - 1) * 8..order * 8];
                        for m in 0..i {
                            acc += last[m] as i64 * ins[h * 8 + i - 1 - m] as i64;
                        }
                        frame[h * 8 + i] = (acc >> 11).clamp(-32768, 32767) as i16;
                    }
                }
                let take = (n - out.len()).min(16);
                out.extend_from_slice(&frame[..take]);
                f += 1;
            }
            if bad_pred > 0 {
                errs.push(format!("{bad_pred} frames used a predictor index >= {}", book.npred));
            }
        }
        CODEC_S8 => {
            for i in 0..n {
                match data.get(i) {
                    Some(&b) => out.push(((b as i8) as i16) << 8),
                    None => {
                        errs.push(format!("S8 data ends at {i} of {n} samples"));
                        break;
                    }
                }
            }
        }
        CODEC_S16 | CODEC_S16_INMEMORY => {
            for i in 0..n {
                match data.get(i * 2..i * 2 + 2) {
                    Some(b) => out.push(i16::from_be_bytes([b[0], b[1]])),
                    None => {
                        errs.push(format!("S16 data ends at {i} of {n} samples"));
                        break;
                    }
                }
            }
        }
        c => errs.push(format!("unsupported codec {c} ({})", codec_name(c))),
    }
    (out, errs)
}

/// Samples the data can hold, from the frame size of the codec.
fn capacity_samples(codec: u8, size: u32) -> usize {
    match codec {
        CODEC_ADPCM => size as usize / 9 * 16,
        CODEC_SMALL_ADPCM => size as usize / 5 * 16,
        CODEC_S8 => size as usize,
        CODEC_S16 | CODEC_S16_INMEMORY => size as usize / 2,
        _ => 0,
    }
}

// ---------------------------------------------------------------------------------------------
// WAV

/// 16-bit mono WAV with a `smpl` chunk (unity note, optional forward loop).
fn wav_bytes(pcm: &[i16], rate: u32, unity: u8, lp: Option<(u32, u32, u32)>) -> Vec<u8> {
    let data_len = pcm.len() * 2;
    let mut smpl = Vec::new();
    let period_ns = (1e9 / rate.max(1) as f64).round() as u32;
    for v in [0u32, 0, period_ns, unity as u32, 0, 0, 0, lp.is_some() as u32, 0] {
        smpl.extend_from_slice(&v.to_le_bytes());
    }
    if let Some((start, end, count)) = lp {
        // one forward loop; smpl end is inclusive; play count 0 = infinite
        let plays = if count == u32::MAX { 0 } else { count };
        for v in [0u32, 0, start, end.saturating_sub(1), 0, plays] {
            smpl.extend_from_slice(&v.to_le_bytes());
        }
    }
    let riff_len = 4 + (8 + 16) + (8 + data_len) + 8 + smpl.len();
    let mut w = Vec::with_capacity(riff_len + 8);
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(riff_len as u32).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes()); // PCM
    w.extend_from_slice(&1u16.to_le_bytes()); // mono
    w.extend_from_slice(&rate.to_le_bytes());
    w.extend_from_slice(&(rate * 2).to_le_bytes());
    w.extend_from_slice(&2u16.to_le_bytes());
    w.extend_from_slice(&16u16.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&(data_len as u32).to_le_bytes());
    for s in pcm {
        w.extend_from_slice(&s.to_le_bytes());
    }
    w.extend_from_slice(b"smpl");
    w.extend_from_slice(&(smpl.len() as u32).to_le_bytes());
    w.extend_from_slice(&smpl);
    w
}

/// WAV rate and smpl unity note for a sample whose C4 playback rate is `c4_rate`.
///
/// Drums and sound effects always play at `c4_rate`, so that is their WAV rate. Instrument
/// samples play at `c4_rate * 2^((key - 60) / 12)`; any `rate = c4_rate * 2^(n/12)` with unity
/// note `60 + n` reproduces that, so we pick the `n` that lands on a standard recording rate
/// (within 5 cents), else keep `n = 0` shifted by octaves into 8-48 kHz.
fn wav_rate(c4_rate: f64, keyed: bool) -> (u32, u8) {
    const STANDARD: [f64; 8] = [8000.0, 11025.0, 16000.0, 22050.0, 24000.0, 32000.0, 44100.0, 48000.0];
    let clamp = |r: f64| r.round().clamp(1.0, 384_000.0) as u32;
    if !keyed || c4_rate <= 0.0 {
        return (clamp(c4_rate), 60);
    }
    let mut best: Option<(f64, i32)> = None;
    for n in -48i32..=48 {
        let r = c4_rate * 2f64.powf(n as f64 / 12.0);
        if !(7900.0..=48500.0).contains(&r) {
            continue;
        }
        for sr in STANDARD {
            let cents = (1200.0 * (r / sr).log2()).abs();
            if cents < 5.0 && best.is_none_or(|(c, bn)| cents < c - 1e-9 || ((cents - c).abs() < 1e-9 && n.abs() < bn.abs())) {
                best = Some((cents, n));
            }
        }
    }
    let n = match best {
        Some((_, n)) => n,
        None => {
            let mut n = 0;
            while c4_rate * 2f64.powf(n as f64 / 12.0) < 8000.0 && n < 60 {
                n += 12;
            }
            while c4_rate * 2f64.powf(n as f64 / 12.0) > 48000.0 && n > -60 {
                n -= 12;
            }
            n
        }
    };
    (clamp(c4_rate * 2f64.powf(n as f64 / 12.0)), (60 + n).clamp(0, 127) as u8)
}

// ---------------------------------------------------------------------------------------------
// Sample registry

struct SampleRec {
    file: String,
    bank: usize,
    hdr: SampleHdr,
    tuning: f32,
    rate: u32,
    unity: u8,
    num_samples: usize,
    capacity: usize,
    refs: Vec<String>,
    other_tunings: BTreeSet<u32>, // f32 bits of differing tunings in other references
    loop_variants: usize,
    rms: f64,
    peak: i32,
    clip_pct: f64,
    loop_state_diff: Option<i32>, // max |predictorState - decoded frame holding the loop start|
    errors: Vec<String>,
    notes: Vec<String>,
    bytes: usize,
}

type SampleKey = (usize, u32, u32, u8, Vec<i16>);

struct Registry<'a> {
    table: &'a [u8],
    banks: &'a [Entry],
    out: &'a Path,
    recs: Vec<SampleRec>,
    index: HashMap<SampleKey, usize>,
    errors: Vec<String>,
}

impl Registry<'_> {
    /// Resolves a tuned-sample reference to a registry index, decoding and writing it the first time.
    fn register(&mut self, font_id: usize, font: &[u8], bank1: u8, bank2: u8, t: &mut TunedRef, name: &str) {
        match self.register_inner(font_id, font, bank1, bank2, t, name) {
            Ok(i) => t.sample = Some(i),
            Err(e) => {
                let msg = format!("font {font_id} {name}: {e:#}");
                t.error = Some(format!("{e:#}"));
                self.errors.push(msg);
            }
        }
    }

    fn register_inner(&mut self, font_id: usize, font: &[u8], bank1: u8, bank2: u8, t: &TunedRef, name: &str) -> Result<usize> {
        let keyed = name.starts_with("inst");
        let hdr = parse_sample(font, t.hdr_off)?;
        if hdr.size == 0 {
            bail!("sample header at 0x{:X} has size 0", t.hdr_off);
        }
        let bank_id = match hdr.medium {
            0 => bank1,
            1 => bank2,
            m => bail!("sample medium {m} is not relocatable (AudioLoad_RelocateSample leaves it unresolved)"),
        };
        if bank_id == 0xFF {
            bail!("sample uses sample bank slot {} but the font has none", hdr.medium + 1);
        }
        let real = real_id(self.banks, bank_id as usize);
        let bank = self.banks.get(real).with_context(|| format!("sample bank {bank_id} (real {real}) not in table"))?;
        let key: SampleKey = (real, hdr.addr, hdr.size, hdr.codec, hdr.book.as_ref().map(|b| b.coefs.clone()).unwrap_or_default());
        let refname = format!("font{font_id:02}/{name}");
        if let Some(&i) = self.index.get(&key) {
            let r = &mut self.recs[i];
            r.refs.push(refname);
            if t.tuning.to_bits() != r.tuning.to_bits() {
                r.other_tunings.insert(t.tuning.to_bits());
            }
            if hdr.lp != r.hdr.lp {
                r.loop_variants += 1;
            }
            return Ok(i);
        }
        if hdr.addr.checked_add(hdr.size).is_none_or(|e| e > bank.size) {
            bail!("sample 0x{:X}+0x{:X} exceeds sample bank {real} size 0x{:X}", hdr.addr, hdr.size, bank.size);
        }
        let start = bank.addr as usize + hdr.addr as usize;
        let capacity = capacity_samples(hdr.codec, hdr.size);
        let mut errors = Vec::new();
        let mut notes = Vec::new();
        // The synthesizer plays up to the loop end. Non-looped samples often end a few samples
        // into a frame the size does not cover; the game then decodes bytes past the size (its
        // DMA reads whole frames), so we read them from the bank too.
        let n = hdr.lp.end as usize;
        let n = if n == 0 {
            errors.push(format!("loop end 0; decoding the full data ({capacity} samples)"));
            capacity
        } else if n > capacity + 16 {
            errors.push(format!("loop end {n} is more than a frame past the data ({capacity} samples); decoding the full data"));
            capacity
        } else {
            if n > capacity {
                notes.push(format!("loop end {n} lies {} samples past the data; the last frame is read past the sample size like the game's DMA", n - capacity));
            }
            n
        };
        let need = match hdr.codec {
            CODEC_ADPCM => n.div_ceil(16) * 9,
            CODEC_SMALL_ADPCM => n.div_ceil(16) * 5,
            _ => hdr.size as usize,
        }
        .max(hdr.size as usize);
        let bank_end = (bank.addr as usize + bank.size as usize).min(self.table.len());
        let data = self.table.get(start..(start + need).min(bank_end)).context("sample outside Audiotable")?;
        let (pcm, errs) = decode_sample(hdr.codec, data, n, hdr.book.as_ref());
        for e in errs {
            // running out inside the final, partial frame at the very end of a bank is harmless
            if e.starts_with("data ends") && pcm.len() >= capacity {
                notes.push(format!("{e} (the bank ends inside the last partial frame)"));
            } else {
                errors.push(e);
            }
        }
        if hdr.lp.count != 0 && (hdr.lp.start >= hdr.lp.end || hdr.lp.end as usize > pcm.len()) {
            errors.push(format!("loop {}..{} outside decoded length {}", hdr.lp.start, hdr.lp.end, pcm.len()));
        }
        // On a loop restart the synthesizer takes the frame holding the loop start from the
        // loop's predictor state (aSetLoop) and decodes on from the next frame, so the state is
        // that frame's decoded output. Comparing it to ours checks the decoder bit for bit.
        let loop_state_diff = hdr.lp.state.and_then(|st| {
            let fs = (hdr.lp.start as usize) & !15;
            if fs + 16 > pcm.len() {
                return None;
            }
            Some((0..16).map(|i| (st[i] as i32 - pcm[fs + i] as i32).abs()).max().unwrap_or(0))
        });
        let sum2: f64 = pcm.iter().map(|&s| (s as f64) * (s as f64)).sum();
        let rms = if pcm.is_empty() { 0.0 } else { (sum2 / pcm.len() as f64).sqrt() / 32768.0 };
        let peak = pcm.iter().map(|&s| (s as i32).abs()).max().unwrap_or(0);
        let clipped = pcm.iter().filter(|&&s| s == i16::MAX || s == i16::MIN).count();
        let clip_pct = if pcm.is_empty() { 0.0 } else { 100.0 * clipped as f64 / pcm.len() as f64 };
        let (rate, unity) = wav_rate(t.tuning as f64 * OUTPUT_RATE, keyed);
        let file = format!("bank_{real}/{}.wav", sanitize(&format!("f{font_id:02}_{name}")));
        let lp = (hdr.lp.count != 0 && hdr.lp.start < hdr.lp.end).then_some((hdr.lp.start, hdr.lp.end.min(pcm.len() as u32), hdr.lp.count));
        let wav = wav_bytes(&pcm, rate, unity, lp);
        let path = self.out.join("samples").join(&file);
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(&path, &wav).with_context(|| format!("writing {}", path.display()))?;
        let i = self.recs.len();
        self.recs.push(SampleRec {
            file,
            bank: real,
            tuning: t.tuning,
            rate,
            unity,
            num_samples: pcm.len(),
            capacity,
            refs: vec![refname],
            other_tunings: BTreeSet::new(),
            loop_variants: 0,
            rms,
            peak,
            clip_pct,
            loop_state_diff,
            errors,
            notes,
            bytes: wav.len(),
            hdr,
        });
        self.index.insert(key, i);
        Ok(i)
    }
}

fn tuned_json(t: &Option<TunedRef>, reg: &Registry) -> Value {
    let Some(t) = t else { return Value::Null };
    let mut v = json!({
        "header_offset": format!("0x{:X}", t.hdr_off),
        "tuning": t.tuning,
        "playback_rate": t.tuning as f64 * OUTPUT_RATE,
    });
    if let Some(i) = t.sample {
        let r = &reg.recs[i];
        // the MIDI key at which this reference plays the WAV at its own rate
        let root = 60.0 + 12.0 * (r.rate as f64 / (t.tuning as f64 * OUTPUT_RATE)).log2();
        v["wav_rate"] = json!(r.rate);
        v["wav_root_key"] = json!((root * 100.0).round() / 100.0);
        v["file"] = json!(format!("samples/{}", r.file));
        v["codec"] = json!(codec_name(r.hdr.codec));
        v["num_samples"] = json!(r.num_samples);
        if r.hdr.lp.count != 0 {
            v["loop"] = json!({"start": r.hdr.lp.start, "end": r.hdr.lp.end, "count": if r.hdr.lp.count == u32::MAX { json!("infinite") } else { json!(r.hdr.lp.count) }});
        }
    }
    if let Some(e) = &t.error {
        v["error"] = json!(e);
    }
    v
}

fn envelope_json(env: &[(i16, i16)]) -> Value {
    Value::Array(
        env.iter()
            .map(|&(d, a)| match d {
                0 => json!({"cmd": "disable"}),
                -1 => json!({"cmd": "hang"}),
                -2 => json!({"cmd": "goto", "index": a}),
                -3 => json!({"cmd": "restart"}),
                _ => json!({"delay": d, "level": a}),
            })
            .collect(),
    )
}

// ---------------------------------------------------------------------------------------------
// Decomp name tables

/// `NA_BGM_*` names (and their trailing comments) from include/sequence.h, by value.
fn sequence_names(decomp: &Path) -> BTreeMap<u32, (String, String)> {
    let mut m = BTreeMap::new();
    let Ok(text) = std::fs::read_to_string(decomp.join("include").join("sequence.h")) else { return m };
    for line in text.lines() {
        let Some(rest) = line.trim().strip_prefix("#define NA_BGM_") else { continue };
        let mut it = rest.split_whitespace();
        let (Some(name), Some(val)) = (it.next(), it.next()) else { continue };
        let Ok(v) = u32::from_str_radix(val.trim_start_matches("0x").trim_start_matches("0X"), 16) else { continue };
        let comment = line.split_once("//").map(|(_, c)| c.split_whitespace().collect::<Vec<_>>().join(" ")).unwrap_or_default();
        m.entry(v).or_insert((name.to_string(), comment));
    }
    m
}

/// Sound effect ids from include/tables/sfx/*.h (`/* 0x800 */ DEFINE_SFX(NA_SE_..., ...)`).
fn sfx_names(decomp: &Path) -> Vec<(u32, String, String)> {
    let mut v = Vec::new();
    let dir = decomp.join("include").join("tables").join("sfx");
    let Ok(rd) = std::fs::read_dir(&dir) else { return v };
    let mut files: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    files.sort();
    for f in files {
        let bank = f.file_stem().and_then(|s| s.to_str()).unwrap_or("").trim_end_matches("_table").to_string();
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        for line in text.lines() {
            let l = line.trim();
            let Some(rest) = l.strip_prefix("/* 0x") else { continue };
            let Some((id, rest)) = rest.split_once("*/") else { continue };
            let Ok(id) = u32::from_str_radix(id.trim(), 16) else { continue };
            let Some(args) = rest.trim().strip_prefix("DEFINE_SFX(") else { continue };
            let name = args.split(',').next().unwrap_or("").trim().to_string();
            v.push((id, name, bank.clone()));
        }
    }
    v.sort();
    v
}

// ---------------------------------------------------------------------------------------------
// Sequence player (for MIDI conversion)

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
enum Logical {
    Conductor,
    Melodic(u8),
    Drum,
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct CtlState {
    vol: u8,
    expr: u8,
    pan: u8,
    bend: u16, // 14-bit, 8192 = centre, range +-12 semitones
}

#[derive(Clone, Debug)]
enum EvKind {
    Tempo(u32),
    MasterVol(u16),
    Text(u8, String),
    Ctl(CtlState),
    NoteOn { key: u8, vel: u8, program: Option<u8>, ctl: CtlState },
    NoteOff { key: u8 },
}

#[derive(Clone, Debug)]
struct Ev {
    tick: u64,
    order: u8,
    seqno: u32,
    track: usize, // 0 = conductor, 1 + seq channel
    logical: Logical,
    kind: EvKind,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Instr {
    None,
    Inst(u8),
    Drums,
    Sfx,
    Wave(u8),
}

#[derive(Clone, Copy)]
struct SimSound {
    tuning: f32,
    len: u32,
    rec: usize,
}

struct SimInst {
    lo: u8,
    hi: u8,
    low: Option<SimSound>,
    normal: Option<SimSound>,
    high: Option<SimSound>,
}

#[derive(Default)]
struct SimFont {
    insts: Vec<Option<SimInst>>,
    drums: Vec<Option<(Option<SimSound>, u8)>>,
    sfx: Vec<Option<SimSound>>,
}

#[derive(Clone, Default)]
struct Script {
    pc: usize,
    stack: [usize; 4],
    rem: [u8; 4],
    inf: [bool; 4],
    depth: usize,
    value: i8,
}

#[derive(Clone)]
struct Layer {
    enabled: bool,
    finished: bool,
    script: Script,
    delay: u16,
    gate_delay: u16,
    gate_time: u8,
    last_delay: u16,
    short_delay: u16,
    velocity: u8,
    pan: u8,
    transposition: i8,
    instr: Option<Instr>,
    stop_something: bool,
    ignore_drum_pan: bool,
    note: Option<(Logical, u8)>,
}

impl Layer {
    fn new() -> Layer {
        Layer {
            enabled: true,
            finished: false,
            script: Script::default(),
            delay: 0,
            gate_delay: 0,
            gate_time: 0x80,
            last_delay: 0,
            short_delay: 0,
            velocity: 0x7F,
            pan: 0x40,
            transposition: 0,
            instr: None,
            stop_something: false,
            ignore_drum_pan: false,
            note: None,
        }
    }
}

struct Chan {
    enabled: bool,
    stop_script: bool,
    script: Script,
    delay: u16,
    font: usize,
    instr: Instr,
    transposition: i8,
    large_notes: bool,
    mute_behavior: u8,
    vol: u8,
    expr: u8,
    pan: u8,
    pan_weight: u8,
    bend: u16,
    io: [i8; 8],
    dyn_table: usize,
    unk22: u16,
    layers: [Option<Layer>; 4],
    visited: Vec<bool>,
    starts: Vec<usize>,
    looped: bool,
    loop_tick: Option<u64>,
    used: bool,
}

impl Chan {
    fn new(len: usize, font: usize) -> Chan {
        Chan {
            enabled: false,
            stop_script: false,
            script: Script::default(),
            delay: 0,
            font,
            instr: Instr::None,
            transposition: 0,
            large_notes: false,
            mute_behavior: 0,
            vol: 127,
            expr: 127,
            pan: 0x40,
            pan_weight: 0x80,
            bend: 8192,
            io: [-1; 8],
            dyn_table: 0,
            unk22: 0,
            layers: [None, None, None, None],
            visited: vec![false; len],
            starts: Vec::new(),
            looped: false,
            loop_tick: None,
            used: false,
        }
    }
    fn logical(&self, ci: usize) -> Logical {
        if self.instr == Instr::Drums {
            Logical::Drum
        } else {
            Logical::Melodic(ci as u8)
        }
    }
    fn ctl(&self) -> CtlState {
        // The game scales amplitude linearly by volume/127; GM players map CC7/CC11 to
        // amplitude (v/127)^2, so send sqrt to keep loudness (velocity is already squared in-game).
        let sq = |v: u8| ((v.min(127) as f64 / 127.0).sqrt() * 127.0).round() as u8;
        CtlState { vol: sq(self.vol), expr: sq(self.expr), pan: self.pan.min(127), bend: self.bend }
    }
}

/// sSeqInstructionArgsTable: argument count/types of channel commands 0xB0..=0xFF.
const ARGS_TABLE: [u8; 80] = {
    const fn a1(s16: bool) -> u8 {
        ((s16 as u8) << 7) | 1
    }
    const fn a2(s0: bool, s1: bool) -> u8 {
        ((s0 as u8) << 7) | ((s1 as u8) << 6) | 2
    }
    const U8: bool = false;
    const S16: bool = true;
    const A3: u8 = 3; // (u8, u8, u8)
    [
        a1(S16), 0, a1(S16), a1(U8), 0, 0, 0, a1(S16), a1(U8), a1(U8), a1(U8), a2(U8, S16), a1(S16), a2(S16, S16), 0, 0, // B0
        0, a1(U8), a1(S16), 0, 0, 0, a1(U8), a2(U8, S16), a1(U8), a1(U8), a1(U8), a1(S16), a1(U8), a1(U8), a1(S16), a1(S16), // C0
        a1(U8), a1(U8), a1(U8), a1(U8), a1(U8), a1(U8), a1(U8), a1(U8), a1(U8), a1(U8), a1(S16), a1(U8), a1(U8), a1(U8), a1(S16), a1(U8), // D0
        a1(U8), A3, A3, a1(U8), 0, a1(U8), a1(U8), a1(S16), A3, a1(U8), 0, a2(U8, U8), 0, a1(U8), a1(U8), a2(S16, U8), // E0
        0, a1(U8), a1(U8), a1(U8), a1(U8), a1(S16), 0, 0, a1(U8), a1(S16), a1(S16), a1(S16), a1(S16), 0, 0, 0, // F0
    ]
};

fn rd8(d: &[u8], s: &mut Script) -> u8 {
    let v = d.get(s.pc).copied().unwrap_or(0xFF);
    s.pc += 1;
    v
}
fn rd16(d: &[u8], s: &mut Script) -> u16 {
    let hi = rd8(d, s) as u16;
    (hi << 8) | rd8(d, s) as u16
}
fn rdvar(d: &[u8], s: &mut Script) -> u16 {
    let mut v = rd8(d, s) as u16;
    if v & 0x80 != 0 {
        v = ((v << 8) & 0x7F00) | rd8(d, s) as u16;
    }
    v
}
fn be16_at(d: &[u8], o: usize) -> u16 {
    ((d.get(o).copied().unwrap_or(0) as u16) << 8) | d.get(o + 1).copied().unwrap_or(0) as u16
}

/// Control-flow argument (AudioSeq_GetScriptControlFlowArgument).
fn cf_arg(d: &[u8], s: &mut Script, cmd: u8) -> u16 {
    let hb = ARGS_TABLE[(cmd - 0xB0) as usize];
    if hb & 3 == 1 {
        if hb & 0x80 == 0 {
            rd8(d, s) as u16
        } else {
            rd16(d, s)
        }
    } else {
        0
    }
}

enum Flow {
    Cont,
    Delay(u16),
    End,
}

/// AudioSeq_HandleScriptFlowControl, plus loop detection: a jump to an already-executed
/// command (or the end of a 256-iteration counted loop) marks the script as looped.
fn flow(d: &[u8], s: &mut Script, cmd: u8, arg: u16, visited: Option<&[bool]>, looped: &mut bool) -> Flow {
    let mut jump = |s: &mut Script, target: usize| {
        if visited.is_some_and(|v| v.get(target).copied().unwrap_or(false)) {
            *looped = true;
        }
        s.pc = target;
    };
    match cmd {
        0xFF => {
            if s.depth == 0 {
                return Flow::End;
            }
            s.depth -= 1;
            s.pc = s.stack[s.depth];
        }
        0xFD => return Flow::Delay(rdvar(d, s)),
        0xFE => return Flow::Delay(1),
        0xFC => {
            if s.depth >= 4 {
                return Flow::End;
            }
            s.stack[s.depth] = s.pc;
            s.depth += 1;
            s.pc = arg as usize;
        }
        0xF8 => {
            if s.depth >= 4 {
                return Flow::End;
            }
            s.rem[s.depth] = arg as u8;
            s.inf[s.depth] = arg as u8 == 0;
            s.stack[s.depth] = s.pc;
            s.depth += 1;
        }
        0xF7 => {
            if s.depth == 0 {
                return Flow::End;
            }
            let i = s.depth - 1;
            s.rem[i] = s.rem[i].wrapping_sub(1);
            if s.rem[i] != 0 {
                if s.inf[i] {
                    *looped = true;
                }
                s.pc = s.stack[i];
            } else {
                s.depth -= 1;
            }
        }
        0xF6 => {
            if s.depth == 0 {
                return Flow::End;
            }
            s.depth -= 1;
        }
        0xF5 | 0xF9 | 0xFA | 0xFB => {
            let skip = (cmd == 0xFA && s.value != 0) || (cmd == 0xF9 && s.value >= 0) || (cmd == 0xF5 && s.value < 0);
            if !skip {
                jump(s, arg as usize);
            }
        }
        0xF2..=0xF4 => {
            let skip = (cmd == 0xF3 && s.value != 0) || (cmd == 0xF2 && s.value >= 0);
            if !skip {
                let t = (s.pc as isize + (arg as u8 as i8) as isize).max(0) as usize;
                jump(s, t);
            }
        }
        _ => {}
    }
    Flow::Cont
}

/// gPitchFrequencies index -> MIDI key. Index 0 is A0 (MIDI 21); indices 0x75..0x7F wrap to
/// Bb-1..Ab0 in the game's table.
fn semitone_to_key(s: u8) -> u8 {
    if s < 0x75 {
        (s as u32 + 21).min(127) as u8
    } else {
        s - 0x75 + 10
    }
}

fn pitch_freq(s: u8) -> f64 {
    let key = semitone_to_key(s) as f64;
    2f64.powf((key - 60.0) / 12.0)
}

fn bend_from_semis(semis: f64) -> u16 {
    (8192.0 + semis / 12.0 * 8192.0).round().clamp(0.0, 16383.0) as u16
}

struct Sim<'a> {
    data: Vec<u8>,
    fonts: &'a [SimFont],
    font_alias: &'a [usize],
    seq_font_raw: Vec<u8>,
    all_seqs: &'a [Vec<u8>],
    default_font: usize,
    tick: u64,
    seconds: f64,
    tempo: u16,
    tempo_add: i16,
    transposition: i16,
    delay: u16,
    script: Script,
    seq_visited: Vec<bool>,
    seq_looped: bool,
    seq_loop_tick: Option<u64>,
    /// event count when a script first looped during the current tick
    tick_cut: Option<usize>,
    finished: bool,
    frozen: bool,
    muted: bool,
    mute_behavior: u8,
    io: [i8; 8],
    short_vel: Option<usize>,
    short_gate: Option<usize>,
    channels: Vec<Chan>,
    rng: u32,
    events: Vec<Ev>,
    seqno: u32,
    warnings: BTreeMap<String, u32>,
    chain: Option<u8>,
    notes: u64,
    /// What notes played: "font00/inst003" style sources, and sample registry indices
    played_src: BTreeSet<String>,
    played_samples: BTreeSet<usize>,
    chan_used: Vec<BTreeSet<String>>,
    trace: bool,
}

const DEFAULT_VEL: [u8; 16] = [12, 25, 38, 51, 57, 64, 71, 76, 83, 89, 96, 102, 109, 115, 121, 127];
const DEFAULT_GATE: [u8; 16] = [229, 203, 177, 151, 139, 126, 113, 100, 87, 74, 61, 48, 36, 23, 10, 0];

impl<'a> Sim<'a> {
    fn new(data: Vec<u8>, fonts: &'a [SimFont], font_alias: &'a [usize], seq_font_raw: Vec<u8>, all_seqs: &'a [Vec<u8>]) -> Sim<'a> {
        // AudioLoad_SyncInitSeqPlayerInternal: the default font is the last one in the list
        let n = seq_font_raw.first().copied().unwrap_or(0) as usize;
        let default_font = if n > 0 { seq_font_raw.get(n).copied().unwrap_or(0xFF) as usize } else { 0xFF };
        let default_font = if default_font != 0xFF { font_alias.get(default_font).copied().unwrap_or(default_font) } else { 0xFF };
        let len = data.len();
        let mut s = Sim {
            channels: (0..16).map(|_| Chan::new(len, default_font)).collect(),
            data,
            fonts,
            font_alias,
            seq_font_raw,
            all_seqs,
            default_font,
            tick: 0,
            seconds: 0.0,
            tempo: 120 * TATUMS_PER_BEAT as u16,
            tempo_add: 0,
            transposition: 0,
            delay: 0,
            script: Script::default(),
            seq_visited: vec![false; len],
            seq_looped: false,
            seq_loop_tick: None,
            tick_cut: None,
            finished: false,
            frozen: false,
            muted: false,
            mute_behavior: 0x20 | 0x40,
            io: [-1; 8],
            short_vel: None,
            short_gate: None,
            rng: 0x1234_5678,
            events: Vec::new(),
            seqno: 0,
            warnings: BTreeMap::new(),
            chain: None,
            notes: 0,
            played_src: BTreeSet::new(),
            played_samples: BTreeSet::new(),
            chan_used: vec![BTreeSet::new(); 16],
            trace: false,
        };
        s.emit_tempo();
        s
    }

    fn mark(&mut self, ci: usize, src: String) {
        self.chan_used[ci].insert(src.clone());
        self.played_src.insert(src);
    }

    fn warn(&mut self, w: impl Into<String>) {
        *self.warnings.entry(w.into()).or_default() += 1;
    }

    fn random(&mut self) -> u32 {
        self.rng = self.rng.wrapping_mul(1_103_515_245).wrapping_add(12345);
        self.rng >> 8
    }

    fn push(&mut self, track: usize, logical: Logical, kind: EvKind) {
        let order = match kind {
            EvKind::NoteOff { .. } => 0,
            EvKind::NoteOn { .. } => 2,
            _ => 1,
        };
        self.seqno += 1;
        self.events.push(Ev { tick: self.tick, order, seqno: self.seqno, track, logical, kind });
    }

    fn eff_tempo(&self) -> f64 {
        (self.tempo as i32 + self.tempo_add as i32).max(1) as f64
    }

    fn emit_tempo(&mut self) {
        let us = (60_000_000.0 * TATUMS_PER_BEAT as f64 / self.eff_tempo()).round() as u32;
        self.push(0, Logical::Conductor, EvKind::Tempo(us.min(0xFF_FFFF)));
    }

    fn emit_ctl(&mut self, ci: usize) {
        let c = &self.channels[ci];
        let (l, s) = (c.logical(ci), c.ctl());
        self.push(1 + ci, l, EvKind::Ctl(s));
    }

    fn resolve_font(&self, id: usize) -> Option<usize> {
        let r = self.font_alias.get(id).copied()?;
        (r < self.fonts.len()).then_some(r)
    }

    /// Picks a font from this sequence's font list (channel commands C6/EB).
    fn seq_font(&self, cmd: u8) -> u8 {
        if self.default_font == 0xFF {
            return cmd;
        }
        let n = self.seq_font_raw.first().copied().unwrap_or(0) as usize;
        self.seq_font_raw.get(n.wrapping_sub(cmd as usize)).copied().unwrap_or(0xFF)
    }

    fn inst<'s>(&'s self, font: usize, id: u8) -> Option<&'s SimInst> {
        self.fonts.get(font)?.insts.get(id as usize)?.as_ref()
    }

    fn set_instrument(&mut self, ci: usize, id: u8) {
        let font = self.channels[ci].font;
        let i = match id {
            0x80..=0xFF => Instr::Wave(id),
            0x7F => Instr::Drums,
            0x7E => Instr::Sfx,
            _ if self.inst(font, id).is_some() => Instr::Inst(id),
            _ => {
                self.warn(format!("missing instrument {id} in font {font}"));
                Instr::None
            }
        };
        self.channels[ci].instr = i;
    }

    fn note_off(&mut self, ci: usize, l: &mut Layer) {
        if let Some((lg, key)) = l.note.take() {
            self.push(1 + ci, lg, EvKind::NoteOff { key });
        }
    }

    fn free_layer(&mut self, ci: usize, li: usize) {
        if let Some(mut l) = self.channels[ci].layers[li].take() {
            self.note_off(ci, &mut l);
        }
    }

    fn disable_channel(&mut self, ci: usize) {
        for li in 0..4 {
            self.free_layer(ci, li);
        }
        self.channels[ci].enabled = false;
    }

    fn enable_channel(&mut self, ci: usize, pc: usize) {
        for li in 0..4 {
            self.free_layer(ci, li);
        }
        let c = &mut self.channels[ci];
        c.enabled = true;
        c.used = true;
        c.script = Script { pc, ..Default::default() };
        c.delay = 0;
        if c.starts.contains(&pc) {
            if !c.looped && self.tick_cut.is_none() {
                self.tick_cut = Some(self.events.len());
            }
            c.looped = true;
            c.loop_tick.get_or_insert(self.tick);
        } else {
            c.starts.push(pc);
        }
    }

    fn set_layer(&mut self, ci: usize, li: usize, pc: usize) {
        let mut l = Layer::new();
        if let Some(mut old) = self.channels[ci].layers[li].take() {
            self.note_off(ci, &mut old);
            // fields AudioSeq_SeqChannelSetLayer does not reset
            l.velocity = old.velocity;
            l.last_delay = old.last_delay;
            l.short_delay = old.short_delay;
        }
        l.script.pc = pc;
        self.channels[ci].layers[li] = Some(l);
    }

    /// One tick of AudioSeq_SequencePlayerProcessSequence.
    fn tick_seq(&mut self) {
        if self.finished || self.frozen {
            return;
        }
        if self.delay > 1 {
            self.delay -= 1;
        } else {
            let mut guard = 0;
            loop {
                guard += 1;
                if guard > 20000 {
                    self.warn("sequence script: too many commands in one tick");
                    self.finished = true;
                    return;
                }
                let pc = self.script.pc;
                if let Some(v) = self.seq_visited.get_mut(pc) {
                    *v = true;
                } else {
                    self.warn("sequence script ran off the end of the data");
                }
                let cmd = rd8(&self.data, &mut self.script);
                if self.trace {
                    eprintln!("{:6} seq  {pc:04X}: {cmd:02X} value={}", self.tick, self.script.value);
                }
                if cmd >= 0xF2 {
                    let arg = cf_arg(&self.data, &mut self.script, cmd);
                    let mut looped = false;
                    let r = flow(&self.data, &mut self.script, cmd, arg, Some(&self.seq_visited), &mut looped);
                    if looped && !self.seq_looped && self.tick_cut.is_none() {
                        self.tick_cut = Some(self.events.len());
                    }
                    self.seq_looped |= looped;
                    if looped {
                        self.seq_loop_tick.get_or_insert(self.tick);
                    }
                    match r {
                        Flow::Cont | Flow::Delay(0) => continue,
                        Flow::Delay(n) => {
                            self.delay = n;
                            break;
                        }
                        Flow::End => {
                            for ci in 0..16 {
                                self.disable_channel(ci);
                            }
                            self.finished = true;
                            return;
                        }
                    }
                }
                let d = &self.data;
                let s = &mut self.script;
                if cmd >= 0xC0 {
                    match cmd {
                        0xF1 | 0xD0 | 0xD5 | 0xD9 => {
                            rd8(d, s);
                        }
                        0xDF | 0xDE => {
                            if cmd == 0xDF {
                                self.transposition = 0;
                            }
                            self.transposition += rd8(d, s) as i8 as i16;
                        }
                        0xDD => {
                            let t = (rd8(d, s) as u16 * TATUMS_PER_BEAT as u16).max(1);
                            self.tempo = t;
                            self.emit_tempo();
                        }
                        0xDC => {
                            self.tempo_add = rd8(d, s) as i8 as i16 * TATUMS_PER_BEAT as i16;
                            self.emit_tempo();
                        }
                        0xDA => {
                            rd8(d, s);
                            rd16(d, s);
                        }
                        0xDB => {
                            let v = rd8(d, s);
                            let mv = ((v.min(127) as u32 * 16383) / 127) as u16;
                            self.push(0, Logical::Conductor, EvKind::MasterVol(mv));
                        }
                        0xD7 => {
                            let bits = rd16(d, s);
                            for i in 0..16 {
                                if bits & (1 << i) != 0 {
                                    self.channels[i].font = self.default_font;
                                    self.channels[i].mute_behavior = self.mute_behavior;
                                }
                            }
                        }
                        0xD6 | 0xC5 => {
                            rd16(d, s);
                        }
                        0xD4 => self.muted = true,
                        0xD3 => self.mute_behavior = rd8(d, s),
                        0xD1 | 0xD2 => {
                            let t = rd16(d, s) as usize;
                            if cmd == 0xD2 {
                                self.short_vel = Some(t);
                            } else {
                                self.short_gate = Some(t);
                            }
                        }
                        0xCE => {
                            let c = rd8(d, s);
                            let r = self.random() >> 2;
                            self.script.value = if c == 0 { (r & 0xFF) as u8 as i8 } else { (r % c as u32) as u8 as i8 };
                        }
                        0xCD => {
                            let t = rd16(d, s) as usize;
                            if s.value != -1 && s.depth != 3 && s.depth < 4 {
                                let o = t.wrapping_add((s.value as isize * 2) as usize);
                                s.stack[s.depth] = s.pc;
                                s.depth += 1;
                                s.pc = be16_at(d, o) as usize;
                            }
                        }
                        0xCC => s.value = rd8(d, s) as i8,
                        0xC9 => s.value &= rd8(d, s) as i8,
                        0xC8 => s.value = s.value.wrapping_sub(rd8(d, s) as i8),
                        0xC7 => {
                            let c = rd8(d, s);
                            let t = rd16(d, s) as usize;
                            let v = (s.value as u8).wrapping_add(c);
                            if let Some(b) = self.data.get_mut(t) {
                                *b = v;
                            }
                        }
                        0xC6 => {
                            self.frozen = true;
                            return;
                        }
                        0xEF => {
                            rd16(d, s);
                            rd8(d, s);
                        }
                        0xC4 => {
                            let player = rd8(d, s);
                            let seq = rd8(d, s);
                            if player == 0xFF || player == 0 {
                                self.chain = Some(seq);
                                self.finished = true;
                                return;
                            }
                        }
                        _ => {}
                    }
                    continue;
                }
                let lo = (cmd & 0xF) as usize;
                match cmd & 0xF0 {
                    0x00 => self.script.value = (!self.channels[lo].enabled) as i8,
                    0x50 => {
                        let v = self.io[lo & 7];
                        self.script.value = self.script.value.wrapping_sub(v);
                    }
                    0x70 => self.io[lo & 7] = self.script.value,
                    0x80 => {
                        self.script.value = self.io[lo & 7];
                        if lo < 2 {
                            self.io[lo] = -1;
                        }
                    }
                    0x40 => self.disable_channel(lo),
                    0x90 => {
                        let t = rd16(&self.data, &mut self.script) as usize;
                        self.enable_channel(lo, t);
                    }
                    0xA0 => {
                        let rel = rd16(&self.data, &mut self.script) as i16;
                        let t = (self.script.pc as isize + rel as isize).max(0) as usize;
                        self.enable_channel(lo, t);
                    }
                    0xB0 => {
                        let id = rd8(&self.data, &mut self.script) as usize;
                        let t = rd16(&self.data, &mut self.script) as usize;
                        // AudioLoad_SlowLoadSeq: copy another sequence into this one's data
                        if let Some(src) = self.all_seqs.get(id).filter(|v| !v.is_empty()) {
                            let src = src.clone();
                            if self.data.len() < t + src.len() {
                                self.data.resize(t + src.len(), 0);
                                self.seq_visited.resize(self.data.len(), false);
                                for c in &mut self.channels {
                                    c.visited.resize(self.data.len(), false);
                                }
                            }
                            self.data[t..t + src.len()].copy_from_slice(&src);
                        }
                        self.io[lo & 7] = 0;
                    }
                    0x60 => {
                        rd8(&self.data, &mut self.script);
                        rd8(&self.data, &mut self.script);
                        self.io[lo & 7] = 0; // script load finished
                    }
                    _ => {}
                }
            }
        }
        for ci in 0..16 {
            if self.channels[ci].enabled {
                self.tick_channel(ci);
            }
        }
    }

    fn tick_channel(&mut self, ci: usize) {
        if self.muted && self.channels[ci].mute_behavior & 0x80 != 0 {
            return;
        }
        if !self.channels[ci].stop_script {
            if self.channels[ci].delay >= 2 {
                self.channels[ci].delay -= 1;
            } else {
                self.run_channel(ci);
            }
        }
        for li in 0..4 {
            if let Some(mut l) = self.channels[ci].layers[li].take() {
                self.tick_layer(ci, &mut l);
                if self.channels[ci].layers[li].is_none() {
                    self.channels[ci].layers[li] = Some(l);
                }
            }
        }
    }

    fn run_channel(&mut self, ci: usize) {
        let mut guard = 0;
        loop {
            guard += 1;
            if guard > 20000 {
                self.warn("channel script: too many commands in one tick");
                self.disable_channel(ci);
                return;
            }
            let c = &mut self.channels[ci];
            let pc = c.script.pc;
            if let Some(v) = c.visited.get_mut(pc) {
                *v = true;
            }
            let cmd = rd8(&self.data, &mut c.script);
            if self.trace {
                eprintln!("{:6} ch{ci:<2} {pc:04X}: {cmd:02X} value={}", self.tick, c.script.value);
            }
            if cmd >= 0xB0 {
                let mut hb = ARGS_TABLE[(cmd - 0xB0) as usize];
                let n = hb & 3;
                let mut args = [0u16; 3];
                if cmd < 0xF2 {
                    for a in args.iter_mut().take(n as usize) {
                        *a = if hb & 0x80 == 0 { rd8(&self.data, &mut c.script) as u16 } else { rd16(&self.data, &mut c.script) };
                        hb <<= 1;
                    }
                } else {
                    args[0] = cf_arg(&self.data, &mut c.script, cmd);
                    let mut looped = false;
                    let r = flow(&self.data, &mut c.script, cmd, args[0], Some(&c.visited), &mut looped);
                    if looped && !c.looped && self.tick_cut.is_none() {
                        self.tick_cut = Some(self.events.len());
                    }
                    c.looped |= looped;
                    if looped {
                        c.loop_tick.get_or_insert(self.tick);
                    }
                    match r {
                        Flow::Cont | Flow::Delay(0) => continue,
                        Flow::Delay(n) => {
                            c.delay = n;
                            return;
                        }
                        Flow::End => {
                            self.disable_channel(ci);
                            return;
                        }
                    }
                }
                let a0 = args[0];
                let (a0u8, a0s8) = (a0 as u8, a0 as u8 as i8);
                match cmd {
                    0xEA => {
                        c.stop_script = true;
                        return;
                    }
                    0xC2 => c.dyn_table = a0 as usize,
                    0xC5 => {
                        if c.script.value != -1 {
                            let o = c.dyn_table.wrapping_add((c.script.value as isize * 2) as usize);
                            c.dyn_table = be16_at(&self.data, o) as usize;
                        }
                    }
                    0xEB | 0xC6 => {
                        let f = self.seq_font(a0u8) as usize;
                        match self.resolve_font(f) {
                            Some(r) => self.channels[ci].font = r,
                            None => self.warn(format!("channel selects unknown font {f}")),
                        }
                        if cmd == 0xEB {
                            self.set_instrument(ci, args[1] as u8);
                        }
                    }
                    0xC1 => self.set_instrument(ci, a0u8),
                    0xC3 => c.large_notes = false,
                    0xC4 => c.large_notes = true,
                    0xDF => {
                        c.vol = a0u8;
                        self.emit_ctl(ci);
                    }
                    0xE0 => {
                        c.expr = a0u8.min(127);
                        self.emit_ctl(ci);
                    }
                    0xDE => {
                        let f = (a0 as f64 / 32768.0).max(1e-6);
                        c.bend = bend_from_semis(12.0 * f.log2());
                        self.emit_ctl(ci);
                    }
                    0xD3 => {
                        c.bend = bend_from_semis(a0s8 as f64 / 127.0 * 12.0);
                        self.emit_ctl(ci);
                    }
                    0xEE => {
                        c.bend = bend_from_semis(a0s8 as f64 / 127.0 * 2.0);
                        self.emit_ctl(ci);
                    }
                    0xDD => {
                        c.pan = a0u8;
                        self.emit_ctl(ci);
                    }
                    0xDC => c.pan_weight = a0u8,
                    0xDB => c.transposition = a0s8,
                    0xC7 => {
                        let v = (c.script.value as u8).wrapping_add(a0u8);
                        if let Some(b) = self.data.get_mut(args[1] as usize) {
                            *b = v;
                        }
                    }
                    0xC8 => c.script.value = c.script.value.wrapping_sub(a0s8),
                    0xCC => c.script.value = a0s8,
                    0xC9 => c.script.value &= a0s8,
                    0xCD => {
                        if (a0u8 as usize) < 16 {
                            self.disable_channel(a0u8 as usize);
                        }
                    }
                    0xCA => c.mute_behavior = a0u8,
                    0xCB => {
                        let o = (a0 as usize).wrapping_add(c.script.value as isize as usize);
                        c.script.value = self.data.get(o).copied().unwrap_or(0) as i8;
                    }
                    0xCE => c.unk22 = a0,
                    0xCF => {
                        let (hi, lo) = ((c.unk22 >> 8) as u8, c.unk22 as u8);
                        let o = a0 as usize;
                        if o + 1 < self.data.len() {
                            self.data[o] = hi;
                            self.data[o + 1] = lo;
                        }
                    }
                    0xE4 => {
                        if c.script.value != -1 {
                            if c.script.value < 0 || c.script.depth >= 4 {
                                self.warn("channel dyncall with bad index/stack");
                            } else {
                                let o = c.dyn_table + c.script.value as usize * 2;
                                let t = be16_at(&self.data, o) as usize;
                                c.script.stack[c.script.depth] = c.script.pc;
                                c.script.depth += 1;
                                c.script.pc = t;
                            }
                        }
                    }
                    0xE7 => {
                        let o = a0 as usize;
                        let b = |i: usize| self.data.get(o + i).copied().unwrap_or(0);
                        c.mute_behavior = b(0);
                        c.transposition = b(3) as i8;
                        c.pan = b(4);
                        c.pan_weight = b(5);
                        self.emit_ctl(ci);
                    }
                    0xE8 => {
                        c.mute_behavior = a0u8;
                        c.transposition = rd8(&self.data, &mut c.script) as i8;
                        c.pan = rd8(&self.data, &mut c.script);
                        c.pan_weight = rd8(&self.data, &mut c.script);
                        rd8(&self.data, &mut c.script); // reverb
                        rd8(&self.data, &mut c.script); // reverb index
                        self.emit_ctl(ci);
                    }
                    0xEC => {
                        c.bend = 8192;
                        self.emit_ctl(ci);
                    }
                    0xB2 => {
                        let o = (a0 as usize).wrapping_add((c.script.value as isize * 2) as usize);
                        c.unk22 = be16_at(&self.data, o);
                    }
                    0xB4 => c.dyn_table = c.unk22 as usize,
                    0xB5 => {
                        let o = c.dyn_table.wrapping_add((c.script.value as isize * 2) as usize);
                        c.unk22 = be16_at(&self.data, o);
                    }
                    0xB6 => {
                        let o = c.dyn_table.wrapping_add(c.script.value as isize as usize);
                        c.script.value = self.data.get(o).copied().unwrap_or(0) as i8;
                    }
                    0xB7 | 0xB8 | 0xBD => {
                        let r = self.random();
                        let c = &mut self.channels[ci];
                        let v = if a0 == 0 { r & 0xFFFF } else { r % a0 as u32 };
                        match cmd {
                            0xB7 => c.unk22 = v as u16,
                            0xB8 => c.script.value = v as u8 as i8,
                            _ => {
                                let u = (v as u16).wrapping_add(args[1]);
                                c.unk22 = ((((u / 0x100) + 0x80) & 0xFF) << 8) | (u % 0x100);
                            }
                        }
                    }
                    0xBC => c.unk22 = c.unk22.wrapping_add(a0),
                    _ => {}
                }
                continue;
            }
            if cmd >= 0x70 {
                let mut lo = (cmd & 7) as usize;
                if cmd & 0xF8 != 0x70 && lo >= 4 {
                    lo = 0;
                }
                match cmd & 0xF8 {
                    0x80 => {
                        c.script.value = match &c.layers[lo] {
                            Some(l) => l.finished as i8,
                            None => -1,
                        }
                    }
                    0x88 => {
                        let t = rd16(&self.data, &mut c.script) as usize;
                        self.set_layer(ci, lo, t);
                    }
                    0x90 => self.free_layer(ci, lo),
                    0x98 => {
                        if c.script.value != -1 {
                            let o = c.dyn_table.wrapping_add((c.script.value as isize * 2) as usize);
                            let t = be16_at(&self.data, o) as usize;
                            self.set_layer(ci, lo, t);
                        }
                    }
                    0x70 => c.io[lo] = c.script.value,
                    0x78 => {
                        let rel = rd16(&self.data, &mut c.script) as i16;
                        let t = (c.script.pc as isize + rel as isize).max(0) as usize;
                        self.set_layer(ci, lo, t);
                    }
                    _ => {}
                }
                continue;
            }
            let lo = (cmd & 0xF) as usize;
            match cmd & 0xF0 {
                0x00 => {
                    c.delay = lo as u16;
                    return;
                }
                0x10 => c.io[lo & 7] = 0, // slow sample load: treat as finished
                0x60 => {
                    c.script.value = if lo < 8 { c.io[lo] } else { -1 };
                    if lo < 2 {
                        c.io[lo] = -1;
                    }
                }
                0x50 => {
                    let v = if lo < 8 { c.io[lo] } else { -1 };
                    c.script.value = c.script.value.wrapping_sub(v);
                }
                0x20 => {
                    let t = rd16(&self.data, &mut c.script) as usize;
                    self.enable_channel(lo, t);
                }
                0x30 => {
                    let port = rd8(&self.data, &mut c.script) as usize & 7;
                    let v = c.script.value;
                    self.channels[lo].io[port] = v;
                }
                0x40 => {
                    let port = rd8(&self.data, &mut c.script) as usize & 7;
                    let v = self.channels[lo].io[port];
                    self.channels[ci].script.value = v;
                }
                _ => {}
            }
        }
    }

    /// AudioSeq_SeqLayerProcessScript (steps 1-5).
    fn tick_layer(&mut self, ci: usize, l: &mut Layer) {
        if !l.enabled {
            return;
        }
        if l.delay > 1 {
            l.delay -= 1;
            if !l.stop_something && l.delay <= l.gate_delay {
                self.note_off(ci, l);
                l.stop_something = true;
            }
            return;
        }
        // step 1: the previous note ends (continuous notes are also re-struck in MIDI)
        self.note_off(ci, l);

        // step 2
        let mut guard = 0;
        let cmd = loop {
            guard += 1;
            if guard > 20000 {
                self.warn("layer script: too many commands in one tick");
                l.enabled = false;
                l.finished = true;
                return;
            }
            let pc = l.script.pc;
            let cmd = rd8(&self.data, &mut l.script);
            if self.trace {
                eprintln!("{:6} ch{ci:<2} layer {pc:04X}: {cmd:02X}", self.tick);
            }
            if cmd <= 0xC0 {
                break cmd;
            }
            if cmd >= 0xF2 {
                let arg = cf_arg(&self.data, &mut l.script, cmd);
                let mut looped = false;
                match flow(&self.data, &mut l.script, cmd, arg, None, &mut looped) {
                    Flow::Cont => continue,
                    _ => {
                        l.enabled = false;
                        l.finished = true;
                        return;
                    }
                }
            }
            let d = &self.data;
            match cmd {
                0xC1 => l.velocity = rd8(d, &mut l.script),
                0xCA => l.pan = rd8(d, &mut l.script),
                0xC9 => l.gate_time = rd8(d, &mut l.script),
                0xC2 => l.transposition = rd8(d, &mut l.script) as i8,
                0xC4 | 0xC5 => {}
                0xC3 => l.short_delay = rdvar(d, &mut l.script),
                0xC6 => {
                    let id = rd8(d, &mut l.script);
                    l.instr = match id {
                        0xFF => None,
                        0x80..=0xFE => Some(Instr::Wave(id)),
                        0x7F => Some(Instr::Drums),
                        0x7E => Some(Instr::Sfx),
                        _ => {
                            if self.inst(self.channels[ci].font, id).is_some() {
                                Some(Instr::Inst(id))
                            } else {
                                None
                            }
                        }
                    };
                }
                0xC7 => {
                    let mode = rd8(d, &mut l.script);
                    rd8(d, &mut l.script);
                    if mode & 0x80 != 0 {
                        rd8(d, &mut l.script);
                    } else {
                        rdvar(d, &mut l.script);
                    }
                }
                0xC8 | 0xCC => {
                    if cmd == 0xCC {
                        l.ignore_drum_pan = true;
                    }
                }
                0xCB => {
                    rd16(d, &mut l.script);
                    rd8(d, &mut l.script);
                }
                0xCF | 0xCD | 0xCE => {
                    rd8(d, &mut l.script);
                }
                _ => match cmd & 0xF0 {
                    0xD0 => {
                        let i = (cmd & 0xF) as usize;
                        l.velocity = match self.short_vel {
                            Some(o) => self.data.get(o + i).copied().unwrap_or(0x7F),
                            None => DEFAULT_VEL[i],
                        };
                    }
                    0xE0 => {
                        let i = (cmd & 0xF) as usize;
                        l.gate_time = match self.short_gate {
                            Some(o) => self.data.get(o + i).copied().unwrap_or(0),
                            None => DEFAULT_GATE[i],
                        };
                    }
                    _ => {}
                },
            }
        };

        // step 3
        let d = &self.data;
        if cmd == 0xC0 {
            l.delay = rdvar(d, &mut l.script);
            l.stop_something = true;
            return;
        }
        l.stop_something = false;
        let delay;
        if self.channels[ci].large_notes {
            match cmd & 0xC0 {
                0x00 => {
                    delay = rdvar(d, &mut l.script);
                    l.velocity = rd8(d, &mut l.script).min(0x7F);
                    l.gate_time = rd8(d, &mut l.script);
                    l.last_delay = delay;
                }
                0x40 => {
                    delay = rdvar(d, &mut l.script);
                    l.velocity = rd8(d, &mut l.script).min(0x7F);
                    l.gate_time = 0;
                    l.last_delay = delay;
                }
                _ => {
                    delay = l.last_delay;
                    l.velocity = rd8(d, &mut l.script).min(0x7F);
                    l.gate_time = rd8(d, &mut l.script);
                }
            }
        } else {
            match cmd & 0xC0 {
                0x00 => {
                    delay = rdvar(d, &mut l.script);
                    l.last_delay = delay;
                }
                0x40 => delay = l.short_delay,
                _ => delay = l.last_delay,
            }
        }
        let cmd = cmd & 0x3F;
        l.delay = delay;
        l.gate_delay = ((l.gate_time as u32 * delay as u32) >> 8) as u16;
        if self.muted && self.channels[ci].mute_behavior & (0x40 | 0x10) != 0 {
            if self.trace {
                eprintln!("{:6} ch{ci:<2} note skipped: sequence muted", self.tick);
            }
            l.stop_something = true;
            return;
        }

        // step 4
        let ch = &self.channels[ci];
        let instr = match l.instr {
            Some(i) => i,
            None => {
                if ch.instr == Instr::None {
                    self.warn("note on a channel without an instrument");
                    return;
                }
                ch.instr
            }
        };
        let font = ch.font;
        let (logical, key, program, sound, freq, pan);
        match instr {
            Instr::Drums => {
                let s = (cmd as i32 + ch.transposition as i32 + l.transposition as i32) as u8;
                let Some(Some((snd, dpan))) = self.fonts.get(font).and_then(|f| f.drums.get(s as usize)) else {
                    self.warn(format!("missing drum {s} in font {font}"));
                    l.stop_something = true;
                    return;
                };
                logical = Logical::Drum;
                self.mark(ci, format!("font{font:02}/drum{s:03}"));
                key = (s as u32 + 21).min(127) as u8;
                program = None;
                sound = *snd;
                freq = snd.map(|x| x.tuning as f64).unwrap_or(1.0);
                pan = if l.ignore_drum_pan { l.pan } else { *dpan };
            }
            Instr::Sfx => {
                let id = ((l.transposition as i32) << 6) + cmd as i32;
                let Some(Some(snd)) = self.fonts.get(font).and_then(|f| f.sfx.get(id.max(0) as usize)) else {
                    self.warn(format!("missing sfx {id} in font {font}"));
                    l.stop_something = true;
                    return;
                };
                self.mark(ci, format!("font{font:02}/sfx{id:03}"));
                logical = Logical::Melodic(ci as u8);
                key = 60;
                program = Some(126);
                sound = Some(*snd);
                freq = snd.tuning as f64;
                pan = l.pan;
            }
            Instr::Inst(_) | Instr::Wave(_) | Instr::None => {
                let s = cmd as i32 + self.transposition as i32 + ch.transposition as i32 + l.transposition as i32;
                let s = s as u8; // u8 arithmetic like the game
                if s >= 0x80 {
                    if self.trace {
                        eprintln!("{:6} ch{ci:<2} note skipped: semitone 0x{s:02X} >= 0x80", self.tick);
                    }
                    l.stop_something = true;
                    return;
                }
                logical = Logical::Melodic(ci as u8);
                key = semitone_to_key(s);
                pan = l.pan;
                match instr {
                    Instr::Inst(id) => {
                        let Some(inst) = self.inst(font, id) else {
                            self.warn(format!("missing instrument {id} in font {font}"));
                            return;
                        };
                        let snd = if s < inst.lo {
                            inst.low
                        } else if s <= inst.hi {
                            inst.normal
                        } else {
                            inst.high
                        };
                        self.mark(ci, format!("font{font:02}/inst{id:03}"));
                        program = Some(id.min(127));
                        sound = snd;
                        freq = pitch_freq(s) * snd.map(|x| x.tuning as f64).unwrap_or(1.0);
                    }
                    Instr::Wave(w) => {
                        self.mark(ci, format!("synthetic_wave_0x{w:02X}"));
                        program = Some(80 + (w & 7));
                        sound = None;
                        freq = pitch_freq(s);
                    }
                    _ => return,
                }
            }
        }
        if l.delay == 0 {
            // length of the sample at this pitch, in ticks
            let secs = sound.map(|x| x.len as f64 / (OUTPUT_RATE * freq.max(1e-6))).unwrap_or(0.0);
            let ticks = (secs * self.eff_tempo() / 60.0).min(0x7FFE as f64);
            l.gate_delay = 0;
            l.delay = ticks as u16 + 1;
        }
        if let Some(x) = sound {
            self.played_samples.insert(x.rec);
        }
        let ch = &self.channels[ci];
        let note_pan = ((ch.pan as u32 * ch.pan_weight as u32 + pan as u32 * (0x80u32.saturating_sub(ch.pan_weight as u32))) >> 7).min(127) as u8;
        let mut ctl = ch.ctl();
        ctl.pan = note_pan;
        let vel = l.velocity.clamp(1, 127);
        self.notes += 1;
        self.push(1 + ci, logical, EvKind::NoteOn { key, vel, program, ctl });
        l.note = Some((logical, key));
    }

    /// Runs the sequence until it ends or loops; `self.tick` is then the length in ticks.
    fn run(&mut self) -> &'static str {
        let reason;
        let mut marked = false;
        loop {
            self.tick_cut = None;
            self.tick_seq();
            if self.chain.is_some() {
                reason = "chains to another sequence";
                break;
            }
            if self.finished {
                reason = "sequence script ended";
                break;
            }
            if self.frozen {
                reason = "sequence script stopped (C6)";
                break;
            }
            let all = self.channels.iter().all(|c| !c.enabled || c.looped || c.stop_script);
            if self.seq_looped && all {
                // drop what the scripts did after jumping back: that is the second pass
                if let Some(cut) = self.tick_cut {
                    self.events.truncate(cut);
                }
                reason = "loop detected (every script jumped back to code it had already run)";
                break;
            }
            if self.seq_looped && !marked {
                marked = true;
                self.push(0, Logical::Conductor, EvKind::Text(6, "sequence script loops".into()));
            }
            self.tick += 1;
            self.seconds += 60.0 / self.eff_tempo();
            if self.seconds >= MIDI_MAX_SECONDS {
                reason = "length cap";
                break;
            }
        }
        self.push(0, Logical::Conductor, EvKind::Text(6, format!("end: {reason}")));
        for ci in 0..16 {
            for li in 0..4 {
                if let Some(mut l) = self.channels[ci].layers[li].take() {
                    self.note_off(ci, &mut l);
                    self.channels[ci].layers[li] = Some(l);
                }
            }
        }
        reason
    }
}

// ---------------------------------------------------------------------------------------------
// MIDI writing

fn vlq(out: &mut Vec<u8>, mut v: u32) {
    let mut buf = [0u8; 5];
    let mut n = 0;
    loop {
        buf[n] = (v & 0x7F) as u8;
        n += 1;
        v >>= 7;
        if v == 0 {
            break;
        }
    }
    for i in (0..n).rev() {
        out.push(buf[i] | if i > 0 { 0x80 } else { 0 });
    }
}

struct Track {
    data: Vec<u8>,
    last: u64,
}

impl Track {
    fn ev(&mut self, tick: u64, bytes: &[u8]) {
        vlq(&mut self.data, (tick.saturating_sub(self.last)).min(0x0FFF_FFFF) as u32);
        self.last = self.last.max(tick);
        self.data.extend_from_slice(bytes);
    }
    fn meta(&mut self, tick: u64, kind: u8, payload: &[u8]) {
        let mut b = vec![0xFF, kind];
        vlq(&mut b, payload.len() as u32);
        b.extend_from_slice(payload);
        self.ev(tick, &b);
    }
}

#[derive(Default, Clone, Copy)]
struct PhysState {
    init: bool,
    program: Option<u8>,
    vol: Option<u8>,
    expr: Option<u8>,
    pan: Option<u8>,
    bend: Option<u16>,
}

struct MidiOut {
    bytes: Vec<u8>,
    phys_of: HashMap<u8, u8>,
    tracks: usize,
    channels_used: usize,
    overflow_channels: usize,
    seconds: f64,
}

fn write_midi(events: &mut [Ev], end_tick: u64, title: &str) -> MidiOut {
    events.sort_by_key(|e| (e.tick, e.order, e.seqno));
    // MIDI channels: drums on 10 (index 9); melodic sequence channels take the other 15 in order.
    let melodic: BTreeSet<u8> = events
        .iter()
        .filter_map(|e| match (e.logical, &e.kind) {
            (Logical::Melodic(c), EvKind::NoteOn { .. }) => Some(c),
            _ => None,
        })
        .collect();
    let pool: Vec<u8> = (0..16).filter(|&c| c != 9).collect();
    let mut phys_of: HashMap<u8, u8> = HashMap::new();
    let mut overflow = 0;
    for (i, &c) in melodic.iter().enumerate() {
        if i >= pool.len() {
            overflow += 1;
        }
        phys_of.insert(c, pool[i.min(pool.len() - 1)]);
    }
    let has_drums = events.iter().any(|e| e.logical == Logical::Drum && matches!(e.kind, EvKind::NoteOn { .. }));
    let used_tracks: BTreeSet<usize> = events.iter().filter(|e| matches!(e.kind, EvKind::NoteOn { .. })).map(|e| e.track).collect();
    let mut track_of: HashMap<usize, usize> = HashMap::new();
    let mut tracks: Vec<Track> = vec![Track { data: Vec::new(), last: 0 }];
    tracks[0].meta(0, 0x03, title.as_bytes());
    for &t in &used_tracks {
        track_of.insert(t, tracks.len());
        let mut tr = Track { data: Vec::new(), last: 0 };
        tr.meta(0, 0x03, format!("seq channel {}", t - 1).as_bytes());
        tracks.push(tr);
    }
    let mut phys = [PhysState::default(); 16];
    let mut refs = [[0u16; 128]; 16];
    let mut owner = [[0usize; 128]; 16];
    let mut last_on = [[u64::MAX; 128]; 16];
    let mut tempo_map: Vec<(u64, u32)> = Vec::new();

    let sync = |tr: &mut Track, st: &mut PhysState, ch: u8, tick: u64, program: Option<u8>, ctl: &CtlState| {
        if !st.init {
            st.init = true;
            // pitch bend range: +-12 semitones (RPN 0), then null RPN
            for (c, v) in [(101u8, 0u8), (100, 0), (6, 12), (38, 0), (101, 127), (100, 127)] {
                tr.ev(tick, &[0xB0 | ch, c, v]);
            }
        }
        if let Some(p) = program {
            if st.program != Some(p) && ch != 9 {
                tr.ev(tick, &[0xC0 | ch, p & 0x7F]);
                st.program = Some(p);
            }
        }
        if st.vol != Some(ctl.vol) {
            tr.ev(tick, &[0xB0 | ch, 7, ctl.vol]);
            st.vol = Some(ctl.vol);
        }
        if st.expr != Some(ctl.expr) {
            tr.ev(tick, &[0xB0 | ch, 11, ctl.expr]);
            st.expr = Some(ctl.expr);
        }
        if st.pan != Some(ctl.pan) {
            tr.ev(tick, &[0xB0 | ch, 10, ctl.pan]);
            st.pan = Some(ctl.pan);
        }
        if st.bend != Some(ctl.bend) {
            tr.ev(tick, &[0xE0 | ch, (ctl.bend & 0x7F) as u8, (ctl.bend >> 7) as u8]);
            st.bend = Some(ctl.bend);
        }
    };

    for e in events.iter() {
        let ph = match e.logical {
            Logical::Conductor => None,
            Logical::Drum => Some(9u8),
            Logical::Melodic(c) => phys_of.get(&c).copied(),
        };
        match &e.kind {
            EvKind::Tempo(us) => {
                tempo_map.push((e.tick, *us));
                tracks[0].meta(e.tick, 0x51, &us.to_be_bytes()[1..]);
            }
            EvKind::MasterVol(v) => tracks[0].ev(e.tick, &[0xF0, 0x07, 0x7F, 0x7F, 0x04, 0x01, (v & 0x7F) as u8, (v >> 7) as u8, 0xF7]),
            EvKind::Text(kind, s) => tracks[0].meta(e.tick, *kind, s.as_bytes()),
            EvKind::Ctl(ctl) => {
                let (Some(ch), Some(&ti)) = (ph, track_of.get(&e.track)) else { continue };
                if !phys[ch as usize].init {
                    continue; // channel not sounding yet: state is sent with its first note
                }
                sync(&mut tracks[ti], &mut phys[ch as usize], ch, e.tick, None, ctl);
            }
            EvKind::NoteOn { key, vel, program, ctl } => {
                let (Some(ch), Some(&ti)) = (ph, track_of.get(&e.track)) else { continue };
                let k = (*key & 0x7F) as usize;
                sync(&mut tracks[ti], &mut phys[ch as usize], ch, e.tick, *program, ctl);
                let (c, r) = (ch as usize, refs[ch as usize][k]);
                if r > 0 && last_on[c][k] == e.tick {
                    // unison: another layer struck the same key on the same tick
                    refs[c][k] += 1;
                    continue;
                }
                if r > 0 {
                    tracks[ti].ev(e.tick, &[0x80 | ch, k as u8, 0]); // re-strike
                }
                tracks[ti].ev(e.tick, &[0x90 | ch, k as u8, *vel]);
                refs[c][k] += 1;
                owner[c][k] = ti;
                last_on[c][k] = e.tick;
            }
            EvKind::NoteOff { key } => {
                let Some(ch) = ph else { continue };
                let k = (*key & 0x7F) as usize;
                let r = &mut refs[ch as usize][k];
                if *r == 0 {
                    continue;
                }
                *r -= 1;
                if *r == 0 {
                    let ti = track_of.get(&e.track).copied().unwrap_or(owner[ch as usize][k]);
                    tracks[ti].ev(e.tick, &[0x80 | ch, k as u8, 0]);
                }
            }
        }
    }
    for ch in 0..16 {
        for k in 0..128 {
            if refs[ch][k] > 0 {
                tracks[owner[ch][k]].ev(end_tick, &[0x80 | ch as u8, k as u8, 0]);
            }
        }
    }
    for t in tracks.iter_mut() {
        t.meta(end_tick.max(t.last), 0x2F, &[]);
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"MThd");
    bytes.extend_from_slice(&6u32.to_be_bytes());
    bytes.extend_from_slice(&1u16.to_be_bytes());
    bytes.extend_from_slice(&(tracks.len() as u16).to_be_bytes());
    bytes.extend_from_slice(&(TATUMS_PER_BEAT as u16).to_be_bytes());
    for t in &tracks {
        bytes.extend_from_slice(b"MTrk");
        bytes.extend_from_slice(&(t.data.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&t.data);
    }
    // duration from the tempo map
    let mut seconds = 0.0;
    let (mut t0, mut us) = (0u64, 500_000u32);
    for &(t, u) in &tempo_map {
        seconds += (t - t0) as f64 * us as f64 / 1e6 / TATUMS_PER_BEAT as f64;
        t0 = t;
        us = u;
    }
    seconds += end_tick.saturating_sub(t0) as f64 * us as f64 / 1e6 / TATUMS_PER_BEAT as f64;
    MidiOut { bytes, tracks: tracks.len(), channels_used: phys_of.len() + has_drums as usize, phys_of, overflow_channels: overflow, seconds }
}

// ---------------------------------------------------------------------------------------------
// Sound effects: map sfx ids to what sequence 0 plays for them

/// Runs sequence 0 with one sfx request, like Audio_PlayActiveSfx: channel IO port 0 = 1,
/// port 4 = id & 0xFF, port 5 = (id & 0x100) >> 8 on large banks. Returns the sources
/// (instruments, drums, font sfx, synthetic waves) and samples played in the first 3 seconds.
fn probe_sfx(seq0: &[u8], fonts: &[SimFont], alias: &[usize], raw_fonts: &[u8], all: &[Vec<u8>], bank: usize, index: u32) -> (Vec<String>, Vec<usize>, u64, Vec<String>) {
    // gChannelsPerBank[0] = {3, 2, 3, 3, 2, 1, 2}; the first channel of each bank
    const FIRST_CHAN: [usize; 7] = [0, 3, 5, 8, 11, 13, 14];
    let mut sim = Sim::new(seq0.to_vec(), fonts, alias, raw_fonts.to_vec(), all);
    // let the sequence set up its channels
    for _ in 0..4 {
        sim.tick_seq();
        sim.tick += 1;
    }
    let ci = FIRST_CHAN[bank];
    sim.trace = std::env::var("OOT_AUDIO_TRACE_SFX").ok().and_then(|v| u32::from_str_radix(v.trim_start_matches("0x"), 16).ok()) == Some(((bank as u32) << 12) | 0x800 | index);
    sim.played_src.clear();
    sim.played_samples.clear();
    sim.warnings.clear();
    sim.notes = 0;
    sim.channels[ci].io[0] = 1;
    sim.channels[ci].io[4] = (index & 0xFF) as u8 as i8;
    if bank == 3 {
        sim.channels[ci].io[5] = ((index & 0x100) >> 8) as i8;
    }
    // ~3 seconds at the sequence's tempo
    let start = sim.seconds;
    while sim.seconds - start < 3.0 && !sim.finished && !sim.frozen {
        sim.tick_seq();
        sim.tick += 1;
        sim.seconds += 60.0 / sim.eff_tempo();
    }
    (sim.played_src.into_iter().collect(), sim.played_samples.into_iter().collect(), sim.notes, sim.warnings.into_keys().collect())
}

// ---------------------------------------------------------------------------------------------

pub fn extract(p: &Project, out: &Path) -> Result<serde_json::Value> {
    let out = out.join("audio");
    std::fs::create_dir_all(&out)?;
    let rom = &p.rom.data;
    let font_table = parse_table(rom, SOUNDFONT_TABLE).context("gSoundFontTable")?;
    let seq_table = parse_table(rom, SEQUENCE_TABLE).context("gSequenceTable")?;
    let bank_table = parse_table(rom, SAMPLE_BANK_TABLE).context("gSampleBankTable")?;
    let seq_font_table = rom.get(SEQUENCE_FONT_TABLE.0..SEQUENCE_FONT_TABLE.0 + SEQUENCE_FONT_TABLE.1).context("gSequenceFontTable")?;
    let audiobank = p.rom.file_by_name("Audiobank").context("Audiobank")?;
    let audioseq = p.rom.file_by_name("Audioseq").context("Audioseq")?;
    let audiotable = p.rom.file_by_name("Audiotable").context("Audiotable")?;
    let mut errors: Vec<String> = Vec::new();

    // --- sample banks
    let banks_json: Vec<Value> = bank_table
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let mut v = json!({"id": i, "offset": format!("0x{:X}", e.addr), "size": e.size, "medium": e.medium, "cache_policy": e.cache});
            if e.size == 0 {
                v["alias_of"] = json!(e.addr);
            } else if e.addr as usize + e.size as usize > audiotable.len() {
                errors.push(format!("sample bank {i} exceeds Audiotable"));
            }
            v
        })
        .collect();

    // --- soundfonts + samples
    let mut reg = Registry { table: &audiotable, banks: &bank_table, out: &out, recs: Vec::new(), index: HashMap::new(), errors: Vec::new() };
    let mut fonts: Vec<Font> = Vec::new();
    for (fi, e) in font_table.iter().enumerate() {
        if e.size == 0 {
            fonts.push(Font::default());
            continue;
        }
        let data = match entry_slice(&audiobank, e, &format!("font {fi}")) {
            Ok(d) => d,
            Err(x) => {
                errors.push(format!("{x:#}"));
                fonts.push(Font::default());
                continue;
            }
        };
        let mut f = parse_font(data, e);
        let (b1, b2) = (f.bank1, f.bank2);
        for (ii, inst) in f.instruments.iter_mut().enumerate() {
            let Some(inst) = inst else { continue };
            for (slot, t) in [("low", &mut inst.low), ("norm", &mut inst.normal), ("high", &mut inst.high)] {
                if let Some(t) = t {
                    reg.register(fi, data, b1, b2, t, &format!("inst{ii:03}_{slot}"));
                }
            }
        }
        for (di, d) in f.drums.iter_mut().enumerate() {
            if let Some(Drum { sound: Some(t), .. }) = d {
                reg.register(fi, data, b1, b2, t, &format!("drum{di:03}"));
            }
        }
        for (si, s) in f.sfx.iter_mut().enumerate() {
            if let Some(t) = s {
                reg.register(fi, data, b1, b2, t, &format!("sfx{si:03}"));
            }
        }
        errors.extend(f.errors.iter().map(|x| format!("font {fi}: {x}")));
        fonts.push(f);
    }
    errors.append(&mut reg.errors);
    for r in &reg.recs {
        errors.extend(r.errors.iter().map(|e| format!("{} ({}): {e}", r.file, r.refs.join(", "))));
    }

    for (fi, (f, e)) in fonts.iter().zip(&font_table).enumerate() {
        let path = out.join("soundfonts").join(format!("font_{fi:02}.json"));
        let mut v = json!({
            "id": fi,
            "offset": format!("0x{:X}", e.addr),
            "size": e.size,
            "medium": e.medium,
            "cache_policy": e.cache,
        });
        if e.size == 0 {
            v["alias_of"] = json!(e.addr);
            write_json(&path, &v)?;
            continue;
        }
        let bank = |b: u8| if b == 0xFF { Value::Null } else { json!({"id": b, "real": real_id(&bank_table, b as usize)}) };
        v["sample_banks"] = json!([bank(f.bank1), bank(f.bank2)]);
        v["num_instruments"] = json!((e.d2 >> 8) & 0xFF);
        v["num_drums"] = json!(e.d2 & 0xFF);
        v["num_sfx"] = json!(e.d3);
        v["notes"] = json!("Keys are gPitchFrequencies semitones (0 = A0 = MIDI 21, 39 = C4). playback_rate = tuning * 32000 Hz is the rate an instrument sample plays at C4 (other keys scale by 2^((key-60)/12)); drums and sfx always play at playback_rate. wav_root_key is the MIDI key at which the reference plays the WAV at its own rate (for instruments, the smpl unity note of the first reference). release_rate_index indexes the ADSR decay table.");
        v["instruments"] = Value::Array(
            f.instruments
                .iter()
                .enumerate()
                .map(|(i, x)| match x {
                    None => Value::Null,
                    Some(x) => json!({
                        "index": i,
                        "offset": format!("0x{:X}", x.off),
                        "normal_range": [x.lo, x.hi],
                        "normal_range_midi": [(x.lo as u32 + 21).min(127), (x.hi as u32 + 21).min(127)],
                        "range_rule": "semitone < lo: low sample; <= hi: normal; else high",
                        "release_rate_index": x.release,
                        "envelope_offset": format!("0x{:X}", x.env_off),
                        "envelope": envelope_json(&x.envelope),
                        "low": tuned_json(&x.low, &reg),
                        "normal": tuned_json(&x.normal, &reg),
                        "high": tuned_json(&x.high, &reg),
                    }),
                })
                .collect(),
        );
        v["drums"] = Value::Array(
            f.drums
                .iter()
                .enumerate()
                .map(|(i, x)| match x {
                    None => Value::Null,
                    Some(x) => json!({
                        "index": i,
                        "midi_key": i + 21,
                        "offset": format!("0x{:X}", x.off),
                        "release_rate_index": x.release,
                        "pan": x.pan,
                        "envelope_offset": format!("0x{:X}", x.env_off),
                        "envelope": envelope_json(&x.envelope),
                        "sound": tuned_json(&x.sound, &reg),
                    }),
                })
                .collect(),
        );
        v["sfx"] = Value::Array(
            f.sfx
                .iter()
                .enumerate()
                .map(|(i, x)| {
                    let mut t = tuned_json(x, &reg);
                    if !t.is_null() {
                        t["index"] = json!(i);
                    }
                    t
                })
                .collect(),
        );
        if !f.errors.is_empty() {
            v["errors"] = json!(f.errors);
        }
        write_json(&path, &v)?;
    }

    // sample index + stats
    let mut wav_bytes_total = 0usize;
    let mut silent = 0;
    let mut clipping = 0;
    let mut loops = 0;
    let (mut state_exact, mut state_checked, mut state_worst) = (0, 0, 0);
    let mut codecs: BTreeMap<&str, usize> = BTreeMap::new();
    let mut orders: BTreeMap<String, usize> = BTreeMap::new();
    let mut sample_errors = 0;
    let (mut rms_min, mut rms_max, mut rms_sum) = (f64::MAX, 0f64, 0f64);
    let mut worst_clip = 0f64;
    let samples_json: Vec<Value> = reg
        .recs
        .iter()
        .map(|r| {
            wav_bytes_total += r.bytes;
            *codecs.entry(codec_name(r.hdr.codec)).or_default() += 1;
            if let Some(b) = &r.hdr.book {
                *orders.entry(format!("order{}_pred{}", b.order, b.npred)).or_default() += 1;
            }
            if r.rms < 1e-4 {
                silent += 1;
            }
            if r.clip_pct > 1.0 {
                clipping += 1;
            }
            worst_clip = worst_clip.max(r.clip_pct);
            rms_min = rms_min.min(r.rms);
            rms_max = rms_max.max(r.rms);
            rms_sum += r.rms;
            if r.hdr.lp.count != 0 {
                loops += 1;
            }
            if let Some(d) = r.loop_state_diff {
                state_checked += 1;
                if d == 0 {
                    state_exact += 1;
                }
                state_worst = state_worst.max(d);
            }
            if !r.errors.is_empty() {
                sample_errors += 1;
            }
            let mut v = json!({
                "file": format!("samples/{}", r.file),
                "bank": r.bank,
                "bank_offset": format!("0x{:X}", r.hdr.addr),
                "size": r.hdr.size,
                "codec": codec_name(r.hdr.codec),
                "medium_slot": r.hdr.medium,
                "unk_bit26": r.hdr.unk_bit26,
                "loop_offset": format!("0x{:X}", r.hdr.loop_off),
                "book_offset": format!("0x{:X}", r.hdr.book_off),
                "tuning": r.tuning,
                "sample_rate": r.rate,
                "smpl_unity_note": r.unity,
                "num_samples": r.num_samples,
                "capacity_samples": r.capacity,
                "loop": {"start": r.hdr.lp.start, "end": r.hdr.lp.end, "count": if r.hdr.lp.count == u32::MAX { json!("infinite") } else { json!(r.hdr.lp.count) }, "state": r.hdr.lp.state.map(|s| s.to_vec())},
                "rms": (r.rms * 1e5).round() / 1e5,
                "peak": r.peak,
                "clip_pct": (r.clip_pct * 1000.0).round() / 1000.0,
                "references": r.refs,
            });
            if let Some(b) = &r.hdr.book {
                v["book"] = json!({"order": b.order, "predictors": b.npred});
            }
            if !r.other_tunings.is_empty() {
                v["other_tunings"] = json!(r.other_tunings.iter().map(|&b| f32::from_bits(b)).collect::<Vec<_>>());
            }
            if r.loop_variants > 0 {
                v["loop_variants_in_other_refs"] = json!(r.loop_variants);
            }
            if let Some(d) = r.loop_state_diff {
                v["loop_state_max_diff"] = json!(d);
            }
            if !r.errors.is_empty() {
                v["errors"] = json!(r.errors);
            }
            if !r.notes.is_empty() {
                v["notes"] = json!(r.notes);
            }
            v
        })
        .collect();
    write_json(
        &out.join("samples").join("index.json"),
        &json!({
            "notes": "Rate rule, from the first reference (fonts in order; per font: instruments low/normal/high, drums, sfx): drums/sfx use round(tuning * 32000), their exact in-game rate, unity note 60. Instrument samples use tuning * 32000 * 2^(n/12) with unity note 60 + n, n chosen so the rate is a standard recording rate (8000/11025/16000/22050/24000/32000/44100/48000 within 5 cents), else n = 0 moved by octaves into 8-48 kHz; pitch is exact either way. Length = loop end (the synthesizer never plays past it). smpl loop end is inclusive (loop.end - 1); play count 0 = infinite.",
            "samples": samples_json,
        }),
    )?;

    // --- sequences
    let names = sequence_names(&p.config.decomp);
    let seq_fonts = |i: usize| -> (Vec<u8>, Vec<u8>) {
        let off = be16_at(seq_font_table, i * 2) as usize;
        let n = seq_font_table.get(off).copied().unwrap_or(0) as usize;
        let raw = seq_font_table.get(off..off + 1 + n).map(|s| s.to_vec()).unwrap_or_default();
        (raw.get(1..).map(|s| s.to_vec()).unwrap_or_default(), raw)
    };
    let seq_data: Vec<Vec<u8>> = seq_table
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let r = real_id(&seq_table, i);
            seq_table.get(r).and_then(|e2| entry_slice(&audioseq, e2, "seq").ok()).map(|s| s.to_vec()).unwrap_or_else(|| {
                if e.size != 0 {
                    errors.push(format!("sequence {i} outside Audioseq"));
                }
                Vec::new()
            })
        })
        .collect();

    // simplified fonts for the sequence player
    let font_alias: Vec<usize> = (0..font_table.len()).map(|i| real_id(&font_table, i)).collect();
    let snd = |t: &Option<TunedRef>| -> Option<SimSound> {
        let t = t.as_ref()?;
        let r = &reg.recs[t.sample?];
        Some(SimSound { tuning: t.tuning, len: r.num_samples as u32, rec: t.sample? })
    };
    let sim_fonts: Vec<SimFont> = fonts
        .iter()
        .map(|f| SimFont {
            insts: f.instruments.iter().map(|x| x.as_ref().map(|x| SimInst { lo: x.lo, hi: x.hi, low: snd(&x.low), normal: snd(&x.normal), high: snd(&x.high) })).collect(),
            drums: f.drums.iter().map(|x| x.as_ref().map(|d| (snd(&d.sound), d.pan))).collect(),
            sfx: f.sfx.iter().map(snd).collect(),
        })
        .collect();

    let seq_dir = out.join("sequences");
    std::fs::create_dir_all(&seq_dir)?;
    let (mut seq_bytes, mut midi_bytes, mut midi_files, mut midi_notes) = (0usize, 0usize, 0usize, 0u64);
    let mut midi_reasons: BTreeMap<&str, usize> = BTreeMap::new();
    let mut seqs_json = Vec::new();
    for (i, e) in seq_table.iter().enumerate() {
        let (name, title) = names.get(&(i as u32)).cloned().unwrap_or_else(|| (format!("SEQ_{i:03}"), String::new()));
        let base = format!("{i:03}_{}", sanitize(&name));
        let (fonts_list, raw_fonts) = seq_fonts(i);
        let mut v = json!({
            "id": i,
            "name": format!("NA_BGM_{name}"),
            "title": title,
            "offset": format!("0x{:X}", e.addr),
            "size": e.size,
            "medium": e.medium,
            "cache_policy": e.cache,
            "fonts": fonts_list,
        });
        if e.size == 0 {
            let target = e.addr as usize;
            v["alias_of"] = json!(target);
            v["alias_of_name"] = json!(names.get(&(target as u32)).map(|n| format!("NA_BGM_{}", n.0)));
            seqs_json.push(v);
            continue;
        }
        let data = &seq_data[i];
        std::fs::write(seq_dir.join(format!("{base}.seq")), data)?;
        seq_bytes += data.len();
        v["file"] = json!(format!("sequences/{base}.seq"));

        let mut sim = Sim::new(data.clone(), &sim_fonts, &font_alias, raw_fonts, &seq_data);
        // debugging aid: OOT_AUDIO_TRACE=<seq id> prints every executed command to stderr
        sim.trace = std::env::var("OOT_AUDIO_TRACE").ok().and_then(|v| v.parse::<usize>().ok()) == Some(i);
        let reason = sim.run();
        // sequences that only react to the game (sfx, ambience) produce no notes: keep them short
        let notes = sim.events.iter().filter(|e| matches!(e.kind, EvKind::NoteOn { .. })).count() as u64;
        let end_tick = if notes == 0 { 0 } else { sim.tick };
        let midi = write_midi(&mut sim.events, end_tick, &format!("{} {}", name, title));
        let mid_name = format!("{base}.mid");
        std::fs::write(seq_dir.join(&mid_name), &midi.bytes)?;
        midi_bytes += midi.bytes.len();
        midi_files += 1;
        midi_notes += notes;
        *midi_reasons.entry(reason).or_default() += 1;
        let mut m = json!({
            "file": format!("sequences/{mid_name}"),
            "ticks": end_tick,
            "seconds": (midi.seconds * 100.0).round() / 100.0,
            "notes": notes,
            "tracks": midi.tracks,
            "midi_channels": midi.channels_used,
            "stopped_because": reason,
            "default_font": sim.default_font,
        });
        m["channels"] = Value::Array(
            (0..16)
                .filter(|&c| !sim.chan_used[c].is_empty())
                .map(|c| {
                    let drums = sim.chan_used[c].iter().any(|x| x.contains("/drum"));
                    json!({"seq_channel": c, "midi_channel": midi.phys_of.get(&(c as u8)).map(|&p| p + 1), "drums_on_midi_channel_10": drums, "sources": sim.chan_used[c]})
                })
                .collect(),
        );
        m["seq_script_loop_tick"] = json!(sim.seq_loop_tick);
        m["channel_loop_ticks"] = json!((0..16).filter(|&c| sim.channels[c].used).map(|c| (c.to_string(), sim.channels[c].loop_tick)).collect::<BTreeMap<_, _>>());
        if midi.overflow_channels > 0 {
            m["channels_sharing_midi_16"] = json!(midi.overflow_channels);
        }
        if let Some(c) = sim.chain {
            m["chains_to_sequence"] = json!(c);
        }
        if !sim.warnings.is_empty() {
            m["warnings"] = json!(sim.warnings);
        }
        v["midi"] = m;
        seqs_json.push(v);
    }
    write_json(
        &out.join("sequences.json"),
        &json!({
            "notes": "fonts: gSequenceFontTable entry (the last font is the default). MIDI: division 48 (TATUMS_PER_BEAT); one track per sequence channel; drums on MIDI channel 10; program = instrument id in the channel's font (126 = font sfx, 80-87 = synthetic waves; see midi.channels for the sources); velocity = the script's velocity; CC7/CC11 = 127 * sqrt(volume / 127) because the game scales amplitude linearly and GM players square these; pan = channel/layer pan mix; pitch bend range +-12. Loops are played once: conversion stops when the sequence script and every enabled channel have jumped back to code they already ran (or at 15 minutes). Game IO ports are left at -1, so sequences that wait for the game stay silent in those branches.",
            "sequences": seqs_json,
        }),
    )?;

    // --- sound effects: names from the decomp, and what sequence 0 plays for each
    let sfx = sfx_names(&p.config.decomp);
    let mut sfx_mapped = 0;
    let sfx_json: Vec<Value> = if !seq_data.is_empty() && !seq_data[0].is_empty() {
        let (_, raw0) = seq_fonts(0);
        let bank_file_to_id = |b: &str| match b {
            "playerbank" => Some(0),
            "itembank" => Some(1),
            "environmentbank" => Some(2),
            "enemybank" => Some(3),
            "systembank" => Some(4),
            "ocarinabank" => Some(5),
            "voicebank" => Some(6),
            _ => None,
        };
        sfx.iter()
            .map(|(id, name, bank)| {
                let mut v = json!({"id": format!("0x{id:04X}"), "name": name, "bank": bank});
                if let Some(b) = bank_file_to_id(bank) {
                    let (sources, recs, notes, warnings) = probe_sfx(&seq_data[0], &sim_fonts, &font_alias, &raw0, &seq_data, b, id & 0x1FF);
                    if !sources.is_empty() {
                        sfx_mapped += 1;
                    }
                    v["sources"] = json!(sources);
                    v["files"] = json!(recs.iter().map(|&i| format!("samples/{}", reg.recs[i].file)).collect::<Vec<_>>());
                    v["notes_played"] = json!(notes);
                    if !warnings.is_empty() {
                        v["warnings"] = json!(warnings);
                    }
                }
                v
            })
            .collect()
    } else {
        Vec::new()
    };
    write_json(
        &out.join("sfx.json"),
        &json!({
            "notes": "Sound effect ids from include/tables/sfx/*.h. sources/files: the instruments, drums, font sound effects and synthetic waves (and their sample files) sequence 0 plays in the first 3 s after the game's request (IO port 0 = 1, port 4 = id & 0xFF, port 5 = bit 8 on the enemy bank). Sfx scripts that also depend on other IO ports (set by the game per sound) may pick a different branch in-game.",
            "sfx": sfx_json,
        }),
    )?;

    write_json(&out.join("sample_banks.json"), &json!({"banks": banks_json}))?;

    let n_err = errors.len();
    errors.truncate(20);
    Ok(json!({
        "fonts": font_table.len(),
        "sequences": seq_table.len(),
        "sample_banks": bank_table.len(),
        "samples": reg.recs.len(),
        "wav_bytes": wav_bytes_total,
        "codecs": codecs,
        "books": orders,
        "looped_samples": loops,
        "loop_state_exact": format!("{state_exact}/{state_checked} (worst diff {state_worst})"),
        "silent_samples": silent,
        "clipping_samples_over_1pct": clipping,
        "worst_clip_pct": (worst_clip * 1000.0).round() / 1000.0,
        "rms": {"min": (if rms_min == f64::MAX { 0.0 } else { rms_min } * 1e5).round() / 1e5, "max": (rms_max * 1e5).round() / 1e5, "mean": ((rms_sum / reg.recs.len().max(1) as f64) * 1e5).round() / 1e5},
        "samples_with_errors": sample_errors,
        "seq_bytes": seq_bytes,
        "midi_files": midi_files,
        "midi_bytes": midi_bytes,
        "midi_notes": midi_notes,
        "midi_stop_reasons": midi_reasons,
        "sfx_names": sfx.len(),
        "sfx_mapped": sfx_mapped,
        "error_count": n_err,
        "errors": errors,
    }))
}
