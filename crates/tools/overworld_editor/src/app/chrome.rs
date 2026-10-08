//! The window round the views: the top bar (file, level settings, undo, the build's status and
//! problems, Play), the tool rail, the status bar, the overlays on the views, the keys sheet and
//! the level settings window.

use super::icons::{self, Icon};
use super::style::{self, ACCENT, ACCENT_INK, BAD, FAINT, FIELD, LINE, LINE2, MUTED, PANEL2, RAISED, TEXT, WARN};
use super::widgets::{self, field, section};
use super::{App, Layout, Sel, Tool, ROOT};
use crate::scene::Shading;
use eframe::egui::{self, Align, Align2, Color32, FontId, Id, Margin, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};
use overworld::textures::Library;
use overworld::{export, Theme};
use std::path::Path;
use std::sync::Arc;

/// A tool's icon, name, key, what it's for, and the hint for the status bar.
pub struct ToolInfo {
    pub icon: Icon,
    pub name: &'static str,
    pub key: &'static str,
    pub about: &'static str,
    pub hint: &'static str,
}

/// The rail's tools, in groups.
pub const RAIL: [&[Tool]; 5] = [
    &[Tool::Select],
    &[Tool::Region, Tool::Path],
    &[Tool::Brush],
    &[Tool::Prop],
    &[Tool::Line("dirt"), Tool::Line("fence"), Tool::Line("bridge"), Tool::Line("hedge"), Tool::Line("tunnel")],
];

pub fn tool_info(t: Tool) -> ToolInfo {
    let line = "click points along it · double-click or Enter to finish · Backspace: undo point · Esc: cancel";
    match t {
        Tool::Select => ToolInfo {
            icon: Icon::Select,
            name: "Select",
            key: "V",
            about: "Pick and move regions, paths, lines and props.",
            hint: "drag nodes (onto another node to share it; Alt: no snap) · double-click a line: add node · Del: delete · S: sharp · PgUp/PgDn: raise/sink · right-drag: pan · wheel: zoom",
        },
        Tool::Region => ToolInfo {
            icon: Icon::Region,
            name: "Region",
            key: "R",
            about: "A floor or a pond: a loop of nodes at one height.",
            hint: "click points (on a node or edge to share it) · click the first point or Enter to close · Backspace: undo point · Esc: cancel",
        },
        Tool::Path => ToolInfo {
            icon: Icon::Path,
            name: "Path",
            key: "P",
            about: "A ramp or a bridge from one floor to another.",
            hint: "click points · put the ends inside the floors they start and finish on (a ramp ending inside a plateau cuts into it) · double-click or Enter to finish · Esc: cancel",
        },
        Tool::Brush => ToolInfo {
            icon: Icon::Brush,
            name: "Brush",
            key: "B",
            about: "Paint the ground up and down, under everything.",
            hint: "drag to paint · Ctrl: lower · Shift: smooth · [ ]: size · 1-6: raise, lower, smooth, flatten, bumps, erase · right-drag: pan",
        },
        Tool::Prop => ToolInfo {
            icon: Icon::Prop,
            name: "Props",
            key: "K",
            about: "Houses, stumps, stones, openings and wall pieces from the kit.",
            hint: "click: place the chosen piece · drag a prop: move · its arrow's handle: turn (Q / E) · its corner: scale · Alt: no snap · Ctrl+D: duplicate · Del: delete",
        },
        Tool::Line("dirt") => ToolInfo { icon: Icon::Dirt, name: "Dirt path", key: "D", about: "Dirt painted into the floor, with dirt footsteps.", hint: line },
        Tool::Line("fence") => ToolInfo { icon: Icon::Fence, name: "Fence", key: "G", about: "Straight between its nodes, a post at every node.", hint: line },
        Tool::Line("bridge") => ToolInfo { icon: Icon::Bridge, name: "Rope bridge", key: "H", about: "Planks and ropes sagging from one floor to another.", hint: line },
        Tool::Line("tunnel") => ToolInfo {
            icon: Icon::Tunnel,
            name: "Tunnel",
            key: "U",
            about: "A passage through a wall: under a plateau, through a ridge, or from one area to another.",
            hint: "click on the floor in front of the wall it goes into, then any bends, then on the floor beyond · double-click or Enter to finish · Esc: cancel",
        },
        Tool::Line(_) => ToolInfo {
            icon: Icon::Hedge,
            name: "Hedge",
            key: "J",
            about: "Tall grass over a shape: Link wades through it.",
            hint: "click its corners (straight between them) · click the first point or Enter to close · Backspace: undo point · Esc: cancel",
        },
    }
}

