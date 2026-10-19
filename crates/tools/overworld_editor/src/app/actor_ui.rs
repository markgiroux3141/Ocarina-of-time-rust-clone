//! The Actors tool (docs/EDITOR-ACTORS.md): the game's actors placed in the level. Their markers in
//! the plan and in 3D, the palette (every actor the port has, by group, with the configurations
//! the game places it with, and the room's objects), and the inspector, whose settings come from
//! the actor's schema (ADR 0066): each field as the control its kind calls for, the flags it
//! shares with the level's other actors, the layers it's in, and its raw params and rotation.
//! Each actor shows as the game draws it where it has a capture (`crate::captures`): in 3D, as
//! its outline from above in the plan, and as thumbnails in the palette.

use super::icons::Icon;
use super::style::{self, ACCENT, BAD, FAINT, FIELD, LINE, MUTED, PANEL2, RAISED, TEXT, WARN};
use super::widgets::{self, field, section};
use super::{ActorDrag, App, Sel, Tool, PICK, ROOT};
use crate::actors::{self, Entry, Group};
use crate::captures::Found;
use crate::view3d::{actor_transform, ActorDraw, Camera};
use glam::{Mat4, Vec3};
use oot_game::actor_capture::{CaptureKey, Outcome};
use std::path::Path;
use eframe::egui::{self, Align2, Color32, FontId, Margin, Pos2, Rect, RichText, Sense, Shape, Stroke, StrokeKind, Vec2};
use oot_game::actor_schema::{Field, Flag, Kind, Source, GI_NAMES, ITEM00_NAMES};
use overworld::doc::Actor;
use overworld::geom::{dist, P2};
use std::collections::BTreeMap;

/// How many objects a room may list beside the keeps and Link's (`oot_import::level::room_object_limit`
/// for a scene with a keep, as the overworld's all have).
const ROOM_OBJECTS: usize = 16;

/// A group's colour in the plan, the 3D view, the palette and the outliner.
pub fn group_colour(g: Group) -> Color32 {
    match g {
        Group::Character => Color32::from_rgb(244, 204, 96),
        Group::Enemy => Color32::from_rgb(240, 104, 92),
        Group::Boss => Color32::from_rgb(222, 86, 168),
        Group::Object => Color32::from_rgb(214, 176, 128),
        Group::Item => Color32::from_rgb(110, 214, 240),
        Group::Trigger => Color32::from_rgb(172, 146, 255),
        Group::Door => Color32::from_rgb(232, 150, 84),
        Group::SetPiece => Color32::from_rgb(150, 172, 194),
        Group::Effect => Color32::from_rgb(240, 172, 222),
        Group::Hidden => Color32::from_rgb(160, 160, 160),
    }
}

/// The game layers' names, as `Actor::layers` orders them.
const LAYERS: [&str; 4] = ["Child · day", "Child · night", "Adult · day", "Adult · night"];

/// A flag field's highest number: switch and collectible flags 0x3F, chest flags 0x1F.
fn flag_max(f: &Field) -> i32 {
    let bits = if f.bits >= 16 { 0xFFFF } else { (1 << f.bits) - 1 };
    match f.kind {
        Kind::ChestFlag => bits.min(0x1F),
        _ => bits.min(0x3F),
    }
}

/// The fields the inspector shows: the settings, less the ones split into parts (`part_of`),
/// whose parts are shown instead.
fn shown_fields(e: &Entry) -> Vec<&'static Field> {
    e.settings().filter(|f| !e.schema.fields.iter().any(|g| g.part_of == Some(f.name))).collect()
}

