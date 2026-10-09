//! The blue warp (GAME-05 milestone 6b) against the C: `Door_Warp1` (`z_door_warp1.c`) as this
//! ROM's Deku Tree reaches it (the child warp Queen Gohma leaves; the destination warp at
//! Kokiri Forest's arrival), one-point 9703 (`z_onepointdemo.c`), and Player's blue-warp arrival
//! (`Player_StartMode_BlueWarp`, `Player_Action_BlueWarpArrive`); then the exit's run
//! (`Route::BlueWarp`).
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::door_warp1::{Action, DoorWarp1, WARP_DESTINATION, WARP_DUNGEON_CHILD};
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::audio::sfx::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::transition::{TRANS_TRIGGER_START, TRANS_TYPE_FADE_WHITE, TRANS_TYPE_FADE_WHITE_SLOW};

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// `entrance` with `preset`, the sound log on, three frames run (the room's objects in).
fn enter(a: &Arc<GameAssets>, entrance: &str, preset: &str) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset(preset).unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 3);
    w
}

fn warp_h(w: &PlayState) -> oot_game::actor_ctx::ActorHandle {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<DoorWarp1>(h).is_some_and(|d| !d.actor.killed && d.actor.params == WARP_DUNGEON_CHILD)).expect("the blue warp")
}

fn warp(w: &PlayState) -> &DoorWarp1 {
    w.actors.downcast::<DoorWarp1>(warp_h(w)).unwrap()
}

fn sfx_on(w: &PlayState, frame: u32, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == frame && s == id)
}

/// Link out of the warp's way, on the room's floor.
const LINK_AWAY: Vec3 = Vec3::new(400.0, -640.0, -300.0);

#[test]
fn the_child_warp_grows_in_then_waits() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_DEKU_TREE_BOSS_0", "deku-tree-gohma-cleared");
    w.place_player(LINK_AWAY, 0);
    // Queen Gohma's init in the cleared room: the warp at (0, -640, 0), WARP_DUNGEON_CHILD.
    let d = warp(&w);
    assert_eq!(d.actor.world_pos, Vec3::new(0.0, -640.0, 0.0));
    // DoorWarp1_Init: its two lights; DoorWarp1_SetupWarp: ring 0, widths -140 and -80, 0.3 up,
    // offset 1; the lights at it (200, 255, 255, radius 255).
    assert!(d.upper_light.is_some() && d.lower_light.is_some());
    let mut scales = Vec::new();
    for _ in 0..90 {
        idle(&mut w, 1);
        let d = warp(&w);
        scales.push((d.scale, d.unk_1ae, d.unk_1b0, d.action));
        if d.action == Action::ChildWarpIdle {
            break;
        }
        assert!(sfx_on(&w, w.audio.frames, NA_SE_EV_WARP_HOLE - SFX_FLAG));
    }
    // DoorWarp1_WarpAppear: the ring 2 a frame to 100 (its actor scale scale / 100), the widths 4 a
    // frame while under 120 and 230 (so 232: -80 + 4 x 78), then idle the update after.
    let (s, ae, b0, act) = *scales.last().unwrap();
    assert_eq!((s, ae, b0, act), (100, 120, 232, Action::ChildWarpIdle));
    let (s0, _, b00, _) = scales[0];
    assert!(s0 % 2 == 0 && (b00 + 80) % 4 == 0);
    assert_eq!(warp(&w).actor.scale, Vec3::ONE);
    assert_eq!((warp(&w).unk_194, warp(&w).unk_198, warp(&w).actor.shape_y_offset), (0.3, 0.3, 1.0));
}

