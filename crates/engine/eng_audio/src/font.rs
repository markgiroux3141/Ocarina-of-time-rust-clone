//! Reading a soundfont straight from the ROM's bytes, for tools and tests: its instruments,
//! drums and sound effects, their envelopes, samples, loops and books (the layout
//! `AudioLoad_RelocateFont` relocates; moved here from `oot_extract::audio`). The game itself
//! reads fonts from RAM once relocated (`load.rs`).

use anyhow::{Context, Result, bail};

use crate::adpcm::{Book, CODEC_ADPCM, CODEC_SMALL_ADPCM, capacity_samples, decode_sample};
use crate::context::AudioTable;
use crate::data::AudioData;

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

/// `AdpcmLoop`.
#[derive(Debug, Clone, PartialEq)]
pub struct Loop {
    pub start: u32,
    pub end: u32,
    pub count: u32,
    /// `predictorState`, only there if `count != 0`.
    pub state: Option<[i16; 16]>,
}

/// A `Sample`, its offsets resolved: `addr` is in sample bank `bank` (the real one).
#[derive(Debug, Clone, PartialEq)]
pub struct SampleInfo {
    pub codec: u8,
    pub medium: u8,
    pub unk_bit26: bool,
    pub size: u32,
    pub bank: usize,
    pub addr: u32,
    pub lp: Loop,
    pub book: Option<Book>,
}

/// A `TunedSample`.
#[derive(Debug, Clone, PartialEq)]
pub struct Tuned {
    pub sample: SampleInfo,
    pub tuning: f32,
}

/// An `Instrument`.
#[derive(Debug, Clone, PartialEq)]
pub struct Instrument {
    pub normal_range_lo: u8,
    pub normal_range_hi: u8,
    pub adsr_decay_index: u8,
    pub envelope: Vec<(i16, i16)>,
    pub low: Option<Tuned>,
    pub normal: Option<Tuned>,
    pub high: Option<Tuned>,
}

impl Instrument {
    /// `Audio_GetInstrumentTunedSample`.
    pub fn tuned_sample(&self, semitone: i32) -> Option<&Tuned> {
        if semitone < self.normal_range_lo as i32 {
            self.low.as_ref()
        } else if semitone <= self.normal_range_hi as i32 {
            self.normal.as_ref()
        } else {
            self.high.as_ref()
        }
    }
}

/// A `Drum`.
#[derive(Debug, Clone, PartialEq)]
pub struct Drum {
    pub adsr_decay_index: u8,
    pub pan: u8,
    pub envelope: Vec<(i16, i16)>,
    pub sound: Option<Tuned>,
}

/// A font, read.
#[derive(Debug, Clone, Default)]
pub struct Font {
    pub id: usize,
    pub sample_bank_id1: u8,
    pub sample_bank_id2: u8,
    pub instruments: Vec<Option<Instrument>>,
    pub drums: Vec<Option<Drum>>,
    pub sfx: Vec<Option<Tuned>>,
}

/// An envelope's points, up to the first command that ends it (`ADSR_DISABLE`, `HANG`,
/// `GOTO`, `RESTART`: delay <= 0).
fn parse_envelope(font: &[u8], off: u32) -> Vec<(i16, i16)> {
    let mut v = Vec::new();
    for i in 0..48 {
        let o = off as usize + i * 4;
        let (Ok(d), Ok(a)) = (rd_u16(font, o), rd_u16(font, o + 2)) else { break };
        v.push((d as i16, a as i16));
        if (d as i16) <= 0 {
            break;
        }
    }
    v
}

/// The tables and how to resolve a sample's bank.
pub struct FontReader<'a> {
    pub data: &'a AudioData,
    pub fonts: AudioTable,
    pub banks: AudioTable,
}

impl<'a> FontReader<'a> {
    pub fn new(data: &'a AudioData) -> FontReader<'a> {
        FontReader { data, fonts: AudioTable::parse(&data.tables.sound_font_table), banks: AudioTable::parse(&data.tables.sample_bank_table) }
    }

    /// `AudioLoad_GetRealTableIndex` on a table.
    pub fn real(t: &AudioTable, id: usize) -> usize {
        match t.entries.get(id) {
            Some(e) if e.size == 0 => e.rom_addr as usize,
            _ => id,
        }
    }

    /// The font's bytes in `Audiobank`.
    pub fn font_bytes(&self, font_id: usize) -> Result<&'a [u8]> {
        let r = Self::real(&self.fonts, font_id);
        let e = self.fonts.entries.get(r).with_context(|| format!("font {font_id} isn't in the table"))?;
        let (a, s) = (e.rom_addr as usize, e.size as usize);
        self.data.audiobank.bytes.get(a..a + s).with_context(|| format!("font {font_id}: 0x{a:X}+0x{s:X} outside Audiobank"))
    }

