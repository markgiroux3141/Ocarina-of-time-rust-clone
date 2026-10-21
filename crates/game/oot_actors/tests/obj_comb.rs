//! GAME-10 milestone 2 against the C: the beehives (`z_obj_comb.c`).
//!
//! The placements (`ootx scene-info --scene spot07_scene`), the child layers, room 0: three
//! `Obj_Comb` 0xFF02 (a red rupee, `ITEM00_RUPEE_RED`, one time in two) at (382, 1173, -1337),
//! (948, 1217, -1500) and (701, 1250, -2056). Child Link enters at `ENTR_ZORAS_DOMAIN_1` (room 0)
//! with `fairy-magic`, the Domain's intro seen. Each test runs the hive's update alone
//! (`Actor_UpdateAll`'s call) or its draw's sphere update, its hits set as the collision check
//! leaves them; the drop's `Rand` is replayed on a copy of the generator. Expected values are
//! worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use eng_math::sin_s;
use glam::Vec3;
use oot_actors::en_item00::EnItem00;
use oot_actors::obj_comb::*;
use oot_game::actor_ctx::ActorHandle;
use oot_game::collision_check as cc;
use oot_game::effect::EFFECT_SS_KAKERA;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

const EVENTCHKINF_A4: u16 = 0xA4;

fn domain_top(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_ZORAS_DOMAIN_1").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("fairy-magic").unwrap();
    save.set_event_chk_inf(EVENTCHKINF_A4);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true)).expect("Play_Init");
    for _ in 0..2 {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
    w
}

fn hives(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<ObjComb>(h).is_some_and(|c| !c.actor.killed)).collect()
}

fn comb(w: &PlayState, h: ActorHandle) -> &ObjComb {
    w.actors.downcast::<ObjComb>(h).expect("a hive")
}

/// The update alone, as the updating actor.
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

fn draw_update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.draw_update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// A hit as the collision check leaves it: `AC_HIT`, the attack's damage flags.
fn hit(w: &mut PlayState, h: ActorHandle, dmg_flags: u32) {
    let player = w.player.unwrap();
    let c = w.actors.downcast_mut::<ObjComb>(h).unwrap();
    c.collider.base.ac_flags |= cc::AC_HIT;
    c.collider.elements[0].info.ac_hit_elem = Some(cc::HitElem {
        elem: cc::ElemRef { col: cc::ColliderRef { actor: player, id: 0 }, elem: 0 },
        at_dmg_info: cc::ColliderElementDamageInfoAT { dmg_flags, hit_special_effect: cc::HIT_SPECIAL_EFFECT_NONE, damage: 1 },
        ac_dmg_info: Default::default(),
        elem_material: 0,
    });
}

fn items(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnItem00>(h).is_some_and(|i| !i.actor.killed)).collect()
}

#[test]
fn three_hives_hang_in_the_domain_with_their_spheres() {
    let Some(a) = assets() else { return };
    let mut w = domain_top(&a);
    let hs = hives(&w);
    assert_eq!(hs.len(), 3);
    for &h in &hs {
        // sInitChain: scale 0.1; culling volume 1100, 100, 900. Params 0xFF02: a red rupee.
        let c = comb(&w, h);
        assert_eq!((c.actor.params as u16, c.actor.scale), (0xFF02, Vec3::splat(0.1)));
        assert_eq!((c.actor.culling_volume_distance, c.actor.culling_volume_scale, c.actor.culling_volume_downward), (1100.0, 100.0, 900.0));
        assert_eq!(c.action, Action::Wait);
        // ObjComb_Draw's Collider_UpdateSpheres: the sphere round the matrix's origin (tipped
        // about 11.8 up), radius 15 (scale 100).
        draw_update(&mut w, h);
        let c = comb(&w, h);
        let r = c.actor.shape_rot;
        let centre = ObjComb::draw_matrix(c.actor.world_pos, [r.x, r.y, r.z], c.actor.scale).transform_point3(Vec3::ZERO);
        let s = c.collider.elements[0].dim.world_sphere;
        assert_eq!((s.center, s.radius), ([centre.x as i16, centre.y as i16, centre.z as i16], 15));
    }
}

#[test]
fn a_stick_shakes_it_and_the_swing_dies_down() {
    let Some(a) = assets() else { return };
    let mut w = domain_top(&a);
    let h = hives(&w)[0];
    let home_x = comb(&w, h).actor.home_rot.x;
    // A Deku Stick's hit (DMG_DEKU_STICK): ObjComb_Wait sets unk_1B0 to 1500 (after its fall of
    // 50); ObjComb_Update turns unk_1B2 by 0x2EE0 first, then tips it by sin(unk_1B2) * 1500.
    let phase = comb(&w, h).unk_1b2;
    hit(&mut w, h, cc::DMG_DEKU_STICK);
    update(&mut w, h);
    let c = comb(&w, h);
    assert!(!c.actor.killed);
    assert_eq!(c.unk_1b0, 1500);
    let phase = phase.wrapping_add(0x2EE0);
    assert_eq!(c.actor.shape_rot.x, (sin_s(phase) * 1500.0 + home_x as f32) as i32 as i16);
    // 30 frames to still: 1450, 1400, ... 0.
    for n in 1..=30 {
        update(&mut w, h);
        assert_eq!(comb(&w, h).unk_1b0, 1500 - 50 * n);
    }
    update(&mut w, h);
    assert_eq!((comb(&w, h).unk_1b0, comb(&w, h).actor.shape_rot.x), (0, home_x));
    // The hammer, an arrow and a seed shake it too.
    for f in [cc::DMG_HAMMER_SWING, cc::DMG_ARROW_NORMAL, cc::DMG_SLINGSHOT] {
        hit(&mut w, h, f);
        update(&mut w, h);
        assert_eq!((comb(&w, h).unk_1b0, comb(&w, h).actor.killed), (1500, false));
    }
}

