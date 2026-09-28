//! A real scene loaded from the ROM for play: rooms interpreted by `oot_core::room`, the
//! environment's lights and fog from `oot_game::env`, and the per-frame draw order of
//! `Play_Draw` / `Room_Draw`.

use std::collections::BTreeSet;

use anyhow::Result;
use glam::{Mat4, Vec3};
use oot_core::drawcfg;
use oot_core::gbi::SegmentValues;
use oot_core::project::Project;
use oot_core::room::{RoomMesh, SceneDraw, SceneTables, ShapeKind, cullable_order, gu_perspective};
use oot_core::scene;
use oot_game::env::{self, EnvLights, EnvState, EnvTables};
use oot_render::{Fog, GpuModel, Lighting, Renderer};

/// `View_Init`: `zNear` 10 (the far plane is `lightCtx.fogFar`, set in `Play_Draw`).
pub const Z_NEAR: f32 = 10.0;
/// `View_Init`: `fovy` 60.
pub const FOVY: f32 = 60.0;

/// CPU side of a loaded scene.
pub struct LoadedScene {
    pub draw: SceneDraw,
    pub meshes: Vec<RoomMesh>,
    pub lights: EnvLights,
    pub state: drawcfg::State,
    pub notes: BTreeSet<String>,
}

impl LoadedScene {
    /// Loads `name` for Link's age at `day_time` (`gSaveContext.dayTime`), picking the scene
    /// layer the way the game does for a non-cutscene entrance.
    pub fn load(p: &Project, tables: &SceneTables, env_tables: &EnvTables, name: &str, child: bool, day_time: u16) -> Result<LoadedScene> {
        // Environment_Init: nightFlag = dayTime > 18:00 || dayTime < 6:30.
        let night = day_time > env::clock_time(18, 0) as u16 || day_time < env::clock_time(6, 30) as u16;
        let layer = match (child, night) {
            (true, false) => scene::LAYER_CHILD_DAY,
            (true, true) => scene::LAYER_CHILD_NIGHT,
            (false, false) => scene::LAYER_ADULT_DAY,
            (false, true) => scene::LAYER_ADULT_NIGHT,
        };
        let draw = SceneDraw::load(p, tables, name, layer)?;
        // Room 0's time settings (Scene_CommandTimeSettings runs from the room header).
        let (day, sky, _speed) = env::scene_times(day_time, draw.rooms.first().and_then(|r| r.time));
        let lights = env::update(
            env_tables,
            &draw.scene.light_settings,
            &EnvState { light_mode: draw.scene.skybox.light_mode, light_config: 0, light_setting: 0, day_time: day, skybox_time: sky },
        );
        let state = drawcfg::State { gameplay_frames: 0, child, night, scene_layer: layer as i64, day_time: day };
        let mut notes = BTreeSet::new();
        let meshes = draw.build(p, tables, &state, &mut notes);
        Ok(LoadedScene { draw, meshes, lights, state, notes })
    }

    pub fn segment_values(&self, p: &Project, tables: &SceneTables, gameplay_frames: u32) -> [SegmentValues; 2] {
        let st = drawcfg::State { gameplay_frames, ..self.state.clone() };
        self.draw.segment_values(p, tables, &st)
    }

    /// The renderer's lights and fog for this environment.
    pub fn lighting(&self) -> Lighting {
        let l = &self.lights;
        let c = |v: [u8; 3]| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32) / 255.0;
        let d = |v: [i8; 3]| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32);
        // Play_SetFog: Gfx_SetFog2(fogColor, 0, fogNear, 1000).
        let (fm, fo) = env::fog_factor(l.fog_near as i32, 1000);
        Lighting {
            dir: d(l.light1_dir),
            color: c(l.light1_color),
            dir2: d(l.light2_dir),
            color2: c(l.light2_color),
            ambient: c(l.ambient),
            fog: Some(Fog { color: c(l.fog_color), multiplier: fm as f32, offset: fo as f32, near: Z_NEAR, far: self.fog_far() }),
        }
    }

    /// `lightCtx.fogFar`, which `Play_Draw` also uses as the far plane.
    pub fn fog_far(&self) -> f32 {
        self.lights.fog_far as f32
    }

    /// Background: with no skybox drawn (`SKYBOX_UNSET_1D`, or skyboxes not ported),
    /// `Environment_DrawSkyboxFilters` fills the screen with the fog colour.
    pub fn clear_color(&self) -> [f64; 4] {
        let f = self.lights.fog_color;
        [f[0] as f64 / 255.0, f[1] as f64 / 255.0, f[2] as f64 / 255.0, 1.0]
    }

    pub fn triangles(&self) -> usize {
        self.meshes.iter().flat_map(|r| &r.entries).flat_map(|e| [&e.opa, &e.xlu]).flatten().map(|d| d.triangle_count()).sum()
    }
}

