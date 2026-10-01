//! `code_800EC960.c`'s sequence parts: the scene's music (`func_800F5550`), the sequence modes
//! and the enemy music (`Audio_SetSequenceMode`), fanfares and the music they interrupt, the
//! music some actors play nearby (Malon's), the nature ambience and its channels' IO ports, the
//! resets, and `Audio_Update` (`func_800F3054`), which the graph thread runs at the end of every
//! frame.
//!
//! Left out here: the ocarina (`AudioOcarina_*`) and the debug screen (`AudioDebug_*`). The
//! sound effects' side is `sfx`.

use super::sfx::SfxPos;
use super::*;

/// `SfxPlayerState` (`code_800EC960.c`): a sound effect channel's state.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SfxPlayerState {
    pub vol: f32,
    pub freq_scale: f32,
    pub reverb: i8,
    pub pan_signed: i8,
    pub stereo_bits: i8,
    pub filter: u8,
    pub unk_0c: u8,
}

/// `Audio_StartSeq` (`code_800EC960.c`'s, with its casts).
pub fn start_seq(player_idx: u8, fade_timer: u8, seq_id: u16) -> u32 {
    ((player_idx as u32) << 24) | ((fade_timer as u32) << 0x10) | seq_id as u32
}
/// `Audio_SeqCmd7`.
pub fn seq_cmd7(player_idx: u8, a: u8, b: u8) -> u32 {
    0x7000_0000 | ((player_idx as u32) << 0x18) | ((a as u32) << 0x10) | b as u32
}
/// `Audio_SeqCmdC`.
pub fn seq_cmd_c(player_idx: u8, a: u8, b: u8, c: u8) -> u32 {
    0xC000_0000 | ((player_idx as u32) << 24) | ((a as u32) << 16) | ((b as u32) << 8) | c as u32
}
/// `Audio_SeqCmdA`.
pub fn seq_cmd_a(player_idx: u8, a: u16) -> u32 {
    0xA000_0000 | ((player_idx as u32) << 24) | a as u32
}
/// `Audio_SeqCmd1`.
pub fn seq_cmd1(player_idx: u8, a: u8) -> u32 {
    0x1000_00FF | ((player_idx as u32) << 24) | ((a as u32) << 16)
}
/// `Audio_SeqCmdB`.
pub fn seq_cmd_b(player_idx: u8, a: u8, b: u8, c: u8) -> u32 {
    0xB000_0000 | ((player_idx as u32) << 24) | ((a as u32) << 16) | ((b as u32) << 8) | c as u32
}
/// `Audio_SeqCmdB40`.
pub fn seq_cmd_b40(player_idx: u8, a: u8, b: u8) -> u32 {
    0xB000_4000 | ((player_idx as u32) << 24) | ((a as u32) << 16) | b as u32
}
/// `Audio_SeqCmd6`.
pub fn seq_cmd6(player_idx: u8, a: u8, b: u8, c: u8) -> u32 {
    0x6000_0000 | ((player_idx as u32) << 24) | ((a as u32) << 16) | ((b as u32) << 8) | c as u32
}
/// `Audio_SeqCmdE0`.
pub fn seq_cmd_e0(player_idx: u8, a: u8) -> u32 {
    0xE000_0000 | ((player_idx as u32) << 24) | a as u32
}
/// `Audio_SeqCmdE01`.
pub fn seq_cmd_e01(player_idx: u8, a: u16) -> u32 {
    0xE000_0100 | ((player_idx as u32) << 24) | a as u32
}
/// `Audio_SeqCmd8`.
pub fn seq_cmd8(player_idx: u8, a: u8, b: u8, c: u8) -> u32 {
    0x8000_0000 | ((player_idx as u32) << 24) | ((a as u32) << 16) | ((b as u32) << 8) | c as u32
}

impl GameAudio {
    /// `Audio_DisableSeq`.
    fn disable_seq(&mut self, player_idx: u8, fade_out: i32) {
        self.queue_cmd_s32(0x8300_0000 | ((player_idx as u32) << 16), fade_out);
    }

    /// `func_800F3054`: `Audio_Update`, at the end of every graph frame (and as a game state
    /// ends, `GameState_Destroy`), with no positioned sound effects (`audio_update_with`).
    pub fn audio_update(&mut self) {
        self.audio_update_with(&|p| (p == SfxPos::Default).then_some(glam::Vec3::ZERO));
    }

    /// `Audio_Update`, the sound effects' positions read through `pos_of` (an actor's
    /// `projectedPos`; `None` keeps the last read, as a dangling pointer would read on).
    pub fn audio_update_with(&mut self, pos_of: &dyn Fn(SfxPos) -> Option<glam::Vec3>) {
        // What the entries point at, as func_800F8F88 will read it.
        for bank in self.sfx.banks.iter_mut() {
            for e in bank.iter_mut() {
                if let Some(p) = e.pos {
                    if let Some(v) = pos_of(p) {
                        e.pos_now = v;
                    }
                }
            }
        }
        if self.func_800fad34() == 0 {
            // (sAudioUpdateTaskStart, sAudioUpdateStartTime: timing for the debug screen.)
            // AudioOcarina_Update: the ocarina isn't ported.
            Self::step_freq_lerp(&mut self.river_freq_scale_lerp);
            Self::step_freq_lerp(&mut self.waterfall_freq_scale_lerp);
            self.update_river_sound_volumes();
            self.func_800f56a8();
            self.func_800f5cf8();
            if self.audio_spec_id == 7 {
                self.clear_saria_bgm();
            }
            self.process_sfx_requests();
            self.process_seq_cmds();
            self.func_800f8f88();
            self.func_800fa3dc();
            // AudioDebug_SetInput, AudioDebug_ProcessInput: the debug screen.
            self.schedule_process_cmds();
        }
    }

