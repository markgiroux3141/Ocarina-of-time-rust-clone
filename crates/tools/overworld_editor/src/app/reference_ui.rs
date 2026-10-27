//! The Reference mode: a dungeon of the game's (`crate::reference`) in a 3D view of its own, to
//! orbit one floor at a time while working out what the editor's tools need. A floor bar (the
//! pause map's floors) shows a floor's rooms with what's over it cut away, a height cut can be
//! dragged anywhere, and the room list shows, hides or isolates rooms. The collision can be drawn
//! coloured by what the game does with each surface (alone, or over the rooms), with the water
//! boxes; the scene's actors are drawn as the game draws them (their captures) with markers, the
//! doors between rooms marked with the rooms they join; the measure tool gives the distance and
//! the height between two points. The level being edited is left as it is underneath; the top
//! bar's Reference button goes back to it.

use super::actor_ui::group_colour;
use super::chrome::{glass, glass_button, overlay};
use super::style::{self, ACCENT, FAINT, MUTED, TEXT};
use super::widgets::{self, section};
use super::App;
use crate::actors::{Catalogue, Group};
use crate::captures::{Captures, Found};
use crate::reference::{self, Loaded, Surf, DUNGEONS, DUNGEON_COUNT};
use crate::view3d::{actor_transform, ActorDraw, Camera, Overlay, OverlayTri, View3d};
use eframe::egui::{self, Align, Align2, Color32, FontId, Key, Modifiers, PointerButton, Pos2, RichText, Shape, Stroke, Vec2};
use eframe::egui_wgpu;
use glam::Vec3;
use std::collections::HashSet;
use std::time::Instant;

/// Behind the dungeons: a dark slate, so unlit corners and the void read apart.
const BACKGROUND: [f64; 3] = [0.07, 0.08, 0.09];

/// The game draws its scenes at 20 frames a second: the draw config's animations go at that rate.
const GAME_FPS: f32 = 20.0;

/// How far from a marker (pixels) the pointer picks it.
const MARKER_PICK: f32 = 9.0;

/// The water boxes' colour.
const WATER: [u8; 4] = [70, 130, 235, 110];

/// What the view draws: the rooms as the game draws them, the collision alone, or the rooms with
/// the collision's telling surfaces over them.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Look {
    Rooms,
    Collision,
    Both,
}

/// An actor's group for the markers' colours and the panel's switches: the catalogue's, or none
/// for one the port hasn't got.
type Kind = Option<Group>;

fn kind_name(k: Kind) -> &'static str {
    k.map_or("Not ported", |g| g.title())
}

fn kind_colour(k: Kind) -> Color32 {
    k.map_or(Color32::from_rgb(150, 150, 150), group_colour)
}

pub struct Reference {
    pub on: bool,
    rs: Option<egui_wgpu::RenderState>,
    v3: Option<View3d>,
    pack: Option<Result<oot_game::pack::GamePack, String>>,
    loaded: Option<Loaded>,
    error: Option<String>,
    /// The floor shown (`Loaded::floors`), or every room.
    floor: Option<usize>,
    /// With a floor, the rooms of the floors under it too.
    below: bool,
    /// Nothing over this height (the game's y) is drawn.
    cut: Option<f32>,
    /// Rooms hidden in the room list, by index, and the one isolated.
    hidden: Vec<bool>,
    solo: Option<usize>,
    labels: bool,
    animate: bool,
    started: Instant,
    look: Look,
    water: bool,
    /// The surfaces the legend has switched off.
    surf_off: HashSet<Surf>,
    /// The overlays need making again (another look, dungeon or legend).
    overlay_dirty: bool,
    /// The actors' markers and models, and the groups switched off.
    markers: bool,
    models: bool,
    kinds_off: HashSet<Kind>,
    /// Each actor's capture, and the captures' generation and the catalogue's size they were found
    /// with.
    found: Vec<Option<Found>>,
    found_with: (u64, usize),
    /// The actor clicked, and the one under the pointer.
    picked: Option<usize>,
    hover_actor: Option<usize>,
    /// Measuring: the points clicked (the game's axes), two at most.
    measure: bool,
    points: Vec<Vec3>,
    /// The point under the pointer (the game's axes), the collision poly there, and the room
    /// list's row under it.
    hover: Option<Vec3>,
    hover_poly: Option<usize>,
    hover_room: Option<usize>,
    frame_pending: bool,
}

impl Reference {
    pub fn new(rs: Option<&egui_wgpu::RenderState>) -> Reference {
        Reference {
            on: false,
            rs: rs.cloned(),
            v3: None,
            pack: None,
            loaded: None,
            error: None,
            floor: None,
            below: false,
            cut: None,
            hidden: vec![],
            solo: None,
            labels: true,
            animate: true,
            started: Instant::now(),
            look: Look::Rooms,
            water: true,
            surf_off: HashSet::new(),
            overlay_dirty: true,
            markers: true,
            models: true,
            kinds_off: HashSet::new(),
            found: vec![],
            found_with: (u64::MAX, 0),
            picked: None,
            hover_actor: None,
            measure: false,
            points: vec![],
            hover: None,
            hover_poly: None,
            hover_room: None,
            frame_pending: false,
        }
    }

    /// Opens `DUNGEONS[i]`: every room shown, nothing cut, framed.
    fn open(&mut self, i: usize) {
        let pack = self.pack.get_or_insert_with(|| oot_game::pack::GamePack::open_default().map_err(|e| format!("the game's pack: {e:#}")));
        let pack = match pack {
            Ok(p) => p,
            Err(e) => {
                self.error = Some(e.clone());
                return;
            }
        };
        match reference::load(pack, i) {
            Ok(l) => {
                if self.v3.is_none() {
                    self.v3 = self.rs.as_ref().map(View3d::new);
                }
                if let Some(v) = &mut self.v3 {
                    v.set_rooms(&l);
                    v.background = Some(BACKGROUND);
                }
                self.hidden = vec![false; l.rooms.len()];
                self.solo = None;
                self.floor = None;
                self.cut = None;
                self.loaded = Some(l);
                self.error = None;
                self.frame_pending = true;
                self.overlay_dirty = true;
                self.found_with = (u64::MAX, 0);
                self.picked = None;
                self.points.clear();
            }
            Err(e) => self.error = Some(e),
        }
    }

