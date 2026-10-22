//! GAME-10 milestone 6 against the C: `Door_Warp1`'s Ruto warp (`WARP_BLUE_RUTO`,
//! `DoorWarp1_RutoWarpIdle` to `DoorWarp1_RutoWarpOut`, `z_door_warp1.c:558-641`) with Barinade's
//! room's Ruto (`En_Ru1` type 0), one-point 9705, and the fountain's cutscene it leads to
//! (`gZorasFountainSapphireCs`, spot08 layer 4) played through.
//!
//! Child Link enters Barinade's room (`ENTR_JABU_JABU_BOSS_0`) on `barinade-beaten`: the cleared
//! room's `Boss_Va` init spawns Ruto (type 0) at his position and the heart beside her. Expected
//! values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::door_warp1::{self as warp, Action, DoorWarp1};
use oot_actors::en_ru1::{self as ru1, EnRu1};
use oot_game::actor_ctx::ActorHandle;
use oot_game::camera::{CAM_ID_MAIN, CAM_STAT_WAIT};
use oot_game::cutscene::CS_STATE_IDLE;
use oot_game::message::TEXT_STATE_NONE;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::transition::{TRANS_TRIGGER_START, TRANS_TYPE_FADE_WHITE_SLOW};

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// A frame with A pressed on even `f` (released on odd): texts read.
fn tick_a(w: &mut PlayState, f: usize) {
    let a = with(PadState::default(), BTN_A);
    let (prev, cur) = if f % 2 == 0 { (PadState::default(), a) } else { (a, PadState::default()) };
    w.tick_with(scripted_input(prev, cur));
}

/// Child Link at `entrance` on `preset`, `cs_index` as the save's cutscene index, the sound log on.
fn play_at(a: &Arc<GameAssets>, entrance: &str, preset: &str, cs_index: Option<u16>) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset(preset).unwrap();
    if let Some(c) = cs_index {
        save.cutscene_index = c;
    }
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init")
}

/// `Boss_Va`'s position in room 1 (`bdan_boss_room_1`): where his cleared room spawns Ruto.
const BARINADE: Vec3 = Vec3::new(10.0, 0.0, -220.0);

/// Barinade's room after his defeat, Link landed: Ruto (type 0, at `BARINADE`) as his cleared
/// init spawned her.
fn beaten_room(a: &Arc<GameAssets>) -> (PlayState, ActorHandle) {
    let mut w = play_at(a, "ENTR_JABU_JABU_BOSS_0", "barinade-beaten", None);
    // (Actor_Spawn from his init: in the arena from the next update.)
    idle(&mut w, 2);
    let h = w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnRu1>(h).is_some_and(|r| !r.actor.killed)).expect("Ruto, from Barinade's cleared init");
    let r = w.actors.downcast::<EnRu1>(h).unwrap();
    assert_eq!((r.actor.world_pos, r.actor.params), (BARINADE, 0));
    (w, h)
}

fn ruto_warp(w: &PlayState) -> Option<(ActorHandle, &DoorWarp1)> {
    w.actors.all().into_iter().find_map(|h| w.actors.downcast::<DoorWarp1>(h).filter(|d| !d.actor.killed && d.actor.params == warp::WARP_BLUE_RUTO).map(|d| (h, d)))
}

/// Barinade's room with Link landed and Ruto (type 0) spawned at Barinade's place, run until she
/// has risen and her warp is ready (`WARP_BLUE_RUTO_STATE_READY`, idle).
fn ruto_risen(a: &Arc<GameAssets>) -> (PlayState, ActorHandle) {
    let (mut w, h) = beaten_room(a);
    assert_eq!(w.room_ctx.cur.num, 1);
    for _ in 0..360 {
        idle(&mut w, 1);
        if ruto_warp(&w).is_some_and(|(_, d)| d.ruto_warp_state == warp::WARP_BLUE_RUTO_STATE_READY && d.action == Action::RutoWarpIdle) {
            return (w, h);
        }
    }
    panic!("Ruto never rose: {:?}", ruto_warp(&w).map(|(_, d)| (d.action, d.ruto_warp_state)));
}

#[test]
fn rutos_warp_grows_in_and_waits_for_her_then_for_link() {
    let Some(a) = assets() else { return };
    let (mut w, _) = beaten_room(&a);
    let mut seen = Vec::new();
    for _ in 0..90 {
        idle(&mut w, 1);
        if let Some((_, d)) = ruto_warp(&w) {
            seen.push((d.action, d.ruto_warp_state));
        }
    }
    // DoorWarp1_WarpAppear as the child's (the ring to 100, the widths to 120 and 232), then
    // DoorWarp1_RutoWarpIdle (no longer logged as not ported).
    let (_, d) = ruto_warp(&w).expect("her warp");
    assert_eq!((d.scale, d.unk_1ae, d.unk_1b0, d.action), (100, 120, 232, Action::RutoWarpIdle));
    // Link far from it (his spawn, 560 away): Ruto rising, state still INITIAL at first; nothing
    // happens until he's in it.
    assert!(seen.iter().any(|&(a, s)| a == Action::RutoWarpIdle && s == warp::WARP_BLUE_RUTO_STATE_INITIAL));
    let (_, d) = ruto_warp(&w).unwrap();
    assert_ne!(d.ruto_warp_state, warp::WARP_BLUE_RUTO_STATE_ENTERED);
}

