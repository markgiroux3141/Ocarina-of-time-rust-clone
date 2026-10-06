//! The torches (`Obj_Syokudai`, `z_obj_syokudai.c`) against the C, in the Deku Tree (MQ):
//! - room 0's three golden torches 0x03E7 (switch flag 0x27, count 15) at (-153, 0, 2), (400,
//!   360, 121) and (-490, 800, -36);
//! - room 4's timed pair 0x1099 (count 2, flag 0x19) at (-281, -880, 881) and (-282, -880, 1041);
//! - room 10's wooden torch 0x2400 (bit 10: always lit) at (-653, 800, -105), and its timed one
//!   0x1053 (count 1, flag 0x13) at (-1067, 720, -5).
//!
//! Expected values are worked out from the C in the comments. Link can't light a torch yet (the
//! Deku Stick is milestone 4b's, Din's Fire and fire arrows later): a fire hit on the flame's
//! collider is injected as the collision check leaves it (`AC_HIT`, `acHitElem`).

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::{Mat4, Vec3};
use oot_actors::PlayExt;
use oot_actors::en_firefly::{self, EnFirefly};
use oot_actors::obj_syokudai::{self, ObjSyokudai};
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::audio::sfx::*;
use oot_game::collision_check as cc;
use oot_game::lights::{LIGHT_POINT_GLOW, LightInfo};
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

/// Inside the Deku Tree (`deku-tree-inside`), the sound log on, then in `room` (room 0 is the
/// entrance's), Link put in it: room 4 between its torches, room 10 by its door from room 0.
fn deku_tree_room(a: &Arc<GameAssets>, room: i8) -> PlayState {
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
        match room {
            4 => place_link(&mut w, Vec3::new(-281.0, -880.0, 961.0), 0x4000),
            10 => place_link(&mut w, Vec3::new(-653.0, 800.0, 0.0), -0x4000),
            _ => {}
        }
        // The room's object list reloads object_syokudai: the torches' inits wait for it.
        for _ in 0..20 {
            if w.actors.all().into_iter().any(|h| w.actors.downcast::<ObjSyokudai>(h).is_some()) {
                break;
            }
            idle(&mut w, 1);
        }
        // The room's enemies gone (room 4's Mad Scrub knocks Link about with its nuts, and a
        // finishing blow's freeze stops a frame's updates): the torches are what's tested. With
        // the last enemy gone the room is cleared (Actor_RemoveFromCategory's temp clear), so a
        // door barred until the clear unbars with its attention cameras meanwhile.
        for h in w.actors.category(oot_game::actor_ctx::ACTORCAT_ENEMY).to_vec() {
            if let Some(a) = w.actors.actor_mut(h) {
                a.kill();
            }
        }
        idle(&mut w, 1);
    }
    w
}

/// Link on the floor below `pos` (found within 200), facing `yaw`.
fn place_link(w: &mut PlayState, pos: Vec3, yaw: i16) {
    let (y, _) = w.col.entity_raycast_down(pos + Vec3::Y * 50.0);
    assert!(y > pos.y - 200.0, "no floor below {pos}");
    w.place_player(Vec3::new(pos.x, y, pos.z), yaw);
}

/// The torches with `params`, by z.
fn torches(w: &PlayState, params: i16) -> Vec<ActorHandle> {
    let mut v: Vec<ActorHandle> = w.actors.all().into_iter().filter(|&h| w.actors.downcast::<ObjSyokudai>(h).is_some_and(|t| t.actor.params == params)).collect();
    v.sort_by(|&a, &b| w.actors.actor(a).unwrap().world_pos.z.total_cmp(&w.actors.actor(b).unwrap().world_pos.z));
    v
}

fn torch(w: &PlayState, h: ActorHandle) -> &ObjSyokudai {
    w.actors.downcast::<ObjSyokudai>(h).expect("Obj_Syokudai")
}

fn torch_mut(w: &mut PlayState, h: ActorHandle) -> &mut ObjSyokudai {
    w.actors.downcast_mut::<ObjSyokudai>(h).expect("Obj_Syokudai")
}

