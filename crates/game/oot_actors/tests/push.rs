//! Player's push and pull (GAME-05 milestone 4c) and the push block against the C:
//! `Player_ActionHandler_5`'s push branch, `func_8083F72C`, `func_8083F9D0`, `func_8083FAB8`,
//! `func_8083FB14`, `func_8083FFB8`, `func_8084B840` and the actions `Player_Action_8084B78C`
//! (holding on), `_8084B898` (pushing), `_8084B9E4` (pulling); `Obj_Oshihiki` (room 3's small
//! block, the init's flags and the strength check on blocks a test spawns, a block riding on
//! another: `ObjOshihiki_MoveWithBlockUnder`, from the draw) and `Obj_Makeoshihiki` (the flags
//! its draw sets where the block rests, its init's places).
//!
//! Expected values are worked out from the C in the comments. The numbers they use:
//! - `ObjOshihiki_Push`: `pushSpeed` 0.5 more a frame up to 2, `pushDist` stepped to 20 by it:
//!   0.5, 1.5, 3, 5, 7, ... 19, 20 (twelve frames, the twelfth the stop); then `timer` 10;
//! - `func_8084B840`: 2 added to the block's `unk_150` each frame Link pushes (-2 pulling), and
//!   `unk_158` his `world.rot.y`;
//! - room 3's block (`Obj_Makeoshihiki` 0xFF10, `sBlocks[1]`): a small one (scale 0.1, 60 across)
//!   at (-605, -820, -290) in a channel of the upper floor that ends at x -395 over the pit,
//!   whose floor is at -905; the channel is 10 below the floor at its west end (a step at
//!   x -635, from -820 up to -810).

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_haka::{Action as HakaAction, BgHaka};
use oot_actors::obj_makeoshihiki::ObjMakeoshihiki;
use oot_actors::obj_oshihiki::{self as osh, ACTOR_OBJ_OSHIHIKI, Action as BA, ObjOshihiki};
use oot_actors::player::{Action as PA, STATE2_0, STATE2_4, STATE2_6, STATE2_8};
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::sfx::*;
use oot_game::camera::{CAM_ID_MAIN, CAM_MODE_PUSH_PULL};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::player_lib::{PLAYER_STR_BRACELET, PLAYER_STR_GOLD_G, PLAYER_STR_NONE, player_get_strength};
use oot_game::save::SaveContext;

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

/// The stick forward (Link faces +x, the camera behind him) at a run's tilt, with `buttons`.
fn fwd(buttons: u16) -> PadState {
    PadState { button: buttons, stick_x: 0, stick_y: 60 }
}

/// The stick back.
fn back(buttons: u16) -> PadState {
    PadState { button: buttons, stick_x: 0, stick_y: -60 }
}

fn held(buttons: u16) -> PadState {
    PadState { button: buttons, ..Default::default() }
}

/// Inside the Deku Tree (`deku-tree-inside`, the sound log on), `flags` set first (the room's
/// actors read them as they spawn), in room 3 with Link at `at` facing +x, Navi's hint at the
/// block heard (`Elf_Msg` 0x3208's flag 0x32), the enemies gone, standing on the main camera.
fn room3(a: &Arc<GameAssets>, flags: &[i32], at: Vec3) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-inside").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    for &f in flags {
        w.flags.set_switch(f);
    }
    w.flags.set_switch(0x32);
    oot_actors::playthrough::deku_tree_room_start(&mut w, 3, at, 0x4000);
    for h in w.actors.category(oot_game::actor_ctx::ACTORCAT_ENEMY).to_vec() {
        if let Some(a) = w.actors.actor_mut(h) {
            a.kill();
        }
    }
    for _ in 0..300 {
        idle(&mut w, 1);
        let p = w.player();
        if w.active_cam_id == CAM_ID_MAIN && p.action == PA::StandingStill && p.grounded() {
            break;
        }
    }
    w
}

fn blocks(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<ObjOshihiki>(h).is_some()).collect()
}

fn block(w: &PlayState) -> ActorHandle {
    let b = blocks(w);
    assert_eq!(b.len(), 1, "room 3 has one push block");
    b[0]
}

fn get(w: &PlayState, h: ActorHandle) -> &ObjOshihiki {
    w.actors.downcast::<ObjOshihiki>(h).expect("the block")
}

fn sfx_on(w: &PlayState, frame: u32, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == frame && s == id)
}

