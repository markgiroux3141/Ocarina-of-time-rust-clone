//! The memory the audio library runs in: RDRAM, addressed as the CPU addresses it (KSEG0,
//! `0x80000000 | offset`), and the cartridge it DMAs sound data from.
//!
//! The C keeps pointers everywhere: a relocated font holds the absolute addresses of its
//! instruments, samples, loops and books; a channel's script counter points into the sequence;
//! the RSP's command list holds the addresses of each note's decoder state. Porting it with a
//! byte array behind real addresses keeps those as they are: `AudioLoad_RelocateFont` rewrites
//! offsets into addresses, `AudioLoad_RelocateSample`'s `<= AUDIO_RELOCATED_ADDRESS_START` test
//! (`K0BASE`) tells relocated pointers from offsets, and the microcode's DMA sees the physical
//! address (`& 0xFFFFFF`) with the alignment the hardware gives it.
//!
//! Everything here is big-endian, as the N64 stores it.

use std::sync::Arc;

/// `K0BASE` (`ultra64/rcp.h`): the start of KSEG0, where the CPU sees RDRAM cached.
pub const K0BASE: u32 = 0x8000_0000;

/// The RDRAM this port uses (the debug ROM runs with 8 MB; the audio library touches far less).
pub const RAM_SIZE: usize = 0x40_0000;

/// RDRAM: `RAM_SIZE` bytes, addressed by their KSEG0 or physical addresses.
#[derive(Clone)]
pub struct Ram {
    mem: Vec<u8>,
}

impl Default for Ram {
    fn default() -> Self {
        Self::new()
    }
}

impl Ram {
    pub fn new() -> Ram {
        Ram { mem: vec![0; RAM_SIZE] }
    }

    /// The physical offset of an address (`OS_K0_TO_PHYSICAL`; the RSP's DMA takes 24 bits).
    #[inline]
    fn at(addr: u32) -> usize {
        (addr & 0x00FF_FFFF) as usize
    }

    #[inline]
    pub fn u8(&self, addr: u32) -> u8 {
        self.mem.get(Self::at(addr)).copied().unwrap_or(0)
    }
    #[inline]
    pub fn s8(&self, addr: u32) -> i8 {
        self.u8(addr) as i8
    }
    #[inline]
    pub fn u16(&self, addr: u32) -> u16 {
        ((self.u8(addr) as u16) << 8) | self.u8(addr.wrapping_add(1)) as u16
    }
    #[inline]
    pub fn s16(&self, addr: u32) -> i16 {
        self.u16(addr) as i16
    }
    #[inline]
    pub fn u32(&self, addr: u32) -> u32 {
        ((self.u16(addr) as u32) << 16) | self.u16(addr.wrapping_add(2)) as u32
    }
    #[inline]
    pub fn s32(&self, addr: u32) -> i32 {
        self.u32(addr) as i32
    }
    #[inline]
    pub fn f32(&self, addr: u32) -> f32 {
        f32::from_bits(self.u32(addr))
    }

    #[inline]
    pub fn set_u8(&mut self, addr: u32, v: u8) {
        if let Some(b) = self.mem.get_mut(Self::at(addr)) {
            *b = v;
        }
    }
    #[inline]
    pub fn set_u16(&mut self, addr: u32, v: u16) {
        self.set_u8(addr, (v >> 8) as u8);
        self.set_u8(addr.wrapping_add(1), v as u8);
    }
    #[inline]
    pub fn set_s16(&mut self, addr: u32, v: i16) {
        self.set_u16(addr, v as u16);
    }
    #[inline]
    pub fn set_u32(&mut self, addr: u32, v: u32) {
        self.set_u16(addr, (v >> 16) as u16);
        self.set_u16(addr.wrapping_add(2), v as u16);
    }
    #[inline]
    pub fn set_f32(&mut self, addr: u32, v: f32) {
        self.set_u32(addr, v.to_bits());
    }

    /// Copies `out.len()` bytes from `addr` (zeros past the end of RDRAM).
    pub fn read(&self, addr: u32, out: &mut [u8]) {
        for (i, b) in out.iter_mut().enumerate() {
            *b = self.u8(addr.wrapping_add(i as u32));
        }
    }

    pub fn write(&mut self, addr: u32, bytes: &[u8]) {
        let at = Self::at(addr);
        let end = (at + bytes.len()).min(self.mem.len());
        if at < end {
            self.mem[at..end].copy_from_slice(&bytes[..end - at]);
        }
    }

    pub fn fill(&mut self, addr: u32, len: usize, v: u8) {
        let at = Self::at(addr);
        let end = (at + len).min(self.mem.len());
        if at < end {
            self.mem[at..end].fill(v);
        }
    }
}

/// The cartridge: the ROM files the audio library DMAs from (`Audiobank`, `Audioseq`,
/// `Audiotable`), each at its ROM address. A DMA reads ROM bytes; outside the files it reads
/// zeros.
#[derive(Clone, Default)]
pub struct Cart {
    files: Vec<(u32, Arc<Vec<u8>>)>,
}

impl Cart {
    pub fn new() -> Cart {
        Cart::default()
    }

    /// Adds a file at its ROM address.
    pub fn add(&mut self, rom_start: u32, bytes: Arc<Vec<u8>>) {
        self.files.push((rom_start, bytes));
    }

    pub fn read(&self, dev_addr: u32, out: &mut [u8]) {
        out.fill(0);
        for (start, bytes) in &self.files {
            let end = start + bytes.len() as u32;
            let (lo, hi) = (dev_addr.max(*start), (dev_addr + out.len() as u32).min(end));
            if lo < hi {
                let n = (hi - lo) as usize;
                out[(lo - dev_addr) as usize..(lo - dev_addr) as usize + n].copy_from_slice(&bytes[(lo - start) as usize..(lo - start) as usize + n]);
            }
        }
    }

    /// `osPiStartDma` into RDRAM, as the PI does it: `size` bytes from `dev_addr` to `ram_addr`.
    pub fn dma(&self, dev_addr: u32, ram: &mut Ram, ram_addr: u32, size: usize) {
        let mut buf = vec![0u8; size];
        self.read(dev_addr, &mut buf);
        ram.write(ram_addr, &buf);
    }
}
