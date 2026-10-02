//! Message table / text extraction, ported from the decomp's `tools/msgdis.py`.
//!
//! Layout (see `src/code/z_message.c` and `include/message_data_static.h`):
//! - `nes`/`staff` are tables of `MessageTableEntry { textId: u16, typePos: u8, pad: u8,
//!   segment_ptr: u32 }` (8 bytes each), giving id, box type/position and a segment-7
//!   pointer into that language's `*_message_data_static` file.
//! - `ger`/`fra` are plain arrays of segment-7 pointers only, one per id, in the same
//!   order as `nes`'s ids with the English-only 0xFFFC id removed.
//!
//! The four message tables themselves live at fixed VROM addresses in `msgdis.py`; since
//! this ROM is an uncompressed debug build, ROM offset == VROM, so we read them directly
//! out of `Rom::data` rather than resolving them through a named file.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use oot_import::project::Project;
use serde_json::json;

// ===================================================
//   Message table addresses (from msgdis.py)
// ===================================================

const NES_TABLE_ADDR: usize = 0x00BC24C0;
const GER_TABLE_ADDR: usize = 0x00BC66E8;
const FRA_TABLE_ADDR: usize = 0x00BC87F8;
const STAFF_TABLE_ADDR: usize = 0x00BCA908;
const STAFF_TABLE_ADDR_END: usize = 0x00BCAA90;

const ID_EMPTY: u16 = 0xFFFD; // always decodes to an empty message in every language
const ID_ENGLISH_ONLY: u16 = 0xFFFC; // conditionally-compiled, English (nes) only
const ID_END: u16 = 0xFFFF; // table terminator, not a real message

#[derive(Clone, Copy)]
struct Entry {
    id: u16,
    box_type: u8,
    box_pos: u8,
    ptr: u32,
}

fn parse_entries(bytes: &[u8]) -> Vec<Entry> {
    bytes
        .chunks_exact(8)
        .map(|c| {
            let id = u16::from_be_bytes([c[0], c[1]]);
            let type_pos = c[2];
            let ptr = u32::from_be_bytes([c[4], c[5], c[6], c[7]]);
            Entry { id, box_type: (type_pos >> 4) & 0xF, box_pos: type_pos & 0xF, ptr }
        })
        .collect()
}

fn parse_words(bytes: &[u8]) -> Vec<u32> {
    bytes.chunks_exact(4).map(|c| u32::from_be_bytes([c[0], c[1], c[2], c[3]])).collect()
}

/// Segment-7 pointer -> offset within the (already extracted) `*_message_data_static` file.
fn segmented_to_offset(address: u32) -> usize {
    (address & !0x0700_0000) as usize
}

fn box_type_name(t: u8) -> String {
    match t {
        0 => "TEXTBOX_TYPE_BLACK",
        1 => "TEXTBOX_TYPE_WOODEN",
        2 => "TEXTBOX_TYPE_BLUE",
        3 => "TEXTBOX_TYPE_OCARINA",
        4 => "TEXTBOX_TYPE_NONE_BOTTOM",
        5 => "TEXTBOX_TYPE_NONE_NO_SHADOW",
        0xB => "TEXTBOX_TYPE_CREDITS",
        _ => return format!("UNKNOWN_{t:#X}"),
    }
    .to_string()
}

fn box_pos_name(p: u8) -> String {
    match p {
        0 => "TEXTBOX_POS_VARIABLE",
        1 => "TEXTBOX_POS_TOP",
        2 => "TEXTBOX_POS_MIDDLE",
        3 => "TEXTBOX_POS_BOTTOM",
        _ => return format!("UNKNOWN_{p:#X}"),
    }
    .to_string()
}

// ===================================================
//   Decode message_data_static encoded strings
// ===================================================

