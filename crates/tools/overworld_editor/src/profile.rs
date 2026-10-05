//! A path seen from the side: its height along its length over the ground under it, laid out
//! by the builder's own `paths::layout` (so landings and interpolated heights are exactly what
//! gets built). Nodes are dragged up and down; stretches steeper than the walkable slope are red.

use crate::edit::{self, Shapes};
use eframe::egui::{self, epaint, Align2, Color32, FontId, PointerButton, Pos2, Rect, Sense, Shape, Stroke, Vec2};
use overworld::doc::Doc;
use overworld::geom::P2;
use overworld::paths::{self, PathGeo};
use overworld::terrain::Field;

pub struct Profile {
    pub geo: PathGeo,
    /// The ground under each station (the regions' floors, before any path).
    pub ground: Vec<f64>,
    pub node_s: Vec<f64>,
    /// Each node's height as built, and whether the document gives it.
    pub node_z: Vec<f64>,
    /// The painted terrain under each node (built heights are the design's plus this).
    pub node_t: Vec<f64>,
    pub given: Vec<bool>,
    pub total: f64,
}

pub fn profile(doc: &Doc, shapes: &Shapes, k: usize) -> Result<Profile, String> {
    let base = |p: P2| edit::base_z(doc, shapes, p);
    let path = &doc.paths[k];
    let geo = paths::layout(path, &base, edit::path_sampling(doc))?;
    let xy: Vec<P2> = path.nodes.iter().map(|n| [n[0].unwrap_or(0.0), n[1].unwrap_or(0.0)]).collect();
    let (_, node_s) = paths::centre_line(&xy, edit::path_sampling(doc));
    let z_at = |s: f64| -> f64 {
        for w in geo.st.windows(2) {
            if s <= w[1].s + 1e-6 {
                let t = if w[1].s - w[0].s > 1e-9 { ((s - w[0].s) / (w[1].s - w[0].s)).clamp(0.0, 1.0) } else { 0.0 };
                return w[0].z + (w[1].z - w[0].z) * t;
            }
        }
        geo.st.last().map_or(0.0, |x| x.z)
    };
    let mut node_z: Vec<f64> = node_s.iter().map(|&s| z_at(s)).collect();
    let given = path.nodes.iter().map(|n| n.get(2).copied().flatten().is_some()).collect();
    let mut ground: Vec<f64> = geo.st.iter().map(|x| base(x.p)).collect();
    let total = geo.st.last().map_or(0.0, |x| x.s);
    let mut geo = geo;
    // painted terrain lifts the path and the ground under it alike
    let mut node_t = vec![0.0; node_z.len()];
    if let Some(t) = doc.terrain.as_ref() {
        let polys: Vec<Vec<P2>> = (0..shapes.loops.len()).map(|l| shapes.poly(l)).collect();
        let f = Field::new(t, doc, &polys);
        for (x, g) in geo.st.iter_mut().zip(ground.iter_mut()) {
            let dz = f.at(x.p);
            x.z += dz;
            *g += dz;
        }
        for (i, z) in node_z.iter_mut().enumerate() {
            node_t[i] = f.at(xy[i]);
            *z += node_t[i];
        }
        geo.max_slope = geo
            .st
            .windows(2)
            .map(|w| ((w[1].z - w[0].z).abs() / (w[1].s - w[0].s).max(1e-9)).atan().to_degrees())
            .fold(0.0, f64::max);
    }
    Ok(Profile { geo, ground, node_s, node_z, node_t, given, total })
}

pub struct Out {
    /// A node's new height (None: automatic again).
    pub set_z: Option<(usize, Option<f64>)>,
    pub select: Option<usize>,
    /// The point on the path under the pointer, to show on the plan.
    pub hover: Option<P2>,
}

pub struct ProfileView {
    pub exaggerate: f64,
    drag: Option<usize>,
    menu_node: Option<usize>,
}

impl Default for ProfileView {
    fn default() -> Self {
        ProfileView { exaggerate: 1.0, drag: None, menu_node: None }
    }
}

