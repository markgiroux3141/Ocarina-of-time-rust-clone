//! GAME-10 milestone 4 against the C: Lord Jabu-Jabu's switches (`z_bg_bdan_switch.c`), moving
//! parts (`z_bg_bdan_objects.c`) and water spouts (`z_en_siofuki.c`), as Master Quest places them
//! (`ootx scene-info --scene bdan_scene`).
//!
//! Child Link enters at `ENTR_JABU_JABU_0` on a `jabu-fish` save, and the room changes to the one
//! tested (`Room_RequestNewRoom`). Each test runs one actor's update alone (`Actor_UpdateAll`'s
//! call), what stands on a bg actor set as `DynaPoly` would (`dyna.interactFlags`), so nothing is
//! walked and nothing waits on `Rand`. Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_collision::dyna::{DYNA_INTERACT_ACTOR_SWITCH_PRESSED, DYNA_INTERACT_PLAYER_ON_TOP};
use eng_input::pad::PadState;
use eng_math::cos_s;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_bdan_objects::{self as objects, BgBdanObjects};
use oot_actors::bg_bdan_switch::{self as switch, BgBdanSwitch};
use oot_actors::en_siofuki::{self as siofuki, EnSiofuki};
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::sfx::SfxPos;
use oot_game::camera::{CAM_ID_MAIN, CAM_SET_NORMAL0, CAM_SET_PIVOT_VERTICAL};
use oot_game::collision_check::AC_HIT;
use oot_game::play::{PlayState, scripted_input};
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

/// Child Link in Jabu-Jabu (`jabu-fish`), the sound log on, in room `room`, its actors in.
fn jabu(a: &Arc<GameAssets>, room: i8) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_JABU_JABU_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("jabu-fish").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    if room != 0 {
        assert!(w.room_request(room));
        idle(&mut w, 1);
        w.room_change_done();
    }
    assert_eq!(w.room_ctx.cur.num, room);
    idle(&mut w, 20);
    w
}

fn find<T: oot_game::actor_ctx::ActorImpl + 'static>(w: &PlayState, params: i16, home: Vec3) -> ActorHandle {
    *w.actors
        .all()
        .iter()
        .find(|&&h| w.actors.downcast::<T>(h).is_some() && w.actors.actor(h).is_some_and(|a| !a.killed && a.params == params && a.home_pos == home))
        .unwrap_or_else(|| panic!("params {params:#06x} at {home}"))
}

fn sw(w: &mut PlayState, h: ActorHandle) -> &mut BgBdanSwitch {
    w.actors.downcast_mut::<BgBdanSwitch>(h).expect("Bg_Bdan_Switch")
}

fn ob(w: &mut PlayState, h: ActorHandle) -> &mut BgBdanObjects {
    w.actors.downcast_mut::<BgBdanObjects>(h).expect("Bg_Bdan_Objects")
}

fn sp(w: &mut PlayState, h: ActorHandle) -> &mut EnSiofuki {
    w.actors.downcast_mut::<EnSiofuki>(h).expect("En_Siofuki")
}

/// The actor's update alone (`Actor_UpdateAll`'s call), as the updating actor.
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

fn draw_update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.draw_update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

fn draw_sfx(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.draw_sfx(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// What stands on bg actor `bg` this frame (`dyna.interactFlags`, which `Actor_UpdateAll` clears
/// after the updates).
fn stand(w: &PlayState, bg: u16, flag: u8) {
    w.col.dyna.unset_all_interact_flags(bg);
    if flag != 0 {
        w.col.dyna.set_interact_flag(bg, flag);
    }
}

/// The times a sound was asked for at `h` (`Actor_PlaySfx`).
fn sfx(w: &PlayState, h: ActorHandle, id: u16) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, p)| s == id && p == SfxPos::Actor(h)).count()
}

const ROOM_1_YELLOW: Vec3 = Vec3::new(-612.0, -1234.0, -1836.0);
const ROOM_1_TALL: Vec3 = Vec3::new(-302.0, -1552.0, -1689.0);
const ROOM_4_BLUE: Vec3 = Vec3::new(-813.0, 79.0, -1707.0);

