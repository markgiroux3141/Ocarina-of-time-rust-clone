//! GAME-10 milestone 9 against the C: the frogs on Zora's River's log (`z_en_fr.c`) and one-point
//! cutscene 4110 (`z_onepointdemo.c`). Zora's River's room 0, layers 0 and 1, places the song spot
//! (params 0) at (990, 205, -1220) yaw 5461 and the five frogs: params 1 at (1093, 205, -1069),
//! 2 at (1098, 194, -1110), 3 at (1082, 199, -1028), 4 at (1169, 226, -1021), 5 at (1122, 230,
//! -980) (`ootx scene-info --scene spot03 --layer 0`).
//!
//! Child Link enters at `ENTR_ZORAS_RIVER_0` (room 0). The tests then run the six actors' updates
//! alone (`Actor_UpdateAll`'s calls: the frogs, then the spot), driving the spot through the
//! message box's fields as its modes would leave them (`ocarinaMode`, `msgMode`,
//! `lastOcarinaButtonIndex`); no song is played on the pad and nothing waits on `Rand` (the song
//! game's fixed first song). Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_B, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_fr::*;
use oot_actors::player::STATE2_25;
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorHandle, PLAYER_STATE1_28, PLAYER_STATE1_29};
use oot_game::audio::ocarina::{OCARINA_BTN_A, OCARINA_BTN_C_DOWN, OCARINA_BTN_C_LEFT};
use oot_game::audio::sfx::{NA_SE_SY_CORRECT_CHIME, NA_SE_SY_OCARINA_ERROR, NA_SE_SY_TRE_BOX_APPEAR, SfxPos};
use oot_game::effect::EFFECT_SS_G_SPLASH;
use oot_game::env::clock_time;
use oot_game::item::{GI_HEART_PIECE, GI_NONE, GI_RUPEE_BLUE, GI_RUPEE_PURPLE};
use oot_game::message::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// The song spot's placement.
const SPOT: Vec3 = Vec3::new(990.0, 205.0, -1220.0);
const SPOT_YAW: i16 = 5461;
/// The frogs' placements, by `frogIndex` (params 1 to 5).
const LOG: [Vec3; 5] =
    [Vec3::new(1093.0, 205.0, -1069.0), Vec3::new(1098.0, 194.0, -1110.0), Vec3::new(1082.0, 199.0, -1028.0), Vec3::new(1169.0, 226.0, -1021.0), Vec3::new(1122.0, 230.0, -980.0)];

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// Child Link at the river's mouth with `setup`'s save, the sound log on, until he's his own.
fn river(a: &Arc<GameAssets>, setup: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_ZORAS_RIVER_0").expect("entrance");
    let mut save = SaveContext::new(e, false, clock_time(10, 0) as u16);
    save.apply_preset("fairy-magic").unwrap();
    setup(&mut save);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    assert_eq!(w.room_ctx.cur.num, 0);
    for _ in 0..200 {
        if w.player().state1 & (PLAYER_STATE1_28 | PLAYER_STATE1_29) == 0 {
            break;
        }
        idle(&mut w, 1);
    }
    idle(&mut w, 1);
    w
}

fn fr(w: &PlayState, h: ActorHandle) -> &EnFr {
    w.actors.downcast::<EnFr>(h).expect("En_Fr")
}

fn fr_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnFr {
    w.actors.downcast_mut::<EnFr>(h).expect("En_Fr")
}

/// The En_Fr of params `params`.
fn find(w: &PlayState, params: i16) -> ActorHandle {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnFr>(h).is_some_and(|f| !f.actor.killed && f.actor.params == params)).expect("the En_Fr")
}

fn spot(w: &PlayState) -> ActorHandle {
    find(w, 0)
}

/// The frogs by `frogIndex`.
fn frogs(w: &PlayState) -> [ActorHandle; 5] {
    std::array::from_fn(|i| find(w, i as i16 + 1))
}

fn statics(w: &mut PlayState) -> &mut Statics {
    oot_actors::en_fr::statics(w)
}

/// The actor's update alone (`Actor_UpdateAll`'s call), as the updating actor.
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// One frame of the six in `Actor_UpdateAll`'s order: the frogs (`ACTORCAT_NPC`, newest first:
/// params 5, 2, 4, 3, 1, the room's list backwards), then the spot (`ACTORCAT_PROP`); Link on the
/// spot as `Actor_UpdateAll` measures him from it.
fn frame(w: &mut PlayState) {
    let s = spot(w);
    for h in w.actors.all() {
        if h != s && w.actors.downcast::<EnFr>(h).is_some() {
            update(w, h);
        }
    }
    let a = &mut fr_mut(w, s).actor;
    a.xz_dist_to_player = 0.0;
    a.y_dist_to_player = 0.0;
    update(w, s);
}

/// Frames until the spot's action isn't `action` (at most `max`); the frames run.
fn spot_until_not(w: &mut PlayState, action: Action, max: usize) -> usize {
    let s = spot(w);
    for n in 1..=max {
        frame(w);
        if fr(w, s).action != action {
            return n;
        }
    }
    panic!("the spot still {action:?} after {max}");
}

