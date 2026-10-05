//! Display-list commands written out as data: the F3DEX2 encodings of the `gbi.h` macros that
//! draw code uses, for bakes of lists the game builds itself (docs/adr/0012-actor-bakes.md):
//! the room skyboxes (`crate::skybox`) and the interface's sprites (`crate::sprite`).

/// A segmented address.
pub const fn seg(s: u32, off: u32) -> u32 {
    (s << 24) | off
}

// G_IM_FMT_*, G_IM_SIZ_* (gbi.h).
pub const G_IM_FMT_RGBA: u32 = 0;
pub const G_IM_FMT_CI: u32 = 2;
pub const G_IM_FMT_IA: u32 = 3;
pub const G_IM_FMT_I: u32 = 4;
pub const G_IM_SIZ_4B: u32 = 0;
pub const G_IM_SIZ_8B: u32 = 1;
pub const G_IM_SIZ_16B: u32 = 2;
pub const G_IM_SIZ_32B: u32 = 3;

// G_TX_* (gbi.h).
pub const G_TX_NOMIRROR: u32 = 0;
pub const G_TX_WRAP: u32 = 0;
pub const G_TX_MIRROR: u32 = 1;
pub const G_TX_CLAMP: u32 = 2;
pub const G_TX_NOMASK: u32 = 0;
pub const G_TX_NOLOD: u32 = 0;
pub const G_TX_LOADTILE: u32 = 7;
pub const G_TX_RENDERTILE: u32 = 0;

// G_CCMUX_* and G_ACMUX_* as `gsDPSetCombineLERP` takes them, per slot.
/// Colour a and b slots.
pub mod cc_ab {
    pub const COMBINED: u32 = 0;
    pub const TEXEL0: u32 = 1;
    pub const PRIMITIVE: u32 = 3;
    pub const SHADE: u32 = 4;
    pub const ENVIRONMENT: u32 = 5;
    pub const ONE: u32 = 6;
    pub const ZERO: u32 = 15;
}
/// The colour c slot.
pub mod cc_c {
    pub const COMBINED: u32 = 0;
    pub const TEXEL0: u32 = 1;
    pub const PRIMITIVE: u32 = 3;
    pub const SHADE: u32 = 4;
    pub const ENVIRONMENT: u32 = 5;
    pub const ZERO: u32 = 31;
}
/// The colour d slot.
pub mod cc_d {
    pub const COMBINED: u32 = 0;
    pub const TEXEL0: u32 = 1;
    pub const PRIMITIVE: u32 = 3;
    pub const SHADE: u32 = 4;
    pub const ENVIRONMENT: u32 = 5;
    pub const ONE: u32 = 6;
    pub const ZERO: u32 = 7;
}
/// The alpha slots (a, b, d; c differs only in 0 = LOD_FRACTION, and 7 = 0 in every slot).
pub mod ac {
    pub const COMBINED: u32 = 0;
    pub const TEXEL0: u32 = 1;
    pub const PRIMITIVE: u32 = 3;
    pub const SHADE: u32 = 4;
    pub const ENVIRONMENT: u32 = 5;
    pub const ONE: u32 = 6;
    pub const ZERO: u32 = 7;
}

/// A display list being written: `(w0, w1)` pairs.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Dl(pub Vec<(u32, u32)>);

