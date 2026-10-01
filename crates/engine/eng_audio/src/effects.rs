//! `audio_effects.c`: what a player's fade and a channel's volume, pan and bend make of their
//! layers' notes, portamento, vibrato, and the ADSR envelopes.

use crate::context::*;

impl AudioContext {
    /// `Audio_SequenceChannelProcessSound`.
    pub fn sequence_channel_process_sound(&mut self, c: ChanId, recalculate_volume: bool, apply_bend: bool) {
        let p = self.channels[c].seq_player.expect("a player's channel");
        let (applied_fade_volume, muted, mute_volume_scale, bend) = {
            let sp = &self.seq_players[p];
            (sp.applied_fade_volume, sp.muted, sp.mute_volume_scale, sp.bend)
        };
        let ch = &mut self.channels[c];
        if ch.changes & CHANGES_VOLUME != 0 || recalculate_volume {
            let mut channel_volume = ch.volume * ch.volume_scale * applied_fade_volume;
            if muted && (ch.mute_behavior & MUTE_BEHAVIOR_SOFTEN) != 0 {
                channel_volume *= mute_volume_scale;
            }
            ch.applied_volume = channel_volume * channel_volume;
        }
        if ch.changes & CHANGES_PAN != 0 {
            ch.pan = ch.new_pan as i32 * ch.pan_channel_weight as i32;
        }
        let mut chan_freq_scale = ch.freq_scale;
        if apply_bend {
            chan_freq_scale *= bend;
            ch.changes |= CHANGES_FREQ_SCALE;
        }
        let ch = self.channels[c].clone();
        for l in ch.layers.iter().flatten() {
            let layer = &mut self.sequence_layers[*l];
            if layer.enabled && layer.note.is_some() {
                let pan = || ((ch.pan + layer.pan as i32 * (0x80 - ch.pan_channel_weight as i32)) >> 7) as u8;
                if layer.note_properties_need_init {
                    layer.note_freq_scale = layer.freq_scale * chan_freq_scale;
                    layer.note_velocity = layer.velocity_square2 * ch.applied_volume;
                    layer.note_pan = pan();
                    layer.note_properties_need_init = false;
                } else {
                    if ch.changes & CHANGES_FREQ_SCALE != 0 {
                        layer.note_freq_scale = layer.freq_scale * chan_freq_scale;
                    }
                    if ch.changes & CHANGES_VOLUME != 0 || recalculate_volume {
                        layer.note_velocity = layer.velocity_square2 * ch.applied_volume;
                    }
                    if ch.changes & CHANGES_PAN != 0 {
                        layer.note_pan = pan();
                    }
                }
            }
        }
        self.channels[c].changes = 0;
    }

    /// `Audio_SequencePlayerProcessSound`.
    pub fn sequence_player_process_sound(&mut self, p: usize) {
        let sp = &mut self.seq_players[p];
        if sp.fade_timer != 0 {
            sp.fade_volume += sp.fade_velocity;
            sp.recalculate_volume = true;
            if sp.fade_volume > 1.0 {
                sp.fade_volume = 1.0;
            }
            if sp.fade_volume < 0.0 {
                sp.fade_volume = 0.0;
            }
            sp.fade_timer -= 1;
            if sp.fade_timer == 0 && sp.state == 2 {
                self.sequence_player_disable(p);
                return;
            }
        }
        let sp = &mut self.seq_players[p];
        if sp.recalculate_volume {
            sp.applied_fade_volume = sp.fade_volume * sp.fade_volume_scale;
        }
        let (recalc, bend) = (sp.recalculate_volume, sp.apply_bend);
        for i in 0..16 {
            let c = self.seq_players[p].channels[i];
            if self.channels[c].enabled {
                self.sequence_channel_process_sound(c, recalc, bend);
            }
        }
        self.seq_players[p].recalculate_volume = false;
    }

    /// `Audio_GetPortamentoFreqScale`.
    pub fn get_portamento_freq_scale(&self, portamento: &mut Portamento) -> f32 {
        portamento.cur = portamento.cur.wrapping_add(portamento.speed);
        let mut lo_res_cur = ((portamento.cur >> 8) & 0xFF) as usize;
        if lo_res_cur >= 127 {
            lo_res_cur = 127;
            portamento.mode = 0;
        }
        1.0f32 + portamento.extent * (self.tables.bend_pitch_one_octave_frequencies[lo_res_cur + 128] - 1.0f32)
    }

