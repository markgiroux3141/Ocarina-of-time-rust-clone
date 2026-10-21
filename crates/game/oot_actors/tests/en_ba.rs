//! GAME-10 milestone 5 against the C: the parasitic tentacles (`z_en_ba.c`) of Jabu-Jabu's Belly,
//! as Master Quest places them (`ootx scene-info --scene bdan_scene`): red 0x0B00 in room 8 at
//! (1, 20, -5801), gray 0x0A02 in room 9 at (1561, 20, -4679), green 0x0C01 in room 10 at (-1562,
//! 20, -4683).
//!
//! Child Link enters at `ENTR_JABU_JABU_0` on a `jabu-boomerang` save, and the room changes to the
//! one tested (`Room_RequestNewRoom`). The tests run the tentacle's update alone
//! (`Actor_UpdateAll`'s call) with Link's distance set as `Actor_UpdateAll` would; the hits go
//! through the collision check (`CollisionCheck_AT`) from a boomerang's quad (`En_Boom`) laid
//! across the tentacle's hit sphere. No fight is scripted and nothing waits on `Rand`. Expected
//! values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::en_ba::*;
use oot_actors::en_boom::{ACTOR_EN_BOOM, EnBoom};
use oot_actors::obj_roomtimer::ObjRoomtimer;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ActorHandle};
use oot_game::audio::sfx::SfxPos;
use oot_game::collision_check::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::shadow::ShadowDraw;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// Child Link in Jabu-Jabu (`jabu-boomerang`), the sound log on, `flags` set, in room `room`, its
/// actors in.
fn jabu(a: &Arc<GameAssets>, room: i8, flags: &[i32]) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_JABU_JABU_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("jabu-boomerang").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    for &f in flags {
        w.flags.set_switch(f);
    }
    assert!(w.room_request(room));
    idle(&mut w, 1);
    w.room_change_done();
    assert_eq!(w.room_ctx.cur.num, room);
    idle(&mut w, 20);
    w
}

const RED_HOME: Vec3 = Vec3::new(1.0, 20.0, -5801.0);
const GRAY_HOME: Vec3 = Vec3::new(1561.0, 20.0, -4679.0);
const GREEN_HOME: Vec3 = Vec3::new(-1562.0, 20.0, -4683.0);

fn tentacles(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnBa>(h).is_some_and(|b| !b.actor.killed && b.ty() < EN_BA_DEAD_BLOB)).collect()
}

fn blobs(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnBa>(h).is_some_and(|b| !b.actor.killed && b.ty() == EN_BA_DEAD_BLOB)).collect()
}

fn ba(w: &PlayState, h: ActorHandle) -> &EnBa {
    w.actors.downcast::<EnBa>(h).expect("En_Ba")
}

fn ba_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnBa {
    w.actors.downcast_mut::<EnBa>(h).expect("En_Ba")
}

fn gone(w: &PlayState, h: ActorHandle) -> bool {
    w.actors.downcast::<EnBa>(h).is_none_or(|b| b.actor.killed)
}

/// The actor's update alone (`Actor_UpdateAll`'s call), as the updating actor.
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// Its draw's changes (`EnBa_Draw`'s `Collider_UpdateSpheres`).
fn draw_update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.draw_update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// The times a sound was asked for at `h`.
fn sfx(w: &PlayState, h: ActorHandle, id: u16) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, p)| s == id && p == SfxPos::Actor(h)).count()
}

