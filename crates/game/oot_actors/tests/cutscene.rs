//! The cutscene system (`z_demo.c`, `oot_game::cutscene`), the Deku Tree's scripts
//! (`Bg_Treemouth`) and Player's cutscene modes, against the scripts' C and `z_demo.c`:
//! - `gDekuTreeMeetingCs` (`z_bg_treemouth_cutscene_data.c`): `CS_HEADER(12, 3000)`; Link's cues 2
//!   (frames 0..33, (2614, 0, -451) to (2808, 0, -559)) and 4 (33..42, to (2857, 0, -594)); eye
//!   and at lists from 0 and from 60; texts 0x107D (40..60) and 0x1015 (160..170, no branches);
//!   `CS_MISC(0x000C, 180, 200)`: the script's end; the mouth's cue 1 (list 46);
//! - `gDekuTreeMouthOpeningCs` (yes): `CS_HEADER(8, 3000)`; the mouth's cues 1 (0..20) and 3
//!   (20..357); text 0x1017 (20..60); `CS_MISC(0x000C, 100, 150)`;
//! - `gDekuTreeAskAgainCs` (no): text 0x1018 (20..60); `CS_MISC(0x000C, 80, 110)`;
//! - `gDekuTreeIntroCs` (`ydan_scene`): `CS_HEADER(4, 1270)`, `CS_MISC(0x000C, 165, 166)`,
//!   a title card (`CS_MISC` 15), an eye and an at list; `sEntranceCutsceneTable` plays it on
//!   `ENTR_DEKU_TREE_0` for either age with `EVENTCHKINF_A8`.

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

/// `EVENTCHKINF_A8` (`save.h`): the Deku Tree's intro seen.
const EVENTCHKINF_A8: u16 = 0xA8;
const SCENE_DEKU_TREE: u16 = 0x00;

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
    let s = a.cutscene("gDekuTreeMeetingCs").unwrap();
    assert_eq!((s.file.as_str(), be_i32(&s.data, 0), be_i32(&s.data, 4)), ("ovl_Bg_Treemouth", 12, 3000));
    let (cmds, end) = walk(&s.data).unwrap();
    let types: Vec<i32> = cmds.iter().map(|c| c.cmd_type).collect();
    // CS_UNK_DATA_LIST(0x15), the player cues, 2 eye lists, 2 at lists, the texts, the misc, the
    // mouth's cues (46: npcActions[0]), Navi's (62, CS_CMD_ACTOR_CUE_8_0: npcActions[8]),
    // the BGM and its fade.
    assert_eq!(types, vec![0x15, CS_CMD_PLAYER_CUE, CS_CMD_CAM_EYE_SPLINE, CS_CMD_CAM_EYE_SPLINE, CS_CMD_CAM_AT_SPLINE, CS_CMD_CAM_AT_SPLINE, CS_CMD_TEXT, CS_CMD_MISC, 46, 62, CS_CMD_START_SEQ, CS_CMD_FADE_OUT_SEQ]);
    assert_eq!(end, Some(s.data.len()));
    assert_eq!((actor_action_slot(46), actor_action_slot(62)), (Some(0), Some(8)));
    let cue = CsCmdActorCue::read(&s.data, cmds[1].entries_offset);
    // CS_PLAYER_CUE's fourth argument is the struct's rot.x; rot.y, the facing
    // func_808529D0 gives Link, is 0.
    assert_eq!((cue.action, cue.start_frame, cue.end_frame, cue.rot), (2, 0, 33, [0x54B2, 0, 0]));
    assert_eq!((cue.start_pos.to_array(), cue.end_pos.to_array()), ([2614, 0, -451], [2808, 0, -559]));
    // CMD_F(5.878788f) read as the Vec3i the struct declares.
    assert_eq!(cue.normal.x as u32, 5.878788f32.to_bits());
    // The first eye list: 5 points, the last CS_CAM_STOP; at list 0's third point waits 1000.
    let eye = CutsceneCameraPoint::read_list(&s.data, cmds[2].entries_offset);
    assert_eq!((eye.len(), eye[4].continue_flag, eye[0].pos), (5, CS_CAM_STOP, [2753, 46, -354]));
    let at = CutsceneCameraPoint::read_list(&s.data, cmds[4].entries_offset);
    assert_eq!((at[2].next_point_frame, at[0].view_angle), (1000, 47.199955));
    for n in ["gDekuTreeChoiceCs", "gDekuTreeMouthOpeningCs", "gDekuTreeAskAgainCs"] {
        let s = a.cutscene(n).unwrap();
        assert_eq!(walk(&s.data).unwrap().1, Some(s.data.len()), "{n}");
    }
    let t = &a.cutscenes;
    assert_eq!(t.entrance_cutscenes.len(), 34, "sEntranceCutsceneTable's rows");
    let ydan = t.entrance_cutscenes.iter().find(|e| e.entrance_name == "ENTR_DEKU_TREE_0").unwrap();
    assert_eq!((ydan.entrance, ydan.age_restriction, ydan.flag as u16, ydan.script_name.as_str()), (0, 2, EVENTCHKINF_A8, "gDekuTreeIntroCs"));
    let intro = a.cutscene("gDekuTreeIntroCs").unwrap();
    assert_eq!((intro.file.as_str(), be_i32(&intro.data, 0), be_i32(&intro.data, 4)), ("ydan_scene", 4, 1270));
}

