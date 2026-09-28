//! Every file in the ROM's file table, dumped as-is (decompressed) under its decomp name.

use std::path::Path;

use anyhow::Result;
use oot_import::project::Project;
use serde_json::json;

pub fn extract(p: &Project, out: &Path) -> Result<serde_json::Value> {
    let dir = out.join("raw");
    std::fs::create_dir_all(&dir)?;
    let mut files = Vec::new();
    let mut bytes = 0usize;
    for (i, e) in p.rom.files.iter().enumerate() {
        if !e.is_present() {
            continue;
        }
        let name = p.rom.name_of(i).map(crate::sanitize).unwrap_or_else(|| format!("file_{i:04}"));
        match p.rom.file(i) {
            Ok(data) => {
                std::fs::write(dir.join(format!("{name}.bin")), &data)?;
                bytes += data.len();
                files.push(json!({ "index": i, "name": name, "vrom": format!("0x{:08X}", e.vrom_start), "size": data.len() }));
            }
            Err(err) => log::warn!("raw: file {i} ({name}): {err:#}"),
        }
    }
    crate::write_json(&dir.join("files.json"), &files)?;
    Ok(json!({ "files": files.len(), "bytes": bytes }))
}
