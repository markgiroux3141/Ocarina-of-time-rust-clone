//! Navi's C-Up texts and Saria's (`oot_game::elf_message`): the `ElfMessage` arrays of
//! `src/elf_message/*.c` and `src/code/z_elf_message.c`, built from their `ELF_MSG_*` macros
//! (`z64elf_message.h`) and checked against the ROM. The pack holds the ROM's bytes.
//!
//! - `gOverworldNaviMsgs` and `gDungeonNaviMsgs` are the whole of the files `elf_message_field`
//!   and `elf_message_ydan` (`sNaviMsgFiles`);
//! - `sChildSariaMsgs` and `sAdultSariaMsgs` are data in `code`, found there by their bytes.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use oot_game::elf_message::ElfMessageTables;

use crate::csrc::{find_initializer, parse_enum, parse_int, strip_comments};
use crate::rom::Rom;

/// The constants the macros' arguments name: `ELF_MSG_TYPE_*`, `ELF_MSG_CONDITION_*`
/// (`z64elf_message.h`), `EVENTCHKINF_*` (`z64save.h`, some of them `((X_INDEX << 4) |
/// X_SHIFT)`) and the `ITEM_*` enum (`z64item.h`).
struct Consts {
    values: HashMap<String, i64>,
    defines: HashMap<String, String>,
}

