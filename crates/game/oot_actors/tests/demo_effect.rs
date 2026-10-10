//! `Demo_Effect` (GAME-06 milestone 1a) against the C (`z_demo_effect.c`): each type's init, the
//! goddesses' lights and what they leave, the light rings, the blue orb, the light, the
//! Triforce, the jewels, the Song of Time blocks' time warp on its curve skeleton
//! (`z_fcurve_data_skelanime.c`), the vertices its draws write; then the exit's run
//! (`Route::Emerald`): Kokiri Forest's cutscene layer 6, the Kokiri Emerald and the Deku Tree's
//! death.
//!
//! The cues are put on the cutscene's channels by hand (`play->csCtx.actorCues`), and each actor
//! updated alone, as `Actor_UpdateAll` would. Expected values are worked out from the C in the
//! comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::{IVec3, Vec3};
use oot_actors::demo_effect::*;
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::actor::ACTOR_FLAG_UPDATE_DURING_OCARINA;
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::audio::sfx::*;
use oot_game::cutscene::CsCmdActorCue;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, scripted_input};
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

/// `CS_STATE_RUN` (`cutscene.h`).
const CS_STATE_RUN: u8 = 2;

/// The objects these tests load (`object_table.h`).
const OBJECTS: [i16; 7] = [0x008E, 0x008F, 0x0091, 0x0093, 0x0094, 0x0095, 0x00A8];
const OBJECT_GI_JEWEL: i16 = 0x00AD;

/// Link's house (`ENTR_LINKS_HOUSE_0`: few objects of its own, room for the effects'), after the
/// tree's death, three frames run, the effects' objects loaded, the sound log on.
fn forest(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_LINKS_HOUSE_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-dead").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 3);
    for o in OBJECTS.into_iter().chain([OBJECT_GI_JEWEL]) {
        if w.object_ctx.get_index(o).is_none() {
            w.object_ctx.spawn(o);
        }
    }
    w
}

fn spawn(w: &mut PlayState, params: i16, pos: Vec3, rot: [i16; 3]) -> ActorHandle {
    w.actor_spawn(ACTOR_DEMO_EFFECT, pos, rot, params).expect("Demo_Effect spawned")
}

/// The actor's update alone (`Actor_UpdateAll`'s call), as the updating actor.
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// The draw's changes to the actor, as `Actor_DrawAll` makes them.
fn draw_update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.draw_update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

fn draws(w: &PlayState, h: ActorHandle) -> DrawOut {
    let a = w.actors.downcast::<DemoEffect>(h).unwrap();
    let mut out = DrawOut::default();
    let view = ViewInfo { eye: Vec3::ZERO, billboard: glam::Mat4::IDENTITY };
    a.draw(&RenderState::of(&a.actor), w, &view, &mut out);
    out
}

fn fx(w: &PlayState, h: ActorHandle) -> &DemoEffect {
    w.actors.downcast::<DemoEffect>(h).expect("a Demo_Effect")
}

fn effects(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<DemoEffect>(h).is_some_and(|d| !d.actor.killed)).collect()
}

/// A cue on `channel`, from `start` to `end` over the frames `from..to`, the cutscene running at
/// frame `frame`.
fn cue(w: &mut PlayState, channel: usize, action: u16, from: u16, to: u16, start: IVec3, end: IVec3, frame: u16) {
    w.cs_ctx.state = CS_STATE_RUN;
    w.cs_ctx.frames = frame;
    w.cs_ctx.npc_actions[channel] = Some(CsCmdActorCue { action, start_frame: from, end_frame: to, rot: [0; 3], start_pos: start, end_pos: end, normal: IVec3::ZERO });
}

fn sfx_heard(w: &PlayState, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(_, s, _)| s == id)
}

