//! Navi's hint places (`Elf_Msg`, `z_elf_msg.c`) and tags (`Elf_Msg2`, `z_elf_msg2.c`) against
//! the C, in the Deku Tree (MQ):
//! - room 0's `Elf_Msg` 0x1F02 at (118, 0, -325), rot (20, 0, 0): a cylinder (bit 14 clear) 0.8 *
//!   100 across and 0.4 * 100 up, text 0x102 called at once (bit 15 clear: `naviTextId` -0x102),
//!   gone with its switch flag 0x1F; its two twins on the middle floor and room 2's three;
//! - room 0's 0x8001 at (0, -24, 0), rot (40, 6, 0): a C-Up hint (0x101), gone once switch flag 5
//!   is set, setting its own flag 0;
//! - room 4's two `Elf_Msg2` 0x3F06, rot (2, 26, 0), 60 above its timed torches: text 0x106,
//!   attention range 1, no flag of their own, gone once 0x19 is set.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, BTN_CUP, BTN_Z, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::elf_msg::{self, ElfMsg};
use oot_actors::elf_msg2::{self, ElfMsg2};
use oot_actors::en_elf::EnElf;
use oot_game::actor::*;
use oot_game::actor_ctx::ActorHandle;
use oot_game::message::*;
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

fn pad(button: u16) -> PadState {
    PadState { button, ..Default::default() }
}

/// Inside the Deku Tree (`deku-tree-inside`), with switch flags `flags` set first, then in `room`
/// (room 0 is the entrance's).
fn deku_tree_room(a: &Arc<GameAssets>, room: i8, flags: &[i32]) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-inside").unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    idle(&mut w, 60);
    for &f in flags {
        w.flags.set_switch(f);
    }
    if room != 0 {
        assert!(w.room_request(room));
        idle(&mut w, 1);
        w.room_change_done();
        idle(&mut w, 1);
    }
    w
}

/// The live `Elf_Msg`s with `params`.
fn elf_msgs(w: &PlayState, params: u16) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<ElfMsg>(h).is_some_and(|m| m.actor.params as u16 == params && !m.actor.killed)).collect()
}

fn elf_msg_at(w: &PlayState, params: u16, pos: Vec3) -> ActorHandle {
    elf_msgs(w, params).into_iter().find(|&h| w.actors.actor(h).unwrap().home_pos == pos).expect("the Elf_Msg")
}

/// The live `Elf_Msg2`s.
fn elf_msg2s(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<ElfMsg2>(h).is_some_and(|m| !m.actor.killed)).collect()
}

fn navi(w: &PlayState) -> &EnElf {
    w.actors.downcast::<EnElf>(w.player().navi_actor.expect("naviActor")).expect("Navi")
}

/// Link on the floor below `pos`, facing `yaw`.
fn place_link(w: &mut PlayState, pos: Vec3, yaw: i16) {
    let (y, _) = w.col.entity_raycast_down(pos + Vec3::Y * 50.0);
    w.place_player(Vec3::new(pos.x, y, pos.z), yaw);
}

const CALL: Vec3 = Vec3::new(118.0, 0.0, -325.0);

