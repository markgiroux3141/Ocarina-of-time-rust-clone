//! GAME-09 milestone 1 against the C: the bomb (`z_en_bom.c`), the Bomb Flower (`z_en_bombf.c`),
//! Player's explosive in hand (`Player_InitExplosiveIA`, `Player_UpperAction_CarryActor`) and the
//! carry's hand-over (`func_8002F5C4`), the fuse's sparks (`z_eff_ss_g_spk.c`); and what the blasts
//! break (`Bg_Breakwall`, `Obj_Bombiwa`).
//!
//! The bombs are taken out on Hyrule Field's open grass (`deku-tree-dead` with the 20 bomb bag);
//! the walls, rocks and flowers are Dodongo's Cavern's lobby's (`ddan` room 0, Master Quest).
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, BTN_CLEFT, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_bom::{ACTOR_EN_BOM, EnBom};
use oot_actors::en_bombf::{ACTOR_EN_BOMBF, EnBombf};
use oot_actors::player::Action;
use oot_game::actor::ACTOR_FLAG_ATTENTION_ENABLED;
use oot_game::actor_ctx::{ACTORCAT_EXPLOSIVE, ActorHandle};
use oot_game::effect::{EFFECT_SS_BLAST, EFFECT_SS_BOMB2, EFFECT_SS_G_SPK};
use oot_game::env::clock_time;
use oot_game::item::{ITEM_BOMB, ITEM_BOMB_BAG_20, ITEM_STRENGTH_GORONS_BRACELET, item_give};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// `PLAYER_STATE1_CARRYING_ACTOR` (`player.h`: 1 << 11).
const CARRYING: u32 = 1 << 11;
/// Where Link stands in Hyrule Field: the grass north of Lon Lon Ranch (as `tests/field.rs`).
const OPEN_FIELD: Vec3 = Vec3::new(1500.0, 0.0, 2000.0);

/// `deku-tree-dead`, the 20 bomb bag (`Item_Give(ITEM_BOMB_BAG_20)`: 20 bombs) with the bombs on
/// C-Left.
fn with_bombs(save: &mut SaveContext) {
    save.apply_preset("deku-tree-dead").unwrap();
    item_give(save, None, ITEM_BOMB_BAG_20);
    save.equips.button_items[1] = ITEM_BOMB;
    save.equips.c_button_slots[0] = oot_game::item::slot(ITEM_BOMB) as u8;
}

/// Child Link on Hyrule Field's grass at 10:00 with bombs, facing east, two frames on.
fn field(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_HYRULE_FIELD_0").expect("entrance");
    let mut save = SaveContext::new(e, false, clock_time(10, 0) as u16);
    with_bombs(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    w.place_player(OPEN_FIELD, 0x4000);
    idle(&mut w, 2);
    w
}

/// Child Link in Dodongo's Cavern's lobby (`ENTR_DODONGOS_CAVERN_0`), placed at `pos` facing
/// `yaw`, two frames on; with bombs, and the bracelet if `bracelet`.
fn cavern(a: &Arc<GameAssets>, pos: Vec3, yaw: i16, bracelet: bool) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DODONGOS_CAVERN_0").expect("entrance");
    let mut save = SaveContext::new(e, false, clock_time(10, 0) as u16);
    with_bombs(&mut save);
    if bracelet {
        item_give(&mut save, None, ITEM_STRENGTH_GORONS_BRACELET);
    }
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    w.place_player(pos, yaw);
    idle(&mut w, 2);
    w
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// One frame with `buttons` pressed (not held the frame before).
fn press(w: &mut PlayState, buttons: u16) {
    w.tick_with(scripted_input(PadState::default(), with(PadState::default(), buttons)));
}

fn all<T: 'static + oot_game::actor_ctx::ActorImpl>(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<T>(h).is_some()).collect()
}

fn bomb(w: &PlayState, h: ActorHandle) -> &EnBom {
    w.actors.downcast::<EnBom>(h).expect("the bomb")
}

fn effects_of(w: &PlayState, ty: u8) -> usize {
    w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == ty).count()
}

