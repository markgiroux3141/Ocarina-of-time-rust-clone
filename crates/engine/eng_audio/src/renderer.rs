//! Running the library offline: VI retraces at the refresh rate, each one an audio frame
//! (`AudioMgr_HandleRetrace`), with the AI playing the buffers it's handed at its DAC rate.
//! What the AI plays is the output: interleaved stereo at `ai_sampling_frequency`.

use crate::context::AudioContext;
use crate::data::AudioData;
use crate::link::{AudioView, GameOp};
use crate::thread::Ai;

pub struct Renderer {
    pub ctx: AudioContext,
    pub ai: Ai,
    /// Everything the AI has played: interleaved stereo frames.
    pub out: Vec<i16>,
    /// Retraces run so far.
    pub retraces: u64,
}

impl Renderer {
    /// `AudioLoad_Init` on the pack's audio data.
    pub fn new(data: &AudioData) -> Renderer {
        Renderer { ctx: AudioContext::new(data), ai: Ai::new(), out: Vec::new(), retraces: 0 }
    }

    /// The rate the output plays at (`osAiSetFrequency(32000)` on NTSC: 32006 Hz).
    pub fn output_rate(&self) -> u32 {
        self.ctx.audio_buffer_parameters.ai_sampling_frequency as u32
    }

    /// One VI period: the AI plays for 1/60 s, then the retrace runs the audio thread, which
    /// hands the AI its next buffer.
    pub fn retrace(&mut self) {
        let frames = self.output_rate() as f64 / self.ctx.refresh_rate.max(1) as f64;
        self.ai.play(frames, &mut self.out);
        let remaining = self.ai.remaining_in_current();
        if let Some(buf) = self.ctx.audio_thread_update(remaining) {
            self.ai.set_next_buffer(buf);
        }
        self.retraces += 1;
    }

    /// `n` retraces.
    pub fn run(&mut self, n: u64) {
        for _ in 0..n {
            self.retrace();
        }
    }

    /// Queues a command as the game does (`AudioThread_QueueCmd*`, then `AudioThread_ScheduleProcessCmds`
    /// at the end of its frame); the next retrace runs it.
    pub fn command(&mut self, op_args: u32, data: u32) {
        self.ctx.queue_cmd(op_args, data);
        self.ctx.schedule_process_cmds();
    }

    /// One game frame's hand-over (docs/adr/0026-the-games-audio.md): what the game's thread did
    /// (`ops`), then `retraces` VI retraces (3 per 20 Hz game frame), then what the game reads
    /// next.
    pub fn game_frame(&mut self, ops: &[GameOp], retraces: u64) -> AudioView {
        self.ctx.apply(ops);
        self.run(retraces);
        self.ctx.view()
    }

    /// Takes what the AI has played so far.
    pub fn take_output(&mut self) -> Vec<i16> {
        std::mem::take(&mut self.out)
    }

    /// `Audio_QueueSeqCmd`'s simplest: start `seq_id` on `player` (`0x82` with a fade-in of 0).
    pub fn start_sequence(&mut self, player: u8, seq_id: u8) {
        self.command(0x8200_0000 | ((player as u32) << 16) | ((seq_id as u32) << 8), 0);
    }

    /// Not in the C: plays a sequence script of our own on `player` with `font_id` as its
    /// font, as `AudioLoad_SyncInitSeqPlayerInternal` starts a sequence (the font loaded and
    /// relocated, the player reset and pointed at the script). For tools and tests: one note
    /// through the whole library (`note_script`).
    pub fn play_script(&mut self, player: usize, font_id: u32, script: &[u8]) {
        let c = &mut self.ctx;
        c.sequence_player_disable(player);
        c.sync_load_font(font_id);
        let at = SCRIPT_RAM + 0x1000 * player as u32;
        c.ram.write(at, script);
        c.reset_sequence_player(player);
        let default_font = c.get_real_table_index(crate::context::FONT_TABLE, font_id) as u8;
        let sp = &mut c.seq_players[player];
        // No sequence table entry: 0xFF passes `AudioLoad_IsSeqLoadComplete`.
        sp.seq_id = 0xFF;
        sp.default_font = default_font;
        sp.seq_data = at;
        sp.enabled = true;
        sp.script_state.set_pc(at);
        sp.script_state.set_depth(0);
        sp.delay = 0;
        sp.finished = false;
        sp.player_idx = player as i8;
    }
}

/// Where `play_script` puts its scripts in RAM (past the C's data, `load.rs`).
const SCRIPT_RAM: u32 = 0x8038_0000;

/// A sequence that plays one note: the player sets its tempo (`bpm`) and volume and starts
/// channel 0, which takes `instrument` (0x7F: the drums, the note picks one) and starts layer
/// 0, which plays `semitone` (`gPitchFrequencies`' index; for drums the drum's) at `velocity`
/// (0-127) for `ticks` tatums, then ends (the note decays). The channel and the player wait
/// `hold` tatums before they end too. No reverb.
pub fn note_script(instrument: u8, semitone: u8, velocity: u8, ticks: u16, hold: u16, bpm: u8) -> Vec<u8> {
    note_script_reverb(instrument, semitone, velocity, ticks, hold, bpm, 0)
}

/// `note_script` with the channel's reverb (`D4`: how much of the note goes to reverb 0).
pub fn note_script_reverb(instrument: u8, semitone: u8, velocity: u8, ticks: u16, hold: u16, bpm: u8, reverb: u8) -> Vec<u8> {
    // `AudioSeq_ScriptReadCompressedU16`'s encoding.
    fn var(v: u16) -> Vec<u8> {
        if v < 0x80 { vec![v as u8] } else { vec![0x80 | (v >> 8) as u8 & 0x7F, v as u8] }
    }
    let (cmd_note, transposition) = if semitone > 0x3F { (0x3F, semitone - 0x3F) } else { (semitone, 0) };
    let hold = hold.min(0x7FFF);
    // Player: DD tempo, DB fade volume, D7 channels set up, 90 channel 0, FD wait, FF end.
    let mut player = vec![0xDD, bpm, 0xDB, 0x7F, 0xD7, 0x00, 0x01, 0x90, 0x00, 0x00];
    player.push(0xFD);
    player.extend(var(hold));
    player.push(0xFF);
    // Channel: C1 instrument, DF volume, DD pan centre, D4 reverb, 88 layer 0, FD wait, FF end.
    let chan_at = player.len();
    player[8] = (chan_at >> 8) as u8;
    player[9] = chan_at as u8;
    let mut chan = vec![0xC1, instrument, 0xDF, 0x7F, 0xDD, 0x40, 0xD4, reverb, 0x88, 0x00, 0x00];
    chan.push(0xFD);
    chan.extend(var(hold));
    chan.push(0xFF);
    let layer_at = chan_at + chan.len();
    chan[9] = (layer_at >> 8) as u8;
    chan[10] = layer_at as u8;
    // Layer: C2 transposition, C1 velocity, C9 gate 0 (held to the end), the note, FF end.
    let mut layer = vec![0xC2, transposition, 0xC1, velocity.min(127), 0xC9, 0x00, cmd_note];
    layer.extend(var(ticks.min(0x7FFF)));
    layer.push(0xFF);
    let mut out = player;
    out.extend(chan);
    out.extend(layer);
    out
}
