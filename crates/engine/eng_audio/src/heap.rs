//! `heap.c`: the audio heap's pools and caches, the ADSR decay table, the filters, the
//! reset steps and `AudioHeap_Init`.

use crate::context::*;
use crate::ram::Ram;

impl AudioAllocPool {
    /// `AudioHeap_Alloc`.
    pub fn alloc(&mut self, size: u32) -> u32 {
        let aligned = align16(size);
        let ram_addr = self.cur_ram_addr;
        if self.start_ram_addr as i64 + self.size as i64 >= self.cur_ram_addr as i64 + aligned as i64 {
            self.cur_ram_addr += aligned;
        } else {
            return 0;
        }
        self.num_entries += 1;
        ram_addr
    }

    /// `AudioHeap_InitPool`.
    pub fn init(&mut self, ram_addr: u32, size: u32) {
        self.start_ram_addr = align16(ram_addr);
        self.cur_ram_addr = self.start_ram_addr;
        self.size = size as i32 - (ram_addr & 0xF) as i32;
        self.num_entries = 0;
    }

    /// `AudioHeap_ResetPool`.
    pub fn reset(&mut self) {
        self.num_entries = 0;
        self.cur_ram_addr = self.start_ram_addr;
    }
}

/// `AudioHeap_AllocZeroed`: zeroes up to the pool's new current address.
pub fn alloc_zeroed(ram: &mut Ram, pool: &mut AudioAllocPool, size: u32) -> u32 {
    let ram_addr = pool.alloc(size);
    if ram_addr != 0 {
        ram.fill(ram_addr, (pool.cur_ram_addr - ram_addr) as usize, 0);
    }
    ram_addr
}

/// `AudioHeap_InitPersistentCache`.
fn init_persistent_cache(persistent: &mut AudioPersistentCache) {
    persistent.pool.num_entries = 0;
    persistent.num_entries = 0;
    persistent.pool.cur_ram_addr = persistent.pool.start_ram_addr;
}

/// `AudioHeap_InitTemporaryCache`.
fn init_temporary_cache(temporary: &mut AudioTemporaryCache) {
    temporary.pool.num_entries = 0;
    temporary.pool.cur_ram_addr = temporary.pool.start_ram_addr;
    temporary.next_side = 0;
    temporary.entries[0].ram_addr = temporary.pool.start_ram_addr;
    temporary.entries[1].ram_addr = temporary.pool.start_ram_addr.wrapping_add(temporary.pool.size as u32);
    temporary.entries[0].id = -1;
    temporary.entries[1].id = -1;
}

impl AudioContext {
    /// `AudioHeap_CalculateAdsrDecay`.
    pub fn calculate_adsr_decay(&self, scale_inv: f32) -> f32 {
        (256.0f32 * self.audio_buffer_parameters.updates_per_frame_inv_scaled) / scale_inv
    }

    /// `AudioHeap_InitAdsrDecayTable`.
    pub fn init_adsr_decay_table(&mut self) {
        let mut t = vec![0.0f32; 256];
        t[255] = self.calculate_adsr_decay(0.25);
        t[254] = self.calculate_adsr_decay(0.33);
        t[253] = self.calculate_adsr_decay(0.5);
        t[252] = self.calculate_adsr_decay(0.66);
        t[251] = self.calculate_adsr_decay(0.75);
        for (i, v) in t.iter_mut().enumerate().take(251).skip(128) {
            *v = self.calculate_adsr_decay((251 - i as i32) as f32);
        }
        for (i, v) in t.iter_mut().enumerate().take(128).skip(16) {
            *v = self.calculate_adsr_decay((4 * (143 - i as i32)) as f32);
        }
        for (i, v) in t.iter_mut().enumerate().take(16).skip(1) {
            *v = self.calculate_adsr_decay((60 * (23 - i as i32)) as f32);
        }
        t[0] = 0.0;
        self.adsr_decay_table = t;
    }

    /// `AudioHeap_ResetLoadStatus`.
    pub fn reset_load_status(&mut self) {
        for s in self.font_load_status.iter_mut().chain(self.sample_font_load_status.iter_mut()).chain(self.seq_load_status.iter_mut()) {
            if *s != LOAD_STATUS_PERMANENTLY_LOADED {
                *s = LOAD_STATUS_NOT_LOADED;
            }
        }
    }

    /// `AudioHeap_DiscardFont`.
    pub fn heap_discard_font(&mut self, font_id: i32) {
        for i in 0..self.num_notes as usize {
            if self.notes[i].playback_state.font_id as i32 == font_id {
                let ps = self.notes[i].playback_state;
                if ps.unk_04 == 0 && ps.priority != 0 {
                    if let Some(l) = ps.parent_layer {
                        self.sequence_layers[l].enabled = false;
                        self.sequence_layers[l].finished = true;
                    }
                }
                self.note_disable(i);
                self.audio_list_remove_node(crate::playback::note_node(i));
                let d = self.pools[self.note_free_lists].disabled;
                self.audio_list_push_back(d, crate::playback::note_node(i));
            }
        }
    }

    /// `AudioHeap_ReleaseNotesForFont`.
    pub fn release_notes_for_font(&mut self, font_id: i32) {
        let inv = self.audio_buffer_parameters.updates_per_frame_inv;
        for n in self.notes.iter_mut().take(self.num_notes as usize) {
            let ps = &mut n.playback_state;
            if ps.font_id as i32 == font_id && ps.priority != 0 && ps.adsr.state() == ADSR_STATE_DECAY {
                ps.priority = 1;
                ps.adsr.fade_out_vel = inv;
                ps.adsr.set_release(true);
            }
        }
    }

    /// `AudioHeap_DiscardSequence`.
    pub fn discard_sequence(&mut self, seq_id: i32) {
        for i in 0..self.audio_buffer_parameters.num_sequence_players as usize {
            if self.seq_players[i].enabled && self.seq_players[i].seq_id as i32 == seq_id {
                self.sequence_player_disable(i);
            }
        }
    }

    fn cache_mut(&mut self, table_type: i32) -> &mut AudioCache {
        match table_type {
            SEQUENCE_TABLE => &mut self.seq_cache,
            FONT_TABLE => &mut self.font_cache,
            _ => &mut self.sample_bank_cache,
        }
    }

    fn cache(&self, table_type: i32) -> &AudioCache {
        match table_type {
            SEQUENCE_TABLE => &self.seq_cache,
            FONT_TABLE => &self.font_cache,
            _ => &self.sample_bank_cache,
        }
    }

