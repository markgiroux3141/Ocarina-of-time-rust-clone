//! Adult Malon (`z_en_ma2.c`) against the C: her init (`EnMa2_GetSpawnIndex`: GAME-08 milestone
//! 3), killed for a child; and (GAME-12 milestone 4) the adult's: who she is where, her texts, and
//! Epona's Song played to her by night (`EnMa2_WaitForOcarina` to `EnMa2_ForceTalkAfterSong`), the
//! ocarina's result put in the message context as the ocarina box leaves it.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_ma2::{self, EnMa2};
use oot_actors::player::{STATE2_23, STATE2_24, STATE2_25};
use oot_game::actor::{ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED, Actor};
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::message::{MSGMODE_TEXT_AWAIT_INPUT, OCARINA_ACTION_CHECK_EPONA, OCARINA_MODE_03, OCARINA_MODE_04, TEXT_STATE_AWAITING_NEXT, TEXT_STATE_DONE, TEXT_STATE_DONE_HAS_NEXT, TEXT_STATE_NONE};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

/// `EVENTCHKINF_B4` (`save.h`): the ranch's intro seen.
const EVENTCHKINF_B4: u16 = 0xB4;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Link (`adult` or a child) entered at `entrance` at `hour`:00 with `preset`, `edit` applied to
/// the save last; four frames on (the room's actors are in on the third).
fn enter_age(a: &Arc<GameAssets>, entrance: &str, adult: bool, hour: i32, preset: &str, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, adult, oot_game::env::clock_time(hour, 0) as u16);
    save.apply_preset(preset).unwrap();
    save.set_event_chk_inf(EVENTCHKINF_B4);
    edit(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    for _ in 0..4 {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
    w
}

/// A child in the stable after Talon's return (`castle-courtyard`).
fn stable(a: &Arc<GameAssets>, hour: i32, _talon_back: bool) -> PlayState {
    enter_age(a, "ENTR_STABLE_0", false, hour, "castle-courtyard", |_| {})
}

/// The live placeholders standing in for actor `id`.
fn placeholders(w: &PlayState, id: i16) -> Vec<Vec3> {
    w.actors.all().into_iter().filter(|&h| w.actors.get(h).is_some_and(|a| a.name() == "Placeholder" && a.base().id == id && !a.base().killed)).map(|h| w.actors.actor(h).unwrap().home_pos).collect()
}

/// The one live adult Malon.
fn one_malon(w: &PlayState) -> ActorHandle {
    let v: Vec<_> = w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnMa2>(h).is_some_and(|a| !a.base().killed)).collect();
    assert_eq!(v.len(), 1, "one En_Ma2");
    v[0]
}

fn malon(w: &PlayState, h: ActorHandle) -> &EnMa2 {
    w.actors.downcast::<EnMa2>(h).unwrap()
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// `EnMa2_Update` alone, as the actor context runs it.
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut x = w.actors.take(h).unwrap();
    w.cur_actor = Some(h);
    x.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, x);
}

fn player_state2(w: &PlayState) -> u32 {
    w.player().state2
}

/// Link `d` in front of `h` (along its facing), facing it.
fn before(w: &mut PlayState, h: ActorHandle, d: f32) {
    let a = w.actors.actor(h).unwrap();
    let (p, yaw) = (a.world_pos, a.shape_rot.y);
    let r = yaw as f32 * std::f32::consts::PI / 32768.0;
    w.place_player(Vec3::new(p.x + d * r.sin(), p.y, p.z + d * r.cos()), yaw.wrapping_add(i16::MIN));
}