fn sfx_count(w: &PlayState, id: u16) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).count()
}

/// Walks Link (facing +x) into the block's west face at a walk until "Grab" shows
/// (`PLAYER_STATE2_0`), then lets him come to a stop; returns the last pad.
fn to_the_block(w: &mut PlayState) -> PadState {
    let mut prev = PadState::default();
    for _ in 0..120 {
        if w.player().state2 & STATE2_0 != 0 {
            break;
        }
        prev = frame(w, prev, PadState { button: 0, stick_x: 0, stick_y: 40 });
    }
    assert_ne!(w.player().state2 & STATE2_0, 0, "no Grab at the block (Link at {:?})", w.player().actor.world_pos);
    for _ in 0..20 {
        if w.player().linear_velocity == 0.0 {
            break;
        }
        prev = frame(w, prev, PadState::default());
    }
    prev
}

/// A held at the block until Link holds on.
fn grab(w: &mut PlayState) -> PadState {
    let mut prev = to_the_block(w);
    for _ in 0..10 {
        prev = frame(w, prev, held(BTN_A));
        if w.player().action == PA::PushWait {
            return prev;
        }
    }
    panic!("never held on ({:?})", w.player().action);
}

/// Holding on to room 3's block and pushing it one block length, frame by frame.
#[test]
fn holding_on_and_pushing_the_block() {
    let Some(a) = assets() else { return };
    let mut w = room3(&a, &[], Vec3::new(-670.0, -810.0, -290.0));
    let b = block(&w);
    let bg = get(&w, b).bg;
    assert_eq!((get(&w, b).actor.world_pos, get(&w, b).action), (Vec3::new(-605.0, -820.0, -290.0), Some(BA::OnScene)));
    let mut prev = to_the_block(&mut w);
    // Player_ActionHandler_5: at a WALL_FLAG_6 wall (the block's), facing it (sShapeYawToTouchedWall
    // under 0x3000), on the ground, its top 50 over Link's floor (39 or more): "Grab".
    let p = w.player();
    assert!(p.wall_height >= 39.0);
    // A: func_8083F72C (nothing in hand to put away): Player_Action_8084B78C, push_wait from its
    // first frame, still (func_80832224), turned off the wall (wallYaw + 0x8000: +x, 0x4000);
    // unk_3C4 the block.
    prev = frame(&mut w, prev, held(BTN_A));
    let p = w.player();
    assert_eq!((p.action, p.skel.animation, p.skel.cur_frame, p.linear_velocity), (PA::PushWait, w.data.anim("link_normal_push_wait"), 0.0, 0.0));
    assert_eq!(p.actor.shape_rot.y, 0x4000);
    assert_eq!(p.unk_3c4, Some(b));
    // Its first frame: PLAYER_STATE2_0, _6, _8; func_8083F524 keeps him wallCheckRadius + 5 off
    // the face (x -635), 26 up; STATE2_8 asks for CAM_MODE_PUSH_PULL.
    prev = frame(&mut w, prev, held(BTN_A));
    let p = w.player();
    assert_eq!(p.state2 & (STATE2_0 | STATE2_6 | STATE2_8), STATE2_0 | STATE2_6 | STATE2_8);
    assert!((p.actor.world_pos.x - (-635.0 - (p.age.wall_radius + 5.0))).abs() < 0.01, "Link at {:?}", p.actor.world_pos);
    assert_eq!(w.game_camera.mode, CAM_MODE_PUSH_PULL);
    // The stick forward, A held: once push_wait has played (LinkAnimation_Update true),
    // func_8083F9D0 (the same wall, A held, no STATE2_4: 0) and func_8083FFB8 (forward: 1):
    // func_8083FAB8, Player_Action_8084B898 with push_start and STATE2_4. The block, which
    // updates after Player (ACTORCAT_PROP), sees no push yet (unk_150 is added from the action's
    // next frame): ObjOshihiki_OnScene's else, STATE2_4 cleared.
    let mut n = 0;
    while w.player().action == PA::PushWait {
        prev = frame(&mut w, prev, fwd(BTN_A));
        n += 1;
        assert!(n < 30);
    }
    let p = w.player();
    assert_eq!((p.action, p.skel.animation, p.state2 & STATE2_4), (PA::Push, w.data.anim("link_normal_push_start"), 0));
    assert_eq!((get(&w, b).action, w.col.dyna.unk_150(bg)), (Some(BA::OnScene), 0.0));
    // Next: func_8083F9D0 again (A held, 0), forward: STATE2_4, func_8084B840: unk_150 2, unk_158
    // Link's world yaw 0x4000; Link at speed 2. The block: ObjOshihiki_StrongEnough (small),
    // no wall in the next 20 (ObjOshihiki_CheckWall): pushed, direction 2.
    prev = frame(&mut w, prev, fwd(BTN_A));
    let p = w.player();
    assert_eq!((p.state2 & STATE2_4, p.linear_velocity), (STATE2_4, 2.0));
    let o = get(&w, b);
    assert_eq!((o.action, o.direction, o.state_flags), (Some(BA::Push), 2.0, osh::PUSHBLOCK_SETUP_PUSH | osh::PUSHBLOCK_ON_SCENE));
    assert_eq!((w.col.dyna.unk_150(bg), w.col.dyna.unk_158(bg)), (2.0, 0x4000));
    // ObjOshihiki_Push: pushDist 0.5, 1.5, 3, 5, ... 19, 20 along +x (yawSin 1, yawCos 0), the
    // slide's sound every frame; unk_150 2 more each frame Player pushes.
    let slide = osh::NA_SE_EV_ROCK_SLIDE - SFX_FLAG;
    let want = [0.5f32, 1.5, 3.0, 5.0, 7.0, 9.0, 11.0, 13.0, 15.0, 17.0, 19.0];
    for (i, &d) in want.iter().enumerate() {
        prev = frame(&mut w, prev, fwd(BTN_A));
        let o = get(&w, b);
        assert_eq!((o.actor.world_pos.x, o.push_dist, o.action), (-605.0 + d, d, Some(BA::Push)), "push frame {i}");
        assert_eq!(w.col.dyna.unk_150(bg), 2.0 * (i as f32 + 2.0));
        assert!(sfx_on(&w, w.audio.frames, slide), "the slide's sound on push frame {i}");
    }
    // 20: stopped: home there, unk_150 and Player's STATE2_4 cleared, timer 10, at rest on the
    // channel's floor (the scene's: ObjOshihiki_SetupOnScene). No wall within 20 ahead: no
    // NA_SE_EV_BLOCK_BOUND.
    prev = frame(&mut w, prev, fwd(BTN_A));
    let o = get(&w, b);
    assert_eq!((o.actor.world_pos, o.actor.home_pos, o.timer, o.action), (Vec3::new(-585.0, -820.0, -290.0), Vec3::new(-585.0, -820.0, -290.0), 10, Some(BA::OnScene)));
    assert_eq!(w.col.dyna.unk_150(bg), 0.0);
    assert_eq!(w.player().state2 & STATE2_4, 0);
    assert!(sfx_on(&w, w.audio.frames, slide));
    assert_eq!(sfx_count(&w, osh::NA_SE_EV_BLOCK_BOUND), 0);
    // The wait: Player keeps pushing (STATE2_4 and 2 on unk_150 each frame, the stick forward),
    // the block refusing it while its timer is up (STATE2_4 and unk_150 cleared); the timer, 9
    // at the next update down to 1, reaches 0 on the tenth: the next push starts.
    for t in (1..=9).rev() {
        prev = frame(&mut w, prev, fwd(BTN_A));
        let o = get(&w, b);
        assert_eq!((o.timer, o.action, w.col.dyna.unk_150(bg)), (t, Some(BA::OnScene), 0.0));
        assert_eq!(w.player().state2 & STATE2_4, 0);
    }
    frame(&mut w, prev, fwd(BTN_A));
    let o = get(&w, b);
    assert_eq!((o.timer, o.action, o.actor.home_pos.x), (0, Some(BA::Push), -585.0));
    // Link said NA_SE_VO_LI_PUSH once, on push_start's frame 11 (av2.actionVar2 still 0), and
    // the push's slips (D_80854870: NA_SE_PL_SLIP on frames 3 and 21, ANIMSFX_TYPE_FLOOR) came
    // with the floor's offset.
    assert_eq!(sfx_count(&w, NA_SE_VO_LI_PUSH + w.player().age.climb.unk_92), 1);
    assert!(w.audio.log.as_ref().unwrap().sfx.iter().any(|&(_, s, _)| s >= NA_SE_PL_SLIP && s < NA_SE_PL_SLIP + 0x10));
}

