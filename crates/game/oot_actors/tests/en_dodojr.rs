//! GAME-09 milestone 3 against the C: `En_Dodojr` (`z_en_dodojr.c`), the Baby Dodongo, in Dodongo's
//! Cavern's room 7 (Master Quest: two, params 0xFFFF, at (-203, 16, -2929) and (22, 16, -2970) on
//! the room's flat floor, y 16).
//!
//! Entered by `ENTR_DODONGOS_CAVERN_0` on a `cavern-open` save with the entrance wall broken, room 7
//! loaded with `Room_RequestNewRoom`, Link placed out of reach. Hits are put on its collider as
//! the collision check leaves them (`AC_HIT` with the damage table's damage and reaction).
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_bom::{ACTOR_EN_BOM, EnBom};
use oot_actors::en_dodojr::*;
use oot_game::actor::ACTOR_FLAG_ATTENTION_ENABLED;
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::sfx::{NA_SE_EN_DODO_M_GND, NA_SE_EN_GOMA_JR_FREEZE, NA_SE_IT_BOMB_EXPLOSION};
use oot_game::collision_check::AC_HIT;
use oot_game::env::clock_time;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// `SCENE_DODONGOS_CAVERN` (`scene_table.h`: 0x01).
const SCENE_DODONGOS_CAVERN: usize = 0x01;
/// Room 7's two, and a spot out of reach of both (over 320 across).
const WEST: Vec3 = Vec3::new(-203.0, 16.0, -2929.0);
const EAST: Vec3 = Vec3::new(22.0, 16.0, -2970.0);
const AWAY: Vec3 = Vec3::new(0.0, 16.0, -2600.0);
/// Within 320 of the east one only.
const NEAR_EAST: Vec3 = Vec3::new(22.0, 16.0, -2700.0);
const SKEL_MESH: &str = "mesh/object_dodojr/object_dodojr_Skel_0020E0";

/// Child Link in room 7 (`cavern-open`, the entrance wall broken), out of reach, three frames on,
/// the audio logged.
fn room7(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DODONGOS_CAVERN_0").expect("entrance");
    let mut save = SaveContext::new(e, false, clock_time(10, 0) as u16);
    save.apply_preset("cavern-open").unwrap();
    save.set_event_chk_inf(oot_actors::bg_dodoago::EVENTCHKINF_B0);
    save.scene_flags[SCENE_DODONGOS_CAVERN].swch |= (1 << 7) | (1 << 0x0C);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    w.audio.log = Some(Default::default());
    idle(&mut w, 2);
    assert!(w.room_request(7));
    idle(&mut w, 1);
    w.room_change_done();
    w.place_player(AWAY, i16::MIN);
    idle(&mut w, 3);
    w
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn dodojr_at(w: &PlayState, home: Vec3) -> ActorHandle {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnDodojr>(h).is_some_and(|d| (d.actor.home_pos.x - home.x).abs() < 1.0 && (d.actor.home_pos.z - home.z).abs() < 1.0)).expect("the Baby Dodongo")
}

fn dodojr(w: &PlayState, h: ActorHandle) -> &EnDodojr {
    w.actors.downcast::<EnDodojr>(h).unwrap()
}

fn sfx_frames(w: &PlayState, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, _)| f).collect()
}

/// The meshes the last frame draws.
fn drawn(w: &PlayState) -> Vec<String> {
    let f = w.current_frame();
    let mut out = oot_game::play::DrawOut::default();
    w.draw(&f, &oot_game::play::ViewInfo::new(f.view.eye, glam::Mat4::IDENTITY), &mut out);
    out.opa.iter().chain(out.xlu.iter()).map(|c| c.mesh.name.clone()).collect()
}

/// Ticks until `f` holds of it; the frames it took.
fn until(w: &mut PlayState, h: ActorHandle, max: usize, f: impl Fn(&EnDodojr) -> bool) -> usize {
    for n in 1..=max {
        idle(w, 1);
        if f(dodojr(w, h)) {
            return n;
        }
    }
    panic!("not within {max} frames: {:?} at {}", dodojr(w, h).action, dodojr(w, h).actor.world_pos);
}