/// A whenever the box waits (once each wait) until it closes. The texts shown.
fn talk_on(w: &mut PlayState) -> Vec<u16> {
    let a = PadState { button: BTN_A, ..Default::default() };
    let mut prev = PadState::default();
    let mut texts = Vec::new();
    let mut pressed_at = None;
    for _ in 0..2000 {
        let (st, mode) = (w.message_state(), w.msg_ctx.msg_mode);
        if st != TEXT_STATE_NONE && texts.last() != Some(&w.msg_ctx.text_id) {
            texts.push(w.msg_ctx.text_id);
        }
        if st == TEXT_STATE_NONE && mode == 0 {
            return texts;
        }
        if pressed_at.is_some_and(|p| p != (st, mode)) {
            pressed_at = None;
        }
        let waits = matches!(st, TEXT_STATE_AWAITING_NEXT | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT) || mode == MSGMODE_TEXT_AWAIT_INPUT;
        let pad = if waits && prev.button == 0 && pressed_at.is_none() {
            pressed_at = Some((st, mode));
            a
        } else {
            PadState::default()
        };
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
    }
    panic!("the talk didn't end: {texts:x?}");
}

/// Any live actor of `id`, ported or a placeholder.
fn any_live(w: &PlayState, id: i16) -> usize {
    w.actors.all().into_iter().filter(|&h| w.actors.actor(h).is_some_and(|a| a.id == id && !a.killed)).count()
}

#[test]
fn adult_malon_is_killed_for_a_child_and_stands_in_the_stable_for_an_adult() {
    let Some(a) = assets() else { return };
    // malon_stable: En_Ma2 -1 at (120, 0, 0), rot.z 5. EnMa2_GetSpawnIndex: a child,
    // MALON_ADULT_SPAWN_NONE (killed).
    for hour in [10, 22] {
        let w = stable(&a, hour, true);
        assert_eq!(any_live(&w, en_ma2::ACTOR_EN_MA2), 0);
    }
    // An adult by day without Epona: MALON_ADULT_SPAWN_DAY_STABLES_NO_EPONA:
    // MALON_ADULT_ANIM_STANDING (gMalonAdultStandStillAnim, her basket), EnMa2_WaitToEndTalk,
    // scale 0.01, ATTENTION_RANGE_6.
    let w = enter_age(&a, "ENTR_STABLE_0", true, 10, "deku-tree-dead", |_| {});
    assert!(placeholders(&w, en_ma2::ACTOR_EN_MA2).is_empty());
    let m = w.actors.downcast::<EnMa2>(one_malon(&w)).unwrap();
    assert_eq!((m.action, m.actor.shape_rot.z, m.actor.scale.x, m.actor.target_mode), (en_ma2::Action::WaitToEndTalk, 5, 0.01, 6));
    assert!(m.skel.is(en_ma2::STAND_STILL_ANIM));
    // Her text: 0x204C (by day, before Epona).
    assert_eq!(EnMa2::get_text_id(&w), 0x204C);
    // By night, or with Epona: MALON_ADULT_SPAWN_NONE (the stable is only her day's before Epona).
    let w = enter_age(&a, "ENTR_STABLE_0", true, 22, "deku-tree-dead", |_| {});
    assert_eq!(any_live(&w, en_ma2::ACTOR_EN_MA2), 0);
    let w = enter_age(&a, "ENTR_STABLE_0", true, 10, "deku-tree-dead", |s| s.set_event_chk_inf(en_ma2::EVENTCHKINF_EPONA_OBTAINED));
    assert_eq!(any_live(&w, en_ma2::ACTOR_EN_MA2), 0);
}

