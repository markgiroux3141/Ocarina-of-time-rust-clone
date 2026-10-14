//! GAME-07 milestone 6 against the C: the graveyard kid (`z_en_cs.c`) and the soft soil where
//! bugs make a Gold Skulltula (`z_obj_makekinsuta.c`), in the graveyard (`spot02_scene`, the
//! child's day: layer 0).
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use eng_gfx::MeshKey;
use eng_math::{cos_s, sin_s};
use glam::{Mat4, Vec3};
use oot_actors::PlayExt;
use oot_actors::en_cs::{self, Action, EnCs};
use oot_actors::en_sw::{self, EnSw};
use oot_actors::obj_makekinsuta::{self, ObjMakekinsuta};
use oot_actors::player::Player;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::audio::sfx::NA_SE_SY_TRE_BOX_APPEAR;
use oot_game::message::{MSGMODE_TEXT_AWAIT_INPUT, TEXT_STATE_AWAITING_NEXT, TEXT_STATE_CHOICE, TEXT_STATE_DONE, TEXT_STATE_DONE_HAS_NEXT, TEXT_STATE_EVENT, TEXT_STATE_NONE};
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, ViewInfo, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

/// spot02 layer 0: En_Cs params 0 at (-474, 61, 447); its path 0's six points.
const KID_HOME: Vec3 = Vec3::new(-474.0, 61.0, 447.0);
const PATH_0: [[f32; 3]; 6] = [[-474.0, 61.0, 447.0], [-474.0, 121.0, -239.0], [48.0, 121.0, -232.0], [48.0, 121.0, 680.0], [-470.0, 61.0, 680.0], [-474.0, 61.0, 447.0]];
/// spot02 layers 0 and 1: Obj_Makekinsuta params 0x5101 at (-715, 120, -340) (over Obj_Bean's
/// soft soil).
const SOIL_POS: Vec3 = Vec3::new(-715.0, 120.0, -340.0);
/// `EVENTCHKINF_B6` (`save.h`): the graveyard's intro (`gGraveyardIntroCs`, `z_demo.c`'s
/// `sEntranceCutsceneTable` for `ENTR_GRAVEYARD_0`) seen.
const EVENTCHKINF_B6: u16 = 0xB6;
/// `ITEM_MASK_SPOOKY` (`item.h`: 0x26).
const ITEM_MASK_SPOOKY: u8 = 0x26;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Link, a child, entered at `ENTR_GRAVEYARD_0` (spawn 0, (-1411, 0, 330)) at `hour`:00, the
/// graveyard's intro seen, `edit` applied to the save last; two frames on.
fn graveyard(a: &Arc<GameAssets>, hour: i32, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_GRAVEYARD_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(hour, 0) as u16);
    save.apply_preset("deku-tree-dead").unwrap();
    save.adult = false;
    save.set_event_chk_inf(EVENTCHKINF_B6);
    edit(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    idle(&mut w, 2);
    w
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn all<T: 'static + ActorImpl>(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<T>(h).is_some_and(|a| !a.base().killed)).collect()
}

fn one<T: 'static + ActorImpl>(w: &PlayState) -> ActorHandle {
    let v = all::<T>(w);
    assert_eq!(v.len(), 1, "one {}", std::any::type_name::<T>());
    v[0]
}

fn kid(w: &PlayState, h: ActorHandle) -> &EnCs {
    w.actors.downcast::<EnCs>(h).unwrap()
}

fn base(w: &PlayState, h: ActorHandle) -> &Actor {
    w.actors.actor(h).unwrap()
}

fn player(w: &PlayState) -> &Player {
    w.actors.downcast::<Player>(w.player.unwrap()).unwrap()
}

fn sfx_frames(w: &PlayState, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, _)| f).collect()
}

/// The kid, made to update out of view too (he has no `ACTOR_FLAG_UPDATE_CULLING_DISABLED`:
/// in the game he walks only while the camera sees him).
fn the_kid(w: &mut PlayState) -> ActorHandle {
    let h = one::<EnCs>(w);
    w.actors.actor_mut(h).unwrap().flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
    h
}

fn xz(v: Vec3) -> glam::Vec2 {
    glam::Vec2::new(v.x, v.z)
}

