//! `Door_Shutter` (`z_door_shutter.c`) and Player's sliding door against the C, in the Deku
//! Tree (MQ). Its nine doors are transition actors; the tests use:
//! - transition 6, rooms 0/10 at (-560, 800, 0), rot y 0xC000, params 0x007F: type 1
//!   (`SHUTTER_FRONT_CLEAR`), the door on room 0's top floor into room 10;
//! - transition 0, rooms 1/0 at (-455, 400, 455), rot y 0x6000, params 0x003F: type 0, plain;
//! - transition 5, rooms 2/1 at (-936, 400, 936), params 0x008C: type 2
//!   (`SHUTTER_FRONT_SWITCH`), flag 0x0C, room 1's eye switch.
//!
//! Expected values are worked out from the C in the comments. The Keese are taken out of room
//! 0 where Link walks (they dive at him at `Rand`'s times). The key-locked door, which the MQ
//! Deku Tree doesn't place, is transition 0 given type 11 before it spawns.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::door_shutter::*;
use oot_actors::player::{Action as PlayerAction, PLAYER_DOORTYPE_SLIDING};
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::sfx::*;
use oot_game::camera::CAM_SET_DOORC;
use oot_game::play::{DrawOut, PlayState, ViewInfo, scripted_input};
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

/// One frame with A pressed (after a frame without it).
fn press_a(w: &mut PlayState) {
    w.tick_with(scripted_input(PadState::default(), with(PadState::default(), BTN_A)));
}

/// Inside the Deku Tree (`deku-tree-inside`), the sound log on, 60 frames in room 0, then
/// in `room` (`Room_RequestNewRoom`, a frame, `Room_FinishRoomChange`), the Keese gone.
fn deku_tree_room(a: &Arc<GameAssets>, room: i8) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-inside").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 60);
    if room != 0 {
        assert!(w.room_request(room));
        idle(&mut w, 1);
        w.room_change_done();
        idle(&mut w, 2);
    }
    for k in w.actors.all() {
        if w.actors.downcast::<oot_actors::en_firefly::EnFirefly>(k).is_some() {
            w.actors.actor_mut(k).unwrap().kill();
        }
    }
    idle(&mut w, 1);
    w
}

/// The `Door_Shutter` spawned from transition-actor entry `index`.
fn door_of(w: &PlayState, index: usize) -> ActorHandle {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<DoorShutter>(h).is_some_and(|d| d.transition_index() == index)).unwrap_or_else(|| panic!("no Door_Shutter from transition {index}"))
}

fn door(w: &PlayState, h: ActorHandle) -> &DoorShutter {
    w.actors.downcast::<DoorShutter>(h).expect("Door_Shutter")
}

fn sfx_on(w: &PlayState, frame: u32, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == frame && s == id)
}

/// Link standing on room 0's top floor `dx` in front of room 10's door (on room 0's side,
/// +x), facing it (-x).
fn before_room_10s_door(w: &mut PlayState, dx: f32) {
    let x = -560.0 + dx;
    let (y, _) = w.col.entity_raycast_down(Vec3::new(x, 850.0, 0.0));
    w.place_player(Vec3::new(x, y, 0.0), -0x4000);
}

