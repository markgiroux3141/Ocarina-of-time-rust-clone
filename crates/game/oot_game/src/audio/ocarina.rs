//! The ocarina's audio (`src/audio/game/general.c`'s `AudioOcarina_*`, docs/adr/0061-the-ocarina.md).
//!
//! Its statics are the code segment's, so they live in `GameAudio` (`ocarina`). The pad is read
//! where `AudioOcarina_ReadControllerInput` reads it (`PadMgr_RequestPadData` with
//! `gameRequest` false: the newest state, not the game's frame's presses), from `Ocarina::pad`,
//! which the play state sets before `Audio_Update`. A note is the sound effect
//! `NA_SE_OC_OCARINA` on `SFX_CHANNEL_OCARINA`; the sequence's channel reads its instrument
//! (port 7), pitch (port 5) and vibrato (port 6) from the channel's IO ports, which the game sets
//! with `AUDIOCMD_CHANNEL_SET_IO`. The playback's timing counts the audio side's tasks
//! (`gAudioCtx.totalTaskCount`, three a 20 Hz frame), as the C does.
//!
//! The song tables (`sOcarinaSongNotes`, `gOcarinaSongButtons`, `sButtonToPitchMap`,
//! `sPitchToButtonMap`) are the pack's (`AudioGameTables`), read from the C; `sOcarinaSongNotes`
//! is copied into the statics at boot, as the scarecrow's songs and the memory game write it.
//!
//! The Skull Kids' memory game (`AudioOcarina_MemoryGameInit`, `_MemoryGameNextNote`) came with
//! GAME-08 milestone 2; the scarecrows' recording (`AudioOcarina_SetRecordingSong`,
//! `_SetRecordingState`, `_RecordSong`) and the long scarecrow's song after the credits
//! (`AudioOcarina_PlayLongScarecrowSong`) with GAME-10 milestone 9. `Sram_OpenSave`'s copies of
//! the save's songs into the statics are `Ocarina::sram_open_save_scarecrow_songs`, called where a
//! play state starts on a loaded file.

use eng_input::pad::{BTN_A, BTN_B, BTN_CDOWN, BTN_CLEFT, BTN_CRIGHT, BTN_CUP, BTN_L, BTN_R, BTN_Z, Input};

use super::GameAudio;
use super::bgm::seq_cmd8;
use super::sfx::{NA_SE_OC_OCARINA, SfxF32, SfxPos, SfxS8};
use super::{SEQ_PLAYER_SFX, SFX_CHANNEL_OCARINA};

/// `OcarinaSongId` (`ocarina.h`).
pub const OCARINA_SONG_MINUET: u8 = 0;
pub const OCARINA_SONG_BOLERO: u8 = 1;
pub const OCARINA_SONG_SERENADE: u8 = 2;
pub const OCARINA_SONG_REQUIEM: u8 = 3;
pub const OCARINA_SONG_NOCTURNE: u8 = 4;
pub const OCARINA_SONG_PRELUDE: u8 = 5;
pub const OCARINA_SONG_SARIAS: u8 = 6;
pub const OCARINA_SONG_EPONAS: u8 = 7;
pub const OCARINA_SONG_LULLABY: u8 = 8;
pub const OCARINA_SONG_SUNS: u8 = 9;
pub const OCARINA_SONG_TIME: u8 = 10;
pub const OCARINA_SONG_STORMS: u8 = 11;
pub const OCARINA_SONG_SCARECROW_SPAWN: u8 = 12;
pub const OCARINA_SONG_MEMORY_GAME: u8 = 13;
pub const OCARINA_SONG_MAX: u8 = 14;
pub const OCARINA_SONG_SCARECROW_LONG: u8 = OCARINA_SONG_MAX;

/// `OcarinaButtonIndex` (`ocarina.h`).
pub const OCARINA_BTN_A: u8 = 0;
pub const OCARINA_BTN_C_DOWN: u8 = 1;
pub const OCARINA_BTN_C_RIGHT: u8 = 2;
pub const OCARINA_BTN_C_LEFT: u8 = 3;
pub const OCARINA_BTN_C_UP: u8 = 4;
pub const OCARINA_BTN_C_RIGHT_OR_C_LEFT: u8 = 5;
pub const OCARINA_BTN_INVALID: u8 = 0xFF;

/// `OcarinaPitch` (`ocarina.h`), the ones the code names.
pub const OCARINA_PITCH_C4: u8 = 0x0;
pub const OCARINA_PITCH_D4: u8 = 0x2;
pub const OCARINA_PITCH_F4: u8 = 0x5;
pub const OCARINA_PITCH_A4: u8 = 0x9;
pub const OCARINA_PITCH_BFLAT4: u8 = 0xA;
pub const OCARINA_PITCH_B4: u8 = 0xB;
pub const OCARINA_PITCH_D5: u8 = 0xE;
pub const OCARINA_PITCH_EFLAT5: u8 = 0xF;
pub const OCARINA_PITCH_NONE: u8 = 0xFF;

/// `OcarinaInstrumentId` (`ocarina.h`).
pub const OCARINA_INSTRUMENT_OFF: u8 = 0;
pub const OCARINA_INSTRUMENT_DEFAULT: u8 = 1;
pub const OCARINA_INSTRUMENT_MALON: u8 = 2;
pub const OCARINA_INSTRUMENT_WHISTLE: u8 = 3;
pub const OCARINA_INSTRUMENT_HARP: u8 = 4;
pub const OCARINA_INSTRUMENT_GRIND_ORGAN: u8 = 5;
pub const OCARINA_INSTRUMENT_FLUTE: u8 = 6;

/// `OcarinaRecordingState` (`ocarina.h`).
pub const OCARINA_RECORD_OFF: u8 = 0;
pub const OCARINA_RECORD_SCARECROW_LONG: u8 = 1;
pub const OCARINA_RECORD_SCARECROW_SPAWN: u8 = 2;
pub const OCARINA_RECORD_REJECTED: u8 = 0xFF;

/// `OcarinaNote` (`ocarina.h`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OcarinaNote {
    /// Semitones above middle C (`OCARINA_PITCH_*`).
    pub pitch: u8,
    /// Frames (audio tasks) the note is held.
    pub length: u16,
    pub volume: u8,
    pub vibrato: u8,
    /// The bend's index (`AudioOcarina_BendPitchTwoSemitones`).
    pub bend: i8,
    /// 0x80 (R) or 0x40 (Z): which button a B-flat 4 is.
    pub b_flat4_flag: u8,
}

/// `OcarinaSongButtons` (`ocarina.h`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OcarinaSongButtons {
    pub num_buttons: u8,
    pub buttons_index: [u8; 8],
}

/// `OcarinaStaff` (`ocarina.h`): what the message box reads of the ocarina.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OcarinaStaff {
    pub button_index: u8,
    /// Playing: the song played (0xFE none yet, 0xFF off). Playback: the times left to play.
    /// Recording: the `OcarinaRecordingState`.
    pub state: u8,
    /// `pos` ("locate"): the notes so far.
    pub pos: u8,
}

/// Which staff `msgCtx->ocarinaStaff` points at (`AudioOcarina_Get*Staff`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum StaffRef {
    #[default]
    Recording,
    Playing,
    Playback,
}

/// `sPlaybackSong`: a song of `sOcarinaSongNotes` or `sScarecrowsLongSongNotes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackSong {
    Song(u8),
    ScarecrowLong,
}

/// `sOcarinaSongNotes`' second dimension.
pub const SONG_NOTES: usize = 20;
/// `sScarecrowsLongSongNotes`' size.
const SCARECROW_LONG_NOTES: usize = 108;

/// `OcarinaNote`s as the C's memory holds them (size 8, big-endian: `pitch`, a pad byte,
/// `length`, `volume`, `vibrato`, `bend`, `bFlat4Flag`), as many as fit.
fn notes_to_bytes(notes: &[OcarinaNote], out: &mut [u8]) {
    for (n, b) in notes.iter().zip(out.chunks_exact_mut(8)) {
        let l = n.length.to_be_bytes();
        b.copy_from_slice(&[n.pitch, 0, l[0], l[1], n.volume, n.vibrato, n.bend as u8, n.b_flat4_flag]);
    }
}

/// The other way: `notes_to_bytes`' bytes back into notes.
fn notes_from_bytes(bytes: &[u8], notes: &mut [OcarinaNote]) {
    for (n, b) in notes.iter_mut().zip(bytes.chunks_exact(8)) {
        *n = OcarinaNote { pitch: b[0], length: u16::from_be_bytes([b[2], b[3]]), volume: b[4], vibrato: b[5], bend: b[6] as i8, b_flat4_flag: b[7] };
    }
}

/// The pad as the ocarina reads it: `cur.button` and the stick after
/// `PadUtils_UpdateRelXY` (`rel.stick_x`, `rel.stick_y`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OcarinaPad {
    pub button: u16,
    pub stick_x: i8,
    pub stick_y: i8,
}

impl OcarinaPad {
    pub fn of(input: &Input) -> OcarinaPad {
        OcarinaPad { button: input.cur.button, stick_x: input.rel.stick_x, stick_y: input.rel.stick_y }
    }
}

