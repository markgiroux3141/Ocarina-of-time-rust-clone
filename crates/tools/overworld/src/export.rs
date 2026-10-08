//! Writing a built level to a folder: `level.json` (meshes with materials and collision
//! surfaces), `level.obj` + `level.mtl`, and the textures it uses, copied from a texture library
//! into `textures/`. Used by the CLI and the editor.
//!
//! Files are written to a temporary name and renamed, so a viewer watching the folder (pd-walk
//! reloads when `level.obj` changes) never reads half a file.

use crate::textures::{Library, TexInfo};
use crate::{Doc, Level, Theme};
use std::path::Path;

/// Writes the level and returns its problems: the build's, plus missing textures.
pub fn write(doc: &Doc, theme: &Theme, lvl: &Level, lib: Option<&Library>, out: &Path) -> Result<Vec<String>, String> {
    std::fs::create_dir_all(out.join("textures")).map_err(|e| format!("{}: {e}", out.display()))?;
    let tex = |role: &str| -> (String, TexInfo) {
        let name = theme.texture_name(role);
        let info = lib.map(|l| l.get(&name)).unwrap_or_else(|| TexInfo::plain(&name));
        (name, info)
    };
    let mut problems = lvl.problems.clone();
    if let Some(l) = lib {
        for m in &lvl.mesh.materials {
            let (name, info) = tex(m);
            let src = l.path(&name);
            let dst = out.join("textures").join(&info.file);
            if name.contains('@') {
                // a derived texture (a wall's middle rows): made from its base
                match l.png(&name) {
                    Some(bytes) => {
                        if std::fs::read(&dst).ok().as_ref() != Some(&bytes) && std::fs::write(&dst, &bytes).is_err() {
                            problems.push(format!("texture {name}: can't write {}", dst.display()));
                        }
                    }
                    None => problems.push(format!("texture {name}: its base isn't in {}", l.dir.display())),
                }
                continue;
            }
            let same = std::fs::metadata(&src).ok().zip(std::fs::metadata(&dst).ok()).is_some_and(|(a, b)| {
                a.len() == b.len() && a.modified().ok() <= b.modified().ok()
            });
            if !same && std::fs::copy(&src, &dst).is_err() {
                problems.push(format!("texture {name} ({}) not found", src.display()));
            }
        }
    } else {
        problems.push("no texture library: the MTL names textures/<texture>.png, which aren't there".into());
    }
    let mut j = lvl.mesh.to_json(&tex);
    // blend materials (the ground under a dirt path) draw a second texture by vertex weight
    for (i, m) in lvl.mesh.materials.iter().enumerate() {
        let Some(name2) = theme.overlay_texture(m) else { continue };
        let Some(l) = lib else { continue };
        let info2 = l.get(&name2);
        let dst = out.join("textures").join(&info2.file);
        match l.png(&name2) {
            Some(bytes) => {
                if std::fs::read(&dst).ok().as_ref() != Some(&bytes) && std::fs::write(&dst, &bytes).is_err() {
                    problems.push(format!("texture {name2}: can't write {}", dst.display()));
                }
            }
            None => problems.push(format!("texture {name2}: its parts aren't in {}", l.dir.display())),
        }
        let jm = &mut j["materials"][i];
        jm["texture2"] = name2.clone().into();
        jm["file2"] = format!("textures/{}", info2.file).into();
        jm["blend"] = "vertex".into();
    }
    j["name"] = doc.name.clone().into();
    j["theme"] = theme.name.clone().into();
    j["problems"] = problems.clone().into();
    j["rim"] = vec![lvl.rim.0, lvl.rim.1].into();
    let (obj, mtl) = lvl.mesh.to_obj("level.mtl", &tex);
    // the MTL first and the OBJ last: a viewer reloads on the OBJ
    put(&out.join("level.json"), serde_json::to_string(&j).unwrap())?;
    put(&out.join("level.mtl"), mtl)?;
    put(&out.join("level.obj"), obj)?;
    Ok(problems)
}

fn put(path: &Path, text: String) -> Result<(), String> {
    let tmp = path.with_file_name(format!("{}.tmp", path.file_name().unwrap().to_string_lossy()));
    std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}