/// PAL-specific characters: accented letters and button glyphs, mapped to Unicode
/// exactly as `msgdis.py`'s `extraction_charmap` does (buttons as bracketed text).
fn special_char(byte: u8) -> Option<&'static str> {
    Some(match byte {
        0x7F => "\u{203E}", // ‾
        0x80 => "À",
        0x81 => "î",
        0x82 => "Â",
        0x83 => "Ä",
        0x84 => "Ç",
        0x85 => "È",
        0x86 => "É",
        0x87 => "Ê",
        0x88 => "Ë",
        0x89 => "Ï",
        0x8A => "Ô",
        0x8B => "Ö",
        0x8C => "Ù",
        0x8D => "Û",
        0x8E => "Ü",
        0x8F => "ß",
        0x90 => "à",
        0x91 => "á",
        0x92 => "â",
        0x93 => "ä",
        0x94 => "ç",
        0x95 => "è",
        0x96 => "é",
        0x97 => "ê",
        0x98 => "ë",
        0x99 => "ï",
        0x9A => "ô",
        0x9B => "ö",
        0x9C => "ù",
        0x9D => "û",
        0x9E => "ü",
        0x9F => "[A]",
        0xA0 => "[B]",
        0xA1 => "[C]",
        0xA2 => "[L]",
        0xA3 => "[R]",
        0xA4 => "[Z]",
        0xA5 => "[C-Up]",
        0xA6 => "[C-Down]",
        0xA7 => "[C-Left]",
        0xA8 => "[C-Right]",
        0xA9 => "\u{25BC}", // ▼
        0xAA => "[Control-Pad]",
        0xAB => "[D-Pad]",
        _ => return None,
    })
}

fn color_name(byte: u8) -> String {
    match byte {
        0x40 => "DEFAULT",
        0x41 => "RED",
        0x42 => "ADJUSTABLE",
        0x43 => "BLUE",
        0x44 => "LIGHTBLUE",
        0x45 => "PURPLE",
        0x46 => "YELLOW",
        0x47 => "BLACK",
        _ => return format!("0x{byte:02X}"),
    }
    .to_string()
}

fn highscore_name(byte: u8) -> String {
    match byte {
        0x00 => "HS_HBA",
        0x01 => "HS_POE_POINTS",
        0x02 => "HS_FISHING",
        0x03 => "HS_HORSE_RACE",
        0x04 => "HS_MARATHON",
        0x06 => "HS_DAMPE_RACE",
        _ => return format!("0x{byte:02X}"),
    }
    .to_string()
}

/// Control codes that take no argument bytes: just `{NAME}` (msgdis.py's fallback case,
/// minus NEWLINE/END which are special-cased below).
fn plain_tag_name(byte: u8) -> Option<&'static str> {
    Some(match byte {
        0x08 => "QUICKTEXT_ENABLE",
        0x09 => "QUICKTEXT_DISABLE",
        0x0A => "PERSISTENT",
        0x0B => "EVENT",
        0x0D => "AWAIT_BUTTON_PRESS",
        0x0F => "NAME",
        0x10 => "OCARINA",
        0x16 => "MARATHON_TIME",
        0x17 => "RACE_TIME",
        0x18 => "POINTS",
        0x19 => "TOKENS",
        0x1A => "UNSKIPPABLE",
        0x1B => "TWO_CHOICE",
        0x1C => "THREE_CHOICE",
        0x1D => "FISH_INFO",
        0x1F => "TIME",
        _ => return None,
    })
}

