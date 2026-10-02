//! The message tables (`z_message.c`'s `sNesMessageEntryTable`, `sGerMessageEntryTable`,
//! `sFraMessageEntryTable`, `sStaffMessageEntryTable`) and their text.
//!
//! The tables are data in `code`, built from extracted text, so the decomp gives their
//! addresses instead: `baseroms/<version>/config.yml`'s `variables` (`sNesMessageEntryTable`
//! ...), read from `code` at those addresses' offsets (`crate::version`).
//!
//! - `nes` and `staff` are `MessageTableEntry { u16 textId; u8 typePos; const char* segment; }`
//!   (8 bytes), ending with `{ 0xFFFF, 0, NULL }`.
//! - `ger` and `fra` are the segments alone, one per `nes` id except the English-only `0xFFFC`.

use anyhow::{Context, Result};
use oot_game::message::{MessageTable, MessageTableEntry};

use crate::project::Project;
use crate::version::VersionConfig;

/// The tables' offsets in `code`, in their order there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableAddrs {
    pub nes: usize,
    pub ger: usize,
    pub fra: usize,
    pub staff: usize,
}

/// The tables' offsets in `code`, from `config.yml`'s `variables`.
pub fn table_addrs(v: &VersionConfig) -> Result<TableAddrs> {
    let at = |name: &str| -> Result<usize> { v.file_offset("code", v.variable(name)?) };
    Ok(TableAddrs { nes: at("sNesMessageEntryTable")?, ger: at("sGerMessageEntryTable")?, fra: at("sFraMessageEntryTable")?, staff: at("sStaffMessageEntryTable")? })
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
    let a = table_addrs(&VersionConfig::load(&p.config.decomp)?)?;
    let code = p.rom.file_by_name("code")?;
    let bytes = code.get(a.nes..a.ger).context("the nes message table is past code's end")?;
    let entries = parse_entries(bytes);
    anyhow::ensure!(entries.last().is_some_and(|e| e.text_id == 0xFFFF), "the nes message table has no 0xFFFF end");
    let data = p.rom.file_by_name("nes_message_data_static").context("nes_message_data_static")?;
    Ok(MessageTable { entries, data: data.to_vec() })
}
