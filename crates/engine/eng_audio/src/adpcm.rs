//! The reference VADPCM decoder: a whole sample at once, any predictor order, the way the
//! extractor has always decoded them (moved here from `oot_extract::audio`). The game decodes
//! as it plays (the microcode's `aADPCMdec`, `rsp.rs`); this one is for tools and tests, which
//! check the two against each other and against the loops' stored predictor states.

/// `AdpcmBook`: `order` × `npred` predictors of 8 coefficients each, `[pred][order][8]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Book {
    pub order: usize,
    pub npred: usize,
    pub coefs: Vec<i16>,
}

pub const CODEC_ADPCM: u8 = 0;
pub const CODEC_S8: u8 = 1;
pub const CODEC_S16_INMEMORY: u8 = 2;
pub const CODEC_SMALL_ADPCM: u8 = 3;
pub const CODEC_S16: u8 = 5;

/// Decodes `n` samples of `codec` data. Returns the PCM and what went wrong, if anything (the
/// data running out early, predictor indexes past the book).
pub fn decode_sample(codec: u8, data: &[u8], n: usize, book: Option<&Book>) -> (Vec<i16>, Vec<String>) {
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
        c => errs.push(format!("unsupported codec {c}")),
    }
    (out, errs)
}

/// Samples the data can hold, from the codec's frame size.
pub fn capacity_samples(codec: u8, size: u32) -> usize {
    match codec {
        CODEC_ADPCM => size as usize / 9 * 16,
        CODEC_SMALL_ADPCM => size as usize / 5 * 16,
        CODEC_S8 => size as usize,
        CODEC_S16 | CODEC_S16_INMEMORY => size as usize / 2,
        _ => 0,
    }
}
