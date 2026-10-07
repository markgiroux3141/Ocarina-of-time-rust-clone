//! The Deku Seed and the Deku Nut as projectiles (`En_Arrow`, `z_en_arrow.c`), the nut's stun
//! (`En_M_Fire1`), their burst (`Effect_Ss_Stone1`) and the nut's flash (Play's
//! `transitionFadeFlash`), against the C, in the Deku Tree (MQ). Player can't shoot yet: each test
//! spawns the projectile as Player's shot leaves it (`Actor_Spawn` at its pitch and yaw; for a
//! seed `unk_A73` 4, as `func_808350A4` sets it), then lets it fly. The real hits the milestone-4
//! tests injected (tests/switches.rs, tests/maruta.rs) are shot here: room 1's eye switch
//! (`Obj_Switch` 0x0C02), room 2's ladder (`Bg_Ydan_Maruta` 0x0121) and room 3's eye (0x1502).
//!
//! The frame: Player updates before the item actions (`unk_A73` counted down first), the
//! projectile's quad is set in its draw (`func_809B4800`) and checked at the start of the next
//! frame (`CollisionCheck_AT`), so a hit shows the frame after the quad reaches.
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use eng_math::{atan2_s, cos_s, sin_s, vec3f_yaw};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_ydan_maruta::{self, BgYdanMaruta};
use oot_actors::en_arrow::{self, ARROW_CS_NUT, ARROW_NUT, ARROW_SEED, Action, EnArrow};
use oot_actors::en_m_fire1::EnMFire1;
use oot_actors::en_st::{self, EnSt};
use oot_actors::obj_switch::{self, ObjSwitch};
use oot_actors::playthrough::{DEKU_TREE_ROOM_STARTS, deku_tree_room_start};
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ActorHandle};
use oot_game::audio::sfx::*;
use oot_game::camera::CAM_ID_MAIN;
use oot_game::collision_check as cc;
use oot_game::effect::{EFFECT_SS_STONE1, EffectSs, stone1};
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

/// Inside the Deku Tree (`deku-tree-inside`), the sound log on, 60 frames in, then at `room`'s
/// debug start, the room's enemies gone unless `keep_enemies`, the attention cameras its clear
/// brings waited out.
fn deku_tree_with(a: &Arc<GameAssets>, room: i8, keep_enemies: bool) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-inside").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 60);
    let &(r, pos, yaw, _) = DEKU_TREE_ROOM_STARTS.iter().find(|s| s.0 == room).expect("a debug start");
    deku_tree_room_start(&mut w, r, pos, yaw);
    if !keep_enemies {
        for h in w.actors.category(ACTORCAT_ENEMY).to_vec() {
            if let Some(a) = w.actors.actor_mut(h) {
                a.kill();
            }
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
    w
}

fn deku_tree(a: &Arc<GameAssets>, room: i8) -> PlayState {
    deku_tree_with(a, room, false)
}

fn arrow(w: &PlayState, h: ActorHandle) -> &EnArrow {
    w.actors.downcast::<EnArrow>(h).expect("En_Arrow")
}

/// Whether the actor is gone (deleted by `Actor_UpdateAll` the frame after its `Actor_Kill`).
fn gone(w: &PlayState, h: ActorHandle) -> bool {
    !w.actors.exists(h)
}

fn sfx_frames(w: &PlayState, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, _)| f).collect()
}

fn sfx_at(w: &PlayState, id: u16) -> Vec<(u32, SfxPos)> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, p)| (f, p)).collect()
}

/// The live soft sprites of type `ty`.
fn effects(w: &PlayState, ty: u8) -> Vec<&EffectSs> {
    w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == ty).collect()
}

/// `Math_Vec3f_Pitch(a, b)`: `Math_Atan2S(Math_Vec3f_DistXZ(a, b), a->y - b->y)`, positive
/// looking down.
fn vec3f_pitch(a: Vec3, b: Vec3) -> i16 {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    atan2_s((dx * dx + dz * dz).sqrt(), a.y - b.y)
}

/// A projectile of kind `params` as Player's shot leaves it: at `pos`, its pitch and yaw
/// (`world.rot`) `rot`, no parent; for a shot (`a73`) Player's `unk_A73` 4.
fn spawn_shot(w: &mut PlayState, params: i16, pos: Vec3, pitch: i16, yaw: i16, a73: u8) -> ActorHandle {
    w.player_mut().unk_A73 = a73;
    w.actor_spawn(en_arrow::ACTOR_EN_ARROW, pos, [pitch, yaw, 0], params).expect("En_Arrow")
}