#[test]
fn the_b_spline_through_four_points() {
    use oot_game::camera::func_800bb2b4;
    let p = |x: i16, frame: u16, flag: i8| CutsceneCameraPoint { continue_flag: flag, camera_roll: 0, next_point_frame: frame, view_angle: 60.0, pos: [x, 0, 0] };
    let points = [p(0, 0, 0), p(600, 30, 0), p(1200, 30, 0), p(1800, 30, 0), p(2400, 30, 0), p(3000, 30, CS_CAM_STOP)];
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
    // ENTR_KOKIRI_FOREST_1: the Deku Tree's meadow, Link walking in facing the tree.
    let Some(mut w) = enter(&a, "ENTR_KOKIRI_FOREST_1", |_| {}) else { return };
    let mut d = Driver { prev: PadState::default(), no: false };
    // func_808BC8B8: near and facing him, EVENTCHKINF_0C and gDekuTreeMeetingCs (cutsceneTrigger 1);
    // Cutscene_UpdateScripted then starts it (cutsceneIndex 0xFFFD, CS_STATE_START).
    d.until(&mut w, 30, "the talk's start", |w| w.cs_ctx.state != CS_STATE_IDLE);
    assert!(w.save.get_event_chk_inf(EVENTCHKINF_0C));
    assert_eq!((w.cs_ctx.segment.as_ref().unwrap().name.as_str(), w.save.cutscene_index, w.cs_ctx.state, w.save.cutscene_trigger), ("gDekuTreeMeetingCs", 0xFFFD, CS_STATE_START, 0));
    assert_eq!(treemouth(&w).unwrap().action, bg_treemouth::Action::WaitCsStart);
    // The sub camera (gUseCutsceneCam, set by Environment_Init) on CAM_SET_CS_0 once both lists were
    // seen, the main camera waiting.
    assert_eq!((w.cs_ctx.sub_cam_id, w.active_cam_id, w.demo.use_cutscene_cam), (1, 1, 1));
    assert_eq!((w.active_camera().setting, w.active_camera().status, w.game_camera.status), (CAM_SET_CS_0, CAM_STAT_ACTIVE, CAM_STAT_WAIT));
    // The spline of five equal points: the point.
    assert!(w.active_camera().eye.distance(Vec3::new(2753.0, 46.0, -354.0)) < 1e-3);
    // Link's cue 2 (sCueToCsActionMap[2] = 3): mode 6, then moved to the cue's start (more than 50 from
    // it), a child in Kokiri Forest 1 lower (func_808529D0).
    d.until(&mut w, 5, "cue 2's start", |w| w.player().cue_id == 2);
    let p = w.player();
    assert_eq!((p.cs_mode, p.action), (6, Action::Cutscene));
    assert_eq!((p.actor.world_pos.x, p.actor.world_pos.z), (2614.0, -451.0));
    assert!(p.actor.world_pos.y <= -1.0 + 1e-3, "{}", p.actor.world_pos.y);
    // Cue 4 from frame 34: func_808519C0 stops short of its end, 4x its own pace from it.
    d.until(&mut w, 60, "cue 4", |w| w.player().cue_id == 4);
    // Text 0x107D opens at frame 41 (after its start 40); the script holds at 59 while it's up.
    d.until(&mut w, 60, "0x107D", |w| w.msg_ctx.text_id == 0x107D);
    assert_eq!(w.cs_ctx.frames, 41);
    // Then 0x1015 at 161, which goes on to 0x1016, the question: at its end frame (170) the
    // script holds at 169 until it's answered.
    d.until(&mut w, 1200, "0x1016's choice", |w| w.message_state() == TEXT_STATE_CHOICE);
    assert_eq!((w.msg_ctx.text_id, w.cs_ctx.frames, w.msg_ctx.choice_index), (0x1016, 169, 0));
    // Yes; CS_MISC 12 at 180 ends the script (CS_STATE_STOP), and func_808BC9EC
    // goes on at once with gDekuTreeMouthOpeningCs, EVENTCHKINF_05 set.
    d.until(&mut w, 60, "the answer's script", |w| w.cs_ctx.segment.as_ref().is_some_and(|s| s.name == "gDekuTreeMouthOpeningCs"));
    assert!(w.save.get_event_chk_inf(EVENTCHKINF_05));
    assert_eq!((w.cs_ctx.state, w.cs_ctx.frames), (CS_STATE_RUN, 1));
    assert_eq!(treemouth(&w).unwrap().action, bg_treemouth::Action::WaitCsCue);
    // gDekuTreeMouthOpeningCs's lists on the same sub camera: the eye at (3740, -141, -530).
    assert_eq!(w.active_cam_id, 1);
    assert!(w.active_camera().eye.distance(Vec3::new(3740.0, -141.0, -530.0)) < 1e-3);
    // Cue 3 for the mouth from frame 21 (after its start, 20): the mouth reads it in its next
    // update, before the script's frame 22 (Actor_UpdateAll runs before Cutscene_UpdateScripted), and
    // opens by 0.01 a frame (func_808BC6F8); text 0x1017.
    d.until(&mut w, 30, "the mouth's cue 3", |w| treemouth(w).unwrap().action == bg_treemouth::Action::Open);
    assert_eq!((w.cs_ctx.frames, w.cs_ctx.npc_actions[0].unwrap().action), (22, 3));
    d.until(&mut w, 5, "0x1017", |w| w.msg_ctx.text_id == 0x1017);
    // CS_MISC 12 at 100: the end; CutsceneHandler_StopScript brings the main camera back once timer is 0.
    d.until(&mut w, 400, "the end", |w| w.cs_ctx.state == CS_STATE_STOP);
    assert_eq!(w.cs_ctx.frames, 100);
    let n = d.until(&mut w, 20, "idle", |w| w.cs_ctx.state == CS_STATE_IDLE);
    assert_eq!(n, 10, "timer from 1 to 0 by 0.1");
    assert_eq!((w.active_cam_id, w.camera(1).is_none(), w.game_camera.status, w.save.cutscene_index), (CAM_ID_MAIN, true, CAM_STAT_ACTIVE, 0));
    // Link: mode 7 on CS_STATE_STOP (func_80852C50), then func_80852944: standing.
    d.until(&mut w, 5, "Link free", |w| w.player().cs_mode == 0);
    assert_eq!((w.player().action, w.player().unk_6AD), (Action::StandingStill, 0));
    // The mouth keeps opening to 1, and EVENTCHKINF_05 holds it there.
    d.until(&mut w, 100, "the mouth open", |w| treemouth(w).unwrap().unk_168 >= 1.0);
}

