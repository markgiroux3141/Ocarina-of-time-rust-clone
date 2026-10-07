//! The Song of Time's blocks (`Obj_Timeblock`, `z_obj_timeblock.c`) against the C, in the Master
//! Quest Deku Tree (`ootx scene-info --scene ydan`):
//! - room 2's four, params 0x39FF at (-1208, 547, 1390), (-1166, 547, 1348), (-1165, 547, 1432)
//!   and (-1123, 547, 1390), turned -8192;
//! - room 7's five, 0x39FF at (-2142, -760, 93), (-2085, -701, 217) turned 8192, (-1972, -635,
//!   290), (-1846, -564, 242) turned 24576 and (-1798, -493, 115): a stair up the room;
//! - room 5's one, 0xB9FF at (-1365, -880, 1070), over the purple rupee's chest (`En_Box` 0x5AA0
//!   at (-1376, -880, 1078)).
//!
//! 0x39FF: flag 0x3F, bit 6 set (`unk_177` 0: the block's state is params bit 15, not the flag),
//! bit 8 set (scale 0.6, focus 40 up, `Demo_Effect` 0x19), bit 10 clear (`ObjTimeblock_Normal`),
//! range 7 (300), bit 15 clear: hidden. 0xB9FF: the same with bit 15 set: shown.
//!
//! The ocarina isn't ported: the tests play the song as the ocarina would, Player's
//! `PLAYER_STATE2_24` (the ocarina out) and `msgCtx.lastPlayedSong` (the staff's 254 while it's
//! played, then `OCARINA_SONG_TIME`, 10). Expected values are worked out from the C in the
//! comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_collision::dyna::BGCHECK_SCENE;
use eng_input::pad::PadState;
use glam::{Mat4, Vec3};
use oot_actors::PlayExt;
use oot_actors::obj_timeblock::*;
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_LOCK_ON_DISABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_UPDATE_DURING_OCARINA};
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ACTORCAT_ITEMACTION, ActorHandle, ActorImpl};
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
/// hints as if heard (flag 0x1F), 60 frames in; then in `room` (a debug start's room change),
/// its enemies gone.
fn deku_tree_room(a: &Arc<GameAssets>, room: i8) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-inside").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    w.flags.set_switch(0x1F);
    idle(&mut w, 60);
    if room != 0 {
        assert!(w.room_request(room));
        idle(&mut w, 1);
        w.room_change_done();
        // The room's objects load (Object_UpdateEntries: two frames), then its actors' inits.
        idle(&mut w, 3);
    }
    for h in w.actors.category(ACTORCAT_ENEMY).to_vec() {
        if let Some(a) = w.actors.actor_mut(h) {
            a.kill();
        }
    }
    idle(&mut w, 1);
    w
}

/// The purple rupee's chest's (`En_Box` 0x5AA0) bg actor.
fn chest_bg(w: &PlayState) -> u16 {
    let h = w.actors.all().into_iter().find(|&h| w.actors.downcast::<oot_actors::en_box::EnBox>(h).is_some_and(|c| c.actor.home_pos == ROOM_5_CHEST)).expect("the chest");
    let c = w.actors.downcast::<oot_actors::en_box::EnBox>(h).unwrap();
    assert_eq!(c.actor.params as u16, 0x5AA0);
    c.bg
}

fn sfx_on(w: &PlayState, frame: u32, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == frame && s == id)
}

/// The blocks, oldest first.
fn blocks(w: &PlayState) -> Vec<ActorHandle> {
    let mut v: Vec<ActorHandle> = w.actors.category(ACTORCAT_ITEMACTION).iter().copied().filter(|&h| w.actors.downcast::<ObjTimeblock>(h).is_some()).collect();
    v.reverse();
    v
}

fn block_at(w: &PlayState, home: Vec3) -> ActorHandle {
    blocks(w).into_iter().find(|&h| w.actors.actor(h).unwrap().home_pos == home).expect("the block")
}

fn tb(w: &PlayState, h: ActorHandle) -> &ObjTimeblock {
    w.actors.downcast::<ObjTimeblock>(h).expect("Obj_Timeblock")
}

fn collision_on(w: &PlayState, h: ActorHandle) -> bool {
    let bg = &w.col.dyna.actors[tb(w, h).bg as usize];
    bg.in_use() && !bg.collision_disabled
}

