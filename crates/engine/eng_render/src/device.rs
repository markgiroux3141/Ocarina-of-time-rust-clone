//! The GPU device for headless use and the offscreen render target.

use anyhow::{Context, Result};

pub const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
pub(crate) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub const SAMPLES: u32 = 4;

pub struct Target {
    pub size: (u32, u32),
    pub(crate) msaa: wgpu::TextureView,
    pub(crate) depth: wgpu::TextureView,
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

/// The optional features the renderer asks for: `DEPTH_CLIP_CONTROL`, for the game's NoN
/// microcode (no near-plane clipping: `pipelines.rs`, `shader.wgsl`). Without it the near plane
/// clips as on any GPU.
pub const NON_FEATURES: wgpu::Features = wgpu::Features::DEPTH_CLIP_CONTROL;

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
    let desc = wgpu::DeviceDescriptor { required_features: adapter.features() & NON_FEATURES, ..Default::default() };
    let (device, queue) = pollster::block_on(adapter.request_device(&desc))?;
    Ok((device, queue))
}
