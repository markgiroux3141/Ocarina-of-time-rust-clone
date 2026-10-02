//! `audio/internal/load.c`: the sample DMAs, loading sequences, fonts and sample banks into the heap's
//! caches, relocating fonts, the slow, async and script loads, and `AudioLoad_Init`.
//!
//! The PI's DMAs complete at once (`Cart::dma`); a load's completion message is there the next
//! time its queue is read, as it would be a frame later on the hardware.

use std::collections::VecDeque;
use std::sync::Arc;

use crate::context::*;
use crate::data::AudioData;
use crate::heap::alloc_zeroed;
use crate::layout::*;
use crate::ram::{Cart, K0BASE, Ram};
use crate::rsp::Rsp;

/// `MK_ASYNC_MSG`.
fn mk_async_msg(ret_data: u32, table_type: u32, id: u32, load_status: u32) -> u32 {
    (ret_data << 24) | (table_type << 16) | (id << 8) | load_status
}

/// `SampleBankRelocInfo`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SampleBankRelocInfo {
    pub sample_bank_id1: i32,
    pub sample_bank_id2: i32,
    pub base_addr1: u32,
    pub base_addr2: u32,
    pub medium1: u32,
    pub medium2: u32,
}

/// `SLOW_LOAD_STATE_*`.
const SLOW_LOAD_STATE_WAITING: i32 = 0;
const SLOW_LOAD_STATE_START: i32 = 1;
const SLOW_LOAD_STATE_LOADING: i32 = 2;
const SLOW_LOAD_STATE_DONE: i32 = 3;

/// Where the C's static data is in this port's RAM (`Statics`).
mod place {
    pub const STATICS: u32 = 0x8020_0000;
    pub const AUDIO_HEAP: u32 = 0x8030_0000;
}

impl AudioContext {
    /// Builds the context from the pack's audio data and runs `AudioLoad_Init(NULL, 0)`.
    pub fn new(data: &AudioData) -> AudioContext {
        let tables = Arc::new(data.tables.clone());
        let mut ram = Ram::new();
        let mut cart = Cart::new();
        for f in [&data.audiobank, &data.audioseq, &data.audiotable] {
            cart.add(f.rom_start, Arc::new(f.bytes.clone()));
        }

        // The C's static data, in RAM.
        let mut statics = Statics::default();
        let mut at = place::STATICS;
        let put16 = |ram: &mut Ram, vals: &[i16], at: &mut u32| -> u32 {
            let a = *at;
            for (i, v) in vals.iter().enumerate() {
                ram.set_s16(a + 2 * i as u32, *v);
            }
            *at = align16(a + 2 * vals.len() as u32);
            a
        };
        let waves: Vec<u32> = tables.wave_samples.iter().map(|w| put16(&mut ram, w, &mut at)).collect();
        for (i, idx) in tables.wave_sample_index.iter().take(8).enumerate() {
            statics.wave_samples[i] = waves.get(*idx as usize).copied().unwrap_or(0);
        }
        let env: Vec<i16> = tables.default_envelope.iter().flat_map(|&(d, a)| [d, a]).collect();
        statics.default_envelope = put16(&mut ram, &env, &mut at);
        let put8 = |ram: &mut Ram, vals: &[u8], at: &mut u32| -> u32 {
            let a = *at;
            ram.write(a, vals);
            *at = align16(a + vals.len() as u32);
            a
        };
        statics.default_short_note_velocity_table = put8(&mut ram, &tables.default_short_note_velocity_table, &mut at);
        statics.default_short_note_gate_time_table = put8(&mut ram, &tables.default_short_note_gate_time_table, &mut at);
        statics.sequence_font_table = put8(&mut ram, &tables.sequence_font_table, &mut at);
        statics.d_8012fba8 = put16(&mut ram, &tables.d_8012fba8, &mut at);
        for r in statics.reverb_tuned_samples.iter_mut() {
            *r = at;
            at += 0x50;
        }
        // AudioThread_Update's code, read as noise by gWaveSamples[8].
        statics.noise_base = tables.noise_code_vram;
        ram.write(tables.noise_code_vram, &tables.noise_code);

        let rsp = Rsp::new(&tables.resample_lut);
        let mut c = AudioContext {
            ram,
            cart,
            rsp,
            tables,
            statics,
            segment_rom_starts: [data.audioseq.rom_start, data.audiobank.rom_start, data.audiotable.rom_start],
            num_synthesis_reverbs: 0,
            unk_2: 0,
            unk_4: 0,
            cur_loaded_book: 0,
            note_subs_eu: Vec::new(),
            synthesis_reverbs: [SynthesisReverb::default(); 4],
            used_samples: vec![0; 128],
            preload_sample_stack: vec![AudioPreloadReq::default(); 128],
            num_used_samples: 0,
            preload_sample_stack_top: 0,
            async_loads: [AudioAsyncLoad::default(); 16],
            cur_unk_medium_load: None,
            slow_load_pos: 0,
            slow_loads: [AudioSlowLoad::default(); 2],
            external_load_queue: VecDeque::new(),
            preload_sample_queue: VecDeque::new(),
            script_load_queue: VecDeque::new(),
            script_load_done_pointers: [None; 16],
            script_load_index: 0,
            sample_dmas: Vec::new(),
            sample_dma_count: 0,
            sample_dma_list_size1: 0,
            sample_dma_reuse_queue1: [0; 0x100],
            sample_dma_reuse_queue2: [0; 0x100],
            sample_dma_reuse_queue1_rd_pos: 0,
            sample_dma_reuse_queue2_rd_pos: 0,
            sample_dma_reuse_queue1_wr_pos: 0,
            sample_dma_reuse_queue2_wr_pos: 0,
            sequence_table: AudioTable::default(),
            sound_font_table: AudioTable::default(),
            sample_bank_table: AudioTable::default(),
            num_sequences: 0,
            sound_font_list: Vec::new(),
            audio_buffer_parameters: AudioBufferParameters::default(),
            unk_2870: 0.0,
            sample_dma_buf_size1: 0,
            sample_dma_buf_size2: 0,
            sample_dma_buf_size: 0,
            max_audio_cmds: 0,
            num_notes: 0,
            tempo_internal_to_external: 0,
            sound_mode: 0,
            total_task_count: 0,
            cur_audio_frame_dma_count: 0,
            rsp_task_index: 0,
            cur_ai_buf_index: 0,
            abi_cmd_bufs: [0; 2],
            cmds: Vec::new(),
            max_tempo_tv_type_factors: 0.0,
            refresh_rate: 0,
            ai_buffers: [0; 3],
            ai_buf_lengths: [0; 3],
            audio_random: 0,
            audio_error_flags: 0,
            reset_timer: 0,
            session_pool: AudioAllocPool::default(),
            external_pool: AudioAllocPool::default(),
            init_pool: AudioAllocPool::default(),
            misc_pool: AudioAllocPool::default(),
            cache_pool: AudioAllocPool::default(),
            persistent_common_pool: AudioAllocPool::default(),
            temporary_common_pool: AudioAllocPool::default(),
            seq_cache: AudioCache::default(),
            font_cache: AudioCache::default(),
            sample_bank_cache: AudioCache::default(),
            permanent_pool: AudioAllocPool::default(),
            permanent_cache: [AudioCacheEntry::default(); 32],
            persistent_sample_cache: AudioSampleCache::default(),
            temporary_sample_cache: AudioSampleCache::default(),
            session_pool_split: AudioSessionPoolSplit::default(),
            cache_pool_split: AudioCachePoolSplit::default(),
            persistent_common_pool_split: AudioCommonPoolSplit::default(),
            temporary_common_pool_split: AudioCommonPoolSplit::default(),
            sample_font_load_status: [0; 0x30],
            font_load_status: [0; 0x30],
            seq_load_status: [0; 0x80],
            reset_status: 0,
            audio_reset_spec_id_to_load: 0,
            audio_reset_fade_out_frames_left: 0,
            adsr_decay_table_addr: 0,
            adsr_decay_table: vec![0.0; 256],
            audio_heap: 0,
            audio_heap_size: 0,
            notes_addr: 0,
            notes: Vec::new(),
            seq_players: Default::default(),
            sequence_layers: vec![SequenceLayer::default(); 64],
            channels: vec![SequenceChannel::default()],
            note_sub_eu_offset: 0,
            layer_free_list: 0,
            note_free_lists: 0,
            lists: Vec::new(),
            pools: Vec::new(),
            cmd_wr_pos: 0,
            cmd_rd_pos: 0,
            cmd_queue_finished: 0,
            thread_cmd_channel_mask: [0; 4],
            cmd_buf: [AudioCmd::default(); 0x100],
            cmd_proc_queue: VecDeque::new(),
            audio_reset_queue: VecDeque::new(),
            task_start_queue: VecDeque::new(),
            wave8: 0,
            audio_context_initialized: false,
            max_abi_cmd_cnt: 0x80,
            cur_cmd_rd_pos: 0,
            d_801304e8: 0,
            aud_rand: 0x1234_5678,
            d_80130510: 0.0,
            d_80130514: 0,
            os_count: 0,
            osc_frames: 0,
            stats: Stats::default(),
        };
        c.load_init();
        c
    }

