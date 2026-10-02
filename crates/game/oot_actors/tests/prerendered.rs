//! Prerendered rooms and fixed cameras (GAME-02 milestone 2): Kokiri Forest's interiors and
//! the bg camera list, through `Play_Init` from the asset pack.
//!
//! Expected values come from the decomp and the scenes' data, quoted per test:
//! - `link_home_scene`'s `BgCamInfo` list: 0 `CAM_SET_PREREND_FIXED` at (-118, 345, 47), rotation
//!   (12743, 20389, 0), fov 4683; 1 `CAM_SET_PREREND_PIVOT` at (0, 34, 0), fov 6000; 2
//!   `CAM_SET_NONE` (the floors' index). Its `SCENE_CMD_MISC_SETTINGS` is
//!   `SCENE_CAM_TYPE_FIXED_TOGGLE_VIEWPOINT`, its skybox `SKYBOX_HOUSE_LINK`;
//! - `kokiri_shop_scene`: 0 `PREREND_FIXED` at (-100, 100, 260), rotation (2367, 28945, 0), fov
//!   5000; `SCENE_CAM_TYPE_FIXED_SHOP_VIEWPOINT`;
//! - `spot04_scene`: Link's porch's floor names bg camera 4, `CAM_SET_PIVOT_IN_FRONT` at
//!   (-97, 170, 906); spawn 1's params 0x0F05 name camera 5, `CAM_SET_START1` at
//!   (3778, 288, -608), fov 45, timer 10.
//!
//! (The lists are the ROM's; the extractor, `oot_extract::scenes`, reads the same values into
//! `collision.json`.)

mod common;

use std::sync::Arc;

use eng_input::pad::{BTN_CUP, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_game::camera::*;
use oot_game::play::{PlayState, VIEWPOINT_LOCKED, VIEWPOINT_PIVOT, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::scene::{SCENE_CAM_TYPE_FIXED_SHOP_VIEWPOINT, SCENE_CAM_TYPE_FIXED_TOGGLE_VIEWPOINT, ShapeKind};

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn enter(entrance: &str) -> Option<PlayState> {
    enter_with(entrance, |_| {})
}

fn enter_with(entrance: &str, f: impl FnOnce(&mut SaveContext)) -> Option<PlayState> {
    let a = assets()?;
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    f(&mut save);
    Some(oot_actors::play_entrance(a, common::data()?, common::rules()?, save).expect("Play_Init"))
}

fn frames(w: &mut PlayState, prev: &mut PadState, pad: PadState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(*prev, pad));
        *prev = pad;
    }
}

fn press(w: &mut PlayState, prev: &mut PadState, button: u16) {
    frames(w, prev, PadState { button, ..Default::default() }, 1);
    frames(w, prev, PadState::default(), 1);
}

/// `Camera_AddVecGeoToVec3f(dst, eye, {r, yaw, pitch})` in plain trigonometry: pitch up from the
/// horizon, yaw 0 along +z.
fn geo_add(eye: Vec3, r: f32, yaw: i16, pitch: i16) -> Vec3 {
    let (y, p) = (yaw as f32 * std::f32::consts::PI / 32768.0, pitch as f32 * std::f32::consts::PI / 32768.0);
    eye + Vec3::new(r * p.cos() * y.sin(), r * p.sin(), r * p.cos() * y.cos())
}

