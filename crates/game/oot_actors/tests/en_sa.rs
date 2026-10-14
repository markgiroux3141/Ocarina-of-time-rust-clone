//! `En_Sa` (Saria) against the C (`ovl_En_Sa/z_en_sa.c`): her type by the scene and the save
//! (`EnSa_GetType`), her texts by the flags (`EnSa_GetTextId`) and a talk's flag
//! (`EnSa_UpdateTalkState`), the meadow's line at z -2220 (`EnSa_AwaitTeachingSariasSong`), the
//! song's script run to its end and the song given (`EnSa_BecomeIdle`), and the greeting on a new
//! save in Kokiri Forest (`gKokiriForestSariaGreetingCs`, `INFTABLE_SARIA_GREETED_LINK` at its
//! end).
//!
//! Expected values come from the C, the scenes' actor lists (spot04 room 0 at (18, -80, 873),
//! kokiri_home5 at (2, 0, -4), spot05 at (118, 489, -2941)) and the scripts' cues (`ootx cutscene`).

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, BTN_CDOWN, BTN_CLEFT, BTN_CRIGHT, BTN_CUP, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_sa::*;
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::ocarina::*;
use oot_game::cutscene::CS_STATE_IDLE;
use oot_game::item::{ITEM_OCARINA_FAIRY, QUEST_KOKIRI_EMERALD, QUEST_SONG_SARIA};
use oot_game::message::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::{EVENTCHKINF_OBTAINED_ZELDAS_LETTER, SaveContext};

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Child Link entering by `entrance` at 10:00 on a new save changed by `f`, the audio side running
/// (the ocarina's staffs count its tasks).
fn enter(a: &Arc<GameAssets>, entrance: &str, f: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    f(&mut save);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    w.audio_side = Some(Box::new(oot_game::audio::offline::OfflineAudio::new(&a.pack.audio_data().unwrap(), false)));
    w
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn saria(w: &PlayState) -> Option<(ActorHandle, &EnSa)> {
    w.actors.all().into_iter().find_map(|h| w.actors.downcast::<EnSa>(h).map(|s| (h, s)))
}

fn script_name(w: &PlayState) -> Option<String> {
    w.cs_ctx.segment.as_ref().map(|s| s.name.clone())
}

/// Through texts until `f` holds (at most `max` frames): A pressed on a frame of its own whenever
/// a text waits (done, an event, or a choice: its first). Records each text id seen.
fn answer_until(w: &mut PlayState, max: usize, what: &str, texts: &mut Vec<u16>, f: impl Fn(&PlayState) -> bool) {
    answer_watching(w, max, what, texts, f, |_| {});
}

/// `answer_until`, with `each` called after every frame.
fn answer_watching(w: &mut PlayState, max: usize, what: &str, texts: &mut Vec<u16>, f: impl Fn(&PlayState) -> bool, mut each: impl FnMut(&PlayState)) {
    let mut pressed = false;
    for _ in 0..max {
        if f(w) {
            return;
        }
        let st = w.message_state();
        let a = !pressed && matches!(st, TEXT_STATE_CHOICE | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_EVENT | TEXT_STATE_AWAITING_NEXT);
        let pad = PadState { button: if a { BTN_A } else { 0 }, ..Default::default() };
        w.tick_with(scripted_input(PadState::default(), pad));
        pressed = a;
        if st != TEXT_STATE_NONE && texts.last() != Some(&w.msg_ctx.text_id) {
            texts.push(w.msg_ctx.text_id);
        }
        each(w);
    }
    panic!("{what}: not after {max} frames (msgMode {:#x}, text {:#x}, cs frame {})", w.msg_ctx.msg_mode, w.msg_ctx.text_id, w.cs_ctx.frames);
}

/// The pad button of an `OCARINA_BTN_*` (`sOcarinaButtonsToPad`'s order: A, C-Down, C-Right,
/// C-Left, C-Up).
fn pad_button(b: u8) -> u16 {
    [BTN_A, BTN_CDOWN, BTN_CRIGHT, BTN_CLEFT, BTN_CUP][b as usize]
}

/// `song`'s buttons from the pack's `gOcarinaSongButtons`, each held 3 frames and let go 2.
fn play_song(w: &mut PlayState, song: u8) {
    let b = w.audio.ocarina.song_buttons(song);
    let notes: Vec<u16> = b.buttons_index[..b.num_buttons as usize].iter().map(|&i| pad_button(i)).collect();
    for b in notes {
        let pad = PadState { button: b, ..Default::default() };
        for (prev, cur) in [(PadState::default(), pad), (pad, pad), (pad, pad), (pad, PadState::default()), (PadState::default(), PadState::default())] {
            w.tick_with(scripted_input(prev, cur));
        }
    }
}

/// The Sacred Forest Meadow (`ENTR_SACRED_FOREST_MEADOW_0`) with Zelda's Letter, the Fairy Ocarina
/// and without Saria's Song (the `graveyard-sun` preset), Link put down at `pos` facing north.
fn meadow(a: &Arc<GameAssets>, pos: Vec3) -> PlayState {
    let mut w = enter(a, "ENTR_SACRED_FOREST_MEADOW_0", |s| s.apply_preset("graveyard-sun").unwrap());
    assert!(!w.save.check_quest_item(QUEST_SONG_SARIA));
    idle(&mut w, 40);
    w.place_player(pos, -0x8000);
    idle(&mut w, 20);
    w
}

#[test]
fn her_type_follows_the_scene_and_the_save() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_KOKIRI_FOREST_0", |_| {});
    let ty = |w: &mut PlayState, scene: u16, f: &dyn Fn(&mut SaveContext)| {
        let keep = (w.scene_id, w.save.clone());
        w.scene_id = scene;
        f(&mut w.save);
        let t = get_type(w);
        (w.scene_id, w.save) = keep;
        t
    };
    let none = |_: &mut SaveContext| {};
    let letter = |s: &mut SaveContext| s.set_event_chk_inf(EVENTCHKINF_OBTAINED_ZELDAS_LETTER);
    let ocarina = |s: &mut SaveContext| {
        oot_game::item::item_give(s, None, ITEM_OCARINA_FAIRY);
    };
    let quest = |q: u32| move |s: &mut SaveContext| s.inventory.quest_items |= 1 << q;
    // A cutscene layer (cutsceneIndex CS_INDEX_0 and up, not CS_INDEX_D): the greeting in Kokiri
    // Forest (even greeted, with the emerald), the wait in the meadow (even with the song).
    for cs in [0xFFF0u16, 0xFFF8, 0xFFFF] {
        let at_cs = move |s: &mut SaveContext| {
            s.cutscene_index = cs;
            s.set_inf_table(INFTABLE_SARIA_GREETED_LINK);
            s.inventory.quest_items |= (1 << QUEST_KOKIRI_EMERALD) | (1 << QUEST_SONG_SARIA);
        };
        assert_eq!(ty(&mut w, SCENE_KOKIRI_FOREST, &at_cs), SARIA_TYPE_GREETING, "{cs:#x}");
        assert_eq!(ty(&mut w, SCENE_SACRED_FOREST_MEADOW, &at_cs), SARIA_TYPE_MEADOW_WAITING, "{cs:#x}");
    }
    // CS_INDEX_D loads no cutscene layer: the save decides (greeted, emerald: none).
    let cs_d = |s: &mut SaveContext| {
        s.cutscene_index = 0xFFFD;
        s.set_inf_table(INFTABLE_SARIA_GREETED_LINK);
        s.inventory.quest_items |= 1 << QUEST_KOKIRI_EMERALD;
    };
    assert_eq!(ty(&mut w, SCENE_KOKIRI_FOREST, &cs_d), SARIA_TYPE_NONE);
    // Her house: child Link with the Fairy Ocarina, without the letter.
    assert_eq!(ty(&mut w, SCENE_SARIAS_HOUSE, &ocarina), SARIA_TYPE_STANDING);
    assert_eq!(ty(&mut w, SCENE_SARIAS_HOUSE, &none), SARIA_TYPE_NONE);
    assert_eq!(ty(&mut w, SCENE_SARIAS_HOUSE, &|s: &mut SaveContext| {
        ocarina(s);
        letter(s);
    }), SARIA_TYPE_NONE);
    assert_eq!(ty(&mut w, SCENE_SARIAS_HOUSE, &|s: &mut SaveContext| {
        ocarina(s);
        s.adult = true;
    }), SARIA_TYPE_NONE);
    // The meadow: with the letter, waiting until the song, then playing; nobody without it.
    assert_eq!(ty(&mut w, SCENE_SACRED_FOREST_MEADOW, &letter), SARIA_TYPE_MEADOW_WAITING);
    assert_eq!(ty(&mut w, SCENE_SACRED_FOREST_MEADOW, &|s: &mut SaveContext| {
        letter(s);
        quest(QUEST_SONG_SARIA)(s);
    }), SARIA_TYPE_MEADOW_PLAYING);
    assert_eq!(ty(&mut w, SCENE_SACRED_FOREST_MEADOW, &quest(QUEST_SONG_SARIA)), SARIA_TYPE_NONE);
    // Kokiri Forest without the emerald: the greeting, then standing.
    assert_eq!(ty(&mut w, SCENE_KOKIRI_FOREST, &none), SARIA_TYPE_GREETING);
    assert_eq!(ty(&mut w, SCENE_KOKIRI_FOREST, &|s: &mut SaveContext| s.set_inf_table(INFTABLE_SARIA_GREETED_LINK)), SARIA_TYPE_STANDING);
    assert_eq!(ty(&mut w, SCENE_KOKIRI_FOREST, &quest(QUEST_KOKIRI_EMERALD)), SARIA_TYPE_NONE);
    // Anywhere else: none (Mido's house).
    assert_eq!(ty(&mut w, 0x28, &ocarina), SARIA_TYPE_NONE);

    // And as placed: her house (kokiri_home5, (2, 0, -4)) standing, with her fairy.
    let mut w = enter(&a, "ENTR_SARIAS_HOUSE_0", |s| {
        oot_game::item::item_give(s, None, ITEM_OCARINA_FAIRY);
    });
    idle(&mut w, 5);
    let (h, s) = saria(&w).expect("Saria in her house");
    assert_eq!((s.actor.home_pos.x, s.actor.home_pos.z), (2.0, -4.0));
    assert_eq!(s.action, Action::Idle);
    assert!(s.skel.is("gSariaWaitArmsToSideAnim"));
    assert_eq!(s.actor.gravity, -1.0);
    let fairy = w.actors.actor(h).unwrap().child.and_then(|c| w.actors.actor(c)).expect("her fairy");
    assert_eq!((fairy.id, fairy.params), (oot_actors::en_elf::ACTOR_EN_ELF, oot_actors::en_elf::FAIRY_KOKIRI));
    // Without the ocarina she isn't there (killed in her init).
    let mut w = enter(&a, "ENTR_SARIAS_HOUSE_0", |_| {});
    idle(&mut w, 5);
    assert!(saria(&w).is_none());
    // The meadow with the song: playing (sAnimationInfo1[11], gSariaPlayingOcarinaAnim), idle.
    let mut w = enter(&a, "ENTR_SACRED_FOREST_MEADOW_0", |s| {
        s.apply_preset("graveyard-sun").unwrap();
        s.inventory.quest_items |= 1 << QUEST_SONG_SARIA;
    });
    idle(&mut w, 5);
    let (_, s) = saria(&w).expect("Saria in the meadow");
    assert_eq!((s.action, s.actor.home_pos), (Action::Idle, Vec3::new(118.0, 489.0, -2941.0)));
    assert!(s.skel.is("gSariaPlayingOcarinaAnim"));
    assert!(w.cs_ctx.segment.is_none());
}

