//! Kokiri Forest from the asset pack: its rooms, its ported draw config and its environment.
//! Expected values are derived from the decomp's C (formulas and constants quoted in each
//! test), not from the port. Skips without a pack (`oot import` builds it).
//!
//! The same scene through the ROM and the decomp (and the C interpreter for the draw config)
//! is compared with these records in `oot_import`'s tests.

use std::sync::OnceLock;

use glam::Vec3;
use oot_game::env::{self, EnvState, EnvTables};
use oot_game::pack::GamePack;
use oot_game::room::cullable_order;
use oot_game::scene::{RoomData, SceneData, ShapeKind};
use oot_game::scene_table::{self, DrawConfigState};

struct Ctx {
    scene: SceneData,
    rooms: Vec<RoomData>,
    env: EnvTables,
}

fn ctx() -> Option<&'static Ctx> {
    static C: OnceLock<Option<Ctx>> = OnceLock::new();
    C.get_or_init(|| {
        let pack = match GamePack::open_default() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("skipping: {e:#}");
                return None;
            }
        };
        let scene = pack.scene("spot04").expect("spot04");
        let rooms = scene.layers[0].rooms.iter().map(|k| pack.room(k).expect("room")).collect();
        Some(Ctx { env: pack.env_tables().expect("env tables"), scene, rooms })
    })
    .as_ref()
}

fn state(f: u32) -> DrawConfigState {
    // Child, day: the main header (gSaveContext.sceneLayer 0).
    DrawConfigState { gameplay_frames: f, child: true, night: false, scene_layer: 0, day_time: env::clock_time(10, 0) as u16, ..Default::default() }
}

#[test]
fn kokiri_rooms_interpret_cleanly() {
    let Some(c) = ctx() else { return };
    let l = &c.scene.layers[0];
    // spot04_scene: SCENE_CMD_ROOM_LIST(3, ...), SCENE_CMD_SPECIAL_FILES(.., OBJECT_GAMEPLAY_FIELD_KEEP),
    // DEFINE_SCENE(spot04_scene, .., SDC_KOKIRI_FOREST, ..).
    assert_eq!(c.rooms.len(), 3);
    assert_eq!(l.keep_object.as_deref(), Some("gameplay_field_keep"));
    assert_eq!(c.scene.draw_config, "SDC_KOKIRI_FOREST");
    assert!(scene_table::is_ported("SDC_KOKIRI_FOREST"));
    assert!(l.notes.is_empty(), "{:?}", l.notes);
    let mut tris = 0;
    for r in &c.rooms {
        assert_eq!(r.shape, Some(ShapeKind::Cullable), "Kokiri rooms are ROOM_SHAPE_TYPE_CULLABLE");
        for e in &r.entries {
            for d in [&e.opa, &e.xlu].into_iter().flatten() {
                assert!(d.stats.unknown_opcodes.is_empty(), "{:?}", d.stats.unknown_opcodes);
                assert!(d.stats.unresolved_addresses.is_empty(), "{:?}", d.stats.unresolved_addresses);
                tris += d.triangle_count();
            }
        }
    }
    // Same total as the extractor, which runs each room's DLs in one interpreter pass.
    assert_eq!(tris, 2060);
}

#[test]
fn kokiri_water_scrolls_like_gfx_two_tex_scroll() {
    for f in [0u32, 1, 5, 127, 128, 200, 1000] {
        let [opa, xlu] = scene_table::segment_values("SDC_KOKIRI_FOREST", &mut state(f)).expect("SDC_KOKIRI_FOREST is ported");
        // Scene_DrawConfigKokiriForest: segment 9 = Gfx_TwoTexScroll(tile 0: 127 - f % 128, (f * 1) % 128;
        // tile 1: f % 128, (f * 1) % 128), segment 8 the same with f * 10 on t. Gfx_TwoTexScroll
        // reduces each coordinate mod 512 << 2.
        let m = |v: u32| (v % 2048) as u16;
        assert_eq!(xlu.tiles[9][0], Some((m(127 - f % 128), m(f % 128))), "seg 9 tile 0, frame {f}");
        assert_eq!(xlu.tiles[9][1], Some((m(f % 128), m(f % 128))), "seg 9 tile 1, frame {f}");
        assert_eq!(xlu.tiles[8][0], Some((m(127 - f % 128), m((f * 10) % 128))), "seg 8 tile 0, frame {f}");
        assert_eq!(xlu.tiles[8][1], Some((m(f % 128), m((f * 10) % 128))), "seg 8 tile 1, frame {f}");
        // Segments 0xA/0xB: gDPSetEnvColor(128, 128, 128, spA3 = 128) and (.., spA0 * 0.1f) with
        // spA0 = 500 (no event flag set).
        assert_eq!(opa.env[0xA], Some([128, 128, 128, 128]));
        assert_eq!(opa.env[0xB], Some([128, 128, 128, 50]));
        assert_eq!(xlu.env[0xB], Some([128, 128, 128, 50]));
    }
    // With EVENTCHKINF_07 set outside the cutscene layers, spA0 = 2150: alpha 215.
    let [opa, _] = scene_table::segment_values("SDC_KOKIRI_FOREST", &mut DrawConfigState { event_chk_inf_07: true, ..state(0) }).unwrap();
    assert_eq!(opa.env[0xB], Some([128, 128, 128, 215]));
}

