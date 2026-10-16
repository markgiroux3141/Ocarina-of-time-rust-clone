//! GAME-09 milestone 2 against the C: `Bg_Spot16_Bombstone` (`z_bg_spot16_bombstone.c`), the
//! boulder over Dodongo's Cavern's mouth on Death Mountain Trail (spot16 room 0: params 0x04FF at
//! (-1679, 684, -690), facing +z), its pieces, the watch on a carried Bomb Flower, one-point 4180.
//!
//! Entered by `ENTR_DEATH_MOUNTAIN_TRAIL_3` (spawn 3, (-1671, 677, -924): out of the cavern) on a
//! `cavern-bombs` save with the trail's intro seen (`EVENTCHKINF_A1`). Expected values are worked
//! out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, BTN_CRIGHT, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_spot16_bombstone::{Action, BOULDER_DL, BgSpot16Bombstone, D_808B5DD8, EVENTCHKINF_23, statics};
use oot_actors::en_bom::{ACTOR_EN_BOM, EnBom};
use oot_actors::en_bombf::EnBombf;
use oot_actors::player::Action as PA;
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::sfx::{NA_SE_EV_ROCK_BROKEN, NA_SE_SY_CORRECT_CHIME};
use oot_game::camera::CAM_ID_MAIN;
use oot_game::onepoint::CAM_SET_FREE2;
use oot_game::effect::{EFFECT_SS_DUST, EFFECT_SS_KAKERA};
use oot_game::env::clock_time;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// `EVENTCHKINF_A1` (`save.h`): the trail's intro (`gDMTIntroCs`) seen.
const EVENTCHKINF_A1: u16 = 0xA1;
/// The boulder's placement and switch flag (params 0x04FF: `PARAMS_GET_U(params, 8, 6)` 4).
const BOULDER: Vec3 = Vec3::new(-1679.0, 684.0, -690.0);
const SWITCH: i32 = 4;
/// `SCENE_DEATH_MOUNTAIN_TRAIL` (`scene_table.h`: 0x60, spot16).
const SCENE_DEATH_MOUNTAIN_TRAIL: usize = 0x60;
/// The cliff's Bomb Flower (`En_Bombf` -1).
const FLOWER: Vec3 = Vec3::new(-1397.0, 1408.0, -646.0);

/// Child Link on the trail by the cavern's mouth (`cavern-bombs`, the intro seen), `prepare`
/// run on the save first, three frames on, the audio logged.
fn trail_with(a: &Arc<GameAssets>, prepare: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DEATH_MOUNTAIN_TRAIL_3").expect("entrance");
    let mut save = SaveContext::new(e, false, clock_time(10, 0) as u16);
    save.apply_preset("cavern-bombs").unwrap();
    save.set_event_chk_inf(EVENTCHKINF_A1);
    prepare(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    w.audio.log = Some(Default::default());
    idle(&mut w, 3);
    w
}

fn trail(a: &Arc<GameAssets>) -> PlayState {
    trail_with(a, |_| {})
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn stones(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<BgSpot16Bombstone>(h).is_some_and(|b| !b.actor.killed)).collect()
}

fn boulder(w: &PlayState) -> Option<&BgSpot16Bombstone> {
    stones(w).into_iter().filter_map(|h| w.actors.downcast::<BgSpot16Bombstone>(h)).find(|b| b.ty() == 0xFF)
}

fn pieces(w: &PlayState) -> Vec<&BgSpot16Bombstone> {
    stones(w).into_iter().filter_map(|h| w.actors.downcast::<BgSpot16Bombstone>(h)).filter(|b| b.ty() != 0xFF).collect()
}

fn sfx_frames(w: &PlayState, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, _)| f).collect()
}

fn effects_of(w: &PlayState, ty: u8) -> usize {
    w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == ty).count()
}

/// The meshes the last frame draws.
fn drawn(w: &PlayState) -> Vec<String> {
    let f = w.current_frame();
    let mut out = oot_game::play::DrawOut::default();
    w.draw(&f, &oot_game::play::ViewInfo::new(f.view.eye, glam::Mat4::IDENTITY), &mut out);
    out.opa.iter().chain(out.xlu.iter()).map(|c| c.mesh.name.clone()).collect()
}

