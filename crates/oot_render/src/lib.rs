//! wgpu renderer for `oot_core::gbi::DrawList`s: CPU skinning of flex-bound vertices,
//! per-material colour-combiner uniforms, pipeline variants for the RDP render modes, and an
//! offscreen target that can be shown in a UI or read back to an image.

use std::cell::Cell;
use std::collections::HashMap;

use anyhow::{Context, Result};
use bytemuck::{Pod, Zeroable};
use glam::{Mat3, Mat4, Vec3};
use oot_core::gbi::{BlendMode, CullMode, DrawList, Material, NO_BONE, SegmentValues, TextureSlot};
use oot_core::texture::WrapMode;

pub const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub const SAMPLES: u32 = 4;
const MATERIAL_STRIDE: u64 = 256;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    view_proj: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
    light_dir: [f32; 4],
    light_color: [f32; 4],
    light2_dir: [f32; 4],
    light2_color: [f32; 4],
    ambient: [f32; 4],
    fog_color: [f32; 4],
    /// x: multiplier, y: offset (the RSP's fm/fo), z: near, w: far of the fog projection.
    fog: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MaterialUniform {
    sel: [[u32; 4]; 4],
    prim: [f32; 4],
    env: [f32; 4],
    params: [f32; 4],
    flags: [u32; 4],
    /// Texture coordinate offsets from dynamic segments: slot 0 in xy, slot 1 in zw.
    uv_off: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuVertex {
    pos: [f32; 3],
    normal: [f32; 3],
    color: [u8; 4],
    uv0: [f32; 2],
    uv1: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct LineVertex {
    pub pos: [f32; 3],
    pub color: [f32; 4],
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct PipelineKey {
    translucent: bool,
    cull: CullMode,
    depth_test: bool,
    depth_write: bool,
    decal: bool,
}

pub struct Camera {
    pub target: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub fov_y: f32,
    /// Near and far planes; `None` scales them with the orbit distance (viewer default).
    pub clip: Option<(f32, f32)>,
}

impl Camera {
    pub fn eye(&self) -> Vec3 {
        let dir = Vec3::new(self.pitch.cos() * self.yaw.sin(), self.pitch.sin(), self.pitch.cos() * self.yaw.cos());
        self.target + dir * self.distance
    }
    pub fn view(&self) -> Mat4 {
        glam::camera::rh::view::look_at_mat4(self.eye(), self.target, Vec3::Y)
    }
    pub fn clip_planes(&self) -> (f32, f32) {
        self.clip.unwrap_or((self.distance * 0.02, self.distance * 20.0))
    }
    pub fn proj(&self, aspect: f32) -> Mat4 {
        let (n, f) = self.clip_planes();
        glam::camera::rh::proj::directx::perspective(self.fov_y, aspect, n, f)
    }
}

/// N64 vertex fog (F3DEX2): each vertex's fog factor is `z_ndc * fm + fo` (in 1/256ths,
/// clamped), with `z_ndc` the OpenGL-style depth of the game's projection (`near`..`far`).
/// The blender then mixes the fog colour in by that factor (`G_RM_FOG_SHADE_A`).
#[derive(Debug, Clone, Copy)]
pub struct Fog {
    pub color: Vec3,
    /// `gSPFogFactor` multiplier and offset.
    pub multiplier: f32,
    pub offset: f32,
    /// The projection the factor is computed against (the game's `zNear` and `fogFar`).
    pub near: f32,
    pub far: f32,
}

/// Directional lights and ambient in world space, as `Lights_Draw` loads them for F3DEX2:
/// shade = ambient + sum of colour * max(0, n . dir), `dir` pointing towards the light.
pub struct Lighting {
    pub dir: Vec3,
    pub color: Vec3,
    pub dir2: Vec3,
    pub color2: Vec3,
    pub ambient: Vec3,
    pub fog: Option<Fog>,
}

impl Default for Lighting {
    fn default() -> Self {
        Lighting {
            dir: Vec3::new(0.45, 0.8, 0.6).normalize(),
            color: Vec3::splat(0.7),
            dir2: Vec3::Y,
            color2: Vec3::ZERO,
            ambient: Vec3::splat(0.38),
            fog: None,
        }
    }
}

pub struct Renderer {
    shader: wgpu::ShaderModule,
    globals_buf: wgpu::Buffer,
    globals_bg: wgpu::BindGroup,
    material_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    line_pipeline_depth: wgpu::RenderPipeline,
    line_pipeline_overlay: wgpu::RenderPipeline,
    pipelines: HashMap<PipelineKey, wgpu::RenderPipeline>,
    samplers: HashMap<(WrapMode, WrapMode, bool), wgpu::Sampler>,
    white: wgpu::TextureView,
}

pub struct GpuModel {
    vertex_buf: wgpu::Buffer,
    /// The skinned vertices changed since the last upload.
    dirty: Cell<bool>,
    materials: Vec<Material>,
    /// Materials whose uniforms depend on dynamic segments.
    dynamic: Vec<usize>,
    material_buf: wgpu::Buffer,
    material_bg: wgpu::BindGroup,
    texture_bgs: Vec<wgpu::BindGroup>,
    /// (vertex start, count, material index, texture bind group index, pipeline key)
    draws: Vec<(u32, u32, u32, usize, PipelineKey)>,
    base: Vec<GpuVertex>,
    bones: Vec<u16>,
    skinned: Vec<GpuVertex>,
}

pub struct Target {
    pub size: (u32, u32),
    msaa: wgpu::TextureView,
    depth: wgpu::TextureView,
    pub resolve: wgpu::Texture,
    pub resolve_view: wgpu::TextureView,
}

impl Target {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Target {
        let (w, h) = (width.max(1), height.max(1));
        let size = wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 };
        let mk = |label, format, samples, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size,
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let msaa = mk("msaa", COLOR_FORMAT, SAMPLES, wgpu::TextureUsages::RENDER_ATTACHMENT);
        let depth = mk("depth", DEPTH_FORMAT, SAMPLES, wgpu::TextureUsages::RENDER_ATTACHMENT);
        let resolve = mk(
            "resolve",
            COLOR_FORMAT,
            1,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
        );
        Target {
            size: (w, h),
            msaa: msaa.create_view(&Default::default()),
            depth: depth.create_view(&Default::default()),
            resolve_view: resolve.create_view(&Default::default()),
            resolve,
        }
    }

    /// Copies the resolved image back to the CPU as tightly packed RGBA8.
    pub fn read_rgba(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<Vec<u8>> {
        let (w, h) = self.size;
        let row = (w * 4).div_ceil(256) * 256;
        let buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (row * h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = device.create_command_encoder(&Default::default());
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.resolve,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buf,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(h) },
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        queue.submit([enc.finish()]);
        let slice = buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::PollType::wait_indefinitely()).context("device poll")?;
        rx.recv().context("map callback")?.context("buffer map")?;
        let data = slice.get_mapped_range().map_err(|e| anyhow::anyhow!("mapping readback buffer: {e:?}"))?;
        let mut out = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            let s = (y * row) as usize;
            out.extend_from_slice(&data[s..s + (w * 4) as usize]);
        }
        drop(data);
        buf.unmap();
        Ok(out)
    }
}

/// Creates a device without a window, for screenshots and tests.
pub fn headless_device() -> Result<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
        apply_limit_buckets: false,
    }))
    .context("no GPU adapter")?;
    log::info!("adapter: {:?}", adapter.get_info().name);
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    Ok((device, queue))
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

    fn pipeline(&mut self, device: &wgpu::Device, key: PipelineKey) -> &wgpu::RenderPipeline {
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

    fn sampler(&mut self, device: &wgpu::Device, s: WrapMode, t: WrapMode, bilinear: bool) -> wgpu::Sampler {
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

    pub fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, draw: &DrawList) -> GpuModel {
        let views: Vec<wgpu::TextureView> = draw
            .textures
            .iter()
            .map(|t| {
                let tex = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("n64 texture"),
                    size: wgpu::Extent3d { width: t.image.width, height: t.image.height, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: COLOR_FORMAT,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });
                write_texture(queue, &tex, t.image.width, t.image.height, &t.image.rgba);
                tex.create_view(&Default::default())
            })
            .collect();

        let uniforms: Vec<u8> = draw
            .materials
            .iter()
            .flat_map(|m| {
                let mut bytes = bytemuck::bytes_of(&material_uniform(m)).to_vec();
                bytes.resize(MATERIAL_STRIDE as usize, 0);
                bytes
            })
            .collect();
        let material_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("materials"),
            size: (uniforms.len() as u64).max(MATERIAL_STRIDE),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&material_buf, 0, &uniforms);
        let material_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("material"),
            layout: &self.material_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &material_buf,
                    offset: 0,
                    size: wgpu::BufferSize::new(std::mem::size_of::<MaterialUniform>() as u64),
                }),
            }],
        });

        let mut texture_bgs = Vec::new();
        let mut tex_lookup: HashMap<([Option<TextureSlot>; 2], bool), usize> = HashMap::new();
        let mut draws = Vec::new();
        let mut base = Vec::new();
        let mut bones = Vec::new();
        for batch in &draw.batches {
            let mat = &draw.materials[batch.material];
            let key = (mat.textures, mat.bilinear);
            let tbg = match tex_lookup.get(&key) {
                Some(&i) => i,
                None => {
                    let slot = |i: usize, this: &mut Self| -> (wgpu::TextureView, wgpu::Sampler) {
                        match mat.textures[i] {
                            Some(s) => (views[s.image].clone(), this.sampler(device, s.wrap_s, s.wrap_t, mat.bilinear)),
                            None => (this.white.clone(), this.sampler(device, WrapMode::Clamp, WrapMode::Clamp, false)),
                        }
                    };
                    let (v0, s0) = slot(0, self);
                    let (v1, s1) = slot(1, self);
                    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("textures"),
                        layout: &self.texture_layout,
                        entries: &[
                            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&v0) },
                            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&s0) },
                            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&v1) },
                            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&s1) },
                        ],
                    });
                    texture_bgs.push(bg);
                    tex_lookup.insert(key, texture_bgs.len() - 1);
                    texture_bgs.len() - 1
                }
            };
            let pkey = PipelineKey {
                translucent: mat.blend == BlendMode::Translucent,
                cull: mat.cull,
                depth_test: mat.depth_test,
                depth_write: mat.depth_write,
                decal: mat.decal,
            };
            self.pipeline(device, pkey);
            draws.push((base.len() as u32, batch.vertices.len() as u32, batch.material as u32, tbg, pkey));
            for v in &batch.vertices {
                base.push(GpuVertex {
                    pos: v.pos.to_array(),
                    normal: v.normal.to_array(),
                    color: v.color,
                    uv0: v.uv[0].to_array(),
                    uv1: v.uv[1].to_array(),
                });
                bones.push(v.bone);
            }
        }
        let vertex_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vertices"),
            size: ((base.len() * std::mem::size_of::<GpuVertex>()) as u64).max(64),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let dynamic = draw.materials.iter().enumerate().filter(|(_, m)| m.is_dynamic()).map(|(i, _)| i).collect();
        GpuModel {
            vertex_buf,
            dirty: Cell::new(true),
            materials: draw.materials.clone(),
            dynamic,
            material_buf,
            material_bg,
            texture_bgs,
            draws,
            skinned: base.clone(),
            base,
            bones,
        }
    }

    /// Records the scene: the posed models (each in submission order, like the RDP) and debug
    /// lines.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &Target,
        models: &[&GpuModel],
        camera: &Camera,
        light: &Lighting,
        world_lines: &[LineVertex],
        overlay_lines: &[LineVertex],
        clear: [f64; 4],
    ) {
        let aspect = target.size.0 as f32 / target.size.1 as f32;
        let view = camera.view();
        let fog = light.fog.unwrap_or(Fog { color: Vec3::ZERO, multiplier: 0.0, offset: 0.0, near: 1.0, far: 2.0 });
        let globals = Globals {
            view_proj: (camera.proj(aspect) * view).to_cols_array_2d(),
            view: view.to_cols_array_2d(),
            light_dir: light.dir.normalize_or_zero().extend(0.0).to_array(),
            light_color: light.color.extend(1.0).to_array(),
            light2_dir: light.dir2.normalize_or_zero().extend(0.0).to_array(),
            light2_color: light.color2.extend(1.0).to_array(),
            ambient: light.ambient.extend(1.0).to_array(),
            fog_color: fog.color.extend(1.0).to_array(),
            fog: [fog.multiplier, fog.offset, fog.near, fog.far],
        };
        queue.write_buffer(&self.globals_buf, 0, bytemuck::bytes_of(&globals));
        let line_buf = |lines: &[LineVertex]| {
            (!lines.is_empty()).then(|| {
                let b = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("lines"),
                    size: std::mem::size_of_val(lines) as u64,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                queue.write_buffer(&b, 0, bytemuck::cast_slice(lines));
                b
            })
        };
        let world_buf = line_buf(world_lines);
        let overlay_buf = line_buf(overlay_lines);
        for m in models {
            if m.dirty.replace(false) {
                queue.write_buffer(&m.vertex_buf, 0, bytemuck::cast_slice(&m.skinned));
            }
        }

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("scene"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.msaa,
                depth_slice: None,
                resolve_target: Some(&target.resolve_view),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r: clear[0], g: clear[1], b: clear[2], a: clear[3] }),
                    store: wgpu::StoreOp::Discard,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &target.depth,
                depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Discard }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, &self.globals_bg, &[]);
        for m in models {
            pass.set_vertex_buffer(0, m.vertex_buf.slice(..));
            for &(start, count, mat, tbg, key) in &m.draws {
                pass.set_pipeline(&self.pipelines[&key]);
                pass.set_bind_group(1, &m.material_bg, &[mat * MATERIAL_STRIDE as u32]);
                pass.set_bind_group(2, &m.texture_bgs[tbg], &[]);
                pass.draw(start..start + count, 0..1);
            }
        }
        // World lines after the models: they are depth-tested but don't write depth, so the
        // models would otherwise paint over them.
        if let Some(b) = &world_buf {
            pass.set_pipeline(&self.line_pipeline_depth);
            pass.set_bind_group(0, &self.globals_bg, &[]);
            pass.set_vertex_buffer(0, b.slice(..));
            pass.draw(0..world_lines.len() as u32, 0..1);
        }
        if let Some(b) = &overlay_buf {
            pass.set_pipeline(&self.line_pipeline_overlay);
            pass.set_bind_group(0, &self.globals_bg, &[]);
            pass.set_vertex_buffer(0, b.slice(..));
            pass.draw(0..overlay_lines.len() as u32, 0..1);
        }
    }
}