/// Controls floating over a view, a row of them with `pivot` at `at`.
fn overlay(ui: &egui::Ui, id: &str, pivot: Align2, at: Pos2, add: impl FnOnce(&mut egui::Ui)) {
    egui::Area::new(Id::new(("overlay", id))).pivot(pivot).fixed_pos(at).order(egui::Order::Middle).show(ui.ctx(), |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            add(ui);
        });
    });
}

/// A translucent frame for controls over a view.
fn glass() -> egui::Frame {
    egui::Frame::new().fill(Color32::from_rgba_unmultiplied(18, 23, 21, 225)).stroke(Stroke::new(1.0, LINE2)).corner_radius(8.0).inner_margin(Margin::same(2))
}

/// A button in a glass frame: highlighted when `on`, dimmed when not `enabled`.
fn glass_button(ui: &mut egui::Ui, text: &str, on: bool, enabled: bool) -> egui::Response {
    let g = ui.painter().layout_no_wrap(text.to_string(), FontId::proportional(12.5), TEXT);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(g.size().x + 18.0, 24.0), if enabled { Sense::click() } else { Sense::hover() });
    let p = ui.painter();
    if on {
        p.rect_filled(rect, 6.0, RAISED);
    } else if resp.hovered() && enabled {
        p.rect_filled(rect, 6.0, style::soft(RAISED, 0.6));
    }
    let col = if !enabled {
        FAINT
    } else if on || resp.hovered() {
        TEXT
    } else {
        MUTED
    };
    p.galley(rect.center() - g.size() / 2.0, g, col);
    resp
}

impl App {
    // ---- top bar ----------------------------------------------------------------------------