/// Decodes one message's raw bytes into readable text with `{TAG}` control codes, following
/// `msgdis.py`'s `decode()`. Returns the text plus the count of unrecognised (unknown) bytes.
fn decode(bytes: &[u8]) -> (String, u32) {
    let mut out = String::new();
    let mut unknown = 0u32;
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        match b {
            0x00 => {} // padding/terminator, stripped entirely (msgdis strips \x00 post-decode)
            0x01 => out.push('\n'),                 // NEWLINE
            0x02 => {}                               // END, omitted
            0x04 => out.push_str("{BOX_BREAK}"),
            0x05 => {
                // COLOR + 1 byte
                if let Some(&v) = bytes.get(i + 1) {
                    out.push_str(&format!("{{COLOR:{}}}", color_name(v)));
                    i += 1;
                } else {
                    unknown += 1;
                }
            }
            0x06 => {
                // SHIFT + 1 byte
                if let Some(&v) = bytes.get(i + 1) {
                    out.push_str(&format!("{{SHIFT:{v}}}"));
                    i += 1;
                } else {
                    unknown += 1;
                }
            }
            0x07 => {
                // TEXTID + 2 bytes (halfword)
                if i + 2 < bytes.len() {
                    let v = u16::from_be_bytes([bytes[i + 1], bytes[i + 2]]);
                    out.push_str(&format!("{{TEXTID:0x{v:04X}}}"));
                    i += 2;
                } else {
                    unknown += 1;
                }
            }
            0x0C => {
                // BOX_BREAK_DELAYED + 1 byte
                if let Some(&v) = bytes.get(i + 1) {
                    out.push_str(&format!("{{BOX_BREAK_DELAYED:{v}}}"));
                    i += 1;
                } else {
                    unknown += 1;
                }
            }
            0x0E => {
                // FADE + 1 byte
                if let Some(&v) = bytes.get(i + 1) {
                    out.push_str(&format!("{{FADE:{v}}}"));
                    i += 1;
                } else {
                    unknown += 1;
                }
            }
            0x11 => {
                // FADE2 + 2 bytes (halfword)
                if i + 2 < bytes.len() {
                    let v = u16::from_be_bytes([bytes[i + 1], bytes[i + 2]]);
                    out.push_str(&format!("{{FADE2:0x{v:04X}}}"));
                    i += 2;
                } else {
                    unknown += 1;
                }
            }
            0x12 => {
                // SFX + 2 bytes (halfword)
                if i + 2 < bytes.len() {
                    let v = u16::from_be_bytes([bytes[i + 1], bytes[i + 2]]);
                    out.push_str(&format!("{{SFX:0x{v:04X}}}"));
                    i += 2;
                } else {
                    unknown += 1;
                }
            }
            0x13 => {
                // ITEM_ICON + 1 byte
                if let Some(&v) = bytes.get(i + 1) {
                    out.push_str(&format!("{{ITEM_ICON:{v}}}"));
                    i += 1;
                } else {
                    unknown += 1;
                }
            }
            0x14 => {
                // TEXT_SPEED + 1 byte
                if let Some(&v) = bytes.get(i + 1) {
                    out.push_str(&format!("{{TEXT_SPEED:{v}}}"));
                    i += 1;
                } else {
                    unknown += 1;
                }
            }
            0x15 => {
                // BACKGROUND + 3 bytes
                if i + 3 < bytes.len() {
                    let (a, b2, c) = (bytes[i + 1], bytes[i + 2], bytes[i + 3]);
                    out.push_str(&format!("{{BACKGROUND:0x{a:02X},0x{b2:02X},0x{c:02X}}}"));
                    i += 3;
                } else {
                    unknown += 1;
                }
            }
            0x1E => {
                // HIGHSCORE + 1 byte
                if let Some(&v) = bytes.get(i + 1) {
                    out.push_str(&format!("{{HIGHSCORE:{}}}", highscore_name(v)));
                    i += 1;
                } else {
                    unknown += 1;
                }
            }
            _ => {
                if let Some(name) = plain_tag_name(b) {
                    out.push('{');
                    out.push_str(name);
                    out.push('}');
                } else if let Some(s) = special_char(b) {
                    out.push_str(s);
                } else if b.is_ascii() {
                    if b == b'"' {
                        out.push_str("\\\"");
                    } else {
                        out.push(b as char);
                    }
                } else {
                    out.push_str(&format!("{{UNKNOWN:0x{b:02X}}}"));
                    unknown += 1;
                }
            }
        }
        i += 1;
    }
    (out, unknown)
}

// ===================================================
//   Message tables
// ===================================================

struct Combined {
    id: u16,
    box_type: u8,
    box_pos: u8,
    nes_ptr: u32,
    ger_ptr: Option<u32>,
    fra_ptr: Option<u32>,
}

