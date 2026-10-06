//! The Deku Stick (GAME-05 milestone 4b) against the C: taken out from C-Left
//! (`Player_ProcessItemButtons`, `Player_UseItem`, the change animation), lit at a torch and
//! burning down (`Player_UpdateBurningDekuStick`), breaking on a hit (`func_80842AC4`), swung
//! (`func_80837818`), drawn in the left hand (`Player_PostLimbDrawGameplay`), the pause menu's
//! stand-in putting sticks on C-Left, in water (`Player_GetEnvironmentalHazard` disabling the
//! buttons), and in the Master Quest Deku Tree: room 4's two timed
//! torches opening its door, room 10's timed torch dropping its chest.
//!
//! Expected values are worked out from the C in the comments. The tables they read:
//! - `gPlayerModelTypes`: `PLAYER_MODELGROUP_DEFAULT` is `PLAYER_ANIMTYPE_0`,
//!   `PLAYER_MODELGROUP_10` (the stick: closed hands, `SHEATH_18`) `PLAYER_ANIMTYPE_3`;
//! - `sItemChangeTypes[0][3]` is `-PLAYER_ITEM_CHG_6`, `[3][0]` `PLAYER_ITEM_CHG_6`;
//!   `sItemChangeInfo[6]` is `gPlayerAnim_link_hammer_long2free`, changing on frame 7.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, BTN_B, BTN_CLEFT, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::door_shutter::DoorShutter;
use oot_actors::en_box::{Action as BoxAction, EnBox};
use oot_actors::obj_syokudai::ObjSyokudai;
use oot_actors::player::{PLAYER_IA_DEKU_STICK, STATE1_8, STATE1_27, UpperAction};
use oot_actors::playthrough::{Route, STICK_TORCH_HOME};
use oot_actors::script::stick_towards;
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::audio::sfx::*;
use oot_game::camera::CAM_ID_MAIN;
use oot_game::collision_check as cc;
use oot_game::effect::{EFFECT_SS_DUST, EFFECT_SS_STICK};
use oot_game::item::{ITEM_DEKU_STICK, ITEM_DEKU_STICKS_10, ITEM_NONE, SLOT_DEKU_STICK};
use oot_game::play::{DrawOut, PlayState, ViewInfo, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Runs a frame with `cur` held (`prev` the frame before's), returning it as the next `prev`.
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

/// Idle until the main camera is the active one and Link stands.
fn settle(w: &mut PlayState) {
    for _ in 0..600 {
        let p = w.player();
        if w.active_cam_id == CAM_ID_MAIN && p.action == oot_actors::player::Action::StandingStill && p.grounded() {
            return;
        }
        idle(w, 1);
    }
    panic!("never settled");
}

/// Inside the Deku Tree with `preset` (the sound log on), at `Route::Stick`'s debug start on room
/// 0's middle floor by its golden torch (flag 0x27 set: the torches lit), Navi's hints there heard
/// (`Elf_Msg` 0x1F02's flag 0x1F); then in `room` if it isn't 0, its enemies gone (their attacks
/// would get in the way; the room then clears); Link at `at` if given.
fn deku_tree(a: &Arc<GameAssets>, preset: &str, room: i8, at: Option<(Vec3, i16)>) -> PlayState {
    let route = Route::Stick;
    let e = a.scenes.entrance_index(route.entrance()).expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset(preset).unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    route.debug_start(&mut w);
    w.flags.set_switch(0x1F);
    if room != 0 {
        assert!(w.room_request(room));
        idle(&mut w, 1);
        w.room_change_done();
        idle(&mut w, 20);
        for h in w.actors.category(oot_game::actor_ctx::ACTORCAT_ENEMY).to_vec() {
            if let Some(a) = w.actors.actor_mut(h) {
                a.kill();
            }
        }
        idle(&mut w, 1);
        settle(&mut w);
    }
    if let Some((pos, yaw)) = at {
        let (y, _) = w.col.entity_raycast_down(pos + Vec3::Y * 50.0);
        w.place_player(Vec3::new(pos.x, y, pos.z), yaw);
    }
    settle(&mut w);
    w
}

fn sfx_on(w: &PlayState, frame: u32, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == frame && s == id)
}

/// C-Left until the stick is out and the change over.
fn take_stick(w: &mut PlayState) {
    let mut prev = frame(w, PadState::default(), press(BTN_CLEFT));
    for _ in 0..60 {
        let p = w.player();
        if p.held_item_ap == PLAYER_IA_DEKU_STICK && p.upper != UpperAction::Change {
            return;
        }
        prev = frame(w, prev, PadState::default());
    }
    panic!("the stick never came out");
}

/// The pad that steers Link so the stick's tip comes to `to`: towards `to` less the tip's
/// offset from him (the stick is held out to his right), slowly; still once there.
fn tip_towards(w: &PlayState, to: Vec3) -> PadState {
    let p = w.player();
    let link = p.actor.world_pos;
    let at = to - (p.melee_weapon_info[0].tip - link);
    let d = Vec3::new(at.x - link.x, 0.0, at.z - link.z).length();
    if d < 3.0 { PadState::default() } else { stick_towards(w, at, if d < 15.0 { 30.0 } else { 40.0 }) }
}

/// The torch whose home is `home`.
fn torch_at(w: &PlayState, home: Vec3) -> ActorHandle {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<ObjSyokudai>(h).is_some_and(|t| t.actor.home_pos.distance(home) < 1.0)).expect("the torch")
}