    /// `AudioLoad_DecreaseSampleDmaTtls`.
    pub fn decrease_sample_dma_ttls(&mut self) {
        for i in 0..self.sample_dma_list_size1 as usize {
            let dma = &mut self.sample_dmas[i];
            if dma.ttl != 0 {
                dma.ttl -= 1;
                if dma.ttl == 0 {
                    dma.reuse_index = self.sample_dma_reuse_queue1_wr_pos;
                    self.sample_dma_reuse_queue1[self.sample_dma_reuse_queue1_wr_pos as usize] = i as u8;
                    self.sample_dma_reuse_queue1_wr_pos = self.sample_dma_reuse_queue1_wr_pos.wrapping_add(1);
                }
            }
        }
        for i in self.sample_dma_list_size1 as usize..self.sample_dma_count as usize {
            let dma = &mut self.sample_dmas[i];
            if dma.ttl != 0 {
                dma.ttl -= 1;
                if dma.ttl == 0 {
                    dma.reuse_index = self.sample_dma_reuse_queue2_wr_pos;
                    self.sample_dma_reuse_queue2[self.sample_dma_reuse_queue2_wr_pos as usize] = i as u8;
                    self.sample_dma_reuse_queue2_wr_pos = self.sample_dma_reuse_queue2_wr_pos.wrapping_add(1);
                }
            }
        }
    }

    /// `AudioLoad_DmaSampleData`: a RAM address holding `size` bytes of the sample data at
    /// `dev_addr`, DMAing it into one of the sample DMA buffers if none holds it; 0 if no
    /// buffer is free.
    pub fn dma_sample_data(&mut self, dev_addr: u32, size: u32, arg2: i32, dma_index_ref: &mut u8, medium: i32) -> u32 {
        let mut has_dma = false;
        let mut dma_index: usize = 0;
        let search_short_lived;
        if arg2 != 0 || (*dma_index_ref as u32) >= self.sample_dma_list_size1 {
            for i in self.sample_dma_list_size1 as usize..self.sample_dma_count as usize {
                let dma = self.sample_dmas[i];
                let buffer_pos = dev_addr.wrapping_sub(dma.dev_addr) as i32;
                if 0 <= buffer_pos && (buffer_pos as u32) <= (dma.size as u32).wrapping_sub(size) {
                    // We already have a DMA request for this memory range.
                    if dma.ttl == 0 && self.sample_dma_reuse_queue2_rd_pos != self.sample_dma_reuse_queue2_wr_pos {
                        // Move the DMA out of the reuse queue, by swapping it with the read pos,
                        // and then incrementing the read pos.
                        let rd = self.sample_dma_reuse_queue2_rd_pos as usize;
                        if dma.reuse_index as usize != rd {
                            let moved = self.sample_dma_reuse_queue2[rd];
                            self.sample_dma_reuse_queue2[dma.reuse_index as usize] = moved;
                            self.sample_dmas[moved as usize].reuse_index = dma.reuse_index;
                        }
                        self.sample_dma_reuse_queue2_rd_pos = self.sample_dma_reuse_queue2_rd_pos.wrapping_add(1);
                    }
                    self.sample_dmas[i].ttl = 32;
                    *dma_index_ref = i as u8;
                    return dma.ram_addr + (dev_addr - dma.dev_addr);
                }
            }
            if arg2 == 0 {
                search_short_lived = true;
            } else {
                search_short_lived = false;
                if self.sample_dma_reuse_queue2_rd_pos != self.sample_dma_reuse_queue2_wr_pos {
                    // Allocate a DMA from reuse queue 2, unless full.
                    dma_index = self.sample_dma_reuse_queue2[self.sample_dma_reuse_queue2_rd_pos as usize] as usize;
                    self.sample_dma_reuse_queue2_rd_pos = self.sample_dma_reuse_queue2_rd_pos.wrapping_add(1);
                    has_dma = true;
                }
            }
        } else {
            search_short_lived = true;
        }

        if search_short_lived {
            // `again`: the note's last buffer, then the short-lived ones from the first.
            let mut idx = *dma_index_ref as usize;
            let mut i = 0usize;
            loop {
                let dma = self.sample_dmas.get(idx).copied().unwrap_or_default();
                let buffer_pos = dev_addr.wrapping_sub(dma.dev_addr) as i32;
                if 0 <= buffer_pos && (buffer_pos as u32) <= (dma.size as u32).wrapping_sub(size) {
                    // We already have DMA for this memory range.
                    if dma.ttl == 0 {
                        // Move the DMA out of the reuse queue, by swapping it with the read pos,
                        // and then incrementing the read pos.
                        let rd = self.sample_dma_reuse_queue1_rd_pos as usize;
                        if dma.reuse_index as usize != rd {
                            let moved = self.sample_dma_reuse_queue1[rd];
                            self.sample_dma_reuse_queue1[dma.reuse_index as usize] = moved;
                            self.sample_dmas[moved as usize].reuse_index = dma.reuse_index;
                        }
                        self.sample_dma_reuse_queue1_rd_pos = self.sample_dma_reuse_queue1_rd_pos.wrapping_add(1);
                    }
                    self.sample_dmas[idx].ttl = 2;
                    return dma.ram_addr + (dev_addr - dma.dev_addr);
                }
                idx = i;
                i += 1;
                if i > self.sample_dma_list_size1 as usize {
                    break;
                }
            }
        }

        if !has_dma {
            if self.sample_dma_reuse_queue1_rd_pos == self.sample_dma_reuse_queue1_wr_pos {
                return 0;
            }
            // Allocate a DMA from reuse queue 1.
            dma_index = self.sample_dma_reuse_queue1[self.sample_dma_reuse_queue1_rd_pos as usize] as usize;
            self.sample_dma_reuse_queue1_rd_pos = self.sample_dma_reuse_queue1_rd_pos.wrapping_add(1);
        }

        let dma = &mut self.sample_dmas[dma_index];
        let transfer = dma.size as u32;
        let dma_dev_addr = dev_addr & !0xF;
        dma.ttl = 3;
        dma.dev_addr = dma_dev_addr;
        dma.size_unused = transfer as u16;
        let ram_addr = dma.ram_addr;
        self.dma(dma_dev_addr, ram_addr, transfer, medium);
        self.cur_audio_frame_dma_count += 1;
        *dma_index_ref = dma_index as u8;
        (dev_addr - dma_dev_addr) + ram_addr
    }

    /// `AudioLoad_InitSampleDmaBuffers`.
    pub fn init_sample_dma_buffers(&mut self, _num_notes: i32) {
        self.sample_dma_buf_size = self.sample_dma_buf_size1;
        let n = 4 * self.num_notes as usize * self.audio_buffer_parameters.spec_unk4 as usize;
        self.misc_pool.alloc(n as u32 * SIZEOF_SAMPLE_DMA);
        self.sample_dmas = vec![SampleDma::default(); n];
        let t2 = 3 * self.num_notes * self.audio_buffer_parameters.spec_unk4 as i32;
        for _ in 0..t2 {
            let a = self.misc_pool.alloc(self.sample_dma_buf_size as u32);
            if a == 0 {
                break;
            }
            let dma = &mut self.sample_dmas[self.sample_dma_count as usize];
            *dma = SampleDma { ram_addr: a, dev_addr: 0, size_unused: 0, size: self.sample_dma_buf_size as u16, unused: 0, reuse_index: 0, ttl: 0 };
            self.sample_dma_count += 1;
        }
        for i in 0..self.sample_dma_count as usize {
            self.sample_dma_reuse_queue1[i] = i as u8;
            self.sample_dmas[i].reuse_index = i as u8;
        }
        for i in self.sample_dma_count as usize..0x100 {
            self.sample_dma_reuse_queue1[i] = 0;
        }
        self.sample_dma_reuse_queue1_rd_pos = 0;
        self.sample_dma_reuse_queue1_wr_pos = self.sample_dma_count as u8;
        self.sample_dma_list_size1 = self.sample_dma_count;
        self.sample_dma_buf_size = self.sample_dma_buf_size2;

        for _ in 0..self.num_notes {
            let a = self.misc_pool.alloc(self.sample_dma_buf_size as u32);
            if a == 0 {
                break;
            }
            let dma = &mut self.sample_dmas[self.sample_dma_count as usize];
            *dma = SampleDma { ram_addr: a, dev_addr: 0, size_unused: 0, size: self.sample_dma_buf_size as u16, unused: 0, reuse_index: 0, ttl: 0 };
            self.sample_dma_count += 1;
        }
        for i in self.sample_dma_list_size1 as usize..self.sample_dma_count as usize {
            self.sample_dma_reuse_queue2[i - self.sample_dma_list_size1 as usize] = i as u8;
            self.sample_dmas[i].reuse_index = (i - self.sample_dma_list_size1 as usize) as u8;
        }
        for i in self.sample_dma_count as usize..0x100 {
            self.sample_dma_reuse_queue2[i] = self.sample_dma_list_size1 as u8;
        }
        self.sample_dma_reuse_queue2_rd_pos = 0;
        self.sample_dma_reuse_queue2_wr_pos = (self.sample_dma_count - self.sample_dma_list_size1) as u8;
    }

