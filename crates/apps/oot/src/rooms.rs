//! Drawing a play state's scene (`oot_game::play_scene::SceneState`): the environment's lights
//! and fog, and the rooms' draws in `Room_Draw`'s order, with the scene draw config's segment
//! values for this frame; a prerendered room's background (`Room_DrawImage`) and the room
//! skybox around the eye (docs/adr/0014-prerendered-backgrounds.md).

use anyhow::Result;
use eng_gfx::{DrawCmd, DrawList, DrawLists, MeshKey};
use eng_math::gu_perspective;
use eng_render::{Fog, Lighting};
use glam::{Mat4, Vec3};
use oot_game::env::{self, EnvTables};
use oot_game::pack::GamePack;
use oot_game::play::PlayState;
use oot_game::play_scene::SceneState;
use oot_game::camera::CAM_SET_PREREND_FIXED;
use oot_game::room::cullable_order;
use oot_game::scene::{RoomData, ShapeKind, layer_for};

/// `View_Init`: `zNear` 10 (the far plane is `lightCtx.fogFar`, set in `Play_Draw`).
pub const Z_NEAR: f32 = 10.0;
/// `View_Init`: `fovy` 60.
pub const FOVY: f32 = 60.0;

/// Loads `name` for Link's age at `day_time` the spikes' way: the layer the game picks for a
/// non-cutscene entrance, every room drawn.
pub fn load_all_rooms(pack: &GamePack, env_tables: &EnvTables, name: &str, child: bool, day_time: u16) -> Result<SceneState> {
    // Environment_Init: nightFlag = dayTime > 18:00 || dayTime < 6:30.
    let night = day_time > env::clock_time(18, 0) as u16 || day_time < env::clock_time(6, 30) as u16;
    let mut s = SceneState::load(pack, env_tables, name, layer_for(child, night), child, night, day_time)?;
    s.all_rooms = true;
    Ok(s)
}

/// The renderer's lights and fog for the scene's environment.
pub fn lighting(s: &SceneState) -> Lighting {
    let l = &s.lights;
    let c = |v: [u8; 3]| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32) / 255.0;
    let d = |v: [i8; 3]| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32);
    // Play_SetFog: Gfx_SetFog2(fogColor, 0, fogNear, 1000).
    let (fm, fo) = env::fog_factor(l.fog_near as i32, 1000);
    Lighting {
        dir: d(l.light1_dir),
        color: c(l.light1_color),
        dir2: d(l.light2_dir),
        color2: c(l.light2_color),
        ambient: c(l.ambient),
        fog: Some(Fog { color: c(l.fog_color), multiplier: fm as f32, offset: fo as f32, near: Z_NEAR, far: fog_far(s) }),
    }
}

/// `lightCtx.fogFar`, which `Play_Draw` also uses as the far plane.
pub fn fog_far(s: &SceneState) -> f32 {
    s.lights.fog_far as f32
}

/// Background: with no skybox drawn (`SKYBOX_UNSET_1D`, or skyboxes not ported),
/// `Environment_DrawSkyboxFilters` fills the screen with the fog colour.
pub fn clear_color(s: &SceneState) -> [f64; 4] {
    let f = s.lights.fog_color;
    [f[0] as f64 / 255.0, f[1] as f64 / 255.0, f[2] as f64 / 255.0, 1.0]
}

/// The rooms `Play_Draw` draws: the room context's current room, then the previous one (or
/// every room with a shape, in the spikes' view).
pub fn drawn_rooms<'a>(play: &'a PlayState, s: &'a SceneState) -> Vec<&'a RoomData> {
    if s.all_rooms {
        return s.rooms.iter().filter(|r| r.shape.is_some()).map(|r| r.as_ref()).collect();
    }
    play.room_ctx.drawn().filter_map(|n| s.room(n)).filter(|r| r.shape.is_some()).map(|r| r.as_ref()).collect()
}

/// A room entry's mesh key: `room/<scene>/<layer>/<room>/<entry>/<opa|xlu>`.
fn entry_key(s: &SceneState, room: usize, entry: usize, xlu: bool) -> String {
    format!("room/{}/{}/{room}/{entry}/{}", s.data.name, s.layer, if xlu { "xlu" } else { "opa" })
}

