//! The cutscene system (`z_demo.c`, `oot_game::cutscene`), the Deku Tree's scripts
//! (`Bg_Treemouth`) and Player's cutscene modes, against the scripts' C and `z_demo.c`:
//! - `D_808BCE20` (`z_bg_treemouth_cutscene_data.c`): `CS_BEGIN_CUTSCENE(12, 3000)`; Link's cues 2
//!   (frames 0..33, (2614, 0, -451) to (2808, 0, -559)) and 4 (33..42, to (2857, 0, -594)); eye
//!   and at lists from 0 and from 60; texts 0x107D (40..60) and 0x1015 (160..170, no branches);
//!   `CS_MISC(0x000C, 180, 200)`: the script's end; the mouth's cue 1 (list 46);
//! - `D_808BD520` (yes): `CS_BEGIN_CUTSCENE(8, 3000)`; the mouth's cues 1 (0..20) and 3
//!   (20..357); text 0x1017 (20..60); `CS_MISC(0x000C, 100, 150)`;
//! - `D_808BD790` (no): text 0x1018 (20..60); `CS_MISC(0x000C, 80, 110)`;
//! - `gDekuTreeIntroCs` (`ydan_scene`): `CS_BEGIN_CUTSCENE(4, 1270)`, `CS_MISC(0x000C, 165, 166)`,
//!   a title card (`CS_MISC` 15), an eye and an at list; `sEntranceCutsceneTable` plays it on
//!   `ENTR_YDAN_0` for either age with `EVENTCHKINF_A8`.

mod common;

use std::sync::Arc;

use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_treemouth::{self, BgTreemouth};
use oot_actors::en_wonder_talk2::EnWonderTalk2;
use oot_actors::player::{Action, STATE1_29};
use oot_game::camera::{CAM_ID_MAIN, CAM_SET_CS_0, CAM_STAT_ACTIVE, CAM_STAT_WAIT};
use oot_game::cutscene::*;
use oot_game::message::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::{EVENTCHKINF_0C, EVENTCHKINF_05, SaveContext};

/// `EVENTCHKINF_A8` (`z64save.h`): the Deku Tree's intro seen.
const EVENTCHKINF_A8: u16 = 0xA8;
const SCENE_YDAN: u16 = 0x00;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn enter(a: &Arc<GameAssets>, entrance: &str, f: impl FnOnce(&mut SaveContext)) -> Option<PlayState> {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    f(&mut save);
    Some(oot_actors::play_entrance(a.clone(), common::data()?, common::rules()?, save).expect("Play_Init"))
}

fn treemouth(w: &PlayState) -> Option<&BgTreemouth> {
    w.actors.all().into_iter().find_map(|h| w.actors.downcast::<BgTreemouth>(h))
}

/// A frame with `pad`, A pressed on its own frame when a box waits (released the next); `no`
/// answers a choice with its second option.
struct Driver {
    prev: PadState,
    no: bool,
}

impl Driver {
    fn tick(&mut self, w: &mut PlayState) {
        let st = w.message_state();
        let waits = matches!(st, TEXT_STATE_AWAITING_NEXT | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_CHOICE | TEXT_STATE_EVENT) || w.msg_ctx.msg_mode == MSGMODE_TEXT_AWAIT_INPUT;
        let mut pad = PadState::default();
        if st == TEXT_STATE_CHOICE && self.no && w.msg_ctx.choice_index == 0 {
            // Message_HandleChoiceSelection: the stick down to the second option.
            if self.prev.stick_y == 0 {
                pad.stick_y = -60;
            }
        } else if waits && self.prev.button == 0 {
            pad.button = BTN_A;
        }
        w.tick_with(scripted_input(self.prev, pad));
        self.prev = pad;
    }
    fn until(&mut self, w: &mut PlayState, max: usize, what: &str, done: impl Fn(&PlayState) -> bool) -> usize {
        for i in 0..max {
            if done(w) {
                return i;
            }
            self.tick(w);
        }
        panic!("{what} not within {max} frames: cs state {}, frames {}, text {:#x}", w.cs_ctx.state, w.cs_ctx.frames, w.msg_ctx.text_id);
    }
}