/// The times a sound was asked for at `pos`.
fn sfx_at(w: &PlayState, id: u16, pos: SfxPos) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, p)| s == id && p == pos).count()
}

/// The times a sound was asked for anywhere.
fn sfx_any(w: &PlayState, id: u16) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).count()
}

/// Link's ocarina out on the spot (`PLAYER_STATE2_25`, Player's own the frame after), then
/// `come_out`.
fn out_on_the_log(w: &mut PlayState) -> Vec<usize> {
    let s = spot(w);
    w.player_mut().state2 |= STATE2_25;
    update(w, s);
    assert_eq!(fr(w, s).action, Action::Activate);
    w.player_mut().state2 &= !STATE2_25;
    come_out(w)
}

/// The frogs out of the water and all five sitting on the log, the spot past
/// `EnFr_ActivateCheckFrogSong` (at `flags` 11) and `func_80A1BE98`. Returns the order they
/// jumped out in.
///
/// @bug (game): `EnFr_JumpingOutOfWater` bumps `flags` every frame a frog holds the jump's frame
/// 6 (`playSpeed` 0 until it's falling within 40 of its spot: four frames), not once, so the
/// frogs come out overlapping, each catching its `sTimerJumpingOutOfWater` on the way, and
/// `flags` runs on past 12 (26 here) while the spot goes on at 11.
fn come_out(w: &mut PlayState) -> Vec<usize> {
    let s = spot(w);
    let fs = frogs(w);
    let mut order = Vec::new();
    let mut at = None;
    for _ in 0..600 {
        let before = fr(w, s).action;
        frame(w);
        if before == Action::ActivateCheckFrogSong && fr(w, s).action != before {
            at = Some(statics(w).flags);
        }
        for (i, &h) in fs.iter().enumerate() {
            if fr(w, h).action == Action::JumpingOutOfWater && !order.contains(&i) {
                order.push(i);
            }
        }
        let sitting = fs.iter().all(|&h| fr(w, h).action == Action::ChooseJumpFromLogSpot);
        if sitting && !matches!(fr(w, s).action, Action::Activate | Action::ActivateCheckFrogSong | Action::Func80a1be98) {
            break;
        }
    }
    assert_eq!(at, Some(11), "the spot went on at flags 11");
    assert!(fs.iter().all(|&h| fr(w, h).action == Action::ChooseJumpFromLogSpot));
    order
}

#[test]
fn the_spot_and_the_frogs_set_up_under_the_water() {
    let Some(a) = assets() else { return };
    let mut w = river(&a, |_| {});
    let s = spot(&w);
    let fs = frogs(&w);
    // EnFr_Init, params 0: a prop, its update EnFr_UpdateIdle, no draw, no lock-on, culled, text
    // 0x40AC, EnFr_Idle.
    let sp = fr(&w, s);
    assert_eq!((sp.actor.category, sp.update_fn, sp.has_draw, sp.action, sp.actor.text_id), (ACTORCAT_PROP, UpdateFn::UpdateIdle, false, Action::Idle, 0x40AC));
    assert_eq!(sp.actor.flags & (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_UPDATE_CULLING_DISABLED), 0);
    // EnFr_Update (gameplay_field_keep loaded): each frog in sEnFrPointers.frogs[params - 1].
    assert_eq!(statics(&mut w).frogs, fs.map(Some));
    assert_eq!(statics(&mut w).flags, 0);
    // EnFr_OrientUnderwater: 80 out at sLogSpotToFromWater's yaw, 80 below; facing the log
    // (RAD_TO_BINANG(yaw) + 0x8000: -12288, -16384, -8192, 28672, 16384 turned round).
    let yaws: [i16; 5] = [20480, 16384, 24576, -4096, -16384];
    for (i, &h) in fs.iter().enumerate() {
        let f = fr(&w, h);
        assert_eq!(f.update_fn, UpdateFn::UpdateActive);
        assert_eq!(f.action, Action::SetupJumpingOutOfWater, "frog {i}");
        assert_eq!(f.pos_log_spot, LOG[i]);
        let yaw = S_LOG_SPOT_TO_FROM_WATER[i + 1].1;
        let expect = Vec3::new(LOG[i].x + 80.0 * yaw.sin(), LOG[i].y - 80.0, LOG[i].z + 80.0 * yaw.cos());
        assert!((f.actor.world_pos - expect).length() < 0.01, "frog {i}: {} vs {expect}", f.actor.world_pos);
        assert_eq!((f.actor.world_rot.y, f.actor.shape_rot.y), (yaws[i], yaws[i]));
        assert_eq!((f.xz_dist_to_log_spot, f.actor.gravity, f.actor.min_velocity_y), (80.0, 0.0, -9999.0));
        // 150 (no song played yet), a ten-thousandth.
        assert_eq!((f.scale, f.actor.scale.x), (150.0, 0.015));
        // EnFr_DrawIdle: only the purple frog drawn under the water.
        assert_eq!(f.has_draw, i == FROG_PURPLE, "frog {i}");
        assert_eq!((f.song_index, f.growing_scale_index, f.is_active), (FROG_NO_SONG, 0, false));
        assert_eq!(f.actor.flags & (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_UPDATE_CULLING_DISABLED), 0);
        assert!(f.light_node.is_some());
        assert_eq!((f.actor.target_mode, f.actor.target_arrow_offset), (2, 30.0));
    }
}