fn read_combined_table(rom_data: &[u8]) -> Result<Vec<Combined>> {
    let nes_bytes = rom_data.get(NES_TABLE_ADDR..GER_TABLE_ADDR).context("nes message table out of ROM bounds")?;
    let ger_bytes = rom_data.get(GER_TABLE_ADDR..FRA_TABLE_ADDR).context("ger message table out of ROM bounds")?;
    let fra_bytes = rom_data.get(FRA_TABLE_ADDR..STAFF_TABLE_ADDR).context("fra message table out of ROM bounds")?;

    let nes = parse_entries(nes_bytes);
    let ids: Vec<u16> = nes.iter().map(|e| e.id).filter(|&id| id != ID_ENGLISH_ONLY).collect();
    let ger_ptrs = parse_words(ger_bytes);
    let fra_ptrs = parse_words(fra_bytes);
    anyhow::ensure!(ids.len() == ger_ptrs.len(), "ger message table entry count ({}) != nes ids ({})", ger_ptrs.len(), ids.len());
    anyhow::ensure!(ids.len() == fra_ptrs.len(), "fra message table entry count ({}) != nes ids ({})", fra_ptrs.len(), ids.len());
    let ger_by_id: BTreeMap<u16, u32> = ids.iter().copied().zip(ger_ptrs.iter().copied()).collect();
    let fra_by_id: BTreeMap<u16, u32> = ids.iter().copied().zip(fra_ptrs.iter().copied()).collect();

    let mut combined = Vec::with_capacity(nes.len());
    for e in &nes {
        if e.id == ID_ENGLISH_ONLY {
            combined.push(Combined { id: e.id, box_type: e.box_type, box_pos: e.box_pos, nes_ptr: e.ptr, ger_ptr: None, fra_ptr: None });
        } else {
            combined.push(Combined {
                id: e.id,
                box_type: e.box_type,
                box_pos: e.box_pos,
                nes_ptr: e.ptr,
                ger_ptr: ger_by_id.get(&e.id).copied(),
                fra_ptr: fra_by_id.get(&e.id).copied(),
            });
        }
    }
    Ok(combined)
}

fn read_staff_table(rom_data: &[u8]) -> Result<Vec<Entry>> {
    let bytes = rom_data.get(STAFF_TABLE_ADDR..STAFF_TABLE_ADDR_END).context("staff message table out of ROM bounds")?;
    Ok(parse_entries(bytes))
}

#[derive(serde::Serialize)]
struct Message {
    id: String,
    box_type: String,
    box_position: String,
    text: String,
}

/// One decoded language's set of messages, plus how many unknown control-code bytes were
/// hit while decoding it.
struct LangResult {
    messages: Vec<Message>,
    unknown: u32,
}

fn decode_bytes_at(data: &[u8], offset: usize, len: usize, unknown: &mut u32) -> String {
    let Some(slice) = data.get(offset..offset + len) else {
        log::warn!("text: message range 0x{offset:X}..0x{:X} out of bounds ({} bytes)", offset + len, data.len());
        return String::new();
    };
    let (text, u) = decode(slice);
    *unknown += u;
    text
}

fn extract_main_languages(combined: &[Combined], nes_data: &[u8], ger_data: &[u8], fra_data: &[u8]) -> (LangResult, LangResult, LangResult) {
    let mut nes = LangResult { messages: Vec::new(), unknown: 0 };
    let mut ger = LangResult { messages: Vec::new(), unknown: 0 };
    let mut fra = LangResult { messages: Vec::new(), unknown: 0 };

    for (i, entry) in combined.iter().enumerate() {
        if entry.id == ID_END {
            continue; // table terminator, not a real message
        }
        let id = format!("0x{:04X}", entry.id);
        let box_type = box_type_name(entry.box_type);
        let box_position = box_pos_name(entry.box_pos);

        if entry.id == ID_EMPTY {
            nes.messages.push(Message { id: id.clone(), box_type: box_type.clone(), box_position: box_position.clone(), text: String::new() });
            ger.messages.push(Message { id: id.clone(), box_type: box_type.clone(), box_position: box_position.clone(), text: String::new() });
            fra.messages.push(Message { id, box_type, box_position, text: String::new() });
            continue;
        }

        let next = &combined[i + 1];
        let nes_off = segmented_to_offset(entry.nes_ptr);
        let nes_len = (next.nes_ptr - entry.nes_ptr) as usize;
        let nes_text = decode_bytes_at(nes_data, nes_off, nes_len, &mut nes.unknown);
        nes.messages.push(Message { id: id.clone(), box_type: box_type.clone(), box_position: box_position.clone(), text: nes_text });

        if entry.id == ID_ENGLISH_ONLY {
            continue; // no ger/fra counterpart
        }

        // ger/fra pointer tables have no entry for the English-only id, so when the next
        // combined entry is that id, the length must be measured against the one after it.
        let next_lang = if next.id == ID_ENGLISH_ONLY { &combined[i + 2] } else { next };

        let ger_ptr = entry.ger_ptr.expect("non-FFFC entry has a ger pointer");
        let ger_off = segmented_to_offset(ger_ptr);
        let ger_len = (next_lang.ger_ptr.expect("next entry has a ger pointer") - ger_ptr) as usize;
        let ger_text = decode_bytes_at(ger_data, ger_off, ger_len, &mut ger.unknown);
        ger.messages.push(Message { id: id.clone(), box_type: box_type.clone(), box_position: box_position.clone(), text: ger_text });

        let fra_ptr = entry.fra_ptr.expect("non-FFFC entry has a fra pointer");
        let fra_off = segmented_to_offset(fra_ptr);
        let fra_len = (next_lang.fra_ptr.expect("next entry has a fra pointer") - fra_ptr) as usize;
        let fra_text = decode_bytes_at(fra_data, fra_off, fra_len, &mut fra.unknown);
        fra.messages.push(Message { id, box_type, box_position, text: fra_text });
    }

    (nes, ger, fra)
}

