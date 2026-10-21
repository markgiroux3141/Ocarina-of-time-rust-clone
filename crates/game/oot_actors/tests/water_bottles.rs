//! GAME-10 milestone 1: Player's water and bottles. The dive's depth by the scale
//! (`D_80854784`), swimming in from an entrance in deep water (`Player_Action_8084D7C4`), the
//! currents (`sWaterConveyorSpeeds`), Zora's Domain's finite water box
//! (`BgCheck_GetWaterSurfaceAllHack`), and the bottles' own uses: the swing
//! (`Player_Action_SwingBottle`), a drink (`Player_Action_8084EAC0`), a fairy let out
//! (`Player_Action_8084EED8`).
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, BTN_CLEFT, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Enters `entrance` on `preset`, then `tweak`, at 10:00.
fn enter(a: &Arc<GameAssets>, entrance: &str, preset: &str, tweak: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index(entrance).unwrap();
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset(preset).unwrap();
    tweak(&mut save);
    oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap()
}

fn step(w: &mut PlayState, prev: &mut PadState, pad: PadState) {
    w.tick_with(scripted_input(*prev, pad));
    *prev = pad;
}

fn idle(w: &mut PlayState, n: u32) {
    let mut prev = PadState::default();
    for _ in 0..n {
        step(w, &mut prev, PadState::default());
    }
}

fn action(w: &PlayState) -> &'static str {
    w.player().action.decomp_name()
}

/// `ITEM_SCALE_SILVER` (`item.h`: 0x53).
const ITEM_SCALE_SILVER: u8 = 0x53;

/// Lake Hylia over Ruto's letter (`Item_Etcetera` 0x0001 at (-916, -1580, 4297)): Link dropped at
/// the surface (`WATER_LEVEL_RAISED`, -1313), then A held to dive. Returns the depth the dive turns
/// back at (the frame `actionVar1` leaves 0: the action reads that frame's `depthInWater`, the
/// collision having run before it), and the depth the frame before.
fn dive_turn(a: &Arc<GameAssets>, scale: bool) -> (f32, f32) {
    // EVENTCHKINF_B1: the lake's intro seen; EVENTCHKINF_31: Ruto's letter already taken
    // (Item_Etcetera killed at init), or the dive picks it up 217 down and turns there.
    let mut w = enter(a, "ENTR_LAKE_HYLIA_0", "fairy-magic", |s| {
        s.set_event_chk_inf(0xB1);
        s.set_event_chk_inf(0x31);
        if scale {
            oot_game::item::item_give(s, None, ITEM_SCALE_SILVER);
        }
    });
    idle(&mut w, 20);
    w.place_player(Vec3::new(-930.0, -1313.0, 4297.0), 0);
    idle(&mut w, 60);
    assert!(w.player().state1 & oot_actors::player::STATE1_27 != 0, "swimming: {}", action(&w));
    let mut prev = PadState::default();
    let mut a_pad = PadState::default();
    a_pad.button |= BTN_A;
    let mut depths = Vec::new();
    for _ in 0..240 {
        let before = (w.player().actor.y_dist_to_water, w.player().action_var1, action(&w));
        step(&mut w, &mut prev, a_pad);
        let p = w.player();
        if before.2 == "Player_Action_8084DC48" && before.1 == 0 && p.action_var1 == 1 {
            eprintln!("turned at {} (scale {scale}), {} before", p.actor.y_dist_to_water, before.0);
            depths.push((p.actor.y_dist_to_water, before.0));
        }
    }
    *depths.first().expect("the dive turned back")
}

#[test]
fn a_dive_goes_as_deep_as_the_scale_lets_it() {
    let Some(a) = assets() else { return };
    // Player_Action_8084DC48: down at -2 (func_8084DBC4, eased by func_8084AEEC) while A is held
    // and depthInWater < D_80854784[CUR_UPG_VALUE(UPG_SCALE)]: 120 without a scale, 240 with the
    // silver one; the frame it reaches it he turns back up.
    let (none, before) = dive_turn(&a, false);
    assert!(none >= 120.0 && before < 120.0, "{none} {before}");
    let (silver, before) = dive_turn(&a, true);
    assert!(silver >= 240.0 && before < 240.0, "{silver} {before}");
}

