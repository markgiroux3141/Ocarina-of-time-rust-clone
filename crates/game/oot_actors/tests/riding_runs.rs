//! GAME-12 milestone 1's runs: `Route::RideEpona` (Lon Lon Ranch's corral as an adult before
//! Epona: Epona's Song, on her, over a fence, off her) and `Route::FieldRide` (Hyrule Field with
//! Epona had: on her, into the ranch through its gate on her). Each run's end is checked against
//! its preset.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use oot_actors::PlayExt;
use oot_actors::en_horse::*;
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::*;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// `SCENE_LON_LON_RANCH` (`scene_table.h`: 0x63).
const SCENE_LON_LON_RANCH: u16 = 0x63;
/// `CAM_SET_HORSE` (`camera.h`: 0x06).
const CAM_SET_HORSE: i16 = 0x06;

/// Runs `route` from its start, `watch` called after each frame; the play state at its end and
/// its run.
fn run(a: &Arc<GameAssets>, route: Route, mut watch: impl FnMut(&PlayState)) -> (PlayState, Playthrough) {
    let e = a.scenes.entrance_index(route.entrance()).unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), route.save(e)).unwrap();
    w.audio.log = Some(Default::default());
    route.debug_start(&mut w);
    // Epona's Song: AudioOcarina_Update runs in Audio_Update, on the audio side.
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

/// The run's save is the preset's in its age, flags, inventory and equipment.
fn ends_as_preset(w: &mut PlayState, preset: &str) {
    w.save_scene_flags();
    let s = w.save.clone();
    let mut p = SaveContext::new(s.entrance_index, false, s.day_time);
    p.apply_preset(preset).unwrap();
    assert_eq!(p.adult, s.adult);
    assert_eq!((&p.event_chk_inf, &p.inf_table, &p.item_get_inf, &p.event_inf), (&s.event_chk_inf, &s.inf_table, &s.item_get_inf, &s.event_inf));
    assert_eq!((&p.inventory.items, &p.inventory.ammo, p.inventory.quest_items, p.inventory.equipment), (&s.inventory.items, &s.inventory.ammo, s.inventory.quest_items, s.inventory.equipment));
    assert_eq!((p.equips, p.child_equips, p.adult_equips), (s.equips, s.child_equips, s.adult_equips));
}

fn steps(run: &Playthrough) -> Vec<Step> {
    run.steps.iter().map(|s| s.0).collect()
}

fn epona(w: &PlayState) -> Option<&EnHorse> {
    w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<EnHorse>(h)).find(|h| h.ty == HORSE_EPONA && !h.actor.killed)
}

/// What a riding run saw, frame by frame.
#[derive(Default)]
struct Seen {
    /// Epona's actions, each change.
    actions: Vec<i32>,
    /// The game camera's settings, each change.
    settings: Vec<i16>,
    /// The fastest she went.
    top_speed: f32,
    /// Spurs taken (`numBoosts` falling).
    spurs: u32,
    boosts: u8,
}

impl Seen {
    fn watch(&mut self, w: &PlayState) {
        if let Some(e) = epona(w) {
            if self.actions.last() != Some(&e.action) {
                self.actions.push(e.action);
            }
            self.top_speed = self.top_speed.max(e.actor.speed_xz);
            if e.num_boosts < self.boosts {
                self.spurs += 1;
            }
            self.boosts = e.num_boosts;
        }
        if self.settings.last() != Some(&w.game_camera.setting) {
            self.settings.push(w.game_camera.setting);
        }
    }
}

