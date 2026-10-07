//! The gravestones (`Bg_Haka`, `z_bg_haka.c`) against the C, in the Master Quest Deku Tree
//! (`ootx scene-info --scene ydan`): room 7's eight, params 0, at y -775:
//! (-2280, 129) and (-2283, -20) turned 16384, (-2079, 277), (-1811, -23) and (-1770, 282)
//! unturned, (-1628, 180) turned -16384, (-1723, -177) turned -7646, (-2084, -104) unturned.
//!
//! Master Quest sinks these stones 15 into the floor, too low for Player to hold on to (`--test
//! push` covers that, and Player pulling a stone flush with the floor): the tests add the pull to
//! `dyna.unk_150` as Player would (`func_8002DFA4(bg, -2.0, yaw)`, his pull each frame). The
//! stone is BG: it updates before Player, so it reads what was added before its frame. The
//! graveyard's and Lake Hylia's paths are run on a room 7 stone with the play state's scene set
//! to theirs for its update. Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use eng_math::step_to_f;
use glam::{Mat4, Vec3};
use oot_actors::PlayExt;
use oot_actors::bg_haka::*;
use oot_game::actor_ctx::{ACTORCAT_BG, ACTORCAT_ENEMY, ActorHandle, ActorImpl};
use oot_game::audio::sfx::{NA_SE_SY_CORRECT_CHIME, SFX_FLAG};
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

/// Inside the Deku Tree (`deku-tree-inside`: child Link, by day), the sound log on, Navi's room 0
/// hints as if heard (flag 0x1F), 60 frames in; then in room 7 (a debug start's room change),
/// its enemies gone.
fn room_7(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-inside").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    w.flags.set_switch(0x1F);
    idle(&mut w, 60);
    assert!(w.room_request(7));
    idle(&mut w, 1);
    w.room_change_done();
    // The room's objects load (Object_UpdateEntries: two frames), then its actors' inits.
    idle(&mut w, 3);
    for h in w.actors.category(ACTORCAT_ENEMY).to_vec() {
        if let Some(a) = w.actors.actor_mut(h) {
            a.kill();
        }
    }
    idle(&mut w, 1);
    w
}

fn sfx_on(w: &PlayState, frame: u32, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == frame && s == id)
}

fn stones(w: &PlayState) -> Vec<ActorHandle> {
    let mut v: Vec<ActorHandle> = w.actors.category(ACTORCAT_BG).iter().copied().filter(|&h| w.actors.downcast::<BgHaka>(h).is_some()).collect();
    v.reverse();
    v
}

fn stone_at(w: &PlayState, home: Vec3) -> ActorHandle {
    stones(w).into_iter().find(|&h| w.actors.actor(h).unwrap().home_pos == home).expect("the gravestone")
}

fn haka(w: &PlayState, h: ActorHandle) -> &BgHaka {
    w.actors.downcast::<BgHaka>(h).expect("Bg_Haka")
}

fn player_state2(w: &PlayState) -> u32 {
    w.player.and_then(|h| w.actors.get(h)).and_then(|p| p.as_player()).unwrap().state_flags2()
}

fn set_player_state2(w: &mut PlayState, set: u32, clear: u32) {
    let h = w.player.unwrap();
    w.actors.get_mut(h).and_then(|p| p.as_player_mut()).unwrap().change_state_flags2(set, clear);
}

/// The floor straight down from 300 above `p`, and whose it is.
fn floor_at(w: &PlayState, p: Vec3) -> (f32, Option<u16>) {
    let (y, poly) = w.col.entity_raycast_down(Vec3::new(p.x, p.y + 300.0, p.z));
    (y, poly.map(|p| p.bg))
}

/// `BgHaka_Update` on its own (between frames), with the play state's scene `scene` for it.
fn update_in(w: &mut PlayState, h: ActorHandle, scene: u16) {
    let saved = w.scene_id;
    w.scene_id = scene;
    let mut act = w.actors.take(h).unwrap();
    act.update(w);
    w.actors.put_back(h, act);
    w.scene_id = saved;
}

const STONES: [(Vec3, i16); 8] = [
    (Vec3::new(-2280.0, -775.0, 129.0), 16384),
    (Vec3::new(-2283.0, -775.0, -20.0), 16384),
    (Vec3::new(-2079.0, -775.0, 277.0), 0),
    (Vec3::new(-1811.0, -775.0, -23.0), 0),
    (Vec3::new(-1770.0, -775.0, 282.0), 0),
    (Vec3::new(-1628.0, -775.0, 180.0), -16384),
    (Vec3::new(-1723.0, -775.0, -177.0), -7646),
    (Vec3::new(-2084.0, -775.0, -104.0), 0),
];