/// A let go while holding on: `func_8083F9D0`'s fall-through: standing (`func_80839FFC`),
/// `gPlayerAnim_link_normal_push_wait_end`, STATE2_4 cleared.
#[test]
fn letting_go() {
    let Some(a) = assets() else { return };
    let mut w = room3(&a, &[], Vec3::new(-670.0, -810.0, -290.0));
    let mut prev = grab(&mut w);
    for _ in 0..3 {
        prev = frame(&mut w, prev, held(BTN_A));
    }
    // push_wait still playing: LinkAnimation_Update false, nothing decided. Once it's done,
    // A up: let go.
    while w.player().skel.animation == w.data.anim("link_normal_push_wait") && w.player().action == PA::PushWait {
        prev = frame(&mut w, prev, PadState::default());
    }
    let p = w.player();
    assert_eq!((p.action, p.skel.animation, p.state2 & STATE2_4), (PA::StandingStill, w.data.anim("link_normal_push_wait_end"), 0));
    let _ = prev;
}

/// The pull: one push out from the start (the block at -585), then the stick back with A: the
/// pull (`func_8083FB14`: `PLAYER_ANIMGROUP_pull_start`), the floor 40 behind Link 10 above his
/// (within 20) and no wall between at 26 up: -2 on the block a frame, which slides 20 back to
/// its start, `NA_SE_EV_BLOCK_BOUND` as it stops against the channel's step 49.5 behind it. Then
/// it won't come any further: the step is within its `ObjOshihiki_CheckWall` reach.
#[test]
fn pulling_the_block_back_to_its_start() {
    let Some(a) = assets() else { return };
    let mut w = room3(&a, &[], Vec3::new(-670.0, -810.0, -290.0));
    let b = block(&w);
    let bg = get(&w, b).bg;
    let mut prev = grab(&mut w);
    // One push.
    for _ in 0..80 {
        prev = frame(&mut w, prev, fwd(BTN_A));
        if get(&w, b).actor.home_pos.x == -585.0 && get(&w, b).action == Some(BA::OnScene) {
            break;
        }
    }
    assert_eq!(get(&w, b).actor.home_pos.x, -585.0);
    // The stick back: func_8083FFB8 -1 (the stick's angle off Link's facing past 90 degrees):
    // the pull.
    for _ in 0..40 {
        prev = frame(&mut w, prev, back(BTN_A));
        if w.player().action == PA::Pull {
            break;
        }
    }
    let p = w.player();
    assert_eq!((p.action, p.skel.animation), (PA::Pull, w.data.player_anim(0x23, p.model_anim_type)));
    let bounds = sfx_count(&w, osh::NA_SE_EV_BLOCK_BOUND);
    // The block takes it after its wait: direction -2, back along -x.
    for _ in 0..80 {
        prev = frame(&mut w, prev, back(BTN_A));
        if get(&w, b).action == Some(BA::Push) {
            break;
        }
    }
    let o = get(&w, b);
    assert!(o.direction < 0.0, "pulled: direction {}", o.direction);
    assert!(w.col.dyna.unk_150(bg) < 0.0);
    for _ in 0..12 {
        prev = frame(&mut w, prev, back(BTN_A));
    }
    let o = get(&w, b);
    assert_eq!((o.actor.world_pos.x, o.actor.home_pos.x, o.action), (-605.0, -605.0, Some(BA::OnScene)));
    assert_eq!(sfx_count(&w, osh::NA_SE_EV_BLOCK_BOUND), bounds + 1);
    // Pulling on: refused every frame (the step behind it), the block stays.
    for _ in 0..60 {
        prev = frame(&mut w, prev, back(BTN_A));
        assert_eq!(get(&w, b).actor.world_pos.x, -605.0);
    }
    assert_ne!(get(&w, b).action, Some(BA::Push));
}

