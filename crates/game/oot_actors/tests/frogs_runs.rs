//! GAME-10 milestone 9's runs, short debug starts on `ocarina-c` (the Ocarina of Time on C-Down,
//! the Song of Storms): `Route::Frogs` (Zora's River's log: the six songs to the frogs, their
//! rewards, then their song game) and `Route::Bonooru` (Lake Hylia: the spawn song recorded for
//! Bonooru, then played back by him). Each run's end that a preset holds is checked against the
//! run.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use oot_actors::PlayExt;
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn run(a: &Arc<GameAssets>, route: Route) -> (PlayState, Playthrough) {
    let e = a.scenes.entrance_index(route.entrance()).unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), route.save(e)).unwrap();
    w.audio.log = Some(Default::default());
    // The ocarina counts the audio side's tasks.
    w.audio_side = Some(Box::new(oot_game::audio::offline::OfflineAudio::new(&a.pack.audio_data().unwrap(), false)));
    route.debug_start(&mut w);
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    let trace = std::env::var("FROGS_TRACE").is_ok();
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
        if trace && w.gameplay_frames % 10 == 0 {
            let pl = w.player();
            eprintln!(
                "f {} pos {} action {:?} msgMode {:#x} ocarinaMode {} text {:#x} button {:x} state2 {:x} talk {:x?}",
                w.gameplay_frames, pl.actor.world_pos, pl.action, w.msg_ctx.msg_mode, w.msg_ctx.ocarina_mode, w.msg_ctx.text_id, p.button, pl.state2, w.msg_ctx.talk_actor.and_then(|h| w.actors.actor(h)).map(|a| (a.id, a.params))
            );
        }
    }
    eprintln!("steps {:?}", run.steps);
    eprintln!("texts {:x?}", run.texts);
    assert!(run.failure.is_none(), "{:?}", run.failure);
    (w, run)
}

/// The run's save is the preset's in its flags, inventory and equipment, with `extra` applied to
/// the run's (what the run doesn't do that the preset has).
fn ends_as_preset(w: &mut PlayState, preset: &str, extra: impl Fn(&mut SaveContext)) {
    w.save_scene_flags();
    let mut s = w.save.clone();
    extra(&mut s);
    let mut p = SaveContext::new(s.entrance_index, false, s.day_time);
    p.apply_preset(preset).unwrap();
    assert_eq!((&p.event_chk_inf, &p.inf_table, &p.item_get_inf, &p.event_inf), (&s.event_chk_inf, &s.inf_table, &s.item_get_inf, &s.event_inf));
    assert_eq!((&p.inventory.items, &p.inventory.ammo, p.inventory.upgrades, p.inventory.quest_items), (&s.inventory.items, &s.inventory.ammo, s.inventory.upgrades, s.inventory.quest_items));
    assert_eq!(&p.equips.button_items, &s.equips.button_items);
    for (i, (a, b)) in p.scene_flags.iter().zip(s.scene_flags.iter()).enumerate() {
        assert_eq!(format!("{a:?}"), format!("{b:?}"), "scene {i:#x}'s flags");
    }
}

#[test]
fn the_six_songs_and_the_song_game_on_the_frogs_log() {
    let Some(a) = assets() else { return };
    let e = a.scenes.entrance_index(Route::Frogs.entrance()).unwrap();
    let start = Route::Frogs.save(e);
    let (w, run) = run(&a, Route::Frogs);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(
        steps,
        vec![
            Step::FrogsTip,
            Step::OcarinaOut,
            Step::LullabyPlayed,
            Step::FrogsZl,
            Step::EponasSong,
            Step::FrogsEpona,
            Step::SariasSong,
            Step::FrogsSaria,
            Step::SunsSong,
            Step::FrogsSuns,
            Step::TimeSongPlayed,
            Step::FrogsTime,
            Step::SongOfStorms,
            Step::FrogsStorms,
            Step::FrogsGame,
            Step::FrogsChoir,
        ]
    );
    // The log's forced text 0x22E; each song's staff (0x86E), "Ocarina" (0x86F) and "You played"
    // (0x893 + song), then the frogs' 0x40A9 and the purple rupee's get-item text (0xF1); the Song
    // of Storms' 0x40AA and the piece of heart's (0xC4); with all six, 0x40AB, the song game's staff,
    // its 0x40AC and the second piece's (0xC5: the fourth piece, a heart container made).
    let mut texts = vec![0x22E];
    for played in [0x89B, 0x89A, 0x899, 0x89C, 0x89D] {
        texts.extend([0x86E, 0x86F, played, 0x40A9, 0xF1]);
    }
    texts.extend([0x86E, 0x86F, 0x89E, 0x40AA, 0xC4, 0x40AB, 0x86E, 0x40AC, 0xC5]);
    assert_eq!(run.texts, texts);
    // EVENTCHKINF_SONGS_FOR_FROGS_CHOIR (0xD0) to _STORMS (0xD6), the text's switch flag 0x3B.
    for f in 0xD0..=0xD6 {
        assert!(w.save.get_event_chk_inf(f), "eventChkInf {f:#x}");
    }
    assert!(w.flags.get_switch(0x3B));
    // Two pieces of heart (Item_Give(ITEM_HEART_PIECE_2): 0x10000000 each in questItems' top
    // nibble; four make a container, 0x10 more capacity). Five purple rupees (50 each) in the
    // child's wallet of 99.
    let quarters = |s: &SaveContext| s.health_capacity as u32 / 0x10 * 4 + (s.inventory.quest_items >> 28);
    assert_eq!(quarters(&w.save), quarters(&start) + 2);
    assert_eq!(w.save.rupees + w.save.rupee_accumulator, 99);
}

