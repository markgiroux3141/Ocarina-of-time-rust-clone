//! Spike 03 play mode: Link from the ROM, driven by Player movement ported from the decomp,
//! on the synthetic test course or a real scene's collision, with an N64 pad or keyboard.
//!
//! Headless modes (`--sheet`, `--trace`) run a scripted input and write images / JSON to
//! the git-ignored `out/` folder.

mod gfx;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use eframe::egui;
use glam::{Mat4, Vec3};
use oot_core::player::{Age, PlayerModel, PlayerRules};
use oot_core::project::Project;
use oot_core::scene::Scene;
use oot_game::bgcheck::StaticCollision;
use oot_game::course;
use oot_game::data::GameData;
use oot_game::input::{BTN_A, PadState};
use oot_game::world::{Snapshot, World, scripted_input};
use oot_pad::{Keyboard, PadConfig, Pads, merge};
use oot_render::{Camera, GpuModel, Lighting, LineVertex, Renderer, Target};

#[derive(Parser)]
#[command(about = "Link moving through a test course with Player logic ported from the decomp")]
struct Cli {
    /// Load this scene's collision from the ROM instead of the test course (e.g. spot04 =
    /// Kokiri Forest, spot00 = Hyrule Field, ydan = Deku Tree). Link starts at spawn 0.
    #[arg(long)]
    scene: Option<String>,
    /// Start as child Link.
    #[arg(long)]
    child: bool,
    /// Headless: run a built-in script and write a contact sheet PNG.
    #[arg(long)]
    sheet: Option<PathBuf>,
    /// Headless: run a built-in script and write a per-frame JSON trace.
    #[arg(long)]
    trace: Option<PathBuf>,
    /// Script for --sheet / --trace: run-roll, ledge, pit, stairs, walls, turn.
    #[arg(long, default_value = "run-roll")]
    script: String,
    /// Headless: one screenshot after the script, from the chase camera.
    #[arg(long)]
    screenshot: Option<PathBuf>,
    /// Headless: draw the collision wireframe over the mesh.
    #[arg(long)]
    wire: bool,
    #[arg(long, default_value_t = 1280)]
    width: u32,
    #[arg(long, default_value_t = 720)]
    height: u32,
}

const CLEAR: [f64; 4] = [0.55, 0.68, 0.82, 1.0];

