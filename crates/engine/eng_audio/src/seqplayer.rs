//! `seqplayer.c`: the sequence player. A sequence is three kinds of script, each with
//! its own interpreter and the shared flow control (0xF2 and up): the player's (which channels
//! play), each channel's (instrument, volume, pan, its layers) and each layer's (the notes).
//! Every update runs the player's script when its tempo says a tick has passed.
//!
//! The scripts run from RAM (`seqData` is where the sequence was loaded), so the
//! self-modifying commands (0xC7, 0xCF) and the game's IO ports work as in the C.

use crate::context::*;
use crate::layout::*;
use crate::playback::layer_node;

/// `PROCESS_SCRIPT_END`.
const PROCESS_SCRIPT_END: i32 = -1;

/// `PortamentoMode`.
const PORTAMENTO_MODE_OFF: u8 = 0;
const PORTAMENTO_MODE_1: u8 = 1;
const PORTAMENTO_MODE_2: u8 = 2;
const PORTAMENTO_MODE_3: u8 = 3;
const PORTAMENTO_MODE_4: u8 = 4;
const PORTAMENTO_MODE_5: u8 = 5;

fn portamento_is_special(p: &Portamento) -> bool {
    p.mode & 0x80 != 0
}
fn portamento_mode(p: &Portamento) -> u8 {
    p.mode & !0x80
}

/// `sSeqInstructionArgsTable`: `abcUUUnn`, `n` the number of arguments, `a`/`b`/`c` set when
/// that argument is an s16 (else a u8). Built with the C's `CMD_ARGS_*`.
const fn args1(s16: bool) -> u8 {
    ((s16 as u8) << 7) | 1
}
const fn args2(a: bool, b: bool) -> u8 {
    ((a as u8) << 7) | ((b as u8) << 6) | 2
}
const fn args3(a: bool, b: bool, c: bool) -> u8 {
    ((a as u8) << 7) | ((b as u8) << 6) | ((c as u8) << 5) | 3
}
const U8: bool = false;
const S16: bool = true;
#[rustfmt::skip]
pub const SEQ_INSTRUCTION_ARGS_TABLE: [u8; 80] = [
    args1(S16), 0, args1(S16), args1(U8), 0, 0, 0, args1(S16), // 0xB0
    args1(U8), args1(U8), args1(U8), args2(U8, S16), args1(S16), args2(S16, S16), 0, 0, // 0xB8
    0, args1(U8), args1(S16), 0, 0, 0, args1(U8), args2(U8, S16), // 0xC0
    args1(U8), args1(U8), args1(U8), args1(S16), args1(U8), args1(U8), args1(S16), args1(S16), // 0xC8
    args1(U8), args1(U8), args1(U8), args1(U8), args1(U8), args1(U8), args1(U8), args1(U8), // 0xD0
    args1(U8), args1(U8), args1(S16), args1(U8), args1(U8), args1(U8), args1(S16), args1(U8), // 0xD8
    args1(U8), args3(U8, U8, U8), args3(U8, U8, U8), args1(U8), 0, args1(U8), args1(U8), args1(S16), // 0xE0
    args3(U8, U8, U8), args1(U8), 0, args2(U8, U8), 0, args1(U8), args1(U8), args2(S16, U8), // 0xE8
    0, args1(U8), // 0xF0
    // Control flow instructions (>= 0xF2) can only have 0 or 1 args
    args1(U8), args1(U8), args1(U8), args1(S16), 0, 0, args1(U8), args1(S16), // 0xF2
    args1(S16), args1(S16), args1(S16), 0, 0, 0, // 0xFA
];

/// Which script a state is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Script {
    Player(usize),
    Channel(ChanId),
    Layer(LayerId),
}

impl AudioContext {
    fn ss(&mut self, s: Script) -> &mut SeqScriptState {
        match s {
            Script::Player(p) => &mut self.seq_players[p].script_state,
            Script::Channel(c) => &mut self.channels[c].script_state,
            Script::Layer(l) => &mut self.sequence_layers[l].script_state,
        }
    }

    /// `AudioSeq_ScriptReadU8`.
    pub fn script_read_u8(&mut self, s: Script) -> u8 {
        let st = self.ss(s);
        let pc = st.pc();
        st.set_pc(pc.wrapping_add(1));
        self.ram.u8(pc)
    }

    /// `AudioSeq_ScriptReadS16`.
    pub fn script_read_s16(&mut self, s: Script) -> i16 {
        let hi = (self.script_read_u8(s) as u16) << 8;
        (self.script_read_u8(s) as u16 | hi) as i16
    }

    /// `AudioSeq_ScriptReadCompressedU16`.
    pub fn script_read_compressed_u16(&mut self, s: Script) -> u16 {
        let mut ret = self.script_read_u8(s) as u16;
        if ret & 0x80 != 0 {
            ret = (ret << 8) & 0x7F00;
            ret |= self.script_read_u8(s) as u16;
        }
        ret
    }

    /// `AudioSeq_GetScriptControlFlowArgument`.
    pub fn get_script_control_flow_argument(&mut self, s: Script, cmd: u8) -> u16 {
        let high_bits = SEQ_INSTRUCTION_ARGS_TABLE[(cmd - 0xB0) as usize];
        if high_bits & 3 == 1 { if high_bits & 0x80 == 0 { self.script_read_u8(s) as u16 } else { self.script_read_s16(s) as u16 } } else { 0 }
    }

    /// `AudioSeq_HandleScriptFlowControl`: the number of frames until the next command, or
    /// `PROCESS_SCRIPT_END`.
    pub fn handle_script_flow_control(&mut self, seq_data: u32, s: Script, cmd: u8, cmd_arg: u16) -> i32 {
        match cmd {
            0xFF => {
                let st = self.ss(s);
                if st.depth() == 0 {
                    return PROCESS_SCRIPT_END;
                }
                let d = st.depth().wrapping_sub(1);
                st.set_depth(d);
                let pc = st.stack(d as i32);
                st.set_pc(pc);
            }
            0xFD => return self.script_read_compressed_u16(s) as i32,
            0xFE => return 1,
            0xFC => {
                let st = self.ss(s);
                let d = st.depth();
                let pc = st.pc();
                st.set_stack(d as i32, pc);
                st.set_depth(d.wrapping_add(1));
                st.set_pc(seq_data.wrapping_add(cmd_arg as u32));
            }
            0xF8 => {
                let st = self.ss(s);
                let d = st.depth();
                st.set_rem_loop_iters(d as i32, cmd_arg as u8);
                let pc = st.pc();
                st.set_stack(d as i32, pc);
                st.set_depth(d.wrapping_add(1));
            }
            0xF7 => {
                let st = self.ss(s);
                let i = st.depth() as i32 - 1;
                let r = st.rem_loop_iters(i).wrapping_sub(1);
                st.set_rem_loop_iters(i, r);
                if r != 0 {
                    let pc = st.stack(i);
                    st.set_pc(pc);
                } else {
                    let d = st.depth().wrapping_sub(1);
                    st.set_depth(d);
                }
            }
            0xF6 => {
                let st = self.ss(s);
                let d = st.depth().wrapping_sub(1);
                st.set_depth(d);
            }
            0xF5 | 0xF9 | 0xFA | 0xFB => {
                let st = self.ss(s);
                let v = st.value();
                if cmd == 0xFA && v != 0 {
                    return 0;
                }
                if cmd == 0xF9 && v >= 0 {
                    return 0;
                }
                if cmd == 0xF5 && v < 0 {
                    return 0;
                }
                st.set_pc(seq_data.wrapping_add(cmd_arg as u32));
            }
            0xF2..=0xF4 => {
                let st = self.ss(s);
                let v = st.value();
                if cmd == 0xF3 && v != 0 {
                    return 0;
                }
                if cmd == 0xF2 && v >= 0 {
                    return 0;
                }
                let pc = st.pc();
                st.set_pc(pc.wrapping_add((cmd_arg & 0xFF) as u8 as i8 as i32 as u32));
            }
            _ => {}
        }
        0
    }

    /// `AudioSeq_InitSequenceChannel`.
    pub fn init_sequence_channel(&mut self, c: ChanId) {
        if c == CHANNEL_NONE {
            return;
        }
        let default_envelope = self.statics.default_envelope;
        let ch = &mut self.channels[c];
        ch.enabled = false;
        ch.finished = false;
        ch.stop_script = false;
        ch.stop_something2 = false;
        ch.has_instrument = false;
        ch.stereo_headset_effects = false;
        ch.transposition = 0;
        ch.large_notes = false;
        ch.book_offset = 0;
        ch.stereo = 0;
        ch.changes = 0xFF;
        ch.script_state.set_depth(0);
        ch.new_pan = 0x40;
        ch.pan_channel_weight = 0x80;
        ch.velocity_random_variance = 0;
        ch.gate_time_random_variance = 0;
        ch.note_unused = None;
        ch.reverb_index = 0;
        ch.reverb = 0;
        ch.gain = 0;
        ch.note_priority = 3;
        ch.some_other_priority = 1;
        ch.delay = 0;
        ch.adsr.envelope = default_envelope;
        ch.adsr.decay_index = 0xF0;
        ch.adsr.sustain = 0;
        ch.vibrato_rate_target = 0x800;
        ch.vibrato_rate_start = 0x800;
        ch.vibrato_extent_target = 0;
        ch.vibrato_extent_start = 0;
        ch.vibrato_rate_change_delay = 0;
        ch.vibrato_extent_change_delay = 0;
        ch.vibrato_delay = 0;
        ch.filter = 0;
        ch.comb_filter_gain = 0;
        ch.comb_filter_size = 0;
        ch.volume = 1.0;
        ch.volume_scale = 1.0;
        ch.freq_scale = 1.0;
        ch.sound_script_io = [-1; 8];
        ch.unused = false;
        let pool = ch.note_pool;
        self.init_note_lists(pool);
    }

