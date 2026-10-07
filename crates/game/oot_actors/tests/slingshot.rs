//! The Fairy Slingshot, first person and Deku nuts (GAME-05 milestone 5a): Player's side
//! against the C. The slingshot comes out and is raised in first person (`Player_UseItem`, the
//! change, `func_8083501C`, `func_80834F2C`, `func_80834D2C`, `func_8083442C`,
//! `func_80834EB8`, `Player_ActionHandler_13`), the string drawn and let go (`func_808351D4`,
//! `func_808350A4`, `func_808353D8`, `func_80835588`, `func_8084FF7C`), the look with C-Up
//! (`func_8083B8F4`, `Player_Action_8084B1D8`, `func_8084ABD8`), aiming Z-targeted
//! (`CAM_MODE_Z_AIM`), a Deku nut thrown (`func_8083C61C`, `Player_Action_8084E604`), Start's
//! stand-in putting them on C buttons, and the first-person draw
//! (`Player_OverrideLimbDrawGameplayFirstPerson`, the string).
//!
//! Expected values are worked out from the C in the comments. The tables they read:
//! - `gPlayerModelTypes`: `PLAYER_MODELGROUP_BOW_SLINGSHOT` is `PLAYER_ANIMTYPE_4`;
//! - `sItemChangeTypes[0][4]` is `PLAYER_ITEM_CHG_8`: `gPlayerAnim_link_normal_free2free`
//!   forwards, changing on frame 12 (`sItemChangeInfo[8]`).
//!
//! Without `En_Arrow` registered (it's its own test, `--test arrows`), the held seed is whatever
//! `Actor_SpawnAsChild` gives for `ACTOR_EN_ARROW`.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, BTN_CDOWN, BTN_CRIGHT, BTN_CUP, BTN_START, BTN_Z, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::player::{ARROW_NUT, ARROW_SEED, Action, STATE1_3, STATE1_9, STATE1_20, UpperAction};
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ActorImpl};
use oot_game::audio::sfx::*;
use oot_game::camera::{CAM_ID_MAIN, CAM_MODE_AIM_CHILD, CAM_MODE_FIRST_PERSON, CAM_MODE_NORMAL, CAM_MODE_Z_AIM};
use oot_game::item::{ITEM_DEKU_NUT, ITEM_DEKU_NUTS_10, ITEM_DEKU_STICK, ITEM_NONE, ITEM_SLINGSHOT, item_give};
use oot_game::play::{DrawOut, PlayState, ViewInfo, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

/// `NA_SE_IT_SLING_DRAW`, `NA_SE_IT_SLING_FLICK` (`itembank_table.h`: 0x1821, 0x1835).
const NA_SE_IT_SLING_DRAW: u16 = 0x1821;
const NA_SE_IT_SLING_FLICK: u16 = 0x1835;
/// `ACTOR_EN_ARROW` (`actor_table.h`).
const ACTOR_EN_ARROW: i16 = 0x0016;
/// The slingshot's start in room 1, 250 in front of its eye switch, facing it.
const ROOM1_START: (Vec3, i16) = (Vec3::new(-743.0, 400.0, 741.0), -0x2000);

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn frame(w: &mut PlayState, prev: PadState, cur: PadState) -> PadState {
    w.tick_with(scripted_input(prev, cur));
    cur
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        frame(w, PadState::default(), PadState::default());
    }
}

fn press(button: u16) -> PadState {
    PadState { button, ..Default::default() }
}

fn sfx_on(w: &PlayState, frame: u32, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == frame && s == id)
}

/// Inside the Deku Tree with `preset` (the sound log on), in `room` at `pos` facing `yaw`, the
/// room's enemies gone, and Link standing with the main camera.
fn deku_tree(a: &Arc<GameAssets>, preset: &str, room: i8, pos: Vec3, yaw: i16) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset(preset).unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    oot_actors::playthrough::deku_tree_room_start(&mut w, room, pos, yaw);
    idle(&mut w, 20);
    for h in w.actors.category(ACTORCAT_ENEMY).to_vec() {
        if let Some(a) = w.actors.actor_mut(h) {
            a.kill();
        }
    }
    // Their turn in Actor_UpdateAll deletes them.
    idle(&mut w, 1);
    for _ in 0..600 {
        let p = w.player();
        if w.active_cam_id == CAM_ID_MAIN && p.action == Action::StandingStill && p.grounded() {
            return w;
        }
        idle(&mut w, 1);
    }
    panic!("never settled");
}

