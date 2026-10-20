//! GAME-09 milestone 5 against the C: `En_G_Switch` (`z_en_g_switch.c`), the silver rupees and
//! their tracker, the shooting gallery's rupees and horseback archery's pots. Master Quest's
//! Dodongo's Cavern room 2 has the tracker 0x0165 at (-1733, 0, -1461) and five rupees 0x1165.
//!
//! Entered by `ENTR_DODONGOS_CAVERN_0` on a `fairy-magic` save, then room 2 as walking into it
//! would. Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_g_switch::*;
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::sfx::{NA_SE_SY_CORRECT_CHIME, NA_SE_SY_GET_RUPY};
use oot_game::collision_check::AC_HIT;
use oot_game::env::clock_time;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// The silver rupees' flag (the tracker's params 0x0165, the rupees' 0x1165).
const SILVER_FLAG: i32 = 0x25;

/// Child Link in room 2 (`fairy-magic`), by the room's south door, three frames on, the audio
/// logged.
fn room2() -> Option<PlayState> {
    let a = assets()?;
    let e = a.scenes.entrance_index("ENTR_DODONGOS_CAVERN_0").expect("entrance");
    let mut save = SaveContext::new(e, false, clock_time(10, 0) as u16);
    save.apply_preset("fairy-magic").unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    assert!(w.room_request(2));
    idle(&mut w, 1);
    w.room_change_done();
    w.audio.log = Some(Default::default());
    w.place_player(Vec3::new(-1960.0, 0.0, -1150.0), i16::MIN);
    idle(&mut w, 3);
    Some(w)
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn switches(w: &PlayState) -> Vec<ActorHandle> {
    let mut v: Vec<ActorHandle> = w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnGSwitch>(h).is_some_and(|s| !s.actor.killed)).collect();
    v.sort_by_key(|&h| w.actors.actor(h).unwrap().params);
    v
}

fn sw(w: &PlayState, h: ActorHandle) -> &EnGSwitch {
    w.actors.downcast::<EnGSwitch>(h).unwrap()
}

fn tracker(w: &PlayState) -> Option<ActorHandle> {
    switches(w).into_iter().find(|&h| sw(w, h).type_ == ENGSWITCH_SILVER_TRACKER)
}

fn rupees(w: &PlayState) -> Vec<ActorHandle> {
    switches(w).into_iter().filter(|&h| sw(w, h).type_ == ENGSWITCH_SILVER_RUPEE).collect()
}

/// Killed (`Actor_Kill`) or deleted already.
fn gone(w: &PlayState, h: ActorHandle) -> bool {
    w.actors.actor(h).is_none_or(|a| a.killed)
}

fn sfx_count(w: &PlayState, id: u16) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).count()
}

/// The rupee brought to Link (within 30 of him, `xyzDistToPlayerSq`): most of room 2's are in
/// crates, on ledges or under a Beamos, which the hand test reaches.
fn take(w: &mut PlayState, h: ActorHandle) {
    let p = w.actors.actor(w.player.unwrap()).unwrap().world_pos;
    w.actors.actor_mut(h).unwrap().world_pos = p + Vec3::new(0.0, 10.0, 0.0);
    idle(w, 1);
}

#[test]
fn the_tracker_and_five_rupees_wait_in_room_2() {
    let Some(w) = room2() else { return };
    // The tracker 0x0165: ENGSWITCH_SILVER_TRACKER, switch flag 0x25, silverCount (0x0165 >> 6)
    // & 0x3F = 5; sCollectedCount zeroed; no draw.
    let t = tracker(&w).expect("the tracker");
    let s = sw(&w, t);
    assert_eq!((s.switch_flag, s.silver_count, s.action, s.draw), (0x25, 5, Action::SilverRupeeTracker, Draw::None));
    // The rupees 0x1165: silver (colorIdx 5), scale 0.03, 700 up, 20 effect slots, the cylinder
    // 13 by 40, spinning 0x800 a frame.
    let r = rupees(&w);
    assert_eq!(r.len(), 5);
    for &h in &r {
        let s = sw(&w, h);
        assert_eq!((s.switch_flag, s.color_idx, s.num_effects, s.action, s.draw), (0x25, 5, 20, Action::SilverRupeeIdle, Draw::Rupee));
        assert_eq!((s.actor.scale, s.actor.shape_y_offset), (Vec3::splat(0.03), 700.0));
        assert_eq!((s.collider.dim.radius, s.collider.dim.height), (13, 40));
    }
    let y0 = sw(&w, r[0]).actor.shape_rot.y;
    let mut w = w;
    idle(&mut w, 1);
    assert_eq!(sw(&w, r[0]).actor.shape_rot.y, y0.wrapping_add(0x800));
}

