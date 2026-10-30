//! `En_Xc` (GAME-11 milestones 2 and 3) against the C (`z_en_xc.c`): the Temple of Time's Sheik.
//! Type 9's init (`EnXc_InitTempleOfTime`) for a child, for an adult without `EVENTCHKINF_C5` (the
//! awakening's script started), with `EVENTCHKINF_48` and no `_55` (the Prelude given, its script
//! started), and after (the pedestal block); type 1's actions on their cues; the pedestal block's
//! text by the hookshot and its talk; type 2's (the Prelude's) actions 29 to 44 on their cues,
//! with the harp's draws; type 6's (the Minuet's) trigger, fall and walk (GAME-13 milestone 2); the
//! other types as placeholders after their checks.
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
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE};

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

/// Sheik's cue `action` on channel 4 at `start`, turned `rot`.
fn cue_rot(w: &mut PlayState, action: u16, start: IVec3, rot: [i16; 3]) {
    w.cs_ctx.state = CS_STATE_RUN;
    w.cs_ctx.npc_actions[4] = Some(CsCmdActorCue { action, start_frame: 0, end_frame: 100, rot, start_pos: start, end_pos: start, normal: IVec3::ZERO });
}

/// The Temple of Time with the placed Sheik gone (the Prelude learned) and a type 2 spawned
/// where room 0's layer 6 places hers, the sound log on.
fn prelude(a: &Arc<GameAssets>) -> (PlayState, ActorHandle) {
    let mut w = temple(a, true, |s| {
        s.set_event_chk_inf(EVENTCHKINF_C5);
        s.set_event_chk_inf(EVENTCHKINF_55);
    });
    w.audio.log = Some(oot_game::audio::AudioLog::default());
    let h = w.actor_spawn(ACTOR_EN_XC, Vec3::new(-60.0, 71.0, 6.0), [0, -910, 0], SHEIK_TYPE_2).expect("En_Xc 2");
    (w, h)
}

/// Updates while the action stays `action` (at most 400); how many until it moved.
fn run_while(w: &mut PlayState, h: ActorHandle, action: i32) -> u32 {
    for n in 1..=400 {
        update(w, h);
        if xc(w, h).action != action {
            return n;
        }
    }
    panic!("still in action {action}");
}

fn sfx(w: &PlayState, id: u16) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).count()
}

/// The mesh Sheik's draw puts down (one opaque skeleton).
fn drawn_mesh(w: &PlayState, h: ActorHandle) -> Option<eng_gfx::MeshKey> {
    let x = xc(w, h);
    let view = oot_game::play::ViewInfo { eye: Vec3::ZERO, billboard: glam::Mat4::IDENTITY };
    let mut out = oot_game::play::DrawOut::default();
    oot_game::actor_ctx::ActorImpl::draw(x, &oot_game::actor_ctx::ActorImpl::render_state(x), w, &view, &mut out);
    assert!(out.opa.len() <= 1);
    out.opa.first().map(|c| c.mesh.clone())
}

