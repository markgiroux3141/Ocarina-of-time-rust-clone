//! The outliner (the right panel's bottom): everything in the level as a tree by kind, with the
//! plan's colours, heights, and a mark on anything the build has a problem with. Click selects
//! (Shift: several regions), double-click frames it.

use super::icons::{self, Icon};
use super::style::{self, ACCENT, FAINT, MUTED, PANEL2, RAISED, TEXT, WARN};
use super::widgets;
use super::{line_colour, App, Sel, PROP_COLOUR};
use crate::edit::{self, NodeRef};
use eframe::egui::{self, Align2, Color32, FontId, Id, Margin, Pos2, Rect, Sense, Stroke, Vec2};
use overworld::geom::P2;

/// One row of the tree.
struct Row {
    sel: Sel,
    name: String,
    meta: String,
    col: Color32,
}

impl App {
    pub(super) fn outliner(&mut self, ui: &mut egui::Ui) {
        let total = 1 + self.doc.regions.len() + self.doc.paths.len() + self.doc.lines.len() + self.doc.props.len();
        ui.horizontal(|ui| {
            ui.add_space(14.0);
            let g = ui.painter().layout_job(widgets::caps("Level contents", 10.5, MUTED));
            let (r, _) = ui.allocate_exact_size(Vec2::new(g.size().x, 30.0), Sense::hover());
            ui.painter().galley(Pos2::new(r.left(), r.center().y - g.size().y / 2.0), g, MUTED);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(14.0);
                ui.label(egui::RichText::new(total.to_string()).monospace().size(11.0).color(FAINT));
            });
        });
        egui::Frame::new().inner_margin(Margin { left: 12, right: 12, top: 0, bottom: 8 }).show(ui, |ui| {
            widgets::search(ui, &mut self.filter, "Filter by name");
        });
        let q = self.filter.to_lowercase();
        let flagged: Vec<Sel> = self.problems.iter().filter_map(|p| self.problem_target(p)).collect();
        let cur = match self.sel {
            Sel::Loop(l) | Sel::Node(NodeRef::Loop(l, _)) | Sel::Edge(l, _) => Some(Sel::Loop(l)),
            Sel::Path(p) | Sel::Node(NodeRef::Path(p, _)) => Some(Sel::Path(p)),
            Sel::Line(k) | Sel::Node(NodeRef::Line(k, _)) => Some(Sel::Line(k)),
            Sel::Prop(i) => Some(Sel::Prop(i)),
            Sel::None => None,
        };
        let loops = self.selected_loops();
        let is_sel = |s: Sel| cur == Some(s) || matches!(s, Sel::Loop(l) if loops.contains(&l));

        let mut terrain = vec![Row { sel: Sel::Loop(0), name: "Outline".into(), meta: format!("{:.0}", self.doc.outline.z), col: style::FLOOR }];
        for (i, r) in self.doc.regions.iter().enumerate() {
            let water = r.kind == "water";
            terrain.push(Row {
                sel: Sel::Loop(i + 1),
                name: edit::loop_name(&self.doc, i + 1),
                meta: format!("{}{}", if water { format!("bed {:.0}", r.z) } else { format!("{:.0}", r.z) }, profile_tag(r)),
                col: if water { style::WATER } else { style::FLOOR },
            });
        }
        let paths: Vec<Row> = self
            .doc
            .paths
            .iter()
            .enumerate()
            .map(|(k, p)| Row { sel: Sel::Path(k), name: edit::path_name(&self.doc, k), meta: if p.mode == "floating" { "bridge".into() } else { "ramp".into() }, col: style::PATH })
            .collect();
        let lines: Vec<Row> = self
            .doc
            .lines
            .iter()
            .enumerate()
            .map(|(k, l)| {
                let meta = match l.kind.as_str() {
                    "fence" => "rails".to_string(),
                    "lattice" => "lattice".to_string(),
                    "rock" => format!("rock {:.0}", l.contours.last().map_or(0.0, |c| c.z)),
                    "arch" => format!("arch {:.0}", l.height.or(self.theme.rocks.as_ref().map(|r| r.arch.height)).unwrap_or(0.0)),
                    k => k.to_string(),
                };
                Row { sel: Sel::Line(k), name: edit::line_name(&self.doc, k), meta, col: line_colour(&l.kind) }
            })
            .collect();
        // props by the kit's kinds, in the kit's order
        let mut kinds: Vec<(String, Vec<Row>)> = vec![];
        for (i, p) in self.doc.props.iter().enumerate() {
            let piece = self.kit.as_ref().and_then(|k| k.get(&p.piece));
            let kind = piece.map_or("other".to_string(), |x| x.kind.clone());
            let name = piece.map_or(p.piece.clone(), |x| x.label.clone());
            let row = Row { sel: Sel::Prop(i), name, meta: format!("{:.0}, {:.0}", p.at[0], p.at[1]), col: PROP_COLOUR };
            match kinds.iter_mut().find(|k| k.0 == kind) {
                Some(k) => k.1.push(row),
                None => kinds.push((kind, vec![row])),
            }
        }

        let mut clicked: Option<(Sel, bool)> = None;
        let mut frame: Option<Sel> = None;
        egui::ScrollArea::vertical().id_salt("outliner").auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let mut show = |ui: &mut egui::Ui, rows: &[Row], depth: f32| {
                for r in rows.iter().filter(|r| q.is_empty() || r.name.to_lowercase().contains(&q)) {
                    let resp = row(ui, r, depth, is_sel(r.sel), flagged.contains(&r.sel));
                    if resp.double_clicked() {
                        frame = Some(r.sel);
                    } else if resp.clicked() {
                        clicked = Some((r.sel, ui.input(|i| i.modifiers.shift)));
                    }
                }
            };
            let count = |rows: &[Row]| rows.iter().filter(|r| q.is_empty() || r.name.to_lowercase().contains(&q)).count();
            for (id, icon, title, rows) in [("terrain", Icon::Region, "Terrain", &terrain), ("paths", Icon::Path, "Paths", &paths), ("lines", Icon::Dirt, "Lines", &lines)] {
                let n = count(rows);
                if n == 0 && !q.is_empty() {
                    continue;
                }
                if group(ui, id, Some(icon), title, n, 0.0, !q.is_empty()) {
                    show(ui, rows, 1.0);
                }
            }
            let n: usize = kinds.iter().map(|k| count(&k.1)).sum();
            if (n > 0 || q.is_empty()) && group(ui, "props", Some(Icon::Prop), "Props", n, 0.0, !q.is_empty()) {
                for (kind, rows) in &kinds {
                    let m = count(rows);
                    if m == 0 {
                        continue;
                    }
                    if group(ui, &format!("props {kind}"), None, kind_title(kind), m, 1.0, !q.is_empty()) {
                        show(ui, rows, 2.0);
                    }
                }
            }
            ui.add_space(8.0);
        });
        if let Some((s, shift)) = clicked {
            self.click_select(s, shift);
        }
        if let Some(s) = frame {
            self.click_select(s, false);
            self.frame_sel();
        }
    }

    /// Fits the plan (and the 3D view) round the selection.
    pub(super) fn frame_sel(&mut self) {
        let pts: Vec<P2> = match self.sel {
            Sel::Loop(l) | Sel::Node(NodeRef::Loop(l, _)) | Sel::Edge(l, _) => self.shapes.loops.get(l).map(|c| c.iter().map(|x| x.0).collect()).unwrap_or_default(),
            Sel::Path(k) | Sel::Node(NodeRef::Path(k, _)) => self.shapes.paths.get(k).map(|c| c.iter().map(|x| x.0).collect()).unwrap_or_default(),
            Sel::Line(k) | Sel::Node(NodeRef::Line(k, _)) => self.shapes.lines.get(k).map(|c| c.iter().map(|x| x.0).collect()).unwrap_or_default(),
            Sel::Prop(i) if i < self.doc.props.len() => self.prop_footprint(i),
            _ => vec![],
        };
        if pts.is_empty() {
            return;
        }
        let mut bb = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
        for p in &pts {
            bb = [bb[0].min(p[0]), bb[1].min(p[1]), bb[2].max(p[0]), bb[3].max(p[1])];
        }
        let pad = 350.0;
        let bb = [bb[0] - pad, bb[1] - pad, bb[2] + pad, bb[3] + pad];
        self.view.fit(bb);
        let z = match self.sel {
            Sel::Loop(l) => edit::loop_z(&self.doc, l),
            Sel::Prop(i) => self.prop_z(i).unwrap_or(self.doc.outline.z),
            _ => self.doc.outline.z,
        };
        if let Some(v) = &mut self.v3 {
            v.cam.frame(bb, z, v.aspect);
        }
    }
}

