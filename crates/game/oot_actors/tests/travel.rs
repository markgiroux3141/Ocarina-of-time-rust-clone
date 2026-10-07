//! Room-to-room travel in the Master Quest Deku Tree (GAME-05 milestone 4c): from one start, Link
//! takes every connection milestone 4 opened, each room change through the real door or drop:
//! 0 to 10 and back (`Door_Shutter` barred until the room is cleared), 0 to 1 and back (a plain
//! door, its web burnt with a Deku Stick), the drop from 0 to 3 (room 0's floor web broken by a
//! fall of over 750, `En_Holl`), the drop from 3 to 9 (room 3's floor web burnt), and 9 to 11
//! (the hint scrubs' puzzle clearing room 9, which unbars its door). Within a room Link is placed
//! where walking there needs what isn't ported (said where); enemies in the way are killed.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, BTN_CLEFT, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_ydan_sp::{Action as WebAction, BgYdanSp};
use oot_actors::door_shutter::{Action as DoorAction, DoorShutter};
use oot_actors::en_hintnuts::{Action as HintAction, EnHintnuts};
use oot_actors::obj_syokudai::ObjSyokudai;
use oot_actors::player::{Action as PA, PLAYER_DOORTYPE_SLIDING, PLAYER_IA_DEKU_STICK, UpperAction};
use oot_actors::playthrough::{Playthrough, Route, SHUTTER_DOOR, STICK_DOOR, STICK_START, Step};
use oot_actors::script::stick_towards;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ActorHandle};
use oot_game::camera::CAM_ID_MAIN;
use oot_game::collision_check as cc;
use oot_game::message::{MSGMODE_TEXT_AWAIT_INPUT, TEXT_STATE_AWAITING_NEXT, TEXT_STATE_CHOICE, TEXT_STATE_DONE, TEXT_STATE_DONE_HAS_NEXT, TEXT_STATE_EVENT, TEXT_STATE_NONE};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

const NONE: PadState = PadState { button: 0, stick_x: 0, stick_y: 0 };

struct Run {
    w: PlayState,
    prev: PadState,
    frames: usize,
}

impl Run {
    fn frame(&mut self, pad: PadState) {
        self.w.tick_with(scripted_input(self.prev, pad));
        self.prev = pad;
        self.frames += 1;
    }

    /// A text box waiting for A.
    fn text_waits(&self) -> bool {
        let s = self.w.message_state();
        s != TEXT_STATE_NONE
            && (matches!(s, TEXT_STATE_AWAITING_NEXT | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_CHOICE | TEXT_STATE_EVENT) || self.w.msg_ctx.msg_mode == MSGMODE_TEXT_AWAIT_INPUT)
    }

    /// `button` pressed this frame if it wasn't held on the last (each press an edge).
    fn press(&self, button: u16) -> PadState {
        if self.prev.button & button != 0 { NONE } else { PadState { button, ..NONE } }
    }

    /// Frames with `pad(w)` until `done`, at most `cap`; a text that opens is read (A on each
    /// box that waits for it). Returns the frames run.
    #[track_caller]
    fn until(&mut self, cap: usize, what: &str, done: impl Fn(&PlayState) -> bool, mut pad: impl FnMut(&PlayState, PadState) -> PadState) -> usize {
        for n in 0..cap {
            if done(&self.w) {
                return n;
            }
            let p = if self.w.message_state() != TEXT_STATE_NONE { if self.text_waits() { self.press(BTN_A) } else { NONE } } else { pad(&self.w, self.prev) };
            self.frame(p);
        }
        let p = self.w.player();
        panic!(
            "{what}: not done in {cap} frames (Link {:?} at {:?}, room {} {}, cam {}, text {:#x} state {})",
            p.action,
            p.actor.world_pos,
            self.w.room_ctx.cur.num,
            self.w.room_ctx.prev.num,
            self.w.active_cam_id,
            self.w.msg_ctx.text_id,
            self.w.message_state()
        );
    }

    #[track_caller]
    fn idle_until(&mut self, cap: usize, what: &str, done: impl Fn(&PlayState) -> bool) -> usize {
        self.until(cap, what, done, |_, _| NONE)
    }

