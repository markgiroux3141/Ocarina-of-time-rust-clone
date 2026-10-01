//! The sound effects (`code_800F7260.c`, `code_800EC960.c`'s sound effect parts), headless with
//! the audio library offline, against the C:
//! - walking: `func_8084029C` plays a footstep (`func_808327F8`: `NA_SE_PL_WALK_GROUND` + the
//!   floor's `D_80119E10` offset + the age's `unk_94`, through `func_800F4010` at Player's
//!   `projectedPos`) each time the walk phase `unk_868` crosses 10 or 24 of its 29
//!   (`func_8084021C`), and each one sounds: the request goes into the player bank, a channel
//!   of the sound effects' sequence is started on it (port 0 to 1, port 4 the sound's index),
//!   and the sequence strikes its notes;
//! - the message box: `Message_DrawText` plays `NA_SE_SY_MESSAGE_END` as a text with the
//!   default ending finishes, and `Message_Update` `NA_SE_SY_DECIDE` as A closes it.

mod common;

use std::sync::Arc;

use eng_audio::GameOp;
use eng_input::pad::{BTN_A, PadState};
use oot_game::audio::offline::OfflineAudio;
use oot_game::audio::sfx::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::surface::SurfaceType;

fn assets() -> Option<(Arc<GameAssets>, eng_audio::AudioData)> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    let data = pack.audio_data().expect("the pack's audio data");
    Some((oot_actors::game_assets(pack).expect("the pack's tables"), data))
}

/// Kokiri Forest at 10:00 with the audio offline beside it and the log on.
fn forest(a: &Arc<GameAssets>, data: &eng_audio::AudioData) -> Option<PlayState> {
    let e = a.scenes.entrance_index("ENTR_SPOT04_0").expect("entrance");
    let save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), common::data()?, common::rules()?, save, audio).expect("Play_Init");
    w.audio_side = Some(Box::new(OfflineAudio::new(data, false)));
    Some(w)
}

fn offline(w: &PlayState) -> &OfflineAudio {
    w.audio_side.as_ref().unwrap().as_any().downcast_ref::<OfflineAudio>().unwrap()
}

/// `func_8084021C`: whether the phase `arg0` crosses `arg3` stepping by `arg1` in a cycle of
/// `arg2`.
fn func_8084021c(arg0: f32, arg1: f32, arg2: f32, mut arg3: f32) -> bool {
    if arg3 == 0.0 && arg1 > 0.0 {
        arg3 = arg2;
    }
    let t = (arg0 + arg1) - arg3;
    t * arg1 >= 0.0 && (t - arg1) * arg1 < 0.0
}

#[test]
fn walking_plays_footsteps_where_the_c_does() {
    let Some((a, data)) = assets() else { return };
    let Some(mut w) = forest(&a, &data) else { return };
    let idle = scripted_input(PadState::default(), PadState::default());
    for _ in 0..30 {
        w.tick_with(idle);
    }
    let notes_before = offline(&w).renderer.ctx.stats.notes_struck;
    let first = w.audio.frames + 1;
    let fwd = PadState { button: 0, stick_x: 0, stick_y: 80 };
    let mut prev = PadState::default();
    // The walk phase before and after each frame, and the footstep's id for the floors Link
    // stood on at either end of it (unk_89E comes from the last bg check, func_80847BA0).
    let mut phases = Vec::new();
    let mut actions = Vec::new();
    let mut floor_ids = std::collections::HashMap::new();
    let id_of = |w: &PlayState| {
        let p = oot_actors::PlayExt::player(w);
        let t = p.actor.floor_poly.map(|f| w.col.sfx_type(f)).unwrap_or(0);
        NA_SE_PL_WALK_GROUND + a.audio.surface_sfx_id(t) + p.age.climb.unk_94
    };
    for _ in 0..60 {
        let before = oot_actors::PlayExt::player(&w).unk_868;
        let id0 = id_of(&w);
        w.tick_with(scripted_input(prev, fwd));
        prev = fwd;
        let after = oot_actors::PlayExt::player(&w).unk_868;
        phases.push((w.audio.frames, before, after));
        actions.push((w.audio.frames, format!("{:?}", oot_actors::PlayExt::player(&w).action)));
        floor_ids.insert(w.audio.frames, [id0, id_of(&w)]);
    }
    let log = w.audio.log.as_ref().unwrap();
    let steps: Vec<(u32, u16)> = log.sfx.iter().filter(|(f, id, _)| *f >= first && sfx_bank(*id) == BANK_PLAYER && (*id & 0x7F0) < 0x10).map(|&(f, id, _)| (f, id)).collect();
    assert!(steps.len() >= 4, "footsteps: {steps:x?}");
    // The id: NA_SE_PL_WALK_GROUND + D_80119E10[the floor's sfx type] + ageProperties->unk_94.
    for &(f, id) in &steps {
        assert!(floor_ids[&f].contains(&id), "the footstep on frame {f}: {id:#x}, the floors {:x?}", floor_ids[&f]);
    }
    let want = steps.last().unwrap().1;
    // The footsteps come on exactly the frames where the phase crossed 10 or 24.
    let crossings: Vec<u32> = phases
        .iter()
        .filter(|&&(_, before, after)| {
            let mut step = after - before;
            if step < -14.5 {
                step += 29.0;
            }
            step != 0.0 && (func_8084021c(before, step, 29.0, 10.0) || func_8084021c(before, step, 29.0, 24.0))
        })
        .map(|&(f, _, _)| f)
        .collect();
    // (Only while Link runs: past the forest's ledge he catches it, and the grab plays
    // func_8084BBE4's func_80832770(NA_SE_PL_WALK_GROUND) too.)
    let running: std::collections::HashSet<u32> = actions.iter().filter(|(_, a)| a == "Run").map(|(f, _)| *f).collect();
    let step_frames: Vec<u32> = steps.iter().map(|&(f, _)| f).filter(|f| running.contains(f)).collect();
    let crossings: Vec<u32> = crossings.into_iter().filter(|f| running.contains(f)).collect();
    assert!(step_frames.len() >= 4);
    assert_eq!(step_frames, crossings, "actions {:?}", &actions[actions.len() - 6..]);
    // They sound: the sound effects' sequence takes them on its channels (port 0 to 1, port 4
    // the index) and strikes notes past the music's.
    let starts = log.ops.iter().filter(|(f, op)| *f >= first && matches!(op, GameOp::Cmd(a, d) if a & 0xFFFF_00FF == 0x0602_0004 && (*d >> 24) as u16 == want & 0xFF)).count();
    let kinds: std::collections::BTreeSet<u16> = steps.iter().map(|&(_, id)| id).collect();
    assert!(starts >= 1, "a channel started on the footstep");
    for _ in 0..10 {
        w.tick_with(idle);
    }
    let notes = offline(&w).renderer.ctx.stats.notes_struck - notes_before;
    // The same frames standing still: only the music's notes.
    let mut still = forest(&a, &data).unwrap();
    for _ in 0..30 {
        still.tick_with(idle);
    }
    let still_before = offline(&still).renderer.ctx.stats.notes_struck;
    for _ in 0..70 {
        still.tick_with(idle);
    }
    let still_notes = offline(&still).renderer.ctx.stats.notes_struck - still_before;
    eprintln!("{} footsteps ({kinds:x?}), {starts} channel starts for {want:#x}; {notes} notes struck walking, {still_notes} standing", steps.len());
    assert!(notes >= still_notes + steps.len() as u64, "each footstep strikes a note of the sound effects' sequence");
}