impl Consts {
    fn load(decomp: &Path) -> Result<Consts> {
        let mut defines = HashMap::new();
        for f in ["include/z64elf_message.h", "include/z64save.h"] {
            let text = strip_comments(&std::fs::read_to_string(decomp.join(f))?);
            for line in text.lines() {
                let Some(rest) = line.trim().strip_prefix("#define ") else { continue };
                let name_end = rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).unwrap_or(rest.len());
                // Object-like defines only.
                if rest[name_end..].starts_with('(') {
                    continue;
                }
                defines.insert(rest[..name_end].to_string(), rest[name_end..].trim().to_string());
            }
        }
        let mut values = HashMap::new();
        for (v, n) in parse_enum(&std::fs::read_to_string(decomp.join("include/z64item.h"))?, "ITEM_STICK") {
            values.insert(n, v);
        }
        values.insert("false".into(), 0);
        values.insert("true".into(), 1);
        let c = Consts { values, defines };
        anyhow::ensure!(c.get("ELF_MSG_TYPE_END")? == 7 && c.get("ELF_MSG_CONDITION_MAGIC")? == 4, "z64elf_message.h's constants not read");
        anyhow::ensure!(c.get("EVENTCHKINF_05")? == 5 && c.get("EVENTCHKINF_40")? == 0x40 && c.get("ITEM_SONG_SARIA").is_ok(), "EVENTCHKINF_* or ITEM_* not read");
        Ok(c)
    }

    fn get(&self, s: &str) -> Result<i64> {
        self.eval(s, 0)
    }

    /// An integer expression of `|`, `<<`, `+`, `-`, parentheses, numbers and names.
    fn eval(&self, s: &str, depth: usize) -> Result<i64> {
        anyhow::ensure!(depth < 16, "{s}: nested too deep");
        let s = s.trim();
        if let Some(v) = self.values.get(s) {
            return Ok(*v);
        }
        if let Some(v) = parse_int(s) {
            return Ok(v);
        }
        if let Some(body) = self.defines.get(s) {
            return self.eval(body, depth + 1);
        }
        // The lowest-precedence operator outside parentheses: |, then <<, then + and -.
        for ops in [&["|"][..], &["<<"][..], &["+", "-"][..]] {
            let b = s.as_bytes();
            let mut level = 0i32;
            let mut split = None;
            let mut i = 0;
            while i < b.len() {
                match b[i] {
                    b'(' => level += 1,
                    b')' => level -= 1,
                    _ if level == 0 && i > 0 => {
                        for op in ops {
                            if s[i..].starts_with(op) && !(*op == "|" && s[i..].starts_with("||")) {
                                split = Some((i, *op));
                            }
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            if let Some((i, op)) = split {
                let (l, r) = (self.eval(&s[..i], depth + 1)?, self.eval(&s[i + op.len()..], depth + 1)?);
                return Ok(match op {
                    "|" => l | r,
                    "<<" => l << r,
                    "+" => l + r,
                    _ => l - r,
                });
            }
        }
        if let Some(inner) = s.strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
            return self.eval(inner, depth + 1);
        }
        bail!("unknown constant {s}")
    }
}

/// `_SHIFTL(v, s, w)`.
fn shiftl(v: i64, s: u32, w: u32) -> u8 {
    (((v as u32) & ((1u32 << w) - 1)) << s) as u8
}

/// One `ELF_MSG_*(...)` command's four bytes, as `z64elf_message.h` packs them:
/// `ELF_MSG_B0(type, cond_type, tf)` is `_SHIFTL(ELF_MSG_TYPE_##type, 5, 3) |
/// _SHIFTL(ELF_MSG_CONDITION_##cond_type, 1, 4) | _SHIFTL(tf, 0, 1)`, `ELF_MSG_B1(cond_type,
/// data)` is `_SHIFTL(ELF_MSG_CONDITION_##cond_type, 4, 4) | _SHIFTL(data, 0, 4)`, and each
/// macro lays out its bytes as follows.
fn encode(c: &Consts, item: &str) -> Result<[u8; 4]> {
    let item = item.trim();
    let open = item.find('(').with_context(|| format!("not a macro call: {item}"))?;
    let name = item[..open].trim();
    let args: Vec<&str> = item[open + 1..item.len() - 1].split(',').map(|a| a.trim()).collect();
    let a = |i: usize| -> Result<i64> { c.get(args.get(i).with_context(|| format!("{item}: argument {i}"))?) };
    let b0 = |ty: &str, cond: &str, tf: i64| -> Result<u8> { Ok(shiftl(c.get(&format!("ELF_MSG_TYPE_{ty}"))?, 5, 3) | shiftl(c.get(&format!("ELF_MSG_CONDITION_{cond}"))?, 1, 4) | shiftl(tf, 0, 1)) };
    let b1 = |cond: &str, data: i64| -> Result<u8> { Ok(shiftl(c.get(&format!("ELF_MSG_CONDITION_{cond}"))?, 4, 4) | shiftl(data, 0, 4)) };
    let b = |v: i64| shiftl(v, 0, 8);
    let ty = args.first().copied().unwrap_or("");
    Ok(match name {
        "ELF_MSG_FLAG" => [b0(ty, "FLAG", a(2)?)?, b(a(3)?), b(a(1)?), 0],
        "ELF_MSG_END" => [b0("END", "FLAG", 0)?, 0, b(a(0)?), 0],
        "ELF_MSG_DUNGEON_ITEM" => [b0(ty, "DUNGEON_ITEM", a(2)?)?, b(a(3)?), b(a(1)?), 0],
        "ELF_MSG_ITEM" => [b0(ty, "ITEM", a(2)?)?, b(a(3)?), b(a(1)?), b(a(4)?)],
        "ELF_MSG_STRENGTH_UPG" => [b0(ty, "OTHER", a(2)?)?, b1("STRENGTH_UPG", a(3)?)?, b(a(1)?), 0],
        "ELF_MSG_BOOTS" => [b0(ty, "OTHER", a(2)?)?, b1("BOOTS", 0)?, b(a(1)?), b(a(3)?)],
        "ELF_MSG_SONG" => [b0(ty, "OTHER", a(2)?)?, b1("SONG", 0)?, b(a(1)?), b(a(3)?)],
        "ELF_MSG_MEDALLION" => [b0(ty, "OTHER", a(2)?)?, b1("MEDALLION", 0)?, b(a(1)?), b(a(3)?)],
        "ELF_MSG_MAGIC" => [b0(ty, "OTHER", a(2)?)?, b1("MAGIC", 0)?, b(a(1)?), 0],
        _ => bail!("unknown ElfMessage macro {name}"),
    })
}

/// The bytes of the `ElfMessage` array `name` in `file` (under the decomp).
fn array(c: &Consts, decomp: &Path, file: &str, name: &str) -> Result<Vec<u8>> {
    let src = strip_comments(&std::fs::read_to_string(decomp.join(file))?);
    let init = find_initializer(&src, name)?;
    let mut out = Vec::new();
    for i in init.list() {
        let atom = i.atom().with_context(|| format!("{name}: a nested list"))?;
        out.extend(encode(c, atom).with_context(|| format!("{name}: {atom}"))?);
    }
    Ok(out)
}

/// `ElfMessageTables` from the decomp's C and the ROM.
pub fn load(decomp: &Path, rom: &Rom) -> Result<ElfMessageTables> {
    let c = Consts::load(decomp)?;
    let mut files = Vec::new();
    // sNaviMsgFiles (z_scene.c): ROM_FILE(elf_message_field), ROM_FILE(elf_message_ydan).
    for (file, src, name) in [("elf_message_field", "src/elf_message/elf_message_field.c", "gOverworldNaviMsgs"), ("elf_message_ydan", "src/elf_message/elf_message_ydan.c", "gDungeonNaviMsgs")] {
        let want = array(&c, decomp, src, name)?;
        let data = rom.file_by_name(file)?;
        anyhow::ensure!(data.len() >= want.len() && data[..want.len()] == want[..], "{file}: the ROM's bytes aren't {name}'s ({} bytes from the C)", want.len());
        anyhow::ensure!(data[want.len()..].iter().all(|&b| b == 0), "{file}: more than {name} in the file");
        files.push(data[..want.len()].to_vec());
    }
    let code = rom.file_by_name("code")?;
    let find = |name: &str| -> Result<Vec<u8>> {
        let want = array(&c, decomp, "src/code/z_elf_message.c", name)?;
        let at = (0..code.len().saturating_sub(want.len()) + 1).step_by(4).find(|&o| code[o..o + want.len()] == want[..]);
        let Some(at) = at else { bail!("{name} ({} bytes from its C) isn't in code", want.len()) };
        Ok(code[at..at + want.len()].to_vec())
    };
    Ok(ElfMessageTables { files, child_saria: find("sChildSariaMsgs")?, adult_saria: find("sAdultSariaMsgs")? })
}