    /// `AudioSeq_SeqChannelSetLayer`.
    pub fn seq_channel_set_layer(&mut self, c: ChanId, layer_index: usize) -> i32 {
        if self.channels[c].layers[layer_index].is_none() {
            let free = self.layer_free_list;
            match self.audio_list_pop_back(free) {
                Some(Item::Layer(l)) => self.channels[c].layers[layer_index] = Some(l),
                _ => {
                    self.channels[c].layers[layer_index] = None;
                    return -1;
                }
            }
        } else {
            let l = self.channels[c].layers[layer_index];
            self.seq_layer_note_decay(l);
        }
        let l = self.channels[c].layers[layer_index].unwrap();
        let adsr = self.channels[c].adsr;
        let layer = &mut self.sequence_layers[l];
        layer.channel = Some(c);
        layer.adsr = adsr;
        layer.adsr.decay_index = 0;
        layer.enabled = true;
        layer.finished = false;
        layer.stop_something = false;
        layer.continuous_notes = false;
        layer.bit3 = false;
        layer.ignore_drum_pan = false;
        layer.bit1 = false;
        layer.note_properties_need_init = false;
        layer.stereo = 0;
        layer.portamento.mode = PORTAMENTO_MODE_OFF;
        layer.script_state.set_depth(0);
        layer.gate_time = 0x80;
        layer.pan = 0x40;
        layer.transposition = 0;
        layer.delay = 0;
        layer.gate_delay = 0;
        layer.delay2 = 0;
        layer.note = None;
        layer.instrument = 0;
        layer.freq_scale = 1.0;
        layer.bend = 1.0;
        layer.velocity_square2 = 0.0;
        layer.inst_or_wave = 0xFF;
        0
    }

    /// `AudioSeq_SeqLayerDisable`.
    pub fn seq_layer_disable(&mut self, l: LayerId) {
        let chan = self.sequence_layers[l].channel;
        let finished = chan.filter(|&c| c != CHANNEL_NONE).and_then(|c| self.channels[c].seq_player).map(|p| self.seq_players[p].finished).unwrap_or(false);
        if finished {
            self.seq_layer_note_release(Some(l));
        } else {
            self.seq_layer_note_decay(Some(l));
        }
        self.sequence_layers[l].enabled = false;
        self.sequence_layers[l].finished = true;
    }

    /// `AudioSeq_SeqLayerFree`.
    pub fn seq_layer_free(&mut self, c: ChanId, layer_index: usize) {
        if let Some(l) = self.channels[c].layers[layer_index] {
            let free = self.layer_free_list;
            self.audio_list_push_back(free, layer_node(l));
            self.seq_layer_disable(l);
            self.channels[c].layers[layer_index] = None;
        }
    }

    /// `AudioSeq_SequenceChannelDisable`.
    pub fn sequence_channel_disable(&mut self, c: ChanId) {
        for i in 0..4 {
            self.seq_layer_free(c, i);
        }
        let pool = self.channels[c].note_pool;
        self.note_pool_clear(pool);
        self.channels[c].enabled = false;
        self.channels[c].finished = true;
    }

    /// `AudioSeq_SequencePlayerSetupChannels`.
    pub fn sequence_player_setup_channels(&mut self, p: usize, mut channel_bits: u16) {
        for i in 0..SEQ_NUM_CHANNELS {
            if channel_bits & 1 != 0 {
                let c = self.seq_players[p].channels[i];
                let (font, mute, policy) = (self.seq_players[p].default_font, self.seq_players[p].mute_behavior, self.seq_players[p].note_alloc_policy);
                let ch = &mut self.channels[c];
                ch.font_id = font;
                ch.mute_behavior = mute;
                ch.note_alloc_policy = policy;
            }
            channel_bits >>= 1;
        }
    }

    /// `AudioSeq_SequencePlayerDisableChannels`.
    pub fn sequence_player_disable_channels(&mut self, p: usize, _channel_bits_unused: u16) {
        for i in 0..SEQ_NUM_CHANNELS {
            let c = self.seq_players[p].channels[i];
            if c != CHANNEL_NONE {
                self.sequence_channel_disable(c);
            }
        }
    }

    /// `AudioSeq_SequenceChannelEnable`.
    pub fn sequence_channel_enable(&mut self, p: usize, channel_index: u8, script: u32) {
        let c = self.seq_players[p].channels[channel_index as usize & 0xF];
        let ch = &mut self.channels[c];
        ch.enabled = true;
        ch.finished = false;
        ch.script_state.set_depth(0);
        ch.script_state.set_pc(script);
        ch.delay = 0;
        for i in 0..4 {
            if self.channels[c].layers[i].is_some() {
                self.seq_layer_free(c, i);
            }
        }
    }

    /// `AudioSeq_SequencePlayerDisableAsFinished`.
    pub fn sequence_player_disable_as_finished(&mut self, p: usize) {
        self.seq_players[p].finished = true;
        self.sequence_player_disable(p);
    }

    /// `AudioSeq_SequencePlayerDisable`.
    pub fn sequence_player_disable(&mut self, p: usize) {
        self.sequence_player_disable_channels(p, 0xFFFF);
        let pool = self.seq_players[p].note_pool;
        self.note_pool_clear(pool);
        if !self.seq_players[p].enabled {
            return;
        }
        self.seq_players[p].enabled = false;
        self.seq_players[p].finished = true;
        let (seq_id, default_font) = (self.seq_players[p].seq_id as i32, self.seq_players[p].default_font as i32);
        if self.is_seq_load_complete(seq_id) {
            self.set_seq_load_status(seq_id, LOAD_STATUS_DISCARDABLE);
        }
        if self.is_font_load_complete(default_font) {
            self.set_font_load_status(default_font, LOAD_STATUS_MAYBE_DISCARDABLE);
        }
        if default_font == self.font_cache.temporary.entries[0].id as i32 {
            self.font_cache.temporary.next_side = 0;
        } else if default_font == self.font_cache.temporary.entries[1].id as i32 {
            self.font_cache.temporary.next_side = 1;
        }
    }

    /// `AudioSeq_InitLayerFreelist`.
    pub fn init_layer_freelist(&mut self) {
        let h = self.layer_free_list;
        self.lists[h] = ListNode { prev: Some(h), next: h, count: 0, pool: None, value: Item::Head };
        for l in 0..64 {
            self.lists[layer_node(l)].prev = None;
            self.audio_list_push_back(h, layer_node(l));
        }
    }

    /// `AudioSeq_SeqLayerProcessScript`.
    pub fn seq_layer_process_script(&mut self, l: LayerId) {
        if !self.sequence_layers[l].enabled {
            return;
        }
        let layer = &mut self.sequence_layers[l];
        if layer.delay > 1 {
            layer.delay -= 1;
            if !layer.stop_something && layer.delay <= layer.gate_delay {
                self.seq_layer_note_decay(Some(l));
                self.sequence_layers[l].stop_something = true;
            }
            return;
        }

        self.seq_layer_process_script_step1(l);

        let mut cmd = self.seq_layer_process_script_step2(l);
        if cmd == PROCESS_SCRIPT_END {
            return;
        }
        cmd = self.seq_layer_process_script_step3(l, cmd);
        if cmd != PROCESS_SCRIPT_END {
            // returns `sameSound` instead of a command
            cmd = self.seq_layer_process_script_step4(l, cmd);
        }
        if cmd != PROCESS_SCRIPT_END {
            self.seq_layer_process_script_step5(l, cmd);
        }
        let layer = &self.sequence_layers[l];
        if layer.stop_something && (layer.note.is_some() || layer.continuous_notes) {
            self.seq_layer_note_decay(Some(l));
        }
    }

    /// `AudioSeq_SeqLayerProcessScriptStep1`.
    fn seq_layer_process_script_step1(&mut self, l: LayerId) {
        let layer = &self.sequence_layers[l];
        if !layer.continuous_notes {
            self.seq_layer_note_decay(Some(l));
        } else if let Some(n) = layer.note {
            if self.notes[n].playback_state.wanted_parent_layer == Some(l) {
                self.seq_layer_note_decay(Some(l));
            }
        }
        let layer = &mut self.sequence_layers[l];
        let m = portamento_mode(&layer.portamento);
        if m == PORTAMENTO_MODE_1 || m == PORTAMENTO_MODE_2 {
            layer.portamento.mode = PORTAMENTO_MODE_OFF;
        }
        layer.note_properties_need_init = true;
    }

