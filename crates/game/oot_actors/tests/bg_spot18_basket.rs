//! GAME-09 milestone 2 against the C: Goron City's spinning urn (`z_bg_spot18_basket.c`) and its
//! lid (`z_bg_spot18_futa.c`).
//!
//! Goron City's bottom (`ENTR_GORON_CITY_2`: spawn 2, room 3, `ootx scene-info --scene spot18
//! --layer 0`) has the urn, params 0x1C1F at (3, -3, 20): switch flag 0x1C (the four timed torches
//! 0x111C round it), collectible flag 0x1F. A child with the `cavern-bombs` save (the bracelet,
//! the 20 bomb bag). Bombs are spawned as `tests/bombs.rs` spawns them (timer 2). Expected values
//! are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_spot18_basket::{self as basket, Action, BgSpot18Basket};
use oot_actors::bg_spot18_futa::BgSpot18Futa;
use oot_actors::en_bom::{ACTOR_EN_BOM, EnBom};
use oot_actors::en_item00::{EnItem00, ITEM00_BOMBS_A, ITEM00_HEART_PIECE, ITEM00_RUPEE_BLUE, ITEM00_RUPEE_GREEN, ITEM00_RUPEE_PURPLE, ITEM00_RUPEE_RED};
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::audio::sfx::{NA_SE_SY_CORRECT_CHIME, NA_SE_SY_TRE_BOX_APPEAR};
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

/// The urn's switch flag and collectible flag (params 0x1C1F).
const SWITCH: i32 = 0x1C;
const COLLECTIBLE: i32 = 0x1F;
/// Where it's placed.
const HOME: Vec3 = Vec3::new(3.0, -3.0, 20.0);

/// A child at Goron City's bottom (`ENTR_GORON_CITY_2`) at 10:00 with `cavern-bombs`, `edit`
/// applied last, the sound log on; four frames on, Link 150 from the urn.
fn city(a: &Arc<GameAssets>, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_GORON_CITY_2").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("cavern-bombs").unwrap();
    edit(&mut save);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 4);
    w.place_player(Vec3::new(-147.0, -3.0, 20.0), 0x4000);
    idle(&mut w, 1);
    w
}

fn sfx_on(w: &PlayState, frame: u32, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == frame && s == id)
}

fn all<T: 'static + ActorImpl>(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<T>(h).is_some_and(|a| !a.base().killed)).collect()
}

fn the_urn(w: &PlayState) -> ActorHandle {
    let v = all::<BgSpot18Basket>(w);
    assert_eq!(v.len(), 1, "one urn");
    v[0]
}

fn urn(w: &PlayState, h: ActorHandle) -> &BgSpot18Basket {
    w.actors.downcast::<BgSpot18Basket>(h).unwrap()
}

/// The sub camera running one-point cutscene `cs_id`, if any.
fn onepoint(w: &PlayState, cs_id: i16) -> Option<i16> {
    (1..4).find(|&i| w.camera(i).is_some_and(|c| c.cs_id == cs_id && c.timer > 0)).map(|i| w.camera(i).unwrap().timer)
}

/// The flag set, until the urn turns (`BgSpot18Basket_Activation`'s 21 frames), then 30 more.
fn turning(w: &mut PlayState, h: ActorHandle) {
    w.flags.set_switch(SWITCH);
    for _ in 0..40 {
        idle(w, 1);
        if urn(w, h).action == Action::Spinning {
            break;
        }
    }
    assert_eq!(urn(w, h).action, Action::Spinning);
    idle(w, 30);
}

/// A bomb with 2 on its fuse `up` above the urn (where it is now); its handle.
fn bomb_above(w: &mut PlayState, h: ActorHandle, up: f32) -> ActorHandle {
    let p = urn(w, h).actor.world_pos;
    let b = w.actor_spawn(ACTOR_EN_BOM, p + Vec3::Y * up, [0; 3], 0).expect("En_Bom");
    w.actors.downcast_mut::<EnBom>(b).unwrap().timer = 2;
    b
}

/// The wild spin started by a bomb 160 above.
fn exploded(w: &mut PlayState, h: ActorHandle) {
    bomb_above(w, h, 160.0);
    for _ in 0..8 {
        idle(w, 1);
        if urn(w, h).action == Action::ExplosionCs {
            return;
        }
    }
    panic!("the bomb didn't start the urn: {:?}", urn(w, h).action);
}

