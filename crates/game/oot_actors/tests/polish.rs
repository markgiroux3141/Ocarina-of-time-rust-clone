//! Cutscene polish (GAME-04b milestone 7): the opening's and the Deku Tree's cutscenes checked
//! against the C frame by frame, not by eye. `Camera_Demo1` on the scripts' splines, the
//! letterbox (`Letterbox_Update`), the narration's text box (`Message_Update`'s placement), and
//! Link's poses on every cue (`D_80854B18`'s starts).

mod common;

use std::sync::Arc;

use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_game::camera::func_800bb2b4;
use oot_game::cutscene::*;
use oot_game::message::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Presses A for each text that waits (as a player reading along would).
struct Driver {
    prev: PadState,
}

impl Driver {
    fn tick(&mut self, w: &mut PlayState) {
        let st = w.message_state();
        let waits = matches!(st, TEXT_STATE_AWAITING_NEXT | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_CHOICE | TEXT_STATE_EVENT) || w.msg_ctx.msg_mode == MSGMODE_TEXT_AWAIT_INPUT;
        let mut pad = PadState::default();
        if waits && self.prev.button == 0 {
            pad.button = BTN_A;
        }
        w.tick_with(scripted_input(self.prev, pad));
        self.prev = pad;
    }
}

/// A script's camera lists of type `ty`: (start frame, end frame, points).
fn lists(s: &CutsceneScript, ty: i32) -> Vec<(u16, u16, Vec<CutsceneCameraPoint>)> {
    let (cmds, _) = walk(&s.data).unwrap();
    cmds.iter()
        .filter(|c| c.cmd_type == ty)
        .map(|c| {
            let o = c.offset + 4;
            let be16 = |o: usize| u16::from_be_bytes([s.data[o], s.data[o + 1]]);
            (be16(o + 2), be16(o + 4), CutsceneCameraPoint::read_list(&s.data, o + 8))
        })
        .collect()
}

/// `Camera_Demo1`'s eye and at after `n` updates from its start (`animState` 0): each update
/// runs the eye's spline, then (unless it ended) the at's, on one shared keyframe and frame
/// counter (`func_800BB2B4(.., &rwData->keyframe, &rwData->curFrame) || ..`); once either ends,
/// `animState` 2 does nothing more.
fn demo1(eye: &[CutsceneCameraPoint], at: &[CutsceneCameraPoint], n: usize) -> (Vec3, Vec3) {
    let (mut key, mut cur, mut fov) = (0i16, 0.0f32, 60.0f32);
    let (mut e, mut a) = (Vec3::ZERO, Vec3::ZERO);
    for _ in 0..n {
        let (eye_done, eu) = func_800bb2b4(eye, &mut key, &mut cur, &mut fov);
        if let Some((v, _)) = eu {
            e = v;
        }
        let mut done = eye_done;
        if !eye_done {
            let (at_done, au) = func_800bb2b4(at, &mut key, &mut cur, &mut fov);
            if let Some((v, _)) = au {
                a = v;
            }
            done = at_done;
        }
        if done {
            break;
        }
    }
    (e, a)
}