/// `EnArrow_Fly`'s moves for a seed or nut shot from `start` at `pitch`, `yaw`, with nothing in
/// the way: `Actor_SetProjectileSpeed(80)` (speed `80 cosS(pitch)`, `velocity.y` `80 × -sinS(pitch)`);
/// each frame the timer down from 15, gravity -0.4 once it's under 7.2 (frame 8 on: timer 7),
/// `Actor_MoveXZGravity` (`velocity.xz` from the yaw, `velocity.y` plus the gravity, at least
/// -150; then `R_UPDATE_RATE × 0.5` = 1.5 a frame of it). Positions and velocities after flying
/// frames 1 to 14 (the 15th frame's timer is 0: killed, not moved).
fn flight(start: Vec3, pitch: i16, yaw: i16) -> Vec<(Vec3, Vec3)> {
    let speed = 80.0 * cos_s(pitch);
    let mut vy = 80.0 * -sin_s(pitch);
    let mut gravity = 0.0f32;
    let mut p = start;
    let mut out = Vec::new();
    for k in 1..=14u8 {
        let timer = 15 - k;
        if (timer as f32) < 7.2000003 {
            gravity = -0.4;
        }
        vy += gravity;
        if vy < -150.0 {
            vy = -150.0;
        }
        let v = Vec3::new(sin_s(yaw) * speed, vy, cos_s(yaw) * speed);
        p += v * 1.5 + Vec3::ZERO;
        out.push((p, v));
    }
    out
}

/// The flight cut where `BgCheck_ProjectileLineTest` (prevPos to the new position) first finds
/// a poly: the frame (1-based) and the point.
fn first_poly(w: &PlayState, start: Vec3, path: &[(Vec3, Vec3)]) -> Option<(usize, Vec3)> {
    let mut prev = start;
    for (k, &(p, _)) in path.iter().enumerate() {
        if let Some((hit, _)) = w.col.projectile_line_test(prev, p, true, true, true, true) {
            return Some((k + 1, hit));
        }
        prev = p;
    }
    None
}

/// Room 3's open middle: 400 over its floor (-820), the ceiling the lobby's top (~1000). A
/// level seed shot east (+x, yaw 0x4000) from here meets nothing for its whole flight.
const ROOM3_OPEN: Vec3 = Vec3::new(-718.0, -420.0, 177.0);

/// A seed let go (no parent) with Player's `unk_A73` 0 (no shot): `EnArrow_Shoot` kills it at
/// once, without a sound; `Actor_UpdateAll` deletes it the next frame.
#[test]
fn a_seed_let_go_without_a_shot_is_gone_at_once() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, 3);
    let h = spawn_shot(&mut w, ARROW_SEED, ROOM3_OPEN, 0, 0x4000, 0);
    // EnArrow_Init: no skeleton (params > ARROW_0E), the quad with DMG_SLINGSHOT; waiting to be
    // shot.
    let s = arrow(&w, h);
    assert_eq!((s.action, s.skel.is_none(), s.actor.min_velocity_y), (Action::Shoot, true, -150.0));
    assert_eq!(s.collider.info.at_dmg_info.dmg_flags, cc::DMG_SLINGSHOT);
    assert_eq!(s.collider.base.at_flags, cc::AT_ON | cc::AT_TYPE_PLAYER);
    assert_eq!(s.collider.info.at_elem_flags, cc::ATELEM_ON | cc::ATELEM_NEAREST | cc::ATELEM_SFX_NONE);
    idle(&mut w, 1);
    assert!(arrow(&w, h).actor.killed);
    assert!(sfx_frames(&w, en_arrow::NA_SE_IT_SLING_SHOT).is_empty());
    idle(&mut w, 1);
    assert!(gone(&w, h));
}

