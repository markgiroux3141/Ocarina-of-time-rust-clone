//! wgpu renderer for `eng_gfx` draw lists: CPU skinning of flex-bound vertices, per-material
//! colour-combiner uniforms, pipeline variants for the RDP render modes, and an offscreen
//! target that can be shown in a UI or read back to an image.
//!
//! - `device`: the headless device and the offscreen `Target`.
//! - `view`: the camera, lights and fog a frame is drawn with.
//! - `pipelines`: vertex formats and the render-mode pipeline and sampler caches.
//! - `materials`: material uniforms (combiner selectors, colours, dynamic-segment values).
//! - `model`: uploaded meshes (`GpuModel`), posing and per-frame segment values.
//! - `passes`: recording a frame.
//! - `lists`: drawing submitted OPA/XLU lists, with the mesh cache.

use std::collections::HashMap;

use eng_gfx::texture::WrapMode;

mod device;
mod lists;
mod materials;
mod model;
mod passes;
mod pipelines;
mod view;

pub use device::{COLOR_FORMAT, SAMPLES, Target, headless_device};
pub use lists::{MeshCache, MeshSource};
pub use model::GpuModel;
pub use pipelines::LineVertex;
pub use view::{Camera, Fog, Lighting};

use pipelines::PipelineKey;

pub struct Renderer {
    pub(crate) shader: wgpu::ShaderModule,
    pub(crate) globals_buf: wgpu::Buffer,
    pub(crate) globals_bg: wgpu::BindGroup,
    pub(crate) material_layout: wgpu::BindGroupLayout,
    pub(crate) texture_layout: wgpu::BindGroupLayout,
    pub(crate) pipeline_layout: wgpu::PipelineLayout,
    pub(crate) line_pipeline_depth: wgpu::RenderPipeline,
    pub(crate) line_pipeline_overlay: wgpu::RenderPipeline,
    pub(crate) pipelines: HashMap<PipelineKey, wgpu::RenderPipeline>,
    pub(crate) samplers: HashMap<(WrapMode, WrapMode, bool), wgpu::Sampler>,
    pub(crate) white: wgpu::TextureView,
}
