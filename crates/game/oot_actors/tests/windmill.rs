//! GAME-07 milestone 5 against the C: Impa's house's cow (`z_en_cow.c`) and the windmill
//! (`z_en_fu.c`, `z_bg_relay_objects.c`, with `En_Okarina_Tag`'s Song of Storms and the well's
//! draining cutscene).
//!
//! The songs are played on the pad as tests/ocarina.rs does (each note held 3 frames, let go 2),
//! with the `ocarina-songs` preset. Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, BTN_CDOWN, BTN_CLEFT, BTN_CRIGHT, BTN_CUP, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_relay_objects::{self, BgRelayObjects};
use oot_actors::en_cow::{self, EnCow};
use oot_actors::en_fu::{self, EnFu};
use oot_actors::player::Player;
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::audio::ocarina::*;
use oot_game::item::{ITEM_BOTTLE_EMPTY, ITEM_BOTTLE_MILK_FULL, SLOT_BOTTLE_1};
use oot_game::message::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

/// `QUEST_SONG_STORMS` (`item.h`: 0x11).
const QUEST_SONG_STORMS: u32 = 0x11;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Link (a child, or an adult with `adult`) entered at `entrance` at `hour`:00 with
/// `ocarina-songs`, `edit` applied to the save last, the audio side running (the ocarina counts
/// its tasks); two frames on.
fn enter(a: &Arc<GameAssets>, entrance: &str, adult: bool, hour: i32, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, adult, oot_game::env::clock_time(hour, 0) as u16);
    save.apply_preset("ocarina-songs").unwrap();
    edit(&mut save);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    w.audio_side = Some(Box::new(oot_game::audio::offline::OfflineAudio::new(&a.pack.audio_data().unwrap(), false)));
    idle(&mut w, 2);
    w
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// `b` pressed and held for `frames` frames, then let go.
fn press(w: &mut PlayState, b: u16, frames: usize) {
    let pad = PadState { button: b, ..Default::default() };
    for i in 0..frames {
        w.tick_with(scripted_input(if i == 0 { PadState::default() } else { pad }, pad));
    }
    w.tick_with(scripted_input(pad, PadState::default()));
}

fn all<T: 'static + ActorImpl>(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<T>(h).is_some_and(|a| !a.base().killed)).collect()
}

fn one<T: 'static + ActorImpl>(w: &PlayState) -> ActorHandle {
    let v = all::<T>(w);
    assert_eq!(v.len(), 1, "one {}", std::any::type_name::<T>());
    v[0]
}

fn get<T: 'static + ActorImpl>(w: &PlayState, h: ActorHandle) -> &T {
    w.actors.downcast::<T>(h).unwrap()
}

fn base(w: &PlayState, h: ActorHandle) -> &Actor {
    w.actors.actor(h).unwrap()
}

fn player(w: &PlayState) -> &Player {
    w.actors.downcast::<Player>(w.player.unwrap()).unwrap()
}

/// The pad button of an `OCARINA_BTN_*`.
fn pad_button(b: u8) -> u16 {
    [BTN_A, BTN_CDOWN, BTN_CRIGHT, BTN_CLEFT, BTN_CUP][b as usize]
}

/// `song`'s notes (the pack's `gOcarinaSongButtons`), each held 3 frames and let go 2.
fn play_song(w: &mut PlayState, song: u8) {
    let b = w.audio.ocarina.song_buttons(song);
    let notes: Vec<u16> = b.buttons_index[..b.num_buttons as usize].iter().map(|&i| pad_button(i)).collect();
    for n in notes {
        press(w, n, 3);
        idle(w, 1);
    }
}

/// Frames until `f` holds (at most `max`); panics with `what` if it never does.
fn until(w: &mut PlayState, max: usize, what: &str, f: impl Fn(&PlayState) -> bool) -> usize {
    for n in 0..max {
        if f(w) {
            return n;
        }
        idle(w, 1);
    }
    panic!("{what}: not after {max} frames (msgMode {:#x}, ocarinaMode {}, text {:#x}, cs state {})", w.msg_ctx.msg_mode, w.msg_ctx.ocarina_mode, w.msg_ctx.text_id, w.cs_ctx.state);
}