impl Dl {
    pub fn pipe_sync(&mut self) {
        self.0.push((0xE700_0000, 0));
    }
    pub fn tile_sync(&mut self) {
        self.0.push((0xE800_0000, 0));
    }
    pub fn load_sync(&mut self) {
        self.0.push((0xE600_0000, 0));
    }
    pub fn end(&mut self) {
        self.0.push((0xDF00_0000, 0));
    }
    pub fn display_list(&mut self, addr: u32) {
        self.0.push((0xDE00_0000, addr));
    }
    /// Appends `cmds` as they are (a setup list written out elsewhere).
    pub fn extend(&mut self, cmds: &[(u32, u32)]) {
        self.0.extend_from_slice(cmds);
    }
    /// `gSPVertex(v, n, v0)`.
    pub fn vertex(&mut self, addr: u32, n: u32, v0: u32) {
        self.0.push((0x0100_0000 | (n << 12) | ((v0 + n) << 1), addr));
    }
    /// `gSPCullDisplayList(vstart, vend)`.
    pub fn cull_dl(&mut self, vstart: u32, vend: u32) {
        self.0.push((0x0300_0000 | (vstart * 2), vend * 2));
    }
    /// `gSP1Quadrangle(v0, v1, v2, v3, 3)`: triangles (v3, v0, v1) and (v3, v1, v2).
    pub fn quad3(&mut self, v0: u32, v1: u32, v2: u32, v3: u32) {
        let tri = |a: u32, b: u32, c: u32| ((a * 2) << 16) | ((b * 2) << 8) | (c * 2);
        self.0.push((0x0700_0000 | tri(v3, v0, v1), tri(v3, v1, v2)));
    }
    /// `gSP1Quadrangle(v0, v1, v2, v3, 0)`: triangles (v0, v1, v2) and (v0, v2, v3).
    pub fn quad0(&mut self, v0: u32, v1: u32, v2: u32, v3: u32) {
        let tri = |a: u32, b: u32, c: u32| ((a * 2) << 16) | ((b * 2) << 8) | (c * 2);
        self.0.push((0x0700_0000 | tri(v0, v1, v2), tri(v0, v2, v3)));
    }
    /// `gDPSetTextureImage(fmt, siz, width, img)`.
    pub fn set_timg(&mut self, fmt: u32, siz: u32, width: u32, addr: u32) {
        self.0.push((0xFD00_0000 | (fmt << 21) | (siz << 19) | (width - 1), addr));
    }
    /// `gDPSetTile(fmt, siz, line, tmem, tile, palette, 0...)` (no mirror, mask or shift).
    pub fn set_tile(&mut self, fmt: u32, siz: u32, line: u32, tmem: u32, tile: u32, pal: u32) {
        self.set_tile_full(fmt, siz, line, tmem, tile, pal, 0, 0, 0, 0, 0, 0);
    }
    /// `gDPSetTile(fmt, siz, line, tmem, tile, palette, cmt, maskt, shiftt, cms, masks, shifts)`.
    #[allow(clippy::too_many_arguments)]
    pub fn set_tile_full(&mut self, fmt: u32, siz: u32, line: u32, tmem: u32, tile: u32, pal: u32, cmt: u32, maskt: u32, shiftt: u32, cms: u32, masks: u32, shifts: u32) {
        self.0.push((
            0xF500_0000 | (fmt << 21) | (siz << 19) | (line << 9) | tmem,
            (tile << 24) | (pal << 20) | (cmt << 18) | (maskt << 14) | (shiftt << 10) | (cms << 8) | (masks << 4) | shifts,
        ));
    }
    /// `gDPLoadBlock(tile, uls, ult, lrs, dxt)`.
    pub fn load_block(&mut self, tile: u32, uls: u32, ult: u32, lrs: u32, dxt: u32) {
        self.0.push((0xF300_0000 | (uls << 12) | ult, (tile << 24) | (lrs << 12) | dxt));
    }
    /// `gDPSetTileSize(tile, uls, ult, lrs, lrt)` (10.2 fixed point).
    pub fn set_tile_size(&mut self, tile: u32, uls: u32, ult: u32, lrs: u32, lrt: u32) {
        self.0.push((0xF200_0000 | (uls << 12) | ult, (tile << 24) | (lrs << 12) | lrt));
    }
    /// `gDPLoadTLUT_pal256(dram)`.
    pub fn load_tlut_pal256(&mut self, addr: u32) {
        self.set_timg(0, 2, 1, addr);
        self.tile_sync();
        self.set_tile(0, 0, 0, 256, 7, 0);
        self.load_sync();
        // gDPLoadTLUTCmd(G_TX_LOADTILE, 255).
        self.0.push((0xF000_0000, (7 << 24) | (255 << 14)));
        self.pipe_sync();
    }
    /// `gDPLoadTextureTile(timg, G_IM_FMT_CI, G_IM_SIZ_8b, width, 0, uls, ult, lrs, lrt, 0,
    /// G_TX_NOMIRROR | G_TX_WRAP, same, G_TX_NOMASK, same, G_TX_NOLOD, same)`.
    pub fn load_texture_tile_ci8(&mut self, addr: u32, width: u32, uls: u32, ult: u32, lrs: u32, lrt: u32) {
        let (fmt, siz) = (2, 1);
        // ((lrs - uls + 1) * G_IM_SIZ_8b_TILE_BYTES + 7) >> 3, and _LINE_BYTES (both 1).
        let line = ((lrs - uls + 1) + 7) >> 3;
        self.set_timg(fmt, siz, width, addr);
        self.set_tile(fmt, siz, line, 0, 7, 0);
        self.load_sync();
        // gDPLoadTile(G_TX_LOADTILE, uls << 2, ult << 2, lrs << 2, lrt << 2).
        self.0.push((0xF400_0000 | ((uls << 2) << 12) | (ult << 2), (7 << 24) | ((lrs << 2) << 12) | (lrt << 2)));
        self.pipe_sync();
        self.set_tile(fmt, siz, line, 0, 0, 0);
        // gDPSetTileSize(G_TX_RENDERTILE, ...).
        self.0.push((0xF200_0000 | ((uls << 2) << 12) | (ult << 2), ((lrs << 2) << 12) | (lrt << 2)));
    }
    /// `gDPLoadTextureBlock(timg, fmt, siz, width, height, pal, cms, cmt, masks, maskt,
    /// shifts, shiftt)` for an 8-, 16- or 32-bit texture.
    #[allow(clippy::too_many_arguments)]
    pub fn load_texture_block(&mut self, addr: u32, fmt: u32, siz: u32, width: u32, height: u32, pal: u32, cms: u32, cmt: u32, masks: u32, maskt: u32, shifts: u32, shiftt: u32) {
        // siz##_LOAD_BLOCK, siz##_INCR, siz##_SHIFT, siz##_BYTES, siz##_LINE_BYTES.
        let (load_siz, incr, shift, bytes, line_bytes) = match siz {
            G_IM_SIZ_8B => (G_IM_SIZ_16B, 1, 1, 1, 1),
            G_IM_SIZ_16B => (G_IM_SIZ_16B, 0, 0, 2, 2),
            G_IM_SIZ_32B => (G_IM_SIZ_32B, 0, 0, 4, 2),
            _ => panic!("gDPLoadTextureBlock takes 8, 16 or 32 bits (use _4b)"),
        };
        self.set_timg(fmt, load_siz, 1, addr);
        self.set_tile_full(fmt, load_siz, 0, 0, G_TX_LOADTILE, 0, cmt, maskt, shiftt, cms, masks, shifts);
        self.load_sync();
        self.load_block(G_TX_LOADTILE, 0, 0, ((width * height + incr) >> shift) - 1, calc_dxt(width, bytes));
        self.pipe_sync();
        self.set_tile_full(fmt, siz, ((width * line_bytes) + 7) >> 3, 0, G_TX_RENDERTILE, pal, cmt, maskt, shiftt, cms, masks, shifts);
        self.set_tile_size(G_TX_RENDERTILE, 0, 0, (width - 1) << 2, (height - 1) << 2);
    }
    /// `gDPLoadTextureBlock_4b(timg, fmt, width, height, pal, cms, cmt, masks, maskt, shifts,
    /// shiftt)`.
    #[allow(clippy::too_many_arguments)]
    pub fn load_texture_block_4b(&mut self, addr: u32, fmt: u32, width: u32, height: u32, pal: u32, cms: u32, cmt: u32, masks: u32, maskt: u32, shifts: u32, shiftt: u32) {
        self.set_timg(fmt, G_IM_SIZ_16B, 1, addr);
        self.set_tile_full(fmt, G_IM_SIZ_16B, 0, 0, G_TX_LOADTILE, 0, cmt, maskt, shiftt, cms, masks, shifts);
        self.load_sync();
        self.load_block(G_TX_LOADTILE, 0, 0, ((width * height + 3) >> 2) - 1, calc_dxt_4b(width));
        self.pipe_sync();
        self.set_tile_full(fmt, G_IM_SIZ_4B, ((width >> 1) + 7) >> 3, 0, G_TX_RENDERTILE, pal, cmt, maskt, shiftt, cms, masks, shifts);
        self.set_tile_size(G_TX_RENDERTILE, 0, 0, (width - 1) << 2, (height - 1) << 2);
    }
    /// `gDPSetCombineLERP(a0, b0, c0, d0, Aa0, Ab0, Ac0, Ad0, a1, ...)`.
    pub fn combine_lerp(&mut self, c0: [u32; 8], c1: [u32; 8]) {
        let cc = eng_gfx::combiner::encode(c0, c1);
        self.0.push((0xFC00_0000 | (cc >> 32) as u32, cc as u32));
    }
    /// `gDPSetPrimColor(0, 0, r, g, b, a)`.
    pub fn prim_color(&mut self, rgba: [u8; 4]) {
        self.0.push((0xFA00_0000, u32::from_be_bytes(rgba)));
    }
    /// `gDPSetEnvColor(r, g, b, a)`.
    pub fn env_color(&mut self, rgba: [u8; 4]) {
        self.0.push((0xFB00_0000, u32::from_be_bytes(rgba)));
    }
    /// `G_SETOTHERMODE_H` (shift, len, value): `gDPSetTextureFilter` and the like.
    pub fn othermode_h(&mut self, shift: u32, len: u32, value: u32) {
        self.0.push((0xE300_0000 | ((32 - shift - len) << 8) | (len - 1), value));
    }
    /// `G_SETOTHERMODE_L` (shift, len, value): `gDPSetAlphaCompare`, `gDPSetRenderMode`.
    pub fn othermode_l(&mut self, shift: u32, len: u32, value: u32) {
        self.0.push((0xE200_0000 | ((32 - shift - len) << 8) | (len - 1), value));
    }
    /// `gDPSetAlphaCompare(G_AC_NONE)`.
    pub fn alpha_compare_none(&mut self) {
        self.othermode_l(0, 2, 0);
    }
    /// `gDPSetRenderMode(c1, c2)`: `G_SETOTHERMODE_L` from `G_MDSFT_RENDERMODE` (3), 29 bits.
    pub fn render_mode(&mut self, c1: u32, c2: u32) {
        self.othermode_l(3, 29, c1 | c2);
    }
    /// `gDPSetCycleType(type)`: `G_SETOTHERMODE_H` from `G_MDSFT_CYCLETYPE` (20), 2 bits.
    pub fn cycle_type(&mut self, ty: u32) {
        self.othermode_h(20, 2, ty);
    }
    /// `gDPSetColorDither(mode)`: `G_SETOTHERMODE_H` from `G_MDSFT_RGBDITHER` (6), 2 bits.
    pub fn color_dither(&mut self, mode: u32) {
        self.othermode_h(6, 2, mode);
    }
    /// `gSPSetGeometryMode(mode)` (F3DEX2: `gSPGeometryMode(0, mode)`).
    pub fn set_geometry_mode(&mut self, mode: u32) {
        self.0.push((0xD9FF_FFFF, mode));
    }
    /// `gSPClearGeometryMode(mode)` (F3DEX2: `gSPGeometryMode(mode, 0)`).
    pub fn clear_geometry_mode(&mut self, mode: u32) {
        self.0.push((0xD900_0000 | (!mode & 0x00FF_FFFF), 0));
    }
    /// `gSPTexture(0xFFFF, 0xFFFF, 0, G_TX_RENDERTILE, on)`.
    pub fn texture_on(&mut self, on: bool) {
        self.0.push((0xD700_0000 | on as u32 * 2, 0xFFFF_FFFF));
    }
    /// `gSP2Triangles(v00, v01, v02, flag0, v10, v11, v12, flag1)` (F3DEX2: `G_TRI2`, indices
    /// times 2).
    pub fn tri2(&mut self, a: [u32; 3], b: [u32; 3]) {
        self.0.push((0x0600_0000 | (a[0] * 2) << 16 | (a[1] * 2) << 8 | a[2] * 2, (b[0] * 2) << 16 | (b[1] * 2) << 8 | b[2] * 2));
    }
}