fn room1(a: &Arc<GameAssets>) -> PlayState {
    deku_tree(a, "deku-tree-slingshot", 1, ROOM1_START.0, ROOM1_START.1)
}

/// C-Right held from standing until the slingshot is drawn with a seed in hand (the frames it
/// took), the button still held.
fn draw_slingshot(w: &mut PlayState) -> (PadState, usize) {
    let mut prev = PadState::default();
    for n in 1..60 {
        prev = frame(w, prev, press(BTN_CRIGHT));
        if w.player().held_actor.is_some() && w.player().unk_836 > 0 {
            return (prev, n);
        }
    }
    panic!("never drawn");
}

#[test]
fn the_slingshot_comes_out_raised_then_draws_a_seed() {
    let Some(a) = assets() else { return };
    let mut w = room1(&a);
    // The preset: Item_Give(ITEM_SLINGSHOT) (the bullet bag, 30 seeds), on C-Right.
    assert_eq!((w.save.equips.button_items[3], w.save.ammo(ITEM_SLINGSHOT)), (ITEM_SLINGSHOT, 30));
    let free2free = w.data.anim("link_normal_free2free");
    // C-Right: Player_GetItemOnButton(3); Player_UseItem: a new item, nextModelGroup
    // BOW_SLINGSHOT (anim type 4); sItemChangeTypes[0][4] is PLAYER_ITEM_CHG_8, not 0: the change
    // (heldItemId the slingshot), started at once: link_normal_free2free forwards at 1.2, doubled
    // for an item coming out, 3.6 a frame.
    let mut prev = frame(&mut w, PadState::default(), press(BTN_CRIGHT));
    let p = w.player();
    assert_eq!((p.held_item_id, p.held_item_button, p.upper, p.skel2.animation, p.skel2.play_speed), (ITEM_SLINGSHOT, 3, UpperAction::Change, free2free, 2.4));
    // The swap on frame 12 (sItemChangeInfo[8]): 3.6, 7.2, 10.8, then 14.4, the 4th frame.
    for _ in 0..3 {
        prev = frame(&mut w, prev, press(BTN_CRIGHT));
    }
    let swap = w.audio.frames;
    let ap = w.data.items.ap("SLINGSHOT");
    let p = w.player();
    // Player_InitBowOrSlingshotIA: PLAYER_STATE1_3, unk_860 -2 (not the bow's -1); model group
    // BOW_SLINGSHOT; NA_SE_PL_CHANGE_ARMS (func_8008F2BC: not a sword; nothing in hand went).
    assert_eq!((p.held_item_ap, p.state1 & STATE1_3, p.unk_860, p.model_group), (ap, STATE1_3, -2, w.data.items.model_group("BOW_SLINGSHOT")));
    assert!(sfx_on(&w, swap, NA_SE_PL_CHANGE_ARMS));
    // The next frame Player_UpperAction_ChangeHeldItem sees it in hand, and anim type 4 isn't 3:
    // sUseHeldItem, the change over, func_8083501C at once (unk_860 -2 stays negative), and
    // func_80834F2C raises it: func_80834D2C, func_8083442C (func_808351D4, PLAYER_STATE1_9,
    // unk_834 14; unk_860 negative: no sound and no seed, the first raise is dry), then
    // link_bow_bow_ready; func_80834EB8: not Z-targeting and the setting has the aim: unk_6AD 2.
    // The upper body returns false and Player_ActionHandler_0 runs Player_ActionHandler_13:
    // func_8083AD4C asks for CAM_MODE_AIM_CHILD, Player_Action_8084B1D8 (actionVar2 13),
    // PLAYER_STATE1_20, NA_SE_SY_CAMERA_ZOOM_UP.
    prev = frame(&mut w, prev, press(BTN_CRIGHT));
    let raise = w.audio.frames;
    let p = w.player();
    assert_eq!((p.upper, p.state1 & (STATE1_9 | STATE1_20), p.unk_834, p.unk_6AD, p.action), (UpperAction::BowDrawn, STATE1_9 | STATE1_20, 14, 2, Action::FirstPerson));
    assert_eq!((p.held_actor, p.skel2.animation), (None, w.data.anim("link_bow_bow_ready")));
    assert!(sfx_on(&w, raise, NA_SE_SY_CAMERA_ZOOM_UP) && !sfx_on(&w, raise, NA_SE_IT_SLING_DRAW));
    assert_eq!(w.game_camera.mode, CAM_MODE_AIM_CHILD);
    // func_808351D4: link_bow_bow_ready at 1.5 a frame (LinkAnimation_Once returns true the frame
    // after it reaches its end); then the wait loop (unk_836 1), and with unk_860 negative the
    // "shot" at once (func_808353D8, unk_834 10, nothing let go, no sound). unk_834 steps down to
    // 10 meanwhile.
    let last = w.data.anims[w.data.anim("link_bow_bow_ready")].last_frame();
    let ready = (last / 1.5).ceil() as u32 + 1;
    let mut n = 0;
    while w.player().upper == UpperAction::BowDrawn {
        prev = frame(&mut w, prev, press(BTN_CRIGHT));
        n += 1;
        assert!(n < 30);
    }
    let p = w.player();
    assert_eq!((n, p.upper, p.unk_834, p.unk_860, p.held_actor), (ready, UpperAction::BowShot, 10, -2, None));
    assert!(!sfx_on(&w, w.audio.frames, NA_SE_IT_SLING_FLICK));
    // The button still held, func_808353D8: unk_860 made positive and func_8083442C draws again:
    // NA_SE_IT_SLING_DRAW (D_80854398[1]), a seed (En_Arrow, ARROW_SEED) spawned as Player's
    // child at his feet facing his way, held (heldActor); link_bow_bow_shoot_next.
    frame(&mut w, prev, press(BTN_CRIGHT));
    let p = w.player();
    assert_eq!((p.upper, p.unk_834, p.unk_860, p.skel2.animation), (UpperAction::BowDrawn, 14, 2, w.data.anim("link_bow_bow_shoot_next")));
    assert!(sfx_on(&w, w.audio.frames, NA_SE_IT_SLING_DRAW));
    let h = p.held_actor.expect("a seed in hand");
    assert_eq!(p.actor.child, Some(h));
    let seed = w.actors.actor(h).unwrap();
    assert_eq!((seed.id, seed.params, seed.parent), (ACTOR_EN_ARROW, ARROW_SEED, w.player));
    // Player_PostLimbDrawGameplay: drawn back, the seed sits at D_80126128 from the left hand
    // (the seed's position this frame's draw gave it), by his head, the slingshot raised.
    let link = w.player().actor.world_pos;
    let seed = w.actors.actor(h).unwrap().world_pos;
    assert!((seed - link).length() < 50.0 && seed.y > link.y + 30.0, "seed {seed} Link {link}");
}

