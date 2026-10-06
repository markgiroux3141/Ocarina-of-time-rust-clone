//! The large crates (`Obj_Kibako2`, `z_obj_kibako2.c`) against the C, in the Master Quest Deku
//! Tree (`ootx scene-info --scene ydan`): room 0's on the middle floor, params 0xFFFF at
//! (279, 360, 333) rot (0, 24576, 0), its Gold Skulltula (`En_Sw` 0x8102) placed inside it at
//! (278, 360, 332); room 10's two, 0xFFFF at (-805, 720, -2) and (-805, 720, -62) rot
//! (0, 16384, 0).
//!
//! Link has no hammer and no bombs yet: the tests inject a hammer's hit on the crate's cylinder
//! (its `AC_HIT`, as the collision check leaves it) and a bomb's explosion (an actor in
//! `ACTORCAT_EXPLOSIVE` with params 1, which `func_80033684` looks for: the bomb's own isn't
//! ported). Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_B, PadState};
use eng_math::{cos_s, sin_s};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_item00::EnItem00;
use oot_actors::en_sw::EnSw;
use oot_actors::obj_kibako2::{self, Action, NA_SE_EV_WOODBOX_BREAK, OBJECT_KIBAKO2, ObjKibako2};
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTORCAT_BG, ACTORCAT_EXPLOSIVE, ActorHandle, ActorImpl};
use oot_game::collision_check as cc;
use oot_game::effect::kakera::{self as k, KAKERA_COLOR_NONE};
use oot_game::effect::{EFFECT_SS_DUST, EFFECT_SS_KAKERA, dust};
use oot_game::play::{PlayState, Rand, scripted_input};
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

fn tick(w: &mut PlayState, p: PadState, prev: &mut PadState) {
    w.tick_with(scripted_input(*prev, p));
    *prev = p;
}

/// Inside the Deku Tree (`deku-tree-inside`), the sound log on, Navi's room 0 hints as if heard
/// (flag 0x1F), 60 frames in; then in `room` (a debug start's room change).
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
    w
}

fn sfx_on(w: &PlayState, frame: u32, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == frame && s == id)
}

/// The live crates, as placed (oldest first).
fn crates(w: &PlayState) -> Vec<ActorHandle> {
    let mut v: Vec<ActorHandle> = w.actors.category(ACTORCAT_BG).iter().copied().filter(|&h| w.actors.downcast::<ObjKibako2>(h).is_some_and(|c| !c.actor.killed)).collect();
    v.reverse();
    v
}

fn crate_at(w: &PlayState, pos: Vec3) -> ActorHandle {
    crates(w).into_iter().find(|&h| w.actors.actor(h).unwrap().home_pos == pos).expect("the crate")
}

fn kibako(w: &PlayState, h: ActorHandle) -> &ObjKibako2 {
    w.actors.downcast::<ObjKibako2>(h).expect("Obj_Kibako2")
}

fn kibako_mut(w: &mut PlayState, h: ActorHandle) -> &mut ObjKibako2 {
    w.actors.downcast_mut::<ObjKibako2>(h).expect("Obj_Kibako2")
}

/// The effects of type `ty` spawned this frame (`EffectSs_UpdateAll` has taken one off `life`).
fn new_effects(w: &PlayState, ty: u8, life: i16) -> Vec<&oot_game::effect::EffectSs> {
    w.effect_ss.table.iter().filter(|e| e.ty == ty && e.life == life - 1).collect()
}

fn items(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&i| w.actors.downcast::<EnItem00>(i).is_some_and(|e| !e.actor.killed)).collect()
}

fn skulltulas(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&i| w.actors.downcast::<EnSw>(i).is_some_and(|e| !e.actor.killed)).collect()
}

/// A bomb's explosion as `func_80033684` sees one: an actor in `ACTORCAT_EXPLOSIVE` with its
/// params and `shape.rot.z` (the blast's radius, `rot.z × 10 + 80`). It does nothing itself.
struct Blast {
    actor: Actor,
}