#[test]
fn the_kid_is_there_by_day_only() {
    let Some(a) = assets() else { return };
    // EnCs_Init by day: ATTENTION_RANGE_6, path = params bits 0-7 (0), talkState 0, gravity -1,
    // ENCS_ANIM_0 (gGraveyardKidWalkAnim, once), EnCs_Walk, walkSpeed 1; immovable, cylinder
    // 18 x 63.
    let w = graveyard(&a, 10, |_| {});
    let h = one::<EnCs>(&w);
    let k = kid(&w, h);
    eprintln!("the kid at {:?}, walk anim {} frames", k.actor.world_pos, k.skel.anim_length);
    assert_eq!(k.actor.home_pos, KID_HOME);
    assert_eq!((k.actor.target_mode, k.path, k.actor.gravity), (6, 0, -1.0));
    assert_eq!((k.current_anim_index, k.action), (en_cs::ENCS_ANIM_0, Action::Walk));
    assert!(k.skel.is("gGraveyardKidWalkAnim"));
    assert_eq!((k.collider.dim.radius, k.collider.dim.height), (18, 63));
    assert_eq!(k.actor.col_chk_info.mass, oot_game::collision_check::MASS_IMMOVABLE);
    // His path is spot02's path 0.
    let path = &w.setup_path_list()[0];
    assert_eq!(path.count(), 6);
    for (i, p) in PATH_0.iter().enumerate() {
        assert_eq!(path.point(i), Vec3::from(*p));
    }
    // By night (!IS_DAY): Actor_Kill.
    let w = graveyard(&a, 20, |_| {});
    assert!(all::<EnCs>(&w).is_empty(), "no kid by night");
}

/// One frame of the kid's walk, what it changed.
struct Step {
    frame: usize,
    action_before: Action,
    action: Action,
    pos_before: Vec3,
    rot_before: i16,
    rot: i16,
    waypoint_before: i32,
    waypoint: i32,
}

/// `n` frames of the kid walking, Link far off at his spawn.
fn walk(w: &mut PlayState, h: ActorHandle, n: usize) -> Vec<Step> {
    let mut v = Vec::new();
    for frame in 0..n {
        let k = kid(w, h);
        let (action_before, pos_before, rot_before, waypoint_before) = (k.action, k.actor.world_pos, k.actor.shape_rot.y, k.waypoint);
        idle(w, 1);
        let k = kid(w, h);
        assert_eq!(k.talk_state, 0, "no talk with Link far off");
        v.push(Step { frame, action_before, action: k.action, pos_before, rot_before, rot: k.actor.shape_rot.y, waypoint_before, waypoint: k.waypoint });
        if action_before == Action::Walk {
            // The speed by the walk's frame after this update's SkelAnime_Update (0 on frames
            // 0, 8-15, 23-29; else 1); a wait begun this frame changed the animation, frame 0.
            let f = k.skel.cur_frame as i32;
            assert_eq!(k.actor.speed_xz, if EnCs::stands_on_frame(f) { 0.0 } else { 1.0 }, "frame {frame}: anim frame {f}");
            // Math_SmoothStepToS(&shape.rot.y, walkAngle, 1, 2500, 0): straight to the angle
            // within 2500, else 2500 towards it.
            let target = k.walk_angle as i16;
            let d = target.wrapping_sub(rot_before);
            let expect = if (d as i32).abs() <= 2500 { target } else { rot_before.wrapping_add(2500 * d.signum()) };
            assert_eq!(k.actor.shape_rot.y, expect, "frame {frame}: the turn");
            assert_eq!(k.actor.world_rot.y, k.actor.shape_rot.y);
            // Actor_MoveXZGravity: speed along the new yaw, by Actor_UpdatePos' speedRate
            // (R_UPDATE_RATE 3 * 0.5: 1.5).
            let s = k.actor.speed_xz * 1.5;
            let r = k.actor.shape_rot.y;
            let moved = xz(k.actor.world_pos) - xz(pos_before);
            assert!((moved - glam::Vec2::new(s * sin_s(r), s * cos_s(r))).length() < 1e-3, "frame {frame}: moved {moved:?}");
        } else {
            // Waiting (EnCs_Wait) he doesn't move.
            assert_eq!(xz(k.actor.world_pos), xz(pos_before), "frame {frame}: waiting");
        }
    }
    v
}

