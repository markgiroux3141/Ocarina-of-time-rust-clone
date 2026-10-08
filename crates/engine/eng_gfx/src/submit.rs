//! Draw submission: what game code hands the renderer each frame, like the game's
//! `POLY_OPA_DISP` / `POLY_XLU_DISP` buffers. Each command names a mesh, where it goes (a
//! transform, and a bone palette for skinned meshes) and its per-draw material parameters.
//! The renderer draws the OPA list, then the XLU list, each in submission order
//! (docs/adr/0006-rendering-model.md).
//!
//! Meshes are named by `MeshKey`: the game's own names (asset-pack record names, or the app's
//! built-in meshes). The renderer asks the app's mesh source for a key the first time it sees
//! it. Segment texture bindings (Link's eyes and mouth) are part of the key, since they select
//! different texture images.

use glam::{Mat4, Vec3};

use crate::draw::SegmentValues;

/// A mesh the renderer can upload: a name, and the textures bound to segments by this draw
/// (`(segment, index)`, e.g. Link's eyes on 8), which the mesh source resolves.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MeshKey {
    pub name: String,
    pub segment_textures: Vec<(u8, u16)>,
}

impl MeshKey {
    pub fn named(name: impl Into<String>) -> MeshKey {
        MeshKey { name: name.into(), segment_textures: Vec::new() }
    }
}

/// Per-draw material parameters.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DrawParams {
    /// This frame's dynamic segment values (tile scroll, env and prim colours), for the mesh's
    /// materials that read them. `None` keeps the mesh's own values.
    pub segments: Option<SegmentValues>,
    /// Drawn in the interface's orthographic projection (`DrawLists::overlay_2d`'s) where it
    /// stands in its list: the prerendered backgrounds, which `gSPBgRectCopy` copies to the
    /// screen in the middle of `POLY_OPA_DISP`.
    pub screen: bool,
    /// Point lights bound for this draw, as directional lights the lit materials add to the
    /// frame's (the game binds an actor's near its position, `Lights_BindAll`). At most
    /// `MAX_POINT_LIGHTS` are used.
    pub lights: Vec<PointLight>,
    /// This draw's fog in place of the frame's (`gDPSetFogColor` and `gSPFogFactor` set in the
    /// list before it): the materials that fog use it.
    pub fog: Option<FogOverride>,
    /// The mesh's vertex colours replaced, vertex by vertex in the mesh's order (its batches'
    /// triangle-list vertices): vertices the game builds every frame (`GRAPH_ALLOC`'d `Vtx`
    /// with the frame's colours, `z_eff_spark.c`). Vertices past the end keep their own.
    pub vertex_colors: Option<Vec<[u8; 4]>>,
}

/// A draw's own fog: the fog colour and the RSP's fog factor (`gSPFogFactor`'s multiplier
/// and offset, in the frame's projection).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FogOverride {
    pub color: [u8; 4],
    pub multiplier: i16,
    pub offset: i16,
}

/// A point light bound for one draw: the RSP's directional light it becomes, a direction (not
/// normalised) towards the light and a colour already scaled by the distance.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PointLight {
    pub dir: [i8; 3],
    pub color: [u8; 3],
}

/// The point lights a draw's materials take.
pub const MAX_POINT_LIGHTS: usize = 3;

/// One draw: a mesh, its model matrix, its bone matrices (empty for unskinned meshes), and its
/// parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawCmd {
    pub mesh: MeshKey,
    pub transform: Mat4,
    pub bones: Vec<Mat4>,
    pub params: DrawParams,
}

impl DrawCmd {
    pub fn new(mesh: MeshKey, transform: Mat4) -> DrawCmd {
        DrawCmd { mesh, transform, bones: Vec::new(), params: DrawParams::default() }
    }
}

/// A perspective projection: the vertical field of view in degrees, the near and far planes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Perspective {
    pub fovy: f32,
    pub near: f32,
    pub far: f32,
}

/// A coloured line vertex (debug lines come in pairs).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinePoint {
    pub pos: Vec3,
    pub color: [f32; 4],
}

/// A frame's submissions.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DrawLists {
    /// `POLY_OPA_DISP`, drawn first.
    pub opa: Vec<DrawCmd>,
    /// `POLY_XLU_DISP`, drawn after the OPA list.
    pub xlu: Vec<DrawCmd>,
    /// Lines drawn over everything, without depth (debug views).
    pub overlay: Vec<LinePoint>,
    /// `OVERLAY_DISP`: meshes drawn after the 3D lists and the letterbox, in the interface's
    /// orthographic projection (`View_ApplyOrthoToOverlay`): the 320x240 screen centred on 0,
    /// x right, y up. A wider target extends x, so a point keeps its place over the 3D view.
    pub overlay_2d: Vec<DrawCmd>,
    /// The letterbox bars' height in rows of the 240-row frame (`Letterbox_GetSize`): black
    /// over the 3D lists, under `overlay_2d`.
    pub letterbox_rows: f32,
    /// The pause menu (`KaleidoScope_Draw` into `POLY_OPA_DISP`, over the saved frame): drawn
    /// after the 3D lists, their fills and the letterbox, before the overlay's fill and
    /// `overlay_2d`, in `pause_view`'s perspective. Each command's transform is the view and the
    /// model (the menu's `View_LookAt` times its matrix), so the GPU clips what's behind the eye.
    pub pause: Vec<DrawCmd>,
    /// The pause menu's projection (`View_Apply`'s `guPerspective`); the aspect is the target's,
    /// as the 3D view's.
    pub pause_view: Option<Perspective>,
    /// Full-screen fills (RGBA, blended by alpha), drawn where the game's lists put them: at the
    /// end of the OPA list, at the end of the XLU list, and at the start of the overlay (over the
    /// letterbox, under `overlay_2d`).
    pub opa_fill: Option<[u8; 4]>,
    pub xlu_fill: Option<[u8; 4]>,
    pub overlay_fill: Option<[u8; 4]>,
}

impl DrawLists {
    /// Everything in drawing order: the OPA list, then the XLU list.
    pub fn ordered(&self) -> impl Iterator<Item = &DrawCmd> {
        self.opa.iter().chain(self.xlu.iter())
    }
}
