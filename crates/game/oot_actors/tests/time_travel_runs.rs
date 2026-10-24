//! GAME-11 milestone 3's runs: `Route::Prelude` (`prelude`, a debug start with the Forest
//! Medallion's `EVENTCHKINF_48`: Sheik's Prelude of Light), `Route::SwordBack` (the sword put back:
//! a child again), `Route::SwordAgain` (pulled again: an adult) and `Route::PreludeWarp` (the
//! Prelude played in the field: the warp to the Temple of Time's light pad, Link hidden through
//! the white-out and the arrival). Each run's end is checked against its preset.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::item::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::*;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// `SCENE_TEMPLE_OF_TIME` (`scene_table.h`: 0x43).
const SCENE_TEMPLE_OF_TIME: u16 = 0x43;
/// `QUEST_SONG_PRELUDE` (`z64save.h`: 0x0B, after the Minuet's 6), `QUEST_MEDALLION_FOREST`'s.
const QUEST_SONG_PRELUDE: u32 = QUEST_SONG_MINUET + 5;
/// `ACTOR_EN_ARROW` (`actor_table.h`: 0x0016), `ARROW_NUT` (10).
const ACTOR_EN_ARROW: i16 = 0x0016;
const ARROW_NUT: i16 = 10;

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


/// `save` (a preset, then what the C does to it) and the run's are the same in their age, flags,
/// inventory and equipment.
fn ends_as(w: &mut PlayState, p: SaveContext) {
    w.save_scene_flags();
    let s = &w.save;
    assert_eq!(p.adult, s.adult);
    assert_eq!((&p.event_chk_inf, &p.inf_table, &p.item_get_inf), (&s.event_chk_inf, &s.inf_table, &s.item_get_inf));
    assert_eq!((&p.inventory.items, p.inventory.quest_items, p.inventory.equipment), (&s.inventory.items, s.inventory.quest_items, s.inventory.equipment));
    assert_eq!((p.equips, p.child_equips, p.adult_equips), (s.equips, s.child_equips, s.adult_equips));
}

#[test]
fn the_prelude_of_light() {
    let Some(a) = assets() else { return };
    // Sheik's draws through the script (SHEIK_DRAW_*), and her nut (En_Arrow ARROW_CS_NUT).
    let mut draws: Vec<i32> = Vec::new();
    let mut nut = false;
    let (mut w, run) = run(&a, Route::Prelude, |w| {
        for h in w.actors.all() {
            if let Some(x) = w.actors.downcast::<oot_actors::en_xc::EnXc>(h)
                && draws.last() != Some(&x.draw_mode)
            {
                draws.push(x.draw_mode);
            }
            // (EnArrow_Init makes ARROW_CS_NUT an ARROW_NUT that updates in cutscenes.)
            nut |= w.actors.actor(h).is_some_and(|a| a.id == ACTOR_EN_ARROW && a.params == ARROW_NUT);
        }
    });
    assert_eq!(steps(&run), vec![Step::PreludeStarted, Step::PreludePlayed, Step::PreludeLearned]);
    // Sheik's 0x7074, 0x708B, 0x708C; the playback's 0x0875 and 0x086D; "You have learned the
    // Prelude of Light!" (0x0078); her 0x7075.
    assert_eq!(run.texts, vec![0x7074, 0x708B, 0x708C, 0x0875, 0x086D, 0x0078, 0x7075]);
    // EnXc_InitTempleOfTime: EVENTCHKINF_55 and Item_Give(ITEM_SONG_PRELUDE).
    assert!(w.save.get_event_chk_inf(0x55) && w.save.check_quest_item(QUEST_SONG_PRELUDE));
    // Type 2's actions: drawn (cue 6), pulling out the harp (cue 23), the harp, the harp put away
    // (cue 5) and drawn plain, then the nut thrown (action 43, EnXc_SpawnNut) and gone (cue 9:
    // Actor_Kill).
    use oot_actors::en_xc::{SHEIK_DRAW_DEFAULT, SHEIK_DRAW_HARP, SHEIK_DRAW_PULLING_OUT_HARP};
    eprintln!("draws {draws:?}");
    assert!(draws.windows(3).any(|d| d == [SHEIK_DRAW_DEFAULT, SHEIK_DRAW_PULLING_OUT_HARP, SHEIK_DRAW_HARP]), "{draws:?}");
    assert!(nut);
    assert!(w.actors.all().into_iter().all(|h| w.actors.downcast::<oot_actors::en_xc::EnXc>(h).is_none()));
    // The script's last player cues: (0, 40, 214).
    assert_eq!(w.player().actor.world_pos, Vec3::new(0.0, 40.0, 214.0));
    ends_as_preset(&mut w, "prelude-learned", |_| {});
}