#[test]
fn the_warp_takes_link_and_out_to_kokiri_forest_with_the_emerald() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_DEKU_TREE_BOSS_0", "deku-tree-gohma-cleared");
    w.place_player(LINK_AWAY, 0);
    while warp(&w).action != Action::ChildWarpIdle {
        idle(&mut w, 1);
    }
    // Link in it (within 60 across, 20 up or down).
    w.place_player(Vec3::new(20.0, -640.0, 10.0), 0);
    idle(&mut w, 1);
    let f = w.audio.frames;
    // DoorWarp1_ChildWarpIdle: NA_SE_EV_LINK_WARP, one-point 9703 (a sub camera on CAM_SET_CS_C,
    // its three keyframes), Link walking to its centre (PLAYER_CSACTION_10, unk_450).
    assert_eq!(warp(&w).action, Action::ChildWarpOut);
    assert!(sfx_on(&w, f, NA_SE_EV_LINK_WARP));
    assert_ne!(w.active_cam_id, oot_game::camera::CAM_ID_MAIN);
    let cam = w.camera(w.active_cam_id).unwrap();
    assert_eq!((cam.setting, cam.cs_info.key_frame_cnt), (oot_game::onepoint::CAM_SET_CS_C, 3));
    assert_eq!(w.player().cs_mode, 10);
    assert_eq!((w.player().unk_450.x, w.player().unk_450.z), (0.0, 0.0));
    // DoorWarp1_ChildWarpOut: from unk_1B2 101 Link floats (gravity 0.1 while rising slower than
    // 10); warpTimer past sWarpTimerTarget (100), with no next cutscene: the way out.
    let ph = w.player.unwrap();
    let mut floated = None;
    for k in 1..=200 {
        idle(&mut w, 1);
        if floated.is_none() && w.actors.actor(ph).is_some_and(|p| p.velocity.y > 0.0) {
            floated = Some(k);
        }
        if w.transition.trigger == TRANS_TRIGGER_START {
            // The Deku Tree's first time: EVENTCHKINF_07 and _09, the Kokiri Emerald
            // (QUEST_KOKIRI_EMERALD), ENTR_KOKIRI_FOREST_0 with cutscene 0xFFF1, the slow white fade
            // and white in.
            assert_eq!(warp(&w).warp_timer, 101);
            assert!(w.save.get_event_chk_inf(oot_game::save::EVENTCHKINF_07));
            assert!(w.save.get_event_chk_inf(oot_game::save::EVENTCHKINF_09));
            assert!(w.save.check_quest_item(oot_game::item::QUEST_KOKIRI_EMERALD));
            assert_eq!(Some(w.transition.next_entrance_index), w.entrance_by_name("ENTR_KOKIRI_FOREST_0"));
            assert_eq!(w.save.next_cutscene_index, 0xFFF1);
            assert_eq!((w.transition.ty, w.save.next_transition_type), (TRANS_TYPE_FADE_WHITE_SLOW, TRANS_TYPE_FADE_WHITE));
            // The float starts with it (unk_1B2 101 and warpTimer 101 come together): Link rises
            // through the fade.
            assert!(floated.is_none());
            idle(&mut w, 20);
            assert!(w.actors.actor(ph).is_some_and(|p| p.velocity.y > 0.0));
            return;
        }
    }
    panic!("the warp never took Link out");
}

#[test]
fn a_second_time_the_warp_goes_to_the_forest_without_a_cutscene() {
    let Some(a) = assets() else { return };
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_BOSS_0").unwrap();
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-gohma-cleared").unwrap();
    save.set_event_chk_inf(oot_game::save::EVENTCHKINF_07);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), false)).unwrap();
    idle(&mut w, 3);
    w.place_player(LINK_AWAY, 0);
    while warp(&w).action != Action::ChildWarpIdle {
        idle(&mut w, 1);
    }
    w.place_player(Vec3::new(0.0, -640.0, 0.0), 0);
    for _ in 0..200 {
        idle(&mut w, 1);
        if w.transition.trigger == TRANS_TRIGGER_START {
            break;
        }
    }
    // EVENTCHKINF_07 set: ENTR_KOKIRI_FOREST_11, CS_INDEX_NONE, no emerald given.
    assert_eq!(Some(w.transition.next_entrance_index), w.entrance_by_name("ENTR_KOKIRI_FOREST_11"));
    assert_eq!(w.save.next_cutscene_index, 0xFFEF);
    assert!(!w.save.check_quest_item(oot_game::item::QUEST_KOKIRI_EMERALD));
}