#[test]
fn kokiri_scrolling_materials_shift_uvs() {
    let Some(c) = ctx() else { return };
    let stream = c.rooms[0]
        .entries
        .iter()
        .filter_map(|e| e.xlu.as_ref())
        .flat_map(|d| d.materials.iter())
        .find(|m| m.uv_dyn[0].is_some_and(|t| t.segment == 9))
        .expect("a room 0 XLU material scrolled by segment 9");
    let f = 40u32;
    let [_, xlu] = scene_table::segment_values("SDC_KOKIRI_FOREST", &mut state(f)).unwrap();
    let off = stream.uv_offsets(&xlu);
    // Tile 0 origin moves from (127, 0) to (127 - 40, 40) in 10.2 fixed point over a 32x32
    // texture: the UVs shift by -(delta / 4) / 32.
    let t = stream.uv_dyn[0].unwrap();
    assert_eq!((t.width, t.height), (32, 32));
    assert!((off[0].x - (40.0 / 4.0 / 32.0)).abs() < 1e-6, "{off:?}");
    assert!((off[0].y - (-40.0 / 4.0 / 32.0)).abs() < 1e-6, "{off:?}");
}

#[test]
fn the_castles_courtyards_and_water_scroll() {
    let m = |v: u32| (v % 2048) as u16;
    for f in [0u32, 1, 5, 127, 128, 200, 1000] {
        // Scene_DrawConfigHyruleCastle: segment 8 at f * 10 on t, segment 9 at f * 3.
        let [_, xlu] = scene_table::segment_values("SDC_HYRULE_CASTLE", &mut state(f)).unwrap();
        assert_eq!(xlu.tiles[8][0], Some((m(127 - f % 128), m((f * 10) % 128))), "castle seg 8, frame {f}");
        assert_eq!(xlu.tiles[9][1], Some((m(f % 128), m((f * 3) % 128))), "castle seg 9, frame {f}");
        // Scene_DrawConfigCastleCourtyardGuards: segment 8 at f * 3; by day (SCENE_CASTLE_COURTYARD_GUARDS_DAY,
        // 0x45) the fountains on segment 9, Gfx_TexScroll(0, (f * 10) % 256, 32, 64).
        let mut day = DrawConfigState { scene_id: 0x45, ..state(f) };
        let [_, xlu] = scene_table::segment_values("SDC_CASTLE_COURTYARD_GUARDS", &mut day).unwrap();
        assert_eq!(xlu.tiles[8][0], Some((m(127 - f % 128), m((f * 3) % 128))), "courtyard seg 8, frame {f}");
        assert_eq!(xlu.tiles[9][0], Some((0, m((f * 10) % 256))), "courtyard seg 9, frame {f}");
        // By night (SCENE_CASTLE_COURTYARD_GUARDS_NIGHT, 0x46): no segment 9.
        let mut night = DrawConfigState { scene_id: 0x46, ..state(f) };
        let [_, xlu] = scene_table::segment_values("SDC_CASTLE_COURTYARD_GUARDS", &mut night).unwrap();
        assert_eq!(xlu.tiles[9][0], None);
        // Scene_DrawConfigCalmWater (Zelda's courtyard): segment 8 at f * 1.
        let [_, xlu] = scene_table::segment_values("SDC_CALM_WATER", &mut state(f)).unwrap();
        assert_eq!(xlu.tiles[8][1], Some((m(f % 128), m(f % 128))), "calm water, frame {f}");
    }
}

