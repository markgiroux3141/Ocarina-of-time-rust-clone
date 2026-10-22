//! GAME-10 milestone 7 against the C: Ganondorf's cape (`z_en_ganon_mant.c`) and `En_Viewer`'s
//! writes to it (`EnViewer_UpdateGanondorfCape`, `z_en_viewer.c`).
//!
//! The capes are spawned in Zora's River (any loaded scene does: the cape's object is
//! `gameplay_keep`) and driven directly: their fields set as the parent writes them, the draw's
//! strand update (`EnGanonMant_Draw`'s, `draw_strands`) and the update run alone. Nothing waits on
//! `Rand`: the tear's numbers are taken from a copy of the generator. Expected values are worked
//! out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::en_ganon_mant::*;
use oot_actors::en_viewer::{ACTOR_EN_VIEWER, EnViewer, Statics};
use oot_game::actor_ctx::ActorHandle;
use oot_game::env::clock_time;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::sys_matrix::MtxF;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn river(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_ZORAS_RIVER_0").expect("entrance");
    let mut save = SaveContext::new(e, false, clock_time(10, 0) as u16);
    save.apply_preset("fairy-magic").unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    for _ in 0..2 {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
    w
}

/// A cape of `params` at the origin, no parent.
fn spawn_cape(w: &mut PlayState, params: i16) -> ActorHandle {
    w.actor_spawn(ACTOR_EN_GANON_MANT, Vec3::ZERO, [0; 3], params).expect("En_Ganon_Mant")
}

fn cape(w: &mut PlayState, h: ActorHandle) -> &mut EnGanonMant {
    w.actors.downcast_mut::<EnGanonMant>(h).expect("En_Ganon_Mant")
}

/// The actor's update alone (`Actor_UpdateAll`'s call).
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("spawned");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// An `En_Viewer` of `ty` facing `yaw`, and the cape its init spawned (its child).
fn viewer(w: &mut PlayState, ty: i16, yaw: i16) -> (ActorHandle, ActorHandle) {
    let v = w.actor_spawn(ACTOR_EN_VIEWER, Vec3::new(0.0, 0.0, 0.0), [0, yaw, 0], ty << 8).expect("En_Viewer");
    let c = w.actors.actor(v).and_then(|a| a.child).expect("the cape");
    (v, c)
}

fn near(a: Vec3, b: Vec3, eps: f32) -> bool {
    (a - b).abs().max_element() <= eps
}

/// The cape hanging still: `params` 1 (`Boss_Ganon`'s), the forearms 40 apart at y 100, gravity
/// -3, no floor. After 300 draws each strand hangs straight down from its root, its joints 9.5
/// apart; the roots on the half circle of radius 20 between the forearms, strand 0 at the right
/// one, 11 at the left.
#[test]
fn the_strands_settle_under_gravity() {
    let Some(a) = assets() else { return };
    let mut w = river(&a);
    let h = spawn_cape(&mut w, PARAMS_BOSS);
    let c = cape(&mut w, h);
    let (right, left) = (Vec3::new(-20.0, 100.0, 0.0), Vec3::new(20.0, 100.0, 0.0));
    c.right_forearm_pos = right;
    c.left_forearm_pos = left;
    c.gravity = -3.0;
    c.min_y = -10000.0;
    for _ in 0..300 {
        c.update_has_run = true;
        c.draw_strands();
    }
    assert!(near(c.strands[0].root, right, 0.01), "{:?}", c.strands[0].root);
    assert!(near(c.strands[11].root, left, 0.01), "{:?}", c.strands[11].root);
    let mid = Vec3::new(0.0, 100.0, 0.0);
    for (i, s) in c.strands.iter().enumerate() {
        assert!(((s.root - mid).length() - 20.0).abs() < 0.01, "strand {i}: root {:?}", s.root);
        for j in 0..NUM_JOINTS {
            let want = s.root - Vec3::new(0.0, 9.5 * j as f32, 0.0);
            // Within 0.1: the table's angles (`Math_Atan2S`) leave the joints a hair off plumb.
            assert!(near(s.joints[j], want, 0.1), "strand {i} joint {j}: {:?} not {want:?}", s.joints[j]);
            if j > 0 {
                assert!(((s.joints[j] - s.joints[j - 1]).length() - 9.5).abs() < 1e-3, "strand {i} joint {j}");
            }
        }
    }
}

/// One strand update from rest, every joint at the root (0, 100, 0), gravity -13.
/// Params 1: joint 1 drops 9.5 straight down, its speed -7.6 kept to -5 (`vel->y < -5.0f`).
/// Params 0x23: every joint first pushed 30 back (-z, the yaw 0), so joint 1 goes 6.5 along
/// (0, -13, -30): (0, 97.4155, -5.9641), its speed 0.8 × (0, -2.5845, 24.0359) = (0, -2.0676,
/// 19.2287), not kept to 5. Either way joint 11's speed takes joint 10's x and y (the C's
/// `rot[11]` past the end) and its rotation stays 0.
#[test]
fn the_speeds_are_kept_within_5_except_with_params_0x23() {
    let Some(a) = assets() else { return };
    let mut w = river(&a);
    for params in [PARAMS_BOSS, PARAMS_VIEWER] {
        let h = spawn_cape(&mut w, params);
        let c = cape(&mut w, h);
        let root = Vec3::new(0.0, 100.0, 0.0);
        c.strands[0].root = root;
        c.strands[0].joints = [root; NUM_JOINTS];
        c.gravity = -13.0;
        c.min_y = -10000.0;
        c.update_strand(0, 1);
        let s = c.strands[0];
        if params == PARAMS_BOSS {
            assert!(near(s.joints[1], Vec3::new(0.0, 90.5, 0.0), 0.01), "{:?}", s.joints[1]);
            assert_eq!(s.velocities[1].y, -5.0);
            for j in 1..NUM_JOINTS {
                assert!(s.velocities[j].abs().max_element() <= 5.0, "joint {j}: {:?}", s.velocities[j]);
                assert!(((s.joints[j] - s.joints[j - 1]).length() - 9.5).abs() < 0.01, "joint {j}");
            }
        } else {
            assert!(near(s.joints[1], Vec3::new(0.0, 97.4155, -5.9641), 0.01), "{:?}", s.joints[1]);
            assert!(near(s.velocities[1], Vec3::new(0.0, -2.0676, 19.2287), 0.01), "{:?}", s.velocities[1]);
            for j in 1..NUM_JOINTS {
                assert!(((s.joints[j] - s.joints[j - 1]).length() - 6.5).abs() < 0.01, "joint {j}");
            }
        }
        assert_eq!((s.velocities[11].x, s.velocities[11].y), (s.velocities[10].x, s.velocities[10].y));
        assert_eq!(s.rotations[11], Vec3::ZERO);
    }
}

/// `minDist` 20 with the actor at the origin and the root 1 across: every joint is pushed out to
/// exactly 20 × `sDistMultipliers[i]` across from the actor (1.0 for joints 1 to 4, then 1.1 to
/// 1.7).
#[test]
fn the_joints_keep_min_dist_by_the_multipliers() {
    let Some(a) = assets() else { return };
    let mut w = river(&a);
    let h = spawn_cape(&mut w, PARAMS_BOSS);
    let c = cape(&mut w, h);
    let root = Vec3::new(1.0, 100.0, 0.0);
    c.strands[0].root = root;
    c.strands[0].joints = [root; NUM_JOINTS];
    c.gravity = -3.0;
    c.min_y = -10000.0;
    c.min_dist = 20.0;
    c.update_strand(0, 1);
    for i in 1..NUM_JOINTS {
        let p = c.strands[0].joints[i];
        let d = (p.x * p.x + p.z * p.z).sqrt();
        assert!((d - 20.0 * DIST_MULTIPLIERS[i]).abs() < 1e-3, "joint {i}: {d}");
    }
    // minY: a floor at 95 under the root: joint 1 stops on it, none goes below.
    let c = cape(&mut w, h);
    c.min_dist = 0.0;
    c.min_y = 95.0;
    c.strands[0].joints = [root; NUM_JOINTS];
    c.strands[0].velocities = [Vec3::ZERO; NUM_JOINTS];
    c.update_strand(0, 1);
    assert_eq!(c.strands[0].joints[1].y, 95.0);
    assert!(c.strands[0].joints[1..].iter().all(|p| p.y >= 95.0));
}

/// The timers pick the collar's ends (`EnGanonMant_Draw`): the right forearm and left shoulder
/// (gravity -13), the right shoulder and left forearm (-13), both shoulders (-3), else both
/// forearms (gravity left as it was). The right arm's timer wins over the left's.
#[test]
fn the_attach_timers_pick_the_endpoints() {
    let Some(a) = assets() else { return };
    let mut w = river(&a);
    let h = spawn_cape(&mut w, PARAMS_BOSS);
    let (rf, lf, rs, ls) = (Vec3::new(-30.0, 100.0, 5.0), Vec3::new(30.0, 110.0, -5.0), Vec3::new(-15.0, 140.0, 0.0), Vec3::new(15.0, 140.0, 0.0));
    let cases: [([f32; 3], Vec3, Vec3, f32); 5] =
        [([2.0, 0.0, 0.0], rf, ls, -13.0), ([0.0, 2.0, 0.0], rs, lf, -13.0), ([0.0, 0.0, 2.0], rs, ls, -3.0), ([0.0, 0.0, 0.0], rf, lf, 7.0), ([2.0, 2.0, 2.0], rf, ls, -13.0)];
    for ([right_arm, left_arm, shoulders], want_right, want_left, gravity) in cases {
        let c = cape(&mut w, h);
        (c.right_forearm_pos, c.left_forearm_pos, c.right_shoulder_pos, c.left_shoulder_pos) = (rf, lf, rs, ls);
        (c.attach_right_arm_timer, c.attach_left_arm_timer, c.attach_shoulders_timer) = (right_arm, left_arm, shoulders);
        c.gravity = 7.0;
        c.update_has_run = true;
        c.draw_strands();
        assert!(near(c.strands[0].root, want_right, 0.02), "{:?} not {want_right:?}", c.strands[0].root);
        assert!(near(c.strands[11].root, want_left, 0.02), "{:?} not {want_left:?}", c.strands[11].root);
        assert_eq!(c.gravity, gravity);
    }
}

/// `EnGanonMant_Update` with `En_Viewer`'s Ganondorf as the parent: the timers down 1 (to 0, no
/// further), `frameTimer` on, the parent's shape yaw; then the draw writes `gMant1Vtx` on odd
/// frames and `gMant2Vtx` on even ones, and nothing without an update since the last draw.
#[test]
fn the_double_buffer_alternates_with_frame_timer() {
    let Some(a) = assets() else { return };
    let mut w = river(&a);
    let (_, h) = viewer(&mut w, 5, 0x1234);
    let c = cape(&mut w, h);
    (c.attach_right_arm_timer, c.attach_left_arm_timer, c.attach_shoulders_timer) = (2.0, 1.0, 0.0);
    c.right_forearm_pos = Vec3::new(-20.0, 100.0, 0.0);
    c.left_forearm_pos = Vec3::new(20.0, 100.0, 0.0);
    assert_eq!(c.vertices, [None, None]);
    update(&mut w, h);
    let c = cape(&mut w, h);
    assert_eq!((c.attach_right_arm_timer, c.attach_left_arm_timer, c.attach_shoulders_timer), (1.0, 0.0, 0.0));
    assert_eq!((c.frame_timer, c.update_has_run, c.actor.shape_rot.y), (1, true, 0x1234));
    assert_eq!(c.buffer(), 0);
    c.draw_strands();
    assert!(c.vertices[0].is_some() && c.vertices[1].is_none() && !c.update_has_run);
    // No update since: the draw writes nothing.
    let before = c.vertices;
    c.strands[3].joints[4] = Vec3::splat(1234.0);
    c.draw_strands();
    assert_eq!(c.vertices, before);
    update(&mut w, h);
    let c = cape(&mut w, h);
    assert_eq!((c.frame_timer, c.buffer(), c.attach_right_arm_timer), (2, 1, 0.0));
    c.draw_strands();
    assert!(c.vertices[1].is_some());
}

/// `EnGanonMant_UpdateVertices`: strand `i`'s joint `j` is vertex `12 i + 11 - j`, its `ob` the
/// joint truncated, its normal (0, 30, 0) turned by the joint's yaw then pitch, truncated; joint
/// 11's rotation is never written, so its normal is straight up.
#[test]
fn the_vertices_follow_the_strands_through_the_map() {
    let Some(a) = assets() else { return };
    let mut w = river(&a);
    let h = spawn_cape(&mut w, PARAMS_BOSS);
    let c = cape(&mut w, h);
    c.right_forearm_pos = Vec3::new(-25.0, 120.0, 10.0);
    c.left_forearm_pos = Vec3::new(25.0, 118.0, -10.0);
    (c.gravity, c.min_y, c.side_sway_magnitude, c.back_sway_magnitude, c.back_push) = (-3.0, -10000.0, -4.5, 0.25, 2.0);
    for _ in 0..5 {
        c.update_has_run = true;
        c.frame_timer = c.frame_timer.wrapping_add(1);
        c.draw_strands();
    }
    let v = c.vertices[c.buffer()].expect("written");
    for (i, s) in c.strands.iter().enumerate() {
        for j in 0..NUM_JOINTS {
            let x = v[12 * i + 11 - j];
            let p = s.joints[j];
            assert_eq!(x.ob, [p.x as i32 as i16, p.y as i32 as i16, p.z as i32 as i16], "strand {i} joint {j}");
            let mut m = MtxF::IDENTITY;
            m.rotate_y(s.rotations[j].y);
            m.rotate_x(s.rotations[j].x);
            let n = m.mult_vec3f(Vec3::new(0.0, 30.0, 0.0));
            assert_eq!(x.n, [n.x as i32 as i8, n.y as i32 as i8, n.z as i32 as i8], "strand {i} joint {j}");
        }
        assert_eq!(v[12 * i].n, [0, 30, 0]);
    }
}

/// `EnGanonMant_Tear` (from the update while `tearTimer` runs): the first texel of the line (the
/// generator's first two numbers, 32 and 64 across) and the others are zeroed in this cape's copy
/// of `gMantTex`, the pack's untouched; up to four joints marked torn.
#[test]
fn a_tear_zeroes_texels_of_the_capes_copy() {
    let Some(a) = assets() else { return };
    let mut w = river(&a);
    let (_, h) = viewer(&mut w, 5, 0);
    let file = a.pack.texture(OVERLAY, "gMantTex").expect("gMantTex").texels;
    assert_eq!(file.len(), TEX_WIDTH * TEX_HEIGHT * 2);
    assert_eq!(cape(&mut w, h).texture.as_deref(), Some(file.as_slice()));
    cape(&mut w, h).tear_timer = 2;
    w.rand.state = 0x1234_5678;
    let mut r = w.rand;
    let (tx, ty) = (r.zero_float(32.0) as usize, r.zero_float(64.0) as usize);
    update(&mut w, h);
    let c = cape(&mut w, h);
    assert_eq!(c.tear_timer, 1);
    let t = c.texture.as_ref().unwrap();
    let k = (tx + ty * TEX_WIDTH) * 2;
    assert_eq!(&t[k..k + 2], &[0, 0]);
    let zeroed = |t: &[u8]| t.chunks(2).filter(|p| p == &[0, 0]).count();
    assert!(zeroed(t) > zeroed(&file));
    let torn = c.strands.iter().flat_map(|s| s.torn).filter(|&b| b).count();
    assert!((1..=4).contains(&torn), "{torn}");
    assert!(c.torn_image.is_some());
    assert_eq!(a.pack.texture(OVERLAY, "gMantTex").unwrap().texels, file);
}

/// `EnViewer_UpdateGanondorfCape` for type 5 facing 0x4000 with the neck at (100, 50, -20): the
/// forearms (-13, 3, -10) and (13, 3, -10) turned a quarter (x' = z, z' = -x): (-10, 3, 13) and
/// (-10, 3, -13) from the neck; back sway 0.25, side sway -4.5, gravity -1, no floor
/// (-10000), no distance, no push; the phase 0x2000 on. Type 3 writes nothing.
#[test]
fn en_viewer_type_5_writes_the_forearms_from_the_neck() {
    let Some(a) = assets() else { return };
    let mut w = river(&a);
    let (v, h) = viewer(&mut w, 5, 0x4000);
    let neck = Vec3::new(100.0, 50.0, -20.0);
    w.overlay_static::<Statics>(ACTOR_EN_VIEWER).ganondorf_neck_world_pos = neck;
    assert_eq!(w.overlay_static::<Statics>(ACTOR_EN_VIEWER).ganondorf_cape, Some(h));
    let taken = w.actors.take(v).unwrap();
    taken.as_any().downcast_ref::<EnViewer>().unwrap().update_ganondorf_cape(&mut w);
    w.actors.put_back(v, taken);
    let c = cape(&mut w, h);
    assert!(near(c.right_forearm_pos, Vec3::new(90.0, 53.0, -7.0), 1e-3), "{:?}", c.right_forearm_pos);
    assert!(near(c.left_forearm_pos, Vec3::new(90.0, 53.0, -33.0), 1e-3), "{:?}", c.left_forearm_pos);
    assert_eq!((c.back_push, c.back_sway_magnitude, c.side_sway_magnitude, c.gravity, c.min_y, c.min_dist), (0.0, 0.25, -4.5, -1.0, -10000.0, 0.0));
    assert_eq!(w.overlay_static::<Statics>(ACTOR_EN_VIEWER).y_oscillation_phase, 0x2000);

    // Type 3: its own cape (the static now) is never written.
    let (v3, h3) = viewer(&mut w, 3, 0x4000);
    let taken = w.actors.take(v3).unwrap();
    taken.as_any().downcast_ref::<EnViewer>().unwrap().update_ganondorf_cape(&mut w);
    w.actors.put_back(v3, taken);
    let c = cape(&mut w, h3);
    assert_eq!((c.right_forearm_pos, c.left_forearm_pos, c.gravity), (Vec3::ZERO, Vec3::ZERO, 0.0));
}

/// Type 3's cape, never written: every root at the origin, gravity 0, the floor at 0; the
/// params-0x23 push lays every strand back along y 0, all twelve the same: no width.
#[test]
fn an_unwritten_cape_has_no_width() {
    let Some(a) = assets() else { return };
    let mut w = river(&a);
    let h = spawn_cape(&mut w, PARAMS_VIEWER);
    let c = cape(&mut w, h);
    for _ in 0..10 {
        c.update_has_run = true;
        c.draw_strands();
    }
    for s in &c.strands {
        assert_eq!(s.root, Vec3::ZERO);
        for j in 0..NUM_JOINTS {
            assert!(near(s.joints[j], c.strands[0].joints[j], 1e-4));
            assert!(s.joints[j].y.abs() < 1e-3);
        }
    }
    assert!(c.strands[0].joints[11].z < -60.0, "{:?}", c.strands[0].joints[11]);
}

/// The bakes: `gMantMaterialDL` and `gMantDL` from the overlay with segment 0x0C on each copy,
/// every vertex on its own bone at its place in the file (a grid 1000 apart at y 0): 11 × 11
/// quads, 242 triangles; the texture loaded from `gMantTex`'s link-time address.
#[test]
fn the_bakes_build_from_the_pack() {
    let Some(a) = assets() else { return };
    for name in BAKES {
        let d = a.pack.bake(name).expect("the bake");
        let tris: usize = d.batches.iter().map(|b| b.vertices.len() / 3).sum();
        assert_eq!(tris, 242, "{name}");
        let mut bones = std::collections::BTreeSet::new();
        for v in d.batches.iter().flat_map(|b| &b.vertices) {
            bones.insert(v.bone);
            assert_eq!(v.pos.y, 0.0);
            assert_eq!((v.pos.x % 1000.0, v.pos.z % 1000.0), (0.0, 0.0));
            assert!(v.normal.y > 0.99);
        }
        assert_eq!(bones.len(), NUM_VERTICES, "{name}");
        assert!(d.textures.iter().any(|t| t.source_addr == Some(MANT_TEX_ADDR)), "{:?}", d.textures.iter().map(|t| t.source_addr).collect::<Vec<_>>());
    }
}

/// `EnGanonMant_DrawCloak`: the copy's bake on OPA in world coordinates, each bone taking its
/// vertex from the file's place to the written one and its normal (the file's, straight up) to
/// the written one; before the first write the copy draws as the file has it. Once torn, the
/// copy's texture goes in place of `gMantTex`.
#[test]
fn the_draw_moves_each_vertex_to_its_written_place() {
    let Some(a) = assets() else { return };
    let mut w = river(&a);
    let h = spawn_cape(&mut w, PARAMS_BOSS);
    let draw = |w: &mut PlayState| {
        let taken = w.actors.take(h).unwrap();
        let rs = taken.render_state();
        let mut out = oot_game::play::DrawOut::default();
        taken.draw(&rs, w, &oot_game::play::ViewInfo::new(Vec3::ZERO, glam::Mat4::IDENTITY), &mut out);
        w.actors.put_back(h, taken);
        out.opa.pop().expect("drawn on OPA")
    };
    let cmd = draw(&mut w);
    assert_eq!(cmd.mesh.name, oot_game::pack::keys::bake(BAKES[1]));
    assert!(cmd.bones.iter().all(|&m| m == glam::Mat4::IDENTITY) && cmd.params.texture_images.is_empty());

    let c = cape(&mut w, h);
    c.right_forearm_pos = Vec3::new(-25.0, 120.0, 10.0);
    c.left_forearm_pos = Vec3::new(25.0, 118.0, -10.0);
    (c.gravity, c.min_y, c.side_sway_magnitude) = (-3.0, -10000.0, -4.5);
    c.frame_timer = 1;
    c.update_has_run = true;
    c.draw_strands();
    c.tear(&mut oot_game::play::Rand::default());
    let v = c.vertices[0].expect("gMant1Vtx written");
    let cmd = draw(&mut w);
    assert_eq!(cmd.mesh.name, oot_game::pack::keys::bake(BAKES[0]));
    assert_eq!(cmd.transform, glam::Mat4::IDENTITY);
    let d = a.pack.bake(BAKES[0]).unwrap();
    for x in d.batches.iter().flat_map(|b| &b.vertices) {
        let k = x.bone as usize;
        let m = cmd.bones[k];
        let want = Vec3::new(v[k].ob[0] as f32, v[k].ob[1] as f32, v[k].ob[2] as f32);
        assert!(near(m.transform_point3(x.pos), want, 1e-2), "vertex {k}");
        let n = Vec3::new(v[k].n[0] as f32, v[k].n[1] as f32, v[k].n[2] as f32).normalize();
        assert!(near(m.transform_vector3(x.normal), n, 1e-3), "vertex {k}");
    }
    assert_eq!(cmd.params.texture_images.len(), 1);
    assert_eq!(cmd.params.texture_images[0].source, MANT_TEX_ADDR);
}
