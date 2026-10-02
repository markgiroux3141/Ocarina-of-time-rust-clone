//! `synthesis.c`: the RSP command list of one audio frame. `AudioSynth_Update` runs the
//! frame's sequence updates, then for each update decodes, resamples and mixes every enabled
//! note into DMEM's dry and wet channels, adds and saves the reverbs, and interleaves the
//! result into the AI buffer. The commands go to `self.cmds`; `rsp.rs` runs them.

use crate::context::*;
use crate::rsp::abi::*;
use crate::rsp::{A_CONTINUE, A_ENVMIXER, A_INIT, A_LOOP};

// DMEM Addresses for the RSP
const DMEM_TEMP: u32 = 0x3C0;
const DMEM_UNCOMPRESSED_NOTE: u32 = 0x580;
const DMEM_HAAS_TEMP: u32 = 0x5C0;
const DMEM_COMB_TEMP: u32 = 0x760; // = DMEM_TEMP + DMEM_2CH_SIZE + a bit more
const DMEM_COMPRESSED_ADPCM_DATA: u32 = 0x940; // = DMEM_LEFT_CH
const DMEM_LEFT_CH: u32 = 0x940;
const DMEM_RIGHT_CH: u32 = 0xAE0;
const DMEM_WET_TEMP: u32 = 0x3E0;
const DMEM_WET_SCRATCH: u32 = 0x720; // = DMEM_WET_TEMP + DMEM_2CH_SIZE
const DMEM_WET_LEFT_CH: u32 = 0xC80;
const DMEM_WET_RIGHT_CH: u32 = 0xE20; // = DMEM_WET_LEFT_CH + DMEM_1CH_SIZE

const D1CH: u32 = DMEM_1CH_SIZE as u32;
const D2CH: u32 = DMEM_2CH_SIZE as u32;
const SS: i32 = SAMPLE_SIZE;
const SPF: i32 = SAMPLES_PER_FRAME;

/// `HaasEffectDelaySide`.
const HAAS_EFFECT_DELAY_NONE: i32 = 0;
const HAAS_EFFECT_DELAY_LEFT: i32 = 1;
const HAAS_EFFECT_DELAY_RIGHT: i32 = 2;

const fn mk_cmd(b0: u32, b1: u32, b2: u32, b3: u32) -> u32 {
    ((b0 & 0xFF) << 24) | ((b1 & 0xFF) << 16) | ((b2 & 0xFF) << 8) | (b3 & 0xFF)
}

/// `sEnvMixerOp`.
const S_ENV_MIXER_OP: u32 = (A_ENVMIXER & 0xFF) << 24;
/// Store the left dry channel in a temp space to be delayed to produce the haas effect.
const S_ENV_MIXER_LEFT_HAAS_DMEM_DESTS: u32 = mk_cmd(DMEM_HAAS_TEMP >> 4, DMEM_RIGHT_CH >> 4, DMEM_WET_LEFT_CH >> 4, DMEM_WET_RIGHT_CH >> 4);
/// Store the right dry channel in a temp space to be delayed to produce the haas effect.
const S_ENV_MIXER_RIGHT_HAAS_DMEM_DESTS: u32 = mk_cmd(DMEM_LEFT_CH >> 4, DMEM_HAAS_TEMP >> 4, DMEM_WET_LEFT_CH >> 4, DMEM_WET_RIGHT_CH >> 4);
const S_ENV_MIXER_DEFAULT_DMEM_DESTS: u32 = mk_cmd(DMEM_LEFT_CH >> 4, DMEM_RIGHT_CH >> 4, DMEM_WET_LEFT_CH >> 4, DMEM_WET_RIGHT_CH >> 4);

/// `sNumSamplesPerWavePeriod`.
const S_NUM_SAMPLES_PER_WAVE_PERIOD: [i32; 4] = [WAVE_SAMPLE_COUNT, WAVE_SAMPLE_COUNT / 2, WAVE_SAMPLE_COUNT / 4, WAVE_SAMPLE_COUNT / 8];

const fn align16i(v: i32) -> i32 {
    (v + 0xF) & !0xF
}

impl AudioContext {
    fn cmd(&mut self, c: crate::rsp::Acmd) {
        self.cmds.push(c);
    }

    /// `AudioSynth_InitNextRingBuf`.
    pub fn init_next_ring_buf(&mut self, chunk_len: i32, update_index: usize, reverb_index: usize) {
        let r = self.synthesis_reverbs[reverb_index];
        if r.downsample_rate >= 2 && r.frames_to_ignore == 0 {
            let item = r.items[r.cur_frame as usize][update_index];
            let mut j = 0;
            for i in 0..(item.length_a as i32 / SS) {
                let l = self.ram.s16(item.to_downsample_left + 2 * j as u32);
                let rr = self.ram.s16(item.to_downsample_right + 2 * j as u32);
                self.ram.set_s16(r.left_ring_buf + 2 * (item.start_pos + i) as u32, l);
                self.ram.set_s16(r.right_ring_buf + 2 * (item.start_pos + i) as u32, rr);
                j += r.downsample_rate as i32;
            }
            for i in 0..(item.length_b as i32 / SS) {
                let l = self.ram.s16(item.to_downsample_left + 2 * j as u32);
                let rr = self.ram.s16(item.to_downsample_right + 2 * j as u32);
                self.ram.set_s16(r.left_ring_buf + 2 * i as u32, l);
                self.ram.set_s16(r.right_ring_buf + 2 * i as u32, rr);
                j += r.downsample_rate as i32;
            }
        }

        let r = &mut self.synthesis_reverbs[reverb_index];
        let cur = r.cur_frame as usize;
        let num_samples = chunk_len / r.downsample_rate as i32;
        let extra_samples = (num_samples + r.next_ring_buf_pos) - r.buf_size_per_chan;
        let temp_a0_2 = r.next_ring_buf_pos;
        let item = &mut r.items[cur][update_index];
        if extra_samples < 0 {
            item.length_a = (num_samples * SS) as i16;
            item.length_b = 0;
            item.start_pos = r.next_ring_buf_pos;
            r.next_ring_buf_pos += num_samples;
        } else {
            // End of the buffer is reached. Loop back around
            item.length_a = ((num_samples - extra_samples) * SS) as i16;
            item.length_b = (extra_samples * SS) as i16;
            item.start_pos = r.next_ring_buf_pos;
            r.next_ring_buf_pos = extra_samples;
        }
        item.num_samples_after_downsampling = num_samples as i16;
        item.chunk_len = chunk_len as i16;

        if r.unk_14 != 0 {
            let mut temp_a0_4 = r.unk_14 as i32 + temp_a0_2;
            if temp_a0_4 >= r.buf_size_per_chan {
                temp_a0_4 -= r.buf_size_per_chan;
            }
            let num_samples = chunk_len / r.downsample_rate as i32;
            let extra_samples = (temp_a0_4 + num_samples) - r.buf_size_per_chan;
            let item = &mut r.items2[cur][update_index];
            if extra_samples < 0 {
                item.length_a = (num_samples * SS) as i16;
                item.length_b = 0;
                item.start_pos = temp_a0_4;
            } else {
                // End of the buffer is reached. Loop back around
                item.length_a = ((num_samples - extra_samples) * SS) as i16;
                item.length_b = (extra_samples * SS) as i16;
                item.start_pos = temp_a0_4;
            }
            item.num_samples_after_downsampling = num_samples as i16;
            item.chunk_len = chunk_len as i16;
        }
    }

