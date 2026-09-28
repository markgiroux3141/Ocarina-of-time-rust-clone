//! The asset pack against the ROM path: every record the game reads is compared with what the
//! spikes read from the ROM and the decomp's C, and the ported draw configs with the C
//! interpreter (`drawcfg`). These are the oracles for the pack (docs/adr/0008-asset-pack.md).
//! Skips without `oot.toml` or without a pack (`oot import` builds it).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use eng_math::Tables;
use oot_game::data::GameData;
use oot_game::env::{EnvTables, clock_time, scene_times};
use oot_game::pack::{GamePack, keys};
use oot_game::player_lib::{Age, Loadout, PlayerRules};
use oot_game::scene::{GAME_LAYERS, layer_for};
use oot_game::scene_table::{self, DrawConfigState};
use oot_import::drawcfg::State;
use oot_import::pack::{bake_state, layer_records};
use oot_import::player::{LoadPlayerRules, PlayerModel};
use oot_import::project::Project;
use oot_import::room::{SceneDraw, SceneTables};
use oot_import::tables::{LoadActorTable, LoadEnvTables, LoadGameData, LoadMathTables};
use oot_import::z64::CollisionCodec;

struct Ctx {
    p: Project,
    pack: GamePack,
    tables: SceneTables,
}

fn ctx() -> Option<&'static Ctx> {
    static C: OnceLock<Option<Ctx>> = OnceLock::new();
    C.get_or_init(|| {
        let p = match Project::open_default() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("skipping: {e:#}");
                return None;
            }
        };
        let pack = match GamePack::open_default() {
            Ok(k) => k,
            Err(e) => {
                eprintln!("skipping: {e:#}");
                return None;
            }
        };
        let tables = SceneTables::load(&p.config.decomp).expect("scene tables");
        Some(Ctx { p, pack, tables })
    })
    .as_ref()
}

#[test]
fn pack_is_for_this_rom() {
    let Some(c) = ctx() else { return };
    let h = c.pack.header();
    assert_eq!(h.source_sha1, oot_import::pack::rom_sha1(&c.p.config.rom).unwrap());
    assert!(oot_game::pack::is_current(&oot_game::pack::default_pack_path().unwrap(), Some(&h.source_sha1)));
    if let Some(commit) = oot_import::pack::decomp_commit(&c.p.config.decomp) {
        assert_eq!(h.info.get("decomp_commit"), Some(&commit));
    }
}

#[test]
fn tables_match_the_c() {
    let Some(c) = ctx() else { return };
    let decomp = &c.p.config.decomp;
    // sintable / sATan2Tbl.
    let math: Tables = c.pack.assets.get(keys::MATH).unwrap();
    assert_eq!(math, Tables::load(decomp).unwrap());
    // Player: REGs, sAgeProperties, the animation tables, items, the camera data, foot IK, the
    // target ranges, Link's rigs, and every animation's frames.
    let want = GameData::load(&c.p).unwrap();
    let got = c.pack.game_data().unwrap();
    assert_eq!(got, want);
    assert_eq!(got.anims.len(), 573);
    assert_eq!(got.anims, want.anims);
    assert_eq!(got.by_name, want.by_name);
    // z_player_lib.c's draw rules, z_kankyo.c's light configs.
    assert_eq!(c.pack.player_rules().unwrap(), PlayerRules::load(decomp).unwrap());
    assert_eq!(c.pack.env_tables().unwrap(), EnvTables::load(decomp).unwrap());
    // scene_table.h / object_table.h.
    let st = c.pack.scene_table().unwrap();
    assert_eq!(st.scenes.len(), c.tables.scenes.len());
    for (a, b) in st.scenes.iter().zip(&c.tables.scenes) {
        assert_eq!((a.id as usize, &a.file, &a.enum_name, &a.draw_config), (b.id, &b.file, &b.enum_name, &b.draw_config));
    }
    assert_eq!(st.objects, c.tables.objects);
    // entrance_table.h, actor_table.h with every overlay's ActorInit.
    assert_eq!(st.entrances, c.tables.entrances);
    assert_eq!(c.pack.actor_table().unwrap(), oot_game::actor_table::ActorTable::load(decomp).unwrap());
}