/// The floor straight down from 300 above `p`, and whose it is.
fn floor_at(w: &PlayState, p: Vec3) -> (f32, u16) {
    let (y, poly) = w.col.entity_raycast_down(Vec3::new(p.x, p.y + 300.0, p.z));
    (y, poly.map_or(BGCHECK_SCENE, |p| p.bg))
}

fn player_state2(w: &PlayState) -> u32 {
    w.player.and_then(|h| w.actors.get(h)).and_then(|p| p.as_player()).unwrap().state_flags2()
}

fn set_player_state2(w: &mut PlayState, set: u32, clear: u32) {
    let h = w.player.unwrap();
    w.actors.get_mut(h).and_then(|p| p.as_player_mut()).unwrap().change_state_flags2(set, clear);
}

fn demo_effects(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.actor(h).is_some_and(|a| a.id == ACTOR_DEMO_EFFECT && !a.killed)).collect()
}

/// The attention cutscene (`OnePointCutscene_Attention`, 5010) on block `h` is queued or running.
fn attention_on(w: &PlayState, h: ActorHandle) -> bool {
    (0..oot_game::camera::NUM_CAMS as i16).any(|i| w.camera(i).is_some_and(|c| c.cs_id == 5010 && c.target == Some(h)))
}

/// Link's placed between frames (`place_player`), as if he'd walked there.
fn put_link(w: &mut PlayState, pos: Vec3, yaw: i16) {
    let (y, _) = floor_at(w, pos);
    w.place_player(Vec3::new(pos.x, y, pos.z), yaw);
}

const ROOM_2: [Vec3; 4] = [Vec3::new(-1208.0, 547.0, 1390.0), Vec3::new(-1166.0, 547.0, 1348.0), Vec3::new(-1165.0, 547.0, 1432.0), Vec3::new(-1123.0, 547.0, 1390.0)];
const ROOM_7: [Vec3; 5] =
    [Vec3::new(-2142.0, -760.0, 93.0), Vec3::new(-2085.0, -701.0, 217.0), Vec3::new(-1972.0, -635.0, 290.0), Vec3::new(-1846.0, -564.0, 242.0), Vec3::new(-1798.0, -493.0, 115.0)];
const ROOM_5: Vec3 = Vec3::new(-1365.0, -880.0, 1070.0);
const ROOM_5_CHEST: Vec3 = Vec3::new(-1376.0, -880.0, 1078.0);

/// What `ObjTimeblock_Init` and a first update leave of a placed block of params 0x39FF or
/// 0xB9FF: rot.z 0, scale 0.6, `ATTENTION_RANGE_2`, focus 40 up, `unk_177` 0, flag 0x3F (unset)
/// in `unk_174`, bit 15 in `unk_175` and the visibility, `ObjTimeblock_Normal`,
/// `ObjTimeblock_WaitForOcarina`; the collision on only while it's shown.
fn check_placed(w: &PlayState, h: ActorHandle, home: Vec3, shown: bool) {
    let t = tb(w, h);
    assert_eq!(t.actor.params as u16, if shown { 0xB9FF } else { 0x39FF });
    assert_eq!((t.actor.world_pos, t.actor.scale), (home, Vec3::splat(0.6)));
    assert_eq!((t.actor.shape_rot.z, t.actor.world_rot.z, t.actor.home_rot.z), (0, 0, 0));
    assert_eq!(t.actor.target_mode, 2);
    assert_eq!(t.actor.focus_pos, home + Vec3::new(0.0, 40.0, 0.0));
    let f = ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA | ACTOR_FLAG_LOCK_ON_DISABLED;
    assert_eq!(t.actor.flags & f, f);
    assert_eq!((t.unk_177, t.switch_flag(), t.unk_174, t.unk_175, t.is_visible), (0, 0x3F, false, shown, shown));
    assert_eq!((t.action, t.song_observer, t.demo_effect_timer), (Action::Normal, SongObserver::WaitForOcarina, 0));
    assert_eq!(collision_on(w, h), shown);
    // ObjTimeblock_Draw: nothing while hidden; shown, gSongOfTimeBlockDL with sPrimColors[0].
    let rs = t.render_state();
    let mut out = DrawOut::default();
    t.draw(&rs, w, &ViewInfo::new(Vec3::ZERO, Mat4::IDENTITY), &mut out);
    assert!(out.xlu.is_empty());
    if shown {
        assert_eq!(out.opa.len(), 1);
        assert_eq!(out.opa[0].mesh, eng_gfx::MeshKey::named(oot_game::pack::keys::bake("Obj_Timeblock/gSongOfTimeBlockDL")));
        let sv = out.opa[0].params.segments.as_ref().expect("the prim colour");
        assert_eq!(sv.prim.iter().flatten().copied().collect::<Vec<_>>(), vec![[100, 120, 140, 255]]);
    } else {
        assert!(out.opa.is_empty());
    }
}