#[test]
fn the_temple_of_time_fades_its_colours_by_its_draw_params() {
    // Scene_DrawConfigTempleOfTime with drawParams[0] 255 (temp 1): segment 8's prim
    // 255 - 185, 255 - 145, 255 - 105; 9's 76 + 6, 76 + 34, 76 + 74; 0xB's grey 89 + 166 = 255; 0xC's
    // 255 + 179 = 434, its low byte 178; the env alphas drawParams[0] (0xA to 0xC) and [1] (0xD).
    let mut st = DrawConfigState { room_draw_params: [255, 40], ..state(0) };
    let [opa, xlu] = scene_table::segment_values("SDC_TEMPLE_OF_TIME", &mut st).unwrap();
    for v in [&opa, &xlu] {
        assert_eq!(v.prim[8], Some([70, 110, 150, 255]));
        assert_eq!(v.prim[9], Some([82, 110, 150, 255]));
        assert_eq!(v.env[0xA], Some([0, 0, 0, 255]));
        assert_eq!(v.prim[0xB], Some([255, 255, 255, 255]));
        assert_eq!(v.prim[0xC], Some([178, 178, 178, 255]));
        assert_eq!(v.env[0xD], Some([0, 0, 0, 40]));
    }
    assert_eq!(st.custom_lens_flare, None);
    // On layer 5 the custom lens flare at the window: gCustomLensFlarePos (-20, 1220, -684), scale 10,
    // colour intensity 8, glare 200.
    let mut st = DrawConfigState { scene_layer: 5, ..state(0) };
    scene_table::segment_values("SDC_TEMPLE_OF_TIME", &mut st).unwrap();
    let flare = st.custom_lens_flare.expect("layer 5's lens flare");
    assert_eq!((flare.pos, flare.scale, flare.color_intensity, flare.glare_strength), ([-20.0, 1220.0, -684.0], 10, 8.0, 200));
}

#[test]
fn hyrule_field_fades_the_night_overlay_in() {
    // Scene_DrawConfigHyruleField after 18:30: Math_StepToS(&roomCtx.drawParams[0], 255, 5) every
    // frame, and the prim alpha of segment 0xA's display list is the result.
    let mut st = DrawConfigState { child: false, night: true, scene_layer: 3, day_time: env::clock_time(19, 0) as u16, ..Default::default() };
    for f in 1..=60u32 {
        st.gameplay_frames = f;
        let [_, xlu] = scene_table::segment_values("SDC_HYRULE_FIELD", &mut st).unwrap();
        let alpha = (5 * f).min(255) as u8;
        assert_eq!(xlu.prim[0xA], Some([255, 255, 255, alpha]), "frame {f}");
    }
    // By day (7:00 < t <= 18:30) segment 0xA is an empty list.
    let mut day = DrawConfigState { day_time: env::clock_time(12, 0) as u16, ..Default::default() };
    let [_, xlu] = scene_table::segment_values("SDC_HYRULE_FIELD", &mut day).unwrap();
    assert_eq!(xlu.prim[0xA], None);
}

#[test]
fn kokiri_lights_at_ten() {
    let Some(c) = ctx() else { return };
    let l0 = &c.scene.layers[0];
    let list = &l0.light_settings;
    assert_eq!(list.len(), 12);
    // SCENE_CMD_SKYBOX_SETTINGS(SKYBOX_UNSET_1D, 0, LIGHT_MODE_TIME).
    assert_eq!((l0.skybox.skybox_id, l0.skybox.light_mode), (29, env::LIGHT_MODE_TIME));
    // A new save starts at CLOCK_TIME(10, 0) (Sram_InitNewSave); the rooms keep the time
    // (SCENE_CMD_TIME_SETTINGS(0xFF, 0xFF, 0)), and 10:00 is outside the snapping ranges.
    let t = env::clock_time(10, 0) as u16;
    assert_eq!(t, 27307);
    let (day, sky, speed) = env::scene_times(t, c.rooms[0].time);
    assert_eq!((day, sky, speed), (t, t, 0));
    let l = env::update(&c.env, list, &EnvState { light_mode: 0, light_config: 0, light_setting: 0, day_time: day, skybox_time: sky });
    // sTimeBasedLightConfigs[0][3] = { CLOCK_TIME(8, 0) + 1, CLOCK_TIME(16, 0), 1, 1 }: light setting 1 unblended.
    assert_eq!(c.env.time_based[0][3].light_setting, 1);
    assert_eq!(l.ambient, list[1].ambient);
    assert_eq!(l.light1_color, list[1].light1_color);
    assert_eq!(l.fog_color, list[1].fog_color);
    assert_eq!(l.fog_near, (list[1].fog_near_raw & 0x3FF) as i16);
    assert_eq!(l.fog_far, list[1].fog_far);
    // Sun: -(sin(dayTime - 12:00) * 120), cos * 120, cos * 20 with dayTime - 12:00 = -5461.
    let a = -5461.0f64 * std::f64::consts::TAU / 65536.0;
    let expect = [(-(a.sin() * 120.0)) as i8, (a.cos() * 120.0) as i8, (a.cos() * 20.0) as i8];
    assert_eq!(l.light1_dir, expect);
    assert_eq!(l.light2_dir, expect.map(|v| -v));
}