/// Until `f` holds (at most `max` frames), A pressed on a frame of its own whenever a text waits.
fn answer_until(w: &mut PlayState, max: usize, what: &str, f: impl Fn(&PlayState) -> bool) {
    let mut pressed = false;
    for _ in 0..max {
        if f(w) {
            return;
        }
        let st = w.message_state();
        let a = !pressed && matches!(st, TEXT_STATE_CHOICE | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_EVENT | TEXT_STATE_AWAITING_NEXT);
        let pad = PadState { button: if a { BTN_A } else { 0 }, ..Default::default() };
        w.tick_with(scripted_input(PadState::default(), pad));
        pressed = a;
    }
    panic!("{what}: not after {max} frames (msgMode {:#x}, text {:#x}, state {})", w.msg_ctx.msg_mode, w.msg_ctx.text_id, w.message_state());
}

/// The ocarina out with C-Left.
fn ocarina_out(w: &mut PlayState) {
    press(w, BTN_CLEFT, 1);
}

// The cow.

/// The cow's body, Impa's house's (`En_Cow` 0 at (140, 0, -255)).
fn cow(w: &PlayState) -> ActorHandle {
    let v: Vec<_> = all::<EnCow>(w).into_iter().filter(|&h| base(w, h).params == en_cow::COW_TYPE_BODY).collect();
    assert_eq!(v.len(), 1, "one cow");
    v[0]
}

#[test]
fn impas_cow_spawns_its_tail() {
    let Some(a) = assets() else { return };
    let w = enter(&a, "ENTR_IMPAS_HOUSE_0", false, 10, |_| {});
    // EnCow_Init, COW_TYPE_BODY: not Link's house, so it stays; its tail spawned as its child
    // (Actor_SpawnAsChild, COW_TYPE_TAIL) at its place with its yaw; animationTimer
    // Rand_ZeroFloat(1000) + 40; ATTENTION_RANGE_6; MASS_IMMOVABLE; R_EPONAS_SONG_PLAYED false.
    let h = cow(&w);
    let c = get::<EnCow>(&w, h);
    assert_eq!(c.actor.home_pos, Vec3::new(140.0, 0.0, -255.0));
    assert_eq!(c.action, en_cow::Action::Idle);
    assert_eq!(c.actor.target_mode, 6);
    assert_eq!(c.actor.col_chk_info.mass, 0xFF);
    assert!(c.skel.is("gCowBodyChewAnim"));
    assert!(!w.msg_ctx.r_eponas_song_played);
    // EnCow_SetColliderPos at yaw 0: the front cylinder 30 ahead (+z), the rear 20 behind.
    assert_eq!(c.colliders[0].dim.pos, [140, 0, -225]);
    assert_eq!(c.colliders[1].dim.pos, [140, 0, -275]);
    // The tail: EnCow_SetTailPos at yaw 0, (0, 57, -36) from the body; no attention
    // (ACTOR_FLAG_ATTENTION_ENABLED cleared), EnCow_IdleTail with gCowTailIdleAnim, scale 0.01.
    let tail = c.actor.child.expect("the tail");
    let t = get::<EnCow>(&w, tail);
    assert_eq!(t.actor.params, en_cow::COW_TYPE_TAIL);
    assert_eq!(t.actor.parent, Some(h));
    assert_eq!(t.actor.world_pos, Vec3::new(140.0, 57.0, -291.0));
    assert_eq!(t.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!(t.action, en_cow::Action::IdleTail);
    assert!(t.skel.is("gCowTailIdleAnim"));
    assert_eq!(t.actor.scale, Vec3::splat(0.01));
    assert!(t.animation_timer < 1040);
    assert_eq!(all::<EnCow>(&w).len(), 2);
}

#[test]
fn a_cow_breathes() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_IMPAS_HOUSE_0", false, 10, |_| {});
    let h = cow(&w);
    w.actors.actor_mut(h).unwrap().flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
    // EnCow_UpdateAnimation: breathTimer 0..48; at 8, scale.x ((sin(0x2000) / 100) + 1) * 0.01
    // and scale.y 0.01 (not past 16); at 24, scale.x (sin(0x6000)/100 + 1) * 0.01 and scale.y
    // (sin(0x2000)/100 + 1) * 0.01.
    let t = get::<EnCow>(&w, h).breath_timer as usize;
    idle(&mut w, (8 + 49 - t) % 49);
    let c = get::<EnCow>(&w, h);
    assert_eq!(c.breath_timer, 8);
    let s = |a: i16| ((eng_math::sin_s(a) * (1.0 / 100.0)) + 1.0) * 0.01;
    assert_eq!((c.actor.scale.x, c.actor.scale.y), (s(0x2000), 0.01));
    idle(&mut w, 16);
    let c = get::<EnCow>(&w, h);
    assert_eq!((c.actor.scale.x, c.actor.scale.y), (s(0x6000), s(0x2000)));
}

