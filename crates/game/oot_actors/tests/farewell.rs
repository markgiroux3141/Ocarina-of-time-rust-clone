//! GAME-06 milestone 2 against the C: `Demo_Sa` on the bridge (`z_demo_sa.c`), the soft soil
//! (`z_obj_bean.c`), the owl (`z_en_owl.c`) and his one-point cutscene 8700
//! (`z_onepointdemo.c`), `En_Ko` child 3's place with the emerald, `Play_Init`'s Hyrule Field and
//! Kokiri Forest layers, Link's ocarina in a cutscene (`func_80851D2C`); then the exit's run
//! (`Route::Farewell`): out of Kokiri Forest past Mido, Saria's goodbye and the Fairy Ocarina,
//! Hyrule Field's intro, the owl's talk and his flight.
//!
//! The cues are put on the cutscene's channels by hand (`play->csCtx.actorCues`), and each actor
//! updated alone, as `Actor_UpdateAll` would. Expected values are worked out from the C in the
//! comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::{IVec3, Vec3};
use oot_actors::PlayExt;
use oot_actors::demo_sa::*;
use oot_actors::en_owl::*;
use oot_actors::obj_bean::*;
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::actor_ctx::ActorHandle;
use oot_game::cutscene::CsCmdActorCue;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// `CS_STATE_RUN` (`cutscene.h`).
const CS_STATE_RUN: u8 = 2;
/// `object_table.h`.
const OBJECT_SA: i16 = 0x00BC;
const OBJECT_MAMENOKI: i16 = 0x011E;
const OBJECT_OWL: i16 = 0x0131;

/// A play state entered at `entrance`, three frames run, these objects loaded.
fn world(a: &Arc<GameAssets>, entrance: &str, adult: bool, prep: impl FnOnce(&mut SaveContext), objects: &[i16]) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, adult, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-dead").unwrap();
    prep(&mut save);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 3);
    for &o in objects {
        if w.object_ctx.get_index(o).is_none() {
            w.object_ctx.spawn(o);
        }
    }
    w
}

fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

fn cue(w: &mut PlayState, channel: usize, action: u16, start: IVec3, rot_y: i16) {
    w.cs_ctx.state = CS_STATE_RUN;
    w.cs_ctx.npc_actions[channel] = Some(CsCmdActorCue { action, start_frame: 0, end_frame: 100, rot: [0, rot_y, 0], start_pos: start, end_pos: start, normal: IVec3::ZERO });
}

fn saria(w: &PlayState, h: ActorHandle) -> &DemoSa {
    w.actors.downcast::<DemoSa>(h).expect("Demo_Sa")
}

#[test]
fn saria_on_the_bridge_follows_her_cues() {
    let Some(a) = assets() else { return };
    let mut w = world(&a, "ENTR_LINKS_HOUSE_0", false, |_| {}, &[OBJECT_SA]);
    let before = w.actors.all().len();
    let h = w.actor_spawn(ACTOR_DEMO_SA, Vec3::new(0.0, 0.0, 0.0), [0; 3], 5).unwrap();
    // DemoSa_InitBridge: hidden, sad, her fairy (En_Elf FAIRY_KOKIRI) as her child.
    let s = saria(&w, h);
    assert_eq!((s.action, s.draw_config, s.shadow_alpha), (DEMOSA_ACTION_BRIDGE_INVISIBLE, DEMOSA_DRAW_NOTHING, 0));
    assert_eq!((s.eye_index, s.mouth_index), (SARIA_EYE_SAD, SARIA_MOUTH_CLOSED));
    assert!(s.actor.child.is_some());
    assert_eq!(w.actors.all().len(), before + 2);
    // Cue 4: still hidden. Cue 12 after it: placed and turned by the cue, faded in over
    // kREG(17) + 10 = 10 frames (alpha 25, 51, ... 229, then 255 and the opaque draw).
    cue(&mut w, 1, 4, IVec3::new(-1156, -220, 1683), -20025);
    update(&mut w, h);
    assert_eq!((saria(&w, h).cue_id, saria(&w, h).action), (4, DEMOSA_ACTION_BRIDGE_INVISIBLE));
    cue(&mut w, 1, 12, IVec3::new(-1156, -220, 1683), -20025);
    update(&mut w, h);
    let s = saria(&w, h);
    assert_eq!((s.action, s.draw_config), (DEMOSA_ACTION_BRIDGE_FADE_IN, DEMOSA_DRAW_XLU));
    assert_eq!((s.actor.world_pos, s.actor.shape_rot.y), (Vec3::new(-1156.0, -220.0, 1683.0), -20025));
    let mut alphas = Vec::new();
    for _ in 0..10 {
        update(&mut w, h);
        alphas.push(saria(&w, h).alpha);
    }
    assert_eq!(alphas, vec![25, 51, 76, 102, 127, 153, 178, 204, 229, 255]);
    let s = saria(&w, h);
    assert_eq!((s.action, s.draw_config, s.shadow_alpha), (DEMOSA_ACTION_BRIDGE_LOOKING_SAD, DEMOSA_DRAW_OPA, 255));
    // Cue 13: she clutches the ocarina, eyes shut.
    cue(&mut w, 1, 13, IVec3::ZERO, 0);
    update(&mut w, h);
    let s = saria(&w, h);
    assert_eq!((s.action, s.is_holding_ocarina, s.eye_index), (DEMOSA_ACTION_BRIDGE_CLUTCH_OCARINA, true, SARIA_EYE_CLOSED));
    assert!(s.skel.is("gSariaHoldOcarinaAnim"));
    // Cue 14: she gives it, once, then holds it out, looping.
    cue(&mut w, 1, 14, IVec3::ZERO, 0);
    update(&mut w, h);
    assert!(saria(&w, h).skel.is("gSariaGiveLinkOcarinaAnim"));
    assert_eq!(saria(&w, h).action, DEMOSA_ACTION_BRIDGE_GIVE_OCARINA);
    let mut n = 0;
    while !saria(&w, h).skel.is("gSariaHoldOutOcarinaAnim") && n < 200 {
        update(&mut w, h);
        n += 1;
    }
    assert!(saria(&w, h).skel.is("gSariaHoldOutOcarinaAnim"), "never held it out");
    // Cue 12 not after 4: sad at once, opaque, the wait animation.
    cue(&mut w, 1, 12, IVec3::ZERO, 0);
    update(&mut w, h);
    let s = saria(&w, h);
    assert_eq!((s.action, s.draw_config, s.is_holding_ocarina), (DEMOSA_ACTION_BRIDGE_LOOKING_SAD, DEMOSA_DRAW_OPA, false));
    assert!(s.skel.is("gSariaWaitOnBridgeAnim"));
}

