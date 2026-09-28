//! N64 texture formats and a simplified TMEM.
//!
//! TMEM is modelled as a plain 4 KiB byte array filled by LOADBLOCK/LOADTILE, plus a
//! separate 256-entry TLUT. Real hardware swizzles odd rows on load and un-swizzles on
//! sample; since we do neither, the result is identical for well-formed loads, which is
//! what the game's `gsDPLoadTextureBlock*` macros produce.

pub use eng_gfx::texture::*;

pub const TLUT_NONE: u8 = 0;
pub const TLUT_RGBA16: u8 = 2;
pub const TLUT_IA16: u8 = 3;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TileDescriptor {
    pub fmt: u8,
    pub siz: u8,
    pub line: u16,
    pub tmem: u16,
    pub palette: u8,
    pub cmt: u8,
    pub maskt: u8,
    pub shiftt: u8,
    pub cms: u8,
    pub masks: u8,
    pub shifts: u8,
    // Tile size in 10.2 fixed point.
    pub uls: u16,
    pub ult: u16,
    pub lrs: u16,
    pub lrt: u16,
}

impl TileDescriptor {
    pub fn tile_width(&self) -> u32 {
        ((self.lrs.saturating_sub(self.uls)) as u32 >> 2) + 1
    }
    pub fn tile_height(&self) -> u32 {
        ((self.lrt.saturating_sub(self.ult)) as u32 >> 2) + 1
    }
    /// Dimension of the decoded image along one axis: the wrap period when the axis wraps.
    fn axis_dim(tile_dim: u32, mask: u8, cm: u8) -> u32 {
        if mask != 0 && cm & 2 == 0 { 1 << mask.min(10) } else { tile_dim.max(1) }
    }
    pub fn image_width(&self) -> u32 {
        Self::axis_dim(self.tile_width(), self.masks, self.cms)
    }
    pub fn image_height(&self) -> u32 {
        Self::axis_dim(self.tile_height(), self.maskt, self.cmt)
    }
    pub fn wrap_s(&self) -> WrapMode {
        WrapMode::from_cm(self.cms, self.masks)
    }
    pub fn wrap_t(&self) -> WrapMode {
        WrapMode::from_cm(self.cmt, self.maskt)
    }
    /// Scale factor applied to texture coordinates by the tile's shift field.
    pub fn shift_scale(shift: u8) -> f32 {
        match shift {
            0 => 1.0,
            1..=10 => 1.0 / (1u32 << shift) as f32,
            11..=15 => (1u32 << (16 - shift)) as f32,
            _ => 1.0,
        }
    }
}

pub struct Tmem {
    pub bytes: [u8; 4096],
    pub tlut: [u16; 256],
}

impl Default for Tmem {
    fn default() -> Self {
        Tmem { bytes: [0; 4096], tlut: [0; 256] }
    }
}

impl Tmem {
    pub fn row_stride(tile: &TileDescriptor) -> usize {
        let line_bytes = tile.line as usize * 8;
        // 32-bit textures occupy both TMEM halves on hardware, so `line` counts half a row.
        if tile.siz == G_IM_SIZ_32B { line_bytes * 2 } else { line_bytes }
    }

    pub fn write(&mut self, dst: usize, src: &[u8]) {
        for (i, b) in src.iter().enumerate() {
            self.bytes[(dst + i) & 0xFFF] = *b;
        }
    }

    pub fn load_row(&mut self, tile: &TileDescriptor, row: usize, src: &[u8]) {
        let dst = tile.tmem as usize * 8 + row * Self::row_stride(tile);
        self.write(dst, src);
    }

    /// The TMEM byte ranges (start, length) `decode` reads for this tile, before wrapping at
    /// 4 KiB.
    pub fn footprint(tile: &TileDescriptor) -> Vec<(usize, usize)> {
        let (w, h) = (tile.image_width() as usize, tile.image_height() as usize);
        let stride = Self::row_stride(tile);
        let row_bytes = (w * bits_per_texel(tile.siz)).div_ceil(8).max(1);
        (0..h).map(|t| (tile.tmem as usize * 8 + t * stride, row_bytes)).collect()
    }