#[test]
fn her_texts_follow_the_flags() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_KOKIRI_FOREST_0", |_| {});
    // (text, the box count reset): each case's flags on a new save.
    let text = |w: &mut PlayState, f: &dyn Fn(&mut SaveContext)| {
        let keep = w.save.clone();
        f(&mut w.save);
        let (mut index, mut state) = (7u8, TEXT_STATE_DONE);
        let t = get_text_id(w, &mut index, &mut state);
        w.save = keep;
        (t, (index, state) == (0, TEXT_STATE_NONE))
    };
    let greeted = |s: &mut SaveContext| s.set_inf_table(INFTABLE_SARIA_GREETED_LINK);
    assert_eq!(text(&mut w, &|_| {}), (0x1001, false));
    assert_eq!(text(&mut w, &greeted), (0x1002, true));
    assert_eq!(text(&mut w, &|s| {
        greeted(s);
        s.set_inf_table(INFTABLE_SARIA_NOTICED_FAIRY);
    }), (0x1003, true));
    // Mido's first talk read (EVENTCHKINF_MIDO_DENIED_DEKU_TREE_ACCESS 0x02) wins over the greeting.
    let mido = |s: &mut SaveContext| {
        greeted(s);
        s.set_inf_table(INFTABLE_SARIA_NOTICED_FAIRY);
        s.set_event_chk_inf(0x02);
    };
    assert_eq!(text(&mut w, &mido), (0x1031, true));
    assert_eq!(text(&mut w, &|s| {
        mido(s);
        s.set_inf_table(INFTABLE_SARIA_WAS_TOLD_ABOUT_MIDO);
    }), (0x1032, true));
    // The Kokiri Emerald wins over Mido.
    let emerald = |s: &mut SaveContext| {
        mido(s);
        s.inventory.quest_items |= 1 << QUEST_KOKIRI_EMERALD;
    };
    assert_eq!(text(&mut w, &emerald), (0x1047, true));
    assert_eq!(text(&mut w, &|s| {
        emerald(s);
        s.set_inf_table(INFTABLE_SARIA_SPOKE_IN_HER_HOUSE);
    }), (0x1048, true));
    // Saria's Song wins over everything, and leaves the count.
    assert_eq!(text(&mut w, &|s| {
        emerald(s);
        s.inventory.quest_items |= 1 << QUEST_SONG_SARIA;
    }), (0x10AD, false));
}