#[test]
fn a_frog_whose_song_was_played_starts_grown() {
    let Some(a) = assets() else { return };
    // sFrogToFrogSongIndex: the yellow frog's is Saria's Song.
    let w = river(&a, |s| s.set_event_chk_inf(EVENTCHKINF_SONGS_FOR_FROGS_SARIA));
    let fs = frogs(&w);
    assert_eq!(fr(&w, fs[FROG_YELLOW]).scale, 270.0);
    assert_eq!(fr(&w, fs[FROG_YELLOW]).actor.scale.x, 270.0 * 0.0001);
    assert_eq!(fr(&w, fs[FROG_BLUE]).scale, 150.0);
}

#[test]
fn the_ocarina_on_the_spot_starts_4110_and_the_frogs_come_out() {
    use oot_game::onepoint::CAM_SET_CS_C;
    let Some(a) = assets() else { return };
    let mut w = river(&a, |_| {});
    // z_onepointdemo.c's D_8012269C: (actionFlags, initField, initFlags, timer, roll, fov, lerp,
    // at, eye); ONEPOINT_CS_INIT_FIELD_NONE 0xFF.
    let row = |w: &PlayState, i: usize| {
        let k = w.onepoint.keyframes[w.onepoint.table("D_8012269C")][i];
        (k.action_flags, k.init_field, k.init_flags, k.timer_init, k.roll_target_init, k.fov_target_init, k.lerp_step_scale, k.at_target_init, k.eye_target_init)
    };
    assert_eq!(w.onepoint.len(w.onepoint.table("D_8012269C")), 3);
    assert_eq!(row(&w, 0), (15, 0xFF, 0x0101, 20, 2, 45.0, 1.0, Vec3::new(975.0, 225.0, -1195.0), Vec3::new(918.0, 228.0, -1228.0)));
    assert_eq!(row(&w, 1), (16, 0xFF, 0x0000, 1, 0, 60.0, 1.0, Vec3::splat(-1.0), Vec3::splat(-1.0)));
    assert_eq!(row(&w, 2), (17, 0xFF, 0x0000, 1, -1, -1.0, -1.0, Vec3::splat(-1.0), Vec3::splat(-1.0)));
    let s = spot(&w);
    // EnFr_Idle: Link above the spot within 30 across: his ocarina turned to it (unk_6A8).
    w.player_mut().actor.world_pos = SPOT + Vec3::new(20.0, 0.0, 20.0);
    update(&mut w, s);
    assert_eq!(w.player().unk_6a8, Some(s));
    assert!(is_above_and_within_30_dist_xz(SPOT + Vec3::new(21.0, 0.0, 21.0), SPOT));
    assert!(!is_above_and_within_30_dist_xz(SPOT + Vec3::new(22.0, 0.0, 22.0), SPOT));
    assert!(!is_above_and_within_30_dist_xz(SPOT - Vec3::new(0.0, 1.0, 0.0), SPOT));
    // The ocarina out there (PLAYER_STATE2_25), the ocarina's mode 4 cleared: one-point 4110
    // (D_8012269C's three keyframes, CAM_SET_CS_C; Player_SetCsAction PLAYER_CSACTION_8), the box
    // paused, Link on the spot facing its way, no reward yet.
    w.msg_ctx.ocarina_mode = OCARINA_MODE_04;
    w.player_mut().state2 |= STATE2_25;
    update(&mut w, s);
    assert_eq!(fr(&w, s).action, Action::Activate);
    assert_eq!(w.msg_ctx.ocarina_mode, OCARINA_MODE_00);
    assert_eq!(w.msg_ctx.msg_mode, MSGMODE_PAUSED);
    let cam = w.sub_cameras.iter().flatten().find(|c| c.cs_id == 4110).expect("one-point 4110's camera");
    assert_eq!((cam.setting, cam.cs_info.key_frame_cnt), (CAM_SET_CS_C, 3));
    assert_eq!(cam.cs_info.key_frames.map(|k| (k.table, k.start)), Some((w.onepoint.table("D_8012269C"), 0)));
    assert_eq!(w.player().cs_mode, 8);
    let p = w.player();
    assert_eq!((p.actor.world_pos, p.actor.world_rot.y, p.actor.shape_rot.y, p.current_yaw), (SPOT, SPOT_YAW, SPOT_YAW, SPOT_YAW));
    assert_eq!(fr(&w, s).reward, GI_NONE);
    // EnFr_Activate: the box paused, flags 1.
    w.player_mut().state2 &= !STATE2_25;
    update(&mut w, s);
    assert_eq!((fr(&w, s).action, statics(&mut w).flags), (Action::ActivateCheckFrogSong, 1));
    // The blue frog's turn (sTimerJumpingOutOfWater 1): the jump (object_fr_Anim_0007BC), drawn.
    let fs = frogs(&w);
    update(&mut w, fs[FROG_BLUE]);
    let b = fr(&w, fs[FROG_BLUE]);
    assert_eq!((b.action, b.has_draw, b.skel.cur_frame), (Action::JumpingOutOfWater, true, 1.0));
    // Frames 1 and 2 (its action, then SkelAnime_Update); at frame 3, 47 up with gravity -10:
    // 37 (Actor_MoveXZGravity), up 37 by 1.5 (Actor_UpdatePos' R_UPDATE_RATE * 0.5), and 10 of
    // the 80 closer.
    update(&mut w, fs[FROG_BLUE]);
    update(&mut w, fs[FROG_BLUE]);
    let y0 = fr(&w, fs[FROG_BLUE]).actor.world_pos.y;
    update(&mut w, fs[FROG_BLUE]);
    let b = fr(&w, fs[FROG_BLUE]);
    assert_eq!((b.actor.velocity.y, b.xz_dist_to_log_spot), (37.0, 70.0));
    assert_eq!(b.actor.world_pos.y, y0 + 37.0 * 1.5);
    // Then blue, purple, yellow, red, white (sTimerJumpingOutOfWater 1, 3, 5, 7, 9; overlapping:
    // `come_out`'s bug), each landing on its log spot and turning to Link; at 11, the free play
    // (OCARINA_ACTION_CHECK_NOWARP) once all five sit (func_80A1BE98).
    let order = come_out(&mut w);
    assert_eq!(order, vec![FROG_BLUE, FROG_PURPLE, FROG_YELLOW, FROG_RED, FROG_WHITE]);
    assert_eq!(fr(&w, s).action, Action::ListeningToOcarinaNotes);
    assert_eq!(fr(&w, s).song_index, FROG_ZL);
    assert_eq!(w.msg_ctx.ocarina_action, OCARINA_ACTION_CHECK_NOWARP);
    for (i, &h) in fs.iter().enumerate() {
        let f = fr(&w, h);
        assert_eq!((f.action, f.has_draw), (Action::ChooseJumpFromLogSpot, true), "frog {i}");
        assert_eq!(f.actor.world_pos.y, LOG[i].y, "frog {i}");
        assert_eq!(f.xz_dist_to_log_spot, 0.0, "frog {i}");
        // Out of the water: a splash and NA_SE_EV_DIVE_INTO_WATER_L at it.
        assert!(sfx_at(&w, NA_SE_EV_DIVE_INTO_WATER_L, SfxPos::Actor(h)) >= 1, "frog {i}");
    }
    assert!(w.effect_ss.table.iter().any(|e| e.ty == EFFECT_SS_G_SPLASH));
}

