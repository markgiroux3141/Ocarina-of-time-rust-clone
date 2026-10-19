//! A message's bytes as text one can read and edit, and back: the game editor's texts
//! (docs/EDITOR-ACTORS.md, "Texts: what an actor says"; docs/adr/0079-the-level-texts.md).
//!
//! **The readable form** is the decomp's charmap (`assets/text/charmap.nes.txt`: `[A]`,
//! `[C-Up]`, `▼`, `é`) with the control codes as tags in braces, as the tools' port of
//! `msgdis.py` prints them (`crates/tools/oot_extract/src/text.rs`):
//!
//! - a newline is a newline (`MESSAGE_NEWLINE`); the end (`MESSAGE_END`) is the text's end;
//! - `{BOX_BREAK}`, `{NAME}`, `{TWO_CHOICE}`, `{THREE_CHOICE}`, `{QUICKTEXT_ENABLE}`, `{EVENT}`
//!   and the other codes without arguments by their `message_data_fmt.h` names;
//! - with arguments: `{COLOR:RED}` (the `MSGCOL_*` names, or a byte), `{TEXTID:0x1036}`,
//!   `{SHIFT:12}`, `{BOX_BREAK_DELAYED:20}`, `{FADE:40}`, `{FADE2:0x0028}`, `{SFX:0x4807}`,
//!   `{ITEM_ICON:0x2D}`, `{TEXT_SPEED:2}`, `{BACKGROUND:0x00,0x10,0x20}`, `{HIGHSCORE:HS_FISHING}`;
//! - any other byte as `{0xNN}` (and `[`, `{` and `}` themselves, which would read as the start
//!   of a button or a tag).
//!
//! `encode` is `decode`'s inverse: every message of the game's table comes back byte for byte
//! (`tests::every_message_round_trips`), with its padding after the end to four bytes, as the
//! table lays them out.

use crate::message::*;

/// `HS_*` (`save.h`): the high scores `MESSAGE_HIGHSCORE` reads.
const HIGHSCORES: [(u8, &str); 6] = [(0x00, "HS_HBA"), (0x01, "HS_POE_POINTS"), (0x02, "HS_FISHING"), (0x03, "HS_HORSE_RACE"), (0x04, "HS_MARATHON"), (0x06, "HS_DAMPE_RACE")];

/// `MSGCOL_*` (`message_data_fmt.h`): the colour byte is 0x40 and the colour.
const COLORS: [&str; 8] = ["DEFAULT", "RED", "ADJUSTABLE", "BLUE", "LIGHTBLUE", "PURPLE", "YELLOW", "BLACK"];

/// The codes without arguments, by name.
const PLAIN: [(u8, &str); 19] = [
    (MESSAGE_BOX_BREAK, "BOX_BREAK"),
    (MESSAGE_QUICKTEXT_ENABLE, "QUICKTEXT_ENABLE"),
    (MESSAGE_QUICKTEXT_DISABLE, "QUICKTEXT_DISABLE"),
    (MESSAGE_PERSISTENT, "PERSISTENT"),
    (MESSAGE_EVENT, "EVENT"),
    (MESSAGE_AWAIT_BUTTON_PRESS, "AWAIT_BUTTON_PRESS"),
    (MESSAGE_NAME, "NAME"),
    (MESSAGE_OCARINA, "OCARINA"),
    (MESSAGE_MARATHON_TIME, "MARATHON_TIME"),
    (MESSAGE_RACE_TIME, "RACE_TIME"),
    (MESSAGE_POINTS, "POINTS"),
    (MESSAGE_TOKENS, "TOKENS"),
    (MESSAGE_UNSKIPPABLE, "UNSKIPPABLE"),
    (MESSAGE_TWO_CHOICE, "TWO_CHOICE"),
    (MESSAGE_THREE_CHOICE, "THREE_CHOICE"),
    (MESSAGE_FISH_INFO, "FISH_INFO"),
    (MESSAGE_TIME, "TIME"),
    // Not in message_data_fmt.h's list: the end, written only where bytes follow it.
    (MESSAGE_END, "END"),
    (0x00, "0x00"),
];

/// The codes with arguments: their name and how many argument bytes.
const WITH_ARGS: [(u8, &str, usize); 11] = [
    (MESSAGE_COLOR, "COLOR", 1),
    (MESSAGE_SHIFT, "SHIFT", 1),
    (MESSAGE_TEXTID, "TEXTID", 2),
    (MESSAGE_BOX_BREAK_DELAYED, "BOX_BREAK_DELAYED", 1),
    (MESSAGE_FADE, "FADE", 1),
    (MESSAGE_FADE2, "FADE2", 2),
    (MESSAGE_SFX, "SFX", 2),
    (MESSAGE_ITEM_ICON, "ITEM_ICON", 1),
    (MESSAGE_TEXT_SPEED, "TEXT_SPEED", 1),
    (MESSAGE_BACKGROUND, "BACKGROUND", 3),
    (MESSAGE_HIGHSCORE, "HIGHSCORE", 1),
];