#[test]
fn letting_go_shoots_and_the_string_springs() {
    let Some(a) = assets() else { return };
    let mut w = room1(&a);
    let (prev, _) = draw_slingshot(&mut w);
    // Held in the wait (unk_836 2): unk_834 down to 10, so Player_PostLimbDrawGameplay draws the
    // string back: unk_858 its stretch (6.5 from the hand at Link's scale: (6.5 - 3) * 1.6, at most
    // 1), unk_85C -0.5.
    let mut prev = frame(&mut w, prev, press(BTN_CRIGHT));
    for _ in 0..8 {
        prev = frame(&mut w, prev, press(BTN_CRIGHT));
    }
    let p = w.player();
    assert_eq!((p.unk_836, p.unk_834, p.unk_858, p.unk_85c), (2, 10, 1.0, -0.5));
    let h = p.held_actor.unwrap();
    // C-Right let go: func_808351D4's shot (sHeldItemButtonIsHeldDown off): func_808353D8,
    // func_808350A4: one seed less (29), unk_A73 4, the seed's parent cleared, heldActor and
    // actor.child NULL; unk_834 10, Link stopped.
    frame(&mut w, prev, PadState::default());
    let p = w.player();
    assert_eq!((p.upper, p.unk_A73, p.held_actor, p.actor.child), (UpperAction::BowShot, 4, None, None));
    assert_eq!(w.save.ammo(ITEM_SLINGSHOT), 29);
    if let Some(seed) = w.actors.actor(h) {
        assert_eq!(seed.parent, None);
    }
    // func_8084FF7C (Player_UpdateCommon, PLAYER_STATE1_3) the frame after: the spring from the
    // drawn string (unk_858 1, unk_85C -0.5): 858 += 85C; 85C -= 858 * 5; 85C *= 0.3. (The shot
    // frame's draw no longer draws it back: PLAYER_STATE1_9 is off.)
    let (mut s, mut v) = (1.0f32, -0.5f32);
    s += v;
    v -= s * 5.0;
    v *= 0.3;
    let p = w.player();
    assert_eq!((p.unk_858, p.unk_85c), (s, v));
    idle(&mut w, 1);
    s += v;
    v -= s * 5.0;
    v *= 0.3;
    let p = w.player();
    assert_eq!((p.unk_858, p.unk_85c, p.unk_A73), (s, v, 3));
}