fn bake(name: &str) -> eng_gfx::MeshKey {
    eng_gfx::MeshKey::named(oot_game::pack::keys::bake(name))
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
    // gTempleOfTimePreludeCs, cutsceneTrigger 1, SHEIK_ACTION_30 (type 2's).
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
    for ty in [0, 3, 4, 5] {
        let h = w.actor_spawn(ACTOR_EN_XC, at, [0; 3], ty).expect("spawned");
        assert!(w.actors.downcast::<EnXc>(h).is_none(), "type {ty} is a placeholder");
        assert!(live(&w, h));
    }
    // func_80B3CA38 (the Minuet, ported): an adult without EVENTCHKINF_50 stays, hidden; with
    // it, gone.
    let h = w.actor_spawn(ACTOR_EN_XC, at, [0; 3], SHEIK_TYPE_MINUET).unwrap();
    assert!(live(&w, h));
    assert_eq!((xc(&w, h).action, xc(&w, h).draw_mode), (SHEIK_ACTION_INIT, SHEIK_DRAW_NOTHING));
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

#[test]
fn type_2_goes_through_the_beta_serenade_to_its_first_cue() {
    let Some(a) = assets() else { return };
    let (mut w, h) = prelude(&a);
    // func_80B3EE64: SHEIK_ACTION_SERENADE, drawn as nothing.
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_SERENADE);
    assert_eq!(xc(&w, h).draw_mode, SHEIK_DRAW_NOTHING);
    // EnXc_Serenade (func_80B3EE74): EnXc_SerenadeCS is true for a type other than 8: 30.
    update(&mut w, h);
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_30);
    // EnXc_ActionFunc30: no script, then cue 1: hidden, waiting.
    update(&mut w, h);
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_30);
    cue(&mut w, 1, IVec3::new(-60, 71, 6), -911);
    update(&mut w, h);
    assert_eq!((xc(&w, h).action, xc(&w, h).draw_mode), (SHEIK_ACTION_30, SHEIK_DRAW_NOTHING));
    // Cue 6 (the script's first): func_80B3EC0C puts her at its start, turned, drawn (action 22),
    // and func_80B3EEA4 steps on from 30: 31.
    cue(&mut w, 6, IVec3::new(-60, 71, 6), -911);
    update(&mut w, h);
    assert_eq!((xc(&w, h).action, xc(&w, h).draw_mode), (SHEIK_ACTION_31, SHEIK_DRAW_DEFAULT));
    assert_eq!(xc(&w, h).actor.world_pos, Vec3::new(-60.0, 71.0, 6.0));
    assert_eq!((xc(&w, h).actor.world_rot.y, xc(&w, h).actor.shape_rot.y), (-911, -911));
    assert_eq!(drawn_mesh(&w, h), Some(bake(BAKE)));
}

#[test]
fn the_prelude_pulls_out_the_harp_and_plays() {
    let Some(a) = assets() else { return };
    assert!(a.pack.bake(HARP_BAKE).is_ok(), "the harp's bake (re-import the pack)");
    let (mut w, h) = prelude(&a);
    xc_mut(&mut w, h).action = SHEIK_ACTION_31;
    xc_mut(&mut w, h).draw_mode = SHEIK_DRAW_DEFAULT;
    // EnXc_ActionFunc31 under cue 6: idling; func_80B3C588 holds her at the cue's start, every
    // axis of its rotation on both rotations.
    let at = IVec3::new(-60, 71, 6);
    cue_rot(&mut w, 6, at, [0x100, -2731, 0x200]);
    update(&mut w, h);
    let x = xc(&w, h);
    assert_eq!(x.action, SHEIK_ACTION_31);
    assert_eq!(x.actor.world_pos, Vec3::new(-60.0, 71.0, 6.0));
    assert_eq!((x.actor.world_rot.x, x.actor.world_rot.y, x.actor.world_rot.z), (0x100, -2731, 0x200));
    assert_eq!((x.actor.shape_rot.x, x.actor.shape_rot.y, x.actor.shape_rot.z), (0x100, -2731, 0x200));
    // Cue 23 (func_80B3DAF0: 3, 11, 12, 13 or 23): gSheikPullingOutHarpAnim once from 0 (morph
    // -4: weight 1, rate 1/4), SHEIK_DRAW_PULLING_OUT_HARP, action 7, so 32.
    cue_rot(&mut w, 23, at, [0, -2731, 0]);
    update(&mut w, h);
    let x = xc(&w, h);
    assert_eq!((x.action, x.draw_mode), (SHEIK_ACTION_32, SHEIK_DRAW_PULLING_OUT_HARP));
    assert!(x.skel.is("gSheikPullingOutHarpAnim"));
    assert_eq!((x.skel.mode, x.skel.cur_frame, x.skel.play_speed, x.skel.morph_weight, x.skel.morph_rate), (ANIMMODE_ONCE, 0.0, 1.0, 1.0, 0.25));
    // func_80B3E87C: the hand without the harp before frame 34 (the plain skeleton)...
    assert!(!x.harp_drawn());
    assert_eq!(drawn_mesh(&w, h), Some(bake(BAKE)));
    let end = x.skel.end_frame;
    assert!(end >= 34.0, "the harp shows while she pulls it out ({end})");
    // ...and with it (limb 12's gSheikHarpDL) from frame 34: one frame a frame.
    for _ in 0..33 {
        update(&mut w, h);
    }
    assert!(!xc(&w, h).harp_drawn());
    update(&mut w, h);
    assert_eq!(xc(&w, h).skel.cur_frame, 34.0);
    assert!(xc(&w, h).harp_drawn());
    assert_eq!(drawn_mesh(&w, h), Some(bake(HARP_BAKE)));
    // EnXc_ActionFunc32: SkelAnime_Update says it's done the update after it reached its end
    // frame (34 updates so far, end + 1 in all): EnXc_SetupInitialHarpAction:
    // gSheikInitialHarpAnim once, SHEIK_DRAW_HARP (action 8), so 33.
    let n = run_while(&mut w, h, SHEIK_ACTION_32);
    assert_eq!(34 + n, end as u32 + 1);
    let x = xc(&w, h);
    assert_eq!((x.action, x.draw_mode), (SHEIK_ACTION_33, SHEIK_DRAW_HARP));
    assert!(x.skel.is("gSheikInitialHarpAnim"));
    assert_eq!((x.skel.mode, x.skel.cur_frame), (ANIMMODE_ONCE, 0.0));
    // EnXc_DrawHarp: limb 12 always the harp.
    assert!(x.harp_drawn());
    assert_eq!(drawn_mesh(&w, h), Some(bake(HARP_BAKE)));
    // EnXc_ActionFunc33: that done (end + 1), EnXc_SetupPlayingHarpAction: cue 23's tune
    // (EnXc_GetCurrentHarpAnim: gSheikPlayingHarpAnim) looped, morph -8; SHEIK_PLAYING_HARP, 34.
    let end = xc(&w, h).skel.end_frame;
    assert_eq!(run_while(&mut w, h, SHEIK_ACTION_33), end as u32 + 1);
    let x = xc(&w, h);
    assert_eq!((x.action, x.draw_mode), (SHEIK_ACTION_34, SHEIK_DRAW_HARP));
    assert!(x.skel.is("gSheikPlayingHarpAnim"));
    assert_eq!((x.skel.mode, x.skel.morph_rate), (ANIMMODE_LOOP, 1.0 / 8.0));
    // Playing on under cue 23 (func_80B3DCA8 waits for cue 8).
    update(&mut w, h);
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_34);
}