/// Through the wild spin to its 140th frame, then the angle set so that the next frame's turn
/// (`spinRate` stepped to 0x3E8 by 0x32 first) lands on `rot_y`: `BgSpot18Basket_SetupStopping`
/// reads it.
fn stop_at(w: &mut PlayState, h: ActorHandle, rot_y: i16) {
    while urn(w, h).timer < 140 {
        idle(w, 1);
        assert_eq!(urn(w, h).action, Action::ExplosionCs);
    }
    let u = w.actors.downcast_mut::<BgSpot18Basket>(h).unwrap();
    let mut spin = u.spin_rate;
    eng_math::step_to_s(&mut spin, 0x3E8, 0x32);
    u.actor.shape_rot.y = rot_y.wrapping_sub(spin);
    idle(w, 1);
    assert_eq!(urn(w, h).action, Action::Stopping);
    assert_eq!(urn(w, h).actor.shape_rot.y, rot_y);
}

/// Until it gives its prize (`BgSpot18Basket_GivingPrize`'s frame 1): the drops spawned that
/// frame (their handles, new ones only).
fn prize(w: &mut PlayState, h: ActorHandle) -> Vec<ActorHandle> {
    let before = all::<EnItem00>(w);
    for _ in 0..300 {
        idle(w, 1);
        if urn(w, h).action == Action::GivingPrize && urn(w, h).timer == 1 {
            return all::<EnItem00>(w).into_iter().filter(|d| !before.contains(d)).collect();
        }
    }
    panic!("no prize");
}

/// The drops' (item, collectible flag, yaw), sorted by yaw.
fn drops(w: &PlayState, v: &[ActorHandle]) -> Vec<(i16, i16, i16)> {
    let mut d: Vec<(i16, i16, i16)> = v
        .iter()
        .map(|&h| {
            let e = w.actors.downcast::<EnItem00>(h).unwrap();
            (e.actor.params, e.collectible_flag, e.actor.world_rot.y)
        })
        .collect();
    d.sort_by_key(|x| x.2);
    d
}

#[test]
fn the_urn_waits_under_its_lid() {
    let Some(a) = assets() else { return };
    let w = city(&a, |_| {});
    let h = the_urn(&w);
    let u = urn(&w, h);
    // BgSpot18Basket_Init: scale 0.1 (sInitChain); home.pos.y + 0.01; the flag clear: waiting,
    // the lid spawned as its child (Actor_SpawnAsChild) turned 0x1555.
    assert_eq!(u.action, Action::Inactive);
    assert_eq!(u.actor.home_pos, Vec3::new(HOME.x, HOME.y + 0.01, HOME.z));
    assert_eq!(u.actor.world_pos, u.actor.home_pos);
    assert_eq!(u.actor.scale, Vec3::splat(0.1));
    // DYNA_TRANSFORM_POS | DYNA_TRANSFORM_ROT_Y.
    assert_eq!(w.col.dyna.actors[u.bg as usize].move_flags, 3);
    let lid = u.actor.child.expect("the lid");
    let l = w.actors.downcast::<BgSpot18Futa>(lid).expect("Bg_Spot18_Futa");
    assert_eq!(l.actor.parent, Some(h));
    assert_eq!((l.actor.shape_rot.y, l.actor.scale), (0x1555, Vec3::splat(0.1)));
    assert_eq!(l.actor.world_pos, u.actor.world_pos);
    // DynaPolyActor_Init(0): the lid carries nothing.
    assert_eq!(w.col.dyna.actors[l.bg as usize].move_flags, 0);
    // The lid's on the urn: a raycast down onto the urn's middle lands on the lid's collision.
    let (y, poly) = w.col.entity_raycast_down(HOME + Vec3::Y * 600.0);
    assert_eq!(poly.map(|p| p.bg), Some(l.bg), "the lid's top at {y}");
    // Waiting: no spheres registered (no OC push, no hits).
    assert_eq!(u.collider_jnt_sph.base.ac_flags & oot_game::collision_check::AC_HIT, 0);
}

