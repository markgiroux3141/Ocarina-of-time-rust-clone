//! The actors as the game draws them (docs/EDITOR-ACTORS.md, "Models in the views"; ADR 0078):
//! the captures the game makes of each actor's own draws (`oot capture-actors`,
//! `oot_game::actor_capture`), in `out\editor\actors\`. The editor runs the game built beside it
//! to make them in the background at the start (only stale ones are made again: a new pack or a
//! changed actor module) and whenever the level has an actor in a configuration not captured yet.
//! Read here: the manifest, each actor's file when it's first needed, and each capture's outline
//! from above (for the plan).

use oot_game::actor_capture::{ActorCaptures, Capture, CaptureKey, Manifest, Outcome, EXTRA};
use std::collections::{HashMap, HashSet};
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{mpsc, Arc};

/// A capture found for an actor: its actor's file, the capture's index there, and whether it's
/// another configuration's (the nearest one captured) standing in.
#[derive(Clone)]
pub struct Found {
    pub file: Arc<ActorCaptures>,
    pub index: usize,
    pub exact: bool,
}

impl Found {
    pub fn capture(&self) -> &Capture {
        &self.file.captures[self.index]
    }

    /// The key of the capture drawn.
    pub fn key(&self) -> (String, u16, [i16; 3]) {
        let c = self.capture();
        (self.file.actor.clone(), c.params, c.rot)
    }
}

enum Msg {
    Line(String),
    Done(Result<(), String>),
}

pub struct Captures {
    pub dir: PathBuf,
    pub manifest: Option<Manifest>,
    /// Each actor's file once read (`None`: it has none).
    files: HashMap<String, Option<Arc<ActorCaptures>>>,
    /// Outlines from above by key: in the game's x and z, round the actor's origin.
    hulls: HashMap<(String, u16, [i16; 3]), Arc<Vec<[f32; 2]>>>,
    /// The invisible actors' areas made so far (`area`).
    areas: HashMap<(String, u16, [i16; 3]), Found>,
    running: Option<mpsc::Receiver<Msg>>,
    /// Configurations asked for since the last run started: run again when it ends.
    queued: bool,
    /// What the editor has asked for besides the game's configurations (`EXTRA`).
    extra: HashSet<CaptureKey>,
    /// The last run's progress or outcome, for the status bar.
    pub status: String,
    /// The last run's summary line ("captured 929, nothing drawn 381, ...").
    summary: String,
    /// Bumped whenever the captures are read again: the views drop what they made from them.
    pub generation: u64,
}

/// The game built beside the editor (the scripts' target), else the default release build's.
pub fn game_exe(root: &Path) -> PathBuf {
    let name = if cfg!(windows) { "oot.exe" } else { "oot" };
    let beside = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join(name))).filter(|p| p.exists());
    beside.unwrap_or_else(|| root.join("target/release").join(name))
}

impl Captures {
    pub fn new(root: &Path) -> Captures {
        let dir = root.join("out/editor/actors");
        let extra = std::fs::read(dir.join(EXTRA)).ok().and_then(|b| serde_json::from_slice::<Vec<CaptureKey>>(&b).ok()).unwrap_or_default().into_iter().collect();
        Captures { manifest: Manifest::read(&dir), dir, files: HashMap::new(), hulls: HashMap::new(), areas: HashMap::new(), summary: String::new(), running: None, queued: false, extra, status: String::new(), generation: 0 }
    }

