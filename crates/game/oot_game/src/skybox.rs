//! The prerendered room skyboxes of `z_vr_box.c` (`skyboxCtx.drawType != 0`): the 360° images
//! of the houses and shops that `Play_Draw` draws around the eye whenever the active camera
//! isn't `CAM_SET_PREREND_FIXED` (whose view has the room's JPEG instead, `crate::room`).
//!
//! `Skybox_Init` builds their display lists once (`Skybox_Calculate256` → `Skybox_CalculateFace256`): each face
//! is a 5x9 grid of vertices in two halves, and each half loads its 64x32 tiles of a 256x256 CI8
//! texture before drawing its quads. `Skybox_Draw` draws the faces after `SETUPDL_40`,
//! each pair with its own 256-colour palette. Here the same commands are written out and baked
//! into one mesh per skybox (docs/adr/0012-actor-bakes.md), drawn at the eye
//! (`Skybox_UpdateMatrix` with `skyboxCtx->rot` 0). SETUPDL_40 has no z-buffer, so like the
//! background it paints over the room's opaque geometry and leaves its depth.
//!
//! Which skybox uses which files and how many faces (`Skybox_Setup`) is read from the C by the
//! importer, which bakes `bake(...)` for each (`RoomSkybox`).
//!
//! The outdoor skies (`SKYBOX_DRAW_128`: `SKYBOX_NORMAL_SKY`, `SKYBOX_CUTSCENE_MAP`,
//! `SKYBOX_OVERCAST_SUNSET`) are 128x64 side faces and a 128x128 top (and, in the cutscene map, a
//! bottom), two CI8 textures blended by the primitive alpha (`SETUPDL_40`'s combiner), their two
//! palettes in the halves of one 256-colour TLUT (`Skybox_CalculateFace128`, `Skybox_Draw`). They
//! are drawn first, before the rooms, around the eye, turned by `skyboxCtx.rot`. The normal sky
//! has a bake per pair of `gNormalSkyFiles` its time-based configs show together (`bake_128`,
//! `normal_sky_pairs`); which pair is loaded follows `Skybox_Setup` and `Environment_UpdateSkybox`'s
//! loads (`SkyboxContext`), its blend `envCtx.skyboxBlend` (docs/adr/0054).

use crate::gbi::{Dl, seg};
use crate::pack::{BakeBody, BakeSegment, MeshBake};

/// A room skybox from `Skybox_Setup`: its `SKYBOX_*` id and name, `drawType` (1 or 2), and
/// its texture and palette files.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RoomSkybox {
    pub id: u8,
    pub name: String,
    pub draw_type: u8,
    /// `_vr_*_staticSegmentRomStart`'s file, e.g. `vr_LHVR_static`.
    pub static_file: String,
    /// `_vr_*_pal_staticSegmentRomStart`'s file.
    pub pal_file: String,
}

// SKYBOX_* (skybox.h).
pub const SKYBOX_NONE: u8 = 0x00;
pub const SKYBOX_NORMAL_SKY: u8 = 0x01;
pub const SKYBOX_OVERCAST_SUNSET: u8 = 0x03;
pub const SKYBOX_CUTSCENE_MAP: u8 = 0x05;
pub const SKYBOX_UNSET_1D: u8 = 0x1D;
/// `SKYBOX_DRAW_128` (`skyboxCtx.drawType`).
pub const SKYBOX_DRAW_128: u8 = 0;
pub const SKYBOX_BAZAAR: u8 = 0x02;
pub const SKYBOX_HOUSE_KAKARIKO: u8 = 0x10;
pub const SKYBOX_BOMBCHU_SHOP: u8 = 0x18;

/// The bake's record name (`pack::keys::bake`).
pub fn bake_name(name: &str) -> String {
    format!("skybox/{name}")
}

