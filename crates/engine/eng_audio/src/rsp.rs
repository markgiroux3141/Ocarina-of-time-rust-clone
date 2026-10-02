//! The audio microcode (`aspMain`, the ROM's `rsp.text`/`rsp.rodata`) as a high-level
//! emulation: the command list `AudioSynth_Update` builds, run against a 4 KB DMEM and RDRAM.
//!
//! The commands are built as the decomp's `abi.h` macros build them, into the same two words,
//! and decoded as the microcode reads them, so the fields' widths and truncations are the
//! hardware's: `aLoadBuffer`'s size rounded down to 16 bytes, `aEnvMixer`'s sample count in
//! a byte, the DMA's 8-byte alignment, the 4 KB DMEM wrapping.
//!
//! The microcode itself isn't in the decomp (it's `.incbin`'d from the ROM). What each command
//! does follows mupen64plus-rsp-hle's `alist_nead.c` and `alist.c` for this game
//! (`alist_process_nead_oot`), checked where it can be against the C that builds the lists.
//! Where this port differs from that HLE, it says so (docs/adr/0025-audio-mixer.md):
//! - `aClearBuffer` clears `size` rounded up to 16 bytes, as `abi.h` documents it (the HLE
//!   clears the bytes asked for);
//! - `aFilter` with `A_INIT` starts from a zero history (the HLE ignores the flag).
//!
//! The resampler's 64 × 4 filter (`RESAMPLE_LUT` in the HLE) is the microcode's data,
//! read from the ROM by the importer (`aspMainData + 0xE0`).

use crate::ram::Ram;

// `abi.h`'s command numbers.
pub const A_SPNOOP: u32 = 0;
pub const A_ADPCM: u32 = 1;
pub const A_CLEARBUFF: u32 = 2;
pub const A_UNK3: u32 = 3;
pub const A_ADDMIXER: u32 = 4;
pub const A_RESAMPLE: u32 = 5;
pub const A_RESAMPLE_ZOH: u32 = 6;
pub const A_FILTER: u32 = 7;
pub const A_SETBUFF: u32 = 8;
pub const A_DUPLICATE: u32 = 9;
pub const A_DMEMMOVE: u32 = 10;
pub const A_LOADADPCM: u32 = 11;
pub const A_MIXER: u32 = 12;
pub const A_INTERLEAVE: u32 = 13;
pub const A_HILOGAIN: u32 = 14;
pub const A_SETLOOP: u32 = 15;
pub const A_INTERL: u32 = 17;
pub const A_ENVSETUP1: u32 = 18;
pub const A_ENVMIXER: u32 = 19;
pub const A_LOADBUFF: u32 = 20;
pub const A_SAVEBUFF: u32 = 21;
pub const A_ENVSETUP2: u32 = 22;
pub const A_S8DEC: u32 = 23;
pub const A_UNK19: u32 = 25;

// `abi.h`'s flags.
pub const A_INIT: u32 = 0x01;
pub const A_CONTINUE: u32 = 0x00;
pub const A_LOOP: u32 = 0x02;

/// One command: `Acmd`'s two words.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Acmd {
    pub w0: u32,
    pub w1: u32,
}

/// `_SHIFTL(v, s, w)`.
#[inline]
fn shiftl(v: u32, s: u32, w: u32) -> u32 {
    (v & ((1u32 << w) - 1)) << s
}

/// The `abi.h` macros, each returning the command it writes.
pub mod abi {
    use super::*;