#[test]
fn a_talk_after_the_greeting_says_0x1002_and_sets_its_flag() {
    let Some(a) = assets() else { return };
    // Kokiri Forest after the greeting: standing at (18, -80, 873).
    let mut w = enter(&a, "ENTR_KOKIRI_FOREST_0", |s| s.set_inf_table(INFTABLE_SARIA_GREETED_LINK));
    idle(&mut w, 60);
    let (h, s) = saria(&w).expect("Saria");
    assert_eq!(s.action, Action::Idle);
    assert!(w.cs_ctx.segment.is_none(), "no greeting once greeted");
    let home = s.actor.home_pos;
    assert_eq!((home.x, home.z), (18.0, 873.0));
    // Link 40 north of her (within her radius 20 + 30), facing her.
    let pos = Vec3::new(home.x, s.actor.world_pos.y, home.z - 40.0);
    w.place_player(pos, oot_game::target::yaw_to(pos, home));
    idle(&mut w, 3);
    assert_eq!(w.player().target_actor, Some(h), "her talk offered");
    assert_eq!(w.actors.actor(h).unwrap().text_id, 0x1002);
    // A: her talk; EnSa_Idle's 0x1002 at box 0: the right hand out, her mouth open.
    w.tick_with(scripted_input(PadState::default(), PadState { button: BTN_A, ..Default::default() }));
    idle(&mut w, 2);
    let s = w.actors.downcast::<EnSa>(h).unwrap();
    assert_eq!(s.interact_info.talk_state, 1);
    assert_eq!(w.msg_ctx.text_id, 0x1002);
    assert_eq!((s.anim_group, s.mouth_index), (ENSA_ANIMGROUP_EXTEND_RIGHT_HAND, SARIA_MOUTH_SMILING_OPEN));
    assert!(!w.save.get_inf_table(INFTABLE_SARIA_NOTICED_FAIRY));
    // Through its boxes to its close: INFTABLE_SARIA_NOTICED_FAIRY, talk state idle.
    let mut texts = Vec::new();
    answer_until(&mut w, 2000, "0x1002 read", &mut texts, |w| w.actors.downcast::<EnSa>(h).unwrap().interact_info.talk_state == 0);
    assert!(w.save.get_inf_table(INFTABLE_SARIA_NOTICED_FAIRY));
    assert!(w.actors.downcast::<EnSa>(h).unwrap().message_index >= 5, "her box count");
    // Then her hands behind her back (not talking, the fairy noticed, no emerald), and 0x1003.
    idle(&mut w, 2);
    let s = w.actors.downcast::<EnSa>(h).unwrap();
    assert_eq!(s.anim_group, ENSA_ANIMGROUP_HANDS_BEHIND_BACK);
    assert!(s.skel.is("gSariaTransitionHandsSideToBackAnim") || s.skel.is("gSariaHandsBehindBackWaitAnim"));
}

