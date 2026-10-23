//! GAME-10 milestone 9 against the C: the long song's scarecrow (`z_en_kakasi.c`, Lake Hylia's
//! room 0 at (12, -1253, 2664)) and the one-point cutscenes 2260, 2270 and 2280
//! (`z_onepointdemo.c`) the scarecrows start. Each state driven directly: the ocarina's and the
//! message box's modes set by hand (`ocarinaMode`, `msgMode`, `lastOcarinaButtonIndex`), the
//! scarecrow's update run alone.
//!
//! Entered from `fairy-magic` with the lake's intro seen. Expected values are worked out from the
//! C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_kakasi::{self as kakasi, Action, EnKakasi};
use oot_actors::player::{STATE2_23, STATE2_24};
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::ocarina::{OCARINA_BTN_A, OCARINA_BTN_C_LEFT, OCARINA_BTN_C_UP};
use oot_game::camera::{CAM_ID_NONE, GameCamera};
use oot_game::message::*;
use oot_game::onepoint::CAM_SET_CS_C;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

/// `EVENTCHKINF_B1` (`save.h`): Lake Hylia's intro seen.
const EVENTCHKINF_B1: u16 = 0xB1;
/// Where Lake Hylia's room 0 places him.
const HOME: Vec3 = Vec3::new(12.0, -1253.0, 2664.0);

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// A child's Lake Hylia (`ENTR_LAKE_HYLIA_0`) at 10:00 from `fairy-magic`, `tweak` last; eight
/// frames on.
fn lake(a: &Arc<GameAssets>, tweak: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_LAKE_HYLIA_0").unwrap();
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("fairy-magic").unwrap();
    save.adult = false;
    save.set_event_chk_inf(EVENTCHKINF_B1);
    tweak(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap();
    for _ in 0..8 {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
    w
}

/// Him, his first update run (`func_80A8F660`: far from Link at the lake's entrance he's culled).
fn find(w: &mut PlayState) -> ActorHandle {
    let h = *w.actors.all().iter().find(|&&h| w.actors.downcast::<EnKakasi>(h).is_some_and(|k| k.actor.home_pos == HOME && !k.actor.killed)).expect("En_Kakasi");
    assert_eq!(k(w, h).action, Action::Func80a8f660);
    update(w, h);
    h
}

fn k(w: &mut PlayState, h: ActorHandle) -> &mut EnKakasi {
    w.actors.downcast_mut::<EnKakasi>(h).expect("En_Kakasi")
}

/// The actor's update alone (`Actor_UpdateAll`'s call), as the updating actor; no button.
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

/// A box at its end, of `end_type` (`TEXTBOX_ENDTYPE_EVENT`: `TEXT_STATE_EVENT`).
fn box_done(w: &mut PlayState, end_type: u8) {
    w.msg_ctx.msg_length = 1;
    w.msg_ctx.msg_mode = MSGMODE_TEXT_DONE;
    w.msg_ctx.textbox_end_type = end_type;
}

/// Link before him, in front (`yawTowardsPlayer` his own yaw), `dist` across.
fn link_before(w: &mut PlayState, h: ActorHandle, dist: f32) {
    let s = k(w, h);
    s.actor.xz_dist_to_player = dist;
    s.actor.yaw_towards_player = s.actor.shape_rot.y;
}

fn onepoint_cam(w: &PlayState, cs_id: i16) -> Option<&GameCamera> {
    w.sub_cameras.iter().flatten().find(|c| c.cs_id == cs_id)
}

/// z_onepointdemo.c's rows: (actionFlags, initField (0xFF none), initFlags, timer, roll, fov,
/// lerp, at, eye).
type Row = (u8, u8, i16, i16, i16, f32, f32, Vec3, Vec3);
fn row(w: &PlayState, t: &str, i: usize) -> Row {
    let k = w.onepoint.keyframes[w.onepoint.table(t)][i];
    (k.action_flags, k.init_field, k.init_flags, k.timer_init, k.roll_target_init, k.fov_target_init, k.lerp_step_scale, k.at_target_init, k.eye_target_init)
}

#[test]
fn the_pack_has_the_scarecrow_and_the_one_points_keyframes() {
    let Some(a) = assets() else { return };
    assert!(a.pack.mesh(kakasi::OBJECT, kakasi::SKEL).is_ok());
    assert!(a.skeleton(kakasi::OBJECT, kakasi::SKEL).is_ok());
    assert!(a.animation(kakasi::OBJECT, kakasi::ANIM).is_ok());
    let w = lake(&a, |_| {});
    let m1 = Vec3::splat(-1.0);
    // D_80120ACC (2260): five rows, ONEPOINT_CS_ACTION(ID_15, false, true) = 0x8F, ID_21 0x95,
    // ID_17 0x11; the third holds Link (PLAYER_CSACTION_1).
    assert_eq!(w.onepoint.len(w.onepoint.table("D_80120ACC")), 5);
    assert_eq!(row(&w, "D_80120ACC", 0), (0x8F, 0xFF, 0x0442, 10, 0, 40.0, 1.0, Vec3::new(-10.0, 45.0, 20.0), Vec3::new(20.0, 30.0, 160.0)));
    assert_eq!(row(&w, "D_80120ACC", 1), (0x95, 0xFF, 0x0000, 1, 0, 40.0, 1.0, m1, m1));
    assert_eq!(row(&w, "D_80120ACC", 2), (0x8F, 1, 0x0442, 10, 0, 40.0, 1.0, Vec3::new(-10.0, 45.0, 20.0), Vec3::new(20.0, 30.0, 160.0)));
    assert_eq!(row(&w, "D_80120ACC", 4), (0x11, 0xFF, 0x0000, 1, 0, 60.0, 1.0, m1, m1));
    // D_80120B94 (2270): eleven; ID_4 | 0x80 = 0x84, ID_13 0x8D, ID_24 0x98.
    assert_eq!(w.onepoint.len(w.onepoint.table("D_80120B94")), 11);
    assert_eq!(row(&w, "D_80120B94", 0), (0x8F, 1, 0x2142, 1, 0, 40.0, 1.0, Vec3::new(20.0, 40.0, 20.0), Vec3::new(-20.0, 0.0, -30.0)));
    assert_eq!(row(&w, "D_80120B94", 1), (0x84, 0xFF, 0x0404, 19, 5, 70.0, 0.01, Vec3::new(0.0, 30.0, 20.0), Vec3::new(120.0, 60.0, 120.0)));
    assert_eq!(row(&w, "D_80120B94", 6), (0x84, 0xFF, 0x0404, 40, 10, 50.0, 0.01, Vec3::new(0.0, 20.0, 20.0), Vec3::new(120.0, 60.0, 120.0)));
    assert_eq!(row(&w, "D_80120B94", 9), (0x8D, 0xFF, 0x4141, 150, 0, 50.0, 1.0, Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.0, 4.0, 120.0)));
    assert_eq!(row(&w, "D_80120B94", 10), (0x98, 0xFF, 0x0000, 1, 0, 50.0, 1.0, m1, m1));
    // D_80120D4C (2280): seven, its first four 2270's.
    assert_eq!(w.onepoint.len(w.onepoint.table("D_80120D4C")), 7);
    for i in 0..4 {
        assert_eq!(row(&w, "D_80120D4C", i), row(&w, "D_80120B94", i), "row {i}");
    }
    assert_eq!(row(&w, "D_80120D4C", 3), (0x84, 0xFF, 0x0404, 40, -10, 50.0, 0.02, Vec3::new(0.0, 30.0, 20.0), Vec3::new(120.0, 60.0, 120.0)));
    assert_eq!(row(&w, "D_80120D4C", 4), (0x8F, 0xFF, 0x4141, 1, 0, 50.0, 1.0, Vec3::new(0.0, -10.0, 0.0), Vec3::new(80.0, 20.0, 60.0)));
    assert_eq!(row(&w, "D_80120D4C", 6), (0x98, 0xFF, 0x0000, 1, 0, 50.0, 1.0, m1, m1));
}

#[test]
fn his_text_by_age_and_the_long_song() {
    let Some(a) = assets() else { return };
    let mut w = lake(&a, |_| {});
    let h = find(&mut w);
    // EnKakasi_Init: scale 0.01, attention range 6, the hookshot pulling Link, immovable; then
    // func_80A8F660: a child, no long song: 0x4076, TEXT_STATE_DONE.
    let s = k(&mut w, h);
    assert_eq!((s.actor.scale, s.actor.target_mode, s.actor.col_chk_info.mass), (Vec3::splat(0.01), 6, 0xFF));
    assert_ne!(s.actor.flags & kakasi::ACTOR_FLAG_HOOKSHOT_PULLS_PLAYER, 0);
    assert_eq!((s.action, s.actor.text_id, s.unk_196, s.unk_194), (Action::Func80a8f75c, 0x4076, TEXT_STATE_DONE, false));
    // The long song set: 0x407A, an event.
    w.save.scarecrow_long_song_set = 1;
    k(&mut w, h).action = Action::Func80a8f660;
    update(&mut w, h);
    let s = k(&mut w, h);
    assert_eq!((s.action, s.actor.text_id, s.unk_196), (Action::Func80a8f75c, 0x407A, TEXT_STATE_EVENT));
    // An adult: 0x4079; without the song 0x4076.
    w.save.adult = true;
    k(&mut w, h).action = Action::Func80a8f660;
    update(&mut w, h);
    let s = k(&mut w, h);
    assert_eq!((s.actor.text_id, s.unk_196, s.unk_194), (0x4079, TEXT_STATE_EVENT, true));
    w.save.scarecrow_long_song_set = 0;
    k(&mut w, h).action = Action::Func80a8f660;
    update(&mut w, h);
    assert_eq!((k(&mut w, h).actor.text_id, k(&mut w, h).unk_196), (0x4076, TEXT_STATE_DONE));
}

#[test]
fn a_childs_ocarina_out_records_the_long_song_and_he_dances_to_it() {
    let Some(a) = assets() else { return };
    let mut w = lake(&a, |_| {});
    let h = find(&mut w);
    // Within 80, no ocarina: PLAYER_STATE2_23 only.
    w.player_mut().state2 &= !(STATE2_23 | STATE2_24);
    link_before(&mut w, h, 70.0);
    update(&mut w, h);
    assert_ne!(w.player().state2 & STATE2_23, 0);
    assert_eq!(k(&mut w, h).action, Action::Func80a8f75c);
    // Behind him (yaw 0x4300 off): nothing.
    w.player_mut().state2 &= !STATE2_23;
    w.player_mut().state2 |= STATE2_24;
    link_before(&mut w, h, 70.0);
    k(&mut w, h).actor.yaw_towards_player = 0x4300;
    update(&mut w, h);
    assert_eq!(k(&mut w, h).action, Action::Func80a8f75c);
    // In front, the ocarina out: 2260 on him, OCARINA_ACTION_SCARECROW_LONG_RECORDING.
    link_before(&mut w, h, 70.0);
    update(&mut w, h);
    let s = k(&mut w, h);
    assert_eq!((s.action, s.dance.unk_19a, s.dance.unk_1b8), (Action::Func80a8f8d0, 0, 0.0));
    let sub = s.sub_cam_id;
    assert_ne!(sub, CAM_ID_NONE);
    let cam = onepoint_cam(&w, 2260).expect("2260's camera");
    assert_eq!((cam.cam_id, cam.setting, cam.cs_info.key_frame_cnt), (sub, CAM_SET_CS_C, 5));
    assert_eq!(cam.cs_info.key_frames.map(|k| (k.table, k.start)), Some((w.onepoint.table("D_80120ACC"), 0)));
    // The first and third rows' at.x: ±10 plus Rand × 8; eye.x ∓20 plus Rand × 5.
    let (r0, r2) = (row(&w, "D_80120ACC", 0), row(&w, "D_80120ACC", 2));
    assert_eq!((r0.7.x, r0.8.x), (r2.7.x, r2.8.x));
    assert!((-10.0..-2.0).contains(&r0.7.x) || (10.0..18.0).contains(&r0.7.x), "{}", r0.7.x);
    assert!((20.0..25.0).contains(&r0.8.x) || (-20.0..-15.0).contains(&r0.8.x), "{}", r0.8.x);
    assert_eq!(w.msg_ctx.ocarina_action, OCARINA_ACTION_SCARECROW_LONG_RECORDING);
    assert_ne!(w.player().state2 & STATE2_23, 0);

    // Link playing (OCARINA_MODE_01): C-Left rocks him on x (0x1388), the count up one, gravity
    // on; his animation's speed approaching unk_1B8 (0): 0.
    w.msg_ctx.ocarina_mode = OCARINA_MODE_01;
    w.msg_ctx.last_ocarina_button_index = OCARINA_BTN_C_LEFT;
    update(&mut w, h);
    let s = k(&mut w, h);
    assert_eq!((s.dance.unk_19a, s.dance.rock_x, s.actor.gravity, s.skel.play_speed), (1, 0x1388, -1.0, 0.0));
    // Math_SmoothStepToS(rot.x, 0x1388, 5, 0x3E8, 0): a fifth, at most 0x3E8.
    assert_eq!(s.actor.shape_rot.x, 0x3E8);
    // A spins him: 0x1000 a frame.
    let y0 = s.actor.shape_rot.y;
    w.msg_ctx.last_ocarina_button_index = OCARINA_BTN_A;
    update(&mut w, h);
    let s = k(&mut w, h);
    assert_eq!((s.dance.unk_19a, s.dance.unk_1a4), (2, 1));
    assert_eq!(s.actor.shape_rot.y, y0.wrapping_add(0x1000));
    // C-Up: speed 2, approached 0.1 of the way, at most 0.2.
    w.msg_ctx.last_ocarina_button_index = OCARINA_BTN_C_UP;
    update(&mut w, h);
    let s = k(&mut w, h);
    assert_eq!((s.dance.unk_19a, s.dance.unk_1b8, s.skel.play_speed), (3, 2.0, 0.2));
    // No note (0xFF): the count stays, the dance goes on.
    w.msg_ctx.last_ocarina_button_index = 0xFF;
    update(&mut w, h);
    assert_eq!(k(&mut w, h).dance.unk_19a, 3);
    // Eight notes in, on the ground: a hop, 3 up.
    k(&mut w, h).dance.unk_19a = 7;
    k(&mut w, h).actor.bg_check_flags |= oot_game::actor::BGCHECKFLAG_GROUND;
    w.msg_ctx.last_ocarina_button_index = OCARINA_BTN_C_UP;
    update(&mut w, h);
    let s = k(&mut w, h);
    assert_eq!(s.dance.unk_19a, 8);
    // velocity.y 3, then Actor_MoveXZGravity's gravity: 2.
    assert_eq!(s.actor.velocity.y, 2.0);

    // The recording over (OCARINA_MODE_04, the box closed): notes played, 0x4077 (an event).
    w.msg_ctx.ocarina_mode = OCARINA_MODE_04;
    w.msg_ctx.msg_mode = MSGMODE_NONE;
    update(&mut w, h);
    let s = k(&mut w, h);
    assert_eq!((s.action, s.actor.text_id, s.unk_196), (Action::Func80a8f9c8, 0x4077, TEXT_STATE_EVENT));
    assert_eq!(w.msg_ctx.text_id, 0x4077);
}

#[test]
fn a_recording_without_a_note_ends_the_cutscene() {
    let Some(a) = assets() else { return };
    let mut w = lake(&a, |_| {});
    let h = find(&mut w);
    w.player_mut().state2 |= STATE2_24;
    link_before(&mut w, h, 70.0);
    update(&mut w, h);
    let sub = k(&mut w, h).sub_cam_id;
    assert_eq!(k(&mut w, h).action, Action::Func80a8f8d0);
    w.msg_ctx.ocarina_mode = OCARINA_MODE_04;
    w.msg_ctx.msg_mode = MSGMODE_NONE;
    update(&mut w, h);
    // OnePointCutscene_EndCutscene: the camera's timer 0; back to standing.
    let s = k(&mut w, h);
    assert_eq!((s.action, s.sub_cam_id), (Action::Func80a8f660, CAM_ID_NONE));
    assert_eq!(w.camera(sub).map(|c| c.timer), Some(0));
}

#[test]
fn the_long_song_played_back_to_a_child() {
    let Some(a) = assets() else { return };
    let mut w = lake(&a, |s| s.scarecrow_long_song_set = 1);
    let h = find(&mut w);
    // Talked to (0x407A's event): func_80A8F9C8 holds Link.
    k(&mut w, h).action = Action::Func80a8f9c8;
    update(&mut w, h);
    assert_eq!(w.player().cs_mode, kakasi::PLAYER_CSACTION_8);
    assert_eq!(k(&mut w, h).action, Action::Func80a8f9c8);
    // The box at its event, A: 2270 and the long song's playback.
    box_done(&mut w, TEXTBOX_ENDTYPE_EVENT);
    update_a(&mut w, h);
    let s = k(&mut w, h);
    assert_eq!(s.action, Action::Func80a8faa4);
    let sub = s.sub_cam_id;
    let cam = onepoint_cam(&w, 2270).expect("2270's camera");
    assert_eq!((cam.cam_id, cam.setting, cam.cs_info.key_frame_cnt), (sub, CAM_SET_CS_C, 11));
    assert_eq!(cam.cs_info.key_frames.map(|k| (k.table, k.start)), Some((w.onepoint.table("D_80120B94"), 0)));
    // The first eight rows but the 0x8F ones (0 and 4) from Rand: at.x 0 to 5, at.z 10 to 40,
    // eye.x 20 to 120, eye.z 50 to 130.
    for i in [1, 2, 3, 5, 6, 7] {
        let r = row(&w, "D_80120B94", i);
        assert!((0.0..5.0).contains(&r.7.x) && (10.0..40.0).contains(&r.7.z), "row {i} at {}", r.7);
        assert!((20.0..120.0).contains(&r.8.x) && (50.0..130.0).contains(&r.8.z), "row {i} eye {}", r.8);
    }
    assert_eq!((row(&w, "D_80120B94", 4).7.x, row(&w, "D_80120B94", 4).8.z), (0.0, 50.0));
    // @bug (game): the row [subCamId - 1]'s eye.y: ±3 plus Rand.
    let y = row(&w, "D_80120B94", (sub - 1) as usize).8.y;
    assert!((3.0..4.0).contains(&y) || (-3.0..-2.0).contains(&y), "{y}");
    // Its quake: QUAKE_TYPE_5 on the sub camera, speed 400, (4, 5, 40, 0x3C), 1600 frames.
    let q = w.quake.requests.iter().find(|r| r.cam_id == sub).expect("2270's quake");
    assert_eq!((q.type_, q.speed, q.y, q.x, q.fov, q.up_pitch_offset, q.duration), (oot_game::quake::QUAKE_TYPE_5, 400, 4, 5, 40, 0x3C, 1600));
    assert_eq!(w.msg_ctx.ocarina_action, OCARINA_ACTION_SCARECROW_LONG_PLAYBACK);
    assert_eq!(w.player().cs_mode, kakasi::PLAYER_CSACTION_8);

    // Playing back (not OCARINA_MODE_0F): dancing by Rand, the count 5 to 7 plus its note's one.
    w.msg_ctx.ocarina_mode = OCARINA_MODE_01;
    update(&mut w, h);
    let s = k(&mut w, h);
    assert!((5..=8).contains(&s.dance.unk_19a), "{}", s.dance.unk_19a);
    // unk_19C[3]: 30 to 40 (set after the update's countdown).
    assert!((30..=40).contains(&s.dance.unk_19c[3]), "{}", s.dance.unk_19c[3]);
    assert_eq!(s.actor.gravity, -1.0);

    // Over (OCARINA_MODE_0F, the box closed): a child's 0x4078, 2260 again.
    w.msg_ctx.ocarina_mode = OCARINA_MODE_0F;
    w.msg_ctx.msg_mode = MSGMODE_NONE;
    update(&mut w, h);
    let s = k(&mut w, h);
    assert_eq!((s.action, s.actor.text_id, s.unk_196), (Action::Func80a8fbb8, 0x4078, TEXT_STATE_EVENT));
    assert_eq!(w.msg_ctx.text_id, 0x4078);
    assert!(onepoint_cam(&w, 2260).is_some());
    // At its event, A: the box closing, Link let go.
    box_done(&mut w, TEXTBOX_ENDTYPE_EVENT);
    update_a(&mut w, h);
    assert_eq!(k(&mut w, h).action, Action::Func80a8f660);
    assert_eq!(w.player().cs_mode, kakasi::PLAYER_CSACTION_7);
    assert_eq!(w.msg_ctx.msg_mode, MSGMODE_TEXT_CLOSING);
    update(&mut w, h);
    assert_eq!((k(&mut w, h).action, k(&mut w, h).actor.text_id), (Action::Func80a8f75c, 0x407A));
}

#[test]
fn an_adult_hears_the_long_song_back_with_0x4077() {
    let Some(a) = assets() else { return };
    let mut w = lake(&a, |s| s.scarecrow_long_song_set = 1);
    let h = find(&mut w);
    w.save.adult = true;
    k(&mut w, h).action = Action::Func80a8f660;
    update(&mut w, h);
    assert_eq!(k(&mut w, h).actor.text_id, 0x4079);
    // An adult's ocarina doesn't start a recording.
    w.player_mut().state2 |= STATE2_24;
    link_before(&mut w, h, 70.0);
    update(&mut w, h);
    assert_eq!(k(&mut w, h).action, Action::Func80a8f75c);
    k(&mut w, h).action = Action::Func80a8faa4;
    w.msg_ctx.ocarina_mode = OCARINA_MODE_0F;
    w.msg_ctx.msg_mode = MSGMODE_NONE;
    update(&mut w, h);
    assert_eq!((k(&mut w, h).action, w.msg_ctx.text_id), (Action::Func80a8fbb8, 0x4077));
}