    pub fn a_adpcm_dec(f: u32, s: u32) -> Acmd {
        Acmd { w0: shiftl(A_ADPCM, 24, 8) | shiftl(f, 16, 8), w1: s }
    }
    pub fn a_clear_buffer(dmem: u32, size: u32) -> Acmd {
        Acmd { w0: shiftl(A_CLEARBUFF, 24, 8) | shiftl(dmem, 0, 24), w1: size }
    }
    /// `aEnvMixer(pkt, dmemi, count, swapLR, x0, x1, x2, x3, m, bits)`.
    #[allow(clippy::too_many_arguments)]
    pub fn a_env_mixer(dmemi: u32, count: u32, swap_lr: u32, x0: u32, x1: u32, x2: u32, x3: u32, m: u32, bits: u32) -> Acmd {
        Acmd { w0: bits | shiftl(dmemi >> 4, 16, 8) | shiftl(count, 8, 8) | shiftl(swap_lr, 4, 1) | shiftl(x0, 3, 1) | shiftl(x1, 2, 1) | shiftl(x2, 1, 1) | shiftl(x3, 0, 1), w1: m }
    }
    pub fn a_interleave(o: u32, l: u32, r: u32, c: u32) -> Acmd {
        Acmd { w0: shiftl(A_INTERLEAVE, 24, 8) | shiftl(c >> 4, 16, 8) | shiftl(o, 0, 16), w1: shiftl(l, 16, 16) | shiftl(r, 0, 16) }
    }
    pub fn a_load_buffer(addr_src: u32, dmem_dest: u32, size: u32) -> Acmd {
        Acmd { w0: shiftl(A_LOADBUFF, 24, 8) | shiftl(size >> 4, 16, 8) | shiftl(dmem_dest, 0, 16), w1: addr_src }
    }
    pub fn a_mix(f: u32, g: u32, i: u32, o: u32) -> Acmd {
        Acmd { w0: shiftl(A_MIXER, 24, 8) | shiftl(f, 16, 8) | shiftl(g, 0, 16), w1: shiftl(i, 16, 16) | shiftl(o, 0, 16) }
    }
    pub fn a_resample(f: u32, p: u32, s: u32) -> Acmd {
        Acmd { w0: shiftl(A_RESAMPLE, 24, 8) | shiftl(f, 16, 8) | shiftl(p, 0, 16), w1: s }
    }
    pub fn a_save_buffer(dmem_src: u32, addr_dest: u32, size: u32) -> Acmd {
        Acmd { w0: shiftl(A_SAVEBUFF, 24, 8) | shiftl(size >> 4, 16, 8) | shiftl(dmem_src, 0, 16), w1: addr_dest }
    }
    pub fn a_set_buffer(f: u32, i: u32, o: u32, c: u32) -> Acmd {
        Acmd { w0: shiftl(A_SETBUFF, 24, 8) | shiftl(f, 16, 8) | shiftl(i, 0, 16), w1: shiftl(o, 16, 16) | shiftl(c, 0, 16) }
    }
    pub fn a_set_loop(a: u32) -> Acmd {
        Acmd { w0: shiftl(A_SETLOOP, 24, 8), w1: a }
    }
    pub fn a_dmem_move(i: u32, o: u32, c: u32) -> Acmd {
        Acmd { w0: shiftl(A_DMEMMOVE, 24, 8) | shiftl(i, 0, 24), w1: shiftl(o, 16, 16) | shiftl(c, 0, 16) }
    }
    pub fn a_load_adpcm(c: u32, d: u32) -> Acmd {
        Acmd { w0: shiftl(A_LOADADPCM, 24, 8) | shiftl(c, 0, 24), w1: d }
    }
    pub fn a_env_setup1(a: u32, b: u32, c: u32, d: u32) -> Acmd {
        Acmd { w0: shiftl(A_ENVSETUP1, 24, 8) | shiftl(a, 16, 8) | shiftl(b, 0, 16), w1: shiftl(c, 16, 16) | shiftl(d, 0, 16) }
    }
    pub fn a_env_setup2(vol_left: u32, vol_right: u32) -> Acmd {
        Acmd { w0: shiftl(A_ENVSETUP2, 24, 8), w1: shiftl(vol_left, 16, 16) | shiftl(vol_right, 0, 16) }
    }
    pub fn a_filter(f: u32, count_or_buf: u32, addr: u32) -> Acmd {
        Acmd { w0: shiftl(A_FILTER, 24, 8) | shiftl(f, 16, 8) | shiftl(count_or_buf, 0, 16), w1: addr }
    }
    pub fn a_duplicate(num_copies: u32, dmem_src: u32, dmem_dest: u32) -> Acmd {
        Acmd { w0: shiftl(A_DUPLICATE, 24, 8) | shiftl(num_copies, 16, 8) | shiftl(dmem_src, 0, 16), w1: shiftl(dmem_dest, 16, 16) | shiftl(0x80, 0, 16) }
    }
    pub fn a_add_mixer(count: u32, dmemi: u32, dmemo: u32, a4: u32) -> Acmd {
        Acmd { w0: shiftl(A_ADDMIXER, 24, 8) | shiftl(count >> 4, 16, 8) | shiftl(a4, 0, 16), w1: shiftl(dmemi, 16, 16) | shiftl(dmemo, 0, 16) }
    }
    pub fn a_resample_zoh(pitch: u32, pitch_accu: u32) -> Acmd {
        Acmd { w0: shiftl(A_RESAMPLE_ZOH, 24, 8) | shiftl(pitch, 0, 16), w1: shiftl(pitch_accu, 0, 16) }
    }
    pub fn a_s8_dec(a1: u32, a2: u32) -> Acmd {
        Acmd { w0: shiftl(A_S8DEC, 24, 8) | shiftl(a1, 16, 8), w1: a2 }
    }