#[test]
fn a_yellow_switch_stepped_on_sets_its_flag_and_stays_down() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a, 1);
    // Room 1's 0x1902: yellow (type 2), flag 0x19 (door 2).
    let h = find::<BgBdanSwitch>(&w, 0x1902, ROOM_1_YELLOW);
    let s = sw(&mut w, h);
    // BgBdanSwitch_Init: scale.x, .z 0.1; func_8086D5C4: unk_1C8 1.
    assert_eq!((s.actor.scale.x, s.actor.scale.z), (0.1, 0.1));
    assert_eq!((s.action, s.unk_1c8), (switch::Action::Up, 1.0));
    assert!(!w.flags.get_switch(0x19));
    let bg = sw(&mut w, h).bg;
    // func_8086D0EC's size: unk_1CC 0 -> 0xFA0 (not above 0), then 0x5DC a frame.
    sw(&mut w, h).unk_1cc = 0;
    stand(&w, bg, 0);
    update(&mut w, h);
    let s = sw(&mut w, h);
    let c = cos_s(0xFA0);
    assert_eq!(s.unk_1cc, 0xFA0);
    assert_eq!(s.unk_1d4, ((c * 0.5) + (53.000004 / 6.0)) * 0.012);
    assert_eq!(s.unk_1d0, ((c * 0.5) + 20.5) * (1.0 * 0.0050000004));
    assert_eq!(s.actor.scale.y, 0.1);
    assert_eq!(s.actor.shape_y_offset, 1.2 / s.unk_1d0);
    // Link on it (DynaPolyActor_IsPlayerOnTop): sinking, the flag set (func_8086D4B4).
    stand(&w, bg, DYNA_INTERACT_PLAYER_ON_TOP);
    update(&mut w, h);
    assert_eq!(sw(&mut w, h).action, switch::Action::Sinking);
    assert!(w.flags.get_switch(0x19));
    // func_8086D694: with the attention camera turned to it (or 100 frames on), 0.2 a frame
    // down to 0.1 or less: 1 - 5 x 0.2; then NA_SE_EV_FOOT_SWITCH, down.
    let foot = sfx(&w, h, switch::NA_SE_EV_FOOT_SWITCH);
    for i in 0..5 {
        sw(&mut w, h).unk_1da = 0;
        update(&mut w, h);
        assert_eq!(sw(&mut w, h).action == switch::Action::Down, i == 4, "update {i}");
    }
    assert_eq!(sw(&mut w, h).unk_1c8, 0.1);
    assert_eq!(sfx(&w, h, switch::NA_SE_EV_FOOT_SWITCH), foot + 1);
    // Link off: it stays down while the flag stays set.
    stand(&w, bg, 0);
    update(&mut w, h);
    assert_eq!(sw(&mut w, h).action, switch::Action::Down);
    // The flag cleared: rising 0.2 a frame to 1 (0.1 + 5 x 0.2), up.
    w.flags.unset_switch(0x19);
    update(&mut w, h);
    assert_eq!(sw(&mut w, h).action, switch::Action::Rising);
    for i in 0..5 {
        update(&mut w, h);
        assert_eq!(sw(&mut w, h).action == switch::Action::Up, i == 4, "update {i}");
    }
    assert_eq!(sw(&mut w, h).unk_1c8, 1.0);
}

#[test]
fn a_blue_switch_holds_its_flag_while_pressed() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a, 4);
    // Room 4's 0x1200: blue (type 0), flag 0x12.
    let h = find::<BgBdanSwitch>(&w, 0x1200, ROOM_4_BLUE);
    let bg = sw(&mut w, h).bg;
    stand(&w, bg, DYNA_INTERACT_ACTOR_SWITCH_PRESSED);
    update(&mut w, h);
    assert_eq!(sw(&mut w, h).action, switch::Action::Sinking);
    assert!(w.flags.get_switch(0x12));
    for _ in 0..5 {
        sw(&mut w, h).unk_1da = 0;
        update(&mut w, h);
    }
    assert_eq!(sw(&mut w, h).action, switch::Action::Down);
    // func_8086D730 set unk_1D8 6, the update took one off. Pressed, it's 6 again each frame.
    update(&mut w, h);
    assert_eq!(sw(&mut w, h).unk_1d8, 5);
    // Let go: unk_1D8 counts down (after the action); the action that sees it at 0 (the sixth)
    // rises and clears the flag.
    stand(&w, bg, 0);
    for i in 0..6 {
        update(&mut w, h);
        assert_eq!(w.flags.get_switch(0x12), i < 5, "update {i}");
    }
    assert_eq!(sw(&mut w, h).action, switch::Action::Rising);
}