#[test]
fn link_home_has_its_bg_cameras_background_and_skybox() {
    let Some(w) = enter("ENTR_LINKS_HOUSE_1") else { return };
    let cams = &w.col.header.bg_cams;
    assert_eq!(cams.iter().map(|c| c.setting as i16).collect::<Vec<_>>(), [CAM_SET_PREREND_FIXED, CAM_SET_PREREND_PIVOT, CAM_SET_NONE]);
    let fixed = bg_cam_func_data(&w.col, 0).unwrap();
    assert_eq!((fixed.pos, fixed.rot, fixed.fov, fixed.flags), ([-118, 345, 47], [12743, 20389, 0], 4683, -1));
    let pivot = bg_cam_func_data(&w.col, 1).unwrap();
    assert_eq!((pivot.pos, pivot.fov), ([0, 34, 0], 6000));
    assert!(bg_cam_func_data(&w.col, 2).is_none(), "CAM_SET_NONE has no data");
    // The room: ROOM_SHAPE_TYPE_IMAGE, ROOM_SHAPE_IMAGE_AMOUNT_SINGLE, a 320x240 JPEG decoded
    // into a screen quad (two triangles).
    let s = w.scene.as_ref().unwrap();
    let room = &s.rooms[0];
    assert_eq!(room.shape, Some(ShapeKind::Image));
    assert_eq!(room.backgrounds.len(), 1);
    let bg = &room.backgrounds[0];
    assert_eq!(bg.bg_cam_index, None);
    assert_eq!(bg.mesh.triangle_count(), 2);
    let img = &bg.mesh.textures[0].image;
    assert_eq!((img.width, img.height), (320, 240));
    // RGBA5551 (Jpeg_Decode's output): at most 32 levels a channel, alpha set.
    assert!(img.rgba.chunks_exact(4).all(|p| p[3] == 255));
    for ch in 0..3 {
        let levels: std::collections::BTreeSet<u8> = img.rgba.chunks_exact(4).map(|p| p[ch]).collect();
        assert!(levels.len() <= 32, "channel {ch}: {} levels", levels.len());
    }
    assert!(img.rgba.chunks_exact(4).any(|p| p[..3] != img.rgba[..3]), "an image, not a fill");
    // SCENE_CMD_MISC_SETTINGS and the skybox (Skybox_Setup: SKYBOX_HOUSE_LINK sets drawType 1,
    // from vr_LHVR_static and vr_LHVR_pal_static).
    assert_eq!(w.scene_cam_type, SCENE_CAM_TYPE_FIXED_TOGGLE_VIEWPOINT);
    let a = w.assets.as_ref().unwrap();
    let sky = a.scenes.room_skybox(s.layer_data().skybox.skybox_id).expect("a room skybox");
    assert_eq!((sky.name.as_str(), sky.draw_type, sky.static_file.as_str(), sky.pal_file.as_str()), ("SKYBOX_HOUSE_LINK", 1, "vr_LHVR_static", "vr_LHVR_pal_static"));
    // Skybox_Calculate256: four faces, each two halves of 4x4 quads (gSP1Quadrangle: 2 triangles).
    let mesh: eng_gfx::DrawList = a.pack.assets.get(&oot_game::pack::keys::bake(&oot_game::skybox::bake_name(&sky.name))).unwrap();
    assert_eq!(mesh.triangle_count(), 4 * 2 * 16 * 2);
    // SETUPDL_40: G_RM_OPA_SURF without G_ZBUFFER: no depth test or write.
    assert!(mesh.materials.iter().all(|m| !m.depth_test && !m.depth_write));
}

#[test]
fn house_pivot_camera_and_the_c_up_toggle() {
    let Some(mut w) = enter("ENTR_LINKS_HOUSE_1") else { return };
    // Play_Init: func_80057FC4 gives a prerendered room CAM_SET_FREE0; spawn 1's params
    // 0x0EFF name no start camera; SCENE_CAM_TYPE_FIXED_TOGGLE_VIEWPOINT starts at
    // VIEWPOINT_PIVOT.
    assert_eq!((w.game_camera.setting, w.game_camera.bg_cam_index), (CAM_SET_FREE0, -1));
    assert_eq!(w.viewpoint, VIEWPOINT_PIVOT);
    let mut prev = PadState::default();
    frames(&mut w, &mut prev, PadState::default(), 1);
    // Play_RequestViewpointBgCam every frame: bg camera viewpoint - 1 = 1, PREREND_PIVOT;
    // Camera_Unique7: the eye at its position, the fov 60 whatever the data says.
    let c = &w.game_camera;
    assert_eq!((c.setting, c.bg_cam_index), (CAM_SET_PREREND_PIVOT, 1));
    assert_eq!(c.eye, Vec3::new(0.0, 34.0, 0.0));
    assert_eq!(c.fov, 60.0);
    // Unique7's at: the direction to Player at the distance to him, pitch
    // -rot.x * cos(yaw - rot.y) = 0.
    let p = w.player().actor.world_pos;
    assert!((c.at.y - 34.0).abs() < 0.01, "level: {:?}", c.at);
    let (to_at, to_p) = ((c.at - c.eye).with_y(0.0).normalize(), (p - c.eye).with_y(0.0).normalize());
    assert!(to_at.dot(to_p) > 0.9999, "at {:?} towards Player {p:?}", c.at);
    // No background with the pivot camera (Room_DrawImage draws it only for PREREND_FIXED).
    let room = w.scene.as_ref().unwrap().rooms[0].clone();
    assert_eq!(oot_game::room::image_background(&w.game_camera, &w.col, &room), None);

    // C-Up while Link walks in (PLAYER_STATE1_29: Player_InCsMode) is refused.
    assert!(w.player_in_cs_mode());
    press(&mut w, &mut prev, BTN_CUP);
    assert_eq!(w.viewpoint, VIEWPOINT_PIVOT);
    frames(&mut w, &mut prev, PadState::default(), 30);
    assert!(!w.player_in_cs_mode());
    // C-Up: VIEWPOINT_LOCKED, bg camera 0 (PREREND_FIXED): Camera_Fixed3's eye at its position,
    // at 150 along its rotation (pitch -rot.x), fov 4683 * 0.01.
    press(&mut w, &mut prev, BTN_CUP);
    assert_eq!(w.viewpoint, VIEWPOINT_LOCKED);
    let c = &w.game_camera;
    assert_eq!((c.setting, c.bg_cam_index), (CAM_SET_PREREND_FIXED, 0));
    assert_eq!(c.eye, Vec3::new(-118.0, 345.0, 47.0));
    let at = geo_add(c.eye, 150.0, 20389, -12743);
    // (Math_SinS / Math_CosS are table lookups: a fraction of a unit off float trigonometry.)
    assert!((c.at - at).length() < 0.5, "at {:?}, want {at:?}", c.at);
    assert!((c.fov - 46.83).abs() < 1e-4);
    assert_eq!(oot_game::room::image_background(&w.game_camera, &w.col, &room), Some(0));
    // And back.
    press(&mut w, &mut prev, BTN_CUP);
    assert_eq!((w.viewpoint, w.game_camera.setting, w.game_camera.bg_cam_index), (VIEWPOINT_PIVOT, CAM_SET_PREREND_PIVOT, 1));
}

