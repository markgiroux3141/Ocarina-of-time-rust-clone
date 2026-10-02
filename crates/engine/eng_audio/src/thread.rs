//! The audio thread (`audio/internal/thread.c`): `AudioThread_UpdateImpl`, what runs on every VI retrace
//! (`AudioMgr_HandleRetrace`), the command queue the game talks to it through
//! (`AudioThread_QueueCmd*`, `AudioThread_ProcessCmds`), and the commands themselves.
//!
//! Also the audio interface the retrace hands buffers to: `Ai` models its queue and its DAC
//! rate, so `osAiGetLength` is what the hardware would say and the buffer lengths the thread
//! picks follow (`renderer.rs` drives both, offline or for a device).

use std::collections::VecDeque;

use crate::context::*;

/// `AudioThreadCmdOp`.
const AUDIOCMD_OP_CHANNEL_SET_VOL_SCALE: u8 = 1;
const AUDIOCMD_OP_CHANNEL_SET_VOL: u8 = 2;
const AUDIOCMD_OP_CHANNEL_SET_PAN: u8 = 3;
const AUDIOCMD_OP_CHANNEL_SET_FREQ_SCALE: u8 = 4;
const AUDIOCMD_OP_CHANNEL_SET_REVERB_VOLUME: u8 = 5;
const AUDIOCMD_OP_CHANNEL_SET_IO: u8 = 6;
const AUDIOCMD_OP_CHANNEL_SET_PAN_WEIGHT: u8 = 7;
const AUDIOCMD_OP_CHANNEL_SET_MUTE: u8 = 8;
const AUDIOCMD_OP_CHANNEL_SET_MUTE_BEHAVIOR: u8 = 9;
const AUDIOCMD_OP_CHANNEL_SET_VIBRATO_DEPTH: u8 = 10;
const AUDIOCMD_OP_CHANNEL_SET_VIBRATO_RATE: u8 = 11;
const AUDIOCMD_OP_CHANNEL_SET_COMB_FILTER_SIZE: u8 = 12;
const AUDIOCMD_OP_CHANNEL_SET_COMB_FILTER_GAIN: u8 = 13;
const AUDIOCMD_OP_CHANNEL_SET_STEREO: u8 = 14;

/// `SAMPLES_TO_OVERPRODUCE` and `EXTRA_BUFFERED_AI_SAMPLES_TARGET`.
const SAMPLES_TO_OVERPRODUCE: i32 = 0x10;
const EXTRA_BUFFERED_AI_SAMPLES_TARGET: i32 = 0x80;

/// `osGetCount`'s rate: the CPU's count register runs at half its 93.75 MHz.
pub const OS_COUNT_HZ: u32 = 46_875_000;