    /// Until the main camera is the active one again (the attention cameras over: off it
    /// `Player_UpdateItems` takes no button), no text, and Link standing free.
    #[track_caller]
    fn settle(&mut self) {
        self.idle_until(600, "settle", |w| {
            let p = w.player();
            w.active_cam_id == CAM_ID_MAIN && w.message_state() == TEXT_STATE_NONE && p.action == PA::StandingStill && p.grounded() && p.cs_mode == 0
        });
    }

    /// Through `points` at stick magnitude `mag`, each passed within 15.
    #[track_caller]
    fn walk(&mut self, points: &[Vec3], mag: f32) {
        for &to in points {
            self.until(400, &format!("walk to {to}"), |w| xz_dist(w.player().actor.world_pos, to) < 15.0, |w, _| stick_towards(w, to, mag));
        }
    }

    /// The room's enemies out of the way (they move and attack at `Rand`'s times), but those
    /// `keep` keeps.
    fn kill_enemies(&mut self, keep: impl Fn(&PlayState, ActorHandle) -> bool) {
        for h in self.w.actors.category(ACTORCAT_ENEMY).to_vec() {
            if keep(&self.w, h) {
                continue;
            }
            if let Some(a) = self.w.actors.actor_mut(h) {
                a.kill();
            }
        }
    }

    /// A scripted route's tasks driven from where Link is, each step checked by `check`.
    #[track_caller]
    fn route(&mut self, route: Route, mut check: impl FnMut(&PlayState, Step)) -> Vec<Step> {
        let mut run = Playthrough::for_route(route);
        while let Some(p) = run.next(&self.w) {
            self.frame(p);
            if let Some(s) = run.take_done() {
                check(&self.w, s);
            }
        }
        assert!(run.failure.is_none(), "{route:?}: {:?}", run.failure);
        run.steps.iter().map(|s| s.0).collect()
    }

    /// The sliding door from transition `index`, opened from `front` (in front of it on Link's
    /// side): there, at the door slowly until it offers itself (`doorType`
    /// `PLAYER_DOORTYPE_SLIDING`: `DoorShutter_Idle` with Link within 50 through it, inside its
    /// sides and facing it), then A until Link walks through (`Player_Action_80845CA4`).
    #[track_caller]
    fn open_door(&mut self, index: usize, front: Vec3) {
        self.walk(&[front], 40.0);
        let at = door(&self.w, index).actor.world_pos;
        self.until(120, "the door's offer", |w| w.player().door_type == PLAYER_DOORTYPE_SLIDING, |w, _| stick_towards(w, at, 30.0));
        self.until(10, "A on the door", |w| w.player().action == PA::ExitWalk, |_, prev| if prev.button & BTN_A != 0 { NONE } else { PadState { button: BTN_A, ..NONE } });
    }

    /// Until the door from transition `index` is down behind Link and he's free.
    #[track_caller]
    fn wait_door_shut(&mut self, index: usize) {
        self.idle_until(300, "the door shut", |w| {
            let d = door(w, index);
            let p = w.player();
            !matches!(d.action, DoorAction::Open | DoorAction::Close | DoorAction::WaitPlayerSurprised) && p.cs_mode == 0 && p.action == PA::StandingStill
        });
    }

    fn rooms(&self) -> (i8, i8) {
        (self.w.room_ctx.cur.num, self.w.room_ctx.prev.num)
    }
}

fn xz_dist(a: Vec3, b: Vec3) -> f32 {
    Vec3::new(a.x - b.x, 0.0, a.z - b.z).length()
}

/// The `Door_Shutter` from transition-actor entry `index`.
#[track_caller]
fn door(w: &PlayState, index: usize) -> &DoorShutter {
    w.actors.all().into_iter().find_map(|h| w.actors.downcast::<DoorShutter>(h).filter(|d| d.transition_index() == index)).unwrap_or_else(|| panic!("no Door_Shutter from transition {index}"))
}

/// The live web whose destroyed flag is `flag`.
fn web(w: &PlayState, flag: u8) -> Option<&BgYdanSp> {
    w.actors.all().into_iter().find_map(|h| w.actors.downcast::<BgYdanSp>(h).filter(|s| s.is_destroyed_switch_flag == flag && !s.actor.killed))
}

