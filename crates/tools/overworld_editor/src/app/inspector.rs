//! The inspector (the right panel's top): the selection's header card, its problems, and its
//! settings in sections. Values the theme supplies show greyed until set, with ↺ to go back.

use super::icons::{self, Icon};
use super::palette::{style_combo, tile_backdrop};
use super::style::{self, ACCENT, BAD, FAINT, FIELD, LINE, LINE2, MUTED, PANEL2, TEXT, WARN};
use super::widgets::{self, field, section};
use super::{line_colour, App, Sel, PROP_COLOUR};
use crate::edit::{self, NodeRef};
use eframe::egui::{self, Color32, Margin, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};
use overworld::doc::{rock_looks, stack_looks, Contour, Layers, Part, Profile, Prop, RockLook, Skyline, StackLook};

/// What the header card shows on the left.
enum Tile {
    Icon(Icon),
    Thumb(Option<egui::TextureId>),
}

/// The header's name: editable, or fixed.
enum Name<'a> {
    Edit(&'a mut String),
    Fixed(String),
}

/// The header card: the thing's icon (or thumbnail), what it is, its name, and a delete button.
/// Returns whether delete was clicked.
fn header(ui: &mut egui::Ui, tile: Tile, eyebrow: &str, col: Color32, name: Name, deletable: bool) -> bool {
    let mut delete = false;
    egui::Frame::new().inner_margin(Margin { left: 14, right: 12, top: 13, bottom: 12 }).show(ui, |ui| {
        ui.horizontal(|ui| {
            match tile {
                Tile::Icon(i) => {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::hover());
                    ui.painter().rect(r, 9.0, PANEL2, Stroke::new(1.0, LINE), StrokeKind::Inside);
                    icons::paint(ui.painter(), r.shrink(11.0), i, col, 1.8);
                }
                Tile::Thumb(t) => {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(60.0), Sense::hover());
                    tile_backdrop(ui.painter(), r);
                    ui.painter().rect_stroke(r, 9.0, Stroke::new(1.0, LINE), StrokeKind::Inside);
                    if let Some(t) = t {
                        ui.painter().image(t, r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                    }
                }
            }
            let right = if deletable { 36.0 } else { 0.0 };
            ui.vertical(|ui| {
                ui.set_width(ui.available_width() - right);
                ui.spacing_mut().item_spacing.y = 1.0;
                let g = ui.painter().layout_job(widgets::caps(eyebrow, 10.5, col));
                let (r, _) = ui.allocate_exact_size(g.size(), Sense::hover());
                ui.painter().galley(r.min, g, col);
                match name {
                    Name::Edit(s) => {
                        ui.add(egui::TextEdit::singleline(s).font(style::semibold(15.0)).frame(egui::Frame::NONE).desired_width(ui.available_width()).margin(Margin::symmetric(0, 2)))
                            .on_hover_text("Its name: click to rename");
                    }
                    Name::Fixed(s) => {
                        ui.add(egui::Label::new(RichText::new(s).font(style::semibold(15.0)).color(TEXT)).truncate());
                    }
                }
            });
            if deletable {
                let (r, resp) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::click());
                let red = Color32::from_rgb(0xff, 0x9a, 0x90);
                if resp.hovered() {
                    ui.painter().rect(r, 8.0, style::soft(BAD, 0.12), Stroke::new(1.0, style::soft(BAD, 0.4)), StrokeKind::Inside);
                }
                icons::paint(ui.painter(), r.shrink(7.5), Icon::Trash, if resp.hovered() { red } else { MUTED }, 1.8);
                delete = resp.on_hover_text("Delete (Del)").clicked();
            }
        });
    });
    delete
}

/// A count, and what to do with it, as chips.
fn chips(ui: &mut egui::Ui, items: &[(Option<String>, &str)]) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(5.0, 5.0);
        for (v, t) in items {
            widgets::chip(ui, t, v.as_deref(), false);
        }
    });
}

impl App {
    pub(super) fn inspector(&mut self, ui: &mut egui::Ui) {
        if !self.multi.is_empty() {
            self.multi_ui(ui);
            return;
        }
        match self.sel {
            Sel::None => empty(ui),
            Sel::Loop(l) | Sel::Node(NodeRef::Loop(l, _)) | Sel::Edge(l, _) => self.loop_ui(ui, l),
            Sel::Path(k) | Sel::Node(NodeRef::Path(k, _)) => self.path_ui(ui, k),
            Sel::Line(k) | Sel::Node(NodeRef::Line(k, _)) => self.line_ui(ui, k),
            Sel::Prop(i) => self.prop_ui(ui, i),
        }
    }

    /// The build's problems about the selection, as warning boxes.
    fn problems_for(&self, ui: &mut egui::Ui, s: Sel) {
        let mine: Vec<String> = self.problems.iter().filter(|p| self.problem_target(p) == Some(s)).map(|p| p.split_once(": ").map_or(p.as_str(), |x| x.1).to_string()).collect();
        if mine.is_empty() {
            return;
        }
        egui::Frame::new().inner_margin(Margin { left: 14, right: 14, top: 0, bottom: 12 }).show(ui, |ui| {
            for p in mine {
                widgets::callout(ui, &upper_first(&p), WARN);
            }
        });
    }

    fn multi_ui(&mut self, ui: &mut egui::Ui) {
        let ls = self.selected_loops();
        let names: Vec<String> = ls.iter().map(|&l| edit::loop_name(&self.doc, l)).collect();
        let delete = header(ui, Tile::Icon(Icon::Region), &format!("{} regions", ls.len()), style::FLOOR, Name::Fixed(names.join(", ")), true);
        let mut dz = 0.0;
        section(ui, "ins multi", "Together", None, true, |ui| {
            field(ui, "Raise", Some("All of them by the same (PgUp / PgDn: 20, Shift: 100; or drag them in 3D)"), |ui| {
                if widgets::button(ui, None, "−20", Some(52.0), false).clicked() {
                    dz = -20.0;
                }
                if widgets::button(ui, None, "+20", Some(52.0), false).clicked() {
                    dz = 20.0;
                }
            });
            widgets::hint(ui, "Shift-click a region (in the plan, in 3D or in the outliner) to add or take it out.");
        });
        if dz != 0.0 {
            self.raise(dz);
        }
        if delete {
            self.delete_selection();
        }
    }

    fn loop_ui(&mut self, ui: &mut egui::Ui, l: usize) {
        if l >= edit::loop_count(&self.doc) {
            return;
        }
        let styles = self.theme.style_names();
        let node = match self.sel {
            Sel::Node(r) => Some(r),
            _ => None,
        };
        if l == 0 {
            header(ui, Tile::Icon(Icon::Outline), "Outline", style::FLOOR, Name::Fixed("The whole level".into()), false);
            self.problems_for(ui, Sel::Loop(0));
            if let Some(r) = node {
                self.node_section(ui, r);
            }
            if let Sel::Edge(..) = self.sel {
                self.beyond_section(ui);
            }
            let o = &mut self.doc.outline;
            section(ui, "ins ground", "Ground", None, true, |ui| {
                field(ui, "Height", Some("The ground outside every region"), |ui| ui.add(egui::DragValue::new(&mut o.z).speed(1.0)));
                noise_ui(ui, &mut o.noise);
            });
            let n = o.nodes.len();
            section(ui, "ins shape", "Shape", None, true, |ui| {
                chips(ui, &[(Some(n.to_string()), "nodes"), (None, "double-click its edge to add one")]);
            });
            return;
        }
        let water = self.doc.regions[l - 1].kind == "water";
        let beside = self.beside_z(l);
        let r = &mut self.doc.regions[l - 1];
        let delete = header(
            ui,
            Tile::Icon(if water { Icon::Water } else { Icon::Region }),
            if water { "Region · water" } else { "Region · floor" },
            if water { style::WATER } else { style::FLOOR },
            Name::Edit(&mut r.name),
            true,
        );
        self.problems_for(ui, Sel::Loop(l));
        if let Some(n) = node {
            self.node_section(ui, n);
        }
        if let Sel::Edge(..) = self.sel {
            self.edge_section(ui, l);
        }
        let r = &mut self.doc.regions[l - 1];
        section(ui, "ins ground", "Ground", None, true, |ui| {
            field(ui, "Kind", Some("Floor, a pond (its height is the bed), or a pit: a drop into the void, its height how deep its walls go"), |ui| {
                if widgets::segmented(ui, &mut r.kind, &[("floor".to_string(), "Floor"), ("water".to_string(), "Water"), ("pit".to_string(), "Pit")]) && r.kind == "water" && r.surface.is_none() {
                    r.surface = Some(r.z + 60.0);
                }
            });
            let mut dz = 0.0;
            field(ui, if water { "Bed" } else { "Height" }, Some("PgUp / PgDn raise and sink it 20 (Shift: 100), or drag it up and down in 3D"), |ui| {
                ui.add(egui::DragValue::new(&mut r.z).speed(1.0));
                if widgets::button(ui, None, "−20", Some(44.0), false).clicked() {
                    dz = -20.0;
                }
                if widgets::button(ui, None, "+20", Some(44.0), false).clicked() {
                    dz = 20.0;
                }
            });
            r.z += dz;
            if water {
                field(ui, "Surface", Some("The water's height"), |ui| {
                    let mut s = r.surface.unwrap_or(r.z + 60.0);
                    if ui.add(egui::DragValue::new(&mut s).speed(1.0)).changed() {
                        r.surface = Some(s);
                    }
                });
            }
        });
        section(ui, "ins look", "Look", None, true, |ui| {
            field(ui, "Edge", Some("The wall style of this region's own walls (where it's the higher side)"), |ui| style_combo(ui, "edge", &mut r.edge, &styles, "theme rules"));
            noise_ui(ui, &mut r.noise);
        });
        let own = r.profiles.iter().filter(|p| p.is_some()).count();
        let drop = (r.z - beside).abs();
        section(ui, "ins edges", "Edges", (own > 0).then(|| format!("{own} own")), true, |ui| {
            profile_ui(ui, "region", &mut r.profile, None, drop, &styles, true);
            widgets::hint(
                ui,
                if water {
                    "How the bed meets the shore: a slope shelves up to it. Click an edge in the plan to give it a profile of its own."
                } else {
                    "How its edges meet the floor beside them, built inward from the edge (a sunken region's go up). Click an edge in the plan to give it a profile of its own."
                },
            );
        });
        let n = r.nodes.len();
        section(ui, "ins shape", "Shape", None, true, |ui| {
            chips(ui, &[(Some(n.to_string()), "nodes"), (None, "S: sharp corner"), (None, "Shift-click: select several")]);
        });
        if delete {
            self.sel = Sel::Loop(l);
            self.delete_selection();
        }
    }