#[test]
fn the_soft_soil_offers_its_talk_for_a_bean() {
    let Some(a) = assets() else { return };
    // A child with no bean planted: the patch (DRAW_LEAVES), text 0x2F.
    let mut w = world(&a, "ENTR_LINKS_HOUSE_0", false, |_| {}, &[OBJECT_MAMENOKI]);
    let at = w.player().actor.world_pos + Vec3::new(20.0, 0.0, 0.0);
    let h = w.actor_spawn(ACTOR_OBJ_BEAN, at, [0; 3], 0x1F04).unwrap();
    let b = w.actors.downcast::<ObjBean>(h).unwrap();
    assert_eq!((b.action, b.state_flags, b.actor.text_id, b.actor.scale.x), (oot_actors::obj_bean::Action::WaitForBean, BEAN_STATE_DRAW_LEAVES, 0x2F, 0.1));
    // Within 40 it offers (Player's targetActor, which his update clears at its end) for the magic
    // bean (EXCH_ITEM_MAGIC_BEAN).
    idle(&mut w, 2);
    update(&mut w, h);
    assert_eq!(w.player().target_actor, Some(h));
    assert_eq!(w.player().exchange_item_id, EXCH_ITEM_MAGIC_BEAN);
    // An adult without the flag: gone. A child with it (a bean planted): not ported, kept.
    let mut w = world(&a, "ENTR_LINKS_HOUSE_0", true, |_| {}, &[OBJECT_MAMENOKI]);
    let h = w.actor_spawn(ACTOR_OBJ_BEAN, at, [0; 3], 0x1F04).unwrap();
    assert!(w.actors.downcast::<ObjBean>(h).unwrap().actor.killed);
    let mut w = world(&a, "ENTR_LINKS_HOUSE_0", false, |_| {}, &[OBJECT_MAMENOKI]);
    w.flags.set_switch(4);
    let h = w.actor_spawn(ACTOR_OBJ_BEAN, at, [0; 3], 0x1F04).unwrap();
    let b = w.actors.downcast::<ObjBean>(h).unwrap();
    assert_eq!((b.action, b.state_flags, b.actor.killed), (oot_actors::obj_bean::Action::NotPorted, 0, false));
}