#[test]
fn the_eye_under_water_distorts_the_view() {
    let Some(a) = assets() else { return };
    // Lake Hylia, Link treading at the surface. A dive keeps the eye above it
    // (CAM_SET_PIVOT_WATER_SURFACE), so the eye is put under the surface for one frame:
    // Camera_UpdateWater reads it at the start of Camera_Update.
    let mut w = enter(&a, "ENTR_LAKE_HYLIA_0", "fairy-magic", |s| s.set_event_chk_inf(0xB1));
    idle(&mut w, 20);
    w.place_player(Vec3::new(-930.0, -1313.0, 4297.0), 0);
    idle(&mut w, 60);
    let (config_before, next_before) = (w.env_ctx.light_config, w.env_ctx.change_light_next_config);
    let eye = w.game_camera.eye;
    w.game_camera.eye = Vec3::new(eye.x, -1400.0, eye.z);
    idle(&mut w, 1);
    let c = &w.game_camera;
    // Under: CAM_STATE_CAMERA_IN_WATER, the water box's lights (Environment_EnableUnderwaterLights
    // by time: the config, the one before kept), the strong distortion's 80 frames (one counted),
    // the water's quake; Camera_UpdateDistortion then sets the view's scale and speed 0.2.
    assert!(c.state_flags & (1 << 8) != 0);
    assert_eq!(c.water_distortion_timer, 79);
    assert!(c.distortion_flags & oot_game::camera::DISTORTION_UNDERWATER_STRONG != 0);
    // Quake_Request's index: the slot with a random tag in its upper bits (any sign), 0 for none.
    assert!(c.water_quake_index != 0 && c.water_quake_index != -1);
    assert_eq!(w.env_statics.light_config_after_underwater as usize, next_before);
    eprintln!("lights {config_before} to {}, distortion {:?}", w.env_ctx.light_config, w.view_distortion);
    assert!((w.view_distortion.speed - 0.2).abs() < 1e-6);
    // The eye back above: the lights back, no distortion (View_ClearDistortion once).
    idle(&mut w, 2);
    let c = &w.game_camera;
    assert!(c.state_flags & (1 << 8) == 0);
    assert_eq!((c.distortion_flags, c.water_distortion_timer), (0, 0));
    assert_eq!(w.env_ctx.light_config, next_before);
}

#[test]
fn an_entrance_in_deep_water_swims_in() {
    let Some(a) = assets() else { return };
    // ENTR_LAKE_HYLIA_7 (from Zora's Domain's tunnel): its spawn is 231 below the surface, so
    // Player_SetStartingMovement swims him in (Player_Action_8084D7C4: speed 2, 20 frames).
    let mut w = enter(&a, "ENTR_LAKE_HYLIA_7", "fairy-magic", |s| s.set_event_chk_inf(0xB1));
    let mut seen = Vec::new();
    let mut prev = PadState::default();
    for _ in 0..60 {
        seen.push(action(&w));
        step(&mut w, &mut prev, PadState::default());
    }
    // DECR(av2.actionVar2) from 20: 20 frames, then treading (func_80838F18).
    let swim_in = seen.iter().take_while(|&&s| s == "Player_Action_8084D7C4").count();
    assert_eq!(swim_in, 20, "{seen:?}");
    assert_eq!(seen[20], "Player_Action_8084D610");
}

#[test]
fn milk_is_drunk_by_halves() {
    let Some(a) = assets() else { return };
    // fairy-magic's bottle of milk on C-Left, Link three hearts down (health 0x10 of 0x40).
    let mut w = enter(&a, "ENTR_KAKARIKO_VILLAGE_14", "fairy-magic", |s| {
        s.equip_item_on_c_left(oot_game::item::ITEM_BOTTLE_MILK_FULL);
        s.health = 0x10;
    });
    idle(&mut w, 40);
    let mut prev = PadState::default();
    let mut c = PadState::default();
    c.button |= BTN_CLEFT;
    step(&mut w, &mut prev, c);
    let mut actions = Vec::new();
    for _ in 0..400 {
        step(&mut w, &mut prev, PadState::default());
        let s = action(&w);
        if actions.last() != Some(&s) {
            actions.push(s);
        }
    }
    eprintln!("{actions:?} health {:#x}", w.save.health);
    assert!(actions.contains(&"Player_Action_8084EAC0"), "{actions:?}");
    // D_808549FC's milk: healthAccumulator 0x50, up to the five hearts' capacity;
    // Inventory_UpdateBottleItem's half-milk case.
    assert_eq!(w.save.health, (0x10 + 0x50).min(w.save.health_capacity));
    let slot = w.save.equips.c_button_slots[0] as usize;
    assert_eq!(w.save.inventory.items[slot], oot_game::item::ITEM_BOTTLE_MILK_HALF);
    assert_eq!(w.save.equips.button_items[1], oot_game::item::ITEM_BOTTLE_MILK_HALF);
}