fn extract_staff(staff_entries: &[Entry], staff_data: &[u8]) -> LangResult {
    let mut r = LangResult { messages: Vec::new(), unknown: 0 };
    for (i, entry) in staff_entries.iter().enumerate() {
        if entry.id == ID_END {
            continue;
        }
        let off = segmented_to_offset(entry.ptr);
        // The last real entry's length can't be measured against the terminator (it doesn't
        // reliably point past the final message), so msgdis.py falls back to the file's size
        // for the one known-last id.
        let len = if entry.id == 0x052F {
            staff_data.len().saturating_sub(off)
        } else {
            let next = &staff_entries[i + 1];
            segmented_to_offset(next.ptr).saturating_sub(off)
        };
        let text = decode_bytes_at(staff_data, off, len, &mut r.unknown);
        r.messages.push(Message {
            id: format!("0x{:04X}", entry.id),
            box_type: box_type_name(entry.box_type),
            box_position: box_pos_name(entry.box_pos),
            text,
        });
    }
    r
}

fn write_lang(out: &Path, name: &str, r: &LangResult) -> Result<()> {
    let dir = out.join("text");
    std::fs::create_dir_all(&dir)?;
    crate::write_json(&dir.join(format!("{name}.json")), &r.messages)?;

    let mut txt = String::new();
    for m in &r.messages {
        txt.push_str(&format!("=== {} ({}, {}) ===\n", m.id, m.box_type, m.box_position));
        txt.push_str(&m.text);
        if !m.text.ends_with('\n') {
            txt.push('\n');
        }
        txt.push('\n');
    }
    std::fs::write(dir.join(format!("{name}.txt")), txt)?;
    Ok(())
}

pub fn extract(p: &Project, out: &Path) -> Result<serde_json::Value> {
    let combined = read_combined_table(&p.rom.data)?;
    let staff_entries = read_staff_table(&p.rom.data)?;

    let nes_data = p.rom.file_by_name("nes_message_data_static").context("nes_message_data_static")?;
    let ger_data = p.rom.file_by_name("ger_message_data_static").context("ger_message_data_static")?;
    let fra_data = p.rom.file_by_name("fra_message_data_static").context("fra_message_data_static")?;
    let staff_data = p.rom.file_by_name("staff_message_data_static").context("staff_message_data_static")?;

    let (nes, ger, fra) = extract_main_languages(&combined, &nes_data, &ger_data, &fra_data);
    let staff = extract_staff(&staff_entries, &staff_data);

    write_lang(out, "nes", &nes)?;
    write_lang(out, "ger", &ger)?;
    write_lang(out, "fra", &fra)?;
    write_lang(out, "staff", &staff)?;

    let total_unknown = nes.unknown + ger.unknown + fra.unknown + staff.unknown;
    if total_unknown > 0 {
        log::warn!("text: {total_unknown} unknown control-code bytes encountered");
    }

    Ok(json!({
        "nes": nes.messages.len(),
        "ger": ger.messages.len(),
        "fra": fra.messages.len(),
        "staff": staff.messages.len(),
        "unknown_control_codes": total_unknown,
    }))
}