/// A seed shot level along +x from room 3's open middle, `unk_A73` 4 (3 when it reads it:
/// Player updates first). The first frame `EnArrow_Shoot`: `NA_SE_IT_SLING_SHOT` at Player, the
/// speed 80 (`Actor_SetProjectileSpeed`: pitch 0, `velocity.y` -0), timer 15, `shape.rot` 0,
/// `unk_210` its position; not moved. Then `EnArrow_Fly` frame by frame as `flight` works out,
/// gravity from frame 8, the quad stretched each draw from the last edge to the new one: 15 ahead
/// of the seed, ±4 (`sPosAOffset` / `sPosBOffset` × 0.01, turned by its yaw). Frame 15: timer 0,
/// killed where it was. Then gone.
#[test]
fn a_shot_seed_flies_15_frames() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, 3);
    let (start, yaw) = (ROOM3_OPEN, 0x4000);
    let path = flight(start, 0, yaw);
    assert_eq!(first_poly(&w, start, &path), None, "the flight meets nothing");
    let player = w.player.unwrap();
    let h = spawn_shot(&mut w, ARROW_SEED, start, 0, yaw, 4);
    idle(&mut w, 1);
    assert_eq!(w.player().unk_A73, 3);
    let shot = w.audio.frames;
    let s = arrow(&w, h);
    assert_eq!((s.action, s.timer, s.actor.world_pos, s.unk_210), (Action::Fly, 15, start, start));
    assert_eq!((s.actor.speed_xz, s.actor.velocity.y, s.actor.gravity), (80.0, -0.0, 0.0));
    assert_eq!((s.actor.shape_rot.x, s.actor.shape_rot.y, s.actor.shape_rot.z), (0, 0, 0));
    assert_eq!(sfx_at(&w, en_arrow::NA_SE_IT_SLING_SHOT), vec![(shot, SfxPos::Actor(player))]);
    // Its draw: the first edge recorded (Player_UpdateWeaponInfo: not active yet), no attack.
    let edge = |p: Vec3, up: f32| p + Vec3::new(sin_s(yaw) * 15.0, up, cos_s(yaw) * 15.0);
    let near = |a: Vec3, b: Vec3| (a - b).abs().max_element() < 1e-3;
    assert!(s.weapon_info.active && near(s.weapon_info.pos_a, edge(start, 4.0)) && near(s.weapon_info.pos_b, edge(start, -4.0)));
    assert!(!w.col_chk.col_at.contains(&cc::ColliderRef { actor: h, id: 0 }));
    let mut prev = start;
    for (k, &(p, v)) in path.iter().enumerate() {
        let k = k + 1;
        idle(&mut w, 1);
        let s = arrow(&w, h);
        assert_eq!((s.action, s.timer as usize), (Action::Fly, 15 - k), "frame {k}");
        assert_eq!(s.actor.gravity, if k >= 8 { -0.4 } else { 0.0 }, "frame {k}");
        assert_eq!((s.actor.world_pos, s.actor.velocity), (p, v), "frame {k}");
        assert_eq!((s.unk_210, s.touched_poly), (prev, false), "frame {k}");
        // The quad from the last edge to this one (Collider_SetQuadVertices(newB, newA, oldB,
        // oldA)), attacking (CollisionCheck_SetAT in the draw).
        let q = s.collider.dim.quad;
        assert!(near(q[0], edge(p, -4.0)) && near(q[1], edge(p, 4.0)) && near(q[2], edge(prev, -4.0)) && near(q[3], edge(prev, 4.0)), "frame {k}: {q:?}");
        assert!(w.col_chk.col_at.contains(&cc::ColliderRef { actor: h, id: 0 }), "frame {k}");
        prev = p;
    }
    // Frame 15: DECR(timer) 0, Actor_Kill, not moved.
    idle(&mut w, 1);
    let s = arrow(&w, h);
    assert!(s.actor.killed);
    assert_eq!((s.timer, s.actor.world_pos), (0, path[13].0));
    idle(&mut w, 1);
    assert!(gone(&w, h));
    assert!(effects(&w, EFFECT_SS_STONE1).is_empty());
}

/// A seed in Player's hand (spawned as his child: `parent` set) waits in `EnArrow_Shoot` whatever
/// `unk_A73` is, unmoved and silent, no quad. Let go (`parent` NULL) with a shot it flies.
#[test]
fn a_held_seed_waits() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, 3);
    let player = w.player.unwrap();
    let h = spawn_shot(&mut w, ARROW_SEED, ROOM3_OPEN, 0, 0x4000, 4);
    w.actors.actor_mut(h).unwrap().parent = Some(player);
    for k in 0..10 {
        if k == 5 {
            w.player_mut().unk_A73 = 4;
        }
        idle(&mut w, 1);
        let s = arrow(&w, h);
        assert_eq!((s.action, s.actor.killed, s.actor.world_pos, s.actor.speed_xz), (Action::Shoot, false, ROOM3_OPEN, 0.0), "frame {k}");
        assert!(!s.weapon_info.active, "frame {k}");
        assert!(!w.col_chk.col_at.contains(&cc::ColliderRef { actor: h, id: 0 }), "frame {k}");
    }
    assert!(sfx_frames(&w, en_arrow::NA_SE_IT_SLING_SHOT).is_empty());
    w.actors.actor_mut(h).unwrap().parent = None;
    w.player_mut().unk_A73 = 4;
    idle(&mut w, 1);
    assert_eq!((arrow(&w, h).action, arrow(&w, h).timer), (Action::Fly, 15));
    assert_eq!(sfx_frames(&w, en_arrow::NA_SE_IT_SLING_SHOT), vec![w.audio.frames]);
}

