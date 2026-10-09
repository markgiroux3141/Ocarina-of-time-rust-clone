//! Room-to-room travel in the Master Quest Deku Tree (GAME-05 milestones 4c and 5a): from one
//! start, Link takes every connection milestones 4 and 5a opened, each room change through the
//! real door, drop or plane: 0 to 10 and back (`Door_Shutter` barred until the room is cleared),
//! 0 to 1 and back (a plain door, its web burnt with a Deku Stick), 1 to 2 and back (the Fairy
//! Slingshot's seed into room 1's eye switch, and into room 2's ladder, climbed), the drop from 0
//! to 3 (room 0's floor web broken by a fall of over 750, `En_Holl`), round from room 3's floor
//! to its upper floor (3 to 4 by room 3's eye switch, 4 to 5 by room 4's timed torches lit with a
//! stick from room 3, 5 to 6 by room 5's, lit from its held switch's torch across the pool on the
//! floating block, 6 to 7 once room 6 is cleared, 7 to 8 and back through a burnt web, and 7 to
//! 3 through the crawlspace and its `En_Holl`), the drop from 3 to 9 (room 3's floor web burnt
//! with fire carried up from its lower floor by way of its push block), and 9 to 11 (the hint
//! scrubs' puzzle clearing room 9, which unbars its door); then (milestone 6c) room 11's floor
//! into Queen Gohma's room, her fight and the heart, and the blue warp out to Kokiri Forest's
//! emerald cutscene, part 1, by the scripted routes (`Route::BossRoom`, `Gohma`, `BlueWarp`)
//! chained, the whole Deku Tree from one Play_Init. Within a room Link walks, climbs,
//! swims, jumps or crawls; he's placed twice, where walking there needs what isn't ported or the
//! scripted climb fails (said where); enemies in the way are killed.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, BTN_CLEFT, BTN_R, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_ydan_hasi::BgYdanHasi;
use oot_actors::bg_ydan_maruta::{Action as MarutaAction, BgYdanMaruta};
use oot_actors::bg_ydan_sp::{Action as WebAction, BgYdanSp};
use oot_actors::door_shutter::{Action as DoorAction, DoorShutter};
use oot_actors::en_hintnuts::{Action as HintAction, EnHintnuts};
use oot_actors::obj_syokudai::ObjSyokudai;
use oot_actors::player::{Action as PA, PLAYER_DOORTYPE_SLIDING, PLAYER_IA_DEKU_STICK, STATE2_16, UpperAction};
use oot_actors::playthrough::{PUSH_START, Playthrough, ROOM1_DOOR, ROOM1_EYE_FLAG, ROOM2_LADDER_FLAG, ROOM2_LADDER_HOME, ROOM3_EYE_FLAG, ROOM3_EYE_HOME, Route, SHUTTER_DOOR, SLINGSHOT_START, STICK_DOOR, STICK_START, Step};
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
    fn route(&mut self, route: Route, check: impl FnMut(&PlayState, Step)) -> Vec<Step> {
        self.run(Playthrough::for_route(route), check)
    }

    /// A scripted run's tasks driven from where Link is, each step checked by `check`.
    #[track_caller]
    fn run(&mut self, mut run: Playthrough, mut check: impl FnMut(&PlayState, Step)) -> Vec<Step> {
        while let Some(p) = run.next(&self.w) {
            self.frame(p);
            if let Some(s) = run.take_done() {
                check(&self.w, s);
            }
        }
        assert!(run.failure.is_none(), "{:?}: {:?}", run.route, run.failure);
        eprintln!("{:?}: steps {:?}", run.route, run.steps);
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

    /// `open_door` at a run, for a burning stick: to `front` at full tilt, then straight at the
    /// door until it offers itself, then A.
    #[track_caller]
    fn run_door(&mut self, index: usize, front: Vec3) {
        self.walk(&[front], 80.0);
        let at = door(&self.w, index).actor.world_pos;
        self.until(120, "the door's offer", |w| w.player().door_type == PLAYER_DOORTYPE_SLIDING, |w, _| stick_towards(w, at, 60.0));
        self.until(10, "A on the door", |w| w.player().action == PA::ExitWalk, |_, prev| if prev.button & BTN_A != 0 { NONE } else { PadState { button: BTN_A, ..NONE } });
    }

    /// C-Left until a Deku Stick is in hand and the change is over (pressed again only once it's
    /// let go: each press an edge).
    #[track_caller]
    fn take_stick(&mut self) {
        self.until(
            60,
            "the stick out",
            |w| w.player().held_item_ap == PLAYER_IA_DEKU_STICK && w.player().upper != UpperAction::Change,
            |w, prev| {
                if w.player().held_item_ap == PLAYER_IA_DEKU_STICK || prev.button & BTN_CLEFT != 0 { NONE } else { PadState { button: BTN_CLEFT, ..NONE } }
            },
        );
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

/// Room 2's ladder (`Bg_Ydan_Maruta` 0x0121).
fn ladder(w: &PlayState) -> &BgYdanMaruta {
    w.actors.all().into_iter().find_map(|h| w.actors.downcast::<BgYdanMaruta>(h).filter(|m| m.actor.params == oot_actors::bg_ydan_maruta::MARUTA_LADDER)).expect("room 2's ladder")
}

/// Room 5's floating block (`Bg_Ydan_Hasi` 0xFF00: kind 0, `HASI_WATER_BLOCK`).
fn block5(w: &PlayState) -> &BgYdanHasi {
    w.actors.all().into_iter().find_map(|h| w.actors.downcast::<BgYdanHasi>(h).filter(|b| b.actor.params == 0)).expect("room 5's floating block")
}

fn torch_at(w: &PlayState, home: Vec3) -> &ObjSyokudai {
    w.actors.all().into_iter().find_map(|h| w.actors.downcast::<ObjSyokudai>(h).filter(|t| t.actor.home_pos.distance(home) < 1.0)).expect("the torch")
}

#[test]
fn link_travels_the_deku_trees_open_connections() {
    let Some(a) = assets() else { return };
    // One start: `Route::Shutter`'s debug start on room 0's top floor, ten Deku Sticks on C-Left,
    // ten Deku nuts on C-Down and the Fairy Slingshot with 30 seeds on C-Right
    // (`deku-tree-slingshot`), Navi's forced hints in rooms 0 and 2 heard (`Elf_Msg` 0x1F02's flag
    // 0x1F).
    let route = Route::Shutter;
    let e = a.scenes.entrance_index(route.entrance()).expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-slingshot").unwrap();
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
    eprintln!("2: in room 1 at frame {}", r.frames);

    // 3. Room 1 to room 2 and back. Across room 1 (its enemies gone, the big Deku Baba among
    // them) to `Route::Slingshot`'s start, 250 in front of the eye switch over the door to room 2
    // (Obj_Switch 0x0C02: eye, once, flag 0x0C), and its run: the Fairy Slingshot drawn from
    // C-Right, a seed into the eye (ObjSwitch_EyeIsHit: the seed from in front; ObjSwitch_SetOn,
    // flag 0x0C). The door (transition 5, params 0x008C: type 2 SHUTTER_FRONT_SWITCH on 0x0C,
    // sides 2 and 1) is barred from room 1, its back (DoorShutter_SetupDoor: front room 2 isn't
    // the door's room), until the flag: DoorShutter_BarAndWaitSwitchFlag unbars with its
    // attention camera. Through it: Room_RequestNewRoom(2).
    r.walk(&[SLINGSHOT_START.0], 60.0);
    r.settle();
    let steps = r.route(Route::Slingshot, |w, s| match s {
        Step::EyeShot => assert!(w.flags.get_switch(ROOM1_EYE_FLAG)),
        Step::DoorOpened => assert_eq!(w.room_ctx.cur.num, 2),
        Step::ThroughDoor => assert_eq!((w.room_ctx.cur.num, w.room_ctx.prev.num), (2, -1)),
        _ => {}
    });
    assert_eq!(steps, vec![Step::SlingshotDrawn, Step::EyeShot, Step::DoorOpened, Step::ThroughDoor]);
    r.kill_enemies(|_, _| false);
    r.settle();
    // Room 2's ladder (Bg_Ydan_Maruta 0x0121) hangs from the 560 floor over the door's ledge
    // (400), 280 above its own floor (280) until a seed hits it. From the ledge it's out of reach:
    // a seed aimed up at its middle (67 up) meets the 560 floor's edge first (BgCheck_
    // ProjectileLineTest stops it about 38 short). So off the ledge onto the floor (a 120 drop), and
    // from 300 out in front of it (its face looks along -0x2000, into the room), clear of the lift
    // (Obj_Lift 0x0080, whose top, 408, is in the way from the debug start's (-1278, 1278)), a
    // seed: func_808BF078 (DMG_SLINGSHOT), flag 0x21, 20 frames of shaking, the fall to 280.
    r.walk(&[Vec3::new(-1010.0, 400.0, 1090.0)], 60.0);
    r.until(120, "off the ledge", |w| !w.player().grounded(), |w, _| stick_towards(w, Vec3::new(-1150.0, 280.0, 1090.0), 60.0));
    r.idle_until(120, "on room 2's floor", |w| w.player().grounded());
    assert_eq!(r.w.player().actor.world_pos.y, 280.0);
    r.walk(&[Vec3::new(-1140.0, 280.0, 1150.0), Vec3::new(-1130.0, 280.0, 1360.0)], 80.0);
    r.settle();
    let steps = r.run(Playthrough::slingshot_shot(ROOM2_LADDER_HOME + Vec3::Y * 67.0, ROOM2_LADDER_FLAG), |_, _| {});
    assert_eq!(steps, vec![Step::SlingshotDrawn, Step::EyeShot]);
    r.idle_until(120, "the ladder down", |w| ladder(w).action == MarutaAction::DoNothing);
    assert_eq!(ladder(&r.w).actor.world_pos, Vec3::new(-1066.0, 280.0, 1066.0));
    // Up it: at its foot, into its face until Link takes hold (func_8083EC18: the ladder's wall),
    // the stick up (Player_Action_8084BF1C reads it straight) until he steps off the top
    // (Player_Action_8084C5F8) onto the ledge.
    r.walk(&[Vec3::new(-1110.0, 280.0, 1110.0)], 80.0);
    r.settle();
    let foot = Vec3::new(ROOM2_LADDER_HOME.x, 280.0, ROOM2_LADDER_HOME.z);
    r.until(120, "onto the ladder", |w| matches!(w.player().action, PA::Climb | PA::ItemPutAway), |w, _| stick_towards(w, foot, 60.0));
    r.until(400, "up the ladder", |w| w.player().action == PA::ClimbEnd, |w, _| if w.player().action == PA::Climb { PadState { stick_y: 60, ..NONE } } else { NONE });
    r.settle();
    assert_eq!(r.w.player().actor.world_pos.y, 400.0);
    // Back through the door from room 2's side (it faces room 1, rot y 0x6000): from its front
    // room it's plain (DoorShutter_SetupDoor: SHUTTER), Room_RequestNewRoom(1). Room 1's enemies
    // spawned again with it.
    r.open_door(ROOM1_DOOR, Vec3::new(-960.0, 400.0, 960.0));
    assert_eq!(r.w.room_ctx.cur.num, 1);
    r.wait_door_shut(ROOM1_DOOR);
    assert_eq!(r.rooms(), (1, -1));
    r.frame(NONE);
    r.kill_enemies(|_, _| false);
    r.settle();
    eprintln!("3: back in room 1 at frame {}", r.frames);
    // Back through room 0's door from room 1's side (it faces room 0, rot y 0x6000): a plain door
    // both ways.
    r.open_door(STICK_DOOR, Vec3::new(-480.0, 400.0, 480.0));
    assert_eq!(r.w.room_ctx.cur.num, 0);
    r.wait_door_shut(STICK_DOOR);
    assert_eq!(r.rooms(), (0, -1));
    assert_eq!((door(&r.w, STICK_DOOR).actor.room, door(&r.w, STICK_DOOR).action), (0, DoorAction::Idle));
    r.frame(NONE);
    r.kill_enemies(|_, _| false);
    r.settle();
    eprintln!("4: back in room 0 at frame {}", r.frames);

    // 5. Room 0 to room 3: the drop. It takes a fall of over 750 onto the floor web (y 0), so
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
    eprintln!("5: in room 3 at frame {}", r.frames);

    // 6. Room 3 to room 4. Out of the water he lands in (the pit, floor -940), up the trench
    // (-905) and the 60 ledge onto the raised floor (-845), onto the floor switch (Obj_Switch
    // 0x0200: flag 0x02); the golden torch (Obj_Syokudai 0x03C2, flag 0x02) lights with its
    // attention camera (ObjSyokudai_Update: litTimer -1).
    let torch3 = Vec3::new(-102.0, -880.0, 244.0);
    r.walk(&[Vec3::new(-200.0, -905.0, -100.0), Vec3::new(-150.0, -905.0, -250.0), Vec3::new(-50.0, -845.0, -250.0)], 80.0);
    r.until(200, "room 3's switch", |w| w.flags.get_switch(0x02), |w, _| stick_towards(w, Vec3::new(-102.0, -845.0, -410.0), 40.0));
    r.settle();
    assert_eq!(torch_at(&r.w, torch3).lit_timer, -1);
    // Back down through the water and out onto the floor by the torch, under the eye switch over
    // the door to room 4 (Obj_Switch 0x1502 at (-76, -727, 551), facing -z: its triangles 8.5 in
    // front), and a seed up into it from 220 in front: flag 0x15. The door (transition 2, params
    // 0x01D5: type 7 SHUTTER_FRONT_SWITCH_BACK_CLEAR on 0x15, sides 4 and 3) is barred from room 3,
    // its back, until the flag (DoorShutter_BarAndWaitSwitchFlag), and unbars with its camera.
    r.walk(&[Vec3::new(-100.0, -905.0, -60.0), Vec3::new(-200.0, -940.0, 0.0), Vec3::new(-200.0, -880.0, 200.0), Vec3::new(-102.0, -880.0, 330.0)], 80.0);
    r.settle();
    assert!(door(&r.w, 2).bars_closed_amount > 0.0);
    let steps = r.run(Playthrough::slingshot_shot(ROOM3_EYE_HOME + Vec3::new(0.0, 0.0, -8.5), ROOM3_EYE_FLAG), |_, _| {});
    assert_eq!(steps, vec![Step::SlingshotDrawn, Step::EyeShot]);
    r.settle();
    assert_eq!((door(&r.w, 2).action, door(&r.w, 2).bars_closed_amount), (DoorAction::UnbarredCheckSwitchFlag, 0.0));
    // A stick from C-Left (once the main camera is back), its tip at the flame (67 up):
    // Player's unk_860 210 (ObjSyokudai_Update, interactionType -1).
    r.take_stick();
    r.until(60, "the stick lit at room 3's torch", |w| w.player().unk_860 != 0, |w, _| tip_into_flame(w, torch3));
    assert_eq!(r.w.player().unk_860, 210);
    let lit = r.frames;
    // At a run through the door: Room_RequestNewRoom(4). From room 4, its front, the door is
    // SHUTTER_FRONT_CLEAR (DoorShutter_SetupDoor), and room 4 isn't cleared: shut behind Link, it
    // bars (DoorShutter_WaitClear) with Link surprised (PLAYER_CSACTION_2) for 30 frames.
    r.run_door(2, Vec3::new(-75.0, -880.0, 530.0));
    assert_eq!(r.w.room_ctx.cur.num, 4);
    r.idle_until(300, "room 4's door barred", |w| !matches!(door(w, 2).action, DoorAction::Open | DoorAction::Close));
    assert_eq!((r.rooms(), door(&r.w, 2).action, door(&r.w, 2).bars_closed_amount), ((4, -1), DoorAction::WaitPlayerSurprised, 1.0));
    // Room 4's two timed torches (Obj_Syokudai 0x1099: type 1, count 2, flag 0x19), with the
    // stick from room 3. The room's enemies would clear it, and DoorShutter_WaitClear's unbarring
    // brings two attention cameras (the door and Link: 60 frames with Link held) that the stick's
    // 210 frames can't spare, so the Mad Scrub and the Gohma eggs near the torches are killed now
    // and the egg in the room's east corner (En_Goma 6 at (47, -880, 1052): EnGoma_Egg hatches
    // only with Link within 100 in x and z) once the torches are lit.
    let egg = Vec3::new(47.0, -880.0, 1052.0);
    r.kill_enemies(|w, h| w.actors.actor(h).is_some_and(|a| a.home_pos.distance(egg) < 1.0));
    // The first: sLitTorchCount 1, under the count: litTimer 50 * 2 + 110 (210), the stick back
    // up to 200. The second within that: the count reached, Flags_SetSwitch(0x19), both kept lit.
    let (t4a, t4b) = (Vec3::new(-281.0, -880.0, 881.0), Vec3::new(-282.0, -880.0, 1041.0));
    r.until(200, "room 4's first torch lit", |w| torch_at(w, t4a).lit_timer > 0, |w, _| tip_into_flame(w, t4a));
    assert_eq!(r.w.player().unk_860, 200);
    eprintln!("   room 4's first torch {} frames after the stick was lit", r.frames - lit);
    r.until(200, "room 4's second torch lit", |w| w.flags.get_switch(0x19), |w, _| tip_into_flame(w, t4b));
    r.frame(NONE);
    assert_eq!((torch_at(&r.w, t4a).lit_timer, torch_at(&r.w, t4b).lit_timer), (-1, -1));
    // The egg: deleted once the torch's attention camera is over (Actor_UpdateAll freezes the
    // enemy category, killed ones too, while Player is in a cutscene: sCategoryFreezeMasks'
    // PLAYER_STATE1_29), room 4's temporary clear, and DoorShutter_WaitClear unbars. The door to room 5
    // (transition 8, params 0x0099: type 2 SHUTTER_FRONT_SWITCH on 0x19, sides 5 and 4) is barred
    // from room 4 until 0x19: it unbars too.
    r.kill_enemies(|_, _| false);
    r.idle_until(200, "room 4's door unbarred", |w| door(w, 2).bars_closed_amount == 0.0);
    r.settle();
    assert!(r.w.flags.get_clear(4));
    assert_eq!([door(&r.w, 2).bars_closed_amount, door(&r.w, 8).bars_closed_amount], [0.0, 0.0]);
    eprintln!("6: room 4's torches lit at frame {}", r.frames);

    // 7. Room 4 to room 5 (the door from room 4's side, its back: it faces +x, rot y 0x4000);
    // from room 5, its front, plain.
    r.open_door(8, Vec3::new(-305.0, -880.0, 960.0));
    assert_eq!(r.w.room_ctx.cur.num, 5);
    r.wait_door_shut(8);
    assert_eq!((r.rooms(), door(&r.w, 8).action), ((5, -1), DoorAction::Idle));
    r.frame(NONE);
    r.kill_enemies(|_, _| false);
    r.settle();
    // Room 5's door to room 6 (transition 1, params 0x01C9: type 7 on 0x09, sides 6 and 5) is
    // barred from room 5 until its two timed torches (Obj_Syokudai 0x1089: type 1, count 2, flag
    // 0x09) are lit, over the pool on its west bank. The only fire is the golden torch
    // (Obj_Syokudai 0x03FE: type 0, count 15, flag 0x3E) on the east bank, lit while its held
    // floor switch (Obj_Switch 0x3E20: floor, OBJSWITCH_SUBTYPE_HOLD, flag 0x3E) 130 north of it
    // is pressed: ObjSyokudai_Update keeps litTimer -1 while the flag is set, and 20 once it isn't,
    // counting down to out; ObjSwitch_FloorDown clears the flag 6 frames (releaseTimer) after
    // Link steps off. Any lit litTimer lights the stick (unk_860 210). So: a stick out (the one
    // from room 4 burnt out first), onto the switch (its camera and the torch's), off it and the
    // stick into the flame within those 26 frames.
    let (switch5, torch5) = (Vec3::new(-527.0, -880.0, 1114.0), Vec3::new(-528.0, -880.0, 984.0));
    r.idle_until(200, "room 4's stick burnt out", |w| w.player().held_item_ap != PLAYER_IA_DEKU_STICK);
    r.walk(&[Vec3::new(-470.0, -880.0, 1114.0)], 60.0);
    r.settle();
    r.take_stick();
    r.until(100, "room 5's switch", |w| w.flags.get_switch(0x3E), |w, _| stick_towards(w, switch5, 40.0));
    r.settle();
    assert_eq!(torch_at(&r.w, torch5).lit_timer, -1);
    // The pool's floating block (Bg_Ydan_Hasi 0xFF00, BgYdanHasi_UpdateFloatingBlock) carries Link
    // across: along x, -835 + 165 sin(gameplayFrames & 0xFF * pi / 128): at the east end
    // (-670) at 64, the west end (-1000) at 192. So Link waits on the switch for the block (until
    // 20), steps off and lights the stick (about 40), and runs off the bank onto the block as it
    // comes to the east end: the stick's 210 frames last the crossing.
    r.idle_until(256, "the block coming east", |w| w.gameplay_frames & 0xFF == 20);
    r.until(60, "the stick lit at room 5's torch", |w| w.player().unk_860 != 0, |w, _| tip_into_flame(w, torch5));
    let lit = r.frames;
    assert!(torch_at(&r.w, torch5).lit_timer > 0 && !r.w.flags.get_switch(0x3E));
    r.until(60, "on the floating block", |w| w.player().grounded() && w.player().actor.floor_bg_id == block5(w).bg, |w, _| stick_towards(w, block5(w).actor.world_pos, 80.0));
    // Over the block's middle turns the spiked log (Bg_Ydan_Maruta 0x00FF: its triangles across
    // x = -835 from 30 to 50 over the block's top), which knocks a standing Link into the water
    // (his cylinder 38 high). R (Player_ActionHandler_11: the guard, PLAYER_STATE1_SHIELDING)
    // crouches him behind the shield, the stick still in hand: his cylinder the feet to the head
    // plus 10, times 0.8: 19. Held till the log is behind him.
    r.until(100, "under the spiked log", |w| w.gameplay_frames & 0xFF >= 140, |_, _| PadState { button: BTN_R, ..NONE });
    assert!(r.w.player().actor.world_pos.x < -850.0 && r.w.player().actor.floor_bg_id == block5(&r.w).bg);
    // Off the block's west end onto the bank at a run (the jump off its edge: a 20 rise), and the
    // timed torches: the first, sLitTorchCount 1 (litTimer 210, the stick back up to 200); the
    // second, the count reached: 0x09, and the door unbars.
    r.idle_until(60, "the block nearing the west bank", |w| w.gameplay_frames & 0xFF >= 155);
    r.until(60, "on room 5's west bank", |w| w.player().grounded() && w.player().actor.world_pos.x < -1070.0, |w, _| stick_towards(w, Vec3::new(-1160.0, -880.0, 1054.0), 80.0));
    assert_eq!(r.w.player().actor.world_pos.y, -880.0);
    let (t5a, t5b) = (Vec3::new(-1160.0, -880.0, 993.0), Vec3::new(-1161.0, -880.0, 1147.0));
    r.until(100, "room 5's first timed torch lit", |w| torch_at(w, t5a).lit_timer > 0, |w, _| tip_into_flame(w, t5a));
    eprintln!("   room 5's first timed torch {} frames after the stick was lit", r.frames - lit);
    r.until(100, "room 5's second timed torch lit", |w| w.flags.get_switch(0x09), |w, _| tip_into_flame(w, t5b));
    r.settle();
    assert_eq!((door(&r.w, 1).action, door(&r.w, 1).bars_closed_amount), (DoorAction::UnbarredCheckSwitchFlag, 0.0));
    eprintln!("7: room 5's torches lit at frame {}", r.frames);

    // 8. Room 5 to room 6 to room 7. Up to the door's floor (-760): onto the Song of Time block
    // (Obj_Timeblock 0xB9FF, there from the start: its top -820) 50 up, and on 60 up, each a
    // ledge climb (Player_ActionHandler_12).
    r.walk(&[Vec3::new(-1290.0, -870.0, 1070.0)], 80.0);
    r.until(150, "up onto room 5's -760 floor", |w| w.player().grounded() && w.player().actor.world_pos.y == -760.0, |w, _| stick_towards(w, Vec3::new(-1480.0, -760.0, 1070.0), 80.0));
    // Through the door from room 5's side (it faces +x, rot y 0x4000): from room 6, its front,
    // it's SHUTTER_FRONT_CLEAR: barred behind Link until room 6 is cleared, as is the door on to
    // room 7 (transition 7, params 0x007F: type 1, sides 7 and 6), from room 6, its back.
    r.open_door(1, Vec3::new(-1505.0, -760.0, 1070.0));
    assert_eq!(r.w.room_ctx.cur.num, 6);
    r.idle_until(300, "room 6's door barred", |w| !matches!(door(w, 1).action, DoorAction::Open | DoorAction::Close));
    assert_eq!((r.rooms(), door(&r.w, 1).bars_closed_amount, door(&r.w, 7).action), ((6, -1), 1.0, DoorAction::WaitClear));
    // Room 6's enemies die (deleted once Link's surprise is over: Actor_UpdateAll's freeze of
    // the enemy category in a cutscene): its temporary clear, made permanent, and both doors
    // unbar.
    r.kill_enemies(|_, _| false);
    r.idle_until(60, "room 6's temporary clear", |w| w.flags.get_temp_clear(6));
    r.idle_until(400, "room 6's doors unbarred", |w| door(w, 1).bars_closed_amount == 0.0 && door(w, 7).bars_closed_amount == 0.0);
    r.settle();
    assert!(r.w.flags.get_clear(6));
    // Through the door to room 7 from room 6's side (it faces +z, rot y 0): plain from room 7.
    r.open_door(7, Vec3::new(-1855.0, -760.0, 800.0));
    assert_eq!(r.w.room_ctx.cur.num, 7);
    r.wait_door_shut(7);
    assert_eq!((r.rooms(), door(&r.w, 7).action), ((7, -1), DoorAction::Idle));
    r.frame(NONE);
    r.kill_enemies(|_, _| false);
    r.settle();
    eprintln!("8: in room 7 at frame {}", r.frames);

    // 9. Room 7 to room 8 and back. Room 7's door to room 8 (transition 3, params 0x003F: type 0,
    // plain) is behind a wall web (Bg_Ydan_Sp 0x1FCF: WEB_WALL, destroyed flag 0x0F, no burn
    // flag), which a burning stick's tip burns within 100 across its face (BgYdanSp_WallWebIdle).
    // The fire: room 7's four golden torches (Obj_Syokudai 0x03F8: type 0, count 15, flag 0x38)
    // round its held floor switch (Obj_Switch 0x3820: flag 0x38), lit while it's pressed, as room
    // 5's. A stick out, onto the switch, and off it into the south torch's flame.
    let switch7 = Vec3::new(-1959.0, -760.0, 90.0);
    r.walk(&[Vec3::new(-1900.0, -760.0, 400.0), Vec3::new(-1915.0, -760.0, 135.0)], 80.0);
    r.settle();
    r.take_stick();
    r.until(100, "room 7's switch", |w| w.flags.get_switch(0x38), |w, _| stick_towards(w, switch7, 40.0));
    r.settle();
    let t7s = Vec3::new(-1961.0, -760.0, 5.0);
    r.until(60, "the stick lit at room 7's south torch", |w| w.player().unk_860 != 0, |w, _| tip_into_flame(w, t7s));
    // Round the gravestone (Bg_Haka at (-2084, -775, -104)) to the web: it burns (the chime, its
    // destroyed flag, one-point cutscene 3020).
    let web8 = Vec3::new(-2297.0, -760.0, -327.0);
    r.walk(&[Vec3::new(-1990.0, -760.0, -200.0), Vec3::new(-2250.0, -760.0, -290.0)], 80.0);
    r.until(200, "the web over room 8's door burnt", |w| w.flags.get_switch(0x0F), |w, _| stick_towards(w, web8, 40.0));
    r.settle();
    // Through the door (it faces into room 7, rot y 0x2000) and back.
    r.open_door(3, Vec3::new(-2395.0, -760.0, -428.0));
    assert_eq!(r.w.room_ctx.cur.num, 8);
    r.wait_door_shut(3);
    assert_eq!(r.rooms(), (8, -1));
    r.frame(NONE);
    r.kill_enemies(|_, _| false);
    r.settle();
    r.open_door(3, Vec3::new(-2445.0, -760.0, -478.0));
    assert_eq!(r.w.room_ctx.cur.num, 7);
    r.wait_door_shut(3);
    assert_eq!(r.rooms(), (7, -1));
    r.frame(NONE);
    r.kill_enemies(|_, _| false);
    r.settle();
    eprintln!("9: back in room 7 at frame {}", r.frames);

    // 10. Room 7 to room 3's upper floor. Room 7's crawlspace east to room 3 is behind another
    // wall web (Bg_Ydan_Sp 0x1FC7: destroyed flag 0x07): the switch again, the stick lit at the
    // east torch, down the slope to the web.
    r.idle_until(200, "the stick burnt out", |w| w.player().held_item_ap != PLAYER_IA_DEKU_STICK);
    r.walk(&[Vec3::new(-2000.0, -760.0, -200.0), Vec3::new(-1915.0, -760.0, 50.0)], 80.0);
    r.settle();
    r.take_stick();
    r.until(100, "room 7's switch again", |w| w.flags.get_switch(0x38), |w, _| stick_towards(w, switch7, 40.0));
    r.settle();
    let t7e = Vec3::new(-1875.0, -760.0, 89.0);
    r.until(60, "the stick lit at room 7's east torch", |w| w.player().unk_860 != 0, |w, _| tip_into_flame(w, t7e));
    let web7 = Vec3::new(-1355.0, -820.0, 3.0);
    r.walk(&[Vec3::new(-1600.0, -760.0, 40.0), Vec3::new(-1420.0, -805.0, 10.0)], 80.0);
    r.until(200, "the web over the crawlspace burnt", |w| w.flags.get_switch(0x07), |w, _| stick_towards(w, web7, 40.0));
    r.settle();
    // Into the crawlspace: at its mouth (WALL_FLAG_CRAWLSPACE) "Enter" (PLAYER_STATE2_DO_ACTION_
    // ENTER), A: Player_TryEnteringCrawlspace's Player_SetupWaitForPutAway puts the stick away
    // first (so no fire comes through it). The stick forward until he's out the other end
    // (Player_Action_8084C81C). On the way the En_Holl (transition 11, params 0x013F:
    // ENHOLL_H_INVISIBLE at (-1055, -820, 0) facing +x, sides 7 and 3), with him 50 to 100 past
    // it on its +x side, requests room 3, and EnHoll_WaitRoomLoaded finishes the change.
    let mouth = Vec3::new(-1100.0, -820.0, 0.0);
    r.until(120, "the crawlspace's Enter", |w| w.player().state2 & STATE2_16 != 0, |w, _| stick_towards(w, mouth, 40.0));
    r.until(10, "into the crawlspace", |w| matches!(w.player().action, PA::Crawl | PA::ItemPutAway), |w, prev| {
        let mut p = stick_towards(w, mouth, 40.0);
        if prev.button & BTN_A == 0 {
            p.button = BTN_A;
        }
        p
    });
    r.until(600, "out of the crawlspace", |w| w.player().action == PA::CrawlExit, |_, _| PadState { stick_y: 60, ..NONE });
    r.settle();
    assert_eq!(r.rooms(), (3, -1));
    assert_eq!(r.w.player().actor.world_pos.y, -820.0);
    r.kill_enemies(|_, _| false);
    eprintln!("10: on room 3's upper floor at frame {}", r.frames);

    // 11. Room 3 to room 9. The floor web (Bg_Ydan_Sp 0x0FC6 at (-635, -820, 0)) wants fire, and
    // the only fire is room 3's golden torch on the floor below (-880), where the pit's water
    // (surface -895, 45 deep over -940: past child Link's 32, he swims, and the stick goes) is
    // between it and the upper floor. The way back up with the stick: room 3's push block,
    // `Route::Push` (Obj_Oshihiki pushed along the upper floor's channel and off its end into
    // the trench, flag 0x10: Navi's hint there read on the way), Link down beside it and up on it.
    r.walk(&[Vec3::new(-800.0, -815.0, -200.0), PUSH_START.0], 60.0);
    r.settle();
    let steps = r.route(Route::Push, |w, s| {
        if s == Step::BlockInPit {
            assert!(w.flags.get_switch(0x10));
        }
    });
    assert_eq!(steps, vec![Step::BlockGrabbed, Step::BlockInPit, Step::OnBlock]);
    // To the torch: down the trench (-905, 10 under the water), across the water swimming, out
    // onto the floor by the torch; a stick from C-Left, lit.
    r.walk(&[Vec3::new(-280.0, -905.0, -230.0), Vec3::new(-180.0, -905.0, -60.0), Vec3::new(-170.0, -940.0, 30.0), Vec3::new(-160.0, -880.0, 140.0), Vec3::new(-102.0, -880.0, 320.0)], 80.0);
    r.settle();
    r.take_stick();
    r.until(60, "the stick lit at room 3's torch", |w| w.player().unk_860 != 0, |w, _| tip_into_flame(w, torch3));
    let lit = r.frames;
    // Back with it dry: off the torch's floor's edge at a run (z 80, -880) the jump carries Link
    // over the 80 of deep water onto the trench (-905). Along the trench to the block's south
    // face and up it (its top 60 up: Player_ActionHandler_12's climb, the stick kept), and from
    // its top up onto the upper floor (35 up: the ledge climb).
    r.walk(&[Vec3::new(-165.0, -880.0, 100.0)], 80.0);
    r.until(60, "over the water", |w| w.player().grounded() && w.player().actor.world_pos.z < -5.0, |w, _| stick_towards(w, Vec3::new(-200.0, -905.0, -120.0), 80.0));
    assert_eq!((r.w.player().actor.world_pos.y, r.w.player().held_item_ap), (-905.0, PLAYER_IA_DEKU_STICK));
    r.walk(&[Vec3::new(-230.0, -905.0, -150.0), Vec3::new(-300.0, -905.0, -210.0), Vec3::new(-365.0, -905.0, -210.0)], 80.0);
    r.until(100, "up on the block", |w| w.player().grounded() && w.player().actor.world_pos.y > -850.0, |w, _| stick_towards(w, Vec3::new(-365.0, -845.0, -290.0), 80.0));
    r.until(100, "up on the upper floor", |w| w.player().grounded() && w.player().actor.world_pos.y > -825.0 && w.player().actor.world_pos.x < -400.0, |w, _| stick_towards(w, Vec3::new(-470.0, -810.0, -290.0), 80.0));
    // Onto the web's edge.
    r.walk(&[Vec3::new(-500.0, -815.0, -150.0), Vec3::new(-560.0, -820.0, 0.0)], 80.0);
    eprintln!("   at room 3's floor web {} frames after the stick was lit", r.frames - lit);
    let web3 = web(&r.w, 0x06).expect("room 3's floor web").actor.world_pos;
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
    eprintln!("11: in room 9 at frame {}", r.frames);

    // 12. Room 9 to room 11: the hint scrubs' order puzzle (tests/scrubs.rs), nuts injected in
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
    // The rest is the scripted routes from room 9's debug start on (milestone 6c): through the
    // door from room 9's side (it faces room 9, rot y 0x105B): Room_RequestNewRoom(11). Room 11's
    // floor past it is exit 2 (floor_effect 2): ENTR_DEKU_TREE_BOSS_0, Queen Gohma's room. Then
    // her fight and the heart (`Route::Gohma`), and the blue warp out to Kokiri Forest's emerald
    // cutscene, part 1 (`Route::BlueWarp`).
    let start = r.frames;
    let capacity = r.w.save.health_capacity;
    let steps = r.run(Playthrough::for_routes(&[Route::BossRoom, Route::Gohma, Route::BlueWarp]), |w, s| {
        match s {
            Step::DoorOpened => assert_eq!((w.room_ctx.cur.num, door(w, 4).actor.room), (11, 11)),
            // SCENE_DEKU_TREE_BOSS (0x11), room 1 (the corridor; room 0 is hers).
            Step::BossRoom => assert_eq!((w.scene_id, w.room_ctx.cur.num), (0x11, 1)),
            // BossGoma_Defeated: the room cleared (Flags_SetClear), the warp spawned.
            Step::GohmaGone => assert!(w.flags.get_clear(w.room_ctx.cur.num)),
            // Item_B_Heart: its collectible flag, and a heart more (GI_HEART_CONTAINER_2).
            Step::HeartTaken => assert!(w.flags.get_collectible(0x1F) && w.save.health_capacity == capacity + 0x10),
            Step::WarpedOut => assert!(w.save.check_quest_item(oot_game::item::QUEST_KOKIRI_EMERALD)),
            // SCENE_KOKIRI_FOREST (0x55).
            Step::Arrived => assert_eq!(w.scene_id, 0x55),
            _ => {}
        }
    });
    eprintln!("12-14: room 11, Gohma and the blue warp in {} frames, {} in all", r.frames - start, r.frames);
    // In order (the fight's own steps between, as many as it takes).
    let mut rest = steps.iter();
    for s in [
        Step::DoorOpened,
        Step::BossRoom,
        Step::GohmaEntered,
        Step::GohmaWaiting,
        Step::GohmaLookedAt,
        Step::GohmaBattle,
        Step::GohmaStunned,
        Step::GohmaHit,
        Step::GohmaDefeated,
        Step::GohmaGone,
        Step::HeartTaken,
        Step::WarpEntered,
        Step::WarpedOut,
        Step::Arrived,
        Step::EmeraldPart1Over,
    ] {
        assert!(rest.any(|&t| t == s), "{s:?} in order in {steps:?}");
    }
    assert!(r.w.save.get_event_chk_inf(oot_actors::boss_goma::EVENTCHKINF_BEGAN_GOHMA_BATTLE));
    assert!(r.frames < 12000, "{} frames", r.frames);
}

/// The pad that takes a stick's tip into the flame of the torch at `home` quickly (a burning
/// stick's time is short): at the torch at a run, slowing as it nears, until Link is against
/// its stand (its OC cylinder, radius 12, keeps him about 25 off); then, standing, turned in
/// place (a stick under 27: `Player_CalcSpeedAndYawFromControlStick` takes 20 off, no speed)
/// to the facing that puts the standing pose's tip (22 ahead, 11.5 to the left, 67 up: the
/// flame's height) nearest the flame.
fn tip_into_flame(w: &PlayState, home: Vec3) -> PadState {
    let p = w.player();
    let link = p.actor.world_pos;
    let d = xz_dist(link, home);
    if d > 34.0 {
        return stick_towards(w, home, 80.0);
    }
    let tip_at = |yaw: i16| {
        let (s, c) = (eng_math::sin_s(yaw), eng_math::cos_s(yaw));
        link + Vec3::new(s, 0.0, c) * 22.0 + Vec3::new(c, 0.0, -s) * 11.5
    };
    let best = (0..64).map(|k| (k * 0x400) as i16).min_by(|&a, &b| xz_dist(tip_at(a), home).total_cmp(&xz_dist(tip_at(b), home))).unwrap();
    if (best.wrapping_sub(p.actor.shape_rot.y) as i32).abs() < 0x300 {
        return NONE;
    }
    let to = link + Vec3::new(eng_math::sin_s(best), 0.0, eng_math::cos_s(best)) * 100.0;
    stick_towards(w, to, 24.0)
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