/// The block off the channel's end: from 20 short of it (put there), a push as Player gives it
/// (`func_8002DFA4`, 2 a frame). At `pushDist` 19 its west corners are still over the channel
/// (x -395.99); at 20 all five points are over the pit (-394.99): `ObjOshihiki_CheckFloor` fails,
/// `ObjOshihiki_SetupFall` (home there, the push dropped). Falling with gravity -1 (1.5, 4.5,
/// 9 ... down: `Actor_MoveXZGravity`'s 1.5), it lands on the pit's floor (-905) on the 11th frame
/// (82.5 is short of 85), with `NA_SE_EV_BLOCK_BOUND` and the floor's footstep. That frame
/// `Obj_Makeoshihiki`'s draw finds it at `sBlocks[1]`'s second place: flag 0x10 set (0x3F
/// cleared), the chime (`unk_24[1]` & 1, the flag changed), and the block immovable (`& 2`).
#[test]
fn pushed_off_the_channels_end_into_the_pit() {
    let Some(a) = assets() else { return };
    let mut w = room3(&a, &[], Vec3::new(-700.0, -810.0, -400.0));
    let b = block(&w);
    let bg = get(&w, b).bg;
    {
        let o = w.actors.downcast_mut::<ObjOshihiki>(b).unwrap();
        o.actor.world_pos.x = -385.0;
        o.actor.home_pos.x = -385.0;
    }
    idle(&mut w, 2);
    assert_eq!((get(&w, b).actor.world_pos, get(&w, b).action), (Vec3::new(-385.0, -820.0, -290.0), Some(BA::OnScene)));
    let chimes = sfx_count(&w, NA_SE_SY_TRE_BOX_APPEAR);
    w.col.dyna.func_8002DFA4(bg, 2.0, 0x4000);
    let mut frames = 0;
    while get(&w, b).action != Some(BA::Fall) {
        if get(&w, b).action == Some(BA::Push) {
            w.col.dyna.func_8002DFA4(bg, 2.0, 0x4000);
        }
        idle(&mut w, 1);
        frames += 1;
        assert!(frames < 20);
        assert!(!w.flags.get_switch(0x10));
    }
    let o = get(&w, b);
    assert_eq!((o.actor.world_pos, o.actor.home_pos.x, o.push_dist, w.col.dyna.unk_150(bg)), (Vec3::new(-365.0, -820.0, -290.0), -365.0, 0.0, 0.0));
    assert_eq!(o.actor.floor_height, -905.0);
    for i in 1..=10 {
        idle(&mut w, 1);
        let o = get(&w, b);
        assert_eq!(o.action, Some(BA::Fall), "fall frame {i}");
        // Σ k·1.5 for k = 1..i.
        assert!((o.actor.world_pos.y - (-820.0 - 1.5 * (i * (i + 1) / 2) as f32)).abs() < 0.01, "fall frame {i}: {}", o.actor.world_pos.y);
    }
    assert!(!w.flags.get_switch(0x10));
    idle(&mut w, 1);
    let o = get(&w, b);
    assert_eq!((o.actor.world_pos, o.action), (Vec3::new(-365.0, -905.0, -290.0), Some(BA::OnScene)));
    assert!(sfx_on(&w, w.audio.frames, osh::NA_SE_EV_BLOCK_BOUND));
    assert!(w.flags.get_switch(0x10));
    assert!(!w.flags.get_switch(0x3F));
    assert!(o.cant_move);
    assert_eq!(sfx_count(&w, NA_SE_SY_TRE_BOX_APPEAR), chimes + 1);
    // Resting there the flags don't change: no more chimes; a push is refused (cantMove).
    for _ in 0..20 {
        w.col.dyna.func_8002DFA4(bg, 2.0, 0x4000);
        idle(&mut w, 1);
        assert_eq!(get(&w, b).action, Some(BA::OnScene));
        assert_eq!(w.col.dyna.unk_150(bg), 0.0);
    }
    assert_eq!(sfx_count(&w, NA_SE_SY_TRE_BOX_APPEAR), chimes + 1);
}