#[test]
fn in_links_house_the_cow_waits_for_the_adult_who_won_it() {
    let Some(a) = assets() else { return };
    // SCENE_LINKS_HOUSE: Actor_Kill for a child, or without EVENTCHKINF_HORSE_RACE_COW_UNLOCK (and
    // no tail then). Its adult layer's cow at (-83, 0, -78).
    let w = enter(&a, "ENTR_LINKS_HOUSE_0", true, 10, |_| {});
    assert!(all::<EnCow>(&w).is_empty());
    let mut w = enter(&a, "ENTR_LINKS_HOUSE_0", true, 10, |s| s.set_event_chk_inf(en_cow::EVENTCHKINF_HORSE_RACE_COW_UNLOCK));
    assert_eq!(all::<EnCow>(&w).len(), 2);
    assert_eq!(base(&w, cow(&w)).home_pos, Vec3::new(-83.0, 0.0, -78.0));
    // A child's: spawned there all the same, killed in its init.
    let pos = player(&w).actor.world_pos;
    w.save.adult = false;
    let h = w.actor_spawn(en_cow::ACTOR_EN_COW, pos + Vec3::new(0.0, 0.0, 100.0), [0; 3], 0).expect("En_Cow");
    assert!(base(&w, h).killed);
    assert!(base(&w, h).child.is_none());
}

#[test]
fn a_cow_out_of_reach_leaves_eponas_song_to_another() {
    let Some(a) = assets() else { return };
    // Link at the door (88, 0, 265), 530 from the cow. EnCow_Idle with R_EPONAS_SONG_PLAYED: out
    // of its 150, COW_FLAG_FAILED_TO_GIVE_MILK, the flag left for another cow; the next frame,
    // with its mark, it clears both.
    let mut w = enter(&a, "ENTR_IMPAS_HOUSE_0", false, 10, |_| {});
    let h = cow(&w);
    w.actors.actor_mut(h).unwrap().flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
    assert_eq!(w.msg_ctx.ocarina_mode, OCARINA_MODE_00);
    w.msg_ctx.r_eponas_song_played = true;
    idle(&mut w, 1);
    assert!(w.msg_ctx.r_eponas_song_played);
    assert_ne!(get::<EnCow>(&w, h).cow_flags & en_cow::COW_FLAG_FAILED_TO_GIVE_MILK, 0);
    idle(&mut w, 1);
    assert!(!w.msg_ctx.r_eponas_song_played);
    assert_eq!(get::<EnCow>(&w, h).cow_flags & en_cow::COW_FLAG_FAILED_TO_GIVE_MILK, 0);
    assert_eq!(get::<EnCow>(&w, h).action, en_cow::Action::Idle);
}

/// Epona's Song played by Impa's cow (`ENTR_IMPAS_HOUSE_1`, Link at (87, 0, -262), 53 west of
/// it), with an empty bottle or not; on to the cow's talk (0x2006 by `EnCow_Idle`).
fn eponas_song_to_the_cow(bottle: bool) -> Option<(PlayState, ActorHandle)> {
    let a = assets()?;
    let mut w = enter(&a, "ENTR_IMPAS_HOUSE_1", false, 10, |s| {
        if bottle {
            s.inventory.items[SLOT_BOTTLE_1] = ITEM_BOTTLE_EMPTY;
        }
    });
    let h = cow(&w);
    w.actors.actor_mut(h).unwrap().flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
    // (Link's walk in from the spawn done.)
    idle(&mut w, 40);
    // In reach: within 150 and |yawTowardsPlayer - shape.rot.y| < 25000 (Link west of it: about
    // -0x4000).
    let c = base(&w, h);
    assert!(c.xz_dist_to_player < 150.0, "{}", c.xz_dist_to_player);
    assert!((c.yaw_towards_player as i32).abs() < 25000, "{:#x}", c.yaw_towards_player);
    // Free play: MSGMODE_SONG_PLAYED_ACT closes the box and sets R_EPONAS_SONG_PLAYED; the
    // ocarina put away, ocarinaMode 04 (or 00): EnCow_Idle clears it, EnCow_Talk with
    // ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED, 0x2006 offered within 170: Player takes it at once.
    ocarina_out(&mut w);
    until(&mut w, 40, "the staff", |w| w.msg_ctx.msg_mode == MSGMODE_OCARINA_PLAYING);
    play_song(&mut w, OCARINA_SONG_EPONAS);
    until(&mut w, 400, "EnCow_Talk", |w| get::<EnCow>(w, h).action != en_cow::Action::Idle);
    assert!(!w.msg_ctx.r_eponas_song_played);
    assert_eq!(get::<EnCow>(&w, h).action, en_cow::Action::Talk);
    until(&mut w, 60, "0x2006", |w| w.msg_ctx.text_id == 0x2006 && w.message_state() != TEXT_STATE_NONE);
    assert_eq!(get::<EnCow>(&w, h).action, en_cow::Action::CheckForEmptyBottle);
    Some((w, h))
}

