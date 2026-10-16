//! GAME-09 milestone 2 against the C: `En_Hata` (`z_en_hata.c`), the flagpole on Death Mountain
//! Trail (spot16 room 0: (-746, 1273, 515), yaw -5461), moved by the room's wind
//! (`SCENE_CMD_WIND_SETTINGS`: direction (-100, -1, 20), speed 130).
//!
//! Entered by `ENTR_DEATH_MOUNTAIN_TRAIL_0` on a `cavern-bombs` save with the trail's intro seen.
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_hata::{EnHata, FLAGPOLE_LIMB_FLAG_1_BASE, FLAGPOLE_LIMB_FLAG_1_HOIST_END_BASE, FLAGPOLE_LIMB_FLAG_2_BASE, FLAGPOLE_LIMB_FLAG_2_HOIST_END_BASE};
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::env::clock_time;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

const EVENTCHKINF_A1: u16 = 0xA1;
const FLAGPOLE: Vec3 = Vec3::new(-746.0, 1273.0, 515.0);

fn trail(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DEATH_MOUNTAIN_TRAIL_0").expect("entrance");
    let mut save = SaveContext::new(e, false, clock_time(10, 0) as u16);
    save.apply_preset("cavern-bombs").unwrap();
    save.set_event_chk_inf(EVENTCHKINF_A1);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    idle(&mut w, 3);
    // Link 150 north of the pole (downhill) facing it (FLAGS 0: culled out of view, it doesn't
    // update).
    let at = FLAGPOLE + Vec3::new(0.0, 0.0, 150.0);
    let (y, _) = w.col.entity_raycast_down(at + Vec3::Y * 600.0);
    w.place_player(Vec3::new(at.x, y, at.z), i16::MIN);
    idle(&mut w, 1);
    w
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn flag(w: &PlayState) -> ActorHandle {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnHata>(h).is_some()).expect("the flagpole")
}

fn hata(w: &PlayState) -> &EnHata {
    w.actors.downcast::<EnHata>(flag(w)).unwrap()
}

/// The meshes the last frame draws.
fn drawn(w: &PlayState) -> Vec<String> {
    let f = w.current_frame();
    let mut out = oot_game::play::DrawOut::default();
    w.draw(&f, &oot_game::play::ViewInfo::new(f.view.eye, glam::Mat4::IDENTITY), &mut out);
    out.opa.iter().chain(out.xlu.iter()).map(|c| c.mesh.name.clone()).collect()
}

/// `Math_Atan2S`-free expectations: the wind's pitch and yaw as the C computes them, in floats.
/// `Math_Vec3f_Pitch(0, (-100, -1, 20))` = atan2(1, √10400) ≈ 0.0098 rad ≈ 102; its yaw
/// `Math_Vec3f_Yaw` = atan2(-100, 20) ≈ -1.3734 rad ≈ -14317.
fn binang(rad: f64) -> f64 {
    rad * 32768.0 / std::f64::consts::PI
}

#[test]
fn the_trail_s_wind_turns_the_flag() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    // Scene_CommandWindSettings: the room's wind.
    assert_eq!((w.env_ctx.wind_direction, w.env_ctx.wind_speed), ([-100, -1, 20], 130.0));
    let h = hata(&w);
    // EnHata_Init: scale 1/75, gFlagpoleCol, the culling volume, the step 6, 1000, 1.
    assert_eq!(h.actor.world_pos, FLAGPOLE);
    assert_eq!((h.actor.scale.x, h.inv_scale, h.max_step, h.min_step), (1.0 / 75.0, 6, 1000, 1));
    assert_eq!((h.actor.culling_volume_scale, h.actor.culling_volume_downward, h.actor.culling_volume_distance), (500.0, 550.0, 2200.0));
    assert!(h.dyna_bg_id().is_some_and(|bg| w.col.dyna.is_bg_actor(bg)));
    let pitch = binang((1.0f64).atan2((10400.0f64).sqrt()));
    let yaw = binang((-100.0f64).atan2(20.0));
    // The hoist ends' target: -pitch + (15000 + pitch) × (1 - 130 / (255 - 80 sin(unk_278))),
    // between 15102 × (1 - 130/175) - 102 ≈ 3779 and 15102 × (1 - 130/335) - 102 ≈ 9139. After 60
    // frames (steps of a sixth, at most 1000, from 0) the limbs hover in that range.
    idle(&mut w, 60);
    for _ in 0..60 {
        idle(&mut w, 1);
        let h = hata(&w);
        let l = &h.limbs;
        // Both cloths hang down (their bases -0x4000 on y), their hoist ends together.
        assert_eq!((l[FLAGPOLE_LIMB_FLAG_1_BASE][1], l[FLAGPOLE_LIMB_FLAG_2_BASE][1]), (-0x4000, -0x4000));
        assert_eq!(l[FLAGPOLE_LIMB_FLAG_1_HOIST_END_BASE], l[FLAGPOLE_LIMB_FLAG_2_HOIST_END_BASE]);
        let y = l[FLAGPOLE_LIMB_FLAG_1_HOIST_END_BASE][1] as f64;
        assert!((15102.0 * (1.0 - 130.0 / 175.0) - pitch - 8.0..=15102.0 * (1.0 - 130.0 / 335.0) - pitch + 8.0).contains(&y), "hoist end y {y}");
        // Turned to the wind: z = -yaw (Math_Atan2S's table is a few units off the float).
        assert!((l[FLAGPOLE_LIMB_FLAG_1_HOIST_END_BASE][2] as f64 - -yaw).abs() <= 8.0, "{} {}", l[FLAGPOLE_LIMB_FLAG_1_HOIST_END_BASE][2], -yaw);
        // The flapping's speed: (2.75 + Rand_ZeroFloat(1.25)) × 130 / 255.
        let s = h.skel.as_ref().unwrap().play_speed;
        assert!((2.75 * 130.0 / 255.0 - 1e-4..=4.0 * 130.0 / 255.0 + 1e-4).contains(&s), "{s}");
    }
    // Drawn: its bake, gFlagpoleSkel after Gfx_SetupDL_37Opa.
    assert!(drawn(&w).iter().any(|m| m == "bake/En_Hata/flagpole"), "{:?}", drawn(&w));
}

#[test]
fn no_wind_lifts_the_hoist_ends_to_15000_and_stops_the_flapping() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    // The speed clamped to 0 to 255 in the environment itself.
    w.env_ctx.wind_speed = 300.0;
    idle(&mut w, 1);
    assert_eq!(w.env_ctx.wind_speed, 255.0);
    w.env_ctx.wind_speed = -5.0;
    idle(&mut w, 1);
    assert_eq!(w.env_ctx.wind_speed, 0.0);
    // Speed 0: the target (15000 + 102) × 1 - 102 = 15000 whatever the gust; the play speed 0.
    idle(&mut w, 60);
    let h = hata(&w);
    assert_eq!(h.limbs[FLAGPOLE_LIMB_FLAG_1_HOIST_END_BASE][1], 15000);
    let sk = h.skel.as_ref().unwrap();
    assert_eq!(sk.play_speed, 0.0);
    let frame = sk.cur_frame;
    idle(&mut w, 5);
    assert_eq!(hata(&w).skel.as_ref().unwrap().cur_frame, frame, "the animation still");
}
