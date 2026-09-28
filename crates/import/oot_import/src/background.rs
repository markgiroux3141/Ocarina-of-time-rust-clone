//! Prerendered room backgrounds (`ROOM_SHAPE_TYPE_IMAGE`, docs/adr/0014-prerendered-backgrounds.md).
//!
//! `Room_DrawBackground2D` (`z_room.c`) runs `Room_DecodeJpeg` on the image the first time it's
//! drawn: data starting with `JPEG_MARKER` (0xFFD8FFE0) is decoded by `Jpeg_Decode` (Huffman on
//! the CPU, the IDCT and colour conversion in the RSP's JPEG microcode) into a 320x240 RGBA16
//! image over the original data. Then `gSPBgRectCopy` copies it to the frame in `G_CYC_COPY`.
//!
//! Here the decode happens at import time with a standard baseline decoder (`zune-jpeg`), and
//! the result is packed to RGBA5551 as the RSP writes it (5 bits per channel, alpha 1). The
//! microcode isn't in the decomp, so its IDCT rounding can't be reproduced: a pixel can differ
//! from the console's by a step of the 5-bit channels. Data that isn't JPEG is the image
//! itself, in its `fmt`/`siz` (with its TLUT for CI).
//!
//! The background becomes a mesh: one quad over the 320x240 screen in the interface's
//! orthographic space (`eng_gfx::DrawParams::screen`), textured in copy mode (point sampled,
//! no depth test or write), so that the room's opaque geometry drawn before it leaves only
//! its depth behind, as on the N64.

use anyhow::{Context, Result};
use eng_gfx::combiner::{self, Combiner};
use eng_gfx::texture::{DecodedImage, G_IM_FMT_CI, G_IM_FMT_RGBA, G_IM_SIZ_16B, WrapMode};
use eng_gfx::{Batch, BlendMode, CullMode, DrawList, Material, NO_BONE, TextureImage, TextureSlot, Vertex};
use glam::{Vec2, Vec3};

/// `JPEG_MARKER` (`z_room.c`).
pub const JPEG_MARKER: u32 = 0xFFD8_FFE0;

/// One background of an image room shape: `RoomShapeImageSingle`'s fields, or one
/// `RoomShapeImageMultiBgEntry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawBackground {
    /// `ROOM_SHAPE_IMAGE_AMOUNT_MULTI`: the bg camera this background is drawn for.
    pub bg_cam_index: Option<u8>,
    pub source: u32,
    pub tlut: u32,
    pub width: u16,
    pub height: u16,
    pub fmt: u8,
    pub siz: u8,
    pub tlut_mode: u16,
    pub tlut_count: u16,
}

/// Decodes a background whose data starts at `data[0]` (the JPEG runs to its end marker), with
/// the TLUT for a CI image.
pub fn decode(bg: &RawBackground, data: &[u8], tlut: Option<&[u8]>) -> Result<DecodedImage> {
    let (w, h) = (bg.width as u32, bg.height as u32);
    let marker = data.get(..4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]));
    if marker != Some(JPEG_MARKER) {
        anyhow::ensure!(bg.fmt == G_IM_FMT_RGBA || bg.fmt == G_IM_FMT_CI, "a raw background in format {}", bg.fmt);
        return Ok(eng_gbi::texture::decode_linear(data, bg.fmt, bg.siz, w, h, tlut));
    }
    let mut dec = zune_jpeg::JpegDecoder::new(zune_core::bytestream::ZCursor::new(data));
    let rgb = dec.decode().context("decoding the JPEG")?;
    let (jw, jh) = dec.dimensions().context("JPEG dimensions")?;
    anyhow::ensure!((jw as u32, jh as u32) == (w, h), "JPEG is {jw}x{jh}, the room shape says {w}x{h}");
    let n = (w * h) as usize;
    anyhow::ensure!(rgb.len() == n * 3, "JPEG decoded to {} bytes, not RGB", rgb.len());
    // Jpeg_Decode's output: RGBA5551, alpha set.
    let mut rgba16 = Vec::with_capacity(n * 2);
    for p in rgb.chunks_exact(3) {
        let v = ((p[0] as u16 >> 3) << 11) | ((p[1] as u16 >> 3) << 6) | ((p[2] as u16 >> 3) << 1) | 1;
        rgba16.extend_from_slice(&v.to_be_bytes());
    }
    Ok(eng_gbi::texture::decode_linear(&rgba16, G_IM_FMT_RGBA, G_IM_SIZ_16B, w, h, None))
}

