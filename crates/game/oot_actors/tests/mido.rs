//! Mido (`En_Md`, `ovl_En_Md/z_en_md.c`) in Kokiri Forest: room 0's placement 0x0100 at
//! (1522, 0, 105), path 1 ((1522, 0, 105) to (1412, 0, 211)).
//!
//! Expected values from the C:
//! - `EnMd_BlockPath`: blocking, 60 from his home towards Link, facing him;
//! - `EnMd_GetTextIdKokiriForest`: 0x102F, 0x1030 once `INFTABLE_0C` is set, 0x1033 with the Deku
//!   Shield and the Kokiri Sword worn, 0x1034 with `EVENTCHKINF_04`;
//! - `EnMd_UpdateTalkState`: 0x102F sets `EVENTCHKINF_MIDO_DENIED_DEKU_TREE_ACCESS` and `INFTABLE_0C` as it closes; 0x1033
//!   returns 2, and `EnMd_BlockPath` sets `EVENTCHKINF_04` and walks him along path 1 at 1.5;
//! - `EnMd_Walk`: within 10 of path 1's last point he stops (`EnMd_Watch`);
//! - `EnMd_Init`: with `EVENTCHKINF_04`, at path 1's last point (`EnMd_SetMovedPos`);
//! - `EnMd_ShouldSpawn`: not in Kokiri Forest once Link has Zelda's letter (`EVENTCHKINF_OBTAINED_ZELDAS_LETTER`)
//!   or has said goodbye (`EVENTCHKINF_1C`);
//! - `EnMd_UpdateAlphaByDistance` / `Actor_UpdateAlphaByDistance`: beyond 400 of Link he fades out 20 a step, and can't be
//!   targeted.

mod common;

use std::sync::Arc;

use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_md::{self, Action, EnMd};
use oot_actors::script::stick_towards;
use oot_game::actor::ACTOR_FLAG_ATTENTION_ENABLED;
use oot_game::message::{MSGMODE_TEXT_AWAIT_INPUT, TEXT_STATE_AWAITING_NEXT, TEXT_STATE_CHOICE, TEXT_STATE_DONE, TEXT_STATE_DONE_HAS_NEXT, TEXT_STATE_NONE};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::{EVENTCHKINF_04, EVENTCHKINF_OBTAINED_ZELDAS_LETTER, SaveContext};

const HOME: Vec3 = Vec3::new(1522.0, 0.0, 105.0);
const PATH_END: Vec3 = Vec3::new(1412.0, 0.0, 211.0);

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Kokiri Forest on a new save changed by `f`, a few frames in.
fn forest(f: impl FnOnce(&mut SaveContext)) -> Option<(PlayState, PadState)> {
    let a = assets()?;
    let e = a.scenes.entrance_index("ENTR_KOKIRI_FOREST_0").unwrap();
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    f(&mut save);
    let mut w = oot_actors::play_entrance(a, common::data()?, common::rules()?, save).expect("Play_Init");
    let mut prev = PadState::default();
    step(&mut w, &mut prev, PadState::default(), 5);
    Some((w, prev))
}

fn step(w: &mut PlayState, prev: &mut PadState, pad: PadState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(*prev, pad));
        *prev = pad;
    }
}

fn mido(w: &PlayState) -> Option<&EnMd> {
    w.actors.all().into_iter().find_map(|h| w.actors.downcast::<EnMd>(h))
}

fn xz(a: Vec3, b: Vec3) -> f32 {
    Vec3::new(a.x - b.x, 0.0, a.z - b.z).length()
}

/// Walks at Mido until he offers to talk, presses A, then A whenever the box waits until it
/// closes. Returns the texts shown.
fn talk(w: &mut PlayState, prev: &mut PadState) -> Vec<u16> {
    let h = w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnMd>(h).is_some()).expect("Mido");
    for _ in 0..60 {
        if w.player().target_actor == Some(h) {
            break;
        }
        let at = mido(w).unwrap().actor.world_pos;
        let pad = stick_towards(w, at, 40.0);
        step(w, prev, pad, 1);
    }
    assert_eq!(w.player().target_actor, Some(h), "Mido offers to talk");
    let mut texts = Vec::new();
    for i in 0..3000 {
        let st = w.message_state();
        if st != TEXT_STATE_NONE && texts.last() != Some(&w.msg_ctx.text_id) {
            texts.push(w.msg_ctx.text_id);
        }
        if i > 5 && st == TEXT_STATE_NONE && w.msg_ctx.msg_mode == 0 {
            return texts;
        }
        let waits = matches!(st, TEXT_STATE_AWAITING_NEXT | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_CHOICE) || w.msg_ctx.msg_mode == MSGMODE_TEXT_AWAIT_INPUT;
        let pad = if (i == 0 || waits) && prev.button == 0 { PadState { button: BTN_A, ..Default::default() } } else { PadState::default() };
        step(w, prev, pad, 1);
    }
    panic!("the talk didn't end: {texts:x?}");
}