impl ActorImpl for Blast {
    fn name(&self) -> &'static str {
        "Injected explosion"
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    fn update(&mut self, _play: &mut PlayState) {}
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// `ACTOR_EN_BOM` (`actor_table.h`: 0x0010), the bomb.
const ACTOR_EN_BOM: i16 = 0x0010;

fn spawn_blast(w: &mut PlayState, pos: Vec3, params: i16, rot_z: i16) -> ActorHandle {
    let mut a = Actor::new(pos, 0);
    a.id = ACTOR_EN_BOM;
    a.category = ACTORCAT_EXPLOSIVE;
    a.params = params;
    a.shape_rot.z = rot_z;
    w.spawn(Box::new(Blast { actor: a })).expect("spawn")
}

const ROOM_0_CRATE: Vec3 = Vec3::new(279.0, 360.0, 333.0);
const ROOM_10_CRATES: [Vec3; 2] = [Vec3::new(-805.0, 720.0, -2.0), Vec3::new(-805.0, 720.0, -62.0)];

/// What `ObjKibako2_Init` leaves of a placed crate: scale 0.1, the cylinder 31 by 48 at its
/// position taking only `0x40000040`, `gLargeCrateCol` registered with no transform flags, the
/// collectible `home.rot.x` 0 (a green rupee) and flag `home.rot.z & 0x3F` 0, `rot.x` and `rot.z`
/// zeroed, `ObjKibako2_Idle`, drawn.
fn check_placed(w: &PlayState, h: ActorHandle, pos: Vec3, yaw: i16) {
    let c = kibako(w, h);
    assert_eq!(c.actor.params, -1, "params 0xFFFF");
    assert_eq!((c.actor.world_pos, c.actor.home_pos, c.actor.scale), (pos, pos, Vec3::splat(0.1)));
    assert_eq!((c.actor.shape_rot.x, c.actor.shape_rot.y, c.actor.shape_rot.z), (0, yaw, 0));
    assert_eq!((c.actor.world_rot.x, c.actor.world_rot.z, c.actor.home_rot.x, c.actor.home_rot.z), (0, 0, 0, 0));
    assert_eq!((c.collectible_flag, c.action, c.drawn), (0, Action::Idle, true));
    let cyl = c.collider.dim;
    assert_eq!((cyl.radius, cyl.height, cyl.y_shift), (31, 48, 0));
    assert_eq!(cyl.pos, [pos.x as i16, pos.y as i16, pos.z as i16]);
    assert_eq!(c.collider.info.ac_dmg_info.dmg_flags, cc::DMG_HAMMER);
    assert_eq!(c.collider.info.ac_dmg_info.dmg_flags, 0x4000_0040);
    // FLAGS 0 (Actor_UpdateAll sets ACTOR_FLAG_INSIDE_CULLING_VOLUME for every actor here).
    assert_eq!(c.actor.flags & oot_game::actor::ACTOR_FLAG_UPDATE_CULLING_DISABLED, 0);
    let bg = &w.col.dyna.actors[c.bg as usize];
    assert!(bg.in_use() && !bg.collision_disabled);
    assert_eq!(bg.move_flags, 0);
    // gLargeCrateCol, at scale 0.1, round its position.
    assert!(!bg.floor.is_empty() && !bg.wall.is_empty());
    assert!((bg.sphere_center() - pos).length() < 60.0);
}

#[test]
fn room_0s_and_room_10s_crates_are_placed_whole() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 0);
    let h = crate_at(&w, ROOM_0_CRATE);
    check_placed(&w, h, ROOM_0_CRATE, 24576);
    // Its top is a floor of the scene now: 360 + the box's height.
    let (top, poly) = w.col.entity_raycast_down(ROOM_0_CRATE + Vec3::new(0.0, 200.0, 0.0));
    assert!(poly.is_some_and(|p| p.bg == kibako(&w, h).bg), "the crate's top under {top}");
    assert!(top > 400.0 && top < 430.0, "top {top}");
    // The Gold Skulltula placed inside it, on its own (params 0x8102 has bit 15: the crate spawns
    // none of its own).
    assert_eq!(skulltulas(&w).iter().filter(|&&s| w.actors.actor(s).unwrap().home_pos.distance(Vec3::new(278.0, 360.0, 332.0)) < 2.0).count(), 1);
    // Room 10's two.
    assert!(w.room_request(10));
    idle(&mut w, 1);
    w.room_change_done();
    idle(&mut w, 3);
    for p in ROOM_10_CRATES {
        let h = crate_at(&w, p);
        check_placed(&w, h, p, 16384);
    }
}

