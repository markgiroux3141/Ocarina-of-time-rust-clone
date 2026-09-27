//! Reads the decomp's asset XMLs (`assets/xml/**/*.xml`), which name every skeleton,
//! animation, display list and texture inside each ROM file together with its offset.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use quick_xml::events::Event;

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

impl SymbolIndex {
    pub fn load_dir(xml_root: &Path) -> Result<SymbolIndex> {
        let mut idx = SymbolIndex::default();
        let mut stack = vec![xml_root.to_path_buf()];
        let mut paths = Vec::new();
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).with_context(|| format!("reading {}", dir.display()))? {
                let p = entry?.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|e| e == "xml") {
                    paths.push(p);
                }
            }
        }
        paths.sort();
        for p in paths {
            match parse_xml(&p) {
                Ok(files) => {
                    for f in files {
                        idx.by_file.insert(f.name.clone(), idx.files.len());
                        idx.files.push(f);
                    }
                }
                Err(e) => log::warn!("skipping {}: {e:#}", p.display()),
            }
        }
        Ok(idx)
    }

    pub fn file(&self, name: &str) -> Option<&AssetFile> {
        self.by_file.get(name).map(|&i| &self.files[i])
    }
}

fn parse_offset(s: &str) -> Option<u32> {
    let t = s.trim();
    let t = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")).unwrap_or(t);
    u32::from_str_radix(t, 16).ok()
}

fn parse_xml(path: &Path) -> Result<Vec<AssetFile>> {
    let text = std::fs::read_to_string(path)?;
    let mut reader = quick_xml::Reader::from_str(&text);
    let mut out: Vec<AssetFile> = Vec::new();
    let mut depth_in_file = 0usize;
    let mut buf = Vec::new();
    loop {
        let ev = reader.read_event_into(&mut buf)?;
        let (e, is_empty) = match &ev {
            Event::Start(e) => (Some(e.clone()), false),
            Event::Empty(e) => (Some(e.clone()), true),
            Event::End(_) => {
                depth_in_file = depth_in_file.saturating_sub(1);
                (None, false)
            }
            Event::Eof => break,
            _ => (None, false),
        };
        if let Some(e) = e {
            let tag = AsRef::<str>::as_ref(&e.name()).to_string();
            let mut attrs = HashMap::new();
            #[allow(deprecated)]
            for a in e.attributes().flatten() {
                let k = AsRef::<str>::as_ref(&a.key).to_string();
                let v = a.unescape_value().map(|v| v.to_string()).unwrap_or_default();
                attrs.insert(k, v);
            }
            if tag == "File" {
                out.push(AssetFile {
                    name: attrs.get("Name").cloned().unwrap_or_default(),
                    segment: attrs.get("Segment").and_then(|s| s.parse().ok()),
                    xml_path: path.to_path_buf(),
                    symbols: Vec::new(),
                });
                if !is_empty {
                    depth_in_file = 1;
                }
                buf.clear();
                continue;
            }
            // Only direct children of <File> are symbols; nested (<Array><Vtx/>) entries are not.
            if depth_in_file == 1
                && let (Some(file), Some(name), Some(off)) =
                    (out.last_mut(), attrs.get("Name").cloned(), attrs.get("Offset").and_then(|o| parse_offset(o)))
            {
                file.symbols.push(Symbol { kind: tag, name, offset: off, attrs });
            }
            if !is_empty && depth_in_file > 0 {
                depth_in_file += 1;
            }
        }
        buf.clear();
    }
    Ok(out)
}
