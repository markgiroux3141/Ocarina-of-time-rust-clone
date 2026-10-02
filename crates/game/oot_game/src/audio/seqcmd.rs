//! `sequence.c`: the sequence commands. The game queues 32-bit commands
//! (`Audio_QueueSeqCmd`, top nibble the op, the next the player); `Audio_ProcessSeqCmds`
//! turns them into the library's commands each `Audio_Update`, and keeps per player what the
//! fades, tempo changes and queued sequences need (`gActiveSeqs`, `Audio_UpdateActiveSequences`).

use super::*;

/// `struct_801D9D50`: a channel's volume and frequency fades.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Unk50 {
    /// The volume scale, its target, its step and the frames left.
    pub unk_00: f32,
    pub unk_04: f32,
    pub unk_08: f32,
    pub unk_0c: u16,
    /// The frequency scale, its target, its step and the frames left.
    pub unk_10: f32,
    pub unk_14: f32,
    pub unk_18: f32,
    pub unk_1c: u16,
}

/// `unk_D_8016E750`: a player as the game's side keeps it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ActiveSeq {
    pub vol_cur: f32,
    pub vol_target: f32,
    pub vol_step: f32,
    pub vol_timer: u16,
    pub vol_scales: [u8; 4],
    pub vol_fade_timer: u8,
    pub fade_vol_update: u8,
    /// A tempo command (`0xB`) waiting for its player.
    pub tempo_cmd: u32,
    pub tempo_original: u16,
    pub tempo_cur: f32,
    pub tempo_target: f32,
    pub tempo_step: f32,
    pub tempo_timer: u16,
    /// The setup commands (`0xC`) to run once the player stops.
    pub setup_cmd: [u32; 8],
    pub setup_cmd_timer: u8,
    pub setup_cmd_num: u8,
    pub setup_fade_timer: u8,
    pub channel_data: [Unk50; 16],
    /// Channels with frequency fades, with volume fades.
    pub freq_scale_channel_flags: u16,
    pub vol_channel_flags: u16,
    /// The sequence playing (and its args, `<< 8`), and the one started last.
    pub seq_id: u16,
    pub prev_seq_id: u16,
    /// Channels `0x8` leaves alone.
    pub channel_port_mask: u16,
    pub start_seq_cmd: u32,
    pub is_waiting_for_fonts: u8,
}

/// `_SHIFTL(v, s, w)`.
fn shiftl(v: u32, s: u32, w: u32) -> u32 {
    (v & ((1 << w) - 1)) << s
}

impl GameAudio {
    /// `Audio_StartSequence`: starts `seq_id` on `player_idx` (`0x82`, or `0x85` skipping ahead for
    /// `arg2` 0x7F), fading in over `fade_timer`.
    pub fn audio_start_sequence(&mut self, player_idx: u8, seq_id: u8, arg2: u8, fade_timer: u16) {
        if self.start_seq_disabled == 0 || player_idx == SEQ_PLAYER_SFX {
            let arg2 = arg2 & 0x7F;
            let p = player_idx as usize;
            let upf = self.view.updates_per_frame;
            if arg2 == 0x7F {
                let dur = ((fade_timer >> 3) as i32 * 60 * upf as i32) as u16;
                self.queue_cmd_s32(0x8500_0000 | shiftl(player_idx as u32, 16, 8) | shiftl(seq_id as u32, 8, 8), dur as i32);
            } else {
                self.queue_cmd_s32(0x8200_0000 | shiftl(player_idx as u32, 16, 8) | shiftl(seq_id as u32, 8, 8), (fade_timer as i32 * upf as u16 as i32) / 4);
            }

            self.active[p].seq_id = seq_id as u16 | ((arg2 as u16) << 8);
            self.active[p].prev_seq_id = seq_id as u16 | ((arg2 as u16) << 8);

            if self.active[p].vol_cur != 1.0 {
                let v = self.active[p].vol_cur;
                self.queue_cmd_f32(0x4100_0000 | shiftl(player_idx as u32, 16, 8), v);
            }

            let a = &mut self.active[p];
            a.tempo_timer = 0;
            a.tempo_original = 0;
            a.tempo_cmd = 0;
            for c in a.channel_data.iter_mut() {
                c.unk_00 = 1.0;
                c.unk_0c = 0;
                c.unk_10 = 1.0;
                c.unk_1c = 0;
            }
            a.freq_scale_channel_flags = 0;
            a.vol_channel_flags = 0;
        }
    }