#[test]
fn bonooru_learns_a_song_and_plays_it_back() {
    let Some(a) = assets() else { return };
    let (mut w, run) = run(&a, Route::Bonooru);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::OcarinaOut, Step::BonooruSong, Step::BonooruRemembers, Step::BonooruPlayback]);
    // His 0x40A4, the recording's staff (0x86E), 0x40A5 once it's taken; talked to, 0x40A0 and the
    // playback's box (0x86F, Message_StartOcarinaImpl's for OCARINA_ACTION_SCARECROW_SPAWN_PLAYBACK).
    assert_eq!(run.texts, vec![0x40A4, 0x86E, 0x40A5, 0x40A0, 0x86F]);
    // The box's MSGMODE_SCARECROW_SPAWN_RECORDING_ONGOING: scarecrowSpawnSongSet, the spawn song
    // MemCopy'd into the save: A, C-Down, A, C-Up twice over (D4, F4, D4, D5) with rests between.
    assert_eq!(w.save.scarecrow_spawn_song_set, 1);
    let pitches: Vec<u8> = w.save.scarecrow_spawn_song.chunks(8).take(15).map(|n| n[0]).collect();
    let r = 0xFF;
    assert_eq!(pitches, [0x2, r, 0x5, r, 0x2, r, 0xE, r, 0x2, r, 0x5, r, 0x2, r, 0xE]);
    // A child's never sets EVENTCHKINF_9C (that's the adult's playback to him).
    assert!(!w.save.get_event_chk_inf(0x9C));
    ends_as_preset(&mut w, "scarecrow-song", |_| {});
}

/// Frames until `f` holds (at most `max`); panics with `what` if it never does.
fn until(w: &mut PlayState, max: usize, what: &str, f: impl Fn(&PlayState) -> bool) {
    for _ in 0..max {
        if f(w) {
            return;
        }
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
    panic!("{what}: not after {max} frames (msgMode {:#x}, ocarinaMode {})", w.msg_ctx.msg_mode, w.msg_ctx.ocarina_mode);
}

/// `b` pressed for `frames` frames, then let go.
fn press(w: &mut PlayState, b: u16, frames: usize) {
    let pad = PadState { button: b, ..Default::default() };
    for i in 0..frames {
        w.tick_with(scripted_input(if i == 0 { PadState::default() } else { pad }, pad));
    }
    w.tick_with(scripted_input(pad, PadState::default()));
}

// game-frogs.bat's `pierre` start: adult Lake Hylia on `scarecrow-pierre` (the spawn song set,
// EVENTCHKINF_9C), the song copied into the ocarina as oot's start does (Sram_OpenSave's MemCopy,
// `Ocarina::sram_open_save_scarecrow_songs`). The ocarina out (free play: AudioOcarina_Start with
// the spawn song's bit, 0x1000) and the song played: the box's OCARINA_MODE_0B, and Pierre
// (En_Kakasi2 0x053B at (1257, -1295, 3333)) comes up: his switch flag 0x3B.
#[test]
fn pierre_rises_to_the_scarecrows_song() {
    use eng_input::pad::{BTN_A, BTN_CDOWN, BTN_CUP};
    let Some(a) = assets() else { return };
    let e = a.scenes.entrance_index("ENTR_LAKE_HYLIA_0").unwrap();
    let mut save = SaveContext::new(e, true, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("scarecrow-pierre").unwrap();
    let mut audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), false);
    audio.ocarina.sram_open_save_scarecrow_songs(&save);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).unwrap();
    w.audio_side = Some(Box::new(oot_game::audio::offline::OfflineAudio::new(&a.pack.audio_data().unwrap(), false)));
    until(&mut w, 60, "Play_Init's frames", |w| w.gameplay_frames > 5);
    w.place_player(glam::Vec3::new(1337.0, -1269.0, 3373.0), -0x4000);
    until(&mut w, 60, "settled", |w| w.gameplay_frames > 40);
    assert!(!w.flags.get_switch(0x3B));
    press(&mut w, BTN_CDOWN, 1);
    until(&mut w, 60, "the staff", |w| w.msg_ctx.msg_mode == oot_game::message::MSGMODE_OCARINA_PLAYING);
    for b in [BTN_A, BTN_CDOWN, BTN_A, BTN_CUP, BTN_A, BTN_CDOWN, BTN_A, BTN_CUP] {
        press(&mut w, b, 3);
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
    until(&mut w, 300, "Pierre's switch flag", |w| w.flags.get_switch(0x3B));
}
