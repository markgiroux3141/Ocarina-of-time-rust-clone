//! `En_Po_Field` (`z_en_po_field.c`) against the C: the adult's Hyrule Field (`spot00` layer 2) on
//! `epona-won`, its ten placements (params 0x16 to 0x1F), Link on foot or riding Epona
//! (`R_EXITED_SCENE_RIDING_HORSE`: `Horse_SetupInGameplay` spawns her under him).
//!
//! No fight is scripted: the soul's states are driven directly. The expectations are worked out
//! from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_horse::EnHorse;
use oot_actors::en_po_field::*;
use oot_game::actor::*;
use oot_game::actor_ctx::ActorHandle;
use oot_game::item::{ITEM_BOTTLE_BIG_POE, ITEM_BOTTLE_EMPTY, SLOT_BOTTLE_1, item_give};
use oot_game::message::{TEXT_STATE_AWAITING_NEXT, TEXT_STATE_CHOICE, TEXT_STATE_DONE, TEXT_STATE_DONE_HAS_NEXT, TEXT_STATE_EVENT, TEXT_STATE_NONE};
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

/// The placements' places and switch flags, in the room's order (`spot00_room_0`, layers 2 and 3).
const PLACES: [([i16; 3], u8); 10] = [
    ([-5215, -273, 1504], 0x16),
    ([-1933, 104, 5455], 0x17),
    ([-118, 27, 2167], 0x18),
    ([3709, 164, 1721], 0x19),
    ([1361, 421, 5869], 0x1A),
    ([679, -118, 8268], 0x1B),
    ([-446, -417, 11256], 0x1C),
    ([-29, -473, 12613], 0x1D),
    ([-6267, -473, 7135], 0x1E),
    ([-6146, -473, 9007], 0x1F),
];

/// The field from Lon Lon Ranch (`ENTR_HYRULE_FIELD_6`, spawn 6 at (-1850, 196, 5956) on the
/// ramp: 501 from place 0x17 on z), an adult on `epona-won`, on Epona if `riding`; `edit` before
/// Play_Init.
fn field(a: &Arc<GameAssets>, riding: bool, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_HYRULE_FIELD_6").unwrap();
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("epona-won").unwrap();
    save.r_exited_scene_riding_horse = riding;
    edit(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap();
    w.audio.log = Some(Default::default());
    assert_eq!(w.save.scene_layer, 2);
    // The room's actors init once their objects are in (object_po_field's, object_horse's).
    for _ in 0..30 {
        if w.overlay_statics.contains_key(&ACTOR_EN_PO_FIELD) {
            break;
        }
        idle(&mut w, 1);
    }
    idle(&mut w, 1);
    assert_eq!(w.player().state1 & PLAYER_STATE1_23 != 0, riding);
    w
}

/// Place 0x17 (by the ramp up to the ranch's gate).
const PLACE: Vec3 = Vec3::new(-1933.0, 104.0, 5455.0);
/// Place 0x17's index in the statics (the ten recorded last placed first).
const PLACE_0X17: u8 = 8;

fn po_fields(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnPoField>(h).is_some_and(|p| !p.actor.killed)).collect()
}

fn the_poe(w: &PlayState) -> ActorHandle {
    let all = po_fields(w);
    assert_eq!(all.len(), 1, "one En_Po_Field lives");
    all[0]
}

fn poe(w: &PlayState, h: ActorHandle) -> &EnPoField {
    w.actors.downcast::<EnPoField>(h).expect("En_Po_Field")
}

fn poe_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnPoField {
    w.actors.downcast_mut::<EnPoField>(h).expect("En_Po_Field")
}

fn statics_of(w: &PlayState) -> Statics {
    w.overlay_statics.get(&ACTOR_EN_PO_FIELD).and_then(|s| s.downcast_ref::<Statics>()).cloned().expect("the overlay's statics")
}

fn sfx_since(w: &PlayState, from: usize, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx[from..].iter().any(|&(_, s, _)| s == id)
}

fn until(w: &mut PlayState, max: usize, what: &str, f: impl Fn(&PlayState) -> bool) -> usize {
    for n in 0..max {
        if f(w) {
            return n;
        }
        idle(w, 1);
    }
    panic!("{what}: not after {max} frames");
}

/// Epona (Link on her) put at `at` on the floor, facing `yaw`, standing.
fn ride_to(w: &mut PlayState, at: Vec3, yaw: i16) {
    let floor = w.col.entity_raycast_down(at + Vec3::Y * 200.0).0;
    let horse = w.player().ride_actor.expect("riding");
    let name = w.actors.get(horse).map(|a| a.name());
    let h = w.actors.downcast_mut::<EnHorse>(horse).unwrap_or_else(|| panic!("Epona: {name:?}"));
    h.actor.world_pos = Vec3::new(at.x, floor, at.z);
    h.actor.prev_pos = h.actor.world_pos;
    h.actor.shape_rot.y = yaw;
    h.actor.world_rot.y = yaw;
    h.actor.speed_xz = 0.0;
}

