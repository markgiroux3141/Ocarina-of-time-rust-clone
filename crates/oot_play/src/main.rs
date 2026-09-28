//! Spike 03 play mode: Link from the ROM, driven by Player movement ported from the decomp,
//! on the synthetic test course or a real scene's collision, with an N64 pad or keyboard.
//!
//! Headless modes (`--sheet`, `--trace`) run a scripted input and write images / JSON to
//! the git-ignored `out/` folder.

mod gfx;
mod rooms;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use eframe::egui;
use glam::{Mat4, Vec3};
use oot_core::player::{Age, PlayerModel, PlayerRules};
use oot_core::project::Project;
use oot_core::room::SceneTables;
use oot_game::bgcheck::StaticCollision;
use oot_game::course;
use oot_game::data::GameData;
use oot_game::env::EnvTables;
use oot_game::input::{BTN_A, BTN_B, BTN_Z, PadState};
use oot_game::world::{CameraKind, Snapshot, World, scripted_input};
use oot_pad::{Keyboard, PadConfig, Pads, merge};
use oot_render::{Camera, GpuModel, Lighting, LineVertex, Renderer, Target};

#[derive(Parser)]
#[command(about = "Link moving through a test course with Player logic ported from the decomp")]
struct Cli {
    /// Load this scene from the ROM instead of the test course (e.g. spot04 = Kokiri
    /// Forest, spot00 = Hyrule Field, ydan = Deku Tree): rooms, lights, fog and collision.
    #[arg(long)]
    scene: Option<String>,
    /// Spawn point (index into the scene's spawn list).
    #[arg(long, default_value_t = 0)]
    spawn: usize,
    /// Time of day as HH:MM (`gSaveContext.dayTime`; a new save starts at 10:00).
    #[arg(long, default_value = "10:00")]
    time: String,
    /// Draw the scene's collision (coloured by surface class) instead of its rooms.
    #[arg(long)]
    collision: bool,
    /// Headless: fixed camera `eye_x,eye_y,eye_z,at_x,at_y,at_z` instead of the game camera.
    #[arg(long, value_delimiter = ',', allow_negative_numbers = true)]
    view: Vec<f32>,
    /// Use spike 03's follow camera instead of z_camera.c's (F3 toggles in the window).
    #[arg(long)]
    follow_camera: bool,
    /// Turn off foot IK (func_8008F87C) to compare (F4 toggles in the window).
    #[arg(long)]
    no_foot_ik: bool,
    /// Place a dummy Z-target this many units in front of the spawn (0 = none).
    #[arg(long, default_value_t = 0.0)]
    target: f32,
    /// Place Link at `x,y,z,yaw` (yaw in binary angle units) instead of the spawn or the
    /// script's start; works in scenes too.
    #[arg(long, value_delimiter = ',', allow_negative_numbers = true)]
    at: Vec<f32>,
    /// Headless: contact sheet frame stride.
    #[arg(long, default_value_t = 2)]
    step: usize,
    /// Headless: run exactly this many game frames of the script (padding with idle frames or
    /// cutting it short; 0 = the script's own length).
    #[arg(long, default_value_t = 0)]
    frames: usize,
    /// Start as child Link.
    #[arg(long)]
    child: bool,
    /// Headless: run a built-in script and write a contact sheet PNG.
    #[arg(long)]
    sheet: Option<PathBuf>,
    /// Headless: run a built-in script and write a per-frame JSON trace.
    #[arg(long)]
    trace: Option<PathBuf>,
    /// Script for --sheet / --trace: run-roll, ledge, pit, stairs, walls, turn, idle, still,
    /// forward, tour, climb50, climb70, climb100, hang, ramp-stand, target, parallel, sword,
    /// swim, tread, platform.
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
    project: Project,
    data: Arc<GameData>,
    link: gfx::LinkGfx,
    collision: oot_core::collision::CollisionHeader,
    spawn: (Vec3, i16),
    place: String,
    marks: Vec<(&'static str, Vec3)>,
    tables: Option<SceneTables>,
    env_tables: Option<EnvTables>,
    scene: Option<rooms::LoadedScene>,
    scene_name: Option<String>,
    spawn_index: usize,
    day_time: u16,
    show_collision: bool,
    view: Option<(Vec3, Vec3)>,
    follow_camera: bool,
    foot_ik: bool,
    /// Dummy target positions.
    targets: Vec<Vec3>,
    /// `gDTSlidingPlatformCol`, for the course's moving platform.
    platform_col: Option<Arc<oot_core::collision::CollisionHeader>>,
}

fn parse_time(s: &str) -> Result<u16> {
    let (h, m) = s.split_once(':').context("--time wants HH:MM")?;
    Ok(oot_game::env::clock_time(h.trim().parse()?, m.trim().parse()?) as u16)
}

fn load_assets(cli: &Cli) -> Result<Assets> {
    let project = Project::open_default()?;
    let data = Arc::new(GameData::load(&project).context("loading Player data")?);
    let rules = PlayerRules::load(&project.config.decomp)?;
    let adult = PlayerModel::load(&project, &rules, Age::Adult)?;
    let child = PlayerModel::load(&project, &rules, Age::Child)?;
    let link = gfx::LinkGfx::new(rules, adult, child);
    let c = course::build();
    let mut a = Assets {
        data,
        link,
        collision: c.collision,
        spawn: (c.spawn, c.spawn_yaw),
        place: "test course".to_string(),
        marks: c.marks,
        tables: None,
        env_tables: None,
        scene: None,
        scene_name: cli.scene.clone(),
        spawn_index: cli.spawn,
        day_time: parse_time(&cli.time)?,
        show_collision: cli.collision,
        follow_camera: cli.follow_camera,
        foot_ik: !cli.no_foot_ik,
        targets: Vec::new(),
        platform_col: None,
        view: (cli.view.len() == 6).then(|| (Vec3::new(cli.view[0], cli.view[1], cli.view[2]), Vec3::new(cli.view[3], cli.view[4], cli.view[5]))),
        project,
    };
    if a.scene_name.is_none() {
        a.platform_col = Some(oot_game::bg_ydan_hasi::load_collision(&a.project)?);
    }
    if a.scene_name.is_some() {
        a.tables = Some(SceneTables::load(&a.project.config.decomp)?);
        a.env_tables = Some(EnvTables::load(&a.project.config.decomp)?);
        load_scene(&mut a, cli.child)?;
    }
    if let [x, y, z, yaw] = cli.at[..] {
        a.spawn = (Vec3::new(x, y, z), yaw as i32 as i16);
    }
    let dist = if cli.target == 0.0 && cli.script == "target" { 200.0 } else { cli.target };
    if dist > 0.0 {
        let (pos, yaw) = a.spawn;
        let r = oot_game::math::binang_to_rad(yaw);
        a.targets.push(pos + Vec3::new(r.sin(), 0.0, r.cos()) * dist);
    }
    Ok(a)
}

/// (Re)loads the scene for Link's age: the layer, and with it the collision, spawns and rooms,
/// depend on it.
fn load_scene(a: &mut Assets, child: bool) -> Result<()> {
    let (Some(name), Some(tables), Some(env_tables)) = (&a.scene_name, &a.tables, &a.env_tables) else { return Ok(()) };
    let t0 = Instant::now();
    let s = rooms::LoadedScene::load(&a.project, tables, env_tables, name, child, a.day_time)?;
    let sc = &s.draw.scene;
    let sp = sc.spawns.get(a.spawn_index).or(sc.spawns.first()).copied();
    let (pos, yaw) = sp.map(|e| (Vec3::new(e.pos[0] as f32, e.pos[1] as f32, e.pos[2] as f32), e.rot[1])).unwrap_or((Vec3::ZERO, 0));
    println!(
        "{} (layer {}): {} rooms, {} triangles, draw config {}, keep {}; {} polys, {} spawns; spawn {} at {pos} yaw {yaw:#x}; \
         lights: ambient {:?}, light1 {:?} dir {:?}, fog {:?} near {} far {} ({:.0} ms)",
        sc.name,
        sc.layer,
        s.meshes.len(),
        s.triangles(),
        s.draw.draw_fn,
        s.draw.keep_file.as_deref().unwrap_or("-"),
        sc.collision.polys.len(),
        sc.spawns.len(),
        a.spawn_index,
        s.lights.ambient,
        s.lights.light1_color,
        s.lights.light1_dir,
        s.lights.fog_color,
        s.lights.fog_near,
        s.lights.fog_far,
        t0.elapsed().as_secs_f64() * 1000.0
    );
    for n in &s.notes {
        println!("  note: {n}");
    }
    a.collision = sc.collision.clone();
    a.spawn = (pos, yaw);
    a.place = sc.short_name().to_string();
    a.marks = Vec::new();
    a.scene = Some(s);
    Ok(())
}

fn new_world(a: &Assets, child: bool) -> World {
    let mut w = World::new(a.data.clone(), StaticCollision::new(a.collision.clone()), !child, a.spawn.0, a.spawn.1);
    if a.follow_camera {
        w.toggle_camera();
    }
    w.foot_ik = a.foot_ik;
    w.targets = a.targets.iter().map(|&p| oot_game::target::TargetActor::dummy(p)).collect();
    add_platform(&mut w, a);
    w
}

/// The course's moving platform (`Bg_Ydan_Hasi` floating block) in its channel.
fn add_platform(w: &mut World, a: &Assets) {
    if let (Some(h), None) = (&a.platform_col, &a.scene) {
        w.spawn_platform(h.clone(), course::PLATFORM_HOME, course::PLATFORM_YAW, course::CHANNEL_WATER);
    }
}

/// The render camera for a snapshot, pulled in front of any wall between Link and the eye
/// (a simple stand-in; `z_camera.c` is not ported). In a scene it uses the game's projection:
/// `fovy` 60, `zNear` 10 and the far plane at `fogFar` (`Play_Draw`).
fn camera_of(s: &Snapshot, col: &StaticCollision, scene: Option<&rooms::LoadedScene>, kind: CameraKind) -> Camera {
    let clip = scene.map(|sc| (rooms::Z_NEAR, sc.fog_far()));
    if kind == CameraKind::Game {
        // z_camera.c's eye and at, and its fov (View_SetPerspective).
        let d = s.view.eye - s.view.at;
        let distance = d.length().max(0.01);
        return Camera {
            target: s.view.at,
            yaw: d.x.atan2(d.z),
            pitch: (d.y / distance).clamp(-1.0, 1.0).asin(),
            distance,
            fov_y: s.view.fov.to_radians(),
            clip: Some(clip.unwrap_or((rooms::Z_NEAR, 12800.0))),
        };
    }
    let mut distance = s.camera.distance;
    let eye = s.camera.eye();
    if let Some((hit, _)) = col.check_line(0, 0, s.camera.at, eye, 1.0, oot_game::bgcheck::CHECK_WALL | oot_game::bgcheck::CHECK_CEILING | oot_game::bgcheck::CHECK_FLOOR) {
        distance = ((hit - s.camera.at).length() - 12.0).max(30.0);
    }
    let (fov_y, clip) = match scene {
        Some(sc) => (rooms::FOVY.to_radians(), Some((rooms::Z_NEAR, sc.fog_far()))),
        None => (50f32.to_radians(), None),
    };
    Camera { target: s.camera.at, yaw: s.camera.yaw, pitch: s.camera.pitch, distance, fov_y, clip }
}

/// Uploads the static scene (rooms or collision mesh) and the shadow.
struct SceneGfx {
    course: Option<GpuModel>,
    /// The course's water boxes.
    water: Option<GpuModel>,
    /// One model per bg actor (the moving platform).
    platforms: Vec<GpuModel>,
    rooms: Option<rooms::RoomsGfx>,
    shadow: GpuModel,
    wire: Vec<LineVertex>,
    /// One box per dummy target.
    targets: Vec<GpuModel>,
}

impl SceneGfx {
    fn new(r: &mut Renderer, device: &wgpu::Device, queue: &wgpu::Queue, a: &Assets) -> SceneGfx {
        let rooms = a.scene.as_ref().filter(|_| !a.show_collision).map(|s| rooms::RoomsGfx::upload(r, device, queue, &s.meshes));
        let course = rooms.is_none().then(|| {
            let mut course = r.upload(device, queue, &gfx::collision_draw_list(&a.collision));
            course.pose(&[], Mat4::IDENTITY);
            course
        });
        let water = (course.is_some() && !a.collision.water_boxes.is_empty()).then(|| {
            let mut m = r.upload(device, queue, &gfx::water_draw_list(&a.collision));
            m.pose(&[], Mat4::IDENTITY);
            m
        });
        let shadow = r.upload(device, queue, &gfx::shadow_draw_list());
        let platforms = match (&a.platform_col, &a.scene) {
            (Some(_), None) => match gfx::platform_draw_list(&a.project) {
                Ok(d) => vec![r.upload(device, queue, &d)],
                Err(e) => {
                    eprintln!("platform: {e:#}");
                    Vec::new()
                }
            },
            _ => Vec::new(),
        };
        SceneGfx { course, water, platforms, rooms, shadow, wire: gfx::collision_lines(&a.collision, 0.5), targets: Vec::new() }
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
    let root = gfx::actor_matrix(snap.pos + Vec3::Y * (snap.y_offset * 0.01), snap.facing, 0.01);
    // Blob shadow on the floor below Player.
    let (floor, _) = world.col.entity_raycast_down(snap.pos + Vec3::Y * 20.0);
    let drop = (snap.pos.y - floor).max(0.0);
    let size = if world.player.adult { 22.0 } else { 16.0 } * (1.0 - (drop / 400.0).min(0.7));
    scene.shadow.pose(&[], Mat4::from_translation(Vec3::new(snap.pos.x, floor + 0.3, snap.pos.z)) * Mat4::from_scale(Vec3::new(size, 1.0, size)));
    let fists = snap.speed_xz > 2.0;
    let group = assets.data.items.model_group_names.get(snap.model_group).cloned().unwrap_or_default();
    let link = assets.link.get(r, device, queue, age, snap.joints.face, snap.face, fists, &group);
    link.pose(&mats, root);
    let link: &GpuModel = link;
    let mut cam = camera_of(snap, &world.col, assets.scene.as_ref(), world.camera_kind);
    if let Some((eye, at)) = assets.view {
        let d = eye - at;
        cam.target = at;
        cam.distance = d.length();
        cam.pitch = (d.y / cam.distance).asin();
        cam.yaw = d.x.atan2(d.z);
    }
    let lines = if show_wire { scene.wire.clone() } else { Vec::new() };
    let mut overlay = Vec::new();
    // Dummy targets, and the reticle on the locked one.
    while scene.targets.len() < world.targets.len() {
        scene.targets.push(r.upload(device, queue, &gfx::target_draw_list()));
    }
    for (m, t) in scene.targets.iter_mut().zip(&world.targets) {
        m.pose(&[], Mat4::from_translation(t.pos));
    }
    if let Some(i) = world.target_ctx.targeted {
        let spin = world.frames as f32 * 0.15;
        overlay.extend(gfx::reticle_lines(world.targets[i].focus, world.target_ctx.unk_44, cam.eye(), spin));
    }
    let (light, clear) = match &assets.scene {
        Some(s) => (s.lighting(), s.clear_color()),
        None => (Lighting::default(), CLEAR),
    };
    // Play_Draw: the OPA buffer (rooms, then actors) runs before the XLU buffer (rooms, then
    // actors' translucent parts such as the circle shadow).
    let mut models: Vec<&GpuModel> = Vec::new();
    if let (Some(rg), Some(s)) = (&mut scene.rooms, &assets.scene) {
        let (tables, project) = (assets.tables.as_ref().unwrap(), &assets.project);
        rg.update(queue, world.frames, || s.segment_values(project, tables, world.frames));
        let (opa, xlu) = rg.ordered(cam.view(), s.fog_far());
        models.extend(opa);
        models.extend(scene.targets.iter().take(world.targets.len()));
        models.push(link);
        models.extend(xlu);
        models.push(&scene.shadow);
    } else {
        models.extend(scene.course.as_ref());
        for (m, t) in scene.platforms.iter_mut().zip(&snap.platforms) {
            m.pose(&[], oot_game::dyna::srt_matrix(t));
        }
        models.extend(scene.platforms.iter().take(snap.platforms.len()));
        models.extend(scene.targets.iter().take(world.targets.len()));
        models.push(&scene.shadow);
        models.push(link);
        models.extend(scene.water.as_ref());
    }
    let mut enc = device.create_command_encoder(&Default::default());
    r.render(device, queue, &mut enc, target, &models, &cam, &light, &lines, &overlay, clear);
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
        // Run into the 50 / 70 / 100 high ledges (climb classes 2, 3, 4).
        "climb50" | "climb70" | "climb100" => {
            s.extend(rep(stick(0, 80), 45));
            s.extend(rep(stick(0, 0), 15));
            let z = match name { "climb50" => 550.0, "climb70" => 700.0, _ => 850.0 };
            start = Some((Vec3::new(560.0, 0.0, z), 0x4000));
        }
        // Walk slowly off the plateau, hang, then climb back up.
        "hang" => {
            s.extend(rep(stick(0, 30), 32));
            s.extend(rep(stick(0, 0), 16));
            s.extend(rep(stick(0, 80), 40));
            start = Some((Vec3::new(-540.0, 150.0, -700.0), 0x4000));
        }
        // Z-targeting a dummy 200 ahead: lock on, sidestep left, side hop, backflip, let go.
        "target" => {
            let z = |p: PadState| PadState { button: p.button | BTN_Z, ..p };
            s.push(stick(0, 0));
            s.push(z(stick(0, 0)));
            s.extend(rep(stick(0, 0), 12));
            s.extend(rep(stick(-80, 0), 24));
            s.push(PadState { button: BTN_A, stick_x: -80, stick_y: 0 });
            s.extend(rep(stick(0, 0), 16));
            s.push(stick(0, -80));
            s.push(PadState { button: BTN_A, stick_x: 0, stick_y: -80 });
            s.extend(rep(stick(0, 0), 20));
            s.push(z(stick(0, 0)));
            s.extend(rep(stick(0, 0), 6));
            start = None;
        }
        // Parallel mode: hold Z with nothing to target, strafe left, walk back, release.
        "parallel" => {
            let z = |p: PadState| PadState { button: p.button | BTN_Z, ..p };
            s.push(stick(0, 0));
            s.extend(rep(z(stick(0, 0)), 4));
            s.extend(rep(z(stick(-80, 0)), 16));
            s.extend(rep(z(stick(0, -80)), 16));
            s.extend(rep(stick(0, 0), 8));
            start = None;
        }
        // B three times (draw the sword and slash, slash, combo finisher), then A puts it away.
        "sword" => {
            let b = PadState { button: BTN_B, stick_x: 0, stick_y: 0 };
            s.extend(rep(stick(0, 0), 2));
            for _ in 0..3 {
                s.push(b);
                s.extend(rep(stick(0, 0), 14));
            }
            s.extend(rep(stick(0, 0), 10));
            s.push(PadState { button: BTN_A, stick_x: 0, stick_y: 0 });
            s.extend(rep(stick(0, 0), 16));
            start = None;
        }
        // Run down the pool's ramp into deep water, tread water, dive holding A, rise and
        // surface, then swim back up the ramp and out.
        "swim" => {
            let a = PadState { button: BTN_A, stick_x: 0, stick_y: 0 };
            s.extend(rep(stick(0, 80), 30));
            s.extend(rep(stick(0, 0), 36));
            s.extend(rep(a, 50));
            s.extend(rep(stick(0, 0), 70));
            s.extend(rep(stick(0, -80), 80));
            start = Some((Vec3::new(-450.0, 0.0, 800.0), -0x4000));
        }
        // Ride the moving platform (Bg_Ydan_Hasi) across the channel, then run off its far
        // edge and jump to the bank.
        "platform" => {
            s.extend(rep(stick(0, 0), 44));
            s.extend(rep(stick(0, 80), 24));
            s.extend(rep(stick(0, 0), 12));
            start = Some((Vec3::new(650.0, 10.0, -200.0), 0x4000));
        }
        // Tread water, then swim a lap (for scenes, with --at).
        "tread" => {
            s.extend(rep(stick(0, 0), 60));
            s.extend(rep(stick(0, 80), 40));
            s.extend(rep(stick(80, 40), 20));
            start = None;
        }
        // Standing across ramp A (20.6°), for foot IK.
        "ramp-stand" => {
            s.extend(rep(stick(0, 0), 20));
            start = Some((Vec3::new(-700.0, 80.0, -300.0), 0x4000));
        }
        // A few frames standing at the spawn (for screenshots).
        "still" => {
            s.extend(rep(stick(0, 0), 8));
            start = None;
        }
        // Run, curve left, curve right, then stop and let the camera recentre.
        "tour" => {
            s.extend(rep(stick(0, 80), 20));
            s.extend(rep(stick(-60, 60), 25));
            s.extend(rep(stick(60, 60), 25));
            s.extend(rep(stick(0, 0), 50));
            start = None;
        }
        // Walk forward from the spawn, then stand.
        "forward" => {
            s.extend(rep(stick(0, 80), 40));
            s.extend(rep(stick(0, 0), 10));
            start = None;
        }
        other => anyhow::bail!("unknown script {other}"),
    }
    Ok((s, start))
}

fn run_script(a: &Assets, cli: &Cli) -> Result<(World, Vec<Snapshot>, Vec<serde_json::Value>)> {
    let (mut s, start) = script(&cli.script)?;
    if cli.frames > 0 {
        s.resize(cli.frames, stick(0, 0));
    }
    let mut w = new_world(a, cli.child);
    if let (Some((p, y)), None) = (start, &cli.scene) {
        w = World::new(a.data.clone(), StaticCollision::new(a.collision.clone()), !cli.child, p, y);
        if a.follow_camera {
            w.toggle_camera();
        }
        w.foot_ik = a.foot_ik;
        w.targets = a.targets.iter().map(|&p| oot_game::target::TargetActor::dummy(p)).collect();
        add_platform(&mut w, a);
    }
    if let [x, y, z, yaw] = cli.at[..] {
        w = World::new(a.data.clone(), StaticCollision::new(a.collision.clone()), !cli.child, Vec3::new(x, y, z), yaw as i32 as i16);
        if a.follow_camera {
            w.toggle_camera();
        }
        w.foot_ik = a.foot_ik;
        add_platform(&mut w, a);
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
            "target": p.unk_664,
            "parallel": p.state1 & oot_game::player::STATE1_17 != 0,
            "locked_on": p.state1 & oot_game::player::STATE1_4 != 0,
            "root_joint": p.skel.joint[0],
            "root_rot": p.skel.joint[1],
            "y_offset": p.actor.shape_y_offset,
            "camera": { "eye": w.game_camera.eye.to_array(), "at": w.game_camera.at.to_array(), "dist": w.game_camera.dist, "fov": w.game_camera.fov, "input_yaw": w.game_camera.input_dir_yaw() },
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
    let mut scene = SceneGfx::new(&mut r, &device, &queue, &a);
    if let Some(p) = &cli.screenshot {
        let t = Target::new(&device, cli.width, cli.height);
        draw_frame(&mut r, &device, &queue, &t, &mut a, &mut scene, &w, snaps.last().unwrap(), cli.wire);
        save_png(p, cli.width, cli.height, &t.read_rgba(&device, &queue)?)?;
        println!("{}", p.display());
    }
    if let Some(p) = &cli.sheet {
        // Every `step` frames, 6 columns.
        let picks: Vec<usize> = (0..snaps.len()).step_by(cli.step.max(1)).collect();
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
        let scene = SceneGfx::new(&mut renderer, &device, &queue, &assets);
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
            if i.key_pressed(egui::Key::F3) {
                self.world.toggle_camera();
            }
            if i.key_pressed(egui::Key::F4) {
                self.world.foot_ik = !self.world.foot_ik;
                self.assets.foot_ik = self.world.foot_ik;
            }
            if i.key_pressed(egui::Key::Backspace) {
                self.world.respawn();
            }
        });
        if ctx.input(|i| i.key_pressed(egui::Key::Tab)) {
            let child = self.world.player.adult;
            // The scene layer depends on Link's age.
            if self.assets.scene.is_some() {
                match load_scene(&mut self.assets, child) {
                    Ok(()) => self.scene = SceneGfx::new(&mut self.renderer, &self.device, &self.queue, &self.assets),
                    Err(e) => log::error!("reloading the scene: {e:#}"),
                }
            }
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
                let rooms_line = match (&self.scene.rooms, &self.assets.scene) {
                    (Some(rg), Some(s)) => format!(
                        "rooms {} ({} tris), entries drawn {}, frame {}, fog near {} far {}\n",
                        s.meshes.len(),
                        s.triangles(),
                        rg.drawn_entries,
                        self.world.frames,
                        s.lights.fog_near,
                        s.lights.fog_far
                    ),
                    _ => String::new(),
                };
                let text = format!(
                    "{place} | {age} Link | {fps:.0} fps, logic {hz} Hz | camera {cam:?} (F3)\n{rooms_line}\
                     action {act:?} ({dec})  anim {anim} @{frame:.1}\n\
                     pos ({x:.1}, {y:.1}, {z:.1})  speed {spd:.2}  vy {vy:.2}  {ground}\n\
                     input: {dev}\n  stick ({sx:+}, {sy:+}) [{btns}]  raw buttons held {raw:?}\n\
                     WASD/arrows stick, Shift walk, Space A, E B (sword), Q Z-target, J/L C-left/right, F1 collision, F3 camera, F4 foot IK, Tab age, Backspace respawn",
                    place = self.assets.place,
                    cam = self.world.camera_kind,
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