// Render modes (`gbi.h`): `G_RM_*` for `gDPSetRenderMode`, first cycle then second.
/// `GBL_c1(G_BL_CLR_FOG, G_BL_A_SHADE, G_BL_CLR_IN, G_BL_1MA)`.
pub const G_RM_FOG_SHADE_A: u32 = 0xC800_0000;
/// `GBL_c1(G_BL_CLR_IN, G_BL_0, G_BL_CLR_IN, G_BL_1)`.
pub const G_RM_PASS: u32 = 0x0C08_0000;
/// `Z_CMP | IM_RD | CVG_DST_SAVE | FORCE_BL | ZMODE_XLU | GBL_c2(G_BL_CLR_IN, G_BL_A_IN,
/// G_BL_CLR_MEM, G_BL_1MA)`.
pub const G_RM_ZB_CLD_SURF2: u32 = 0x0010_4B50;
/// `Z_CMP | IM_RD | CVG_DST_SAVE | FORCE_BL | ZMODE_DEC | GBL_c2(G_BL_CLR_IN, G_BL_A_IN,
/// G_BL_CLR_MEM, G_BL_1MA)`.
pub const G_RM_ZB_OVL_SURF2: u32 = 0x0010_4F50;
/// `AA_EN | Z_CMP | IM_RD | CVG_DST_WRAP | CLR_ON_CVG | FORCE_BL | ZMODE_XLU | GBL_c2(G_BL_CLR_IN,
/// G_BL_A_IN, G_BL_CLR_MEM, G_BL_1MA)`.
pub const G_RM_AA_ZB_XLU_SURF2: u32 = 0x0010_49D8;
/// As `G_RM_AA_ZB_XLU_SURF2` with `ZMODE_DEC`.
pub const G_RM_AA_ZB_XLU_DECAL2: u32 = 0x0010_4DD8;