    /// `Audio_StopSequence`: stops `player_idx`, fading out over `arg1`.
    pub fn audio_stop_sequence(&mut self, player_idx: u8, arg1: u16) {
        let upf = self.view.updates_per_frame as u16 as i32;
        self.queue_cmd_s32(0x8300_0000 | ((player_idx as u32) << 16), (arg1 as i32 * upf) / 4);
        self.active[player_idx as usize].seq_id = NA_BGM_DISABLED;
    }

    /// `Audio_ProcessSeqCmd`.
    pub fn process_seq_cmd(&mut self, cmd: u32) {
        // (gAudioDebugPrintSeqCmd: AudioDebug_ScrPrt prints the command on the debug screen.)
        let op = cmd >> 28;
        let player_idx = ((cmd & 0xF00_0000) >> 24) as u8;
        let p = player_idx as usize;

        match op {
            0x0 => {
                // play sequence immediately
                let seq_id = (cmd & 0xFF) as u8;
                let seq_args = ((cmd & 0xFF00) >> 8) as u8;
                let fade_timer = ((cmd & 0xFF_0000) >> 13) as u16;
                if self.active[p].is_waiting_for_fonts == 0 && seq_args < 0x80 {
                    self.audio_start_sequence(player_idx, seq_id, seq_args, fade_timer);
                }
            }
            0x1 => {
                // disable seq player
                let fade_timer = ((cmd & 0xFF_0000) >> 13) as u16;
                self.audio_stop_sequence(player_idx, fade_timer);
            }
            0x2 => {
                // queue sequence
                let seq_id = (cmd & 0xFF) as u8;
                let seq_args = ((cmd & 0xFF00) >> 8) as u8;
                let fade_timer = ((cmd & 0xFF_0000) >> 13) as u16;
                let n = self.num_seq_requests[p];
                for i in 0..n {
                    if self.seq_requests[p][i as usize].0 == seq_id {
                        if i == 0 {
                            self.audio_start_sequence(player_idx, seq_id, seq_args, fade_timer);
                        }
                        return;
                    }
                }
                let mut found = n;
                for i in 0..n {
                    if self.seq_requests[p][i as usize].1 <= seq_args {
                        found = i;
                        break;
                    }
                }
                if self.num_seq_requests[p] < 5 {
                    self.num_seq_requests[p] += 1;
                }
                // @bug (game): with the queue full and the new sequence the least important
                // (`found` 5), the C shifts from index -1 down and writes the new entry past the
                // row, into the next player's queue. Here the shift stops at 0 and the entry is
                // dropped.
                let mut i = self.num_seq_requests[p].wrapping_sub(1);
                while i != found && i != 0 {
                    self.seq_requests[p][i as usize] = self.seq_requests[p][i as usize - 1];
                    i = i.wrapping_sub(1);
                }
                if (found as usize) < 5 {
                    self.seq_requests[p][found as usize] = (seq_id, seq_args);
                }
                if found == 0 {
                    self.audio_start_sequence(player_idx, seq_id, seq_args, fade_timer);
                }
            }
            0x3 => {
                // unqueue/stop sequence
                let seq_id = (cmd & 0xFF) as u8;
                let fade_timer = ((cmd & 0xFF_0000) >> 13) as u16;
                let n = self.num_seq_requests[p];
                let mut found = n;
                for i in 0..n {
                    if self.seq_requests[p][i as usize].0 == seq_id {
                        found = i;
                        break;
                    }
                }
                if found != n {
                    for i in found..n - 1 {
                        self.seq_requests[p][i as usize] = self.seq_requests[p][i as usize + 1];
                    }
                    self.num_seq_requests[p] -= 1;
                }
                if found == 0 {
                    self.audio_stop_sequence(player_idx, fade_timer);
                    if self.num_seq_requests[p] != 0 {
                        let (id, args) = self.seq_requests[p][0];
                        self.audio_start_sequence(player_idx, id, args, fade_timer);
                    }
                }
            }
            0x4 => {
                // transition seq volume
                let mut duration = ((cmd & 0xFF_0000) >> 15) as u8;
                let val = (cmd & 0xFF) as u16;
                if duration == 0 {
                    duration += 1;
                }
                let a = &mut self.active[p];
                a.vol_target = val as f32 / 127.0;
                if a.vol_cur != a.vol_target {
                    a.vol_step = (a.vol_cur - a.vol_target) / duration as f32;
                    a.vol_timer = duration as u16;
                }
            }
            0x5 => {
                // transition freq scale for all channels
                let mut duration = ((cmd & 0xFF_0000) >> 15) as u8;
                let val = (cmd & 0xFFFF) as u16;
                if duration == 0 {
                    duration += 1;
                }
                let freq_scale = val as f32 / 1000.0;
                let a = &mut self.active[p];
                for c in a.channel_data.iter_mut() {
                    c.unk_14 = freq_scale;
                    c.unk_1c = duration as u16;
                    c.unk_18 = (c.unk_10 - freq_scale) / duration as f32;
                }
                a.freq_scale_channel_flags = 0xFFFF;
            }
            0xD => {
                // transition freq scale
                let mut duration = ((cmd & 0xFF_0000) >> 15) as u8;
                let chan_idx = ((cmd & 0xF000) >> 12) as usize;
                let val = (cmd & 0xFFF) as u16;
                if duration == 0 {
                    duration += 1;
                }
                let freq_scale = val as f32 / 1000.0;
                let a = &mut self.active[p];
                a.channel_data[chan_idx].unk_14 = freq_scale;
                a.channel_data[chan_idx].unk_18 = (a.channel_data[chan_idx].unk_10 - freq_scale) / duration as f32;
                a.channel_data[chan_idx].unk_1c = duration as u16;
                a.freq_scale_channel_flags |= 1 << chan_idx;
            }
            0x6 => {
                // transition vol scale
                let mut duration = ((cmd & 0xFF_0000) >> 15) as u8;
                let chan_idx = ((cmd & 0xF00) >> 8) as usize;
                let val = (cmd & 0xFF) as u16;
                if duration == 0 {
                    duration += 1;
                }
                let a = &mut self.active[p];
                a.channel_data[chan_idx].unk_04 = val as f32 / 127.0;
                if a.channel_data[chan_idx].unk_00 != a.channel_data[chan_idx].unk_04 {
                    a.channel_data[chan_idx].unk_08 = (a.channel_data[chan_idx].unk_00 - a.channel_data[chan_idx].unk_04) / duration as f32;
                    a.channel_data[chan_idx].unk_0c = duration as u16;
                    a.vol_channel_flags |= 1 << chan_idx;
                }
            }
            0x7 => {
                // set global io port
                let port = (cmd & 0xFF_0000) >> 16;
                let val = (cmd & 0xFF) as u16;
                self.queue_cmd_s8(0x4600_0000 | shiftl(player_idx as u32, 16, 8) | shiftl(port, 0, 8), val as i8);
            }
            0x8 => {
                // set io port if channel masked
                let chan_idx = (cmd & 0xF00) >> 8;
                let port = (cmd & 0xFF_0000) >> 16;
                let val = (cmd & 0xFF) as u16;
                if self.active[p].channel_port_mask & (1 << chan_idx) == 0 {
                    self.queue_cmd_s8(0x0600_0000 | shiftl(player_idx as u32, 16, 8) | shiftl(chan_idx, 8, 8) | shiftl(port, 0, 8), val as i8);
                }
            }
            0x9 => {
                // set channel mask for command 0x8
                self.active[p].channel_port_mask = (cmd & 0xFFFF) as u16;
            }
            0xA => {
                // set channel stop mask
                let channel_mask = (cmd & 0xFFFF) as u16;
                if channel_mask != 0 {
                    // with channel mask channelMask...
                    self.queue_cmd_u16(0x9000_0000 | shiftl(player_idx as u32, 16, 8), channel_mask);
                    // stop channels
                    self.queue_cmd_s8(0x0800_0000 | shiftl(player_idx as u32, 16, 8) | 0xFF00, 1);
                }
                let reversed = channel_mask as u32 ^ 0xFFFF;
                if reversed != 0 {
                    // with channel mask ~channelMask...
                    self.queue_cmd_u16(0x9000_0000 | shiftl(player_idx as u32, 16, 8), reversed as u16);
                    // unstop channels
                    self.queue_cmd_s8(0x0800_0000 | shiftl(player_idx as u32, 16, 8) | 0xFF00, 0);
                }
            }
            0xB => {
                // update tempo
                self.active[p].tempo_cmd = cmd;
            }
            0xC => {
                // start sequence with setup commands
                let sub_op = (cmd & 0xF0_0000) >> 20;
                if sub_op != 0xF {
                    if self.active[p].setup_cmd_num < 7 {
                        let found = self.active[p].setup_cmd_num;
                        self.active[p].setup_cmd_num += 1;
                        if found < 8 {
                            self.active[p].setup_cmd[found as usize] = cmd;
                            self.active[p].setup_cmd_timer = 2;
                        }
                    }
                } else {
                    self.active[p].setup_cmd_num = 0;
                }
            }
            0xE => {
                let sub_op = (cmd & 0xF00) >> 8;
                let val = (cmd & 0xFF) as u8;
                match sub_op {
                    0 => {
                        // set sound mode
                        let m = self.tables.sound_mode_list.get(val as usize).copied().unwrap_or(0);
                        self.queue_cmd_s32(0xF000_0000, m as i32);
                    }
                    1 => {
                        // set sequence starting disabled?
                        self.start_seq_disabled = val & 1;
                    }
                    _ => {}
                }
            }
            0xF => {
                // change spec
                let spec = (cmd & 0xFF) as u8;
                self.sfx_channel_layout = ((cmd & 0xFF00) >> 8) as u8;
                let old_spec = self.audio_spec_id;
                self.audio_spec_id = spec;
                self.audio_thread_reset_audio_heap(spec);
                self.func_800f71bc(old_spec as i32);
                self.queue_cmd_s32(0xF800_0000, 0);
            }
            _ => {}
        }
    }

