//! Drawing submitted lists (`eng_gfx::DrawLists`): the OPA list, then the XLU list, in order.
//!
//! Meshes are uploaded the first time their key is drawn, from the app's `MeshSource`. A key
//! drawn several times in one frame gets one GPU instance per use (the N-th draw of a key uses
//! its N-th instance), since each carries its own pose and material values. A draw's segment
//! values are written into the instance's dynamic materials; its transform and bones re-skin
//! the instance only when they changed.

use std::collections::HashMap;

use eng_gfx::{DrawCmd, DrawList, DrawLists, MeshKey};
use glam::Mat4;

use crate::{Camera, GpuModel, Lighting, LineVertex, Renderer, Screen, Target};

/// Where meshes come from: the app resolves the game's mesh keys (asset-pack records and its
/// own built-in meshes).
pub trait MeshSource {
    /// The mesh for `key`, or `None` if there's none (the draw is skipped, once logged).
    fn mesh(&mut self, key: &MeshKey) -> Option<DrawList>;
}

struct Instance {
    model: GpuModel,
    /// The pose it was last skinned with.
    posed: Option<(Mat4, Vec<Mat4>)>,
    /// The point lights its lit materials were last written with.
    lights: Vec<eng_gfx::PointLight>,
    /// The fog its fogged materials were last written with.
    fog: Option<eng_gfx::FogOverride>,
    /// The vertex colours it was last given (`DrawParams::vertex_colors`).
    vertex_colors: Option<Vec<[u8; 4]>>,
    /// The image it was last given (`DrawParams::image`).
    image: Option<eng_gfx::DrawImage>,
    /// The textures it was last given by source (`DrawParams::texture_images`).
    texture_images: Vec<eng_gfx::SourceImage>,
}

/// Uploaded meshes by key, with their per-frame instances.
#[derive(Default)]
pub struct MeshCache {
    meshes: HashMap<MeshKey, Vec<Instance>>,
    missing: HashMap<MeshKey, ()>,
}

impl MeshCache {
    pub fn new() -> MeshCache {
        MeshCache::default()
    }

    /// Drops every uploaded mesh (after a scene change).
    pub fn clear(&mut self) {
        self.meshes.clear();
        self.missing.clear();
    }

    /// How many distinct meshes are uploaded.
    pub fn len(&self) -> usize {
        self.meshes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.meshes.is_empty()
    }

    /// Gets the `n`-th instance of `cmd`'s mesh ready for this draw.
    fn prepare(&mut self, r: &mut Renderer, device: &wgpu::Device, queue: &wgpu::Queue, source: &mut dyn MeshSource, cmd: &DrawCmd, n: usize) -> bool {
        if self.missing.contains_key(&cmd.mesh) {
            return false;
        }
        let list = self.meshes.entry(cmd.mesh.clone()).or_default();
        while list.len() <= n {
            let Some(d) = source.mesh(&cmd.mesh) else {
                log::warn!("no mesh {:?}", cmd.mesh);
                self.missing.insert(cmd.mesh.clone(), ());
                return false;
            };
            list.push(Instance { model: r.upload(device, queue, &d), posed: None, lights: Vec::new(), fog: None, vertex_colors: None, image: None, texture_images: Vec::new() });
        }
        let inst = &mut list[n];
        if inst.posed.as_ref().is_none_or(|(t, b)| *t != cmd.transform || *b != cmd.bones) {
            inst.model.pose(&cmd.bones, cmd.transform);
            inst.posed = Some((cmd.transform, cmd.bones.clone()));
        }
        if inst.vertex_colors != cmd.params.vertex_colors {
            inst.model.set_vertex_colors(cmd.params.vertex_colors.as_deref());
            inst.vertex_colors = cmd.params.vertex_colors.clone();
        }
        if inst.image != cmd.params.image || inst.texture_images != cmd.params.texture_images {
            r.set_images(device, queue, &mut inst.model, cmd.params.image.as_ref(), &cmd.params.texture_images);
            inst.image = cmd.params.image.clone();
            inst.texture_images = cmd.params.texture_images.clone();
        }
        let lights_changed = inst.lights != cmd.params.lights;
        let fog_changed = inst.fog != cmd.params.fog;
        if (cmd.params.segments.is_some() && inst.model.has_dynamic_materials()) || lights_changed || fog_changed {
            inst.model.set_draw_values(queue, cmd.params.segments.as_ref(), &cmd.params.lights, cmd.params.fog.as_ref(), fog_changed);
            inst.lights = cmd.params.lights.clone();
            inst.fog = cmd.params.fog;
        }
        true
    }
}