    /// What lies beyond the selected outline edges (all of them at once): the forest, or ground
    /// climbing to a crest (`beyond.rs`).
    fn beyond_section(&mut self, ui: &mut egui::Ui) {
        let ks = self.selected_edges();
        let Some(&k0) = ks.first() else { return };
        let mut b = edit::beyond_of(&self.doc, k0).cloned();
        let mixed = ks.iter().any(|&k| edit::beyond_of(&self.doc, k).cloned() != b);
        let n = self.doc.outline.nodes.len();
        let title = if ks.len() == 1 { format!("Beyond edge {} → {}", k0, (k0 + 1) % n) } else { format!("Beyond {} edges", ks.len()) };
        let styles = self.theme.style_names();
        let ground = self.doc.outline.z;
        let mut changed = false;
        section(ui, "ins beyond", &title, None, true, |ui| {
            if mixed {
                widgets::hint(ui, "They differ: a choice here sets them all.");
            }
            let mut kind = if b.is_some() { "ground" } else { "forest" }.to_string();
            field(ui, "Beyond", Some("Forest: a cliff, a bank and trees, the edge of the world as it's always been. Ground: ground climbing from the edge to a crest, built outward, and nothing past it; its ends slope down into the forest beside it"), |ui| {
                if widgets::segmented(ui, &mut kind, &[("forest".to_string(), "Forest"), ("ground".to_string(), "Ground")]) {
                    b = (kind == "ground").then(|| stack_looks()[0].beyond(ground));
                    changed = true;
                }
            });
            if let Some(be) = b.as_mut() {
                // one of Kakariko's edges, then its height; the rest only when asked for
                let now = be.clone();
                let custom = match looks_ui(ui, |lk| lk.is(&now), (be.top() - ground).abs()) {
                    Pick::Chose(lk) => {
                        *be = lk.beyond(ground);
                        changed = true;
                        false
                    }
                    Pick::Same(custom) => custom,
                };
                field(ui, "Height", Some("How high it reaches above the level's floor: the crest, or a skyline wall's top, where the level ends at the sky"), |ui| {
                    let mut h = be.top() - ground;
                    if ui.add(egui::DragValue::new(&mut h).speed(1.0)).changed() {
                        match be.skyline.as_mut() {
                            // the wall takes the change; the ground in front stays as its parts say
                            Some(sk) => sk.height = (ground + h - be.z).max(0.0),
                            None => be.z = ground + h,
                        }
                        changed = true;
                    }
                });
                if widgets::disclosure(ui, "beyond fine", "Fine-tune", custom) {
                    field(ui, "Roughness", Some("How much its height above the walls at its foot rises and falls, in lumps about 900 across (a skyline wall's too): 0 is smooth, a mountain about 0.4"), |ui| {
                        changed |= ui.add(egui::Slider::new(&mut be.rough, 0.0..=1.0).show_value(true)).changed();
                    });
                    let mut sky = be.skyline.as_ref().map(|s| s.style.clone());
                    field(ui, "Skyline wall", Some("A wall standing on the far edge, its style stretched once over its height, rising above the ground there (Kakariko's mossy wall); Height is then its top"), |ui| {
                        if style_combo(ui, "skyline style", &mut sky, &styles, "none") {
                            let top = be.top();
                            be.skyline = sky.map(|style| Skyline { style, height: be.skyline.as_ref().map_or(400.0, |s| s.height) });
                            if be.skyline.is_none() {
                                be.z = top;
                            }
                            changed = true;
                        }
                    });
                    let mut p = Some(be.profile.clone());
                    if profile_ui(ui, "beyond", &mut p, None, (be.z - ground).abs(), &styles, false) {
                        be.profile = p.unwrap_or(Profile::Cliff);
                        changed = true;
                    }
                }
            }
            widgets::hint(ui, "Shift-click more outline edges to set them together. Del: back to the forest.");
        });
        if changed {
            edit::set_beyond(&mut self.doc, &ks, b);
        }
    }

    /// The selected edges' own profile (all of them at once).
    fn edge_section(&mut self, ui: &mut egui::Ui, l: usize) {
        let ks = self.selected_edges();
        let Some(&k0) = ks.first() else { return };
        let r = &self.doc.regions[l - 1];
        let mut p = edit::own_edge_profile(&self.doc, l, k0).cloned();
        let mixed = ks.iter().any(|&k| edit::own_edge_profile(&self.doc, l, k).cloned() != p);
        let region = profile_name(r.profile.as_ref());
        let n = r.nodes.len();
        let title = if ks.len() == 1 { format!("Edge {} → {}", k0, (k0 + 1) % n) } else { format!("{} edges", ks.len()) };
        let mut changed = false;
        let styles = self.theme.style_names();
        section(ui, "ins edge", &title, None, true, |ui| {
            if mixed {
                widgets::hint(ui, "They differ: a choice here sets them all.");
            }
            let drop = (r.z - self.beside_z(l)).abs();
            changed = profile_ui(ui, "edge", &mut p, Some(region), drop, &styles, true);
            widgets::hint(ui, "Shift-click more edges of this region to set them together. Del: back to the region's.");
        });
        if changed {
            edit::set_edge_profile(&mut self.doc, l, &ks, p);
        }
    }

    fn path_ui(&mut self, ui: &mut egui::Ui, k: usize) {
        if k >= self.doc.paths.len() {
            return;
        }
        let styles = self.theme.style_names();
        let node = match self.sel {
            Sel::Node(r) => Some(r),
            _ => None,
        };
        let floating = self.doc.paths[k].mode == "floating";
        let p = &mut self.doc.paths[k];
        let delete = header(ui, Tile::Icon(Icon::Path), if floating { "Path · bridge" } else { "Path · ramp" }, style::PATH, Name::Edit(&mut p.name), true);
        self.problems_for(ui, Sel::Path(k));
        if let Some(n) = node {
            self.node_section(ui, n);
        }
        let p = &mut self.doc.paths[k];
        section(ui, "ins pshape", "Shape", None, true, |ui| {
            field(ui, "Width", None, |ui| ui.add(egui::DragValue::new(&mut p.width).speed(1.0).range(20.0..=2000.0)));
            let segs = p.nodes.len().saturating_sub(1);
            let mut modes: Vec<String> = (0..segs).map(|i| p.modes.get(i).cloned().unwrap_or(p.mode.clone())).collect();
            field(ui, "Mode", Some("Attached: an embankment down to the ground. Floating: a bridge, open underneath. Each segment can differ."), |ui| {
                if widgets::segmented(ui, &mut p.mode, &[("attached".to_string(), "Attached"), ("floating".to_string(), "Floating")]) {
                    p.modes.clear();
                    modes = vec![p.mode.clone(); segs];
                }
            });
            if segs > 1 {
                let mut changed = false;
                for (i, m) in modes.iter_mut().enumerate() {
                    field(ui, &format!("  {} to {}", i, i + 1), None, |ui| {
                        changed |= widgets::segmented(ui, m, &[("attached".to_string(), "Attached"), ("floating".to_string(), "Floating")]);
                    });
                }
                if changed {
                    p.modes = if modes.iter().all(|m| *m == p.mode) { vec![] } else { modes };
                }
            }
        });
        section(ui, "ins plook", "Look", None, true, |ui| {
            field(
                ui,
                "Surface",
                Some("Ground, or stairs as Kakariko's are: a ramp with steps drawn on it, its sides the stairs' profile. Kakariko's are 1 in 2 (26.6°): a 160 rise over 320"),
                |ui| widgets::segmented(ui, &mut p.look, &[(None, "Ground"), (Some("steps".to_string()), "Steps")]),
            );
            field(ui, "Sides", Some("The wall style of its sides (stairs: the stairs' own unless you set one)"), |ui| style_combo(ui, "pedge", &mut p.edge, &styles, "theme's"));
            field(ui, "Bridge shape", Some("Under its floating segments"), |ui| widgets::segmented(ui, &mut p.shape, &[(None, "Theme's"), (Some("rock".to_string()), "Rock arch"), (Some("slab".to_string()), "Slab")]));
        });
        let n = p.nodes.len();
        section(ui, "ins pnodes", "Nodes", None, true, |ui| {
            chips(ui, &[(Some(n.to_string()), "nodes"), (None, "click one to set its own height or width")]);
            widgets::hint(ui, "The side profile below shows its slope: drag a node there to set its height.");
        });
        if delete {
            self.sel = Sel::Path(k);
            self.delete_selection();
        }
    }