#[test]
fn the_deku_trees_scripts_are_the_c_s() {
    let Some(a) = assets() else { return };
    let s = a.cutscene("D_808BCE20").unwrap();
    assert_eq!((s.file.as_str(), be_i32(&s.data, 0), be_i32(&s.data, 4)), ("ovl_Bg_Treemouth", 12, 3000));
    let (cmds, end) = walk(&s.data).unwrap();
    let types: Vec<i32> = cmds.iter().map(|c| c.cmd_type).collect();
    // CS_UNK_DATA_LIST(0x15), the player cues, 2 eye lists, 2 at lists, the texts, the misc, the
    // mouth's cues (46: npcActions[0]), Navi's (62, CS_CMD_SET_ACTOR_ACTION_9: npcActions[8]),
    // the BGM and its fade.
    assert_eq!(types, vec![0x15, CS_CMD_SET_PLAYER_ACTION, CS_CMD_CAM_EYE, CS_CMD_CAM_EYE, CS_CMD_CAM_AT, CS_CMD_CAM_AT, CS_CMD_TEXTBOX, CS_CMD_MISC, 46, 62, CS_CMD_PLAYBGM, CS_CMD_FADEBGM]);
    assert_eq!(end, Some(s.data.len()));
    assert_eq!((actor_action_slot(46), actor_action_slot(62)), (Some(0), Some(8)));
    let cue = CsCmdActorAction::read(&s.data, cmds[1].entries_offset);
    // CS_PLAYER_ACTION's fourth argument is the struct's rot.x; rot.y, the facing
    // func_808529D0 gives Link, is 0.
    assert_eq!((cue.action, cue.start_frame, cue.end_frame, cue.rot), (2, 0, 33, [0x54B2, 0, 0]));
    assert_eq!((cue.start_pos.to_array(), cue.end_pos.to_array()), ([2614, 0, -451], [2808, 0, -559]));
    // CMD_F(5.878788f) read as the Vec3i the struct declares.
    assert_eq!(cue.normal.x as u32, 5.878788f32.to_bits());
    // The first eye list: 5 points, the last CS_CMD_STOP; at list 0's third point waits 1000.
    let eye = CutsceneCameraPoint::read_list(&s.data, cmds[2].entries_offset);
    assert_eq!((eye.len(), eye[4].continue_flag, eye[0].pos), (5, CS_CMD_STOP, [2753, 46, -354]));
    let at = CutsceneCameraPoint::read_list(&s.data, cmds[4].entries_offset);
    assert_eq!((at[2].next_point_frame, at[0].view_angle), (1000, 47.199955));
    for n in ["D_808BD2A0", "D_808BD520", "D_808BD790"] {
        let s = a.cutscene(n).unwrap();
        assert_eq!(walk(&s.data).unwrap().1, Some(s.data.len()), "{n}");
    }
    let t = &a.cutscenes;
    assert_eq!(t.entrance_cutscenes.len(), 34, "sEntranceCutsceneTable's rows");
    let ydan = t.entrance_cutscenes.iter().find(|e| e.entrance_name == "ENTR_YDAN_0").unwrap();
    assert_eq!((ydan.entrance, ydan.age_restriction, ydan.flag as u16, ydan.script_name.as_str()), (0, 2, EVENTCHKINF_A8, "gDekuTreeIntroCs"));
    let intro = a.cutscene("gDekuTreeIntroCs").unwrap();
    assert_eq!((intro.file.as_str(), be_i32(&intro.data, 0), be_i32(&intro.data, 4)), ("ydan_scene", 4, 1270));
}

#[test]
fn the_b_spline_through_four_points() {
    use oot_game::camera::func_800bb2b4;
    let p = |x: i16, frame: u16, flag: i8| CutsceneCameraPoint { continue_flag: flag, camera_roll: 0, next_point_frame: frame, view_angle: 60.0, pos: [x, 0, 0] };
    let points = [p(0, 0, 0), p(600, 30, 0), p(1200, 30, 0), p(1800, 30, 0), p(2400, 30, 0), p(3000, 30, CS_CMD_STOP)];
    let (mut key, mut cur, mut fov) = (0i16, 0.0f32, 0.0f32);
    // u 0: (p0 + 4 p1 + p2) / 6 = 600; then 1/30 of a key a frame (both next points' 30).
    let (done, v) = func_800bb2b4(&points, &mut key, &mut cur, &mut fov);
    let (pos, _) = v.unwrap();
    assert!(!done);
    assert!((pos.x - 600.0).abs() < 1e-3, "{}", pos.x);
    assert!((cur - 1.0 / 30.0).abs() < 1e-6 && key == 0 && fov == 60.0);
    // The next key after 30 steps of 1/30 (31 if the f32 sum falls short of 1).
    let mut steps = 1;
    while key == 0 && steps < 40 {
        func_800bb2b4(&points, &mut key, &mut cur, &mut fov);
        steps += 1;
    }
    assert!(key == 1 && (30..=31).contains(&steps), "{steps}");
    // Key 1 is the last with three points after it: its end is the spline's.
    let mut done = false;
    for _ in 0..40 {
        done = func_800bb2b4(&points, &mut key, &mut cur, &mut fov).0;
        if done {
            break;
        }
    }
    assert!(done && key == 0);
}