#[test]
fn each_type_starts_with_the_inits_scale_colours_and_cue_channel() {
    let Some(a) = assets() else { return };
    let mut w = forest(&a);
    let at = Vec3::new(0.0, 0.0, 0.0);
    // DEMO_EFFECT_GOD_LGT_DIN: 0.1, prim (255, 170, 255), env (255, 0, 255), cue channel 0.
    let din = spawn(&mut w, 0xF404u16 as i16, at, [0; 3]);
    let d = fx(&w, din);
    assert_eq!((d.actor.scale.x, d.prim_xlu_color, d.env_xlu_color, d.cue_channel, d.u.god_lgt_type()), (0.1, [255, 170, 255], [255, 0, 255], 0, GOD_LGT_DIN));
    // Waiting for its object (DemoEffect_WaitForObject): not drawn yet.
    assert_eq!((d.update_func, d.init_update_func, d.draw), (UpdateFunc::WaitForObject, UpdateFunc::GodLgtDin, None));
    // NAYRU: 0.1 away from Death Mountain Trail's entrance 0, prim (170, 255, 255), env (0, 40,
    // 255), a ring every 5th frame (lightRingSpawnDelay 4), channel 1.
    let nayru = spawn(&mut w, 0xF505u16 as i16, at, [0; 3]);
    let d = fx(&w, nayru);
    assert_eq!((d.actor.scale.x, d.prim_xlu_color, d.env_xlu_color, d.cue_channel, d.u.god_lgt_light_ring_spawn_delay()), (0.1, [170, 255, 255], [0, 40, 255], 1, 4));
    // FARORE at ENTR_KOKIRI_FOREST_0 (0x0EE): 2.4; prim (170, 255, 170), env (0, 200, 0),
    // channel 2.
    w.save.entrance_index = 0x0EE;
    let farore = spawn(&mut w, 0xFF06u16 as i16, at, [0; 3]);
    let d = fx(&w, farore);
    assert_eq!((d.actor.scale.x, d.prim_xlu_color, d.env_xlu_color, d.cue_channel), (2.4, [170, 255, 170], [0, 200, 0], 2));
    // DEMO_EFFECT_LIGHT 0x2112: large (bits 8..11 = 1), DEMO_EFFECT_LIGHT_GREEN (bits 12..15 =
    // 2): prim white, env (0, 200, 0), alpha 255, scale 0, channel 7.
    let light = spawn(&mut w, 0x2112, at, [0; 3]);
    let d = fx(&w, light);
    assert_eq!((d.actor.scale.x, d.prim_xlu_color, d.env_xlu_color, d.u.light_alpha(), d.cue_channel), (0.0, [255, 255, 255], [0, 200, 0], 255, 7));
    // The light rings: expanding from 20 by 4 (FRAMERATE_CONST(20, 6), (4, 5)), seen at 255;
    // the Triforce's at alpha 0 on channel 4; shrinking from 351 by 2.
    let ring = spawn(&mut w, 0x0007, at, [0; 3]);
    assert_eq!((fx(&w, ring).u.light_ring_timer(), fx(&w, ring).u.light_ring_timer_increment(), fx(&w, ring).u.light_ring_alpha()), (20, 4, 255));
    let tri_ring = spawn(&mut w, 0x0011, at, [0; 3]);
    assert_eq!((fx(&w, tri_ring).u.light_ring_alpha(), fx(&w, tri_ring).cue_channel), (0, 4));
    let shrinking = spawn(&mut w, 0x0010, at, [0; 3]);
    assert_eq!((fx(&w, shrinking).u.light_ring_timer(), fx(&w, shrinking).u.light_ring_timer_increment()), (351, 2));
    // The blue orb: 0.05, its counter at 5.
    let orb = spawn(&mut w, 0x0002, at, [0; 3]);
    assert_eq!((fx(&w, orb).actor.scale.x, fx(&w, orb).u.blue_orb_scale(), fx(&w, orb).u.blue_orb_alpha()), (0.05, 5, 255));
    // The Kokiri Emerald: 0.10 outside the Temple of Time, on its side (rot.x 16384), channel 1;
    // DemoEffect_InitJewelColor.
    let jewel = spawn(&mut w, 0x0013, at, [0; 3]);
    let d = fx(&w, jewel);
    assert_eq!((d.actor.scale.x, d.actor.shape_rot.x, d.cue_channel), (0.10, 16384, 1));
    assert_eq!((d.prim_xlu_color, d.env_xlu_color, d.prim_opa_color, d.env_opa_color), ([255, 255, 160], [0, 255, 0], [255, 255, 170], [150, 120, 0]));
    // A medal: GetItem's 0.25, channel 6, GID_MEDALLION_FIRE (0x0C).
    let medal = spawn(&mut w, 0x0009, at, [0; 3]);
    assert_eq!((fx(&w, medal).actor.scale.x, fx(&w, medal).cue_channel, fx(&w, medal).u.get_item_draw_id()), (0.25, 6, 0x0C));
    // The Song of Time block's time warp: updated during the ocarina, env (0, 100, 255).
    let warp = spawn(&mut w, 0x0018, at, [0; 3]);
    assert!(fx(&w, warp).actor.flags & ACTOR_FLAG_UPDATE_DURING_OCARINA != 0);
    assert_eq!(fx(&w, warp).env_xlu_color, [0, 100, 255]);
    // The dust: no draw, channel 2. The fire ball: 0.1. The crystal light: the 0.2 everything
    // starts with.
    let dust = spawn(&mut w, 0x0016, at, [0; 3]);
    assert_eq!((fx(&w, dust).init_draw_func, fx(&w, dust).cue_channel), (None, 2));
    let ball = spawn(&mut w, 0x0001, at, [0; 3]);
    assert_eq!(fx(&w, ball).actor.scale.x, 0.1);
    let crystal = spawn(&mut w, 0x0000, at, [0; 3]);
    assert_eq!(fx(&w, crystal).actor.scale.x, 0.2);
    // Its object in: the first update hands over to the type's update and draw.
    update(&mut w, din);
    assert_eq!((fx(&w, din).update_func, fx(&w, din).draw), (UpdateFunc::GodLgtDin, Some(DrawFunc::GodLgt)));
}

