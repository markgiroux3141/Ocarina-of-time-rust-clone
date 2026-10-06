//! `Bg_Ydan_Hasi` (`z_bg_ydan_hasi.c`) against the C, in the Deku Tree (MQ):
//! - room 5's floating block `0xFF00` at (-835, -905, 1050) facing +x (rot y 0x4000), and its
//!   water `0xFF01` at (-835, -915, 1050) on switch flag 0x3F, which nothing in Master Quest sets
//!   ("never runs in Master Quest"): the tests set it;
//! - room 10's three rising platforms `0x3D02` at (-1080, 600, 0), on flag 0x3D, which the room's
//!   floor switch (`Obj_Switch` 0x3D00, once) at (-739, 800, -147) sets.
//!
//! The scene's `waterBoxes[1]` is room 5's pool: x from -1115 (760 across), z from 650 (800
//! across), its surface at -900, for room 5 only. Expected values are worked out from the C in
//! the comments.

mod common;

use std::f64::consts::PI;
use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::{Mat4, Vec3};
use oot_actors::PlayExt;
use oot_actors::bg_ydan_hasi::{self, Action, BgYdanHasi};
use oot_actors::obj_switch::ObjSwitch;
use oot_game::actor::{ACTOR_AUDIO_FLAG_SFX_CENTERED_2, ACTOR_FLAG_SFX_TIMER};
use oot_game::actor_ctx::{ACTOR_BG_YDAN_HASI, ActorHandle, ActorImpl};
use oot_game::audio::sfx::*;
use oot_game::camera::CAM_ID_MAIN;
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

/// Inside the Deku Tree (`deku-tree-inside`), the sound log on, 60 frames in, then in `room`
/// (changed to unless it's room 0), its enemies gone (they'd knock Link about: the props are
/// what's tested), and the attention cameras the room's clear brings waited out.
fn deku_tree(a: &Arc<GameAssets>, room: i8) -> PlayState {
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
        for h in w.actors.category(oot_game::actor_ctx::ACTORCAT_ENEMY).to_vec() {
            if let Some(a) = w.actors.actor_mut(h) {
                a.kill();
            }
        }
        idle(&mut w, 1);
        for _ in 0..600 {
            if w.active_cam_id == CAM_ID_MAIN && w.sub_cameras.iter().flatten().next().is_none() {
                break;
            }
            idle(&mut w, 1);
        }
        assert_eq!(w.active_cam_id, CAM_ID_MAIN);
    }
    w
}

/// Link on the floor below `pos` (found within 50 above), facing `yaw`.
fn place_link(w: &mut PlayState, pos: Vec3, yaw: i16) {
    let (y, _) = w.col.entity_raycast_down(pos + Vec3::Y * 50.0);
    assert!(y > pos.y - 200.0, "no floor below {pos}");
    w.place_player(Vec3::new(pos.x, y, pos.z), yaw);
}

/// The `Bg_Ydan_Hasi` of kind `kind` (its params after the init).
fn hasi_of(w: &PlayState, kind: i16) -> ActorHandle {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<BgYdanHasi>(h).is_some_and(|p| p.actor.params == kind)).unwrap_or_else(|| panic!("no Bg_Ydan_Hasi of kind {kind}"))
}

fn hasi(w: &PlayState, h: ActorHandle) -> &BgYdanHasi {
    w.actors.downcast::<BgYdanHasi>(h).expect("Bg_Ydan_Hasi")
}

/// What the actor's draw submits: (opaque, translucent) draw counts.
fn draws(w: &PlayState, h: ActorHandle) -> (usize, usize) {
    let p = hasi(w, h);
    let mut out = DrawOut::default();
    p.draw(&p.render_state(), w, &ViewInfo::new(Vec3::ZERO, Mat4::IDENTITY), &mut out);
    (out.opa.len(), out.xlu.len())
}

/// `Actor_PlaySfx_FlaggedTimer`'s sound for `timer`: `NA_SE_PL_WALK_DIRT - SFX_FLAG` under 40,
/// `_CONCRETE`'s under 100, else `_SAND`'s.
fn timer_sfx(timer: i16) -> u16 {
    if timer < 40 {
        NA_SE_PL_WALK_DIRT - SFX_FLAG
    } else if timer < 100 {
        NA_SE_PL_WALK_CONCRETE - SFX_FLAG
    } else {
        NA_SE_PL_WALK_SAND - SFX_FLAG
    }
}