fn torch_at(w: &PlayState, home: Vec3) -> &ObjSyokudai {
    w.actors.all().into_iter().find_map(|h| w.actors.downcast::<ObjSyokudai>(h).filter(|t| t.actor.home_pos.distance(home) < 1.0)).expect("the torch")
}

#[test]
fn link_travels_the_deku_trees_open_connections() {
    let Some(a) = assets() else { return };
    // One start: `Route::Shutter`'s debug start on room 0's top floor, ten Deku Sticks on C-Left
    // (`deku-tree-sticks`), Navi's forced hints in room 0 heard (`Elf_Msg` 0x1F02's flag 0x1F).
    let route = Route::Shutter;
    let e = a.scenes.entrance_index(route.entrance()).expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-sticks").unwrap();
    let w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, oot_game::audio::GameAudio::default()).expect("Play_Init");
    let mut r = Run { w, prev: NONE, frames: 0 };
    route.debug_start(&mut r.w);
    r.w.flags.set_switch(0x1F);
    r.kill_enemies(|_, _| false);
    assert_eq!(r.rooms(), (0, -1));

    // 1. Room 0 to room 10 and back: `Route::Shutter`'s run. The top floor's switch (Obj_Switch
    // 0x2700) sets 0x27: the web over room 10's door (Bg_Ydan_Sp 0x19CA, burn flag 0x27) burns and
    // sets its destroyed flag 0x0A. Room 10's door (transition 6, params 0x007F: type 1
    // SHUTTER_FRONT_CLEAR, sides 0 and 10) is a plain door from its front room 0
    // (DoorShutter_SetupDoor). A on it: Room_RequestNewRoom(10), the door now room 10's; past it,
    // DoorShutter_SetupClosed's Room_FinishRoomChange (prev -1), and from room 10, its back, with
    // its enemies alive (no clear), the door bars: DoorShutter_WaitClear.
    let steps = r.route(Route::Shutter, |w, s| match s {
        Step::WebBurnt => assert!(web(w, 0x0A).is_none() && w.flags.get_switch(0x0A)),
        Step::DoorOpened => assert_eq!((w.room_ctx.cur.num, door(w, SHUTTER_DOOR).actor.room), (10, 10)),
        Step::DoorBarred => {
            assert_eq!((w.room_ctx.cur.num, w.room_ctx.prev.num), (10, -1));
            assert_eq!((door(w, SHUTTER_DOOR).action, door(w, SHUTTER_DOOR).bars_closed_amount), (DoorAction::WaitClear, 1.0));
            assert!(!w.flags.get_clear(10) && !w.flags.get_temp_clear(10));
        }
        _ => {}
    });
    assert_eq!(steps, vec![Step::SwitchPressed, Step::WebBurnt, Step::DoorOpened, Step::DoorBarred]);
    // Room 10's enemies die: the last one's Actor_RemoveFromCategory sets the room's temporary
    // clear; DoorShutter_WaitClear sees Flags_GetTempClear, makes it permanent (Flags_SetClear) and
    // unbars with the attention cameras on the door and Link; type 1 is DoorShutter_Idle after.
    r.kill_enemies(|_, _| false);
    r.idle_until(5, "room 10's temporary clear", |w| w.flags.get_temp_clear(10));
    r.idle_until(2, "room 10's door unbarring", |w| door(w, SHUTTER_DOOR).action == DoorAction::Unbar);
    assert!(r.w.flags.get_clear(10));
    r.idle_until(400, "room 10's door unbarred", |w| door(w, SHUTTER_DOOR).action == DoorAction::Idle && door(w, SHUTTER_DOOR).bars_closed_amount == 0.0);
    r.settle();
    // Back through it from room 10's side (the door faces -x, rot y 0xC000; room 0 is +x of it):
    // Room_RequestNewRoom(0) on the A press, room 10 gone once it's shut behind him, and from room
    // 0 a plain door again.
    r.open_door(SHUTTER_DOOR, Vec3::new(-595.0, 800.0, 0.0));
    assert_eq!(r.w.room_ctx.cur.num, 0);
    r.wait_door_shut(SHUTTER_DOOR);
    assert_eq!(r.rooms(), (0, -1));
    assert_eq!((door(&r.w, SHUTTER_DOOR).actor.room, door(&r.w, SHUTTER_DOOR).action, door(&r.w, SHUTTER_DOOR).bars_closed_amount), (0, DoorAction::Idle, 0.0));
    // Room 0's actors spawned again with the room: its enemies out of the way again.
    r.frame(NONE);
    r.kill_enemies(|_, _| false);
    r.settle();
    eprintln!("1: back in room 0 at frame {}", r.frames);

    // 2. Room 0 to room 1 and back. Round the top floor (y 800, out of the pit's fence) to where it
    // has no rim over the middle floor (35 to 55 degrees from +z towards +x, past 365 from the
    // middle), and off it at a run: Link drops 440 onto the middle floor (y 360).
    r.walk(&[Vec3::new(-340.0, 800.0, 0.0), Vec3::new(-240.0, 800.0, 240.0), Vec3::new(0.0, 800.0, 340.0), Vec3::new(130.0, 800.0, 314.0), Vec3::new(240.0, 800.0, 240.0)], 60.0);
    r.until(120, "off the top floor", |w| !w.player().grounded(), |w, _| stick_towards(w, Vec3::new(330.0, 800.0, 330.0), 80.0));
    r.idle_until(120, "on the middle floor", |w| w.player().grounded());
    assert_eq!(r.w.player().actor.world_pos.y, 360.0);
    r.settle();
    // Round the map's chest (En_Box 0x0823 at (333, 360, 253)) to `Route::Stick`'s start by the
    // golden torch (Obj_Syokudai 0x03E7, lit by 0x27 since step 1), and its run: a Deku Stick from
    // C-Left lit at the torch, round the floor and over its gap, the web over room 1's door
    // (Bg_Ydan_Sp 0x1FD6: Player_IsBurningStickInRange) burnt, destroyed flag 0x16; through the
    // door (transition 0, params 0x003F: type 0, plain) into room 1.
    r.walk(&[Vec3::new(300.0, 360.0, 280.0), Vec3::new(295.0, 360.0, 215.0), Vec3::new(320.0, 360.0, 185.0), STICK_START.0], 40.0);
    r.settle();
    let steps = r.route(Route::Stick, |w, s| match s {
        Step::StickLit => assert_eq!(torch_at(w, oot_actors::playthrough::STICK_TORCH_HOME).lit_timer, -1),
        Step::WebBurnt => assert!(web(w, 0x16).is_none() && w.flags.get_switch(0x16)),
        Step::DoorOpened => assert_eq!(w.room_ctx.cur.num, 1),
        Step::ThroughDoor => assert_eq!((w.room_ctx.cur.num, w.room_ctx.prev.num), (1, -1)),
        _ => {}
    });
    assert_eq!(steps, vec![Step::StickOut, Step::StickLit, Step::WebBurnt, Step::DoorOpened, Step::ThroughDoor]);
    r.kill_enemies(|_, _| false);
    r.settle();
    // Back through it from room 1's side (it faces room 0, rot y 0x6000): a plain door both ways.
    r.open_door(STICK_DOOR, Vec3::new(-480.0, 400.0, 480.0));
    assert_eq!(r.w.room_ctx.cur.num, 0);
    r.wait_door_shut(STICK_DOOR);
    assert_eq!(r.rooms(), (0, -1));
    assert_eq!((door(&r.w, STICK_DOOR).actor.room, door(&r.w, STICK_DOOR).action), (0, DoorAction::Idle));
    r.frame(NONE);
    r.kill_enemies(|_, _| false);
    r.settle();
    eprintln!("2: back in room 0 at frame {}", r.frames);

    // 3. Room 0 to room 3: the drop. It takes a fall of over 750 onto the floor web (y 0), so
    // from the top floor. The middle floor's vines (WALL_FLAG_3, x 416 to 480, z 0 to 240) take
    // Link up there by hand (checked by the user), but this test's scripted climb dropped off
    // under the top floor's rim (the highest it got was 747), and the other climbable wall
    // (x + z = 656) is behind the Gold Skulltula's crate. So he's placed on the top floor (the
    // camera the main one: `settle`), where its fence round the pit has a gap (towards +x, z 0 to
    // -150), facing the middle, and runs off.
    let top = Vec3::new(290.0, 800.0, -78.0);
    r.w.place_player(top, eng_math::vec3f_yaw(top, Vec3::new(0.0, 800.0, 0.0)));
    r.settle();
    let web0 = web(&r.w, 0x05).expect("room 0's floor web").bg;
    r.until(120, "off the top floor", |w| !w.player().grounded(), |w, _| stick_towards(w, Vec3::new(0.0, 800.0, 0.0), 80.0));
    r.idle_until(120, "on the floor web", |w| w.col.dyna.interact_flag(web0, eng_collision::dyna::DYNA_INTERACT_PLAYER_ON_TOP));
    // BgYdanSp_FloorWebIdle the next frame (the BG category before Player): Link on top
    // (DynaPolyActor_IsPlayerOnTop), his fallDistance over 750, within 80 of its middle: it
    // breaks (unk_16C 200, out of the room).
    let p = r.w.player();
    assert!(p.fall_distance > 750, "{}", p.fall_distance);
    assert!(xz_dist(p.actor.world_pos, Vec3::ZERO) < 80.0);
    r.frame(NONE);
    assert_eq!(web(&r.w, 0x05).map(|s| (s.action, s.actor.room)), Some((WebAction::FloorWebBreaking, -1)));
    // It sinks and gives way (BgYdanSp_FloorWebBroken: destroyed flag 5); Link falls through,
    // and En_Holl transition 9 (ENHOLL_V_INVISIBLE at (0, -320, 0), sides 0 and 3), with him
    // within 120 across and 50 to 200 under it, requests room 3 (side 1), then
    // EnHoll_WaitRoomLoaded's Room_FinishRoomChange.
    r.idle_until(200, "room 3", |w| w.room_ctx.cur.num == 3);
    let y = r.w.player().actor.world_pos.y;
    assert!(y < -370.0 && y > -520.0, "{y}");
    assert!(r.w.flags.get_switch(0x05));
    r.idle_until(5, "room 0 gone", |w| w.room_ctx.prev.num == -1);
    r.kill_enemies(|_, _| false);
    eprintln!("3: in room 3 at frame {}", r.frames);

    // 4. Room 3 to room 9. Out of the water he lands in (the pit, floor -940), up the trench
    // (-905) and the 60 ledge onto the raised floor (-845), onto the floor switch (Obj_Switch
    // 0x0200: flag 0x02); the golden torch (Obj_Syokudai 0x03C2, flag 0x02) lights with its
    // attention camera (ObjSyokudai_Update: litTimer -1).
    let torch3 = Vec3::new(-102.0, -880.0, 244.0);
    r.walk(&[Vec3::new(-200.0, -905.0, -100.0), Vec3::new(-150.0, -905.0, -250.0), Vec3::new(-50.0, -845.0, -250.0)], 80.0);
    r.until(200, "room 3's switch", |w| w.flags.get_switch(0x02), |w, _| stick_towards(w, Vec3::new(-102.0, -845.0, -410.0), 40.0));
    r.settle();
    assert_eq!(torch_at(&r.w, torch3).lit_timer, -1);
    // Back down through the water and out onto the floor by the torch, a stick from C-Left
    // (once the main camera is back), its tip at the flame (67 up): Player's unk_860 210.
    r.walk(&[Vec3::new(-100.0, -905.0, -60.0), Vec3::new(-200.0, -940.0, 0.0), Vec3::new(-200.0, -880.0, 200.0), Vec3::new(-102.0, -880.0, 330.0)], 80.0);
    r.settle();
    r.until(
        60,
        "the stick out",
        |w| w.player().held_item_ap == PLAYER_IA_DEKU_STICK && w.player().upper != UpperAction::Change,
        |w, prev| {
            if w.player().held_item_ap == PLAYER_IA_DEKU_STICK || prev.button & BTN_CLEFT != 0 { NONE } else { PadState { button: BTN_CLEFT, ..NONE } }
        },
    );
    r.until(300, "the stick lit", |w| w.player().unk_860 != 0, |w, _| tip_towards(w, torch3 + Vec3::Y * 67.0));
    assert_eq!(r.w.player().unk_860, 210);
    r.settle();
    // The floor web (Bg_Ydan_Sp 0x0FC6 at (-635, -820, 0)) is on the upper floor (-810 to -820),
    // 85 and more over the trench and the water: past what child Link's ledge check reaches
    // (ageProperties->unk_0C, 71). In the game the way up is room 3's push block
    // (Obj_Makeoshihiki, pushed into the trench: Player's push, not ported yet) or round rooms
    // 4 to 7 (the slingshot's eye switches). So he's placed on the upper floor east of the web,
    // the stick still burning, and walks onto its edge.
    r.w.place_player(Vec3::new(-520.0, -820.0, 0.0), -0x4000);
    r.settle();
    let web3 = web(&r.w, 0x06).expect("room 3's floor web").actor.world_pos;
    r.walk(&[Vec3::new(-560.0, -820.0, 0.0)], 40.0);
    r.settle();
    assert!(r.w.player().held_item_ap == PLAYER_IA_DEKU_STICK && r.w.player().unk_860 > 0);
    // C-Left again swings the stick (FORWARD_SLASH_2H): its burning tip comes down into the web,
    // 0 to 50 over the point 50 under it and within 70 across (BgYdanSp_FloorWebIdle's
    // Player_IsBurningStickInRange): BgYdanSp_BurnWeb, destroyed flag 6; 30 frames of flames and
    // it's gone with its collision.
    r.until(
        30,
        "the slash burns the web",
        |w| web(w, 0x06).is_none_or(|s| s.action == WebAction::BurnFloorWeb),
        |w, prev| {
            if w.player().action == PA::Attack || prev.button & BTN_CLEFT != 0 { NONE } else { PadState { button: BTN_CLEFT, ..NONE } }
        },
    );
    assert!(r.w.flags.get_switch(0x06));
    r.idle_until(40, "the web gone", |w| web(w, 0x06).is_none());
    r.settle();
    // Into the hole: En_Holl transition 10 (ENHOLL_V_INVISIBLE at (-635, -1100, 0), sides 3 and
    // 9) requests room 9 as he passes 50 under it.
    r.until(120, "off room 3's upper floor", |w| !w.player().grounded(), |w, _| stick_towards(w, web3, 40.0));
    r.idle_until(200, "room 9", |w| w.room_ctx.cur.num == 9);
    let y = r.w.player().actor.world_pos.y;
    assert!(y < -1150.0 && y > -1300.0, "{y}");
    r.idle_until(5, "room 3 gone", |w| w.room_ctx.prev.num == -1);
    // He lands in room 9's water; out of it, to the room's debug start.
    r.walk(&[Vec3::new(-660.0, -1880.0, -620.0)], 80.0);
    r.settle();
    eprintln!("4: in room 9 at frame {}", r.frames);

    // 5. Room 9 to room 11: the hint scrubs' order puzzle (tests/scrubs.rs), nuts injected in
    // order: 1 (sPuzzleCounter 1), 2 (2), each frozen; 3 with the count at 2:
    // EnHintnuts_HitByScrubProjectile1 makes it friendly (ACTORCAT_BG) and it runs.
    let [s1, s2, s3] = HINT_HOMES.map(|p| {
        r.w.actors
            .all()
            .into_iter()
            .find(|&h| r.w.actors.downcast::<EnHintnuts>(h).is_some_and(|n| n.actor.home_pos.distance(p) < 1.0 && n.actor.params != oot_actors::en_hintnuts::HINTNUTS_FLOWER))
            .expect("En_Hintnuts")
    });
    let hint = |w: &PlayState, h: ActorHandle| w.actors.downcast::<EnHintnuts>(h).map(|n| n.action);
    for (h, want) in [(s1, 1), (s2, 2)] {
        nut_hits(&mut r, h);
        assert_eq!(puzzle_counter(&mut r.w), want);
        r.idle_until(40, "frozen", |w| hint(w, h) == Some(HintAction::Freeze));
    }
    nut_hits(&mut r, s3);
    assert_eq!((hint(&r.w, s3), r.w.actors.actor(s3).unwrap().category, puzzle_counter(&mut r.w)), (Some(HintAction::BeginRun), oot_game::actor_ctx::ACTORCAT_BG, 2));
    r.idle_until(40, "the third runs", |w| hint(w, s3) == Some(HintAction::Run));
    // Caught: Link touching it (OC1_HIT: the offer auto-accepted), its talk 0x109C read; the
    // text's event: EnHintnuts_SetupLeave; its 100 frames (or off screen) done, the third
    // clears the room (Flags_SetClear(9)) and is gone.
    r.idle_until(10, "the main camera", |w| w.active_cam_id == CAM_ID_MAIN);
    let p = r.w.actors.actor(s3).unwrap().world_pos;
    r.w.place_player(p + Vec3::new(0.0, 0.0, 25.0), i16::MIN);
    r.idle_until(60, "its talk", |w| hint(w, s3) == Some(HintAction::Talk));
    assert_eq!(r.w.msg_ctx.text_id, 0x109C);
    r.idle_until(600, "it leaves", |w| hint(w, s3) == Some(HintAction::Leave));
    r.idle_until(120, "it's gone", |w| w.actors.downcast::<EnHintnuts>(s3).is_none_or(|n| n.actor.killed));
    assert!(r.w.flags.get_clear(9));
    // Room 9's door to room 11 (transition 4, params 0x007F: type 1, sides 11 and 9) is barred
    // from room 9, its back: DoorShutter_WaitClear sees Flags_GetClear and unbars (the attention
    // cameras), then DoorShutter_Idle.
    r.idle_until(5, "room 9's door unbarring", |w| door(w, 4).action == DoorAction::Unbar);
    r.idle_until(400, "room 9's door unbarred", |w| door(w, 4).action == DoorAction::Idle && door(w, 4).bars_closed_amount == 0.0);
    r.settle();
    // Through it from room 9's side (it faces room 9, rot y 0x105B): Room_RequestNewRoom(11).
    // Room 11's whole floor is exit 2 (the drop into Gohma's room): the run stops there.
    r.open_door(4, Vec3::new(-882.0, -1880.0, -932.0));
    assert_eq!((r.w.room_ctx.cur.num, door(&r.w, 4).actor.room), (11, 11));
    eprintln!("5: into room 11 after {} frames", r.frames);
    assert!(r.frames < 3500, "{} frames", r.frames);
}

