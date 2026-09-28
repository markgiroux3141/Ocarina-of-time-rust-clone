//! Uploaded meshes: vertex and material buffers, texture bind groups, the draw ranges, and
//! CPU skinning (every vertex follows the matrix of the bone it was loaded under).

use std::cell::Cell;
use std::collections::HashMap;

use eng_gfx::texture::WrapMode;
use eng_gfx::{BlendMode, DrawList, Material, NO_BONE, SegmentValues, TextureSlot};
use glam::{Mat3, Mat4, Vec3};

use crate::Renderer;
use crate::device::COLOR_FORMAT;
use crate::materials::{MATERIAL_STRIDE, MaterialUniform, material_uniform, material_uniform_with};
use crate::pipelines::{GpuVertex, PipelineKey};

pub struct GpuModel {
    pub(crate) vertex_buf: wgpu::Buffer,
    /// The skinned vertices changed since the last upload.
    pub(crate) dirty: Cell<bool>,
    pub(crate) materials: Vec<Material>,
    /// Materials whose uniforms depend on dynamic segments.
    pub(crate) dynamic: Vec<usize>,
    pub(crate) material_buf: wgpu::Buffer,
    pub(crate) material_bg: wgpu::BindGroup,
    pub(crate) texture_bgs: Vec<wgpu::BindGroup>,
    /// (vertex start, count, material index, texture bind group index, pipeline key)
    pub(crate) draws: Vec<(u32, u32, u32, usize, PipelineKey)>,
    pub(crate) base: Vec<GpuVertex>,
    pub(crate) bones: Vec<u16>,
    pub(crate) skinned: Vec<GpuVertex>,
}

impl Renderer {
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
}

impl GpuModel {
    /// CPU skinning: every vertex follows the matrix of the bone it was loaded under. A
    /// projective `root` (a bottom row other than `0 0 0 1`, such as a perspective placed on
    /// part of the overlay) is divided by w here, so the texture is interpolated affinely
    /// across each triangle.
    pub fn pose(&mut self, bone_mats: &[Mat4], root: Mat4) {
        let projective = root.row(3) != glam::Vec4::W;
        for ((out, v), &bone) in self.skinned.iter_mut().zip(&self.base).zip(&self.bones) {
            let m = if bone == NO_BONE { root } else { root * bone_mats.get(bone as usize).copied().unwrap_or(Mat4::IDENTITY) };
            let n = Mat3::from_mat4(m) * Vec3::from(v.normal);
            let p = Vec3::from(v.pos);
            out.pos = if projective { m.project_point3(p) } else { m.transform_point3(p) }.to_array();
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

pub(crate) fn write_texture(queue: &wgpu::Queue, tex: &wgpu::Texture, w: u32, h: u32, rgba: &[u8]) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo { texture: tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        rgba,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: Some(h) },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
    );
}