/// The actor's sound this frame: `Actor_PlaySfx_FlaggedCentered2` (centered) or `_FlaggedTimer`.
fn centered2(p: &BgYdanHasi) -> Option<u16> {
    (p.actor.flags & ACTOR_AUDIO_FLAG_SFX_CENTERED_2 != 0).then_some(p.actor.sfx)
}
fn timer_tick(p: &BgYdanHasi) -> Option<u16> {
    (p.actor.flags & ACTOR_FLAG_SFX_TIMER != 0).then_some(p.actor.sfx)
}

/// `BgYdanHasi_UpdateFloatingBlock`'s bob for `timer`: `2.0f * sinf(timer * (M_PI / 25))`.
fn bob(timer: i16) -> f32 {
    2.0 * ((timer as f64 * (PI / 25.0)) as f32).sin()
}

const ROOM5_BLOCK: Vec3 = Vec3::new(-835.0, -905.0, 1050.0);
const ROOM10_PLATFORMS: Vec3 = Vec3::new(-1080.0, 600.0, 0.0);
const ROOM10_SWITCH: Vec3 = Vec3::new(-739.0, 800.0, -147.0);

/// `waterBoxes[1]` is room 5's pool. Before room 5 is loaded its surface is the scene's, -900;
/// room 5's water (`0xFF01`) lowers it: home -915 + -5 = -920 (`waterBox->ySurface =
/// world.pos.y = home.pos.y += -5.0f`), on flag `type` 0x3F (`PARAMS_GET_U(0xFF01, 8, 6)`),
/// waiting (`BgYdanHasi_InitWater`), drawn translucent with the scroll; it sets no bg actor.
#[test]
fn room_5s_water_lowers_water_box_1_at_init() {
    let Some(a) = assets() else { return };
    let w = deku_tree(&a, 0);
    let b = w.col.header.water_boxes[bg_ydan_hasi::WATER_BOX];
    assert_eq!((b.x_min, b.z_min, b.x_length, b.z_length, b.y_surface), (-1115, 650, 760, 800, -900));
    // WATERBOX_ROOM: bits 13..18.
    assert_eq!((b.properties >> 13) & 0x3F, 5);
    let w = deku_tree(&a, 5);
    let h = hasi_of(&w, bg_ydan_hasi::HASI_WATER);
    let p = hasi(&w, h);
    assert_eq!((p.actor.home_pos, p.actor.world_pos), (Vec3::new(-835.0, -920.0, 1050.0), Vec3::new(-835.0, -920.0, 1050.0)));
    assert_eq!((p.action, p.ty, p.timer, p.bg), (Action::InitWater, 0x3F, 0, eng_collision::dyna::BG_ACTOR_MAX));
    assert_eq!(p.dyna_bg_id(), None);
    assert_eq!(w.col.water_box_surface(bg_ydan_hasi::WATER_BOX), Some(-920));
    assert!(!w.flags.get_switch(0x3F));
    assert_eq!(draws(&w, h), (0, 1));
    // Gfx_TwoTexScroll(.., -frames % 128, frames % 128, .., frames % 128, frames % 128, ..) with
    // gameplayFrames a u32: at frame 5, tile 0's x is (2^32 - 5) % 128 = 123 and its y 5
    // (gDPSetTileSize(tile, x, y, x + ((0x20 - 1) << 2), y + ((0x20 - 1) << 2))).
    let s = bg_ydan_hasi::water_scroll(5);
    assert_eq!(s[1], (0xF200_0000 | 123 << 12 | 5, (123 + 124) << 12 | (5 + 124)));
    assert_eq!(s[3].1 >> 24, 1, "tile 1");
}