/// `Obj_Makeoshihiki`'s init with flag 0x10 set: `typeIdx` 1, the block spawned in the pit as its
/// child, immovable (`unk_24[1]` & 2); with it clear, on the upper floor (0), movable. Navi's
/// hint by the block (`Elf_Msg` 0x3208, rot.y 17: gone once flag 0x10 is set) is gone with it.
#[test]
fn the_block_spawns_where_its_flags_say() {
    let Some(a) = assets() else { return };
    let w = room3(&a, &[], Vec3::new(-700.0, -810.0, -400.0));
    let b = block(&w);
    let maker = w.actors.all().into_iter().find(|&h| w.actors.downcast::<ObjMakeoshihiki>(h).is_some()).expect("Obj_Makeoshihiki");
    assert_eq!(w.actors.actor(maker).unwrap().child, Some(b));
    assert_eq!(w.actors.actor(b).unwrap().parent, Some(maker));
    let o = get(&w, b);
    // ((0xFF << 6) & 0xC0) | PUSHBLOCK_SMALL_START_ON | 0xFF00.
    assert_eq!((o.actor.params as u16, o.actor.world_pos, o.cant_move, o.actor.scale.x), (0xFFC0, Vec3::new(-605.0, -820.0, -290.0), false, 0.1));
    assert_eq!((o.texture, o.color), (Some(osh::Texture::Silver), [110, 86, 40]));
    let w = room3(&a, &[0x10], Vec3::new(-700.0, -810.0, -400.0));
    let o = get(&w, block(&w));
    assert_eq!((o.actor.world_pos, o.cant_move), (Vec3::new(-365.0, -905.0, -290.0), true));
    let hint = w.actors.all().into_iter().any(|h| w.actors.actor(h).is_some_and(|a| a.id == oot_actors::elf_msg::ACTOR_ELF_MSG && a.params as u16 == 0x3208 && !a.killed));
    assert!(!hint);
}