impl AudioContext {
    /// `AudioThread_UpdateImpl`: one audio frame. `samples_remaining_in_ai` is `osAiGetLength() / 4`.
    /// Returns the buffer `osAiSetNextBuffer` gets this frame (interleaved stereo), if any.
    pub fn audio_thread_update(&mut self, samples_remaining_in_ai: u32) -> Option<Vec<i16>> {
        self.total_task_count = self.total_task_count.wrapping_add(1);
        let spec_unk4 = self.audio_buffer_parameters.spec_unk4.max(1) as i32;
        if self.total_task_count % spec_unk4 != 0 {
            return None;
        }
        self.task_start_queue.clear();
        self.task_start_queue.push_back(self.total_task_count as u32);
        self.rsp_task_index ^= 1;
        self.cur_ai_buf_index = (self.cur_ai_buf_index + 1) % 3;
        let index = ((self.cur_ai_buf_index - 2 + 3) % 3) as usize;

        let mut out = None;
        if self.reset_timer < 16 && self.ai_buf_lengths[index] != 0 {
            let n = self.ai_buf_lengths[index] as usize * 2;
            let mut v = Vec::with_capacity(n);
            for i in 0..n {
                v.push(self.ram.s16(self.ai_buffers[index] + 2 * i as u32));
            }
            out = Some(v);
        }

        self.cur_audio_frame_dma_count = 0;
        self.decrease_sample_dma_ttls();
        self.process_loads(self.reset_status as i32);
        self.process_script_loads();

        if self.reset_status != 0 && !self.reset_step() {
            if self.reset_status == 0 {
                self.audio_reset_queue.clear();
                self.audio_reset_queue.push_back(self.audio_reset_spec_id_to_load as u32);
            }
            return out;
        }

        if self.reset_timer > 16 {
            return out;
        }
        if self.reset_timer != 0 {
            self.reset_timer += 1;
        }

        let index = self.cur_ai_buf_index as usize;
        let abp = self.audio_buffer_parameters;
        let mut len = ((((abp.samples_per_frame_target as i32 - samples_remaining_in_ai as i32) + EXTRA_BUFFERED_AI_SAMPLES_TARGET) & !0xF) + SAMPLES_TO_OVERPRODUCE) as i16;
        if len < abp.min_ai_buffer_length {
            len = abp.min_ai_buffer_length;
        }
        if len > abp.max_ai_buffer_length {
            len = abp.max_ai_buffer_length;
        }
        self.ai_buf_lengths[index] = len;

        if self.reset_status == 0 {
            // msg = 0000RREE R = read pos, E = End Pos
            let mut j = 0;
            while let Some(msg) = self.cmd_proc_queue.pop_front() {
                self.process_cmds(msg);
                j += 1;
            }
            if j == 0 && self.cmd_queue_finished != 0 {
                self.schedule_process_cmds();
            }
        }

        let ai = self.ai_buffers[index];
        self.audio_synth_update(ai, len as i32);

        // Update audioRandom to the next random number
        self.os_count = self.os_count.wrapping_add(OS_COUNT_HZ / self.refresh_rate.max(1) as u32);
        self.audio_random = (self.audio_random.wrapping_add(self.total_task_count as u32)).wrapping_mul(self.os_count);
        let k = (self.total_task_count & 0xFF) as u32;
        self.audio_random = self.audio_random.wrapping_add(self.ram.s16(ai + 2 * k) as i32 as u32);

        // gWaveSamples[8] interprets compiled assembly code as s16 samples as a way to generate
        // sound with noise. Start with the address of AudioThread_Update, and offset it by a random
        // number between 0 - 0xFFF0.
        self.wave8 = self.statics.noise_base.wrapping_add(self.audio_random & 0xFFF0);

        // The task: the RSP runs the command list.
        let cmds = std::mem::take(&mut self.cmds);
        if self.max_abi_cmd_cnt < cmds.len() as i32 {
            self.max_abi_cmd_cnt = cmds.len() as i32;
        }
        self.rsp.run(&cmds, &mut self.ram);
        self.cmds = cmds;
        out
    }

