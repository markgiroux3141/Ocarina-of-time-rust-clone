//! GAME-09 milestone 3 against the C: the Lizalfos (`z_en_zf.c`) of Dodongo's Cavern (Master
//! Quest, `ddan`): room 3's two pairs, the lower (0x0500 at (3969, 100, -2161), 0x0501 at (2829,
//! 100, -1961)) and the upper (0x0600 at (4261, 531, -1055), 0x0601 at (4442, 531, -1386)), and
//! room 8's single (0xFFFF at (1558, 16, -3540)) (`ootx scene-info --scene ddan --layer 0`).
//!
//! Child Link enters the cavern on `cavern-open` (the bracelet, the Kokiri Sword); the tests put
//! him where they need him and request the room (`Room_RequestNewRoom`) before its actors init.
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::door_shutter::{Action as DoorAction, DoorShutter};
use oot_actors::en_zf::*;
use oot_game::actor::ACTOR_FLAG_ATTENTION_ENABLED;
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorHandle};
use oot_game::audio::bgm::start_seq;
use oot_game::collision_check::{AC_HIT, MASS_HEAVY};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

const LOWER_A: Vec3 = Vec3::new(3969.0, 100.0, -2161.0);
const LOWER_B: Vec3 = Vec3::new(2829.0, 100.0, -1961.0);
const UPPER_A: Vec3 = Vec3::new(4261.0, 531.0, -1055.0);
const UPPER_B: Vec3 = Vec3::new(4442.0, 531.0, -1386.0);
const SINGLE: Vec3 = Vec3::new(1558.0, 16.0, -3540.0);
/// Link on the lower floor by door 9 (from room 1), on platform 0.
const LINK_LOWER: Vec3 = Vec3::new(3560.0, 100.0, -1450.0);
/// Link on the upper floor by door 10 (from room 10).
const LINK_UPPER: Vec3 = Vec3::new(4300.0, 531.0, -700.0);

/// `NA_SE_EN_GOMA_JR_FREEZE` (`enemybank_table.h`: 0x389E).
const NA_SE_EN_GOMA_JR_FREEZE: u16 = 0x389E;
/// The cavern's music (`ddan_scene`'s sound settings: sequence 0x18).
const NA_BGM_DUNGEON: u16 = 0x18;

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// Child Link on `cavern-open` in the cavern's lobby (`ENTR_DODONGOS_CAVERN_0`), two frames on,
/// `before` run (flags), then Link at `link` facing `yaw` and room `room` requested; two frames
/// on, the change done (the room's actors spawned and their inits run once their objects are
/// in: during those frames, with Link already there).
fn cavern_room(a: &Arc<GameAssets>, room: i8, link: Vec3, yaw: i16, before: impl FnOnce(&mut PlayState)) -> PlayState {
    enter(a, room, link, yaw, before, false)
}

/// `cavern_room` with the music played (the offline audio side from `Play_Init` on: the
/// sequence players' state that `func_800F5ACC` and `func_800F5B58` read).
fn cavern_room_music(a: &Arc<GameAssets>, room: i8, link: Vec3, yaw: i16) -> PlayState {
    enter(a, room, link, yaw, |_| {}, true)
}

fn enter(a: &Arc<GameAssets>, room: i8, link: Vec3, yaw: i16, before: impl FnOnce(&mut PlayState), music: bool) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DODONGOS_CAVERN_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("cavern-open").unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    w.audio.log = Some(Default::default());
    if music {
        w.audio_side = Some(Box::new(oot_game::audio::offline::OfflineAudio::new(&a.pack.audio_data().unwrap(), false)));
        idle(&mut w, 30);
    }
    idle(&mut w, 2);
    before(&mut w);
    w.place_player(link, yaw);
    w.room_request(room);
    idle(&mut w, 2);
    w.room_change_done();
    w.place_player(link, yaw);
    // Until the room's En_Zf are in (object_zf loaded): the frame they init (those killed still
    // in the arena).
    for _ in 0..10 {
        if !zfs(&w).is_empty() {
            break;
        }
        idle(&mut w, 1);
    }
    w
}

fn zfs(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnZf>(h).is_some()).collect()
}

/// The `En_Zf` placed at `home` (killed or not, while it's in the arena).
fn at_home(w: &PlayState, home: Vec3) -> Option<ActorHandle> {
    zfs(w).into_iter().find(|&h| zf(w, h).actor.home_pos == home)
}

fn zf(w: &PlayState, h: ActorHandle) -> &EnZf {
    w.actors.downcast::<EnZf>(h).expect("En_Zf")
}

fn zf_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnZf {
    w.actors.downcast_mut::<EnZf>(h).expect("En_Zf")
}

fn gone(w: &PlayState, h: ActorHandle) -> bool {
    w.actors.get(h).is_none_or(|a| a.base().killed)
}

/// `sPairedActiveFleeFlag`, `sPairedInactiveType`.
fn statics(w: &PlayState) -> Statics {
    w.overlay_statics.get(&ACTOR_EN_ZF).and_then(|s| s.downcast_ref::<Statics>()).copied().unwrap_or_default()
}