/// Room 7's debug start by the gravestones and the time blocks' stair: on the floor between the
/// stones at (-2079, 277) and (-1770, 282), 84 across from the stair's third step overhead
/// (hidden), facing the stair and the torches (-z).
const ROOM_7_START: (Vec3, i16) = (Vec3::new(-1925.0, -760.0, 360.0), i16::MIN);

#[test]
fn room_7s_gravestones_are_placed_whole() {
    let Some(a) = assets() else { return };
    let w = room_7(&a);
    assert_eq!(stones(&w).len(), 8);
    for (home, yaw) in STONES {
        let h = stone_at(&w, home);
        let s = haka(&w, h);
        // BgHaka_Init: minVelocityY 0, scale 0.1 (sInitChain), DynaPolyActor_Init(0),
        // gGravestoneCol, BgHaka_IdleClosed; params 0; ACTORCAT_BG, FLAGS 0.
        assert_eq!((s.actor.params, s.action, s.actor.min_velocity_y, s.actor.speed_xz), (0, Action::IdleClosed, 0.0, 0.0));
        assert_eq!((s.actor.world_pos, s.actor.scale, s.actor.shape_rot.y), (home, Vec3::splat(0.1), yaw));
        assert_eq!((s.actor.category, s.actor.flags & oot_game::actor::ACTOR_FLAG_UPDATE_CULLING_DISABLED), (ACTORCAT_BG, 0));
        let bg = &w.col.dyna.actors[s.bg as usize];
        assert!(bg.in_use() && !bg.collision_disabled);
        assert_eq!(bg.move_flags, 0);
        assert_eq!(w.col.dyna.unk_150(s.bg), 0.0);
        // Its top is a floor of the room now.
        let (top, on) = floor_at(&w, home);
        assert_eq!(on, Some(s.bg), "{home:?}: the floor at {top}");
        println!("{home:?}: top {top}, y {}..{}", bg.min_y, bg.max_y);
    }
}

#[test]
fn a_pull_slides_a_gravestone_60_back_frame_by_frame() {
    let Some(a) = assets() else { return };
    let mut w = room_7(&a);
    let (home, _) = STONES[2];
    let h = stone_at(&w, home);
    let bg = haka(&w, h).bg;
    // Link holds on (PLAYER_STATE2_4, as Player sets it pulling).
    set_player_state2(&mut w, PLAYER_STATE2_4, 0);
    // Player's pull each frame: func_8002DFA4(bg, -2.0, his yaw); the stone reads it the next
    // frame (it updates before him).
    let pull = |w: &mut PlayState| w.col.dyna.func_8002DFA4(bg, -2.0, 0);
    pull(&mut w);
    idle(&mut w, 1);
    // BgHaka_IdleClosed: unk_150 -2, not the graveyard or Lake Hylia, not a push: world.rot.y =
    // shape.rot.y (0) + 0x8000, BgHaka_Pull. Not moved yet, no sound.
    let s = haka(&w, h);
    assert_eq!((s.action, s.actor.world_rot.y, s.actor.world_pos), (Action::Pull, i16::MIN, home));
    assert_eq!(s.actor.sfx, 0);
    // BgHaka_Pull each frame: speed += 0.05 (CLAMP_MAX 1.5), Math_StepToF(minVelocityY, 60,
    // speed), the position minVelocityY from home along world.rot.y (-z), NA_SE_EV_ROCK_SLIDE -
    // SFX_FLAG (Actor_PlaySfx_Flagged). Worked out: 30 frames speeding up to 1.5 cover 0.05 ×
    // (1 + ... + 30) = 23.25, then 1.5 a frame for the 36.75 left: 25 frames, the last one short
    // (24.5): 55 frames.
    let (mut speed, mut dist) = (0.0f32, 0.0f32);
    let mut frames = 0;
    loop {
        pull(&mut w);
        idle(&mut w, 1);
        frames += 1;
        speed += 0.05;
        if speed > 1.5 {
            speed = 1.5;
        }
        let done = step_to_f(&mut dist, 60.0, speed);
        let expect = Vec3::new(eng_math::sin_s(i16::MIN) * dist + home.x, home.y, eng_math::cos_s(i16::MIN) * dist + home.z);
        let s = haka(&w, h);
        assert_eq!((s.actor.speed_xz, s.actor.min_velocity_y, s.actor.world_pos), (speed, dist, expect), "pull frame {frames}");
        assert_eq!(s.actor.sfx, NA_SE_EV_ROCK_SLIDE - SFX_FLAG, "pull frame {frames}");
        if done {
            break;
        }
        // Still pulling: unk_150 not zeroed (Player's pulls add up), Link held on.
        assert_eq!(s.action, Action::Pull);
        assert!(w.col.dyna.unk_150(bg) < 0.0);
        assert_ne!(player_state2(&w) & PLAYER_STATE2_4, 0);
        assert!(frames < 100);
    }
    assert_eq!(frames, 55);
    assert_eq!(haka(&w, h).actor.world_pos.z, home.z - 60.0);
    // At 60: unk_150 0, Link let go of (PLAYER_STATE2_4 cleared); params 0: no chime; not the
    // graveyard: no Poe; BgHaka_IdleOpened.
    let f = w.audio.frames;
    assert_eq!((haka(&w, h).action, w.col.dyna.unk_150(bg)), (Action::IdleOpened, 0.0));
    assert_eq!(player_state2(&w) & PLAYER_STATE2_4, 0);
    assert!(!sfx_on(&w, f, NA_SE_SY_CORRECT_CHIME));
    // Its collision follows (the bg actor's transform from the next DynaPoly_UpdateContext): the
    // stone's top is 60 towards -z now.
    idle(&mut w, 1);
    let (_, on) = floor_at(&w, home - Vec3::new(0.0, 0.0, 60.0));
    assert_eq!(on, Some(bg));
    // BgHaka_IdleOpened: a pull now is refused (unk_150 zeroed, PLAYER_STATE2_4 cleared); it stays.
    set_player_state2(&mut w, PLAYER_STATE2_4, 0);
    pull(&mut w);
    idle(&mut w, 1);
    assert_eq!((haka(&w, h).action, w.col.dyna.unk_150(bg), haka(&w, h).actor.min_velocity_y), (Action::IdleOpened, 0.0, 60.0));
    assert_eq!(player_state2(&w) & PLAYER_STATE2_4, 0);
    assert_eq!(haka(&w, h).actor.sfx, 0);
}