    /// `AudioSeq_SeqLayerProcessScriptStep5`.
    fn seq_layer_process_script_step5(&mut self, l: LayerId, same_tuned_sample: i32) -> i32 {
        let layer = &self.sequence_layers[l];
        if !layer.stop_something && layer.tuned_sample != 0 {
            let s = self.ram.sample(self.ram.u32(layer.tuned_sample));
            if s.codec == CODEC_S16_INMEMORY && s.medium != MEDIUM_RAM {
                self.sequence_layers[l].stop_something = true;
                return PROCESS_SCRIPT_END;
            }
        }
        let layer = &self.sequence_layers[l];
        if layer.continuous_notes && layer.bit1 {
            return 0;
        }
        let parent_is_layer = layer.note.map(|n| self.notes[n].playback_state.parent_layer == Some(l)).unwrap_or(false);
        if layer.continuous_notes && layer.note.is_some() && layer.bit3 && same_tuned_sample == 1 && parent_is_layer {
            if layer.tuned_sample == 0 {
                let n = layer.note.unwrap();
                self.init_synthetic_wave(n, l);
            }
        } else {
            if same_tuned_sample == 0 {
                self.seq_layer_note_decay(Some(l));
            }
            let n = self.alloc_note(l);
            self.sequence_layers[l].note = n;
            if let Some(n) = n {
                if self.notes[n].playback_state.parent_layer == Some(l) {
                    self.note_vibrato_init(n);
                }
            }
        }
        if let Some(n) = self.sequence_layers[l].note {
            if self.notes[n].playback_state.parent_layer == Some(l) {
                self.note_portamento_init(n);
            }
        }
        0
    }

    /// `AudioSeq_SeqLayerProcessScriptStep2`.
    fn seq_layer_process_script_step2(&mut self, l: LayerId) -> i32 {
        let s = Script::Layer(l);
        let c = self.sequence_layers[l].channel.unwrap_or(CHANNEL_NONE);
        let p = self.channels[c].seq_player.unwrap_or(0);
        loop {
            let cmd = self.script_read_u8(s);
            // To be processed in AudioSeq_SeqLayerProcessScriptStep3
            if cmd <= 0xC0 {
                return cmd as i32;
            }
            // Control Flow Commands
            if cmd >= 0xF2 {
                let arg = self.get_script_control_flow_argument(s, cmd);
                let seq_data = self.seq_players[p].seq_data;
                if self.handle_script_flow_control(seq_data, s, cmd, arg) == 0 {
                    continue;
                }
                self.seq_layer_disable(l);
                return PROCESS_SCRIPT_END;
            }
            match cmd {
                0xC1 | 0xCA => {
                    // layer_setshortnotevelocity, layer_setpan
                    let arg = self.script_read_u8(s);
                    if cmd == 0xC1 {
                        self.sequence_layers[l].velocity_square = (arg as i32 * arg as i32) as f32 / (127.0f32 * 127.0f32);
                    } else {
                        self.sequence_layers[l].pan = arg;
                    }
                }
                0xC9 | 0xC2 => {
                    // layer_setshortnotegatetime, layer_transpose
                    let arg = self.script_read_u8(s);
                    if cmd == 0xC9 {
                        self.sequence_layers[l].gate_time = arg;
                    } else {
                        self.sequence_layers[l].transposition = arg as i16;
                    }
                }
                0xC4 | 0xC5 => {
                    // layer_continuousnoteson, layer_continuousnotesoff
                    self.sequence_layers[l].continuous_notes = cmd == 0xC4;
                    self.sequence_layers[l].bit1 = false;
                    self.seq_layer_note_decay(Some(l));
                }
                0xC3 => {
                    // layer_setshortnotedefaultdelay
                    let v = self.script_read_compressed_u16(s);
                    self.sequence_layers[l].short_note_default_delay = v as i16;
                }
                0xC6 => {
                    // layer_setinstr
                    let cmd = self.script_read_u8(s);
                    if cmd >= 0x7E {
                        let layer = &mut self.sequence_layers[l];
                        if cmd == 0x7E {
                            // Sfxs
                            layer.inst_or_wave = 1;
                        } else if cmd == 0x7F {
                            // Drums
                            layer.inst_or_wave = 0;
                        } else {
                            // Synthetic Wave
                            layer.inst_or_wave = cmd;
                            layer.instrument = 0;
                        }
                        if cmd == 0xFF {
                            layer.adsr.decay_index = 0;
                        }
                    } else {
                        // Instrument
                        let mut inst = 0;
                        let mut adsr = self.sequence_layers[l].adsr;
                        let v = self.get_instrument(c, cmd, &mut inst, &mut adsr);
                        let layer = &mut self.sequence_layers[l];
                        layer.instrument = inst;
                        layer.adsr = adsr;
                        layer.inst_or_wave = v;
                        if v == 0 {
                            layer.inst_or_wave = 0xFF;
                        }
                    }
                }
                0xC7 => {
                    // layer_portamento
                    let mode = self.script_read_u8(s);
                    self.sequence_layers[l].portamento.mode = mode;
                    let mut cmd = self.script_read_u8(s);
                    cmd = cmd.wrapping_add(self.channels[c].transposition as u8);
                    cmd = cmd.wrapping_add(self.sequence_layers[l].transposition as u8);
                    cmd = cmd.wrapping_add(self.seq_players[p].transposition as u8);
                    if cmd >= 0x80 {
                        cmd = 0;
                    }
                    self.sequence_layers[l].portamento_target_note = cmd;
                    // If special, the next param is u8 instead of var
                    if portamento_is_special(&self.sequence_layers[l].portamento) {
                        let v = self.script_read_u8(s);
                        self.sequence_layers[l].portamento_time = v as u16;
                    } else {
                        let v = self.script_read_compressed_u16(s);
                        self.sequence_layers[l].portamento_time = v;
                    }
                }
                0xC8 => self.sequence_layers[l].portamento.mode = PORTAMENTO_MODE_OFF, // layer_disableportamento
                0xCB | 0xCF => {
                    if cmd == 0xCB {
                        let off = self.script_read_s16(s) as i32;
                        self.sequence_layers[l].adsr.envelope = self.seq_players[p].seq_data.wrapping_add(off as u32);
                    }
                    let v = self.script_read_u8(s);
                    self.sequence_layers[l].adsr.decay_index = v;
                }
                0xCC => self.sequence_layers[l].ignore_drum_pan = true,
                0xCD => {
                    let v = self.script_read_u8(s);
                    self.sequence_layers[l].stereo = v;
                }
                0xCE => {
                    let v = self.script_read_u8(s);
                    self.sequence_layers[l].bend = self.tables.bend_pitch_two_semitones_frequencies[v.wrapping_add(0x80) as usize];
                }
                _ => match cmd & 0xF0 {
                    0xD0 => {
                        // layer_setshortnotevelocityfromtable
                        let t = self.seq_players[p].short_note_velocity_table;
                        let velocity = self.ram.u8(t + (cmd & 0xF) as u32) as u16;
                        self.sequence_layers[l].velocity_square = (velocity as i32 * velocity as i32) as f32 / (127.0f32 * 127.0f32);
                    }
                    0xE0 => {
                        // layer_setshortnotegatetimefromtable
                        let t = self.seq_players[p].short_note_gate_time_table;
                        self.sequence_layers[l].gate_time = self.ram.u8(t + (cmd & 0xF) as u32);
                    }
                    _ => {}
                },
            }
        }
    }