    /// `AudioLoad_IsFontLoadComplete`.
    pub fn is_font_load_complete(&self, font_id: i32) -> bool {
        if font_id == 0xFF {
            true
        } else if self.font_load_status.get(font_id as usize).copied().unwrap_or(0) >= LOAD_STATUS_COMPLETE {
            true
        } else {
            self.font_load_status.get(self.get_real_table_index(FONT_TABLE, font_id as u32) as usize).copied().unwrap_or(0) >= LOAD_STATUS_COMPLETE
        }
    }

    /// `AudioLoad_IsSeqLoadComplete`.
    pub fn is_seq_load_complete(&self, seq_id: i32) -> bool {
        if seq_id == 0xFF {
            true
        } else if self.seq_load_status.get(seq_id as usize).copied().unwrap_or(0) >= LOAD_STATUS_COMPLETE {
            true
        } else {
            self.seq_load_status.get(self.get_real_table_index(SEQUENCE_TABLE, seq_id as u32) as usize).copied().unwrap_or(0) >= LOAD_STATUS_COMPLETE
        }
    }

    /// `AudioLoad_IsSampleLoadComplete`.
    pub fn is_sample_load_complete(&self, sample_bank_id: i32) -> bool {
        if sample_bank_id == 0xFF {
            true
        } else if self.sample_font_load_status.get(sample_bank_id as usize).copied().unwrap_or(0) >= LOAD_STATUS_COMPLETE {
            true
        } else {
            self.sample_font_load_status.get(self.get_real_table_index(SAMPLE_TABLE, sample_bank_id as u32) as usize).copied().unwrap_or(0) >= LOAD_STATUS_COMPLETE
        }
    }

    /// `AudioLoad_SetFontLoadStatus`.
    pub fn set_font_load_status(&mut self, font_id: i32, load_status: u8) {
        if font_id != 0xFF {
            if let Some(s) = self.font_load_status.get_mut(font_id as usize) {
                if *s != LOAD_STATUS_PERMANENTLY_LOADED {
                    *s = load_status;
                }
            }
        }
    }

    /// `AudioLoad_SetSeqLoadStatus`.
    pub fn set_seq_load_status(&mut self, seq_id: i32, load_status: u8) {
        if seq_id != 0xFF {
            if let Some(s) = self.seq_load_status.get_mut(seq_id as usize) {
                if *s != LOAD_STATUS_PERMANENTLY_LOADED {
                    *s = load_status;
                }
            }
        }
    }

    /// `AudioLoad_SetSampleFontLoadStatusAndApplyCaches`.
    pub fn set_sample_font_load_status_and_apply_caches(&mut self, sample_bank_id: i32, load_status: u8) {
        if sample_bank_id != 0xFF {
            let i = sample_bank_id as usize;
            if self.sample_font_load_status[i] != LOAD_STATUS_PERMANENTLY_LOADED {
                self.sample_font_load_status[i] = load_status;
            }
            if self.sample_font_load_status[i] == LOAD_STATUS_PERMANENTLY_LOADED || self.sample_font_load_status[i] == LOAD_STATUS_COMPLETE {
                self.apply_sample_bank_cache(sample_bank_id);
            }
        }
    }

    /// `AudioLoad_SetSampleFontLoadStatus`.
    pub fn set_sample_font_load_status(&mut self, sample_bank_id: i32, load_status: u8) {
        if sample_bank_id != 0xFF && self.sample_font_load_status[sample_bank_id as usize] != LOAD_STATUS_PERMANENTLY_LOADED {
            self.sample_font_load_status[sample_bank_id as usize] = load_status;
        }
    }

    /// `AudioLoad_InitTable`.
    fn init_table(table: &mut AudioTable, rom_addr: u32, unk_medium_param: u16) {
        table.unk_medium_param = unk_medium_param as i16;
        table.rom_addr = rom_addr;
        for e in table.entries.iter_mut() {
            if e.size != 0 && e.medium as u8 == MEDIUM_CART {
                e.rom_addr += rom_addr;
            }
        }
    }

    /// `((u16*)gAudioCtx.sequenceFontTable)[seqId]`.
    pub fn seq_font_table_index(&self, seq_id: i32) -> u32 {
        self.ram.u16(self.statics.sequence_font_table + 2 * seq_id as u32) as u32
    }

    /// `gAudioCtx.sequenceFontTable[index]`.
    pub fn seq_font_table_byte(&self, index: u32) -> u8 {
        self.ram.u8(self.statics.sequence_font_table + index)
    }

    /// `AudioLoad_SyncLoadSeqFonts`.
    pub fn sync_load_seq_fonts(&mut self, seq_id: i32, out_default_font_id: &mut u32) -> u32 {
        if seq_id >= self.num_sequences as i32 {
            return 0;
        }
        let mut font_id = 0xFF;
        let mut index = self.seq_font_table_index(seq_id);
        let mut num_fonts = self.seq_font_table_byte(index) as i32;
        index += 1;
        let mut font_data = 0;
        while num_fonts > 0 {
            font_id = self.seq_font_table_byte(index) as u32;
            index += 1;
            font_data = self.sync_load_font(font_id);
            num_fonts -= 1;
        }
        *out_default_font_id = font_id;
        font_data
    }

    /// `AudioLoad_SyncLoadSeqParts`.
    pub fn sync_load_seq_parts(&mut self, seq_id: i32, arg1: i32) {
        if seq_id < self.num_sequences as i32 {
            if arg1 & 2 != 0 {
                let mut f = 0;
                self.sync_load_seq_fonts(seq_id, &mut f);
            }
            if arg1 & 1 != 0 {
                self.sync_load_seq(seq_id);
            }
        }
    }

    /// `AudioLoad_SyncLoadSample`.
    pub fn sync_load_sample(&mut self, sample: u32, font_id: i32) -> i32 {
        let s = self.ram.sample(sample);
        if s.is_relocated && s.medium != MEDIUM_RAM {
            let a = self.alloc_sample_cache(s.size, font_id, s.sample_addr, s.medium as i8, CACHE_PERSISTENT);
            if a == 0 {
                return -1;
            }
            if s.medium != MEDIUM_UNK {
                self.sync_dma(s.sample_addr, a, s.size, s.medium as i32);
            }
            self.ram.set_sample_medium(sample, MEDIUM_RAM);
            self.ram.set_sample_addr(sample, a);
        }
        // Missing return in the C.
        0
    }

    /// `AudioLoad_SyncLoadInstrument`.
    pub fn sync_load_instrument(&mut self, font_id: i32, inst_id: i32, drum_id: i32) -> i32 {
        if inst_id < 0x7F {
            let inst = self.get_instrument_inner(font_id, inst_id);
            if inst == 0 {
                return -1;
            }
            if self.ram.u8(inst + INST_NORMAL_RANGE_LO) != 0 {
                let s = self.ram.u32(inst + INST_LOW_PITCH_TUNED_SAMPLE);
                self.sync_load_sample(s, font_id);
            }
            let s = self.ram.u32(inst + INST_NORMAL_PITCH_TUNED_SAMPLE);
            self.sync_load_sample(s, font_id);
            if self.ram.u8(inst + INST_NORMAL_RANGE_HI) != 0x7F {
                let s = self.ram.u32(inst + INST_HIGH_PITCH_TUNED_SAMPLE);
                return self.sync_load_sample(s, font_id);
            }
        } else if inst_id == 0x7F {
            let drum = self.get_drum(font_id, drum_id);
            if drum == 0 {
                return -1;
            }
            let s = self.ram.u32(drum + DRUM_TUNED_SAMPLE);
            self.sync_load_sample(s, font_id);
            return 0;
        }
        0
    }

    /// `AudioLoad_AsyncLoad`.
    pub fn async_load(&mut self, table_type: i32, id: i32, n_chunks: i32, ret_data: i32, ret_queue: RetQueue) {
        if self.async_load_inner(table_type, id, n_chunks, ret_data, ret_queue) == 0 {
            self.send_ret(ret_queue, 0xFFFF_FFFF);
        }
    }

    fn send_ret(&mut self, q: RetQueue, msg: u32) {
        let (queue, cap) = match q {
            RetQueue::External => (&mut self.external_load_queue, 16),
            RetQueue::PreloadSample => (&mut self.preload_sample_queue, 16),
            RetQueue::ScriptLoad => (&mut self.script_load_queue, 16),
        };
        if queue.len() < cap {
            queue.push_back(msg);
        }
    }

    /// `AudioLoad_GetFontsForSequence`: the address of the font list and its length.
    pub fn get_fonts_for_sequence(&self, seq_id: i32) -> (u32, u32) {
        let index = self.seq_font_table_index(seq_id);
        let n = self.seq_font_table_byte(index) as u32;
        if n == 0 {
            return (0, 0);
        }
        (self.statics.sequence_font_table + index + 1, n)
    }