/// The seed's burst where it hit (`EffectSsStone1_Spawn(pos, false)`, then
/// `EffectSs_UpdateAll` the same frame: life 7): eight frames of `sDrawInfo` by the life left,
/// 7 down to 0 (`gUnknownEffStone1Tex` with white and cyan first, `gUnknownEffStone8Tex` dark
/// red last), deleted at -1.
fn burst_runs_its_eight_frames(w: &mut PlayState, at: Vec3) {
    let e = effects(w, EFFECT_SS_STONE1);
    assert_eq!(e.len(), 1);
    assert_eq!((e[0].pos, e[0].vec, e[0].life, e[0].regs[0]), (at, at, 7, 0));
    assert_eq!(stone1::draw_info(7), Some((1, [255, 255, 255], [0, 255, 255])));
    assert_eq!(stone1::draw_info(0), Some((8, [200, 0, 0], [0, 0, 0])));
    for life in (0..7).rev() {
        idle(w, 1);
        let e = effects(w, EFFECT_SS_STONE1);
        assert_eq!((e.len(), e[0].life, e[0].pos), (1, life, at));
    }
    idle(w, 1);
    assert!(effects(w, EFFECT_SS_STONE1).is_empty());
}

/// From room 3's open middle, a level seed north-east (yaw 0x5000) meets the room's wall: the
/// frame its move crosses it, `BgCheck_ProjectileLineTest` puts it at the hit (`touchedPoly`,
/// `wallPoly`); the next frame (`EnArrow_Fly`: `touchedPoly`, a seed) the burst
/// (`Effect_Ss_Stone1`) and `NA_SE_IT_SLING_REFLECT` from a sound source there for 20 frames
/// (`SfxSource_PlaySfxAtFixedWorldPos`; 19 after this frame's `SfxSource_UpdateAll`), killed.
#[test]
fn a_seed_into_a_wall_bursts() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, 3);
    let (start, yaw) = (ROOM3_OPEN, 0x5000);
    let path = flight(start, 0, yaw);
    let (k_hit, hit) = first_poly(&w, start, &path).expect("a wall");
    let h = spawn_shot(&mut w, ARROW_SEED, start, 0, yaw, 4);
    idle(&mut w, 1);
    for k in 1..k_hit {
        idle(&mut w, 1);
        assert_eq!((arrow(&w, h).actor.world_pos, arrow(&w, h).touched_poly), (path[k - 1].0, false), "frame {k}");
    }
    idle(&mut w, 1);
    let s = arrow(&w, h);
    assert_eq!((s.actor.world_pos, s.touched_poly, s.actor.wall_poly.is_some(), s.actor.killed), (hit, true, true, false));
    idle(&mut w, 1);
    let burst = w.audio.frames;
    let s = arrow(&w, h);
    assert!(s.actor.killed);
    assert_eq!(s.actor.world_pos, hit);
    let reflect = sfx_at(&w, en_arrow::NA_SE_IT_SLING_REFLECT);
    assert_eq!(reflect.len(), 1);
    let (f, SfxPos::Source(i)) = reflect[0] else { panic!("{reflect:?}") };
    assert_eq!(f, burst);
    assert_eq!((w.sfx_sources[i as usize].world_pos, w.sfx_sources[i as usize].countdown), (hit, 19));
    assert!(sfx_frames(&w, en_arrow::NA_SE_IT_DEKU).is_empty());
    // No flash for a seed.
    assert_eq!((w.trans_fade_flash_alpha_step, w.transition_fade_flash.color[3]), (0, 0));
    burst_runs_its_eight_frames(&mut w, hit);
    assert!(gone(&w, h));
}

/// A nut thrown free (`ARROW_NUT`, no parent) flies whether or not Player's `unk_A73` is set
/// (`EnArrow_Shoot` kills only the others), without a sound of its own, and has no quad (its
/// collider isn't set up: params above `ARROW_SEED`); its path is the seed's. A cutscene's nut
/// (`ARROW_CS_NUT`) is a nut that also updates in Player's cutscene modes (`isCsNut`).
#[test]
fn a_nut_thrown_free_flies_whatever_unk_a73() {
    let Some(a) = assets() else { return };
    for a73 in [0, 4] {
        let mut w = deku_tree(&a, 3);
        let (start, yaw) = (ROOM3_OPEN, 0x4000);
        let path = flight(start, 0, yaw);
        let h = spawn_shot(&mut w, ARROW_NUT, start, 0, yaw, a73);
        idle(&mut w, 1);
        let s = arrow(&w, h);
        assert_eq!((s.action, s.timer, s.actor.speed_xz, s.is_cs_nut), (Action::Fly, 15, 80.0, false), "unk_A73 {a73}");
        assert_eq!(s.collider.base.at_flags, 0);
        for (k, &(p, v)) in path.iter().enumerate() {
            idle(&mut w, 1);
            let s = arrow(&w, h);
            assert_eq!((s.actor.world_pos, s.actor.velocity, s.timer as usize), (p, v, 14 - k), "unk_A73 {a73}, frame {}", k + 1);
            assert!(!w.col_chk.col_at.contains(&cc::ColliderRef { actor: h, id: 0 }));
        }
        idle(&mut w, 1);
        assert!(arrow(&w, h).actor.killed);
        for id in [en_arrow::NA_SE_IT_SLING_SHOT, en_arrow::NA_SE_IT_ARROW_SHOT, en_arrow::NA_SE_IT_DEKU] {
            assert!(sfx_frames(&w, id).is_empty());
        }
    }
    let mut w = deku_tree(&a, 3);
    let h = spawn_shot(&mut w, ARROW_CS_NUT, ROOM3_OPEN, 0, 0x4000, 0);
    let s = arrow(&w, h);
    assert_eq!((s.actor.params, s.is_cs_nut), (ARROW_NUT, true));
}

