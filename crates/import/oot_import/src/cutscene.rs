//! Cutscene scripts (docs/adr/0022-cutscenes.md): the pack holds each one as the ROM's bytes.
//!
//! - **Scene scripts** are the `<Cutscene>` symbols of the scene XMLs (73). Their length isn't
//!   stored anywhere: `scene_script` walks the commands from the symbol's offset to `CS_END`,
//!   as `Cutscene_ProcessCommands` would (`oot_game::cutscene::walk`).
//! - **Overlay scripts** are the `CutsceneData` arrays of the actors' C (27, in
//!   `*_cutscene_data*.c`), whose offsets in their overlay no XML gives. `expand` builds each
//!   array's words from its macros, with `z64cutscene_commands.h`'s own definitions, and
//!   `overlay_scripts` finds those words in the overlay's file in the ROM and takes the ROM's
//!   bytes. An array that isn't found is an import error.
//! - **`sEntranceCutsceneTable`** (`z_demo.c`) names each entrance's script by symbol.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use oot_game::cutscene::{CutsceneScript, EntranceCutscene, walk};
use oot_game::pack::keys;
use oot_game::scene::EntranceInfo;

use crate::csrc::{find_initializer, parse_enum, parse_int, strip_comments};
use crate::rom::Rom;

/// The script at `offset` in a scene file, through its `CS_END`.
pub fn scene_script(data: &[u8], offset: usize) -> Result<Vec<u8>> {
    let d = data.get(offset..).context("offset past the file")?;
    let (_, end) = walk(d).map_err(|e| anyhow::anyhow!("{e}"))?;
    let end = end.context("no CS_END")?;
    Ok(d[..end].to_vec())
}

/// `#define NAME(params) body` or `#define NAME body`, with line continuations joined.
#[derive(Debug, Clone)]
struct Define {
    params: Option<Vec<String>>,
    body: String,
}

/// The macros of `z64cutscene_commands.h`, and the constants their arguments use: the
/// `CutsceneCmd` and `CutsceneTerminatorDestination` enums and `CS_CMD_CONTINUE` /
/// `CS_CMD_STOP` (`z64cutscene.h`).
pub struct Macros {
    defines: HashMap<String, Define>,
    consts: HashMap<String, i64>,
}