/// `ObjComb_Break` then `ObjComb_ChooseItemDrop` on hive `h` (ObjComb_Wait's order on a breaking
/// hit), the drop's `Rand_ZeroOne` read off a copy of the generator first: whether it drew a drop
/// (0.5 or more), and the items it left.
fn break_and_drop(w: &mut PlayState, h: ActorHandle) -> (bool, Vec<ActorHandle>) {
    let before = items(w);
    let a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    let c = a.as_any().downcast_ref::<ObjComb>().unwrap();
    c.break_(w);
    let mut copy = w.rand;
    let drew = copy.zero_one() >= 0.5;
    c.choose_item_drop(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
    (drew, items(w).into_iter().filter(|i| !before.contains(i)).collect())
}

#[test]
fn the_sword_breaks_it_into_31_pieces() {
    let Some(a) = assets() else { return };
    let mut w = domain_top(&a);
    let h = hives(&w)[1];
    let pos = comb(&w, h).actor.world_pos;
    // A sword's hit (DMG_SLASH_KOKIRI): broken, the drop, gone.
    hit(&mut w, h, cc::DMG_SLASH_KOKIRI);
    update(&mut w, h);
    assert!(comb(&w, h).actor.killed);
    let pieces: Vec<_> = w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == EFFECT_SS_KAKERA && e.gfx.is_some_and(|g| g.1 == FRAGMENT_DL)).collect();
    assert_eq!(pieces.len(), 31);
    // Each from the hive's middle: within 10 across, (i - 15) * 0.7 up or down.
    assert!(pieces.iter().all(|e| (e.pos - pos).abs().max_element() <= 10.5 + 1e-3), "the pieces round the hive");
}

#[test]
fn its_drop_is_a_red_rupee_one_time_in_two_and_a_heart_piece_once() {
    let Some(a) = assets() else { return };
    let mut w = domain_top(&a);
    // ObjComb_ChooseItemDrop: params 2 (ITEM00_RUPEE_RED); not a piece of heart: dropped when
    // Rand_ZeroOne is 0.5 or more (replayed).
    let (mut drops, mut misses) = (0, 0);
    for h in hives(&w) {
        let (drew, new) = break_and_drop(&mut w, h);
        assert_eq!(new.len(), drew as usize);
        if let Some(&i) = new.first() {
            assert_eq!(w.actors.actor(i).unwrap().params & 0xFF, 0x02);
            drops += 1;
        } else {
            misses += 1;
        }
    }
    eprintln!("{drops} rupees, {misses} none");
    // A hive with a piece of heart (ITEM00_HEART_PIECE 6, collectible flag 5: params 0x0506):
    // dropped with its flag (whatever Rand gives) while the flag is clear; none once it's set.
    let p = Vec3::new(600.0, 1100.0, -1600.0);
    let h = w.actor_spawn(ACTOR_OBJ_COMB, p, [0; 3], 0x0506).expect("a hive");
    let before = items(&w);
    let a2 = w.actors.take(h).unwrap();
    a2.as_any().downcast_ref::<ObjComb>().unwrap().choose_item_drop(&mut w);
    w.actors.put_back(h, a2);
    let new: Vec<_> = items(&w).into_iter().filter(|i| !before.contains(i)).collect();
    assert_eq!(new.len(), 1);
    let heart = w.actors.downcast::<EnItem00>(new[0]).unwrap();
    assert_eq!((heart.actor.params, heart.collectible_flag), (0x06, 5));
    w.flags.set_collectible(5);
    let before = items(&w);
    let a2 = w.actors.take(h).unwrap();
    a2.as_any().downcast_ref::<ObjComb>().unwrap().choose_item_drop(&mut w);
    w.actors.put_back(h, a2);
    assert_eq!(items(&w).len(), before.len());
}

#[test]
fn a_spawned_hive_is_hit_by_link_s_attacks() {
    let Some(a) = assets() else { return };
    let w = domain_top(&a);
    let h = hives(&w)[2];
    // sJntSphInit: AC_ON | AC_TYPE_PLAYER, OC1_ON | OC1_TYPE_PLAYER; the element 0x4001FFFE.
    let c = comb(&w, h);
    assert_eq!(c.collider.base.ac_flags & (cc::AC_ON | cc::AC_TYPE_PLAYER), cc::AC_ON | cc::AC_TYPE_PLAYER);
    assert_eq!(c.collider.elements[0].info.ac_dmg_info.dmg_flags, 0x4001_FFFE);
}