fn lit_timer(w: &PlayState, home: Vec3) -> i16 {
    w.actors.downcast::<ObjSyokudai>(torch_at(w, home)).unwrap().lit_timer
}

/// Steers the stick's tip to the flame of the torch at `home` (67 above it: `ObjSyokudai_Update`'s
/// `tipToFlame`) until `done`.
fn tip_to_torch_until(w: &mut PlayState, home: Vec3, done: impl Fn(&PlayState) -> bool) {
    let mut prev = PadState::default();
    for _ in 0..400 {
        if done(w) {
            return;
        }
        let pad = tip_towards(w, home + Vec3::new(0.0, 67.0, 0.0));
        prev = frame(w, prev, pad);
    }
    panic!("the tip never reached the torch at {home} (tip {:?})", w.player().melee_weapon_info[0].tip);
}

#[test]
fn c_left_takes_a_stick_out_and_a_puts_it_away() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, "deku-tree-sticks", 0, None);
    // The preset: Item_Give(ITEM_DEKU_STICKS_10) (the capacity 10, 10 sticks), on C-Left as the
    // pause menu equips it.
    assert_eq!((w.save.equips.button_items[1], w.save.equips.c_button_slots[0], w.save.ammo(ITEM_DEKU_STICK)), (ITEM_DEKU_STICK, SLOT_DEKU_STICK as u8, 10));
    let long2free = w.data.anim("link_hammer_long2free");
    let last = w.data.anims[long2free].last_frame();
    // C-Left: Player_GetItemOnButton(1) is C_BTN_ITEM(0), the stick; Player_UseItem: in use
    // (PLAYER_IA_NONE both), not in water, sticks left, a new item: nextModelGroup 10 (anim type
    // 3), and with sItemChangeTypes[0][3] -6, not 0, the change starts (heldItemId the stick,
    // PLAYER_STATE1_START_CHANGING_HELD_ITEM); at once Player_StartChangingHeldItem:
    // link_hammer_long2free backwards from its last frame at -1.2, doubled for an item coming
    // out, and the upper action's first update (LinkAnimation_Update: playSpeed × R_UPDATE_RATE
    // × 0.5, 3.6 a frame).
    let prev = frame(&mut w, PadState::default(), press(BTN_CLEFT));
    let p = w.player();
    assert_eq!((p.held_item_id, p.held_item_button, p.held_item_ap, p.upper, p.state1 & STATE1_8), (ITEM_DEKU_STICK, 1, 0, UpperAction::Change, 0));
    assert_eq!((p.skel2.animation, p.skel2.play_speed), (long2free, -2.4));
    assert!((p.skel2.cur_frame - (last - 3.6)).abs() < 1e-4, "cur {} last {last}", p.skel2.cur_frame);
    // Player_WaitToFinishItemChange: the swap on frame 7 - 1 (played backwards).
    let mut prev = prev;
    let mut n = 1;
    while w.player().held_item_ap != PLAYER_IA_DEKU_STICK {
        prev = frame(&mut w, prev, PadState::default());
        n += 1;
        assert!(n < 10, "no swap");
    }
    let swap = w.audio.frames;
    let at = |k: i32| last - 3.6 * k as f32;
    assert!(at(n - 1) > 6.0 && at(n) <= 6.0, "swapped at the change's frame {}", at(n));
    // Player_FinishItemChange: nothing in hand went (no sound), Player_UseItem(heldItemId) put the
    // stick in hand (Player_InitItemActionWithAnim: Player_InitDekuStickIA's unk_85C 1, model
    // group 10, anim type 3), then NA_SE_PL_CHANGE_ARMS (func_8008F2BC: not a sword).
    let p = w.player();
    assert_eq!((p.item_ap, p.unk_85c, p.unk_860, p.model_group, p.model_anim_type), (PLAYER_IA_DEKU_STICK, 1.0, 0, w.data.items.model_group("10"), 3));
    assert!(sfx_on(&w, swap, NA_SE_PL_CHANGE_ARMS));
    assert!(!sfx_on(&w, swap, NA_SE_IT_SWORD_PUTAWAY) && !sfx_on(&w, swap, NA_SE_IT_SWORD_PICKOUT));
    // Anim type 3 takes no press to end the change: it plays down to frame 0, then the upper body
    // goes back to the item's (func_8083485C for the stick).
    while w.player().upper == UpperAction::Change {
        prev = frame(&mut w, prev, PadState::default());
    }
    // In the left hand: its tip tracked every frame, 50 along the hand (unk_85C × 5000, at Link's
    // 0.01), at the height of a torch's flame above Link's feet; the stick drawn
    // (gLinkChildLinkDekuStickDL).
    let p = w.player();
    let tip = p.melee_weapon_info[0].tip;
    assert!((tip.y - p.actor.world_pos.y - 66.0).abs() < 4.0, "tip {tip} Link {}", p.actor.world_pos);
    let rs = p.render_state();
    let mut out = DrawOut::default();
    p.draw(&rs, &w, &ViewInfo::new(Vec3::ZERO, glam::Mat4::IDENTITY), &mut out);
    assert!(out.opa.iter().any(|c| c.mesh.name.ends_with("gLinkChildLinkDekuStickDL")));
    // A while standing (Player_ActionHandler_Roll: no roll without the stick held, and the
    // put-away cooldown, 20 frames of A's "Put Away", run out): Player_UseItem(ITEM_NONE) from the
    // action's handlers, after the upper body ran: the change pending (heldItemId ITEM_NONE,
    // PLAYER_STATE1_START_CHANGING_HELD_ITEM); the next frame Player_UpdateItems starts it:
    // forwards (sItemChangeTypes[3][0] +6) at 1.2 (nothing comes out), the stick going on its
    // frame 7 with NA_SE_PL_CHANGE_ARMS, nothing in hand after.
    idle(&mut w, 25);
    let prev = frame(&mut w, PadState::default(), press(BTN_A));
    let p = w.player();
    assert_eq!((p.held_item_id, p.upper, p.state1 & STATE1_8), (ITEM_NONE, UpperAction::Default, STATE1_8));
    let mut prev = frame(&mut w, prev, PadState::default());
    let p = w.player();
    assert_eq!((p.upper, p.skel2.play_speed, p.state1 & STATE1_8), (UpperAction::Change, 1.2, 0));
    while w.player().held_item_ap != 0 {
        prev = frame(&mut w, prev, PadState::default());
    }
    let p = w.player();
    assert_eq!((p.item_ap, p.unk_85c, p.model_group), (0, 0.0, w.data.items.model_group("DEFAULT")));
    assert!(sfx_on(&w, w.audio.frames, NA_SE_PL_CHANGE_ARMS));
    assert_eq!(w.save.ammo(ITEM_DEKU_STICK), 10);
}