#[test]
fn each_rupee_taken_pays_5_and_rides_over_link_15_frames() {
    let Some(mut w) = room2() else { return };
    let r = rupees(&w);
    // The one at (-2189, 0, -1834) (under a crate).
    let h = *r.iter().find(|&&h| (sw(&w, h).actor.home_pos.x + 2189.0).abs() < 1.0).unwrap();
    w.save.rupees = 0;
    w.save.rupee_accumulator = 0;
    let got = sfx_count(&w, NA_SE_SY_GET_RUPY);
    take(&mut w, h);
    // EnGSwitch_SilverRupeeIdle: Rupees_ChangeBy(5), sCollectedCount 1, NA_SE_SY_GET_RUPY; 40
    // over Link (a child), no gravity, killTimer 15: EnGSwitch_SilverRupeeCollected.
    let s = sw(&w, h);
    assert_eq!(s.action, Action::SilverRupeeCollected);
    assert_eq!(w.save.rupees + w.save.rupee_accumulator, 5);
    assert_eq!(oot_actors::en_g_switch::statics(&mut w).collected_count, 1);
    assert!(sfx_count(&w, NA_SE_SY_GET_RUPY) > got);
    // The tracker (updated before the rupee, both ACTORCAT_PROP) hears it the next frame: its
    // note (NA_SE_EV_FIVE_COUNT_LUPY, majorScale[0]) and noteIndex 1.
    assert_eq!(sfx_count(&w, NA_SE_EV_FIVE_COUNT_LUPY), 0);
    idle(&mut w, 1);
    assert_eq!(sfx_count(&w, NA_SE_EV_FIVE_COUNT_LUPY), 1);
    assert_eq!(sw(&w, tracker(&w).unwrap()).note_index, 1);
    // Over Link's head: 40 up plus killTimer * 0.3 * sin(killTimer * 0x3A98).
    let lp = w.actors.actor(w.player.unwrap()).unwrap().world_pos;
    let s = sw(&w, h);
    // (The update counts killTimer down after the action placed it.)
    let k = s.kill_timer + 1;
    let want = lp.y + 40.0 + (k as f32 * 0.3) * eng_math::sin_s((k as i32 * 0x3A98) as i16);
    assert!((s.actor.world_pos.y - want).abs() < 1e-3, "{} against {want}", s.actor.world_pos.y);
    // 15 frames from the take (the update counts down after the action): gone.
    for _ in 0..20 {
        if gone(&w, h) {
            break;
        }
        idle(&mut w, 1);
    }
    assert!(gone(&w, h));
    assert!(!w.flags.get_switch(SILVER_FLAG));
}

#[test]
fn the_fifth_rupee_sets_flag_0x25_with_the_chime() {
    let Some(mut w) = room2() else { return };
    let chime = sfx_count(&w, NA_SE_SY_CORRECT_CHIME);
    let r = rupees(&w);
    for (i, &h) in r.iter().enumerate() {
        take(&mut w, h);
        idle(&mut w, 20);
        if i < 4 {
            assert_eq!(oot_actors::en_g_switch::statics(&mut w).collected_count, i as i16 + 1);
            assert!(!w.flags.get_switch(SILVER_FLAG));
            // A note for each, up the scale (0, 2, 4, 5: the fifth takes the chime instead).
            assert_eq!(sfx_count(&w, NA_SE_EV_FIVE_COUNT_LUPY), i + 1);
        }
    }
    // EnGSwitch_SilverRupeeTracker: sCollectedCount 5 >= silverCount: the chime, flag 0x25, the
    // rupee sound, the tracker gone (no note for the fifth: under 5 only).
    assert!(w.flags.get_switch(SILVER_FLAG));
    // Two chimes: the tracker's, and door 17's (to room 5, SHUTTER_FRONT_SWITCH on 0x25, barred
    // from room 2's side): its bars lift with OnePointCutscene_Attention on it, whose sound is
    // NA_SE_SY_CORRECT_CHIME.
    assert_eq!(sfx_count(&w, NA_SE_SY_CORRECT_CHIME), chime + 2);
    let door = w.actors.all().into_iter().find(|&h| w.actors.downcast::<oot_actors::door_shutter::DoorShutter>(h).is_some_and(|d| d.actor.params & 0x3F == 0x25)).expect("door 17");
    let d = w.actors.downcast::<oot_actors::door_shutter::DoorShutter>(door).unwrap();
    assert!(d.bars_closed_amount < 1.0, "door 17's bars {}", d.bars_closed_amount);
    assert_eq!(sfx_count(&w, NA_SE_EV_FIVE_COUNT_LUPY), 4);
    assert!(tracker(&w).is_none());
    assert!(switches(&w).is_empty());
    // The last En_G_Switch gone, the overlay is freed: sCollectedCount 0 with the next load.
    assert_eq!(oot_actors::en_g_switch::statics(&mut w).collected_count, 0);
}