    /// `AudioLoad_DiscardSeqFonts`.
    pub fn discard_seq_fonts(&mut self, seq_id: i32) {
        let mut index = self.seq_font_table_index(seq_id);
        let mut num_fonts = self.seq_font_table_byte(index) as i32;
        index += 1;
        while num_fonts > 0 {
            num_fonts -= 1;
            let font_id = self.get_real_table_index(FONT_TABLE, self.seq_font_table_byte(index) as u32) as i32;
            index += 1;
            if self.search_permanent_cache(FONT_TABLE, font_id) == 0 {
                self.load_discard_font(font_id);
                self.set_font_load_status(font_id, LOAD_STATUS_NOT_LOADED);
            }
        }
    }

    /// `AudioLoad_DiscardFont`.
    pub fn load_discard_font(&mut self, font_id: i32) {
        let pool = &mut self.font_cache;
        if font_id == pool.temporary.entries[0].id as i32 {
            pool.temporary.entries[0].id = -1;
        } else if font_id == pool.temporary.entries[1].id as i32 {
            pool.temporary.entries[1].id = -1;
        }
        for i in 0..pool.persistent.num_entries as usize {
            if font_id == pool.persistent.entries[i].id as i32 {
                pool.persistent.entries[i].id = -1;
            }
        }
        self.heap_discard_font(font_id);
    }

    /// `AudioLoad_SyncInitSeqPlayer`.
    pub fn sync_init_seq_player(&mut self, player_idx: i32, seq_id: i32, arg2: i32) -> i32 {
        if self.reset_timer != 0 {
            return 0;
        }
        self.seq_players[player_idx as usize].skip_ticks = 0;
        self.sync_init_seq_player_internal(player_idx, seq_id, arg2);
        // Intentionally missing return (see the C).
        0
    }

    /// `AudioLoad_SyncInitSeqPlayerSkipTicks`.
    pub fn sync_init_seq_player_skip_ticks(&mut self, player_idx: i32, seq_id: i32, skip_ticks: i32) -> i32 {
        if self.reset_timer != 0 {
            return 0;
        }
        self.seq_players[player_idx as usize].skip_ticks = skip_ticks;
        self.sync_init_seq_player_internal(player_idx, seq_id, 0);
        0
    }

    /// `AudioLoad_SyncInitSeqPlayerInternal`.
    pub fn sync_init_seq_player_internal(&mut self, player_idx: i32, seq_id: i32, _arg2: i32) -> i32 {
        if seq_id >= self.num_sequences as i32 {
            return 0;
        }
        let p = player_idx as usize;
        self.sequence_player_disable(p);

        let mut font_id = 0xFF;
        let mut index = self.seq_font_table_index(seq_id);
        let mut num_fonts = self.seq_font_table_byte(index) as i32;
        index += 1;
        while num_fonts > 0 {
            font_id = self.seq_font_table_byte(index) as u32;
            index += 1;
            self.sync_load_font(font_id);
            num_fonts -= 1;
        }

        let seq_data = self.sync_load_seq(seq_id);
        if seq_data == 0 {
            return 0;
        }

        self.reset_sequence_player(p);
        let default_font = self.get_real_table_index(FONT_TABLE, font_id) as u8;
        let sp = &mut self.seq_players[p];
        sp.seq_id = seq_id as u8;
        sp.default_font = default_font;
        sp.seq_data = seq_data;
        sp.enabled = true;
        sp.script_state.set_pc(seq_data);
        sp.script_state.set_depth(0);
        sp.delay = 0;
        sp.finished = false;
        sp.player_idx = player_idx as i8;
        self.skip_forward_sequence(p);
        // @bug (game): missing return (but the return value is not used so it's not UB)
        0
    }

    /// `AudioLoad_SyncLoadSeq`.
    pub fn sync_load_seq(&mut self, seq_id: i32) -> u32 {
        let real = self.get_real_table_index(SEQUENCE_TABLE, seq_id as u32) as usize;
        if self.seq_load_status[real] == LOAD_STATUS_IN_PROGRESS {
            return 0;
        }
        let mut did_allocate = false;
        self.sync_load(SEQUENCE_TABLE, seq_id as u32, &mut did_allocate)
    }

    /// `AudioLoad_GetSampleBank`.
    pub fn get_sample_bank(&mut self, sample_bank_id: u32, out_medium: &mut u32) -> u32 {
        self.try_sync_load_sample_bank(sample_bank_id, out_medium, true)
    }

    /// `AudioLoad_TrySyncLoadSampleBank`.
    pub fn try_sync_load_sample_bank(&mut self, sample_bank_id: u32, out_medium: &mut u32, no_load: bool) -> u32 {
        let real = self.get_real_table_index(SAMPLE_TABLE, sample_bank_id);
        let ram_addr = self.load_search_caches(SAMPLE_TABLE, real as i32);
        if ram_addr != 0 {
            if self.sample_font_load_status[real as usize] != LOAD_STATUS_IN_PROGRESS {
                self.set_sample_font_load_status(real as i32, LOAD_STATUS_COMPLETE);
            }
            *out_medium = MEDIUM_RAM as u32;
            return ram_addr;
        }
        let cache_policy = self.sample_bank_table.entries[sample_bank_id as usize].cache_policy;
        if cache_policy == 4 || no_load {
            *out_medium = self.sample_bank_table.entries[sample_bank_id as usize].medium as u32;
            return self.sample_bank_table.entries[real as usize].rom_addr;
        }
        let mut did_allocate = false;
        let ram_addr = self.sync_load(SAMPLE_TABLE, sample_bank_id, &mut did_allocate);
        if ram_addr != 0 {
            *out_medium = MEDIUM_RAM as u32;
            return ram_addr;
        }
        *out_medium = self.sample_bank_table.entries[sample_bank_id as usize].medium as u32;
        self.sample_bank_table.entries[real as usize].rom_addr
    }

    /// `AudioLoad_SyncLoadFont`.
    pub fn sync_load_font(&mut self, font_id: u32) -> u32 {
        let real = self.get_real_table_index(FONT_TABLE, font_id) as usize;
        if self.font_load_status[real] == LOAD_STATUS_IN_PROGRESS {
            return 0;
        }
        let f = self.sound_font_list[real];
        let (b1, b2) = (f.sample_bank_id1 as i32, f.sample_bank_id2 as i32);
        let mut reloc = SampleBankRelocInfo { sample_bank_id1: b1, sample_bank_id2: b2, ..Default::default() };
        if b1 != 0xFF {
            let mut m = 0;
            reloc.base_addr1 = self.try_sync_load_sample_bank(b1 as u32, &mut m, false);
            reloc.medium1 = m;
        }
        if b2 != 0xFF {
            let mut m = 0;
            reloc.base_addr2 = self.try_sync_load_sample_bank(b2 as u32, &mut m, false);
            reloc.medium2 = m;
        }
        let mut did_allocate = false;
        let font_data = self.sync_load(FONT_TABLE, font_id, &mut did_allocate);
        if font_data == 0 {
            return 0;
        }
        if did_allocate {
            self.relocate_font_and_preload_samples(real as i32, font_data, &reloc, false);
        }
        font_data
    }

    fn table(&self, table_type: i32) -> &AudioTable {
        match table_type {
            SEQUENCE_TABLE => &self.sequence_table,
            FONT_TABLE => &self.sound_font_table,
            _ => &self.sample_bank_table,
        }
    }

    /// `AudioLoad_SyncLoad`.
    pub fn sync_load(&mut self, table_type: i32, id: u32, did_allocate: &mut bool) -> u32 {
        let real_id = self.get_real_table_index(table_type, id);
        let mut ram_addr = self.load_search_caches(table_type, real_id as i32);
        let load_status;
        if ram_addr != 0 {
            *did_allocate = false;
            load_status = LOAD_STATUS_COMPLETE;
        } else {
            let t = self.table(table_type);
            let size = align16(t.entries[real_id as usize].size);
            let medium = t.entries[id as usize].medium as i32;
            let cache_policy = t.entries[id as usize].cache_policy;
            let rom_addr = t.entries[real_id as usize].rom_addr;
            match cache_policy {
                0 => {
                    ram_addr = self.alloc_permanent(table_type, real_id as i32, size);
                    if ram_addr == 0 {
                        return 0;
                    }
                }
                1 => {
                    ram_addr = self.alloc_cached(table_type, size as i32, CACHE_PERSISTENT, real_id as i32);
                    if ram_addr == 0 {
                        return 0;
                    }
                }
                2 => {
                    ram_addr = self.alloc_cached(table_type, size as i32, CACHE_TEMPORARY, real_id as i32);
                    if ram_addr == 0 {
                        return 0;
                    }
                }
                3 | 4 => {
                    ram_addr = self.alloc_cached(table_type, size as i32, CACHE_EITHER, real_id as i32);
                    if ram_addr == 0 {
                        return 0;
                    }
                }
                _ => {}
            }
            *did_allocate = true;
            if medium != MEDIUM_UNK as i32 {
                self.sync_dma(rom_addr, ram_addr, size, medium);
            }
            load_status = if cache_policy == 0 { LOAD_STATUS_PERMANENTLY_LOADED } else { LOAD_STATUS_COMPLETE };
        }
        match table_type {
            SEQUENCE_TABLE => self.set_seq_load_status(real_id as i32, load_status),
            FONT_TABLE => self.set_font_load_status(real_id as i32, load_status),
            SAMPLE_TABLE => self.set_sample_font_load_status_and_apply_caches(real_id as i32, load_status),
            _ => {}
        }
        ram_addr
    }