fn split_top(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0;
    let mut cur = String::new();
    for c in s.chars() {
        match c {
            '(' => {
                depth += 1;
                cur.push(c);
            }
            ')' => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 => out.push(std::mem::take(&mut cur).trim().to_string()),
            _ => cur.push(c),
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

/// Replaces whole identifiers of `from` with `to` in `s`.
fn subst(s: &str, map: &HashMap<&str, &str>) -> String {
    let b = s.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_alphabetic() || b[i] == b'_' {
            let st = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            let id = &s[st..i];
            out.push_str(map.get(id).copied().unwrap_or(id));
        } else {
            out.push(b[i] as char);
            i += 1;
        }
    }
    out
}

impl Macros {
    pub fn load(decomp: &Path) -> Result<Macros> {
        let cmds = std::fs::read_to_string(decomp.join("include/z64cutscene_commands.h"))?;
        let header = std::fs::read_to_string(decomp.join("include/z64cutscene.h"))?;
        let mut defines = HashMap::new();
        let joined = strip_comments(&cmds).replace("\\\r\n", " ").replace("\\\n", " ");
        for line in joined.lines() {
            let Some(rest) = line.trim().strip_prefix("#define ") else { continue };
            let name_end = rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).unwrap_or(rest.len());
            let name = rest[..name_end].to_string();
            let after = &rest[name_end..];
            let d = if let Some(p) = after.strip_prefix('(') {
                let close = p.find(')').context("define params")?;
                let params = p[..close].split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                Define { params: Some(params), body: p[close + 1..].trim().to_string() }
            } else {
                Define { params: None, body: after.trim().to_string() }
            };
            defines.insert(name, d);
        }
        let mut consts = HashMap::new();
        for member in ["CS_CMD_CAM_EYE", "KOKIRI_FOREST_INTRO"] {
            for (v, n) in parse_enum(&header, member) {
                consts.insert(n, v);
            }
        }
        anyhow::ensure!(consts.get("CS_CMD_TERMINATOR") == Some(&0x3E8), "CutsceneCmd not read from z64cutscene.h");
        anyhow::ensure!(consts.get("KOKIRI_FOREST_INTRO") == Some(&0x0B), "CutsceneTerminatorDestination not read from z64cutscene.h");
        // `#define CS_CMD_CONTINUE 0`, `#define CS_CMD_STOP -1`.
        for l in strip_comments(&header).lines() {
            if let Some(r) = l.trim().strip_prefix("#define ") {
                let mut it = r.split_whitespace();
                if let (Some(n), Some(v)) = (it.next(), it.next())
                    && n.starts_with("CS_CMD_")
                    && let Some(v) = parse_int(v)
                {
                    consts.insert(n.to_string(), v);
                }
            }
        }
        anyhow::ensure!(consts.get("CS_CMD_STOP") == Some(&-1), "CS_CMD_STOP not read from z64cutscene.h");
        Ok(Macros { defines, consts })
    }

    fn int(&self, s: &str) -> Result<i64> {
        let s = s.trim();
        if let Some(v) = self.consts.get(s) {
            return Ok(*v);
        }
        if let Some(r) = s.strip_prefix('-') {
            return Ok(-self.int(r)?);
        }
        if let Some(r) = s.strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
            return self.int(r);
        }
        parse_int(s).with_context(|| format!("not an integer: {s}"))
    }

    fn float(&self, s: &str) -> Result<f32> {
        let s = s.trim().trim_end_matches(['f', 'F']);
        s.parse::<f32>().or_else(|_| self.int(s).map(|v| v as f32)).with_context(|| format!("not a float: {s}"))
    }

    /// `_SHIFTL(v, s, w)` (`ultra64/gbi.h`): `((u32)v & ((1 << w) - 1)) << s`.
    fn shiftl(v: i64, s: u32, w: u32) -> u32 {
        ((v as u32) & (((1u64 << w) - 1) as u32)) << s
    }

    /// Appends the words of one initializer item (a macro call or a constant) to `out`.
    pub fn expand_item(&self, item: &str, out: &mut Vec<u32>) -> Result<()> {
        let item = item.trim();
        if item.is_empty() {
            return Ok(());
        }
        let (name, args) = match item.find('(') {
            Some(p) if item.ends_with(')') && item[..p].trim().chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && !item[..p].trim().is_empty() => {
                (item[..p].trim(), Some(split_top(&item[p + 1..item.len() - 1])))
            }
            _ => (item, None),
        };
        // command_macros_base.h, and CMD_F (z64cutscene_commands.h: a float's bits).
        let a = |i: usize| -> Result<i64> { self.int(args.as_ref().and_then(|v| v.get(i)).with_context(|| format!("{item}: argument {i}"))?) };
        let w = match (name, &args) {
            ("CMD_W" | "CMD_PTR", Some(_)) => Some(a(0)? as u32),
            ("CMD_HH", Some(_)) => Some(Self::shiftl(a(0)?, 16, 16) | Self::shiftl(a(1)?, 0, 16)),
            ("CMD_BBH", Some(_)) => Some(Self::shiftl(a(0)?, 24, 8) | Self::shiftl(a(1)?, 16, 8) | Self::shiftl(a(2)?, 0, 16)),
            ("CMD_HBB", Some(_)) => Some(Self::shiftl(a(0)?, 16, 16) | Self::shiftl(a(1)?, 8, 8) | Self::shiftl(a(2)?, 0, 8)),
            ("CMD_BBBB", Some(_)) => Some(Self::shiftl(a(0)?, 24, 8) | Self::shiftl(a(1)?, 16, 8) | Self::shiftl(a(2)?, 8, 8) | Self::shiftl(a(3)?, 0, 8)),
            ("CMD_F", Some(v)) => Some(self.float(&v[0])?.to_bits()),
            _ => None,
        };
        if let Some(w) = w {
            out.push(w);
            return Ok(());
        }
        let Some(d) = self.defines.get(name) else {
            out.push(self.int(item).with_context(|| format!("unknown macro or constant {item}"))? as u32);
            return Ok(());
        };
        let body = match (&d.params, &args) {
            (Some(params), Some(args)) => {
                anyhow::ensure!(params.len() == args.len(), "{name}: {} arguments for {} parameters", args.len(), params.len());
                let map: HashMap<&str, &str> = params.iter().map(|p| p.as_str()).zip(args.iter().map(|a| a.as_str())).collect();
                subst(&d.body, &map)
            }
            // An alias (`#define CS_CAM_POS_LIST CS_CAM_EYE_LIST`) called with arguments.
            (None, Some(args)) => format!("{}({})", d.body, args.join(", ")),
            (None, None) => d.body.clone(),
            (Some(_), None) => bail!("{name} needs arguments"),
        };
        for part in split_top(&body) {
            self.expand_item(&part, out)?;
        }
        Ok(())
    }

    /// The words of the `CutsceneData name[]` array in comment-stripped `src`, big-endian.
    pub fn expand(&self, src: &str, name: &str) -> Result<Vec<u8>> {
        let init = find_initializer(src, name)?;
        let mut words = Vec::new();
        for item in init.list() {
            let atom = item.atom().with_context(|| format!("{name}: a nested list"))?;
            self.expand_item(atom, &mut words).with_context(|| format!("{name}: {atom}"))?;
        }
        Ok(words.iter().flat_map(|w| w.to_be_bytes()).collect())
    }
}