impl GpuModel {
    /// CPU skinning: every vertex follows the matrix of the bone it was loaded under.
    pub fn pose(&mut self, bone_mats: &[Mat4], root: Mat4) {
        for ((out, v), &bone) in self.skinned.iter_mut().zip(&self.base).zip(&self.bones) {
            let m = if bone == NO_BONE { root } else { root * bone_mats.get(bone as usize).copied().unwrap_or(Mat4::IDENTITY) };
            let n = Mat3::from_mat4(m) * Vec3::from(v.normal);
            out.pos = m.transform_point3(Vec3::from(v.pos)).to_array();
            out.normal = n.normalize_or_zero().to_array();
        }
        self.dirty.set(true);
    }

    /// Rewrites the uniforms of materials that read dynamic segments (scrolling tile sizes,
    /// draw-config colours) for this frame's values.
    pub fn set_segment_values(&self, queue: &wgpu::Queue, v: &SegmentValues) {
        for &i in &self.dynamic {
            let u = material_uniform_with(&self.materials[i], Some(v));
            queue.write_buffer(&self.material_buf, i as u64 * MATERIAL_STRIDE, bytemuck::bytes_of(&u));
        }
    }

    pub fn has_dynamic_materials(&self) -> bool {
        !self.dynamic.is_empty()
    }