#[test]
fn one_instance_lives_for_the_ten_places() {
    let Some(a) = assets() else { return };
    let mut w = field(&a, false, |_| {});
    // EnPoField_Init: each records its place and flag (sNumSpawned 1 to 10); from the second on,
    // params 0xFF and Actor_Kill. The inits run as object_po_field comes in, in the enemies'
    // list's order (Actor_AddToCategory puts each new actor at its head): the last placed first.
    let s = statics_of(&w);
    assert_eq!(s.num_spawned, 10);
    for (i, (pos, flag)) in PLACES.iter().rev().enumerate() {
        assert_eq!((s.spawn_positions[i], s.spawn_switch_flags[i]), (*pos, *flag), "place {i}");
    }
    let h = the_poe(&w);
    let p = poe(&w, h);
    // The first to init (0x1F's), waiting: EnPoField_SetupWaitForSpawn's 200 frames, scale 0,
    // not targetable, 4 health, gravity 0, no AC.
    assert_eq!(p.actor.home_pos, Vec3::new(-6146.0, -473.0, 9007.0));
    assert_eq!(p.actor.params, 0x1F, "its params as placed until one appears");
    assert_eq!((p.action, p.update_fn, p.draw_fn), (Action::WaitForSpawn, Update::Living, Draw::Body));
    assert_eq!((p.actor.scale, p.actor.col_chk_info.health, p.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED), (Vec3::ZERO, 4, 0));
    assert_eq!(p.collider.base.ac_flags & oot_game::collision_check::AC_ON, 0);
    assert_eq!(p.actor.target_arrow_offset, 3200.0);
    let left = p.action_timer;
    assert!(left <= 200 && left > 150, "{left}");
    // The light: Lights_PointGlowSetInfo at its home, then colour and radius 0.
    let l = w.light_ctx.info(p.light_node.expect("its light")).unwrap();
    assert_eq!(([l.x, l.y, l.z], l.color, l.radius), ([-6146, -473, 9007], [0, 0, 0], 0));
    // The nine are gone (their destroy frees nothing: params 0xFF); the statics stay with the
    // live one.
    let all = |w: &PlayState| w.actors.all().into_iter().filter(|&x| w.actors.downcast::<EnPoField>(x).is_some()).collect::<Vec<_>>();
    until(&mut w, 30, "the nine deleted", |w| all(w).len() == 1);
    assert_eq!(all(&w), vec![h]);
    assert_eq!(statics_of(&w).num_spawned, 10);
    // Away from every place, the timer out: nothing appears.
    idle(&mut w, 250);
    let p = poe(&w, h);
    assert_eq!((p.action, p.action_timer, p.actor.scale), (Action::WaitForSpawn, 0, Vec3::ZERO));
    // The last instance deleted: the overlay unloads and its statics go (a fresh load's zeroes).
    w.delete_actor(h);
    assert!(!w.overlay_statics.contains_key(&ACTOR_EN_PO_FIELD));
}

/// Riding (Epona put at place 0x17 from the start, facing the field's middle): the frame the
/// Poe's 200 frames run out and its first update after.
fn ride_at_place(a: &Arc<GameAssets>, edit: impl FnOnce(&mut SaveContext)) -> (PlayState, ActorHandle, i16) {
    let mut w = field(a, true, edit);
    idle(&mut w, 1);
    assert!(w.player().state1 & PLAYER_STATE1_23 != 0, "on Epona");
    let yaw = eng_math::vec3f_yaw(PLACE, S_FIELD_MIDDLE);
    for _ in 0..2 {
        ride_to(&mut w, PLACE, yaw);
        idle(&mut w, 1);
    }
    let lp = w.player().actor.world_pos;
    assert!((lp.x - PLACE.x).abs() < 150.0 && (lp.z - PLACE.z).abs() < 150.0, "Link at the place: {lp}");
    let h = the_poe(&w);
    (w, h, yaw)
}

