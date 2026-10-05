//! The play client: loads Link, a scene (or the test course) and the game data from the asset
//! pack, runs the ported game logic (`oot_game::play::PlayState` with `oot_actors`), and draws
//! it. The `oot` binary plays with it; `oot_sandbox` adds the scripted headless runs on top.
//!
//! Two ways into a scene:
//! - **An entrance** (`Options::entrance`, the game's way): `Play_Init` from the pack, with
//!   every actor placement spawned (ported or a placeholder), rooms loaded and changed by the
//!   room context, and exits leading to other scenes.
//! - **The spikes' way** (`Options::scene` alone, the sandbox's default): the scene's collision
//!   and every room drawn, Player alone at a spawn, and nothing else spawned.
//!
//! Nothing here reads the ROM or the decomp: `open_pack` builds the pack with `oot_import` if
//! there's none (or it's stale), then everything comes from it.

pub mod gfx;
pub mod rooms;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use eframe::egui;
use eng_app::ViewportTarget;
use eng_collision::bgcheck::CollisionContext;
use eng_input::device::{PadConfig, Pads, merge};
use eng_input::pad::PadState;
use eng_gfx::{DrawCmd, DrawList, DrawLists, MeshKey};
use eng_render::{Camera, Lighting, LineVertex, MeshCache, MeshSource, Renderer, Target};
use glam::{Mat4, Vec3};
use oot_actors::PlayExt;
use oot_game::camera::CameraKind;
use oot_game::play::{PlayState, RenderFrame, ViewInfo};
use oot_game::play_scene::{GameAssets, SceneState};
use oot_game::save::SaveContext;
use oot_game::player_lib::PlayerRules;
use oot_game::course;
use oot_game::data::GameData;
use oot_game::env::EnvTables;
use oot_game::pack::GamePack;

/// What to play and how: the scene (or the test course), where Link starts, the time of day,
/// and the debug views.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// A scene from the ROM (e.g. spot04 = Kokiri Forest, spot00 = Hyrule Field, ydan = Deku
    /// Tree), or the test course when `None`.
    pub scene: Option<String>,
    /// Spawn point (index into the scene's spawn list).
    pub spawn: usize,
    /// Time of day as HH:MM (`gSaveContext.save.dayTime`).
    pub time: String,
    /// Draw the scene's collision (coloured by surface class) instead of its rooms.
    pub collision: bool,
    /// Fixed camera `eye_x,eye_y,eye_z,at_x,at_y,at_z` (headless).
    pub view: Vec<f32>,
    /// Spike 03's follow camera instead of z_camera.c's.
    pub follow_camera: bool,
    /// Foot IK (func_8008F87C) off.
    pub no_foot_ik: bool,
    /// A dummy Z-target this many units in front of the spawn (0 = none); in a scene entered by
    /// `Play_Init`, in front of `at`.
    pub target: f32,
    /// The dummy targets hurt Link when he touches them, with this hit special effect
    /// (`HIT_SPECIAL_EFFECT_*`; `parse_hit_effect`).
    pub target_hurts: Option<u8>,
    /// Link at `x,y,z,yaw` instead of the spawn (with an entrance: after `Play_Init`, and
    /// after `room`).
    pub at: Vec<f32>,
    /// With an entrance: after `Play_Init`, this room loaded as walking into it would load it
    /// (a debug start somewhere the spawns don't reach, e.g. room 2's sword chest).
    pub room: Option<i8>,
    /// Child Link.
    pub child: bool,
    /// A pack file or loose folder to use instead of the default pack.
    pub pack: Option<PathBuf>,
    /// Enter by `Play_Init`: an `ENTR_*` name or index, or `Some("")` for the first entrance
    /// to `scene` at `spawn`.
    pub entrance: Option<String>,
    /// Draw markers where unported actors (placeholders) are.
    pub placeholders: bool,
    /// A debug save preset (`oot_game::save::SAVE_PRESETS`) for the new save `Play_Init`
    /// enters with.
    pub preset: Option<String>,
    /// A new file as the file select starts it (`SaveContext::file_select_new`): Link's house
    /// with the opening (`cutsceneIndex` 0xFFF1). Implies `ENTR_LINKS_HOUSE_0` and child Link.
    pub new_file: bool,
    /// Play sound through the output device (the window only).
    pub audio: bool,
    /// A sequence to play instead of the first scene's own (`gSequenceTable`'s index, e.g. 30
    /// = the title theme): `Environment_ForcePlaySequence` before the first `Play_Init`. In the
    /// spikes' view (no `Play_Init`), started on player 0 directly.
    pub music: Option<u8>,
    /// Log what the game's side does to the audio from the start (`GameAudio::log`).
    pub audio_log: bool,
    /// A custom level: a folder the overworld editor or `overworld build` wrote (`level.json`
    /// and `textures/`), played in Kokiri Forest's light. Reloaded whenever it's rebuilt.
    pub level: Option<PathBuf>,
}