#[test]
fn kokiri_lights_blend_at_dawn() {
    let Some(c) = ctx() else { return };
    let list = &c.scene.layers[0].light_settings;
    // 7:00 lies in { CLOCK_TIME(6, 0), CLOCK_TIME(8, 0) + 1, 0, 1 }; weight =
    // 1 - (end - t) / (end - start) (Environment_LerpWeight).
    let t = env::clock_time(7, 0) as u16;
    let (start, end) = (env::clock_time(6, 0) as f32, env::clock_time(8, 0) as f32 + 1.0);
    let w = 1.0 - (end - t as f32) / (end - start);
    let l = env::update(&c.env, list, &EnvState { light_mode: 0, light_config: 0, light_setting: 0, day_time: t, skybox_time: t });
    for j in 0..3 {
        let (a, b) = (list[0].ambient[j] as f32, list[1].ambient[j] as f32);
        assert_eq!(l.ambient[j], ((b - a) * w + a) as u8);
        let (a, b) = (list[0].fog_color[j] as f32, list[1].fog_color[j] as f32);
        assert_eq!(l.fog_color[j], ((b - a) * w + a) as u8);
    }
}

#[test]
fn fog_position_matches_gbi() {
    // gSPFogPosition(994, 1000): fm = 128000 / 6, fo = (500 - 994) * 256 / 6 (C integer division).
    assert_eq!(env::fog_factor(994, 1000), (21333, -21077));
    // Gfx_SetFog: near >= 1000 gives no fog; 997..999 the fixed (0x7FFF, 0x8100).
    assert_eq!(env::fog_factor(1000, 1000), (0, 0));
    assert_eq!(env::fog_factor(998, 1000), (0x7FFF, 0x8100u16 as i16));
}

#[test]
fn cullable_order_follows_room_draw_cullable() {
    // Depth = z of the centre; keep -r < z and z - r < fogFar; ascending z - r, ties in order.
    let b = [Some((Vec3::new(0.0, 0.0, 500.0), 100.0)), Some((Vec3::new(0.0, 0.0, -150.0), 100.0)), Some((Vec3::new(0.0, 0.0, 200.0), 300.0)), Some((Vec3::new(0.0, 0.0, 6000.0), 100.0))];
    let order = cullable_order(&b, |p| p.z, 5800.0);
    assert_eq!(order, vec![2, 0]);
}

/// The textures a layer's rooms loaded from `source` (`G_SETTIMG`'s segmented address).
fn room_textures_from(pack: &GamePack, scene: &str, layer: usize, source: u32) -> Vec<Vec<u8>> {
    let s = pack.scene(scene).expect("scene");
    let mut out = Vec::new();
    for k in &s.layers[layer].rooms {
        let r = pack.room(k).expect("room");
        for e in &r.entries {
            for d in [&e.opa, &e.xlu].into_iter().flatten() {
                out.extend(d.textures.iter().filter(|t| t.source_addr == Some(source)).map(|t| t.image.rgba.clone()));
            }
        }
    }
    out
}