    /// Runs the capture in the background (the game's `capture-actors`), unless it's running: then
    /// once more when it ends.
    pub fn run(&mut self, root: &Path, repaint: impl Fn() + Send + 'static) {
        if self.running.is_some() {
            self.queued = true;
            return;
        }
        let exe = game_exe(root);
        if !exe.exists() {
            self.status = "the actors' models need the game built (cargo build --release -p oot)".into();
            return;
        }
        let mut cmd = Command::new(&exe);
        cmd.args(["capture-actors", "--out"]).arg(&self.dir).arg("--root").arg(root).current_dir(root).stdout(Stdio::piped()).stderr(Stdio::null()).stdin(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // CREATE_NO_WINDOW: no console flashing up
            cmd.creation_flags(0x0800_0000);
        }
        let (tx, rx) = mpsc::channel();
        match cmd.spawn() {
            Ok(mut child) => {
                let out = child.stdout.take();
                std::thread::spawn(move || {
                    if let Some(out) = out {
                        for line in std::io::BufReader::new(out).lines().map_while(Result::ok) {
                            let _ = tx.send(Msg::Line(line));
                            repaint();
                        }
                    }
                    let r = match child.wait() {
                        Ok(s) if s.success() => Ok(()),
                        Ok(s) => Err(format!("the capture stopped ({s})")),
                        Err(e) => Err(e.to_string()),
                    };
                    let _ = tx.send(Msg::Done(r));
                    repaint();
                });
                self.running = Some(rx);
                self.status = "capturing the actors' models…".into();
            }
            Err(e) => self.status = format!("the actors' models: {e}"),
        }
    }

    pub fn busy(&self) -> bool {
        self.running.is_some()
    }

    /// Takes in the run's progress; when it ends, reads the captures again. True when they were.
    pub fn poll(&mut self, root: &Path, repaint: impl Fn() + Send + Clone + 'static) -> bool {
        let Some(rx) = &self.running else { return false };
        let mut done = None;
        while let Ok(m) = rx.try_recv() {
            match m {
                Msg::Line(l) => {
                    if let Some(p) = l.strip_prefix("progress ") {
                        self.status = format!("capturing the actors' models: {p}");
                    } else if l.starts_with("captured ") {
                        self.summary = format!("actors' models: {l}");
                    }
                }
                Msg::Done(r) => done = Some(r),
            }
        }
        let Some(r) = done else { return false };
        self.running = None;
        self.status = match r {
            Ok(()) => std::mem::take(&mut self.summary),
            Err(e) => e,
        };
        self.reload();
        if std::mem::take(&mut self.queued) {
            self.run(root, repaint);
        }
        true
    }

    /// Reads the manifest again and forgets the files read.
    pub fn reload(&mut self) {
        self.manifest = Manifest::read(&self.dir);
        self.files.clear();
        self.hulls.clear();
        self.generation += 1;
    }

    fn file(&mut self, actor: &str) -> Option<Arc<ActorCaptures>> {
        if !self.files.contains_key(actor) {
            let f = ActorCaptures::read(&self.dir, actor).ok().map(Arc::new);
            self.files.insert(actor.to_string(), f);
        }
        self.files[actor].clone()
    }

    /// The capture for an actor in this configuration; else its nearest captured one (the same
    /// params with another setting rotation, else the params differing in the fewest bits).
    pub fn find(&mut self, actor: &str, params: u16, rot: [i16; 3]) -> Option<Found> {
        let file = self.file(actor)?;
        let score = |c: &Capture| {
            if c.params == params && c.rot == rot {
                0
            } else if c.params == params {
                1
            } else {
                2 + (c.params ^ params).count_ones() + c.rot.iter().zip(rot).map(|(a, b)| (*a as u16 ^ b as u16).count_ones()).sum::<u32>()
            }
        };
        let (index, s) = file.captures.iter().enumerate().map(|(i, c)| (i, score(c))).min_by_key(|x| x.1)?;
        Some(Found { file, index, exact: s == 0 })
    }

    /// What the manifest says of a configuration.
    pub fn outcome(&self, actor: &str, params: u16, rot: [i16; 3]) -> Option<(Outcome, &str)> {
        let m = self.manifest.as_ref()?;
        m.entries.iter().find(|e| e.actor == actor && e.params == params && e.rot == rot).map(|e| (e.outcome, e.note.as_str()))
    }