#[test]
fn the_deku_trees_first_talk_and_yes_open_his_mouth() {
    let Some(a) = assets() else { return };
    // ENTR_SPOT04_1: the Deku Tree's meadow, Link walking in facing the tree.
    let Some(mut w) = enter(&a, "ENTR_SPOT04_1", |_| {}) else { return };
    let mut d = Driver { prev: PadState::default(), no: false };
    // func_808BC8B8: near and facing him, EVENTCHKINF_0C and D_808BCE20 (cutsceneTrigger 1);
    // func_800645A0 then starts it (cutsceneIndex 0xFFFD, CS_STATE_SKIPPABLE_INIT).
    d.until(&mut w, 30, "the talk's start", |w| w.cs_ctx.state != CS_STATE_IDLE);
    assert!(w.save.get_event_chk_inf(EVENTCHKINF_0C));
    assert_eq!((w.cs_ctx.segment.as_ref().unwrap().name.as_str(), w.save.cutscene_index, w.cs_ctx.state, w.save.cutscene_trigger), ("D_808BCE20", 0xFFFD, CS_STATE_SKIPPABLE_INIT, 0));
    assert_eq!(treemouth(&w).unwrap().action, bg_treemouth::Action::WaitCsStart);
    // The sub camera (D_8015FCC8, set by Environment_Init) on CAM_SET_CS_0 once both lists were
    // seen, the main camera waiting.
    assert_eq!((w.cs_ctx.sub_cam_id, w.active_cam_id, w.demo.d_8015fcc8), (1, 1, 1));
    assert_eq!((w.active_camera().setting, w.active_camera().status, w.game_camera.status), (CAM_SET_CS_0, CAM_STAT_ACTIVE, CAM_STAT_WAIT));
    // The spline of five equal points: the point.
    assert!(w.active_camera().eye.distance(Vec3::new(2753.0, 46.0, -354.0)) < 1e-3);
    // Link's cue 2 (D_808547C4[2] = 3): mode 6, then moved to the cue's start (more than 50 from
    // it), a child in Kokiri Forest 1 lower (func_808529D0).
    d.until(&mut w, 5, "cue 2's start", |w| w.player().unk_446 == 2);
    let p = w.player();
    assert_eq!((p.cs_mode, p.action), (6, Action::Cutscene));
    assert_eq!((p.actor.world_pos.x, p.actor.world_pos.z), (2614.0, -451.0));
    assert!(p.actor.world_pos.y <= -1.0 + 1e-3, "{}", p.actor.world_pos.y);
    // Cue 4 from frame 34: func_808519C0 stops short of its end, 4x its own pace from it.
    d.until(&mut w, 60, "cue 4", |w| w.player().unk_446 == 4);
    // Text 0x107D opens at frame 41 (after its start 40); the script holds at 59 while it's up.
    d.until(&mut w, 60, "0x107D", |w| w.msg_ctx.text_id == 0x107D);
    assert_eq!(w.cs_ctx.frames, 41);
    // Then 0x1015 at 161, which goes on to 0x1016, the question: at its end frame (170) the
    // script holds at 169 until it's answered.
    d.until(&mut w, 1200, "0x1016's choice", |w| w.message_state() == TEXT_STATE_CHOICE);
    assert_eq!((w.msg_ctx.text_id, w.cs_ctx.frames, w.msg_ctx.choice_index), (0x1016, 169, 0));
    // Yes; CS_MISC 12 at 180 ends the script (CS_STATE_UNSKIPPABLE_INIT), and func_808BC9EC
    // goes on at once with D_808BD520, EVENTCHKINF_05 set.
    d.until(&mut w, 60, "the answer's script", |w| w.cs_ctx.segment.as_ref().is_some_and(|s| s.name == "D_808BD520"));
    assert!(w.save.get_event_chk_inf(EVENTCHKINF_05));
    assert_eq!((w.cs_ctx.state, w.cs_ctx.frames), (CS_STATE_SKIPPABLE_EXEC, 1));
    assert_eq!(treemouth(&w).unwrap().action, bg_treemouth::Action::WaitCsCue);
    // D_808BD520's lists on the same sub camera: the eye at (3740, -141, -530).
    assert_eq!(w.active_cam_id, 1);
    assert!(w.active_camera().eye.distance(Vec3::new(3740.0, -141.0, -530.0)) < 1e-3);
    // Cue 3 for the mouth from frame 21 (after its start, 20): the mouth reads it in its next
    // update, before the script's frame 22 (Actor_UpdateAll runs before func_800645A0), and
    // opens by 0.01 a frame (func_808BC6F8); text 0x1017.
    d.until(&mut w, 30, "the mouth's cue 3", |w| treemouth(w).unwrap().action == bg_treemouth::Action::Open);
    assert_eq!((w.cs_ctx.frames, w.cs_ctx.npc_actions[0].unwrap().action), (22, 3));
    d.until(&mut w, 5, "0x1017", |w| w.msg_ctx.text_id == 0x1017);
    // CS_MISC 12 at 100: the end; func_80068DC0 brings the main camera back once unk_0C is 0.
    d.until(&mut w, 400, "the end", |w| w.cs_ctx.state == CS_STATE_UNSKIPPABLE_INIT);
    assert_eq!(w.cs_ctx.frames, 100);
    let n = d.until(&mut w, 20, "idle", |w| w.cs_ctx.state == CS_STATE_IDLE);
    assert_eq!(n, 10, "unk_0C from 1 to 0 by 0.1");
    assert_eq!((w.active_cam_id, w.camera(1).is_none(), w.game_camera.status, w.save.cutscene_index), (CAM_ID_MAIN, true, CAM_STAT_ACTIVE, 0));
    // Link: mode 7 on CS_STATE_UNSKIPPABLE_INIT (func_80852C50), then func_80852944: standing.
    d.until(&mut w, 5, "Link free", |w| w.player().cs_mode == 0);
    assert_eq!((w.player().action, w.player().unk_6AD), (Action::StandingStill, 0));
    // The mouth keeps opening to 1, and EVENTCHKINF_05 holds it there.
    d.until(&mut w, 100, "the mouth open", |w| treemouth(w).unwrap().unk_168 >= 1.0);
}