    /// `func_800DB03C`.
    fn func_800db03c(&mut self, update_index: usize) {
        let base = self.num_notes as usize * update_index;
        for i in 0..self.num_notes as usize {
            if self.notes[i].note_sub_eu.enabled {
                self.notes[i].note_sub_eu.needs_init = false;
            } else {
                self.note_subs_eu[base + i].enabled = false;
            }
            self.notes[i].note_sub_eu.harmonic_index_cur_and_prev = 0;
        }
    }

    /// `AudioSynth_Update`: the frame's command list, `ai_buf_len` stereo samples into
    /// `ai_start`.
    pub fn audio_synth_update(&mut self, ai_start: u32, mut ai_buf_len: i32) {
        self.cmds.clear();
        let upf = self.audio_buffer_parameters.updates_per_frame as i32;
        for i in (1..=upf).rev() {
            self.process_sequences(i - 1);
            self.func_800db03c((upf - i) as usize);
        }

        let mut ai_buf_p = ai_start;
        self.cur_loaded_book = 0;

        for i in (1..=upf).rev() {
            let abp = self.audio_buffer_parameters;
            let chunk_len = if i == 1 {
                ai_buf_len
            } else if (ai_buf_len / i) >= abp.samples_per_update_max as i32 {
                abp.samples_per_update_max as i32
            } else if abp.samples_per_update_min as i32 >= (ai_buf_len / i) {
                abp.samples_per_update_min as i32
            } else {
                abp.samples_per_update as i32
            };
            for j in 0..self.num_synthesis_reverbs as usize {
                if self.synthesis_reverbs[j].use_reverb != 0 {
                    self.init_next_ring_buf(chunk_len, (upf - i) as usize, j);
                }
            }
            self.do_one_audio_update(ai_buf_p, chunk_len, (upf - i) as usize);
            ai_buf_len -= chunk_len;
            ai_buf_p += (2 * chunk_len * SS) as u32;
        }

        for j in 0..self.num_synthesis_reverbs as usize {
            let r = &mut self.synthesis_reverbs[j];
            if r.frames_to_ignore != 0 {
                r.frames_to_ignore -= 1;
            }
            r.cur_frame ^= 1;
        }
    }

    /// `func_800DB2C0`.
    fn func_800db2c0(&mut self, update_index: usize, note_index: usize) {
        for i in update_index + 1..self.audio_buffer_parameters.updates_per_frame as usize {
            let s = &mut self.note_subs_eu[self.num_notes as usize * i + note_index];
            if !s.needs_init {
                s.enabled = false;
            } else {
                break;
            }
        }
    }

    /// `AudioSynth_LoadRingBuffer1AtTemp`.
    fn load_ring_buffer1_at_temp(&mut self, ri: usize, update_index: usize) {
        let r = self.synthesis_reverbs[ri];
        let item = r.items[r.cur_frame as usize][update_index];
        self.load_ring_buffer_part(DMEM_WET_TEMP, item.start_pos as u32, item.length_a as i32, ri);
        if item.length_b != 0 {
            // Ring buffer wrapped
            self.load_ring_buffer_part(DMEM_WET_TEMP + item.length_a as u32, 0, item.length_b as i32, ri);
        }
    }

    /// `AudioSynth_SaveRingBuffer1AtTemp`.
    fn save_ring_buffer1_at_temp(&mut self, ri: usize, update_index: usize) {
        let r = self.synthesis_reverbs[ri];
        let item = r.items[r.cur_frame as usize][update_index];
        self.save_ring_buffer_part(DMEM_WET_TEMP, item.start_pos as u32, item.length_a as i32, ri);
        if item.length_b != 0 {
            // Ring buffer wrapped
            self.save_ring_buffer_part(DMEM_WET_TEMP + item.length_a as u32, 0, item.length_b as i32, ri);
        }
    }

    /// `AudioSynth_LeakReverb`: some of each wet channel into the other.
    fn leak_reverb(&mut self, ri: usize) {
        let r = self.synthesis_reverbs[ri];
        self.cmd(a_dmem_move(DMEM_WET_LEFT_CH, DMEM_WET_SCRATCH, D1CH));
        self.cmd(a_mix(D1CH >> 4, r.leak_rtl as u16 as u32, DMEM_WET_RIGHT_CH, DMEM_WET_LEFT_CH));
        self.cmd(a_mix(D1CH >> 4, r.leak_ltr as u16 as u32, DMEM_WET_SCRATCH, DMEM_WET_RIGHT_CH));
    }

    /// `func_800DB4E4`.
    fn func_800db4e4(&mut self, ai_buf_len: i32, ri: usize, update_index: usize) {
        let r = self.synthesis_reverbs[ri];
        let item = r.items[r.cur_frame as usize][update_index];
        let offset_a = ((item.start_pos & 7) * SS) as i16;
        let offset_b = align16i(offset_a as i32 + item.length_a as i32) as i16;
        self.load_ring_buffer_part(DMEM_WET_TEMP, (item.start_pos - (offset_a as i32 / SS)) as u32, D1CH as i32, ri);
        if item.length_b != 0 {
            // Ring buffer wrapped
            self.load_ring_buffer_part(DMEM_WET_TEMP + offset_b as u32, 0, D1CH as i32 - offset_b as i32, ri);
        }
        self.cmd(a_set_buffer(0, DMEM_WET_TEMP + offset_a as u32, DMEM_WET_LEFT_CH, (ai_buf_len * SS) as u32));
        self.cmd(a_resample(r.resample_flags as u32, r.unk_0e as u32, r.unk_30));
        self.cmd(a_set_buffer(0, DMEM_WET_TEMP + D1CH + offset_a as u32, DMEM_WET_RIGHT_CH, (ai_buf_len * SS) as u32));
        self.cmd(a_resample(r.resample_flags as u32, r.unk_0e as u32, r.unk_34));
    }