/// `ObjOshihiki_Init`'s flag (params bits 8..15, 0..0x3F): a "start on" block isn't there with it
/// set, a "start off" one only with it set; the strength each size needs
/// (`ObjOshihiki_StrongEnough`): a large block takes a child's push only with a strength upgrade
/// (the bracelet's, `Player_GetStrength`).
#[test]
fn spawned_blocks_flags_and_strength() {
    let Some(a) = assets() else { return };
    let mut w = room3(&a, &[], Vec3::new(-700.0, -810.0, -400.0));
    // The pit's open floor (-905), clear of the room's block and of walls within the large
    // block's reach along +z (`ObjOshihiki_CheckWall`: 300 × 0.2 + 20 - 0.5 = 79.5 from its middle).
    let at = Vec3::new(-300.0, -905.0, -350.0);
    let spawn = |w: &mut PlayState, params: u16| -> ActorHandle {
        let h = w.actor_spawn(ACTOR_OBJ_OSHIHIKI, at, [0; 3], params as i16).expect("spawned");
        idle(w, 1);
        h
    };
    let alive = |w: &PlayState, h: ActorHandle| w.actors.actor(h).is_some_and(|a| !a.killed);
    // Small, start on, flag 5: there with 5 clear, gone with it set.
    let h = spawn(&mut w, 0x0500);
    assert!(alive(&w, h));
    w.actors.actor_mut(h).unwrap().kill();
    idle(&mut w, 1);
    w.flags.set_switch(5);
    let h = spawn(&mut w, 0x0500);
    assert!(!alive(&w, h));
    // Small, start off: there only with the flag set.
    let h = spawn(&mut w, 0x0504);
    assert!(alive(&w, h));
    w.actors.actor_mut(h).unwrap().kill();
    idle(&mut w, 1);
    w.flags.unset_switch(5);
    let h = spawn(&mut w, 0x0504);
    assert!(!alive(&w, h));
    // Large (scale 0.2, gPushBlockBaseTex), no flag (0xFF): pushed along +z as Player would. No
    // strength upgrade: refused (unk_150 cleared every frame).
    assert_eq!(player_get_strength(&w.save), PLAYER_STR_NONE);
    let h = spawn(&mut w, 0xFF02);
    idle(&mut w, 30);
    let o = get(&w, h);
    let bg = o.bg;
    assert_eq!((o.actor.scale.x, o.texture), (0.2, Some(osh::Texture::Base)));
    for _ in 0..15 {
        w.col.dyna.func_8002DFA4(bg, 2.0, 0);
        idle(&mut w, 1);
        assert_ne!(get(&w, h).action, Some(BA::Push));
        assert_eq!(w.col.dyna.unk_150(bg), 0.0);
    }
    // A child with the Goron's Bracelet (UPG_STRENGTH 1): PLAYER_STR_BRACELET, pushed.
    w.save.inventory_change_upgrade(oot_game::item::UPG_STRENGTH, 1);
    assert_eq!(player_get_strength(&w.save), PLAYER_STR_BRACELET);
    w.col.dyna.func_8002DFA4(bg, 2.0, 0);
    idle(&mut w, 1);
    assert_eq!(get(&w, h).action, Some(BA::Push));
    // Player_GetStrength: an adult's is the upgrade itself.
    w.save.adult = true;
    w.save.inventory_change_upgrade(oot_game::item::UPG_STRENGTH, 3);
    assert_eq!(player_get_strength(&w.save), PLAYER_STR_GOLD_G);
}

