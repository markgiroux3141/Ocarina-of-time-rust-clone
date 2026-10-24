//! GAME-11 milestone 2's runs, chained by presets: `Route::DoorOfTime` (`ocarina-of-time` at the
//! Temple's entrance: its intro, the Ocarina of Time onto C-Down, the Song of Time at the altar,
//! the door opening), `Route::FirstPull` (`door-of-time`: the reveal, the pull, Ganondorf, the
//! Chamber of the Sages and the Light Medallion), `Route::Awakening` (`light-medallion`: Sheik's
//! awakening), `Route::TempleExit` (`adult-awake`: out of the Temple as an adult, Phase 12's exit)
//! and `Route::Ruins` (on through the ruined market, round its ReDeads, to the field by the broken
//! drawbridge). Each run's end is checked against its preset. Also `chamberCutsceneNum`
//! (`z_demo.c`'s destinations 29 to 31) and the Chamber's draw config.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::collections::BTreeSet;
use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::camera::{CAM_SET_PREREND_FIXED, CAM_SET_PREREND_PIVOT};
use oot_game::cutscene::{CS_STATE_IDLE, CutsceneScript};
use oot_game::item::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::*;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// `scene_table.h`: `SCENE_TEMPLE_OF_TIME` 0x43, `SCENE_CHAMBER_OF_THE_SAGES` 0x44,
/// `SCENE_CUTSCENE_MAP` 0x47, `SCENE_TEMPLE_OF_TIME_EXTERIOR_RUINS` 0x25, `SCENE_MARKET_RUINS`
/// 0x22, `SCENE_MARKET_ENTRANCE_RUINS` 0x1D, `SCENE_HYRULE_FIELD` 0x51.
const SCENE_TEMPLE_OF_TIME: u16 = 0x43;
const SCENE_CHAMBER_OF_THE_SAGES: u16 = 0x44;
const SCENE_CUTSCENE_MAP: u16 = 0x47;
const SCENE_SHRINE_R: u16 = 0x25;
const SCENE_MARKET_RUINS: u16 = 0x22;
const SCENE_ENRUI: u16 = 0x1D;
const SCENE_HYRULE_FIELD: u16 = 0x51;
/// `QUEST_MEDALLION_LIGHT` (`z64save.h`: 5, after the Forest's 0).
const QUEST_MEDALLION_LIGHT: u32 = QUEST_MEDALLION_FOREST + 5;
/// `ACTOR_DEMO_EFFECT` (0x008B), `ACTOR_BG_TOKI_HIKARI` (0x006A), `ACTOR_DEMO_6K` (0x00F5)
/// (`actor_table.h`).
const ACTOR_DEMO_EFFECT: i16 = 0x008B;
const ACTOR_BG_TOKI_HIKARI: i16 = 0x006A;
const ACTOR_DEMO_6K: i16 = 0x00F5;

/// Runs `route` from its start, `watch` called after each frame; the play state at its end and
/// its run.
fn run(a: &Arc<GameAssets>, route: Route, mut watch: impl FnMut(&PlayState)) -> (PlayState, Playthrough) {
    let e = a.scenes.entrance_index(route.entrance()).unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), route.save(e)).unwrap();
    w.audio.log = Some(Default::default());
    route.debug_start(&mut w);
    // The ocarina's routes: AudioOcarina_Update runs in Audio_Update, on the audio side.
    if route.needs_audio() {
        w.audio_side = Some(Box::new(oot_game::audio::offline::OfflineAudio::new(&a.pack.audio_data().unwrap(), false)));
    }
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
        watch(&w);
    }
    eprintln!("steps {:?}", run.steps);
    eprintln!("texts {:x?}", run.texts);
    eprintln!("at {:?} scene {:#x} layer {} entrance {:#x} adult {}", w.player().actor.world_pos, w.scene_id, w.save.scene_layer, w.save.entrance_index, w.save.adult);
    assert!(run.failure.is_none(), "{:?}", run.failure);
    (w, run)
}

