//! GAME-11 milestone 1: adult Link and the time machine. The age switch
//! (`Inventory_SwapAgeEquipment`, `linkAgeOnLoad`), `Bg_Toki_Swd`, `Door_Toki`, Player's time
//! travel (the adult's pedestal branch, start mode 1 and `Player_Action_TimeTravelEnd`, cutscene
//! modes 18 and 30) and the adult's draw details; and the two runs on the `master-sword` debug
//! starts: the pull (`Route::MasterSwordPull`) and the sword put back (`Route::MasterSwordBack`).
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use oot_actors::PlayExt;
use oot_actors::bg_toki_swd::{Action as SwdAction, BgTokiSwd};
use oot_actors::player::{Action as PA, LeftHandDLists};
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::item::*;
use oot_game::play::{DrawOut, PlayState, ViewInfo, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::player_lib::{Age, Loadout};
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// `ITEM_SWORD_CS`'s and the swords' item actions.
fn ap(name: &str) -> i32 {
    data().unwrap().items.ap(name)
}

fn find_sword(w: &PlayState) -> Option<&BgTokiSwd> {
    w.actors.all().into_iter().find_map(|h| w.actors.downcast::<BgTokiSwd>(h))
}

fn sword(w: &PlayState) -> &BgTokiSwd {
    find_sword(w).expect("Bg_Toki_Swd")
}

/// The frames a sound was asked for.
fn sfx_frames(w: &PlayState, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, _)| f).collect()
}

/// What a run saw each frame.
#[derive(Default, Debug)]
struct Seen {
    /// The frame `Bg_Toki_Swd` was taken (`func_808BB0AC` to `func_808BB128`), with Player's
    /// `leftHandDLists` and whether the sword was drawn on it.
    taken: Option<(u32, Option<LeftHandDLists>, bool)>,
    /// The highest `roomCtx.drawParams[0]` (the temple dimmed).
    darkest: i16,
    /// The scene change's frame.
    new_scene: Option<u32>,
}

fn run(a: &Arc<GameAssets>, route: Route) -> (PlayState, Playthrough, Seen) {
    let e = a.scenes.entrance_index(route.entrance()).unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), route.save(e)).unwrap();
    w.audio.log = Some(Default::default());
    route.debug_start(&mut w);
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    let mut seen = Seen::default();
    let changes = w.scene_changes;
    while let Some(p) = run.next(&w) {
        let before = (w.scene_changes == changes).then(|| find_sword(&w).map(|s| s.action)).flatten();
        w.tick_with(scripted_input(prev, p));
        prev = p;
        if w.scene_changes != changes {
            seen.new_scene.get_or_insert(w.gameplay_frames);
            continue;
        }
        let Some(s) = find_sword(&w) else { continue };
        if before == Some(SwdAction::Func808BB0AC) && s.action == SwdAction::Func808BB128 {
            seen.taken = Some((w.gameplay_frames, w.player().left_hand_dlists, s.drawn));
        }
        seen.darkest = seen.darkest.max(w.scene.as_ref().unwrap().draw.room_draw_params[0]);
    }
    eprintln!("steps {:?}", run.steps);
    eprintln!("seen {seen:?}");
    assert!(run.failure.is_none(), "{:?}", run.failure);
    (w, run, seen)
}

/// The run's save is the preset's in its flags, inventory and equipment, with `extra` applied to
/// the run's (what the run doesn't do that the preset has).
fn ends_as_preset(w: &mut PlayState, preset: &str, extra: impl Fn(&mut SaveContext)) {
    w.save_scene_flags();
    let mut s = w.save.clone();
    extra(&mut s);
    let mut p = SaveContext::new(s.entrance_index, false, s.day_time);
    p.apply_preset(preset).unwrap();
    assert_eq!(p.adult, s.adult);
    assert_eq!((&p.event_chk_inf, &p.inf_table, &p.item_get_inf, &p.event_inf), (&s.event_chk_inf, &s.inf_table, &s.item_get_inf, &s.event_inf));
    assert_eq!((&p.inventory.items, &p.inventory.ammo, p.inventory.upgrades, p.inventory.quest_items, p.inventory.equipment), (&s.inventory.items, &s.inventory.ammo, s.inventory.upgrades, s.inventory.quest_items, s.inventory.equipment));
    assert_eq!((p.equips, p.child_equips, p.adult_equips), (s.equips, s.child_equips, s.adult_equips));
    for (i, (a, b)) in p.scene_flags.iter().zip(s.scene_flags.iter()).enumerate() {
        assert_eq!(format!("{a:?}"), format!("{b:?}"), "scene {i:#x}'s flags");
    }
}