#[test]
fn the_torches_flag_starts_it_turning() {
    let Some(a) = assets() else { return };
    let mut w = city(&a, |_| {});
    let h = the_urn(&w);
    let lid = urn(&w, h).actor.child.unwrap();
    // BgSpot18Basket_Inactive: the flag set, OnePointCutscene_Init(4220, 80), Activation, timer 0.
    w.flags.set_switch(SWITCH);
    idle(&mut w, 1);
    assert_eq!((urn(&w, h).action, urn(&w, h).timer), (Action::Activation, 0));
    assert!(onepoint(&w, 4220).is_some(), "one-point 4220");
    // BgSpot18Basket_Activation: once timer > 20 (the 21st frame on), Spinning (spin and circle
    // rates 0), the lid let go (its parent and the urn's child NULL).
    idle(&mut w, 20);
    assert_eq!((urn(&w, h).action, urn(&w, h).timer), (Action::Activation, 20));
    assert_eq!(w.actors.actor(lid).unwrap().parent, Some(h));
    idle(&mut w, 1);
    let u = urn(&w, h);
    assert_eq!((u.action, u.spin_rate, u.circle_rate), (Action::Spinning, 0, 0));
    assert_eq!((u.actor.child, w.actors.actor(lid).unwrap().parent), (None, None));
    // BgSpot18Futa_Update: parentless, its scale steps to 0 by 0.005 (the lid updates before the
    // urn, newer in the props' list: from the next frame), then it's gone.
    idle(&mut w, 1);
    let s = w.actors.actor(lid).unwrap().scale;
    assert!((s.x - 0.095).abs() < 1e-6 && s.y == s.x && s.z == s.x, "{s}");
    idle(&mut w, 20);
    assert!(w.actors.actor(lid).is_none_or(|l| l.killed), "the lid's gone");
    // BgSpot18Basket_Spinning: the spin up by 0x1E to 0x1F4, the circle rate by 15 to 400, the
    // radius by 1.5 to 50, round its home; NA_SE_EV_ELEVATOR_MOVE (flagged) each frame.
    let u = urn(&w, h);
    assert_eq!(u.spin_rate, (21 * 0x1E).min(0x1F4) as i16);
    assert_eq!(u.circle_rate, 21 * 15);
    assert!((u.circle_radius - 31.5).abs() < 1e-4, "{}", u.circle_radius);
    let off = Vec3::new(u.actor.world_pos.x - u.actor.home_pos.x, 0.0, u.actor.world_pos.z - u.actor.home_pos.z);
    assert!((off.length() - u.circle_radius).abs() < 0.01);
    assert_eq!(u.actor.sfx, basket::NA_SE_EV_ELEVATOR_MOVE - oot_game::audio::sfx::SFX_FLAG);
    idle(&mut w, 30);
    let u = urn(&w, h);
    assert_eq!((u.spin_rate, u.circle_rate, u.circle_radius), (0x1F4, 400, 50.0));
}

#[test]
fn with_the_flag_set_it_turns_from_the_start_with_no_lid() {
    let Some(a) = assets() else { return };
    // BgSpot18Basket_Init: Flags_GetSwitch(0x1C) set: BgSpot18Basket_SetupSpinning, no lid.
    let w = city(&a, |s| s.scene_flags[0x62].swch |= 1 << SWITCH);
    let h = the_urn(&w);
    assert_eq!(urn(&w, h).action, Action::Spinning);
    assert_eq!(urn(&w, h).actor.child, None);
    assert!(all::<BgSpot18Futa>(&w).is_empty());
}

