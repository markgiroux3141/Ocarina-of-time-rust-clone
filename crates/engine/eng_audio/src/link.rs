//! Between the game's thread and the audio thread: what the game does to the audio library
//! (`GameOp`), and what it reads of it (`AudioView`).
//!
//! On the console both threads share `gAudioContext`. The game queues commands
//! (`Audio_QueueCmd*`), hands them over (`Audio_ScheduleProcessCmds`), and reads a few fields
//! directly: whether a player is enabled and its tempo, the IO ports, the channels' note
//! priorities, the reset and load queues' messages. The audio thread runs on every retrace,
//! three times per 20 Hz game frame.
//!
//! Here the two meet at the end of each game frame (docs/adr/0026-the-games-audio.md): the
//! game's frame produces `GameOp`s, the audio side applies them in order (`AudioContext::apply`)
//! and runs its retraces, and the game's next frame reads an `AudioView` taken after them
//! (`AudioContext::view`). Offline that's `Renderer` (three retraces per game frame); in the
//! window, `output::AudioOutput`, whose thread runs the retraces on the clock.

use crate::context::AudioContext;

/// One thing the game's thread does to the audio library, in the order it did them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameOp {
    /// `Audio_QueueCmd` (and its typed variants): the op and args word, and the data word.
    Cmd(u32, u32),
    /// `Audio_ScheduleProcessCmds`.
    Schedule,
    /// `Audio_ResetCmdQueue`.
    ResetCmdQueue,
    /// `func_800E5F88` (the spec change, `0xF9`): the audio side of it, as `AudioContext::func_800e5f88`.
    ResetSpec(u8),
    /// `Audio_NextRandom`'s `audRand` as the game's thread left it (both threads call it).
    SetAudRand(u32),
}

/// A sequence channel as the game reads it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChannelView {
    /// Not `sequenceChannelNone` (`IS_SEQUENCE_CHANNEL_VALID`).
    pub valid: bool,
    /// `soundScriptIO`: the channel's IO ports (`func_800E6070`).
    pub sound_script_io: [i8; 8],
    /// `notePriority` (`Audio_SplitBgmChannels`).
    pub note_priority: u8,
}

/// A sequence player as the game reads it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlayerView {
    pub enabled: bool,
    /// `seqId` (not read by the game; for logs and tests).
    pub seq_id: u8,
    pub tempo: u16,
    /// `soundScriptIO`: the player's IO ports (`func_800E60C4`).
    pub sound_script_io: [i8; 8],
    pub channels: [ChannelView; 16],
}

/// What the game's thread reads of `gAudioContext`, taken after the audio side's retraces.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AudioView {
    pub players: [PlayerView; 4],
    /// `audioBufferParameters.updatesPerFrame`.
    pub updates_per_frame: i16,
    pub total_task_count: i32,
    pub reset_status: u8,
    pub audio_reset_spec_id_to_load: u8,
    /// `func_800E6680`: the notes sounding.
    pub sounding_notes: i32,
    /// `audioRandom`, `Audio_NextRandom`'s `audRand`, and `osGetCount()` as the audio side last
    /// advanced it (the game's thread reads the count at its own time; here, at the view's).
    pub audio_random: u32,
    pub aud_rand: u32,
    pub os_count: u32,
    /// The messages posted to `audioResetQueue` and `externalLoadQueue` since the last view, in
    /// order: the game receives them (`func_800E5EDC`, `func_800E5E20`).
    pub reset_msgs: Vec<u32>,
    pub external_load_msgs: Vec<u32>,
}

impl AudioView {
    /// What the game reads before the audio side's first frame: `AudioLoad_Init` has loaded
    /// spec 0 at `refresh_rate` 60 (NTSC), no player is on. `updatesPerFrame` is
    /// `AudioHeap_Init`'s arithmetic (`audio_heap.c`). The players' other fields stay zero
    /// (the library's are its init's): the game reads them only of a player that plays.
    pub fn boot(tables: &crate::data::AudioTables) -> AudioView {
        const REFRESH_RATE: u32 = 60;
        let updates_per_frame = tables
            .specs
            .first()
            .map(|spec| {
                let samples_per_frame_target = crate::context::align16(spec.sampling_frequency as u16 as u32 / REFRESH_RATE) as i16;
                (((samples_per_frame_target + 0x10) / 0xD0) + 1) * spec.unk_04 as i16
            })
            .unwrap_or(0);
        AudioView { updates_per_frame, ..AudioView::default() }
    }

