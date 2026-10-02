//! `En_Ko`, the Kokiri children, in Kokiri Forest (child Link, 10:00, a new save): which spawn,
//! what each plays, the fade by Link's distance, and child 3 guarding the way to the Lost Woods.
//! Expected values come from `z_en_ko.c`'s tables (`sOsAnimeLookup`, `sAnimationInfo`,
//! `sInteractInfo`, `sCylinderInit`) and its functions.

mod common;

use std::sync::Arc;

use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_ko::{self, EnKo};
use oot_game::actor::ACTOR_FLAG_ATTENTION_ENABLED;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn enter(entrance: &str) -> Option<PlayState> {
    let a = assets()?;
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    Some(oot_actors::play_entrance(a, common::data()?, common::rules()?, save).expect("Play_Init"))
}

fn frames(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn kokiri(w: &PlayState) -> Vec<&EnKo> {
    w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<EnKo>(h)).collect()
}

fn find(w: &PlayState, ty: u8) -> oot_game::actor_ctx::ActorHandle {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnKo>(h).is_some_and(|k| k.actor.params & 0xFF == ty as i16)).expect("the child")
}

#[test]
fn the_village_children_spawn_and_play_their_animations() {
    let Some(mut w) = enter("ENTR_KOKIRI_FOREST_3") else { return };
    frames(&mut w, 4);
    // Room 0 places children 0..6 and Fado (12); EnKo_CanSpawn keeps types below 7 and Fado in
    // Kokiri Forest for child Link.
    let ks = kokiri(&w);
    let mut types: Vec<i16> = ks.iter().map(|k| k.actor.params & 0xFF).collect();
    types.sort();
    assert_eq!(types, vec![0, 1, 2, 3, 4, 5, 6, 12]);
    // ENKO_FQS_CHILD_START: sOsAnimeLookup[type][0], through sAnimationInfo.
    let expected = [(0, "gKokiriLiftingRockAnim"), (1, "gKokiriStandUpAnim"), (2, "gKokiriPunchingAnim"), (3, "gKokiriBlockingAnim"), (4, "gKokiriCuttingGrassAnim"), (5, "gKokiriStandUpAnim"), (6, "gKokiriStandUpAnim"), (12, "gKokiriIdleAnim")];
    for k in &ks {
        let ty = (k.actor.params & 0xFF) as u8;
        let want = expected.iter().find(|e| e.0 == ty).unwrap().1;
        let skel = k.skel.as_ref().expect("past func_80A99048");
        assert!(skel.is(want), "child {ty} plays {want}");
        // SkelAnime_InitFlex(..., 16): gKm1Skel / gKw1Skel have 15 limbs.
        assert_eq!(skel.limb_count, 16);
        assert_eq!(skel.joint_table.len(), 16);
        // sInteractInfo: targetMode 6 (1 for child 5), lookDist 30 + the collider's 20,
        // appearDist 180 (240 for child 5).
        let (mode, appear) = if ty == en_ko::ENKO_TYPE_CHILD_5 { (1, 240.0) } else { (6, 180.0) };
        assert_eq!((k.actor.target_mode, k.look_dist, k.appear_dist), (mode, 50.0, appear));
        assert_eq!(k.actor.scale, Vec3::splat(0.01));
        // Every child has its fairy (En_Elf params 3, a placeholder) as its child actor.
        let fairy = k.actor.child.and_then(|h| w.actors.actor(h)).expect("the fairy");
        assert_eq!((fairy.id, fairy.params), (0x0018, 3));
    }
}

#[test]
fn a_child_fades_in_near_link_and_out_away_from_him() {
    let Some(mut w) = enter("ENTR_KOKIRI_FOREST_3") else { return };
    frames(&mut w, 4);
    // Child 1 at (45, 0, -272).
    let h = find(&w, en_ko::ENKO_TYPE_CHILD_1);
    let home = w.actors.actor(h).unwrap().home_pos;
    // Far (more than appearDist 180): Math_SmoothStepToF towards 0, and below 10 it can't be
    // targeted (ACTOR_FLAG_ATTENTION_ENABLED cleared).
    w.place_player(home + Vec3::new(0.0, 0.0, 400.0), i16::MIN);
    frames(&mut w, 20);
    let k = w.actors.downcast::<EnKo>(h).unwrap();
    assert_eq!(k.model_alpha, 0.0);
    assert_eq!(k.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    // Near: towards 255 by at most 40 a frame (fraction 0.3), targetable from alpha 10.
    w.place_player(home + Vec3::new(0.0, 0.0, 100.0), i16::MIN);
    frames(&mut w, 1);
    let a1 = w.actors.downcast::<EnKo>(h).unwrap().model_alpha;
    assert_eq!(a1, 40.0, "the first step is capped at 40");
    frames(&mut w, 20);
    let k = w.actors.downcast::<EnKo>(h).unwrap();
    assert_eq!(k.model_alpha, 255.0);
    assert_eq!(k.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_ATTENTION_ENABLED);
    // Facing Link within the preset's yaw range, it turns its head to him (Npc_TrackPoint
    // mode 2): Link is straight ahead (+z of a child facing +z), so the head's yaw is small.
    assert!((k.interact_info.head[1] as i32).abs() < 0x800, "head yaw {:#x}", k.interact_info.head[1]);
}

#[test]
fn child_3_guards_the_way_to_the_lost_woods() {
    let Some(mut w) = enter("ENTR_KOKIRI_FOREST_3") else { return };
    frames(&mut w, 4);
    // No Kokiri Emerald: func_80A995CC, with the body 200 taller (46 + 200).
    let h = find(&w, en_ko::ENKO_TYPE_CHILD_3);
    {
        let k = w.actors.downcast::<EnKo>(h).unwrap();
        assert_eq!(k.action, en_ko::Action::Guard);
        assert_eq!(k.collider.dim.height, 246);
    }
    let home = w.actors.actor(h).unwrap().home_pos;
    // Link due east of its home: the yaw from home to Link is 0x4000, so the child stands
    // 80 east of home (80 * sin, 80 * cos), facing Link.
    w.place_player(home + Vec3::new(200.0, 0.0, 0.0), -0x4000);
    frames(&mut w, 2);
    let a = w.actors.actor(h).unwrap();
    assert!((a.world_pos.x - (home.x + 80.0)).abs() < 1e-3 && (a.world_pos.z - home.z).abs() < 1e-3, "at {:?}, home {:?}", a.world_pos, home);
    assert_eq!(a.shape_rot.y, a.yaw_towards_player);
}