/// Everything loaded from the ROM and decomp.
struct Assets {
    data: Arc<GameData>,
    link: gfx::LinkGfx,
    collision: oot_core::collision::CollisionHeader,
    spawn: (Vec3, i16),
    place: String,
    marks: Vec<(&'static str, Vec3)>,
}

fn load_assets(cli: &Cli) -> Result<Assets> {
    let project = Project::open_default()?;
    let data = Arc::new(GameData::load(&project).context("loading Player data")?);
    let rules = PlayerRules::load(&project.config.decomp)?;
    let adult = PlayerModel::load(&project, &rules, Age::Adult)?;
    let child = PlayerModel::load(&project, &rules, Age::Child)?;
    let link = gfx::LinkGfx::new(rules, adult, child);
    let (collision, spawn, place, marks) = match &cli.scene {
        Some(name) => {
            let s = Scene::load(&project.rom, name)?;
            let sp = s.spawns.first().copied();
            let (pos, yaw) = sp
                .map(|e| (Vec3::new(e.pos[0] as f32, e.pos[1] as f32, e.pos[2] as f32), e.rot[1]))
                .unwrap_or((Vec3::ZERO, 0));
            println!(
                "{}: {} vertices, {} polys, {} surface types, {} spawns; starting at {pos} yaw {yaw:#x}",
                s.name,
                s.collision.vertices.len(),
                s.collision.polys.len(),
                s.collision.surface_types.len(),
                s.spawns.len()
            );
            (s.collision, (pos, yaw), s.name, Vec::new())
        }
        None => {
            let c = course::build();
            (c.collision, (c.spawn, c.spawn_yaw), "test course".to_string(), c.marks)
        }
    };
    Ok(Assets { data, link, collision, spawn, place, marks })
}

fn new_world(a: &Assets, child: bool) -> World {
    World::new(a.data.clone(), StaticCollision::new(a.collision.clone()), !child, a.spawn.0, a.spawn.1)
}

/// The render camera for a snapshot, pulled in front of any wall between Link and the eye
/// (a simple stand-in; `z_camera.c` is not ported).
fn camera_of(s: &Snapshot, col: &StaticCollision) -> Camera {
    let mut distance = s.camera.distance;
    let eye = s.camera.eye();
    if let Some((hit, _)) = col.check_line(0, 0, s.camera.at, eye, 1.0, oot_game::bgcheck::CHECK_WALL | oot_game::bgcheck::CHECK_CEILING | oot_game::bgcheck::CHECK_FLOOR) {
        distance = ((hit - s.camera.at).length() - 12.0).max(30.0);
    }
    Camera { target: s.camera.at, yaw: s.camera.yaw, pitch: s.camera.pitch, distance, fov_y: 50f32.to_radians() }
}

/// Uploads the static scene (collision mesh) and the shadow.
struct SceneGfx {
    course: GpuModel,
    shadow: GpuModel,
    wire: Vec<LineVertex>,
}

impl SceneGfx {
    fn new(r: &mut Renderer, device: &wgpu::Device, queue: &wgpu::Queue, col: &oot_core::collision::CollisionHeader) -> SceneGfx {
        let mut course = r.upload(device, queue, &gfx::collision_draw_list(col));
        course.pose(&[], Mat4::IDENTITY);
        let shadow = r.upload(device, queue, &gfx::shadow_draw_list());
        SceneGfx { course, shadow, wire: gfx::collision_lines(col, 0.5) }
    }
}

/// Poses Link and the shadow for a snapshot and records the frame.
#[allow(clippy::too_many_arguments)]
fn draw_frame(
    r: &mut Renderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    target: &Target,
    assets: &mut Assets,
    scene: &mut SceneGfx,
    world: &World,
    snap: &Snapshot,
    show_wire: bool,
) {
    let age = if world.player.adult { Age::Adult } else { Age::Child };
    let (head, upper) = (assets.data.limb("HEAD"), assets.data.limb("UPPER"));
    let skel = assets.link.model(age).skeleton.clone();
    let mats = gfx::pose_player(&skel, &snap.joints, &snap.look, head, upper);
    let root = gfx::actor_matrix(snap.pos, snap.facing, 0.01);
    // Blob shadow on the floor below Player.
    let (floor, _) = world.col.entity_raycast_down(snap.pos + Vec3::Y * 20.0);
    let drop = (snap.pos.y - floor).max(0.0);
    let size = if world.player.adult { 22.0 } else { 16.0 } * (1.0 - (drop / 400.0).min(0.7));
    scene.shadow.pose(&[], Mat4::from_translation(Vec3::new(snap.pos.x, floor + 0.3, snap.pos.z)) * Mat4::from_scale(Vec3::new(size, 1.0, size)));
    let fists = snap.speed_xz > 2.0;
    let link = assets.link.get(r, device, queue, age, snap.joints.face, snap.face, fists);
    link.pose(&mats, root);
    let link: &GpuModel = link;
    let cam = camera_of(snap, &world.col);
    let lines = if show_wire { scene.wire.clone() } else { Vec::new() };
    let mut enc = device.create_command_encoder(&Default::default());
    r.render(device, queue, &mut enc, target, &[&scene.course, &scene.shadow, link], &cam, &Lighting::default(), &lines, &[], CLEAR);
    queue.submit([enc.finish()]);
}

// ---------------------------------------------------------------------------------------
// Scripts for the headless modes.

fn stick(x: i8, y: i8) -> PadState {
    PadState { button: 0, stick_x: x, stick_y: y }
}

fn script(name: &str) -> Result<(Vec<PadState>, Option<(Vec3, i16)>)> {
    let rep = |p: PadState, n: usize| vec![p; n];
    let mut s = Vec::new();
    let start;
    match name {
        // The brief's example: stand, full stick forward for 40 frames, then A.
        "run-roll" => {
            s.extend(rep(stick(0, 0), 6));
            s.extend(rep(stick(0, 80), 40));
            s.push(PadState { button: BTN_A, stick_x: 0, stick_y: 80 });
            s.extend(rep(stick(0, 80), 14));
            s.extend(rep(stick(0, 0), 20));
            start = Some((Vec3::new(-300.0, 0.0, 800.0), -0x8000));
        }
        "ledge" => {
            s.extend(rep(stick(0, 0), 4));
            s.extend(rep(stick(0, 80), 26));
            s.extend(rep(stick(0, 0), 30));
            start = Some((Vec3::new(-700.0, 150.0, -700.0), 0x4000));
        }
        "pit" => {
            s.extend(rep(stick(0, 80), 14));
            s.extend(rep(stick(0, 0), 46));
            start = Some((Vec3::new(600.0, 0.0, -300.0), -0x8000));
        }
        "stairs" => {
            s.extend(rep(stick(0, 80), 50));
            start = Some((Vec3::new(150.0, 0.0, -300.0), -0x8000));
        }
        "walls" => {
            s.extend(rep(stick(0, 80), 60));
            start = Some((Vec3::new(-300.0, 0.0, 600.0), -0x8000));
        }
        "turn" => {
            s.extend(rep(stick(-26, 0), 12));
            s.extend(rep(stick(80, 0), 12));
            s.extend(rep(stick(0, -80), 16));
            start = Some((Vec3::new(-300.0, 0.0, 900.0), -0x8000));
        }
        "idle" => {
            s.extend(rep(stick(0, 0), 120));
            start = None;
        }
        other => anyhow::bail!("unknown script {other}"),
    }
    Ok((s, start))
}

fn run_script(a: &Assets, cli: &Cli) -> Result<(World, Vec<Snapshot>, Vec<serde_json::Value>)> {
    let (s, start) = script(&cli.script)?;
    let mut w = new_world(a, cli.child);
    if let (Some((p, y)), None) = (start, &cli.scene) {
        w = World::new(a.data.clone(), StaticCollision::new(a.collision.clone()), !cli.child, p, y);
    }
    let mut snaps = Vec::new();
    let mut trace = Vec::new();
    let mut prev = PadState::default();
    for (i, cur) in s.iter().enumerate() {
        w.tick_with(scripted_input(prev, *cur));
        prev = *cur;
        let p = &w.player;
        trace.push(serde_json::json!({
            "frame": i + 1,
            "input": { "x": cur.stick_x, "y": cur.stick_y, "buttons": cur.names() },
            "action": format!("{:?}", p.action),
            "decomp_action": p.action.decomp_name(),
            "pos": [p.actor.world_pos.x, p.actor.world_pos.y, p.actor.world_pos.z],
            "linear_velocity": p.linear_velocity,
            "velocity_y": p.actor.velocity.y,
            "current_yaw": p.current_yaw,
            "shape_yaw": p.actor.shape_rot.y,
            "grounded": p.grounded(),
            "anim": w.data.anim_name(p.skel.animation),
            "anim_frame": p.skel.cur_frame,
            "walk_phase": p.unk_868,
        }));
        snaps.push(w.current_snapshot());
    }
    Ok((w, snaps, trace))
}

fn save_png(path: &PathBuf, w: u32, h: u32, px: &[u8]) -> Result<()> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    image::save_buffer(path, px, w, h, image::ColorType::Rgba8)?;
    Ok(())
}