/// The opening, from the file select's new file (`Sram_InitSave`: `ENTR_LINKS_HOUSE_0`,
/// `cutsceneIndex` 0xFFF1), through the chain its terminators make:
/// - Link's house's cutscene layer 5 (0xFFF1 & 0xF + 4), its script at 0x15D0 (`gLinkHouseIntroSleepCs`): Link asleep (cue
///   0x1C, `sCueToCsActionMap` mode 38: `clink_op3_wait1`, no shadow), `TRANS_TYPE_CS_BLACK_FILL`
///   (`ENTR_LINKS_HOUSE_0_5`) held black until `TRANSITION_FX` 12 (55..81) lowers
///   `cutsceneTransitionControl` from 255 to 100; texts 0x109D (40), 0x109E, 0x109F; the
///   terminator at 280 to destination 35: `ENTR_HYRULE_FIELD_0`, 0xFFF0, `TRANS_TYPE_FADE_BLACK_FAST`;
/// - Hyrule Field's layer 4, the nightmare: Link held (cue 5, mode 8) at (-1, 0, 1348), walked
///   (cue 1, mode 3), turning (cue 6, mode 9: `link_demo_furimuki`); the terminator at 540 to 11:
///   `ENTR_KOKIRI_FOREST_0`, 0xFFF3, `TRANS_TYPE_FADE_WHITE`;
/// - Kokiri Forest's layer 7: `ENTR_KOKIRI_FOREST_0_7`'s `TRANS_TYPE_FADE_WHITE_CS_DELAYED` waits white
///   (`TRANS_MODE_INSTANCE_WAIT`) until `TRANSITION_FX` 9 (from 70) sets
///   `cutsceneTransitionControl`; Link held (no cues: mode 0x31); Navi on her cue 1 (mode 10);
///   the terminator at 940 to 10: `ENTR_LINKS_HOUSE_0`, 0xFFF0, `TRANS_TYPE_FADE_BLACK`;
/// - the house's layer 4, the wake-up: Navi's cues, Link's 0x1C, 0x1D (mode 39: tossing), 0x1E
///   (40: sitting up, the shadow back at its frame 240), 0x1F (41: out of bed) and 5 (8: held at
///   (0, 0, 60)); `CS_MISC` 14 at 645 (`VIEWPOINT_LOCKED`) and 12 at 647, the end: Link free,
///   `cutsceneIndex` 0.
#[test]
fn the_opening_plays_from_the_file_selects_new_file() {
    use oot_game::transition::*;
    let Some(a) = assets() else { return };
    let save = SaveContext::file_select_new();
    assert_eq!((save.entrance_index, save.cutscene_index, save.adult, save.day_time), (a.scenes.entrance_index("ENTR_LINKS_HOUSE_0").unwrap(), 0xFFF1, false, 0x6AAB));
    let mut w = oot_actors::play_entrance(a.clone(), common::data().unwrap(), common::rules().unwrap(), save).expect("Play_Init");
    const SCENE_LINKS_HOUSE: u16 = 0x34;
    const SCENE_HYRULE_FIELD: u16 = 0x51;
    const SCENE_KOKIRI_FOREST: u16 = 0x55;
    // Play_Init: SCENE_LAYER_CUTSCENE_FIRST + 1, the layer's script (no XML names it: keyed by
    // its offset), the entrance's CS black fill.
    assert_eq!((w.scene_id, w.save.scene_layer), (SCENE_LINKS_HOUSE, 5));
    assert_eq!(w.cs_ctx.segment.as_ref().map(|s| (s.file.as_str(), s.name.as_str())), Some(("link_home_scene", "gLinkHouseIntroSleepCs")));
    assert_eq!((w.transition.ty, w.save.cutscene_transition_control), (TRANS_TYPE_CS_BLACK_FILL, 0));
    let mut d = Driver { prev: PadState::default(), no: false };
    d.tick(&mut w);
    // Cutscene_UpdateScripted → Cutscene_SetupScripted: no trigger, so the letterbox at once and the script's
    // frame 1 already run (CS_STATE_RUN); the fill's black.
    assert_eq!((w.cs_ctx.state, w.cs_ctx.frames, w.transition.mode), (CS_STATE_RUN, 1, TRANS_MODE_CS_BLACK_FILL));
    assert_eq!(w.screen_fill(), Some([0, 0, 0, 255]));
    // Cue 0x1C (mode 38), once the spawn's walk-in (start mode 0xF) hands over to the cutscene
    // (Player_ActionHandler_0): at its start (0, 2, 116) facing 0x8000, asleep.
    d.until(&mut w, 30, "mode 38", |w| w.player().cue_id == 0x1C);
    let p = w.player();
    assert_eq!((p.cs_mode, p.actor.world_pos, p.actor.shape_rot.y, w.data.anim_name(p.skel.animation), p.shadow_feet), (6, Vec3::new(0.0, 2.0, 116.0), -0x8000, "clink_op3_wait1", false));
    // The first text at 41; the fill stays black until the FX lowers it.
    d.until(&mut w, 60, "0x109D", |w| w.msg_ctx.text_id == 0x109D);
    assert_eq!((w.cs_ctx.frames, w.screen_fill()), (41, Some([0, 0, 0, 255])));
    // TRANSITION_FX 12: 255 - 155 × Environment_LerpWeight(81, 55, frame); the fill follows it
    // a frame later, and the transition ends at 100.
    d.until(&mut w, 2000, "the fill's end", |w| w.transition.mode == TRANS_MODE_OFF);
    assert_eq!((w.cs_ctx.frames, w.save.cutscene_transition_control, w.screen_fill()), (82, 100, Some([0, 0, 0, 100])));
    // The terminator: destination 35.
    d.until(&mut w, 3000, "the nightmare", |w| w.scene_id == SCENE_HYRULE_FIELD);
    assert_eq!((w.save.scene_layer, w.save.entrance_index), (4, a.scenes.entrance_index("ENTR_HYRULE_FIELD_0").unwrap()));
    // Cue 5 (mode 8): held at its start.
    d.until(&mut w, 30, "cue 5", |w| w.player().cue_id == 5);
    assert_eq!((w.player().cs_mode, w.player().actor.world_pos), (6, Vec3::new(-1.0, 0.0, 1348.0)));
    d.until(&mut w, 400, "cue 6", |w| w.player().cue_id == 6);
    assert_eq!(w.data.anim_name(w.player().skel.animation), "link_demo_furimuki");
    // Destination 11.
    d.until(&mut w, 600, "Kokiri Forest's layer 7", |w| w.scene_id == SCENE_KOKIRI_FOREST);
    assert_eq!(w.save.scene_layer, 7);
    assert_eq!(w.transition.ty, TRANS_TYPE_FADE_WHITE_CS_DELAYED);
    d.tick(&mut w);
    assert_eq!((w.transition.mode, w.screen_fill()), (TRANS_MODE_INSTANCE_WAIT, Some([160, 160, 160, 255])));
    // TRANSITION_FX 9 from 70: the fade runs.
    d.until(&mut w, 200, "the delayed fade", |w| w.transition.mode == TRANS_MODE_INSTANCE_RUNNING);
    assert_eq!((w.cs_ctx.frames, w.save.cutscene_transition_control), (71, 1));
    // Link held (no cues: mode 0x31); Navi on her cue 1 (func_80A0461C: mode 10).
    assert_eq!(w.player().cs_mode, 0x31);
    let navi = w.player().navi_actor.and_then(|h| w.actors.downcast::<oot_actors::en_elf::EnElf>(h)).unwrap();
    assert_eq!((navi.unk_2a8, w.cs_ctx.npc_actions[8].map(|c| c.action)), (10, Some(1)));
    // Destination 10: the house's layer 4.
    d.until(&mut w, 3000, "the wake-up", |w| w.scene_id == SCENE_LINKS_HOUSE);
    assert_eq!((w.save.scene_layer, w.transition.ty), (4, TRANS_TYPE_FADE_BLACK_FAST));
    for (cue, mode, anim) in [(0x1D, 39, "clink_op3_negaeri"), (0x1E, 40, "clink_op3_okiagari"), (0x1F, 41, "clink_op3_tatiagari")] {
        d.until(&mut w, 2000, "the next cue", |w| w.player().cue_id == cue);
        // A negative sCueToCsActionMap mode: no move to the cue's start.
        assert_eq!(w.data.anim_name(w.player().skel.animation), anim, "cue {cue:#x}, mode {mode}");
        if cue == 0x1F {
            assert!(w.player().shadow_feet, "func_80851FB0 put the shadow back");
        }
    }
    // Cue 5 at 645: held at (0, 0, 60); CS_MISC 14 (645) and 12 (647): the end.
    d.until(&mut w, 400, "cue 5", |w| w.player().cue_id == 5);
    assert_eq!(w.player().actor.world_pos, Vec3::new(0.0, 0.0, 60.0));
    d.until(&mut w, 100, "the end", |w| w.cs_ctx.state == CS_STATE_IDLE && w.player().cs_mode == 0);
    let p = w.player();
    assert_eq!((w.save.cutscene_index, w.viewpoint, p.action), (0, oot_game::play::VIEWPOINT_LOCKED, Action::StandingStill));
    assert_eq!(p.actor.shape_rot.y, -0x8000);
}

