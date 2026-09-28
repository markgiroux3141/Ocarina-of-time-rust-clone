//! Recording a frame: the globals (view, projection, lights, fog), then the posed models in
//! submission order, like the RDP, the world lines, the letterbox bars, the overlay models
//! (in the interface's orthographic projection) and the overlay lines.

use bytemuck::{Pod, Zeroable};
use glam::Vec3;

use crate::Renderer;
use crate::device::Target;
use crate::materials::MATERIAL_STRIDE;
use crate::model::GpuModel;
use crate::pipelines::LineVertex;
use crate::view::{Camera, Fog, Lighting};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Globals {
    pub(crate) view_proj: [[f32; 4]; 4],
    pub(crate) view: [[f32; 4]; 4],
    pub(crate) light_dir: [f32; 4],
    pub(crate) light_color: [f32; 4],
    pub(crate) light2_dir: [f32; 4],
    pub(crate) light2_color: [f32; 4],
    pub(crate) ambient: [f32; 4],
    pub(crate) fog_color: [f32; 4],
    /// x: multiplier, y: offset (the RSP's fm/fo), z: near, w: far of the fog projection.
    pub(crate) fog: [f32; 4],
}

/// What a frame draws over the 3D scene.
#[derive(Default)]
pub struct Screen<'a> {
    /// Models in the interface's orthographic projection (`eng_gfx::DrawLists::overlay_2d`).
    pub overlay_models: &'a [&'a GpuModel],
    /// The letterbox bars' height in rows of 240.
    pub letterbox_rows: f32,
    /// Per model of the 3D lists, whether it's drawn in the orthographic projection (a
    /// screen-space draw in the middle of the list, `eng_gfx::DrawParams::screen`). Shorter
    /// than the models (or empty): the rest are 3D.
    pub ortho_models: &'a [bool],
}

impl Renderer {
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
        self.render_screen(device, queue, encoder, target, models, camera, light, world_lines, overlay_lines, clear, &Screen::default());
    }

    /// `render`, then the letterbox and the overlay models before the overlay lines.
    #[allow(clippy::too_many_arguments)]
    pub fn render_screen(
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
        screen: &Screen,
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
        // View_ApplyOrthoToOverlay: guOrtho(-160, 160, -120, 120) on a 4:3 screen; x widens with
        // the target. No lights or fog.
        let ortho = glam::camera::rh::proj::directx::orthographic(-120.0 * aspect, 120.0 * aspect, -120.0, 120.0, -1.0, 1.0);
        let overlay_globals = Globals {
            view_proj: ortho.to_cols_array_2d(),
            view: glam::Mat4::IDENTITY.to_cols_array_2d(),
            light_dir: [0.0; 4],
            light_color: [0.0; 4],
            light2_dir: [0.0; 4],
            light2_color: [0.0; 4],
            ambient: [1.0; 4],
            fog_color: [0.0; 4],
            fog: [0.0, 0.0, 1.0, 2.0],
        };
        queue.write_buffer(&self.overlay_globals_buf, 0, bytemuck::bytes_of(&overlay_globals));
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
        // The letterbox: black bars `rows` of 240 high at the top and bottom, in clip space.
        let rows = screen.letterbox_rows.clamp(0.0, 120.0);
        let bars: Vec<LineVertex> = if rows > 0.0 {
            let h = 2.0 * rows / 240.0;
            let black = [0.0, 0.0, 0.0, 1.0];
            let quad = |y0: f32, y1: f32| {
                [[-1.0, y0], [1.0, y0], [1.0, y1], [-1.0, y0], [1.0, y1], [-1.0, y1]].map(|[x, y]| LineVertex { pos: [x, y, 0.0], color: black })
            };
            quad(1.0 - h, 1.0).into_iter().chain(quad(-1.0, -1.0 + h)).collect()
        } else {
            Vec::new()
        };
        let bars_buf = line_buf(&bars);
        for m in models.iter().chain(screen.overlay_models) {
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
        let mut in_ortho = false;
        for (i, m) in models.iter().enumerate() {
            let ortho = screen.ortho_models.get(i).copied().unwrap_or(false);
            if ortho != in_ortho {
                pass.set_bind_group(0, if ortho { &self.overlay_globals_bg } else { &self.globals_bg }, &[]);
                in_ortho = ortho;
            }
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
        if let Some(b) = &bars_buf {
            pass.set_pipeline(&self.fill_pipeline);
            pass.set_bind_group(0, &self.globals_bg, &[]);
            pass.set_vertex_buffer(0, b.slice(..));
            pass.draw(0..bars.len() as u32, 0..1);
        }
        if !screen.overlay_models.is_empty() {
            pass.set_bind_group(0, &self.overlay_globals_bg, &[]);
            for m in screen.overlay_models {
                pass.set_vertex_buffer(0, m.vertex_buf.slice(..));
                for &(start, count, mat, tbg, key) in &m.draws {
                    pass.set_pipeline(&self.pipelines[&key]);
                    pass.set_bind_group(1, &m.material_bg, &[mat * MATERIAL_STRIDE as u32]);
                    pass.set_bind_group(2, &m.texture_bgs[tbg], &[]);
                    pass.draw(start..start + count, 0..1);
                }
            }
        }
        if let Some(b) = &overlay_buf {
            pass.set_pipeline(&self.line_pipeline_overlay);
            pass.set_bind_group(0, &self.globals_bg, &[]);
            pass.set_vertex_buffer(0, b.slice(..));
            pass.draw(0..overlay_lines.len() as u32, 0..1);
        }
    }
}