    /// Asks for configurations the manifest hasn't tried (the level's own); runs the capture if
    /// any is new.
    pub fn want(&mut self, keys: impl IntoIterator<Item = CaptureKey>, root: &Path, repaint: impl Fn() + Send + Clone + 'static) {
        let Some(m) = &self.manifest else { return };
        let tried: HashSet<(&str, u16, [i16; 3])> = m.entries.iter().map(|e| (e.actor.as_str(), e.params, e.rot)).collect();
        let new: Vec<CaptureKey> = keys.into_iter().filter(|k| !tried.contains(&(k.actor.as_str(), k.params, k.rot)) && !self.extra.contains(k)).collect();
        if new.is_empty() {
            return;
        }
        self.extra.extend(new);
        let mut list: Vec<&CaptureKey> = self.extra.iter().collect();
        list.sort();
        if let Ok(j) = serde_json::to_string_pretty(&list) {
            let _ = std::fs::create_dir_all(&self.dir);
            let _ = std::fs::write(self.dir.join(EXTRA), j);
        }
        self.run(root, repaint);
    }

    /// An invisible actor's area as a translucent mesh in its colour, standing in for a capture.
    pub fn area(&mut self, actor: &str, params: u16, rot: [i16; 3], area: crate::actors::Area, colour: [u8; 3]) -> Found {
        let key = format!("area:{actor}");
        let id = (key.clone(), params, rot);
        if let Some(f) = self.areas.get(&id) {
            return f.clone();
        }
        let mesh = area_mesh(area, colour);
        let (lo, hi) = eng_gfx::bounds(&mesh).unwrap_or_default();
        let file = Arc::new(ActorCaptures { actor: key, stamp: Default::default(), captures: vec![Capture { params, rot, mesh, lo: lo.to_array(), hi: hi.to_array() }] });
        let f = Found { file, index: 0, exact: true };
        self.areas.insert(id, f.clone());
        f
    }

    /// A capture's outline from above: the convex hull of its vertices in the game's x and z.
    pub fn hull(&mut self, f: &Found) -> Arc<Vec<[f32; 2]>> {
        let key = f.key();
        if let Some(h) = self.hulls.get(&key) {
            return h.clone();
        }
        let pts: Vec<[f32; 2]> = f.capture().mesh.batches.iter().flat_map(|b| b.vertices.iter().map(|v| [v.pos.x, v.pos.z])).collect();
        let h = Arc::new(convex_hull(pts));
        self.hulls.insert(key, h.clone());
        h
    }
}

/// An area's mesh in the game's axes (y up): its sides and its top, translucent in `colour`, seen
/// from inside and out.
fn area_mesh(area: crate::actors::Area, colour: [u8; 3]) -> eng_gfx::DrawList {
    use eng_gfx::combiner::{encode, Combiner};
    use eng_gfx::*;
    use glam::{Vec2, Vec3};
    let mut d = DrawList::default();
    // colour and alpha both the vertex's (G_CC_SHADE)
    let shade = [15, 15, 31, 4, 7, 7, 7, 4];
    let m = d.intern_material(Material {
        combiner: Combiner::decode(encode(shade, shade)),
        two_cycle: false,
        prim: [255; 4],
        prim_lod_frac: 0,
        env: [255; 4],
        fog: [0; 4],
        blend_color: [0; 4],
        geometry_mode: G_SHADE | G_SHADING_SMOOTH | G_ZBUFFER,
        othermode_h: 0,
        othermode_l: 0,
        textures: [None, None],
        blend: BlendMode::Translucent,
        cull: CullMode::None,
        depth_test: true,
        depth_write: false,
        decal: false,
        lit: false,
        texgen: false,
        bilinear: false,
        uv_dyn: [None, None],
        env_dyn: None,
        prim_dyn: None,
        fog_blend: false,
    });
    let [r, g, b] = colour;
    let v = |p: Vec3, a: u8| Vertex { bone: NO_BONE, pos: p, normal: Vec3::Y, color: [r, g, b, a], uv: [Vec2::ZERO; 2] };
    // the outline at the foot, anticlockwise from above
    let (ring, h): (Vec<Vec3>, f32) = match area {
        crate::actors::Area::Cylinder { r, h } => ((0..32).map(|i| {
            let t = i as f32 / 32.0 * std::f32::consts::TAU;
            Vec3::new(r * t.cos(), 0.0, r * t.sin())
        }).collect(), h),
        crate::actors::Area::Box { half, h } => (vec![Vec3::new(-half, 0.0, -half), Vec3::new(half, 0.0, -half), Vec3::new(half, 0.0, half), Vec3::new(-half, 0.0, half)], h),
    };
    let up = Vec3::new(0.0, h, 0.0);
    let mut vs = Vec::new();
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        vs.extend([v(a, 70), v(b, 70), v(b + up, 25), v(a, 70), v(b + up, 25), v(a + up, 25)]);
        vs.extend([v(up, 18), v(a + up, 18), v(b + up, 18)]);
    }
    d.batches.push(Batch { material: m, vertices: vs, sources: Vec::new() });
    d
}