#[test]
fn eponas_song_by_a_cow_gives_milk_for_an_empty_bottle() {
    let Some((mut w, h)) = eponas_song_to_the_cow(true) else { return };
    // EnCow_CheckForEmptyBottle at 0x2006's event: Inventory_HasEmptyBottle, 0x2007,
    // EnCow_GiveMilk; at its event the box closed, EnCow_GiveMilkWait and Actor_OfferGetItem(GI_MILK,
    // 10000, 100): Player takes it (Item_Give(ITEM_MILK): the empty bottle filled).
    answer_until(&mut w, 400, "0x2007", |w| w.msg_ctx.text_id == 0x2007);
    assert_eq!(get::<EnCow>(&w, h).action, en_cow::Action::GiveMilk);
    answer_until(&mut w, 400, "the milk taken", |w| get::<EnCow>(w, h).action == en_cow::Action::GiveMilkEnd);
    // EnCow_GiveMilkWait saw its parent: cleared, EnCow_GiveMilkEnd until the get-item text closes.
    assert!(base(&w, h).parent.is_none());
    answer_until(&mut w, 600, "EnCow_Idle", |w| get::<EnCow>(w, h).action == en_cow::Action::Idle);
    assert_eq!(w.save.inventory.items[SLOT_BOTTLE_1], ITEM_BOTTLE_MILK_FULL);
    assert_eq!(base(&w, h).flags & oot_game::actor::ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED, 0);
}

#[test]
fn eponas_song_by_a_cow_without_a_bottle_only_talks() {
    let Some((mut w, h)) = eponas_song_to_the_cow(false) else { return };
    // No empty bottle: 0x2013 and EnCow_TalkEnd; at its event the box closed, EnCow_Idle.
    answer_until(&mut w, 400, "0x2013", |w| w.msg_ctx.text_id == 0x2013);
    assert_eq!(get::<EnCow>(&w, h).action, en_cow::Action::TalkEnd);
    answer_until(&mut w, 400, "EnCow_Idle", |w| get::<EnCow>(w, h).action == en_cow::Action::Idle);
    assert!(!w.save.inventory.items[SLOT_BOTTLE_1..SLOT_BOTTLE_1 + 4].contains(&ITEM_BOTTLE_MILK_FULL));
}

// The windmill.

/// Link in front of the windmill man ((3185, -127, 34), facing -x), `dist` west of him facing
/// him, a few frames on.
fn before_the_man(w: &mut PlayState, dist: f32) -> ActorHandle {
    let h = one::<EnFu>(w);
    let p = base(w, h).world_pos;
    w.place_player(Vec3::new(p.x - dist, p.y, p.z), 0x4000);
    idle(w, 5);
    h
}