fn ammo(w: &PlayState) -> i8 {
    w.save.inventory.ammo[oot_game::item::slot(ITEM_BOMB)]
}

/// C-Left until a bomb is in the arena; the frames it took.
fn take_out_bomb(w: &mut PlayState) -> (ActorHandle, usize) {
    press(w, BTN_CLEFT);
    for n in 1..60 {
        if let Some(&h) = all::<EnBom>(w).first() {
            return (h, n);
        }
        idle(w, 1);
    }
    panic!("no bomb in hand");
}

#[test]
fn a_bomb_taken_out_in_hand_counts_down_and_blows_in_it() {
    let Some(a) = assets() else { return };
    let mut w = field(&a);
    let (health, start_ammo) = (w.save.health, ammo(&w));
    assert_eq!(start_ammo, 20, "Item_Give(ITEM_BOMB_BAG_20): 20 bombs");
    let (h, _) = take_out_bomb(&mut w);
    let me = w.player;
    // Player_InitExplosiveIA: Actor_SpawnAsChild (Player its parent, it his child), the ammo spent,
    // held and carried. En_Bom updates after Player in its own category's turn: its timer 70 is 69
    // at the end of the frame it's spawned in; scale 0 (sInitChain), shape.yOffset 700.
    let p = w.player();
    assert_eq!((p.held_actor, p.actor.child, p.state1 & CARRYING), (Some(h), Some(h), CARRYING));
    assert_eq!(ammo(&w), start_ammo - 1);
    let b = bomb(&w, h);
    assert_eq!((b.actor.parent, b.actor.category, b.actor.params, b.timer), (me, ACTORCAT_EXPLOSIVE, 0, 69));
    assert_eq!((b.actor.scale.x, b.actor.shape_y_offset, b.actor.gravity, b.actor.room), (0.0, 700.0, -1.2, -1));
    // Timer 67: NA_SE_PL_TAKE_OUT_SHIELD and scale 0.01 (two frames on).
    idle(&mut w, 1);
    assert_eq!((bomb(&w, h).timer, bomb(&w, h).actor.scale.x), (68, 0.0));
    idle(&mut w, 1);
    assert_eq!((bomb(&w, h).timer, bomb(&w, h).actor.scale.x), (67, 0.01));
    // Under 63 its fuse sparks on even gameplayFrames (each lives 10 frames).
    idle(&mut w, 5);
    assert_eq!(bomb(&w, h).timer, 62);
    idle(&mut w, 1);
    assert!(effects_of(&w, EFFECT_SS_G_SPK) >= 1, "EffectSsGSpk_SpawnFuse");
    // The flash quickens at 40, 20 and 3 (flashSpeedScale 7, 3, 1, 0).
    while bomb(&w, h).timer > 40 {
        idle(&mut w, 1);
    }
    assert_eq!(bomb(&w, h).flash_speed_scale, 3);
    while bomb(&w, h).timer > 1 {
        idle(&mut w, 1);
    }
    // Timer 2 and 1: the scale 0.002 up each (and once more at 0).
    assert!((bomb(&w, h).actor.scale.x - 0.014).abs() < 1e-6, "{}", bomb(&w, h).actor.scale.x);
    assert_eq!(bomb(&w, h).flash_speed_scale, 0);
    // Timer 0: the blast (layered Bomb2 30 above Link's hand, the shockwave on the floor), the
    // light flash (250), params BOMB_EXPLOSION, ten more frames.
    idle(&mut w, 1);
    let b = bomb(&w, h);
    assert_eq!((b.actor.params, b.timer), (1, 10));
    assert_eq!((w.env_ctx.adj_light1_color, w.env_ctx.adj_ambient_color), ([250; 3], [250; 3]));
    assert_eq!((effects_of(&w, EFFECT_SS_BOMB2), effects_of(&w, EFFECT_SS_BLAST)), (1, 1));
    // EnBom_Explode: the sphere 8 a frame (rot.z 0), the flash 25 down a frame.
    idle(&mut w, 1);
    assert_eq!(bomb(&w, h).explosion_collider.elements[0].dim.world_sphere.radius, 8);
    assert_eq!(w.env_ctx.adj_light1_color, [225; 3]);
    idle(&mut w, 8);
    assert_eq!(bomb(&w, h).explosion_collider.elements[0].dim.world_sphere.radius, 72);
    // The tenth: it lets go of Link itself (child, heldActor, interactRangeActor, the carry off)
    // and is killed.
    idle(&mut w, 1);
    let p = w.player();
    assert_eq!((p.held_actor, p.actor.child, p.state1 & CARRYING), (None, None, 0));
    assert!(w.actors.actor(h).is_none_or(|b| b.killed));
    // Its blast hit Link (AT_TYPE_ALL, DMG_EXPLOSIVE 8).
    assert!(w.save.health < health, "health {} of {}", w.save.health, health);
    // And Player puts the bombs away: Player_UpperAction_CarryActor with no heldActor
    // (func_80834644, Player_DetachHeldActor: no item action).
    idle(&mut w, 2);
    assert_eq!(w.player().held_item_ap, data().unwrap().items.ap("NONE"));
}