/// The font's characters past ASCII (`charmap.nes.txt`), 0x7F on.
const SPECIAL: [(u8, &str); 45] = [
    (0x7F, "\u{203E}"),
    (0x80, "À"),
    (0x81, "î"),
    (0x82, "Â"),
    (0x83, "Ä"),
    (0x84, "Ç"),
    (0x85, "È"),
    (0x86, "É"),
    (0x87, "Ê"),
    (0x88, "Ë"),
    (0x89, "Ï"),
    (0x8A, "Ô"),
    (0x8B, "Ö"),
    (0x8C, "Ù"),
    (0x8D, "Û"),
    (0x8E, "Ü"),
    (0x8F, "ß"),
    (0x90, "à"),
    (0x91, "á"),
    (0x92, "â"),
    (0x93, "ä"),
    (0x94, "ç"),
    (0x95, "è"),
    (0x96, "é"),
    (0x97, "ê"),
    (0x98, "ë"),
    (0x99, "ï"),
    (0x9A, "ô"),
    (0x9B, "ö"),
    (0x9C, "ù"),
    (0x9D, "û"),
    (0x9E, "ü"),
    (0x9F, "[A]"),
    (0xA0, "[B]"),
    (0xA1, "[C]"),
    (0xA2, "[L]"),
    (0xA3, "[R]"),
    (0xA4, "[Z]"),
    (0xA5, "[C-Up]"),
    (0xA6, "[C-Down]"),
    (0xA7, "[C-Left]"),
    (0xA8, "[C-Right]"),
    (0xA9, "▼"),
    (0xAA, "[Control-Pad]"),
    (0xAB, "[D-Pad]"),
];

/// The game's message bytes as readable text: up to its end, or the whole of `bytes` if it has
/// none. Bytes after the end other than the zero padding to four bytes are kept as tags.
pub fn decode(bytes: &[u8]) -> String {
    let mut out = String::new();
    let end = message_end(bytes);
    let mut i = 0;
    while i < end {
        let b = bytes[i];
        if let Some(&(_, name, n)) = WITH_ARGS.iter().find(|w| w.0 == b) {
            let args = &bytes[(i + 1).min(end)..(i + 1 + n).min(end)];
            if args.len() < n {
                // cut short by the end: the code's byte alone
                out.push_str(&format!("{{0x{b:02X}}}"));
                i += 1;
                continue;
            }
            out.push('{');
            out.push_str(name);
            out.push(':');
            out.push_str(&arg_text(b, args));
            out.push('}');
            i += 1 + n;
            continue;
        }
        match b {
            MESSAGE_NEWLINE => out.push('\n'),
            b'[' | b'{' | b'}' => out.push_str(&format!("{{0x{b:02X}}}")),
            0x20..=0x7E => out.push(b as char),
            _ => match (SPECIAL.iter().find(|s| s.0 == b), PLAIN.iter().find(|p| p.0 == b)) {
                (Some((_, s)), _) => out.push_str(s),
                (None, Some((_, name))) if b != 0x00 && b != MESSAGE_END => out.push_str(&format!("{{{name}}}")),
                _ => out.push_str(&format!("{{0x{b:02X}}}")),
            },
        }
        i += 1;
    }
    // what follows the end, past the padding the table puts there
    let tail = &bytes[end.min(bytes.len())..];
    if !tail.is_empty() && encode_tail(end) != tail {
        out.push_str("{END}");
        for &b in tail.iter().skip(1) {
            out.push_str(&format!("{{0x{b:02X}}}"));
        }
    }
    out
}

/// Where the message's end code is (the length of its text), skipping over the arguments of the
/// codes that take some, as `Message_Decode` reads them.
fn message_end(bytes: &[u8]) -> usize {
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == MESSAGE_END {
            return i;
        }
        i += 1 + WITH_ARGS.iter().find(|w| w.0 == b).map_or(0, |w| w.2);
    }
    bytes.len()
}

/// The end code and the zero padding after a text `len` bytes long, to four bytes.
fn encode_tail(len: usize) -> Vec<u8> {
    let mut t = vec![MESSAGE_END];
    while (len + t.len()) % 4 != 0 {
        t.push(0);
    }
    t
}