/// The torch's point light as the context holds it.
fn light(w: &PlayState, h: ActorHandle) -> LightInfo {
    w.light_ctx.info(torch(w, h).light_node.expect("lightNode")).expect("the light")
}

/// A fire hit (`DMG_ARROW_FIRE`, in `DMG_FIRE`) on the torch's `flameCollider`, as the collision
/// check leaves it, for its next update.
fn fire_hit(w: &mut PlayState, h: ActorHandle) {
    let player = w.player.unwrap();
    let c = &mut torch_mut(w, h).flame_collider;
    c.base.ac_flags |= cc::AC_HIT;
    c.info.ac_hit_elem = Some(cc::HitElem {
        elem: cc::ElemRef { col: cc::ColliderRef { actor: player, id: 0 }, elem: 0 },
        at_dmg_info: cc::ColliderElementDamageInfoAT { dmg_flags: cc::DMG_ARROW_FIRE, hit_special_effect: cc::HIT_SPECIAL_EFFECT_NONE, damage: 1 },
        ac_dmg_info: Default::default(),
        elem_material: 0,
    });
}

fn sfx_on(w: &PlayState, frame: u32, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == frame && s == id)
}

/// The attention cutscenes (5010, `CAM_SET_CS_ATTENTION` until `Camera_Demo5` takes over) queued,
/// with their targets.
fn attention_targets(w: &PlayState) -> Vec<Option<ActorHandle>> {
    w.sub_cameras.iter().flatten().filter(|c| c.cs_id == 5010).map(|c| c.target).collect()
}

/// The torch's draw: the stand's mesh and, lit, the flame's transform.
fn draw(w: &PlayState, h: ActorHandle) -> (String, Option<Mat4>) {
    let t = torch(w, h);
    let rs = t.render_state();
    let mut out = DrawOut::default();
    t.draw(&rs, w, &ViewInfo::new(Vec3::ZERO, Mat4::IDENTITY), &mut out);
    assert_eq!(out.opa.len(), 1);
    (out.opa[0].mesh.name.clone(), out.xlu.first().map(|c| c.transform))
}

/// The flame's scale (`Matrix_Scale(flameScale)` on the actor's scale 1).
fn flame_scale(m: Mat4) -> f32 {
    m.x_axis.truncate().length()
}

/// A lit torch's light: 200 across, a glowing point 70 above it, (b, b, 0) with b from
/// `(u8)(Rand_ZeroOne() * 127) + 128`.
fn assert_lit_light(w: &PlayState, h: ActorHandle) {
    let t = torch(w, h);
    let l = light(w, h);
    let p = t.actor.world_pos;
    assert_eq!((l.ty, l.x, l.y, l.z, l.radius), (LIGHT_POINT_GLOW, p.x as i16, (p.y + 70.0) as i16, p.z as i16, 200));
    assert!((128..=254).contains(&l.color[0]), "brightness {}", l.color[0]);
    assert_eq!((l.color[1], l.color[2]), (l.color[0], 0));
}