#[test]
fn the_meadow_lesson_starts_north_of_z_minus_2220() {
    let Some(a) = assets() else { return };
    // Link 80 south of the line (Link's first cue starts him at (0, 480, -2255)).
    let mut w = meadow(&a, Vec3::new(0.0, 480.0, -2140.0));
    let (h, s) = saria(&w).expect("Saria in the meadow");
    assert_eq!(s.action, Action::AwaitTeachingSariasSong);
    assert!(s.skel.is("gSariaPlayingOcarinaAnim"), "sAnimationInfo1[START_PLAYING_OCARINA]");
    assert!(w.player().actor.world_pos.z >= -2220.0);
    assert!(w.cs_ctx.segment.is_none());
    // Walking north: no script while z >= -2220; the frame Link is past it, her update (after
    // Player's) sets gMeadowSariasSongCs and cutsceneTrigger 1.
    let to = Vec3::new(0.0, 480.0, -2400.0);
    let mut prev = PadState::default();
    let mut crossed = None;
    for i in 0..200 {
        let pad = oot_actors::script::stick_towards(&w, to, 60.0);
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        let z = w.player().actor.world_pos.z;
        if script_name(&w).is_some() {
            assert!(z < -2220.0, "the script set with Link at z {z}");
            crossed = Some(i);
            break;
        }
        assert!(z >= -2220.0, "Link at z {z} and no script");
    }
    assert!(crossed.is_some(), "the line wasn't crossed");
    assert_eq!(script_name(&w).as_deref(), Some("gMeadowSariasSongCs"));
    assert_eq!(w.actors.downcast::<EnSa>(h).unwrap().action, Action::HandleCutscene);
    // Cutscene_UpdateScripted runs after Actor_UpdateAll (Play_Update): the same frame
    // Cutscene_SetupScripted takes the trigger and starts it.
    assert_ne!(w.cs_ctx.state, CS_STATE_IDLE);
    assert_eq!(w.save.cutscene_trigger, 0);
}