fn arg_text(code: u8, a: &[u8]) -> String {
    let half = || u16::from_be_bytes([a[0], a[1]]);
    match code {
        MESSAGE_COLOR => match a[0].checked_sub(0x40).and_then(|c| COLORS.get(c as usize)) {
            Some(n) => n.to_string(),
            None => format!("0x{:02X}", a[0]),
        },
        MESSAGE_TEXTID | MESSAGE_FADE2 | MESSAGE_SFX => format!("0x{:04X}", half()),
        MESSAGE_ITEM_ICON => format!("0x{:02X}", a[0]),
        MESSAGE_BACKGROUND => format!("0x{:02X},0x{:02X},0x{:02X}", a[0], a[1], a[2]),
        MESSAGE_HIGHSCORE => match HIGHSCORES.iter().find(|h| h.0 == a[0]) {
            Some((_, n)) => n.to_string(),
            None => format!("0x{:02X}", a[0]),
        },
        _ => a[0].to_string(),
    }
}

/// Why a text won't encode, and where (a char index into it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodeError {
    pub at: usize,
    pub what: String,
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "at {}: {}", self.at, self.what)
    }
}

fn number(s: &str) -> Option<u32> {
    let s = s.trim();
    match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Some(h) => u32::from_str_radix(h, 16).ok(),
        None => s.parse().ok(),
    }
}

/// Readable text back to the game's bytes: the text, its end code and the zero padding to four
/// bytes. `\r` is dropped (a newline is `\n`).
pub fn encode(text: &str) -> Result<Vec<u8>, EncodeError> {
    let chars: Vec<char> = text.chars().filter(|&c| c != '\r').collect();
    let mut out = Vec::new();
    let mut i = 0;
    let mut explicit_end = false;
    while i < chars.len() {
        let c = chars[i];
        if c == '{' {
            let Some(len) = chars[i..].iter().position(|&c| c == '}') else {
                return Err(EncodeError { at: i, what: "a `{` with no `}`".into() });
            };
            let tag: String = chars[i + 1..i + len].iter().collect();
            let err = |what: String| EncodeError { at: i, what };
            let (name, arg) = match tag.split_once(':') {
                Some((n, a)) => (n.trim(), Some(a.trim())),
                None => (tag.trim(), None),
            };
            if let (Some(b), None) = (number(name).filter(|_| name.starts_with("0x")), arg) {
                out.push(u8::try_from(b).map_err(|_| err(format!("{{{tag}}}: a byte is 0x00 to 0xFF")))?);
                explicit_end |= b as u8 == MESSAGE_END;
            } else if let Some(&(code, _)) = PLAIN.iter().find(|p| p.1 == name) {
                if arg.is_some() {
                    return Err(err(format!("{{{name}}} takes nothing")));
                }
                out.push(code);
                explicit_end |= code == MESSAGE_END;
            } else if let Some(&(code, _, n)) = WITH_ARGS.iter().find(|w| w.1 == name) {
                let a = arg.ok_or_else(|| err(format!("{{{name}:...}} needs a value")))?;
                out.push(code);
                out.extend(encode_arg(code, n, a).map_err(|m| err(format!("{{{name}:{a}}}: {m}")))?);
            } else {
                return Err(err(format!("no code {{{name}}}")));
            }
            i += len + 1;
            continue;
        }
        if c == '\n' {
            out.push(MESSAGE_NEWLINE);
            i += 1;
            continue;
        }
        if let Some((b, n)) = SPECIAL.iter().find_map(|(b, s)| {
            let s: Vec<char> = s.chars().collect();
            chars[i..].starts_with(&s).then_some((*b, s.len()))
        }) {
            out.push(b);
            i += n;
            continue;
        }
        if (' '..='~').contains(&c) && c != '}' {
            out.push(c as u8);
            i += 1;
            continue;
        }
        return Err(EncodeError { at: i, what: format!("{c:?} isn't in the game's font") });
    }
    if !explicit_end {
        let tail = encode_tail(out.len());
        out.extend(tail);
    }
    Ok(out)
}

fn encode_arg(code: u8, n: usize, a: &str) -> Result<Vec<u8>, String> {
    let byte = |s: &str| number(s).and_then(|v| u8::try_from(v).ok()).ok_or_else(|| format!("{s:?} isn't a byte"));
    match code {
        MESSAGE_COLOR => match COLORS.iter().position(|c| *c == a) {
            Some(k) => Ok(vec![0x40 + k as u8]),
            None => byte(a).map(|b| vec![b]).map_err(|_| format!("a colour is one of {}", COLORS.join(", "))),
        },
        MESSAGE_HIGHSCORE => match HIGHSCORES.iter().find(|h| h.1 == a) {
            Some((b, _)) => Ok(vec![*b]),
            None => byte(a).map(|b| vec![b]),
        },
        MESSAGE_BACKGROUND => {
            let parts: Vec<&str> = a.split(',').collect();
            if parts.len() != 3 {
                return Err("three bytes".into());
            }
            parts.iter().map(|p| byte(p)).collect()
        }
        _ if n == 2 => number(a).and_then(|v| u16::try_from(v).ok()).map(|v| v.to_be_bytes().to_vec()).ok_or_else(|| format!("{a:?} isn't 0 to 0xFFFF")),
        _ => byte(a).map(|b| vec![b]),
    }
}