// Geometry mode bits (F3DEX2).
pub const G_ZBUFFER: u32 = 0x0000_0001;
pub const G_SHADE: u32 = 0x0000_0004;
pub const G_CULL_BACK: u32 = 0x0000_0400;
pub const G_CULL_BOTH: u32 = 0x0000_0600;
pub const G_FOG: u32 = 0x0001_0000;
pub const G_LIGHTING: u32 = 0x0002_0000;
pub const G_TEXTURE_GEN: u32 = 0x0004_0000;
pub const G_TEXTURE_GEN_LINEAR: u32 = 0x0008_0000;
pub const G_SHADING_SMOOTH: u32 = 0x0020_0000;

/// `G_CYC_1CYCLE`, `G_CYC_2CYCLE`; `G_CD_DISABLE`.
pub const G_CYC_1CYCLE: u32 = 0;
pub const G_CYC_2CYCLE: u32 = 1 << 20;
pub const G_CD_DISABLE: u32 = 3 << 6;

/// `CALC_DXT(width, b_txl)`: `TXL2WORDS` is `MAX(1, width * b_txl / 8)`, `G_TX_DXT_FRAC` 11.
fn calc_dxt(width: u32, b_txl: u32) -> u32 {
    let words = (width * b_txl / 8).max(1);
    ((1 << 11) + words - 1) / words
}