    pub(super) fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let (r, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
            icons::paint(ui.painter(), r, Icon::Leaf, ACCENT, 2.0);
            ui.label(RichText::new("Overworld Editor").font(style::semibold(14.0)).color(TEXT));
            ui.add_space(4.0);
            let name = self.file.as_ref().and_then(|f| f.file_name()).map_or(format!("{}.json", self.doc.name), |f| f.to_string_lossy().into());
            let dirty = self.dirty();
            egui::Frame::new()
                .fill(PANEL2)
                .corner_radius(6.0)
                .inner_margin(Margin::symmetric(8, 3))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        if dirty {
                            let (r, _) = ui.allocate_exact_size(Vec2::splat(6.0), Sense::hover());
                            ui.painter().circle_filled(r.center(), 3.0, WARN);
                        }
                        ui.label(RichText::new(name).monospace().size(12.0).color(MUTED));
                    });
                })
                .response
                .on_hover_text(if dirty { "Unsaved changes (Ctrl+S saves)" } else { "Saved" });
            vsep(ui);
            ui.menu_button("File", |ui| self.file_menu(ui));
            if ui.add(egui::Button::selectable(self.show_level, "Level settings…")).on_hover_text("Detail, edges, walls, the edge of the world and sampling").clicked() {
                self.show_level = !self.show_level;
            }
            let can_undo = !self.undo.is_empty() || self.doc != self.committed;
            if undo_button(ui, Icon::Undo, can_undo).on_hover_text("Undo (Ctrl+Z)").clicked() && can_undo {
                self.undo();
            }
            let can_redo = !self.redo.is_empty();
            if undo_button(ui, Icon::Redo, can_redo).on_hover_text("Redo (Ctrl+Y)").clicked() && can_redo {
                self.redo();
            }
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                if play_button(ui).on_hover_text("Play the level in the game with child Link (W plays from the cursor; right-click: play from here)").clicked() {
                    self.play(None);
                }
                if widgets::icon_button(ui, Icon::Keys, 28.0, self.show_keys).on_hover_text("All keys (?)").clicked() {
                    self.show_keys = !self.show_keys;
                }
                vsep(ui);
                self.problems_button(ui);
                self.collision_meter(ui);
                self.build_pill(ui);
            });
        });
    }

    fn file_menu(&mut self, ui: &mut egui::Ui) {
        ui.set_min_width(250.0);
        ui.set_max_width(330.0);
        if ui.add(egui::Button::new("New").shortcut_text("Ctrl+N")).clicked() {
            ui.close();
            self.new_doc();
        }
        if ui.add(egui::Button::new("Open…").shortcut_text("Ctrl+O")).clicked() {
            ui.close();
            self.open();
        }
        if ui.add(egui::Button::new("Save").shortcut_text("Ctrl+S")).clicked() {
            ui.close();
            self.save(false);
        }
        if ui.add(egui::Button::new("Save as…").shortcut_text("Ctrl+Shift+S")).clicked() {
            ui.close();
            self.save(true);
        }
        ui.separator();
        if ui.button("Export now").on_hover_text(format!("Write the build to {}", self.out_dir.display())).clicked() {
            ui.close();
            match (&self.level, &self.built) {
                (Some(l), Some(d)) => match export::write(d, &self.theme, l, self.lib.as_deref(), &self.out_dir) {
                    Ok(_) => self.status = format!("exported to {}", self.out_dir.display()),
                    Err(e) => self.status = e,
                },
                _ => self.status = "nothing built yet".into(),
            }
        }
        if ui.checkbox(&mut self.live, "Export every build (the game reloads it)").changed() {
            self.sent = None;
        }
        ui.separator();
        ui.label(egui::RichText::new("SOURCES").size(10.5).color(FAINT));
        let src = |ui: &mut egui::Ui, what: &str, value: String, button: &str| -> bool {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(what).color(TEXT));
                    ui.add(egui::Label::new(RichText::new(value).size(11.0).color(FAINT)).truncate());
                });
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| ui.button(button).clicked()).inner
            })
            .inner
        };
        let theme_where = self.theme_file.as_ref().map_or("built in".to_string(), |f| short_path(f));
        if src(ui, &format!("Theme: {}", self.theme.name), theme_where, "Load…") {
            ui.close();
            if let Some(p) = rfd::FileDialog::new().add_filter("theme", &["json"]).set_directory(Path::new(ROOT).join("crates/tools/overworld/themes")).pick_file() {
                match Theme::load(&p.to_string_lossy()).and_then(|t| Theme::for_doc(&self.doc, Some(t))) {
                    Ok(t) => {
                        self.theme = Arc::new(t);
                        self.theme_file = Some(p);
                        self.sent = None;
                    }
                    Err(e) => self.status = e,
                }
            }
        }
        let lib_where = self.lib.as_ref().map_or("(none)".into(), |l| short_path(&l.dir));
        if src(ui, "Textures", lib_where, "Choose…") {
            ui.close();
            if let Some(d) = rfd::FileDialog::new().pick_folder() {
                match Library::load(&d) {
                    Ok(l) => {
                        self.lib = Some(Arc::new(l));
                        self.textures.clear();
                        self.thumbs = Default::default();
                        self.sent = None;
                    }
                    Err(e) => self.status = e,
                }
            }
        }
        if src(ui, "Export to", short_path(&self.out_dir), "Choose…") {
            ui.close();
            if let Some(d) = rfd::FileDialog::new().pick_folder() {
                self.out_dir = d;
                self.sent = None;
            }
        }
    }

    /// Built, building or failed; click for the build's numbers (triangles by object).
    fn build_pill(&mut self, ui: &mut egui::Ui) {
        let (dot, text) = if self.building() {
            (MUTED, "Building…".to_string())
        } else if self.build_error.is_some() {
            (BAD, "Build failed".to_string())
        } else if let Some(s) = &self.scene {
            (ACCENT, format!("Built · {} triangles · {:.0} ms", fmt_int(s.triangles), self.build_ms))
        } else {
            (MUTED, "Not built".to_string())
        };
        let g = ui.painter().layout_no_wrap(text, FontId::proportional(12.0), MUTED);
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(g.size().x + 34.0, 24.0), Sense::click());
        let p = ui.painter();
        p.rect(rect, 12.0, if resp.hovered() { RAISED } else { PANEL2 }, Stroke::new(1.0, LINE), StrokeKind::Inside);
        let c = Pos2::new(rect.left() + 14.0, rect.center().y);
        if self.building() {
            let t = ui.input(|i| i.time) as f32;
            p.circle_stroke(c, 4.5, Stroke::new(1.5, LINE2));
            let a = t * 6.0;
            p.line_segment([c, c + Vec2::new(a.cos(), a.sin()) * 4.5], Stroke::new(1.5, ACCENT));
            ui.ctx().request_repaint();
        } else {
            p.circle_filled(c, 6.5, style::soft(dot, 0.2));
            p.circle_filled(c, 4.0, dot);
        }
        p.galley(Pos2::new(rect.left() + 26.0, rect.center().y - g.size().y / 2.0), g, MUTED);
        let resp = resp.on_hover_text("The level rebuilds as you edit. Click for its numbers.");
        egui::Popup::from_toggle_button_response(&resp).show(|ui| {
            ui.set_width(330.0);
            self.build_details(ui);
        });
    }

    fn build_details(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing.y = 6.0;
        if let Some(e) = &self.shapes.error {
            widgets::callout(ui, e, BAD);
        }
        if let Some(e) = &self.build_error {
            widgets::callout(ui, e, BAD);
            if self.level.is_some() {
                widgets::hint(ui, "Showing the last good build.");
            }
        }
        if let Some(s) = &self.scene {
            ui.label(RichText::new(format!("{} triangles · rim {:.0} to {:.0} · {:.0} ms", fmt_int(s.triangles), s.rim.0, s.rim.1, self.build_ms)).color(TEXT));
        }
        if let Some(l) = &self.level {
            let props: usize = l.props.iter().map(|p| p.collision_vertices).sum();
            ui.label(RichText::new(format!(
                "Collision: {} of {} vertices{}",
                fmt_int(l.collision_vertices),
                fmt_int(overworld::props::MAX_COLLISION_VERTICES),
                if props > 0 { format!(" (props about {})", fmt_int(props)) } else { String::new() }
            ))
            .color(if l.collision_vertices > overworld::props::MAX_COLLISION_VERTICES { BAD } else { MUTED }));
            ui.add_space(4.0);
            let g = ui.painter().layout_job(widgets::caps(&format!("Triangles by object · {} detail", self.doc.settings.detail), 10.0, FAINT));
            let (r, _) = ui.allocate_exact_size(g.size(), Sense::hover());
            ui.painter().galley(r.min, g, FAINT);
            let mut objs: Vec<(&str, usize)> = l.mesh.objects.iter().map(|o| (o.name.as_str(), o.tris.len())).filter(|o| o.1 > 0).collect();
            objs.sort_by(|a, b| b.1.cmp(&a.1));
            let most = objs.first().map_or(1, |o| o.1).max(1);
            for (n, t) in objs {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 18.0), Sense::hover());
                let p = ui.painter();
                let label_w = 120.0;
                p.text(Pos2::new(rect.left(), rect.center().y), Align2::LEFT_CENTER, n, FontId::proportional(12.0), MUTED);
                let bar = Rect::from_min_max(Pos2::new(rect.left() + label_w, rect.center().y - 4.0), Pos2::new(rect.right() - 46.0, rect.center().y + 4.0));
                p.rect_filled(bar, 3.0, FIELD);
                let w = bar.width() * t as f32 / most as f32;
                p.rect_filled(Rect::from_min_size(bar.min, Vec2::new(w.max(2.0), bar.height())), 3.0, style::soft(ACCENT, 0.75));
                p.text(Pos2::new(rect.right(), rect.center().y), Align2::RIGHT_CENTER, fmt_int(t), FontId::monospace(11.5), TEXT);
            }
            widgets::hint(ui, "For scale: Kokiri Forest is about 3,750 triangles in all (its village about 1,200), in a space about an eighth the size of the sketch levels.");
        }
    }

    /// The collision budget: the game indexes collision vertices in 13 bits.
    fn collision_meter(&self, ui: &mut egui::Ui) {
        let Some(l) = &self.level else { return };
        let max = overworld::props::MAX_COLLISION_VERTICES;
        let used = l.collision_vertices;
        let f = used as f32 / max as f32;
        let col = if f > 1.0 {
            BAD
        } else if f > 0.85 {
            WARN
        } else {
            ACCENT
        };
        let resp = ui
            .horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 7.0;
                ui.label(RichText::new(format!("/ {}", fmt_int(max))).monospace().size(12.0).color(MUTED));
                ui.label(RichText::new(fmt_int(used)).monospace().size(12.0).color(if f > 1.0 { BAD } else { TEXT }));
                let (r, _) = ui.allocate_exact_size(Vec2::new(100.0, 8.0), Sense::hover());
                let p = ui.painter();
                p.rect(r, 4.0, FIELD, Stroke::new(1.0, LINE), StrokeKind::Inside);
                p.rect_filled(Rect::from_min_size(r.min, Vec2::new(r.width() * f.min(1.0), r.height())), 4.0, col);
                ui.label(RichText::new("Collision").size(12.0).color(MUTED));
            })
            .response;
        resp.on_hover_text("The game indexes collision vertices in 13 bits. Over the limit it refuses the level: build at a lower detail (Level settings).");
    }

    /// The problems: a warning button with their count, opening the list (click one to select
    /// what it's about).
    fn problems_button(&mut self, ui: &mut egui::Ui) {
        let n = self.problems.len() + self.build_error.is_some() as usize;
        if n == 0 {
            return;
        }
        let g = ui.painter().layout_no_wrap(n.to_string(), FontId::monospace(10.5), Color32::from_rgb(0x24, 0x17, 0x03));
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(g.size().x + 40.0, 26.0), Sense::click());
        let p = ui.painter();
        if resp.hovered() {
            p.rect_filled(rect, 6.0, RAISED);
        }
        let col = if self.build_error.is_some() { BAD } else { WARN };
        icons::paint(p, Rect::from_center_size(Pos2::new(rect.left() + 13.0, rect.center().y), Vec2::splat(16.0)), Icon::Warn, col, 1.9);
        let badge = Rect::from_min_size(Pos2::new(rect.left() + 25.0, rect.center().y - 8.0), Vec2::new(g.size().x + 9.0, 16.0));
        p.rect_filled(badge, 8.0, col);
        p.galley(badge.center() - g.size() / 2.0, g, Color32::from_rgb(0x24, 0x17, 0x03));
        let resp = resp.on_hover_text("Problems in the build");
        let mut go = None;
        egui::Popup::from_toggle_button_response(&resp).show(|ui| {
            ui.set_width(360.0);
            let g = ui.painter().layout_job(widgets::caps("Problems", 10.0, FAINT));
            let (r, _) = ui.allocate_exact_size(g.size(), Sense::hover());
            ui.painter().galley(r.min, g, FAINT);
            if let Some(e) = &self.build_error {
                widgets::callout(ui, e, BAD);
            }
            for prob in &self.problems {
                let target = self.problem_target(prob);
                let resp = ui
                    .horizontal_top(|ui| {
                        let (r, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                        icons::paint(ui.painter(), r, Icon::Warn, WARN, 1.9);
                        ui.add(egui::Label::new(RichText::new(prob).color(TEXT).size(12.5)).wrap().sense(Sense::click()))
                    })
                    .inner;
                if let Some(t) = target {
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Select it").clicked() {
                        go = Some(t);
                        ui.close();
                    }
                }
            }
        });
        if let Some(t) = go {
            self.click_select(t, false);
            self.frame_sel();
        }
    }

    // ---- the tool rail ------------------------------------------------------------------------

    pub(super) fn rail(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.spacing_mut().item_spacing.y = 3.0;
            ui.add_space(4.0);
            for (g, tools) in RAIL.iter().enumerate() {
                if g > 0 {
                    ui.add_space(3.0);
                    let (r, _) = ui.allocate_exact_size(Vec2::new(26.0, 1.0), Sense::hover());
                    ui.painter().rect_filled(r, 0.0, LINE);
                    ui.add_space(3.0);
                }
                for &t in tools.iter() {
                    let info = tool_info(t);
                    let on = self.tool == t;
                    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::click());
                    let p = ui.painter();
                    if on {
                        p.rect(rect, 9.0, style::soft(ACCENT, 0.14), Stroke::new(1.0, style::soft(ACCENT, 0.55)), StrokeKind::Inside);
                    } else if resp.hovered() {
                        p.rect_filled(rect, 9.0, RAISED);
                    }
                    let col = if on {
                        ACCENT
                    } else if resp.hovered() {
                        TEXT
                    } else {
                        MUTED
                    };
                    icons::paint(p, Rect::from_center_size(rect.center(), Vec2::splat(21.0)), info.icon, col, 1.8);
                    p.text(rect.right_bottom() + Vec2::new(-4.0, -3.0), Align2::RIGHT_BOTTOM, info.key, FontId::monospace(9.0), if on { ACCENT } else { FAINT });
                    let resp = resp.on_hover_ui(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(info.name).font(style::semibold(13.0)).color(TEXT));
                            widgets::kbd(ui, info.key);
                        });
                        ui.label(RichText::new(info.about).color(MUTED).size(12.0));
                    });
                    if resp.clicked() {
                        self.set_tool(t);
                    }
                }
            }
        });
    }

    // ---- the status bar -----------------------------------------------------------------------

    pub(super) fn status_bar(&mut self, ui: &mut egui::Ui) {
        let info = tool_info(self.tool);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                if let Some(c) = self.cursor {
                    let z = self.scene.as_ref().and_then(|s| s.floor_z(c));
                    ui.label(RichText::new(format!("x {:6.0}  y {:6.0}  floor {}", c[0], c[1], z.map_or("-".into(), |z| format!("{z:.0}")))).monospace().size(12.0).color(MUTED));
                }
                if !self.status.is_empty() {
                    ui.add(egui::Label::new(RichText::new(&self.status).size(12.0).color(TEXT)).truncate());
                    vsep(ui);
                }
                ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(15.0), Sense::hover());
                    icons::paint(ui.painter(), r, info.icon, ACCENT, 1.9);
                    ui.label(RichText::new(info.name).font(style::semibold(12.5)).color(ACCENT));
                    ui.add_space(4.0);
                    let hint = if self.tool == Tool::Prop && self.chosen_on_wall() {
                        "hover a wall (best in 3D): the ghost shows where it goes, or why it can't · click: put it there · drag one along the walls to move it · Del: delete"
                    } else {
                        info.hint
                    };
                    ui.add(egui::Label::new(RichText::new(hint).size(12.0).color(MUTED)).truncate());
                });
            });
        });
    }

    // ---- over the views ---------------------------------------------------------------------

    /// The plan's own controls: shading and nodes at the top left, zoom and fit at the bottom
    /// right.
    pub(super) fn plan_overlays(&mut self, ui: &mut egui::Ui, rect: Rect) {
        overlay(ui, "plan shading", Align2::LEFT_TOP, rect.left_top() + Vec2::new(10.0, 10.0), |ui| {
            glass().show(ui, |ui| {
                for (s, name) in [(Shading::Textured, "Textured"), (Shading::Height, "Heights"), (Shading::Off, "Lines")] {
                    if glass_button(ui, name, self.shading == s, true).on_hover_text("T cycles").clicked() {
                        self.shading = s;
                    }
                }
            });
            ui.add_space(4.0);
            glass().show(ui, |ui| {
                if glass_button(ui, "Nodes", self.show_nodes, true).on_hover_text("Show the nodes (N)").clicked() {
                    self.show_nodes = !self.show_nodes;
                }
            });
        });
        overlay(ui, "plan zoom", Align2::RIGHT_BOTTOM, rect.right_bottom() - Vec2::new(10.0, 10.0), |ui| {
            glass().show(ui, |ui| {
                let c = self.view.rect.center();
                if glass_button(ui, "+", false, true).on_hover_text("Zoom in (wheel)").clicked() {
                    self.view.zoom_at(c, 1.25);
                }
                if glass_button(ui, "−", false, true).on_hover_text("Zoom out (wheel)").clicked() {
                    self.view.zoom_at(c, 0.8);
                }
                if glass_button(ui, "Fit", false, true).on_hover_text("Fit the level (F)").clicked() {
                    self.fit();
                }
            });
        });
    }

    /// Plan, plan + 3D or 3D, at the top right of the views.
    pub(super) fn layout_overlay(&mut self, ui: &mut egui::Ui, rect: Rect) {
        overlay(ui, "layout", Align2::RIGHT_TOP, rect.right_top() + Vec2::new(-10.0, 10.0), |ui| {
            glass().show(ui, |ui| {
                let has3 = self.v3.is_some();
                for (l, name) in [(Layout::Plan, "Plan"), (Layout::Split, "Plan + 3D"), (Layout::ThreeD, "3D")] {
                    let ok = l == Layout::Plan || has3;
                    let r = glass_button(ui, name, self.layout == l, ok);
                    let r = if ok { r } else { r.on_hover_text("The 3D view needs the wgpu backend") };
                    if r.clicked() {
                        self.layout = l;
                    }
                }
            });
        });
    }

    // ---- windows ------------------------------------------------------------------------------

    pub(super) fn windows(&mut self, ctx: &egui::Context) {
        if self.show_keys {
            let m = egui::Modal::new(Id::new("keys")).show(ctx, |ui| {
                ui.set_width(760.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Keys").font(style::semibold(17.0)).color(TEXT));
                    ui.label(RichText::new("? opens this, Esc closes it").size(12.0).color(FAINT));
                });
                ui.add_space(8.0);
                keys_sheet(ui);
            });
            if m.should_close() {
                self.show_keys = false;
            }
        }
        if self.show_level {
            let mut open = true;
            egui::Window::new(RichText::new("Level settings").font(style::semibold(15.0)))
                .id(Id::new("level settings"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_pos(Pos2::new(ctx.content_rect().center().x - 190.0, 90.0))
                .frame(egui::Frame::window(&ctx.global_style()).inner_margin(Margin::same(0)))
                .show(ctx, |ui| {
                    ui.set_width(380.0);
                    self.level_settings(ui);
                });
            self.show_level &= open;
        }
    }

    fn level_settings(&mut self, ui: &mut egui::Ui) {
        section(ui, "level", "Level", None, true, |ui| {
            field(ui, "Name", Some("Also the export folder's name"), |ui| ui.add(egui::TextEdit::singleline(&mut self.doc.name).desired_width(ui.available_width())));
            let themes: Vec<(String, &str)> = overworld::theme::BUILTIN.iter().map(|&t| (t.to_string(), overworld::kit::scene(t).map_or(t, |sc| sc.label))).collect();
            if field(
                ui,
                "Theme",
                Some("How the level looks: every wall, floor and tree not pinned to another theme's style. Styles of other themes stay pinned when you switch."),
                |ui| widgets::segmented(ui, &mut self.doc.settings.theme, &themes),
            ) {
                // a theme file loaded by hand gives way to the one chosen here
                self.theme_file = None;
            }
            let s = &mut self.doc.settings;
            field(
                ui,
                "Detail",
                Some("Mesh resolution. High: curves every 'Curve sample' units, walls a band per texture repeat. Medium and Low: curves sampled by how much they bend, walls in three bands, coarser floors and bridges."),
                |ui| widgets::segmented(ui, &mut s.detail, &[("high".to_string(), "High"), ("medium".to_string(), "Medium"), ("low".to_string(), "Low")]),
            );
            let mut edges = if s.edges.is_empty() { "smooth".to_string() } else { s.edges.clone() };
            if field(
                ui,
                "Edges",
                Some("How outlines, regions and paths run between their nodes. Smooth: curves. Faceted: the same curves in a few long straight pieces, low-poly like the game's own. Hard: straight from node to node, every node a corner."),
                |ui| widgets::segmented(ui, &mut edges, &[("smooth".to_string(), "Smooth"), ("faceted".to_string(), "Faceted"), ("hard".to_string(), "Hard")]),
            ) {
                s.edges = edges;
            }
            let mut walls = if s.wall_texture.is_empty() { "tiled".to_string() } else { s.wall_texture.clone() };
            if field(
                ui,
                "Walls",
                Some("How the cliffs are textured. Tiled: the grassy top and bottom keep their size and the rock between repeats, sharp on any wall. Middle: the top and bottom keep their size and the rock between is stretched once over the rest. Stretched: the texture once over the wall's height, as Kokiri Forest's own walls are."),
                |ui| widgets::segmented(ui, &mut walls, &[("tiled".to_string(), "Tiled"), ("stretched_middle".to_string(), "Middle"), ("stretched".to_string(), "Stretched")]),
            ) {
                s.wall_texture = walls;
            }
        });
        section(ui, "boundary", "Edge of the world", None, true, |ui| {
            let b = &mut self.doc.boundary;
            let row = |ui: &mut egui::Ui, name: &str, tip: &str, v: &mut f64, speed: f64| {
                field(ui, name, Some(tip), |ui| ui.add(egui::DragValue::new(v).speed(speed).range(0.0..=100000.0)));
            };
            row(ui, "Cliff", "The rim stands at least this far above the floors at the edge", &mut b.cliff_min, 1.0);
            row(ui, "Bank depth", "How far the bank reaches back to the tree line", &mut b.bank, 1.0);
            row(ui, "Bank rise", "The bank's rise above the rim, over its depth", &mut b.bank_rise, 1.0);
            row(ui, "Rise slope", "How fast the rim may climb along the edge (1 in 4 = 0.25)", &mut b.rise_slope, 0.005);
            row(ui, "Reach", "Floors this close to the edge raise the rim", &mut b.reach, 1.0);
            row(ui, "Panels", "How far the tree line's straight panels may stray from the bank", &mut b.panel_tol, 1.0);
        });
        section(ui, "sampling", "Sampling", None, true, |ui| {
            let s = &mut self.doc.settings;
            field(ui, "Curve sample", Some("Curves are sampled about this often"), |ui| ui.add(egui::DragValue::new(&mut s.sample).speed(1.0).range(10.0..=1000.0)));
            field(ui, "Floor points", Some("Interior points in floors, this far apart"), |ui| ui.add(egui::DragValue::new(&mut s.steiner).speed(1.0).range(50.0..=5000.0)));
            field(ui, "Weld", Some("Nodes closer than this are one node"), |ui| ui.add(egui::DragValue::new(&mut s.weld).speed(0.1).range(0.0..=50.0)));
            field(ui, "Seed", Some("Varies the texture and shade noise"), |ui| ui.add(egui::DragValue::new(&mut s.seed)));
        });
    }

    /// What a build problem is about, if it names something in the document.
    pub(super) fn problem_target(&self, p: &str) -> Option<Sel> {
        let index = |rest: &str| rest.split(|c: char| !c.is_ascii_digit()).next().and_then(|n| n.parse::<usize>().ok());
        if let Some(rest) = p.strip_prefix("prop ") {
            return index(rest).filter(|&i| i < self.doc.props.len()).map(Sel::Prop);
        }
        if let Some(rest) = p.strip_prefix("line ") {
            return index(rest).filter(|&i| i < self.doc.lines.len()).map(Sel::Line);
        }
        if let Some(rest) = p.strip_prefix("region ") {
            return index(rest).filter(|&i| i < self.doc.regions.len()).map(|i| Sel::Loop(i + 1));
        }
        if let Some(rest) = p.strip_prefix("path ") {
            let name = rest.split(':').next()?;
            return self.doc.paths.iter().position(|q| q.name == name).map(Sel::Path);
        }
        // "<kind> <name>: ..." or "<kind> \"<name>\": ..." for lines
        let (head, _) = p.split_once(": ")?;
        let (kind, name) = head.split_once(' ')?;
        let name = name.trim_matches('"');
        self.doc.lines.iter().position(|l| (l.kind == kind || kind == "bridge" && l.kind == "bridge") && l.name == name).map(Sel::Line)
    }
}

