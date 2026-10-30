//! GAME-13 milestone 2 against the C: the Moblins (`z_en_mb.c`) in the adult's Sacred Forest
//! Meadow (`spot05` layer 2), entered at `ENTR_SACRED_FOREST_MEADOW_0` (from the Lost Woods, at
//! (-201, 0, 2183)) on `epona-won`. No fight is scripted: each case sets a Moblin's state or puts
//! Link where the case needs him and runs a few frames.
//!
//! The meadow's patrols (params 0x00FF to 0x04FF, paths 0 to 4) and the club Moblin (params 0 at
//! (-3, 160, -1357)). Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_mb::{self, EnMb, Func};
use oot_actors::en_part::EnPart;
use oot_actors::player::STATE2_7;
use oot_game::actor::*;
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::collision_check::{AC_HIT, MASS_HEAVY, MASS_IMMOVABLE};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

/// The club Moblin's placement.
const CLUB_HOME: Vec3 = Vec3::new(-3.0, 160.0, -1357.0);
/// Path 0's patrol: placed at (-44, 0, 1474) facing west (-0x4000); its path from (-400, 0, 1474)
/// to (250, 0, 1474).
const PATROL0_HOME: Vec3 = Vec3::new(-44.0, 0.0, 1474.0);
/// The entrance's spawn (spawn 0).
const SPAWN: Vec3 = Vec3::new(-201.0, 0.0, 2183.0);

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// An adult in the meadow at 10:00 (layer 2) with `epona-won`; the sounds logged; four frames on.
fn meadow(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_SACRED_FOREST_MEADOW_0").expect("entrance");
    let mut save = SaveContext::new(e, true, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("epona-won").unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    w.audio.log = Some(Default::default());
    idle(&mut w, 4);
    // Link stood at the spawn (his entrance walk over: Player's state freezes the enemies'
    // category while it lasts), facing north.
    w.place_player(SPAWN, -0x8000);
    idle(&mut w, 2);
    w
}

fn moblins(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnMb>(h).is_some_and(|a| !a.base().killed)).collect()
}

fn mb(w: &PlayState, h: ActorHandle) -> &EnMb {
    w.actors.downcast::<EnMb>(h).unwrap()
}

fn mb_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnMb {
    w.actors.downcast_mut::<EnMb>(h).unwrap()
}

/// The Moblin placed at `home` (x and z).
fn moblin_at(w: &PlayState, home: Vec3) -> ActorHandle {
    moblins(w).into_iter().find(|&h| (mb(w, h).actor.home_pos.x, mb(w, h).actor.home_pos.z) == (home.x, home.z)).expect("the Moblin")
}

fn sfx_since(w: &PlayState, from: usize, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx[from..].iter().any(|&(_, s, _)| s == id)
}

fn sfx_len(w: &PlayState) -> usize {
    w.audio.log.as_ref().unwrap().sfx.len()
}

/// The floor's height under (x, z), from 1000 up.
fn floor(w: &PlayState, x: f32, z: f32) -> f32 {
    oot_game::shadow::floor_under(&w.col, Vec3::new(x, 1000.0, z)).0
}