#[test]
fn the_hidden_blocks_of_rooms_2_and_7_are_placed_whole() {
    let Some(a) = assets() else { return };
    for (room, homes) in [(2i8, &ROOM_2[..]), (7, &ROOM_7[..])] {
        let w = deku_tree_room(&a, room);
        assert_eq!(blocks(&w).len(), homes.len(), "room {room}");
        for &home in homes {
            let h = block_at(&w, home);
            check_placed(&w, h, home, false);
            // Hidden: no collision under it from above.
            let (y, bg) = floor_at(&w, home);
            assert_ne!(bg, tb(&w, h).bg, "room {room} {home:?}: the floor at {y}");
        }
    }
}

#[test]
fn room_5s_block_stands_shown_on_the_purple_rupees_chest() {
    let Some(a) = assets() else { return };
    let w = deku_tree_room(&a, 5);
    assert_eq!(blocks(&w).len(), 1);
    let h = block_at(&w, ROOM_5);
    check_placed(&w, h, ROOM_5, true);
    // Its top is a floor; the chest, 11 across and 8 deep from its middle, is under it.
    let bg = tb(&w, h).bg;
    let (top, on) = floor_at(&w, ROOM_5);
    println!("room 5 block top {top}");
    assert_eq!(on, bg);
    let (over_chest, on) = floor_at(&w, ROOM_5_CHEST);
    assert_eq!((over_chest, on), (top, bg));
    // The chest: a small one (type 5), there from the start, its top under the block's.
    let chest = chest_bg(&w);
    assert!(w.col.dyna.actors[chest as usize].in_use() && !w.col.dyna.actors[chest as usize].collision_disabled);
    let (y, on) = w.col.entity_raycast_down(ROOM_5_CHEST + Vec3::new(0.0, 50.0, 0.0));
    assert_eq!(on.map(|p| p.bg), Some(chest));
    assert!(y < top, "the chest's top {y}");
}

#[test]
fn link_in_range_gets_the_ocarina_prompt() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 5);
    let h = block_at(&w, ROOM_5);
    // ObjTimeblock_PlayerIsInRange: xzDistToPlayer <= sRanges[7] (300), and outside the block's
    // square (scale 0.6 × 50 + 6 = 36 from its middle along its axes; it's unturned).
    for (off, in_range) in [(Vec3::new(120.0, 0.0, 0.0), true), (Vec3::new(299.0, 0.0, 0.0), true), (Vec3::new(301.0, 0.0, 0.0), false), (Vec3::new(37.0, 0.0, 30.0), true)] {
        put_link(&mut w, ROOM_5 + off, 0x4000);
        set_player_state2(&mut w, 0, PLAYER_STATE2_23);
        idle(&mut w, 1);
        // ObjTimeblock_WaitForOcarina: in range without the ocarina out, PLAYER_STATE2_23 (after
        // Player's update: the block is ITEMACTION).
        assert_eq!(tb(&w, h).player_is_in_range(&w), in_range, "{off:?}");
        assert_eq!(player_state2(&w) & PLAYER_STATE2_23 != 0, in_range, "{off:?}");
        assert_eq!(tb(&w, h).song_observer, SongObserver::WaitForOcarina);
    }
    // Inside the square (35 across): not in range. Link can't stand there (the block's walls),
    // so the check is on the block with Link put there.
    {
        let p = w.player.unwrap();
        w.actors.actor_mut(p).unwrap().world_pos = ROOM_5 + Vec3::new(35.0, 0.0, 35.0);
        let b = w.actors.downcast_mut::<ObjTimeblock>(h).unwrap();
        b.actor.xz_dist_to_player = 49.5;
    }
    assert!(!tb(&w, h).player_is_in_range(&w));
}