/// Every flag the level's actors hold in their fields, or name as fixed ones: by (kind, number),
/// each user's actor index and how it uses it.
fn flag_users(app: &App) -> BTreeMap<(&'static str, i32), Vec<(usize, String)>> {
    let mut map: BTreeMap<(&'static str, i32), Vec<(usize, String)>> = BTreeMap::new();
    for (j, a) in app.doc.actors.iter().enumerate() {
        let Some(e) = app.catalogue.get(&a.actor) else { continue };
        for f in e.schema.fields {
            if !actors::applies(e.schema, f, a.params, a.rot) {
                continue;
            }
            let v = actors::value(f, a.params, a.rot);
            if let Some(k) = actors::flag_key(f, v) {
                map.entry(k).or_default().push((j, actors::field_label(f)));
            }
        }
        // flags a schema names by number, not by a field
        for u in e.schema.flags {
            let (kind, s) = match u.flag {
                Flag::Switch(s) => ("switch", s),
                Flag::Chest(s) => ("chest", s),
                Flag::Collectible(s) => ("collectible", s),
                _ => continue,
            };
            if let Some(n) = s.strip_prefix("0x").and_then(|h| i32::from_str_radix(h, 16).ok()) {
                map.entry((kind, n)).or_default().push((j, "fixed".into()));
            }
        }
    }
    map
}

/// Where a ray (`o` + t `d`) first meets the box `lo`..`hi`, if it does (t from 0).
fn ray_box(o: Vec3, d: Vec3, lo: Vec3, hi: Vec3) -> Option<f32> {
    let (mut t0, mut t1) = (0.0f32, f32::MAX);
    for k in 0..3 {
        if d[k].abs() < 1e-9 {
            if o[k] < lo[k] || o[k] > hi[k] {
                return None;
            }
            continue;
        }
        let (a, b) = ((lo[k] - o[k]) / d[k], (hi[k] - o[k]) / d[k]);
        t0 = t0.max(a.min(b));
        t1 = t1.min(a.max(b));
    }
    (t1 >= t0).then_some(t0)
}

/// A capture's box, a little bigger, so a flat one (soft soil) can still be picked.
fn capture_box(f: &Found) -> (Vec3, Vec3) {
    let c = f.capture();
    (Vec3::from(c.lo) - 3.0, Vec3::from(c.hi) + 3.0)
}

impl App {
    // ---- the captures -----------------------------------------------------------------------

    /// Each frame: the capture run's progress and its lights taken in, the level's own
    /// configurations asked for (once the mouse is up), and each placed actor's capture (its own
    /// configuration's, the nearest one captured, or its area) with its outline from above.
    pub(super) fn refresh_captures(&mut self, ctx: &egui::Context) {
        let c = ctx.clone();
        let repaint = move || c.request_repaint();
        let busy = self.captures.busy();
        if self.captures.poll(Path::new(ROOT), repaint.clone()) || busy {
            self.status = self.captures.status.clone();
        }
        if let Some(b) = self.text_boxes.as_mut() {
            b.new_frame();
        }
        if let Some(v3) = self.v3.as_mut() {
            v3.new_frame();
            if let Some(m) = &self.captures.manifest {
                v3.set_capture_light(&m.light);
            }
        }
        if !ctx.input(|i| i.pointer.any_down()) {
            let keys: Vec<CaptureKey> = self
                .doc
                .actors
                .iter()
                .filter_map(|a| self.catalogue.get(&a.actor).map(|e| CaptureKey { actor: a.actor.clone(), params: a.params, rot: e.schema.setting_rot(a.rot) }))
                .collect();
            if keys != self.actor_wanted {
                self.actor_wanted = keys.clone();
                self.captures.want(keys, Path::new(ROOT), repaint);
            }
        }
        self.actor_found = (0..self.doc.actors.len())
            .map(|i| {
                let f = self.find_capture(i)?;
                let h = self.captures.hull(&f);
                Some((f, h))
            })
            .collect();
    }

    /// An actor's capture: its configuration's, else the nearest captured; for an invisible one,
    /// its area.
    fn find_capture(&mut self, i: usize) -> Option<Found> {
        let a = &self.doc.actors[i];
        let e = self.catalogue.get(&a.actor)?;
        if let Some(f) = self.captures.find(&a.actor, a.params, e.schema.setting_rot(a.rot)) {
            return Some(f);
        }
        let area = actors::area(&a.actor, a.params, a.rot)?;
        let c = group_colour(e.group);
        Some(self.captures.area(&a.actor, a.params, a.rot, area, [c.r(), c.g(), c.b()]))
    }

    /// A capture for the palette's thumbnails: the configuration's, the nearest, or the area.
    fn thumb(&mut self, name: &str, params: u16, rot: [i16; 3]) -> Option<egui::TextureId> {
        let e = self.catalogue.get(name)?;
        let f = match self.captures.find(name, params, rot) {
            Some(f) => f,
            None => {
                let area = actors::area(name, params, rot)?;
                let c = group_colour(e.group);
                self.captures.area(name, params, rot, area, [c.r(), c.g(), c.b()])
            }
        };
        let g = self.captures.generation;
        self.v3.as_mut()?.actor_thumb(g, &f)
    }

    fn found(&self, i: usize) -> Option<&Found> {
        self.actor_found.get(i).and_then(|f| f.as_ref()).map(|f| &f.0)
    }

    /// Where an actor's capture is drawn: at its place and height, turned as the game turns it.
    fn actor_matrix(&self, i: usize) -> Option<Mat4> {
        let a = self.doc.actors.get(i)?;
        let z = self.actor_z(i).unwrap_or(0.0);
        Some(actor_transform(Vec3::new(a.at[0] as f32, a.at[1] as f32, z as f32), a.rot[1]))
    }

    /// The placed actors' captures for the 3D view.
    pub(super) fn actor_draws(&self) -> Vec<ActorDraw> {
        (0..self.doc.actors.len()).filter_map(|i| Some(ActorDraw { found: self.found(i)?.clone(), transform: self.actor_matrix(i)? })).collect()
    }

    /// An actor's outline from above in the plan (its capture's, turned as it's turned).
    fn actor_outline(&self, i: usize) -> Option<Vec<P2>> {
        let (_, hull) = self.actor_found.get(i)?.as_ref()?;
        let a = &self.doc.actors[i];
        let t = a.rot[1] as f64 * std::f64::consts::PI / 32768.0;
        let (sn, cs) = t.sin_cos();
        // the game's x and z turned by its y rotation, then the editor's x and -y
        Some(hull.iter().map(|&[x, z]| {
            let (x, z) = (x as f64, z as f64);
            [a.at[0] + x * cs + z * sn, a.at[1] - (-x * sn + z * cs)]
        }).collect())
    }

    // ---- placing and moving ---------------------------------------------------------------

    /// Where an actor stands: its own height, else the floor under it as last built.
    pub(super) fn actor_z(&self, i: usize) -> Option<f64> {
        let a = self.doc.actors.get(i)?;
        a.z.or_else(|| self.scene.as_ref().and_then(|s| s.floor_z(a.at)))
    }

    /// The actor whose marker is under p in the plan, the topmost (last) first; else the one
    /// whose outline it's in.
    pub(super) fn actor_at(&self, p: P2) -> Option<usize> {
        let reach = (PICK + 2.0) / self.view.scale;
        (0..self.doc.actors.len())
            .rev()
            .find(|&i| dist(p, self.doc.actors[i].at) <= reach)
            .or_else(|| (0..self.doc.actors.len()).rev().find(|&i| self.actor_outline(i).is_some_and(|o| o.len() >= 3 && overworld::geom::point_in_poly(p, &o))))
    }

    /// The selected actor's turn handle in the plan (none for one that reads its y rotation as a
    /// setting: it has no facing to turn).
    pub(super) fn actor_handle(&self, i: usize) -> Option<P2> {
        let a = self.doc.actors.get(i)?;
        if self.catalogue.get(&a.actor).is_some_and(|e| e.rot_y_is_setting()) {
            return None;
        }
        let f = overworld::props::facing(a.yaw());
        let len = 30.0 / self.view.scale;
        Some([a.at[0] + f[0] * len, a.at[1] + f[1] * len])
    }

    /// Places the palette's actor at p, as it starts (the variant chosen), and selects it.
    pub(super) fn place_actor(&mut self, p: P2) {
        let Some(e) = self.catalogue.get(&self.actor_pick) else {
            self.status = "no actor chosen (the Actors palette)".into();
            return;
        };
        if self.catalogue.transition_only(e.name) {
            self.status = format!("{} goes between rooms: it comes with rooms and exits", e.label);
            return;
        }
        let label = e.label.clone();
        let setting = e.rot_y_is_setting();
        let mut a = Actor::new(&self.actor_pick, [p[0].round(), p[1].round()]);
        (a.params, a.rot) = self.actor_start;
        if !setting {
            // facing as the last one placed
            a.set_yaw(self.place_yaw);
        }
        self.doc.actors.push(a);
        self.sel = Sel::Actor(self.doc.actors.len() - 1);
        self.status = format!("{label} placed: drag to move, its handle turns it (Q / E), PgUp / PgDn raise it");
    }

    /// Turns the selected actor by `d` degrees (not one whose y rotation is a setting).
    pub(super) fn turn_actor(&mut self, d: f64) {
        if let Sel::Actor(i) = self.sel {
            if self.actor_handle(i).is_none() {
                return;
            }
            let a = &mut self.doc.actors[i];
            let y = a.yaw() + d;
            a.set_yaw(y);
            self.place_yaw = a.yaw();
        }
    }

    pub(super) fn duplicate_actor(&mut self) {
        if let Sel::Actor(i) = self.sel {
            let mut a = self.doc.actors[i].clone();
            a.at[0] += 60.0;
            a.at[1] -= 60.0;
            self.doc.actors.push(a);
            self.sel = Sel::Actor(self.doc.actors.len() - 1);
        }
    }

    /// An actor drag in the plan or 3D at w (Alt: no snapping).
    pub(super) fn drag_actor(&mut self, d: ActorDrag, w: P2, alt: bool) {
        match d {
            ActorDrag::Move { i, off } => {
                let p = [w[0] + off[0], w[1] + off[1]];
                self.doc.actors[i].at = if alt { p } else { [p[0].round(), p[1].round()] };
            }
            ActorDrag::Turn { i } => {
                let at = self.doc.actors[i].at;
                let a = (w[1] - at[1]).atan2(w[0] - at[0]).to_degrees() - 90.0;
                let a = if alt { a } else { (a / 15.0).round() * 15.0 };
                self.doc.actors[i].set_yaw(a);
                self.place_yaw = self.doc.actors[i].yaw();
            }
        }
    }

    fn actor_colour(&self, a: &Actor) -> Color32 {
        self.catalogue.get(&a.actor).map_or(BAD, |e| group_colour(e.group))
    }

    fn actor_label(&self, a: &Actor) -> String {
        if !a.name.is_empty() {
            return a.name.clone();
        }
        self.catalogue.get(&a.actor).map_or(a.actor.clone(), |e| e.label.clone())
    }

    // ---- drawing ----------------------------------------------------------------------------

    /// The actors in the plan: a dot in its group's colour with a notch the way it faces, its name
    /// when zoomed in; the selected one ringed, with its turn handle.
    pub(super) fn paint_actors(&self, painter: &egui::Painter, hover: Option<Pos2>) {
        let v = self.view;
        let hovered = hover.and_then(|h| self.actor_at(v.to_world(h)));
        for (i, a) in self.doc.actors.iter().enumerate() {
            let o = v.to_screen(a.at);
            if !v.rect.expand(40.0).contains(o) {
                continue;
            }
            let col = self.actor_colour(a);
            let sel = self.sel == Sel::Actor(i);
            let r = if hovered == Some(i) { 8.0 } else { 7.0 };
            // its outline from above, as the game draws it
            if let Some(out) = self.actor_outline(i).filter(|o| o.len() >= 3) {
                let pts: Vec<Pos2> = out.iter().map(|&q| v.to_screen(q)).collect();
                let fill = col.gamma_multiply(if sel { 0.35 } else { 0.22 });
                let line = if sel { Color32::from_rgb(255, 150, 50) } else { col.gamma_multiply(0.75) };
                painter.add(Shape::convex_polygon(pts, fill, Stroke::new(if sel { 1.5 } else { 1.0 }, line)));
            }
            if sel {
                painter.circle_stroke(o, 12.0, Stroke::new(5.0, Color32::from_black_alpha(110)));
                painter.circle_stroke(o, 12.0, Stroke::new(2.0, Color32::from_rgb(255, 150, 50)));
            }
            let setting = self.catalogue.get(&a.actor).is_some_and(|e| e.rot_y_is_setting());
            if !setting {
                let f = overworld::props::facing(a.yaw());
                let d = Vec2::new(f[0] as f32, -f[1] as f32);
                let n = Vec2::new(-d.y, d.x);
                painter.add(Shape::convex_polygon(vec![o + d * (r + 6.0), o + d * (r - 1.0) + n * 4.5, o + d * (r - 1.0) - n * 4.5], col, Stroke::new(1.0, Color32::BLACK)));
            }
            painter.circle(o, r, col, Stroke::new(1.5, Color32::from_black_alpha(220)));
            if a.layers.is_some() {
                // in some layers only: a hollow middle
                painter.circle_filled(o, 2.5, Color32::from_black_alpha(200));
            }
            if v.scale > 0.12 || sel || hovered == Some(i) {
                let t = self.actor_label(a);
                let p = o + Vec2::new(r + 5.0, 0.0);
                painter.text(p + Vec2::new(1.0, 1.0), Align2::LEFT_CENTER, &t, FontId::proportional(11.5), Color32::from_black_alpha(200));
                painter.text(p, Align2::LEFT_CENTER, &t, FontId::proportional(11.5), col.lerp_to_gamma(Color32::WHITE, 0.5));
            }
            if sel {
                if let Some(h) = self.actor_handle(i) {
                    let h = v.to_screen(h);
                    painter.line_segment([o, h], Stroke::new(1.5, col));
                    painter.circle(h, 5.5, col, Stroke::new(1.0, Color32::BLACK));
                }
            }
        }
        // the Actors tool's ghost: the chosen actor where a click would put it
        if let (Tool::Actor, Some(h), super::Gesture::None) = (self.tool, hover, &self.gesture) {
            if hovered.is_none() {
                if let Some(e) = self.catalogue.get(&self.actor_pick) {
                    let col = group_colour(e.group);
                    painter.circle_stroke(h, 7.0, Stroke::new(2.0, col.gamma_multiply(0.8)));
                    painter.text(h + Vec2::new(12.0, 0.0), Align2::LEFT_CENTER, &e.label, FontId::proportional(11.5), col.gamma_multiply(0.8));
                }
            }
        }
    }

    /// Where each actor's marker is in the 3D view: its foot and the top of its post.
    fn actor_marks_3d(&self, cam: &Camera, rect: Rect) -> Vec<(usize, Pos2, Pos2, f64)> {
        let mut out = vec![];
        for (i, a) in self.doc.actors.iter().enumerate() {
            let z = self.actor_z(i).unwrap_or(0.0);
            let foot = glam::Vec3::new(a.at[0] as f32, a.at[1] as f32, z as f32);
            let top = foot + glam::Vec3::new(0.0, 0.0, 70.0);
            if let (Some(f), Some(t)) = (cam.project(rect, foot), cam.project(rect, top)) {
                out.push((i, f, t, (top - cam.eye()).length() as f64));
            }
        }
        out
    }

    /// The actor under s in the 3D view, the nearest first: by its capture's box, or by its
    /// marker (its post's top) for one without a capture.
    pub(super) fn actor_at_3d(&self, cam: &Camera, rect: Rect, s: Pos2) -> Option<usize> {
        let marker = self
            .actor_marks_3d(cam, rect)
            .into_iter()
            .filter(|m| self.found(m.0).is_none() && (m.2.distance(s) <= 11.0 || m.1.distance(s) <= 8.0))
            .min_by(|a, b| a.3.total_cmp(&b.3))
            .map(|m| (m.0, m.3 as f32));
        let ndc = [(s.x - rect.left()) / rect.width() * 2.0 - 1.0, 1.0 - (s.y - rect.top()) / rect.height() * 2.0];
        let (o, d) = cam.ray(rect.width() / rect.height(), ndc);
        // models first; an invisible actor's area only where no model is, and not from inside it
        let hit = |areas: bool| {
            (0..self.doc.actors.len())
                .filter_map(|i| {
                    let f = self.found(i)?;
                    if f.file.actor.starts_with("area:") != areas {
                        return None;
                    }
                    let (lo, hi) = capture_box(f);
                    let inv = self.actor_matrix(i)?.inverse();
                    ray_box(inv.transform_point3(o), inv.transform_vector3(d), lo, hi).filter(|&t| !areas || t > 0.0).map(|t| (i, t))
                })
                .min_by(|a, b| a.1.total_cmp(&b.1))
        };
        let boxed = hit(false).or_else(|| hit(true));
        match (marker, boxed) {
            (Some(m), Some(b)) => Some(if m.1 < b.1 { m.0 } else { b.0 }),
            (m, b) => m.or(b).map(|x| x.0),
        }
    }

    /// A captured actor's box in the 3D view: its 12 edges, as screen segments.
    fn capture_box_lines(&self, cam: &Camera, rect: Rect, i: usize) -> Vec<(Pos2, Pos2)> {
        let (Some(f), Some(m)) = (self.found(i), self.actor_matrix(i)) else { return vec![] };
        let (lo, hi) = capture_box(f);
        let c = |k: usize| m.transform_point3(Vec3::new(if k & 1 == 0 { lo.x } else { hi.x }, if k & 2 == 0 { lo.y } else { hi.y }, if k & 4 == 0 { lo.z } else { hi.z }));
        let mut out = vec![];
        for a in 0..8usize {
            for bit in [1usize, 2, 4] {
                let b = a | bit;
                if b != a {
                    if let (Some(p), Some(q)) = (cam.project(rect, c(a)), cam.project(rect, c(b))) {
                        out.push((p, q));
                    }
                }
            }
        }
        out
    }

    /// The actors in 3D: a post from the ground with a disc on top in the group's colour, faint
    /// where the level hides it; the selected one's facing on the ground.
    pub(super) fn paint_actors_3d(&self, painter: &egui::Painter, cam: &Camera, rect: Rect, hover: Option<Pos2>) {
        let eye = cam.eye();
        let eye = [eye.x as f64, eye.y as f64, eye.z as f64];
        let hovered = hover.and_then(|h| self.actor_at_3d(cam, rect, h));
        let mut marks = self.actor_marks_3d(cam, rect);
        // far ones first, so near ones draw over them
        marks.sort_by(|a, b| b.3.total_cmp(&a.3));
        for (i, foot, top, d) in marks {
            let a = &self.doc.actors[i];
            let z = self.actor_z(i).unwrap_or(0.0) + 70.0;
            // hidden: the level is in the way of the post's top
            let dir = [a.at[0] - eye[0], a.at[1] - eye[1], z - eye[2]];
            let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt().max(1e-6);
            let dir = dir.map(|x| x / len);
            let hidden = self.scene.as_ref().is_some_and(|s| {
                s.raycast(eye, dir).is_some_and(|h| h.0 < len - 8.0) || s.raycast_wall(eye, dir).is_some_and(|h| h.0 < len - 8.0)
            });
            let sel = self.sel == Sel::Actor(i);
            let col = self.actor_colour(a);
            let col = if hidden && !sel { col.gamma_multiply(0.35) } else { col };
            if self.found(i).is_some() {
                // drawn as the game draws it: its box when it's selected or under the pointer
                if sel || hovered == Some(i) {
                    let stroke = if sel { Stroke::new(1.5, Color32::from_rgb(255, 150, 50)) } else { Stroke::new(1.0, Color32::from_white_alpha(170)) };
                    for (p, q) in self.capture_box_lines(cam, rect, i) {
                        painter.line_segment([p, q], stroke);
                    }
                    let t = self.actor_label(a);
                    painter.text(top + Vec2::new(9.0, 1.0), Align2::LEFT_CENTER, &t, FontId::proportional(12.5), Color32::from_black_alpha(220));
                    painter.text(top + Vec2::new(8.0, 0.0), Align2::LEFT_CENTER, &t, FontId::proportional(12.5), Color32::WHITE);
                }
                if sel && self.actor_handle(i).is_some() {
                    let f = overworld::props::facing(a.yaw());
                    let zf = z - 69.0;
                    if let (Some(p0), Some(p1)) = (
                        cam.project(rect, glam::Vec3::new(a.at[0] as f32, a.at[1] as f32, zf as f32)),
                        cam.project(rect, glam::Vec3::new((a.at[0] + f[0] * 60.0) as f32, (a.at[1] + f[1] * 60.0) as f32, zf as f32)),
                    ) {
                        painter.arrow(p0, p1 - p0, Stroke::new(2.0, col));
                    }
                }
                continue;
            }
            let r = (900.0 / d as f32).clamp(4.0, 9.0) + if hovered == Some(i) { 1.5 } else { 0.0 };
            painter.line_segment([foot, top], Stroke::new(2.0, col));
            if sel {
                painter.circle_stroke(top, r + 4.0, Stroke::new(4.0, Color32::from_black_alpha(120)));
                painter.circle_stroke(top, r + 4.0, Stroke::new(2.0, Color32::from_rgb(255, 150, 50)));
                if self.actor_handle(i).is_some() {
                    let f = overworld::props::facing(a.yaw());
                    let zf = z - 69.0;
                    if let (Some(p0), Some(p1)) = (
                        cam.project(rect, glam::Vec3::new(a.at[0] as f32, a.at[1] as f32, zf as f32)),
                        cam.project(rect, glam::Vec3::new((a.at[0] + f[0] * 60.0) as f32, (a.at[1] + f[1] * 60.0) as f32, zf as f32)),
                    ) {
                        painter.arrow(p0, p1 - p0, Stroke::new(2.0, col));
                    }
                }
            }
            painter.circle(top, r, col, Stroke::new(1.5, Color32::from_black_alpha(if hidden { 90 } else { 220 })));
            if sel || hovered == Some(i) {
                let t = self.actor_label(a);
                painter.text(top + Vec2::new(r + 6.0, 1.0), Align2::LEFT_CENTER, &t, FontId::proportional(12.5), Color32::from_black_alpha(220));
                painter.text(top + Vec2::new(r + 5.0, 0.0), Align2::LEFT_CENTER, &t, FontId::proportional(12.5), Color32::WHITE);
            }
        }
    }

    // ---- the palette ------------------------------------------------------------------------

    pub(super) fn actor_palette(&mut self, ui: &mut egui::Ui) {
        if self.captures.busy() {
            let s = self.captures.status.clone();
            pad(ui, |ui| widgets::hint(ui, s));
        }
        if self.catalogue.reading() {
            pad(ui, |ui| widgets::hint(ui, "Reading how the game places its actors…"));
        } else if let Some(e) = &self.catalogue.game_error {
            let e = e.clone();
            pad(ui, |ui| widgets::callout(ui, &format!("Without the game's data: no variants, and doors aren't told apart. {e}"), WARN));
        }
        let q = self.actor_query.to_lowercase();
        // what's listed: by group, search, and whether only other actors spawn it
        let listed: Vec<(&'static str, String, Group, usize, bool)> = self
            .catalogue
            .entries
            .iter()
            .filter(|e| e.group != Group::Hidden)
            .filter(|e| self.actor_spawned || !self.catalogue.spawned_only(e.name))
            .filter(|e| q.is_empty() || e.label.to_lowercase().contains(&q) || e.name.to_lowercase().contains(&q))
            .map(|e| (e.name, e.label.clone(), e.group, self.catalogue.placed(e.name).map_or(0, |p| p.count()), self.catalogue.transition_only(e.name)))
            .collect();
        let counts: Vec<(Group, usize)> = Group::ALL.iter().map(|&g| (g, listed.iter().filter(|x| x.2 == g).count())).collect();
        let mut rows: Vec<_> = listed.into_iter().filter(|x| self.actor_group.is_none_or(|g| g == x.2)).collect();
        rows.sort_by(|a, b| (Group::ALL.iter().position(|g| *g == a.2), &a.1).cmp(&(Group::ALL.iter().position(|g| *g == b.2), &b.1)));
        let n = rows.len();
        // each row's thumbnail: the configuration it starts as
        let thumbs: Vec<Option<egui::TextureId>> = rows
            .iter()
            .map(|r| {
                let (params, rot) = self.catalogue.placed(r.0).and_then(|p| p.variants.first()).map_or((0, [0; 3]), |v| (v.params, v.rot));
                self.thumb(r.0, params, rot)
            })
            .collect();
        let mut pick: Option<&'static str> = None;
        section(ui, "pal actors", "Actors", Some(n.to_string()), true, |ui| {
            widgets::search(ui, &mut self.actor_query, "Search: pot, keese, En_Kusa…");
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(5.0, 5.0);
                if widgets::chip(ui, "All", None, self.actor_group.is_none()).clicked() {
                    self.actor_group = None;
                }
                for &(g, c) in &counts {
                    if c == 0 {
                        continue;
                    }
                    let on = self.actor_group == Some(g);
                    if widgets::chip(ui, g.title(), Some(&c.to_string()), on).on_hover_text(g.about()).clicked() {
                        self.actor_group = if on { None } else { Some(g) };
                    }
                }
            });
            widgets::toggle(ui, &mut self.actor_spawned, "Also what only other actors spawn")
                .on_hover_text("Arrows, songs' effects, an enemy's pieces: the game never places them in a room. Placed, they do what they do when spawned.");
            egui::ScrollArea::vertical().id_salt("actor list").max_height(340.0).auto_shrink([false, true]).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for ((name, label, group, count, door), thumb) in rows.iter().zip(&thumbs) {
                    let on = self.actor_pick == *name;
                    let resp = list_row(ui, label, name, group_colour(*group), *count, on, *door, *thumb);
                    let resp = if *door { resp.on_hover_text("The game places it only between rooms: it comes with rooms and exits (stage E4)") } else { resp.on_hover_text(format!("{name}: placed {count} times in the game")) };
                    if resp.clicked() && !door {
                        pick = Some(name);
                    }
                }
                if rows.is_empty() {
                    widgets::hint(ui, "Nothing matches.");
                }
            });
        });
        if let Some(name) = pick {
            self.actor_pick = name.to_string();
            self.actor_start = self.catalogue.placed(name).and_then(|p| p.variants.first()).map_or((0, [0; 3]), |v| (v.params, v.rot));
            if self.tool != Tool::Actor {
                self.tool = Tool::Actor;
            }
        }
        self.start_section(ui);
        self.room_objects_section(ui);
    }

    /// What the chosen actor starts as: the configurations the game places it with.
    fn start_section(&mut self, ui: &mut egui::Ui) {
        let Some(e) = self.catalogue.get(&self.actor_pick) else { return };
        let title = format!("New {}", e.label.to_lowercase());
        let variants: Vec<(u16, [i16; 3], usize, String, String)> = self
            .catalogue
            .placed(e.name)
            .map(|p| p.variants.iter().take(24).map(|v| (v.params, v.rot, v.count, v.first.clone(), actors::describe(e, v.params, v.rot))).collect())
            .unwrap_or_default();
        let blank = actors::describe(e, 0, [0; 3]);
        let name = e.name;
        let thumbs: Vec<Option<egui::TextureId>> = variants.iter().map(|v| self.thumb(name, v.0, v.1)).collect();
        let blank_thumb = self.thumb(name, 0, [0; 3]);
        let mut choose = None;
        section(ui, "pal astart", &title, None, true, |ui| {
            widgets::hint(ui, "Starts as one of the ways the game places it (most placed first). Change any of it after, in the inspector.");
            egui::ScrollArea::vertical().id_salt("actor variants").max_height(220.0).auto_shrink([false, true]).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                for ((params, rot, count, first, what), thumb) in variants.iter().zip(&thumbs) {
                    let on = self.actor_start == (*params, *rot);
                    let sub = format!("×{count} · {first}");
                    if variant_row(ui, what, &sub, on, *thumb).on_hover_text(format!("params {params:#06x}, rotation {rot:?}")).clicked() {
                        choose = Some((*params, *rot));
                    }
                }
                let on = self.actor_start == (0, [0; 3]) && !variants.iter().any(|v| (v.0, v.1) == (0, [0; 3]));
                if variant_row(ui, &blank, "params 0", on, blank_thumb).clicked() {
                    choose = Some((0, [0; 3]));
                }
            });
        });
        if let Some(c) = choose {
            self.actor_start = c;
        }
    }

    /// The objects the level's actors need (`room_objects`), and the extra ones it loads.
    pub(super) fn room_objects(&self) -> Vec<(String, Vec<usize>)> {
        let mut out: Vec<(String, Vec<usize>)> = vec![];
        for (i, a) in self.doc.actors.iter().enumerate() {
            let Some(e) = self.catalogue.get(&a.actor) else { continue };
            for o in e.objects_for(a.params, a.rot) {
                match out.iter_mut().find(|x| x.0 == o) {
                    Some(x) => x.1.push(i),
                    None => out.push((o.to_string(), vec![i])),
                }
            }
        }
        for o in &self.doc.game.objects {
            if !out.iter().any(|x| &x.0 == o) {
                out.push((o.clone(), vec![]));
            }
        }
        out
    }

    fn room_objects_section(&mut self, ui: &mut egui::Ui) {
        let objs = self.room_objects();
        let over = objs.len() > ROOM_OBJECTS;
        let mut remove = None;
        let mut add: Option<String> = None;
        let all = self.catalogue.objects.clone();
        section(ui, "pal aobjects", "Room objects", Some(format!("{} / {ROOM_OBJECTS}", objs.len())), true, |ui| {
            // a bar: how full the room's object banks are
            let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 6.0), Sense::hover());
            ui.painter().rect_filled(r, 3.0, FIELD);
            let f = (objs.len() as f32 / ROOM_OBJECTS as f32).min(1.0);
            ui.painter().rect_filled(Rect::from_min_size(r.min, Vec2::new(r.width() * f, r.height())), 3.0, if over { BAD } else { ACCENT });
            if over {
                widgets::callout(ui, &format!("The room can load {ROOM_OBJECTS} objects besides the keeps and Link's: the actors of the last {} won't appear.", objs.len() - ROOM_OBJECTS), BAD);
            }
            for (k, (o, users)) in objs.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(o).monospace().size(11.5).color(if k >= ROOM_OBJECTS { BAD } else { TEXT }));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if users.is_empty() {
                            if widgets::icon_button(ui, Icon::Trash, 20.0, false).on_hover_text("Take it out").clicked() {
                                remove = Some(o.clone());
                            }
                            ui.label(RichText::new("added").size(11.0).color(FAINT));
                        } else {
                            ui.label(RichText::new(format!("{} actor{}", users.len(), if users.len() == 1 { "" } else { "s" })).size(11.0).color(FAINT));
                        }
                    });
                });
            }
            if objs.is_empty() {
                widgets::hint(ui, "None yet: every actor placed so far draws from the keeps.");
            }
            if !all.is_empty() {
                egui::ComboBox::from_id_salt("add object").selected_text(RichText::new("Add an object…").color(MUTED)).width(ui.available_width()).height(320.0).show_ui(ui, |ui| {
                    for o in all.iter().filter(|o| o.starts_with("object_") && !objs.iter().any(|x| &x.0 == *o)) {
                        if ui.selectable_label(false, o).clicked() {
                            add = Some(o.clone());
                        }
                    }
                });
            }
            widgets::hint(ui, "What each actor needs for how it's set up is loaded for it (its schema says which). Add one for anything else, as the game's rooms list them. The keeps, gameplay_keep and the field keep, are always there.");
        });
        if let Some(o) = remove {
            self.doc.game.objects.retain(|x| *x != o);
        }
        if let Some(o) = add {
            self.doc.game.objects.push(o);
        }
    }

    // ---- the inspector ----------------------------------------------------------------------

    pub(super) fn actor_ui(&mut self, ui: &mut egui::Ui, i: usize) {
        if i >= self.doc.actors.len() {
            return;
        }
        let a = self.doc.actors[i].clone();
        let entry = self.catalogue.get(&a.actor);
        let group = entry.map_or(Group::Object, |e| e.group);
        let col = group_colour(group);
        let label = self.actor_label(&a);
        let eyebrow = format!("Actor · {}", group.one());
        let delete = super::inspector::icon_header(ui, Icon::Actor, &eyebrow, col, label, true);
        let Some(e) = entry.cloned() else {
            pad(ui, |ui| widgets::callout(ui, &format!("The port has no actor {:?}: the game would spawn whatever has that name, if anything.", a.actor), BAD));
            self.raw_section(ui, i, None);
            if delete {
                self.delete_selection();
            }
            return;
        };
        // what's worth knowing first
        let door = self.catalogue.transition_only(e.name);
        let spawned = self.catalogue.spawned_only(e.name);
        let objs = self.room_objects();
        let cut = objs.iter().enumerate().any(|(k, o)| k >= ROOM_OBJECTS && o.1.contains(&i));
        let loose = actors::loose_bits(e.schema, a.params);
        let drawn = self.drawn_note(i, &e);
        let thumb = self.found(i).cloned().and_then(|f| {
            let g = self.captures.generation;
            self.v3.as_mut()?.actor_thumb(g, &f)
        });
        pad(ui, |ui| {
            let w = ui.available_width();
            // its thumbnail, and how the views draw it
            let tw = if thumb.is_some() { 72.0 } else { 0.0 };
            let g = ui.painter().layout(drawn.0.clone(), FontId::proportional(11.5), if drawn.1 { MUTED } else { WARN }, (w - tw).max(60.0));
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w, g.size().y.max(if thumb.is_some() { 64.0 } else { 0.0 })), Sense::hover());
            if let Some(t) = thumb {
                ui.painter().image(t, Rect::from_min_size(rect.min, Vec2::splat(64.0)), Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
            }
            ui.painter().galley(rect.min + Vec2::new(tw, 0.0), g, TEXT);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(5.0, 5.0);
                widgets::chip(ui, e.name, None, false).on_hover_text("The overlay: the game's own name for it");
                widgets::chip(ui, "id", Some(&format!("{:#06x}", e.id)), false);
                for o in e.objects_for(a.params, a.rot) {
                    widgets::chip(ui, o, None, false).on_hover_text("An object it needs, set up as it is: the room loads it (Room objects, in the palette)");
                }
            });
            if door {
                widgets::callout(ui, "The game only places it between rooms, as a transition actor; as a room actor it isn't what it should be. Doors come with rooms and exits (stage E4).", WARN);
            }
            if spawned {
                widgets::callout(ui, "The game never places it: other actors spawn it. Placed, it does what it does when spawned.", WARN);
            }
            if cut {
                widgets::callout(ui, &format!("An object it needs is past the room's {ROOM_OBJECTS}: it won't appear (Room objects, in the palette)."), BAD);
            }
            if !a.in_layer(0) && !a.in_layer(1) && !a.in_layer(2) && !a.in_layer(3) {
                widgets::callout(ui, "It's in none of the layers, so it never appears.", WARN);
            }
            if loose != 0 {
                widgets::callout(ui, &format!("Params bits {loose:#06x} are set that nothing reads (see Raw)."), WARN);
            }
        });
        self.placement_section(ui, i, &e);
        self.settings_section(ui, i, &e);
        self.flags_section(ui, i, &e);
        self.dialogue_section(ui, i, &e);
        self.layers_section(ui, i);
        self.game_section(ui, i, &e);
        self.raw_section(ui, i, Some(&e));
        self.about_section(ui, &e);
        let mut dup = false;
        pad(ui, |ui| {
            dup = widgets::button(ui, Some(Icon::Duplicate), "Duplicate  (Ctrl+D)", Some(ui.available_width()), false).clicked();
        });
        if dup {
            self.duplicate_actor();
        }
        if delete {
            self.sel = Sel::Actor(i);
            self.delete_selection();
        }
    }

    /// How the views draw an actor, in words, and whether that's as it is set up: its own
    /// capture (and the setting it was caught in), another configuration's, its area, or a marker
    /// (and why).
    fn drawn_note(&self, i: usize, e: &Entry) -> (String, bool) {
        let a = &self.doc.actors[i];
        let rot = e.schema.setting_rot(a.rot);
        let tried = self.captures.outcome(e.name, a.params, rot);
        match (self.found(i), tried) {
            (Some(f), Some((Outcome::Captured, note))) if f.exact => (format!("Drawn as the game draws it ({note})."), true),
            (Some(f), _) if f.file.actor.starts_with("area:") => ("Draws nothing: shown as the area it watches.".into(), true),
            (Some(f), t) => {
                let c = f.capture();
                let why = match t {
                    Some((_, note)) => format!(" This one drew nothing: {note}."),
                    None if self.captures.busy() => " This one is being captured.".into(),
                    None => String::new(),
                };
                (format!("Shown as params {:#06x}, the nearest captured.{why}", c.params), false)
            }
            (None, Some((_, note))) => (format!("A marker: it drew nothing ({note})."), false),
            (None, None) if self.captures.busy() => ("A marker until its capture is made…".into(), false),
            (None, None) => ("A marker: no capture of it yet (the game's capture-actors makes them).".into(), false),
        }
    }

    fn placement_section(&mut self, ui: &mut egui::Ui, i: usize, e: &Entry) {
        let built_z = self.scene.as_ref().and_then(|s| s.floor_z(self.doc.actors[i].at));
        let setting = e.rot_y_is_setting();
        let tilts: Vec<&'static Field> = e.schema.fields.iter().filter(|f| f.kind == Kind::Rotation).collect();
        let mut yaw_changed = None;
        let a = &mut self.doc.actors[i];
        section(ui, "ins aplace", "Placement", None, true, |ui| {
            field(ui, "Position", None, |ui| {
                ui.add(egui::DragValue::new(&mut a.at[0]).speed(2.0).prefix("x "));
                ui.add(egui::DragValue::new(&mut a.at[1]).speed(2.0).prefix("y "));
            });
            field(ui, "Height", Some("On the ground: it stands on the floor there as built (what the game would do with it next is the actor's own business). PgUp / PgDn raise and sink it."), |ui| {
                let mut ground = a.z.is_none();
                if widgets::toggle(ui, &mut ground, "on the ground").changed() {
                    a.z = if ground { None } else { Some(built_z.unwrap_or(0.0).round()) };
                }
                match &mut a.z {
                    Some(z) => {
                        ui.add(egui::DragValue::new(z).speed(1.0));
                    }
                    None => {
                        let mut z = built_z.unwrap_or(0.0).round();
                        ui.add_enabled(false, egui::DragValue::new(&mut z));
                    }
                }
            });
            if setting {
                widgets::hint(ui, "It reads its y rotation as a setting (under Settings), so it has no facing to turn.");
            } else {
                field(ui, "Facing", Some("Degrees counter-clockwise from north. Drag the dial (15° steps; Shift: 1°), or Q / E."), |ui| {
                    let mut yaw = a.yaw();
                    let mut changed = super::inspector::dial(ui, &mut yaw);
                    changed |= ui.add(egui::DragValue::new(&mut yaw).speed(1.0).range(-180.0..=180.0).suffix("°")).changed();
                    if changed {
                        a.set_yaw(yaw);
                        yaw_changed = Some(a.yaw());
                    }
                });
            }
            // rotation components it uses as rotations (a tilted gravestone): in degrees
            for f in tilts {
                let k = match f.source {
                    Source::RotX => 0,
                    Source::RotZ => 2,
                    _ => continue,
                };
                let mut deg = a.rot[k] as f64 / 65536.0 * 360.0;
                field(ui, if k == 0 { "Tilt x" } else { "Tilt z" }, Some(f.meaning), |ui| {
                    if ui.add(egui::DragValue::new(&mut deg).speed(0.5).range(-180.0..=180.0).suffix("°")).changed() {
                        a.rot[k] = ((deg / 360.0 * 65536.0).round() as i64).rem_euclid(65536) as u16 as i16;
                    }
                });
            }
            field(ui, "Name", Some("A name of its own, for the outliner (the game never sees it)"), |ui| {
                ui.add(egui::TextEdit::singleline(&mut a.name).hint_text(RichText::new(e.label.as_str()).color(FAINT)).desired_width(ui.available_width()));
            });
        });
        a.at = a.at.map(|x| (x * 100.0).round() / 100.0);
        if let Some(y) = yaw_changed {
            self.place_yaw = y;
        }
    }

    fn settings_section(&mut self, ui: &mut egui::Ui, i: usize, e: &Entry) {
        let fields = shown_fields(e);
        let users = flag_users(self);
        let labels: Vec<String> = (0..self.doc.actors.len()).map(|j| self.actor_label(&self.doc.actors[j])).collect();
        let objects = self.catalogue.objects.clone();
        let paths = self.doc.paths.len();
        let mut select = None;
        let a = &mut self.doc.actors[i];
        let count = fields.iter().filter(|f| actors::applies(e.schema, f, a.params, a.rot)).count();
        section(ui, "ins asettings", "Settings", Some(count.to_string()), true, |ui| {
            if fields.is_empty() {
                widgets::hint(ui, "It takes nothing from where it's placed.");
            }
            for f in &fields {
                if !actors::applies(e.schema, f, a.params, a.rot) {
                    continue;
                }
                let label = actors::field_label(f);
                let src = match f.source {
                    Source::Params => "params",
                    Source::RotX => "rotation x",
                    Source::RotY => "rotation y",
                    Source::RotZ => "rotation z",
                };
                let bits = if f.bits == 1 { format!("bit {}", f.shift) } else { format!("bits {}–{}", f.shift, f.shift + f.bits - 1) };
                let tip = format!("{}\n\n{} ({src} {bits})", actors::upper_first(f.meaning), f.name);
                let mut v = actors::value(f, a.params, a.rot);
                let before = v;
                let long = label.chars().count() > 13;
                let mut widget = |ui: &mut egui::Ui| field_widget(ui, (i, f.name), f, &mut v, &users, &labels, i, &objects, paths, &mut select);
                if long {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 3.0;
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            ui.label(RichText::new(&label).color(MUTED).size(12.5));
                            widgets::info(ui, &tip);
                        });
                        widget(ui);
                    });
                } else {
                    field(ui, &label, Some(&tip), widget);
                }
                if v != before {
                    actors::set_value(f, &mut a.params, &mut a.rot, v);
                }
            }
        });
        if let Some(j) = select {
            self.sel = Sel::Actor(j);
        }
    }

    fn flags_section(&mut self, ui: &mut egui::Ui, i: usize, e: &Entry) {
        if e.schema.flags.is_empty() {
            return;
        }
        let a = &self.doc.actors[i];
        let users = flag_users(self);
        let mut select = None;
        section(ui, "ins aflags", "Flags", Some(e.schema.flags.len().to_string()), false, |ui| {
            for u in e.schema.flags {
                let how: Vec<&str> = [(u.reads, "reads"), (u.sets, "sets"), (u.clears, "clears")].iter().filter(|x| x.0).map(|x| x.1).collect();
                // the flag, and its number where a field (or the schema) gives one
                let (what, key) = match u.flag {
                    Flag::Switch(s) | Flag::Chest(s) | Flag::Collectible(s) => {
                        let kind = match u.flag {
                            Flag::Switch(_) => "switch",
                            Flag::Chest(_) => "chest",
                            _ => "collectible",
                        };
                        let n = e.schema.field(s).map(|f| actors::value(f, a.params, a.rot)).or_else(|| s.strip_prefix("0x").and_then(|h| i32::from_str_radix(h, 16).ok()));
                        (format!("{} flag {}", actors::upper_first(kind), n.map_or(s.to_string(), |n| format!("{n:#04x}"))), n.map(|n| (kind, n)))
                    }
                    Flag::Clear => ("The room's clear flag".into(), None),
                    Flag::TempClear => ("The room's temporary clear flag".into(), None),
                    Flag::EventChkInf(n, _) | Flag::InfTable(n, _) | Flag::ItemGetInf(n, _) | Flag::EventInf(n, _) => (n.to_string(), None),
                    Flag::Other(n) => (n.to_string(), None),
                };
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(&what).size(12.5).color(TEXT));
                        ui.label(RichText::new(how.join(", ")).size(11.5).color(ACCENT));
                    });
                    let mut note = u.note.to_string();
                    if !u.indirect.is_empty() {
                        note = if note.is_empty() { u.indirect.to_string() } else { format!("{note}; {}", u.indirect) };
                    }
                    if !note.is_empty() {
                        widgets::hint(ui, actors::upper_first(&note));
                    }
                    if let Some(k) = key {
                        let others: Vec<&(usize, String)> = users.get(&k).map(|v| v.iter().filter(|x| x.0 != i).collect()).unwrap_or_default();
                        if !others.is_empty() {
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
                                ui.label(RichText::new("Shared with").size(11.5).color(FAINT));
                                for (j, _) in others {
                                    let t = self.actor_label(&self.doc.actors[*j]);
                                    if widgets::chip(ui, &t, None, false).on_hover_text("Select it").clicked() {
                                        select = Some(*j);
                                    }
                                }
                            });
                        }
                    }
                });
            }
            widgets::hint(ui, "Save flags (EVENTCHKINF, INFTABLE…) are the save's: a debug save preset sets them for play.");
        });
        if let Some(j) = select {
            self.sel = Sel::Actor(j);
        }
    }

    fn layers_section(&mut self, ui: &mut egui::Ui, i: usize) {
        let a = &mut self.doc.actors[i];
        let mut ls = a.layers.unwrap_or([true; 4]);
        let n = ls.iter().filter(|&&x| x).count();
        section(ui, "ins alayers", "Appears", Some(if n == 4 { "always".into() } else { format!("{n} of 4") }), true, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(5.0, 5.0);
                for (k, name) in LAYERS.iter().enumerate() {
                    if widgets::chip(ui, name, None, ls[k]).clicked() {
                        ls[k] = !ls[k];
                    }
                }
            });
            widgets::hint(ui, "The scene's layers, as the game picks them by Link's age and the time of day: a night-only Keese, a stall only the child sees.");
        });
        a.layers = (ls != [true; 4]).then_some(ls);
    }

    fn game_section(&mut self, ui: &mut egui::Ui, i: usize, e: &Entry) {
        let Some(p) = self.catalogue.placed(e.name) else { return };
        let variants: Vec<(u16, [i16; 3], usize, String, String)> = p.variants.iter().take(16).map(|v| (v.params, v.rot, v.count, v.first.clone(), actors::describe(e, v.params, v.rot))).collect();
        let (rooms, transitions, total) = (p.rooms, p.transitions, p.count());
        let a = &self.doc.actors[i];
        let cur = (a.params, actors::setting_rot(e.schema, a.rot));
        let thumbs: Vec<Option<egui::TextureId>> = variants.iter().map(|v| self.thumb(e.name, v.0, v.1)).collect();
        let mut apply = None;
        section(ui, "ins agame", "In the game", Some(format!("×{total}")), false, |ui| {
            let mut parts = vec![];
            if rooms > 0 {
                parts.push(format!("{rooms} in rooms"));
            }
            if transitions > 0 {
                parts.push(format!("{transitions} between rooms"));
            }
            widgets::hint(ui, format!("Placed {total} times: {}. Its ways, most placed first (click one to take its settings):", parts.join(", ")));
            ui.spacing_mut().item_spacing.y = 2.0;
            for ((params, rot, count, first, what), thumb) in variants.iter().zip(&thumbs) {
                let on = cur == (*params, *rot);
                if variant_row(ui, what, &format!("×{count} · {first}"), on, *thumb).on_hover_text(format!("params {params:#06x}, rotation {rot:?}")).clicked() {
                    apply = Some((*params, *rot));
                }
            }
        });
        if let Some((params, rot)) = apply {
            let a = &mut self.doc.actors[i];
            a.params = params;
            // the setting bits of each rotation component from the variant, the rest kept
            let mask = |k: usize| e.schema.fields.iter().filter(|f| f.kind != Kind::Rotation && [Source::RotX, Source::RotY, Source::RotZ][k] == f.source).fold(0u16, |m, f| m | f.mask());
            for k in 0..3 {
                let m = mask(k);
                a.rot[k] = ((a.rot[k] as u16 & !m) | (rot[k] as u16 & m)) as i16;
            }
        }
    }

    fn raw_section(&mut self, ui: &mut egui::Ui, i: usize, e: Option<&Entry>) {
        let loose = e.map_or(0, |e| actors::loose_bits(e.schema, self.doc.actors[i].params));
        let a = &mut self.doc.actors[i];
        section(ui, "ins araw", "Raw", Some(format!("{:#06x}", a.params)), false, |ui| {
            field(ui, "Params", Some("The 16 bits the game reads its settings from"), |ui| {
                ui.add(egui::DragValue::new(&mut a.params).hexadecimal(4, false, true).speed(1.0));
            });
            field(ui, "Rotation", Some("Binary angles (0x10000 a turn) about x, y and z, as the game stores them. y 0 faces south here."), |ui| {
                for k in 0..3 {
                    let mut r = a.rot[k] as u16;
                    if ui.add(egui::DragValue::new(&mut r).hexadecimal(4, false, true).speed(16.0)).changed() {
                        a.rot[k] = r as i16;
                    }
                }
            });
            if loose != 0 {
                ui.horizontal(|ui| {
                    widgets::hint(ui, format!("Bits {loose:#06x} aren't in any field."));
                    if widgets::button(ui, None, "Clear them", None, false).clicked() {
                        a.params &= !loose;
                    }
                });
            }
            widgets::hint(ui, "What's here is what the game gets. Every field above reads from these.");
        });
    }

    fn about_section(&mut self, ui: &mut egui::Ui, e: &Entry) {
        section(ui, "ins aabout", "About", None, false, |ui| {
            let plain = |s: &str| s.replace('`', "");
            ui.add(egui::Label::new(RichText::new(plain(&e.about)).size(12.0).color(MUTED)).wrap());
            if !e.params_doc.is_empty() && widgets::disclosure(ui, &format!("params {}", e.name), "Params, as the code reads them", false) {
                ui.add(egui::Label::new(RichText::new(plain(&e.params_doc)).size(12.0).color(MUTED)).wrap());
            }
            if !e.flags_doc.is_empty() && widgets::disclosure(ui, &format!("flags {}", e.name), "Flags", false) {
                ui.add(egui::Label::new(RichText::new(plain(&e.flags_doc)).size(12.0).color(MUTED)).wrap());
            }
        });
    }
}