/// Every state a ported draw config is compared in: both ages, day and night, the four game
/// layers and two cutscene layers, several times of day, and the event flag. `roomCtx.unk_74`
/// starts at 0 each frame, because the interpreter keeps no state between runs.
fn draw_config_states() -> Vec<DrawConfigState> {
    let mut out = Vec::new();
    for layer in [0usize, 1, 2, 3, 4, 6] {
        for time in [clock_time(0, 0), clock_time(6, 15), clock_time(7, 0), clock_time(10, 0), clock_time(18, 30), clock_time(18, 31), clock_time(23, 0)] {
            for event in [false, true] {
                out.push(DrawConfigState {
                    gameplay_frames: 0,
                    child: layer == 0 || layer == 1 || layer >= 4,
                    night: layer == 1 || layer == 3,
                    scene_layer: layer,
                    day_time: time as u16,
                    room_unk_74: [0, 0],
                    event_chk_inf_07: event,
                });
            }
        }
    }
    out
}

#[test]
fn ported_draw_configs_match_the_c_interpreter() {
    let Some(c) = ctx() else { return };
    // One scene per ported draw config.
    let scenes: BTreeMap<&str, &str> = [("SDC_SPOT04", "spot04_scene"), ("SDC_SPOT00", "spot00_scene"), ("SDC_YDAN", "ydan_scene"), ("SDC_DEFAULT", "ganon_scene")].into_iter().collect();
    for sdc in scene_table::ported() {
        let file = scenes[sdc];
        let def = c.tables.scene(file).unwrap();
        assert_eq!(def.draw_config, sdc, "{file}");
        let sd = SceneDraw::load(&c.p, &c.tables, file, 0).unwrap();
        let mut compared = 0;
        for st in draw_config_states() {
            for f in [0u32, 1, 2, 5, 40, 127, 128, 129, 300, 1000, 4095, 70000] {
                let mut port = DrawConfigState { gameplay_frames: f, ..st.clone() };
                let got = scene_table::segment_values(sdc, &mut port).unwrap();
                // The event flag isn't something the interpreter knows about (it reads 0), so
                // compare only the states it can express.
                if st.event_chk_inf_07 {
                    continue;
                }
                let mut want = sd.segment_values(
                    &c.p,
                    &c.tables,
                    &State { gameplay_frames: f, child: st.child, night: st.night, scene_layer: st.scene_layer as i64, day_time: st.day_time },
                );
                // Scene_DrawConfigSpot00 after 18:30 runs Math_StepToS(&roomCtx.unk_74[0], 255, 5)
                // before using it as the prim alpha. The interpreter doesn't run function calls
                // on game state (unk_74 reads 0 throughout), so it gives alpha 0 where the C,
                // and the port, give 5 on the first frame.
                if sdc == "SDC_SPOT00" && st.day_time as i32 > clock_time(18, 30) {
                    assert_eq!(want[1].prim[0xA], Some([255, 255, 255, 0]));
                    assert_eq!(got[1].prim[0xA], Some([255, 255, 255, 5]), "frame {f}");
                    want[1].prim[0xA] = got[1].prim[0xA];
                }
                assert_eq!(got, want, "{sdc}: frame {f}, state {st:?}");
                compared += 1;
            }
        }
        assert!(compared > 400, "{sdc}: {compared} comparisons");
    }
}

