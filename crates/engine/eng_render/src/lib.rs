//! wgpu renderer for `eng_gfx` draw lists: CPU skinning of flex-bound vertices, per-material
//! colour-combiner uniforms, pipeline variants for the RDP render modes, and an offscreen
//! target that can be shown in a UI or read back to an image.
//!
//! - `device`: the headless device and the offscreen `Target`.
//! - `view`: the camera, lights and fog a frame is drawn with.
//! - `pipelines`: vertex formats and the render-mode pipeline and sampler caches.
//! - `materials`: material uniforms (combiner selectors, colours, dynamic-segment values).
//! - `model`: uploaded meshes (`GpuModel`), posing and per-frame segment values.
//! - `passes`: recording a frame (the 3D, the letterbox, the orthographic overlay).
//! - `lists`: drawing submitted OPA/XLU lists, with the mesh cache.
//! - `probe`: one pixel of a frame's depth read back (`eng_gfx::DrawLists::depth_probe`).

use std::collections::HashMap;

use eng_gfx::texture::WrapMode;

mod device;
mod lists;
mod materials;
mod model;
mod passes;
mod pipelines;
mod probe;
mod view;

pub use device::{COLOR_FORMAT, NON_FEATURES, SAMPLES, Target, headless_device};
pub use lists::{MeshCache, MeshSource};
pub use model::GpuModel;
pub use passes::Screen;
pub use pipelines::LineVertex;
pub use view::{Camera, Fog, Lighting};

use pipelines::PipelineKey;

pub struct Renderer {
    pub(crate) shader: wgpu::ShaderModule,
    pub(crate) globals_buf: wgpu::Buffer,
    pub(crate) globals_bg: wgpu::BindGroup,
    /// The overlay's globals (the orthographic interface projection).
    pub(crate) overlay_globals_buf: wgpu::Buffer,
    pub(crate) overlay_globals_bg: wgpu::BindGroup,
    /// The pause menu's globals (its own perspective, `eng_gfx::DrawLists::pause`).
    pub(crate) pause_globals_buf: wgpu::Buffer,
    pub(crate) pause_globals_bg: wgpu::BindGroup,
    /// Clip-space triangles (the letterbox bars).
    pub(crate) fill_pipeline: wgpu::RenderPipeline,
    pub(crate) material_layout: wgpu::BindGroupLayout,
    pub(crate) texture_layout: wgpu::BindGroupLayout,
    pub(crate) pipeline_layout: wgpu::PipelineLayout,
    pub(crate) line_pipeline_depth: wgpu::RenderPipeline,
    pub(crate) line_pipeline_overlay: wgpu::RenderPipeline,
    pub(crate) pipelines: HashMap<PipelineKey, wgpu::RenderPipeline>,
    pub(crate) samplers: HashMap<(WrapMode, WrapMode, bool), wgpu::Sampler>,
    pub(crate) white: wgpu::TextureView,
    pub(crate) probe: probe::DepthProbe,
}

impl Renderer {
    /// The depth the last frame's `depth_probe` read (1 is the far plane: nothing drawn there;
    /// 0 for a pixel off the target), once that frame's commands are submitted. `None` when the
    /// frame asked for none.
    pub fn read_depth_probe(&mut self, device: &wgpu::Device) -> Option<f32> {
        self.probe.read(device)
    }
}