    /// `AudioThread_ProcessGlobalCmd`: the commands to the whole library.
    pub fn audio_thread_process_global_cmd(&mut self, cmd: AudioCmd) {
        match cmd.op {
            0x81 => self.sync_load_seq_parts(cmd.arg1 as i32, cmd.arg2 as i32),
            0x82 => {
                self.sync_init_seq_player(cmd.arg0 as i32, cmd.arg1 as i32, cmd.arg2 as i32);
                self.audio_thread_set_fade_in_timer(cmd.arg0 as i32, cmd.as_int());
            }
            0x85 => {
                self.sync_init_seq_player_skip_ticks(cmd.arg0 as i32, cmd.arg1 as i32, cmd.as_int());
            }
            0x83 => {
                if self.seq_players[cmd.arg0 as usize & 3].enabled {
                    if cmd.as_int() == 0 {
                        self.sequence_player_disable_as_finished(cmd.arg0 as usize & 3);
                    } else {
                        self.audio_thread_set_fade_out_timer(cmd.arg0 as i32, cmd.as_int());
                    }
                }
            }
            0xF0 => self.sound_mode = cmd.data as i8,
            0xF1 => {
                for i in 0..self.audio_buffer_parameters.num_sequence_players as usize {
                    self.seq_players[i].muted = true;
                    self.seq_players[i].recalculate_volume = true;
                }
            }
            0xF2 => {
                if cmd.data == 1 {
                    for i in 0..self.num_notes as usize {
                        let n = self.notes[i];
                        if n.note_sub_eu.enabled && n.playback_state.unk_04 == 0 {
                            if let Some(l) = n.playback_state.parent_layer {
                                let c = self.sequence_layers[l].channel.unwrap_or(CHANNEL_NONE);
                                if self.channels[c].mute_behavior & MUTE_BEHAVIOR_3 != 0 {
                                    self.notes[i].note_sub_eu.finished = true;
                                }
                            }
                        }
                    }
                }
                for i in 0..self.audio_buffer_parameters.num_sequence_players as usize {
                    self.seq_players[i].muted = false;
                    self.seq_players[i].recalculate_volume = true;
                }
            }
            0xF3 => {
                self.sync_load_instrument(cmd.arg0 as i32, cmd.arg1 as i32, cmd.arg2 as i32);
            }
            0xF4 => self.async_load(SAMPLE_TABLE, cmd.arg0 as i32, 0, cmd.arg2 as i32, RetQueue::External),
            0xF5 => self.async_load(FONT_TABLE, cmd.arg0 as i32, 0, cmd.arg2 as i32, RetQueue::External),
            0xFC => self.async_load(SEQUENCE_TABLE, cmd.arg0 as i32, 0, cmd.arg2 as i32, RetQueue::External),
            0xF6 => self.discard_seq_fonts(cmd.arg1 as i32),
            0x90 => self.thread_cmd_channel_mask[cmd.arg0 as usize & 3] = cmd.as_ushort(),
            0xF9 => {
                self.reset_status = 5;
                self.audio_reset_spec_id_to_load = cmd.data as u8;
            }
            0xFB => {
                // gAudioCustomUpdateFunction: a callback the game never sets.
            }
            0xE0..=0xE2 => {
                self.set_font_instrument((cmd.op - 0xE0) as i32, cmd.arg0 as i32, cmd.arg1 as i32, cmd.data);
            }
            0xFE => {
                if cmd.data == 1 {
                    for i in 0..self.audio_buffer_parameters.num_sequence_players as usize {
                        if self.seq_players[i].enabled {
                            self.sequence_player_disable_as_finished(i);
                        }
                    }
                }
                self.func_800e66c0(cmd.data as i32);
            }
            0xE3 => self.pop_persistent_cache(cmd.as_int()),
            _ => {}
        }
    }

    /// `AudioThread_SetFadeOutTimer`.
    pub fn audio_thread_set_fade_out_timer(&mut self, player_idx: i32, mut fade_timer: i32) {
        let sp = &mut self.seq_players[player_idx as usize & 3];
        if fade_timer == 0 {
            fade_timer = 1;
        }
        sp.fade_velocity = -(sp.fade_volume / fade_timer as f32);
        sp.state = 2;
        sp.fade_timer = fade_timer as u16;
    }

    /// `AudioThread_SetFadeInTimer`.
    pub fn audio_thread_set_fade_in_timer(&mut self, player_idx: i32, fade_timer: i32) {
        if fade_timer != 0 {
            let sp = &mut self.seq_players[player_idx as usize & 3];
            sp.state = 1;
            sp.fade_timer_unk_eu = fade_timer as u16;
            sp.fade_timer = fade_timer as u16;
            sp.fade_volume = 0.0;
            sp.fade_velocity = 0.0;
        }
    }

    /// `AudioThread_QueueCmd`.
    pub fn queue_cmd(&mut self, op_args: u32, data: u32) {
        let c = &mut self.cmd_buf[self.cmd_wr_pos as usize];
        c.op = (op_args >> 24) as u8;
        c.arg0 = (op_args >> 16) as u8;
        c.arg1 = (op_args >> 8) as u8;
        c.arg2 = op_args as u8;
        c.data = data;
        self.cmd_wr_pos = self.cmd_wr_pos.wrapping_add(1);
        if self.cmd_wr_pos == self.cmd_rd_pos {
            self.cmd_wr_pos = self.cmd_wr_pos.wrapping_sub(1);
        }
    }
    /// `AudioThread_QueueCmdF32`.
    pub fn queue_cmd_f32(&mut self, op_args: u32, data: f32) {
        self.queue_cmd(op_args, data.to_bits());
    }
    /// `AudioThread_QueueCmdS32`.
    pub fn queue_cmd_s32(&mut self, op_args: u32, data: i32) {
        self.queue_cmd(op_args, data as u32);
    }
    /// `AudioThread_QueueCmdS8`.
    pub fn queue_cmd_s8(&mut self, op_args: u32, data: i8) {
        self.queue_cmd(op_args, ((data as i32) << 0x18) as u32);
    }
    /// `AudioThread_QueueCmdU16`.
    pub fn queue_cmd_u16(&mut self, op_args: u32, data: u16) {
        self.queue_cmd(op_args, (data as u32) << 0x10);
    }

