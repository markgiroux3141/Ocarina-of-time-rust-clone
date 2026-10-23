//! `En_Xc` (GAME-11 milestone 2) against the C (`z_en_xc.c`): the Temple of Time's Sheik. Type 9's
//! init (`EnXc_InitTempleOfTime`) for a child, for an adult without `EVENTCHKINF_C5` (the
//! awakening's script started), with `EVENTCHKINF_48` and no `_55` (the Prelude given, its script
//! started), and after (the pedestal block); type 1's actions on their cues; the pedestal block's
//! text by the hookshot and its talk; the other types as placeholders after their checks.
//!
//! The Temple of Time is entered at `ENTR_TEMPLE_OF_TIME_4` (room 0, the pedestal's step), where
//! room 0 places `En_Xc` 9. Sheik's update runs alone (`Actor_UpdateAll`'s call), her cues put on
//! the script's channel 4 by hand (`play->csCtx.actorCues`). Expected values are worked out from
//! the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use glam::{IVec3, Vec3};
use oot_actors::en_xc::*;
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_TALK};
use oot_game::actor_ctx::ActorHandle;
use oot_game::cutscene::{CS_STATE_IDLE, CsCmdActorCue};
use oot_game::item::{ITEM_HOOKSHOT, QUEST_SONG_MINUET};
use oot_game::play::PlayState;
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// `EVENTCHKINF_48` (`save.h`).
const EVENTCHKINF_48: u16 = 0x48;
/// `QUEST_SONG_PRELUDE` (`z64save.h`: 0x0B, five after the Minuet).
const QUEST_SONG_PRELUDE: u32 = QUEST_SONG_MINUET + 5;
/// `CS_STATE_RUN` (`cutscene.h`).
const CS_STATE_RUN: u8 = 2;
/// Room 0's Sheik (`ootx scene-info --scene tokinoma --layer 2`).
const SHEIK_AT: Vec3 = Vec3::new(0.0, 71.0, 11.0);

/// The Temple of Time at the pedestal's step (`ENTR_TEMPLE_OF_TIME_4`, room 0) from
/// `ocarina-of-time`, as an adult or a child, `tweak` last; just after `Play_Init` (room 0's
/// actors initialised).
fn temple(a: &Arc<GameAssets>, adult: bool, tweak: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_TEMPLE_OF_TIME_4").expect("entrance");
    let mut save = SaveContext::new(e, adult, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("ocarina-of-time").unwrap();
    save.adult = adult;
    tweak(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    settle(&mut w, ACTOR_EN_XC);
    assert_eq!(w.room_ctx.cur.num, 0);
    w
}

/// Every `En_Xc` (ported) in the arena, killed or not.
fn sheiks(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnXc>(h).is_some()).collect()
}

fn xc(w: &PlayState, h: ActorHandle) -> &EnXc {
    w.actors.downcast::<EnXc>(h).expect("an En_Xc")
}

fn xc_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnXc {
    w.actors.downcast_mut::<EnXc>(h).expect("an En_Xc")
}

/// The actor's update alone (`Actor_UpdateAll`'s call), as the updating actor.
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// Sheik's cue `action` on channel 4, from `start` (turned `yaw`).
fn cue(w: &mut PlayState, action: u16, start: IVec3, yaw: i16) {
    w.cs_ctx.state = CS_STATE_RUN;
    w.cs_ctx.npc_actions[4] = Some(CsCmdActorCue { action, start_frame: 0, end_frame: 100, rot: [0, yaw, 0], start_pos: start, end_pos: start, normal: IVec3::ZERO });
}

fn script_name(w: &PlayState) -> Option<String> {
    w.cs_ctx.segment.as_ref().map(|s| s.name.clone())
}


/// Ticks until the room's actors of `id` are in and initialised (`Actor_UpdateAll` runs an init
/// once its object is loaded, without the update that frame); stops on that tick.
fn settle(w: &mut PlayState, id: i16) {
    for _ in 0..60 {
        let mine: Vec<&dyn oot_game::actor_ctx::ActorImpl> = w.actors.all().into_iter().filter_map(|h| w.actors.get(h)).filter(|a| a.base().id == id).collect();
        if !mine.is_empty() && mine.iter().all(|a| a.name() != "(not initialised)") {
            return;
        }
        w.tick_with(oot_game::play::scripted_input(eng_input::pad::PadState::default(), eng_input::pad::PadState::default()));
    }
    panic!("actor {id:#x} not initialised");
}

#[test]
fn type_9_is_gone_for_a_child() {
    let Some(a) = assets() else { return };
    let w = temple(&a, false, |_| {});
    // EnXc_InitTempleOfTime: !LINK_IS_ADULT: Actor_Kill.
    let s = sheiks(&w);
    assert!(s.iter().all(|&h| xc(&w, h).actor.killed), "no live Sheik for a child");
    assert!(!w.save.get_event_chk_inf(EVENTCHKINF_C5));
}

#[test]
fn type_9_first_as_an_adult_starts_the_awakening() {
    let Some(a) = assets() else { return };
    let w = temple(&a, true, |_| {});
    // No EVENTCHKINF_C5: set; gTempleOfTimeFirstAdultCs; cutsceneTrigger 1; func_80B3EBF0
    // (SHEIK_ACTION_20), drawn as nothing.
    let s: Vec<ActorHandle> = sheiks(&w).into_iter().filter(|&h| !xc(&w, h).actor.killed).collect();
    assert_eq!(s.len(), 1);
    let h = s[0];
    assert_eq!(xc(&w, h).actor.world_pos, SHEIK_AT);
    assert!(w.save.get_event_chk_inf(EVENTCHKINF_C5));
    assert_eq!(script_name(&w).as_deref(), Some("gTempleOfTimeFirstAdultCs"));
    // (Cutscene_HandleConditionalTriggers took the trigger that frame: the script runs.)
    assert!(w.save.cutscene_trigger == 1 || w.cs_ctx.state != CS_STATE_IDLE);
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_20);
    assert_eq!(xc(&w, h).draw_mode, SHEIK_DRAW_NOTHING);
    // Not the Prelude.
    assert!(!w.save.get_event_chk_inf(EVENTCHKINF_55));
    assert!(!w.save.check_quest_item(QUEST_SONG_PRELUDE));
}

