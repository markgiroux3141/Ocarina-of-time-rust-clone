//! `playback.c`: the notes. Each update turns what a note's layer and channel ask for
//! into a `NoteSampleState` for the synthesis (volume and pan, resampling rate, reverb), allocates
//! notes to layers from the pools, and moves notes between the pools' lists as they decay and
//! release. With them, the C's list functions (`AudioListItem`).

use crate::context::*;
use crate::layout::*;

/// `gDefaultNoteSampleState`: enabled and needing init, the rest zero.
pub fn default_note_sub() -> NoteSampleState {
    NoteSampleState { enabled: true, needs_init: true, ..Default::default() }
}

/// The list node of layer `l` and of note `n` (layers first: their number doesn't change).
pub const fn layer_node(l: LayerId) -> NodeId {
    l
}
pub const fn note_node(n: NoteId) -> NodeId {
    64 + n
}

impl AudioContext {
    // --- The lists (`AudioListItem`)

    /// A list head: empty, pointing at itself.
    pub fn new_list_head(&mut self, pool: Option<PoolId>) -> NodeId {
        let id = self.lists.len();
        self.lists.push(ListNode { prev: Some(id), next: id, count: 0, pool, value: Item::Head });
        id
    }

    /// A pool with its four lists (`Audio_InitNoteLists`).
    pub fn new_note_pool(&mut self) -> PoolId {
        let id = self.pools.len();
        self.pools.push(NotePool::default());
        let disabled = self.new_list_head(Some(id));
        let decaying = self.new_list_head(Some(id));
        let releasing = self.new_list_head(Some(id));
        let active = self.new_list_head(Some(id));
        self.pools[id] = NotePool { disabled, decaying, releasing, active };
        id
    }

    /// `Audio_InitNoteLists`: empties a pool's four lists.
    pub fn init_note_lists(&mut self, pool: PoolId) {
        let p = self.pools[pool];
        for h in [p.disabled, p.decaying, p.releasing, p.active] {
            self.lists[h] = ListNode { prev: Some(h), next: h, count: 0, pool: Some(pool), value: Item::Head };
        }
    }

    /// The arena: one node per layer and note, the free lists' heads, and
    /// `sequenceChannelNone`'s pool. The players' and channels' pools are added as they're
    /// initialized.
    pub fn init_lists(&mut self) {
        self.lists.clear();
        self.pools.clear();
        for l in 0..64 {
            self.lists.push(ListNode { prev: None, next: l, count: 0, pool: None, value: Item::Layer(l) });
        }
        for n in 0..self.num_notes as usize {
            self.lists.push(ListNode { prev: None, next: note_node(n), count: 0, pool: None, value: Item::Note(n) });
        }
        self.note_free_lists = self.new_note_pool();
        self.layer_free_list = self.new_list_head(None);
        let none_pool = self.new_note_pool();
        self.channels.clear();
        self.channels.push(SequenceChannel { note_pool: none_pool, ..Default::default() });
    }

    /// `Audio_AudioListPushFront`: adds `item` to the front of `list`, if it's in no list.
    pub fn audio_list_push_front(&mut self, list: NodeId, item: NodeId) {
        if self.lists[item].prev.is_none() {
            let next = self.lists[list].next;
            self.lists[item].prev = Some(list);
            self.lists[item].next = next;
            self.lists[next].prev = Some(item);
            self.lists[list].next = item;
            self.lists[list].count += 1;
            self.lists[item].pool = self.lists[list].pool;
        }
    }

    /// `Audio_AudioListRemove`: takes `item` out of its list, if it's in one.
    pub fn audio_list_remove_node(&mut self, item: NodeId) {
        if let Some(prev) = self.lists[item].prev {
            let next = self.lists[item].next;
            self.lists[prev].next = next;
            self.lists[next].prev = Some(prev);
            self.lists[item].prev = None;
        }
    }

    /// `AudioSeq_AudioListPushBack`.
    pub fn audio_list_push_back(&mut self, list: NodeId, item: NodeId) {
        if self.lists[item].prev.is_none() {
            let prev = self.lists[list].prev.unwrap_or(list);
            self.lists[prev].next = item;
            self.lists[item].prev = Some(prev);
            self.lists[item].next = list;
            self.lists[list].prev = Some(item);
            self.lists[list].count += 1;
            self.lists[item].pool = self.lists[list].pool;
        }
    }

    /// `AudioSeq_AudioListPopBack`.
    pub fn audio_list_pop_back(&mut self, list: NodeId) -> Option<Item> {
        let item = self.lists[list].prev.unwrap_or(list);
        if item == list {
            return None;
        }
        let prev = self.lists[item].prev.unwrap_or(list);
        self.lists[prev].next = list;
        self.lists[list].prev = Some(prev);
        self.lists[item].prev = None;
        self.lists[list].count -= 1;
        Some(self.lists[item].value)
    }

    /// `note->listItem.pool`.
    fn note_list_pool(&self, n: NoteId) -> PoolId {
        self.lists[note_node(n)].pool.unwrap_or(self.note_free_lists)
    }

