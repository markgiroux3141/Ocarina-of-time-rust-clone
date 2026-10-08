//! The pause menu's drawing (docs/adr/0047-the-pause-menu.md).
//!
//! `KaleidoScope_Draw` builds its display list every frame: vertex arrays (`GRAPH_ALLOC`'d by
//! `KaleidoScope_SetVertices`), then `gSPVertex` loads and one `gSP1Quadrangle` per textured
//! quad, under the page's matrix and the menu's own view. The engine has no runtime display
//! lists (docs/adr/0006-rendering-model.md), so the port runs those functions as written over a
//! recorder (`KaleidoGfx`): the vertex loads, the combiner, the prim and env colours, the
//! matrix and the view are its state, and each quadrangle becomes a `KQuad`. A quad is drawn as
//! a sprite bake (docs/adr/0017-interface-sprites.md: its texture loaded as the C loads it,
//! under the C's combiner) whose unit quad the vertices place (`quad_transform`), with the
//! vertices' colours (`DrawParams::vertex_colors`) and the draw's prim and env colours.
//!
//! Every quad the menu draws covers its whole texture (its vertices' texture coordinates are
//! the texture's size), so the baked texture coordinates are the C's.
//!
//! **The cursor's vertices.** `KaleidoScope_SetVertices` rebuilds `cursorVtx` in the draw and
//! sets only its first corner's position (`KaleidoScope_SetCursorPos`, or the arrows' in
//! `KaleidoScope_DrawUIOverlay`); the four corners are spread from it by
//! `KaleidoScope_UpdateCursorVtx`, which `KaleidoScope_Update` calls at the start of the *next*
//! frame, on the same vertices: `Graph_TaskSet00` (`graph.c`) hands the frame's task to the RSP
//! and returns, and the next frame's update reaches the vertices before the RSP does. So the
//! cursor's loads stay references (`VtxSlot::Cursor`), resolved when the frame is drawn, and the
//! port calls `KaleidoScope_UpdateCursorVtx` at the end of the menu's draw, under the update's
//! own condition (`PlayState::kaleido_scope_draw`).

use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use glam::{Mat4, Vec3};

use crate::gbi::{Dl, G_IM_FMT_IA, G_IM_FMT_RGBA, G_IM_SIZ_4B, G_IM_SIZ_8B, G_IM_SIZ_32B, G_TX_WRAP, ac, cc_ab, cc_c, cc_d};
use crate::pack::keys;
use crate::sprite::{Load, Quad, SEG_COLOR, SpriteBake, TexSrc};

/// `Vtx_t` (`gbi.h`): position, texture coordinates (10.5 fixed point), colour.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Vtx {
    pub ob: [i16; 3],
    pub tc: [i16; 2],
    pub cn: [u8; 4],
}

/// A vertex `gSPVertex` loaded: its data, or `cursorVtx[i]` as the RSP reads it (see the
/// module's notes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VtxSlot {
    Data(Vtx),
    Cursor(usize),
}

impl Default for VtxSlot {
    fn default() -> VtxSlot {
        VtxSlot::Data(Vtx::default())
    }
}

/// The colour combiners the menu draws with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Cc {
    /// `G_CC_MODULATEIA`: the texel times the shade (the pages' backgrounds, coloured by their
    /// vertices).
    ModulateIa,
    /// `G_CC_MODULATEIA_PRIM` (and `G_CC_MODULATERGBA_PRIM`, the same): the texel times prim.
    ModulateIaPrim,
    /// `gDPSetCombineLERP(PRIMITIVE, ENVIRONMENT, TEXEL0, ENVIRONMENT, TEXEL0, 0, PRIMITIVE, 0,
    /// ...)`: from env to prim by the texel, alpha the texel's times prim's.
    PrimEnvTexel,
}

impl Cc {
    fn name(self) -> &'static str {
        match self {
            Cc::ModulateIa => "ia",
            Cc::ModulateIaPrim => "iap",
            Cc::PrimEnvTexel => "pe",
        }
    }

    fn lerp(self) -> [u32; 8] {
        match self {
            Cc::ModulateIa => [cc_ab::TEXEL0, cc_ab::ZERO, cc_c::SHADE, cc_d::ZERO, ac::TEXEL0, ac::ZERO, ac::SHADE, ac::ZERO],
            Cc::ModulateIaPrim => [cc_ab::TEXEL0, cc_ab::ZERO, cc_c::PRIMITIVE, cc_d::ZERO, ac::TEXEL0, ac::ZERO, ac::PRIMITIVE, ac::ZERO],
            Cc::PrimEnvTexel => [cc_ab::PRIMITIVE, cc_ab::ENVIRONMENT, cc_c::TEXEL0, cc_d::ENVIRONMENT, ac::TEXEL0, ac::ZERO, ac::PRIMITIVE, ac::ZERO],
        }
    }
}

