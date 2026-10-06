//! The panels' widgets, drawn to the editor's style: collapsible sections, label / value rows,
//! segmented choices, cards, chips, switches, and fields that show the theme's value until you
//! set your own.

use super::icons::{self, Icon};
use super::style::{self, ACCENT, BAD, FAINT, FIELD, LINE, LINE2, MUTED, PANEL2, RAISED, TEXT};
use eframe::egui::{self, text::LayoutJob, Align, Align2, Color32, CornerRadius, FontId, Id, Layout, Margin, Pos2, Rect, Response, RichText, Sense, Stroke, StrokeKind, TextFormat, Ui, Vec2};

/// The width of a row's label column.
pub const LABEL_W: f32 = 92.0;

/// Upper-case, letter-spaced, as the panels' headings are.
pub fn caps(text: &str, size: f32, color: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.append(&text.to_uppercase(), 0.0, TextFormat { font_id: style::semibold(size), color, extra_letter_spacing: size * 0.08, ..Default::default() });
    job
}

/// A collapsible section: a heading row across the panel (with a count on the right, if any), its
/// contents indented beneath. Open or closed is remembered by `id`.
pub fn section<R>(ui: &mut Ui, id: &str, title: &str, count: Option<String>, default_open: bool, add: impl FnOnce(&mut Ui) -> R) -> Option<R> {
    let id = Id::new(("section", id));
    let mut open = ui.data_mut(|d| *d.get_persisted_mut_or(id, default_open));
    let full = ui.max_rect();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 32.0), Sense::click());
    let p = ui.painter();
    p.line_segment([Pos2::new(full.left(), rect.top()), Pos2::new(full.right(), rect.top())], Stroke::new(1.0, LINE));
    let col = if resp.hovered() { TEXT } else { MUTED };
    let chev = Rect::from_center_size(Pos2::new(rect.left() + 20.0, rect.center().y + 1.0), Vec2::splat(13.0));
    if open {
        icons::paint(p, chev, Icon::Chevron, col, 2.2);
    } else {
        // pointing right: the chevron turned a quarter
        let c = chev.center();
        let s = chev.width() / 24.0;
        p.add(egui::Shape::line(vec![c + Vec2::new(-3.0, -6.0) * s, c + Vec2::new(3.0, 0.0) * s, c + Vec2::new(-3.0, 6.0) * s], Stroke::new(2.2 * s.max(0.6), col)));
    }
    let g = p.layout_job(caps(title, 10.5, col));
    p.galley(Pos2::new(rect.left() + 32.0, rect.center().y - g.size().y / 2.0 + 1.0), g, col);
    if let Some(c) = count {
        p.text(Pos2::new(rect.right() - 14.0, rect.center().y + 1.0), Align2::RIGHT_CENTER, c, FontId::monospace(11.0), FAINT);
    }
    if resp.clicked() {
        open = !open;
        ui.data_mut(|d| d.insert_persisted(id, open));
    }
    if !open {
        return None;
    }
    let r = egui::Frame::new().inner_margin(Margin { left: 14, right: 14, top: 0, bottom: 12 }).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 8.0;
        add(ui)
    });
    Some(r.inner)
}

/// A label / value row: the label (with an ⓘ for `tip`) in its column, the value after it.
pub fn field<R>(ui: &mut Ui, label: &str, tip: Option<&str>, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(Vec2::new(LABEL_W, 24.0), Layout::left_to_right(Align::Center), |ui| {
            ui.set_min_width(LABEL_W);
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.label(RichText::new(label).color(MUTED).size(12.5));
            if let Some(t) = tip {
                info(ui, t);
            }
        });
        add(ui)
    })
    .inner
}

/// A small ⓘ that explains on hover.
pub fn info(ui: &mut Ui, tip: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
    let col = if resp.hovered() { TEXT } else { FAINT };
    icons::paint(ui.painter(), rect, Icon::Info, col, 1.9);
    resp.on_hover_text(tip)
}

/// A muted note, wrapped.
pub fn hint(ui: &mut Ui, text: impl Into<String>) {
    ui.label(RichText::new(text.into()).color(FAINT).size(12.0));
}