#[test]
fn with_no_sticks_left_c_left_errors() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, "deku-tree-sticks", 0, None);
    w.save.inventory.ammo[SLOT_DEKU_STICK] = 0;
    idle(&mut w, 1);
    // Player_UseItem: PLAYER_IA_DEKU_STICK with AMMO(ITEM_DEKU_STICK) 0: Sfx_PlaySfxCentered(NA_SE_SY_ERROR), nothing else.
    frame(&mut w, PadState::default(), press(BTN_CLEFT));
    let p = w.player();
    assert_eq!((p.held_item_id, p.held_item_ap, p.upper), (ITEM_NONE, 0, UpperAction::Default));
    assert!(sfx_on(&w, w.audio.frames, NA_SE_SY_ERROR));
}

#[test]
fn b_with_a_stick_out_takes_the_sword_out() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, "deku-tree-sticks", 0, None);
    take_stick(&mut w);
    // B's item is the Kokiri Sword: Player_UseItem with another item than the one in hand: the
    // change (sItemChangeTypes[3][1], the sword with the shield worn: -4, not 0) to it, the stick
    // going (NA_SE_PL_CHANGE_ARMS) and the sword coming out (NA_SE_IT_SWORD_PICKOUT) on the swap.
    let mut prev = frame(&mut w, PadState::default(), press(BTN_B));
    assert_eq!((w.player().held_item_id, w.player().upper), (oot_game::item::ITEM_SWORD_KOKIRI, UpperAction::Change));
    while w.player().held_item_ap == PLAYER_IA_DEKU_STICK {
        prev = frame(&mut w, prev, PadState::default());
    }
    let p = w.player();
    assert_eq!((p.held_item_ap, p.unk_85c), (w.data.items.ap("SWORD_KOKIRI"), 0.0));
    assert!(sfx_on(&w, w.audio.frames, NA_SE_PL_CHANGE_ARMS) && sfx_on(&w, w.audio.frames, NA_SE_IT_SWORD_PICKOUT));
    assert_eq!(w.save.ammo(ITEM_DEKU_STICK), 10);
}