    /// `Audio_QueueSeqCmd`.
    pub fn queue_seq_cmd(&mut self, cmd: u32) {
        if let Some(l) = &mut self.log {
            l.seq_cmds.push((self.frames, cmd));
        }
        self.seq_cmds[self.seq_cmd_wr_pos as usize] = cmd;
        self.seq_cmd_wr_pos = self.seq_cmd_wr_pos.wrapping_add(1);
    }

    /// `Audio_ProcessSeqCmds`.
    pub fn process_seq_cmds(&mut self) {
        while self.seq_cmd_wr_pos != self.seq_cmd_rd_pos {
            let cmd = self.seq_cmds[self.seq_cmd_rd_pos as usize];
            self.seq_cmd_rd_pos = self.seq_cmd_rd_pos.wrapping_add(1);
            self.process_seq_cmd(cmd);
        }
    }

    /// `Audio_GetActiveSeqId`: the sequence `player_idx` plays (with its args), `NA_BGM_DISABLED`
    /// if it's off.
    pub fn audio_get_active_seq_id(&self, player_idx: u8) -> u16 {
        if !self.view.players[player_idx as usize & 3].enabled {
            return NA_BGM_DISABLED;
        }
        self.active[player_idx as usize].seq_id
    }

    /// `Audio_IsSeqCmdNotQueued`: no command waiting to be processed matches `arg0` under `arg1`.
    pub fn audio_is_seq_cmd_not_queued(&self, arg0: u32, arg1: u32) -> bool {
        let mut i = self.seq_cmd_rd_pos;
        while i != self.seq_cmd_wr_pos {
            if arg0 == self.seq_cmds[i as usize] & arg1 {
                return false;
            }
            i = i.wrapping_add(1);
        }
        true
    }