#[test]
fn a_push_is_refused() {
    let Some(a) = assets() else { return };
    let mut w = room_7(&a);
    let h = stone_at(&w, STONES[3].0);
    let bg = haka(&w, h).bg;
    set_player_state2(&mut w, PLAYER_STATE2_4, 0);
    // Player's push: func_8002DFA4(bg, 2.0, yaw). BgHaka_IdleClosed: 0 < unk_150: zeroed, Link let
    // go of; it stays closed.
    w.col.dyna.func_8002DFA4(bg, 2.0, 0x4000);
    assert_eq!(w.col.dyna.unk_158(bg), 0x4000);
    idle(&mut w, 1);
    assert_eq!((haka(&w, h).action, w.col.dyna.unk_150(bg), haka(&w, h).actor.world_pos), (Action::IdleClosed, 0.0, STONES[3].0));
    assert_eq!(player_state2(&w) & PLAYER_STATE2_4, 0);
}

#[test]
fn link_behind_a_gravestone_walks_on_sand() {
    let Some(a) = assets() else { return };
    let mut w = room_7(&a);
    // BgHaka_CheckPlayerOnDirtPatch: Link within 34.6 across of the stone's middle and 36 to
    // 112.8 behind it (its frame: Actor_WorldToActorCoords, -z for an unturned stone) gets
    // PLAYER_STATE2_FORCE_SAND_FLOOR_SOUND, which his floor check (Player_ProcessSceneCollision,
    // before his update clears it) turns into SURFACE_SFX_OFFSET_SAND (1).
    let (home, _) = STONES[2];
    for (off, sand) in [(Vec3::new(0.0, 0.0, -70.0), true), (Vec3::new(30.0, 0.0, -100.0), true), (Vec3::new(0.0, 0.0, -130.0), false), (Vec3::new(40.0, 0.0, -70.0), false)] {
        let p = home + off;
        let (y, _) = floor_at(&w, p);
        w.place_player(Vec3::new(p.x, y, p.z), 0);
        idle(&mut w, 2);
        let pl = w.player();
        println!("{off:?}: floor {y}, offset {}", pl.floor_sfx_offset);
        assert_eq!(pl.floor_sfx_offset == 1, sand, "{off:?}");
    }
    // A stone turned 16384 (its frame's -z is the world's -x): the patch is on its -x side, which
    // for room 7's turned stones is in the wall. Link put there between frames, its update on its
    // own (Player's doesn't run to clear the flag).
    let (home, _) = STONES[0];
    let h = stone_at(&w, home);
    for (dx, sand) in [(-70.0, true), (70.0, false)] {
        set_player_state2(&mut w, 0, PLAYER_STATE2_FORCE_SAND_FLOOR_SOUND);
        let p = w.player.unwrap();
        w.actors.actor_mut(p).unwrap().world_pos = home + Vec3::new(dx, 15.0, 0.0);
        let scene = w.scene_id;
        update_in(&mut w, h, scene);
        assert_eq!(player_state2(&w) & PLAYER_STATE2_FORCE_SAND_FLOOR_SOUND != 0, sand, "{dx}");
    }
}