#[test]
fn the_triforce_spawns_its_crystal_light_and_that_its_ring() {
    let Some(a) = assets() else { return };
    let mut w = forest(&a);
    let before = effects(&w).len();
    let tri = spawn(&mut w, 0xFF08u16 as i16, Vec3::new(10.0, 20.0, 30.0), [0; 3]);
    let new: Vec<_> = effects(&w).into_iter().filter(|&h| h != tri).collect();
    assert_eq!(new.len() - before, 2);
    let crystal = *new.iter().find(|&&h| fx(&w, h).effect_type() == DEMO_EFFECT_CRYSTAL_LIGHT).unwrap();
    let ring = *new.iter().find(|&&h| fx(&w, h).effect_type() == DEMO_EFFECT_LIGHTRING_TRIFORCE).unwrap();
    // The Triforce at 0.02 on channel 3; the crystal light its child at 0.6; the ring the crystal
    // light's child (Actor_SpawnAsChild(&crystalLight->actor, ...)) at 0.4.
    assert_eq!((fx(&w, tri).actor.scale.x, fx(&w, tri).cue_channel), (0.020, 3));
    assert_eq!((fx(&w, crystal).actor.parent, fx(&w, crystal).actor.scale.x), (Some(tri), 0.6));
    assert_eq!((fx(&w, ring).actor.parent, fx(&w, ring).actor.scale.x), (Some(crystal), 0.4));
    assert_eq!((fx(&w, tri).actor.child, fx(&w, crystal).actor.child), (Some(crystal), Some(ring)));
}

#[test]
fn the_triforce_fades_in_then_its_column_then_its_crystal_light() {
    let Some(a) = assets() else { return };
    let mut w = forest(&a);
    let tri = spawn(&mut w, 0xFF08u16 as i16, Vec3::ZERO, [0; 3]);
    update(&mut w, tri);
    // DemoEffect_UpdateTriforceSpot on cue 2: primXluColor[0] counts 1 a frame to 140; under 30
    // the Triforce at count × 8.5; to 60 the column at (count - 30) × 8.5; to 140 the crystal
    // light at (count - 60) × 3.1875. The turn: 0x3E8 a frame.
    let p = IVec3::new(100, 200, 300);
    let mut seen = Vec::new();
    for k in 1..=150 {
        cue(&mut w, 3, 2, 0, 1000, p, p, 10);
        update(&mut w, tri);
        draw_update(&mut w, tri);
        let u = fx(&w, tri).u;
        seen.push((k, u.triforce_spot_opacity(), u.light_column_opacity(), u.crystal_light_opacity()));
    }
    let at = |k: usize| seen[k - 1];
    assert_eq!(at(1), (1, 8, 0, 0));
    assert_eq!(at(29), (29, 246, 0, 0));
    assert_eq!(at(30), (30, 255, 0, 0));
    assert_eq!(at(45), (45, 255, 127, 0));
    assert_eq!(at(60), (60, 255, 255, 0));
    assert_eq!(at(100), (100, 255, 255, 127));
    assert_eq!(at(140), (140, 255, 255, 255));
    assert_eq!(at(150), (150, 255, 255, 255));
    assert_eq!(fx(&w, tri).actor.world_pos, Vec3::new(100.0, 200.0, 300.0));
    // 151 updates (the first waited for the object: no turn) of 0x3E8.
    assert_eq!(fx(&w, tri).u.triforce_spot_rotation(), (150i32 * 0x3E8) as i16);
    // The column's eight vertices (86..89, 92..95) carry its opacity in the object's RAM; its
    // draw's vertex colours take it, and the Triforce is solid from 250 (the opaque bake).
    let out = draws(&w, tri);
    assert_eq!(out.opa.len(), 1);
    let column = out.xlu.iter().find(|c| c.params.vertex_colors.is_some()).expect("the column");
    let colors = column.params.vertex_colors.as_ref().unwrap();
    let bank = fx(&w, tri).required_object_slot.unwrap();
    let ram = w.object_ctx.written(bank, TRIFORCE_VTX).unwrap();
    for i in [86, 87, 88, 89, 92, 93, 94, 95] {
        assert_eq!(ram[i * 4 + 3], 255);
    }
    assert!(colors.iter().all(|c| c[3] == 255 || c[3] == 0), "{colors:?}");
    // The sounds of its draw: NA_SE_EV_AURORA and NA_SE_EV_TRIFORCE (Actor_PlaySfx, flagged).
    let mut b = w.actors.take(tri).unwrap();
    w.cur_actor = Some(tri);
    b.draw_sfx(&mut w);
    w.cur_actor = None;
    w.actors.put_back(tri, b);
    assert!(sfx_heard(&w, NA_SE_EV_AURORA - SFX_FLAG) && sfx_heard(&w, NA_SE_EV_TRIFORCE - SFX_FLAG));
}