#[test]
fn no_says_0x1018_and_the_tree_waits_to_be_targeted() {
    let Some(a) = assets() else { return };
    let Some(mut w) = enter(&a, "ENTR_KOKIRI_FOREST_1", |_| {}) else { return };
    let mut d = Driver { prev: PadState::default(), no: true };
    d.until(&mut w, 1200, "the answer's script", |w| w.cs_ctx.segment.as_ref().is_some_and(|s| s.name != "gDekuTreeMeetingCs"));
    assert_eq!(w.cs_ctx.segment.as_ref().unwrap().name, "gDekuTreeAskAgainCs");
    assert!(!w.save.get_event_chk_inf(EVENTCHKINF_05));
    // func_808BC9EC's no: back to func_808BC8B8.
    assert_eq!(treemouth(&w).unwrap().action, bg_treemouth::Action::Wait);
    d.until(&mut w, 60, "0x1018", |w| w.msg_ctx.text_id == 0x1018);
    d.until(&mut w, 400, "the end", |w| w.cs_ctx.state == CS_STATE_IDLE && w.player().cs_mode == 0);
    // With EVENTCHKINF_0C, only Z-targeting him asks again (gDekuTreeChoiceCs): no script now.
    for _ in 0..30 {
        d.tick(&mut w);
    }
    assert_eq!((w.cs_ctx.state, treemouth(&w).unwrap().action, treemouth(&w).unwrap().unk_168), (CS_STATE_IDLE, bg_treemouth::Action::Wait, 0.0));
}