    /// `func_800DB680`.
    fn func_800db680(&mut self, ri: usize, update_index: usize) {
        let r = self.synthesis_reverbs[ri];
        let item = r.items[r.cur_frame as usize][update_index];
        self.cmd(a_set_buffer(0, DMEM_WET_LEFT_CH, DMEM_WET_SCRATCH, item.unk_18 as u32 * SS as u32));
        self.cmd(a_resample(r.resample_flags as u32, item.unk_16 as u32, r.unk_38));
        self.save_buffer_offset(DMEM_WET_SCRATCH, item.start_pos as u32, item.length_a as i32, r.left_ring_buf);
        if item.length_b != 0 {
            // Ring buffer wrapped
            self.save_buffer_offset(DMEM_WET_SCRATCH + item.length_a as u32, 0, item.length_b as i32, r.left_ring_buf);
        }
        self.cmd(a_set_buffer(0, DMEM_WET_RIGHT_CH, DMEM_WET_SCRATCH, item.unk_18 as u32 * SS as u32));
        self.cmd(a_resample(r.resample_flags as u32, item.unk_16 as u32, r.unk_3c));
        self.save_buffer_offset(DMEM_WET_SCRATCH, item.start_pos as u32, item.length_a as i32, r.right_ring_buf);
        if item.length_b != 0 {
            // Ring buffer wrapped
            self.save_buffer_offset(DMEM_WET_SCRATCH + item.length_a as u32, 0, item.length_b as i32, r.right_ring_buf);
        }
    }

    /// `func_800DB828`.
    fn func_800db828(&mut self, ai_buf_len: i32, ri: usize, update_index: usize) {
        let cur = self.synthesis_reverbs[ri].cur_frame as usize;
        {
            let item = &mut self.synthesis_reverbs[ri].items[cur][update_index];
            item.unk_14 = (((item.unk_18 as i32) << 0xF) / ai_buf_len) as u16;
            item.unk_16 = (((ai_buf_len) << 0xF) / (item.unk_18 as i32).max(1)) as u16;
        }
        let r = self.synthesis_reverbs[ri];
        let item = r.items[cur][update_index];
        let offset_a = ((item.start_pos & 7) * SS) as i16;
        let offset_b = align16i(offset_a as i32 + item.length_a as i32) as i16;
        self.load_ring_buffer_part(DMEM_WET_TEMP, (item.start_pos - (offset_a as i32 / SS)) as u32, D1CH as i32, ri);
        if item.length_b != 0 {
            // Ring buffer wrapped
            self.load_ring_buffer_part(DMEM_WET_TEMP + offset_b as u32, 0, D1CH as i32 - offset_b as i32, ri);
        }
        self.cmd(a_set_buffer(0, DMEM_WET_TEMP + offset_a as u32, DMEM_WET_LEFT_CH, (ai_buf_len * SS) as u32));
        self.cmd(a_resample(r.resample_flags as u32, item.unk_14 as u32, r.unk_30));
        self.cmd(a_set_buffer(0, DMEM_WET_TEMP + D1CH + offset_a as u32, DMEM_WET_RIGHT_CH, (ai_buf_len * SS) as u32));
        self.cmd(a_resample(r.resample_flags as u32, item.unk_14 as u32, r.unk_34));
    }

    /// `AudioSynth_FilterReverb`: a filter (convolution) on each reverb channel.
    fn filter_reverb(&mut self, size: i32, ri: usize) {
        let r = self.synthesis_reverbs[ri];
        if r.filter_left != 0 {
            self.cmd(a_filter(2, size as u32, r.filter_left));
            self.cmd(a_filter(r.resample_flags as u32, DMEM_WET_LEFT_CH, r.filter_left_state));
        }
        if r.filter_right != 0 {
            self.cmd(a_filter(2, size as u32, r.filter_right));
            self.cmd(a_filter(r.resample_flags as u32, DMEM_WET_RIGHT_CH, r.filter_right_state));
        }
    }

    /// `AudioSynth_MaybeMixRingBuffer1`.
    fn maybe_mix_ring_buffer1(&mut self, ri: usize, update_index: usize) {
        let other = self.synthesis_reverbs[ri].unk_05 as usize & 3;
        if self.synthesis_reverbs[other].downsample_rate == 1 {
            self.load_ring_buffer1_at_temp(other, update_index);
            let g = self.synthesis_reverbs[ri].unk_08 as u16 as u32;
            self.cmd(a_mix(D2CH >> 4, g, DMEM_WET_LEFT_CH, DMEM_WET_TEMP));
            self.save_ring_buffer1_at_temp(other, update_index);
        }
    }

    /// `AudioSynth_LoadRingBuffer1`.
    fn load_ring_buffer1(&mut self, ri: usize, update_index: usize) {
        let r = self.synthesis_reverbs[ri];
        let item = r.items[r.cur_frame as usize][update_index];
        self.load_ring_buffer_part(DMEM_WET_LEFT_CH, item.start_pos as u32, item.length_a as i32, ri);
        if item.length_b != 0 {
            // Ring buffer wrapped
            self.load_ring_buffer_part(DMEM_WET_LEFT_CH + item.length_a as u32, 0, item.length_b as i32, ri);
        }
    }

    /// `AudioSynth_LoadRingBuffer2`.
    fn load_ring_buffer2(&mut self, ri: usize, update_index: usize) {
        let r = self.synthesis_reverbs[ri];
        let item = r.items2[r.cur_frame as usize][update_index];
        self.load_ring_buffer_part(DMEM_WET_LEFT_CH, item.start_pos as u32, item.length_a as i32, ri);
        if item.length_b != 0 {
            // Ring buffer wrapped
            self.load_ring_buffer_part(DMEM_WET_LEFT_CH + item.length_a as u32, 0, item.length_b as i32, ri);
        }
    }

    /// `AudioSynth_LoadRingBufferPart`.
    fn load_ring_buffer_part(&mut self, dmem: u32, start_pos: u32, size: i32, ri: usize) {
        let r = self.synthesis_reverbs[ri];
        let start_pos = start_pos as u16 as u32;
        self.cmd(a_load_buffer(r.left_ring_buf + 2 * start_pos, dmem as u16 as u32, size as u32));
        self.cmd(a_load_buffer(r.right_ring_buf + 2 * start_pos, (dmem + D1CH) as u16 as u32, size as u32));
    }

    /// `AudioSynth_SaveRingBufferPart`.
    fn save_ring_buffer_part(&mut self, dmem: u32, start_pos: u32, size: i32, ri: usize) {
        let r = self.synthesis_reverbs[ri];
        let start_pos = start_pos as u16 as u32;
        self.cmd(a_save_buffer(dmem as u16 as u32, r.left_ring_buf + 2 * start_pos, size as u32));
        self.cmd(a_save_buffer((dmem + D1CH) as u16 as u32, r.right_ring_buf + 2 * start_pos, size as u32));
    }