#[test]
fn a_bottled_fairy_is_let_out_and_heals() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_KAKARIKO_VILLAGE_14", "fairy-magic", |s| {
        let slot = oot_game::item::slot(oot_game::item::ITEM_BOTTLE_MILK_FULL);
        s.inventory.items[slot] = oot_game::item::ITEM_BOTTLE_FAIRY;
        s.equip_item_on_c_left(oot_game::item::ITEM_BOTTLE_FAIRY);
        s.health = 0x10;
    });
    idle(&mut w, 40);
    let fairies = |w: &PlayState| w.actors.all().into_iter().filter(|&h| w.actors.actor(h).is_some_and(|x| x.id == oot_actors::en_elf::ACTOR_EN_ELF && x.params == oot_actors::en_elf::FAIRY_REVIVE_BOTTLE)).count();
    assert_eq!(fairies(&w), 0);
    let mut prev = PadState::default();
    let mut c = PadState::default();
    c.button |= BTN_CLEFT;
    step(&mut w, &mut prev, c);
    let mut spawned_at = None;
    for f in 0..300 {
        step(&mut w, &mut prev, PadState::default());
        if spawned_at.is_none() && fairies(&w) > 0 {
            spawned_at = Some(f);
        }
    }
    eprintln!("fairy at {spawned_at:?}, health {:#x}", w.save.health);
    assert!(spawned_at.is_some());
    // On frame 47, healthAccumulator 0x140: full.
    assert_eq!(w.save.health, w.save.health_capacity);
    let slot = w.save.equips.c_button_slots[0] as usize;
    assert_eq!(w.save.inventory.items[slot], oot_game::item::ITEM_BOTTLE_EMPTY);
}

#[test]
fn b_swings_an_empty_bottle() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_KAKARIKO_VILLAGE_14", "fairy-magic", |s| {
        let slot = oot_game::item::slot(oot_game::item::ITEM_BOTTLE_MILK_FULL);
        s.inventory.items[slot] = oot_game::item::ITEM_BOTTLE_EMPTY;
        s.equip_item_on_c_left(oot_game::item::ITEM_BOTTLE_EMPTY);
    });
    idle(&mut w, 40);
    let mut prev = PadState::default();
    let mut c = PadState::default();
    c.button |= BTN_CLEFT;
    // C-Left takes the bottle out (Player_UseItem: a new item in hand).
    step(&mut w, &mut prev, c);
    for _ in 0..30 {
        step(&mut w, &mut prev, PadState::default());
    }
    assert_eq!(w.player().held_item_id, oot_game::item::ITEM_BOTTLE_EMPTY);
    // The bottle's button again uses the item in hand (sUseHeldItem): func_8083C6B8 (from
    // Player_ActionHandler_7): on land, link_bottle_bug_miss.
    step(&mut w, &mut prev, c);
    assert_eq!(action(&w), "Player_Action_SwingBottle");
    // PLAYER_STATE1_SWINGING_BOTTLE is cleared before each frame's action and set by the swing's
    // own (from the next frame, its first 8 frames).
    assert!(w.player().state1 & oot_actors::player::STATE1_1 == 0);
    step(&mut w, &mut prev, PadState::default());
    assert!(w.player().state1 & oot_actors::player::STATE1_1 != 0);
    let mut frames = 0;
    while action(&w) == "Player_Action_SwingBottle" && frames < 200 {
        step(&mut w, &mut prev, PadState::default());
        frames += 1;
    }
    eprintln!("swing over after {frames}: {}", action(&w));
    assert!(frames < 200);
}