#[test]
fn the_window_textures_are_loaded_through_the_segments_the_draw_configs_swap() {
    let Ok(pack) = GamePack::open_default() else { return };
    // Scene_DrawConfigKakarikoVillage: gSPSegment(POLY_OPA_DISP, 0x08,
    // sKakarikoWindowTextures[nightFlag]). The rooms load the windows from 0x08000000, baked with
    // the day's on layer 0 and the night's on layer 1, so a draw can put the other in place.
    let tex = |file: &str, sym: &str| pack.texture(file, sym).expect("texture").rgba;
    let day = tex("spot01_scene", "gKakarikoVillageDayWindowTex");
    let night = tex("spot01_scene", "gKakarikoVillageNightWindowTex");
    let l0 = room_textures_from(&pack, "spot01_scene", 0, 0x0800_0000);
    let l1 = room_textures_from(&pack, "spot01_scene", 1, 0x0800_0000);
    assert!(!l0.is_empty() && l0.iter().all(|t| *t == day), "layer 0's windows: the day's");
    assert!(!l1.is_empty() && l1.iter().all(|t| *t == night), "layer 1's: the night's");
    // segment_textures follows nightFlag inside a layer.
    let st = |night| DrawConfigState { child: true, night, ..Default::default() };
    let t = scene_table::segment_textures("SDC_KAKARIKO_VILLAGE", &st(true));
    assert_eq!((t.len(), t[0].segment, t[0].xlu, t[0].symbol), (1, 0x08, false, "gKakarikoVillageNightWindowTex"));
    // Scene_DrawConfigMarketGuardHouse: segment 8 sGuardHouseView2Textures[var], 9 View1's; var
    // nightFlag for a child, 1 for an adult.
    let v2 = room_textures_from(&pack, "miharigoya_scene", 0, 0x0800_0000);
    let v1 = room_textures_from(&pack, "miharigoya_scene", 0, 0x0900_0000);
    assert!(!v2.is_empty() && v2.iter().all(|t| *t == tex("miharigoya_scene", "gGuardHouseOutSideView2DayTex")));
    assert!(!v1.is_empty() && v1.iter().all(|t| *t == tex("miharigoya_scene", "gGuardHouseOutSideView1DayTex")));
    let adult = DrawConfigState { child: false, night: false, ..Default::default() };
    let t = scene_table::segment_textures("SDC_MARKET_GUARD_HOUSE", &adult);
    assert_eq!(t.iter().map(|t| (t.segment, t.symbol)).collect::<Vec<_>>(), vec![(0x08, "gGuardHouseOutSideView2NightTex"), (0x09, "gGuardHouseOutSideView1NightTex")]);
    // Scene_DrawConfigLonLonRanch: segment 8 (OPA) sLonLonRanchWindowTextures[nightFlag], the
    // day's on layer 0 and the night's on layer 1. The windows' material also sets a second tile
    // of one texel (4-bit) at the same address: left out here.
    let windows = |layer| room_textures_from(&pack, "spot20_scene", layer, 0x0800_0000).into_iter().filter(|t| t.len() > 4).collect::<Vec<_>>();
    let (r0, r1) = (windows(0), windows(1));
    assert!(!r0.is_empty() && r0.iter().all(|t| *t == tex("spot20_scene", "gLonLonRanchDayWindowTex")), "the ranch's day windows");
    assert!(!r1.is_empty() && r1.iter().all(|t| *t == tex("spot20_scene", "gLonLonRangeNightWindowsTex")), "the ranch's night windows");
    let t = scene_table::segment_textures("SDC_LON_LON_RANCH", &st(true));
    assert_eq!((t.len(), t[0].segment, t[0].xlu, t[0].symbol), (1, 0x08, false, "gLonLonRangeNightWindowsTex"));
    // Scene_DrawConfigLonLonBuildings: segment 8 (XLU) sLonLonHouseEntranceTextures[nightFlag].
    // Talon's house has one header, baked by day.
    let h0 = room_textures_from(&pack, "souko_scene", 0, 0x0800_0000);
    assert!(!h0.is_empty() && h0.iter().all(|t| *t == tex("souko_scene", "gLonLonHouseDayEntranceTex")), "Talon's house's entrance");
    let t = scene_table::segment_textures("SDC_LON_LON_BUILDINGS", &st(true));
    assert_eq!((t.len(), t[0].segment, t[0].xlu, t[0].symbol), (1, 0x08, true, "gLonLonHouseNightEntranceTex"));
    // Scene_DrawConfigGoronCity: segment 8 (XLU) sGoronCityEntranceTextures[nightFlag] (its OPA
    // segment 8 is a scroll), the day's on layer 0 and the night's on layer 1.
    let (g0, g1) = (room_textures_from(&pack, "spot18_scene", 0, 0x0800_0000), room_textures_from(&pack, "spot18_scene", 1, 0x0800_0000));
    assert!(!g0.is_empty() && g0.iter().all(|t| *t == tex("spot18_scene", "gGoronCityDayEntranceTex")), "Goron City's day entrance");
    assert!(!g1.is_empty() && g1.iter().all(|t| *t == tex("spot18_scene", "gGoronCityNightEntranceTex")), "Goron City's night entrance");
    let t = scene_table::segment_textures("SDC_GORON_CITY", &st(true));
    assert_eq!((t.len(), t[0].segment, t[0].xlu, t[0].symbol), (1, 0x08, true, "gGoronCityNightEntranceTex"));
    // Scene_DrawConfigDodongosCavern: segment 8 (OPA) gDCEntranceTextures[nightFlag] and segment
    // 9 (OPA) sDCLavaFloorTextures[(gameplayFrames & 14) >> 1]. The cavern has one set of
    // placements on its four headers; the rooms are baked by day at frame 0 (the first frame).
    let (e0, l0) = (room_textures_from(&pack, "ddan_scene", 0, 0x0800_0000), room_textures_from(&pack, "ddan_scene", 0, 0x0900_0000));
    assert!(!e0.is_empty() && e0.iter().all(|t| *t == tex("ddan_scene", "gDCDayEntranceTex")), "the cavern's day entrance");
    assert!(!l0.is_empty() && l0.iter().all(|t| *t == tex("ddan_scene", "gDCLavaFloor1Tex")), "the cavern's lava, frame 0");
    for (f, frame) in [(0, 1), (1, 1), (2, 2), (3, 2), (13, 7), (14, 8), (15, 8), (16, 1), (31, 8)] {
        let t = scene_table::segment_textures("SDC_DODONGOS_CAVERN", &DrawConfigState { gameplay_frames: f, ..st(true) });
        let lava = format!("gDCLavaFloor{frame}Tex");
        assert_eq!(
            t.iter().map(|t| (t.segment, t.xlu, t.symbol.to_string())).collect::<Vec<_>>(),
            vec![(0x08, false, "gDCNightEntranceTex".to_string()), (0x09, false, lava)],
            "frame {f}"
        );
    }
    // Scene_DrawConfigZorasDomain: segment 8 (XLU) sZorasDomainEntranceTextures[nightFlag] (its
    // OPA segment 0xC is a scroll). The Domain's child layers share one header (layer 1's is
    // layer 0's), baked by day: the night's only by the swap.
    let z0 = room_textures_from(&pack, "spot07_scene", 0, 0x0800_0000);
    assert!(!z0.is_empty() && z0.iter().all(|t| *t == tex("spot07_scene", "gZorasDomainDayEntranceTex")), "Zora's Domain's day entrance");
    let t = scene_table::segment_textures("SDC_ZORAS_DOMAIN", &st(true));
    assert_eq!((t.len(), t[0].segment, t[0].xlu, t[0].symbol), (1, 0x08, true, "gZorasDomainNightEntranceTex"));
    assert!(pack.texture("spot07_scene", "gZorasDomainNightEntranceTex").is_ok(), "the night's entrance in the pack");
}