    /// `func_800F314C`.
    pub fn func_800f314c(&mut self, arg0: i8) {
        self.queue_cmd_s32(0x82 << 24 | (SEQ_PLAYER_BGM_MAIN as u32) << 16 | ((arg0 as u8 as u32) << 8), 1);
    }

    /// `Audio_StepFreqLerp`.
    pub fn step_freq_lerp(lerp: &mut FreqLerp) {
        if lerp.remaining_frames != 0 {
            lerp.remaining_frames -= 1;
            if lerp.remaining_frames != 0 {
                lerp.value += lerp.step;
            } else {
                lerp.value = lerp.target;
            }
        }
    }

    /// `func_800F47BC`.
    pub fn func_800f47bc(&mut self) {
        self.set_vol_scale(SEQ_PLAYER_BGM_MAIN, 1, 0, 10);
        self.set_vol_scale(SEQ_PLAYER_BGM_SUB, 1, 0, 10);
    }

    /// `func_800F47FC`.
    pub fn func_800f47fc(&mut self) {
        self.set_vol_scale(SEQ_PLAYER_BGM_MAIN, 1, 0x7F, 3);
        self.set_vol_scale(SEQ_PLAYER_BGM_SUB, 1, 0x7F, 3);
    }

    /// `func_800F483C`.
    pub fn func_800f483c(&mut self, target_vol: u8, vol_fade_timer: u8) {
        self.set_vol_scale(SEQ_PLAYER_BGM_MAIN, 0, target_vol, vol_fade_timer);
    }

    /// `Audio_SetGanonsTowerBgmVolumeLevel`.
    pub fn set_ganons_tower_bgm_volume_level(&mut self, ganons_tower_level: u8) {
        // Ganondorf's Lair
        let pan: i8 = if ganons_tower_level == 0 { 0x7F } else { 0 };
        for channel_idx in 0..16u32 {
            // CHAN_UPD_PAN_UNSIGNED
            self.queue_cmd_s8((0x7 << 24) | ((SEQ_PLAYER_BGM_MAIN as u32) << 16) | (channel_idx << 8), pan);
        }
        // Lowest room in Ganon's Tower (Entrance Room)
        if ganons_tower_level == 7 {
            // Adds a delay to setting the volume in the first room
            self.enter_ganons_tower_timer = 2;
        } else {
            // sGanonsTowerLevelsVol[ganonsTowerLevel % ARRAY_COUNTU(sGanonsTowerLevelsVol)].
            let t = &self.tables.ganons_tower_levels_vol;
            let v = if t.is_empty() { 0 } else { t[ganons_tower_level as usize % t.len()] };
            self.set_ganons_tower_bgm_volume(v);
        }
    }

    /// `Audio_SetGanonsTowerBgmVolume`.
    pub fn set_ganons_tower_bgm_volume(&mut self, target_vol: u8) -> i32 {
        if self.ganons_tower_vol != target_vol {
            // Sets the volume
            self.set_vol_scale(SEQ_PLAYER_BGM_MAIN, 0, target_vol, 2);
            // Sets the filter cutoff of the form (lowPassFilterCutoff << 4) | (highPassFilter & 0xF).
            let low_pass_filter_cutoff: u8 = if target_vol < 0x40 { 1 << 4 } else { (((target_vol - 0x40) >> 2) + 1) << 4 };
            // Set lowPassFilterCutoff to io port 4 from channel 15
            self.queue_seq_cmd(seq_cmd8(SEQ_PLAYER_BGM_MAIN, 4, 15, low_pass_filter_cutoff));
            // Sets the reverb
            for channel_idx in 0..16u32 {
                let ch = self.view.players[SEQ_PLAYER_BGM_MAIN as usize].channels[channel_idx as usize];
                if ch.valid && ch.sound_script_io[5] as u8 != 0xFF {
                    // Higher volume leads to lower reverb
                    let mut reverb = ((ch.sound_script_io[5] as u8 as u16).wrapping_sub(target_vol as u16)).wrapping_add(0x7F);
                    if reverb > 0x7F {
                        reverb = 0x7F;
                    }
                    // CHAN_UPD_REVERB
                    self.queue_cmd_s8((0x5 << 24) | ((SEQ_PLAYER_BGM_MAIN as u32) << 16) | (channel_idx << 8), reverb as u8 as i8);
                }
            }
            self.ganons_tower_vol = target_vol;
        }
        -1
    }

    /// `Audio_LowerMainBgmVolume`: for this frame only.
    pub fn lower_main_bgm_volume(&mut self, volume: u8) {
        self.river_sound_main_bgm_vol = volume;
        self.river_sound_main_bgm_lower = true;
    }

    /// `Audio_UpdateRiverSoundVolumes`.
    pub fn update_river_sound_volumes(&mut self) {
        // Updates Main Bgm Volume (RiverSound of type RS_LOWER_MAIN_BGM_VOLUME)
        if self.river_sound_main_bgm_lower {
            if self.river_sound_main_bgm_current_vol != self.river_sound_main_bgm_vol {
                // lowers the volume for 1 frame
                let v = self.river_sound_main_bgm_vol;
                self.set_vol_scale(SEQ_PLAYER_BGM_MAIN, 0, v, 0xA);
                self.river_sound_main_bgm_current_vol = v;
                self.river_sound_main_bgm_restore = true;
            }
            self.river_sound_main_bgm_lower = false;
        } else if self.river_sound_main_bgm_restore && self.d_80130608 == 0 {
            // restores the volume every frame
            self.set_vol_scale(SEQ_PLAYER_BGM_MAIN, 0, 0x7F, 0xA);
            self.river_sound_main_bgm_current_vol = 0x7F;
            self.river_sound_main_bgm_restore = false;
        }
        // Update Ganon's Tower Volume (RiverSound of type RS_GANON_TOWER_7)
        if self.enter_ganons_tower_timer != 0 {
            self.enter_ganons_tower_timer -= 1;
            if self.enter_ganons_tower_timer == 0 {
                let v = self.tables.ganons_tower_levels_vol.get(7).copied().unwrap_or(0);
                self.set_ganons_tower_bgm_volume(v);
            }
        }
    }