/// A block resting on another moves with it (`ObjOshihiki_MoveWithBlockUnder`, which the C runs
/// in the draw: here `draw_update`): its highest floor is the lower block's (`GetBlockUnder`);
/// while that one takes a push (`PUSHBLOCK_SETUP_PUSH`, no wall in the top one's way) it's
/// `blockUnder`; each frame it slides (`PUSHBLOCK_PUSH`) the top one moves by as much, its home
/// stepped after it by 20s.
#[test]
fn a_block_on_a_block_rides_along() {
    let Some(a) = assets() else { return };
    let mut w = room3(&a, &[], Vec3::new(-700.0, -810.0, -400.0));
    let bottom = block(&w);
    let bg = get(&w, bottom).bg;
    // A small block dropped onto it (no flag: params 0xFF00), landing on its top (-760).
    let top = w.actor_spawn(ACTOR_OBJ_OSHIHIKI, Vec3::new(-605.0, -700.0, -290.0), [0; 3], 0xFF00u16 as i16).expect("spawned");
    for _ in 0..40 {
        idle(&mut w, 1);
    }
    let t = get(&w, top);
    assert_eq!((t.actor.world_pos, t.action, t.floor_bg_ids[t.highest_floor as usize]), (Vec3::new(-605.0, -760.0, -290.0), Some(BA::OnActor), bg));
    w.col.dyna.func_8002DFA4(bg, 2.0, 0x4000);
    let mut xs = Vec::new();
    // ObjOshihiki_SetupPush, then the twelve frames of the slide to 20.
    for _ in 0..13 {
        if get(&w, bottom).action == Some(BA::Push) || get(&w, bottom).action == Some(BA::OnScene) {
            w.col.dyna.func_8002DFA4(bg, 2.0, 0x4000);
        }
        idle(&mut w, 1);
        xs.push((get(&w, bottom).actor.world_pos.x, get(&w, top).actor.world_pos.x));
    }
    for &(xb, xt) in &xs {
        assert_eq!(xb, xt, "the top block rides along: {xs:?}");
    }
    assert_eq!(get(&w, bottom).actor.world_pos.x, -585.0);
    let t = get(&w, top);
    assert_eq!((t.actor.home_pos.x, t.block_under), (-585.0, Some(bottom)));
}

/// Inside the Deku Tree in room 7 (`deku-tree-inside`), Link at `at` facing +z, the enemies gone.
fn room7(a: &Arc<GameAssets>, at: Vec3) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-inside").unwrap();
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, oot_game::audio::GameAudio::default()).expect("Play_Init");
    idle(&mut w, 2);
    oot_actors::playthrough::deku_tree_room_start(&mut w, 7, at, 0);
    for h in w.actors.category(oot_game::actor_ctx::ACTORCAT_ENEMY).to_vec() {
        if let Some(a) = w.actors.actor_mut(h) {
            a.kill();
        }
    }
    idle(&mut w, 30);
    w
}

/// Walks Link (facing +z) forward at a walk for up to `n` frames, until "Grab" shows.
fn walk_until_grab(w: &mut PlayState, n: usize) -> PadState {
    let mut prev = PadState::default();
    for _ in 0..n {
        if w.player().state2 & STATE2_0 != 0 {
            break;
        }
        prev = frame(w, prev, PadState { button: 0, stick_x: 0, stick_y: 40 });
    }
    prev
}