#[test]
fn the_graveyard_and_lake_hylia_paths_follow_the_c() {
    let Some(a) = assets() else { return };
    let mut w = room_7(&a);
    let (home, _) = STONES[2];
    let h = stone_at(&w, home);
    let bg = haka(&w, h).bg;
    // The graveyard, child Link, by day: a pull refused (unk_150 zeroed, Link let go of), the
    // Graveyard Boy's warning (0x5073), params 100 (the cooldown), BgHaka_IdleLockedClosed.
    assert!(!w.save.adult && w.save.is_day());
    set_player_state2(&mut w, PLAYER_STATE2_4, 0);
    w.col.dyna.func_8002DFA4(bg, -2.0, 0);
    update_in(&mut w, h, SCENE_GRAVEYARD);
    assert_eq!((haka(&w, h).action, haka(&w, h).actor.params, w.col.dyna.unk_150(bg)), (Action::IdleLockedClosed, 100, 0.0));
    assert_eq!(player_state2(&w) & PLAYER_STATE2_4, 0);
    assert_eq!(w.msg_ctx.text_id, TEXT_GRAVEYARD_BOY_WARNING);
    // BgHaka_IdleLockedClosed: params counted down (99 after the first), pulls refused; back to
    // BgHaka_IdleClosed when it reaches 0, 100 updates on.
    for i in 1..100 {
        w.col.dyna.func_8002DFA4(bg, -2.0, 0);
        update_in(&mut w, h, SCENE_GRAVEYARD);
        assert_eq!((haka(&w, h).action, haka(&w, h).actor.params, w.col.dyna.unk_150(bg)), (Action::IdleLockedClosed, 100 - i, 0.0), "update {i}");
    }
    update_in(&mut w, h, SCENE_GRAVEYARD);
    assert_eq!((haka(&w, h).action, haka(&w, h).actor.params), (Action::IdleClosed, 0));
    // In a cutscene (Play_InCsMode): refused without the warning; it stays closed.
    w.msg_ctx.text_id = 0;
    w.cs_ctx.state = oot_game::cutscene::CS_STATE_RUN;
    w.col.dyna.func_8002DFA4(bg, -2.0, 0);
    update_in(&mut w, h, SCENE_GRAVEYARD);
    w.cs_ctx.state = oot_game::cutscene::CS_STATE_IDLE;
    assert_eq!((haka(&w, h).action, haka(&w, h).actor.params, w.col.dyna.unk_150(bg), w.msg_ctx.text_id), (Action::IdleClosed, 0, 0.0, 0));
    // Lake Hylia, child Link, switch flag 0x23 unset: refused, no warning.
    assert!(!w.flags.get_switch(0x23));
    w.col.dyna.func_8002DFA4(bg, -2.0, 0);
    update_in(&mut w, h, SCENE_LAKE_HYLIA);
    assert_eq!((haka(&w, h).action, w.col.dyna.unk_150(bg)), (Action::IdleClosed, 0.0));
    // With 0x23 set: the pull starts.
    w.flags.set_switch(0x23);
    w.col.dyna.func_8002DFA4(bg, -2.0, 0);
    update_in(&mut w, h, SCENE_LAKE_HYLIA);
    assert_eq!(haka(&w, h).action, Action::Pull);
    w.flags.unset_switch(0x23);

    // The graveyard at night (no warning: IS_DAY false): the pull; at 60, params 0 and not day in
    // the graveyard: a Poe (En_Poh, a placeholder) at home, turned as the stone, params 1. The
    // graveyard's object_poh put in a bank for it.
    let h2 = stone_at(&w, STONES[0].0);
    let bg2 = haka(&w, h2).bg;
    w.object_ctx.spawn(0x0009);
    w.save.night_flag = true;
    let mut n = 0;
    loop {
        w.col.dyna.func_8002DFA4(bg2, -2.0, 0);
        update_in(&mut w, h2, SCENE_GRAVEYARD);
        n += 1;
        if haka(&w, h2).action == Action::IdleOpened {
            break;
        }
        assert!(n < 100);
    }
    // The first update starts the pull, then 55 of it.
    assert_eq!(n, 56);
    let poes: Vec<&oot_game::actor::Actor> = w.actors.all().into_iter().filter_map(|p| w.actors.actor(p)).filter(|p| p.id == ACTOR_EN_POH).collect();
    assert_eq!(poes.len(), 1);
    assert_eq!((poes[0].world_pos, poes[0].shape_rot.y, poes[0].params), (STONES[0].0, 16384, 1));
    w.save.night_flag = false;

    // Params 1: the chime at 60 (wherever it is).
    let h3 = stone_at(&w, STONES[3].0);
    let bg3 = haka(&w, h3).bg;
    w.actors.actor_mut(h3).unwrap().params = 1;
    let mut n = 0;
    while haka(&w, h3).action != Action::IdleOpened {
        w.col.dyna.func_8002DFA4(bg3, -2.0, 0);
        let scene = w.scene_id;
        update_in(&mut w, h3, scene);
        n += 1;
        assert!(n < 100);
    }
    assert!(sfx_on(&w, w.audio.frames, NA_SE_SY_CORRECT_CHIME));
}