#[test]
fn riding_by_a_place_a_big_poe_appears_ahead_and_flees() {
    let Some(a) = assets() else { return };
    let (mut w, h, yaw) = ride_at_place(&a, |_| {});
    // EnPoField_WaitForSpawn: nothing until its 200 frames are out, Link (Epona held standing)
    // at the place all along.
    let mut frames = 0;
    while poe(&w, h).action == Action::WaitForSpawn {
        let before = poe(&w, h).action_timer;
        assert!(before > 0, "the timer out and still waiting");
        let s = w.audio.log.as_ref().unwrap().sfx.len();
        ride_to(&mut w, PLACE, yaw);
        idle(&mut w, 1);
        frames += 1;
        assert!(frames < 220);
        if poe(&w, h).action != Action::WaitForSpawn {
            // The 200th update: the timer reaches 0 and the check runs at once.
            assert_eq!(before, 1);
            assert!(sfx_since(&w, s, NA_SE_EN_PO_APPEAR));
        }
    }
    let lp = w.player().actor.world_pos;
    let lyaw = w.player().actor.shape_rot.y;
    let p = poe(&w, h);
    // Riding, switch 0x17 clear: a Big Poe (no Rand), spawnFlagIndex 8 (0x17's), 480 ahead of Link on
    // the floor (then moved once at 12 by EnPoField_Update's Actor_MoveXZGravity).
    assert_eq!((p.action, p.actor.params, p.spawn_flag_index), (Action::Appear, EN_PO_FIELD_BIG, PLACE_0X17));
    // (Link as the Poe saw him: Player's update ran before its own, but the frame's collisions
    // nudge him after.)
    let at = p.actor.world_pos - p.actor.velocity;
    let dist = ((at.x - lp.x).powi(2) + (at.z - lp.z).powi(2)).sqrt();
    let dir = eng_math::vec3f_yaw(lp, at);
    assert!((dist - 480.0).abs() < 16.0 && (dir.wrapping_sub(lyaw) as i32).abs() < 0x100, "{at}: {dist} at {dir:#x}, Link's yaw {lyaw:#x}");
    assert_eq!(p.actor.home_pos.y, at.y, "home.y the floor's");
    // EnPoField_SetupAppear's Big Poe: 35 by 100 up 10, shadow 45, NAVI_ENEMY_BIG_POE.
    assert_eq!((p.collider.dim.radius, p.collider.dim.height, p.collider.dim.y_shift), (35, 100, 10));
    assert_eq!((p.actor.shape_shadow_scale, p.actor.navi_enemy_id, p.scale_modifier), (45.0, NAVI_ENEMY_BIG_POE, 0.014));
    // EnPoField_Appear: grows 0.0014 a frame for 10 frames, then fades in; flees at the end.
    until(&mut w, 60, "fleeing", |w| poe(w, h).action == Action::Flee);
    let p = poe(&w, h);
    assert_eq!((p.light_color[3], p.actor.scale.x, p.action_timer), (255, 0.014, 2000));
    assert_ne!(p.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    // EnPoField_Flee: Epona standing (speed 0): 12 at least; away from Link (world.rot.y is
    // shape.rot.y + 0x8000, shape.rot.y towards him).
    idle(&mut w, 1);
    let p = poe(&w, h);
    assert!(p.actor.speed_xz >= 12.0, "{}", p.actor.speed_xz);
    assert_eq!(p.actor.world_rot.y, p.actor.shape_rot.y.wrapping_add(i16::MIN));
    let d0 = p.actor.xz_dist_to_player;
    idle(&mut w, 10);
    assert!(poe(&w, h).actor.xz_dist_to_player > d0, "it moves away");
}

#[test]
fn riding_by_a_place_whose_flag_is_set_nothing_appears() {
    let Some(a) = assets() else { return };
    let (mut w, h, yaw) = ride_at_place(&a, |_| {});
    // The place's switch flag set (its Big Poe bottled): riding, WaitForSpawn returns.
    w.flags.set_switch(0x17);
    for _ in 0..260 {
        ride_to(&mut w, PLACE, yaw);
        idle(&mut w, 1);
        let lp = w.player().actor.world_pos;
        assert!((lp.x - PLACE.x).abs() < 150.0 && (lp.z - PLACE.z).abs() < 150.0, "Link at the place: {lp}");
    }
    let p = poe(&w, h);
    assert_eq!((p.action, p.actor.params, p.action_timer), (Action::WaitForSpawn, 0x1F, 0));
}

/// Through texts until `f` holds: A whenever a text waits; `choice` at the choice.
fn answer_until(w: &mut PlayState, max: usize, texts: &mut Vec<u16>, choice: u8, f: impl Fn(&PlayState) -> bool) {
    let mut pressed = false;
    for _ in 0..max {
        if f(w) {
            return;
        }
        let st = w.message_state();
        if st == TEXT_STATE_CHOICE && !pressed {
            w.msg_ctx.choice_index = choice;
        }
        let a = !pressed && matches!(st, TEXT_STATE_CHOICE | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_EVENT | TEXT_STATE_AWAITING_NEXT);
        let mut record = |w: &PlayState| {
            if w.message_state() != TEXT_STATE_NONE && w.msg_ctx.text_id != 0 && texts.last() != Some(&w.msg_ctx.text_id) {
                texts.push(w.msg_ctx.text_id);
            }
        };
        record(w);
        let pad = PadState { button: if a { BTN_A } else { 0 }, ..Default::default() };
        w.tick_with(scripted_input(PadState::default(), pad));
        pressed = a;
        record(w);
    }
    panic!("not after {max} frames (texts {texts:x?}, state {})", w.message_state());
}

#[test]
fn a_big_poes_soul_goes_in_a_bottle_and_its_flag_is_set() {
    let Some(a) = assets() else { return };
    let mut w = field(&a, false, |s| {
        item_give(s, None, ITEM_BOTTLE_EMPTY);
    });
    // Link on the open field below the ranch's ramp (over 150 from place 0x17), settled.
    let spot = Vec3::new(-1650.0, 0.0, 5250.0);
    let floor = w.col.entity_raycast_down(spot + Vec3::Y * 1000.0).0;
    w.place_player(Vec3::new(spot.x, floor, spot.z), 0);
    idle(&mut w, 10);
    let slot = (SLOT_BOTTLE_1..SLOT_BOTTLE_1 + 4).find(|&i| w.save.inventory.items[i] == ITEM_BOTTLE_EMPTY).expect("an empty bottle");
    let h = the_poe(&w);
    // The Big Poe of place 0x17 burnt to its lantern, broken on the floor 40 east of Link
    // (func_80AD42B0's state, driven directly): EnPoField_UpdateDead, the soul's draw.
    let lp = w.player().actor.world_pos;
    let at = Vec3::new(lp.x + 40.0, 0.0, lp.z);
    let at = Vec3::new(at.x, w.col.entity_raycast_down(at + Vec3::Y * (lp.y + 100.0)).0, at.z);
    {
        let p = poe_mut(&mut w, h);
        p.actor.params = EN_PO_FIELD_BIG;
        p.spawn_flag_index = PLACE_0X17;
        p.update_fn = Update::Dead;
        p.draw_fn = Draw::Soul;
        p.action = Action::Func80AD587C;
        p.actor.world_pos = at;
        p.actor.home_pos = at;
        p.light_color = [0; 4];
        p.actor.scale = Vec3::ZERO;
    }
    // func_80AD587C: 20 a frame up to 255 (13 frames), then func_80AD4384: 0x5005, 400 frames.
    until(&mut w, 20, "the soul whole", |w| poe(w, h).action == Action::Func80AD58D4);
    let p = poe(&w, h);
    assert_eq!((p.light_color[3], p.actor.text_id, p.action_timer), (255, TEXT_POE_SOUL, 400));
    assert_eq!(&p.light_color[..3], &[255, 200, 0], "sPoFieldInfo[1].lightColor");
    assert!(!w.flags.get_switch(0x17));
    // Link walked into it: its OC1_HIT takes the talk at once.
    let sp = poe(&w, h).actor.world_pos;
    w.place_player(Vec3::new(sp.x - 5.0, lp.y, sp.z), 0x4000);
    until(&mut w, 10, "talking", |w| poe(w, h).action == Action::SoulInteract);
    let mut texts = Vec::new();
    let s = w.audio.log.as_ref().unwrap().sfx.len();
    answer_until(&mut w, 600, &mut texts, 0, |w| poe(w, h).action == Action::WaitForSpawn);
    // Yes with an empty bottle: NA_SE_EN_PO_BIG_GET, 0x508F, ITEM_BOTTLE_BIG_POE, and
    // sSpawnSwitchFlags[8] (0x17) set; it fades (13 a frame) and waits again.
    assert_eq!(texts.first(), Some(&TEXT_POE_SOUL), "{texts:x?}");
    assert!(texts.contains(&TEXT_BIG_POE_BOTTLED), "{texts:x?}");
    assert!(sfx_since(&w, s, NA_SE_EN_PO_BIG_GET));
    assert_eq!(w.save.inventory.items[slot], ITEM_BOTTLE_BIG_POE);
    assert!(w.flags.get_switch(0x17));
    let p = poe(&w, h);
    assert_eq!((p.update_fn, p.action_timer, p.actor.scale), (Update::Living, 200, Vec3::ZERO));
    // @bug (game): EnPoField_SetupWaitForSpawn leaves the soul's draw.
    assert_eq!(p.draw_fn, Draw::Soul);
}

#[test]
fn its_bakes_are_in_the_pack() {
    let Some(a) = assets() else { return };
    for b in oot_actors::en_po_field::bakes() {
        let d: eng_gfx::DrawList = a.pack.assets.get(&oot_game::pack::keys::bake(&b.name)).expect(&b.name);
        assert!(d.triangle_count() > 0, "{}", b.name);
    }
    let d: eng_gfx::DrawList = a.pack.assets.get(&oot_game::pack::keys::mesh(PO_FIELD, "gPoeFieldBurnDL")).expect("the burning shape");
    assert!(d.triangle_count() > 0);
}