#[test]
fn adult_malons_spawn_index_follows_the_c() {
    let Some(a) = assets() else { return };
    use en_ma2::*;
    // EnMa2_GetSpawnIndex over the scene, the time, rot.z and Epona, for an adult.
    let mut w = enter_age(&a, "ENTR_STABLE_0", true, 10, "deku-tree-dead", |_| {});
    let mut actor = Actor::new(Vec3::ZERO, 0);
    let mut idx = |w: &mut PlayState, scene: u16, day: bool, rot_z: i16, epona: bool| {
        w.scene_id = scene;
        w.save.night_flag = !day;
        if epona {
            w.save.set_event_chk_inf(EVENTCHKINF_EPONA_OBTAINED);
        } else {
            w.save.clear_event_chk_inf(EVENTCHKINF_EPONA_OBTAINED);
        }
        actor.shape_rot.z = rot_z;
        EnMa2::get_spawn_index(&actor, w)
    };
    const STABLE: u16 = 0x36;
    const RANCH: u16 = 0x63;
    assert_eq!(idx(&mut w, STABLE, true, 5, false), MALON_ADULT_SPAWN_DAY_STABLES_NO_EPONA);
    assert_eq!(idx(&mut w, STABLE, true, 5, true), MALON_ADULT_SPAWN_NONE);
    assert_eq!(idx(&mut w, RANCH, false, 6, false), MALON_ADULT_SPAWN_NIGHT_RANCH_NO_EPONA);
    assert_eq!(idx(&mut w, RANCH, true, 6, false), MALON_ADULT_SPAWN_NONE);
    assert_eq!(idx(&mut w, RANCH, true, 7, true), MALON_ADULT_SPAWN_ALWAYS);
    assert_eq!(idx(&mut w, RANCH, false, 7, true), MALON_ADULT_SPAWN_NONE);
    assert_eq!(idx(&mut w, RANCH, false, 8, true), MALON_ADULT_SPAWN_ALWAYS);
    assert_eq!(idx(&mut w, RANCH, true, 8, true), MALON_ADULT_SPAWN_NONE);
    assert_eq!(idx(&mut w, STABLE, true, 7, true), MALON_ADULT_SPAWN_NONE);
    // A child: none, whatever else.
    w.save.adult = false;
    assert_eq!(idx(&mut w, STABLE, true, 5, false), MALON_ADULT_SPAWN_NONE);
}


