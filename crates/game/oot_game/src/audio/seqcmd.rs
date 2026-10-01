//! `code_800F9280.c`: the sequence commands. The game queues 32-bit commands
//! (`Audio_QueueSeqCmd`, top nibble the op, the next the player); `Audio_ProcessSeqCmds`
//! turns them into the library's commands each `Audio_Update`, and keeps per player what the
//! fades, tempo changes and queued sequences need (`D_8016E750`, `func_800FA3DC`).

use super::*;

/// `unk_50_s`: a channel's volume and frequency fades.
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
    pub unk_08: f32,
    pub unk_0c: u16,
    pub vol_scales: [u8; 4],
    pub vol_fade_timer: u8,
    pub fade_vol_update: u8,
    /// A tempo command (`0xB`) waiting for its player.
    pub unk_14: u32,
    pub unk_18: u16,
    pub unk_1c: f32,
    pub unk_20: f32,
    pub unk_24: f32,
    pub unk_28: u16,
    /// The setup commands (`0xC`) to run once the player stops.
    pub unk_2c: [u32; 8],
    pub unk_4c: u8,
    pub unk_4d: u8,
    pub unk_4e: u8,
    pub unk_50: [Unk50; 16],
    /// Channels with frequency fades, with volume fades.
    pub unk_250: u16,
    pub unk_252: u16,
    /// The sequence playing (and its args, `<< 8`), and the one started last.
    pub unk_254: u16,
    pub unk_256: u16,
    /// Channels `0x8` leaves alone.
    pub unk_258: u16,
    pub unk_25c: u32,
    pub unk_260: u8,
}

/// `_SHIFTL(v, s, w)`.
fn shiftl(v: u32, s: u32, w: u32) -> u32 {
    (v & ((1 << w) - 1)) << s
}

impl GameAudio {
    /// `func_800F9280`: starts `seq_id` on `player_idx` (`0x82`, or `0x85` skipping ahead for
    /// `arg2` 0x7F), fading in over `fade_timer`.
    pub fn func_800f9280(&mut self, player_idx: u8, seq_id: u8, arg2: u8, fade_timer: u16) {
        if self.d_80133408 == 0 || player_idx == SEQ_PLAYER_SFX {
            let arg2 = arg2 & 0x7F;
            let p = player_idx as usize;
            let upf = self.view.updates_per_frame;
            if arg2 == 0x7F {
                let dur = ((fade_timer >> 3) as i32 * 60 * upf as i32) as u16;
                self.queue_cmd_s32(0x8500_0000 | shiftl(player_idx as u32, 16, 8) | shiftl(seq_id as u32, 8, 8), dur as i32);
            } else {
                self.queue_cmd_s32(0x8200_0000 | shiftl(player_idx as u32, 16, 8) | shiftl(seq_id as u32, 8, 8), (fade_timer as i32 * upf as u16 as i32) / 4);
            }

            self.active[p].unk_254 = seq_id as u16 | ((arg2 as u16) << 8);
            self.active[p].unk_256 = seq_id as u16 | ((arg2 as u16) << 8);

            if self.active[p].vol_cur != 1.0 {
                let v = self.active[p].vol_cur;
                self.queue_cmd_f32(0x4100_0000 | shiftl(player_idx as u32, 16, 8), v);
            }

            let a = &mut self.active[p];
            a.unk_28 = 0;
            a.unk_18 = 0;
            a.unk_14 = 0;
            for c in a.unk_50.iter_mut() {
                c.unk_00 = 1.0;
                c.unk_0c = 0;
                c.unk_10 = 1.0;
                c.unk_1c = 0;
            }
            a.unk_250 = 0;
            a.unk_252 = 0;
        }
    }

    /// `func_800F9474`: stops `player_idx`, fading out over `arg1`.
    pub fn func_800f9474(&mut self, player_idx: u8, arg1: u16) {
        let upf = self.view.updates_per_frame as u16 as i32;
        self.queue_cmd_s32(0x8300_0000 | ((player_idx as u32) << 16), (arg1 as i32 * upf) / 4);
        self.active[player_idx as usize].unk_254 = NA_BGM_DISABLED;
    }