#[test]
fn with_no_seeds_the_string_flicks() {
    let Some(a) = assets() else { return };
    let mut w = room1(&a);
    w.save.inventory.ammo[oot_game::item::SLOT_SLINGSHOT] = 0;
    let mut prev = PadState::default();
    // As above, but func_80834380 counts no seed: func_8083442C draws the string (the sound) with
    // nothing in hand; let go, func_808350A4 has nothing to shoot: NA_SE_IT_SLING_FLICK
    // (D_808543DC[1]).
    let mut drawn = false;
    for _ in 0..40 {
        prev = frame(&mut w, prev, press(BTN_CRIGHT));
        drawn |= sfx_on(&w, w.audio.frames, NA_SE_IT_SLING_DRAW);
        assert_eq!(w.player().held_actor, None);
    }
    assert!(drawn);
    frame(&mut w, prev, PadState::default());
    assert_eq!(w.player().upper, UpperAction::BowShot);
    assert!(sfx_on(&w, w.audio.frames, NA_SE_IT_SLING_FLICK));
}

#[test]
fn a_ends_first_person_and_the_slingshot_lowers() {
    let Some(a) = assets() else { return };
    let mut w = room1(&a);
    let (prev, _) = draw_slingshot(&mut w);
    let prev = frame(&mut w, prev, PadState::default());
    // The shot; then C-Right again: func_808353D8 (sUseHeldItem): the next seed drawn.
    let mut prev = frame(&mut w, prev, press(BTN_CRIGHT));
    assert_eq!(w.player().upper, UpperAction::BowDrawn);
    assert!(w.player().held_actor.is_some());
    prev = frame(&mut w, prev, PadState::default());
    // A: Player_Action_8084B1D8 (unk_6AD 2, A pressed): func_8083C148: the head and body
    // straight (func_8083B010), standing (func_80839F90), unk_6AD 0, PLAYER_STATE1_20 off;
    // NA_SE_SY_CAMERA_ZOOM_UP. Player_UpdateCamAndSeqModes asks for CAM_MODE_NORMAL again.
    while w.player().upper == UpperAction::BowDrawn {
        prev = frame(&mut w, prev, PadState::default());
    }
    // (B would too, but Player_Action_8084B1D8 runs the item buttons first: B takes the sword out.)
    frame(&mut w, prev, press(BTN_A));
    let p = w.player();
    assert_eq!((p.unk_6AD, p.state1 & STATE1_20, p.action), (0, 0, Action::StandingStill));
    assert!(sfx_on(&w, w.audio.frames, NA_SE_SY_CAMERA_ZOOM_UP));
    idle(&mut w, 1);
    assert_eq!(w.game_camera.mode, CAM_MODE_NORMAL);
    // func_808353D8 counts unk_834 down (from 10); out of first person and not Z-targeting:
    // func_80835588 with link_bow_bow_shoot_end, then back to func_8083501C, unk_834 0.
    let mut n = 0;
    while w.player().upper != UpperAction::BowLower {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 20);
    }
    assert_eq!((w.player().unk_834, w.player().skel2.animation), (0, w.data.anim("link_bow_bow_shoot_end")));
    while w.player().upper == UpperAction::BowLower {
        idle(&mut w, 1);
    }
    assert_eq!(w.player().upper, UpperAction::Bow);
}