#[test]
fn the_tall_switch_hit_shrinks_and_grows_back_with_its_flag() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a, 1);
    // Room 1's 0x0F03: tall yellow (type 3), flag 0x0F (the compass's chest).
    let h = find::<BgBdanSwitch>(&w, 0x0F03, ROOM_1_TALL);
    let s = sw(&mut w, h);
    // Init: scale.x, .z 0.05, ATTENTION_RANGE_4, func_8086DB4C: twice its height.
    assert_eq!((s.actor.scale.x, s.actor.scale.z), (0.05, 0.05));
    assert_eq!(s.actor.target_mode, 4);
    assert_eq!((s.action, s.unk_1c8), (switch::Action::TallUp, 2.0));
    // The sphere's radius is unk_1D4 x 370 after each update; the draw puts it through the
    // matrix (Collider_UpdateSpheres(0)) and the focus 140 up (D_8086E0E0).
    update(&mut w, h);
    let r = (sw(&mut w, h).unk_1d4 * 370.0) as i16;
    assert_eq!(sw(&mut w, h).collider.elements[0].dim.model_sphere.radius, r);
    draw_update(&mut w, h);
    let s = sw(&mut w, h);
    assert_eq!(s.collider.elements[0].dim.world_sphere.radius, r);
    assert!((s.actor.focus_pos.y - (ROOM_1_TALL.y + 1.2 + 140.0 * s.unk_1d0)).abs() < 0.01, "{}", s.actor.focus_pos);
    // Hit (AC_HIT before the update): shrinking, the flag set, the next hit counted 10 frames on
    // (frames Link isn't held in a cutscene: gJabuIntroCs still plays on this save).
    assert!(w.player_in_cs_mode());
    sw(&mut w, h).collider.base.ac_flags |= AC_HIT;
    update(&mut w, h);
    let s = sw(&mut w, h);
    assert_eq!((s.action, s.unk_1d8), (switch::Action::TallShrinking, 10));
    assert!(w.flags.get_switch(0x0F));
    // func_8086DC48: 0.3 a frame down to 1 or less (2 - 4 x 0.3), then its height 1.
    for i in 0..4 {
        sw(&mut w, h).unk_1da = 0;
        update(&mut w, h);
        assert_eq!(sw(&mut w, h).action == switch::Action::TallDown, i == 3, "update {i}");
    }
    assert_eq!(sw(&mut w, h).unk_1c8, 1.0);
    // Type 3 waits for its flag to clear, then grows 0.3 a frame back to 2.
    w.flags.unset_switch(0x0F);
    update(&mut w, h);
    assert_eq!(sw(&mut w, h).action, switch::Action::TallGrowing);
    for i in 0..4 {
        update(&mut w, h);
        assert_eq!(sw(&mut w, h).action == switch::Action::TallUp, i == 3, "update {i}");
    }
    assert_eq!(sw(&mut w, h).unk_1c8, 2.0);
}

#[test]
fn room_1s_elevator_rises_and_sinks_and_turns_the_camera() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a, 1);
    let home = Vec3::new(0.0, -771.0, -1703.0);
    let h = find::<BgBdanObjects>(&w, objects::JABU_OBJECTS_TYPE_SMALL_AUTO_ELEVATOR, home);
    // Init: params cut to the type, the timer 512, oscillating.
    assert_eq!(ob(&mut w, h).action, objects::Action::ElevatorOscillate);
    ob(&mut w, h).timer = 512;
    let bg = ob(&mut w, h).bg;
    stand(&w, bg, 0);
    // y = home - sin(timer x pi / 256) x 471.24, the timer down first: 128 frames on the top
    // (timer 384), 384 frames on the bottom (timer 128), 512 on home, and round again.
    let y = |t: f32| home.y - (t * (std::f32::consts::PI / 256.0)).sin() * 471.24;
    for f in 1..=512 {
        update(&mut w, h);
        let t = 512 - f;
        assert_eq!(ob(&mut w, h).actor.world_pos.y, y(t as f32), "frame {f}");
        if f == 128 {
            assert!((ob(&mut w, h).actor.world_pos.y - (home.y + 471.24)).abs() < 0.01);
        }
        if f == 384 {
            assert!((ob(&mut w, h).actor.world_pos.y - (home.y - 471.24)).abs() < 0.01);
        }
    }
    assert_eq!(ob(&mut w, h).timer, 512);
    // Link on it: the main camera's setting kept, CAM_SET_NORMAL2 asked for, the bg's settings
    // off (CAM_STATE_CHECK_BG); off it, 10 frames on, the old setting back and the bg's on.
    let before = w.camera(CAM_ID_MAIN).unwrap().setting;
    stand(&w, bg, DYNA_INTERACT_PLAYER_ON_TOP);
    update(&mut w, h);
    assert_eq!(ob(&mut w, h).camera_setting, before);
    assert_eq!(ob(&mut w, h).var, 10);
    assert_eq!(w.camera(CAM_ID_MAIN).unwrap().state_flags & 4, 0);
    stand(&w, bg, 0);
    for i in 0..10 {
        update(&mut w, h);
        assert_eq!(ob(&mut w, h).var, 9 - i);
    }
    assert_ne!(w.camera(CAM_ID_MAIN).unwrap().state_flags & 4, 0);
}