    /// `AudioLoad_GetRealTableIndex`.
    pub fn get_real_table_index(&self, table_type: i32, id: u32) -> u32 {
        let t = self.table(table_type);
        match t.entries.get(id as usize) {
            Some(e) if e.size == 0 => e.rom_addr,
            _ => id,
        }
    }

    /// `AudioLoad_SearchCaches`.
    pub fn load_search_caches(&mut self, table_type: i32, id: i32) -> u32 {
        let a = self.search_permanent_cache(table_type, id);
        if a != 0 {
            return a;
        }
        self.search_caches(table_type, CACHE_EITHER, id)
    }

    /// `AudioLoad_RelocateFont`: turns the font's offsets into RAM addresses in place.
    pub fn relocate_font(&mut self, font_id: i32, font_data_start: u32, reloc: &SampleBankRelocInfo) {
        let f = self.sound_font_list[font_id as usize];
        let num_drums = f.num_drums as u32;
        let mut num_instruments = f.num_instruments as u32;
        let num_sfx = f.num_sfx as u32;
        let reloc_to_ram = |offset: u32| offset.wrapping_add(font_data_start);

        // Drums relocation: the first u32 is an offset to a list of offsets to the drums.
        let sound_list_offset = self.ram.u32(font_data_start);
        if sound_list_offset != 0 && num_drums != 0 {
            let list = reloc_to_ram(sound_list_offset);
            self.ram.set_u32(font_data_start, list);
            for i in 0..num_drums {
                let sound_offset = self.ram.u32(list + 4 * i);
                // Some drum data entries are empty, represented by an offset of 0.
                if sound_offset != 0 {
                    let drum = reloc_to_ram(sound_offset);
                    self.ram.set_u32(list + 4 * i, drum);
                    // The drum may be in the list multiple times and already relocated.
                    if self.ram.u8(drum + DRUM_IS_RELOCATED) == 0 {
                        self.relocate_sample(drum + DRUM_TUNED_SAMPLE, font_data_start, reloc);
                        let env = self.ram.u32(drum + DRUM_ENVELOPE);
                        self.ram.set_u32(drum + DRUM_ENVELOPE, reloc_to_ram(env));
                        self.ram.set_u8(drum + DRUM_IS_RELOCATED, 1);
                    }
                }
            }
        }

        // Sound effects relocation: the second u32 is an offset to the first sound effect.
        let sound_list_offset = self.ram.u32(font_data_start + 4);
        if sound_list_offset != 0 && num_sfx != 0 {
            let list = reloc_to_ram(sound_list_offset);
            self.ram.set_u32(font_data_start + 4, list);
            for i in 0..num_sfx {
                let sfx = list + SIZEOF_SOUND_EFFECT * i;
                if self.ram.u32(sfx) != 0 {
                    self.relocate_sample(sfx, font_data_start, reloc);
                }
            }
        }

        // Instruments relocation. Instrument Id 126 and above is reserved.
        if num_instruments > 126 {
            num_instruments = 126;
        }
        for i in 2..2 + num_instruments {
            let slot = font_data_start + 4 * i;
            if self.ram.u32(slot) != 0 {
                let inst = reloc_to_ram(self.ram.u32(slot));
                self.ram.set_u32(slot, inst);
                if self.ram.u8(inst + INST_IS_RELOCATED) == 0 {
                    if self.ram.u8(inst + INST_NORMAL_RANGE_LO) != 0 {
                        self.relocate_sample(inst + INST_LOW_PITCH_TUNED_SAMPLE, font_data_start, reloc);
                    }
                    self.relocate_sample(inst + INST_NORMAL_PITCH_TUNED_SAMPLE, font_data_start, reloc);
                    if self.ram.u8(inst + INST_NORMAL_RANGE_HI) != 0x7F {
                        self.relocate_sample(inst + INST_HIGH_PITCH_TUNED_SAMPLE, font_data_start, reloc);
                    }
                    let env = self.ram.u32(inst + INST_ENVELOPE);
                    self.ram.set_u32(inst + INST_ENVELOPE, reloc_to_ram(env));
                    self.ram.set_u8(inst + INST_IS_RELOCATED, 1);
                }
            }
        }

        // Store the relocated pointers.
        let f = &mut self.sound_font_list[font_id as usize];
        f.drums = self.ram.u32(font_data_start);
        f.sound_effects = self.ram.u32(font_data_start + 4);
        f.instruments = font_data_start + 8;
    }

    /// `AudioLoad_SyncDma`.
    pub fn sync_dma(&mut self, dev_addr: u32, ram_addr: u32, size: u32, medium: i32) {
        let size = align16(size);
        self.dma(dev_addr, ram_addr, size, medium);
    }

    /// `AudioLoad_Dma`: the PI copies at once.
    pub fn dma(&mut self, dev_addr: u32, ram_addr: u32, size: u32, medium: i32) -> i32 {
        if self.reset_timer > 16 {
            return -1;
        }
        match medium as u8 {
            MEDIUM_CART => {}
            // The disk drive is stubbed out (`driveHandle` is never set).
            MEDIUM_DISK_DRIVE => return 0,
            _ => return 0,
        }
        let size = if size % 0x10 != 0 { align16(size) } else { size };
        self.cart.dma(dev_addr, &mut self.ram, ram_addr, size as usize);
        0
    }

    /// `AudioLoad_SyncLoadSimple`.
    pub fn sync_load_simple(&mut self, table_type: i32, font_id: u32) {
        let mut d = false;
        self.sync_load(table_type, font_id, &mut d);
    }

    /// `AudioLoad_AsyncLoadInner`.
    pub fn async_load_inner(&mut self, table_type: i32, id: i32, n_chunks: i32, ret_data: i32, ret_queue: RetQueue) -> u32 {
        let real_id = self.get_real_table_index(table_type, id as u32);
        let status = match table_type {
            SEQUENCE_TABLE => self.seq_load_status[real_id as usize],
            FONT_TABLE => self.font_load_status[real_id as usize],
            _ => self.sample_font_load_status[real_id as usize],
        };
        if status == LOAD_STATUS_IN_PROGRESS {
            return 0;
        }
        let mut ram_addr = self.load_search_caches(table_type, real_id as i32);
        let mut load_status;
        if ram_addr != 0 {
            load_status = LOAD_STATUS_COMPLETE;
            self.send_ret(ret_queue, mk_async_msg(ret_data as u32, 0, 0, LOAD_STATUS_NOT_LOADED as u32));
        } else {
            let t = self.table(table_type);
            let size = align16(t.entries[real_id as usize].size);
            let medium = t.entries[id as usize].medium as i32;
            let cache_policy = t.entries[id as usize].cache_policy;
            let dev_addr = t.entries[real_id as usize].rom_addr;
            load_status = LOAD_STATUS_COMPLETE;
            match cache_policy {
                0 => {
                    ram_addr = self.alloc_permanent(table_type, real_id as i32, size);
                    if ram_addr == 0 {
                        return 0;
                    }
                    load_status = LOAD_STATUS_PERMANENTLY_LOADED;
                }
                1 => {
                    ram_addr = self.alloc_cached(table_type, size as i32, CACHE_PERSISTENT, real_id as i32);
                    if ram_addr == 0 {
                        return 0;
                    }
                }
                2 => {
                    ram_addr = self.alloc_cached(table_type, size as i32, CACHE_TEMPORARY, real_id as i32);
                    if ram_addr == 0 {
                        return 0;
                    }
                }
                3 | 4 => {
                    ram_addr = self.alloc_cached(table_type, size as i32, CACHE_EITHER, real_id as i32);
                    if ram_addr == 0 {
                        return 0;
                    }
                }
                _ => {}
            }
            if medium == MEDIUM_UNK as i32 {
                self.start_async_load(dev_addr, ram_addr, size, medium, n_chunks, ret_queue, mk_async_msg(ret_data as u32, table_type as u32, id as u32, load_status as u32));
            } else {
                self.start_async_load(dev_addr, ram_addr, size, medium, n_chunks, ret_queue, mk_async_msg(ret_data as u32, table_type as u32, real_id, load_status as u32));
            }
            load_status = LOAD_STATUS_IN_PROGRESS;
        }
        match table_type {
            SEQUENCE_TABLE => self.set_seq_load_status(real_id as i32, load_status),
            FONT_TABLE => self.set_font_load_status(real_id as i32, load_status),
            SAMPLE_TABLE => self.set_sample_font_load_status_and_apply_caches(real_id as i32, load_status),
            _ => {}
        }
        ram_addr
    }

    /// `AudioLoad_ProcessLoads`.
    pub fn process_loads(&mut self, reset_status: i32) {
        self.process_slow_loads(reset_status);
        self.process_sample_preloads(reset_status);
        self.process_async_loads(reset_status);
    }