#[test]
fn type_9_with_the_forest_medallion_gives_the_prelude() {
    let Some(a) = assets() else { return };
    let w = temple(&a, true, |s| {
        s.set_event_chk_inf(EVENTCHKINF_C5);
        s.set_event_chk_inf(EVENTCHKINF_48);
    });
    // EVENTCHKINF_C5, EVENTCHKINF_48, no _55: _55 set, Item_Give(ITEM_SONG_PRELUDE),
    // gTempleOfTimePreludeCs, cutsceneTrigger 1, SHEIK_ACTION_30 (type 2's, milestone 3).
    let h = *sheiks(&w).iter().find(|&&h| !xc(&w, h).actor.killed).expect("Sheik");
    assert!(w.save.get_event_chk_inf(EVENTCHKINF_55));
    assert!(w.save.check_quest_item(QUEST_SONG_PRELUDE));
    assert_eq!(script_name(&w).as_deref(), Some("gTempleOfTimePreludeCs"));
    // (Cutscene_HandleConditionalTriggers took the trigger that frame: the script runs.)
    assert!(w.save.cutscene_trigger == 1 || w.cs_ctx.state != CS_STATE_IDLE);
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_30);
}

#[test]
fn type_9_after_the_prelude_is_gone() {
    let Some(a) = assets() else { return };
    let w = temple(&a, true, |s| {
        s.set_event_chk_inf(EVENTCHKINF_C5);
        s.set_event_chk_inf(EVENTCHKINF_48);
        s.set_event_chk_inf(EVENTCHKINF_55);
    });
    assert!(sheiks(&w).iter().all(|&h| xc(&w, h).actor.killed));
    assert!(script_name(&w).is_none());
}

#[test]
fn the_pedestal_block_and_its_texts() {
    let Some(a) = assets() else { return };
    let mut w = temple(&a, true, |s| s.set_event_chk_inf(EVENTCHKINF_C5));
    // EVENTCHKINF_C5, no _48: func_80B3C9EC: SHEIK_ACTION_BLOCK_PEDESTAL, SHEIK_DRAW_DEFAULT,
    // unk_30C 1; no script.
    let h = *sheiks(&w).iter().find(|&&h| !xc(&w, h).actor.killed).expect("Sheik");
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_BLOCK_PEDESTAL);
    assert_eq!(xc(&w, h).draw_mode, SHEIK_DRAW_DEFAULT);
    assert_eq!(xc(&w, h).unk_30c, 1);
    assert!(script_name(&w).is_none());
    assert!(xc(&w, h).skel.is("gSheikArmsCrossedIdleAnim"));
    // EnXc_SetupDialogueAction: no hookshot: 0x700F, attention and friendly on.
    assert_eq!(w.save.inv_content(ITEM_HOOKSHOT), oot_game::item::ITEM_NONE);
    update(&mut w, h);
    assert_eq!(xc(&w, h).actor.text_id, 0x700F);
    let f = xc(&w, h).actor.flags;
    assert_eq!(f & (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY), ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY);
    // With the hookshot: 0x7010.
    oot_game::item::item_give(&mut w.save, None, ITEM_HOOKSHOT);
    update(&mut w, h);
    assert_eq!(xc(&w, h).actor.text_id, 0x7010);
    // The talk taken (Actor_TalkOfferAccepted): SHEIK_ACTION_IN_DIALOGUE.
    xc_mut(&mut w, h).actor.flags |= ACTOR_FLAG_TALK;
    update(&mut w, h);
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_IN_DIALOGUE);
    assert_eq!(xc(&w, h).actor.flags & ACTOR_FLAG_TALK, 0);
    // No box closing: still talking.
    update(&mut w, h);
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_IN_DIALOGUE);
}