#[test]
fn a_bomb_put_down_offers_itself_again_and_blows_on_its_own_time() {
    let Some(a) = assets() else { return };
    let mut w = field(&a);
    let (h, _) = take_out_bomb(&mut w);
    // C-Left again: Player_ActionHandler_9, standing still: put down (the bomb isn't throw-only and
    // Link is slower than 1.1: Player_CanThrowCarriedActor); on frame 4 it's let go.
    idle(&mut w, 3);
    press(&mut w, BTN_CLEFT);
    let mut let_go = false;
    for _ in 0..20 {
        idle(&mut w, 1);
        if bomb(&w, h).actor.parent.is_none() {
            let_go = true;
            break;
        }
    }
    assert!(let_go, "put down");
    assert!(w.player().held_actor.is_none());
    // On the ground with 4 or more on the fuse it offers itself again (Actor_OfferCarry: Link's
    // interactRangeActor); it blows when its fuse runs out all the same.
    let mut offered = false;
    while bomb(&w, h).actor.params == 0 {
        idle(&mut w, 1);
        offered |= w.player().interact_range_actor == Some(h);
    }
    assert!(offered, "Actor_OfferCarry");
    assert_eq!(bomb(&w, h).timer, 10);
}

#[test]
fn a_bomb_on_the_cavern_s_entrance_wall_plays_its_opening() {
    let Some(a) = assets() else { return };
    let mut w = cavern(&a, Vec3::new(-3.0, 0.0, 369.0), i16::MIN, false);
    assert!(!w.save.get_event_chk_inf(oot_actors::bg_breakwall::EVENTCHKINF_B0));
    // A bomb two frames from blowing against the entrance wall (Bg_Breakwall 0x0007 at (0, 0,
    // 20)), 100 east of the scene's origin. (Its sphere's centre is where it last drew as a bomb:
    // one spawned between frames with no fuse would blow before any draw, at the origin; the
    // game's spawners spawn in an update, and Actor_DrawAll draws it that frame.)
    let h = w.actor_spawn(ACTOR_EN_BOM, Vec3::new(100.0, 0.0, 40.0), [0; 3], 0).unwrap();
    w.actors.downcast_mut::<EnBom>(h).unwrap().timer = 2;
    for _ in 0..12 {
        idle(&mut w, 1);
        if w.save.get_event_chk_inf(oot_actors::bg_breakwall::EVENTCHKINF_B0) {
            break;
        }
    }
    // BgBreakwall_Wait: an explosive's hit, the first time: EVENTCHKINF_B0 and gDcOpeningCs.
    assert!(w.save.get_event_chk_inf(oot_actors::bg_breakwall::EVENTCHKINF_B0), "the wall's EVENTCHKINF_B0");
    assert!(w.flags.get_switch(7), "the wall's switch flag 7");
}