/// Checks the active camera against the script's absolute lists, frame by frame:
/// `Cutscene_Command_CameraEyePoints` and `_LookAtPoints` put a pair on the camera on the first
/// frame after its start (`startFrame < csCtx->frames`), once both have been seen, and reset
/// `Camera_Demo1`. The script's first pair is applied on two frames running: on the first by the
/// at command (the eye list was seen first, with no at list yet: `unk_1A` 0), on the next by the
/// eye command, whose `unk_18` was only set then; a later pair is applied once. After that the
/// camera updates once a frame, also while a text holds the script's frame.
fn check_demo1(w: &mut PlayState, d: &mut Driver, script: &CutsceneScript, frames: u16) -> usize {
    let eyes = lists(script, CS_CMD_CAM_EYE);
    let ats = lists(script, CS_CMD_CAM_AT);
    assert_eq!(eyes.len(), ats.len());
    let mut checked = 0;
    // The pair in charge and the camera's updates since its last reset.
    let mut current: Option<usize> = None;
    let mut applied_twice = false;
    let mut n = 0usize;
    let mut prev_frame = u16::MAX;
    while w.cs_ctx.frames < frames && w.cs_ctx.segment.as_ref().is_some_and(|s| s.name == script.name) {
        let f = w.cs_ctx.frames;
        let new_frame = f != prev_frame;
        prev_frame = f;
        // The pair whose start this frame is just past.
        let starting = (0..eyes.len()).find(|&k| eyes[k].0 + 1 == f && ats[k].0 == eyes[k].0);
        if new_frame && let Some(k) = starting {
            applied_twice = current.is_some();
            current = Some(k);
            n = 1;
        } else if new_frame && !applied_twice && current.is_some_and(|k| eyes[k].0 + 2 == f) {
            // The first pair's second application.
            applied_twice = true;
            n = 1;
        } else if current.is_some() {
            n += 1;
        }
        if let Some(k) = current {
            let (e, a) = demo1(&eyes[k].2, &ats[k].2, n);
            let c = w.active_camera();
            assert!(c.eye.distance(e) < 0.01, "frame {f}: eye {:?}, the spline's {e:?} (pair {k}, {n} updates)", c.eye);
            assert!(c.at.distance(a) < 0.01, "frame {f}: at {:?}, the spline's {a:?} (pair {k}, {n} updates)", c.at);
            checked += 1;
        }
        d.tick(w);
    }
    checked
}

#[test]
fn the_narrations_camera_follows_its_splines() {
    let Some(a) = assets() else { return };
    let save = SaveContext::file_select_new();
    let mut w = oot_actors::play_entrance(a.clone(), common::data().unwrap(), common::rules().unwrap(), save).expect("Play_Init");
    let mut d = Driver { prev: PadState::default() };
    d.tick(&mut w);
    // Link's house, layer 5: the script at 0x15D0 (three pairs from 0, 130 and 180).
    let s = w.cs_ctx.segment.clone().unwrap();
    assert_eq!(s.name, "0x15D0");
    let n = check_demo1(&mut w, &mut d, &s, 280);
    assert!(n > 250, "{n} frames checked");
}

#[test]
fn the_deku_trees_talk_camera_follows_its_splines() {
    let Some(a) = assets() else { return };
    let mut save = SaveContext::new(a.scenes.entrance_index("ENTR_SPOT04_1").unwrap(), false, oot_game::env::clock_time(10, 0) as u16);
    save.cutscene_index = 0;
    let mut w = oot_actors::play_entrance(a.clone(), common::data().unwrap(), common::rules().unwrap(), save).expect("Play_Init");
    let mut d = Driver { prev: PadState::default() };
    for _ in 0..60 {
        if w.cs_ctx.segment.as_ref().is_some_and(|s| s.name == "D_808BCE20") && w.cs_ctx.frames > 0 {
            break;
        }
        d.tick(&mut w);
    }
    let s = w.cs_ctx.segment.clone().unwrap();
    assert_eq!(s.name, "D_808BCE20");
    // Two pairs (from 0 and 60), through the texts to the script's end.
    let n = check_demo1(&mut w, &mut d, &s, 180);
    assert!(n > 100, "{n} frames checked");
}