/// The free play on: the frogs out, the spot listening.
fn listening(w: &mut PlayState) {
    let order = out_on_the_log(w);
    assert_eq!(order, vec![FROG_BLUE, FROG_PURPLE, FROG_YELLOW, FROG_RED, FROG_WHITE]);
    assert_eq!(fr(w, spot(w)).action, Action::ListeningToOcarinaNotes);
}

/// The text read (`EnFr_TalkBeforeReward`'s `TEXT_STATE_EVENT` and A, as the box would give it),
/// then `EnFr_SetReward`.
fn read_text_and_set_reward(w: &mut PlayState) {
    let s = spot(w);
    fr_mut(w, s).action = Action::SetReward;
    update(w, s);
    assert_eq!(statics(w).flags, 12);
    assert_eq!(fr(w, s).action, Action::Deactivate);
}

/// The frogs back under the water (`EnFr_Deactivate`): the ocarina's mode 4, the cry, and the
/// reward offered to Link on the spot.
fn back_in(w: &mut PlayState) {
    let s = spot(w);
    let cries = sfx_at(w, NA_SE_EV_FROG_CRY_0, SfxPos::Actor(s));
    let n = spot_until_not(w, Action::Deactivate, 600);
    assert!(n >= 10, "{n}");
    assert_eq!(w.msg_ctx.ocarina_mode, OCARINA_MODE_04);
    assert_eq!(sfx_at(w, NA_SE_EV_FROG_CRY_0, SfxPos::Actor(s)), cries + 1);
    for (i, h) in frogs(w).into_iter().enumerate() {
        let f = fr(w, h);
        // Back under (EnFr_JumpingBackIntoWater, EnFr_OrientUnderwater), isDeactivating cleared.
        assert_eq!((f.action, f.is_deactivating, f.has_draw), (Action::SetupJumpingOutOfWater, false, i == FROG_PURPLE), "frog {i}");
        assert_eq!(f.actor.world_pos.y, LOG[i].y - 80.0, "frog {i}");
    }
}

