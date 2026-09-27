//! Link from the user's ROM: object_link_boy / object_link_child, every Player animation
//! from gameplay_keep + link_animetion, and Player's draw rules from z_player_lib.c.

use std::cell::RefCell;

use anyhow::Result;
use eframe::egui;
use glam::{Mat4, Vec3};
use oot_core::anim::{Animation, JointTable, LinkAnimation};
use oot_core::gbi::{DrawList, posed_bounds};
use oot_core::player::{Age, Blinker, Loadout, PlayerModel, PlayerRules, player_animations};
use oot_core::project::Project;
use oot_core::skeleton::Skeleton;
use oot_render::Camera;

use crate::scene::{Subject, hash_of};

const GAME_FRAME: f32 = 1.0 / 20.0;
const EYE_NAMES: [&str; 8] = ["open", "half", "closed", "roll left", "roll right", "shock", "unk 1", "unk 2"];

pub struct Link {
    rules: PlayerRules,
    models: [PlayerModel; 2],
    animations: Vec<(String, LinkAnimation)>,
    pub loadout: Loadout,
    blinker: Blinker,
    blink_face: usize,
    blink_acc: f32,
    pub auto_blink: bool,
    /// Manual eye/mouth texture indices; None follows the animation and blink.
    pub eye_override: Option<usize>,
    pub mouth_override: Option<usize>,
    pub lod: usize,
    camera_reset: bool,
    missing: RefCell<Vec<String>>,
}

impl Link {
    pub fn load(age: Age) -> Result<Link> {
        let project = Project::open_default()?;
        let rules = PlayerRules::load(&project.config.decomp)?;
        let models = [PlayerModel::load(&project, &rules, Age::Adult)?, PlayerModel::load(&project, &rules, Age::Child)?];
        let animations = player_animations(&project)?
            .into_iter()
            .map(|(n, a)| (n.strip_prefix("gPlayerAnim_").unwrap_or(&n).to_string(), a))
            .collect();
        let loadout = Loadout::default_for(&rules, age);
        Ok(Link {
            rules,
            models,
            animations,
            loadout,
            blinker: Blinker::default(),
            blink_face: 0,
            blink_acc: 0.0,
            auto_blink: true,
            eye_override: None,
            mouth_override: None,
            lod: 0,
            camera_reset: false,
            missing: RefCell::new(Vec::new()),
        })
    }

    pub fn rules(&self) -> &PlayerRules {
        &self.rules
    }

    fn model(&self) -> &PlayerModel {
        &self.models[self.loadout.age as usize]
    }

    pub fn find_anim(&self, name: &str) -> Option<usize> {
        let n = name.strip_prefix("gPlayerAnim_").unwrap_or(name);
        self.animations.iter().position(|a| a.0 == n)
    }

    pub fn set_age(&mut self, age: Age) {
        if age != self.loadout.age {
            let tunic = self.loadout.tunic;
            let group = self.loadout.model_group;
            self.loadout = Loadout { tunic, model_group: group, ..Loadout::default_for(&self.rules, age) };
            self.camera_reset = true;
        }
    }

    fn face(&self, joints: &JointTable) -> (usize, usize) {
        let blink = if self.auto_blink { self.blink_face } else { 0 };
        let (e, m) = self.rules.face_indices(joints.face, blink);
        (self.eye_override.unwrap_or(e), self.mouth_override.unwrap_or(m))
    }
}

fn combo(ui: &mut egui::Ui, label: &str, value: &mut usize, items: &[String]) {
    egui::ComboBox::from_label(label)
        .selected_text(items.get(*value).cloned().unwrap_or_default())
        .show_ui(ui, |ui| {
            for (i, it) in items.iter().enumerate() {
                ui.selectable_value(value, i, it);
            }
        });
}