fn kind_title(kind: &str) -> &str {
    match kind {
        "house" => "Houses",
        "tower" => "Stumps",
        "stone" => "Stepping stones",
        "hedge" => "Hedges",
        "opening" => "Openings",
        "wall" => "On walls",
        _ => "Other",
    }
}

/// A group's heading row, open or closed (remembered); always open while filtering. Returns
/// whether it's open.
fn group(ui: &mut egui::Ui, id: &str, icon: Option<Icon>, title: &str, n: usize, depth: f32, force_open: bool) -> bool {
    let key = Id::new(("outliner group", id));
    let mut open = ui.data_mut(|d| *d.get_persisted_mut_or(key, true));
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 26.0), Sense::click());
    let p = ui.painter();
    if resp.hovered() {
        p.rect_filled(rect, 0.0, RAISED);
    }
    let open_now = open || force_open;
    let x = rect.left() + 14.0 + depth * 18.0;
    let c = Pos2::new(x + 5.0, rect.center().y);
    let s = Stroke::new(2.0, FAINT);
    if open_now {
        p.add(egui::Shape::line(vec![c + Vec2::new(-4.0, -2.0), c + Vec2::new(0.0, 2.0), c + Vec2::new(4.0, -2.0)], s));
    } else {
        p.add(egui::Shape::line(vec![c + Vec2::new(-2.0, -4.0), c + Vec2::new(2.0, 0.0), c + Vec2::new(-2.0, 4.0)], s));
    }
    let mut tx = x + 16.0;
    if let Some(i) = icon {
        icons::paint(p, Rect::from_center_size(Pos2::new(tx + 7.0, rect.center().y), Vec2::splat(15.0)), i, MUTED, 1.8);
        tx += 21.0;
    }
    let font = if depth > 0.0 { FontId::proportional(12.5) } else { style::semibold(12.5) };
    p.text(Pos2::new(tx, rect.center().y), Align2::LEFT_CENTER, title, font, if depth > 0.0 { MUTED } else { TEXT });
    let g = p.layout_no_wrap(n.to_string(), FontId::monospace(10.5), FAINT);
    let pill = Rect::from_min_size(Pos2::new(rect.right() - 14.0 - g.size().x - 12.0, rect.center().y - 8.5), Vec2::new(g.size().x + 12.0, 17.0));
    p.rect_filled(pill, 8.5, PANEL2);
    p.galley(pill.center() - g.size() / 2.0, g, FAINT);
    if resp.clicked() && !force_open {
        open = !open;
        ui.data_mut(|d| d.insert_persisted(key, open));
    }
    open_now
}

