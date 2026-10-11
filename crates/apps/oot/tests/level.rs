//! Custom levels entered by `Play_Init` (`Options::level_in_game`, the game's `--level`): a scene
//! of their own after the scene table's last, in their theme's scene's header, and reloaded in
//! place when the editor rebuilds them.

use std::path::{Path, PathBuf};

use eng_input::pad::PadState;
use oot::Options;
use oot_actors::PlayExt;
use oot_game::play::{PlayState, scripted_input};

fn pack_is_current() -> bool {
    // Only with a current pack: load_assets would import one otherwise.
    oot_game::pack::default_pack_path().is_ok_and(|p| oot_game::pack::is_current(&p, None))
}

/// A 2000-square floor at height `z` (the export's z up), themed as `scene`.
fn write_level(dir: &Path, scene: &str, z: f32) {
    write_level_playing(dir, scene, z, serde_json::Value::Null);
}

/// The same, with `game` (the export's: how it plays).
fn write_level_playing(dir: &Path, scene: &str, z: f32, game: serde_json::Value) {
    std::fs::create_dir_all(dir.join("textures")).unwrap();
    image::RgbaImage::from_pixel(2, 2, image::Rgba([120, 160, 90, 255])).save(dir.join("textures/g.png")).unwrap();
    let j = serde_json::json!({
        "name": "test_level",
        "scene": scene,
        "materials": [{"name": "ground", "file": "textures/g.png", "wrap_u": "repeat", "wrap_v": "repeat", "alpha": "opaque", "opacity": 1.0, "cull": "back"}],
        "surfaces": ["ground"],
        "objects": [{"name": "ground", "verts": [-1000, -1000, z, 1000, -1000, z, 1000, 1000, z, -1000, 1000, z], "tris": [0, 1, 2, 0, 2, 3],
            "uvs": [0, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1], "colors": [], "material": [0, 0], "surface": [0, 0]}],
        "game": game
    });
    std::fs::write(dir.join("level.json"), j.to_string()).unwrap();
}

fn temp_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("oot_level_in_game_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn enter(dir: &Path) -> (oot::Assets, PlayState) {
    let o = Options { level: Some(dir.to_path_buf()), level_in_game: true, time: "10:00".into(), child: true, ..Default::default() };
    let a = oot::load_assets(&o).expect("the assets");
    let w = oot::new_play(&a, true);
    (a, w)
}