/// A nut into room 3's wall (the seed's wall shot): the frame after the hit (`touchedPoly`, a
/// nut) `R_TRANS_FADE_FLASH_ALPHA_STEP` -1, `En_M_Fire1` at the hit, the burst and `NA_SE_IT_DEKU`
/// there, killed. `En_M_Fire1` (misc, after the item actions) updates the same frame. Then
/// `Play_Update`'s `TransitionFade_Update` of the flash (`TRANS_FADE_TYPE_FLASH`): a negative step,
/// the alpha `Math_StepToS(.., 255, 255)` to 255, the step 150; then each frame the step towards 20
/// by 60 (90, 30, 20, 20 ...) and the alpha down by it: 165, 135, 115, 95, 75, 55, 35, 15, then
/// 0, where the step goes back to 0 and the flash is done. Grey (`RGBA8(160, 160, 160, 255)`),
/// over the screen. `En_M_Fire1`: its timer 0.2, 0.4, 0.6, 0.8 (`Math_StepToF` by 0.2 to 1), the
/// cylinder at the hit (truncated) attacking each of those frames, killed at the fifth.
#[test]
fn a_nut_into_a_wall_flashes_and_stuns() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, 3);
    let (start, yaw) = (ROOM3_OPEN, 0x5000);
    let path = flight(start, 0, yaw);
    let (k_hit, hit) = first_poly(&w, start, &path).expect("a wall");
    assert_eq!(w.transition_fade_flash.fade_type, 2);
    assert_eq!(w.transition_fade_flash.color, [160, 160, 160, 0]);
    let h = spawn_shot(&mut w, ARROW_NUT, start, 0, yaw, 0);
    idle(&mut w, 1 + k_hit);
    assert_eq!((arrow(&w, h).actor.world_pos, arrow(&w, h).touched_poly), (hit, true));
    assert_eq!(w.screen_fill(), None);
    idle(&mut w, 1);
    let burst = w.audio.frames;
    assert!(arrow(&w, h).actor.killed);
    let fires: Vec<ActorHandle> = w.actors.all().into_iter().filter(|&f| w.actors.downcast::<EnMFire1>(f).is_some()).collect();
    assert_eq!(fires.len(), 1);
    let fire = fires[0];
    let f = w.actors.downcast::<EnMFire1>(fire).unwrap();
    assert_eq!((f.actor.world_pos, f.actor.category, f.actor.params), (hit, oot_game::actor_ctx::ACTORCAT_MISC, 0));
    assert!((f.timer - 0.2).abs() < 1e-6);
    assert_eq!(f.collider.dim.pos, [hit.x as i16, hit.y as i16, hit.z as i16]);
    assert_eq!((f.collider.dim.radius, f.collider.dim.height, f.collider.info.at_dmg_info.dmg_flags), (200, 200, cc::DMG_DEKU_NUT));
    assert!(w.col_chk.col_at.contains(&cc::ColliderRef { actor: fire, id: 0 }));
    let deku = sfx_at(&w, en_arrow::NA_SE_IT_DEKU);
    assert_eq!(deku.len(), 1);
    let (fr, SfxPos::Source(i)) = deku[0] else { panic!("{deku:?}") };
    assert_eq!((fr, w.sfx_sources[i as usize].world_pos), (burst, hit));
    assert!(sfx_frames(&w, en_arrow::NA_SE_IT_SLING_REFLECT).is_empty());
    assert_eq!(effects(&w, EFFECT_SS_STONE1).len(), 1);
    // The flash's first frame.
    assert_eq!((w.trans_fade_flash_alpha_step, w.transition_fade_flash.color), (150, [160, 160, 160, 255]));
    assert_eq!(w.screen_fill(), Some([160, 160, 160, 255]));
    let steps = [(90, 165), (30, 135), (20, 115), (20, 95), (20, 75), (20, 55), (20, 35), (20, 15), (0, 0)];
    for (k, &(step, alpha)) in steps.iter().enumerate() {
        idle(&mut w, 1);
        assert_eq!((w.trans_fade_flash_alpha_step, w.transition_fade_flash.color[3]), (step, alpha), "flash frame {}", k + 1);
        assert_eq!(w.transition_fade_flash.is_done, alpha == 0);
        // En_M_Fire1: 0.4, 0.6, 0.8 attacking, then killed (the fifth), then gone.
        match k {
            0..=2 => {
                let f = w.actors.downcast::<EnMFire1>(fire).unwrap();
                assert!((f.timer - 0.2 * (k as f32 + 2.0)).abs() < 1e-5, "{}", f.timer);
                assert!(!f.actor.killed && w.col_chk.col_at.contains(&cc::ColliderRef { actor: fire, id: 0 }));
            }
            3 => {
                let f = w.actors.downcast::<EnMFire1>(fire).unwrap();
                assert!(f.actor.killed && f.timer == 1.0);
                assert!(!w.col_chk.col_at.contains(&cc::ColliderRef { actor: fire, id: 0 }));
            }
            _ => assert!(gone(&w, fire)),
        }
    }
    assert_eq!(w.screen_fill(), None);
    idle(&mut w, 3);
    assert_eq!((w.trans_fade_flash_alpha_step, w.transition_fade_flash.color[3]), (0, 0));
}