#[test]
fn shop_starts_on_its_fixed_camera_and_refuses_c_up() {
    let Some(mut w) = enter("ENTR_KOKIRI_SHOP_0") else { return };
    // SCENE_CAM_TYPE_FIXED_SHOP_VIEWPOINT: VIEWPOINT_LOCKED; spawn 0's params 0x0E00 name start
    // camera 0 as well.
    assert_eq!(w.scene_cam_type, SCENE_CAM_TYPE_FIXED_SHOP_VIEWPOINT);
    assert_eq!(w.viewpoint, VIEWPOINT_LOCKED);
    let mut prev = PadState::default();
    frames(&mut w, &mut prev, PadState::default(), 40);
    let c = &w.game_camera;
    assert_eq!((c.setting, c.bg_cam_index), (CAM_SET_PREREND_FIXED, 0));
    assert_eq!(c.eye, Vec3::new(-100.0, 100.0, 260.0));
    assert!((c.fov - 50.0).abs() < 1e-4);
    // In a shop C-Up is NA_SE_SY_ERROR (the shopkeeper's browsing switches the viewpoint).
    press(&mut w, &mut prev, BTN_CUP);
    assert_eq!((w.viewpoint, w.game_camera.setting), (VIEWPOINT_LOCKED, CAM_SET_PREREND_FIXED));
    // The Kokiri shop's skybox (SKYBOX_KOKIRI_SHOP, past SKYBOX_HOUSE_KAKARIKO): two faces.
    let a = w.assets.clone().unwrap();
    let sky = a.scenes.room_skybox(w.scene.as_ref().unwrap().layer_data().skybox.skybox_id).unwrap();
    let mesh: eng_gfx::DrawList = a.pack.assets.get(&oot_game::pack::keys::bake(&oot_game::skybox::bake_name(&sky.name))).unwrap();
    assert_eq!((sky.name.as_str(), mesh.triangle_count()), ("SKYBOX_KOKIRI_SHOP", 2 * 2 * 16 * 2));
}

#[test]
fn porch_floor_gives_the_pivot_in_front_camera() {
    let Some(mut w) = enter("ENTR_KOKIRI_FOREST_3") else { return };
    // Play_Init's Camera_OverwriteStateFlags(0xFF) enables the floor's bg cameras (stateFlags bit 1).
    assert_eq!(w.game_camera.state_flags & 0xFF, 0xFF);
    let mut prev = PadState::default();
    frames(&mut w, &mut prev, PadState::default(), 1);
    // Standing on the porch (within 2 of the floor): its bg camera 4, PIVOT_IN_FRONT, whose
    // Camera_Fixed4 eases the eye to (-97, 170, 906) by speedToEyePos 0.5 a frame
    // (CAM_FUNCDATA_FIXD4(-40, 50, 80, 60, 0x0004)) at fov 60.
    let c = &w.game_camera;
    assert_eq!((c.setting, c.bg_cam_index), (CAM_SET_PIVOT_IN_FRONT, 4));
    assert_eq!(c.fov, 60.0);
    frames(&mut w, &mut prev, PadState::default(), 40);
    assert!((w.game_camera.eye - Vec3::new(-97.0, 170.0, 906.0)).length() < 0.01, "eye {:?}", w.game_camera.eye);
}

