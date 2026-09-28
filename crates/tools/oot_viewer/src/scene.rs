//! What the viewer can show (`Subject`), the synthetic test character "Tock", and shared
//! debug geometry.

use std::hash::{Hash, Hasher};
use std::sync::Arc;

use eframe::egui;
use eng_anim::anim::{Animation, JointTable, StandardAnimation};
use eng_anim::skeleton::Skeleton;
use eng_gbi::gbi::DrawList;
use eng_gbi::model::{Binding, BuildOptions, build_draw_list};
use eng_render::{Camera, LineVertex};
use glam::{Mat4, Vec3, Vec4Swizzles};
use oot_import::synth::{self, SynthObject};
use oot_import::z64::ParseStandardAnimation;

/// A skinned character the viewer can pose and draw.
pub trait Subject {
    fn title(&self) -> String;
    fn blurb(&self) -> &str;
    fn skeleton(&self) -> &Skeleton;
    fn anim_count(&self) -> usize;
    fn anim_name(&self, i: usize) -> &str;
    fn anim_frames(&self, i: usize) -> usize;
    fn default_anim(&self) -> usize;
    fn joints(&self, anim: usize, frame: f32, interpolate: bool) -> JointTable;
    fn pose(&self, joints: &JointTable) -> Vec<Mat4> {
        self.skeleton().pose(joints)
    }
    /// Advances time-driven state such as blinking.
    fn tick(&mut self, dt: f32);
    /// Hash of everything `draw_list` depends on; the app rebuilds the model when it changes.
    fn appearance(&self, joints: &JointTable) -> u64;
    fn draw_list(&self, joints: &JointTable) -> DrawList;
    fn appearance_ui(&mut self, ui: &mut egui::Ui);
    fn extra_stats(&self) -> Vec<(String, String)>;
    fn default_camera(&self) -> Camera;
    /// Headless contact sheets: adjust appearance for a cell (row animation, column).
    fn prepare_sheet_cell(&mut self, _anim: usize, _col: u32) {}
    /// True once after a change that needs the camera reframed (e.g. switching Link's age).
    fn wants_camera_reset(&mut self) -> bool {
        false
    }
}

pub fn hash_of(v: impl Hash) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}

pub const DEFAULT_ENV: [u8; 4] = [200, 70, 45, 255];

pub struct Tock {
    pub object: SynthObject,
    pub buf: Arc<[u8]>,
    pub skeleton: Skeleton,
    pub animations: Vec<(String, StandardAnimation)>,
    pub auto_blink: bool,
    pub face: usize,
    pub env: [u8; 4],
    clock: f32,
}

impl Tock {
    pub fn new() -> Tock {
        let object = synth::build();
        let buf: Arc<[u8]> = object.data.clone().into();
        let skeleton = object.skeleton();
        let animations = object
            .animations
            .iter()
            .map(|(name, off)| {
                let a = StandardAnimation::parse(&buf, synth::SEGMENT, *off, skeleton.limbs.len())
                    .expect("synthetic animation must parse");
                (name.clone(), a)
            })
            .collect();
        Tock { object, buf, skeleton, animations, auto_blink: true, face: 0, env: DEFAULT_ENV, clock: 0.0 }
    }

    fn current_face(&self) -> usize {
        if self.auto_blink { blink_face(self.clock) } else { self.face }
    }
}

impl Subject for Tock {
    fn title(&self) -> String {
        "Tock".into()
    }
    fn blurb(&self) -> &str {
        "Original test character encoded as N64 data and decoded by the same pipeline used for ROM files."
    }
    fn skeleton(&self) -> &Skeleton {
        &self.skeleton
    }
    fn anim_count(&self) -> usize {
        self.animations.len()
    }
    fn anim_name(&self, i: usize) -> &str {
        &self.animations[i].0
    }
    fn anim_frames(&self, i: usize) -> usize {
        self.animations[i].1.frame_count()
    }
    fn default_anim(&self) -> usize {
        self.animations.iter().position(|a| a.0 == "walk").unwrap_or(0)
    }
    fn joints(&self, anim: usize, frame: f32, interpolate: bool) -> JointTable {
        let a = &self.animations[anim].1;
        if interpolate { a.sample_smooth(frame, true) } else { a.sample(frame.floor().max(0.0) as usize % a.frame_count()) }
    }
    fn tick(&mut self, dt: f32) {
        self.clock += dt;
    }
    fn appearance(&self, _joints: &JointTable) -> u64 {
        hash_of((self.current_face(), self.env))
    }