#[test]
fn a_blast_breaks_the_rocks_it_reaches() {
    let Some(a) = assets() else { return };
    let mut w = cavern(&a, Vec3::new(-3.0, 0.0, 369.0), i16::MIN, false);
    // The lobby's upper level: three Obj_Bombiwa 0x0015 (flag 0x15) round a Bomb Flower at
    // (850, 531, -923). Obj_Bombiwa breaks on func_80033684: an exploding explosive within
    // rot.z × 10 + 80.
    assert!(!w.flags.get_switch(0x15));
    let h = w.actor_spawn(ACTOR_EN_BOM, Vec3::new(790.0, 531.0, -923.0), [0; 3], 0).unwrap();
    w.actors.downcast_mut::<EnBom>(h).unwrap().timer = 2;
    idle(&mut w, 4);
    assert!(w.flags.get_switch(0x15), "the boulders' flag 0x15");
    // The flower 60 from the bomb, hit by its growing sphere (8 a frame; the flower's cylinder 9
    // across takes DMG_EXPLOSIVE), spawns a bomb that blows at once (EnBombf_GrowBomb: timer 0)
    // and starts to regrow (180 frames, size 0).
    let f = flower_at(&w, Vec3::new(850.0, 531.0, -923.0));
    let mut hit = false;
    for _ in 0..10 {
        idle(&mut w, 1);
        let fl = w.actors.downcast::<EnBombf>(f).unwrap();
        if fl.flower_bomb_scale == 0.0 {
            hit = true;
            break;
        }
    }
    assert!(hit, "the flower's bomb set off");
    assert!(all::<EnBombf>(&w).iter().any(|&b| w.actors.downcast::<EnBombf>(b).unwrap().actor.params >= 0), "its bomb");
}

#[test]
fn a_bomb_flower_is_tugged_at_without_the_bracelet() {
    let Some(a) = assets() else { return };
    // The flower at (-293, 20, 307) beside the cavern's spawn; Link 30 south of it facing it.
    let mut w = cavern(&a, Vec3::new(-293.0, 20.0, 337.0), i16::MIN, false);
    let f = flower_at(&w, Vec3::new(-293.0, 20.0, 307.0));
    // Its offer (Actor_OfferCarry: straight in front, getItemDirection 0x8000) is in place a frame
    // on.
    idle(&mut w, 1);
    assert_eq!(w.player().interact_range_actor, Some(f));
    press(&mut w, BTN_A);
    let mut tugged = false;
    for _ in 0..10 {
        idle(&mut w, 1);
        tugged |= w.player().action == Action::NoCarry;
    }
    // func_8083A0F4: Player_GetStrength() <= PLAYER_STR_NONE, Player_Action_80846408.
    assert!(tugged, "the tug");
    let fl = w.actors.downcast::<EnBombf>(f).unwrap();
    assert_eq!((fl.flower_bomb_scale, fl.actor.parent), (1.0, None));
    assert!(all::<EnBombf>(&w).iter().all(|&b| w.actors.downcast::<EnBombf>(b).unwrap().actor.params < 0), "no bomb grown off it");
}

