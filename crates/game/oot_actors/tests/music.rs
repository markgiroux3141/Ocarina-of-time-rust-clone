//! The scenes' music, headless, against the C (`z_scene.c`, `z_kankyo.c`, `general.c`,
//! `sequence.c`), through the audio library offline (`oot_game::audio::offline`):
//! - entering Kokiri Forest from a new game (`gSaveContext.seqId` disabled, 10:00): its sound
//!   settings (spec 1, `NATURE_ID_KOKIRI_REGION`, `NA_BGM_KOKIRI`) queue the spec change
//!   (`0xF0000001`), and `Environment_PlaySceneSequence`, by day with an ambience, the music
//!   (`Audio_PlaySceneSequence`: port 7 to 0xFF, `sSeqFlags[NA_BGM_GENERAL_SFX]` lacking `SEQ_FLAG_RESUME_PREV`,
//!   then the start); `Audio_InitSound`'s commands come before (`0x46` port 0 to -1, the sound
//!   effects' sequence 0 on player 2 with a fade of `(10 * 3) / 4`), and the title's sound mode
//!   (`Audio_SetSoundOutputMode(0)`: `0xE0000000`, stereo);
//! - the first `Audio_Update` turns those into the library's commands (the sound mode, the
//!   reset to spec 1 and its `0xF8`, port 7, `0x82` for sequence 0x3C on player 0), and
//!   `Audio_UpdateActiveSequences` the four players' full volume (`Audio_ResetActiveSequences` asked for it);
//! - `Audio_Update` waits while the audio side resets (`D_80133418`), then restarts the sound
//!   effects' sequence (`func_800FAD34`, `func_800F7170`) and lets the volume commands through;
//! - Kokiri Forest's sequence then plays on player 0, sounds, and plays on past its loop;
//! - from Link's house (`NA_BGM_LINK_HOUSE`, `SEQ_FLAG_RESUME_PREV`) into Kokiri Forest (`SEQ_FLAG_RESUME`),
//!   the exit fades every player out (`func_800F6964(0x14)`) and the forest's music starts
//!   from the position the house kept (`sSeqResumePoint`, port 7).

mod common;

use std::sync::Arc;

use eng_audio::GameOp;
use oot_game::audio::offline::OfflineAudio;
use oot_game::audio::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<(Arc<GameAssets>, eng_audio::AudioData)> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    let data = pack.audio_data().expect("the pack's audio data");
    Some((oot_actors::game_assets(pack).expect("the pack's tables"), data))
}

/// `Play_Init` by `entrance` at 10:00, the audio's game side booted with its log on.
fn enter(a: &Arc<GameAssets>, entrance: &str, f: impl FnOnce(&mut SaveContext)) -> Option<PlayState> {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    f(&mut save);
    let audio = GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    Some(PlayState::play_init_with(a.clone(), common::data()?, common::rules()?, save, audio).expect("Play_Init"))
}

fn seq_cmds(w: &PlayState, frame: u32) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().seq_cmds.iter().filter(|(f, _)| *f == frame).map(|(_, c)| *c).collect()
}

fn ops(w: &PlayState, frame: u32) -> Vec<GameOp> {
    w.audio.log.as_ref().unwrap().ops.iter().filter(|(f, _)| *f == frame).map(|(_, o)| *o).collect()
}

fn hex(v: &[u32]) -> Vec<String> {
    v.iter().map(|c| format!("{c:08X}")).collect()
}