#[test]
fn he_walks_his_path_round_and_round() {
    let Some(a) = assets() else { return };
    let mut w = graveyard(&a, 10, |_| {});
    let h = the_kid(&mut w);
    let steps = walk(&mut w, h, 16000);
    // EnCs_HandleWalking: within 10.44 of his waypoint he takes the next (round to 0 after 5);
    // at his home he's on point 0 (and point 5 is point 0), so 0 -> 1 at once and 5 -> 0 -> 1
    // in one frame.
    let mut reached = Vec::new();
    for s in steps.iter().filter(|s| s.waypoint != s.waypoint_before) {
        let expect = if s.waypoint_before == 5 || s.waypoint_before == 0 { 1 } else { s.waypoint_before + 1 };
        assert_eq!(s.waypoint, expect, "frame {}: {} -> {}", s.frame, s.waypoint_before, s.waypoint);
        let p = Vec3::from(PATH_0[s.waypoint_before as usize]);
        let d = (xz(p) - xz(s.pos_before)).length();
        assert!(d <= en_cs::WAYPOINT_REACHED, "frame {}: {d} from point {}", s.frame, s.waypoint_before);
        reached.push((s.frame, s.waypoint_before, s.pos_before));
    }
    for (f, wp, p) in &reached {
        eprintln!("frame {f:5} ({:6.1} s): point {wp} reached at ({:.0}, {:.0}, {:.0})", *f as f32 / 20.0, p.x, p.y, p.z);
    }
    // He went round at least twice: points 1 to 5 each reached twice, in order.
    let seq: Vec<i32> = reached.iter().map(|r| r.1).collect();
    assert!(seq.len() >= 11, "{seq:?}");
    assert_eq!(&seq[..11], &[0, 1, 2, 3, 4, 5, 1, 2, 3, 4, 5]);
    // He turns at the corners, 2500 a frame at most (checked frame by frame above): at point 1
    // from heading -z (yaw -0x8000) to about +x (0x4000), seven frames.
    let f1 = reached[1].0;
    let turn: Vec<i16> = steps[f1..].iter().filter(|s| s.rot != s.rot_before).take(7).map(|s| s.rot.wrapping_sub(s.rot_before)).collect();
    assert!(turn[..6].iter().all(|d| d.abs() == 2500), "{turn:?}");
    assert!(turn[6].abs() < 2500, "{turn:?}");
    // He waited now and then (one end of his walk in five without the mask): each wait one
    // play of gGraveyardKidGrabStickTwoHandsAnim, then walking.
    let waits = steps.iter().filter(|s| s.action_before == Action::Walk && s.action == Action::Wait).count();
    let ends = steps.iter().filter(|s| s.action_before == Action::Wait && s.action == Action::Walk).count();
    eprintln!("{waits} waits in {} frames", steps.len());
    assert!(waits > 0);
    assert!(ends + 1 >= waits);
}