#[test]
fn room_0s_golden_torches_light_with_switch_flag_0x27() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 0);
    let ts = torches(&w, 0x03E7);
    assert_eq!(ts.len(), 3);
    for &h in &ts {
        let t = torch(&w, h);
        // ObjSyokudai_Init: scale 1, the stand metal (sColMaterialsStand[0]), immovable; the
        // flame's scroll (s32)(Rand_ZeroOne() * 20), plus one a frame since.
        assert_eq!((t.actor.scale, t.stand_collider.base.col_type, t.actor.col_chk_info.mass), (Vec3::ONE, cc::COL_MATERIAL_METAL, cc::MASS_IMMOVABLE));
        assert_eq!((t.flame_collider.dim.radius, t.flame_collider.dim.height, t.flame_collider.dim.y_shift), (15, 45, 45));
        // Unlit (0x27 unset): litTimer 0, so Lights_PointSetColorAndRadius(0, 0, 0, -1).
        assert_eq!(t.lit_timer, 0);
        let l = light(&w, h);
        assert_eq!((l.color, l.radius), ([0, 0, 0], -1));
        // Drawn: gGoldenTorchDL, no flame.
        assert_eq!(draw(&w, h), ("bake/Obj_Syokudai/golden".to_string(), None));
    }
    // Room 0's floor switch sets 0x27 (Obj_Switch: the test sets it). On the next update each
    // torch sees it with litTimer 0: -1, and (torchType 0) OnePointCutscene_Attention. The
    // cutscenes are all on ACTORCAT_PROP actors, so only the first is queued.
    w.flags.set_switch(0x27);
    let frame = w.audio.frames + 1;
    idle(&mut w, 1);
    for &h in &ts {
        assert_eq!(torch(&w, h).lit_timer, -1);
        assert_lit_light(&w, h);
        // Actor_PlaySfx_Flagged(NA_SE_EV_TORCH - SFX_FLAG): the crackle, played by Actor_DrawAll.
        let (mesh, flame) = draw(&w, h);
        assert_eq!(mesh, "bake/Obj_Syokudai/golden");
        // litTimer -1: flameScale 1 * 0.0027.
        assert!((flame_scale(flame.expect("the flame")) - 0.0027).abs() < 1e-6);
    }
    assert!(sfx_on(&w, frame, NA_SE_EV_TORCH - SFX_FLAG));
    let targets = attention_targets(&w);
    assert_eq!(targets.len(), 1, "one attention cutscene");
    assert!(ts.contains(&targets[0].expect("a target")));
    // The brightness is drawn again each frame (Rand_ZeroOne): it flickers.
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..10 {
        idle(&mut w, 1);
        assert_lit_light(&w, ts[0]);
        seen.insert(light(&w, ts[0]).color[0]);
    }
    assert!(seen.len() > 1, "the brightness changes: {seen:?}");
    // 0x27 cleared again (a held switch let go): litTimer < 0 becomes 20, burning out; the light
    // shrinks over the 20 frames ((litTimer * 200) / 20).
    w.flags.temp_swch &= !(1 << (0x27 - 0x20));
    idle(&mut w, 1);
    // 20, then the update's countdown: 19.
    assert_eq!(torch(&w, ts[0]).lit_timer, 19);
    assert_eq!(light(&w, ts[0]).radius, 190);
    idle(&mut w, 19);
    assert_eq!(torch(&w, ts[0]).lit_timer, 0);
    assert_eq!((light(&w, ts[0]).radius, light(&w, ts[0]).color), (-1, [0, 0, 0]));
}

