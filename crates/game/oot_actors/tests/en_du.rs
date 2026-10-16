//! GAME-09 milestone 2 against the C: Darunia (`z_en_du.c`), his door (`z_bg_spot18_shutter.c`)
//! and his room's statue and spear (`z_bg_spot18_obj.c`).
//!
//! The placements (`ootx scene-info --scene spot18`): the child layers 0 and 1, room 1: `En_Du`
//! 0xFFFF at (91, 41, -1462), `Bg_Spot18_Obj` 0xFF00 (the statue, yaw -182) and 0x0001 (the
//! spear) at (92, 40, -1540); room 3: `Bg_Spot18_Shutter` 0x001B at (-72, -1, -410) and the song
//! spot `En_Okarina_Tag` 0x1C9B (type 7, Zelda's Lullaby, switch 0x1B, rot.z 1: range 40) at
//! (-71, -2, -333). The adult layers 2 and 3: room 1's statue 0x1100 (yaw 0), room 3's doors
//! 0x00FF and 0x0100. Room 3 and room 1 meet at the door (transition actor 2, (-69, 31, -589)).
//!
//! Child Link enters Goron City (`ENTR_GORON_CITY_0`, room 3) with `ranch-epona` (the Fairy
//! Ocarina, Zelda's Lullaby, Saria's Song), its intro seen (`EVENTCHKINF_A6`, `z_demo.c`'s
//! `sEntranceCutsceneTable`), and is put in room 1 as the debug starts do. Expected values are
//! worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, BTN_CDOWN, BTN_CLEFT, BTN_CRIGHT, BTN_CUP, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_spot18_obj::{self, BgSpot18Obj};
use oot_actors::bg_spot18_shutter::{self, BgSpot18Shutter};
use oot_actors::en_du::*;
use oot_actors::en_okarina_tag::EnOkarinaTag;
use oot_actors::player::{Player, STATE2_23, STATE2_25};
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::ocarina::{OCARINA_SONG_LULLABY, OCARINA_SONG_SARIAS};
use oot_game::audio::sfx::{NA_SE_EV_STONEDOOR_STOP, NA_SE_SY_CORRECT_CHIME};
use oot_game::cutscene::CS_STATE_IDLE;
use oot_game::item::{ITEM_OCARINA_FAIRY, ITEM_STRENGTH_GORONS_BRACELET, QUEST_GORON_RUBY, UPG_STRENGTH, item_give};
use oot_game::message::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// `EVENTCHKINF_A6` (`save.h`): Goron City's intro (`gGoronCityIntroCs`) seen.
const EVENTCHKINF_A6: u16 = 0xA6;
/// `SCENE_GORON_CITY` (`scene_table.h`: 0x62).
const SCENE_GORON_CITY: usize = 0x62;
/// The door's switch flag (its params 0x001B, the song spot's 0x1C9B).
const DOOR_FLAG: i32 = 0x1B;

const DARUNIA_HOME: Vec3 = Vec3::new(91.0, 41.0, -1462.0);
/// Where the songs' scripts put Link (their `PLAYER_ACTION`'s first cue): 71 in front of
/// Darunia, facing him (-z).
const LINK_BY_DARUNIA: Vec3 = Vec3::new(91.0, 41.0, -1391.0);
const STATUE_HOME: Vec3 = Vec3::new(92.0, 40.0, -1540.0);
const SHUTTER_HOME: Vec3 = Vec3::new(-72.0, -1.0, -410.0);
const TAG_HOME: Vec3 = Vec3::new(-71.0, -2.0, -333.0);