#[test]
fn the_boulder_stands_without_switch_4_and_is_gone_with_it() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    let b = boulder(&w).expect("the boulder");
    // func_808B4C30: switchFlag 4, params cut to 0xFF. func_808B4D9C: scale 0.4, immovable, the
    // culling volume (2000, 500, 1000), gDodongosCavernRock3DL drawn, waiting (func_808B5950).
    assert_eq!((b.switch_flag, b.ty(), b.actor.params), (4, 0xFF, 0xFF));
    assert_eq!((b.actor.scale.x, b.actor.col_chk_info.mass, b.dlist, b.drawn, b.action), (0.4, 0xFF, Some(BOULDER_DL), true, Action::Boulder));
    assert_eq!((b.actor.culling_volume_distance, b.actor.culling_volume_scale, b.actor.culling_volume_downward), (2000.0, 500.0, 1000.0));
    assert_eq!((b.sin_rotation, b.cos_rotation), (0.0, 1.0));
    // func_808B4C4C: the sphere's world sphere set once, 50 up, 120 across.
    let ws = b.collider_jnt_sph.elements[0].dim.world_sphere;
    assert_eq!((ws.center, ws.radius), ([-1679, 734, -690], 120));
    // func_808B4D04: sCylinderInit's (10, 0, 50) plus the position: 190 by 80.
    let c = b.collider_cylinder.dim;
    assert_eq!((c.pos, c.radius, c.height), ([-1669, 684, -640], 190, 80));
    // Drawn (Link by the mouth, the boulder in view).
    assert!(drawn(&w).iter().any(|m| m == "mesh/object_spot16_obj/gDodongosCavernRock3DL"), "{:?}", drawn(&w));
    // Spawned with its switch flag set: gone at init.
    w.flags.set_switch(SWITCH);
    let h = w.actor_spawn(oot_actors::bg_spot16_bombstone::ACTOR_BG_SPOT16_BOMBSTONE, BOULDER, [0; 3], 0x04FF).unwrap();
    assert!(w.actors.actor(h).is_none_or(|a| a.killed));
    // Entering with switch 4 saved (the trail's permanent switch flags): no boulder.
    let w = trail_with(&a, |s| s.scene_flags[SCENE_DEATH_MOUNTAIN_TRAIL].swch |= 1 << SWITCH);
    assert!(w.flags.get_switch(SWITCH));
    assert!(boulder(&w).is_none(), "gone with switch 4");
}