#[test]
fn a_level_is_entered_as_a_scene_of_its_own_in_its_themes_header() {
    if !pack_is_current() {
        return;
    }
    let dir = temp_dir("zd");
    write_level(&dir, "spot07", 0.0);
    let (a, mut w) = enter(&dir);
    idle(&mut w, 3);
    let g = a.game.as_ref().expect("Play_Init's tables");
    let lg = a.level_game.as_ref().expect("the level's records");
    // a new id after the game's 110 scenes, its four entrance rows at the table's end
    assert_eq!(lg.entered.scene_id, 0x6E);
    assert_eq!(w.scene_id, lg.entered.scene_id);
    assert_eq!(g.scenes.entrances.len(), lg.entered.entrance as usize + 4);
    assert!(w.assets.is_some(), "entered by Play_Init");
    // Link at the level's middle, Navi with him, the HUD up
    assert!(w.player().actor.world_pos.distance(glam::Vec3::ZERO) < 1.0, "{:?}", w.player().actor.world_pos);
    assert!(w.player().navi_actor.is_some(), "Navi");
    assert!(w.interface_ctx.initialised);
    // Zora's Domain's music, and its HUD restrictions under the new id
    let zd = g.pack.scene("spot07").unwrap();
    assert_eq!(w.sequence_ctx.seq_id, zd.layers[0].sound.expect("its sound settings").seq_id);
    let row = |id: u16| g.interface.restrictions.iter().find(|r| r[0] as u16 == id).map(|r| r[1..].to_vec());
    let zd_id = g.scenes.by_file("spot07_scene").unwrap().id;
    assert_eq!(row(0x6E), row(zd_id));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_level_themed_as_the_crater_is_a_hot_room() {
    if !pack_is_current() {
        return;
    }
    let dir = temp_dir("crater");
    write_level(&dir, "spot17", 0.0);
    let (_a, mut w) = enter(&dir);
    idle(&mut w, 3);
    // ROOM_ENV_HOT: the heat timer's room, from Death Mountain Crater's first room
    assert_eq!(w.room_ctx.cur.behavior_type2, 3);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_rebuilt_level_reloads_with_link_in_place_and_the_music_carrying_on() {
    if !pack_is_current() {
        return;
    }
    let dir = temp_dir("reload");
    write_level(&dir, "spot04", 0.0);
    let (mut a, mut w) = enter(&dir);
    idle(&mut w, 10);
    let (seq, changes) = (w.save.seq_id, w.scene_changes);
    // the editor raises the floor 100
    write_level(&dir, "spot04", 100.0);
    w.place_player(glam::Vec3::new(300.0, 100.0, 200.0), 0);
    oot::reload_level(&mut a, &mut w).expect("the level reloaded");
    idle(&mut w, 10);
    assert_eq!(w.scene_changes, changes + 1);
    assert_eq!(w.save.seq_id, seq, "the same sequence, not restarted");
    let p = w.player().actor.world_pos;
    assert!((p.y - 100.0).abs() < 1.0 && (p.x - 300.0).abs() < 1.0, "on the raised floor where he was put: {p:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_game_block_changes_how_it_plays_and_the_theme_keeps_the_look() {
    if !pack_is_current() {
        return;
    }
    // Kokiri Forest's look, hot, silent, the sky off and the clock stopped
    let dir = temp_dir("hot");
    write_level_playing(&dir, "spot04", 0.0, serde_json::json!({"hot": true, "music": "none", "sky": false, "clock": false}));
    let (a, mut w) = enter(&dir);
    idle(&mut w, 3);
    assert_eq!(w.room_ctx.cur.behavior_type2, 3, "hot");
    assert_eq!(w.sequence_ctx.seq_id as u16, oot_game::audio::NA_BGM_NO_MUSIC);
    let g = a.game.as_ref().unwrap();
    let room = g.pack.room(&oot_game::pack::keys::room(oot_import::level::SCENE_FILE, 0, 0)).unwrap();
    assert!(room.skybox_disabled);
    assert_eq!(room.time.map(|t| t[2]), Some(0), "the clock stopped");
    let _ = std::fs::remove_dir_all(&dir);

    // Death Mountain Crater's dynamics (its music, its HUD rules) in Kokiri Forest's look
    let dir = temp_dir("start");
    write_level_playing(&dir, "spot04", 0.0, serde_json::json!({"start_from": "spot17"}));
    let (a, mut w) = enter(&dir);
    idle(&mut w, 3);
    let g = a.game.as_ref().unwrap();
    let (crater, kokiri) = (g.pack.scene("spot17").unwrap(), g.pack.scene("spot04").unwrap());
    assert_eq!(w.room_ctx.cur.behavior_type2, 3, "the crater's room is hot");
    assert_eq!(w.sequence_ctx.seq_id, crater.layers[0].sound.unwrap().seq_id);
    let level = g.pack.scene(oot_import::level::SCENE_FILE).unwrap();
    assert_eq!(level.layers[0].light_settings, kokiri.layers[0].light_settings, "the theme's lights and fog");
    assert_eq!(level.layers[0].skybox, kokiri.layers[0].skybox, "and its sky");
    let row = |id: u16| g.interface.restrictions.iter().find(|r| r[0] as u16 == id).map(|r| r[1..].to_vec());
    assert_eq!(row(0x6E), row(g.scenes.by_file("spot17_scene").unwrap().id));
    let _ = std::fs::remove_dir_all(&dir);
}