/// The run's save is the preset's in its age, flags, inventory and equipment, with `extra`
/// applied to the run's (what the run doesn't do that the preset has).
fn ends_as_preset(w: &mut PlayState, preset: &str, extra: impl Fn(&mut SaveContext)) {
    // The scene's flags as leaving it saves them (Play_SaveSceneFlags).
    w.save_scene_flags();
    let mut s = w.save.clone();
    extra(&mut s);
    let mut p = SaveContext::new(s.entrance_index, false, s.day_time);
    p.apply_preset(preset).unwrap();
    assert_eq!(p.adult, s.adult);
    assert_eq!((&p.event_chk_inf, &p.inf_table, &p.item_get_inf, &p.event_inf), (&s.event_chk_inf, &s.inf_table, &s.item_get_inf, &s.event_inf));
    assert_eq!((&p.inventory.items, &p.inventory.ammo, p.inventory.upgrades, p.inventory.quest_items, p.inventory.equipment), (&s.inventory.items, &s.inventory.ammo, s.inventory.upgrades, s.inventory.quest_items, s.inventory.equipment));
    assert_eq!((p.equips, p.child_equips, p.adult_equips), (s.equips, s.child_equips, s.adult_equips));
    assert_eq!((p.rupees, p.health_capacity), (s.rupees + s.rupee_accumulator, s.health_capacity));
    for (i, (a, b)) in p.scene_flags.iter().zip(s.scene_flags.iter()).enumerate() {
        assert_eq!(format!("{a:?}"), format!("{b:?}"), "scene {i:#x}'s flags");
    }
}

fn steps(run: &Playthrough) -> Vec<Step> {
    run.steps.iter().map(|s| s.0).collect()
}

fn count(w: &PlayState, id: i16, params: i16) -> usize {
    w.actors.all().into_iter().filter(|&h| w.actors.actor(h).is_some_and(|a| a.id == id && a.params == params && !a.killed)).count()
}

#[test]
fn the_door_of_time() {
    let Some(a) = assets() else { return };
    // The script frame EVENTCHKINF_OPENED_DOOR_OF_TIME was set on.
    let mut opened_on = None;
    let (mut w, run) = run(&a, Route::DoorOfTime, |w| {
        if opened_on.is_none() && w.save.get_event_chk_inf(0x4B) {
            opened_on = Some((w.cs_ctx.state, w.cs_ctx.frames));
        }
    });
    assert_eq!(steps(&run), vec![Step::TempleIntro, Step::MenuOpened, Step::CursorOnItem, Step::ItemEquipped, Step::MenuClosed, Step::OcarinaOut, Step::TimeSongPlayed, Step::DoorOfTimeOpen]);
    // The intro's EVENTCHKINF_A7 (sEntranceCutsceneTable).
    assert!(w.save.get_event_chk_inf(0xA7));
    // The Ocarina of Time on C-Down (KaleidoScope_SetupItemEquip: buttonItems[2], cButtonSlots[1]
    // SLOT_OCARINA), the boomerang off it.
    assert_eq!((w.save.equips.button_items[2], w.save.equips.c_button_slots[1]), (ITEM_OCARINA_OF_TIME, SLOT_OCARINA as u8));
    // gDoorOfTimeOpeningCs: Demo_Kankyo 0xD sets the flag as the door finishes sliding (its cue
    // 0xD, frames 510 to 611), inside the script (its CS_MISC_STOP_CUTSCENE at 790).
    let (state, frame) = opened_on.expect("the door never opened");
    assert_ne!(state, CS_STATE_IDLE);
    assert!((510..790).contains(&frame), "opened on the script's frame {frame}");
    // Bg_Toki_Hikari 1 (the light over the door) gone; Link where the script's last player cue
    // put him (0, -40, 1400).
    assert_eq!(count(&w, ACTOR_BG_TOKI_HIKARI, 1), 0);
    assert_eq!(w.player().actor.world_pos, Vec3::new(0.0, -40.0, 1400.0));
    // The doorway (the door at z 1080) free: no Door_Toki.
    assert_eq!(w.col.entity_line_test(Vec3::new(0.0, 20.0, 1180.0), Vec3::new(0.0, 20.0, 980.0), true, true, true, true), None);
    ends_as_preset(&mut w, "door-of-time", |_| {});
}