#[test]
fn room_0s_call_sets_navi_text_and_ends_with_her_talk() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 0, &[]);
    let h = elf_msg_at(&w, 0x1F02, CALL);
    let m = w.actors.downcast::<ElfMsg>(h).unwrap();
    // ElfMsg_Init: rot.x 20 → scale.x = scale.z = 20 * 0.04; rot.z 0 → scale.y 0.4; bit 14 clear:
    // ElfMsg_CallNaviCylinder; the shape's rotation zeroed (the world's kept).
    assert_eq!(m.action, elf_msg::Action::CallNaviCylinder);
    assert_eq!(m.actor.scale, Vec3::new(20.0 * 0.04, 0.4, 20.0 * 0.04));
    assert_eq!(m.actor.shape_rot, Rot { x: 0, y: 0, z: 0 });
    assert_eq!(m.actor.world_rot.x, 20);
    // ElfMsg_GetMessageId: bit 15 clear, -(0x02 + 0x100).
    assert_eq!(m.get_message_id(), -0x102);

    // (It stands over a gap in the floor: Link is put on the floor beside it, at y 0.) Link 80
    // off in x and z, 113 across: outside the 80 (ElfMsg_WithinXZDistance): no text (naviTimer
    // is under 600, so Navi sets none either).
    place_link(&mut w, CALL + Vec3::new(80.0, 0.0, 80.0), 0x4000);
    idle(&mut w, 1);
    assert_eq!(w.player().navi_text_id, 0);
    assert_eq!(navi(&w).elf_msg, None);

    // Link 40 off in x and z, 57 across: inside, on its floor (0 above it, under 0.4 * 100).
    // Player clears naviTextId in its update; the Elf_Msg (newer than Navi in
    // ACTORCAT_ITEMACTION) sets it after, and Navi's elfMsg; Navi then sees it negative
    // (ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED) and clears elfMsg at the end of her update.
    place_link(&mut w, CALL + Vec3::new(40.0, 0.0, 40.0), 0x4000);
    assert_eq!(w.player().actor.world_pos.y, 0.0);
    idle(&mut w, 1);
    assert_eq!(w.player().navi_text_id, -0x102);
    assert_ne!(navi(&w).actor.flags & ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED, 0);
    assert_eq!(navi(&w).elf_msg, None);

    // Player_ActionHandler_Talk: naviTextId negative and outside 0x2XX: the talk with Navi at
    // once, her text 0x102. Navi's talk accepted gives the Elf_Msg ACTOR_FLAG_TALK; its next
    // update (Actor_TalkOfferAccepted) sets 0x1F and kills it.
    let mut gone = None;
    for f in 0..10 {
        idle(&mut w, 1);
        if w.actors.actor(h).is_none_or(|a| a.killed) {
            gone = Some(f);
            break;
        }
    }
    assert!(gone.is_some(), "the Elf_Msg is still there");
    assert!(w.flags.get_switch(0x1F));
    assert_ne!(w.message_state(), TEXT_STATE_NONE);
    assert_eq!(w.msg_ctx.text_id, 0x102);
    // ElfMsg_KillCheck: the other two 0x1F02 are gone too, on their next update.
    idle(&mut w, 1);
    assert!(elf_msgs(&w, 0x1F02).is_empty());
}

#[test]
fn elf_msgs_are_gone_with_their_flags() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 0, &[]);
    assert_eq!((elf_msgs(&w, 0x1F02).len(), elf_msgs(&w, 0x0103).len(), elf_msgs(&w, 0x8001).len()), (3, 2, 1));
    // 0x8001: rot (40, 6, 0): scale 1.6 across; a C-Up hint, +0x101.
    let hint = elf_msgs(&w, 0x8001)[0];
    let m = w.actors.downcast::<ElfMsg>(hint).unwrap();
    assert_eq!((m.actor.scale.x, m.get_message_id()), (40.0 * 0.04, 0x101));
    // Switch flag 5 (rot.y 6 - 1): ElfMsg_KillCheck kills it and sets its own flag,
    // PARAMS_GET_U(0x8001, 8, 6) = 0 (not 0x3F).
    assert!(!w.flags.get_switch(0));
    w.flags.set_switch(5);
    idle(&mut w, 1);
    assert!(elf_msgs(&w, 0x8001).is_empty());
    assert!(w.flags.get_switch(0));
    assert_eq!(elf_msgs(&w, 0x1F02).len(), 3);
    // Its own flag set (0x1F): the 0x1F02s go, not the 0x0103s (flag 1).
    w.flags.set_switch(0x1F);
    idle(&mut w, 1);
    assert_eq!((elf_msgs(&w, 0x1F02).len(), elf_msgs(&w, 0x0103).len()), (0, 2));
}