    /// `Audio_ResetSequenceRequests`: empties `player_idx`'s queue.
    pub fn audio_reset_sequence_requests(&mut self, player_idx: u8) {
        self.num_seq_requests[player_idx as usize] = 0;
    }

    /// `Audio_ReplaceSeqCmdSetupOpVolRestore`: drops `player_idx`'s setup commands of kind `arg1`.
    pub fn audio_replace_seq_cmd_setup_op_vol_restore(&mut self, player_idx: u8, arg1: u8) {
        let a = &mut self.active[player_idx as usize];
        for i in 0..a.setup_cmd_num as usize {
            let unkb = ((a.setup_cmd[i] & 0xF0_0000) >> 20) as u8;
            if unkb == arg1 {
                a.setup_cmd[i] = 0xFF00_0000;
            }
        }
    }

    /// `Audio_SetVolumeScale`: scale `scale_idx` of `player_idx`'s volume to `target_vol`, now or
    /// over `vol_fade_timer` frames.
    pub fn set_vol_scale(&mut self, player_idx: u8, scale_idx: u8, target_vol: u8, vol_fade_timer: u8) {
        let p = player_idx as usize;
        self.active[p].vol_scales[scale_idx as usize] = target_vol & 0x7F;
        if vol_fade_timer != 0 {
            self.active[p].fade_vol_update = 1;
            self.active[p].vol_fade_timer = vol_fade_timer;
        } else {
            let mut vol_scale = 1.0f32;
            for i in 0..4 {
                vol_scale *= self.active[p].vol_scales[i] as f32 / 127.0;
            }
            // SEQCMD_SET_SEQPLAYER_VOLUME_NOW.
            self.process_seq_cmd(0x4000_0000 | ((player_idx as u32) << 24) | ((vol_fade_timer as u32) << 16) | ((vol_scale * 127.0) as u8 as u32));
        }
    }