    fn line_ui(&mut self, ui: &mut egui::Ui, k: usize) {
        if k >= self.doc.lines.len() {
            return;
        }
        let node = match self.sel {
            Sel::Node(r) => Some(r),
            _ => None,
        };
        let kind = self.doc.lines[k].kind.clone();
        let theme_w = match kind.as_str() {
            "dirt" => self.theme.dirt.as_ref().map(|d| d.width),
            "bridge" => self.theme.hanging.as_ref().map(|h| h.width),
            "tunnel" => self.theme.tunnel.as_ref().map(|t| t.width),
            _ => None,
        };
        let tunnel_h = self.theme.tunnel.as_ref().map_or(200.0, |t| t.height);
        let (icon, what) = match kind.as_str() {
            "dirt" => (Icon::Dirt, "Dirt path"),
            "fence" => (Icon::Fence, "Fence · rails"),
            "lattice" => (Icon::Fence, "Fence · lattice"),
            "bridge" => (Icon::Bridge, "Rope bridge"),
            "hedge" => (Icon::Hedge, "Hedge"),
            "tunnel" => (Icon::Tunnel, "Tunnel"),
            "rock" => (Icon::Rock, "Rock"),
            "arch" => (Icon::Arch, "Arch"),
            _ => (Icon::Dirt, "Line"),
        };
        let sag = self.theme.hanging.as_ref().map_or(0.0, |h| h.sag * 100.0);
        let hedge_h = self.theme.hedge.as_ref().map_or(28.0, |h| h.height);
        let l = &mut self.doc.lines[k];
        let delete = header(ui, Tile::Icon(icon), what, line_colour(&kind), Name::Edit(&mut l.name), true);
        self.problems_for(ui, Sel::Line(k));
        if let Some(n) = node {
            self.node_section(ui, n);
        }
        if kind == "rock" || kind == "arch" {
            self.rock_sections(ui, k);
            let n = self.doc.lines[k].nodes.len();
            section(ui, "ins lnodes", "Nodes", None, true, |ui| {
                chips(ui, &[(Some(n.to_string()), "nodes"), (None, "double-click it to add one")]);
            });
            if delete {
                self.sel = Sel::Line(k);
                self.delete_selection();
            }
            return;
        }
        let l = &mut self.doc.lines[k];
        section(ui, "ins llook", "Look", None, true, |ui| {
            if l.kind == "fence" || l.kind == "lattice" {
                field(ui, "Style", Some("Rails: the training ground's, 40 tall, a post every 40. Lattice: the tall one by the crawlspace, 120."), |ui| {
                    widgets::segmented(ui, &mut l.kind, &[("fence".to_string(), "Rails · 40"), ("lattice".to_string(), "Lattice · 120")])
                });
                widgets::toggle(ui, &mut l.closed, "Closed (back to the first node)");
                widgets::hint(ui, "Straight between nodes, a post at every node, standing on the ground; collides from both sides.");
            }
            if let Some(tw) = theme_w {
                let range = if l.kind == "tunnel" { overworld::tunnels::WIDTH.0..=overworld::tunnels::WIDTH.1 } else { 20.0..=2000.0 };
                field(ui, "Width", Some("The theme's unless you set one"), |ui| widgets::theme_num(ui, &mut l.width, tw, 1.0, range));
            }
            match l.kind.as_str() {
                "dirt" => widgets::hint(ui, "Painted into the floor: the ground fades to dirt across its soft edge, and the middle has dirt footsteps."),
                "bridge" => {
                    field(ui, "Sag", Some("Set by the theme's hanging bridge"), |ui| {
                        widgets::chip(ui, "of each span", Some(&format!("{sag:.0}%")), false);
                        widgets::theme_tag(ui);
                    });
                    widgets::hint(ui, "Put its ends on the floors it joins: they land at the floors' edges. A deck too steep to walk is reported.");
                }
                "hedge" => {
                    field(ui, "Height", None, |ui| {
                        widgets::chip(ui, "above the ground", Some(&format!("{hedge_h:.0}")), false);
                        widgets::theme_tag(ui);
                    });
                    widgets::hint(ui, "Tall grass over its shape, straight between its nodes, with grass skirts round it. Link wades through it.");
                }
                "tunnel" => {
                    let (h0, h1) = overworld::tunnels::HEIGHT;
                    field(ui, "Height", Some("Its floor to its roof: the theme's unless you set one"), |ui| widgets::theme_num(ui, &mut l.height, tunnel_h, 1.0, h0..=h1));
                    widgets::hint(ui, "Its mouths go where it meets a wall taller than it is, walking in from each end: put the ends on the floors in front of the walls. In between it follows its nodes; a node can set the floor's height there.");
                }
                _ => {}
            }
        });
        if l.kind == "tunnel" {
            section(ui, "ins lrough", "Rough walls", None, true, |ui| rough_ui(ui, &mut l.noise));
        }
        let n = l.nodes.len();
        section(ui, "ins lnodes", "Nodes", None, true, |ui| {
            chips(ui, &[(Some(n.to_string()), "nodes"), (None, "double-click it to add one")]);
        });
        if delete {
            self.sel = Sel::Line(k);
            self.delete_selection();
        }
    }

    /// A rock's shape (its look, its contours in a side view, lumps, layers, style) or an arch's
    /// size, lumps and style.
    fn rock_sections(&mut self, ui: &mut egui::Ui, k: usize) {
        let styles = self.theme.style_names();
        let rk = self.theme.rocks.clone();
        let theme_style = rk.as_ref().map_or("cliff".to_string(), |r| r.style.clone());
        let lumps = rk.as_ref().map_or((14.0, 140.0), |r| (r.lumps, r.lump_scale));
        // the footprint's width as drawn, for a look's heights
        let foot: Vec<overworld::geom::P2> = self.shapes.lines.get(k).map(|c| c.iter().map(|x| x.0).collect()).unwrap_or_default();
        let size = if foot.len() > 3 { 2.0 * (overworld::geom::signed_area(&foot).abs() / std::f64::consts::PI).sqrt() } else { 400.0 };
        let l = &mut self.doc.lines[k];
        if l.kind == "rock" {
            let tall = l.contours.last().map_or(0.0, |c| c.z);
            section(ui, "ins rshape", "Shape", Some(format!("{tall:.0} tall")), true, |ui| {
                if let Some(look) = rock_cards(ui, |lk| lk.is(l)) {
                    edit::rock_shape(l, &look);
                }
                rock_profile(ui, &mut l.contours, 0.5 * size);
                widgets::hint(ui, "Drag a contour's handle up or down for its height, in or out for its width (Shift: finer). The footprint is the shape drawn in the plan; a contour wider than the one below it overhangs. Its top is flat, and Link can stand on it.");
                let custom = !rock_looks().iter().any(|lk| lk.is(l));
                if widgets::disclosure(ui, "ins rfine", "Fine-tune", custom) {
                    contours_ui(ui, &mut l.contours);
                }
            });
            section(ui, "ins rsurface", "Surface", None, true, |ui| {
                lumps_ui(ui, &mut l.noise, lumps);
                let mut on = l.layers.is_some();
                if widgets::toggle(ui, &mut on, "Layers").on_hover_text("Grooves round it every so often, as in banded rock").changed() {
                    l.layers = on.then(|| Layers { height: (0.3 * size).clamp(60.0, 300.0).round(), depth: 12.0 });
                }
                if let Some(ly) = &mut l.layers {
                    field(ui, "  Every", Some("How tall each layer is"), |ui| ui.add(egui::DragValue::new(&mut ly.height).speed(1.0).range(20.0..=2000.0)));
                    field(ui, "  Depth", Some("How far its grooves cut in"), |ui| ui.add(egui::DragValue::new(&mut ly.depth).speed(0.5).range(1.0..=200.0)));
                }
                field(ui, "Style", Some("Its sides' look: a wall style, this theme's or another's. Its top is the theme's ground"), |ui| {
                    style_combo(ui, "rock style", &mut l.style, &styles, &format!("theme's ({theme_style})"))
                });
            });
        } else {
            let (w, h, d) = rk.as_ref().map_or((200.0, 560.0, 110.0), |r| (r.arch.width, r.arch.height, r.arch.depth));
            // how many segments it gets with none of its own, as the builder counts them
            let auto = if foot.len() >= 2 {
                let feet = [edit::base_z(&self.doc, &self.shapes, foot[0]), edit::base_z(&self.doc, &self.shapes, foot[foot.len() - 1])];
                let (a_top, total) = overworld::rocks::arch_lengths(&foot, feet, self.doc.lines[k].height.unwrap_or(h));
                let (m0, m1) = overworld::rocks::arch_split(a_top, total, overworld::rocks::arch_step(&self.doc.settings), None);
                (m0 + m1) as f64
            } else {
                16.0
            };
            let l = &mut self.doc.lines[k];
            section(ui, "ins rarch", "Size", None, true, |ui| {
                field(ui, "Height", Some("From the ground at its feet to the top of its crown: the theme's unless you set one (PgUp/PgDn)"), |ui| widgets::theme_num(ui, &mut l.height, h, 1.0, 40.0..=5000.0));
                field(ui, "Width", Some("Across its top: the theme's unless you set one"), |ui| widgets::theme_num(ui, &mut l.width, w, 1.0, 20.0..=2000.0));
                field(ui, "Thickness", Some("Top to underside at its crown; it grows towards its feet. The theme's unless you set one"), |ui| widgets::theme_num(ui, &mut l.depth, d, 1.0, 10.0..=2000.0));
                let (s0, s1) = overworld::rocks::ARCH_SEGMENTS;
                field(ui, "Segments", Some("How many pieces it's built of along it: few for a faceted, low-poly arch, more for a smooth one. Blank: by the level's detail"), |ui| {
                    let mut n = l.segments.map(f64::from);
                    if own_num(ui, &mut n, auto, 0.2, s0 as f64..=s1 as f64, "auto") {
                        l.segments = n.map(|x| x.round() as u32);
                    }
                });
                widgets::hint(ui, "It stands on the ground at its first and last nodes, rising to its crown halfway; nodes between bend it. Its top is walkable where it isn't steep.");
            });
            section(ui, "ins rsurface", "Surface", None, true, |ui| {
                lumps_ui(ui, &mut l.noise, lumps);
                field(ui, "Style", Some("Its rock's look: a wall style, this theme's or another's"), |ui| style_combo(ui, "arch style", &mut l.style, &styles, &format!("theme's ({theme_style})")));
            });
        }
    }