    // The ones `synthesis.c` writes by hand.

    /// `AudioSynth_DMemMove`.
    pub fn audio_synth_dmem_move(dmem_in: u32, dmem_out: u32, size: u32) -> Acmd {
        Acmd { w0: shiftl(A_DMEMMOVE, 24, 8) | shiftl(dmem_in, 0, 24), w1: shiftl(dmem_out, 16, 16) | shiftl(size, 0, 16) }
    }
    /// `AudioSynth_InterL`.
    pub fn audio_synth_inter_l(dmem_in: u32, dmem_out: u32, num_samples: u32) -> Acmd {
        Acmd { w0: shiftl(A_INTERL, 24, 8) | shiftl(num_samples, 0, 16), w1: shiftl(dmem_in, 16, 16) | shiftl(dmem_out, 0, 16) }
    }
    /// `AudioSynth_EnvSetup2`.
    pub fn audio_synth_env_setup2(vol_left: u32, vol_right: u32) -> Acmd {
        Acmd { w0: shiftl(A_ENVSETUP2, 24, 8), w1: shiftl(vol_left, 16, 16) | shiftl(vol_right, 0, 16) }
    }
    /// `AudioSynth_HiLoGain`.
    pub fn audio_synth_hi_lo_gain(gain: u32, dmem_in: u32, dmem_out: u32, size: u32) -> Acmd {
        Acmd { w0: shiftl(A_HILOGAIN, 24, 8) | shiftl(gain, 16, 8) | shiftl(size, 0, 16), w1: shiftl(dmem_in, 16, 16) | shiftl(dmem_out, 0, 16) }
    }
    /// `AudioSynth_UnkCmd19`.
    pub fn audio_synth_unk_cmd19(a1: u32, a2: u32, size: u32, a4: u32) -> Acmd {
        Acmd { w0: shiftl(A_UNK19, 24, 8) | shiftl(a4, 16, 8) | shiftl(size, 0, 16), w1: shiftl(a1, 16, 16) | shiftl(a2, 0, 16) }
    }
    /// `AudioSynth_UnkCmd3`.
    pub fn audio_synth_unk_cmd3(a1: u32, a2: u32, size: u32) -> Acmd {
        Acmd { w0: shiftl(A_UNK3, 24, 8) | shiftl(size, 0, 16), w1: shiftl(a1, 16, 16) | shiftl(a2, 0, 16) }
    }
}

/// `clamp_s16`.
#[inline]
fn clamp_s16(x: i64) -> i16 {
    x.clamp(i16::MIN as i64, i16::MAX as i64) as i16
}

/// The DMEM size: addresses wrap at 4 KB.
pub const DMEM_SIZE: usize = 0x1000;