#[test]
fn the_harp_held_played_again_and_put_away() {
    let Some(a) = assets() else { return };
    let (mut w, h) = prelude(&a);
    xc_mut(&mut w, h).action = SHEIK_ACTION_34;
    xc_mut(&mut w, h).draw_mode = SHEIK_DRAW_HARP;
    let at = IVec3::new(-60, 71, 6);
    // Cue 8 (func_80B3DCA8): gSheikInitialHarpAnim held on its last frame (speed 0, looped,
    // morph -8), action 10, so 35.
    cue(&mut w, 8, at, -2731);
    update(&mut w, h);
    let x = xc(&w, h);
    assert_eq!(x.action, SHEIK_ACTION_35);
    assert!(x.skel.is("gSheikInitialHarpAnim"));
    let last = x.skel.anim_length - 1.0;
    assert_eq!((x.skel.play_speed, x.skel.cur_frame, x.skel.mode), (0.0, last, ANIMMODE_LOOP));
    // Held under cue 8 (EnXc_CheckForNoCue(8) false).
    update(&mut w, h);
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_35);
    // Cue 12 (neither 5 nor 8): EnXc_SetupPlayingHarpAction(true): gSheikPlayingHarp2Anim,
    // action 9; func_80B3EF58 (func_80B3C7D4): not put away, back to 34.
    cue(&mut w, 12, at, -2731);
    update(&mut w, h);
    let x = xc(&w, h);
    assert_eq!((x.action, x.draw_mode), (SHEIK_ACTION_34, SHEIK_DRAW_HARP));
    assert!(x.skel.is("gSheikPlayingHarp2Anim"));
    // Cue 13 while playing: nothing (34 only waits for 8); then 8 holds it up again.
    cue(&mut w, 13, at, -2731);
    update(&mut w, h);
    assert!(xc(&w, h).skel.is("gSheikPlayingHarp2Anim"));
    cue(&mut w, 8, at, -2731);
    update(&mut w, h);
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_35);
    // Cue 5 with the hold at its end frame (curFrame >= endFrame): gSheikInitialHarpAnim played
    // back from its last frame, once (action SHEIK_ACTION_PUT_HARP_AWAY), so 36.
    cue(&mut w, 5, at, -2731);
    update(&mut w, h);
    let x = xc(&w, h);
    assert_eq!((x.action, x.draw_mode), (SHEIK_ACTION_36, SHEIK_DRAW_HARP));
    assert!(x.skel.is("gSheikInitialHarpAnim"));
    assert_eq!((x.skel.play_speed, x.skel.cur_frame, x.skel.end_frame, x.skel.mode), (-1.0, last, 0.0, ANIMMODE_ONCE));
    // EnXc_ActionFunc36: back to 0 (last + 1 updates), func_80B3DE00:
    // gSheikPullingOutHarpAnim backwards, SHEIK_DRAW_PULLING_OUT_HARP (action 12), so 37.
    assert_eq!(run_while(&mut w, h, SHEIK_ACTION_36), last as u32 + 1);
    let x = xc(&w, h);
    assert_eq!((x.action, x.draw_mode), (SHEIK_ACTION_37, SHEIK_DRAW_PULLING_OUT_HARP));
    assert!(x.skel.is("gSheikPullingOutHarpAnim"));
    assert_eq!((x.skel.play_speed, x.skel.end_frame), (-1.0, 0.0));
    // The harp in her hand down to frame 34, then gone.
    assert!(x.harp_drawn());
    let start = x.skel.cur_frame;
    let mut drawn = Vec::new();
    let n = loop {
        update(&mut w, h);
        let x = xc(&w, h);
        if x.action != SHEIK_ACTION_37 {
            break drawn.len() + 1;
        }
        drawn.push((x.skel.cur_frame, x.harp_drawn()));
    };
    assert!(drawn.iter().all(|&(f, d)| d == (f >= 34.0)));
    assert!(drawn.iter().any(|&(_, d)| !d));
    // EnXc_ActionFunc37: at 0, done (start + 1 updates): func_80B3DE78: gSheikIdleAnim looped,
    // SHEIK_DRAW_DEFAULT, timer 0 (action 13), so 38.
    assert_eq!(n as f32, start + 1.0);
    let x = xc(&w, h);
    assert_eq!((x.action, x.draw_mode, x.timer), (SHEIK_ACTION_38, SHEIK_DRAW_DEFAULT, 0.0));
    assert!(x.skel.is("gSheikIdleAnim"));
    assert!(!x.harp_drawn());
    assert_eq!(drawn_mesh(&w, h), Some(bake(BAKE)));
}