    /// `AudioThread_ScheduleProcessCmds`.
    pub fn schedule_process_cmds(&mut self) -> i32 {
        let pending = (self.cmd_wr_pos.wrapping_sub(self.cmd_rd_pos) as i32 + 0x100) as u8 as i32;
        if self.d_801304e8 < pending {
            self.d_801304e8 = pending;
        }
        if self.cmd_proc_queue.len() >= 4 {
            return -1;
        }
        self.cmd_proc_queue.push_back(((self.cmd_rd_pos as u32) << 8) | self.cmd_wr_pos as u32);
        self.cmd_rd_pos = self.cmd_wr_pos;
        0
    }

    /// `AudioThread_ResetCmdQueue`.
    pub fn reset_cmd_queue(&mut self) {
        self.cmd_queue_finished = 0;
        self.cmd_rd_pos = self.cmd_wr_pos;
    }

    /// `AudioThread_ProcessCmd`.
    pub fn process_cmd(&mut self, cmd: AudioCmd) {
        if cmd.op & 0xF0 == 0xF0 {
            self.audio_thread_process_global_cmd(cmd);
            return;
        }
        if (cmd.arg0 as i16) < self.audio_buffer_parameters.num_sequence_players {
            let p = cmd.arg0 as usize;
            if cmd.op & 0x80 != 0 {
                self.audio_thread_process_global_cmd(cmd);
                return;
            }
            if cmd.op & 0x40 != 0 {
                self.audio_thread_process_seq_player_cmd(p, cmd);
                return;
            }
            if cmd.arg1 < 0x10 {
                let c = self.seq_players[p].channels[cmd.arg1 as usize];
                self.audio_thread_process_channel_cmd(c, cmd);
                return;
            }
            if cmd.arg1 == 0xFF {
                let mut bits = self.thread_cmd_channel_mask[p & 3];
                for i in 0..16 {
                    if bits & 1 != 0 {
                        let c = self.seq_players[p].channels[i];
                        self.audio_thread_process_channel_cmd(c, cmd);
                    }
                    bits >>= 1;
                }
            }
        }
    }

    /// `AudioThread_ProcessCmds`.
    pub fn process_cmds(&mut self, msg: u32) {
        if self.cmd_queue_finished == 0 {
            self.cur_cmd_rd_pos = (msg >> 8) as u8;
        }
        loop {
            let end_pos = msg as u8;
            if self.cur_cmd_rd_pos == end_pos {
                self.cmd_queue_finished = 0;
                return;
            }
            let i = self.cur_cmd_rd_pos as usize;
            self.cur_cmd_rd_pos = self.cur_cmd_rd_pos.wrapping_add(1);
            let cmd = self.cmd_buf[i];
            if cmd.op == 0xF8 {
                self.cmd_queue_finished = 1;
                return;
            }
            self.process_cmd(cmd);
            self.cmd_buf[i].op = 0;
        }
    }

