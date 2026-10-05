//! The 3D view: the built level in perspective, textured with its baked lighting as the N64
//! draws it (texture x vertex colour; cutouts alpha-tested, water blended), with an orbit
//! camera. Rendered offscreen with the window's own wgpu device and shown as an egui image,
//! as the OoT Clone's viewer does.

use crate::scene::Mat;
use bytemuck::{Pod, Zeroable};
use eframe::egui;
use eframe::egui_wgpu::{self, wgpu};
use glam::{Mat4, Vec3, Vec4};
use overworld::textures::Library;
use overworld::Level;
use std::collections::HashMap;
use wgpu::util::DeviceExt;

const COLOR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const SAMPLES: u32 = 4;
pub const FOV: f32 = 50.0;
const SKY: [f64; 3] = [0.55, 0.70, 0.86];

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub target: Vec3,
    /// Radians: 0 looks from the east, -90 degrees from the south.
    pub yaw: f32,
    pub pitch: f32,
    pub dist: f32,
}

impl Camera {
    pub fn eye(&self) -> Vec3 {
        let (cp, sp) = (self.pitch.cos(), self.pitch.sin());
        self.target + Vec3::new(cp * self.yaw.cos(), cp * self.yaw.sin(), sp) * self.dist
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        let near = (self.dist * 0.01).clamp(2.0, 50.0);
        let proj = glam::camera::rh::proj::directx::perspective(FOV.to_radians(), aspect.max(0.01), near, self.dist * 8.0 + 60000.0);
        proj * glam::camera::rh::view::look_at_mat4(self.eye(), self.target, Vec3::Z)
    }

    /// The ray through a point in the view (`ndc` in -1..1, y up).
    pub fn ray(&self, aspect: f32, ndc: [f32; 2]) -> (Vec3, Vec3) {
        let inv = self.view_proj(aspect).inverse();
        let a = inv * Vec4::new(ndc[0], ndc[1], 0.0, 1.0);
        let b = inv * Vec4::new(ndc[0], ndc[1], 1.0, 1.0);
        let (a, b) = (a.truncate() / a.w, b.truncate() / b.w);
        (a, (b - a).normalize())
    }

    /// Screen position of a world point, if it's in front of the camera.
    pub fn project(&self, rect: egui::Rect, p: Vec3) -> Option<egui::Pos2> {
        let c = self.view_proj(rect.width() / rect.height()) * p.extend(1.0);
        if c.w <= 1e-3 {
            return None;
        }
        let n = c.truncate() / c.w;
        Some(egui::Pos2::new(rect.left() + (n.x * 0.5 + 0.5) * rect.width(), rect.top() + (0.5 - n.y * 0.5) * rect.height()))
    }

    /// Orbit drag: yaw and pitch.
    pub fn orbit(&mut self, d: egui::Vec2) {
        self.yaw -= d.x * 0.008;
        self.pitch = (self.pitch + d.y * 0.008).clamp(0.03, 1.55);
    }

    /// Pan drag: the target moves with the pointer, across the view.
    pub fn pan(&mut self, d: egui::Vec2, view_h: f32) {
        let fwd = (self.target - self.eye()).normalize();
        let right = fwd.cross(Vec3::Z).normalize();
        let up = right.cross(fwd);
        let k = 2.0 * self.dist * (FOV.to_radians() * 0.5).tan() / view_h.max(1.0);
        self.target += (-right * d.x + up * d.y) * k;
    }

    pub fn zoom(&mut self, f: f32) {
        self.dist = (self.dist * f).clamp(40.0, 200000.0);
    }