#[test]
fn she_backs_away_and_throws_a_nut() {
    let Some(a) = assets() else { return };
    let (mut w, h) = prelude(&a);
    let start = Vec3::new(-60.0, 71.0, 6.0);
    {
        let x = xc_mut(&mut w, h);
        x.action = SHEIK_ACTION_38;
        x.draw_mode = SHEIK_DRAW_DEFAULT;
        x.actor.world_pos = start;
        x.actor.world_rot.y = 0x1000;
        x.actor.shape_rot.y = 0x1000;
        x.actor.xz_dist_to_player = 50.0;
    }
    let at = IVec3::new(-60, 71, 6);
    // Cue 4 (EnXc_SetupReverseAccel): gSheikWalkingAnim backwards from its last frame (looped,
    // morph -12), her walk turned round (0x1000 + 0x8000) but not her shape, timer 0: 14, so 39.
    cue(&mut w, 4, at, -2731);
    update(&mut w, h);
    let x = xc(&w, h);
    assert_eq!(x.action, SHEIK_ACTION_39);
    assert!(x.skel.is("gSheikWalkingAnim"));
    assert_eq!((x.skel.play_speed, x.skel.cur_frame), (-1.0, x.skel.anim_length - 1.0));
    assert_eq!((x.actor.world_rot.y, x.actor.shape_rot.y), (0x9000u16 as i16, 0x1000));
    assert_eq!(x.timer, 0.0);
    // EnXc_ReverseAccelerate: EnXc_CalcXZAccel reads the timer before it counts: still on
    // updates 1 to 9 (timer 0..8 < 9), 1.2 on 10 and 11 (`timer < 3` never true: @bug (game));
    // on the 12th EnXc_SetupReverseWalkAction's 12 sets 1.2 (action 15), so 40.
    for _ in 0..9 {
        update(&mut w, h);
    }
    assert_eq!((xc(&w, h).actor.speed_xz, xc(&w, h).timer), (0.0, 9.0));
    let before = xc(&w, h).actor.world_pos;
    update(&mut w, h);
    assert_eq!(xc(&w, h).actor.speed_xz, 1.2);
    update(&mut w, h);
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_39);
    update(&mut w, h);
    let x = xc(&w, h);
    assert_eq!((x.action, x.actor.speed_xz), (SHEIK_ACTION_40, 1.2));
    // Moved on 10 to 12 at 1.2 (Actor_UpdatePos: times R_UPDATE_RATE * 0.5, 1.5: 1.8 a frame),
    // along 0x9000 (away, backwards).
    let d = x.actor.world_pos - before;
    let dir = Vec3::new(eng_math::sin_s(0x9000u16 as i16), 0.0, eng_math::cos_s(0x9000u16 as i16));
    assert!((Vec3::new(d.x, 0.0, d.z) - dir * (3.0 * 1.2 * 1.5)).length() < 0.01, "{d:?}");
    // EnXc_ActionFunc40 (func_80B3F010): walking until 140 from Link, then gSheikIdleAnim looped
    // (morph -12), timer 0: 41.
    xc_mut(&mut w, h).actor.xz_dist_to_player = 139.0;
    update(&mut w, h);
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_40);
    xc_mut(&mut w, h).actor.xz_dist_to_player = 140.0;
    update(&mut w, h);
    let x = xc(&w, h);
    assert_eq!((x.action, x.timer), (SHEIK_ACTION_41, 0.0));
    assert!(x.skel.is("gSheikIdleAnim"));
    assert_eq!(x.skel.morph_rate, 1.0 / 12.0);
    // EnXc_HaltAndWaitToThrowNut: EnXc_CalcXZSpeed's 1.2 / 3 * (3 - timer): 1.2, 0.8, 0.4, then 0.
    let mut speeds = Vec::new();
    for _ in 0..4 {
        update(&mut w, h);
        speeds.push(xc(&w, h).actor.speed_xz);
    }
    let want = [1.2f32, 0.8, 0.4, 0.0];
    assert!(speeds.iter().zip(want).all(|(s, w)| (s - w).abs() < 1e-5), "{speeds:?}");
    // EnXc_SetupNutThrow: on the 12th, gSheikThrowingNutAnim once, timer 0, stopped (17): 42.
    for _ in 0..7 {
        update(&mut w, h);
    }
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_41);
    update(&mut w, h);
    let x = xc(&w, h);
    assert_eq!((x.action, x.timer, x.actor.speed_xz), (SHEIK_ACTION_42, 0.0, 0.0));
    assert!(x.skel.is("gSheikThrowingNutAnim"));
    // EnXc_ThrowNut: EnXc_SetNutThrowSFX's NA_SE_VO_SK_SHOUT on frame 20 (the 20th update, one
    // frame a frame).
    for _ in 0..19 {
        update(&mut w, h);
    }
    assert_eq!(sfx(&w, NA_SE_VO_SK_SHOUT), 0);
    update(&mut w, h);
    assert_eq!(sfx(&w, NA_SE_VO_SK_SHOUT), 1);
    // func_80B3E164: on the 30th, EnXc_SpawnNut: En_Arrow's ARROW_CS_NUT 30 in front of her
    // (her shape's 0x1000), 3 up, pitched 0xFA0; action 18, so 43.
    let arrows = |w: &PlayState| w.actors.all().into_iter().filter(|&h| w.actors.downcast::<oot_actors::en_arrow::EnArrow>(h).is_some()).collect::<Vec<_>>();
    assert!(arrows(&w).is_empty());
    for _ in 0..9 {
        update(&mut w, h);
    }
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_42);
    let pos = xc(&w, h).actor.world_pos;
    update(&mut w, h);
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_43);
    let nuts = arrows(&w);
    assert_eq!(nuts.len(), 1);
    let nut = w.actors.actor(nuts[0]).unwrap();
    let want = Vec3::new(eng_math::sin_s(0x1000) * 30.0 + pos.x, pos.y + 3.0, eng_math::cos_s(0x1000) * 30.0 + pos.z);
    assert!((nut.home_pos - want).length() < 1e-3, "{:?} {want:?}", nut.home_pos);
    assert_eq!((nut.home_rot.x, nut.home_rot.y), (0xFA0, 0x1000));
    // EnXc_Delete: under cue 4 waiting; cue 9 (EnXc_SetupDisappear) outside the crater: killed
    // (the action stays 43).
    update(&mut w, h);
    assert!(!xc(&w, h).actor.killed);
    cue(&mut w, 9, at, 0);
    update(&mut w, h);
    assert!(xc(&w, h).actor.killed);
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_43);
}