/// `general.c`'s ocarina statics.
#[derive(Debug, Clone, PartialEq)]
pub struct Ocarina {
    /// `sOcarinaAllowedButtonMask`, `sOcarinaAButtonMap`, `sOcarinaCUpButtonMap`,
    /// `sOcarinaCDownButtonMap` (not `PLATFORM_N64`: variables).
    pub allowed_button_mask: u32,
    pub a_button_map: u32,
    pub cup_button_map: u32,
    pub cdown_button_map: u32,
    pub is_input_enabled: bool,
    pub instrument_id: i8,
    pub cur_pitch: u8,
    pub prev_pitch: u8,
    pub cur_button_index: u8,
    pub music_staff_prev_pitch: u8,
    pub cur_bend_freq: f32,
    pub relative_volume: f32,
    pub cur_bend_index: i8,
    pub cur_volume: i8,
    pub cur_vibrato: i8,
    pub playback_state: u8,
    pub flags: u32,
    pub playback_note_timer: u32,
    pub playback_note_pos: u16,
    pub playback_staff_pos: u16,
    pub prev_with_music_staff_flags: u16,
    pub playback_pitch: u8,
    pub note_playback_volume: u8,
    pub note_playback_vibrato: u8,
    pub note_playback_bend: i8,
    pub relative_note_playback_bend: f32,
    pub relative_note_playback_volume: f32,
    pub playback_task_start: i32,
    /// `sOcarinaSongNotes`, flat (song * 20 + note): the C reads past a song into the next.
    pub song_notes: Vec<OcarinaNote>,
    /// `gOcarinaSongButtons`.
    pub song_buttons: Vec<OcarinaSongButtons>,
    pub button_to_pitch_map: Vec<u8>,
    pub pitch_to_button_map: Vec<u8>,
    /// `gFrogsSongPtr` (`sFrogsSongNotes`): the frogs' song game's 14 buttons.
    pub frogs_song_notes: Vec<u8>,
    pub playback_song: PlaybackSong,
    pub recording_state: u8,
    /// `sScarecrowsLongSongNotes`.
    pub scarecrows_long_song_notes: Vec<OcarinaNote>,
    pub played_song_index_plus_one: u8,
    pub music_staff_num_notes_per_test: u8,
    pub drop_input_timer: u8,
    pub playing_staff: OcarinaStaff,
    pub playback_staff: OcarinaStaff,
    pub recording_staff: OcarinaStaff,
    pub update_task_start: u32,
    /// `sOcarinaInputStickAdj`.
    pub input_stick_adj: (i8, i8),
    pub input_button_cur: u32,
    pub input_button_start: u32,
    pub input_button_prev: u32,
    pub input_button_press: i32,
    pub cur_song_without_music_staff: [u8; 8],
    pub without_music_staff_pos: u8,
    pub has_started_song: bool,
    pub first_song_index: u8,
    pub last_song_index: u8,
    pub avail_song_flags: u16,
    pub staff_playing_pos: u8,
    pub music_staff_pos: [u16; OCARINA_SONG_MAX as usize],
    pub music_staff_cur_held_length: [u16; OCARINA_SONG_MAX as usize],
    pub music_staff_expected_length: [u16; OCARINA_SONG_MAX as usize],
    pub music_staff_expected_pitch: [u8; OCARINA_SONG_MAX as usize],
    /// `sOcaMemoryGameAppendPos`, `sOcaMemoryGameEndPos`: the memory game's notes so far and its
    /// round's last.
    pub memory_game_append_pos: u8,
    pub memory_game_end_pos: u8,
    /// `sRecordSongPos`, `sOcarinaRecordTaskStart`, `sRecordOcarinaPitch`, `sRecordOcarinaVolume`,
    /// `sRecordOcarinaVibrato`, `sRecordOcarinaBendIndex`, `sRecordOcarinaButtonIndex`: the
    /// scarecrows' recording, the note being held since `record_task_start`.
    pub record_song_pos: u8,
    pub record_task_start: u32,
    pub record_pitch: u8,
    pub record_volume: u8,
    pub record_vibrato: u8,
    pub record_bend_index: i8,
    pub record_button_index: u8,
    /// `sScarecrowsLongSongSecondNote`: put back when a long recording ends with no note.
    pub scarecrows_long_song_second_note: OcarinaNote,
    /// `AudioOcarina_PlayLongScarecrowSong`'s `sScarecrowAfterCreditsState`,
    /// `sScarecrowAfterCreditsIntrumentId`, `sScarecrowAfterCreditsTimer`.
    pub scarecrow_after_credits_state: u8,
    pub scarecrow_after_credits_instrument_id: u8,
    pub scarecrow_after_credits_timer: u16,
    /// Not in the C: the pad `PadMgr_RequestPadData` would give at this `Audio_Update`.
    pub pad: OcarinaPad,
}

impl Default for Ocarina {
    fn default() -> Ocarina {
        Ocarina::new(&super::AudioGameTables::default())
    }
}

impl Ocarina {
    /// The statics as the ROM loads them; the song tables from the pack.
    pub fn new(t: &super::AudioGameTables) -> Ocarina {
        let mut song_notes = vec![OcarinaNote::default(); OCARINA_SONG_MAX as usize * SONG_NOTES];
        for (s, notes) in t.ocarina_song_notes.iter().enumerate().take(OCARINA_SONG_MAX as usize) {
            for (n, note) in notes.iter().enumerate().take(SONG_NOTES) {
                song_notes[s * SONG_NOTES + n] = *note;
            }
        }
        let mut scarecrows_long_song_notes = vec![OcarinaNote::default(); SCARECROW_LONG_NOTES];
        scarecrows_long_song_notes[0].pitch = OCARINA_PITCH_NONE;
        scarecrows_long_song_notes[1].pitch = OCARINA_PITCH_NONE;
        Ocarina {
            allowed_button_mask: (BTN_A | BTN_CUP | BTN_CDOWN | BTN_CLEFT | BTN_CRIGHT) as u32,
            a_button_map: BTN_A as u32,
            cup_button_map: BTN_CUP as u32,
            cdown_button_map: BTN_CDOWN as u32,
            is_input_enabled: false,
            instrument_id: OCARINA_INSTRUMENT_OFF as i8,
            cur_pitch: OCARINA_PITCH_NONE,
            prev_pitch: OCARINA_PITCH_C4,
            cur_button_index: OCARINA_BTN_A,
            music_staff_prev_pitch: OCARINA_PITCH_C4,
            cur_bend_freq: 1.0,
            relative_volume: 87.0 / 127.0,
            cur_bend_index: 0,
            cur_volume: 87,
            cur_vibrato: 0,
            playback_state: 0,
            flags: 0,
            playback_note_timer: 0,
            playback_note_pos: 0,
            playback_staff_pos: 0,
            prev_with_music_staff_flags: 0,
            playback_pitch: OCARINA_PITCH_NONE,
            note_playback_volume: 0,
            note_playback_vibrato: 0,
            note_playback_bend: 0,
            relative_note_playback_bend: 1.0,
            relative_note_playback_volume: 1.0,
            playback_task_start: 0,
            song_notes,
            song_buttons: t.ocarina_song_buttons.clone(),
            button_to_pitch_map: t.button_to_pitch_map.clone(),
            pitch_to_button_map: t.pitch_to_button_map.clone(),
            frogs_song_notes: t.frogs_song_notes.clone(),
            playback_song: PlaybackSong::Song(0),
            recording_state: OCARINA_RECORD_OFF,
            scarecrows_long_song_notes,
            played_song_index_plus_one: 0,
            music_staff_num_notes_per_test: 0,
            drop_input_timer: 0,
            playing_staff: OcarinaStaff::default(),
            playback_staff: OcarinaStaff::default(),
            recording_staff: OcarinaStaff::default(),
            update_task_start: 0,
            input_stick_adj: (0, 0),
            input_button_cur: 0,
            input_button_start: 0,
            input_button_prev: 0,
            input_button_press: 0,
            cur_song_without_music_staff: [0; 8],
            without_music_staff_pos: 0,
            has_started_song: false,
            first_song_index: 0,
            last_song_index: 0,
            avail_song_flags: 0,
            staff_playing_pos: 0,
            music_staff_pos: [0; OCARINA_SONG_MAX as usize],
            music_staff_cur_held_length: [0; OCARINA_SONG_MAX as usize],
            music_staff_expected_length: [0; OCARINA_SONG_MAX as usize],
            music_staff_expected_pitch: [0; OCARINA_SONG_MAX as usize],
            memory_game_append_pos: 0,
            memory_game_end_pos: 0,
            record_song_pos: 0,
            record_task_start: 0,
            record_pitch: 0,
            record_volume: 0,
            record_vibrato: 0,
            record_bend_index: 0,
            record_button_index: 0,
            scarecrows_long_song_second_note: OcarinaNote::default(),
            scarecrow_after_credits_state: 0,
            scarecrow_after_credits_instrument_id: OCARINA_INSTRUMENT_DEFAULT,
            scarecrow_after_credits_timer: 1200,
            pad: OcarinaPad::default(),
        }
    }

    /// `sOcarinaSongNotes[song][pos]`, read flat as the C's memory: past a song, the next one's
    /// notes. Outside the table (a song's -1, or past the last), zero.
    pub fn song_note(&self, song: u8, pos: i32) -> OcarinaNote {
        let i = song as i32 * SONG_NOTES as i32 + pos;
        if i < 0 { OcarinaNote::default() } else { self.song_notes.get(i as usize).copied().unwrap_or_default() }
    }

    /// `AudioOcarina_SetRecordingSong`'s `recordedSong[pos]`: the long song's notes
    /// (`gScarecrowLongSongPtr`), or for the spawn song its buffer, the memory game's song
    /// (`sMemoryGameSongPtr`), flat as the C's memory. Past the table, none (the C writes on).
    fn recorded_note_mut(&mut self, pos: usize) -> Option<&mut OcarinaNote> {
        if self.recording_state == OCARINA_RECORD_SCARECROW_LONG {
            self.scarecrows_long_song_notes.get_mut(pos)
        } else {
            self.song_notes.get_mut(OCARINA_SONG_MEMORY_GAME as usize * SONG_NOTES + pos)
        }
    }

    /// `MemCopy(save, gScarecrowLongSongPtr, 0x360)`: the long song as the save keeps it.
    pub fn scarecrow_long_song_bytes(&self) -> [u8; 0x360] {
        let mut out = [0; 0x360];
        notes_to_bytes(&self.scarecrows_long_song_notes, &mut out);
        out
    }

    /// `MemCopy(save, gScarecrowSpawnSongPtr, 0x80)`: the spawn song's first 16 notes as the
    /// save keeps them.
    pub fn scarecrow_spawn_song_bytes(&self) -> [u8; 0x80] {
        let mut out = [0; 0x80];
        let base = OCARINA_SONG_SCARECROW_SPAWN as usize * SONG_NOTES;
        notes_to_bytes(&self.song_notes[base..base + 16], &mut out);
        out
    }

    /// `Sram_OpenSave`'s copies: the save's scarecrows' songs into the statics, each if it's set.
    pub fn sram_open_save_scarecrow_songs(&mut self, save: &crate::save::SaveContext) {
        if save.scarecrow_long_song_set != 0 {
            notes_from_bytes(&save.scarecrow_long_song, &mut self.scarecrows_long_song_notes);
        }
        if save.scarecrow_spawn_song_set != 0 {
            let base = OCARINA_SONG_SCARECROW_SPAWN as usize * SONG_NOTES;
            notes_from_bytes(&save.scarecrow_spawn_song, &mut self.song_notes[base..base + 16]);
        }
    }

    /// `sPlaybackSong[pos]`.
    fn playback_note(&self, pos: i32) -> OcarinaNote {
        match self.playback_song {
            PlaybackSong::Song(s) => self.song_note(s, pos),
            PlaybackSong::ScarecrowLong => {
                if pos < 0 {
                    OcarinaNote::default()
                } else {
                    self.scarecrows_long_song_notes.get(pos as usize).copied().unwrap_or_default()
                }
            }
        }
    }

    /// `gOcarinaSongButtons[song]`.
    pub fn song_buttons(&self, song: u8) -> OcarinaSongButtons {
        self.song_buttons.get(song as usize).copied().unwrap_or_default()
    }

