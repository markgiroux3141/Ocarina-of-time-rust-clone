//! The structs the C keeps in RAM, read and written where they are (`z64audio.h`): a font's
//! `Instrument`s, `Drum`s, `SoundEffect`s, `TunedSample`s, `Sample`s, `AdpcmLoop`s,
//! `AdpcmBook`s and `EnvelopePoint`s, which `AudioLoad_RelocateFont` turns from offsets into
//! addresses in place.

use crate::ram::Ram;

/// `Sample`'s first word: `codec:4, medium:2, unk_bit26:1, isRelocated:1, size:24`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleHdr {
    pub codec: u8,
    pub medium: u8,
    pub unk_bit26: bool,
    pub is_relocated: bool,
    pub size: u32,
    pub sample_addr: u32,
    pub loop_addr: u32,
    pub book: u32,
}

/// `Instrument`'s offsets.
pub const INST_IS_RELOCATED: u32 = 0x00;
pub const INST_NORMAL_RANGE_LO: u32 = 0x01;
pub const INST_NORMAL_RANGE_HI: u32 = 0x02;
pub const INST_ADSR_DECAY_INDEX: u32 = 0x03;
pub const INST_ENVELOPE: u32 = 0x04;
pub const INST_LOW_PITCH_TUNED_SAMPLE: u32 = 0x08;
pub const INST_NORMAL_PITCH_TUNED_SAMPLE: u32 = 0x10;
pub const INST_HIGH_PITCH_TUNED_SAMPLE: u32 = 0x18;
/// `Drum`'s offsets.
pub const DRUM_ADSR_DECAY_INDEX: u32 = 0x00;
pub const DRUM_PAN: u32 = 0x01;
pub const DRUM_IS_RELOCATED: u32 = 0x02;
pub const DRUM_TUNED_SAMPLE: u32 = 0x04;
pub const DRUM_ENVELOPE: u32 = 0x0C;
/// `sizeof(SoundEffect)` (a `TunedSample`).
pub const SIZEOF_SOUND_EFFECT: u32 = 0x08;
/// `AdpcmLoop.predictorState`.
pub const LOOP_PREDICTOR_STATE: u32 = 0x10;
/// `AdpcmBook.book`.
pub const BOOK_BOOK: u32 = 0x08;

impl Ram {
    pub fn sample(&self, a: u32) -> SampleHdr {
        let w = self.u32(a);
        SampleHdr {
            codec: (w >> 28) as u8,
            medium: ((w >> 26) & 3) as u8,
            unk_bit26: (w >> 25) & 1 != 0,
            is_relocated: (w >> 24) & 1 != 0,
            size: w & 0xFF_FFFF,
            sample_addr: self.u32(a + 4),
            loop_addr: self.u32(a + 8),
            book: self.u32(a + 12),
        }
    }
    pub fn set_sample_medium(&mut self, a: u32, medium: u8) {
        let w = self.u32(a);
        self.set_u32(a, (w & !(3 << 26)) | ((medium as u32 & 3) << 26));
    }
    pub fn set_sample_relocated(&mut self, a: u32, v: bool) {
        let w = self.u32(a);
        self.set_u32(a, (w & !(1 << 24)) | ((v as u32) << 24));
    }
    pub fn set_sample_addr(&mut self, a: u32, v: u32) {
        self.set_u32(a + 4, v);
    }
    pub fn set_sample_loop(&mut self, a: u32, v: u32) {
        self.set_u32(a + 8, v);
    }
    pub fn set_sample_book(&mut self, a: u32, v: u32) {
        self.set_u32(a + 12, v);
    }

    /// `TunedSample.sample` and `.tuning`.
    pub fn tuned_sample(&self, a: u32) -> (u32, f32) {
        (self.u32(a), self.f32(a + 4))
    }

    /// `AdpcmLoop`: start, end, count.
    pub fn adpcm_loop(&self, a: u32) -> (u32, u32, u32) {
        (self.u32(a), self.u32(a + 4), self.u32(a + 8))
    }

    /// `AdpcmBook`: order, numPredictors.
    pub fn adpcm_book(&self, a: u32) -> (i32, i32) {
        (self.s32(a), self.s32(a + 4))
    }

    /// `envelope[i]`: delay, arg.
    pub fn envelope_point(&self, env: u32, i: u32) -> (i16, i16) {
        (self.s16(env + 4 * i), self.s16(env + 4 * i + 2))
    }
}