    fn node_section(&mut self, ui: &mut egui::Ui, r: NodeRef) {
        let g = edit::group(&self.doc, r);
        let mut p = edit::node_pos(&self.doc, r);
        let mut delete = false;
        section(ui, "ins node", "Node", None, true, |ui| {
            field(ui, "Position", None, |ui| {
                let a = ui.add(egui::DragValue::new(&mut p[0]).speed(2.0).prefix("x "));
                let b = ui.add(egui::DragValue::new(&mut p[1]).speed(2.0).prefix("y "));
                if a.changed() || b.changed() {
                    edit::move_group(&mut self.doc, &g, p);
                }
            });
            match r {
                NodeRef::Loop(..) => {
                    let mut sharp = edit::is_sharp(&self.doc, &g);
                    if widgets::toggle(ui, &mut sharp, "Sharp corner (S)").changed() {
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
                        widgets::hint(ui, format!("Shared by {}", names.join(", ")));
                    }
                }
                NodeRef::Path(k, i) => {
                    let floor = self.scene.as_ref().and_then(|s| s.floor_z(p)).unwrap_or(edit::base_z(&self.doc, &self.shapes, p)).round();
                    let width = self.doc.paths[k].width;
                    let n = &self.doc.paths[k].nodes[i];
                    let mut z = n.get(2).copied().flatten();
                    let mut w = n.get(3).copied().flatten();
                    let mut changed = false;
                    field(ui, "Height", Some("Blank: an end takes the floor's height, a middle node is interpolated"), |ui| {
                        changed |= own_num(ui, &mut z, floor, 1.0, -100000.0..=100000.0, "auto");
                    });
                    field(ui, "Width", Some("Blank: the path's width"), |ui| {
                        changed |= own_num(ui, &mut w, width, 1.0, 20.0..=2000.0, "path's");
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
                NodeRef::Line(k, i) if self.doc.lines[k].kind == "tunnel" && i > 0 && i + 1 < self.doc.lines[k].nodes.len() => {
                    // the floor there as built (its middle line's nearest point), else the ground's
                    let built = self.level.as_ref().and_then(|lv| lv.tunnels.iter().find(|t| t.0 == k)).and_then(|t| {
                        t.1.iter().min_by(|a, b| (a[0] - p[0]).hypot(a[1] - p[1]).total_cmp(&(b[0] - p[0]).hypot(b[1] - p[1]))).map(|q| q[2])
                    });
                    let auto = built.unwrap_or(self.doc.outline.z).round();
                    let n = &self.doc.lines[k].nodes[i];
                    let mut z = n.get(2).copied();
                    let mut changed = false;
                    field(ui, "Floor height", Some("Blank: on a straight slope between its mouths' floors (and other nodes' heights)"), |ui| {
                        changed |= own_num(ui, &mut z, auto, 1.0, -100000.0..=100000.0, "auto");
                    });
                    if changed {
                        let n = &mut self.doc.lines[k].nodes[i];
                        n.truncate(2);
                        if let Some(z) = z {
                            n.push(z);
                        }
                    }
                }
                NodeRef::Line(..) => {}
            }
            delete = widgets::button(ui, Some(Icon::Trash), "Delete node", Some(ui.available_width()), true).on_hover_text("Del").clicked();
        });
        if delete {
            self.sel = Sel::Node(r);
            self.delete_selection();
        }
    }

    fn prop_ui(&mut self, ui: &mut egui::Ui, i: usize) {
        if i >= self.doc.props.len() {
            return;
        }
        let built_z = self.prop_z(i);
        let fitted = self.on_wall(i);
        let placed = self.level.as_ref().and_then(|l| l.props.iter().find(|pl| pl.index == i).cloned());
        let piece = self.kit.as_ref().and_then(|k| k.get(&self.doc.props[i].piece)).cloned();
        // a wall piece's flat face where it is (asked at its narrowest, which fits wherever any does)
        let room = piece.as_ref().filter(|p| p.kind == "wall").and_then(|p| {
            let probe = Prop { scale: [p.scale.min[0], 1.0, 1.0], ..self.doc.props[i].clone() };
            self.ghost(&probe)?.ok().map(|pv| pv.room)
        });
        let thumb = piece.as_ref().and_then(|p| self.thumbs.get(ui.ctx(), p, self.lib.as_deref()));
        let kind = piece.as_ref().map_or("prop", |p| match p.kind.as_str() {
            "house" => "Prop · house",
            "tower" => "Prop · stump",
            "stone" => "Prop · stepping stone",
            "hedge" => "Prop · hedge",
            "opening" => "Prop · opening",
            "wall" => "Prop · on a wall",
            _ => "Prop",
        });
        let label = piece.as_ref().map_or(self.doc.props[i].piece.clone(), |p| p.label.clone());
        let delete = header(ui, Tile::Thumb(thumb), kind, PROP_COLOUR, Name::Fixed(label), true);
        if piece.is_none() {
            egui::Frame::new().inner_margin(Margin { left: 14, right: 14, top: 0, bottom: 12 }).show(ui, |ui| widgets::callout(ui, "This piece isn't in the kit.", BAD));
        }
        self.problems_for(ui, Sel::Prop(i));
        let prop = &mut self.doc.props[i];
        section(ui, "ins place", "Placement", None, true, |ui| {
            field(ui, "Position", None, |ui| {
                ui.add(egui::DragValue::new(&mut prop.at[0]).speed(2.0).prefix("x "));
                ui.add(egui::DragValue::new(&mut prop.at[1]).speed(2.0).prefix("y "));
            });
            if fitted {
                field(ui, "Fitted", Some("It fits itself to the wall nearest where you put it: on its face, facing out over the floor in front, slid along clear of corners. Drag it along the walls in 3D to move it."), |ui| {
                    let t = match &placed {
                        Some(pl) => format!("at {:.0}, {:.0}, facing {:.0}°", pl.origin[0], pl.origin[1], pl.yaw),
                        None => "not yet (see above)".into(),
                    };
                    ui.add(egui::Label::new(RichText::new(t).size(12.5).color(if placed.is_some() { TEXT } else { WARN })).wrap());
                });
                if let Some(p) = piece.as_ref().filter(|p| p.kind == "wall") {
                    let base = (p.bounds[1][0] - p.bounds[0][0]).max(1.0);
                    let lo = base * p.scale.min[0];
                    let hi = room.map_or(base * p.scale.max[0], |r| (r - 2.0 * overworld::openings::WALL_PIECE_MARGIN).min(base * p.scale.max[0])).max(lo);
                    let mut w = prop.scale[0].clamp(p.scale.min[0], p.scale.max[0]) * base;
                    field(ui, "Width", Some("Across the wall: as wide as the flat face it's on allows"), |ui| {
                        if ui.add(egui::DragValue::new(&mut w).speed(1.0).range(lo..=hi)).changed() {
                            prop.scale[0] = (w / base * 1000.0).round() / 1000.0;
                        }
                        ui.label(RichText::new(room.map_or("no flat face here".to_string(), |r| format!("face {r:.0}, up to {hi:.0}"))).size(11.5).color(FAINT));
                    });
                    let tall = p.bounds[1][2] - p.bounds[0][2];
                    field(ui, "Height", Some("Always from the floor to the wall's top"), |ui| {
                        ui.label(RichText::new(placed.as_ref().map_or("floor to the wall's top".to_string(), |pl| format!("{:.0}, floor to top", pl.scale[2] * tall))).size(12.5).color(MUTED));
                    });
                }
            } else {
                field(ui, "Height", Some("On the ground: its base (a house's doorway, a stone's top) stands on the floor there. PgUp / PgDn raise and sink it."), |ui| {
                    let mut ground = prop.z.is_none();
                    if widgets::toggle(ui, &mut ground, "on the ground").changed() {
                        prop.z = if ground { None } else { Some(built_z.unwrap_or(0.0).round()) };
                    }
                    match &mut prop.z {
                        Some(z) => {
                            ui.add(egui::DragValue::new(z).speed(1.0));
                        }
                        None => {
                            // the floor's height there, as built
                            let mut z = built_z.unwrap_or(0.0).round();
                            ui.add_enabled(false, egui::DragValue::new(&mut z));
                        }
                    }
                });
                field(ui, "Turn", Some("Degrees counter-clockwise from north. Drag the dial (15° steps; Shift: 1°), or Q / E."), |ui| {
                    if dial(ui, &mut prop.yaw) {
                        self.place_yaw = prop.yaw;
                    }
                    if ui.add(egui::DragValue::new(&mut prop.yaw).speed(1.0).range(-180.0..=180.0).suffix("°")).changed() {
                        self.place_yaw = prop.yaw;
                    }
                });
                if let Some(p) = &piece {
                    let default = overworld::props::levels_by_default(&p.kind);
                    field(ui, "Level", Some("Bumps on the floor flatten under its base and ease back around it, so it stands level. Only while it's on the ground."), |ui| {
                        let mut on = prop.level.unwrap_or(default);
                        ui.add_enabled_ui(prop.z.is_none(), |ui| {
                            if widgets::toggle(ui, &mut on, "level the ground").changed() {
                                prop.level = (on != default).then_some(on);
                            }
                        });
                    });
                }
            }
            if let Some(p) = piece.as_ref().filter(|_| !fitted) {
                let lim = p.scale.clone();
                field(ui, "Scale", Some("Its corner handle in the plan scales it too"), |ui| {
                    if lim.locked() {
                        widgets::chip(ui, "fixed: Link must fit", None, false);
                    } else if lim.uniform {
                        let mut f = prop.scale[0];
                        if ui.add(egui::DragValue::new(&mut f).speed(0.01).range(lim.min[0]..=lim.max[0])).changed() {
                            prop.scale = [f; 3];
                        }
                        ui.label(RichText::new(format!("{}–{}", lim.min[0], lim.max[0])).size(11.5).color(FAINT));
                    } else {
                        for (k, name) in ["x", "y", "z"].iter().enumerate() {
                            ui.add_enabled(lim.min[k] < lim.max[k], egui::DragValue::new(&mut prop.scale[k]).speed(0.01).range(lim.min[k]..=lim.max[k]).prefix(format!("{name} ")));
                        }
                    }
                });
            }
        });
        prop.at = prop.at.map(|x| (x * 100.0).round() / 100.0);
        let mut dup = false;
        if let Some(p) = &piece {
            section(ui, "ins piece", "Piece", None, true, |ui| {
                let size = [p.bounds[1][0] - p.bounds[0][0], p.bounds[1][1] - p.bounds[0][1], p.bounds[1][2] - p.bounds[0][2]];
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(5.0, 5.0);
                    widgets::chip(ui, "", Some(&format!("{:.0}×{:.0}×{:.0}", size[0], size[1], size[2])), false);
                    widgets::chip(ui, "triangles", Some(&p.tris.len().to_string()), false);
                    widgets::chip(ui, "collision vertices", Some(&p.collision_vertices().to_string()), false);
                    for f in &p.functions {
                        widgets::chip(ui, f, None, true).on_hover_text("Link can use it for this");
                    }
                });
                if let Some(d) = &p.door {
                    widgets::hint(ui, format!("Its door led to {} (exit {}). Scenery until levels load through Play_Init.", d.entrance, d.exit));
                }
                if let Some(o) = &p.opening {
                    widgets::hint(ui, format!("Opening {:.0} wide, {:.0} high, {:.0} deep (down to {:.0}).", o.width, o.height, o.depth, o.min_depth));
                }
                if !p.about.is_empty() {
                    widgets::hint(ui, p.about.clone());
                }
                dup = widgets::button(ui, Some(Icon::Duplicate), "Duplicate  (Ctrl+D)", Some(ui.available_width()), false).clicked();
            });
        }
        if dup {
            self.duplicate_prop();
        }
        if delete {
            self.sel = Sel::Prop(i);
            self.delete_selection();
        }
    }
}

/// Nothing selected: what you can click, and the main keys.
fn empty(ui: &mut egui::Ui) {
    egui::Frame::new().inner_margin(Margin { left: 18, right: 18, top: 20, bottom: 16 }).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 8.0;
        let (r, _) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::hover());
        ui.painter().rect(r, 10.0, PANEL2, Stroke::new(1.0, LINE), StrokeKind::Inside);
        icons::paint(ui.painter(), r.shrink(10.0), Icon::Select, MUTED, 1.8);
        ui.label(RichText::new("Nothing selected").font(style::semibold(14.5)).color(TEXT));
        ui.add(egui::Label::new(RichText::new("Click a region, path, line or prop in the plan or in 3D, or pick one in the list below.").size(12.5).color(MUTED)).wrap());
        for (k, what) in [("V", "the select tool"), ("Shift", "click to select several regions"), ("F", "fit the level in view"), ("?", "every key")] {
            ui.horizontal(|ui| {
                widgets::kbd(ui, k);
                ui.label(RichText::new(what).size(12.5).color(MUTED));
            });
        }
    });
}

