//! The interface's 2D draws (docs/adr/0017-interface-sprites.md): the message box and the HUD
//! draw textured rectangles into `OVERLAY_DISP` (`gDPLoadTextureBlock` then
//! `gSPTextureRectangle`), and a few textured quads with a matrix (the beating heart, the A
//! button).
//!
//! Each texture, with the setup its draw code loads it under (the `sSetupDL` entry, the
//! combiner, the tile's clamp and mirror), is baked once as a *sprite*: a unit quad from (0, 0)
//! to (1, -1) whose texture coordinates cover the whole texture (`SpriteBake`). A rectangle is
//! then that mesh placed by its transform in `DrawLists::overlay_2d`'s space (the 320x240
//! screen centred on 0, y up), with this draw's prim and env colours as dynamic segments
//! (`Sprite`). Every rectangle these draws make covers its whole texture (its size times its
//! `dsdx` step is the texture's width), so the baked texture coordinates are the draw's.
//!
//! The quad sampling differs from the RDP's texture rectangle by up to half a texel at the
//! edges (the RDP steps from the rectangle's corner, the rasteriser samples pixel centres).

use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use glam::{Mat4, Vec3};

use crate::gbi::{Dl, push_vtx, seg};
use crate::pack::{BakeBody, BakeSegment, MeshBake, keys};

/// Where a sprite's texels come from.
#[derive(Debug, Clone, PartialEq)]
pub enum TexSrc {
    /// A texture the XMLs name.
    Symbol { file: String, symbol: String },
    /// Bytes at `offset` in a ROM file (a font glyph, a `message_static` image the draw code
    /// loads with another format than the XML's).
    File { file: String, offset: u32 },
}

/// The draw code's `gDPLoadTextureBlock` (or `_4b` for 4-bit sizes).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Load {
    pub fmt: u32,
    pub siz: u32,
    pub width: u32,
    pub height: u32,
    pub cms: u32,
    pub cmt: u32,
    pub masks: u32,
    pub maskt: u32,
}

impl Load {
    /// A plain load: no mirror, no mask, clamped (`G_TX_NOMIRROR | G_TX_CLAMP`) or wrapped.
    pub const fn new(fmt: u32, siz: u32, width: u32, height: u32, cm: u32) -> Load {
        Load { fmt, siz, width, height, cms: cm, cmt: cm, masks: 0, maskt: 0 }
    }
}

/// A sprite's geometry.
#[derive(Debug, Clone, PartialEq)]
pub enum Quad {
    /// The unit quad, its texture coordinates spanning `s` by `t` texels (the texture's size,
    /// or twice its width for a mirrored textbox).
    Rect { s: u32, t: u32 },
    /// The draw code's own four vertices (`ob`, `tc`), drawn with `gSP1Quadrangle(0, 2, 3, 1, 0)`
    /// under the draw's matrix.
    Vtx([([i32; 3], [i32; 2]); 4]),
}

/// A sprite to bake: its texture, how the draw code loads it, the commands before it, and
/// which colours the draw sets per draw.
#[derive(Debug, Clone, PartialEq)]
pub struct SpriteBake {
    pub name: String,
    pub tex: TexSrc,
    pub load: Load,
    /// Everything the draw code runs before the rectangle, after `Gfx_SetupDL_25Opa` (which the
    /// bake starts from): its `sSetupDL` entry, the combiner, fixed colours.
    pub setup: Dl,
    pub prim: bool,
    pub env: bool,
    pub quad: Quad,
}

// The bake's segments.
const SEG_TEX: u8 = 0x08;
const SEG_VTX: u8 = 0x09;
const SEG_SETUP: u8 = 0x0A;
/// The dynamic prim and env colours (`SegmentValues::prim[0x0B]`, `env[0x0B]`).
pub const SEG_COLOR: u8 = 0x0B;
const SEG_DRAW: u8 = 0x0D;

