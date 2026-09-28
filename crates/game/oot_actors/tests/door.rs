//! `En_Door` and Player's side of it (`func_80839800`, `func_80845EF8`), in scenes from the
//! asset pack through `Play_Init`. Expected values come from `z_en_door.c`, `z_player.c` and
//! `z_camera.c`, and the scenes' transition-actor lists:
//! - `kakariko_scene` (a Kakariko house): transition 0, `En_Door` at (100, 0, 210), rotation 0,
//!   params 0x01BF (`DOOR_SCENEEXIT`), both sides room 0; exit 1 is `ENTR_SPOT01_6`;
//! - `souko_scene`: transition 2, `En_Door` at (1220, 140, 150), rotation 0xC000, params 0x003F
//!   (`DOOR_ROOMLOAD`), front room 2, back room 1, both bg cameras -1.

mod common;

use std::sync::Arc;

use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_door::{self, EnDoor};
use oot_actors::player::{Action, PLAYER_DOORTYPE_HANDLE, STATE1_29};
use oot_game::actor_ctx::ActorHandle;
use oot_game::camera::CAM_SET_DOORC;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::transition::TRANS_TRIGGER_START;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Play entering by `entrance`, child Link, 10:00.
fn enter(entrance: &str) -> Option<PlayState> {
    let a = assets()?;
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    Some(oot_actors::play_entrance(a, common::data()?, common::rules()?, save).expect("Play_Init"))
}

fn doors(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnDoor>(h).is_some()).collect()
}

fn door(w: &PlayState, h: ActorHandle) -> &EnDoor {
    w.actors.downcast::<EnDoor>(h).expect("the door")
}

fn frames(w: &mut PlayState, prev: &mut PadState, pad: PadState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(*prev, pad));
        *prev = pad;
    }
}

/// The scene settled (the fade in done), Link placed at `pos` facing `yaw`, one frame for the
/// door to see him.
fn at_door(entrance: &str, pos: Vec3, yaw: i16) -> Option<(PlayState, PadState)> {
    let mut w = enter(entrance)?;
    let mut prev = PadState::default();
    frames(&mut w, &mut prev, PadState::default(), 30);
    w.place_player(pos, yaw);
    frames(&mut w, &mut prev, PadState::default(), 1);
    Some((w, prev))
}

#[test]
fn scene_exit_door_opens_and_starts_the_exit() {
    let Some((mut w, mut prev)) = at_door("ENTR_KAKARIKO_0", Vec3::new(100.0, 0.0, 180.0), 0) else { return };
    let d = doors(&w)[0];
    let dr = door(&w, d);
    // Actor_SpawnTransitionActors: index 0 in the params' top bits; EnDoor_SetupType kept
    // DOOR_SCENEEXIT; a scene with gameplay_field_keep takes DOOR_DL_DEFAULT_FIELD_KEEP.
    assert_eq!((dr.transition_index(), dr.door_type(), dr.dlist_index), (0, en_door::DOOR_SCENEEXIT, 4));
    assert_eq!(dr.action, en_door::Action::Idle);
    // EnDoor_Idle: Player 30 in front (func_8002DBD0 z -30), facing it (yaw difference 0):
    // PLAYER_DOORTYPE_HANDLE, doorDirection -1 (z < 0), doorActor.
    let p = w.player();
    // (Player's update resets doorType at its end, after reading it; the door sets it again
    // later in the frame, in ACTORCAT_DOOR.)
    assert_eq!((p.door_type, p.door_direction, p.door_actor), (PLAYER_DOORTYPE_HANDLE, -1, Some(d)));
    frames(&mut w, &mut prev, PadState { button: BTN_A, ..Default::default() }, 1);
    // func_80839800: DOOR_OPEN_ANIM_CHILD_L (behind, a child) and PLAYER_ANIMGROUP_10
    // (gPlayerAnim_clink_demo_doorA_link); facing the door's yaw, 22 behind its centre
    // (-22 along its facing: z 188); PLAYER_STATE1_29.
    let p = w.player();
    assert_eq!(p.action, Action::DoorOpen);
    assert_eq!(w.data.anim_name(p.skel.animation), "clink_demo_doorA_link");
    assert_eq!(p.actor.shape_rot.y, 0);
    assert!((p.actor.world_pos.x - 100.0).abs() < 0.5 && (p.actor.world_pos.z - 188.0).abs() < 0.5, "{:?}", p.actor.world_pos);
    assert_ne!(p.state1 & STATE1_29, 0);
    // DOOR_SCENEEXIT: func_80839034 with the floor 22 past the door's far side (exit 1),
    // entranceSpeed 2.
    let a = w.assets.clone().unwrap();
    assert_eq!(w.transition.trigger, TRANS_TRIGGER_START);
    assert_eq!(w.transition.next_entrance_index, a.scenes.entrance_index("ENTR_SPOT01_6").unwrap());
    assert_eq!(w.save.entrance_speed, 2.0);
    let dr = door(&w, d);
    assert_eq!((dr.open_anim, dr.player_is_opening), (en_door::DOOR_OPEN_ANIM_CHILD_L, true));
    // Next frame the door: EnDoor_Open with sDoorAnims[openAnim] at play speed 1.5.
    frames(&mut w, &mut prev, PadState::default(), 1);
    let dr = door(&w, d);
    assert_eq!(dr.action, en_door::Action::Open);
    assert!(dr.skel.is("gDoorChildOpeningLeftAnim"));
    assert_eq!(dr.skel.play_speed, 1.5);
}