#[test]
fn din_leaves_fire_balls_that_burst_into_a_blue_orb_and_two_rings() {
    let Some(a) = assets() else { return };
    let mut w = forest(&a);
    let din = spawn(&mut w, 0xF404u16 as i16, Vec3::ZERO, [0; 3]);
    update(&mut w, din);
    // DemoEffect_UpdateGodLgtDin on cue 3: along the cue (from (0, 0, 0) to (100, 0, 0) over 0
    // to 10, at frame 5: half way), facing its end (RAD_TO_BINANG(Math_FAtan2F(100, 0)) =
    // 0x4000), and a fire ball, her child, set to DemoEffect_InitCreationFireball at 0.02.
    let before = effects(&w);
    cue(&mut w, 0, 3, 0, 10, IVec3::ZERO, IVec3::new(100, 0, 0), 5);
    update(&mut w, din);
    assert_eq!((fx(&w, din).actor.world_pos, fx(&w, din).actor.shape_rot.y), (Vec3::new(50.0, 0.0, 0.0), 0x4000));
    let ball = *effects(&w).iter().find(|h| !before.contains(h)).expect("a fire ball");
    assert_eq!(
        (fx(&w, ball).effect_type(), fx(&w, ball).init_update_func, fx(&w, ball).actor.scale.x, fx(&w, ball).actor.parent),
        (DEMO_EFFECT_FIRE_BALL, UpdateFunc::InitCreationFireball, 0.020, Some(din))
    );
    // Its object in, then its start: Din's facing, 50 frames, speed 1.5, gravity -0.03 down to
    // -1.5 (this ROM's values).
    w.cs_ctx.npc_actions[0] = None;
    update(&mut w, ball);
    update(&mut w, ball);
    let b = fx(&w, ball);
    assert_eq!((b.actor.world_rot.y, b.u.fire_ball_timer(), b.actor.speed_xz, b.actor.gravity, b.actor.min_velocity_y, b.update_func), (0x4000, 50, 1.5, -0.03, -1.5, UpdateFunc::CreationFireball));
    // DemoEffect_UpdateCreationFireball: falling forward, its speed -0.015 a frame; 50 frames,
    // then on the 51st the burst, NA_SE_IT_DM_RING_EXPLOSION, and gone.
    for _ in 0..50 {
        update(&mut w, ball);
    }
    assert!(!fx(&w, ball).actor.killed);
    assert!((fx(&w, ball).actor.speed_xz - (1.5 - 0.015 * 50.0)).abs() < 1e-4);
    let before = effects(&w);
    update(&mut w, ball);
    assert!(w.actors.downcast::<DemoEffect>(ball).is_none_or(|d| d.actor.killed));
    // (The category's list is newest first.)
    let mut burst: Vec<_> = effects(&w).into_iter().filter(|h| !before.contains(h)).map(|h| (fx(&w, h).effect_type(), fx(&w, h).actor.scale.x)).collect();
    burst.reverse();
    assert_eq!(burst, vec![(DEMO_EFFECT_BLUE_ORB, 0.0), (DEMO_EFFECT_LIGHTRING_EXPANDING, 0.1), (DEMO_EFFECT_LIGHTRING_SHRINKING, 0.2)]);
    assert!(sfx_heard(&w, NA_SE_IT_DM_RING_EXPLOSION));
}

