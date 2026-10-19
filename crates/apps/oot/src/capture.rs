//! `oot capture-actors`: the game editor's captures of what each actor draws
//! (`oot_game::actor_capture`, docs/EDITOR-ACTORS.md "Models in the views", ADR 0078).
//!
//! For every placeable actor, in params 0 and in each configuration the game's scenes place it
//! with (the most placed `VARIANTS`), and in the configurations the editor asks for besides
//! (`EXTRA`): a fresh play state in a flat custom level entered by `Play_Init`, the objects the
//! configuration needs loaded, Link a little way off facing it, the actor spawned at the origin and
//! run for some frames. Then its own draw code (and its children's) is called once more, and what it
//! submits is resolved as the renderer resolves it (`Meshes`) and flattened into one still mesh in
//! the actor's space (`eng_gfx::flatten`).
//!
//! An actor that draws nothing as it's first placed in the game (its age and time of day) is tried
//! again as if in that placement's scene (`play->sceneId`: 46 actors branch on it), then as the other
//! age, then at the other time of day. What came of each is in the manifest.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use eng_gfx::{DrawCmd, DrawList, DrawLists, MeshKey};
use eng_input::pad::PadState;
use glam::{Mat4, Vec3};
use oot_actors::PlayExt;
use oot_game::actor_capture::{self as ac, ActorCaptures, Capture, CaptureKey, CaptureLight, Manifest, ManifestEntry, Outcome, Stamp};
use oot_game::play::{PlayState, ViewInfo, scripted_input};

use crate::{Assets, Meshes, Options};

/// The frame a capture is taken on (after the spawn), if the actor draws then; else the first one
/// it draws on, up to `LAST_FRAME`.
const CAPTURE_FRAME: u32 = 10;
const LAST_FRAME: u32 = 40;

/// Where Link stands: in front of the actor (yaw 0 faces +z), facing it.
const LINK_AT: Vec3 = Vec3::new(0.0, 0.0, 260.0);
const LINK_YAW: i16 = -0x8000;

/// Actors that aren't offered in the editor (Link is the level's start; the sandbox's stand-in).
const NOT_PLACED: &[&str] = &["Player", "Sandbox_Dummy_Target"];

/// Spawners that draw nothing themselves: what they spawn is their look (a ring of bushes or
/// rocks, fish and bugs). Every other actor must draw something itself to be captured, its
/// children with it (a Kokiri child's fairy).
const DRAWN_BY_CHILDREN: &[&str] = &["Obj_Mure", "Obj_Mure2"];

/// What to capture: the out folder, the source tree (for the modules' stamps), and whether to
/// make every capture again.
pub struct CaptureOptions {
    pub out: PathBuf,
    pub root: PathBuf,
    pub all: bool,
    /// Only these actors (overlay names), for a quick look.
    pub only: Vec<String>,
    /// Also draw each capture to `png/` (and every one on a contact sheet, `png/sheet.png`), with
    /// the game's renderer, for a look without the editor.
    pub pngs: bool,
}

/// Where an actor's configuration is first placed in the game: the scene, its id, and the
/// layer's age and time.
#[derive(Clone, Debug)]
struct Placement {
    scene: String,
    scene_id: u16,
    adult: bool,
    night: bool,
}

/// One configuration to capture, with where the game first places it (or any of the actor's
/// configurations, for one it doesn't place).
struct Request {
    key: CaptureKey,
    first: Option<Placement>,
}

/// The game layers' age and time (`Scene_SetupLayer`'s: child day, child night, adult day, adult
/// night; the cutscene layers count as child by day).
fn layer_setting(layer: usize) -> (bool, bool) {
    match layer {
        1 => (false, true),
        2 => (true, false),
        3 => (true, true),
        _ => (false, false),
    }
}