/// The convex hull of points, anticlockwise (Andrew's monotone chain).
fn convex_hull(mut p: Vec<[f32; 2]>) -> Vec<[f32; 2]> {
    p.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    p.dedup();
    if p.len() < 3 {
        return p;
    }
    let cross = |o: [f32; 2], a: [f32; 2], b: [f32; 2]| (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
    let mut h: Vec<[f32; 2]> = Vec::with_capacity(p.len() * 2);
    for pass in 0..2 {
        let start = h.len();
        let it: Box<dyn Iterator<Item = &[f32; 2]>> = if pass == 0 { Box::new(p.iter()) } else { Box::new(p.iter().rev()) };
        for &q in it {
            while h.len() >= start + 2 && cross(h[h.len() - 2], h[h.len() - 1], q) <= 0.0 {
                h.pop();
            }
            h.push(q);
        }
        h.pop();
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A configuration's own capture first; else the same params' (another setting rotation);
    /// else the params differing in the fewest bits.
    #[test]
    fn the_nearest_captured_configuration_stands_in() {
        let root = std::env::temp_dir().join(format!("editor_captures_{}", std::process::id()));
        let dir = root.join("out/editor/actors");
        std::fs::create_dir_all(&dir).unwrap();
        let cap = |params: u16, rot: [i16; 3]| Capture { params, rot, mesh: eng_gfx::DrawList::default(), lo: [0.0; 3], hi: [1.0; 3] };
        ActorCaptures { actor: "En_Test".into(), stamp: Default::default(), captures: vec![cap(0x0100, [0; 3]), cap(0x0103, [0; 3]), cap(0x0103, [0, 0, 2])] }.write(&dir).unwrap();
        let mut c = Captures::new(&root);
        let f = c.find("En_Test", 0x0103, [0, 0, 2]).unwrap();
        assert!(f.exact && f.key() == ("En_Test".into(), 0x0103, [0, 0, 2]));
        let f = c.find("En_Test", 0x0103, [0, 0, 5]).unwrap();
        assert!(!f.exact && f.capture().params == 0x0103);
        let f = c.find("En_Test", 0x0107, [0; 3]).unwrap();
        assert_eq!((f.exact, f.capture().params), (false, 0x0103));
        assert!(c.find("En_Other", 0, [0; 3]).is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn hulls_keep_the_outside_points() {
        let h = convex_hull(vec![[0.0, 0.0], [2.0, 0.0], [1.0, 1.0], [2.0, 2.0], [0.0, 2.0], [1.0, 0.5]]);
        assert_eq!(h.len(), 4);
        assert!(!h.contains(&[1.0, 1.0]) && !h.contains(&[1.0, 0.5]));
    }
}