#[test]
fn an_elf_msg_with_its_flag_set_is_gone_from_its_init() {
    let Some(a) = assets() else { return };
    // Room 2's three 0x1F02, with 0x1F already set: ElfMsg_Init's ElfMsg_KillCheck kills them
    // before anything else.
    let w = deku_tree_room(&a, 2, &[0x1F]);
    assert!(elf_msgs(&w, 0x1F02).is_empty());
    let w = deku_tree_room(&a, 2, &[]);
    assert_eq!(elf_msgs(&w, 0x1F02).len(), 3);
}

#[test]
fn room_4s_tag_says_its_text_with_c_up() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 4, &[]);
    let tag_pos = Vec3::new(-281.0, -820.0, 881.0);
    let tags = elf_msg2s(&w);
    assert_eq!(tags.len(), 2);
    let h = tags.into_iter().find(|&h| w.actors.actor(h).unwrap().world_pos == tag_pos).expect("the tag");
    let m = w.actors.downcast::<ElfMsg2>(h).unwrap();
    // ElfMsg2_Init: rot.x 2 → attentionRangeType 1; scale 0.2 (ICHAIN_VEC3F_DIV1000(scale, 200));
    // rot.y 26 (under 0x41): ElfMsg2_WaitForTextRead, targetable with C-Up, textId 0x106.
    assert_eq!((m.actor.target_mode, m.actor.scale, m.action), (1, Vec3::splat(0.2), elf_msg2::Action::WaitForTextRead));
    assert_eq!(m.actor.flags & (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_TALK_WITH_C_UP), ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_TALK_WITH_C_UP);
    assert_eq!((m.actor.text_id, m.get_message_id()), (0x106, 0x106));
    assert_eq!(m.actor.shape_rot, Rot { x: 0, y: 0, z: 0 });

    // Link 80 off in +z, facing it (-z): Z locks on to it.
    place_link(&mut w, Vec3::new(tag_pos.x, tag_pos.y, tag_pos.z + 80.0), -0x8000);
    idle(&mut w, 2);
    let mut prev = PadState::default();
    for p in [pad(BTN_Z), PadState::default(), PadState::default()] {
        w.tick_with(scripted_input(prev, p));
        prev = p;
    }
    assert_eq!(w.player().focus_actor, Some(h), "locked on to the tag");
    // C-Up (Player_ActionHandler_Talk: a focus actor with ACTOR_FLAG_ATTENTION_ENABLED |
    // ACTOR_FLAG_TALK_WITH_C_UP): the talk with the tag, its text 0x106.
    let mut opened = false;
    for p in [pad(BTN_CUP), PadState::default(), PadState::default(), PadState::default(), PadState::default()] {
        w.tick_with(scripted_input(prev, p));
        prev = p;
        if w.message_state() != TEXT_STATE_NONE {
            opened = true;
            break;
        }
    }
    assert!(opened, "its text opens");
    assert_eq!(w.msg_ctx.text_id, 0x106);
    idle(&mut w, 1);
    prev = PadState::default();
    assert_eq!(w.actors.downcast::<ElfMsg2>(h).unwrap().action, elf_msg2::Action::WaitForTextClose);
    // A through the text: on TEXT_STATE_CLOSING (Actor_TextboxIsClosing), rot.z 0 (not 1): it's
    // gone; its flag 0x3F is none.
    let swch = (w.flags.swch, w.flags.temp_swch);
    for _ in 0..600 {
        if w.actors.actor(h).is_none_or(|a| a.killed) {
            break;
        }
        let st = w.message_state();
        let waits = matches!(st, TEXT_STATE_AWAITING_NEXT | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT) || w.msg_ctx.msg_mode == MSGMODE_TEXT_AWAIT_INPUT;
        let p = if waits && prev.button == 0 { pad(BTN_A) } else { PadState::default() };
        w.tick_with(scripted_input(prev, p));
        prev = p;
    }
    assert!(w.actors.actor(h).is_none_or(|a| a.killed), "the tag is gone");
    assert_eq!((w.flags.swch, w.flags.temp_swch), swch);
    assert_eq!(elf_msg2s(&w).len(), 1, "the other tag stays");
}