/// Every configuration to capture: each placeable actor's params 0 and its most placed
/// configurations, and the extra ones asked for.
fn requests(a: &Assets, extra: &[CaptureKey]) -> Result<Vec<Request>> {
    let pack = &a.pack;
    let at = pack.actor_table()?;
    let st = pack.scene_table()?;
    let schemas: HashMap<&str, &oot_game::actor_schema::ActorSchema> = oot_actors::PROFILES.iter().map(|p| (p.name, p.schema)).collect();
    // (actor, params, setting rot) -> (count, first placement)
    let mut placed: BTreeMap<String, Vec<(u16, [i16; 3], usize, Placement)>> = BTreeMap::new();
    let mut seen = HashSet::new();
    for e in &st.scenes {
        let Ok(sd) = pack.scene(&e.file) else { continue };
        for (li, ld) in sd.layers.iter().enumerate() {
            let (adult, night) = layer_setting(li);
            let here = Placement { scene: e.file.clone(), scene_id: e.id, adult, night };
            let mut add = |id: i16, params: i16, rot: [i16; 3]| {
                let name = at.name(id);
                let Some(s) = schemas.get(name.as_str()) else { return };
                let rot = s.setting_rot(rot);
                let vs = placed.entry(name).or_default();
                match vs.iter_mut().find(|v| v.0 == params as u16 && v.1 == rot) {
                    Some(v) => v.2 += 1,
                    None => vs.push((params as u16, rot, 1, here.clone())),
                }
            };
            for s in &ld.spawns {
                add(s.id, s.params, s.rot);
            }
            for t in &ld.transition_actors {
                add(t.id, t.params, [0, t.rot_y, 0]);
            }
            for key in &ld.rooms {
                if !seen.insert(key.clone()) {
                    continue;
                }
                let Ok(r) = pack.room(key) else { continue };
                for x in &r.actors {
                    add(x.id, x.params, x.rot);
                }
            }
        }
    }
    let mut out = Vec::new();
    let mut have = HashSet::new();
    for p in oot_actors::PROFILES.iter().filter(|p| !NOT_PLACED.contains(&p.name)) {
        let mut vs = placed.remove(p.name).unwrap_or_default();
        vs.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
        let any = vs.first().map(|v| v.3.clone());
        for v in vs.iter().take(ac::VARIANTS) {
            let key = CaptureKey { actor: p.name.to_string(), params: v.0, rot: v.1 };
            have.insert(key.clone());
            out.push(Request { key, first: Some(v.3.clone()) });
        }
        let blank = CaptureKey { actor: p.name.to_string(), params: 0, rot: [0; 3] };
        if have.insert(blank.clone()) {
            out.push(Request { key: blank, first: any.clone() });
        }
    }
    for k in extra {
        if oot_actors::PROFILES.iter().any(|p| p.name == k.actor) && have.insert(k.clone()) {
            let first = out.iter().find(|r| r.key.actor == k.actor).and_then(|r| r.first.clone());
            out.push(Request { key: k.clone(), first });
        }
    }
    Ok(out)
}

/// A flat floor 6000 across at height 0, in Kokiri Forest's look (`oot_import::level`'s format).
fn write_floor(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir.join("textures"))?;
    image::RgbaImage::from_pixel(2, 2, image::Rgba([120, 160, 90, 255])).save(dir.join("textures/g.png"))?;
    let j = serde_json::json!({
        "name": "actor_captures",
        "scene": "spot04",
        "materials": [{"name": "ground", "file": "textures/g.png", "wrap_u": "repeat", "wrap_v": "repeat", "alpha": "opaque", "opacity": 1.0, "cull": "back"}],
        "surfaces": ["ground"],
        "objects": [{"name": "ground", "verts": [-3000, -3000, 0, 3000, -3000, 0, 3000, 3000, 0, -3000, 3000, 0], "tris": [0, 1, 2, 0, 2, 3],
            "uvs": [0, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1], "colors": [], "material": [0, 0], "surface": [0, 0]}],
        "game": {"music": "none"}
    });
    std::fs::write(dir.join("level.json"), j.to_string())?;
    Ok(())
}

/// An actor's code that panics while it's being captured is quiet: its message goes in the
/// manifest. Panics anywhere else go to the hook there was.
fn quiet_panics() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let before = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if !CAPTURING.with(|c| c.get()) {
                return before(info);
            }
            let msg = info.payload().downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| info.payload().downcast_ref::<String>().cloned()).unwrap_or_default();
            let at = info.location().map(|l| format!(" ({}:{})", l.file(), l.line())).unwrap_or_default();
            PANIC.with(|p| *p.borrow_mut() = Some(format!("panicked: {msg}{at}")));
        }));
    });
}

