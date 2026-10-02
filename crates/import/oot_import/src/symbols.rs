//! Reads the decomp's asset XMLs, which name every skeleton, animation, display list and
//! texture inside each ROM file with its offset.
//!
//! The decomp's XMLs describe every ROM version it builds, so for ours:
//! - the files and their XMLs are the ones `baseroms/<version>/config.yml` lists under `assets`
//!   (`ydan_mq.xml` for gc-eu-mq-dbg's Deku Tree), each with the range of its ROM file it covers
//!   (`start_offset`, for the assets inside `code` and the overlays);
//! - a `<Version Pattern="...">` block applies when the pattern matches the version's name;
//! - an element without an `Offset` follows the previous one, and `Offset=".+0x8"` is that many
//!   bytes after it, so each element's size is worked out from its attributes as the decomp's
//!   `tools/assets/descriptor` does (`get_size`): a texture's width × height × bits, an array of
//!   vertices 16 each, a blob its `Size`, a display list 8 × its `Length`, a collision header
//!   0x2C, an animation header 0x10, a skeleton 8 (normal) or 0xC (flex), a standard limb 0xC, a
//!   limb table 4 × its `Count`. Anything else has no size, so whatever follows it has an offset.
//!
//! The symbols' offsets are from their ROM file's start (the range's start added), and a
//! texture's `Tlut="<symbol>"` becomes the `TlutOffset` of that symbol.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use quick_xml::events::Event;

/// The ROM version the importer reads: the debug ROM with Master Quest (ADR 0003).
pub const VERSION: &str = "gc-eu-mq-dbg";

#[derive(Debug, Clone)]
pub struct Symbol {
    /// XML element name, e.g. `Skeleton`, `Animation`, `PlayerAnimation`, `DList`, `Texture`.
    pub kind: String,
    pub name: String,
    pub offset: u32,
    pub attrs: HashMap<String, String>,
}

impl Symbol {
    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attrs.get(key).map(|s| s.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct AssetFile {
    pub name: String,
    pub segment: Option<u8>,
    pub xml_path: PathBuf,
    pub symbols: Vec<Symbol>,
}

impl AssetFile {
    pub fn of_kind<'a>(&'a self, kind: &'a str) -> impl Iterator<Item = &'a Symbol> + 'a {
        self.symbols.iter().filter(move |s| s.kind == kind)
    }
    pub fn find(&self, name: &str) -> Option<&Symbol> {
        self.symbols.iter().find(|s| s.name == name)
    }
}

#[derive(Default)]
pub struct SymbolIndex {
    pub files: Vec<AssetFile>,
    by_file: HashMap<String, usize>,
}

/// One `assets` entry of `config.yml`.
#[derive(Debug, Clone, Default)]
struct AssetConfig {
    xml_path: String,
    start_offset: Option<u32>,
}

/// `config.yml`'s `assets:` list: `- name: ...`, `  xml_path: ...`, `  start_offset: 0x...`.
fn asset_configs(config: &str) -> Vec<AssetConfig> {
    let mut out: Vec<AssetConfig> = Vec::new();
    let mut in_assets = false;
    for line in config.lines() {
        if !line.starts_with(' ') && !line.starts_with('-') {
            in_assets = line.trim_end() == "assets:";
            continue;
        }
        if !in_assets {
            continue;
        }
        let t = line.trim_start_matches(['-', ' ']);
        let Some((k, v)) = t.split_once(':') else { continue };
        let v = v.trim();
        if line.starts_with('-') {
            out.push(AssetConfig::default());
        }
        let Some(cur) = out.last_mut() else { continue };
        match k.trim() {
            "xml_path" => cur.xml_path = v.to_string(),
            "start_offset" => cur.start_offset = parse_offset(v),
            _ => {}
        }
    }
    out
}

impl SymbolIndex {
    /// The XMLs `baseroms/<VERSION>/config.yml` lists, for [`VERSION`].
    pub fn load(decomp: &Path) -> Result<SymbolIndex> {
        let config_path = decomp.join("baseroms").join(VERSION).join("config.yml");
        let config = std::fs::read_to_string(&config_path).with_context(|| format!("reading {}", config_path.display()))?;
        let version = regex_version(VERSION);
        let mut idx = SymbolIndex::default();
        for a in asset_configs(&config) {
            let p = decomp.join(&a.xml_path);
            match parse_xml(&p, a.start_offset.unwrap_or(0), &version) {
                Ok(files) => {
                    for f in files {
                        idx.by_file.insert(f.name.clone(), idx.files.len());
                        idx.files.push(f);
                    }
                }
                Err(e) => log::warn!("skipping {}: {e:#}", p.display()),
            }
        }
        if idx.files.is_empty() {
            bail!("{}: no asset XMLs", config_path.display());
        }
        Ok(idx)
    }