#[test]
fn a_bomb_by_the_boulder_breaks_it() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    // Link south of the boulder with a bomb from the bag in hand (C-Right): the watch ignores it
    // (not ACTOR_EN_BOMBF).
    w.place_player(Vec3::new(-1640.0, 681.0, -450.0), i16::MIN);
    idle(&mut w, 2);
    w.tick_with(scripted_input(PadState::default(), PadState { button: BTN_CRIGHT, ..Default::default() }));
    idle(&mut w, 10);
    assert!(w.player().held_actor.is_some_and(|h| w.actors.downcast::<EnBom>(h).is_some()), "a bomb in hand");
    assert_eq!(statics(&mut w).player_bomb, None);
    // A bomb set down 30 east and 70 south of the boulder, two frames from blowing: inside the
    // cylinder (190 round (-1669, -640)), which takes DMG_EXPLOSIVE.
    let h = w.actor_spawn(ACTOR_EN_BOM, Vec3::new(-1649.0, 681.0, -620.0), [0; 3], 0).unwrap();
    w.actors.downcast_mut::<EnBom>(h).unwrap().timer = 2;
    let mut f0 = None;
    for _ in 0..12 {
        idle(&mut w, 1);
        if w.flags.get_switch(SWITCH) {
            f0 = Some(w.audio.frames);
            break;
        }
    }
    let f0 = f0.expect("the boulder broken");
    // func_808B5950 on its AC_HIT: the five pieces, one-point 4180 for 50, switch 4,
    // EVENTCHKINF_23; not drawn, func_808B5A94 from frame 0.
    assert!(w.save.get_event_chk_inf(EVENTCHKINF_23));
    let b = boulder(&w).expect("still there");
    assert_eq!((b.drawn, b.action, b.unk_154, b.unk_158), (false, Action::Broken, 0, 0));
    let mut tys: Vec<i16> = pieces(&w).iter().map(|p| p.ty()).collect();
    tys.sort();
    assert_eq!(tys, vec![0, 1, 2, 3, 4]);
    for p in pieces(&w) {
        // func_808B4E58: its row: scale D[2] / 600, the yaw D[5], spawned at the boulder; gravity
        // -1, at most 10 down.
        let row = D_808B5DD8[p.ty() as usize];
        assert_eq!(p.actor.home_pos, BOULDER);
        assert_eq!((p.actor.world_rot.y, p.actor.scale.x, p.actor.speed_xz), (row[5], row[2] as f32 * (1.0 / 600.0), row[0] as f32));
        assert_eq!((p.actor.gravity, p.actor.min_velocity_y, p.unk_210, p.unk_212), (-1.0, -10.0, row[3], row[4]));
    }
    // OnePointCutscene_Init(4180, 50): the sub camera active, CAM_SET_FREE2, the at (-1881, 766,
    // -330) and the eye (-1979, 703, -269), roll 6, fov 75; Player held (PLAYER_CSACTION_8).
    let sub = w.active_cam_id;
    assert_ne!(sub, CAM_ID_MAIN);
    let c = w.camera(sub).unwrap().clone();
    assert_eq!((c.cs_id, c.setting, c.roll, c.fov), (4180, CAM_SET_FREE2, 6, 75.0));
    assert!(c.at.distance(Vec3::new(-1881.0, 766.0, -330.0)) < 1.0, "at {}", c.at);
    assert!(c.eye.distance(Vec3::new(-1979.0, 703.0, -269.0)) < 1.0, "eye {}", c.eye);
    assert!((48..=50).contains(&c.timer), "timer {}", c.timer);
    idle(&mut w, 1);
    assert_eq!(w.player().cs_mode, 8);
    // func_808B5240: frame 1 (unk_154 counted up first): the rows of frames 0 and 1 (three puffs).
    assert_eq!(boulder(&w).unwrap().unk_158, 3);
    assert!(effects_of(&w, EFFECT_SS_DUST) >= 3);
    // Frame 20: all 26 rows.
    idle(&mut w, 19);
    assert_eq!(boulder(&w).unwrap().unk_158, 26);
    // The pieces land (on the flat round the mouth) and break: their fragments (EffectSsKakera),
    // NA_SE_EV_ROCK_BROKEN; piece 0 spawns piece 5 as it does: six breaks in all.
    let mut saw_five = false;
    for _ in 0..30 {
        idle(&mut w, 1);
        saw_five |= pieces(&w).iter().any(|p| p.ty() == 5);
    }
    assert!(saw_five, "piece 5");
    assert_eq!(sfx_frames(&w, NA_SE_EV_ROCK_BROKEN).len(), 6);
    assert!(pieces(&w).is_empty(), "every piece broken");
    // The chime on frame 56; gone after frame 60.
    while w.audio.frames < f0 + 60 {
        idle(&mut w, 1);
    }
    assert_eq!(sfx_frames(&w, NA_SE_SY_CORRECT_CHIME), vec![f0 + 56]);
    assert!(boulder(&w).is_some(), "frame 60: still there");
    idle(&mut w, 1);
    assert!(boulder(&w).is_none(), "frame 61: gone");
    let _ = EFFECT_SS_KAKERA;
}