/// `CALC_DXT_4b(width)`: `TXL2WORDS_4b` is `MAX(1, width / 16)`.
fn calc_dxt_4b(width: u32) -> u32 {
    let words = (width / 16).max(1);
    ((1 << 11) + words - 1) / words
}

/// `Vtx` (16 bytes, big-endian): `ob`, `flag` 0, `tc` (s10.5), `cn`.
/// `gSPFogPosition(min, max)` after `gDPSetFogColor(color)`: the fog factor's multiplier
/// (`128000 / (max - min)`) and offset (`(500 - min) * 256 / (max - min)`), each the low 16 bits
/// as the command packs them.
pub fn sp_fog_position(color: [u8; 4], min: i32, max: i32) -> eng_gfx::FogOverride {
    let d = max - min;
    eng_gfx::FogOverride { color, multiplier: (128000 / d) as i16, offset: ((500 - min) * 256 / d) as i16 }
}

/// `Gfx_SetFog` (`z_rcp.c`): the fog colour, and its position from `near` to `far` (a far plane
/// of `near + 1` for equal ones). From 1000 there's no fog; above 996 it's clamped there
/// (`gSPFogFactor(0x7FFF, -0x7F00)`); below 0 everything is fogged (`gSPFogFactor(0, 255)`).
///
/// @bug (game): with `far - near` under 4 the multiplier overflows 16 bits (only the case
/// above 996 is caught).
pub fn gfx_set_fog(r: u8, g: u8, b: u8, a: u8, near: i32, mut far: i32) -> eng_gfx::FogOverride {
    if far == near {
        far += 1;
    }
    let color = [r, g, b, a];
    if near >= 1000 {
        eng_gfx::FogOverride { color, multiplier: 0, offset: 0 }
    } else if near > 996 {
        eng_gfx::FogOverride { color, multiplier: 0x7FFF, offset: -0x7F00 }
    } else if near < 0 {
        eng_gfx::FogOverride { color, multiplier: 0, offset: 255 }
    } else {
        sp_fog_position(color, near, far)
    }
}

pub fn push_vtx(out: &mut Vec<u8>, ob: [i32; 3], tc: [i32; 2], cn: [u8; 4]) {
    for v in ob {
        out.extend_from_slice(&(v as i16).to_be_bytes());
    }
    out.extend_from_slice(&0u16.to_be_bytes());
    for v in tc {
        out.extend_from_slice(&(v as i16).to_be_bytes());
    }
    out.extend_from_slice(&cn);
}

/// The `sSetupDL` entries (`z_rcp.c`) the interface draws after, written out from their macros.
pub mod setup_dl {
    use super::*;

    /// `SETUPDL_39`: `gsSPTexture(0xFFFF, 0xFFFF, 0, G_TX_RENDERTILE, G_ON)`,
    /// `gsDPSetCombineMode(G_CC_MODULATEIA_PRIM, G_CC_MODULATEIA_PRIM)`,
    /// `gsDPSetOtherMode(G_AD_DISABLE | G_CD_MAGICSQ | G_CK_NONE | G_TC_FILT | G_TF_BILERP |
    /// G_TT_NONE | G_TL_TILE | G_TD_CLAMP | G_TP_NONE | G_CYC_1CYCLE | G_PM_NPRIMITIVE,
    /// G_AC_THRESHOLD | G_ZS_PIXEL | G_RM_XLU_SURF | G_RM_XLU_SURF2)`,
    /// `gsSPLoadGeometryMode(G_SHADING_SMOOTH)`.
    pub fn setup_dl_39() -> Dl {
        let mut d = Dl::default();
        d.pipe_sync();
        d.0.push((0xD700_0002, 0xFFFF_FFFF));
        d.combine_lerp(MODULATEIA_PRIM, MODULATEIA_PRIM);
        // G_RM_XLU_SURF: IM_RD | CVG_DST_FULL | FORCE_BL | ZMODE_OPA |
        // GBL_c1(G_BL_CLR_IN, G_BL_A_IN, G_BL_CLR_MEM, G_BL_1MA), and _SURF2 in cycle 2.
        d.0.push((0xEF00_2C30, 0x0050_4241));
        d.0.push((0xD900_0000, 0x0020_0000));
        d
    }

    /// `G_CC_MODULATEIA_PRIM`: `TEXEL0, 0, PRIMITIVE, 0, TEXEL0, 0, PRIMITIVE, 0`.
    pub const MODULATEIA_PRIM: [u32; 8] = [cc_ab::TEXEL0, cc_ab::ZERO, cc_c::PRIMITIVE, cc_d::ZERO, ac::TEXEL0, ac::ZERO, ac::PRIMITIVE, ac::ZERO];
    /// `G_CC_MODULATEI_PRIM`: `TEXEL0, 0, PRIMITIVE, 0, 0, 0, 0, PRIMITIVE`.
    pub const MODULATEI_PRIM: [u32; 8] = [cc_ab::TEXEL0, cc_ab::ZERO, cc_c::PRIMITIVE, cc_d::ZERO, ac::ZERO, ac::ZERO, ac::ZERO, ac::PRIMITIVE];