/// The message of the last panic on this thread, for the manifest.
fn take_panic() -> String {
    PANIC.with(|p| p.borrow_mut().take()).unwrap_or_else(|| "its code stopped".into())
}

thread_local! {
    static PANIC: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
    static CAPTURING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// What one try made.
enum Try {
    Drawn { cmds: Vec<DrawCmd>, frame: u32 },
    Nothing(String),
    Failed(String),
}

fn idle(w: &mut PlayState) {
    w.tick_with(scripted_input(PadState::default(), PadState::default()));
}

/// One try at a configuration in a setting: a fresh play state, the objects loaded, the actor
/// spawned and run, and its draws (and its children's) on the frame it's taken.
fn try_capture(a: &mut Assets, key: &CaptureKey, adult: bool, night: bool, scene_id: Option<u16>) -> Try {
    a.day_time = crate::parse_time(if night { "22:00" } else { "10:00" }).unwrap_or(0x8000);
    let mut w = crate::new_play(a, !adult);
    if w.assets.is_none() {
        return Try::Failed("Play_Init failed".into());
    }
    let g = w.assets.clone().unwrap();
    let Some(profile) = oot_actors::PROFILES.iter().find(|p| p.name == key.actor) else { return Try::Failed("not ported".into()) };
    idle(&mut w);
    if let Some(id) = scene_id {
        w.scene_id = id;
    }
    // its row of the actor table, by name (`Boss_Dodongo`'s profile says ACTOR_EN_DODONGO: the id
    // the actor gets, not the row it's spawned from)
    let Some(id) = g.actors.actors.iter().position(|t| t.name == key.actor).map(|i| i as i16) else { return Try::Failed("not in the actor table".into()) };
    // its objects: the table's and what its schema needs for these params
    let mut objects: Vec<i16> = g.actors.get(id).and_then(|t| t.init.as_ref()).map(|i| i.object_id).into_iter().collect();
    objects.extend(profile.schema.needed_objects(key.params as i16, key.rot).into_iter().filter_map(|o| g.scenes.object_id(o)));
    for o in objects {
        if w.object_ctx.get_index(o).is_none() {
            w.object_ctx.spawn(o);
        }
    }
    w.place_player(LINK_AT, LINK_YAW);
    idle(&mut w);
    let before: HashSet<_> = w.actors.all().into_iter().collect();
    let h = match w.actor_spawn(id, Vec3::ZERO, key.rot, key.params as i16) {
        Ok(h) => h,
        Err(e) => return Try::Failed(format!("wouldn't spawn: {e:?}")),
    };
    let mut first = None;
    for frame in 1..=LAST_FRAME {
        idle(&mut w);
        if w.actors.get(h).is_none_or(|x| x.base().killed) {
            return Try::Nothing(format!("gone by frame {frame} (killed itself)"));
        }
        let cmds = draws(&w, h, &before);
        if !cmds.is_empty() && (frame >= CAPTURE_FRAME || first.is_none()) {
            first = Some((cmds, frame));
            if frame >= CAPTURE_FRAME {
                break;
            }
        }
    }
    match first {
        Some((cmds, frame)) => Try::Drawn { cmds, frame },
        None => Try::Nothing("drew nothing".into()),
    }
}

/// What the actor `me` and the actors spawned with it since `before` draw now, OPA then XLU:
/// each one's own draw code from its state now (the frame's, if `Actor_DrawAll` drew it). Empty
/// if `me` draws nothing itself (a Kokiri child hidden, its fairy still out).
fn draws(w: &PlayState, me: oot_game::actor_ctx::ActorHandle, before: &HashSet<oot_game::actor_ctx::ActorHandle>) -> Vec<DrawCmd> {
    let frame = w.current_frame();
    let view = ViewInfo::new(frame.view.eye, glam::camera::rh::view::look_at_mat4(frame.view.eye, frame.view.at, frame.view.up));
    let mut out = DrawLists::default();
    let mut handles: Vec<_> = w.actors.all().into_iter().filter(|h| !before.contains(h) && *h != me).collect();
    handles.insert(0, me);
    for h in handles {
        let Some(x) = w.actors.get(h) else { continue };
        if x.base().killed {
            continue;
        }
        let rs = frame.actors.iter().find(|f| f.0 == h).map(|f| f.1.clone()).unwrap_or_else(|| x.render_state());
        x.draw(&rs, w, &view, &mut out);
        if h == me && out.opa.is_empty() && out.xlu.is_empty() && !DRAWN_BY_CHILDREN.contains(&x.name()) {
            return Vec::new();
        }
    }
    out.opa.into_iter().chain(out.xlu).collect()
}

/// A configuration's tries, in order: as first placed; as if in that scene; the other age; the
/// other time of day.
fn settings(first: Option<&Placement>) -> Vec<(bool, bool, Option<u16>, String)> {
    let (adult, night) = first.map_or((false, false), |p| (p.adult, p.night));
    let age = |a: bool| if a { "adult" } else { "child" };
    let time = |n: bool| if n { "night" } else { "day" };
    let mut v = vec![(adult, night, None, format!("{}, {}", age(adult), time(night)))];
    if let Some(p) = first {
        let s = Some(p.scene_id);
        let at = format!("as in {}", p.scene.trim_end_matches("_scene"));
        v.push((adult, night, s, format!("{at}, {}, {}", age(adult), time(night))));
        v.push((!adult, night, s, format!("{at}, {}, {}", age(!adult), time(night))));
        v.push((adult, !night, s, format!("{at}, {}, {}", age(adult), time(!night))));
    } else {
        v.push((!adult, night, None, format!("{}, {}", age(!adult), time(night))));
    }
    v
}

/// The draws resolved and flattened into the actor's space.
fn flatten(a: &mut Assets, cache: &mut HashMap<MeshKey, Option<DrawList>>, cmds: &[DrawCmd], yaw: i16) -> (DrawList, Vec<String>) {
    let col = eng_collision::bgcheck::CollisionContext::new(a.collision.clone());
    let mut missing = Vec::new();
    for c in cmds {
        if !cache.contains_key(&c.mesh) {
            let m = eng_render::MeshSource::mesh(&mut Meshes { a, scene: None, col: &col }, &c.mesh);
            cache.insert(c.mesh.clone(), m);
        }
    }
    let draws: Vec<(&DrawCmd, &DrawList)> = cmds
        .iter()
        .filter_map(|c| match cache.get(&c.mesh).and_then(|m| m.as_ref()) {
            Some(m) => Some((c, m)),
            None => {
                missing.push(c.mesh.name.clone());
                None
            }
        })
        .collect();
    let root = Mat4::from_rotation_y(eng_math::binang_to_rad(yaw)).inverse();
    (eng_gfx::flatten(draws, root), missing)
}

/// Captures every configuration whose actor's file is missing or stale (or all of them), and
/// writes the files and the manifest.
pub fn capture_actors(o: &CaptureOptions) -> Result<()> {
    let t0 = Instant::now();
    std::fs::create_dir_all(&o.out)?;
    let level = o.out.join("level");
    write_floor(&level)?;
    quiet_panics();
    let opts = Options { level: Some(level.clone()), level_in_game: true, time: "10:00".into(), child: true, ..Default::default() };
    let mut a = crate::load_assets(&opts)?;
    let pack = ac::pack_id(a.pack.header());
    let game = std::env::current_exe().map_or(0, |p| ac::file_stamp(&p));
    let extra: Vec<CaptureKey> = std::fs::read(o.out.join(ac::EXTRA)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    let reqs = requests(&a, &extra)?;
    let old = Manifest::read(&o.out).filter(|m| m.pack == pack);
    let mut by_actor: BTreeMap<String, Vec<Request>> = BTreeMap::new();
    for r in reqs {
        if o.only.is_empty() || o.only.contains(&r.key.actor) {
            by_actor.entry(r.key.actor.clone()).or_default().push(r);
        }
    }
    let total: usize = by_actor.values().map(Vec::len).sum();
    println!("capturing: {} actors, {total} configurations, into {}", by_actor.len(), o.out.display());
    let mut entries: Vec<ManifestEntry> = Vec::new();
    // the lights of the setting (child, day): what the editor lights the captures with
    let light = {
        let mut w = crate::new_play(&a, true);
        idle(&mut w);
        let ls = &w.env_ctx.light_settings;
        CaptureLight { ambient: ls.ambient, dirs: [ls.light1_dir, ls.light2_dir], colors: [ls.light1_color, ls.light2_color] }
    };
    let mut cache: HashMap<MeshKey, Option<DrawList>> = HashMap::new();
    let (mut done, mut made, mut kept) = (0usize, 0usize, 0usize);
    for (actor, reqs) in &by_actor {
        let stamp = Stamp { format: ac::FORMAT, pack: pack.clone(), game, module: ac::module_stamp(&o.root, actor) };
        let file = ActorCaptures::read(&o.out, actor).ok().filter(|f| f.stamp == stamp && !o.all);
        let mut caps = ActorCaptures { actor: actor.clone(), stamp: stamp.clone(), captures: Vec::new() };
        let mut changed = file.is_none();
        for r in reqs {
            done += 1;
            let k = &r.key;
            // kept: in an up-to-date file, or tried before under this stamp with nothing to keep
            let before = old.as_ref().and_then(|m| m.entries.iter().find(|e| e.actor == k.actor && e.params == k.params && e.rot == k.rot));
            if let Some(f) = &file {
                if let Some(c) = f.get(k.params, k.rot) {
                    caps.captures.push(c.clone());
                    if let Some(e) = before {
                        entries.push(e.clone());
                    }
                    kept += 1;
                    continue;
                }
                if let Some(e) = before.filter(|e| e.outcome != Outcome::Captured) {
                    entries.push(e.clone());
                    kept += 1;
                    continue;
                }
            }
            changed = true;
            let mut entry = ManifestEntry { actor: k.actor.clone(), params: k.params, rot: k.rot, outcome: Outcome::Nothing, note: String::new(), frame: 0, triangles: 0, size: [0.0; 3] };
            let mut notes = Vec::new();
            for (adult, night, scene, what) in settings(r.first.as_ref()) {
                CAPTURING.with(|c| c.set(true));
                let t = std::panic::catch_unwind(AssertUnwindSafe(|| try_capture(&mut a, k, adult, night, scene))).unwrap_or_else(|_| Try::Failed(take_panic()));
                CAPTURING.with(|c| c.set(false));
                match t {
                    Try::Drawn { cmds, frame } => {
                        let (mesh, missing) = flatten(&mut a, &mut cache, &cmds, k.rot[1]);
                        let Some((lo, hi)) = eng_gfx::bounds(&mesh) else {
                            notes.push(format!("{what}: no mesh for {}", missing.join(", ")));
                            continue;
                        };
                        entry.outcome = Outcome::Captured;
                        entry.frame = frame;
                        entry.triangles = mesh.triangle_count();
                        entry.size = (hi - lo).to_array().map(|v| v.round());
                        entry.note = what;
                        if !missing.is_empty() {
                            entry.note += &format!(" (no mesh for {})", missing.join(", "));
                        }
                        caps.captures.push(Capture { params: k.params, rot: k.rot, mesh, lo: lo.to_array(), hi: hi.to_array() });
                        break;
                    }
                    Try::Nothing(why) => notes.push(format!("{what}: {why}")),
                    Try::Failed(why) => {
                        entry.outcome = Outcome::Failed;
                        notes.push(format!("{what}: {why}"));
                    }
                }
            }
            if entry.outcome != Outcome::Captured {
                // failed only if no try ran to the end
                if notes.iter().any(|n| !n.contains("panicked") && !n.contains("wouldn't spawn") && !n.contains("Play_Init")) {
                    entry.outcome = Outcome::Nothing;
                }
                entry.note = notes.join("; ");
            }
            made += 1;
            entries.push(entry);
            if done % 25 == 0 {
                println!("progress {done}/{total}");
            }
        }
        if changed {
            caps.write(&o.out).with_context(|| format!("writing {actor}'s captures"))?;
        }
    }
    // the actors not asked for this time keep their entries
    if let Some(m) = &old {
        for e in &m.entries {
            if !by_actor.contains_key(&e.actor) {
                entries.push(e.clone());
            }
        }
    }
    entries.sort_by(|a, b| (&a.actor, a.params, a.rot).cmp(&(&b.actor, b.params, b.rot)));
    let n = |o: Outcome| entries.iter().filter(|e| e.outcome == o).count();
    println!(
        "captured {}, nothing drawn {}, failed {} ({made} tried now, {kept} kept) in {:.1} s",
        n(Outcome::Captured),
        n(Outcome::Nothing),
        n(Outcome::Failed),
        t0.elapsed().as_secs_f64()
    );
    let m = Manifest { format: ac::FORMAT, pack, light, entries };
    m.write(&o.out)?;
    println!("progress {total}/{total}");
    if o.pngs {
        draw_pngs(&o.out, &m, &o.only)?;
    }
    Ok(())
}

/// The lights the captures were made under, as the renderer takes them (no fog).
pub fn lighting(l: &CaptureLight) -> eng_render::Lighting {
    let c = |v: [u8; 3]| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32) / 255.0;
    let d = |v: [i8; 3]| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32);
    eng_render::Lighting { dir: d(l.dirs[0]), color: c(l.colors[0]), dir2: d(l.dirs[1]), color2: c(l.colors[1]), ambient: c(l.ambient), fog: None }
}

