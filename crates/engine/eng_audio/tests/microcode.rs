//! The microcode (`rsp.rs`) on made-up data (no game data): its ADPCM decoder against the
//! reference decoder, the resampler's stepping, the envelope mixer's volumes and ramps, the
//! commands' encodings as `abi.h` writes them; the AI model; the note script.

use eng_audio::adpcm::{Book, decode_sample};
use eng_audio::ram::Ram;
use eng_audio::renderer::note_script;
use eng_audio::rsp::abi::*;
use eng_audio::rsp::{A_CONTINUE, A_INIT, Rsp};
use eng_audio::thread::Ai;

/// A deterministic pseudo-random sequence (no game data).
struct Lcg(u32);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_103_515_245).wrapping_add(12345);
        self.0 >> 8
    }
}

/// A resampler table whose phases all pass the third tap through (0x8000 = 1.0 would overflow
/// an s16; 0x7FFF is the nearest).
fn passthrough_lut() -> Vec<i16> {
    (0..64).flat_map(|_| [0i16, 0, 0x7FFF, 0]).collect()
}

#[test]
fn the_adpcm_decoder_matches_the_reference_decoder() {
    let mut rng = Lcg(0xC0FFEE);
    for small in [false, true] {
        for npred in [1usize, 2, 4] {
            // A book of order 2 with modest coefficients, like the game's.
            let coefs: Vec<i16> = (0..8 * 2 * npred).map(|_| (rng.next() % 4096) as i16 - 2048).collect();
            let book = Book { order: 2, npred, coefs };
            let fsize = if small { 5 } else { 9 };
            let frames = 40;
            let data: Vec<u8> = (0..frames * fsize).map(|i| if i % fsize == 0 { (((rng.next() % 13) << 4) | (rng.next() % npred as u32)) as u8 } else { rng.next() as u8 }).collect();
            let codec = if small { 3 } else { 0 };
            let (reference, errs) = decode_sample(codec, &data, frames * 16, Some(&book));
            assert!(errs.is_empty(), "{errs:?}");

            let mut ram = Ram::new();
            let mut rsp = Rsp::new(&passthrough_lut());
            let (book_at, state, src) = (0x8001_0000, 0x8001_1000, 0x8002_0000);
            for (i, c) in book.coefs.iter().enumerate() {
                ram.set_s16(book_at + 2 * i as u32, *c);
            }
            ram.write(src, &data);
            rsp.command(a_load_adpcm((16 * 2 * npred) as u32, book_at), &mut ram);
            let mut out = Vec::new();
            // Five frames per command, continuing from the state each time.
            for chunk in 0..frames / 5 {
                let at = src + (chunk * 5 * fsize) as u32;
                let pad = at & 0xF;
                let aligned = ((5 * fsize + 16 + 15) & !15) as u32;
                rsp.command(a_load_buffer(at - pad, 0x800 - aligned, aligned), &mut ram);
                rsp.command(a_set_buffer(0, 0x800 - aligned + pad, 0x400, 5 * 32), &mut ram);
                rsp.command(a_adpcm_dec(if chunk == 0 { A_INIT } else { A_CONTINUE } | if small { 4 } else { 0 }, state), &mut ram);
                for i in 0..5 * 16 {
                    out.push(rsp.s16(0x400 + 32 + 2 * i as u32));
                }
            }
            assert_eq!(out, reference, "small {small}, {npred} predictors");
        }
    }
}

#[test]
fn the_resampler_steps_by_the_pitch() {
    let mut ram = Ram::new();
    let mut rsp = Rsp::new(&passthrough_lut());
    // A ramp of input at 0x500.
    for i in 0..64 {
        rsp.set_s16(0x500 + 2 * i, (i * 100) as i16);
    }
    let state = 0x8001_0000;
    // pitch 0x4000 (Q1.15 0.5): each input sample twice. The table's third tap is
    // `ipos + 2`, `ipos` starting 4 before the input (the state's samples).
    rsp.command(a_set_buffer(0, 0x500, 0x700, 32), &mut ram);
    rsp.command(a_resample(A_INIT, 0x4000, state), &mut ram);
    let out: Vec<i16> = (0..16).map(|i| rsp.s16(0x700 + 2 * i)).collect();
    let want: Vec<i16> = (0..16)
        .map(|k| {
            let ipos = k / 2;
            let src = ipos as i32 - 2; // ipos + 2 - 4
            let v = if src < 0 { 0 } else { src * 100 };
            ((v as i64 * 0x7FFF) >> 15) as i16
        })
        .collect();
    assert_eq!(out, want);
    // The state holds the four samples from where it stopped, and the fraction.
    assert_eq!(ram.u16(state + 8), 0);
}

