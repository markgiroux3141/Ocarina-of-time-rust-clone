//! One-point cutscenes (`z_onepointdemo.c`, GAME-04b milestone 1): the attention camera
//! (`OnePointCutscene_Attention`, `Camera_Demo5` into `Camera_Unique9`) and the falling chest's
//! fixed shot (4500, `CAM_SET_FREE2`), in Kokiri Forest on a new save. The crawlspace's exits
//! (9601, 9602, `Camera_Demo9`) are in `crawl.rs`.
//!
//! Expected values come from the C: `OnePointCutscene_Init`'s queue and statuses,
//! `OnePointCutscene_SetInfo`'s cases, `Camera_Demo5`'s branches with `z_camera_data.inc.c`'s
//! keyframe tables (`D_8011D9F4`), `Camera_Unique9`'s keyframe timing, and `Camera_Finish`.

mod common;

use std::sync::Arc;

use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::player::Action;
use oot_game::actor_ctx::ACTORCAT_NPC;
use oot_game::audio::sfx::{NA_SE_SY_CORRECT_CHIME, SfxPos};
use oot_game::camera::{CAM_ID_MAIN, CAM_SET_CS_ATTENTION, CAM_STAT_ACTIVE, CAM_STAT_UNK3, GameCamera, VecSphGeo, diff_to_sph_geo, sph_geo_add, vec3_to_sph_geo};
use oot_game::onepoint::{CAM_SET_CS_C, CAM_SET_FREE2};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Kokiri Forest's village (`ENTR_KOKIRI_FOREST_3`) on a new save, settled.
fn village() -> Option<PlayState> {
    let a = assets()?;
    let e = a.scenes.entrance_index("ENTR_KOKIRI_FOREST_3").expect("entrance");
    let save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    let mut w = oot_actors::play_entrance(a, common::data()?, common::rules()?, save).expect("Play_Init");
    frames(&mut w, 20);
    Some(w)
}