#[test]
fn a_child_song_the_first_time_grows_its_frog_for_a_purple_rupee() {
    let Some(a) = assets() else { return };
    let mut w = river(&a, |_| {});
    listening(&mut w);
    let s = spot(&w);
    let fs = frogs(&w);
    // Saria's Song (OCARINA_MODE_05): FROG_SARIA, 48 frames of jumps.
    w.msg_ctx.ocarina_mode = OCARINA_MODE_05;
    update(&mut w, s);
    assert_eq!((fr(&w, s).action, fr(&w, s).song_index, fr(&w, s).jump_counter), (Action::ChildSong, FROG_SARIA, 0));
    w.msg_ctx.ocarina_mode = OCARINA_MODE_00;
    // jumpCounter 4: sJumpOrder[1], the yellow frog, NA_SE_EV_FROG_JUMP at it.
    for _ in 0..3 {
        frame(&mut w);
    }
    assert!(!fr(&w, fs[FROG_YELLOW]).is_jumping_up);
    let jumps = sfx_at(&w, NA_SE_EV_FROG_JUMP, SfxPos::Actor(fs[FROG_YELLOW]));
    frame(&mut w);
    assert_eq!(fr(&w, s).jump_counter, 4);
    assert!(fr(&w, fs[FROG_YELLOW]).is_jumping_up);
    assert_eq!(sfx_at(&w, NA_SE_EV_FROG_JUMP, SfxPos::Actor(fs[FROG_YELLOW])), jumps + 1);
    // Not the choir: no NA_SE_EN_DODO_M_EAT.
    assert!(!fr(&w, fs[FROG_YELLOW]).is_jumping_to_frog_song);
    // At 48 (Saria's Song not played): the chime, and its frog (sSongToFrog[FROG_SARIA], yellow)
    // grows once it sits.
    let chimes = sfx_any(&w, NA_SE_SY_CORRECT_CHIME);
    spot_until_not(&mut w, Action::ChildSong, 100);
    assert_eq!(fr(&w, s).action, Action::ChildSongFirstTime);
    assert_eq!(sfx_any(&w, NA_SE_SY_CORRECT_CHIME), chimes + 1);
    let y = fr(&w, fs[FROG_YELLOW]);
    assert!(y.is_active && y.is_jumping_up);
    assert_eq!(sfx_at(&w, NA_SE_EV_FROG_GROW_UP, SfxPos::Actor(fs[FROG_YELLOW])), 1);
    // EnFr_SetScaleActive: Math_ApproachF(.., 180, 2, 25): 175, then 185 caught at 180; back to
    // 150 (155, 145 caught); to 210 ...; done at 270 (growingScaleIndex 3).
    update(&mut w, fs[FROG_YELLOW]);
    assert_eq!(fr(&w, fs[FROG_YELLOW]).scale, 175.0);
    update(&mut w, fs[FROG_YELLOW]);
    assert_eq!((fr(&w, fs[FROG_YELLOW]).scale, fr(&w, fs[FROG_YELLOW]).is_growing), (180.0, true));
    update(&mut w, fs[FROG_YELLOW]);
    update(&mut w, fs[FROG_YELLOW]);
    assert_eq!((fr(&w, fs[FROG_YELLOW]).scale, fr(&w, fs[FROG_YELLOW]).growing_scale_index), (150.0, 1));
    let n = spot_until_not(&mut w, Action::ChildSongFirstTime, 100);
    assert!(n > 5);
    let y = fr(&w, fs[FROG_YELLOW]);
    assert_eq!((y.scale, y.growing_scale_index, y.is_active, y.is_jumping_up), (270.0, 3, false, false));
    // Grown, it's done: 0x40A9 with the treasure's sound, the ocarina off, the box paused.
    assert_eq!((fr(&w, s).action, fr(&w, s).actor.text_id), (Action::PrintTextBox, 0x40A9));
    assert_eq!(sfx_any(&w, NA_SE_SY_TRE_BOX_APPEAR), 1);
    assert_eq!(w.msg_ctx.msg_mode, MSGMODE_PAUSED);
    update(&mut w, s);
    assert_eq!((fr(&w, s).action, w.msg_ctx.text_id), (Action::TalkBeforeReward, 0x40A9));
    // EnFr_SetReward: a purple rupee, EVENTCHKINF_SONGS_FOR_FROGS_SARIA.
    assert!(!w.save.get_event_chk_inf(EVENTCHKINF_SONGS_FOR_FROGS_SARIA));
    read_text_and_set_reward(&mut w);
    assert_eq!(fr(&w, s).reward, GI_RUPEE_PURPLE);
    assert!(w.save.get_event_chk_inf(EVENTCHKINF_SONGS_FOR_FROGS_SARIA));
    // The others' flags untouched.
    for f in [EVENTCHKINF_SONGS_FOR_FROGS_ZL, EVENTCHKINF_SONGS_FOR_FROGS_EPONA, EVENTCHKINF_SONGS_FOR_FROGS_SUNS, EVENTCHKINF_SONGS_FOR_FROGS_SOT, EVENTCHKINF_SONGS_FOR_FROGS_STORMS, EVENTCHKINF_SONGS_FOR_FROGS_CHOIR] {
        assert!(!w.save.get_event_chk_inf(f), "{f:#x}");
    }
    // All five back under: the purple rupee offered (30 across, 100 up), taken, back to waiting.
    back_in(&mut w);
    assert_eq!(fr(&w, s).action, Action::GiveReward);
    assert_eq!((w.player().get_item_id, w.player().interact_range_actor), (GI_RUPEE_PURPLE, Some(s)));
    fr_mut(&mut w, s).actor.parent = w.player;
    update(&mut w, s);
    assert_eq!((fr(&w, s).action, fr(&w, s).actor.parent), (Action::SetIdle, None));
}