/// The microcode's state between commands (the HLE's `alist_nead`), and DMEM.
pub struct Rsp {
    pub dmem: Box<[u8; DMEM_SIZE]>,
    /// The ADPCM codebook (`aLoadADPCM`).
    table: [i16; 16 * 8],
    /// `aSetLoop`'s address.
    loop_addr: u32,
    /// `aSetBuffer`'s in, out and count.
    buf_in: u16,
    buf_out: u16,
    buf_count: u16,
    env_values: [u16; 3],
    env_steps: [u16; 3],
    filter_count: u16,
    filter_lut_address: [u32; 2],
    /// The resampler's 4-tap filters, 64 phases (the microcode's data).
    resample_lut: Vec<i16>,
    /// Commands this port doesn't run, seen so far (logged once each).
    unknown: Vec<u32>,
}

impl Rsp {
    /// `resample_lut`: the 256 coefficients at `aspMainData + 0xE0`.
    pub fn new(resample_lut: &[i16]) -> Rsp {
        assert_eq!(resample_lut.len(), 256, "the resampler's table is 64 × 4 coefficients");
        Rsp {
            dmem: Box::new([0; DMEM_SIZE]),
            table: [0; 128],
            loop_addr: 0,
            buf_in: 0,
            buf_out: 0,
            buf_count: 0,
            env_values: [0; 3],
            env_steps: [0; 3],
            filter_count: 0,
            filter_lut_address: [0; 2],
            resample_lut: resample_lut.to_vec(),
            unknown: Vec::new(),
        }
    }

    #[inline]
    fn u8(&self, a: u32) -> u8 {
        self.dmem[(a as usize) & (DMEM_SIZE - 1)]
    }
    #[inline]
    fn set_u8(&mut self, a: u32, v: u8) {
        self.dmem[(a as usize) & (DMEM_SIZE - 1)] = v;
    }
    /// The 16-bit sample at byte address `a`.
    #[inline]
    pub fn s16(&self, a: u32) -> i16 {
        ((self.u8(a) as u16) << 8 | self.u8(a.wrapping_add(1)) as u16) as i16
    }
    #[inline]
    pub fn set_s16(&mut self, a: u32, v: i16) {
        self.set_u8(a, (v as u16 >> 8) as u8);
        self.set_u8(a.wrapping_add(1), v as u8);
    }

    /// Runs a command list (one audio task).
    pub fn run(&mut self, cmds: &[Acmd], ram: &mut Ram) {
        for c in cmds {
            self.command(*c, ram);
        }
    }