#[test]
fn the_cavern_s_skull_eyes_glow_by_the_room_s_draw_params() {
    // Scene_DrawConfigDodongosCavern: gSPSegment(POLY_OPA_DISP, 0x0B/0x0C, a list with
    // gDPSetEnvColor(255, 255, 255, play->roomCtx.drawParams[BGDODOAGO_EYE_LEFT/RIGHT])). Bg_Dodoago
    // raises each by 5 a frame to 255 as its eye lights; they start at 0 (Room_Init).
    for (left, right) in [(0, 0), (5, 0), (130, 255), (255, 255)] {
        let mut st = DrawConfigState { room_draw_params: [left, right], ..state(0) };
        let [opa, xlu] = scene_table::segment_values("SDC_DODONGOS_CAVERN", &mut st).unwrap();
        assert_eq!((opa.env[0xB], opa.env[0xC]), (Some([255, 255, 255, left as u8]), Some([255, 255, 255, right as u8])));
        assert_eq!(xlu.env[0xB], None);
        // The config keeps no state of its own in drawParams.
        assert_eq!(st.room_draw_params, [left, right]);
    }
    // The two scrolls: segment 9 (XLU) by gameplayFrames % 256 and % 128 (64 by 32), 0xA (OPA)
    // by gameplayFrames % 128 and 2 * gameplayFrames % 128 (32 by 32): Gfx_TwoTexScroll puts them
    // in gDPSetTileSize's uls and ult as they are.
    let [opa, xlu] = scene_table::segment_values("SDC_DODONGOS_CAVERN", &mut state(300)).unwrap();
    assert_eq!((xlu.tiles[9][0], xlu.tiles[9][1]), (Some((300 % 256, 0)), Some((0, 300 % 128))));
    assert_eq!((opa.tiles[0xA][0], opa.tiles[0xA][1]), (Some((0, 300 % 128)), Some((0, 600 % 128))));
}