/// `EnDodojr_EmergeFromGround`'s frames from rot.x -0x4000 to level.
fn emerge_frames() -> usize {
    let mut rot = -0x4000i16;
    for n in 1.. {
        eng_math::smooth_step_to_s(&mut rot, 0, 4, 0x3E8, 0x64);
        if rot == 0 {
            return n;
        }
    }
    unreachable!()
}

/// The east one surfaced and crawling, Link where he stood to wake it (270 north of it).
fn crawling(a: &Arc<GameAssets>) -> (PlayState, ActorHandle) {
    let mut w = room7(a);
    let h = dodojr_at(&w, EAST);
    w.place_player(NEAR_EAST, i16::MIN);
    until(&mut w, h, 100, |d| d.action == Action::CrawlTowardsTarget);
    (w, h)
}

/// A hit on it as the collision check leaves one: `AC_HIT`, the table's damage and reaction.
fn hit(w: &mut PlayState, h: ActorHandle, damage: u8, reaction: u8) {
    let d = w.actors.downcast_mut::<EnDodojr>(h).unwrap();
    d.collider.base.ac_flags |= AC_HIT;
    d.actor.col_chk_info.damage = damage;
    d.actor.col_chk_info.damage_reaction = reaction;
}

#[test]
fn buried_until_link_comes_within_320_then_it_surfaces_and_crawls() {
    let Some(a) = assets() else { return };
    let mut w = room7(&a);
    let (east, west) = (dodojr_at(&w, EAST), dodojr_at(&w, WEST));
    // EnDodojr_Init: scale 0.02, the Baby Dodongo's Navi text, not targetable, one health
    // (sColChkInit), buried; not drawn (EnDodojr_Draw skips EnDodojr_WaitUnderground).
    for h in [east, west] {
        let d = dodojr(&w, h);
        assert_eq!((d.action, d.actor.scale.x, d.actor.navi_enemy_id, d.actor.col_chk_info.health), (Action::WaitUnderground, 0.02, NAVI_ENEMY_BABY_DODONGO, 1));
        assert_eq!(d.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
        assert_eq!(d.actor.shape_shadow_draw, oot_game::shadow::ShadowDraw::None);
    }
    assert!(!drawn(&w).iter().any(|m| m == SKEL_MESH));
    // Link 270 from the east one (321 from the west one): only the east one wakes:
    // NA_SE_EN_DODO_M_UP, 60 down, nose-down (rot.x -0x4000), targetable; the dust at y 0
    // (@bug (game): floorHeight not read yet).
    w.place_player(NEAR_EAST, i16::MIN);
    let n = until(&mut w, east, 5, |d| d.action == Action::EmergeFromGround);
    assert!(n <= 2);
    let d = dodojr(&w, east);
    assert_eq!((d.actor.world_pos.y, d.actor.home_pos.y, d.actor.shape_rot.x, d.actor.world_rot.x), (-44.0, 16.0, -0x4000, -0x4000));
    assert_ne!(d.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!(d.dust_pos.y, 0.0);
    assert_eq!(sfx_frames(&w, NA_SE_EN_DODO_M_UP).len(), 1);
    assert_eq!(dodojr(&w, west).action, Action::WaitUnderground);
    // EnDodojr_EmergeFromGround: tipping level (a quarter, 100 to 1000 a frame) and rising with
    // it; level: its shadow, speed 2.6, crawling.
    let n = until(&mut w, east, 60, |d| d.action == Action::CrawlTowardsTarget);
    assert_eq!(n, emerge_frames());
    let d = dodojr(&w, east);
    assert_eq!((d.actor.shape_rot.x, d.actor.world_rot.x, d.actor.speed_xz), (0, 0, 2.6));
    assert_eq!(d.actor.shape_shadow_draw, oot_game::shadow::ShadowDraw::Circle);
    assert!((d.actor.world_pos.y - 16.0).abs() < 1.0, "{}", d.actor.world_pos);
    // Drawn now.
    assert!(drawn(&w).iter().any(|m| m == SKEL_MESH));
}

#[test]
fn it_crawls_to_link_and_jumps_at_him_within_40() {
    let Some(a) = assets() else { return };
    let (mut w, h) = crawling(&a);
    let health = w.save.health;
    let start = w.player().actor.world_pos;
    let d0 = dodojr(&w, h).actor.world_pos.distance(start);
    // EnDodojr_CrawlTowardsTarget: at 2.6 (3.9 a frame), NA_SE_EN_DODO_M_MOVE every 5 frames (DECR from 5),
    // turning towards Link (or 80 off him): it comes. Within 40 across: NA_SE_EN_DODO_M_CRY and
    // the jump (up at 10, three bounces).
    let n = until(&mut w, h, 300, |d| d.action == Action::JumpAttackBounce);
    let d = dodojr(&w, h);
    assert!(d.actor.xz_dist_to_player <= 40.0, "{}", d.actor.xz_dist_to_player);
    assert!(n as f32 * 3.9 >= d0 - 45.0);
    assert_eq!(d.counter, 3);
    assert!(sfx_frames(&w, NA_SE_EN_DODO_M_CRY).len() == 1);
    let moves = sfx_frames(&w, NA_SE_EN_DODO_M_MOVE);
    assert!(moves.windows(2).all(|p| p[1] - p[0] == 5), "{moves:?}");
    // EnDodojr_UpdateBounces: landing (NA_SE_EN_DODO_M_GND and dust) up at 10 / (4 - counter): 10,
    // 5; the third landing stops it: crawling again.
    let gnd0 = sfx_frames(&w, NA_SE_EN_DODO_M_GND).len();
    let mut ups = Vec::new();
    for _ in 0..120 {
        let before = dodojr(&w, h).counter;
        idle(&mut w, 1);
        let d = dodojr(&w, h);
        if d.counter != before {
            ups.push(d.actor.velocity.y);
        }
        if d.action == Action::CrawlTowardsTarget {
            break;
        }
    }
    assert_eq!(ups, vec![10.0, 5.0, 0.0]);
    assert_eq!(sfx_frames(&w, NA_SE_EN_DODO_M_GND).len() - gnd0, 3);
    assert_eq!(dodojr(&w, h).action, Action::CrawlTowardsTarget);
    // Its bite (AT, 8) hit Link on the way.
    assert!(w.save.health < health, "{} of {}", w.save.health, health);
}

#[test]
fn a_hit_kills_it_and_it_blows_up_in_its_own_bomb() {
    let Some(a) = assets() else { return };
    let (mut w, h) = crawling(&a);
    // Damage with no reaction (a sword's: the table's Kokiri Sword entry 1): Enemy_StartFinishingBlow
    // (the freeze-flash 5), two frames' freeze, then NA_SE_EN_DODO_M_DEAD, untargetable, flipping
    // (up at 10, three bounces).
    hit(&mut w, h, 1, 0);
    idle(&mut w, 1);
    // (EnDodojr_WaitFreezeFrames runs the same update: 1 left.)
    assert_eq!((dodojr(&w, h).action, dodojr(&w, h).freeze_frame_timer), (Action::WaitFreezeFrames, 1));
    assert_eq!(w.actors.freeze_flash_timer, 5);
    until(&mut w, h, 20, |d| d.action == Action::StandardDeathBounce);
    let d = dodojr(&w, h);
    assert_eq!((d.counter, d.actor.velocity.y, d.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED), (3, 10.0, 0));
    assert_eq!(sfx_frames(&w, NA_SE_EN_DODO_M_DEAD).len(), 1);
    // Landed for good: lying, 60 on its timer, seven flashes of red (7 frames, then 6, ... 1).
    until(&mut w, h, 120, |d| d.action == Action::DeathSequence);
    assert_eq!((dodojr(&w, h).timer, dodojr(&w, h).counter), (60, 7));
    let mut flashes = Vec::new();
    let pos = loop {
        let before = dodojr(&w, h).counter;
        idle(&mut w, 1);
        let d = dodojr(&w, h);
        if d.counter != before {
            flashes.push(d.actor.color_filter_timer);
        }
        if d.action == Action::DropItem {
            break d.actor.world_pos;
        }
        assert!(flashes.len() <= 8);
    };
    assert_eq!(flashes, vec![7, 6, 5, 4, 3, 2, 1]);
    // Then a bomb where it lies (ACTOR_EN_BOM, BOMB_BODY) with its fuse at 0, and 8 frames to
    // its drop.
    let bombs: Vec<ActorHandle> = w.actors.all().into_iter().filter(|&b| w.actors.downcast::<EnBom>(b).is_some()).collect();
    assert_eq!(bombs.len(), 1);
    let b = w.actors.downcast::<EnBom>(bombs[0]).unwrap();
    assert!(b.actor.world_pos.distance(pos) < 0.01);
    assert_eq!(b.timer, 0);
    assert_eq!(dodojr(&w, h).timer, 8);
    // Not drawn while it drops its item.
    assert!(!drawn(&w).iter().any(|m| m == SKEL_MESH));
    // The bomb blows at once (BOMB_EXPLOSION) in the bomb's next turn.
    idle(&mut w, 1);
    assert_eq!(w.actors.actor(bombs[0]).unwrap().params, 1);
    // EnDodojr_DropItem: 8 frames, the drop (table 4), gone.
    let n = until_gone(&mut w, h, 20);
    assert_eq!(n, 7);
    let _ = ACTOR_EN_BOM;
}

/// Ticks until it's gone; the frames it took.
fn until_gone(w: &mut PlayState, h: ActorHandle, max: usize) -> usize {
    for n in 1..=max {
        idle(w, 1);
        if w.actors.actor(h).is_none_or(|a| a.killed) {
            return n;
        }
    }
    panic!("still there");
}

#[test]
fn a_deku_nut_stuns_it_for_120_frames() {
    let Some(a) = assets() else { return };
    let (mut w, h) = crawling(&a);
    // Reaction 1 (the table's Deku Nut: damage 0): NA_SE_EN_GOMA_JR_FREEZE, stunTimer 120, blue
    // 120 frames, flipping (up at 10).
    hit(&mut w, h, 0, 1);
    idle(&mut w, 1);
    let d = dodojr(&w, h);
    assert_eq!((d.action, d.stun_timer, d.counter), (Action::StunnedBounce, 120, 3));
    assert_eq!(sfx_frames(&w, NA_SE_EN_GOMA_JR_FREEZE).len(), 1);
    // EnDodojr_StunnedBounce: the filter held at the stun (120); landed for good, stunned.
    until(&mut w, h, 120, |d| d.action == Action::Stunned);
    assert_eq!(dodojr(&w, h).stun_timer, 120);
    // EnDodojr_Stunned: the stun down a frame; for its last 30, shaking 1.5 on x and z each way.
    until(&mut w, h, 120, |d| d.stun_timer == 29);
    let mut xs = Vec::new();
    for _ in 0..4 {
        xs.push(dodojr(&w, h).actor.world_pos.x);
        idle(&mut w, 1);
    }
    assert!((xs[1] - xs[0]).abs() > 1.0 && (xs[2] - xs[1]).abs() > 1.0, "{xs:?}");
    // Out: crawling at 2.6.
    let n = until(&mut w, h, 40, |d| d.action == Action::CrawlTowardsTarget);
    assert!(n <= 29);
    assert_eq!(dodojr(&w, h).actor.speed_xz, 2.6);
}

#[test]
fn it_goes_for_a_bomb_and_eats_it_and_blows_up() {
    let Some(a) = assets() else { return };
    let (mut w, h) = crawling(&a);
    // A bomb (params 0, no parent) 45 ahead of it: EnDodojr_CheckNearbyBombs finds it; it turns
    // to it and within 30 eats it (EnDodojr_TryEatBomb: the bomb's parent set to it):
    // NA_SE_EN_DODO_M_EAT, stopped.
    let d = dodojr(&w, h);
    let ahead = d.actor.world_pos + Vec3::new(eng_math::sin_s(d.actor.world_rot.y), 0.0, eng_math::cos_s(d.actor.world_rot.y)) * 45.0;
    let b = w.actor_spawn(ACTOR_EN_BOM, ahead, [0; 3], 0).unwrap();
    until(&mut w, h, 60, |d| d.action == Action::EatBomb);
    assert_eq!(w.actors.actor(b).unwrap().parent, Some(h));
    assert_eq!(dodojr(&w, h).bomb, Some(b));
    assert_eq!(sfx_frames(&w, NA_SE_EN_DODO_M_EAT).len(), 1);
    assert_eq!(dodojr(&w, h).actor.speed_xz, 0.0);
    // EnDodojr_EatBomb: frames 8 to 11 of the jump the bomb at its mouth (headPos, from the last
    // draw) with its fuse held; then NA_SE_EN_DODO_K_DRINK, the bomb gone, 24 frames.
    let fuse = w.actors.downcast::<EnBom>(b).unwrap().timer;
    idle(&mut w, 1);
    let bomb = w.actors.downcast::<EnBom>(b).unwrap();
    assert_eq!(bomb.timer, fuse);
    let head = dodojr(&w, h).head_pos;
    assert!(head.distance(dodojr(&w, h).actor.world_pos) < 30.0, "head {head}");
    until(&mut w, h, 20, |d| d.action == Action::SwallowBomb);
    assert!(w.actors.actor(b).is_none_or(|a| a.killed));
    assert_eq!(sfx_frames(&w, NA_SE_EN_DODO_K_DRINK).len(), 1);
    assert_eq!(dodojr(&w, h).timer, 24);
    // 24 frames on, the bomb inside: NA_SE_IT_BOMB_EXPLOSION, red 8 frames, untargetable,
    // flipping; its body swollen 1.2 times the red's frames over 8.
    let n = until(&mut w, h, 40, |d| d.action == Action::SwallowedBombDeathBounce);
    assert_eq!(n, 24);
    assert_eq!(sfx_frames(&w, NA_SE_IT_BOMB_EXPLOSION).len(), 1);
    idle(&mut w, 1);
    let d = dodojr(&w, h);
    assert!((d.root_scale - 1.2 * (d.actor.color_filter_timer as f32 / 8.0)).abs() < 1e-6);
    assert!(d.root_scale > 0.0);
    // Its bounces, lying, its flashes, then its own bomb.
    until(&mut w, h, 120, |d| d.action == Action::SwallowedBombDeathSequence);
    assert_eq!((dodojr(&w, h).timer, dodojr(&w, h).counter), (60, 7));
    until(&mut w, h, 120, |d| d.action == Action::DropItem);
    let bombs = w.actors.all().into_iter().filter(|&x| w.actors.downcast::<EnBom>(x).is_some_and(|b| !b.actor.killed)).count();
    assert_eq!(bombs, 1);
}

#[test]
fn a_wall_sends_it_back_underground() {
    let Some(a) = assets() else { return };
    let (mut w, h) = crawling(&a);
    // Crawling with a wall hit (BGCHECKFLAG_WALL from its floor and wall check):
    // NA_SE_EN_DODO_M_DOWN, its shadow off, untargetable, its home where it is; 30 frames tipping
    // nose-down and sinking 60; gone.
    {
        let d = w.actors.downcast_mut::<EnDodojr>(h).unwrap();
        d.actor.world_pos = Vec3::new(-300.0, 16.0, -3100.0);
        d.actor.world_rot.y = i16::MIN;
        d.actor.shape_rot.y = i16::MIN;
    }
    // Link facing it (an enemy updates only inside its culling volume as last drawn).
    w.place_player(Vec3::new(-300.0, 16.0, -2750.0), i16::MIN);
    until(&mut w, h, 120, |d| d.action == Action::Despawn);
    let d = dodojr(&w, h);
    assert_eq!((d.timer, d.actor.shape_shadow_draw, d.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED), (30, oot_game::shadow::ShadowDraw::None, 0));
    assert_eq!(sfx_frames(&w, NA_SE_EN_DODO_M_DOWN).len(), 1);
    let home_y = d.actor.home_pos.y;
    idle(&mut w, 15);
    // DECR(timer) to 15: (30 - 15) / 30 of 60 down.
    let d = dodojr(&w, h);
    assert_eq!(d.timer, 15);
    assert!((d.actor.world_pos.y - (home_y - 30.0)).abs() < 1e-3, "{}", d.actor.world_pos);
    let n = until_gone(&mut w, h, 30);
    assert_eq!(n, 15);
}