    /// `AudioSeq_SeqLayerProcessScriptStep4`: picks the sample and frequency for the note;
    /// returns whether the sample is the one the layer played before (`sameTunedSample`).
    fn seq_layer_process_script_step4(&mut self, l: LayerId, cmd: i32) -> i32 {
        let mut same_tuned_sample = 1;
        let mut semitone = cmd as u8;
        let c = self.sequence_layers[l].channel.unwrap_or(CHANNEL_NONE);
        let p = self.channels[c].seq_player.unwrap_or(0);
        let mut inst_or_wave = self.sequence_layers[l].inst_or_wave as i32;
        if inst_or_wave == 0xFF {
            if !self.channels[c].has_instrument {
                return PROCESS_SCRIPT_END;
            }
            inst_or_wave = self.channels[c].inst_or_wave as i32;
        }
        let mut portamento_special_delay0 = false;
        match inst_or_wave {
            0 => {
                // Drums
                semitone = semitone.wrapping_add(self.channels[c].transposition as u8).wrapping_add(self.sequence_layers[l].transposition as u8);
                self.sequence_layers[l].semitone = semitone;
                let font = self.channels[c].font_id as i32;
                let drum = self.get_drum(font, semitone as i32);
                let layer = &mut self.sequence_layers[l];
                if drum == 0 {
                    layer.stop_something = true;
                    layer.delay2 = layer.delay;
                    return PROCESS_SCRIPT_END;
                }
                let tuned_sample = drum + DRUM_TUNED_SAMPLE;
                layer.adsr.envelope = self.ram.u32(drum + DRUM_ENVELOPE);
                layer.adsr.decay_index = self.ram.u8(drum + DRUM_ADSR_DECAY_INDEX);
                if !layer.ignore_drum_pan {
                    layer.pan = self.ram.u8(drum + DRUM_PAN);
                }
                layer.tuned_sample = tuned_sample;
                layer.freq_scale = self.ram.f32(tuned_sample + 4);
            }
            1 => {
                // Sfxs
                self.sequence_layers[l].semitone = semitone;
                let sfx_id = ((self.sequence_layers[l].transposition as i32) << 6) as u16 as i32 + semitone as i32;
                let font = self.channels[c].font_id as i32;
                let sfx = self.get_sound_effect(font, sfx_id & 0xFFFF);
                let layer = &mut self.sequence_layers[l];
                if sfx == 0 {
                    layer.stop_something = true;
                    layer.delay2 = layer.delay + 1;
                    return PROCESS_SCRIPT_END;
                }
                layer.tuned_sample = sfx;
                layer.freq_scale = self.ram.f32(sfx + 4);
            }
            _ => {
                semitone = semitone.wrapping_add(self.seq_players[p].transposition as u8).wrapping_add(self.channels[c].transposition as u8).wrapping_add(self.sequence_layers[l].transposition as u8);
                let semitone2 = semitone as usize;
                self.sequence_layers[l].semitone = semitone;
                if semitone >= 0x80 {
                    self.sequence_layers[l].stop_something = true;
                    return PROCESS_SCRIPT_END;
                }
                let instrument = if self.sequence_layers[l].inst_or_wave == 0xFF { self.channels[c].instrument } else { self.sequence_layers[l].instrument };
                let pf = &self.tables.pitch_frequencies;

                if self.sequence_layers[l].portamento.mode != PORTAMENTO_MODE_OFF {
                    let target = self.sequence_layers[l].portamento_target_note;
                    let vel = if semitone > target { semitone } else { target };
                    let tuning;
                    if instrument != 0 {
                        let tuned_sample = self.get_instrument_tuned_sample(instrument, vel as i32);
                        same_tuned_sample = (self.sequence_layers[l].tuned_sample == tuned_sample) as i32;
                        self.sequence_layers[l].tuned_sample = tuned_sample;
                        tuning = self.ram.f32(tuned_sample + 4);
                    } else {
                        self.sequence_layers[l].tuned_sample = 0;
                        tuning = 1.0f32;
                        if inst_or_wave >= 0xC0 {
                            self.sequence_layers[l].tuned_sample = self.synthesis_reverbs[(inst_or_wave - 0xC0) as usize & 3].tuned_sample_addr;
                        }
                    }
                    let temp_f2 = pf[semitone2] * tuning;
                    let temp_f14 = pf[target as usize] * tuning;
                    let layer = &mut self.sequence_layers[l];
                    let (freq_scale, freq_scale2) = match portamento_mode(&layer.portamento) {
                        PORTAMENTO_MODE_1 | PORTAMENTO_MODE_3 | PORTAMENTO_MODE_5 => (temp_f14, temp_f2),
                        PORTAMENTO_MODE_2 | PORTAMENTO_MODE_4 => (temp_f2, temp_f14),
                        _ => (temp_f2, temp_f2),
                    };
                    layer.portamento.extent = (freq_scale2 / freq_scale) - 1.0f32;
                    let mut speed: i32;
                    if portamento_is_special(&layer.portamento) {
                        speed = self.seq_players[p].tempo as i32 * 0x8000 / self.tempo_internal_to_external as i32;
                        if layer.delay != 0 {
                            speed = speed * 0x100 / (layer.delay as i32 * layer.portamento_time as i32).max(1);
                        }
                    } else {
                        speed = 0x20000 / (layer.portamento_time as i32 * self.audio_buffer_parameters.updates_per_frame as i32).max(1);
                    }
                    speed = speed.clamp(1, 0x7FFF);
                    layer.portamento.speed = speed as u16;
                    layer.portamento.cur = 0;
                    layer.freq_scale = freq_scale;
                    if portamento_mode(&layer.portamento) == PORTAMENTO_MODE_5 {
                        layer.portamento_target_note = semitone;
                    }
                    portamento_special_delay0 = true;
                } else if instrument != 0 {
                    let tuned_sample = self.get_instrument_tuned_sample(instrument, semitone as i32);
                    same_tuned_sample = (tuned_sample == self.sequence_layers[l].tuned_sample) as i32;
                    let tuning = self.ram.f32(tuned_sample + 4);
                    let layer = &mut self.sequence_layers[l];
                    layer.tuned_sample = tuned_sample;
                    layer.freq_scale = self.tables.pitch_frequencies[semitone2] * tuning;
                } else {
                    let layer = &mut self.sequence_layers[l];
                    layer.tuned_sample = 0;
                    layer.freq_scale = self.tables.pitch_frequencies[semitone2];
                    if inst_or_wave >= 0xC0 {
                        layer.tuned_sample = self.synthesis_reverbs[(inst_or_wave - 0xC0) as usize & 3].tuned_sample_addr;
                    }
                }
            }
        }

        let tempo = self.seq_players[p].tempo;
        let layer = &mut self.sequence_layers[l];
        layer.delay2 = layer.delay;
        layer.freq_scale *= layer.bend;
        if layer.delay == 0 {
            let mut time: f32 = if layer.tuned_sample != 0 {
                let sample = self.ram.u32(layer.tuned_sample);
                let lp = self.ram.sample(sample).loop_addr;
                self.ram.u32(lp + 4) as f32
            } else {
                0.0
            };
            time *= tempo as f32;
            time *= self.unk_2870;
            time /= layer.freq_scale;
            if time > 0x7FFE as f32 {
                time = 0x7FFE as f32;
            }
            layer.gate_delay = 0;
            layer.delay = ((time as i32 as u16) + 1) as i16;
            if layer.portamento.mode != PORTAMENTO_MODE_OFF {
                // (It's a bit unclear if 'portamento' has actually always been set when this is
                // reached...)
                if portamento_is_special(&layer.portamento) {
                    let _ = portamento_special_delay0;
                    let mut speed2 = tempo as i32 * 0x8000 / self.tempo_internal_to_external as i32;
                    speed2 = speed2 * 0x100 / (layer.delay as i32 * layer.portamento_time as i32).max(1);
                    layer.portamento.speed = speed2.clamp(1, 0x7FFF) as u16;
                }
            }
        }
        self.stats.notes_struck += 1;
        same_tuned_sample
    }

    /// `AudioSeq_SeqLayerProcessScriptStep3`: the note's delay, velocity and gate.
    fn seq_layer_process_script_step3(&mut self, l: LayerId, cmd: i32) -> i32 {
        let s = Script::Layer(l);
        let c = self.sequence_layers[l].channel.unwrap_or(CHANNEL_NONE);
        let p = self.channels[c].seq_player.unwrap_or(0);
        if cmd == 0xC0 {
            let d = self.script_read_compressed_u16(s);
            let layer = &mut self.sequence_layers[l];
            layer.delay = d as i16;
            layer.stop_something = true;
            layer.bit1 = false;
            return PROCESS_SCRIPT_END;
        }
        self.sequence_layers[l].stop_something = false;
        let mut cmd = cmd;
        let delay: u16;
        if self.channels[c].large_notes {
            let mut velocity: i32;
            match cmd & 0xC0 {
                0x00 => {
                    delay = self.script_read_compressed_u16(s);
                    velocity = self.script_read_u8(s) as i32;
                    let g = self.script_read_u8(s);
                    self.sequence_layers[l].gate_time = g;
                    self.sequence_layers[l].last_delay = delay as i16;
                }
                0x40 => {
                    delay = self.script_read_compressed_u16(s);
                    velocity = self.script_read_u8(s) as i32;
                    self.sequence_layers[l].gate_time = 0;
                    self.sequence_layers[l].last_delay = delay as i16;
                }
                _ => {
                    delay = self.sequence_layers[l].last_delay as u16;
                    velocity = self.script_read_u8(s) as i32;
                    let g = self.script_read_u8(s);
                    self.sequence_layers[l].gate_time = g;
                }
            }
            if !(0..=0x7F).contains(&velocity) {
                velocity = 0x7F;
            }
            self.sequence_layers[l].velocity_square = (velocity as f32 * velocity as f32) / (127.0f32 * 127.0f32);
            cmd -= cmd & 0xC0;
        } else {
            match cmd & 0xC0 {
                0x00 => {
                    delay = self.script_read_compressed_u16(s);
                    self.sequence_layers[l].last_delay = delay as i16;
                }
                0x40 => delay = self.sequence_layers[l].short_note_default_delay as u16,
                _ => delay = self.sequence_layers[l].last_delay as u16,
            }
            cmd -= cmd & 0xC0;
        }

        let ch = &self.channels[c];
        let (vrv, gtrv) = (ch.velocity_random_variance as u32, ch.gate_time_random_variance);
        let random = self.audio_random;
        let layer = &mut self.sequence_layers[l];
        if vrv != 0 {
            let mut float_delta = layer.velocity_square * (random % vrv) as f32 / 100.0f32;
            if random & 0x8000 != 0 {
                float_delta = -float_delta;
            }
            layer.velocity_square2 = (layer.velocity_square + float_delta).clamp(0.0, 1.0);
        } else {
            layer.velocity_square2 = layer.velocity_square;
        }

        layer.delay = delay as i16;
        layer.gate_delay = ((layer.gate_time as i32 * delay as i32) >> 8) as i16;

        if gtrv != 0 {
            // @bug (game): should probably be gateTimeRandomVariance
            let mut int_delta = (layer.gate_delay as i32 * random.checked_rem(vrv).unwrap_or(0) as i32) / 100;
            if random & 0x4000 != 0 {
                int_delta = -int_delta;
            }
            layer.gate_delay = (layer.gate_delay as i32 + int_delta) as i16;
            if layer.gate_delay < 0 {
                layer.gate_delay = 0;
            } else if layer.gate_delay > layer.delay {
                layer.gate_delay = layer.delay;
            }
        }

        let sp = &self.seq_players[p];
        let ch = &self.channels[c];
        if (sp.muted && (ch.mute_behavior & (MUTE_BEHAVIOR_STOP_NOTES | MUTE_BEHAVIOR_4)) != 0) || ch.stop_something2 {
            self.sequence_layers[l].stop_something = true;
            return PROCESS_SCRIPT_END;
        }
        if sp.skip_ticks != 0 {
            self.sequence_layers[l].stop_something = true;
            return PROCESS_SCRIPT_END;
        }
        cmd
    }