#[test]
fn no_says_0x1018_and_the_tree_waits_to_be_targeted() {
    let Some(a) = assets() else { return };
    let Some(mut w) = enter(&a, "ENTR_SPOT04_1", |_| {}) else { return };
    let mut d = Driver { prev: PadState::default(), no: true };
    d.until(&mut w, 1200, "the answer's script", |w| w.cs_ctx.segment.as_ref().is_some_and(|s| s.name != "D_808BCE20"));
    assert_eq!(w.cs_ctx.segment.as_ref().unwrap().name, "D_808BD790");
    assert!(!w.save.get_event_chk_inf(EVENTCHKINF_05));
    // func_808BC9EC's no: back to func_808BC8B8.
    assert_eq!(treemouth(&w).unwrap().action, bg_treemouth::Action::Wait);
    d.until(&mut w, 60, "0x1018", |w| w.msg_ctx.text_id == 0x1018);
    d.until(&mut w, 400, "the end", |w| w.cs_ctx.state == CS_STATE_IDLE && w.player().cs_mode == 0);
    // With EVENTCHKINF_0C, only Z-targeting him asks again (D_808BD2A0): no script now.
    for _ in 0..30 {
        d.tick(&mut w);
    }
    assert_eq!((w.cs_ctx.state, treemouth(&w).unwrap().action, treemouth(&w).unwrap().unk_168), (CS_STATE_IDLE, bg_treemouth::Action::Wait, 0.0));
}