/// The song as the ocarina plays it to block `h` with Link in range: the ocarina out
/// (`PLAYER_STATE2_24`: `ObjTimeblock_WaitForSong` the next frame; `Message_StartOcarina` logs),
/// the staff's 254 for a frame, then `OCARINA_SONG_TIME`; returns after the frame its 110 frames
/// start (`songEndTimer`). The ocarina's put away (`PLAYER_STATE2_24` off) after.
fn play_the_song_of_time(w: &mut PlayState, h: ActorHandle) {
    set_player_state2(w, PLAYER_STATE2_24, 0);
    idle(w, 1);
    set_player_state2(w, 0, PLAYER_STATE2_24);
    assert_eq!(tb(w, h).song_observer, SongObserver::WaitForSong);
    w.msg_ctx.last_played_song = 254;
    idle(w, 1);
    assert_eq!(tb(w, h).unk_172, 254);
    w.msg_ctx.last_played_song = OCARINA_SONG_TIME;
    idle(w, 1);
    assert_eq!((tb(w, h).song_end_timer, tb(w, h).unk_172), (110, OCARINA_SONG_TIME));
}

#[test]
fn the_song_of_time_hides_room_5s_block_and_shows_it_again() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 5);
    let h = block_at(&w, ROOM_5);
    let bg = tb(&w, h).bg;
    put_link(&mut w, ROOM_5 + Vec3::new(120.0, 0.0, 0.0), 0x4000);
    idle(&mut w, 1);
    play_the_song_of_time(&mut w, h);
    // ObjTimeblock_WaitForSong counts songEndTimer down a frame at a time: 109 frames more and
    // it's 1, nothing changed.
    idle(&mut w, 109);
    let t = tb(&w, h);
    assert_eq!((t.song_end_timer, t.actor.params as u16, t.demo_effect_timer), (1, 0xB9FF, 0));
    assert!(demo_effects(&w).is_empty());
    // The 110th: true. ObjTimeblock_Normal: Demo_Effect at the block (params
    // sSizeOptions[1].demoEffectParams, 0x19), demoEffectTimer 160 (159 after the update's
    // count), the attention camera, demoEffectFirstPartTimer 12 (11 after this frame's count),
    // params ^ 0x8000 (unk_177 0). Still shown: unk_175 is read at the 12 frames' end.
    idle(&mut w, 1);
    let toggled = w.audio.frames;
    let t = tb(&w, h);
    assert_eq!((t.actor.params as u16, t.demo_effect_timer, t.demo_effect_first_part_timer), (0x39FF, 159, 11));
    assert!(t.is_visible && t.unk_175);
    let fx = demo_effects(&w);
    assert_eq!(fx.len(), 1);
    let e = w.actors.actor(fx[0]).unwrap();
    assert_eq!((e.world_pos, e.params, e.shape_rot.y), (ROOM_5, 0x19, 0));
    assert!(attention_on(&w, h));
    assert!(collision_on(&w, h));
    // 10 frames more: still shown; the 11th, unk_175 = bit 15 (0): hidden, its collision off
    // (DynaPoly_DisableCollision in the same update).
    idle(&mut w, 10);
    assert!(tb(&w, h).is_visible && collision_on(&w, h));
    idle(&mut w, 1);
    assert_eq!(tb(&w, h).demo_effect_first_part_timer, 0);
    assert!(!tb(&w, h).is_visible && !tb(&w, h).unk_175);
    assert!(!collision_on(&w, h));
    let (_, on) = floor_at(&w, ROOM_5_CHEST);
    assert_eq!(on, chest_bg(&w), "the chest's out");
    // NA_SE_SY_TRE_BOX_APPEAR when the action sees demoEffectTimer 50: 160 - 50 = 110 frames
    // after the toggle.
    while w.audio.frames < toggled + 109 {
        idle(&mut w, 1);
        assert!(!sfx_on(&w, w.audio.frames, oot_game::audio::sfx::NA_SE_SY_TRE_BOX_APPEAR) || w.audio.frames == toggled + 110, "frame {}", w.audio.frames);
    }
    idle(&mut w, 1);
    assert_eq!(w.audio.frames, toggled + 110);
    assert!(sfx_on(&w, w.audio.frames, oot_game::audio::sfx::NA_SE_SY_TRE_BOX_APPEAR));
    assert_eq!(tb(&w, h).demo_effect_timer, 49);
    // The song again before the effect's over (demoEffectTimer 160 frames from the toggle):
    // ignored. songEndTimer keeps counting down with lastPlayedSong still 10 (past 0 to -1, ...).
    idle(&mut w, 30);
    assert_eq!(tb(&w, h).demo_effect_timer, 19);
    // The ocarina put away (OCARINA_MODE_04): back to ObjTimeblock_WaitForOcarina.
    w.msg_ctx.ocarina_mode = OCARINA_MODE_04;
    idle(&mut w, 1);
    assert_eq!(tb(&w, h).song_observer, SongObserver::WaitForOcarina);
    w.msg_ctx.ocarina_mode = 0;
    idle(&mut w, 20);
    assert_eq!(tb(&w, h).demo_effect_timer, 0);
    // Played again: shown 12 frames after the toggle, its collision back, the chest covered.
    play_the_song_of_time(&mut w, h);
    idle(&mut w, 110);
    assert_eq!(tb(&w, h).actor.params as u16, 0xB9FF);
    assert!(!tb(&w, h).is_visible);
    idle(&mut w, 11);
    assert!(tb(&w, h).is_visible && collision_on(&w, h));
    // Its polys go into the lookup at the next DynaPoly_UpdateContext (it updates after the BG
    // category's): the chest's covered from the next frame.
    idle(&mut w, 1);
    let (_, on) = floor_at(&w, ROOM_5_CHEST);
    assert_eq!(on, bg);
}