    /// `AudioSeq_SetChannelPriorities`.
    pub fn set_channel_priorities(&mut self, c: ChanId, priority: u8) {
        let ch = &mut self.channels[c];
        if priority & 0xF != 0 {
            ch.note_priority = priority & 0xF;
        }
        let p = priority >> 4;
        if p != 0 {
            ch.some_other_priority = p;
        }
    }

    /// `AudioSeq_GetInstrument`: the instrument's id + 2 (0 and 1 being drums and sound
    /// effects), or 0.
    pub fn get_instrument(&mut self, c: ChanId, inst_id: u8, inst_out: &mut u32, adsr: &mut AdsrSettings) -> u8 {
        let font = self.channels[c].font_id as i32;
        let inst = self.get_instrument_inner(font, inst_id as i32);
        if inst == 0 {
            *inst_out = 0;
            return 0;
        }
        adsr.envelope = self.ram.u32(inst + INST_ENVELOPE);
        adsr.decay_index = self.ram.u8(inst + INST_ADSR_DECAY_INDEX);
        *inst_out = inst;
        // temporarily offset instrument id by 2 so that instId 0, 1 can be reserved by drums and
        // sfxs respectively.
        inst_id.wrapping_add(2)
    }

    /// `AudioSeq_SetInstrument`.
    pub fn set_instrument(&mut self, c: ChanId, inst_id: u8) {
        if inst_id >= 0x80 {
            // Synthetic Waves
            self.channels[c].inst_or_wave = inst_id as i16;
            self.channels[c].instrument = 0;
        } else if inst_id == 0x7F {
            // Drums
            self.channels[c].inst_or_wave = 0;
            self.channels[c].instrument = 1; // invalid pointer, never dereferenced
        } else if inst_id == 0x7E {
            // Sfxs
            self.channels[c].inst_or_wave = 1;
            self.channels[c].instrument = 2; // invalid pointer, never dereferenced
        } else {
            // Instruments
            let mut inst = 0;
            let mut adsr = self.channels[c].adsr;
            let v = self.get_instrument(c, inst_id, &mut inst, &mut adsr);
            self.channels[c].instrument = inst;
            self.channels[c].adsr = adsr;
            self.channels[c].inst_or_wave = v as i16;
            if v == 0 {
                self.channels[c].has_instrument = false;
                return;
            }
        }
        self.channels[c].has_instrument = true;
    }

    /// `AudioSeq_SequenceChannelSetVolume`.
    pub fn sequence_channel_set_volume(&mut self, c: ChanId, volume: u8) {
        self.channels[c].volume = volume as i32 as f32 / 127.0f32;
    }

    /// The font a channel's 0xC6/0xEB picks: `cmd` counts back from the sequence's last font.
    fn channel_font_from_seq(&self, p: usize, cmd: u8) -> u8 {
        let sp = &self.seq_players[p];
        if sp.default_font != 0xFF {
            let idx = self.seq_font_table_index(sp.seq_id as i32);
            let low_bits = self.seq_font_table_byte(idx) as u32;
            self.seq_font_table_byte((idx + low_bits).wrapping_sub(cmd as u32))
        } else {
            cmd
        }
    }