    pub fn file(&self, name: &str) -> Option<&AssetFile> {
        self.by_file.get(name).map(|&i| &self.files[i])
    }
}

/// Matches `<Version Pattern>`s (Python `re.fullmatch` in the decomp's tools) against a version.
struct VersionMatch(String);

fn regex_version(v: &str) -> VersionMatch {
    VersionMatch(v.to_string())
}

impl VersionMatch {
    fn matches(&self, pattern: &str) -> bool {
        regex::Regex::new(&format!("^(?:{pattern})$")).is_ok_and(|r| r.is_match(&self.0))
    }
}

fn parse_offset(s: &str) -> Option<u32> {
    let t = s.trim();
    let t = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")).unwrap_or(t);
    u32::from_str_radix(t, 16).ok()
}

/// An element's size from its attributes (`descriptor`'s `get_size`), or None.
fn element_size(tag: &str, attrs: &HashMap<String, String>, children: &[String]) -> Option<u32> {
    let num = |k: &str| attrs.get(k).and_then(|v| v.trim().parse::<u32>().ok());
    match tag {
        "Texture" => {
            let bits = match attrs.get("Format")?.to_ascii_lowercase().as_str() {
                "rgba32" => 32,
                "rgba16" | "ia16" => 16,
                "ci8" | "i8" | "ia8" => 8,
                "ci4" | "i4" | "ia4" => 4,
                _ => return None,
            };
            Some(num("Width")? * num("Height")? * bits / 8)
        }
        "Array" => (children.first().map(String::as_str) == Some("Vtx")).then(|| num("Count").map(|c| c * 0x10)).flatten(),
        "Blob" => attrs.get("Size").and_then(|s| parse_offset(s)),
        "DList" => num("Length").map(|l| l * 8),
        "Collision" => Some(0x2C),
        "Animation" => Some(0x10),
        "Skeleton" => match attrs.get("Type").map(String::as_str) {
            Some("Normal") => Some(0x8),
            Some("Flex") => Some(0xC),
            _ => None,
        },
        "Limb" => (attrs.get("LimbType").map(String::as_str) == Some("Standard")).then_some(0xC),
        "LimbTable" => num("Count").map(|c| c * 4),
        _ => None,
    }
}

/// A resource element under `<File>` (directly or in a `<Version>` block) as read.
struct Element {
    tag: String,
    attrs: HashMap<String, String>,
    children: Vec<String>,
}

fn parse_xml(path: &Path, base: u32, version: &VersionMatch) -> Result<Vec<AssetFile>> {
    let text = std::fs::read_to_string(path)?;
    let mut reader = quick_xml::Reader::from_str(&text);
    // (file name, segment, its resource elements in order)
    let mut files: Vec<(String, Option<u8>, Vec<Element>)> = Vec::new();
    // Nesting below <File>: 0 outside a file, 1 directly in it; a taken <Version> doesn't count.
    let mut depth = 0usize;
    // Open <Version> blocks: whether each is taken.
    let mut versions: Vec<bool> = Vec::new();
    let mut buf = Vec::new();
    loop {
        let ev = reader.read_event_into(&mut buf)?;
        let (e, is_empty) = match &ev {
            Event::Start(e) => (Some(e.clone()), false),
            Event::Empty(e) => (Some(e.clone()), true),
            Event::End(e) => {
                let tag = AsRef::<str>::as_ref(&e.name()).to_string();
                if tag == "Version" {
                    versions.pop();
                } else {
                    depth = depth.saturating_sub(1);
                }
                buf.clear();
                continue;
            }
            Event::Eof => break,
            _ => (None, false),
        };
        let Some(e) = e else {
            buf.clear();
            continue;
        };
        let tag = AsRef::<str>::as_ref(&e.name()).to_string();
        let mut attrs = HashMap::new();
        #[allow(deprecated)]
        for a in e.attributes().flatten() {
            let k = AsRef::<str>::as_ref(&a.key).to_string();
            let v = a.unescape_value().map(|v| v.to_string()).unwrap_or_default();
            attrs.insert(k, v);
        }
        let skipping = versions.iter().any(|t| !t);
        if tag == "Version" {
            let taken = attrs.get("Pattern").is_some_and(|p| version.matches(p));
            if !is_empty {
                versions.push(taken);
            }
        } else if tag == "File" {
            files.push((attrs.get("Name").cloned().unwrap_or_default(), attrs.get("Segment").and_then(|s| s.parse().ok()), Vec::new()));
            if !is_empty {
                depth = 1;
            }
        } else {
            if !skipping && depth == 1 {
                if let Some(f) = files.last_mut() {
                    f.2.push(Element { tag, attrs, children: Vec::new() });
                }
            } else if !skipping && depth == 2 {
                // An element's own children (<Array><Vtx/></Array>).
                if let Some(el) = files.last_mut().and_then(|f| f.2.last_mut()) {
                    el.children.push(tag);
                }
            }
            if !is_empty && depth > 0 {
                depth += 1;
            }
        }
        buf.clear();
    }
    let mut out = Vec::new();
    for (name, segment, elements) in files {
        let mut symbols = Vec::new();
        let mut prev_end: Option<u32> = Some(0);
        for el in elements {
            let Some(sym_name) = el.attrs.get("Name").cloned() else {
                prev_end = None;
                continue;
            };
            let offset = match el.attrs.get("Offset") {
                Some(o) if o.starts_with(".+") => prev_end.and_then(|p| parse_offset(&o[2..]).map(|r| p + r)),
                Some(o) => parse_offset(o),
                None => prev_end,
            };
            let Some(offset) = offset else {
                log::warn!("{}: {name}'s {sym_name} has no offset (it follows an element of unknown size)", path.display());
                prev_end = None;
                continue;
            };
            prev_end = element_size(&el.tag, &el.attrs, &el.children).map(|s| offset + s);
            let mut attrs = el.attrs;
            // Offsets in the file: the range's start added (TlutOffset is in the same file).
            if let Some(t) = attrs.get("TlutOffset").and_then(|t| parse_offset(t)) {
                attrs.insert("TlutOffset".into(), format!("0x{:X}", t + base));
            }
            symbols.push(Symbol { kind: el.tag, name: sym_name, offset: offset + base, attrs });
        }
        // `Tlut="<symbol>"`: the palette by name, in the same file.
        let offsets: HashMap<String, u32> = symbols.iter().map(|s| (s.name.clone(), s.offset)).collect();
        for s in &mut symbols {
            if let Some(o) = s.attrs.get("Tlut").and_then(|t| offsets.get(t)).copied() {
                s.attrs.insert("TlutOffset".into(), format!("0x{o:X}"));
            }
        }
        out.push(AssetFile { name, segment, xml_path: path.to_path_buf(), symbols });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_and_offsets() {
        let dir = std::env::temp_dir().join("oot_import_symbols_test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("t.xml");
        std::fs::write(
            &p,
            r#"<Root><File Name="f" Segment="6">
                <Texture Name="a" Format="rgba16" Width="4" Height="4" Offset="0x10"/>
                <Version Pattern="ntsc-.*"><Blob Name="x" Size="0x8"/></Version>
                <Version Pattern="gc-eu-mq(-dbg)?|gc-us-mq"><Blob Name="y" Size="0x4"/></Version>
                <Array Name="v" Count="2"><Vtx/></Array>
                <DList Name="d" Offset=".+0x8"/>
                <Texture Name="c" Format="ci4" Width="8" Height="8" Offset="0x100" Tlut="a"/>
            </File></Root>"#,
        )
        .unwrap();
        let f = &parse_xml(&p, 0x20, &regex_version(VERSION)).unwrap()[0];
        let o = |n: &str| f.find(n).map(|s| s.offset);
        assert_eq!((o("a"), o("x"), o("y"), o("v"), o("d")), (Some(0x30), None, Some(0x50), Some(0x54), Some(0x7C)));
        assert_eq!(f.find("c").unwrap().attr("TlutOffset"), Some("0x30"));
    }

    #[test]
    fn reads_the_asset_list() {
        let c = asset_configs("dmadata_start: 0x12F70\nassets:\n- name: code/x\n  xml_path: assets/xml/code/x.xml\n  start_offset: 0x10ED48\n  end_offset: 0x110038\n- name: objects/y\n  xml_path: assets/xml/objects/y.xml\nvariables:\n  a: 1\n");
        assert_eq!(c.len(), 2);
        assert_eq!((c[0].xml_path.as_str(), c[0].start_offset), ("assets/xml/code/x.xml", Some(0x10ED48)));
        assert_eq!((c[1].xml_path.as_str(), c[1].start_offset), ("assets/xml/objects/y.xml", None));
    }
}