#[test]
fn a_child_song_played_again_gives_a_blue_rupee() {
    let Some(a) = assets() else { return };
    let mut w = river(&a, |s| s.set_event_chk_inf(EVENTCHKINF_SONGS_FOR_FROGS_EPONA));
    listening(&mut w);
    let s = spot(&w);
    let fs = frogs(&w);
    // Epona's Song (OCARINA_MODE_06) played before: after the jumps, 0x40A9 at once (the treasure's
    // sound), no frog grows.
    w.msg_ctx.ocarina_mode = OCARINA_MODE_06;
    update(&mut w, s);
    assert_eq!(fr(&w, s).song_index, FROG_EPONA);
    let n = spot_until_not(&mut w, Action::ChildSong, 100);
    assert_eq!(n, 48);
    assert_eq!((fr(&w, s).action, fr(&w, s).actor.text_id), (Action::PrintTextBox, 0x40A9));
    assert_eq!(sfx_any(&w, NA_SE_SY_TRE_BOX_APPEAR), 1);
    assert!(fs.iter().all(|&h| !fr(&w, h).is_active));
    assert_eq!(sfx_any(&w, NA_SE_EV_FROG_GROW_UP), 0);
    update(&mut w, s);
    read_text_and_set_reward(&mut w);
    assert_eq!(fr(&w, s).reward, GI_RUPEE_BLUE);
    assert!(w.save.get_event_chk_inf(EVENTCHKINF_SONGS_FOR_FROGS_EPONA));
}

#[test]
fn the_song_of_storms_gives_a_piece_of_heart_once() {
    let Some(a) = assets() else { return };
    let mut w = river(&a, |_| {});
    listening(&mut w);
    let s = spot(&w);
    // The Song of Storms (OCARINA_MODE_0A): after the jumps the chime and 0x40AA (no frog of its
    // own grows).
    w.msg_ctx.ocarina_mode = OCARINA_MODE_0A;
    update(&mut w, s);
    assert_eq!(fr(&w, s).song_index, FROG_STORMS);
    let chimes = sfx_any(&w, NA_SE_SY_CORRECT_CHIME);
    spot_until_not(&mut w, Action::ChildSong, 100);
    assert_eq!((fr(&w, s).action, fr(&w, s).actor.text_id), (Action::PrintTextBox, 0x40AA));
    assert_eq!((sfx_any(&w, NA_SE_SY_CORRECT_CHIME), sfx_any(&w, NA_SE_SY_TRE_BOX_APPEAR)), (chimes + 1, 0));
    update(&mut w, s);
    assert_eq!(w.msg_ctx.text_id, 0x40AA);
    // The first time: a piece of heart and EVENTCHKINF_SONGS_FOR_FROGS_STORMS.
    read_text_and_set_reward(&mut w);
    assert_eq!(fr(&w, s).reward, GI_HEART_PIECE);
    assert!(w.save.get_event_chk_inf(EVENTCHKINF_SONGS_FOR_FROGS_STORMS));
    // Again (its flag set): a blue rupee.
    read_text_and_set_reward(&mut w);
    assert_eq!(fr(&w, s).reward, GI_RUPEE_BLUE);
}

#[test]
fn a_note_makes_its_frog_jump_and_putting_the_ocarina_away_ends_it() {
    let Some(a) = assets() else { return };
    let mut w = river(&a, |_| {});
    listening(&mut w);
    let s = spot(&w);
    let fs = frogs(&w);
    // A note, no song (OCARINA_MODE_01): C-Left, the purple frog; its note
    // (NA_SE_EV_FROG_JUMP transposed by sSmallFrogNotes[3]).
    w.msg_ctx.ocarina_mode = OCARINA_MODE_01;
    w.msg_ctx.last_ocarina_button_index = OCARINA_BTN_C_LEFT;
    update(&mut w, s);
    assert_eq!(fr(&w, s).action, Action::ListeningToOcarinaNotes);
    assert!(fr(&w, fs[FROG_PURPLE]).is_jumping_up);
    assert!(fs.iter().enumerate().all(|(i, &h)| fr(&w, h).is_jumping_up == (i == FROG_PURPLE)));
    assert_eq!(sfx_at(&w, NA_SE_EV_FROG_JUMP, SfxPos::Actor(fs[FROG_PURPLE])), 1);
    // Again while it's jumping: nothing (EnFr_SetupJumpingUp's !isJumpingUp).
    update(&mut w, s);
    assert_eq!(sfx_at(&w, NA_SE_EV_FROG_JUMP, SfxPos::Actor(fs[FROG_PURPLE])), 1);
    // The purple frog's jump up: 25 up at frame 3 and back down on its spot, the landing.
    for _ in 0..3 {
        update(&mut w, fs[FROG_PURPLE]);
    }
    assert_eq!(fr(&w, fs[FROG_PURPLE]).action, Action::JumpingUp);
    update(&mut w, fs[FROG_PURPLE]);
    assert_eq!(fr(&w, fs[FROG_PURPLE]).actor.velocity.y, 15.0);
    for _ in 0..20 {
        update(&mut w, fs[FROG_PURPLE]);
        if fr(&w, fs[FROG_PURPLE]).action != Action::JumpingUp {
            break;
        }
    }
    let p = fr(&w, fs[FROG_PURPLE]);
    assert_eq!((p.action, p.is_jumping_up, p.actor.world_pos.y), (Action::ChooseJumpFromLogSpot, false, LOG[FROG_PURPLE].y));
    // The ocarina put away (OCARINA_MODE_04): EnFr_OcarinaMistake, no reward, the error, the
    // frogs back in, then waiting again.
    w.msg_ctx.ocarina_mode = OCARINA_MODE_04;
    update(&mut w, s);
    assert_eq!((fr(&w, s).action, fr(&w, s).reward, statics(&mut w).flags), (Action::Deactivate, GI_NONE, 12));
    assert_eq!(sfx_any(&w, NA_SE_SY_OCARINA_ERROR), 1);
    w.msg_ctx.ocarina_mode = OCARINA_MODE_00;
    back_in(&mut w);
    assert_eq!(fr(&w, s).action, Action::Idle);
    assert_eq!(w.player().get_item_id, GI_NONE);
}