/// The pack to play from: `path` if given, else `$OOT_PACK`, else the default one
/// (`oot_game::pack`), which is imported from the ROM and decomp in `oot.toml` first if it's
/// missing or stale. An explicit pack is never imported over.
pub fn open_pack(path: Option<&std::path::Path>) -> Result<GamePack> {
    if let Some(p) = path {
        return GamePack::open(p);
    }
    if let Some(p) = std::env::var_os(oot_game::pack::ENV_PACK).filter(|v| !v.is_empty()) {
        return GamePack::open(std::path::Path::new(&p));
    }
    let current = oot_game::pack::default_pack_path().ok().filter(|p| oot_game::pack::is_current(p, None));
    let path = match current {
        Some(p) => p,
        None => {
            eprintln!("no current asset pack: importing from the ROM in oot.toml (once) ...");
            let project = oot_import::project::Project::open_default().context("importing needs oot.toml with your ROM and the decomp")?;
            let (report, path) = oot_import::pack::import_to_default(&project, None)?;
            eprint!("{}", oot_import::pack::summary(&report, &path));
            path
        }
    };
    GamePack::open(&path)
}

pub const CLEAR: [f64; 4] = [0.55, 0.68, 0.82, 1.0];

/// Everything loaded from the pack.
pub struct Assets {
    pub pack: GamePack,
    pub data: Arc<GameData>,
    pub rules: Arc<PlayerRules>,
    pub link: gfx::LinkGfx,
    pub collision: eng_collision::collision::CollisionHeader,
    pub spawn: (Vec3, i16),
    pub place: String,
    pub marks: Vec<(&'static str, Vec3)>,
    pub env_tables: EnvTables,
    /// The spikes' way into a scene: every room, attached to each new play state.
    pub scene: Option<SceneState>,
    /// The game's way: the pack's tables and the ported actors, and the entrance.
    pub game: Option<Arc<GameAssets>>,
    pub entrance: Option<u16>,
    pub placeholders: bool,
    /// The debug save preset, if any.
    pub preset: Option<String>,
    /// Enter with the file select's new file (`Options::new_file`).
    pub new_file: bool,
    /// With an entrance: the room to change to after `Play_Init`, and where to put Link then
    /// (`--room`, `--at`).
    pub start_room: Option<i8>,
    pub start_at: Option<(Vec3, i16)>,
    pub scene_name: Option<String>,
    pub spawn_index: usize,
    /// Open the output device (`Options::audio`), the sequence to force (`Options::music`), the
    /// audio log (`Options::audio_log`).
    pub audio: bool,
    pub music: Option<u8>,
    pub audio_log: bool,
    pub day_time: u16,
    pub show_collision: bool,
    pub view: Option<(Vec3, Vec3)>,
    pub follow_camera: bool,
    pub foot_ik: bool,
    /// Dummy target positions, and their touch's hit special effect if they hurt.
    pub targets: Vec<Vec3>,
    pub target_hurts: Option<u8>,
    /// `gDTSlidingPlatformCol`, for the course's moving platform.
    pub platform_col: Option<Arc<eng_collision::collision::CollisionHeader>>,
    /// A custom level's folder, and its `level.json`'s time when loaded (to reload a rebuild).
    pub level: Option<(PathBuf, Option<std::time::SystemTime>)>,
}

pub fn parse_time(s: &str) -> Result<u16> {
    let (h, m) = s.split_once(':').context("--time wants HH:MM")?;
    Ok(oot_game::env::clock_time(h.trim().parse()?, m.trim().parse()?) as u16)
}

/// An entrance by `ENTR_*` name, index (decimal or 0x hex), or "" for the first entrance to
/// `scene` at `spawn` (the layer-0 row of its group).
pub fn find_entrance(g: &GameAssets, spec: &str, scene: Option<&str>, spawn: usize) -> Result<u16> {
    if spec.is_empty() {
        let name = scene.context("an entrance needs --scene or a name")?;
        let file = if name.ends_with("_scene") { name.to_string() } else { format!("{name}_scene") };
        let id = g.scenes.by_file(&file).with_context(|| format!("no scene {file}"))?.id;
        return g.scenes.entrances.iter().position(|e| e.scene == id && e.spawn as usize == spawn).map(|i| i as u16).with_context(|| format!("no entrance to {file} spawn {spawn}"));
    }
    if let Some(i) = g.scenes.entrance_index(spec) {
        return Ok(i);
    }
    let n = match spec.strip_prefix("0x") {
        Some(h) => u16::from_str_radix(h, 16).ok(),
        None => spec.parse().ok(),
    };
    n.filter(|&i| (i as usize) < g.scenes.entrances.len()).with_context(|| format!("no entrance {spec}"))
}

pub fn load_assets(o: &Options) -> Result<Assets> {
    let t0 = Instant::now();
    let pack = open_pack(o.pack.as_deref())?;
    let t_open = t0.elapsed();
    let data = Arc::new(pack.game_data().context("loading Player data")?);
    let t_data = t0.elapsed();
    let link = gfx::LinkGfx::load(&pack)?;
    log::info!(
        "pack {}: opened in {:.1} ms, Player data in {:.1} ms, Link's meshes in {:.1} ms",
        pack.assets.location(),
        t_open.as_secs_f64() * 1e3,
        (t_data - t_open).as_secs_f64() * 1e3,
        (t0.elapsed() - t_data).as_secs_f64() * 1e3
    );
    let c = course::build();
    let mut a = Assets {
        rules: Arc::new(pack.player_rules()?),
        data,
        link,
        collision: c.collision,
        spawn: (c.spawn, c.spawn_yaw),
        place: "test course".to_string(),
        marks: c.marks,
        env_tables: pack.env_tables()?,
        scene: None,
        game: None,
        entrance: None,
        placeholders: o.placeholders,
        preset: o.preset.clone(),
        new_file: o.new_file,
        start_room: o.room,
        start_at: None,
        scene_name: o.scene.clone(),
        spawn_index: o.spawn,
        audio: o.audio,
        music: o.music,
        audio_log: o.audio_log,
        day_time: parse_time(&o.time)?,
        show_collision: o.collision,
        follow_camera: o.follow_camera,
        foot_ik: !o.no_foot_ik,
        targets: Vec::new(),
        target_hurts: o.target_hurts,
        platform_col: None,
        level: None,
        view: (o.view.len() == 6).then(|| (Vec3::new(o.view[0], o.view[1], o.view[2]), Vec3::new(o.view[3], o.view[4], o.view[5]))),
        pack,
    };
    if let Some(p) = &o.preset {
        // Checked up front: a preset only applies to a save, so it needs an entrance.
        anyhow::ensure!(o.entrance.is_some(), "--preset needs --entrance (it sets the save Play_Init enters with)");
        SaveContext::default().apply_preset(p).map_err(anyhow::Error::msg)?;
    }
    anyhow::ensure!(!(o.new_file && o.preset.is_some()), "--new-file is the file select's new save: no --preset");
    let new_file_entrance = o.new_file.then(|| "ENTR_LINKS_HOUSE_0".to_string());
    if let Some(spec) = new_file_entrance.as_ref().or(o.entrance.as_ref().filter(|_| !o.new_file)) {
        // Play_Init reads the pack's tables through its own handle.
        let g = oot_actors::game_assets(open_pack(o.pack.as_deref())?)?;
        let e = find_entrance(&g, spec, o.scene.as_deref(), o.spawn)?;
        let info = &g.scenes.entrances[e as usize];
        println!("entrance {} ({e:#05x}): {} spawn {}", info.name, g.scenes.scenes[info.scene as usize].file, info.spawn);
        a.entrance = Some(e);
        a.place = info.name.clone();
        a.game = Some(g);
    } else if let Some(dir) = &o.level {
        load_level(&mut a, dir, o.child)?;
    } else if a.scene_name.is_none() {
        a.platform_col = Some(oot_actors::bg_ydan_hasi::load_collision(&a.pack)?);
    } else {
        load_scene(&mut a, o.child)?;
    }
    if let [x, y, z, yaw] = o.at[..] {
        a.spawn = (Vec3::new(x, y, z), yaw as i32 as i16);
        a.start_at = Some(a.spawn);
    }
    anyhow::ensure!(o.room.is_none() || o.entrance.is_some(), "--room needs --entrance (it changes rooms after Play_Init)");
    let dist = o.target;
    if dist > 0.0 {
        let (pos, yaw) = a.spawn;
        let r = eng_math::binang_to_rad(yaw);
        a.targets.push(pos + Vec3::new(r.sin(), 0.0, r.cos()) * dist);
    }
    Ok(a)
}

/// When a custom level's `level.json` was last written.
pub fn level_stamp(dir: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(dir.join("level.json")).and_then(|m| m.modified()).ok()
}

/// Loads a custom level (`oot_import::level`): its room and collision, Link at its middle (or
/// `--at`), in Kokiri Forest's light at the time of day (the overworld theme is Kokiri's).
pub fn load_level(a: &mut Assets, dir: &std::path::Path, child: bool) -> Result<()> {
    let t0 = Instant::now();
    let stamp = level_stamp(dir);
    let l = oot_import::level::load(dir)?;
    let lights = rooms::load_all_rooms(&a.pack, &a.env_tables, "spot04", child, a.day_time).context("Kokiri Forest's lights")?.lights;
    println!(
        "level {} from {}: {} triangles, {} collision polys ({} vertices), {} water boxes; spawn at {} ({:.0} ms)",
        l.name,
        dir.display(),
        l.triangles,
        l.collision.polys.len(),
        l.collision.vertices.len(),
        l.collision.water_boxes.len(),
        l.spawn.0,
        t0.elapsed().as_secs_f64() * 1000.0
    );
    for n in &l.notes {
        println!("  note: {n}");
    }
    a.scene = Some(oot_import::level::scene(&l, lights, child));
    a.collision = l.collision;
    a.spawn = l.spawn;
    a.place = l.name;
    a.marks = Vec::new();
    a.scene_name = None;
    a.level = Some((dir.to_path_buf(), stamp));
    Ok(())
}

/// (Re)loads the scene for Link's age: the layer, and with it the collision, spawns and rooms,
/// depend on it.
pub fn load_scene(a: &mut Assets, child: bool) -> Result<()> {
    let Some(name) = &a.scene_name else { return Ok(()) };
    let t0 = Instant::now();
    let s = rooms::load_all_rooms(&a.pack, &a.env_tables, name, child, a.day_time)?;
    let collision = a.pack.collision(&s.layer_data().collision)?;
    let spawns = s.layer_data().spawns.clone();
    let sp = spawns.get(a.spawn_index).or(spawns.first()).copied();
    let (pos, yaw) = sp.map(|e| (Vec3::new(e.pos[0] as f32, e.pos[1] as f32, e.pos[2] as f32), e.rot[1])).unwrap_or((Vec3::ZERO, 0));
    let ld = s.layer_data();
    println!(
        "{} (layer {}): {} rooms, {} triangles, draw config {}{}, keep {}; {} polys, {} spawns; spawn {} at {pos} yaw {yaw:#x}; \
         lights: ambient {:?}, light1 {:?} dir {:?}, fog {:?} near {} far {} ({:.0} ms)",
        s.data.name,
        s.layer,
        s.rooms.iter().filter(|r| r.shape.is_some()).count(),
        rooms::triangles(&s),
        s.data.draw_config,
        if oot_game::scene_table::is_ported(&s.data.draw_config) { "" } else { " (not ported: static)" },
        ld.keep_object.as_deref().unwrap_or("-"),
        collision.polys.len(),
        ld.spawns.len(),
        a.spawn_index,
        s.lights.ambient,
        s.lights.light1_color,
        s.lights.light1_dir,
        s.lights.fog_color,
        s.lights.fog_near,
        s.lights.fog_far,
        t0.elapsed().as_secs_f64() * 1000.0
    );
    for n in &ld.notes {
        println!("  note: {n}");
    }
    a.collision = collision;
    a.spawn = (pos, yaw);
    a.place = s.short_name().to_string();
    a.marks = Vec::new();
    a.scene = Some(s);
    Ok(())
}

/// The play state: `Play_Init` at the entrance, or Player at the spawn (or `--at`) with the
/// course's platform and the dummy targets.
pub fn new_play(a: &Assets, child: bool) -> PlayState {
    if let (Some(g), Some(e)) = (&a.game, a.entrance) {
        let mut save = if a.new_file { SaveContext::file_select_new() } else { SaveContext::new(e, !child, a.day_time) };
        if let Some(p) = &a.preset
            && let Err(e) = save.apply_preset(p)
        {
            log::error!("{e}");
        }
        if let Some(m) = a.music {
            // Environment_ForcePlaySequence: the first scene plays it instead of its own.
            save.forced_seq_id = m as u16;
        }
        let audio = oot_game::audio::GameAudio::boot_logged(g.audio.clone(), g.audio_tables.clone(), a.audio_log);
        match PlayState::play_init_with(g.clone(), a.data.clone(), a.rules.clone(), save, audio) {
            Ok(mut w) => {
                if let Some(r) = a.start_room {
                    // Room_RequestNewRoom, a frame for it to load, then Room_FinishRoomChange.
                    if w.room_request(r) {
                        w.tick_with(oot_game::play::scripted_input(PadState::default(), PadState::default()));
                        w.room_change_done();
                    }
                }
                if let Some((pos, yaw)) = a.start_at {
                    w.place_player(pos, yaw);
                }
                spawn_targets(&mut w, a);
                if a.follow_camera {
                    w.toggle_camera();
                }
                w.debug.foot_ik = a.foot_ik;
                w.debug.placeholders = a.placeholders;
                return w;
            }
            Err(e) => log::error!("Play_Init: {e:#}"),
        }
    }
    new_play_at(a, child, a.spawn.0, a.spawn.1, true)
}

/// The play state with Player at `pos` facing `yaw`, with the dummy targets or without.
pub fn new_play_at(a: &Assets, child: bool, pos: Vec3, yaw: i16, targets: bool) -> PlayState {
    let mut w = oot_actors::new_play(a.data.clone(), a.rules.clone(), CollisionContext::new(a.collision.clone()), !child, pos, yaw);
    w.scene = a.scene.clone();
    if a.follow_camera {
        w.toggle_camera();
    }
    w.debug.foot_ik = a.foot_ik;
    w.debug.placeholders = a.placeholders;
    // a custom level's camera starts as Play_Init's does, which lets floors call for their bg
    // cameras (a crawlspace's line)
    if a.level.is_some() {
        w.play_init_camera = true;
        w.reset_cameras();
    }
    if targets {
        spawn_targets(&mut w, a);
    }
    add_platform(&mut w, a);
    w
}

/// The dummy targets, hurting ones with `target_hurts`.
fn spawn_targets(w: &mut PlayState, a: &Assets) {
    for &p in &a.targets {
        match a.target_hurts {
            Some(e) => w.spawn_hurting_target(p, e),
            None => w.spawn_target(p),
        };
    }
}

/// `--target-hurts`'s names for the hit special effects (`HIT_SPECIAL_EFFECT_*`): none, fire,
/// ice, electric, knockback.
pub fn parse_hit_effect(s: &str) -> Result<u8> {
    use oot_game::collision_check::*;
    Ok(match s {
        "none" => HIT_SPECIAL_EFFECT_NONE,
        "fire" => HIT_SPECIAL_EFFECT_FIRE,
        "ice" => HIT_SPECIAL_EFFECT_ICE,
        "electric" => HIT_SPECIAL_EFFECT_ELECTRIC,
        "knockback" => HIT_SPECIAL_EFFECT_KNOCKBACK,
        _ => anyhow::bail!("--target-hurts: none, fire, ice, electric or knockback, not {s}"),
    })
}

/// The course's moving platform (`Bg_Ydan_Hasi` floating block) in its channel.
pub fn add_platform(w: &mut PlayState, a: &Assets) {
    if let (Some(h), None) = (&a.platform_col, &a.scene) {
        w.spawn_platform(h.clone(), course::PLATFORM_HOME, course::PLATFORM_YAW, course::CHANNEL_WATER);
    }
}

/// The render camera for a frame. The game camera uses `z_camera.c`'s eye, at and fov; the
/// follow camera is pulled in front of any wall between Link and the eye. In a scene it uses
/// the game's projection: `fovy` 60, `zNear` 10 and the far plane at `fogFar` (`Play_Draw`).
pub fn camera_of(f: &RenderFrame, col: &CollisionContext, scene: Option<&SceneState>, kind: CameraKind) -> Camera {
    let clip = scene.map(|sc| (rooms::Z_NEAR, rooms::fog_far(sc)));
    if kind == CameraKind::Game {
        // z_camera.c's eye and at, and its fov (View_SetPerspective).
        let d = f.view.eye - f.view.at;
        let distance = d.length().max(0.01);
        return Camera {
            target: f.view.at,
            yaw: d.x.atan2(d.z),
            pitch: (d.y / distance).clamp(-1.0, 1.0).asin(),
            distance,
            fov_y: f.view.fov.to_radians(),
            clip: Some(clip.unwrap_or((rooms::Z_NEAR, 12800.0))),
        };
    }
    let mut distance = f.follow.distance;
    let eye = f.follow.eye();
    if let Some((hit, _)) = col.check_line(0, 0, f.follow.at, eye, 1.0, eng_collision::bgcheck::CHECK_WALL | eng_collision::bgcheck::CHECK_CEILING | eng_collision::bgcheck::CHECK_FLOOR) {
        distance = ((hit - f.follow.at).length() - 12.0).max(30.0);
    }
    let (fov_y, clip) = match scene {
        Some(sc) => (rooms::FOVY.to_radians(), Some((rooms::Z_NEAR, rooms::fog_far(sc)))),
        None => (50f32.to_radians(), None),
    };
    Camera { target: f.follow.at, yaw: f.follow.yaw, pitch: f.follow.pitch, distance, fov_y, clip }
}

/// The meshes a frame draws: the renderer's cache, and where the meshes come from.
pub struct SceneGfx {
    cache: MeshCache,
    wire: Vec<LineVertex>,
    /// The play state's scene change count the wireframe is for.
    wire_for: u32,
    /// Culled room entries last frame.
    pub drawn_entries: usize,
}

impl SceneGfx {
    pub fn new(a: &Assets) -> SceneGfx {
        SceneGfx { cache: MeshCache::new(), wire: gfx::collision_lines(&a.collision, 0.5), wire_for: 0, drawn_entries: 0 }
    }
}

/// Mesh keys the app draws itself.
const COURSE_MESH: &str = "sandbox/course";
const WATER_MESH: &str = "sandbox/water";

/// Resolves mesh keys: Link's variants with their face, the pack's meshes, the loaded rooms,
/// the course and the built-in meshes.
struct Meshes<'a> {
    a: &'a mut Assets,
    scene: Option<&'a SceneState>,
    col: &'a CollisionContext,
}