#[test]
fn the_song_shows_room_7s_stair_and_link_stands_on_it() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 7);
    // Link by the lowest step (-2142, -760, 93), 120 off along +z: every block within 300 of him
    // hears the song.
    let start = ROOM_7[0] + Vec3::new(0.0, 0.0, 120.0);
    put_link(&mut w, start, i16::MIN);
    idle(&mut w, 1);
    let near: Vec<ActorHandle> = blocks(&w).into_iter().filter(|&h| tb(&w, h).actor.xz_dist_to_player <= 300.0).collect();
    println!("{} blocks in range", near.len());
    let h = block_at(&w, ROOM_7[0]);
    play_the_song_of_time(&mut w, h);
    idle(&mut w, 110 + 11);
    // Shown and enabled this frame; in the lookup from the next DynaPoly_UpdateContext.
    idle(&mut w, 1);
    for &b in &near {
        assert!(tb(&w, b).is_visible && collision_on(&w, b), "{:?}", tb(&w, b).actor.home_pos);
        assert_eq!(tb(&w, b).actor.params as u16, 0xB9FF);
    }
    // One attention cutscene only: OnePointCutscene_Attention refuses a second on the same
    // category (ITEMACTION).
    let cams = (0..oot_game::camera::NUM_CAMS as i16).filter(|&i| w.camera(i).is_some_and(|c| c.cs_id == 5010 && c.target.is_some_and(|t| near.contains(&t)))).count();
    assert!(cams <= 1, "{cams} attention cameras");
    // The lowest step's top is a floor now.
    let bg = tb(&w, h).bg;
    let (top, on) = floor_at(&w, ROOM_7[0]);
    println!("lowest step top {top}");
    assert_eq!(on, bg);
    // Link put on it stands there, and it's no longer in range (DynaPolyActor_IsPlayerAbove).
    let on_top = Vec3::new(ROOM_7[0].x, top, ROOM_7[0].z);
    w.place_player(on_top, i16::MIN);
    idle(&mut w, 20);
    let p = w.player();
    assert!(p.grounded(), "{:?}", p.actor.world_pos);
    assert_eq!(p.actor.floor_bg_id, bg);
    assert!((p.actor.world_pos - on_top).length() < 1.0, "{:?}", p.actor.world_pos);
    assert!(!tb(&w, h).player_is_in_range(&w));
}

/// Blocks of other params, spawned in room 5 (`object_timeblock` is in its list).
fn spawn_block(w: &mut PlayState, pos: Vec3, params: u16, rot_z: i16) -> ActorHandle {
    let (y, _) = floor_at(w, pos);
    let pos = Vec3::new(pos.x, y, pos.z);
    let h = w.actor_spawn(ACTOR_OBJ_TIMEBLOCK, pos, [0, 0, rot_z], params as i16).expect("Obj_Timeblock");
    idle(w, 1);
    h
}