#[test]
fn room_meshes_match_the_rom_path() {
    let Some(c) = ctx() else { return };
    // The scenes the golden renders use, in every game layer: the pack's rooms and collision
    // equal what the spikes built from the ROM for that layer at the bake time.
    for file in ["spot04_scene", "spot00_scene", "ydan_scene"] {
        let scene = c.pack.scene(file).unwrap();
        assert_eq!(scene.layers.len(), GAME_LAYERS);
        for layer in 0..GAME_LAYERS {
            let (ld, rooms, collision) = layer_records(&c.p, &c.tables, file, layer).unwrap();
            assert_eq!(scene.layers[layer], ld, "{file} layer {layer}");
            assert_eq!(c.pack.collision(&ld.collision).unwrap(), collision, "{file} layer {layer} collision");
            for (ri, r) in rooms.iter().enumerate() {
                assert_eq!(&c.pack.room(&keys::room(file, layer, ri)).unwrap(), r, "{file} layer {layer} room {ri}");
            }
            // The spike loaded a scene with the state from the requested time: for the day
            // layers at 10:00 that is the bake state itself; at night the bake time is
            // midnight, and the meshes built for 19:00 are the same for these draw configs.
            let sd = SceneDraw::load(&c.p, &c.tables, file, layer).unwrap();
            let night = layer == 1 || layer == 3;
            let t = clock_time(if night { 19 } else { 10 }, 0) as u16;
            let (day, _, _) = scene_times(t, sd.rooms.first().and_then(|r| r.time));
            let spike_state = State { gameplay_frames: 0, child: layer < 2, night, scene_layer: layer as i64, day_time: day };
            let mut notes = BTreeSet::new();
            let spike = sd.build(&c.p, &c.tables, &spike_state, &mut notes);
            for m in &spike {
                let r = &rooms[m.index];
                assert_eq!(r.entries.len(), m.entries.len());
                for (e, want) in r.entries.iter().zip(&m.entries) {
                    assert_eq!((&e.opa, &e.xlu), (&want.opa, &want.xlu), "{file} layer {layer} room {} at 19:00 vs the bake", m.index);
                }
            }
            assert_eq!(layer_for(layer < 2, night), layer);
            let _ = bake_state(layer, None);
        }
    }
}

#[test]
fn link_meshes_match_interpreting_the_rom() {
    let Some(c) = ctx() else { return };
    let rules = PlayerRules::load(&c.p.config.decomp).unwrap();
    for age in [Age::Adult, Age::Child] {
        let model = PlayerModel::load(&c.p, &rules, age).unwrap();
        let faces = c.pack.link_faces(age).unwrap();
        assert_eq!(c.pack.link_skeleton(age).unwrap().limbs.len(), model.skeleton.limbs.len());
        for (gi, g) in rules.model_groups.iter().enumerate() {
            for fists in [false, true] {
                // The default loadout, no shield, and (for the child) no sword on B.
                for (shield, sword) in [(Loadout::default_for(&rules, age).shield, true), (0, true), (1, false)] {
                    let lo = Loadout { model_group: gi, moving_fast: fists, shield, child_has_kokiri_sword: sword, ..Loadout::default_for(&rules, age) };
                    let v = c.pack.link_variant(age, &rules.limb_dlists(&lo, 0)).unwrap();
                    for (eye, mouth) in [(0, 0), (1, 0), (2, 3), (5, 1)] {
                        let (want, _) = model.draw_list(&rules, &lo, eye, mouth, 0).unwrap();
                        assert!(v.with_face(&faces, eye, mouth) == want, "{} {} fists={fists} shield {shield} sword {sword} eye {eye} mouth {mouth}", age.name(), g.name);
                    }
                }
            }
        }
    }
}

#[test]
fn object_records_match_the_rom() {
    let Some(c) = ctx() else { return };
    // Bg_Ydan_Hasi's platform: the collision header, and the mesh the spike interpreted with
    // segments 0 (code), 4 (gameplay_keep) and 6 (the object) after setup DL 25.
    let f = c.p.symbols.file("object_ydan_objects").unwrap();
    let data = c.p.rom.file_by_name("object_ydan_objects").unwrap();
    let col = f.find("gDTSlidingPlatformCol").unwrap();
    let want = eng_collision::collision::CollisionHeader::parse(&data, 6, col.offset as usize).unwrap();
    assert_eq!(c.pack.collision(&keys::collision("object_ydan_objects", "gDTSlidingPlatformCol")).unwrap(), want);
    let dl = f.find("gDTSlidingPlatformDL").unwrap();
    let mut segments: [Option<eng_gbi::gbi::Segment>; 16] = Default::default();
    segments[6] = Some(eng_gbi::gbi::Segment::Data { buf: data.clone(), base: 0 });
    segments[4] = Some(eng_gbi::gbi::Segment::Data { buf: c.p.rom.file_by_name("gameplay_keep").unwrap(), base: 0 });
    segments[0] = oot_import::room::code_ram_image(&c.p).map(|b| eng_gbi::gbi::Segment::Data { buf: b, base: 0 });
    let spike = oot_import::room::run_dls(&oot_import::room::BufferSegments { segments, pre: Vec::new(), dynamic: 0 }, &[0x0600_0000 | dl.offset]);
    let mesh = c.pack.mesh("object_ydan_objects", "gDTSlidingPlatformDL").unwrap();
    assert!(mesh == spike, "gDTSlidingPlatformDL");
    // Textures: the pack's pixels and texels are the decode of the ROM's bytes.
    let files = oot_import::objects::Files::new(&c.p);
    for file in ["object_link_boy", "gameplay_keep", "spot04_scene"] {
        let f = c.p.symbols.file(file).unwrap();
        let data = c.p.rom.file_by_name(file).unwrap();
        for t in f.of_kind("Texture").take(20) {
            let d = oot_import::objects::decode_texture(f, &data, t, &files).unwrap();
            let tex = c.pack.texture(file, &t.name).unwrap();
            assert_eq!((tex.width, tex.height, &tex.rgba, &tex.texels), (d.width, d.height, &d.image.rgba, &d.texels), "{file} / {}", t.name);
        }
    }
}

