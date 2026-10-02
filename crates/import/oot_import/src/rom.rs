//! ROM loading: byte-order normalisation, the dmadata file table, and file access by name.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result, bail};

use crate::yaz0;

#[derive(Debug, Clone, Copy)]
pub struct DmaEntry {
    pub vrom_start: u32,
    pub vrom_end: u32,
    pub rom_start: u32,
    pub rom_end: u32,
}

impl DmaEntry {
    pub fn is_present(&self) -> bool {
        self.rom_start != 0xFFFF_FFFF && self.vrom_end > self.vrom_start
    }
    pub fn is_compressed(&self) -> bool {
        self.rom_end != 0
    }
    pub fn size(&self) -> usize {
        (self.vrom_end - self.vrom_start) as usize
    }
}

pub struct Rom {
    pub data: Vec<u8>,
    pub title: String,
    pub game_code: String,
    pub dmadata_offset: usize,
    pub files: Vec<DmaEntry>,
    names: Vec<Option<String>>,
    by_name: HashMap<String, usize>,
    cache: std::sync::Mutex<HashMap<usize, Arc<[u8]>>>,
}

impl Rom {
    pub fn open(path: &Path) -> Result<Rom> {
        let raw = std::fs::read(path).with_context(|| format!("reading ROM {}", path.display()))?;
        Self::from_bytes(raw)
    }

    pub fn from_bytes(mut data: Vec<u8>) -> Result<Rom> {
        normalize_byte_order(&mut data)?;
        let title = String::from_utf8_lossy(&data[0x20..0x34]).trim().to_string();
        let game_code = String::from_utf8_lossy(&data[0x3B..0x3F]).to_string();
        let dmadata_offset = find_dmadata(&data).context("could not locate dmadata table")?;
        let files = parse_dmadata(&data, dmadata_offset);
        Ok(Rom {
            data,
            title,
            game_code,
            dmadata_offset,
            files,
            names: Vec::new(),
            by_name: HashMap::new(),
            cache: Default::default(),
        })
    }

    /// Attach file names (in dmadata order). The ROM carries no names itself, so they
    /// come from the decomp's `spec`. Returns false if the counts disagree.
    pub fn set_names(&mut self, names: Vec<String>) -> bool {
        if names.len() != self.files.len() {
            log::warn!(
                "spec lists {} segments but dmadata has {} entries; names not applied",
                names.len(),
                self.files.len()
            );
            return false;
        }
        self.by_name = names.iter().enumerate().map(|(i, n)| (n.clone(), i)).collect();
        self.names = names.into_iter().map(Some).collect();
        true
    }

    pub fn name_of(&self, index: usize) -> Option<&str> {
        self.names.get(index).and_then(|n| n.as_deref())
    }

    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.by_name.get(name).copied()
    }

    pub fn file(&self, index: usize) -> Result<Arc<[u8]>> {
        if let Some(hit) = self.cache.lock().unwrap().get(&index) {
            return Ok(hit.clone());
        }
        let e = *self.files.get(index).context("file index out of range")?;
        if !e.is_present() {
            bail!("file {index} is not present in this ROM");
        }
        let bytes: Vec<u8> = if e.is_compressed() {
            yaz0::decompress(&self.data[e.rom_start as usize..e.rom_end as usize])?
        } else {
            self.data[e.rom_start as usize..e.rom_start as usize + e.size()].to_vec()
        };
        let arc: Arc<[u8]> = bytes.into();
        self.cache.lock().unwrap().insert(index, arc.clone());
        Ok(arc)
    }

    pub fn file_by_name(&self, name: &str) -> Result<Arc<[u8]>> {
        let idx = self.index_of(name).with_context(|| format!("no file named {name}"))?;
        self.file(idx)
    }

    /// Returns the index of the file whose VROM range contains `vrom`.
    pub fn file_containing_vrom(&self, vrom: u32) -> Option<usize> {
        self.files.iter().position(|e| e.vrom_start <= vrom && vrom < e.vrom_end)
    }
}

fn normalize_byte_order(data: &mut [u8]) -> Result<()> {
    match data.get(0..4) {
        Some([0x80, 0x37, 0x12, 0x40]) => Ok(()),
        Some([0x37, 0x80, 0x40, 0x12]) => {
            for c in data.chunks_exact_mut(2) {
                c.swap(0, 1);
            }
            Ok(())
        }
        Some([0x40, 0x12, 0x37, 0x80]) => {
            for c in data.chunks_exact_mut(4) {
                c.reverse();
            }
            Ok(())
        }
        _ => bail!("unrecognised ROM header (not z64/v64/n64)"),
    }
}

/// dmadata always starts with the makerom entry (0..0x1060) followed by boot at 0x1060.
fn find_dmadata(data: &[u8]) -> Option<usize> {
    const SIG: [u8; 20] = [
        0, 0, 0, 0, 0, 0, 0x10, 0x60, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10, 0x60,
    ];
    (0..data.len().saturating_sub(32)).step_by(16).find(|&off| data[off..off + 20] == SIG)
}

fn parse_dmadata(data: &[u8], offset: usize) -> Vec<DmaEntry> {
    let be = |o: usize| u32::from_be_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]]);
    let mut out = Vec::new();
    let mut o = offset;
    loop {
        let e = DmaEntry { vrom_start: be(o), vrom_end: be(o + 4), rom_start: be(o + 8), rom_end: be(o + 12) };
        if e.vrom_start == 0 && e.vrom_end == 0 && !out.is_empty() {
            break;
        }
        out.push(e);
        o += 16;
    }
    out
}

/// File names in dmadata order: `baseroms/<version>/segments.csv`'s `Name` column, the list
/// the decomp splits this exact ROM by.
pub fn decomp_file_names(decomp: &Path) -> Result<Vec<String>> {
    let p = decomp.join("baseroms").join(crate::symbols::VERSION).join("segments.csv");
    let text = std::fs::read_to_string(&p).with_context(|| format!("reading {}", p.display()))?;
    let mut lines = text.lines();
    anyhow::ensure!(lines.next().is_some_and(|h| h.starts_with("Name")), "{}: no Name column", p.display());
    Ok(lines.filter_map(|l| l.split(',').next()).map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).collect())
}