/// All six child songs played: the song game.
fn all_songs(s: &mut SaveContext) {
    for f in [EVENTCHKINF_SONGS_FOR_FROGS_ZL, EVENTCHKINF_SONGS_FOR_FROGS_EPONA, EVENTCHKINF_SONGS_FOR_FROGS_SARIA, EVENTCHKINF_SONGS_FOR_FROGS_SUNS, EVENTCHKINF_SONGS_FOR_FROGS_SOT, EVENTCHKINF_SONGS_FOR_FROGS_STORMS] {
        s.set_event_chk_inf(f);
    }
}

/// The song game started: 0x40AB read (`EnFr_TalkBeforeFrogSong`'s 2 frames), the first note's.
fn song_game(w: &mut PlayState) {
    let s = spot(w);
    let order = out_on_the_log(w);
    assert_eq!(order, vec![FROG_BLUE, FROG_PURPLE, FROG_YELLOW, FROG_RED, FROG_WHITE]);
    // All six songs: FROG_CHOIR_SONG, 0x40AB.
    assert_eq!((fr(w, s).action, fr(w, s).song_index, w.msg_ctx.text_id), (Action::TalkBeforeFrogSong, FROG_CHOIR_SONG, 0x40AB));
    // The text read: as EnFr_TalkBeforeFrogSong leaves it.
    fr_mut(w, s).action = Action::SetupFrogSong;
    fr_mut(w, s).frog_song_timer = 2;
    update(w, s);
    update(w, s);
    assert_eq!((fr(w, s).action, fr(w, s).frog_song_timer), (Action::SetupFrogSong, 0));
    update(w, s);
}

/// The frog with the butterfly (`isButterflyDrawn`), if one.
fn butterfly(w: &PlayState) -> Vec<usize> {
    frogs(w).iter().enumerate().filter(|&(_, &h)| fr(w, h).is_butterfly_drawn).map(|(i, _)| i).collect()
}

/// The button's frog (A blue, C-down yellow, C-right red, C-left purple, C-up white).
fn frog_of(b: u8) -> usize {
    [FROG_BLUE, FROG_YELLOW, FROG_RED, FROG_PURPLE, FROG_WHITE][b as usize]
}