#[test]
fn nayru_leaves_a_ring_every_fifth_frame_and_farore_a_light_shower() {
    let Some(a) = assets() else { return };
    let mut w = forest(&a);
    let nayru = spawn(&mut w, 0xF505u16 as i16, Vec3::ZERO, [0x100, 0, 0]);
    update(&mut w, nayru);
    let mut spawned = Vec::new();
    for k in 1..=11 {
        let before = effects(&w);
        cue(&mut w, 1, 3, 0, 100, IVec3::ZERO, IVec3::ZERO, 1);
        update(&mut w, nayru);
        for h in effects(&w).into_iter().filter(|h| !before.contains(h)) {
            spawned.push((k, fx(&w, h).effect_type(), fx(&w, h).actor.world_rot.x, fx(&w, h).actor.scale.x));
        }
    }
    // The timer starts at 0: a ring at once, then every lightRingSpawnDelay + 1 = 5 frames; each
    // turned a quarter up from her (world.rot.x + 0x4000), at 1.0.
    let ring = |k| (k, DEMO_EFFECT_LIGHTRING_EXPANDING, 0x4100, 1.0);
    assert_eq!(spawned, vec![ring(1), ring(6), ring(11)]);
    // Farore on cue 3: a light shower 150 below her at (0.23, 0.15, 0.23), her dash.
    let farore = spawn(&mut w, 0xFF06u16 as i16, Vec3::new(0.0, 500.0, 0.0), [0; 3]);
    update(&mut w, farore);
    let before = effects(&w);
    let p = IVec3::new(0, 500, 0);
    cue(&mut w, 2, 3, 0, 100, p, p, 1);
    update(&mut w, farore);
    let shower = *effects(&w).iter().find(|h| !before.contains(h)).expect("the shower");
    assert_eq!((fx(&w, shower).effect_type(), fx(&w, shower).actor.world_pos, fx(&w, shower).actor.scale), (DEMO_EFFECT_LGT_SHOWER, Vec3::new(0.0, 350.0, 0.0), Vec3::new(0.23, 0.15, 0.23)));
    assert!(sfx_heard(&w, NA_SE_IT_DM_FLYING_GOD_DASH));
    // DemoEffect_UpdateLgtShower: 3 off its alpha and ×1.05 a frame while it's over 3: 84
    // frames to 3, gone on the 85th.
    w.cs_ctx.npc_actions[2] = None;
    update(&mut w, shower);
    for _ in 0..84 {
        update(&mut w, shower);
    }
    assert_eq!((fx(&w, shower).u.lgt_shower_alpha(), fx(&w, shower).actor.killed), (3, false));
    update(&mut w, shower);
    assert!(fx(&w, shower).actor.killed);
}

#[test]
fn light_rings_widen_and_fade_out_or_close_in_and_fade_in() {
    let Some(a) = assets() else { return };
    let mut w = forest(&a);
    let ring = spawn(&mut w, 0x0007, Vec3::ZERO, [0; 3]);
    update(&mut w, ring);
    // DemoEffect_UpdateLightRingExpanding: 20 + 4k; alpha 2048 - 8 × timer from 225 (k = 52:
    // 228, alpha 224); past 255 (k = 59: 256) held at 255, gone, the timer 0.
    let mut alphas = Vec::new();
    for _ in 1..=58 {
        update(&mut w, ring);
        alphas.push((fx(&w, ring).u.light_ring_timer(), fx(&w, ring).u.light_ring_alpha()));
    }
    assert_eq!(alphas[50], (224, 255));
    assert_eq!(alphas[51], (228, 224));
    assert_eq!(alphas[57], (252, 32));
    update(&mut w, ring);
    assert_eq!((fx(&w, ring).actor.killed, fx(&w, ring).u.light_ring_timer()), (true, 0));
    // DemoEffect_UpdateLightRingShrinking: 351 - 2k, unseen (alpha 0) until 255 (k = 48, Din's
    // magic: SEQ_CS_EFFECTS_DIN_MAGIC), then 2048 - 8 × timer down to 225 and 255 below it; gone
    // when under 2 (k = 176).
    let shrink = spawn(&mut w, 0x0010, Vec3::ZERO, [0; 3]);
    update(&mut w, shrink);
    let mut seen = Vec::new();
    for _ in 1..=175 {
        update(&mut w, shrink);
        seen.push((fx(&w, shrink).u.light_ring_timer(), fx(&w, shrink).u.light_ring_alpha()));
    }
    assert_eq!(seen[46], (257, 0));
    assert_eq!(seen[47], (255, 8));
    assert_eq!(seen[62], (225, 248));
    assert_eq!(seen[63], (223, 255));
    assert_eq!(seen[174], (1, 255));
    assert!(!fx(&w, shrink).actor.killed);
    update(&mut w, shrink);
    assert!(fx(&w, shrink).actor.killed);
}