    /// One command, as `alist_process_nead_oot` dispatches it (`acmd = (w1 >> 24) & 0x7f`,
    /// 0x18 entries).
    pub fn command(&mut self, c: Acmd, ram: &mut Ram) {
        let (w1, w2) = (c.w0, c.w1);
        let op = (w1 >> 24) & 0x7F;
        match op {
            A_ADPCM => {
                let flags = (w1 >> 16) as u8;
                self.adpcm(ram, flags & 1 != 0, flags & 2 != 0, flags & 4 != 0, w2 & 0xFF_FFFF);
            }
            A_CLEARBUFF => {
                let dmem = w1 as u16 as u32;
                let count = w2 & 0xFFF;
                if count != 0 {
                    // Rounded up to 16 bytes (`abi.h`'s `aClearBuffer`).
                    let count = (count + 15) & !15;
                    for i in 0..count {
                        self.set_u8(dmem + i, 0);
                    }
                }
            }
            A_ADDMIXER => {
                let count = (w1 >> 12) & 0xFF0;
                let (dmemi, dmemo) = (w2 >> 16, w2 & 0xFFFF);
                for i in 0..count / 2 {
                    let v = self.s16(dmemo + 2 * i) as i64 + self.s16(dmemi + 2 * i) as i64;
                    self.set_s16(dmemo + 2 * i, clamp_s16(v));
                }
            }
            A_RESAMPLE => {
                let flags = (w1 >> 16) as u8;
                let pitch = (w1 & 0xFFFF) << 1;
                let count = (self.buf_count as u32 + 0xF) & !0xF;
                self.resample(ram, flags & 1 != 0, self.buf_out as u32, self.buf_in as u32, count, pitch, w2 & 0xFF_FFFF);
            }
            A_RESAMPLE_ZOH => {
                let pitch = (w1 & 0xFFFF) << 1;
                let mut pitch_accu = w2 & 0xFFFF;
                let mut ipos = (self.buf_in as u32) >> 1;
                let mut opos = (self.buf_out as u32) >> 1;
                for _ in 0..(self.buf_count as u32) >> 1 {
                    let v = self.s16(ipos * 2);
                    self.set_s16(opos * 2, v);
                    opos += 1;
                    pitch_accu += pitch;
                    ipos += pitch_accu >> 16;
                    pitch_accu &= 0xFFFF;
                }
            }
            A_FILTER => {
                let flags = (w1 >> 16) as u8;
                let address = w2 & 0xFF_FFFF;
                if flags > 1 {
                    self.filter_count = w1 as u16;
                    self.filter_lut_address[0] = address;
                } else {
                    self.filter_lut_address[1] = address + 0x10;
                    self.filter(ram, flags & A_INIT as u8 != 0, w1 & 0xFFFF, address);
                }
            }
            A_SETBUFF => {
                self.buf_in = w1 as u16;
                self.buf_out = (w2 >> 16) as u16;
                self.buf_count = w2 as u16;
            }
            A_DUPLICATE => {
                let count = (w1 >> 16) & 0xFF;
                let dmemi = w1 & 0xFFFF;
                let mut dmemo = w2 >> 16;
                let mut buf = [0u8; 128];
                for (i, b) in buf.iter_mut().enumerate() {
                    *b = self.u8(dmemi + i as u32);
                }
                for _ in 0..count {
                    for (i, b) in buf.iter().enumerate() {
                        self.set_u8(dmemo + i as u32, *b);
                    }
                    dmemo += 128;
                }
            }
            A_DMEMMOVE => {
                let dmemi = w1 & 0xFFFF;
                let dmemo = w2 >> 16;
                let count = w2 & 0xFFFF;
                if count != 0 {
                    let count = (count + 3) & !3;
                    for i in 0..count {
                        let v = self.u8(dmemi + i);
                        self.set_u8(dmemo + i, v);
                    }
                }
            }
            A_LOADADPCM => {
                let count = (w1 & 0xFFFF) >> 1;
                let address = w2 & 0xFF_FFFF;
                for i in 0..(count as usize).min(self.table.len()) {
                    self.table[i] = ram.s16(address + 2 * i as u32);
                }
            }
            A_MIXER => {
                let count = (w1 >> 12) & 0xFF0;
                let gain = w1 as u16 as i16 as i64;
                let (dmemi, dmemo) = (w2 >> 16, w2 & 0xFFFF);
                for i in 0..count / 2 {
                    let dst = self.s16(dmemo + 2 * i) as i64;
                    let src = self.s16(dmemi + 2 * i) as i64;
                    self.set_s16(dmemo + 2 * i, clamp_s16(dst + ((src * gain) >> 15)));
                }
            }
            A_INTERLEAVE => {
                let count = (w1 >> 12) & 0xFF0;
                let dmemo = w1 & 0xFFFF;
                let (left, right) = (w2 >> 16, w2 & 0xFFFF);
                // Two samples of each side per step, `count / 4` steps: L R L R.
                for i in 0..count / 2 {
                    let l = self.s16(left + 2 * i);
                    let r = self.s16(right + 2 * i);
                    self.set_s16(dmemo + 4 * i, l);
                    self.set_s16(dmemo + 4 * i + 2, r);
                }
            }
            A_HILOGAIN => {
                let gain = (w1 >> 16) as u8 as i8 as i64; // Q4.4, signed
                let count = w1 & 0xFFF;
                let dmem = w2 >> 16;
                for i in 0..count / 2 {
                    let v = self.s16(dmem + 2 * i) as i64;
                    self.set_s16(dmem + 2 * i, clamp_s16((v * gain) >> 4));
                }
            }
            A_SETLOOP => self.loop_addr = w2 & 0xFF_FFFF,
            16 => {
                // NEAD_16: copies `count` blocks of `block_size` bytes in 32-byte steps.
                let count = (w1 >> 16) & 0xFF;
                let (mut dmemi, mut dmemo) = (w1 & 0xFFFF, w2 >> 16);
                let block_size = (w2 & 0xFFFF) as i32;
                let mut blocks = count as i32;
                loop {
                    let mut left = block_size;
                    loop {
                        for i in 0..0x20 {
                            let v = self.u8(dmemi + i);
                            self.set_u8(dmemo + i, v);
                        }
                        left -= 0x20;
                        dmemi += 0x20;
                        dmemo += 0x20;
                        if left <= 0 {
                            break;
                        }
                    }
                    blocks -= 1;
                    if blocks <= 0 {
                        break;
                    }
                }
            }
            A_INTERL => {
                let count = w1 & 0xFFFF;
                let (mut dmemi, mut dmemo) = (w2 >> 16, w2 & 0xFFFF);
                for _ in 0..count {
                    let v = self.s16(dmemi);
                    self.set_s16(dmemo, v);
                    dmemo += 2;
                    dmemi += 4;
                }
            }
            A_ENVSETUP1 => {
                self.env_values[2] = ((w1 >> 8) & 0xFF00) as u16;
                self.env_steps[2] = w1 as u16;
                self.env_steps[0] = (w2 >> 16) as u16;
                self.env_steps[1] = w2 as u16;
            }
            A_ENVMIXER => self.env_mixer(w1, w2),
            A_LOADBUFF | A_SAVEBUFF => {
                let count = (w1 >> 12) & 0xFFF;
                let dmem = (w1 & 0xFFF) & !3;
                let address = (w2 & 0xFF_FFFF) & !7;
                let count = (count + 7) & !7;
                if op == A_LOADBUFF {
                    for i in 0..count {
                        let v = ram.u8(address + i);
                        self.set_u8(dmem + i, v);
                    }
                } else {
                    for i in 0..count {
                        ram.set_u8(address + i, self.u8(dmem + i));
                    }
                }
            }
            A_ENVSETUP2 => {
                self.env_values[0] = (w2 >> 16) as u16;
                self.env_values[1] = w2 as u16;
            }
            _ => {
                // A_SPNOOP (`UNKNOWN` in this microcode's table), A_UNK3 and A_S8DEC
                // (`UNKNOWN`), A_UNK19 (past the table): nothing happens.
                if !self.unknown.contains(&op) {
                    self.unknown.push(op);
                    log::debug!("audio microcode: command {op} isn't run by this microcode ({w1:08X} {w2:08X})");
                }
            }
        }
    }

