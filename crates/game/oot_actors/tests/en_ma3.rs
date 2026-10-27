//! GAME-12 milestone 4 against the C: adult Malon by the corral (`z_en_ma3.c`): where she is
//! (`func_80AA2EC8`), her texts by the flags (`EnMa3_GetTextId`), the course offered and started
//! (`EnMa3_UpdateTalkState`'s choice and event), and its results (the high score, the cow's
//! unlock). No course is ridden: its result is put in the save as the referee leaves it.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_ma3::{self, EnMa3};
use oot_game::actor::{ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED, Actor};
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::horse::{ACTOR_EN_HORSE, ENTR_LON_LON_RANCH_0};
use oot_game::message::{MSGMODE_TEXT_AWAIT_INPUT, TEXT_STATE_AWAITING_NEXT, TEXT_STATE_CHOICE, TEXT_STATE_DONE, TEXT_STATE_DONE_HAS_NEXT, TEXT_STATE_EVENT, TEXT_STATE_NONE};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::{SaveContext, TIMER_STATE_OFF, TIMER_STATE_STOP, TIMER_STATE_UP_FREEZE};
use oot_game::transition::TRANS_TRIGGER_START;

/// `EVENTCHKINF_B4` (`save.h`): the ranch's intro seen.
const EVENTCHKINF_B4: u16 = 0xB4;
/// `HIGH_SCORE(HS_HORSE_RACE)`.
const HS: usize = en_ma3::HS_HORSE_RACE;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// Link (an adult, or a child) entered at `entrance` at 10:00 with `preset`, the ranch's intro
/// seen, `edit` applied to the save last; four frames on.
fn enter(a: &Arc<GameAssets>, entrance: &str, adult: bool, preset: &str, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, adult, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset(preset).unwrap();
    save.set_event_chk_inf(EVENTCHKINF_B4);
    edit(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    idle(&mut w, 4);
    w
}

fn malons(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnMa3>(h).is_some_and(|a| !a.base().killed)).collect()
}

fn malon(w: &PlayState) -> ActorHandle {
    let v = malons(w);
    assert_eq!(v.len(), 1, "one En_Ma3");
    v[0]
}

fn any_live(w: &PlayState, id: i16) -> usize {
    w.actors.all().into_iter().filter(|&h| w.actors.actor(h).is_some_and(|a| a.id == id && !a.killed)).count()
}

fn base(w: &PlayState, h: ActorHandle) -> &Actor {
    w.actors.actor(h).unwrap()
}

/// Every horse's place (`ACTOR_EN_HORSE`), `at` from Malon (or far off with None).
fn horses_at(w: &mut PlayState, at: Option<f32>) {
    let m = base(w, malon(w)).world_pos;
    let horses: Vec<_> = w.actors.all().into_iter().filter(|&h| w.actors.actor(h).is_some_and(|a| a.id == ACTOR_EN_HORSE && !a.killed)).collect();
    assert!(!horses.is_empty(), "no horse");
    for h in horses {
        let a = w.actors.actor_mut(h).unwrap();
        a.world_pos = match at {
            Some(d) => m + Vec3::new(d, 0.0, 0.0),
            None => m + Vec3::new(5000.0, 0.0, 0.0),
        };
    }
}

/// `EnMa3_GetTextId` on Malon as she is.
fn text_id(w: &mut PlayState) -> u16 {
    let h = malon(w);
    let mut actor = base(w, h).clone();
    w.cur_actor = Some(h);
    let id = EnMa3::get_text_id(w, &mut actor);
    w.cur_actor = None;
    id
}

/// Link `d` in front of `h` (along its facing), facing it.
fn before(w: &mut PlayState, h: ActorHandle, d: f32) {
    let a = base(w, h);
    let (p, yaw) = (a.world_pos, a.shape_rot.y);
    let r = yaw as f32 * std::f32::consts::PI / 32768.0;
    w.place_player(Vec3::new(p.x + d * r.sin(), p.y, p.z + d * r.cos()), yaw.wrapping_add(i16::MIN));
}