#[test]
fn entering_the_deku_tree_the_first_time_plays_its_intro() {
    let Some(a) = assets() else { return };
    let Some(mut w) = enter(&a, "ENTR_DEKU_TREE_0", |_| {}) else { return };
    // Cutscene_HandleEntranceTriggers: EVENTCHKINF_A8 set and gDekuTreeIntroCs with
    // cutsceneTrigger 2, showTitleCard false, so Player_Init shows no title card (and sets it
    // back to true).
    assert_eq!(w.scene_id, SCENE_DEKU_TREE);
    assert!(w.save.get_event_chk_inf(EVENTCHKINF_A8));
    assert_eq!((w.cs_ctx.segment.as_ref().unwrap().name.as_str(), w.save.cutscene_trigger, w.save.show_title_card), ("gDekuTreeIntroCs", 2, true));
    assert_eq!((w.title_ctx.texture.as_deref(), w.title_ctx.duration_timer), (None, 0));
    let mut d = Driver { prev: PadState::default(), no: false };
    d.tick(&mut w);
    // CS_MISC 15 at frame 0 (run with frame 1 on the first tick): TitleCard_InitPlaceName with
    // the Deku Tree's place name (g_pn_06) at (160, 120), 144 by 24, after a delay of 20 (then one
    // TitleCard_Update in the next frame's Actor_UpdateAll).
    assert_eq!((w.title_ctx.texture.as_deref(), w.title_ctx.x, w.title_ctx.y, w.title_ctx.width, w.title_ctx.height), (Some("title/g_pn_06"), 160, 120, 144, 24));
    assert_eq!((w.title_ctx.delay_timer, w.title_ctx.duration_timer, w.title_ctx.alpha), (20, 80, 0));
    assert_eq!((w.save.cutscene_index, w.cs_ctx.state, w.save.cutscene_trigger), (0xFFFD, CS_STATE_START, 0));
    // No cue for Link: held still (mode 0x31) once his walk in is over.
    d.until(&mut w, 60, "Link held", |w| w.player().cs_mode == 0x31 && w.player().action == Action::Cutscene);
    assert_ne!(w.player().state1 & STATE1_29, 0);
    // CS_MISC 12 at 165: the end.
    d.until(&mut w, 300, "the end", |w| w.cs_ctx.state == CS_STATE_STOP);
    assert_eq!(w.cs_ctx.frames, 165);
    d.until(&mut w, 20, "idle", |w| w.cs_ctx.state == CS_STATE_IDLE);
    // func_80851688: mode 0x31 ends by itself (mode 7) once no script runs.
    d.until(&mut w, 5, "Link free", |w| w.player().cs_mode == 0);
    assert_eq!(w.active_cam_id, CAM_ID_MAIN);
    // A second time in: the flag is set, no intro.
    let Some(mut w2) = enter(&a, "ENTR_DEKU_TREE_0", |s| s.set_event_chk_inf(EVENTCHKINF_A8)) else { return };
    assert!(w2.cs_ctx.segment.is_none() && w2.save.cutscene_trigger == 0);
    d.tick(&mut w2);
    assert_eq!(w2.cs_ctx.state, CS_STATE_IDLE);
}