#[test]
fn the_letterbox_grows_and_shrinks_by_ten_rows() {
    let Some(a) = assets() else { return };
    let save = SaveContext::new(a.scenes.entrance_index("ENTR_SPOT04_1").unwrap(), false, oot_game::env::clock_time(10, 0) as u16);
    let mut w = oot_actors::play_entrance(a.clone(), common::data().unwrap(), common::rules().unwrap(), save).expect("Play_Init");
    let mut d = Driver { prev: PadState::default() };
    // The Deku Tree's talk starts with cutsceneTrigger 1 (func_808BC8B8): no Letterbox_SetSize,
    // so func_80064760 sets the target 32 each frame of CS_STATE_SKIPPABLE_INIT and
    // Letterbox_Update(R_UPDATE_RATE 3) steps 10 rows a frame: 10, 20, 30, 32.
    let mut rows = vec![w.letterbox.rows()];
    for _ in 0..120 {
        d.tick(&mut w);
        rows.push(w.letterbox.rows());
        if w.cs_ctx.state != CS_STATE_IDLE && rows.ends_with(&[32, 32]) {
            break;
        }
    }
    let run = |v: &[i32], want: &[i32]| v.windows(want.len()).any(|x| x == want);
    assert!(run(&rows, &[0, 10, 20, 30, 32]), "{rows:?}");
    // At the talk's end Link's camera takes the view back; its interface flags have no
    // letterbox, so it shrinks by 10: 22, 12, 2, 0.
    let mut seen = Vec::new();
    for _ in 0..3000 {
        d.tick(&mut w);
        seen.push(w.letterbox.rows());
        if w.cs_ctx.state == CS_STATE_IDLE && seen.ends_with(&[0, 0]) && seen.contains(&32) {
            break;
        }
    }
    assert!(run(&seen, &[32, 22, 12, 2, 0]), "{seen:?}");
}

#[test]
fn the_narrations_text_sits_where_the_c_puts_it() {
    let Some(a) = assets() else { return };
    let save = SaveContext::file_select_new();
    let mut w = oot_actors::play_entrance(a.clone(), common::data().unwrap(), common::rules().unwrap(), save).expect("Play_Init");
    let mut d = Driver { prev: PadState::default() };
    for _ in 0..200 {
        d.tick(&mut w);
        if w.msg_ctx.text_id == 0x109D && w.msg_ctx.msg_mode >= MSGMODE_TEXT_STARTING {
            break;
        }
    }
    assert_eq!(w.msg_ctx.text_id, 0x109D);
    // Message_Update's MSGMODE_TEXT_START: by the type (typePos >> 4) and the position
    // (typePos & 0xF): sTextboxXPositions, and the upper, middle or lower Y; a box with no
    // background (TEXTBOX_TYPE_NONE_*) goes straight to its place at its full size.
    const X: [i16; 6] = [34, 34, 34, 34, 34, 34];
    const LOWER: [i16; 6] = [142, 142, 142, 142, 174, 142];
    const UPPER: [i16; 6] = [38, 38, 38, 38, 174, 38];
    const MID: [i16; 6] = [90, 90, 90, 90, 174, 90];
    const END_ICON: [i16; 6] = [59, 59, 59, 59, 34, 59];
    let m = &w.msg_ctx;
    let t = (m.text_box_type as usize).min(5);
    let y = match m.text_box_pos {
        TEXTBOX_POS_TOP => UPPER[t],
        TEXTBOX_POS_MIDDLE => MID[t],
        // TEXTBOX_POS_VARIABLE (0): the lower place, or the upper if Link is low on the screen
        // (averageY against XREG(92) in a scene with a fixed camera).
        0 if m.regs.textbox_y_target == UPPER[t] => UPPER[t],
        _ => LOWER[t],
    };
    let r = &m.regs;
    assert_eq!((r.textbox_x_target, r.textbox_y_target, r.textbox_end_ypos), (X[t], y, END_ICON[t] + y));
    if m.text_box_type == TEXTBOX_TYPE_NONE_BOTTOM || m.text_box_type == TEXTBOX_TYPE_NONE_NO_SHADOW {
        assert_eq!((r.textbox_x, r.textbox_y, r.textbox_width, r.textbox_height), (X[t], y, 256, 64));
    }
}