#[test]
fn the_owls_leave_by_their_flags_and_the_story() {
    let Some(a) = assets() else { return };
    let mut w = world(&a, "ENTR_LINKS_HOUSE_0", false, |_| {}, &[OBJECT_OWL]);
    let owl = |w: &mut PlayState, params: i16| {
        let h = w.actor_spawn(ACTOR_EN_OWL, Vec3::ZERO, [0; 3], params).unwrap();
        let o = w.actors.downcast::<EnOwl>(h).unwrap();
        (o.actor.killed, o.action, o.action_flags, o.unk_3ee)
    };
    // 0xFFF: outside Kokiri Forest, flag 0x20 (never set).
    assert_eq!(owl(&mut w, 0xFFF), (false, oot_actors::en_owl::Action::WaitOutsideKokiri, 0, 0));
    // Hyrule Field's (type 1, flag 0x0C): gone once its flag is set.
    assert!(!owl(&mut w, 0x004C).0);
    w.flags.set_switch(0x0C);
    assert!(owl(&mut w, 0x004C).0);
    // Hyrule Castle's starts turned round (actionFlags 2, unk_3EE 0x20).
    assert_eq!(owl(&mut w, 2 << 6 | 0x0D), (false, oot_actors::en_owl::Action::WaitHyruleCastle, 2, 0x20));
    // Kakariko's leaves with Zelda's letter; the Lost Woods' need Zelda's Lullaby and Saria's Song.
    assert!(!owl(&mut w, 3 << 6 | 0x0E).0);
    w.save.set_event_chk_inf(0x40);
    assert!(owl(&mut w, 3 << 6 | 0x0E).0);
    assert!(owl(&mut w, 11 << 6 | 0x10).0);
    assert!(owl(&mut w, 12 << 6 | 0x11).0);
}

#[test]
fn the_fields_owl_talks_turns_for_his_question_and_flies_off() {
    let Some(a) = assets() else { return };
    // Hyrule Field's intro seen (EVENTCHKINF_A0): Link below the owl's perch.
    let mut w = world(&a, "ENTR_HYRULE_FIELD_3", false, |s| s.set_event_chk_inf(0xA0), &[]);
    // The audio side, which finishes the field's audio spec change (else Audio_Update waits).
    w.audio_side = Some(Box::new(oot_game::audio::offline::OfflineAudio::new(&a.pack.audio_data().unwrap(), false)));
    let h = w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnOwl>(h).is_some_and(|o| o.actor.params == 0x004C)).expect("the field's owl");
    w.place_player(Vec3::new(4368.0, -160.0, 8217.0), -0x8000);
    let mut prev = PadState::default();
    let mut repeated = false;
    let mut seen = Vec::new();
    let mut cam_8700 = false;
    let mut flown = false;
    for _ in 0..2000 {
        let st = w.message_state();
        if st != 0 && seen.last() != Some(&w.msg_ctx.text_id) {
            seen.push(w.msg_ctx.text_id);
        }
        cam_8700 |= w.active_cam_id != oot_game::camera::CAM_ID_MAIN && w.message_state() != 0;
        let mut p = PadState::default();
        if st == oot_game::message::TEXT_STATE_CHOICE {
            // The first time, again (choice 0); then OK (choice 1).
            let want = if repeated { 1 } else { 0 };
            if w.msg_ctx.choice_index != want {
                if prev.stick_y == 0 {
                    p.stick_y = -60;
                }
            } else if prev.button & BTN_A == 0 {
                p.button = BTN_A;
                repeated = true;
            }
        } else if st != 0 && prev.button & BTN_A == 0 {
            p.button = BTN_A;
        }
        w.tick_with(scripted_input(prev, p));
        prev = p;
        if w.actors.downcast::<EnOwl>(h).is_none_or(|o| o.actor.killed) {
            flown = true;
            break;
        }
    }
    // EnOwl_WaitOutsideKokiri: 0x2064 (its chain 0x2065, 0x2066 the question); again: 0x2065;
    // OK: 0x2067. EVENTCHKINF_6F; the owl's fanfare (NA_BGM_OWL on SEQ_PLAYER_FANFARE); one-point
    // 8700 while he talks.
    assert_eq!(seen, vec![0x2064, 0x2065, 0x2066, 0x2065, 0x2066, 0x2067]);
    assert!(w.save.get_event_chk_inf(0x6F));
    let fanfare = w.audio.log.as_ref().unwrap().seq_cmds.iter().any(|&(_, c)| (c >> 24) & 0xF == 1 && c & 0xFF == 0x5A);
    assert!(fanfare, "no NA_BGM_OWL");
    assert!(cam_8700);
    // His flag (0x0C) at the talk's end; flown off, gone 6000 away; Link free, the main camera.
    assert!(w.flags.get_switch(0x0C));
    assert!(flown, "the owl never left");
    assert_eq!(w.active_cam_id, oot_game::camera::CAM_ID_MAIN);
    assert_eq!(w.player().cs_mode, 0);
}

