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
}

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
}

impl DrawLists {
    /// Everything in drawing order: the OPA list, then the XLU list.
    pub fn ordered(&self) -> impl Iterator<Item = &DrawCmd> {
        self.opa.iter().chain(self.xlu.iter())
    }
}
