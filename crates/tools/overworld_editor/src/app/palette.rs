//! The palette (the left panel): what the current tool adds, and how. The brush's modes and
//! falloff, the kit's pieces as thumbnails, and the kinds and styles new regions, paths and lines
//! start with.

use super::chrome::tool_info;
use super::icons::{self, Icon};
use super::style::{self, ACCENT, FAINT, FIELD, LINE, LINE2, MUTED, PANEL2, TEXT, WARN};
use super::widgets::{self, field, section};
use super::{brush_colour, App, Tool};
use eframe::egui::{self, text::LayoutJob, Align2, Color32, FontId, Margin, Pos2, Rect, RichText, Sense, Shape, Stroke, StrokeKind, TextFormat, Vec2};
use overworld::pieces::Piece;
use overworld::terrain::Mode;

/// The kit's kinds, as the palette's categories (hedges are drawn with the Hedge tool).
const CATS: [(&str, &str); 6] = [("all", "All"), ("house", "Houses"), ("tower", "Stumps"), ("stone", "Stones"), ("opening", "Openings"), ("wall", "On walls")];

impl App {
    pub(super) fn palette(&mut self, ui: &mut egui::Ui) {
        let info = tool_info(self.tool);
        egui::Frame::new().inner_margin(Margin { left: 14, right: 14, top: 14, bottom: 12 }).show(ui, |ui| {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(38.0), Sense::hover());
                ui.painter().rect_filled(r, 9.0, style::soft(ACCENT, 0.14));
                icons::paint(ui.painter(), r.shrink(8.5), info.icon, ACCENT, 1.8);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(info.name).font(style::semibold(15.0)).color(TEXT));
                        widgets::kbd(ui, info.key);
                    });
                    ui.add(egui::Label::new(RichText::new(info.about).size(12.0).color(MUTED)).wrap());
                });
            });
        });
        match self.tool {
            Tool::Select => self.select_palette(ui),
            Tool::Region => self.region_palette(ui),
            Tool::Path => self.path_palette(ui),
            Tool::Brush => self.brush_palette(ui),
            Tool::Prop => self.kit_palette(ui),
            Tool::Line(kind) => self.line_palette(ui, kind),
        }
    }

    fn select_palette(&mut self, ui: &mut egui::Ui) {
        section(ui, "pal pick", "Clicks pick", None, true, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(5.0, 5.0);
                let p = &mut self.pickable;
                for (on, name) in [(&mut p.regions, "Regions"), (&mut p.edges, "Edges"), (&mut p.paths, "Paths"), (&mut p.lines, "Lines"), (&mut p.props, "Props")] {
                    if widgets::chip(ui, name, None, *on).on_hover_text("Off: clicks in the plan go through it").clicked() {
                        *on = !*on;
                    }
                }
            });
            widgets::hint(ui, "Turn a kind off to click through it in the plan, say to grab the floor under a house. A click near a region's edge picks the edge, to give it a slope or terraces.");
        });
        section(ui, "pal quick", "Quick keys", None, true, |ui| {
            for (keys, what) in [
                (&["Shift", "click"][..], "select several regions or edges"),
                (&["Alt", "drag"][..], "move without snapping"),
                (&["S"][..], "sharp or smooth corner"),
                (&["PgUp", "PgDn"][..], "raise or sink 20"),
                (&["W"][..], "play from the cursor"),
                (&["?"][..], "every key"),
            ] {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.0;
                    for k in keys {
                        widgets::kbd(ui, k);
                    }
                    ui.add_space(4.0);
                    ui.label(RichText::new(what).size(12.5).color(MUTED));
                });
            }
        });
    }

    fn region_palette(&mut self, ui: &mut egui::Ui) {
        let ground = self.lib_texture(ui.ctx(), &self.theme.texture_name(&self.theme.floor.material.clone()));
        let water = self.lib_texture(ui.ctx(), &self.theme.texture_name(&self.theme.water.material.clone()));
        let styles: Vec<String> = self.theme.wall_styles.keys().cloned().collect();
        section(ui, "pal rkind", "Kind", None, true, |ui| {
            let w = (ui.available_width() - 16.0) / 3.0;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let n = &mut self.new;
                if widgets::card(ui, w, n.region_kind == "floor", "Floor", "Ground at one height", |p, r| widgets::texture_swatch(p, r, ground, 1.6, Color32::WHITE, style::FLOOR)).clicked() {
                    n.region_kind = "floor".into();
                }
                if widgets::card(ui, w, n.region_kind == "water", "Water", "A pond: a bed and a surface", |p, r| widgets::texture_swatch(p, r, water, 3.0, Color32::WHITE, style::WATER)).clicked() {
                    n.region_kind = "water".into();
                }
                if widgets::card(ui, w, n.region_kind == "pit", "Pit", "A drop into the void", |p, r| widgets::texture_swatch(p, r, ground, 1.6, Color32::from_gray(40), Color32::from_gray(20))).clicked() {
                    n.region_kind = "pit".into();
                }
            });
        });
        section(ui, "pal rnew", "New regions", None, true, |ui| {
            let n = &mut self.new;
            if n.region_kind == "water" {
                field(ui, "Depth", Some("From its surface, 20 below the ground round it, down to its bed"), |ui| ui.add(egui::DragValue::new(&mut n.depth).speed(1.0).range(10.0..=2000.0)));
            } else if n.region_kind == "pit" {
                field(ui, "Depth", Some("How far its walls go down below the ground round it: Link voids out falling in"), |ui| ui.add(egui::DragValue::new(&mut n.pit_depth).speed(5.0).range(220.0..=5000.0)));
            } else {
                field(ui, "Rise", Some("How far above the ground it's drawn on a new region starts. Change it later in the inspector (PgUp / PgDn)."), |ui| {
                    ui.add(egui::DragValue::new(&mut n.rise).speed(1.0).range(-2000.0..=2000.0))
                });
            }
            field(ui, "Edge", Some("The wall style of its own walls (where it's the higher side)"), |ui| style_combo(ui, "new edge", &mut n.edge, &styles, "theme rules"));
            widgets::hint(ui, "Click points; click the first one or press Enter to close it. A click on a node or an edge shares it.");
            widgets::hint(ui, "Drawn outside everything, it's a new area at the ground's height, with an edge of the world of its own: join it to the rest with a tunnel (U).");
        });
    }

    fn path_palette(&mut self, ui: &mut egui::Ui) {
        section(ui, "pal pmode", "Mode", None, true, |ui| {
            let w = (ui.available_width() - 8.0) / 2.0;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let n = &mut self.new;
                if widgets::card(ui, w, n.path_mode == "attached", "Attached", "An embankment down to the ground", |p, r| mode_swatch(p, r, false)).clicked() {
                    n.path_mode = "attached".into();
                }
                if widgets::card(ui, w, n.path_mode == "floating", "Floating", "A bridge, open underneath", |p, r| mode_swatch(p, r, true)).clicked() {
                    n.path_mode = "floating".into();
                }
            });
        });
        section(ui, "pal pnew", "New paths", None, true, |ui| {
            let n = &mut self.new;
            field(ui, "Width", None, |ui| ui.add(egui::DragValue::new(&mut n.path_width).speed(1.0).range(20.0..=2000.0)));
            field(ui, "Bridge shape", Some("Under a floating path"), |ui| widgets::segmented(ui, &mut n.path_shape, &[(None, "Theme's"), (Some("rock".to_string()), "Rock arch"), (Some("slab".to_string()), "Slab")]));
            widgets::hint(ui, "Put each end inside the floor it starts or finishes on: an end takes that floor's height.");
        });
    }

    fn brush_palette(&mut self, ui: &mut egui::Ui) {
        section(ui, "pal bmode", "Mode", None, true, |ui| {
            let w = (ui.available_width() - 12.0) / 3.0;
            let modes = [
                (Mode::Raise, "Raise", Icon::Raise, 1),
                (Mode::Lower, "Lower", Icon::Lower, 2),
                (Mode::Smooth, "Smooth", Icon::Smooth, 3),
                (Mode::Flatten, "Flatten", Icon::Flatten, 4),
                (Mode::Bumps, "Bumps", Icon::Bumps, 5),
                (Mode::Erase, "Erase", Icon::Erase, 6),
            ];
            for row in modes.chunks(3) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    for &(m, name, icon, key) in row {
                        let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 58.0), Sense::click());
                        let on = self.brush.mode == m;
                        let c = brush_colour(m);
                        let p = ui.painter();
                        let (fill, stroke) = if on { (style::soft(c, 0.12), c) } else if resp.hovered() { (PANEL2, LINE2) } else { (PANEL2, LINE) };
                        p.rect(rect, 9.0, fill, Stroke::new(1.0, stroke), StrokeKind::Inside);
                        icons::paint(p, Rect::from_center_size(rect.center() - Vec2::new(0.0, 8.0), Vec2::splat(22.0)), icon, c, 1.8);
                        p.text(rect.center_bottom() - Vec2::new(0.0, 9.0), Align2::CENTER_BOTTOM, name, FontId::proportional(12.0), if on || resp.hovered() { TEXT } else { MUTED });
                        p.text(rect.right_top() + Vec2::new(-6.0, 4.0), Align2::RIGHT_TOP, key.to_string(), FontId::monospace(9.5), FAINT);
                        if resp.on_hover_text(format!("key {key}")).clicked() {
                            self.brush.mode = m;
                        }
                    }
                });
            }
        });
        section(ui, "pal bset", "Brush", None, true, |ui| {
            let b = &mut self.brush;
            field(ui, "Size", Some("Radius ([ and ])"), |ui| ui.add(egui::DragValue::new(&mut b.radius).speed(5.0).range(30.0..=5000.0).suffix(" units")));
            field(ui, "Strength", Some("Raise, lower, bumps: units per second at the middle. Others: how fast they act"), |ui| ui.add(egui::DragValue::new(&mut b.strength).speed(1.0).range(1.0..=2000.0)));
            field(ui, "Hard core", Some("The share of the radius at full strength"), |ui| ui.add(egui::Slider::new(&mut b.falloff, 0.0..=0.95).show_value(false)));
            if b.mode == Mode::Bumps {
                field(ui, "Bump size", None, |ui| ui.add(egui::DragValue::new(&mut b.bump_scale).speed(5.0).range(50.0..=5000.0)));
                field(ui, "Seed", None, |ui| ui.add(egui::DragValue::new(&mut b.seed)));
            }
            falloff_curve(ui, b.falloff, b.radius, brush_colour(b.mode));
            widgets::hint(ui, "One smooth height offset over everything: regions, paths and bridges ride on it, ponds stay level. Region heights stay as they are.");
            if self.doc.terrain.is_some() && widgets::button(ui, Some(Icon::Erase), "Clear all painted terrain", Some(ui.available_width()), true).clicked() {
                self.doc.terrain = None;
            }
        });
    }

    fn kit_palette(&mut self, ui: &mut egui::Ui) {
        let Some(kit) = self.kit.clone() else {
            egui::Frame::new().inner_margin(Margin::symmetric(14, 0)).show(ui, |ui| {
                widgets::callout(ui, "No kit. It's cut from the extracted Kokiri Forest (extracted/scenes/overworld/spot04) by `overworld kit-pieces`, into out/overworld/kit/kokiri.", WARN);
            });
            return;
        };
        let pieces: Vec<&Piece> = kit.pieces.iter().filter(|p| p.kind != "hedge").collect();
        egui::Frame::new().inner_margin(Margin { left: 14, right: 14, top: 0, bottom: 12 }).show(ui, |ui| {
            widgets::search(ui, &mut self.kit_query, &format!("Search the {} kit", self.theme.name));
            ui.add_space(2.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(5.0, 5.0);
                for (k, name) in CATS {
                    let n = pieces.iter().filter(|p| k == "all" || p.kind == k).count();
                    if n == 0 {
                        continue;
                    }
                    if widgets::chip(ui, name, Some(&n.to_string()), self.kit_cat == k).clicked() {
                        self.kit_cat = k.into();
                    }
                }
            });
        });
        let recent: Vec<&Piece> = self.recent.iter().filter_map(|n| pieces.iter().find(|p| &p.name == n).copied()).collect();
        if !recent.is_empty() {
            section(ui, "pal recent", "Recent", None, true, |ui| {
                let w = (ui.available_width() - 6.0 * 3.0) / 4.0;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    for p in recent.iter().take(4) {
                        if self.thumb_tile(ui, p, w, false).clicked() {
                            self.choose_piece(&p.name);
                        }
                    }
                });
            });
        }
        let q = self.kit_query.to_lowercase();
        let shown: Vec<&Piece> = pieces.iter().filter(|p| (self.kit_cat == "all" || p.kind == self.kit_cat) && (q.is_empty() || p.label.to_lowercase().contains(&q) || p.name.contains(&q))).copied().collect();
        let title = CATS.iter().find(|c| c.0 == self.kit_cat).map_or("Kit", |c| if c.0 == "all" { "Kit" } else { c.1 });
        section(ui, "pal kit", title, Some(shown.len().to_string()), true, |ui| {
            if shown.is_empty() {
                widgets::hint(ui, format!("Nothing in the kit matches “{}”.", self.kit_query));
            }
            let w = (ui.available_width() - 6.0 * 2.0) / 3.0;
            for row in shown.chunks(3) {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    for p in row {
                        if self.thumb_tile(ui, p, w, true).clicked() {
                            self.choose_piece(&p.name);
                        }
                    }
                });
            }
        });
        section(ui, "pal place", "Placing", None, true, |ui| {
            field(ui, "Turn new", Some("Degrees counter-clockwise from north, for the next prop placed (Q / E turn the selected one)"), |ui| {
                ui.add(egui::DragValue::new(&mut self.place_yaw).speed(1.0).range(-180.0..=180.0).suffix("°"))
            });
            widgets::hint(ui, "Click in the plan or 3D to place. Openings and wall pieces fit themselves to the nearest wall: hover one (best in 3D) and a ghost shows where it'd go. Hedges are drawn with the Hedge tool (J).");
        });
    }

    fn choose_piece(&mut self, name: &str) {
        self.piece = name.to_string();
        self.recent.retain(|n| n != name);
        self.recent.insert(0, name.to_string());
        self.recent.truncate(4);
    }

    /// A kit piece's tile: its thumbnail, and its name under it if `label`; a card with its size
    /// and what it's for on hover.
    fn thumb_tile(&mut self, ui: &mut egui::Ui, piece: &Piece, w: f32, label: bool) -> egui::Response {
        let tex = self.thumbs.get(ui.ctx(), piece, self.lib.as_deref());
        let on = self.piece == piece.name;
        let galley = label.then(|| {
            let mut job = LayoutJob::single_section(piece.label.clone(), TextFormat { font_id: FontId::proportional(11.5), color: TEXT, ..Default::default() });
            job.wrap.max_width = w - 8.0;
            job.wrap.max_rows = 2;
            job.halign = egui::Align::Center;
            ui.painter().layout_job(job)
        });
        let img = w - 8.0;
        let h = 4.0 + img + if label { 4.0 + 30.0 } else { 4.0 };
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
        let p = ui.painter();
        let (fill, stroke) = if on { (style::soft(ACCENT, 0.12), style::soft(ACCENT, 0.6)) } else if resp.hovered() { (PANEL2, LINE2) } else { (PANEL2, LINE) };
        p.rect(rect, 9.0, fill, Stroke::new(1.0, stroke), StrokeKind::Inside);
        let ir = Rect::from_min_size(rect.min + Vec2::new(4.0, 4.0), Vec2::splat(img));
        tile_backdrop(p, ir);
        if let Some(t) = tex {
            p.image(t, ir, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        }
        if piece.scale.locked() && label {
            let g = p.layout_job(widgets::caps("fixed", 8.5, WARN));
            let tag = Rect::from_min_size(ir.min + Vec2::new(4.0, 4.0), g.size() + Vec2::new(8.0, 4.0));
            p.rect_filled(tag, 3.0, Color32::from_rgba_unmultiplied(20, 16, 8, 200));
            p.galley(tag.center() - g.size() / 2.0, g, WARN);
        }
        if let Some(g) = galley {
            // centred rows: the galley's x is their middle
            p.galley(Pos2::new(rect.center().x, ir.bottom() + 4.0), g.clone(), TEXT);
        }
        let size = [piece.bounds[1][0] - piece.bounds[0][0], piece.bounds[1][1] - piece.bounds[0][1], piece.bounds[1][2] - piece.bounds[0][2]];
        resp.on_hover_ui(|ui| {
            ui.set_max_width(260.0);
            ui.label(RichText::new(&piece.label).font(style::semibold(13.5)).color(TEXT));
            ui.label(
                RichText::new(format!("{:.0} × {:.0} × {:.0} · {} triangles · {} collision vertices{}", size[0], size[1], size[2], piece.tris.len(), piece.collision_vertices(), if piece.scale.locked() { " · fixed size" } else { "" }))
                    .monospace()
                    .size(11.0)
                    .color(MUTED),
            );
            if !piece.functions.is_empty() {
                ui.label(RichText::new(format!("Link can use: {}", piece.functions.join(", "))).size(12.0).color(ACCENT));
            }
            if !piece.about.is_empty() {
                ui.add(egui::Label::new(RichText::new(&piece.about).size(12.0).color(MUTED)).wrap());
            }
        })
    }

    fn line_palette(&mut self, ui: &mut egui::Ui, kind: &'static str) {
        let th = self.theme.clone();
        let tex = |s: &mut Self, ui: &egui::Ui, role: &str| s.lib_texture(ui.ctx(), &th.texture_name(role));
        match kind {
            "dirt" => {
                let sw = th.overlay_texture(&format!("{}+dirt", th.floor.material)).and_then(|n| self.lib_texture(ui.ctx(), &n));
                section(ui, "pal look", "Look", None, true, |ui| {
                    let w = (ui.available_width() - 8.0) / 2.0;
                    widgets::card(ui, w, true, "Kokiri dirt", "Fades into the floor over a soft edge", |p, r| widgets::texture_swatch(p, r, sw, 1.4, Color32::WHITE, style::soft(Color32::from_rgb(0xbf, 0x9a, 0x5e), 1.0)));
                    let theme_w = th.dirt.as_ref().map_or(160.0, |d| d.width);
                    field(ui, "Width", Some("The theme's unless you set one"), |ui| widgets::theme_num(ui, &mut self.new.dirt_width, theme_w, 1.0, 20.0..=2000.0));
                });
            }
            "fence" => {
                let rails = tex(self, ui, th.fences.get("fence").map_or("fence", |f| f.material.as_str()));
                let lattice = tex(self, ui, th.fences.get("lattice").map_or("fence_post", |f| f.material.as_str()));
                section(ui, "pal look", "Style", None, true, |ui| {
                    let w = (ui.available_width() - 8.0) / 2.0;
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        let n = &mut self.new;
                        let tall = |k: &str| th.fences.get(k).map_or(0.0, |f| f.height);
                        if widgets::card(ui, w, n.fence == "fence", "Rails", &format!("{:.0} tall, a post every 40", tall("fence")), |p, r| widgets::texture_swatch(p, r, rails, 1.0, Color32::WHITE, PANEL2)).clicked() {
                            n.fence = "fence".into();
                        }
                        if widgets::card(ui, w, n.fence == "lattice", "Lattice", &format!("{:.0} tall, by the crawlspace", tall("lattice")), |p, r| widgets::texture_swatch(p, r, lattice, 1.5, Color32::WHITE, PANEL2)).clicked() {
                            n.fence = "lattice".into();
                        }
                    });
                    widgets::toggle(ui, &mut self.new.closed, "Close the loop (back to the first node)");
                    widgets::hint(ui, "Straight between nodes, standing on the ground; collides from both sides.");
                });
            }
            "bridge" => {
                let deck = th.hanging.as_ref().and_then(|h| tex(self, ui, &h.deck.clone()));
                section(ui, "pal look", "Look", None, true, |ui| {
                    let w = (ui.available_width() - 8.0) / 2.0;
                    let sag = th.hanging.as_ref().map_or(0.0, |h| h.sag * 100.0);
                    widgets::card(ui, w, true, "Planks and rope", &format!("Sags {sag:.0}% of each span"), |p, r| widgets::texture_swatch(p, r, deck, 2.0, Color32::WHITE, PANEL2));
                    let theme_w = th.hanging.as_ref().map_or(80.0, |h| h.width);
                    field(ui, "Width", Some("The theme's unless you set one"), |ui| widgets::theme_num(ui, &mut self.new.bridge_width, theme_w, 1.0, 20.0..=2000.0));
                    widgets::hint(ui, "Click a point on each floor it joins (anywhere on it: the ends land at the floor's edge), then press Enter. A deck too steep to walk is reported.");
                });
            }
            "tunnel" => {
                let wall = th.tunnel.as_ref().and_then(|t| tex(self, ui, &t.wall.clone()));
                section(ui, "pal look", "Look", None, true, |ui| {
                    let w = (ui.available_width() - 8.0) / 2.0;
                    widgets::card(ui, w, true, "Rock", "An arch, darker inside, a grass floor", |p, r| widgets::texture_swatch(p, r, wall, 1.5, Color32::WHITE, PANEL2));
                    let (tw, tt) = th.tunnel.as_ref().map_or((200.0, 200.0), |t| (t.width, t.height));
                    let (w0, w1) = overworld::tunnels::WIDTH;
                    let (h0, h1) = overworld::tunnels::HEIGHT;
                    field(ui, "Width", Some("Across its floor: the theme's unless you set one"), |ui| widgets::theme_num(ui, &mut self.new.tunnel_width, tw, 1.0, w0..=w1));
                    field(ui, "Height", Some("Its floor to its roof: the theme's unless you set one"), |ui| widgets::theme_num(ui, &mut self.new.tunnel_height, tt, 1.0, h0..=h1));
                    widgets::toggle(ui, &mut self.new.tunnel_rough, "Rough walls").on_hover_text("Its walls and roof push in and out and it wanders a little, like a cave (the inspector has the settings)");
                    widgets::hint(ui, "Click on the floor in front of the wall it goes into, any bends, then on the floor beyond the far wall; Enter to finish. Its mouths go where it meets the walls. The wall has to be taller than the tunnel, with ground over it all the way.");
                });
            }
            _ => {
                let top = th.hedge.as_ref().and_then(|h| tex(self, ui, &h.top.clone()));
                section(ui, "pal look", "Look", None, true, |ui| {
                    let w = (ui.available_width() - 8.0) / 2.0;
                    let tall = th.hedge.as_ref().map_or(28.0, |h| h.height);
                    widgets::card(ui, w, true, "Tall grass", &format!("{tall:.0} tall, with grass skirts"), |p, r| widgets::texture_swatch(p, r, top, 2.0, Color32::WHITE, style::FLOOR));
                    widgets::hint(ui, "Click its corners; click the first one or press Enter to close it. Link wades through it, with tall-grass footsteps.");
                });
            }
        }
    }

    /// A texture from the library as an egui texture (for swatches), loaded once.
    pub(super) fn lib_texture(&mut self, ctx: &egui::Context, name: &str) -> Option<egui::TextureId> {
        let lib = self.lib.clone()?;
        let h = self.textures.entry(name.to_string()).or_insert_with(|| {
            let (w, h, px) = lib.rgba(name)?;
            let ci = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &px);
            let opts = egui::TextureOptions { magnification: egui::TextureFilter::Linear, minification: egui::TextureFilter::Linear, wrap_mode: egui::TextureWrapMode::Repeat, mipmap_mode: None };
            Some(ctx.load_texture(name.to_string(), ci, opts))
        });
        h.as_ref().map(|h| h.id())
    }
}

