//! Asset packs: named records, serialized with serde (bincode) and compressed (zstd).
//!
//! - **One file** (`PackWriter::write`, `PackFile`): `ASSETPAK`, the header, the index, then the
//!   blobs. Identical records are stored once, and every name that has them points there.
//! - **Loose files** (`PackWriter::write_loose`, `LooseDir`), the dev mode: `header.json` plus one
//!   uncompressed `<name>.bin` per record, to inspect, diff or edit one asset without rebuilding
//!   the pack.
//! - **Layering** (`Assets`): several sources looked up by name, later ones first, so a mod
//!   pack overrides the base pack's records of the same name.
//!
//! The header says what built the pack (`importer`, `importer_version`), from what
//! (`source_sha1`) and in which layout (`format_version`). `PackHeader::is_current` is how a
//! game decides to rebuild.
//!
//! Records are whatever serde types the importer and the game agree on; this crate doesn't
//! know any of them.

use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// The first 8 bytes of a pack file.
pub const MAGIC: [u8; 8] = *b"ASSETPAK";

/// zstd level for pack blobs: fast to write, and within a few percent of the higher levels
/// on these records.
const ZSTD_LEVEL: i32 = 9;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PackHeader {
    /// The layout of the records, bumped whenever a record type changes.
    pub format_version: u32,
    /// What wrote the records (e.g. `oot_import`) and its output version, bumped when the same
    /// input would give different records.
    pub importer: String,
    pub importer_version: u32,
    /// SHA-1 of the source the pack was built from (for the game, the ROM), lowercase hex.
    pub source_sha1: String,
    /// Other facts about the build (decomp commit, ROM title, import time, ...).
    pub info: BTreeMap<String, String>,
}

impl PackHeader {
    /// True if this pack was built by `importer` at `importer_version` in `format_version`,
    /// and (when given) from the source with this SHA-1.
    pub fn is_current(&self, format_version: u32, importer: &str, importer_version: u32, source_sha1: Option<&str>) -> bool {
        self.format_version == format_version
            && self.importer == importer
            && self.importer_version == importer_version
            && source_sha1.is_none_or(|s| s.eq_ignore_ascii_case(&self.source_sha1))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct IndexEntry {
    name: String,
    /// Offset of the blob from the start of the data section.
    offset: u64,
    len: u64,
    raw_len: u64,
}

/// Serializes a record the way packs store it (before compression).
pub fn encode<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>> {
    bincode::serialize(value).context("serializing record")
}

/// Deserializes a record's bytes.
pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    bincode::deserialize(bytes).context("deserializing record")
}

// ---------------------------------------------------------------------------------------------
// Writing
// ---------------------------------------------------------------------------------------------

struct Blob {
    compressed: Vec<u8>,
    raw: Vec<u8>,
}

#[derive(Default)]
struct WriterState {
    names: Vec<(String, usize)>,
    blobs: Vec<Blob>,
    /// (hash, length) of the raw bytes -> blobs, for storing identical records once.
    by_content: HashMap<(u64, usize), Vec<usize>>,
}

/// Collects records, then writes them as a pack file or a loose folder. `put` can be called
/// from several threads.
pub struct PackWriter {
    header: PackHeader,
    state: Mutex<WriterState>,
}

/// What a write produced.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct PackStats {
    pub records: usize,
    /// Distinct records after merging identical ones.
    pub blobs: usize,
    pub raw_bytes: u64,
    pub stored_bytes: u64,
    pub file_bytes: u64,
}

impl PackWriter {
    pub fn new(header: PackHeader) -> PackWriter {
        PackWriter { header, state: Mutex::new(WriterState::default()) }
    }

    /// Adds `value` under `name`. A second record with the same name replaces the first.
    pub fn put<T: Serialize + ?Sized>(&self, name: &str, value: &T) -> Result<()> {
        self.put_raw(name, encode(value)?)
    }