    /// Decode the texture a tile descriptor currently points at.
    pub fn decode(&self, tile: &TileDescriptor, tlut_mode: u8) -> DecodedImage {
        let w = tile.image_width();
        let h = tile.image_height();
        let stride = Self::row_stride(tile);
        let base = tile.tmem as usize * 8;
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        let rd8 = |off: usize| self.bytes[off & 0xFFF];
        let rd16 = |off: usize| u16::from_be_bytes([rd8(off), rd8(off + 1)]);
        let pal = |idx: usize| -> [u8; 4] {
            let e = self.tlut[idx & 0xFF];
            if tlut_mode == TLUT_IA16 { ia16(e) } else { rgba16(e) }
        };
        for t in 0..h as usize {
            let row = base + t * stride;
            for s in 0..w as usize {
                let px = match (tile.fmt, tile.siz) {
                    (G_IM_FMT_RGBA, G_IM_SIZ_16B) => rgba16(rd16(row + s * 2)),
                    (G_IM_FMT_RGBA, G_IM_SIZ_32B) => {
                        let o = row + s * 4;
                        [rd8(o), rd8(o + 1), rd8(o + 2), rd8(o + 3)]
                    }
                    (G_IM_FMT_CI, G_IM_SIZ_4B) => {
                        let b = rd8(row + s / 2);
                        let n = if s & 1 == 0 { b >> 4 } else { b & 0xF };
                        pal(tile.palette as usize * 16 + n as usize)
                    }
                    (G_IM_FMT_CI, G_IM_SIZ_8B) => pal(rd8(row + s) as usize),
                    (G_IM_FMT_IA, G_IM_SIZ_4B) => {
                        let b = rd8(row + s / 2);
                        let n = if s & 1 == 0 { b >> 4 } else { b & 0xF };
                        let i = scale3((n >> 1) & 7);
                        [i, i, i, if n & 1 != 0 { 255 } else { 0 }]
                    }
                    (G_IM_FMT_IA, G_IM_SIZ_8B) => {
                        let b = rd8(row + s);
                        let i = (b >> 4) * 17;
                        [i, i, i, (b & 0xF) * 17]
                    }
                    (G_IM_FMT_IA, G_IM_SIZ_16B) => ia16(rd16(row + s * 2)),
                    (G_IM_FMT_I, G_IM_SIZ_4B) | (G_IM_FMT_RGBA, G_IM_SIZ_4B) => {
                        let b = rd8(row + s / 2);
                        let i = (if s & 1 == 0 { b >> 4 } else { b & 0xF }) * 17;
                        [i, i, i, i]
                    }
                    (G_IM_FMT_I, G_IM_SIZ_8B) | (G_IM_FMT_RGBA, G_IM_SIZ_8B) => {
                        let i = rd8(row + s);
                        [i, i, i, i]
                    }
                    (G_IM_FMT_CI, G_IM_SIZ_16B) => rgba16(rd16(row + s * 2)),
                    (G_IM_FMT_I, G_IM_SIZ_16B) => ia16(rd16(row + s * 2)),
                    _ => [255, 0, 255, 255],
                };
                rgba.extend_from_slice(&px);
            }
        }
        DecodedImage { width: w, height: h, rgba }
    }
}

fn scale3(v: u8) -> u8 {
    (v as u32 * 255 / 7) as u8
}

pub fn rgba16(v: u16) -> [u8; 4] {
    let c5 = |x: u16| ((x as u32 * 255 + 15) / 31) as u8;
    [c5((v >> 11) & 0x1F), c5((v >> 6) & 0x1F), c5((v >> 1) & 0x1F), if v & 1 != 0 { 255 } else { 0 }]
}

pub fn ia16(v: u16) -> [u8; 4] {
    let i = (v >> 8) as u8;
    [i, i, i, (v & 0xFF) as u8]
}

/// Decode a texture straight from a DRAM buffer (for dumping textures listed in the XMLs).
pub fn decode_linear(data: &[u8], fmt: u8, siz: u8, width: u32, height: u32, tlut: Option<&[u8]>) -> DecodedImage {
    let mut tmem = Tmem::default();
    let bytes = (width * height) as usize * bits_per_texel(siz) / 8;
    let mut tile = TileDescriptor {
        fmt,
        siz,
        line: ((width as usize * bits_per_texel(siz)).div_ceil(64)) as u16,
        lrs: ((width - 1) << 2) as u16,
        lrt: ((height - 1) << 2) as u16,
        ..Default::default()
    };
    if siz == G_IM_SIZ_32B {
        tile.line = ((width as usize * 2).div_ceil(8)) as u16;
    }
    // Bypass the 4 KiB limit for large dumps by decoding directly.
    if bytes > 4096 {
        let mut big = DecodedImage { width, height, rgba: Vec::with_capacity((width * height * 4) as usize) };
        let rows_per_chunk = (4096 * 8 / (width as usize * bits_per_texel(siz))).max(1);
        let row_bytes = width as usize * bits_per_texel(siz) / 8;
        let mut y = 0;
        while y < height as usize {
            let n = rows_per_chunk.min(height as usize - y);
            let chunk = &data[(y * row_bytes).min(data.len())..((y + n) * row_bytes).min(data.len())];
            let img = decode_linear(chunk, fmt, siz, width, n as u32, tlut);
            big.rgba.extend_from_slice(&img.rgba);
            y += n;
        }
        return big;
    }
    tmem.write(0, &data[..bytes.min(data.len())]);
    if let Some(t) = tlut {
        for (i, c) in t.chunks_exact(2).take(256).enumerate() {
            tmem.tlut[i] = u16::from_be_bytes([c[0], c[1]]);
        }
    }
    tmem.decode(&tile, TLUT_RGBA16)
}