/// A key cap.
pub fn kbd(ui: &mut Ui, text: &str) -> Response {
    let font = FontId::monospace(11.0);
    let g = ui.painter().layout_no_wrap(text.to_string(), font, TEXT);
    let size = Vec2::new(g.size().x + 10.0, 19.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    let p = ui.painter();
    p.rect(rect, 4.0, RAISED, Stroke::new(1.0, LINE2), StrokeKind::Inside);
    p.line_segment([rect.left_bottom() + Vec2::new(3.0, -0.5), rect.right_bottom() + Vec2::new(-3.0, -0.5)], Stroke::new(1.5, LINE2));
    p.galley(rect.center() - g.size() / 2.0 - Vec2::new(0.0, 0.5), g, TEXT);
    resp
}

/// A choice of a few, as one segmented control across the available width. Returns whether it
/// changed.
pub fn segmented<T: PartialEq + Clone>(ui: &mut Ui, cur: &mut T, options: &[(T, &str)]) -> bool {
    let w = ui.available_width().max(40.0 * options.len() as f32);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 27.0), Sense::hover());
    let p = ui.painter().clone();
    p.rect(rect, 7.0, FIELD, Stroke::new(1.0, LINE), StrokeKind::Inside);
    let inner = rect.shrink(2.0);
    let n = options.len() as f32;
    let mut changed = false;
    for (k, (v, label)) in options.iter().enumerate() {
        let r = Rect::from_min_size(inner.min + Vec2::new(inner.width() / n * k as f32, 0.0), Vec2::new(inner.width() / n, inner.height())).shrink2(Vec2::new(1.0, 0.0));
        let resp = ui.interact(r, ui.id().with(("seg", rect.min.x as i32, rect.min.y as i32, k)), Sense::click());
        let on = *cur == *v;
        if on {
            p.rect_filled(r.translate(Vec2::new(0.0, 1.0)), 5.0, egui::Color32::from_black_alpha(70));
            p.rect_filled(r, 5.0, RAISED);
        }
        let col = if on || resp.hovered() { TEXT } else { MUTED };
        let g = p.layout_no_wrap(label.to_string(), FontId::proportional(12.5), col);
        let pos = r.center() - g.size() / 2.0;
        p.with_clip_rect(r).galley(pos, g, col);
        if resp.clicked() && !on {
            *cur = v.clone();
            changed = true;
        }
    }
    changed
}

/// A pill: `text`, highlighted when `on`. Clickable if `sense` is.
pub fn chip(ui: &mut Ui, text: &str, value: Option<&str>, on: bool) -> Response {
    let font = FontId::proportional(11.5);
    let vfont = FontId::monospace(11.0);
    let tcol = if on { ACCENT } else { MUTED };
    let gv = value.map(|v| ui.painter().layout_no_wrap(v.to_string(), vfont, if on { ACCENT } else { TEXT }));
    let gt = ui.painter().layout_no_wrap(text.to_string(), font, tcol);
    let w = gt.size().x + gv.as_ref().map_or(0.0, |g| g.size().x + 5.0) + 16.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 21.0), Sense::click());
    let p = ui.painter();
    let (fill, stroke) = if on { (style::soft(ACCENT, 0.14), style::soft(ACCENT, 0.55)) } else if resp.hovered() { (RAISED, LINE2) } else { (PANEL2, LINE) };
    p.rect(rect, 10.5, fill, Stroke::new(1.0, stroke), StrokeKind::Inside);
    let mut x = rect.left() + 8.0;
    if let Some(g) = gv {
        let gw = g.size().x;
        p.galley(Pos2::new(x, rect.center().y - g.size().y / 2.0), g, TEXT);
        x += gw + 5.0;
    }
    p.galley(Pos2::new(x, rect.center().y - gt.size().y / 2.0), gt, tcol);
    resp
}

/// A small square button with an icon, framed only on hover (or `on`).
pub fn icon_button(ui: &mut Ui, icon: Icon, size: f32, on: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let p = ui.painter();
    if on {
        p.rect_filled(rect, 6.0, style::soft(ACCENT, 0.14));
    } else if resp.hovered() {
        p.rect_filled(rect, 6.0, RAISED);
    }
    let col = if on { ACCENT } else if resp.hovered() { TEXT } else { MUTED };
    icons::paint(p, rect.shrink(size * 0.2), icon, col, 1.8);
    resp
}