/// Every `CutsceneData` array of the actors' C, found in its overlay's file in the ROM.
pub fn overlay_scripts(decomp: &Path, rom: &Rom) -> Result<Vec<CutsceneScript>> {
    let macros = Macros::load(decomp)?;
    let mut out = Vec::new();
    let root = decomp.join("src/overlays/actors");
    let mut dirs: Vec<_> = std::fs::read_dir(&root)?.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.is_dir()).collect();
    dirs.sort();
    for dir in dirs {
        let ovl = dir.file_name().unwrap().to_string_lossy().to_string();
        let mut files: Vec<_> = std::fs::read_dir(&dir)?.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "c")).collect();
        files.sort();
        for f in files {
            let src = strip_comments(&std::fs::read_to_string(&f)?);
            let names: Vec<String> = src
                .match_indices("CutsceneData ")
                .filter_map(|(i, _)| {
                    let rest = &src[i + "CutsceneData ".len()..];
                    let n: String = rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
                    let after = rest[n.len()..].trim_start();
                    (!n.is_empty() && after.starts_with("[]") && after[2..].trim_start().starts_with('=')).then_some(n)
                })
                .collect();
            if names.is_empty() {
                continue;
            }
            let file = rom.file_by_name(&ovl).with_context(|| format!("{ovl} not in the ROM"))?;
            for name in names {
                let words = macros.expand(&src, &name).with_context(|| format!("{}", f.display()))?;
                let at = (0..file.len().saturating_sub(words.len()) + 1).step_by(4).find(|&o| file[o..o + words.len()] == words[..]);
                let Some(at) = at else { bail!("{ovl}: {name} ({} bytes from its C) isn't in the overlay", words.len()) };
                let data = file[at..at + words.len()].to_vec();
                walk(&data).map_err(|e| anyhow::anyhow!("{ovl} {name}: {e}"))?;
                out.push(CutsceneScript { file: ovl.clone(), name, data });
            }
        }
    }
    Ok(out)
}

/// `sEntranceCutsceneTable` (`z_demo.c`): `{ entrance, ageRestriction, flag, segAddr }` rows,
/// the scripts by symbol (`script_key` gives each one's pack key).
pub fn entrance_cutscenes(decomp: &Path, entrances: &[EntranceInfo], script_key: impl Fn(&str) -> Option<String>) -> Result<Vec<EntranceCutscene>> {
    let src = strip_comments(&std::fs::read_to_string(decomp.join("src/code/z_demo.c"))?);
    let save = std::fs::read_to_string(decomp.join("include/z64save.h"))?;
    let flag = |n: &str| -> Result<u8> {
        save.lines()
            .find_map(|l| {
                let r = l.trim().strip_prefix("#define ")?;
                let mut it = r.split_whitespace();
                (it.next()? == n).then(|| it.next().and_then(parse_int))?
            })
            .map(|v| v as u8)
            .with_context(|| format!("no {n} in z64save.h"))
    };
    let init = find_initializer(&src, "sEntranceCutsceneTable")?;
    let mut out = Vec::new();
    for row in init.list() {
        let v = row.flatten();
        anyhow::ensure!(v.len() == 4, "sEntranceCutsceneTable row {v:?}");
        let entrance = entrances.iter().position(|e| e.name == v[0]).with_context(|| format!("no entrance {}", v[0]))? as u16;
        let age_restriction = parse_int(&v[1]).context("ageRestriction")? as u8;
        let script = script_key(&v[3]).with_context(|| format!("no script {}", v[3]))?;
        out.push(EntranceCutscene { entrance, entrance_name: v[0].clone(), age_restriction, flag: flag(&v[2])?, script, script_name: v[3].clone() });
    }
    Ok(out)
}

/// The pack key of a scene XML's script.
pub fn scene_script_key(file: &str, name: &str) -> String {
    keys::cutscene(file, name)
}
