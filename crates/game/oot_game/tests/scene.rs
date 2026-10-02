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