/// Room 5's floating block. A new one spawned on the lowered water: 20 above it (-920 + 20),
/// home kept, scale 0.1 with x and z 0.15, its bg actor set, the timer 0. Then the room's own,
/// frame by frame: `x = home.x + sinS(0x4000) · sinf((frames & 0xFF) · π/128) · 165` (z the same
/// with `cosS(0x4000)`, 0), `y = ySurface + 20 + 2 sinf(timer · π/25)`, the timer counting 50
/// down to 1 and back to 50; its collision's top at the truncated height.
#[test]
fn room_5s_block_floats_slides_and_bobs() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, 5);
    let n = w.actor_spawn(ACTOR_BG_YDAN_HASI, ROOM5_BLOCK, [0, 0x4000, 0], 0xFF00u16 as i16).expect("spawn");
    let p = hasi(&w, n);
    assert_eq!((p.action, p.ty, p.timer, p.actor.params), (Action::UpdateFloatingBlock, 0x3F, 0, bg_ydan_hasi::HASI_WATER_BLOCK));
    assert_eq!((p.actor.world_pos, p.actor.home_pos), (Vec3::new(-835.0, -900.0, 1050.0), ROOM5_BLOCK));
    assert_eq!(p.actor.scale, Vec3::new(0.15, 0.1, 0.15));
    assert!(w.col.dyna.is_bg_actor(p.bg));
    assert_eq!(draws(&w, n), (1, 0));
    w.actors.actor_mut(n).unwrap().kill();
    idle(&mut w, 1);

    let h = hasi_of(&w, bg_ydan_hasi::HASI_WATER_BLOCK);
    let mut timer = hasi(&w, h).timer;
    for k in 1..=300 {
        idle(&mut w, 1);
        timer -= 1;
        if timer == 0 {
            timer = 50;
        }
        let f = w.gameplay_frames;
        let slide = (((f & 0xFF) as f64 * (PI / 128.0)) as f32).sin() * 165.0;
        let p = hasi(&w, h);
        assert_eq!(p.timer, timer, "frame {k}");
        let want = Vec3::new(eng_math::sin_s(0x4000) * slide + -835.0, -920.0 + 20.0 + bob(timer), eng_math::cos_s(0x4000) * slide + 1050.0);
        assert_eq!(p.actor.world_pos, want, "frame {k}");
        // DynaPoly_UpdateContext after the BG category: the top face from the s16 vertices.
        let (top, poly) = w.col.entity_raycast_down(Vec3::new(want.x, -850.0, want.z));
        assert_eq!((top, poly.map(|p| p.bg)), (want.y as i16 as f32, Some(p.bg)), "frame {k}");
    }
}

/// Room 5's water with flag 0x3F set (as nothing in MQ does), Link standing on the pool's bank
/// at (-500, -880, 950), inside the water box. Frame 1, `BgYdanHasi_InitWater`: 600 frames and
/// `BgYdanHasi_MoveWater`. Frames 2 to 95: down 0.5 a frame to home - 47 (-967, 94 steps), the
/// water box's surface the height truncated (`(s16)`, -920.5 to -920), `NA_SE_EV_WATER_LEVEL_DOWN`
/// centered; at -967 `BgYdanHasi_DecWaterTimer`. Frames 96 to 695: the timer 599 down to 0, its
/// tick each frame; at 0 back to `MoveWater`. Frames 696 to 742: up 1 a frame to home (47), the
/// sound; at home the flag unset and `InitWater`. Each frame Link's `depthInWater` (the actors'
/// `yDistToWater`, from `Actor_UpdateBgCheckInfo` after the BG category) is the new surface less
/// his height, and the block floats 20 above it.
#[test]
fn room_5s_water_sinks_waits_and_rises_on_flag_0x3f() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, 5);
    place_link(&mut w, Vec3::new(-500.0, -880.0, 950.0), 0);
    idle(&mut w, 3);
    let link_y = w.player().actor.world_pos.y;
    assert_eq!(link_y, -880.0);
    assert_eq!(w.player().actor.y_dist_to_water, -920.0 - -880.0);
    let h = hasi_of(&w, bg_ydan_hasi::HASI_WATER);
    let block = hasi_of(&w, bg_ydan_hasi::HASI_WATER_BLOCK);
    w.flags.set_switch(0x3F);
    idle(&mut w, 1);
    let p = hasi(&w, h);
    assert_eq!((p.action, p.timer, p.actor.world_pos.y), (Action::MoveWater, 600, -920.0));
    assert_eq!(w.col.water_box_surface(1), Some(-920));

    let check = |w: &PlayState, k: u32, y: f32| {
        let surface = y as i32 as i16;
        assert_eq!(w.col.water_box_surface(1), Some(surface), "frame {k}");
        let l = &w.player().actor;
        assert_eq!((l.world_pos.y, l.y_dist_to_water), (link_y, surface as f32 - link_y), "frame {k}");
        assert_eq!(l.bg_check_flags & oot_game::actor::BGCHECKFLAG_WATER, 0, "frame {k}");
        let b = hasi(w, block);
        assert_eq!(b.actor.world_pos.y, surface as f32 + 20.0 + bob(b.timer), "frame {k}");
    };
    let mut y = -920.0f32;
    for k in 2..=95 {
        idle(&mut w, 1);
        y -= 0.5;
        let p = hasi(&w, h);
        assert_eq!(p.actor.world_pos.y, y, "frame {k}");
        assert_eq!(p.action, if k == 95 { Action::DecWaterTimer } else { Action::MoveWater }, "frame {k}");
        assert_eq!(centered2(p), Some(bg_ydan_hasi::NA_SE_EV_WATER_LEVEL_DOWN - SFX_FLAG), "frame {k}");
        check(&w, k, y);
    }
    assert_eq!(y, -967.0);
    for k in 96..=695 {
        idle(&mut w, 1);
        let t = 600 - (k - 95) as i16;
        let p = hasi(&w, h);
        assert_eq!((p.timer, p.actor.world_pos.y), (t, -967.0), "frame {k}");
        assert_eq!(p.action, if t == 0 { Action::MoveWater } else { Action::DecWaterTimer }, "frame {k}");
        assert_eq!(timer_tick(p), Some(timer_sfx(t)), "frame {k}");
        check(&w, k, y);
    }
    for k in 696..=742 {
        idle(&mut w, 1);
        y += 1.0;
        let p = hasi(&w, h);
        assert_eq!(p.actor.world_pos.y, y, "frame {k}");
        assert_eq!(p.action, if k == 742 { Action::InitWater } else { Action::MoveWater }, "frame {k}");
        assert_eq!(w.flags.get_switch(0x3F), k != 742, "frame {k}");
        assert_eq!(centered2(p), Some(bg_ydan_hasi::NA_SE_EV_WATER_LEVEL_DOWN - SFX_FLAG), "frame {k}");
        check(&w, k, y);
    }
    assert_eq!(y, -920.0);
    // The sound logged through Actor_UpdateFlaggedAudio: Sfx_PlaySfxCentered2, then the timer's
    // NA_SE_SY_TIMER (func_800F4C58).
    let log = &w.audio.log.as_ref().unwrap().sfx;
    assert!(log.iter().any(|&(_, s, _)| s == bg_ydan_hasi::NA_SE_EV_WATER_LEVEL_DOWN - SFX_FLAG));
    // Waiting again: nothing moves.
    idle(&mut w, 30);
    let p = hasi(&w, h);
    assert_eq!((p.action, p.actor.world_pos.y), (Action::InitWater, -920.0));
    assert_eq!(w.col.water_box_surface(1), Some(-920));
}

