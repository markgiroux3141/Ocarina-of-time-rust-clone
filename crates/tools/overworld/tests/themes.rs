//! Every built-in theme builds the example levels, and every texture it draws with is in the
//! regions' libraries. Needs the libraries (`overworld kit-textures`, made from the git-ignored
//! extract): skipped without them.

use overworld::textures::Library;
use overworld::theme::BUILTIN;
use overworld::{build, Doc, Theme};
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

#[test]
fn every_builtin_theme_builds_the_examples_with_textures_that_exist() {
    let dirs: Vec<PathBuf> = overworld::kit::REGIONS.iter().map(|r| repo().join("out/overworld/textures").join(r)).filter(|d| d.join("textures.json").exists()).collect();
    let themed: Vec<&str> = BUILTIN.iter().copied().filter(|t| repo().join("out/overworld/textures").join(t).join("textures.json").exists()).collect();
    if themed.len() < BUILTIN.len() {
        eprintln!("skipping: only {} of {} theme libraries made (overworld kit-textures)", themed.len(), BUILTIN.len());
        return;
    }
    let lib = Library::load_all(&dirs).unwrap();
    let examples = repo().join("crates/tools/overworld/examples/sketch");
    let mut bad = vec![];
    for name in BUILTIN {
        // each theme has a region (its label, its library)
        assert!(overworld::kit::REGIONS.contains(&name), "{name}");
        for ex in ["sketch_village.json", "sketch_profiles.json", "sketch_rocks.json", "sketch_sections.json"] {
            let mut doc: Doc = serde_json::from_str(&std::fs::read_to_string(examples.join(ex)).unwrap()).unwrap();
            doc.settings.theme = name.to_string();
            let theme = Theme::for_doc(&doc, None).unwrap();
            assert_eq!(theme.name, name);
            let lvl = build(&doc, &theme).unwrap_or_else(|e| panic!("{name} {ex}: {e}"));
            for m in &lvl.mesh.materials {
                for t in std::iter::once(theme.texture_name(m)).chain(theme.overlay_texture(m)) {
                    if lib.rgba(&t).is_none() {
                        bad.push(format!("{name} {ex}: {m} -> {t}"));
                    }
                }
            }
        }
    }
    bad.dedup();
    assert!(bad.is_empty(), "textures not in any library:\n{}", bad.join("\n"));
}