    fn parse_sample(&self, font: &[u8], off: u32, bank1: u8, bank2: u8) -> Result<SampleInfo> {
        let o = off as usize;
        let w = rd_u32(font, o)?;
        let codec = (w >> 28) as u8;
        let medium = ((w >> 26) & 3) as u8;
        let unk_bit26 = (w >> 25) & 1 != 0;
        let size = w & 0xFF_FFFF;
        let addr = rd_u32(font, o + 4)?;
        let lo = rd_u32(font, o + 8)? as usize;
        let bo = rd_u32(font, o + 12)? as usize;
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
            let order = rd_u32(font, bo)? as usize;
            let npred = rd_u32(font, bo + 4)? as usize;
            if !(1..=8).contains(&order) || !(1..=16).contains(&npred) {
                bail!("sample at 0x{off:X}: implausible ADPCM book order {order} predictors {npred}");
            }
            let coefs = (0..8 * order * npred).map(|i| rd_u16(font, bo + 8 + i * 2).map(|v| v as i16)).collect::<Result<_>>()?;
            Some(Book { order, npred, coefs })
        } else {
            None
        };
        let bank_id = match medium {
            0 => bank1,
            1 => bank2,
            m => bail!("sample medium {m} is not relocatable"),
        };
        if bank_id == 0xFF {
            bail!("sample uses sample bank slot {} but the font has none", medium + 1);
        }
        let bank = Self::real(&self.banks, bank_id as usize);
        Ok(SampleInfo { codec, medium, unk_bit26, size, bank, addr, lp: Loop { start, end, count, state }, book })
    }

    fn parse_tuned(&self, font: &[u8], o: usize, b1: u8, b2: u8) -> Result<Option<Tuned>> {
        let off = rd_u32(font, o)?;
        if off == 0 {
            return Ok(None);
        }
        let tuning = f32::from_bits(rd_u32(font, o + 4)?);
        Ok(Some(Tuned { sample: self.parse_sample(font, off, b1, b2)?, tuning }))
    }

    /// Reads font `font_id` (its real entry).
    pub fn font(&self, font_id: usize) -> Result<Font> {
        let r = Self::real(&self.fonts, font_id);
        let e = self.fonts.entries[r];
        let data = self.font_bytes(font_id)?;
        let (b1, b2) = ((e.short_data1 >> 8) as u8, e.short_data1 as u8);
        let (ninst, ndrums, nsfx) = (((e.short_data2 >> 8) & 0xFF) as usize, (e.short_data2 & 0xFF) as usize, e.short_data3 as u16 as usize);
        let mut f = Font { id: r, sample_bank_id1: b1, sample_bank_id2: b2, ..Default::default() };

        let list = rd_u32(data, 0)? as usize;
        if list != 0 {
            for i in 0..ndrums {
                let off = rd_u32(data, list + i * 4)? as usize;
                if off == 0 {
                    f.drums.push(None);
                    continue;
                }
                let env = rd_u32(data, off + 12)?;
                f.drums.push(Some(Drum { adsr_decay_index: rd_u8(data, off)?, pan: rd_u8(data, off + 1)?, envelope: parse_envelope(data, env), sound: self.parse_tuned(data, off + 4, b1, b2)? }));
            }
        }
        let list = rd_u32(data, 4)? as usize;
        if list != 0 {
            for i in 0..nsfx {
                f.sfx.push(self.parse_tuned(data, list + i * 8, b1, b2)?);
            }
        }
        // ids >= 126 are reserved (AudioLoad_RelocateFont clamps to 126)
        for i in 0..ninst.min(126) {
            let off = rd_u32(data, 8 + i * 4)? as usize;
            if off == 0 {
                f.instruments.push(None);
                continue;
            }
            let lo = rd_u8(data, off + 1)?;
            let hi = rd_u8(data, off + 2)?;
            let env = rd_u32(data, off + 4)?;
            f.instruments.push(Some(Instrument {
                normal_range_lo: lo,
                normal_range_hi: hi,
                adsr_decay_index: rd_u8(data, off + 3)?,
                envelope: parse_envelope(data, env),
                // RelocateFont only touches the low/high samples when the ranges say they exist
                low: if lo != 0 { self.parse_tuned(data, off + 8, b1, b2)? } else { None },
                normal: self.parse_tuned(data, off + 0x10, b1, b2)?,
                high: if hi != 0x7F { self.parse_tuned(data, off + 0x18, b1, b2)? } else { None },
            }));
        }
        Ok(f)
    }

    /// The sample's bytes in `Audiotable`, from its start to the bank's end.
    pub fn sample_data(&self, s: &SampleInfo) -> Result<&'a [u8]> {
        let b = self.banks.entries.get(s.bank).with_context(|| format!("sample bank {}", s.bank))?;
        let start = (b.rom_addr + s.addr) as usize;
        let end = ((b.rom_addr + b.size) as usize).min(self.data.audiotable.bytes.len());
        self.data.audiotable.bytes.get(start..end).with_context(|| format!("sample at 0x{start:X} outside Audiotable"))
    }

    /// Decodes the sample up to its loop end, as the synthesis plays it (reading past `size`
    /// into the last frame, as the game's DMA does).
    pub fn decode(&self, s: &SampleInfo) -> Result<Vec<i16>> {
        let capacity = capacity_samples(s.codec, s.size);
        let n = if s.lp.end == 0 { capacity } else { s.lp.end as usize };
        let (pcm, errs) = decode_sample(s.codec, self.sample_data(s)?, n, s.book.as_ref());
        if pcm.len() < n.min(capacity) {
            bail!("decoding: {}", errs.join("; "));
        }
        Ok(pcm)
    }
}