/// `Room_Draw` for this view: each drawn room's entries in shape order, cullable shapes z-sorted
/// and depth-culled (`Room_DrawCullable`), OPA and XLU, with the draw config's segment values
/// for this frame. Returns how many entries were drawn.
pub fn submit_rooms(play: &PlayState, s: &SceneState, view: Mat4, out: &mut DrawLists) -> usize {
    let far = fog_far(s);
    let vp = gu_perspective(FOVY, 4.0 / 3.0, Z_NEAR, far) * view;
    let clip_z = |p: Vec3| (vp * p.extend(1.0)).z;
    let mut drawn = 0;
    for room in drawn_rooms(play, s) {
        let order: Vec<usize> = match room.shape {
            Some(ShapeKind::Cullable) => {
                let bounds: Vec<_> = room.entries.iter().map(|e| e.bounds).collect();
                cullable_order(&bounds, clip_z, far)
            }
            _ => (0..room.entries.len()).collect(),
        };
        drawn += order.len();
        for i in order {
            let e = &room.entries[i];
            for (k, (d, list)) in [(&e.opa, &mut out.opa), (&e.xlu, &mut out.xlu)].into_iter().enumerate() {
                if d.as_ref().is_some_and(|d| !d.batches.is_empty()) {
                    let mut cmd = DrawCmd::new(MeshKey::named(entry_key(s, room.index, i, k == 1)), Mat4::IDENTITY);
                    cmd.params.segments = s.segments.as_ref().map(|v| v[k].clone());
                    list.push(cmd);
                }
            }
        }
        // Room_DrawImage: after the room's opaque list, its background (the opaque geometry
        // keeps only its depth under it).
        if room.shape == Some(ShapeKind::Image)
            && !s.all_rooms
            // The image only fits the game camera's view (not the sandbox's follow camera).
            && play.camera_kind == oot_game::camera::CameraKind::Game
            && let Some(b) = oot_game::room::image_background(&play.game_camera, &play.col, room)
        {
            let mut cmd = DrawCmd::new(MeshKey::named(format!("room/{}/{}/{}/bg{b}", s.data.name, s.layer, room.index)), Mat4::IDENTITY);
            cmd.params.screen = true;
            out.opa.push(cmd);
        }
    }
    drawn
}

/// `Play_Draw`'s room skybox (`skyboxCtx.unk_140 != 0`: the houses' and shops' 360° images),
/// after the rooms, when the active camera isn't `CAM_SET_PREREND_FIXED`: centred on the eye.
pub fn submit_room_skybox(play: &PlayState, s: &SceneState, eye: Vec3, out: &mut DrawLists) -> bool {
    if s.all_rooms || play.game_camera.setting == CAM_SET_PREREND_FIXED {
        return false;
    }
    let Some(sky) = play.assets.as_ref().and_then(|a| a.scenes.room_skybox(s.layer_data().skybox.skybox_id)) else { return false };
    let key = oot_game::pack::keys::bake(&oot_game::skybox::bake_name(&sky.name));
    out.opa.push(DrawCmd::new(MeshKey::named(key), Mat4::from_translation(eye)));
    true
}

/// The mesh of a `room/<scene>/<layer>/<room>/<entry>/<opa|xlu>` key (without the `room/`).
pub fn entry_mesh(s: &SceneState, key: &str) -> Option<DrawList> {
    let f: Vec<&str> = key.split('/').collect();
    let [scene, layer, room, entry, kind] = f[..] else { return None };
    if scene != s.data.name || layer.parse::<usize>().ok()? != s.layer {
        return None;
    }
    let r = s.rooms.iter().find(|r| Some(r.index) == room.parse().ok())?;
    let e = r.entries.get(entry.parse::<usize>().ok()?)?;
    if kind == "opa" { e.opa.clone() } else { e.xlu.clone() }
}

/// The mesh of a `room/<scene>/<layer>/<room>/bg<i>` key (without the `room/`): a background.
pub fn background_mesh(s: &SceneState, key: &str) -> Option<DrawList> {
    let f: Vec<&str> = key.split('/').collect();
    let [scene, layer, room, bg] = f[..] else { return None };
    if scene != s.data.name || layer.parse::<usize>().ok()? != s.layer {
        return None;
    }
    let r = s.rooms.iter().find(|r| Some(r.index) == room.parse().ok())?;
    Some(r.backgrounds.get(bg.strip_prefix("bg")?.parse::<usize>().ok()?)?.mesh.clone())
}

/// Every room's triangles (the HUD's count).
pub fn triangles(s: &SceneState) -> usize {
    s.rooms.iter().filter(|r| r.shape.is_some()).map(|r| r.triangles()).sum()
}

/// Blends a screen fill (a transition's fade) over RGBA8 pixels, as `TransitionFade_Draw`'s
/// `G_RM_CLD_SURF` rectangle does.
pub fn apply_fill(px: &mut [u8], fill: [u8; 4]) {
    let a = fill[3] as u32;
    for p in px.chunks_exact_mut(4) {
        for c in 0..3 {
            p[c] = ((p[c] as u32 * (255 - a) + fill[c] as u32 * a) / 255) as u8;
        }
    }
}
