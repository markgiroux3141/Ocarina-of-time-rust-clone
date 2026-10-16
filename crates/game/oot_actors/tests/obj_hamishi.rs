//! GAME-09 milestone 2 against the C: the bronze boulders (`z_obj_hamishi.c`) in Goron City's room
//! 0 (`ootx scene-info --scene spot18`): params 0x2A at (-1541, 570, -631), 0x2B at (-1536, 529,
//! -861), 0x2C at (-1536, 520, -1102), 0x2F at (-1534, 549, -752) and 0x30 at (-1536, 520, -991),
//! all placed with no yaw.
//!
//! A child has no hammer (an adult's, Phase 15): the tests inject its hit on the cylinder (its
//! `AC_HIT` and `acHitElem`, as the collision check leaves them). Expected values are worked out
//! from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_bom::{ACTOR_EN_BOM, EnBom};
use oot_actors::obj_hamishi::{self as hamishi, ObjHamishi};
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::sfx::NA_SE_EV_WALL_BROKEN;
use oot_game::collision_check as cc;
use oot_game::effect::kakera as k;
use oot_game::effect::{EFFECT_SS_DUST, EFFECT_SS_KAKERA};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// The boulders as placed: (home, switch flag).
const BOULDERS: [(Vec3, i32); 5] = [
    (Vec3::new(-1541.0, 570.0, -631.0), 0x2A),
    (Vec3::new(-1536.0, 529.0, -861.0), 0x2B),
    (Vec3::new(-1536.0, 520.0, -1102.0), 0x2C),
    (Vec3::new(-1534.0, 549.0, -752.0), 0x2F),
    (Vec3::new(-1536.0, 520.0, -991.0), 0x30),
];

/// A child (`cavern-bombs`) at Goron City's bottom, then room 0 loaded (a debug start's room
/// change), the sound log on, Link at (-1400, 520, -1100).
fn room_0(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_GORON_CITY_2").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("cavern-bombs").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 4);
    assert!(w.room_request(0));
    idle(&mut w, 1);
    w.room_change_done();
    w.place_player(Vec3::new(-1400.0, 520.0, -1100.0), -0x4000);
    idle(&mut w, 5);
    w
}

fn sfx_on(w: &PlayState, frame: u32, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == frame && s == id)
}

fn boulders(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<ObjHamishi>(h).is_some_and(|b| !b.actor.killed)).collect()
}

fn boulder_at(w: &PlayState, home: Vec3) -> ActorHandle {
    boulders(w).into_iter().find(|&h| w.actors.actor(h).unwrap().home_pos == home).expect("the boulder")
}

fn b(w: &PlayState, h: ActorHandle) -> &ObjHamishi {
    w.actors.downcast::<ObjHamishi>(h).unwrap()
}

/// A hit with `dmg_flags` as the collision check leaves it on the cylinder.
fn hit(w: &mut PlayState, h: ActorHandle, dmg_flags: u32) {
    let player = w.player.unwrap();
    let r = w.actors.downcast_mut::<ObjHamishi>(h).unwrap();
    r.collider.base.ac_flags |= cc::AC_HIT;
    r.collider.info.ac_hit_elem = Some(cc::HitElem {
        elem: cc::ElemRef { col: cc::ColliderRef { actor: player, id: 0 }, elem: 0 },
        at_dmg_info: cc::ColliderElementDamageInfoAT { dmg_flags, hit_special_effect: cc::HIT_SPECIAL_EFFECT_NONE, damage: 2 },
        ac_dmg_info: Default::default(),
        elem_material: 0,
    });
}

/// The effects of type `ty` spawned this frame (`EffectSs_UpdateAll` has taken one off `life`).
fn new_effects(w: &PlayState, ty: u8, life: i16) -> Vec<&oot_game::effect::EffectSs> {
    w.effect_ss.table.iter().filter(|e| e.ty == ty && e.life == life - 1).collect()
}