/// The background as a mesh: a quad over the screen (x −160..160, y 120 at the top to −120, in
/// `View_ApplyOrthoToOverlay`'s space), its image in copy mode (`gSPBgRectCopy`:
/// `G_CYC_COPY`, `G_AC_THRESHOLD`, `G_RM_NOOP`, no z).
pub fn mesh(img: DecodedImage, fmt: u8, siz: u8) -> DrawList {
    let mut d = DrawList::default();
    let hash = img.rgba.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3));
    let (w, h) = (img.width as f32, img.height as f32);
    let image = d.intern_texture(hash, || TextureImage { image: img, fmt, siz, hash, source_segments: 0 });
    // Copy mode has no combiner; as 1-cycle (Room_DrawBackground2D's scaled path):
    // gDPSetCombineLERP(0, 0, 0, TEXEL0, 0, 0, 0, 1, 0, 0, 0, TEXEL0, 0, 0, 0, 1).
    let cc = combiner::encode([15, 15, 31, 1, 7, 7, 7, 6], [15, 15, 31, 1, 7, 7, 7, 6]);
    let mat = Material {
        combiner: Combiner::decode(cc),
        two_cycle: false,
        prim: [0; 4],
        prim_lod_frac: 0,
        env: [0; 4],
        fog: [0; 4],
        blend_color: [0; 4],
        geometry_mode: 0,
        // G_CYC_COPY | G_TT_NONE | G_TL_TILE | G_TD_CLAMP | G_TP_NONE | G_PM_NPRIMITIVE.
        othermode_h: 2 << 20,
        // G_AC_THRESHOLD | G_ZS_PIXEL | G_RM_NOOP | G_RM_NOOP2.
        othermode_l: 1,
        textures: [Some(TextureSlot { image, wrap_s: WrapMode::Clamp, wrap_t: WrapMode::Clamp }), None],
        blend: BlendMode::Opaque,
        cull: CullMode::None,
        depth_test: false,
        depth_write: false,
        decal: false,
        lit: false,
        texgen: false,
        bilinear: false,
        uv_dyn: [None, None],
        env_dyn: None,
        prim_dyn: None,
        fog_blend: false,
    };
    let material = d.intern_material(mat);
    let (x0, x1, y0, y1) = (-w / 2.0, w / 2.0, h / 2.0, -h / 2.0);
    let v = |x: f32, y: f32, u: f32, t: f32| Vertex { bone: NO_BONE, pos: Vec3::new(x, y, 0.0), normal: Vec3::Z, color: [255; 4], uv: [Vec2::new(u, t), Vec2::ZERO] };
    let (tl, tr, br, bl) = (v(x0, y0, 0.0, 0.0), v(x1, y0, 1.0, 0.0), v(x1, y1, 1.0, 1.0), v(x0, y1, 0.0, 1.0));
    d.batches.push(Batch { material, vertices: vec![tl, bl, br, tl, br, tr] });
    d.stats.triangles = 2;
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quad_covers_the_screen_in_copy_mode() {
        let img = DecodedImage { width: 320, height: 240, rgba: vec![0x80; 320 * 240 * 4] };
        let d = mesh(img, G_IM_FMT_RGBA, G_IM_SIZ_16B);
        assert_eq!(d.triangle_count(), 2);
        let m = &d.materials[0];
        assert!(!m.depth_test && !m.depth_write && !m.bilinear && m.blend == BlendMode::Opaque);
        let xs: Vec<f32> = d.batches[0].vertices.iter().map(|v| v.pos.x).collect();
        let ys: Vec<f32> = d.batches[0].vertices.iter().map(|v| v.pos.y).collect();
        assert_eq!((xs.iter().cloned().fold(f32::MAX, f32::min), xs.iter().cloned().fold(f32::MIN, f32::max)), (-160.0, 160.0));
        assert_eq!((ys.iter().cloned().fold(f32::MAX, f32::min), ys.iter().cloned().fold(f32::MIN, f32::max)), (-120.0, 120.0));
    }

    #[test]
    fn raw_rgba16_background() {
        // A 2x1 RGBA5551 image: pure red, then white with alpha 0.
        let bg = RawBackground { bg_cam_index: None, source: 0, tlut: 0, width: 2, height: 1, fmt: G_IM_FMT_RGBA, siz: G_IM_SIZ_16B, tlut_mode: 0, tlut_count: 0 };
        let img = decode(&bg, &[0xF8, 0x01, 0xFF, 0xFE], None).unwrap();
        assert_eq!(&img.rgba[..4], &[255, 0, 0, 255]);
        assert_eq!(&img.rgba[4..8], &[255, 255, 255, 0]);
    }
}
