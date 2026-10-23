//! GAME-10 milestone 9 against the C: Bonooru (`z_en_kakasi3.c`), Lake Hylia's room 0 (a child's
//! layer at (-154, -1293, 2888), an adult's at (-164, -1293, 2914)). His texts by
//! `scarecrowSpawnSongSet`; a child's recording taken and refused; the adult's check right
//! (`EVENTCHKINF_9C` as its text closes) and wrong; a child's playback with 2280. Each state
//! driven directly: the ocarina's and the message box's modes set by hand, his update run alone.
//!
//! Entered from `fairy-magic` with the lake's intro seen. Expected values are worked out from the
//! C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_kakasi::{PLAYER_CSACTION_7, PLAYER_CSACTION_8};
use oot_actors::en_kakasi3::{Action, EVENTCHKINF_9C, EnKakasi3};
use oot_actors::player::{STATE2_23, STATE2_24};
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::ocarina::OCARINA_BTN_C_RIGHT;
use oot_game::camera::{CAM_ID_NONE, GameCamera};
use oot_game::message::*;
use oot_game::onepoint::CAM_SET_CS_C;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

/// `EVENTCHKINF_B1` (`save.h`): Lake Hylia's intro seen.
const EVENTCHKINF_B1: u16 = 0xB1;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Lake Hylia (`ENTR_LAKE_HYLIA_0`) at 10:00 from `fairy-magic`, `tweak` last; eight frames on.
fn lake(a: &Arc<GameAssets>, adult: bool, tweak: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_LAKE_HYLIA_0").unwrap();
    let mut save = SaveContext::new(e, adult, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("fairy-magic").unwrap();
    save.adult = adult;
    save.set_event_chk_inf(EVENTCHKINF_B1);
    tweak(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap();
    for _ in 0..8 {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
    w
}

fn b(w: &mut PlayState, h: ActorHandle) -> &mut EnKakasi3 {
    w.actors.downcast_mut::<EnKakasi3>(h).expect("En_Kakasi3")
}

/// Bonooru at `home`, two updates run (`func_80A911F0`, `func_80A91284`: culled far from Link).
fn find(w: &mut PlayState, home: Vec3) -> ActorHandle {
    let h = *w.actors.all().iter().find(|&&h| w.actors.downcast::<EnKakasi3>(h).is_some_and(|k| k.actor.home_pos == home && !k.actor.killed)).expect("En_Kakasi3");
    assert_eq!(b(w, h).action, Action::Func80a911f0);
    update(w, h);
    update(w, h);
    assert_eq!(b(w, h).action, Action::Func80a91348);
    h
}

const CHILD_HOME: Vec3 = Vec3::new(-154.0, -1293.0, 2888.0);
const ADULT_HOME: Vec3 = Vec3::new(-164.0, -1293.0, 2914.0);

fn update(w: &mut PlayState, h: ActorHandle) {
    w.input = scripted_input(PadState::default(), PadState::default());
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// The update with A pressed this frame (`Message_ShouldAdvance`).
fn update_a(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.input = scripted_input(PadState::default(), PadState { button: BTN_A, ..Default::default() });
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
    w.input = scripted_input(PadState::default(), PadState::default());
}

/// The box at its event (`TEXT_STATE_EVENT`), then A.
fn advance(w: &mut PlayState, h: ActorHandle) {
    w.msg_ctx.msg_length = 1;
    w.msg_ctx.msg_mode = MSGMODE_TEXT_DONE;
    w.msg_ctx.textbox_end_type = TEXTBOX_ENDTYPE_EVENT;
    update_a(w, h);
}

/// Link's ocarina out before him, in front, 70 across.
fn ocarina_out(w: &mut PlayState, h: ActorHandle) {
    w.player_mut().state2 |= STATE2_24;
    let s = b(w, h);
    s.actor.xz_dist_to_player = 70.0;
    s.actor.yaw_towards_player = s.actor.shape_rot.y;
}

fn onepoint_cam(w: &PlayState, cs_id: i16) -> Option<&GameCamera> {
    w.sub_cameras.iter().flatten().find(|c| c.cs_id == cs_id)
}

/// His text after `func_80A91284` again.
fn text(w: &mut PlayState, h: ActorHandle) -> (u16, u8) {
    b(w, h).action = Action::Func80a91284;
    update(w, h);
    let s = b(w, h);
    (s.actor.text_id, s.dialog_state)
}

#[test]
fn his_texts_by_age_and_the_song() {
    let Some(a) = assets() else { return };
    let mut w = lake(&a, false, |_| {});
    let h = find(&mut w, CHILD_HOME);
    // EnKakasi3_Init: scale 0.01, attention range 6, immovable.
    let s = b(&mut w, h);
    assert_eq!((s.actor.scale, s.actor.target_mode, s.actor.col_chk_info.mass), (Vec3::splat(0.01), 6, 0xFF));
    // A child, no song: 0x40A1, TEXT_STATE_DONE; the song set: 0x40A0, an event, unk_1A8 1.
    assert_eq!(text(&mut w, h), (0x40A1, TEXT_STATE_DONE));
    assert_eq!(b(&mut w, h).unk_1a8, 0);
    w.save.scarecrow_spawn_song_set = 1;
    assert_eq!(text(&mut w, h), (0x40A0, TEXT_STATE_EVENT));
    assert_eq!(b(&mut w, h).unk_1a8, 1);

    let mut w = lake(&a, true, |_| {});
    let h = find(&mut w, ADULT_HOME);
    // An adult, no song: 0x40A1; the song set: 0x40A3, then 0x40A2 once he's heard it (unk_195);
    // both TEXT_STATE_DONE.
    assert_eq!(text(&mut w, h), (0x40A1, TEXT_STATE_DONE));
    assert!(b(&mut w, h).unk_194);
    w.save.scarecrow_spawn_song_set = 1;
    assert_eq!(text(&mut w, h), (0x40A3, TEXT_STATE_DONE));
    b(&mut w, h).unk_195 = true;
    assert_eq!(text(&mut w, h), (0x40A2, TEXT_STATE_DONE));
}

#[test]
fn a_childs_spawn_song_recorded() {
    let Some(a) = assets() else { return };
    let mut w = lake(&a, false, |_| {});
    let h = find(&mut w, CHILD_HOME);
    ocarina_out(&mut w, h);
    update(&mut w, h);
    // 2260, the box paused then 0x40A4, an event; Link's ocarina turned to him.
    let s = b(&mut w, h);
    assert_eq!((s.action, s.dialog_state, s.dance.unk_1b8), (Action::Func80a915b8, TEXT_STATE_EVENT, 0.0));
    let sub = s.sub_cam_id;
    let cam = onepoint_cam(&w, 2260).expect("2260's camera");
    assert_eq!((cam.cam_id, cam.setting), (sub, CAM_SET_CS_C));
    assert_eq!(w.msg_ctx.text_id, 0x40A4);
    assert_ne!(w.player().state2 & STATE2_23, 0);
    // Not at its event: waiting.
    update_a(&mut w, h);
    assert_eq!(b(&mut w, h).action, Action::Func80a915b8);
    // At its event, A: the spawn song's recording.
    advance(&mut w, h);
    assert_eq!(b(&mut w, h).action, Action::Func80a91620);
    assert_eq!(w.msg_ctx.ocarina_action, OCARINA_ACTION_SCARECROW_SPAWN_RECORDING);
    // Link playing: C-Right rocks him on z.
    w.msg_ctx.ocarina_mode = OCARINA_MODE_01;
    w.msg_ctx.last_ocarina_button_index = OCARINA_BTN_C_RIGHT;
    update(&mut w, h);
    let s = b(&mut w, h);
    assert_eq!((s.dance.unk_19a, s.dance.rock_z, s.actor.shape_rot.z), (1, 0x1388, 0x3E8));
    // Taken (OCARINA_MODE_03, the box closed): 0x40A5, Link held.
    w.msg_ctx.ocarina_mode = OCARINA_MODE_03;
    w.msg_ctx.msg_mode = MSGMODE_NONE;
    update(&mut w, h);
    assert_eq!(b(&mut w, h).action, Action::Func80a91a90);
    assert_eq!(w.msg_ctx.text_id, 0x40A5);
    assert_eq!(w.player().cs_mode, PLAYER_CSACTION_8);
    // Its event, A: no EVENTCHKINF_9C (a child: unk_195 never set), the song's mode to 4, Link
    // let go, the camera told it's done.
    advance(&mut w, h);
    assert_eq!(b(&mut w, h).action, Action::Func80a911f0);
    assert!(!w.save.get_event_chk_inf(EVENTCHKINF_9C));
    assert_eq!(w.msg_ctx.ocarina_mode, OCARINA_MODE_04);
    assert_eq!(w.player().cs_mode, PLAYER_CSACTION_7);
    assert_eq!(w.msg_ctx.msg_mode, MSGMODE_TEXT_CLOSING);
}

#[test]
fn a_childs_spawn_song_refused() {
    let Some(a) = assets() else { return };
    for mode in [OCARINA_MODE_04, 0x05, 0x0A] {
        let mut w = lake(&a, false, |_| {});
        let h = find(&mut w, CHILD_HOME);
        ocarina_out(&mut w, h);
        update(&mut w, h);
        advance(&mut w, h);
        let sub = b(&mut w, h).sub_cam_id;
        // OCARINA_MODE_0B (not refused): still recording.
        w.msg_ctx.ocarina_mode = OCARINA_MODE_0B;
        w.msg_ctx.msg_mode = MSGMODE_NONE;
        update(&mut w, h);
        assert_eq!(b(&mut w, h).action, Action::Func80a91620);
        // 4, or 5 to 10, the box closed: the cutscene ended (timer 0), back to standing.
        w.msg_ctx.ocarina_mode = mode;
        update(&mut w, h);
        assert_eq!(b(&mut w, h).action, Action::Func80a911f0, "mode {mode}");
        assert_eq!(w.camera(sub).map(|c| c.timer), Some(0));
        assert_eq!(w.player().cs_mode, 0);
    }
}

#[test]
fn the_adult_plays_the_song_back_and_pierre_may_come() {
    let Some(a) = assets() else { return };
    let mut w = lake(&a, true, |s| s.scarecrow_spawn_song_set = 1);
    let h = find(&mut w, ADULT_HOME);
    ocarina_out(&mut w, h);
    update(&mut w, h);
    // The song set, not yet heard: 2260 and 0x40A8.
    assert_eq!(b(&mut w, h).action, Action::Func80a9187c);
    assert!(onepoint_cam(&w, 2260).is_some());
    assert_eq!(w.msg_ctx.text_id, 0x40A8);
    advance(&mut w, h);
    assert_eq!(b(&mut w, h).action, Action::Func80a918e4);
    assert_eq!(w.msg_ctx.ocarina_action, OCARINA_ACTION_CHECK_SCARECROW_SPAWN);
    // Right (OCARINA_MODE_03): heard, 0x40A7, Link held. (Message_StartTextbox puts the mode the
    // C set to 4 back to 0.)
    w.msg_ctx.ocarina_mode = OCARINA_MODE_03;
    w.msg_ctx.msg_mode = MSGMODE_NONE;
    update(&mut w, h);
    let s = b(&mut w, h);
    assert_eq!((s.action, s.unk_195, s.dialog_state), (Action::Func80a91a90, true, TEXT_STATE_EVENT));
    assert_eq!(w.msg_ctx.text_id, 0x40A7);
    assert_eq!(w.player().cs_mode, PLAYER_CSACTION_8);
    assert!(!w.save.get_event_chk_inf(EVENTCHKINF_9C));
    // Its event, A: EVENTCHKINF_9C, the mode 4, Link let go.
    advance(&mut w, h);
    assert!(w.save.get_event_chk_inf(EVENTCHKINF_9C));
    assert_eq!(w.msg_ctx.ocarina_mode, OCARINA_MODE_04);
    assert_eq!(w.player().cs_mode, PLAYER_CSACTION_7);
    // Back to standing: 0x40A2; the ocarina out no longer asks.
    update(&mut w, h);
    update(&mut w, h);
    assert_eq!(b(&mut w, h).actor.text_id, 0x40A2);
    ocarina_out(&mut w, h);
    update(&mut w, h);
    assert_eq!(b(&mut w, h).action, Action::Func80a91348);
}

#[test]
fn the_adults_wrong_song() {
    let Some(a) = assets() else { return };
    let mut w = lake(&a, true, |s| s.scarecrow_spawn_song_set = 1);
    let h = find(&mut w, ADULT_HOME);
    ocarina_out(&mut w, h);
    update(&mut w, h);
    advance(&mut w, h);
    let sub = b(&mut w, h).sub_cam_id;
    // 5 (another song), the box closed: 0x40A6, the cutscene ended, Link held.
    w.msg_ctx.ocarina_mode = 0x05;
    w.msg_ctx.msg_mode = MSGMODE_NONE;
    update(&mut w, h);
    let s = b(&mut w, h);
    assert_eq!((s.action, s.unk_195, s.sub_cam_id), (Action::Func80a91a90, false, CAM_ID_NONE));
    assert_eq!(w.msg_ctx.text_id, 0x40A6);
    assert_eq!(w.camera(sub).map(|c| c.timer), Some(0));
    advance(&mut w, h);
    assert!(!w.save.get_event_chk_inf(EVENTCHKINF_9C));
    assert_eq!(w.player().cs_mode, PLAYER_CSACTION_7);
    assert_eq!(b(&mut w, h).action, Action::Func80a911f0);
}

#[test]
fn a_child_hears_the_song_played_back_with_2280() {
    let Some(a) = assets() else { return };
    let mut w = lake(&a, false, |s| s.scarecrow_spawn_song_set = 1);
    let h = find(&mut w, CHILD_HOME);
    // Talked to (0x40A0's event): func_80A91760.
    b(&mut w, h).action = Action::Func80a91760;
    advance(&mut w, h);
    let s = b(&mut w, h);
    assert_eq!(s.action, Action::Func80a917fc);
    let sub = s.sub_cam_id;
    let cam = onepoint_cam(&w, 2280).expect("2280's camera");
    assert_eq!((cam.cam_id, cam.setting, cam.cs_info.key_frame_cnt), (sub, CAM_SET_CS_C, 7));
    assert_eq!(cam.cs_info.key_frames.map(|k| (k.table, k.start)), Some((w.onepoint.table("D_80120D4C"), 0)));
    // D_80120D4C's first four rows but the 0x8F one (0) from Rand: at.x 0 to 20, at.z 10 to 50,
    // eye.x 60 to 100, eye.z 80 to 120; row [subCamId - 1]'s eye.y ±3 plus Rand (@bug (game)).
    let t = w.onepoint.table("D_80120D4C");
    for i in 1..4 {
        let k = w.onepoint.keyframes[t][i];
        assert!((0.0..20.0).contains(&k.at_target_init.x) && (10.0..50.0).contains(&k.at_target_init.z), "row {i} at {}", k.at_target_init);
        assert!((60.0..100.0).contains(&k.eye_target_init.x) && (80.0..120.0).contains(&k.eye_target_init.z), "row {i} eye {}", k.eye_target_init);
    }
    assert_eq!(w.onepoint.keyframes[t][0].at_target_init.x, 20.0);
    let y = w.onepoint.keyframes[t][(sub - 1) as usize].eye_target_init.y;
    assert!((3.0..4.0).contains(&y) || (-3.0..-2.0).contains(&y), "{y}");
    // QUAKE_TYPE_5, speed 400, (2, 3, 200, 0x32), 9999 frames.
    let q = w.quake.requests.iter().find(|r| r.cam_id == sub).expect("2280's quake");
    assert_eq!((q.type_, q.speed, q.y, q.x, q.fov, q.up_pitch_offset, q.duration), (oot_game::quake::QUAKE_TYPE_5, 400, 2, 3, 200, 0x32, 9999));
    assert_eq!(w.msg_ctx.ocarina_action, OCARINA_ACTION_SCARECROW_SPAWN_PLAYBACK);
    // Dancing by Rand until OCARINA_MODE_0F.
    w.msg_ctx.ocarina_mode = OCARINA_MODE_01;
    update(&mut w, h);
    assert!((5..=8).contains(&b(&mut w, h).dance.unk_19a));
    // Over: the mode 4, the box closed, the cutscene ended.
    w.msg_ctx.ocarina_mode = OCARINA_MODE_0F;
    update(&mut w, h);
    assert_eq!(b(&mut w, h).action, Action::Func80a911f0);
    assert_eq!(w.msg_ctx.ocarina_mode, OCARINA_MODE_04);
    assert_eq!(w.camera(sub).map(|c| c.timer), Some(0));
}