/// A button with an optional icon, `w` wide (or as wide as its text), in red if `danger`.
pub fn button(ui: &mut Ui, icon: Option<Icon>, text: &str, w: Option<f32>, danger: bool) -> Response {
    let col = if danger { Color32::from_rgb(0xff, 0x9a, 0x90) } else { TEXT };
    let g = ui.painter().layout_no_wrap(text.to_string(), FontId::proportional(12.5), col);
    let iw = if icon.is_some() { 21.0 } else { 0.0 };
    let w = w.unwrap_or(g.size().x + iw + 22.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 30.0), Sense::click());
    let p = ui.painter();
    let (fill, stroke) = match (danger, resp.hovered()) {
        (true, true) => (style::soft(BAD, 0.14), style::soft(BAD, 0.55)),
        (true, false) => (PANEL2, style::soft(BAD, 0.4)),
        (false, true) => (RAISED, LINE2),
        (false, false) => (PANEL2, LINE2),
    };
    p.rect(rect, 7.0, fill, Stroke::new(1.0, stroke), StrokeKind::Inside);
    let total = g.size().x + iw;
    let x0 = rect.center().x - total / 2.0;
    if let Some(i) = icon {
        icons::paint(p, Rect::from_center_size(Pos2::new(x0 + 8.0, rect.center().y), Vec2::splat(15.0)), i, col, 1.8);
    }
    p.galley(Pos2::new(x0 + iw, rect.center().y - g.size().y / 2.0), g, col);
    resp
}

/// A switch with its label.
pub fn toggle(ui: &mut Ui, on: &mut bool, label: &str) -> Response {
    let g = ui.painter().layout_no_wrap(label.to_string(), FontId::proportional(12.5), MUTED);
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(30.0 + 8.0 + g.size().x, 20.0), Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = ui.ctx().animate_bool(resp.id, *on);
    let p = ui.painter();
    let track = Rect::from_min_size(Pos2::new(rect.left(), rect.center().y - 8.5), Vec2::new(30.0, 17.0));
    let (fill, stroke) = if *on { (style::soft(ACCENT, 0.16), style::soft(ACCENT, 0.6)) } else { (RAISED, LINE2) };
    p.rect(track, 8.5, fill, Stroke::new(1.0, stroke), StrokeKind::Inside);
    let knob = Pos2::new(egui::lerp(track.left() + 8.5..=track.right() - 8.5, t), track.center().y);
    p.circle_filled(knob, 5.5, if *on { ACCENT } else { MUTED });
    let tc = if resp.hovered() { TEXT } else { MUTED };
    p.galley(Pos2::new(track.right() + 8.0, rect.center().y - g.size().y / 2.0), g, tc);
    resp
}

/// A warning box: an icon and the text, wrapped.
pub fn callout(ui: &mut Ui, text: &str, col: Color32) {
    egui::Frame::new()
        .fill(style::soft(col, 0.1))
        .stroke(Stroke::new(1.0, style::soft(col, 0.4)))
        .corner_radius(8.0)
        .inner_margin(Margin { left: 10, right: 10, top: 8, bottom: 8 })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_top(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                icons::paint(ui.painter(), r, Icon::Warn, col, 2.0);
                ui.add(egui::Label::new(RichText::new(text).color(col.lerp_to_gamma(TEXT, 0.45)).size(12.5)).wrap());
            });
        });
}

/// A number the theme supplies unless the document sets its own: greyed with "theme" until you
/// drag or type a value, and a reset button (back to the theme's) once you have.
pub fn theme_num(ui: &mut Ui, value: &mut Option<f64>, theme: f64, speed: f64, range: std::ops::RangeInclusive<f64>) -> bool {
    let mut changed = false;
    match value {
        Some(v) => {
            changed |= ui.add(egui::DragValue::new(v).speed(speed).range(range)).changed();
            if reset(ui).clicked() {
                *value = None;
                changed = true;
            }
        }
        None => {
            let mut v = theme;
            let r = ui.scope(|ui| {
                ui.visuals_mut().override_text_color = Some(FAINT);
                ui.add(egui::DragValue::new(&mut v).speed(speed).range(range))
            });
            if r.inner.changed() {
                *value = Some(v);
                changed = true;
            }
            theme_tag(ui);
        }
    }
    changed
}