    /// `AudioSeq_SequenceChannelProcessScript`.
    pub fn sequence_channel_process_script(&mut self, c: ChanId) {
        'exit_loop: {
            if self.channels[c].stop_script {
                break 'exit_loop;
            }
            let p = self.channels[c].seq_player.expect("a player's channel");
            if self.seq_players[p].muted && (self.channels[c].mute_behavior & MUTE_BEHAVIOR_STOP_SCRIPT) != 0 {
                return;
            }
            if self.channels[c].delay >= 2 {
                self.channels[c].delay -= 1;
                break 'exit_loop;
            }
            let s = Script::Channel(c);
            loop {
                let seq_data = self.seq_players[p].seq_data;
                let cmd = self.script_read_u8(s);
                if cmd >= 0xB0 {
                    let mut high_bits = SEQ_INSTRUCTION_ARGS_TABLE[(cmd - 0xB0) as usize];
                    let low_bits = high_bits & 3;
                    let mut cmd_args = [0u32; 3];
                    // read in arguments for the instruction
                    for a in cmd_args.iter_mut().take(low_bits as usize) {
                        *a = if high_bits & 0x80 == 0 { self.script_read_u8(s) as u32 } else { self.script_read_s16(s) as i32 as u32 };
                        high_bits <<= 1;
                    }
                    // Control Flow Commands
                    if cmd >= 0xF2 {
                        let delay = self.handle_script_flow_control(seq_data, s, cmd, cmd_args[0] as u16);
                        if delay != 0 {
                            if delay == PROCESS_SCRIPT_END {
                                self.sequence_channel_disable(c);
                            } else {
                                self.channels[c].delay = delay as u16;
                            }
                            break;
                        }
                        continue;
                    }
                    let value = self.channels[c].script_state.value();
                    match cmd {
                        0xEA => {
                            self.channels[c].stop_script = true;
                            break 'exit_loop;
                        }
                        0xF1 => {
                            let pool = self.channels[c].note_pool;
                            self.note_pool_clear(pool);
                            self.note_pool_fill(pool, cmd_args[0] as u8 as i32);
                        }
                        0xF0 => {
                            let pool = self.channels[c].note_pool;
                            self.note_pool_clear(pool);
                        }
                        0xC2 => self.channels[c].dyn_table = seq_data.wrapping_add(cmd_args[0] as u16 as u32),
                        0xC5 => {
                            if value != -1 {
                                let data = self.channels[c].dyn_table.wrapping_add((value as i32 * 2) as u32);
                                let v = ((self.ram.u8(data) as u32) << 8) + self.ram.u8(data + 1) as u32;
                                self.channels[c].dyn_table = seq_data.wrapping_add(v as u16 as u32);
                            }
                        }
                        0xEB | 0xC1 => {
                            let mut arg = cmd_args[0];
                            if cmd == 0xEB {
                                let font = self.channel_font_from_seq(p, cmd_args[0] as u8);
                                if self.search_caches(FONT_TABLE, CACHE_EITHER, font as i32) != 0 {
                                    self.channels[c].font_id = font;
                                }
                                arg = cmd_args[1];
                            }
                            self.set_instrument(c, arg as u8);
                        }
                        0xC3 => self.channels[c].large_notes = false,
                        0xC4 => self.channels[c].large_notes = true,
                        0xDF => {
                            self.sequence_channel_set_volume(c, cmd_args[0] as u8);
                            self.channels[c].changes |= CHANGES_VOLUME;
                        }
                        0xE0 => {
                            self.channels[c].volume_scale = (cmd_args[0] as u8) as i32 as f32 / 128.0f32;
                            self.channels[c].changes |= CHANGES_VOLUME;
                        }
                        0xDE => {
                            self.channels[c].freq_scale = (cmd_args[0] as u16) as i32 as f32 / 32768.0f32;
                            self.channels[c].changes |= CHANGES_FREQ_SCALE;
                        }
                        0xD3 => {
                            let i = (cmd_args[0] as u8).wrapping_add(0x80);
                            self.channels[c].freq_scale = self.tables.bend_pitch_one_octave_frequencies[i as usize];
                            self.channels[c].changes |= CHANGES_FREQ_SCALE;
                        }
                        0xEE => {
                            let i = (cmd_args[0] as u8).wrapping_add(0x80);
                            self.channels[c].freq_scale = self.tables.bend_pitch_two_semitones_frequencies[i as usize];
                            self.channels[c].changes |= CHANGES_FREQ_SCALE;
                        }
                        0xDD => {
                            self.channels[c].new_pan = cmd_args[0] as u8;
                            self.channels[c].changes |= CHANGES_PAN;
                        }
                        0xDC => {
                            self.channels[c].pan_channel_weight = cmd_args[0] as u8;
                            self.channels[c].changes |= CHANGES_PAN;
                        }
                        0xDB => self.channels[c].transposition = cmd_args[0] as u8 as i8 as i16,
                        0xDA => self.channels[c].adsr.envelope = seq_data.wrapping_add(cmd_args[0] as u16 as u32),
                        0xD9 => self.channels[c].adsr.decay_index = cmd_args[0] as u8,
                        0xD8 => {
                            let ch = &mut self.channels[c];
                            ch.vibrato_extent_target = (cmd_args[0] as u8 as u16) * 8;
                            ch.vibrato_extent_start = 0;
                            ch.vibrato_extent_change_delay = 0;
                        }
                        0xD7 => {
                            let ch = &mut self.channels[c];
                            ch.vibrato_rate_change_delay = 0;
                            ch.vibrato_rate_target = (cmd_args[0] as u8 as u16) * 32;
                            ch.vibrato_rate_start = (cmd_args[0] as u8 as u16) * 32;
                        }
                        0xE2 => {
                            let ch = &mut self.channels[c];
                            ch.vibrato_extent_start = (cmd_args[0] as u8 as u16) * 8;
                            ch.vibrato_extent_target = (cmd_args[1] as u8 as u16) * 8;
                            ch.vibrato_extent_change_delay = (cmd_args[2] as u8 as u16) * 16;
                        }
                        0xE1 => {
                            let ch = &mut self.channels[c];
                            ch.vibrato_rate_start = (cmd_args[0] as u8 as u16) * 32;
                            ch.vibrato_rate_target = (cmd_args[1] as u8 as u16) * 32;
                            ch.vibrato_rate_change_delay = (cmd_args[2] as u8 as u16) * 16;
                        }
                        0xE3 => self.channels[c].vibrato_delay = (cmd_args[0] as u8 as u16) * 16,
                        0xD4 => self.channels[c].reverb = cmd_args[0] as u8,
                        0xC6 => {
                            let font = self.channel_font_from_seq(p, cmd_args[0] as u8);
                            if self.search_caches(FONT_TABLE, CACHE_EITHER, font as i32) != 0 {
                                self.channels[c].font_id = font;
                            }
                        }
                        0xC7 => {
                            let at = seq_data.wrapping_add(cmd_args[1] as u16 as u32);
                            self.ram.set_u8(at, (value as u8).wrapping_add(cmd_args[0] as u8));
                        }
                        0xC8 | 0xCC | 0xC9 => {
                            let a = cmd_args[0] as u8 as i8;
                            let v = match cmd {
                                0xC8 => value.wrapping_sub(a),
                                0xCC => a,
                                _ => value & a,
                            };
                            self.channels[c].script_state.set_value(v);
                        }
                        0xCD => {
                            let other = self.seq_players[p].channels[cmd_args[0] as u8 as usize & 0xF];
                            self.sequence_channel_disable(other);
                        }
                        0xCA => {
                            self.channels[c].mute_behavior = cmd_args[0] as u8;
                            self.channels[c].changes |= CHANGES_VOLUME;
                        }
                        0xCB => {
                            let at = seq_data.wrapping_add((cmd_args[0] as u16 as u32).wrapping_add(value as i32 as u32));
                            self.channels[c].script_state.set_value(self.ram.u8(at) as i8);
                        }
                        0xCE => self.channels[c].unk_22 = cmd_args[0] as u16,
                        0xCF => {
                            let at = seq_data.wrapping_add(cmd_args[0] as u16 as u32);
                            let v = self.channels[c].unk_22;
                            self.ram.set_u8(at, (v >> 8) as u8);
                            self.ram.set_u8(at + 1, v as u8);
                        }
                        0xD0 => {
                            let v = cmd_args[0] as u8;
                            self.channels[c].stereo_headset_effects = v & 0x80 != 0;
                            self.channels[c].stereo = v & 0x7F;
                        }
                        0xD1 => self.channels[c].note_alloc_policy = cmd_args[0] as u8,
                        0xD2 => self.channels[c].adsr.sustain = cmd_args[0] as u8,
                        0xE5 => self.channels[c].reverb_index = cmd_args[0] as u8,
                        0xE4 => {
                            if value != -1 {
                                let data = self.channels[c].dyn_table.wrapping_add((value as i32 * 2) as u32);
                                // @bug (game): Missing a stack depth check here
                                let st = &mut self.channels[c].script_state;
                                let d = st.depth();
                                let pc = st.pc();
                                st.set_stack(d as i32, pc);
                                st.set_depth(d.wrapping_add(1));
                                let v = ((self.ram.u8(data) as u32) << 8) + self.ram.u8(data + 1) as u32;
                                self.channels[c].script_state.set_pc(seq_data.wrapping_add(v as u16 as u32));
                            }
                        }
                        0xE6 => self.channels[c].book_offset = cmd_args[0] as u8,
                        0xE7 => {
                            let mut data = seq_data.wrapping_add(cmd_args[0] as u16 as u32);
                            self.channels[c].mute_behavior = self.ram.u8(data);
                            data += 3;
                            self.channels[c].note_alloc_policy = self.ram.u8(data - 2);
                            let prio = self.ram.u8(data - 1);
                            self.set_channel_priorities(c, prio);
                            self.channels[c].transposition = self.ram.u8(data) as i8 as i16;
                            data += 4;
                            let ch = &mut self.channels[c];
                            ch.new_pan = self.ram.u8(data - 3);
                            ch.pan_channel_weight = self.ram.u8(data - 2);
                            ch.reverb = self.ram.u8(data - 1);
                            ch.reverb_index = self.ram.u8(data);
                            // @bug (game): Not marking reverb state as changed
                            ch.changes |= CHANGES_PAN;
                        }
                        0xE8 => {
                            self.channels[c].mute_behavior = cmd_args[0] as u8;
                            self.channels[c].note_alloc_policy = cmd_args[1] as u8;
                            self.set_channel_priorities(c, cmd_args[2] as u8);
                            let t = self.script_read_u8(s) as i8 as i16;
                            let pan = self.script_read_u8(s);
                            let w = self.script_read_u8(s);
                            let r = self.script_read_u8(s);
                            let ri = self.script_read_u8(s);
                            let ch = &mut self.channels[c];
                            ch.transposition = t;
                            ch.new_pan = pan;
                            ch.pan_channel_weight = w;
                            ch.reverb = r;
                            ch.reverb_index = ri;
                            // @bug (game): Not marking reverb state as changed
                            ch.changes |= CHANGES_PAN;
                        }
                        0xEC => {
                            let ch = &mut self.channels[c];
                            ch.vibrato_extent_target = 0;
                            ch.vibrato_extent_start = 0;
                            ch.vibrato_extent_change_delay = 0;
                            ch.vibrato_rate_target = 0;
                            ch.vibrato_rate_start = 0;
                            ch.vibrato_rate_change_delay = 0;
                            ch.filter = 0;
                            ch.gain = 0;
                            ch.adsr.sustain = 0;
                            ch.velocity_random_variance = 0;
                            ch.gate_time_random_variance = 0;
                            ch.comb_filter_size = 0;
                            ch.comb_filter_gain = 0;
                            ch.book_offset = 0;
                            ch.freq_scale = 1.0;
                        }
                        0xE9 => self.set_channel_priorities(c, cmd_args[0] as u8),
                        0xED => self.channels[c].gain = cmd_args[0] as u8,
                        0xB0 => self.channels[c].filter = seq_data.wrapping_add(cmd_args[0] as u16 as u32),
                        0xB1 => self.channels[c].filter = 0,
                        0xB3 => {
                            let v = cmd_args[0] as u8;
                            let filter = self.channels[c].filter;
                            if filter != 0 {
                                // LowPassCutoff, HighPassCutoff
                                self.load_filter(filter, ((v >> 4) & 0xF) as i32, (v & 0xF) as i32);
                            }
                        }
                        0xB2 => {
                            let at = seq_data.wrapping_add((cmd_args[0] as u16 as u32).wrapping_add((value as i32 * 2) as u32));
                            self.channels[c].unk_22 = self.ram.u16(at);
                        }
                        0xB4 => self.channels[c].dyn_table = seq_data.wrapping_add(self.channels[c].unk_22 as u32),
                        0xB5 => {
                            let at = self.channels[c].dyn_table.wrapping_add((value as i32 * 2) as u32);
                            self.channels[c].unk_22 = self.ram.u16(at);
                        }
                        0xB6 => {
                            let at = self.channels[c].dyn_table.wrapping_add(value as i32 as u32);
                            self.channels[c].script_state.set_value(self.ram.u8(at) as i8);
                        }
                        0xB7 => {
                            let r = self.audio_random;
                            self.channels[c].unk_22 = if cmd_args[0] == 0 { (r & 0xFFFF) as u16 } else { (r % cmd_args[0]) as u16 };
                        }
                        0xB8 => {
                            let r = self.audio_random;
                            let v = if cmd_args[0] == 0 { r & 0xFFFF } else { r % cmd_args[0] };
                            self.channels[c].script_state.set_value(v as u8 as i8);
                        }
                        0xBD => {
                            let temp2 = self.next_random();
                            let a0 = cmd_args[0];
                            let mut v = if a0 == 0 { (temp2 & 0xFFFF) as u16 } else { (temp2 % a0) as u16 };
                            v = v.wrapping_add(cmd_args[1] as u16);
                            let hi = ((v / 0x100) as i32 + 0x80) as u16;
                            let lo = v % 0x100;
                            self.channels[c].unk_22 = (hi << 8) | lo;
                        }
                        0xB9 => self.channels[c].velocity_random_variance = cmd_args[0] as u8,
                        0xBA => self.channels[c].gate_time_random_variance = cmd_args[0] as u8,
                        0xBB => {
                            self.channels[c].comb_filter_size = cmd_args[0] as u8;
                            self.channels[c].comb_filter_gain = cmd_args[1] as u16;
                        }
                        0xBC => self.channels[c].unk_22 = self.channels[c].unk_22.wrapping_add(cmd_args[0] as u16),
                        _ => {}
                    }
                    continue;
                }

                if cmd >= 0x70 {
                    let mut low_bits = (cmd & 0x7) as usize;
                    if (cmd & 0xF8) != 0x70 && low_bits >= 4 {
                        low_bits = 0;
                    }
                    match cmd & 0xF8 {
                        0x80 => {
                            let v = match self.channels[c].layers[low_bits] {
                                Some(l) => self.sequence_layers[l].finished as i8,
                                None => -1,
                            };
                            self.channels[c].script_state.set_value(v);
                        }
                        0x88 => {
                            let off = self.script_read_s16(s) as u16;
                            if self.seq_channel_set_layer(c, low_bits) == 0 {
                                let l = self.channels[c].layers[low_bits].unwrap();
                                self.sequence_layers[l].script_state.set_pc(seq_data.wrapping_add(off as u32));
                            }
                        }
                        0x90 => self.seq_layer_free(c, low_bits),
                        0x98 => {
                            let value = self.channels[c].script_state.value();
                            if value != -1 && self.seq_channel_set_layer(c, low_bits) != -1 {
                                let data = self.channels[c].dyn_table.wrapping_add((value as i32 * 2) as u32);
                                let v = ((self.ram.u8(data) as u32) << 8) + self.ram.u8(data + 1) as u32;
                                let l = self.channels[c].layers[low_bits].unwrap();
                                self.sequence_layers[l].script_state.set_pc(seq_data.wrapping_add(v as u16 as u32));
                            }
                        }
                        0x70 => {
                            let v = self.channels[c].script_state.value();
                            self.channels[c].sound_script_io[low_bits] = v;
                        }
                        0x78 => {
                            let rel = self.script_read_s16(s) as i32;
                            if self.seq_channel_set_layer(c, low_bits) == 0 {
                                let pc = self.channels[c].script_state.pc();
                                let l = self.channels[c].layers[low_bits].unwrap();
                                self.sequence_layers[l].script_state.set_pc(pc.wrapping_add(rel as u32));
                            }
                        }
                        _ => {}
                    }
                    continue;
                }

                let low_bits = (cmd & 0xF) as usize;
                match cmd & 0xF0 {
                    0x00 => {
                        self.channels[c].delay = low_bits as u16;
                        break 'exit_loop;
                    }
                    0x10 => {
                        let font = self.channels[c].font_id as i32;
                        if low_bits < 8 {
                            self.channels[c].sound_script_io[low_bits] = -1;
                            let v = self.channels[c].script_state.value() as i32;
                            self.slow_load_sample(font, v, IoPort::Channel(c, low_bits));
                        } else {
                            let lb = low_bits - 8;
                            self.channels[c].sound_script_io[lb] = -1;
                            let id = self.channels[c].unk_22 as i32 + 0x100;
                            self.slow_load_sample(font, id, IoPort::Channel(c, lb));
                        }
                    }
                    0x60 => {
                        let v = self.channels[c].sound_script_io.get(low_bits).copied().unwrap_or(-1);
                        self.channels[c].script_state.set_value(v);
                        if low_bits < 2 {
                            self.channels[c].sound_script_io[low_bits] = -1;
                        }
                    }
                    0x50 => {
                        let io = self.channels[c].sound_script_io.get(low_bits).copied().unwrap_or(0);
                        let v = self.channels[c].script_state.value().wrapping_sub(io);
                        self.channels[c].script_state.set_value(v);
                    }
                    0x20 => {
                        let off = self.script_read_s16(s) as u16;
                        self.sequence_channel_enable(p, low_bits as u8, seq_data.wrapping_add(off as u32));
                    }
                    0x30 => {
                        let port = self.script_read_u8(s) as usize;
                        let v = self.channels[c].script_state.value();
                        let other = self.seq_players[p].channels[low_bits];
                        if let Some(io) = self.channels[other].sound_script_io.get_mut(port) {
                            *io = v;
                        }
                    }
                    0x40 => {
                        let port = self.script_read_u8(s) as usize;
                        let other = self.seq_players[p].channels[low_bits];
                        let v = self.channels[other].sound_script_io.get(port).copied().unwrap_or(0);
                        self.channels[c].script_state.set_value(v);
                    }
                    _ => {}
                }
            }
        }
        // exit_loop:
        for i in 0..4 {
            if let Some(l) = self.channels[c].layers[i] {
                self.seq_layer_process_script(l);
            }
        }
    }

    /// `AudioSeq_SequencePlayerProcessSequence`: one update of the player's script and its
    /// channels', when the tempo says a tick is due.
    pub fn sequence_player_process_sequence(&mut self, p: usize) {
        if !self.seq_players[p].enabled {
            return;
        }
        let (seq_id, default_font) = (self.seq_players[p].seq_id as i32, self.seq_players[p].default_font as i32);
        if !self.is_seq_load_complete(seq_id) || !self.is_font_load_complete(default_font) {
            self.sequence_player_disable(p);
            return;
        }
        self.set_seq_load_status(seq_id, LOAD_STATUS_COMPLETE);
        self.set_font_load_status(default_font, LOAD_STATUS_COMPLETE);

        let tie = self.tempo_internal_to_external;
        let sp = &mut self.seq_players[p];
        if sp.muted && (sp.mute_behavior & MUTE_BEHAVIOR_STOP_SCRIPT) != 0 {
            return;
        }
        sp.script_counter = sp.script_counter.wrapping_add(1);
        sp.tempo_acc = sp.tempo_acc.wrapping_add(sp.tempo);
        sp.tempo_acc = sp.tempo_acc.wrapping_add(sp.tempo_change as i16 as u16);
        if (sp.tempo_acc as i32) < tie as i32 {
            return;
        }
        sp.tempo_acc = sp.tempo_acc.wrapping_sub(tie as u16);
        self.stats.seq_ticks[p] += 1;
        let sp = &mut self.seq_players[p];
        if sp.stop_script {
            return;
        }

        if sp.delay > 1 {
            sp.delay -= 1;
        } else {
            sp.recalculate_volume = true;
            let s = Script::Player(p);
            loop {
                let seq_data = self.seq_players[p].seq_data;
                let cmd = self.script_read_u8(s);
                // 0xF2 and above are "flow control" commands, including termination.
                if cmd >= 0xF2 {
                    let arg = self.get_script_control_flow_argument(s, cmd);
                    let delay = self.handle_script_flow_control(seq_data, s, cmd, arg);
                    if delay != 0 {
                        if delay == -1 {
                            self.sequence_player_disable(p);
                        } else {
                            self.seq_players[p].delay = delay as u16;
                        }
                        break;
                    }
                    continue;
                }
                if cmd >= 0xC0 {
                    match cmd {
                        0xF1 => {
                            let pool = self.seq_players[p].note_pool;
                            self.note_pool_clear(pool);
                            let n = self.script_read_u8(s);
                            self.note_pool_fill(pool, n as i32);
                        }
                        0xF0 => {
                            let pool = self.seq_players[p].note_pool;
                            self.note_pool_clear(pool);
                        }
                        0xDF | 0xDE => {
                            if cmd == 0xDF {
                                self.seq_players[p].transposition = 0;
                            }
                            let v = self.script_read_u8(s) as i8 as i16;
                            let t = &mut self.seq_players[p].transposition;
                            *t = t.wrapping_add(v);
                        }
                        0xDD => {
                            let v = self.script_read_u8(s) as u16 * SEQTICKS_PER_BEAT as u16;
                            let sp = &mut self.seq_players[p];
                            sp.tempo = v;
                            if sp.tempo as i32 > tie as i32 {
                                sp.tempo = tie as u16;
                            }
                            if (sp.tempo as i16) <= 0 {
                                sp.tempo = 1;
                            }
                        }
                        0xDC => {
                            let v = self.script_read_u8(s) as i8 as i16 * SEQTICKS_PER_BEAT as i16;
                            self.seq_players[p].tempo_change = v as u16;
                        }
                        0xDA => {
                            let c = self.script_read_u8(s);
                            let temp = self.script_read_s16(s) as u16;
                            let sp = &mut self.seq_players[p];
                            match c {
                                0 | 1 => {
                                    if sp.state != 2 {
                                        sp.fade_timer_unk_eu = temp;
                                        sp.state = c;
                                    }
                                }
                                2 => {
                                    sp.fade_timer = temp;
                                    sp.state = c;
                                    sp.fade_velocity = (0.0f32 - sp.fade_volume) / sp.fade_timer as i32 as f32;
                                }
                                _ => {}
                            }
                        }
                        0xDB => {
                            let value = self.script_read_u8(s) as i32;
                            let sp = &mut self.seq_players[p];
                            match sp.state {
                                1 | 0 => {
                                    if sp.state == 1 {
                                        sp.state = 0;
                                        sp.fade_volume = 0.0;
                                    }
                                    sp.fade_timer = sp.fade_timer_unk_eu;
                                    if sp.fade_timer_unk_eu != 0 {
                                        sp.fade_velocity = ((value as f32 / 127.0f32) - sp.fade_volume) / sp.fade_timer as i32 as f32;
                                    } else {
                                        sp.fade_volume = value as f32 / 127.0f32;
                                    }
                                }
                                _ => {}
                            }
                        }
                        0xD9 => {
                            let v = self.script_read_u8(s) as i8;
                            self.seq_players[p].fade_volume_scale = v as f32 / 127.0f32;
                        }
                        0xD7 => {
                            let temp = self.script_read_s16(s) as u16;
                            self.sequence_player_setup_channels(p, temp);
                        }
                        0xD6 => {
                            self.script_read_s16(s);
                        }
                        0xD5 => {
                            let v = self.script_read_u8(s) as i8;
                            self.seq_players[p].mute_volume_scale = v as f32 / 127.0f32;
                        }
                        0xD4 => self.seq_players[p].muted = true,
                        0xD3 => {
                            let v = self.script_read_u8(s);
                            self.seq_players[p].mute_behavior = v;
                        }
                        0xD1 | 0xD2 => {
                            let temp = self.script_read_s16(s) as u16;
                            let data3 = seq_data.wrapping_add(temp as u32);
                            if cmd == 0xD2 {
                                self.seq_players[p].short_note_velocity_table = data3;
                            } else {
                                self.seq_players[p].short_note_gate_time_table = data3;
                            }
                        }
                        0xD0 => {
                            let v = self.script_read_u8(s);
                            self.seq_players[p].note_alloc_policy = v;
                        }
                        0xCE => {
                            let c = self.script_read_u8(s);
                            let r = self.audio_random >> 2;
                            let v = if c == 0 { r & 0xFF } else { r % c as u32 };
                            self.seq_players[p].script_state.set_value(v as u8 as i8);
                        }
                        0xCD => {
                            let temp = self.script_read_s16(s) as u16;
                            let st = self.seq_players[p].script_state;
                            if st.value() != -1 && st.depth() != 3 {
                                let data = seq_data.wrapping_add((temp as u32).wrapping_add(((st.value() as i32) << 1) as u32));
                                let st = &mut self.seq_players[p].script_state;
                                let d = st.depth();
                                let pc = st.pc();
                                st.set_stack(d as i32, pc);
                                st.set_depth(d.wrapping_add(1));
                                let t = ((self.ram.u8(data) as u32) << 8) + self.ram.u8(data + 1) as u32;
                                self.seq_players[p].script_state.set_pc(seq_data.wrapping_add(t as u16 as u32));
                            }
                        }
                        0xCC => {
                            let v = self.script_read_u8(s) as i8;
                            self.seq_players[p].script_state.set_value(v);
                        }
                        0xC9 => {
                            let v = self.script_read_u8(s) as i8;
                            let st = &mut self.seq_players[p].script_state;
                            st.set_value(st.value() & v);
                        }
                        0xC8 => {
                            let v = self.script_read_u8(s) as i8;
                            let st = &mut self.seq_players[p].script_state;
                            st.set_value(st.value().wrapping_sub(v));
                        }
                        0xC7 => {
                            let c = self.script_read_u8(s);
                            let temp = self.script_read_s16(s) as u16;
                            let v = (self.seq_players[p].script_state.value() as u8).wrapping_add(c);
                            self.ram.set_u8(seq_data.wrapping_add(temp as u32), v);
                        }
                        0xC6 => {
                            self.seq_players[p].stop_script = true;
                            return;
                        }
                        0xC5 => {
                            let v = self.script_read_s16(s) as u16;
                            self.seq_players[p].script_counter = v as u32;
                        }
                        0xEF => {
                            self.script_read_s16(s);
                            self.script_read_u8(s);
                        }
                        0xC4 => {
                            let mut c = self.script_read_u8(s);
                            if c == 0xFF {
                                c = self.seq_players[p].player_idx as u8;
                            }
                            let low = self.script_read_u8(s);
                            self.sync_init_seq_player(c as i32, low as i32, 0);
                            if c == self.seq_players[p].player_idx as u8 {
                                return;
                            }
                        }
                        _ => {}
                    }
                    continue;
                }

                let low_bits = (cmd & 0x0F) as usize;
                match cmd & 0xF0 {
                    0x00 => {
                        let c = self.seq_players[p].channels[low_bits];
                        let v = (self.channels[c].enabled as u8 ^ 1) as i8;
                        self.seq_players[p].script_state.set_value(v);
                    }
                    0x50 => {
                        let io = self.seq_players[p].sound_script_io.get(low_bits).copied().unwrap_or(0);
                        let st = &mut self.seq_players[p].script_state;
                        st.set_value(st.value().wrapping_sub(io));
                    }
                    0x70 => {
                        let v = self.seq_players[p].script_state.value();
                        if let Some(io) = self.seq_players[p].sound_script_io.get_mut(low_bits) {
                            *io = v;
                        }
                    }
                    0x80 => {
                        let v = self.seq_players[p].sound_script_io.get(low_bits).copied().unwrap_or(-1);
                        self.seq_players[p].script_state.set_value(v);
                        if low_bits < 2 {
                            self.seq_players[p].sound_script_io[low_bits] = -1;
                        }
                    }
                    0x40 => {
                        let c = self.seq_players[p].channels[low_bits];
                        self.sequence_channel_disable(c);
                    }
                    0x90 => {
                        let temp = self.script_read_s16(s) as u16;
                        self.sequence_channel_enable(p, low_bits as u8, seq_data.wrapping_add(temp as u32));
                    }
                    0xA0 => {
                        let temp_s = self.script_read_s16(s) as i32;
                        let pc = self.seq_players[p].script_state.pc();
                        self.sequence_channel_enable(p, low_bits as u8, pc.wrapping_add(temp_s as u32));
                    }
                    0xB0 => {
                        let c = self.script_read_u8(s);
                        let temp = self.script_read_s16(s) as u16;
                        let data2 = seq_data.wrapping_add(temp as u32);
                        self.slow_load_seq(c as i32, data2, IoPort::Player(p, low_bits & 7));
                    }
                    0x60 => {
                        let c = self.script_read_u8(s);
                        let temp = self.script_read_u8(s);
                        self.script_load(c as i32, temp as i32, IoPort::Player(p, low_bits & 7));
                    }
                    _ => {}
                }
            }
        }

        for i in 0..SEQ_NUM_CHANNELS {
            let c = self.seq_players[p].channels[i];
            if self.channels[c].enabled {
                self.sequence_channel_process_script(c);
            }
        }
    }

    /// `AudioSeq_ProcessSequences`: one update (`arg0` counts down to 0 over the frame).
    pub fn process_sequences(&mut self, arg0: i32) {
        self.note_sub_eu_offset = (self.audio_buffer_parameters.updates_per_frame as i32 - arg0 - 1) * self.num_notes;
        for i in 0..self.audio_buffer_parameters.num_sequence_players as usize {
            if self.seq_players[i].enabled {
                self.sequence_player_process_sequence(i);
                self.sequence_player_process_sound(i);
            }
        }
        self.process_notes();
    }

    /// `AudioSeq_SkipForwardSequence`.
    pub fn skip_forward_sequence(&mut self, p: usize) {
        while self.seq_players[p].skip_ticks > 0 {
            self.sequence_player_process_sequence(p);
            self.sequence_player_process_sound(p);
            self.seq_players[p].skip_ticks -= 1;
        }
    }

    /// `AudioSeq_ResetSequencePlayer`.
    pub fn reset_sequence_player(&mut self, p: usize) {
        self.sequence_player_disable(p);
        let (vel, gate) = (self.statics.default_short_note_velocity_table, self.statics.default_short_note_gate_time_table);
        let sp = &mut self.seq_players[p];
        sp.stop_script = false;
        sp.delay = 0;
        sp.state = 1;
        sp.fade_timer = 0;
        sp.fade_timer_unk_eu = 0;
        sp.tempo_acc = 0;
        sp.tempo = 120 * SEQTICKS_PER_BEAT as u16; // 120 BPM
        sp.tempo_change = 0;
        sp.transposition = 0;
        sp.note_alloc_policy = 0;
        sp.short_note_velocity_table = vel;
        sp.short_note_gate_time_table = gate;
        sp.script_counter = 0;
        sp.fade_volume = 1.0;
        sp.fade_velocity = 0.0;
        sp.volume = 0.0;
        sp.mute_volume_scale = 0.5;
        for i in 0..SEQ_NUM_CHANNELS {
            let c = self.seq_players[p].channels[i];
            self.init_sequence_channel(c);
        }
    }

    /// `AudioSeq_InitSequencePlayerChannels`: the player's 16 channels, on the heap.
    pub fn init_sequence_player_channels(&mut self, p: usize) {
        for i in 0..SEQ_NUM_CHANNELS {
            let a = crate::heap::alloc_zeroed(&mut self.ram, &mut self.misc_pool, SIZEOF_SEQUENCE_CHANNEL);
            if a == 0 {
                self.seq_players[p].channels[i] = CHANNEL_NONE;
            } else {
                let pool = self.new_note_pool();
                let id = self.channels.len();
                self.channels.push(SequenceChannel { seq_player: Some(p), enabled: false, layers: [None; 4], note_pool: pool, ..Default::default() });
                self.seq_players[p].channels[i] = id;
            }
            let c = self.seq_players[p].channels[i];
            self.init_sequence_channel(c);
        }
    }

    /// `AudioSeq_InitSequencePlayer`.
    pub fn init_sequence_player(&mut self, p: usize) {
        let pool = self.new_note_pool();
        let sp = &mut self.seq_players[p];
        sp.channels = [CHANNEL_NONE; 16];
        sp.enabled = false;
        sp.muted = false;
        sp.font_dma_in_progress = false;
        sp.seq_dma_in_progress = false;
        sp.apply_bend = false;
        sp.sound_script_io = [-1; 8];
        sp.mute_behavior = MUTE_BEHAVIOR_SOFTEN | MUTE_BEHAVIOR_STOP_NOTES;
        sp.fade_volume_scale = 1.0;
        sp.bend = 1.0;
        sp.note_pool = pool;
        self.init_note_lists(pool);
        self.reset_sequence_player(p);
    }

    /// `AudioSeq_InitSequencePlayers`.
    pub fn init_sequence_players(&mut self) {
        self.init_layer_freelist();
        for l in self.sequence_layers.iter_mut() {
            l.channel = None;
            l.enabled = false;
        }
        for p in 0..4 {
            self.init_sequence_player(p);
        }
    }
}