#[test]
fn his_waits_his_steps_and_his_eyes() {
    let Some(a) = assets() else { return };
    let mut w = graveyard(&a, 10, |_| {});
    w.audio.log = Some(Default::default());
    let h = the_kid(&mut w);
    let mut eyes = Vec::new();
    let mut expect_steps = Vec::new();
    let mut wait_anims = Vec::new();
    for _ in 0..3000 {
        let k = kid(&w, h);
        // EnCs_Update: NA_SE_EV_CHIBI_WALK on the animation's frame before this update's
        // SkelAnime_Update: walk 9 and 23, stick swing 10 and 25, stick grab 20.
        let (i, f) = (k.current_anim_index, k.skel.cur_frame as i32);
        let step = match i {
            0 => f == 9 || f == 23,
            1 => f == 10 || f == 25,
            2 => f == 20,
            _ => false,
        };
        let was = k.action;
        idle(&mut w, 1);
        if step {
            expect_steps.push(w.audio.frames);
        }
        let k = kid(&w, h);
        if was == Action::Walk && k.action == Action::Wait {
            wait_anims.push(k.current_anim_index);
        }
        eyes.push(k.eye_index);
    }
    // EnCs_Walk without ITEMGETINF_3A: a wait plays ENCS_ANIM_2 (the stick grab).
    assert!(!wait_anims.is_empty());
    assert!(wait_anims.iter().all(|&i| i == en_cs::ENCS_ANIM_2), "{wait_anims:?}");
    let got = sfx_frames(&w, en_cs::NA_SE_EV_CHIBI_WALK);
    assert!(!expect_steps.is_empty());
    assert_eq!(got, expect_steps);
    // eyeBlinkFrames { 70, 1, 1 }: the timer counts below 0 before the next eye, so each eye
    // stays one frame more: open 71 frames, half 2, closed 2.
    let mut runs: Vec<(i32, usize)> = Vec::new();
    for e in eyes {
        match runs.last_mut() {
            Some((x, n)) if *x == e => *n += 1,
            _ => runs.push((e, 1)),
        }
    }
    let steady = &runs[1..runs.len() - 1];
    assert!(steady.len() >= 6);
    for &(e, n) in steady {
        assert_eq!(n, if e == 0 { 71 } else { 2 }, "{runs:?}");
    }
    // ITEMGETINF_3A: one end in ten waits, half the time with ENCS_ANIM_1 (the stick swing).
    let mut w = graveyard(&a, 10, |s| s.set_item_get_inf(en_cs::ITEMGETINF_3A));
    let h = the_kid(&mut w);
    let mut wait_anims = Vec::new();
    for _ in 0..12000 {
        let was = kid(&w, h).action;
        idle(&mut w, 1);
        let k = kid(&w, h);
        if was == Action::Walk && k.action == Action::Wait {
            wait_anims.push(k.current_anim_index);
        }
    }
    assert!(wait_anims.iter().all(|&i| i == en_cs::ENCS_ANIM_1 || i == en_cs::ENCS_ANIM_2), "{wait_anims:?}");
    eprintln!("with the mask: waits {wait_anims:?}");
}

#[test]
fn his_texts() {
    // EnCs_GetTextId: the mask reaction (0: none) unless... ITEMGETINF_3A: 0x2028 (a reaction
    // wins); else the Spooky Mask on: 0x2023 (over any reaction); else 0x2022 (a reaction wins).
    assert_eq!(EnCs::text_id_for(false, 0, 0), 0x2022);
    assert_eq!(EnCs::text_id_for(false, en_cs::PLAYER_MASK_SPOOKY, 0), 0x2023);
    assert_eq!(EnCs::text_id_for(true, 0, 0), 0x2028);
    assert_eq!(EnCs::text_id_for(true, en_cs::PLAYER_MASK_SPOOKY, 0), 0x2028);
    assert_eq!(EnCs::text_id_for(false, 1, 0), 0x2022, "the Keaton Mask: no reaction set here");
    assert_eq!(EnCs::text_id_for(false, 0, 0x7123), 0x7123);
    assert_eq!(EnCs::text_id_for(true, 0, 0x7123), 0x7123);
    assert_eq!(EnCs::text_id_for(false, en_cs::PLAYER_MASK_SPOOKY, 0x7123), 0x2023);

    let Some(a) = assets() else { return };
    // In play (no masks: Player's currentMask is PLAYER_MASK_NONE): 0x2022, and 0x2028 once he
    // has the mask, offered within 100 while on screen.
    for (bought, text) in [(false, 0x2022), (true, 0x2028)] {
        let mut w = graveyard(&a, 10, |s| {
            if bought {
                s.set_item_get_inf(en_cs::ITEMGETINF_3A)
            }
        });
        let h = the_kid(&mut w);
        let offered = |w: &mut PlayState, d: f32| {
            for _ in 0..20 {
                let p = base(w, h).world_pos;
                // Link `d` south of him (+z), facing him (north, yaw 0x8000).
                w.place_player(Vec3::new(p.x, p.y, p.z + d), i16::MIN);
                idle(w, 1);
            }
            let p = base(w, h).world_pos;
            w.place_player(Vec3::new(p.x, p.y, p.z + d), i16::MIN);
            idle(w, 1);
            player(w).target_actor == Some(h)
        };
        // Actor_OfferTalk(.., 100): xz and y within 100.
        assert!(!offered(&mut w, 110.0), "no offer at 110");
        assert!(offered(&mut w, 90.0), "an offer at 90");
        assert_eq!(base(&w, h).text_id, text);
    }
}