#[test]
fn kakariko_windows_stand_halfway_to_their_walls() {
    let Ok(pack) = GamePack::open_default() else { return };
    // A departure (the user's call, 2026-10-13): the ROM's window panels (textured from segment 8)
    // stand 6 to 10 units in front of their walls; the import moves each halfway, so every gap
    // to the wall behind is 3 to 5.
    for layer in [0, 1] {
        let s = pack.scene("spot01_scene").unwrap();
        let (mut walls, mut wins) = (Vec::new(), Vec::new());
        for k in &s.layers[layer].rooms {
            let r = pack.room(k).unwrap();
            for d in r.entries.iter().flat_map(|e| [e.opa.as_ref(), e.xlu.as_ref()]).flatten() {
                for b in &d.batches {
                    let win = d.materials[b.material].textures.iter().flatten().any(|t| d.textures[t.image].source_addr == Some(0x0800_0000));
                    for t in b.vertices.chunks_exact(3) {
                        let tri = [t[0].pos, t[1].pos, t[2].pos];
                        if win { wins.push(tri) } else { walls.push(tri) }
                    }
                }
            }
        }
        let normal = |t: &[Vec3; 3]| (t[1] - t[0]).cross(t[2] - t[0]).normalize_or_zero();
        assert!(!wins.is_empty());
        for w in &wins {
            let (n, p) = (normal(w), (w[0] + w[1] + w[2]) / 3.0);
            let gap = walls
                .iter()
                .filter(|t| normal(t).dot(n) > 0.95)
                .filter_map(|t| {
                    let tn = normal(t);
                    let d = (p - t[0]).dot(tn);
                    let q = p - tn * d;
                    let inside = (0..3).all(|i| (t[(i + 1) % 3] - t[i]).cross(q - t[i]).dot(tn) >= -0.01);
                    (inside && (0.5..30.0).contains(&d)).then_some(d)
                })
                .reduce(f32::min);
            let gap = gap.expect("a wall behind each window");
            assert!((2.7..5.05).contains(&gap), "layer {layer}: a window at {p} (facing {n}) is {gap} from its wall");
        }
    }
}

#[test]
fn rooms_carry_their_wind_settings() {
    let Ok(pack) = GamePack::open_default() else { return };
    // spot16_room_0: SCENE_CMD_WIND_SETTINGS(-100, -1, 20, 130) (the pack's bytes, read as the
    // C's s8 x, y, z and its speed); Goron City's rooms have none (windSpeed stays 0, the
    // zeroed play state's: Environment_Init doesn't set it).
    let room = |scene: &str, layer: usize, i: usize| pack.room(&pack.scene(scene).expect("scene").layers[layer].rooms[i]).expect("room");
    assert_eq!(room("spot16_scene", 0, 0).wind, Some(([-100, -1, 20], 130)));
    assert!((0..4).all(|i| room("spot18_scene", 0, i).wind.is_none()));
}