#[test]
fn entering_the_deku_tree_the_first_time_plays_its_intro() {
    let Some(a) = assets() else { return };
    let Some(mut w) = enter(&a, "ENTR_YDAN_0", |_| {}) else { return };
    // Cutscene_HandleEntranceTriggers: EVENTCHKINF_A8 set and gDekuTreeIntroCs with
    // cutsceneTrigger 2, no title card.
    assert_eq!(w.scene_id, SCENE_YDAN);
    assert!(w.save.get_event_chk_inf(EVENTCHKINF_A8));
    assert_eq!((w.cs_ctx.segment.as_ref().unwrap().name.as_str(), w.save.cutscene_trigger, w.save.show_title_card), ("gDekuTreeIntroCs", 2, false));
    let mut d = Driver { prev: PadState::default(), no: false };
    d.tick(&mut w);
    assert_eq!((w.save.cutscene_index, w.cs_ctx.state, w.save.cutscene_trigger), (0xFFFD, CS_STATE_SKIPPABLE_INIT, 0));
    // No cue for Link: held still (mode 0x31) once his walk in is over.
    d.until(&mut w, 60, "Link held", |w| w.player().cs_mode == 0x31 && w.player().action == Action::Cutscene);
    assert_ne!(w.player().state1 & STATE1_29, 0);
    // CS_MISC 12 at 165: the end.
    d.until(&mut w, 300, "the end", |w| w.cs_ctx.state == CS_STATE_UNSKIPPABLE_INIT);
    assert_eq!(w.cs_ctx.frames, 165);
    d.until(&mut w, 20, "idle", |w| w.cs_ctx.state == CS_STATE_IDLE);
    // func_80851688: mode 0x31 ends by itself (mode 7) once no script runs.
    d.until(&mut w, 5, "Link free", |w| w.player().cs_mode == 0);
    assert_eq!(w.active_cam_id, CAM_ID_MAIN);
    // A second time in: the flag is set, no intro.
    let Some(mut w2) = enter(&a, "ENTR_YDAN_0", |s| s.set_event_chk_inf(EVENTCHKINF_A8)) else { return };
    assert!(w2.cs_ctx.segment.is_none() && w2.save.cutscene_trigger == 0);
    d.tick(&mut w2);
    assert_eq!(w2.cs_ctx.state, CS_STATE_IDLE);
}

#[test]
fn a_forced_text_holds_link_until_it_is_read() {
    let Some(a) = assets() else { return };
    // Out of the Kokiri shop: its forced En_Wonder_Talk2 (text 0x218) is by the door.
    let Some(mut w) = enter(&a, "ENTR_SPOT04_4", |_| {}) else { return };
    let mut d = Driver { prev: PadState::default(), no: false };
    for _ in 0..40 {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
    let spot = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<EnWonderTalk2>(h).filter(|s| s.base_msg_id == 0x18).map(|s| s.actor.world_pos)).expect("0x218's spot");
    w.place_player(spot + Vec3::new(10.0, 0.0, 0.0), 0);
    // func_80B3A4F8: in range, the text opens and func_8002DF54(8) holds Link: cutscene mode 8
    // through func_8083B998 / func_8083B040 (func_8083ADD4), PLAYER_STATE1_29.
    let hold = |w: &PlayState| w.player().cs_mode == 8 && w.player().action == Action::Cutscene;
    for _ in 0..10 {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
        if hold(&w) {
            break;
        }
    }
    assert!(hold(&w), "cs mode {}, action {:?}", w.player().cs_mode, w.player().action);
    assert_eq!(w.msg_ctx.text_id, 0x218);
    assert_ne!(w.player().state1 & STATE1_29, 0);
    // The stick does nothing while held (Player_Update zeroes the input under PLAYER_STATE1_29).
    let at = w.player().actor.world_pos;
    let stick = PadState { stick_y: 60, ..Default::default() };
    for _ in 0..5 {
        w.tick_with(scripted_input(stick, stick));
    }
    assert!(w.player().actor.world_pos.distance(at) < 1.0);
    // Read: func_80B3A3D4's func_8002DF54(7), then func_80852944 lets him go.
    d.until(&mut w, 400, "Link free", |w| w.message_state() == TEXT_STATE_NONE && w.player().cs_mode == 0);
    assert_eq!(w.player().state1 & STATE1_29, 0);
}