#[test]
fn the_earth_stays_put_as_the_stone_slides() {
    let Some(a) = assets() else { return };
    let mut w = room_7(&a);
    let (home, _) = STONES[2];
    let h = stone_at(&w, home);
    let draw = |w: &PlayState| {
        let s = haka(w, h);
        let mut out = DrawOut::default();
        s.draw(&s.render_state(), w, &ViewInfo::new(Vec3::ZERO, Mat4::IDENTITY), &mut out);
        out
    };
    // BgHaka_Draw: gGravestoneStoneDL opaque at the actor's matrix, gGravestoneEarthDL
    // translucent, moved minVelocityY × 10 along its model z (scale 0.1: minVelocityY in the
    // world, back along +z where the stone slid -z).
    let out = draw(&w);
    assert_eq!(out.opa.len(), 1);
    assert_eq!(out.xlu.len(), 1);
    assert_eq!(out.opa[0].mesh, eng_gfx::MeshKey::named(oot_game::pack::keys::mesh("object_haka", "gGravestoneStoneDL")));
    assert_eq!(out.xlu[0].mesh, eng_gfx::MeshKey::named(oot_game::pack::keys::mesh("object_haka", "gGravestoneEarthDL")));
    assert_eq!(out.opa[0].transform, out.xlu[0].transform);
    let bg = haka(&w, h).bg;
    for _ in 0..57 {
        w.col.dyna.func_8002DFA4(bg, -2.0, 0);
        idle(&mut w, 1);
    }
    assert_eq!(haka(&w, h).action, Action::IdleOpened);
    let out = draw(&w);
    let stone = out.opa[0].transform.w_axis;
    let earth = out.xlu[0].transform.w_axis;
    assert!((Vec3::new(stone.x, stone.y, stone.z) - (home - Vec3::new(0.0, 0.0, 60.0))).length() < 1e-3);
    assert!((Vec3::new(earth.x, earth.y, earth.z) - home).length() < 1e-3, "{earth:?}");
}

#[test]
fn room_7s_debug_start_by_the_gravestones_stands() {
    let Some(a) = assets() else { return };
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-inside").unwrap();
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, oot_game::audio::GameAudio::default()).expect("Play_Init");
    idle(&mut w, 2);
    let (pos, yaw) = ROOM_7_START;
    oot_actors::playthrough::deku_tree_room_start(&mut w, 7, pos, yaw);
    for h in w.actors.category(ACTORCAT_ENEMY).to_vec() {
        if let Some(a) = w.actors.actor_mut(h) {
            a.kill();
        }
    }
    idle(&mut w, 40);
    let p = w.player();
    let at = p.actor.world_pos;
    assert_eq!((w.room_ctx.cur.num, w.room_ctx.prev.num), (7, -1), "the room changed");
    assert!(p.grounded(), "not on the ground at {at:?}");
    assert!((at - pos).length() < 1.0, "moved from {pos:?} to {at:?} ({:?})", p.action);
    // The stones and the stair round it.
    assert_eq!(stones(&w).len(), 8);
}