fn sfx_frames(w: &PlayState, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, _)| f).collect()
}

fn seq_cmds(w: &PlayState, cmd: u32) -> usize {
    w.audio.log.as_ref().unwrap().seq_cmds.iter().filter(|&&(_, c)| c == cmd).count()
}

fn anim_name(z: &EnZf) -> &str {
    z.skel.animation.as_ref().map(|a| a.name.as_str()).unwrap_or("")
}

/// A hit as `CollisionCheck_AC` leaves it for the next update: the body's `AC_HIT`, the damage
/// table's damage and reaction in `colChkInfo`.
fn hit(w: &mut PlayState, h: ActorHandle, damage: u8, reaction: u8) {
    let z = zf_mut(w, h);
    z.body_collider.base.ac_flags |= AC_HIT;
    z.actor.col_chk_info.damage = damage;
    z.actor.col_chk_info.damage_reaction = reaction;
}

/// The pair's one that may be hit now: in an action up to `ENZF_ACTION_STUNNED`
/// (`EnZf_CheckCollide`), opaque, alive, not the one waiting (`sPairedInactiveType`).
fn hittable(w: &PlayState, h: ActorHandle) -> bool {
    let z = zf(w, h);
    let waiting = is_type_paired(z.actor.params) && statics(w).s_paired_inactive_type == z.actor.params;
    !z.actor.killed && z.action <= ENZF_ACTION_STUNNED && z.alpha == 255 && z.actor.col_chk_info.health > 0 && !waiting
}

/// Link kept where he stands (back at `at` if knocked away) and his hearts full, the Lizalfos's
/// slashes notwithstanding.
fn keep_link(w: &mut PlayState, at: Vec3) {
    w.save.health = 0x30;
    if w.player().actor.world_pos.distance(at) > 30.0 {
        w.place_player(at, -0x8000);
    }
}

/// `sDamageTable` as the C lists it: (damage, reaction) per `DMG_*` bit.
const DAMAGE_TABLE: [(u8, u8); 32] = [
    (0, 1),  // Deku nut: stunned
    (2, 0),  // Deku stick
    (1, 13), // Slingshot: ENZF_DMG_REACT_FLEE
    (2, 0),  // Explosive
    (0, 1),  // Boomerang: stunned
    (2, 0),  // Normal arrow
    (2, 0),  // Hammer swing
    (0, 1),  // Hookshot: stunned
    (1, 0),  // Kokiri sword
    (2, 0),  // Master sword
    (4, 0),  // Giant's Knife
    (2, 13), // Fire arrow
    (4, 15), // Ice arrow: ENZF_DMG_REACT_ICE
    (2, 13), // Light arrow
    (2, 13), // Unk arrow 1
    (2, 13), // Unk arrow 2
    (2, 13), // Unk arrow 3
    (0, 6),  // Fire magic: ENZF_DMG_REACT_6
    (3, 15), // Ice magic
    (0, 6),  // Light magic
    (0, 0),  // Shield
    (0, 0),  // Mirror Ray
    (1, 0),  // Kokiri spin
    (4, 0),  // Giant spin
    (2, 0),  // Master spin
    (2, 0),  // Kokiri jump
    (8, 0),  // Giant jump
    (4, 0),  // Master jump
    (0, 0),  // Unknown 1
    (0, 0),  // Unblockable
    (4, 0),  // Hammer jump
    (0, 0),  // Unknown 2
];

#[test]
fn the_damage_table_is_the_cs() {
    for (i, &(damage, reaction)) in DAMAGE_TABLE.iter().enumerate() {
        // DMG_ENTRY(damage, effect): (effect << 4) | damage.
        assert_eq!(S_DAMAGE_TABLE.table[i], (reaction << 4) | damage, "entry {i}");
    }
}