#[test]
fn a_bomb_flower_lifted_hands_its_bomb_on_and_grows_another() {
    let Some(a) = assets() else { return };
    let mut w = cavern(&a, Vec3::new(-293.0, 20.0, 337.0), i16::MIN, true);
    let f = flower_at(&w, Vec3::new(-293.0, 20.0, 307.0));
    idle(&mut w, 1);
    press(&mut w, BTN_A);
    let mut body = None;
    for _ in 0..30 {
        idle(&mut w, 1);
        body = all::<EnBombf>(&w).into_iter().find(|&b| w.actors.downcast::<EnBombf>(b).unwrap().actor.params == 0);
        if body.is_some() {
            break;
        }
    }
    let body = body.expect("the flower's bomb in hand");
    // func_8002F5C4: Link holds the bomb (heldActor, interactRangeActor, his child), its parent
    // Link; the flower let go, 180 frames to regrow from 0, not a target.
    let p = w.player();
    assert_eq!((p.held_actor, p.actor.child, p.state1 & CARRYING), (Some(body), Some(body), CARRYING));
    let b = w.actors.downcast::<EnBombf>(body).unwrap();
    assert_eq!((b.actor.parent, b.actor.category, b.actor.gravity, b.actor.shape_y_offset), (w.player, ACTORCAT_EXPLOSIVE, -1.5, 1000.0));
    // Spawned in the flower's (PROP) turn, after the explosives': its first update is the next frame.
    assert_eq!(b.timer, 140);
    let fl = w.actors.downcast::<EnBombf>(f).unwrap();
    assert_eq!((fl.actor.parent, fl.timer, fl.flower_bomb_scale, fl.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED), (None, 180, 0.0, 0));
    // The regrowth: the timer from 180 to 0 (the fuse counts it: 0 on the 180th frame, which grows
    // it the first 0.05), then 0.05 a frame to 1 (f32 sums), a target again.
    let mut s = 0.0f32;
    let mut grow = 0;
    while s < 1.0 {
        s += 0.05;
        grow += 1;
    }
    idle(&mut w, 180 + grow - 2);
    assert!(w.actors.downcast::<EnBombf>(f).unwrap().flower_bomb_scale < 1.0);
    idle(&mut w, 1);
    let fl = w.actors.downcast::<EnBombf>(f).unwrap();
    assert_eq!(fl.flower_bomb_scale, s);
    assert_ne!(fl.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
}

/// The Bomb Flower placed at `pos`.
fn flower_at(w: &PlayState, pos: Vec3) -> ActorHandle {
    all::<EnBombf>(w).into_iter().find(|&h| w.actors.actor(h).unwrap().home_pos.distance(pos) < 1.0).expect("the Bomb Flower")
}

#[test]
fn the_fuse_spark_follows_its_bomb() {
    let Some(a) = assets() else { return };
    // A bomb of the flowers' (object_bombf is loaded in the cavern's lobby).
    let mut w = cavern(&a, Vec3::new(-3.0, 0.0, 369.0), i16::MIN, false);
    let h = w.actor_spawn(ACTOR_EN_BOMBF, Vec3::new(-3.0, 0.0, 250.0), [0; 3], 0).unwrap();
    // EnBombf_Init for a bomb: the fuse 140; under 127 the spark, on even frames, 25 up.
    while w.actors.downcast::<EnBombf>(h).unwrap().timer >= 127 {
        idle(&mut w, 1);
    }
    idle(&mut w, 2);
    let pos = w.actors.actor(h).unwrap().world_pos;
    let sparks: Vec<_> = w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == EFFECT_SS_G_SPK).collect();
    assert!(!sparks.is_empty());
    // EffectSsGSpk_Update: at the bomb plus its offset (25 up, drifting across by up to 1.5 a
    // frame).
    for s in sparks {
        assert!((s.pos.y - (pos.y + 25.0)).abs() < 1e-3, "{:?} {:?}", s.pos, pos);
        assert!((s.pos.x - pos.x).abs() < 30.0 && (s.pos.z - pos.z).abs() < 30.0);
    }
}



#[test]
fn exit_the_cavern_s_entrance_opened_by_a_bomb_flower_and_a_bomb_from_the_bag() {
    use oot_actors::playthrough::{Playthrough, Route, Step};
    let Some(a) = assets() else { return };
    let route = Route::Bombs;
    let e = a.scenes.entrance_index(route.entrance()).unwrap();
    let start = route.save(e);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), start.clone()).unwrap();
    route.debug_start(&mut w);
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
    }
    assert!(run.failure.is_none(), "{:?}", run.failure);
    eprintln!("steps {:?}", run.steps);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    use Step::*;
    assert_eq!(steps, vec![FlowerLifted, PutDown, DcOpening, BombOut, PutDown, FlowerSetOff]);
    // The entrance wall's flag 7 and EVENTCHKINF_B0 (BgBreakwall_Wait's first break); one bomb
    // spent of the bag's 20; no hit taken (both put down and left beyond the blast).
    assert!(w.flags.get_switch(7) && w.save.get_event_chk_inf(oot_actors::bg_breakwall::EVENTCHKINF_B0));
    assert_eq!(w.save.inventory.ammo[oot_game::item::slot(ITEM_BOMB)], 19);
    assert_eq!(w.save.health, start.health);
}
