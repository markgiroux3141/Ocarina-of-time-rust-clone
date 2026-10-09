//! `ootx sram`: the save file (the cartridge's SRAM image), and the file select's screens the
//! port doesn't have, stood in for through `z_sram.c` (`oot_game::sram`; docs/adr/0049-saving.md).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use oot_game::file_select::title_and_file_select;
use oot_game::sram::*;

#[derive(Subcommand, Clone, Copy)]
pub enum SramAction {
    /// The three files: name, hearts, deaths, where each was saved, the checksums (the default).
    List,
    /// The title's checks (`Sram_InitSram`, `Sram_VerifyAndLoadAllSaves`): a bad header or file
    /// put right (a file from its backup, or made a new save) and written.
    Verify,
    /// The file select's erase (`Sram_EraseSave`): file N (1 to 3) and its backup a new save.
    Erase { file: usize },
    /// The file select's copy (`Sram_CopySave`): file FROM over the empty file TO and its backup.
    Copy { from: usize, to: usize },
}

/// The save file of the default pack's ROM (`oot_game::pack::save_path`), unless `path`.
fn sram_path(path: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = path {
        return Ok(p);
    }
    let pack = oot_game::pack::GamePack::open_default().context("the default pack names the ROM whose save file it is (or pass --path)")?;
    Ok(oot_game::pack::save_path(&pack.header().source_sha1))
}

fn read(path: &Path) -> Result<Sram> {
    let bytes = std::fs::read(path).with_context(|| format!("no save file at {}", path.display()))?;
    Sram::from_bytes(bytes).map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))
}

fn write(path: &Path, sram: &Sram) -> Result<()> {
    eng_asset::write_file_atomic(path, &sram.bytes)?;
    println!("wrote {}", path.display());
    Ok(())
}

/// A file name in the file select's characters (`FILENAME_DIGIT`, `_UPPERCASE`, `_LOWERCASE`,
/// `_SPACE`, `_DASH`, `_PERIOD`; `!OOT_NTSC`).
fn file_name(name: &[u8]) -> String {
    name.iter()
        .map(|&c| match c {
            0x00..=0x09 => (b'0' + c) as char,
            0x0A..=0x23 => (b'A' + c - 0x0A) as char,
            0x24..=0x3D => (b'a' + c - 0x24) as char,
            0x3E => ' ',
            0x3F => '-',
            0x40 => '.',
            _ => '?',
        })
        .collect::<String>()
        .trim_end()
        .to_string()
}

fn check_file(n: usize) -> Result<usize> {
    if !(1..=3).contains(&n) {
        bail!("there is no file {n}: the files are 1, 2 and 3");
    }
    Ok(n - 1)
}

/// `ootx sram`.
pub fn run(action: Option<SramAction>, path: Option<PathBuf>) -> Result<()> {
    let path = sram_path(path)?;
    let mut sram = read(&path)?;
    println!("{}", path.display());
    match action.unwrap_or(SramAction::List) {
        SramAction::List => list(&sram),
        SramAction::Verify => {
            let before = sram.bytes.clone();
            title_and_file_select(&mut sram);
            if sram.bytes == before {
                println!("the header and every file check out");
                Ok(())
            } else {
                println!("put right ({} writes)", sram.writes);
                list(&sram)?;
                write(&path, &sram)
            }
        }
        SramAction::Erase { file } => {
            let i = check_file(file)?;
            let (mut save, mut sram_ctx, mut fs) = title_and_file_select(&mut sram);
            if !slot_occupied(&sram_ctx.read_buff, i) {
                bail!("file {file} is empty already");
            }
            fs.selected_file_index = i as i16;
            sram_erase_save(&mut fs, &mut save, &mut sram_ctx, &mut sram);
            println!("file {file} erased");
            write(&path, &sram)
        }
        SramAction::Copy { from, to } => {
            let (i, j) = (check_file(from)?, check_file(to)?);
            let (mut save, mut sram_ctx, mut fs) = title_and_file_select(&mut sram);
            if !slot_occupied(&sram_ctx.read_buff, i) {
                bail!("file {from} is empty: nothing to copy");
            }
            if slot_occupied(&sram_ctx.read_buff, j) {
                bail!("file {to} holds a save: the file select copies only into an empty file (erase it first)");
            }
            fs.selected_file_index = i as i16;
            fs.copy_dest_file_index = j as i16;
            sram_copy_save(&mut fs, &mut save, &mut sram_ctx, &mut sram);
            println!("file {from} copied to file {to}");
            write(&path, &sram)
        }
    }
}

/// What the file select shows of each file, from the image as it stands (nothing put right),
/// and each slot's and backup's checksum.
fn list(sram: &Sram) -> Result<()> {
    let b = &sram.bytes;
    let magic = b[SRAM_HEADER_MAGIC..SRAM_DEFAULT_HEADER.len()] == SRAM_DEFAULT_HEADER[SRAM_HEADER_MAGIC..];
    println!(
        "header: {} (sound {}, Z-targeting {}, language {})",
        if magic { "ok" } else { "no magic: the title screen rewrites it" },
        b[SRAM_HEADER_SOUND],
        b[SRAM_HEADER_Z_TARGET],
        b[SRAM_HEADER_LANGUAGE]
    );
    let be16 = |at: usize| u16::from_be_bytes([b[at], b[at + 1]]);
    let ok = |slot: usize| {
        let o = SRAM_SLOT_OFFSETS[slot] as usize;
        checksum(&b[o..o + SAVE_SIZE]) == be16(o + CHECKSUM_OFFSET)
    };
    for i in 0..3 {
        let o = SRAM_SLOT_OFFSETS[i] as usize;
        let sums = format!("checksum {}, backup {}", if ok(i) { "ok" } else { "bad" }, if ok(i + 3) { "ok" } else { "bad" });
        if !slot_occupied(b, i) {
            println!("file {}: empty ({sums})", i + 1);
            continue;
        }
        let mut s = oot_game::save::SaveContext::default();
        s.read_save(&b[o..o + SAVE_SIZE]);
        println!(
            "file {}: {:8} {}/{} hearts, {} deaths, {} rupees, {}, saved in scene {:#04x} ({sums})",
            i + 1,
            file_name(&s.player_name),
            s.health / 16,
            s.health_capacity / 16,
            s.deaths,
            s.rupees,
            if s.adult { "adult" } else { "child" },
            s.saved_scene_id
        );
    }
    Ok(())
}