    /// Adds already-serialized record bytes under `name`.
    pub fn put_raw(&self, name: &str, raw: Vec<u8>) -> Result<()> {
        if name.is_empty() || name.starts_with('/') || name.contains("..") || name.contains('\\') {
            bail!("invalid record name {name:?}");
        }
        let key = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            raw.hash(&mut h);
            (h.finish(), raw.len())
        };
        let find = |st: &WriterState| st.by_content.get(&key).and_then(|c| c.iter().copied().find(|&b| st.blobs[b].raw == raw));
        let existing = find(&self.state.lock().unwrap());
        let blob = match existing {
            Some(b) => b,
            None => {
                // Compress outside the lock so several threads can.
                let compressed = zstd::encode_all(&raw[..], ZSTD_LEVEL).context("compressing record")?;
                let mut st = self.state.lock().unwrap();
                match find(&st) {
                    Some(b) => b,
                    None => {
                        st.blobs.push(Blob { compressed, raw });
                        let b = st.blobs.len() - 1;
                        st.by_content.entry(key).or_default().push(b);
                        b
                    }
                }
            }
        };
        let mut st = self.state.lock().unwrap();
        st.names.retain(|(n, _)| n != name);
        st.names.push((name.to_string(), blob));
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.state.lock().unwrap().names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The names in order, and the blobs renumbered in the order the sorted names first use
    /// them, so the file doesn't depend on which thread added what first.
    fn sorted(&self) -> (Vec<(String, usize)>, WriterState) {
        let mut st = std::mem::take(&mut *self.state.lock().unwrap());
        let mut names = std::mem::take(&mut st.names);
        names.sort();
        let mut order: Vec<Option<usize>> = vec![None; st.blobs.len()];
        let mut blobs = Vec::with_capacity(st.blobs.len());
        let mut old: Vec<Option<Blob>> = std::mem::take(&mut st.blobs).into_iter().map(Some).collect();
        for (_, b) in names.iter_mut() {
            let new = *order[*b].get_or_insert_with(|| {
                blobs.push(old[*b].take().unwrap());
                blobs.len() - 1
            });
            *b = new;
        }
        st.blobs = blobs;
        st.by_content.clear();
        (names, st)
    }

    /// Writes one pack file. It's written next to `path` and renamed into place, so a reader
    /// never sees half a pack.
    pub fn write(self, path: &Path) -> Result<PackStats> {
        let (names, st) = self.sorted();
        let mut offsets = Vec::with_capacity(st.blobs.len());
        let mut at = 0u64;
        for b in &st.blobs {
            offsets.push(at);
            at += b.compressed.len() as u64;
        }
        let index: Vec<IndexEntry> = names
            .iter()
            .map(|(n, b)| IndexEntry { name: n.clone(), offset: offsets[*b], len: st.blobs[*b].compressed.len() as u64, raw_len: st.blobs[*b].raw.len() as u64 })
            .collect();
        let header = encode(&self.header)?;
        let index_bytes = zstd::encode_all(&encode(&index)?[..], ZSTD_LEVEL)?;
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d).with_context(|| format!("creating {}", d.display()))?;
        }
        let tmp = path.with_extension("tmp");
        {
            let mut f = std::io::BufWriter::new(std::fs::File::create(&tmp).with_context(|| format!("creating {}", tmp.display()))?);
            f.write_all(&MAGIC)?;
            f.write_all(&(header.len() as u64).to_le_bytes())?;
            f.write_all(&header)?;
            f.write_all(&(index_bytes.len() as u64).to_le_bytes())?;
            f.write_all(&index_bytes)?;
            for b in &st.blobs {
                f.write_all(&b.compressed)?;
            }
            f.flush()?;
        }
        std::fs::rename(&tmp, path).with_context(|| format!("moving {} into place", path.display()))?;
        Ok(PackStats {
            records: names.len(),
            blobs: st.blobs.len(),
            raw_bytes: st.blobs.iter().map(|b| b.raw.len() as u64).sum(),
            stored_bytes: at,
            file_bytes: std::fs::metadata(path)?.len(),
        })
    }

    /// Writes the records as loose files under `dir`: `header.json` and `<name>.bin`
    /// (uncompressed). Existing `.bin` files there are removed first.
    pub fn write_loose(self, dir: &Path) -> Result<PackStats> {
        let (names, st) = self.sorted();
        if dir.exists() {
            remove_bins(dir)?;
        }
        std::fs::create_dir_all(dir)?;
        std::fs::write(dir.join("header.json"), serde_json::to_string_pretty(&self.header)?)?;
        let mut raw_bytes = 0;
        for (n, b) in &names {
            let p = dir.join(format!("{n}.bin"));
            if let Some(d) = p.parent() {
                std::fs::create_dir_all(d)?;
            }
            std::fs::write(&p, &st.blobs[*b].raw)?;
            raw_bytes += st.blobs[*b].raw.len() as u64;
        }
        Ok(PackStats { records: names.len(), blobs: names.len(), raw_bytes, stored_bytes: raw_bytes, file_bytes: raw_bytes })
    }
}