#[test]
fn mido_blocks_the_way_and_his_first_talk_sets_its_flags() {
    let Some((mut w, mut prev)) = forest(|_| {}) else { return };
    let m = mido(&w).expect("Mido spawns on a new save");
    assert_eq!((m.action, m.actor.home_pos, m.actor.scale), (Action::Blocking, HOME, Vec3::splat(0.01)));
    assert_eq!(m.actor.target_mode, 6);
    // Link west of him: Mido 60 from home towards Link, facing him.
    w.place_player(Vec3::new(1400.0, 0.0, 150.0), 0x4000);
    step(&mut w, &mut prev, PadState::default(), 3);
    let m = mido(&w).unwrap();
    let link = w.player().actor.world_pos;
    assert!((xz(m.actor.world_pos, HOME) - 60.0).abs() < 0.01);
    let yaw = oot_game::target::yaw_to(HOME, link);
    let expect = HOME + Vec3::new(60.0 * eng_math::sin_s(yaw), 0.0, 60.0 * eng_math::cos_s(yaw));
    assert!(xz(m.actor.world_pos, expect) < 0.01, "{} vs {expect}", m.actor.world_pos);
    assert_eq!(m.actor.shape_rot.y, m.actor.yaw_towards_player);

    // 0x102F and its next texts (0x10D0, 0x10D1, 0x1030); closed: EVENTCHKINF_MIDO_DENIED_DEKU_TREE_ACCESS, INFTABLE_0C.
    let texts = talk(&mut w, &mut prev);
    assert_eq!(texts, [0x102F, 0x10D0, 0x10D1, 0x1030]);
    assert!(w.save.get_event_chk_inf(en_md::EVENTCHKINF_MIDO_DENIED_DEKU_TREE_ACCESS));
    assert!(w.save.get_inf_table(en_md::INFTABLE_0C));
    assert!(!w.save.get_event_chk_inf(EVENTCHKINF_04));
    let m = mido(&w).unwrap();
    assert_eq!((m.action, m.interact_info.talk_state), (Action::Blocking, 0));
    // Again: 0x1030 only.
    step(&mut w, &mut prev, PadState::default(), 15);
    assert_eq!(talk(&mut w, &mut prev), [0x1030]);
    assert_eq!(mido(&w).unwrap().action, Action::Blocking);
}

#[test]
fn his_gestures_follow_the_texts_boxes() {
    let Some((mut w, mut prev)) = forest(|_| {}) else { return };
    w.place_player(Vec3::new(1400.0, 0.0, 150.0), 0x4000);
    step(&mut w, &mut prev, PadState::default(), 3);
    let h = w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnMd>(h).is_some()).unwrap();
    for _ in 0..60 {
        if w.player().target_actor == Some(h) {
            break;
        }
        let pad = stick_towards(&w, mido(&w).unwrap().actor.world_pos, 40.0);
        step(&mut w, &mut prev, pad, 1);
    }
    step(&mut w, &mut prev, PadState { button: BTN_A, ..Default::default() }, 1);
    step(&mut w, &mut prev, PadState::default(), 2);
    // EnMd_UpdateAnimSequence_WithTalking, 0x102F at its first box (messageEntry 0): sequence 1, gMidoIdleToHaltAnim then
    // gMidoHaltAnim (EnMd_UpdateAnimSequence_IdleToHalt).
    let m = mido(&w).unwrap();
    assert_eq!((m.interact_info.talk_state, m.message_entry, m.anim_sequence), (1, 0, 1));
    assert!(m.skel.is("gMidoIdleToHaltAnim"));
    for _ in 0..40 {
        if mido(&w).unwrap().skel.is("gMidoHaltAnim") {
            break;
        }
        step(&mut w, &mut prev, PadState::default(), 1);
    }
    assert!(mido(&w).unwrap().skel.is("gMidoHaltAnim"));
}