#[test]
fn the_doors_room_0_spawns() {
    let Some(a) = assets() else { return };
    let w = deku_tree_room(&a, 0);
    let ydan = w.object_ctx.get_index(0x0036).expect("object_ydan_objects");
    for (index, ty, pos) in [(0usize, SHUTTER, Vec3::new(-455.0, 400.0, 455.0)), (6, SHUTTER_FRONT_CLEAR, Vec3::new(-560.0, 800.0, 0.0))] {
        let d = door(&w, door_of(&w, index));
        // DOORSHUTTER_GET_TYPE; SCENE_DEKU_TREE's style (sSceneInfo) and its object.
        assert_eq!((d.door_type, d.style_type, d.required_object_slot), (ty, DOORSHUTTER_STYLE_DEKU_TREE, Some(ydan)), "transition {index}");
        assert_eq!(d.actor.world_pos, pos);
        // Actor_SetFocus(60); sInitChain's scale 1.
        assert_eq!((d.actor.focus_pos, d.actor.scale), (pos + Vec3::Y * 60.0, Vec3::ONE));
        // home.pos.z = shape.yOffset, still 0 in the init (unused afterwards).
        assert_eq!(d.actor.home_pos, Vec3::new(pos.x, pos.y, 0.0));
        // Spawned with Link in room 0. Transition 0's front room is 1, so it's a plain door
        // (type 0 anyway); transition 6's front room is 0, its own: plain from here
        // (DoorShutter_SetupDoor's SHUTTER). gfxType1 for SHUTTER: gDTDungeonDoor1DL.
        assert_eq!((d.actor.room, d.action, d.gfx_type, d.bars_closed_amount, d.unlock_timer), (0, Action::Idle, DOORSHUTTER_GFX_DEKU_TREE_1, 0.0, 0), "transition {index}");
    }
}