    fn load_status(&self, table_type: i32, id: i32) -> u8 {
        let id = id as usize;
        match table_type {
            SEQUENCE_TABLE => self.seq_load_status.get(id).copied().unwrap_or(0),
            FONT_TABLE => self.font_load_status.get(id).copied().unwrap_or(0),
            _ => self.sample_font_load_status.get(id).copied().unwrap_or(0),
        }
    }

    fn set_load_status_raw(&mut self, table_type: i32, id: i32, v: u8) {
        let id = id as usize;
        let s = match table_type {
            SEQUENCE_TABLE => self.seq_load_status.get_mut(id),
            FONT_TABLE => self.font_load_status.get_mut(id),
            _ => self.sample_font_load_status.get_mut(id),
        };
        if let Some(s) = s {
            *s = v;
        }
    }

    /// `AudioHeap_PopPersistentCache`.
    pub fn pop_persistent_cache(&mut self, table_type: i32) {
        let persistent = self.cache(table_type).persistent;
        if persistent.num_entries == 0 {
            return;
        }
        let last = persistent.entries[persistent.num_entries as usize - 1];
        {
            let p = &mut self.cache_mut(table_type).persistent;
            p.pool.cur_ram_addr = last.ram_addr;
            p.pool.num_entries -= 1;
        }
        if table_type == SAMPLE_TABLE {
            self.discard_sample_bank(last.id as i32);
        }
        if table_type == FONT_TABLE {
            self.heap_discard_font(last.id as i32);
        }
        self.set_load_status_raw(table_type, last.id as i32, LOAD_STATUS_NOT_LOADED);
        self.cache_mut(table_type).persistent.num_entries -= 1;
    }

    /// `AudioHeap_InitMainPools`.
    pub fn init_main_pools(&mut self, init_pool_size: u32) {
        self.init_pool.init(self.audio_heap, init_pool_size);
        self.session_pool.init(self.audio_heap + init_pool_size, self.audio_heap_size - init_pool_size);
        self.external_pool.start_ram_addr = 0;
    }

    /// `AudioHeap_InitSessionPools`.
    fn init_session_pools(&mut self, split: AudioSessionPoolSplit) {
        self.session_pool.cur_ram_addr = self.session_pool.start_ram_addr;
        let a = self.session_pool.alloc(split.misc_pool_size);
        self.misc_pool.init(a, split.misc_pool_size);
        let a = self.session_pool.alloc(split.cache_pool_size);
        self.cache_pool.init(a, split.cache_pool_size);
    }

    /// `AudioHeap_InitCachePools`.
    fn init_cache_pools(&mut self, split: AudioCachePoolSplit) {
        self.cache_pool.cur_ram_addr = self.cache_pool.start_ram_addr;
        let a = self.cache_pool.alloc(split.persistent_common_pool_size);
        self.persistent_common_pool.init(a, split.persistent_common_pool_size);
        let a = self.cache_pool.alloc(split.temporary_common_pool_size);
        self.temporary_common_pool.init(a, split.temporary_common_pool_size);
    }

    /// `AudioHeap_InitPersistentPoolsAndCaches`.
    fn init_persistent_pools_and_caches(&mut self, split: AudioCommonPoolSplit) {
        self.persistent_common_pool.cur_ram_addr = self.persistent_common_pool.start_ram_addr;
        let a = self.persistent_common_pool.alloc(split.seq_cache_size);
        self.seq_cache.persistent.pool.init(a, split.seq_cache_size);
        let a = self.persistent_common_pool.alloc(split.font_cache_size);
        self.font_cache.persistent.pool.init(a, split.font_cache_size);
        let a = self.persistent_common_pool.alloc(split.sample_bank_cache_size);
        self.sample_bank_cache.persistent.pool.init(a, split.sample_bank_cache_size);
        init_persistent_cache(&mut self.seq_cache.persistent);
        init_persistent_cache(&mut self.font_cache.persistent);
        init_persistent_cache(&mut self.sample_bank_cache.persistent);
    }

    /// `AudioHeap_InitTemporaryPoolsAndCaches`.
    fn init_temporary_pools_and_caches(&mut self, split: AudioCommonPoolSplit) {
        self.temporary_common_pool.cur_ram_addr = self.temporary_common_pool.start_ram_addr;
        let a = self.temporary_common_pool.alloc(split.seq_cache_size);
        self.seq_cache.temporary.pool.init(a, split.seq_cache_size);
        let a = self.temporary_common_pool.alloc(split.font_cache_size);
        self.font_cache.temporary.pool.init(a, split.font_cache_size);
        let a = self.temporary_common_pool.alloc(split.sample_bank_cache_size);
        self.sample_bank_cache.temporary.pool.init(a, split.sample_bank_cache_size);
        init_temporary_cache(&mut self.seq_cache.temporary);
        init_temporary_cache(&mut self.font_cache.temporary);
        init_temporary_cache(&mut self.sample_bank_cache.temporary);
    }

    /// Whether any enabled note plays `font_id` (the loops in `AudioHeap_AllocCached`).
    fn font_in_use(&self, font_id: i16) -> bool {
        self.notes.iter().take(self.num_notes as usize).any(|n| n.playback_state.font_id as i16 == font_id && n.note_sub_eu.enabled)
    }

    /// Whether an enabled player plays `seq_id`.
    fn seq_in_use(&self, seq_id: i16) -> bool {
        self.seq_players.iter().take(self.audio_buffer_parameters.num_sequence_players as usize).any(|p| p.enabled && p.seq_id as i16 == seq_id)
    }