#[test]
fn type_1_follows_its_cues_to_the_pedestal() {
    let Some(a) = assets() else { return };
    // The placed type 9 gone (the Prelude learned), a type 1 spawned in room 0.
    let mut w = temple(&a, true, |s| {
        s.set_event_chk_inf(EVENTCHKINF_C5);
        s.set_event_chk_inf(EVENTCHKINF_55);
    });
    let h = w.actor_spawn(ACTOR_EN_XC, Vec3::new(-60.0, 71.0, 6.0), [0, -910, 0], 1).expect("En_Xc 1");
    // func_80B3EBF0: SHEIK_ACTION_20; EnXc_ActionFunc20: _21.
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_20);
    update(&mut w, h);
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_21);
    // func_80B3EC0C: no script, then cue 1: waiting, hidden.
    update(&mut w, h);
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_21);
    let start = IVec3::new(-60, 71, 6);
    cue(&mut w, 1, start, -910);
    update(&mut w, h);
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_21);
    assert_eq!(xc(&w, h).draw_mode, SHEIK_DRAW_NOTHING);
    // Cue 6 (not 1): at its start, turned its way, drawn, SHEIK_ACTION_22.
    let at = IVec3::new(-40, 71, 20);
    cue(&mut w, 6, at, 0x1234);
    update(&mut w, h);
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_22);
    assert_eq!(xc(&w, h).draw_mode, SHEIK_DRAW_DEFAULT);
    assert_eq!(xc(&w, h).actor.world_pos, Vec3::new(-40.0, 71.0, 20.0));
    assert_eq!((xc(&w, h).actor.world_rot.y, xc(&w, h).actor.shape_rot.y), (0x1234, 0x1234));
    // EnXc_ActionFunc22 under cue 6: idling (func_80B3EC90 waits for another cue).
    let frame = xc(&w, h).skel.cur_frame;
    update(&mut w, h);
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_22);
    assert!(xc(&w, h).skel.cur_frame != frame, "her idle plays");
    // Cue 7: func_80B3C9EC: blocking the pedestal, arms crossed, the head turns on.
    cue(&mut w, 7, at, 0x1234);
    update(&mut w, h);
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_BLOCK_PEDESTAL);
    assert_eq!(xc(&w, h).unk_30c, 1);
    assert!(xc(&w, h).skel.is("gSheikArmsCrossedIdleAnim"));
    // The script over: the block goes on (offering her text).
    w.cs_ctx.state = CS_STATE_IDLE;
    update(&mut w, h);
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_BLOCK_PEDESTAL);
    assert_eq!(xc(&w, h).actor.text_id, 0x700F);
}

#[test]
fn the_other_types_are_placeholders_after_their_checks() {
    let Some(a) = assets() else { return };
    let mut w = temple(&a, true, |s| {
        s.set_event_chk_inf(EVENTCHKINF_C5);
        s.set_event_chk_inf(EVENTCHKINF_55);
    });
    let at = Vec3::new(0.0, 71.0, 60.0);
    let live = |w: &PlayState, h: ActorHandle| !w.actors.actor(h).unwrap().killed;
    for ty in [0, 2, 3, 4, 5] {
        let h = w.actor_spawn(ACTOR_EN_XC, at, [0; 3], ty).expect("spawned");
        assert!(w.actors.downcast::<EnXc>(h).is_none(), "type {ty} is a placeholder");
        assert!(live(&w, h));
    }
    // func_80B3CA38 (the Minuet): an adult without EVENTCHKINF_50 stays; with it, gone.
    let h = w.actor_spawn(ACTOR_EN_XC, at, [0; 3], SHEIK_TYPE_MINUET).unwrap();
    assert!(live(&w, h) && w.actors.downcast::<EnXc>(h).is_none());
    w.save.set_event_chk_inf(EVENTCHKINF_50);
    let h = w.actor_spawn(ACTOR_EN_XC, at, [0; 3], SHEIK_TYPE_MINUET).unwrap();
    assert!(!live(&w, h));
    // func_80B3CB58 (the Bolero): the same with EVENTCHKINF_51.
    let h = w.actor_spawn(ACTOR_EN_XC, at, [0; 3], SHEIK_TYPE_BOLERO).unwrap();
    assert!(live(&w, h));
    w.save.set_event_chk_inf(EVENTCHKINF_51);
    let h = w.actor_spawn(ACTOR_EN_XC, at, [0; 3], SHEIK_TYPE_BOLERO).unwrap();
    assert!(!live(&w, h));
    // EnXc_SetupSerenadeAction: gone with the Iron Boots (DEBUG_FEATURES) or EVENTCHKINF_52.
    let h = w.actor_spawn(ACTOR_EN_XC, at, [0; 3], SHEIK_TYPE_SERENADE).unwrap();
    assert!(live(&w, h));
    w.save.set_event_chk_inf(EVENTCHKINF_52);
    let h = w.actor_spawn(ACTOR_EN_XC, at, [0; 3], SHEIK_TYPE_SERENADE).unwrap();
    assert!(!live(&w, h));
    // A child: the songs' Sheiks are gone.
    w.save.adult = false;
    let h = w.actor_spawn(ACTOR_EN_XC, at, [0; 3], SHEIK_TYPE_SERENADE).unwrap();
    assert!(!live(&w, h));
}