    /// Which rooms are drawn, by index: not hidden, the isolated one if there is one, else the
    /// floor's (and the floors' under it, with `below`).
    fn shown(&self) -> Vec<bool> {
        let Some(l) = &self.loaded else { return vec![] };
        let n = l.rooms.iter().map(|r| r.index + 1).max().unwrap_or(0);
        (0..n)
            .map(|i| {
                if let Some(s) = self.solo {
                    return s == i;
                }
                if self.hidden.get(i).copied().unwrap_or(false) {
                    return false;
                }
                match self.floor {
                    None => true,
                    Some(f) => l.floors[f..].iter().take(if self.below { usize::MAX } else { 1 }).any(|fl| fl.rooms.contains(&i)),
                }
            })
            .collect()
    }

    /// Shows floor `f` (or everything): its rooms, cut over it.
    fn to_floor(&mut self, f: Option<usize>) {
        let Some(l) = &self.loaded else { return };
        self.floor = f.filter(|&f| f < l.floors.len());
        self.cut = self.floor.and_then(|f| l.floors[f].above);
        self.solo = None;
    }

    /// The floor over or under the one shown; from all of them, the top or the bottom one.
    fn step_floor(&mut self, up: bool) {
        let Some(l) = &self.loaded else { return };
        let n = l.floors.len();
        if n == 0 {
            return;
        }
        let f = match (self.floor, up) {
            (None, true) => n - 1,
            (None, false) => 0,
            (Some(f), true) => f.saturating_sub(1),
            (Some(f), false) => (f + 1).min(n - 1),
        };
        self.to_floor(Some(f));
    }

    /// The camera looking at the rooms shown (or at room `only`), from the south and above.
    fn frame(&mut self, only: Option<usize>, aspect: f32) {
        let shown = self.shown();
        let (Some(l), Some(v)) = (&self.loaded, &mut self.v3) else { return };
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for r in l.rooms.iter().filter(|r| r.tris > 0 && only.map_or(shown.get(r.index).copied().unwrap_or(false), |o| o == r.index)) {
            match self.cut {
                // what's left under the cut
                Some(c) => {
                    for p in r.opa.iter().chain(&r.xlu).flat_map(|d| &d.batches).flat_map(|b| &b.vertices).map(|v| v.pos).filter(|p| p.y <= c) {
                        lo = lo.min(p);
                        hi = hi.max(p);
                    }
                }
                None => {
                    lo = lo.min(r.lo);
                    hi = hi.max(r.hi);
                }
            }
        }
        if lo.x > hi.x {
            (lo, hi) = (l.lo, l.hi);
        }
        // from the south and above, far enough back that the box (up to the cut) fits the view
        // whichever way it's turned
        hi.y = self.cut.map_or(hi.y, |c| c.min(hi.y)).max(lo.y);
        v.cam.target = to_editor((lo + hi) * 0.5);
        v.cam.yaw = -90f32.to_radians();
        v.cam.pitch = 50f32.to_radians();
        // each corner of the box inside the view: across and up at its depth
        let c = &v.cam;
        let back = Vec3::new(c.pitch.cos() * c.yaw.cos(), c.pitch.cos() * c.yaw.sin(), c.pitch.sin());
        let fwd = -back;
        let right = fwd.cross(Vec3::Z).normalize();
        let up = right.cross(fwd);
        let tv = (crate::view3d::FOV.to_radians() * 0.5).tan();
        let th = tv * aspect.max(0.1);
        let (a, b) = (to_editor(lo), to_editor(hi));
        let mut dist: f32 = 400.0;
        for k in 0..8 {
            let corner = Vec3::new(if k & 1 == 0 { a.x } else { b.x }, if k & 2 == 0 { a.y } else { b.y }, if k & 4 == 0 { a.z } else { b.z }) - c.target;
            let depth = corner.dot(fwd);
            dist = dist.max(corner.dot(right).abs() / th - depth).max(corner.dot(up).abs() / tv - depth);
        }
        v.cam.dist = dist * 1.05;
    }

    /// The camera turned round a point (the game's axes), as near as it is.
    fn look_at(&mut self, p: Vec3) {
        if let Some(v) = &mut self.v3 {
            v.cam.target = to_editor(p);
            v.cam.dist = v.cam.dist.min(1500.0);
        }
    }

    /// Each actor's group (the catalogue's), and whether it's drawn: in a room shown, under the
    /// cut, its group not switched off.
    fn actor_kinds(&self, catalogue: &Catalogue, shown: &[bool]) -> (Vec<Kind>, Vec<bool>) {
        let Some(l) = &self.loaded else { return (vec![], vec![]) };
        let kinds: Vec<Kind> = l.actors.iter().map(|a| catalogue.get(&a.name).map(|e| e.group)).collect();
        let visible = l.actors.iter().zip(&kinds).map(|(a, k)| a.shown(shown) && self.cut.is_none_or(|c| a.pos.y <= c + 1.0) && !self.kinds_off.contains(k)).collect();
        (kinds, visible)
    }

