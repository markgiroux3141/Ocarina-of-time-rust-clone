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
        assert_eq!((a.id as usize, &a.file, &a.enum_name, &a.draw_config, &a.title_file), (b.id, &b.file, &b.enum_name, &b.draw_config, &b.title_file));
        // Each place name's bake (oot_game::title_card).
        if !a.title_file.is_empty() {
            assert!(c.pack.assets.contains(&keys::bake(&oot_game::title_card::sprite_name(&a.title_file))), "{}", a.title_file);
        }
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
    // The scenes the golden renders use and the opening's, in every layer (the game layers,
    // then the cutscene layers their alternate header lists name: spot04 to 13, spot00 to 12,
    // link_home to 5, ydan none): the pack's rooms and collision equal what the spikes built from
    // the ROM for that layer at the bake time.
    for (file, layers) in [("spot04_scene", 14), ("spot00_scene", 13), ("ydan_scene", GAME_LAYERS), ("link_home_scene", 6)] {
        let scene = c.pack.scene(file).unwrap();
        assert_eq!(scene.layers.len(), layers, "{file}");
        assert_eq!(oot_import::scene::layer_count(&c.p.rom.file_by_name(file).unwrap()), layers, "{file}");
        for layer in 0..layers {
            let (ld, rooms, collision) = layer_records(&c.p, &c.tables, file, layer).unwrap();
            assert_eq!(scene.layers[layer], ld, "{file} layer {layer}");
            assert_eq!(c.pack.collision(&ld.collision).unwrap(), collision, "{file} layer {layer} collision");
            for (ri, r) in rooms.iter().enumerate() {
                assert_eq!(&c.pack.room(&keys::room(file, layer, ri)).unwrap(), r, "{file} layer {layer} room {ri}");
            }
            if layer >= GAME_LAYERS {
                continue;
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
    // The skin skeletons (the horses', GAME-04b milestone 6) are read too; the 3 left out are
    // Curve limbs and an overlay's.
    assert_eq!(m.counts["Skeleton"], (194, 191));
    // `ootx scan-scenes --all-layers`: 141 distinct headers, 456 rooms, 2533 entries in the game
    // layers; the 106 cutscene layers of 30 scenes add 106 headers, 199 rooms and 1489 entries
    // (docs/adr/0023); 168,566 triangles in the main headers, 0 unknown opcodes, the 4 unresolved
    // references.
    let s = &m.scenes;
    assert_eq!((s.scenes, s.headers, s.rooms, s.entries), (110, 141 + 106, 456 + 199, 2533 + 1489));
    assert_eq!(s.main_triangles, 168_566);
    assert_eq!(s.unknown_opcodes, 0);
    assert_eq!(s.unresolved.len(), 4, "{:?}", s.unresolved);
    // Every record the manifest counts is in the pack.
    assert_eq!(c.pack.assets.names("tex/").len(), m.counts["Texture"].1);
    assert_eq!(c.pack.assets.names("scene/").len(), 110);
}

/// The horses' skin skeletons (`z64skin.h`): the record is the ROM's `SkinLimb`s as the header
/// counts them, and `En_Viewer`'s bake gives every vertex a bone: a normal limb's, or its
/// animated limb's vertex group's (`oot_game::skin`).
#[test]
fn the_horses_skins_are_the_roms() {
    let Some(c) = ctx() else { return };
    for (file, skel, limbs) in [("object_horse_zelda", "gHorseZeldaSkel", 46usize), ("object_horse_ganon", "gHorseGanonSkel", 53)] {
        let f = c.p.symbols.file(file).unwrap();
        let sym = f.find(skel).unwrap();
        let data = c.p.rom.file_by_name(file).unwrap();
        // SkeletonHeader.limbCount, the byte after the limb table's pointer.
        assert_eq!(data[sym.offset as usize + 4] as usize, limbs, "{skel}");
        let raw = oot_import::skin::parse(&data, 6, sym.offset as usize).unwrap();
        let record = c.pack.skin_skeleton(file, skel).unwrap();
        assert_eq!(record, raw.skeleton);
        assert_eq!(record.limbs.len(), limbs);
        // One animated limb (the body), the rest normal limbs or joints without a list.
        let animated: Vec<usize> = (0..limbs).filter(|&i| record.limbs[i].segment_type == oot_game::skin::SKIN_LIMB_TYPE_ANIMATED).collect();
        assert_eq!(animated.len(), 1, "{skel}");
        let l = animated[0];
        // Every group's vertices together are the limb's totalVtxCount, each index once.
        let mut seen = vec![false; raw.total_vtx[l] as usize];
        for g in &raw.vertices[l] {
            for v in g {
                assert!(!std::mem::replace(&mut seen[v.index as usize], true), "{skel}: vertex {} twice", v.index);
            }
        }
        assert!(seen.iter().all(|&b| b), "{skel}: a vertex in no group");
        let d: eng_gfx::DrawList = c.pack.assets.get(&oot_game::pack::keys::bake(&format!("En_Viewer/{skel}"))).unwrap();
        let bones = record.bone_count() as u16;
        assert!(d.batches.iter().flat_map(|b| &b.vertices).all(|v| v.bone < bones), "{skel}: a vertex without a bone");
    }
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
        // The game layers: an exit from a cutscene layer leads to the layer the next
        // cutsceneIndex picks, not to the same one.
        for (layer, ld) in sd.layers.iter().enumerate().take(GAME_LAYERS) {
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

/// `SCENE_CMD_ID_PATH_LIST` has no count, like the exit list: the importer reads entries while
/// they point at points in the scene file (`oot_import::scene::path_list`). Each header's list
/// must have the decomp XML's `NumPaths` where the XML names it, and the pack must hold the
/// points the ROM has. Kokiri Forest's path 2 is `En_Goroiwa`'s (params 0x0C02).
#[test]
fn path_lists_match_the_xmls() {
    let Some(c) = ctx() else { return };
    // <Path Offset="0x.." NumPaths="n"/> under each <File Name="..._scene">.
    let mut xml: BTreeMap<String, BTreeMap<usize, usize>> = BTreeMap::new();
    let attr = |line: &str, name: &str| -> Option<String> {
        let at = line.find(&format!("{name}=\""))? + name.len() + 2;
        Some(line[at..].split('"').next()?.to_string())
    };
    let mut stack = vec![c.p.config.decomp.join("assets/xml/scenes")];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = e.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let mut file = String::new();
            for line in std::fs::read_to_string(&path).unwrap().lines() {
                if line.contains("<File ") {
                    file = attr(line, "Name").unwrap_or_default();
                } else if line.contains("<Path ") {
                    let off = usize::from_str_radix(attr(line, "Offset").unwrap().trim_start_matches("0x"), 16).unwrap();
                    let n: usize = attr(line, "NumPaths").unwrap().parse().unwrap();
                    xml.entry(file.clone()).or_default().insert(off, n);
                }
            }
        }
    }
    let st = c.pack.scene_table().unwrap();
    let (mut problems, mut matched, mut unnamed) = (Vec::new(), 0, 0);
    for d in &st.scenes {
        let Ok(sd) = c.pack.scene(&d.file) else { continue };
        for (layer, ld) in sd.layers.iter().enumerate() {
            let scene = oot_import::scene::Scene::load_layer(&c.p.rom, &d.file, layer).unwrap();
            assert_eq!(ld.paths, scene.paths, "{} layer {layer}", d.file);
            let Some(cmd) = scene.header.iter().find(|c| c.code == oot_import::scene::CMD_PATH_LIST) else {
                assert!(ld.paths.is_empty());
                continue;
            };
            let off = (cmd.data2 & 0xFF_FFFF) as usize;
            match xml.get(&d.file).and_then(|m| m.get(&off)) {
                Some(&n) if n == ld.paths.len() => matched += 1,
                Some(&n) => problems.push(format!("{} layer {layer}: {} paths at {off:#x}, the XML says {n}", d.file, ld.paths.len())),
                None => unnamed += 1,
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    assert!(matched >= 26, "{matched} path lists matched the XMLs ({unnamed} not named there)");
    let spot04 = c.pack.scene("spot04_scene").unwrap();
    let boulder = &spot04.layers[0].paths[2];
    assert_eq!(boulder.points, [[-247, 120, 1869], [-247, 120, 1538], [-575, 120, 1538], [-575, 120, 1869], [-247, 120, 1869]]);
}

/// Navi's C-Up texts (`table/elf_messages`): the ROM's `elf_message_field` and
/// `elf_message_ydan` (the import checks them against `ElfMessage` arrays built from the C's
/// macros), and `code`'s Saria tables; `SCENE_CMD_ID_SPECIAL_FILES`' `cUpElfMsgNum` per scene.
#[test]
fn the_c_up_texts_are_the_c_s() {
    let Some(c) = ctx() else { return };
    let t = c.pack.elf_messages().unwrap();
    assert_eq!(t.files.len(), 2);
    // gOverworldNaviMsgs: 28 commands; ELF_MSG_FLAG(CHECK, 0x40, false, EVENTCHKINF_05) first
    // (B0: CHECK 0 << 5 | FLAG 0 << 1 | false), ELF_MSG_END(0x5F) last (END 7 << 5).
    assert_eq!(t.files[0].len(), 28 * 4);
    assert_eq!(&t.files[0][..4], &[0x00, 0x05, 0x40, 0x00]);
    assert_eq!(&t.files[0][27 * 4..], &[0xE0, 0x00, 0x5F, 0x00]);
    // gDungeonNaviMsgs: ELF_MSG_END(0x5F) alone.
    assert_eq!(t.files[1], [0xE0, 0x00, 0x5F, 0x00]);
    // sChildSariaMsgs: ELF_MSG_STRENGTH_UPG(SKIP, 3, false, 0) (SKIP 3 << 5 | OTHER 3 << 1,
    // STRENGTH_UPG 0 << 4 | 0) first; 13 commands. sAdultSariaMsgs: 6.
    assert_eq!(&t.child_saria[..4], &[0x66, 0x00, 0x03, 0x00]);
    assert_eq!((t.child_saria.len(), t.adult_saria.len()), (13 * 4, 6 * 4));
    for (file, num) in [("spot04_scene", 1), ("spot00_scene", 1), ("link_home_scene", 0), ("ydan_scene", 2)] {
        assert_eq!(c.pack.scene(file).unwrap().layers[0].c_up_elf_msg_num, num, "{file}");
    }
}

/// The cutscene layers' scripts (docs/adr/0023): no XML names them, so each is keyed by its
/// scene file and offset, and walked to its `CS_END` like the named ones.
#[test]
fn the_cutscene_layers_scripts_are_keyed_by_offset() {
    let Some(c) = ctx() else { return };
    for (file, layer, key) in [
        ("link_home_scene", 4, "cutscene/link_home_scene/0x1040"),
        ("link_home_scene", 5, "cutscene/link_home_scene/0x15D0"),
        ("spot00_scene", 4, "cutscene/spot00_scene/0x12400"),
        ("spot04_scene", 7, "cutscene/spot04_scene/0xA6D0"),
    ] {
        let ld = &c.pack.scene(file).unwrap().layers[layer];
        assert_eq!(ld.cutscene.as_deref(), Some(key), "{file} layer {layer}");
        let s = c.pack.cutscene(key).unwrap();
        let data = c.p.rom.file_by_name(file).unwrap();
        let off = usize::from_str_radix(key.rsplit("0x").next().unwrap(), 16).unwrap();
        assert_eq!(&data[off..off + s.data.len()], &s.data[..], "{key}: the ROM's bytes");
        assert_eq!(oot_game::cutscene::walk(&s.data).unwrap().1, Some(s.data.len()), "{key}: through CS_END");
    }
    // A script the XML names keeps its symbol's key: Kokiri Forest's Deku Sprout, the last of
    // its cutscene layers' ten (the other nine have none).
    let keys: Vec<String> = c.pack.scene("spot04_scene").unwrap().layers.iter().filter_map(|l| l.cutscene.clone()).collect();
    assert_eq!(keys.len(), 10);
    assert_eq!(keys.iter().filter(|k| !k.contains("/0x")).collect::<Vec<_>>(), ["cutscene/spot04_scene/gKokiriForestDekuSproutCs"]);
}

#[test]
fn the_audio_data_is_the_roms_and_the_cs() {
    use oot_import::audio::LoadAudioData;
    let Some(c) = ctx() else { return };
    let pack = c.pack.audio_data().expect("the pack's audio data");
    let fresh = eng_audio::AudioData::load(&c.p).expect("loading the audio data");
    assert!(pack == fresh, "the pack's audio data is what the importer reads");
    // The ROM's files, as they are.
    for f in [&pack.audiobank, &pack.audioseq, &pack.audiotable] {
        assert_eq!(&f.bytes[..], &c.p.rom.file_by_name(&f.name).unwrap()[..], "{}", f.name);
    }
    let t = &pack.tables;
    let count = |b: &[u8]| eng_audio::context::AudioTable::parse(b).entries.len();
    assert_eq!((count(&t.sound_font_table), count(&t.sequence_table), count(&t.sample_bank_table)), (38, 110, 7), "fonts, sequences, sample banks");
    assert_eq!(t.heap_sizes.num_soundfonts as usize, count(&t.sound_font_table), "NUM_SOUNDFONTS");
    assert_eq!(t.heap_sizes.audio_heap, 0x38000, "sizeof(gAudioHeap)");
    assert_eq!(t.tatums_per_beat, 48, "gTatumsPerBeat");
    // audio_data.c: gPitchFrequencies' C4 (0x27) and its wrap at 0x75 (PITCH_BFLATNEG1).
    assert_eq!((t.pitch_frequencies[0x27], t.pitch_frequencies[0x75]), (1.0, 0.055681));
    assert_eq!(t.default_envelope, [(1, 32000), (1000, 32000), (-1, 0), (0, 0)], "gDefaultEnvelope with ADSR_HANG, ADSR_DISABLE");
    assert_eq!((t.haas_effect_delay_sizes[0], t.haas_effect_delay_sizes[29], t.haas_effect_delay_sizes[30]), (60, 2, 0), "30 * SAMPLE_SIZE down to 0");
    assert_eq!(t.wave_sample_index, [0, 1, 2, 3, 4, 5, 6, 7, 7], "gWaveSamples: the quarter pulse twice");
    // audio_init_params.c: 18 specs; the first plays 24 notes on 4 players with 2 reverbs,
    // DEFAULT_REVERB_SETTINGS first.
    assert_eq!(t.specs.len(), 18);
    let s0 = &t.specs[0];
    assert_eq!((s0.sampling_frequency, s0.num_notes, s0.num_sequence_players, s0.num_reverbs), (32000, 24, 4, 2));
    assert_eq!((s0.reverb_settings[0].window_size, s0.reverb_settings[0].decay_ratio, s0.reverb_settings[0].unk_10), (0x30, 0x3000, -1));
    assert_eq!(s0.reverb_settings[1].window_size, 0x20);
    assert_eq!((s0.temporary_seq_cache_size, s0.temporary_font_cache_size), (0x4000, 0x2880));
    // The microcode's resampler filters: 64 phases of 4 taps, each summing to about 1.0, the
    // phases mirroring each other (phase i is phase 63 - i reversed).
    assert_eq!(t.resample_lut.len(), 256);
    for i in 0..64 {
        let row = &t.resample_lut[i * 4..i * 4 + 4];
        let mirror = &t.resample_lut[(63 - i) * 4..(63 - i) * 4 + 4];
        assert_eq!([row[0], row[1], row[2], row[3]], [mirror[3], mirror[2], mirror[1], mirror[0]], "phase {i}");
    }
    // The noise starts at func_800E4FE0's first instruction (`addiu sp, sp, -n`).
    assert_eq!(t.noise_code_vram, 0x800E_4FE0);
    assert_eq!(&t.noise_code[..2], &[0x27, 0xBD], "func_800E4FE0 starts its stack frame");
}

#[test]
fn the_games_audio_tables_and_the_scenes_sound_settings_are_the_cs() {
    let Some(c) = ctx() else { return };
    let t = c.pack.audio_game_tables().expect("the pack's game audio tables");
    let fresh = oot_import::audio::audio_game_tables(&c.p.config.decomp).expect("reading them");
    assert!(t == fresh, "the pack's game audio tables are what the importer reads");
    use oot_game::audio::*;
    // code_800EC960.c's sSeqFlags, by the NA_BGM_* each row's comment names.
    assert_eq!(t.seq_flags.len(), 0x6E);
    assert_eq!(t.seq_flags[0x00], SEQ_FLAG_FANFARE, "NA_BGM_GENERAL_SFX");
    assert_eq!(t.seq_flags[0x01], SEQ_FLAG_ENEMY, "NA_BGM_NATURE_AMBIENCE");
    assert_eq!(t.seq_flags[0x18], SEQ_FLAG_5 | SEQ_FLAG_ENEMY, "NA_BGM_DUNGEON");
    assert_eq!(t.seq_flags[0x1B], SEQ_FLAG_NO_AMBIENCE | SEQ_FLAG_RESTORE, "NA_BGM_BOSS");
    assert_eq!(t.seq_flags[0x1F], SEQ_FLAG_5, "NA_BGM_LINK_HOUSE");
    assert_eq!(t.seq_flags[NA_BGM_KOKIRI as usize], SEQ_FLAG_4 | SEQ_FLAG_ENEMY, "NA_BGM_KOKIRI");
    assert_eq!(t.seq_flags[0x6D], 0, "NA_BGM_CUTSCENE_EFFECTS");
    // sSpecReverbs: 40 for spec 7, 15 for spec 9.
    assert_eq!(t.spec_reverbs, [0, 0, 0, 0, 0, 0, 0, 40, 0, 15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    // sNatureAmbienceDataIO[NATURE_ID_GENERAL_NIGHT]: 0xC0FF, 0xC0FE; the stream's two triples
    // (NATURE_CHANNEL_STREAM_0, CHANNEL_IO_PORT_2, NATURE_STREAM_RUSHING_WATER; ..._PORT3(0)),
    // then the crows (NATURE_CHANNEL_CRITTER_0: type NATURE_CRITTER_CROWS_CAWS 9, bend 64,
    // 0 layers, port 5 32), ... 30 triples and NATURE_IO_ENTRIES_END, zero to 100.
    assert_eq!(t.nature_ambience.len(), 20);
    let n0 = &t.nature_ambience[0];
    assert_eq!((n0.player_io, n0.channel_mask), (0xC0FF, 0xC0FE));
    assert_eq!(&n0.channel_io[..18], &[0, 2, 0, 0, 3, 0, 1, 2, 9, 1, 3, 64, 1, 4, 0, 1, 5, 32]);
    assert_eq!(n0.channel_io[90], 0xFF);
    assert_eq!(n0.channel_io.len(), 100);
    assert!(n0.channel_io[91..].iter().all(|&b| b == 0));
    for (i, n) in t.nature_ambience.iter().enumerate() {
        let end = n.channel_io.iter().position(|&b| b == 0xFF).unwrap_or_else(|| panic!("ambience {i}: no NATURE_IO_ENTRIES_END"));
        assert_eq!(end % 3, 0, "ambience {i}: whole triples");
    }
    // audio_external_data.c's gSoundModeList.
    assert_eq!(t.sound_mode_list, [SOUNDMODE_STEREO as u8, SOUNDMODE_HEADSET as u8, SOUNDMODE_SURROUND as u8, SOUNDMODE_MONO as u8]);

    // SCENE_CMD_SOUND_SETTINGS: what the pack holds for each layer is the ROM's header command.
    let mut with = 0;
    for name in ["spot04_scene", "link_home_scene", "kokiri_shop_scene", "ydan_scene", "spot00_scene"] {
        let sd = c.pack.scene(name).expect("scene");
        let file: std::sync::Arc<[u8]> = c.p.rom.file_by_name(name).expect("scene file").into();
        for (layer, ld) in sd.layers.iter().enumerate() {
            let rom = oot_import::scene::Scene::parse_layer(name, file.clone(), layer).expect("the header");
            assert_eq!(ld.sound, rom.sound, "{name} layer {layer}");
            with += ld.sound.is_some() as usize;
        }
    }
    assert!(with > 0);
    // Kokiri Forest by day: spec 1, its region's ambience, its music ("Kokiri Forest",
    // sequence.h).
    let k = c.pack.scene("spot04").unwrap().layers[0].sound.unwrap();
    assert_eq!((k.spec_id, k.nature_ambience_id, k.seq_id as u16), (1, NATURE_ID_KOKIRI_REGION, NA_BGM_KOKIRI));
}

#[test]
fn the_sound_effects_tables_are_the_cs() {
    let Some(c) = ctx() else { return };
    let t = c.pack.audio_game_tables().expect("the pack's game audio tables");
    use oot_game::audio::sfx::*;
    // gSfxParams' seven banks, as long as their tables (include/tables/sfx/*.h).
    let lens: Vec<usize> = t.sfx_params.iter().map(|b| b.len()).collect();
    assert_eq!(lens, [224, 80, 248, 499, 72, 8, 128]);
    // DEFINE_SFX(NA_SE_PL_WALK_GROUND, 0x20, 0, 2, SFX_FLAG_10): randParam 2 << 6 | SFX_FLAG_10.
    assert_eq!(t.sfx_params(0x800), (0x20, (2 << SFX_PARAM_67_SHIFT) | SFX_FLAG_10));
    assert_eq!(t.sfx_params[0][0].name, "NA_SE_PL_WALK_GROUND");
    // DEFINE_SFX(NA_SE_SY_WIN_OPEN, 0xC0, 0, 0, 0), (NA_SE_SY_CORRECT_CHIME, 0xB0, 0, 0, SFX_FLAG_5).
    assert_eq!(t.sfx_params(NA_SE_SY_WIN_OPEN), (0xC0, 0));
    assert_eq!(t.sfx_params(0x4802), (0xB0, SFX_FLAG_5));
    assert_eq!(t.sfx_params(NA_SE_SY_WIN_OPEN - SFX_FLAG), t.sfx_params(NA_SE_SY_WIN_OPEN), "the id's 0x800 bit isn't its index");
    // The ids the code names are the tables' rows.
    for &(name, id) in NAMED_SFX {
        assert_eq!(t.sfx_id(name), Some(id), "{name}");
    }
    // gSfxBanks' arrays (D_8016BAD0[9] ...), gIsLargeSfxBank, the channel layouts' first row.
    assert_eq!(t.sfx_bank_sizes, [9, 12, 22, 20, 8, 3, 5]);
    assert_eq!(t.is_large_sfx_bank, [0, 0, 0, 1, 0, 0, 0]);
    assert_eq!(t.channels_per_bank[0], [3, 2, 3, 3, 2, 1, 2]);
    assert_eq!(t.used_channels_per_bank[0], [3, 2, 3, 2, 2, 1, 1]);
    assert_eq!(t.channels_per_bank.len(), 4);
    assert_eq!(t.behind_screen_z, [-15.0, -65.0]);
    assert_eq!(t.charge_freq_scales, [1.0, 1.12246, 1.33484, 1.33484]);
    assert_eq!(t.ganons_tower_levels_vol, [127, 80, 75, 73, 70, 68, 65, 60]);
}

#[test]
fn the_one_point_tables_are_the_roms() {
    // z_onepointdemo_data.c's tables and Camera_Demo5's (z_camera_data.c), as the importer read
    // them from the C, against the bytes of the ROM's code file: each D_ symbol is named after
    // its address, so with one table found by its bytes, every other is at its address's
    // offset from that one.
    let Some(c) = ctx() else { return };
    let d = c.pack.game_data().unwrap().camera.onepoint;
    let code = c.p.rom.file_by_name("code").unwrap();
    let point_bytes = |p: &oot_game::cutscene::CutsceneCameraPoint| {
        let mut b = vec![p.continue_flag as u8, p.camera_roll as u8];
        b.extend(p.next_point_frame.to_be_bytes());
        b.extend(p.view_angle.to_bits().to_be_bytes());
        for v in p.pos {
            b.extend(v.to_be_bytes());
        }
        b.extend([0, 0]);
        b
    };
    let kf_bytes = |k: &oot_game::camera::OnePointCsFull| {
        let mut b = vec![k.action_flags, k.unk_01];
        for v in [k.init_flags, k.timer_init, k.roll_target_init] {
            b.extend(v.to_be_bytes());
        }
        for v in [k.fov_target_init, k.lerp_step_scale, k.at_target_init.x, k.at_target_init.y, k.at_target_init.z, k.eye_target_init.x, k.eye_target_init.y, k.eye_target_init.z] {
            b.extend(v.to_bits().to_be_bytes());
        }
        b
    };
    let addr = |name: &str| u32::from_str_radix(name.strip_prefix("D_").unwrap(), 16).unwrap();
    // The anchor: D_8012013C (3050's at points), found by its bytes.
    let (anchor_name, anchor) = &d.points[0];
    let want: Vec<u8> = anchor.iter().flat_map(point_bytes).collect();
    let at = code.windows(want.len()).position(|w| w == want.as_slice()).expect("the first point list in code");
    let base = addr(anchor_name) as usize - at;
    let mut checked = 0;
    for (name, pts) in &d.points {
        let o = addr(name) as usize - base;
        let want: Vec<u8> = pts.iter().flat_map(point_bytes).collect();
        assert_eq!(&code[o..o + want.len()], want.as_slice(), "{name}");
        checked += 1;
    }
    for (name, kfs) in &d.keyframes {
        let o = addr(name) as usize - base;
        let want: Vec<u8> = kfs.iter().flat_map(kf_bytes).collect();
        assert_eq!(&code[o..o + want.len()], want.as_slice(), "{name}");
        checked += 1;
    }
    for (name, v) in &d.shorts {
        let o = addr(name) as usize - base;
        assert_eq!(&code[o..o + 2], v.to_be_bytes().as_slice(), "{name}");
        checked += 1;
    }
    // 10 point lists, 83 keyframe tables (75 and Camera_Demo5's 8), 12 shorts.
    assert_eq!((d.points.len(), d.keyframes.len(), d.shorts.len(), checked), (10, 83, 12, 105));
    // The crawlspace's: D_80120308 (at) with D_80120398 (9601's eye) and D_80120434 (9602's),
    // D_8012042C frames, action D_80120430 1 (around the main camera's Player).
    assert_eq!(d.short("D_8012042C"), Some(90));
    assert_eq!(d.short("D_80120430"), Some(1));
    assert_eq!(d.points("D_80120398").unwrap()[0].pos, [0, 9, 45]);
    assert_eq!(d.points("D_80120308").unwrap()[1].view_angle, 40.000004);
    // The settings the one-point cutscenes name, against z64camera.h's enum.
    let cam = c.pack.game_data().unwrap().camera;
    for (id, name) in [
        (oot_game::onepoint::CAM_SET_FREE2, "CAM_SET_FREE2"),
        (oot_game::onepoint::CAM_SET_CS_3, "CAM_SET_CS_3"),
        (oot_game::onepoint::CAM_SET_CS_C, "CAM_SET_CS_C"),
        (oot_game::camera::CAM_SET_CS_ATTENTION, "CAM_SET_CS_ATTENTION"),
        (oot_game::camera::CAM_SET_TURN_AROUND, "CAM_SET_TURN_AROUND"),
    ] {
        assert_eq!(cam.setting_id(name), Some(id), "{name}");
    }
    let k = &d.keyframes[d.keyframe_table("D_8011D9F4").unwrap()].1;
    assert_eq!((k[0].action_flags, k[0].init_flags as u16, k[0].timer_init, k[0].eye_target_init.z), (0x8F, 0x0504, 0x14, 300.0));
}