    /// `Audio_ClearSariaBgm`. (`Audio_PlaySariaBgm`, the music an actor plays nearby, isn't
    /// ported: no caller is.)
    pub fn clear_saria_bgm(&mut self) {
        self.saria_bgm_set = false;
    }

    /// `Audio_SplitBgmChannels`: turns the two bgm players' channels on and off by their notes'
    /// priority, splitting them by `vol_split`.
    pub fn split_bgm_channels(&mut self, vol_split: i8) {
        let bgm_players = [SEQ_PLAYER_BGM_MAIN, SEQ_PLAYER_BGM_SUB];
        if self.func_800fa0b4(SEQ_PLAYER_FANFARE) == NA_BGM_DISABLED && self.func_800fa0b4(SEQ_PLAYER_BGM_SUB) != NA_BGM_LONLON {
            for (i, &pl) in bgm_players.iter().enumerate() {
                let volume: u8 = if i == 0 { vol_split as u8 } else { (0x7F - vol_split as i32) as u8 };
                let note_priority: u8 = if volume > 100 {
                    11
                } else if volume < 20 {
                    2
                } else {
                    ((volume - 20) / 10) + 2
                };
                let mut channel_bits: u16 = 0;
                for channel_idx in 0..16 {
                    // The note playing in the channel is of a high enough priority: it stays on.
                    if note_priority > self.view.players[pl as usize].channels[channel_idx].note_priority {
                        channel_bits = channel_bits.wrapping_add(1 << channel_idx);
                    }
                }
                self.queue_seq_cmd(seq_cmd_a(pl, channel_bits));
            }
        }
    }

    /// `func_800F5510`.
    pub fn func_800f5510(&mut self, seq_id: u16) {
        self.func_800f5550(seq_id);
        self.func_800f5e18(SEQ_PLAYER_BGM_MAIN, seq_id, 0, 0, 1);
    }

    /// `func_800F5550`: the scene's music on the main bgm player.
    pub fn func_800f5550(&mut self, seq_id: u16) {
        let mut sp27: u8 = 0;
        if self.func_800fa0b4(SEQ_PLAYER_BGM_MAIN) != NA_BGM_WINDMILL {
            if self.func_800fa0b4(SEQ_PLAYER_BGM_SUB) == NA_BGM_LONLON {
                self.func_800f9474(SEQ_PLAYER_BGM_SUB, 0);
                self.queue_cmd_s32(0xF800_0000, 0);
            }
            let t = self.tables.clone();
            if t.seq_flags(self.d_80130630) & SEQ_FLAG_5 != 0 && t.seq_flags(seq_id as u8) & SEQ_FLAG_4 != 0 {
                if self.d_8013062c & 0x3F != 0 {
                    sp27 = 0x1E;
                }
                let d = self.d_8013062c;
                self.func_800f5e18(SEQ_PLAYER_BGM_MAIN, seq_id, sp27, 7, d as i8);
                self.d_8013062c = 0;
            } else {
                let nv: u8 = if t.seq_flags(seq_id as u8) & SEQ_FLAG_6 != 0 { 1 } else { 0xFF };
                self.func_800f5e18(SEQ_PLAYER_BGM_MAIN, seq_id, 0, 7, nv as i8);
                if t.seq_flags(seq_id as u8) & SEQ_FLAG_5 == 0 {
                    self.d_8013062c = 0xC0;
                }
            }
            self.d_80130630 = seq_id as u8;
        }
    }

    /// `func_800F56A8`: where the music with `SEQ_FLAG_4` has got to (its player's IO port 3),
    /// for the next scene's music to start from (`func_800F5550`).
    pub fn func_800f56a8(&mut self) {
        let temp_v0 = self.func_800fa0b4(SEQ_PLAYER_BGM_MAIN);
        let bvar = temp_v0 as u8;
        if temp_v0 != NA_BGM_DISABLED && self.tables.seq_flags(bvar) & SEQ_FLAG_4 != 0 {
            if self.d_8013062c != 0xC0 {
                self.d_8013062c = self.view.players[SEQ_PLAYER_BGM_MAIN as usize].sound_script_io[3] as u8;
            } else {
                self.d_8013062c = 0;
            }
        }
    }

    /// `func_800F5718`.
    pub fn func_800f5718(&mut self) {
        if self.func_800fa0b4(SEQ_PLAYER_BGM_MAIN) != NA_BGM_WINDMILL {
            self.queue_seq_cmd(start_seq(SEQ_PLAYER_BGM_MAIN, 0, NA_BGM_WINDMILL));
        }
    }

    /// `func_800F574C`.
    pub fn func_800f574c(&mut self, arg0: f32, arg2: u8) {
        if arg0 == 1.0 {
            self.queue_seq_cmd(seq_cmd_b40(SEQ_PLAYER_BGM_MAIN, arg2, 0));
        } else {
            self.queue_seq_cmd(seq_cmd_c(SEQ_PLAYER_FANFARE, 0x30, arg2, (arg0 * 100.0) as u8));
        }
        self.queue_seq_cmd(seq_cmd_c(SEQ_PLAYER_FANFARE, 0xA0, arg2, (arg0 * 100.0) as u8));
    }