#[test]
fn the_sword_put_back_in_the_story() {
    let Some(a) = assets() else { return };
    let (mut w, run) = run(&a, Route::SwordBack, |_| {});
    assert_eq!(steps(&run), vec![Step::SwordPutBack, Step::TimeTravelled]);
    // Terminator 8 with EVENTCHKINF_OBTAINED_MASTER_SWORD: a child at ENTR_TEMPLE_OF_TIME_2 with
    // his own buttons (the Kokiri Sword, sticks, the Ocarina of Time on C-Down, the slingshot); the
    // adult's kept in adultEquips.
    assert!(!w.save.adult);
    assert_eq!(w.save.entrance_index, a.scenes.entrance_index("ENTR_TEMPLE_OF_TIME_2").unwrap());
    assert_eq!(w.save.equips.button_items[0], ITEM_SWORD_KOKIRI);
    assert_eq!(w.save.equips.button_items[2], ITEM_OCARINA_OF_TIME);
    assert_eq!(w.save.adult_equips.button_items, [ITEM_SWORD_MASTER, ITEM_NONE, ITEM_BOMB, ITEM_OCARINA_OF_TIME]);
    ends_as_preset(&mut w, "child-again", |_| {});
}

#[test]
fn the_sword_pulled_again() {
    let Some(a) = assets() else { return };
    let (mut w, run) = run(&a, Route::SwordAgain, |_| {});
    assert_eq!(steps(&run), vec![Step::SwordPulled, Step::TimeTravelled]);
    // An adult again with his own buttons. Inventory_SwapAgeEquipment keeps what it stored: the
    // adult's buttons stay in adultEquips (prelude-learned, made by the first swap, has none there).
    assert!(w.save.adult);
    assert_eq!(w.save.equips.button_items, [ITEM_SWORD_MASTER, ITEM_NONE, ITEM_BOMB, ITEM_OCARINA_OF_TIME]);
    let mut p = SaveContext::new(w.save.entrance_index, false, w.save.day_time);
    p.apply_preset("child-again").unwrap();
    // The pull's Item_Give(ITEM_SWORD_MASTER) (on the child's B, worn), then the swap: the child's
    // equipment stored as it is (the Master Sword's nibble), his B as the Kokiri Sword.
    item_give(&mut p, None, ITEM_SWORD_MASTER);
    p.inventory_swap_age_equipment();
    p.adult = true;
    ends_as(&mut w, p);
}

#[test]
fn the_preludes_warp() {
    let Some(a) = assets() else { return };
    // Player drawn (`actor.draw`) each frame: in the field, and in the Temple after the warp.
    let mut field: Vec<bool> = Vec::new();
    let mut temple: Vec<bool> = Vec::new();
    let (mut w, run) = run(&a, Route::PreludeWarp, |w| {
        let drawn = w.player().actor.is_drawn;
        if w.scene_id == SCENE_TEMPLE_OF_TIME { temple.push(drawn) } else { field.push(drawn) }
    });
    assert_eq!(steps(&run), vec![Step::OcarinaOut, Step::PreludeWarpPlayed, Step::PreludeWarpOut, Step::PreludeWarped]);
    // The ocarina's 0x086E and 0x086F, the Prelude played (0x0898), "warp?" (0x0892).
    assert_eq!(run.texts, vec![0x086E, 0x086F, 0x0898, 0x0892]);
    // The warp-out: drawn until its timer's 15 (the white at its fullest), hidden from then to the
    // scene change (BACKLOG #26).
    let first_hidden = field.iter().position(|d| !d).expect("never hidden in the warp-out");
    assert!(field[first_hidden..].iter().all(|d| !d));
    // The arrival (start mode 5: draw NULL) hidden until the warp-in's script's first action
    // (Player_Action_WaitForCutscene's 40 frames, then func_80852280), drawn after.
    let shown = temple.iter().position(|d| *d).expect("never shown after the warp");
    assert!(shown >= 40, "shown on the arrival's frame {shown}");
    assert!(temple[shown..].iter().all(|d| *d));
    // ENTR_TEMPLE_OF_TIME_7: spawn 7 on Bg_Mjin's pad (0, -20, 2025).
    assert_eq!(w.save.entrance_index, a.scenes.entrance_index("ENTR_TEMPLE_OF_TIME_7").unwrap());
    assert_eq!(w.player().actor.world_pos, Vec3::new(0.0, -20.0, 2025.0));
    ends_as_preset(&mut w, "prelude-learned", |_| {});
}