    /// Everything the view draws this frame but its camera: the overlays (made again when they've
    /// changed), the actors' captures (found again when the captures are read again), the cut,
    /// the rooms shown and the draw config's animations at `seconds`.
    fn sync_view(&mut self, captures: &mut Captures, catalogue: &Catalogue, shown: &[bool], seconds: Option<f32>) {
        if self.overlay_dirty {
            self.make_overlays();
        }
        let with = (captures.generation, catalogue.entries.len());
        let (_, visible) = self.actor_kinds(catalogue, shown);
        let (Some(l), Some(v)) = (&mut self.loaded, &mut self.v3) else { return };
        if self.found_with != with {
            self.found = l.actors.iter().map(|a| catalogue.get(&a.name).and_then(|e| captures.find(&a.name, a.params, e.schema.setting_rot(a.rot)))).collect();
            self.found_with = with;
        }
        if let Some(t) = seconds {
            l.scene.run_draw_config((t * GAME_FPS) as u32);
            v.set_room_values(&l.scene, false);
        }
        let draws: Vec<ActorDraw> = if self.models {
            l.actors
                .iter()
                .enumerate()
                .filter(|(i, _)| visible[*i])
                .filter_map(|(i, a)| Some(ActorDraw { found: self.found.get(i)?.clone()?, transform: actor_transform(to_editor(a.pos), a.rot[1]) }))
                .collect()
        } else {
            vec![]
        };
        v.set_actors(captures.generation, &draws);
        v.cut = self.cut;
        v.rooms_shown = shown.to_vec();
        v.show_overlay = [self.look != Look::Rooms, self.water];
    }

    /// The collision and the water boxes for the view, as the look and the legend have them.
    fn make_overlays(&mut self) {
        let (Some(l), Some(v)) = (&self.loaded, &mut self.v3) else { return };
        let light = Vec3::new(0.35, 0.85, 0.4).normalize();
        let mut tris = vec![];
        if self.look != Look::Rooms {
            for p in l.polys.iter().filter(|p| !self.surf_off.contains(&p.surf) && (self.look == Look::Collision || p.surf.special())) {
                let c = p.surf.colour();
                // shaded by how it faces, floors checked every 100 units so sizes read
                let mut k = 0.45 + 0.55 * p.normal.dot(light).abs();
                if p.normal.y > 0.5 {
                    let m = (p.tri[0] + p.tri[1] + p.tri[2]) / 3.0;
                    if ((m.x / 100.0).floor() as i32 + (m.z / 100.0).floor() as i32) & 1 == 0 {
                        k *= 0.9;
                    }
                }
                let s = |x: u8| (x as f32 * k).min(255.0) as u8;
                let a = if self.look == Look::Both { 170 } else { 255 };
                tris.push(OverlayTri { room: p.room, tri: p.tri.map(to_editor), color: [s(c[0]), s(c[1]), s(c[2]), a] });
            }
        }
        v.set_overlay(Overlay::Collision, &tris);
        let mut water = vec![];
        for w in &l.water {
            let (a, b) = (w.lo, w.hi);
            let q = [Vec3::new(a.x, a.y, a.z), Vec3::new(b.x, a.y, a.z), Vec3::new(b.x, a.y, b.z), Vec3::new(a.x, a.y, b.z)].map(to_editor);
            // facing up: seen from above
            water.push(OverlayTri { room: w.room, tri: [q[0], q[2], q[1]], color: WATER });
            water.push(OverlayTri { room: w.room, tri: [q[0], q[3], q[2]], color: WATER });
        }
        v.set_overlay(Overlay::Water, &water);
        v.draw_rooms = self.look != Look::Collision;
        self.overlay_dirty = false;
    }
}

/// A point of the game's in the editor's axes (z up).
fn to_editor(p: Vec3) -> Vec3 {
    Vec3::new(p.x, -p.z, p.y)
}

/// A point of the editor's in the game's axes (y up).
fn to_game(p: Vec3) -> Vec3 {
    Vec3::new(p.x, p.z, -p.y)
}

/// A measure's readout: the distance, across (on the ground) and up.
fn measure_text(a: Vec3, b: Vec3) -> String {
    let d = b - a;
    format!("{:.0} · across {:.0} · up {:+.0}", d.length(), (d.x * d.x + d.z * d.z).sqrt(), d.y)
}