/// A floor's bumps: a switch, and their settings when on.
fn noise_ui(ui: &mut egui::Ui, noise: &mut Option<overworld::doc::Noise>) {
    let mut on = noise.is_some();
    if widgets::toggle(ui, &mut on, "Bumps").on_hover_text("Smooth noise on the floor, fading out towards its edges").changed() {
        *noise = on.then(overworld::doc::Noise::default);
    }
    if let Some(n) = noise {
        field(ui, "  Height", Some("Up to this far up or down"), |ui| ui.add(egui::DragValue::new(&mut n.amplitude).speed(0.5).range(0.0..=500.0)));
        field(ui, "  Size", Some("About how far apart the bumps are"), |ui| ui.add(egui::DragValue::new(&mut n.scale).speed(5.0).range(50.0..=5000.0)));
        field(ui, "  Edge fade", Some("Bumps fade out over this distance from the floor's edges, which keep its height"), |ui| ui.add(egui::DragValue::new(&mut n.edge).speed(2.0).range(1.0..=3000.0)));
        field(ui, "  Seed", Some("Another pattern"), |ui| ui.add(egui::DragValue::new(&mut n.seed)));
    }
}

/// A tunnel's rough walls: a switch, and their settings when on.
fn rough_ui(ui: &mut egui::Ui, noise: &mut Option<overworld::doc::Noise>) {
    let mut on = noise.is_some();
    if widgets::toggle(ui, &mut on, "Rough").on_hover_text("Its walls and roof push in and out, it wanders a little from side to side and its floor rolls, like a cave").changed() {
        *noise = on.then(edit::rough_walls);
    }
    if let Some(n) = noise {
        field(ui, "  How much", Some("Walls and roof in or out by up to this (at most a quarter of its width); it wanders half as much, its floor rolls a third"), |ui| ui.add(egui::DragValue::new(&mut n.amplitude).speed(0.5).range(0.0..=200.0)));
        field(ui, "  Size", Some("About how far apart the lumps are"), |ui| ui.add(egui::DragValue::new(&mut n.scale).speed(5.0).range(50.0..=5000.0)));
        field(ui, "  Smooth mouths", Some("Fades in over this far from each mouth, so the mouths stay clean arches (0: rough right up to the wall)"), |ui| ui.add(egui::DragValue::new(&mut n.edge).speed(2.0).range(0.0..=3000.0)));
        field(ui, "  Seed", Some("Another pattern"), |ui| ui.add(egui::DragValue::new(&mut n.seed)));
    }
}

/// A rock's or an arch's lumps: the theme's (`theme`: how much, how big) unless it has its own.
fn lumps_ui(ui: &mut egui::Ui, noise: &mut Option<overworld::doc::Noise>, theme: (f64, f64)) {
    let mut own = noise.is_some();
    let tip = format!("Its faces push in and out. Off: the theme's ({:.0}, about {:.0} across)", theme.0, theme.1);
    if widgets::toggle(ui, &mut own, "Own lumps").on_hover_text(tip).changed() {
        *noise = own.then_some(overworld::doc::Noise { amplitude: theme.0, scale: theme.1, edge: 0.0, seed: 0 });
    }
    if let Some(n) = noise {
        field(ui, "  How much", Some("In or out by up to this (less where it's thin)"), |ui| ui.add(egui::DragValue::new(&mut n.amplitude).speed(0.5).range(0.0..=200.0)));
        field(ui, "  Size", Some("About how far apart the lumps are"), |ui| ui.add(egui::DragValue::new(&mut n.scale).speed(2.0).range(20.0..=3000.0)));
        field(ui, "  Seed", Some("Another pattern"), |ui| ui.add(egui::DragValue::new(&mut n.seed)));
    }
}