    /// Runs every limb display list with the face texture segment and env colour bound.
    fn draw_list(&self, _joints: &JointTable) -> DrawList {
        let faces = &self.object.face_textures;
        let face_off = faces[self.current_face().min(faces.len() - 1)].1;
        let opts = BuildOptions {
            bindings: vec![
                Binding { segment: synth::SEGMENT, buf: self.buf.clone(), base: 0 },
                Binding { segment: synth::FACE_SEGMENT, buf: self.buf.clone(), base: face_off },
            ],
            env_color: Some(self.env),
            ..Default::default()
        };
        build_draw_list(&self.skeleton, &opts).expect("synthetic draw list")
    }

    fn appearance_ui(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(&mut self.auto_blink, "Auto blink (swaps segment 0x08)");
        if !self.auto_blink {
            ui.horizontal(|ui| {
                for (i, (name, _)) in self.object.face_textures.iter().enumerate() {
                    ui.radio_value(&mut self.face, i, name);
                }
            });
        }
        ui.horizontal(|ui| {
            ui.label("Emblem (env colour)");
            let mut c = egui::Color32::from_rgb(self.env[0], self.env[1], self.env[2]);
            if ui.color_edit_button_srgba(&mut c).changed() {
                self.env = [c.r(), c.g(), c.b(), 255];
            }
        });
    }

    fn extra_stats(&self) -> Vec<(String, String)> {
        vec![("object size".into(), format!("{} bytes", self.object.data.len()))]
    }

    fn default_camera(&self) -> Camera {
        Camera { target: Vec3::new(0.0, 2050.0, 0.0), yaw: 0.5, pitch: 0.15, distance: 7600.0, fov_y: 45f32.to_radians(), clip: None }
    }

    fn prepare_sheet_cell(&mut self, anim: usize, col: u32) {
        self.auto_blink = false;
        self.face = if col == 3 { 1 } else if self.animations[anim].0 == "wave" { 2 } else { 0 };
    }
}

/// Lines from each limb's origin to its parent's, drawn over the model.
pub fn bone_lines(skeleton: &Skeleton, mats: &[Mat4], root: Mat4) -> Vec<LineVertex> {
    let mut out = Vec::new();
    for (i, p) in skeleton.parents.iter().enumerate() {
        let Some(p) = p else { continue };
        let a = (root * mats[*p as usize]).w_axis.xyz();
        let b = (root * mats[i]).w_axis.xyz();
        out.push(LineVertex { pos: a.to_array(), color: [1.0, 0.9, 0.2, 0.9] });
        out.push(LineVertex { pos: b.to_array(), color: [1.0, 0.4, 0.1, 0.9] });
    }
    out
}

pub fn grid_lines(extent: f32, step: f32) -> Vec<LineVertex> {
    let mut out = Vec::new();
    let n = (extent / step) as i32;
    for i in -n..=n {
        let t = i as f32 * step;
        let c = if i == 0 { [0.55, 0.6, 0.7, 0.9] } else { [0.35, 0.38, 0.45, 0.6] };
        out.push(LineVertex { pos: Vec3::new(t, 0.0, -extent).to_array(), color: c });
        out.push(LineVertex { pos: Vec3::new(t, 0.0, extent).to_array(), color: c });
        out.push(LineVertex { pos: Vec3::new(-extent, 0.0, t).to_array(), color: c });
        out.push(LineVertex { pos: Vec3::new(extent, 0.0, t).to_array(), color: c });
    }
    out
}

/// Automatic blinking: eyes close for a few frames every couple of seconds.
pub fn blink_face(time: f32) -> usize {
    if (time % 2.7) > 2.55 { 1 } else { 0 }
}