#[test]
fn the_envelope_mixer_scales_and_ramps() {
    let mut ram = Ram::new();
    let mut rsp = Rsp::new(&passthrough_lut());
    for i in 0..16 {
        rsp.set_s16(0x3C0 + 2 * i, 10000);
    }
    // Left 0x8000 (half), right 0x4000 (a quarter), reverb 0x40 * 2 << 8 (half the dry, into
    // the wet), no ramps; then the left ramping by 0x1000 per 8 samples.
    rsp.command(a_env_setup1(0x40 * 2, 0, 0x1000, 0), &mut ram);
    rsp.command(a_env_setup2(0x8000, 0x4000), &mut ram);
    let dests = (0x940 >> 4) << 24 | (0xAE0 >> 4) << 16 | (0xC80 >> 4) << 8 | (0xE20 >> 4);
    rsp.command(a_env_mixer(0x3C0, 16, 0, 0, 0, 0, 0, dests, 19 << 24), &mut ram);
    let l: Vec<i16> = (0..16).map(|i| rsp.s16(0x940 + 2 * i)).collect();
    let r: Vec<i16> = (0..16).map(|i| rsp.s16(0xAE0 + 2 * i)).collect();
    let wl: Vec<i16> = (0..16).map(|i| rsp.s16(0xC80 + 2 * i)).collect();
    assert_eq!(&l[..8], &[5000; 8]);
    assert_eq!(&l[8..], &[(10000 * 0x9000 >> 16) as i16; 8], "after one step of the ramp");
    assert_eq!(&r[..], &[2500; 16]);
    assert_eq!(wl[0], ((5000i32 * 0x8000) >> 16) as i16);
}

#[test]
fn the_commands_truncate_as_abi_h_writes_them() {
    // aLoadBuffer's size is stored in 16-byte units: 0x2F loads 0x20 (then the DMA's 8-byte
    // rounding: still 0x20).
    let mut ram = Ram::new();
    let mut rsp = Rsp::new(&passthrough_lut());
    ram.write(0x8001_0000, &[0xAB; 0x40]);
    rsp.command(a_load_buffer(0x8001_0000, 0x100, 0x2F), &mut ram);
    assert_eq!(rsp.s16(0x100 + 0x1E) as u16, 0xABAB);
    assert_eq!(rsp.s16(0x100 + 0x20), 0, "the 0x2F rounded down to 0x20");
    // aClearBuffer clears its size rounded up to 16 bytes.
    for i in 0..0x20 {
        rsp.set_s16(0x200 + 2 * i, -1);
    }
    rsp.command(a_clear_buffer(0x200, 0x12), &mut ram);
    assert_eq!(rsp.s16(0x200 + 0x1E), 0);
    assert_eq!(rsp.s16(0x200 + 0x20), -1);
}

#[test]
fn the_ai_plays_its_buffers_in_order() {
    let mut ai = Ai::new();
    ai.set_next_buffer(vec![1; 2 * 100]);
    ai.set_next_buffer(vec![2; 2 * 100]);
    let mut out = Vec::new();
    ai.play(150.5, &mut out);
    assert_eq!(out.len(), 2 * 150);
    assert_eq!(ai.remaining_in_current(), 50);
    ai.play(100.5, &mut out);
    assert_eq!(out.len(), 2 * 251, "the half frames add up");
    assert_eq!(&out[2 * 199..2 * 201], &[2, 2, 0, 0], "silence after the last buffer");
}

#[test]
fn a_note_script_reads_as_a_sequence() {
    let s = note_script(4, 39, 100, 96, 300, 120);
    // The player: tempo 120, volume, channel 0's setup, channel 0 at its offset.
    assert_eq!(&s[..7], &[0xDD, 120, 0xDB, 0x7F, 0xD7, 0x00, 0x01]);
    let chan = ((s[8] as usize) << 8) | s[9] as usize;
    assert_eq!(&s[chan..chan + 2], &[0xC1, 4], "the channel's instrument");
    assert_eq!(&s[chan + 6..chan + 8], &[0xD4, 0], "no reverb");
    let layer = ((s[chan + 9] as usize) << 8) | s[chan + 10] as usize;
    // Semitone 39 is a note command of its own; 96 tatums (compressed).
    assert_eq!(&s[layer..], &[0xC2, 0, 0xC1, 100, 0xC9, 0, 39, 96, 0xFF]);
    // Past 0x3F the layer transposes.
    let s = note_script(4, 70, 100, 200, 300, 120);
    assert!(s.ends_with(&[0xC2, 70 - 0x3F, 0xC1, 100, 0xC9, 0, 0x3F, 0x80, 200, 0xFF]));
}