#[test]
fn the_song_game_won_gives_a_piece_of_heart() {
    let Some(a) = assets() else { return };
    let mut w = river(&a, all_songs);
    // The five frogs' songs played: all start grown (270).
    assert!(frogs(&w).iter().all(|&h| fr(&w, h).scale == 270.0));
    song_game(&mut w);
    let s = spot(&w);
    let fs = frogs(&w);
    // EnFr_SetupFrogSong: OCARINA_ACTION_FROGS (the box's MSGMODE_FROGS_START), 40 frames, the
    // first of gFrogsSongPtr's notes (A), the blue frog's butterfly.
    let notes: Vec<u8> = w.audio.ocarina.frogs_song_notes.clone();
    assert_eq!(notes, vec![0, 3, 2, 1, 3, 2, 1, 0, 1, 0, 1, 2, 3, 0]);
    assert_eq!(fr(&w, s).action, Action::ContinueFrogSong);
    assert_eq!((w.msg_ctx.ocarina_action, w.msg_ctx.msg_mode), (OCARINA_ACTION_FROGS, MSGMODE_FROGS_START));
    assert_eq!((fr(&w, s).frog_song_timer, fr(&w, s).ocarina_note_index, fr(&w, s).ocarina_note), (40, 0, OCARINA_BTN_A));
    assert_eq!(butterfly(&w), vec![FROG_BLUE]);
    // The box listening (MSGMODE_FROGS_PLAYING): the timer runs.
    w.msg_ctx.msg_mode = MSGMODE_FROGS_PLAYING;
    for _ in 0..5 {
        frame(&mut w);
    }
    assert_eq!(fr(&w, s).frog_song_timer, 35);
    // A wrong note (C-Down): its frog jumps, the box listens again, the note's still A.
    w.msg_ctx.msg_mode = MSGMODE_FROGS_WAITING;
    w.msg_ctx.last_ocarina_button_index = OCARINA_BTN_C_DOWN;
    update(&mut w, s);
    assert_eq!(w.msg_ctx.msg_mode, MSGMODE_FROGS_START);
    assert!(fr(&w, fs[FROG_YELLOW]).is_jumping_up && fr(&w, fs[FROG_YELLOW]).is_jumping_to_frog_song);
    assert_eq!((fr(&w, s).ocarina_note_index, fr(&w, s).ocarina_note, fr(&w, s).frog_song_timer), (0, OCARINA_BTN_A, 34));
    // Each right note: its frog jumps (the choir's), the next note and its frog's butterfly, and
    // sTimerFrogSong's frames for it: 20, 15, 12, then 12.
    let timers = [20, 15, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12];
    for i in 0..13 {
        for _ in 0..8 {
            frame(&mut w);
        }
        w.msg_ctx.msg_mode = MSGMODE_FROGS_WAITING;
        w.msg_ctx.last_ocarina_button_index = notes[i];
        update(&mut w, s);
        assert_eq!(w.msg_ctx.msg_mode, MSGMODE_FROGS_START, "note {i}");
        assert!(fr(&w, fs[frog_of(notes[i])]).is_jumping_up, "note {i}");
        let sp = fr(&w, s);
        assert_eq!((sp.action, sp.ocarina_note_index, sp.ocarina_note, sp.frog_song_timer), (Action::ContinueFrogSong, i as u8 + 1, notes[i + 1], timers[i]), "note {i}");
        assert_eq!(butterfly(&w), vec![frog_of(notes[i + 1])], "note {i}");
    }
    // The 14th: done (ocarinaNoteIndex kept at 13), no butterflies, the chime, 0x40AC.
    for _ in 0..8 {
        frame(&mut w);
    }
    let chimes = sfx_any(&w, NA_SE_SY_CORRECT_CHIME);
    w.msg_ctx.msg_mode = MSGMODE_FROGS_WAITING;
    w.msg_ctx.last_ocarina_button_index = notes[13];
    update(&mut w, s);
    let sp = fr(&w, s);
    assert_eq!((sp.action, sp.ocarina_note_index, sp.actor.text_id), (Action::PrintTextBox, 13, 0x40AC));
    assert_eq!(sfx_any(&w, NA_SE_SY_CORRECT_CHIME), chimes + 1);
    assert_eq!(w.msg_ctx.msg_mode, MSGMODE_PAUSED);
    assert!(butterfly(&w).is_empty());
    // The choir's jumps: NA_SE_EN_DODO_M_EAT at frame 3 of the jump.
    let h = fs[frog_of(notes[13])];
    for _ in 0..4 {
        update(&mut w, h);
    }
    assert!(sfx_at(&w, NA_SE_EN_DODO_M_EAT, SfxPos::Actor(h)) >= 1);
    update(&mut w, s);
    assert_eq!(w.msg_ctx.text_id, 0x40AC);
    // The first time: a piece of heart, EVENTCHKINF_SONGS_FOR_FROGS_CHOIR.
    read_text_and_set_reward(&mut w);
    assert_eq!(fr(&w, s).reward, GI_HEART_PIECE);
    assert!(w.save.get_event_chk_inf(EVENTCHKINF_SONGS_FOR_FROGS_CHOIR));
    // Won before: a purple rupee.
    read_text_and_set_reward(&mut w);
    assert_eq!(fr(&w, s).reward, GI_RUPEE_PURPLE);
    // After it the notes are random, one of sOcarinaNotes' five.
    for i in 0..20 {
        assert!(S_OCARINA_NOTES.contains(&get_next_note_frog_song(&mut w, i % 14)));
    }
}

#[test]
fn the_song_game_out_of_time_or_b_is_a_mistake() {
    let Some(a) = assets() else { return };
    let mut w = river(&a, all_songs);
    song_game(&mut w);
    let s = spot(&w);
    // B with all five sitting (MSGMODE_FROGS_PLAYING): a mistake.
    w.msg_ctx.msg_mode = MSGMODE_FROGS_PLAYING;
    w.input.press.button = BTN_B;
    update(&mut w, s);
    w.input.press.button = 0;
    assert_eq!((fr(&w, s).action, fr(&w, s).reward, statics(&mut w).flags), (Action::Deactivate, GI_NONE, 12));
    assert_eq!(sfx_any(&w, NA_SE_SY_OCARINA_ERROR), 1);
    assert!(butterfly(&w).is_empty());
    back_in(&mut w);
    assert_eq!(fr(&w, s).action, Action::Idle);
    assert!(!w.save.get_event_chk_inf(EVENTCHKINF_SONGS_FOR_FROGS_CHOIR));
    // Again, the game left to run out: 40 frames, then the mistake on the 41st.
    w.msg_ctx.ocarina_mode = OCARINA_MODE_00;
    song_game(&mut w);
    w.msg_ctx.msg_mode = MSGMODE_FROGS_PLAYING;
    for _ in 0..40 {
        update(&mut w, s);
    }
    assert_eq!((fr(&w, s).action, fr(&w, s).frog_song_timer), (Action::ContinueFrogSong, 0));
    update(&mut w, s);
    assert_eq!((fr(&w, s).action, statics(&mut w).flags), (Action::Deactivate, 12));
    assert_eq!(sfx_any(&w, NA_SE_SY_OCARINA_ERROR), 2);
}