#[test]
fn link_goes_through_room_10s_door_and_it_bars_behind_him() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 0);
    let h = door_of(&w, 6);
    before_room_10s_door(&mut w, 30.0);
    idle(&mut w, 1);
    // DoorShutter_GetPlayerSide: 30 behind the door (rel.z -30: Actor_WorldToActorCoords with
    // rot y -0x4000), within 20 to its sides and 15 up or down, facing it (yawDiff 0): -1.
    {
        let p = w.player();
        assert_eq!((p.door_type, p.door_direction, p.door_actor), (PLAYER_DOORTYPE_SLIDING, -1, Some(h)));
    }
    let p0 = w.player().actor.world_pos;
    press_a(&mut w);
    let f = w.audio.frames;
    {
        // Player_ActionHandler_1's sliding door: the walk (func_80838E70, actionVar1 0), the
        // door's yaw (home.rot.y -0x4000, doorDirection < 0: unturned), speed 0.1, unk_450 20
        // towards the door's front (+x here) and unk_45C 120 past it, the bg camera of the
        // side behind (sides[1].bgCamIndex -1), room 10 loading.
        let p = w.player();
        assert_eq!((p.action, p.current_yaw, p.linear_velocity, p.sliding_door_bg_cam_index), (PlayerAction::ExitWalk, -0x4000, 0.1, -1));
        assert!((p.unk_450.x - (p0.x + 20.0)).abs() < 0.01 && (p.unk_45C.x - (p0.x - 120.0)).abs() < 0.01, "{:?} {:?}", p.unk_450, p.unk_45C);
        // isActive: DoorShutter_Idle goes to DoorShutter_Open. The door is in room 10 now
        // (doorActor->room = curRoom.num after Room_RequestNewRoom).
        let d = door(&w, h);
        assert_eq!((d.is_active, d.action, d.actor.room, d.actor.velocity.y), (1, Action::Open, 10, 0.0));
        assert_eq!((w.room_ctx.cur.num, w.room_ctx.prev.num, w.room_ctx.status), (10, 0, 1));
    }
    // DoorShutter_Open once room 10 is in (roomCtx.status 0): the sound and the door camera
    // on the first frame (DoorShutter_InitOpeningDoorCam: bgCamIndex -1 is CAM_SET_DOORC),
    // then Math_StepToF(velocity.y, 15, 3) and up by it to home + 200.
    let mut y = 800.0;
    let mut vy = 0.0f32;
    let mut close_frame = None;
    for k in 1..=40 {
        idle(&mut w, 1);
        let d = door(&w, h);
        if d.action == Action::Close {
            close_frame = Some(k);
            break;
        }
        vy = (vy + 3.0).min(15.0);
        y = (y + vy).min(1000.0);
        assert_eq!((d.actor.velocity.y, d.actor.world_pos.y), (vy, y), "frame {k}");
        if k == 1 {
            assert!(sfx_on(&w, f + 1, NA_SE_EV_SLIDE_DOOR_OPEN));
            assert_eq!((w.room_ctx.status, w.game_camera.setting), (0, CAM_SET_DOORC));
        }
    }
    // Open at 1000 (16 frames: 3 + 6 + 9 + 12 + 15 x 11 = 195, then 5) and Link more than 50
    // past it: DoorShutter_SetupDoor from room 10, where the door's type 1 is barred until the
    // room is cleared (bars 1, velocity 30: it slams), and the closing sound.
    let c = close_frame.expect("the door never closed");
    let d = door(&w, h);
    assert_eq!((d.actor.world_pos.y, d.actor.velocity.y, d.bars_closed_amount), (1000.0, 30.0, 1.0));
    assert!(d.actor.xz_dist_to_player > 50.0);
    assert!(sfx_on(&w, w.audio.frames, NA_SE_EV_SLIDE_DOOR_CLOSE));
    // DoorShutter_Close: velocity 30 (not under 20: kept), down 30 a frame to home: 970 ... 820,
    // then 800 on the seventh.
    for k in 1..=6 {
        idle(&mut w, 1);
        assert_eq!((door(&w, h).action, door(&w, h).actor.world_pos.y), (Action::Close, 1000.0 - 30.0 * k as f32), "close frame {k}");
    }
    let dust_before = w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == oot_game::effect::EFFECT_SS_DUST).count();
    idle(&mut w, 1);
    let shut = w.audio.frames;
    {
        // Landed, faster than 20: Actor_SpawnFloorDustRing (11 clouds), NA_SE_EV_STONE_BOUND;
        // DoorShutter_SetupClosed: the door's room the side Link is on (room 10, unchanged), the
        // old room gone (Room_FinishRoomChange), the respawn point here
        // (PLAYER_PARAMS(PLAYER_START_MODE_MOVE_FORWARD_SLOW, PLAYER_START_BG_CAM_DEFAULT)),
        // and barred on his side: Link held (PLAYER_CSACTION_2).
        let d = door(&w, h);
        assert_eq!((d.actor.world_pos.y, d.action, d.actor.room, d.is_active, d.bars_closed_amount), (800.0, Action::WaitPlayerSurprised, 10, 0, 1.0));
        assert!(sfx_on(&w, shut, NA_SE_EV_STONE_BOUND));
        let dust = w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == oot_game::effect::EFFECT_SS_DUST).count();
        assert_eq!(dust - dust_before, 11);
        assert_eq!((w.room_ctx.cur.num, w.room_ctx.prev.num), (10, -1));
        let r = &w.save.respawn[0];
        assert_eq!((r.player_params, r.pos), (0x0DFF, w.player().actor.world_pos));
        assert_eq!(w.player().cs_mode, 2);
        assert!(c > 0);
    }
    // DoorShutter_WaitPlayerSurprised: actionTimer++ > 30, so 32 frames; then
    // PLAYER_CSACTION_7 and DoorShutter_SetupDoor: barred, waiting for the clear.
    for k in 1..32 {
        idle(&mut w, 1);
        assert_eq!(door(&w, h).action, Action::WaitPlayerSurprised, "frame {k}");
    }
    idle(&mut w, 1);
    assert_eq!(door(&w, h).action, Action::WaitClear);
    assert_eq!(w.player().cs_mode, 7);
    // Link went 120 past where he started (unk_45C), stopping within 20 of it.
    let p = w.player().actor.world_pos;
    assert!(p.x < p0.x - 100.0 && p.x > p0.x - 121.0, "{p:?}");
}