/// Link entered at Goron City (`ENTR_GORON_CITY_0`) at 10:00 with `ranch-epona`, its intro seen,
/// `setup`, the sound log and the audio side on (the ocarina counts its tasks).
fn goron_city(a: &Arc<GameAssets>, adult: bool, setup: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_GORON_CITY_0").expect("entrance");
    let mut save = SaveContext::new(e, adult, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("ranch-epona").unwrap();
    save.set_event_chk_inf(EVENTCHKINF_A6);
    setup(&mut save);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    w.audio_side = Some(Box::new(oot_game::audio::offline::OfflineAudio::new(&a.pack.audio_data().unwrap(), false)));
    idle(&mut w, 2);
    w
}

/// Room `room` loaded and Link put at `pos` facing `yaw`, `n` frames run.
fn in_room(w: &mut PlayState, room: i8, pos: Vec3, yaw: i16, n: usize) {
    oot_actors::playthrough::deku_tree_room_start(w, room, pos, yaw);
    idle(w, n);
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// `b` pressed on a frame of its own.
fn press(w: &mut PlayState, b: u16) {
    w.tick_with(scripted_input(PadState::default(), PadState { button: b, ..Default::default() }));
}

/// The pad button of an `OCARINA_BTN_*` (`sOcarinaButtonsToPad`: A, C-Down, C-Right, C-Left,
/// C-Up).
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

/// The C button the Fairy Ocarina is on.
fn ocarina_button(w: &PlayState) -> u16 {
    let i = (1..4).find(|&i| w.save.equips.button_items[i] == ITEM_OCARINA_FAIRY).expect("the ocarina on a C button");
    [BTN_CLEFT, BTN_CDOWN, BTN_CRIGHT][i - 1]
}

/// The ocarina out, the staff up.
fn ocarina_out(w: &mut PlayState) {
    let b = ocarina_button(w);
    press(w, b);
    until(w, 60, "the staff", |w| w.msg_ctx.msg_mode == MSGMODE_OCARINA_PLAYING);
}

/// Frames until `f` holds (at most `max`); panics with `what` if it never does.
fn until(w: &mut PlayState, max: usize, what: &str, f: impl Fn(&PlayState) -> bool) -> usize {
    for n in 0..max {
        if f(w) {
            return n;
        }
        idle(w, 1);
    }
    panic!("{what}: not after {max} frames (msgMode {:#x}, ocarinaMode {}, text {:#x})", w.msg_ctx.msg_mode, w.msg_ctx.ocarina_mode, w.msg_ctx.text_id);
}

/// Through texts until `f` holds (at most `max` frames): A on a frame of its own whenever a text
/// waits, each text id recorded, `each` called after every frame. Returns the frames run and the
/// A presses.
fn answer_watching(w: &mut PlayState, max: usize, what: &str, texts: &mut Vec<u16>, f: impl Fn(&PlayState) -> bool, mut each: impl FnMut(&PlayState)) -> (usize, usize) {
    let mut pressed = false;
    let mut presses = 0;
    for n in 0..max {
        if f(w) {
            return (n, presses);
        }
        let st = w.message_state();
        let a = !pressed && matches!(st, TEXT_STATE_CHOICE | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_EVENT | TEXT_STATE_AWAITING_NEXT);
        presses += a as usize;
        let pad = PadState { button: if a { BTN_A } else { 0 }, ..Default::default() };
        w.tick_with(scripted_input(PadState::default(), pad));
        pressed = a;
        if w.message_state() != TEXT_STATE_NONE && w.msg_ctx.text_id != 0 && texts.last() != Some(&w.msg_ctx.text_id) {
            texts.push(w.msg_ctx.text_id);
        }
        each(w);
    }
    panic!("{what}: not after {max} frames (msgMode {:#x}, text {:#x}, cs frame {}, texts {texts:x?})", w.msg_ctx.msg_mode, w.msg_ctx.text_id, w.cs_ctx.frames);
}

fn answer_until(w: &mut PlayState, max: usize, what: &str, texts: &mut Vec<u16>, f: impl Fn(&PlayState) -> bool) -> (usize, usize) {
    answer_watching(w, max, what, texts, f, |_| {})
}

fn player(w: &PlayState) -> &Player {
    w.actors.downcast::<Player>(w.player.unwrap()).unwrap()
}

fn live<T: 'static + oot_game::actor_ctx::ActorImpl>(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<T>(h).is_some_and(|a| !a.base().killed)).collect()
}

/// Darunia, alive.
fn darunia(w: &PlayState) -> Option<(ActorHandle, &EnDu)> {
    live::<EnDu>(w).first().map(|&h| (h, w.actors.downcast::<EnDu>(h).unwrap()))
}

fn du(w: &PlayState) -> &EnDu {
    darunia(w).expect("Darunia").1
}

fn shutter(w: &PlayState) -> (ActorHandle, &BgSpot18Shutter) {
    let h = *live::<BgSpot18Shutter>(w).first().expect("the door");
    (h, w.actors.downcast::<BgSpot18Shutter>(h).unwrap())
}

fn spot18_objs(w: &PlayState) -> Vec<&BgSpot18Obj> {
    live::<BgSpot18Obj>(w).into_iter().map(|h| w.actors.downcast::<BgSpot18Obj>(h).unwrap()).collect()
}

fn sfx_frames(w: &PlayState, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, _)| f).collect()
}

fn script_name(w: &PlayState) -> Option<String> {
    w.cs_ctx.segment.as_ref().map(|s| s.name.clone())
}

/// The attention cutscene (`OnePointCutscene_Attention`, 5010) on `h` is queued or running.
fn attention_on(w: &PlayState, h: ActorHandle) -> bool {
    (0..oot_game::camera::NUM_CAMS as i16).any(|i| w.camera(i).is_some_and(|c| c.cs_id == 5010 && c.target == Some(h)))
}

/// Child Link in Darunia's room, in front of him, facing him.
fn by_darunia(a: &Arc<GameAssets>, setup: impl FnOnce(&mut SaveContext)) -> PlayState {
    let mut w = goron_city(a, false, setup);
    in_room(&mut w, 1, LINK_BY_DARUNIA, -0x8000, 4);
    w
}