#[test]
fn a_forced_text_holds_link_until_it_is_read() {
    let Some(a) = assets() else { return };
    // Out of the Kokiri shop: its forced En_Wonder_Talk2 (text 0x218) is by the door.
    let Some(mut w) = enter(&a, "ENTR_KOKIRI_FOREST_4", |_| {}) else { return };
    let mut d = Driver { prev: PadState::default(), no: false };
    for _ in 0..40 {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
    let spot = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<EnWonderTalk2>(h).filter(|s| s.base_msg_id == 0x18).map(|s| s.actor.world_pos)).expect("0x218's spot");
    w.place_player(spot + Vec3::new(10.0, 0.0, 0.0), 0);
    // func_80B3A4F8: in range, the text opens and Player_SetCsActionWithHaltedActors(8) holds Link: cutscene mode 8
    // through Player_ActionHandler_0 / Player_ActionHandler_13 (Player_StartCsAction), PLAYER_STATE1_29.
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
    // Read: func_80B3A3D4's Player_SetCsActionWithHaltedActors(7), then func_80852944 lets him go.
    d.until(&mut w, 400, "Link free", |w| w.message_state() == TEXT_STATE_NONE && w.player().cs_mode == 0);
    assert_eq!(w.player().state1 & STATE1_29, 0);
}