#[test]
fn kokiri_forest_starts_its_music_and_loops_it() {
    let Some((a, data)) = assets() else { return };
    let Some(mut w) = enter(&a, "ENTR_KOKIRI_FOREST_0", |_| {}) else { return };
    let mut audio = OfflineAudio::new(&data, true);

    // Play_Init: the scene's settings, then the music.
    assert_eq!(w.sequence_ctx.seq_id as u16, NA_BGM_KOKIRI);
    assert_eq!(w.sequence_ctx.nature_ambience_id, NATURE_ID_KOKIRI_REGION);
    assert_eq!((w.save.seq_id as u16, w.save.nature_ambience_id), (NA_BGM_KOKIRI, NATURE_ID_KOKIRI_REGION));
    assert_eq!(hex(&seq_cmds(&w, 0)), ["E0000000", "F0000001", "700700FF", "0000003C"]);
    assert_eq!(w.time_seq_state, scene::TIMESEQ_FADE_DAY_BGM);
    // Audio_InitSound: func_800F6C34's port, the sound effects' sequence (fade 10 frames:
    // (10 * updatesPerFrame 3) / 4).
    assert_eq!(ops(&w, 0), [GameOp::Cmd(0x4600_0000, 0xFF00_0000), GameOp::Cmd(0x8202_0000, 7)]);

    // The first Audio_Update.
    w.tick_with(scripted_input(Default::default(), Default::default()));
    assert_eq!(
        ops(&w, 1),
        [
            GameOp::Cmd(0xF000_0000, SOUND_OUTPUT_STEREO as u32),
            GameOp::ResetSpec(1),
            GameOp::Cmd(0x4600_0000, 0xFF00_0000),
            GameOp::Cmd(0xF800_0000, 0),
            GameOp::Cmd(0x4600_0007, 0xFF00_0000),
            GameOp::Cmd(0x8200_3C00, 0),
            GameOp::Schedule,
        ]
    );
    assert_eq!(hex(&seq_cmds(&w, 1)), ["4001007F", "4101007F", "4201007F", "4301007F"]);
    assert_eq!(w.audio.d_80133418, 1);
    audio.frame(&mut w);

    // While the audio side resets, Audio_Update does nothing.
    let mut frame = 1;
    loop {
        let reset_done = !w.audio.view.reset_msgs.is_empty();
        w.tick_with(scripted_input(Default::default(), Default::default()));
        frame += 1;
        if reset_done {
            break;
        }
        assert!(ops(&w, frame).is_empty(), "frame {frame}: {:?}", ops(&w, frame));
        audio.frame(&mut w);
        assert!(frame < 20, "the reset to spec 1 never finished");
    }
    // func_800FAD34 (gSfxChannelLayout 0), func_800F7170 (fade 1: (1 * updatesPerFrame) / 4),
    // then the rest of the update: the volume commands change nothing (already 1.0). The sound
    // effects' channel commands of that update (player 2, ops 0x01..0x0F) are the forest's
    // small waterfall starting (En_River_Sound, RS_SMALL_WATERFALL: NA_SE_EV_WATER_WALL -
    // SFX_FLAG asked for by every draw but its first), the only sound asked for so far.
    let upf = audio.renderer.ctx.audio_buffer_parameters.updates_per_frame;
    assert_eq!(audio.renderer.ctx.audio_reset_spec_id_to_load, 1);
    let (sfx_ops, music_ops): (Vec<GameOp>, Vec<GameOp>) = ops(&w, frame).into_iter().partition(|o| matches!(o, GameOp::Cmd(c, _) if (c >> 16) & 0xFF == 2 && (0x01..=0x0F).contains(&(c >> 24))));
    assert_eq!(music_ops, [GameOp::Cmd(0x4602_0000, 0), GameOp::Cmd(0x8202_0000, (upf as u32) / 4), GameOp::Cmd(0xF200_0000, 1), GameOp::Schedule, GameOp::Cmd(0xF800_0000, 0), GameOp::Schedule,]);
    assert!(!sfx_ops.is_empty());
    let asked: Vec<u16> = w.audio.log.as_ref().unwrap().sfx.iter().filter(|(_, id, _)| *id != 0).map(|(_, id, _)| *id).collect();
    let waterfall = oot_game::audio::sfx::NA_SE_EV_WATER_WALL - oot_game::audio::sfx::SFX_FLAG;
    assert!(!asked.is_empty() && asked.iter().all(|&id| id == waterfall), "{asked:04X?}");
    assert_eq!(w.audio.d_80133418, 0);
    audio.frame(&mut w);
    eprintln!("the reset to spec 1 took {} game frames; spec 1: {upf} updates a frame", frame - 1);

    // A few frames on, the forest's music plays on player 0 and the sound effects' on 2.
    audio.run_idle(&mut w, 5);
    assert_eq!(audio.playing(0), Some(NA_BGM_KOKIRI as u8));
    assert_eq!(audio.playing(2), Some(0));
    assert_eq!(w.audio.audio_get_active_seq_id(SEQ_PLAYER_BGM_MAIN), NA_BGM_KOKIRI);

    // It sounds, and plays on past its loop (5952 ticks, about 51 s at its tempo).
    let notes_before = audio.renderer.ctx.stats.notes_struck;
    audio.run_idle(&mut w, 20 * 60);
    let ticks = audio.renderer.ctx.stats.seq_ticks[0];
    assert!(ticks > 5952, "{ticks} ticks");
    eprintln!("Kokiri Forest after {} game frames: {ticks} ticks, {} notes", w.audio.frames, audio.renderer.ctx.stats.notes_struck);
    assert_eq!(audio.playing(0), Some(NA_BGM_KOKIRI as u8));
    assert!(audio.renderer.ctx.stats.notes_struck > notes_before + 600);
    let out = &audio.renderer.out;
    let rate = audio.renderer.output_rate() as usize;
    let last5: &[i16] = &out[out.len() - 2 * 5 * rate..];
    let peak = last5.iter().map(|v| v.unsigned_abs()).max().unwrap();
    assert!(peak > 2000, "the last 5 s peak at {peak}");
    // Player stood still all along: SEQ_MODE_STILL, but Kokiri Forest's music has
    // SEQ_FLAG_ENEMY, so Audio_SetSequenceMode only remembers it (no port 2 commands).
    assert!(w.audio.log.as_ref().unwrap().seq_cmds.iter().all(|(_, c)| c & 0xFFFF_FF00 != 0x7002_0000));
}