#[test]
fn room_1s_falling_platform_falls_400_with_one_point_3100() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a, 1);
    let home = Vec3::new(120.0, 80.0, -1583.0);
    let h = find::<BgBdanObjects>(&w, objects::JABU_OBJECTS_TYPE_FALLING_PLATFORM, home);
    assert_eq!(ob(&mut w, h).action, objects::Action::WaitForPlayerOnTop);
    assert_eq!(ob(&mut w, h).var, 0x1D);
    let bg = ob(&mut w, h).bg;
    // Link on it: the flag 0x1D, its home 200 down, 50 frames, one-point 3100: a sub camera on
    // CAM_SET_PIVOT_VERTICAL at (0, -280, -1400), fov 70.
    stand(&w, bg, DYNA_INTERACT_PLAYER_ON_TOP);
    update(&mut w, h);
    assert!(w.flags.get_switch(0x1D));
    assert_eq!(ob(&mut w, h).actor.home_pos.y, home.y - 200.0);
    assert_eq!((ob(&mut w, h).action, ob(&mut w, h).timer), (objects::Action::FallToLowerPos, 50));
    let sub = w.sub_cameras.iter().flatten().find(|c| c.setting == CAM_SET_PIVOT_VERTICAL).expect("one-point 3100's camera");
    assert_eq!(sub.eye, Vec3::new(0.0, -280.0, -1400.0));
    assert_eq!(sub.fov, 70.0);
    // y = home - cos(timer x pi / 50) x 200, the timer down first; at 0 NA_SE_EV_BUYOSTAND_STOP_U.
    stand(&w, bg, 0);
    for f in 1..=50 {
        update(&mut w, h);
        let t = (50 - f) as f32;
        assert_eq!(ob(&mut w, h).actor.world_pos.y, home.y - 200.0 - (t * (std::f32::consts::PI / 50.0)).cos() * 200.0, "frame {f}");
    }
    assert_eq!(ob(&mut w, h).actor.world_pos.y, home.y - 400.0);
    assert_eq!(ob(&mut w, h).action, objects::Action::DoNothing);
    assert_eq!(sfx(&w, h, objects::NA_SE_EV_BUYOSTAND_STOP_U), 1);
}

#[test]
fn room_14s_water_rises_75_on_its_switch_for_100_frames() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a, 14);
    let home = Vec3::new(1100.0, -1283.0, -2383.0);
    let h = find::<BgBdanObjects>(&w, objects::JABU_OBJECTS_TYPE_WATERBOX_HEIGHT_CHANGER, home);
    // Init: waterBoxes[7] at its height.
    assert_eq!(w.col.water_box_surface(7), Some(-1283));
    assert_eq!(ob(&mut w, h).action, objects::Action::WaitForSwitch);
    // The yellow switch 0x39 beside it: 100 frames' hold, up 0.5 a frame to 75 above home (150
    // frames), the box following.
    w.flags.set_switch(0x39);
    update(&mut w, h);
    assert_eq!((ob(&mut w, h).action, ob(&mut w, h).timer), (objects::Action::ChangeWaterBoxLevel, 100));
    for f in 1..=150 {
        update(&mut w, h);
        assert_eq!(ob(&mut w, h).action == objects::Action::WaitForTimerExpired, f == 150, "frame {f}");
    }
    assert_eq!(ob(&mut w, h).actor.world_pos.y, home.y + 75.0);
    assert_eq!(w.col.water_box_surface(7), Some(-1208));
    // The timer's 100 frames, then down 150 frames, the flag cleared.
    for _ in 0..100 {
        update(&mut w, h);
    }
    assert_eq!(ob(&mut w, h).action, objects::Action::ChangeWaterBoxLevel);
    for f in 1..=150 {
        update(&mut w, h);
        assert_eq!(w.flags.get_switch(0x39), f < 150, "frame {f}");
    }
    assert_eq!(ob(&mut w, h).action, objects::Action::WaitForSwitch);
    assert_eq!(w.col.water_box_surface(7), Some(-1283));
}