#[test]
fn the_blue_orb_grows_for_five_frames_then_shrinks_away() {
    let Some(a) = assets() else { return };
    let mut w = forest(&a);
    let orb = spawn(&mut w, 0x0002, Vec3::ZERO, [0; 3]);
    update(&mut w, orb);
    // DemoEffect_UpdateBlueOrbGrow with no parent: (5 - counter) × 0.01, the counter 5 down to
    // 0; then 15, and DemoEffect_UpdateBlueOrbShrink: alpha counter × 16, ×0.9 a frame, gone at 0.
    let mut scales = Vec::new();
    for _ in 0..6 {
        update(&mut w, orb);
        scales.push(fx(&w, orb).actor.scale.x);
    }
    // (5.0f - counter) × 0.01f in f32: 0.049999997 for 0.05.
    assert_eq!(scales, (0..6).map(|k| (5.0f32 - (5 - k) as f32) * 0.01).collect::<Vec<_>>());
    assert_eq!((fx(&w, orb).u.blue_orb_scale(), fx(&w, orb).update_func), (15, UpdateFunc::BlueOrbShrink));
    update(&mut w, orb);
    assert_eq!((fx(&w, orb).u.blue_orb_alpha(), fx(&w, orb).u.blue_orb_scale()), (240, 14));
    for _ in 0..13 {
        update(&mut w, orb);
    }
    assert!(!fx(&w, orb).actor.killed);
    update(&mut w, orb);
    assert!(fx(&w, orb).actor.killed);
    // Its draw's spin: 0x1F4 a frame (after the matrix it draws with).
    let orb = spawn(&mut w, 0x0002, Vec3::ZERO, [0; 3]);
    update(&mut w, orb);
    draw_update(&mut w, orb);
    draw_update(&mut w, orb);
    assert_eq!(fx(&w, orb).u.blue_orb_rotation(), 0x3E8);
}

#[test]
fn the_light_grows_and_turns_on_its_cue_and_shows_from_its_second_draw() {
    let Some(a) = assets() else { return };
    let mut w = forest(&a);
    let light = spawn(&mut w, 0x2112, Vec3::ZERO, [0; 3]);
    update(&mut w, light);
    // Cue 1: nothing drawn (DemoEffect_CheckForCue(1)), the flicker untouched.
    let p = IVec3::new(5, 6, 7);
    cue(&mut w, 7, 1, 0, 100, p, p, 1);
    update(&mut w, light);
    draw_update(&mut w, light);
    assert_eq!((fx(&w, light).u.light_flicker(), draws(&w, light).xlu.len()), (0, 0));
    // Cue 2, large: +0.05 a frame while rotation (6 a frame) is under 240: 40 frames, then held.
    for _ in 0..50 {
        cue(&mut w, 7, 2, 0, 100, p, p, 2);
        update(&mut w, light);
    }
    let l = fx(&w, light);
    let mut want = 0.0f32;
    for _ in 0..40 {
        want += 0.05;
    }
    assert_eq!((l.actor.scale.x, l.u.light_rotation(), l.u.light_scale_flag(), l.actor.world_pos), (want, 300, 50, Vec3::new(5.0, 6.0, 7.0)));
    // Its first draw on cue 2 only sets the flicker; from the second, two flashes.
    draw_update(&mut w, light);
    assert_eq!((fx(&w, light).u.light_flicker(), draws(&w, light).xlu.len()), (1, 0));
    draw_update(&mut w, light);
    assert_eq!(draws(&w, light).xlu.len(), 2);
    // Cue 3: Math_SmoothStepToF(scale, 0, 0.1, 0.1, 0.005) a frame.
    cue(&mut w, 7, 3, 0, 100, p, p, 3);
    update(&mut w, light);
    let mut s = want;
    eng_math::smooth_step_to_f(&mut s, 0.0, 0.1, 0.1, 0.005);
    assert_eq!(fx(&w, light).actor.scale.x, s);
}