#[test]
fn c_up_looks_in_first_person() {
    let Some(a) = assets() else { return };
    let mut w = room1(&a);
    let yaw = w.player().actor.shape_rot.y;
    // C-Up with no Navi text (room 1 has no hint spot): Player_ActionHandler_0's func_8083B8F4:
    // on the ground, the setting has CAM_MODE_FIRST_PERSON: unk_6AD 1 (no error). The next frame
    // Player_ActionHandler_0 runs Player_ActionHandler_13: func_8083AD4C asks for the look's
    // camera, Player_Action_8084B1D8, NA_SE_SY_CAMERA_ZOOM_UP.
    let prev = frame(&mut w, PadState::default(), press(BTN_CUP));
    assert_eq!((w.player().unk_6AD, w.player().action), (1, Action::StandingStill));
    assert!(!sfx_on(&w, w.audio.frames, NA_SE_SY_ERROR));
    let prev = frame(&mut w, prev, PadState::default());
    let p = w.player();
    assert_eq!((p.unk_6AD, p.action, p.action_var2), (1, Action::FirstPerson, 13));
    assert!(sfx_on(&w, w.audio.frames, NA_SE_SY_CAMERA_ZOOM_UP));
    assert_eq!(w.game_camera.mode, CAM_MODE_FIRST_PERSON);
    // The stick right (rel x 60): the look (not aiming) turns the focus by 60 * -16 = -960 a frame
    // (func_8084ABD8), and the body follows (func_80836AB8: the head first, then the upper body;
    // the returned yaw is the body's).
    let right = PadState { stick_x: 67, ..Default::default() };
    let mut prev = frame(&mut w, prev, right);
    let f1 = w.player().actor.focus_rot.y;
    assert_eq!(f1, yaw.wrapping_sub(960));
    prev = frame(&mut w, prev, right);
    assert_eq!(w.player().actor.focus_rot.y, yaw.wrapping_sub(1920));
    // C-Up again ends the look (any C button in the look): func_8083C148.
    prev = frame(&mut w, prev, PadState::default());
    frame(&mut w, prev, press(BTN_CUP));
    let p = w.player();
    assert_eq!((p.unk_6AD, p.action), (0, Action::StandingStill));
}

#[test]
fn z_targeted_the_slingshot_aims_in_third_person() {
    // Child Link on the sandbox's course (nothing to lock on to), the slingshot on C-Right.
    let Some(d) = data() else { return };
    let c = oot_game::course::build();
    let mut w = new_world(d, eng_collision::bgcheck::CollisionContext::new(c.collision), false, c.spawn, c.spawn_yaw);
    item_give(&mut w.save, None, ITEM_SLINGSHOT);
    w.save.equip_item_on_c(2, ITEM_SLINGSHOT);
    // Z: parallel (Player_SetParallel); then C-Right: the slingshot out and raised;
    // func_80834EB8 while Z-targeting stays in third person (unk_6AD 0, no first person), and
    // Player_UpdateCamAndSeqModes in parallel with the item raised (func_8002DD78):
    // CAM_MODE_Z_AIM.
    let mut prev = frame(&mut w, PadState::default(), press(BTN_Z));
    prev = frame(&mut w, prev, press(BTN_Z));
    assert!(w.player().state1 & oot_actors::player::STATE1_17 != 0);
    let mut raised = false;
    for _ in 0..12 {
        prev = frame(&mut w, prev, press(BTN_Z | BTN_CRIGHT));
        let p = w.player();
        assert_eq!((p.unk_6AD, p.state1 & STATE1_20), (0, 0));
        raised |= p.unk_834 != 0;
    }
    assert!(raised);
    let p = w.player();
    assert_eq!(p.update_cam_and_seq_modes().map(|m| m.0), Some(CAM_MODE_Z_AIM));
    assert_eq!(w.game_camera.mode, CAM_MODE_Z_AIM);
}