impl App {
    /// The Reference mode's keys: PageUp / PageDown the floor over or under, Home every floor,
    /// F frames what's shown, C the collision's look, M measures, Esc stops measuring.
    pub(super) fn reference_keys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let pressed = |k: Key| ctx.input_mut(|i| i.consume_key(Modifiers::NONE, k));
        let r = &mut self.reference;
        if pressed(Key::PageUp) {
            r.step_floor(true);
        }
        if pressed(Key::PageDown) {
            r.step_floor(false);
        }
        if pressed(Key::Home) {
            r.to_floor(None);
        }
        if pressed(Key::F) {
            r.frame_pending = true;
        }
        if pressed(Key::C) {
            r.look = match r.look {
                Look::Rooms => Look::Collision,
                Look::Collision => Look::Both,
                Look::Both => Look::Rooms,
            };
            r.overlay_dirty = true;
        }
        if pressed(Key::M) {
            r.measure = !r.measure;
            r.points.clear();
        }
        if pressed(Key::Escape) {
            if r.measure && !r.points.is_empty() {
                r.points.clear();
            } else {
                r.measure = false;
                r.picked = None;
            }
        }
        if ctx.input(|i| i.key_pressed(Key::Questionmark) || i.key_pressed(Key::F1)) {
            self.show_keys = !self.show_keys;
        }
    }

    /// The left panel: the dungeon, the floor and the cut, what's drawn, the surfaces, the actors,
    /// measuring, the rooms.
    pub(super) fn reference_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        let catalogue = &self.catalogue;
        let r = &mut self.reference;
        section(ui, "ref dungeon", "Dungeon", None, true, |ui| {
            let chosen = r.loaded.as_ref().map_or("Choose a dungeon…", |l| DUNGEONS[l.dungeon].name);
            let mut pick = None;
            egui::ComboBox::from_id_salt("ref dungeon").width(ui.available_width() - 4.0).selected_text(chosen).height(600.0).show_ui(ui, |ui| {
                ui.label(RichText::new("DUNGEONS").size(10.5).color(FAINT));
                for (i, d) in DUNGEONS.iter().enumerate() {
                    if i == DUNGEON_COUNT {
                        ui.separator();
                        ui.label(RichText::new("BOSSES").size(10.5).color(FAINT));
                    }
                    let on = r.loaded.as_ref().is_some_and(|l| l.dungeon == i);
                    if ui.selectable_label(on, d.name).on_hover_text(format!("{}_scene", d.scene)).clicked() {
                        pick = Some(i);
                    }
                }
            });
            if let Some(i) = pick {
                r.open(i);
            }
            if let Some(e) = &r.error {
                ui.add(egui::Label::new(RichText::new(e).size(12.0).color(style::BAD)).wrap());
            }
            if let Some(l) = &r.loaded {
                let tris: usize = l.rooms.iter().map(|r| r.tris).sum();
                ui.label(RichText::new(format!("{}_scene · {} rooms · {} triangles", DUNGEONS[l.dungeon].scene, l.rooms.len(), tris)).size(11.5).color(MUTED));
                ui.label(RichText::new(format!("{} collision polys · {} water boxes · {} actors", l.polys.len(), l.water.len(), l.actors.len())).size(11.5).color(MUTED));
                ui.label(RichText::new("Master Quest, as the ROM has it").size(11.0).color(FAINT));
            }
        });
        let Some(l) = &r.loaded else {
            ui.add_space(6.0);
            ui.label(RichText::new("Pick one of the game's dungeons to orbit it a floor at a time.").size(12.0).color(MUTED));
            return;
        };
        let (lo, hi) = (l.lo.y, l.hi.y);
        let floors = l.floors.clone();
        // what the sections list, so their buttons can change `r`
        let rooms: Vec<(usize, usize, Vec<usize>)> = l.rooms.iter().map(|x| (x.index, x.tris, x.floors.clone())).collect();
        let surf_counts: Vec<(Surf, usize)> = Surf::ALL.iter().map(|&k| (k, l.polys.iter().filter(|p| p.surf == k).count())).filter(|x| x.1 > 0).collect();
        let kinds: Vec<Kind> = l.actors.iter().map(|a| catalogue.get(&a.name).map(|e| e.group)).collect();
        let picked = r.picked.and_then(|i| l.actors.get(i).cloned().map(|a| (i, a)));
        section(ui, "ref floor", "Floor and cut", None, true, |ui| {
            if floors.is_empty() {
                ui.label(RichText::new("No pause map: no floors. Use the height cut.").size(12.0).color(MUTED));
            } else {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.0;
                    if ui.selectable_label(r.floor.is_none(), "All").on_hover_text("Every room, nothing cut (Home)").clicked() {
                        r.to_floor(None);
                    }
                    for (i, f) in floors.iter().enumerate() {
                        if ui.selectable_label(r.floor == Some(i), &f.name).on_hover_text(floor_tip(f)).clicked() {
                            r.to_floor(Some(i));
                        }
                    }
                });
                if widgets::toggle(ui, &mut r.below, "Floors below too").on_hover_text("With a floor, the rooms of the floors under it as well, so you see it over them").changed() {
                    r.solo = None;
                }
            }
            let mut on = r.cut.is_some();
            if widgets::toggle(ui, &mut on, "Height cut").on_hover_text("Nothing over this height is drawn (the game's y). A floor sets it to the floor over it's height").changed() {
                r.cut = on.then(|| r.cut.unwrap_or(hi));
            }
            if let Some(c) = &mut r.cut {
                ui.add(egui::Slider::new(c, lo.floor()..=hi.ceil()).step_by(10.0).suffix(" y"));
            }
        });
        section(ui, "ref show", "Show", None, true, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                for (look, name, tip) in [
                    (Look::Rooms, "Rooms", "The rooms as the game draws them"),
                    (Look::Collision, "Collision", "The collision alone, coloured by what the game does with each surface"),
                    (Look::Both, "Both", "The rooms, with the collision's telling surfaces over them (ladders, hookshot, exits...)"),
                ] {
                    if ui.selectable_label(r.look == look, name).on_hover_text(format!("{tip} (C: the next)")).clicked() {
                        r.look = look;
                        r.overlay_dirty = true;
                    }
                }
            });
            if widgets::toggle(ui, &mut r.water, "Water boxes").on_hover_text("The water boxes' surfaces, blue: where Link swims").changed() {
                r.overlay_dirty = true;
            }
            widgets::toggle(ui, &mut r.labels, "Room numbers").on_hover_text("Each room's number over it, and the rooms each door joins");
            widgets::toggle(ui, &mut r.animate, "Animate").on_hover_text("The scene's scrolling and changing textures, as the game runs them");
            if widgets::button(ui, None, "Frame what's shown", None, false).on_hover_text("F").clicked() {
                r.frame_pending = true;
            }
        });
        if r.look != Look::Rooms {
            section(ui, "ref surfaces", "Surfaces", Some(surf_counts.iter().map(|x| x.1).sum::<usize>().to_string()), true, |ui| {
                ui.label(RichText::new("Click one to hide or show it. Point at a surface: the status bar says all the game knows of it.").size(11.5).color(FAINT));
                for (k, n) in &surf_counts {
                    if r.look == Look::Both && !k.special() {
                        continue;
                    }
                    let on = !r.surf_off.contains(k);
                    let c = k.colour();
                    let resp = ui
                        .horizontal(|ui| {
                            let (rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 3.0, Color32::from_rgb(c[0], c[1], c[2]).gamma_multiply(if on { 1.0 } else { 0.25 }));
                            let resp = ui.add(egui::Label::new(RichText::new(k.name()).color(if on { TEXT } else { FAINT })).sense(egui::Sense::click()));
                            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| ui.label(RichText::new(n.to_string()).monospace().size(11.0).color(MUTED)));
                            resp
                        })
                        .inner;
                    if resp.on_hover_text(surf_tip(*k)).clicked() {
                        if on {
                            r.surf_off.insert(*k);
                        } else {
                            r.surf_off.remove(k);
                        }
                        r.overlay_dirty = true;
                    }
                }
            });
        }
        section(ui, "ref actors", "Actors", Some(kinds.len().to_string()), true, |ui| {
            ui.horizontal(|ui| {
                widgets::toggle(ui, &mut r.markers, "Markers").on_hover_text("A dot for each actor, in its group's colour; doors marked with the rooms they join");
                widgets::toggle(ui, &mut r.models, "Models").on_hover_text("The actors as the game draws them (their captures; doors, chests, platforms, enemies)");
            });
            let mut groups: Vec<Kind> = kinds.clone();
            groups.sort_by_key(|k| k.map_or(usize::MAX, |g| Group::ALL.iter().position(|x| *x == g).unwrap_or(99)));
            groups.dedup();
            for k in groups {
                let n = kinds.iter().filter(|x| **x == k).count();
                let on = !r.kinds_off.contains(&k);
                let resp = ui
                    .horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), egui::Sense::hover());
                        ui.painter().circle_filled(rect.center(), 5.0, kind_colour(k).gamma_multiply(if on { 1.0 } else { 0.25 }));
                        let resp = ui.add(egui::Label::new(RichText::new(kind_name(k)).color(if on { TEXT } else { FAINT })).sense(egui::Sense::click()));
                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| ui.label(RichText::new(n.to_string()).monospace().size(11.0).color(MUTED)));
                        resp
                    })
                    .inner;
                if resp.on_hover_text("Click: hide or show them").clicked() {
                    if on {
                        r.kinds_off.insert(k);
                    } else {
                        r.kinds_off.remove(&k);
                    }
                }
            }
            if let Some((i, a)) = &picked {
                ui.add_space(4.0);
                let e = catalogue.get(&a.name);
                egui::Frame::new().fill(style::PANEL2).corner_radius(6.0).inner_margin(egui::Margin::same(8)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let title = e.map_or(a.name.clone(), |e| format!("{} ({})", e.label, a.name));
                    ui.label(RichText::new(title).font(style::semibold(13.0)).color(TEXT));
                    let at = match (a.room, a.sides) {
                        (Some(r), _) => format!("room {r}"),
                        (None, Some([f, b])) => format!("a door: room {f} in front, room {b} behind"),
                        _ => String::new(),
                    };
                    for line in [
                        format!("{} · {at}", kind_name(kinds[*i])),
                        format!("actor {:#06x} · params {:#06x} · rotation {} {} {}", a.id, a.params, a.rot[0], a.rot[1], a.rot[2]),
                        format!("at x {:.0} y {:.0} z {:.0}", a.pos.x, a.pos.y, a.pos.z),
                    ] {
                        ui.label(RichText::new(line).size(11.5).color(MUTED));
                    }
                    if let Some(e) = e {
                        let about = e.about.split(". ").next().unwrap_or("");
                        if !about.is_empty() {
                            ui.add(egui::Label::new(RichText::new(about).size(11.5).color(FAINT)).wrap());
                        }
                    }
                    if r.found.get(*i).is_some_and(|f| f.is_none()) {
                        ui.label(RichText::new("No capture: drawn as a marker only").size(11.0).color(FAINT));
                    }
                    ui.horizontal(|ui| {
                        if widgets::button(ui, None, "Look at it", None, false).clicked() {
                            r.look_at(a.pos);
                        }
                        if widgets::button(ui, None, "Deselect", None, false).on_hover_text("Esc").clicked() {
                            r.picked = None;
                        }
                    });
                });
            } else {
                ui.label(RichText::new("Click a marker for what it is.").size(11.5).color(FAINT));
            }
        });
        section(ui, "ref measure", "Measure", None, true, |ui| {
            if ui.add(egui::Button::selectable(r.measure, "Measure  (M)")).on_hover_text("Click two points in the view: the distance between them, across and up. Esc clears").clicked() {
                r.measure = !r.measure;
                r.points.clear();
            }
            match r.points.as_slice() {
                [a, b] => {
                    ui.label(RichText::new(measure_text(*a, *b)).monospace().size(12.0).color(TEXT));
                    ui.label(RichText::new(format!("from {:.0} {:.0} {:.0}\nto   {:.0} {:.0} {:.0}", a.x, a.y, a.z, b.x, b.y, b.z)).monospace().size(11.0).color(MUTED));
                }
                [a] => {
                    ui.label(RichText::new(format!("from {:.0} {:.0} {:.0}: click the second point", a.x, a.y, a.z)).size(11.5).color(MUTED));
                }
                _ if r.measure => {
                    ui.label(RichText::new("Click the first point").size(11.5).color(MUTED));
                }
                _ => {}
            }
        });
        let shown = r.shown();
        let count = rooms.len();
        let mut frame_room = None;
        r.hover_room = None;
        section(ui, "ref rooms", "Rooms", Some(count.to_string()), true, |ui| {
            if r.solo.is_some() && widgets::button(ui, None, "Show them all again", None, false).clicked() {
                r.solo = None;
            }
            for (i, tris, on_floors) in &rooms {
                let i = *i;
                let resp = ui
                    .horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let mut vis = !r.hidden.get(i).copied().unwrap_or(false);
                        if ui.checkbox(&mut vis, "").on_hover_text("Show or hide it").changed() {
                            if let Some(h) = r.hidden.get_mut(i) {
                                *h = !vis;
                            }
                        }
                        let on = shown.get(i).copied().unwrap_or(false);
                        let name = RichText::new(format!("Room {i}")).color(if on { TEXT } else { FAINT });
                        if ui.add(egui::Label::new(name).sense(egui::Sense::click())).on_hover_text("Click: frame it").clicked() {
                            frame_room = Some(i);
                        }
                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                            let solo = r.solo == Some(i);
                            if ui.selectable_label(solo, "solo").on_hover_text("Only this room (click again: all of them)").clicked() {
                                r.solo = if solo { None } else { Some(i) };
                            }
                            let fl: Vec<&str> = on_floors.iter().map(|&f| floors[f].name.as_str()).collect();
                            ui.label(RichText::new(format!("{tris:>5} tris  {}", fl.join(" "))).monospace().size(11.0).color(MUTED));
                        });
                    })
                    .response;
                if resp.hovered() {
                    r.hover_room = Some(i);
                }
            }
        });
        if let Some(i) = frame_room {
            let aspect = r.v3.as_ref().map_or(1.0, |v| v.aspect);
            r.frame(Some(i), aspect);
        }
    }

    /// The status bar in the Reference mode: the point under the pointer, its floor and the
    /// surface there.
    pub(super) fn reference_status(&mut self, ui: &mut egui::Ui) {
        let r = &self.reference;
        ui.horizontal(|ui| {
            ui.label(RichText::new(if r.measure { "Measure" } else { "Reference" }).font(style::semibold(12.5)).color(ACCENT));
            ui.add_space(4.0);
            let hint = if r.measure {
                "click two points · Esc: clear · M: stop measuring"
            } else {
                "drag: orbit · right-drag: pan · wheel: zoom · double-click: look there · click a marker · PageUp/PageDown: floor · C: collision · M: measure · F: frame"
            };
            ui.add(egui::Label::new(RichText::new(hint).size(12.0).color(MUTED)).truncate());
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                let Some(l) = &r.loaded else { return };
                if let Some(p) = r.hover {
                    let fl = reference::floor_at(&l.floors, p.y).map_or(String::new(), |f| format!("  on {}", l.floors[f].name));
                    ui.label(RichText::new(format!("x {:6.0}  y {:6.0}  z {:6.0}{fl}", p.x, p.y, p.z)).monospace().size(12.0).color(MUTED));
                }
                if let Some(p) = r.hover_poly.and_then(|i| l.polys.get(i)) {
                    let mut s = p.bits.describe(p.normal.y);
                    if let Some(e) = (p.bits.exit as usize).checked_sub(1).and_then(|e| l.exits.get(e)) {
                        s = s.replacen(&format!("exit {}", p.bits.exit), &format!("exit {} (entrance {e:#06x})", p.bits.exit), 1);
                    }
                    let c = p.surf.colour();
                    ui.add(egui::Label::new(RichText::new(s).size(12.0).color(TEXT)).truncate());
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 2.0, Color32::from_rgb(c[0], c[1], c[2]));
                }
            });
        });
    }

    /// The 3D view of the dungeon, with the floor bar over it.
    pub(super) fn reference_view(&mut self, ui: &mut egui::Ui) {
        let (resp, painter) = super::fixed_painter(ui, "reference view");
        let rect = resp.rect;
        if rect.width() < 4.0 || rect.height() < 4.0 {
            return;
        }
        let ctx = ui.ctx().clone();
        let (r, captures, catalogue) = (&mut self.reference, &mut self.captures, &self.catalogue);
        if r.rs.is_none() {
            painter.text(rect.center(), Align2::CENTER_CENTER, "The 3D view needs the wgpu backend.", FontId::proportional(14.0), MUTED);
            return;
        }
        if r.loaded.is_none() {
            painter.rect_filled(rect, 0.0, Color32::from_rgb(18, 20, 23));
            painter.text(rect.center(), Align2::CENTER_CENTER, "Choose a dungeon on the left", FontId::proportional(15.0), MUTED);
            return;
        }
        let aspect = rect.width() / rect.height();
        if std::mem::take(&mut r.frame_pending) {
            r.frame(None, aspect);
        }
        let shown = r.shown();
        let (kinds, visible) = r.actor_kinds(catalogue, &shown);
        let cut = r.cut;
        let Some(mut cam) = r.v3.as_ref().map(|v| v.cam) else { return };
        let ray = |cam: &Camera, p: Pos2| {
            let ndc = [(p.x - rect.left()) / rect.width() * 2.0 - 1.0, 1.0 - (p.y - rect.top()) / rect.height() * 2.0];
            let (o, d) = cam.ray(aspect, ndc);
            (to_game(o), to_game(d))
        };
        let look = r.look;
        let l = r.loaded.as_ref().unwrap();
        // what's drawn under the pointer: the rooms' meshes, else (or with only the collision drawn)
        // the collision
        let hit = |cam: &Camera, p: Pos2| {
            let (o, d) = ray(cam, p);
            let mesh = if look == Look::Collision { None } else { l.raycast(&shown, o, d, cut) };
            mesh.or_else(|| l.raycast_collision(&shown, o, d, cut).map(|x| x.0)).map(|t| o + d * t)
        };
        // the markers on the screen, for picking
        let marker = |cam: &Camera, a: &reference::Actor| cam.project(rect, to_editor(a.pos + Vec3::Y * 8.0)).filter(|p| rect.contains(*p));
        let marker_at = |cam: &Camera, p: Pos2| -> Option<usize> {
            if !r.markers {
                return None;
            }
            l.actors
                .iter()
                .enumerate()
                .filter(|(i, _)| visible[*i])
                .filter_map(|(i, a)| marker(cam, a).map(|m| (i, m.distance(p))))
                .filter(|x| x.1 <= MARKER_PICK)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|x| x.0)
        };
        r.hover = None;
        r.hover_poly = None;
        r.hover_actor = None;
        if let Some(h) = resp.hover_pos() {
            r.hover = hit(&cam, h);
            let (o, d) = ray(&cam, h);
            r.hover_poly = l.raycast_collision(&shown, o, d, cut).map(|x| x.1);
            r.hover_actor = marker_at(&cam, h);
            let (scroll, zoom) = ctx.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
            let f = (-scroll * 0.0018).exp() / zoom;
            if (f - 1.0).abs() > 1e-6 {
                cam.zoom(f);
            }
        }
        if resp.dragged_by(PointerButton::Primary) || resp.dragged_by(PointerButton::Middle) {
            cam.orbit(resp.drag_delta());
        }
        if resp.dragged_by(PointerButton::Secondary) {
            cam.pan(resp.drag_delta(), rect.height());
        }
        if resp.clicked() {
            let p = resp.interact_pointer_pos();
            if r.measure {
                if let Some(q) = p.and_then(|p| hit(&cam, p)) {
                    if r.points.len() >= 2 {
                        r.points.clear();
                    }
                    r.points.push(q);
                }
            } else {
                r.picked = p.and_then(|p| marker_at(&cam, p));
            }
        }
        if resp.double_clicked() && !r.measure {
            if let Some(p) = resp.interact_pointer_pos().and_then(|p| hit(&cam, p)) {
                cam.target = to_editor(p);
            }
        }
        // the draw config's animations at the game's rate
        let seconds = r.animate.then(|| r.started.elapsed().as_secs_f32());
        if r.animate {
            ctx.request_repaint();
        }
        r.sync_view(captures, catalogue, &shown, seconds);
        let l = r.loaded.as_ref().unwrap();
        let marker = |cam: &Camera, a: &reference::Actor| cam.project(rect, to_editor(a.pos + Vec3::Y * 8.0)).filter(|p| rect.contains(*p));
        let v = r.v3.as_mut().unwrap();
        v.cam = cam;
        let ppp = ctx.pixels_per_point();
        let id = v.render((rect.width() * ppp) as u32, (rect.height() * ppp) as u32);
        painter.image(id, rect, egui::Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);

        // the room list's row under the pointer: that room's box
        if let Some(room) = r.hover_room.and_then(|i| l.rooms.iter().find(|x| x.index == i)).filter(|x| x.tris > 0) {
            let (a, b) = (room.lo, room.hi);
            let c = |x: f32, y: f32, z: f32| cam.project(rect, to_editor(Vec3::new(x, y, z)));
            let corners = [(a.x, a.z), (b.x, a.z), (b.x, b.z), (a.x, b.z)];
            let stroke = Stroke::new(1.5, ACCENT);
            for k in 0..4 {
                let (p, q) = (corners[k], corners[(k + 1) % 4]);
                for y in [a.y, b.y] {
                    if let (Some(s), Some(e)) = (c(p.0, y, p.1), c(q.0, y, q.1)) {
                        painter.line_segment([s, e], stroke);
                    }
                }
                if let (Some(s), Some(e)) = (c(p.0, a.y, p.1), c(p.0, b.y, p.1)) {
                    painter.line_segment([s, e], stroke);
                }
            }
        }
        let label = |p: Pos2, text: &str, colour: Color32| {
            let g = painter.layout_no_wrap(text.to_string(), FontId::proportional(12.0), colour);
            let bg = egui::Rect::from_center_size(p, g.size() + Vec2::new(8.0, 2.0));
            painter.rect_filled(bg, 4.0, Color32::from_rgba_unmultiplied(10, 12, 14, 200));
            painter.galley(p - g.size() / 2.0, g, colour);
        };
        if r.labels {
            for room in l.rooms.iter().filter(|x| x.tris > 0 && shown.get(x.index).copied().unwrap_or(false)) {
                let mid = (room.lo + room.hi) * 0.5;
                let y = cut.map_or(mid.y, |c| mid.y.min(c));
                if let Some(p) = cam.project(rect, to_editor(Vec3::new(mid.x, y, mid.z))).filter(|p| rect.contains(*p)) {
                    label(p, &room.index.to_string(), TEXT);
                }
            }
        }
        // the actors' markers: doors as diamonds with the rooms they join
        if r.markers {
            for (i, a) in l.actors.iter().enumerate().filter(|(i, _)| visible[*i]) {
                let Some(m) = marker(&cam, a) else { continue };
                let c = kind_colour(kinds[i]);
                let ring = if r.picked == Some(i) { Some(Stroke::new(2.0, Color32::WHITE)) } else if r.hover_actor == Some(i) { Some(Stroke::new(1.5, TEXT)) } else { None };
                if let Some([f, b]) = a.sides {
                    let d = 6.0;
                    let pts = vec![m + Vec2::new(0.0, -d), m + Vec2::new(d, 0.0), m + Vec2::new(0.0, d), m + Vec2::new(-d, 0.0)];
                    painter.add(Shape::convex_polygon(pts, c, ring.unwrap_or(Stroke::new(1.0, Color32::BLACK))));
                    if r.labels {
                        let side = |s: i8| if s < 0 { "-".to_string() } else { s.to_string() };
                        label(m + Vec2::new(0.0, -16.0), &format!("{}↔{}", side(f), side(b)), c);
                    }
                } else {
                    painter.circle(m, 4.5, c, ring.unwrap_or(Stroke::new(1.0, Color32::BLACK)));
                }
            }
            if let Some(a) = r.hover_actor.and_then(|i| l.actors.get(i)) {
                if let Some(m) = marker(&cam, a) {
                    let name = catalogue.get(&a.name).map_or(a.name.clone(), |e| format!("{} ({})", e.label, a.name));
                    label(m + Vec2::new(0.0, 18.0), &format!("{name} · {:#06x}", a.params), TEXT);
                }
            }
        }
        // the measure: its points and the line between them, with the readout
        let mpts: Vec<Pos2> = r.points.iter().filter_map(|p| cam.project(rect, to_editor(*p))).collect();
        if mpts.len() == 2 {
            painter.line_segment([mpts[0], mpts[1]], Stroke::new(2.0, style::WARN));
            if let [a, b] = r.points.as_slice() {
                // its rise drawn too: straight up from the lower end
                let (lo, hi) = if a.y < b.y { (*a, *b) } else { (*b, *a) };
                if let (Some(s), Some(e)) = (cam.project(rect, to_editor(lo)), cam.project(rect, to_editor(Vec3::new(lo.x, hi.y, lo.z)))) {
                    painter.line_segment([s, e], Stroke::new(1.0, style::WARN.gamma_multiply(0.6)));
                }
                label(mpts[0].lerp(mpts[1], 0.5) + Vec2::new(0.0, -14.0), &measure_text(*a, *b), style::WARN);
            }
        }
        for p in &mpts {
            painter.circle(*p, 4.0, style::WARN, Stroke::new(1.0, Color32::BLACK));
        }

        // the floor bar, top to bottom down the view's left side, as the pause map has them
        let floors = l.floors.clone();
        let mut pick: Option<Option<usize>> = None;
        if !floors.is_empty() {
            egui::Area::new(egui::Id::new(("overlay", "ref floors"))).pivot(Align2::LEFT_TOP).fixed_pos(rect.left_top() + Vec2::new(10.0, 10.0)).order(egui::Order::Middle).show(&ctx, |ui| {
                glass().show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    if glass_button(ui, "All", r.floor.is_none(), true).on_hover_text("Every room, nothing cut (Home)").clicked() {
                        pick = Some(None);
                    }
                    for (i, f) in floors.iter().enumerate() {
                        if glass_button(ui, &f.name, r.floor == Some(i), true).on_hover_text(floor_tip(f) + "\nPageUp/PageDown: the floor over or under").clicked() {
                            pick = Some(Some(i));
                        }
                    }
                });
            });
        }
        if let Some(f) = pick {
            r.to_floor(f);
        }
        let what = match (r.floor, r.cut) {
            (Some(f), Some(c)) => format!("{}: cut at y {c:.0}", floors[f].name),
            (Some(f), None) => format!("{}: the top floor, nothing over it cut", floors[f].name),
            (None, Some(c)) => format!("cut at y {c:.0}"),
            (None, None) => String::new(),
        };
        if !what.is_empty() {
            overlay(ui, "ref cut", Align2::LEFT_BOTTOM, rect.left_bottom() + Vec2::new(10.0, -10.0), |ui| {
                glass().show(ui, |ui| {
                    ui.label(RichText::new(what).size(12.0).color(TEXT));
                });
            });
        }
    }
}