/// A texture the menu draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum KTex {
    /// A page's background tile (IA8 80x32), by its symbol: `icon_item_static`'s, or the
    /// language file's (`icon_item_nes_static`, the English ones).
    PageBg(&'static str),
    /// `gItemIcons[item]` (`icon_item_static`, RGBA32 32x32).
    ItemIcon(u8),
    /// The same, greyed when the menu opens (`KaleidoScope_GrayOutTextureRGBA32`).
    ItemIconGray(u8),
    /// `gEquippedItemOutlineTex` (`parameter_static`, IA8 32x32).
    EquippedOutline,
    /// `gAmmoDigit0Tex + 8 * 8 * n` (`parameter_static`, IA8 8x8).
    AmmoDigit(u8),
    /// `sCursorTexs[i]` (`icon_item_static`, IA4 16x16).
    Cursor(u8),
    /// `gInfoPanelBgDL`'s two halves (`gInfoPanelBgLeftTex`, `gInfoPanelBgRightTex`, IA8 72x24).
    InfoPanelBg(u8),
    /// `gLButtonIconDL`'s and `gRButtonIconDL`'s (`gLButtonTex`, `gRButtonTex`, IA8 24x32).
    LButton,
    RButton,
    /// `gCButtonIconsDL`'s `gCBtnSymbolsTex` (IA8 48x16) and `gAButtonIconDL`'s `gABtnSymbolTex`
    /// (IA8 24x16).
    CBtnSymbols,
    ABtnSymbol,
    /// An item's name (`item_name_static`, IA4 128x16, the English one at `item * 0x400`), as
    /// `KaleidoScope_UpdateNamePanel` loads it into `nameSegment`.
    ItemName(u16),
    /// A label of the language file (`icon_item_nes_static`, IA8, 16 high): its symbol and width.
    Label(&'static str, u32),
    /// `gMagicArrowEquipEffectTex` (`icon_item_static`, IA8 32x32).
    MagicArrowEffect,
}

impl KTex {
    /// The bake key's part for the texture.
    pub fn key(self) -> String {
        match self {
            KTex::PageBg(s) => format!("bg/{s}"),
            KTex::ItemIcon(i) => format!("icon/{i:02x}"),
            KTex::ItemIconGray(i) => format!("icon_gray/{i:02x}"),
            KTex::EquippedOutline => "outline".into(),
            KTex::AmmoDigit(d) => format!("ammo/{d}"),
            KTex::Cursor(i) => format!("cursor/{i}"),
            KTex::InfoPanelBg(i) => format!("info_bg/{i}"),
            KTex::LButton => "l".into(),
            KTex::RButton => "r".into(),
            KTex::CBtnSymbols => "c_symbols".into(),
            KTex::ABtnSymbol => "a_symbol".into(),
            KTex::ItemName(i) => format!("name/{i:02x}"),
            KTex::Label(s, _) => format!("label/{s}"),
            KTex::MagicArrowEffect => "magic_arrow".into(),
        }
    }

    /// Where its texels are and how the C loads them (`gDPLoadTextureBlock`, wrapped).
    pub fn source(self) -> (TexSrc, Load) {
        let sym = |file: &str, s: &str| TexSrc::Symbol { file: file.into(), symbol: s.into() };
        let ia8 = |w: u32, h: u32| Load::new(G_IM_FMT_IA, G_IM_SIZ_8B, w, h, G_TX_WRAP);
        let ia4 = |w: u32, h: u32| Load::new(G_IM_FMT_IA, G_IM_SIZ_4B, w, h, G_TX_WRAP);
        let rgba32 = Load::new(G_IM_FMT_RGBA, G_IM_SIZ_32B, 32, 32, G_TX_WRAP);
        match self {
            KTex::PageBg(s) => (sym(if s.ends_with("ENGTex") { "icon_item_nes_static" } else { "icon_item_static" }, s), ia8(80, 32)),
            KTex::ItemIcon(i) => (TexSrc::File { file: "icon_item_static".into(), offset: i as u32 * 0x1000 }, rgba32),
            KTex::ItemIconGray(i) => (TexSrc::GrayRgba32 { file: "icon_item_static".into(), offset: i as u32 * 0x1000, pixels: 32 * 32 }, rgba32),
            KTex::EquippedOutline => (sym("parameter_static", "gEquippedItemOutlineTex"), ia8(32, 32)),
            KTex::AmmoDigit(d) => (sym("parameter_static", &format!("gAmmoDigit{d}Tex")), ia8(8, 8)),
            KTex::Cursor(i) => {
                const CURSOR: [&str; 4] = ["gPauseMenuCursorTopLeftTex", "gPauseMenuCursorTopRightTex", "gPauseMenuCursorBottomLeftTex", "gPauseMenuCursorBottomRightTex"];
                (sym("icon_item_static", CURSOR[i as usize]), ia4(16, 16))
            }
            KTex::InfoPanelBg(i) => (sym("icon_item_static", ["gInfoPanelBgLeftTex", "gInfoPanelBgRightTex"][i as usize]), ia8(72, 24)),
            KTex::LButton => (sym("icon_item_static", "gLButtonTex"), ia8(24, 32)),
            KTex::RButton => (sym("icon_item_static", "gRButtonTex"), ia8(24, 32)),
            KTex::CBtnSymbols => (sym("icon_item_static", "gCBtnSymbolsTex"), ia8(48, 16)),
            KTex::ABtnSymbol => (sym("icon_item_static", "gABtnSymbolTex"), ia8(24, 16)),
            KTex::ItemName(i) => (TexSrc::File { file: "item_name_static".into(), offset: i as u32 * ITEM_NAME_TEX_SIZE }, ia4(ITEM_NAME_TEX_WIDTH, ITEM_NAME_TEX_HEIGHT)),
            KTex::Label(s, w) => (sym("icon_item_nes_static", s), ia8(w, 16)),
            KTex::MagicArrowEffect => (sym("icon_item_static", "gMagicArrowEquipEffectTex"), ia8(32, 32)),
        }
    }

    /// Its width and height (texels).
    pub fn size(self) -> (u32, u32) {
        let (_, l) = self.source();
        (l.width, l.height)
    }
}

/// `ITEM_NAME_TEX_WIDTH`, `ITEM_NAME_TEX_HEIGHT`, `ITEM_NAME_TEX_SIZE` (`item_name_static.h`).
pub const ITEM_NAME_TEX_WIDTH: u32 = 128;
pub const ITEM_NAME_TEX_HEIGHT: u32 = 16;
pub const ITEM_NAME_TEX_SIZE: u32 = ITEM_NAME_TEX_WIDTH * ITEM_NAME_TEX_HEIGHT / 2;

/// One `gSP1Quadrangle`: the texture and combiner it was drawn under, the view and model
/// matrix, its four vertices in the C's order (top left, top right, bottom left, bottom right:
/// `gSP1Quadrangle(j, j + 2, j + 3, j + 1)`), and the prim and env colours.
#[derive(Debug, Clone, PartialEq)]
pub struct KQuad {
    pub tex: KTex,
    pub cc: Cc,
    pub mtx: Mat4,
    pub vtx: [VtxSlot; 4],
    pub prim: [u8; 4],
    pub env: [u8; 4],
}

/// The menu's display list as it's built: the state the C's commands set, and the quads.
#[derive(Debug, Clone)]
pub struct KaleidoGfx {
    pub quads: Vec<KQuad>,
    view: Mat4,
    model: Mat4,
    loaded: [VtxSlot; 32],
    pub cc: Cc,
    pub prim: [u8; 4],
    pub env: [u8; 4],
}

impl Default for KaleidoGfx {
    fn default() -> KaleidoGfx {
        KaleidoGfx { quads: Vec::new(), view: Mat4::IDENTITY, model: Mat4::IDENTITY, loaded: [VtxSlot::default(); 32], cc: Cc::ModulateIa, prim: [0, 0, 0, 255], env: [0, 0, 0, 255] }
    }
}

impl KaleidoGfx {
    /// `KaleidoScope_SetView`'s `View_LookAt` from `eye` to the origin, up +Y (`View_Apply`'s
    /// projection is the list's, `PAUSE_VIEW`).
    pub fn set_view(&mut self, eye: Vec3) {
        self.view = glam::camera::rh::view::look_at_mat4(eye, Vec3::ZERO, Vec3::Y);
    }

    /// `MATRIX_FINALIZE_AND_LOAD`: the modelview.
    pub fn matrix(&mut self, m: Mat4) {
        self.model = m;
    }

    /// `gSPVertex(vtx, n, v0)`.
    pub fn vertex(&mut self, vtx: &[Vtx], n: usize, v0: usize) {
        for (i, v) in vtx.iter().take(n).enumerate() {
            if let Some(s) = self.loaded.get_mut(v0 + i) {
                *s = VtxSlot::Data(*v);
            }
        }
    }

    /// `gSPVertex(&pauseCtx->cursorVtx[start], n, v0)`, read when the frame is drawn.
    pub fn vertex_cursor(&mut self, start: usize, n: usize, v0: usize) {
        for i in 0..n {
            if let Some(s) = self.loaded.get_mut(v0 + i) {
                *s = VtxSlot::Cursor(start + i);
            }
        }
    }

    /// `gDPSetCombineMode` / `gDPSetCombineLERP`.
    pub fn combine(&mut self, cc: Cc) {
        self.cc = cc;
    }

    /// `gDPSetPrimColor(0, 0, r, g, b, a)`.
    pub fn prim_color(&mut self, r: i16, g: i16, b: i16, a: i16) {
        self.prim = [r as u8, g as u8, b as u8, a as u8];
    }

    /// `gDPSetEnvColor(r, g, b, a)`.
    pub fn env_color(&mut self, r: i16, g: i16, b: i16, a: i16) {
        self.env = [r as u8, g as u8, b as u8, a as u8];
    }

    /// The texture's load and `gSP1Quadrangle(point, point + 2, point + 3, point + 1, 0)`
    /// (`KaleidoScope_QuadTextureIA4`, `_IA8`, `KaleidoScope_DrawQuadTextureRGBA32`).
    pub fn quad(&mut self, tex: KTex, point: usize) {
        let v = |k: usize| self.loaded.get(point + k).copied().unwrap_or_default();
        let q = KQuad { tex, cc: self.cc, mtx: self.view * self.model, vtx: [v(0), v(1), v(2), v(3)], prim: self.prim, env: self.env };
        self.quads.push(q);
    }
}

/// `View_Init`'s perspective for the menu's `View_Apply`: `fovy` 60, `zNear` 10, `zFar` 12800
/// (`z_view.c`).
pub const PAUSE_VIEW: eng_gfx::Perspective = eng_gfx::Perspective { fovy: 60.0, near: 10.0, far: 12800.0 };

/// The unit quad ((0, 0) top left, (1, -1) bottom right) onto the quad's vertices: x along the
/// top edge, y up the left one.
pub fn quad_transform(v: &[Vtx; 4]) -> Mat4 {
    let p = |i: usize| Vec3::new(v[i].ob[0] as f32, v[i].ob[1] as f32, v[i].ob[2] as f32);
    Mat4::from_cols((p(1) - p(0)).extend(0.0), (p(0) - p(2)).extend(0.0), Vec3::Z.extend(0.0), p(0).extend(1.0))
}

/// The bake's vertices in its mesh's order: `gSP1Quadrangle(0, 2, 3, 1)`'s triangles (0, 2, 3)
/// and (0, 3, 1).
pub const MESH_ORDER: [usize; 6] = [0, 2, 3, 0, 3, 1];

/// The bake a quad draws.
pub fn bake_name(tex: KTex, cc: Cc) -> String {
    format!("kaleido/{}/{}", tex.key(), cc.name())
}

impl KQuad {
    /// The quad's vertices, the cursor's as they are now.
    pub fn resolve(&self, cursor: &[Vtx]) -> [Vtx; 4] {
        self.vtx.map(|s| match s {
            VtxSlot::Data(v) => v,
            VtxSlot::Cursor(i) => cursor.get(i).copied().unwrap_or_default(),
        })
    }

    /// Its draw: the bake, the matrix times the quad's placement, the vertex colours, prim and
    /// env.
    pub fn draw_cmd(&self, cursor: &[Vtx]) -> DrawCmd {
        let v = self.resolve(cursor);
        let mut sv = SegmentValues::default();
        sv.prim[SEG_COLOR as usize] = Some(self.prim);
        sv.env[SEG_COLOR as usize] = Some(self.env);
        let params = DrawParams { segments: Some(sv), vertex_colors: Some(MESH_ORDER.iter().map(|&k| v[k].cn).collect()), ..Default::default() };
        DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(self.tex, self.cc))), transform: self.mtx * quad_transform(&v), bones: Vec::new(), params }
    }
}