/// A boomerang (`En_Boom`) whose quad, hurting with `dmg_flags`, crosses the tentacle's hit sphere
/// (a 60-wide square through its centre), through `CollisionCheck_AT` with the tentacle's AC as its
/// update registers it. Whether the tentacle took the hit (`AC_HIT`).
fn hit_with(w: &mut PlayState, h: ActorHandle, dmg_flags: u32) -> bool {
    w.col_chk.clear();
    draw_update(w, h);
    let c = ba(w, h).collider.elements[0].dim.world_sphere.center;
    let c = Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32);
    {
        let mut a = w.actors.take(h).unwrap();
        let b = a.as_any_mut().downcast_mut::<EnBa>().unwrap();
        w.col_chk.set_ac(Some(h), &b.actor, 0, &mut b.collider);
        w.actors.put_back(h, a);
    }
    let boom = w.actor_spawn(ACTOR_EN_BOOM, c, [0, 0, 0], 0).expect("En_Boom");
    {
        let mut a = w.actors.take(boom).unwrap();
        let b = a.as_any_mut().downcast_mut::<EnBoom>().unwrap();
        b.collider.set_vertices(c + Vec3::new(-30.0, -30.0, 0.0), c + Vec3::new(30.0, -30.0, 0.0), c + Vec3::new(-30.0, 30.0, 0.0), c + Vec3::new(30.0, 30.0, 0.0));
        b.collider.info.at_dmg_info.dmg_flags = dmg_flags;
        w.col_chk.set_at(Some(boom), &b.actor, 0, &mut b.collider);
        w.actors.put_back(boom, a);
    }
    let _ = w.col_chk.check(&mut w.actors);
    w.col_chk.clear();
    w.delete_actor(boom);
    ba(w, h).collider.base.ac_flags & AC_HIT != 0
}

/// The distance from each segment to the next (and from the base to the first): the chain's 32.
fn assert_chain(b: &EnBa) {
    assert!((b.unk_158[0].distance(b.actor.world_pos) - 32.0).abs() < 0.01);
    for i in 0..13 {
        assert!((b.unk_158[i + 1].distance(b.unk_158[i]) - 32.0).abs() < 0.01, "segment {i}");
    }
}

#[test]
fn the_three_tentacles_hang_in_rooms_8_9_and_10() {
    let Some(a) = assets() else { return };
    for (room, ty, flag, home, yaw) in [(8, EN_BA_TENTACLE_RED, 0x0B, RED_HOME, 0), (9, EN_BA_TENTACLE_GRAY, 0x0A, GRAY_HOME, -16384), (10, EN_BA_TENTACLE_GREEN, 0x0C, GREEN_HOME, 16384)] {
        let w = jabu(&a, room, &[]);
        let hs = tentacles(&w);
        assert_eq!(hs.len(), 1, "room {room}");
        let b = ba(&w, hs[0]);
        // EnBa_Init: upperParams PARAMS_GET_U(params, 8, 8), params &= 0xFF (the type); 100 over its
        // home; the circle shadow 48; scale 0.01; health 4, MASS_HEAVY; sInitChain's
        // NAVI_ENEMY_PARASITIC_TENTACLE, ATTENTION_RANGE_4; EnBa_SetupIdle: unk_14C 2, 1500, speed 10.
        assert_eq!((b.actor.params, b.upper_params as i32, b.actor.home_pos, b.actor.home_rot.y), (ty, flag, home, yaw), "room {room}");
        assert_eq!(b.actor.world_pos.y, home.y + 100.0);
        assert_eq!((b.actor.shape_shadow_draw, b.actor.shape_shadow_scale), (ShadowDraw::Circle, 48.0));
        assert_eq!(b.actor.scale, Vec3::splat(0.01));
        assert_eq!((b.actor.col_chk_info.health, b.actor.col_chk_info.mass), (4, MASS_HEAVY));
        assert_eq!((b.actor.navi_enemy_id, b.actor.target_mode), (NAVI_ENEMY_PARASITIC_TENTACLE, 4));
        assert_eq!((b.action, b.unk_14c, b.unk_31c, b.actor.speed_xz), (Action::Idle, 2, 1500, 10.0));
        assert_ne!(b.actor.flags & (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED), 0);
        // sJntSphElementsInit: the hit sphere (limb 8, 20, the boomerang's 0x10) and the tip's (limb
        // 13, 25, hurting 4).
        let e = &b.collider.elements;
        assert_eq!((e[0].dim.limb, e[0].dim.model_sphere.radius, e[0].info.ac_dmg_info.dmg_flags), (8, 20, DMG_BOOMERANG));
        assert_eq!((e[1].dim.limb, e[1].dim.model_sphere.radius, e[1].info.at_dmg_info.damage), (13, 25, 4));
        // The chain hangs from the base, 32 a segment.
        assert_chain(b);
        // Its Obj_Roomtimer shares its flag, no time; a prop after its first update.
        let t = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<ObjRoomtimer>(h)).expect("Obj_Roomtimer");
        assert_eq!((t.switch_flag, t.time(), t.actor.category), (flag, 0x3FF, oot_game::actor_ctx::ACTORCAT_PROP));
        assert!(!w.flags.get_switch(flag));
    }
}