/// The pack has Darunia's bake and the lists the door, the statue and the spear draw.
#[test]
fn the_pack_has_the_meshes() {
    let Some(a) = assets() else { return };
    assert!(a.pack.bake(BAKE).is_ok(), "{BAKE}");
    for dl in ["gGoronCityDoorDL", "gGoronCityStatueDL", "gGoronCityStatueSpearDL"] {
        assert!(a.pack.mesh(bg_spot18_obj::OBJECT, dl).is_ok(), "{dl}");
    }
}

#[test]
fn darunia_is_in_goron_city_for_a_child_only() {
    let Some(a) = assets() else { return };
    // func_809FDDB4: SCENE_GORON_CITY and LINK_IS_CHILD: here; EnDu_Init's last branch for a child
    // without a cutscene index: func_809FE3C0, ENDU_ANIM_0 (gDaruniaIdleAnim), scale 0.01,
    // ATTENTION_RANGE_1.
    let w = by_darunia(&a, |_| {});
    let (_, d) = darunia(&w).expect("a child's Darunia");
    assert_eq!(d.action, Action::Func809FE3C0);
    assert!(d.skel.is("gDaruniaIdleAnim"));
    assert_eq!((d.actor.scale.x, d.actor.target_mode), (0.01, 1));
    assert_eq!((d.actor.home_pos.x, d.actor.home_pos.z), (DARUNIA_HOME.x, DARUNIA_HOME.z));
    assert!(w.cs_ctx.segment.is_none(), "no cutscene index: no dance");

    // An adult in Goron City: func_809FDDB4 is 0, Actor_Kill.
    let mut w = goron_city(&a, true, |_| {});
    in_room(&mut w, 1, LINK_BY_DARUNIA, -0x8000, 4);
    assert!(darunia(&w).is_none(), "killed for an adult in Goron City");

    // The Fire Temple: an adult without INFTABLE_11A only.
    w.scene_id = SCENE_FIRE_TEMPLE;
    assert!(EnDu::is_here(&w));
    w.save.set_inf_table(INFTABLE_11A);
    assert!(!EnDu::is_here(&w));
    w.save.adult = false;
    w.save.inf_table[(INFTABLE_11A >> 4) as usize] &= !(1 << (INFTABLE_11A & 0xF));
    assert!(!EnDu::is_here(&w));
}

#[test]
fn his_texts_are_0x301a_then_0x301b_and_he_asks_for_the_ocarina() {
    let Some(a) = assets() else { return };
    let mut w = by_darunia(&a, |_| {});
    let (h, _) = darunia(&w).unwrap();
    // func_809FE3C0: xzDistToPlayer (71) < 116 + 20: PLAYER_STATE2_23 every frame (Player clears
    // it in its own update, which runs first).
    assert!(du(&w).actor.xz_dist_to_player < 136.0);
    assert_ne!(player(&w).state2 & STATE2_23, 0);
    // Npc_UpdateTalking offers 0x301A (no mask, no bracelet, no INFTABLE_113).
    assert_eq!(player(&w).target_actor, Some(h));
    assert_eq!(w.actors.actor(h).unwrap().text_id, 0x301A);
    press(&mut w, BTN_A);
    let mut texts = Vec::new();
    answer_until(&mut w, 600, "0x301A read", &mut texts, |w| w.message_state() == TEXT_STATE_NONE && du(w).interact_info.talk_state == NPC_TALK_STATE_IDLE);
    assert_eq!(texts.first(), Some(&0x301A));
    // EnDu_UpdateTalkState: 0x301A closing sets INFTABLE_113.
    assert!(w.save.get_inf_table(INFTABLE_113));
    idle(&mut w, 3);
    assert_eq!(w.actors.actor(h).unwrap().text_id, 0x301B);

    // Out of 136: no PLAYER_STATE2_23.
    w.place_player(DARUNIA_HOME + Vec3::new(0.0, 0.0, 200.0), -0x8000);
    idle(&mut w, 2);
    assert_eq!(player(&w).state2 & STATE2_23, 0);
}

#[test]
fn with_the_bracelet_he_says_0x301d_and_0x301e_with_the_ruby() {
    let Some(a) = assets() else { return };
    // CUR_UPG_VALUE(UPG_STRENGTH) != 0: 0x301D; with QUEST_GORON_RUBY 0x301E.
    let mut w = by_darunia(&a, |s| {
        item_give(s, None, ITEM_STRENGTH_GORONS_BRACELET);
    });
    let h = darunia(&w).unwrap().0;
    assert_eq!(w.actors.actor(h).unwrap().text_id, 0x301D);
    w.save.inventory.quest_items |= 1 << QUEST_GORON_RUBY;
    idle(&mut w, 2);
    assert_eq!(w.actors.actor(h).unwrap().text_id, 0x301E);
}