/// Room 5's Skulltula brought down by Link 100 east of it, waiting on the ground (as
/// tests/skulltula_st.rs does it: its Song of Time block hidden).
fn skulltula_on_the_ground(a: &Arc<GameAssets>) -> (PlayState, ActorHandle) {
    const HOME5: Vec3 = Vec3::new(-1347.0, -806.0, 1079.0);
    let mut w = deku_tree_with(a, 5, true);
    let h = w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnSt>(h).is_some_and(|s| s.actor.home_pos == HOME5)).expect("room 5's En_St");
    let hide = |w: &mut PlayState| {
        for b in w.actors.all() {
            if let Some(t) = w.actors.downcast_mut::<oot_actors::obj_timeblock::ObjTimeblock>(b) {
                t.actor.params &= !(0x8000u16 as i16);
                t.unk_175 = false;
                t.is_visible = false;
                let bg = t.bg;
                w.col.dyna.set_collision_disabled(bg, true);
            }
        }
    };
    hide(&mut w);
    let (y, _) = w.col.entity_raycast_down(Vec3::new(HOME5.x + 100.0, HOME5.y, HOME5.z));
    w.place_player(Vec3::new(HOME5.x + 100.0, y, HOME5.z), -0x4000);
    let mut n = 0;
    while w.actors.downcast::<EnSt>(h).unwrap().action != en_st::Action::WaitOnGround {
        idle(&mut w, 1);
        hide(&mut w);
        n += 1;
        assert!(n < 60, "it never landed");
    }
    (w, h)
}

/// A nut thrown down at the floor 40 west of room 5's Skulltula on the ground (from 60 further
/// west and 100 up, aimed there: `Math_Vec3f_Pitch`, `Math_Vec3f_Yaw`). It bursts there and its
/// `En_M_Fire1` (200 round, 200 up) attacks with `DMG_DEKU_NUT`: the next frame
/// (`CollisionCheck_AT`) the Skulltula's body cylinder (`colliderCylinders[0]`, the one that takes
/// nuts) is hit, `CollisionCheck_Damage` gives its table's `DMG_ENTRY(0, 1)`, and its update
/// (`EnSt_CheckHitBackside`, damage reaction 1): `NA_SE_EN_GOMA_JR_FREEZE`, `stunTimer` 120, blue
/// for 120 frames, `invulnerableTimer` 8; no damage; the frame goes on stunned (119, 7). The
/// stun's later frames don't stun it again (`stunTimer` isn't 0).
#[test]
fn a_nuts_en_m_fire1_stuns_a_skulltula() {
    let Some(a) = assets() else { return };
    let (mut w, st) = skulltula_on_the_ground(&a);
    let sp = w.actors.downcast::<EnSt>(st).unwrap().actor.world_pos;
    let (floor, _) = w.col.entity_raycast_down(Vec3::new(sp.x - 40.0, sp.y + 50.0, sp.z));
    let target = Vec3::new(sp.x - 40.0, floor, sp.z);
    let start = target + Vec3::new(-60.0, 100.0, 0.0);
    let (pitch, yaw) = (vec3f_pitch(start, target), vec3f_yaw(start, target));
    let path = flight(start, pitch, yaw);
    let (k_hit, hit) = first_poly(&w, start, &path).expect("the floor");
    assert!((hit - target).length() < 2.0, "{hit} {target}");
    let health = w.actors.downcast::<EnSt>(st).unwrap().actor.col_chk_info.health;
    let h = spawn_shot(&mut w, ARROW_NUT, start, pitch, yaw, 0);
    idle(&mut w, 1 + k_hit + 1);
    assert!(arrow(&w, h).actor.killed);
    let fire = w.actors.all().into_iter().find(|&f| w.actors.downcast::<EnMFire1>(f).is_some()).expect("En_M_Fire1");
    assert_eq!(w.actors.downcast::<EnMFire1>(fire).unwrap().actor.world_pos, hit);
    assert_eq!(w.actors.downcast::<EnSt>(st).unwrap().stun_timer, 0);
    idle(&mut w, 1);
    let s = w.actors.downcast::<EnSt>(st).unwrap();
    assert_eq!(sfx_frames(&w, NA_SE_EN_GOMA_JR_FREEZE), vec![w.audio.frames]);
    assert_eq!((s.stun_timer, s.invulnerable_timer, s.actor.col_chk_info.health), (119, 7, health));
    assert_eq!(oot_game::actor::colorfilter_get_duration(s.actor.color_filter_params), 120);
    idle(&mut w, 4);
    assert!(gone(&w, fire));
    assert_eq!(w.actors.downcast::<EnSt>(st).unwrap().stun_timer, 115);
    assert_eq!(sfx_frames(&w, NA_SE_EN_GOMA_JR_FREEZE).len(), 1);
}