#[test]
fn with_flag_0x25_set_the_tracker_and_rupees_are_gone_at_init() {
    let Some(a) = assets() else { return };
    let e = a.scenes.entrance_index("ENTR_DODONGOS_CAVERN_0").expect("entrance");
    let mut save = SaveContext::new(e, false, clock_time(10, 0) as u16);
    save.apply_preset("fairy-magic").unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    // Flag 0x25 is a temp flag (0x20 and up: the scene's, lost on leaving it): set in the scene
    // before room 2 loads.
    w.flags.set_switch(SILVER_FLAG);
    assert!(w.room_request(2));
    idle(&mut w, 1);
    w.room_change_done();
    idle(&mut w, 2);
    assert!(switches(&w).is_empty());
}

#[test]
fn the_tracker_counts_from_zero_with_a_fresh_overlay() {
    let Some(mut w) = room2() else { return };
    let r0 = rupees(&w)[0];
    take(&mut w, r0);
    idle(&mut w, 20);
    assert_eq!(oot_actors::en_g_switch::statics(&mut w).collected_count, 1);
    // A second tracker spawned (its init zeroes sCollectedCount, shared by the overlay).
    let h = w.actor_spawn(ACTOR_EN_G_SWITCH, Vec3::new(-1733.0, 0.0, -1400.0), [0; 3], 0x0165).unwrap();
    assert_eq!(sw(&w, h).silver_count, 5);
    assert_eq!(oot_actors::en_g_switch::statics(&mut w).collected_count, 0);
}

#[test]
fn a_gallery_rupee_hit_bursts_and_goes_50_frames_later() {
    let Some(mut w) = room2() else { return };
    // ENGSWITCH_TARGET_RUPEE: scale 0.05, 700 up, the cylinder 20 by 60 by 5, drawn as a rupee,
    // 100 effect slots (EN_GSWITCH_EFFECT_COUNT).
    let h = w.actor_spawn(ACTOR_EN_G_SWITCH, Vec3::new(-1960.0, 40.0, -1250.0), [0; 3], 0x3000).unwrap();
    let s = sw(&w, h);
    assert_eq!((s.type_, s.actor.scale, s.actor.shape_y_offset, s.num_effects, s.action), (ENGSWITCH_TARGET_RUPEE, Vec3::splat(0.05), 700.0, 100, Action::GalleryRupee));
    assert_eq!((s.collider.dim.radius, s.collider.dim.height, s.collider.dim.y_shift), (20, 60, 5));
    // Its parent the gallery (here the tracker stands in: En_Syateki_Itm isn't ported, its writes
    // logged); `moveMode` GSWITCH_NONE heads for targetPos (0, 0, 0 as the init leaves it) at 0.3
    // of the way, at most 30.
    let t = tracker(&w).unwrap();
    w.actors.actor_mut(h).unwrap().parent = Some(t);
    idle(&mut w, 1);
    let p = sw(&w, h).actor.world_pos;
    assert_eq!((p.x, p.y), (-1960.0 + 30.0, 40.0 - 12.0));
    // Hit: the gallery's count (logged), NA_SE_EV_HIT_SOUND, the break: 100 small rupees burst 30
    // up round it, broken (not drawn), EnGSwitch_Kill with killTimer 50.
    let s = w.actors.downcast_mut::<EnGSwitch>(h).unwrap();
    s.collider.base.ac_flags |= AC_HIT;
    idle(&mut w, 1);
    let s = sw(&w, h);
    assert_eq!((s.broken, s.action, s.kill_timer), (true, Action::Kill, 49));
    assert_eq!(s.effects.iter().filter(|e| e.flag).count(), 100);
    assert!(sfx_count(&w, NA_SE_EV_HIT_SOUND) == 1);
    // EnGSwitch_UpdateEffects: 30 frames out, then 2 smaller a frame from scale 100 to under 10
    // (47 more): the kill (50) takes the burst with it.
    // EnGSwitch_Kill reads the timer before the update counts it down: 49 frames on it's at 0,
    // the next it goes.
    idle(&mut w, 49);
    assert_eq!((gone(&w, h), sw(&w, h).kill_timer), (false, 0));
    assert_eq!(sw(&w, h).effects.iter().filter(|e| e.flag).count(), 100);
    idle(&mut w, 1);
    assert!(gone(&w, h));
}

#[test]
fn an_archery_pot_without_its_object_goes_at_init() {
    let Some(mut w) = room2() else { return };
    // ENGSWITCH_ARCHERY_POT: object_tsubo isn't in room 2's objects: Object_GetSlot < 0, killed.
    let h = w.actor_spawn(ACTOR_EN_G_SWITCH, Vec3::new(-1960.0, 0.0, -1250.0), [0; 3], 0x2000);
    if let Ok(h) = h {
        let s = sw(&w, h);
        assert!(s.actor.killed);
        assert_eq!((s.actor.gravity, s.actor.scale), (-3.0, Vec3::new(0.25, 0.45, 0.25)));
        assert_eq!((s.collider.dim.radius, s.collider.dim.height, s.collider.info.ac_dmg_info.dmg_flags), (24, 74, DMG_ARROW));
        assert!(s.color_idx <= 2);
    }
    idle(&mut w, 1);
}