    /// `G_AD_NOTPATTERN | G_CD_MAGICSQ | G_CK_NONE | G_TC_FILT | G_TF_BILERP | G_TT_NONE |
    /// G_TL_TILE | G_TD_CLAMP | G_TP_PERSP | G_CYC_1CYCLE | G_PM_NPRIMITIVE`.
    const OTHERMODE_H_1CYCLE_PERSP: u32 = 0x10 | (6 << 9) | (2 << 12) | (1 << 19);
    /// `gsSPLoadGeometryMode(G_ZBUFFER | G_SHADE | G_CULL_BACK | G_LIGHTING | G_SHADING_SMOOTH)`.
    const GEOMETRY_LIT: (u32, u32) = (0xD900_0000, 0x0000_0001 | 0x0000_0004 | 0x0000_0400 | 0x0002_0000 | 0x0020_0000);

    /// `SETUPDL_26` (`z_rcp.c`): `gsSPTexture(0xFFFF, 0xFFFF, 0, G_TX_RENDERTILE, G_ON)`,
    /// `gsDPSetCombineMode(G_CC_MODULATEI_PRIM, G_CC_MODULATEI_PRIM)`, `gsDPSetOtherMode(..
    /// G_CYC_1CYCLE .., G_AC_NONE | G_ZS_PIXEL | G_RM_AA_ZB_OPA_SURF | G_RM_AA_ZB_OPA_SURF2)`,
    /// `gsSPLoadGeometryMode(G_ZBUFFER | G_SHADE | G_CULL_BACK | G_LIGHTING | G_SHADING_SMOOTH)`.
    pub fn setup_dl_26() -> Dl {
        let mut d = Dl::default();
        d.pipe_sync();
        d.0.push((0xD700_0002, 0xFFFF_FFFF));
        d.combine_lerp(MODULATEI_PRIM, MODULATEI_PRIM);
        // G_RM_AA_ZB_OPA_SURF: AA_EN | Z_CMP | Z_UPD | IM_RD | CVG_DST_CLAMP | ZMODE_OPA |
        // ALPHA_CVG_SEL | GBL_c1(G_BL_CLR_IN, G_BL_A_IN, G_BL_CLR_MEM, G_BL_A_MEM), and _SURF2.
        d.0.push((0xEF00_0000 | OTHERMODE_H_1CYCLE_PERSP, 0x0055_2078));
        d.0.push(GEOMETRY_LIT);
        d
    }

    /// `SETUPDL_20`: `G_CC_MODULATEIA_PRIM`, `gsDPSetOtherMode(.. G_CYC_1CYCLE .., G_AC_THRESHOLD |
    /// G_ZS_PIXEL | G_RM_ZB_CLD_SURF | G_RM_ZB_CLD_SURF2)`, `gsSPLoadGeometryMode(G_ZBUFFER |
    /// G_SHADE | G_SHADING_SMOOTH)`.
    pub fn setup_dl_20() -> Dl {
        let mut d = Dl::default();
        d.pipe_sync();
        d.0.push((0xD700_0002, 0xFFFF_FFFF));
        d.combine_lerp(MODULATEIA_PRIM, MODULATEIA_PRIM);
        // G_RM_ZB_CLD_SURF: Z_CMP | IM_RD | CVG_DST_SAVE | FORCE_BL | ZMODE_XLU |
        // GBL_c1(G_BL_CLR_IN, G_BL_A_IN, G_BL_CLR_MEM, G_BL_1MA), and _SURF2; G_AC_THRESHOLD.
        d.0.push((0xEF00_0000 | OTHERMODE_H_1CYCLE_PERSP, 0x0050_4B51));
        d.0.push((0xD900_0000, 0x0000_0001 | 0x0000_0004 | 0x0020_0000));
        d
    }

    /// `SETUPDL_61`: `gsDPSetCombineLERP(PRIMITIVE, ENVIRONMENT, TEXEL0, ENVIRONMENT, PRIMITIVE,
    /// 0, TEXEL0, 0, ..)` in both cycles, `gsDPSetOtherMode(G_AD_NOISE | G_CD_NOISE | .. |
    /// G_CYC_1CYCLE .., G_AC_NONE | G_ZS_PIXEL | G_RM_ZB_CLD_SURF | G_RM_ZB_CLD_SURF2)`,
    /// `gsSPLoadGeometryMode(G_ZBUFFER | G_SHADE | G_CULL_BACK | G_SHADING_SMOOTH)`.
    pub fn setup_dl_61() -> Dl {
        let mut d = Dl::default();
        d.pipe_sync();
        d.0.push((0xD700_0002, 0xFFFF_FFFF));
        let c = [cc_ab::PRIMITIVE, cc_ab::ENVIRONMENT, cc_c::TEXEL0, cc_d::ENVIRONMENT, ac::PRIMITIVE, ac::ZERO, ac::TEXEL0, ac::ZERO];
        d.combine_lerp(c, c);
        // G_AD_NOISE (2 << 4), G_CD_NOISE (2 << 6), G_TC_FILT, G_TF_BILERP, G_TP_PERSP.
        d.0.push((0xEF00_0000 | 0x20 | 0x80 | (6 << 9) | (2 << 12) | (1 << 19), 0x0050_4B50));
        d.0.push((0xD900_0000, 0x0000_0001 | 0x0000_0004 | 0x0000_0400 | 0x0020_0000));
        d
    }