#[test]
fn start_puts_owned_sticks_on_an_empty_c_left() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, "deku-tree-inside", 0, None);
    oot_game::item::item_give(&mut w.save, None, ITEM_DEKU_STICKS_10);
    assert_eq!(w.save.equips.button_items[1], ITEM_NONE);
    // The stand-in for the pause menu's item screen: C-Left gets the stick (its slot), as
    // KaleidoScope_UpdateItemEquip equips it.
    assert!(w.pause_menu_equip());
    assert_eq!((w.save.equips.button_items[1], w.save.equips.c_button_slots[0]), (ITEM_DEKU_STICK, SLOT_DEKU_STICK as u8));
    // Nothing more to do.
    assert!(!w.pause_menu_equip());
}

#[test]
fn a_stick_lit_at_the_torch_burns_down_in_210_frames() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, "deku-tree-sticks", 0, None);
    take_stick(&mut w);
    assert_eq!(lit_timer(&w, STICK_TORCH_HOME), -1);
    // The tip at the flame: the golden torch (lit) sets unk_860 210, NA_SE_EV_FLAME_IGNITION.
    tip_to_torch_until(&mut w, STICK_TORCH_HOME, |w| w.player().unk_860 != 0);
    assert_eq!(w.player().unk_860, 210);
    assert!(sfx_on(&w, w.audio.frames, NA_SE_EV_FLAME_IGNITION));
    // Away from the torch (which would set it back up to 200 under 200): Player_UpdateBurningDekuStick
    // each frame: DECR(unk_860) and the flame (func_8002836C: dust at the tip, temp × 200 big):
    // over 200, temp (210 - unk_860) / 10; down to 20, 1; under 20, unk_860 / 20, and the stick
    // as long (unk_85C).
    let mut prev = PadState::default();
    // Along the walkway (y 360 there), clear of the torch.
    let away = Vec3::new(400.0, 360.0, 0.0);
    for k in 1..=210i16 {
        let pad = if k < 40 { stick_towards(&w, away, 40.0) } else { PadState::default() };
        prev = frame(&mut w, prev, pad);
        let p = w.player();
        let v = 210 - k;
        // This frame's flame: the newest dust (the longest life left).
        let dust = w.effect_ss.table.iter().filter(|e| e.life >= 0 && e.ty == EFFECT_SS_DUST).max_by_key(|e| e.life).map(|e| e.regs[oot_game::effect::dust::R_SCALE]);
        let temp = if v == 0 {
            // At 0: one stick less, unk_860 1 and the stick gone (unk_85C 0), the flame 0 big.
            assert_eq!((p.unk_860, p.unk_85c, w.save.ammo(ITEM_DEKU_STICK)), (1, 0.0, 9), "k {k}");
            0.0
        } else {
            assert_eq!(p.unk_860, v, "k {k}");
            if v > 200 {
                (210 - v) as f32 / 10.0
            } else if v < 20 {
                v as f32 / 20.0
            } else {
                1.0
            }
        };
        assert_eq!(dust, Some((temp * 200.0) as i16), "k {k}");
        if v != 0 {
            assert_eq!(p.unk_85c, if v < 20 { temp } else { 1.0 }, "k {k}");
        }
    }
    // The next frame, unk_85C 0: Player_UseItem(ITEM_NONE) puts away what's left.
    frame(&mut w, prev, PadState::default());
    let p = w.player();
    assert_eq!((p.held_item_id, p.upper), (ITEM_NONE, UpperAction::Change));
}