    /// `func_800F5918`.
    pub fn func_800f5918(&mut self) {
        if self.func_800fa0b4(SEQ_PLAYER_BGM_MAIN) == NA_BGM_TIMED_MINI_GAME && self.func_800fa11c(0, 0xF000_0000) {
            self.queue_seq_cmd(seq_cmd_b(SEQ_PLAYER_BGM_MAIN, 5, 0, 0xD2));
        }
    }

    /// `func_800F595C`: a fanfare, or the main bgm.
    pub fn func_800f595c(&mut self, arg0: u16) {
        let flags = self.tables.seq_flags(arg0 as u8);
        if flags & SEQ_FLAG_FANFARE != 0 {
            self.play_fanfare(arg0);
        } else if flags & SEQ_FLAG_FANFARE_GANON != 0 {
            self.queue_seq_cmd(start_seq(SEQ_PLAYER_FANFARE, 0, arg0));
        } else {
            self.func_800f5e18(SEQ_PLAYER_BGM_MAIN, arg0, 0, 7, -1);
            self.queue_seq_cmd(seq_cmd1(SEQ_PLAYER_FANFARE, 0));
        }
    }

    /// `func_800F59E8`.
    pub fn func_800f59e8(&mut self, arg0: u16) {
        let flags = self.tables.seq_flags(arg0 as u8);
        if flags & (SEQ_FLAG_FANFARE | SEQ_FLAG_FANFARE_GANON) != 0 {
            self.queue_seq_cmd(seq_cmd1(SEQ_PLAYER_FANFARE, 0));
        } else {
            self.queue_seq_cmd(seq_cmd1(SEQ_PLAYER_BGM_MAIN, 0));
        }
    }

    /// `func_800F5A58`: whether `arg0` plays (on the fanfare player for a fanfare).
    pub fn func_800f5a58(&self, arg0: u8) -> bool {
        let flags = self.tables.seq_flags(arg0);
        let phi_a1 = if flags & (SEQ_FLAG_FANFARE | SEQ_FLAG_FANFARE_GANON) != 0 { 1 } else { 0 };
        arg0 == self.func_800fa0b4(phi_a1) as u8
    }

    /// `func_800F5ACC`: the mini-boss music on the main bgm player, keeping what played.
    pub fn func_800f5acc(&mut self, seq_id: u16) {
        let cur = self.func_800fa0b4(SEQ_PLAYER_BGM_MAIN);
        if (cur & 0xFF) != NA_BGM_GANON_TOWER && (cur & 0xFF) != NA_BGM_ESCAPE && cur != seq_id {
            self.set_sequence_mode(SEQ_MODE_IGNORE);
            if cur != NA_BGM_DISABLED {
                self.prev_main_bgm_seq_id = cur;
            } else {
                log::debug!("Middle Boss BGM Start not stack");
            }
            self.queue_seq_cmd(start_seq(SEQ_PLAYER_BGM_MAIN, 0, seq_id));
        }
    }

    /// `func_800F5B58`: restores what `func_800F5ACC` replaced.
    pub fn func_800f5b58(&mut self) {
        if self.func_800fa0b4(SEQ_PLAYER_BGM_MAIN) != NA_BGM_DISABLED
            && self.prev_main_bgm_seq_id != NA_BGM_DISABLED
            && self.tables.seq_flags(self.func_800fa0b4(SEQ_PLAYER_BGM_MAIN) as u8) & SEQ_FLAG_RESTORE != 0
        {
            if self.prev_main_bgm_seq_id == NA_BGM_DISABLED {
                self.queue_seq_cmd(seq_cmd1(SEQ_PLAYER_BGM_MAIN, 0));
            } else {
                let s = self.prev_main_bgm_seq_id;
                self.queue_seq_cmd(start_seq(SEQ_PLAYER_BGM_MAIN, 0, s));
            }
            self.prev_main_bgm_seq_id = NA_BGM_DISABLED;
        }
    }

    /// `func_800F5BF0`: the nature ambience on the main bgm player, keeping what played.
    pub fn func_800f5bf0(&mut self, nature_ambience_id: u8) {
        let cur = self.func_800fa0b4(SEQ_PLAYER_BGM_MAIN);
        if cur != NA_BGM_NATURE_AMBIENCE {
            self.prev_main_bgm_seq_id = cur;
        }
        self.play_nature_ambience_sequence(nature_ambience_id);
    }

    /// `func_800F5C2C`: restores what `func_800F5BF0` replaced.
    pub fn func_800f5c2c(&mut self) {
        if self.prev_main_bgm_seq_id != NA_BGM_DISABLED {
            let s = self.prev_main_bgm_seq_id;
            self.queue_seq_cmd(start_seq(SEQ_PLAYER_BGM_MAIN, 0, s));
        }
        self.prev_main_bgm_seq_id = NA_BGM_DISABLED;
    }

    /// `Audio_PlayFanfare`: starts in a frame (`func_800F5CF8`), or in five once the fanfare
    /// playing with another font stops.
    pub fn play_fanfare(&mut self, seq_id: u16) {
        let sp26 = self.func_800fa0b4(SEQ_PLAYER_FANFARE);
        let sp1c = self.func_800e5e84(sp26 as u8).first().copied();
        let sp18 = self.func_800e5e84(seq_id as u8).first().copied();
        if sp26 == NA_BGM_DISABLED || sp1c == sp18 {
            self.d_8016b9f4 = 1;
        } else {
            self.d_8016b9f4 = 5;
            self.queue_seq_cmd(seq_cmd1(SEQ_PLAYER_FANFARE, 0));
        }
        self.d_8016b9f6 = seq_id;
    }