#[test]
fn type_9s_prelude_runs_type_2s_actions() {
    let Some(a) = assets() else { return };
    let mut w = temple(&a, true, |s| {
        s.set_event_chk_inf(EVENTCHKINF_C5);
        s.set_event_chk_inf(EVENTCHKINF_48);
    });
    let h = *sheiks(&w).iter().find(|&&h| !xc(&w, h).actor.killed).expect("Sheik");
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_30);
    // The script's first cue for her (gTempleOfTimePreludeCs: 6, frames 0..100): drawn, 31.
    cue(&mut w, 6, IVec3::new(-60, 71, 6), -911);
    update(&mut w, h);
    assert_eq!((xc(&w, h).action, xc(&w, h).draw_mode), (SHEIK_ACTION_31, SHEIK_DRAW_DEFAULT));
    // EnXc_ActionFunc44: nothing.
    xc_mut(&mut w, h).action = SHEIK_ACTION_44;
    let pos = xc(&w, h).actor.world_pos;
    update(&mut w, h);
    assert_eq!((xc(&w, h).action, xc(&w, h).actor.world_pos), (SHEIK_ACTION_44, pos));
}

/// Sacred Forest Meadow (`ENTR_SACRED_FOREST_MEADOW_0`) from `hookshot`, the sound log on, and
/// room 0's Sheik (`En_Xc` 6 at (-184, 480, -2640)).
fn meadow(a: &Arc<GameAssets>) -> (PlayState, ActorHandle) {
    let e = a.scenes.entrance_index("ENTR_SACRED_FOREST_MEADOW_0").expect("entrance");
    let mut save = SaveContext::new(e, true, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("hookshot").unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    settle(&mut w, ACTOR_EN_XC);
    w.audio.log = Some(oot_game::audio::AudioLog::default());
    let h = sheiks(&w).into_iter().find(|&h| xc(&w, h).actor.params == SHEIK_TYPE_MINUET && !xc(&w, h).actor.killed).expect("the meadow's Sheik");
    (w, h)
}

#[test]
fn type_6_teaches_the_minuet_past_z_2225_and_drops_on_her_cue() {
    use oot_actors::PlayExt;
    use oot_game::item::QUEST_SONG_MINUET;
    let Some(a) = assets() else { return };
    let (mut w, h) = meadow(&a);
    // func_80B3CA38: an adult without EVENTCHKINF_50: SHEIK_ACTION_INIT, drawn as nothing.
    assert_eq!((xc(&w, h).action, xc(&w, h).draw_mode), (SHEIK_ACTION_INIT, SHEIK_DRAW_NOTHING));
    // EnXc_ActionFunc0 (EnXc_MinuetCS): Link at z -2200 isn't past -2225: waiting.
    w.place_player(Vec3::new(0.0, 480.0, -2200.0), i16::MIN);
    update(&mut w, h);
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_INIT);
    assert!(!w.save.get_event_chk_inf(EVENTCHKINF_50));
    // Past it, no cutscene: gMeadowMinuetCs, cutsceneTrigger 1, EVENTCHKINF_50 and the Minuet
    // (Item_Give: its quest bit); func_80B3D750 (EnXc_BoleroCS true for type 6): waiting.
    w.place_player(Vec3::new(0.0, 480.0, -2300.0), i16::MIN);
    update(&mut w, h);
    assert!(w.save.get_event_chk_inf(EVENTCHKINF_50));
    assert_ne!(w.save.inventory.quest_items & (1 << QUEST_SONG_MINUET), 0);
    assert!(w.cs_ctx.segment.is_some());
    assert_eq!(w.save.cutscene_trigger, 1);
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_WAIT);
    // EnXc_ActionFunc1: cue 1 holds her; cue 2 (gMeadowMinuetCs's at frame 350) drops her at its
    // start, turned away from Link's cue (func_80B3D48C: 30401 + 0x8000), drawn, falling
    // (gSheikFallingFromSkyAnim), NA_SE_PL_SKIP (func_80B3D118).
    let start = IVec3::new(-232, 480, -2889);
    cue(&mut w, 1, start, 0x4000);
    update(&mut w, h);
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_WAIT);
    w.cs_ctx.link_action = Some(CsCmdActorCue { action: 5, start_frame: 0, end_frame: 100, rot: [0, 30401, 0], start_pos: IVec3::ZERO, end_pos: IVec3::ZERO, normal: IVec3::ZERO });
    cue(&mut w, 2, start, 0x4000);
    update(&mut w, h);
    let x = xc(&w, h);
    assert_eq!((x.action, x.draw_mode), (SHEIK_ACTION_GRACEFUL_FALL, SHEIK_DRAW_DEFAULT));
    assert_eq!(x.actor.world_pos, start.as_vec3());
    assert_eq!(x.actor.shape_rot.y, 30401i16.wrapping_add(i16::MIN));
    assert!(x.skel.is("gSheikFallingFromSkyAnim"));
    assert_eq!(sfx(&w, oot_game::audio::sfx::NA_SE_PL_SKIP), 1);
    // EnXc_GracefulFall: her root's motion moves her (AnimTask_ActorMovement) until the fall's
    // done (func_80B3D8A4): walking (gSheikWalkingAnim), gravity -13, the root's motion off.
    let mut n = 0;
    while xc(&w, h).action == SHEIK_ACTION_GRACEFUL_FALL {
        update(&mut w, h);
        oot_game::actor_ctx::ActorImpl::animation_update(xc_mut(&mut w, h));
        n += 1;
        assert!(n < 200, "still falling");
    }
    let x = xc(&w, h);
    assert_eq!(x.action, SHEIK_ACTION_ACCEL);
    assert!(x.skel.is("gSheikWalkingAnim"));
    assert_eq!((x.actor.gravity, x.actor.min_velocity_y), (-13.0, -13.0));
    eprintln!("fell {n} frames, to {}", x.actor.world_pos);
    // EnXc_Accelerate: walking at 1.2 on the 12th frame (EnXc_SetupWalkAction).
    assert_eq!(run_while(&mut w, h, SHEIK_ACTION_ACCEL), 12);
    assert_eq!((xc(&w, h).action, xc(&w, h).actor.speed_xz), (SHEIK_ACTION_WALK, 1.2));
    // EnXc_Walk: on until within 95 of Link (EnXc_SetupHaltAction: idling), then slowing to a
    // stop, stopped on the 12th frame (EnXc_SetupStoppedAction).
    xc_mut(&mut w, h).actor.xz_dist_to_player = 96.0;
    update(&mut w, h);
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_WALK);
    xc_mut(&mut w, h).actor.xz_dist_to_player = 95.0;
    update(&mut w, h);
    assert_eq!(xc(&w, h).action, SHEIK_ACTION_HALT);
    assert!(xc(&w, h).skel.is("gSheikIdleAnim"));
    assert_eq!(run_while(&mut w, h, SHEIK_ACTION_HALT), 12);
    assert_eq!((xc(&w, h).action, xc(&w, h).actor.speed_xz), (SHEIK_ACTION_STOPPED, 0.0));
}