#[test]
fn a_sword_slash_does_nothing_to_a_crate() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 10);
    let h = crate_at(&w, ROOM_10_CRATES[0]);
    // Link 45 from the crate (on +x), facing it: the Kokiri Sword's slash reaches its cylinder
    // (radius 31), but its DMG_SLASH_KOKIRI isn't in 0x40000040.
    let p = ROOM_10_CRATES[0] + Vec3::new(45.0, 0.0, 0.0);
    let (floor, _) = w.col.entity_raycast_down(p + Vec3::Y * 50.0);
    w.place_player(Vec3::new(p.x, floor, p.z), -0x4000);
    let mut prev = PadState::default();
    let b = with(PadState::default(), BTN_B);
    let mut slashes = 0;
    for i in 0..60 {
        tick(&mut w, if i % 15 < 2 { b } else { PadState::default() }, &mut prev);
        if format!("{:?}", w.player().action) == "Attack" {
            slashes += 1;
        }
        let c = kibako(&w, h);
        assert_eq!((c.action, c.drawn), (Action::Idle, true), "frame {i}");
        assert_eq!(c.collider.base.ac_flags & cc::AC_HIT, 0);
    }
    assert!(slashes > 0, "Link never swung");
    assert!(!w.col.dyna.actors[kibako(&w, h).bg as usize].collision_disabled);
}

/// `ObjKibako2_Break`'s fragments and dust, the C's way, from `r`: for each of 16 at a turn of
/// `i × 0x4E20`, `Rand_ZeroOne() × 30` out, `Rand × 10 + 2` up, the velocity a fifth of that out
/// and `Rand × 10 + 2` up, the tumble from `Rand` (under 0.05: 0x60, under 0.7: 0x40, else
/// 0x20), the scale `Rand × 30 + 5`; `EffectSsKakera_Init`'s pitch and yaw; then
/// `func_80033480(pos, 90, 6, 100, 160, 1)`: seven puffs, each three `Rand` places, its scale
/// `(s16)((100 × Rand) × 0.2) + 100`, and the dust's own `Rand` (draw flags 5 have bit 2).
struct Fragment {
    pos: Vec3,
    velocity: Vec3,
    tumble: i16,
    scale: i16,
    pitch: i16,
    yaw: i16,
}

fn expected_break(r: &mut Rand, at: Vec3) -> (Vec<Fragment>, Vec<(Vec3, i16)>) {
    let mut frags = Vec::new();
    let mut angle: i16 = 0;
    for _ in 0..16 {
        let (sn, cs) = (sin_s(angle), cos_s(angle));
        let t = r.zero_one() * 30.0;
        let mut pos = Vec3::new(sn * t, (r.zero_one() * 10.0) + 2.0, cs * t);
        let velocity = Vec3::new(pos.x * 0.2, (r.zero_one() * 10.0) + 2.0, pos.z * 0.2);
        pos = Vec3::new(pos.x + at.x, pos.y + at.y, pos.z + at.z);
        let t = r.zero_one();
        let tumble = if t < 0.05 {
            0x60
        } else if t < 0.7 {
            0x40
        } else {
            0x20
        };
        let scale = ((r.zero_one() * 30.0) + 5.0) as i16;
        let pitch = (r.zero_one() * 32767.0) as i16;
        let yaw = (r.zero_one() * 32767.0) as i16;
        frags.push(Fragment { pos, velocity, tumble, scale, pitch, yaw });
        angle = angle.wrapping_add(0x4E20);
    }
    let mut dust = Vec::new();
    for _ in 0..7 {
        let x = at.x + (r.zero_one() - 0.5) * 90.0;
        let y = at.y + (r.zero_one() - 0.5) * 90.0;
        let z = at.z + (r.zero_one() - 0.5) * 90.0;
        let scale = (((100.0 * r.zero_one()) * 0.2) as i16).wrapping_add(100);
        // EffectSsDust_Init: draw flags 5 (bit 2): the colours' random offset.
        r.zero_one();
        dust.push((Vec3::new(x, y, z), scale));
    }
    (frags, dust)
}