/// Master Quest's room 7 stones are sunk 15 into the floor (their bottom at -775, the floor at
/// -760): the front of (-2079, 277) stands 34 over Link's floor, its top sloping down from there
/// (-726 to -757), and Player's ledge check, a little way into it, finds it 28 up.
/// `Player_ActionHandler_5` wants a wall 39 high (`yDistToLedge`): no "Grab", and walking on, Link
/// climbs onto it. In Master Quest they're pulled only by injection.
#[test]
fn room_7s_sunk_gravestones_are_too_low_to_hold() {
    let Some(a) = assets() else { return };
    let mut w = room7(&a, Vec3::new(-2079.0, -760.0, 200.0));
    let mut seen_wall = None;
    let mut prev = PadState::default();
    for _ in 0..60 {
        prev = frame(&mut w, prev, PadState { button: 0, stick_x: 0, stick_y: 40 });
        let p = w.player();
        assert_eq!(p.state2 & STATE2_0, 0, "Grab at {:?}", p.actor.world_pos);
        if p.actor.bg_check_flags & oot_game::actor::BGCHECKFLAG_PLAYER_WALL_INTERACT != 0 && seen_wall.is_none() {
            seen_wall = Some(p.wall_height);
        }
    }
    let h = seen_wall.expect("Link walked into the stone");
    assert!(h > 0.0 && h < 39.0, "the stone's front {h} over his floor");
    // He's over it now, the stone where it was.
    let home = Vec3::new(-2079.0, -775.0, 277.0);
    let stone = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<BgHaka>(h).filter(|s| s.actor.home_pos.distance(home) < 1.0)).expect("the gravestone");
    assert_eq!((stone.action, stone.actor.world_pos), (HakaAction::IdleClosed, home));
    assert!(w.player().actor.world_pos.z > 240.0);
}

/// A gravestone flush with the floor, as the graveyard places them (spawned here on room 7's
/// floor at -760: its front 48 high), pulled by Player: from its front (its local -z side, facing
/// +z), A, then the stick back: the pull (`func_8083FB14`), -2 a frame on `unk_150`.
/// `BgHaka_IdleClosed` takes only a pull (a push is refused), turns its slide to its yaw + 0x8000
/// (-z) and slides 60 (`BgHaka_Pull`), Link backing off in front of it; at 60 it clears Player's
/// `PLAYER_STATE2_4` and the pull, and takes none again (`BgHaka_IdleOpened`).
#[test]
fn link_pulls_a_gravestone() {
    let Some(a) = assets() else { return };
    let mut w = room7(&a, Vec3::new(-1990.0, -760.0, 100.0));
    let home = Vec3::new(-1990.0, -760.0, 200.0);
    let stone = w.actor_spawn(oot_actors::bg_haka::ACTOR_BG_HAKA, home, [0; 3], 0).expect("a gravestone");
    idle(&mut w, 2);
    let mut prev = walk_until_grab(&mut w, 120);
    assert_ne!(w.player().state2 & STATE2_0, 0, "no Grab at the stone (Link at {:?}, wall {})", w.player().actor.world_pos, w.player().wall_height);
    for _ in 0..20 {
        prev = frame(&mut w, prev, PadState::default());
    }
    for _ in 0..10 {
        prev = frame(&mut w, prev, held(BTN_A));
        if w.player().action == PA::PushWait {
            break;
        }
    }
    assert_eq!((w.player().action, w.player().unk_3c4), (PA::PushWait, Some(stone)));
    // A push first: refused (BgHaka_IdleClosed's `0.0f < unk_150`), nothing moves.
    for _ in 0..30 {
        prev = frame(&mut w, prev, fwd(BTN_A));
        assert_eq!(w.actors.downcast::<BgHaka>(stone).unwrap().actor.world_pos, home);
    }
    // The stick back: the pull, the stone sliding.
    let mut frames = 0;
    while w.actors.downcast::<BgHaka>(stone).unwrap().action != HakaAction::IdleOpened {
        prev = frame(&mut w, prev, back(BTN_A));
        frames += 1;
        assert!(frames < 200, "the stone never opened (Link {:?} at {:?}, stone {:?})", w.player().action, w.player().actor.world_pos, w.actors.downcast::<BgHaka>(stone).unwrap().action);
    }
    let s = w.actors.downcast::<BgHaka>(stone).unwrap();
    assert_eq!(s.actor.min_velocity_y, 60.0);
    assert!((s.actor.world_pos - (home - Vec3::Z * 60.0)).length() < 0.01, "the stone at {:?}", s.actor.world_pos);
    // Opened: no more pulls taken. The stone (BG) updates before Player: each frame it zeroes the
    // pull and clears his PLAYER_STATE2_4, then Player, still pulling, sets it again and adds this
    // frame's -2: it never adds up again.
    let bg = s.bg;
    for _ in 0..30 {
        prev = frame(&mut w, prev, back(BTN_A));
        assert_eq!(w.actors.downcast::<BgHaka>(stone).unwrap().actor.world_pos.z, home.z - 60.0);
        assert_eq!(w.col.dyna.unk_150(bg), -2.0);
    }
}