#[test]
fn from_links_house_the_forest_resumes_its_music() {
    let Some((a, data)) = assets() else { return };
    let Some(mut w) = enter(&a, "ENTR_LINKS_HOUSE_0", |_| {}) else { return };
    let mut audio = OfflineAudio::new(&data, false);
    // Link's house: spec 5, no ambience, NA_BGM_LINK_HOUSE: Audio_PlaySceneSequence straight.
    assert_eq!(hex(&seq_cmds(&w, 0)), ["E0000000", "F0000005", "700700FF", "0000001F"]);
    assert_eq!(w.audio.seq_resume_point, 0, "SEQ_FLAG_RESUME_PREV: sSeqResumePoint is left alone");
    audio.run_idle(&mut w, 30);
    assert_eq!(audio.playing(0), Some(0x1F));

    // Out of the door: Player's exit (the house's exit 0 to Kokiri Forest).
    let exit = w.exit_list()[0];
    w.transition.next_entrance_index = exit;
    w.transition.trigger = oot_game::transition::TRANS_TRIGGER_START;
    w.transition.ty = oot_game::transition::TRANS_TYPE_FADE_BLACK;
    let changes = w.scene_changes;
    let start = w.audio.frames;
    for _ in 0..200 {
        w.tick_with(scripted_input(Default::default(), Default::default()));
        audio.frame(&mut w);
        if w.scene_changes != changes {
            break;
        }
    }
    assert_ne!(w.scene_changes, changes, "never left the house");
    let log: Vec<u32> = w.audio.log.as_ref().unwrap().seq_cmds.iter().filter(|(f, _)| *f > start).map(|(_, c)| *c).collect();
    // func_800F6964(0x14): players 0 and 1 out over 30, every sound effects channel but the
    // ocarina's over 10 (spec 5 isn't 10, so the system's go too), player 3 over 30. Player's
    // Audio_SetSequenceMode queues port 2 every frame here (NA_BGM_LINK_HOUSE lacks
    // SEQ_FLAG_ENEMY): those are left out.
    let log: Vec<u32> = log.into_iter().filter(|c| c & 0xFFFF_FF00 != 0x7002_0000).collect();
    let mut fade = vec![0x101E_00FF, 0x111E_00FF];
    for c in 0..16u32 {
        if c != SFX_CHANNEL_OCARINA as u32 {
            fade.push(0x620A_0000 | (c << 8));
        }
    }
    fade.push(0x131E_00FF);
    assert_eq!(hex(&log[..fade.len()]), hex(&fade));
    // The forest: seqId was disabled by the exit, so the spec change; then Audio_PlaySceneSequence from
    // NA_BGM_LINK_HOUSE (SEQ_FLAG_RESUME_PREV) to NA_BGM_KOKIRI (SEQ_FLAG_RESUME): port 7 to sSeqResumePoint (0:
    // the house never set it), fade 0 since sSeqResumePoint & 0x3F is 0.
    assert_eq!(w.scene_id, oot_game::play_scene::SCENE_KOKIRI_FOREST);
    let after: Vec<u32> = log[fade.len()..].to_vec();
    let i = after.iter().position(|&c| c == 0xF000_0001).expect("the spec change");
    assert_eq!(hex(&after[i..i + 3]), ["F0000001", "70070000", "0000003C"]);
    audio.run_idle(&mut w, 40);
    assert_eq!(audio.playing(0), Some(NA_BGM_KOKIRI as u8));
}