    /// Looks at a box (x0, y0, x1, y1) at height z from the south, from above, far enough back
    /// that it fits a view of this aspect (width / height).
    pub fn frame(&mut self, bb: [f64; 4], z: f64, aspect: f32) {
        let (w, h) = (((bb[2] - bb[0]) as f32).max(500.0), ((bb[3] - bb[1]) as f32).max(500.0));
        self.target = Vec3::new(((bb[0] + bb[2]) * 0.5) as f32, ((bb[1] + bb[3]) * 0.5) as f32, z as f32);
        self.yaw = -90f32.to_radians();
        self.pitch = 50f32.to_radians();
        let half_v = (FOV.to_radians() * 0.5).tan();
        let half_h = half_v * aspect.max(0.1);
        // across: the level's width against the horizontal field; up the screen: its depth, foreshortened
        self.dist = (0.5 * w / half_h).max(0.5 * h * self.pitch.sin() / half_v) * 1.1;
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 3],
    uv: [f32; 2],
    color: [u8; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    view_proj: [[f32; 4]; 4],
    eye: [f32; 4],
    /// Fog start and end distances.
    fog: [f32; 4],
    sky: [f32; 4],
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Kind {
    Opaque,
    Cutout,
    Blend,
}

struct Batch {
    vbuf: wgpu::Buffer,
    count: u32,
    bind: wgpu::BindGroup,
    kind: Kind,
    cull: bool,
    decal: bool,
}

struct Target {
    size: (u32, u32),
    msaa: wgpu::TextureView,
    depth: wgpu::TextureView,
    resolve: wgpu::TextureView,
}

pub struct View3d {
    rs: egui_wgpu::RenderState,
    shader: wgpu::ShaderModule,
    layout: wgpu::PipelineLayout,
    tex_layout: wgpu::BindGroupLayout,
    globals: wgpu::Buffer,
    globals_bg: wgpu::BindGroup,
    pipelines: HashMap<(Kind, bool, bool), wgpu::RenderPipeline>,
    batches: Vec<Batch>,
    /// By texture name.
    textures: HashMap<String, wgpu::TextureView>,
    white: wgpu::TextureView,
    target: Option<(Target, egui::TextureId)>,
    pub cam: Camera,
    pub framed: bool,
    /// The last rendered view's width / height.
    pub aspect: f32,
}

impl View3d {
    pub fn new(rs: &egui_wgpu::RenderState) -> View3d {
        let device = &rs.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("level"),
            source: wgpu::ShaderSource::Wgsl(include_str!("view3d.wgsl").into()),
        });
        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("level"),
            bind_group_layouts: &[Some(&globals_layout), Some(&tex_layout)],
            immediate_size: 0,
        });
        let globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let globals_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals"),
            layout: &globals_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: globals.as_entire_binding() }],
        });
        let white = upload_texture(device, &rs.queue, &image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 255, 255, 255])));
        View3d {
            rs: rs.clone(),
            shader,
            layout,
            tex_layout,
            globals,
            globals_bg,
            pipelines: HashMap::new(),
            batches: vec![],
            textures: HashMap::new(),
            white,
            target: None,
            cam: Camera { target: Vec3::ZERO, yaw: -90f32.to_radians(), pitch: 0.7, dist: 4000.0 },
            framed: false,
            aspect: 1.0,
        }
    }

    /// A pipeline per alpha kind, culling and decal (drawn on the surface it lies on: pulled
    /// towards the eye, as the N64's decal depth mode lets it win ties).
    fn pipeline(&mut self, kind: Kind, cull: bool, decal: bool) {
        if self.pipelines.contains_key(&(kind, cull, decal)) {
            return;
        }
        let fs = match kind {
            Kind::Opaque => "fs_opaque",
            Kind::Cutout => "fs_cutout",
            Kind::Blend => "fs_blend",
        };
        let p = self.rs.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(fs),
            layout: Some(&self.layout),
            vertex: wgpu::VertexState {
                module: &self.shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Unorm8x4],
                })],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: if cull { Some(wgpu::Face::Back) } else { None },
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: Some(kind != Kind::Blend),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: if decal { wgpu::DepthBiasState { constant: -8, slope_scale: -2.0, clamp: 0.0 } } else { Default::default() },
            }),
            multisample: wgpu::MultisampleState { count: SAMPLES, ..Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &self.shader,
                entry_point: Some(fs),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: COLOR,
                    blend: (kind == Kind::Blend).then_some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        self.pipelines.insert((kind, cull, decal), p);
    }

    /// Uploads a built level: one batch per material.
    pub fn set_level(&mut self, lvl: &Level, mats: &[Mat], lib: Option<&Library>) {
        let mut verts: Vec<Vec<Vertex>> = vec![vec![]; mats.len()];
        for o in lvl.mesh.objects.iter().filter(|o| !o.collision_only) {
            for (t, tri) in o.tris.iter().enumerate() {
                let m = o.mat[t];
                let alpha = if mats[m].info.alpha == "blend" { (mats[m].info.opacity * 255.0) as u8 } else { 255 };
                for c in 0..3 {
                    let v = o.verts[tri[c]];
                    let col = o.colors.get(tri[c]).copied().unwrap_or([255, 255, 255]);
                    let uv = o.uvs[t][c];
                    // a blend material's alpha is how much of its second texture shows
                    let a = match (&mats[m].overlay, o.blend.get(tri[c])) {
                        (Some(_), Some(w)) => (w * 255.0).round().clamp(0.0, 255.0) as u8,
                        (Some(_), None) => 0,
                        _ => alpha,
                    };
                    verts[m].push(Vertex {
                        pos: [v[0] as f32, v[1] as f32, v[2] as f32],
                        uv: [uv[0] as f32, 1.0 - uv[1] as f32],
                        color: [col[0], col[1], col[2], a],
                    });
                }
            }
        }
        let device = self.rs.device.clone();
        let queue = self.rs.queue.clone();
        self.batches.clear();
        for (m, vs) in verts.into_iter().enumerate() {
            if vs.is_empty() {
                continue;
            }
            let info = &mats[m].info;
            let name = &mats[m].name;
            for n in std::iter::once(name).chain(mats[m].overlay.as_ref()) {
                if !self.textures.contains_key(n) {
                    if let Some((w, h, px)) = lib.and_then(|l| l.rgba(n)) {
                        if let Some(img) = image::RgbaImage::from_raw(w, h, px) {
                            let v = upload_texture(&device, &queue, &img);
                            self.textures.insert(n.clone(), v);
                        }
                    }
                }
            }
            let view = self.textures.get(name).unwrap_or(&self.white);
            let view2 = mats[m].overlay.as_ref().and_then(|o| self.textures.get(o)).unwrap_or(view);
            let wrap = |w: &str| match w {
                "clamp" => wgpu::AddressMode::ClampToEdge,
                "mirror" => wgpu::AddressMode::MirrorRepeat,
                _ => wgpu::AddressMode::Repeat,
            };
            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("material"),
                address_mode_u: wrap(&info.wrap_u),
                address_mode_v: wrap(&info.wrap_v),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Linear,
                ..Default::default()
            });
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("material"),
                layout: &self.tex_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(view2) },
                ],
            });
            let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("material verts"),
                contents: bytemuck::cast_slice(&vs),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let kind = match info.alpha.as_str() {
                "cutout" => Kind::Cutout,
                "blend" => Kind::Blend,
                _ => Kind::Opaque,
            };
            let cull = info.cull != "none";
            self.pipeline(kind, cull, info.decal);
            self.batches.push(Batch { vbuf, count: vs.len() as u32, bind, kind, cull, decal: info.decal });
        }
        // opaque, then cutouts, then blended (water) last
        self.batches.sort_by_key(|b| b.kind as u8);
    }

    /// Renders the view at `w` x `h` pixels and returns its egui texture.
    pub fn render(&mut self, w: u32, h: u32) -> egui::TextureId {
        let (w, h) = (w.max(1), h.max(1));
        self.aspect = w as f32 / h as f32;
        let device = self.rs.device.clone();
        if self.target.as_ref().is_none_or(|(t, _)| t.size != (w, h)) {
            let t = make_target(&device, w, h);
            let mut er = self.rs.renderer.write();
            let id = match self.target.take() {
                Some((_, id)) => {
                    er.update_egui_texture_from_wgpu_texture(&device, &t.resolve, wgpu::FilterMode::Linear, id);
                    id
                }
                None => er.register_native_texture(&device, &t.resolve, wgpu::FilterMode::Linear),
            };
            self.target = Some((t, id));
        }
        let eye = self.cam.eye();
        let g = Globals {
            view_proj: self.cam.view_proj(w as f32 / h as f32).to_cols_array_2d(),
            eye: [eye.x, eye.y, eye.z, 1.0],
            fog: [self.cam.dist * 1.6 + 2000.0, self.cam.dist * 5.0 + 9000.0, 0.0, 0.0],
            sky: [SKY[0] as f32, SKY[1] as f32, SKY[2] as f32, 1.0],
        };
        self.rs.queue.write_buffer(&self.globals, 0, bytemuck::bytes_of(&g));
        let (t, id) = self.target.as_ref().unwrap();
        let mut enc = device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("level"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &t.msaa,
                    depth_slice: None,
                    resolve_target: Some(&t.resolve),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: SKY[0], g: SKY[1], b: SKY[2], a: 1.0 }),
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &t.depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Discard }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.globals_bg, &[]);
            for b in &self.batches {
                pass.set_pipeline(&self.pipelines[&(b.kind, b.cull, b.decal)]);
                pass.set_bind_group(1, &b.bind, &[]);
                pass.set_vertex_buffer(0, b.vbuf.slice(..));
                pass.draw(0..b.count, 0..1);
            }
        }
        self.rs.queue.submit([enc.finish()]);
        *id
    }
}