/// A until the talk starts (or it starts by itself), then A whenever the box waits (once each
/// wait) until it closes or `stop` holds. The texts shown.
fn talk(w: &mut PlayState, stop: impl Fn(&PlayState) -> bool) -> Vec<u16> {
    let a = PadState { button: BTN_A, ..Default::default() };
    let mut prev = PadState::default();
    let mut n = 0;
    while w.message_state() == TEXT_STATE_NONE {
        let pad = if prev.button == 0 { a } else { PadState::default() };
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        n += 1;
        assert!(n < 60, "the talk never started");
    }
    let mut texts = Vec::new();
    let mut pressed_at = None;
    for _ in 0..3000 {
        let (st, mode) = (w.message_state(), w.msg_ctx.msg_mode);
        if st != TEXT_STATE_NONE && texts.last() != Some(&w.msg_ctx.text_id) {
            texts.push(w.msg_ctx.text_id);
        }
        if (st == TEXT_STATE_NONE && mode == 0) || stop(w) {
            return texts;
        }
        if pressed_at.is_some_and(|p| p != (st, mode)) {
            pressed_at = None;
        }
        let waits = matches!(st, TEXT_STATE_AWAITING_NEXT | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_CHOICE | TEXT_STATE_EVENT) || mode == MSGMODE_TEXT_AWAIT_INPUT;
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

// Where she is.

#[test]
fn malon_is_by_the_corral_only_after_epona() {
    let Some(a) = assets() else { return };
    // spot20 layer 2: En_Ma3 -1 at (-158, 0, -1596). func_80AA2EC8: no
    // EVENTCHKINF_EPONA_OBTAINED, 2: killed.
    let w = enter(&a, "ENTR_LON_LON_RANCH_0", true, "adult-awake", |_| {});
    assert_eq!(any_live(&w, en_ma3::ACTOR_EN_MA3), 0);
    // With Epona: 0, ENMA3_ANIM_0 (gMalonAdultIdleAnim), func_80AA3200, scale 0.01, talk idle.
    let w = enter(&a, "ENTR_LON_LON_RANCH_0", true, "epona-won", |_| {});
    let h = malon(&w);
    let m = w.actors.downcast::<EnMa3>(h).unwrap();
    assert_eq!((m.actor.home_pos.x, m.actor.home_pos.z, m.actor.params, m.actor.scale.x), (-158.0, -1596.0, -1, 0.01));
    assert_eq!(m.action, en_ma3::Action::Func80aa3200);
    assert!(m.skel.is(oot_actors::en_ma2::IDLE_ANIM));
    assert_eq!(m.interact_info.talk_state, 0);
    // A child (LINK_IS_CHILD): 2, whatever the flags.
    assert_eq!(EnMa3::func_80aa2ec8(&{
        let mut w2 = enter(&a, "ENTR_LON_LON_RANCH_0", true, "epona-won", |_| {});
        w2.save.adult = false;
        w2
    }), 2);
    // The course's result due (EVENTINF_HORSES_0A): 1.
    let mut w = w;
    w.save.set_event_inf(en_ma3::EVENTINF_HORSES_0A);
    assert_eq!(EnMa3::func_80aa2ec8(&w), 1);
}

// Her texts.

#[test]
fn her_texts_follow_the_flags() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_LON_LON_RANCH_0", true, "epona-won", |_| {});
    // Not INFTABLE_B8: 0x2000, whatever else.
    horses_at(&mut w, Some(100.0));
    assert_eq!(text_id(&mut w), 0x2000);
    w.save.set_inf_table(en_ma3::INFTABLE_B8);
    // On foot, no horse within 1,200 (Actor_FindNearby, ACTORCAT_BG): 0x2001.
    horses_at(&mut w, None);
    assert_eq!(text_id(&mut w), 0x2001);
    // A horse within 1,200 (1,199 off): 0x2002 before INFTABLE_B9, 0x2003 after.
    horses_at(&mut w, Some(1199.0));
    assert_eq!(text_id(&mut w), 0x2002);
    w.save.set_inf_table(en_ma3::INFTABLE_B9);
    assert_eq!(text_id(&mut w), 0x2003);
    // 1,201 off: 0x2001 again.
    horses_at(&mut w, Some(1201.0));
    assert_eq!(text_id(&mut w), 0x2001);
    // Nothing set by the picking.
    let h = malon(&w);
    assert_eq!(base(&w, h).flags & ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED, 0);
}

#[test]
fn the_first_talk_thanks_link_and_the_course_starts() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_LON_LON_RANCH_0", true, "epona-won", |_| {});
    let h = malon(&w);
    horses_at(&mut w, Some(150.0));
    // The first talk: 0x2000; its closing sets INFTABLE_B8.
    before(&mut w, h, 60.0);
    idle(&mut w, 2);
    let texts = talk(&mut w, |_| false);
    assert_eq!(texts, vec![0x2000]);
    assert!(w.save.get_inf_table(en_ma3::INFTABLE_B8));
    assert!(!w.save.get_inf_table(en_ma3::INFTABLE_B9));
    // Epona near: 0x2002 runs on into 0x2003; "Let's go" (choiceIndex 0) sets INFTABLE_B9 and, with
    // no high score (HIGH_SCORE(HS_HORSE_RACE) 0), continues to 0x2092. Its event: to
    // ENTR_LON_LON_RANCH_0 with CS_INDEX_0, EVENTINF_HORSES_0A, TIMER_STATE_UP_FREEZE.
    assert_eq!(w.save.high_scores[HS], 0);
    idle(&mut w, 10);
    before(&mut w, h, 60.0);
    idle(&mut w, 2);
    let texts = talk(&mut w, |w| w.save.get_event_inf(en_ma3::EVENTINF_HORSES_0A));
    assert_eq!(texts, vec![0x2002, 0x2003, 0x2092]);
    assert!(w.save.get_inf_table(en_ma3::INFTABLE_B9));
    assert_eq!((w.transition.next_entrance_index, w.save.next_cutscene_index, w.transition.trigger), (ENTR_LON_LON_RANCH_0, 0xFFF0, TRANS_TRIGGER_START));
    // TRANS_TYPE_CIRCLE(TCA_STARBURST, TCC_BLACK, TCS_FAST).
    assert_eq!(w.transition.ty, 0x26);
    assert_eq!(w.save.timer_state, TIMER_STATE_UP_FREEZE);
}