/// The pad that steers Link so the stick's tip comes to `to`: towards `to` less the tip's offset
/// from him (the stick is held out to his right), slowly; still once there.
fn tip_towards(w: &PlayState, to: Vec3) -> PadState {
    let p = w.player();
    let link = p.actor.world_pos;
    let at = to - (p.melee_weapon_info[0].tip - link);
    let d = xz_dist(at, link);
    if d < 3.0 { NONE } else { stick_towards(w, at, if d < 15.0 { 30.0 } else { 40.0 }) }
}

/// Room 9's three hint scrubs' homes, by their place in the puzzle (params 1, 2, 3).
const HINT_HOMES: [Vec3; 3] = [Vec3::new(-369.0, -1880.0, -904.0), Vec3::new(-947.0, -1880.0, -757.0), Vec3::new(-660.0, -1880.0, -951.0)];

fn puzzle_counter(w: &mut PlayState) -> i16 {
    w.overlay_static::<oot_actors::en_hintnuts::Statics>(oot_actors::en_hintnuts::ACTOR_EN_HINTNUTS).puzzle_counter
}

/// A nut (`ACTOR_EN_NUTSBALL`, as the collision check would report it) hitting scrub `h`, driven.
fn nut_hits(r: &mut Run, h: ActorHandle) {
    let nut = r.w.actor_spawn(oot_actors::en_nutsball::ACTOR_EN_NUTSBALL, Vec3::new(-660.0, -1800.0, -700.0), [0; 3], 1).expect("a nut");
    let n = r.w.actors.downcast_mut::<EnHintnuts>(h).expect("En_Hintnuts");
    n.collider.base.ac_flags |= cc::AC_HIT;
    n.collider.base.ac = Some(nut);
    r.frame(NONE);
    if let Some(a) = r.w.actors.actor_mut(nut) {
        a.kill();
    }
}