#[test]
fn ride_epona_in_the_corral() {
    let Some(a) = assets() else { return };
    let mut seen = Seen::default();
    let (mut w, run) = run(&a, Route::RideEpona, |w| seen.watch(w));
    eprintln!("actions {:?} settings {:?} top {} spurs {}", seen.actions, seen.settings, seen.top_speed, seen.spurs);
    assert_eq!(steps(&run), vec![Step::OcarinaOut, Step::EponaSongPlayed, Step::Mounted, Step::RodeOver, Step::Dismounted]);
    assert_eq!(w.scene_id, SCENE_LON_LON_RANCH);
    // Before the song EnHorse_FleePlayer; after it EnHorse_Inactive's song makes her come
    // (EnHorse_FollowPlayer), then EnHorse_Idle until mounted (EnHorse_MountedIdle).
    let at = |act: i32| seen.actions.iter().position(|&x| x == act).unwrap_or_else(|| panic!("no action {act} in {:?}", seen.actions));
    assert!(at(ENHORSE_ACT_FLEE_PLAYER) < at(ENHORSE_ACT_FOLLOW_PLAYER));
    assert!(at(ENHORSE_ACT_FOLLOW_PLAYER) < at(ENHORSE_ACT_MOUNTED_IDLE));
    // The walk, the trot and the gallop; the fence high-jumped (EnHorse_HighJumpInit: an
    // obstacle 70 or more high at a gallop).
    for act in [ENHORSE_ACT_MOUNTED_WALK, ENHORSE_ACT_MOUNTED_TROT, ENHORSE_ACT_MOUNTED_GALLOP, ENHORSE_ACT_HIGH_JUMP] {
        assert!(seen.actions.contains(&act), "no action {act} in {:?}", seen.actions);
    }
    // A spur: EnHorse_StartBraking's spur raises the gallop past 8 (by the boost's curve,
    // `EnHorse_UpdateSpeed`'s boostSpeed 13 for Epona).
    assert!(seen.spurs >= 1 && seen.top_speed > 12.0, "spurs {} top {}", seen.spurs, seen.top_speed);
    // The horse camera while on her (Horse_RequestHorseCameraSetting: CAM_SET_HORSE), and back.
    assert!(seen.settings.contains(&CAM_SET_HORSE), "{:?}", seen.settings);
    assert_ne!(w.game_camera.setting, CAM_SET_HORSE);
    // Off her, standing.
    let p = w.player();
    assert_eq!(p.state1 & oot_actors::player::STATE1_23, 0);
    assert!(p.ride_actor.is_none());
    ends_as_preset(&mut w, "adult-awake");
}

#[test]
fn ride_epona_out_of_the_field_into_the_ranch() {
    let Some(a) = assets() else { return };
    let mut seen = Seen::default();
    // In the ranch: whether Link was on her from its first frame, and her actions.
    let mut first_in_ranch: Option<bool> = None;
    let mut ranch = Seen::default();
    let (mut w, run) = run(&a, Route::FieldRide, |w| {
        if w.scene_id == SCENE_LON_LON_RANCH {
            first_in_ranch.get_or_insert(w.player().state1 & oot_actors::player::STATE1_23 != 0);
            ranch.watch(w);
        } else {
            seen.watch(w);
        }
    });
    eprintln!("actions {:?} settings {:?} top {}", seen.actions, seen.settings, seen.top_speed);
    assert_eq!(steps(&run), vec![Step::OcarinaOut, Step::EponaSongPlayed, Step::Mounted, Step::RodeOut, Step::RiddenIn, Step::Dismounted]);
    assert_eq!(w.scene_id, SCENE_LON_LON_RANCH);
    eprintln!("in the ranch: actions {:?} settings {:?}", ranch.actions, ranch.settings);
    // The field: called, on her, up the ramp at a trot and a gallop, on the horse camera.
    for act in [ENHORSE_ACT_FOLLOW_PLAYER, ENHORSE_ACT_MOUNTED_IDLE, ENHORSE_ACT_MOUNTED_TROT, ENHORSE_ACT_MOUNTED_GALLOP] {
        assert!(seen.actions.contains(&act), "no action {act} in {:?}", seen.actions);
    }
    assert!(seen.settings.contains(&CAM_SET_HORSE), "{:?}", seen.settings);
    // The ranch: Horse_SetupInGameplay with R_EXITED_SCENE_RIDING_HORSE spawns her under Link
    // (HORSE_PTYPE_PLAYER_SPAWNED_RIDING) and Player starts on her; from EnHorse_Init's idle,
    // frozen by the mount (EnHorse_Freeze), EnHorse_Frozen's PLAYER_SPAWNED_RIDING outside a
    // cutscene gallops in at 8 (EnHorse_StartGalloping), then the run halts her.
    assert_eq!(first_in_ranch, Some(true));
    assert_eq!(ranch.actions.iter().find(|&&x| x != ENHORSE_ACT_IDLE && x != ENHORSE_ACT_FROZEN), Some(&ENHORSE_ACT_MOUNTED_GALLOP), "{:?}", ranch.actions);
    assert!(ranch.settings.contains(&CAM_SET_HORSE), "{:?}", ranch.settings);
    // R_EXITED_SCENE_RIDING_HORSE: set by the exit, cleared by EnHorse_Init.
    assert!(!w.save.r_exited_scene_riding_horse);
    ends_as_preset(&mut w, "epona");
}