/// `sSkybox256VtxBufIndices`: per half, the grid points of its 32 vertices.
const S_SKYBOX256_VTX_BUF_INDICES: [[u16; 0x20]; 2] = [
    [
        0x00, 0x02, 0x0A, 0x0C, 0x02, 0x04, 0x0C, 0x0E, 0x0A, 0x0C, 0x14, 0x16, 0x0C, 0x0E, 0x16, 0x18, 0x01, 0x03, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0B, 0x0D, 0x0F, 0x10,
        0x11, 0x12, 0x13, 0x15, 0x17,
    ],
    [
        0x14, 0x16, 0x1E, 0x20, 0x16, 0x18, 0x20, 0x22, 0x1E, 0x20, 0x28, 0x2A, 0x20, 0x22, 0x2A, 0x2C, 0x15, 0x17, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1F, 0x21, 0x23, 0x24,
        0x25, 0x26, 0x27, 0x29, 0x2B,
    ],
];
/// `sSkybox256TexSCoords`, `sSkybox256TexTCoords`: the grid's texture coordinates (s10.5).
const S_SKYBOX256_TEX_S_COORDS: [i32; 5] = [0x0000, 0x0FC0, 0x1F80, 0x2F40, 0x3F00];
const S_SKYBOX256_TEX_T_COORDS: [i32; 9] = [0x0000, 0x07C0, 0x0F80, 0x1740, 0x1F00, 0x26C0, 0x2E80, 0x3640, 0x3E00];
/// `sSkybox256VtxIndices`: each quad's four vertices among the 32 loaded.
const S_SKYBOX256_VTX_INDICES: [u32; 0x40] = [
    0x00, 0x10, 0x13, 0x12, 0x10, 0x01, 0x14, 0x13, 0x01, 0x11, 0x15, 0x14, 0x11, 0x05, 0x16, 0x15, 0x12, 0x13, 0x17, 0x02, 0x13, 0x14, 0x03, 0x17, 0x14, 0x15, 0x18,
    0x03, 0x15, 0x16, 0x07, 0x18, 0x02, 0x17, 0x1A, 0x19, 0x17, 0x03, 0x1B, 0x1A, 0x03, 0x18, 0x1C, 0x1B, 0x18, 0x07, 0x1D, 0x1C, 0x19, 0x1A, 0x1E, 0x0A, 0x1A, 0x1B,
    0x0B, 0x1E, 0x1B, 0x1C, 0x1F, 0x0B, 0x1C, 0x1D, 0x0F, 0x1F,
];
/// `sSkybox256TexOffsets`: each face's texture in `staticSegments[0]`.
const S_SKYBOX256_TEX_OFFSETS: [u32; 4] = [0x00000, 0x10000, 0x20000, 0x30000];
/// `sSkybox256FaceParams`: each face's corner and steps (`unk_0`..`unk_10`).
const S_SKYBOX256_FACE_PARAMS: [[i32; 5]; 4] = [[-0x7E, 0x7C, -0x7E, 0x3F, -0x1F], [0x7E, 0x7C, -0x7E, 0x3F, -0x1F], [0x7E, 0x7C, 0x7E, -0x3F, -0x1F], [-0x7E, 0x7C, 0x7E, -0x3F, -0x1F]];

// Segments of the bake: Skybox_Draw's gSPSegment(7) and (9), and the draw's own buffers.
const SEG_STATIC: u32 = 0x07;
const SEG_PALETTES: u32 = 0x09;
const SEG_ROOM_VTX: u32 = 0x0A;
const SEG_DLIST_BUF: u32 = 0x0B;
const SEG_SETUP_DL: u8 = 0x0D;
const SEG_DRAW: u8 = 0x0E;

/// `Vtx` with `cn` (255, 0, 0; alpha unset, 255 here).
fn push_vtx(out: &mut Vec<u8>, ob: [i32; 3], tc: [i32; 2]) {
    crate::gbi::push_vtx(out, ob, tc, [255, 0, 0, 255]);
}

