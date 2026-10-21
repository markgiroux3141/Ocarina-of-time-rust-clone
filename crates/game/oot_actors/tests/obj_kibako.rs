//! GAME-10 milestone 5 against the C: the small crate (`z_obj_kibako.c`) in Jabu-Jabu's Belly's
//! room 7, as Master Quest places it (`ootx scene-info --scene bdan_scene`): two 0xFFFF (no drop)
//! at (-666, -340, -4671) and (-512, -340, -4592), and the blue switch (`Bg_Bdan_Switch` 0x0D00,
//! flag 0x0D) at (1, -341, -5178) between two boulders (`Obj_Bombiwa`).
//!
//! Child Link enters at `ENTR_JABU_JABU_0` on a `jabu-boomerang` save, the room changes to room 7
//! (`Room_RequestNewRoom`) and the room's Lizalfos (`En_Zf`) is taken out (no fight). Link is put
//! by a crate and lifts it (A), walks a step and throws it (A); the rest is driven directly.
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_bdan_switch::{self as switch, BgBdanSwitch};
use oot_actors::en_item00::EnItem00;
use oot_actors::obj_kibako::*;
use oot_actors::player::{Action as PA, Player, STATE1_11};
use oot_game::actor::*;
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::sfx::SfxPos;
use oot_game::collision_check::*;
use oot_game::effect::{EFFECT_SS_DUST, EFFECT_SS_KAKERA};
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

fn a_button() -> PadState {
    PadState { button: BTN_A, ..Default::default() }
}

/// One frame with `pad`, then one released.
fn press(w: &mut PlayState, pad: PadState) {
    w.tick_with(scripted_input(PadState::default(), pad));
    w.tick_with(scripted_input(pad, PadState::default()));
}

/// Child Link in Jabu-Jabu (`jabu-boomerang`), the sound log on, in room 7, its actors in, the
/// Lizalfos gone, Link his own (no cutscene's hold).
fn room_7(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_JABU_JABU_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("jabu-boomerang").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    assert!(w.room_request(7));
    idle(&mut w, 1);
    w.room_change_done();
    assert_eq!(w.room_ctx.cur.num, 7);
    idle(&mut w, 20);
    for h in w.actors.all() {
        if w.actors.downcast::<oot_actors::en_zf::EnZf>(h).is_some() {
            w.actors.actor_mut(h).unwrap().kill();
        }
    }
    for _ in 0..400 {
        if !w.player_in_cs_mode() {
            break;
        }
        idle(&mut w, 1);
    }
    idle(&mut w, 2);
    w
}

const WEST: Vec3 = Vec3::new(-666.0, -340.0, -4671.0);
const EAST: Vec3 = Vec3::new(-512.0, -340.0, -4592.0);
const BLUE_SWITCH: Vec3 = Vec3::new(1.0, -341.0, -5178.0);

fn crates(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<ObjKibako>(h).is_some_and(|c| !c.actor.killed)).collect()
}

fn at_home(w: &PlayState, home: Vec3) -> ActorHandle {
    *crates(w).iter().find(|&&h| kb(w, h).actor.home_pos == home).expect("the crate")
}

fn kb(w: &PlayState, h: ActorHandle) -> &ObjKibako {
    w.actors.downcast::<ObjKibako>(h).expect("Obj_Kibako")
}

fn kb_mut(w: &mut PlayState, h: ActorHandle) -> &mut ObjKibako {
    w.actors.downcast_mut::<ObjKibako>(h).expect("Obj_Kibako")
}

fn gone(w: &PlayState, h: ActorHandle) -> bool {
    w.actors.downcast::<ObjKibako>(h).is_none_or(|c| c.actor.killed)
}

fn player(w: &PlayState) -> &Player {
    w.actors.downcast::<Player>(w.player.unwrap()).unwrap()
}

/// The actor's update alone (`Actor_UpdateAll`'s call), as the updating actor.
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

fn sfx(w: &PlayState, h: ActorHandle, id: u16) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, p)| s == id && p == SfxPos::Actor(h)).count()
}

fn effects(w: &PlayState, ty: u8) -> usize {
    w.effect_ss.table.iter().filter(|e| e.ty == ty && e.life >= 0).count()
}

#[test]
fn room_7s_two_crates_stand_where_placed() {
    let Some(a) = assets() else { return };
    let w = room_7(&a);
    assert_eq!(crates(&w).len(), 2);
    for home in [WEST, EAST] {
        let c = kb(&w, at_home(&w, home));
        // ObjKibako_Init: scale 0.1, gravity -1.2, minVelocityY -13; sCCInfoInit 12 by 60,
        // MASS_HEAVY; the cylinder 12 by 27; ObjKibako_Idle, on the floor.
        assert_eq!((c.actor.params, c.actor.scale, c.actor.gravity, c.actor.min_velocity_y), (-1, Vec3::splat(0.1), -1.2, -13.0));
        let i = &c.actor.col_chk_info;
        assert_eq!((i.cyl_radius, i.cyl_height, i.mass), (12, 60, MASS_HEAVY));
        assert_eq!((c.collider.dim.radius, c.collider.dim.height), (12, 27));
        assert_eq!(c.action, Action::Idle);
        assert_ne!(c.actor.flags & ACTOR_FLAG_CAN_PRESS_SWITCHES, 0);
        assert_ne!(c.actor.bg_check_flags & BGCHECKFLAG_GROUND, 0);
        assert_eq!(c.actor.world_pos.y, c.actor.floor_height);
    }
}