#[test]
fn the_windmill_man_talks_to_a_child_of_the_well() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_WINDMILL_AND_DAMPES_GRAVE_1", false, 10, |_| {});
    // EnFu_Init for a child: EnFu_WaitChild, FU_FACE_CALM, gWindmillManPlayStillAnim looped.
    let h = before_the_man(&mut w, 60.0);
    let f = get::<EnFu>(&w, h);
    assert_eq!(f.actor.home_pos, Vec3::new(3185.0, -127.0, 34.0));
    assert_eq!((f.action, f.facial_expression), (en_fu::Action::WaitChild, en_fu::FU_FACE_CALM));
    // func_80A1D94C: within 100 and 0x2301 of his facing, 0x5032 offered (the well not drained).
    assert_eq!(f.actor.text_id, 0x5032);
    assert_eq!(player(&w).target_actor, Some(h));
    press(&mut w, BTN_A, 1);
    until(&mut w, 10, "func_80A1DA04", |w| get::<EnFu>(w, h).action == en_fu::Action::Func80a1da04);
    assert_eq!(w.msg_ctx.text_id, 0x5032);
    // The box closed (Actor_TextboxIsClosing): EnFu_WaitChild again.
    answer_until(&mut w, 400, "EnFu_WaitChild", |w| get::<EnFu>(w, h).action == en_fu::Action::WaitChild);
    // Out of his reach (behind him): FU_RESET_LOOK_ANGLE, his head turned back.
    let p = base(&w, h).world_pos;
    w.place_player(Vec3::new(p.x + 60.0, p.y, p.z), 0x4000);
    idle(&mut w, 1);
    assert_eq!(get::<EnFu>(&w, h).actor.text_id, 0x5032);

    // EVENTCHKINF_DRAINED_WELL: 0x5033, and with it gWindmillManPlayAndMoveHeadAnim (once, -4).
    let mut w = enter(&a, "ENTR_WINDMILL_AND_DAMPES_GRAVE_1", false, 10, |s| s.set_event_chk_inf(en_fu::EVENTCHKINF_DRAINED_WELL));
    let h = before_the_man(&mut w, 60.0);
    assert_eq!(get::<EnFu>(&w, h).actor.text_id, 0x5033);
    press(&mut w, BTN_A, 1);
    until(&mut w, 10, "func_80A1DA04", |w| get::<EnFu>(w, h).action == en_fu::Action::Func80a1da04);
    assert!(get::<EnFu>(&w, h).skel.is("gWindmillManPlayAndMoveHeadAnim"));
    // Closing: back to gWindmillManPlayStillAnim.
    answer_until(&mut w, 400, "EnFu_WaitChild", |w| get::<EnFu>(w, h).action == en_fu::Action::WaitChild);
    assert!(get::<EnFu>(&w, h).skel.is("gWindmillManPlayStillAnim"));
}

#[test]
fn the_windmills_gear_turns_by_the_flag() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_WINDMILL_AND_DAMPES_GRAVE_1", false, 10, |_| {});
    // Room 6's Bg_Relay_Objects 0x00FF: type (params >> 8) 0, WINDMILL_ROTATING_GEAR; room -1;
    // D_808A9508 bit 1. Without EVENTCHKINF_65 its speed (world.rot.y) 0x80, added to shape.rot.y
    // each frame.
    let gears: Vec<_> = all::<BgRelayObjects>(&w).into_iter().filter(|&h| base(&w, h).params == bg_relay_objects::WINDMILL_ROTATING_GEAR).collect();
    assert_eq!(gears.len(), 1);
    let h = gears[0];
    let g = get::<BgRelayObjects>(&w, h);
    assert_eq!((g.action, g.actor.room, g.actor.world_rot.y), (bg_relay_objects::Action::Func808a939c, -1, 0x80));
    let s = w.overlay_statics.get(&bg_relay_objects::ACTOR_BG_RELAY_OBJECTS).and_then(|s| s.downcast_ref::<bg_relay_objects::Statics>()).unwrap().d_808a9508;
    assert_ne!(s & 2, 0);
    let y = g.actor.shape_rot.y;
    idle(&mut w, 1);
    assert_eq!(base(&w, h).shape_rot.y, y.wrapping_add(0x80));
    // Room 6's door 0x0134 (type 1, switch flag 0x34): unk_169 room + 1 = 7, 6 or more, the first
    // such: D_808A9508 bit 0, BgRelayObjects_DoNothing.
    let doors: Vec<_> = all::<BgRelayObjects>(&w).into_iter().filter(|&h| base(&w, h).params == bg_relay_objects::WINDMILL_DAMPE_STONE_DOOR).collect();
    assert_eq!(doors.len(), 1);
    let d = get::<BgRelayObjects>(&w, doors[0]);
    assert_eq!((d.unk_169, d.switch_flag, d.action), (7, 0x34, bg_relay_objects::Action::DoNothing));
    assert_ne!(s & 1, 0);
    // The cutscene flag 5 (CutsceneFlags_Get): EVENTCHKINF_65 set, the speed stepped to 0x400 by
    // 8 (Math_ScaledStepToS: 12 a frame at R_UPDATE_RATE 3).
    w.flags_set_env(5);
    idle(&mut w, 1);
    assert!(w.save.get_event_chk_inf(bg_relay_objects::EVENTCHKINF_65));
    assert_eq!(base(&w, h).world_rot.y, 0x80 + 12);
    idle(&mut w, 80);
    assert_eq!(base(&w, h).world_rot.y, 0x400);
    // Entered with EVENTCHKINF_65: 0x400 from the start.
    let w2 = enter(&a, "ENTR_WINDMILL_AND_DAMPES_GRAVE_1", false, 10, |s| s.set_event_chk_inf(bg_relay_objects::EVENTCHKINF_65));
    let h2 = all::<BgRelayObjects>(&w2).into_iter().find(|&h| base(&w2, h).params == bg_relay_objects::WINDMILL_ROTATING_GEAR).unwrap();
    assert_eq!(base(&w2, h2).world_rot.y, 0x400);
    // BgRelayObjects_Destroy out of a cutscene's entrance (cutsceneIndex 0, below CS_INDEX_0):
    // EVENTCHKINF_65 cleared.
    w.flags_unset_env(5);
    w.delete_actor(h);
    assert!(!w.save.get_event_chk_inf(bg_relay_objects::EVENTCHKINF_65));
}