    /// `func_800F5CF8`: the fanfare's start, the bgm players turned down under it.
    pub fn func_800f5cf8(&mut self) {
        if self.d_8016b9f4 != 0 {
            self.d_8016b9f4 -= 1;
            if self.d_8016b9f4 == 0 {
                self.queue_cmd_s32(0xE300_0000, SEQUENCE_TABLE);
                self.queue_cmd_s32(0xE300_0000, FONT_TABLE);
                self.func_800fa0b4(SEQ_PLAYER_BGM_MAIN);
                let sp26 = self.func_800fa0b4(SEQ_PLAYER_FANFARE);
                let sp22 = self.func_800fa0b4(SEQ_PLAYER_BGM_SUB);
                if sp26 == NA_BGM_DISABLED {
                    self.set_vol_scale(SEQ_PLAYER_BGM_MAIN, 1, 0, 5);
                    self.set_vol_scale(SEQ_PLAYER_BGM_SUB, 1, 0, 5);
                    self.queue_seq_cmd(seq_cmd_c(SEQ_PLAYER_FANFARE, 0x80, 1, 0xA));
                    self.queue_seq_cmd(seq_cmd_c(SEQ_PLAYER_FANFARE, 0x83, 1, 0xA));
                    self.queue_seq_cmd(seq_cmd_c(SEQ_PLAYER_FANFARE, 0x90, 0, 0));
                    if sp22 != NA_BGM_LONLON {
                        self.queue_seq_cmd(seq_cmd_c(SEQ_PLAYER_FANFARE, 0x93, 0, 0));
                    }
                }
                let s = self.d_8016b9f6;
                self.queue_seq_cmd(start_seq(SEQ_PLAYER_FANFARE, 1, s));
                self.queue_seq_cmd(seq_cmd_a(0, 0xFFFF));
                if sp22 != NA_BGM_LONLON {
                    self.queue_seq_cmd(seq_cmd_a(SEQ_PLAYER_BGM_SUB, 0xFFFF));
                }
            }
        }
    }

    /// `func_800F5E18`: sets the player's IO port `arg3` to `arg4`, then starts `seq_id`.
    pub fn func_800f5e18(&mut self, player_idx: u8, seq_id: u16, fade_timer: u8, arg3: i8, arg4: i8) {
        self.queue_seq_cmd(seq_cmd7(player_idx, arg3 as u8, arg4 as u8));
        self.queue_seq_cmd(start_seq(player_idx, fade_timer, seq_id));
    }

    /// `Audio_SetSequenceMode`: the enemy music on the sub bgm player, or the field music's
    /// moving and still variants (the main player's IO port 2). Player calls it every frame.
    pub fn set_sequence_mode(&mut self, mut seq_mode: u8) {
        self.seq_mode_input = seq_mode;
        if self.prev_main_bgm_seq_id == NA_BGM_DISABLED {
            if self.audio_cutscene_flag != 0 {
                seq_mode = SEQ_MODE_IGNORE;
            }
            let seq_id = self.active[SEQ_PLAYER_BGM_MAIN as usize].unk_254;
            if seq_id == NA_BGM_FIELD_LOGIC && self.func_800fa0b4(SEQ_PLAYER_BGM_SUB) == (NA_BGM_ENEMY | 0x800) {
                seq_mode = SEQ_MODE_IGNORE;
            }
            if seq_id == NA_BGM_DISABLED || self.tables.seq_flags(seq_id as u8) & SEQ_FLAG_ENEMY != 0 || (self.prev_seq_mode & 0x7F) == SEQ_MODE_ENEMY {
                if seq_mode != (self.prev_seq_mode & 0x7F) {
                    if seq_mode == SEQ_MODE_ENEMY {
                        // Start playing enemy bgm
                        let d = self.active[SEQ_PLAYER_BGM_SUB as usize].vol_scales[1] as i32 - self.audio_enemy_vol as i32;
                        let volume_fade_in_timer = d.abs();
                        let ev = self.audio_enemy_vol as u8;
                        self.set_vol_scale(SEQ_PLAYER_BGM_SUB, 3, ev, volume_fade_in_timer as u8);
                        self.queue_seq_cmd(start_seq(SEQ_PLAYER_BGM_SUB, 10, NA_BGM_ENEMY | 0x800));
                        if seq_id != NA_BGM_NATURE_AMBIENCE {
                            self.set_vol_scale(SEQ_PLAYER_BGM_MAIN, 3, ((0x7F - self.audio_enemy_vol as i32) & 0xFF) as u8, 0xA);
                            let ev = self.audio_enemy_vol;
                            self.split_bgm_channels(ev);
                        }
                    } else if (self.prev_seq_mode & 0x7F) == SEQ_MODE_ENEMY {
                        // Stop playing enemy bgm
                        self.queue_seq_cmd(seq_cmd1(SEQ_PLAYER_BGM_SUB, 10));
                        let volume_fade_out_timer = if seq_mode == SEQ_MODE_IGNORE { 0 } else { 10 };
                        self.set_vol_scale(SEQ_PLAYER_BGM_MAIN, 3, 0x7F, volume_fade_out_timer);
                        self.split_bgm_channels(0);
                    }
                    self.prev_seq_mode = seq_mode.wrapping_add(0x80);
                }
            } else {
                // Hyrule Field plays slightly different music standing still and moving.
                if seq_mode == SEQ_MODE_DEFAULT {
                    if self.prev_seq_mode == SEQ_MODE_STILL {
                        self.num_frames_moving = 0;
                    }
                    self.num_frames_still = 0;
                    self.num_frames_moving += 1;
                } else {
                    self.num_frames_still += 1;
                }
                if seq_mode == SEQ_MODE_STILL && self.num_frames_still < 30 && self.num_frames_moving > 20 {
                    seq_mode = SEQ_MODE_DEFAULT;
                }
                self.prev_seq_mode = seq_mode;
                self.queue_seq_cmd(seq_cmd7(SEQ_PLAYER_BGM_MAIN, 2, seq_mode));
            }
        }
    }

