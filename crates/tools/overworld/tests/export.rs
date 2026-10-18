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

    doc.game = Game { start_from: "dm_crater".into(), music: "none".into(), hot: Some(false), sky: None, clock: Some(true), objects: vec![] };
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

#[test]
fn actors_are_exported_standing_on_the_level() {
    use overworld::doc::Actor;
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/sketch");
    let mut doc: Doc = serde_json::from_str(&std::fs::read_to_string(examples.join("sketch_village.json")).unwrap()).unwrap();
    let theme = Theme::for_doc(&doc, None).unwrap();
    let lvl = build(&doc, &theme).unwrap();
    let out = std::env::temp_dir().join(format!("overworld_export_actors_{}", std::process::id()));
    let read = |out: &Path| -> serde_json::Value { serde_json::from_str(&std::fs::read_to_string(out.join("level.json")).unwrap()).unwrap() };

    // none: no list, and the document saves as before
    export::write(&doc, &theme, &lvl, None, &out).unwrap();
    assert!(read(&out).get("actors").is_none());
    assert!(!serde_json::to_string(&doc).unwrap().contains("\"actors\""));

    // a pot on the ground, a raised sign facing west by night only, one off the level
    let ground = overworld::props::Ground::new(&lvl.mesh);
    let at = [200.0, 300.0];
    let floor = ground.at(&lvl.mesh, at).0.expect("a floor in the village");
    let mut sign = Actor::new("En_Kanban", [0.0, 0.0]);
    sign.z = Some(floor + 40.0);
    sign.set_yaw(90.0);
    sign.params = 0x0300;
    sign.layers = Some([false, true, false, true]);
    doc.actors = vec![Actor::new("Obj_Tsubo", at), sign, Actor::new("En_Kusa", [90000.0, 0.0])];
    let problems = export::write(&doc, &theme, &lvl, None, &out).unwrap();
    let a = &read(&out)["actors"];
    assert_eq!(a.as_array().unwrap().len(), 3);
    assert_eq!(a[0]["actor"], "Obj_Tsubo");
    assert_eq!(a[0]["pos"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect::<Vec<_>>(), vec![200.0, 300.0, floor]);
    assert!(a[0].get("layers").is_none());
    assert_eq!((a[1]["pos"][2].as_f64(), a[1]["params"].as_u64()), (Some(floor + 40.0), Some(0x0300)));
    // west: the game's y rotation 0xC000 (0 faces south)
    assert_eq!(a[1]["rot"][1].as_i64(), Some(0xC000u16 as i16 as i64));
    assert_eq!(a[1]["layers"], serde_json::json!([false, true, false, true]));
    assert!(problems.iter().any(|p| p.starts_with("actor 2 (En_Kusa) stands outside the level")), "{problems:?}");
    // the document keeps them
    let back: Doc = serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
    assert_eq!(back.actors, doc.actors);
    assert_eq!(back.actors[1].yaw(), 90.0);
    let _ = std::fs::remove_dir_all(&out);
}
