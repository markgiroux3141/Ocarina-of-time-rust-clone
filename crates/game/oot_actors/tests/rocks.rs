//! The brown boulders (`Obj_Bombiwa`, `z_obj_bombiwa.c`) against the C, in the Master Quest Deku
//! Tree (`ootx scene-info --scene ydan`): room 2's three on the ledge at y 656, params 0x0003 at
//! (-1129, 656, 1469), 0x0004 at (-1183, 656, 1522) and 0x0008 at (-1237, 656, 1558), all turned
//! (0, -24576, 0).
//!
//! Link has no bombs and no hammer yet: the tests inject a bomb's explosion (an actor in
//! `ACTORCAT_EXPLOSIVE` with params 1, which `func_80033684` looks for) and a hammer's hit on the
//! cylinder (its `AC_HIT` and `acHitElem`, as the collision check leaves them). Expected values
//! are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::obj_bombiwa::{ACTOR_OBJ_BOMBIWA, DL, OBJECT_BOMBIWA, ObjBombiwa};
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTORCAT_EXPLOSIVE, ACTORCAT_PROP, ActorHandle, ActorImpl};
use oot_game::audio::sfx::{NA_SE_EV_WALL_BROKEN, NA_SE_SY_CORRECT_CHIME};
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

/// Inside the Deku Tree (`deku-tree-inside`), the sound log on, Navi's room 0 hints as if heard
/// (flag 0x1F), 60 frames in; then in room 2 (a debug start's room change).
fn room_2(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-inside").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    w.flags.set_switch(0x1F);
    idle(&mut w, 60);
    assert!(w.room_request(2));
    idle(&mut w, 1);
    w.room_change_done();
    // The room's objects load (Object_UpdateEntries: two frames), then its actors' inits.
    idle(&mut w, 3);
    w
}

fn sfx_on(w: &PlayState, frame: u32, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == frame && s == id)
}

/// The live boulders, oldest first.
fn rocks(w: &PlayState) -> Vec<ActorHandle> {
    let mut v: Vec<ActorHandle> = w.actors.category(ACTORCAT_PROP).iter().copied().filter(|&h| w.actors.downcast::<ObjBombiwa>(h).is_some_and(|c| !c.actor.killed)).collect();
    v.reverse();
    v
}

fn rock_at(w: &PlayState, home: Vec3) -> ActorHandle {
    rocks(w).into_iter().find(|&h| w.actors.actor(h).unwrap().home_pos == home).expect("the boulder")
}