#[test]
fn clearing_room_10_unbars_its_door() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 10);
    let h = door_of(&w, 6);
    // From room 10 (its back), transition 6's type 1 is barred until room 10 is cleared.
    {
        let d = door(&w, h);
        assert_eq!((d.actor.room, d.action, d.bars_closed_amount, d.gfx_type), (10, Action::WaitClear, 1.0, DOORSHUTTER_GFX_DEKU_TREE_2));
    }
    // Facing it from inside, Navi has something to say (naviTextId -0x202); no door offer.
    let x = -560.0 - 30.0;
    let (y, _) = w.col.entity_raycast_down(Vec3::new(x, 850.0, 0.0));
    w.place_player(Vec3::new(x, y, 0.0), 0x4000);
    idle(&mut w, 1);
    assert_eq!(w.player().door_type, 0);
    // Flags_GetTempClear (all the room's enemies gone): Flags_SetClear, DoorShutter_Unbar,
    // the attention cameras on the door and Link, actionTimer -100.
    w.flags.set_temp_clear(10);
    idle(&mut w, 1);
    assert!(w.flags.get_clear(10));
    assert_eq!((door(&w, h).action, door(&w, h).action_timer), (Action::Unbar, -100));
    // The timer climbs on the odd frames until the attention camera turns to a door
    // (func_8005B198 is the category it attended, ACTORCAT_DOOR) or it reaches 0; then 5, and
    // down to 0, then the bars lift 0.2 a frame with NA_SE_EV_METALDOOR_OPEN.
    let mut n = 0;
    while door(&w, h).action_timer != 5 {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 210, "the attention never came: {}", door(&w, h).action_timer);
    }
    idle(&mut w, 5);
    assert_eq!(door(&w, h).action_timer, 0);
    // Math_StepToF in f32: 1 - 0.2 five times leaves 3e-8, which isn't past 0 yet, so the
    // bars are up on the sixth frame.
    let mut want = Vec::new();
    let mut b = 1.0f32;
    loop {
        b += -0.2;
        if (b - 0.0) * -0.2 >= 0.0 {
            want.push(0.0);
            break;
        }
        want.push(b);
    }
    assert_eq!(want.len(), 6);
    for (k, &e) in want.iter().enumerate() {
        idle(&mut w, 1);
        assert_eq!(door(&w, h).bars_closed_amount, e, "frame {k}");
        if k == 0 {
            assert!(sfx_on(&w, w.audio.frames, NA_SE_EV_METALDOOR_OPEN));
        }
        if k < 5 {
            assert_eq!(door(&w, h).action, Action::Unbar);
        }
    }
    // Type 1: back to DoorShutter_Idle.
    assert_eq!(door(&w, h).action, Action::Idle);
}

#[test]
fn a_switch_bars_room_1s_door_to_room_2() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 1);
    let h = door_of(&w, 5);
    // From room 1, transition 5's front room (2) isn't the door's: SHUTTER_FRONT_SWITCH, its
    // flag 0x0C unset: barred (DoorShutter_BarAndWaitSwitchFlag).
    assert_eq!((door(&w, h).action, door(&w, h).bars_closed_amount), (Action::BarAndWaitSwitchFlag, 1.0));
    // The eye switch's flag: unbarred after the attention camera, then
    // DoorShutter_UnbarredCheckSwitchFlag (not a type 0 or 1).
    w.flags.set_switch(0x0C);
    idle(&mut w, 1);
    assert_eq!((door(&w, h).action, door(&w, h).action_timer), (Action::Unbar, -100));
    let mut n = 0;
    while door(&w, h).action != Action::UnbarredCheckSwitchFlag {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 230);
    }
    assert_eq!(door(&w, h).bars_closed_amount, 0.0);
    // The flag cleared (not while the door's open): barred again, 0.2 a frame, with
    // NA_SE_EV_METALDOOR_CLOSE as the bars start down.
    w.flags.unset_switch(0x0C);
    idle(&mut w, 1);
    assert_eq!(door(&w, h).action, Action::BarAndWaitSwitchFlag);
    idle(&mut w, 1);
    assert!((door(&w, h).bars_closed_amount - 0.2).abs() < 1e-6);
    assert!(sfx_on(&w, w.audio.frames, NA_SE_EV_METALDOOR_CLOSE));
}

/// Transition 0 respawned as a key-locked door (type 11) on switch flag 0x20.
fn key_locked_door(w: &mut PlayState) -> ActorHandle {
    let h = door_of(w, 0);
    w.actors.actor_mut(h).unwrap().kill();
    idle(w, 1);
    // DoorShutter_Destroy gave the entry back (id positive); the next room change spawns it.
    assert!(w.transi_actors[0].id > 0);
    w.transi_actors[0].params = ((SHUTTER_KEY_LOCKED as i16) << 6) | 0x20;
    w.room_change_done();
    idle(w, 1);
    door_of(w, 0)
}