impl MeshSource for Meshes<'_> {
    fn mesh(&mut self, key: &MeshKey) -> Option<DrawList> {
        let n = key.name.as_str();
        if n.starts_with("player/") {
            let t = |seg: u8| key.segment_textures.iter().find(|x| x.0 == seg).map(|x| x.1 as usize).unwrap_or(0);
            return self.a.link.mesh(n, t(8), t(9));
        }
        if let Some(rest) = n.strip_prefix("room/") {
            return rooms::entry_mesh(self.scene?, rest).or_else(|| rooms::background_mesh(self.scene?, rest));
        }
        match n {
            COURSE_MESH => Some(gfx::collision_draw_list(&self.col.header)),
            WATER_MESH => Some(gfx::water_draw_list(&self.a.collision)),
            oot_game::play::builtin::SHADOW => Some(gfx::shadow_draw_list()),
            oot_game::play::builtin::TARGET_BOX => Some(gfx::target_draw_list()),
            oot_game::play::builtin::PLACEHOLDER => Some(gfx::placeholder_draw_list()),
            _ => self.a.pack.assets.try_get::<DrawList>(n).ok().flatten(),
        }
    }
}

/// Draws one frame: `Play_Draw`'s OPA buffer (rooms, then actors), then its XLU buffer (rooms,
/// then actors' translucent parts such as the circle shadow). On the course there are no
/// rooms: the course's collision mesh draws first and its water boxes last.
#[allow(clippy::too_many_arguments)]
pub fn draw_frame(
    r: &mut Renderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    target: &Target,
    assets: &mut Assets,
    scene: &mut SceneGfx,
    play: &PlayState,
    frame: &RenderFrame,
    show_wire: bool,
) {
    let mut cam = camera_of(frame, &play.col, play.scene.as_ref(), play.camera_kind);
    if let Some((eye, at)) = assets.view {
        let d = eye - at;
        cam.target = at;
        cam.distance = d.length();
        cam.pitch = (d.y / cam.distance).asin();
        cam.yaw = d.x.atan2(d.z);
    }
    if scene.wire_for != play.scene_changes {
        scene.wire = gfx::collision_lines(&play.col.header, 0.5);
        scene.wire_for = play.scene_changes;
    }
    let lines = if show_wire { scene.wire.clone() } else { Vec::new() };
    let (light, clear) = match &play.scene {
        Some(s) => (rooms::lighting(s), rooms::clear_color(s)),
        None => (Lighting::default(), CLEAR),
    };
    let mut lists = DrawLists::default();
    match play.scene.as_ref().filter(|_| !assets.show_collision) {
        // Room_Draw, with the draw config's segment values for this frame.
        Some(s) => {
            scene.drawn_entries = rooms::submit_rooms(play, s, cam.view(), &mut lists);
            rooms::submit_room_skybox(play, s, cam.eye(), &mut lists);
        }
        None => lists.opa.push(DrawCmd::new(MeshKey::named(COURSE_MESH), Mat4::IDENTITY)),
    }
    // Actor_DrawAll.
    play.draw(frame, &ViewInfo::new(cam.eye(), cam.view()), &mut lists);
    if play.scene.is_none() && !assets.collision.water_boxes.is_empty() {
        lists.xlu.push(DrawCmd::new(MeshKey::named(WATER_MESH), Mat4::IDENTITY));
    }
    let mut enc = device.create_command_encoder(&Default::default());
    let mut meshes = Meshes { a: assets, scene: play.scene.as_ref(), col: &play.col };
    r.render_lists(device, queue, &mut enc, target, &lists, &mut scene.cache, &mut meshes, &cam, &light, &lines, clear);
    queue.submit([enc.finish()]);
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
    world: PlayState,
    pads: Option<Pads>,
    pad_error: Option<String>,
    target: ViewportTarget,
    last: Instant,
    show_wire: bool,
    show_hud: bool,
    fps: f32,
    last_pad: PadState,
    /// The audio library through the output device, and what the HUD says about it. Held:
    /// dropping it stops the sound. After each game frame the play state's `GameOp`s go to it,
    /// and it gives back what the game reads (docs/adr/0026-the-games-audio.md).
    audio: Option<eng_audio::output::AudioOutput>,
    audio_status: String,
    /// When a custom level's file was last checked for a rebuild.
    level_check: Instant,
}