    /// `AudioSynth_SaveBufferOffset`.
    fn save_buffer_offset(&mut self, dmem: u32, offset: u32, size: i32, buf: u32) {
        self.cmd(a_save_buffer(dmem as u16 as u32, buf + 2 * (offset as u16 as u32), size as u32));
    }

    /// `AudioSynth_MaybeLoadRingBuffer2`.
    fn maybe_load_ring_buffer2(&mut self, ri: usize, update_index: usize) {
        if self.synthesis_reverbs[ri].downsample_rate == 1 {
            self.load_ring_buffer2(ri, update_index);
        }
    }

    /// `AudioSynth_LoadReverbSamples`: sets the wet channels, clobbers `DMEM_TEMP`.
    fn load_reverb_samples(&mut self, ai_buf_len: i32, ri: usize, update_index: usize) {
        let r = self.synthesis_reverbs[ri];
        if r.downsample_rate == 1 {
            if r.unk_18 != 0 {
                self.func_800db828(ai_buf_len, ri, update_index);
            } else {
                self.load_ring_buffer1(ri, update_index);
            }
        } else {
            self.func_800db4e4(ai_buf_len, ri, update_index);
        }
    }

    /// `AudioSynth_SaveReverbSamples`.
    fn save_reverb_samples(&mut self, ri: usize, update_index: usize) {
        let r = self.synthesis_reverbs[ri];
        let item = r.items[r.cur_frame as usize][update_index];
        if r.downsample_rate == 1 {
            if r.unk_18 != 0 {
                self.func_800db680(ri, update_index);
            } else {
                // Put the oldest samples in the ring buffer into the wet channels
                self.save_ring_buffer_part(DMEM_WET_LEFT_CH, item.start_pos as u32, item.length_a as i32, ri);
                if item.length_b != 0 {
                    // Ring buffer wrapped
                    self.save_ring_buffer_part(DMEM_WET_LEFT_CH + item.length_a as u32, 0, item.length_b as i32, ri);
                }
            }
        } else {
            // Downsampling is done later by CPU when RSP is done, therefore we need to have
            // double buffering. Left and right buffers are adjacent in memory.
            self.cmd(a_save_buffer(DMEM_WET_LEFT_CH, item.to_downsample_left, D2CH));
        }
        self.synthesis_reverbs[ri].resample_flags = 0;
    }

    /// `AudioSynth_SaveRingBuffer2`.
    fn save_ring_buffer2(&mut self, ri: usize, update_index: usize) {
        let r = self.synthesis_reverbs[ri];
        let item = r.items2[r.cur_frame as usize][update_index];
        self.save_ring_buffer_part(DMEM_WET_LEFT_CH, item.start_pos as u32, item.length_a as i32, ri);
        if item.length_b != 0 {
            // Ring buffer wrapped
            self.save_ring_buffer_part(DMEM_WET_LEFT_CH + item.length_a as u32, 0, item.length_b as i32, ri);
        }
    }

    /// `AudioSynth_DoOneAudioUpdate`.
    fn do_one_audio_update(&mut self, ai_buf: u32, ai_buf_len: i32, update_index: usize) {
        let t = self.num_notes as usize * update_index;
        let mut note_indices: Vec<usize> = Vec::with_capacity(0x5C);
        let nr = self.num_synthesis_reverbs as usize;
        if nr == 0 {
            for i in 0..self.num_notes as usize {
                if self.note_subs_eu[t + i].enabled {
                    note_indices.push(i);
                }
            }
        } else {
            for reverb_index in 0..nr {
                for i in 0..self.num_notes as usize {
                    let s = &self.note_subs_eu[t + i];
                    if s.enabled && s.reverb_index as usize == reverb_index {
                        note_indices.push(i);
                    }
                }
            }
            for i in 0..self.num_notes as usize {
                let s = &self.note_subs_eu[t + i];
                if s.enabled && s.reverb_index as usize >= nr {
                    note_indices.push(i);
                }
            }
        }
        let count = note_indices.len();

        self.cmd(a_clear_buffer(DMEM_LEFT_CH, D2CH));

        let mut i = 0;
        for reverb_index in 0..nr {
            let r = self.synthesis_reverbs[reverb_index];
            let use_reverb = r.use_reverb != 0;
            let mut unk14 = 0;
            if use_reverb {
                // Loads reverb samples from RDRAM (ringBuffer) into DMEM (DMEM_WET_LEFT_CH)
                self.load_reverb_samples(ai_buf_len, reverb_index, update_index);
                // Mixes reverb sample into the main dry channel. reverb->volume is always 0x7FFF
                // (audio spec), and DMEM_LEFT_CH is cleared before the loop.
                self.cmd(a_mix(D2CH >> 4, r.volume as u16 as u32, DMEM_WET_LEFT_CH, DMEM_LEFT_CH));
                unk14 = r.unk_14;
                if unk14 != 0 {
                    self.cmd(a_dmem_move(DMEM_WET_LEFT_CH, DMEM_WET_TEMP, D2CH));
                }
                // Decays reverb over time. The (+ 0x8000) here is -100%
                self.cmd(a_mix(D2CH >> 4, (r.decay_ratio as u32 + 0x8000) & 0xFFFF, DMEM_WET_LEFT_CH, DMEM_WET_LEFT_CH));
                // Leak reverb between the left and right channels
                if r.leak_rtl != 0 || r.leak_ltr != 0 {
                    self.leak_reverb(reverb_index);
                }
                if unk14 != 0 {
                    // Saves the wet channel sample from DMEM into RDRAM (ringBuffer) for future use
                    self.save_reverb_samples(reverb_index, update_index);
                    if self.synthesis_reverbs[reverb_index].unk_05 != -1 {
                        self.maybe_mix_ring_buffer1(reverb_index, update_index);
                    }
                    self.maybe_load_ring_buffer2(reverb_index, update_index);
                    self.cmd(a_mix(D2CH >> 4, r.unk_16 as u16 as u32, DMEM_WET_TEMP, DMEM_WET_LEFT_CH));
                }
            }

            while i < count {
                let ni = note_indices[i];
                if self.note_subs_eu[ni + t].reverb_index as usize == reverb_index {
                    self.process_note(ni, ni + t, ai_buf, ai_buf_len, update_index);
                } else {
                    break;
                }
                i += 1;
            }

            if use_reverb {
                let r = self.synthesis_reverbs[reverb_index];
                if r.filter_left != 0 || r.filter_right != 0 {
                    self.filter_reverb(ai_buf_len * SS, reverb_index);
                }
                // Saves the wet channel sample from DMEM (DMEM_WET_LEFT_CH) into RDRAM
                // (ringBuffer) for future use
                if unk14 != 0 {
                    self.save_ring_buffer2(reverb_index, update_index);
                } else {
                    self.save_reverb_samples(reverb_index, update_index);
                    if self.synthesis_reverbs[reverb_index].unk_05 != -1 {
                        self.maybe_mix_ring_buffer1(reverb_index, update_index);
                    }
                }
            }
        }

        while i < count {
            let ni = note_indices[i];
            self.process_note(ni, t + ni, ai_buf, ai_buf_len, update_index);
            i += 1;
        }

        self.cmd(a_interleave(DMEM_TEMP, DMEM_LEFT_CH, DMEM_RIGHT_CH, (2 * ai_buf_len) as u32));
        self.cmd(a_save_buffer(DMEM_TEMP, ai_buf, (2 * (ai_buf_len * SS)) as u32));
    }

