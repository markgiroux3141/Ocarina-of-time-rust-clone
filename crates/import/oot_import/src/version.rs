//! What the decomp knows about our ROM version (`crate::symbols::VERSION`) beyond its source:
//! `baseroms/<version>/config.yml`'s `variables` (the addresses of tables built from extracted
//! data, `sNesMessageEntryTable`, `gSoundFontTable`...) and `incbins` (the data `.incbin`'d from
//! the ROM, `aspMainData`...), and `segments.csv`'s VRAM addresses of the ROM's files.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};

use crate::rom::Rom;
use crate::symbols::VERSION;

/// One `incbins` entry: `name`, `segment` (the ROM file), `vram`, `size`.
#[derive(Debug, Clone)]
pub struct Incbin {
    pub segment: String,
    pub vram: u32,
    pub size: u32,
}

#[derive(Debug, Clone, Default)]
pub struct VersionConfig {
    pub variables: HashMap<String, u32>,
    pub incbins: HashMap<String, Incbin>,
    /// `segments.csv`: each ROM file's VRAM start, for the files that have one.
    pub segment_vram: HashMap<String, u32>,
}

fn hex(s: &str) -> Option<u32> {
    let t = s.trim();
    u32::from_str_radix(t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")).unwrap_or(t), 16).ok()
}

impl VersionConfig {
    pub fn load(decomp: &Path) -> Result<VersionConfig> {
        let dir = decomp.join("baseroms").join(VERSION);
        let p = dir.join("config.yml");
        let config = std::fs::read_to_string(&p).with_context(|| format!("reading {}", p.display()))?;
        let mut out = VersionConfig::default();
        let mut section = "";
        let mut cur: Option<(String, Incbin)> = None;
        for line in config.lines() {
            if !line.starts_with([' ', '-']) {
                section = line.trim_end().trim_end_matches(':');
                continue;
            }
            let t = line.trim_start_matches(['-', ' ']);
            let Some((k, v)) = t.split_once(':') else { continue };
            let (k, v) = (k.trim(), v.trim());
            match section {
                "variables" => {
                    if let Some(a) = hex(v) {
                        out.variables.insert(k.to_string(), a);
                    }
                }
                "incbins" => {
                    if line.trim_start().starts_with('-') {
                        if let Some((n, i)) = cur.take() {
                            out.incbins.insert(n, i);
                        }
                        cur = Some((String::new(), Incbin { segment: String::new(), vram: 0, size: 0 }));
                    }
                    if let Some((n, i)) = cur.as_mut() {
                        match k {
                            "name" => *n = v.to_string(),
                            "segment" => i.segment = v.to_string(),
                            "vram" => i.vram = hex(v).unwrap_or(0),
                            "size" => i.size = hex(v).unwrap_or(0),
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        if let Some((n, i)) = cur.take() {
            out.incbins.insert(n, i);
        }
        let p = dir.join("segments.csv");
        let csv = std::fs::read_to_string(&p).with_context(|| format!("reading {}", p.display()))?;
        for line in csv.lines().skip(1) {
            let mut f = line.split(',');
            if let (Some(n), Some(v)) = (f.next(), f.next())
                && let Some(v) = hex(v)
            {
                out.segment_vram.insert(n.trim().to_string(), v);
            }
        }
        Ok(out)
    }

    pub fn variable(&self, name: &str) -> Result<u32> {
        self.variables.get(name).copied().with_context(|| format!("config.yml has no variable {name}"))
    }

    /// `vram`'s offset in the ROM file `segment`.
    pub fn file_offset(&self, segment: &str, vram: u32) -> Result<usize> {
        let base = self.segment_vram.get(segment).copied().with_context(|| format!("segments.csv: no VRAM for {segment}"))?;
        anyhow::ensure!(vram >= base, "0x{vram:08X} is before {segment} (0x{base:08X})");
        Ok((vram - base) as usize)
    }

    /// `len` bytes of `segment` (a ROM file) at `vram`.
    pub fn bytes(&self, rom: &Rom, segment: &str, vram: u32, len: usize) -> Result<Vec<u8>> {
        let off = self.file_offset(segment, vram)?;
        let file = rom.file_by_name(segment)?;
        Ok(file.get(off..off + len).with_context(|| format!("0x{vram:08X}+0x{len:X} is past {segment}'s end"))?.to_vec())
    }

    /// An `incbins` entry's bytes.
    pub fn incbin(&self, rom: &Rom, name: &str) -> Result<Vec<u8>> {
        let i = self.incbins.get(name).with_context(|| format!("config.yml has no incbin {name}"))?;
        self.bytes(rom, &i.segment, i.vram, i.size as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_variables_and_incbins() {
        let dir = std::env::temp_dir().join("oot_import_version_test").join("baseroms").join(VERSION);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("config.yml"), "dmadata_start: 0x12F70\nincbins:\n  - name: ipl3\n    segment: makerom\n    vram: 0x80000040\n    size: 0xFC0\n  - name: aspMainData\n    segment: code\n    vram: 0x80155C70\n    size: 0x2E0\nvariables:\n  gSoundFontTable: 0x801550D0\nassets:\n- name: x\n").unwrap();
        std::fs::write(dir.join("segments.csv"), "Name,VRAM start\nmakerom,80000000\ndmadata,\ncode,8001CE60\n").unwrap();
        let c = VersionConfig::load(&dir.parent().unwrap().parent().unwrap()).unwrap();
        assert_eq!(c.variable("gSoundFontTable").unwrap(), 0x801550D0);
        assert_eq!(c.file_offset("code", 0x801550D0).unwrap(), 0x138270);
        let a = &c.incbins["aspMainData"];
        assert_eq!((a.segment.as_str(), a.vram, a.size), ("code", 0x80155C70, 0x2E0));
        assert!(c.segment_vram.get("dmadata").is_none());
    }
}