/// Kokiri Forest at 20:00: `Environment_PlaySceneSequence` plays its nature ambience
/// (`Audio_PlayNatureAmbienceSequence(NATURE_ID_KOKIRI_REGION)`) rather than the music, and
/// `TIMESEQ_NIGHT_CRITTERS` sets the night's critters going the next frame.
#[test]
fn kokiri_forest_at_night_plays_its_ambience() {
    let Some((a, data)) = assets() else { return };
    let e = a.scenes.entrance_index("ENTR_KOKIRI_FOREST_0").unwrap();
    let save = SaveContext::new(e, false, oot_game::env::clock_time(20, 0) as u16);
    let audio = GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let Some((d, r)) = common::data().zip(common::rules()) else { return };
    let mut w = PlayState::play_init_with(a.clone(), d, r, save, audio).expect("Play_Init");
    let mut audio = OfflineAudio::new(&data, true);
    assert_eq!(w.time_seq_state, scene::TIMESEQ_NIGHT_CRITTERS, "after 19:00");

    // Audio_StartNatureAmbienceSequence(playerIO, channelMask), then the triples, then the
    // sound mode on NATURE_CHANNEL_UNK's port 7 (stereo, as the title left it).
    let n = &a.audio.nature_ambience[NATURE_ID_KOKIRI_REGION as usize];
    let mut want = vec![0xE000_0000, 0xF000_0001, 0x7000_0001, 0x7004_0000 | (n.player_io >> 8) as u32, 0x7005_0000 | (n.player_io & 0xFF) as u32, 0x0000_0001];
    for c in 0..16u32 {
        if n.channel_mask & (1 << c) == 0 && n.player_io & (1 << c) != 0 {
            want.push(0x8001_0001 | (c << 8));
        }
    }
    for t in n.channel_io.chunks(3).take_while(|t| t[0] != 0xFF) {
        want.push(0x8000_0000 | ((t[1] as u32) << 16) | ((t[0] as u32) << 8) | t[2] as u32);
    }
    want.push(0x8007_0D00 | SOUND_OUTPUT_STEREO as u32);
    assert_eq!(hex(&seq_cmds(&w, 0)), hex(&want));

    // Frame 1, TIMESEQ_NIGHT_CRITTERS: critter 0 off, critters 1 to 3 on. The ambience's start
    // is still queued (Audio_IsSeqCmdNotQueued finds 0x00000001), so the IO commands go ahead.
    w.tick_with(scripted_input(Default::default(), Default::default()));
    let f1 = seq_cmds(&w, 1);
    assert_eq!(hex(&f1[..4]), ["80010100", "80010201", "80010301", "80010401"]);
    assert_eq!(w.time_seq_state, scene::TIMESEQ_DAY_BEGIN_SFX, "and it waits for the morning");
    audio.frame(&mut w);

    audio.run_idle(&mut w, 20 * 20);
    assert_eq!(audio.playing(0), Some(NA_BGM_NATURE_AMBIENCE as u8));
    assert_eq!(w.audio.audio_get_active_seq_id(SEQ_PLAYER_BGM_MAIN), NA_BGM_NATURE_AMBIENCE);
    // The critters' channels hold the types they were given (port 2); port 1, their on and
    // off, the script has read, which resets it (seqplayer.c:1680, ports 0 and 1).
    let ch = |c: usize, port: usize| audio.renderer.ctx.channels[audio.renderer.ctx.seq_players[0].channels[c]].sound_script_io[port];
    for t in n.channel_io.chunks(3).take_while(|t| t[0] != 0xFF).filter(|t| t[1] == 2) {
        assert_eq!(ch(t[0] as usize, 2), t[2] as i8, "channel {}'s type", t[0]);
    }
    assert_eq!((ch(1, 1), ch(2, 1), ch(3, 1), ch(4, 1)), (-1, -1, -1, -1));
    let out = &audio.renderer.out;
    let peak = out[out.len() / 2..].iter().map(|v| v.unsigned_abs()).max().unwrap();
    assert!(peak > 200, "the night sounds: peak {peak}");
    eprintln!("Kokiri Forest's night: {} notes struck in 20 s, peak {peak}", audio.renderer.ctx.stats.notes_struck);
}