    /// `AudioSynth_ProcessNote`: decodes the note's sample for this update, resamples it to
    /// the output rate, and mixes it in.
    fn process_note(&mut self, note_index: usize, sub: usize, _ai_buf: u32, ai_buf_len: i32, update_index: usize) {
        let s = self.note_subs_eu[sub];
        let book_offset = s.book_offset;
        let mut finished = s.finished;
        let mut flags = A_CONTINUE;
        let buffers = self.notes[note_index].synthesis_state.synthesis_buffers;

        if s.needs_init {
            flags = A_INIT;
            let start = self.notes[note_index].start_sample_pos;
            let st = &mut self.notes[note_index].synthesis_state;
            st.restart = 0;
            st.sample_pos_int = start as i32;
            st.sample_pos_frac = 0;
            st.cur_vol_left = 0;
            st.cur_vol_right = 0;
            st.prev_haas_effect_left_delay_size = 0;
            st.prev_haas_effect_right_delay_size = 0;
            st.reverb_vol = s.reverb_vol;
            st.num_parts = 0;
            st.comb_filter_needs_init = 1;
            self.notes[note_index].note_sub_eu.finished = false;
            finished = false;
        }

        let resampling_rate_fixed_point = s.resampling_rate_fixed_point;
        let n_parts = s.has_two_parts as i32 + 1;
        let st = &mut self.notes[note_index].synthesis_state;
        let samples_len_fixed_point = (resampling_rate_fixed_point as u32).wrapping_mul(ai_buf_len as u32 * 2).wrapping_add(st.sample_pos_frac as u32);
        let num_samples_to_load = (samples_len_fixed_point >> 16) as i32;
        st.sample_pos_frac = (samples_len_fixed_point & 0xFFFF) as u16;
        st.num_parts = n_parts as u8;

        let mut sample_dmem_before_resampling: u32 = 0;
        let mut skip_bytes: i32 = 0;
        if s.is_synthetic_wave {
            self.load_wave_samples(sub, note_index, num_samples_to_load);
            let st = &mut self.notes[note_index].synthesis_state;
            sample_dmem_before_resampling = (DMEM_UNCOMPRESSED_NOTE as i32 + st.sample_pos_int * SS) as u32;
            st.sample_pos_int += num_samples_to_load;
        } else {
            let sample_ptr = self.ram.u32(s.tuned_sample);
            let sample = self.ram.sample(sample_ptr);
            let (loop_start, loop_end, loop_count) = self.ram.adpcm_loop(sample.loop_addr);
            let loop_end_pos = loop_end as i32;
            let sample_addr = sample.sample_addr;
            let mut resampled_temp_len = 0;

            for cur_part in 0..n_parts {
                let mut n_samples_processed = 0;
                let mut s5: i32 = 0;
                let samples_len_adjusted = if n_parts == 1 {
                    num_samples_to_load
                } else if num_samples_to_load & 1 != 0 {
                    (num_samples_to_load & !1) + (cur_part * 2)
                } else {
                    num_samples_to_load
                };

                if sample.codec == CODEC_ADPCM || sample.codec == CODEC_SMALL_ADPCM {
                    let book = sample.book + crate::layout::BOOK_BOOK;
                    if self.cur_loaded_book != book {
                        self.cur_loaded_book = match book_offset {
                            1 => self.statics.d_8012fba8 + 2,
                            _ => book,
                        };
                        let (order, num_predictors) = self.ram.adpcm_book(sample.book);
                        let n_entries = (SPF * order * num_predictors) as u32;
                        let b = self.cur_loaded_book;
                        self.cmd(a_load_adpcm(n_entries, b));
                    }
                }

                while n_samples_processed != samples_len_adjusted {
                    let mut note_finished = false;
                    let mut restart = false;
                    let mut phi_s4: i32 = 0;
                    let st = self.notes[note_index].synthesis_state;
                    let mut n_first_frame_samples_to_ignore = st.sample_pos_int & 0xF;
                    let n_samples_until_loop_end = loop_end_pos - st.sample_pos_int;
                    let n_samples_to_process = samples_len_adjusted - n_samples_processed;
                    if n_first_frame_samples_to_ignore == 0 && st.restart == 0 {
                        n_first_frame_samples_to_ignore = SPF;
                    }
                    let mut n_samples_in_first_frame = SPF - n_first_frame_samples_to_ignore;
                    let mut n_samples_to_decode;
                    let n_trailing_samples_to_ignore;
                    let n_frames_to_decode;
                    if n_samples_to_process < n_samples_until_loop_end {
                        n_frames_to_decode = (n_samples_to_process - n_samples_in_first_frame + SPF - 1) / SPF;
                        n_samples_to_decode = n_frames_to_decode * SPF;
                        n_trailing_samples_to_ignore = n_samples_in_first_frame + n_samples_to_decode - n_samples_to_process;
                    } else {
                        n_samples_to_decode = n_samples_until_loop_end - n_samples_in_first_frame;
                        n_trailing_samples_to_ignore = 0;
                        if n_samples_to_decode <= 0 {
                            n_samples_to_decode = 0;
                            n_samples_in_first_frame = n_samples_until_loop_end;
                        }
                        n_frames_to_decode = (n_samples_to_decode + SPF - 1) / SPF;
                        if loop_count != 0 {
                            // Loop around and restart
                            restart = true;
                        } else {
                            note_finished = true;
                        }
                    }

                    let (frame_size, skip_initial_samples, sample_data_start): (i32, i32, i32);
                    let mut skipped = false;
                    match sample.codec {
                        CODEC_ADPCM => (frame_size, skip_initial_samples, sample_data_start) = (9, SPF, 0),
                        CODEC_SMALL_ADPCM => (frame_size, skip_initial_samples, sample_data_start) = (5, SPF, 0),
                        CODEC_S8 => (frame_size, skip_initial_samples, sample_data_start) = (16, SPF, 0),
                        CODEC_S16_INMEMORY | CODEC_S16 => {
                            self.cmd(a_clear_buffer(DMEM_UNCOMPRESSED_NOTE, ((samples_len_adjusted + SPF) * SS) as u32));
                            flags = A_CONTINUE;
                            skip_bytes = 0;
                            n_samples_processed = samples_len_adjusted;
                            s5 = samples_len_adjusted;
                            skipped = true;
                            (frame_size, skip_initial_samples, sample_data_start) = (0, 0, 0);
                        }
                        // CODEC_REVERB: the C leaves these unset.
                        _ => (frame_size, skip_initial_samples, sample_data_start) = (0, SPF, 0),
                    }

                    if !skipped {
                        let sample_data_start_pad: i32;
                        if n_frames_to_decode != 0 {
                            let frame_index = (st.sample_pos_int + skip_initial_samples - n_first_frame_samples_to_ignore) / SPF;
                            let sample_data_offset = frame_index * frame_size;
                            let size = align16i((n_frames_to_decode * frame_size) + SPF);
                            let sample_data = if sample.medium == MEDIUM_RAM {
                                (sample_data_start + sample_data_offset) as u32 + sample_addr
                            } else if sample.medium == MEDIUM_UNK {
                                return;
                            } else {
                                let mut dma_index = self.notes[note_index].synthesis_state.sample_dma_index;
                                let a = self.dma_sample_data((sample_data_start + sample_data_offset) as u32 + sample_addr, size as u32, flags as i32, &mut dma_index, sample.medium as i32);
                                self.notes[note_index].synthesis_state.sample_dma_index = dma_index;
                                a
                            };
                            if sample_data == 0 {
                                return;
                            }
                            sample_data_start_pad = (sample_data & 0xF) as i32;
                            let addr = DMEM_COMPRESSED_ADPCM_DATA as i32 - size;
                            self.cmd(a_load_buffer(sample_data - sample_data_start_pad as u32, addr as u16 as u32, size as u32));
                        } else {
                            n_samples_to_decode = 0;
                            sample_data_start_pad = 0;
                        }

                        if self.notes[note_index].synthesis_state.restart != 0 {
                            self.cmd(a_set_loop(sample.loop_addr + crate::layout::LOOP_PREDICTOR_STATE));
                            flags = A_LOOP;
                            self.notes[note_index].synthesis_state.restart = 0;
                        }

                        let n_samples_in_this_iteration = n_samples_to_decode + n_samples_in_first_frame - n_trailing_samples_to_ignore;
                        if n_samples_processed == 0 {
                            skip_bytes = n_first_frame_samples_to_ignore * SS;
                        } else {
                            phi_s4 = align16i(s5 + 8 * SS);
                        }

                        let aligned = align16i((n_frames_to_decode * frame_size) + SPF);
                        let addr = (DMEM_COMPRESSED_ADPCM_DATA as i32 - aligned) as u32;
                        match sample.codec {
                            CODEC_ADPCM => {
                                self.cmd(a_set_buffer(0, (addr as i32 + sample_data_start_pad) as u32 & 0xFFFF, DMEM_UNCOMPRESSED_NOTE + phi_s4 as u32, (n_samples_to_decode * SS) as u32));
                                self.cmd(a_adpcm_dec(flags, buffers + NSB_ADPCMDEC_STATE));
                            }
                            CODEC_SMALL_ADPCM => {
                                self.cmd(a_set_buffer(0, (addr as i32 + sample_data_start_pad) as u32 & 0xFFFF, DMEM_UNCOMPRESSED_NOTE + phi_s4 as u32, (n_samples_to_decode * SS) as u32));
                                self.cmd(a_adpcm_dec(flags | 4, buffers + NSB_ADPCMDEC_STATE));
                            }
                            CODEC_S8 => {
                                self.cmd(a_set_buffer(0, (addr as i32 + sample_data_start_pad) as u32 & 0xFFFF, DMEM_UNCOMPRESSED_NOTE + phi_s4 as u32, (n_samples_to_decode * SS) as u32));
                                self.cmd(a_s8_dec(flags, buffers + NSB_ADPCMDEC_STATE));
                            }
                            _ => {}
                        }

                        if n_samples_processed != 0 {
                            self.cmd(a_dmem_move(
                                (DMEM_UNCOMPRESSED_NOTE as i32 + phi_s4 + (n_first_frame_samples_to_ignore * SS)) as u32,
                                (DMEM_UNCOMPRESSED_NOTE as i32 + s5) as u32,
                                (n_samples_in_this_iteration * SS) as u32,
                            ));
                        }

                        n_samples_processed += n_samples_in_this_iteration;

                        match flags {
                            A_INIT => {
                                skip_bytes = SPF * SS;
                                s5 = (n_samples_to_decode + SPF) * SS;
                            }
                            A_LOOP => s5 += n_samples_in_this_iteration * SS,
                            _ => {
                                if s5 != 0 {
                                    s5 += n_samples_in_this_iteration * SS;
                                } else {
                                    s5 = (n_first_frame_samples_to_ignore + n_samples_in_this_iteration) * SS;
                                }
                            }
                        }
                        flags = A_CONTINUE;
                    }

                    // skip:
                    if note_finished {
                        self.cmd(a_clear_buffer((DMEM_UNCOMPRESSED_NOTE as i32 + s5) as u32, ((samples_len_adjusted - n_samples_processed) * SS) as u32));
                        finished = true;
                        self.notes[note_index].note_sub_eu.finished = true;
                        self.func_800db2c0(update_index, note_index);
                        break;
                    } else {
                        let st = &mut self.notes[note_index].synthesis_state;
                        if restart {
                            st.restart = 1;
                            st.sample_pos_int = loop_start as i32;
                        } else {
                            st.sample_pos_int += n_samples_to_process;
                        }
                    }
                }

                match n_parts {
                    1 => sample_dmem_before_resampling = (DMEM_UNCOMPRESSED_NOTE as i32 + skip_bytes) as u32,
                    2 => match cur_part {
                        0 => {
                            self.cmd(audio_synth_inter_l((DMEM_UNCOMPRESSED_NOTE as i32 + skip_bytes) as u32, DMEM_TEMP + (SPF * SS) as u32, align8(samples_len_adjusted / 2) as u32));
                            resampled_temp_len = samples_len_adjusted;
                            sample_dmem_before_resampling = DMEM_TEMP + (SPF * SS) as u32;
                            if finished {
                                self.cmd(a_clear_buffer(sample_dmem_before_resampling + resampled_temp_len as u32, (samples_len_adjusted + SPF) as u32));
                            }
                        }
                        _ => {
                            self.cmd(audio_synth_inter_l(
                                (DMEM_UNCOMPRESSED_NOTE as i32 + skip_bytes) as u32,
                                DMEM_TEMP + (SPF * SS) as u32 + resampled_temp_len as u32,
                                align8(samples_len_adjusted / 2) as u32,
                            ));
                        }
                    },
                    _ => {}
                }
                if finished {
                    break;
                }
            }
        }

        flags = A_CONTINUE;
        if self.note_subs_eu[sub].needs_init {
            self.note_subs_eu[sub].needs_init = false;
            flags = A_INIT;
        }

        self.final_resample(buffers, ai_buf_len * SS, resampling_rate_fixed_point as u32, sample_dmem_before_resampling, flags);
        if book_offset == 3 {
            self.cmd(audio_synth_unk_cmd19(DMEM_TEMP, DMEM_TEMP, (ai_buf_len * SS) as u32, 0));
        }
        if book_offset == 2 {
            self.cmd(audio_synth_unk_cmd3(DMEM_TEMP, DMEM_TEMP, (ai_buf_len * SS) as u32));
        }

        let mut gain = s.gain as u32;
        if gain != 0 {
            // A gain of 0x10 (a UQ4.4 number) is equivalent to 1.0 and represents no volume change
            if gain < 0x10 {
                gain = 0x10;
            }
            self.cmd(audio_synth_hi_lo_gain(gain, DMEM_TEMP, 0, ((ai_buf_len + SPF) * SS) as u32));
        }

        if s.filter != 0 {
            self.cmd(a_filter(2, (ai_buf_len * SS) as u32, s.filter));
            self.cmd(a_filter(flags, DMEM_TEMP, buffers + NSB_MIX_ENVELOPE_STATE));
        }

        let unk7 = s.comb_filter_size as u32;
        let unk_e = s.comb_filter_gain as u32;
        let buf = buffers + NSB_UNK_STATE;
        if unk7 != 0 && unk_e != 0 {
            self.cmd(audio_synth_dmem_move(DMEM_TEMP, DMEM_COMB_TEMP, (ai_buf_len * SS) as u32));
            let thing = DMEM_COMB_TEMP - unk7;
            if self.notes[note_index].synthesis_state.comb_filter_needs_init != 0 {
                self.cmd(a_clear_buffer(thing, unk7));
                self.notes[note_index].synthesis_state.comb_filter_needs_init = 0;
            } else {
                self.cmd(a_load_buffer(buf, thing, unk7));
            }
            self.cmd(a_save_buffer(DMEM_TEMP + (ai_buf_len * SS) as u32 - unk7, buf, unk7));
            self.cmd(a_mix(((ai_buf_len * SS) >> 4) as u32, unk_e, DMEM_COMB_TEMP, thing));
            self.cmd(audio_synth_dmem_move(thing, DMEM_TEMP, (ai_buf_len * SS) as u32));
        } else {
            self.notes[note_index].synthesis_state.comb_filter_needs_init = 1;
        }

        let st = self.notes[note_index].synthesis_state;
        let haas_effect_delay_side = if s.haas_effect_left_delay_size != 0 || st.prev_haas_effect_left_delay_size != 0 {
            HAAS_EFFECT_DELAY_LEFT
        } else if s.haas_effect_right_delay_size != 0 || st.prev_haas_effect_right_delay_size != 0 {
            HAAS_EFFECT_DELAY_RIGHT
        } else {
            HAAS_EFFECT_DELAY_NONE
        };

        self.process_envelope(sub, note_index, ai_buf_len, DMEM_TEMP, haas_effect_delay_side);

        if s.use_haas_effect {
            if flags & A_INIT == 0 {
                flags = A_CONTINUE;
            }
            self.apply_haas_effect(sub, note_index, ai_buf_len * SS, flags, haas_effect_delay_side);
        }
    }