struct EntryGpu {
    bounds: Option<(Vec3, f32)>,
    opa: Option<GpuModel>,
    xlu: Option<GpuModel>,
}

struct RoomGpu {
    kind: ShapeKind,
    entries: Vec<EntryGpu>,
}

/// The rooms on the GPU.
pub struct RoomsGfx {
    rooms: Vec<RoomGpu>,
    last_frame: Option<u32>,
    pub drawn_entries: usize,
}

impl RoomsGfx {
    pub fn upload(r: &mut Renderer, device: &wgpu::Device, queue: &wgpu::Queue, meshes: &[RoomMesh]) -> RoomsGfx {
        let mut up = |d: &Option<oot_core::gbi::DrawList>| {
            d.as_ref().filter(|d| !d.batches.is_empty()).map(|d| {
                let mut m = r.upload(device, queue, d);
                m.pose(&[], Mat4::IDENTITY);
                m
            })
        };
        let rooms = meshes
            .iter()
            .map(|rm| RoomGpu {
                kind: rm.kind,
                entries: rm.entries.iter().map(|e| EntryGpu { bounds: e.bounds, opa: up(&e.opa), xlu: up(&e.xlu) }).collect(),
            })
            .collect();
        RoomsGfx { rooms, last_frame: None, drawn_entries: 0 }
    }

    /// Updates the dynamic-segment materials when the game frame changed.
    pub fn update(&mut self, queue: &wgpu::Queue, frame: u32, values: impl FnOnce() -> [SegmentValues; 2]) {
        if self.last_frame == Some(frame) {
            return;
        }
        self.last_frame = Some(frame);
        let [opa, xlu] = values();
        for e in self.rooms.iter().flat_map(|r| &r.entries) {
            if let Some(m) = &e.opa
                && m.has_dynamic_materials()
            {
                m.set_segment_values(queue, &opa);
            }
            if let Some(m) = &e.xlu
                && m.has_dynamic_materials()
            {
                m.set_segment_values(queue, &xlu);
            }
        }
    }

    /// The OPA and XLU models in `Room_Draw` order for this view: every room's entries in
    /// shape order, cullable shapes z-sorted and depth-culled (`Room_DrawCullable`).
    pub fn ordered(&mut self, view: Mat4, fog_far: f32) -> (Vec<&GpuModel>, Vec<&GpuModel>) {
        let vp = gu_perspective(FOVY, 4.0 / 3.0, Z_NEAR, fog_far) * view;
        let clip_z = |p: Vec3| (vp * p.extend(1.0)).z;
        let (mut opa, mut xlu) = (Vec::new(), Vec::new());
        let mut drawn = 0;
        for room in &self.rooms {
            let order: Vec<usize> = match room.kind {
                ShapeKind::Cullable => {
                    let bounds: Vec<_> = room.entries.iter().map(|e| e.bounds).collect();
                    cullable_order(&bounds, clip_z, fog_far)
                }
                _ => (0..room.entries.len()).collect(),
            };
            drawn += order.len();
            for i in order {
                let e = &room.entries[i];
                opa.extend(e.opa.as_ref());
                xlu.extend(e.xlu.as_ref());
            }
        }
        self.drawn_entries = drawn;
        (opa, xlu)
    }
}