    /// `G_CC_PASS2`: `0, 0, 0, COMBINED, 0, 0, 0, COMBINED`.
    pub const PASS2: [u32; 8] = [cc_ab::ZERO, cc_ab::ZERO, cc_c::ZERO, cc_d::COMBINED, ac::ZERO, ac::ZERO, ac::ZERO, ac::COMBINED];
    /// `PRIMITIVE, ENVIRONMENT, TEXEL0, ENVIRONMENT, PRIMITIVE, 0, TEXEL0, 0`: a texture tinted
    /// from the env colour to the prim one.
    pub const PRIM_ENV_TEXEL0: [u32; 8] = [cc_ab::PRIMITIVE, cc_ab::ENVIRONMENT, cc_c::TEXEL0, cc_d::ENVIRONMENT, ac::PRIMITIVE, ac::ZERO, ac::TEXEL0, ac::ZERO];

    /// `SETUPDL_0`: `gsSPTexture(.., G_ON)`, `gsDPSetCombineLERP(PRIMITIVE, ENVIRONMENT, TEXEL0,
    /// ENVIRONMENT, PRIMITIVE, 0, TEXEL0, 0, 0, 0, 0, COMBINED, 0, 0, 0, COMBINED)`,
    /// `gsDPSetOtherMode(G_AD_NOISE | G_CD_NOISE | G_CK_NONE | G_TC_FILT | G_TF_BILERP | G_TT_NONE |
    /// G_TL_TILE | G_TD_CLAMP | G_TP_PERSP | G_CYC_2CYCLE | G_PM_NPRIMITIVE, G_AC_NONE | G_ZS_PIXEL |
    /// G_RM_FOG_SHADE_A | G_RM_ZB_CLD_SURF2)`, `gsSPLoadGeometryMode(G_ZBUFFER | G_SHADE |
    /// G_CULL_BACK | G_FOG | G_SHADING_SMOOTH)`.
    pub fn setup_dl_0() -> Dl {
        let mut d = Dl::default();
        d.pipe_sync();
        d.texture_on(true);
        d.combine_lerp(PRIM_ENV_TEXEL0, PASS2);
        d.0.push((0xEF00_0000 | 0x20 | 0x80 | (6 << 9) | (2 << 12) | (1 << 19) | G_CYC_2CYCLE, G_RM_FOG_SHADE_A | G_RM_ZB_CLD_SURF2));
        d.0.push((0xD900_0000, G_ZBUFFER | G_SHADE | G_CULL_BACK | G_FOG | G_SHADING_SMOOTH));
        d
    }

    /// `SETUPDL_38`: `gsSPTexture(.., G_OFF)`, `gsDPSetCombineMode(G_CC_SHADE, G_CC_SHADE)`,
    /// `gsDPSetOtherMode(G_AD_NOTPATTERN | G_CD_MAGICSQ | .. | G_TP_PERSP | G_CYC_1CYCLE |
    /// G_PM_NPRIMITIVE, G_AC_NONE | G_ZS_PIXEL | G_RM_AA_ZB_XLU_SURF | G_RM_AA_ZB_XLU_SURF2)`,
    /// `gsSPLoadGeometryMode(G_ZBUFFER | G_SHADE | G_SHADING_SMOOTH)`.
    pub fn setup_dl_38() -> Dl {
        let mut d = Dl::default();
        d.pipe_sync();
        d.texture_on(false);
        // G_CC_SHADE: 0, 0, 0, SHADE, 0, 0, 0, SHADE.
        let shade = [cc_ab::ZERO, cc_ab::ZERO, cc_c::ZERO, cc_d::SHADE, ac::ZERO, ac::ZERO, ac::ZERO, ac::SHADE];
        d.combine_lerp(shade, shade);
        // G_RM_AA_ZB_XLU_SURF (its first-cycle blender GBL_c1(G_BL_CLR_IN, G_BL_A_IN,
        // G_BL_CLR_MEM, G_BL_1MA) = 0x00400000) | G_RM_AA_ZB_XLU_SURF2.
        d.0.push((0xEF00_0000 | OTHERMODE_H_1CYCLE_PERSP, 0x0040_0000 | G_RM_AA_ZB_XLU_SURF2));
        d.0.push((0xD900_0000, G_ZBUFFER | G_SHADE | G_SHADING_SMOOTH));
        d
    }