    /// `Audio_SetBgmEnemyVolume`: the enemy music louder the nearer the enemy.
    pub fn set_bgm_enemy_volume(&mut self, dist: f32) {
        if self.prev_seq_mode == (0x80 | SEQ_MODE_ENEMY) {
            if dist != self.audio_enemy_dist {
                let adj_dist = if dist < 150.0 {
                    0.0
                } else if dist > 500.0 {
                    350.0
                } else {
                    dist - 150.0
                };
                self.audio_enemy_vol = (((350.0 - adj_dist) * 127.0) / 350.0) as i32 as i8;
                let ev = self.audio_enemy_vol as u8;
                self.set_vol_scale(SEQ_PLAYER_BGM_SUB, 3, ev, 10);
                if self.active[SEQ_PLAYER_BGM_MAIN as usize].unk_254 != NA_BGM_NATURE_AMBIENCE {
                    self.set_vol_scale(SEQ_PLAYER_BGM_MAIN, 3, (0x7F - self.audio_enemy_vol as i32) as u8, 10);
                }
            }
            if self.active[SEQ_PLAYER_BGM_MAIN as usize].unk_254 != NA_BGM_NATURE_AMBIENCE {
                let ev = self.audio_enemy_vol;
                self.split_bgm_channels(ev);
            }
        }
        self.audio_enemy_dist = dist;
    }

    /// `func_800F64E0`: a message box opens (`arg0` 1) or closes; the players are muted
    /// meanwhile. (Its window sound effects come with milestone 3.)
    pub fn func_800f64e0(&mut self, arg0: u8) {
        self.d_80130608 = arg0 as i8;
        if arg0 != 0 {
            // Audio_PlaySfxGeneral(NA_SE_SY_WIN_OPEN): milestone 3.
            self.queue_cmd_s32(0xF100_0000, 0);
        } else {
            // Audio_PlaySfxGeneral(NA_SE_SY_WIN_CLOSE): milestone 3.
            self.queue_cmd_s32(0xF200_0000, 0);
        }
    }

    /// `Audio_SetEnvReverb`.
    pub fn set_env_reverb(&mut self, reverb: i8) {
        self.audio_env_reverb = reverb & 0x7F;
    }

    /// `Audio_SetCodeReverb`.
    pub fn set_code_reverb(&mut self, reverb: i8) {
        if reverb != 0 {
            self.audio_code_reverb = reverb & 0x7F;
        }
    }

    /// `func_800F6700`: the options' sound setting.
    pub fn func_800f6700(&mut self, audio_setting: i8) {
        let sound_mode_index = match audio_setting {
            1 => {
                self.sound_mode = SOUNDMODE_MONO;
                3
            }
            2 => {
                self.sound_mode = SOUNDMODE_HEADSET;
                1
            }
            3 => {
                self.sound_mode = SOUNDMODE_SURROUND;
                0
            }
            _ => {
                self.sound_mode = SOUNDMODE_STEREO;
                0
            }
        };
        self.queue_seq_cmd(seq_cmd_e0(SEQ_PLAYER_BGM_MAIN, sound_mode_index));
    }

    /// `Audio_SetExtraFilter`.
    pub fn set_extra_filter(&mut self, filter: u8) {
        self.audio_extra_filter2 = filter;
        self.audio_extra_filter = filter;
        if self.active[SEQ_PLAYER_BGM_MAIN as usize].unk_254 == NA_BGM_NATURE_AMBIENCE {
            for t in 0..16u32 {
                // CHAN_UPD_SCRIPT_IO (seq player 0, all channels, slot 6)
                self.queue_cmd_s8((0x6 << 24) | ((SEQ_PLAYER_BGM_MAIN as u32) << 16) | ((t & 0xFF) << 8) | 6, filter as i8);
            }
        }
    }

    /// `Audio_SetCutsceneFlag`.
    pub fn set_cutscene_flag(&mut self, flag: i8) {
        self.audio_cutscene_flag = flag;
    }

    /// `func_800F6964`: fades every player out over `arg0` frames (leaving a scene, the file
    /// select), the sound effects' channels but the system's and the ocarina's.
    pub fn func_800f6964(&mut self, arg0: u16) {
        self.queue_seq_cmd(seq_cmd1(SEQ_PLAYER_BGM_MAIN, ((arg0 as u32 * 3) / 2) as u8));
        self.queue_seq_cmd(seq_cmd1(SEQ_PLAYER_FANFARE, ((arg0 as u32 * 3) / 2) as u8));
        for channel_idx in 0..16u8 {
            let skip = match channel_idx {
                SFX_CHANNEL_SYSTEM0 | SFX_CHANNEL_SYSTEM1 => self.audio_spec_id == 10,
                SFX_CHANNEL_OCARINA => true,
                _ => false,
            };
            if !skip {
                self.queue_seq_cmd(seq_cmd6(SEQ_PLAYER_SFX, (arg0 >> 1) as u8, channel_idx, 0));
            }
        }
        self.queue_seq_cmd(seq_cmd1(SEQ_PLAYER_BGM_SUB, ((arg0 as u32 * 3) / 2) as u8));
    }

