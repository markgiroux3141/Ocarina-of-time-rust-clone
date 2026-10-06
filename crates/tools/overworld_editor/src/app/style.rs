//! The editor's look: its colours, fonts and egui visuals. One forest-green accent on green-grey
//! neutrals; semantic colours (warnings, errors) apart from it.

use eframe::egui::{self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Shadow, Stroke, TextStyle, Vec2};
use std::sync::Arc;

pub const BG: Color32 = Color32::from_rgb(0x12, 0x17, 0x15);
pub const PANEL: Color32 = Color32::from_rgb(0x17, 0x1c, 0x1a);
pub const PANEL2: Color32 = Color32::from_rgb(0x1e, 0x24, 0x22);
pub const RAISED: Color32 = Color32::from_rgb(0x26, 0x2e, 0x2b);
pub const FIELD: Color32 = Color32::from_rgb(0x11, 0x16, 0x14);
pub const LINE: Color32 = Color32::from_rgb(0x2a, 0x33, 0x2f);
pub const LINE2: Color32 = Color32::from_rgb(0x36, 0x41, 0x3c);
pub const TEXT: Color32 = Color32::from_rgb(0xe2, 0xe9, 0xe4);
pub const MUTED: Color32 = Color32::from_rgb(0x93, 0xa2, 0x9a);
pub const FAINT: Color32 = Color32::from_rgb(0x64, 0x71, 0x6a);
pub const ACCENT: Color32 = Color32::from_rgb(0x8c, 0xc9, 0x5e);
pub const ACCENT_INK: Color32 = Color32::from_rgb(0x11, 0x20, 0x0a);
pub const WARN: Color32 = Color32::from_rgb(0xf0, 0xb5, 0x4a);
pub const BAD: Color32 = Color32::from_rgb(0xef, 0x6a, 0x5e);
/// The plan's background, round the level.
pub const VIEW_BG: Color32 = Color32::from_rgb(0x13, 0x1a, 0x16);

/// The plan's colours for the document's things, used for their chips and icons too.
pub const FLOOR: Color32 = Color32::from_rgb(0x9c, 0xcf, 0x72);
pub const WATER: Color32 = Color32::from_rgb(0x5a, 0xa9, 0xe6);
pub const PATH: Color32 = Color32::from_rgb(235, 130, 255);

/// `c` at `a` of its strength over whatever is behind it.
pub fn soft(c: Color32, a: f32) -> Color32 {
    c.gamma_multiply(a)
}

/// Fonts: Segoe UI and Consolas where Windows has them (sharper than egui's own at these sizes),
/// egui's built-in fonts after them for everything else (and as the fallback elsewhere). A
/// "semibold" family for headings and names.
fn fonts(ctx: &egui::Context) {
    let mut defs = FontDefinitions::default();
    let dir = std::path::Path::new("C:/Windows/Fonts");
    let mut load = |key: &str, file: &str| -> bool {
        match std::fs::read(dir.join(file)) {
            Ok(b) => {
                defs.font_data.insert(key.into(), Arc::new(FontData::from_owned(b)));
                true
            }
            Err(_) => false,
        }
    };
    let ui = load("segoe", "segoeui.ttf");
    let semi = load("segoe-semibold", "seguisb.ttf");
    let mono = load("consolas", "consola.ttf");
    let base_prop = defs.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let prop = defs.families.entry(FontFamily::Proportional).or_default();
    if ui {
        prop.insert(0, "segoe".into());
    }
    let mut bold = base_prop;
    if semi {
        bold.insert(0, "segoe-semibold".into());
    } else if ui {
        bold.insert(0, "segoe".into());
    }
    defs.families.insert(FontFamily::Name("semibold".into()), bold);
    if mono {
        defs.families.entry(FontFamily::Monospace).or_default().insert(0, "consolas".into());
    }
    ctx.set_fonts(defs);
}

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("semibold".into()))
}

pub fn apply(ctx: &egui::Context) {
    fonts(ctx);
    ctx.set_theme(egui::Theme::Dark);
    ctx.style_mut_of(egui::Theme::Dark, |s| {
        s.text_styles = [
            (TextStyle::Small, FontId::proportional(11.0)),
            (TextStyle::Body, FontId::proportional(13.5)),
            (TextStyle::Button, FontId::proportional(13.0)),
            (TextStyle::Heading, semibold(16.0)),
            (TextStyle::Monospace, FontId::monospace(12.5)),
        ]
        .into();
        let sp = &mut s.spacing;
        sp.item_spacing = Vec2::new(8.0, 6.0);
        sp.button_padding = Vec2::new(8.0, 4.0);
        sp.interact_size = Vec2::new(36.0, 24.0);
        sp.window_margin = Margin::same(14);
        sp.menu_margin = Margin::same(6);
        sp.combo_width = 120.0;
        sp.slider_width = 150.0;
        sp.icon_width = 15.0;
        sp.icon_width_inner = 9.0;
        let v = &mut s.visuals;
        v.dark_mode = true;
        v.override_text_color = None;
        v.panel_fill = PANEL;
        v.window_fill = PANEL2;
        v.window_stroke = Stroke::new(1.0, LINE2);
        v.window_corner_radius = CornerRadius::same(10);
        v.menu_corner_radius = CornerRadius::same(8);
        v.window_shadow = Shadow { offset: [0, 12], blur: 36, spread: 0, color: Color32::from_black_alpha(140) };
        v.popup_shadow = Shadow { offset: [0, 8], blur: 24, spread: 0, color: Color32::from_black_alpha(120) };
        v.faint_bg_color = PANEL2;
        v.extreme_bg_color = FIELD;
        v.text_edit_bg_color = Some(FIELD);
        v.code_bg_color = PANEL2;
        v.warn_fg_color = WARN;
        v.error_fg_color = BAD;
        v.hyperlink_color = ACCENT;
        v.selection.bg_fill = soft(ACCENT, 0.32);
        v.selection.stroke = Stroke::new(1.0, ACCENT);
        v.weak_text_color = Some(FAINT);
        let r = CornerRadius::same(6);
        let w = &mut v.widgets;
        w.noninteractive.bg_fill = PANEL;
        w.noninteractive.weak_bg_fill = PANEL;
        w.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
        w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
        w.noninteractive.corner_radius = r;
        w.inactive.bg_fill = FIELD;
        w.inactive.weak_bg_fill = PANEL2;
        w.inactive.bg_stroke = Stroke::new(1.0, LINE);
        w.inactive.fg_stroke = Stroke::new(1.0, TEXT);
        w.inactive.corner_radius = r;
        w.hovered.bg_fill = RAISED;
        w.hovered.weak_bg_fill = RAISED;
        w.hovered.bg_stroke = Stroke::new(1.0, LINE2);
        w.hovered.fg_stroke = Stroke::new(1.5, TEXT);
        w.hovered.corner_radius = r;
        w.hovered.expansion = 0.0;
        w.active.bg_fill = RAISED;
        w.active.weak_bg_fill = RAISED;
        w.active.bg_stroke = Stroke::new(1.0, soft(ACCENT, 0.7));
        w.active.fg_stroke = Stroke::new(1.5, TEXT);
        w.active.corner_radius = r;
        w.active.expansion = 0.0;
        w.open.bg_fill = RAISED;
        w.open.weak_bg_fill = RAISED;
        w.open.bg_stroke = Stroke::new(1.0, LINE2);
        w.open.fg_stroke = Stroke::new(1.0, TEXT);
        w.open.corner_radius = r;
    });
}