#[test]
fn the_platform_searches_are_the_cs() {
    // EnZf_FindPlatform: within 150 up or down and 210 across (110 from y 420); the preferred
    // one first, else the last in the table that holds the point.
    assert_eq!(find_platform(Vec3::new(3560.0, 100.0, -1517.0), -1), 0);
    // Between platforms 0 and 5 (each within 210 both ways): 5 from the search, 0 preferred.
    let between = Vec3::new(3750.0, 100.0, -1640.0);
    assert_eq!((find_platform(between, -1), find_platform(between, 0), find_platform(between, 1)), (5, 0, 5));
    // Upstairs (range 110), between 20 (4070, -1575) and 10 (4170, -1395): 20, or 10 preferred.
    let up = Vec3::new(4120.0, 531.0, -1485.0);
    assert_eq!((find_platform(up, -1), find_platform(up, 10)), (20, 10));
    // 300 up is on no floor's platform (100 + 150 < 300 < 531 - 150): -1. 250 is platform 0's
    // edge; at 381 (under 420: range 210) platform 23 atop the column holds it.
    assert_eq!(find_platform(Vec3::new(3560.0, 300.0, -1517.0), -1), -1);
    assert_eq!(find_platform(Vec3::new(3560.0, 250.0, -1517.0), -1), 0);
    assert_eq!(find_platform(Vec3::new(3560.0, 381.0, -1985.0), -1), 23);
    // The pairs' homes (their inits: preferred 0): 4 and 7 below, 15 and 9 above.
    assert_eq!([LOWER_A, LOWER_B, UPPER_A, UPPER_B].map(|p| find_platform(p, 0)), [4, 7, 15, 9]);
    // @bug (game): -1 reads the padding before the table: (0, 0, 0).
    assert_eq!(platform_pos(-1), Vec3::ZERO);

    // EnZf_FindPlatformWithoutPlayer from B's home (cur 7), Link on platform 0: the lower floor's
    // search covers 5 down to 0 only, within 585, not Link's: 2 (422.41) and 1 (392.32). The
    // nearest, 1, lies 60.36° off +z, within 0x36B0 (76.9°) of Link's 55.04°: the second, 2.
    assert_eq!(find_platform_without_player(LOWER_B, 7, 7, LINK_LOWER), 2);
    // From upstairs B's home (cur 9, under 15: 14 down to 8 within 380), Link on 15: 10 (272.15)
    // and 8 (254.61); 8 is at 19.5° against Link's -27.41°: within, so 10.
    assert_eq!(find_platform_without_player(UPPER_B, 9, 9, Vec3::new(4260.0, 531.0, -1035.0)), 10);
    // Nothing within reach (no platform in 585): the fallback.
    assert_eq!(find_platform_without_player(Vec3::new(2000.0, 100.0, -1000.0), -1, 6, LINK_LOWER), 6);

    // EnZf_FindPlatformCloseToPlayer from A's home (cur 4, fallback 4), Link on 0 (762.9 away,
    // beyond 500): of 5 (396.92), 4 (60.54) and 3 (490.93), 5 is the nearest Link (495.13): 5.
    assert_eq!(find_platform_close_to_player(LOWER_A, 4, 4, LINK_LOWER), 5);
    // Next to platform 0, Link on it: the search meets 0 within 500 and stops there: 0.
    assert_eq!(find_platform_close_to_player(Vec3::new(3600.0, 100.0, -1600.0), 0, 0, LINK_LOWER), 0);
}