#[test]
fn links_poses_follow_the_cutscene_mode_tables() {
    let Some(a) = assets() else { return };
    let save = SaveContext::file_select_new();
    let mut w = oot_actors::play_entrance(a.clone(), common::data().unwrap(), common::rules().unwrap(), save).expect("Play_Init");
    let mut d = Driver { prev: PadState::default() };
    let data = w.data.clone();
    // Through the opening (the narration, the nightmare, Navi sent, the wake-up): each time
    // Link's cutscene mode changes, the mode's start in D_80854B18 (func_80852B4C). The types
    // that start an animation at once set it on that frame: 2 to 10 and 14 (D_80854AA4's
    // func_80851030 to func_80851174, and func_80851050).
    let mut last = (0u16, 0u16);
    let mut woke = false;
    let mut checked = Vec::new();
    for _ in 0..12000 {
        d.tick(&mut w);
        let Some(p) = w.player.and_then(|h| w.actors.downcast::<oot_actors::player::Player>(h)) else { continue };
        let now = (w.scene_id, p.unk_446);
        if now != last && p.cs_mode == 6 && p.unk_446 != 0 {
            // func_80852C50: the cue's mode by D_808547C4.
            let mode = oot_actors::player::d_808547c4(p.unk_446).unsigned_abs();
            // The starts that are functions, by what the C's function plays: func_80851F84
            // (mode 38: func_80851134 with clink_op3_wait1, the shadow off), func_80851E90 (39:
            // func_8083303C with clink_op3_negaeri), func_808515A4 (8: PLAYER_ANIMGROUP_44 of
            // the model's type, Link's standing wait).
            let by_function = match mode {
                38 => Some("clink_op3_wait1"),
                39 => Some("clink_op3_negaeri"),
                _ => None,
            };
            if let Some(name) = by_function {
                assert_eq!(data.anim_name(p.skel.animation), name, "scene {:#x}, cue {:#x}, mode {mode}", w.scene_id, p.unk_446);
                if mode == 38 {
                    assert!(!p.shadow_feet);
                }
                checked.push(mode);
            }
            if let Some(e) = data.cs_mode_starts.get(mode as usize)
                && (matches!(e.ty, 2..=10) || e.ty == 14)
                && let Some(anim) = e.anim
            {
                assert_eq!(data.anim_name(p.skel.animation), data.anim_name(anim), "scene {:#x}, cue {:#x}, mode {mode} (type {})", w.scene_id, p.unk_446, e.ty);
                checked.push(mode);
            }
        }
        last = now;
        // The wake-up (Link's house, layer 4) over: its cues seen, the script done.
        let home = w.scene_id == 0x34 && w.save.scene_layer == 4;
        woke |= home && p.unk_446 != 0;
        if woke && w.cs_ctx.state == CS_STATE_IDLE && p.cs_mode == 0 {
            break;
        }
    }
    // The narration's 38, the nightmare's 9 (link_demo_furimuki, type 2), the wake-up's 39, 40
    // and 41 (clink_op3_okiagari, clink_op3_tatiagari: type 6).
    checked.sort_unstable();
    checked.dedup();
    assert_eq!(checked, vec![9, 38, 39, 40, 41]);
}

/// Plays the new file's opening up to Kokiri Forest's layer 7 (Navi sent).
fn to_navi_sent(w: &mut PlayState, d: &mut Driver) {
    for _ in 0..3000 {
        if w.scene_id == 0x55 && w.save.scene_layer == 7 {
            return;
        }
        d.tick(w);
    }
    panic!("no layer 7");
}