#[test]
fn with_its_flag_set_the_tentacle_isnt_there() {
    let Some(a) = assets() else { return };
    // EnBa_Init: Flags_GetSwitch(upperParams): Actor_Kill.
    let w = jabu(&a, 8, &[0x0B]);
    assert!(tentacles(&w).is_empty());
}

#[test]
fn a_sword_passes_through_it_and_four_boomerang_hits_kill_it() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a, 8, &[]);
    let h = tentacles(&w)[0];
    ba_mut(&mut w, h).actor.xz_dist_to_player = 1000.0;
    // Its hit sphere (EnBa_Draw: Collider_UpdateSpheres(8) under the 7th segment's matrix) at the
    // 7th segment, 20 across.
    draw_update(&mut w, h);
    let b = ba(&w, h);
    let s = b.collider.elements[0].dim.world_sphere;
    let p = b.unk_158[6];
    assert_eq!((s.center, s.radius), ([p.x as i16, p.y as i16, p.z as i16], 20));
    // A sword's slash (DMG_SLASH_KOKIRI, the spin's, the jump's) isn't in its 0x10: no hit.
    for flags in [DMG_SLASH_KOKIRI, DMG_SPIN_KOKIRI, DMG_JUMP_KOKIRI, DMG_DEKU_STICK, DMG_SLINGSHOT] {
        assert!(!hit_with(&mut w, h, flags), "{flags:#x}");
    }
    // A boomerang's (0x10) hits.
    let y_sfx = |w: &PlayState| sfx(w, h, NA_SE_EN_BALINADE_HAND_DAMAGE);
    for n in 1..=3u8 {
        assert!(hit_with(&mut w, h, DMG_BOOMERANG), "hit {n}");
        update(&mut w, h);
        // EnBa_Update: health down; func_809B7174: unk_14C 1, 1500, unk_318 20, MASS_IMMOVABLE, speed
        // 10, NA_SE_EN_BALINADE_HAND_DAMAGE, red 12 frames; then EnBa_RecoilFromDamage's first run
        // (unk_318 19).
        let b = ba(&w, h);
        assert_eq!(b.actor.col_chk_info.health, 4 - n);
        assert_eq!((b.action, b.unk_14c, b.unk_318, b.actor.col_chk_info.mass, b.actor.speed_xz), (Action::RecoilFromDamage, 1, 19, MASS_IMMOVABLE, 10.0));
        assert_eq!(b.actor.color_filter_timer, 12);
        assert_ne!(b.actor.color_filter_params & COLORFILTER_COLORFLAG_RED, 0);
        assert_eq!(y_sfx(&w), n as usize);
        assert_eq!(b.collider.base.ac_flags & AC_HIT, 0);
        // Recoiling it isn't hittable (unk_14C < 2: no CollisionCheck_SetAC), the base rising 30 a
        // frame to 330 over home; 19 more frames and it hangs again.
        w.col_chk.clear();
        let y0 = ba(&w, h).actor.world_pos.y;
        update(&mut w, h);
        assert!(!w.col_chk.col_ac.iter().any(|r| r.actor == h));
        assert_eq!(ba(&w, h).actor.world_pos.y, (y0 + 30.0).min(RED_HOME.y + 330.0));
        for _ in 0..18 {
            update(&mut w, h);
        }
        assert_eq!((ba(&w, h).action, ba(&w, h).unk_14c), (Action::Idle, 2));
        assert_chain(ba(&w, h));
    }
    // The fourth: func_809B75A0: NA_SE_EN_BALINADE_HAND_DEAD (EffectSsDeadSound, 40 frames), seven
    // blobs at segments 7 to 13, lock-on off, unk_31A 15; EnBa_Die's first run (14, 8 up).
    assert!(blobs(&w).is_empty());
    let seg: Vec<Vec3> = ba(&w, h).unk_158[7..14].to_vec();
    assert!(hit_with(&mut w, h, DMG_BOOMERANG), "hit 4");
    let y0 = ba(&w, h).actor.world_pos.y;
    update(&mut w, h);
    let b = ba(&w, h);
    assert_eq!((b.actor.col_chk_info.health, b.action, b.unk_14c, b.unk_31a), (0, Action::Die, 0, 14));
    assert_eq!(b.actor.world_pos.y, y0 + 8.0);
    assert_eq!(b.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    let bl = blobs(&w);
    assert_eq!(bl.len(), 7);
    for (i, &bh) in bl.iter().enumerate() {
        // EnBa_Init for EN_BA_DEAD_BLOB: scale 0.021; EnBa_SetupFallAsBlob: unk_318 20, gravity -2,
        // up to 4 out (Rand_CenteredFloat(8)).
        let o = ba(&w, bh);
        assert!(seg.iter().any(|&p| p == o.actor.home_pos), "blob {i} at {}", o.actor.home_pos);
        assert_eq!((o.action, o.actor.scale, o.unk_318, o.actor.gravity), (Action::FallAsBlob, Vec3::splat(0.021), 20, -2.0));
        assert!(o.actor.speed_xz.abs() <= 4.0);
        assert_eq!(o.actor.category, ACTORCAT_ENEMY);
    }
    // EnBa_Die: 14 more frames up 8 a frame; then its switch flag (0x0B) and gone.
    for n in 0..14 {
        update(&mut w, h);
        assert!(!w.flags.get_switch(0x0B), "frame {n}");
    }
    assert_eq!(ba(&w, h).actor.world_pos.y, y0 + 8.0 * 15.0);
    update(&mut w, h);
    assert!(w.flags.get_switch(0x0B));
    assert!(gone(&w, h));
}