/// The dark backdrop behind a thumbnail, lighter at the top.
pub fn tile_backdrop(p: &egui::Painter, r: Rect) {
    let mut mesh = egui::Mesh::default();
    let (top, bottom) = (Color32::from_rgb(0x34, 0x44, 0x3e), Color32::from_rgb(0x1f, 0x27, 0x24));
    let i = mesh.vertices.len() as u32;
    mesh.colored_vertex(r.left_top(), top);
    mesh.colored_vertex(r.right_top(), top);
    mesh.colored_vertex(r.right_bottom(), bottom);
    mesh.colored_vertex(r.left_bottom(), bottom);
    mesh.add_triangle(i, i + 1, i + 2);
    mesh.add_triangle(i, i + 2, i + 3);
    p.add(mesh);
}

/// A theme wall style, or the theme's own (`none`).
pub fn style_combo(ui: &mut egui::Ui, id: &str, value: &mut Option<String>, styles: &[String], none: &str) -> bool {
    let before = value.clone();
    let text = RichText::new(value.clone().unwrap_or(none.into())).color(if value.is_some() { TEXT } else { FAINT });
    egui::ComboBox::from_id_salt(id).selected_text(text).width(ui.available_width() - if value.is_some() { 32.0 } else { 0.0 }).show_ui(ui, |ui| {
        ui.selectable_value(value, None, none);
        for s in styles {
            ui.selectable_value(value, Some(s.clone()), s);
        }
    });
    if value.is_some() && widgets::reset(ui).clicked() {
        *value = None;
    }
    *value != before
}

