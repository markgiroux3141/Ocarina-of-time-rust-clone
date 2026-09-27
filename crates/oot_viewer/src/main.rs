//! Viewer for skinned characters rendered through the full pipeline:
//! N64 bytes -> skeleton/animation parsers -> F3DEX2 interpreter -> combiner shader.
//! Shows Link from the user's ROM, and the synthetic test character "Tock".

mod link;
mod scene;

use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use eframe::egui;
use glam::{Mat4, Vec3};
use oot_core::gbi::DrawList;
use oot_core::player::Age;
use oot_render::{Camera, GpuModel, Lighting, Renderer, Target};
use scene::{Subject, Tock, bone_lines, grid_lines};

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum SubjectArg {
    Link,
    Tock,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum AgeArg {
    Adult,
    Child,
}

#[derive(Parser)]
#[command(about = "Render skinned characters through the N64 decode pipeline")]
struct Cli {
    /// What to show. Link needs oot.toml pointing at your ROM and decomp.
    #[arg(long, value_enum, default_value = "link")]
    subject: SubjectArg,
    #[arg(long, value_enum, default_value = "adult")]
    age: AgeArg,
    /// Render one frame to this PNG instead of opening a window.
    #[arg(long)]
    screenshot: Option<PathBuf>,
    /// Render a contact sheet (rows = animations, columns = frames) to this PNG.
    #[arg(long)]
    sheet: Option<PathBuf>,
    /// Animation for --screenshot (and the initial one in the window).
    #[arg(long)]
    anim: Option<String>,
    /// Comma-separated animations for --sheet rows.
    #[arg(long, value_delimiter = ',')]
    anims: Vec<String>,
    #[arg(long, default_value_t = 0.0)]
    frame: f32,
    /// Camera yaw in degrees (0 = looking at the character's front).
    #[arg(long, default_value_t = 30.0)]
    yaw: f32,
    /// Link: model group (PLAYER_MODELGROUP_* without prefix, e.g. DEFAULT, SWORD, BOW_SLINGSHOT).
    #[arg(long)]
    group: Option<String>,
    /// Link: shield (NONE, DEKU, HYLIAN, MIRROR).
    #[arg(long)]
    shield: Option<String>,
    /// Link: tunic index (0 Kokiri, 1 Goron, 2 Zora).
    #[arg(long)]
    tunic: Option<usize>,
    #[arg(long, default_value_t = 960)]
    width: u32,
    #[arg(long, default_value_t = 720)]
    height: u32,
}

const CLEAR: [f64; 4] = [0.09, 0.10, 0.13, 1.0];
const LINK_SHEET_ANIMS: &[&str] =
    &["link_normal_wait", "link_normal_walk", "link_fighter_run", "link_fighter_normal_kiru", "link_normal_jump"];

fn load_link(cli: &Cli) -> Result<link::Link> {
    let age = if cli.age == AgeArg::Child { Age::Child } else { Age::Adult };
    let mut l = link::Link::load(age)?;
    if let Some(g) = &cli.group {
        l.loadout.model_group = l.rules().model_group(g).ok_or_else(|| anyhow::anyhow!("unknown model group {g}"))?;
    }
    if let Some(s) = &cli.shield {
        l.loadout.shield = l.rules().shields.iter().position(|x| x == s).ok_or_else(|| anyhow::anyhow!("unknown shield {s}"))?;
    }
    if let Some(t) = cli.tunic {
        l.loadout.tunic = t;
    }
    Ok(l)
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let cli = Cli::parse();
    if cli.screenshot.is_some() || cli.sheet.is_some() {
        let mut subject: Box<dyn Subject> = match cli.subject {
            SubjectArg::Link => Box::new(load_link(&cli)?),
            SubjectArg::Tock => Box::new(Tock::new()),
        };
        if let Some(path) = &cli.screenshot {
            return screenshot(&cli, subject.as_mut(), path);
        }
        if let Some(path) = &cli.sheet {
            return contact_sheet(&cli, subject.as_mut(), path);
        }
    }
    // Interactive: offer both subjects; Link only if the ROM loads.
    let mut subjects: Vec<Box<dyn Subject>> = Vec::new();
    let mut link_error = None;
    match load_link(&cli) {
        Ok(l) => subjects.push(Box::new(l)),
        Err(e) => link_error = Some(format!("{e:#}")),
    }
    subjects.push(Box::new(Tock::new()));
    let first = if cli.subject == SubjectArg::Tock { subjects.len() - 1 } else { 0 };
    let initial_anim = cli.anim.clone();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 800.0]).with_title("OoT pipeline viewer"),
        ..Default::default()
    };
    eframe::run_native(
        "oot_viewer",
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, subjects, first, initial_anim.as_deref(), link_error)?))),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}