#[test]
fn the_emerald_turns_hidden_on_cue_1_and_sounds_from_one_jewel() {
    let Some(a) = assets() else { return };
    let mut w = forest(&a);
    let jewel = spawn(&mut w, 0x0013, Vec3::ZERO, [0; 3]);
    let ruby = spawn(&mut w, 0x0014, Vec3::ZERO, [0; 3]);
    update(&mut w, jewel);
    update(&mut w, ruby);
    // DemoEffect_UpdateJewelChild, cue 1: placed by it (the default case), turning 0x400 a
    // frame, its timer counting (from its first update after the object's wait), no sound
    // (DemoEffect_PlayJewelSfx skips cue 1), not drawn.
    let p = IVec3::new(2857, 53, -593);
    cue(&mut w, 1, 1, 0, 218, p, p, 10);
    update(&mut w, jewel);
    let j = fx(&w, jewel);
    assert_eq!((j.actor.world_pos, j.actor.shape_rot.y, j.actor.sfx, j.u.jewel_timer()), (Vec3::new(2857.0, 53.0, -593.0), 0x400, 0, 1));
    let out = draws(&w, jewel);
    assert!(out.opa.is_empty() && out.xlu.is_empty());
    // Cue 2: drawn, the gem translucent and the setting opaque; NA_SE_EV_SPIRIT_STONE (flagged)
    // from the first jewel to sound (sSfxJewelId), not from the ruby.
    cue(&mut w, 1, 2, 218, 641, p, p, 300);
    update(&mut w, jewel);
    update(&mut w, ruby);
    assert_eq!(fx(&w, jewel).actor.sfx, NA_SE_EV_SPIRIT_STONE - SFX_FLAG);
    assert_eq!(fx(&w, ruby).actor.sfx, 0);
    let out = draws(&w, jewel);
    assert_eq!((out.xlu.len(), out.opa.len()), (1, 1));
}

#[test]
fn the_song_of_time_blocks_warp_grows_on_its_curve_then_shrinks_and_fades() {
    let Some(a) = assets() else { return };
    let mut w = forest(&a);
    let warp = spawn(&mut w, 0x0018, Vec3::ZERO, [0; 3]);
    update(&mut w, warp);
    // DemoEffect_InitTimeWarp: SkelCurve_SetAnim(1, 59, 1, 1.7) and one SkelCurve_Update at
    // R_UPDATE_RATE 3: frame 1 + 1.7 × 3 × 0.5 = 3.55; the large block's 0.14.
    update(&mut w, warp);
    let d = fx(&w, warp);
    assert_eq!((d.update_func, d.skel_curve.cur_frame, d.actor.scale.x), (UpdateFunc::InitTimeWarpTimeblock, 1.0 + 1.7 * 3.0 * 0.5, 0.14));
    // gTimeWarpAnim curves limb 1's y scale only, from 0.00001 at frame 1 to 20 at 60, cubic
    // with flat ends (flags 12): Curve_CubicHermiteSpline(t, 59 / 30, 1e-5, 20, 0, 0) × 1024.
    let t: f32 = (3.55f32 - 1.0) / (60.0 - 1.0);
    let (t2, t3) = (t * t, t * t * t);
    let (t3x2, t2x3) = (t3 * 2.0, t2 * 3.0);
    let y = (t3x2 - t2x3 + 1.0) * 1e-5 + (t2x3 - t3x2) * 20.0;
    let joints = d.skel_curve.joint_table.as_ref().unwrap();
    assert_eq!(joints[1][1], (y * 1024.0) as i32 as i16);
    assert_eq!((joints[1][0], joints[0]), (1024, [1024, 1024, 1024, 0, 0, 0, 0, 0, 0]));
    // DemoEffect_InitTimeWarpTimeblock: 2.55 a frame past 59: 22 frames, then shrinking.
    for _ in 0..21 {
        update(&mut w, warp);
    }
    assert_eq!(fx(&w, warp).update_func, UpdateFunc::InitTimeWarpTimeblock);
    update(&mut w, warp);
    assert_eq!((fx(&w, warp).update_func, fx(&w, warp).skel_curve.cur_frame), (UpdateFunc::TimeWarpTimeblock, 59.0));
    // DemoEffect_UpdateTimeWarpTimeblock: at its 50th frame, progress 0.5: x and z 0.5 × 0.14,
    // and DemoEffect_TimewarpShrink(0.5): the alphas (s32)(202 × 0.5) = 101 and (s32)(255 ×
    // 0.5) = 127 by sTimewarpVertexSizeIndices, in the object's RAM.
    for _ in 0..50 {
        update(&mut w, warp);
    }
    let d = fx(&w, warp);
    assert_eq!((d.actor.scale.x, d.actor.scale.z, d.actor.scale.y), (0.5 * 0.14, 0.5 * 0.14, 0.14));
    let bank = d.required_object_slot.unwrap();
    let ram = w.object_ctx.written(bank, TIME_WARP_VTX).unwrap().to_vec();
    let alpha: Vec<u8> = (0..21).map(|i| ram[i * 4 + 3]).collect();
    let want: Vec<u8> = S_TIMEWARP_VERTEX_SIZE_INDICES.iter().map(|&k| [0, 101, 127][k as usize]).collect();
    assert_eq!(alpha, want);
    // Its draw: limb 1's gTimeWarpDL with those alphas.
    let out = draws(&w, warp);
    let c = out.xlu.first().expect("the warp drawn");
    assert!(c.params.vertex_colors.as_ref().unwrap().iter().any(|c| c[3] == 101));
    // The 101st: the alphas back (DemoEffect_TimewarpShrink(1)), gone.
    for _ in 0..50 {
        update(&mut w, warp);
    }
    assert!(!fx(&w, warp).actor.killed);
    update(&mut w, warp);
    assert!(fx(&w, warp).actor.killed);
    let ram = w.object_ctx.written(bank, TIME_WARP_VTX).unwrap();
    let back: Vec<u8> = (0..21).map(|i| ram[i * 4 + 3]).collect();
    assert_eq!(back, S_TIMEWARP_VERTEX_SIZE_INDICES.iter().map(|&k| [0, 202, 255][k as usize]).collect::<Vec<_>>());
}