/// Each capture drawn by the game's renderer from the front and a little to the side, into
/// `png/<actor>_<params>_<rot>.png`, and all of them on `png/sheet.png` (12 across).
fn draw_pngs(dir: &Path, m: &Manifest, only: &[String]) -> Result<()> {
    const SIZE: u32 = 128;
    let (device, queue) = eng_render::headless_device()?;
    let mut r = eng_render::Renderer::new(&device, &queue);
    let target = eng_render::Target::new(&device, SIZE, SIZE);
    let light = lighting(&m.light);
    let out = dir.join("png");
    std::fs::create_dir_all(&out)?;
    let mut tiles = Vec::new();
    let mut actors: Vec<&str> = m.entries.iter().filter(|e| e.outcome == Outcome::Captured).map(|e| e.actor.as_str()).collect();
    actors.dedup();
    for actor in actors.into_iter().filter(|a| only.is_empty() || only.iter().any(|o| o == a)) {
        let Ok(f) = ActorCaptures::read(dir, actor) else { continue };
        for c in &f.captures {
            let mut model = r.upload(&device, &queue, &c.mesh);
            model.pose(&[], Mat4::IDENTITY);
            let cam = eng_render::Camera::framing(Vec3::from(c.lo), Vec3::from(c.hi), 0.6, 0.35, 30f32.to_radians());
            let mut enc = device.create_command_encoder(&Default::default());
            r.render(&device, &queue, &mut enc, &target, &[&model], &cam, &light, &[], &[], [0.22, 0.24, 0.27, 1.0]);
            queue.submit([enc.finish()]);
            let px = target.read_rgba(&device, &queue)?;
            let img = image::RgbaImage::from_raw(SIZE, SIZE, px).context("the image")?;
            img.save(out.join(format!("{actor}_{:04x}_{}_{}_{}.png", c.params, c.rot[0], c.rot[1], c.rot[2])))?;
            tiles.push(img);
        }
    }
    let across = 12u32;
    let rows = (tiles.len() as u32).div_ceil(across).max(1);
    let mut sheet = image::RgbaImage::from_pixel(across * SIZE, rows * SIZE, image::Rgba([0, 0, 0, 255]));
    for (i, t) in tiles.iter().enumerate() {
        image::imageops::overlay(&mut sheet, t, ((i as u32 % across) * SIZE) as i64, ((i as u32 / across) * SIZE) as i64);
    }
    sheet.save(out.join("sheet.png"))?;
    println!("drew {} captures into {}", tiles.len(), out.display());
    Ok(())
}