    /// `Audio_ProcessSeqCmd`.
    pub fn process_seq_cmd(&mut self, cmd: u32) {
        // (D_8013340C: AudioDebug_ScrPrt prints the command on the debug screen.)
        let op = cmd >> 28;
        let player_idx = ((cmd & 0xF00_0000) >> 24) as u8;
        let p = player_idx as usize;

        match op {
            0x0 => {
                // play sequence immediately
                let seq_id = (cmd & 0xFF) as u8;
                let seq_args = ((cmd & 0xFF00) >> 8) as u8;
                let fade_timer = ((cmd & 0xFF_0000) >> 13) as u16;
                if self.active[p].unk_260 == 0 && seq_args < 0x80 {
                    self.func_800f9280(player_idx, seq_id, seq_args, fade_timer);
                }
            }
            0x1 => {
                // disable seq player
                let fade_timer = ((cmd & 0xFF_0000) >> 13) as u16;
                self.func_800f9474(player_idx, fade_timer);
            }
            0x2 => {
                // queue sequence
                let seq_id = (cmd & 0xFF) as u8;
                let seq_args = ((cmd & 0xFF00) >> 8) as u8;
                let fade_timer = ((cmd & 0xFF_0000) >> 13) as u16;
                let n = self.d_8016e348[p];
                for i in 0..n {
                    if self.d_8016e320[p][i as usize].0 == seq_id {
                        if i == 0 {
                            self.func_800f9280(player_idx, seq_id, seq_args, fade_timer);
                        }
                        return;
                    }
                }
                let mut found = n;
                for i in 0..n {
                    if self.d_8016e320[p][i as usize].1 <= seq_args {
                        found = i;
                        break;
                    }
                }
                if self.d_8016e348[p] < 5 {
                    self.d_8016e348[p] += 1;
                }
                // @bug (game): with the queue full and the new sequence the least important
                // (`found` 5), the C shifts from index -1 down and writes the new entry past the
                // row, into the next player's queue. Here the shift stops at 0 and the entry is
                // dropped.
                let mut i = self.d_8016e348[p].wrapping_sub(1);
                while i != found && i != 0 {
                    self.d_8016e320[p][i as usize] = self.d_8016e320[p][i as usize - 1];
                    i = i.wrapping_sub(1);
                }
                if (found as usize) < 5 {
                    self.d_8016e320[p][found as usize] = (seq_id, seq_args);
                }
                if found == 0 {
                    self.func_800f9280(player_idx, seq_id, seq_args, fade_timer);
                }
            }
            0x3 => {
                // unqueue/stop sequence
                let seq_id = (cmd & 0xFF) as u8;
                let fade_timer = ((cmd & 0xFF_0000) >> 13) as u16;
                let n = self.d_8016e348[p];
                let mut found = n;
                for i in 0..n {
                    if self.d_8016e320[p][i as usize].0 == seq_id {
                        found = i;
                        break;
                    }
                }
                if found != n {
                    for i in found..n - 1 {
                        self.d_8016e320[p][i as usize] = self.d_8016e320[p][i as usize + 1];
                    }
                    self.d_8016e348[p] -= 1;
                }
                if found == 0 {
                    self.func_800f9474(player_idx, fade_timer);
                    if self.d_8016e348[p] != 0 {
                        let (id, args) = self.d_8016e320[p][0];
                        self.func_800f9280(player_idx, id, args, fade_timer);
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
                    a.unk_08 = (a.vol_cur - a.vol_target) / duration as f32;
                    a.unk_0c = duration as u16;
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
                for c in a.unk_50.iter_mut() {
                    c.unk_14 = freq_scale;
                    c.unk_1c = duration as u16;
                    c.unk_18 = (c.unk_10 - freq_scale) / duration as f32;
                }
                a.unk_250 = 0xFFFF;
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
                a.unk_50[chan_idx].unk_14 = freq_scale;
                a.unk_50[chan_idx].unk_18 = (a.unk_50[chan_idx].unk_10 - freq_scale) / duration as f32;
                a.unk_50[chan_idx].unk_1c = duration as u16;
                a.unk_250 |= 1 << chan_idx;
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
                a.unk_50[chan_idx].unk_04 = val as f32 / 127.0;
                if a.unk_50[chan_idx].unk_00 != a.unk_50[chan_idx].unk_04 {
                    a.unk_50[chan_idx].unk_08 = (a.unk_50[chan_idx].unk_00 - a.unk_50[chan_idx].unk_04) / duration as f32;
                    a.unk_50[chan_idx].unk_0c = duration as u16;
                    a.unk_252 |= 1 << chan_idx;
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
                if self.active[p].unk_258 & (1 << chan_idx) == 0 {
                    self.queue_cmd_s8(0x0600_0000 | shiftl(player_idx as u32, 16, 8) | shiftl(chan_idx, 8, 8) | shiftl(port, 0, 8), val as i8);
                }
            }
            0x9 => {
                // set channel mask for command 0x8
                self.active[p].unk_258 = (cmd & 0xFFFF) as u16;
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
                self.active[p].unk_14 = cmd;
            }
            0xC => {
                // start sequence with setup commands
                let sub_op = (cmd & 0xF0_0000) >> 20;
                if sub_op != 0xF {
                    if self.active[p].unk_4d < 7 {
                        let found = self.active[p].unk_4d;
                        self.active[p].unk_4d += 1;
                        if found < 8 {
                            self.active[p].unk_2c[found as usize] = cmd;
                            self.active[p].unk_4c = 2;
                        }
                    }
                } else {
                    self.active[p].unk_4d = 0;
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
                        self.d_80133408 = val & 1;
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
                self.func_800e5f88(spec);
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

    /// `func_800FA0B4`: the sequence `player_idx` plays (with its args), `NA_BGM_DISABLED`
    /// if it's off.
    pub fn func_800fa0b4(&self, player_idx: u8) -> u16 {
        if !self.view.players[player_idx as usize & 3].enabled {
            return NA_BGM_DISABLED;
        }
        self.active[player_idx as usize].unk_254
    }

    /// `func_800FA11C`: no command waiting to be processed matches `arg0` under `arg1`.
    pub fn func_800fa11c(&self, arg0: u32, arg1: u32) -> bool {
        let mut i = self.seq_cmd_rd_pos;
        while i != self.seq_cmd_wr_pos {
            if arg0 == self.seq_cmds[i as usize] & arg1 {
                return false;
            }
            i = i.wrapping_add(1);
        }
        true
    }

    /// `func_800FA174`: empties `player_idx`'s queue.
    pub fn func_800fa174(&mut self, player_idx: u8) {
        self.d_8016e348[player_idx as usize] = 0;
    }

    /// `func_800FA18C`: drops `player_idx`'s setup commands of kind `arg1`.
    pub fn func_800fa18c(&mut self, player_idx: u8, arg1: u8) {
        let a = &mut self.active[player_idx as usize];
        for i in 0..a.unk_4d as usize {
            let unkb = ((a.unk_2c[i] & 0xF0_0000) >> 20) as u8;
            if unkb == arg1 {
                a.unk_2c[i] = 0xFF00_0000;
            }
        }
    }

    /// `Audio_SetVolScale`: scale `scale_idx` of `player_idx`'s volume to `target_vol`, now or
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
            // Audio_SetVolScaleNow.
            self.process_seq_cmd(0x4000_0000 | ((player_idx as u32) << 24) | ((vol_fade_timer as u32) << 16) | ((vol_scale * 127.0) as u8 as u32));
        }
    }

    /// `func_800FA3DC`: each player's fades, tempo changes and setup commands, a frame on.
    pub fn func_800fa3dc(&mut self) {
        for player_idx in 0..4u8 {
            let p = player_idx as usize;
            if self.active[p].unk_260 != 0 {
                let mut dummy = 0;
                if let 1..=4 = self.func_800e5e20(&mut dummy) {
                    self.active[p].unk_260 = 0;
                    let c = self.active[p].unk_25c;
                    self.process_seq_cmd(c);
                }
            }

            if self.active[p].fade_vol_update != 0 {
                let mut phi_f0 = 1.0f32;
                for j in 0..4 {
                    phi_f0 *= self.active[p].vol_scales[j] as f32 / 127.0;
                }
                // Audio_SeqCmd4.
                let t = self.active[p].vol_fade_timer as u32;
                self.queue_seq_cmd(0x4000_0000 | ((player_idx as u32) << 24) | (t << 16) | ((phi_f0 * 127.0) as u8 as u32));
                self.active[p].fade_vol_update = 0;
            }

            if self.active[p].unk_0c != 0 {
                let a = &mut self.active[p];
                a.unk_0c -= 1;
                if a.unk_0c != 0 {
                    a.vol_cur -= a.unk_08;
                } else {
                    a.vol_cur = a.vol_target;
                }
                let v = a.vol_cur;
                self.queue_cmd_f32(0x4100_0000 | shiftl(player_idx as u32, 16, 8), v);
            }

            if self.active[p].unk_14 != 0 {
                let temp_a1 = self.active[p].unk_14;
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
                            phi_a2 = if self.active[p].unk_18 != 0 { self.active[p].unk_18 } else { temp_lo };
                        }
                        _ => {}
                    }
                    if phi_a2 > 300 {
                        phi_a2 = 300;
                    }
                    let a = &mut self.active[p];
                    if a.unk_18 == 0 {
                        a.unk_18 = temp_lo;
                    }
                    a.unk_20 = phi_a2 as f32;
                    a.unk_1c = (pv.tempo / 0x30) as f32;
                    a.unk_24 = (a.unk_1c - a.unk_20) / phi_t0 as f32;
                    a.unk_28 = phi_t0 as u16;
                    a.unk_14 = 0;
                }
            }

            if self.active[p].unk_28 != 0 {
                let a = &mut self.active[p];
                a.unk_28 -= 1;
                if a.unk_28 != 0 {
                    a.unk_1c -= a.unk_24;
                } else {
                    a.unk_1c = a.unk_20;
                }
                // set tempo
                let t = a.unk_1c as i32;
                self.queue_cmd_s32(0x4700_0000 | shiftl(player_idx as u32, 16, 8), t);
            }

            if self.active[p].unk_252 != 0 {
                for k in 0..16u32 {
                    let a = &mut self.active[p];
                    if a.unk_50[k as usize].unk_0c != 0 {
                        let c = &mut a.unk_50[k as usize];
                        c.unk_0c -= 1;
                        if c.unk_0c != 0 {
                            c.unk_00 -= c.unk_08;
                        } else {
                            c.unk_00 = c.unk_04;
                            a.unk_252 ^= 1 << k;
                        }
                        // CHAN_UPD_VOL_SCALE (playerIdx = seq, k = chan)
                        let v = a.unk_50[k as usize].unk_00;
                        self.queue_cmd_f32(0x0100_0000 | shiftl(player_idx as u32, 16, 8) | shiftl(k, 8, 8), v);
                    }
                }
            }

            if self.active[p].unk_250 != 0 {
                for k in 0..16u32 {
                    let a = &mut self.active[p];
                    if a.unk_50[k as usize].unk_1c != 0 {
                        let c = &mut a.unk_50[k as usize];
                        c.unk_1c -= 1;
                        if c.unk_1c != 0 {
                            c.unk_10 -= c.unk_18;
                        } else {
                            c.unk_10 = c.unk_14;
                            a.unk_250 ^= 1 << k;
                        }
                        // CHAN_UPD_FREQ_SCALE
                        let v = a.unk_50[k as usize].unk_10;
                        self.queue_cmd_f32(0x0400_0000 | shiftl(player_idx as u32, 16, 8) | shiftl(k, 8, 8), v);
                    }
                }
            }

            if self.active[p].unk_4d != 0 {
                if !self.func_800fa11c(0xF000_0000, 0xF000_0000) {
                    self.active[p].unk_4d = 0;
                    return;
                }
                if self.active[p].unk_4c != 0 {
                    self.active[p].unk_4c -= 1;
                    continue;
                }
                if self.view.players[p].enabled {
                    continue;
                }
                for j in 0..self.active[p].unk_4d as usize {
                    let c = self.active[p].unk_2c[j];
                    let temp_a0 = ((c & 0x00F0_0000) >> 20) as u8;
                    let temp_s1 = ((c & 0x000F_0000) >> 16) as u8;
                    let temp_s0_3 = ((c & 0xFF00) >> 8) as u8;
                    let temp_a3_3 = (c & 0xFF) as u8;
                    let s1 = temp_s1 as usize;
                    match temp_a0 {
                        0 => self.set_vol_scale(temp_s1, 1, 0x7F, temp_a3_3),
                        7 => {
                            if self.d_8016e348[p] == temp_a3_3 {
                                self.set_vol_scale(temp_s1, 1, 0x7F, temp_s0_3);
                            }
                        }
                        1 => {
                            // Audio_SeqCmd3 (the macro without casts: the whole unk_254).
                            let s = self.active[p].unk_254 as u32;
                            self.queue_seq_cmd(0x3000_0000 | ((player_idx as u32) << 24) | s);
                        }
                        2 => {
                            // Audio_StartSeq.
                            let s = self.active[s1].unk_254 as u32;
                            self.queue_seq_cmd(((temp_s1 as u32) << 24) | (1 << 16) | s);
                            self.active[s1].fade_vol_update = 1;
                            self.active[s1].vol_scales[1] = 0x7F;
                        }
                        3 => {
                            // Audio_SeqCmdB30.
                            self.queue_seq_cmd(0xB000_3000 | ((temp_s1 as u32) << 24) | ((temp_s0_3 as u32) << 16) | temp_a3_3 as u32);
                        }
                        4 => {
                            // Audio_SeqCmdB40.
                            self.queue_seq_cmd(0xB000_4000 | ((temp_s1 as u32) << 24) | ((temp_a3_3 as u32) << 16));
                        }
                        5 => {
                            let temp_v1 = (c & 0xFFFF) as u16;
                            let t = self.active[s1].unk_4e as u32;
                            self.queue_seq_cmd(((temp_s1 as u32) << 24) | (t << 16) | temp_v1 as u32);
                            self.set_vol_scale(temp_s1, 1, 0x7F, 0);
                            self.active[s1].unk_4e = 0;
                        }
                        6 => self.active[p].unk_4e = temp_s0_3,
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
                            // Audio_SeqCmdA.
                            let temp_v1 = (c & 0xFFFF) as u32;
                            self.queue_seq_cmd(0xA000_0000 | ((temp_s1 as u32) << 24) | temp_v1);
                        }
                        10 => {
                            // Audio_SeqCmd5.
                            let v = ((temp_a3_3 as u32 * 10) & 0xFFFF) as u32;
                            self.queue_seq_cmd(0x5000_0000 | ((temp_s1 as u32) << 24) | ((temp_s0_3 as u32) << 16) | v);
                        }
                        _ => {}
                    }
                }
                self.active[p].unk_4d = 0;
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

    /// `func_800FADF8`: the players' queues and the game's side of them, reset.
    pub fn func_800fadf8(&mut self) {
        for p in 0..4 {
            self.d_8016e348[p] = 0;
            let a = &mut self.active[p];
            a.unk_254 = NA_BGM_DISABLED;
            a.unk_256 = NA_BGM_DISABLED;
            a.unk_28 = 0;
            a.unk_18 = 0;
            a.unk_14 = 0;
            a.unk_258 = 0;
            a.unk_4d = 0;
            a.unk_4e = 0;
            a.unk_250 = 0;
            a.unk_252 = 0;
            a.vol_scales = [0x7F; 4];
            a.vol_fade_timer = 1;
            a.fade_vol_update = 1;
        }
    }

    /// `func_800FAEB4`: as `func_800FADF8`, with the volumes at full.
    pub fn func_800faeb4(&mut self) {
        for a in self.active.iter_mut() {
            a.vol_cur = 1.0;
            a.unk_0c = 0;
            a.fade_vol_update = 0;
            a.vol_scales = [0x7F; 4];
        }
        self.func_800fadf8();
    }
}