#[test]
fn the_kokiri_by_the_lost_woods_stands_aside_with_the_emerald() {
    let Some(a) = assets() else { return };
    let ko3 = |w: &PlayState| w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<oot_actors::en_ko::EnKo>(h)).find(|k| k.actor.params & 0xFF == 3).map(|k| k.actor.world_pos);
    // With the emerald: Path_CopyLastPoint of path 0 (params 0x0003: ENKO_PATH 0).
    let w = world(&a, "ENTR_KOKIRI_FOREST_11", false, |_| {}, &[]);
    let path = w.setup_path_list()[0].clone();
    let (p, end) = (ko3(&w).expect("child 3"), path.point(path.count() - 1));
    // (On the floor since: his y falls to it.)
    assert_eq!((p.x, p.z), (end.x, end.z));
    // Without it: guarding by his home (-1472, -80, -294), 80 towards Link.
    let w = world(&a, "ENTR_KOKIRI_FOREST_0", false, |s| *s = SaveContext::new(s.entrance_index, false, s.day_time), &[]);
    let p = ko3(&w).expect("child 3");
    assert!((Vec3::new(p.x, 0.0, p.z) - Vec3::new(-1472.0, 0.0, -294.0)).length() <= 80.5, "{p}");
}

#[test]
fn play_init_picks_hyrule_fields_and_kokiri_forests_layers() {
    let Some(a) = assets() else { return };
    // A child in Hyrule Field: layer 0 without the three stones, 1 with them (whatever the time).
    let w = world(&a, "ENTR_HYRULE_FIELD_3", false, |s| s.set_event_chk_inf(0xA0), &[]);
    assert_eq!(w.save.scene_layer, 0);
    let w = world(
        &a,
        "ENTR_HYRULE_FIELD_3",
        false,
        |s| {
            s.set_event_chk_inf(0xA0);
            for q in [oot_game::item::QUEST_GORON_RUBY, oot_game::item::QUEST_ZORA_SAPPHIRE] {
                s.inventory.quest_items |= 1 << q;
            }
        },
        &[],
    );
    assert_eq!(w.save.scene_layer, 1);
    // An adult in Kokiri Forest: 2, or 3 with EVENTCHKINF_48.
    let w = world(&a, "ENTR_KOKIRI_FOREST_0", true, |_| {}, &[]);
    assert_eq!(w.save.scene_layer, 2);
    let w = world(&a, "ENTR_KOKIRI_FOREST_0", true, |s| s.set_event_chk_inf(0x48), &[]);
    assert_eq!(w.save.scene_layer, 3);
}

#[test]
fn exit_out_of_the_forest_saria_the_ocarina_and_the_owl() {
    let Some(a) = assets() else { return };
    let route = Route::Farewell;
    let e = a.scenes.entrance_index(route.entrance()).expect("entrance");
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), route.save(e), audio).expect("Play_Init");
    route.debug_start(&mut w);
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    let ocarina_group = w.data.items.model_group("OCARINA");
    let mut ocarina_in_hand = false;
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
        ocarina_in_hand |= w.player().models_group == Some(ocarina_group);
    }
    assert!(run.failure.is_none(), "{:?}", run.failure);
    eprintln!("steps {:?}; texts {:x?}", run.steps, run.texts);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(
        steps,
        vec![Step::Mido, Step::ForestLeft, Step::Bridge, Step::FairyOcarina, Step::Field, Step::FieldIntroOver, Step::OwlTalk, Step::OwlFlown]
    );
    // Mido's with the emerald (0x1045); the bridge's TEXT_LIST (0x1011, 0x1094, 0x1012, 0x1013,
    // the ocarina's 0x004A, 0x1014); the owl's, answered OK at once.
    assert_eq!(run.texts, vec![0x1045, 0x1011, 0x1094, 0x1012, 0x1013, 0x004A, 0x1014, 0x2064, 0x2065, 0x2066, 0x2067]);
    let s = &w.save;
    // Mido gone (EVENTCHKINF_1C), the bridge seen (C1) with the Fairy Ocarina owned, the field's
    // intro seen (A0), the owl's talk (6F) and flag (0x0C).
    for f in [0x1C, 0xC1, 0xA0, 0x6F] {
        assert!(s.get_event_chk_inf(f), "EVENTCHKINF_{f:02X}");
    }
    assert_eq!(s.inv_content(oot_game::item::ITEM_OCARINA_FAIRY), oot_game::item::ITEM_OCARINA_FAIRY);
    assert!(w.flags.get_switch(0x0C));
    // Link took the ocarina out in the bridge's cutscene (func_80851D2C's Player_SetModels).
    assert!(ocarina_in_hand);
    assert_eq!(Some(s.entrance_index), a.scenes.entrance_index("ENTR_HYRULE_FIELD_3"));
}