#[test]
fn a_key_locked_door_takes_a_small_key() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 0);
    let h = key_locked_door(&mut w);
    // DoorShutter_Init: flag 0x20 unset, so locked (unlockTimer 10). From room 0 its type is
    // kept (DoorShutter_SetupDoor skips the side swap for SHUTTER_KEY_LOCKED).
    assert_eq!((door(&w, h).door_type, door(&w, h).unlock_timer, door(&w, h).action), (SHUTTER_KEY_LOCKED, 10, Action::Idle));
    // Link in front of it (rot y 0x6000: its front faces (sin, cos) of 0x6000), on room 0's
    // middle floor, facing it.
    let d = door(&w, h).actor.clone();
    let out = Vec3::new(eng_math::sin_s(d.shape_rot.y), 0.0, eng_math::cos_s(d.shape_rot.y));
    let pos = d.world_pos - out * 30.0;
    let (y, _) = w.col.entity_raycast_down(pos + Vec3::Y * 50.0);
    w.place_player(Vec3::new(pos.x, y, pos.z), d.shape_rot.y);
    // No key: Navi's -0x203, no offer.
    w.save.inventory.dungeon_keys[0] = 0;
    idle(&mut w, 1);
    assert_eq!(w.player().door_type, 0);
    // A key: the offer, and Player's doorTimer 10.
    w.save.inventory.dungeon_keys[0] = 1;
    idle(&mut w, 1);
    assert_eq!((w.player().door_type, w.player().door_timer), (PLAYER_DOORTYPE_SLIDING, 10));
    press_a(&mut w);
    // DoorShutter_Idle with isActive: the flag set, a key spent, the chains' sound.
    assert!(w.flags.get_switch(0x20));
    assert_eq!(w.save.inventory.dungeon_keys[0], 0);
    assert!(sfx_on(&w, w.audio.frames, NA_SE_EV_CHAIN_KEY_UNLOCK));
    // Player waits for his doorTimer (actionVar2 0) while the door's unlockTimer counts down
    // (DECR): it opens on the frame the timer reaches 0 (10 frames).
    for k in 1..10 {
        idle(&mut w, 1);
        assert_eq!((door(&w, h).unlock_timer, door(&w, h).actor.world_pos.y), (10 - k, 400.0), "frame {k}");
    }
    idle(&mut w, 1);
    assert_eq!((door(&w, h).unlock_timer, door(&w, h).actor.velocity.y), (0, 3.0));
}

#[test]
fn the_door_draws_from_links_side_and_its_lock() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 0);
    let h = door_of(&w, 6);
    before_room_10s_door(&mut w, 100.0);
    idle(&mut w, 1);
    let draw = |w: &PlayState, h: ActorHandle, eye: Vec3| {
        let d = door(w, h);
        let rs = oot_game::actor_ctx::ActorImpl::render_state(d);
        let mut out = DrawOut::default();
        oot_game::actor_ctx::ActorImpl::draw(d, &rs, w, &ViewInfo { eye, billboard: glam::Mat4::IDENTITY }, &mut out);
        out.opa.iter().map(|c| format!("{:?}", c.mesh)).collect::<Vec<_>>()
    };
    // DoorShutter_ShouldDraw: the eye on Link's side (+x of the door): drawn, its list alone
    // (no bars).
    let drawn = draw(&w, h, Vec3::new(-300.0, 900.0, 0.0));
    assert_eq!(drawn.len(), 1, "{drawn:?}");
    assert!(drawn[0].contains("gDTDungeonDoor1DL"), "{drawn:?}");
    // The eye on the other side of it: not drawn.
    assert!(draw(&w, h, Vec3::new(-800.0, 900.0, 0.0)).is_empty());
    // The key-locked door draws its four chains and its lock (Actor_DrawDoorLock).
    let k = key_locked_door(&mut w);
    let d = door(&w, k).actor.clone();
    let out = Vec3::new(eng_math::sin_s(d.shape_rot.y), 0.0, eng_math::cos_s(d.shape_rot.y));
    let pos = d.world_pos - out * 100.0;
    let (y, _) = w.col.entity_raycast_down(pos + Vec3::Y * 50.0);
    w.place_player(Vec3::new(pos.x, y, pos.z), d.shape_rot.y);
    idle(&mut w, 1);
    let drawn = draw(&w, k, pos + Vec3::Y * 100.0 - out * 100.0);
    assert_eq!(drawn.iter().filter(|m| m.contains("gDoorChainDL")).count(), 4, "{drawn:?}");
    assert_eq!(drawn.iter().filter(|m| m.contains("gDoorLockDL")).count(), 1, "{drawn:?}");
}