    /// `func_800F6AB0`.
    pub fn func_800f6ab0(&mut self, arg0: u16) {
        self.queue_seq_cmd(seq_cmd1(SEQ_PLAYER_BGM_MAIN, arg0 as u8));
        self.queue_seq_cmd(seq_cmd1(SEQ_PLAYER_FANFARE, arg0 as u8));
        self.queue_seq_cmd(seq_cmd1(SEQ_PLAYER_BGM_SUB, arg0 as u8));
        self.set_vol_scale(SEQ_PLAYER_BGM_MAIN, 3, 0x7F, 0);
        self.set_vol_scale(SEQ_PLAYER_BGM_MAIN, 1, 0x7F, 0);
    }

    /// `func_800F6B3C`: the sound effects' sequence, again.
    pub fn func_800f6b3c(&mut self) {
        self.func_800f9280(SEQ_PLAYER_SFX, 0, 0xFF, 5);
    }

    /// `Audio_DisableAllSeq`.
    pub fn disable_all_seq(&mut self) {
        self.disable_seq(SEQ_PLAYER_BGM_MAIN, 0);
        self.disable_seq(SEQ_PLAYER_FANFARE, 0);
        self.disable_seq(SEQ_PLAYER_SFX, 0);
        self.disable_seq(SEQ_PLAYER_BGM_SUB, 0);
        self.schedule_process_cmds();
    }

    /// `func_800F6BB8`: the notes sounding.
    pub fn func_800f6bb8(&self) -> i8 {
        self.view.sounding_notes as i8
    }

    /// `func_800F6C34`: the game's side reset (a spec change, `Audio_InitSound`).
    pub fn func_800f6c34(&mut self) {
        self.prev_seq_mode = 0;
        self.d_8016b7a8 = 1.0;
        self.d_8016b7b0 = 1.0;
        self.audio_base_filter = 0;
        self.audio_extra_filter = 0;
        self.audio_base_filter2 = 0;
        self.audio_extra_filter2 = 0;
        // AudioOcarina_SetInstrument(OCARINA_INSTRUMENT_OFF): the ocarina isn't ported (and no
        // instrument is playing).
        self.river_freq_scale_lerp.remaining_frames = 0;
        self.waterfall_freq_scale_lerp.remaining_frames = 0;
        self.river_freq_scale_lerp.value = 1.0;
        self.waterfall_freq_scale_lerp.value = 1.0;
        self.d_8016b7d8 = 1.0;
        self.river_sound_main_bgm_vol = 0x7F;
        self.river_sound_main_bgm_current_vol = 0x7F;
        self.river_sound_main_bgm_lower = false;
        self.river_sound_main_bgm_restore = false;
        self.ganons_tower_vol = 0xFF;
        self.d_8016b9d8 = 0;
        // sSpecReverbs[gAudioSpecId] (20 of them, for the 18 specs).
        self.spec_reverb = self.tables.spec_reverbs.get(self.audio_spec_id as usize).copied().unwrap_or(0);
        self.d_80130608 = 0;
        self.prev_main_bgm_seq_id = NA_BGM_DISABLED;
        self.queue_cmd_s8((0x46 << 24) | ((SEQ_PLAYER_BGM_MAIN as u32) << 16), -1);
        self.saria_bgm_set = false;
        self.d_8016b9f4 = 0;
        self.d_8016b9f3 = 1;
        self.d_8016b9f2 = 0;
    }

    /// `Audio_ResetSfxChannelState`.
    pub fn reset_sfx_channel_state(&mut self) {
        for s in self.sfx_channel_state.iter_mut() {
            *s = SfxPlayerState { vol: 1.0, freq_scale: 1.0, reverb: 0, pan_signed: 0x40, stereo_bits: 0, filter: 0xFF, unk_0c: 0xFF };
        }
        self.sfx_channel_state[SFX_CHANNEL_OCARINA as usize].unk_0c = 0;
        self.prev_seq_mode = 0;
        self.audio_code_reverb = 0;
    }

    /// `Audio_PlayCutsceneEffectsSequence`.
    pub fn play_cutscene_effects_sequence(&mut self, cs_effect_type: u8) {
        if self.sfx_bank_muted[0] != 1 {
            self.queue_seq_cmd(start_seq(SEQ_PLAYER_BGM_SUB, 0, NA_BGM_CUTSCENE_EFFECTS));
            self.queue_seq_cmd(seq_cmd8(SEQ_PLAYER_BGM_SUB, 0, 0, cs_effect_type));
        }
    }

    /// `Audio_SetNatureAmbienceChannelIO`: IO port `port` of the ambience's channels in
    /// `channel_idx_range` (first << 4 | last) to `val`.
    pub fn set_nature_ambience_channel_io(&mut self, channel_idx_range: u8, port: u8, val: u8) {
        if self.active[SEQ_PLAYER_BGM_MAIN as usize].unk_254 != NA_BGM_NATURE_AMBIENCE && self.func_800fa11c(1, 0xF000_00FF) {
            // (sAudioNatureFailed = true: only the debug screen reads it.)
            return;
        }
        // channelIdxRange = 01 on port 1
        if ((channel_idx_range as u32) << 8) + port as u32 == ((NATURE_CHANNEL_CRITTER_0 as u32) << 8) + CHANNEL_IO_PORT_1 as u32 && self.func_800fa0b4(SEQ_PLAYER_BGM_SUB) != NA_BGM_LONLON {
            self.d_8016b9d8 = 0;
        }
        let mut first = channel_idx_range >> 4;
        let last = channel_idx_range & 0xF;
        if first == 0 {
            first = channel_idx_range & 0xF;
        }
        for channel_idx in first..=last {
            self.queue_seq_cmd(seq_cmd8(SEQ_PLAYER_BGM_MAIN, port, channel_idx, val));
        }
    }