#[test]
fn a_bomb_in_its_mouth_spins_it_wild_and_it_stops_on_a_prize() {
    let Some(a) = assets() else { return };
    let mut w = city(&a, |_| {});
    let h = the_urn(&w);
    turning(&mut w, h);
    // BgSpot18Basket_Spinning: an explosion on the mouth sphere from an actor 120 to 200 above
    // and within 32 across: OnePointCutscene_Init(4210, 240), ExplosionCs (timer 0, altitude 0),
    // its collision off (DynaPoly_DisableCollision).
    exploded(&mut w, h);
    let u = urn(&w, h);
    assert_eq!(u.timer, 0);
    assert!(onepoint(&w, 4210).is_some(), "one-point 4210");
    assert!(w.col.dyna.actors[u.bg as usize].collision_disabled);
    // The wild spin: the spin by 0x64 to 0xBB8; the wobble leaning by 0x1E to 0x5DC.
    idle(&mut w, 10);
    let u = urn(&w, h);
    assert_eq!(u.spin_rate, (0x1F4 + 10 * 0x64).min(0xBB8) as i16);
    assert_eq!(u.pivot_altitude, 10 * 0x1E);
    // Stopping on the piece of heart's third: at 0xD555 once the wobble's gone, its collision
    // back; then the prize.
    stop_at(&mut w, h, -0x6000);
    assert_eq!(urn(&w, h).prize, basket::PRIZE_HEART_PIECE);
    let got = prize(&mut w, h);
    let u = urn(&w, h);
    assert_eq!(u.actor.shape_rot.y, basket::STOPPED_ROT_Y[2]);
    assert_eq!((u.actor.shape_rot.x, u.actor.shape_rot.z, u.pivot_altitude), (0, 0, 0));
    assert!(!w.col.dyna.actors[u.bg as usize].collision_disabled);
    // The piece with collectible flag 0x1F (params 0x1F06) at sGoronVaseCollectibleRotY[1], a red
    // rupee at [0], a blue one at [2]; each thrown up at 11 (the C's write after the drop's own
    // 8, the frame's own update after it).
    assert_eq!(
        drops(&w, &got),
        vec![(ITEM00_RUPEE_RED, 0, -0x0FA0), (ITEM00_HEART_PIECE, COLLECTIBLE as i16, 0x0320), (ITEM00_RUPEE_BLUE, 0, 0x0FA0)]
    );
    assert!(urn(&w, h).is_heart_piece_given);
    // Frame 2: the chime (the piece's third).
    idle(&mut w, 1);
    assert!(sfx_on(&w, w.audio.frames, NA_SE_SY_CORRECT_CHIME));
    assert!(!sfx_on(&w, w.audio.frames, NA_SE_SY_TRE_BOX_APPEAR));
    // Frame 200: turning again.
    while urn(&w, h).timer < 199 {
        idle(&mut w, 1);
    }
    assert_eq!(urn(&w, h).action, Action::GivingPrize);
    idle(&mut w, 1);
    assert_eq!(urn(&w, h).action, Action::Spinning);
    // The piece given (isHeartPieceGiven): the third again gives a purple rupee in its place.
    idle(&mut w, 40);
    exploded(&mut w, h);
    stop_at(&mut w, h, 0x7C19);
    let got = prize(&mut w, h);
    assert_eq!(drops(&w, &got), vec![(ITEM00_RUPEE_RED, 0, -0x0FA0), (ITEM00_RUPEE_PURPLE, 0, 0x0320), (ITEM00_RUPEE_BLUE, 0, 0x0FA0)]);
}

#[test]
fn each_third_of_the_turn_has_its_prize() {
    let Some(a) = assets() else { return };
    // BgSpot18Basket_SetupStopping: rot.y in [0x26C2, 0x7C19): bombs, stopped at 0x8000;
    // [-0x2E93, 0x26C2): rupees, stopped at 0x2AAA.
    for (rot_y, want_prize, stopped, item, sfx) in [
        (0x5000i16, basket::PRIZE_BOMBS, 0x8000u16 as i16, ITEM00_BOMBS_A, NA_SE_SY_TRE_BOX_APPEAR),
        (0x0000, basket::PRIZE_RUPEES, 0x2AAA, ITEM00_RUPEE_GREEN, NA_SE_SY_TRE_BOX_APPEAR),
    ] {
        let mut w = city(&a, |_| {});
        let h = the_urn(&w);
        turning(&mut w, h);
        exploded(&mut w, h);
        stop_at(&mut w, h, rot_y);
        assert_eq!(urn(&w, h).prize, want_prize);
        let got = prize(&mut w, h);
        assert_eq!(urn(&w, h).actor.shape_rot.y, stopped);
        // Three of them, at the three yaws.
        assert_eq!(drops(&w, &got), vec![(item, 0, -0x0FA0), (item, 0, 0x0320), (item, 0, 0x0FA0)]);
        idle(&mut w, 1);
        assert!(sfx_on(&w, w.audio.frames, sfx));
    }
}

#[test]
fn the_piece_of_hearts_flag_set_gives_a_purple_rupee() {
    let Some(a) = assets() else { return };
    // Flags_GetCollectible(0x1F) set: ITEM00_RUPEE_PURPLE in the piece's place.
    let mut w = city(&a, |_| {});
    w.flags.set_collectible(COLLECTIBLE);
    let h = the_urn(&w);
    turning(&mut w, h);
    exploded(&mut w, h);
    stop_at(&mut w, h, -0x7000);
    let got = prize(&mut w, h);
    assert_eq!(drops(&w, &got), vec![(ITEM00_RUPEE_RED, 0, -0x0FA0), (ITEM00_RUPEE_PURPLE, 0, 0x0320), (ITEM00_RUPEE_BLUE, 0, 0x0FA0)]);
    assert!(!urn(&w, h).is_heart_piece_given);
}