#[test]
fn the_switch_flag_blocks_and_the_alt_ones_follow_the_c() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 5);
    // On the room's west floor (about -875), Link between the blocks, by the torches.
    let at = Vec3::new(-1300.0, -880.0, 870.0);
    let link = Vec3::new(-1240.0, -880.0, 1010.0);
    let alt_at = Vec3::new(-1250.0, -880.0, 1150.0);
    // Params 0x3838 (flag 0x38, bit 6 clear, size 0: scale 1, range 7: 300): unk_177 1 (0x38 >= 0x38): shown
    // as the flag xor bit 15: hidden. rot.z 5: colour 5, and rot.z zeroed but home.rot.z kept.
    let h = spawn_block(&mut w, at, 0x3838, 5);
    let t = tb(&w, h);
    assert_eq!((t.unk_177, t.is_visible, t.actor.scale.x, t.actor.focus_pos.y), (1, false, 1.0, t.actor.world_pos.y + 60.0));
    assert_eq!((t.actor.shape_rot.z, t.actor.home_rot.z), (0, 5));
    // Params 0x3810 (flag 0x10, unk_177 2 below 0x38): the flag xor bit 15 xor child Link
    // (LINK_AGE_IN_YEARS == YEARS_CHILD): shown for child Link.
    let h2 = spawn_block(&mut w, at + Vec3::new(0.0, 0.0, 130.0), 0x3810, 0);
    assert_eq!((tb(&w, h2).unk_177, tb(&w, h2).is_visible), (2, true));
    // Its colour: sPrimColors[5].
    {
        let mut out = DrawOut::default();
        let b = w.actors.downcast_mut::<ObjTimeblock>(h).unwrap();
        b.is_visible = true;
        let b = tb(&w, h);
        b.draw(&b.render_state(), &w, &ViewInfo::new(Vec3::ZERO, Mat4::IDENTITY), &mut out);
        let sv = out.opa[0].params.segments.as_ref().unwrap();
        assert_eq!(sv.prim.iter().flatten().copied().collect::<Vec<_>>(), vec![[70, 160, 225, 255]]);
        w.actors.downcast_mut::<ObjTimeblock>(h).unwrap().is_visible = false;
    }
    // The song to the 0x3838 one: the flag toggled (set), unk_174 read 12 frames later: shown;
    // unk_177 1 and the visibility changed: ObjTimeblock_DoNothing from then on.
    w.actors.actor_mut(h2).unwrap().kill();
    put_link(&mut w, link, -0x8000);
    idle(&mut w, 1);
    play_the_song_of_time(&mut w, h);
    idle(&mut w, 110);
    assert!(w.flags.get_switch(0x38));
    assert!(!tb(&w, h).is_visible);
    idle(&mut w, 11);
    assert!(tb(&w, h).is_visible && collision_on(&w, h));
    assert_eq!(tb(&w, h).action, Action::DoNothing);

    // Params 0x3C11 (flag 0x11, alt, range 300): shown as bit 15 xor the flag: hidden,
    // ObjTimeblock_AltBehaviourNotVisible.
    let alt = spawn_block(&mut w, alt_at, 0x3C11, 0);
    assert_eq!((tb(&w, alt).action, tb(&w, alt).is_visible), (Action::AltBehaviourNotVisible, false));
    let fx_before = demo_effects(&w).len();
    // The flag set elsewhere: unk_176 (0) ^ 1 and 1 ^ bit 15 (0): Demo_Effect, 160, 12 frames; the
    // flag read 12 frames on (func_80BA06AC): shown, its collision on; ObjTimeblock_AltBehaviorVisible
    // only once the effect's 160 frames are over.
    w.flags.set_switch(0x11);
    idle(&mut w, 1);
    assert_eq!(demo_effects(&w).len(), fx_before + 1);
    assert_eq!((tb(&w, alt).demo_effect_timer, tb(&w, alt).demo_effect_first_part_timer, tb(&w, alt).unk_176), (159, 11, true));
    assert!(!tb(&w, alt).is_visible);
    idle(&mut w, 11);
    assert!(tb(&w, alt).is_visible && collision_on(&w, alt));
    assert_eq!(tb(&w, alt).action, Action::AltBehaviourNotVisible);
    idle(&mut w, 148);
    assert_eq!(tb(&w, alt).demo_effect_timer, 0);
    idle(&mut w, 1);
    assert_eq!(tb(&w, alt).action, Action::AltBehaviorVisible);
    // The song to it there toggles the flag back (unset): hidden 12 frames on, then
    // ObjTimeblock_AltBehaviourNotVisible when the effect's over.
    w.msg_ctx.last_played_song = 0;
    idle(&mut w, 1);
    play_the_song_of_time(&mut w, alt);
    idle(&mut w, 110);
    assert!(!w.flags.get_switch(0x11));
    assert_eq!(tb(&w, alt).demo_effect_timer, 159);
    idle(&mut w, 11);
    assert!(!tb(&w, alt).is_visible && !collision_on(&w, alt));
    assert_eq!(tb(&w, alt).action, Action::AltBehaviorVisible);
    idle(&mut w, 149);
    assert_eq!(tb(&w, alt).action, Action::AltBehaviourNotVisible);
}