/// `Gfx_SetupDL_42Opa` (`SETUPDL_42`, as `crate::interface` writes it), then the combiner.
fn setup_42(cc: Cc) -> Dl {
    let mut d = Dl::default();
    d.pipe_sync();
    d.0.push((0xD700_0002, 0xFFFF_FFFF));
    // G_CC_MODULATEIDECALA, replaced below.
    let decala = [cc_ab::TEXEL0, cc_ab::ZERO, cc_ab::SHADE, cc_d::ZERO, ac::ZERO, ac::ZERO, ac::ZERO, ac::TEXEL0];
    d.combine_lerp(decala, decala);
    // G_AD_NOTPATTERN | G_CD_MAGICSQ | G_CK_NONE | G_TC_FILT | G_TF_BILERP | G_TT_NONE | G_TL_TILE |
    // G_TD_CLAMP | G_TP_PERSP | G_CYC_1CYCLE | G_PM_NPRIMITIVE, G_AC_NONE | G_ZS_PIXEL |
    // G_RM_XLU_SURF | G_RM_XLU_SURF2.
    d.0.push((0xEF08_2C10, 0x0050_4240));
    // G_SHADE | G_CULL_BACK | G_SHADING_SMOOTH.
    d.0.push((0xD900_0000, 0x0020_0404));
    let l = cc.lerp();
    d.combine_lerp(l, l);
    d
}