#[test]
fn the_first_text_shows_over_the_white_out() {
    let Some(a) = assets() else { return };
    let mut w = oot_actors::play_entrance(a.clone(), common::data().unwrap(), common::rules().unwrap(), SaveContext::file_select_new()).expect("Play_Init");
    let mut d = Driver { prev: PadState::default() };
    to_navi_sent(&mut w, &mut d);
    // The Deku Tree's first text comes while the delayed white-out still covers the screen
    // (TRANS_TYPE_FADE_WHITE_CS_DELAYED: (160, 160, 160) until its fade runs). Play_Draw draws
    // the transition at the start of OVERLAY_DISP and the message box after it
    // (Play_DrawOverlayElements): the text is over the fill.
    let view = oot_game::play::ViewInfo::new(w.view.eye, glam::Mat4::IDENTITY);
    let mut shown = None;
    for _ in 0..200 {
        d.tick(&mut w);
        let mut out = oot_game::play::DrawOut::default();
        w.draw(&w.current_frame(), &view, &mut out);
        // The first frame with the text's characters on screen.
        if out.overlay_2d.iter().any(|c| c.mesh.name.contains("message/char")) {
            shown = Some(out);
            break;
        }
    }
    let out = shown.expect("the text's characters");
    assert_eq!(out.overlay_fill, Some([160, 160, 160, 255]));
    // The cutscene's own fill (envCtx.fillScreen) is in the 3D lists, under the overlay.
    assert_eq!(out.opa_fill, out.xlu_fill);
}

#[test]
fn the_camera_loads_the_village_as_it_flies_in() {
    let Some(a) = assets() else { return };
    let mut w = oot_actors::play_entrance(a.clone(), common::data().unwrap(), common::rules().unwrap(), SaveContext::file_select_new()).expect("Play_Init");
    let mut d = Driver { prev: PadState::default() };
    to_navi_sent(&mut w, &mut d);
    // Link stands in room 1 (the Deku Tree's meadow) the whole time; Navi's flight takes the
    // camera through the plane to the village. In a cutscene En_Holl's func_80A59014 tests the
    // view's eye, not Player (useViewEye), so room 0 loads.
    assert_eq!(w.room_ctx.cur.num, 1);
    let mut village = false;
    for _ in 0..3000 {
        d.tick(&mut w);
        if w.scene_id != 0x55 {
            break;
        }
        if w.room_ctx.cur.num == 0 {
            assert_ne!(w.cs_ctx.state, CS_STATE_IDLE);
            village = true;
            break;
        }
    }
    assert!(village, "room 0 never loaded");
}

#[test]
fn navis_flight_has_its_sounds() {
    use oot_game::audio::GameAudio;
    use oot_game::audio::sfx::*;
    let Some(a) = assets() else { return };
    let audio = GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), common::data().unwrap(), common::rules().unwrap(), SaveContext::file_select_new(), audio).expect("Play_Init");
    let mut d = Driver { prev: PadState::default() };
    to_navi_sent(&mut w, &mut d);
    // Object_Kankyo (params 0) in Kokiri Forest's layer 7, on the script's frames: Navi's calls
    // (473: func_800788CC, 583: func_800F4524 with 32), her crash into the fence (763) and her
    // cry (771: NA_SE_VO_RT_THROW, func_80078884); the wing hum every frame (func_800F436C).
    let mut heard: Vec<(u16, u16)> = Vec::new();
    let mut hum = 0;
    let mut seen = w.audio.log.as_ref().unwrap().sfx.len();
    for _ in 0..3000 {
        // The actor updates before the frame's cutscene step: the frame it sees.
        let f = w.cs_ctx.frames;
        d.tick(&mut w);
        if w.scene_id != 0x55 {
            break;
        }
        let log = &w.audio.log.as_ref().unwrap().sfx;
        for &(_, id, _) in &log[seen..] {
            if [NA_SE_VO_NA_HELLO_3, NA_SE_VO_NA_HELLO_2, NA_SE_EV_NAVY_CRASH - SFX_FLAG, NA_SE_VO_RT_THROW].contains(&id) {
                heard.push((f, id));
            }
            if id == NA_SE_EV_NAVY_FLY - SFX_FLAG {
                hum += 1;
            }
        }
        seen = log.len();
    }
    assert_eq!(heard, vec![(473, NA_SE_VO_NA_HELLO_3), (583, NA_SE_VO_NA_HELLO_2), (763, NA_SE_EV_NAVY_CRASH - SFX_FLAG), (771, NA_SE_VO_RT_THROW)]);
    assert!(hum > 500, "{hum}");
}