#[test]
fn the_childs_song_of_storms_drains_the_well() {
    let Some(a) = assets() else { return };
    // The windmill (ENTR_WINDMILL_AND_DAMPES_GRAVE_1: Link at (2890, -150, 275)); En_Okarina_Tag
    // 0x0BC2 (type 2, any song, switch flag 2) at (2892, -150, 30), its range 30 * 40 more.
    let mut w = enter(&a, "ENTR_WINDMILL_AND_DAMPES_GRAVE_1", false, 10, |_| {});
    // (Link's walk in from the spawn done.)
    idle(&mut w, 40);
    // The ocarina out near it: func_80ABF28C's OCARINA_ACTION_CHECK_STORMS; the Song of Storms:
    // OCARINA_MODE_03, func_80ABF4C8: gWindmillSpinningFasterCs.
    ocarina_out(&mut w);
    until(&mut w, 40, "CHECK_STORMS", |w| w.msg_ctx.ocarina_action == OCARINA_ACTION_CHECK_STORMS && w.msg_ctx.msg_mode == MSGMODE_OCARINA_PLAYING);
    play_song(&mut w, OCARINA_SONG_STORMS);
    until(&mut w, 400, "the cutscene", |w| w.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE);
    assert!(w.flags.get_switch(2));
    // CS_MISC_SET_FLAG_FAST_WINDMILL at frame 30: EVENTCHKINF_65 (the gear speeding up);
    // CS_DEST_KAKARIKO_VILLAGE_DRAIN_WELL at 200: ENTR_KAKARIKO_VILLAGE_0 with CS_INDEX_2
    // (gKakarikoVillageWellDrainingCs), whose CS_MISC_SET_FLAG_DRAINED_WELL at 180 sets
    // EVENTCHKINF_DRAINED_WELL; then CS_DEST_WINDMILL_FROM_WELL_DRAINED (at 320) back.
    until(&mut w, 300, "EVENTCHKINF_65", |w| w.save.get_event_chk_inf(bg_relay_objects::EVENTCHKINF_65));
    let kak = a.scenes.entrance_index("ENTR_KAKARIKO_VILLAGE_0").unwrap();
    until(&mut w, 600, "Kakariko", |w| w.save.entrance_index == kak && w.scene_id == 0x52);
    assert_eq!(w.save.cutscene_index, 0xFFF2);
    until(&mut w, 600, "EVENTCHKINF_DRAINED_WELL", |w| w.save.get_event_chk_inf(en_fu::EVENTCHKINF_DRAINED_WELL));
    let back = a.scenes.entrance_index("ENTR_WINDMILL_AND_DAMPES_GRAVE_2").unwrap();
    until(&mut w, 600, "the windmill again", |w| w.save.entrance_index == back && w.scene_id == 0x48);
    // EVENTCHKINF_65 kept (the windmill left from a cutscene: the gear's destroy, were it run,
    // sees CS_INDEX_2), so the gear's back at 0x400.
    assert!(w.save.get_event_chk_inf(bg_relay_objects::EVENTCHKINF_65));
    idle(&mut w, 2);
    let g = all::<BgRelayObjects>(&w).into_iter().find(|&h| base(&w, h).params == bg_relay_objects::WINDMILL_ROTATING_GEAR).expect("the gear");
    assert_eq!(base(&w, g).world_rot.y, 0x400);
}