    /// `AudioHeap_AllocCached`.
    pub fn alloc_cached(&mut self, table_type: i32, size: i32, cache: i32, id: i32) -> u32 {
        if cache == CACHE_TEMPORARY {
            let temporary = self.cache(table_type).temporary;
            if temporary.pool.size < size {
                return 0;
            }
            let status = |s: &Self, i: usize| if temporary.entries[i].id == -1 { LOAD_STATUS_NOT_LOADED } else { s.load_status(table_type, temporary.entries[i].id as i32) };
            let mut status0 = status(self, 0);
            let mut status1 = status(self, 1);

            if table_type == FONT_TABLE {
                if status0 == LOAD_STATUS_MAYBE_DISCARDABLE && !self.font_in_use(temporary.entries[0].id) {
                    self.set_font_load_status(temporary.entries[0].id as i32, LOAD_STATUS_DISCARDABLE);
                    status0 = LOAD_STATUS_DISCARDABLE;
                }
                if status1 == LOAD_STATUS_MAYBE_DISCARDABLE && !self.font_in_use(temporary.entries[1].id) {
                    self.set_font_load_status(temporary.entries[1].id as i32, LOAD_STATUS_DISCARDABLE);
                    status1 = LOAD_STATUS_DISCARDABLE;
                }
            }

            'done: {
                let tc = &mut self.cache_mut(table_type).temporary;
                if status0 == LOAD_STATUS_NOT_LOADED {
                    tc.next_side = 0;
                } else if status1 == LOAD_STATUS_NOT_LOADED {
                    tc.next_side = 1;
                } else if status0 == LOAD_STATUS_DISCARDABLE && status1 == LOAD_STATUS_DISCARDABLE {
                    // Use the opposite side from last time.
                } else if status0 == LOAD_STATUS_DISCARDABLE {
                    tc.next_side = 0;
                } else if status1 == LOAD_STATUS_DISCARDABLE {
                    tc.next_side = 1;
                } else {
                    // Check if there is a side which isn't in active use, if so, evict that one.
                    if table_type == SEQUENCE_TABLE {
                        if status0 == LOAD_STATUS_COMPLETE && !self.seq_in_use(temporary.entries[0].id) {
                            self.cache_mut(table_type).temporary.next_side = 0;
                            break 'done;
                        }
                        if status1 == LOAD_STATUS_COMPLETE && !self.seq_in_use(temporary.entries[1].id) {
                            self.cache_mut(table_type).temporary.next_side = 1;
                            break 'done;
                        }
                    } else if table_type == FONT_TABLE {
                        if status0 == LOAD_STATUS_COMPLETE && !self.font_in_use(temporary.entries[0].id) {
                            self.cache_mut(table_type).temporary.next_side = 0;
                            break 'done;
                        }
                        if status1 == LOAD_STATUS_COMPLETE && !self.font_in_use(temporary.entries[1].id) {
                            self.cache_mut(table_type).temporary.next_side = 1;
                            break 'done;
                        }
                    }
                    // No such luck. Evict the side that wasn't chosen last time, except if it is
                    // being loaded into.
                    let tc = &mut self.cache_mut(table_type).temporary;
                    if tc.next_side == 0 {
                        if status0 == LOAD_STATUS_IN_PROGRESS {
                            if status1 == LOAD_STATUS_IN_PROGRESS {
                                return 0;
                            }
                            tc.next_side = 1;
                        }
                    } else if status1 == LOAD_STATUS_IN_PROGRESS {
                        if status0 == LOAD_STATUS_IN_PROGRESS {
                            return 0;
                        }
                        tc.next_side = 0;
                    }
                }
            }

            let side = self.cache(table_type).temporary.next_side as usize;
            let old_id = self.cache(table_type).temporary.entries[side].id;
            if old_id != -1 {
                if table_type == SAMPLE_TABLE {
                    self.discard_sample_bank(old_id as i32);
                }
                self.set_load_status_raw(table_type, old_id as i32, LOAD_STATUS_NOT_LOADED);
                if table_type == FONT_TABLE {
                    self.heap_discard_font(old_id as i32);
                }
            }

            let temporary_ram_addr;
            match side {
                0 => {
                    let tc = &mut self.cache_mut(table_type).temporary;
                    tc.entries[0].ram_addr = tc.pool.start_ram_addr;
                    tc.entries[0].id = id as i16;
                    tc.entries[0].size = size as u32;
                    tc.pool.cur_ram_addr = tc.pool.start_ram_addr + size as u32;
                    let other = tc.entries[1];
                    if other.id != -1 && other.ram_addr < tc.pool.cur_ram_addr {
                        if table_type == SAMPLE_TABLE {
                            self.discard_sample_bank(other.id as i32);
                        }
                        self.set_load_status_raw(table_type, other.id as i32, LOAD_STATUS_NOT_LOADED);
                        match table_type {
                            SEQUENCE_TABLE => self.discard_sequence(other.id as i32),
                            FONT_TABLE => self.heap_discard_font(other.id as i32),
                            _ => {}
                        }
                        let tc = &mut self.cache_mut(table_type).temporary;
                        tc.entries[1].id = -1;
                        tc.entries[1].ram_addr = tc.pool.start_ram_addr + tc.pool.size as u32;
                    }
                    temporary_ram_addr = self.cache(table_type).temporary.entries[0].ram_addr;
                }
                1 => {
                    let tc = &mut self.cache_mut(table_type).temporary;
                    tc.entries[1].ram_addr = (tc.pool.start_ram_addr + tc.pool.size as u32 - size as u32) & !0xF;
                    tc.entries[1].id = id as i16;
                    tc.entries[1].size = size as u32;
                    let other = tc.entries[0];
                    if other.id != -1 && tc.entries[1].ram_addr < tc.pool.cur_ram_addr {
                        if table_type == SAMPLE_TABLE {
                            self.discard_sample_bank(other.id as i32);
                        }
                        self.set_load_status_raw(table_type, other.id as i32, LOAD_STATUS_NOT_LOADED);
                        match table_type {
                            SEQUENCE_TABLE => self.discard_sequence(other.id as i32),
                            FONT_TABLE => self.heap_discard_font(other.id as i32),
                            _ => {}
                        }
                        let tc = &mut self.cache_mut(table_type).temporary;
                        tc.entries[0].id = -1;
                        tc.pool.cur_ram_addr = tc.pool.start_ram_addr;
                    }
                    temporary_ram_addr = self.cache(table_type).temporary.entries[1].ram_addr;
                }
                _ => return 0,
            }
            self.cache_mut(table_type).temporary.next_side ^= 1;
            return temporary_ram_addr;
        }