#[test]
fn the_first_pull_to_the_chamber_of_the_sages() {
    let Some(a) = assets() else { return };
    // Each scene and layer as entered, and whether Link was an adult in it.
    let mut scenes: Vec<(u16, usize, bool)> = Vec::new();
    let mut medallion_effects = 0;
    let mut chamber_segments = false;
    let (mut w, run) = run(&a, Route::FirstPull, |w| {
        let here = (w.scene_id, w.save.scene_layer, w.save.adult);
        if scenes.last() != Some(&here) {
            scenes.push(here);
        }
        if w.scene_id == SCENE_CHAMBER_OF_THE_SAGES {
            medallion_effects = medallion_effects.max(count(w, ACTOR_DEMO_EFFECT, oot_actors::demo_effect::DEMO_EFFECT_MEDAL_LIGHT as i16));
            chamber_segments |= w.scene.as_ref().unwrap().segments.is_some();
        }
    });
    assert_eq!(
        steps(&run),
        vec![Step::SwordReveal, Step::SwordRevealed, Step::SwordPulled, Step::GanondorfFromSword, Step::Chamber, Step::LightMedallion, Step::ChamberLeft]
    );
    // Navi's 0x70E6 to 0x70E8 (gRevealMasterSwordCs), Ganondorf's 0x700A, Rauru's 0x704E,
    // 0x704F, Navi's 0x7050, Rauru's 0x7051 and its first answer's 0x7053, the medallion's 0x0040
    // and 0x70E9.
    assert_eq!(run.texts, vec![0x70E6, 0x70E7, 0x70E8, 0x700A, 0x704E, 0x704F, 0x7050, 0x7051, 0x7053, 0x0040, 0x70E9]);
    // The Temple a child; terminator 8's first time: the cutscene map's layer 7 (CS_INDEX_3), still
    // a child; terminator 25: the Chamber's layer 4 (CS_INDEX_0), an adult.
    assert_eq!(scenes, vec![(SCENE_TEMPLE_OF_TIME, 0, false), (SCENE_CUTSCENE_MAP, 7, false), (SCENE_CHAMBER_OF_THE_SAGES, 4, true)]);
    // SDC_CHAMBER_OF_THE_SAGES ported: its segments each frame.
    assert!(chamber_segments);
    // Rauru's Demo_Effect 0xE (the medallion) once, and Item_Give(ITEM_MEDALLION_LIGHT).
    assert_eq!(medallion_effects, 1);
    assert!(w.save.check_quest_item(QUEST_MEDALLION_LIGHT));
    // The pull: Item_Give(ITEM_SWORD_MASTER), EVENTCHKINF_OBTAINED_MASTER_SWORD; the reveal's 0x4F.
    assert!(w.save.get_event_chk_inf(0x45) && w.save.get_event_chk_inf(0x4F));
    // The first adult's equips (Inventory_SwapAgeEquipment): the Master Sword on B, bombs on
    // C-Down, the Ocarina of Time (the ocarina's slot) on C-Right; the child's, the Ocarina of Time
    // on C-Down, kept.
    assert_eq!(w.save.equips.button_items, [ITEM_SWORD_MASTER, ITEM_NONE, ITEM_BOMB, ITEM_OCARINA_OF_TIME]);
    assert_eq!(w.save.child_equips.button_items[2], ITEM_OCARINA_OF_TIME);
    // Terminator 18: to ENTR_TEMPLE_OF_TIME_4 with a black fade.
    assert_eq!(w.transition.next_entrance_index, a.scenes.entrance_index("ENTR_TEMPLE_OF_TIME_4").unwrap());
    assert_eq!(w.transition.ty, oot_game::transition::TRANS_TYPE_FADE_BLACK);
    // chamberCutsceneNum untouched (the Light Medallion's is layer 4, not a number).
    assert_eq!(w.save.chamber_cutscene_num, CHAMBER_CS_FOREST);
    ends_as_preset(&mut w, "light-medallion", |_| {});
}