    // --- playback.c

    /// `Audio_InitSampleState`: writes `noteSubsEu[sub]` from the note and the attributes.
    pub fn init_note_sub(&mut self, n: NoteId, sub: usize, attrs: &NoteSampleStateAttributes) {
        let note = self.notes[n];
        let stereo_headset_effects = note.playback_state.stereo_headset_effects != 0;
        let mut vel = attrs.velocity;
        let mut pan = attrs.pan;
        let reverb_vol = attrs.reverb_vol;
        let stereo_data = attrs.stereo;
        let t = self.tables.clone();
        let sound_mode = self.sound_mode;
        let s = &mut self.note_subs_eu[sub];

        s.copy_bitfields_from(&note.note_sub_eu);
        s.tuned_sample = note.note_sub_eu.tuned_sample;
        s.harmonic_index_cur_and_prev = note.note_sub_eu.harmonic_index_cur_and_prev;

        note_set_resampling_rate(s, attrs.frequency);

        pan &= 0x7F;

        s.stereo_strong_right = false;
        s.stereo_strong_left = false;
        s.stereo_headset_effects = stereo::stereo_headset_effects(stereo_data);
        s.uses_headset_pan_effects = stereo::uses_headset_pan_effects(stereo_data);
        let (vol_left, vol_right);
        if stereo_headset_effects && sound_mode == SOUND_OUTPUT_HEADSET {
            let half_pan_index = ((pan >> 1) as usize).min(0x3F);
            s.haas_effect_right_delay_size = t.haas_effect_delay_sizes[half_pan_index] as u8;
            s.haas_effect_left_delay_size = t.haas_effect_delay_sizes[0x3F - half_pan_index] as u8;
            s.use_haas_effect = true;
            vol_left = t.headset_pan_volume[pan as usize];
            vol_right = t.headset_pan_volume[0x7F - pan as usize];
        } else if stereo_headset_effects && sound_mode == SOUND_OUTPUT_STEREO {
            let (mut strong_left, mut strong_right) = (false, false);
            s.haas_effect_left_delay_size = 0;
            s.haas_effect_right_delay_size = 0;
            s.use_haas_effect = false;
            vol_left = t.stereo_pan_volume[pan as usize];
            vol_right = t.stereo_pan_volume[0x7F - pan as usize];
            if pan < 0x20 {
                strong_left = true;
            } else if pan > 0x60 {
                strong_right = true;
            }
            s.stereo_strong_right = strong_right;
            s.stereo_strong_left = strong_left;
            match stereo::bit2(stereo_data) {
                0 => {}
                1 => {
                    s.stereo_strong_right = stereo::strong_right(stereo_data);
                    s.stereo_strong_left = stereo::strong_left(stereo_data);
                }
                2 => {
                    s.stereo_strong_right = stereo::strong_right(stereo_data) | strong_right;
                    s.stereo_strong_left = stereo::strong_left(stereo_data) | strong_left;
                }
                _ => {
                    s.stereo_strong_right = stereo::strong_right(stereo_data) ^ strong_right;
                    s.stereo_strong_left = stereo::strong_left(stereo_data) ^ strong_left;
                }
            }
        } else if sound_mode == SOUND_OUTPUT_MONO {
            s.stereo_headset_effects = false;
            s.uses_headset_pan_effects = false;
            vol_left = 0.707f32; // approx 1/sqrt(2)
            vol_right = 0.707f32;
        } else {
            s.stereo_strong_right = stereo::strong_right(stereo_data);
            s.stereo_strong_left = stereo::strong_left(stereo_data);
            vol_left = t.default_pan_volume[pan as usize];
            vol_right = t.default_pan_volume[0x7F - pan as usize];
        }

        vel = if 0.0f32 > vel { 0.0 } else { vel };
        vel = if 1.0f32 < vel { 1.0 } else { vel };

        s.target_vol_left = ((vel * vol_left) * (0x1000 as f32 - 0.001f32)) as i32 as u16;
        s.target_vol_right = ((vel * vol_right) * (0x1000 as f32 - 0.001f32)) as i32 as u16;

        s.gain = attrs.gain;
        s.filter = attrs.filter;
        s.comb_filter_size = attrs.comb_filter_size;
        s.comb_filter_gain = attrs.comb_filter_gain;
        s.reverb_vol = reverb_vol;
    }

    /// `Audio_NoteInit`.
    pub fn note_init(&mut self, n: NoteId) {
        let layer = self.notes[n].playback_state.parent_layer.expect("Audio_NoteInit with no parent layer");
        let l = &self.sequence_layers[layer];
        let envelope = if l.adsr.decay_index == 0 { self.channels[l.channel.unwrap_or(CHANNEL_NONE)].adsr.envelope } else { l.adsr.envelope };
        let ps = &mut self.notes[n].playback_state;
        audio_adsr_init(&mut ps.adsr, envelope);
        ps.unk_04 = 0;
        ps.adsr.set_state(ADSR_STATE_INITIAL);
        self.notes[n].note_sub_eu = default_note_sub();
    }