    pub fn draw_count(&self) -> usize {
        self.draws.len()
    }
    pub fn vertex_count(&self) -> usize {
        self.base.len()
    }
    pub fn material_buffer(&self) -> &wgpu::Buffer {
        &self.material_buf
    }
}

fn write_texture(queue: &wgpu::Queue, tex: &wgpu::Texture, w: u32, h: u32, rgba: &[u8]) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo { texture: tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        rgba,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: Some(h) },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
    );
}

fn material_uniform(m: &Material) -> MaterialUniform {
    material_uniform_with(m, None)
}

fn material_uniform_with(m: &Material, dynamic: Option<&SegmentValues>) -> MaterialUniform {
    let s = m.combiner.selectors();
    let (env, prim) = dynamic.map(|v| m.colors(v)).unwrap_or((m.env, m.prim));
    let uv = dynamic.map(|v| m.uv_offsets(v)).unwrap_or_default();
    let c = |c: [u8; 4]| c.map(|x| x as f32 / 255.0);
    let threshold = match m.blend {
        BlendMode::Cutout(t) => t as f32 / 255.0,
        _ => 0.0,
    };
    let mut flags = 0u32;
    if m.lit {
        flags |= 1;
    }
    if m.texgen {
        flags |= 2;
    }
    if m.blend != BlendMode::Translucent {
        flags |= 4;
    }
    if m.fog_blend {
        flags |= 8;
    }
    if m.geometry_mode & oot_core::gbi::G_FOG != 0 {
        flags |= 16;
    }
    MaterialUniform {
        sel: [[s[0], s[1], s[2], s[3]], [s[4], s[5], s[6], s[7]], [s[8], s[9], s[10], s[11]], [s[12], s[13], s[14], s[15]]],
        prim: c(prim),
        env: c(env),
        params: [m.prim_lod_frac as f32 / 255.0, threshold, 0.0, 0.0],
        flags: [flags, m.two_cycle as u32, 0, 0],
        uv_off: [uv[0].x, uv[0].y, uv[1].x, uv[1].y],
    }
}