#[test]
fn without_bombs_the_bomb_prize_is_nothing() {
    let Some(a) = assets() else { return };
    // Item_DropCollectible's func_8001F404: ITEM00_BOMBS_A with no bombs (INV_CONTENT(ITEM_BOMB)
    // ITEM_NONE): -1, nothing dropped (a child who brought a Bomb Flower's bomb).
    let mut w = city(&a, |s| s.inventory.items[oot_game::item::SLOT_BOMB] = oot_game::item::ITEM_NONE);
    let h = the_urn(&w);
    turning(&mut w, h);
    exploded(&mut w, h);
    stop_at(&mut w, h, 0x5000);
    assert!(prize(&mut w, h).is_empty());
    idle(&mut w, 1);
    // The sound still plays.
    assert!(sfx_on(&w, w.audio.frames, NA_SE_SY_TRE_BOX_APPEAR));
}

#[test]
fn a_blast_too_high_or_before_it_turns_does_nothing() {
    let Some(a) = assets() else { return };
    let mut w = city(&a, |_| {});
    let h = the_urn(&w);
    // Waiting under its lid its spheres aren't set for hits: a bomb in its mouth does nothing.
    bomb_above(&mut w, h, 160.0);
    idle(&mut w, 15);
    assert_eq!(urn(&w, h).action, Action::Inactive);
    turning(&mut w, h);
    // An explosion 210 above (over 200: the mouth reached, the height check fails).
    bomb_above(&mut w, h, 210.0);
    idle(&mut w, 15);
    assert_eq!(urn(&w, h).action, Action::Spinning);
}

#[test]
fn a_bomb_must_rest_in_its_mouth() {
    let Some(a) = assets() else { return };
    let mut w = city(&a, |_| {});
    let h = the_urn(&w);
    turning(&mut w, h);
    // gGoronCityVaseCol down its middle: the inside of the mouth (a bomb resting there blows 120
    // to 200 above the urn); 30 out, the funnel round the hole (over 200 above: too high); 45
    // out, past its rim, the floor.
    let p = urn(&w, h).actor.world_pos;
    let down = |w: &PlayState, r: f32| w.col.entity_raycast_down(p + Vec3::new(r, 600.0, 0.0));
    let (inside, poly) = down(&w, 0.0);
    assert_eq!(poly.map(|q| q.bg), Some(urn(&w, h).bg));
    assert!(inside - p.y > 120.0 && inside - p.y < 200.0, "{}", inside - p.y);
    let (funnel, poly) = down(&w, 30.0);
    assert_eq!(poly.map(|q| q.bg), Some(urn(&w, h).bg));
    assert!(funnel - p.y > 200.0, "{}", funnel - p.y);
    let (out, poly) = down(&w, 45.0);
    assert_ne!(poly.map(|q| q.bg), Some(urn(&w, h).bg), "{out}");
}

#[test]
fn a_bomb_dropped_into_it_rides_it_and_blows_in_its_mouth() {
    let Some(a) = assets() else { return };
    let mut w = city(&a, |_| {});
    let h = the_urn(&w);
    turning(&mut w, h);
    // Dropped 250 above where the turning urn will be 15 frames on (circleRate 400 a frame, 50
    // out), with 40 on its fuse: it falls into the mouth, rests on its inside and is carried
    // round with it (DYNA_TRANSFORM_POS), and blows there.
    let u = urn(&w, h);
    let angle = u.circle_move_angle.wrapping_add(15 * 400);
    let at = u.actor.home_pos + Vec3::new(eng_math::sin_s(angle) * 50.0, 250.0, eng_math::cos_s(angle) * 50.0);
    let b = w.actor_spawn(ACTOR_EN_BOM, at, [0; 3], 0).expect("En_Bom");
    w.actors.downcast_mut::<EnBom>(b).unwrap().timer = 40;
    idle(&mut w, 30);
    let (bp, up) = (w.actors.actor(b).unwrap().world_pos, urn(&w, h).actor.world_pos);
    let across = Vec3::new(bp.x - up.x, 0.0, bp.z - up.z).length();
    assert!(bp.y - up.y > 120.0 && bp.y - up.y < 200.0 && across < 32.0, "the bomb at {bp}, the urn at {up}");
    for _ in 0..20 {
        idle(&mut w, 1);
        if urn(&w, h).action == Action::ExplosionCs {
            return;
        }
    }
    panic!("the bomb didn't start it: {:?}", urn(&w, h).action);
}