fn remove_bins(dir: &Path) -> Result<()> {
    for e in std::fs::read_dir(dir)? {
        let p = e?.path();
        if p.is_dir() {
            remove_bins(&p)?;
            let _ = std::fs::remove_dir(&p);
        } else if p.extension().is_some_and(|x| x == "bin") {
            std::fs::remove_file(&p)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------------------------

/// Somewhere records can be read from by name.
pub trait Source: Send + Sync {
    fn header(&self) -> &PackHeader;
    /// The record's serialized bytes, or `None` if this source doesn't have it.
    fn read(&self, name: &str) -> Result<Option<Vec<u8>>>;
    fn names(&self) -> Vec<String>;
    /// Where it was opened from, for messages.
    fn location(&self) -> String;
}

/// A pack file. The header and index are read at open; blobs are read and decompressed on
/// demand.
pub struct PackFile {
    path: PathBuf,
    header: PackHeader,
    index: HashMap<String, IndexEntry>,
    data_start: u64,
    file: Mutex<std::fs::File>,
}

impl PackFile {
    pub fn open(path: &Path) -> Result<PackFile> {
        let mut f = std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
        let mut magic = [0u8; 8];
        f.read_exact(&mut magic).context("reading pack magic")?;
        if magic != MAGIC {
            bail!("{} is not an asset pack", path.display());
        }
        let mut len = [0u8; 8];
        f.read_exact(&mut len)?;
        let mut header = vec![0u8; u64::from_le_bytes(len) as usize];
        f.read_exact(&mut header)?;
        let header: PackHeader = decode(&header).context("pack header")?;
        f.read_exact(&mut len)?;
        let mut index = vec![0u8; u64::from_le_bytes(len) as usize];
        f.read_exact(&mut index)?;
        let index: Vec<IndexEntry> = decode(&zstd::decode_all(&index[..])?).context("pack index")?;
        let data_start = f.stream_position()?;
        Ok(PackFile {
            path: path.to_path_buf(),
            header,
            index: index.into_iter().map(|e| (e.name.clone(), e)).collect(),
            data_start,
            file: Mutex::new(f),
        })
    }

    /// Reads only the header (to check a pack is current without opening it fully).
    pub fn read_header(path: &Path) -> Result<PackHeader> {
        let mut f = std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
        let mut magic = [0u8; 8];
        f.read_exact(&mut magic)?;
        if magic != MAGIC {
            bail!("{} is not an asset pack", path.display());
        }
        let mut len = [0u8; 8];
        f.read_exact(&mut len)?;
        let mut header = vec![0u8; u64::from_le_bytes(len) as usize];
        f.read_exact(&mut header)?;
        decode(&header)
    }
}

impl Source for PackFile {
    fn header(&self) -> &PackHeader {
        &self.header
    }
    fn read(&self, name: &str) -> Result<Option<Vec<u8>>> {
        let Some(e) = self.index.get(name) else { return Ok(None) };
        let mut buf = vec![0u8; e.len as usize];
        {
            let mut f = self.file.lock().unwrap();
            f.seek(SeekFrom::Start(self.data_start + e.offset))?;
            f.read_exact(&mut buf).with_context(|| format!("reading {name}"))?;
        }
        let raw = zstd::decode_all(&buf[..]).with_context(|| format!("decompressing {name}"))?;
        if raw.len() as u64 != e.raw_len {
            bail!("{name}: {} bytes after decompression, index says {}", raw.len(), e.raw_len);
        }
        Ok(Some(raw))
    }
    fn names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.index.keys().cloned().collect();
        v.sort();
        v
    }
    fn location(&self) -> String {
        self.path.display().to_string()
    }
}

/// A folder of loose records (`PackWriter::write_loose`).
pub struct LooseDir {
    root: PathBuf,
    header: PackHeader,
}

impl LooseDir {
    pub fn open(root: &Path) -> Result<LooseDir> {
        let h = root.join("header.json");
        let text = std::fs::read_to_string(&h).with_context(|| format!("reading {}", h.display()))?;
        Ok(LooseDir { root: root.to_path_buf(), header: serde_json::from_str(&text).context("header.json")? })
    }
}

impl Source for LooseDir {
    fn header(&self) -> &PackHeader {
        &self.header
    }
    fn read(&self, name: &str) -> Result<Option<Vec<u8>>> {
        let p = self.root.join(format!("{name}.bin"));
        match std::fs::read(&p) {
            Ok(b) => Ok(Some(b)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e).with_context(|| format!("reading {}", p.display())),
        }
    }
    fn names(&self) -> Vec<String> {
        fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) {
            let Ok(rd) = std::fs::read_dir(dir) else { return };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, root, out);
                } else if p.extension().is_some_and(|x| x == "bin")
                    && let Ok(rel) = p.with_extension("").strip_prefix(root)
                {
                    out.push(rel.to_string_lossy().replace('\\', "/"));
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.root, &self.root, &mut out);
        out.sort();
        out
    }
    fn location(&self) -> String {
        format!("{} (loose)", self.root.display())
    }
}