/// Presses A whenever the message box waits, from the talk's start until `done` or 3000 frames;
/// the texts shown. At a choice, `choice` is picked.
fn read_on(w: &mut PlayState, choice: u8, done: impl Fn(&PlayState) -> bool) -> Vec<u16> {
    let mut prev = PadState::default();
    let mut texts = Vec::new();
    for _ in 0..3000 {
        let st = w.message_state();
        if st != TEXT_STATE_NONE && texts.last() != Some(&w.msg_ctx.text_id) {
            texts.push(w.msg_ctx.text_id);
        }
        if done(w) {
            return texts;
        }
        if st == TEXT_STATE_CHOICE {
            w.msg_ctx.choice_index = choice;
        }
        let waits = matches!(st, TEXT_STATE_AWAITING_NEXT | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_CHOICE | TEXT_STATE_EVENT) || w.msg_ctx.msg_mode == MSGMODE_TEXT_AWAIT_INPUT;
        let pad = if waits && prev.button == 0 { PadState { button: BTN_A, ..Default::default() } } else { PadState::default() };
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
    }
    panic!("never done: {texts:x?}");
}

/// Link 40 south of the kid facing him until he offers to talk; then A, his text forced to
/// `text` (what `EnCs_GetTextId` gives with the Spooky Mask on: masks aren't ported).
fn talk_with(w: &mut PlayState, h: ActorHandle, text: u16) {
    for _ in 0..40 {
        let p = base(w, h).world_pos;
        w.place_player(Vec3::new(p.x, p.y, p.z + 40.0), i16::MIN);
        idle(w, 1);
        if player(w).target_actor == Some(h) {
            w.actors.actor_mut(h).unwrap().text_id = text;
            let a = PadState { button: BTN_A, ..Default::default() };
            w.tick_with(scripted_input(PadState::default(), a));
            w.tick_with(scripted_input(a, PadState::default()));
            return;
        }
    }
    panic!("the kid never offered to talk");
}

#[test]
fn the_spooky_mask_sold() {
    let Some(a) = assets() else { return };
    // The Spooky Mask in the child's trade slot, on C-Left.
    let mut w = graveyard(&a, 10, |s| {
        s.inventory.items[oot_game::item::SLOT_TRADE_CHILD] = ITEM_MASK_SPOOKY;
        s.equip_item_on_c_left(ITEM_MASK_SPOOKY);
    });
    w.audio.log = Some(Default::default());
    let h = the_kid(&mut w);
    let money = |w: &PlayState| w.save.rupees as i32 + w.save.rupee_accumulator as i32;
    let before = money(&w);
    // Talked to with 0x2023 (the mask on): EnCs_HandleTalking, accepted: not 0x2028:
    // ENCS_ANIM_3; 0x2023: ENCS_ANIM_1; NA_SE_SY_TRE_BOX_APPEAR; talkState 1.
    talk_with(&mut w, h, 0x2023);
    let k = kid(&w, h);
    assert_eq!(w.msg_ctx.text_id, 0x2023);
    assert_eq!((k.talk_state, k.current_anim_index), (1, en_cs::ENCS_ANIM_1));
    assert_eq!(sfx_frames(&w, NA_SE_SY_TRE_BOX_APPEAR).len(), 1);
    // EnCs_Walk sees talkState (on the A's release frame): EnCs_Talk, which next frame sets flag
    // bit 0 (the head and torso turns drawn) and turns him to Link.
    idle(&mut w, 1);
    let k = kid(&w, h);
    assert_eq!(k.action, Action::Talk);
    assert_eq!(k.flag & 1, 1);
    // Its choice, yes (0): 0x2026, ENCS_ANIM_3, talkState 2: next frame Message_ContinueTextbox.
    // 0x2026 read (TEXT_STATE_DONE): Player_UnsetMask, Item_Give(ITEM_SOLD_OUT) (the slot sold
    // out, C-Left emptied), ITEMGETINF_3A, Rupees_ChangeBy(30), 0x2027.
    let texts = read_on(&mut w, 0, |w| w.save.get_item_get_inf(en_cs::ITEMGETINF_3A));
    assert_eq!(texts, [0x2023, 0x2026], "{texts:x?}");
    assert_eq!(kid(&w, h).actor.text_id, 0x2027);
    assert_eq!(w.save.inventory.items[oot_game::item::SLOT_TRADE_CHILD], oot_game::item::ITEM_SOLD_OUT);
    assert_eq!(w.save.equips.button_items[1], oot_game::item::ITEM_NONE);
    assert_eq!(money(&w), before + 30);
    // (talkState 2: 0x2027 continued from the next frame.) 0x2027 read: talkState 0:
    // ENCS_ANIM_0, EnCs_Walk, flag bit 0 cleared.
    let texts = read_on(&mut w, 0, |w| kid(w, h).talk_state == 0);
    assert_eq!(texts, [0x2026, 0x2027], "{texts:x?}");
    idle(&mut w, 1);
    let k = kid(&w, h);
    assert_eq!((k.action, k.flag & 1, k.current_anim_index), (Action::Walk, 0, en_cs::ENCS_ANIM_0));
    // From now on, 0x2028.
    assert_eq!(EnCs::get_text_id(&w), 0x2028);
}