#[test]
fn a_pair_off_links_level_kills_itself_at_init() {
    let Some(a) = assets() else { return };
    // EnZf_Init: a pair more than 100 above or below Link dies. Link at y 100: the upper pair
    // (431 up) dies, the lower lives.
    let w = cavern_room(&a, 3, LINK_LOWER, -0x8000, |_| {});
    let (la, lb, ua, ub) = (at_home(&w, LOWER_A).unwrap(), at_home(&w, LOWER_B).unwrap(), at_home(&w, UPPER_A), at_home(&w, UPPER_B));
    assert!(ua.is_none_or(|h| zf(&w, h).actor.killed) && ub.is_none_or(|h| zf(&w, h).actor.killed), "the upper pair dies");
    assert!(!zf(&w, la).actor.killed && !zf(&w, lb).actor.killed);
    // 0x0500, 0x0501: switchFlag PARAMS_GET_S(params, 8, 8) 5; the type (0, 1) written over the
    // params; homes found with preferred 0: 4 and 7.
    for (h, ty, plat) in [(la, 0, 4), (lb, 1, 7)] {
        let z = zf(&w, h);
        assert_eq!((z.actor.params, z.switch_flag, z.home_platform, z.cur_platform), (ty, 5, plat, plat));
        // 6 health, NAVI_ENEMY_LIZALFOS, MASS_HEAVY; sInitChain: scale 0.015, gravity -3.5,
        // lockOnArrowOffset 2000; colChkInfo's cylinder 40 by 100; the body's 20 by 70.
        let c = &z.actor.col_chk_info;
        assert_eq!((c.health, c.mass, c.cyl_radius, c.cyl_height, z.actor.navi_enemy_id), (6, MASS_HEAVY, 40, 100, 0x0F));
        assert_eq!((z.actor.scale.x, z.actor.gravity, z.actor.target_arrow_offset), (0.015, -3.5, 2000.0));
        assert_eq!((z.body_collider.dim.radius, z.body_collider.dim.height), (20, 70));
        assert_eq!(z.sword_collider.info.at_dmg_info.damage, 8);
        // EnZf_SetupWaitAppear: 300 above its floor, unseen, not targetable.
        assert_eq!(z.func, Func::WaitAppear);
        assert_eq!((z.actor.world_pos.y, z.alpha), (400.0, 0));
        assert_eq!(z.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    }
    assert_eq!(statics(&w).s_paired_inactive_type, ENZF_TYPE_LIZALFOS_PAIRED_MINIBOSS_B);

    // Link upstairs: the lower pair dies, the upper lives (switch flag 6; homes 15 and 9).
    let w = cavern_room(&a, 3, LINK_UPPER, -0x8000, |_| {});
    let (la, lb) = (at_home(&w, LOWER_A), at_home(&w, LOWER_B));
    assert!(la.is_none_or(|h| zf(&w, h).actor.killed) && lb.is_none_or(|h| zf(&w, h).actor.killed), "the lower pair dies");
    let (ua, ub) = (at_home(&w, UPPER_A).unwrap(), at_home(&w, UPPER_B).unwrap());
    assert_eq!([ua, ub].map(|h| (zf(&w, h).actor.killed, zf(&w, h).switch_flag, zf(&w, h).home_platform)), [(false, 6, 15), (false, 6, 9)]);

    // Link in the air between (y 320: 220 over the lower, 211 under the upper): all four die.
    let w = cavern_room(&a, 3, Vec3::new(3560.0, 320.0, -1450.0), -0x8000, |_| {});
    assert!(zfs(&w).iter().all(|&h| zf(&w, h).actor.killed), "all four die");
}

#[test]
fn its_switch_flag_set_kills_the_pair_at_init() {
    let Some(a) = assets() else { return };
    // EnZf_Init: Flags_GetSwitch(switchFlag): the lower pair's 5 set, it dies with Link on its
    // level.
    let w = cavern_room(&a, 3, LINK_LOWER, -0x8000, |w| w.flags.set_switch(5));
    assert!([LOWER_A, LOWER_B, UPPER_A, UPPER_B].iter().all(|&p| at_home(&w, p).is_none_or(|h| zf(&w, h).actor.killed)));
    // The upper pair's 6 set doesn't touch the lower.
    let w = cavern_room(&a, 3, LINK_LOWER, -0x8000, |w| w.flags.set_switch(6));
    assert!(!zf(&w, at_home(&w, LOWER_A).unwrap()).actor.killed);
}

#[test]
fn the_pair_drops_in_and_a_starts_the_mini_boss_music() {
    let Some(a) = assets() else { return };
    let mut w = cavern_room(&a, 3, LINK_LOWER, -0x8000, |_| {});
    let (la, lb) = (at_home(&w, LOWER_A).unwrap(), at_home(&w, LOWER_B).unwrap());
    let mini_boss = start_seq(0, 0, NA_BGM_MINI_BOSS);
    // EnZf_WaitAppear: timer 10, down 1 a frame (a pair); at 1 NA_SE_EN_RIZA_CRY, targetable,
    // and A alone starts the mini-boss music (func_800F5ACC: SEQ_PLAYER_BGM_MAIN).
    let mut t = zf(&w, la).timer;
    assert!(t > 1, "{t}");
    while t > 1 {
        assert_eq!(seq_cmds(&w, mini_boss), 0);
        idle(&mut w, 1);
        t -= 1;
        assert_eq!(zf(&w, la).timer, t);
    }
    // The frame it reads 1: the cry, the attention, the music; then 0.
    idle(&mut w, 1);
    let z = zf(&w, la);
    assert_eq!(z.timer, 0);
    assert_ne!(z.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!(sfx_frames(&w, NA_SE_EN_RIZA_CRY).iter().filter(|&&f| f == w.audio.frames).count(), 2, "both cry");
    assert_eq!(seq_cmds(&w, mini_boss), 1, "A's start only");
    // Then it falls, 51 more alpha a frame (255 in 5), until it lands: NA_SE_EN_RIZA_ONGND, on
    // its floor, gZfLandOnFeetAnim (to 17).
    for i in 1..=5u32 {
        idle(&mut w, 1);
        assert_eq!(zf(&w, la).alpha as u32, 51 * i);
    }
    let mut n = 0;
    while sfx_frames(&w, NA_SE_EN_RIZA_ONGND).is_empty() {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 40, "never landed");
    }
    let z = zf(&w, la);
    assert_eq!((z.actor.world_pos.y, anim_name(z), z.skel.end_frame), (100.0, "gZfLandOnFeetAnim", 17.0));
    // The landing's end: A fights (EnZf_SetupIdle); B turns away to jump (its sword sheathed once
    // turned).
    for _ in 0..30 {
        idle(&mut w, 1);
    }
    assert_eq!(zf(&w, la).func, Func::Idle);
    let b = zf(&w, lb);
    assert!(matches!(b.func, Func::PairedJumpAwayStartTurnAround | Func::PairedJumpAway | Func::PairedIdleHop), "{:?}", b.func);
    let mut n = 0;
    while zf(&w, lb).func == Func::PairedJumpAwayStartTurnAround {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 60);
    }
    assert!(zf(&w, lb).is_sword_sheathed);
}

