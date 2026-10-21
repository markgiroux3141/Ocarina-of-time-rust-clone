//! GAME-10 milestone 3 against the C: `Bg_Spot08_Bakudankabe` (`z_bg_spot08_bakudankabe.c`),
//! Zora's Fountain's bombable wall (spot08 room 0: 0x0003 at (316, 60, 2634), facing +z), in
//! front of exit 5.
//!
//! Entered by `ENTR_ZORAS_FOUNTAIN_2` on the `fairy-magic` save, its intro seen
//! (`EVENTCHKINF_B7`). Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_spot08_bakudankabe::*;
use oot_actors::en_bom::{ACTOR_EN_BOM, EnBom};
use oot_game::actor_ctx::ActorImpl;
use oot_game::audio::sfx::{NA_SE_EV_WALL_BROKEN, NA_SE_SY_CORRECT_CHIME};
use oot_game::effect::EFFECT_SS_KAKERA;
use oot_game::env::clock_time;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// `EVENTCHKINF_B7` (`save.h`: 0xB7): the fountain's intro seen.
const EVENTCHKINF_B7: u16 = 0xB7;
const WALL: Vec3 = Vec3::new(316.0, 60.0, 2634.0);

fn fountain(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_ZORAS_FOUNTAIN_2").expect("entrance");
    let mut save = SaveContext::new(e, false, clock_time(10, 0) as u16);
    save.apply_preset("fairy-magic").unwrap();
    save.set_event_chk_inf(EVENTCHKINF_B7);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    w.audio.log = Some(Default::default());
    idle(&mut w, 3);
    w
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn walls(w: &PlayState) -> Vec<oot_game::actor_ctx::ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<BgSpot08Bakudankabe>(h).is_some_and(|b| !b.actor.killed)).collect()
}

fn sfx_frames(w: &PlayState, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, _)| f).collect()
}

fn effects_of(w: &PlayState, ty: u8) -> usize {
    w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == ty).count()
}

/// Link `d` in front of the wall (it faces +z), facing it, on the floor there.
fn before_wall(w: &mut PlayState, d: f32) {
    let z = WALL.z + d;
    let (y, _) = w.col.entity_raycast_down(Vec3::new(WALL.x, WALL.y + 100.0, z));
    w.place_player(Vec3::new(WALL.x, y, z), i16::MIN);
    idle(w, 2);
}

#[test]
fn the_fountain_s_wall_stands() {
    let Some(a) = assets() else { return };
    let mut w = fountain(&a);
    before_wall(&mut w, 300.0);
    let v = walls(&w);
    assert_eq!(v.len(), 1);
    let b = w.actors.downcast::<BgSpot08Bakudankabe>(v[0]).unwrap();
    assert_eq!((b.actor.params, b.actor.world_pos, b.switch_flag()), (3, WALL, 3));
    // sInitChain: scale 1 (ICHAIN_VEC3F), the culling volume (3200, 800, 1000).
    assert_eq!((b.actor.scale.x, b.actor.culling_volume_distance, b.actor.culling_volume_scale, b.actor.culling_volume_downward), (1.0, 3200.0, 800.0, 1000.0));
    let bg = b.dyna_bg_id().expect("its collision");
    assert!(w.col.dyna.is_bg_actor(bg));
    assert_eq!(w.col.dyna.actors[bg as usize].source.scale, Vec3::ONE);
    // Collider_UpdateSpheres at its model matrix (yaw 0, scale 1): the three centres 50 up and 50
    // out, -100, 0 and 100 across, 70 big (scale 100).
    let want = [Vec3::new(316.0, 110.0, 2684.0), Vec3::new(216.0, 110.0, 2684.0), Vec3::new(416.0, 110.0, 2684.0)];
    for (e, c) in b.collider.elements.iter().zip(want) {
        let s = e.dim.world_sphere;
        assert_eq!((Vec3::new(s.center[0] as f32, s.center[1] as f32, s.center[2] as f32), s.radius), (c, 70));
    }
    // Drawn whole.
    let f = w.current_frame();
    let mut out = oot_game::play::DrawOut::default();
    w.draw(&f, &oot_game::play::ViewInfo::new(f.view.eye, glam::Mat4::IDENTITY), &mut out);
    assert!(out.opa.iter().any(|c| c.mesh.name == "mesh/object_spot08_obj/gZorasFountainBombableWallDL"));
}

#[test]
fn with_its_flag_set_it_is_gone() {
    let Some(a) = assets() else { return };
    let mut w = fountain(&a);
    // BgSpot08Bakudankabe_Init: Flags_GetSwitch(params & 0x3F): killed, no collision.
    w.flags.set_switch(3);
    let h = w.actor_spawn(ACTOR_BG_SPOT08_BAKUDANKABE, WALL, [0; 3], 3).unwrap();
    let b = w.actors.downcast::<BgSpot08Bakudankabe>(h).unwrap();
    assert!(b.actor.killed);
    assert_eq!(b.dyna_bg_id(), None);
}

#[test]
fn a_bomb_blows_it_up() {
    let Some(a) = assets() else { return };
    let mut w = fountain(&a);
    // The rock and the boulder placed in the same spot (En_Ishi 0xFF21 at (316, 50, 2634),
    // Obj_Bombiwa 0x0016 at (317, 1, 2631)) would break too and add their own pieces: gone here.
    for h in w.actors.all() {
        if let Some(x) = w.actors.actor_mut(h)
            && (x.id == oot_actors::en_ishi::ACTOR_EN_ISHI || x.id == oot_actors::obj_bombiwa::ACTOR_OBJ_BOMBIWA)
        {
            x.kill();
        }
    }
    // Link 300 in front: within 800 across, its spheres set for hits.
    before_wall(&mut w, 300.0);
    let h = walls(&w)[0];
    assert!(w.actors.actor(h).unwrap().xz_dist_to_player < 800.0);
    let bg = w.actors.get(h).unwrap().dyna_bg_id().unwrap();
    let (y, _) = w.col.entity_raycast_down(Vec3::new(WALL.x, WALL.y + 100.0, WALL.z + 40.0));
    let bomb = w.actor_spawn(ACTOR_EN_BOM, Vec3::new(WALL.x, y, WALL.z + 40.0), [0; 3], 0).unwrap();
    w.actors.downcast_mut::<EnBom>(bomb).unwrap().timer = 2;
    let kakera_before = effects_of(&w, EFFECT_SS_KAKERA);
    let mut gone = None;
    for f in 0..12 {
        idle(&mut w, 1);
        if walls(&w).is_empty() {
            gone = Some(f);
            break;
        }
    }
    assert!(gone.is_some(), "blown up");
    // func_808B0324: 24 pieces (EffectSsKakera); the flag, the wall's sound and the chime; gone,
    // its collision with it.
    assert_eq!(effects_of(&w, EFFECT_SS_KAKERA) - kakera_before, 24);
    assert!(w.flags.get_switch(3));
    assert_eq!(sfx_frames(&w, NA_SE_EV_WALL_BROKEN).len(), 1);
    assert_eq!(sfx_frames(&w, NA_SE_SY_CORRECT_CHIME).len(), 1);
    idle(&mut w, 2);
    assert!(!w.col.dyna.is_bg_actor(bg));
}