/// A script's entries of command type `ty`: their offsets.
fn entries_of(s: &CutsceneScript, ty: i32) -> Vec<usize> {
    let (cmds, _) = walk(&s.data).unwrap();
    cmds.iter().filter(|c| c.cmd_type == ty).flat_map(|c| (0..c.entries).map(move |i| c.entries_offset + i * c.entry_size)).collect()
}

#[test]
fn the_deku_trees_talk_plays_its_music() {
    // CutsceneCmd_FadeOutSequence (CsCmdFadeOutSeq: the type, start and end frames),
    // CutsceneCmd_StartSequence and _StopBGM (CsCmdStartSeq: the sequence, plus one, at byte
    // 1), each on its start frame: Audio_QueueSeqCmd(SEQCMD_STOP_SEQUENCE over the fade's
    // frames), Audio_PlaySequenceInCutscene and Audio_StopSequenceInCutscene.
    let Some(a) = assets() else { return };
    let Some(mut w) = enter(&a, "ENTR_KOKIRI_FOREST_1", |_| {}) else { return };
    w.audio.log = Some(Default::default());
    let mut d = Driver { prev: PadState::default(), no: false };
    // Each game frame, the script and its frame after it.
    let mut seen: Vec<(u32, String, u16)> = Vec::new();
    for _ in 0..2000 {
        d.tick(&mut w);
        if w.cs_ctx.state != CS_STATE_IDLE
            && let Some(s) = &w.cs_ctx.segment
        {
            seen.push((w.audio.frames, s.name.clone(), w.cs_ctx.frames));
        }
        if seen.last().is_some_and(|(_, n, f)| n == "gDekuTreeMouthOpeningCs" && *f > 110) {
            break;
        }
    }
    // The game frame on which `script` first reached `frame`.
    let at = |script: &str, frame: u16| seen.iter().find(|(_, n, f)| n == script && *f >= frame).map(|s| s.0).unwrap_or_else(|| panic!("{script} never reached {frame}"));
    let cmds = |frame: u32| -> Vec<u32> { w.audio.log.as_ref().unwrap().seq_cmds.iter().filter(|c| c.0 == frame).map(|c| c.1).collect() };
    let talk = a.cutscene("gDekuTreeMeetingCs").unwrap();
    let yes = a.cutscene("gDekuTreeMouthOpeningCs").unwrap();
    // The talk's fade (type 4, frames 0..20): the main bgm stopped over 20 frames, on the
    // script's first frame (Cutscene_SetupScripted runs frames 0 and 1 together).
    let o = entries_of(&talk, CS_CMD_FADE_OUT_SEQ)[0];
    let (ty, start, end) = (be_u16(&talk.data, o), be_u16(&talk.data, o + 2), be_u16(&talk.data, o + 4));
    assert_eq!((ty, start, end), (4, 0, 20));
    let player = if ty == 3 { 1 } else { 0 };
    let want = ((end - start) as u8 as u32) << 16 | (1 << 28 | player << 24 | 0xFF);
    assert!(cmds(at("gDekuTreeMeetingCs", 1)).contains(&want), "{:#x?}", cmds(at("gDekuTreeMeetingCs", 1)));
    // Its music at frame 140: SEQCMD_PLAY_SEQUENCE (op 0) of sequence 0x4C - 1.
    let o = entries_of(&talk, CS_CMD_START_SEQ)[0];
    let (seq, frame) = (talk.data[o + 1] as u32 - 1, be_u16(&talk.data, o + 2));
    assert_eq!((seq, frame), (0x4B, 140));
    let c = cmds(at("gDekuTreeMeetingCs", frame));
    assert!(c.iter().any(|&c| c >> 28 == 0 && c & 0xFF == seq), "{c:#x?}");
    // Yes: at 90 it stops that sequence (SEQCMD_STOP_SEQUENCE, op 1, no fade), at 99 plays 0x3D - 1.
    let o = entries_of(&yes, CS_CMD_STOP_SEQ)[0];
    let frame = be_u16(&yes.data, o + 2);
    assert_eq!((yes.data[o + 1], frame), (0x4C, 90));
    let c = cmds(at("gDekuTreeMouthOpeningCs", frame));
    assert!(c.iter().any(|&c| c >> 28 == 1 && (c >> 16) & 0xFF == 0), "{c:#x?}");
    let o = entries_of(&yes, CS_CMD_START_SEQ)[0];
    let (seq, frame) = (yes.data[o + 1] as u32 - 1, be_u16(&yes.data, o + 2));
    assert_eq!((seq, frame), (0x3C, 99));
    let c = cmds(at("gDekuTreeMouthOpeningCs", frame));
    assert!(c.iter().any(|&c| c >> 28 == 0 && c & 0xFF == seq), "{c:#x?}");
}