/// The little "theme" tag beside a value the theme supplies.
pub fn theme_tag(ui: &mut Ui) {
    let g = ui.painter().layout_job(caps("theme", 9.0, FAINT));
    let (rect, _) = ui.allocate_exact_size(g.size() + Vec2::new(8.0, 5.0), Sense::hover());
    ui.painter().rect_stroke(rect, 4.0, Stroke::new(1.0, LINE2), StrokeKind::Inside);
    ui.painter().galley(rect.center() - g.size() / 2.0, g, FAINT);
}

/// The ↺ button: back to the theme's (or the default).
pub fn reset(ui: &mut Ui) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
    let p = ui.painter();
    p.rect(rect, 6.0, if resp.hovered() { RAISED } else { PANEL2 }, Stroke::new(1.0, LINE), StrokeKind::Inside);
    icons::paint(p, rect.shrink(5.0), Icon::Reset, ACCENT, 2.0);
    resp.on_hover_text("Back to the theme's")
}

/// A search field with its magnifier; `hint` while empty.
pub fn search(ui: &mut Ui, text: &mut String, hint: &str) -> Response {
    egui::Frame::new()
        .fill(FIELD)
        .stroke(Stroke::new(1.0, LINE))
        .corner_radius(7.0)
        .inner_margin(Margin { left: 8, right: 8, top: 3, bottom: 3 })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(15.0), Sense::hover());
                icons::paint(ui.painter(), r, Icon::Search, FAINT, 1.9);
                ui.add(egui::TextEdit::singleline(text).frame(egui::Frame::NONE).hint_text(RichText::new(hint).color(FAINT)).desired_width(ui.available_width()))
            })
            .inner
        })
        .inner
}

/// An option card: a swatch (`paint` draws it), a title and a line under it. `w` wide.
pub fn card(ui: &mut Ui, w: f32, on: bool, title: &str, sub: &str, paint: impl FnOnce(&egui::Painter, Rect)) -> Response {
    let gt = ui.painter().layout_no_wrap(title.to_string(), style::semibold(12.5), TEXT);
    let gs = ui.painter().layout(sub.to_string(), FontId::proportional(11.0), MUTED, w - 12.0);
    let h = 6.0 + 46.0 + 6.0 + gt.size().y + 2.0 + gs.size().y + 8.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
    let p = ui.painter();
    let (fill, stroke) = if on { (style::soft(ACCENT, 0.12), style::soft(ACCENT, 0.6)) } else if resp.hovered() { (PANEL2, LINE2) } else { (PANEL2, LINE) };
    p.rect(rect, 9.0, fill, Stroke::new(1.0, stroke), StrokeKind::Inside);
    let sw = Rect::from_min_size(rect.min + Vec2::new(6.0, 6.0), Vec2::new(w - 12.0, 46.0));
    paint(&p.with_clip_rect(sw), sw);
    let mut y = sw.bottom() + 6.0;
    let th = gt.size().y;
    p.galley(Pos2::new(rect.left() + 8.0, y), gt, TEXT);
    y += th + 2.0;
    p.galley(Pos2::new(rect.left() + 8.0, y), gs, MUTED);
    resp
}

/// Draws a texture across a swatch, `tiles` repeats across it, rounded.
pub fn texture_swatch(p: &egui::Painter, rect: Rect, tex: Option<egui::TextureId>, tiles: f32, tint: Color32, fallback: Color32) {
    match tex {
        Some(id) => {
            let aspect = rect.height() / rect.width();
            let mut mesh = egui::Mesh::with_texture(id);
            mesh.add_rect_with_uv(rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(tiles, tiles * aspect)), tint);
            p.add(mesh);
        }
        None => {
            p.rect_filled(rect, 6.0, fallback);
        }
    }
    p.rect_stroke(rect, CornerRadius::same(2), Stroke::new(1.0, Color32::from_black_alpha(60)), StrokeKind::Inside);
}