    /// `AudioSynth_FinalResample`.
    fn final_resample(&mut self, buffers: u32, size: i32, pitch: u32, inp_dmem: u32, resample_flags: u32) {
        if pitch == 0 {
            self.cmd(a_clear_buffer(DMEM_TEMP, size as u32));
        } else {
            self.cmd(a_set_buffer(0, inp_dmem & 0xFFFF, DMEM_TEMP, size as u32));
            self.cmd(a_resample(resample_flags, pitch, buffers + NSB_FINAL_RESAMPLE_STATE));
        }
    }

    /// `AudioSynth_ProcessEnvelope`: the volume ramps to this update's targets, and the mix.
    fn process_envelope(&mut self, sub: usize, n: usize, ai_buf_len: i32, dmem_src: u32, haas_effect_delay_side: i32) {
        let s = self.note_subs_eu[sub];
        let st = &mut self.notes[n].synthesis_state;
        let cur_vol_left = st.cur_vol_left;
        let target_vol_left = s.target_vol_left.wrapping_shl(4);
        let reverb_vol = s.reverb_vol as i16;
        let cur_vol_right = st.cur_vol_right;
        let target_vol_right = s.target_vol_right.wrapping_shl(4);
        let steps = ai_buf_len >> 3;

        let ramp_left: i16 = if target_vol_left != cur_vol_left { ((target_vol_left as i32 - cur_vol_left as i32) / steps) as i16 } else { 0 };
        let ramp_right: i16 = if target_vol_right != cur_vol_right { ((target_vol_right as i32 - cur_vol_right as i32) / steps) as i16 } else { 0 };

        let source_reverb_vol = st.reverb_vol as i16;
        let phi_t1 = (source_reverb_vol & 0x7F) as i32;
        let ramp_reverb: i16 = if source_reverb_vol != reverb_vol {
            st.reverb_vol = reverb_vol as u8;
            ((((reverb_vol as i32 & 0x7F) - phi_t1) << 9) / steps) as i16
        } else {
            0
        };

        st.cur_vol_left = (cur_vol_left as i32 + ramp_left as i32 * steps) as u16;
        st.cur_vol_right = (cur_vol_right as i32 + ramp_right as i32 * steps) as u16;

        let dmem_dests;
        if s.use_haas_effect {
            self.cmd(a_clear_buffer(DMEM_HAAS_TEMP, D1CH));
            self.cmd(a_env_setup1((phi_t1 * 2) as u32, ramp_reverb as u16 as u32, ramp_left as u16 as u32, ramp_right as u16 as u32));
            self.cmd(audio_synth_env_setup2(cur_vol_left as u32, cur_vol_right as u32));
            dmem_dests = match haas_effect_delay_side {
                HAAS_EFFECT_DELAY_LEFT => S_ENV_MIXER_LEFT_HAAS_DMEM_DESTS,
                HAAS_EFFECT_DELAY_RIGHT => S_ENV_MIXER_RIGHT_HAAS_DMEM_DESTS,
                _ => S_ENV_MIXER_DEFAULT_DMEM_DESTS,
            };
        } else {
            self.cmd(a_env_setup1((phi_t1 * 2) as u32, ramp_reverb as u16 as u32, ramp_left as u16 as u32, ramp_right as u16 as u32));
            self.cmd(a_env_setup2(cur_vol_left as u32, cur_vol_right as u32));
            dmem_dests = S_ENV_MIXER_DEFAULT_DMEM_DESTS;
        }
        self.cmd(a_env_mixer(
            dmem_src,
            ai_buf_len as u32,
            ((source_reverb_vol & 0x80) >> 7) as u32,
            s.stereo_headset_effects as u32,
            s.uses_headset_pan_effects as u32,
            s.stereo_strong_right as u32,
            s.stereo_strong_left as u32,
            dmem_dests,
            S_ENV_MIXER_OP,
        ));
    }

