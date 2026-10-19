//! The actor inspector's Dialogue section (docs/EDITOR-ACTORS.md, "Texts"; ADR 0079): every line
//! the actor's code can show (its schema's `texts`, in the order the code uses them, under its
//! conditions), each drawn as the game's message box draws it (`crate::text_box`), with the texts
//! it goes on to by itself, and an editor for each in the readable form
//! (`oot_game::message_text`). An edit is the level's: stored in the document by id, it's what the
//! game shows while the level is played; the game's scenes keep their own.

use super::style::{ACCENT, BAD, FAINT, MUTED, TEXT, WARN};
use super::widgets::{self, section};
use super::App;
use crate::actors::Entry;
use eframe::egui::{self, Color32, RichText, Sense, Vec2};
use oot_game::message_text::{self, BoxEnd};
use overworld::doc::Text;

/// `TEXTBOX_TYPE_*`, `TEXTBOX_POS_*` (`message_data_static.h`), in words.
const BOX_TYPES: [&str; 6] = ["Black", "Wooden (a sign)", "Blue (an item)", "Ocarina", "None (at the bottom)", "None, no shadow"];
const BOX_POSITIONS: [&str; 4] = ["Variable", "Top", "Middle", "Bottom"];
/// How many boxes a line shows at most, and how many texts it goes on to.
const MAX_BOXES: usize = 24;
const MAX_CHAIN: usize = 8;
/// The codes a text may use, for the editor's hint.
const CODES: &str = "A newline is a line. {BOX_BREAK} the next box, {COLOR:RED} … {COLOR:DEFAULT} (RED, ADJUSTABLE, BLUE, LIGHTBLUE, PURPLE, YELLOW, BLACK), {NAME} the player's name, {TWO_CHOICE} / {THREE_CHOICE} before the answers, {QUICKTEXT_ENABLE} … {QUICKTEXT_DISABLE} typed at once, {TEXTID:0x1036} go on to another text, {ITEM_ICON:0x2D}, {SHIFT:12}, {TEXT_SPEED:2}, {SFX:0x4807}, {EVENT}. Buttons: [A] [B] [C] [C-Up] [C-Down] [C-Left] [C-Right] [Z] [R] [L] [Control-Pad] ▼.";

impl App {
    pub(super) fn dialogue_section(&mut self, ui: &mut egui::Ui, i: usize, e: &Entry) {
        let a = &self.doc.actors[i];
        let lines = e.schema.lines(a.params as i16, a.rot);
        if lines.is_empty() {
            return;
        }
        let edited = lines.iter().filter(|(id, _)| self.doc.text(*id).is_some()).count();
        let count = if edited > 0 { format!("{} · {edited} edited", lines.len()) } else { lines.len().to_string() };
        section(ui, "ins adialogue", "Dialogue", Some(count), true, |ui| {
            let Some(tb) = self.text_boxes.as_mut() else {
                widgets::hint(ui, "No GPU: the text boxes can't be drawn.");
                return;
            };
            if let Some(err) = &tb.error {
                widgets::callout(ui, &format!("The texts come from {err}"), WARN);
                return;
            }
            let Some(table) = tb.level_table(&self.doc.texts) else { return };
            let game = tb.table.clone().unwrap();
            widgets::hint(ui, "What it says, in the order its code says it. An edit is this level's: the game's own scenes keep their text.");
            for (k, (id, line)) in lines.iter().enumerate() {
                let id = *id;
                ui.add_space(6.0);
                // the line: its id, when, what for
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(6.0, 4.0);
                    widgets::chip(ui, &format!("{:#06x}", id), None, self.doc.text(id).is_some()).on_hover_text(if self.doc.text(id).is_some() { "The level's own text" } else { "The game's text" });
                    let cond = if line.cond.is_empty() { "Whenever it talks".to_string() } else { crate::actors::upper_first(line.cond) };
                    let cond = match line.when {
                        Some(w) => format!("{cond} ({} {})", w.field, w.values.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", ")),
                        None => cond,
                    };
                    ui.add(egui::Label::new(RichText::new(cond).color(TEXT).size(12.5)).wrap());
                });
                if !line.note.is_empty() {
                    ui.add(egui::Label::new(RichText::new(crate::actors::upper_first(line.note)).color(MUTED).size(11.5)).wrap());
                }
                if line.cond.contains("only in") || line.cond.contains("only at") {
                    ui.label(RichText::new("Not in a custom level: it plays as a scene of its own.").color(WARN).size(11.5));
                }
                // the texts it goes on to by itself
                let mut chain = vec![id];
                while chain.len() < MAX_CHAIN {
                    match table.raw(*chain.last().unwrap()).and_then(message_text::next_text_id) {
                        Some(n) if !chain.contains(&n) => chain.push(n),
                        _ => break,
                    }
                }
                let key = crate::text_box::boxes_key(&table, &chain);
                let drawn = if table.entry(id).is_some() { tb.boxes(&table, id, key, MAX_BOXES) } else { Some(vec![]) };
                let Some(drawn) = drawn else {
                    ui.ctx().request_repaint();
                    widgets::hint(ui, "…");
                    continue;
                };
                for (c_i, &c) in chain.iter().enumerate() {
                    if c_i > 0 {
                        ui.label(RichText::new(format!("goes on to {c:#06x}")).color(FAINT).size(11.5));
                    }
                    for d in drawn.iter().filter(|d| d.text_id == c) {
                        let w = ui.available_width();
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(w, d.size.y * w / d.size.x), Sense::hover());
                        ui.painter().image(d.texture, rect, d.uv, Color32::WHITE);
                    }
                    if table.entry(c).is_none() {
                        ui.label(RichText::new(format!("The game has no text {c:#06x}: it shows its first instead.")).color(WARN).size(11.5));
                    }
                    let label = if self.doc.text(c).is_some() { format!("Edit {c:#06x} · edited") } else { format!("Edit {c:#06x}") };
                    if widgets::disclosure(ui, &format!("text {c:x} {k} {i}"), &label, false) {
                        text_editor(ui, &mut self.doc, &game, c);
                    }
                }
                match drawn.last().map(|d| d.end) {
                    Some(BoxEnd::Choice(n)) => widgets::hint(ui, format!("Then a choice of {n}: its code picks the reply (the lines below).")),
                    Some(BoxEnd::Event) => widgets::hint(ui, "Then its code takes over (an event, a fade)."),
                    Some(BoxEnd::Ocarina) => widgets::hint(ui, "Then the ocarina comes up."),
                    _ => {}
                }
            }
            ui.add_space(6.0);
            if widgets::disclosure(ui, "dialogue codes", "The codes a text can use", false) {
                ui.add(egui::Label::new(RichText::new(CODES).color(MUTED).size(11.5)).wrap());
            }
        });
    }
}