    /// `Audio_GetVibratoPitchChange`.
    pub fn get_vibrato_pitch_change(&self, vib: &mut VibratoState) -> i16 {
        vib.time = vib.time.wrapping_add(vib.rate as i32 as u32);
        let index = (vib.time >> 10) & 0x3F;
        self.ram.s16(vib.curve + 2 * index)
    }

    /// `Audio_GetVibratoFreqScale`.
    pub fn get_vibrato_freq_scale(&mut self, n: NoteId) -> f32 {
        let mut vib = self.notes[n].playback_state.vibrato_state;
        if vib.delay != 0 {
            vib.delay -= 1;
            self.notes[n].playback_state.vibrato_state = vib;
            return 1.0;
        }
        // @bug (game): this probably meant to compare with gAudioContext.sequenceChannelNone.
        // -1 isn't used as a channel pointer anywhere else.
        let channel = &self.channels[vib.channel.unwrap_or(CHANNEL_NONE)];
        if vib.extent_change_timer != 0 {
            if vib.extent_change_timer == 1 {
                vib.extent = channel.vibrato_extent_target as i32 as f32;
            } else {
                vib.extent += (channel.vibrato_extent_target as i32 as f32 - vib.extent) / vib.extent_change_timer as i32 as f32;
            }
            vib.extent_change_timer -= 1;
        } else if channel.vibrato_extent_target as i32 != vib.extent as i32 {
            vib.extent_change_timer = channel.vibrato_extent_change_delay;
            if vib.extent_change_timer == 0 {
                vib.extent = channel.vibrato_extent_target as i32 as f32;
            }
        }
        if vib.rate_change_timer != 0 {
            if vib.rate_change_timer == 1 {
                vib.rate = channel.vibrato_rate_target as i32 as f32;
            } else {
                vib.rate += (channel.vibrato_rate_target as i32 as f32 - vib.rate) / vib.rate_change_timer as i32 as f32;
            }
            vib.rate_change_timer -= 1;
        } else if channel.vibrato_rate_target as i32 != vib.rate as i32 {
            vib.rate_change_timer = channel.vibrato_rate_change_delay;
            if vib.rate_change_timer == 0 {
                vib.rate = channel.vibrato_rate_target as i32 as f32;
            }
        }

        if vib.extent == 0.0 {
            self.notes[n].playback_state.vibrato_state = vib;
            return 1.0;
        }
        let pitch_change = self.get_vibrato_pitch_change(&mut vib) as f32 + 32768.0f32;
        let temp = vib.extent / 4096.0f32;
        let extent = temp + 1.0f32;
        let inv_extent = 1.0f32 / extent;
        let result = 1.0f32 / ((extent - inv_extent) * pitch_change / 65536.0f32 + inv_extent);
        self.d_80130510 += result;
        self.d_80130514 += 1;
        self.notes[n].playback_state.vibrato_state = vib;
        result
    }

    /// `Audio_NoteVibratoUpdate`.
    pub fn note_vibrato_update(&mut self, n: NoteId) {
        if self.notes[n].playback_state.portamento.mode != 0 {
            let mut p = self.notes[n].playback_state.portamento;
            let f = self.get_portamento_freq_scale(&mut p);
            self.notes[n].playback_state.portamento = p;
            self.notes[n].playback_state.portamento_freq_scale = f;
        }
        if self.notes[n].playback_state.vibrato_state.active != 0 {
            let f = self.get_vibrato_freq_scale(n);
            self.notes[n].playback_state.vibrato_freq_scale = f;
        }
    }

    /// `Audio_NoteVibratoInit`.
    pub fn note_vibrato_init(&mut self, n: NoteId) {
        let layer = self.notes[n].playback_state.parent_layer.expect("vibrato init on a note with a layer");
        let chan = self.sequence_layers[layer].channel.unwrap_or(CHANNEL_NONE);
        let curve = self.statics.wave_samples[2];
        let ch = &self.channels[chan];
        let ps = &mut self.notes[n].playback_state;
        ps.vibrato_freq_scale = 1.0;
        let vib = &mut ps.vibrato_state;
        vib.active = 1;
        vib.time = 0;
        vib.curve = curve;
        vib.channel = Some(chan);
        vib.extent_change_timer = ch.vibrato_extent_change_delay;
        vib.extent = if vib.extent_change_timer == 0 { ch.vibrato_extent_target as i32 as f32 } else { ch.vibrato_extent_start as i32 as f32 };
        vib.rate_change_timer = ch.vibrato_rate_change_delay;
        vib.rate = if vib.rate_change_timer == 0 { ch.vibrato_rate_target as i32 as f32 } else { ch.vibrato_rate_start as i32 as f32 };
        vib.delay = ch.vibrato_delay;
    }