    fn button_to_pitch(&self, button: u8) -> u8 {
        self.button_to_pitch_map.get(button as usize).copied().unwrap_or(0)
    }

    fn pitch_to_button(&self, pitch: u8) -> u8 {
        self.pitch_to_button_map.get(pitch as usize).copied().unwrap_or(0)
    }

    /// The staff `StaffRef` names.
    pub fn staff(&self, r: StaffRef) -> &OcarinaStaff {
        match r {
            StaffRef::Recording => &self.recording_staff,
            StaffRef::Playing => &self.playing_staff,
            StaffRef::Playback => &self.playback_staff,
        }
    }

    pub fn staff_mut(&mut self, r: StaffRef) -> &mut OcarinaStaff {
        match r {
            StaffRef::Recording => &mut self.recording_staff,
            StaffRef::Playing => &mut self.playing_staff,
            StaffRef::Playback => &mut self.playback_staff,
        }
    }
}

/// `CHECK_BTN_ANY`.
fn check_btn_any(cur: u32, mask: u32) -> bool {
    cur & mask != 0
}

impl GameAudio {
    /// `AudioOcarina_SetCustomButtonMapping` (not `PLATFORM_N64`).
    pub fn audio_ocarina_set_custom_button_mapping(&mut self, use_custom: bool) {
        let o = &mut self.ocarina;
        if !use_custom {
            log::debug!("AUDIO : Ocarina Control Assign Normal");
            o.allowed_button_mask = (BTN_A | BTN_CUP | BTN_CDOWN | BTN_CLEFT | BTN_CRIGHT) as u32;
            o.a_button_map = BTN_A as u32;
            o.cup_button_map = BTN_CUP as u32;
            o.cdown_button_map = BTN_CDOWN as u32;
        } else {
            log::debug!("AUDIO : Ocarina Control Assign Custom");
            o.allowed_button_mask = (BTN_A | BTN_B | BTN_CDOWN | BTN_CLEFT | BTN_CRIGHT) as u32;
            o.a_button_map = BTN_B as u32;
            o.cup_button_map = BTN_CDOWN as u32;
            o.cdown_button_map = BTN_A as u32;
        }
    }

    /// `AudioOcarina_ReadControllerInput`.
    pub fn audio_ocarina_read_controller_input(&mut self) {
        let o = &mut self.ocarina;
        let prev = o.input_button_cur;
        o.input_button_cur = o.pad.button as u32;
        o.input_button_prev = prev;
        o.input_stick_adj = (o.pad.stick_x, o.pad.stick_y);
    }

    /// `AudioOcarina_BendPitchTwoSemitones`: the frequency scale of a bend, up to two
    /// semitones each way (`gBendPitchTwoSemitonesFrequencies`, 1.0 at index 128).
    pub fn audio_ocarina_bend_pitch_two_semitones(&self, bend_index: i8) -> f32 {
        let adj: i8 = if bend_index > 64 {
            127
        } else if bend_index < -64 {
            -128
        } else if bend_index >= 0 {
            ((bend_index as i32 * 127) / 64) as i8
        } else {
            ((bend_index as i32 * 128) / 64) as i8
        };
        self.audio_tables.bend_pitch_two_semitones_frequencies.get((adj as i32 + 128) as usize).copied().unwrap_or(1.0)
    }

    /// `AudioOcarina_GetPlayingState`: the song played (once), else 0xFE while the ocarina is
    /// on, 0xFF off.
    pub fn audio_ocarina_get_playing_state(&mut self) -> u8 {
        let o = &mut self.ocarina;
        if o.played_song_index_plus_one != 0 {
            let s = o.played_song_index_plus_one - 1;
            o.played_song_index_plus_one = 0;
            s
        } else if o.flags != 0 {
            0xFE
        } else {
            0xFF
        }
    }

    /// `AudioOcarina_MapNoteToButton`: a pitch's button; B-flat 4 by its flag (0x80: C-Right).
    pub fn audio_ocarina_map_note_to_button(&self, pitch_and_b_flat_flag: u8) -> u8 {
        let button = self.ocarina.pitch_to_button(pitch_and_b_flat_flag & 0x3F);
        if button == OCARINA_BTN_C_RIGHT_OR_C_LEFT {
            if pitch_and_b_flat_flag & 0x80 != 0 {
                return OCARINA_BTN_C_RIGHT;
            }
            return OCARINA_BTN_C_LEFT;
        }
        button
    }

    /// `AudioOcarina_MapNotesToScarecrowButtons`: the scarecrow's song's buttons from its notes.
    pub fn audio_ocarina_map_notes_to_scarecrow_buttons(&mut self, note_song_index: u8) {
        let (mut button_pos, mut note_pos) = (0usize, 0i32);
        while button_pos < 8 && note_pos < 16 {
            let pitch = self.ocarina.song_note(note_song_index, note_pos).pitch;
            note_pos += 1;
            if pitch != OCARINA_PITCH_NONE {
                let b = self.ocarina.pitch_to_button(pitch);
                if let Some(sb) = self.ocarina.song_buttons.get_mut(OCARINA_SONG_SCARECROW_SPAWN as usize) {
                    sb.buttons_index[button_pos] = b;
                }
                button_pos += 1;
            }
        }
    }

    /// `AudioOcarina_Start` (`Na_StartOcarinaSinglePlayCheck2`): the songs to listen for
    /// (bits 0x3FFF) and how (0xC000: 0 checks eight notes at a time, 0x8000 one, 0x4000 and
    /// 0xC000 free play); 0xFFFF turns the listening off.
    pub fn audio_ocarina_start(&mut self, mut ocarina_flags: u16) {
        let scarecrow_set = self.ocarina.song_note(OCARINA_SONG_SCARECROW_SPAWN, 1).volume != 0xFF;
        if scarecrow_set && (ocarina_flags & 0xFFF) == 0xFFF {
            ocarina_flags |= 0x1000;
        }
        if ocarina_flags == 0xCFFF && scarecrow_set {
            ocarina_flags = 0xDFFF;
        }
        if ocarina_flags == 0xFFF && scarecrow_set {
            ocarina_flags = 0x1FFF;
        }
        if ocarina_flags != 0xFFFF {
            let o = &mut self.ocarina;
            o.flags = 0x8000_0000u32.wrapping_add(ocarina_flags as u32);
            o.first_song_index = 0;
            o.last_song_index = OCARINA_SONG_MAX;
            if ocarina_flags != 0xA000 {
                o.last_song_index -= 1;
            }
            o.avail_song_flags = ocarina_flags & 0x3FFF;
            o.music_staff_num_notes_per_test = 8; // Ocarina Check
            o.has_started_song = false;
            o.played_song_index_plus_one = 0;
            o.staff_playing_pos = 0;
            let st = self.audio_ocarina_get_playing_state();
            let o = &mut self.ocarina;
            o.playing_staff.state = st;
            o.is_input_enabled = true;
            o.prev_with_music_staff_flags = 0;
            // Reset music staff song check.
            for i in 0..OCARINA_SONG_MAX as usize {
                o.music_staff_pos[i] = 0;
                o.music_staff_cur_held_length[i] = 0;
                o.music_staff_expected_length[i] = 0;
                o.music_staff_expected_pitch[i] = 0;
            }
            if ocarina_flags & 0x8000 != 0 {
                o.music_staff_num_notes_per_test = 0; // Ocarina Playback
            }
            if ocarina_flags & 0x4000 != 0 {
                o.without_music_staff_pos = 0;
            }
            if ocarina_flags & 0xD000 != 0 {
                self.audio_ocarina_map_notes_to_scarecrow_buttons(OCARINA_SONG_SCARECROW_SPAWN);
            }
        } else {
            self.ocarina.flags = 0;
            self.ocarina.is_input_enabled = false;
        }
    }

    /// `AudioOcarina_CheckIfStartedSong`.
    fn audio_ocarina_check_if_started_song(&mut self) {
        let o = &mut self.ocarina;
        if o.cur_pitch != OCARINA_PITCH_NONE && !o.has_started_song {
            o.has_started_song = true;
            o.music_staff_prev_pitch = OCARINA_PITCH_NONE;
        }
    }

    /// `AudioOcarina_CheckSongsWithMusicStaff`: the check and playback modes, note by note
    /// against each song still possible.
    fn audio_ocarina_check_songs_with_music_staff(&mut self) {
        self.audio_ocarina_check_if_started_song();
        let o = &mut self.ocarina;
        if !o.has_started_song {
            return;
        }
        if (o.cur_bend_index as i32).abs() > 20 {
            o.flags = 0;
            return;
        }
        let no_new_valid_input = o.prev_pitch == o.cur_pitch || o.cur_pitch == OCARINA_PITCH_NONE;
        let mut staff_playing_pos_offset: i8 = 0;
        let mut song_index = o.first_song_index;
        while song_index < o.last_song_index {
            let si = song_index as usize;
            let cur_song_flag: u16 = 1 << song_index;
            if o.avail_song_flags & cur_song_flag != 0 {
                o.music_staff_cur_held_length[si] = o.music_staff_expected_length[si].wrapping_add(18);
                let held = o.music_staff_cur_held_length[si] as i32;
                let expected = o.music_staff_expected_length[si] as i32;
                if no_new_valid_input {
                    if held >= expected - 18
                        && held >= expected + 18
                        && o.song_note(song_index, o.music_staff_pos[si] as i32).length == 0
                        && o.music_staff_prev_pitch == o.music_staff_expected_pitch[si]
                    {
                        // The song is finished and played right (its end: a length of 0).
                        o.played_song_index_plus_one = song_index + 1;
                        o.is_input_enabled = false;
                        o.flags = 0;
                    }
                } else if held >= expected - 18 {
                    // Always taken when a new note is played.
                    if o.music_staff_prev_pitch != OCARINA_PITCH_NONE {
                        if o.music_staff_prev_pitch == o.music_staff_expected_pitch[si] {
                            if song_index == OCARINA_SONG_SCARECROW_SPAWN {
                                o.music_staff_cur_held_length[si] = 0;
                            }
                        } else {
                            // Not this song's note: it's no longer possible.
                            o.avail_song_flags ^= cur_song_flag;
                        }
                    }
                    let mut cur_pos = o.music_staff_pos[si] as i32;
                    let mut cur = o.song_note(song_index, cur_pos);
                    o.music_staff_pos[si] = o.music_staff_pos[si].wrapping_add(1);
                    let mut next = o.song_note(song_index, o.music_staff_pos[si] as i32);
                    o.music_staff_expected_length[si] = cur.length;
                    o.music_staff_expected_pitch[si] = cur.pitch;
                    // The current note isn't the one expected.
                    if o.cur_pitch != o.music_staff_expected_pitch[si] {
                        o.avail_song_flags ^= cur_song_flag;
                    }
                    // @bug (game): compares a pitch with OCARINA_BTN_INVALID (both 0xFF), as the C.
                    while cur.pitch == next.pitch || (next.pitch == OCARINA_BTN_INVALID && next.length != 0) {
                        o.music_staff_expected_length[si] = o.music_staff_expected_length[si].wrapping_add(next.length);
                        cur_pos = o.music_staff_pos[si] as i32;
                        cur = o.song_note(song_index, cur_pos);
                        next = o.song_note(song_index, cur_pos + 1);
                        o.music_staff_pos[si] = o.music_staff_pos[si].wrapping_add(1);
                    }
                } else if held < 10 {
                    // Never taken.
                    staff_playing_pos_offset = -1;
                    o.music_staff_cur_held_length[si] = 0;
                    o.music_staff_prev_pitch = o.cur_pitch;
                } else {
                    // Never taken.
                    o.avail_song_flags ^= cur_song_flag;
                }
            }
            // No song left that it could be, and the notes' limit passed.
            if o.avail_song_flags == 0 && o.staff_playing_pos >= o.music_staff_num_notes_per_test {
                o.is_input_enabled = false;
                if o.flags & 0x4000 != 0 && o.cur_pitch == o.song_note(song_index, 0).pitch {
                    // Never taken: this function doesn't run with 0x4000.
                    o.prev_with_music_staff_flags = o.flags as u16;
                }
                o.flags = 0;
                return;
            }
            song_index += 1;
        }
        if !no_new_valid_input {
            o.music_staff_prev_pitch = o.cur_pitch;
            o.staff_playing_pos = o.staff_playing_pos.wrapping_add((staff_playing_pos_offset as i32 + 1) as u8);
        }
    }