/// The rock looks as cards, three to a row, each with its silhouette; the ones `is` says it is
/// lit. Returns the one clicked.
pub(super) fn rock_cards(ui: &mut egui::Ui, is: impl Fn(&RockLook) -> bool) -> Option<RockLook> {
    let looks = rock_looks();
    let mut chose = None;
    let gap = 6.0;
    let w = ((ui.available_width() - 2.0 * gap) / 3.0).floor();
    for row in looks.chunks(3) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for lk in row {
                let on = is(lk);
                let cs = lk.contours(400.0);
                if widgets::card(ui, w, on, lk.name, "", |p, r| {
                    paint_rock(p, r, &cs, 200.0, None);
                })
                .on_hover_text(lk.tip)
                .clicked()
                    && !on
                {
                    chose = Some(lk.name);
                }
            }
        });
        ui.add_space(gap);
    }
    chose.and_then(|n| rock_looks().into_iter().find(|lk| lk.name == n))
}

/// The extent of a rock's side view, its footprint `r` either side of its centre: (width, height)
/// in its units, with a margin.
fn rock_span(contours: &[Contour], r: f64) -> (f64, f64) {
    let (shape, _) = overworld::rocks::Shape::new(contours);
    let top = shape.top().max(1.0);
    let wide = (0..=32)
        .map(|i| {
            let z = top * i as f64 / 32.0;
            shape.scale(z) * r + shape.shift(z)[0].abs()
        })
        .fold(r, f64::max);
    (2.0 * wide * 1.15, top * 1.12)
}

/// A rock's side view in `rect`, seen from the south: the ground, and its silhouette from the
/// footprint (`r` either side of its centre) up through its contours, as the builder lofts it,
/// `span` (width, height: else its own) fitted in. Returns where each contour's handle goes (its
/// right edge) and the scale (pixels per unit).
fn paint_rock(p: &egui::Painter, rect: Rect, contours: &[Contour], r: f64, span: Option<(f64, f64)>) -> (Vec<Pos2>, f64) {
    p.rect_filled(rect, 6.0, Color32::from_rgb(0x9f, 0xc3, 0xe8));
    let (shape, _) = overworld::rocks::Shape::new(contours);
    let top = shape.top().max(1.0);
    let n = 48;
    let samples: Vec<(f64, f64, f64)> = (0..=n)
        .map(|i| {
            let z = top * i as f64 / n as f64;
            (z, shape.scale(z) * r, shape.shift(z)[0])
        })
        .collect();
    let (sw, sh) = span.unwrap_or_else(|| rock_span(contours, r));
    let inner = rect.shrink2(Vec2::new(8.0, 6.0));
    let k = (inner.width() as f64 / sw.max(1.0)).min((inner.height() as f64 - 4.0) / sh.max(1.0));
    let ground_y = inner.bottom() - 2.0;
    let cx = inner.center().x as f64;
    let at = |x: f64, z: f64| Pos2::new((cx + x * k) as f32, ground_y - (z * k) as f32);
    p.rect_filled(Rect::from_min_max(Pos2::new(rect.left(), ground_y), rect.right_bottom()), 0.0, Color32::from_rgb(0x4a, 0x3d, 0x2c));
    let rock = Color32::from_rgb(0xa8, 0x8f, 0x6c);
    // the silhouette in horizontal strips (each convex)
    for w in samples.windows(2) {
        let (a, b) = (w[0], w[1]);
        let quad = vec![at(a.2 - a.1, a.0), at(a.2 + a.1, a.0), at(b.2 + b.1, b.0), at(b.2 - b.1, b.0)];
        p.add(egui::Shape::convex_polygon(quad, rock, Stroke::new(0.6, rock)));
    }
    let edge = Stroke::new(1.4, Color32::from_rgb(0x5c, 0x4a, 0x34));
    p.add(egui::Shape::line(samples.iter().map(|s| at(s.2 - s.1, s.0)).collect(), edge));
    p.add(egui::Shape::line(samples.iter().map(|s| at(s.2 + s.1, s.0)).collect(), edge));
    let t = samples[n];
    p.line_segment([at(t.2 - t.1, top), at(t.2 + t.1, top)], Stroke::new(2.0, style::FLOOR));
    p.line_segment([Pos2::new(rect.left(), ground_y), Pos2::new(rect.right(), ground_y)], Stroke::new(1.5, style::FLOOR));
    (contours.iter().map(|c| at(c.shift[0] + c.scale * r, c.z)).collect(), k)
}

/// A rock's contours in a side view, its footprint `r` either side of its centre: drag a contour's
/// handle up and down for its height, in and out for its scale (Shift: finer). The view keeps its
/// scale during a drag, so the handle stays under the pointer.
fn rock_profile(ui: &mut egui::Ui, contours: &mut [Contour], r: f64) {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 170.0), Sense::click_and_drag());
    let p = ui.painter_at(rect);
    let id = ui.id().with("rock profile drag");
    let held: Option<(usize, f64, f64)> = ui.ctx().data(|d| d.get_temp(id));
    let span = held.map(|h| (h.1, h.2)).unwrap_or_else(|| rock_span(contours, r));
    let (handles, k) = paint_rock(&p, rect, contours, r, Some(span));
    let hover = resp.hover_pos();
    for (i, h) in handles.iter().enumerate() {
        let on = held.is_some_and(|x| x.0 == i) || hover.is_some_and(|q| q.distance(*h) < 14.0);
        p.circle(*h, if on { 6.0 } else { 5.0 }, if on { ACCENT } else { Color32::WHITE }, Stroke::new(1.2, Color32::BLACK));
        p.text(*h + Vec2::new(9.0, 0.0), egui::Align2::LEFT_CENTER, format!("{:.0}", contours[i].z), egui::FontId::proportional(10.5), Color32::BLACK);
    }
    if resp.drag_started() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let best = handles.iter().enumerate().map(|(i, h)| (i, h.distance(pos))).filter(|x| x.1 < 14.0).min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((i, _)) = best {
                ui.ctx().data_mut(|d| d.insert_temp(id, (i, span.0, span.1)));
            }
        }
    }
    if let (Some((i, _, _)), true) = (held, resp.dragged()) {
        let d = resp.drag_delta();
        let fine = if ui.input(|x| x.modifiers.shift) { 0.25 } else { 1.0 };
        let lo = if i == 0 { 5.0 } else { contours[i - 1].z + 5.0 };
        let hi = contours.get(i + 1).map_or(20000.0, |c| c.z - 5.0);
        let c = &mut contours[i];
        c.z = (c.z - d.y as f64 * fine / k).clamp(lo, hi);
        c.scale = (c.scale + d.x as f64 * fine / k / r.max(1.0)).clamp(0.02, 3.0);
    }
    if resp.drag_stopped() && held.is_some() {
        ui.ctx().data_mut(|d| d.remove::<(usize, f64, f64)>(id));
        for c in contours.iter_mut() {
            c.z = c.z.round();
            c.scale = (c.scale * 100.0).round() / 100.0;
        }
    }
}

/// A rock's contours as numbers, lowest first: each one's height, scale and shift; add, remove.
fn contours_ui(ui: &mut egui::Ui, contours: &mut Vec<Contour>) {
    let mut remove = None;
    let n = contours.len();
    for (i, c) in contours.iter_mut().enumerate() {
        ui.push_id(("contour", i), |ui| {
            ui.add_space(4.0);
            field(ui, &format!("  {}", i + 1), Some("Its height over the ground at the rock's foot"), |ui| {
                ui.add(egui::DragValue::new(&mut c.z).speed(1.0).range(1.0..=20000.0).prefix("height "));
            });
            field(ui, "    Scale", Some("Its size, of the footprint's: over 100% it's wider than the footprint"), |ui| {
                let mut pc = c.scale * 100.0;
                if ui.add(egui::DragValue::new(&mut pc).speed(0.5).range(2.0..=300.0).suffix("%")).changed() {
                    c.scale = pc / 100.0;
                }
            });
            field(ui, "    Shift", Some("How far it's moved from over the footprint's centre: a leaning rock"), |ui| {
                ui.add(egui::DragValue::new(&mut c.shift[0]).speed(1.0).prefix("x "));
                ui.add(egui::DragValue::new(&mut c.shift[1]).speed(1.0).prefix("y "));
            });
            if n > 1 {
                field(ui, "    ", None, |ui| {
                    if widgets::button(ui, None, "Remove", None, false).clicked() {
                        remove = Some(i);
                    }
                });
            }
        });
    }
    if let Some(i) = remove {
        contours.remove(i);
    }
    ui.add_space(4.0);
    if widgets::button(ui, None, "+ Contour", None, false).on_hover_text("A new top, a little higher and narrower").clicked() {
        let (z, s) = contours.last().map_or((300.0, 0.85), |c| (c.z + 80.0, c.scale * 0.8));
        contours.push(Contour::new(z.round(), (s * 100.0).round() / 100.0));
    }
    if contours.windows(2).any(|w| w[1].z <= w[0].z) {
        widgets::hint(ui, "Each contour must be higher than the one below it: the build leaves out any that aren't.");
    }
}