/// The last item with an icon the menu can draw: the grid's items and the bows with magic
/// arrows a magic arrow's equip turns into (`ITEM_BOW_LIGHT`).
const LAST_MENU_ICON: u8 = crate::item::ITEM_BOW_LIGHT;
/// `item_name_static`'s English names (123).
pub const ITEM_NAME_COUNT: u16 = 123;

/// Every texture and combiner the menu draws (the item page, the frame, the HUD's flying icon).
pub fn bake_list() -> Vec<(KTex, Cc)> {
    let mut v = Vec::new();
    for page in super::scope::PAGE_BGS {
        v.extend(page.iter().map(|&s| (KTex::PageBg(s), Cc::ModulateIa)));
    }
    for item in 0..=LAST_MENU_ICON {
        v.push((KTex::ItemIcon(item), Cc::ModulateIaPrim));
        if super::ITEM_AGE_REQS[item as usize] != super::AGE_REQ_NONE {
            v.push((KTex::ItemIconGray(item), Cc::ModulateIaPrim));
        }
    }
    v.push((KTex::EquippedOutline, Cc::ModulateIaPrim));
    v.extend((0..10).map(|d| (KTex::AmmoDigit(d), Cc::PrimEnvTexel)));
    v.extend((0..4).map(|i| (KTex::Cursor(i), Cc::PrimEnvTexel)));
    v.extend([(KTex::InfoPanelBg(0), Cc::ModulateIaPrim), (KTex::InfoPanelBg(1), Cc::ModulateIaPrim), (KTex::LButton, Cc::ModulateIaPrim), (KTex::RButton, Cc::ModulateIaPrim)]);
    v.extend([(KTex::CBtnSymbols, Cc::PrimEnvTexel), (KTex::ABtnSymbol, Cc::PrimEnvTexel)]);
    v.extend((0..ITEM_NAME_COUNT).map(|i| (KTex::ItemName(i), Cc::PrimEnvTexel)));
    v.extend(super::scope::LABELS.iter().map(|&(s, w)| (KTex::Label(s, w as u32), Cc::PrimEnvTexel)));
    v.push((KTex::MagicArrowEffect, Cc::ModulateIaPrim));
    v
}

/// The menu's sprite bakes.
pub fn bakes() -> Vec<SpriteBake> {
    bake_list().into_iter().map(|(t, c)| sprite_bake(t, c)).collect()
}

/// The bake for a texture under a combiner: its load as the C's, the unit quad.
pub fn sprite_bake(tex: KTex, cc: Cc) -> SpriteBake {
    let (src, load) = tex.source();
    let (s, t) = (load.width as i32 * 32, load.height as i32 * 32);
    let vtx = [([0, 0, 0], [0, 0]), ([1, 0, 0], [s, 0]), ([0, -1, 0], [0, t]), ([1, -1, 0], [s, t])];
    SpriteBake { name: bake_name(tex, cc), tex: src, load, setup: setup_42(cc), prim: true, env: true, quad: Quad::Vtx(vtx) }
}