    /// `alist_adpcm`: decodes `count` bytes' worth of 32-byte output frames from `buf_in` to
    /// `buf_out`, writing the 16 samples it starts from first.
    fn adpcm(&mut self, ram: &mut Ram, init: bool, looped: bool, two_bit: bool, last_frame_address: u32) {
        let mut dmemo = self.buf_out as u32;
        let mut dmemi = self.buf_in as u32;
        let mut count = (self.buf_count as u32 + 0x1F) & !0x1F;
        let mut last = [0i16; 16];
        if !init {
            let a = if looped { self.loop_addr } else { last_frame_address };
            for (i, v) in last.iter_mut().enumerate() {
                *v = ram.s16(a + 2 * i as u32);
            }
        }
        for v in last {
            self.set_s16(dmemo, v);
            dmemo += 2;
        }
        while count != 0 {
            let code = self.u8(dmemi);
            dmemi += 1;
            let scale = (code & 0xF0) >> 4;
            let cb = ((code & 0xF) as usize) << 4;
            let mut frame = [0i16; 16];
            if two_bit {
                let rshift = if scale < 14 { 14 - scale } else { 0 };
                for i in 0..4 {
                    let byte = self.u8(dmemi + i as u32) as u16;
                    for (j, (mask, lshift)) in [(0xC0u16, 8u32), (0x30, 10), (0x0C, 12), (0x03, 14)].into_iter().enumerate() {
                        frame[i * 4 + j] = (((byte & mask) << lshift) as i16) >> rshift;
                    }
                }
                dmemi += 4;
            } else {
                let rshift = if scale < 12 { 12 - scale } else { 0 };
                for i in 0..8 {
                    let byte = self.u8(dmemi + i as u32) as u16;
                    frame[i * 2] = (((byte & 0xF0) << 8) as i16) >> rshift;
                    frame[i * 2 + 1] = (((byte & 0x0F) << 12) as i16) >> rshift;
                }
                dmemi += 8;
            }
            // adpcm_compute_residuals, both halves; the second half's history is the first's
            // last two outputs.
            for half in 0..2 {
                let (l1, l2) = if half == 0 { (last[14], last[15]) } else { (last[6], last[7]) };
                let book1 = &self.table[cb.min(128 - 16)..cb.min(128 - 16) + 8];
                let book2 = &self.table[cb.min(128 - 16) + 8..cb.min(128 - 16) + 16];
                let src = &frame[half * 8..half * 8 + 8];
                for i in 0..8 {
                    let mut accu: i32 = (src[i] as i32) << 11;
                    accu = accu.wrapping_add((book1[i] as i32) * (l1 as i32)).wrapping_add((book2[i] as i32) * (l2 as i32));
                    // rdot(i, book2, src)
                    for k in 0..i {
                        accu = accu.wrapping_add((book2[k] as i32) * (src[i - 1 - k] as i32));
                    }
                    last[half * 8 + i] = clamp_s16((accu >> 11) as i64);
                }
            }
            for v in last {
                self.set_s16(dmemo, v);
                dmemo += 2;
            }
            count -= 32;
        }
        for (i, v) in last.iter().enumerate() {
            ram.set_s16(last_frame_address + 2 * i as u32, *v);
        }
    }

