//! GAME-09 milestone 2 against the C: `Bg_Bombwall` (`z_bg_bombwall.c`), Death Mountain Trail's
//! three bombable walls (spot16 room 0): 0x8001 at (-1825, 340, 1189) facing -x (yaw -0x4000)
//! over a Gold Skulltula's nook, 0x8002 at (-540, 1448, -81) over a chest's, 0x8003 at (-245,
//! 3305, -4480) over the Great Fairy's fountain.
//!
//! Entered by `ENTR_DEATH_MOUNTAIN_TRAIL_0` on a `cavern-bombs` save with the trail's intro seen
//! (`EVENTCHKINF_A1`). Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_bombwall::{Action, BROKEN_DL, BgBombwall, NORMAL_DL};
use oot_actors::en_bom::{ACTOR_EN_BOM, EnBom};
use oot_game::actor_ctx::ActorImpl;
use oot_game::audio::sfx::NA_SE_SY_CORRECT_CHIME;
use oot_game::effect::EFFECT_SS_DUST;
use oot_game::env::clock_time;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// `EVENTCHKINF_A1` (`save.h`): the trail's intro seen.
const EVENTCHKINF_A1: u16 = 0xA1;
/// `SCENE_DEATH_MOUNTAIN_TRAIL` (`scene_table.h`: 0x60, spot16).
const SCENE_DEATH_MOUNTAIN_TRAIL: usize = 0x60;
/// The Gold Skulltula's wall (params 0x8001: switch flag 1, the chime).
const WALL_1: Vec3 = Vec3::new(-1825.0, 340.0, 1189.0);

fn trail_with(a: &Arc<GameAssets>, prepare: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DEATH_MOUNTAIN_TRAIL_0").expect("entrance");
    let mut save = SaveContext::new(e, false, clock_time(10, 0) as u16);
    save.apply_preset("cavern-bombs").unwrap();
    save.set_event_chk_inf(EVENTCHKINF_A1);
    prepare(&mut save);
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

fn wall(w: &PlayState, params: u16) -> &BgBombwall {
    w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<BgBombwall>(h)).find(|b| b.actor.params as u16 == params).expect("the wall")
}

fn sfx_frames(w: &PlayState, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, _)| f).collect()
}

fn effects_of(w: &PlayState, ty: u8) -> usize {
    w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == ty).count()
}

/// The meshes the last frame draws.
fn drawn(w: &PlayState) -> Vec<String> {
    let f = w.current_frame();
    let mut out = oot_game::play::DrawOut::default();
    w.draw(&f, &oot_game::play::ViewInfo::new(f.view.eye, glam::Mat4::IDENTITY), &mut out);
    out.opa.iter().chain(out.xlu.iter()).map(|c| c.mesh.name.clone()).collect()
}

const NORMAL_MESH: &str = "mesh/gameplay_field_keep/gBgBombwallNormalDL";
const BROKEN_MESH: &str = "mesh/gameplay_field_keep/gBgBombwallBrokenDL";

/// Child Link `d` west of wall 1 (in front of it: it faces -x), facing it, on the floor there.
fn before_wall_1(w: &mut PlayState, d: f32) {
    let x = WALL_1.x - d;
    let (y, _) = w.col.entity_raycast_down(Vec3::new(x, WALL_1.y + 100.0, WALL_1.z));
    w.place_player(Vec3::new(x, y, WALL_1.z), 0x4000);
    idle(w, 2);
}

#[test]
fn the_trail_s_walls_stand_whole() {
    let Some(a) = assets() else { return };
    let w = trail_with(&a, |_| {});
    for params in [0x8001, 0x8002, 0x8003] {
        let b = wall(&w, params);
        // BgBombwall_Init: scale 0.1, the culling volume (1800, 300, 1000); its collision
        // (gBgBombwallCol) and its triangles set (unk_2A2 3); gBgBombwallNormalDL, waiting.
        assert_eq!((b.actor.scale.x, b.actor.culling_volume_distance, b.actor.culling_volume_scale, b.actor.culling_volume_downward), (0.1, 1800.0, 300.0, 1000.0));
        assert_eq!((b.dlist, b.action, b.unk_2a2), (NORMAL_DL, Action::Wait, 3));
        let bg = b.dyna_bg_id().expect("its collision");
        assert!(w.col.dyna.is_bg_actor(bg));
    }
    // Wall 1's triangles: sTrisElementsInit 2 further out (z + 2), turned by its yaw -0x4000
    // (BgBombwall_RotateVec, sin -1, cos 0: x' = z × sin, z' = -x × sin), at its position.
    // Triangle 0: (-70, 176, 2) to (-1827, 516, 1119); (-70, -4, 2) to (-1827, 336, 1119); (0, -4,
    // 32) to (-1857, 336, 1189).
    let b = wall(&w, 0x8001);
    let v = b.collider.elements[0].dim.vtx;
    let expect = [Vec3::new(-1827.0, 516.0, 1119.0), Vec3::new(-1827.0, 336.0, 1119.0), Vec3::new(-1857.0, 336.0, 1189.0)];
    for (got, want) in v.iter().zip(expect) {
        assert!(got.distance(want) < 0.01, "{got} {want}");
    }
}