#[test]
fn a_crate_is_lifted_carried_and_thrown_and_breaks_where_it_lands() {
    let Some(a) = assets() else { return };
    let mut w = room_7(&a);
    let h = at_home(&w, EAST);
    // Link 20 from it, facing it a little off-axis: within 100 ObjKibako_Idle offers it
    // (Actor_OfferCarry: 50 across, 10 up).
    let c = kb(&w, h).actor.world_pos;
    let mut offered = false;
    for _ in 0..20 {
        w.place_player(Vec3::new(c.x + 2.0, c.y, c.z - 20.0), -0x200);
        idle(&mut w, 1);
        if player(&w).interact_range_actor == Some(h) {
            offered = true;
            break;
        }
    }
    assert!(offered, "never offered");
    // A: the lift; on its frame Player takes it (its parent): ObjKibako_SetupHeld, room -1,
    // NA_SE_PL_PULL_UP_WOODBOX.
    press(&mut w, a_button());
    let mut n = 0;
    while player(&w).action != PA::StandingStill || player(&w).held_actor.is_none() {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 80, "never carried it: {:?}", player(&w).action);
    }
    assert_eq!(player(&w).held_actor, Some(h));
    assert_ne!(player(&w).state1 & STATE1_11, 0);
    assert_eq!((kb(&w, h).action, kb(&w, h).actor.room), (Action::Held, -1));
    assert_eq!(sfx(&w, h, NA_SE_PL_PULL_UP_WOODBOX), 1);
    let (cr, link) = (kb(&w, h).actor.world_pos, player(&w).actor.world_pos);
    assert!(cr.y > link.y + 20.0, "over his head: {cr}, Link at {link}");
    // Walking (it's not throw-only: thrown only at 1.1 or faster, else put down), A: thrown
    // (func_8083EA94); let go: ObjKibako_Held into room 7 and ObjKibako_SetupThrown (velocity along
    // its yaw at its speed, mass 240).
    let walk = stick(0, 60);
    for _ in 0..6 {
        w.tick_with(scripted_input(walk, walk));
    }
    let ab = with(walk, BTN_A);
    w.tick_with(scripted_input(walk, ab));
    w.tick_with(scripted_input(ab, walk));
    let mut n = 0;
    while kb(&w, h).action == Action::Held {
        w.tick_with(scripted_input(walk, walk));
        n += 1;
        assert!(n < 20, "never thrown");
    }
    let k = kb(&w, h);
    assert_eq!((k.action, k.actor.room, k.actor.col_chk_info.mass), (Action::Thrown, 7, 240));
    assert!(k.actor.speed_xz >= 1.1);
    // ObjKibako_Thrown until the ground (or a wall): twelve splinters (gSmallWoodenBoxFragmentDL),
    // dust (func_80033480: four puffs), NA_SE_EV_WOODBOX_BREAK (its source), no drop (0x1F), gone.
    let kakera = effects(&w, EFFECT_SS_KAKERA);
    let items = w.actors.all().into_iter().filter(|&i| w.actors.downcast::<EnItem00>(i).is_some()).count();
    let mut n = 0;
    while !gone(&w, h) {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 60, "never landed");
    }
    assert_eq!(effects(&w, EFFECT_SS_KAKERA), kakera + 12);
    assert!(w.effect_ss.table.iter().any(|e| e.ty == EFFECT_SS_KAKERA && e.gfx == Some((OBJECT, FRAGMENT_DL))));
    assert!(effects(&w, EFFECT_SS_DUST) >= 4);
    assert_eq!(w.actors.all().into_iter().filter(|&i| w.actors.downcast::<EnItem00>(i).is_some()).count(), items);
    assert_eq!(player(&w).held_actor, None);
}

#[test]
fn a_crate_put_down_stands_again() {
    let Some(a) = assets() else { return };
    let mut w = room_7(&a);
    let h = at_home(&w, WEST);
    // Held and let go still (speed under 0.1): NA_SE_EV_PUT_DOWN_WOODBOX, ObjKibako_SetupIdle
    // (MASS_HEAVY), Link's push off (OC1_TYPE_PLAYER) until he's 28 away.
    {
        let k = kb_mut(&mut w, h);
        k.action = Action::Held;
        k.actor.room = -1;
        k.actor.parent = None;
        k.actor.speed_xz = 0.05;
        k.actor.xz_dist_to_player = 10.0;
    }
    update(&mut w, h);
    let k = kb(&w, h);
    assert_eq!((k.action, k.actor.room, k.actor.col_chk_info.mass), (Action::Idle, 7, MASS_HEAVY));
    assert_eq!(k.collider.base.oc_flags1 & OC1_TYPE_PLAYER, 0);
    assert_eq!(sfx(&w, h, NA_SE_EV_PUT_DOWN_WOODBOX), 1);
    // Standing with Link 28 across, still off; over 28, on.
    update(&mut w, h);
    assert_eq!(kb(&w, h).collider.base.oc_flags1 & OC1_TYPE_PLAYER, 0);
    kb_mut(&mut w, h).actor.xz_dist_to_player = 29.0;
    update(&mut w, h);
    assert_ne!(kb(&w, h).collider.base.oc_flags1 & OC1_TYPE_PLAYER, 0);
}

