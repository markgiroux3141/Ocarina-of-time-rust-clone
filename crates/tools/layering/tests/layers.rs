//! The crate layering rules (docs/adr/0002-crate-layout.md), checked on every workspace crate's
//! normal, dev and build dependencies as `cargo metadata` reports them.
//!
//! Layers, by folder under `crates/`:
//! - `engine/`: knows nothing about Zelda. Depends on engine crates only.
//! - `game/oot_game`: the game framework. Depends on the engine.
//! - `game/oot_actors`: content. Depends on the engine and `oot_game`.
//! - `import/`: reads the ROM and the decomp, and writes the asset pack of the game's records.
//!   Depends on the engine and the game. Nothing in the game depends on it: the runtime never
//!   reads the ROM or the decomp.
//! - `apps/`: anything below, and the `oot` client library.
//! - `tools/`: anything below except apps.
//!
//! On top of that, the GPU, windowing and device crates stay out of the game: only
//! `eng_render`, `eng_app`, apps and tools may use wgpu/eframe/egui, only `eng_input` (and
//! apps/tools) may use gilrs, only `eng_audio` (and apps/tools) may use cpal.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Layer {
    Engine,
    Import,
    Game,
    Content,
    Apps,
    Tools,
}

/// Dependencies that break the rules on purpose, each with the milestone that removes it.
/// (Milestone 1 had `oot_game` / `oot_actors` → `oot_import` here, until the asset pack.)
const TRANSITIONAL: &[(&str, &str, &str)] = &[];

const GPU_CRATES: &[&str] = &["wgpu", "eframe", "egui", "egui-wgpu", "egui_wgpu", "winit", "pollster"];
const GPU_ALLOWED: &[&str] = &["eng_render", "eng_app"];
const DEVICE_CRATES: &[&str] = &["gilrs"];
const DEVICE_ALLOWED: &[&str] = &["eng_input"];
const AUDIO_DEVICE_CRATES: &[&str] = &["cpal"];
const AUDIO_DEVICE_ALLOWED: &[&str] = &["eng_audio"];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(3).unwrap().to_path_buf()
}

fn layer_of(root: &Path, manifest: &Path) -> Option<Layer> {
    let rel = manifest.strip_prefix(root.join("crates")).ok()?;
    let top = rel.components().next()?.as_os_str().to_str()?;
    let name = rel.components().nth(1)?.as_os_str().to_str()?;
    Some(match (top, name) {
        ("engine", _) => Layer::Engine,
        ("import", _) => Layer::Import,
        ("game", "oot_game") => Layer::Game,
        ("game", _) => Layer::Content,
        ("apps", _) => Layer::Apps,
        ("tools", _) => Layer::Tools,
        _ => return None,
    })
}

fn allowed(from: Layer, to: Layer) -> bool {
    use Layer::*;
    match from {
        Engine => to == Engine,
        Import => matches!(to, Engine | Game | Content),
        Game => to == Engine,
        Content => matches!(to, Engine | Game),
        Apps => to != Tools,
        Tools => to != Apps,
    }
}

struct Package {
    name: String,
    layer: Layer,
    /// (dependency name, kind)
    deps: Vec<(String, String)>,
}

fn packages() -> Vec<Package> {
    let root = workspace_root();
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(&root)
        .output()
        .expect("running cargo metadata");
    assert!(out.status.success(), "cargo metadata failed: {}", String::from_utf8_lossy(&out.stderr));
    let meta: Value = serde_json::from_slice(&out.stdout).expect("cargo metadata JSON");
    let mut result = Vec::new();
    for p in meta["packages"].as_array().unwrap() {
        let name = p["name"].as_str().unwrap().to_string();
        let manifest = PathBuf::from(p["manifest_path"].as_str().unwrap());
        let layer = layer_of(&root, &manifest).unwrap_or_else(|| panic!("{name} ({}) is outside the layer folders", manifest.display()));
        let deps = p["dependencies"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| (d["name"].as_str().unwrap().to_string(), d["kind"].as_str().unwrap_or("normal").to_string()))
            .collect();
        result.push(Package { name, layer, deps });
    }
    result
}

#[test]
fn crates_respect_the_layers() {
    let pkgs = packages();
    let layers: BTreeMap<&str, Layer> = pkgs.iter().map(|p| (p.name.as_str(), p.layer)).collect();
    let mut violations = Vec::new();
    let mut used_exceptions = Vec::new();
    for p in &pkgs {
        for (dep, kind) in &p.deps {
            if let Some(&to) = layers.get(dep.as_str()) {
                if allowed(p.layer, to) {
                    continue;
                }
                if TRANSITIONAL.iter().any(|(a, b, _)| *a == p.name && b == dep) {
                    used_exceptions.push((p.name.clone(), dep.clone()));
                    continue;
                }
                violations.push(format!("{} ({:?}) -> {} ({:?}) [{kind}]", p.name, p.layer, dep, to));
            }
            let in_game = matches!(p.layer, Layer::Engine | Layer::Import | Layer::Game | Layer::Content);
            if in_game && GPU_CRATES.contains(&dep.as_str()) && !GPU_ALLOWED.contains(&p.name.as_str()) {
                violations.push(format!("{} uses {dep} (GPU and windowing crates stay in eng_render / eng_app)", p.name));
            }
            if in_game && DEVICE_CRATES.contains(&dep.as_str()) && !DEVICE_ALLOWED.contains(&p.name.as_str()) {
                violations.push(format!("{} uses {dep} (input devices stay in eng_input)", p.name));
            }
            if in_game && AUDIO_DEVICE_CRATES.contains(&dep.as_str()) && !AUDIO_DEVICE_ALLOWED.contains(&p.name.as_str()) {
                violations.push(format!("{} uses {dep} (the audio device stays in eng_audio)", p.name));
            }
        }
    }
    assert!(violations.is_empty(), "layering violations:\n  {}", violations.join("\n  "));
    // An exception that's no longer needed should be deleted, so the rule tightens.
    for (a, b, why) in TRANSITIONAL {
        assert!(used_exceptions.iter().any(|(x, y)| x == a && y == b), "transitional exception {a} -> {b} ({why}) is unused: remove it");
    }
}

#[test]
fn engine_crates_are_named_eng() {
    for p in packages() {
        if p.layer == Layer::Engine {
            assert!(p.name.starts_with("eng_"), "{} is in crates/engine but not named eng_*", p.name);
        } else {
            assert!(!p.name.starts_with("eng_"), "{} is named eng_* but lives outside crates/engine", p.name);
        }
    }
}