    /// `alist_resample`: `count` bytes of output from the input four samples before `dmemi`
    /// on, stepping `pitch` (Q16.16) per sample through the 4-tap filters.
    #[allow(clippy::too_many_arguments)]
    fn resample(&mut self, ram: &mut Ram, init: bool, dmemo: u32, dmemi: u32, count: u32, pitch: u32, address: u32) {
        let mut ipos = (dmemi >> 1).wrapping_sub(4) & 0xFFFF;
        let mut opos = dmemo >> 1;
        let mut pitch_accu: u32;
        if init {
            for k in 0..4 {
                self.set_s16((ipos + k) * 2, 0);
            }
            pitch_accu = 0;
        } else {
            for k in 0..4 {
                let v = ram.s16(address + 2 * k);
                self.set_s16((ipos + k) * 2, v);
            }
            pitch_accu = ram.u16(address + 8) as u32;
        }
        for _ in 0..count >> 1 {
            let lut = (((pitch_accu & 0xFC00) >> 8) as usize).min(252);
            let mut acc: i64 = 0;
            for k in 0..4 {
                acc += self.s16((ipos + k as u32) * 2) as i64 * self.resample_lut[lut + k] as i64;
            }
            self.set_s16(opos * 2, clamp_s16(acc >> 15));
            opos += 1;
            pitch_accu += pitch;
            ipos += pitch_accu >> 16;
            pitch_accu &= 0xFFFF;
        }
        for k in 0..4 {
            let v = self.s16((ipos + k) * 2);
            ram.set_s16(address + 2 * k, v);
        }
        ram.set_u16(address + 8, pitch_accu as u16);
    }