#[test]
fn the_windmill_man_teaches_an_adult_the_song_of_storms() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_WINDMILL_AND_DAMPES_GRAVE_1", true, 10, |s| s.inventory.quest_items &= !(1 << QUEST_SONG_STORMS));
    // EnFu_Init for an adult: EnFu_WaitAdult, FU_FACE_MAD, playSpeed 2. Within 100 in front:
    // 0x5034 offered and PLAYER_STATE2_23 (something listens).
    let h = before_the_man(&mut w, 60.0);
    let f = get::<EnFu>(&w, h);
    assert_eq!((f.action, f.facial_expression, f.skel.play_speed), (en_fu::Action::WaitAdult, en_fu::FU_FACE_MAD, 2.0));
    assert_eq!(f.actor.text_id, 0x5034);
    // The ocarina out (PLAYER_STATE2_24): 0x5035, EnFu_TeachSong, FU_WAIT.
    ocarina_out(&mut w);
    until(&mut w, 60, "EnFu_TeachSong", |w| get::<EnFu>(w, h).action == en_fu::Action::TeachSong);
    assert_eq!(w.msg_ctx.text_id, 0x5035);
    assert_ne!(get::<EnFu>(&w, h).behavior_flags & en_fu::FU_WAIT, 0);
    // The box closing: OCARINA_ACTION_TEACH_STORMS (the grind organ: the message box's choice for
    // it), EnFu_WaitForPlayback.
    answer_until(&mut w, 400, "TEACH_STORMS", |w| w.msg_ctx.ocarina_action == OCARINA_ACTION_TEACH_STORMS);
    assert_eq!(get::<EnFu>(&w, h).action, en_fu::Action::WaitForPlayback);
    until(&mut w, 60, "the organ", |w| w.msg_ctx.msg_mode == MSGMODE_SONG_DEMONSTRATION);
    assert_eq!(w.audio.ocarina.instrument_id as u8, OCARINA_INSTRUMENT_GRIND_ORGAN);
    // The demonstration done: OCARINA_ACTION_PLAYBACK_STORMS (0x1A), func_80A1DBD4.
    until(&mut w, 1200, "PLAYBACK_STORMS", |w| w.msg_ctx.ocarina_action == en_fu::OCARINA_ACTION_PLAYBACK_STORMS && w.msg_ctx.msg_mode == MSGMODE_SONG_PLAYBACK);
    assert_eq!(get::<EnFu>(&w, h).action, en_fu::Action::Func80a1dbd4);
    // Played back: OCARINA_MODE_03; the chime, gSongOfStormsCs, Item_Give(ITEM_SONG_STORMS),
    // ocarinaMode 00, EVENTCHKINF_65, func_80A1DB60.
    play_song(&mut w, OCARINA_SONG_STORMS);
    until(&mut w, 600, "func_80A1DB60", |w| get::<EnFu>(w, h).action == en_fu::Action::Func80a1db60);
    assert!(w.save.check_quest_item(QUEST_SONG_STORMS));
    assert!(w.save.get_event_chk_inf(en_fu::EVENTCHKINF_65));
    assert!(!w.save.get_event_chk_inf(en_fu::EVENTCHKINF_5B));
    // The cutscene over: EVENTCHKINF_5B, OCARINA_MODE_04, EnFu_WaitAdult; then he says 0x508E.
    answer_until(&mut w, 1200, "EnFu_WaitAdult", |w| get::<EnFu>(w, h).action == en_fu::Action::WaitAdult);
    assert!(w.save.get_event_chk_inf(en_fu::EVENTCHKINF_5B));
    idle(&mut w, 2);
    assert_eq!(get::<EnFu>(&w, h).actor.text_id, 0x508E);
}