fn anim_index(subject: &dyn Subject, name: &str) -> Result<usize> {
    let n = name.strip_prefix("gPlayerAnim_").unwrap_or(name);
    (0..subject.anim_count())
        .find(|&i| subject.anim_name(i) == n)
        .ok_or_else(|| anyhow::anyhow!("unknown animation {name}"))
}

// ---------------------------------------------------------------------------------------
// Headless rendering.

struct Headless {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: Renderer,
}

impl Headless {
    fn new() -> Result<Headless> {
        let (device, queue) = oot_render::headless_device()?;
        let renderer = Renderer::new(&device, &queue);
        Ok(Headless { device, queue, renderer })
    }

    /// Poses and draws one frame; returns RGBA pixels and the draw list used.
    fn render(&mut self, s: &dyn Subject, anim: usize, frame: f32, cam: &Camera, w: u32, h: u32) -> Result<(Vec<u8>, DrawList, usize)> {
        let joints = s.joints(anim, frame, true);
        let draw = s.draw_list(&joints);
        let mut model = self.renderer.upload(&self.device, &self.queue, &draw);
        model.pose(&s.pose(&joints), Mat4::IDENTITY);
        let target = Target::new(&self.device, w, h);
        let mut enc = self.device.create_command_encoder(&Default::default());
        let grid = grid_lines(3000.0, 250.0);
        self.renderer.render(&self.device, &self.queue, &mut enc, &target, &[&model], cam, &Lighting::default(), &grid, &[], CLEAR);
        self.queue.submit([enc.finish()]);
        Ok((target.read_rgba(&self.device, &self.queue)?, draw, model.draw_count()))
    }
}

fn screenshot(cli: &Cli, s: &mut dyn Subject, path: &PathBuf) -> Result<()> {
    let mut h = Headless::new()?;
    let anim = match &cli.anim {
        Some(a) => anim_index(s, a)?,
        None => s.default_anim(),
    };
    let mut cam = s.default_camera();
    cam.yaw = cli.yaw.to_radians();
    let (px, draw, draws) = h.render(s, anim, cli.frame, &cam, cli.width, cli.height)?;
    save_png(path, cli.width, cli.height, &px)?;
    println!("{} ({} {}, {} tris, {} draws)", path.display(), s.title(), s.anim_name(anim), draw.triangle_count(), draws);
    Ok(())
}

fn contact_sheet(cli: &Cli, s: &mut dyn Subject, path: &PathBuf) -> Result<()> {
    let mut h = Headless::new()?;
    let (tw, th, cols) = (320u32, 360u32, 8u32);
    let rows: Vec<usize> = if !cli.anims.is_empty() {
        cli.anims.iter().map(|a| anim_index(s, a)).collect::<Result<_>>()?
    } else if cli.subject == SubjectArg::Link {
        LINK_SHEET_ANIMS.iter().filter_map(|a| anim_index(s, a).ok()).collect()
    } else {
        (0..s.anim_count()).filter(|&i| s.anim_frames(i) > 1).collect()
    };
    let (sw, sh) = (tw * cols, th * rows.len() as u32);
    let mut sheet = vec![0u8; (sw * sh * 4) as usize];
    let mut cam = s.default_camera();
    cam.yaw = cli.yaw.to_radians();
    for (r, &anim) in rows.iter().enumerate() {
        let n = s.anim_frames(anim) as f32;
        for c in 0..cols {
            s.prepare_sheet_cell(anim, c);
            let frame = n * c as f32 / cols as f32;
            let (px, _, _) = h.render(s, anim, frame, &cam, tw, th)?;
            for y in 0..th {
                let dst = (((r as u32 * th + y) * sw + c * tw) * 4) as usize;
                let src = (y * tw * 4) as usize;
                sheet[dst..dst + (tw * 4) as usize].copy_from_slice(&px[src..src + (tw * 4) as usize]);
            }
        }
    }
    save_png(path, sw, sh, &sheet)?;
    let names: Vec<&str> = rows.iter().map(|&i| s.anim_name(i)).collect();
    println!("{} ({}) rows: {names:?}, {cols} frames each", path.display(), s.title());
    Ok(())
}

fn save_png(path: &PathBuf, w: u32, h: u32, px: &[u8]) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    image::save_buffer(path, px, w, h, image::ColorType::Rgba8)?;
    Ok(())
}

// ---------------------------------------------------------------------------------------
// Interactive app.

struct App {
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_state: egui_wgpu::RenderState,
    renderer: Renderer,
    subjects: Vec<Box<dyn Subject>>,
    current: usize,
    link_error: Option<String>,
    draw: DrawList,
    model: GpuModel,
    built_for: Option<(usize, u64)>,
    target: Option<(Target, egui::TextureId)>,
    camera: Camera,
    anim: usize,
    anim_filter: String,
    frame: f32,
    playing: bool,
    speed: f32,
    interpolate: bool,
    show_bones: bool,
    show_grid: bool,
    last: Instant,
}