/// The control for one field, by its kind. `v` is its value; `me` the actor being edited.
#[allow(clippy::too_many_arguments)]
fn field_widget(
    ui: &mut egui::Ui,
    id: (usize, &str),
    f: &Field,
    v: &mut i32,
    users: &BTreeMap<(&'static str, i32), Vec<(usize, String)>>,
    labels: &[String],
    me: usize,
    objects: &[String],
    paths: usize,
    select: &mut Option<usize>,
) {
    let max = if f.bits >= 16 { 0xFFFF } else { (1i32 << f.bits) - 1 };
    let (lo, hi) = if f.signed { (-(max + 1) / 2, max / 2) } else { (0, max) };
    let w = ui.available_width();
    match f.kind {
        Kind::Named(list) => {
            let text = list.iter().find(|x| x.0 == *v).map_or(format!("{v:#x} (not a named value)"), |x| x.1.to_string());
            egui::ComboBox::from_id_salt(("named", id)).selected_text(text).width(w).height(300.0).show_ui(ui, |ui| {
                for &(n, name) in list {
                    ui.selectable_value(v, n, format!("{name}  ({n})"));
                }
            });
        }
        Kind::Drop => {
            let text = ITEM00_NAMES.iter().find(|x| x.0 == *v).map_or("No drop".to_string(), |x| x.1.to_string());
            egui::ComboBox::from_id_salt(("drop", id)).selected_text(text).width(w).height(300.0).show_ui(ui, |ui| {
                ui.selectable_value(v, max, "No drop");
                for &(n, name) in ITEM00_NAMES.iter().filter(|x| x.0 <= max) {
                    ui.selectable_value(v, n, name);
                }
            });
        }
        Kind::ItemId => {
            let text = GI_NAMES.iter().find(|x| x.0 == *v).map_or(format!("{v:#x}"), |x| x.1.to_string());
            egui::ComboBox::from_id_salt(("gi", id)).selected_text(text).width(w).height(300.0).show_ui(ui, |ui| {
                for &(n, name) in GI_NAMES.iter().filter(|x| x.0 <= max) {
                    ui.selectable_value(v, n, format!("{name}  ({n:#04x})"));
                }
            });
        }
        Kind::SwitchFlag | Kind::TempFlag | Kind::ChestFlag | Kind::CollectibleFlag => {
            let kind = actors::flag_key(f, 0).map_or("switch", |k| k.0);
            let fmax = flag_max(f);
            let start = if f.kind == Kind::TempFlag { 0x20 } else { 0 };
            let others = |n: i32| -> Vec<usize> { users.get(&(kind, n)).map(|u| u.iter().map(|x| x.0).filter(|&j| j != me).collect()).unwrap_or_default() };
            let shared = others(*v);
            let text = if *v > fmax {
                format!("{v:#04x} (out of range: none)")
            } else if shared.is_empty() {
                format!("{v:#04x}")
            } else {
                format!("{v:#04x} · shared with {}", shared.len())
            };
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt(("flag", id)).selected_text(text).width((w - 52.0).max(80.0)).height(320.0).show_ui(ui, |ui| {
                    for n in start..=fmax {
                        let o = others(n);
                        let who = if o.is_empty() { String::new() } else { format!("   {}", o.iter().map(|&j| labels[j].as_str()).collect::<Vec<_>>().join(", ")) };
                        ui.selectable_value(v, n, format!("{n:#04x}{who}"));
                    }
                    if max > fmax {
                        ui.selectable_value(v, max, format!("{max:#04x}  none"));
                    }
                });
                if widgets::button(ui, None, "New", Some(46.0), false).on_hover_text("The lowest one no other actor in the level holds").clicked() {
                    if let Some(n) = (start..=fmax).find(|&n| others(n).is_empty()) {
                        *v = n;
                    }
                }
            });
            if !shared.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
                    ui.label(RichText::new("Shared with").size(11.5).color(FAINT));
                    for j in shared {
                        if widgets::chip(ui, &labels[j], None, false).on_hover_text("Select it").clicked() {
                            *select = Some(j);
                        }
                    }
                });
            }
        }
        Kind::TextId => {
            ui.add(egui::DragValue::new(v).hexadecimal(4, false, true).range(lo..=hi).speed(1.0));
        }
        Kind::PathIndex => {
            ui.add(egui::DragValue::new(v).range(lo..=hi).speed(0.1));
            ui.label(RichText::new(if paths == 0 { "the scene's paths: not editable yet" } else { "" }).size(11.0).color(FAINT));
        }
        Kind::Object => {
            ui.add(egui::DragValue::new(v).hexadecimal(3, false, true).range(lo..=hi).speed(0.2));
            ui.label(RichText::new(objects.get(*v as usize).map_or("", String::as_str)).monospace().size(11.0).color(FAINT));
        }
        Kind::Range(a, b) => {
            if b - a <= 1 && a == 0 {
                let mut on = *v != 0;
                let text = if on { "yes" } else { "no" };
                if widgets::toggle(ui, &mut on, text).changed() {
                    *v = on as i32;
                }
            } else {
                ui.add(egui::DragValue::new(v).range(a.max(lo)..=b.min(hi)).speed(if b - a > 64 { 1.0 } else { 0.1 }));
            }
        }
        Kind::Debug | Kind::Unused | Kind::Rotation => {
            ui.add(egui::DragValue::new(v).range(lo..=hi));
        }
    }
}