#[test]
fn a_blast_breaks_a_wall_two_frames_on() {
    let Some(a) = assets() else { return };
    let mut w = trail_with(&a, |_| {});
    // Link more than 600 away (650 south of it, on the trail's floor, facing it): the triangles
    // aren't set for hits (func_8086ED70); a bomb's blast at the wall does nothing.
    let far = WALL_1 + Vec3::Z * 650.0;
    let (y, _) = w.col.entity_raycast_down(far + Vec3::Y * 100.0);
    w.place_player(Vec3::new(far.x, y, far.z), i16::MIN);
    idle(&mut w, 2);
    assert!(wall(&w, 0x8001).actor.xz_dist_to_player >= 600.0);
    // (In view: it updates, the 600 is what keeps it from being hit.)
    assert_ne!(wall(&w, 0x8001).actor.flags & oot_game::actor::ACTOR_FLAG_INSIDE_CULLING_VOLUME, 0);
    let h = w.actor_spawn(ACTOR_EN_BOM, WALL_1 - Vec3::X * 20.0, [0; 3], 0).unwrap();
    w.actors.downcast_mut::<EnBom>(h).unwrap().timer = 2;
    idle(&mut w, 14);
    assert_ne!(wall(&w, 0x8001).actor.flags & oot_game::actor::ACTOR_FLAG_INSIDE_CULLING_VOLUME, 0);
    assert!(!w.flags.get_switch(1), "no hit beyond 600");
    assert_eq!(wall(&w, 0x8001).action, Action::Wait);
    // Link 150 in front, the wall drawn whole.
    before_wall_1(&mut w, 150.0);
    assert!(drawn(&w).iter().any(|m| m == NORMAL_MESH));
    let bg = wall(&w, 0x8001).dyna_bg_id().unwrap();
    let h = w.actor_spawn(ACTOR_EN_BOM, WALL_1 - Vec3::X * 20.0, [0; 3], 0).unwrap();
    w.actors.downcast_mut::<EnBom>(h).unwrap().timer = 2;
    let dust_before = effects_of(&w, EFFECT_SS_DUST);
    let mut f0 = None;
    for _ in 0..12 {
        idle(&mut w, 1);
        if w.flags.get_switch(1) {
            f0 = Some(w.audio.frames);
            break;
        }
    }
    let f0 = f0.expect("the wall hit");
    // func_8086ED70's AC_HIT: func_8086EDFC (the dust, unk_2A0 1), switch flag 1. The dust:
    // seven func_80033480 of three puffs and one of five.
    let b = wall(&w, 0x8001);
    assert_eq!((b.action, b.unk_2a0, b.dlist), (Action::Break, 1, NORMAL_DL));
    assert!(effects_of(&w, EFFECT_SS_DUST) >= dust_before + 26, "{} {}", effects_of(&w, EFFECT_SS_DUST), dust_before);
    // A frame on: unk_2A0 0, still whole.
    idle(&mut w, 1);
    let b = wall(&w, 0x8001);
    assert_eq!((b.action, b.unk_2a0, b.dlist), (Action::Break, 0, NORMAL_DL));
    assert!(w.col.dyna.is_bg_actor(bg));
    // Two frames on: func_8086EE94 (gBgBombwallBrokenDL, the collision and triangles gone, no
    // action) and bit 15's chime.
    idle(&mut w, 1);
    let b = wall(&w, 0x8001);
    assert_eq!((b.action, b.dlist, b.unk_2a2, b.dyna_bg_id()), (Action::None, BROKEN_DL, 0, None));
    assert!(!w.col.dyna.is_bg_actor(bg), "its collision gone");
    assert_eq!(sfx_frames(&w, NA_SE_SY_CORRECT_CHIME), vec![f0 + 2]);
    assert!(drawn(&w).iter().any(|m| m == BROKEN_MESH), "{:?}", drawn(&w));
}

#[test]
fn a_wall_whose_flag_is_set_is_broken_from_the_start() {
    let Some(a) = assets() else { return };
    // Switch flag 2 saved: wall 0x8002 is broken at init (func_8086EE94): no collision, no
    // triangles, nothing to do, drawn broken.
    let w = trail_with(&a, |s| s.scene_flags[SCENE_DEATH_MOUNTAIN_TRAIL].swch |= 1 << 2);
    let b = wall(&w, 0x8002);
    assert_eq!((b.action, b.dlist, b.unk_2a2, b.dyna_bg_id()), (Action::None, BROKEN_DL, 0, None));
    // The others whole.
    assert_eq!(wall(&w, 0x8001).dlist, NORMAL_DL);
    assert_eq!(wall(&w, 0x8003).dlist, NORMAL_DL);
}