#[test]
fn the_break_spawns_the_cs_fragments_and_dust() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 0);
    let h = crate_at(&w, ROOM_0_CRATE);
    // ObjKibako2_Break on its own, on an empty effect table (the spawns take slots 0, 1, ...).
    w.effect_ss = Default::default();
    let mut r = w.rand;
    let (frags, dust) = expected_break(&mut r, ROOM_0_CRATE);
    let mut act = w.actors.take(h).unwrap();
    act.as_any_mut().downcast_mut::<ObjKibako2>().unwrap().break_(&mut w);
    w.actors.put_back(h, act);
    assert_eq!(w.rand, r, "the Rand calls, in the C's order");
    let slot = w.object_ctx.get_index(OBJECT_KIBAKO2).expect("object_kibako2 in a bank") as i16;
    for (i, f) in frags.iter().enumerate() {
        let e = &w.effect_ss.table[i];
        assert_eq!((e.ty, e.priority, e.life), (EFFECT_SS_KAKERA, 101, 70), "fragment {i}");
        assert_eq!((e.pos, e.velocity, e.vec, e.accel), (f.pos, f.velocity, f.pos, Vec3::ZERO), "fragment {i}");
        assert_eq!(e.gfx, Some(("object_kibako2", "gLargeCrateFragmentDL")));
        // EffectSsKakera_Spawn(.., -200, phi_s0, 28, 2, 0, scale, 0, 0, 70, KAKERA_COLOR_NONE,
        // OBJECT_KIBAKO2, ..): rReg0 0, gravity -200, rReg4 the tumble, drag 28 and 2, no bounces,
        // no jitter; its object checked (not a keep).
        let rg = &e.regs;
        assert_eq!((rg[k::R_REG0], rg[k::R_GRAVITY], rg[k::R_PITCH], rg[k::R_YAW]), (0, -200, f.pitch, f.yaw), "fragment {i}");
        assert_eq!((rg[k::R_REG4], rg[k::R_REG5], rg[k::R_REG6], rg[k::R_SCALE]), (f.tumble, 28, 2, f.scale), "fragment {i}");
        assert_eq!((rg[k::R_REG8], rg[k::R_REG9], rg[k::R_OBJ_ID], rg[k::R_OBJECT_SLOT], rg[k::R_COLOR_IDX]), (0, 0, OBJECT_KIBAKO2, slot, KAKERA_COLOR_NONE));
    }
    for (j, (pos, scale)) in dust.iter().enumerate() {
        let e = &w.effect_ss.table[16 + j];
        // func_800286CC: draw flags 5, 10 frames, rising at 0.3, step 160.
        assert_eq!((e.ty, e.life, e.pos, e.velocity, e.accel), (EFFECT_SS_DUST, 10, *pos, Vec3::ZERO, Vec3::new(0.0, 0.3, 0.0)), "puff {j}");
        assert_eq!((e.regs[dust::R_SCALE], e.regs[dust::R_DRAW_FLAGS]), (*scale, 5), "puff {j}");
    }
    assert_eq!(w.effect_ss.table[23].life, -1, "seven puffs");
}

#[test]
fn an_explosion_in_reach_breaks_room_0s_crate() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 0);
    let h = crate_at(&w, ROOM_0_CRATE);
    let bg = kibako(&w, h).bg;
    let before_items = items(&w);
    let before_sw = skulltulas(&w);
    // func_80033684: an explosive with params 1 within rot.z × 10 + 80 of the crate. One 81 off
    // with rot.z 0 is out of reach; one with params 0 (a bomb not exploding) never counts.
    let far = spawn_blast(&mut w, ROOM_0_CRATE + Vec3::new(81.0, 0.0, 0.0), 1, 0);
    let unlit = spawn_blast(&mut w, ROOM_0_CRATE + Vec3::new(10.0, 0.0, 0.0), 0, 0);
    idle(&mut w, 3);
    assert_eq!(kibako(&w, h).action, Action::Idle);
    // rot.z 1: 90.
    w.actors.actor_mut(far).unwrap().shape_rot.z = 1;
    let _ = unlit;
    idle(&mut w, 1);
    let f = w.audio.frames;
    // ObjKibako2_Break, NA_SE_EV_WOODBOX_BREAK, ACTOR_FLAG_UPDATE_CULLING_DISABLED, its collision
    // off (DynaPoly_DisableCollision), not drawn, ObjKibako2_Kill.
    let c = kibako(&w, h);
    assert_eq!((c.action, c.drawn), (Action::Kill, false));
    assert_ne!(c.actor.flags & oot_game::actor::ACTOR_FLAG_UPDATE_CULLING_DISABLED, 0);
    assert!(w.col.dyna.actors[bg as usize].collision_disabled);
    assert!(sfx_on(&w, f, NA_SE_EV_WOODBOX_BREAK));
    // 16 fragments (70 frames) and 7 puffs of dust (func_800286CC: 10 frames) this frame.
    let frags = new_effects(&w, EFFECT_SS_KAKERA, 70);
    assert_eq!(frags.len(), 16);
    assert!(frags.iter().all(|e| e.gfx == Some(("object_kibako2", "gLargeCrateFragmentDL")) && e.regs[k::R_GRAVITY] == -200));
    assert_eq!(new_effects(&w, EFFECT_SS_DUST, 10).len(), 7);
    // Nothing more drops yet; the crate's top is no floor any more.
    assert_eq!(items(&w), before_items);
    let (_, poly) = w.col.entity_raycast_down(ROOM_0_CRATE + Vec3::new(0.0, 200.0, 0.0));
    assert!(poly.is_none_or(|p| p.bg != bg));
    // The next frame, ObjKibako2_Kill: params 0xFFFF has bit 15, so no En_Sw; the collectible
    // home.rot.x 0 with flag 0: Item_DropCollectible(pos, 0), a green rupee popping up; it's gone.
    idle(&mut w, 1);
    assert!(w.actors.downcast::<ObjKibako2>(h).is_none_or(|c| c.actor.killed));
    assert_eq!(skulltulas(&w), before_sw);
    let new: Vec<ActorHandle> = items(&w).into_iter().filter(|i| !before_items.contains(i)).collect();
    assert_eq!(new.len(), 1);
    let it = w.actors.actor(new[0]).unwrap();
    assert_eq!((it.params, it.home_pos), (oot_actors::en_item00::ITEM00_RUPEE_GREEN, ROOM_0_CRATE));
    idle(&mut w, 1);
    assert!(w.actors.actor(h).is_none(), "deleted");
    // The Gold Skulltula, placed inside, is in the open: Link's sword reaches it (tests/skulltulas.rs).
    assert_eq!(skulltulas(&w).len(), before_sw.len());
}