/// A frame with the panels' side margins.
fn pad(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new().inner_margin(Margin { left: 14, right: 14, top: 0, bottom: 10 }).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 6.0;
        add(ui);
    });
}

/// A row of the palette's actor list: its thumbnail (as the game draws it) with its group's dot,
/// its name and overlay, and how often the game places it. Dimmed when it can't be placed.
#[allow(clippy::too_many_arguments)]
fn list_row(ui: &mut egui::Ui, label: &str, name: &str, col: Color32, count: usize, on: bool, dim: bool, thumb: Option<egui::TextureId>) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 48.0), Sense::click());
    let p = ui.painter();
    if on {
        p.rect(rect, 6.0, style::soft(ACCENT, 0.14), Stroke::new(1.0, style::soft(ACCENT, 0.5)), StrokeKind::Inside);
    } else if resp.hovered() && !dim {
        p.rect_filled(rect, 6.0, RAISED);
    }
    let a = if dim { 0.45 } else { 1.0 };
    let t = Rect::from_min_size(Pos2::new(rect.left() + 4.0, rect.top() + 3.0), Vec2::splat(42.0));
    match thumb {
        Some(id) => {
            p.image(id, t, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE.gamma_multiply(a));
        }
        None => {
            p.rect_filled(t, 5.0, RAISED);
        }
    }
    p.circle(Pos2::new(t.left() + 5.0, t.bottom() - 5.0), 4.0, col.gamma_multiply(a), Stroke::new(1.0, Color32::from_black_alpha(160)));
    p.text(Pos2::new(rect.left() + 54.0, rect.top() + 16.0), Align2::LEFT_CENTER, label, FontId::proportional(12.5), if on { ACCENT } else { TEXT.gamma_multiply(a) });
    p.text(Pos2::new(rect.left() + 54.0, rect.bottom() - 15.0), Align2::LEFT_CENTER, name, FontId::monospace(10.5), FAINT);
    if count > 0 {
        p.text(Pos2::new(rect.right() - 8.0, rect.center().y), Align2::RIGHT_CENTER, format!("×{count}"), FontId::monospace(10.5), FAINT);
    }
    resp
}