fn headless(cli: &Cli) -> Result<()> {
    let mut a = load_assets(cli)?;
    let (w, snaps, trace) = run_script(&a, cli)?;
    if let Some(p) = &cli.trace {
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(p, serde_json::to_string_pretty(&serde_json::json!({ "script": cli.script, "place": a.place, "frames": trace }))?)?;
        println!("{} ({} frames)", p.display(), trace.len());
    }
    let (device, queue) = oot_render::headless_device()?;
    let mut r = Renderer::new(&device, &queue);
    let mut scene = SceneGfx::new(&mut r, &device, &queue, &a.collision);
    if let Some(p) = &cli.screenshot {
        let t = Target::new(&device, cli.width, cli.height);
        draw_frame(&mut r, &device, &queue, &t, &mut a, &mut scene, &w, snaps.last().unwrap(), cli.wire);
        save_png(p, cli.width, cli.height, &t.read_rgba(&device, &queue)?)?;
        println!("{}", p.display());
    }
    if let Some(p) = &cli.sheet {
        // Every other frame, 6 columns.
        let picks: Vec<usize> = (0..snaps.len()).step_by(2).collect();
        let (tw, th, cols) = (360u32, 270u32, 6u32);
        let rows = (picks.len() as u32).div_ceil(cols);
        let (sw, sh) = (tw * cols, th * rows);
        let mut sheet = vec![0u8; (sw * sh * 4) as usize];
        let t = Target::new(&device, tw, th);
        for (k, &i) in picks.iter().enumerate() {
            draw_frame(&mut r, &device, &queue, &t, &mut a, &mut scene, &w, &snaps[i], cli.wire);
            let px = t.read_rgba(&device, &queue)?;
            let (cx, cy) = (k as u32 % cols, k as u32 / cols);
            for y in 0..th {
                let dst = (((cy * th + y) * sw + cx * tw) * 4) as usize;
                let src = (y * tw * 4) as usize;
                sheet[dst..dst + (tw * 4) as usize].copy_from_slice(&px[src..src + (tw * 4) as usize]);
            }
        }
        save_png(p, sw, sh, &sheet)?;
        let labels: Vec<String> = picks.iter().map(|&i| format!("{}:{}", i + 1, trace[i]["action"].as_str().unwrap_or(""))).collect();
        println!("{} ({} cells, left to right): {}", p.display(), picks.len(), labels.join(" "));
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------
// Interactive app.

struct App {
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_state: egui_wgpu::RenderState,
    renderer: Renderer,
    assets: Assets,
    scene: SceneGfx,
    world: World,
    pads: Option<Pads>,
    pad_error: Option<String>,
    target: Option<(Target, egui::TextureId)>,
    last: Instant,
    show_wire: bool,
    show_hud: bool,
    fps: f32,
    last_pad: PadState,
}

impl App {
    fn new(cc: &eframe::CreationContext, assets: Assets, child: bool) -> Result<App> {
        let rs = cc.wgpu_render_state.clone().ok_or_else(|| anyhow::anyhow!("wgpu backend required"))?;
        let (device, queue) = (rs.device.clone(), rs.queue.clone());
        let mut renderer = Renderer::new(&device, &queue);
        let scene = SceneGfx::new(&mut renderer, &device, &queue, &assets.collision);
        let world = new_world(&assets, child);
        let (pads, pad_error) = match Pads::new(PadConfig::find(&std::env::current_dir()?)) {
            Ok(p) => (Some(p), None),
            Err(e) => (None, Some(format!("{e:#}"))),
        };
        Ok(App {
            device,
            queue,
            render_state: rs,
            renderer,
            assets,
            scene,
            world,
            pads,
            pad_error,
            target: None,
            last: Instant::now(),
            show_wire: false,
            show_hud: true,
            fps: 0.0,
            last_pad: PadState::default(),
        })
    }

    fn keyboard(ctx: &egui::Context) -> Keyboard {
        use egui::Key::*;
        ctx.input(|i| {
            let k = |keys: &[egui::Key]| keys.iter().any(|&x| i.key_down(x));
            Keyboard {
                up: k(&[W, ArrowUp]),
                down: k(&[S, ArrowDown]),
                left: k(&[A, ArrowLeft]),
                right: k(&[D, ArrowRight]),
                walk: i.modifiers.shift,
                a: k(&[Space]),
                b: k(&[E]),
                z: k(&[Q]),
                r: k(&[R]),
                l: k(&[T]),
                start: k(&[Enter]),
                c_left: k(&[J]),
                c_right: k(&[L]),
                c_up: k(&[I]),
                c_down: k(&[K]),
            }
        })
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f32();
        self.last = now;
        if dt > 0.0 {
            self.fps = self.fps * 0.9 + (1.0 / dt) * 0.1;
        }

        // Input: poll the pad every display frame (PadMgr accumulates presses between game
        // frames), keyboard merged in.
        if let Some(p) = &mut self.pads {
            p.update();
        }
        let kb = Self::keyboard(&ctx);
        let pad = merge(self.pads.as_ref().and_then(|p| p.state()), &kb);
        self.last_pad = pad;
        self.world.poll(pad);
        ctx.input(|i| {
            if i.key_pressed(egui::Key::F1) {
                self.show_wire = !self.show_wire;
            }
            if i.key_pressed(egui::Key::F2) {
                self.show_hud = !self.show_hud;
            }
            if i.key_pressed(egui::Key::Backspace) {
                self.world.respawn();
            }
        });
        if ctx.input(|i| i.key_pressed(egui::Key::Tab)) {
            let child = self.world.player.adult;
            self.world = new_world(&self.assets, child);
        }
        self.world.advance(dt);

        let snap = self.world.render_snapshot();
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui, |ui| {
            let avail = ui.available_size();
            let ppp = ui.ctx().pixels_per_point();
            let (w, h) = (((avail.x * ppp) as u32).max(1), ((avail.y * ppp) as u32).max(1));
            if self.target.as_ref().is_none_or(|(t, _)| t.size != (w, h)) {
                let t = Target::new(&self.device, w, h);
                let mut er = self.render_state.renderer.write();
                let id = match self.target.take() {
                    Some((_, id)) => {
                        er.update_egui_texture_from_wgpu_texture(&self.device, &t.resolve_view, wgpu::FilterMode::Linear, id);
                        id
                    }
                    None => er.register_native_texture(&self.device, &t.resolve_view, wgpu::FilterMode::Linear),
                };
                self.target = Some((t, id));
            }
            let (target, id) = self.target.as_ref().unwrap();
            draw_frame(&mut self.renderer, &self.device, &self.queue, target, &mut self.assets, &mut self.scene, &self.world, &snap, self.show_wire);
            let rect = ui.add(egui::Image::new((*id, avail))).rect;

            if self.show_hud {
                let p = &self.world.player;
                let dev = match (&self.pads, &self.pad_error) {
                    (Some(pads), _) => pads.active_name().map(|n| format!("{n} [{}]", pads.profile().name)).unwrap_or("no pad (keyboard)".into()),
                    (None, Some(e)) => format!("pad backend error: {e}"),
                    _ => "keyboard".into(),
                };
                let text = format!(
                    "{place} | {age} Link | {fps:.0} fps, logic {hz} Hz\n\
                     action {act:?} ({dec})  anim {anim} @{frame:.1}\n\
                     pos ({x:.1}, {y:.1}, {z:.1})  speed {spd:.2}  vy {vy:.2}  {ground}\n\
                     input: {dev}\n  stick ({sx:+}, {sy:+}) [{btns}]  raw buttons held {raw:?}\n\
                     WASD/arrows stick, Shift walk, Space A, J/L C-left/right, F1 collision, Tab age, Backspace respawn",
                    place = self.assets.place,
                    age = if p.adult { "adult" } else { "child" },
                    fps = self.fps,
                    hz = oot_game::math::GAME_HZ,
                    act = p.action,
                    dec = p.action.decomp_name(),
                    anim = self.world.data.anim_name(p.skel.animation),
                    frame = p.skel.cur_frame,
                    x = p.actor.world_pos.x,
                    y = p.actor.world_pos.y,
                    z = p.actor.world_pos.z,
                    spd = p.linear_velocity,
                    vy = p.actor.velocity.y,
                    ground = if p.grounded() { "grounded" } else { "airborne" },
                    sx = self.last_pad.stick_x,
                    sy = self.last_pad.stick_y,
                    btns = self.last_pad.names(),
                    raw = self.pads.as_ref().map(|p| p.raw_held()).unwrap_or_default(),
                );
                let painter = ui.painter_at(rect);
                let galley = painter.layout_no_wrap(text, egui::FontId::monospace(13.0), egui::Color32::WHITE);
                let r = egui::Rect::from_min_size(rect.min + egui::vec2(8.0, 8.0), galley.size() + egui::vec2(12.0, 8.0));
                painter.rect_filled(r, 4.0, egui::Color32::from_black_alpha(150));
                painter.galley(r.min + egui::vec2(6.0, 4.0), galley, egui::Color32::WHITE);
                if !self.assets.marks.is_empty() && self.show_wire {
                    let _ = &self.assets.marks;
                }
            }
        });
        ctx.request_repaint();
    }
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let cli = Cli::parse();
    if cli.sheet.is_some() || cli.trace.is_some() || cli.screenshot.is_some() {
        return headless(&cli);
    }
    let assets = load_assets(&cli)?;
    let child = cli.child;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([cli.width as f32, cli.height as f32]).with_title("OoT clone: spike 03 (Player movement)"),
        ..Default::default()
    };
    eframe::run_native("oot_play", options, Box::new(move |cc| Ok(Box::new(App::new(cc, assets, child)?))))
        .map_err(|e| anyhow::anyhow!("{e}"))
}