#[test]
fn room_door_loads_the_room_behind_with_the_door_camera() {
    let Some((mut w, mut prev)) = at_door("ENTR_SOUKO_2", Vec3::new(1190.0, 140.0, 150.0), 0x4000) else { return };
    assert_eq!(w.room_ctx.cur.num, 1);
    let d = doors(&w).into_iter().find(|&h| door(&w, h).transition_index() == 2).expect("transition 2");
    assert_eq!(door(&w, d).door_type(), en_door::DOOR_ROOMLOAD);
    // In front (z 30): doorDirection 1.
    assert_eq!(w.player().door_direction, 1);
    frames(&mut w, &mut prev, PadState { button: BTN_A, ..Default::default() }, 1);
    // DOOR_OPEN_ANIM_CHILD_R, PLAYER_ANIMGROUP_12 (gPlayerAnim_clink_demo_doorB_link); facing
    // the door's yaw - 0x8000, 22 in front of it (x 1198).
    let p = w.player();
    assert_eq!(w.data.anim_name(p.skel.animation), "clink_demo_doorB_link");
    assert_eq!(p.actor.shape_rot.y, 0x4000);
    assert!((p.actor.world_pos.x - 1198.0).abs() < 0.5, "{:?}", p.actor.world_pos);
    // Camera_ChangeDoorCam(camera, door, sides[0].bgCamIndex (-1), 0, 38, 26, 10):
    // CAM_SET_DOORC with the door's parameters; func_8009728C(sides[0].room 2); the door
    // belongs to the room being entered.
    let c = &w.game_camera;
    assert_eq!(c.setting, CAM_SET_DOORC);
    // (Camera_Update ran after Player's update this frame: Special9's case 1 took timer1 to 37.)
    let dp = c.door_params;
    assert_eq!((dp.door_actor, dp.bg_cam_index, dp.timer1, dp.timer2, dp.timer3), (Some(d), -1, 37, 26, 10));
    assert_eq!((w.room_ctx.cur.num, w.room_ctx.prev.num, w.room_ctx.status), (2, 1, 1));
    assert_eq!(door(&w, d).actor.room, 2);
    // Camera_Special9 (CAM_FUNCDATA_SPEC9(-5, 60, 0x3202)): the letterbox target 32
    // (sCameraInterfaceFlags 0x3000), bars in steps of 10 a frame.
    frames(&mut w, &mut prev, PadState::default(), 5);
    assert_eq!(w.letterbox.rows(), 32);
    // Through the door: after the animation func_80845EF8 stands Link, drops the previous room
    // (func_80097534), tells the camera (func_8005B1A4: unk_14C |= 8) and moves the void-out
    // point.
    let mut done = false;
    for _ in 0..120 {
        frames(&mut w, &mut prev, PadState::default(), 1);
        if w.player().action != Action::DoorOpen {
            done = true;
            break;
        }
    }
    assert!(done, "Link finishes the door");
    assert_eq!((w.room_ctx.cur.num, w.room_ctx.prev.num), (2, -1));
    assert!(w.player().actor.world_pos.x > 1220.0, "through the door: {:?}", w.player().actor.world_pos);
    let r = w.save.respawn[oot_game::save::RESPAWN_MODE_DOWN];
    assert!((r.pos - w.player().actor.world_pos).length() < 5.0, "respawn at {:?}", r.pos);
    // EnDoor_Open: back to idle once the door animation ends.
    for _ in 0..60 {
        frames(&mut w, &mut prev, PadState::default(), 1);
    }
    let dr = door(&w, d);
    assert_eq!((dr.action, dr.player_is_opening), (en_door::Action::Idle, false));
}

#[test]
fn a_checkable_door_shows_its_text() {
    use oot_game::message::TEXT_STATE_NONE;
    // market_day's transition 0: En_Door at (-482, 0, 615), rotation 0x8000, params 0x028D:
    // DOOR_CHECKABLE with text 0x0D + 0x200 (EnDoor_SetupType), offering within 40
    // (EnDoor_WaitForCheck).
    let Some((mut w, mut prev)) = at_door("ENTR_MARKET_DAY_0", Vec3::new(-482.0, 0.0, 585.0), 0) else { return };
    let d = doors(&w).into_iter().find(|&h| w.actors.actor(h).unwrap().home_pos.distance(Vec3::new(-482.0, 0.0, 615.0)) < 1.0).expect("the door");
    assert_eq!((door(&w, d).door_type(), door(&w, d).action), (en_door::DOOR_CHECKABLE, en_door::Action::WaitForCheck));
    assert_eq!(w.actors.actor(d).unwrap().text_id, 0x20D);
    // Not a door to open: the door never tells Player it can be opened, it offers to talk.
    let p = w.player();
    assert_eq!((p.door_type, p.target_actor), (oot_actors::player::PLAYER_DOORTYPE_NONE, Some(d)));
    frames(&mut w, &mut prev, PadState { button: BTN_A, ..Default::default() }, 1);
    assert_eq!(w.player().action, Action::Talk);
    assert_eq!(w.msg_ctx.text_id, 0x20D);
    assert_eq!(door(&w, d).action, en_door::Action::Check);
    // Read it (A at each wait), and the door waits to be checked again.
    let mut n = 0;
    while w.message_state() != TEXT_STATE_NONE {
        let pad = if w.message_state() == oot_game::message::TEXT_STATE_DONE && prev.button == 0 { PadState { button: BTN_A, ..Default::default() } } else { PadState::default() };
        frames(&mut w, &mut prev, pad, 1);
        n += 1;
        assert!(n < 400);
    }
    assert_eq!(door(&w, d).action, en_door::Action::WaitForCheck);
}