    /// `AudioLoad_InitSoundFont`.
    fn init_sound_font(&mut self, font_id: usize) {
        let e = self.sound_font_table.entries[font_id];
        let f = &mut self.sound_font_list[font_id];
        f.sample_bank_id1 = ((e.short_data1 >> 8) & 0xFF) as u8;
        f.sample_bank_id2 = (e.short_data1 & 0xFF) as u8;
        f.num_instruments = ((e.short_data2 >> 8) & 0xFF) as u8;
        f.num_drums = (e.short_data2 & 0xFF) as u8;
        f.num_sfx = e.short_data3 as u16;
    }

    /// `AudioLoad_Init(NULL, 0)`, on NTSC (`osTvType`).
    pub fn load_init(&mut self) {
        self.reset_timer = 0;
        // OS_TV_NTSC (audio/internal/load.c)
        self.max_tempo_tv_type_factors = 16.713f32;
        self.refresh_rate = 60;

        self.cmd_wr_pos = 0;
        self.cmd_rd_pos = 0;
        self.cmd_queue_finished = 0;
        self.ai_buf_lengths = [0xA0; 3];
        self.total_task_count = 0;
        self.rsp_task_index = 0;
        self.cur_ai_buf_index = 0;
        self.sound_mode = SOUND_OUTPUT_STEREO;
        self.cur_audio_frame_dma_count = 0;
        self.sample_dma_count = 0;

        // gAudioHeap and gAudioHeapInitSizes (session_config.c)
        let hs = self.tables.heap_sizes;
        self.audio_heap = place::AUDIO_HEAP;
        self.audio_heap_size = align16(hs.audio_heap - 0x100);
        let permanent_pool_size = hs.sfx_seq_size + hs.sfx_soundfont_1_size + hs.sfx_soundfont_2_size;
        let ai_buffers_size = AIBUF_SIZE as u32 * 3;
        let soundfont_list_size = hs.num_soundfonts * SIZEOF_SOUND_FONT;
        let init_pool_size = align16(permanent_pool_size + ai_buffers_size + soundfont_list_size);
        let permanent_pool_size = align16(permanent_pool_size);
        self.ram.fill(self.audio_heap, self.audio_heap_size as usize, 0);

        // Main Pool Split (split entirety of audio heap into initPool and sessionPool)
        self.init_main_pools(init_pool_size);

        // Initialize the audio interface buffers
        for i in 0..3 {
            self.ai_buffers[i] = alloc_zeroed(&mut self.ram, &mut self.init_pool, AIBUF_SIZE as u32);
        }

        // Set audio tables pointers
        self.sequence_table = AudioTable::parse(&self.tables.sequence_table);
        self.sound_font_table = AudioTable::parse(&self.tables.sound_font_table);
        self.sample_bank_table = AudioTable::parse(&self.tables.sample_bank_table);
        self.num_sequences = self.sequence_table.num_entries as u16;

        self.audio_reset_spec_id_to_load = 0;
        self.reset_status = 1; // Set reset to immediately initialize the audio heap
        self.reset_step();

        // Initialize audio tables (the segments' ROM starts, from the pack's files)
        let [seq_rom, bank_rom, table_rom] = self.segment_rom_starts;
        Self::init_table(&mut self.sequence_table, seq_rom, 0);
        Self::init_table(&mut self.sound_font_table, bank_rom, 0);
        Self::init_table(&mut self.sample_bank_table, table_rom, 0);
        let num_fonts = self.sound_font_table.num_entries as usize;
        self.init_pool.alloc(num_fonts as u32 * SIZEOF_SOUND_FONT);
        self.sound_font_list = vec![SoundFont::default(); num_fonts];
        for i in 0..num_fonts {
            self.init_sound_font(i);
        }

        let ram_addr = self.init_pool.alloc(permanent_pool_size);
        let pps = if ram_addr == 0 { 0 } else { permanent_pool_size };
        self.permanent_pool.init(ram_addr, pps);
        self.audio_context_initialized = true;
        self.task_start_queue.push_back(self.total_task_count as u32);
    }

    /// `AudioLoad_InitSlowLoads`.
    pub fn init_slow_loads(&mut self) {
        self.slow_loads[0].state = SLOW_LOAD_STATE_WAITING;
        self.slow_loads[1].state = SLOW_LOAD_STATE_WAITING;
    }

    /// Writes an `s8*` a load reports to.
    pub fn set_io_port(&mut self, port: IoPort, v: i8) {
        match port {
            IoPort::Player(p, i) => self.seq_players[p].sound_script_io[i] = v,
            IoPort::Channel(c, i) => self.channels[c].sound_script_io[i] = v,
        }
    }

    /// `AudioLoad_SlowLoadSample`.
    pub fn slow_load_sample(&mut self, font_id: i32, inst_id: i32, status: IoPort) -> i32 {
        let sample = self.get_font_sample(font_id, inst_id);
        if sample == 0 {
            self.set_io_port(status, 0);
            return -1;
        }
        let s = self.ram.sample(sample);
        if s.medium == MEDIUM_RAM {
            self.set_io_port(status, 2);
            return 0;
        }
        let pos = self.slow_load_pos as usize;
        if self.slow_loads[pos].state == SLOW_LOAD_STATE_DONE {
            self.slow_loads[pos].state = SLOW_LOAD_STATE_WAITING;
        }
        self.slow_loads[pos].sample_addr = s.sample_addr;
        self.slow_loads[pos].status = Some(status);
        let cur = self.alloc_sample_cache(s.size, font_id, s.sample_addr, s.medium as i8, CACHE_TEMPORARY);
        self.slow_loads[pos].cur_ram_addr = cur;
        if cur == 0 {
            if s.medium == MEDIUM_UNK || s.codec == CODEC_S16_INMEMORY {
                self.set_io_port(status, 0);
            } else {
                self.set_io_port(status, 3);
            }
            return -1;
        }
        let sl = &mut self.slow_loads[pos];
        sl.state = SLOW_LOAD_STATE_START;
        sl.bytes_remaining = align16(s.size) as i32;
        sl.ram_addr = sl.cur_ram_addr;
        sl.cur_dev_addr = s.sample_addr;
        sl.medium = s.medium;
        sl.seq_or_font_id = font_id as u8;
        sl.inst_id = inst_id as u16;
        self.slow_load_pos ^= 1;
        0
    }

    /// `AudioLoad_GetFontSample`.
    pub fn get_font_sample(&mut self, font_id: i32, inst_id: i32) -> u32 {
        if inst_id < 0x80 {
            let inst = self.get_instrument_inner(font_id, inst_id);
            if inst == 0 {
                return 0;
            }
            self.ram.u32(inst + INST_NORMAL_PITCH_TUNED_SAMPLE)
        } else if inst_id < 0x100 {
            let drum = self.get_drum(font_id, inst_id - 0x80);
            if drum == 0 {
                return 0;
            }
            self.ram.u32(drum + DRUM_TUNED_SAMPLE)
        } else {
            let sfx = self.get_sound_effect(font_id, inst_id - 0x100);
            if sfx == 0 {
                return 0;
            }
            self.ram.u32(sfx)
        }
    }

    /// `AudioLoad_FinishSlowLoad`.
    fn finish_slow_load(&mut self, i: usize) {
        let sl = self.slow_loads[i];
        if sl.sample_addr == 0 {
            return;
        }
        let sample = self.get_font_sample(sl.seq_or_font_id as i32, sl.inst_id as i32);
        if sample == 0 {
            return;
        }
        self.slow_loads[i].sample_addr = self.ram.sample(sample).sample_addr;
        self.ram.set_sample_addr(sample, sl.ram_addr);
        self.ram.set_sample_medium(sample, MEDIUM_RAM);
    }

    /// `AudioLoad_ProcessSlowLoads`.
    pub fn process_slow_loads(&mut self, reset_status: i32) {
        for i in 0..2 {
            let state = self.slow_loads[i].state;
            if state == SLOW_LOAD_STATE_LOADING || state == SLOW_LOAD_STATE_START {
                if state == SLOW_LOAD_STATE_LOADING && reset_status != 0 {
                    self.slow_loads[i].state = SLOW_LOAD_STATE_DONE;
                    continue;
                }
                self.slow_loads[i].state = SLOW_LOAD_STATE_LOADING;
                let sl = self.slow_loads[i];
                if sl.bytes_remaining == 0 {
                    self.finish_slow_load(i);
                    self.slow_loads[i].state = SLOW_LOAD_STATE_DONE;
                    if let Some(p) = sl.status {
                        self.set_io_port(p, 1);
                    }
                } else if sl.bytes_remaining < 0x400 {
                    if sl.medium != MEDIUM_UNK {
                        self.dma(sl.cur_dev_addr, sl.cur_ram_addr, sl.bytes_remaining as u32, sl.medium as i32);
                    }
                    self.slow_loads[i].bytes_remaining = 0;
                } else {
                    if sl.medium != MEDIUM_UNK {
                        self.dma(sl.cur_dev_addr, sl.cur_ram_addr, 0x400, sl.medium as i32);
                    }
                    let sl = &mut self.slow_loads[i];
                    sl.bytes_remaining -= 0x400;
                    sl.cur_ram_addr += 0x400;
                    sl.cur_dev_addr += 0x400;
                }
            }
        }
    }