#[test]
fn eponas_song_played_to_her_by_night() {
    let Some(a) = assets() else { return };
    // spot20 layer 3 (an adult by night): En_Ma2 -1 at (66, 0, -402), rot.z 6 and 8. Before Epona
    // (adult-awake): rot.z 6 is MALON_ADULT_SPAWN_NIGHT_RANCH_NO_EPONA (MALON_ADULT_ANIM_SING,
    // EnMa2_WaitForOcarina); rot.z 8 is MALON_ADULT_SPAWN_NONE, killed.
    let mut w = enter_age(&a, "ENTR_LON_LON_RANCH_0", true, 22, "adult-awake", |_| {});
    let h = one_malon(&w);
    let m = malon(&w, h);
    assert_eq!((m.actor.home_pos.x, m.actor.home_pos.z, m.actor.shape_rot.z), (66.0, -402.0, 6));
    assert_eq!(m.action, en_ma2::Action::WaitForOcarina);
    assert!(m.skel.is(en_ma2::SING_ANIM));
    // Her text before the song: 0x2050.
    assert_eq!(EnMa2::get_text_id(&w), 0x2050);

    // Link within 30 + collider.dim.radius (48) of her: PLAYER_STATE2_23 (he may take out the
    // ocarina), set by her each frame after Player's update.
    before(&mut w, h, 40.0);
    idle(&mut w, 2);
    assert!(malon(&w, h).actor.xz_dist_to_player < 48.0);
    assert_ne!(player_state2(&w) & STATE2_23, 0);

    // The ocarina out (PLAYER_STATE2_24): PLAYER_STATE2_25, unk_6A8 her,
    // Message_StartOcarina(OCARINA_ACTION_CHECK_EPONA), EnMa2_WaitForEponasSong.
    w.player_mut().state2 |= STATE2_24;
    update(&mut w, h);
    assert_eq!(malon(&w, h).action, en_ma2::Action::WaitForEponasSong);
    assert_ne!(player_state2(&w) & STATE2_25, 0);
    assert_eq!(w.player().unk_6a8, Some(h));
    assert_eq!(w.msg_ctx.ocarina_action, OCARINA_ACTION_CHECK_EPONA);
    // Another song (OCARINA_MODE_04 and up): back to EnMa2_WaitForOcarina, the mode 4.
    w.msg_ctx.ocarina_mode = OCARINA_MODE_04;
    update(&mut w, h);
    assert_eq!(malon(&w, h).action, en_ma2::Action::WaitForOcarina);
    assert!(!w.save.get_inf_table(en_ma2::INFTABLE_PLAYED_SONG_FOR_ADULT_MALON));
    w.player_mut().state2 |= STATE2_24;
    update(&mut w, h);
    assert_eq!(malon(&w, h).action, en_ma2::Action::WaitForEponasSong);

    // Epona's Song (OCARINA_MODE_03): the chime, timer 30, INFTABLE_PLAYED_SONG_FOR_ADULT_MALON,
    // EnMa2_ForceTalkAfterSong, the mode 4.
    w.msg_ctx.ocarina_mode = OCARINA_MODE_03;
    update(&mut w, h);
    let m = malon(&w, h);
    assert_eq!((m.action, m.timer), (en_ma2::Action::ForceTalkAfterSong, 30));
    assert!(w.save.get_inf_table(en_ma2::INFTABLE_PLAYED_SONG_FOR_ADULT_MALON));
    assert_eq!(w.msg_ctx.ocarina_mode, OCARINA_MODE_04);
    assert_eq!(EnMa2::get_text_id(&w), 0x2051);

    // 30 frames on (DECR(timer)), she closes the ocarina's box and talks at once
    // (ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED): 0x2051, running on into 0x2052.
    let mut frames = 0;
    while w.msg_ctx.text_id != 0x2051 {
        idle(&mut w, 1);
        frames += 1;
        assert!(frames < 90, "she didn't talk (text {:#x})", w.msg_ctx.text_id);
    }
    assert!(frames >= 30, "{frames}");
    let texts = talk_on(&mut w);
    assert_eq!(texts, vec![0x2051, 0x2052]);
    // 0x2051's closing: INFTABLE_TALKED_TO_ADULT_MALON_AFTER_SONG, NPC_TALK_STATE_ACTION; then
    // EnMa2_WaitToEndTalk takes the auto-accept off and the talk state back to idle.
    idle(&mut w, 2);
    assert!(w.save.get_inf_table(en_ma2::INFTABLE_TALKED_TO_ADULT_MALON_AFTER_SONG));
    let m = malon(&w, h);
    assert_eq!((m.action, m.interact_info.talk_state), (en_ma2::Action::WaitToEndTalk, 0));
    assert_eq!(m.actor.flags & ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED, 0);
    // After: 0x2052.
    assert_eq!(EnMa2::get_text_id(&w), 0x2052);
    // With Epona: 0x2056.
    w.save.set_event_chk_inf(en_ma2::EVENTCHKINF_EPONA_OBTAINED);
    assert_eq!(EnMa2::get_text_id(&w), 0x2056);
}

#[test]
fn after_epona_she_is_in_the_ranch_by_night() {
    let Some(a) = assets() else { return };
    // spot20 layer 3 with Epona: rot.z 8 is MALON_ADULT_SPAWN_ALWAYS (no INFTABLE_8D: singing,
    // EnMa2_WaitToEndTalk); rot.z 6 killed.
    let w = enter_age(&a, "ENTR_LON_LON_RANCH_0", true, 22, "epona-won", |_| {});
    let m = malon(&w, one_malon(&w));
    assert_eq!((m.actor.shape_rot.z, m.action), (8, en_ma2::Action::WaitToEndTalk));
    assert!(m.skel.is(en_ma2::SING_ANIM));
    assert_eq!(EnMa2::get_text_id(&w), 0x2056);
    // With INFTABLE_8D (unreachable in the game): the idle.
    let w = enter_age(&a, "ENTR_LON_LON_RANCH_0", true, 22, "epona-won", |s| s.set_inf_table(en_ma2::INFTABLE_8D));
    assert!(malon(&w, one_malon(&w)).skel.is(en_ma2::IDLE_ANIM));
}