/// Back from the course (`ENTR_LON_LON_RANCH_7`, as the referee sends Link) with `seconds` on the
/// timer and `high` the best, `cow` the cow won: Malon talks at once. The texts, and the save after.
fn result(a: &Arc<GameAssets>, seconds: i16, high: i32, cow: bool) -> (Vec<u16>, PlayState) {
    let mut w = enter(a, "ENTR_LON_LON_RANCH_7", true, "epona-won", |s| {
        s.set_inf_table(en_ma3::INFTABLE_B8);
        s.set_inf_table(en_ma3::INFTABLE_B9);
        s.high_scores[HS] = high;
        if cow {
            s.set_event_chk_inf(en_ma3::EVENTCHKINF_HORSE_RACE_COW_UNLOCK);
        }
    });
    // As the referee leaves it (EnHorseGameCheck_UpdateMalonRace): the course's flag, the time
    // frozen.
    w.save.set_event_inf(en_ma3::EVENTINF_HORSES_0A);
    w.save.timer_seconds = seconds;
    w.save.timer_state = TIMER_STATE_UP_FREEZE;
    let h = malon(&w);
    before(&mut w, h, 80.0);
    // EnMa3_GetTextId on the offer sets ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED: the talk starts by
    // itself.
    let mut started = false;
    for _ in 0..40 {
        idle(&mut w, 1);
        if w.message_state() != TEXT_STATE_NONE {
            started = true;
            break;
        }
    }
    assert!(started, "she didn't talk at once");
    let texts = talk(&mut w, |_| false);
    // Each closing: EVENTINF_HORSES_0A cleared, the auto-accept off, TIMER_STATE_STOP (which the
    // obedient Ingo, EnIn_Update, and the timer's own count turn off a few frames on).
    assert!(!w.save.get_event_inf(en_ma3::EVENTINF_HORSES_0A));
    assert_eq!(base(&w, h).flags & ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED, 0);
    assert!(matches!(w.save.timer_state, TIMER_STATE_STOP | TIMER_STATE_OFF), "{}", w.save.timer_state);
    (texts, w)
}

#[test]
fn the_courses_results_and_the_high_score() {
    let Some(a) = assets() else { return };
    // Over 210 (a foul: the referee's 240): 0x208E, before the high score is touched.
    let (texts, w) = result(&a, 240, 0, false);
    assert_eq!(texts, vec![0x208E]);
    assert_eq!(w.save.high_scores[HS], 0);
    assert!(!w.save.get_event_chk_inf(en_ma3::EVENTCHKINF_HORSE_RACE_COW_UNLOCK));
    // Under 50 without the cow: 0x208F; its closing sets EVENTCHKINF_HORSE_RACE_COW_UNLOCK and
    // keeps the best (180, from 0, then 45).
    let (texts, w) = result(&a, 45, 0, false);
    assert_eq!(texts, vec![0x208F]);
    assert!(w.save.get_event_chk_inf(en_ma3::EVENTCHKINF_HORSE_RACE_COW_UNLOCK));
    assert_eq!(w.save.high_scores[HS], 45);
    // Under 50 with the cow won: better than the best (60): 0x2012, the best 45.
    let (texts, w) = result(&a, 45, 60, true);
    assert_eq!(texts, vec![0x2012]);
    assert_eq!(w.save.high_scores[HS], 45);
    // Not better than the best: 0x2004, the best kept.
    let (texts, w) = result(&a, 80, 70, true);
    assert_eq!(texts, vec![0x2004]);
    assert_eq!(w.save.high_scores[HS], 70);
    // A best over 180 is made 180 first: 100 is better, 0x2012.
    let (texts, w) = result(&a, 100, 200, false);
    assert_eq!(texts, vec![0x2012]);
    assert_eq!(w.save.high_scores[HS], 100);
}