#[test]
fn sarias_song_starts_the_dance_and_gives_the_bracelet() {
    let Some(a) = assets() else { return };
    let mut w = by_darunia(&a, |_| {});
    let h = darunia(&w).unwrap().0;
    // The ocarina out: Player_Action_8084E3C4 sees PLAYER_STATE2_23 and sets _24; func_809FE3C0:
    // Message_StartOcarinaSunsSongDisabled(OCARINA_ACTION_CHECK_SARIA), PLAYER_STATE2_25, unk_6A8
    // Darunia, func_809FE4A4.
    ocarina_out(&mut w);
    assert_eq!(du(&w).action, Action::Func809FE4A4);
    assert_eq!((w.msg_ctx.ocarina_action, w.msg_ctx.disable_suns_song), (OCARINA_ACTION_CHECK_SARIA, 1));
    assert_ne!(player(&w).state2 & STATE2_25, 0);
    assert_eq!(player(&w).unk_6a8, Some(h));
    // Saria's Song: OCARINA_MODE_03: the chime, gGoronCityDaruniaCorrectSongCs on csCtx,
    // cutsceneTrigger 1, unk_1E8 0, func_809FE890, ocarinaMode 04.
    play_song(&mut w, OCARINA_SONG_SARIAS);
    let audio_at_last_note = w.audio.frames;
    let to_cs = until(&mut w, 300, "the right song's script", |w| du(w).action == Action::Func809FE890);
    assert_eq!(script_name(&w).as_deref(), Some("gGoronCityDaruniaCorrectSongCs"));
    assert_eq!(du(&w).unk_1e8, 0);
    assert_eq!(w.msg_ctx.ocarina_mode, OCARINA_MODE_04);
    assert!(!sfx_frames(&w, NA_SE_SY_CORRECT_CHIME).is_empty());

    // The script: his cues on channel 2: 1 (0-55), 7 (55-1350); CS_MISC_STOP_CUTSCENE at 710 ends
    // it in cue 7. Cue 7: blinkTimer 11, unk_1EC 2 (eyes shut), unk_1ED 2 (mouth open), unk_1EE 1
    // (nose happy), the dance's steps (ENDU_ANIM_7, then func_809FE040's eight).
    let mut cues: Vec<i16> = Vec::new();
    let mut steps = std::collections::BTreeSet::new();
    let mut face_7 = None;
    let mut texts = Vec::new();
    let (cs_frames, cs_presses) = answer_watching(
        &mut w,
        3000,
        "the script's end",
        &mut texts,
        |w| du(w).action != Action::Func809FE890,
        |w| {
            let d = du(w);
            if cues.last() != Some(&d.unk_1ea) {
                cues.push(d.unk_1ea);
            }
            if d.unk_1ea == 7 {
                steps.insert(d.unk_1e6);
                face_7.get_or_insert((d.unk_1ec, d.unk_1ed, d.unk_1ee));
            }
        },
    );
    assert_eq!(cues, [1, 7]);
    assert_eq!(face_7, Some((2, 2, 1)));
    assert!(steps.len() > 1, "the dance steps on: {steps:?}");
    // The script's text 0x3007 (335-706) held its frame until read.
    assert_eq!(texts, [0x3007]);
    assert_eq!(w.cs_ctx.state, CS_STATE_IDLE);

    // func_809FEB08 (the frame after func_809FE890 saw the state idle and held Link,
    // PLAYER_CSACTION_1): no bracelet: 0x301C, func_809FEC14, ENDU_ANIM_14, talking; his face back.
    idle(&mut w, 1);
    let d = du(&w);
    assert_eq!(d.action, Action::Func809FEC14);
    assert_eq!(d.actor.text_id, 0x301C);
    assert!(d.skel.is(DANCING_END_ANIM));
    assert_eq!((d.unk_1ec, d.unk_1ed, d.unk_1ee), (0, 0, 0));
    assert_eq!(w.msg_ctx.text_id, 0x301C);

    // 0x301C closing: NPC_TALK_STATE_ACTION: Link let go (PLAYER_CSACTION_7), the bracelet offered
    // (GI_GORONS_BRACELET, within xzDistToPlayer + 1); taken: its text (0x79, sGetItemTable); read:
    // NPC_TALK_STATE_ITEM_GIVEN, func_809FE3C0 again.
    let mut texts = Vec::new();
    let (mut n, mut given_at, mut offered_at, mut text_starts) = (0, None, None, Vec::new());
    let (give_frames, give_presses) = answer_watching(
        &mut w,
        2000,
        "the bracelet's text read",
        &mut texts,
        |w| w.save.cur_upg_value(UPG_STRENGTH) == 1 && du(w).action == Action::Func809FE3C0 && w.message_state() == TEXT_STATE_NONE,
        |w| {
            n += 1;
            if given_at.is_none() && w.save.cur_upg_value(UPG_STRENGTH) == 1 {
                given_at = Some(n);
            }
            if offered_at.is_none() && du(w).action == Action::Func809FEC70 {
                offered_at = Some(n);
            }
            if text_starts.last().map(|&(_, t)| t) != Some(w.msg_ctx.text_id) && w.message_state() != TEXT_STATE_NONE {
                text_starts.push((n, w.msg_ctx.text_id));
            }
        },
    );
    assert_eq!(texts, [0x301C, 0x79]);
    assert_eq!(w.save.cur_upg_value(UPG_STRENGTH), 1, "Item_Give(ITEM_STRENGTH_GORONS_BRACELET)");
    assert_eq!(du(&w).interact_info.talk_state, NPC_TALK_STATE_IDLE);
    // gDaruniaDancingEndAnim played out: ENDU_ANIM_1 (EnDu_Update).
    assert!(du(&w).skel.is("gDaruniaIdleAnim"));
    // With the bracelet: 0x301D.
    idle(&mut w, 3);
    assert_eq!(w.actors.actor(h).unwrap().text_id, 0x301D);
    println!(
        "the right song: {} frames from the last note to the script, {} in it ({} A presses), {} to the bracelet's text read ({} A presses: 0x301C offered at {:?}, Item_Give at {:?}, texts at {:x?}); {} audio frames",
        to_cs,
        cs_frames,
        cs_presses,
        give_frames,
        give_presses,
        offered_at,
        given_at,
        text_starts,
        w.audio.frames - audio_at_last_note
    );
}