#[test]
fn link_in_rutos_warp_hears_her_and_warps_out_to_the_fountain_with_the_sapphire() {
    let Some(a) = assets() else { return };
    let (mut w, rh) = ruto_risen(&a);
    let (wh, d) = ruto_warp(&w).unwrap();
    let at = d.actor.world_pos;
    // DoorWarp1_PlayerInRange: within 60 across, 20 up or down.
    w.place_player(at + Vec3::new(20.0, 0.0, 10.0), 0);
    idle(&mut w, 1);
    let d = w.actors.downcast::<DoorWarp1>(wh).unwrap();
    // DoorWarp1_RutoWarpIdle: ENTERED, PLAYER_CSACTION_10, unk_1B2 1, func_80999EE0.
    assert_eq!((d.action, d.ruto_warp_state, d.unk_1b2), (Action::Func80999EE0, warp::WARP_BLUE_RUTO_STATE_ENTERED, 1));
    assert_eq!(w.player().cs_mode, 10);
    // Ruto marks Link's spot and, once he's on it facing her, folds her arms (state 3): the warp's
    // sub camera (at the warp at y 49, eye at Link at y 43, fov 90), TALKING, text 0x4022.
    let mut talking = false;
    for _ in 0..200 {
        idle(&mut w, 1);
        if w.actors.downcast::<DoorWarp1>(wh).unwrap().ruto_warp_state == warp::WARP_BLUE_RUTO_STATE_TALKING {
            talking = true;
            break;
        }
    }
    assert!(talking, "Ruto never folded her arms: {:?}", w.actors.downcast::<EnRu1>(rh).map(|r| r.action));
    assert_eq!(w.actors.downcast::<DoorWarp1>(wh).unwrap().action, Action::Func80999FE4);
    assert_ne!(w.active_cam_id, CAM_ID_MAIN);
    let sub = w.active_cam_id;
    let cam = w.camera(sub).unwrap();
    assert_eq!((cam.at.x, cam.at.y, cam.at.z, cam.eye.y), (at.x, 49.0, at.z, 43.0));
    assert_eq!(cam.fov, 90.0);
    assert_eq!(w.camera(CAM_ID_MAIN).unwrap().status, CAM_STAT_WAIT);
    assert_eq!(w.msg_ctx.text_id, 0x4022);
    // The text read: func_80999FE4: NA_SE_EV_LINK_WARP, one-point 9705 (a sub camera on
    // CAM_SET_CS_C with D_8012395C's three keyframes), the Ruto sub camera copied into it and set
    // waiting; WARPING; Ruto rises with Link.
    let mut f = 0;
    while w.message_state() != TEXT_STATE_NONE && f < 400 {
        tick_a(&mut w, f);
        f += 1;
    }
    assert_eq!(w.message_state(), TEXT_STATE_NONE);
    idle(&mut w, 1);
    let d = w.actors.downcast::<DoorWarp1>(wh).unwrap();
    assert_eq!((d.action, d.ruto_warp_state), (Action::RutoWarpOut, warp::WARP_BLUE_RUTO_STATE_WARPING));
    let cam = w.camera(w.active_cam_id).unwrap();
    assert_eq!((cam.setting, cam.cs_info.key_frame_cnt), (oot_game::onepoint::CAM_SET_CS_C, 3));
    assert_eq!(w.camera(sub).unwrap().status, CAM_STAT_WAIT);
    idle(&mut w, 1);
    assert_eq!(w.actors.downcast::<EnRu1>(rh).unwrap().action, ru1::ENRU1_ACTION_WARPING_OUT);
    // DoorWarp1_RutoWarpOut: from unk_1B2 61 Link floats (gravity 0.02 while rising slower than
    // 10); past sWarpTimerTarget (100) with no next cutscene: EVENTCHKINF_37, the Sapphire,
    // ENTR_ZORAS_FOUNTAIN_0 with CS_INDEX_0 (0xFFF0), the slow white fade; the next transition
    // type isn't touched.
    let ph = w.player.unwrap();
    let next_type = w.save.next_transition_type;
    let mut floated = false;
    for _ in 0..200 {
        idle(&mut w, 1);
        if w.actors.actor(ph).is_some_and(|p| p.gravity == 0.02) {
            floated = true;
        }
        if w.transition.trigger == TRANS_TRIGGER_START {
            assert!(floated);
            assert_eq!(w.actors.downcast::<DoorWarp1>(wh).unwrap().warp_timer, 101);
            assert!(w.save.get_event_chk_inf(0x37));
            assert!(w.save.check_quest_item(oot_game::item::QUEST_ZORA_SAPPHIRE));
            assert_eq!(Some(w.transition.next_entrance_index), w.entrance_by_name("ENTR_ZORAS_FOUNTAIN_0"));
            assert_eq!(w.save.next_cutscene_index, 0xFFF0);
            assert_eq!((w.transition.ty, w.save.next_transition_type), (TRANS_TYPE_FADE_WHITE_SLOW, next_type));
            return;
        }
    }
    panic!("the warp never took Link out");
}