#[test]
fn room_4s_timed_torches_lit_together_set_0x19() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 4);
    let ts = torches(&w, 0x1099);
    assert_eq!(ts.len(), 2);
    let (first, second) = (ts[0], ts[1]);
    assert_eq!(torch(&w, first).actor.world_pos, Vec3::new(-281.0, -880.0, 881.0));
    assert_eq!(torch(&w, second).actor.world_pos, Vec3::new(-282.0, -880.0, 1041.0));
    for &h in &ts {
        // sColMaterialsStand[1]: wood; unlit; gTimedTorchDL.
        assert_eq!((torch(&w, h).stand_collider.base.col_type, torch(&w, h).lit_timer), (cc::COL_MATERIAL_WOOD, 0));
        assert_eq!(draw(&w, h).0, "bake/Obj_Syokudai/timed");
    }
    assert_eq!(*obj_syokudai::lit_torch_count(&mut w), 0);
    // Room 4's two Elf_Msg2 tags above them (rot.y 26: gone once switch flag 25, 0x19, is set).
    let tags = |w: &PlayState| w.actors.all().into_iter().filter(|&h| w.actors.downcast::<oot_actors::elf_msg2::ElfMsg2>(h).is_some_and(|m| !m.actor.killed)).count();
    assert_eq!(tags(&w), 2);

    // Fire on the first: interactionType 1 (DMG_FIRE), litTimer 0, torchType 0x1000, count 2:
    // sLitTorchCount 1 (under 2), litTimer 50 * 2 + 110 = 210, NA_SE_EV_FLAME_IGNITION; then the
    // update's countdown: 209.
    fire_hit(&mut w, first);
    let frame = w.audio.frames + 1;
    idle(&mut w, 1);
    assert_eq!(torch(&w, first).lit_timer, 209);
    assert_eq!(*obj_syokudai::lit_torch_count(&mut w), 1);
    assert!(sfx_on(&w, frame, NA_SE_EV_FLAME_IGNITION));
    assert!(!w.flags.get_switch(0x19));
    assert_lit_light(&w, first);
    // The draw: litTimer 209 over timerMax 2 * 50 + 100 = 200, the flame growing in:
    // (200 - 209 + 10) / 10 = 0.1, times 0.0027.
    let flame = draw(&w, first).1.expect("the flame");
    assert!((flame_scale(flame) - 0.1 * 0.0027).abs() < 1e-7, "{}", flame_scale(flame));
    // 52 up from the torch.
    assert!((flame.w_axis.truncate() - Vec3::new(-281.0, -828.0, 881.0)).length() < 1e-3);
    // Grown in after 9 more frames (litTimer 200: not over timerMax): 1 * 0.0027.
    idle(&mut w, 9);
    assert_eq!(torch(&w, first).lit_timer, 200);
    assert!((flame_scale(draw(&w, first).1.unwrap()) - 0.0027).abs() < 1e-6);

    // Fire on the second in time: sLitTorchCount 2 reaches the count: Flags_SetSwitch(0x19),
    // the attention cutscene on it, litTimer -1. The second is the newer (the room's list order),
    // so it updates first: the first, later in the same frame, sees 0x19 with litTimer > 0: -1.
    fire_hit(&mut w, second);
    idle(&mut w, 1);
    assert!(w.flags.get_switch(0x19));
    assert_eq!((torch(&w, first).lit_timer, torch(&w, second).lit_timer), (-1, -1));
    assert_eq!(*obj_syokudai::lit_torch_count(&mut w), 2);
    // The newest attention camera is the torch's (room 4's door to room 3, unbarred by the
    // clear, attends itself and Link before it).
    assert_eq!(attention_targets(&w).last(), Some(&Some(second)));
    // ElfMsg2_KillCheck on their next update (ACTORCAT_BG updates before the torches'
    // ACTORCAT_PROP): 0x19 set, both tags gone.
    idle(&mut w, 1);
    assert_eq!(tags(&w), 0);
    // Lit for good.
    idle(&mut w, 300);
    for &h in &ts {
        assert_eq!(torch(&w, h).lit_timer, -1);
        assert_lit_light(&w, h);
    }
}

#[test]
fn a_timed_torch_lit_alone_burns_out() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 4);
    let ts = torches(&w, 0x1099);
    let first = ts[0];
    fire_hit(&mut w, first);
    idle(&mut w, 1);
    assert_eq!((torch(&w, first).lit_timer, *obj_syokudai::lit_torch_count(&mut w)), (209, 1));
    // Fire again while lit (litTimer 0 < it < 50 * 2 + 100): back up to 200, not 210, and no
    // second count; then the countdown: 199.
    idle(&mut w, 100);
    assert_eq!(torch(&w, first).lit_timer, 109);
    fire_hit(&mut w, first);
    idle(&mut w, 1);
    assert_eq!((torch(&w, first).lit_timer, *obj_syokudai::lit_torch_count(&mut w)), (199, 1));
    // Its last 20 frames: the light (litTimer * 200) / 20 and the flame litTimer / 20 shrink.
    idle(&mut w, 199 - 19);
    for lit in (1..=19).rev() {
        let t = torch(&w, first);
        assert_eq!(t.lit_timer, lit);
        assert_eq!(light(&w, first).radius, (lit as f32 * 200.0 / 20.0) as i16);
        let s = flame_scale(draw(&w, first).1.expect("the flame"));
        assert!((s - (lit as f32 / 20.0) * 0.0027).abs() < 1e-7, "litTimer {lit}: {s}");
        idle(&mut w, 1);
    }
    // Out: litTimer 0 leaves sLitTorchCount (torchType 0x1000), the light off, no flame; the
    // flag never set.
    let t = torch(&w, first);
    assert_eq!(t.lit_timer, 0);
    assert_eq!(*obj_syokudai::lit_torch_count(&mut w), 0);
    assert_eq!((light(&w, first).radius, light(&w, first).color), (-1, [0, 0, 0]));
    assert_eq!(draw(&w, first).1, None);
    assert!(!w.flags.get_switch(0x19));
}

