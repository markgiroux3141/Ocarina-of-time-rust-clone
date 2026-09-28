//! The message tables (`z_message_PAL.c`'s `sNesMessageEntryTable`, `sGerMessageEntryTable`,
//! `sFraMessageEntryTable`, `sStaffMessageEntryTable`) and their text.
//!
//! The tables are data in `code`, built from `assets/text/message_data.h`, which isn't
//! extracted. The decomp's `tools/msgdis.py` reads them from the ROM at fixed VROM addresses
//! (`nes_message_entry_table_addr` ...); they're read from that script here, and since this
//! debug ROM is uncompressed, VROM is the ROM offset.
//!
//! - `nes` and `staff` are `MessageTableEntry { u16 textId; u8 typePos; const char* segment; }`
//!   (8 bytes), ending with `{ 0xFFFF, 0, NULL }`.
//! - `ger` and `fra` are the segments alone, one per `nes` id except the English-only `0xFFFC`.

use std::path::Path;

use anyhow::{Context, Result};
use oot_game::message::{MessageTable, MessageTableEntry};

use crate::project::Project;

/// `msgdis.py`'s table addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableAddrs {
    pub nes: usize,
    pub ger: usize,
    pub fra: usize,
    pub staff: usize,
    pub staff_end: usize,
}

/// Reads `name = 0x...` assignments from the decomp's `tools/msgdis.py`.
pub fn table_addrs(decomp: &Path) -> Result<TableAddrs> {
    let path = decomp.join("tools").join("msgdis.py");
    let src = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let get = |name: &str| -> Result<usize> {
        let (_, v) = src.lines().filter_map(|l| l.split_once('=')).find(|(k, _)| k.trim() == name).with_context(|| format!("{name} not in {}", path.display()))?;
        let v = v.split('#').next().unwrap_or("").trim();
        let hex = v.strip_prefix("0x").or_else(|| v.strip_prefix("0X")).with_context(|| format!("{name}: {v} isn't hexadecimal"))?;
        Ok(usize::from_str_radix(hex, 16)?)
    };
    Ok(TableAddrs {
        nes: get("nes_message_entry_table_addr")?,
        ger: get("ger_message_entry_table_addr")?,
        fra: get("fra_message_entry_table_addr")?,
        staff: get("staff_message_entry_table_addr")?,
        staff_end: get("staff_message_entry_table_addr_end")?,
    })
}

/// `MessageTableEntry`s up to and including the `0xFFFF` terminator.
pub fn parse_entries(bytes: &[u8]) -> Vec<MessageTableEntry> {
    let mut out = Vec::new();
    for c in bytes.chunks_exact(8) {
        let e = MessageTableEntry { text_id: u16::from_be_bytes([c[0], c[1]]), type_pos: c[2], segment: u32::from_be_bytes([c[4], c[5], c[6], c[7]]) };
        out.push(e);
        if e.text_id == 0xFFFF {
            break;
        }
    }
    out
}

/// The English table and `nes_message_data_static`.
pub fn load_messages(p: &Project) -> Result<MessageTable> {
    let a = table_addrs(&p.config.decomp)?;
    let bytes = p.rom.data.get(a.nes..a.ger).context("the nes message table is past the ROM's end")?;
    let entries = parse_entries(bytes);
    anyhow::ensure!(entries.last().is_some_and(|e| e.text_id == 0xFFFF), "the nes message table has no 0xFFFF end");
    let data = p.rom.file_by_name("nes_message_data_static").context("nes_message_data_static")?;
    Ok(MessageTable { entries, data: data.to_vec() })
}
