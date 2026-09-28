//! Vertex formats, the renderer's shared layouts, and the pipeline and sampler caches: one
//! pipeline per render-mode combination (`PipelineKey`), one sampler per wrap/filter pair.

use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use eng_gfx::CullMode;
use eng_gfx::texture::WrapMode;

use crate::Renderer;
use crate::device::{COLOR_FORMAT, DEPTH_FORMAT, SAMPLES};
use crate::model::write_texture;
use crate::passes::Globals;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct GpuVertex {
    pub(crate) pos: [f32; 3],
    pub(crate) normal: [f32; 3],
    pub(crate) color: [u8; 4],
    pub(crate) uv0: [f32; 2],
    pub(crate) uv1: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct LineVertex {
    pub pos: [f32; 3],
    pub color: [f32; 4],
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct PipelineKey {
    pub(crate) translucent: bool,
    pub(crate) cull: CullMode,
    pub(crate) depth_test: bool,
    pub(crate) depth_write: bool,
    pub(crate) decal: bool,
}

fn wrap(w: WrapMode) -> wgpu::AddressMode {
    match w {
        WrapMode::Repeat => wgpu::AddressMode::Repeat,
        WrapMode::Mirror => wgpu::AddressMode::MirrorRepeat,
        WrapMode::Clamp => wgpu::AddressMode::ClampToEdge,
    }
}

impl Renderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Renderer {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("combiner"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let uniform_entry = |binding, dynamic| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: dynamic,
                min_binding_size: None,
            },
            count: None,
        };
        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals"),
            entries: &[uniform_entry(0, false)],
        });
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("material"),
            entries: &[uniform_entry(0, true)],
        });
        let tex_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let samp_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("textures"),
            entries: &[tex_entry(0), samp_entry(1), tex_entry(2), samp_entry(3)],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("main"),
            bind_group_layouts: &[Some(&globals_layout), Some(&material_layout), Some(&texture_layout)],
            immediate_size: 0,
        });
        let globals_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let globals_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals"),
            layout: &globals_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: globals_buf.as_entire_binding() }],
        });

        let line_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("lines"),
            bind_group_layouts: &[Some(&globals_layout)],
            immediate_size: 0,
        });
        let line_pipeline = |depth: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("lines"),
                layout: Some(&line_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_line"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<LineVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4],
                    })],
                },
                primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::LineList, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(if depth { wgpu::CompareFunction::LessEqual } else { wgpu::CompareFunction::Always }),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState { count: SAMPLES, ..Default::default() },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_line"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: COLOR_FORMAT,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let line_pipeline_depth = line_pipeline(true);
        let line_pipeline_overlay = line_pipeline(false);

        let white_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("white"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COLOR_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        write_texture(queue, &white_tex, 1, 1, &[255, 255, 255, 255]);

        Renderer {
            white: white_tex.create_view(&Default::default()),
            shader,
            globals_buf,
            globals_bg,
            material_layout,
            texture_layout,
            pipeline_layout,
            line_pipeline_depth,
            line_pipeline_overlay,
            pipelines: HashMap::new(),
            samplers: HashMap::new(),
        }
    }

    pub(crate) fn pipeline(&mut self, device: &wgpu::Device, key: PipelineKey) -> &wgpu::RenderPipeline {
        let shader = &self.shader;
        let layout = &self.pipeline_layout;
        self.pipelines.entry(key).or_insert_with(|| {
            let cull = match key.cull {
                CullMode::Back => Some(wgpu::Face::Back),
                CullMode::Front => Some(wgpu::Face::Front),
                // Culling both faces draws nothing; approximate as no culling so it stays visible.
                CullMode::None | CullMode::Both => None,
            };
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("material"),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<GpuVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![
                            0 => Float32x3, 1 => Float32x3, 2 => Unorm8x4, 3 => Float32x2, 4 => Float32x2
                        ],
                    })],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: cull,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(key.depth_write),
                    depth_compare: Some(if key.depth_test { wgpu::CompareFunction::LessEqual } else { wgpu::CompareFunction::Always }),
                    stencil: Default::default(),
                    bias: if key.decal {
                        wgpu::DepthBiasState { constant: -2, slope_scale: -1.0, clamp: 0.0 }
                    } else {
                        Default::default()
                    },
                }),
                multisample: wgpu::MultisampleState { count: SAMPLES, ..Default::default() },
                fragment: Some(wgpu::FragmentState {
                    module: shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: COLOR_FORMAT,
                        blend: key.translucent.then_some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        })
    }

    pub(crate) fn sampler(&mut self, device: &wgpu::Device, s: WrapMode, t: WrapMode, bilinear: bool) -> wgpu::Sampler {
        self.samplers
            .entry((s, t, bilinear))
            .or_insert_with(|| {
                let f = if bilinear { wgpu::FilterMode::Linear } else { wgpu::FilterMode::Nearest };
                device.create_sampler(&wgpu::SamplerDescriptor {
                    label: Some("n64"),
                    address_mode_u: wrap(s),
                    address_mode_v: wrap(t),
                    mag_filter: f,
                    min_filter: f,
                    ..Default::default()
                })
            })
            .clone()
    }
}