#[test]
fn link_is_put_back_out_of_the_boulder_s_footprint() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    // Link 50 behind the boulder's face (south of it is in front: it faces +z): xzDistToPlayer
    // 50, yDistToPlayer -3. func_808B56BC: the yaw to him 0x8000 off its facing, ABS less 0x3FFF
    // is 0x4001; sin(0x4001) × 50 ≈ 50 along its facing (+z): onto the face's plane, z -690.
    w.place_player(Vec3::new(-1679.0, 681.0, -740.0), 0);
    idle(&mut w, 1);
    let p = w.player().actor.world_pos;
    assert!((p.z - -690.0).abs() < 0.01, "{p}");
    assert_eq!(p.x, -1679.0);
    // And kept there (the sphere's push too) while he walks into it.
    for _ in 0..20 {
        let pad = oot_actors::script::stick_towards(&w, BOULDER - Vec3::Z * 200.0, 60.0);
        w.tick_with(scripted_input(PadState::default(), pad));
        assert!(w.player().actor.world_pos.z >= -690.01, "{}", w.player().actor.world_pos);
    }
}

#[test]
fn the_cliff_s_bomb_flower_thrown_down_starts_4180_then_breaks_the_boulder() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    // Link east of the cliff's flower facing west (yaw -0x4000), lifts it (the bracelet).
    w.place_player(FLOWER + Vec3::new(32.0, 0.0, 0.0), -0x4000);
    idle(&mut w, 2);
    w.tick_with(scripted_input(PadState::default(), PadState { button: BTN_A, ..Default::default() }));
    let mut body = None;
    for _ in 0..60 {
        idle(&mut w, 1);
        body = w.player().held_actor.filter(|&h| w.actors.downcast::<EnBombf>(h).is_some_and(|b| b.actor.params == 0));
        if body.is_some() && w.player().action == PA::StandingStill {
            break;
        }
    }
    let body = body.expect("the flower's bomb in hand");
    // func_808B57E0: Link carrying (PLAYER_STATE1_CARRYING_ACTOR) an explosive of ACTOR_EN_BOMBF:
    // sPlayerBomb.
    assert_eq!(statics(&mut w).player_bomb, Some(body));
    // Run west and throw (A at a run: Player_CanThrowCarriedActor) off the cliff (its edge at
    // x -1440; the floor by the boulder is at 681).
    let mut prev = PadState::default();
    let mut thrown = false;
    let mut watch_frame = None;
    for _ in 0..200 {
        let p = w.player();
        let mut pad = PadState::default();
        if !thrown {
            pad = oot_actors::script::stick_towards(&w, Vec3::new(-1700.0, 1408.0, -646.0), 60.0);
            if p.actor.world_pos.x < -1400.0 {
                pad.button = if prev.button & BTN_A != 0 { 0 } else { BTN_A };
                thrown = matches!(p.action, PA::Throw);
            }
        }
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        let Some(b) = w.actors.downcast::<EnBombf>(body) else { break };
        let (y, fuse, xz) = (b.actor.world_pos.y, b.timer, (b.actor.world_pos.x + 1579.0, b.actor.world_pos.z + 790.0));
        let s_timer = statics(&mut w).timer;
        let cam = w.camera(w.active_cam_id).map(|c| (c.cs_id, c.timer));
        if watch_frame.is_none() {
            if y < 1400.0 {
                // The bomb below y 1400 within 400 of (-1579, -790), its fuse running: one-point
                // 4180 for its fuse plus 20, sTimer the same.
                assert!(xz.0 * xz.0 + xz.1 * xz.1 < 400.0 * 400.0);
                assert_eq!(s_timer, fuse + 20);
                assert_eq!(cam.map(|c| c.0), Some(4180));
                watch_frame = Some(w.audio.frames);
            } else {
                assert_eq!(w.active_cam_id, CAM_ID_MAIN, "no cutscene above y 1400 (bomb y {y})");
            }
        }
        if w.flags.get_switch(SWITCH) {
            break;
        }
    }
    let watch_frame = watch_frame.expect("the watch's 4180");
    // It lands by the boulder and blows inside its cylinder: switch 4, EVENTCHKINF_23, the blast's
    // own 4180 (50 frames) after the watch's.
    while !w.flags.get_switch(SWITCH) {
        idle(&mut w, 1);
        assert!(w.audio.frames < watch_frame + 200, "never broke");
    }
    assert!(w.save.get_event_chk_inf(EVENTCHKINF_23));
    let c = w.camera(w.active_cam_id).unwrap();
    assert_eq!(c.cs_id, 4180);
    assert_eq!(boulder(&w).unwrap().action, Action::Broken);
}