    /// `ENVMIXER` + `alist_envmix_nead`: the sample at `dmemi` times the left and right volumes
    /// into the dry channels, and those times the reverb volume into the wet ones, eight
    /// samples per volume step.
    fn env_mixer(&mut self, w1: u32, w2: u32) {
        let dmemi = (w1 >> 12) & 0xFF0;
        let mut count = (w1 >> 8) & 0xFF;
        let swap_wet_lr = (w1 >> 4) & 1 != 0;
        let dmem_dl = (w2 >> 20) & 0xFF0;
        let dmem_dr = (w2 >> 12) & 0xFF0;
        let (mut dmem_wl, mut dmem_wr) = ((w2 >> 4) & 0xFF0, (w2 << 4) & 0xFF0);
        let xors = [0i16.wrapping_sub(((w1 & 0x2) >> 1) as i16), 0i16.wrapping_sub((w1 & 0x1) as i16), 0i16.wrapping_sub(((w1 & 0x8) >> 1) as i16), 0i16.wrapping_sub(((w1 & 0x4) >> 1) as i16)];
        count = (count + 7) & !7;
        if swap_wet_lr {
            std::mem::swap(&mut dmem_wl, &mut dmem_wr);
        }
        let mut n = 0u32;
        while count != 0 {
            for i in 0..8 {
                let o = 2 * (n + i);
                let x = self.s16(dmemi + o) as i64;
                let l = (((x * self.env_values[0] as i64) >> 16) as i16) ^ xors[0];
                let r = (((x * self.env_values[1] as i64) >> 16) as i16) ^ xors[1];
                let l2 = (((l as i64 * self.env_values[2] as i64) >> 16) as i16) ^ xors[2];
                let r2 = (((r as i64 * self.env_values[2] as i64) >> 16) as i16) ^ xors[3];
                for (dst, v) in [(dmem_dl, l), (dmem_dr, r), (dmem_wl, l2), (dmem_wr, r2)] {
                    let cur = self.s16(dst + o) as i64;
                    self.set_s16(dst + o, clamp_s16(cur + v as i64));
                }
            }
            for k in 0..3 {
                self.env_values[k] = self.env_values[k].wrapping_add(self.env_steps[k]);
            }
            n += 8;
            count -= 8;
        }
    }

    /// `alist_filter`: an 8-tap FIR over `count` bytes at `dmem` (`filter_count`), its
    /// coefficients averaged with the last call's first, its history the 8 samples before.
    fn filter(&mut self, ram: &mut Ram, init: bool, dmem: u32, address: u32) {
        let count = self.filter_count as u32;
        let (t6, t5) = (self.filter_lut_address[0], self.filter_lut_address[1]);
        let mut c = [0i16; 8];
        for (x, cx) in c.iter_mut().enumerate() {
            let a = ram.s16(t5 + 2 * x as u32) as i32;
            let b = ram.s16(t6 + 2 * x as u32) as i32;
            let v = if init { b } else { (a + b) >> 1 };
            *cx = v as i16;
            ram.set_s16(t5 + 2 * x as u32, *cx);
            ram.set_s16(t6 + 2 * x as u32, *cx);
        }
        let mut prev = [0i16; 8];
        if !init {
            for (i, p) in prev.iter_mut().enumerate() {
                *p = ram.s16(address + 2 * i as u32);
            }
        }
        let mut out = Vec::with_capacity(count as usize / 2);
        let mut x = 0;
        while x < count {
            let mut cur = [0i16; 8];
            for (i, v) in cur.iter_mut().enumerate() {
                *v = self.s16(dmem + x + 2 * i as u32);
            }
            for j in 0..8i32 {
                let mut v: i32 = 0;
                for k in 0..8i32 {
                    let m = j - k;
                    let s = if m >= 0 { cur[m as usize] } else { prev[(8 + m) as usize] };
                    v = v.wrapping_add(c[k as usize] as i32 * s as i32);
                }
                out.push(((v + 0x4000) >> 15) as i16);
            }
            prev = cur;
            x += 16;
        }
        for (i, p) in prev.iter().enumerate() {
            ram.set_s16(address + 2 * i as u32, *p);
        }
        for (i, v) in out.iter().enumerate().take(count as usize / 2) {
            self.set_s16(dmem + 2 * i as u32, *v);
        }
    }
}