    /// `AudioOcarina_CheckSongsWithoutMusicStaff`: free play, the last eight notes against
    /// each song's buttons. L with a note starts the listening again.
    fn audio_ocarina_check_songs_without_music_staff(&mut self) {
        let mask = self.ocarina.allowed_button_mask;
        if check_btn_any(self.ocarina.input_button_cur, BTN_L as u32) && check_btn_any(self.ocarina.input_button_cur, mask) {
            let f = self.ocarina.flags as u16;
            self.audio_ocarina_start(f);
            return;
        }
        self.audio_ocarina_check_if_started_song();
        let o = &mut self.ocarina;
        if !o.has_started_song {
            return;
        }
        if o.prev_pitch != o.cur_pitch && o.cur_pitch != OCARINA_PITCH_NONE {
            o.staff_playing_pos = o.staff_playing_pos.wrapping_add(1);
            if o.staff_playing_pos as usize > o.cur_song_without_music_staff.len() {
                o.staff_playing_pos = 1;
            }
            if o.without_music_staff_pos == 8 {
                for i in 0..7 {
                    o.cur_song_without_music_staff[i] = o.cur_song_without_music_staff[i + 1];
                }
            } else {
                o.without_music_staff_pos += 1;
            }
            let p = o.without_music_staff_pos as usize - 1;
            o.cur_song_without_music_staff[p] = if (o.cur_bend_index as i32).abs() > 20 { OCARINA_PITCH_NONE } else { o.cur_pitch };
            // Each song possible against the notes' tail.
            let mut i = o.first_song_index;
            while i < o.last_song_index {
                if o.avail_song_flags & (1u16 << i) != 0 {
                    let sb = o.song_buttons(i);
                    let n = sb.num_buttons;
                    let (mut j, mut k) = (0u8, 0u8);
                    while j < n && k == 0 && o.without_music_staff_pos >= n {
                        let pitch = o.cur_song_without_music_staff[(o.without_music_staff_pos - n + j) as usize];
                        if pitch == o.button_to_pitch(sb.buttons_index[j as usize]) {
                            j += 1;
                        } else {
                            k += 1;
                        }
                    }
                    // The song `i` was played.
                    if j == n {
                        o.played_song_index_plus_one = i + 1;
                        o.is_input_enabled = false;
                        o.flags = 0;
                    }
                }
                i += 1;
            }
        }
    }

    /// `AudioOcarina_PlayControllerInput`: the pad to a note (A D4, C-Down F4, C-Right A4,
    /// C-Left B4, C-Up D5; R a semitone up, Z down), the stick's y bending it and x its
    /// vibrato; a new note starts `NA_SE_OC_OCARINA` with its instrument and pitch on the
    /// channel's ports, a release stops it.
    pub fn audio_ocarina_play_controller_input(&mut self) {
        let o = &mut self.ocarina;
        // Two different notes aren't played on consecutive frames.
        if o.flags != 0 && o.drop_input_timer != 0 {
            o.drop_input_timer -= 1;
            return;
        }
        let mask = o.allowed_button_mask;
        // The button that took the ocarina out doesn't play a note.
        if o.input_button_start == 0 || (o.input_button_start & mask) != (o.input_button_cur & mask) {
            o.input_button_start = 0;
            o.cur_pitch = OCARINA_PITCH_NONE;
            o.cur_button_index = OCARINA_BTN_INVALID;
            let held = (o.input_button_cur & mask) & (o.input_button_prev & mask);
            if (o.input_button_press as u32 & held) == 0 && o.input_button_cur != 0 {
                o.input_button_press = o.input_button_cur as i32;
            } else {
                o.input_button_press &= held as i32;
            }
            let press = o.input_button_press as u32;
            if check_btn_any(press, o.a_button_map) {
                log::trace!("Presss NA_KEY_D4 {:08x}", o.a_button_map);
                o.cur_pitch = OCARINA_PITCH_D4;
                o.cur_button_index = OCARINA_BTN_A;
            } else if check_btn_any(press, o.cdown_button_map) {
                log::trace!("Presss NA_KEY_F4 {:08x}", o.cdown_button_map);
                o.cur_pitch = OCARINA_PITCH_F4;
                o.cur_button_index = OCARINA_BTN_C_DOWN;
            } else if check_btn_any(press, BTN_CRIGHT as u32) {
                o.cur_pitch = OCARINA_PITCH_A4;
                o.cur_button_index = OCARINA_BTN_C_RIGHT;
            } else if check_btn_any(press, BTN_CLEFT as u32) {
                o.cur_pitch = OCARINA_PITCH_B4;
                o.cur_button_index = OCARINA_BTN_C_LEFT;
            } else if check_btn_any(press, o.cup_button_map) {
                o.cur_pitch = OCARINA_PITCH_D5;
                o.cur_button_index = OCARINA_BTN_C_UP;
            }
            // R raises the pitch a semitone.
            if o.cur_pitch != OCARINA_PITCH_NONE && check_btn_any(o.input_button_cur, BTN_R as u32) && o.recording_state != OCARINA_RECORD_SCARECROW_SPAWN {
                o.cur_button_index = o.cur_button_index.wrapping_add(0x80); // B-flat 4's flag
                o.cur_pitch = o.cur_pitch.wrapping_add(1);
            }
            // Z lowers it a semitone.
            if o.cur_pitch != OCARINA_PITCH_NONE && check_btn_any(o.input_button_cur, BTN_Z as u32) && o.recording_state != OCARINA_RECORD_SCARECROW_SPAWN {
                o.cur_button_index = o.cur_button_index.wrapping_add(0x40); // B-flat 4's flag
                o.cur_pitch = o.cur_pitch.wrapping_sub(1);
            }
            if o.recording_state != OCARINA_RECORD_SCARECROW_SPAWN {
                // The stick's y bends the note, its x is the vibrato.
                o.cur_bend_index = o.input_stick_adj.1;
                let bend = o.cur_bend_index;
                let freq = self.audio_ocarina_bend_pitch_two_semitones(bend);
                let o = &mut self.ocarina;
                o.cur_bend_freq = freq;
                o.cur_vibrato = ((o.input_stick_adj.0 as i32).abs() >> 2) as i8;
                let v = o.cur_vibrato;
                // The vibrato on IO port 6.
                self.audiocmd_channel_set_io_ocarina(6, v);
            } else {
                // No bend or vibrato while recording the scarecrow's spawn song.
                o.cur_bend_index = 0;
                o.cur_vibrato = 0;
                o.cur_bend_freq = 1.0;
            }
            let o = &self.ocarina;
            if o.cur_pitch != OCARINA_PITCH_NONE && o.prev_pitch != o.cur_pitch {
                // The instrument on port 7 (an index into sequence 0's table), the pitch on 5.
                let (inst, pitch) = (o.instrument_id.wrapping_sub(1), o.cur_pitch as i8);
                self.audiocmd_channel_set_io_ocarina(7, inst);
                self.audiocmd_channel_set_io_ocarina(5, pitch);
                self.play_sfx_general(NA_SE_OC_OCARINA, SfxPos::Default, 4, SfxF32::OcarinaBendFreq, SfxF32::OcarinaVolume, SfxS8::Zero);
            } else if o.prev_pitch != OCARINA_PITCH_NONE && o.cur_pitch == OCARINA_PITCH_NONE {
                // Released: the note stops.
                self.stop_sfx_by_id(NA_SE_OC_OCARINA);
            }
        }
    }

    /// `AUDIOCMD_CHANNEL_SET_IO(SEQ_PLAYER_SFX, SFX_CHANNEL_OCARINA, port, data)`.
    fn audiocmd_channel_set_io_ocarina(&mut self, port: u8, data: i8) {
        // AUDIOCMD_OP_CHANNEL_SET_IO (0x06).
        self.queue_cmd_s8((0x06 << 24) | ((SEQ_PLAYER_SFX as u32) << 16) | ((SFX_CHANNEL_OCARINA as u32) << 8) | port as u32, data);
    }

    /// `AudioOcarina_EnableInput` (unused by the game).
    pub fn audio_ocarina_enable_input(&mut self, input_enabled: bool) {
        self.ocarina.is_input_enabled = input_enabled;
    }