#[test]
fn a_deku_nut_is_thrown() {
    let Some(a) = assets() else { return };
    let mut w = room1(&a);
    assert_eq!((w.save.equips.button_items[2], w.save.ammo(ITEM_DEKU_NUT)), (ITEM_DEKU_NUT, 10));
    let bom = w.data.anim("link_normal_light_bom");
    // C-Down: Player_UseItem(ITEM_DEKU_NUT), nuts left: func_8083C61C (not indoors, on the
    // ground): Player_Action_8084E604, link_normal_light_bom, unk_6AD 0.
    let mut prev = frame(&mut w, PadState::default(), press(BTN_CDOWN));
    let p = w.player();
    assert_eq!((p.action, p.skel.animation), (Action::ThrowNut, bom));
    // LinkAnimation_OnFrame(3): one nut less, En_Arrow ARROW_NUT at the right hand
    // (bodyPartsPos[PLAYER_BODYPART_R_HAND]) pitched 4000 along Link's facing, the voice
    // (NA_SE_VO_LI_SWORD_N + the child's unk_92).
    let yaw = p.actor.shape_rot.y;
    let before: Vec<_> = w.actors.category(oot_game::actor_ctx::ACTORCAT_ITEMACTION).to_vec();
    let mut thrown = None;
    for _ in 0..4 {
        prev = frame(&mut w, prev, PadState::default());
        if w.save.ammo(ITEM_DEKU_NUT) == 9 {
            thrown = Some(w.audio.frames);
            break;
        }
    }
    let thrown = thrown.expect("the nut thrown");
    let cur = w.player().skel.cur_frame;
    assert!((3.0..4.5).contains(&cur), "thrown on frame 3: {cur}");
    let voice = NA_SE_VO_LI_SWORD_N.wrapping_add(w.player().age.climb.unk_92);
    assert!(sfx_on(&w, thrown, voice));
    let nut = w.actors.category(oot_game::actor_ctx::ACTORCAT_ITEMACTION).iter().copied().find(|h| !before.contains(h)).expect("En_Arrow");
    let nut = w.actors.actor(nut).unwrap();
    assert_eq!((nut.id, nut.params, nut.home_rot.x, nut.home_rot.y), (ACTOR_EN_ARROW, ARROW_NUT, 4000, yaw));
    // At the animation's end, link_normal_light_bom_end back to standing (func_8083A098).
    let mut n = 0;
    while w.player().action == Action::ThrowNut {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 60);
    }
    assert_eq!(w.player().skel.animation, w.data.anim("link_normal_light_bom_end"));
}

#[test]
fn start_puts_nuts_and_the_slingshot_on_empty_c_buttons() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, "deku-tree-sticks", 1, ROOM1_START.0, ROOM1_START.1);
    // Picked up (Item_Give): nuts and the slingshot owned, on no button.
    item_give(&mut w.save, None, ITEM_DEKU_NUTS_10);
    item_give(&mut w.save, None, ITEM_SLINGSHOT);
    assert_eq!(&w.save.equips.button_items[1..], &[ITEM_DEKU_STICK, ITEM_NONE, ITEM_NONE]);
    // Start (the pause menu's stand-in): each on the first empty C button, as
    // KaleidoScope_UpdateItemEquip equips them.
    frame(&mut w, PadState::default(), press(BTN_START));
    assert_eq!(&w.save.equips.button_items[1..], &[ITEM_DEKU_STICK, ITEM_DEKU_NUT, ITEM_SLINGSHOT]);
    let s = &w.save.equips.c_button_slots;
    assert_eq!(*s, [oot_game::item::SLOT_DEKU_STICK as u8, oot_game::item::SLOT_DEKU_NUT as u8, oot_game::item::SLOT_SLINGSHOT as u8]);
    // KaleidoScope_UpdateItemEquip's swap: the slingshot onto C-Left (already on C-Right)
    // swaps the sticks there.
    w.save.equip_item_on_c(0, ITEM_SLINGSHOT);
    assert_eq!(&w.save.equips.button_items[1..], &[ITEM_SLINGSHOT, ITEM_DEKU_NUT, ITEM_DEKU_STICK]);
}