#[test]
fn room_10s_wooden_torch_is_always_lit_and_its_timed_one_sets_0x13() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 10);
    let wooden = torches(&w, 0x2400)[0];
    // ObjSyokudai_Init: bit 10 (0x400), litTimer -1; sColMaterialsStand[2]: wood;
    // gWoodenTorchDL with its flame at full size; its light set by its first update.
    idle(&mut w, 1);
    let t = torch(&w, wooden);
    assert_eq!((t.lit_timer, t.stand_collider.base.col_type), (-1, cc::COL_MATERIAL_WOOD));
    assert_lit_light(&w, wooden);
    let (mesh, flame) = draw(&w, wooden);
    assert_eq!(mesh, "bake/Obj_Syokudai/wooden");
    assert!((flame_scale(flame.unwrap()) - 0.0027).abs() < 1e-6);
    // Its flag (0) and count (0) change nothing: still lit.
    idle(&mut w, 30);
    assert_eq!(torch(&w, wooden).lit_timer, -1);

    // The timed one (count 1): lit by fire, sLitTorchCount reaches 1 at once: 0x13 (the chest's
    // flag), the attention cutscene, litTimer -1.
    let timed = torches(&w, 0x1053)[0];
    assert_eq!(torch(&w, timed).lit_timer, 0);
    fire_hit(&mut w, timed);
    idle(&mut w, 1);
    assert!(w.flags.get_switch(0x13));
    assert_eq!(torch(&w, timed).lit_timer, -1);
    // The torch's attention camera, queued before those of room 10's door (unbarred by the clear)
    // on itself and Link.
    assert_eq!(attention_targets(&w).first(), Some(&Some(timed)));
}

/// One frame with Player holding a Deku Stick (`heldItemAction` `PLAYER_IA_DEKU_STICK`), its tip
/// (`meleeWeaponInfo[0]`'s, which the draw sets after the updates) at `tip` and its `unk_860` at
/// `unk_860`: what milestone 4b's stick will give, set by hand. Its held item is put back after
/// (its own update would start changing to it otherwise).
fn frame_with_stick(w: &mut PlayState, tip: Vec3, unk_860: i16) {
    let stick = w.data.items.ap("DEKU_STICK");
    let p = w.player_mut();
    let held = (p.held_item_ap, p.item_ap);
    p.held_item_ap = stick;
    p.item_ap = stick;
    p.melee_weapon_info[0].tip = tip;
    p.unk_860 = unk_860;
    idle(w, 1);
    let p = w.player_mut();
    (p.held_item_ap, p.item_ap) = held;
}

#[test]
fn a_deku_stick_at_the_flame_is_lit_or_lights_it() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 4);
    let ts = torches(&w, 0x1099);
    let (first, second) = (ts[0], ts[1]);
    let flame = |w: &PlayState, h: ActorHandle| torch(w, h).actor.world_pos + Vec3::new(0.0, 67.0, 0.0);
    // An unlit stick (unk_860 0) at an unlit torch: interactionType -1, but neither lit: nothing.
    let f = flame(&w, first);
    frame_with_stick(&mut w, f, 0);
    assert_eq!((torch(&w, first).lit_timer, w.player().unk_860), (0, 0));
    // A burning one (unk_860 150) within 20 of the flame (19 off): the torch lit (litTimer 2 * 50
    // + 110, then 209), sLitTorchCount 1, and the stick's unk_860 back up to 200.
    let f = flame(&w, first) + Vec3::new(19.0, 0.0, 0.0);
    let frame = w.audio.frames + 1;
    frame_with_stick(&mut w, f, 150);
    assert_eq!((torch(&w, first).lit_timer, *obj_syokudai::lit_torch_count(&mut w), w.player().unk_860), (209, 1, 200));
    assert!(sfx_on(&w, frame, NA_SE_EV_FLAME_IGNITION));
    // 20 off: not within (SQ(20) is not under SQ(20)).
    let f = flame(&w, second) + Vec3::new(0.0, 0.0, 20.0);
    frame_with_stick(&mut w, f, 150);
    assert_eq!(torch(&w, second).lit_timer, 0);
    // An unlit stick at the lit torch catches fire: unk_860 210, NA_SE_EV_FLAME_IGNITION; the
    // torch's litTimer (208, not under 2 * 50 + 100) left as it is, then 207.
    let f = flame(&w, first);
    let frame = w.audio.frames + 1;
    frame_with_stick(&mut w, f, 0);
    assert_eq!((w.player().unk_860, torch(&w, first).lit_timer), (210, 207));
    assert!(sfx_on(&w, frame, NA_SE_EV_FLAME_IGNITION));
    // Burnt down to 150 (set here: waiting for it, Player's own update, idling, puts his real
    // held item back with Player_InitItemAction, which zeroes unk_860), under 200, the stick at
    // it again: back up to 200, then 199.
    torch_mut(&mut w, first).lit_timer = 150;
    let f = flame(&w, first);
    frame_with_stick(&mut w, f, 205);
    assert_eq!(torch(&w, first).lit_timer, 199);
    // ... the stick burning on (unk_860 205, not under 200): left as it is.
    assert_eq!(w.player().unk_860, 205);
}