#[test]
fn room_0s_five_boulders_are_placed_whole() {
    let Some(a) = assets() else { return };
    let w = room_0(&a);
    assert_eq!(boulders(&w).len(), 5);
    for (home, flag) in BOULDERS {
        let h = boulder_at(&w, home);
        let o = b(&w, h);
        // ObjHamishi_Init: scale 0.4 (sInitChain), a random yaw (placed 0) on home, world and
        // shape alike, the cylinder 50 by 70 at the placement, shape.yOffset 80; the flag clear.
        assert_eq!(hamishi::SWITCH_FLAG.get(o.actor.params) as i32, flag);
        assert_eq!(o.actor.scale, Vec3::splat(0.4));
        assert_eq!(o.actor.shape_y_offset, 80.0);
        assert_eq!((o.actor.home_rot.y, o.actor.world_rot.y), (o.actor.shape_rot.y, o.actor.shape_rot.y));
        assert_eq!(o.collider.dim.pos, [home.x as i16, home.y as i16, home.z as i16]);
        assert_eq!((o.collider.dim.radius, o.collider.dim.height), (50, 70));
        assert_eq!((o.actor.col_chk_info.cyl_radius, o.actor.col_chk_info.cyl_height, o.actor.col_chk_info.mass), (12, 60, cc::MASS_IMMOVABLE));
    }
    // Five Rand_ZeroFloat(65536) draws: not all the same.
    let yaws: std::collections::BTreeSet<i16> = boulders(&w).iter().map(|&h| b(&w, h).actor.shape_rot.y).collect();
    assert!(yaws.len() > 1);
}

#[test]
fn a_boulder_is_solid_to_a_child() {
    let Some(a) = assets() else { return };
    let mut w = room_0(&a);
    let (home, _) = BOULDERS[2];
    // Link within 600: the cylinder pushes (OC, OC2_TYPE_2 against Player's OC2_TYPE_PLAYER);
    // placed 30 into it, he's pushed out to its edge.
    w.place_player(home + Vec3::new(30.0, 0.0, 0.0), -0x4000);
    idle(&mut w, 10);
    let p = w.player().actor.world_pos;
    let d = Vec3::new(p.x - home.x, 0.0, p.z - home.z).length();
    assert!(d >= 50.0, "Link {d} from it");
}

#[test]
fn a_sword_or_a_bomb_doesnt_break_it() {
    let Some(a) = assets() else { return };
    let mut w = room_0(&a);
    let (home, flag) = BOULDERS[2];
    let h = boulder_at(&w, home);
    // A sword's slash (DMG_SLASH_KOKIRI, in 0x4FC1FFF6 but not DMG_HAMMER): AC_HIT cleared,
    // nothing more.
    hit(&mut w, h, cc::DMG_SLASH_KOKIRI);
    idle(&mut w, 1);
    assert_eq!((b(&w, h).hit_count, b(&w, h).shake_frames), (0, 0));
    assert_eq!(b(&w, h).collider.base.ac_flags & cc::AC_HIT, 0);
    // A bomb blowing against it: DMG_EXPLOSIVE (0x8) isn't in its 0x4FC1FFF6, no hit at all.
    let bomb = w.actor_spawn(ACTOR_EN_BOM, home + Vec3::new(55.0, 0.0, 0.0), [0; 3], 0).expect("En_Bom");
    w.actors.downcast_mut::<EnBom>(bomb).unwrap().timer = 2;
    for _ in 0..14 {
        idle(&mut w, 1);
        assert_eq!(b(&w, h).hit_count, 0);
    }
    assert!(!b(&w, h).actor.killed);
    assert!(!w.flags.get_switch(flag));
}