fn vsep(ui: &mut egui::Ui) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(9.0, 20.0), Sense::hover());
    ui.painter().line_segment([r.center_top(), r.center_bottom()], Stroke::new(1.0, LINE));
}

fn undo_button(ui: &mut egui::Ui, icon: Icon, enabled: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
    let p = ui.painter();
    if enabled && resp.hovered() {
        p.rect_filled(rect, 6.0, RAISED);
    }
    icons::paint(p, rect.shrink(6.0), icon, if !enabled { style::soft(MUTED, 0.45) } else if resp.hovered() { TEXT } else { MUTED }, 1.9);
    resp
}

fn play_button(ui: &mut egui::Ui) -> egui::Response {
    let g = ui.painter().layout_no_wrap("Play".into(), style::semibold(13.0), ACCENT_INK);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(g.size().x + 40.0, 28.0), Sense::click());
    let p = ui.painter();
    p.rect_filled(rect, 7.0, if resp.hovered() { ACCENT.lerp_to_gamma(Color32::WHITE, 0.12) } else { ACCENT });
    icons::paint(p, Rect::from_center_size(Pos2::new(rect.left() + 16.0, rect.center().y), Vec2::splat(13.0)), Icon::Play, ACCENT_INK, 1.0);
    p.galley(Pos2::new(rect.left() + 27.0, rect.center().y - g.size().y / 2.0), g, ACCENT_INK);
    resp
}