#[test]
fn the_message_box_sounds_its_end_and_its_close() {
    let Some((a, data)) = assets() else { return };
    let Some(mut w) = forest(&a, &data) else { return };
    let idle = scripted_input(PadState::default(), PadState::default());
    for _ in 0..30 {
        w.tick_with(idle);
    }
    let first = w.audio.frames + 1;
    w.start_textbox(0x1005, None);
    // Through the boxes: A at each box break (TEXT_STATE_AWAITING_NEXT), on its own frame.
    let a_press = PadState { button: BTN_A, stick_x: 0, stick_y: 0 };
    let mut done_frame = None;
    let mut breaks = 0;
    let mut prev = PadState::default();
    for _ in 0..800 {
        let st = w.message_state();
        let pad = if st == oot_game::message::TEXT_STATE_AWAITING_NEXT && prev.button == 0 { a_press } else { PadState::default() };
        if pad.button != 0 {
            breaks += 1;
        }
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        if w.message_state() == oot_game::message::TEXT_STATE_DONE {
            done_frame = Some(w.audio.frames);
            break;
        }
    }
    let done_frame = done_frame.expect("the text finishes");
    let sfx = |w: &PlayState| -> Vec<(u32, u16)> { w.audio.log.as_ref().unwrap().sfx.iter().filter(|(f, id, _)| *f >= first && *id != 0).map(|&(f, id, _)| (f, id)).collect() };
    let s = sfx(&w);
    // Each A at a box break: Message_ShouldAdvance's NA_SE_SY_MESSAGE_PASS.
    assert!(breaks >= 1);
    assert_eq!(s.iter().filter(|(_, id)| *id == NA_SE_SY_MESSAGE_PASS).count(), breaks, "{s:x?}");
    assert!(w.msg_ctx.textbox_end_type == oot_game::message::TEXTBOX_ENDTYPE_DEFAULT, "a text with the default ending");
    assert_eq!(s.iter().filter(|(_, id)| *id == NA_SE_SY_MESSAGE_END).count(), 1, "{s:x?}");
    assert!(s.iter().any(|&(f, id)| id == NA_SE_SY_MESSAGE_END && f <= done_frame));
    // A closes it: NA_SE_SY_DECIDE (Message_ShouldAdvanceSilent, then the sound).
    w.tick_with(scripted_input(PadState::default(), a_press));
    let s = sfx(&w);
    assert!(s.iter().any(|&(_, id)| id == NA_SE_SY_DECIDE), "{s:x?}");
    assert_eq!(w.msg_ctx.msg_mode, oot_game::message::MSGMODE_TEXT_CLOSING);
}