#[test]
fn a_hammer_hit_breaks_a_crate_and_one_with_bit_15_clear_lets_a_gold_skulltula_out() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 0);
    // A crate of params 0x0104 on room 0's ground floor (object_kibako2 is in room 0's list),
    // Link 100 off.
    let at = Vec3::new(100.0, 0.0, -200.0);
    w.place_player(at + Vec3::new(0.0, 0.0, 100.0), i16::MIN);
    let h = w.actor_spawn(obj_kibako2::ACTOR_OBJ_KIBAKO2, at, [0, 0x1000, 0], 0x0104).expect("Obj_Kibako2");
    idle(&mut w, 2);
    assert_eq!(kibako(&w, h).action, Action::Idle);
    let before_sw = skulltulas(&w);
    let before_items = items(&w);
    // The hammer's hit (DMG_HAMMER_SWING, in 0x40000040): the cylinder's AC_HIT.
    kibako_mut(&mut w, h).collider.base.ac_flags |= cc::AC_HIT;
    idle(&mut w, 1);
    assert_eq!((kibako(&w, h).action, kibako(&w, h).drawn), (Action::Kill, false));
    assert!(sfx_on(&w, w.audio.frames, NA_SE_EV_WOODBOX_BREAK));
    assert_eq!(new_effects(&w, EFFECT_SS_KAKERA, 70).len(), 16);
    idle(&mut w, 1);
    // ObjKibako2_Kill: bit 15 clear: Actor_Spawn(ACTOR_EN_SW, pos, rot (0, shape.rot.y, 0),
    // 0x0104 | 0x8000). EnSw_Init: 0x8104 has 0x8000: type ((0x8104 - 0x8000) >> 13 & 7) + 1 = 1:
    // 0x2104, its index one less: 0x2004 (GET_GS_FLAGS(0) & 4).
    let new_sw: Vec<ActorHandle> = skulltulas(&w).into_iter().filter(|s| !before_sw.contains(s)).collect();
    assert_eq!(new_sw.len(), 1);
    let s = w.actors.downcast::<EnSw>(new_sw[0]).unwrap();
    assert_eq!(s.actor.params, 0x2004);
    assert_eq!((s.actor.home_pos.x, s.actor.home_pos.z), (at.x, at.z));
    // And the green rupee.
    let new: Vec<ActorHandle> = items(&w).into_iter().filter(|i| !before_items.contains(i)).collect();
    assert_eq!(new.len(), 1);
    assert_eq!(w.actors.actor(new[0]).unwrap().params, oot_actors::en_item00::ITEM00_RUPEE_GREEN);
}