impl ProfileView {
    pub fn show(&mut self, ui: &mut egui::Ui, doc: &Doc, shapes: &Shapes, k: usize, selected: Option<usize>, max_slope: f64) -> Out {
        let mut out = Out { set_z: None, select: None, hover: None };
        let pr = match profile(doc, shapes, k) {
            Ok(p) => p,
            Err(e) => {
                ui.colored_label(Color32::from_rgb(255, 90, 90), e);
                return out;
            }
        };
        let steepest = pr.geo.max_slope;
        ui.horizontal(|ui| {
            ui.strong(format!("Profile: {}", edit::path_name(doc, k)));
            ui.label(format!("· {:.0} long · steepest {:.0}°", pr.total, steepest));
            if steepest > max_slope + 1e-9 {
                ui.colored_label(Color32::from_rgb(255, 90, 90), format!("(over the walkable {max_slope:.0}°)"));
            }
            ui.separator();
            ui.label("Heights");
            for e in [1.0, 2.0, 4.0] {
                if ui.selectable_label(self.exaggerate == e, format!("×{e:.0}")).on_hover_text("Vertical exaggeration (×1: true slopes)").clicked() {
                    self.exaggerate = e;
                }
            }
            ui.separator();
            ui.weak("drag a node up or down · right-click: automatic height");
        });
        let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = resp.rect;
        painter.rect_filled(rect, 0.0, Color32::from_rgb(28, 32, 36));
        let plot = Rect::from_min_max(rect.min + Vec2::new(46.0, 10.0), rect.max - Vec2::new(14.0, 18.0));
        if plot.width() < 20.0 || plot.height() < 20.0 || pr.geo.st.len() < 2 {
            return out;
        }
        let zs = pr.geo.st.iter().map(|x| x.z).chain(pr.ground.iter().copied());
        let (lo, hi) = zs.fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), z| (a.min(z), b.max(z)));
        let (zlo, zhi) = (lo - 40.0, hi + 60.0);
        let ex = self.exaggerate;
        let sh = (plot.width() as f64 / pr.total.max(1.0)).min(plot.height() as f64 / ((zhi - zlo) * ex));
        let sv = sh * ex;
        let x0 = plot.left() as f64 + (plot.width() as f64 - pr.total * sh) * 0.5;
        let yb = plot.bottom() as f64 - (plot.height() as f64 - (zhi - zlo) * sv) * 0.5;
        let to = |s: f64, z: f64| Pos2::new((x0 + s * sh) as f32, (yb - (z - zlo) * sv) as f32);
        let z_of_y = |y: f32| zlo + (yb - y as f64) / sv;

        // height lines every 50/100/250 units
        let step = [25.0, 50.0, 100.0, 250.0, 500.0].into_iter().find(|&st| st * sv >= 28.0).unwrap_or(1000.0);
        let mut z = (zlo / step).ceil() * step;
        while z < zhi {
            let y = to(0.0, z).y;
            painter.line_segment([Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)], Stroke::new(1.0, Color32::from_white_alpha(18)));
            painter.text(Pos2::new(rect.left() + 40.0, y), Align2::RIGHT_CENTER, format!("{:.0}", z + 0.0), FontId::monospace(11.0), Color32::from_gray(150));
            z += step;
        }
        // the ground, filled down to the bottom
        let bottom = plot.bottom() + 18.0;
        let st = &pr.geo.st;
        let mut mesh = epaint::Mesh::default();
        let ground_col = Color32::from_rgb(70, 96, 64);
        for i in 0..st.len() {
            let p = to(st[i].s, pr.ground[i]);
            let base = mesh.vertices.len() as u32;
            mesh.colored_vertex(p, ground_col);
            mesh.colored_vertex(Pos2::new(p.x, bottom), ground_col);
            if i > 0 {
                mesh.add_triangle(base - 2, base - 1, base);
                mesh.add_triangle(base - 1, base + 1, base);
            }
        }
        painter.add(Shape::mesh(mesh));
        let gl: Vec<Pos2> = (0..st.len()).map(|i| to(st[i].s, pr.ground[i])).collect();
        painter.add(Shape::line(gl, Stroke::new(1.5, Color32::from_rgb(120, 160, 100))));
        // embankments: solid from the path down to the ground; bridges: a deck over open air
        for r in &pr.geo.runs {
            if r.floating {
                continue;
            }
            let mut m = epaint::Mesh::default();
            let col = Color32::from_rgba_unmultiplied(150, 110, 70, 150);
            for i in r.i0..=r.i1 {
                let top = st[i].z.max(pr.ground[i]);
                let base = m.vertices.len() as u32;
                m.colored_vertex(to(st[i].s, top), col);
                m.colored_vertex(to(st[i].s, pr.ground[i].min(top)), col);
                if i > r.i0 {
                    m.add_triangle(base - 2, base - 1, base);
                    m.add_triangle(base - 1, base + 1, base);
                }
            }
            painter.add(Shape::mesh(m));
        }
        // the path's surface, red where it's too steep
        for i in 1..st.len() {
            let slope = ((st[i].z - st[i - 1].z).abs() / (st[i].s - st[i - 1].s).max(1e-9)).atan().to_degrees();
            let floating = pr.geo.runs.iter().any(|r| r.floating && i > r.i0 && i <= r.i1);
            let col = if slope > max_slope + 1e-9 {
                Color32::from_rgb(255, 70, 70)
            } else if floating {
                Color32::from_rgb(130, 200, 255)
            } else {
                Color32::from_rgb(240, 225, 200)
            };
            painter.line_segment([to(st[i - 1].s, st[i - 1].z), to(st[i].s, st[i].z)], Stroke::new(if floating { 4.0 } else { 2.5 }, col));
        }
        for r in pr.geo.runs.iter().filter(|r| r.floating) {
            let m = (r.i0 + r.i1) / 2;
            painter.text(to(st[m].s, st[m].z) + Vec2::new(0.0, -10.0), Align2::CENTER_BOTTOM, "bridge", FontId::proportional(11.0), Color32::from_rgb(130, 200, 255));
        }
        // slope per segment between nodes
        for w in 0..pr.node_s.len().saturating_sub(1) {
            let (a, b) = (pr.node_s[w], pr.node_s[w + 1]);
            let deg = st
                .windows(2)
                .filter(|x| x[0].s >= a - 1e-6 && x[1].s <= b + 1e-6)
                .map(|x| ((x[1].z - x[0].z).abs() / (x[1].s - x[0].s).max(1e-9)).atan().to_degrees())
                .fold(0.0, f64::max);
            let m = 0.5 * (a + b);
            let zm = 0.5 * (pr.node_z[w] + pr.node_z[w + 1]);
            let col = if deg > max_slope + 1e-9 { Color32::from_rgb(255, 110, 110) } else { Color32::from_gray(190) };
            painter.text(to(m, zm) + Vec2::new(0.0, 14.0), Align2::CENTER_TOP, format!("{deg:.0}°"), FontId::proportional(11.0), col);
        }
        // nodes
        let pos: Vec<Pos2> = (0..pr.node_s.len()).map(|i| to(pr.node_s[i], pr.node_z[i])).collect();
        let near = |p: Pos2| (0..pos.len()).filter(|&i| pos[i].distance(p) <= 12.0).min_by(|&a, &b| pos[a].distance(p).total_cmp(&pos[b].distance(p)));
        let hover_node = resp.hover_pos().and_then(near);
        for (i, &p) in pos.iter().enumerate() {
            let sel = selected == Some(i) || self.drag == Some(i);
            let col = if sel { Color32::from_rgb(255, 120, 40) } else { Color32::from_rgb(235, 130, 255) };
            let r = if hover_node == Some(i) { 8.0 } else { 6.0 };
            if pr.given[i] {
                painter.circle(p, r, col, Stroke::new(1.0, Color32::BLACK));
            } else {
                painter.circle(p, r, Color32::from_rgb(28, 32, 36), Stroke::new(2.0, col));
            }
            let text = if pr.given[i] { format!("{:.0}", pr.node_z[i]) } else { format!("auto {:.0}", pr.node_z[i]) };
            painter.text(p + Vec2::new(0.0, -10.0), Align2::CENTER_BOTTOM, text, FontId::proportional(11.0), col);
        }

        // interaction
        if resp.drag_started_by(PointerButton::Primary) {
            self.drag = resp.interact_pointer_pos().and_then(|_| ui.input(|i| i.pointer.press_origin())).and_then(near);
            if let Some(i) = self.drag {
                out.select = Some(i);
            }
        }
        if let (Some(i), true) = (self.drag, resp.dragged_by(PointerButton::Primary)) {
            if let Some(p) = resp.interact_pointer_pos() {
                let snap = if ui.input(|i| i.modifiers.shift) { 1.0 } else { 5.0 };
                // the profile shows built heights: the node keeps the design height under them
                out.set_z = Some((i, Some(((z_of_y(p.y) / snap).round() * snap - pr.node_t[i]).round())));
            }
        }
        if resp.drag_stopped() {
            self.drag = None;
        }
        if resp.clicked() {
            if let Some(i) = resp.interact_pointer_pos().and_then(near) {
                out.select = Some(i);
            }
        }
        if resp.secondary_clicked() {
            self.menu_node = resp.interact_pointer_pos().and_then(near);
        }
        if let Some(i) = self.menu_node {
            resp.context_menu(|ui| {
                if ui.button("Automatic height").on_hover_text("An end takes the floor's height; a middle node is interpolated").clicked() {
                    out.set_z = Some((i, None));
                    ui.close();
                }
            });
        }
        if let Some(h) = resp.hover_pos() {
            if plot.contains(h) {
                let s = ((h.x as f64 - x0) / sh).clamp(0.0, pr.total);
                if let Some(j) = st.iter().position(|x| x.s >= s) {
                    let x = &st[j];
                    out.hover = Some(x.p);
                    let p = to(x.s, x.z);
                    painter.line_segment([Pos2::new(p.x, plot.top()), Pos2::new(p.x, plot.bottom())], Stroke::new(1.0, Color32::from_white_alpha(50)));
                    painter.text(Pos2::new(p.x + 6.0, plot.top() + 2.0), Align2::LEFT_TOP, format!("{:.0} along · height {:.0}", x.s, x.z), FontId::monospace(11.0), Color32::from_gray(200));
                }
            }
        }
        out
    }
}
