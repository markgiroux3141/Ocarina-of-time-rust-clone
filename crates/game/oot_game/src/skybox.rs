//! The prerendered room skyboxes of `z_vr_box.c` (`skyboxCtx.unk_140 != 0`): the 360° images
//! of the houses and shops that `Play_Draw` draws around the eye whenever the active camera
//! isn't `CAM_SET_PREREND_FIXED` (whose view has the room's JPEG instead, `crate::room`).
//!
//! `Skybox_Init` builds their display lists once (`func_800AEFC8` → `func_800ADBB0`): each face
//! is a 5x9 grid of vertices in two halves, and each half loads its 64x32 tiles of a 256x256 CI8
//! texture before drawing its quads. `SkyboxDraw_Draw` draws the faces after `SETUPDL_40`,
//! each pair with its own 256-colour palette. Here the same commands are written out and baked
//! into one mesh per skybox (docs/adr/0012-actor-bakes.md), drawn at the eye
//! (`SkyboxDraw_UpdateMatrix` with `skyboxCtx->rot` 0). SETUPDL_40 has no z-buffer, so like the
//! background it paints over the room's opaque geometry and leaves its depth.
//!
//! Which skybox uses which files and how many faces (`Skybox_Setup`) is read from the C by the
//! importer, which bakes `bake(...)` for each (`RoomSkybox`).

use crate::pack::{BakeBody, BakeSegment, MeshBake};

/// A room skybox from `Skybox_Setup`: its `SKYBOX_*` id and name, `unk_140` (1 or 2), and
/// its texture and palette files.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RoomSkybox {
    pub id: u8,
    pub name: String,
    pub unk_140: u8,
    /// `_vr_*_staticSegmentRomStart`'s file, e.g. `vr_LHVR_static`.
    pub static_file: String,
    /// `_vr_*_pal_staticSegmentRomStart`'s file.
    pub pal_file: String,
}

// SKYBOX_* (z64.h) the face counts compare against.
pub const SKYBOX_BAZAAR: u8 = 0x02;
pub const SKYBOX_HOUSE_KAKARIKO: u8 = 0x10;
pub const SKYBOX_BOMBCHU_SHOP: u8 = 0x18;

/// The bake's record name (`pack::keys::bake`).
pub fn bake_name(name: &str) -> String {
    format!("skybox/{name}")
}

/// `D_8012ACA0`: per half, the grid points of its 32 vertices.
const D_8012ACA0: [[u16; 0x20]; 2] = [
    [
        0x00, 0x02, 0x0A, 0x0C, 0x02, 0x04, 0x0C, 0x0E, 0x0A, 0x0C, 0x14, 0x16, 0x0C, 0x0E, 0x16, 0x18, 0x01, 0x03, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0B, 0x0D, 0x0F, 0x10,
        0x11, 0x12, 0x13, 0x15, 0x17,
    ],
    [
        0x14, 0x16, 0x1E, 0x20, 0x16, 0x18, 0x20, 0x22, 0x1E, 0x20, 0x28, 0x2A, 0x20, 0x22, 0x2A, 0x2C, 0x15, 0x17, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1F, 0x21, 0x23, 0x24,
        0x25, 0x26, 0x27, 0x29, 0x2B,
    ],
];
/// `D_8012AD20`, `D_8012AD2C`: the grid's texture coordinates (s10.5).
const D_8012AD20: [i32; 5] = [0x0000, 0x0FC0, 0x1F80, 0x2F40, 0x3F00];
const D_8012AD2C: [i32; 9] = [0x0000, 0x07C0, 0x0F80, 0x1740, 0x1F00, 0x26C0, 0x2E80, 0x3640, 0x3E00];
/// `D_8012AD40`: each quad's four vertices among the 32 loaded.
const D_8012AD40: [u32; 0x40] = [
    0x00, 0x10, 0x13, 0x12, 0x10, 0x01, 0x14, 0x13, 0x01, 0x11, 0x15, 0x14, 0x11, 0x05, 0x16, 0x15, 0x12, 0x13, 0x17, 0x02, 0x13, 0x14, 0x03, 0x17, 0x14, 0x15, 0x18,
    0x03, 0x15, 0x16, 0x07, 0x18, 0x02, 0x17, 0x1A, 0x19, 0x17, 0x03, 0x1B, 0x1A, 0x03, 0x18, 0x1C, 0x1B, 0x18, 0x07, 0x1D, 0x1C, 0x19, 0x1A, 0x1E, 0x0A, 0x1A, 0x1B,
    0x0B, 0x1E, 0x1B, 0x1C, 0x1F, 0x0B, 0x1C, 0x1D, 0x0F, 0x1F,
];
/// `D_8012AC90`: each face's texture in `staticSegments[0]`.
const D_8012AC90: [u32; 4] = [0x00000, 0x10000, 0x20000, 0x30000];
/// `D_8012AEBC`: each face's corner and steps (`unk_0`..`unk_10`).
const D_8012AEBC: [[i32; 5]; 4] = [[-0x7E, 0x7C, -0x7E, 0x3F, -0x1F], [0x7E, 0x7C, -0x7E, 0x3F, -0x1F], [0x7E, 0x7C, 0x7E, -0x3F, -0x1F], [-0x7E, 0x7C, 0x7E, -0x3F, -0x1F]];