    /// `Audio_UpdateActiveSequences`: each player's fades, tempo changes and setup commands, a frame on.
    pub fn audio_update_active_sequences(&mut self) {
        for player_idx in 0..4u8 {
            let p = player_idx as usize;
            if self.active[p].is_waiting_for_fonts != 0 {
                let mut dummy = 0;
                if let 1..=4 = self.func_800e5e20(&mut dummy) {
                    self.active[p].is_waiting_for_fonts = 0;
                    let c = self.active[p].start_seq_cmd;
                    self.process_seq_cmd(c);
                }
            }

            if self.active[p].fade_vol_update != 0 {
                let mut phi_f0 = 1.0f32;
                for j in 0..4 {
                    phi_f0 *= self.active[p].vol_scales[j] as f32 / 127.0;
                }
                // SEQCMD_SET_SEQPLAYER_VOLUME.
                let t = self.active[p].vol_fade_timer as u32;
                self.queue_seq_cmd(0x4000_0000 | ((player_idx as u32) << 24) | (t << 16) | ((phi_f0 * 127.0) as u8 as u32));
                self.active[p].fade_vol_update = 0;
            }

            if self.active[p].vol_timer != 0 {
                let a = &mut self.active[p];
                a.vol_timer -= 1;
                if a.vol_timer != 0 {
                    a.vol_cur -= a.vol_step;
                } else {
                    a.vol_cur = a.vol_target;
                }
                let v = a.vol_cur;
                self.queue_cmd_f32(0x4100_0000 | shiftl(player_idx as u32, 16, 8), v);
            }

            if self.active[p].tempo_cmd != 0 {
                let temp_a1 = self.active[p].tempo_cmd;
                let mut phi_t0 = ((temp_a1 & 0xFF_0000) >> 15) as u8;
                let mut phi_a2 = (temp_a1 & 0xFFF) as u16;
                if phi_t0 == 0 {
                    phi_t0 += 1;
                }
                let pv = self.view.players[p];
                if pv.enabled {
                    let temp_lo = pv.tempo / 0x30;
                    let temp_v0_4 = ((temp_a1 & 0xF000) >> 12) as u8;
                    match temp_v0_4 {
                        1 => phi_a2 = phi_a2.wrapping_add(temp_lo),
                        2 => {
                            if phi_a2 < temp_lo {
                                phi_a2 = temp_lo - phi_a2;
                            }
                        }
                        3 => phi_a2 = (temp_lo as f32 * (phi_a2 as f32 / 100.0)) as u16,
                        4 => {
                            phi_a2 = if self.active[p].tempo_original != 0 { self.active[p].tempo_original } else { temp_lo };
                        }
                        _ => {}
                    }
                    if phi_a2 > 300 {
                        phi_a2 = 300;
                    }
                    let a = &mut self.active[p];
                    if a.tempo_original == 0 {
                        a.tempo_original = temp_lo;
                    }
                    a.tempo_target = phi_a2 as f32;
                    a.tempo_cur = (pv.tempo / 0x30) as f32;
                    a.tempo_step = (a.tempo_cur - a.tempo_target) / phi_t0 as f32;
                    a.tempo_timer = phi_t0 as u16;
                    a.tempo_cmd = 0;
                }
            }

            if self.active[p].tempo_timer != 0 {
                let a = &mut self.active[p];
                a.tempo_timer -= 1;
                if a.tempo_timer != 0 {
                    a.tempo_cur -= a.tempo_step;
                } else {
                    a.tempo_cur = a.tempo_target;
                }
                // set tempo
                let t = a.tempo_cur as i32;
                self.queue_cmd_s32(0x4700_0000 | shiftl(player_idx as u32, 16, 8), t);
            }

            if self.active[p].vol_channel_flags != 0 {
                for k in 0..16u32 {
                    let a = &mut self.active[p];
                    if a.channel_data[k as usize].unk_0c != 0 {
                        let c = &mut a.channel_data[k as usize];
                        c.unk_0c -= 1;
                        if c.unk_0c != 0 {
                            c.unk_00 -= c.unk_08;
                        } else {
                            c.unk_00 = c.unk_04;
                            a.vol_channel_flags ^= 1 << k;
                        }
                        // AUDIOCMD_OP_CHANNEL_SET_VOL_SCALE (playerIdx = seq, k = chan)
                        let v = a.channel_data[k as usize].unk_00;
                        self.queue_cmd_f32(0x0100_0000 | shiftl(player_idx as u32, 16, 8) | shiftl(k, 8, 8), v);
                    }
                }
            }

            if self.active[p].freq_scale_channel_flags != 0 {
                for k in 0..16u32 {
                    let a = &mut self.active[p];
                    if a.channel_data[k as usize].unk_1c != 0 {
                        let c = &mut a.channel_data[k as usize];
                        c.unk_1c -= 1;
                        if c.unk_1c != 0 {
                            c.unk_10 -= c.unk_18;
                        } else {
                            c.unk_10 = c.unk_14;
                            a.freq_scale_channel_flags ^= 1 << k;
                        }
                        // AUDIOCMD_OP_CHANNEL_SET_FREQ_SCALE
                        let v = a.channel_data[k as usize].unk_10;
                        self.queue_cmd_f32(0x0400_0000 | shiftl(player_idx as u32, 16, 8) | shiftl(k, 8, 8), v);
                    }
                }
            }

            if self.active[p].setup_cmd_num != 0 {
                if !self.audio_is_seq_cmd_not_queued(0xF000_0000, 0xF000_0000) {
                    self.active[p].setup_cmd_num = 0;
                    return;
                }
                if self.active[p].setup_cmd_timer != 0 {
                    self.active[p].setup_cmd_timer -= 1;
                    continue;
                }
                if self.view.players[p].enabled {
                    continue;
                }
                for j in 0..self.active[p].setup_cmd_num as usize {
                    let c = self.active[p].setup_cmd[j];
                    let temp_a0 = ((c & 0x00F0_0000) >> 20) as u8;
                    let temp_s1 = ((c & 0x000F_0000) >> 16) as u8;
                    let temp_s0_3 = ((c & 0xFF00) >> 8) as u8;
                    let temp_a3_3 = (c & 0xFF) as u8;
                    let s1 = temp_s1 as usize;
                    match temp_a0 {
                        0 => self.set_vol_scale(temp_s1, 1, 0x7F, temp_a3_3),
                        7 => {
                            if self.num_seq_requests[p] == temp_a3_3 {
                                self.set_vol_scale(temp_s1, 1, 0x7F, temp_s0_3);
                            }
                        }
                        1 => {
                            // SEQCMD_UNQUEUE_SEQUENCE (the macro without casts: the whole seqId).
                            let s = self.active[p].seq_id as u32;
                            self.queue_seq_cmd(0x3000_0000 | ((player_idx as u32) << 24) | s);
                        }
                        2 => {
                            // SEQCMD_PLAY_SEQUENCE.
                            let s = self.active[s1].seq_id as u32;
                            self.queue_seq_cmd(((temp_s1 as u32) << 24) | (1 << 16) | s);
                            self.active[s1].fade_vol_update = 1;
                            self.active[s1].vol_scales[1] = 0x7F;
                        }
                        3 => {
                            // SEQCMD_SCALE_TEMPO.
                            self.queue_seq_cmd(0xB000_3000 | ((temp_s1 as u32) << 24) | ((temp_s0_3 as u32) << 16) | temp_a3_3 as u32);
                        }
                        4 => {
                            // SEQCMD_RESET_TEMPO.
                            self.queue_seq_cmd(0xB000_4000 | ((temp_s1 as u32) << 24) | ((temp_a3_3 as u32) << 16));
                        }
                        5 => {
                            let temp_v1 = (c & 0xFFFF) as u16;
                            let t = self.active[s1].setup_fade_timer as u32;
                            self.queue_seq_cmd(((temp_s1 as u32) << 24) | (t << 16) | temp_v1 as u32);
                            self.set_vol_scale(temp_s1, 1, 0x7F, 0);
                            self.active[s1].setup_fade_timer = 0;
                        }
                        6 => self.active[p].setup_fade_timer = temp_s0_3,
                        8 => self.set_vol_scale(temp_s1, temp_s0_3, 0x7F, temp_a3_3),
                        14 => {
                            if temp_a3_3 & 1 != 0 {
                                self.queue_cmd_s32(0xE300_0000, SEQUENCE_TABLE);
                            }
                            if temp_a3_3 & 2 != 0 {
                                self.queue_cmd_s32(0xE300_0000, FONT_TABLE);
                            }
                            if temp_a3_3 & 4 != 0 {
                                self.queue_cmd_s32(0xE300_0000, SAMPLE_TABLE);
                            }
                        }
                        9 => {
                            // SEQCMD_SET_CHANNEL_DISABLE_MASK.
                            let temp_v1 = (c & 0xFFFF) as u32;
                            self.queue_seq_cmd(0xA000_0000 | ((temp_s1 as u32) << 24) | temp_v1);
                        }
                        10 => {
                            // SEQCMD_SET_SEQPLAYER_FREQ.
                            let v = ((temp_a3_3 as u32 * 10) & 0xFFFF) as u32;
                            self.queue_seq_cmd(0x5000_0000 | ((temp_s1 as u32) << 24) | ((temp_s0_3 as u32) << 16) | v);
                        }
                        _ => {}
                    }
                }
                self.active[p].setup_cmd_num = 0;
            }
        }
    }

