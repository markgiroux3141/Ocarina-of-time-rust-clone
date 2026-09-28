//! Helpers for scripted play (the tests and the sandbox's headless runs): steering the stick
//! towards a point, as a player would, and finding a scene's exits.

use eng_collision::bgcheck::{BGCHECK_SCENE, PolyId};
use eng_input::pad::PadState;
use glam::Vec3;
use oot_game::play::PlayState;
use oot_game::surface::SurfaceType;

use crate::PlayExt;

/// The stick pointing Player towards `to` at magnitude `mag`, relative to the camera as
/// `func_8083315C` reads it (`stick_to_mag_angle`: angle = `Math_Atan2S(stickY, -stickX)`, plus
/// the camera's input yaw).
pub fn stick_towards(play: &PlayState, to: Vec3, mag: f32) -> PadState {
    let p = play.player().actor.world_pos;
    let yaw = eng_math::vec3f_yaw(p, to);
    let rel = eng_math::binang_to_rad(yaw.wrapping_sub(play.input_dir_yaw()));
    PadState { button: 0, stick_x: (-rel.sin() * mag) as i8, stick_y: (rel.cos() * mag) as i8 }
}

/// The centre of the scene's polys that lead to exit list entry `exit` (1-based), if any.
pub fn exit_centre(play: &PlayState, exit: u32) -> Option<Vec3> {
    let col = &play.col;
    let mut c = Vec3::ZERO;
    let mut n = 0.0;
    for (i, poly) in col.header.polys.iter().enumerate() {
        if col.exit_index(PolyId { bg: BGCHECK_SCENE, idx: i as u16 }) == exit {
            for &v in &poly.vtx {
                c += col.header.vertex((v & 0x1FFF) as usize);
                n += 1.0;
            }
        }
    }
    (n > 0.0).then(|| c / n)
}

/// The exit (1-based) of the current scene that leads to the entrance `name`, and where its
/// floor is: the first exit list entry for it that some floor leads to.
pub fn exit_to(play: &PlayState, name: &str) -> Option<(u32, Vec3)> {
    let e = play.assets.as_ref()?.scenes.entrance_index(name)?;
    play.exit_list().iter().enumerate().filter(|(_, x)| **x == e).find_map(|(i, _)| exit_centre(play, i as u32 + 1).map(|c| (i as u32 + 1, c)))
}