#[test]
fn the_course_is_its_binary_round_trip() {
    // The course used to go through encode/parse at build time; now it builds the header
    // directly, which must be what the round trip gives.
    let c = oot_game::course::build();
    let back = eng_collision::collision::CollisionHeader::parse(&c.collision.encode(2), 2, 0).unwrap();
    assert_eq!(back, c.collision);
}

#[test]
fn the_import_covers_the_xmls_and_the_scans() {
    let Some(c) = ctx() else { return };
    let m = c.pack.manifest().unwrap();
    // Everything the XMLs name, by kind.
    let mut listed: BTreeMap<String, usize> = BTreeMap::new();
    for f in &c.p.symbols.files {
        for s in &f.symbols {
            *listed.entry(s.kind.clone()).or_default() += 1;
        }
    }
    for (kind, n) in &listed {
        let (l, imported) = m.counts.get(kind).copied().unwrap_or((0, 0));
        assert_eq!(l, *n, "{kind}: listed");
        let skipped: usize = m.skipped.get(kind).map(|s| s.values().sum()).unwrap_or(0);
        assert_eq!(imported + skipped, *n, "{kind}: imported {imported} + skipped {skipped}");
    }
    // Every texture, Player animation, scene and room is in; the skeletons and animations
    // left out are the ones `ootx scan-skeletons` can't read either (Skin/Curve limbs,
    // overlays) or can't size.
    for kind in ["Texture", "PlayerAnimation", "Scene", "Room"] {
        let (l, i) = m.counts[kind];
        assert_eq!(i, l, "{kind}");
    }
    assert_eq!(m.counts["Skeleton"], (194, 184));
    // `ootx scan-scenes --all-layers`: 141 distinct headers, 456 rooms, 2533 entries, 168,566
    // triangles in the main headers, 0 unknown opcodes, the 4 unresolved references.
    let s = &m.scenes;
    assert_eq!((s.scenes, s.headers, s.rooms, s.entries), (110, 141, 456, 2533));
    assert_eq!(s.main_triangles, 168_566);
    assert_eq!(s.unknown_opcodes, 0);
    assert_eq!(s.unresolved.len(), 4, "{:?}", s.unresolved);
    // Every record the manifest counts is in the pack.
    assert_eq!(c.pack.assets.names("tex/").len(), m.counts["Texture"].1);
    assert_eq!(c.pack.assets.names("scene/").len(), 110);
}