#[test]
fn the_bakes_know_where_their_written_vertices_came_from() {
    let Some(pack) = pack() else { return };
    // gTimeWarpDL loads gTimeWarpVtx's 21 vertices from 0x06000060.
    let d = pack.bake(BAKE_TIME_WARP).unwrap();
    let sources: std::collections::BTreeSet<u32> = d.batches.iter().flat_map(|b| b.sources.iter().copied()).collect();
    assert_eq!(sources, (0..21).map(|i| 0x0600_0060 + i * 16).collect());
    // gTriforceLightColumnDL loads gTriforceVtx[86..96] (0x06000560 on).
    let d = pack.bake(BAKE_TRIFORCE_COLUMN).unwrap();
    let sources: std::collections::BTreeSet<u32> = d.batches.iter().flat_map(|b| b.sources.iter().copied()).collect();
    assert_eq!(sources, (86..96).map(|i| 0x0600_0000 + i * 16).collect());
    assert!(d.batches.iter().all(|b| b.sources.len() == b.vertices.len()));
}

#[test]
fn exit_the_emerald_over_link_and_the_deku_trees_death() {
    let Some(a) = assets() else { return };
    let route = Route::Emerald;
    let e = a.scenes.entrance_index(route.entrance()).expect("entrance");
    let save = route.save(e);
    // CS_INDEX_2: Kokiri Forest's layer 6.
    assert_eq!(save.cutscene_index, 0xFFF2);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    assert_eq!(w.save.scene_layer, 6);
    route.debug_start(&mut w);
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
    }
    assert!(run.failure.is_none(), "{:?}", run.failure);
    eprintln!("steps {:?}; texts {:x?}", run.steps, run.texts);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::EmeraldShown, Step::TreeDeath, Step::EmeraldOver, Step::Forest11]);
    // gKokiriForestKokiriEmeraldPart9Cs's six texts, the Kokiri Emerald's get text among them.
    assert_eq!(run.texts, vec![0x1029, 0x1093, 0x0080, 0x102A, 0x102B, 0x102C]);
    // The terminator (CS_DEST_KOKIRI_FOREST_FROM_KOKIRI_EMERALD): ENTR_KOKIRI_FOREST_11.
    assert_eq!(Some(w.save.entrance_index), a.scenes.entrance_index("ENTR_KOKIRI_FOREST_11"));
    // The emerald's sound (DemoEffect_PlayJewelSfx), the light's white out at the script's frame
    // 197 (Kokiri Forest's layer 6), the tree's death (CS_MISC_DEKU_TREE_DEATH's at 0x30F).
    let log = &w.audio.log.as_ref().unwrap().sfx;
    for id in [NA_SE_EV_SPIRIT_STONE - SFX_FLAG, NA_SE_EV_WHITE_OUT, NA_SE_EV_DEKU_DEATH] {
        assert!(log.iter().any(|&(_, s, _)| s == id), "{id:#x} not heard");
    }
}