#[test]
fn a_slash_with_the_stick_breaks_it_on_a_target() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, "deku-tree-sticks", 0, Some((Vec3::new(300.0, 360.0, 260.0), 0x2000)));
    take_stick(&mut w);
    let ahead = w.player().actor.world_pos + Vec3::new(30.0, 0.0, 30.0);
    w.spawn_target(ahead);
    idle(&mut w, 2);
    // C-Left again (B is the sword's: Player_UseItem would change to it): Player_UseItem on the
    // item in hand sets sUseHeldItem; Player_ActionHandler_7, func_80837818: the stick's always
    // PLAYER_MWA_FORWARD_SLASH_1H, one more for a two-handed weapon (Player_HoldsTwoHandedWeapon:
    // the Biggoron's Sword to the hammer, the stick between): FORWARD_SLASH_2H. func_80837948:
    // D_80854488[4 - 1][0], DMG_DEKU_STICK, with ATELEM_SFX_WOOD.
    let mut prev = frame(&mut w, PadState::default(), press(BTN_CLEFT));
    let p = w.player();
    assert_eq!(p.melee_weapon_animation, w.data.items.mwa("FORWARD_SLASH_2H"));
    for q in &p.melee_weapon_quads {
        assert_eq!((q.info.at_dmg_info.dmg_flags, q.info.at_elem_flags), (cc::DMG_DEKU_STICK, cc::ATELEM_ON | cc::ATELEM_NEAREST | cc::ATELEM_SFX_WOOD));
    }
    let yaw = p.actor.shape_rot.y;
    // Until it hits (AT_HIT): func_80842DF4 → func_80842AC4: a stick over half long with sticks
    // left breaks: EffectSsStick_Spawn at the right hand, backwards (yaw + 0x8000), half of it
    // left (unk_85C 0.5), one stick less, Player_UseItem(ITEM_NONE), NA_SE_IT_WOODSTICK_BROKEN.
    let mut hit = None;
    for _ in 0..30 {
        prev = frame(&mut w, prev, PadState::default());
        if w.save.ammo(ITEM_DEKU_STICK) == 9 {
            hit = Some(w.audio.frames);
            break;
        }
    }
    let hit = hit.expect("the stick never broke");
    assert!(sfx_on(&w, hit, NA_SE_IT_WOODSTICK_BROKEN));
    let p = w.player();
    assert_eq!((p.unk_85c, p.held_item_id), (0.5, ITEM_NONE));
    // EffectSsStick_Init: 20 frames, thrown 26 up and 6 along the yaw, falling at 4; spawned in
    // Player's update, EffectSs_UpdateAll (after the actors) has run it once: 19 left, 22 up.
    let e = w.effect_ss.table.iter().find(|e| e.life >= 0 && e.ty == EFFECT_SS_STICK).expect("the broken half");
    let back = yaw.wrapping_add(0x8000u16 as i16);
    assert_eq!((e.life, e.accel.y, e.velocity.y), (19, -4.0, 22.0));
    assert!((e.velocity.x - eng_math::sin_s(back) * 6.0).abs() < 1e-4 && (e.velocity.z - eng_math::cos_s(back) * 6.0).abs() < 1e-4);
    // The half left goes with the change; nothing in hand after.
    for _ in 0..30 {
        prev = frame(&mut w, prev, PadState::default());
    }
    assert_eq!(w.player().held_item_ap, 0);
}