/// A floor's tooltip: its height and its rooms.
fn floor_tip(f: &reference::Floor) -> String {
    let rooms: Vec<String> = f.rooms.iter().map(|r| r.to_string()).collect();
    let cut = f.above.map_or("the top floor".to_string(), |a| format!("cut at y {a:.0}"));
    format!("{}: Link is on it above y {:.0}; {cut}\nRooms on its map: {}", f.name, f.y, rooms.join(", "))
}

/// What the legend says of a surface.
fn surf_tip(k: Surf) -> &'static str {
    match k {
        Surf::Exit => "An exit to another scene (its entrance in the status bar)",
        Surf::Void => "Falling onto it voids out: back to where Link came in (floor type 9; floor properties 5, 12)",
        Surf::Hurts => "It hurts: hot floors (floor types 2, 3) and damaging walls",
        Surf::Hookshot => "The hookshot holds on it. Climbing surfaces that also take it are coloured as climbing",
        Surf::Ladder => "A ladder, or a ladder's top (wall types 2, 3)",
        Surf::Climb => "Climbable: vines, fences, grates (wall type 4)",
        Surf::Crawl => "A crawlspace's walls (wall types 5, 6)",
        Surf::Grab => "A wall to hold on to, to push or pull (wall type 7)",
        Surf::Sand => "Sand and quicksand: Link sinks (floor types 4, 7, 12)",
        Surf::Slippery => "Slippery: ice (floor effect 1)",
        Surf::Floor => "A plain floor",
        Surf::Wall => "A plain wall",
        Surf::Ceiling => "A ceiling",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui_wgpu::wgpu;
    use std::path::Path;

    /// Draws a few dungeons through the Reference mode's own view, headless (an egui render
    /// state without a window), into PNGs in `$REF_RENDER_DIR` (else the temp folder): the
    /// rooms, the collision alone and over the rooms, the actors' captures. The markers and labels
    /// (egui's painter) aren't in them. `cargo test -p overworld_editor --release render_reference_views
    /// -- --ignored`.
    #[test]
    #[ignore]
    fn render_reference_views() {
        if oot_game::pack::GamePack::open_default().is_err() {
            return;
        }
        let dir = std::env::var_os("REF_RENDER_DIR").map_or_else(std::env::temp_dir, std::path::PathBuf::from);
        let instance = wgpu::Instance::default();
        let rs = pollster::block_on(egui_wgpu::RenderState::create(&egui_wgpu::WgpuConfiguration::default(), &instance, None, egui_wgpu::RendererOptions::default())).unwrap();
        let root = Path::new(super::super::ROOT);
        let catalogue = Catalogue::load(root);
        let mut captures = Captures::new(root);
        let mut r = Reference::new(Some(&rs));
        let views = [
            ("ydan", Some("1F"), Look::Rooms),
            ("ydan", Some("1F"), Look::Both),
            ("ydan", None, Look::Collision),
            ("HAKAdan", Some("B2"), Look::Both),
            ("MIZUsin", None, Look::Collision),
            ("bdan", Some("1F"), Look::Rooms),
        ];
        for (scene, floor, look) in views {
            r.open(DUNGEONS.iter().position(|d| d.scene == scene).unwrap());
            r.look = look;
            r.overlay_dirty = true;
            let f = floor.and_then(|n| r.loaded.as_ref().unwrap().floors.iter().position(|f| f.name == n));
            r.to_floor(f);
            r.frame(None, 4.0 / 3.0);
            let shown = r.shown();
            r.sync_view(&mut captures, &catalogue, &shown, None);
            let v = r.v3.as_mut().unwrap();
            v.render(960, 720);
            let (w, h, px) = v.read_rgba();
            let what = match look {
                Look::Rooms => "rooms",
                Look::Collision => "collision",
                Look::Both => "both",
            };
            let out = dir.join(format!("refview_{scene}_{}_{what}.png", floor.unwrap_or("all")));
            image::RgbaImage::from_raw(w, h, px).unwrap().save(&out).unwrap();
            let models = r.found.iter().filter(|f| f.is_some()).count();
            println!("{} ({models} of {} actors captured)", out.display(), r.found.len());
        }
    }
}