#[test]
fn gohmas_slab_falls_and_thuds_before_her_battle() {
    let Some(a) = assets() else { return };
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_BOSS_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-inside").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 3);
    // BossGoma's spawn (Actor_SpawnAsChild, here without the parent): DOORSHUTTER_PARAMS(SHUTTER_GOHMA_BLOCK, 0).
    let at = Vec3::new(164.72, -480.0, 397.68002);
    let h = w.actor_spawn(ACTOR_DOOR_SHUTTER, at, [0, -0x705C, 0], (SHUTTER_GOHMA_BLOCK as i16) << 6).expect("spawn");
    // DoorShutter_Init: sTypeStyles' Gohma block (object_goma), out of every room (room -1).
    assert_eq!((door(&w, h).style_type, door(&w, h).actor.room, door(&w, h).action), (DOORSHUTTER_STYLE_GOHMA_BLOCK, -1, Action::WaitForObject));
    idle(&mut w, 1);
    // DoorShutter_WaitForObject: its collision (gGohmaDoorCol), gravity -2, the sound.
    {
        let d = door(&w, h);
        assert_eq!((d.action, d.gfx_type, d.actor.gravity, d.actor.velocity.y), (Action::GohmaBlockFall, DOORSHUTTER_GFX_GOHMA_BLOCK, -2.0, 0.0));
        assert!(d.bg != eng_collision::dyna::BG_ACTOR_MAX);
        assert!(sfx_on(&w, w.audio.frames, NA_SE_EV_SLIDE_DOOR_CLOSE));
    }
    // DoorShutter_GohmaBlockFall: Actor_MoveXZGravity (2 more a frame, at most 20, times
    // Actor_UpdatePos's R_UPDATE_RATE * 0.5) until Actor_UpdateBgCheckInfo (its own collision
    // skipped) finds the floor.
    let (floor, _) = w.col.entity_raycast_down(Vec3::new(at.x, at.y - 1.0, at.z));
    let (mut y, mut vy) = (at.y, 0.0f32);
    loop {
        idle(&mut w, 1);
        vy = (vy - 2.0).max(-20.0);
        y += vy * eng_math::UPDATE_SCALE;
        let d = door(&w, h);
        if y <= floor {
            assert_eq!((d.actor.world_pos.y, d.action), (floor, Action::GohmaBlockBounce));
            break;
        }
        assert_eq!((d.actor.world_pos.y, d.actor.velocity.y, d.action), (y, vy, Action::GohmaBlockFall));
    }
    // Landed before EVENTCHKINF_BEGAN_GOHMA_BATTLE: isActive 10, the thud, a ring of 21 lit
    // dust clouds (Actor_SpawnFloorDustRing 70, 20).
    assert_eq!(door(&w, h).is_active, 10);
    assert!(sfx_on(&w, w.audio.frames, NA_SE_EV_STONE_BOUND));
    // DoorShutter_GohmaBlockBounce: shape.yOffset = isActive * 3 / 10 * sinf(isActive * 250 / 100).
    for k in (0..10).rev() {
        idle(&mut w, 1);
        let want = k as f32 * 3.0 / 10.0 * (k as f32 * 250.0 / 100.0).sin();
        assert_eq!((door(&w, h).is_active, door(&w, h).actor.shape_y_offset), (k, want), "isActive {k}");
    }
}