    /// Takes `newer`'s state, and adds its messages to the ones not received yet.
    pub fn update(&mut self, mut newer: AudioView) {
        let mut reset = std::mem::take(&mut self.reset_msgs);
        let mut ext = std::mem::take(&mut self.external_load_msgs);
        reset.append(&mut newer.reset_msgs);
        ext.append(&mut newer.external_load_msgs);
        *self = AudioView { reset_msgs: reset, external_load_msgs: ext, ..newer };
    }
}

impl AudioContext {
    /// Applies what the game's thread did, in order.
    pub fn apply(&mut self, ops: &[GameOp]) {
        for op in ops {
            match *op {
                GameOp::Cmd(op_args, data) => self.queue_cmd(op_args, data),
                GameOp::Schedule => {
                    self.schedule_process_cmds();
                }
                GameOp::ResetCmdQueue => self.reset_cmd_queue(),
                GameOp::ResetSpec(id) => {
                    self.func_800e5f88(id as i32);
                }
                GameOp::SetAudRand(v) => self.aud_rand = v,
            }
        }
    }

    /// What the game reads, now. Takes the reset and load queues' messages (the game receives
    /// them from the view).
    pub fn view(&mut self) -> AudioView {
        let mut players = [PlayerView::default(); 4];
        for (i, p) in players.iter_mut().enumerate() {
            let sp = &self.seq_players[i];
            p.enabled = sp.enabled;
            p.seq_id = sp.seq_id;
            p.tempo = sp.tempo;
            p.sound_script_io = sp.sound_script_io;
            for (c, cv) in p.channels.iter_mut().enumerate() {
                let ch = &self.channels[sp.channels[c]];
                cv.valid = sp.channels[c] != crate::context::CHANNEL_NONE;
                cv.sound_script_io = ch.sound_script_io;
                cv.note_priority = ch.note_priority;
            }
        }
        AudioView {
            players,
            updates_per_frame: self.audio_buffer_parameters.updates_per_frame,
            total_task_count: self.total_task_count,
            reset_status: self.reset_status,
            audio_reset_spec_id_to_load: self.audio_reset_spec_id_to_load,
            sounding_notes: self.func_800e6680(),
            audio_random: self.audio_random,
            aud_rand: self.aud_rand,
            os_count: self.os_count,
            reset_msgs: self.audio_reset_queue.drain(..).collect(),
            external_load_msgs: self.external_load_queue.drain(..).collect(),
        }
    }

    /// `func_800E5F34`: empties `audioResetQueue`.
    pub fn func_800e5f34(&mut self) {
        self.audio_reset_queue.clear();
    }

    /// `func_800E5F88`: starts the reset that loads spec `reset_preload_id` (`0xF9`).
    ///
    /// The C runs on the game's thread; here it runs as the audio side takes the game's frame
    /// (`GameOp::ResetSpec`), so the queues it empties and the fields it reads are the audio
    /// side's at that moment. Where the C blocks on `audioResetQueue` until a reset under way
    /// finishes (its status 1 or 2, so at most two more audio frames), the reset's steps run
    /// here at once, without those frames' output.
    pub fn func_800e5f88(&mut self, reset_preload_id: i32) -> i32 {
        self.func_800e5f34();
        let reset_status = self.reset_status;
        if reset_status != 0 {
            self.reset_cmd_queue();
            if self.audio_reset_spec_id_to_load as i32 == reset_preload_id {
                return -2;
            } else if reset_status > 2 {
                self.audio_reset_spec_id_to_load = reset_preload_id as u8;
                return -3;
            } else {
                // osRecvMesg(audioResetQueueP, OS_MESG_BLOCK): the reset finishes, and the game
                // takes its message.
                while self.reset_status != 0 {
                    self.reset_step();
                }
            }
        }
        self.func_800e5f34();
        self.queue_cmd(0xF900_0000, reset_preload_id as u32);
        self.schedule_process_cmds()
    }

    /// `func_800E6680`: the notes sounding.
    pub fn func_800e6680(&mut self) -> i32 {
        self.func_800e66c0(0)
    }
}