#[test]
fn the_meadow_has_five_patrols_and_the_club_moblin() {
    let Some(a) = assets() else { return };
    // EnMb_Draw's two skeletons are baked.
    for club in [false, true] {
        assert!(pack().unwrap().bake(en_mb::bake_name(club)).is_ok(), "{}", en_mb::bake_name(club));
    }
    let w = meadow(&a);
    let all = moblins(&w);
    assert_eq!(all.len(), 6, "the six En_Mb of spot05's layer 2");
    let (clubs, patrols): (Vec<_>, Vec<_>) = all.into_iter().partition(|&h| mb(&w, h).ty() == en_mb::ENMB_TYPE_CLUB);
    assert_eq!(clubs.len(), 1);
    // EnMb_Init's default case: scale 0.014, params written to ENMB_TYPE_SPEAR_PATROL, path from
    // bits 8 to 15, 1 health, heavy, not targetable; EnMb_SetupSpearPatrolTurnTowardsWaypoint's
    // EnMb_NextWaypoint from 0: direction 1, waypoint 1.
    let mut paths: Vec<i8> = patrols.iter().map(|&h| mb(&w, h).path).collect();
    paths.sort();
    assert_eq!(paths, vec![0, 1, 2, 3, 4]);
    for &h in &patrols {
        let m = mb(&w, h);
        assert_eq!(m.actor.params, en_mb::ENMB_TYPE_SPEAR_PATROL);
        assert_eq!(m.actor.scale, Vec3::splat(0.014));
        assert_eq!((m.actor.col_chk_info.health, m.actor.col_chk_info.mass), (1, MASS_HEAVY));
        assert_eq!(m.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
        assert_eq!(m.actor.navi_enemy_id, en_mb::NAVI_ENEMY_MOBLIN);
        assert_eq!((m.waypoint, m.direction), (1, 1));
        assert_eq!((m.max_home_dist, m.player_detection_range), (350.0, 1750.0));
        // Link 700 south of the nearest corridor: none has seen him.
        assert_eq!(m.func, Func::SpearPatrolTurnTowardsWaypoint, "patrol on path {}", m.path);
    }
    // Path 0's: the next point (250, 0, 1474).
    let p0 = moblin_at(&w, PATROL0_HOME);
    assert_eq!(mb(&w, p0).waypoint_pos, Vec3::new(250.0, 0.0, 1474.0));
    // The club Moblin: placed facing Link (yaw 0 towards +z, Link south of it), so not turned;
    // 0.02, 6 health, immovable, its cylinder 45 by 170, not targetable, NAVI_ENEMY_MOBLIN_CLUB.
    let c = mb(&w, clubs[0]);
    assert_eq!((c.actor.world_pos.x, c.actor.world_pos.z), (CLUB_HOME.x, CLUB_HOME.z));
    assert_eq!(c.actor.world_rot.y, 0);
    assert_eq!(c.actor.scale, Vec3::splat(0.02));
    assert_eq!((c.actor.col_chk_info.health, c.actor.col_chk_info.mass), (6, MASS_IMMOVABLE));
    assert_eq!((c.body_collider.dim.radius, c.body_collider.dim.height), (45, 170));
    assert_eq!(c.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!(c.actor.navi_enemy_id, en_mb::NAVI_ENEMY_MOBLIN_CLUB);
    assert_eq!(c.player_detection_range, 710.0);
    assert_eq!(c.attack_collider.info.at_dmg_info.dmg_flags, oot_game::collision_check::DMG_UNBLOCKABLE);
    assert_eq!((c.func, c.state), (Func::ClubWaitPlayerNear, en_mb::ENMB_STATE_IDLE));
}

#[test]
fn init_picks_the_type_from_the_params() {
    let Some(a) = assets() else { return };
    let mut w = meadow(&a);
    // ENMB_TYPE_SPEAR_GUARD (-1): the spear skeleton at the default scale (0.01), 2 health,
    // heavy, maxHomeDist 1000; EnMb_SetupSpearGuardLookAround (timer1 30 to 79).
    let g = w.actor_spawn(en_mb::ACTOR_EN_MB, Vec3::new(0.0, 0.0, 1800.0), [0; 3], -1).expect("spawned");
    let m = mb(&w, g);
    assert_eq!((m.ty(), m.actor.scale), (en_mb::ENMB_TYPE_SPEAR_GUARD, Vec3::splat(0.01)));
    assert_eq!((m.actor.col_chk_info.health, m.actor.col_chk_info.mass), (2, MASS_HEAVY));
    assert_eq!((m.max_home_dist, m.player_detection_range), (1000.0, 1750.0));
    assert_eq!(m.func, Func::SpearGuardLookAround);
    assert!((30..80).contains(&m.timer1));
    // 0x0200: a patrol on path 2 (whose points are (-360, 0, 877) and (300, 0, 877)).
    let p = w.actor_spawn(en_mb::ACTOR_EN_MB, Vec3::new(0.0, 0.0, 877.0), [0; 3], 0x0200).expect("spawned");
    let m = mb(&w, p);
    assert_eq!((m.actor.params, m.path, m.waypoint), (en_mb::ENMB_TYPE_SPEAR_PATROL, 2, 1));
    assert_eq!(m.waypoint_pos, Vec3::new(300.0, 0.0, 877.0));
    // A club Moblin placed facing away from Link (yaw 0x8000 here, Link south): turned round
    // (0x8000 more) and moved 600 along z.
    let c = w.actor_spawn(en_mb::ACTOR_EN_MB, Vec3::new(0.0, 0.0, 1000.0), [0, -0x8000, 0], 0).expect("spawned");
    let m = mb(&w, c);
    assert_eq!((m.actor.world_rot.y, m.actor.shape_rot.y), (0, 0));
    assert_eq!(m.actor.world_pos.z, 1600.0);
}

#[test]
fn a_patrol_turns_back_at_its_paths_end() {
    let Some(a) = assets() else { return };
    let mut w = meadow(&a);
    let h = moblin_at(&w, PATROL0_HOME);
    // 10 short of its point 1 (250, 0, 1474), facing it, its looking round done.
    {
        let m = mb_mut(&mut w, h);
        m.actor.world_pos = Vec3::new(240.0, 0.0, 1474.0);
        m.actor.world_rot.y = 0x4000;
        m.actor.shape_rot.y = 0x4000;
        m.timer1 = 0;
    }
    // EnMb_SpearPatrolTurnTowardsWaypoint: facing its point already (Math_SmoothStepToS returns
    // 0): EnMb_SetupSpearPatrolWalkTowardsWaypoint.
    idle(&mut w, 1);
    assert_eq!(mb(&w, h).func, Func::SpearPatrolWalkTowardsWaypoint);
    assert_eq!(mb(&w, h).state, en_mb::ENMB_STATE_WALK);
    // At 0.6 a frame it's within 8 of the point in 4 frames: looking round again, the next
    // point: waypoint 1 is the last (count 2), so direction -1, waypoint 0 at (-400, 0, 1474).
    let mut n = 0;
    while mb(&w, h).func == Func::SpearPatrolWalkTowardsWaypoint {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 10, "never reached its point");
    }
    let m = mb(&w, h);
    assert_eq!(m.func, Func::SpearPatrolTurnTowardsWaypoint);
    assert_eq!((m.waypoint, m.direction), (0, -1));
    assert_eq!(m.waypoint_pos, Vec3::new(-400.0, 0.0, 1474.0));
    assert!((40..120).contains(&m.timer1), "Rand_S16Offset(40, 80): {}", m.timer1);
    assert_eq!(m.actor.speed_xz, 0.0);
}

#[test]
fn a_patrol_sees_link_in_its_corridor_and_charges() {
    let Some(a) = assets() else { return };
    let mut w = meadow(&a);
    let h = moblin_at(&w, PATROL0_HOME);
    let me = mb(&w, h);
    // EnMb_IsPlayerInCorridor: facing west (-0x4000, aligned -0x4000): |dx| < 400, |dz| < 37.
    assert!(!me.is_player_in_corridor(&w));
    // Link 150 west of it in its corridor, facing it.
    w.place_player(PATROL0_HOME - Vec3::X * 150.0, 0x4000);
    assert!(mb(&w, h).is_player_in_corridor(&w));
    let from = sfx_len(&w);
    idle(&mut w, 1);
    // In front of it (shape.rot.y -0x4000, Link at -0x4000): EnMb_FindWaypointTowardsPlayer
    // picks point 0 (-400, 0, 1474), straight towards him; NA_SE_EN_MORIBLIN_VOICE;
    // EnMb_SetupSpearPrepareAndCharge: NA_SE_EN_MORIBLIN_SPEAR_AT, the attack state.
    let m = mb(&w, h);
    assert_eq!(m.func, Func::SpearPatrolPrepareAndCharge);
    assert_eq!(m.state, en_mb::ENMB_STATE_ATTACK);
    assert_eq!((m.waypoint, m.waypoint_pos), (0, Vec3::new(-400.0, 0.0, 1474.0)));
    assert_eq!(m.actor.world_rot.y, -0x4000);
    assert!(sfx_since(&w, from, en_mb::NA_SE_EN_MORIBLIN_VOICE));
    assert!(sfx_since(&w, from, en_mb::NA_SE_EN_MORIBLIN_SPEAR_AT));
    // timer3 (the prepare animation's last frame + 6) runs out: the charge at 10, its spear's
    // quad on (attack 1); the spear catches Link (play->grabPlayer, PLAYER_STATE2_7, his parent
    // the Moblin), who's carried 89 in front of it.
    let mut held = false;
    for _ in 0..80 {
        idle(&mut w, 1);
        let p = w.player();
        if p.state2 & STATE2_7 != 0 && p.actor.parent == Some(h) {
            held = true;
            break;
        }
    }
    assert!(held, "Link wasn't caught (func {:?})", mb(&w, h).func);
    let m = mb(&w, h);
    assert_eq!((m.attack, m.actor.speed_xz), (en_mb::ENMB_ATTACK_SPEAR, 10.0));
    // The next frame he's put 89 ahead along its facing (and 10 to the side), from where it was
    // before its move (10 a frame) and his own update after.
    idle(&mut w, 1);
    let (m, p) = (mb(&w, h), w.player());
    if p.state2 & STATE2_7 != 0 {
        let r = m.actor.shape_rot.y;
        let want_x = m.actor.world_pos.x + eng_math::cos_s(r) * 10.0 + eng_math::sin_s(r) * 89.0;
        assert!((p.actor.world_pos.x - want_x).abs() < 20.0, "Link at {} for {want_x}", p.actor.world_pos.x);
        assert!((p.actor.world_pos.z - m.actor.world_pos.z).abs() < 15.0);
    }
    // At the floor's end (none 110 ahead): Link let go, the slide back (EnMb_SpearPatrolEndCharge).
    let mut ended = false;
    for _ in 0..120 {
        idle(&mut w, 1);
        if mb(&w, h).func == Func::SpearPatrolEndCharge {
            ended = true;
            break;
        }
    }
    assert!(ended, "the charge never ended (at {:?})", mb(&w, h).actor.world_pos);
    assert_eq!(w.player().state2 & STATE2_7, 0, "Link still held");
    assert_eq!(mb(&w, h).attack, en_mb::ENMB_ATTACK_NONE);
}

#[test]
fn the_club_moblin_strikes_and_its_shock_wave_runs_at_link() {
    let Some(a) = assets() else { return };
    let mut w = meadow(&a);
    let h = moblins(&w).into_iter().find(|&h| mb(&w, h).ty() == en_mb::ENMB_TYPE_CLUB).unwrap();
    // Link 300 in front of it (south), within 710 of its home.
    let at = Vec3::new(CLUB_HOME.x, 0.0, CLUB_HOME.z + 300.0);
    let y = floor(&w, at.x, at.z);
    w.place_player(Vec3::new(at.x, y, at.z), -0x8000);
    let health = w.save.health;
    let from = sfx_len(&w);
    idle(&mut w, 1);
    // EnMb_ClubWaitPlayerNear: EnMb_SetupClubAttack, Link straight ahead (within 0x258): the
    // middle.
    let m = mb(&w, h);
    assert_eq!(m.func, Func::ClubAttack);
    assert_eq!((m.state, m.attack), (en_mb::ENMB_STATE_ATTACK, en_mb::ENMB_ATTACK_CLUB_MIDDLE));
    // The lift (at 3), the strike (at 1.5), then where the club's head is on the floor: the
    // shockwave's sound, a quake (25, 5 frames), and an En_Part of kind -1 (func_800358DC) at
    // 18 a frame towards Link (the Moblin's world.rot.y, 0), 20 frames, action 2.
    let mut part = None;
    for _ in 0..120 {
        idle(&mut w, 1);
        if let Some(p) = w.actors.all().into_iter().find(|&p| w.actors.downcast::<EnPart>(p).is_some_and(|e| e.actor.parent == Some(h))) {
            part = Some(p);
            break;
        }
    }
    let part = part.expect("no shock wave");
    assert_eq!(mb(&w, h).func, Func::ClubWaitAfterAttack);
    assert!(sfx_since(&w, from, en_mb::NA_SE_EN_MONBLIN_HAM_UP));
    assert!(sfx_since(&w, from, en_mb::NA_SE_EN_MONBLIN_HAM_DOWN));
    assert!(sfx_since(&w, from, en_mb::NA_SE_EN_MONBLIN_HAM_LAND));
    assert!(w.quake.requests.iter().any(|q| q.duration > 0 && q.y == 25), "the quake");
    let e = w.actors.downcast::<EnPart>(part).unwrap();
    assert_eq!((e.actor.params, e.actor.world_rot.y, e.actor.speed_xz), (-1, 0, 18.0));
    assert_eq!(e.actor.scale, Vec3::splat(0.02));
    // effSpawnPos (8000 down the left hand, from the draw) on the floor: in front of the Moblin.
    let eff = mb(&w, h).eff_spawn_pos;
    assert!(eff.distance(CLUB_HOME) < 250.0 && eff.z > CLUB_HOME.z, "effSpawnPos {eff}");
    // The wave reaches Link (within 40, func_80ACE5C8): 8 off (half a heart).
    for _ in 0..25 {
        idle(&mut w, 1);
        if w.save.health != health {
            break;
        }
    }
    assert_eq!(w.save.health, health - 8, "the shock wave's damage");
}

#[test]
fn a_patrol_hit_dies_and_goes() {
    let Some(a) = assets() else { return };
    let mut w = meadow(&a);
    let h = moblin_at(&w, PATROL0_HOME);
    // A sword's hit on its body (the AC flag, ENMB_DMG_REACT_DEFAULT, 1 damage, as the collision
    // check leaves them): EnMb_CheckColliding, 1 - 1 = 0: EnMb_SetupSpearDead (red 12 frames,
    // NA_SE_EN_MORIBLIN_DEAD, not targetable).
    {
        let m = mb_mut(&mut w, h);
        m.body_collider.base.ac_flags |= AC_HIT;
        m.actor.col_chk_info.damage_reaction = en_mb::ENMB_DMG_REACT_DEFAULT;
        m.actor.col_chk_info.damage = 1;
    }
    let from = sfx_len(&w);
    idle(&mut w, 1);
    let m = mb(&w, h);
    assert_eq!(m.func, Func::SpearDead);
    assert_eq!((m.actor.col_chk_info.health, m.state), (0, en_mb::ENMB_STATE_SPEAR_SPEARPATH_DAMAGED));
    assert_eq!(m.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert!(sfx_since(&w, from, en_mb::NA_SE_EN_MORIBLIN_DEAD));
    // Its fall's end, then 30 frames of flames (its shadow gone), then the drop and it's gone.
    let mut n = 0;
    while w.actors.actor(h).is_some_and(|a| !a.killed) {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 200, "still there");
    }
    assert!(n > 30, "gone after {n} frames");
}