#[test]
fn with_the_sword_and_shield_worn_he_steps_aside() {
    let Some((mut w, mut prev)) = forest(oot_game::save::kokiri_sword_and_deku_shield) else { return };
    w.place_player(Vec3::new(1420.0, 0.0, 120.0), 0x4000);
    step(&mut w, &mut prev, PadState::default(), 3);
    let texts = talk(&mut w, &mut prev);
    assert_eq!(texts, [0x1033, 0x10D2, 0x10D3, 0x1034]);
    // EnMd_BlockPath on EnMd_UpdateTalkState's 2: EVENTCHKINF_04, waypoint 1, speed 1.5, walking.
    assert!(w.save.get_event_chk_inf(EVENTCHKINF_04));
    let m = mido(&w).unwrap();
    assert_eq!((m.action, m.waypoint, m.actor.speed_xz, m.interact_info.talk_state), (Action::Walking, 1, 1.5, 0));
    let mut walked = Vec::new();
    for _ in 0..300 {
        let m = mido(&w).unwrap();
        if m.action == Action::Arrived {
            break;
        }
        walked.push(m.actor.world_pos);
        step(&mut w, &mut prev, PadState::default(), 1);
    }
    let m = mido(&w).unwrap();
    // EnMd_Walk: within 10 of path 1's last point (EnMd_FollowPath), waypoint back to 0,
    // stopped (speed 0, play speed 0), home there (EnMd_Watch).
    assert_eq!(m.action, Action::Arrived);
    assert!(xz(m.actor.world_pos, PATH_END) < 10.0, "{}", m.actor.world_pos);
    assert_eq!((m.waypoint, m.actor.speed_xz, m.skel.play_speed, m.actor.home_pos), (0, 0.0, 0.0, m.actor.world_pos));
    // speedXZ 1.5: Actor_UpdatePos moves velocity * (R_UPDATE_RATE * 0.5f), 2.25 a frame.
    for p in walked.windows(2) {
        assert!((xz(p[0], p[1]) - 2.25).abs() < 0.01, "{} -> {}", p[0], p[1]);
    }
    // Now 0x1034.
    step(&mut w, &mut prev, PadState::default(), 15);
    assert_eq!(talk(&mut w, &mut prev), [0x1034]);
    assert_eq!(mido(&w).unwrap().action, Action::Arrived);
}

#[test]
fn with_eventchkinf_04_he_starts_at_his_paths_end() {
    let Some((mut w, mut prev)) = forest(|s| {
        oot_game::save::kokiri_sword_and_deku_shield(s);
        s.set_event_chk_inf(EVENTCHKINF_04);
    }) else {
        return;
    };
    let m = mido(&w).expect("Mido");
    // EnMd_SetMovedPos, EnMd_Idle.
    assert_eq!((m.action, m.actor.world_pos), (Action::Moved, PATH_END));
    w.place_player(Vec3::new(1380.0, 0.0, 160.0), 0x2000);
    step(&mut w, &mut prev, PadState::default(), 3);
    assert_eq!(talk(&mut w, &mut prev), [0x1034]);
}

#[test]
fn he_fades_out_beyond_400_and_comes_back() {
    let Some((mut w, mut prev)) = forest(|_| {}) else { return };
    // Link spawns at Kokiri Forest's spawn 0, far away: Actor_UpdateAlphaByDistance steps the alpha down by
    // 0x14 a frame (Math_SmoothStepToS(&alpha, 0, 6, 0x14, 1)), and clears ACTOR_FLAG_ATTENTION_ENABLED.
    w.place_player(Vec3::new(0.0, 0.0, 0.0), 0);
    step(&mut w, &mut prev, PadState::default(), 1);
    let a0 = mido(&w).unwrap().alpha;
    step(&mut w, &mut prev, PadState::default(), 1);
    let m = mido(&w).unwrap();
    assert_eq!(m.alpha, (a0 - 0x14).max(0));
    assert_eq!(m.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    // (A smooth step: 0x14 at most, a sixth of what's left, at least 1.)
    step(&mut w, &mut prev, PadState::default(), 60);
    assert_eq!(mido(&w).unwrap().alpha, 0);
    // Near: back up, targetable.
    w.place_player(Vec3::new(1400.0, 0.0, 150.0), 0x4000);
    step(&mut w, &mut prev, PadState::default(), 2);
    let m = mido(&w).unwrap();
    assert!(m.alpha > 0);
    assert_ne!(m.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
}

#[test]
fn en_md_should_spawn() {
    // EnMd_ShouldSpawn: in Kokiri Forest only before Zelda's letter (EVENTCHKINF_OBTAINED_ZELDAS_LETTER) and the
    // goodbye (EVENTCHKINF_1C).
    let Some((w, _)) = forest(|s| s.set_event_chk_inf(EVENTCHKINF_OBTAINED_ZELDAS_LETTER)) else { return };
    assert!(mido(&w).is_none(), "killed in his init");
    let Some((w, _)) = forest(|s| s.set_event_chk_inf(en_md::EVENTCHKINF_1C)) else { return };
    assert!(mido(&w).is_none());
}

#[test]
fn his_bakes_by_eye_and_pass() {
    // EnMd_Draw: sEyeTextures on segment 8, opaque at alpha 255 (func_80034BA0), translucent
    // otherwise (func_80034CC4).
    let names: Vec<String> = en_md::bakes().into_iter().map(|b| b.name).collect();
    assert_eq!(names, ["En_Md/opa_eye0", "En_Md/xlu_eye0", "En_Md/opa_eye1", "En_Md/xlu_eye1", "En_Md/opa_eye2", "En_Md/xlu_eye2"]);
}