/// One box of a message as the game types it, measured: its lines' widths (in screen pixels, as
/// `Message_DrawText` advances) and what ends it.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxMeasure {
    pub lines: Vec<f32>,
    /// The width of the widest line (the start of the first, after an item icon or a choice's
    /// indent, included).
    pub widest: f32,
}

/// `R_TEXT_INIT_XPOS`, the text's left edge (`Message_OpenText`), and the box's right edge
/// (`R_TEXTBOX_X` 34 plus its width 256): the game's own lines reach it (an item's text, indented
/// past its icon), never past.
pub const TEXT_LEFT: f32 = 65.0;
pub const TEXT_RIGHT: f32 = 34.0 + 256.0;
/// How many lines a box shows: `Message_Decode` starts the text 8 below the box's top for four
/// lines, and the box (64 high) has no room for a fifth at 12 apart.
pub const BOX_LINES: usize = 4;

/// The boxes `bytes` types (split at its box breaks), each line's width as the message box draws
/// it at its scale (`R_TEXT_CHAR_SCALE` 75): the font's widths, spaces 6, `{SHIFT}`, 32 for an
/// item icon or a background image, and the player's name as `name`'s characters. Lines are
/// indented by 32 as `Message_DrawText`'s newline does it: every line after the first in a box
/// with an item icon or three choices, every line after the second with two (`choiceNum`, which
/// `Message_Decode` sets as it meets the code and nothing resets until the next message).
pub fn measure(bytes: &[u8], name: &str) -> Vec<BoxMeasure> {
    let end = message_end(bytes);
    let args = |b: u8| WITH_ARGS.iter().find(|w| w.0 == b).map_or(0, |w| w.2);
    // the choiceNum a box's decode leaves (the last such code in it), or the one before
    let choice_after = |from: usize, before: u8| {
        let (mut j, mut c) = (from, before);
        while j < end && !matches!(bytes[j], MESSAGE_BOX_BREAK | MESSAGE_BOX_BREAK_DELAYED) {
            match bytes[j] {
                MESSAGE_ITEM_ICON => c = 1,
                MESSAGE_TWO_CHOICE => c = 2,
                MESSAGE_THREE_CHOICE => c = 3,
                _ => {}
            }
            j += 1 + args(bytes[j]);
        }
        c
    };
    let mut boxes = vec![];
    let mut lines = vec![0f32];
    let mut choice = choice_after(0, 0);
    let mut i = 0;
    // the pen's x on the line, and how far its ink reaches (trailing spaces draw nothing)
    let mut pen = 0f32;
    let advance = |lines: &mut Vec<f32>, pen: &mut f32, c: u8| {
        if c == b' ' {
            *pen += MESSAGE_SPACE_WIDTH as f32;
        } else {
            *pen += (char_width(c) * 0.75) as i32 as f32;
            *lines.last_mut().unwrap() = *pen;
        }
    };
    let close = |lines: &mut Vec<f32>, boxes: &mut Vec<BoxMeasure>| {
        let widest = lines.iter().cloned().fold(0.0, f32::max);
        boxes.push(BoxMeasure { lines: std::mem::replace(lines, vec![0.0]), widest });
    };
    while i < end {
        let b = bytes[i];
        let n = args(b);
        match b {
            MESSAGE_NEWLINE => {
                let indent = choice == 1 || choice == 3 || (choice == 2 && lines.len() >= 2);
                pen = if indent { 32.0 } else { 0.0 };
                lines.push(0.0);
            }
            MESSAGE_BOX_BREAK | MESSAGE_BOX_BREAK_DELAYED => {
                close(&mut lines, &mut boxes);
                pen = 0.0;
                choice = choice_after(i + 1 + n, choice);
            }
            MESSAGE_SHIFT => pen += bytes.get(i + 1).copied().unwrap_or(0) as f32,
            MESSAGE_ITEM_ICON | MESSAGE_BACKGROUND => pen += 32.0,
            MESSAGE_NAME => name.bytes().for_each(|c| advance(&mut lines, &mut pen, c)),
            MESSAGE_TIME => b"00:00".iter().for_each(|&c| advance(&mut lines, &mut pen, c)),
            MESSAGE_MARATHON_TIME | MESSAGE_RACE_TIME => b"00\"00\"".iter().for_each(|&c| advance(&mut lines, &mut pen, c)),
            MESSAGE_POINTS | MESSAGE_TOKENS | MESSAGE_FISH_INFO | MESSAGE_HIGHSCORE => b"000".iter().for_each(|&c| advance(&mut lines, &mut pen, c)),
            0x20..=0xAB => advance(&mut lines, &mut pen, b),
            _ => {}
        }
        i += 1 + n;
    }
    close(&mut lines, &mut boxes);
    boxes
}