#[test]
fn link_arrives_by_blue_warp_from_800_up_into_the_emerald_cutscene() {
    let Some(a) = assets() else { return };
    // As the warp leaves it: ENTR_KOKIRI_FOREST_0 with cutscene 0xFFF1 (layer 5).
    let e = a.scenes.entrance_index("ENTR_KOKIRI_FOREST_0").unwrap();
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-gohma-cleared").unwrap();
    save.cutscene_index = 0xFFF1;
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true)).unwrap();
    assert_eq!(w.save.scene_layer, 5);
    // Player_StartMode_BlueWarp (params 0x02FF): 800 above the spawn, held (PLAYER_STATE1_29),
    // the warp's arrival pose.
    let p = w.player();
    assert_eq!(p.action, oot_actors::player::Action::BlueWarpArrive);
    assert!(p.state1 & oot_actors::player::STATE1_29 != 0);
    // The layer's destination warp (Door_Warp1 6) is killed at its init: Link isn't within 100.
    assert!(w.actors.all().into_iter().all(|h| w.actors.downcast::<DoorWarp1>(h).is_none_or(|d| d.actor.params != WARP_DESTINATION || d.actor.killed)));
    // The script holds him over its cue's start (2857, -594) as he falls; landed (the landing's
    // sound), the pose to its end, then the cutscene's mode.
    let (mut landed, mut over_cue) = (false, false);
    for _ in 0..400 {
        // (Cutscene_UpdateScripted runs after the actors: a cue set this frame moves him next.)
        let cue_before = w.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE && w.cs_ctx.link_action.is_some();
        idle(&mut w, 1);
        let p = w.player();
        if p.action == oot_actors::player::Action::BlueWarpArrive && cue_before {
            over_cue = true;
            assert_eq!((p.actor.world_pos.x, p.actor.world_pos.z), (2857.0, -594.0));
        }
        if p.action != oot_actors::player::Action::BlueWarpArrive {
            landed = true;
            break;
        }
    }
    assert!(landed && over_cue);
    assert_eq!(w.player().action, oot_actors::player::Action::Cutscene);
    assert!(w.player().actor.bg_check_flags & oot_game::actor::BGCHECKFLAG_GROUND != 0);
}

#[test]
fn onepoint_9703_starts_from_the_view_and_closes_in() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_DEKU_TREE_BOSS_0", "deku-tree-gohma-cleared");
    w.place_player(LINK_AWAY, 0);
    while warp(&w).action != Action::ChildWarpIdle {
        idle(&mut w, 1);
    }
    let view = w.view;
    w.place_player(Vec3::new(0.0, -640.0, 0.0), 0);
    idle(&mut w, 1);
    // D_80123894[0]: the at, eye and fov from play->view; [1] and [2] as the table has them for a
    // child (no LINK_IS_ADULT change: 28 and 20 up).
    let t = w.onepoint.table("D_80123894");
    let k0 = *w.onepoint.kf(t, 0);
    assert!(k0.at_target_init.distance(view.at) < 0.01 && k0.eye_target_init.distance(view.eye) < 0.01);
    let k1 = *w.onepoint.kf(t, 1);
    assert_eq!((k1.at_target_init.y, k1.eye_target_init.y, k1.timer_init), (28.0, 20.0, 30));
}

#[test]
fn the_warps_bake_has_its_two_rings_on_their_matrices() {
    let Ok(pack) = oot_game::pack::GamePack::open_default() else { return };
    let d: eng_gfx::DrawList = pack.assets.get(&oot_game::pack::keys::bake("Door_Warp1/portal")).expect("the portal");
    assert!(d.stats.unresolved_addresses.is_empty());
    // gWarpPortalDL: 13 vertices under segment 0x0A's matrix (bone 0), 13 under segment 9's
    // (bone 1), 24 triangles joining them.
    assert_eq!(d.triangle_count(), 24);
    let bones: std::collections::BTreeSet<u16> = d.batches.iter().flat_map(|b| b.vertices.iter().map(|v| v.bone)).collect();
    assert_eq!(bones, [0u16, 1].into_iter().collect());
}

#[test]
fn exit_into_the_blue_warp_and_out_to_the_deku_trees_emerald() {
    let Some(a) = assets() else { return };
    let route = Route::BlueWarp;
    let e = a.scenes.entrance_index(route.entrance()).expect("entrance");
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), route.save(e)).expect("Play_Init");
    route.debug_start(&mut w);
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
    }
    assert!(run.failure.is_none(), "{:?}", run.failure);
    eprintln!("steps {:?}; texts {:x?}", run.steps, run.texts);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::WarpEntered, Step::WarpedOut, Step::Arrived, Step::EmeraldPart1Over]);
    // Kokiri Forest's layer 5 (gKokiriForestKokiriEmeraldPart1Cs): the Deku Tree's texts, 0x1092 a
    // choice; its terminator (1) on to the castle town's cutscene map (Ganondorf on his horse).
    assert_eq!(&run.texts[..3], &[0x1024, 0x1091, 0x1092]);
    assert!(w.save.check_quest_item(oot_game::item::QUEST_KOKIRI_EMERALD));
    assert_eq!(w.transition.trigger, TRANS_TRIGGER_START);
}