    /// `AudioThread_ProcessSeqPlayerCmd`: a player's commands.
    pub fn audio_thread_process_seq_player_cmd(&mut self, p: usize, cmd: AudioCmd) {
        let sp = &mut self.seq_players[p];
        match cmd.op {
            0x41 => {
                if sp.fade_volume_scale != cmd.as_float() {
                    sp.fade_volume_scale = cmd.as_float();
                    sp.recalculate_volume = true;
                }
            }
            0x47 => sp.tempo = (cmd.as_int() * 0x30) as u16,
            0x49 => sp.tempo_change = (cmd.as_int() * 0x30) as u16,
            0x4E => sp.tempo_change = cmd.as_int() as u16,
            0x48 => sp.transposition = cmd.as_sbyte() as i16,
            0x46 => sp.sound_script_io[cmd.arg2 as usize & 7] = cmd.as_sbyte(),
            0x4A | 0x4B => {
                let fade_volume = if cmd.op == 0x4A { cmd.arg1 as i32 as f32 / 127.0f32 } else { (cmd.arg1 as i32 as f32 / 100.0f32) * sp.fade_volume };
                if sp.state != 2 {
                    sp.volume = sp.fade_volume;
                    if cmd.as_int() == 0 {
                        sp.fade_volume = fade_volume;
                    } else {
                        let fade_timer = cmd.as_int();
                        sp.state = 0;
                        sp.fade_timer = fade_timer as u16;
                        sp.fade_velocity = (fade_volume - sp.fade_volume) / fade_timer as f32;
                    }
                }
            }
            0x4C => {
                if sp.state != 2 {
                    if cmd.as_int() == 0 {
                        sp.fade_volume = sp.volume;
                    } else {
                        let fade_timer = cmd.as_int();
                        sp.state = 0;
                        sp.fade_timer = fade_timer as u16;
                        sp.fade_velocity = (sp.volume - sp.fade_volume) / fade_timer as f32;
                    }
                }
            }
            0x4D => {
                sp.bend = cmd.as_float();
                sp.apply_bend = sp.bend != 1.0;
            }
            _ => {}
        }
    }

    /// `AudioThread_ProcessChannelCmd`: a channel's commands.
    pub fn audio_thread_process_channel_cmd(&mut self, c: ChanId, cmd: AudioCmd) {
        let ch = &mut self.channels[c];
        match cmd.op {
            AUDIOCMD_OP_CHANNEL_SET_VOL_SCALE => {
                if ch.volume_scale != cmd.as_float() {
                    ch.volume_scale = cmd.as_float();
                    ch.changes |= CHANGES_VOLUME;
                }
            }
            AUDIOCMD_OP_CHANNEL_SET_VOL => {
                if ch.volume != cmd.as_float() {
                    ch.volume = cmd.as_float();
                    ch.changes |= CHANGES_VOLUME;
                }
            }
            AUDIOCMD_OP_CHANNEL_SET_PAN => {
                if ch.new_pan as i8 != cmd.as_sbyte() {
                    ch.new_pan = cmd.as_sbyte() as u8;
                    ch.changes |= CHANGES_PAN;
                }
            }
            AUDIOCMD_OP_CHANNEL_SET_PAN_WEIGHT => {
                if ch.new_pan as i8 != cmd.as_sbyte() {
                    ch.pan_channel_weight = cmd.as_sbyte() as u8;
                    ch.changes |= CHANGES_PAN;
                }
            }
            AUDIOCMD_OP_CHANNEL_SET_FREQ_SCALE => {
                if ch.freq_scale != cmd.as_float() {
                    ch.freq_scale = cmd.as_float();
                    ch.changes |= CHANGES_FREQ_SCALE;
                }
            }
            AUDIOCMD_OP_CHANNEL_SET_REVERB_VOLUME => {
                if ch.reverb as i8 != cmd.as_sbyte() {
                    ch.reverb = cmd.as_sbyte() as u8;
                }
            }
            AUDIOCMD_OP_CHANNEL_SET_IO => {
                if cmd.arg2 < 8 {
                    ch.sound_script_io[cmd.arg2 as usize] = cmd.as_sbyte();
                }
            }
            AUDIOCMD_OP_CHANNEL_SET_MUTE => ch.stop_something2 = cmd.as_sbyte() != 0,
            AUDIOCMD_OP_CHANNEL_SET_MUTE_BEHAVIOR => ch.mute_behavior = cmd.as_sbyte() as u8,
            AUDIOCMD_OP_CHANNEL_SET_VIBRATO_DEPTH => {
                ch.vibrato_extent_target = cmd.as_ubyte() as u16 * 8;
                ch.vibrato_extent_change_delay = 1;
            }
            AUDIOCMD_OP_CHANNEL_SET_VIBRATO_RATE => {
                ch.vibrato_rate_target = cmd.as_ubyte() as u16 * 32;
                ch.vibrato_rate_change_delay = 1;
            }
            AUDIOCMD_OP_CHANNEL_SET_COMB_FILTER_SIZE => ch.comb_filter_size = cmd.as_ubyte(),
            AUDIOCMD_OP_CHANNEL_SET_COMB_FILTER_GAIN => ch.comb_filter_gain = cmd.as_ushort(),
            AUDIOCMD_OP_CHANNEL_SET_STEREO => ch.stereo = cmd.as_ubyte(),
            _ => {}
        }
    }