#[test]
fn first_person_draws_the_arms_and_the_string() {
    let Some(a) = assets() else { return };
    let mut w = room1(&a);
    let (_, _) = draw_slingshot(&mut w);
    let p = w.player();
    let draw = |w: &PlayState| {
        let p = w.player();
        let rs = p.render_state();
        let mut out = DrawOut::default();
        p.draw(&rs, w, &ViewInfo::new(Vec3::ZERO, glam::Mat4::IDENTITY), &mut out);
        out
    };
    assert_eq!(p.unk_6AD, 2);
    // Player_Draw: unk_6AD set and the head behind the view (the aim's camera is at it):
    // Player_OverrideLimbDrawGameplayFirstPerson: the child's arms only (sFirstPerson*DLs: the
    // right shoulder and gLinkChildRightArmStretchedSlingshotDL); the slingshot's string from the
    // right hand (gLinkChildSlingshotStringDL) in the XLU list.
    let out = draw(&w);
    let link = out.opa.iter().find(|c| c.mesh.name.starts_with("player/")).expect("Link's mesh");
    assert!(link.mesh.name.contains("gLinkChildRightArmStretchedSlingshotDL") && link.mesh.name.contains("gLinkChildRightShoulderNearDL"), "{}", link.mesh.name);
    assert!(!link.mesh.name.contains("gLinkChildLeftHand"), "{}", link.mesh.name);
    assert!(out.xlu.iter().any(|c| c.mesh.name.ends_with("gLinkChildSlingshotStringDL")));
    // The variant is in the pack.
    assert!(a.pack.link_variant_names(oot_game::player_lib::Age::Child).iter().any(|n| n == &link.mesh.name), "{} not baked", link.mesh.name);
}




#[test]
fn the_aim_is_camera_subj3_behind_links_focus() {
    let Some(a) = assets() else { return };
    let mut w = room1(&a);
    let (mut prev, _) = draw_slingshot(&mut w);
    // CAM_MODE_AIM_CHILD on the room's setting runs Camera_Subj3
    // (sSetNormal0ModeAimChildData: CAM_FUNCDATA_SUBJ3(-7, 14, 50, 10, -9, -63, -30, 45, ...)).
    let (setting, mode) = (w.game_camera.setting, w.game_camera.mode);
    assert_eq!(mode, CAM_MODE_AIM_CHILD);
    let m = w.data.camera.mode(setting, mode).expect("the aim's data").clone();
    assert_eq!(m.func, "CAM_FUNC_SUBJ3");
    let v = |i: usize| m.values[i] as f32;
    // Past the ease in (CAM_DEFAULT_ANIM_TIME frames): each frame, at the data's at offset
    // (x0.1) turned by the focus's pitch (-rot.x about x) then yaw (rot.y - 0x7FFF about y) from
    // Player's focus (Actor_GetFocus); eyeNext 50 behind it along the focus, the eye 14.
    for _ in 0..30 {
        prev = frame(&mut w, prev, press(BTN_CRIGHT));
    }
    let p = w.player();
    let (fp, fr) = (p.actor.focus_pos, p.actor.focus_rot);
    let (off_x, off_y, off_z) = (v(4) * 0.1, v(5) * 0.1, v(6) * 0.1);
    let (s, c) = (eng_math::sin_s(fr.x.wrapping_neg()), eng_math::cos_s(fr.x.wrapping_neg()));
    let sp98 = Vec3::new(off_x, off_y * c - off_z * s, off_y * s + off_z * c);
    let yaw = fr.y.wrapping_sub(0x7FFF);
    let (s, c) = (eng_math::sin_s(yaw), eng_math::cos_s(yaw));
    let at = Vec3::new(sp98.z * s + sp98.x * c, sp98.y, sp98.z * c - sp98.x * s) + fp;
    let geo = |r: f32| oot_game::camera::sph_geo_add(at, oot_game::camera::VecSphGeo { r, yaw, pitch: fr.x });
    let cam = &w.game_camera;
    assert!((cam.at - at).length() < 0.01, "at {} want {at}", cam.at);
    assert!((cam.eye_next - geo(v(2))).length() < 0.01, "eyeNext {} want {}", cam.eye_next, geo(v(2)));
    assert!((cam.eye - geo(v(1))).length() < 0.01, "eye {} want {}", cam.eye, geo(v(1)));
    // The fov stepped to the data's 45 (Camera_LERPCeilF by 0.25, within 1 at once).
    assert_eq!(cam.fov, v(7));
}