/// `gEntranceTable` and the actor table as the decomp defines them, spot-checked.
#[test]
fn entrance_and_actor_tables() {
    let Some(c) = ctx() else { return };
    let st = c.pack.scene_table().unwrap();
    let rows = std::fs::read_to_string(c.p.config.decomp.join("include/tables/entrance_table.h")).unwrap().matches("DEFINE_ENTRANCE(").count();
    assert_eq!(st.entrances.len(), rows);
    let e = &st.entrances[st.entrance_index("ENTR_LINK_HOME_0").unwrap() as usize];
    // DEFINE_ENTRANCE(ENTR_LINK_HOME_0, SCENE_LINK_HOME, 0, false, true, TRANS_TYPE_FADE_BLACK_FAST, TRANS_TYPE_FADE_BLACK_FAST)
    assert_eq!((e.scene, e.spawn, e.continue_bgm, e.title_card, e.end_trans_type, e.start_trans_type), (0x34, 0, false, true, 4, 4));
    assert_eq!(st.entrance_index("ENTR_LINK_HOME_0"), Some(0xBB));
    // TRANS_TYPE_CIRCLE(appearance, color, speed) = (1 << 5) | (color << 3) | (appearance << 1) | speed.
    let circles: Vec<u8> = st.entrances.iter().flat_map(|e| [e.end_trans_type, e.start_trans_type]).filter(|&t| t >= 32).collect();
    assert!(!circles.is_empty() && circles.iter().all(|&t| t < 56));
    let at = c.pack.actor_table().unwrap();
    let rows = std::fs::read_to_string(c.p.config.decomp.join("include/tables/actor_table.h")).unwrap();
    let defined = rows.lines().filter(|l| l.contains("DEFINE_ACTOR(") || l.contains("DEFINE_ACTOR_INTERNAL(")).count();
    let unset = rows.lines().filter(|l| l.contains("DEFINE_ACTOR_UNSET(")).count();
    assert_eq!(at.actors.len(), defined + unset);
    // Every actor has its ActorInit.
    let missing: Vec<&str> = at.actors.iter().filter(|a| !a.name.is_empty() && a.init.is_none()).map(|a| a.name.as_str()).collect();
    assert!(missing.is_empty(), "no ActorInit for {missing:?}");
    // En_Holl_InitVars: ACTORCAT_DOOR, ACTOR_FLAG_4, OBJECT_GAMEPLAY_KEEP.
    let holl = at.get(at.id("ACTOR_EN_HOLL").unwrap()).unwrap().init.clone().unwrap();
    assert_eq!((holl.category, holl.flags, holl.object_id), (10, 1 << 4, 1));
    assert_eq!(holl.update.as_deref(), Some("EnHoll_Update"));
    // Boss_Dodongo_InitVars says ACTOR_EN_DODONGO.
    let king = at.get(at.id("ACTOR_BOSS_DODONGO").unwrap()).unwrap().init.clone().unwrap();
    assert_eq!(king.id, at.id("ACTOR_EN_DODONGO").unwrap());
}