    /// `Audio_NoteDisable`.
    pub fn note_disable(&mut self, n: NoteId) {
        let note = &mut self.notes[n];
        if note.note_sub_eu.needs_init {
            note.note_sub_eu.needs_init = false;
        }
        note.playback_state.priority = 0;
        note.note_sub_eu.enabled = false;
        note.playback_state.unk_04 = 0;
        note.note_sub_eu.finished = false;
        note.playback_state.parent_layer = None;
        note.playback_state.prev_parent_layer = None;
        note.playback_state.adsr.set_state(ADSR_STATE_DISABLED);
        note.playback_state.adsr.current = 0.0;
    }

    /// `Audio_ProcessNotes`.
    pub fn process_notes(&mut self) {
        'notes: for i in 0..self.num_notes as usize {
            let sub2 = (self.note_sub_eu_offset + i as i32) as usize;
            let ps = self.notes[i].playback_state;
            'out: {
                if let Some(parent) = ps.parent_layer {
                    // `(u32)playbackState->parentLayer < 0x7FFFFFFF` is never true.
                    let layer = &self.sequence_layers[parent];
                    let chan = layer.channel.unwrap_or(CHANNEL_NONE);
                    if layer.note != Some(i) && ps.unk_04 == 0 {
                        let inv = self.audio_buffer_parameters.updates_per_frame_inv;
                        let ps = &mut self.notes[i].playback_state;
                        ps.adsr.set_release(true);
                        ps.adsr.fade_out_vel = inv;
                        ps.priority = 1;
                        ps.unk_04 = 2;
                        break 'out;
                    } else if !layer.enabled && ps.unk_04 == 0 && ps.priority >= 1 {
                        // do nothing
                    } else if self.channels[chan].seq_player.is_none() {
                        self.sequence_channel_disable(chan);
                        self.notes[i].playback_state.priority = 1;
                        self.notes[i].playback_state.unk_04 = 1;
                        continue 'notes;
                    } else if self.seq_players[self.channels[chan].seq_player.unwrap()].muted && (self.channels[chan].mute_behavior & MUTE_BEHAVIOR_STOP_NOTES) != 0 {
                        // do nothing
                    } else {
                        break 'out;
                    }
                    self.seq_layer_note_release(Some(parent));
                    self.audio_list_remove_node(note_node(i));
                    let pool = self.note_list_pool(i);
                    let decaying = self.pools[pool].decaying;
                    self.audio_list_push_front(decaying, note_node(i));
                    self.notes[i].playback_state.priority = 1;
                    self.notes[i].playback_state.unk_04 = 2;
                } else if ps.unk_04 == 0 && ps.priority >= 1 {
                    continue 'notes;
                }
            }

            // out:
            if self.notes[i].playback_state.priority == 0 {
                continue;
            }
            let ps = self.notes[i].playback_state;
            let finished = self.notes[i].note_sub_eu.finished;
            if ps.unk_04 >= 1 || finished {
                if ps.adsr.state() == ADSR_STATE_DISABLED || finished {
                    if let Some(wanted) = ps.wanted_parent_layer {
                        self.note_disable(i);
                        if self.sequence_layers[wanted].channel.is_some() {
                            self.note_init_for_layer(i, wanted);
                            self.note_vibrato_init(i);
                            self.note_portamento_init(i);
                            self.audio_list_remove_node(note_node(i));
                            let pool = self.note_list_pool(i);
                            let active = self.pools[pool].active;
                            self.audio_list_push_back(active, note_node(i));
                            self.notes[i].playback_state.wanted_parent_layer = None;
                            // don't skip
                        } else {
                            self.note_disable(i);
                            self.audio_list_remove_node(note_node(i));
                            let pool = self.note_list_pool(i);
                            let disabled = self.pools[pool].disabled;
                            self.audio_list_push_back(disabled, note_node(i));
                            self.notes[i].playback_state.wanted_parent_layer = None;
                            continue; // skip
                        }
                    } else {
                        if let Some(p) = ps.parent_layer {
                            self.sequence_layers[p].bit1 = true;
                        }
                        self.note_disable(i);
                        self.audio_list_remove_node(note_node(i));
                        let pool = self.note_list_pool(i);
                        let disabled = self.pools[pool].disabled;
                        self.audio_list_push_back(disabled, note_node(i));
                        continue;
                    }
                }
            } else if ps.adsr.state() == ADSR_STATE_DISABLED {
                if let Some(p) = ps.parent_layer {
                    self.sequence_layers[p].bit1 = true;
                }
                self.note_disable(i);
                self.audio_list_remove_node(note_node(i));
                let pool = self.note_list_pool(i);
                let disabled = self.pools[pool].disabled;
                self.audio_list_push_back(disabled, note_node(i));
                continue;
            }