/// `Skybox_CalculateFace256`: face `arg8`'s two display lists (`dListBuf[arg9]`, `[arg9 + 1]`) and its 64
/// vertices, appended at vertex `arg2`. Returns the next vertex index.
#[allow(clippy::too_many_arguments)]
fn calculate_face256(vtx: &mut Vec<u8>, dls: &mut [Dl; 8], mut arg2: u32, arg3: i32, arg4: i32, arg5: i32, arg6: i32, arg7: i32, arg8: usize, arg9: usize) -> u32 {
    // The 9x5 grid: x (sp358), y (sp2A4), z (sp1F0), s (sp13C), t (sp88).
    let mut grid = [([0i32; 3], [0i32; 2]); 45];
    let mut k = 0;
    let mut row = arg4;
    for t in S_SKYBOX256_TEX_T_COORDS {
        let mut col = if arg8 == 1 || arg8 == 3 { arg5 } else { arg3 };
        for s in S_SKYBOX256_TEX_S_COORDS {
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
        for &index in &S_SKYBOX256_VTX_BUF_INDICES[half] {
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
                dl.load_texture_tile_ci8(seg(SEG_STATIC, S_SKYBOX256_TEX_OFFSETS[arg8]), 256, phi_a0_4, phi_a2_4, phi_a0_4 + 0x3F, phi_a2_4 + 0x1F);
                let q = &S_SKYBOX256_VTX_INDICES[phi_t2_4..phi_t2_4 + 4];
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

/// The room skybox as a bake: `Skybox_Init`'s `Skybox_Calculate256`, then `Skybox_Draw` with
/// `blend` 0.
pub fn bake(s: &RoomSkybox) -> MeshBake {
    let mut vtx = Vec::new();
    let mut dls: [Dl; 8] = Default::default();
    // Skybox_Calculate256: two faces for the bazaar and the shops after SKYBOX_HOUSE_KAKARIKO up to
    // SKYBOX_BOMBCHU_SHOP, three for drawType 2, four otherwise.
    let faces = if s.id == SKYBOX_BAZAAR || (s.id > SKYBOX_HOUSE_KAKARIKO && s.id <= SKYBOX_BOMBCHU_SHOP) {
        2
    } else if s.draw_type == 2 {
        3
    } else {
        4
    };
    let mut v = 0;
    for (i, f) in S_SKYBOX256_FACE_PARAMS.iter().enumerate().take(faces) {
        v = calculate_face256(&mut vtx, &mut dls, v, f[0], f[1], f[2], f[3], f[4], i, 2 * i);
    }
    // dListBuf: the eight lists one after another.
    let mut buf = Vec::new();
    let mut offsets = [0u32; 8];
    for (i, d) in dls.iter().enumerate() {
        offsets[i] = buf.len() as u32 * 8;
        buf.extend_from_slice(&d.0);
    }

    // Skybox_Draw after Gfx_SetupDL_40Opa (segments 7 and 9 are bindings).
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
        if s.draw_type != 2 {
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

/// `skyboxCtx`: what the port keeps of `SkyboxContext`: its draw type, its turn, and for the
/// 128 skies which of `gNormalSkyFiles` are in `staticSegments[0]`, `[1]` and which palettes in the
/// two halves of `palettes` (the cutscene map's and the overcast sunset's are fixed files).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SkyboxContext {
    pub skybox_id: u8,
    /// `drawType` (`SKYBOX_DRAW_*`).
    pub draw_type: u8,
    /// `rot`.
    pub rot: [f32; 3],
    /// The normal sky's textures in `staticSegments[0]`, `[1]`.
    pub textures: [u8; 2],
    /// The normal sky's palettes in the first and second halves of `palettes`.
    pub palettes: [u8; 2],
}

impl SkyboxContext {
    /// `Skybox_Init` for the 128 skies (`Skybox_Setup`'s `SKYBOX_NORMAL_SKY` case: the pair the
    /// time-based config (1 in a retained storm outside cutscene layers, else 0) shows at
    /// `skybox_time`, the palettes by the parity of the first; it also sets `envCtx`'s indices and
    /// blend, which `Environment_Init` then resets but for the blend). `draw_type` is a room
    /// skybox's (`RoomSkybox::draw_type`), or `SKYBOX_DRAW_128`. Returns the blend it set.
    pub fn init(skybox_id: u8, draw_type: u8, tables: &crate::env::EnvTables, skybox_time: u16, storm: bool) -> (SkyboxContext, Option<u8>) {
        let mut ctx = SkyboxContext { skybox_id, draw_type, ..Default::default() };
        let mut blend = None;
        if skybox_id == SKYBOX_NORMAL_SKY {
            let config = if storm { 1 } else { 0 };
            if let Some((_, e)) = tables.skybox_entry(config, skybox_time) {
                blend = Some(if e.change_skybox { (crate::env::lerp_weight(e.end_time, e.start_time, skybox_time) * 255.0) as u8 } else { 0 });
                ctx.textures = [e.skybox1_index, e.skybox2_index];
                ctx.palettes = if palette_half(e.skybox1_index) == 0 { [e.skybox1_index, e.skybox2_index] } else { [e.skybox2_index, e.skybox1_index] };
            }
        }
        (ctx, blend)
    }

    /// A load of `Environment_UpdateSkybox`'s, received.
    pub fn receive(&mut self, d: crate::env::SkyboxDma) {
        match d {
            crate::env::SkyboxDma::Texture(n, i) => self.textures[n & 1] = i,
            crate::env::SkyboxDma::Palette(n, i) => self.palettes[n & 1] = i,
        }
    }

    /// `Environment_Update`'s turn of the sky (not while paused): the normal sky by 0.001 a
    /// frame, the cutscene map's by 0.005.
    pub fn turn(&mut self) {
        if self.skybox_id == SKYBOX_NORMAL_SKY {
            self.rot[1] -= 0.001;
        } else if self.skybox_id == SKYBOX_CUTSCENE_MAP {
            self.rot[1] -= 0.005;
        }
    }

    /// The bake `Skybox_Draw` draws this frame for a 128 sky, if it has one: the normal sky's
    /// loaded pair, or the fixed skies'.
    pub fn bake_128_name(&self) -> Option<String> {
        if self.draw_type != SKYBOX_DRAW_128 {
            return None;
        }
        match self.skybox_id {
            SKYBOX_NORMAL_SKY => Some(normal_sky_bake_name(self.textures[0], self.textures[1])),
            SKYBOX_CUTSCENE_MAP => Some(bake_name("SKYBOX_CUTSCENE_MAP")),
            SKYBOX_OVERCAST_SUNSET => Some(bake_name("SKYBOX_OVERCAST_SUNSET")),
            _ => None,
        }
    }

    /// `Skybox_Draw`'s matrix: at the eye, turned by `rot` (x, then y, then z).
    pub fn draw_matrix(&self, eye: glam::Vec3) -> glam::Mat4 {
        glam::Mat4::from_translation(eye) * glam::Mat4::from_rotation_x(self.rot[0]) * glam::Mat4::from_rotation_y(self.rot[1]) * glam::Mat4::from_rotation_z(self.rot[2])
    }
}

/// The palette half a normal sky's texture `i` reads (`(i & 1) ^ ((i & 4) >> 2)`: 0 the first).
pub fn palette_half(i: u8) -> usize {
    if ((i & 1) ^ ((i & 4) >> 2)) != 0 { 0 } else { 1 }
}

/// The normal sky's bake for textures `a` and `b` of `gNormalSkyFiles`.
pub fn normal_sky_bake_name(a: u8, b: u8) -> String {
    bake_name(&format!("SKYBOX_NORMAL_SKY/{a}_{b}"))
}

/// The pairs of `gNormalSkyFiles` the normal sky can show together: any first texture of the
/// clear and stormy configs' entries (0 to 2) with any second. `Environment_UpdateSkybox` loads
/// the first a call before the second, so between two entries (or after a time jump: the Sun's
/// Song, a debug start) the sky holds the new first with the old second for a few frames; a
/// weather change pairs one config's first with the other's second.
pub fn normal_sky_pairs(tables: &crate::env::EnvTables) -> Vec<(u8, u8)> {
    let entries = || tables.skybox_configs.iter().take(3).flatten();
    let mut firsts: Vec<u8> = entries().map(|e| e.skybox1_index).collect();
    let mut seconds: Vec<u8> = entries().map(|e| e.skybox2_index).collect();
    firsts.sort_unstable();
    firsts.dedup();
    seconds.sort_unstable();
    seconds.dedup();
    firsts.iter().flat_map(|&a| seconds.iter().map(move |&b| (a, b))).collect()
}

/// `sSkybox128TexOffsets`: each face's texture in a `staticSegments`' file.
const S_SKYBOX128_TEX_OFFSETS: [u32; 6] = [0, 128 * 64, 128 * 64 * 2, 128 * 64 * 3, 128 * 64 * 4, 128 * 64 * 4 + 128 * 128];
/// `sSkybox128VtxBufIndices`.
const S_SKYBOX128_VTX_BUF_INDICES: [u16; 32] = [0, 2, 10, 12, 2, 4, 12, 14, 10, 12, 20, 22, 12, 14, 22, 24, 1, 3, 5, 6, 7, 8, 9, 11, 13, 15, 16, 17, 18, 19, 21, 23];
/// `sSkybox128TexSCoords`, `sSkybox128TexTCoordsXZ`, `sSkybox128TexTCoords` (s10.5: `TC(62 * n)`).
const S_SKYBOX128_TEX_S_COORDS: [i32; 5] = [0, 62 * 32, 124 * 32, 186 * 32, 248 * 32];
const S_SKYBOX128_TEX_T_COORDS_XZ: [i32; 5] = [0, 62 * 32, 124 * 32, 186 * 32, 248 * 32];
const S_SKYBOX128_TEX_T_COORDS: [i32; 5] = [0, 62 * 32, 124 * 32, 62 * 32, 0];
/// `sSkybox128VtxIndices` (the same as the 256 skies').
const S_SKYBOX128_VTX_INDICES: [u32; 0x40] = S_SKYBOX256_VTX_INDICES;
/// `sSkybox128FaceParams`: `xStart`, `yStart`, `zStart`, `outerIncrVal`, `innerIncrVal`.
const S_SKYBOX128_FACE_PARAMS: [[i32; 5]; 6] = [[-64, 64, -64, 32, -32], [64, 64, 64, -32, -32], [-64, 64, 64, -32, -32], [64, 64, -64, 32, -32], [-64, 64, 64, 32, -32], [-64, -64, -64, 32, 32]];
/// The 128 skies' second texture's segment (`Skybox_Draw`'s `gSPSegment(0x8, staticSegments[1])`).
const SEG_STATIC2: u32 = 0x08;
/// The blend (`gDPSetPrimColor(0, 0, 0, 0, 0, blend)`), a segment the draw sets.
pub const SEG_BLEND: u8 = 0x0F;

/// `Skybox_CalculateFace128`: face `face`'s list (`dListBuf[2 * face]`) and its 32 vertices,
/// appended at vertex `v`. Returns the next vertex index.
#[allow(clippy::too_many_arguments)]
fn calculate_face128(vtx: &mut Vec<u8>, dls: &mut [Dl; 12], mut v: u32, x_start: i32, y_start: i32, z_start: i32, inner_incr_val: i32, outer_incr_val: i32, face: usize) -> u32 {
    let mut grid = [([0i32; 3], [0i32; 2]); 25];
    let mut k = 0;
    let mut outer = if face == 4 || face == 5 { z_start } else { y_start };
    for i in 0..5 {
        let mut inner = if face == 2 || face == 3 { z_start } else { x_start };
        for j in 0..5 {
            let (ob, t) = match face {
                // xy plane.
                0 | 1 => ([inner, outer, z_start], S_SKYBOX128_TEX_T_COORDS[i]),
                // yz plane.
                2 | 3 => ([x_start, outer, inner], S_SKYBOX128_TEX_T_COORDS[i]),
                // xz plane.
                _ => ([inner, y_start, outer], S_SKYBOX128_TEX_T_COORDS_XZ[i]),
            };
            grid[k] = (ob, [S_SKYBOX128_TEX_S_COORDS[j], t]);
            inner += inner_incr_val;
            k += 1;
        }
        outer += outer_incr_val;
    }
    let dl = &mut dls[2 * face];
    for &index in &S_SKYBOX128_VTX_BUF_INDICES {
        let (ob, tc) = grid[index as usize];
        push_vtx(vtx, ob, tc);
    }
    dl.vertex(seg(SEG_ROOM_VTX, v * 16), 32, 0);
    v += 32;
    dl.cull_dl(0, 15);
    let offset = S_SKYBOX128_TEX_OFFSETS[face];
    let quad = |dl: &mut Dl, vtx_idx: usize, uls: u32, ult: u32| {
        dl.load_multi_tile_ci8(seg(SEG_STATIC, offset), 0, 0, 128, uls, ult, uls + 31, ult + 31);
        dl.load_multi_tile_ci8(seg(SEG_STATIC2, offset), 0x80, 1, 128, uls, ult, uls + 31, ult + 31);
        let q = &S_SKYBOX128_VTX_INDICES[vtx_idx..vtx_idx + 4];
        dl.quad3(q[1], q[2], q[3], q[0]);
    };
    let mut vtx_idx = 0;
    if face == 4 || face == 5 {
        // The top and bottom: 128x128, 4x4 tiles.
        let mut ult = 0;
        for _ in 0..4 {
            let mut uls = 0;
            for _ in 0..4 {
                quad(dl, vtx_idx, uls, ult);
                uls += 31;
                vtx_idx += 4;
            }
            ult += 31;
        }
    } else {
        // The sides: 128x64, its rows down, then back up (the T coordinates mirror).
        let mut ult: i32 = 0;
        for _ in 0..2 {
            let mut uls = 0;
            for _ in 0..4 {
                quad(dl, vtx_idx, uls, ult as u32);
                uls += 31;
                vtx_idx += 4;
            }
            ult += 31;
        }
        ult -= 31;
        for _ in 0..2 {
            let mut uls = 0;
            for _ in 0..4 {
                quad(dl, vtx_idx, uls, ult as u32);
                uls += 31;
                vtx_idx += 4;
            }
            ult -= 31;
        }
    }
    dl.end();
    v
}

/// A 128 sky as a bake: `Skybox_Calculate128` (`faces` 5, or 6 for the cutscene map), then
/// `Skybox_Draw`'s 128 path after `SETUPDL_40`: `textures` on segments 7 and 8, `palettes`
/// (the first half's file, then the second's) on 9, the blend dynamic (`SEG_BLEND`).
pub fn bake_128(name: &str, textures: [&str; 2], palettes: [&str; 2], faces: usize) -> MeshBake {
    let mut vtx = Vec::new();
    let mut dls: [Dl; 12] = Default::default();
    let mut v = 0;
    for (i, f) in S_SKYBOX128_FACE_PARAMS.iter().enumerate().take(faces) {
        v = calculate_face128(&mut vtx, &mut dls, v, f[0], f[1], f[2], f[3], f[4], i);
    }
    let mut buf = Vec::new();
    let mut offsets = [0u32; 12];
    for (i, d) in dls.iter().enumerate() {
        offsets[i] = buf.len() as u32 * 8;
        buf.extend_from_slice(&d.0);
    }
    let mut d = Dl::default();
    // gSPTexture(0x8000, 0x8000, 0, G_TX_RENDERTILE, G_ON); (gSPMatrix: the draw's transform.)
    d.0.push((0xD700_0002, 0x8000_8000));
    // gDPSetColorDither(G_CD_MAGICSQ), gDPSetTextureFilter(G_TF_BILERP).
    d.othermode_h(6, 2, 0);
    d.othermode_h(12, 2, 2 << 12);
    d.load_tlut_pal256(seg(SEG_PALETTES, 0));
    // gDPSetTextureLUT(G_TT_RGBA16), gDPSetTextureConvert(G_TC_FILT).
    d.othermode_h(14, 2, 2 << 14);
    d.othermode_h(9, 3, 6 << 9);
    let list = |i: usize| seg(SEG_DLIST_BUF, offsets[i]);
    // -z, +z, -x, +x, +y; -y only in the cutscene map.
    for i in [0, 2, 4, 6, 8] {
        d.display_list(list(i));
    }
    if faces == 6 {
        d.display_list(list(10));
    }
    d.pipe_sync();
    d.end();
    let cc = eng_gfx::combiner::encode([2, 1, 10, 1, 2, 1, 3, 1], [15, 15, 31, 0, 7, 7, 7, 0]);
    let setup_dl_40 = vec![(0xE700_0000, 0), (0xD700_0002, 0xFFFF_FFFF), (0xFC00_0000 | (cc >> 32) as u32, cc as u32), (0xEF18_2C10, 0x0F0A_4000), (0xD900_0000, 0x0020_0204)];
    let mut vbytes = vtx;
    vbytes.resize(vbytes.len().max(16), 0);
    MeshBake {
        name: name.to_string(),
        object: "gameplay_keep".into(),
        segments: vec![
            (SEG_STATIC as u8, BakeSegment::File(textures[0].into())),
            (SEG_STATIC2 as u8, BakeSegment::File(textures[1].into())),
            (SEG_PALETTES as u8, BakeSegment::Files(vec![palettes[0].into(), palettes[1].into()])),
            (SEG_ROOM_VTX as u8, BakeSegment::Bytes(vbytes)),
            (SEG_DLIST_BUF as u8, BakeSegment::Commands(buf)),
            (SEG_SETUP_DL, BakeSegment::Commands(setup_dl_40)),
            // gDPSetPrimColor(0, 0, 0, 0, 0, blend).
            (SEG_BLEND, BakeSegment::Dynamic(vec![(0xFA00_0000, 0x0000_00FF), (0xDF00_0000, 0)])),
            (SEG_DRAW, BakeSegment::Commands(d.0)),
        ],
        prelude: vec![SEG_SETUP_DL, SEG_BLEND, SEG_DRAW],
        body: BakeBody::DLists(Vec::new()),
    }
}

/// Every 128 sky's bake: the cutscene map's (`vr_holy0`, `vr_holy1`, six faces), the overcast
/// sunset's (`vr_cloud2` twice), and the normal sky's pairs (`gNormalSkyFiles`), their palettes
/// in the halves their parity picks.
pub fn bakes_128(tables: &crate::env::EnvTables) -> Vec<MeshBake> {
    let mut v = vec![
        bake_128(&bake_name("SKYBOX_CUTSCENE_MAP"), ["vr_holy0_static", "vr_holy1_static"], ["vr_holy0_pal_static", "vr_holy1_pal_static"], 6),
        bake_128(&bake_name("SKYBOX_OVERCAST_SUNSET"), ["vr_cloud2_static", "vr_cloud2_static"], ["vr_cloud2_pal_static", "vr_cloud2_pal_static"], 5),
    ];
    let files = &tables.normal_sky_files;
    for (a, b) in normal_sky_pairs(tables) {
        let (Some(fa), Some(fb)) = (files.get(a as usize), files.get(b as usize)) else { continue };
        let pals = if palette_half(a) == 0 { [fa.1.as_str(), fb.1.as_str()] } else { [fb.1.as_str(), fa.1.as_str()] };
        v.push(bake_128(&normal_sky_bake_name(a, b), [&fa.0, &fb.0], pals, 5));
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn house() -> RoomSkybox {
        RoomSkybox { id: 7, name: "SKYBOX_HOUSE_LINK".into(), draw_type: 1, static_file: "vr_LHVR_static".into(), pal_file: "vr_LHVR_pal_static".into() }
    }

    #[test]
    fn faces_and_vertices() {
        // Four faces of two halves, 32 vertices each: roomVtx[0..256].
        let b = bake(&house());
        let BakeSegment::Bytes(v) = &b.segments[2].1 else { panic!() };
        assert_eq!(v.len(), 4 * 2 * 32 * 16);
        // Face 0's first vertex: grid point 0, (-0x7E, 0x7C, -0x7E), tc (0, 0).
        assert_eq!(&v[..10], &[0xFF, 0x82, 0x00, 0x7C, 0xFF, 0x82, 0, 0, 0, 0]);
        // The Kokiri shop (0x11) has two faces; Mido's house (drawType 2) three.
        let shop = RoomSkybox { id: 0x11, draw_type: 1, ..house() };
        let BakeSegment::Bytes(v) = &bake(&shop).segments[2].1 else { panic!() };
        assert_eq!(v.len(), 2 * 2 * 32 * 16);
        let mido = RoomSkybox { id: 0x20, draw_type: 2, ..house() };
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

    #[test]
    fn the_128_skies_faces_tiles_and_palette_halves() {
        // Five faces of 32 vertices; six in the cutscene map.
        let b = bake_128("x", ["a", "b"], ["pa", "pb"], 5);
        let BakeSegment::Bytes(v) = &b.segments[3].1 else { panic!() };
        assert_eq!(v.len(), 5 * 32 * 16);
        let b6 = bake_128("x", ["a", "b"], ["pa", "pb"], 6);
        let BakeSegment::Bytes(v6) = &b6.segments[3].1 else { panic!() };
        assert_eq!(v6.len(), 6 * 32 * 16);
        // Face 0's first vertex: grid point 0, (-64, 64, -64), tc (0, 0).
        assert_eq!(&v[..10], &[0xFF, 0xC0, 0x00, 0x40, 0xFF, 0xC0, 0, 0, 0, 0]);
        // The sides 4x2 tiles down and back up (8 + 8), the top 4x4: 4 x 16 + 16 quads, each
        // loading two tiles (tile 0 at TMEM 0, tile 1 at 0x80).
        let BakeSegment::Commands(buf) = &b.segments[4].1 else { panic!() };
        let loads: Vec<u32> = buf.iter().filter(|c| c.0 >> 24 == 0xF4).map(|c| (c.0 & 0xFFF) >> 2).collect();
        assert_eq!(loads.len(), (4 * 16 + 16) * 2);
        // A side's rows: t 0, 31, then 31, 0 again.
        let side_rows: Vec<u32> = loads.iter().step_by(2).take(16).copied().collect();
        assert_eq!(side_rows, vec![0, 0, 0, 0, 31, 31, 31, 31, 31, 31, 31, 31, 0, 0, 0, 0]);
        // The palettes: `vr_fine0` (index 0, bits 0 and 2 alike) reads the second half,
        // `vr_fine1` (1) the first, `vr_cloud0` (4) the first, `vr_cloud1` (5) the second.
        assert_eq!([palette_half(0), palette_half(1), palette_half(4), palette_half(5)], [1, 0, 0, 1]);
    }
}