    /// `AudioLoad_SlowLoadSeq`.
    pub fn slow_load_seq(&mut self, seq_id: i32, ram_addr: u32, status: IoPort) -> i32 {
        if seq_id >= self.num_sequences as i32 {
            self.set_io_port(status, 0);
            return -1;
        }
        let seq_id = self.get_real_table_index(SEQUENCE_TABLE, seq_id as u32) as usize;
        let e = self.sequence_table.entries[seq_id];
        let pos = self.slow_load_pos as usize;
        let sl = &mut self.slow_loads[pos];
        if sl.state == SLOW_LOAD_STATE_DONE {
            sl.state = SLOW_LOAD_STATE_WAITING;
        }
        sl.sample_addr = 0;
        sl.status = Some(status);
        sl.cur_ram_addr = ram_addr;
        sl.state = SLOW_LOAD_STATE_START;
        sl.bytes_remaining = align16(e.size) as i32;
        sl.ram_addr = ram_addr;
        sl.cur_dev_addr = e.rom_addr;
        sl.medium = e.medium as u8;
        sl.seq_or_font_id = seq_id as u8;
        self.slow_load_pos ^= 1;
        0
    }

    /// `AudioLoad_InitAsyncLoads`.
    pub fn init_async_loads(&mut self) {
        for l in self.async_loads.iter_mut() {
            l.status = 0;
        }
    }

    /// `AudioLoad_StartAsyncLoad`.
    #[allow(clippy::too_many_arguments)]
    pub fn start_async_load(&mut self, dev_addr: u32, ram_addr: u32, size: u32, medium: i32, n_chunks: i32, ret_queue: RetQueue, ret_msg: u32) -> Option<usize> {
        let i = self.async_loads.iter().position(|l| l.status == 0)?;
        let l = &mut self.async_loads[i];
        l.status = 1;
        l.cur_dev_addr = dev_addr;
        l.ram_addr = ram_addr;
        l.cur_ram_addr = ram_addr;
        l.bytes_remaining = size;
        l.chunk_size = if n_chunks == 0 {
            0x1000
        } else if n_chunks == 1 {
            size
        } else {
            (align256(size as i32 / n_chunks) as u32).max(0x100)
        };
        l.ret_queue = ret_queue;
        l.delay = 3;
        l.medium = medium as i8;
        l.ret_msg = ret_msg;
        l.dma_done = false;
        Some(i)
    }

    /// `AudioLoad_ProcessAsyncLoads` (the unknown medium's queue is never used).
    pub fn process_async_loads(&mut self, reset_status: i32) {
        if self.reset_timer == 1 {
            return;
        }
        for i in 0..self.async_loads.len() {
            if self.async_loads[i].status == 1 && self.async_loads[i].medium as u8 != MEDIUM_UNK {
                self.process_async_load(i, reset_status);
            }
        }
    }

    /// `AudioLoad_FinishAsyncLoad`.
    fn finish_async_load(&mut self, i: usize) {
        let ret_msg = self.async_loads[i].ret_msg;
        let (table_type, id, status) = ((ret_msg >> 16) as u8 as i32, (ret_msg >> 8) as u8 as i32, ret_msg as u8);
        match table_type {
            SEQUENCE_TABLE => self.set_seq_load_status(id, status),
            SAMPLE_TABLE => self.set_sample_font_load_status_and_apply_caches(id, status),
            FONT_TABLE => {
                let f = self.sound_font_list[id as usize];
                let (b1, b2) = (f.sample_bank_id1 as i32, f.sample_bank_id2 as i32);
                let mut reloc = SampleBankRelocInfo { sample_bank_id1: b1, sample_bank_id2: b2, ..Default::default() };
                if b1 != 0xFF {
                    let mut m = 0;
                    reloc.base_addr1 = self.get_sample_bank(b1 as u32, &mut m);
                    reloc.medium1 = m;
                }
                if b2 != 0xFF {
                    let mut m = 0;
                    reloc.base_addr2 = self.get_sample_bank(b2 as u32, &mut m);
                    reloc.medium2 = m;
                }
                self.set_font_load_status(id, status);
                let ram = self.async_loads[i].ram_addr;
                self.relocate_font_and_preload_samples(id, ram, &reloc, true);
            }
            _ => {}
        }
        let q = self.async_loads[i].ret_queue;
        self.async_loads[i].status = 0;
        self.send_ret(q, ret_msg);
    }

    /// `AudioLoad_ProcessAsyncLoad`.
    fn process_async_load(&mut self, i: usize, reset_status: i32) {
        let l = &mut self.async_loads[i];
        if l.delay >= 2 {
            l.delay -= 1;
            return;
        }
        if l.delay == 1 {
            l.delay = 0;
        } else if reset_status != 0 {
            // Await the previous DMA response synchronously, then return.
            l.dma_done = false;
            l.status = 0;
            return;
        } else if !l.dma_done {
            // If the previous DMA step isn't done, return.
            return;
        } else {
            l.dma_done = false;
        }
        if l.bytes_remaining == 0 {
            self.finish_async_load(i);
            return;
        }
        let l = self.async_loads[i];
        if l.bytes_remaining < l.chunk_size {
            self.async_dma(i, l.bytes_remaining);
            self.async_loads[i].bytes_remaining = 0;
            return;
        }
        self.async_dma(i, l.chunk_size);
        let l = &mut self.async_loads[i];
        l.bytes_remaining -= l.chunk_size;
        l.cur_dev_addr += l.chunk_size;
        l.cur_ram_addr += l.chunk_size;
    }

    /// `AudioLoad_AsyncDma`.
    fn async_dma(&mut self, i: usize, size: u32) {
        let size = align16(size);
        let l = self.async_loads[i];
        self.dma(l.cur_dev_addr, l.cur_ram_addr, size, l.medium as i32);
        self.async_loads[i].dma_done = true;
    }

    /// `AudioLoad_RelocateSample`.
    pub fn relocate_sample(&mut self, tuned_sample: u32, font_data: u32, reloc: &SampleBankRelocInfo) {
        let sample_ptr = self.ram.u32(tuned_sample);
        // If this has not already been relocated
        if sample_ptr <= K0BASE {
            let sample = sample_ptr.wrapping_add(font_data);
            self.ram.set_u32(tuned_sample, sample);
            let s = self.ram.sample(sample);
            // If the sample exists and has not already been relocated
            if s.size != 0 && !s.is_relocated {
                self.ram.set_sample_loop(sample, s.loop_addr.wrapping_add(font_data));
                self.ram.set_sample_book(sample, s.book.wrapping_add(font_data));
                // Resolve the sample medium 2-bit bitfield into a real value.
                match s.medium {
                    0 => {
                        self.ram.set_sample_addr(sample, s.sample_addr.wrapping_add(reloc.base_addr1));
                        self.ram.set_sample_medium(sample, reloc.medium1 as u8);
                    }
                    1 => {
                        self.ram.set_sample_addr(sample, s.sample_addr.wrapping_add(reloc.base_addr2));
                        self.ram.set_sample_medium(sample, reloc.medium2 as u8);
                    }
                    // Invalid? This leaves the medium as MEDIUM_CART and MEDIUM_DISK_DRIVE
                    // respectively, and the sampleAddr unrelocated.
                    _ => {}
                }
                self.ram.set_sample_relocated(sample, true);
                let s = self.ram.sample(sample);
                if s.unk_bit26 && s.medium != MEDIUM_RAM {
                    let n = self.num_used_samples as usize;
                    if n < self.used_samples.len() {
                        self.used_samples[n] = sample;
                    }
                    self.num_used_samples += 1;
                }
            }
        }
    }

    /// `AudioLoad_RelocateFontAndPreloadSamples`.
    pub fn relocate_font_and_preload_samples(&mut self, font_id: i32, font_data: u32, reloc: &SampleBankRelocInfo, is_async: bool) {
        let preload_in_progress = self.preload_sample_stack_top != 0;
        self.num_used_samples = 0;
        self.relocate_font(font_id, font_data, reloc);

        for i in 0..self.num_used_samples.min(128) as usize {
            if self.preload_sample_stack_top == 120 {
                break;
            }
            let sample = self.used_samples[i];
            let s = self.ram.sample(sample);
            let cache = if is_async { CACHE_TEMPORARY } else { CACHE_PERSISTENT };
            let sample_ram_addr = if s.medium as u32 == reloc.medium1 {
                self.alloc_sample_cache(s.size, reloc.sample_bank_id1, s.sample_addr, s.medium as i8, cache)
            } else if s.medium as u32 == reloc.medium2 {
                self.alloc_sample_cache(s.size, reloc.sample_bank_id2, s.sample_addr, s.medium as i8, cache)
            } else if s.medium == MEDIUM_DISK_DRIVE {
                self.alloc_sample_cache(s.size, 0xFE, s.sample_addr, s.medium as i8, cache)
            } else {
                0
            };
            if sample_ram_addr == 0 {
                continue;
            }
            if !is_async {
                if s.medium != MEDIUM_UNK {
                    self.sync_dma(s.sample_addr, sample_ram_addr, s.size, s.medium as i32);
                }
                self.ram.set_sample_addr(sample, sample_ram_addr);
                self.ram.set_sample_medium(sample, MEDIUM_RAM);
            } else {
                let top = self.preload_sample_stack_top as usize;
                self.preload_sample_stack[top] =
                    AudioPreloadReq { sample, ram_addr: sample_ram_addr, encoded_info: ((top as u32) << 24) | 0xFF_FFFF, is_free: false, end_and_medium_key: s.sample_addr + s.size + s.medium as u32 };
                self.preload_sample_stack_top += 1;
            }
        }
        self.num_used_samples = 0;

        if self.preload_sample_stack_top != 0 && !preload_in_progress {
            let top = self.preload_sample_stack[self.preload_sample_stack_top as usize - 1];
            let s = self.ram.sample(top.sample);
            let n_chunks = (s.size >> 12) + 1;
            self.start_async_load(s.sample_addr, top.ram_addr, s.size, s.medium as i32, n_chunks as i32, RetQueue::PreloadSample, top.encoded_info);
        }
    }