    /// `SETUPDL_44`: `gsSPTexture(.., G_ON)`, `gsDPSetCombineMode(G_CC_MODULATEIA_PRIM,
    /// G_CC_PASS2)`, `gsDPSetOtherMode(G_AD_NOTPATTERN | G_CD_MAGICSQ | .. | G_TP_PERSP |
    /// G_CYC_2CYCLE | G_PM_NPRIMITIVE, G_AC_NONE | G_ZS_PIXEL | G_RM_FOG_SHADE_A | G_RM_ZB_OVL_SURF2)`,
    /// `gsSPLoadGeometryMode(G_ZBUFFER | G_SHADE | G_CULL_BACK | G_FOG | G_SHADING_SMOOTH)`.
    pub fn setup_dl_44() -> Dl {
        let mut d = Dl::default();
        d.pipe_sync();
        d.texture_on(true);
        d.combine_lerp(MODULATEIA_PRIM, PASS2);
        d.0.push((0xEF00_0000 | OTHERMODE_H_1CYCLE_PERSP | G_CYC_2CYCLE, G_RM_FOG_SHADE_A | G_RM_ZB_OVL_SURF2));
        d.0.push((0xD900_0000, G_ZBUFFER | G_SHADE | G_CULL_BACK | G_FOG | G_SHADING_SMOOTH));
        d
    }

    /// `SETUPDL_60`: `gsSPTexture(.., G_ON)`, `PRIM_ENV_TEXEL0` in both cycles,
    /// `gsDPSetOtherMode(G_AD_NOTPATTERN | G_CD_MAGICSQ | .. | G_TP_PERSP | G_CYC_1CYCLE |
    /// G_PM_NPRIMITIVE, G_AC_NONE | G_ZS_PIXEL | G_RM_ZB_CLD_SURF | G_RM_ZB_CLD_SURF2)`,
    /// `gsSPLoadGeometryMode(G_ZBUFFER | G_SHADE | G_CULL_BACK | G_SHADING_SMOOTH)`.
    pub fn setup_dl_60() -> Dl {
        let mut d = Dl::default();
        d.pipe_sync();
        d.texture_on(true);
        d.combine_lerp(PRIM_ENV_TEXEL0, PRIM_ENV_TEXEL0);
        d.0.push((0xEF00_0000 | OTHERMODE_H_1CYCLE_PERSP, 0x0050_4B50));
        d.0.push((0xD900_0000, G_ZBUFFER | G_SHADE | G_CULL_BACK | G_SHADING_SMOOTH));
        d
    }

    /// `SETUPDL_5`: as `SETUPDL_26` with `G_RM_AA_ZB_XLU_SURF | G_RM_AA_ZB_XLU_SURF2`.
    pub fn setup_dl_5() -> Dl {
        let mut d = Dl::default();
        d.pipe_sync();
        d.0.push((0xD700_0002, 0xFFFF_FFFF));
        d.combine_lerp(MODULATEI_PRIM, MODULATEI_PRIM);
        // G_RM_AA_ZB_XLU_SURF: AA_EN | Z_CMP | IM_RD | CVG_DST_WRAP | CLR_ON_CVG | FORCE_BL |
        // ZMODE_XLU | GBL_c1(G_BL_CLR_IN, G_BL_A_IN, G_BL_CLR_MEM, G_BL_1MA), and _SURF2.
        d.0.push((0xEF00_0000 | OTHERMODE_H_1CYCLE_PERSP, 0x0050_49D8));
        d.0.push(GEOMETRY_LIT);
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_texture_block_4b_16x16() {
        // gDPLoadTextureBlock_4b(tex, G_IM_FMT_I, 16, 16, 0, G_TX_NOMIRROR | G_TX_CLAMP, same,
        // G_TX_NOMASK, same, G_TX_NOLOD, same): a 128-byte block as 64 16-bit texels, dxt
        // (2048 + 1 - 1) / 1 (TXL2WORDS_4b(16) = 1), the render tile's line (8 + 7) >> 3 = 1.
        let mut d = Dl::default();
        d.load_texture_block_4b(seg(8, 0x80), G_IM_FMT_I, 16, 16, 0, G_TX_CLAMP, G_TX_CLAMP, 0, 0, 0, 0);
        assert_eq!(d.0[0], (0xFD90_0000, 0x0800_0080));
        assert_eq!(d.0[1], (0xF590_0000, 0x0708_0200));
        assert_eq!(d.0[3], (0xF300_0000, 0x0703_F800));
        assert_eq!(d.0[5], (0xF580_0200, 0x0008_0200));
        assert_eq!(d.0[6], (0xF200_0000, 0x0003_C03C));
    }

    #[test]
    fn setup_dl_39_combiner() {
        // G_CC_MODULATEIA_PRIM's standard encoding (gbi.h).
        let d = setup_dl::setup_dl_39();
        assert_eq!(d.0[2], (0xFC11_9623, 0xFF2F_FFFF));
    }
}