#[test]
fn big_octos_platform_waits_for_ruto_then_rises_and_brings_big_octo() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a, 6);
    let home = Vec3::new(-1360.0, -1025.0, -3343.0);
    let h = find::<BgBdanObjects>(&w, objects::JABU_OBJECTS_TYPE_BIG_OCTO_PLATFORM, home);
    // Init (not cleared, Big Octo not met): 79 below home, the flag 0x1F set, waiting for Ruto.
    assert_eq!(ob(&mut w, h).action, objects::Action::OctoPlatformWaitForRutoToStartCutscene);
    assert!(w.flags.get_switch(0x1F));
    // gJabuIntroCs (300 frames on this save) holds Play_InCsMode: 5 lower then, 79 below after.
    assert!(w.play_in_cs_mode());
    update(&mut w, h);
    assert_eq!(ob(&mut w, h).actor.world_pos.y, home.y - 79.0 - 5.0);
    // Ruto's CAM_SET_NORMAL0 (BgBdanObjects_SetProperty is hers) and Link within 250: NORMAL1,
    // 20 frames, Link put at (-1130, -1025, -3300).
    ob(&mut w, h).camera_setting = CAM_SET_NORMAL0;
    ob(&mut w, h).actor.xz_dist_to_player = 100.0;
    update(&mut w, h);
    assert_eq!((ob(&mut w, h).camera_setting, ob(&mut w, h).timer), (objects::CAM_SET_NORMAL1, 20));
    assert_eq!(w.actors.actor(w.player.unwrap()).unwrap().world_pos, Vec3::new(-1130.0, -1025.0, -3300.0));
    for i in 0..20 {
        update(&mut w, h);
        assert_eq!(ob(&mut w, h).action == objects::Action::OctoPlatformRaiseToUpperPosition, i == 19, "update {i}");
    }
    // Up to 500 above home (Math_SmoothStepToF 0.5, 7.5, 1), then DUNGEON0 for Ruto.
    for _ in 0..200 {
        update(&mut w, h);
        if ob(&mut w, h).action != objects::Action::OctoPlatformRaiseToUpperPosition {
            break;
        }
    }
    assert_eq!(ob(&mut w, h).action, objects::Action::OctoPlatformWaitForRutoToAdvanceCutscene);
    assert!((ob(&mut w, h).actor.world_pos.y - (home.y + 500.0)).abs() < 0.1);
    assert_eq!(ob(&mut w, h).camera_setting, oot_game::camera::CAM_SET_DUNGEON0);
    assert_eq!(sfx(&w, h, objects::NA_SE_EV_BUYOSTAND_STOP_A), 1);
    // Ruto's DUNGEON1: Big Octo (a placeholder until milestone 8) spawned as its child,
    // INFTABLE_RUTO_ABDUCTED; 10 frames on the flag cleared, 40 more and it descends.
    ob(&mut w, h).camera_setting = objects::CAM_SET_DUNGEON1;
    update(&mut w, h);
    assert!(ob(&mut w, h).actor.child.is_some());
    assert!(w.save.get_inf_table(objects::INFTABLE_RUTO_ABDUCTED));
    for _ in 0..10 {
        update(&mut w, h);
    }
    assert!(!w.flags.get_switch(0x1F));
    for _ in 0..40 {
        update(&mut w, h);
    }
    assert_eq!(ob(&mut w, h).action, objects::Action::OctoPlatformDescendWithBigOcto);
    // The battle won (the room's clear flag): the flag set, turned to the nearest quarter, down 3
    // a frame to 125 below home.
    ob(&mut w, h).action = objects::Action::OctoPlatformBattleInProgress;
    ob(&mut w, h).actor.shape_rot.y = 0x2345;
    w.flags.set_clear(6);
    update(&mut w, h);
    assert!(w.flags.get_switch(0x1F));
    assert_eq!(ob(&mut w, h).actor.home_rot.y, 0x4000);
    for _ in 0..400 {
        update(&mut w, h);
    }
    assert_eq!(ob(&mut w, h).action, objects::Action::WaitForPlayerInRange);
    assert_eq!(ob(&mut w, h).actor.world_pos.y, home.y - 125.0);
    assert_eq!(ob(&mut w, h).actor.shape_rot.y, 0x4000);
}