fn switch_with(w: &PlayState, params: i16) -> ActorHandle {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<ObjSwitch>(h).is_some_and(|s| s.actor.params == params)).unwrap_or_else(|| panic!("no Obj_Switch {params:#06x}"))
}

/// A seed shot from `start` at `target` (`Math_Vec3f_Pitch`, `Math_Vec3f_Yaw`) until `reacted`
/// says the actor `by` took it (at most 15 frames). Returns the seed, the flying frame the target
/// reacted on, and where the seed burst. The seed bursts the frame the target reacts: both see
/// the hit its last draw's quad made on `by`'s collider (`CollisionCheck_AT` at the frame's
/// start): the target (switch and bg categories first) its `AC_HIT`, the seed its `AT_HIT`
/// (`atTouched`: a seed's burst, at `(pos + prevPos) / 2`, which `Actor_UpdateAll` has just made
/// its position). Its line test had stopped it at the wall behind the target the frame before
/// (`touchedPoly` too).
fn shoot_at(w: &mut PlayState, start: Vec3, target: Vec3, by: ActorHandle, reacted: impl Fn(&PlayState) -> bool) -> (ActorHandle, usize, Vec3) {
    let (pitch, yaw) = (vec3f_pitch(start, target), vec3f_yaw(start, target));
    let h = spawn_shot(w, ARROW_SEED, start, pitch, yaw, 4);
    idle(w, 1);
    for n in 1..=15 {
        let before = arrow(w, h).actor.world_pos;
        idle(w, 1);
        if reacted(w) {
            let s = arrow(w, h);
            assert!(s.actor.killed, "the seed bursts with the hit");
            assert_eq!(s.actor.world_pos, before);
            assert_eq!((s.collider.base.at_flags & cc::AT_HIT, s.collider.base.at, s.touched_poly), (cc::AT_HIT, Some(by), true));
            assert_eq!(sfx_frames(w, en_arrow::NA_SE_IT_SLING_REFLECT), vec![w.audio.frames]);
            let e = effects(w, EFFECT_SS_STONE1);
            assert_eq!((e.len(), e[0].pos, e[0].life), (1, before, 7));
            return (h, n, before);
        }
        assert!(!arrow(w, h).actor.killed, "frame {n}: the seed burst short of its target");
    }
    panic!("the seed never reached its target");
}

/// Room 1's eye switch (0x0C02: eye, once, flag 0x0C) at (-920, 542, 918), facing 0x6000: its
/// triangles 8.5 in front (`sEyeTrisElementsInit`, turned by its yaw: towards +x, -z). A seed
/// from 200 in front, level, at it: yaw -0x2000, `ObjSwitch_EyeIsHit`'s
/// `ABS(-0x2000 - 0x6000)` 0x8000 over 0x5000: the frame of the hit `ObjSwitch_EyeClosingInit` and
/// `ObjSwitch_SetOn` (flag 0x0C, `NA_SE_SY_CORRECT_CHIME`); then shut (`ObjSwitch_EyeClosed`).
#[test]
fn a_seed_closes_room_1s_eye() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, 1);
    let sw = switch_with(&w, 0x0C02);
    let s = w.actors.downcast::<ObjSwitch>(sw).unwrap();
    let home = s.actor.home_pos;
    assert_eq!((home, s.actor.shape_rot.y, s.action), (Vec3::new(-920.0, 542.0, 918.0), 0x6000, obj_switch::Action::EyeOpen));
    assert!(!w.flags.get_switch(0x0C));
    let front = Vec3::new(sin_s(0x6000), 0.0, cos_s(0x6000));
    let (seed, n, _) = shoot_at(&mut w, home + front * 200.0, home, sw, |w| w.flags.get_switch(0x0C));
    // -0x2000 but for the sine table's rounding (-8182).
    assert!((vec3f_yaw(home + front * 200.0, home) as i32 - -0x2000).abs() < 16);
    // 200 away: frame 2's move (to 240 along) meets the wall just behind the eye (203.6 along) and
    // stops there; that draw's quad (its edges 15 ahead: from 135 to 218.6 along) crosses the
    // triangles (8.5 in front of the eye, 191.5 along); frame 3 sees it.
    assert_eq!(n, 3);
    let s = w.actors.downcast::<ObjSwitch>(sw).unwrap();
    assert_eq!(s.action, obj_switch::Action::EyeClosing);
    assert_eq!(sfx_frames(&w, NA_SE_SY_CORRECT_CHIME), vec![w.audio.frames]);
    idle(&mut w, 1);
    assert!(gone(&w, seed));
    for _ in 0..120 {
        if w.actors.downcast::<ObjSwitch>(sw).unwrap().action == obj_switch::Action::EyeClosed {
            break;
        }
        idle(&mut w, 1);
    }
    assert_eq!(w.actors.downcast::<ObjSwitch>(sw).unwrap().action, obj_switch::Action::EyeClosed);
}

