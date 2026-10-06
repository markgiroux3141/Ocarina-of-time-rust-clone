//! The editor window: a plan view of the level with its outline, regions, paths and props drawn
//! over the built level, tools to draw and edit them, and panels for their properties. Every edit
//! is rebuilt in the background; with live export on, the build is also written out for the game
//! (`oot_sandbox --level`), which reloads it.

use crate::edit::{self, NodeRef, Shapes};
use crate::profile::ProfileView;
use crate::scene::{self, Scene, Shading, View};
use crate::view3d::{View3d, FOV};
use crate::worker::{Done, Job, Msg, Worker};
use eframe::egui::{self, Align2, Color32, FontId, Key, KeyboardShortcut, Modifiers, PointerButton, Pos2, Sense, Shape, Stroke, Vec2};
use overworld::doc::Prop;
use overworld::geom::{dist, dist_to_loop, point_in_poly, P2};
use overworld::openings::Preview;
use overworld::pieces::Kit;
use overworld::props::Ground;
use overworld::terrain::{Brush, Mode, Terrain};
use overworld::textures::Library;
use overworld::{export, Doc, Level, Theme};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::Arc;

/// The repository root (the editor lives in crates/tools/overworld_editor).
pub const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../..");

/// Pick and snap distance, in pixels.
const PICK: f64 = 9.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tool {
    Select,
    Region,
    Path,
    Brush,
    /// Placing kit pieces.
    Prop,
    /// Drawing a line of a kind: "dirt", "fence", "bridge" or "hedge" (a closed shape).
    Line(&'static str),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Sel {
    None,
    Loop(usize),
    Path(usize),
    Node(NodeRef),
    Prop(usize),
    Line(usize),
}

enum Gesture {
    None,
    Nodes(Vec<NodeRef>),
    Pan,
    Prop(PropDrag),
}

/// Dragging a prop in the plan: moving it (the grab point's offset from its origin), turning it by
/// its facing handle, or scaling it by its corner handle (its scale at the start, and the handle's
/// distance from the origin at scale 1).
#[derive(Clone, Copy)]
enum PropDrag {
    Move { i: usize, off: P2 },
    Turn { i: usize },
    Scale { i: usize, start: [f64; 3], d0: f64 },
}

/// A drag in the 3D view.
enum Gesture3 {
    None,
    Orbit,
    /// Raising or sinking the selected loops: each one's height at the start, world units per
    /// pixel, pixels dragged.
    Height { ls: Vec<(usize, f64)>, per_px: f64, dy: f64 },
    /// Painting terrain.
    Paint,
    /// Moving a prop over the plane at its height (the grab point's offset, that height).
    Prop { i: usize, off: P2, z: f64 },
    /// Sliding a wall piece or opening along the walls under the pointer.
    WallProp { i: usize },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Layout {
    Plan,
    Split,
    ThreeD,
}

enum Action {
    PlayAt(P2),
    Insert(P2),
    Delete,
    ToggleSharp,
    Raise(f64),
    Fit,
}

pub struct Options {
    pub doc: Option<PathBuf>,
    pub theme: Option<PathBuf>,
    pub textures: Option<PathBuf>,
    /// The kit props come from (`pieces.json`); by default out/overworld/kit/kokiri.
    pub kit: Option<PathBuf>,
    /// A region or path to select at the start, by name.
    pub select: Option<String>,
}

pub struct App {
    doc: Doc,
    file: Option<PathBuf>,
    saved: Doc,
    /// Undo: `committed` is the last settled document; edits since then become one step.
    committed: Doc,
    undo: Vec<Doc>,
    redo: Vec<Doc>,
    shapes: Shapes,
    shapes_doc: Doc,
    sel: Sel,
    tool: Tool,
    drawing: Vec<P2>,
    gesture: Gesture,
    menu_at: Option<P2>,
    view: View,
    fit_pending: bool,
    theme: Arc<Theme>,
    theme_file: Option<PathBuf>,
    lib: Option<Arc<Library>>,
    worker: Worker,
    sent: Option<(Doc, bool)>,
    built: Option<Doc>,
    level: Option<Arc<Level>>,
    /// The last build's floors, for fitting wall pieces where they'd go (the ghost).
    ground: Option<Arc<Ground>>,
    scene: Option<Arc<Scene>>,
    build_error: Option<String>,
    problems: Vec<String>,
    build_ms: f64,
    /// Plan view textures by name.
    textures: HashMap<String, Option<egui::TextureHandle>>,
    shading: Shading,
    show_nodes: bool,
    out_dir: PathBuf,
    live: bool,
    walker: Option<Child>,
    status: String,
    cursor: Option<P2>,
    title: String,
    profile: ProfileView,
    /// The point on the selected path under the pointer in the profile.
    profile_hover: Option<P2>,
    v3: Option<View3d>,
    layout: Layout,
    gesture3: Gesture3,
    menu3_at: Option<P2>,
    brush: Brush,
    /// Flatten's target: the offset where the stroke started.
    flatten_to: f64,
    /// Loops selected besides `sel` (Shift-click).
    multi: Vec<usize>,
    kit: Option<Arc<Kit>>,
    /// The piece the Prop tool places, and the turn it places it with.
    piece: String,
    place_yaw: f64,
}

impl App {
    pub fn new(cc: &eframe::CreationContext, opts: Options) -> App {
        let v3 = cc.wgpu_render_state.as_ref().map(View3d::new);
        let ctx = cc.egui_ctx.clone();
        let worker = Worker::new(move || ctx.request_repaint());
        let mut status = String::new();
        let (doc, file) = match &opts.doc {
            Some(p) => match load_doc(p) {
                Ok(d) => (d, Some(p.clone())),
                Err(e) => {
                    status = e;
                    (edit::blank_doc(), None)
                }
            },
            None => (edit::blank_doc(), None),
        };
        let theme = match &opts.theme {
            Some(p) => Theme::load(&p.to_string_lossy()).unwrap_or_else(|e| {
                status = e;
                Theme::kokiri()
            }),
            None => Theme::kokiri(),
        };
        // the Kokiri library, made from the extracted scene the first time
        let tex_dir = opts.textures.clone().or_else(|| {
            let d = Path::new(ROOT).join("out/overworld/textures/kokiri");
            let glb = Path::new(ROOT).join("extracted/scenes/overworld/spot04/spot04.glb");
            if !d.join("textures.json").exists() && glb.exists() {
                match overworld::kit::export_kokiri(&glb, &d) {
                    Ok(n) => status = format!("made the Kokiri texture library ({n} textures) in {}", d.display()),
                    Err(e) => status = format!("Kokiri textures: {e}"),
                }
            }
            d.join("textures.json").exists().then_some(d)
        });
        let lib = tex_dir.and_then(|d| match Library::load(&d) {
            Ok(l) => Some(Arc::new(l)),
            Err(e) => {
                status = e;
                None
            }
        });
        // the Kokiri kit, cut from the extracted scene the first time (and again when its
        // manifest changes)
        let kit_dir = opts.kit.clone().unwrap_or(Path::new(ROOT).join("out/overworld/kit/kokiri"));
        if opts.kit.is_none() {
            let manifest = overworld::pieces::kokiri_manifest();
            let modified = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
            let stale = modified(&kit_dir.join("pieces.json")).is_none_or(|k| modified(manifest).is_some_and(|m| m > k));
            if stale && Path::new(ROOT).join("extracted/scenes/overworld/spot04/spot04.glb").exists() {
                match overworld::pieces::export(manifest, Path::new(ROOT), &kit_dir) {
                    Ok(k) => status = format!("cut the Kokiri kit ({} pieces) into {}", k.pieces.len(), kit_dir.display()),
                    Err(e) => status = format!("Kokiri kit: {e}"),
                }
            }
        }
        let kit = Kit::load(&kit_dir).ok().map(Arc::new);
        let piece = kit.as_ref().and_then(|k| k.pieces.first()).map(|p| p.name.clone()).unwrap_or_default();
        let out_dir = out_dir_for(&doc);
        let sel = match &opts.select {
            Some(n) => match (doc.regions.iter().position(|r| &r.name == n), doc.paths.iter().position(|p| &p.name == n)) {
                (Some(i), _) => Sel::Loop(i + 1),
                (None, Some(k)) => Sel::Path(k),
                _ => match n.strip_prefix("prop:").and_then(|i| i.parse::<usize>().ok()) {
                    Some(i) if i < doc.props.len() => Sel::Prop(i),
                    _ => Sel::None,
                },
            },
            None => Sel::None,
        };
        App {
            shapes: Shapes::new(&doc),
            shapes_doc: doc.clone(),
            saved: doc.clone(),
            committed: doc.clone(),
            doc,
            file,
            undo: vec![],
            redo: vec![],
            sel,
            tool: Tool::Select,
            drawing: vec![],
            gesture: Gesture::None,
            menu_at: None,
            view: View { center: [0.0, 0.0], scale: 0.1, rect: egui::Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0)) },
            fit_pending: true,
            theme: Arc::new(theme),
            theme_file: opts.theme,
            lib,
            worker,
            sent: None,
            built: None,
            level: None,
            ground: None,
            scene: None,
            build_error: None,
            problems: vec![],
            build_ms: 0.0,
            textures: HashMap::new(),
            shading: Shading::Textured,
            show_nodes: true,
            out_dir,
            live: false,
            walker: None,
            status,
            cursor: None,
            title: String::new(),
            profile: ProfileView::default(),
            profile_hover: None,
            layout: if v3.is_some() { Layout::Split } else { Layout::Plan },
            v3,
            gesture3: Gesture3::None,
            menu3_at: None,
            brush: Brush::default(),
            flatten_to: 0.0,
            multi: vec![],
            kit,
            piece,
            place_yaw: 0.0,
        }
    }

    // ---- document state -------------------------------------------------------------------

    fn refresh_shapes(&mut self) {
        if self.shapes_doc != self.doc {
            self.shapes = Shapes::new(&self.doc);
            self.shapes_doc = self.doc.clone();
        }
    }

    fn fix_sel(&mut self) {
        let ok = match self.sel {
            Sel::None => true,
            Sel::Loop(l) => l < edit::loop_count(&self.doc),
            Sel::Path(p) => p < self.doc.paths.len(),
            Sel::Node(NodeRef::Loop(l, i)) => l < edit::loop_count(&self.doc) && i < edit::loop_nodes(&self.doc, l).len(),
            Sel::Node(NodeRef::Path(p, i)) => p < self.doc.paths.len() && i < self.doc.paths[p].nodes.len(),
            Sel::Prop(i) => i < self.doc.props.len(),
            Sel::Line(k) => k < self.doc.lines.len(),
            Sel::Node(NodeRef::Line(k, i)) => k < self.doc.lines.len() && i < self.doc.lines[k].nodes.len(),
        };
        if !ok {
            self.sel = Sel::None;
        }
    }

    fn commit(&mut self) {
        if self.doc != self.committed {
            self.undo.push(std::mem::replace(&mut self.committed, self.doc.clone()));
            self.redo.clear();
            if self.undo.len() > 300 {
                self.undo.remove(0);
            }
        }
    }

    fn undo(&mut self) {
        self.commit();
        if let Some(prev) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.doc, prev.clone()));
            self.committed = prev;
            self.drawing.clear();
            self.fix_sel();
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.doc, next.clone()));
            self.committed = next;
            self.fix_sel();
        }
    }

    fn dirty(&self) -> bool {
        self.doc != self.saved
    }

    fn replace_doc(&mut self, doc: Doc, file: Option<PathBuf>) {
        self.out_dir = out_dir_for(&doc);
        self.saved = doc.clone();
        self.committed = doc.clone();
        self.doc = doc;
        self.file = file;
        self.undo.clear();
        self.redo.clear();
        self.sel = Sel::None;
        self.drawing.clear();
        self.fit_pending = true;
        self.live = false;
        if let Some(v) = &mut self.v3 {
            v.framed = false;
        }
    }

    fn confirm_discard(&self) -> bool {
        !self.dirty()
            || rfd::MessageDialog::new()
                .set_title("Unsaved changes")
                .set_description(format!("Discard the changes to {}?", self.doc.name))
                .set_buttons(rfd::MessageButtons::YesNo)
                .show()
                == rfd::MessageDialogResult::Yes
    }

    fn new_doc(&mut self) {
        if self.confirm_discard() {
            self.replace_doc(edit::blank_doc(), None);
            self.status = "new level".into();
        }
    }

    fn open(&mut self) {
        if !self.confirm_discard() {
            return;
        }
        let mut dlg = rfd::FileDialog::new().add_filter("level document", &["json"]);
        if let Some(d) = self.file.as_ref().and_then(|f| f.parent()) {
            dlg = dlg.set_directory(d);
        } else {
            dlg = dlg.set_directory(Path::new(ROOT).join("crates/tools/overworld/examples"));
        }
        if let Some(p) = dlg.pick_file() {
            match load_doc(&p) {
                Ok(d) => {
                    self.replace_doc(d, Some(p.clone()));
                    self.status = format!("opened {}", p.display());
                }
                Err(e) => self.status = e,
            }
        }
    }

    fn save(&mut self, ask: bool) {
        let path = match (&self.file, ask) {
            (Some(f), false) => f.clone(),
            _ => {
                let mut dlg = rfd::FileDialog::new().add_filter("level document", &["json"]).set_file_name(format!("{}.json", self.doc.name));
                if let Some(d) = self.file.as_ref().and_then(|f| f.parent()) {
                    dlg = dlg.set_directory(d);
                }
                match dlg.save_file() {
                    Some(p) => p,
                    None => return,
                }
            }
        };
        match std::fs::write(&path, edit::to_json(&self.doc)) {
            Ok(()) => {
                self.saved = self.doc.clone();
                self.status = format!("saved {}", path.display());
                self.file = Some(path);
            }
            Err(e) => self.status = format!("{}: {e}", path.display()),
        }
    }

    // ---- building -------------------------------------------------------------------------

    fn send_build(&mut self) {
        let key = (self.doc.clone(), self.live);
        if self.sent.as_ref() == Some(&key) {
            return;
        }
        self.worker.send(Job {
            doc: self.doc.clone(),
            theme: self.theme.clone(),
            lib: self.lib.clone(),
            kit: self.kit.clone(),
            export: self.live.then(|| self.out_dir.clone()),
        });
        self.sent = Some(key);
    }

    fn receive_build(&mut self) {
        let msgs = self.worker.poll();
        // only the newest build is worth showing; export problems apply in order
        let last = msgs.iter().rposition(|m| matches!(m, Msg::Built(_)));
        for (i, m) in msgs.into_iter().enumerate() {
            match m {
                Msg::Built(d) if Some(i) == last => self.show_build(d),
                Msg::Built(_) => {}
                Msg::Exported(p) => self.problems = p,
            }
        }
    }

    fn show_build(&mut self, d: Done) {
        {
            match d.level {
                Ok(l) => {
                    let t = std::time::Instant::now();
                    let scene = Scene::new(&l, &self.theme, self.lib.as_deref());
                    let t_scene = t.elapsed().as_secs_f64() * 1000.0;
                    if let Some(v) = &mut self.v3 {
                        v.set_level(&l, &scene.mats, self.lib.as_deref());
                        if !v.framed {
                            v.cam.frame(scene.bounds, self.doc.outline.z, v.aspect);
                            v.framed = true;
                        }
                    }
                    if std::env::var_os("OW_TIMING").is_some() {
                        eprintln!("build {:.1} ms, plan scene {t_scene:.1} ms, 3D upload {:.1} ms", d.ms, t.elapsed().as_secs_f64() * 1000.0 - t_scene);
                    }
                    self.scene = Some(Arc::new(scene));
                    self.ground = Some(Arc::new(Ground::new(&l.mesh)));
                    self.level = Some(l);
                    self.build_error = None;
                }
                Err(e) => self.build_error = Some(e),
            }
            self.problems = d.problems;
            self.build_ms = d.ms;
            self.built = Some(d.doc);
        }
    }

    fn building(&self) -> bool {
        self.sent.as_ref().map(|s| &s.0) != self.built.as_ref()
    }

    /// Writes the last good build and plays it in the clone (`oot_sandbox --level`) with child
    /// Link, at `at` facing north if given. Live export then keeps it up to date, and the game
    /// reloads each rebuild with Link where he stands.
    fn play(&mut self, at: Option<P2>) {
        let exe = Path::new(ROOT).join(if cfg!(windows) { "target/release/oot_sandbox.exe" } else { "target/release/oot_sandbox" });
        if !exe.exists() {
            self.status = "the game isn't built: run cargo build --release -p oot_sandbox".into();
            return;
        }
        let (Some(level), Some(doc)) = (&self.level, &self.built) else {
            self.status = "nothing built yet".into();
            return;
        };
        if let Err(e) = export::write(doc, &self.theme, level, self.lib.as_deref(), &self.out_dir) {
            self.status = format!("export: {e}");
            return;
        }
        self.live = true;
        let running = self.walker.as_mut().is_some_and(|c| matches!(c.try_wait(), Ok(None)));
        if running && at.is_none() {
            self.status = "the game is open: it reloads the level by itself".into();
            return;
        }
        if let Some(mut c) = self.walker.take() {
            let _ = c.kill();
        }
        // from the repo root, so the game finds oot.toml (the pad's settings) and its pack
        let mut cmd = Command::new(&exe);
        cmd.arg("--level").arg(&self.out_dir).arg("--child").current_dir(ROOT);
        if let Some(p) = at {
            // the game's axes are (x, z up, -y); binary angle 0x8000 faces -z, which is north
            let z = self.scene.as_ref().and_then(|s| s.floor_z(p)).unwrap_or_else(|| edit::base_z(&self.doc, &self.shapes, p));
            cmd.arg(format!("--at={:.0},{:.0},{:.0},-32768", p[0], z + 2.0, -p[1]));
        }
        match cmd.spawn() {
            Ok(c) => {
                self.walker = Some(c);
                self.status = format!("playing {} (pad or keyboard: WASD, Space A, E B, Q Z-target)", self.out_dir.display());
            }
            Err(e) => self.status = format!("the game: {e}"),
        }
    }

    // ---- edits ----------------------------------------------------------------------------

    fn delete_selection(&mut self) {
        match self.sel {
            Sel::Node(r) => match edit::delete_node(&mut self.doc, r) {
                Ok(()) => self.sel = Sel::None,
                Err(e) => self.status = e,
            },
            Sel::Loop(_) => {
                let mut ls: Vec<usize> = self.selected_loops().into_iter().filter(|&l| l > 0).collect();
                if ls.is_empty() {
                    self.status = "the outline can't be deleted".into();
                    return;
                }
                ls.sort_by(|a, b| b.cmp(a));
                for l in ls {
                    self.doc.regions.remove(l - 1);
                }
                self.sel = Sel::None;
                self.multi.clear();
            }
            Sel::Path(p) => {
                self.doc.paths.remove(p);
                self.sel = Sel::None;
            }
            Sel::Prop(i) => {
                self.doc.props.remove(i);
                self.sel = Sel::None;
            }
            Sel::Line(k) => {
                self.doc.lines.remove(k);
                self.sel = Sel::None;
            }
            Sel::None => {}
        }
    }

    // ---- props ----------------------------------------------------------------------------

    /// A prop's footprint in the level (from the kit, so it follows edits before the rebuild).
    fn prop_footprint(&self, i: usize) -> Vec<P2> {
        let prop = &self.doc.props[i];
        match self.kit.as_ref().and_then(|k| k.get(&prop.piece)) {
            // set into a wall: where the builder fitted it
            Some(piece) if self.on_wall(i) => self
                .level
                .as_ref()
                .and_then(|l| l.props.iter().find(|pl| pl.index == i && pl.piece == prop.piece))
                .map(|pl| pl.footprint.clone())
                .unwrap_or_else(|| overworld::props::footprint(piece, prop)),
            Some(piece) => overworld::props::footprint(piece, prop),
            None => (0..8).map(|k| {
                let a = k as f64 / 8.0 * std::f64::consts::TAU;
                [prop.at[0] + 60.0 * a.cos(), prop.at[1] + 60.0 * a.sin()]
            }).collect(),
        }
    }

    /// Whether a prop fits itself to the nearest wall (an opening or a wall piece).
    fn on_wall(&self, i: usize) -> bool {
        let prop = &self.doc.props[i];
        self.kit.as_ref().and_then(|k| k.get(&prop.piece)).is_some_and(|p| p.kind == "opening" || p.kind == "wall")
    }

    /// The prop under p, the topmost (last) first. A wall piece seen from above is a line along
    /// the wall: it's under p when p is near it.
    fn prop_at(&self, p: P2) -> Option<usize> {
        let reach = (PICK / self.view.scale).max(10.0);
        (0..self.doc.props.len()).rev().find(|&i| {
            let fp = self.prop_footprint(i);
            point_in_poly(p, &fp) || (self.on_wall(i) && fp.len() >= 2 && dist_to_loop(p, &fp) <= reach)
        })
    }

    /// Whether the Prop tool's piece fits itself to a wall (an opening or a wall piece).
    fn chosen_on_wall(&self) -> bool {
        self.kit.as_ref().and_then(|k| k.get(&self.piece)).is_some_and(|p| p.kind == "opening" || p.kind == "wall")
    }

    /// Where a wall piece or opening would go in the last build if it were `prop` (the ghost), or
    /// why it can't go there. None before a build, or for a piece the kit hasn't got.
    fn ghost(&self, prop: &Prop) -> Option<Result<Preview, String>> {
        let piece = self.kit.as_ref()?.get(&prop.piece)?;
        let (l, g) = (self.level.as_ref()?, self.ground.as_ref()?);
        Some(overworld::openings::preview(piece, prop, &l.mesh, g))
    }

    /// The wall piece or opening (as last built) at a point on its face or in its mouth.
    fn wall_prop_near(&self, q: [f64; 3]) -> Option<usize> {
        let (kit, l) = (self.kit.as_ref()?, self.level.as_ref()?);
        l.props
            .iter()
            .rev()
            .filter(|pl| self.doc.props.get(pl.index).is_some_and(|p| p.piece == pl.piece) && self.on_wall(pl.index))
            .find(|pl| {
                let Some(piece) = kit.get(&pl.piece) else { return false };
                // into the piece's own frame: unturned, unscaled
                let (s, c) = pl.yaw.to_radians().sin_cos();
                let d = [q[0] - pl.origin[0], q[1] - pl.origin[1], q[2] - pl.origin[2]];
                let x = (d[0] * c + d[1] * s) / pl.scale[0].max(1e-6);
                let y = (-d[0] * s + d[1] * c) / pl.scale[1].max(1e-6);
                let z = d[2] / pl.scale[2].max(1e-6);
                let b = piece.bounds;
                x >= b[0][0] - 10.0 && x <= b[1][0] + 10.0 && y >= b[0][1] - 10.0 && y <= b[1][1] + 30.0 && z >= b[0][2] - 10.0 && z <= b[1][2] + 10.0
            })
            .map(|pl| pl.index)
    }

    /// Where a prop's origin stands in the last build (its height), if it's been built.
    fn prop_z(&self, i: usize) -> Option<f64> {
        self.level.as_ref().and_then(|l| l.props.iter().find(|pl| pl.index == i)).map(|pl| pl.origin[2])
    }

    /// The handles of a prop in the plan: (turn handle, scale handle, the scale handle's distance
    /// from the origin at scale 1), in level coordinates.
    fn prop_handles(&self, i: usize) -> Option<(P2, P2, f64)> {
        if self.on_wall(i) {
            return None;
        }
        let prop = &self.doc.props[i];
        let piece = self.kit.as_ref()?.get(&prop.piece)?;
        let scale = piece.scale.clamp(prop.scale);
        let r = piece.footprint.iter().map(|q| (q[0] * scale[0]).hypot(q[1] * scale[1])).fold(0.0, f64::max);
        let len = (r * 1.15).max(36.0 / self.view.scale);
        let f = overworld::props::facing(prop.yaw);
        let turn = [prop.at[0] + f[0] * len, prop.at[1] + f[1] * len];
        let corner = [piece.bounds[1][0], piece.bounds[1][1]];
        let w = overworld::props::transform([corner[0], corner[1], 0.0], [prop.at[0], prop.at[1], 0.0], prop.yaw, scale);
        Some((turn, [w[0], w[1]], corner[0].hypot(corner[1]).max(1.0)))
    }

    /// Places the chosen piece at p, turned as the last one placed, and selects it.
    fn place_prop(&mut self, p: P2) {
        let Some(piece) = self.kit.as_ref().and_then(|k| k.get(&self.piece)) else {
            self.status = "no kit piece chosen (Kit panel)".into();
            return;
        };
        let label = piece.label.clone();
        self.doc.props.push(Prop { piece: self.piece.clone(), at: [p[0].round(), p[1].round()], z: None, yaw: self.place_yaw, scale: [1.0; 3] });
        self.sel = Sel::Prop(self.doc.props.len() - 1);
        self.status = format!("{label} placed: drag to move, its arrow's handle turns it (Q / E), its corner scales it");
    }

    /// Turns the selected prop by `d` degrees.
    fn turn_prop(&mut self, d: f64) {
        if let Sel::Prop(i) = self.sel {
            let y = &mut self.doc.props[i].yaw;
            *y = ((*y + d) % 360.0 + 360.0) % 360.0;
            if *y > 180.0 {
                *y -= 360.0;
            }
            self.place_yaw = *y;
        }
    }

    fn duplicate_prop(&mut self) {
        if let Sel::Prop(i) = self.sel {
            let mut p = self.doc.props[i].clone();
            p.at[0] += 120.0;
            p.at[1] -= 120.0;
            self.doc.props.push(p);
            self.sel = Sel::Prop(self.doc.props.len() - 1);
        }
    }

    fn toggle_sharp(&mut self) {
        if let Sel::Node(r @ NodeRef::Loop(..)) = self.sel {
            let g = edit::group(&self.doc, r);
            let s = edit::is_sharp(&self.doc, &g);
            edit::set_sharp(&mut self.doc, &g, !s);
        }
    }

    /// Raise (or sink) the selection: a region or the outline, or a path node's height.
    fn raise(&mut self, dz: f64) {
        match self.sel {
            Sel::Loop(_) => {
                for l in self.selected_loops() {
                    if l == 0 {
                        self.doc.outline.z += dz;
                    } else {
                        self.doc.regions[l - 1].z += dz;
                    }
                }
            }
            Sel::Node(NodeRef::Path(p, i)) => {
                let xy = edit::node_pos(&self.doc, NodeRef::Path(p, i));
                let cur = self.doc.paths[p].nodes[i].get(2).copied().flatten();
                let z = cur.unwrap_or_else(|| self.scene.as_ref().and_then(|s| s.floor_z(xy)).unwrap_or(edit::base_z(&self.doc, &self.shapes, xy)).round());
                let n = &mut self.doc.paths[p].nodes[i];
                n.resize(n.len().max(3), None);
                n[2] = Some(z + dz);
            }
            Sel::Node(NodeRef::Loop(l, _)) => {
                self.sel = Sel::Loop(l);
                self.raise(dz);
            }
            Sel::Prop(i) => {
                let z = self.doc.props[i].z.or(self.prop_z(i)).unwrap_or(0.0);
                self.doc.props[i].z = Some((z + dz).round());
            }
            _ => {}
        }
    }

    /// The selected loops: the primary selection and the Shift-clicked ones.
    fn selected_loops(&self) -> Vec<usize> {
        let mut out = vec![];
        if let Sel::Loop(l) = self.sel {
            out.push(l);
        }
        for &l in &self.multi {
            if !out.contains(&l) {
                out.push(l);
            }
        }
        out
    }

    /// A click selecting `s`: with Shift, a loop joins (or leaves) the selected loops.
    fn click_select(&mut self, s: Sel, shift: bool) {
        match (s, shift) {
            (Sel::Loop(l), true) => {
                let mut ls = self.selected_loops();
                if let Some(i) = ls.iter().position(|&x| x == l) {
                    ls.remove(i);
                } else {
                    ls.push(l);
                }
                self.sel = ls.first().map_or(Sel::None, |&l| Sel::Loop(l));
                self.multi = ls.into_iter().skip(1).collect();
            }
            _ => {
                self.sel = s;
                self.multi.clear();
            }
        }
    }

    /// One brush dab at w (in a stroke that `start`s now or goes on).
    fn paint_terrain(&mut self, w: P2, ctx: &egui::Context, start: bool) {
        let (dt, ctrl, shift) = ctx.input(|i| (i.stable_dt.clamp(0.0, 0.1) as f64, i.modifiers.command, i.modifiers.shift));
        let mut b = self.brush;
        // Ctrl turns raise into lower (and back); Shift smooths, as in most terrain editors
        b.mode = match (b.mode, ctrl, shift) {
            (_, _, true) => Mode::Smooth,
            (Mode::Raise, true, _) => Mode::Lower,
            (Mode::Lower, true, _) => Mode::Raise,
            (m, _, _) => m,
        };
        let t = self.doc.terrain.get_or_insert_with(Terrain::default);
        if start {
            self.flatten_to = t.at(w);
        }
        t.paint(&b, w, dt, self.flatten_to);
        if t.chunks.is_empty() {
            self.doc.terrain = None;
        }
        ctx.request_repaint();
    }

    /// Inserts a node on the nearest curve or path line at w, if one is close.
    fn insert_at(&mut self, w: P2) -> bool {
        let tol = PICK / self.view.scale;
        if let Some((k, i, q)) = self.shapes.line_near(&self.doc, w, tol, false) {
            self.sel = Sel::Node(edit::insert_line_node(&mut self.doc, k, i, q));
            return true;
        }
        if let Some((k, i, q)) = self.shapes.path_near(&self.doc, w, tol, false) {
            self.sel = Sel::Node(edit::insert_path_node(&mut self.doc, k, i, q));
            return true;
        }
        if let Some((l, k, q)) = self.shapes.loop_edge_near(w, tol) {
            self.sel = Sel::Node(edit::insert_loop_node(&mut self.doc, l, k, q));
            return true;
        }
        false
    }

    fn pick(&self, w: P2) -> Sel {
        let tol = PICK / self.view.scale;
        if let Some(r) = edit::node_near(&self.doc, w, tol).filter(|_| self.show_nodes) {
            return Sel::Node(r);
        }
        if let Some(i) = self.prop_at(w) {
            return Sel::Prop(i);
        }
        if let Some((k, _, _)) = self.shapes.line_near(&self.doc, w, tol, true) {
            return Sel::Line(k);
        }
        if let Some((k, _, _)) = self.shapes.path_near(&self.doc, w, tol, true) {
            return Sel::Path(k);
        }
        self.shapes.loop_at(w).map_or(Sel::None, Sel::Loop)
    }

    /// A drawing click: onto an existing loop node (shared), onto a loop's edge (a node is
    /// inserted there, so the edge is shared), or a free point.
    fn draw_click(&mut self, w: P2) {
        let tol = PICK / self.view.scale;
        if matches!(self.tool, Tool::Region | Tool::Line("hedge")) && self.drawing.len() >= 3 && dist(w, self.drawing[0]) <= tol {
            self.finish_drawing();
            return;
        }
        if matches!(self.tool, Tool::Path | Tool::Line(_)) {
            // no snapping: a path end exactly on a region's edge could take either side's height.
            // Ends go inside the floor they start from: a ramp slopes on to its end (cutting into a
            // plateau it runs into), a bridge's floating end lands at the floor's edge.
            let p = [w[0].round(), w[1].round()];
            if self.drawing.last() != Some(&p) {
                self.drawing.push(p);
            }
            return;
        }
        let p = match edit::node_near(&self.doc, w, tol) {
            Some(r @ NodeRef::Loop(..)) => edit::node_pos(&self.doc, r),
            _ => match self.shapes.loop_edge_near(w, tol) {
                Some((l, k, q)) => {
                    let r = edit::insert_loop_node(&mut self.doc, l, k, q);
                    self.refresh_shapes();
                    edit::node_pos(&self.doc, r)
                }
                None => [w[0].round(), w[1].round()],
            },
        };
        if self.drawing.last() != Some(&p) {
            self.drawing.push(p);
        }
    }

    fn finish_drawing(&mut self) {
        let pts = std::mem::take(&mut self.drawing);
        match self.tool {
            Tool::Region if pts.len() >= 3 => {
                let n = pts.len() as f64;
                let c = [pts.iter().map(|p| p[0]).sum::<f64>() / n, pts.iter().map(|p| p[1]).sum::<f64>() / n];
                let z = edit::base_z(&self.doc, &self.shapes, c) + 120.0;
                let r = edit::new_region(&self.doc, pts, z);
                self.status = format!("{} added at height {z:.0}: PageUp/PageDown raise and sink it", r.name);
                self.doc.regions.push(r);
                self.sel = Sel::Loop(self.doc.regions.len());
                self.tool = Tool::Select;
            }
            Tool::Path if pts.len() >= 2 => {
                let p = edit::new_path(&self.doc, pts);
                self.status = format!("{} added: its ends take the floor's height", p.name);
                self.doc.paths.push(p);
                self.sel = Sel::Path(self.doc.paths.len() - 1);
                self.tool = Tool::Select;
            }
            Tool::Line("hedge") if pts.len() < 3 => self.status = "a hedge needs 3 points".into(),
            Tool::Line(kind) if pts.len() >= 2 => {
                let l = edit::new_line(&self.doc, kind, pts);
                self.status = format!("{} added", l.name);
                self.doc.lines.push(l);
                self.sel = Sel::Line(self.doc.lines.len() - 1);
                self.tool = Tool::Select;
            }
            Tool::Region => self.status = "a region needs 3 points".into(),
            Tool::Path | Tool::Line(_) => self.status = "it needs 2 points".into(),
            Tool::Select | Tool::Brush | Tool::Prop => {}
        }
    }

    fn set_tool(&mut self, t: Tool) {
        self.tool = t;
        self.drawing.clear();
    }

    fn fit(&mut self) {
        let pts: Vec<P2> = self.doc.outline.nodes.iter().filter(|n| n.len() >= 2).map(|n| [n[0], n[1]]).collect();
        let mut bb = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
        for p in &pts {
            bb = [bb[0].min(p[0]), bb[1].min(p[1]), bb[2].max(p[0]), bb[3].max(p[1])];
        }
        if let Some(s) = &self.scene {
            bb = [bb[0].min(s.bounds[0]), bb[1].min(s.bounds[1]), bb[2].max(s.bounds[2]), bb[3].max(s.bounds[3])];
        }
        if bb[0].is_finite() {
            self.view.fit(bb);
            if let Some(v) = &mut self.v3 {
                v.cam.frame(bb, self.doc.outline.z, v.aspect);
            }
        }
    }

    // ---- input ----------------------------------------------------------------------------

    fn keys(&mut self, ctx: &egui::Context) {
        let sc = |m: Modifiers, k: Key| ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(m, k)));
        if sc(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z) || sc(Modifiers::COMMAND, Key::Y) {
            self.redo();
        }
        if sc(Modifiers::COMMAND, Key::Z) {
            self.undo();
        }
        if sc(Modifiers::COMMAND | Modifiers::SHIFT, Key::S) {
            self.save(true);
        }
        if sc(Modifiers::COMMAND, Key::S) {
            self.save(false);
        }
        if sc(Modifiers::COMMAND, Key::O) {
            self.open();
        }
        if sc(Modifiers::COMMAND, Key::N) {
            self.new_doc();
        }
        if sc(Modifiers::COMMAND, Key::D) {
            self.duplicate_prop();
        }
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let (shift, ctrl) = ctx.input(|i| (i.modifiers.shift, i.modifiers.command));
        if ctrl {
            return;
        }
        let pressed = |k: Key| ctx.input(|i| i.key_pressed(k));
        let step = if shift { 100.0 } else { 20.0 };
        if pressed(Key::Escape) {
            if !self.drawing.is_empty() {
                self.drawing.clear();
            } else if self.tool != Tool::Select {
                self.tool = Tool::Select;
            } else {
                self.sel = Sel::None;
            }
        }
        if pressed(Key::Enter) && !self.drawing.is_empty() {
            self.finish_drawing();
        }
        if pressed(Key::Backspace) && !self.drawing.is_empty() {
            self.drawing.pop();
        } else if pressed(Key::Delete) || pressed(Key::Backspace) {
            self.delete_selection();
        }
        if pressed(Key::V) {
            self.set_tool(Tool::Select);
        }
        if pressed(Key::R) {
            self.set_tool(Tool::Region);
        }
        if pressed(Key::B) {
            self.set_tool(Tool::Brush);
        }
        if pressed(Key::K) {
            self.set_tool(Tool::Prop);
        }
        if pressed(Key::D) {
            self.set_tool(Tool::Line("dirt"));
        }
        if pressed(Key::G) {
            self.set_tool(Tool::Line("fence"));
        }
        if pressed(Key::H) {
            self.set_tool(Tool::Line("bridge"));
        }
        if pressed(Key::J) {
            self.set_tool(Tool::Line("hedge"));
        }
        let fine = if shift { 1.0 } else { 15.0 };
        if pressed(Key::Q) {
            self.turn_prop(fine);
        }
        if pressed(Key::E) {
            self.turn_prop(-fine);
        }
        if self.tool == Tool::Brush {
            for (k, m) in [(Key::Num1, Mode::Raise), (Key::Num2, Mode::Lower), (Key::Num3, Mode::Smooth), (Key::Num4, Mode::Flatten), (Key::Num5, Mode::Bumps), (Key::Num6, Mode::Erase)] {
                if pressed(k) {
                    self.brush.mode = m;
                }
            }
            if pressed(Key::CloseBracket) {
                self.brush.radius = (self.brush.radius * 1.2).min(5000.0);
            }
            if pressed(Key::OpenBracket) {
                self.brush.radius = (self.brush.radius / 1.2).max(30.0);
            }
        }
        if pressed(Key::P) {
            self.set_tool(Tool::Path);
        }
        if pressed(Key::S) {
            self.toggle_sharp();
        }
        if pressed(Key::F) {
            self.fit();
        }
        if pressed(Key::T) {
            self.shading = match self.shading {
                Shading::Textured => Shading::Height,
                Shading::Height => Shading::Off,
                Shading::Off => Shading::Textured,
            };
        }
        if pressed(Key::N) {
            self.show_nodes = !self.show_nodes;
        }
        if pressed(Key::W) {
            if let Some(c) = self.cursor {
                self.play(Some(c));
            }
        }
        let brush = self.tool == Tool::Brush;
        if pressed(Key::PageUp) || (pressed(Key::CloseBracket) && !brush) {
            self.raise(step);
        }
        if pressed(Key::PageDown) || (pressed(Key::OpenBracket) && !brush) {
            self.raise(-step);
        }
    }

    fn canvas(&mut self, ui: &mut egui::Ui) {
        let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        self.view.rect = resp.rect;
        if self.fit_pending && resp.rect.width() > 50.0 {
            self.fit();
            self.fit_pending = false;
        }
        let ctx = ui.ctx().clone();
        let hover = resp.hover_pos();
        self.cursor = hover.map(|p| self.view.to_world(p));

        // zoom and pan
        if resp.hovered() {
            let (scroll, zoom) = ctx.input(|i| (i.smooth_scroll_delta.y as f64, i.zoom_delta() as f64));
            let f = zoom * (scroll * 0.0018).exp();
            if (f - 1.0).abs() > 1e-6 {
                if let Some(h) = hover {
                    self.view.zoom_at(h, f);
                }
            }
        }
        if resp.dragged_by(PointerButton::Middle) || resp.dragged_by(PointerButton::Secondary) {
            self.pan(resp.drag_delta());
        }
        let tol = PICK / self.view.scale;
        if self.tool == Tool::Brush && resp.is_pointer_button_down_on() && ctx.input(|i| i.pointer.primary_down()) {
            if let Some(p) = ctx.input(|i| i.pointer.interact_pos()) {
                let start = ctx.input(|i| i.pointer.primary_pressed());
                self.paint_terrain(self.view.to_world(p), &ctx, start);
            }
        }
        if resp.drag_started_by(PointerButton::Primary) {
            let origin = ctx.input(|i| i.pointer.press_origin()).map(|p| self.view.to_world(p));
            self.gesture = if self.tool == Tool::Brush { Gesture::None } else { Gesture::Pan };
            if let (Tool::Select | Tool::Prop, Some(o)) = (self.tool, origin) {
                // the selected prop's handles, then nodes, then any prop's footprint
                let handles = match self.sel {
                    Sel::Prop(i) => self.prop_handles(i).map(|h| (i, h)),
                    _ => None,
                };
                if let Some((i, _)) = handles.filter(|(_, (t, _, _))| dist(o, *t) <= tol * 1.3) {
                    self.gesture = Gesture::Prop(PropDrag::Turn { i });
                } else if let Some((i, (_, _, d0))) = handles.filter(|(_, (_, c, _))| dist(o, *c) <= tol * 1.3) {
                    self.gesture = Gesture::Prop(PropDrag::Scale { i, start: self.doc.props[i].scale, d0 });
                } else if let Some(r) = edit::node_near(&self.doc, o, tol).filter(|_| self.show_nodes && self.tool == Tool::Select) {
                    self.sel = Sel::Node(r);
                    self.gesture = Gesture::Nodes(edit::group(&self.doc, r));
                } else if let Some(i) = self.prop_at(o) {
                    self.sel = Sel::Prop(i);
                    let at = self.doc.props[i].at;
                    self.gesture = Gesture::Prop(PropDrag::Move { i, off: [at[0] - o[0], at[1] - o[1]] });
                }
            }
        }
        if resp.dragged_by(PointerButton::Primary) {
            match &self.gesture {
                Gesture::Nodes(refs) => {
                    if let Some(p) = resp.interact_pointer_pos() {
                        let w = self.view.to_world(p);
                        let alt = ctx.input(|i| i.modifiers.alt);
                        // snap onto another loop node (welding to it), else whole units
                        let target = edit::node_near_except(&self.doc, w, tol, refs)
                            .filter(|r| !alt && matches!(r, NodeRef::Loop(..)))
                            .map(|r| edit::node_pos(&self.doc, r))
                            .unwrap_or([w[0].round(), w[1].round()]);
                        let refs = refs.clone();
                        edit::move_group(&mut self.doc, &refs, target);
                    }
                }
                Gesture::Prop(d) => {
                    if let Some(p) = resp.interact_pointer_pos() {
                        let w = self.view.to_world(p);
                        let alt = ctx.input(|i| i.modifiers.alt);
                        self.drag_prop(*d, w, alt);
                    }
                }
                Gesture::Pan => self.pan(resp.drag_delta()),
                Gesture::None => {}
            }
        }
        if resp.drag_stopped() {
            self.gesture = Gesture::None;
        }
        if resp.clicked() {
            if let Some(w) = resp.interact_pointer_pos().map(|p| self.view.to_world(p)) {
                match self.tool {
                    Tool::Select if resp.double_clicked() => {
                        if !matches!(self.pick(w), Sel::Node(_)) && !self.insert_at(w) {
                            self.status = "double-click on a line to add a node".into();
                        }
                    }
                    Tool::Select => {
                        let shift = ctx.input(|i| i.modifiers.shift);
                        self.click_select(self.pick(w), shift);
                    }
                    Tool::Brush => {}
                    Tool::Prop => match self.prop_at(w) {
                        Some(i) => self.sel = Sel::Prop(i),
                        None => self.place_prop(w),
                    },
                    Tool::Path | Tool::Line(_) if resp.double_clicked() => self.finish_drawing(),
                    _ => self.draw_click(w),
                }
            }
        }
        if resp.secondary_clicked() {
            self.menu_at = resp.interact_pointer_pos().map(|p| self.view.to_world(p));
        }
        let mut action = None;
        if let Some(at) = self.menu_at {
            let near_node = edit::node_near(&self.doc, at, tol);
            let near_line = self.shapes.loop_edge_near(at, tol).is_some() || self.shapes.path_near(&self.doc, at, tol, false).is_some();
            resp.context_menu(|ui| {
                if ui.button("Play from here").clicked() {
                    action = Some(Action::PlayAt(at));
                    ui.close();
                }
                if let Some(r) = near_node {
                    self.sel = Sel::Node(r);
                    if ui.button("Delete node").clicked() {
                        action = Some(Action::Delete);
                        ui.close();
                    }
                    if matches!(r, NodeRef::Loop(..)) && ui.button("Sharp / smooth corner").clicked() {
                        action = Some(Action::ToggleSharp);
                        ui.close();
                    }
                } else if near_line && ui.button("Add node here").clicked() {
                    action = Some(Action::Insert(at));
                    ui.close();
                }
                if near_node.is_none() {
                    if let Some(l) = self.shapes.loop_at(at) {
                        ui.separator();
                        ui.label(edit::loop_name(&self.doc, l));
                        if ui.button("Raise 20").clicked() {
                            self.sel = Sel::Loop(l);
                            action = Some(Action::Raise(20.0));
                        }
                        if ui.button("Sink 20").clicked() {
                            self.sel = Sel::Loop(l);
                            action = Some(Action::Raise(-20.0));
                        }
                    }
                }
                ui.separator();
                if ui.button("Fit view").clicked() {
                    action = Some(Action::Fit);
                    ui.close();
                }
            });
        }
        match action {
            Some(Action::PlayAt(p)) => self.play(Some(p)),
            Some(Action::Insert(p)) => {
                self.insert_at(p);
            }
            Some(Action::Delete) => self.delete_selection(),
            Some(Action::ToggleSharp) => self.toggle_sharp(),
            Some(Action::Raise(dz)) => self.raise(dz),
            Some(Action::Fit) => self.fit(),
            None => {}
        }

        self.refresh_shapes();
        self.paint(&painter, &ctx, hover);
    }

    /// A prop drag in the plan or 3D at w (Alt: no snapping).
    fn drag_prop(&mut self, d: PropDrag, w: P2, alt: bool) {
        match d {
            PropDrag::Move { i, off } => {
                let p = [w[0] + off[0], w[1] + off[1]];
                self.doc.props[i].at = if alt { p } else { [p[0].round(), p[1].round()] };
            }
            PropDrag::Turn { i } => {
                let at = self.doc.props[i].at;
                let a = (w[1] - at[1]).atan2(w[0] - at[0]).to_degrees() - 90.0;
                let a = if alt { a } else { (a / 15.0).round() * 15.0 };
                let a = ((a % 360.0) + 540.0) % 360.0 - 180.0;
                self.doc.props[i].yaw = (a * 100.0).round() / 100.0;
                self.place_yaw = self.doc.props[i].yaw;
            }
            PropDrag::Scale { i, start, d0 } => {
                let at = self.doc.props[i].at;
                let f = dist(w, at) / d0;
                let f = if alt { f } else { (f / 0.05).round() * 0.05 };
                let s = [f, f, start[2] * f / start[0].max(1e-6)];
                let lim = self.kit.as_ref().and_then(|k| k.get(&self.doc.props[i].piece)).map(|p| p.scale.clone()).unwrap_or_default();
                // the footprint scales; the height follows only where the piece scales evenly
                let s = if lim.uniform { lim.clamp(s) } else { lim.clamp([s[0], s[1], start[2]]) };
                self.doc.props[i].scale = s.map(|x| (x * 1000.0).round() / 1000.0);
            }
        }
    }

    /// The 3D view: orbit, pan and zoom; click to select; drag the selected region to raise or
    /// sink it.
    fn view3d_ui(&mut self, ui: &mut egui::Ui) {
        let Some(mut cam) = self.v3.as_ref().map(|v| v.cam) else {
            ui.label("The 3D view needs the wgpu backend.");
            return;
        };
        let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = resp.rect;
        if rect.width() < 4.0 || rect.height() < 4.0 {
            return;
        }
        let ctx = ui.ctx().clone();
        let aspect = rect.width() / rect.height();
        let ray = |cam: &crate::view3d::Camera, p: Pos2| {
            let ndc = [(p.x - rect.left()) / rect.width() * 2.0 - 1.0, 1.0 - (p.y - rect.top()) / rect.height() * 2.0];
            let (o, d) = cam.ray(aspect, ndc);
            ([o.x as f64, o.y as f64, o.z as f64], [d.x as f64, d.y as f64, d.z as f64])
        };
        let scene = self.scene.clone();
        let hit_at = |cam: &crate::view3d::Camera, p: Pos2| {
            let (o, d) = ray(cam, p);
            scene.as_ref().and_then(|s| s.raycast(o, d))
        };
        // for wall pieces: the point under the pointer (a wall's face, or a floor) and where to put
        // one down for it to fit there (a little out from a face, so that face is the nearest wall)
        let wall_spot = |cam: &crate::view3d::Camera, p: Pos2| -> Option<([f64; 3], P2)> {
            let (o, d) = ray(cam, p);
            let s = scene.as_ref()?;
            let floor = s.raycast(o, d);
            match s.raycast_wall(o, d) {
                Some((t, q, n)) if floor.is_none_or(|f| t < f.0) => Some((q, [q[0] + n[0] * 5.0, q[1] + n[1] * 5.0])),
                _ => floor.map(|(_, q)| (q, [q[0], q[1]])),
            }
        };
        let wall_tool = self.tool == Tool::Prop && self.chosen_on_wall();
        if let Some(h) = resp.hover_pos() {
            if let Some((_, q)) = hit_at(&cam, h) {
                self.cursor = Some([q[0], q[1]]);
            }
            let (scroll, zoom) = ctx.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
            let f = (-scroll * 0.0018).exp() / zoom;
            if (f - 1.0).abs() > 1e-6 {
                cam.zoom(f);
            }
        }
        let alt = ctx.input(|i| i.modifiers.alt);
        let painting = self.tool == Tool::Brush && !alt;
        if painting && resp.is_pointer_button_down_on() && ctx.input(|i| i.pointer.primary_down()) {
            let start = ctx.input(|i| i.pointer.primary_pressed());
            if let Some(q) = ctx.input(|i| i.pointer.interact_pos()).and_then(|p| hit_at(&cam, p)).map(|x| x.1) {
                self.paint_terrain([q[0], q[1]], &ctx, start);
            }
        }
        if resp.drag_started_by(PointerButton::Primary) {
            self.gesture3 = if painting { Gesture3::Paint } else { Gesture3::Orbit };
            let origin = ctx.input(|i| i.pointer.press_origin());
            // dragging a prop moves it, over the plane at its height; a wall piece or opening
            // slides along the walls under the pointer
            if let (Some(o), false) = (origin, painting) {
                if let Some(i) = wall_spot(&cam, o).and_then(|(q, _)| self.wall_prop_near(q)) {
                    self.sel = Sel::Prop(i);
                    self.gesture3 = Gesture3::WallProp { i };
                } else if let Some((_, q)) = hit_at(&cam, o) {
                    if let Some(i) = self.prop_at([q[0], q[1]]) {
                        let at = self.doc.props[i].at;
                        self.sel = Sel::Prop(i);
                        self.gesture3 = if self.on_wall(i) { Gesture3::WallProp { i } } else { Gesture3::Prop { i, off: [at[0] - q[0], at[1] - q[1]], z: q[2] } };
                    }
                }
            }
            if let (Sel::Loop(_), Some(o), false) = (self.sel, origin, painting) {
                if let Some((t, q)) = hit_at(&cam, o) {
                    let ls = self.selected_loops();
                    if self.shapes.loop_at([q[0], q[1]]).is_some_and(|l| ls.contains(&l)) {
                        let per_px = 2.0 * t * (FOV.to_radians() as f64 * 0.5).tan() / rect.height() as f64;
                        let ls = ls.into_iter().map(|l| (l, edit::loop_z(&self.doc, l))).collect();
                        self.gesture3 = Gesture3::Height { ls, per_px, dy: 0.0 };
                    }
                }
            }
        }
        if resp.dragged_by(PointerButton::Primary) {
            let d = resp.drag_delta();
            let snap = if ctx.input(|i| i.modifiers.shift) { 1.0 } else { 5.0 };
            match &mut self.gesture3 {
                Gesture3::Orbit => cam.orbit(d),
                Gesture3::Height { ls, per_px, dy } => {
                    *dy += d.y as f64;
                    // the first region snaps; the others keep their heights relative to it
                    let delta = ((ls[0].1 - *dy * *per_px) / snap).round() * snap - ls[0].1;
                    for &(l, start) in ls.iter() {
                        if l == 0 {
                            self.doc.outline.z = start + delta;
                        } else {
                            self.doc.regions[l - 1].z = start + delta;
                        }
                    }
                }
                Gesture3::Prop { i, off, z } => {
                    let (i, off, z) = (*i, *off, *z);
                    if let Some(p) = resp.interact_pointer_pos() {
                        let (o, d) = ray(&cam, p);
                        if d[2].abs() > 1e-6 {
                            let t = (z - o[2]) / d[2];
                            if t > 0.0 {
                                let w = [o[0] + d[0] * t - off[0], o[1] + d[1] * t - off[1]];
                                let alt = ctx.input(|i| i.modifiers.alt);
                                self.drag_prop(PropDrag::Move { i, off: [0.0, 0.0] }, w, alt);
                            }
                        }
                    }
                }
                Gesture3::WallProp { i } => {
                    let i = *i;
                    if let Some((_, at)) = resp.interact_pointer_pos().and_then(|p| wall_spot(&cam, p)) {
                        self.doc.props[i].at = at.map(|x| x.round());
                    }
                }
                Gesture3::Paint | Gesture3::None => {}
            }
        }
        if resp.dragged_by(PointerButton::Middle) {
            cam.orbit(resp.drag_delta());
        }
        if resp.dragged_by(PointerButton::Secondary) {
            cam.pan(resp.drag_delta(), rect.height());
        }
        if resp.drag_stopped() {
            self.gesture3 = Gesture3::None;
        }
        if resp.clicked() && !painting {
            let hit = resp.interact_pointer_pos().and_then(|p| hit_at(&cam, p));
            let spot = resp.interact_pointer_pos().and_then(|p| wall_spot(&cam, p));
            let on_wall_prop = spot.and_then(|(q, _)| self.wall_prop_near(q));
            let s = match (on_wall_prop, hit) {
                (Some(i), _) => Sel::Prop(i),
                (None, Some((_, q))) => {
                    let w = [q[0], q[1]];
                    match (self.prop_at(w), self.shapes.line_near(&self.doc, w, 1.0, true), self.shapes.path_near(&self.doc, w, 1.0, true)) {
                        (Some(i), _, _) => Sel::Prop(i),
                        (None, Some((k, _, _)), _) => Sel::Line(k),
                        (None, None, Some((k, _, _))) => Sel::Path(k),
                        _ => self.shapes.loop_at(w).map_or(Sel::None, Sel::Loop),
                    }
                }
                (None, None) => Sel::None,
            };
            let shift = ctx.input(|i| i.modifiers.shift);
            match (self.tool, hit, s) {
                (Tool::Prop, _, Sel::Loop(_) | Sel::Path(_) | Sel::Line(_) | Sel::None) if wall_tool => {
                    if let Some((_, at)) = spot {
                        self.place_prop(at);
                    }
                }
                (Tool::Prop, Some((_, q)), Sel::Loop(_) | Sel::Path(_) | Sel::None) => self.place_prop([q[0], q[1]]),
                _ => self.click_select(s, shift),
            }
            if resp.double_clicked() {
                if let Some((_, q)) = hit {
                    cam.target = glam::Vec3::new(q[0] as f32, q[1] as f32, q[2] as f32);
                }
            }
        }
        if resp.secondary_clicked() {
            self.menu3_at = resp.interact_pointer_pos().and_then(|p| hit_at(&cam, p)).map(|(_, q)| [q[0], q[1]]);
        }
        let mut walk_at = None;
        let mut frame = false;
        let menu_at = self.menu3_at;
        resp.context_menu(|ui| {
            if let Some(at) = menu_at {
                if ui.button("Play from here").clicked() {
                    walk_at = Some(at);
                    ui.close();
                }
            }
            if ui.button("Frame the level").clicked() {
                frame = true;
                ui.close();
            }
        });

        let v3 = self.v3.as_mut().unwrap();
        v3.cam = cam;
        let ppp = ctx.pixels_per_point();
        let id = v3.render((rect.width() * ppp) as u32, (rect.height() * ppp) as u32);
        painter.image(id, rect, egui::Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);

        // overlays: the selection at its height, drawn over everything like a gizmo
        let line3 = |pts: &[[f64; 3]], closed: bool, stroke: Stroke| {
            let n = pts.len();
            let m = if closed { n } else { n.saturating_sub(1) };
            for i in 0..m {
                let (a, b) = (pts[i], pts[(i + 1) % n]);
                let pa = cam.project(rect, glam::Vec3::new(a[0] as f32, a[1] as f32, a[2] as f32));
                let pb = cam.project(rect, glam::Vec3::new(b[0] as f32, b[1] as f32, b[2] as f32));
                if let (Some(pa), Some(pb)) = (pa, pb) {
                    painter.line_segment([pa, pb], stroke);
                }
            }
        };
        let mut sel_loops = self.selected_loops();
        if let Sel::Node(NodeRef::Loop(l, _)) = self.sel {
            sel_loops.push(l);
        }
        // outlines at the design height plus the painted terrain under them
        let field = self.doc.terrain.as_ref().map(|t| {
            let polys: Vec<Vec<P2>> = (0..self.shapes.loops.len()).map(|l| self.shapes.poly(l)).collect();
            (t, polys)
        });
        let field = field.as_ref().map(|(t, polys)| overworld::terrain::Field::new(t, &self.doc, polys));
        let lift = |p: P2| field.as_ref().map_or(0.0, |f| f.at(p));
        for (n, &l) in sel_loops.iter().enumerate() {
            let z = edit::loop_z(&self.doc, l);
            let pts: Vec<[f64; 3]> = self.shapes.loops[l].iter().map(|x| [x.0[0], x.0[1], z + 1.0 + lift(x.0)]).collect();
            line3(&pts, true, Stroke::new(4.0, Color32::from_black_alpha(120)));
            line3(&pts, true, Stroke::new(2.0, Color32::from_rgb(255, 150, 50)));
            if let (Gesture3::Height { .. }, 0) = (&self.gesture3, n) {
                if let Some(p) = resp.interact_pointer_pos() {
                    let more = if sel_loops.len() > 1 { format!(" (+{} more)", sel_loops.len() - 1) } else { String::new() };
                    painter.text(p + Vec2::new(14.0, -14.0), Align2::LEFT_BOTTOM, format!("{}  {z:.0}{more}", edit::loop_name(&self.doc, l)), FontId::proportional(14.0), Color32::WHITE);
                }
            }
        }
        if self.tool == Tool::Brush {
            if let Some((_, q)) = resp.hover_pos().and_then(|h| hit_at(&cam, h)) {
                let scene = self.scene.as_ref();
                for (r, w) in [(self.brush.radius, 2.0), (self.brush.radius * self.brush.falloff, 1.0)] {
                    let pts: Vec<[f64; 3]> = (0..48)
                        .map(|k| {
                            let a = k as f64 / 48.0 * std::f64::consts::TAU;
                            let p = [q[0] + r * a.cos(), q[1] + r * a.sin()];
                            [p[0], p[1], scene.and_then(|s| s.floor_z(p)).unwrap_or(q[2]) + 3.0]
                        })
                        .collect();
                    line3(&pts, true, Stroke::new(w, brush_colour(self.brush.mode)));
                }
            }
        }
        let sel_path = match self.sel {
            Sel::Path(k) | Sel::Node(NodeRef::Path(k, _)) => Some(k),
            _ => None,
        };
        if let Some(k) = sel_path {
            if let Ok(pr) = crate::profile::profile(&self.doc, &self.shapes, k) {
                let pts: Vec<[f64; 3]> = pr.geo.st.iter().map(|x| [x.p[0], x.p[1], x.z + 2.0]).collect();
                line3(&pts, false, Stroke::new(3.0, Color32::from_rgb(235, 130, 255)));
            }
        }
        if let Some(p) = self.profile_hover {
            if let Some(z) = self.scene.as_ref().and_then(|s| s.floor_z(p)) {
                if let Some(s) = cam.project(rect, glam::Vec3::new(p[0] as f32, p[1] as f32, z as f32)) {
                    painter.circle_stroke(s, 9.0, Stroke::new(2.5, Color32::WHITE));
                }
            }
        }
        // the selected prop: its footprint at its base, and which way it faces
        if let Sel::Prop(i) = self.sel {
            let prop = &self.doc.props[i];
            let z = prop.z.or(self.prop_z(i)).or_else(|| self.scene.as_ref().and_then(|s| s.floor_z(prop.at))).unwrap_or(0.0) + 2.0;
            let pts: Vec<[f64; 3]> = self.prop_footprint(i).iter().map(|q| [q[0], q[1], z]).collect();
            line3(&pts, true, Stroke::new(4.0, Color32::from_black_alpha(120)));
            line3(&pts, true, Stroke::new(2.0, PROP_COLOUR));
            let f = overworld::props::facing(prop.yaw);
            let r = pts.iter().map(|q| (q[0] - prop.at[0]).hypot(q[1] - prop.at[1])).fold(40.0, f64::max);
            line3(&[[prop.at[0], prop.at[1], z], [prop.at[0] + f[0] * r * 1.15, prop.at[1] + f[1] * r * 1.15, z]], false, Stroke::new(2.0, PROP_COLOUR));
        }
        // the ghost: where a wall piece would go (the one being put down, or dragged), or why not
        let ghost = match (&self.gesture3, resp.hover_pos()) {
            (Gesture3::WallProp { i }, _) => self.doc.props.get(*i).cloned().zip(resp.interact_pointer_pos()),
            (_, Some(h)) if wall_tool => wall_spot(&cam, h).map(|(_, at)| (Prop { piece: self.piece.clone(), at, z: None, yaw: 0.0, scale: [1.0; 3] }, h)),
            _ => None,
        };
        if let Some((prop, h)) = ghost {
            match self.ghost(&prop) {
                Some(Ok(pv)) => {
                    let fill = Color32::from_rgba_unmultiplied(90, 235, 170, 70);
                    let mut mesh = egui::epaint::Mesh::default();
                    for t in &pv.tris {
                        let s: Vec<Pos2> = t.iter().filter_map(|q| cam.project(rect, glam::Vec3::new(q[0] as f32, q[1] as f32, q[2] as f32))).collect();
                        if s.len() == 3 {
                            let base = mesh.vertices.len() as u32;
                            for p in s {
                                mesh.colored_vertex(p, fill);
                            }
                            mesh.add_triangle(base, base + 1, base + 2);
                        }
                    }
                    painter.add(Shape::mesh(mesh));
                    // its foot along the wall, and which way it faces
                    let f = overworld::props::facing(pv.prop.yaw);
                    let z = pv.prop.z.unwrap_or(0.0) + 1.0;
                    let (o, ac) = (pv.prop.at, [f[1], -f[0]]);
                    let w = pv.tris.iter().flatten().map(|q| ((q[0] - o[0]) * ac[0] + (q[1] - o[1]) * ac[1]).abs()).fold(20.0, f64::max);
                    line3(&[[o[0] - ac[0] * w, o[1] - ac[1] * w, z], [o[0] + ac[0] * w, o[1] + ac[1] * w, z]], false, Stroke::new(3.0, PROP_COLOUR));
                    line3(&[[o[0], o[1], z], [o[0] + f[0] * 60.0, o[1] + f[1] * 60.0, z]], false, Stroke::new(2.0, PROP_COLOUR));
                    let what = if matches!(self.gesture3, Gesture3::WallProp { .. }) { "let go: move it here" } else { "click: put it here" };
                    let text = format!("{what} · the wall is {:.0} wide here", pv.room);
                    painter.text(h + Vec2::new(15.0, 1.0), Align2::LEFT_BOTTOM, &text, FontId::proportional(13.0), Color32::from_black_alpha(220));
                    painter.text(h + Vec2::new(14.0, 0.0), Align2::LEFT_BOTTOM, &text, FontId::proportional(13.0), PROP_COLOUR);
                }
                Some(Err(e)) => {
                    let red = Color32::from_rgb(255, 110, 90);
                    painter.circle_stroke(h, 10.0, Stroke::new(2.5, red));
                    let e = e.split_once(": ").map_or(e.as_str(), |x| x.1).to_string();
                    painter.text(h + Vec2::new(15.0, 1.0), Align2::LEFT_BOTTOM, &e, FontId::proportional(13.0), Color32::from_black_alpha(220));
                    painter.text(h + Vec2::new(14.0, 0.0), Align2::LEFT_BOTTOM, &e, FontId::proportional(13.0), red);
                }
                None => {}
            }
        }
        painter.text(
            rect.left_bottom() + Vec2::new(10.0, -8.0),
            Align2::LEFT_BOTTOM,
            if self.tool == Tool::Brush {
                "drag: paint · Alt-drag or middle-drag: orbit · right-drag: pan · wheel: zoom · Ctrl: lower · Shift: smooth"
            } else if wall_tool {
                "hover a wall: the ghost shows where it goes (red: why it can't) · click: put it there · drag one along the walls · drag elsewhere: orbit"
            } else if self.tool == Tool::Prop {
                "click: place the Kit panel's piece · drag a prop: move it · Q / E: turn · drag elsewhere: orbit · right-drag: pan"
            } else {
                "drag: orbit · right-drag: pan · wheel: zoom · double-click: orbit round that spot · drag the selected regions: raise / sink"
            },
            FontId::proportional(11.0),
            Color32::from_black_alpha(200),
        );
        if let Some(at) = walk_at {
            self.play(Some(at));
        }
        if frame {
            self.fit();
        }
    }

    fn pan(&mut self, d: Vec2) {
        self.view.center[0] -= d.x as f64 / self.view.scale;
        self.view.center[1] += d.y as f64 / self.view.scale;
    }

    // ---- drawing --------------------------------------------------------------------------

    fn texture_ids(&mut self, ctx: &egui::Context) -> Vec<Option<egui::TextureId>> {
        let (Some(scene), Some(lib)) = (&self.scene, &self.lib) else { return vec![] };
        scene
            .mats
            .iter()
            .map(|m| {
                let h = self.textures.entry(m.name.clone()).or_insert_with(|| {
                    let (w, h, px) = lib.rgba(&m.name)?;
                    let ci = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &px);
                    Some(ctx.load_texture(m.name.clone(), ci, scene::texture_options(&m.info)))
                });
                h.as_ref().map(|h| h.id())
            })
            .collect()
    }

    fn paint(&mut self, painter: &egui::Painter, ctx: &egui::Context, hover: Option<Pos2>) {
        let rect = self.view.rect;
        painter.rect_filled(rect, 0.0, Color32::from_rgb(22, 26, 28));
        let ids = self.texture_ids(ctx);
        let v = self.view;
        if let Some(s) = &self.scene {
            s.paint(painter, &v, self.shading, &|m| ids.get(m).copied().flatten());
        }
        let font = FontId::proportional(12.0);
        let label = |p: Pos2, text: &str, col: Color32| {
            painter.text(p + Vec2::new(1.0, 1.0), Align2::CENTER_CENTER, text, font.clone(), Color32::from_black_alpha(200));
            painter.text(p, Align2::CENTER_CENTER, text, font.clone(), col);
        };

        // loops
        let sel_loops = self.selected_loops();
        let sel_loop = |l: usize| sel_loops.contains(&l);
        for (l, c) in self.shapes.loops.iter().enumerate() {
            if c.len() < 2 {
                continue;
            }
            let water = l > 0 && self.doc.regions[l - 1].kind == "water";
            let col = if l == 0 {
                Color32::from_rgb(235, 235, 235)
            } else if water {
                Color32::from_rgb(110, 180, 255)
            } else {
                Color32::from_rgb(255, 196, 80)
            };
            let width = if sel_loop(l) { 3.5 } else { 1.6 };
            let pts: Vec<Pos2> = c.iter().map(|x| v.to_screen(x.0)).collect();
            if sel_loop(l) {
                painter.add(Shape::closed_line(pts.clone(), Stroke::new(7.0, Color32::from_black_alpha(110))));
            }
            painter.add(Shape::closed_line(pts, Stroke::new(width, col)));
        }
        // props: footprints, which way they face, and the selected one's handles
        for i in 0..self.doc.props.len() {
            let sel = self.sel == Sel::Prop(i);
            let pts: Vec<Pos2> = self.prop_footprint(i).iter().map(|&q| v.to_screen(q)).collect();
            if sel {
                painter.add(Shape::closed_line(pts.clone(), Stroke::new(6.0, Color32::from_black_alpha(110))));
            }
            painter.add(Shape::closed_line(pts, Stroke::new(if sel { 2.5 } else { 1.3 }, PROP_COLOUR)));
            let prop = &self.doc.props[i];
            let o = v.to_screen(prop.at);
            let f = overworld::props::facing(prop.yaw);
            painter.arrow(o, Vec2::new(f[0] as f32, -f[1] as f32) * 16.0, Stroke::new(1.5, PROP_COLOUR));
            if sel {
                if let Some((turn, corner, _)) = self.prop_handles(i) {
                    let (t, c) = (v.to_screen(turn), v.to_screen(corner));
                    painter.line_segment([o, t], Stroke::new(1.5, PROP_COLOUR));
                    painter.circle(t, 6.0, PROP_COLOUR, Stroke::new(1.0, Color32::BLACK));
                    painter.rect(egui::Rect::from_center_size(c, Vec2::splat(10.0)), 0.0, PROP_COLOUR, Stroke::new(1.0, Color32::BLACK), egui::StrokeKind::Middle);
                }
            }
        }
        // lines: dirt paths as their band, others as their line
        for (k, c) in self.shapes.lines.iter().enumerate() {
            let l = &self.doc.lines[k];
            let pts: Vec<Pos2> = c.iter().map(|x| v.to_screen(x.0)).collect();
            let sel = self.sel == Sel::Line(k) || matches!(self.sel, Sel::Node(NodeRef::Line(j, _)) if j == k);
            let col = line_colour(&l.kind);
            if l.kind == "dirt" {
                let w = l.width.or(self.theme.dirt.as_ref().map(|d| d.width)).unwrap_or(160.0);
                let px = (w * v.scale) as f32;
                painter.add(Shape::line(pts.clone(), Stroke::new(px.max(2.0), col.gamma_multiply(if sel { 0.45 } else { 0.25 }))));
            }
            if sel {
                painter.add(Shape::line(pts.clone(), Stroke::new(5.0, Color32::from_black_alpha(110))));
            }
            painter.extend(Shape::dashed_line(&pts, Stroke::new(if sel { 3.0 } else { 1.6 }, col), 10.0, 4.0));
        }
        // paths: dashed centre lines
        for (k, c) in self.shapes.paths.iter().enumerate() {
            let pts: Vec<Pos2> = c.iter().map(|x| v.to_screen(x.0)).collect();
            let w = if self.sel == Sel::Path(k) { 3.0 } else { 1.6 };
            painter.extend(Shape::dashed_line(&pts, Stroke::new(w, Color32::from_rgb(235, 130, 255)), 8.0, 5.0));
        }
        // labels
        if v.scale > 0.02 {
            for l in 1..self.shapes.loops.len() {
                let c = &self.shapes.loops[l];
                if c.is_empty() || self.shapes.areas[l] * v.scale * v.scale < 2500.0 {
                    continue;
                }
                let n = c.len() as f64;
                let m = [c.iter().map(|x| x.0[0]).sum::<f64>() / n, c.iter().map(|x| x.0[1]).sum::<f64>() / n];
                let r = &self.doc.regions[l - 1];
                label(v.to_screen(m), &format!("{}  {:.0}", edit::loop_name(&self.doc, l), r.z), Color32::WHITE);
            }
            for (k, c) in self.shapes.paths.iter().enumerate() {
                if let Some(m) = c.get(c.len() / 2) {
                    label(v.to_screen(m.0) + Vec2::new(0.0, -12.0), &edit::path_name(&self.doc, k), Color32::from_rgb(245, 200, 255));
                }
            }
            if v.scale > 0.06 {
                for prop in &self.doc.props {
                    let name = self.kit.as_ref().and_then(|k| k.get(&prop.piece)).map_or(prop.piece.as_str(), |p| p.label.as_str());
                    label(v.to_screen(prop.at) + Vec2::new(0.0, 14.0), name, Color32::from_rgb(200, 255, 230));
                }
            }
        }
        // nodes
        let sel_group: Vec<NodeRef> = match self.sel {
            Sel::Node(r) => edit::group(&self.doc, r),
            _ => vec![],
        };
        if self.show_nodes {
            let hovered = hover.filter(|_| self.tool == Tool::Select).and_then(|h| edit::node_near(&self.doc, v.to_world(h), PICK / v.scale));
            for l in 0..edit::loop_count(&self.doc) {
                for (i, n) in edit::loop_nodes(&self.doc, l).iter().enumerate() {
                    if n.len() < 2 {
                        continue;
                    }
                    let p = v.to_screen([n[0], n[1]]);
                    if !rect.expand(10.0).contains(p) {
                        continue;
                    }
                    let r = NodeRef::Loop(l, i);
                    let shared = edit::group(&self.doc, r).len() > 1;
                    let sharp = overworld::doc::node_sharp(n);
                    let fill = if sel_group.contains(&r) {
                        Color32::from_rgb(255, 120, 40)
                    } else if l == 0 {
                        Color32::from_rgb(240, 240, 240)
                    } else {
                        Color32::from_rgb(255, 200, 90)
                    };
                    let rad = if shared { 5.0 } else { 3.8 } + if hovered == Some(r) { 2.0 } else { 0.0 };
                    let stroke = Stroke::new(if shared { 2.0 } else { 1.0 }, if shared { Color32::from_rgb(40, 220, 160) } else { Color32::BLACK });
                    if sharp {
                        painter.rect(egui::Rect::from_center_size(p, Vec2::splat(rad * 2.0)), 0.0, fill, stroke, egui::StrokeKind::Middle);
                    } else {
                        painter.circle(p, rad, fill, stroke);
                    }
                }
            }
            for (k, line) in self.doc.lines.iter().enumerate() {
                for i in 0..line.nodes.len() {
                    let r = NodeRef::Line(k, i);
                    let p = v.to_screen(edit::node_pos(&self.doc, r));
                    let rad = 4.5 + if hovered == Some(r) { 2.0 } else { 0.0 };
                    let fill = if sel_group.contains(&r) { Color32::from_rgb(255, 120, 40) } else { line_colour(&line.kind) };
                    painter.rect(egui::Rect::from_center_size(p, Vec2::splat(rad * 2.0)), 2.0, fill, Stroke::new(1.0, Color32::BLACK), egui::StrokeKind::Middle);
                }
            }
            for (k, path) in self.doc.paths.iter().enumerate() {
                for (i, n) in path.nodes.iter().enumerate() {
                    let r = NodeRef::Path(k, i);
                    let p = v.to_screen(edit::node_pos(&self.doc, r));
                    let rad = 5.5 + if hovered == Some(r) { 2.0 } else { 0.0 };
                    let fill = if sel_group.contains(&r) { Color32::from_rgb(255, 120, 40) } else { Color32::from_rgb(235, 130, 255) };
                    let pts = vec![p + Vec2::new(0.0, -rad), p + Vec2::new(rad, 0.0), p + Vec2::new(0.0, rad), p + Vec2::new(-rad, 0.0)];
                    painter.add(Shape::convex_polygon(pts, fill, Stroke::new(1.0, Color32::BLACK)));
                    if let Some(z) = n.get(2).copied().flatten() {
                        painter.text(p + Vec2::new(8.0, -8.0), Align2::LEFT_BOTTOM, format!("z {z:.0}"), FontId::proportional(11.0), Color32::from_rgb(245, 200, 255));
                    }
                }
            }
        }
        // the ghost of a wall piece about to be put down: where it fits, or why it can't
        if let (Tool::Prop, true, Some(h), Gesture::None) = (self.tool, self.chosen_on_wall(), hover, &self.gesture) {
            let probe = Prop { piece: self.piece.clone(), at: v.to_world(h), z: None, yaw: 0.0, scale: [1.0; 3] };
            let piece = self.kit.as_ref().and_then(|k| k.get(&self.piece)).cloned();
            match (self.ghost(&probe), piece) {
                (Some(Ok(pv)), Some(piece)) => {
                    let pts: Vec<Pos2> = overworld::props::footprint(&piece, &pv.prop).iter().map(|&q| v.to_screen(q)).collect();
                    painter.add(Shape::closed_line(pts, Stroke::new(2.5, PROP_COLOUR)));
                    let f = overworld::props::facing(pv.prop.yaw);
                    painter.arrow(v.to_screen(pv.prop.at), Vec2::new(f[0] as f32, -f[1] as f32) * 18.0, Stroke::new(2.0, PROP_COLOUR));
                }
                (Some(Err(e)), _) => {
                    let e = e.split_once(": ").map_or(e.as_str(), |x| x.1).to_string();
                    painter.text(h + Vec2::new(14.0, 0.0), Align2::LEFT_BOTTOM, e, FontId::proportional(12.0), Color32::from_rgb(255, 110, 90));
                }
                _ => {}
            }
        }
        // drawing in progress
        if !self.drawing.is_empty() {
            let col = match self.tool {
                Tool::Region => Color32::from_rgb(255, 230, 120),
                Tool::Line(kind) => line_colour(kind),
                _ => Color32::from_rgb(235, 130, 255),
            };
            let mut pts: Vec<Pos2> = self.drawing.iter().map(|&p| v.to_screen(p)).collect();
            if let Some(h) = hover {
                pts.push(h);
            }
            painter.add(Shape::line(pts.clone(), Stroke::new(2.0, col)));
            for p in &pts[..self.drawing.len()] {
                painter.circle_filled(*p, 4.0, col);
            }
            if matches!(self.tool, Tool::Region | Tool::Line("hedge")) && self.drawing.len() >= 3 {
                let first = v.to_screen(self.drawing[0]);
                let close = hover.is_some_and(|h| h.distance(first) as f64 <= PICK);
                painter.circle_stroke(first, if close { 10.0 } else { 7.0 }, Stroke::new(2.0, col));
            }
        }
        // the brush
        if let (Tool::Brush, Some(h)) = (self.tool, hover) {
            let col = brush_colour(self.brush.mode);
            painter.circle_stroke(h, (self.brush.radius * v.scale) as f32, Stroke::new(2.0, col));
            painter.circle_stroke(h, (self.brush.radius * self.brush.falloff * v.scale) as f32, Stroke::new(1.0, col));
        }
        // where the profile's pointer is
        if let Some(p) = self.profile_hover {
            let s = v.to_screen(p);
            painter.circle_stroke(s, 9.0, Stroke::new(2.5, Color32::WHITE));
            painter.circle_filled(s, 3.0, Color32::WHITE);
        }
        // where the build failed
        if let Some(e) = &self.build_error {
            if let Some(p) = error_point(e) {
                let s = v.to_screen(p);
                painter.circle_stroke(s, 14.0, Stroke::new(3.0, Color32::from_rgb(255, 60, 60)));
                painter.circle_stroke(s, 22.0, Stroke::new(1.5, Color32::from_rgb(255, 60, 60)));
            }
        }
        // scale bar
        let target = 140.0 / v.scale;
        let len = [10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0].into_iter().rfind(|&l| l <= target).unwrap_or(10.0);
        let a = rect.left_bottom() + Vec2::new(16.0, -18.0);
        let b = a + Vec2::new((len * v.scale) as f32, 0.0);
        painter.line_segment([a, b], Stroke::new(3.0, Color32::WHITE));
        painter.line_segment([a + Vec2::new(0.0, -5.0), a + Vec2::new(0.0, 5.0)], Stroke::new(2.0, Color32::WHITE));
        painter.line_segment([b + Vec2::new(0.0, -5.0), b + Vec2::new(0.0, 5.0)], Stroke::new(2.0, Color32::WHITE));
        painter.text(a + Vec2::new(0.0, -8.0), Align2::LEFT_BOTTOM, format!("{len:.0} units"), FontId::proportional(12.0), Color32::WHITE);
    }

    // ---- panels ---------------------------------------------------------------------------

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("New").clicked() {
                self.new_doc();
            }
            if ui.button("Open…").clicked() {
                self.open();
            }
            if ui.button("Save").clicked() {
                self.save(false);
            }
            if ui.button("Save as…").clicked() {
                self.save(true);
            }
            ui.separator();
            for (t, name, key) in [
                (Tool::Select, "Select", "V"),
                (Tool::Region, "Region", "R"),
                (Tool::Path, "Path", "P"),
                (Tool::Brush, "Brush", "B"),
                (Tool::Prop, "Prop", "K"),
                (Tool::Line("dirt"), "Dirt", "D"),
                (Tool::Line("fence"), "Fence", "G"),
                (Tool::Line("bridge"), "Bridge", "H"),
                (Tool::Line("hedge"), "Hedge", "J"),
            ] {
                if ui.selectable_label(self.tool == t, name).on_hover_text(format!("key {key}")).clicked() {
                    self.set_tool(t);
                }
            }
            ui.separator();
            for (s, name) in [(Shading::Textured, "Textured"), (Shading::Height, "Heights"), (Shading::Off, "Lines")] {
                if ui.selectable_label(self.shading == s, name).on_hover_text("key T cycles").clicked() {
                    self.shading = s;
                }
            }
            ui.checkbox(&mut self.show_nodes, "Nodes");
            ui.separator();
            for (l, name) in [(Layout::Plan, "Plan"), (Layout::Split, "Plan + 3D"), (Layout::ThreeD, "3D")] {
                if ui.add_enabled(l == Layout::Plan || self.v3.is_some(), egui::Button::selectable(self.layout == l, name)).clicked() {
                    self.layout = l;
                }
            }
            if ui.button("Fit").on_hover_text("key F").clicked() {
                self.fit();
            }
            ui.separator();
            if ui.button("▶ Play").on_hover_text("Play the level in the game with child Link (W plays from the cursor; right-click: play from here)").clicked() {
                self.play(None);
            }
            ui.separator();
            if self.building() {
                ui.spinner();
                ui.label("building");
            } else if self.build_error.is_some() {
                ui.colored_label(Color32::from_rgb(255, 90, 90), "build failed");
            } else if self.level.is_some() {
                ui.label(format!("built in {:.0} ms", self.build_ms));
            }
        });
    }

    fn bottom_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(c) = self.cursor {
                let z = self.scene.as_ref().and_then(|s| s.floor_z(c));
                ui.monospace(format!("x {:6.0}  y {:6.0}  floor {}", c[0], c[1], z.map_or("-".into(), |z| format!("{z:.0}"))));
                ui.separator();
            }
            let hint = match self.tool {
                Tool::Select => "drag nodes (onto another node to share it; Alt: no snap) · double-click a line: add node · Del: delete · S: sharp · PgUp/PgDn: raise/sink · right-drag: pan · wheel: zoom",
                Tool::Region => "click points (on a node or edge to share it) · click the first point or Enter to close · Backspace: undo point · Esc: cancel",
                Tool::Path => "click points · put the ends inside the floors they start and finish on (a ramp ending inside a plateau cuts into it) · double-click or Enter to finish · Esc: cancel",
                Tool::Brush => "drag to paint · Ctrl: lower · Shift: smooth · [ ]: size · 1-6: raise, lower, smooth, flatten, bumps, erase · right-drag: pan",
                Tool::Prop if self.chosen_on_wall() => "hover a wall (best in 3D): the ghost shows where it goes, or why it can't · click: put it there · drag one along the walls to move it · Del: delete",
                Tool::Prop => "click: place the Kit panel's piece · drag a prop: move · its arrow's handle: turn (Q / E) · its corner: scale · Alt: no snap · Ctrl+D: duplicate · Del: delete",
                Tool::Line("hedge") => "click its corners (straight between them) · click the first point or Enter to close · Backspace: undo point · Esc: cancel",
                Tool::Line(_) => "click points along it · double-click or Enter to finish · Backspace: undo point · Esc: cancel",
            };
            ui.label(hint);
        });
        if !self.status.is_empty() {
            ui.label(&self.status);
        }
    }

    fn side_panel(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.tool == Tool::Brush {
                egui::CollapsingHeader::new("Brush").default_open(true).show(ui, |ui| self.brush_ui(ui));
            }
            if self.tool == Tool::Prop {
                egui::CollapsingHeader::new("Kit").default_open(true).show(ui, |ui| self.kit_ui(ui));
            }
            egui::CollapsingHeader::new("Selection").default_open(true).show(ui, |ui| self.selection_ui(ui));
            egui::CollapsingHeader::new("Contents").default_open(true).show(ui, |ui| self.contents_ui(ui));
            egui::CollapsingHeader::new("Build").default_open(true).show(ui, |ui| self.build_ui(ui));
            egui::CollapsingHeader::new("Level settings").show(ui, |ui| self.level_ui(ui));
            egui::CollapsingHeader::new("Files").show(ui, |ui| self.files_ui(ui));
            egui::CollapsingHeader::new("Keys").show(ui, |ui| {
                ui.label(KEYS);
            });
        });
    }

    fn brush_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (m, name, key) in [(Mode::Raise, "Raise", 1), (Mode::Lower, "Lower", 2), (Mode::Smooth, "Smooth", 3), (Mode::Flatten, "Flatten", 4), (Mode::Bumps, "Bumps", 5), (Mode::Erase, "Erase", 6)] {
                if ui.selectable_label(self.brush.mode == m, name).on_hover_text(format!("key {key}")).clicked() {
                    self.brush.mode = m;
                }
            }
        });
        let b = &mut self.brush;
        egui::Grid::new("brush").num_columns(2).show(ui, |ui| {
            ui.label("Size").on_hover_text("Radius ([ and ])");
            ui.add(egui::DragValue::new(&mut b.radius).speed(5.0).range(30.0..=5000.0));
            ui.end_row();
            ui.label("Strength").on_hover_text("Raise, lower, bumps: units per second at the middle. Others: how fast they act");
            ui.add(egui::DragValue::new(&mut b.strength).speed(1.0).range(1.0..=2000.0));
            ui.end_row();
            ui.label("Hard core").on_hover_text("The share of the radius at full strength");
            ui.add(egui::Slider::new(&mut b.falloff, 0.0..=0.95));
            ui.end_row();
            if b.mode == Mode::Bumps {
                ui.label("Bump size");
                ui.add(egui::DragValue::new(&mut b.bump_scale).speed(5.0).range(50.0..=5000.0));
                ui.end_row();
                ui.label("Seed");
                ui.add(egui::DragValue::new(&mut b.seed));
                ui.end_row();
            }
        });
        ui.weak("The brush paints one smooth height offset over everything: regions, paths and bridges ride on it, ponds stay level. Region heights stay as they are.");
        if self.doc.terrain.is_some() && ui.button("Clear all painted terrain").clicked() {
            self.doc.terrain = None;
        }
    }

    fn selection_ui(&mut self, ui: &mut egui::Ui) {
        if !self.multi.is_empty() {
            let ls = self.selected_loops();
            let names: Vec<String> = ls.iter().map(|&l| edit::loop_name(&self.doc, l)).collect();
            ui.strong(format!("{} selected: {}", ls.len(), names.join(", ")));
            let mut dz = 0.0;
            ui.horizontal(|ui| {
                ui.label("All together");
                if ui.button("-20").clicked() {
                    dz = -20.0;
                }
                if ui.button("+20").clicked() {
                    dz = 20.0;
                }
            });
            if dz != 0.0 {
                self.raise(dz);
            }
            if ui.button("Delete these regions").clicked() {
                self.delete_selection();
                return;
            }
            ui.separator();
        }
        match self.sel {
            Sel::None => {
                ui.label("Nothing selected. Click a region, path or node.");
            }
            Sel::Node(r) => {
                self.node_ui(ui, r);
                ui.separator();
                match r {
                    NodeRef::Loop(l, _) => self.loop_ui(ui, l),
                    NodeRef::Path(p, _) => self.path_ui(ui, p),
                    NodeRef::Line(k, _) => self.line_ui(ui, k),
                }
            }
            Sel::Loop(l) => self.loop_ui(ui, l),
            Sel::Path(p) => self.path_ui(ui, p),
            Sel::Prop(i) => self.prop_ui(ui, i),
            Sel::Line(k) => self.line_ui(ui, k),
        }
    }

    fn line_ui(&mut self, ui: &mut egui::Ui, k: usize) {
        let default_width = match self.doc.lines[k].kind.as_str() {
            "dirt" => self.theme.dirt.as_ref().map(|d| d.width),
            "bridge" => self.theme.hanging.as_ref().map(|h| h.width),
            _ => None,
        };
        let l = &mut self.doc.lines[k];
        ui.horizontal(|ui| {
            ui.strong(match l.kind.as_str() {
                "dirt" => "Dirt path",
                "fence" | "lattice" => "Fence",
                "bridge" => "Hanging bridge",
                "hedge" => "Hedge",
                k => k,
            });
            ui.text_edit_singleline(&mut l.name);
        });
        if l.kind == "fence" || l.kind == "lattice" {
            ui.horizontal(|ui| {
                ui.label("Style");
                ui.selectable_value(&mut l.kind, "fence".to_string(), "rails (40)").on_hover_text("The training ground's: 40 tall, a post every 40");
                ui.selectable_value(&mut l.kind, "lattice".to_string(), "lattice (120)").on_hover_text("The tall lattice by the crawlspace");
            });
            ui.checkbox(&mut l.closed, "Closed (back to the first node)");
            ui.weak("Straight between nodes, a post at every node, standing on the ground; collides from both sides.");
        }
        if let Some(dw) = default_width {
            ui.horizontal(|ui| {
                let mut own = l.width.is_some();
                if ui.checkbox(&mut own, "Width").on_hover_text("Off: the theme's").changed() {
                    l.width = own.then_some(dw);
                }
                match &mut l.width {
                    Some(w) => {
                        ui.add(egui::DragValue::new(w).speed(1.0).range(20.0..=2000.0));
                    }
                    None => {
                        ui.weak(format!("{dw:.0}"));
                    }
                }
            });
        }
        if l.kind == "hedge" {
            let h = self.theme.hedge.as_ref().map_or(28.0, |h| h.height);
            ui.weak(format!(
                "Tall grass over the shape of its nodes (straight between them), {h:.0} above the ground with grass skirts round it. Link wades through it, with tall-grass footsteps."
            ));
        }
        if l.kind == "dirt" {
            ui.weak("Painted into the floor: the ground under it fades to dirt across its soft edge (the theme's), and the middle has dirt footsteps.");
        }
        if l.kind == "bridge" {
            let sag = self.theme.hanging.as_ref().map_or(0.0, |h| h.sag * 100.0);
            ui.weak(format!(
                "Planks and ropes between its nodes, sagging {sag:.0}% of each span. Put its ends on the floors it joins: they land at the floors' edges. A deck too steep to walk is reported."
            ));
        }
        ui.label(format!("{} nodes · double-click it to add one", l.nodes.len()));
        if ui.button("Delete").clicked() {
            self.sel = Sel::Line(k);
            self.delete_selection();
        }
    }

    /// The kit's pieces, by kind: the one the Prop tool places.
    fn kit_ui(&mut self, ui: &mut egui::Ui) {
        let Some(kit) = self.kit.clone() else {
            ui.label("No kit. It's cut from the extracted Kokiri Forest (extracted/scenes/overworld/spot04) by `overworld kit-pieces`, into out/overworld/kit/kokiri.");
            return;
        };
        // hedges are drawn as shapes (the Hedge tool), not dropped in
        let mut kinds: Vec<&str> = kit.pieces.iter().map(|p| p.kind.as_str()).filter(|&k| k != "hedge").collect();
        kinds.dedup();
        for kind in kinds {
            ui.label(egui::RichText::new(kind_name(kind)).weak());
            ui.horizontal_wrapped(|ui| {
                for p in kit.pieces.iter().filter(|p| p.kind == kind) {
                    let tip = format!(
                        "{:.0} x {:.0} x {:.0} · {} triangles · {} collision vertices{}{}",
                        p.bounds[1][0] - p.bounds[0][0],
                        p.bounds[1][1] - p.bounds[0][1],
                        p.bounds[1][2] - p.bounds[0][2],
                        p.tris.len(),
                        p.collision_vertices(),
                        if p.scale.locked() { " · fixed size" } else { "" },
                        if p.about.is_empty() { String::new() } else { format!("\n{}", p.about) }
                    );
                    if ui.selectable_label(self.piece == p.name, &p.label).on_hover_text(tip).clicked() {
                        self.piece = p.name.clone();
                    }
                }
            });
        }
        ui.horizontal(|ui| {
            ui.label("Turn new props");
            ui.add(egui::DragValue::new(&mut self.place_yaw).speed(1.0).range(-180.0..=180.0).suffix("°"));
        });
        ui.weak("Click in the plan or 3D to place. Openings and wall pieces go best in 3D: hover a wall and a ghost shows where it'd go. Hedges: draw them with the Hedge tool (J). Doors and the log tunnel's exit are scenery until levels load through Play_Init (ADR 0035's next step).");
    }

    fn prop_ui(&mut self, ui: &mut egui::Ui, i: usize) {
        let built_z = self.prop_z(i);
        let fitted = self.on_wall(i);
        let placed = self.level.as_ref().and_then(|l| l.props.iter().find(|pl| pl.index == i).cloned());
        let tag = format!("prop {i}: ");
        let trouble: Vec<String> = self.problems.iter().filter(|p| p.starts_with(&tag)).map(|p| p[tag.len()..].to_string()).collect();
        let piece = self.kit.as_ref().and_then(|k| k.get(&self.doc.props[i].piece)).cloned();
        // a wall piece's flat face where it is (asked at its narrowest, which fits wherever any does)
        let room = piece.as_ref().filter(|p| p.kind == "wall").and_then(|p| {
            let probe = Prop { scale: [p.scale.min[0], 1.0, 1.0], ..self.doc.props[i].clone() };
            self.ghost(&probe)?.ok().map(|pv| pv.room)
        });
        let prop = &mut self.doc.props[i];
        ui.horizontal(|ui| {
            ui.strong(piece.as_ref().map_or(prop.piece.clone(), |p| p.label.clone()));
            if piece.is_none() {
                ui.colored_label(Color32::from_rgb(255, 90, 90), "not in the kit");
            }
        });
        egui::Grid::new("prop").num_columns(2).show(ui, |ui| {
            ui.label("Position");
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut prop.at[0]).speed(2.0).prefix("x "));
                ui.add(egui::DragValue::new(&mut prop.at[1]).speed(2.0).prefix("y "));
            });
            ui.end_row();
            if fitted {
                ui.label("Fitted");
                ui.label(match &placed {
                    Some(pl) => format!("on the wall at {:.0}, {:.0}, facing {:.0}°, depth x{:.2}", pl.origin[0], pl.origin[1], pl.yaw, pl.scale[1]),
                    None => "not yet (see below)".into(),
                });
                ui.end_row();
                if let Some(p) = piece.as_ref().filter(|p| p.kind == "wall") {
                    let base = (p.bounds[1][0] - p.bounds[0][0]).max(1.0);
                    let lo = base * p.scale.min[0];
                    let hi = room.map_or(base * p.scale.max[0], |r| (r - 2.0 * overworld::openings::WALL_PIECE_MARGIN).min(base * p.scale.max[0])).max(lo);
                    let mut w = prop.scale[0].clamp(p.scale.min[0], p.scale.max[0]) * base;
                    ui.label("Width").on_hover_text("Across the wall: as wide as the flat face it's on allows");
                    ui.horizontal(|ui| {
                        if ui.add(egui::DragValue::new(&mut w).speed(1.0).range(lo..=hi)).changed() {
                            prop.scale[0] = (w / base * 1000.0).round() / 1000.0;
                        }
                        match room {
                            Some(r) => ui.weak(format!("the flat face here is {r:.0}: up to {hi:.0}")),
                            None => ui.weak("no flat face here"),
                        };
                    });
                    ui.end_row();
                    let tall = p.bounds[1][2] - p.bounds[0][2];
                    ui.label("Height").on_hover_text("Always from the floor to the wall's top");
                    ui.weak(match &placed {
                        Some(pl) => format!("floor to the wall's top: {:.0}", pl.scale[2] * tall),
                        None => "floor to the wall's top".into(),
                    });
                    ui.end_row();
                }
                return;
            }
            ui.label("Height").on_hover_text("On the ground: its base (a house's doorway, a stone's top) stands on the floor there");
            ui.horizontal(|ui| {
                let mut ground = prop.z.is_none();
                if ui.checkbox(&mut ground, "on the ground").changed() {
                    prop.z = if ground { None } else { Some(built_z.unwrap_or(0.0).round()) };
                }
                match &mut prop.z {
                    Some(z) => {
                        ui.add(egui::DragValue::new(z).speed(1.0));
                    }
                    None => {
                        ui.weak(built_z.map_or(String::new(), |z| format!("{z:.0}")));
                    }
                }
            });
            ui.end_row();
            ui.label("Turn").on_hover_text("Degrees counter-clockwise from north (Q / E: 15, with Shift 1)");
            ui.add(egui::DragValue::new(&mut prop.yaw).speed(1.0).range(-180.0..=180.0).suffix("°"));
            ui.end_row();
            if let Some(p) = &piece {
                let lim = p.scale.clone();
                ui.label("Scale");
                if lim.locked() {
                    ui.weak("fixed (Link must fit)");
                } else if lim.uniform {
                    let mut f = prop.scale[0];
                    if ui.add(egui::DragValue::new(&mut f).speed(0.01).range(lim.min[0]..=lim.max[0])).changed() {
                        prop.scale = [f; 3];
                    }
                } else {
                    ui.horizontal(|ui| {
                        for (k, name) in ["x", "y", "z"].iter().enumerate() {
                            ui.add_enabled(lim.min[k] < lim.max[k], egui::DragValue::new(&mut prop.scale[k]).speed(0.01).range(lim.min[k]..=lim.max[k]).prefix(format!("{name} ")));
                        }
                    });
                }
                ui.end_row();
            }
        });
        prop.at = prop.at.map(|x| (x * 100.0).round() / 100.0);
        if fitted {
            ui.weak("It fits itself to the wall nearest where you put it: on its face, facing out over the floor in front, slid along clear of corners. Drag it along the walls in 3D to move it.");
        }
        for t in &trouble {
            ui.colored_label(Color32::from_rgb(255, 200, 80), t);
        }
        if let Some(p) = &piece {
            ui.label(format!("{} triangles · {} collision vertices", p.tris.len(), p.collision_vertices()));
            if let Some(d) = &p.door {
                ui.weak(format!("Its door led to {} (exit {}). Scenery until levels load through Play_Init.", d.entrance, d.exit));
            }
            if let Some(o) = &p.opening {
                ui.weak(format!("Opening {:.0} wide, {:.0} high, {:.0} deep (down to {:.0}).", o.width, o.height, o.depth, o.min_depth));
            }
            if !p.functions.is_empty() {
                ui.weak(format!("Link can use: {}", p.functions.join(", ")));
            }
            if !p.about.is_empty() {
                ui.weak(&p.about);
            }
        }
        ui.horizontal(|ui| {
            if ui.button("Duplicate (Ctrl+D)").clicked() {
                self.duplicate_prop();
            }
            if ui.button("Delete (Del)").clicked() {
                self.delete_selection();
            }
        });
    }

    fn node_ui(&mut self, ui: &mut egui::Ui, r: NodeRef) {
        let g = edit::group(&self.doc, r);
        let mut p = edit::node_pos(&self.doc, r);
        ui.horizontal(|ui| {
            ui.strong("Node");
            ui.label("x");
            let a = ui.add(egui::DragValue::new(&mut p[0]).speed(2.0));
            ui.label("y");
            let b = ui.add(egui::DragValue::new(&mut p[1]).speed(2.0));
            if a.changed() || b.changed() {
                edit::move_group(&mut self.doc, &g, p);
            }
        });
        match r {
            NodeRef::Loop(..) => {
                let mut sharp = edit::is_sharp(&self.doc, &g);
                if ui.checkbox(&mut sharp, "Sharp corner (S)").changed() {
                    edit::set_sharp(&mut self.doc, &g, sharp);
                }
                if g.len() > 1 {
                    let names: Vec<String> = g
                        .iter()
                        .map(|q| match q {
                            NodeRef::Loop(l, _) => edit::loop_name(&self.doc, *l),
                            NodeRef::Path(k, _) => edit::path_name(&self.doc, *k),
                            NodeRef::Line(k, _) => edit::line_name(&self.doc, *k),
                        })
                        .collect();
                    ui.label(format!("Shared by {}", names.join(", ")));
                }
            }
            NodeRef::Path(k, i) => {
                let floor = self.scene.as_ref().and_then(|s| s.floor_z(p)).unwrap_or(edit::base_z(&self.doc, &self.shapes, p)).round();
                let n = &mut self.doc.paths[k].nodes[i];
                let mut z = n.get(2).copied().flatten();
                let mut w = n.get(3).copied().flatten();
                let width = self.doc.paths[k].width;
                let (mut zc, mut wc) = (z.is_some(), w.is_some());
                let mut changed = false;
                ui.horizontal(|ui| {
                    changed |= ui.checkbox(&mut zc, "Height").on_hover_text("Off: an end takes the floor's height, a middle node is interpolated").changed();
                    if zc {
                        let mut v = z.unwrap_or(floor);
                        changed |= ui.add(egui::DragValue::new(&mut v).speed(1.0)).changed();
                        z = Some(v);
                    } else {
                        z = None;
                    }
                });
                ui.horizontal(|ui| {
                    changed |= ui.checkbox(&mut wc, "Width").on_hover_text("Off: the path's width").changed();
                    if wc {
                        let mut v = w.unwrap_or(width);
                        changed |= ui.add(egui::DragValue::new(&mut v).speed(1.0).range(20.0..=2000.0)).changed();
                        w = Some(v);
                    } else {
                        w = None;
                    }
                });
                if changed {
                    let n = &mut self.doc.paths[k].nodes[i];
                    n.truncate(2);
                    if z.is_some() || w.is_some() {
                        n.push(z);
                    }
                    if w.is_some() {
                        n.push(w);
                    }
                }
            }
            NodeRef::Line(..) => {}
        }
        if ui.button("Delete node (Del)").clicked() {
            self.delete_selection();
        }
    }

    fn loop_ui(&mut self, ui: &mut egui::Ui, l: usize) {
        if l == 0 {
            ui.strong("Outline");
            ui.horizontal(|ui| {
                ui.label("Ground height");
                ui.add(egui::DragValue::new(&mut self.doc.outline.z).speed(1.0));
            });
            noise_ui(ui, &mut self.doc.outline.noise);
            ui.label(format!("{} nodes", self.doc.outline.nodes.len()));
            return;
        }
        let styles: Vec<String> = self.theme.wall_styles.keys().cloned().collect();
        let r = &mut self.doc.regions[l - 1];
        ui.horizontal(|ui| {
            ui.strong("Region");
            ui.text_edit_singleline(&mut r.name);
        });
        ui.horizontal(|ui| {
            ui.label("Kind");
            egui::ComboBox::from_id_salt("kind").selected_text(r.kind.clone()).show_ui(ui, |ui| {
                ui.selectable_value(&mut r.kind, "floor".to_string(), "floor");
                ui.selectable_value(&mut r.kind, "water".to_string(), "water");
            });
        });
        let mut dz = 0.0;
        ui.horizontal(|ui| {
            ui.label(if r.kind == "water" { "Bed" } else { "Height" });
            ui.add(egui::DragValue::new(&mut r.z).speed(1.0));
            if ui.button("-20").clicked() {
                dz = -20.0;
            }
            if ui.button("+20").clicked() {
                dz = 20.0;
            }
        });
        r.z += dz;
        if r.kind == "water" {
            ui.horizontal(|ui| {
                ui.label("Surface");
                let mut s = r.surface.unwrap_or(r.z + 60.0);
                if ui.add(egui::DragValue::new(&mut s).speed(1.0)).changed() {
                    r.surface = Some(s);
                }
            });
        }
        ui.horizontal(|ui| {
            ui.label("Edge");
            egui::ComboBox::from_id_salt("edge")
                .selected_text(r.edge.clone().unwrap_or("theme rules".into()))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut r.edge, None, "theme rules");
                    for s in &styles {
                        ui.selectable_value(&mut r.edge, Some(s.clone()), s);
                    }
                })
                .response
                .on_hover_text("The wall style of this region's own walls (where it's the higher side)");
        });
        noise_ui(ui, &mut r.noise);
        ui.label(format!("{} nodes", r.nodes.len()));
        if ui.button("Delete region").clicked() {
            self.sel = Sel::Loop(l);
            self.delete_selection();
        }
    }

    fn path_ui(&mut self, ui: &mut egui::Ui, k: usize) {
        let styles: Vec<String> = self.theme.wall_styles.keys().cloned().collect();
        let p = &mut self.doc.paths[k];
        ui.horizontal(|ui| {
            ui.strong("Path");
            ui.text_edit_singleline(&mut p.name);
        });
        ui.horizontal(|ui| {
            ui.label("Width");
            ui.add(egui::DragValue::new(&mut p.width).speed(1.0).range(20.0..=2000.0));
        });
        let segs = p.nodes.len().saturating_sub(1);
        let mut modes: Vec<String> = (0..segs).map(|i| p.modes.get(i).cloned().unwrap_or(p.mode.clone())).collect();
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Mode");
            let before = p.mode.clone();
            egui::ComboBox::from_id_salt("mode").selected_text(p.mode.clone()).show_ui(ui, |ui| {
                ui.selectable_value(&mut p.mode, "attached".to_string(), "attached (embankment)");
                ui.selectable_value(&mut p.mode, "floating".to_string(), "floating (bridge)");
            });
            if p.mode != before {
                p.modes.clear();
                modes = vec![p.mode.clone(); segs];
            }
        });
        if segs > 1 {
            ui.label("Segments");
            for (i, m) in modes.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("  {} to {}", i, i + 1));
                    egui::ComboBox::from_id_salt(("seg", i)).selected_text(m.clone()).show_ui(ui, |ui| {
                        changed |= ui.selectable_value(m, "attached".to_string(), "attached").changed();
                        changed |= ui.selectable_value(m, "floating".to_string(), "floating").changed();
                    });
                });
            }
        }
        if changed {
            p.modes = if modes.iter().all(|m| *m == p.mode) { vec![] } else { modes };
        }
        ui.horizontal(|ui| {
            ui.label("Sides");
            egui::ComboBox::from_id_salt("pedge").selected_text(p.edge.clone().unwrap_or("theme's".into())).show_ui(ui, |ui| {
                ui.selectable_value(&mut p.edge, None, "theme's");
                for s in &styles {
                    ui.selectable_value(&mut p.edge, Some(s.clone()), s);
                }
            });
        });
        ui.horizontal(|ui| {
            ui.label("Bridge shape");
            egui::ComboBox::from_id_salt("shape").selected_text(p.shape.clone().unwrap_or("theme's".into())).show_ui(ui, |ui| {
                ui.selectable_value(&mut p.shape, None, "theme's");
                ui.selectable_value(&mut p.shape, Some("rock".into()), "rock arch");
                ui.selectable_value(&mut p.shape, Some("slab".into()), "slab");
            });
        });
        ui.label(format!("{} nodes", p.nodes.len()));
        if ui.button("Delete path").clicked() {
            self.sel = Sel::Path(k);
            self.delete_selection();
        }
    }

    fn profile_ui(&mut self, ui: &mut egui::Ui, k: usize) {
        let selected = match self.sel {
            Sel::Node(NodeRef::Path(p, i)) if p == k => Some(i),
            _ => None,
        };
        let max_slope = self.theme.paths.as_ref().map_or(35.0, |p| p.max_slope);
        let out = self.profile.show(ui, &self.doc, &self.shapes, k, selected, max_slope);
        if let Some(i) = out.select {
            self.sel = Sel::Node(NodeRef::Path(k, i));
        }
        if let Some((i, z)) = out.set_z {
            let n = &mut self.doc.paths[k].nodes[i];
            n.resize(n.len().max(3), None);
            n[2] = z;
            while n.len() > 2 && n.last() == Some(&None) {
                n.pop();
            }
        }
        self.profile_hover = out.hover;
    }

    fn contents_ui(&mut self, ui: &mut egui::Ui) {
        let cur = match self.sel {
            Sel::Loop(l) | Sel::Node(NodeRef::Loop(l, _)) => Some(Sel::Loop(l)),
            Sel::Path(p) | Sel::Node(NodeRef::Path(p, _)) => Some(Sel::Path(p)),
            Sel::Prop(i) => Some(Sel::Prop(i)),
            Sel::Line(k) | Sel::Node(NodeRef::Line(k, _)) => Some(Sel::Line(k)),
            Sel::None => None,
        };
        let shift = ui.input(|i| i.modifiers.shift);
        let ls = self.selected_loops();
        let on = |l: usize| cur == Some(Sel::Loop(l)) || ls.contains(&l);
        if ui.selectable_label(on(0), format!("outline  {:.0}", self.doc.outline.z)).clicked() {
            self.click_select(Sel::Loop(0), shift);
        }
        for l in 1..edit::loop_count(&self.doc) {
            let r = &self.doc.regions[l - 1];
            let text = format!("{}  {:.0}{}", edit::loop_name(&self.doc, l), r.z, if r.kind == "water" { "  (water)" } else { "" });
            if ui.selectable_label(on(l), text).on_hover_text("Shift-click: select several").clicked() {
                self.click_select(Sel::Loop(l), shift);
            }
        }
        for k in 0..self.doc.paths.len() {
            if ui.selectable_label(cur == Some(Sel::Path(k)), format!("{}  ({})", edit::path_name(&self.doc, k), self.doc.paths[k].mode)).clicked() {
                self.click_select(Sel::Path(k), false);
            }
        }
        for k in 0..self.doc.lines.len() {
            if ui.selectable_label(cur == Some(Sel::Line(k)), format!("{}  ({})", edit::line_name(&self.doc, k), self.doc.lines[k].kind)).clicked() {
                self.click_select(Sel::Line(k), false);
            }
        }
        for i in 0..self.doc.props.len() {
            let p = &self.doc.props[i];
            let name = self.kit.as_ref().and_then(|k| k.get(&p.piece)).map_or(p.piece.clone(), |x| x.label.clone());
            if ui.selectable_label(cur == Some(Sel::Prop(i)), format!("{name}  at {:.0}, {:.0}", p.at[0], p.at[1])).clicked() {
                self.click_select(Sel::Prop(i), false);
            }
        }
    }

    fn build_ui(&mut self, ui: &mut egui::Ui) {
        if let Some(e) = &self.shapes.error {
            ui.colored_label(Color32::from_rgb(255, 90, 90), e);
        }
        if let Some(e) = &self.build_error {
            ui.colored_label(Color32::from_rgb(255, 90, 90), e);
            if self.level.is_some() {
                ui.label("Showing the last good build.");
            }
        }
        if let Some(s) = &self.scene {
            ui.label(format!("{} triangles · rim {:.0} to {:.0} · {:.0} ms", s.triangles, s.rim.0, s.rim.1, self.build_ms));
        }
        if let Some(l) = &self.level {
            let props: usize = l.props.iter().map(|p| p.collision_vertices).sum();
            let text = format!(
                "Collision: {} of {} vertices{}",
                l.collision_vertices,
                overworld::props::MAX_COLLISION_VERTICES,
                if props > 0 { format!(" (props about {props})") } else { String::new() }
            );
            let over = l.collision_vertices > overworld::props::MAX_COLLISION_VERTICES;
            let r = if over { ui.colored_label(Color32::from_rgb(255, 90, 90), text) } else { ui.label(text) };
            r.on_hover_text("The game indexes collision vertices in 13 bits. Over the limit the game refuses the level: build at a lower detail.");
        }
        if let Some(l) = &self.level {
            egui::CollapsingHeader::new(format!("Triangles ({} detail)", self.doc.settings.detail)).id_salt("tris").show(ui, |ui| {
                egui::Grid::new("tris_grid").num_columns(2).show(ui, |ui| {
                    let mut objs: Vec<(&str, usize)> = l.mesh.objects.iter().map(|o| (o.name.as_str(), o.tris.len())).collect();
                    objs.sort_by(|a, b| b.1.cmp(&a.1));
                    for (n, t) in objs {
                        ui.label(n);
                        ui.monospace(format!("{t:6}"));
                        ui.end_row();
                    }
                });
                ui.weak("For scale: Kokiri Forest is about 3,750 triangles in all (its village about 1,200), in a space about an eighth the size of the sketch levels.");
            });
        }
        for p in &self.problems {
            ui.colored_label(Color32::from_rgb(255, 200, 80), p);
        }
    }

    fn level_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Name");
            ui.text_edit_singleline(&mut self.doc.name);
        });
        ui.horizontal(|ui| {
            ui.label("Detail").on_hover_text(
                "Mesh resolution. High: curves every 'Curve sample' units, walls a band per texture repeat. \
                 Medium and Low: curves sampled by how much they bend, walls in three bands, coarser floors and bridges.",
            );
            for (d, name) in [("high", "High"), ("medium", "Medium"), ("low", "Low")] {
                if ui.selectable_label(self.doc.settings.detail == d, name).clicked() {
                    self.doc.settings.detail = d.into();
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Edges").on_hover_text(
                "How outlines, regions and paths run between their nodes. Smooth: curves.                  Faceted: the same curves in a few long straight pieces, low-poly like the game's own.                  Hard: straight from node to node, every node a corner.",
            );
            for (e, name) in [("smooth", "Smooth"), ("faceted", "Faceted"), ("hard", "Hard")] {
                let on = self.doc.settings.edges == e || (e == "smooth" && self.doc.settings.edges.is_empty());
                if ui.selectable_label(on, name).clicked() {
                    self.doc.settings.edges = e.into();
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Walls").on_hover_text(
                "How the cliffs are textured. Tiled: the grassy top and bottom keep their size and the rock between repeats, \
                 sharp on any wall. Middle stretched: the grassy top and bottom keep their size and the rock between is \
                 stretched once over the rest. Stretched: the texture once over the wall's height, growing across with it, \
                 as Kokiri Forest's own walls are: blurrier on tall walls, and walls of different heights don't quite meet.",
            );
            for (w, name) in [("tiled", "Tiled"), ("stretched_middle", "Middle stretched"), ("stretched", "Stretched")] {
                let on = self.doc.settings.wall_texture == w || (w == "tiled" && self.doc.settings.wall_texture.is_empty());
                if ui.selectable_label(on, name).clicked() {
                    self.doc.settings.wall_texture = w.into();
                }
            }
        });
        let b = &mut self.doc.boundary;
        ui.label("Edge of the world");
        egui::Grid::new("boundary").num_columns(2).show(ui, |ui| {
            let row = |ui: &mut egui::Ui, name: &str, tip: &str, v: &mut f64, speed: f64| {
                ui.label(name).on_hover_text(tip);
                ui.add(egui::DragValue::new(v).speed(speed).range(0.0..=100000.0));
                ui.end_row();
            };
            row(ui, "Cliff", "The rim stands at least this far above the floors at the edge", &mut b.cliff_min, 1.0);
            row(ui, "Bank depth", "How far the bank reaches back to the tree line", &mut b.bank, 1.0);
            row(ui, "Bank rise", "The bank's rise above the rim, over its depth", &mut b.bank_rise, 1.0);
            row(ui, "Rise slope", "How fast the rim may climb along the edge (1 in 4 = 0.25)", &mut b.rise_slope, 0.005);
            row(ui, "Reach", "Floors this close to the edge raise the rim", &mut b.reach, 1.0);
            row(ui, "Panel tolerance", "How far the tree line's straight panels may stray from the bank", &mut b.panel_tol, 1.0);
        });
        let s = &mut self.doc.settings;
        ui.label("Sampling");
        egui::Grid::new("settings").num_columns(2).show(ui, |ui| {
            ui.label("Curve sample").on_hover_text("Curves are sampled about this often");
            ui.add(egui::DragValue::new(&mut s.sample).speed(1.0).range(10.0..=1000.0));
            ui.end_row();
            ui.label("Floor points").on_hover_text("Interior points in floors, this far apart");
            ui.add(egui::DragValue::new(&mut s.steiner).speed(1.0).range(50.0..=5000.0));
            ui.end_row();
            ui.label("Weld").on_hover_text("Nodes closer than this are one node");
            ui.add(egui::DragValue::new(&mut s.weld).speed(0.1).range(0.0..=50.0));
            ui.end_row();
            ui.label("Seed").on_hover_text("Varies the texture and shade noise");
            ui.add(egui::DragValue::new(&mut s.seed));
            ui.end_row();
        });
    }

    fn files_ui(&mut self, ui: &mut egui::Ui) {
        ui.label(format!("Document: {}", self.file.as_ref().map_or("(unsaved)".into(), |f| f.display().to_string())));
        ui.horizontal(|ui| {
            ui.label(format!("Theme: {}", self.theme.name));
            if ui.button("Load…").clicked() {
                if let Some(p) = rfd::FileDialog::new().add_filter("theme", &["json"]).set_directory(Path::new(ROOT).join("crates/tools/overworld/themes")).pick_file() {
                    match Theme::load(&p.to_string_lossy()) {
                        Ok(t) => {
                            self.theme = Arc::new(t);
                            self.theme_file = Some(p);
                            self.sent = None;
                        }
                        Err(e) => self.status = e,
                    }
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label(format!("Textures: {}", self.lib.as_ref().map_or("(none)".into(), |l| l.dir.display().to_string())));
            if ui.button("Choose…").clicked() {
                if let Some(d) = rfd::FileDialog::new().pick_folder() {
                    match Library::load(&d) {
                        Ok(l) => {
                            self.lib = Some(Arc::new(l));
                            self.textures.clear();
                            self.sent = None;
                        }
                        Err(e) => self.status = e,
                    }
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label(format!("Export to: {}", self.out_dir.display()));
            if ui.button("Choose…").clicked() {
                if let Some(d) = rfd::FileDialog::new().pick_folder() {
                    self.out_dir = d;
                    self.sent = None;
                }
            }
        });
        if ui.checkbox(&mut self.live, "Export every build (the game reloads it)").changed() {
            self.sent = None;
        }
        if ui.button("Export now").clicked() {
            match (&self.level, &self.built) {
                (Some(l), Some(d)) => match export::write(d, &self.theme, l, self.lib.as_deref(), &self.out_dir) {
                    Ok(_) => self.status = format!("exported to {}", self.out_dir.display()),
                    Err(e) => self.status = e,
                },
                _ => self.status = "nothing built yet".into(),
            }
        }
    }
}

const PROP_COLOUR: Color32 = Color32::from_rgb(90, 235, 170);

fn line_colour(kind: &str) -> Color32 {
    match kind {
        "dirt" => Color32::from_rgb(235, 200, 110),
        "fence" | "lattice" => Color32::from_rgb(190, 130, 80),
        "bridge" => Color32::from_rgb(160, 210, 240),
        _ => Color32::from_rgb(200, 150, 100),
    }
}

fn kind_name(kind: &str) -> &str {
    match kind {
        "house" => "Houses",
        "tower" => "Stumps",
        "stone" => "Stepping stones",
        "hedge" => "Hedges",
        "opening" => "Openings (click near a wall: it's set into it)",
        "wall" => "On walls (click near a wall)",
        k => k,
    }
}

fn brush_colour(m: Mode) -> Color32 {
    match m {
        Mode::Raise => Color32::from_rgb(255, 200, 80),
        Mode::Lower => Color32::from_rgb(120, 180, 255),
        Mode::Smooth => Color32::from_rgb(200, 255, 200),
        Mode::Flatten => Color32::from_rgb(255, 255, 255),
        Mode::Bumps => Color32::from_rgb(255, 140, 220),
        Mode::Erase => Color32::from_rgb(255, 90, 90),
    }
}

/// A floor's bumps: on or off, and their settings.
fn noise_ui(ui: &mut egui::Ui, noise: &mut Option<overworld::doc::Noise>) {
    let mut on = noise.is_some();
    if ui.checkbox(&mut on, "Bumps").on_hover_text("Smooth noise on the floor, fading out towards its edges").changed() {
        *noise = on.then(overworld::doc::Noise::default);
    }
    if let Some(n) = noise {
        egui::Grid::new(ui.next_auto_id()).num_columns(2).show(ui, |ui| {
            ui.label("  Height").on_hover_text("Up to this far up or down");
            ui.add(egui::DragValue::new(&mut n.amplitude).speed(0.5).range(0.0..=500.0));
            ui.end_row();
            ui.label("  Size").on_hover_text("About how far apart the bumps are");
            ui.add(egui::DragValue::new(&mut n.scale).speed(5.0).range(50.0..=5000.0));
            ui.end_row();
            ui.label("  Edge fade").on_hover_text("Bumps fade out over this distance from the floor's edges, which keep its height");
            ui.add(egui::DragValue::new(&mut n.edge).speed(2.0).range(1.0..=3000.0));
            ui.end_row();
            ui.label("  Seed").on_hover_text("Another pattern");
            ui.add(egui::DragValue::new(&mut n.seed));
            ui.end_row();
        });
    }
}

const KEYS: &str = "\
V select · R draw region · P draw path · B brush · K props · D dirt path · G fence · H bridge · J hedge
Drag a node to move it (and every loop sharing it)
Drop a node on another to share it
Alt while dragging: no snapping
Double-click a line: add a node
Del / Backspace: delete node, region or path
S: sharp / smooth corner
Shift-click regions to select several
PgUp / PgDn or ] / [: raise / sink 20 (Shift: 100)
Brush: drag to paint · Ctrl: lower · Shift: smooth
  [ ]: size · 1-6: raise, lower, smooth, flatten, bumps, erase
Wheel: zoom · right or middle drag: pan
F: fit · T: textured / heights / lines · N: nodes
Props: drag to move · the arrow's handle turns, the corner scales
  Q / E: turn 15 (Shift: 1) · PgUp / PgDn: raise / sink
  Ctrl+D: duplicate · Alt while dragging: no snapping
W: play from the cursor (the game, child Link)
Ctrl+Z / Ctrl+Y: undo / redo · Ctrl+S: save";

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.receive_build();
        self.keys(&ctx);
        self.refresh_shapes();

        egui::Panel::top("top").show(ui, |ui| self.top_bar(ui));
        egui::Panel::bottom("bottom").show(ui, |ui| self.bottom_bar(ui));
        egui::Panel::left("side").resizable(true).default_size(320.0).show(ui, |ui| self.side_panel(ui));
        self.profile_hover = None;
        let profile_path = match self.sel {
            Sel::Path(k) | Sel::Node(NodeRef::Path(k, _)) => Some(k),
            _ => None,
        };
        if let Some(k) = profile_path {
            egui::Panel::bottom("profile").resizable(true).default_size(240.0).show(ui, |ui| self.profile_ui(ui, k));
        }
        egui::CentralPanel::no_frame().show(ui, |ui| {
            let r = ui.max_rect();
            match self.layout {
                Layout::Plan => self.canvas(ui),
                Layout::ThreeD => self.view3d_ui(ui),
                Layout::Split => {
                    let (a, b) = r.split_left_right_at_fraction(0.5);
                    self.canvas(&mut ui.new_child(egui::UiBuilder::new().max_rect(a.shrink2(Vec2::new(1.0, 0.0)))));
                    self.view3d_ui(&mut ui.new_child(egui::UiBuilder::new().max_rect(b.shrink2(Vec2::new(1.0, 0.0)))));
                }
            }
        });

        // edits settle into one undo step once the mouse is up and no field is being typed in
        if !ctx.input(|i| i.pointer.any_down()) && !ctx.egui_wants_keyboard_input() {
            self.commit();
        }
        self.fix_sel();
        let n = edit::loop_count(&self.doc);
        let primary = match self.sel {
            Sel::Loop(l) => Some(l),
            _ => None,
        };
        self.multi.retain(|&l| l < n && Some(l) != primary);
        if primary.is_none() {
            self.multi.clear();
        }
        self.send_build();

        let title = format!("{}{} — Overworld editor", self.file.as_ref().and_then(|f| f.file_name()).map_or(self.doc.name.clone(), |f| f.to_string_lossy().into()), if self.dirty() { " *" } else { "" });
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        if ctx.input(|i| i.viewport().close_requested()) && !self.confirm_discard() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
    }
}

fn load_doc(p: &Path) -> Result<Doc, String> {
    let s = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
    serde_json::from_str(&s).map_err(|e| format!("{}: {e}", p.display()))
}

fn out_dir_for(doc: &Doc) -> PathBuf {
    let name: String = doc.name.chars().map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect();
    Path::new(ROOT).join("out/overworld").join(if name.is_empty() { "untitled".into() } else { name })
}

/// The point a build error names ("... near (x, y) ...").
fn error_point(e: &str) -> Option<P2> {
    let i = e.find("near (")? + 6;
    let rest = &e[i..];
    let j = rest.find(')')?;
    let mut it = rest[..j].split(',').map(|s| s.trim().parse::<f64>());
    Some([it.next()?.ok()?, it.next()?.ok()?])
}