    /// `AudioThread_GetChannelIO`: a channel's IO port, -1 if its player is off.
    pub fn audio_thread_get_channel_io(&self, player_idx: usize, channel_idx: usize, script_idx: usize) -> i8 {
        let sp = &self.seq_players[player_idx];
        if sp.enabled { self.channels[sp.channels[channel_idx]].sound_script_io[script_idx] } else { -1 }
    }

    /// `AudioThread_GetSeqPlayerIO`: a player's IO port.
    pub fn audio_thread_get_seq_player_io(&self, player_idx: usize, port: usize) -> i8 {
        self.seq_players[player_idx].sound_script_io[port]
    }

    /// `func_800E66C0`: counts the sounding notes (and releases them, `arg0 & 1`).
    pub fn func_800e66c0(&mut self, arg0: i32) -> i32 {
        let mut phi_v1 = 0;
        let inv = self.audio_buffer_parameters.updates_per_frame_inv;
        for i in 0..self.num_notes as usize {
            let n = self.notes[i];
            if n.note_sub_eu.enabled && n.playback_state.adsr.state() != 0 {
                if arg0 >= 2 {
                    let ts = n.note_sub_eu.tuned_sample;
                    if ts == 0 || n.note_sub_eu.is_synthetic_wave {
                        continue;
                    }
                    if self.ram.sample(self.ram.u32(ts)).medium == MEDIUM_RAM {
                        continue;
                    }
                }
                phi_v1 += 1;
                if arg0 & 1 == 1 {
                    let a = &mut self.notes[i].playback_state.adsr;
                    a.fade_out_vel = inv;
                    a.set_release(true);
                }
            }
        }
        phi_v1
    }

    /// `AudioThread_NextRandom`.
    pub fn next_random(&mut self) -> u32 {
        self.aud_rand = self.os_count.wrapping_add(0x123_4567).wrapping_mul(self.aud_rand.wrapping_add(self.total_task_count as u32));
        self.aud_rand = self.aud_rand.wrapping_add(self.audio_random);
        self.aud_rand
    }
}

/// The audio interface: the buffers handed to it (`osAiSetNextBuffer`) play one after the other
/// at the DAC's rate (`aiSamplingFrequency`).
#[derive(Debug, Default)]
pub struct Ai {
    /// The buffers queued, as interleaved stereo; the first is playing.
    queue: VecDeque<Vec<i16>>,
    /// Stereo frames of the first buffer already played.
    pos: usize,
    /// Fractional frames owed from the last VI period.
    frac: f64,
}

impl Ai {
    pub fn new() -> Ai {
        Ai::default()
    }

    /// `osAiGetLength() / 4`: the frames left in the buffer playing.
    pub fn remaining_in_current(&self) -> u32 {
        self.queue.front().map(|b| (b.len() / 2).saturating_sub(self.pos) as u32).unwrap_or(0)
    }

    /// `osAiSetNextBuffer`.
    pub fn set_next_buffer(&mut self, buf: Vec<i16>) {
        self.queue.push_back(buf);
    }

    /// Plays `frames` stereo frames (plus the fraction carried), appending them to `out`;
    /// silence when the queue runs dry, as the DAC repeats nothing.
    pub fn play(&mut self, frames: f64, out: &mut Vec<i16>) {
        self.frac += frames;
        let mut n = self.frac.floor() as usize;
        self.frac -= n as f64;
        while n > 0 {
            let Some(b) = self.queue.front() else {
                out.extend(std::iter::repeat_n(0, n * 2));
                return;
            };
            let left = b.len() / 2 - self.pos;
            let take = left.min(n);
            out.extend_from_slice(&b[self.pos * 2..(self.pos + take) * 2]);
            self.pos += take;
            n -= take;
            if self.pos == b.len() / 2 {
                self.queue.pop_front();
                self.pos = 0;
            }
        }
    }
}