    /// `AudioOcarina_SetInstrument` (`Na_StopOcarinaMode`): the ocarina on with an instrument,
    /// the bgm turned down and the button that took it out kept; or off, everything stopped.
    pub fn audio_ocarina_set_instrument(&mut self, ocarina_instrument_id: u8) {
        if self.ocarina.instrument_id as u8 == ocarina_instrument_id {
            return;
        }
        self.queue_seq_cmd(seq_cmd8(SEQ_PLAYER_SFX, 1, SFX_CHANNEL_OCARINA, ocarina_instrument_id));
        self.ocarina.instrument_id = ocarina_instrument_id as i8;
        if ocarina_instrument_id == OCARINA_INSTRUMENT_OFF {
            let o = &mut self.ocarina;
            o.input_button_cur = 0;
            o.input_button_prev = 0;
            o.input_button_press = 0;
            o.input_button_start = 0xFFFF;
            self.audio_ocarina_play_controller_input();
            self.stop_sfx_by_id(NA_SE_OC_OCARINA);
            self.set_sfx_banks_mute(0);
            let o = &mut self.ocarina;
            o.playback_state = 0;
            o.playback_staff_pos = 0;
            o.is_input_enabled = false;
            o.flags = 0;
            // The bgm (players 0 and 3) back to full volume.
            self.clear_bgm_mute(SFX_CHANNEL_OCARINA);
        } else {
            self.ocarina.input_button_cur = 0;
            self.audio_ocarina_read_controller_input();
            // The button that took the ocarina out.
            self.ocarina.input_button_start = self.ocarina.input_button_cur;
            // The bgm (players 0 and 3) turned down while playing.
            self.queue_seq_cmd_mute(SFX_CHANNEL_OCARINA);
        }
    }

    /// `AudioOcarina_SetPlaybackSong` (`Na_StartOcarinaBgm`): song `song_index_plus_one - 1`
    /// played `playback_state` times (0 stops it).
    pub fn audio_ocarina_set_playback_song(&mut self, song_index_plus_one: i8, playback_state: i8) {
        if song_index_plus_one == 0 {
            self.ocarina.playback_state = 0;
            self.stop_sfx_by_id(NA_SE_OC_OCARINA);
            return;
        }
        let o = &mut self.ocarina;
        o.playback_song = if (song_index_plus_one as i32) < OCARINA_SONG_SCARECROW_LONG as i32 + 1 { PlaybackSong::Song(song_index_plus_one as u8 - 1) } else { PlaybackSong::ScarecrowLong };
        o.playback_state = playback_state as u8;
        o.playback_note_timer = 0;
        o.playback_pitch = OCARINA_PITCH_NONE;
        o.playback_note_pos = 0;
        o.playback_staff_pos = 0;
        // (The long scarecrow's song, never recorded, is all OCARINA_PITCH_NONE: the C reads on
        // past it; stopped here at its end.)
        while o.playback_note(o.playback_note_pos as i32).pitch == OCARINA_PITCH_NONE && (o.playback_note_pos as usize) < SCARECROW_LONG_NOTES {
            o.playback_note_pos += 1;
        }
    }

    /// `AudioOcarina_PlaybackSong`: the song played back note by note, timed by the audio
    /// side's tasks.
    fn audio_ocarina_playback_song(&mut self) {
        let o = &mut self.ocarina;
        if o.playback_state == 0 {
            return;
        }
        let note_timer_step: u32 = if o.playback_staff_pos == 0 { 3 } else { o.update_task_start.wrapping_sub(o.playback_task_start as u32) };
        // @bug (game): nextNoteTimerStep is left unset when the timer runs on; it's only read
        // once the timer is 0, when it's set.
        let mut next_note_timer_step: u32 = 0;
        if note_timer_step < o.playback_note_timer {
            o.playback_note_timer -= note_timer_step;
        } else {
            next_note_timer_step = note_timer_step - o.playback_note_timer;
            o.playback_note_timer = 0;
        }
        if o.playback_note_timer != 0 {
            return;
        }
        let pos = o.playback_note_pos as i32;
        let note = o.playback_note(pos);
        o.playback_note_timer = note.length as u32;
        if o.playback_note_pos == 1 {
            o.playback_note_timer += 1;
        }
        if o.playback_note_timer == 0 {
            o.playback_state -= 1;
            if o.playback_state != 0 {
                o.playback_note_pos = 0;
                o.playback_staff_pos = 0;
                o.playback_pitch = OCARINA_PITCH_NONE;
            } else {
                self.stop_sfx_by_id(NA_SE_OC_OCARINA);
            }
            return;
        }
        o.playback_note_timer = o.playback_note_timer.wrapping_sub(next_note_timer_step);
        // The volume.
        if o.note_playback_volume != note.volume {
            o.note_playback_volume = note.volume;
            o.relative_note_playback_volume = o.note_playback_volume as f32 / 127.0;
        }
        // The vibrato (not OOT_VERSION < PAL_1_0 || !PLATFORM_N64: only when it changes).
        if o.note_playback_vibrato != note.vibrato {
            o.note_playback_vibrato = note.vibrato;
            let v = o.note_playback_vibrato as i8;
            // The vibrato on IO port 6.
            self.audiocmd_channel_set_io_ocarina(6, v);
        }
        // The bend.
        let o = &self.ocarina;
        if o.note_playback_bend != note.bend {
            let bend = note.bend;
            let f = self.audio_ocarina_bend_pitch_two_semitones(bend);
            let o = &mut self.ocarina;
            o.note_playback_bend = bend;
            o.relative_note_playback_bend = f;
        }
        let o = &mut self.ocarina;
        // No change in volume, vibrato or bend from the last note.
        let prev = o.playback_note(pos - 1);
        if note.volume == prev.volume && note.vibrato == prev.vibrato && note.bend == prev.bend {
            o.playback_pitch = 0xFE;
        }
        if o.playback_pitch != note.pitch {
            let pitch = note.pitch;
            // B-flat 4 is between C-Right and C-Left: its flag says which.
            o.playback_pitch = if pitch == OCARINA_PITCH_BFLAT4 { pitch.wrapping_add(note.b_flat4_flag) } else { pitch };
            if o.playback_pitch != OCARINA_PITCH_NONE {
                o.playback_staff_pos = o.playback_staff_pos.wrapping_add(1);
                let (inst, p) = (o.instrument_id.wrapping_sub(1), (o.playback_pitch & 0x3F) as i8);
                // The instrument on port 7, the pitch on port 5.
                self.audiocmd_channel_set_io_ocarina(7, inst);
                self.audiocmd_channel_set_io_ocarina(5, p);
                self.play_sfx_general(NA_SE_OC_OCARINA, SfxPos::Default, 4, SfxF32::OcarinaPlaybackBend, SfxF32::OcarinaPlaybackVolume, SfxS8::Zero);
            } else {
                self.stop_sfx_by_id(NA_SE_OC_OCARINA);
            }
        }
        self.ocarina.playback_note_pos = self.ocarina.playback_note_pos.wrapping_add(1);
    }

    /// `AudioOcarina_SetRecordingSong`: the note held since `record_task_start` written to the
    /// recording (`recorded_note_mut`), the current one becomes the held one. Unless
    /// `is_recording_complete` (or the long song's 107th entry), that's all. Complete: the
    /// trailing rests cut to one, the song ended (a zero length). The spawn song is then checked
    /// (eight notes or more): shifted down past its first entry (the rest before the first note),
    /// its buttons mapped, and refused (`OCARINA_RECORD_REJECTED`, the spawn song's second note's
    /// volume 0xFF: unset) if it holds any of the twelve songs or is one note eight times; else
    /// copied into the spawn song and the input turned off. Fewer than eight: unset.
    fn audio_ocarina_set_recording_song(&mut self, is_recording_complete: bool) {
        let o = &mut self.ocarina;
        let pos = o.record_song_pos as usize;
        let (pitch, length, volume, vibrato, bend, b_flat4_flag) = (o.record_pitch, o.update_task_start.wrapping_sub(o.record_task_start) as u16, o.record_volume, o.record_vibrato, o.record_bend_index, o.record_button_index & 0xC0);
        match o.recorded_note_mut(pos) {
            Some(n) => *n = OcarinaNote { pitch, length, volume, vibrato, bend, b_flat4_flag },
            None => log::warn!("AudioOcarina_SetRecordingSong: note {pos} is past the recording's table (the C writes on)"),
        }
        o.record_pitch = o.cur_pitch;
        o.record_volume = o.cur_volume as u8;
        o.record_vibrato = o.cur_vibrato as u8;
        o.record_bend_index = o.cur_bend_index;
        o.record_button_index = o.cur_button_index;
        o.record_song_pos = o.record_song_pos.wrapping_add(1);

        if o.record_song_pos as usize != SCARECROW_LONG_NOTES - 1 && !is_recording_complete {
            // Continue recording.
            return;
        }

        // Recording is complete.
        let mut i = o.record_song_pos as u16;
        let mut pitch = OCARINA_PITCH_NONE;
        while i != 0 && pitch == OCARINA_PITCH_NONE {
            i -= 1;
            pitch = o.recorded_note_mut(i as usize).map_or(0, |n| n.pitch);
        }
        if o.record_song_pos as u16 != i + 1 {
            o.record_song_pos = (i + 2) as u8;
            let p = o.record_song_pos as usize - 1;
            if let Some(n) = o.recorded_note_mut(p) {
                n.length = 0;
            }
        }
        let p = o.record_song_pos as usize;
        if let Some(n) = o.recorded_note_mut(p) {
            n.length = 0;
        }

        if o.recording_state == OCARINA_RECORD_SCARECROW_SPAWN {
            let spawn_second = OCARINA_SONG_SCARECROW_SPAWN as usize * SONG_NOTES + 1;
            if o.staff_playing_pos >= 8 {
                // The first entry (the rest before the first note) dropped.
                let base = OCARINA_SONG_MEMORY_GAME as usize * SONG_NOTES;
                for i in 0..o.record_song_pos as usize {
                    let next = o.song_notes.get(base + i + 1).copied().unwrap_or_default();
                    if let Some(n) = o.song_notes.get_mut(base + i) {
                        *n = next;
                    }
                }
                // The buffer's buttons into the spawn song's, to be checked.
                self.audio_ocarina_map_notes_to_scarecrow_buttons(OCARINA_SONG_MEMORY_GAME);
                let o = &mut self.ocarina;
                let spawn = o.song_buttons(OCARINA_SONG_SCARECROW_SPAWN);
                // Each song, at each place it could start within the eight notes.
                for s in 0..OCARINA_SONG_SCARECROW_SPAWN {
                    let sb = o.song_buttons(s);
                    let num = sb.num_buttons as i32;
                    let mut j = 0i32;
                    while j < 9 - num {
                        let mut k = 0i32;
                        while k < num && k + j < 8 && sb.buttons_index[k as usize] == spawn.buttons_index[(k + j) as usize] {
                            k += 1;
                        }
                        // The recorded song holds a song.
                        if k == num {
                            o.recording_state = OCARINA_RECORD_REJECTED;
                            o.song_notes[spawn_second].volume = 0xFF;
                            return;
                        }
                        j += 1;
                    }
                }
                // One note eight times.
                let mut i = 1;
                while i < 8 {
                    if spawn.buttons_index[0] != spawn.buttons_index[i] {
                        i = 9; // break
                    } else {
                        i += 1;
                    }
                }
                if i == 8 {
                    o.recording_state = OCARINA_RECORD_REJECTED;
                    o.song_notes[spawn_second].volume = 0xFF;
                    return;
                }
                // Accepted: the buffer copied into the spawn song.
                let (from, to) = (OCARINA_SONG_MEMORY_GAME as usize * SONG_NOTES, OCARINA_SONG_SCARECROW_SPAWN as usize * SONG_NOTES);
                for i in 0..o.record_song_pos as usize {
                    o.song_notes[to + i] = o.song_notes.get(from + i).copied().unwrap_or_default();
                }
                o.is_input_enabled = false;
            } else {
                o.song_notes[spawn_second].volume = 0xFF;
            }
        }
        self.ocarina.recording_state = OCARINA_RECORD_OFF;
    }