#[test]
fn a_slash_with_the_stick_breaks_it_on_a_wall() {
    let Some(a) = assets() else { return };
    // On the middle floor facing its outer wall (480 from the room's middle).
    let mut w = deku_tree(&a, "deku-tree-sticks", 0, Some((Vec3::new(310.0, 360.0, 310.0), 0x2000)));
    take_stick(&mut w);
    let mut prev = frame(&mut w, PadState::default(), press(BTN_CLEFT));
    // func_80842DF4: from the swing's frame 2, the tip 10 beyond the stick meeting the wall:
    // sparks, func_80842CF0 → func_80842AC4: the stick breaks.
    for _ in 0..30 {
        prev = frame(&mut w, prev, PadState::default());
        if w.save.ammo(ITEM_DEKU_STICK) == 9 {
            break;
        }
    }
    assert_eq!(w.save.ammo(ITEM_DEKU_STICK), 9, "the stick never broke on the wall");
    assert!(sfx_on(&w, w.audio.frames, NA_SE_IT_WOODSTICK_BROKEN));
    assert!(w.effect_ss.table.iter().any(|e| e.life >= 0 && e.ty == EFFECT_SS_STICK));
}

#[test]
fn room_4s_timed_torches_lit_with_a_stick_open_its_door() {
    let Some(a) = assets() else { return };
    // Room 4, between its torches (0x1099: count 2, flag 0x19; the door to room 5, transition 8,
    // type 2 SHUTTER_FRONT_SWITCH on 0x19).
    let mut w = deku_tree(&a, "deku-tree-sticks", 4, Some((Vec3::new(-230.0, -880.0, 961.0), 0x4000)));
    let door =
        |w: &PlayState| w.actors.all().into_iter().find_map(|h| w.actors.downcast::<DoorShutter>(h).filter(|d| d.transition_index() == 8)).map(|d| (d.action, d.bars_closed_amount)).expect("the door");
    assert!(door(&w).1 > 0.0, "{:?}", door(&w));
    take_stick(&mut w);
    // No fire in room 4: the stick as if lit at room 3's golden torch (Obj_Syokudai's 210).
    w.player_mut().unk_860 = 210;
    let (first, second) = (Vec3::new(-281.0, -880.0, 881.0), Vec3::new(-282.0, -880.0, 1041.0));
    // The burning tip at the first (interactionType -1 with unk_860 set): lit for 50 × 2 + 110
    // (litTimeScale 2) less the frames since; the stick under 200 back up to 200.
    tip_to_torch_until(&mut w, first, |w| lit_timer(w, first) > 0);
    assert_eq!(w.player().unk_860, 200);
    // The second makes sLitTorchCount 2: flag 0x19, both kept lit (-1); the door unbars with its
    // attention camera.
    tip_to_torch_until(&mut w, second, |w| w.flags.get_switch(0x19));
    idle(&mut w, 1);
    assert_eq!((lit_timer(&w, first), lit_timer(&w, second)), (-1, -1));
    for _ in 0..300 {
        if door(&w).1 == 0.0 && w.active_cam_id == CAM_ID_MAIN {
            break;
        }
        idle(&mut w, 1);
    }
    assert_eq!(door(&w).1, 0.0, "{:?}", door(&w));
}