#[test]
fn it_swings_at_link_within_175_and_knocks_him_back() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a, 8, &[]);
    let h = tentacles(&w)[0];
    let near = |w: &mut PlayState, d: f32| {
        let b = ba_mut(w, h);
        b.actor.xz_dist_to_player = d;
        b.actor.yaw_towards_player = 0x2000;
    };
    // Hanging at its 100 with Link 175 across: EnBa_SetupSwingAtPlayer: unk_14C 3, unk_318 20,
    // unk_31A 0, 1500, MASS_IMMOVABLE, speed 20.
    near(&mut w, 175.0);
    update(&mut w, h);
    let b = ba(&w, h);
    assert_eq!((b.action, b.unk_14c, b.unk_318, b.unk_31a, b.unk_31c), (Action::SwingAtPlayer, 3, 20, 0, 1500));
    assert_eq!((b.actor.col_chk_info.mass, b.actor.speed_xz), (MASS_IMMOVABLE, 20.0));
    // The wind-up: NA_SE_EN_BALINADE_HAND_UP at its start, 20 frames (unk_31A held at 10), the base
    // down to 60 over home (10 a frame), its tip's sphere hurting (CollisionCheck_SetAT).
    for n in 0..20 {
        w.col_chk.clear();
        update(&mut w, h);
        let b = ba(&w, h);
        assert_eq!((b.unk_318, b.unk_31a), (19 - n, 10), "wind-up {n}");
        assert!(w.col_chk.col_at.iter().any(|r| r.actor == h));
    }
    assert_eq!(sfx(&w, h, NA_SE_EN_BALINADE_HAND_UP), 1);
    assert_eq!(ba(&w, h).actor.world_pos.y, RED_HOME.y + 60.0);
    // The swing: NA_SE_EN_BALINADE_HAND_DOWN, 8000 a frame, speed 30, 10 frames.
    for n in 0..10 {
        update(&mut w, h);
        let b = ba(&w, h);
        assert_eq!((b.unk_31a, b.unk_31c, b.actor.speed_xz), (9 - n, 8000, 30.0), "swing {n}");
    }
    assert_eq!(sfx(&w, h, NA_SE_EN_BALINADE_HAND_DOWN), 1);
    assert_chain(ba(&w, h));
    // Link still within 175: again, 27 frames' wind-up at 750.
    update(&mut w, h);
    let b = ba(&w, h);
    assert_eq!((b.action, b.unk_318, b.unk_31a, b.unk_31c), (Action::SwingAtPlayer, 27, 0, 750));
    // Its AT touching Link: Actor_SetPlayerKnockbackLargeNoDamage(8 out along yawTowardsPlayer, 8
    // up), AT_HIT cleared.
    let link = w.player;
    {
        let b = ba_mut(&mut w, h);
        b.collider.base.at_flags |= AT_HIT;
        b.collider.base.at = link;
    }
    update(&mut w, h);
    let p = w.actors.downcast::<oot_actors::player::Player>(link.unwrap()).unwrap();
    assert_eq!((p.knockback_type, p.knockback_damage, p.knockback_speed, p.knockback_y_velocity, p.knockback_rot), (2, 0, 8.0, 8.0, 0x2000));
    assert_eq!(ba(&w, h).collider.base.at_flags & AT_HIT, 0);
    // Link over 175 across at a wind-up's end (unk_31A 0): hanging again (EnBa_SetupIdle).
    {
        let b = ba_mut(&mut w, h);
        b.unk_318 = 0;
        b.unk_31a = 0;
    }
    near(&mut w, 300.0);
    update(&mut w, h);
    let b = ba(&w, h);
    assert_eq!((b.action, b.unk_14c, b.unk_31c, b.actor.speed_xz), (Action::Idle, 2, 1500, 10.0));
    // Hanging with MASS_IMMOVABLE (it has swung) and Link over 175: the base eases up to 330 over
    // home, 7 a frame.
    let y0 = ba(&w, h).actor.world_pos.y;
    update(&mut w, h);
    assert_eq!(ba(&w, h).actor.world_pos.y, y0 + 7.0);
}