const ROOM_1_SPOUT: Vec3 = Vec3::new(341.0, -1233.0, -1704.0);

#[test]
fn a_lowering_spout_lifts_link_and_drops_on_its_flag_and_for_good_on_its_chest() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a, 1);
    // Room 1's 0x15C6 before door 14, rot (5, 5, 5): lowering, switch 0x17, chest 6 (the
    // boomerang's); 200 high (5 x 40), down 100 frames (5 x 20), scale.x 5 / 1.73 x 0.1, .z 0.25.
    let h = find::<EnSiofuki>(&w, 0x15C6, ROOM_1_SPOUT);
    let s = sp(&mut w, h);
    assert_eq!((s.action, s.max_height, s.current_height, s.active_time), (siofuki::Action::Wait, 200.0, 200.0, 5));
    assert_eq!((s.actor.scale.x, s.actor.scale.z), (5.0 * (1.0 / 1.73) * 0.1, 5.0 * 0.5 * 0.1));
    assert_eq!(s.actor.shape_rot, Default::default());
    // func_80AFBDC8: y = initPosY + height + sin((frames & 31) / 32 x 2pi) x 4.
    update(&mut w, h);
    let osc = ((w.gameplay_frames & 0x1F) as f32 / 32.0 * std::f32::consts::PI * 2.0).sin() * 4.0;
    assert_eq!(sp(&mut w, h).actor.world_pos.y, ROOM_1_SPOUT.y + 200.0 + osc);
    // Link in its column below its top, facing it (rot.y 0 at its middle: dAngle 0x8000, out of
    // the quarter): his gravity and fall cut, drawn up 4 a frame at most, his speed halved, pushed
    // his own way, 0.1 (Math_ApproachF to 3 by 1, the push towards it by 0.1).
    let top = sp(&mut w, h).actor.world_pos.y;
    let ph = w.player.unwrap();
    {
        let p = w.actors.actor_mut(ph).unwrap();
        p.world_pos = Vec3::new(ROOM_1_SPOUT.x, top - 60.0, ROOM_1_SPOUT.z);
        p.world_rot.y = 0;
        p.gravity = -1.0;
    }
    let bg = sp(&mut w, h).bg;
    stand(&w, bg, 0);
    update(&mut w, h);
    let top = sp(&mut w, h).actor.world_pos.y;
    let p = w.actors.actor(ph).unwrap();
    assert_eq!((p.gravity, p.velocity.y), (0.0, 0.0));
    assert!((p.world_pos.y - (top - 60.0 + 4.0)).abs() < 5.0, "{} under {top}", p.world_pos.y);
    assert_eq!((w.player().pushed_yaw, w.player().pushed_speed), (0, 0.1));
    assert!(sp(&mut w, h).apply_speed);
    // Its flag (0x17): the attention camera, 20 frames, then -45 for 100 frames and back, the
    // flag cleared.
    w.actors.actor_mut(ph).unwrap().world_pos = Vec3::new(0.0, -1233.0, -1704.0);
    w.flags.set_switch(0x17);
    update(&mut w, h);
    assert_eq!((sp(&mut w, h).action, sp(&mut w, h).timer), (siofuki::Action::WaitLower, 20));
    for _ in 0..21 {
        update(&mut w, h);
    }
    assert_eq!((sp(&mut w, h).action, sp(&mut w, h).timer, sp(&mut w, h).target_height), (siofuki::Action::Active, 100, -45.0));
    for _ in 0..101 {
        update(&mut w, h);
    }
    assert_eq!((sp(&mut w, h).action, sp(&mut w, h).target_height), (siofuki::Action::Wait, 200.0));
    assert!(!w.flags.get_switch(0x17));
    // The boomerang's chest opened (chest flag 6): -45 for good.
    w.flags.set_treasure(6);
    update(&mut w, h);
    let s = sp(&mut w, h);
    assert_eq!((s.action, s.current_height, s.target_height), (siofuki::Action::Down, -45.0, -45.0));
    // Its sound: NA_SE_EV_FOUNTAIN only above -35 (a lowering one).
    let before = w.audio.log.as_ref().unwrap().sfx.len();
    draw_sfx(&mut w, h);
    assert_eq!(w.audio.log.as_ref().unwrap().sfx.len(), before);
}