#[test]
fn sheiks_awakening() {
    let Some(a) = assets() else { return };
    // The frame EVENTCHKINF_C9 was set on, and whether the return's time warp (Demo_Effect
    // 0xFF0F, the shrinking version) was out then; Demo_6K 0x10 seen.
    let mut c9_on = None;
    let mut warp_seen = false;
    let mut light_ball = false;
    let (mut w, run) = run(&a, Route::Awakening, |w| {
        let warp = count(w, ACTOR_DEMO_EFFECT, 0xFF0Fu16 as i16) > 0;
        warp_seen |= warp;
        if c9_on.is_none() && w.save.get_event_chk_inf(0xC9) {
            c9_on = Some((w.gameplay_frames, warp));
        }
        light_ball |= count(w, ACTOR_DEMO_6K, 0x10) > 0;
    });
    assert_eq!(steps(&run), vec![Step::SheikAwakening, Step::AdultAwake]);
    // Navi's 0x7054, Sheik's 0x7056, 0x7057 and 0x7077 to 0x707A.
    assert_eq!(run.texts, vec![0x7054, 0x7056, 0x7057, 0x7077, 0x7078, 0x7079, 0x707A]);
    // DemoEffect_UpdateTimeWarpReturnFromChamberOfSages: 0xC9 on its 251st update, the actor killed
    // the same update.
    assert!(warp_seen && light_ball);
    let (frame, warp) = c9_on.expect("EVENTCHKINF_C9 never set");
    assert!(!warp);
    assert!((251..300).contains(&frame), "EVENTCHKINF_C9 on frame {frame}");
    // gTempleOfTimeFirstAdultCs' last player cues: Link at (0, 40, 214), facing 30037; mode 0x30's
    // sword drawn (func_80851D80): the Master Sword in hand.
    let p = w.player();
    assert_eq!(p.actor.world_pos, Vec3::new(0.0, 40.0, 214.0));
    assert_eq!((p.held_item_id, p.actor.shape_rot.y), (ITEM_SWORD_MASTER, 30037));
    assert!(w.save.get_event_chk_inf(0xC5));
    ends_as_preset(&mut w, "adult-awake", |_| {});
}

#[test]
fn out_of_the_temple_as_an_adult() {
    let Some(a) = assets() else { return };
    let mut cams: BTreeSet<(i16, i16)> = BTreeSet::new();
    let (mut w, run) = run(&a, Route::TempleExit, |w| {
        if w.scene_id == SCENE_SHRINE_R {
            cams.insert((w.game_camera.setting, w.game_camera.bg_cam_index));
        }
    });
    assert_eq!(steps(&run), vec![Step::TempleExterior]);
    // Exit 1 (ENTR_TEMPLE_OF_TIME_EXTERIOR_DAY_1): the entrance table's adult row, the ruins'
    // shrine_r (layer 2), spawn 1.
    assert_eq!((w.scene_id, w.save.scene_layer, w.save.adult), (SCENE_SHRINE_R, 2, true));
    // Its floor's bg cameras: PREREND_FIXED 0 or 1 (Camera_Fixed3 with the JPEG backgrounds).
    eprintln!("shrine_r cameras {cams:x?}");
    assert!(!cams.is_empty() && cams.iter().all(|&(s, i)| s == CAM_SET_PREREND_FIXED && (0..=1).contains(&i)), "{cams:x?}");
    ends_as_preset(&mut w, "adult-awake", |_| {});
}

#[test]
fn the_ruins_to_the_field() {
    let Some(a) = assets() else { return };
    let mut near = f32::MAX;
    let mut redead_not_idle = false;
    let mut cams: BTreeSet<(u16, i16)> = BTreeSet::new();
    let (mut w, run) = run(&a, Route::Ruins, |w| {
        if [SCENE_SHRINE_R, SCENE_MARKET_RUINS, SCENE_ENRUI].contains(&w.scene_id) && w.transition.mode == oot_game::transition::TRANS_MODE_OFF {
            cams.insert((w.scene_id, w.game_camera.setting));
        }
        if w.scene_id == SCENE_MARKET_RUINS {
            for h in w.actors.all() {
                if let Some(r) = w.actors.downcast::<oot_actors::en_rd::EnRd>(h) {
                    near = near.min(r.actor.xz_dist_to_player);
                    redead_not_idle |= r.action != oot_actors::en_rd::REDEAD_ACTION_IDLE;
                }
            }
        }
    });
    assert_eq!(steps(&run), vec![Step::MarketRuins, Step::MarketEntranceRuins, Step::FieldByTheDrawbridge]);
    // EnRd_Idle hears Link only within 150: round the fountain no ReDead comes within 160, none
    // leaves its idle, Link never frozen.
    eprintln!("nearest ReDead {near}, cameras {cams:x?}");
    assert!(near > 160.0, "{near}");
    assert!(!redead_not_idle);
    // shrine_r and enrui: PREREND_FIXED; market_ruins (SCENE_CAM_TYPE_FIXED_MARKET): its one bg
    // camera, PREREND_PIVOT.
    assert_eq!(
        cams,
        BTreeSet::from([(SCENE_ENRUI, CAM_SET_PREREND_FIXED), (SCENE_MARKET_RUINS, CAM_SET_PREREND_PIVOT), (SCENE_SHRINE_R, CAM_SET_PREREND_FIXED)])
    );
    // ENTR_HYRULE_FIELD_7: the field's adult layer 2 at spawn 7, by the broken drawbridge
    // (Bg_Spot00_Break 0 at (0, -10, 670)) and its three fences.
    assert_eq!((w.scene_id, w.save.scene_layer), (SCENE_HYRULE_FIELD, 2));
    let pieces: Vec<_> = w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<oot_actors::bg_spot00_break::BgSpot00Break>(h)).collect();
    assert_eq!(pieces.len(), 4);
    assert!(pieces.iter().any(|b| !b.is_fence() && b.actor.home_pos == Vec3::new(0.0, -10.0, 670.0)));
    ends_as_preset(&mut w, "adult-awake", |_| {});
}