/// `equipment`'s nibbles: sword, shield, tunic, boots (`EQUIP_VALUE_*`).
fn worn(sword: u16, shield: u16, tunic: u16, boots: u16) -> u16 {
    sword | shield << 4 | tunic << 8 | boots << 12
}

#[test]
fn the_age_swap_gives_the_first_adult_his_equips_and_keeps_both_ages() {
    // master-sword and the pull's Item_Give(ITEM_SWORD_MASTER): a child with the Master Sword on B
    // (Item_Give puts it there and wears it).
    let mut s = SaveContext::new(0, false, 0);
    s.apply_preset("master-sword").unwrap();
    item_give(&mut s, None, ITEM_SWORD_MASTER);
    let child = s.equips;
    assert_eq!(child.button_items[0], ITEM_SWORD_MASTER);
    assert_eq!(child.equipment, worn(EQUIP_VALUE_SWORD_MASTER, EQUIP_VALUE_SHIELD_HYLIAN, EQUIP_VALUE_TUNIC_KOKIRI, EQUIP_VALUE_BOOTS_KOKIRI));
    // ocarina-of-time has no Deku Nuts.
    assert_eq!(s.inventory.items[SLOT_DEKU_NUT], ITEM_NONE);
    assert!(s.adult_equips.button_items[0] == ITEM_NONE);

    // The child's swap (LINK_AGE_IN_YEARS == YEARS_CHILD): childEquips are his buttons with the
    // Kokiri Sword for B, and his equipment; adultEquips' B is ITEM_NONE, so the first adult's: the
    // Master Sword on B, no nuts (C-Left ITEM_NONE, its slot SLOT_NONE's 0xFF), bombs on C-Down
    // (SLOT_BOMB), the ocarina's slot on C-Right (the Ocarina of Time), the Master Sword, Hylian
    // Shield, Kokiri Tunic and Kokiri Boots.
    s.inventory_swap_age_equipment();
    assert_eq!(s.child_equips.button_items, [ITEM_SWORD_KOKIRI, child.button_items[1], child.button_items[2], child.button_items[3]]);
    assert_eq!((s.child_equips.c_button_slots, s.child_equips.equipment), (child.c_button_slots, child.equipment));
    assert_eq!(s.equips.button_items, [ITEM_SWORD_MASTER, ITEM_NONE, ITEM_BOMB, ITEM_OCARINA_OF_TIME]);
    assert_eq!(s.equips.c_button_slots, [0xFF, SLOT_BOMB as u8, SLOT_OCARINA as u8]);
    assert_eq!(s.equips.equipment, worn(EQUIP_VALUE_SWORD_MASTER, EQUIP_VALUE_SHIELD_HYLIAN, EQUIP_VALUE_TUNIC_KOKIRI, EQUIP_VALUE_BOOTS_KOKIRI));
    s.adult = true;
    let adult = s.equips;

    // The adult's swap: adultEquips his; childEquips' B isn't ITEM_NONE, so the child's back, the
    // Kokiri Sword worn (the sword's nibble replaced: the child's had the Master Sword's).
    s.inventory_swap_age_equipment();
    assert_eq!(s.adult_equips, adult);
    assert_eq!(s.equips.button_items, [ITEM_SWORD_KOKIRI, child.button_items[1], child.button_items[2], child.button_items[3]]);
    assert_eq!(s.equips.c_button_slots, child.c_button_slots);
    assert_eq!(s.equips.equipment, worn(EQUIP_VALUE_SWORD_KOKIRI, EQUIP_VALUE_SHIELD_HYLIAN, EQUIP_VALUE_TUNIC_KOKIRI, EQUIP_VALUE_BOOTS_KOKIRI));
    s.adult = false;

    // The child again: the adult's own (no first time any more).
    s.inventory_swap_age_equipment();
    assert_eq!(s.equips, adult);
}