impl App {
    fn new(cc: &eframe::CreationContext, assets: Assets, child: bool) -> Result<App> {
        let (device, queue, rs) = eng_app::wgpu_state(cc)?;
        let renderer = Renderer::new(&device, &queue);
        let scene = SceneGfx::new(&assets);
        let world = new_play(&assets, child);
        let (pads, pad_error) = match Pads::new(PadConfig::find(&std::env::current_dir()?, "oot.toml")) {
            Ok(p) => (Some(p), None),
            Err(e) => (None, Some(format!("{e:#}"))),
        };
        let (audio, audio_status) = if assets.audio {
            match assets.pack.audio_data().and_then(eng_audio::output::AudioOutput::start) {
                Ok(out) => {
                    let status = match assets.music {
                        Some(m) if world.assets.is_none() => {
                            // The spikes' view: no Play_Init to play anything.
                            out.start_sequence(0, m);
                            format!("audio: {} (sequence {m})", out.description)
                        }
                        Some(m) => format!("audio: {} (sequence {m} forced)", out.description),
                        None => format!("audio: {}", out.description),
                    };
                    (Some(out), status)
                }
                Err(e) => {
                    log::warn!("no audio: {e:#}");
                    (None, format!("no audio: {e:#}"))
                }
            }
        } else {
            (None, "audio off".to_string())
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
            target: ViewportTarget::default(),
            last: Instant::now(),
            show_wire: false,
            show_hud: true,
            fps: 0.0,
            last_pad: PadState::default(),
            audio,
            audio_status,
            level_check: Instant::now(),
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
        let kb = eng_app::keyboard(&ctx);
        let pad = merge(self.pads.as_ref().and_then(|p| p.state()), &kb);
        // Start (Enter) is read by the play frame: KaleidoSetup_Update runs the pause menu's
        // equipping stand-in (PlayState::pause_menu_equip).
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
                self.world.debug.foot_ik = !self.world.debug.foot_ik;
                self.assets.foot_ik = self.world.debug.foot_ik;
            }
            if i.key_pressed(egui::Key::Backspace) {
                if self.world.assets.is_some() {
                    // Play_TriggerVoidOut: the fade, then Play_Init at the respawn point.
                    self.world.trigger_void_out();
                } else {
                    self.world.respawn();
                }
            }
        });
        if ctx.input(|i| i.key_pressed(egui::Key::P)) {
            self.world.debug.placeholders = !self.world.debug.placeholders;
            self.assets.placeholders = self.world.debug.placeholders;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Tab)) {
            let child = self.world.player().adult;
            // The scene layer depends on Link's age.
            if self.assets.scene.is_some() && self.assets.entrance.is_none() {
                match load_scene(&mut self.assets, child) {
                    Ok(()) => self.scene = SceneGfx::new(&self.assets),
                    Err(e) => log::error!("reloading the scene: {e:#}"),
                }
            }
            self.world = new_play(&self.assets, child);
        }
        // A custom level that was rebuilt (the overworld editor exports every edit): reloaded,
        // Link staying where he is.
        if let Some((dir, stamp)) = self.assets.level.clone()
            && self.level_check.elapsed().as_secs_f32() >= 0.5
        {
            self.level_check = Instant::now();
            if level_stamp(&dir) != stamp {
                let p = self.world.player();
                let (child, pos, yaw) = (!p.adult, p.actor.world_pos, p.actor.shape_rot.y);
                match load_level(&mut self.assets, &dir, child) {
                    Ok(()) => {
                        self.scene = SceneGfx::new(&self.assets);
                        self.world = new_play_at(&self.assets, child, pos, yaw, false);
                    }
                    Err(e) => {
                        log::error!("reloading the level: {e:#}");
                        if let Some(l) = &mut self.assets.level {
                            l.1 = level_stamp(&dir);
                        }
                    }
                }
            }
        }
        // Each game frame's hand-over to the audio side: its ops, and the view back.
        let audio = &self.audio;
        self.world.advance_with(dt, |a| match audio {
            Some(out) => {
                out.send_ops(a.take_ops());
                a.set_view(out.take_view());
            }
            None => {
                a.take_ops();
            }
        });

        let frame = self.world.render_frame();
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui, |ui| {
            let avail = ui.available_size();
            let ppp = ui.ctx().pixels_per_point();
            let (w, h) = (((avail.x * ppp) as u32).max(1), ((avail.y * ppp) as u32).max(1));
            let id = self.target.ensure(&self.device, &self.render_state, w, h);
            let target = self.target.target();
            draw_frame(&mut self.renderer, &self.device, &self.queue, target, &mut self.assets, &mut self.scene, &self.world, &frame, self.show_wire);
            // The fills are in the frame (PlayState::draw_fills), under the HUD and messages.
            let rect = ui.add(egui::Image::new((id, avail))).rect;

            if self.show_hud {
                let p = self.world.player();
                let dev = match (&self.pads, &self.pad_error) {
                    (Some(pads), _) => pads.active_name().map(|n| format!("{n} [{}]", pads.profile().name)).unwrap_or("no pad (keyboard)".into()),
                    (None, Some(e)) => format!("pad backend error: {e}"),
                    _ => "keyboard".into(),
                };
                let rooms_line = match &self.world.scene {
                    Some(s) => format!(
                        "{} layer {}: room {} (prev {}), {} actors ({} placeholders, P shows them), entries drawn {}, frame {}, fog near {} far {}\n",
                        s.short_name(),
                        s.layer,
                        self.world.room_ctx.cur.num,
                        self.world.room_ctx.prev.num,
                        self.world.actors.total(),
                        self.world.actors.all().into_iter().filter(|&h| self.world.actors.get(h).is_some_and(|a| a.as_any().is::<oot_game::spawn::Placeholder>())).count(),
                        self.scene.drawn_entries,
                        self.world.gameplay_frames,
                        s.lights.fog_near,
                        s.lights.fog_far
                    ),
                    _ => String::new(),
                };
                let text = format!(
                    "{place} | {age} Link | {fps:.0} fps, logic {hz} Hz | camera {cam:?} (F3)\n{rooms_line}\
                     action {act:?} ({dec})  anim {anim} @{frame:.1}\n\
                     pos ({x:.1}, {y:.1}, {z:.1})  speed {spd:.2}  vy {vy:.2}  {ground}\n\
                     input: {dev}\n  stick ({sx:+}, {sy:+}) [{btns}]  raw buttons held {raw:?}\n{audio}\n\
                     WASD/arrows stick, Shift walk, Space A, E B (sword), Q Z-target, J/L C-left/right, F1 collision, F3 camera, F4 foot IK, P placeholders, Tab age, Backspace respawn/void out",
                    place = self.world.scene.as_ref().map(|s| s.short_name().to_string()).unwrap_or_else(|| self.assets.place.clone()),
                    cam = self.world.camera_kind,
                    age = if p.adult { "adult" } else { "child" },
                    fps = self.fps,
                    hz = eng_math::GAME_HZ,
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
                    audio = self.audio_status,
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

/// Opens the play window.
pub fn run_window(opts: &Options, title: &str, width: u32, height: u32) -> Result<()> {
    let assets = load_assets(opts)?;
    let child = opts.child;
    eng_app::run_window("oot", title, [width as f32, height as f32], move |cc| App::new(cc, assets, child))
}