/// Room 2's ladder (0x0121) at (-1066, 560, 1066), facing -0x2000: its triangles (x ±16, y 0 to
/// 135, turned by its yaw) upright across (1, 0, 1); the room is on the (-1, 0, 1) side. A seed
/// level from 200 out that side at its middle (67 up): yaw 0x6000. The frame of the hit
/// `func_808BF078`: flag 0x21, `NA_SE_SY_CORRECT_CHIME`, `unk_16A` 20, shaking; 20 frames of it, then
/// the fall to 280 (tests/maruta.rs follows them frame by frame).
#[test]
fn a_seed_drops_room_2s_ladder() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, 2);
    let lad = w.actors.all().into_iter().find(|&h| w.actors.downcast::<BgYdanMaruta>(h).is_some_and(|m| m.actor.params == bg_ydan_maruta::MARUTA_LADDER)).expect("the ladder");
    let home = Vec3::new(-1066.0, 560.0, 1066.0);
    assert_eq!(w.actors.downcast::<BgYdanMaruta>(lad).unwrap().actor.world_pos, home);
    let mid = home + Vec3::Y * 67.0;
    let out = Vec3::new(-1.0, 0.0, 1.0).normalize();
    let start = mid + out * 200.0;
    assert_eq!(vec3f_yaw(start, mid), 0x6000);
    let (_, n, _) = shoot_at(&mut w, start, mid, lad, |w| w.flags.get_switch(0x21));
    // Frame 2's move (to 240 along) stops at the ladder's collision (201.5 along); its quad
    // (to 216.5) reaches past its triangles (200).
    assert_eq!(n, 3);
    let m = w.actors.downcast::<BgYdanMaruta>(lad).unwrap();
    assert_eq!((m.action, m.unk_16a), (bg_ydan_maruta::Action::Shake, 20));
    assert_eq!(sfx_frames(&w, NA_SE_SY_CORRECT_CHIME), vec![w.audio.frames]);
    idle(&mut w, 20 + 24);
    let m = w.actors.downcast::<BgYdanMaruta>(lad).unwrap();
    assert_eq!((m.action, m.actor.world_pos.y), (bg_ydan_maruta::Action::DoNothing, 280.0));
    assert_eq!(sfx_frames(&w, bg_ydan_maruta::NA_SE_EV_LADDER_DOUND).len(), 1);
}

/// Room 3's eye (0x1502) at (-76, -727, 551), facing -z: a seed from below and in front (250
/// out, 110 down, about Link's slingshot height off the floor below), up at it: yaw 0, pitch
/// negative (up). `ObjSwitch_EyeIsHit`: `ABS(0 - -0x8000)` 0x8000. Flag 0x15, closing.
#[test]
fn a_seed_from_below_front_closes_room_3s_eye() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, 3);
    let sw = switch_with(&w, 0x1502);
    let home = w.actors.downcast::<ObjSwitch>(sw).unwrap().actor.home_pos;
    assert_eq!(home, Vec3::new(-76.0, -727.0, 551.0));
    assert!(!w.flags.get_switch(0x15));
    let start = home + Vec3::new(0.0, -110.0, -250.0);
    assert_eq!(vec3f_yaw(start, home), 0);
    assert!(vec3f_pitch(start, home) < 0);
    let (_, n, _) = shoot_at(&mut w, start, home, sw, |w| w.flags.get_switch(0x15));
    // 273 away: frame 3's move (to 360 along) stops at the wall 9 behind the eye; its quad
    // reaches the triangles (8.5 in front); frame 4 sees it.
    assert_eq!(n, 4);
    assert_eq!(w.actors.downcast::<ObjSwitch>(sw).unwrap().action, obj_switch::Action::EyeClosing);
    assert_eq!(sfx_frames(&w, NA_SE_SY_CORRECT_CHIME), vec![w.audio.frames]);
}