#[test]
fn room_10s_timed_torch_lit_from_its_wooden_one_drops_the_chest() {
    let Some(a) = assets() else { return };
    // Room 10: the wooden torch 0x2400 (always lit) at (-653, 800, -105), the timed one 0x1053
    // (count 1, flag 0x13) at (-1067, 720, -5), the Deku Shield's chest falling on 0x13.
    let wooden = Vec3::new(-653.0, 800.0, -105.0);
    let timed = Vec3::new(-1067.0, 720.0, -5.0);
    let mut w = deku_tree(&a, "deku-tree-sticks", 10, Some((Vec3::new(-700.0, 800.0, -60.0), -0x4000)));
    let falling_chest = |w: &PlayState| w.actors.all().into_iter().find_map(|h| w.actors.downcast::<EnBox>(h).filter(|b| b.switch_flag == 0x13)).map(|b| b.action);
    assert_eq!(falling_chest(&w), Some(BoxAction::FallOnSwitchFlag));
    take_stick(&mut w);
    // The wooden torch lights the stick: 210.
    tip_to_torch_until(&mut w, wooden, |w| w.player().unk_860 != 0);
    assert_eq!(w.player().unk_860, 210);
    // Down to the timed torch and into its flame: its count 1 makes it at once: flag 0x13 set,
    // and EnBox_FallOnSwitchFlag sees it.
    let mut prev = PadState::default();
    for _ in 0..200 {
        let link = w.player().actor.world_pos;
        if Vec3::new(link.x - timed.x, 0.0, link.z - timed.z).length() < 80.0 {
            break;
        }
        let pad = stick_towards(&w, timed, 60.0);
        prev = frame(&mut w, prev, pad);
    }
    tip_to_torch_until(&mut w, timed, |w| w.flags.get_switch(0x13));
    for _ in 0..60 {
        if falling_chest(&w) == Some(BoxAction::Fall) {
            break;
        }
        idle(&mut w, 1);
    }
    assert_eq!(falling_chest(&w), Some(BoxAction::Fall));
}

/// Kokiri Forest with ten Deku Sticks on C-Left (`deku-tree-sticks`), Link on the bank of its
/// water at (950, 0, -100) facing it (+x): the floor slopes down under the water (surface -12)
/// to -60 by x 1150, deep enough for child Link to swim (`unk_2C` 32).
fn kokiri_forest_bank(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_KOKIRI_FOREST_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-sticks").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 30);
    w.place_player(Vec3::new(950.0, 0.0, -100.0), 0x4000);
    settle(&mut w);
    w
}

/// B and the three C buttons all `BTN_DISABLED`.
fn buttons_disabled(w: &PlayState) -> bool {
    w.save.button_status[..4].iter().all(|&s| s == oot_game::interface::BTN_DISABLED)
}

/// Into the water at a run (+x) until Link swims (`PLAYER_STATE1_27`); the pad last held.
fn wade_in(w: &mut PlayState, prev: PadState) -> PadState {
    let mut prev = prev;
    for _ in 0..120 {
        if w.player().state1 & STATE1_27 != 0 {
            return prev;
        }
        let pad = stick_towards(w, Vec3::new(1300.0, -60.0, -100.0), 60.0);
        prev = frame(w, prev, pad);
    }
    panic!("never swam");
}