        let c = self.cache_mut(table_type);
        let persistent_ram_addr = c.persistent.pool.alloc(size as u32);
        let n = c.persistent.num_entries as usize;
        if n < c.persistent.entries.len() {
            c.persistent.entries[n].ram_addr = persistent_ram_addr;
        }
        if persistent_ram_addr == 0 {
            return match cache {
                CACHE_EITHER => self.alloc_cached(table_type, size, CACHE_TEMPORARY, id),
                _ => 0,
            };
        }
        let c = self.cache_mut(table_type);
        let n = c.persistent.num_entries as usize;
        c.persistent.entries[n].id = id as i16;
        c.persistent.entries[n].size = size as u32;
        c.persistent.num_entries += 1;
        c.persistent.entries[n].ram_addr
    }

    /// `AudioHeap_SearchCaches`.
    pub fn search_caches(&mut self, table_type: i32, cache: i32, id: i32) -> u32 {
        // Always search the permanent cache in addition to the regular ones.
        let ram_addr = self.search_permanent_cache(table_type, id);
        if ram_addr != 0 {
            return ram_addr;
        }
        if cache == CACHE_PERMANENT {
            return 0;
        }
        self.search_regular_caches(table_type, cache, id)
    }

    /// `AudioHeap_SearchRegularCaches`.
    pub fn search_regular_caches(&mut self, table_type: i32, cache: i32, id: i32) -> u32 {
        let c = self.cache_mut(table_type);
        if cache == CACHE_TEMPORARY {
            if c.temporary.entries[0].id as i32 == id {
                c.temporary.next_side = 1;
                return c.temporary.entries[0].ram_addr;
            } else if c.temporary.entries[1].id as i32 == id {
                c.temporary.next_side = 0;
                return c.temporary.entries[1].ram_addr;
            } else {
                return 0;
            }
        }
        for i in 0..c.persistent.num_entries as usize {
            if c.persistent.entries[i].id as i32 == id {
                return c.persistent.entries[i].ram_addr;
            }
        }
        if cache == CACHE_EITHER {
            return self.search_caches(table_type, CACHE_TEMPORARY, id);
        }
        0
    }

    /// `AudioHeap_ClearFilter`.
    pub fn clear_filter(&mut self, filter: u32) {
        for i in 0..8 {
            self.ram.set_s16(filter + 2 * i, 0);
        }
    }

    /// `AudioHeap_LoadLowPassFilter`.
    pub fn load_low_pass_filter(&mut self, filter: u32, cutoff: i32) {
        for i in 0..8 {
            let v = self.tables.low_pass_filter_data.get((8 * cutoff + i) as usize).copied().unwrap_or(0);
            self.ram.set_s16(filter + 2 * i as u32, v);
        }
    }

    /// `AudioHeap_LoadHighPassFilter`.
    pub fn load_high_pass_filter(&mut self, filter: u32, cutoff: i32) {
        for i in 0..8 {
            let v = self.tables.high_pass_filter_data.get((8 * (cutoff - 1) + i) as usize).copied().unwrap_or(0);
            self.ram.set_s16(filter + 2 * i as u32, v);
        }
    }

    /// `AudioHeap_LoadFilter`.
    pub fn load_filter(&mut self, filter: u32, low_pass_cutoff: i32, high_pass_cutoff: i32) {
        if low_pass_cutoff == 0 && high_pass_cutoff == 0 {
            // Identity filter
            self.load_low_pass_filter(filter, 0);
        } else if high_pass_cutoff == 0 {
            self.load_low_pass_filter(filter, low_pass_cutoff);
        } else if low_pass_cutoff == 0 {
            self.load_high_pass_filter(filter, high_pass_cutoff);
        } else {
            for i in 0..8 {
                let a = self.tables.low_pass_filter_data.get((8 * low_pass_cutoff + i) as usize).copied().unwrap_or(0) as i32;
                let b = self.tables.high_pass_filter_data.get((8 * (high_pass_cutoff - 1) + i) as usize).copied().unwrap_or(0) as i32;
                self.ram.set_s16(filter + 2 * i as u32, ((a + b) / 2) as i16);
            }
        }
    }

    /// `AudioHeap_UpdateReverbs` (its `AudioHeap_UpdateReverb` is empty).
    pub fn update_reverbs(&mut self) {}

    /// `AudioHeap_ClearCurrentAiBuffer`.
    pub fn clear_current_ai_buffer(&mut self) {
        let i = self.cur_ai_buf_index as usize;
        self.ai_buf_lengths[i] = self.audio_buffer_parameters.min_ai_buffer_length;
        self.ram.fill(self.ai_buffers[i], AIBUF_SIZE as usize, 0);
    }

    /// `AudioHeap_ResetStep`.
    pub fn reset_step(&mut self) -> bool {
        let sp24 = if self.audio_buffer_parameters.spec_unk4 == 2 { 2 } else { 1 };
        match self.reset_status {
            5 => {
                for i in 0..self.audio_buffer_parameters.num_sequence_players as usize {
                    self.sequence_player_disable_as_finished(i);
                }
                self.audio_reset_fade_out_frames_left = 2 / sp24;
                self.reset_status -= 1;
            }
            4 => {
                if self.audio_reset_fade_out_frames_left != 0 {
                    self.audio_reset_fade_out_frames_left -= 1;
                    self.update_reverbs();
                } else {
                    let inv = self.audio_buffer_parameters.updates_per_frame_inv;
                    for n in self.notes.iter_mut().take(self.num_notes as usize) {
                        if n.note_sub_eu.enabled && n.playback_state.adsr.state() != ADSR_STATE_DISABLED {
                            n.playback_state.adsr.fade_out_vel = inv;
                            n.playback_state.adsr.set_release(true);
                        }
                    }
                    self.audio_reset_fade_out_frames_left = 8 / sp24;
                    self.reset_status -= 1;
                }
            }
            3 => {
                if self.audio_reset_fade_out_frames_left != 0 {
                    self.audio_reset_fade_out_frames_left -= 1;
                    self.update_reverbs();
                } else {
                    self.audio_reset_fade_out_frames_left = 2 / sp24;
                    self.reset_status -= 1;
                }
            }
            2 => {
                self.clear_current_ai_buffer();
                if self.audio_reset_fade_out_frames_left != 0 {
                    self.audio_reset_fade_out_frames_left -= 1;
                } else {
                    self.reset_status -= 1;
                    self.discard_sample_caches();
                    self.discard_sample_banks();
                }
            }
            1 => {
                self.heap_init();
                self.reset_status = 0;
                for i in 0..3 {
                    self.ai_buf_lengths[i] = self.audio_buffer_parameters.max_ai_buffer_length;
                    self.ram.fill(self.ai_buffers[i], AIBUF_SIZE as usize, 0);
                }
            }
            _ => {}
        }
        self.reset_status >= 3
    }

    /// `AudioHeap_Init`.
    pub fn heap_init(&mut self) {
        let spec = self.tables.specs[self.audio_reset_spec_id_to_load as usize].clone();
        self.sample_dma_count = 0;

        // audio buffer parameters
        let abp = &mut self.audio_buffer_parameters;
        abp.sampling_frequency = spec.sampling_frequency as u16;
        abp.ai_sampling_frequency = os_ai_set_frequency(spec.sampling_frequency) as u16;
        abp.samples_per_frame_target = align16(abp.sampling_frequency as u32 / self.refresh_rate as u32) as i16;
        abp.min_ai_buffer_length = abp.samples_per_frame_target - 0x10;
        abp.max_ai_buffer_length = abp.samples_per_frame_target + 0x10;
        abp.updates_per_frame = ((abp.samples_per_frame_target + 0x10) / 0xD0) + 1;
        abp.samples_per_update = (abp.samples_per_frame_target / abp.updates_per_frame) & !7;
        abp.samples_per_update_max = abp.samples_per_update + 8;
        abp.samples_per_update_min = abp.samples_per_update - 8;
        abp.resample_rate = 32000.0f32 / (abp.sampling_frequency as i32) as f32;
        abp.updates_per_frame_inv_scaled = (1.0f32 / 256.0f32) / abp.updates_per_frame as f32;
        abp.updates_per_frame_scaled = abp.updates_per_frame as f32 / 4.0f32;
        abp.updates_per_frame_inv = 1.0f32 / abp.updates_per_frame as f32;

        // SampleDma buffer size
        self.sample_dma_buf_size1 = spec.sample_dma_buf_size1 as i32;
        self.sample_dma_buf_size2 = spec.sample_dma_buf_size2 as i32;

        self.num_notes = spec.num_notes as i32;
        self.audio_buffer_parameters.num_sequence_players = (spec.num_sequence_players as i16).min(4);
        self.unk_2 = spec.unk_14;
        let tatums = self.tables.tatums_per_beat as f32;
        self.tempo_internal_to_external = (self.audio_buffer_parameters.updates_per_frame as f32 * 2880000.0f32 / tatums / self.max_tempo_tv_type_factors) as u32 as i16;

        let mut unk_2870 = self.refresh_rate as f32;
        unk_2870 *= self.audio_buffer_parameters.updates_per_frame as f32;
        unk_2870 /= self.audio_buffer_parameters.ai_sampling_frequency as f32;
        unk_2870 /= self.tempo_internal_to_external as f32;
        self.unk_2870 = unk_2870;

        let abp = &mut self.audio_buffer_parameters;
        abp.spec_unk4 = spec.unk_04 as i16;
        abp.samples_per_frame_target *= abp.spec_unk4;
        abp.max_ai_buffer_length *= abp.spec_unk4;
        abp.min_ai_buffer_length *= abp.spec_unk4;
        abp.updates_per_frame *= abp.spec_unk4;
        if abp.spec_unk4 >= 2 {
            abp.max_ai_buffer_length -= 0x10;
        }

        // Determine the length of the buffer for storing the audio command list passed to the
        // rsp audio microcode
        self.max_audio_cmds = self.num_notes * 0x10 * self.audio_buffer_parameters.updates_per_frame as i32 + spec.num_reverbs as i32 * 0x18 + 0x140;

        // Calculate sizes for various caches on the audio heap
        let persistent_size = spec.persistent_seq_cache_size + spec.persistent_font_cache_size + spec.persistent_sample_bank_cache_size + 0x10;
        let temporary_size = spec.temporary_seq_cache_size + spec.temporary_font_cache_size + spec.temporary_sample_bank_cache_size + 0x10;
        let cache_pool_size = persistent_size + temporary_size;
        let misc_pool_size = (self.session_pool.size - cache_pool_size as i32 - 0x100) as u32;

        if self.external_pool.start_ram_addr != 0 {
            self.external_pool.cur_ram_addr = self.external_pool.start_ram_addr;
        }

        // Session Pool Split (split into Cache and Misc pools)
        self.session_pool_split = AudioSessionPoolSplit { misc_pool_size, cache_pool_size };
        self.init_session_pools(self.session_pool_split);

        // Cache Pool Split (split into Persistent and Temporary pools)
        self.cache_pool_split = AudioCachePoolSplit { persistent_common_pool_size: persistent_size, temporary_common_pool_size: temporary_size };
        self.init_cache_pools(self.cache_pool_split);

        // Persistent Pool Split (split into Sequences, SoundFonts, Samples pools)
        self.persistent_common_pool_split =
            AudioCommonPoolSplit { seq_cache_size: spec.persistent_seq_cache_size, font_cache_size: spec.persistent_font_cache_size, sample_bank_cache_size: spec.persistent_sample_bank_cache_size };
        self.init_persistent_pools_and_caches(self.persistent_common_pool_split);

        // Temporary Pool Split (split into Sequences, SoundFonts, Samples pools)
        self.temporary_common_pool_split =
            AudioCommonPoolSplit { seq_cache_size: spec.temporary_seq_cache_size, font_cache_size: spec.temporary_font_cache_size, sample_bank_cache_size: spec.temporary_sample_bank_cache_size };
        self.init_temporary_pools_and_caches(self.temporary_common_pool_split);

        self.reset_load_status();

        // Initialize notes
        self.notes_addr = alloc_zeroed(&mut self.ram, &mut self.misc_pool, self.num_notes as u32 * SIZEOF_NOTE);
        self.note_init_all();
        self.init_lists();
        self.init_note_free_list();
        let n = self.audio_buffer_parameters.updates_per_frame as usize * self.num_notes as usize;
        alloc_zeroed(&mut self.ram, &mut self.misc_pool, n as u32 * SIZEOF_NOTE_SUB_EU);
        self.note_subs_eu = vec![NoteSampleState::default(); n];
        // Initialize audio binary interface command list buffers
        for i in 0..2 {
            self.abi_cmd_bufs[i] = alloc_zeroed(&mut self.ram, &mut self.misc_pool, self.max_audio_cmds as u32 * SIZEOF_ACMD);
        }

        // Initialize the decay rate table for adsr
        self.adsr_decay_table_addr = self.misc_pool.alloc(256 * 4);
        self.init_adsr_decay_table();

        // Initialize reverbs
        for r in self.synthesis_reverbs.iter_mut() {
            r.use_reverb = 0;
        }
        self.num_synthesis_reverbs = spec.num_reverbs as i8;
        for i in 0..self.num_synthesis_reverbs as usize {
            let settings = spec.reverb_settings[i];
            let tuned = self.statics.reverb_tuned_samples[i];
            let r = &mut self.synthesis_reverbs[i];
            r.downsample_rate = settings.downsample_rate;
            r.window_size = settings.window_size.wrapping_mul(64);
            r.window_size /= r.downsample_rate as u16;
            r.decay_ratio = settings.decay_ratio;
            r.volume = settings.volume as i16;
            r.unk_14 = settings.unk_6.wrapping_mul(64);
            r.unk_16 = settings.unk_8 as i16;
            r.unk_18 = 0;
            r.leak_rtl = settings.leak_rtl as i16;
            r.leak_ltr = settings.leak_ltr as i16;
            r.unk_05 = settings.unk_10;
            r.unk_08 = settings.unk_12 as i16;
            r.use_reverb = 8;
            let ws = r.window_size as u32;
            r.left_ring_buf = alloc_zeroed(&mut self.ram, &mut self.misc_pool, ws * SAMPLE_SIZE as u32);
            r.right_ring_buf = alloc_zeroed(&mut self.ram, &mut self.misc_pool, ws * SAMPLE_SIZE as u32);
            r.next_ring_buf_pos = 0;
            r.unk_20 = 0;
            r.cur_frame = 0;
            r.buf_size_per_chan = ws as i32;
            r.frames_to_ignore = 2;
            r.resample_flags = 1;
            r.tuned_sample_addr = tuned;
            // tunedSample.sample = &sample; sample.loop = &loop; tunedSample.tuning = 1.0f
            let (sample, lp) = (tuned + 8, tuned + 0x18);
            self.ram.set_u32(tuned, sample);
            self.ram.set_f32(tuned + 4, 1.0);
            // codec CODEC_REVERB, medium MEDIUM_RAM, size windowSize * SAMPLE_SIZE
            let bits = self.ram.u32(sample) & (1 << 25 | 1 << 24);
            self.ram.set_u32(sample, bits | ((CODEC_REVERB as u32) << 28) | ((MEDIUM_RAM as u32) << 26) | ((ws * SAMPLE_SIZE as u32) & 0xFF_FFFF));
            self.ram.set_sample_addr(sample, r.left_ring_buf);
            self.ram.set_sample_loop(sample, lp);
            self.ram.set_u32(lp, 0);
            self.ram.set_u32(lp + 8, 1);
            self.ram.set_u32(lp + 4, ws);

            if r.downsample_rate != 1 {
                r.unk_0e = (0x8000 / r.downsample_rate as u32) as u16;
                r.unk_30 = alloc_zeroed(&mut self.ram, &mut self.misc_pool, SIZEOF_RESAMPLE_STATE);
                r.unk_34 = alloc_zeroed(&mut self.ram, &mut self.misc_pool, SIZEOF_RESAMPLE_STATE);
                r.unk_38 = alloc_zeroed(&mut self.ram, &mut self.misc_pool, SIZEOF_RESAMPLE_STATE);
                r.unk_3c = alloc_zeroed(&mut self.ram, &mut self.misc_pool, SIZEOF_RESAMPLE_STATE);
                for j in 0..self.audio_buffer_parameters.updates_per_frame as usize {
                    let a = alloc_zeroed(&mut self.ram, &mut self.misc_pool, DMEM_2CH_SIZE as u32);
                    r.items[0][j].to_downsample_left = a;
                    r.items[0][j].to_downsample_right = a + DMEM_1CH_SIZE as u32;
                    let a = alloc_zeroed(&mut self.ram, &mut self.misc_pool, DMEM_2CH_SIZE as u32);
                    r.items[1][j].to_downsample_left = a;
                    r.items[1][j].to_downsample_right = a + DMEM_1CH_SIZE as u32;
                }
            }

            // 2 * (FILTER_BUF_PART1 + FILTER_BUF_PART2) and FILTER_SIZE
            if settings.low_pass_filter_cutoff_left != 0 {
                r.filter_left_state = alloc_zeroed(&mut self.ram, &mut self.misc_pool, 2 * (16 + 16));
                r.filter_left = self.misc_pool.alloc(16);
            } else {
                r.filter_left = 0;
            }
            if settings.low_pass_filter_cutoff_right != 0 {
                r.filter_right_state = alloc_zeroed(&mut self.ram, &mut self.misc_pool, 2 * (16 + 16));
                r.filter_right = self.misc_pool.alloc(16);
            } else {
                r.filter_right = 0;
            }
            let (fl, fr) = (r.filter_left, r.filter_right);
            if fl != 0 {
                self.load_low_pass_filter(fl, settings.low_pass_filter_cutoff_left as i32);
            }
            if fr != 0 {
                self.load_low_pass_filter(fr, settings.low_pass_filter_cutoff_right as i32);
            }
        }

        // Initialize sequence players
        self.init_sequence_players();
        for j in 0..self.audio_buffer_parameters.num_sequence_players as usize {
            self.init_sequence_player_channels(j);
            self.reset_sequence_player(j);
        }

        // Initialize two additional sample caches for individual samples
        self.init_sample_caches(spec.persistent_sample_cache_size as u32, spec.temporary_sample_cache_size as u32);
        self.init_sample_dma_buffers(self.num_notes);

        // Initialize Loads
        self.preload_sample_stack_top = 0;
        self.init_slow_loads();
        self.init_script_loads();
        self.init_async_loads();
        self.unk_4 = 0x1000;
        self.load_permanent_samples();
    }

    /// `AudioHeap_SearchPermanentCache`.
    pub fn search_permanent_cache(&self, table_type: i32, id: i32) -> u32 {
        for i in 0..self.permanent_pool.num_entries as usize {
            let e = &self.permanent_cache[i];
            if e.table_type as i32 == table_type && e.id as i32 == id {
                return e.ram_addr;
            }
        }
        0
    }

    /// `AudioHeap_AllocPermanent`.
    pub fn alloc_permanent(&mut self, table_type: i32, id: i32, size: u32) -> u32 {
        let index = self.permanent_pool.num_entries as usize;
        let ram_addr = self.permanent_pool.alloc(size);
        self.permanent_cache[index].ram_addr = ram_addr;
        if ram_addr == 0 {
            return 0;
        }
        self.permanent_cache[index].table_type = table_type as i16;
        self.permanent_cache[index].id = id as i16;
        self.permanent_cache[index].size = size;
        // @bug (game): UB, a missing return; `ramAddr` is in v0 (the decomp's AVOID_UB return).
        ram_addr
    }

    /// `AudioHeap_AllocSampleCache`.
    pub fn alloc_sample_cache(&mut self, size: u32, font_id: i32, sample_addr: u32, medium: i8, cache: i32) -> u32 {
        let entry = if cache == CACHE_TEMPORARY { self.alloc_temporary_sample_cache_entry(size) } else { self.alloc_persistent_sample_cache_entry(size) };
        if let Some((temporary, i)) = entry {
            let e = if temporary { &mut self.temporary_sample_cache.entries[i] } else { &mut self.persistent_sample_cache.entries[i] };
            // @bug (game): Should use sampleBankId, not fontId
            e.sample_bank_id = font_id as i8;
            e.sample_addr = sample_addr;
            e.orig_medium = medium;
            return e.allocated_addr;
        }
        0
    }

    /// `AudioHeap_InitSampleCaches`.
    pub fn init_sample_caches(&mut self, persistent_size: u32, temporary_size: u32) {
        let a = self.misc_pool.alloc(persistent_size);
        if a == 0 {
            self.persistent_sample_cache.pool.size = 0;
        } else {
            self.persistent_sample_cache.pool.init(a, persistent_size);
        }
        let a = self.misc_pool.alloc(temporary_size);
        if a == 0 {
            self.temporary_sample_cache.pool.size = 0;
        } else {
            self.temporary_sample_cache.pool.init(a, temporary_size);
        }
        self.persistent_sample_cache.num_entries = 0;
        self.temporary_sample_cache.num_entries = 0;
    }

    /// `AudioHeap_AllocTemporarySampleCacheEntry`: the entry's index in the temporary cache.
    pub fn alloc_temporary_sample_cache_entry(&mut self, size: u32) -> Option<(bool, usize)> {
        let cache = &mut self.temporary_sample_cache;
        let mut alloc_before = cache.pool.cur_ram_addr;
        let mut ram_addr = cache.pool.alloc(size);
        if ram_addr == 0 {
            // Reset the pool and try again. We still keep pointers to within the pool, so we
            // have to be careful to discard existing overlapping allocations further down.
            let old = cache.pool.cur_ram_addr;
            cache.pool.cur_ram_addr = cache.pool.start_ram_addr;
            ram_addr = cache.pool.alloc(size);
            if ram_addr == 0 {
                cache.pool.cur_ram_addr = old;
                return None;
            }
            alloc_before = cache.pool.start_ram_addr;
        }
        let alloc_after = cache.pool.cur_ram_addr;

        for i in 0..self.preload_sample_stack_top as usize {
            let p = self.preload_sample_stack[i];
            if !p.is_free {
                let start = p.ram_addr;
                let end = p.ram_addr + self.ram.sample(p.sample).size - 1;
                if end < alloc_before && start < alloc_before {
                    continue;
                }
                if end >= alloc_after && start >= alloc_after {
                    continue;
                }
                // Overlap, skip this preload.
                self.preload_sample_stack[i].is_free = true;
            }
        }

        let mut index = -1i32;
        for i in 0..self.temporary_sample_cache.num_entries as usize {
            let e = self.temporary_sample_cache.entries[i];
            if !e.in_use {
                continue;
            }
            let start = e.allocated_addr;
            let end = start + e.size - 1;
            if end < alloc_before && start < alloc_before {
                continue;
            }
            if end >= alloc_after && start >= alloc_after {
                continue;
            }
            // Overlap, discard existing entry.
            self.discard_sample_cache_entry(e);
            if index == -1 {
                index = i as i32;
            }
        }
        let cache = &mut self.temporary_sample_cache;
        if index == -1 {
            index = cache.num_entries;
            cache.num_entries += 1;
        }
        let e = &mut cache.entries[index as usize];
        e.in_use = true;
        e.allocated_addr = ram_addr;
        e.size = size;
        Some((true, index as usize))
    }

    /// `AudioHeap_UnapplySampleCacheForFont`.
    fn unapply_sample_cache_for_font(&mut self, entry: SampleCacheEntry, font_id: i32) {
        let f = self.sound_font_list[font_id as usize];
        for inst_id in 0..f.num_instruments as i32 {
            let inst = self.get_instrument_inner(font_id, inst_id);
            if inst != 0 {
                use crate::layout::*;
                if self.ram.u8(inst + INST_NORMAL_RANGE_LO) != 0 {
                    let s = self.ram.u32(inst + INST_LOW_PITCH_TUNED_SAMPLE);
                    self.unapply_sample_cache(entry, s);
                }
                if self.ram.u8(inst + INST_NORMAL_RANGE_HI) != 0x7F {
                    let s = self.ram.u32(inst + INST_HIGH_PITCH_TUNED_SAMPLE);
                    self.unapply_sample_cache(entry, s);
                }
                let s = self.ram.u32(inst + INST_NORMAL_PITCH_TUNED_SAMPLE);
                self.unapply_sample_cache(entry, s);
            }
        }
        for drum_id in 0..f.num_drums as i32 {
            let drum = self.get_drum(font_id, drum_id);
            if drum != 0 {
                let s = self.ram.u32(drum + crate::layout::DRUM_TUNED_SAMPLE);
                self.unapply_sample_cache(entry, s);
            }
        }
        for sfx_id in 0..f.num_sfx as i32 {
            let sfx = self.get_sound_effect(font_id, sfx_id);
            if sfx != 0 {
                let s = self.ram.u32(sfx);
                self.unapply_sample_cache(entry, s);
            }
        }
    }

    /// `AudioHeap_DiscardSampleCacheEntry`.
    fn discard_sample_cache_entry(&mut self, entry: SampleCacheEntry) {
        let num_fonts = self.sound_font_table.num_entries as i32;
        for font_id in 0..num_fonts {
            let f = self.sound_font_list[font_id as usize];
            let (b1, b2) = (f.sample_bank_id1 as i32, f.sample_bank_id2 as i32);
            let id = entry.sample_bank_id as i32;
            if ((b1 != 0xFF) && (id == b1)) || ((b2 != 0xFF) && (id == b2)) || id == 0 {
                if self.search_caches(FONT_TABLE, CACHE_EITHER, font_id) != 0 && self.is_font_load_complete(font_id) {
                    self.unapply_sample_cache_for_font(entry, font_id);
                }
            }
        }
    }

    /// `AudioHeap_UnapplySampleCache`.
    fn unapply_sample_cache(&mut self, entry: SampleCacheEntry, sample: u32) {
        if sample != 0 && self.ram.sample(sample).sample_addr == entry.allocated_addr {
            self.ram.set_sample_addr(sample, entry.sample_addr);
            self.ram.set_sample_medium(sample, entry.orig_medium as u8);
        }
    }

    /// `AudioHeap_AllocPersistentSampleCacheEntry`.
    pub fn alloc_persistent_sample_cache_entry(&mut self, size: u32) -> Option<(bool, usize)> {
        let cache = &mut self.persistent_sample_cache;
        let ram_addr = cache.pool.alloc(size);
        if ram_addr == 0 {
            return None;
        }
        let i = cache.num_entries as usize;
        let e = &mut cache.entries[i];
        e.in_use = true;
        e.allocated_addr = ram_addr;
        e.size = size;
        cache.num_entries += 1;
        Some((false, i))
    }

    /// `AudioHeap_DiscardSampleCacheForFont`.
    fn discard_sample_cache_for_font(&mut self, entry: SampleCacheEntry, b1: i32, b2: i32, font_id: i32) {
        let id = entry.sample_bank_id as i32;
        if id == b1 || id == b2 || id == 0 {
            self.unapply_sample_cache_for_font(entry, font_id);
        }
    }

    /// `AudioHeap_DiscardSampleCaches`.
    pub fn discard_sample_caches(&mut self) {
        let num_fonts = self.sound_font_table.num_entries as i32;
        for font_id in 0..num_fonts {
            let f = self.sound_font_list[font_id as usize];
            let (b1, b2) = (f.sample_bank_id1 as i32, f.sample_bank_id2 as i32);
            if b1 == 0xFF && b2 == 0xFF {
                continue;
            }
            if self.search_caches(FONT_TABLE, CACHE_PERMANENT, font_id) == 0 || !self.is_font_load_complete(font_id) {
                continue;
            }
            for j in 0..self.persistent_sample_cache.num_entries as usize {
                let e = self.persistent_sample_cache.entries[j];
                self.discard_sample_cache_for_font(e, b1, b2, font_id);
            }
            for j in 0..self.temporary_sample_cache.num_entries as usize {
                let e = self.temporary_sample_cache.entries[j];
                self.discard_sample_cache_for_font(e, b1, b2, font_id);
            }
        }
    }

    /// `AudioHeap_ChangeStorage`.
    fn change_storage(&mut self, old_addr: u32, new_addr: u32, size: u32, new_medium: u8, sample: u32) {
        if sample != 0 {
            let s = self.ram.sample(sample);
            if old_addr <= s.sample_addr && s.sample_addr < old_addr + size {
                self.ram.set_sample_addr(sample, s.sample_addr - old_addr + new_addr);
                self.ram.set_sample_medium(sample, new_medium);
            }
        }
    }

    /// `AudioHeap_DiscardSampleBank`.
    pub fn discard_sample_bank(&mut self, sample_bank_id: i32) {
        self.apply_sample_bank_cache_internal(false, sample_bank_id);
    }

    /// `AudioHeap_ApplySampleBankCache`.
    pub fn apply_sample_bank_cache(&mut self, sample_bank_id: i32) {
        self.apply_sample_bank_cache_internal(true, sample_bank_id);
    }

    /// `AudioHeap_ApplySampleBankCacheInternal`.
    fn apply_sample_bank_cache_internal(&mut self, apply: bool, sample_bank_id: i32) {
        let num_fonts = self.sound_font_table.num_entries as i32;
        let mut old_addr = self.search_caches(SAMPLE_TABLE, CACHE_EITHER, sample_bank_id);
        if old_addr == 0 {
            return;
        }
        let entry = self.sample_bank_table.entries[sample_bank_id as usize];
        let size = entry.size;
        let mut new_medium = entry.medium as u8;
        let mut new_addr = if new_medium == MEDIUM_CART || new_medium == MEDIUM_DISK_DRIVE { entry.rom_addr } else { 0 };
        if apply {
            std::mem::swap(&mut old_addr, &mut new_addr);
            new_medium = MEDIUM_RAM;
        }
        for font_id in 0..num_fonts {
            let f = self.sound_font_list[font_id as usize];
            let (b1, b2) = (f.sample_bank_id1 as i32, f.sample_bank_id2 as i32);
            if b1 != 0xFF || b2 != 0xFF {
                if !self.is_font_load_complete(font_id) || self.search_caches(FONT_TABLE, CACHE_EITHER, font_id) == 0 {
                    continue;
                }
                if b1 != sample_bank_id && b2 != sample_bank_id {
                    continue;
                }
                use crate::layout::*;
                for inst_id in 0..f.num_instruments as i32 {
                    let inst = self.get_instrument_inner(font_id, inst_id);
                    if inst != 0 {
                        if self.ram.u8(inst + INST_NORMAL_RANGE_LO) != 0 {
                            let s = self.ram.u32(inst + INST_LOW_PITCH_TUNED_SAMPLE);
                            self.change_storage(old_addr, new_addr, size, new_medium, s);
                        }
                        if self.ram.u8(inst + INST_NORMAL_RANGE_HI) != 0x7F {
                            let s = self.ram.u32(inst + INST_HIGH_PITCH_TUNED_SAMPLE);
                            self.change_storage(old_addr, new_addr, size, new_medium, s);
                        }
                        let s = self.ram.u32(inst + INST_NORMAL_PITCH_TUNED_SAMPLE);
                        self.change_storage(old_addr, new_addr, size, new_medium, s);
                    }
                }
                for drum_id in 0..f.num_drums as i32 {
                    let drum = self.get_drum(font_id, drum_id);
                    if drum != 0 {
                        let s = self.ram.u32(drum + DRUM_TUNED_SAMPLE);
                        self.change_storage(old_addr, new_addr, size, new_medium, s);
                    }
                }
                for sfx_id in 0..f.num_sfx as i32 {
                    let sfx = self.get_sound_effect(font_id, sfx_id);
                    if sfx != 0 {
                        let s = self.ram.u32(sfx);
                        self.change_storage(old_addr, new_addr, size, new_medium, s);
                    }
                }
            }
        }
    }

    /// `AudioHeap_DiscardSampleBanks`.
    pub fn discard_sample_banks(&mut self) {
        let t = self.sample_bank_cache.temporary;
        if t.entries[0].id != -1 {
            self.discard_sample_bank(t.entries[0].id as i32);
        }
        if t.entries[1].id != -1 {
            self.discard_sample_bank(t.entries[1].id as i32);
        }
        let p = self.sample_bank_cache.persistent;
        for i in 0..p.num_entries as usize {
            self.discard_sample_bank(p.entries[i].id as i32);
        }
    }
}

/// `osAiSetFrequency`: the rate the AI really plays at, from the VI clock (NTSC's here: the
/// port runs the VI at 60 Hz, ADR 0025).
pub fn os_ai_set_frequency(frequency: u32) -> i32 {
    // VI_NTSC_CLOCK (ultra64/rcp.h), osViClock for NTSC (src/libultra/os/initialize.c).
    const OS_VI_CLOCK: i32 = 48681812;
    let dac_rate_f = (OS_VI_CLOCK as f32 / frequency as f32) + 0.5f32;
    let dac_rate = dac_rate_f as u32;
    if dac_rate < 132 {
        return -1;
    }
    OS_VI_CLOCK / dac_rate as i32
}