/// A side view of a path mode: an embankment, or a deck on its own.
fn mode_swatch(p: &egui::Painter, r: Rect, floating: bool) {
    p.rect_filled(r, 6.0, Color32::from_rgb(0x24, 0x30, 0x2b));
    let at = |x: f32, y: f32| Pos2::new(r.left() + x / 92.0 * r.width(), r.top() + y / 46.0 * r.height());
    p.line_segment([at(2.0, 40.0), at(90.0, 40.0)], Stroke::new(2.0, Color32::from_rgb(0x4a, 0x5a, 0x52)));
    let green = Color32::from_rgb(0x9c, 0xcf, 0x72);
    if floating {
        p.add(Shape::convex_polygon(vec![at(6.0, 15.0), at(86.0, 15.0), at(86.0, 21.0), at(6.0, 21.0)], Color32::from_rgb(0x8a, 0x8f, 0x86), Stroke::NONE));
        p.add(egui::epaint::QuadraticBezierShape::from_points_stroke([at(22.0, 21.0), at(46.0, 36.0), at(70.0, 21.0)], false, Color32::TRANSPARENT, Stroke::new(4.0, Color32::from_rgb(0x8a, 0x8f, 0x86))));
        p.line_segment([at(6.0, 15.0), at(86.0, 15.0)], Stroke::new(2.0, green));
    } else {
        p.add(Shape::convex_polygon(vec![at(8.0, 40.0), at(70.0, 12.0), at(86.0, 12.0), at(86.0, 40.0)], Color32::from_rgb(0x6b, 0x9a, 0x4b), Stroke::NONE));
        p.add(Shape::line(vec![at(8.0, 40.0), at(70.0, 12.0), at(86.0, 12.0)], Stroke::new(2.0, green)));
    }
}