#[test]
fn zoras_river_s_current_carries_a_swimmer() {
    let Some(a) = assets() else { return };
    use oot_game::surface::SurfaceType;
    let mut w = enter(&a, "ENTR_ZORAS_RIVER_0", "fairy-magic", |_| {});
    idle(&mut w, 10);
    // A floor polygon under the river's water with a current (SurfaceType_GetConveyorSpeed, not
    // a floor conveyor), at least 60 under a water box: the deepest such.
    let col = &w.col;
    let mut best: Option<(Vec3, u32, u32, f32)> = None;
    for i in 0..col.header.polys.len() {
        let id = eng_collision::bgcheck::PolyId::scene(i as u16);
        let p = &col.header.polys[i];
        if p.normal[1] < 16000 || col.conveyor_speed(id) == 0 || col.is_floor_conveyor(id) {
            continue;
        }
        let c = (col.header.vertex(p.a()) + col.header.vertex(p.b()) + col.header.vertex(p.c())) / 3.0;
        if let Some(y) = col.water_surface(c.x, c.z, col.water_room) {
            let depth = y - c.y;
            if depth > 60.0 && best.is_none_or(|b| depth > b.3) {
                best = Some((c, col.conveyor_speed(id), col.conveyor_direction(id), depth));
            }
        }
    }
    let (c, speed, dir, depth) = best.expect("a current under the river's water");
    eprintln!("current at {c:?}: speed {speed}, direction {dir}, depth {depth}");
    w.place_player(Vec3::new(c.x, c.y + depth - 20.0, c.z), 0);
    idle(&mut w, 90);
    let p = w.player();
    // Swimming (PLAYER_STATE1_27) in it: pushedSpeed eases to sWaterConveyorSpeeds[speed - 1]
    // (2, 4, 7) by a tenth a frame, pushedYaw to CONVEYOR_DIRECTION_TO_BINANG(direction).
    let target = [2.0f32, 4.0, 7.0][speed as usize - 1];
    eprintln!("{} pushed {} at {:#x}, {:?}", action(&w), p.pushed_speed, p.pushed_yaw, p.actor.world_pos);
    if p.s.conveyor_speed != 0 {
        assert!(p.state1 & oot_actors::player::STATE1_27 != 0);
        assert!((p.pushed_speed - target).abs() < 0.01, "{} vs {target}", p.pushed_speed);
    } else {
        // Carried off the current's floor: easing back to 0 by 0.5 a frame.
        assert!(p.pushed_speed <= target);
    }
    assert!(p.actor.world_pos.distance(c) > 50.0, "carried");
}

#[test]
fn zoras_domain_s_pool_is_a_finite_water_box() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_ZORAS_DOMAIN_0", "fairy-magic", |s| s.set_event_chk_inf(0xA4));
    idle(&mut w, 10);
    // BgCheck_GetWaterSurfaceAllHack: inside sZorasDomainWaterBox's bounds, its surface 877.
    let col = &w.col;
    assert_eq!(col.water_surface_all_hack(0.0, 850.0, -1200.0, col.water_room), Some(877.0));
    // Below its floor (y 777), the scene's own boxes.
    assert_ne!(col.water_surface_all_hack(0.0, 700.0, -1200.0, col.water_room), Some(877.0));
}

#[test]
fn the_bottle_run_drinks_the_milk_catches_a_fish_and_lets_it_go() {
    use oot_actors::playthrough::{Playthrough, Route, Step};
    let Some(a) = assets() else { return };
    let route = Route::Bottle;
    let e = a.scenes.entrance_index(route.entrance()).unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), route.save(e)).unwrap();
    route.debug_start(&mut w);
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    let mut fish_dropped = 0;
    while let Some(p) = run.next(&w) {
        step(&mut w, &mut prev, p);
        if w.actors.all().into_iter().filter_map(|h| w.actors.actor(h)).any(|x| x.id == 0x21 && x.params == 0) {
            fish_dropped += 1;
        }
    }
    eprintln!("steps {:?}", run.steps);
    assert!(run.failure.is_none(), "{:?}", run.failure);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::MilkDrunk, Step::MilkDrunk, Step::FishCaught, Step::FishLetOut]);
    // The bottle: full milk, half (Inventory_UpdateBottleItem), empty, a fish (sBottleCatchInfo's
    // ITEM_BOTTLE_FISH, text 0x47), empty again; the fish let out on frame 76 as
    // EN_FISH_TYPE_DROPPED (D_80854A28).
    assert_eq!(w.save.equips.button_items[1], oot_game::item::ITEM_BOTTLE_EMPTY);
    assert!(run.texts.contains(&0x47), "{:x?}", run.texts);
    assert!(fish_dropped > 0);
}