#[test]
fn wading_in_puts_the_lit_stick_out_and_the_buttons_are_disabled() {
    let Some(a) = assets() else { return };
    let mut w = kokiri_forest_bank(&a);
    take_stick(&mut w);
    // As if lit at a torch (210).
    w.player_mut().unk_860 = 210;
    assert!(!buttons_disabled(&w));
    let prev = wade_in(&mut w, PadState::default());
    // The frame Link starts swimming: Interface_Update (after the actors) finds
    // Player_GetEnvironmentalHazard PLAYER_ENV_HAZARD_SWIMMING (3), in the range
    // UNDERWATER_FLOOR (2) to UNDERWATER_FREE (4) func_80083108 disables B and the C buttons for,
    // and the HUD to HUD_VISIBILITY_ALL (50). The stick still burns.
    assert_eq!(w.interface_ctx.env_hazard, oot_game::play::PLAYER_ENV_HAZARD_SWIMMING);
    assert!(buttons_disabled(&w), "{:?}", w.save.button_status);
    assert_eq!(w.save.hud_visibility_mode, 50);
    let p = w.player();
    assert!(p.held_item_ap == PLAYER_IA_DEKU_STICK && p.unk_860 > 0);
    // The next frame: the swim's action runs its handlers with the upper body
    // (Player_TryActionHandlerList(..., true): Player_UpdateUpperBody), and no button has the stick:
    // Player_ProcessItemButtons puts it away (Player_UseItem(ITEM_NONE)), Player_InitItemAction zeroing
    // unk_860: it's out.
    frame(&mut w, prev, PadState::default());
    let p = w.player();
    assert!(p.state1 & STATE1_27 != 0);
    assert_eq!((p.held_item_ap, p.held_item_id, p.unk_860, p.unk_85c), (0, ITEM_NONE, 0, 0.0));
}

#[test]
fn the_sword_goes_away_in_the_water_and_the_buttons_come_back_on_land() {
    let Some(a) = assets() else { return };
    let mut w = kokiri_forest_bank(&a);
    // The Kokiri Sword out (B).
    let mut prev = frame(&mut w, PadState::default(), press(BTN_B));
    for _ in 0..30 {
        prev = frame(&mut w, prev, PadState::default());
    }
    assert_eq!(w.player().held_item_ap, w.data.items.ap("SWORD_KOKIRI"));
    // In the water B is disabled (B_BTN_ITEM ITEM_NONE): the sword in use is on no button and goes
    // away the same way.
    prev = wade_in(&mut w, prev);
    prev = frame(&mut w, prev, PadState::default());
    assert_eq!(w.player().held_item_ap, 0);
    // Back up the bank: on land Player_GetEnvironmentalHazard is PLAYER_ENV_HAZARD_NONE, and
    // func_80083108's scene restrictions enable the buttons again.
    for _ in 0..120 {
        if w.player().state1 & STATE1_27 == 0 && w.player().grounded() {
            break;
        }
        let pad = stick_towards(&w, Vec3::new(900.0, 0.0, -100.0), 60.0);
        prev = frame(&mut w, prev, pad);
    }
    frame(&mut w, prev, PadState::default());
    assert_eq!(w.interface_ctx.env_hazard, oot_game::play::PLAYER_ENV_HAZARD_NONE);
    assert!(!buttons_disabled(&w), "{:?}", w.save.button_status);
    assert_eq!(w.player().held_item_ap, 0);
}

#[test]
fn falling_into_deep_water_puts_the_lit_stick_out() {
    let Some(a) = assets() else { return };
    // Room 5's pool: its bank at -880 drops to a floor at -1020 under the water. Link runs off
    // the bank and plunges in; with the buttons disabled (swimming), his first action in the water
    // to run Player_UpdateUpperBody puts the stick away and zeroes its unk_860.
    let mut w = deku_tree(&a, "deku-tree-sticks", 5, Some((Vec3::new(-1197.0, -880.0, 1079.0), 0x4000)));
    take_stick(&mut w);
    w.player_mut().unk_860 = 200;
    let mut prev = PadState::default();
    let mut wet_at = None;
    for n in 0..120 {
        let p = w.player();
        if p.state1 & STATE1_27 != 0 && wet_at.is_none() {
            wet_at = Some(n);
        }
        if p.held_item_ap == 0 {
            break;
        }
        let pad = if wet_at.is_none() { stick_towards(&w, Vec3::new(-1000.0, -880.0, 800.0), 60.0) } else { PadState::default() };
        prev = frame(&mut w, prev, pad);
    }
    let p = w.player();
    assert!(wet_at.is_some(), "never in the water");
    assert!(p.state1 & STATE1_27 != 0, "out of the water again");
    assert_eq!((p.held_item_ap, p.unk_860), (0, 0));
}