impl Renderer {
    /// Records `lists` (OPA, then XLU) with the world lines, then the letterbox, the overlay
    /// meshes and the overlay lines.
    #[allow(clippy::too_many_arguments)]
    pub fn render_lists(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &Target,
        lists: &DrawLists,
        cache: &mut MeshCache,
        source: &mut dyn MeshSource,
        camera: &Camera,
        light: &Lighting,
        world_lines: &[LineVertex],
        clear: [f64; 4],
    ) {
        let mut uses: HashMap<&MeshKey, usize> = HashMap::new();
        let mut order: Vec<(&MeshKey, usize)> = Vec::new();
        let mut ortho: Vec<bool> = Vec::new();
        let mut order_2d: Vec<(&MeshKey, usize)> = Vec::new();
        let mut order_pause: Vec<(&MeshKey, usize)> = Vec::new();
        let mut ortho_pause: Vec<bool> = Vec::new();
        let mut opa_models = 0;
        let n_opa = lists.opa.len();
        // 0: the 3D lists, 1: the pause menu's, 2: the overlay's.
        let all = lists.ordered().map(|c| (c, 0)).chain(lists.pause.iter().map(|c| (c, 1))).chain(lists.overlay_2d.iter().map(|c| (c, 2)));
        for (k, (cmd, list)) in all.enumerate() {
            if k == n_opa {
                opa_models = order.len();
            }
            let n = uses.entry(&cmd.mesh).or_default();
            if cache.prepare(self, device, queue, source, cmd, *n) {
                if list == 2 {
                    order_2d.push((&cmd.mesh, *n));
                } else if list == 1 {
                    order_pause.push((&cmd.mesh, *n));
                    ortho_pause.push(cmd.params.screen);
                } else {
                    order.push((&cmd.mesh, *n));
                    ortho.push(cmd.params.screen);
                }
            }
            *n += 1;
        }
        let models: Vec<&GpuModel> = order.iter().map(|(k, n)| &cache.meshes[*k][*n].model).collect();
        let models_2d: Vec<&GpuModel> = order_2d.iter().map(|(k, n)| &cache.meshes[*k][*n].model).collect();
        let models_pause: Vec<&GpuModel> = order_pause.iter().map(|(k, n)| &cache.meshes[*k][*n].model).collect();
        let overlay: Vec<LineVertex> = lists.overlay.iter().map(|p| LineVertex { pos: p.pos.to_array(), color: p.color }).collect();
        if n_opa >= lists.opa.len() + lists.xlu.len() + lists.pause.len() + lists.overlay_2d.len() {
            opa_models = order.len();
        }
        let screen = Screen {
            overlay_models: &models_2d,
            pause_models: &models_pause,
            pause_view: lists.pause_view,
            pause_ortho: &ortho_pause,
            letterbox_rows: lists.letterbox_rows,
            ortho_models: &ortho,
            opa_models,
            opa_fill: lists.opa_fill,
            xlu_fill: lists.xlu_fill,
            overlay_fill: lists.overlay_fill,
        };
        self.render_screen(device, queue, encoder, target, &models, camera, light, world_lines, &overlay, clear, &screen);
    }
}