    /// `AudioSynth_LoadWaveSamples`.
    fn load_wave_samples(&mut self, sub: usize, n: usize, num_samples_to_load: i32) {
        let s = self.note_subs_eu[sub];
        let harmonic_index_cur_and_prev = s.harmonic_index_cur_and_prev as i32;
        let mut sample_pos_int = self.notes[n].synthesis_state.sample_pos_int;
        if s.book_offset != 0 {
            // Move the noise wave (that reads compiled assembly as samples) from ram to dmem
            let w = self.wave8;
            self.cmd(a_load_buffer(w, DMEM_UNCOMPRESSED_NOTE, align16i(num_samples_to_load * SS) as u32));
            // Offset the address for the samples read by gWaveSamples[8] to the next set of samples
            self.wave8 = self.wave8.wrapping_add((num_samples_to_load * SS) as u32);
            return;
        }
        // Move the synthetic wave from ram to dmem
        self.cmd(a_load_buffer(s.tuned_sample, DMEM_UNCOMPRESSED_NOTE, (WAVE_SAMPLE_COUNT * SS) as u32));
        // If the harmonic changes, map the offset in the wave from one harmonic to another for
        // continuity
        if harmonic_index_cur_and_prev != 0 {
            sample_pos_int =
                sample_pos_int * S_NUM_SAMPLES_PER_WAVE_PERIOD[(harmonic_index_cur_and_prev >> 2) as usize & 3] / S_NUM_SAMPLES_PER_WAVE_PERIOD[(harmonic_index_cur_and_prev & 3) as usize];
        }
        // Offset in the WAVE_SAMPLE_COUNT samples of gWaveSamples to start processing the wave
        // for continuity
        sample_pos_int = ((sample_pos_int as u32) % WAVE_SAMPLE_COUNT as u32) as i32;
        let num_samples_avail = WAVE_SAMPLE_COUNT - sample_pos_int;
        // Require duplicates if there are more samples to load than available
        if num_samples_to_load > num_samples_avail {
            let num_duplicates = (num_samples_to_load - num_samples_avail + WAVE_SAMPLE_COUNT - 1) / WAVE_SAMPLE_COUNT;
            if num_duplicates != 0 {
                self.cmd(a_duplicate(num_duplicates as u32, DMEM_UNCOMPRESSED_NOTE, DMEM_UNCOMPRESSED_NOTE + (WAVE_SAMPLE_COUNT * SS) as u32));
            }
        }
        self.notes[n].synthesis_state.sample_pos_int = sample_pos_int;
    }