fn rock(w: &PlayState, h: ActorHandle) -> &ObjBombiwa {
    w.actors.downcast::<ObjBombiwa>(h).expect("Obj_Bombiwa")
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

/// A hit on `h`'s cylinder by Player's weapon element with `dmg_flags`, as the collision check
/// leaves it.
fn hit(w: &mut PlayState, h: ActorHandle, dmg_flags: u32) {
    let player = w.player.unwrap();
    let r = w.actors.downcast_mut::<ObjBombiwa>(h).unwrap();
    r.collider.base.ac_flags |= cc::AC_HIT;
    r.collider.info.ac_hit_elem = Some(cc::HitElem {
        elem: cc::ElemRef { col: cc::ColliderRef { actor: player, id: 0 }, elem: 0 },
        at_dmg_info: cc::ColliderElementDamageInfoAT { dmg_flags, hit_special_effect: cc::HIT_SPECIAL_EFFECT_NONE, damage: 2 },
        ac_dmg_info: Default::default(),
        elem_material: 0,
    });
}

/// The effects of type `ty` spawned this frame (`EffectSs_UpdateAll` has taken one off `life`).
fn new_effects(w: &PlayState, ty: u8, life: i16) -> Vec<&oot_game::effect::EffectSs> {
    w.effect_ss.table.iter().filter(|e| e.ty == ty && e.life == life - 1).collect()
}

const ROCKS: [(Vec3, i16); 3] = [(Vec3::new(-1129.0, 656.0, 1469.0), 3), (Vec3::new(-1183.0, 656.0, 1522.0), 4), (Vec3::new(-1237.0, 656.0, 1558.0), 8)];

#[test]
fn room_2s_boulders_are_placed_whole() {
    let Some(a) = assets() else { return };
    let w = room_2(&a);
    assert_eq!(rocks(&w).len(), 3);
    for (home, params) in ROCKS {
        let h = rock_at(&w, home);
        let r = rock(&w, h);
        // ObjBombiwa_Init: scale 0.1 (sInitChain); flag (params & 0x3F) clear, so not killed;
        // shape.rot.y -24576 isn't 0 (no Rand); shape.yOffset -200; world.pos.y home + 20.
        assert_eq!((r.actor.params, r.switch_flag()), (params, params as i32));
        assert_eq!(r.actor.scale, Vec3::splat(0.1));
        assert_eq!((r.actor.shape_rot.y, r.actor.world_rot.y), (-24576, -24576));
        assert_eq!(r.actor.shape_y_offset, -200.0);
        assert_eq!(r.actor.world_pos, home + Vec3::new(0.0, 20.0, 0.0));
        // ACTORCAT_PROP, FLAGS 0 (Actor_UpdateAll's own flags aside).
        assert_eq!(r.actor.category, ACTORCAT_PROP);
        assert_eq!(r.actor.flags & (oot_game::actor::ACTOR_FLAG_ATTENTION_ENABLED | oot_game::actor::ACTOR_FLAG_UPDATE_CULLING_DISABLED), 0);
        // sCylinderInit: 55 by 70, hard, AC_ON | AC_HARD | AC_TYPE_PLAYER, OC1_ON | OC1_TYPE_ALL,
        // taking 0x4FC1FFFE; Collider_UpdateCylinder ran before the 20 up and never again: the
        // cylinder stands at its placement, 20 below the actor.
        let c = &r.collider;
        assert_eq!((c.dim.radius, c.dim.height, c.dim.y_shift), (55, 70, 0));
        assert_eq!(c.dim.pos, [home.x as i16, home.y as i16, home.z as i16]);
        assert_eq!(c.base.col_type, cc::COL_MATERIAL_HARD);
        assert_eq!(c.base.ac_flags & (cc::AC_ON | cc::AC_HARD | cc::AC_TYPE_PLAYER), cc::AC_ON | cc::AC_HARD | cc::AC_TYPE_PLAYER);
        assert_eq!(c.base.oc_flags1 & (cc::OC1_ON | cc::OC1_TYPE_ALL), cc::OC1_ON | cc::OC1_TYPE_ALL);
        assert_eq!(c.info.ac_dmg_info.dmg_flags, 0x4FC1_FFFE);
        // sColChkInfoInit: health 0, 12 by 60, MASS_IMMOVABLE.
        let ci = &r.actor.col_chk_info;
        assert_eq!((ci.health, ci.cyl_radius, ci.cyl_height, ci.mass), (0, 12, 60, cc::MASS_IMMOVABLE));
    }
}

#[test]
fn an_unturned_boulder_takes_its_yaw_from_rand_and_a_broken_ones_flag_keeps_it_away() {
    let Some(a) = assets() else { return };
    let mut w = room_2(&a);
    // Placed with shape.rot.y 0: (s16)Rand_ZeroFloat(65536.0f), the one Rand call of its init.
    let at = Vec3::new(-1300.0, 480.0, 1450.0);
    let mut r: Rand = w.rand;
    let yaw = r.zero_float(65536.0) as i32 as i16;
    let h = w.actor_spawn(ACTOR_OBJ_BOMBIWA, at, [0, 0, 0], 0x0010).expect("Obj_Bombiwa");
    assert_eq!(w.rand, r, "one Rand call");
    let b = rock(&w, h);
    assert_eq!((b.actor.shape_rot.y, b.actor.world_rot.y), (yaw, yaw));
    assert_eq!(b.actor.world_pos.y, 500.0);
    // Its flag (0x10) set: the next one is killed in its init, before any Rand, and the rest of
    // the init (no col chk info, no yOffset, no 20 up).
    w.flags.set_switch(0x10);
    let r = w.rand;
    let h2 = w.actor_spawn(ACTOR_OBJ_BOMBIWA, at, [0, 0, 0], 0x0010).expect("Obj_Bombiwa");
    assert_eq!(w.rand, r, "no Rand call");
    let b = rock(&w, h2);
    assert!(b.actor.killed);
    assert_eq!((b.actor.shape_rot.y, b.actor.shape_y_offset, b.actor.world_pos.y), (0, 0.0, 480.0));
}

/// `ObjBombiwa_Break`'s fragments and dust, the C's way, from `r`: for each of `sEffectScales`,
/// the position `(Rand - 0.5) × 10` across, `Rand × 5 + 8` up and `(Rand - 0.5) × 10` deep round
/// its home, the velocity `(Rand - 0.5) × 15`, `Rand × 16 + 5`, `(Rand - 0.5) × 15` (six
/// statements, in order); `EffectSsKakera_Init`'s pitch and yaw; then `func_80033480(pos, 60, 8,
/// 100, 160, 1)`: nine puffs, each three `Rand` places, its scale `(s16)((100 × Rand) × 0.2) +
/// 100`, and the dust's own `Rand` (draw flags 5 have bit 2).
struct Fragment {
    pos: Vec3,
    velocity: Vec3,
    pitch: i16,
    yaw: i16,
}

fn expected_break(r: &mut Rand, home: Vec3, at: Vec3) -> (Vec<Fragment>, Vec<(Vec3, i16)>) {
    let mut frags = Vec::new();
    for _ in 0..8 {
        let x = ((r.zero_one() - 0.5) * 10.0) + home.x;
        let y = ((r.zero_one() * 5.0) + home.y) + 8.0;
        let z = ((r.zero_one() - 0.5) * 10.0) + home.z;
        let vx = (r.zero_one() - 0.5) * 15.0;
        let vy = (r.zero_one() * 16.0) + 5.0;
        let vz = (r.zero_one() - 0.5) * 15.0;
        let pitch = (r.zero_one() * 32767.0) as i16;
        let yaw = (r.zero_one() * 32767.0) as i16;
        frags.push(Fragment { pos: Vec3::new(x, y, z), velocity: Vec3::new(vx, vy, vz), pitch, yaw });
    }
    let mut dust = Vec::new();
    for _ in 0..9 {
        let x = at.x + (r.zero_one() - 0.5) * 60.0;
        let y = at.y + (r.zero_one() - 0.5) * 60.0;
        let z = at.z + (r.zero_one() - 0.5) * 60.0;
        let scale = (((100.0 * r.zero_one()) * 0.2) as i16).wrapping_add(100);
        r.zero_one();
        dust.push((Vec3::new(x, y, z), scale));
    }
    (frags, dust)
}

#[test]
fn the_break_spawns_the_cs_fragments_and_dust() {
    let Some(a) = assets() else { return };
    let mut w = room_2(&a);
    let (home, _) = ROCKS[0];
    let h = rock_at(&w, home);
    // ObjBombiwa_Break on its own, on an empty effect table (the spawns take slots 0, 1, ...).
    w.effect_ss = Default::default();
    let mut r = w.rand;
    let (frags, dust) = expected_break(&mut r, home, home + Vec3::new(0.0, 20.0, 0.0));
    let mut act = w.actors.take(h).unwrap();
    act.as_any_mut().downcast_mut::<ObjBombiwa>().unwrap().break_(&mut w);
    w.actors.put_back(h, act);
    assert_eq!(w.rand, r, "the Rand calls, in the C's order");
    let slot = w.object_ctx.get_index(OBJECT_BOMBIWA).expect("object_bombiwa in a bank") as i16;
    let scales = [17, 14, 10, 8, 7, 5, 3, 2];
    for (i, f) in frags.iter().enumerate() {
        let e = &w.effect_ss.table[i];
        // EffectSsKakera_Spawn(pos, velocity, pos, -400, arg5, 10, 2, 0, scale, 1, 0, 80,
        // KAKERA_COLOR_NONE, OBJECT_BOMBIWA, object_bombiwa_DL_0009E0): arg5 37 for a scale of 11
        // and over (17, 14), else 33.
        assert_eq!((e.ty, e.priority, e.life), (EFFECT_SS_KAKERA, 101, 80), "fragment {i}");
        assert_eq!((e.pos, e.velocity, e.vec, e.accel), (f.pos, f.velocity, f.pos, Vec3::ZERO), "fragment {i}");
        assert_eq!(e.gfx, Some(("object_bombiwa", DL)));
        let rg = &e.regs;
        assert_eq!((rg[k::R_REG0], rg[k::R_GRAVITY], rg[k::R_PITCH], rg[k::R_YAW]), (0, -400, f.pitch, f.yaw), "fragment {i}");
        let arg5 = if scales[i] >= 11 { 37 } else { 33 };
        assert_eq!((rg[k::R_REG4], rg[k::R_REG5], rg[k::R_REG6], rg[k::R_SCALE]), (arg5, 10, 2, scales[i]), "fragment {i}");
        assert_eq!((rg[k::R_REG8], rg[k::R_REG9], rg[k::R_OBJ_ID], rg[k::R_OBJECT_SLOT], rg[k::R_COLOR_IDX]), (1, 0, OBJECT_BOMBIWA, slot, KAKERA_COLOR_NONE), "fragment {i}");
    }
    for (j, (pos, scale)) in dust.iter().enumerate() {
        let e = &w.effect_ss.table[8 + j];
        // func_800286CC: draw flags 5, 10 frames, rising at 0.3.
        assert_eq!((e.ty, e.life, e.pos, e.velocity, e.accel), (EFFECT_SS_DUST, 10, *pos, Vec3::ZERO, Vec3::new(0.0, 0.3, 0.0)), "puff {j}");
        assert_eq!((e.regs[dust::R_SCALE], e.regs[dust::R_DRAW_FLAGS]), (*scale, 5), "puff {j}");
    }
    assert_eq!(w.effect_ss.table[17].life, -1, "nine puffs");
}

#[test]
fn an_explosion_in_reach_breaks_a_boulder() {
    let Some(a) = assets() else { return };
    let mut w = room_2(&a);
    let (home, params) = ROCKS[1];
    let h = rock_at(&w, home);
    let pos = home + Vec3::new(0.0, 20.0, 0.0);
    // func_80033684: an explosive with params 1 within rot.z × 10 + 80 of the boulder's position
    // (20 above its home). One 81 off (out from the row of boulders, over 110 from the other two)
    // with rot.z 0 is out of reach; one with params 0 (a bomb not exploding) never counts.
    let out = Vec3::new(1.0, 0.0, 1.0).normalize();
    let far = spawn_blast(&mut w, pos + out * 81.0, 1, 0);
    spawn_blast(&mut w, pos + out * 10.0, 0, 0);
    idle(&mut w, 3);
    assert!(!rock(&w, h).actor.killed);
    assert!(!w.flags.get_switch(params as i32));
    // rot.z 1: 90.
    w.actors.actor_mut(far).unwrap().shape_rot.z = 1;
    idle(&mut w, 1);
    let f = w.audio.frames;
    // ObjBombiwa_Break, Flags_SetSwitch(4), NA_SE_EV_WALL_BROKEN at its position (no chime:
    // params 4 has no bit 15), Actor_Kill.
    assert!(w.actors.actor(h).is_none_or(|a| a.killed));
    assert!(w.flags.get_switch(params as i32));
    assert!(sfx_on(&w, f, NA_SE_EV_WALL_BROKEN));
    assert!(!sfx_on(&w, f, NA_SE_SY_CORRECT_CHIME));
    // 8 fragments (80 frames) and 9 puffs of dust (10 frames) this frame.
    let frags = new_effects(&w, EFFECT_SS_KAKERA, 80);
    assert_eq!(frags.len(), 8);
    assert!(frags.iter().all(|e| e.gfx == Some(("object_bombiwa", DL)) && e.regs[k::R_GRAVITY] == -400));
    assert_eq!(new_effects(&w, EFFECT_SS_DUST, 10).len(), 9);
    // The other two stand.
    assert_eq!(rocks(&w).len(), 2);
    // Back in room 2 later: its flag set, it's killed in its init (Flags_GetSwitch).
    assert!(w.room_request(0));
    idle(&mut w, 1);
    w.room_change_done();
    idle(&mut w, 3);
    assert!(w.room_request(2));
    idle(&mut w, 1);
    w.room_change_done();
    idle(&mut w, 3);
    assert_eq!(rocks(&w).len(), 2);
    assert!(rocks(&w).iter().all(|&r| w.actors.actor(r).unwrap().home_pos != home));
}

#[test]
fn a_hammer_hit_breaks_a_boulder_and_bit_15_chimes() {
    let Some(a) = assets() else { return };
    let mut w = room_2(&a);
    // Link within 800 of a boulder of params 0x8005 (flag 5, the chime) on room 2's 480 floor:
    // its cylinder is set for hits and pushes.
    let at = Vec3::new(-1300.0, 480.0, 1450.0);
    w.place_player(at + Vec3::new(0.0, 0.0, -100.0), 0);
    let h = w.actor_spawn(ACTOR_OBJ_BOMBIWA, at, [0, 0x1000, 0], 0x8005u16 as i16).expect("Obj_Bombiwa");
    idle(&mut w, 2);
    // A sword's slash (DMG_SLASH_KOKIRI, not DMG_HAMMER): its AC_HIT is cleared, nothing more.
    hit(&mut w, h, cc::DMG_SLASH_KOKIRI);
    idle(&mut w, 1);
    assert!(!rock(&w, h).actor.killed);
    assert_eq!(rock(&w, h).collider.base.ac_flags & cc::AC_HIT, 0);
    assert!(!w.flags.get_switch(5));
    // The hammer's (DMG_HAMMER_SWING, in DMG_HAMMER).
    hit(&mut w, h, cc::DMG_HAMMER_SWING);
    idle(&mut w, 1);
    let f = w.audio.frames;
    assert!(w.actors.actor(h).is_none_or(|a| a.killed));
    assert!(w.flags.get_switch(5));
    assert!(sfx_on(&w, f, NA_SE_EV_WALL_BROKEN));
    // PARAMS_GET_U(params, 15, 1): Sfx_PlaySfxCentered(NA_SE_SY_CORRECT_CHIME).
    assert!(sfx_on(&w, f, NA_SE_SY_CORRECT_CHIME));
    assert_eq!(new_effects(&w, EFFECT_SS_KAKERA, 80).len(), 8);
}

#[test]
fn the_draw_is_the_boulders_list() {
    let Some(a) = assets() else { return };
    let w = room_2(&a);
    let h = rock_at(&w, ROCKS[2].0);
    let r = rock(&w, h);
    let rs = r.render_state();
    let mut out = oot_game::play::DrawOut::default();
    r.draw(&rs, &w, &oot_game::play::ViewInfo::new(Vec3::ZERO, glam::Mat4::IDENTITY), &mut out);
    // Gfx_DrawDListOpa(object_bombiwa_DL_0009E0) at Actor_Draw's matrix: yOffset -200 × 0.1 puts
    // it back at its home.
    assert_eq!(out.opa.len(), 1);
    assert_eq!(out.opa[0].mesh, eng_gfx::MeshKey::named(oot_game::pack::keys::mesh("object_bombiwa", DL)));
    let t = out.opa[0].transform.w_axis;
    assert!((Vec3::new(t.x, t.y, t.z) - ROCKS[2].0).length() < 1e-3);
    assert!(out.xlu.is_empty());
}

/// Room 2's debug start by the boulders and the hidden time blocks: on the 480 floor west of the
/// four blocks (their diamond round (-1166, 547, 1390), hidden, 67 over the floor), under the
/// boulders' ledge (656), facing them (+x).
const ROOM_2_START: (Vec3, i16) = (Vec3::new(-1290.0, 480.0, 1440.0), 0x4000);

#[test]
fn room_2s_debug_start_by_the_boulders_stands() {
    let Some(a) = assets() else { return };
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-inside").unwrap();
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, oot_game::audio::GameAudio::default()).expect("Play_Init");
    idle(&mut w, 2);
    let (pos, yaw) = ROOM_2_START;
    oot_actors::playthrough::deku_tree_room_start(&mut w, 2, pos, yaw);
    for h in w.actors.category(oot_game::actor_ctx::ACTORCAT_ENEMY).to_vec() {
        if let Some(a) = w.actors.actor_mut(h) {
            a.kill();
        }
    }
    idle(&mut w, 40);
    let p = w.player();
    let at = p.actor.world_pos;
    assert_eq!((w.room_ctx.cur.num, w.room_ctx.prev.num), (2, -1), "the room changed");
    assert!(p.grounded(), "not on the ground at {at:?}");
    assert!((at - pos).length() < 1.0, "moved from {pos:?} to {at:?} ({:?})", p.action);
    assert_eq!(rocks(&w).len(), 3);
}