    /// `AudioOcarina_SetRecordingState`: `OCARINA_RECORD_SCARECROW_LONG` or `_SPAWN` starts a
    /// recording (the input on, no note held yet); `OCARINA_RECORD_OFF` ends one (the last note
    /// written and the song completed, or with nothing played the long song's second note put
    /// back).
    pub fn audio_ocarina_set_recording_state(&mut self, recording_state: u8) {
        if recording_state as u32 == self.ocarina.recording_state as u32 {
            return;
        }
        if recording_state != OCARINA_RECORD_OFF {
            let o = &mut self.ocarina;
            o.record_task_start = o.update_task_start;
            o.record_pitch = OCARINA_PITCH_NONE;
            o.record_volume = 0x57;
            o.record_vibrato = 0;
            o.record_bend_index = 0;
            o.record_button_index = 0;
            o.record_song_pos = 0;
            o.is_input_enabled = true;
            o.staff_playing_pos = 0;
            o.scarecrows_long_song_second_note = o.scarecrows_long_song_notes[1];
        } else {
            if self.ocarina.record_song_pos == 0 {
                let o = &mut self.ocarina;
                o.scarecrows_long_song_notes[1] = o.scarecrows_long_song_second_note;
            } else {
                if self.ocarina.recording_state == OCARINA_RECORD_SCARECROW_SPAWN {
                    self.ocarina.staff_playing_pos = 1;
                }
                self.audio_ocarina_set_recording_song(true);
            }
            self.ocarina.is_input_enabled = false;
            self.ocarina.staff_playing_pos = 0;
        }
        self.ocarina.recording_state = recording_state;
    }

    /// `AudioOcarina_UpdateRecordingStaff`.
    fn audio_ocarina_update_recording_staff(&mut self) {
        let o = &mut self.ocarina;
        o.recording_staff.state = o.recording_state;
        o.recording_staff.pos = o.staff_playing_pos;
        if o.recording_state == OCARINA_RECORD_REJECTED {
            o.recording_state = OCARINA_RECORD_OFF;
        }
    }

    /// `AudioOcarina_UpdatePlayingStaff`.
    fn audio_ocarina_update_playing_staff(&mut self) {
        self.ocarina.playing_staff.button_index = self.ocarina.cur_button_index & 0x3F;
        let st = self.audio_ocarina_get_playing_state();
        let o = &mut self.ocarina;
        o.playing_staff.state = st;
        o.playing_staff.pos = o.staff_playing_pos;
    }

    /// `AudioOcarina_UpdatePlaybackStaff`.
    fn audio_ocarina_update_playback_staff(&mut self) {
        if (self.ocarina.playback_pitch & 0x3F) <= OCARINA_PITCH_EFLAT5 {
            let b = self.audio_ocarina_map_note_to_button(self.ocarina.playback_pitch);
            self.ocarina.playback_staff.button_index = b;
        }
        let o = &mut self.ocarina;
        o.playback_staff.state = o.playback_state;
        o.playback_staff.pos = if o.playback_song != PlaybackSong::ScarecrowLong {
            o.playback_staff_pos as u8
        } else if o.playback_staff_pos == 0 {
            0
        } else {
            (((o.playback_staff_pos - 1) % 8) + 1) as u8
        };
    }

    /// `AudioOcarina_GetRecordingStaff`.
    pub fn audio_ocarina_get_recording_staff(&mut self) -> StaffRef {
        StaffRef::Recording
    }

    /// `AudioOcarina_GetPlayingStaff`: a song's index read ends the listening.
    pub fn audio_ocarina_get_playing_staff(&mut self) -> StaffRef {
        if self.ocarina.playing_staff.state < 0xFE {
            self.ocarina.flags = 0;
        }
        StaffRef::Playing
    }

    /// `AudioOcarina_GetPlaybackStaff`.
    pub fn audio_ocarina_get_playback_staff(&mut self) -> StaffRef {
        StaffRef::Playback
    }

    /// `AudioOcarina_RecordSong`: while recording, every three audio tasks or more, a change of
    /// note (pitch, volume, vibrato or bend) writes the one held (`SetRecordingSong`). A new
    /// note is counted on the recording staff (its button, its position 1 to 8, round again
    /// for the long song); the spawn song ends at its eighth note's release, or at a ninth.
    fn audio_ocarina_record_song(&mut self) {
        let o = &mut self.ocarina;
        if o.recording_state == OCARINA_RECORD_OFF || o.update_task_start.wrapping_sub(o.record_task_start) < 3 {
            return;
        }
        let mut note_changed = false;
        if o.record_pitch != o.cur_pitch {
            if o.cur_pitch != OCARINA_PITCH_NONE {
                o.recording_staff.button_index = o.cur_button_index & 0x3F;
                o.staff_playing_pos = o.staff_playing_pos.wrapping_add(1);
            } else if o.recording_state == OCARINA_RECORD_SCARECROW_SPAWN && o.staff_playing_pos == 8 {
                self.audio_ocarina_set_recording_song(true);
                return;
            }
            let o = &mut self.ocarina;
            if o.staff_playing_pos > 8 {
                if o.recording_state == OCARINA_RECORD_SCARECROW_SPAWN {
                    // Over eight notes while recording the spawn song.
                    self.audio_ocarina_set_recording_song(true);
                    return;
                }
                o.staff_playing_pos = 1;
            }
            note_changed = true;
        } else if o.record_volume as i32 != o.cur_volume as i32 {
            note_changed = true;
        } else if o.record_vibrato as i32 != o.cur_vibrato as i32 {
            // !PLATFORM_N64: every version's change counts, the spawn song's too.
            note_changed = true;
        } else if o.record_bend_index != o.cur_bend_index {
            note_changed = true;
        }
        if note_changed {
            self.audio_ocarina_set_recording_song(false);
            let o = &mut self.ocarina;
            o.record_task_start = o.update_task_start;
        }
    }

    /// `AudioOcarina_MemoryGameInit`: the Skull Kids' memory game's round (0 to 2, past 2 the
    /// last) ends at `sOcaMemoryGameNumNotes[round]` notes; it starts with three.
    pub fn audio_ocarina_memory_game_init(&mut self, minigame_round: u8) {
        /// `sOcaMemoryGameNumNotes`.
        const MEMORY_GAME_NUM_NOTES: [u8; 3] = [5, 6, 8];
        let minigame_round = minigame_round.min(2);
        self.ocarina.memory_game_append_pos = 0;
        self.ocarina.memory_game_end_pos = MEMORY_GAME_NUM_NOTES[minigame_round as usize];
        for _ in 0..3 {
            self.audio_ocarina_memory_game_next_note();
        }
    }

    /// `AudioOcarina_MemoryGameNextNote`: 1 when the round's notes are all there; else a random
    /// note appended to `OCARINA_SONG_MEMORY_GAME` (not the same as the one before), the song
    /// ended after it, and 0.
    pub fn audio_ocarina_memory_game_next_note(&mut self) -> i32 {
        if self.ocarina.memory_game_append_pos == self.ocarina.memory_game_end_pos {
            return 1;
        }
        let random_button_index = self.next_random();
        let o = &mut self.ocarina;
        let mut random_pitch = o.button_to_pitch((random_button_index % 5) as u8);
        let pos = o.memory_game_append_pos as i32;
        // @bug (game): the first note compares with sOcarinaSongNotes[OCARINA_SONG_MEMORY_GAME][-1],
        // the scarecrow's spawn song's last entry.
        if o.song_note(OCARINA_SONG_MEMORY_GAME, pos - 1).pitch == random_pitch {
            random_pitch = o.button_to_pitch((random_button_index.wrapping_add(1) % 5) as u8);
        }
        let base = OCARINA_SONG_MEMORY_GAME as usize * SONG_NOTES;
        let i = base + pos as usize;
        o.song_notes[i].pitch = random_pitch;
        // FRAMERATE_CONST(45, 38): not a 50 Hz N64 PAL version.
        o.song_notes[i].length = 45;
        o.song_notes[i].volume = 0x50;
        o.song_notes[i].vibrato = 0;
        o.song_notes[i].bend = 0;
        o.memory_game_append_pos += 1;
        let i = base + o.memory_game_append_pos as usize;
        o.song_notes[i].pitch = OCARINA_PITCH_NONE;
        o.song_notes[i].length = 0;
        o.song_notes[i + 1].pitch = OCARINA_PITCH_NONE;
        o.song_notes[i + 1].length = 0;
        0
    }

    /// `AudioOcarina_Update`, the start of `Audio_Update`: the pad read and played, the songs
    /// checked, the playback stepped, the staffs updated for the message box.
    pub fn audio_ocarina_update(&mut self) {
        self.ocarina.update_task_start = self.view.total_task_count as u32;
        if self.ocarina.instrument_id as u8 != OCARINA_INSTRUMENT_OFF {
            if self.ocarina.is_input_enabled {
                self.audio_ocarina_read_controller_input();
            }
            if self.ocarina.playback_state == 0 && self.ocarina.is_input_enabled {
                self.audio_ocarina_play_controller_input();
            }
            if self.ocarina.flags != 0 {
                if self.ocarina.flags & 0x4000 != 0 {
                    self.audio_ocarina_check_songs_without_music_staff();
                } else {
                    self.audio_ocarina_check_songs_with_music_staff();
                }
            }
            self.audio_ocarina_playback_song();
            self.ocarina.playback_task_start = self.ocarina.update_task_start as i32;
            if self.ocarina.playback_state == 0 {
                self.audio_ocarina_record_song();
            }
            let o = &mut self.ocarina;
            if o.flags != 0 && o.prev_pitch != o.cur_pitch {
                o.drop_input_timer = 1; // The input dropped for a frame.
            }
            o.prev_pitch = o.cur_pitch;
        }
        self.audio_ocarina_update_playing_staff();
        self.audio_ocarina_update_playback_staff();
        self.audio_ocarina_update_recording_staff();
    }