fn row(ui: &mut egui::Ui, r: &Row, depth: f32, selected: bool, flagged: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 24.0), Sense::click());
    let p = ui.painter();
    if selected {
        p.rect_filled(rect, 0.0, style::soft(ACCENT, 0.14));
        p.rect_filled(Rect::from_min_size(rect.min + Vec2::new(0.0, 3.0), Vec2::new(2.0, rect.height() - 6.0)), 1.0, ACCENT);
    } else if resp.hovered() {
        p.rect_filled(rect, 0.0, RAISED);
    }
    let x = rect.left() + 16.0 + depth * 18.0;
    p.rect_filled(Rect::from_center_size(Pos2::new(x + 4.5, rect.center().y), Vec2::splat(9.0)), 2.5, r.col);
    let meta = p.layout_no_wrap(r.meta.clone(), FontId::monospace(11.0), FAINT);
    let right = rect.right() - 14.0;
    let meta_x = right - meta.size().x;
    let warn_w = if flagged { 20.0 } else { 0.0 };
    let name_w = (meta_x - warn_w - 8.0 - (x + 16.0)).max(20.0);
    let mut job = egui::text::LayoutJob::single_section(r.name.clone(), egui::TextFormat { font_id: FontId::proportional(12.5), color: TEXT, ..Default::default() });
    job.wrap.max_width = name_w;
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let name = p.layout_job(job);
    p.galley(Pos2::new(x + 16.0, rect.center().y - name.size().y / 2.0), name, TEXT);
    if flagged {
        icons::paint(p, Rect::from_center_size(Pos2::new(meta_x - 13.0, rect.center().y), Vec2::splat(14.0)), Icon::Warn, WARN, 2.0);
    }
    p.galley(Pos2::new(meta_x, rect.center().y - meta.size().y / 2.0), meta, FAINT);
    resp.on_hover_text(if flagged { "The build has a problem with it (see ⚠ at the top). Double-click: frame it." } else { "Click: select (Shift: several regions) · double-click: frame it" })
}

/// What a region's edges do besides cliffs, for its row: " · slope", " · terraces" or both.
fn profile_tag(r: &overworld::doc::Region) -> String {
    let mut names: Vec<&str> = vec![];
    for k in 0..r.nodes.len() {
        if let Some(p) = r.edge_profile(k) {
            let n = super::inspector::profile_name(Some(p));
            if !names.contains(&n) {
                names.push(n);
            }
        }
    }
    let pit = if r.kind == "pit" { " · pit" } else { "" };
    if names.is_empty() {
        return pit.into();
    }
    format!("{pit} · {}", names.join(", ").to_lowercase())
}