/// The platforms' frames from the flag, each against the C. Frame 1 (`SetupThreeBlocks`, the
/// flag seen): timer 260, drawn, one-point cutscene 3040 on them for 30 frames, not moved yet.
/// Then `UpdateThreeBlocks`: frames 2 to 41 up 3 a frame to home + 120 (the elevator's sound
/// while short of it, the timer's tick on the frame it gets there), frames 42 to 260 the timer's
/// tick, frame 261 the timer out and down 3, frames 261 to 300 down to home (the elevator's
/// sound), and on frame 300 the flag unset, undrawn, `SetupThreeBlocks`. `each(w, k, y)` checks
/// more on frame `k` with the platforms at `y`.
fn run_platforms(w: &mut PlayState, h: ActorHandle, each: &mut dyn FnMut(&PlayState, u32, f32)) {
    let mut y = 600.0f32;
    for k in 2..=300u32 {
        idle(w, 1);
        let p = hasi(w, h);
        let t = 260 - (k - 1) as i16;
        assert_eq!(p.timer, t.max(0), "frame {k}");
        let sound;
        if k <= 260 {
            if y < 720.0 {
                y += 3.0;
                sound = if y < 720.0 { centered2(p) } else { timer_tick(p) };
                assert_eq!(sound, Some(if y < 720.0 { bg_ydan_hasi::NA_SE_EV_ELEVATOR_MOVE - SFX_FLAG } else { timer_sfx(t) }), "frame {k}");
            } else {
                assert_eq!(timer_tick(p), Some(timer_sfx(t)), "frame {k}");
            }
        } else {
            y -= 3.0;
            if y > 600.0 {
                assert_eq!(centered2(p), Some(bg_ydan_hasi::NA_SE_EV_ELEVATOR_MOVE - SFX_FLAG), "frame {k}");
            }
        }
        assert_eq!(p.actor.world_pos, Vec3::new(-1080.0, y, 0.0), "frame {k}");
        assert_eq!(p.action, if k == 300 { Action::SetupThreeBlocks } else { Action::UpdateThreeBlocks }, "frame {k}");
        assert_eq!(p.drawn, k != 300, "frame {k}");
        assert_eq!(w.flags.get_switch(0x3D), k != 300, "frame {k}");
        if k == 41 {
            assert_eq!(y, 720.0);
        }
        each(w, k, y);
    }
    assert_eq!(y, 600.0);
}