/// The brush's cross-section: full strength over its hard core, easing to nothing at its edge.
fn falloff_curve(ui: &mut egui::Ui, core: f64, radius: f64, col: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 76.0), Sense::hover());
    let p = ui.painter();
    p.rect(rect, 8.0, FIELD, Stroke::new(1.0, LINE), StrokeKind::Inside);
    let plot = Rect::from_min_max(rect.min + Vec2::new(10.0, 8.0), rect.max - Vec2::new(10.0, 22.0));
    let f = |t: f64| -> f64 {
        if t <= core {
            1.0
        } else {
            let u = ((t - core) / (1.0 - core).max(1e-6)).min(1.0);
            1.0 - (3.0 * u * u - 2.0 * u * u * u)
        }
    };
    let pts: Vec<Pos2> = (0..=80)
        .map(|i| {
            let x = -1.0 + i as f64 / 40.0;
            Pos2::new(plot.left() + ((x + 1.0) / 2.0) as f32 * plot.width(), plot.bottom() - f(x.abs()) as f32 * plot.height())
        })
        .collect();
    p.line_segment([Pos2::new(plot.left(), plot.bottom()), Pos2::new(plot.right(), plot.bottom())], Stroke::new(1.0, LINE2));
    let mut mesh = egui::Mesh::default();
    let fill = style::soft(col, 0.16);
    for w in pts.windows(2) {
        let i = mesh.vertices.len() as u32;
        mesh.colored_vertex(w[0], fill);
        mesh.colored_vertex(w[1], fill);
        mesh.colored_vertex(Pos2::new(w[1].x, plot.bottom()), fill);
        mesh.colored_vertex(Pos2::new(w[0].x, plot.bottom()), fill);
        mesh.add_triangle(i, i + 1, i + 2);
        mesh.add_triangle(i, i + 2, i + 3);
    }
    p.add(mesh);
    p.add(Shape::line(pts, Stroke::new(2.0, col)));
    for s in [-1.0, 1.0] {
        let x = plot.center().x + (s * core) as f32 * plot.width() / 2.0;
        p.add(Shape::dashed_line(&[Pos2::new(x, plot.top()), Pos2::new(x, plot.bottom())], Stroke::new(1.0, style::soft(col, 0.6)), 2.0, 3.0));
    }
    p.text(Pos2::new(rect.center().x, rect.bottom() - 7.0), Align2::CENTER_BOTTOM, format!("hard core {:.0}% · radius {:.0}", core * 100.0, radius), FontId::monospace(10.5), MUTED);
}