#[test]
fn a_keese_flies_to_a_lit_torch_and_catches_fire() {
    let Some(a) = assets() else { return };
    // Room 0 (object_firefly is one of its objects), its golden torches lit by 0x27: the Keese's
    // nearest is the ground floor's at (-153, 0, 2).
    let mut w = deku_tree_room(&a, 0);
    w.flags.set_switch(0x27);
    idle(&mut w, 1);
    let ground = torches(&w, 0x03E7).into_iter().find(|&h| torch(&w, h).actor.world_pos.y == 0.0).expect("the ground floor's torch");
    assert_eq!(torch(&w, ground).lit_timer, -1);
    let torch_pos = torch(&w, ground).actor.world_pos;
    // Link 300 off in x, on the ground floor (the Keese attacks within 200 of him).
    let at = torch_pos + Vec3::new(300.0, 50.0, 0.0);
    let (y, _) = w.col.entity_raycast_down(at);
    assert_eq!(y, 0.0);
    w.place_player(Vec3::new(at.x, y, at.z), -0x4000);
    // A normal Keese (EN_FIREFLY_TYPE_NORMAL) 60 off the torch in z, 100 up, facing it.
    let h = w.actor_spawn(en_firefly::ACTOR_EN_FIREFLY, torch_pos + Vec3::new(0.0, 100.0, 60.0), [0, -0x8000, 0], en_firefly::EN_FIREFLY_TYPE_NORMAL).expect("spawned");
    // EnFirefly_Idle → EnFirefly_ApproachLitTorch: the nearest torch with litTimer != 0 draws it
    // to the flame (52 + 15 up); within 15 of it, EnFirefly_SetElementFire: type 0 (fire),
    // HIT_SPECIAL_EFFECT_FIRE, NAVI_ENEMY_FIRE_KEESE.
    let flame = torch_pos + Vec3::new(0.0, 67.0, 0.0);
    let start = w.actors.downcast::<EnFirefly>(h).unwrap().actor.world_pos.distance(flame);
    let mut closest = start;
    let mut lit = None;
    for f in 0..200 {
        idle(&mut w, 1);
        let k = w.actors.downcast::<EnFirefly>(h).expect("the Keese");
        closest = closest.min(k.actor.world_pos.distance(flame));
        if k.body_elemental_type == en_firefly::EN_FIREFLY_BODY_ELEMENTAL_TYPE_FIRE {
            lit = Some(f);
            break;
        }
    }
    assert!(lit.is_some(), "it never caught fire (closest {closest} of {start})");
    let k = w.actors.downcast::<EnFirefly>(h).unwrap();
    assert_eq!((k.actor.params, k.effects_elemental_type), (en_firefly::EN_FIREFLY_TYPE_FIRE, en_firefly::EN_FIREFLY_EFFECTS_ELEMENTAL_TYPE_FIRE));
    assert_eq!(k.collider.elements[0].info.at_dmg_info.hit_special_effect, cc::HIT_SPECIAL_EFFECT_FIRE);
    assert!(closest < 15.0);
}