    /// `AudioOcarina_PlayLongScarecrowSong` (after the credits, `CS_CMD_PLAY_LONG_SCARECROW_SONG`
    /// each frame): every 1,200 frames the long song played on the next instrument, from the
    /// default to the flute; then the ocarina off.
    pub fn audio_ocarina_play_long_scarecrow_song(&mut self) {
        /// `OCARINA_INSTRUMENT_MAX` (`ocarina.h`).
        const OCARINA_INSTRUMENT_MAX: u8 = 7;
        match self.ocarina.scarecrow_after_credits_state {
            0 => {
                let o = &mut self.ocarina;
                let t = o.scarecrow_after_credits_timer;
                o.scarecrow_after_credits_timer = t.wrapping_sub(1);
                if t == 0 {
                    if o.scarecrow_after_credits_instrument_id < OCARINA_INSTRUMENT_MAX {
                        // The next instrument, again.
                        o.scarecrow_after_credits_state += 1;
                    } else {
                        // Finished.
                        o.scarecrow_after_credits_state = 3;
                        self.audio_ocarina_set_instrument(OCARINA_INSTRUMENT_OFF);
                    }
                    self.ocarina.scarecrow_after_credits_timer = 1200;
                }
            }
            1 => {
                self.set_sfx_banks_mute(0);
                let id = self.ocarina.scarecrow_after_credits_instrument_id;
                self.audio_ocarina_set_instrument(id);
                self.audio_ocarina_set_playback_song(OCARINA_SONG_SCARECROW_LONG as i8 + 1, 1);
                let o = &mut self.ocarina;
                o.scarecrow_after_credits_instrument_id += 1;
                o.scarecrow_after_credits_state += 1;
            }
            2 => {
                if self.ocarina.playback_staff.state == 0 {
                    self.ocarina.scarecrow_after_credits_state = 0;
                }
            }
            _ => {}
        }
    }