/// What's wrong with a message's layout for its box type: a box with more lines than it shows, a
/// line past the box's right edge. Empty if it fits. (A box without a frame, `NONE_BOTTOM` and
/// `NONE_NO_SHADOW`, has the same text area.)
pub fn layout_warnings(bytes: &[u8], name: &str) -> Vec<String> {
    let mut out = vec![];
    let boxes = measure(bytes, name);
    let many = boxes.len() > 1;
    for (k, b) in boxes.iter().enumerate() {
        let which = if many { format!("box {}: ", k + 1) } else { String::new() };
        if b.lines.len() > BOX_LINES {
            out.push(format!("{which}{} lines; the box shows {BOX_LINES}", b.lines.len()));
        }
        for (l, w) in b.lines.iter().enumerate() {
            if TEXT_LEFT + w > TEXT_RIGHT {
                out.push(format!("{which}line {} is {w:.0} wide; the box has room for {:.0}", l + 1, TEXT_RIGHT - TEXT_LEFT));
            }
        }
    }
    out
}

/// The text ids a message goes on to by itself (`{TEXTID:...}`: the box continues with that one).
pub fn next_text_id(bytes: &[u8]) -> Option<u16> {
    let end = message_end(bytes);
    let mut i = 0;
    while i < end {
        let b = bytes[i];
        if b == MESSAGE_TEXTID && i + 2 < bytes.len() {
            return Some(u16::from_be_bytes([bytes[i + 1], bytes[i + 2]]));
        }
        i += 1 + WITH_ARGS.iter().find(|w| w.0 == b).map_or(0, |w| w.2);
    }
    None
}

/// A text a level gives an id: its box (`typePos`: the type in the high four bits, the
/// position in the low) and its bytes as `encode` makes them.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TextOverride {
    pub text_id: u16,
    pub type_pos: u8,
    pub bytes: Vec<u8>,
}

/// A scene's own texts over the game's (`keys::scene_texts`): what `Play_Init` plays the scene
/// with (`GameAssets::messages_for`).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SceneTexts {
    pub texts: Vec<TextOverride>,
}

/// The first id a level may give a text of its own: past the game's last (0x7xxx in the
/// English table) and below the table's special ids (0xFFFC on).
pub const FIRST_NEW_TEXT_ID: u16 = 0x8000;
/// The last: the table's special ids start after it.
pub const LAST_NEW_TEXT_ID: u16 = 0xFFFB;

impl MessageTable {
    /// The table with `texts` in place of the messages of their ids, and the new ids added before
    /// the end (`0xFFFC`, `0xFFFD` and the terminator stay last, as `Font_LoadOrderedFont` and
    /// the message debugger need). Each message is laid out again in order, so
    /// `Message_FindMessagePAL`'s length (to the next entry) is each one's own.
    pub fn layered(&self, texts: &[TextOverride]) -> MessageTable {
        let Some(first) = self.entries.first() else { return self.clone() };
        let seg = first.segment;
        let over = |id: u16| texts.iter().rev().find(|t| t.text_id == id);
        let mut entries: Vec<(u16, u8, Vec<u8>)> = vec![];
        let n = self.entries.len();
        for (k, e) in self.entries.iter().enumerate() {
            if e.text_id == 0xFFFF || k + 1 == n {
                break;
            }
            match over(e.text_id) {
                Some(t) => entries.push((e.text_id, t.type_pos, t.bytes.clone())),
                None => entries.push((e.text_id, e.type_pos, self.raw(e.text_id).map(<[u8]>::to_vec).unwrap_or_default())),
            }
        }
        let special = entries.iter().position(|e| e.0 >= 0xFFFC).unwrap_or(entries.len());
        let mut new: Vec<&TextOverride> = texts.iter().filter(|t| self.entry(t.text_id).is_none()).collect();
        new.sort_by_key(|t| t.text_id);
        new.dedup_by_key(|t| t.text_id);
        let tail = entries.split_off(special);
        entries.extend(new.iter().map(|t| (t.text_id, t.type_pos, t.bytes.clone())));
        entries.extend(tail);
        let mut out = MessageTable { entries: vec![], data: vec![] };
        for (id, tp, bytes) in entries {
            out.entries.push(MessageTableEntry { text_id: id, type_pos: tp, segment: seg + out.data.len() as u32 });
            out.data.extend(bytes);
        }
        // the terminator: where the data ends
        out.entries.push(MessageTableEntry { text_id: 0xFFFF, type_pos: 0, segment: seg + out.data.len() as u32 });
        out
    }
}