    /// `Audio_StartNatureAmbienceSequence`.
    pub fn start_nature_ambience_sequence(&mut self, player_io: u16, channel_mask: u16) {
        if self.func_800fa0b4(SEQ_PLAYER_BGM_MAIN) == NA_BGM_WINDMILL {
            self.play_cutscene_effects_sequence(0xF); // SEQ_CS_EFFECTS_RAINFALL
            return;
        }
        self.queue_seq_cmd(seq_cmd7(SEQ_PLAYER_BGM_MAIN, 0, 1));
        self.queue_seq_cmd(seq_cmd7(SEQ_PLAYER_BGM_MAIN, 4, (player_io >> 8) as u8));
        self.queue_seq_cmd(seq_cmd7(SEQ_PLAYER_BGM_MAIN, 5, player_io as u8));
        self.set_vol_scale(SEQ_PLAYER_BGM_MAIN, 0, 0x7F, 1);

        let mut start_disabled = false;
        if self.d_80133408 != 0 {
            start_disabled = true;
            self.queue_seq_cmd(seq_cmd_e01(SEQ_PLAYER_BGM_MAIN, 0));
        }
        self.queue_seq_cmd(start_seq(SEQ_PLAYER_BGM_MAIN, 0, NA_BGM_NATURE_AMBIENCE));
        if start_disabled {
            self.queue_seq_cmd(seq_cmd_e01(SEQ_PLAYER_BGM_MAIN, 1));
        }
        for channel_idx in 0..16u8 {
            if channel_mask & (1 << channel_idx) == 0 && player_io & (1 << channel_idx) != 0 {
                self.queue_seq_cmd(seq_cmd8(SEQ_PLAYER_BGM_MAIN, CHANNEL_IO_PORT_1, channel_idx, 1));
            }
        }
    }

    /// `Audio_PlayNatureAmbienceSequence`: the nature ambience `nature_ambience_id`
    /// (`sNatureAmbienceDataIO`) on the main bgm player, unless its music has
    /// `SEQ_FLAG_NO_AMBIENCE`.
    pub fn play_nature_ambience_sequence(&mut self, nature_ambience_id: u8) {
        let cur = self.active[SEQ_PLAYER_BGM_MAIN as usize].unk_254;
        if !(cur != NA_BGM_DISABLED && self.tables.seq_flags(cur as u8) & SEQ_FLAG_NO_AMBIENCE != 0) {
            let t = self.tables.clone();
            let Some(data) = t.nature_ambience.get(nature_ambience_id as usize) else {
                log::warn!("no nature ambience {nature_ambience_id}");
                return;
            };
            self.start_nature_ambience_sequence(data.player_io, data.channel_mask);
            let io = &data.channel_io;
            let mut i = 0usize;
            while io.get(i).copied().unwrap_or(0xFF) != 0xFF && i < 100 {
                let channel_idx = io[i] as u32;
                let port = io.get(i + 1).copied().unwrap_or(0) as u32;
                let val = io.get(i + 2).copied().unwrap_or(0) as u32;
                i += 3;
                self.queue_seq_cmd(0x8000_0000 | ((SEQ_PLAYER_BGM_MAIN as u32) << 24) | (port << 0x10) | (channel_idx << 8) | val);
            }
            let m = self.sound_mode as u8;
            self.queue_seq_cmd(seq_cmd8(SEQ_PLAYER_BGM_MAIN, CHANNEL_IO_PORT_7, NATURE_CHANNEL_UNK, m));
        }
    }

    /// `Audio_InitSound`: the game's side at boot, and the sound effects' sequence started.
    pub fn audio_init_sound(&mut self) {
        self.func_800f6c34();
        // AudioOcarina_ResetStaffs: the ocarina isn't ported.
        self.reset_sfx_channel_state();
        self.func_800faeb4();
        self.reset_sfx();
        self.func_800f9280(SEQ_PLAYER_SFX, 0, 0x70, 10);
    }

    /// `func_800F7170`: the sound effects' sequence again after a reset.
    pub fn func_800f7170(&mut self) {
        self.func_800f9280(SEQ_PLAYER_SFX, 0, 0x70, 1);
        self.queue_cmd_s32(0xF200_0000, 1);
        self.schedule_process_cmds();
        self.queue_cmd_s32(0xF800_0000, 0);
    }

    /// `func_800F71BC`: a spec change starts: the game's side reset, and `Audio_Update` waits
    /// for the audio side's reset (`D_80133418`).
    pub fn func_800f71bc(&mut self, _arg0: i32) {
        self.d_80133418 = 1;
        self.func_800f6c34();
        // AudioOcarina_ResetStaffs: the ocarina isn't ported.
        self.reset_sfx_channel_state();
        self.func_800fadf8();
        self.reset_sfx();
    }

    /// `func_800F7208`.
    pub fn func_800f7208(&mut self) {
        self.func_800fadf8();
        self.queue_cmd_s32(0xF200_0000, 1);
        self.func_800f6c34();
        self.reset_sfx_channel_state();
        self.func_800f9280(SEQ_PLAYER_SFX, 0, 0x70, 1);
    }
}