impl Subject for Link {
    fn title(&self) -> String {
        match self.loadout.age {
            Age::Adult => "Link (adult)".into(),
            Age::Child => "Link (child)".into(),
        }
    }
    fn blurb(&self) -> &str {
        "Decoded from your ROM. Limb display lists, eye/mouth segments and tunic colour follow Player's draw rules read from z_player_lib.c."
    }
    fn skeleton(&self) -> &Skeleton {
        &self.model().skeleton
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
        self.find_anim("link_normal_wait").unwrap_or(0)
    }
    fn joints(&self, anim: usize, frame: f32, interpolate: bool) -> JointTable {
        let a = &self.animations[anim].1;
        if interpolate { a.sample_smooth(frame, true) } else { a.sample(frame.floor().max(0.0) as usize % a.frame_count()) }
    }
    fn pose(&self, joints: &JointTable) -> Vec<Mat4> {
        let s = self.loadout.age.root_scale();
        let mut jt = joints.clone();
        if let Some(r) = jt.rot.first_mut() {
            *r = r.map(|c| (c as f32 * s) as i16);
        }
        self.skeleton().pose(&jt)
    }
    fn tick(&mut self, dt: f32) {
        self.blink_acc += dt;
        while self.blink_acc >= GAME_FRAME {
            self.blink_acc -= GAME_FRAME;
            self.blink_face = self.blinker.step();
        }
    }
    fn appearance(&self, joints: &JointTable) -> u64 {
        let l = &self.loadout;
        hash_of((l.age as u8, l.model_group, l.shield, l.tunic, l.child_has_kokiri_sword, l.moving_fast, self.face(joints), self.lod))
    }
    fn draw_list(&self, joints: &JointTable) -> DrawList {
        let (eye, mouth) = self.face(joints);
        match self.model().draw_list(&self.rules, &self.loadout, eye, mouth, self.lod) {
            Ok((d, missing)) => {
                *self.missing.borrow_mut() = missing;
                d
            }
            Err(e) => {
                log::error!("building Link draw list: {e:#}");
                DrawList::default()
            }
        }
    }

    fn appearance_ui(&mut self, ui: &mut egui::Ui) {
        let mut age = self.loadout.age;
        ui.horizontal(|ui| {
            ui.label("Age");
            ui.radio_value(&mut age, Age::Adult, "Adult");
            ui.radio_value(&mut age, Age::Child, "Child");
        });
        self.set_age(age);
        let groups: Vec<String> = self.rules.model_groups.iter().map(|g| g.name.clone()).collect();
        let tunics: Vec<String> = self.rules.tunics.iter().map(|t| t.0.clone()).collect();
        let shields = self.rules.shields.clone();
        combo(ui, "Model group", &mut self.loadout.model_group, &groups);
        combo(ui, "Shield", &mut self.loadout.shield, &shields);
        combo(ui, "Tunic (env colour)", &mut self.loadout.tunic, &tunics);
        ui.checkbox(&mut self.loadout.moving_fast, "Running (speedXZ > 2: open hands become fists)");
        if self.loadout.age == Age::Child {
            ui.checkbox(&mut self.loadout.child_has_kokiri_sword, "Kokiri Sword on B");
        }
        ui.horizontal(|ui| {
            ui.label("LOD");
            ui.radio_value(&mut self.lod, 0, "near");
            ui.radio_value(&mut self.lod, 1, "far");
        });
        ui.checkbox(&mut self.auto_blink, "Auto blink (Player's blink timer)");
        let eyes: Vec<String> =
            std::iter::once("from animation".to_string()).chain(EYE_NAMES.iter().map(|s| s.to_string())).collect();
        let mouths: Vec<String> = std::iter::once("from animation".to_string()).chain((1..=4).map(|i| format!("mouth {i}"))).collect();
        let mut e = self.eye_override.map_or(0, |v| v + 1);
        let mut m = self.mouth_override.map_or(0, |v| v + 1);
        combo(ui, "Eyes (segment 0x08)", &mut e, &eyes);
        combo(ui, "Mouth (segment 0x09)", &mut m, &mouths);
        self.eye_override = e.checked_sub(1);
        self.mouth_override = m.checked_sub(1);
        ui.collapsing("Limb display lists", |ui| {
            for (limb, name) in self.rules.limb_dlists(&self.loadout, self.lod) {
                ui.small(format!("limb {limb}: {}", name.as_deref().unwrap_or("NULL")));
            }
        });
    }

    fn extra_stats(&self) -> Vec<(String, String)> {
        let mut v = vec![
            ("object".into(), format!("{} ({} bytes)", self.loadout.age.object(), self.model().object.len())),
            ("player animations".into(), self.animations.len().to_string()),
        ];
        let missing = self.missing.borrow();
        if !missing.is_empty() {
            v.push(("missing DLs".into(), missing.join(", ")));
        }
        v
    }

    fn wants_camera_reset(&mut self) -> bool {
        std::mem::take(&mut self.camera_reset)
    }

    fn default_camera(&self) -> Camera {
        // Frame the standing pose.
        let joints = self.joints(self.default_anim(), 0.0, false);
        let draw = self.draw_list(&joints);
        let (lo, hi) = posed_bounds(&draw, &self.pose(&joints));
        let h = (hi.y - lo.y).max(1000.0);
        Camera {
            target: Vec3::new(0.0, (lo.y + hi.y) * 0.5, 0.0),
            yaw: 0.5,
            pitch: 0.15,
            distance: h * 1.9,
            fov_y: 45f32.to_radians(),
        }
    }
}