impl App {
    fn new(
        cc: &eframe::CreationContext,
        subjects: Vec<Box<dyn Subject>>,
        current: usize,
        initial_anim: Option<&str>,
        link_error: Option<String>,
    ) -> Result<App> {
        let rs = cc.wgpu_render_state.clone().ok_or_else(|| anyhow::anyhow!("wgpu backend required"))?;
        let (device, queue) = (rs.device.clone(), rs.queue.clone());
        let mut renderer = Renderer::new(&device, &queue);
        let s = &subjects[current];
        let anim = initial_anim.and_then(|a| anim_index(s.as_ref(), a).ok()).unwrap_or(s.default_anim());
        let draw = s.draw_list(&s.joints(anim, 0.0, false));
        let model = renderer.upload(&device, &queue, &draw);
        let camera = s.default_camera();
        Ok(App {
            device,
            queue,
            render_state: rs,
            renderer,
            subjects,
            current,
            link_error,
            draw,
            model,
            built_for: None,
            target: None,
            camera,
            anim,
            anim_filter: String::new(),
            frame: 0.0,
            playing: true,
            speed: 1.0,
            interpolate: true,
            show_bones: false,
            show_grid: true,
            last: Instant::now(),
        })
    }

    fn subject(&self) -> &dyn Subject {
        self.subjects[self.current].as_ref()
    }

