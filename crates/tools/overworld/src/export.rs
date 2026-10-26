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
    // the game lights the level with its theme's scene's environment (its lights and fog)
    if let Some(s) = crate::kit::scene(&theme.name).and_then(|s| s.game_scene()) {
        j["scene"] = s.into();
    }
    // how it plays: the regions as their game scenes (oot_import::level::write_scene)
    let g = &doc.game;
    if !g.is_default() {
        let region = |r: &str| crate::kit::scene(r).and_then(|s| s.game_scene());
        let mut jg = serde_json::json!({});
        if let Some(s) = (!g.start_from.is_empty()).then(|| region(&g.start_from)).flatten() {
            jg["start_from"] = s.into();
        } else if !g.start_from.is_empty() {
            problems.push(format!("game: no region {:?} to start from", g.start_from));
        }
        match g.music.as_str() {
            "" => {}
            "none" => jg["music"] = "none".into(),
            r => match region(r) {
                Some(s) => jg["music"] = s.into(),
                None => problems.push(format!("game: no region {r:?} for the music")),
            },
        }
        for (k, v) in [("hot", g.hot), ("sky", g.sky), ("clock", g.clock)] {
            if let Some(v) = v {
                jg[k] = v.into();
            }
        }
        if !g.objects.is_empty() {
            jg["objects"] = g.objects.clone().into();
        }
        j["game"] = jg;
        j["problems"] = problems.clone().into();
    }
    // the actors, each at its height as built (oot_import::level reads them into the room)
    if !doc.actors.is_empty() || doc.storeys.iter().any(|s| !s.actors.is_empty()) {
        j["actors"] = actors(doc, lvl, &mut problems).into();
        j["problems"] = problems.clone().into();
    }
    // its own texts, as the editor keeps them (oot_import::level encodes them for the game)
    if !doc.texts.is_empty() {
        j["texts"] = doc.texts.iter().map(|t| serde_json::json!({"id": t.id, "type": t.box_type, "pos": t.box_pos, "text": t.text})).collect::<Vec<_>>().into();
    }
    let (obj, mtl) = lvl.mesh.to_obj("level.mtl", &tex);
    // the MTL first and the OBJ last: a viewer reloads on the OBJ
    put(&out.join("level.json"), serde_json::to_string(&j).unwrap())?;
    put(&out.join("level.mtl"), mtl)?;
    put(&out.join("level.obj"), obj)?;
    Ok(problems)
}

/// Where an actor stands: its own height, else the highest floor under it as built (the water's
/// surface where there's no floor), else nowhere.
pub fn actor_z(actor: &crate::doc::Actor, ground: &crate::props::Ground, lvl: &Level) -> Option<f64> {
    actor.z.or_else(|| {
        let (floor, water) = ground.at(&lvl.mesh, actor.at);
        floor.or(water)
    })
}

/// The actors as `level.json` has them: the overlay's name, the position (x east, y north, z up),
/// the game's rotation and params, and the layers it's in. With storeys, every storey's, each
/// standing on its own storey's floor.
fn actors(doc: &Doc, lvl: &Level, problems: &mut Vec<String>) -> Vec<serde_json::Value> {
    let stack: Vec<Option<usize>> = if doc.storeys.is_empty() { vec![None] } else { doc.stack().iter().map(|s| s.at).collect() };
    let mut out = vec![];
    for at in stack {
        let ground = crate::props::Ground::new_in(&lvl.mesh, lvl.band(at));
        let (actors, of) = match at {
            None => (&doc.actors, if doc.storeys.is_empty() { String::new() } else { format!("{}: ", doc.storey_name(None)) }),
            Some(i) => (&doc.storeys[i].actors, format!("{}: ", doc.storey_name(at))),
        };
        for (i, a) in actors.iter().enumerate() {
            let z = actor_z(a, &ground, lvl).unwrap_or_else(|| {
                problems.push(format!("{of}actor {i} ({}) stands outside the level near ({:.0}, {:.0})", a.actor, a.at[0], a.at[1]));
                0.0
            });
            let mut j = serde_json::json!({
                "actor": a.actor,
                "pos": [a.at[0], a.at[1], z],
                "rot": a.rot,
                "params": a.params,
            });
            if let Some(ls) = a.layers {
                j["layers"] = ls.to_vec().into();
            }
            out.push(j);
        }
    }
    out
}

fn put(path: &Path, text: String) -> Result<(), String> {
    let tmp = path.with_file_name(format!("{}.tmp", path.file_name().unwrap().to_string_lossy()));
    std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}