#[test]
fn the_meadows_script_ends_with_sarias_song_given() {
    let Some(a) = assets() else { return };
    // Link put down past the line: the script at once.
    let mut w = meadow(&a, Vec3::new(0.0, 480.0, -2300.0));
    let (h, _) = saria(&w).expect("Saria in the meadow");
    assert_ne!(w.cs_ctx.state, CS_STATE_IDLE);
    assert_eq!(script_name(&w).as_deref(), Some("gMeadowSariasSongCs"));
    // Her cue on channel 1 from frame 0: 6 (playing, at (118, 489, -2941) turned -3277).
    idle(&mut w, 3);
    let s = w.actors.downcast::<EnSa>(h).unwrap();
    assert_eq!(s.cue_id, 6);
    assert_eq!(s.actor.world_pos, Vec3::new(118.0, 489.0, -2941.0));
    assert_eq!(s.actor.shape_rot.y, -3277);
    assert_eq!(s.alpha, 255);
    // The lesson (0x10A8's yes, OCARINA_ACTION_TEACH_SARIA's demonstration), then the playback.
    let mut texts = Vec::new();
    answer_until(&mut w, 2000, "PLAYBACK_SARIA", &mut texts, |w| w.msg_ctx.ocarina_action == OCARINA_ACTION_PLAYBACK_SARIA && w.msg_ctx.msg_mode == MSGMODE_SONG_PLAYBACK);
    // Her cues so far: 7 (stop playing) at 445, 8 (the ocarina to her mouth) at 490.
    assert!(w.cs_ctx.frames >= 490);
    assert_eq!(w.actors.downcast::<EnSa>(h).unwrap().cue_id, 8);
    play_song(&mut w, OCARINA_SONG_SARIAS);
    for _ in 0..600 {
        if w.msg_ctx.ocarina_mode == OCARINA_MODE_03 {
            break;
        }
        idle(&mut w, 1);
    }
    assert_eq!(w.msg_ctx.ocarina_mode, OCARINA_MODE_03, "the song played back");
    // The message box's own Item_Give(ITEM_SONG_MINUET + 8) on the playback's success: taken back,
    // so the song at the script's end is hers.
    assert!(w.save.check_quest_item(QUEST_SONG_SARIA));
    w.save.inventory.quest_items &= !(1 << QUEST_SONG_SARIA);
    answer_until(&mut w, 3000, "the script's end", &mut texts, |w| w.cs_ctx.state == CS_STATE_IDLE);
    println!("the meadow's texts: {texts:x?}, ended at cs frame {}", w.cs_ctx.frames);
    assert!(texts.contains(&0x10A8) && texts.contains(&0x10AC), "{texts:x?}");
    // Her last cue was 10 (back to the ocarina, from 780); the script ends (CS_MISC 0x0C at 825).
    assert_eq!(w.actors.downcast::<EnSa>(h).unwrap().cue_id, 10);
    assert!(!w.save.check_quest_item(QUEST_SONG_SARIA));
    // EnSa_HandleCutscene sees CS_STATE_IDLE (EnSa_BecomeIdle next), then EnSa_BecomeIdle:
    // Item_Give(ITEM_SONG_SARIA), playing again, idle.
    idle(&mut w, 2);
    assert!(w.save.check_quest_item(QUEST_SONG_SARIA), "Saria's Song given at the script's end");
    let s = w.actors.downcast::<EnSa>(h).unwrap();
    assert_eq!(s.action, Action::Idle);
    assert!(s.skel.is("gSariaPlayingOcarinaAnim"));
    // With the song she'd now say 0x10AD.
    let (mut i, mut st) = (0, 0);
    assert_eq!(get_text_id(&w, &mut i, &mut st), 0x10AD);
}