/// Sources layered by name: `get` looks in the last-added source first.
pub struct Assets {
    sources: Vec<Box<dyn Source>>,
}

impl Assets {
    /// Opens a pack file, or a loose folder if `path` is a directory.
    pub fn open(path: &Path) -> Result<Assets> {
        let s: Box<dyn Source> = if path.is_dir() { Box::new(LooseDir::open(path)?) } else { Box::new(PackFile::open(path)?) };
        Ok(Assets { sources: vec![s] })
    }

    pub fn from_source(source: Box<dyn Source>) -> Assets {
        Assets { sources: vec![source] }
    }

    /// Adds a source on top (a mod pack): its records win over the ones below.
    pub fn layer(&mut self, source: Box<dyn Source>) {
        self.sources.push(source);
    }

    /// The base pack's header.
    pub fn header(&self) -> &PackHeader {
        self.sources[0].header()
    }

    pub fn location(&self) -> String {
        self.sources.iter().map(|s| s.location()).collect::<Vec<_>>().join(" + ")
    }

    /// The record's bytes from the topmost source that has it.
    pub fn read(&self, name: &str) -> Result<Option<Vec<u8>>> {
        for s in self.sources.iter().rev() {
            if let Some(b) = s.read(name)? {
                return Ok(Some(b));
            }
        }
        Ok(None)
    }

    pub fn try_get<T: DeserializeOwned>(&self, name: &str) -> Result<Option<T>> {
        match self.read(name)? {
            Some(b) => decode(&b).with_context(|| format!("record {name}")).map(Some),
            None => Ok(None),
        }
    }