/// Every key, by what it's for.
fn keys_sheet(ui: &mut egui::Ui) {
    let groups: [(&str, &[(&str, &str)]); 6] = [
        ("Tools", &[("V", "Select"), ("R", "Draw a region"), ("P", "Draw a path"), ("B", "Brush"), ("K", "Props"), ("D", "Dirt path"), ("G", "Fence"), ("H", "Rope bridge"), ("J", "Hedge"), ("U", "Tunnel")]),
        (
            "Editing",
            &[
                ("Drag", "Move a node, and every loop sharing it"),
                ("Alt", "Drag without snapping"),
                ("Double-click", "Add a node to a line"),
                ("Del", "Delete the node, region, path or prop"),
                ("S", "Sharp or smooth corner"),
                ("Shift click", "Select several regions"),
                ("PgUp PgDn", "Raise or sink 20 (Shift: 100)"),
                ("Ctrl Z / Y", "Undo, redo"),
            ],
        ),
        ("Props", &[("Q E", "Turn 15° (Shift: 1°)"), ("Ctrl D", "Duplicate"), ("Drag", "Move; the arrow's handle turns, the corner scales")]),
        ("Brush", &[("Drag", "Paint"), ("Ctrl", "Lower"), ("Shift", "Smooth"), ("[ ]", "Size"), ("1–6", "Raise, lower, smooth, flatten, bumps, erase")]),
        ("View", &[("Wheel", "Zoom"), ("Right drag", "Pan"), ("F", "Fit"), ("T", "Textured, heights, lines"), ("N", "Nodes"), ("?", "This sheet")]),
        ("Files and play", &[("W", "Play from the cursor"), ("Ctrl S", "Save"), ("Ctrl O", "Open"), ("Ctrl N", "New level")]),
    ];
    egui::Grid::new("keys sheet").num_columns(3).spacing(Vec2::new(28.0, 18.0)).show(ui, |ui| {
        for (i, (title, rows)) in groups.iter().enumerate() {
            ui.vertical(|ui| {
                ui.set_width(220.0);
                let g = ui.painter().layout_job(widgets::caps(title, 10.5, MUTED));
                let (r, _) = ui.allocate_exact_size(g.size(), Sense::hover());
                ui.painter().galley(r.min, g, MUTED);
                ui.add_space(2.0);
                for (k, what) in rows.iter() {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 3.0;
                        ui.allocate_ui_with_layout(Vec2::new(92.0, 20.0), egui::Layout::left_to_right(Align::Center), |ui| {
                            ui.set_min_width(92.0);
                            for part in k.split(' ') {
                                widgets::kbd(ui, part);
                            }
                        });
                        ui.label(RichText::new(*what).size(12.5).color(MUTED));
                    });
                }
            });
            if i % 3 == 2 {
                ui.end_row();
            }
        }
    });
}

/// A path under the repository as it reads from there (out/overworld/...); others whole.
fn short_path(p: &Path) -> String {
    p.strip_prefix(ROOT).map_or_else(|_| p.display().to_string(), |r| r.display().to_string().replace('\\', "/"))
}

pub fn fmt_int(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// The panels' backgrounds, for the window's frames.
pub fn panel_frame(fill: Color32, margin: Margin) -> egui::Frame {
    egui::Frame::new().fill(fill).inner_margin(margin)
}