#[test]
fn a_dead_tentacles_blobs_fall_squash_and_go_and_the_room_is_cleared() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a, 8, &[]);
    let h = tentacles(&w)[0];
    // Its last hit (func_809B75A0) as the update runs it.
    {
        let mut act = w.actors.take(h).unwrap();
        w.cur_actor = Some(h);
        act.as_any_mut().downcast_mut::<EnBa>().unwrap().setup_die(&mut w);
        w.cur_actor = None;
        w.actors.put_back(h, act);
    }
    let bl = blobs(&w);
    assert_eq!(bl.len(), 7);
    // EnBa_FallAsBlob: falling (gravity -2) to the floor, then 20 frames squashing (scale.y down
    // 0.001, .x and .z up 0.0005 a frame), gone.
    let mut landed = vec![None; bl.len()];
    for n in 0..200 {
        idle(&mut w, 1);
        for (i, &b) in bl.iter().enumerate() {
            if landed[i].is_none() && !gone(&w, b) && ba(&w, b).actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
                landed[i] = Some(n);
            }
        }
        if bl.iter().all(|&b| gone(&w, b)) && gone(&w, h) {
            break;
        }
    }
    assert!(gone(&w, h) && bl.iter().all(|&b| gone(&w, b)), "all gone");
    assert!(landed.iter().all(|l| l.is_some()), "each landed first");
    assert!(w.flags.get_switch(0x0B));
    // The room's last enemy gone (Actor_RemoveFromCategory): its temporary clear flag; the next
    // frame Obj_Roomtimer sets the room's clear flag and its switch flag, with the chime.
    idle(&mut w, 2);
    assert!(w.flags.get_clear(8));
    assert!(w.actors.all().into_iter().all(|h| w.actors.downcast::<ObjRoomtimer>(h).is_none_or(|t| t.actor.killed)));
}