#[test]
fn a_rising_spout_stands_400_high_for_300_frames_on_its_flag() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a, 1);
    // Room 3's 0x0EBF (rising, switch 0x3A), spawned here at the room-1 spout's place.
    let h = w.actor_spawn(siofuki::ACTOR_EN_SIOFUKI, ROOM_1_SPOUT + Vec3::X * 300.0, [0, 0, 3], 0x0EBF).expect("spawned");
    idle(&mut w, 1);
    let s = sp(&mut w, h);
    assert_eq!((s.action, s.current_height, s.target_height), (siofuki::Action::WaitRaise, 10.0, 10.0));
    w.flags.set_switch(0x3A);
    update(&mut w, h);
    assert_eq!((sp(&mut w, h).action, sp(&mut w, h).timer, sp(&mut w, h).target_height), (siofuki::Action::Active, 300, 400.0));
    // func_80AFC1D0: Math_SmoothStepToF(height, 400, 0.8, 3, 0.01): 3 a frame.
    update(&mut w, h);
    assert_eq!(sp(&mut w, h).current_height, 10.0 + 3.0);
    for _ in 0..300 {
        update(&mut w, h);
    }
    assert_eq!((sp(&mut w, h).action, sp(&mut w, h).target_height), (siofuki::Action::WaitRaise, 10.0));
    assert!(!w.flags.get_switch(0x3A));
}

#[test]
fn room_2s_floor_drifts_link_towards_its_holes() {
    use oot_game::surface::SurfaceType;
    let Some(a) = assets() else { return };
    let mut w = jabu(&a, 2);
    // Room 2's floor (y -340) holds 80 conveyor polys (speed 3, FLOOR_CONVEYOR: `sIsFloorConveyor`)
    // round its six holes down to room 3; this one's direction is 32 (64ths of a turn: 0x8000,
    // towards -z). Link idle on it is pushed along it (`pushedSpeed`, `sConveyorYaw`).
    use eng_collision::bgcheck::PolyId;
    let n = w.col.header.polys.len() as u16;
    let ours = |w: &PlayState, id: PolyId| (w.col.conveyor_speed(id), w.col.conveyor_direction(id), w.col.is_floor_conveyor(id)) == (3, 32, true);
    let count = (0..n).filter(|&i| w.col.conveyor_speed(PolyId::scene(i)) != 0).count();
    assert_eq!(count, 106, "Jabu-Jabu's conveyor polys (room 2's 80, room 7's 26)");
    // The first such poly whose middle the floor raycast finds as itself.
    let at = (0..n)
        .map(PolyId::scene)
        .filter(|&id| ours(&w, id))
        .map(|id| {
            let v = w.col.poly_vertices(id);
            (v[0] + v[1] + v[2]) / 3.0
        })
        .find(|c| w.col.entity_raycast_down(*c + Vec3::Y * 20.0).1.is_some_and(|p| ours(&w, p)))
        .expect("a speed 3, direction 32 floor conveyor");
    // Past gJabuIntroCs (this save hasn't seen it): Player isn't pushed in a cutscene's hold.
    for _ in 0..400 {
        if !w.player_in_cs_mode() {
            break;
        }
        idle(&mut w, 1);
    }
    w.place_player(at, 0);
    // pushedSpeed steps to sFloorConveyorSpeeds[2] (3) by a tenth of it a frame: 10 frames on,
    // the push turning from his yaw (0, +z) to 0x8000 (Math_ScaledStepToS, 2400 a frame): along
    // -z, swung through +x; 30 on, Link has gone down a hole into room 3 (y -638).
    idle(&mut w, 10);
    let p = w.actors.actor(w.player.unwrap()).unwrap().world_pos;
    assert!(p.z < at.z - 5.0 && (p.x - at.x).abs() < 20.0 && p.y > -350.0, "drifted from {at} to {p}");
    idle(&mut w, 20);
    assert!(w.actors.actor(w.player.unwrap()).unwrap().world_pos.y < -600.0);
}