#[test]
fn the_first_adult_gets_deku_nuts_on_c_left_if_he_has_them() {
    let mut s = SaveContext::new(0, false, 0);
    s.apply_preset("master-sword").unwrap();
    item_give(&mut s, None, ITEM_DEKU_NUTS_5);
    s.inventory_swap_age_equipment();
    assert_eq!((s.equips.button_items[1], s.equips.c_button_slots[0]), (ITEM_DEKU_NUT, SLOT_DEKU_NUT as u8));
}

#[test]
fn a_bottle_on_a_button_shows_its_slot_and_a_lost_shield_comes_off() {
    let mut s = SaveContext::new(0, false, 0);
    s.apply_preset("master-sword-adult").unwrap();
    // The adult's C-Down holds an empty bottle from SLOT_BOTTLE_1, which since holds a Poe; and he
    // wore the Mirror Shield, which he no longer owns.
    s.equips.button_items[2] = ITEM_BOTTLE_EMPTY;
    s.equips.c_button_slots[1] = SLOT_BOTTLE_1 as u8;
    s.equips.equipment = worn(EQUIP_VALUE_SWORD_MASTER, EQUIP_VALUE_SHIELD_MIRROR, EQUIP_VALUE_TUNIC_KOKIRI, EQUIP_VALUE_BOOTS_KOKIRI);
    s.inventory_swap_age_equipment();
    s.adult = false;
    s.inventory.items[SLOT_BOTTLE_1] = ITEM_BOTTLE_POE;
    s.inventory_swap_age_equipment();
    // Register_Item_Pt: the button re-reads its slot. The shield's nibble is cleared as the Mirror
    // Shield isn't owned (CHECK_OWNED_EQUIP_ALT(EQUIP_TYPE_SHIELD, 3 - 1)).
    assert_eq!(s.equips.button_items[2], ITEM_BOTTLE_POE);
    assert_eq!(s.cur_equip_value(EQUIP_TYPE_SHIELD), 0);
    assert_eq!(s.cur_equip_value(EQUIP_TYPE_SWORD), EQUIP_VALUE_SWORD_MASTER);
}

#[test]
fn the_pull_makes_link_an_adult_on_the_pedestal() {
    let Some(a) = assets() else { return };
    let (mut w, run, seen) = run(&a, Route::MasterSwordPull);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::SwordPulled, Step::TimeTravelled]);
    // func_80851A50 on the child's frame 87: the sword taken (no longer drawn), his left hand
    // holding it (gPlayerLeftHandBgsDLs), NA_SE_IT_SWORD_STICK_STN and the cry.
    let (taken, left, drawn) = seen.taken.expect("the sword taken in the cutscene");
    assert_eq!((left, drawn), (Some(LeftHandDLists::Bgs), false));
    assert_eq!(sfx_frames(&w, oot_game::audio::sfx::NA_SE_IT_SWORD_STICK_STN), vec![taken]);
    // Cutscene flag 1 (frame 110): the temple dimmed to 0xFF by Bg_Toki_Swd's 5 a frame.
    assert_eq!(seen.darkest, 0xFF);
    // Terminator 8 with EVENTCHKINF_OBTAINED_MASTER_SWORD: an adult at ENTR_TEMPLE_OF_TIME_2.
    assert!(w.save.adult);
    assert_eq!(w.save.entrance_index, a.scenes.entrance_index("ENTR_TEMPLE_OF_TIME_2").unwrap());
    assert_eq!(w.link_age_on_load, oot_game::save::LINK_AGE_ADULT);
    let p = w.player();
    assert!(p.adult);
    // Player_StartMode_TimeTravel put the Master Sword in his hand; he stands after
    // Player_Action_TimeTravelEnd.
    assert_eq!((p.action, p.held_item_ap, p.held_item_id), (PA::StandingStill, ap("SWORD_MASTER"), ITEM_SWORD_MASTER));
    // The adult's sword isn't in the pedestal (BgTokiSwd_Init: draw NULL for an adult).
    assert!(!sword(&w).drawn);
    ends_as_preset(&mut w, "master-sword-adult", |_| {});
}

