//! The depth probe (`eng_gfx::DrawLists::depth_probe`): one pixel of a frame's depth, read back
//! once the frame is drawn, as a game reads its depth buffer after the frame
//! (docs/adr/0056-the-clock.md). A one-pixel pass loads the multisampled depth at the pixel
//! (sample 0) into an `R32Float` target, which is copied into a buffer the caller maps after the
//! frame's submit (`Renderer::read_depth_probe`).

use crate::device::Target;

const SHADER: &str = r#"
@group(0) @binding(0) var depth: texture_depth_multisampled_2d;
@group(0) @binding(1) var<uniform> pixel: vec4<i32>;

@vertex
fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    var p = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    return vec4<f32>(p[i], 0.0, 1.0);
}

@fragment
fn fs() -> @location(0) vec4<f32> {
    return vec4<f32>(textureLoad(depth, pixel.xy, 0), 0.0, 0.0, 1.0);
}
"#;

pub(crate) struct DepthProbe {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
    target_view: wgpu::TextureView,
    target: wgpu::Texture,
    readback: wgpu::Buffer,
    /// What the next read returns: a copy in flight, or a pixel outside the target (0).
    pending: Option<Pending>,
}

enum Pending {
    Copied,
    Outside,
}

impl DepthProbe {
    pub(crate) fn new(device: &wgpu::Device) -> DepthProbe {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("depth probe"), source: wgpu::ShaderSource::Wgsl(SHADER.into()) });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("depth probe"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Depth, view_dimension: wgpu::TextureViewDimension::D2, multisampled: true },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("depth probe"), bind_group_layouts: &[Some(&layout)], immediate_size: 0 });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("depth probe"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs"), compilation_options: Default::default(), buffers: &[] },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::R32Float, blend: None, write_mask: wgpu::ColorWrites::ALL })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let uniform =
            device.create_buffer(&wgpu::BufferDescriptor { label: Some("depth probe pixel"), size: 16, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth probe"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("depth probe readback"),
            size: 256,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        DepthProbe { layout, pipeline, uniform, target_view: target.create_view(&Default::default()), target, readback, pending: None }
    }

    /// Records the read of the 320x240 screen's `pixel` (x right, y down) from `target`'s depth,
    /// mapped onto the target as the 3D view is (the height scales it, a wider target widens
    /// about the centre).
    pub(crate) fn encode(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder, target: &Target, pixel: [i32; 2]) {
        let (w, h) = (target.size.0 as f32, target.size.1 as f32);
        let s = h / 240.0;
        let x = (w / 2.0 + (pixel[0] as f32 + 0.5 - 160.0) * s).floor();
        let y = ((pixel[1] as f32 + 0.5) * s).floor();
        if x < 0.0 || y < 0.0 || x >= w || y >= h {
            self.pending = Some(Pending::Outside);
            return;
        }
        queue.write_buffer(&self.uniform, 0, bytemuck::cast_slice(&[x as i32, y as i32, 0, 0]));
        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("depth probe"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&target.depth) },
                wgpu::BindGroupEntry { binding: 1, resource: self.uniform.as_entire_binding() },
            ],
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("depth probe"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.target_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo { texture: &self.target, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyBufferInfo { buffer: &self.readback, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(256), rows_per_image: Some(1) } },
            wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        );
        self.pending = Some(Pending::Copied);
    }

    /// The depth the last frame's probe read (0 for a pixel outside the target; 1 is the far
    /// plane, where nothing was drawn), once its commands are submitted. `None` without a probe.
    pub(crate) fn read(&mut self, device: &wgpu::Device) -> Option<f32> {
        match self.pending.take()? {
            Pending::Outside => Some(0.0),
            Pending::Copied => {
                let slice = self.readback.slice(..4);
                let (tx, rx) = std::sync::mpsc::channel();
                slice.map_async(wgpu::MapMode::Read, move |r| {
                    let _ = tx.send(r);
                });
                device.poll(wgpu::PollType::wait_indefinitely()).ok()?;
                rx.recv().ok()?.ok()?;
                let d = {
                    let data = slice.get_mapped_range().ok()?;
                    f32::from_le_bytes([data[0], data[1], data[2], data[3]])
                };
                self.readback.unmap();
                Some(d)
            }
        }
    }
}