#[test]
fn a_wrong_song_plays_the_wrong_cutscene_and_he_waits_again() {
    let Some(a) = assets() else { return };
    let mut w = by_darunia(&a, |_| {});
    ocarina_out(&mut w);
    // Zelda's Lullaby under OCARINA_ACTION_CHECK_SARIA: ocarinaMode lastPlayedSong - 1
    // (OCARINA_SONG_LULLABY 8: 7, >= OCARINA_MODE_06): gGoronCityDaruniaWrongSongCs, unk_1E8 1.
    play_song(&mut w, OCARINA_SONG_LULLABY);
    until(&mut w, 300, "the wrong song's script", |w| du(w).action == Action::Func809FE890);
    assert_eq!(script_name(&w).as_deref(), Some("gGoronCityDaruniaWrongSongCs"));
    assert_eq!(du(&w).unk_1e8, 1);
    assert_eq!(w.msg_ctx.ocarina_mode, OCARINA_MODE_04);
    assert!(sfx_frames(&w, NA_SE_SY_CORRECT_CHIME).is_empty());
    // His cues 1 (0-55), 8 (55-210); the stop at 210. Cue 8: unk_1EC 3 (eyes open), unk_1ED 3
    // (mouth happy), unk_1EE 0, func_809FE104's four steps (the last held).
    let mut cues: Vec<i16> = Vec::new();
    let mut face_8 = None;
    let mut last_step = 0;
    let mut texts = Vec::new();
    answer_watching(
        &mut w,
        2000,
        "the script's end",
        &mut texts,
        |w| du(w).action != Action::Func809FE890,
        |w| {
            let d = du(w);
            if cues.last() != Some(&d.unk_1ea) {
                cues.push(d.unk_1ea);
            }
            if d.unk_1ea == 8 {
                face_8.get_or_insert((d.unk_1ec, d.unk_1ed, d.unk_1ee));
                last_step = d.unk_1e6;
            }
        },
    );
    assert_eq!(cues, [1, 8]);
    assert_eq!(face_8, Some((3, 3, 0)));
    assert!(last_step >= 1, "the wrong song's steps: {last_step}");
    // The script's text 0x3020 (170-200).
    assert_eq!(texts, [0x3020]);
    // func_809FEB08 with unk_1E8 1: Link let go (PLAYER_CSACTION_7), ENDU_ANIM_1, func_809FE3C0.
    idle(&mut w, 2);
    let d = du(&w);
    assert_eq!(d.action, Action::Func809FE3C0);
    assert!(d.skel.is("gDaruniaIdleAnim"));
    // EVENTCHKINF_22: EnDu_UpdateTalkState sets it as 0x3020 closes only through his own talk
    // (Npc_UpdateTalking with his textId 0x3020); the script's 0x3020 isn't his talk (his talkState
    // stays idle, his textId 0x301A), so it's never set.
    assert!(!w.save.get_event_chk_inf(EVENTCHKINF_22));
    assert_eq!(du(&w).interact_info.talk_state, NPC_TALK_STATE_IDLE);
    assert_eq!(w.save.cur_upg_value(UPG_STRENGTH), 0);
}