    /// `AudioLoad_ProcessSamplePreloads`.
    pub fn process_sample_preloads(&mut self, reset_status: i32) -> bool {
        if self.preload_sample_stack_top > 0 {
            if reset_status != 0 {
                // Clear result queue and preload stack and return.
                self.preload_sample_queue.pop_front();
                self.preload_sample_stack_top = 0;
                return false;
            }
            let Some(preload_index) = self.preload_sample_queue.pop_front() else {
                // Previous preload is not done yet.
                return false;
            };
            let preload_index = (preload_index >> 24) as usize;
            let p = self.preload_sample_stack[preload_index];
            if !p.is_free {
                let s = self.ram.sample(p.sample);
                let key = s.sample_addr + s.size + s.medium as u32;
                if key == p.end_and_medium_key {
                    // Change storage for sample to the preloaded version.
                    self.ram.set_sample_addr(p.sample, p.ram_addr);
                    self.ram.set_sample_medium(p.sample, MEDIUM_RAM);
                }
                self.preload_sample_stack[preload_index].is_free = true;
            }
            // Pop requests with isFree = true off the stack, as far as possible, and dispatch
            // the next DMA.
            while self.preload_sample_stack_top > 0 {
                let top = self.preload_sample_stack_top as usize - 1;
                let p = self.preload_sample_stack[top];
                if p.is_free {
                    self.preload_sample_stack_top -= 1;
                    continue;
                }
                let s = self.ram.sample(p.sample);
                let n_chunks = (s.size >> 12) + 1;
                let key = s.sample_addr + s.size + s.medium as u32;
                if key != p.end_and_medium_key {
                    self.preload_sample_stack[top].is_free = true;
                    self.preload_sample_stack_top -= 1;
                } else {
                    self.start_async_load(s.sample_addr, p.ram_addr, s.size, s.medium as i32, n_chunks as i32, RetQueue::PreloadSample, p.encoded_info);
                    break;
                }
            }
        }
        true
    }

    /// `AudioLoad_AddUsedSample`.
    fn add_used_sample(&mut self, tuned_sample: u32) {
        let sample = self.ram.u32(tuned_sample);
        let s = self.ram.sample(sample);
        if s.size != 0 && s.unk_bit26 && s.medium != MEDIUM_RAM {
            let n = self.num_used_samples as usize;
            if n < self.used_samples.len() {
                self.used_samples[n] = sample;
            }
            self.num_used_samples += 1;
        }
    }

    /// `AudioLoad_PreloadSamplesForFont`.
    pub fn preload_samples_for_font(&mut self, font_id: i32, is_async: bool, reloc: &SampleBankRelocInfo) {
        let preload_in_progress = self.preload_sample_stack_top != 0;
        self.num_used_samples = 0;
        let f = self.sound_font_list[font_id as usize];
        for i in 0..f.num_instruments as i32 {
            let inst = self.get_instrument_inner(font_id, i);
            if inst != 0 {
                if self.ram.u8(inst + INST_NORMAL_RANGE_LO) != 0 {
                    self.add_used_sample(inst + INST_LOW_PITCH_TUNED_SAMPLE);
                }
                if self.ram.u8(inst + INST_NORMAL_RANGE_HI) != 0x7F {
                    self.add_used_sample(inst + INST_HIGH_PITCH_TUNED_SAMPLE);
                }
                self.add_used_sample(inst + INST_NORMAL_PITCH_TUNED_SAMPLE);
            }
        }
        for i in 0..f.num_drums as i32 {
            let drum = self.get_drum(font_id, i);
            if drum != 0 {
                self.add_used_sample(drum + DRUM_TUNED_SAMPLE);
            }
        }
        for i in 0..f.num_sfx as i32 {
            let sfx = self.get_sound_effect(font_id, i);
            if sfx != 0 {
                self.add_used_sample(sfx);
            }
        }
        if self.num_used_samples == 0 {
            return;
        }
        let mut addr = 0;
        for i in 0..self.num_used_samples.min(128) as usize {
            if self.preload_sample_stack_top == 120 {
                break;
            }
            let sample = self.used_samples[i];
            let s = self.ram.sample(sample);
            if s.medium == MEDIUM_RAM {
                continue;
            }
            let cache = if is_async { CACHE_TEMPORARY } else { CACHE_PERSISTENT };
            // `addr` keeps its last value when neither medium matches (the C's uninitialized
            // local, which holds the previous iteration's).
            if s.medium as u32 == reloc.medium1 {
                addr = self.alloc_sample_cache(s.size, reloc.sample_bank_id1, s.sample_addr, s.medium as i8, cache);
            } else if s.medium as u32 == reloc.medium2 {
                addr = self.alloc_sample_cache(s.size, reloc.sample_bank_id2, s.sample_addr, s.medium as i8, cache);
            }
            if addr == 0 {
                continue;
            }
            if !is_async {
                if s.medium != MEDIUM_UNK {
                    self.sync_dma(s.sample_addr, addr, s.size, s.medium as i32);
                }
                self.ram.set_sample_addr(sample, addr);
                self.ram.set_sample_medium(sample, MEDIUM_RAM);
            } else {
                let top = self.preload_sample_stack_top as usize;
                self.preload_sample_stack[top] =
                    AudioPreloadReq { sample, ram_addr: addr, encoded_info: ((top as u32) << 24) | 0xFF_FFFF, is_free: false, end_and_medium_key: s.sample_addr + s.size + s.medium as u32 };
                self.preload_sample_stack_top += 1;
            }
        }
        self.num_used_samples = 0;
        if self.preload_sample_stack_top != 0 && !preload_in_progress {
            let top = self.preload_sample_stack[self.preload_sample_stack_top as usize - 1];
            let s = self.ram.sample(top.sample);
            let n_chunks = (s.size >> 12) + 1;
            self.start_async_load(s.sample_addr, top.ram_addr, s.size, s.medium as i32, n_chunks as i32, RetQueue::PreloadSample, top.encoded_info);
        }
    }

    /// `AudioLoad_LoadPermanentSamples`.
    pub fn load_permanent_samples(&mut self) {
        for i in 0..self.permanent_pool.num_entries as usize {
            if self.permanent_cache[i].table_type as i32 == FONT_TABLE {
                let font_id = self.get_real_table_index(FONT_TABLE, self.permanent_cache[i].id as u32) as i32;
                let f = self.sound_font_list[font_id as usize];
                let mut reloc = SampleBankRelocInfo { sample_bank_id1: f.sample_bank_id1 as i32, sample_bank_id2: f.sample_bank_id2 as i32, ..Default::default() };
                if reloc.sample_bank_id1 != 0xFF {
                    reloc.sample_bank_id1 = self.get_real_table_index(SAMPLE_TABLE, reloc.sample_bank_id1 as u32) as i32;
                    reloc.medium1 = self.sample_bank_table.entries[reloc.sample_bank_id1 as usize].medium as u32;
                }
                if reloc.sample_bank_id2 != 0xFF {
                    reloc.sample_bank_id2 = self.get_real_table_index(SAMPLE_TABLE, reloc.sample_bank_id2 as u32) as i32;
                    reloc.medium2 = self.sample_bank_table.entries[reloc.sample_bank_id2 as usize].medium as u32;
                }
                self.preload_samples_for_font(font_id, false, &reloc);
            }
        }
    }

    /// `AudioLoad_ScriptLoad`.
    pub fn script_load(&mut self, table_type: i32, id: i32, status: IoPort) {
        let i = self.script_load_index as usize;
        self.script_load_done_pointers[i] = Some(status);
        self.async_load(table_type, id, 0, i as i32, RetQueue::ScriptLoad);
        self.script_load_index += 1;
        if self.script_load_index == 0x10 {
            self.script_load_index = 0;
        }
    }

    /// `AudioLoad_ProcessScriptLoads`.
    pub fn process_script_loads(&mut self) {
        if let Some(m) = self.script_load_queue.pop_front() {
            let i = (m >> 24) as usize;
            if let Some(Some(p)) = self.script_load_done_pointers.get(i).copied() {
                self.set_io_port(p, 0);
            }
        }
    }

    /// `AudioLoad_InitScriptLoads`.
    pub fn init_script_loads(&mut self) {
        self.script_load_queue.clear();
    }
}