    /// `AudioOcarina_ResetStaffs`.
    pub fn audio_ocarina_reset_staffs(&mut self) {
        let o = &mut self.ocarina;
        o.playing_staff = OcarinaStaff { button_index: OCARINA_BTN_INVALID, state: 0xFF, pos: 0 };
        o.playback_staff = OcarinaStaff { button_index: OCARINA_BTN_INVALID, state: 0, pos: 0 };
        o.recording_staff = OcarinaStaff { button_index: OCARINA_BTN_INVALID, state: OCARINA_RECORD_REJECTED, pos: 0 };
        o.drop_input_timer = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tables as the C has them (a test's own, not the pack's): Saria's and the Sun's
    /// Song's notes and buttons, the maps.
    fn tables() -> super::super::AudioGameTables {
        let n = |pitch, length, volume, vibrato| OcarinaNote { pitch, length, volume, vibrato, bend: 0, b_flat4_flag: 0 };
        let mut notes = vec![vec![n(OCARINA_PITCH_NONE, 0, 0, 0)]; OCARINA_SONG_MAX as usize];
        // OCARINA_SONG_SARIAS (60 Hz lengths).
        notes[OCARINA_SONG_SARIAS as usize] = vec![n(0x5, 17, 84, 0), n(0x9, 17, 88, 0), n(0xB, 34, 80, 0), n(0x5, 17, 84, 0), n(0x9, 17, 88, 0), n(0xB, 136, 80, 0), n(0xFF, 0, 90, 0)];
        notes[OCARINA_SONG_SCARECROW_SPAWN as usize] = vec![n(0x2, 3, 0, 0), n(0xFF, 0, 255, 0)];
        let mut buttons = vec![OcarinaSongButtons::default(); OCARINA_SONG_MAX as usize];
        buttons[OCARINA_SONG_SARIAS as usize] = OcarinaSongButtons { num_buttons: 6, buttons_index: [1, 2, 3, 1, 2, 3, 0, 0] };
        buttons[OCARINA_SONG_SUNS as usize] = OcarinaSongButtons { num_buttons: 6, buttons_index: [2, 1, 4, 2, 1, 4, 0, 0] };
        super::super::AudioGameTables {
            ocarina_song_notes: notes,
            ocarina_song_buttons: buttons,
            button_to_pitch_map: vec![0x2, 0x5, 0x9, 0xB, 0xE],
            pitch_to_button_map: vec![0, 0, 0, 0, 1, 1, 1, 2, 2, 2, 5, 3, 3, 4, 4, 4],
            ..Default::default()
        }
    }

    fn audio() -> GameAudio {
        let t = std::sync::Arc::new(tables());
        let mut a = GameAudio::new(t, Default::default());
        a.view.players[SEQ_PLAYER_SFX as usize].enabled = true;
        a
    }

    fn frame(a: &mut GameAudio, button: u16) {
        a.ocarina.pad = OcarinaPad { button, stick_x: 0, stick_y: 0 };
        a.view.total_task_count += 3;
        a.audio_ocarina_update();
    }

    // AudioOcarina_Start(flags + 0xC000), free play: Saria's Song's buttons (C-Down, C-Right,
    // C-Left twice) give song 6 once its sixth note is played; the staff's state reads it once.
    #[test]
    fn free_play_recognises_sarias_song() {
        let mut a = audio();
        a.audio_ocarina_set_instrument(OCARINA_INSTRUMENT_DEFAULT);
        a.audio_ocarina_start((1 << OCARINA_SONG_SARIAS) | (1 << OCARINA_SONG_SUNS) | 0xC000);
        assert_eq!(a.ocarina.playing_staff.state, 0xFE);
        let mut played = None;
        for b in [BTN_CDOWN, BTN_CRIGHT, BTN_CLEFT, BTN_CDOWN, BTN_CRIGHT, BTN_CLEFT] {
            // Pressed for two frames (the input drop eats one), then released for two.
            for held in [b, b, 0, 0] {
                frame(&mut a, held);
                if a.ocarina.playing_staff.state < 0xFE {
                    played = Some(a.ocarina.playing_staff.state);
                }
            }
        }
        assert_eq!(played, Some(OCARINA_SONG_SARIAS));
        assert_eq!(a.ocarina.flags, 0);
    }

    // The note's pitch: R a semitone up (B-flat 4 from A4, flagged 0x80), Z down.
    #[test]
    fn r_and_z_shift_the_pitch() {
        let mut a = audio();
        a.audio_ocarina_set_instrument(OCARINA_INSTRUMENT_DEFAULT);
        a.audio_ocarina_start(0xC000 | (1 << OCARINA_SONG_SARIAS));
        frame(&mut a, BTN_CRIGHT | BTN_R);
        assert_eq!(a.ocarina.cur_pitch, OCARINA_PITCH_BFLAT4);
        assert_eq!(a.ocarina.cur_button_index, OCARINA_BTN_C_RIGHT + 0x80);
        assert_eq!(a.audio_ocarina_map_note_to_button(OCARINA_PITCH_BFLAT4 | 0x80), OCARINA_BTN_C_RIGHT);
        assert_eq!(a.audio_ocarina_map_note_to_button(OCARINA_PITCH_BFLAT4 | 0x40), OCARINA_BTN_C_LEFT);
    }

    // The button that took the ocarina out (C-Left here) plays no note until it's let go. The
    // first frame's change of pitch (from the boot's C4 to none) drops the next frame's input,
    // so the release takes two frames.
    #[test]
    fn the_starting_button_plays_no_note() {
        let mut a = audio();
        a.ocarina.pad.button = BTN_CLEFT;
        a.audio_ocarina_set_instrument(OCARINA_INSTRUMENT_DEFAULT);
        assert_eq!(a.ocarina.input_button_start, BTN_CLEFT as u32);
        a.audio_ocarina_start(0xC000);
        frame(&mut a, BTN_CLEFT);
        assert_eq!(a.ocarina.cur_pitch, OCARINA_PITCH_NONE);
        assert_eq!(a.ocarina.drop_input_timer, 1);
        frame(&mut a, 0);
        assert_eq!(a.ocarina.input_button_start, BTN_CLEFT as u32);
        frame(&mut a, 0);
        assert_eq!(a.ocarina.input_button_start, 0);
        frame(&mut a, BTN_CLEFT);
        assert_eq!(a.ocarina.cur_pitch, OCARINA_PITCH_B4);
    }

    // AudioOcarina_SetPlaybackSong(SARIAS + 1, 1): the notes follow the table's lengths (the
    // second one a frame longer), the staff counting them; the state drops to 0 at the end.
    #[test]
    fn playback_steps_through_the_song() {
        let mut a = audio();
        a.audio_ocarina_set_instrument(OCARINA_INSTRUMENT_DEFAULT);
        a.audio_ocarina_set_playback_song(OCARINA_SONG_SARIAS as i8 + 1, 1);
        let mut seen = Vec::new();
        for _ in 0..200 {
            frame(&mut a, 0);
            let s = a.ocarina.playback_staff;
            if seen.last() != Some(&(s.pos, s.button_index)) && s.pos != 0 {
                seen.push((s.pos, s.button_index));
            }
            if s.state == 0 {
                break;
            }
        }
        let buttons: Vec<u8> = seen.iter().map(|&(_, b)| b).collect();
        assert_eq!(buttons, vec![OCARINA_BTN_C_DOWN, OCARINA_BTN_C_RIGHT, OCARINA_BTN_C_LEFT, OCARINA_BTN_C_DOWN, OCARINA_BTN_C_RIGHT, OCARINA_BTN_C_LEFT]);
        assert_eq!(a.ocarina.playback_staff.state, 0);
    }

    // AudioOcarina_BendPitchTwoSemitones: 64 and over is the table's top (127), -64 and under
    // its bottom, 0 the middle.
    #[test]
    fn bend_index_clamps() {
        let mut a = audio();
        let mut t = (*a.audio_tables).clone();
        t.bend_pitch_two_semitones_frequencies = (0..256).map(|i| i as f32).collect();
        a.audio_tables = std::sync::Arc::new(t);
        assert_eq!(a.audio_ocarina_bend_pitch_two_semitones(0), 128.0);
        assert_eq!(a.audio_ocarina_bend_pitch_two_semitones(65), 255.0);
        assert_eq!(a.audio_ocarina_bend_pitch_two_semitones(-65), 0.0);
        assert_eq!(a.audio_ocarina_bend_pitch_two_semitones(32), (128 + 63) as f32);
    }

    // AudioOcarina_MemoryGameInit(0): three notes of the round's five; each further note until
    // the fifth gives 0, then 1. With the audio side's random state all zero, every draw is
    // button 0 (sButtonToPitchMap[0], D4), and a note the same as the one before takes the next
    // button's (F4): D4 F4 D4 F4 D4. Each is 45 frames (FRAMERATE_CONST(45, 38)) at volume 0x50,
    // and the song ends after the last (two OCARINA_PITCH_NONE entries).
    #[test]
    fn the_memory_game_appends_a_note_a_round() {
        let mut a = audio();
        a.audio_ocarina_memory_game_init(0);
        assert_eq!((a.ocarina.memory_game_append_pos, a.ocarina.memory_game_end_pos), (3, 5));
        assert_eq!(a.audio_ocarina_memory_game_next_note(), 0);
        assert_eq!(a.audio_ocarina_memory_game_next_note(), 0);
        assert_eq!(a.audio_ocarina_memory_game_next_note(), 1);
        let notes: Vec<OcarinaNote> = (0..7).map(|i| a.ocarina.song_note(OCARINA_SONG_MEMORY_GAME, i)).collect();
        let pitches: Vec<u8> = notes.iter().map(|n| n.pitch).collect();
        assert_eq!(pitches, vec![OCARINA_PITCH_D4, OCARINA_PITCH_F4, OCARINA_PITCH_D4, OCARINA_PITCH_F4, OCARINA_PITCH_D4, OCARINA_PITCH_NONE, OCARINA_PITCH_NONE]);
        assert!(notes[..5].iter().all(|n| n.length == 45 && n.volume == 0x50 && n.vibrato == 0 && n.bend == 0));
        assert_eq!((notes[5].length, notes[6].length), (0, 0));
        // Rounds past the third play the third's eight.
        a.audio_ocarina_memory_game_init(5);
        assert_eq!(a.ocarina.memory_game_end_pos, 8);
    }

    // @bug (game): the first note compares with sOcarinaSongNotes[OCARINA_SONG_MEMORY_GAME][-1],
    // the scarecrow's spawn song's last entry: a D4 there makes the first note F4.
    #[test]
    fn the_memory_games_first_note_reads_the_song_before() {
        let mut a = audio();
        let i = OCARINA_SONG_MEMORY_GAME as usize * SONG_NOTES - 1;
        a.ocarina.song_notes[i].pitch = OCARINA_PITCH_D4;
        a.audio_ocarina_memory_game_init(0);
        assert_eq!(a.ocarina.song_note(OCARINA_SONG_MEMORY_GAME, 0).pitch, OCARINA_PITCH_F4);
    }

    /// `audio()` with every song but Saria's and the Sun's eight C-Ups and an A (none of the
    /// recordings below holds them), and the spawn song unset (its second note's volume 0xFF).
    fn recording_audio() -> GameAudio {
        let mut a = audio();
        for s in 0..OCARINA_SONG_SCARECROW_SPAWN {
            if s != OCARINA_SONG_SARIAS && s != OCARINA_SONG_SUNS {
                a.ocarina.song_buttons[s as usize] = OcarinaSongButtons { num_buttons: 8, buttons_index: [4, 4, 4, 4, 4, 4, 4, 0] };
            }
        }
        a.audio_ocarina_set_instrument(OCARINA_INSTRUMENT_DEFAULT);
        a
    }

    /// Each button pressed for two frames and let go for two.
    fn play(a: &mut GameAudio, buttons: &[u16]) {
        for &b in buttons {
            for held in [b, b, 0, 0] {
                frame(a, held);
            }
        }
    }

    // AudioOcarina_SetRecordingState(OCARINA_RECORD_SCARECROW_SPAWN), then eight notes: the
    // staff counts them; the eighth's release ends the recording (AudioOcarina_RecordSong's
    // SetRecordingSong(true)). The buffer (the memory game's song) is shifted past its first
    // entry (the rest before the first note), its buttons mapped into the spawn song's, none of
    // the songs in them, not one note eight times: copied into the spawn song (its rests at
    // the boot's volume 0x57, set: its second entry's volume isn't 0xFF), the input off.
    #[test]
    fn the_spawn_recording_keeps_eight_new_notes() {
        let mut a = recording_audio();
        a.audio_ocarina_set_recording_state(OCARINA_RECORD_SCARECROW_SPAWN);
        assert!(a.ocarina.is_input_enabled);
        let song = [BTN_A, BTN_CDOWN, BTN_CRIGHT, BTN_CLEFT, BTN_CUP, BTN_A, BTN_CDOWN, BTN_CLEFT];
        play(&mut a, &song[..7]);
        assert_eq!((a.ocarina.recording_staff.state, a.ocarina.recording_staff.pos), (OCARINA_RECORD_SCARECROW_SPAWN, 7));
        play(&mut a, &song[7..]);
        assert_eq!(a.ocarina.recording_state, OCARINA_RECORD_OFF);
        assert_eq!(a.ocarina.recording_staff.state, OCARINA_RECORD_OFF);
        assert!(!a.ocarina.is_input_enabled);
        let buttons = a.ocarina.song_buttons(OCARINA_SONG_SCARECROW_SPAWN).buttons_index;
        assert_eq!(buttons, [OCARINA_BTN_A, OCARINA_BTN_C_DOWN, OCARINA_BTN_C_RIGHT, OCARINA_BTN_C_LEFT, OCARINA_BTN_C_UP, OCARINA_BTN_A, OCARINA_BTN_C_DOWN, OCARINA_BTN_C_LEFT]);
        let notes: Vec<OcarinaNote> = (0..15).map(|i| a.ocarina.song_note(OCARINA_SONG_SCARECROW_SPAWN, i)).collect();
        let pitches: Vec<u8> = notes.iter().map(|n| n.pitch).collect();
        let r = OCARINA_PITCH_NONE;
        assert_eq!(pitches, vec![0x2, r, 0x5, r, 0x9, r, 0xB, r, 0xE, r, 0x2, r, 0x5, r, 0xB]);
        // Each held two frames (six audio tasks), each rest two.
        assert!(notes.iter().all(|n| n.length == 6 && n.volume == 0x57), "{notes:?}");
        assert_ne!(a.ocarina.song_note(OCARINA_SONG_SCARECROW_SPAWN, 1).volume, 0xFF);
        // The save's bytes, and back (Sram_OpenSave).
        let bytes = a.ocarina.scarecrow_spawn_song_bytes();
        assert_eq!(&bytes[..8], &[0x2, 0, 0, 6, 0x57, 0, 0, 0]);
        let mut b = recording_audio();
        let mut save = crate::save::SaveContext::default();
        (save.scarecrow_spawn_song_set, save.scarecrow_spawn_song) = (1, bytes);
        b.ocarina.sram_open_save_scarecrow_songs(&save);
        assert_eq!((0..15).map(|i| b.ocarina.song_note(OCARINA_SONG_SCARECROW_SPAWN, i)).collect::<Vec<_>>(), notes);
    }

    // A spawn recording holding a song (Saria's, from its second note) is refused: the staff
    // reads OCARINA_RECORD_REJECTED once (then off), the spawn song unset (its second entry's
    // volume 0xFF). So is one note eight times.
    #[test]
    fn the_spawn_recording_refuses_a_song_or_one_note() {
        let mut a = recording_audio();
        a.audio_ocarina_set_recording_state(OCARINA_RECORD_SCARECROW_SPAWN);
        play(&mut a, &[BTN_A, BTN_CDOWN, BTN_CRIGHT, BTN_CLEFT, BTN_CDOWN, BTN_CRIGHT, BTN_CLEFT]);
        assert_eq!(a.ocarina.recording_state, OCARINA_RECORD_SCARECROW_SPAWN);
        // The eighth note's release.
        for held in [BTN_A, BTN_A, 0] {
            frame(&mut a, held);
        }
        assert_eq!(a.ocarina.recording_staff.state, OCARINA_RECORD_REJECTED);
        assert_eq!(a.ocarina.song_note(OCARINA_SONG_SCARECROW_SPAWN, 1).volume, 0xFF);
        frame(&mut a, 0);
        assert_eq!(a.ocarina.recording_staff.state, OCARINA_RECORD_OFF);

        let mut a = recording_audio();
        a.audio_ocarina_set_recording_state(OCARINA_RECORD_SCARECROW_SPAWN);
        play(&mut a, &[BTN_CUP; 7]);
        for held in [BTN_CUP, BTN_CUP, 0] {
            frame(&mut a, held);
        }
        assert_eq!(a.ocarina.recording_staff.state, OCARINA_RECORD_REJECTED);
    }

    // Fewer than eight notes when the recording is turned off (the box's B): the spawn song
    // unset. The long song's: AudioOcarina_SetRecordingState(OCARINA_RECORD_OFF) writes the
    // note held (here the rest after the third), cuts the trailing rests to one (its length
    // zeroed) and ends the song (a zero length); the staff's position goes round 1 to 8.
    #[test]
    fn recordings_turned_off() {
        let mut a = recording_audio();
        a.audio_ocarina_set_recording_state(OCARINA_RECORD_SCARECROW_SPAWN);
        play(&mut a, &[BTN_A, BTN_CDOWN]);
        a.audio_ocarina_set_recording_state(OCARINA_RECORD_OFF);
        assert_eq!(a.ocarina.song_note(OCARINA_SONG_SCARECROW_SPAWN, 1).volume, 0xFF);
        assert!(!a.ocarina.is_input_enabled);

        let mut a = recording_audio();
        a.audio_ocarina_set_recording_state(OCARINA_RECORD_SCARECROW_LONG);
        play(&mut a, &[BTN_A, BTN_CUP, BTN_CDOWN]);
        for _ in 0..10 {
            frame(&mut a, 0);
        }
        a.audio_ocarina_set_recording_state(OCARINA_RECORD_OFF);
        let notes: Vec<OcarinaNote> = a.ocarina.scarecrows_long_song_notes[..8].to_vec();
        let r = OCARINA_PITCH_NONE;
        assert_eq!(notes.iter().map(|n| n.pitch).collect::<Vec<_>>()[..7], [r, 0x2, r, 0xE, r, 0x5, r]);
        // The first rest (from the start to the first press) a frame, the last zeroed, then the
        // end.
        assert_eq!(notes.iter().map(|n| n.length).collect::<Vec<_>>()[..8], [3, 6, 6, 6, 6, 6, 0, 0]);
        assert_eq!(a.ocarina.recording_state, OCARINA_RECORD_OFF);

        let mut a = recording_audio();
        a.audio_ocarina_set_recording_state(OCARINA_RECORD_SCARECROW_LONG);
        let mut seen = Vec::new();
        for b in [BTN_A; 10] {
            play(&mut a, &[b]);
            seen.push(a.ocarina.recording_staff.pos);
        }
        assert_eq!(seen, vec![1, 2, 3, 4, 5, 6, 7, 8, 1, 2]);
    }
}