#[test]
fn the_statue_and_the_spear_for_a_child() {
    let Some(a) = assets() else { return };
    let w = by_darunia(&a, |_| {});
    // D_808B90F0: the statue (type 0) for both ages, the spear (type 1) for a child; scale 0.1. The
    // child's statue: func_808B9030 (in place); the spear: func_808B8DD0.
    let objs = spot18_objs(&w);
    assert_eq!(objs.len(), 2);
    let statue = objs.iter().find(|o| bg_spot18_obj::TYPE.get(o.actor.params) == 0).expect("the statue");
    let spear = objs.iter().find(|o| bg_spot18_obj::TYPE.get(o.actor.params) == 1).expect("the spear");
    assert_eq!(statue.action, bg_spot18_obj::Action::Rest);
    assert_eq!(spear.action, bg_spot18_obj::Action::Still);
    assert_eq!(statue.actor.world_pos, STATUE_HOME);
    assert_eq!((statue.actor.scale.x, spear.actor.scale.x), (0.1, 0.1));
    // sInitChain1: gravity -4, minVelocityY -10.
    assert_eq!((statue.actor.gravity, statue.actor.min_velocity_y), (-4.0, -10.0));
    // Their collision (gGoronCityStatueCol, gGoronCityStatueSpearCol) in the DynaPoly: a ray down
    // from above the statue lands on one of them.
    let (_, poly) = w.col.entity_raycast_down(STATUE_HOME + Vec3::new(0.0, 400.0, 0.0));
    let bg = poly.expect("a floor").bg;
    assert!(bg == statue.bg || bg == spear.bg, "the statue's collision under the ray ({bg})");
}

#[test]
fn the_adults_statue_is_pulled_80_along_z_and_sets_its_flag() {
    let Some(a) = assets() else { return };
    let mut w = goron_city(&a, true, |_| {});
    in_room(&mut w, 1, LINK_BY_DARUNIA, -0x8000, 4);
    // The adult layer's statue 0x1100 (flag 0x11, yaw 0); no spear (D_808B90F0[1][1] 0).
    let objs = spot18_objs(&w);
    assert_eq!(objs.len(), 1, "the statue only");
    let s = objs[0];
    assert_eq!(bg_spot18_obj::SWITCH_FLAG.get(s.actor.params), 0x11);
    assert_eq!(s.action, bg_spot18_obj::Action::WaitPull);
    let bg = s.bg;
    let h = live::<BgSpot18Obj>(&w)[0];
    // Pulled (dyna.unk_150 under -0.001, as Player writes it) for 20 frames (unk_168 down from 20,
    // reset by any frame without): func_808B8EE0 on the 20th frame's update.
    let mut frames = 0;
    while w.actors.downcast::<BgSpot18Obj>(h).unwrap().action == bg_spot18_obj::Action::WaitPull {
        w.col.dyna.set_unk_150(bg, -1.0);
        idle(&mut w, 1);
        frames += 1;
        assert!(frames <= 30);
    }
    assert_eq!(frames, 20);
    // Each frame the push is dropped (func_808B8E20).
    assert_eq!(w.col.dyna.unk_150(bg), 0.0);
    // func_808B8F08: speed 0.1 more a frame up to 1.2 along world.rot.y 0 (+z); 80 from home it
    // stops at home + (sin 0, cos 0) * 80, the switch flag 0x11, the chime.
    // Actor_MoveXZGravity moves velocity * 1.5 a frame (Actor_UpdatePos: R_UPDATE_RATE * 0.5):
    // (0.1 + 0.2 + .. + 1.2) * 1.5 = 11.7 in 12 frames, then 1.8 a frame: ceil(68.3 / 1.8) = 38;
    // 50 frames.
    let mut slide = 0;
    while w.actors.downcast::<BgSpot18Obj>(h).unwrap().action == bg_spot18_obj::Action::Slide {
        idle(&mut w, 1);
        slide += 1;
        assert!(slide <= 200);
    }
    assert_eq!(slide, 50);
    let s = w.actors.downcast::<BgSpot18Obj>(h).unwrap();
    assert_eq!(s.action, bg_spot18_obj::Action::Rest);
    assert_eq!((s.actor.world_pos.x, s.actor.world_pos.z), (STATUE_HOME.x, STATUE_HOME.z + 80.0));
    assert!(w.flags.get_switch(0x11));
    assert!(!sfx_frames(&w, NA_SE_SY_CORRECT_CHIME).is_empty());

    // With the flag set the next entry starts it there (func_808B8BB4).
    let mut w = goron_city(&a, true, |s| s.scene_flags[SCENE_GORON_CITY].swch |= 1 << 0x11);
    in_room(&mut w, 1, LINK_BY_DARUNIA, -0x8000, 4);
    let s = spot18_objs(&w)[0];
    assert_eq!(s.action, bg_spot18_obj::Action::Rest);
    assert_eq!((s.actor.world_pos.x, s.actor.world_pos.z), (STATUE_HOME.x, STATUE_HOME.z + 80.0));
}