fn frames(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

#[test]
fn an_attention_cutscene_on_a_far_npc() {
    let Some(mut w) = village() else { return };
    // A Kokiri child (ACTORCAT_NPC), with Link put 900 from it facing it, so the main camera's
    // eye (180 behind him) is more than 700 from it: Camera_Demo5's last branch (D_8011D9F4).
    let kid = w.actors.all().into_iter().find(|&h| w.actors.actor(h).is_some_and(|a| a.category == ACTORCAT_NPC && a.id == oot_actors::en_ko::ACTOR_EN_KO)).expect("a Kokiri");
    let kid_pos = w.actors.actor(kid).unwrap().world_pos;
    let away = Vec3::new(900.0, 0.0, 0.0);
    let mut at = kid_pos + away;
    at.y = w.col.entity_raycast_down(at + Vec3::Y * 200.0).0;
    let yaw = oot_game::camera::cam_deg_to_binang(away.x.atan2(away.z).to_degrees() + 180.0);
    w.place_player(at, yaw);
    frames(&mut w, 30);
    w.audio.log = Some(Default::default());
    let start_frame = w.audio.frames;

    // OnePointCutscene_Attention: an NPC's timer is 100; in front of the main camera (no
    // attention cutscene queued), sub camera 1 on CAM_SET_CS_ATTENTION at the main camera's at
    // and eye, data1 NA_SE_SY_CORRECT_CHIME; the main camera CAM_STAT_UNK3.
    let me = w.cam_actor(kid).unwrap();
    let main_at_eye = (w.game_camera.at, w.game_camera.eye);
    let sub = w.onepoint_attention(me);
    assert_eq!(sub, 1);
    let c = w.camera(1).unwrap();
    assert_eq!((c.cs_id, c.setting, c.timer, c.target, c.data1), (5010, CAM_SET_CS_ATTENTION, 100, Some(kid), NA_SE_SY_CORRECT_CHIME));
    assert_eq!((c.at, c.eye), main_at_eye);
    assert_eq!((c.parent_cam_id, c.child_cam_id, w.game_camera.child_cam_id), (CAM_ID_MAIN, CAM_ID_MAIN, 1));
    assert_eq!((w.active_cam_id, w.game_camera.status), (1, CAM_STAT_UNK3));
    assert!(w.onepoint_check_for_category(ACTORCAT_NPC));

    // The first frame: Camera_Demo5 picks D_8011D9F4 and lengthens the timer by its other two
    // keyframes' (the main camera's child): [1] is 4 if the bgcheck from Player's head to the
    // target's hits, else (eyeTargetDist * 0.005) + 8; [2] is 1. Then CAM_SET_CS_C and
    // Camera_Unique9's first keyframe (timer 100, the camera's).
    let player_head = w.player().actor.focus_pos;
    let kid_focus = w.actors.actor(kid).unwrap().focus_pos;
    let eye_target_dist = kid_focus.distance(main_at_eye.1);
    assert!(eye_target_dist > 700.0, "{eye_target_dist}");
    frames(&mut w, 1);
    let c = w.camera(1).unwrap().clone();
    assert_eq!(c.setting, CAM_SET_CS_C);
    let t = w.onepoint.table("D_8011D9F4");
    assert_eq!(c.cs_info.key_frames.map(|k| (k.table, k.start)), Some((t, 0)));
    assert_eq!(c.cs_info.key_frame_cnt, 3);
    let k1 = w.onepoint.keyframes[t][1].timer_init;
    let mut probe = kid_focus;
    let hit = GameCamera::bg_check(&w.col, player_head, &mut probe);
    assert_eq!(k1, if hit { 4 } else { (eye_target_dist * 0.005) as i16 + 8 });
    // The timer, counted down once by Unique9's first frame.
    assert_eq!(c.timer, 100 + k1 + 1 - 1);
    // Player held about the target (Player_SetCsActionWithHaltedActors(play, target, 1)), the chime once.
    // (The camera updates after the actors: Player takes the mode next frame.)
    assert_eq!(w.player().cs_mode, 1);
    let chimes: Vec<_> = w.audio.log.as_ref().unwrap().sfx.iter().filter(|s| s.1 == NA_SE_SY_CORRECT_CHIME).cloned().collect();
    assert_eq!(chimes, vec![(start_frame + 1, NA_SE_SY_CORRECT_CHIME, SfxPos::Default)]);
    // Keyframe 0 (action 0x8F: a static copy, the eye bgchecked; initFlags 0x0504): the at
    // around the target by its init (0, 5, 50) turned by Player's offset from the target, and
    // the eye from the at by (0, 20, 300) the same way (more than 400 from Player: no random
    // eye x, nor the near target's r).
    let player_pos = w.player().actor.world_pos;
    let offset = diff_to_sph_geo(kid_focus, Vec3::new(player_pos.x, player_head.y, player_pos.z));
    let turned = |v: Vec3| {
        let mut s: VecSphGeo = vec3_to_sph_geo(v);
        s.yaw = s.yaw.wrapping_add(offset.yaw);
        s.pitch = s.pitch.wrapping_add(offset.pitch);
        s
    };
    let at = sph_geo_add(kid_focus, turned(Vec3::new(0.0, 5.0, 50.0)));
    assert!(c.at.distance(at) < 0.01, "at {} want {at}", c.at);
    // (Here the eye's line from the at ends in the hillside behind the child: Camera_BGCheck
    // puts it on the hit, 1 off along the wall's normal.)
    let mut eye = sph_geo_add(at, turned(Vec3::new(0.0, 20.0, 300.0)));
    GameCamera::bg_check(&w.col, at, &mut eye);
    assert!(c.eye.distance(eye) < 0.01, "eye {} want {eye}", c.eye);
    assert_eq!(c.fov, 60.0);

    // Keyframe 0 for 100 frames, keyframe 1 for its timer, then keyframe 2 (action 0x12: the
    // camera copied to the main one, the timer to 0): Camera_Finish gives the main camera back
    // and ends Player's cutscene (mode 7).
    let mut n = 1;
    while w.active_cam_id == 1 {
        frames(&mut w, 1);
        n += 1;
        if n == 2 {
            assert_eq!(w.player().action, Action::Cutscene);
        }
        assert!(n < 400);
    }
    assert_eq!(n, 101 + k1 as i32);
    assert!(w.camera(1).is_none());
    assert_eq!(w.game_camera.status, CAM_STAT_ACTIVE);
    assert_eq!(w.player().cs_mode, 7);
}

#[test]
fn the_falling_chests_shot() {
    let Some(mut w) = village() else { return };
    // OnePointCutscene_SetInfo's 4500 about any actor (the falling chest's): CAM_SET_FREE2,
    // the at 40 above the floor under the actor's focus, the eye 150 from it at pitch 0x3E8
    // along the focus's yaw, fov 50, roll 0; Player held (Player_SetCsActionWithHaltedActors(play, NULL, 8)).
    let kid = w.actors.all().into_iter().find(|&h| w.actors.actor(h).is_some_and(|a| a.category == ACTORCAT_NPC)).expect("an NPC");
    let me = w.cam_actor(kid).unwrap();
    let sub = w.onepoint_cutscene_init(4500, 9999, Some(me), CAM_ID_MAIN);
    let c = w.camera(sub).unwrap().clone();
    let floor = w.col.entity_raycast_down(me.focus_pos).0;
    let at = Vec3::new(me.focus_pos.x, floor + 40.0, me.focus_pos.z);
    let eye = sph_geo_add(at, VecSphGeo { r: 150.0, pitch: 0x3E8, yaw: me.focus_rot[1] });
    assert_eq!((c.cs_id, c.setting, c.timer, c.fov, c.roll), (4500, CAM_SET_FREE2, 9999, 50.0, 0));
    assert_eq!(c.at, at);
    assert!(c.eye.distance(eye) < 1e-3, "{} {eye}", c.eye);
    assert_eq!(w.player().cs_mode, 8);
    frames(&mut w, 5);
    assert_eq!(w.active_cam_id, sub, "a fixed shot until the chest lands");
    // OnePointCutscene_EndCutscene (the chest landed): the timer to 0, finished at the end of
    // the frame.
    w.onepoint_end_cutscene(sub);
    frames(&mut w, 1);
    assert!(w.camera(sub).is_none());
    assert_eq!((w.active_cam_id, w.game_camera.status), (CAM_ID_MAIN, CAM_STAT_ACTIVE));
    assert_eq!(w.player().cs_mode, 7);
}

#[test]
fn an_attention_cutscene_on_link_himself() {
    let Some(mut w) = village() else { return };
    // OnePointCutscene_Attention on Player (ACTORCAT_PLAYER: timer 30). Camera_Demo5's Player
    // branch: the eye more than 30 from his head, so D_8011D6AC, its keyframe 1's timer the
    // camera's - 1, the timer lengthened by keyframe 2's (1); then, less than 3000 frames since
    // sDemo5PrevAction12Frame (-16), Player_SetCsActionWithHaltedActors(play, target, 69).
    let me = w.cam_actor(w.player.unwrap()).unwrap();
    let sub = w.onepoint_attention(me);
    assert_eq!(w.camera(sub).unwrap().timer, 30);
    frames(&mut w, 1);
    let c = w.camera(sub).unwrap();
    let t = w.onepoint.table("D_8011D6AC");
    assert_eq!((c.cs_info.key_frames.map(|k| k.table), c.cs_info.key_frame_cnt), (Some(t), 3));
    assert_eq!(w.onepoint.keyframes[t][1].timer_init, 29);
    assert_eq!(c.timer, 30 + 1 - 1);
    assert_eq!(w.player().cs_mode, 69);
    // Mode 69's start, D_80854B18[69] = { 3, &gPlayerAnim_link_hatto_demo }: func_80851094
    // (Player_AnimChangeOnceMorphAdjustedZeroRootYawSpeed): the animation once at 2/3, morphing over 8 frames. (Player takes the
    // mode in its next update's interrupts, Player_StartCsAction, and runs the mode's start in the
    // cutscene action, Player_Action_CsAction, the update after.)
    frames(&mut w, 1);
    assert_eq!(w.player().action, Action::Cutscene);
    frames(&mut w, 1);
    let p = w.player();
    assert_eq!((p.action, w.data.anim_name(p.skel.animation)), (Action::Cutscene, "link_hatto_demo"));
    assert_eq!((p.skel.play_speed, p.skel.morph_weight > 0.0), (2.0 / 3.0, true));
    // Its update, { 11, NULL }: the animation runs. The cutscene ends with Link's mode 7.
    let mut n = 3;
    while w.active_cam_id == sub {
        frames(&mut w, 1);
        n += 1;
        assert!(n < 200);
    }
    assert_eq!(w.player().cs_mode, 7);
}