#[test]
fn the_spooky_mask_kept() {
    let Some(a) = assets() else { return };
    let mut w = graveyard(&a, 10, |_| {});
    let h = the_kid(&mut w);
    // No (1): 0x2024, ENCS_ANIM_1; read: talkState 0; nothing sold.
    talk_with(&mut w, h, 0x2023);
    let texts = read_on(&mut w, 1, |w| kid(w, h).talk_state == 0);
    assert_eq!(texts, [0x2023, 0x2024], "{texts:x?}");
    assert!(!w.save.get_item_get_inf(en_cs::ITEMGETINF_3A));
    // A plain talk: 0x2022 (ENCS_ANIM_3), read: talkState 0.
    let mut w = graveyard(&a, 10, |_| {});
    let h = the_kid(&mut w);
    talk_with(&mut w, h, 0x2022);
    assert_eq!(kid(&w, h).current_anim_index, en_cs::ENCS_ANIM_3);
    let texts = read_on(&mut w, 0, |w| kid(w, h).talk_state == 0);
    assert_eq!(texts, [0x2022], "{texts:x?}");
}

#[test]
fn he_wears_the_mask_he_bought() {
    let Some(a) = assets() else { return };
    // EnCs_Draw: with ITEMGETINF_3A (and object_link_child in: Link's, a child), the Spooky Mask
    // after the skeleton, at spookyMaskMtx (his head's matrix, 200 down it, 100 degrees about z).
    let mask = MeshKey::named(keys::bake(en_cs::SPOOKY_MASK_BAKE));
    // The bakes are in the pack: each eye's skeleton and the mask.
    let pack = common::pack().unwrap();
    for eye in 0..3 {
        assert!(pack.bake(&en_cs::bake_name(eye)).is_ok_and(|d| !d.batches.is_empty()), "eye {eye}");
    }
    assert!(pack.bake(en_cs::SPOOKY_MASK_BAKE).is_ok_and(|d| !d.batches.is_empty()), "the Spooky Mask");
    for bought in [false, true] {
        let mut w = graveyard(&a, 10, |s| {
            if bought {
                s.set_item_get_inf(en_cs::ITEMGETINF_3A)
            }
        });
        let h = the_kid(&mut w);
        idle(&mut w, 3);
        let k = kid(&w, h);
        let mut out = DrawOut::default();
        k.draw(&k.render_state(), &w, &ViewInfo::new(Vec3::ZERO, Mat4::IDENTITY), &mut out);
        let masks: Vec<_> = out.opa.iter().filter(|c| c.mesh == mask).collect();
        assert_eq!(masks.len(), bought as usize);
        if let Some(m) = masks.first() {
            // On his face: within his height of his position.
            let at = m.transform.transform_point3(Vec3::ZERO);
            assert!((at - k.actor.world_pos).length() < 70.0, "{at:?}");
        }
        // The focus: 500 along and 800 up his head (EnCs_PostLimbDraw), above his feet.
        assert!(k.actor.focus_pos.y > k.actor.world_pos.y + 20.0, "{:?}", k.actor.focus_pos);
    }
}