/// `Math_StepToF`'s frames from `from` to `to` by `step`, in f32 as the C runs it.
fn step_frames(from: f32, to: f32, step: f32) -> usize {
    let mut v = from;
    let mut n = 0;
    loop {
        n += 1;
        v += step;
        if (v - to) * step >= 0.0 {
            return n;
        }
    }
}

#[test]
fn the_lullaby_at_the_song_spot_opens_the_door() {
    let Some(a) = assets() else { return };
    let mut w = goron_city(&a, false, |_| {});
    // Room 3 (the entrance's). The door shut: func_808B95B8, at home.
    let (dh, d) = shutter(&w);
    assert_eq!(d.action, bg_spot18_shutter::Action::WaitSwitch);
    assert_eq!(d.actor.world_pos, SHUTTER_HOME);
    assert_eq!(d.actor.scale.x, 0.1);
    let tag = *live::<EnOkarinaTag>(&w).iter().find(|&&h| w.actors.actor(h).unwrap().params == 0x1C9B).expect("the song spot");
    // Link 40 from the spot (inside 50 + 40 across, 40 high: the spot turns his ocarina to it,
    // unk_6A8), facing the door.
    let (y, _) = w.col.entity_raycast_down(TAG_HOME + Vec3::new(0.0, 100.0, 40.0));
    w.place_player(Vec3::new(TAG_HOME.x, y, TAG_HOME.z + 40.0), -0x8000);
    idle(&mut w, 3);
    assert_eq!(player(&w).unk_6a8, Some(tag));
    // The ocarina out at the spot: PLAYER_STATE2_25; func_80ABEF2C:
    // Message_StartOcarina(OCARINA_ACTION_CHECK_SARIA + 2), Zelda's Lullaby's check.
    ocarina_out(&mut w);
    assert_eq!(w.msg_ctx.ocarina_action, OCARINA_ACTION_CHECK_SARIA + 2);
    play_song(&mut w, OCARINA_SONG_LULLABY);
    until(&mut w, 300, "switch 0x1B", |w| w.flags.get_switch(DOOR_FLAG));
    // The door's next update (it isn't ACTOR_FLAG_UPDATE_DURING_OCARINA: not while Link still holds
    // the ocarina): Actor_SetFocus 70, OnePointCutscene_Attention, func_808B9698.
    let wait = until(&mut w, 120, "the rise", |w| shutter(w).1.action == bg_spot18_shutter::Action::Rise);
    assert!(attention_on(&w, dh));
    let d = shutter(&w).1;
    assert_eq!(d.actor.focus_pos.y, SHUTTER_HOME.y + 70.0);
    // Up 180 at 1.44 a frame from the next update (Math_StepToF in f32), then
    // NA_SE_EV_STONEDOOR_STOP and func_808B95AC.
    assert_eq!(d.actor.world_pos.y, SHUTTER_HOME.y);
    let expected = step_frames(SHUTTER_HOME.y, SHUTTER_HOME.y + 180.0, 1.44);
    let mut frames = 0;
    while shutter(&w).1.action == bg_spot18_shutter::Action::Rise {
        let before = shutter(&w).1.actor.world_pos.y;
        idle(&mut w, 1);
        let after = shutter(&w).1.actor.world_pos.y;
        if shutter(&w).1.action == bg_spot18_shutter::Action::Rise {
            assert!((after - before - 1.44).abs() < 1e-3);
        }
        frames += 1;
        assert!(frames < 400);
    }
    assert_eq!(frames, expected);
    assert_eq!(shutter(&w).1.actor.world_pos.y, SHUTTER_HOME.y + 180.0);
    assert!(!sfx_frames(&w, NA_SE_EV_STONEDOOR_STOP).is_empty());
    // Its collision went up with it: nothing of it left at Link's height in the doorway.
    let (_, poly) = w.col.entity_raycast_down(SHUTTER_HOME + Vec3::new(0.0, 60.0, 0.0));
    assert_ne!(poly.map(|p| p.bg), Some(shutter(&w).1.bg));
    println!("the door: {wait} frames from the flag to its update, {frames} frames to rise (expected {expected})");
}

#[test]
fn with_its_flag_the_door_starts_open() {
    let Some(a) = assets() else { return };
    // Flags_GetSwitch(0x1B) at init: func_808B95AC, 180 up.
    let w = goron_city(&a, false, |s| s.scene_flags[SCENE_GORON_CITY].swch |= 1 << DOOR_FLAG);
    let d = shutter(&w).1;
    assert_eq!(d.action, bg_spot18_shutter::Action::Idle);
    assert_eq!(d.actor.world_pos.y, SHUTTER_HOME.y + 180.0);
}