/// One text's editor: its box, its text in the readable form, what's wrong with it, and back to
/// the game's.
fn text_editor(ui: &mut egui::Ui, doc: &mut overworld::doc::Doc, game: &oot_game::message::MessageTable, id: u16) {
    let game_text = game.raw(id).map(message_text::decode).unwrap_or_default();
    let game_tp = game.entry(id).map_or(0, |e| e.type_pos);
    let (mut text, mut box_type, mut box_pos) = match doc.text(id) {
        Some(t) => (t.text.clone(), t.box_type, t.box_pos),
        None => (game_text.clone(), game_tp >> 4, game_tp & 0xF),
    };
    let mut changed = false;
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt(("box type", id)).selected_text(*BOX_TYPES.get(box_type as usize).unwrap_or(&"?")).width(130.0).show_ui(ui, |ui| {
            for (k, n) in BOX_TYPES.iter().enumerate() {
                changed |= ui.selectable_value(&mut box_type, k as u8, *n).changed();
            }
        });
        egui::ComboBox::from_id_salt(("box pos", id)).selected_text(*BOX_POSITIONS.get(box_pos as usize).unwrap_or(&"?")).width(90.0).show_ui(ui, |ui| {
            for (k, n) in BOX_POSITIONS.iter().enumerate() {
                changed |= ui.selectable_value(&mut box_pos, k as u8, *n).changed();
            }
        });
    });
    let rows = text.lines().count().clamp(3, 14);
    // unwrapped, as the game breaks lines only where the text says (a long line scrolls)
    let mut unwrapped = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, _wrap: f32| {
        let job = egui::text::LayoutJob::simple(buf.as_str().to_owned(), egui::FontId::monospace(12.0), TEXT, f32::INFINITY);
        ui.fonts_mut(|f| f.layout_job(job))
    };
    egui::ScrollArea::horizontal().id_salt(("text edit", id)).show(ui, |ui| {
        let w = ui.available_width();
        changed |= ui.add(egui::TextEdit::multiline(&mut text).layouter(&mut unwrapped).desired_rows(rows).desired_width(w)).changed();
    });
    match message_text::encode(&text) {
        Err(err) => {
            ui.label(RichText::new(format!("Won't go into the game's text {err}: the level plays the game's until it's fixed.")).color(BAD).size(11.5));
        }
        Ok(bytes) => {
            for w in message_text::layout_warnings(&bytes, "Link") {
                ui.label(RichText::new(format!("Too long: {w}")).color(WARN).size(11.5));
            }
        }
    }
    let edited = doc.text(id).is_some();
    if edited {
        ui.horizontal(|ui| {
            ui.label(RichText::new("The level's own").color(ACCENT).size(11.5));
            if widgets::button(ui, None, "Back to the game's", None, false).on_hover_text(game_text.clone()).clicked() {
                doc.set_text(id, None);
            }
        });
    }
    if changed {
        let same = text == game_text && (box_type << 4 | box_pos) == game_tp;
        doc.set_text(id, (!same).then(|| Text { id, box_type, box_pos, text }));
    }
}