#[test]
fn the_soil_makes_a_gold_skulltula_60_frames_after_a_bug() {
    let Some(a) = assets() else { return };
    let mut w = graveyard(&a, 10, |_| {});
    let h = one::<ObjMakekinsuta>(&w);
    assert_eq!((base(&w, h).params, base(&w, h).world_pos), (0x5101, SOIL_POS));
    let gold = |w: &PlayState| w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnSw>(h).is_some()).collect::<Vec<_>>();
    assert!(gold(&w).is_empty());
    // func_80B98320 without unk_152: the timer kept 0, nothing spawned.
    idle(&mut w, 100);
    let s = w.actors.downcast::<ObjMakekinsuta>(h).unwrap();
    assert_eq!((s.timer, s.action), (0, obj_makekinsuta::Action::Func80B98320));
    assert!(gold(&w).is_empty());
    // A bug in (En_Insect's write): each update with the timer under 60 counts it; the 61st
    // (timer 60, Link free: func_8002DEEC false) spawns ACTOR_EN_SW at the soil, params
    // 0x5101 | 0x8000, then ObjMakekinsuta_DoNothing.
    w.actors.downcast_mut::<ObjMakekinsuta>(h).unwrap().unk_152 = 1;
    idle(&mut w, 60);
    let s = w.actors.downcast::<ObjMakekinsuta>(h).unwrap();
    assert_eq!((s.timer, s.action), (60, obj_makekinsuta::Action::Func80B98320));
    assert!(gold(&w).is_empty());
    idle(&mut w, 1);
    assert_eq!(w.actors.downcast::<ObjMakekinsuta>(h).unwrap().action, obj_makekinsuta::Action::DoNothing);
    let g = gold(&w);
    assert_eq!(g.len(), 1);
    let sw = w.actors.downcast::<EnSw>(g[0]).unwrap();
    // EnSw_Init's conversion of 0xD101: type (0x5101 >> 13) + 1 = 3 (spawned, jumping out), the
    // flags' index 0x11 - 1 = 0x10, flag 1: 0x7001.
    assert_eq!(sw.actor.params, en_sw::init_params((0x5101u16 | 0x8000) as i16));
    assert_eq!(sw.actor.params, 0x7001);
    assert_eq!(sw.actor.home_pos, SOIL_POS);
    // Nothing more after.
    idle(&mut w, 100);
    assert_eq!(gold(&w).len(), 1);
}

#[test]
fn the_soil_waits_while_link_is_busy() {
    let Some(a) = assets() else { return };
    let mut w = graveyard(&a, 10, |_| {});
    let h = one::<ObjMakekinsuta>(&w);
    // Link walks in from the entrance in PLAYER_STATE1_29 for some 18 frames: free after.
    idle(&mut w, 30);
    // func_8002DEEC: PLAYER_STATE1_DEAD or PLAYER_STATE1_29, or a csAction.
    assert!(!obj_makekinsuta::func_8002deec(&w));
    let ph = w.player.unwrap();
    let set = |w: &mut PlayState, state1: u32, cs: u8| {
        let p = w.actors.downcast_mut::<Player>(ph).unwrap();
        p.state1 = state1;
        p.cs_mode = cs;
    };
    let state1 = player(&w).state1;
    set(&mut w, state1 | oot_actors::player::STATE1_29, 0);
    assert!(obj_makekinsuta::func_8002deec(&w));
    set(&mut w, state1 | oot_actors::player::STATE1_7, 0);
    assert!(obj_makekinsuta::func_8002deec(&w));
    set(&mut w, state1, 1);
    assert!(obj_makekinsuta::func_8002deec(&w));
    // Busy at 60: the timer counts on (61, 62), nothing spawned; free: spawned.
    {
        let s = w.actors.downcast_mut::<ObjMakekinsuta>(h).unwrap();
        s.unk_152 = 1;
        s.timer = 60;
    }
    let update_once = |w: &mut PlayState| {
        let mut s = w.actors.take(h).unwrap();
        s.update(w);
        w.actors.put_back(h, s);
    };
    update_once(&mut w);
    update_once(&mut w);
    let s = w.actors.downcast::<ObjMakekinsuta>(h).unwrap();
    assert_eq!((s.timer, s.action), (62, obj_makekinsuta::Action::Func80B98320));
    set(&mut w, state1, 0);
    update_once(&mut w);
    assert_eq!(w.actors.downcast::<ObjMakekinsuta>(h).unwrap().action, obj_makekinsuta::Action::DoNothing);
    assert_eq!(w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnSw>(h).is_some()).count(), 1);
}