#[test]
fn start1_camera_until_link_moves() {
    // ENTR_KOKIRI_FOREST_1 is the Deku Tree's meadow: with EVENTCHKINF_0C unset, his first talk
    // (Bg_Treemouth's gDekuTreeMeetingCs) would start at once and hold Link. Met already.
    let Some(mut w) = enter_with("ENTR_KOKIRI_FOREST_1", |s| s.set_event_chk_inf(oot_game::save::EVENTCHKINF_0C)) else { return };
    // Spawn 1's params 0x0F05: Camera_RequestBgCam(5) in Play_Init, CAM_SET_START1.
    assert_eq!((w.game_camera.setting, w.game_camera.bg_cam_index), (0x20, 5));
    let mut prev = PadState::default();
    frames(&mut w, &mut prev, PadState::default(), 1);
    // Camera_Unique0: the eye at the camera's position, its fov 45.
    let c = &w.game_camera;
    assert_eq!((c.eye, c.fov), (Vec3::new(3778.0, 288.0, -608.0), 45.0));
    // sSetStart1ModeNormalData's flags 1: after the timer (10), once Player is 10 (xz) from where
    // it started, back to the previous setting (NORMAL0).
    let start = w.player().actor.world_pos;
    let mut moved = false;
    for _ in 0..80 {
        frames(&mut w, &mut prev, PadState { stick_y: 60, ..Default::default() }, 1);
        let p = w.player().actor.world_pos;
        if w.game_camera.setting != 0x20 {
            moved = ((p.x - start.x).powi(2) + (p.z - start.z).powi(2)).sqrt() >= 10.0;
            break;
        }
    }
    assert!(moved, "left START1 after moving 10: {:?}", w.game_camera.setting);
    assert_eq!(w.game_camera.setting, CAM_SET_NORMAL0);
}

#[test]
fn exits_use_the_scene_transition_camera_outside_only() {
    let Some(mut w) = enter("ENTR_KOKIRI_FOREST_3") else { return };
    let mut prev = PadState::default();
    frames(&mut w, &mut prev, PadState::default(), 30);
    // Into the house: Player_HandleExitsAndVoids → Player_RequestCameraSetting(play, CAM_SET_SCENE_TRANSITION), which
    // Play_CamIsNotFixed lets through in Kokiri Forest.
    let (_, door) = oot_actors::script::exit_to(&w, "ENTR_LINKS_HOUSE_1").unwrap();
    let mut setting = None;
    for _ in 0..60 {
        let pad = oot_actors::script::stick_towards(&w, door, 60.0);
        frames(&mut w, &mut prev, pad, 1);
        if w.transition.trigger != 0 {
            setting = Some(w.game_camera.setting);
            break;
        }
    }
    assert_eq!(setting, Some(CAM_SET_SCENE_TRANSITION));
    // Camera_Unique2 (CAM_FUNCDATA_UNIQ2(-20, 150, 60, 0x0210)): flags 0x10 clear stateFlags bit 4
    // (no floor cameras while it runs).
    frames(&mut w, &mut prev, PadState::default(), 1);
    assert_eq!(w.game_camera.state_flags & 4, 0);
    // Out of the house: the room is prerendered, so the setting stays (Interface_ChangeHudVisibilityMode
    // only).
    let mut w = enter("ENTR_LINKS_HOUSE_1").unwrap();
    frames(&mut w, &mut prev, PadState::default(), 40);
    let (_, out) = oot_actors::script::exit_to(&w, "ENTR_KOKIRI_FOREST_3").unwrap();
    for _ in 0..80 {
        let pad = oot_actors::script::stick_towards(&w, out, 60.0);
        frames(&mut w, &mut prev, pad, 1);
        if w.transition.trigger != 0 {
            break;
        }
    }
    assert_ne!(w.transition.trigger, 0, "Link reached the exit");
    assert_eq!(w.game_camera.setting, CAM_SET_PREREND_PIVOT);
}