/// A value of the node's own, or (None) the default shown greyed with `what` beside it. Setting it
/// takes the default as the start; ↺ clears it.
fn own_num(ui: &mut egui::Ui, v: &mut Option<f64>, default: f64, speed: f64, range: std::ops::RangeInclusive<f64>, what: &str) -> bool {
    let mut changed = false;
    match v {
        Some(x) => {
            changed |= ui.add(egui::DragValue::new(x).speed(speed).range(range)).changed();
            if widgets::reset(ui).on_hover_text("Back to automatic").clicked() {
                *v = None;
                changed = true;
            }
        }
        None => {
            let mut x = default;
            let r = ui.scope(|ui| {
                ui.visuals_mut().override_text_color = Some(FAINT);
                ui.add(egui::DragValue::new(&mut x).speed(speed).range(range))
            });
            if r.inner.changed() {
                *v = Some(x);
                changed = true;
            }
            let g = ui.painter().layout_job(widgets::caps(what, 9.0, FAINT));
            let (rect, _) = ui.allocate_exact_size(g.size() + Vec2::new(8.0, 5.0), Sense::hover());
            ui.painter().rect_stroke(rect, 4.0, Stroke::new(1.0, LINE2), StrokeKind::Inside);
            ui.painter().galley(rect.center() - g.size() / 2.0, g, FAINT);
        }
    }
    changed
}

/// A compass dial for a prop's turn: drag it round (15° steps; Shift: 1°). Returns whether it
/// changed.
fn dial(ui: &mut egui::Ui, yaw: &mut f64) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(56.0), Sense::click_and_drag());
    let c = rect.center();
    let rad = 26.0;
    let mut changed = false;
    if let (true, Some(p)) = (resp.dragged() || resp.clicked(), resp.interact_pointer_pos()) {
        let d = p - c;
        if d.length() > 3.0 {
            let mut a = (-d.x as f64).atan2(-d.y as f64).to_degrees();
            a = if ui.input(|i| i.modifiers.shift) { a.round() } else { (a / 15.0).round() * 15.0 };
            if a <= -180.0 {
                a += 360.0;
            }
            if a != *yaw {
                *yaw = a;
                changed = true;
            }
        }
    }
    let p = ui.painter();
    p.circle(c, rad, FIELD, Stroke::new(1.0, if resp.hovered() { LINE2 } else { LINE }));
    // north gets a longer, lighter tick
    for t in [0.0f32, 90.0, 180.0, 270.0] {
        let a = t.to_radians();
        let v = Vec2::new(-a.sin(), -a.cos());
        let (len, col) = if t == 0.0 { (8.0, MUTED) } else { (4.0, FAINT) };
        p.line_segment([c + v * (rad - 1.0 - len), c + v * (rad - 1.0)], Stroke::new(1.5, col));
    }
    let a = (*yaw as f32).to_radians();
    let v = Vec2::new(-a.sin(), -a.cos());
    p.line_segment([c, c + v * (rad - 6.0)], Stroke::new(2.5, ACCENT));
    p.circle_filled(c + v * (rad - 6.0), 4.0, ACCENT);
    p.circle_filled(c, 2.5, ACCENT);
    changed
}

fn upper_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// A profile's name for people.
pub(super) fn profile_name(p: Option<&Profile>) -> &'static str {
    match p {
        None | Some(Profile::Cliff) => "Cliff",
        Some(Profile::Slope { .. }) => "Slope",
        Some(Profile::Terraces { .. }) => "Terraces",
        Some(Profile::Overhang { .. }) => "Overhang",
        Some(Profile::Ragged { .. }) => "Ragged",
        Some(Profile::Stack { .. }) => "Stack",
    }
}

/// What the look cards did (`looks_ui`).
enum Pick {
    /// One was chosen.
    Chose(StackLook),
    /// None: whether the profile is none of them (custom).
    Same(bool),
}

/// Kakariko's edges as cards, each with a cross-section of its parts (`drop` high), the ones `is`
/// says it is lit.
fn looks_ui(ui: &mut egui::Ui, is: impl Fn(&StackLook) -> bool, drop: f64) -> Pick {
    let looks = stack_looks();
    let on: Vec<bool> = looks.iter().map(is).collect();
    let mut chose = None;
    let gap = 6.0;
    let w = ((ui.available_width() - 2.0 * gap) / 3.0).floor();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for (lk, &on) in looks.iter().zip(&on) {
            let resp = widgets::card(ui, w, on, lk.name, "", |pt, r| paint_stack(pt, r, &lk.parts, drop.max(lk.crest), lk.skyline.map(|s| (lk.crest, s))));
            if resp.on_hover_text(lk.tip).clicked() && !on {
                chose = Some(looks.iter().position(|x| x.name == lk.name).unwrap());
            }
        }
    });
    let custom = !on.iter().any(|&x| x);
    if custom {
        widgets::hint(ui, "Its own mix of walls and slopes: see Fine-tune.");
    }
    match chose {
        Some(k) => Pick::Chose(looks.into_iter().nth(k).unwrap()),
        None => Pick::Same(custom),
    }
}

/// A stack's cross-section in `rect`: the floor at the foot on the left, its parts climbing to
/// the right, each drawn in its style's colour (a slope with none in the floor's).
///
/// With a skyline wall (`(top, style)`), the parts climb as far as they say and the wall stands
/// at the end, up to `top`.
fn paint_stack(p: &egui::Painter, rect: Rect, parts: &[Part], drop: f64, skyline: Option<(f64, &str)>) {
    p.rect_filled(rect, 6.0, Color32::from_rgb(0x9f, 0xc3, 0xe8));
    let fixed: f64 = parts.iter().filter_map(|q| q.rise).sum();
    let free = parts.iter().filter(|q| q.rise.is_none()).count();
    let share = if free > 0 { ((drop - fixed) / free as f64).max(0.0) } else { 0.0 };
    // (in, up, colour of the piece ending there)
    let mut pts: Vec<(f64, f64, Color32)> = vec![(0.0, 0.0, style::FLOOR)];
    let (mut d, mut h) = (0.0f64, 0.0f64);
    let sky = skyline.map(|(top, s)| Part::wall(Some((top - fixed).max(0.0)), Some(s)));
    for q in parts.iter().chain(sky.as_ref()) {
        let r = q.rise.unwrap_or(share).max(0.0);
        // a skyline wall stands a little way back from the slope's top
        if sky.as_ref().is_some_and(|s| std::ptr::eq(s, q)) {
            d += 0.15 * d.max(h);
            pts.push((d, h, style::FLOOR));
        }
        let col = match q.style.as_deref().map(|s| s.rsplit(':').next().unwrap_or(s)) {
            Some("brick" | "brick_top" | "stone") => Color32::from_rgb(0x9a, 0x9c, 0x98),
            Some("rock") => Color32::from_rgb(0xa8, 0x8f, 0x6c),
            Some("mountain") => Color32::from_rgb(0x7d, 0x8a, 0x48),
            Some(_) => Color32::from_rgb(0x9a, 0x74, 0x4c),
            None if q.kind == "wall" => Color32::from_rgb(0x9a, 0x74, 0x4c),
            None => style::FLOOR,
        };
        if q.kind == "wall" {
            h += r;
        } else {
            d += r / q.angle.clamp(1.0, 89.0).to_radians().tan();
            h += r;
        }
        pts.push((d, h, col));
    }
    let lead = 0.3 * d.max(h);
    let (x0, x1, y1) = (-lead, d + 0.2 * d.max(h), h.max(1.0));
    let inner = rect.shrink2(Vec2::new(6.0, 6.0));
    let s = (inner.width() as f64 / (x1 - x0)).min(inner.height() as f64 / y1);
    let at = |x: f64, y: f64| Pos2::new(inner.center().x + ((x - 0.5 * (x0 + x1)) * s) as f32, inner.bottom() - (y * s) as f32);
    let ground = Color32::from_rgb(0x4a, 0x3d, 0x2c);
    let mut outline = vec![at(x0, 0.0)];
    outline.extend(pts.iter().map(|&(x, y, _)| at(x, y)));
    outline.push(at(x1, h));
    // the ground under the line, in strips (each convex, overlapping a little so no seams show)
    for k in 0..outline.len() - 1 {
        let (a, b) = (outline[k], outline[k + 1]);
        if (b.x - a.x).abs() < 0.5 {
            continue;
        }
        let (a, b) = (a - Vec2::new(0.6, 0.0), b + Vec2::new(0.6, 0.0));
        p.add(egui::Shape::convex_polygon(vec![a, b, Pos2::new(b.x, rect.bottom()), Pos2::new(a.x, rect.bottom())], ground, Stroke::NONE));
    }
    p.line_segment([outline[0], outline[1]], Stroke::new(2.5, style::FLOOR));
    // slopes, then walls over them (thicker: they're short and upright)
    for wall in [false, true] {
        for k in 1..pts.len() {
            let (a, b) = (at(pts[k - 1].0, pts[k - 1].1), at(pts[k].0, pts[k].1));
            if ((b.x - a.x).abs() < 0.5) == wall {
                p.line_segment([a, b], Stroke::new(if wall { 3.5 } else { 2.5 }, pts[k].2));
            }
        }
    }
    p.line_segment([outline[outline.len() - 2], outline[outline.len() - 1]], Stroke::new(2.5, pts[pts.len() - 1].2));
}