impl SpriteBake {
    /// The mesh record: the setup, the colours, then the load and the quad.
    pub fn mesh_bake(&self) -> MeshBake {
        let tex = match &self.tex {
            TexSrc::Symbol { file, symbol } => BakeSegment::Texture { file: file.clone(), symbol: symbol.clone() },
            TexSrc::File { file, .. } => BakeSegment::File(file.clone()),
        };
        let offset = match &self.tex {
            TexSrc::Symbol { .. } => 0,
            TexSrc::File { offset, .. } => *offset,
        };
        let l = self.load;
        let mut d = Dl::default();
        let addr = seg(SEG_TEX as u32, offset);
        if l.siz == crate::gbi::G_IM_SIZ_4B {
            d.load_texture_block_4b(addr, l.fmt, l.width, l.height, 0, l.cms, l.cmt, l.masks, l.maskt, 0, 0);
        } else {
            d.load_texture_block(addr, l.fmt, l.siz, l.width, l.height, 0, l.cms, l.cmt, l.masks, l.maskt, 0, 0);
        }
        let mut vtx = Vec::new();
        let white = [255, 255, 255, 255];
        match &self.quad {
            Quad::Rect { s, t } => {
                let (s, t) = (*s as i32 * 32, *t as i32 * 32);
                for (ob, tc) in [([0, 0, 0], [0, 0]), ([1, 0, 0], [s, 0]), ([1, -1, 0], [s, t]), ([0, -1, 0], [0, t])] {
                    push_vtx(&mut vtx, ob, tc, white);
                }
                d.vertex(seg(SEG_VTX as u32, 0), 4, 0);
                d.quad0(0, 1, 2, 3);
            }
            Quad::Vtx(v) => {
                for (ob, tc) in v {
                    push_vtx(&mut vtx, *ob, *tc, white);
                }
                d.vertex(seg(SEG_VTX as u32, 0), 4, 0);
                // gSP1Quadrangle(0, 2, 3, 1, 0): triangles (0, 2, 3) and (0, 3, 1).
                d.quad0(0, 2, 3, 1);
            }
        }
        d.end();
        let mut setup = self.setup.clone();
        setup.end();
        let mut segments = vec![(SEG_TEX, tex), (SEG_VTX, BakeSegment::Bytes(vtx)), (SEG_SETUP, BakeSegment::Commands(setup.0))];
        let mut prelude = vec![SEG_SETUP];
        if self.prim || self.env {
            segments.push((SEG_COLOR, BakeSegment::DynamicColor { env: self.env, prim: self.prim }));
            prelude.push(SEG_COLOR);
        }
        segments.push((SEG_DRAW, BakeSegment::Commands(d.0)));
        prelude.push(SEG_DRAW);
        MeshBake { name: self.name.clone(), object: "gameplay_keep".into(), segments, prelude, body: BakeBody::DLists(Vec::new()) }
    }
}

/// One sprite draw: a baked sprite (`keys::bake(name)`), where it goes, and its colours.
#[derive(Debug, Clone, PartialEq)]
pub struct Sprite {
    pub name: String,
    /// The model matrix in the overlay's space.
    pub transform: Mat4,
    pub prim: Option<[u8; 4]>,
    pub env: Option<[u8; 4]>,
}

impl Sprite {
    /// `gSPTextureRectangle(x0 << 2, y0 << 2, x1 << 2, y1 << 2, ...)`: screen pixels, y down.
    pub fn rect(name: impl Into<String>, x0: f32, y0: f32, x1: f32, y1: f32, prim: Option<[u8; 4]>, env: Option<[u8; 4]>) -> Sprite {
        Sprite { name: name.into(), transform: rect_transform(x0, y0, x1, y1), prim, env }
    }

    pub fn draw_cmd(&self) -> DrawCmd {
        let mut sv = SegmentValues::default();
        sv.prim[SEG_COLOR as usize] = self.prim;
        sv.env[SEG_COLOR as usize] = self.env;
        let params = DrawParams { segments: Some(sv), ..Default::default() };
        DrawCmd { mesh: MeshKey::named(keys::bake(&self.name)), transform: self.transform, bones: Vec::new(), params }
    }
}

/// The unit quad onto the screen rectangle (x0, y0)–(x1, y1) (pixels, y down).
pub fn rect_transform(x0: f32, y0: f32, x1: f32, y1: f32) -> Mat4 {
    Mat4::from_translation(Vec3::new(x0 - 160.0, 120.0 - y0, 0.0)) * Mat4::from_scale(Vec3::new(x1 - x0, y1 - y0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gbi::{G_IM_FMT_I, G_TX_CLAMP};

    #[test]
    fn rect_maps_the_unit_quad_to_the_screen() {
        let m = rect_transform(65.0, 60.0, 77.0, 72.0);
        assert_eq!(m.transform_point3(Vec3::ZERO), Vec3::new(-95.0, 60.0, 0.0));
        assert_eq!(m.transform_point3(Vec3::new(1.0, -1.0, 0.0)), Vec3::new(-83.0, 48.0, 0.0));
    }

    #[test]
    fn bake_layout() {
        let b = SpriteBake {
            name: "t".into(),
            tex: TexSrc::File { file: "nes_font_static".into(), offset: 0x80 },
            load: Load::new(G_IM_FMT_I, crate::gbi::G_IM_SIZ_4B, 16, 16, G_TX_CLAMP),
            setup: Dl::default(),
            prim: true,
            env: false,
            quad: Quad::Rect { s: 16, t: 16 },
        }
        .mesh_bake();
        assert_eq!(b.prelude, vec![SEG_SETUP, SEG_COLOR, SEG_DRAW]);
        let BakeSegment::Bytes(v) = &b.segments[1].1 else { panic!() };
        // The third corner: ob (1, -1, 0), tc (16 << 5, 16 << 5).
        assert_eq!(&v[32..42], &[0, 1, 0xFF, 0xFF, 0, 0, 0, 0, 2, 0]);
    }
}