#[test]
fn the_greeting_plays_on_a_new_save_and_ends_greeted() {
    let Some(a) = assets() else { return };
    // A new save out of Link's house (ENTR_KOKIRI_FOREST_3, room 0): Saria's init (once object_sa
    // is in, room 0's) sets gKokiriForestSariaGreetingCs and cutsceneTrigger 1.
    let mut w = enter(&a, "ENTR_KOKIRI_FOREST_3", |_| {});
    assert_eq!(w.room_ctx.cur.num, 0);
    let mut started = None;
    for i in 0..30 {
        idle(&mut w, 1);
        if script_name(&w).is_some() {
            started = Some(i);
            break;
        }
    }
    assert!(started.is_some(), "no greeting");
    assert_eq!(script_name(&w).as_deref(), Some("gKokiriForestSariaGreetingCs"));
    let (h, s) = saria(&w).expect("Saria");
    assert_eq!(s.action, Action::HandleCutscene);
    assert_eq!(s.actor.gravity, -1.0);
    let home = s.actor.home_pos;
    assert_eq!(home, Vec3::new(18.0, -80.0, 873.0));
    idle(&mut w, 2);
    assert_ne!(w.cs_ctx.state, CS_STATE_IDLE);
    // Her first cue (4, frames 0 to 70): placed at its start (0, 15, 163), arms to her sides.
    let s = w.actors.downcast::<EnSa>(h).unwrap();
    assert_eq!(s.cue_id, 4);
    assert!(s.actor.world_pos.distance(Vec3::new(0.0, 15.0, 163.0)) < 40.0, "{:?}", s.actor.world_pos);
    assert!(!w.save.get_inf_table(INFTABLE_SARIA_GREETED_LINK));
    // She runs (cue 3) to (-20, -74, 822), looks up (1), waves (2, her mouth open) and says 0x1001.
    let mut cues = vec![4];
    let mut texts = Vec::new();
    let mut run_end = Vec3::ZERO;
    let mut mouths = Vec::new();
    answer_watching(&mut w, 2000, "the greeting's end", &mut texts, |w| w.cs_ctx.state == CS_STATE_IDLE, |w| {
        let s = w.actors.downcast::<EnSa>(h).unwrap();
        if cues.last() != Some(&s.cue_id) {
            cues.push(s.cue_id);
            mouths.push(s.mouth_index);
            if s.cue_id == 1 {
                run_end = s.actor.world_pos;
            }
        }
    });
    println!("the greeting's cues {cues:?}, texts {texts:x?}, the run's end {run_end:?}");
    assert_eq!(cues, [4, 3, 1, 2]);
    // Her mouth: shut for 3 and 1, open for 2.
    assert_eq!(mouths, [SARIA_MOUTH_CLOSED2, SARIA_MOUTH_CLOSED2, SARIA_MOUTH_SMILING_OPEN]);
    // The run: a cue's velocity, (end - start) / frames, is set while curFrame < endFrame and moves
    // her the frame after; a cue is in effect while startFrame < curFrame <= endFrame (z_demo.c), so
    // she runs frames 71 to 79 of the first (24.6 a frame in z, -0.7 in x) and 81 to 240 of the
    // second (413 / 161 in z, -13 / 161 in x): from (0, 163) to (-19.22, 794.84), short of the cues'
    // (-20, 822).
    let expect = Vec3::new(-9.0 * 0.7 - 160.0 * 13.0 / 161.0, 0.0, 163.0 + 9.0 * 24.6 + 160.0 * 413.0 / 161.0);
    assert!(Vec3::new(run_end.x, 0.0, run_end.z).distance(expect) < 0.05, "the run ended at {run_end:?}, not {expect:?}");
    assert!(texts.contains(&0x1001), "{texts:x?}");
    // EnSa_HandleCutscene sees the end (EnSa_BecomeIdle), then EnSa_BecomeIdle: home, facing as
    // placed, her mouth shut, INFTABLE_SARIA_GREETED_LINK.
    idle(&mut w, 2);
    assert!(w.save.get_inf_table(INFTABLE_SARIA_GREETED_LINK));
    let s = w.actors.downcast::<EnSa>(h).unwrap();
    assert_eq!(s.action, Action::Idle);
    assert_eq!((s.actor.world_pos.x, s.actor.world_pos.z), (home.x, home.z));
    assert_eq!(s.actor.world_rot, s.init_rot);
    assert_eq!(s.init_rot.y, -7282);
    assert_eq!(s.mouth_index, SARIA_MOUTH_CLOSED2);
    // From now on she's standing, and talks 0x1002.
    assert_eq!(get_type(&w), SARIA_TYPE_STANDING);
}