#[test]
fn link_groans_and_sighs_as_navi_wakes_him() {
    // The wake-up (Link's house's layer 4): mode 39's start, func_80851E90, groans
    // (Player_PlayVoiceSfx(NA_SE_VO_LI_GROAN): his voice, plus the age's voice offset unk_92); mode
    // 40's update, func_80851FB0, plays D_808551BC on its animation's frames: the sigh
    // (NA_SE_VO_LI_RELAX, a voice) on 35, the slips off the bed (NA_SE_PL_SLIPDOWN) on 236 and
    // 256.
    use oot_game::audio::sfx::{NA_SE_PL_SLIPDOWN, NA_SE_VO_LI_GROAN, NA_SE_VO_LI_RELAX, SfxPos};
    let Some(a) = assets() else { return };
    let save = SaveContext::file_select_new();
    let mut w = oot_actors::play_entrance(a.clone(), common::data().unwrap(), common::rules().unwrap(), save).expect("Play_Init");
    w.audio.log = Some(Default::default());
    let mut d = Driver { prev: PadState::default(), no: false };
    // Past the narration, the nightmare and Navi's flight, to the wake-up's cue 0x1D (mode 39).
    d.until(&mut w, 5000, "the wake-up's toss", |w| w.save.scene_layer == 4 && w.scene_id == 0x34 && w.player().cue_id == 0x1D);
    let toss = w.audio.frames;
    let voice = w.player().age.climb.unk_92;
    let me = w.player;
    let sfx = |w: &PlayState, id: u16| -> Vec<u32> {
        w.audio.log.as_ref().unwrap().sfx.iter().filter(|s| s.1 == id && s.2 == SfxPos::Actor(me.unwrap())).map(|s| s.0).collect()
    };
    // (The narration, Link's house's layer 5, tosses him once too: cue 0x1D.)
    let groans = sfx(&w, NA_SE_VO_LI_GROAN + voice);
    assert_eq!((groans.len(), groans.last()), (2, Some(&toss)), "{groans:?}");
    d.until(&mut w, 2000, "mode 40", |w| w.player().cue_id == 0x1E);
    let sit = w.audio.frames;
    d.until(&mut w, 400, "the sit-up's end", |w| w.player().cue_id != 0x1E || w.player().action_var2 != 0);
    // On frames 35, 236 and 256 of the animation: LinkAnimation_Update advances it by playSpeed
    // (1) x R_UPDATE_RATE (3) x 0.5 a game frame, from the mode's start, which updates it once,
    // and LinkAnimation_OnFrame(n) is the frame it passes n: ceil(n / 1.5) frames on.
    let relax = sfx(&w, NA_SE_VO_LI_RELAX + voice);
    let slips = sfx(&w, NA_SE_PL_SLIPDOWN);
    assert_eq!(relax.len(), 1, "{relax:?}");
    assert_eq!(slips.len(), 2, "{slips:?}");
    let on = |n: f32| sit + (n / 1.5).ceil() as u32 - 1;
    assert_eq!((relax[0], slips[0], slips[1]), (on(35.0), on(236.0), on(256.0)));
}