#[test]
fn the_dance_layer_starts_the_dancing_script() {
    let Some(a) = assets() else { return };
    // EnDu_Init with gSaveContext.save.cutsceneIndex >= CS_INDEX_0 (0xFFF0: layer 4, whose entrance
    // list 0 is spawn 0 in room 1): gGoronCityDaruniaDancingCs, cutsceneTrigger 1, func_809FE890.
    let w = goron_city(&a, false, |s| s.cutscene_index = 0xFFF0);
    assert_eq!(w.save.scene_layer, 4);
    let d = du(&w);
    assert_eq!(d.action, Action::Func809FE890);
    assert_eq!(script_name(&w).as_deref(), Some("gGoronCityDaruniaDancingCs"));
    assert_ne!(w.cs_ctx.state, CS_STATE_IDLE);
}

/// `ENTR_FIRE_TEMPLE_0` for an adult, put in room 2 at its spawn 1 (-1478, 200, -5) facing +x.
fn fire_temple(a: &Arc<GameAssets>, setup: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_FIRE_TEMPLE_0").expect("entrance");
    let mut save = SaveContext::new(e, true, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("ranch-epona").unwrap();
    setup(&mut save);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    oot_actors::playthrough::deku_tree_room_start(&mut w, 2, Vec3::new(-1478.0, 200.0, -5.0), 0x4000);
    w
}

#[test]
fn the_fire_temple_meeting_opens_the_way_and_he_leaves() {
    let Some(a) = assets() else { return };
    let mut w = fire_temple(&a, |_| {});
    // The first update with Link free (PLAYER_STATE1_29 clear): OnePointCutscene_Init(3330) (not
    // ported: logged), Link turned to world.rot.y + 0x7FFF (0x4000 + 0x7FFF), the fanfare, 50
    // frames (func_809FE6CC).
    until(&mut w, 30, "the meeting", |w| darunia(w).is_some_and(|(_, d)| d.action == Action::Func809FE6CC));
    let home_x = du(&w).actor.home_pos.x;
    assert_eq!(home_x, -1423.0);
    assert_eq!(player(&w).actor.shape_rot.y, 0x4000i16.wrapping_add(0x7FFF));
    // DECR(unk_1E2) from 50: 0x3039 on the 50th update, talking (func_809FE740).
    let n = until(&mut w, 60, "0x3039", |w| du(w).action == Action::Func809FE740);
    assert_eq!(n, 50);
    assert_eq!(w.msg_ctx.text_id, 0x3039);
    // Read: EnDu_UpdateTalkState's close: idle: Camera_SetFinishedFlag, 90 frames (func_809FE798).
    let mut texts = Vec::new();
    answer_until(&mut w, 1000, "0x3039 read", &mut texts, |w| du(w).action == Action::Func809FE798);
    assert_eq!(texts, [0x3039]);
    // The door: 10 a frame along -x while 61 or more are left after DECR (89 down to 61: 29
    // frames); at 0 (the 90th update) gone and INFTABLE_11A.
    let h = darunia(&w).unwrap().0;
    let mut frames = 0;
    let mut min_x = f32::MAX;
    while w.actors.downcast::<EnDu>(h).is_some_and(|d| !d.actor.killed) {
        idle(&mut w, 1);
        frames += 1;
        if let Some(d) = w.actors.downcast::<EnDu>(h) {
            min_x = min_x.min(d.actor.world_pos.x);
        }
        assert!(frames < 200);
    }
    assert_eq!(frames, 90);
    assert_eq!(min_x, home_x - 290.0);
    assert!(w.save.get_inf_table(INFTABLE_11A));

    // With INFTABLE_11A he isn't there.
    let w = fire_temple(&a, |s| s.set_inf_table(INFTABLE_11A));
    assert!(darunia(&w).is_none());
}

#[test]
fn sarias_song_again_with_the_bracelet_ends_with_0x301f() {
    let Some(a) = assets() else { return };
    let mut w = by_darunia(&a, |s| {
        item_give(s, None, ITEM_STRENGTH_GORONS_BRACELET);
    });
    ocarina_out(&mut w);
    play_song(&mut w, OCARINA_SONG_SARIAS);
    until(&mut w, 300, "the right song's script", |w| du(w).action == Action::Func809FE890);
    let mut texts = Vec::new();
    answer_until(&mut w, 3000, "the script's end", &mut texts, |w| du(w).action != Action::Func809FE890);
    // func_809FEB08 with the bracelet: 0x301F, func_809FE3C0 straight away; its close is his action
    // (NPC_TALK_STATE_ACTION), which func_809FE3C0 turns into Link let go and idle.
    idle(&mut w, 1);
    assert_eq!((du(&w).action, du(&w).actor.text_id), (Action::Func809FE3C0, 0x301F));
    assert_eq!(w.msg_ctx.text_id, 0x301F);
    let mut texts = Vec::new();
    answer_until(&mut w, 2000, "0x301F read", &mut texts, |w| w.message_state() == TEXT_STATE_NONE && du(w).interact_info.talk_state == NPC_TALK_STATE_IDLE);
    assert_eq!(texts, [0x301F]);
    assert_eq!(w.save.cur_upg_value(UPG_STRENGTH), 1, "no second bracelet");
}