    pub fn get<T: DeserializeOwned>(&self, name: &str) -> Result<T> {
        self.try_get(name)?.with_context(|| format!("{} has no record {name}", self.location()))
    }

    pub fn contains(&self, name: &str) -> bool {
        self.sources.iter().any(|s| s.read(name).ok().flatten().is_some())
    }

    /// Every record name starting with `prefix`, across all sources, sorted.
    pub fn names(&self, prefix: &str) -> Vec<String> {
        let mut v: Vec<String> = self.sources.iter().flat_map(|s| s.names()).filter(|n| n.starts_with(prefix)).collect();
        v.sort();
        v.dedup();
        v
    }
}

/// Writes `bytes` to `path` whole: to a file beside it, then renamed into place, so a reader (or
/// a crash) never sees half of it. The folder is made if it's missing.
pub fn write_file_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(d) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(d).with_context(|| format!("making {}", d.display()))?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, bytes).with_context(|| format!("writing {}", tmp.display()))?;
    std::fs::rename(&tmp, path).with_context(|| format!("moving {} into place", path.display()))
}

/// The per-user data folder for `app`: `%LOCALAPPDATA%\<app>` on Windows,
/// `$XDG_DATA_HOME/<app>` (or `~/.local/share/<app>`) elsewhere, `~/Library/Application
/// Support/<app>` on macOS. Packs are caches that can be rebuilt, so Windows gets the local
/// (non-roaming) folder.
pub fn user_data_dir(app: &str) -> Option<PathBuf> {
    let env = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    let base = if cfg!(windows) {
        env("LOCALAPPDATA").or_else(|| env("APPDATA"))?
    } else if cfg!(target_os = "macos") {
        env("HOME")?.join("Library/Application Support")
    } else {
        env("XDG_DATA_HOME").or_else(|| env("HOME").map(|h| h.join(".local/share")))?
    };
    Some(base.join(app))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header() -> PackHeader {
        PackHeader { format_version: 3, importer: "test".into(), importer_version: 7, source_sha1: "ab".into(), info: BTreeMap::new() }
    }

    #[test]
    fn pack_and_loose_roundtrip_with_dedup_and_layering() {
        let dir = std::env::temp_dir().join(format!("eng_asset_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let w = PackWriter::new(header());
        w.put("a/one", &vec![1u32, 2, 3]).unwrap();
        w.put("a/same", &vec![1u32, 2, 3]).unwrap();
        w.put("b/two", &"hello".to_string()).unwrap();
        let stats = w.write(&dir.join("p.pak")).unwrap();
        assert_eq!((stats.records, stats.blobs), (3, 2));

        let a = Assets::open(&dir.join("p.pak")).unwrap();
        assert!(a.header().is_current(3, "test", 7, Some("AB")));
        assert!(!a.header().is_current(4, "test", 7, None));
        assert_eq!(a.get::<Vec<u32>>("a/same").unwrap(), vec![1, 2, 3]);
        assert_eq!(a.get::<String>("b/two").unwrap(), "hello");
        assert!(a.try_get::<String>("missing").unwrap().is_none());
        assert_eq!(a.names("a/"), vec!["a/one", "a/same"]);
        assert_eq!(PackFile::read_header(&dir.join("p.pak")).unwrap(), header());

        let w = PackWriter::new(header());
        w.put("b/two", &"modded".to_string()).unwrap();
        w.write_loose(&dir.join("loose")).unwrap();
        let mut a = Assets::open(&dir.join("p.pak")).unwrap();
        a.layer(Box::new(LooseDir::open(&dir.join("loose")).unwrap()));
        assert_eq!(a.get::<String>("b/two").unwrap(), "modded");
        assert_eq!(a.get::<Vec<u32>>("a/one").unwrap(), vec![1, 2, 3]);
        assert_eq!(a.names("b/"), vec!["b/two"]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