            let scale = self.adsr_update_note(i);
            self.note_vibrato_update(i);
            let ps = self.notes[i].playback_state;
            let attrs = ps.attributes;
            let mut sub_attrs;
            let book_offset;
            if ps.unk_04 == 1 || ps.unk_04 == 2 {
                sub_attrs = NoteSampleStateAttributes {
                    frequency: attrs.freq_scale,
                    velocity: attrs.velocity,
                    pan: attrs.pan,
                    reverb_vol: attrs.reverb,
                    stereo: attrs.stereo,
                    gain: attrs.gain,
                    filter: attrs.filter,
                    comb_filter_size: attrs.comb_filter_size,
                    comb_filter_gain: attrs.comb_filter_gain,
                };
                book_offset = self.notes[i].note_sub_eu.book_offset;
            } else {
                let layer = &self.sequence_layers[ps.parent_layer.expect("an attached note has a layer")];
                let channel = &self.channels[layer.channel.unwrap_or(CHANNEL_NONE)];
                sub_attrs = NoteSampleStateAttributes {
                    frequency: layer.note_freq_scale,
                    velocity: layer.note_velocity,
                    pan: layer.note_pan,
                    stereo: if layer.stereo == 0 { channel.stereo } else { layer.stereo },
                    reverb_vol: channel.reverb,
                    gain: channel.gain,
                    filter: channel.filter,
                    comb_filter_size: channel.comb_filter_size,
                    comb_filter_gain: channel.comb_filter_gain,
                };
                book_offset = channel.book_offset & 0x7;
                let muted = channel.seq_player.map(|p| self.seq_players[p].muted).unwrap_or(false);
                if muted && (channel.mute_behavior & MUTE_BEHAVIOR_3) != 0 {
                    sub_attrs.frequency = 0.0;
                    sub_attrs.velocity = 0.0;
                }
            }
            sub_attrs.frequency *= ps.vibrato_freq_scale * ps.portamento_freq_scale;
            sub_attrs.frequency *= self.audio_buffer_parameters.resample_rate;
            sub_attrs.velocity *= scale;
            self.init_note_sub(i, sub2, &sub_attrs);
            self.notes[i].note_sub_eu.book_offset = book_offset & 3;
        }
    }

    /// `Audio_GetInstrumentTunedSample`: the `TunedSample*` in `inst` for `semitone`.
    pub fn get_instrument_tuned_sample(&self, inst: u32, semitone: i32) -> u32 {
        if semitone < self.ram.u8(inst + INST_NORMAL_RANGE_LO) as i32 {
            inst + INST_LOW_PITCH_TUNED_SAMPLE
        } else if semitone <= self.ram.u8(inst + INST_NORMAL_RANGE_HI) as i32 {
            inst + INST_NORMAL_PITCH_TUNED_SAMPLE
        } else {
            inst + INST_HIGH_PITCH_TUNED_SAMPLE
        }
    }

    /// `Audio_GetInstrumentInner`.
    pub fn get_instrument_inner(&mut self, font_id: i32, inst_id: i32) -> u32 {
        if font_id == 0xFF {
            return 0;
        }
        if !self.is_font_load_complete(font_id) {
            self.audio_error_flags = font_id + 0x1000_0000;
            return 0;
        }
        let f = self.sound_font_list[font_id as usize];
        if inst_id >= f.num_instruments as i32 {
            self.audio_error_flags = ((font_id << 8) + inst_id) + 0x300_0000;
            return 0;
        }
        let inst = self.ram.u32(f.instruments + 4 * inst_id as u32);
        if inst == 0 {
            self.audio_error_flags = ((font_id << 8) + inst_id) + 0x100_0000;
        }
        inst
    }

    /// `Audio_GetDrum`.
    pub fn get_drum(&mut self, font_id: i32, drum_id: i32) -> u32 {
        if font_id == 0xFF {
            return 0;
        }
        if !self.is_font_load_complete(font_id) {
            self.audio_error_flags = font_id + 0x1000_0000;
            return 0;
        }
        let f = self.sound_font_list[font_id as usize];
        if drum_id >= f.num_drums as i32 {
            self.audio_error_flags = ((font_id << 8) + drum_id) + 0x400_0000;
            return 0;
        }
        if f.drums < crate::ram::K0BASE {
            return 0;
        }
        let drum = self.ram.u32(f.drums + 4 * drum_id as u32);
        if drum == 0 {
            self.audio_error_flags = ((font_id << 8) + drum_id) + 0x500_0000;
        }
        drum
    }

    /// `Audio_GetSoundEffect`: the `SoundEffect*` (its `TunedSample`).
    pub fn get_sound_effect(&mut self, font_id: i32, sfx_id: i32) -> u32 {
        if font_id == 0xFF {
            return 0;
        }
        if !self.is_font_load_complete(font_id) {
            self.audio_error_flags = font_id + 0x1000_0000;
            return 0;
        }
        let f = self.sound_font_list[font_id as usize];
        if sfx_id >= f.num_sfx as i32 {
            self.audio_error_flags = ((font_id << 8) + sfx_id) + 0x400_0000;
            return 0;
        }
        if f.sound_effects < crate::ram::K0BASE {
            return 0;
        }
        let sfx = f.sound_effects + SIZEOF_SOUND_EFFECT * sfx_id as u32;
        if self.ram.u32(sfx) == 0 {
            return 0;
        }
        sfx
    }

    /// `Audio_SetFontInstrument`.
    pub fn set_font_instrument(&mut self, instrument_type: i32, font_id: i32, index: i32, value: u32) -> i32 {
        if font_id == 0xFF {
            return -1;
        }
        if !self.is_font_load_complete(font_id) {
            return -2;
        }
        let f = self.sound_font_list[font_id as usize];
        match instrument_type {
            0 => {
                if index >= f.num_drums as i32 {
                    return -3;
                }
                self.ram.set_u32(f.drums + 4 * index as u32, value);
            }
            1 => {
                if index >= f.num_sfx as i32 {
                    return -3;
                }
                // `soundEffects[index] = *(SoundEffect*)value`
                let (s, t) = (self.ram.u32(value), self.ram.u32(value + 4));
                let at = f.sound_effects + SIZEOF_SOUND_EFFECT * index as u32;
                self.ram.set_u32(at, s);
                self.ram.set_u32(at + 4, t);
            }
            _ => {
                if index >= f.num_instruments as i32 {
                    return -3;
                }
                self.ram.set_u32(f.instruments + 4 * index as u32, value);
            }
        }
        0
    }

    /// `Audio_SeqLayerDecayRelease`.
    pub fn seq_layer_decay_release(&mut self, layer: Option<LayerId>, target: u8) {
        let Some(l) = layer else { return };
        self.sequence_layers[l].bit3 = false;
        let Some(n) = self.sequence_layers[l].note else { return };

        if self.notes[n].playback_state.wanted_parent_layer == Some(l) {
            self.notes[n].playback_state.wanted_parent_layer = None;
        }

        let ps = self.notes[n].playback_state;
        if ps.parent_layer != Some(l) {
            if ps.parent_layer.is_none() && ps.wanted_parent_layer.is_none() && ps.prev_parent_layer == Some(l) && target != ADSR_STATE_DECAY {
                let inv = self.audio_buffer_parameters.updates_per_frame_inv;
                let a = &mut self.notes[n].playback_state.adsr;
                a.fade_out_vel = inv;
                a.set_release(true);
            }
            return;
        }

        if ps.adsr.state() != ADSR_STATE_DECAY {
            let lay = self.sequence_layers[l].clone();
            {
                let attrs = &mut self.notes[n].playback_state.attributes;
                attrs.freq_scale = lay.note_freq_scale;
                attrs.velocity = lay.note_velocity;
                attrs.pan = lay.note_pan;
            }
            if let Some(c) = lay.channel {
                let chan = self.channels[c].clone();
                let filter_buf = self.notes_addr + n as u32 * SIZEOF_NOTE + NOTE_FILTER_BUF_OFFSET;
                let attrs = &mut self.notes[n].playback_state.attributes;
                attrs.reverb = chan.reverb;
                attrs.gain = chan.gain;
                attrs.filter = chan.filter;
                if attrs.filter != 0 {
                    for i in 0..8 {
                        let v = self.ram.s16(chan.filter + 2 * i);
                        self.ram.set_s16(filter_buf + 2 * i, v);
                    }
                    self.notes[n].playback_state.attributes.filter = filter_buf;
                }
                let attrs = &mut self.notes[n].playback_state.attributes;
                attrs.comb_filter_gain = chan.comb_filter_gain;
                attrs.comb_filter_size = chan.comb_filter_size;
                let muted = chan.seq_player.map(|p| self.seq_players[p].muted).unwrap_or(false);
                if muted && (chan.mute_behavior & MUTE_BEHAVIOR_3) != 0 {
                    self.notes[n].note_sub_eu.finished = true;
                }
                let attrs = &mut self.notes[n].playback_state.attributes;
                attrs.stereo = if lay.stereo == 0 { chan.stereo } else { lay.stereo };
                self.notes[n].playback_state.priority = chan.some_other_priority;
            } else {
                self.notes[n].playback_state.attributes.stereo = lay.stereo;
                self.notes[n].playback_state.priority = 1;
            }

            let inv = self.audio_buffer_parameters.updates_per_frame_inv;
            let ps = &mut self.notes[n].playback_state;
            ps.prev_parent_layer = ps.parent_layer;
            ps.parent_layer = None;
            if target == ADSR_STATE_RELEASE {
                ps.adsr.fade_out_vel = inv;
                ps.adsr.set_release(true);
                ps.unk_04 = 2;
            } else {
                ps.unk_04 = 1;
                ps.adsr.set_decay(true);
                let chan = lay.channel.unwrap_or(CHANNEL_NONE);
                let idx = if lay.adsr.decay_index == 0 { self.channels[chan].adsr.decay_index } else { lay.adsr.decay_index };
                let ps = &mut self.notes[n].playback_state;
                ps.adsr.fade_out_vel = self.adsr_decay_table[idx as usize];
                ps.adsr.sustain = ((self.channels[chan].adsr.sustain as i32) as f32 * ps.adsr.current) / 256.0f32;
            }
        }

        if target == ADSR_STATE_DECAY {
            self.audio_list_remove_node(note_node(n));
            let pool = self.note_list_pool(n);
            let decaying = self.pools[pool].decaying;
            self.audio_list_push_front(decaying, note_node(n));
        }
    }

    /// `Audio_SeqLayerNoteDecay`.
    pub fn seq_layer_note_decay(&mut self, layer: Option<LayerId>) {
        self.seq_layer_decay_release(layer, ADSR_STATE_DECAY);
    }

    /// `Audio_SeqLayerNoteRelease`.
    pub fn seq_layer_note_release(&mut self, layer: Option<LayerId>) {
        self.seq_layer_decay_release(layer, ADSR_STATE_RELEASE);
    }

    /// `Audio_BuildSyntheticWave`: picks the wave's harmonic for the layer's frequency.
    pub fn build_synthetic_wave(&mut self, n: NoteId, l: LayerId, wave_id: i32) -> i32 {
        let wave_id = wave_id.max(128);
        let layer = &mut self.sequence_layers[l];
        let mut freq_scale = layer.freq_scale;
        if layer.portamento.mode != 0 && 0.0f32 < layer.portamento.extent {
            freq_scale *= layer.portamento.extent + 1.0f32;
        }
        // Map frequency to the harmonic to use from gWaveSamples
        let (harmonic_index, freq_ratio): (u8, f32) = if freq_scale < 0.99999f32 {
            (0, 1.0465f32)
        } else if freq_scale < 1.99999f32 {
            (1, 1.0465f32 / 2.0)
        } else if freq_scale < 3.99999f32 {
            (2, 1.0465f32 / 4.0 + 1.005E-3f32)
        } else {
            (3, 1.0465f32 / 8.0 - 2.5E-6f32)
        };
        layer.freq_scale *= freq_ratio;
        let ps = &mut self.notes[n].playback_state;
        ps.wave_id = wave_id as u8;
        ps.harmonic_index = harmonic_index;
        // waveId index starts at 128, there are WAVE_SAMPLE_COUNT samples to read from
        let base = self.statics.wave_samples.get((wave_id - 128) as usize).copied().unwrap_or(0);
        self.notes[n].note_sub_eu.tuned_sample = base + (harmonic_index as u32 * WAVE_SAMPLE_COUNT as u32) * 2;
        harmonic_index as i32
    }

    /// `Audio_InitSyntheticWave`.
    pub fn init_synthetic_wave(&mut self, n: NoteId, l: LayerId) {
        let mut wave_id = self.sequence_layers[l].inst_or_wave as i32;
        if wave_id == 0xFF {
            wave_id = self.channels[self.sequence_layers[l].channel.unwrap_or(CHANNEL_NONE)].inst_or_wave as i32;
        }
        let prev = self.notes[n].playback_state.harmonic_index as i32;
        let cur = self.build_synthetic_wave(n, l, wave_id);
        if cur != prev {
            self.notes[n].note_sub_eu.harmonic_index_cur_and_prev = ((cur << 2) + prev) as u8;
        }
    }

    /// `Audio_InitNoteFreeList`.
    pub fn init_note_free_list(&mut self) {
        let pool = self.note_free_lists;
        self.init_note_lists(pool);
        for i in 0..self.num_notes as usize {
            self.lists[note_node(i)].prev = None;
            let d = self.pools[pool].disabled;
            self.audio_list_push_back(d, note_node(i));
        }
    }

    /// `Audio_NotePoolClear`: returns every note to the free lists.
    pub fn note_pool_clear(&mut self, pool: PoolId) {
        let free = self.pools[self.note_free_lists];
        let p = self.pools[pool];
        for (source, dest) in [(p.disabled, free.disabled), (p.decaying, free.decaying), (p.releasing, free.releasing), (p.active, free.active)] {
            loop {
                let cur = self.lists[source].next;
                if cur == source {
                    break;
                }
                self.audio_list_remove_node(cur);
                self.audio_list_push_back(dest, cur);
            }
        }
    }

    /// `Audio_NotePoolFill`.
    pub fn note_pool_fill(&mut self, pool: PoolId, count: i32) {
        self.note_pool_clear(pool);
        let free = self.pools[self.note_free_lists];
        let p = self.pools[pool];
        let mut j = 0;
        for (source, dest) in [(free.disabled, p.disabled), (free.decaying, p.decaying), (free.releasing, p.releasing), (free.active, p.active)] {
            if j >= count {
                return;
            }
            while j < count {
                let Some(Item::Note(n)) = self.audio_list_pop_back(source) else { break };
                self.audio_list_push_back(dest, note_node(n));
                j += 1;
            }
        }
    }

    /// `Audio_FindNodeWithPrioLessThan`.
    pub fn find_node_with_prio_less_than(&self, list: NodeId, limit: i32) -> Option<NoteId> {
        let mut cur = self.lists[list].next;
        if cur == list {
            return None;
        }
        let prio = |node: NodeId| match self.lists[node].value {
            Item::Note(n) => self.notes[n].playback_state.priority,
            _ => 0,
        };
        let mut best = cur;
        while cur != list {
            if prio(best) >= prio(cur) {
                best = cur;
            }
            cur = self.lists[cur].next;
        }
        if limit <= prio(best) as i32 {
            return None;
        }
        match self.lists[best].value {
            Item::Note(n) => Some(n),
            _ => None,
        }
    }

    /// `Audio_NoteInitForLayer`.
    pub fn note_init_for_layer(&mut self, n: NoteId, l: LayerId) {
        let chan = self.sequence_layers[l].channel.unwrap_or(CHANNEL_NONE);
        {
            let ps = &mut self.notes[n].playback_state;
            ps.prev_parent_layer = None;
            ps.parent_layer = Some(l);
            ps.priority = self.channels[chan].note_priority;
        }
        {
            let layer = &mut self.sequence_layers[l];
            layer.note_properties_need_init = true;
            layer.bit3 = true;
            layer.note = Some(n);
            layer.note_velocity = 0.0;
        }
        self.channels[chan].note_unused = Some(n);
        self.channels[chan].layer_unused = Some(l);
        self.note_init(n);
        let mut inst_id = self.sequence_layers[l].inst_or_wave as i16;
        if inst_id == 0xFF {
            inst_id = self.channels[chan].inst_or_wave;
        }
        self.notes[n].note_sub_eu.tuned_sample = self.sequence_layers[l].tuned_sample;
        self.notes[n].note_sub_eu.is_synthetic_wave = (0x80..0xC0).contains(&inst_id);
        if self.notes[n].note_sub_eu.is_synthetic_wave {
            self.build_synthetic_wave(n, l, inst_id as i32);
        }
        let c = &self.channels[chan];
        let ps = &mut self.notes[n].playback_state;
        ps.font_id = c.font_id;
        ps.stereo_headset_effects = c.stereo_headset_effects as u8;
        self.notes[n].note_sub_eu.reverb_index = c.reverb_index & 3;
    }

    /// `func_800E82C0`.
    pub fn func_800e82c0(&mut self, n: NoteId, l: LayerId) {
        // similar to Audio_NoteReleaseAndTakeOwnership, hard to say what the difference is
        let parent = self.notes[n].playback_state.parent_layer;
        self.seq_layer_note_release(parent);
        self.notes[n].playback_state.wanted_parent_layer = Some(l);
    }

    /// `Audio_NoteReleaseAndTakeOwnership`.
    pub fn note_release_and_take_ownership(&mut self, n: NoteId, l: LayerId) {
        let prio = self.channels[self.sequence_layers[l].channel.unwrap_or(CHANNEL_NONE)].note_priority;
        let inv = self.audio_buffer_parameters.updates_per_frame_inv;
        let ps = &mut self.notes[n].playback_state;
        ps.wanted_parent_layer = Some(l);
        ps.priority = prio;
        ps.adsr.fade_out_vel = inv;
        ps.adsr.set_release(true);
    }

    /// `Audio_AllocNoteFromDisabled`.
    pub fn alloc_note_from_disabled(&mut self, pool: PoolId, l: LayerId) -> Option<NoteId> {
        let d = self.pools[pool].disabled;
        let Some(Item::Note(n)) = self.audio_list_pop_back(d) else { return None };
        self.note_init_for_layer(n, l);
        let a = self.pools[pool].active;
        self.audio_list_push_front(a, note_node(n));
        Some(n)
    }

    /// `Audio_AllocNoteFromDecaying`.
    pub fn alloc_note_from_decaying(&mut self, pool: PoolId, l: LayerId) -> Option<NoteId> {
        let d = self.pools[pool].decaying;
        let Some(Item::Note(n)) = self.audio_list_pop_back(d) else { return None };
        self.note_release_and_take_ownership(n, l);
        let r = self.pools[pool].releasing;
        self.audio_list_push_back(r, note_node(n));
        Some(n)
    }

    /// `Audio_AllocNoteFromActive`.
    pub fn alloc_note_from_active(&mut self, pool: PoolId, l: LayerId) -> Option<NoteId> {
        let prio = self.channels[self.sequence_layers[l].channel.unwrap_or(CHANNEL_NONE)].note_priority;
        let (mut r_priority, mut a_priority) = (0x10, 0x10);
        let r_note = self.find_node_with_prio_less_than(self.pools[pool].releasing, prio as i32);
        if let Some(r) = r_note {
            r_priority = self.notes[r].playback_state.priority as i32;
        }
        let a_note = self.find_node_with_prio_less_than(self.pools[pool].active, prio as i32);
        if let Some(a) = a_note {
            a_priority = self.notes[a].playback_state.priority as i32;
        }
        if r_note.is_none() && a_note.is_none() {
            return None;
        }
        if a_priority < r_priority {
            let a = a_note.unwrap();
            self.audio_list_remove_node(note_node(a));
            self.func_800e82c0(a, l);
            let rel = self.pools[pool].releasing;
            self.audio_list_push_back(rel, note_node(a));
            self.notes[a].playback_state.priority = prio;
            return Some(a);
        }
        let r = r_note.unwrap();
        self.notes[r].playback_state.wanted_parent_layer = Some(l);
        self.notes[r].playback_state.priority = prio;
        Some(r)
    }

    /// `Audio_AllocNote`.
    pub fn alloc_note(&mut self, l: LayerId) -> Option<NoteId> {
        let chan = self.sequence_layers[l].channel.unwrap_or(CHANNEL_NONE);
        let policy = self.channels[chan].note_alloc_policy as u32;
        let chan_pool = self.channels[chan].note_pool;
        let player_pool = self.channels[chan].seq_player.map(|p| self.seq_players[p].note_pool).unwrap_or(self.note_free_lists);
        let free = self.note_free_lists;

        if policy & 1 != 0 {
            if let Some(n) = self.sequence_layers[l].note {
                let ps = self.notes[n].playback_state;
                if ps.prev_parent_layer == Some(l) && ps.wanted_parent_layer.is_none() {
                    self.note_release_and_take_ownership(n, l);
                    self.audio_list_remove_node(note_node(n));
                    let pool = self.note_list_pool(n);
                    let r = self.pools[pool].releasing;
                    self.audio_list_push_back(r, note_node(n));
                    return Some(n);
                }
            }
        }

        let result = if policy & 2 != 0 {
            self.alloc_note_from_disabled(chan_pool, l).or_else(|| self.alloc_note_from_decaying(chan_pool, l)).or_else(|| self.alloc_note_from_active(chan_pool, l))
        } else if policy & 4 != 0 {
            self.alloc_note_from_disabled(chan_pool, l)
                .or_else(|| self.alloc_note_from_disabled(player_pool, l))
                .or_else(|| self.alloc_note_from_decaying(chan_pool, l))
                .or_else(|| self.alloc_note_from_decaying(player_pool, l))
                .or_else(|| self.alloc_note_from_active(chan_pool, l))
                .or_else(|| self.alloc_note_from_active(player_pool, l))
        } else if policy & 8 != 0 {
            self.alloc_note_from_disabled(free, l).or_else(|| self.alloc_note_from_decaying(free, l)).or_else(|| self.alloc_note_from_active(free, l))
        } else {
            self.alloc_note_from_disabled(chan_pool, l)
                .or_else(|| self.alloc_note_from_disabled(player_pool, l))
                .or_else(|| self.alloc_note_from_disabled(free, l))
                .or_else(|| self.alloc_note_from_decaying(chan_pool, l))
                .or_else(|| self.alloc_note_from_decaying(player_pool, l))
                .or_else(|| self.alloc_note_from_decaying(free, l))
                .or_else(|| self.alloc_note_from_active(chan_pool, l))
                .or_else(|| self.alloc_note_from_active(player_pool, l))
                .or_else(|| self.alloc_note_from_active(free, l))
        };
        if result.is_none() {
            self.sequence_layers[l].bit3 = true;
        }
        result
    }

    /// `Audio_NoteInitAll`.
    pub fn note_init_all(&mut self) {
        self.notes = vec![Note::default(); self.num_notes as usize];
        for i in 0..self.num_notes as usize {
            let buffers = self.misc_pool.alloc(SIZEOF_NOTE_SYNTHESIS_BUFFERS);
            let note = &mut self.notes[i];
            note.note_sub_eu = NoteSampleState::default();
            let ps = &mut note.playback_state;
            ps.priority = 0;
            ps.unk_04 = 0;
            ps.parent_layer = None;
            ps.wanted_parent_layer = None;
            ps.prev_parent_layer = None;
            ps.wave_id = 0;
            ps.attributes.velocity = 0.0;
            ps.adsr_vol_scale_unused = 0;
            ps.adsr.action = 0;
            ps.vibrato_state.active = 0;
            ps.portamento.cur = 0;
            ps.portamento.speed = 0;
            ps.stereo_headset_effects = 0;
            note.start_sample_pos = 0;
            note.synthesis_state.synthesis_buffers = buffers;
        }
    }
}

/// `Audio_NoteSetResamplingRate`.
pub fn note_set_resampling_rate(s: &mut NoteSampleState, resampling_rate_input: f32) {
    let resampling_rate;
    if resampling_rate_input < 2.0f32 {
        s.has_two_parts = false;
        resampling_rate = resampling_rate_input.min(1.99998f32);
    } else {
        s.has_two_parts = true;
        resampling_rate = if resampling_rate_input > 3.99996f32 { 1.99998f32 } else { resampling_rate_input * 0.5f32 };
    }
    s.resampling_rate_fixed_point = (resampling_rate * 32768.0f32) as i32 as u16;
}

/// `Audio_AdsrInit`.
pub fn audio_adsr_init(adsr: &mut AdsrState, envelope: u32) {
    adsr.action = 0;
    adsr.delay = 0;
    adsr.envelope = envelope;
    adsr.sustain = 0.0;
    adsr.current = 0.0;
}