#[test]
fn onepoint_9705_starts_from_the_view_and_rises_behind_link() {
    let Some(a) = assets() else { return };
    let (mut w, _) = ruto_risen(&a);
    let (wh, d) = ruto_warp(&w).unwrap();
    let at = d.actor.world_pos;
    w.place_player(at, 0);
    for _ in 0..200 {
        idle(&mut w, 1);
        if w.actors.downcast::<DoorWarp1>(wh).unwrap().ruto_warp_state == warp::WARP_BLUE_RUTO_STATE_TALKING {
            break;
        }
    }
    let view = w.view;
    let mut f = 0;
    while w.message_state() != TEXT_STATE_NONE && f < 400 {
        tick_a(&mut w, f);
        f += 1;
    }
    idle(&mut w, 1);
    // D_8012395C[0]: the at, eye and fov from play->view; [1] 28 and 20 up, the eye 45 behind;
    // [2] 120 frames, fov 180 (as the table has it).
    let t = w.onepoint.table("D_8012395C");
    let k1 = *w.onepoint.kf(t, 1);
    assert_eq!((k1.at_target_init.y, k1.eye_target_init.y, k1.eye_target_init.z, k1.timer_init), (28.0, 20.0, -45.0, 30));
    let k2 = *w.onepoint.kf(t, 2);
    assert_eq!((k2.timer_init, k2.fov_target_init), (120, 180.0));
    let k0 = *w.onepoint.kf(t, 0);
    assert!(k0.fov_target_init > 0.0 && (k0.fov_target_init - view.fov).abs() < 30.0);
}

#[test]
fn the_fountains_sapphire_cutscene_plays_through_to_its_terminator() {
    let Some(a) = assets() else { return };
    // As Ruto's warp leaves it: ENTR_ZORAS_FOUNTAIN_0 with CS_INDEX_0 (layer 4), the Sapphire
    // given and EVENTCHKINF_37 set.
    let mut w = {
        let e = a.scenes.entrance_index("ENTR_ZORAS_FOUNTAIN_0").unwrap();
        let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
        save.apply_preset("jabu-barinade").unwrap();
        save.set_event_chk_inf(0x37);
        oot_game::item::item_give(&mut save, None, oot_game::item::ITEM_ZORA_SAPPHIRE);
        save.cutscene_index = 0xFFF0;
        let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
        PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init")
    };
    assert_eq!(w.save.scene_layer, 4);
    let mut texts: Vec<u16> = Vec::new();
    let mut ruto_cues = std::collections::BTreeSet::new();
    let mut f = 0usize;
    let mut last_frame = 0;
    while f < 4000 {
        if w.message_state() != TEXT_STATE_NONE {
            if texts.last() != Some(&w.msg_ctx.text_id) {
                texts.push(w.msg_ctx.text_id);
            }
            tick_a(&mut w, f);
        } else {
            idle(&mut w, 1);
        }
        f += 1;
        if w.cs_ctx.state != CS_STATE_IDLE {
            last_frame = w.cs_ctx.frames;
            if let Some(c) = w.cs_ctx.npc_actions.get(3).copied().flatten() {
                ruto_cues.insert(c.action);
            }
        }
        if w.transition.trigger == TRANS_TRIGGER_START {
            break;
        }
    }
    eprintln!("texts {texts:x?}; Ruto's cues {ruto_cues:?}; last cutscene frame {last_frame}; {f} frames");
    // Its terminator (13, from frame 1110) back to ENTR_ZORAS_FOUNTAIN_0.
    assert_eq!(w.transition.trigger, TRANS_TRIGGER_START, "the cutscene never ended (frame {last_frame})");
    assert!(last_frame >= 1110);
    assert_eq!(Some(w.transition.next_entrance_index), w.entrance_by_name("ENTR_ZORAS_FOUNTAIN_0"));
    // Ruto (type 1) followed her cues (channel 3) to the dive, back, and away: on cue 8, her swim
    // away done, she's gone.
    assert!(ruto_cues.contains(&3) && ruto_cues.contains(&8), "{ruto_cues:?}");
    assert!(w.actors.all().into_iter().all(|h| w.actors.downcast::<EnRu1>(h).is_none_or(|r| r.actor.killed)));
    // With the Ruby held, the Sapphire's text is 0x4035 (CS_TEXT_ZORA_SAPPHIRE), after 0x0082.
    assert_eq!(texts, vec![0x4025, 0x4026, 0x4027, 0x0082, 0x4035, 0x4028]);
}