    /// `func_800FAD34`: while a spec change waits (`D_80133418`), the reset's message from the
    /// audio side ends it: the sfx channel layout goes to the sfx player and its sequence
    /// starts again (`func_800F7170`). Returns `D_80133418`.
    pub fn func_800fad34(&mut self) -> u8 {
        if self.d_80133418 != 0 {
            // D_80133418 == 2 waits on the reset in a loop in the C; nothing sets 2.
            if self.func_800e5edc() == 1 {
                self.d_80133418 = 0;
                let l = self.sfx_channel_layout as i8;
                self.queue_cmd_s8(0x4602_0000, l);
                self.func_800f7170();
            }
        }
        self.d_80133418
    }

    /// `Audio_ResetActiveSequences`: the players' queues and the game's side of them, reset.
    pub fn audio_reset_active_sequences(&mut self) {
        for p in 0..4 {
            self.num_seq_requests[p] = 0;
            let a = &mut self.active[p];
            a.seq_id = NA_BGM_DISABLED;
            a.prev_seq_id = NA_BGM_DISABLED;
            a.tempo_timer = 0;
            a.tempo_original = 0;
            a.tempo_cmd = 0;
            a.channel_port_mask = 0;
            a.setup_cmd_num = 0;
            a.setup_fade_timer = 0;
            a.freq_scale_channel_flags = 0;
            a.vol_channel_flags = 0;
            a.vol_scales = [0x7F; 4];
            a.vol_fade_timer = 1;
            a.fade_vol_update = 1;
        }
    }

    /// `Audio_ResetActiveSequencesAndVolume`: as `Audio_ResetActiveSequences`, with the volumes at full.
    pub fn audio_reset_active_sequences_and_volume(&mut self) {
        for a in self.active.iter_mut() {
            a.vol_cur = 1.0;
            a.vol_timer = 0;
            a.fade_vol_update = 0;
            a.vol_scales = [0x7F; 4];
        }
        self.audio_reset_active_sequences();
    }
}