#[test]
fn a_hit_hurts_it_and_a_nut_stuns_it() {
    let Some(a) = assets() else { return };
    let mut w = cavern_room(&a, 3, LINK_LOWER, -0x8000, |_| {});
    let la = at_home(&w, LOWER_A).unwrap();
    let mut n = 0;
    while zf(&w, la).func != Func::Idle {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 120);
    }
    // EnZf_CheckCollide: the Kokiri Sword's 1 (reaction 0): NA_SE_EN_RIZA_CRY, red 8 frames, 6 to
    // 5, EnZf_SetupDamaged (gZfDamagedAnim at 1.5, NA_SE_EN_RIZA_DAMAGE).
    hit(&mut w, la, 1, 0);
    idle(&mut w, 1);
    let z = zf(&w, la);
    assert_eq!((z.actor.col_chk_info.health, z.func, z.action, z.last_damage_reaction), (5, Func::Damaged, ENZF_ACTION_DAMAGED, 0));
    assert_eq!((anim_name(z), z.skel.play_speed), ("gZfDamagedAnim", 1.5));
    assert_eq!(sfx_frames(&w, NA_SE_EN_RIZA_DAMAGE), [w.audio.frames]);
    // Actor_SetColorFilter(RED, 255, OPA, 8).
    assert!((6..=8).contains(&z.actor.color_filter_timer), "{}", z.actor.color_filter_timer);
    // Hurt (ENZF_ACTION_DAMAGED, over STUNNED) it can't be hit again.
    hit(&mut w, la, 1, 0);
    idle(&mut w, 1);
    assert_eq!(zf(&w, la).actor.col_chk_info.health, 5);
    // Back in a hittable action: a nut (0, ENZF_DMG_REACT_STUN): blue 80 frames
    // (Actor_SetColorFilter(BLUE, 120, OPA, 80)), EnZf_SetupStunned (gZfDamagedAnim held,
    // NA_SE_EN_GOMA_JR_FREEZE), the health kept.
    let mut n = 0;
    while !hittable(&w, la) {
        keep_link(&mut w, LINK_LOWER);
        idle(&mut w, 1);
        n += 1;
        assert!(n < 200);
    }
    hit(&mut w, la, 0, 1);
    idle(&mut w, 1);
    let z = zf(&w, la);
    assert_eq!((z.actor.col_chk_info.health, z.func, z.action), (5, Func::Stunned, ENZF_ACTION_STUNNED));
    assert_eq!((anim_name(z), z.skel.play_speed), ("gZfDamagedAnim", 0.0));
    assert!((78..=80).contains(&z.actor.color_filter_timer), "{}", z.actor.color_filter_timer);
    assert_eq!(sfx_frames(&w, NA_SE_EN_GOMA_JR_FREEZE), [w.audio.frames]);
    // Stunned, another nut does nothing (the check skips the stun while ENZF_ACTION_STUNNED).
    hit(&mut w, la, 0, 1);
    idle(&mut w, 1);
    assert_eq!(sfx_frames(&w, NA_SE_EN_GOMA_JR_FREEZE).len(), 1);
    // EnZf_Stunned: until the colour's gone (on the ground), then EnZf_PickAction.
    let mut n = 0;
    while zf(&w, la).func == Func::Stunned {
        keep_link(&mut w, LINK_LOWER);
        idle(&mut w, 1);
        n += 1;
        assert!(n < 100);
    }
    assert!((76..=80).contains(&n), "stunned {n} frames");
    assert_eq!(zf(&w, la).actor.color_filter_timer, 0);
}

#[test]
fn the_waiting_one_cant_be_hit() {
    let Some(a) = assets() else { return };
    let mut w = cavern_room(&a, 3, LINK_LOWER, -0x8000, |_| {});
    let lb = at_home(&w, LOWER_B).unwrap();
    idle(&mut w, 60);
    // B is sPairedInactiveType: EnZf_CheckCollide clears its AC_HIT and does nothing (and
    // EnZf_Update never sets its AC).
    assert_eq!(statics(&w).s_paired_inactive_type, 1);
    hit(&mut w, lb, 1, 0);
    idle(&mut w, 1);
    let z = zf(&w, lb);
    assert_eq!(z.actor.col_chk_info.health, 6);
    assert_ne!(z.func, Func::Damaged);
    // (Hopping, ENZF_ACTION_PAIRED_IDLE_HOP is over ENZF_ACTION_STUNNED: the check doesn't even
    // clear its AC_HIT. In the game nothing sets it: its AC isn't registered.)
    assert!(z.action > ENZF_ACTION_STUNNED);
}