/// A script of one command, `CS_DESTINATION(destination, 0, 1)` (`CS_HEADER(1, 10)`,
/// `CS_END_OF_SCRIPT`), started as an actor starts one (`cutsceneTrigger` 1).
fn play_terminator(w: &mut PlayState, destination: u16) {
    let mut data = Vec::new();
    // CS_DESTINATION: CS_CMD_DESTINATION, 1, CMD_HH(destination, startFrame), CMD_HH(endFrame, endFrame).
    for word in [1u32, 10, 0x03E8, 1, (destination as u32) << 16, 1 << 16 | 1, 0xFFFF_FFFF, 0] {
        data.extend_from_slice(&word.to_be_bytes());
    }
    // The scene's fade-in over first.
    for _ in 0..60 {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
    assert_eq!(w.transition.trigger, oot_game::transition::TRANS_TRIGGER_OFF);
    w.cs_ctx.segment = Some(Arc::new(CutsceneScript { file: "test".into(), name: format!("terminator {destination}"), data }));
    w.save.cutscene_trigger = 1;
    for _ in 0..10 {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
        if w.transition.trigger == oot_game::transition::TRANS_TRIGGER_START {
            return;
        }
    }
    panic!("terminator {destination} never started its transition");
}

#[test]
fn the_medallions_destinations_set_chamber_cutscene_num() {
    let Some(a) = assets() else { return };
    // SaveContext_Init and the file select: CHAMBER_CS_FOREST.
    assert_eq!(SaveContext::new(0, false, 0).chamber_cutscene_num, CHAMBER_CS_FOREST);
    assert_eq!(SaveContext::file_select_new().chamber_cutscene_num, CHAMBER_CS_FOREST);
    // z_demo.c:849-868: 29 the Forest, 30 the Fire (and Item_Give(ITEM_MEDALLION_FIRE)), 31 the
    // Water; each to ENTR_CHAMBER_OF_THE_SAGES_0 with a white fade.
    for (destination, num) in [(29, CHAMBER_CS_FOREST), (30, CHAMBER_CS_FIRE), (31, CHAMBER_CS_WATER)] {
        let e = a.scenes.entrance_index("ENTR_TEMPLE_OF_TIME_4").unwrap();
        let mut save = SaveContext::new(e, false, 0);
        save.apply_preset("adult-awake").unwrap();
        save.chamber_cutscene_num = CHAMBER_CS_LIGHT;
        let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap();
        // (Sheik's awakening seen: no script of its own.)
        play_terminator(&mut w, destination);
        assert_eq!(w.save.chamber_cutscene_num, num, "destination {destination}");
        assert_eq!(w.transition.next_entrance_index, a.scenes.entrance_index("ENTR_CHAMBER_OF_THE_SAGES_0").unwrap());
        assert_eq!(w.transition.ty, oot_game::transition::TRANS_TYPE_FADE_WHITE);
        assert_eq!(w.save.check_quest_item(QUEST_MEDALLION_FOREST + 1), destination == 30, "the Fire Medallion");
    }
    // Not in the save (Save ends at 0x1354), but in the slot's copy of SaveContext:
    // chamberCutsceneNum at 0x1415.
    let mut s = SaveContext::new(0, false, 0);
    s.chamber_cutscene_num = CHAMBER_CS_WATER;
    assert_eq!(s.slot_bytes()[0x1415], CHAMBER_CS_WATER);
}