fn make_target(device: &wgpu::Device, w: u32, h: u32) -> Target {
    let size = wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 };
    let mk = |label, format, samples, usage| {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size,
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
            .create_view(&Default::default())
    };
    Target {
        size: (w, h),
        msaa: mk("msaa", COLOR, SAMPLES, wgpu::TextureUsages::RENDER_ATTACHMENT),
        depth: mk("depth", DEPTH, SAMPLES, wgpu::TextureUsages::RENDER_ATTACHMENT),
        resolve: mk("resolve", COLOR, 1, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING),
    }
}

/// A texture with its mip chain (box-filtered on the CPU), so distant ground doesn't shimmer.
fn upload_texture(device: &wgpu::Device, queue: &wgpu::Queue, img: &image::RgbaImage) -> wgpu::TextureView {
    let (w, h) = img.dimensions();
    let levels = 32 - w.max(h).max(1).leading_zeros();
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("material"),
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: levels,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: COLOR,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let mut cur = img.clone();
    for level in 0..levels {
        let (lw, lh) = cur.dimensions();
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &tex, mip_level: level, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            cur.as_raw(),
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(lw * 4), rows_per_image: Some(lh) },
            wgpu::Extent3d { width: lw, height: lh, depth_or_array_layers: 1 },
        );
        if level + 1 < levels {
            cur = image::imageops::resize(&cur, (lw / 2).max(1), (lh / 2).max(1), image::imageops::FilterType::Triangle);
        }
    }
    tex.create_view(&Default::default())
}