    /// `Audio_NotePortamentoInit`.
    pub fn note_portamento_init(&mut self, n: NoteId) {
        let layer = self.notes[n].playback_state.parent_layer.expect("portamento init on a note with a layer");
        let p = self.sequence_layers[layer].portamento;
        let ps = &mut self.notes[n].playback_state;
        ps.portamento_freq_scale = 1.0;
        ps.portamento = p;
    }

    /// `Audio_AdsrUpdate` on a note's envelope.
    pub fn adsr_update_note(&mut self, n: NoteId) -> f32 {
        let mut adsr = self.notes[n].playback_state.adsr;
        let v = self.adsr_update(&mut adsr);
        self.notes[n].playback_state.adsr = adsr;
        v
    }

    /// `Audio_AdsrUpdate`.
    pub fn adsr_update(&self, adsr: &mut AdsrState) -> f32 {
        let state = adsr.state();
        'sw: {
            match state {
                ADSR_STATE_DISABLED => return 0.0,
                ADSR_STATE_INITIAL | ADSR_STATE_START_LOOP | ADSR_STATE_LOOP | ADSR_STATE_FADE | ADSR_STATE_HANG => {
                    let mut st = state;
                    if st == ADSR_STATE_INITIAL {
                        if adsr.hang() {
                            adsr.set_state(ADSR_STATE_HANG);
                            break 'sw;
                        }
                        st = ADSR_STATE_START_LOOP;
                    }
                    if st == ADSR_STATE_START_LOOP {
                        adsr.env_index = 0;
                        adsr.set_state(ADSR_STATE_LOOP);
                        st = ADSR_STATE_LOOP;
                    }
                    if st == ADSR_STATE_LOOP {
                        // retry:
                        loop {
                            let (delay, arg) = self.ram.envelope_point(adsr.envelope, adsr.env_index as u32);
                            adsr.delay = delay;
                            match delay {
                                ADSR_DISABLE => adsr.set_state(ADSR_STATE_DISABLED),
                                ADSR_HANG => adsr.set_state(ADSR_STATE_HANG),
                                ADSR_GOTO => {
                                    adsr.env_index = arg as u8;
                                    continue;
                                }
                                ADSR_RESTART => adsr.set_state(ADSR_STATE_INITIAL),
                                _ => {
                                    adsr.delay = (adsr.delay as f32 * self.audio_buffer_parameters.updates_per_frame_scaled) as i32 as i16;
                                    if adsr.delay == 0 {
                                        adsr.delay = 1;
                                    }
                                    adsr.target = arg as f32 / 32767.0f32;
                                    adsr.target *= adsr.target;
                                    adsr.velocity = (adsr.target - adsr.current) / adsr.delay as f32;
                                    adsr.set_state(ADSR_STATE_FADE);
                                    adsr.env_index = adsr.env_index.wrapping_add(1);
                                }
                            }
                            break;
                        }
                        if adsr.state() != ADSR_STATE_FADE {
                            break 'sw;
                        }
                        st = ADSR_STATE_FADE;
                    }
                    if st == ADSR_STATE_FADE {
                        adsr.current += adsr.velocity;
                        adsr.delay -= 1;
                        if adsr.delay <= 0 {
                            adsr.set_state(ADSR_STATE_LOOP);
                        }
                    }
                    // ADSR_STATE_HANG: nothing.
                }
                ADSR_STATE_DECAY | ADSR_STATE_RELEASE => {
                    adsr.current -= adsr.fade_out_vel;
                    if adsr.sustain != 0.0 && state == ADSR_STATE_DECAY {
                        if adsr.current < adsr.sustain {
                            adsr.current = adsr.sustain;
                            adsr.delay = 128;
                            adsr.set_state(ADSR_STATE_SUSTAIN);
                        }
                        break 'sw;
                    }
                    if adsr.current < 0.00001f32 {
                        adsr.current = 0.0;
                        adsr.set_state(ADSR_STATE_DISABLED);
                    }
                }
                ADSR_STATE_SUSTAIN => {
                    adsr.delay -= 1;
                    if adsr.delay == 0 {
                        adsr.set_state(ADSR_STATE_RELEASE);
                    }
                }
                _ => {}
            }
        }
        if adsr.decay() {
            adsr.set_state(ADSR_STATE_DECAY);
            adsr.set_decay(false);
        }
        if adsr.release() {
            adsr.set_state(ADSR_STATE_RELEASE);
            adsr.set_release(false);
        }
        if adsr.current < 0.0 {
            return 0.0;
        }
        if adsr.current > 1.0 {
            return 1.0;
        }
        adsr.current
    }
}