    fn switch_to(&mut self, i: usize) {
        if i != self.current {
            self.current = i;
            self.anim = self.subject().default_anim();
            self.frame = 0.0;
            self.built_for = None;
            self.camera = self.subject().default_camera();
            self.anim_filter.clear();
        }
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        if self.subjects.len() > 1 {
            ui.horizontal(|ui| {
                for i in 0..self.subjects.len() {
                    let name = if i == 0 && self.link_error.is_none() { "Link".to_string() } else { self.subjects[i].title() };
                    if ui.selectable_label(self.current == i, name).clicked() {
                        self.switch_to(i);
                    }
                }
            });
        }
        if let Some(e) = &self.link_error {
            ui.colored_label(egui::Color32::from_rgb(230, 120, 90), format!("Link unavailable: {e}"));
        }
        ui.heading(self.subject().title());
        ui.label(self.subject().blurb());
        ui.separator();

        ui.strong("Animation");
        let count = self.subject().anim_count();
        if count > 12 {
            ui.horizontal(|ui| {
                ui.label("Filter");
                ui.text_edit_singleline(&mut self.anim_filter);
            });
        }
        let filter = self.anim_filter.to_lowercase();
        egui::ScrollArea::vertical().id_salt("anims").max_height(260.0).show(ui, |ui| {
            for i in 0..count {
                let name = self.subjects[self.current].anim_name(i);
                if !filter.is_empty() && !name.to_lowercase().contains(&filter) {
                    continue;
                }
                let label = format!("{name} ({})", self.subjects[self.current].anim_frames(i));
                if ui.selectable_label(self.anim == i, label).clicked() {
                    self.anim = i;
                    self.frame = 0.0;
                }
            }
        });
        let n = self.subject().anim_frames(self.anim) as f32;
        ui.horizontal(|ui| {
            if ui.button(if self.playing { "Pause" } else { "Play" }).clicked() {
                self.playing = !self.playing;
            }
            if ui.button("<").clicked() {
                self.playing = false;
                self.frame = (self.frame.floor() - 1.0).rem_euclid(n);
            }
            if ui.button(">").clicked() {
                self.playing = false;
                self.frame = (self.frame.floor() + 1.0).rem_euclid(n);
            }
        });
        ui.add(egui::Slider::new(&mut self.frame, 0.0..=(n - 0.001).max(0.0)).text("frame"));
        ui.add(egui::Slider::new(&mut self.speed, 0.05..=3.0).logarithmic(true).text("speed"));
        ui.checkbox(&mut self.interpolate, "Interpolate between frames");
        ui.separator();

        ui.strong("Appearance");
        self.subjects[self.current].appearance_ui(ui);
        ui.checkbox(&mut self.show_bones, "Show skeleton");
        ui.checkbox(&mut self.show_grid, "Show grid");
        if ui.button("Reset camera").clicked() {
            self.camera = self.subject().default_camera();
        }
        ui.separator();

        ui.strong("Decoded data");
        let s = &self.draw.stats;
        let skel = self.subject().skeleton();
        let mut rows: Vec<(String, String)> = vec![
            ("limbs".into(), skel.limbs.len().to_string()),
            ("flex matrices".into(), skel.flex_matrix_map(0).len().to_string()),
            ("triangles".into(), self.draw.triangle_count().to_string()),
            ("batches".into(), self.draw.batches.len().to_string()),
            ("materials".into(), self.draw.materials.len().to_string()),
            ("textures".into(), self.draw.textures.len().to_string()),
            ("GBI commands".into(), s.commands.to_string()),
            ("matrix loads".into(), s.matrix_loads.to_string()),
            ("unknown opcodes".into(), s.unknown_opcodes.len().to_string()),
            ("unresolved refs".into(), s.unresolved_addresses.len().to_string()),
        ];
        rows.extend(self.subject().extra_stats());
        egui::Grid::new("stats").striped(true).show(ui, |ui| {
            for (k, v) in rows {
                ui.label(k);
                ui.label(v);
                ui.end_row();
            }
        });
        egui::CollapsingHeader::new("Materials (combiner)").show(ui, |ui| {
            for (i, m) in self.draw.materials.iter().enumerate() {
                ui.label(format!(
                    "#{i} {:?} {:?} lit={} 2cyc={}\n{}",
                    m.blend,
                    m.cull,
                    m.lit,
                    m.two_cycle,
                    m.combiner.describe(m.two_cycle)
                ));
            }
        });
        egui::CollapsingHeader::new("Textures").show(ui, |ui| {
            for t in &self.draw.textures {
                ui.label(format!("{} {}x{}", oot_core::texture::format_name(t.fmt, t.siz), t.image.width, t.image.height));
            }
        });
        ui.separator();
        ui.small("Drag: orbit · Right-drag: pan · Scroll: zoom");
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f32().min(0.1);
        self.last = now;
        let n = self.subject().anim_frames(self.anim) as f32;
        if self.playing {
            // OoT animations advance one frame per game frame at 20 fps.
            self.frame = (self.frame + dt * 20.0 * self.speed).rem_euclid(n);
        }
        self.subjects[self.current].tick(dt);

        egui::Panel::left("side").resizable(true).default_size(340.0).show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| self.sidebar(ui));
        });
        if self.subjects[self.current].wants_camera_reset() {
            self.camera = self.subject().default_camera();
        }

        let joints = self.subject().joints(self.anim, self.frame, self.interpolate);
        let key = (self.current, self.subject().appearance(&joints));
        if self.built_for != Some(key) {
            self.draw = self.subject().draw_list(&joints);
            self.model = self.renderer.upload(&self.device, &self.queue, &self.draw);
            self.built_for = Some(key);
        }

        egui::CentralPanel::default().show(ui, |ui| {
            let avail = ui.available_size();
            let ppp = ui.ctx().pixels_per_point();
            let (w, h) = ((avail.x * ppp) as u32, (avail.y * ppp) as u32);
            if self.target.as_ref().is_none_or(|(t, _)| t.size != (w.max(1), h.max(1))) {
                let t = Target::new(&self.device, w, h);
                let mut egui_renderer = self.render_state.renderer.write();
                let id = match self.target.take() {
                    Some((_, id)) => {
                        egui_renderer.update_egui_texture_from_wgpu_texture(&self.device, &t.resolve_view, wgpu::FilterMode::Linear, id);
                        id
                    }
                    None => egui_renderer.register_native_texture(&self.device, &t.resolve_view, wgpu::FilterMode::Linear),
                };
                self.target = Some((t, id));
            }
            let (target, id) = self.target.as_ref().unwrap();

            let mats = self.subject().pose(&joints);
            self.model.pose(&mats, Mat4::IDENTITY);
            let grid = if self.show_grid { grid_lines(3000.0, 250.0) } else { Vec::new() };
            let bones = if self.show_bones { bone_lines(self.subject().skeleton(), &mats, Mat4::IDENTITY) } else { Vec::new() };
            let mut enc = self.device.create_command_encoder(&Default::default());
            self.renderer.render(&self.device, &self.queue, &mut enc, target, &[&self.model], &self.camera, &Lighting::default(), &grid, &bones, CLEAR);
            self.queue.submit([enc.finish()]);

            let resp = ui.add(egui::Image::new((*id, avail)).sense(egui::Sense::click_and_drag()));
            if resp.dragged_by(egui::PointerButton::Primary) {
                let d = resp.drag_delta();
                self.camera.yaw -= d.x * 0.01;
                self.camera.pitch = (self.camera.pitch + d.y * 0.01).clamp(-1.4, 1.4);
            }
            if resp.dragged_by(egui::PointerButton::Secondary) {
                let d = resp.drag_delta();
                let right = Vec3::new(self.camera.yaw.cos(), 0.0, -self.camera.yaw.sin());
                let s = self.camera.distance * 0.0015;
                self.camera.target += -right * d.x * s + Vec3::Y * d.y * s;
            }
            if resp.hovered() {
                let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                self.camera.distance = (self.camera.distance * (1.0 - scroll * 0.002)).clamp(800.0, 40000.0);
            }
        });
        ui.ctx().request_repaint();
    }
}