/// How a box of a message ends, as the game waits at it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxEnd {
    /// A box break: the triangle, A for the next box.
    Next,
    /// A box break after a pause (`{BOX_BREAK_DELAYED}`): no button.
    Delayed,
    /// The message goes on to another (`{TEXTID}`): the triangle, A for it.
    Continues(u16),
    /// Two or three choices, the arrow at the first.
    Choice(u8),
    /// The end: the square, A to close.
    End,
    /// The actor takes over (`{EVENT}`, `{PERSISTENT}`), or it fades by itself (`{FADE}`).
    Event,
    /// The ocarina comes up (`{OCARINA}`).
    Ocarina,
}

/// One box as the message box draws it once it's typed out: its sprites (the box, the glyphs and
/// their shadows, the icons, as `Message_DrawMain` submits them to the 320x240 screen), where the
/// box is (`R_TEXTBOX_X`, `_Y`, `_WIDTH`, `_HEIGHT`), and how it ends.
#[derive(Debug, Clone, PartialEq)]
pub struct PreviewBox {
    pub text_id: u16,
    pub box_type: u8,
    pub rect: [i16; 4],
    pub sprites: Vec<crate::sprite::Sprite>,
    pub end: BoxEnd,
}

/// The file name's codes for `name` (`FILENAME_*`: digits, upper case from 0x0A, lower case from
/// 0x24, a space 0x3E), padded with spaces to eight.
pub fn file_name(name: &str) -> [u8; 8] {
    let mut out = [0x3E; 8];
    for (k, c) in name.bytes().take(8).enumerate() {
        out[k] = match c {
            b'0'..=b'9' => c - b'0',
            b'A'..=b'Z' => c - b'7',
            b'a'..=b'z' => c - b'=',
            b'-' => 0x3F,
            b'.' => 0x40,
            _ => 0x3E,
        };
    }
    out
}