#[test]
fn the_sword_put_back_makes_link_a_child_again() {
    let Some(a) = assets() else { return };
    let (mut w, run, seen) = run(&a, Route::MasterSwordBack);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::SwordPutBack, Step::TimeTravelled]);
    // func_80851A50 on the adult's frame 70: the sword let go of (drawn in the pedestal again), his
    // left hand closed (gPlayerLeftHandClosedDLs), NA_SE_IT_SWORD_PUTAWAY_STN.
    let (taken, left, drawn) = seen.taken.expect("the sword put back in the cutscene");
    assert_eq!((left, drawn), (Some(LeftHandDLists::Closed), true));
    assert!(sfx_frames(&w, oot_game::audio::sfx::NA_SE_IT_SWORD_PUTAWAY_STN).contains(&taken));
    assert_eq!(seen.darkest, 0xFF);
    // Terminator 8 again: a child at ENTR_TEMPLE_OF_TIME_2, the child's equips back, the adult's
    // kept.
    assert!(!w.save.adult);
    assert_eq!(w.save.equips.button_items[0], ITEM_SWORD_KOKIRI);
    assert_eq!(w.save.cur_equip_value(EQUIP_TYPE_SWORD), EQUIP_VALUE_SWORD_KOKIRI);
    assert_eq!(w.save.adult_equips.button_items, [ITEM_SWORD_MASTER, ITEM_NONE, ITEM_BOMB, ITEM_OCARINA_OF_TIME]);
    let mut start = SaveContext::new(0, false, 0);
    start.apply_preset("master-sword-adult").unwrap();
    assert_eq!(w.save.equips.button_items, start.child_equips.button_items);
    let p = w.player();
    // The child's arrival: no sword in hand (Player_PutSwordInHand is the adult's), standing.
    assert!(!p.adult);
    assert_eq!((p.action, p.held_item_ap), (PA::StandingStill, ap("NONE")));
    // The child sees the sword in the pedestal (drawn for a child).
    assert!(sword(&w).drawn);
    let _ = &mut w;
}