/// A row of a list of configurations: its thumbnail, what it is, and a line under it.
fn variant_row(ui: &mut egui::Ui, what: &str, sub: &str, on: bool, thumb: Option<egui::TextureId>) -> egui::Response {
    let w = ui.available_width();
    let left = 52.0;
    let g = ui.painter().layout(what.to_string(), FontId::proportional(12.0), TEXT, w - left - 8.0);
    let h = (g.size().y + 22.0).max(48.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
    let p = ui.painter();
    let (fill, stroke) = if on { (style::soft(ACCENT, 0.12), style::soft(ACCENT, 0.55)) } else if resp.hovered() { (RAISED, LINE) } else { (PANEL2, LINE) };
    p.rect(rect, 6.0, fill, Stroke::new(1.0, stroke), StrokeKind::Inside);
    let t = Rect::from_min_size(Pos2::new(rect.left() + 4.0, rect.top() + 3.0), Vec2::splat(42.0));
    match thumb {
        Some(id) => {
            p.image(id, t, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        }
        None => {
            p.rect_filled(t, 5.0, RAISED);
        }
    }
    let gh = g.size().y;
    p.galley(Pos2::new(rect.left() + left, rect.top() + 5.0), g, if on { ACCENT } else { TEXT });
    p.text(Pos2::new(rect.left() + left, rect.top() + 7.0 + gh), Align2::LEFT_TOP, sub, FontId::proportional(10.5), FAINT);
    resp
}