#[test]
fn room_10s_chest_gives_the_slingshot() {
    use oot_actors::en_box::EnBox;
    use oot_actors::playthrough::ROOM10_SLINGSHOT_CHEST_HOME;
    let Some(a) = assets() else { return };
    // Room 10 with only Deku Sticks (no slingshot yet): its enemies killed, the last one sets the
    // room's temporary clear, and En_Box 0x10A6 (type 1, ENBOX_TYPE_ROOM_CLEAR_BIG) falls into
    // place with its camera (EnBox_FallOnSwitchFlag's clear branch).
    let mut w = deku_tree(&a, "deku-tree-sticks", 10, Vec3::new(-1082.0, 820.0, 300.0), 0);
    assert_eq!(w.save.inv_content(ITEM_SLINGSHOT), ITEM_NONE);
    assert!(w.flags.get_temp_clear(10));
    let chest = |w: &PlayState| w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnBox>(h).is_some_and(|b| b.actor.home_pos.distance(ROOM10_SLINGSHOT_CHEST_HOME) < 1.0)).expect("the slingshot's chest");
    let h = chest(&w);
    for _ in 0..300 {
        if w.active_cam_id == CAM_ID_MAIN && w.player().action == Action::StandingStill {
            break;
        }
        idle(&mut w, 1);
    }
    // Its params: GI_SLINGSHOT ((0x10A6 >> 5) & 0x7F = 5), chest flag 6. In front of it (the side
    // away from its facing), A: Player_ActionHandler_2 opens it (the get-item id negative), then
    // the item held up with its text; A through it.
    let home = ROOM10_SLINGSHOT_CHEST_HOME;
    let mut prev = PadState::default();
    let mut opened = false;
    for _ in 0..900 {
        let p = w.player();
        let pad = if w.message_state() != oot_game::message::TEXT_STATE_NONE {
            if prev.button & eng_input::pad::BTN_A == 0 { press(eng_input::pad::BTN_A) } else { PadState::default() }
        } else if p.action == Action::GetItem {
            opened = true;
            PadState::default()
        } else if opened && p.action == Action::StandingStill {
            break;
        } else if p.interact_range_actor == Some(h) && p.get_item_id < 0 {
            if prev.button & eng_input::pad::BTN_A == 0 { press(eng_input::pad::BTN_A) } else { PadState::default() }
        } else {
            oot_actors::script::stick_towards(&w, home, 30.0)
        };
        prev = frame(&mut w, prev, pad);
    }
    assert!(opened, "the chest never opened");
    // Item_Give(ITEM_SLINGSHOT): the slingshot in its slot, the bullet bag (UPG_BULLET_BAG 1),
    // 30 seeds; the chest's flag 6 (Flags_SetTreasure).
    assert_eq!((w.save.inv_content(ITEM_SLINGSHOT), w.save.ammo(ITEM_SLINGSHOT)), (ITEM_SLINGSHOT, 30));
    assert!(w.flags.get_treasure(6));
    // Nothing puts it on a button but the pause menu: Start's stand-in, on C-Down (C-Left has the
    // sticks).
    assert!(!w.save.equips.button_items[1..].contains(&ITEM_SLINGSHOT));
    frame(&mut w, PadState::default(), press(BTN_START));
    assert_eq!(&w.save.equips.button_items[1..], &[ITEM_DEKU_STICK, ITEM_SLINGSHOT, ITEM_NONE]);
}

#[test]
fn the_room2_start_shoots_the_ladder() {
    use oot_actors::playthrough::{Playthrough, ROOM2_LADDER_FLAG, ROOM2_LADDER_HOME, SLINGSHOT_STARTS};
    let Some(a) = assets() else { return };
    // `game-slingshot.bat room2`: from there a seed reaches the ladder (Bg_Ydan_Maruta 0x0121) over
    // the lift, aimed 67 above its home, and drops it (flag 0x21).
    let (room, pos, yaw, _) = SLINGSHOT_STARTS[2];
    let mut w = deku_tree(&a, "deku-tree-slingshot", room, pos, yaw);
    let mut run = Playthrough::slingshot_shot(ROOM2_LADDER_HOME + Vec3::Y * 67.0, ROOM2_LADDER_FLAG);
    let mut prev = PadState::default();
    while let Some(p) = run.next(&w) {
        prev = frame(&mut w, prev, p);
    }
    assert!(run.failure.is_none(), "{:?}", run.failure);
    assert!(w.flags.get_switch(ROOM2_LADDER_FLAG));
}