/// A stack's parts, from the foot in: each a wall or a slope, its height (blank: a share of the
/// rest), a slope's angle, and its style. Returns whether they changed.
fn stack_ui(ui: &mut egui::Ui, parts: &mut Vec<Part>, drop: f64, styles: &[String]) -> bool {
    let mut changed = false;
    let fixed: f64 = parts.iter().filter_map(|p| p.rise).sum();
    let free = parts.iter().filter(|p| p.rise.is_none()).count();
    let share = if free > 0 { ((drop - fixed) / free as f64).max(0.0).round() } else { 0.0 };
    if fixed > drop + 0.5 {
        widgets::hint(ui, &format!("Its parts climb {fixed:.0}, more than the {drop:.0} drop: it's a cliff until they fit."));
    }
    let mut action: Option<(usize, i32)> = None;
    let n = parts.len();
    for (i, p) in parts.iter_mut().enumerate() {
        ui.push_id(("part", i), |ui| {
            ui.add_space(4.0);
            field(ui, &format!("  {}", i + 1), Some("A wall stands upright; a slope climbs at its angle"), |ui| {
                changed |= widgets::segmented(ui, &mut p.kind, &[("wall".to_string(), "Wall"), ("slope".to_string(), "Slope")]);
            });
            field(ui, "    Height", Some("How much of the drop it climbs. Blank: an even share of what the others leave"), |ui| {
                changed |= own_num(ui, &mut p.rise, share, 1.0, 0.0..=10000.0, "share");
            });
            if p.kind == "slope" {
                field(ui, "    Angle", Some("Link walks up to 35°; past 60° a slope with no style is drawn as the theme's cliff"), |ui| {
                    changed |= ui.add(egui::DragValue::new(&mut p.angle).speed(0.5).range(2.0..=88.0).suffix("°")).changed();
                    if p.angle > 35.0 {
                        ui.label(RichText::new("steep").size(11.0).color(WARN));
                    }
                });
            }
            let none = if p.kind == "slope" { "floor" } else { "theme rules" };
            field(ui, "    Style", Some("Its look: a wall style, this theme's or another's. A slope with one is drawn as that wall, along the edge and up the face"), |ui| {
                changed |= style_combo(ui, &format!("part style {i}"), &mut p.style, styles, none);
            });
            field(ui, "    ", None, |ui| {
                ui.horizontal(|ui| {
                    if i > 0 && widgets::button(ui, None, "↑", Some(30.0), false).on_hover_text("Nearer the foot").clicked() {
                        action = Some((i, -1));
                    }
                    if i + 1 < n && widgets::button(ui, None, "↓", Some(30.0), false).on_hover_text("Further in").clicked() {
                        action = Some((i, 1));
                    }
                    if n > 1 && widgets::button(ui, None, "Remove", None, false).clicked() {
                        action = Some((i, 0));
                    }
                });
            });
        });
    }
    match action {
        Some((i, 0)) => {
            parts.remove(i);
            changed = true;
        }
        Some((i, d)) => {
            parts.swap(i, (i as i32 + d) as usize);
            changed = true;
        }
        None => {}
    }
    field(ui, "  Add", None, |ui| {
        ui.horizontal(|ui| {
            if widgets::button(ui, None, "+ Wall", None, false).clicked() {
                parts.push(Part::wall(Some(160.0), None));
                changed = true;
            }
            if widgets::button(ui, None, "+ Slope", None, false).clicked() {
                parts.push(Part::slope(overworld::doc::slope_angle(), None, None));
                changed = true;
            }
        });
    });
    changed
}

/// A profile: cliff, slope, terraces, overhang or ragged rock, and their numbers. With `inherit`
/// (what the region's is), also "Region's" (None). Returns whether it changed.
///
/// `drop`: about how far the region stands above (or below) the floor beside it, for the terraces'
/// even step height. `looks`: a stack shows Kakariko's edges to pick from, its parts under
/// Fine-tune (else just its parts: the caller shows the looks).
fn profile_ui(ui: &mut egui::Ui, id: &str, p: &mut Option<Profile>, inherit: Option<&str>, drop: f64, styles: &[String], looks: bool) -> bool {
    let kind = |p: &Option<Profile>| match p {
        None if inherit.is_some() => "inherit",
        None | Some(Profile::Cliff) => "cliff",
        Some(Profile::Slope { .. }) => "slope",
        Some(Profile::Terraces { .. }) => "terraces",
        Some(Profile::Overhang { .. }) => "overhang",
        Some(Profile::Ragged { .. }) => "ragged",
        Some(Profile::Stack { .. }) => "stack",
    };
    let mut k = kind(p).to_string();
    let mut changed = false;
    let region = inherit.map(|r| format!("Region's ({r})"));
    ui.push_id(id, |ui| {
        field(ui, "Profile", Some("Cliff: a wall. Slope: a ramp of ground. Terraces: steps. Overhang: the top juts out over an undercut. Ragged: a cliff of lumpy rock. Stack: walls and slopes one above the other, each with its own look (Kakariko's edges)."), |ui| {
            let mut opts: Vec<(String, &str)> = vec![];
            if let Some(r) = &region {
                opts.push(("inherit".into(), r.as_str()));
            }
            opts.extend([
                ("cliff".to_string(), "Cliff"),
                ("slope".to_string(), "Slope"),
                ("terraces".to_string(), "Terraces"),
                ("overhang".to_string(), "Overhang"),
                ("ragged".to_string(), "Ragged"),
                ("stack".to_string(), "Stack"),
            ]);
            // too many for one row: rows of three (the region's on its own above, for an edge)
            ui.vertical(|ui| {
                let mut c = false;
                let first = if inherit.is_some() { 1 } else { 3 };
                let (a, rest) = opts.split_at(first);
                ui.horizontal(|ui| c |= widgets::segmented(ui, &mut k, a));
                for row in rest.chunks(3) {
                    ui.horizontal(|ui| c |= widgets::segmented(ui, &mut k, row));
                }
                c
            })
            .inner
        });
        if k != kind(p) {
            *p = match k.as_str() {
                "inherit" => None,
                "cliff" if inherit.is_none() => None,
                "cliff" => Some(Profile::Cliff),
                "slope" => Some(Profile::Slope { angle: overworld::doc::slope_angle(), round: 0.0 }),
                "overhang" => Some(Profile::Overhang { depth: overworld::doc::overhang_depth() }),
                "ragged" => Some(Profile::Ragged { amplitude: overworld::doc::ragged_amplitude(), scale: overworld::doc::ragged_scale(), seed: 0 }),
                "stack" => Some(stack_looks()[0].region_profile()),
                _ => Some(Profile::Terraces { steps: overworld::doc::terrace_steps(), rise: None, depth: overworld::doc::terrace_depth() }),
            };
            changed = true;
        }
        match p {
            Some(Profile::Slope { angle, round }) => {
                field(ui, "  Angle", Some("At its steepest, in degrees: Link walks up to 35"), |ui| {
                    changed |= ui.add(egui::DragValue::new(angle).speed(0.5).range(2.0..=80.0).suffix("°")).changed();
                    if *angle > 35.0 {
                        ui.label(RichText::new("steep").size(11.0).color(WARN));
                    }
                });
                field(ui, "  Rounding", Some("0: a straight slope with a sharp crest and foot. 1: an S curve, half as long again"), |ui| {
                    changed |= ui.add(egui::Slider::new(round, 0.0..=1.0).show_value(true)).changed();
                });
            }
            Some(Profile::Terraces { steps, rise, depth }) => {
                field(ui, "  Steps", Some("Risers from the floor beside it up to the top (the last onto its own floor)"), |ui| {
                    changed |= ui.add(egui::DragValue::new(steps).speed(0.1).range(1..=40)).changed();
                });
                let even = (drop / (*steps).max(1) as f64).round().max(1.0);
                field(ui, "  Step height", Some("Blank: the drop shared evenly. Set: the rest is a cliff at the edge"), |ui| {
                    changed |= own_num(ui, rise, even, 1.0, 1.0..=1000.0, "even");
                });
                field(ui, "  Depth", Some("How deep each tread is"), |ui| {
                    changed |= ui.add(egui::DragValue::new(depth).speed(1.0).range(10.0..=2000.0)).changed();
                });
            }
            Some(Profile::Overhang { depth }) => {
                field(ui, "  Depth", Some("How far the top juts out over the foot: the cliff is undercut this far, and the floor below runs in under it"), |ui| {
                    changed |= ui.add(egui::DragValue::new(depth).speed(1.0).range(5.0..=1000.0)).changed();
                });
            }
            Some(Profile::Stack { parts }) if looks => {
                let now = Profile::Stack { parts: parts.clone() };
                let custom = match looks_ui(ui, |lk| lk.region_profile() == now, drop) {
                    Pick::Chose(lk) => {
                        if let Profile::Stack { parts: ps } = lk.region_profile() {
                            *parts = ps;
                        }
                        changed = true;
                        false
                    }
                    Pick::Same(custom) => custom,
                };
                if widgets::disclosure(ui, &format!("{id} fine"), "Fine-tune", custom) {
                    changed |= stack_ui(ui, parts, drop, styles);
                }
            }
            Some(Profile::Stack { parts }) => {
                changed |= stack_ui(ui, parts, drop, styles);
            }
            Some(Profile::Ragged { amplitude, scale, seed }) => {
                field(ui, "  How much", Some("The rock pushes in and out by up to this; the top and foot stay on the edge"), |ui| {
                    changed |= ui.add(egui::DragValue::new(amplitude).speed(0.5).range(0.0..=300.0)).changed();
                });
                field(ui, "  Size", Some("About how far apart the lumps are"), |ui| {
                    changed |= ui.add(egui::DragValue::new(scale).speed(2.0).range(10.0..=3000.0)).changed();
                });
                field(ui, "  Seed", Some("Another pattern"), |ui| {
                    changed |= ui.add(egui::DragValue::new(seed)).changed();
                });
            }
            _ => {}
        }
    });
    changed
}
