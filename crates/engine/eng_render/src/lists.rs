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

use crate::{Camera, GpuModel, Lighting, LineVertex, Renderer, Target};

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
            list.push(Instance { model: r.upload(device, queue, &d), posed: None });
        }
        let inst = &mut list[n];
        if inst.posed.as_ref().is_none_or(|(t, b)| *t != cmd.transform || *b != cmd.bones) {
            inst.model.pose(&cmd.bones, cmd.transform);
            inst.posed = Some((cmd.transform, cmd.bones.clone()));
        }
        if let Some(v) = &cmd.params.segments
            && inst.model.has_dynamic_materials()
        {
            inst.model.set_segment_values(queue, v);
        }
        true
    }
}

impl Renderer {
    /// Records `lists` (OPA, then XLU) with the world lines and the lists' overlay lines.
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
        for cmd in lists.ordered() {
            let n = uses.entry(&cmd.mesh).or_default();
            if cache.prepare(self, device, queue, source, cmd, *n) {
                order.push((&cmd.mesh, *n));
            }
            *n += 1;
        }
        let models: Vec<&GpuModel> = order.iter().map(|(k, n)| &cache.meshes[*k][*n].model).collect();
        let overlay: Vec<LineVertex> = lists.overlay.iter().map(|p| LineVertex { pos: p.pos.to_array(), color: p.color }).collect();
        self.render(device, queue, encoder, target, &models, camera, light, world_lines, &overlay, clear);
    }
}