    /// `AudioSynth_ApplyHaasEffect`: delays one side by a few samples (headset mode).
    fn apply_haas_effect(&mut self, sub: usize, n: usize, size: i32, flags: u32, haas_effect_delay_side: i32) {
        let s = self.note_subs_eu[sub];
        let buffers = self.notes[n].synthesis_state.synthesis_buffers;
        let st = &mut self.notes[n].synthesis_state;
        let (dmem_dest, haas, prev): (u32, i32, i32) = match haas_effect_delay_side {
            HAAS_EFFECT_DELAY_LEFT => {
                let r = (DMEM_LEFT_CH, s.haas_effect_left_delay_size as i32, st.prev_haas_effect_left_delay_size as i32);
                st.prev_haas_effect_right_delay_size = 0;
                st.prev_haas_effect_left_delay_size = s.haas_effect_left_delay_size;
                r
            }
            HAAS_EFFECT_DELAY_RIGHT => {
                let r = (DMEM_RIGHT_CH, s.haas_effect_right_delay_size as i32, st.prev_haas_effect_right_delay_size as i32);
                st.prev_haas_effect_right_delay_size = s.haas_effect_right_delay_size;
                st.prev_haas_effect_left_delay_size = 0;
                r
            }
            _ => return,
        };
        let state = buffers + NSB_HAAS_EFFECT_DELAY_STATE;
        if flags != A_INIT {
            // Slightly adjust the sample rate in order to fit a change in sample delay
            if haas != prev {
                let pitch = ((((size << 0xF) / 2) - 1) / ((size + haas - prev - 2) / 2)) as u16;
                self.cmd(a_set_buffer(0, DMEM_HAAS_TEMP, DMEM_TEMP, (size + haas - prev) as u32));
                self.cmd(a_resample_zoh(pitch as u32, 0));
            } else {
                self.cmd(a_dmem_move(DMEM_HAAS_TEMP, DMEM_TEMP, size as u32));
            }
            if prev != 0 {
                self.cmd(a_load_buffer(state, DMEM_HAAS_TEMP, align16i(prev) as u32));
                self.cmd(a_dmem_move(DMEM_TEMP, DMEM_HAAS_TEMP + prev as u32, (size + haas - prev) as u32));
            } else {
                self.cmd(a_dmem_move(DMEM_TEMP, DMEM_HAAS_TEMP, (size + haas) as u32));
            }
        } else {
            // Just apply a delay directly
            self.cmd(a_dmem_move(DMEM_HAAS_TEMP, DMEM_TEMP, size as u32));
            self.cmd(a_clear_buffer(DMEM_HAAS_TEMP, haas as u32));
            self.cmd(a_dmem_move(DMEM_TEMP, DMEM_HAAS_TEMP + haas as u32, size as u32));
        }
        if haas != 0 {
            // Save excessive samples for next iteration
            self.cmd(a_save_buffer(DMEM_HAAS_TEMP + size as u32, state, align16i(haas) as u32));
        }
        self.cmd(a_add_mixer(align64(size) as u32, DMEM_HAAS_TEMP, dmem_dest, 0x7FFF));
    }
}