#[test]
fn a_hit_breaks_a_crate_and_it_drops_its_collectible() {
    let Some(a) = assets() else { return };
    let mut w = room_7(&a);
    // A crate with a red rupee (ITEM00_RUPEE_RED, 2) under collectible flag 5 (params 0x0502).
    let at = Vec3::new(-600.0, -340.0, -4700.0);
    let h = w.actor_spawn(ACTOR_OBJ_KIBAKO, at, [0, 0, 0], 0x0502).expect("Obj_Kibako");
    idle(&mut w, 3);
    // A sword's hit (its AC): ObjKibako_AirBreak, NA_SE_EV_WOODBOX_BREAK, the drop, gone.
    let kakera = effects(&w, EFFECT_SS_KAKERA);
    kb_mut(&mut w, h).collider.base.ac_flags |= AC_HIT;
    update(&mut w, h);
    assert!(gone(&w, h));
    assert_eq!(effects(&w, EFFECT_SS_KAKERA), kakera + 12);
    let drop = w.actors.all().into_iter().find_map(|i| w.actors.downcast::<EnItem00>(i).filter(|d| d.actor.home_pos.distance(at) < 30.0)).expect("the drop");
    assert_eq!((drop.actor.params & 0xFF, drop.collectible_flag), (2, 5));
    // Its own AC takes Player's 0x4FC00748 (a sword, Deku stick, slingshot, hammer, explosives,
    // arrows): the boomerang's 0x10 isn't in it.
    assert_ne!(CYLINDER_INIT.info.ac_dmg_info.dmg_flags & DMG_SLASH_KOKIRI, 0);
    assert_eq!(CYLINDER_INIT.info.ac_dmg_info.dmg_flags & DMG_BOOMERANG, 0);
}

#[test]
fn a_crate_on_the_blue_switch_holds_it_down() {
    let Some(a) = assets() else { return };
    let mut w = room_7(&a);
    let sh = w.actors.all().into_iter().find(|&h| w.actors.downcast::<BgBdanSwitch>(h).is_some_and(|s| s.actor.home_pos == BLUE_SWITCH)).expect("the blue switch");
    assert_eq!(w.actors.actor(sh).unwrap().params, 0x0D00);
    assert_eq!(w.actors.downcast::<BgBdanSwitch>(sh).unwrap().action, switch::Action::Up);
    assert!(!w.flags.get_switch(0x0D));
    // The two boulders over it blown away (taken out), and the west crate put just over the switch.
    for h in w.actors.all() {
        if w.actors.downcast::<oot_actors::obj_bombiwa::ObjBombiwa>(h).is_some() {
            w.actors.actor_mut(h).unwrap().kill();
        }
    }
    let h = at_home(&w, WEST);
    {
        let k = kb_mut(&mut w, h);
        k.actor.world_pos = BLUE_SWITCH + Vec3::Y * 20.0;
        k.actor.velocity = Vec3::ZERO;
    }
    // Falling onto it, it presses it (Actor_UpdateBgCheckInfo's floor on its collision with
    // ACTOR_FLAG_CAN_PRESS_SWITCHES: DYNA_INTERACT_ACTOR_SWITCH_PRESSED): func_8086D5E0 sets the flag
    // and it sinks to 0.1 (func_8086D694), down (func_8086D754) while the crate stays on it.
    let mut n = 0;
    while w.actors.downcast::<BgBdanSwitch>(sh).unwrap().action != switch::Action::Down {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 200, "never pressed: {:?}", w.actors.downcast::<BgBdanSwitch>(sh).unwrap().action);
    }
    assert!(w.flags.get_switch(0x0D));
    assert_eq!(kb(&w, h).actor.floor_bg_id, w.actors.downcast::<BgBdanSwitch>(sh).unwrap().bg);
    idle(&mut w, 30);
    assert_eq!(w.actors.downcast::<BgBdanSwitch>(sh).unwrap().action, switch::Action::Down);
    assert!(w.flags.get_switch(0x0D));
    // The crate gone: 6 frames on, the switch rises and clears the flag.
    w.actors.actor_mut(h).unwrap().kill();
    let mut n = 0;
    while w.actors.downcast::<BgBdanSwitch>(sh).unwrap().action == switch::Action::Down {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 20, "never let go");
    }
    assert!(!w.flags.get_switch(0x0D));
}
