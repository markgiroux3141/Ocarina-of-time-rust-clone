//! The export carries how the level plays (`Doc::game`) to the game, its regions as their game
//! scenes (`oot_import::level::write_scene` reads it).

use overworld::doc::Game;
use overworld::{build, export, Doc, Theme};
use std::path::Path;

#[test]
fn the_game_block_is_exported_with_its_regions_as_scenes() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/sketch");
    let mut doc: Doc = serde_json::from_str(&std::fs::read_to_string(examples.join("sketch_village.json")).unwrap()).unwrap();
    let theme = Theme::for_doc(&doc, None).unwrap();
    let lvl = build(&doc, &theme).unwrap();
    let out = std::env::temp_dir().join(format!("overworld_export_game_{}", std::process::id()));
    let read = |out: &Path| -> serde_json::Value { serde_json::from_str(&std::fs::read_to_string(out.join("level.json")).unwrap()).unwrap() };

    // nothing set: no block, and the document saves as before
    export::write(&doc, &theme, &lvl, None, &out).unwrap();
    assert!(read(&out).get("game").is_none());
    assert!(!serde_json::to_string(&doc).unwrap().contains("\"game\""));

    doc.game = Game { start_from: "dm_crater".into(), music: "none".into(), hot: Some(false), sky: None, clock: Some(true) };
    let problems = export::write(&doc, &theme, &lvl, None, &out).unwrap();
    let g = &read(&out)["game"];
    assert_eq!(g["start_from"], "spot17", "{problems:?}");
    assert_eq!((g["music"].as_str(), g["hot"].as_bool(), g["clock"].as_bool()), (Some("none"), Some(false), Some(true)));
    assert!(g.get("sky").is_none());

    // a region that isn't one is a problem, not a scene
    doc.game = Game { music: "nowhere".into(), ..Default::default() };
    let problems = export::write(&doc, &theme, &lvl, None, &out).unwrap();
    assert!(problems.iter().any(|p| p.contains("nowhere")), "{problems:?}");
    assert!(read(&out)["game"].get("music").is_none());
    let _ = std::fs::remove_dir_all(&out);
}