#[test]
fn jabu_jabu_breathes_and_its_pulse_quickens_after_a_hit() {
    // Scene_DrawConfigJabuJabu (z_scene_table.c): the statics start at D_8012A39C 538,
    // D_8012A3A0 4272, D_8012A398 0.0; each frame both phases go up 1820 and the pulse 0.15 plus
    // drawParams[1] thousandths.
    let mut st = DrawConfigState { scene_id: 0x02, ..state(0) };
    scene_table::segment_values("SDC_JABU_JABU", &mut st).unwrap();
    assert_eq!((st.jabu.d_8012a39c, st.jabu.d_8012a3a0), (538 + 1820, 4272 + 1820));
    let temp = 0.020000001f32;
    let turn = (360.00018f32 / 65535.0) * (std::f32::consts::PI / 180.0);
    let (a, c) = (2358i16, 6092i16);
    let d = st.distortion.expect("View_SetDistortion*");
    assert_eq!(d.orientation, [turn * temp * eng_math::cos_s(a), turn * temp * eng_math::sin_s(a), turn * temp * eng_math::sin_s(c)]);
    assert_eq!(d.scale, [1.0 + 0.79999995 * temp * eng_math::sin_s(c), 1.0 + 0.39999998 * temp * eng_math::cos_s(c), 1.0 + temp * eng_math::cos_s(a)]);
    assert_eq!(d.speed, 0.95);
    // Matrix_Scale(1.005f, sinf(D_8012A398) * 0.8f, 1.005f) but in room 2, as MATRIX_FINALIZE's s15.16.
    let fixed = |v: f32| (v * 65536.0) as i32 as f32 / 65536.0;
    assert_eq!(st.jabu.d_8012a398, 0.15);
    let m = st.segment_matrix.expect("segment 0x0D");
    assert_eq!(m, glam::Mat4::from_scale(Vec3::new(fixed(1.005), fixed(0.15f32.sin() * 0.8), fixed(1.005))));
    assert_eq!(fixed(1.005), 65863.0 / 65536.0);
    st.cur_room = 2;
    scene_table::segment_values("SDC_JABU_JABU", &mut st).unwrap();
    assert_eq!(st.segment_matrix.unwrap().x_axis.x, 1.0);
    // A hit on the flesh (func_8002F9EC: drawParams[0] 1): drawParams[1] up 200 a frame to 1200,
    // then (drawParams[0] 2) down 30 a frame to 0, and back to 0, 0.
    st.room_draw_params = [1, 0];
    let mut seen = Vec::new();
    let mut pulse = st.jabu.d_8012a398;
    for _ in 0..50 {
        scene_table::segment_values("SDC_JABU_JABU", &mut st).unwrap();
        pulse += 0.15 + st.room_draw_params[1] as f32 * 0.001;
        assert_eq!(st.jabu.d_8012a398, pulse);
        seen.push(st.room_draw_params);
    }
    assert_eq!(&seen[..7], &[[1, 200], [1, 400], [1, 600], [1, 800], [1, 1000], [1, 1200], [2, 1200]]);
    assert_eq!(seen[7], [2, 1170]);
    assert_eq!(seen[46], [2, 0]);
    assert_eq!(seen[47], [0, 0]);
    // Barinade's room (SCENE_JABU_JABU_BOSS 0x13): one scroll on segment 8, Gfx_TexScroll((127 -
    // frames) % 128, frames % 128, 32, 32) with the frames a u32.
    let mut boss = DrawConfigState { scene_id: 0x13, ..state(130) };
    let [opa, xlu] = scene_table::segment_values("SDC_JABU_JABU", &mut boss).unwrap();
    assert_eq!(opa, { let mut v = eng_gfx::SegmentValues::default(); v.read(8, &scene_table::gfx_tex_scroll(125, 2, 32, 32)); v });
    assert_eq!(xlu, eng_gfx::SegmentValues::default());
}

#[test]
fn jabu_jabu_s_flesh_pulses_on_bones_of_its_own() {
    let Ok(pack) = GamePack::open_default() else { return };
    // The rooms' lists multiply the matrix of segment 0x0D (gSPMatrix(0x0D000000, G_MTX_NOPUSH |
    // G_MTX_MUL | G_MTX_MODELVIEW)) onto one they load from the room's file: Jabu-Jabu's rooms 2
    // (8 times) and 8 to 12 (17 each), Barinade's room 1 (8), all in their OPA lists. Each load
    // is a bone, its base the room's matrix (bdan_room_2's first, at 0x3070: a translation to
    // (-75.6, -340, -2825)), the vertices under it in their own space (the panels 20 thick).
    let check = |scene: &str, want: &[(usize, usize)]| {
        let s = pack.scene(scene).expect("scene");
        for (i, k) in s.layers[0].rooms.iter().enumerate() {
            let r = pack.room(k).expect("room");
            let n: usize = r.entries.iter().filter_map(|e| e.opa.as_ref()).map(|d| d.site_bases.len()).sum();
            let n_xlu: usize = r.entries.iter().filter_map(|e| e.xlu.as_ref()).map(|d| d.site_bases.len()).sum();
            assert_eq!(n, want.iter().find(|(w, _)| *w == i).map_or(0, |(_, n)| *n), "{scene} room {i}");
            assert_eq!(n_xlu, 0, "{scene} room {i}");
        }
    };
    check("bdan_scene", &[(2, 8), (8, 17), (9, 17), (10, 17), (11, 17), (12, 17)]);
    check("bdan_boss_scene", &[(1, 8)]);
    let s = pack.scene("bdan_scene").unwrap();
    let r = pack.room(&s.layers[0].rooms[2]).unwrap();
    let d = r.entries[0].opa.as_ref().unwrap();
    let base = glam::Mat4::from_cols_array(&d.site_bases[0]);
    assert!(base.w_axis.truncate().abs_diff_eq(Vec3::new(-75.60287, -340.0, -2824.9805), 0.001), "{base}");
    let under: Vec<_> = d.batches.iter().flat_map(|b| &b.vertices).filter(|v| v.bone == 0).collect();
    assert!(!under.is_empty());
    assert!(under.iter().all(|v| v.pos.y.abs() <= 10.0), "the panel in its own space");
    assert!(d.batches.iter().flat_map(|b| &b.vertices).all(|v| v.bone == eng_gfx::NO_BONE || (v.bone as usize) < d.site_bases.len()));
}