#[test]
fn the_time_travel_arrival_waits_twenty_frames_then_plays_its_animation() {
    let Some(a) = assets() else { return };
    let e = a.scenes.entrance_index("ENTR_TEMPLE_OF_TIME_2").unwrap();
    let mut save = SaveContext::new(e, false, 0);
    save.apply_preset("master-sword-adult").unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap();
    w.audio.log = Some(Default::default());
    let d = data().unwrap();
    // Player_StartMode_TimeTravel: on the pedestal (sPedestalPos (-1, 69, 20)) facing -z, held
    // (PLAYER_STATE1_29), the adult's timeTravelEndAnim frozen on frame 0, the Master Sword in hand.
    let p = w.player();
    assert_eq!(p.action, PA::TimeTravelEnd);
    assert_eq!((p.actor.shape_rot.y, p.current_yaw), (i16::MIN, i16::MIN));
    assert_ne!(p.state1 & oot_actors::player::STATE1_29, 0);
    assert_eq!(p.skel.animation, d.ages[0].climb.time_travel_end_anim);
    assert_eq!((p.skel.cur_frame, p.skel.end_frame), (0.0, 0.0));
    assert_eq!((p.held_item_ap, p.action_var2), (ap("SWORD_MASTER"), 20));
    // Each update's LinkAnimation_Update reports the frozen animation done, so animDelayTimer
    // counts down; at 0 (the 20th update) the end frame is set back to the animation's last, and
    // it plays.
    let mut updates = 0;
    while w.player().skel.end_frame == 0.0 {
        let before = w.player().action_var2;
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
        if w.player().action_var2 != before || w.player().skel.end_frame != 0.0 {
            updates += 1;
        }
        assert!(updates <= 20 && w.gameplay_frames < 200, "the animation never started");
    }
    assert_eq!(updates, 20);
    let p = w.player();
    assert_eq!(p.skel.end_frame, p.skel.anim_length - 1.0);
    // Then to its end and standing (func_8083C0E8); the cry on frame 158 (NA_SE_VO_LI_SWORD_N)
    // (below) and the swing (D_808549F0: NA_SE_IT_MASTER_SWORD_SWING on frame 60).
    for _ in 0..600 {
        if w.player().action != PA::TimeTravelEnd {
            break;
        }
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
    assert_eq!(w.player().action, PA::StandingStill);
    // (The cry's frame 158 is past link_demo_return_to_past's 89: never heard, in the C too.)
    assert_eq!(w.player().skel.anim_length, 89.0);
    assert_eq!(sfx_frames(&w, oot_game::audio::sfx::NA_SE_VO_LI_SWORD_N).len(), 0);
    assert_eq!(sfx_frames(&w, 0x184E).len(), 1);
}

#[test]
fn cutscene_mode_30_draws_the_sword_on_frame_6() {
    let Some(a) = assets() else { return };
    // The adult on the pedestal's step (ENTR_TEMPLE_OF_TIME_4, start mode 13): nothing in hand.
    let e = a.scenes.entrance_index("ENTR_TEMPLE_OF_TIME_4").unwrap();
    let mut save = SaveContext::new(e, false, 0);
    save.apply_preset("master-sword-adult").unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap();
    w.audio.log = Some(Default::default());
    for _ in 0..60 {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
    assert_eq!(w.player().held_item_ap, ap("NONE"));
    // PLAYER_CSACTION_30: link_normal_normal2fighter_free (D_80854B18), then func_80851D80: the
    // sword in hand on the animation's frame 6 (Player_PutSwordInHand, silently), and D_808551B8's
    // NA_SE_IT_SWORD_PICKOUT on frame 12.
    assert!(w.player_set_cs_action(None, 30));
    let mut drawn_on = None;
    for _ in 0..40 {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
        let p = w.player();
        if drawn_on.is_none() && p.held_item_ap == ap("SWORD_MASTER") {
            drawn_on = Some(p.skel.cur_frame);
        }
    }
    let f = drawn_on.expect("the sword never drawn");
    assert!((6.0..7.0).contains(&f), "drawn on frame {f}");
    assert_eq!(sfx_frames(&w, oot_game::audio::sfx::NA_SE_IT_SWORD_PICKOUT).len(), 1);
}

#[test]
fn door_toki_blocks_the_doorway_until_the_door_is_open() {
    let Some(a) = assets() else { return };
    let e = a.scenes.entrance_index("ENTR_TEMPLE_OF_TIME_0").unwrap();
    let mut save = SaveContext::new(e, false, 0);
    save.apply_preset("ocarina-of-time").unwrap();
    // The intro seen (EVENTCHKINF_A7).
    save.set_event_chk_inf(0xA7);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save.clone()).unwrap();
    let tick = |w: &mut PlayState| w.tick_with(scripted_input(PadState::default(), PadState::default()));
    // Link at the altar, facing the door (Door_Toki updates only inside its culling volume).
    w.place_player(glam::Vec3::new(0.0, -40.0, 1450.0), i16::MIN);
    for _ in 0..60 {
        tick(&mut w);
    }
    // Demo_Kankyo 0xD (the door) spawns Door_Toki as its child, with gDoorTokiCol.
    let h = w.actors.all().into_iter().find(|&h| w.actors.downcast::<oot_actors::door_toki::DoorToki>(h).is_some()).expect("Door_Toki");
    let bg = w.actors.downcast::<oot_actors::door_toki::DoorToki>(h).unwrap().bg;
    assert!(w.col.dyna.is_bg_actor(bg));
    // A line through the doorway (the door at z 1080) hits its polys while it's shut.
    let (from, to) = (glam::Vec3::new(0.0, 20.0, 1180.0), glam::Vec3::new(0.0, 20.0, 980.0));
    let hit = w.col.entity_line_test(from, to, true, true, true, true);
    assert!(hit.is_some_and(|(_, poly)| poly.bg == bg), "{hit:?}");
    // EVENTCHKINF_OPENED_DOOR_OF_TIME set (Demo_Kankyo, as the door finishes opening): from
    // Door_Toki's next update the doorway is free (DynaPoly_DisableCollision).
    w.save.set_event_chk_inf(0x4B);
    tick(&mut w);
    tick(&mut w);
    assert_eq!(w.col.entity_line_test(from, to, true, true, true, true), None);
    // With the door open from the start, Demo_Kankyo spawns no Door_Toki (and goes).
    save.set_event_chk_inf(0x4B);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap();
    for _ in 0..60 {
        tick(&mut w);
    }
    assert!(w.actors.all().into_iter().all(|h| w.actors.downcast::<oot_actors::door_toki::DoorToki>(h).is_none()));
    assert_eq!(w.col.entity_line_test(from, to, true, true, true, true), None);
}

#[test]
fn the_left_hand_lists_for_the_pedestal_and_the_broken_knife() {
    let Some(r) = rules() else { return };
    let left = |lo: &Loadout| r.limb_dlists(lo, 0)[0].1.clone().unwrap_or_default();
    let default = |age| Loadout::default_for(&r, age);
    // func_80851A50: gPlayerLeftHandBgsDLs + linkAge (the child's: holding the Master Sword),
    // gPlayerLeftHandClosedDLs + linkAge (the adult's).
    let child = Loadout { left_hand: Some(r.model_type("LH_BGS")), ..default(Age::Child) };
    assert_eq!(left(&child), "gLinkChildLeftHandHoldingMasterSwordDL");
    let adult = Loadout { left_hand: Some(r.model_type("LH_CLOSED")), ..default(Age::Adult) };
    assert_eq!(left(&adult), "gLinkAdultLeftHandClosedNearDL");
    // The Biggoron's Sword's group (LH_BGS): whole, and with swordHealth 0 its second half
    // (dLists += 4: gLinkAdultHandHoldingBrokenGiantsKnifeDL).
    let bgs = r.model_groups.iter().position(|g| g.left == r.model_type("LH_BGS")).expect("a group holding the Biggoron's Sword");
    let whole = Loadout { model_group: bgs, ..default(Age::Adult) };
    assert_eq!(left(&whole), "gLinkAdultLeftHandHoldingBgsNearDL");
    let broken = Loadout { knife_broken: true, ..whole.clone() };
    assert_eq!(left(&broken), "gLinkAdultHandHoldingBrokenGiantsKnifeDL");
    // Each has its mesh in the pack.
    let pack = pack().unwrap();
    for lo in [&child, &adult, &broken] {
        let names = pack.link_variant_names(lo.age);
        assert!(names.contains(&lo.variant_key(&r)), "{} not baked", lo.variant_key(&r));
    }
}

#[test]
fn the_adults_gauntlets_boots_and_tunic_are_drawn() {
    let Some(a) = assets() else { return };
    let e = a.scenes.entrance_index("ENTR_TEMPLE_OF_TIME_4").unwrap();
    let mut save = SaveContext::new(e, false, 0);
    save.apply_preset("master-sword-adult").unwrap();
    // The golden gauntlets (strength 3), the hover boots and the Goron Tunic worn.
    save.inventory_change_upgrade(UPG_STRENGTH, 3);
    // ITEM_TUNIC_GORON (item.h: 0x42).
    item_give(&mut save, None, 0x42);
    item_give(&mut save, None, ITEM_BOOTS_HOVER);
    save.inventory_change_equipment(EQUIP_TYPE_TUNIC, 2);
    save.inventory_change_equipment(EQUIP_TYPE_BOOTS, 3);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap();
    for _ in 0..30 {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
    let p = w.player();
    let view = ViewInfo { eye: glam::Vec3::ZERO, billboard: glam::Mat4::IDENTITY };
    let mut out = DrawOut::default();
    use oot_game::actor_ctx::ActorImpl;
    p.draw(&p.render_state(), &w, &view, &mut out);
    let names: Vec<String> = out.opa.iter().map(|c| c.mesh.name.clone()).collect();
    // Standing with open hands: the plates' second lists (gLinkAdult{Left,Right}GauntletPlate2DL),
    // sGauntletColors[1] (254, 207, 15); the hover boots (sBootDListGroups[1]).
    let key = |b: &str| oot_game::pack::keys::bake(b);
    let gauntlets = out.opa.iter().find(|c| c.mesh.name == key("Player/gauntlets_open_open")).unwrap_or_else(|| panic!("no gauntlets in {names:?}"));
    assert_eq!(gauntlets.params.segments.as_ref().unwrap().env[0x0E], Some([254, 207, 15, 0]));
    assert!(names.contains(&key("Player/boots_hover")), "{names:?}");
    // sTunicColors[PLAYER_TUNIC_GORON] (100, 20, 0) on the body's tunic segment.
    let body = &out.opa[0];
    assert_eq!(body.params.segments.as_ref().unwrap().env[oot_game::player_lib::SEG_TUNIC as usize], Some([100, 20, 0, 0]));
    // Each of the bakes is in the pack.
    let pack = pack().unwrap();
    for b in ["Player/gauntlets_open_open", "Player/gauntlets_open_closed", "Player/gauntlets_closed_open", "Player/gauntlets_closed_closed", "Player/boots_iron", "Player/boots_hover", "Bg_Toki_Swd/sword"] {
        assert!(pack.bake(b).is_ok(), "{b} not baked");
    }
}

#[test]
fn the_pulls_time_warp_is_hidden_before_the_pull_unless_entered_from_the_chamber() {
    let Some(a) = assets() else { return };
    // DemoEffect_DrawTimeWarp: drawn on cutscene flag 1, on a cutscene layer, or whenever the
    // entrance is ENTR_TEMPLE_OF_TIME_4 (the adult's return from the Chamber). The pull's debug
    // start enters by the hall (ENTR_TEMPLE_OF_TIME_0) and moves to room 0: nothing until the pull.
    // Entered at ENTR_TEMPLE_OF_TIME_4 (as this milestone's first debug start was), the time warp
    // shows round the pedestal: the C's, on an entrance a child never takes.
    for (entrance, drawn) in [("ENTR_TEMPLE_OF_TIME_0", false), ("ENTR_TEMPLE_OF_TIME_4", true)] {
        let e = a.scenes.entrance_index(entrance).unwrap();
        let mut save = Route::MasterSwordPull.save(e);
        save.entrance_index = e;
        let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap();
        if entrance == "ENTR_TEMPLE_OF_TIME_0" {
            Route::MasterSwordPull.debug_start(&mut w);
        }
        for _ in 0..30 {
            w.tick_with(scripted_input(PadState::default(), PadState::default()));
        }
        let h = w.actors.all().into_iter().find(|&h| w.actors.actor(h).is_some_and(|a| a.id == 0x008B && a.params == 0xFF0Fu16 as i16)).expect("Demo_Effect 0xFF0F");
        let fx = w.actors.get(h).unwrap();
        let view = ViewInfo { eye: glam::Vec3::ZERO, billboard: glam::Mat4::IDENTITY };
        let mut out = DrawOut::default();
        fx.draw(&fx.render_state(), &w, &view, &mut out);
        assert_eq!(!out.xlu.is_empty() || !out.opa.is_empty(), drawn, "{entrance}");
    }
}