/// What a pair's fight did, by the simulation below.
#[derive(Debug, Default)]
#[allow(dead_code)]
struct Fight {
    frames: u32,
    hits: Vec<(u32, &'static str, i8)>,
    /// (frame, who) of each flight (`EnZf_Paired_JumpAwayStartTurnAround` begun), engage
    /// (`EnZf_Paired_EngageCombat` begun) and death.
    flights: Vec<(u32, &'static str)>,
    engages: Vec<(u32, &'static str)>,
    deaths: Vec<(u32, &'static str)>,
    /// Each one's platforms, in order (`curPlatform` changes), and the jump-away targets.
    platforms: [Vec<i16>; 2],
    targets: [Vec<i16>; 2],
    /// The lowest each stood.
    min_y: [f32; 2],
    flag_at: Option<u32>,
}

/// The pair hit with the Kokiri Sword (1, reaction 0) whenever one may be, as fast as can be, Link
/// standing at `link` (kept there, his hearts full), until both are gone; the frames and what
/// happened.
fn fight(w: &mut PlayState, pair: [ActorHandle; 2], link: Vec3, flag: i32, max: u32) -> Fight {
    let names = ["A", "B"];
    let mut f = Fight { min_y: [f32::MAX; 2], ..Default::default() };
    let mut prev_func = [Func::WaitAppear; 2];
    for frame in 0..max {
        keep_link(w, link);
        for (i, &h) in pair.iter().enumerate() {
            if !gone(w, h) && hittable(w, h) {
                hit(w, h, 1, 0);
                let hp = zf(w, h).actor.col_chk_info.health as i8 - 1;
                f.hits.push((frame, names[i], hp));
            }
        }
        idle(w, 1);
        f.frames = frame + 1;
        for (i, &h) in pair.iter().enumerate() {
            let Some(z) = w.actors.downcast::<EnZf>(h) else { continue };
            if z.func != prev_func[i] {
                match z.func {
                    Func::PairedJumpAwayStartTurnAround => f.flights.push((frame, names[i])),
                    Func::PairedEngageCombat => f.engages.push((frame, names[i])),
                    Func::Die => f.deaths.push((frame, names[i])),
                    Func::PairedJumpAway => f.targets[i].push(z.target_platform),
                    _ => {}
                }
                prev_func[i] = z.func;
            }
            if f.platforms[i].last() != Some(&z.cur_platform) {
                f.platforms[i].push(z.cur_platform);
            }
            if !z.actor.killed && z.actor.category != ACTORCAT_PROP {
                f.min_y[i] = f.min_y[i].min(z.actor.world_pos.y);
            }
        }
        if f.flag_at.is_none() && w.flags.get_switch(flag) {
            f.flag_at = Some(frame);
        }
        if pair.iter().all(|&h| gone(w, h)) {
            break;
        }
    }
    f
}

fn door(w: &PlayState, index: usize) -> &DoorShutter {
    let h = w.actors.all().into_iter().find(|&h| w.actors.downcast::<DoorShutter>(h).is_some_and(|d| d.transition_index() == index)).expect("the door");
    w.actors.downcast::<DoorShutter>(h).unwrap()
}

#[test]
fn the_lower_pairs_fight_to_the_second_death_opens_doors_8_and_9() {
    let Some(a) = assets() else { return };
    let mut w = cavern_room_music(&a, 3, LINK_LOWER, -0x8000);
    let pair = [at_home(&w, LOWER_A).unwrap(), at_home(&w, LOWER_B).unwrap()];
    // Doors 8 (to room 4) and 9 (to room 1): SHUTTER_FRONT_SWITCH (0x0085: type 2, switch flag
    // 5), barred.
    idle(&mut w, 1);
    for d in [8, 9] {
        assert_eq!((door(&w, d).action, door(&w, d).bars_closed_amount), (DoorAction::BarAndWaitSwitchFlag, 1.0), "door {d}");
    }
    let f = fight(&mut w, pair, LINK_LOWER, 5, 6000);
    println!("{f:#?}");
    // Twelve hits: A 6 to 3 (from 4 to under 4: ENZF_DMG_REACT_FLEE), flees once hurt; B (90
    // frames after the signal) 6 to 3, flees; A back, 3 to 0, dies (B's health kept at 3 or
    // more); B back, 3 to 0.
    let order: Vec<(&str, i8)> = f.hits.iter().map(|&(_, n, hp)| (n, hp)).collect();
    assert_eq!(order, [("A", 5), ("A", 4), ("A", 3), ("B", 5), ("B", 4), ("B", 3), ("A", 2), ("A", 1), ("A", 0), ("B", 2), ("B", 1), ("B", 0)]);
    // B's first flight is its drop-in's; then A's, B's.
    let flights: Vec<&str> = f.flights.iter().map(|&(_, n)| n).collect();
    assert_eq!(flights, ["B", "A", "B"]);
    // B engages at A's flight, A at B's, B at A's death.
    let engages: Vec<&str> = f.engages.iter().map(|&(_, n)| n).collect();
    assert_eq!(engages, ["B", "A", "B"]);
    // Each engage 89 frames or more after the signal (timer2 90, counted down to 1 on the ground).
    let signals = [f.flights[1].0, f.flights[2].0, f.deaths[0].0];
    for (&s, &(en, _)) in signals.iter().zip(&f.engages) {
        assert!(en >= s + 89, "engaged {en}, signalled {s}");
    }
    // The first death sets no flag; the second's animation end does (Flags_SetSwitch(5)), with
    // the music back (func_800F5B58).
    let deaths: Vec<&str> = f.deaths.iter().map(|&(_, n)| n).collect();
    assert_eq!(deaths, ["A", "B"]);
    let flag_at = f.flag_at.expect("switch flag 5 set");
    assert!(flag_at > f.deaths[1].0);
    assert!(pair.iter().all(|&h| gone(&w, h)));
    // The lower floor's platforms only, no one below its platforms' 100.
    for i in 0..2 {
        assert!(f.targets[i].iter().all(|&t| (0..=7).contains(&t)), "{:?}", f.targets[i]);
        assert!(f.min_y[i] >= 99.0, "{}", f.min_y[i]);
    }
    // func_800F5B58 queued the cavern's music back (SEQ_PLAYER_BGM_MAIN's before the mini-boss's).
    assert!(seq_cmds(&w, start_seq(0, 0, NA_BGM_DUNGEON)) >= 1);
    // The doors unbar (DoorShutter_Unbar, then DoorShutter_UnbarredCheckSwitchFlag).
    idle(&mut w, 60);
    for d in [8, 9] {
        assert_eq!((door(&w, d).action, door(&w, d).bars_closed_amount), (DoorAction::UnbarredCheckSwitchFlag, 0.0), "door {d}");
    }
}

#[test]
fn the_upper_pair_keeps_to_the_upper_floor() {
    let Some(a) = assets() else { return };
    let mut w = cavern_room(&a, 3, LINK_UPPER, -0x8000, |_| {});
    let pair = [at_home(&w, UPPER_A).unwrap(), at_home(&w, UPPER_B).unwrap()];
    let f = fight(&mut w, pair, LINK_UPPER, 6, 6000);
    println!("{f:#?}");
    assert_eq!(f.hits.len(), 12);
    assert!(f.flag_at.is_some(), "switch flag 6 set");
    // Above 420 the searches cover the upper floor's platforms (8 to 23) only; no one falls to
    // the lower floor.
    for i in 0..2 {
        assert!(f.targets[i].iter().all(|&t| (8..=23).contains(&t)), "{:?}", f.targets[i]);
        assert!(f.min_y[i] > 420.0, "{}", f.min_y[i]);
    }
}

#[test]
fn the_trail_follows_the_sword_while_it_swings() {
    let Some(a) = assets() else { return };
    let mut w = cavern_room(&a, 3, LINK_LOWER, -0x8000, |_| {});
    let la = at_home(&w, LOWER_A).unwrap();
    // A comes to Link and slashes (EnZf_SetupAttack: gZfSlashAnim at 1.25).
    let mut n = 0;
    while zf(&w, la).func != Func::Attack {
        keep_link(&mut w, LINK_LOWER);
        idle(&mut w, 1);
        n += 1;
        assert!(n < 1500, "it never slashed");
    }
    // EnZf_PostLimbDraw, each frame of the slash: before frame 14 a gap (EffectBlure_AddSpace),
    // to 20 an edge (EffectBlure_AddVertex) from the sword's limb's (300, -1700, 0) to its
    // (-600, 300, 0). The sword's quad comes off the same matrix: (0, 1500, 0) is quad[1],
    // (-600, -3000, ±1000) quad[0] and [3], (1500, -3000, 0) quad[2]; so the edge's ends are
    // (91 q1 + 115 (q0 + q3) / 2 + 109 q2) / 315 and (77 q1 + 50 (q0 + q3) / 2 - 22 q2) / 105.
    let (mut gaps, mut edges) = (0, 0);
    let idx = zf(&w, la).sword_blure_effect_index;
    loop {
        idle(&mut w, 1);
        let z = zf(&w, la);
        if z.func != Func::Attack {
            break;
        }
        let frame = z.skel.cur_frame;
        let q = z.sword_collider.dim.quad;
        let b = w.effect_ctx.blures[idx - oot_game::effect::SPARK_COUNT].as_ref().expect("the trail");
        // The element this frame's draw added, the last (EffectBlure_Update drops a leading gap
        // and the expired before the draw).
        let last = || b.elements[b.num_elements.checked_sub(1).expect("an element this frame") as usize];
        if frame < 14.0 {
            let e = last();
            assert_eq!(e.state, 0, "a gap at frame {frame}");
            gaps += 1;
        } else if frame < 20.0 {
            let e = last();
            assert_eq!(e.state, 1, "an edge at frame {frame}");
            let mid = (q[0] + q[3]) * 0.5;
            let p1 = (q[1] * 91.0 + mid * 115.0 + q[2] * 109.0) / 315.0;
            let p2 = (q[1] * 77.0 + mid * 50.0 - q[2] * 22.0) / 105.0;
            let v = |p: [i16; 3]| Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32);
            assert!(v(e.p1).distance(p1) < 2.0, "p1 {:?} against {p1}", e.p1);
            assert!(v(e.p2).distance(p2) < 2.0, "p2 {:?} against {p2}", e.p2);
            edges += 1;
        }
    }
    // At 1.25 a frame: gaps at 1.25 to 13.75, edges at 15, 16.25, 17.5 and 18.75.
    assert_eq!((gaps, edges), (11, 4));
}

#[test]
fn the_single_waits_unseen_until_link_is_within_160() {
    let Some(a) = assets() else { return };
    // Room 8 from the ledge north of its pit (y 76), 240 from the single.
    let start = Vec3::new(1558.0, 76.0, -3300.0);
    let mut w = cavern_room(&a, 8, start, -0x8000, |_| {});
    let h = at_home(&w, SINGLE).expect("the single");
    // ENZF_TYPE_LIZALFOS_SINGLE (0xFFFF: -1, switch flag 0xFF): no platform, and
    // sPairedInactiveType ENZF_TYPE_LIZALFOS_SINGLE; EnZf_SetupWaitAppear, 300 over its floor
    // (16), unseen.
    let z = zf(&w, h);
    assert_eq!((z.actor.params, z.switch_flag, z.cur_platform, z.home_platform), (-1, 0xFF, -1, -1));
    assert_eq!(statics(&w).s_paired_inactive_type, ENZF_TYPE_LIZALFOS_SINGLE);
    idle(&mut w, 60);
    // EnZf_WaitAppear: the timer stays 10 while Link is over 160 across.
    let z = zf(&w, h);
    assert_eq!((z.func, z.timer, z.alpha, z.actor.world_pos.y), (Func::WaitAppear, 10, 0, 316.0));
    assert_eq!(z.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    let cries = sfx_frames(&w, NA_SE_EN_RIZA_CRY).len();
    // Within 160 (159 across, 60 up: the height isn't read): timer 0, targetable,
    // NA_SE_EN_RIZA_CRY.
    w.place_player(SINGLE + Vec3::new(0.0, 60.0, 159.0), -0x8000);
    idle(&mut w, 1);
    let z = zf(&w, h);
    assert_eq!(z.timer, 0);
    assert_ne!(z.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!(sfx_frames(&w, NA_SE_EN_RIZA_CRY).len(), cries + 1);
    // It falls, 51 alpha a frame, lands (NA_SE_EN_RIZA_ONGND) and once its landing's done
    // stands (EnZf_SetupIdle).
    let mut n = 0;
    while sfx_frames(&w, NA_SE_EN_RIZA_ONGND).is_empty() {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 40, "never landed");
    }
    assert_eq!((zf(&w, h).actor.world_pos.y, zf(&w, h).alpha), (16.0, 255));
    let mut n = 0;
    while zf(&w, h).func == Func::WaitAppear {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 40);
    }
    assert_eq!(zf(&w, h).func, Func::Idle);
}

#[test]
fn the_single_drops_in_on_links_way_from_room_7_to_the_fire_walls() {
    let Some(a) = assets() else { return };
    // Link in by En_Holl 3 (620, 76, -2784) from room 7, along the floor's way to the fire walls
    // (Bg_Hidan_Curtain at (1655..1770, 16, -3690)): east along the y 76 corridor, down the ramp
    // to the ledge over the pit, down into the pit (y 16) at its west end and across it. Placed
    // 10 a frame along the way, on the floor under him.
    let way = [
        Vec3::new(640.0, 0.0, -2784.0),
        Vec3::new(1000.0, 0.0, -2784.0),
        Vec3::new(1300.0, 0.0, -2830.0),
        Vec3::new(1520.0, 0.0, -2900.0),
        Vec3::new(1575.0, 0.0, -3000.0),
        Vec3::new(1575.0, 0.0, -3250.0),
        Vec3::new(1400.0, 0.0, -3300.0),
        Vec3::new(1400.0, 0.0, -3450.0),
        Vec3::new(1712.0, 0.0, -3650.0),
    ];
    let mut w = cavern_room(&a, 8, Vec3::new(640.0, 76.0, -2784.0), 0x4000, |_| {});
    let h = at_home(&w, SINGLE).expect("the single");
    let mut seen: Option<(Vec3, f32)> = None;
    for leg in way.windows(2) {
        let (from, to) = (leg[0], leg[1]);
        let steps = (from.distance(to) / 10.0).ceil() as usize;
        for s in 0..steps {
            let mut p = from.lerp(to, s as f32 / steps as f32);
            let (floor, _) = w.col.entity_raycast_down(Vec3::new(p.x, 250.0, p.z));
            assert!(floor > -1000.0, "no floor at {p}");
            p.y = floor;
            w.place_player(p, eng_math::vec3f_yaw(from, to));
            idle(&mut w, 1);
            assert!(w.actors.get(h).is_some(), "the single gone with Link at {p}, room {}", w.room_ctx.cur.num);
            let z = zf(&w, h);
            if seen.is_none() && z.timer == 0 {
                seen = Some((p, z.actor.xz_dist_to_player));
            }
        }
    }
    let (p, d) = seen.expect("it dropped in on the way");
    println!("it dropped in with Link at {p} ({d} across)");
    // It drops in the frame Link is first within 160 across (10 a frame).
    assert!(d <= 160.0 && d > 145.0, "{d}");
    // By the walls it's down and fighting.
    idle(&mut w, 60);
    let z = zf(&w, h);
    assert!(z.func != Func::WaitAppear && z.alpha == 255, "{:?}", z.func);
}

#[test]
fn leaving_the_room_mid_fight_puts_the_music_back() {
    let Some(a) = assets() else { return };
    let mut w = cavern_room_music(&a, 3, LINK_LOWER, -0x8000);
    idle(&mut w, 40);
    assert_eq!(seq_cmds(&w, start_seq(0, 0, NA_BGM_MINI_BOSS)), 1);
    let before = seq_cmds(&w, start_seq(0, 0, NA_BGM_DUNGEON));
    // Room 1 (through door 9): room 3's actors deleted. EnZf_Destroy: the first finds the other
    // still in the list (Actor_FindNearby, 10000); the last finds none: func_800F5B58.
    w.place_player(Vec3::new(3560.0, 100.0, -1200.0), 0);
    w.room_request(1);
    idle(&mut w, 2);
    w.room_change_done();
    idle(&mut w, 2);
    assert!(zfs(&w).is_empty());
    assert_eq!(seq_cmds(&w, start_seq(0, 0, NA_BGM_DUNGEON)), before + 1);
    // The overlay's statics go with its last instance (its reload's .data).
    assert!(!w.overlay_statics.contains_key(&ACTOR_EN_ZF));
}