/// The lists with no stored length (the entrance and exit lists, `list_extent`) cover every
/// index gameplay reads them with: each exit index a floor carries, and for that exit the
/// spawn number the entrance table gives for the traveller's layer (`Play_Init` enters the
/// destination with the same age and time; Hyrule Field and Kokiri Forest pick their own
/// layers). Some entrance rows name spawns past their scene's list in layers only cutscenes and
/// actors use them from (the Gerudo guards' jail, the blue warps): those aren't reached here.
/// Transition actors' rooms and the room object lists are in range too.
#[test]
fn scene_lists_cover_what_the_game_indexes() {
    let Some(c) = ctx() else { return };
    let st = c.pack.scene_table().unwrap();
    let scenes: BTreeMap<u16, oot_game::scene::SceneData> = st.scenes.iter().filter_map(|d| Some((d.id, c.pack.scene(&d.file).ok()?))).collect();
    let mut problems = Vec::new();
    let mut checked = 0;
    for (&id, sd) in &scenes {
        for (layer, ld) in sd.layers.iter().enumerate() {
            let col = c.pack.collision(&ld.collision).unwrap();
            let used: BTreeSet<usize> = col.polys.iter().map(|p| (col.surface_types[p.ty as usize].data[0] >> 8 & 0x1F) as usize).filter(|&e| e != 0).collect();
            for &exit in &used {
                let Some(&next) = ld.exits.get(exit - 1) else {
                    problems.push(format!("{} layer {layer}: floors use exit {exit}, the list has {}", sd.name, ld.exits.len()));
                    continue;
                };
                if next >= 0x7FF9 {
                    continue; // ENTR_RETURN_*: the respawn point or a return group.
                }
                let Some(row) = st.entrances.get(next as usize + layer) else {
                    problems.push(format!("{} layer {layer}: exit {exit} to {next:#x} has no row for the layer", sd.name));
                    continue;
                };
                let child = layer == 0 || layer == 1;
                let dest_layer = match row.scene {
                    0x51 if child => 0,
                    0x55 if !child => 2,
                    _ => layer,
                };
                let Some(dest) = scenes.get(&row.scene) else { continue };
                checked += 1;
                let n = dest.layers[dest_layer].entrances.len();
                // @bug (game): Bongo Bongo's room's exit 1 is ENTR_HAKADAN_2, whose spawn 2 is
                // past the Shadow Temple's two-entry entrance list (the exit list follows it, and
                // the game would read its first entry, 0x0205, as {spawn 2, room 5}).
                if row.spawn as usize >= n && !(sd.name == "HAKAdan_bs_scene" && row.name.starts_with("ENTR_HAKADAN_2")) {
                    problems.push(format!("{} layer {layer} exit {exit} -> {}: spawn {} but {} layer {dest_layer} has {n}", sd.name, row.name, row.spawn, dest.name));
                }
            }
            let rooms = ld.rooms.len() as i8;
            for t in &ld.transition_actors {
                for (r, _) in t.sides {
                    if r >= rooms {
                        problems.push(format!("{} layer {layer}: transition actor to room {r} of {rooms}", sd.name));
                    }
                }
            }
            for key in &ld.rooms {
                let r = c.pack.room(key).unwrap();
                if r.objects.len() > 19 || r.objects.iter().any(|&o| o <= 0 || o as usize >= st.objects.len()) {
                    problems.push(format!("{key}: objects {:?}", r.objects));
                }
            }
        }
        let _ = id;
    }
    assert!(checked > 1000, "{checked} exits checked");
    assert!(problems.is_empty(), "{}", problems.join("
"));
}

#[test]
fn bg_camera_lists_cover_what_the_game_indexes() {
    // The bg camera list has no count (the importer reads at least what's named, then on while
    // the entries look like entries). Everything a scene names must be in it with a setting:
    // the floors' and water boxes' indices (SurfaceType_GetBgCamIndex, WATERBOX_BGCAM_INDEX),
    // the spawns' start cameras (Play_Init: params & 0xFF), the transition actors' sides, the
    // two viewpoints of a fixed-camera scene, and each multi-image room's backgrounds.
    let Some(c) = ctx() else { return };
    let st = c.pack.scene_table().unwrap();
    let mut problems = Vec::new();
    let (mut named, mut prerendered) = (0, 0);
    for d in &st.scenes {
        let Ok(sd) = c.pack.scene(&d.file) else { continue };
        for (layer, ld) in sd.layers.iter().enumerate() {
            let col = c.pack.collision(&ld.collision).unwrap();
            let mut want: BTreeSet<usize> = col.surface_types.iter().map(|s| (s.data[0] & 0xFF) as usize).collect();
            if col.bg_cams.is_empty() {
                // bgCamList NULL: the floors' indices then name nothing.
                continue;
            }
            want.extend(col.water_boxes.iter().map(|w| (w.properties & 0xFF) as usize).filter(|&i| i != 0xFF));
            want.extend(ld.spawns.iter().map(|s| (s.params as u16 & 0xFF) as usize).filter(|&i| i != 0xFF));
            want.extend(ld.transition_actors.iter().flat_map(|t| t.sides).filter(|s| s.1 >= 0).map(|s| s.1 as usize));
            if matches!(ld.scene_cam_type, 0x10 | 0x20) {
                want.extend([0, 1]);
            }
            for key in &ld.rooms {
                let r = c.pack.room(key).unwrap();
                want.extend(r.backgrounds.iter().filter_map(|b| b.bg_cam_index).map(usize::from));
                if !r.backgrounds.is_empty() {
                    prerendered += 1;
                }
            }
            for &i in &want {
                named += 1;
                if i >= col.bg_cams.len() {
                    problems.push(format!("{} layer {layer}: bg camera {i} of {}", sd.name, col.bg_cams.len()));
                } else if col.bg_cams[i].setting >= 0x42 {
                    problems.push(format!("{} layer {layer}: bg camera {i} setting {:#x}", sd.name, col.bg_cams[i].setting));
                }
            }
        }
    }
    assert!(named > 1000 && prerendered > 50, "{named} indices, {prerendered} prerendered rooms");
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