// Segments of the bake: SkyboxDraw_Draw's gSPSegment(7) and (9), and the draw's own buffers.
const SEG_STATIC: u32 = 0x07;
const SEG_PALETTES: u32 = 0x09;
const SEG_ROOM_VTX: u32 = 0x0A;
const SEG_DLIST_BUF: u32 = 0x0B;
const SEG_SETUP_DL: u8 = 0x0D;
const SEG_DRAW: u8 = 0x0E;

const fn seg(s: u32, off: u32) -> u32 {
    (s << 24) | off
}

/// Display-list commands (F3DEX2 encodings of the `gbi.h` macros used here).
#[derive(Default)]
struct Dl(Vec<(u32, u32)>);

impl Dl {
    fn pipe_sync(&mut self) {
        self.0.push((0xE700_0000, 0));
    }
    fn tile_sync(&mut self) {
        self.0.push((0xE800_0000, 0));
    }
    fn load_sync(&mut self) {
        self.0.push((0xE600_0000, 0));
    }
    fn end(&mut self) {
        self.0.push((0xDF00_0000, 0));
    }
    fn display_list(&mut self, addr: u32) {
        self.0.push((0xDE00_0000, addr));
    }
    /// `gSPVertex(v, n, v0)`.
    fn vertex(&mut self, addr: u32, n: u32, v0: u32) {
        self.0.push((0x0100_0000 | (n << 12) | ((v0 + n) << 1), addr));
    }
    /// `gSPCullDisplayList(vstart, vend)`.
    fn cull_dl(&mut self, vstart: u32, vend: u32) {
        self.0.push((0x0300_0000 | (vstart * 2), vend * 2));
    }
    /// `gSP1Quadrangle(v0, v1, v2, v3, 3)`: triangles (v3, v0, v1) and (v3, v1, v2).
    fn quad3(&mut self, v0: u32, v1: u32, v2: u32, v3: u32) {
        let tri = |a: u32, b: u32, c: u32| ((a * 2) << 16) | ((b * 2) << 8) | (c * 2);
        self.0.push((0x0700_0000 | tri(v3, v0, v1), tri(v3, v1, v2)));
    }
    /// `gDPSetTextureImage(fmt, siz, width, img)`.
    fn set_timg(&mut self, fmt: u32, siz: u32, width: u32, addr: u32) {
        self.0.push((0xFD00_0000 | (fmt << 21) | (siz << 19) | (width - 1), addr));
    }
    /// `gDPSetTile(fmt, siz, line, tmem, tile, palette, 0...)` (no mirror, mask or shift).
    fn set_tile(&mut self, fmt: u32, siz: u32, line: u32, tmem: u32, tile: u32, pal: u32) {
        self.0.push((0xF500_0000 | (fmt << 21) | (siz << 19) | (line << 9) | tmem, (tile << 24) | (pal << 20)));
    }
    /// `gDPLoadTLUT_pal256(dram)`.
    fn load_tlut_pal256(&mut self, addr: u32) {
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
    fn load_texture_tile_ci8(&mut self, addr: u32, width: u32, uls: u32, ult: u32, lrs: u32, lrt: u32) {
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
    /// `gDPSetOtherModeH`-style `G_SETOTHERMODE_H` (shift, len, value).
    fn othermode_h(&mut self, shift: u32, len: u32, value: u32) {
        self.0.push((0xE300_0000 | ((32 - shift - len) << 8) | (len - 1), value));
    }
}

/// `Vtx` (16 bytes, big-endian): `ob`, `flag` 0, `tc`, `cn` (255, 0, 0; alpha unset, 255 here).
fn push_vtx(out: &mut Vec<u8>, ob: [i32; 3], tc: [i32; 2]) {
    for v in ob {
        out.extend_from_slice(&(v as i16).to_be_bytes());
    }
    out.extend_from_slice(&0u16.to_be_bytes());
    for v in tc {
        out.extend_from_slice(&(v as i16).to_be_bytes());
    }
    out.extend_from_slice(&[255, 0, 0, 255]);
}

/// `func_800ADBB0`: face `arg8`'s two display lists (`dListBuf[arg9]`, `[arg9 + 1]`) and its 64
/// vertices, appended at vertex `arg2`. Returns the next vertex index.
#[allow(clippy::too_many_arguments)]
fn func_800adbb0(vtx: &mut Vec<u8>, dls: &mut [Dl; 8], mut arg2: u32, arg3: i32, arg4: i32, arg5: i32, arg6: i32, arg7: i32, arg8: usize, arg9: usize) -> u32 {
    // The 9x5 grid: x (sp358), y (sp2A4), z (sp1F0), s (sp13C), t (sp88).
    let mut grid = [([0i32; 3], [0i32; 2]); 45];
    let mut k = 0;
    let mut row = arg4;
    for t in D_8012AD2C {
        let mut col = if arg8 == 1 || arg8 == 3 { arg5 } else { arg3 };
        for s in D_8012AD20 {
            let ob = match arg8 {
                // case 0, case 2: z fixed.
                0 | 2 => [col, row, arg5],
                // case 1, case 3: x fixed.
                _ => [arg3, row, col],
            };
            grid[k] = (ob, [s, t]);
            col += arg6;
            k += 1;
        }
        row += arg7;
    }
    let mut phi_a2_4 = 0u32;
    for half in 0..2 {
        let dl = &mut dls[arg9 + half];
        for &index in &D_8012ACA0[half] {
            let (ob, tc) = grid[index as usize];
            push_vtx(vtx, ob, tc);
        }
        dl.vertex(seg(SEG_ROOM_VTX, arg2 * 16), 32, 0);
        arg2 += 0x20;
        dl.cull_dl(0, 15);
        let mut phi_t2_4 = 0usize;
        for _ in 0..4 {
            let mut phi_a0_4 = 0u32;
            for _ in 0..4 {
                dl.load_texture_tile_ci8(seg(SEG_STATIC, D_8012AC90[arg8]), 256, phi_a0_4, phi_a2_4, phi_a0_4 + 0x3F, phi_a2_4 + 0x1F);
                let q = &D_8012AD40[phi_t2_4..phi_t2_4 + 4];
                dl.quad3(q[1], q[2], q[3], q[0]);
                phi_a0_4 += 0x3F;
                phi_t2_4 += 4;
            }
            phi_a2_4 += 0x1F;
        }
        dl.end();
    }
    arg2
}

/// The room skybox as a bake: `Skybox_Init`'s `func_800AEFC8`, then `SkyboxDraw_Draw` with
/// `blend` 0.
pub fn bake(s: &RoomSkybox) -> MeshBake {
    let mut vtx = Vec::new();
    let mut dls: [Dl; 8] = Default::default();
    // func_800AEFC8: two faces for the bazaar and the shops after SKYBOX_HOUSE_KAKARIKO up to
    // SKYBOX_BOMBCHU_SHOP, three for unk_140 2, four otherwise.
    let faces = if s.id == SKYBOX_BAZAAR || (s.id > SKYBOX_HOUSE_KAKARIKO && s.id <= SKYBOX_BOMBCHU_SHOP) {
        2
    } else if s.unk_140 == 2 {
        3
    } else {
        4
    };
    let mut v = 0;
    for (i, f) in D_8012AEBC.iter().enumerate().take(faces) {
        v = func_800adbb0(&mut vtx, &mut dls, v, f[0], f[1], f[2], f[3], f[4], i, 2 * i);
    }
    // dListBuf: the eight lists one after another.
    let mut buf = Vec::new();
    let mut offsets = [0u32; 8];
    for (i, d) in dls.iter().enumerate() {
        offsets[i] = buf.len() as u32 * 8;
        buf.extend_from_slice(&d.0);
    }

    // SkyboxDraw_Draw after Gfx_SetupDL_40Opa (segments 7 and 9 are bindings).
    let mut d = Dl::default();
    // gDPSetPrimColor(0, 0, 0, 0, 0, blend = 0).
    d.0.push((0xFA00_0000, 0));
    // gSPTexture(0x8000, 0x8000, 0, G_TX_RENDERTILE, G_ON).
    d.0.push((0xD700_0002, 0x8000_8000));
    // (gSPMatrix: the draw's transform, the eye.) gDPSetColorDither(G_CD_MAGICSQ),
    // gDPSetTextureFilter(G_TF_BILERP).
    d.othermode_h(6, 2, 0);
    d.othermode_h(12, 2, 2 << 12);
    let pal = |i: u32| seg(SEG_PALETTES, i * 0x200);
    d.load_tlut_pal256(pal(0));
    // gDPSetTextureLUT(G_TT_RGBA16), gDPSetTextureConvert(G_TC_FILT).
    d.othermode_h(14, 2, 2 << 14);
    d.othermode_h(9, 3, 6 << 9);
    let list = |i: usize| seg(SEG_DLIST_BUF, offsets[i]);
    d.display_list(list(0));
    d.display_list(list(1));
    d.pipe_sync();
    d.load_tlut_pal256(pal(1));
    d.display_list(list(2));
    d.display_list(list(3));
    if s.id != SKYBOX_BAZAAR && (s.id <= SKYBOX_HOUSE_KAKARIKO || s.id > SKYBOX_BOMBCHU_SHOP) {
        d.pipe_sync();
        d.load_tlut_pal256(pal(2));
        d.display_list(list(4));
        d.display_list(list(5));
        d.pipe_sync();
        if s.unk_140 != 2 {
            d.load_tlut_pal256(pal(3));
            d.display_list(list(6));
            d.display_list(list(7));
        }
    }
    d.pipe_sync();
    d.end();

    // SETUPDL_40 (z_rcp.c): gsDPPipeSync(), gsSPTexture(0xFFFF, 0xFFFF, 0, G_TX_RENDERTILE, G_ON),
    // gsDPSetCombineLERP(TEXEL1, TEXEL0, PRIMITIVE_ALPHA, TEXEL0, TEXEL1, TEXEL0, PRIMITIVE,
    //                    TEXEL0, 0, 0, 0, COMBINED, 0, 0, 0, COMBINED),
    // gsDPSetOtherMode(G_AD_NOTPATTERN | G_CD_MAGICSQ | G_CK_NONE | G_TC_FILT | G_TF_BILERP |
    //                  G_TT_NONE | G_TL_TILE | G_TD_CLAMP | G_TP_PERSP | G_CYC_2CYCLE | G_PM_NPRIMITIVE,
    //                  G_AC_NONE | G_ZS_PIXEL | G_RM_OPA_SURF | G_RM_OPA_SURF2),
    // gsSPLoadGeometryMode(G_SHADE | G_CULL_FRONT | G_SHADING_SMOOTH).
    let cc = eng_gfx::combiner::encode([2, 1, 10, 1, 2, 1, 3, 1], [15, 15, 31, 0, 7, 7, 7, 0]);
    let setup_dl_40 = vec![(0xE700_0000, 0), (0xD700_0002, 0xFFFF_FFFF), (0xFC00_0000 | (cc >> 32) as u32, cc as u32), (0xEF18_2C10, 0x0F0A_4000), (0xD900_0000, 0x0020_0204)];

    let mut vbytes = vtx;
    vbytes.resize(vbytes.len().max(16), 0);
    MeshBake {
        name: bake_name(&s.name),
        object: "gameplay_keep".into(),
        segments: vec![
            (SEG_STATIC as u8, BakeSegment::File(s.static_file.clone())),
            (SEG_PALETTES as u8, BakeSegment::File(s.pal_file.clone())),
            (SEG_ROOM_VTX as u8, BakeSegment::Bytes(vbytes)),
            (SEG_DLIST_BUF as u8, BakeSegment::Commands(buf)),
            (SEG_SETUP_DL, BakeSegment::Commands(setup_dl_40)),
            (SEG_DRAW, BakeSegment::Commands(d.0)),
        ],
        prelude: vec![SEG_SETUP_DL, SEG_DRAW],
        body: BakeBody::DLists(Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn house() -> RoomSkybox {
        RoomSkybox { id: 7, name: "SKYBOX_HOUSE_LINK".into(), unk_140: 1, static_file: "vr_LHVR_static".into(), pal_file: "vr_LHVR_pal_static".into() }
    }

    #[test]
    fn faces_and_vertices() {
        // Four faces of two halves, 32 vertices each: roomVtx[0..256].
        let b = bake(&house());
        let BakeSegment::Bytes(v) = &b.segments[2].1 else { panic!() };
        assert_eq!(v.len(), 4 * 2 * 32 * 16);
        // Face 0's first vertex: grid point 0, (-0x7E, 0x7C, -0x7E), tc (0, 0).
        assert_eq!(&v[..10], &[0xFF, 0x82, 0x00, 0x7C, 0xFF, 0x82, 0, 0, 0, 0]);
        // The Kokiri shop (0x11) has two faces; Mido's house (unk_140 2) three.
        let shop = RoomSkybox { id: 0x11, unk_140: 1, ..house() };
        let BakeSegment::Bytes(v) = &bake(&shop).segments[2].1 else { panic!() };
        assert_eq!(v.len(), 2 * 2 * 32 * 16);
        let mido = RoomSkybox { id: 0x20, unk_140: 2, ..house() };
        let BakeSegment::Bytes(v) = &bake(&mido).segments[2].1 else { panic!() };
        assert_eq!(v.len(), 3 * 2 * 32 * 16);
    }

    #[test]
    fn second_half_starts_its_tiles_at_row_124() {
        // phi_a2_4 carries over between the halves: the second half's first tile is at t 124.
        let b = bake(&house());
        let BakeSegment::Commands(buf) = &b.segments[3].1 else { panic!() };
        let load_tiles: Vec<u32> = buf.iter().filter(|c| c.0 >> 24 == 0xF4).map(|c| (c.0 & 0xFFF) >> 2).collect();
        // 16 tiles per half, 2 halves per face, 4 faces.
        assert_eq!(load_tiles.len(), 128);
        assert_eq!(&load_tiles[..4], &[0, 0, 0, 0]);
        assert_eq!(load_tiles[16], 124);
    }
}