/// What the game's message box shows for `text_id` in `table`, box by box: the real
/// `MessageContext` started on it (`Message_StartTextbox`, with no actor: a variable box goes to
/// the bottom) and run a frame at a time, A pressed at each box's wait and at a `{TEXTID}`, up to
/// `max_boxes` or the box that ends it (the end, a choice, an event, the ocarina). The player's
/// name is `name`.
pub fn preview(table: &MessageTable, text_id: u16, name: &str, max_boxes: usize) -> Vec<PreviewBox> {
    use eng_input::pad::{BTN_A, PadState};
    let mut msg = MessageContext::new();
    let mut audio = crate::audio::GameAudio::default();
    let mut save = crate::save::SaveContext::default();
    save.player_name = file_name(name);
    let mut iface = crate::interface::InterfaceContext::default();
    let idle = crate::play::scripted_input(PadState::default(), PadState::default());
    let press = crate::play::scripted_input(PadState::default(), PadState { button: BTN_A, ..Default::default() });
    let mut boxes = vec![];
    let mut settle = 0;
    let mut pressing = false;
    macro_rules! frame {
        ($input:expr) => {
            MsgFrame {
                table,
                input: $input,
                audio: &mut audio,
                save: &mut save,
                iface: &mut iface,
                scene_cam_type: 0,
                scene_id: 0,
                player_screen_y: 0,
                talk_actor_screen_y: None,
                cs_idle: true,
                active_cam_main: true,
                player_pos: glam::Vec3::ZERO,
                requests: vec![],
            }
        };
    }
    msg.start_textbox(&mut frame!(&idle), text_id, None);
    for _ in 0..6000 {
        let input = if pressing { &press } else { &idle };
        pressing = false;
        msg.update(&mut frame!(input));
        msg.draw_update(&mut frame!(&idle));
        if msg.msg_length == 0 || msg.msg_mode == MSGMODE_NONE {
            break;
        }
        let ocarina = (MSGMODE_OCARINA_STARTING..MSGMODE_TEXT_AWAIT_NEXT).contains(&msg.msg_mode);
        let end = match msg.msg_mode {
            MSGMODE_TEXT_AWAIT_NEXT => Some(BoxEnd::Next),
            MSGMODE_TEXT_DELAYED_BREAK => Some(BoxEnd::Delayed),
            MSGMODE_TEXT_DONE => Some(match msg.textbox_end_type {
                TEXTBOX_ENDTYPE_HAS_NEXT => BoxEnd::Continues(next_text_id(&msg.font.msg_buf).unwrap_or(0)),
                TEXTBOX_ENDTYPE_2_CHOICE => BoxEnd::Choice(2),
                TEXTBOX_ENDTYPE_3_CHOICE => BoxEnd::Choice(3),
                TEXTBOX_ENDTYPE_DEFAULT => BoxEnd::End,
                _ => BoxEnd::Event,
            }),
            _ if ocarina => Some(BoxEnd::Ocarina),
            _ => None,
        };
        let Some(end) = end else {
            settle = 0;
            // a pause inside a box (`{AWAIT_BUTTON_PRESS}`): on with A
            pressing = msg.msg_mode == MSGMODE_TEXT_AWAIT_INPUT;
            continue;
        };
        // a frame more at the wait, for its icon
        settle += 1;
        if settle < 2 {
            continue;
        }
        settle = 0;
        let r = &msg.regs;
        boxes.push(PreviewBox { text_id: msg.text_id, box_type: msg.text_box_type, rect: [r.textbox_x, r.textbox_y, r.textbox_width, r.textbox_height], sprites: msg.sprites.clone(), end });
        if boxes.len() >= max_boxes || !matches!(end, BoxEnd::Next | BoxEnd::Continues(_) | BoxEnd::Delayed) {
            break;
        }
        if end == BoxEnd::Delayed {
            // it goes on by itself, to the next box
            while msg.msg_mode == MSGMODE_TEXT_DELAYED_BREAK {
                msg.update(&mut frame!(&idle));
                msg.draw_update(&mut frame!(&idle));
            }
        } else {
            pressing = true;
        }
    }
    boxes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_preview_types_each_box_and_follows_the_next_text() {
        let mut t = MessageTable::default();
        for (id, tp, text) in [(0x1000u16, 0x00u8, "Hi, {NAME}!{BOX_BREAK}Two{TEXTID:0x1001}"), (0x1001, 0x00, "Three?\n{TWO_CHOICE}Yes\nNo"), (0x1002, 0x10, "A sign.")] {
            t.entries.push(MessageTableEntry { text_id: id, type_pos: tp, segment: 0x0700_0000 + t.data.len() as u32 });
            t.data.extend(encode(text).unwrap());
        }
        t.entries.push(MessageTableEntry { text_id: 0xFFFF, type_pos: 0, segment: 0x0700_0000 + t.data.len() as u32 });
        let p = preview(&t, 0x1000, "Link", 8);
        let ends: Vec<BoxEnd> = p.iter().map(|b| b.end).collect();
        assert_eq!(ends, [BoxEnd::Next, BoxEnd::Continues(0x1001), BoxEnd::Choice(2)]);
        assert_eq!(p.iter().map(|b| b.text_id).collect::<Vec<_>>(), [0x1000, 0x1000, 0x1001]);
        // the box, then each glyph and its shadow ("Hi," and "Link!": 8), then the triangle
        let glyphs = |b: &PreviewBox| b.sprites.iter().filter(|s| s.name.starts_with("message/char")).count();
        assert_eq!(p[0].sprites[0].name, "message/box0");
        assert_eq!(glyphs(&p[0]), 16);
        assert_eq!(p[0].sprites.last().unwrap().name, "message/icon0");
        assert_eq!(p[2].sprites.last().unwrap().name, "message/icon2", "the choice's arrow");
        // a variable box with no actor sits at the bottom: y 142
        assert_eq!(p[0].rect, [34, 142, 256, 64]);
        let sign = preview(&t, 0x1002, "Link", 8);
        assert_eq!((sign.len(), sign[0].box_type, sign[0].end), (1, 1, BoxEnd::End));
        assert_eq!(sign[0].sprites.last().unwrap().name, "message/icon1", "the square");
    }

    #[test]
    fn codes_and_characters_read_and_write() {
        let bytes = [b'H', b'i', MESSAGE_COLOR, 0x41, b' ', 0x9F, 0xA9, MESSAGE_NEWLINE, MESSAGE_NAME, MESSAGE_BOX_BREAK, MESSAGE_TEXTID, 0x10, 0x36, b'[', MESSAGE_END, 0];
        let t = decode(&bytes);
        assert_eq!(t, "Hi{COLOR:RED} [A]▼\n{NAME}{BOX_BREAK}{TEXTID:0x1036}{0x5B}");
        assert_eq!(encode(&t).unwrap(), bytes);
        assert_eq!(encode("Hi[").unwrap(), [b'H', b'i', b'[', MESSAGE_END]);
        assert!(encode("{COLOR:PINK}").is_err());
        assert!(encode("{NOPE}").is_err());
        assert!(encode("€").is_err());
        assert_eq!(encode("a\r\nb").unwrap(), [b'a', MESSAGE_NEWLINE, b'b', MESSAGE_END]);
    }

    #[test]
    fn a_long_line_or_a_fifth_line_is_warned_of() {
        let ok = encode("Hey, {NAME}!\nYou're up early.").unwrap();
        assert!(layout_warnings(&ok, "Link").is_empty());
        let long = encode(&"W".repeat(30)).unwrap();
        assert_eq!(layout_warnings(&long, "Link").len(), 1);
        let tall = encode("a\nb\nc\nd\ne").unwrap();
        assert_eq!(layout_warnings(&tall, "Link"), vec!["5 lines; the box shows 4".to_string()]);
        let two = encode("a{BOX_BREAK}a\nb\nc\nd\ne").unwrap();
        assert_eq!(layout_warnings(&two, "Link"), vec!["box 2: 5 lines; the box shows 4".to_string()]);
    }

    #[test]
    fn a_layered_table_finds_the_new_text_and_keeps_the_rest() {
        let mut t = MessageTable::default();
        for (k, (id, bytes)) in [(0x1000u16, encode("one").unwrap()), (0x1001, encode("two").unwrap()), (0xFFFC, encode("x").unwrap()), (0xFFFD, encode("").unwrap())].into_iter().enumerate() {
            let _ = k;
            t.entries.push(MessageTableEntry { text_id: id, type_pos: 0x00, segment: 0x0700_0000 + t.data.len() as u32 });
            t.data.extend(bytes);
        }
        t.entries.push(MessageTableEntry { text_id: 0xFFFF, type_pos: 0, segment: 0x0700_0000 + t.data.len() as u32 });
        let l = t.layered(&[
            TextOverride { text_id: 0x1001, type_pos: 0x10, bytes: encode("TWO, longer").unwrap() },
            TextOverride { text_id: 0x8000, type_pos: 0x00, bytes: encode("new").unwrap() },
        ]);
        assert_eq!(decode(l.raw(0x1000).unwrap()), "one");
        assert_eq!(decode(l.raw(0x1001).unwrap()), "TWO, longer");
        assert_eq!(l.entry(0x1001).unwrap().type_pos, 0x10);
        assert_eq!(decode(l.raw(0x8000).unwrap()), "new");
        let ids: Vec<u16> = l.entries.iter().map(|e| e.text_id).collect();
        assert_eq!(ids, [0x1000, 0x1001, 0x8000, 0xFFFC, 0xFFFD, 0xFFFF]);
        assert_eq!(t.layered(&[]), t);
    }

    /// The measure holds the game's own messages: every box of every message fits (but 0xFFFC,
    /// `Font_LoadOrderedFont`'s list of the name entry's characters, never shown as a box).
    #[test]
    fn the_games_own_messages_fit_their_boxes() {
        let Ok(pack) = crate::pack::GamePack::open_default() else {
            eprintln!("skipping: no pack");
            return;
        };
        let table = pack.messages().unwrap();
        let mut bad = vec![];
        let mut widest = 0f32;
        for e in table.entries.iter().take_while(|e| e.text_id < 0xFFFC) {
            let Some(raw) = table.raw(e.text_id) else { continue };
            widest = measure(raw, "Link").iter().map(|b| b.widest).fold(widest, f32::max);
            let w = layout_warnings(raw, "Link");
            if !w.is_empty() {
                bad.push(format!("{:#06x}: {w:?}", e.text_id));
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("
"));
        // and it isn't loose: the widest line nearly fills the box
        assert!(TEXT_LEFT + widest > TEXT_RIGHT - 4.0, "the widest line is {widest}");
    }

    /// Every message of the game's English table reads and writes back to its own bytes.
    #[test]
    fn every_message_round_trips() {
        let Ok(pack) = crate::pack::GamePack::open_default() else {
            eprintln!("skipping: no pack");
            return;
        };
        let table = pack.messages().unwrap();
        let mut bad = vec![];
        let mut n = 0;
        for e in table.entries.iter().take_while(|e| e.text_id != 0xFFFF) {
            let Some(raw) = table.raw(e.text_id) else { continue };
            n += 1;
            let text = decode(raw);
            match encode(&text) {
                Ok(b) if b == raw => {}
                Ok(b) => bad.push(format!("{:#06x}: {text:?}\n  {raw:02X?}\n  {b:02X?}", e.text_id)),
                Err(err) => bad.push(format!("{:#06x}: {text:?}: {err}", e.text_id)),
            }
        }
        assert!(n > 2000, "only {n} messages");
        let last = table.entries.iter().map(|e| e.text_id).filter(|&id| id < 0xFFFC).max().unwrap();
        assert!(last < FIRST_NEW_TEXT_ID, "the game's last id {last:#06x} is in the new ids");
        assert!(bad.is_empty(), "{} of {n} messages don't round-trip:\n{}", bad.len(), bad.iter().take(10).cloned().collect::<Vec<_>>().join("\n"));
        // the bytes the readable form can only escape: none but the table's own oddities
        let escaped: Vec<String> = table
            .entries
            .iter()
            .take_while(|e| e.text_id != 0xFFFF)
            .filter_map(|e| Some((e.text_id, decode(table.raw(e.text_id)?))))
            .filter(|(_, t)| t.contains("{0x") || t.contains("{END}"))
            .map(|(id, t)| format!("{id:#06x} {t:?}"))
            .collect();
        eprintln!("{n} messages round-trip; {} with escaped bytes:\n{}", escaped.len(), escaped.join("\n"));
    }
}

