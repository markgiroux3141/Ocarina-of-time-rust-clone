//! `Bg_Spot00_Break` (GAME-11 milestone 2) against the C (`z_bg_spot00_break.c`): the adult's
//! Hyrule Field's broken drawbridge (params 0) and barbed wire fence (params 1). Each is a
//! DynaPoly actor with its own collision and display list; a child's is killed at init.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::bg_spot00_break::*;
use oot_game::play::{DrawOut, PlayState, ViewInfo, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn tick(w: &mut PlayState) {
    w.tick_with(scripted_input(PadState::default(), PadState::default()));
}

/// The field's spawn 7 (`ENTR_HYRULE_FIELD_7`, by the broken drawbridge), an adult from
/// `master-sword-adult` (the field's layer 2); the room's actors in.
fn field(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_HYRULE_FIELD_7").unwrap();
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("master-sword-adult").unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap();
    assert_eq!(w.save.scene_layer, 2);
    for _ in 0..30 {
        tick(&mut w);
    }
    w
}

fn pieces(w: &PlayState) -> Vec<&BgSpot00Break> {
    w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<BgSpot00Break>(h)).collect()
}

#[test]
fn the_adults_field_has_the_broken_drawbridge_and_the_fence() {
    let Some(a) = assets() else { return };
    let w = field(&a);
    // spot00 layers 2 and 3's room 0: params 0 at (0, -10, 670) (in front of the market's
    // entrance), and three fences (params 1) by the Lost Woods' bridge and the river, the first at
    // (-3713, -400, 11456), turned (0, 27307, 1274).
    let p = pieces(&w);
    assert_eq!(p.len(), 4);
    let bridge = p.iter().find(|b| !b.is_fence()).expect("the broken drawbridge");
    assert_eq!(p.iter().filter(|b| b.is_fence()).count(), 3);
    let fence = p.iter().find(|b| b.is_fence() && b.actor.home_pos == Vec3::new(-3713.0, -400.0, 11456.0)).expect("the fence");
    assert_eq!(bridge.actor.home_pos, Vec3::new(0.0, -10.0, 670.0));
    assert_eq!((fence.actor.shape_rot.y, fence.actor.shape_rot.z), (27307, 1274));
    // sInitChain: scale 1, the culling volume 1200, 1200, 2000.
    for b in &p {
        assert_eq!(b.actor.scale, Vec3::ONE);
        assert_eq!((b.actor.culling_volume_scale, b.actor.culling_volume_downward, b.actor.culling_volume_distance), (1200.0, 1200.0, 2000.0));
        assert!(w.col.dyna.is_bg_actor(b.bg));
    }
    // The broken drawbridge's collision is there to stand on: a line down through it hits its
    // polys (gBrokenDrawbridgeCol).
    let hit = w.col.entity_line_test(Vec3::new(0.0, 300.0, 670.0), Vec3::new(0.0, -300.0, 670.0), true, true, true, true);
    assert!(hit.is_some_and(|(_, poly)| poly.bg == bridge.bg), "{hit:?}");
    // BgSpot00Break_Draw: Gfx_DrawDListOpa of each piece's list.
    let view = ViewInfo { eye: Vec3::ZERO, billboard: glam::Mat4::IDENTITY };
    use oot_game::actor_ctx::ActorImpl;
    for (b, dl) in [(*bridge, "gBrokenDrawbridgeDL"), (*fence, "gBarbedWireFenceDL")] {
        let mut out = DrawOut::default();
        b.draw(&b.render_state(), &w, &view, &mut out);
        let names: Vec<String> = out.opa.iter().map(|c| c.mesh.name.clone()).collect();
        assert_eq!(names, vec![oot_game::pack::keys::mesh(OBJECT, dl)]);
    }
}

#[test]
fn a_childs_is_killed_at_init_and_its_collision_goes() {
    let Some(a) = assets() else { return };
    let mut w = field(&a);
    // !LINK_IS_ADULT in BgSpot00Break_Init: Actor_Kill after DynaPoly_SetBgActor; its Destroy
    // deletes the bg actor.
    w.save.adult = false;
    let h = w.actor_spawn(ACTOR_BG_SPOT00_BREAK, Vec3::new(0.0, 0.0, 400.0), [0; 3], 0).expect("spawned");
    let bg = w.actors.downcast::<BgSpot00Break>(h).map(|b| b.bg);
    tick(&mut w);
    assert!(w.actors.downcast::<BgSpot00Break>(h).is_none(), "killed");
    if let Some(bg) = bg {
        assert!(!w.col.dyna.is_bg_actor(bg));
    }
    // The field's child layer doesn't place it.
    let e = a.scenes.entrance_index("ENTR_HYRULE_FIELD_7").unwrap();
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("ocarina-of-time").unwrap();
    let w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap();
    // (Layer 1 after Zelda's escape.)
    assert!(w.save.scene_layer < 2);
    let room0 = &w.scene.as_ref().unwrap().rooms[0];
    assert!(room0.actors.iter().all(|e| e.id != ACTOR_BG_SPOT00_BREAK));
}