#[test]
fn the_hammer_shakes_it_then_breaks_it() {
    let Some(a) = assets() else { return };
    let mut w = room_0(&a);
    let (home, flag) = BOULDERS[2];
    let h = boulder_at(&w, home);
    // The first hammer hit (DMG_HAMMER_SWING): hitCount 1, 15 frames of shake from 2 across and
    // 400 of tilt.
    hit(&mut w, h, cc::DMG_HAMMER_SWING);
    idle(&mut w, 1);
    let o = b(&w, h);
    assert_eq!((o.hit_count, o.shake_frames, o.shake_pos_size, o.shake_rot_size), (1, 15, 2.0, 400.0));
    assert!(!o.actor.killed);
    // ObjHamishi_Shake the next frame: 14 left, the phases on by 5000 and 0xE10, the sizes eased
    // by 0.15 and 40, its place and tilt off home by them.
    idle(&mut w, 1);
    let o = b(&w, h);
    assert_eq!((o.shake_frames, o.shake_pos_phase, o.shake_rot_phase), (14, 5000, 0xE10));
    assert!((o.shake_pos_size - 1.85).abs() < 1e-6 && o.shake_rot_size == 360.0);
    let x = home.x + eng_math::sin_s(5000i16.wrapping_mul(4)) * o.shake_pos_size;
    assert_eq!(o.actor.world_pos.x, x);
    assert_eq!(o.actor.shape_rot.x, o.actor.home_rot.x.wrapping_add((eng_math::sin_s(0xE10i16.wrapping_mul(4)) * 360.0) as i16));
    // The shake over, it steps home (1 a frame) and level.
    idle(&mut w, 20);
    let o = b(&w, h);
    assert_eq!((o.actor.world_pos.x, o.actor.world_pos.z), (home.x, home.z));
    assert_eq!((o.actor.shape_rot.x, o.actor.shape_rot.z), (o.actor.home_rot.x, o.actor.home_rot.z));
    // The second: ObjHamishi_Break, NA_SE_EV_WALL_BROKEN, its flag, gone.
    hit(&mut w, h, cc::DMG_HAMMER_SWING);
    idle(&mut w, 1);
    let f = w.audio.frames;
    assert!(w.actors.actor(h).is_none_or(|a| a.killed));
    assert!(w.flags.get_switch(flag));
    assert!(sfx_on(&w, f, NA_SE_EV_WALL_BROKEN));
    // Nine bronze (KAKERA_COLOR_BROWN) pieces of gSilverRockFragmentsDL (70 frames), sEffectScales,
    // the first heavy (-450), three medium (-380), the rest light (-320).
    let pieces = new_effects(&w, EFFECT_SS_KAKERA, 70);
    assert_eq!(pieces.len(), 9);
    assert!(pieces.iter().all(|e| e.gfx == Some(("gameplay_field_keep", "gSilverRockFragmentsDL")) && e.regs[k::R_COLOR_IDX] == k::KAKERA_COLOR_BROWN));
    let mut scales: Vec<i16> = pieces.iter().map(|e| e.regs[k::R_SCALE]).collect();
    scales.sort();
    let mut want = hamishi::EFFECT_SCALES.to_vec();
    want.sort();
    assert_eq!(scales, want);
    let gravity = |g: i16| pieces.iter().filter(|e| e.regs[k::R_GRAVITY] == g).count();
    assert_eq!((gravity(-450), gravity(-380), gravity(-320)), (1, 3, 5));
    // func_80033480 twice: 7 and 13 puffs (10 frames).
    assert_eq!(new_effects(&w, EFFECT_SS_DUST, 10).len(), 20);
}

#[test]
fn a_broken_boulder_stays_gone() {
    let Some(a) = assets() else { return };
    let mut w = room_0(&a);
    let (home, flag) = BOULDERS[0];
    // ObjHamishi_Init: its flag set (0x2A, a temporary one, as the room keeps them): killed.
    w.flags.set_switch(flag);
    let h = w.actor_spawn(hamishi::ACTOR_OBJ_HAMISHI, home + Vec3::X * 10.0, [0; 3], flag as i16).expect("Obj_Hamishi");
    assert!(w.actors.actor(h).is_none_or(|a| a.killed));
}

#[test]
fn the_draw_is_the_tinted_silver_rock() {
    let Some(a) = assets() else { return };
    // ObjHamishi_Draw: gSilverRockDL after prim (255, 170, 130, 255): the bake.
    let bake: eng_gfx::DrawList = a.pack.assets.get(&oot_game::pack::keys::bake("Obj_Hamishi/gSilverRockDL")).expect("the bake");
    assert!(bake.triangle_count() > 0);
}