/// Room 10's platforms: undrawn and still until Link steps on the room's floor switch (0x3D00,
/// once). The switch sets 0x3D in its update (the SWITCH category, before BG), so the platforms
/// see it that frame: `BgYdanHasi_SetupThreeBlocks` (one-point cutscene 3040 on them, `focus` 40
/// up), and they rise, wait and come down as `run_platforms` checks.
#[test]
fn room_10s_floor_switch_raises_the_platforms() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, 10);
    let h = hasi_of(&w, bg_ydan_hasi::HASI_THREE_BLOCKS);
    let p = hasi(&w, h);
    // PARAMS_GET_U(0x3D02, 8, 6): 0x3D; Actor_SetFocus(40).
    assert_eq!((p.action, p.ty, p.timer, p.drawn), (Action::SetupThreeBlocks, 0x3D, 0, false));
    assert_eq!((p.actor.world_pos, p.actor.focus_pos), (ROOM10_PLATFORMS, ROOM10_PLATFORMS + Vec3::Y * 40.0));
    assert!(w.col.dyna.is_bg_actor(p.bg));
    assert_eq!(draws(&w, h), (0, 0));
    // Link away from the switch: nothing.
    place_link(&mut w, Vec3::new(-700.0, 800.0, 100.0), 0);
    idle(&mut w, 30);
    let p = hasi(&w, h);
    assert_eq!((p.action, p.actor.world_pos, p.drawn), (Action::SetupThreeBlocks, ROOM10_PLATFORMS, false));
    assert_eq!(draws(&w, h), (0, 0));
    // On the switch.
    let sw = w.actors.all().into_iter().find(|&s| w.actors.downcast::<ObjSwitch>(s).is_some_and(|s| s.actor.params == 0x3D00)).expect("room 10's switch");
    place_link(&mut w, ROOM10_SWITCH, 0);
    let mut n = 0;
    while !w.flags.get_switch(0x3D) {
        assert_eq!(hasi(&w, h).action, Action::SetupThreeBlocks);
        idle(&mut w, 1);
        n += 1;
        assert!(n < 10, "the switch isn't pressed");
    }
    let p = hasi(&w, h);
    assert_eq!((p.action, p.timer, p.drawn, p.actor.world_pos), (Action::UpdateThreeBlocks, 260, true, ROOM10_PLATFORMS));
    assert_eq!(draws(&w, h), (1, 0));
    let cam = w.sub_cameras.iter().flatten().find(|c| c.cs_id == 3040).expect("one-point cutscene 3040");
    assert_eq!(cam.target, Some(h));
    assert_eq!(w.actors.downcast::<ObjSwitch>(sw).unwrap().actor.params, 0x3D00);
    run_platforms(&mut w, h, &mut |_, _, _| {});
    assert_eq!(draws(&w, h), (0, 0));
}

/// Link standing over the middle platform's spot on the lower floor (720), the flag set: it rises
/// under him from inside the floor (its top home + 80: 680 at rest), and once its top is above
/// the floor he stands on it (`floorBgId` its bg actor) and is carried up with it
/// (`DYNA_TRANSFORM_POS`) to 800; coming down he's carried down to the floor, where he stays.
#[test]
fn the_platforms_carry_link_up_and_let_him_down() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, 10);
    let h = hasi_of(&w, bg_ydan_hasi::HASI_THREE_BLOCKS);
    let bg = hasi(&w, h).bg;
    place_link(&mut w, Vec3::new(-950.0, 720.0, -110.0), 0);
    idle(&mut w, 3);
    assert_eq!(w.player().actor.world_pos.y, 720.0);
    assert!(w.player().actor.floor_bg_id != bg);
    w.flags.set_switch(0x3D);
    idle(&mut w, 1);
    assert_eq!(hasi(&w, h).action, Action::UpdateThreeBlocks);
    // The platform's top is y + 80; Link stands on whichever floor is higher (the platform's
    // moved by DynaPoly_UpdateContext after the BG category, before Player).
    let mut carried = 0;
    run_platforms(&mut w, h, &mut |w, k, y| {
        let l = &w.player().actor;
        let top = y + 80.0;
        assert_eq!(l.world_pos.y, top.max(720.0), "frame {k}");
        if top > 720.0 {
            assert_eq!(l.floor_bg_id, bg, "frame {k}");
            carried += 1;
        }
    });
    // On it while its top is above the floor's 720 (y over 640): going up from frame 15 (y 642)
    // to 41 (27 frames), at the top from 42 to 260 (219), coming down from 261 to 286 (y 642, 26).
    assert_eq!(carried, 27 + 219 + 26);
    let l = &w.player().actor;
    assert_eq!(l.world_pos.y, 720.0);
    assert!(l.floor_bg_id != bg);
}
