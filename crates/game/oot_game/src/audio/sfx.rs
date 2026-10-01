//! The sound effects: `code_800F7260.c` (the requests, the seven banks of entries, choosing
//! which play on the sound effects player's channels, starting and refreshing them, the stops),
//! `code_800EC960.c`'s sound effect parts (each channel's volume, reverb, pan, frequency, stereo
//! and filter, `Audio_SetSfxProperties`; the helpers that play one with a scale of their own:
//! footsteps, the sword's charge, the river) and `z_lib.c`'s three shorthands.
//!
//! The C passes pointers: a request names where the sound is (`Vec3f*`, usually an actor's
//! `projectedPos`, read again every frame) and where its frequency, volume and reverb are
//! (`f32*`, `s8*`, the audio code's statics). Here those are `SfxPos`, `SfxF32` and `SfxS8`,
//! compared as the C compares the pointers and read as it reads them: the positions once per
//! `Audio_Update`, before `func_800F8F88` reads them (`GameAudio::audio_update_with`).

use glam::Vec3;

use super::*;
use crate::actor_ctx::ActorHandle;

/// `SfxBankType` (`sfx.h`).
pub const BANK_PLAYER: u8 = 0;
pub const BANK_ITEM: u8 = 1;
pub const BANK_ENV: u8 = 2;
pub const BANK_ENEMY: u8 = 3;
pub const BANK_SYSTEM: u8 = 4;
pub const BANK_OCARINA: u8 = 5;
pub const BANK_VOICE: u8 = 6;
/// `ARRAY_COUNT(gSfxBanks)`.
pub const NUM_SFX_BANKS: usize = 7;

/// `SfxState` (`sfx.h`).
pub const SFX_STATE_EMPTY: u8 = 0;
pub const SFX_STATE_QUEUED: u8 = 1;
pub const SFX_STATE_READY: u8 = 2;
pub const SFX_STATE_PLAYING_REFRESH: u8 = 3;
pub const SFX_STATE_PLAYING_1: u8 = 4;
pub const SFX_STATE_PLAYING_2: u8 = 5;

/// `SFX_FLAG` (`sfx.h`): an id's 0x800 bit.
pub const SFX_FLAG: u16 = 0x800;
/// `SfxParams` bit-packing (`sfx.h`).
pub const SFX_PARAM_01_SHIFT: u16 = 0;
pub const SFX_PARAM_01_MASK: u16 = 3 << SFX_PARAM_01_SHIFT;
pub const SFX_FLAG_2: u16 = 1 << 2;
pub const SFX_FLAG_3: u16 = 1 << 3;
pub const SFX_FLAG_4: u16 = 1 << 4;
pub const SFX_FLAG_5: u16 = 1 << 5;
pub const SFX_PARAM_67_SHIFT: u16 = 6;
pub const SFX_PARAM_67_MASK: u16 = 3 << SFX_PARAM_67_SHIFT;
pub const SFX_FLAG_9: u16 = 1 << 9;
pub const SFX_FLAG_10_SHIFT: u16 = 10;
pub const SFX_FLAG_10: u16 = 1 << SFX_FLAG_10_SHIFT;
pub const SFX_FLAG_11: u16 = 1 << 11;
pub const SFX_FLAG_12: u16 = 1 << 12;
pub const SFX_FLAG_13: u16 = 1 << 13;
pub const SFX_FLAG_14: u16 = 1 << 14;
pub const SFX_FLAG_15: u16 = 1 << 15;
/// `MAX_CHANNELS_PER_BANK` (`z64audio.h`).
pub const MAX_CHANNELS_PER_BANK: usize = 3;

/// `SFX_BANK_SHIFT`, `SFX_BANK_MASK`, `SFX_INDEX`, `SFX_BANK` (`sfx.h`).
pub fn sfx_bank_shift(sfx_id: u16) -> u8 {
    ((sfx_id >> 12) & 0xFF) as u8
}
pub fn sfx_bank_mask(sfx_id: u16) -> u16 {
    sfx_id & 0xF000
}
pub fn sfx_index(sfx_id: u16) -> u16 {
    sfx_id & 0x01FF
}
pub fn sfx_bank(sfx_id: u16) -> u8 {
    sfx_bank_shift(sfx_bank_mask(sfx_id))
}

/// Where a sound effect is: the `Vec3f*` the C passes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SfxPos {
    /// `gSfxDefaultPos` (0, 0, 0): the middle of the screen, no position.
    Default,
    /// `&actor->projectedPos`.
    Actor(ActorHandle),
    /// `&play->sfxSources[i].projectedPos` (`z_sfx_source.c`).
    Source(u8),
}

/// Where a sound effect's frequency or volume scale is: the `f32*` the C passes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SfxF32 {
    /// `gSfxDefaultFreqAndVolScale` (1.0).
    One,
    D8016B7A8,
    D8016B7AC,
    D8016B7B0,
    D8016B7D8,
    D8016B7E0,
    D801305B0,
    D801305F4,
    /// `&sRiverFreqScaleLerp.value`, `&sWaterfallFreqScaleLerp.value`.
    RiverFreq,
    WaterfallFreq,
    /// `&gPitchFrequencies[i]`.
    Pitch(u8),
}

/// Where a sound effect's reverb is: the `s8*` the C passes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SfxS8 {
    /// `gSfxDefaultReverb` (0).
    Zero,
    D801305B4,
    D8016B7DC,
}

/// `SfxRequest` (`code_800F7260.c`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SfxRequest {
    pub sfx_id: u16,
    pub pos: SfxPos,
    pub token: u8,
    pub freq_scale: SfxF32,
    pub vol: SfxF32,
    pub reverb_add: SfxS8,
}

impl Default for SfxRequest {
    fn default() -> SfxRequest {
        SfxRequest { sfx_id: 0, pos: SfxPos::Default, token: 0, freq_scale: SfxF32::One, vol: SfxF32::One, reverb_add: SfxS8::Zero }
    }
}

/// `SfxBankEntry` (`sfx.h`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SfxBankEntry {
    /// `posX`, `posY`, `posZ`: `None` for the BSS's NULL.
    pub pos: Option<SfxPos>,
    /// What they point at, as of this `Audio_Update`.
    pub pos_now: Vec3,
    pub token: u8,
    pub freq_scale: SfxF32,
    pub vol: SfxF32,
    pub reverb_add: SfxS8,
    pub dist: f32,
    pub priority: u32,
    pub sfx_importance: u8,
    pub sfx_params: u16,
    pub sfx_id: u16,
    pub state: u8,
    pub freshness: u8,
    pub prev: u8,
    pub next: u8,
    pub channel_idx: u8,
    pub unk_2f: u8,
}

impl Default for SfxBankEntry {
    fn default() -> SfxBankEntry {
        SfxBankEntry {
            pos: None,
            pos_now: Vec3::ZERO,
            token: 0,
            freq_scale: SfxF32::One,
            vol: SfxF32::One,
            reverb_add: SfxS8::Zero,
            dist: 0.0,
            priority: 0,
            sfx_importance: 0,
            sfx_params: 0,
            sfx_id: 0,
            state: 0,
            freshness: 0,
            prev: 0,
            next: 0,
            channel_idx: 0,
            unk_2f: 0,
        }
    }
}

/// `ActiveSfx` (`sfx.h`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ActiveSfx {
    pub priority: u32,
    pub entry_index: u8,
}

/// `UnusedBankLerp` (`code_800F7260.c`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct UnusedBankLerp {
    pub value: f32,
    pub target: f32,
    pub step: f32,
    pub remaining_frames: u16,
}

/// The statics of `code_800F7260.c` and the sound effect parts of `audio_external_data.c` and
/// `code_800EC960.c`.
#[derive(Debug, Clone)]
pub struct SfxStatics {
    /// `gSfxBanks`: `D_8016BAD0[9]` and the six others, sized by `gSfxBankSizes`.
    pub banks: Vec<Vec<SfxBankEntry>>,
    /// `sSfxRequests`, `gSfxRequestWriteIndex`, `gSfxRequestReadIndex`.
    pub requests: Box<[SfxRequest; 0x100]>,
    pub request_write_index: u8,
    pub request_read_index: u8,
    /// `D_801333D0`: the channels muting the bgm (`SFX_FLAG_3`).
    pub d_801333d0: u16,
    pub list_end: [u8; NUM_SFX_BANKS],
    pub free_list_start: [u8; NUM_SFX_BANKS],
    pub unused: [u8; NUM_SFX_BANKS],
    /// `gActiveSfx`.
    pub active: [[ActiveSfx; MAX_CHANNELS_PER_BANK]; NUM_SFX_BANKS],
    /// `sCurSfxPlayerChannelIdx`.
    pub cur_channel_idx: u8,
    pub unused_bank_lerp: [UnusedBankLerp; NUM_SFX_BANKS],
    /// `gAudioSfxSwapSource`, `gAudioSfxSwapTarget`, `gAudioSfxSwapMode` (the debug screen's).
    pub swap_source: [u16; 10],
    pub swap_target: [u16; 10],
    pub swap_mode: [u8; 10],
    /// `gAudioSfxSwapOff`, `D_801333F0` (the debug screen's), `D_801333F8`.
    pub swap_off: u8,
    pub d_801333f0: u8,
    pub d_801333f8: u8,
    // code_800EC960.c
    pub d_801305b0: f32,
    pub d_801305b4: i8,
    pub d_801305b8: i8,
    pub d_801305bc: i8,
    pub d_801305c0: i8,
    pub increasing_transpose: u8,
    pub prev_charge_level: u8,
    pub d_801305f4: f32,
    pub d_8016b7ac: f32,
    pub d_8016b7dc: i8,
    pub d_8016b7e0: f32,
    pub d_80131c8c: f32,
}

impl SfxStatics {
    /// The BSS and the initialisers, the banks sized as `code_800F7260.c` declares them.
    pub fn new(bank_sizes: &[u8]) -> SfxStatics {
        SfxStatics {
            banks: bank_sizes.iter().map(|&n| vec![SfxBankEntry::default(); n as usize]).collect(),
            requests: Box::new([SfxRequest::default(); 0x100]),
            request_write_index: 0,
            request_read_index: 0,
            d_801333d0: 0,
            list_end: [0; NUM_SFX_BANKS],
            free_list_start: [0; NUM_SFX_BANKS],
            unused: [0; NUM_SFX_BANKS],
            active: [[ActiveSfx::default(); MAX_CHANNELS_PER_BANK]; NUM_SFX_BANKS],
            cur_channel_idx: 0,
            unused_bank_lerp: [UnusedBankLerp::default(); NUM_SFX_BANKS],
            swap_source: [0; 10],
            swap_target: [0; 10],
            swap_mode: [0; 10],
            swap_off: 0,
            d_801333f0: 0,
            d_801333f8: 0,
            // code_800EC960.c's initialisers.
            d_801305b0: 0.7950898,
            d_801305b4: 35,
            d_801305b8: 20,
            d_801305bc: 30,
            d_801305c0: 20,
            increasing_transpose: 0,
            prev_charge_level: 0,
            d_801305f4: 1.0,
            d_8016b7ac: 0.0,
            d_8016b7dc: 0,
            d_8016b7e0: 0.0,
            d_80131c8c: 0.0,
        }
    }
}

/// `Audio_RemoveMatchingSfxRequests`' comparand (the fields of an `SfxBankEntry` it reads).
#[derive(Debug, Clone, Copy, Default)]
struct Cmp {
    sfx_id: u16,
    pos: Option<SfxPos>,
    token: u8,
}

impl GameAudio {
    fn bank_count(&self) -> usize {
        self.sfx.banks.len().min(NUM_SFX_BANKS)
    }

    fn channels_per_bank(&self, bank: u8) -> u8 {
        self.tables.channels_per_bank.get(self.sfx_channel_layout as usize).and_then(|r| r.get(bank as usize)).copied().unwrap_or(0)
    }

    fn used_channels_per_bank(&self, bank: u8) -> u8 {
        self.tables.used_channels_per_bank.get(self.sfx_channel_layout as usize).and_then(|r| r.get(bank as usize)).copied().unwrap_or(0)
    }

    /// What an `f32*` points at.
    pub fn sfx_f32(&self, p: SfxF32) -> f32 {
        match p {
            SfxF32::One => 1.0,
            SfxF32::D8016B7A8 => self.d_8016b7a8,
            SfxF32::D8016B7AC => self.sfx.d_8016b7ac,
            SfxF32::D8016B7B0 => self.d_8016b7b0,
            SfxF32::D8016B7D8 => self.d_8016b7d8,
            SfxF32::D8016B7E0 => self.sfx.d_8016b7e0,
            SfxF32::D801305B0 => self.sfx.d_801305b0,
            SfxF32::D801305F4 => self.sfx.d_801305f4,
            SfxF32::RiverFreq => self.river_freq_scale_lerp.value,
            SfxF32::WaterfallFreq => self.waterfall_freq_scale_lerp.value,
            SfxF32::Pitch(i) => self.audio_tables.pitch_frequencies.get(i as usize).copied().unwrap_or(1.0),
        }
    }

    /// What an `s8*` points at.
    pub fn sfx_s8(&self, p: SfxS8) -> i8 {
        match p {
            SfxS8::Zero => 0,
            SfxS8::D801305B4 => self.sfx.d_801305b4,
            SfxS8::D8016B7DC => self.sfx.d_8016b7dc,
        }
    }

    /// `Audio_NextRandom` from the game's thread: `audRand` as the audio side left it, the
    /// count register at the view's time; the new `audRand` goes back to the audio side.
    pub fn next_random(&mut self) -> u32 {
        let v = &self.view;
        let r = v.os_count.wrapping_add(0x123_4567).wrapping_mul(v.aud_rand.wrapping_add(v.total_task_count as u32)).wrapping_add(v.audio_random);
        self.view.aud_rand = r;
        self.op(GameOp::SetAudRand(r));
        r
    }

    // ---------------------------------------------------------------------------------------
    // code_800F7260.c

    /// `Audio_SetSfxBanksMute`.
    pub fn set_sfx_banks_mute(&mut self, mut mute_mask: u16) {
        for b in 0..NUM_SFX_BANKS {
            self.sfx_bank_muted[b] = (mute_mask & 1 != 0) as u8;
            mute_mask >>= 1;
        }
    }

    /// `Audio_QueueSeqCmdMute`: a sound effect with `SFX_FLAG_3` turns the bgm down.
    pub fn queue_seq_cmd_mute(&mut self, channel_idx: u8) {
        self.sfx.d_801333d0 |= 1 << channel_idx;
        self.set_vol_scale(SEQ_PLAYER_BGM_MAIN, 2, 0x40, 0xF);
        self.set_vol_scale(SEQ_PLAYER_BGM_SUB, 2, 0x40, 0xF);
    }

    /// `Audio_ClearBGMMute`.
    pub fn clear_bgm_mute(&mut self, channel_idx: u8) {
        self.sfx.d_801333d0 &= (1u16 << channel_idx) ^ 0xFFFF;
        if self.sfx.d_801333d0 == 0 {
            self.set_vol_scale(SEQ_PLAYER_BGM_MAIN, 2, 0x7F, 0xF);
            self.set_vol_scale(SEQ_PLAYER_BGM_SUB, 2, 0x7F, 0xF);
        }
    }

    /// `Audio_PlaySfxGeneral`: queues a request (`Audio_ProcessSfxRequests` takes it in the
    /// next `Audio_Update`), unless its bank is muted.
    pub fn play_sfx_general(&mut self, mut sfx_id: u16, pos: SfxPos, token: u8, freq_scale: SfxF32, vol: SfxF32, reverb_add: SfxS8) {
        if let Some(l) = &mut self.log {
            l.sfx.push((self.frames, sfx_id, pos));
        }
        if self.sfx_bank_muted.get(sfx_bank_shift(sfx_id) as usize).copied().unwrap_or(0) == 0 {
            if self.sfx.swap_off == 0 {
                for i in 0..10 {
                    if sfx_id == self.sfx.swap_source[i] {
                        if self.sfx.swap_mode[i] == 0 {
                            // "SWAP"
                            sfx_id = self.sfx.swap_target[i];
                        } else {
                            // "ADD"
                            let w = self.sfx.request_write_index as usize;
                            self.sfx.requests[w] = SfxRequest { sfx_id: self.sfx.swap_target[i], pos, token, freq_scale, vol, reverb_add };
                            self.sfx.request_write_index = self.sfx.request_write_index.wrapping_add(1);
                        }
                        break;
                    }
                }
            }
            let w = self.sfx.request_write_index as usize;
            self.sfx.requests[w] = SfxRequest { sfx_id, pos, token, freq_scale, vol, reverb_add };
            self.sfx.request_write_index = self.sfx.request_write_index.wrapping_add(1);
        }
    }

    /// `Audio_RemoveMatchingSfxRequests`.
    fn remove_matching_sfx_requests(&mut self, aspect: u8, cmp: Cmp) {
        let mut i = self.sfx.request_read_index;
        while i != self.sfx.request_write_index {
            let req = &mut self.sfx.requests[i as usize];
            let remove = match aspect {
                0 => sfx_bank_mask(req.sfx_id) == sfx_bank_mask(cmp.sfx_id),
                1 => sfx_bank_mask(req.sfx_id) == sfx_bank_mask(cmp.sfx_id) && Some(req.pos) == cmp.pos,
                2 => Some(req.pos) == cmp.pos,
                3 => Some(req.pos) == cmp.pos && req.sfx_id == cmp.sfx_id,
                4 => req.token == cmp.token && req.sfx_id == cmp.sfx_id,
                5 => req.sfx_id == cmp.sfx_id,
                _ => false,
            };
            if remove {
                req.sfx_id = 0;
            }
            i = i.wrapping_add(1);
        }
    }

    /// `Audio_ProcessSfxRequest`: the request into its bank: refreshing the entry already
    /// playing it at that position, replacing the least important of the bank's channels'
    /// worth there, or into a free entry.
    fn process_sfx_request(&mut self) {
        let req = self.sfx.requests[self.sfx.request_read_index as usize];
        let mut evict_index: u8 = 0x80;
        let mut evict_importance: u8 = 0;
        if req.sfx_id == 0 {
            return;
        }
        let bank = sfx_bank(req.sfx_id);
        let b = bank as usize;
        if b >= self.bank_count() {
            return;
        }
        // ((1 << bankId) & D_801333F0): AudioDebug_ScrPrt, the debug screen.
        let (req_importance, req_params) = self.tables.sfx_params(req.sfx_id);
        let used = self.used_channels_per_bank(bank);
        let mut count: u8 = 0;
        let mut index = self.sfx.banks[b][0].next;
        while index != 0xFF && index != 0 {
            let e = self.sfx.banks[b][index as usize];
            if e.pos == Some(req.pos) {
                if req_params & SFX_FLAG_5 != 0 && req_importance == e.sfx_importance {
                    return;
                }
                if e.sfx_id == req.sfx_id {
                    count = used;
                } else {
                    if count == 0 {
                        evict_index = index;
                        evict_importance = self.tables.sfx_params(e.sfx_id).0;
                    } else if e.sfx_importance < evict_importance {
                        evict_index = index;
                        evict_importance = self.tables.sfx_params(e.sfx_id).0;
                    }
                    count = count.wrapping_add(1);
                    if count == used {
                        index = if req_importance >= evict_importance { evict_index } else { 0 };
                    }
                }
                if count == used {
                    if req.sfx_id & 0xC00 != 0 || req_params & SFX_FLAG_2 != 0 || index == evict_index {
                        let i = index as usize;
                        let t = self.sfx.banks[b][i];
                        if t.sfx_params & SFX_FLAG_3 != 0 && t.state != SFX_STATE_QUEUED {
                            self.clear_bgm_mute(t.channel_idx);
                        }
                        let t = &mut self.sfx.banks[b][i];
                        t.token = req.token;
                        t.sfx_id = req.sfx_id;
                        t.state = SFX_STATE_QUEUED;
                        t.freshness = 2;
                        t.freq_scale = req.freq_scale;
                        t.vol = req.vol;
                        t.reverb_add = req.reverb_add;
                        t.sfx_params = req_params;
                        t.sfx_importance = req_importance;
                    } else if self.sfx.banks[b][index as usize].state == SFX_STATE_PLAYING_2 {
                        self.sfx.banks[b][index as usize].state = SFX_STATE_PLAYING_1;
                    }
                    index = 0;
                }
            }
            if index != 0 {
                index = self.sfx.banks[b][index as usize].next;
            }
        }
        let free = self.sfx.free_list_start[b];
        if self.sfx.banks[b][free as usize].next != 0xFF && index != 0 {
            let index = free;
            let list_end = self.sfx.list_end[b];
            let e = &mut self.sfx.banks[b][index as usize];
            e.pos = Some(req.pos);
            e.token = req.token;
            e.freq_scale = req.freq_scale;
            e.vol = req.vol;
            e.reverb_add = req.reverb_add;
            e.sfx_params = req_params;
            e.sfx_importance = req_importance;
            e.sfx_id = req.sfx_id;
            e.state = SFX_STATE_QUEUED;
            e.freshness = 2;
            e.prev = list_end;
            self.sfx.banks[b][list_end as usize].next = free;
            self.sfx.list_end[b] = free;
            let next_free = self.sfx.banks[b][free as usize].next;
            self.sfx.free_list_start[b] = next_free;
            self.sfx.banks[b][next_free as usize].prev = 0xFF;
            self.sfx.banks[b][index as usize].next = 0xFF;
        }
    }

    /// `Audio_RemoveSfxBankEntry`.
    fn remove_sfx_bank_entry(&mut self, bank: u8, entry_index: u8) {
        let b = bank as usize;
        let e = self.sfx.banks[b][entry_index as usize];
        if e.sfx_params & SFX_FLAG_3 != 0 {
            self.clear_bgm_mute(e.channel_idx);
        }
        if entry_index == self.sfx.list_end[b] {
            self.sfx.list_end[b] = e.prev;
        } else {
            self.sfx.banks[b][e.next as usize].prev = e.prev;
        }
        // @bug (game): an entry at the head's index 0 (prev 0xFF) would write past the bank;
        // entries in the list always have a prev.
        if let Some(p) = self.sfx.banks[b].get_mut(e.prev as usize) {
            p.next = e.next;
        }
        let free = self.sfx.free_list_start[b];
        let ent = &mut self.sfx.banks[b][entry_index as usize];
        ent.next = free;
        ent.prev = 0xFF;
        self.sfx.banks[b][free as usize].prev = entry_index;
        self.sfx.free_list_start[b] = entry_index;
        self.sfx.banks[b][entry_index as usize].state = SFX_STATE_EMPTY;

        let n = self.channels_per_bank(bank);
        for i in 0..n as usize {
            if self.sfx.active[b][i].entry_index == entry_index {
                self.sfx.active[b][i].entry_index = 0xFF;
                break;
            }
        }
    }

    /// `Audio_ChooseActiveSfx`: the bank's entries by priority (distance and importance), the
    /// most pressing on the bank's channels.
    fn choose_active_sfx(&mut self, bank: u8) {
        let b = bank as usize;
        let mut num_chosen: u8 = 0;
        let mut chosen = [ActiveSfx { priority: 0x7FFF_FFFF, entry_index: 0xFF }; MAX_CHANNELS_PER_BANK];
        let mut entry_index = self.sfx.banks[b][0].next;
        let mut k: u8 = 0;
        while entry_index != 0xFF {
            let ei = entry_index as usize;
            {
                let e = self.sfx.banks[b][ei];
                if e.state == SFX_STATE_QUEUED && e.sfx_id & 0xC00 != 0 {
                    self.sfx.banks[b][ei].freshness = e.freshness.wrapping_sub(1);
                } else if e.sfx_id & 0xC00 == 0 && e.state == SFX_STATE_PLAYING_2 {
                    self.queue_cmd_s8(((e.channel_idx as u32) << 8) | 0x0602_0000, 0);
                    self.remove_sfx_bank_entry(bank, entry_index);
                }
            }
            if self.sfx.banks[b][ei].freshness == 0 {
                self.remove_sfx_bank_entry(bank, entry_index);
            } else if self.sfx.banks[b][ei].state != SFX_STATE_EMPTY {
                let e = &mut self.sfx.banks[b][ei];
                if e.pos == Some(SfxPos::Default) {
                    e.dist = 0.0;
                } else {
                    let p = e.pos_now;
                    let tempf1 = p.y * 1.0;
                    e.dist = (p.x * p.x + tempf1 * tempf1 + p.z * p.z) * 1.0;
                }
                let imp = e.sfx_importance as u32;
                let base = (0xFF - imp) * (0xFF - imp) * (76 * 76);
                if e.sfx_params & SFX_FLAG_4 != 0 {
                    e.priority = base;
                } else {
                    if e.dist > 0x7FFF_FFD0 as f32 {
                        e.dist = 0x7000_0008 as f32;
                        log::debug!("<INAGAKI CHECK> dist over! flag:{:04X}", e.sfx_id);
                    }
                    e.priority = (e.dist as u32).wrapping_add(base);
                    if e.pos_now.z < 0.0 {
                        e.priority = e.priority.wrapping_add((-e.pos_now.z * 6.0) as i32 as u32);
                    }
                }
                let e = self.sfx.banks[b][ei];
                if e.dist > 1e5f32 * 1e5f32 {
                    if e.state == SFX_STATE_PLAYING_1 {
                        self.queue_cmd_s8(((e.channel_idx as u32) << 8) | 0x0602_0000, 0);
                        if e.sfx_id & 0xC00 != 0 {
                            self.remove_sfx_bank_entry(bank, entry_index);
                            entry_index = k;
                        }
                    }
                } else {
                    let num_channels = self.channels_per_bank(bank) as usize;
                    for i in 0..num_channels.min(MAX_CHANNELS_PER_BANK) {
                        if chosen[i].priority >= e.priority {
                            if (num_chosen as usize) < num_channels {
                                num_chosen += 1;
                            }
                            let mut j = num_channels - 1;
                            while j > i {
                                chosen[j] = chosen[j - 1];
                                j -= 1;
                            }
                            chosen[i] = ActiveSfx { priority: e.priority, entry_index };
                            break;
                        }
                    }
                }
                k = entry_index;
            }
            entry_index = self.sfx.banks[b][k as usize].next;
        }
        for c in chosen.iter().take(num_chosen as usize) {
            let e = &mut self.sfx.banks[b][c.entry_index as usize];
            if e.state == SFX_STATE_QUEUED {
                e.state = SFX_STATE_READY;
            } else if e.state == SFX_STATE_PLAYING_1 {
                e.state = SFX_STATE_PLAYING_REFRESH;
            }
        }

        // Pick something to play for all channels.
        let num_channels = (self.channels_per_bank(bank) as usize).min(MAX_CHANNELS_PER_BANK);
        for i in 0..num_channels {
            let mut need_new = false;
            let active = self.sfx.active[b][i];
            if active.entry_index == 0xFF {
                need_new = true;
            } else {
                let e = self.sfx.banks[b][active.entry_index as usize];
                if e.state == SFX_STATE_PLAYING_1 {
                    if e.sfx_id & 0xC00 != 0 {
                        self.remove_sfx_bank_entry(bank, active.entry_index);
                    } else {
                        self.sfx.banks[b][active.entry_index as usize].state = SFX_STATE_QUEUED;
                    }
                    need_new = true;
                } else if e.state == SFX_STATE_EMPTY {
                    self.sfx.active[b][i].entry_index = 0xFF;
                    need_new = true;
                } else {
                    // Sfx is already playing as it should, nothing to do.
                    for c in chosen.iter_mut().take(num_channels) {
                        if active.entry_index == c.entry_index {
                            c.entry_index = 0xFF;
                            break;
                        }
                    }
                    num_chosen = num_chosen.wrapping_sub(1);
                }
            }
            if need_new {
                let mut j = 0usize;
                while j < num_channels {
                    let chosen_entry = chosen[j].entry_index;
                    if chosen_entry != 0xFF && self.sfx.banks[b][chosen_entry as usize].state != SFX_STATE_PLAYING_REFRESH {
                        for kk in 0..num_channels {
                            if chosen_entry == self.sfx.active[b][kk].entry_index {
                                need_new = false;
                                break;
                            }
                        }
                        if need_new {
                            self.sfx.active[b][i].entry_index = chosen_entry;
                            chosen[j].entry_index = 0xFF;
                            j = num_channels + 1;
                            num_chosen = num_chosen.wrapping_sub(1);
                        }
                    }
                    j += 1;
                }
                if j == num_channels {
                    // nothing found
                    self.sfx.active[b][i].entry_index = 0xFF;
                }
            }
        }
    }

    /// `Audio_PlayActiveSfx`: starts the bank's newly chosen on their channels (the sound
    /// effects player's IO ports: 0 to start, 4 and 5 the sound), refreshes the others'
    /// properties, and lets go of those the channel says are done (port 1 at -1).
    fn play_active_sfx(&mut self, bank: u8) {
        let b = bank as usize;
        let n = self.channels_per_bank(bank) as usize;
        for i in 0..n.min(MAX_CHANNELS_PER_BANK) {
            let entry_index = self.sfx.active[b][i].entry_index;
            if entry_index != 0xFF {
                let ei = entry_index as usize;
                let cur = self.sfx.cur_channel_idx;
                let channel = self.view.players[SEQ_PLAYER_SFX as usize].channels[cur as usize & 0xF];
                let e = self.sfx.banks[b][ei];
                if e.state == SFX_STATE_READY {
                    self.sfx.banks[b][ei].channel_idx = cur;
                    if e.sfx_params & SFX_FLAG_3 != 0 {
                        self.queue_seq_cmd_mute(cur);
                    }
                    if e.sfx_params & SFX_PARAM_67_MASK != 0 {
                        let r = match e.sfx_params & SFX_PARAM_67_MASK {
                            p if p == 1 << SFX_PARAM_67_SHIFT => (self.next_random() & 0xF) as u8,
                            p if p == 2 << SFX_PARAM_67_SHIFT => (self.next_random() & 0x1F) as u8,
                            p if p == 3 << SFX_PARAM_67_SHIFT => (self.next_random() & 0x3F) as u8,
                            _ => 0,
                        };
                        self.sfx.banks[b][ei].unk_2f = r;
                    }
                    self.set_sfx_properties(bank, entry_index, cur);
                    let c = (cur as u32 & 0xFF) << 8;
                    self.queue_cmd_s8((0x6 << 24) | ((SEQ_PLAYER_SFX as u32) << 16) | c, 1);
                    self.queue_cmd_s8((0x6 << 24) | ((SEQ_PLAYER_SFX as u32) << 16) | c | 4, (e.sfx_id & 0xFF) as u8 as i8);
                    if self.tables.is_large_sfx_bank.get(b).copied().unwrap_or(0) != 0 {
                        self.queue_cmd_s8((0x6 << 24) | ((SEQ_PLAYER_SFX as u32) << 16) | c | 5, ((e.sfx_id & 0x100) >> 8) as i8);
                    }
                    self.sfx.banks[b][ei].state = if e.sfx_id & 0xC00 != 0 { SFX_STATE_PLAYING_1 } else { SFX_STATE_PLAYING_2 };
                } else if channel.sound_script_io[1] as u8 == 0xFF {
                    self.remove_sfx_bank_entry(bank, entry_index);
                } else if e.state == SFX_STATE_PLAYING_REFRESH {
                    self.set_sfx_properties(bank, entry_index, cur);
                    self.sfx.banks[b][ei].state = if e.sfx_id & 0xC00 != 0 { SFX_STATE_PLAYING_1 } else { SFX_STATE_PLAYING_2 };
                }
            }
            self.sfx.cur_channel_idx = self.sfx.cur_channel_idx.wrapping_add(1);
        }
    }

    /// Stops and frees an entry (what the stop functions do for each match).
    fn stop_entry(&mut self, bank: u8, entry_index: u8) {
        let e = self.sfx.banks[bank as usize][entry_index as usize];
        if e.state >= SFX_STATE_PLAYING_REFRESH {
            self.queue_cmd_s8((0x6 << 24) | ((SEQ_PLAYER_SFX as u32) << 16) | ((e.channel_idx as u32 & 0xFF) << 8), 0);
        }
        if e.state != SFX_STATE_EMPTY {
            self.remove_sfx_bank_entry(bank, entry_index);
        }
    }

    /// `Audio_StopSfxByBank`.
    pub fn stop_sfx_by_bank(&mut self, bank: u8) {
        let b = bank as usize;
        if b >= self.bank_count() {
            return;
        }
        let mut entry_index = self.sfx.banks[b][0].next;
        while entry_index != 0xFF {
            self.stop_entry(bank, entry_index);
            entry_index = self.sfx.banks[b][0].next;
        }
        self.remove_matching_sfx_requests(0, Cmp { sfx_id: (bank as u16) << 12, ..Default::default() });
    }

    /// `func_800F8884`: stops the bank's entries at `pos`.
    pub fn func_800f8884(&mut self, bank: u8, pos: SfxPos) {
        let b = bank as usize;
        if b >= self.bank_count() {
            return;
        }
        let mut entry_index = self.sfx.banks[b][0].next;
        let mut prev_entry_index = 0u8;
        while entry_index != 0xFF {
            if self.sfx.banks[b][entry_index as usize].pos == Some(pos) {
                self.stop_entry(bank, entry_index);
            } else {
                prev_entry_index = entry_index;
            }
            entry_index = self.sfx.banks[b][prev_entry_index as usize].next;
        }
    }

    /// `Audio_StopSfxByPosAndBank`.
    pub fn stop_sfx_by_pos_and_bank(&mut self, bank: u8, pos: SfxPos) {
        self.func_800f8884(bank, pos);
        self.remove_matching_sfx_requests(1, Cmp { sfx_id: (bank as u16) << 12, pos: Some(pos), token: 0 });
    }

    /// `Audio_StopSfxByPos`: every sound at `pos` (an actor going).
    pub fn stop_sfx_by_pos(&mut self, pos: SfxPos) {
        for b in 0..self.bank_count() as u8 {
            self.func_800f8884(b, pos);
        }
        self.remove_matching_sfx_requests(2, Cmp { pos: Some(pos), ..Default::default() });
    }

    /// `Audio_StopSfxByPosAndId`.
    pub fn stop_sfx_by_pos_and_id(&mut self, pos: SfxPos, sfx_id: u16) {
        let bank = sfx_bank(sfx_id);
        let b = bank as usize;
        if b < self.bank_count() {
            let mut entry_index = self.sfx.banks[b][0].next;
            let mut prev_entry_index = 0u8;
            while entry_index != 0xFF {
                let e = self.sfx.banks[b][entry_index as usize];
                if e.pos == Some(pos) && e.sfx_id == sfx_id {
                    self.stop_entry(bank, entry_index);
                    entry_index = 0xFF;
                } else {
                    prev_entry_index = entry_index;
                }
                if entry_index != 0xFF {
                    entry_index = self.sfx.banks[b][prev_entry_index as usize].next;
                }
            }
        }
        self.remove_matching_sfx_requests(3, Cmp { sfx_id, pos: Some(pos), token: 0 });
    }

    /// `Audio_StopSfxByTokenAndId`.
    pub fn stop_sfx_by_token_and_id(&mut self, token: u8, sfx_id: u16) {
        let bank = sfx_bank(sfx_id);
        let b = bank as usize;
        if b < self.bank_count() {
            let mut entry_index = self.sfx.banks[b][0].next;
            let mut prev_entry_index = 0u8;
            while entry_index != 0xFF {
                let e = self.sfx.banks[b][entry_index as usize];
                if e.token == token && e.sfx_id == sfx_id {
                    self.stop_entry(bank, entry_index);
                } else {
                    prev_entry_index = entry_index;
                }
                if entry_index != 0xFF {
                    entry_index = self.sfx.banks[b][prev_entry_index as usize].next;
                }
            }
        }
        self.remove_matching_sfx_requests(4, Cmp { sfx_id, pos: None, token });
    }

    /// `Audio_StopSfxById`.
    pub fn stop_sfx_by_id(&mut self, sfx_id: u16) {
        let bank = sfx_bank(sfx_id);
        let b = bank as usize;
        if b < self.bank_count() {
            let mut entry_index = self.sfx.banks[b][0].next;
            let mut prev_entry_index = 0u8;
            while entry_index != 0xFF {
                if self.sfx.banks[b][entry_index as usize].sfx_id == sfx_id {
                    self.stop_entry(bank, entry_index);
                } else {
                    prev_entry_index = entry_index;
                }
                entry_index = self.sfx.banks[b][prev_entry_index as usize].next;
            }
        }
        self.remove_matching_sfx_requests(5, Cmp { sfx_id, ..Default::default() });
    }

    /// `Audio_ProcessSfxRequests`.
    pub fn process_sfx_requests(&mut self) {
        while self.sfx.request_write_index != self.sfx.request_read_index {
            self.process_sfx_request();
            self.sfx.request_read_index = self.sfx.request_read_index.wrapping_add(1);
        }
    }

    /// `Audio_SetUnusedBankLerp`.
    pub fn set_unused_bank_lerp(&mut self, bank: u8, target: u8, mut delay: u16) {
        if delay == 0 {
            delay += 1;
        }
        let l = &mut self.sfx.unused_bank_lerp[bank as usize];
        l.target = target as f32 / 127.0;
        l.remaining_frames = delay;
        l.step = (l.value - l.target) / delay as f32;
    }

    /// `Audio_StepUnusedBankLerp`.
    fn step_unused_bank_lerp(&mut self, bank: u8) {
        let l = &mut self.sfx.unused_bank_lerp[bank as usize];
        if l.remaining_frames != 0 {
            l.remaining_frames -= 1;
            if l.remaining_frames != 0 {
                l.value -= l.step;
            } else {
                l.value = l.target;
            }
        }
    }

    /// `func_800F8F88`: each bank's choice and play, once the sound effects' sequence has its
    /// channels.
    pub fn func_800f8f88(&mut self) {
        if self.view.players[SEQ_PLAYER_SFX as usize].channels[0].valid {
            self.sfx.cur_channel_idx = 0;
            for bank in 0..self.bank_count() as u8 {
                self.choose_active_sfx(bank);
                self.play_active_sfx(bank);
                self.step_unused_bank_lerp(bank);
            }
        }
    }

    /// `Audio_IsSfxPlaying`: whether any entry of the id's bank holds it.
    pub fn is_sfx_playing(&self, sfx_id: u16) -> bool {
        let b = sfx_bank(sfx_id) as usize;
        let Some(bank) = self.sfx.banks.get(b) else { return false };
        let mut entry_index = bank[0].next;
        while entry_index != 0xFF {
            let e = &bank[entry_index as usize];
            if e.sfx_id == sfx_id {
                return true;
            }
            entry_index = e.next;
        }
        false
    }

    /// `Audio_ResetSfx`: the requests, the banks' lists (each a used list from entry 0 and a
    /// free list from entry 1), the active channels.
    pub fn reset_sfx(&mut self) {
        let sizes = self.tables.sfx_bank_sizes.clone();
        if self.sfx.banks.len() != sizes.len() {
            self.sfx.banks = sizes.iter().map(|&n| vec![SfxBankEntry::default(); n as usize]).collect();
        }
        let s = &mut self.sfx;
        s.request_write_index = 0;
        s.request_read_index = 0;
        s.d_801333d0 = 0;
        for b in 0..NUM_SFX_BANKS {
            s.list_end[b] = 0;
            s.free_list_start[b] = 1;
            s.unused[b] = 0;
            self.sfx_bank_muted[b] = 0;
            s.unused_bank_lerp[b].value = 1.0;
            s.unused_bank_lerp[b].remaining_frames = 0;
        }
        for b in 0..NUM_SFX_BANKS {
            for a in s.active[b].iter_mut() {
                a.entry_index = 0xFF;
            }
        }
        for bank in s.banks.iter_mut() {
            let n = bank.len();
            if n < 2 {
                continue;
            }
            bank[0].prev = 0xFF;
            bank[0].next = 0xFF;
            let mut i = 1;
            while i < n - 1 {
                bank[i].prev = (i - 1) as u8;
                bank[i].next = (i + 1) as u8;
                i += 1;
            }
            bank[i].prev = (i - 1) as u8;
            bank[i].next = 0xFF;
        }
        if s.d_801333f8 == 0 {
            s.swap_source = [0; 10];
            s.swap_target = [0; 10];
            s.swap_mode = [0; 10];
            s.d_801333f8 += 1;
        }
    }

    // ---------------------------------------------------------------------------------------
    // code_800EC960.c: a channel's properties.

    /// `Audio_ComputeSfxVolume`: by distance, slower with `SFX_PARAM_01` (none with
    /// `SFX_FLAG_13`).
    fn compute_sfx_volume(&self, bank: u8, entry_idx: u8) -> f32 {
        let e = &self.sfx.banks[bank as usize][entry_idx as usize];
        if e.sfx_params & SFX_FLAG_13 != 0 {
            return 1.0;
        }
        if e.dist > 10000.0 {
            return 0.0;
        }
        let base_dist = match (e.sfx_params & SFX_PARAM_01_MASK) >> SFX_PARAM_01_SHIFT {
            1 => 10000.0f32 / 15.0,
            2 => 10000.0f32 / 10.5,
            3 => 10000.0f32 / 2.6,
            _ => 10000.0f32 / 20.0,
        };
        let min_dist = base_dist / 5.0;
        // Volume grows as inverse square of distance. Linearly approximate the inverse part,
        // then square.
        let ret = if e.dist < min_dist {
            1.0
        } else if e.dist < base_dist {
            ((((base_dist - min_dist) - (e.dist - min_dist)) / (base_dist - min_dist)) * 0.19) + 0.81
        } else {
            (1.0 - ((e.dist - base_dist) / (10000.0 - base_dist))) * 0.81
        };
        ret * ret
    }

    /// `Audio_ComputeSfxReverb`.
    fn compute_sfx_reverb(&self, bank: u8, entry_idx: u8, channel_idx: u8) -> i8 {
        let e = &self.sfx.banks[bank as usize][entry_idx as usize];
        let mut dist_add: i8 = 0;
        let mut script_add: i32 = 0;
        if e.sfx_params & SFX_FLAG_12 == 0 {
            dist_add = if e.dist < 2500.0 { (if e.pos_now.z > 0.0 { (e.dist / 2500.0) * 70.0 } else { (e.dist / 2500.0) * 91.0 }) as i32 as i8 } else { 70 };
        }
        let ch = self.view.players[SEQ_PLAYER_SFX as usize].channels[channel_idx as usize & 0xF];
        if ch.valid {
            script_add = ch.sound_script_io[1] as i32;
            if ch.sound_script_io[1] < 0 {
                script_add = 0;
            }
        }
        let mut reverb = self.sfx_s8(e.reverb_add) as i32 + dist_add as i32 + script_add;
        if bank != BANK_OCARINA || !((e.sfx_id & 0x1FF) < 2) {
            reverb += self.audio_env_reverb as i32 + self.audio_code_reverb as i32 + self.spec_reverb as i32;
        }
        if reverb > 0x7F {
            reverb = 0x7F;
        }
        reverb as i8
    }

    /// `Audio_ComputeSfxFreqScale`.
    fn compute_sfx_freq_scale(&self, bank: u8, entry_idx: u8) -> f32 {
        let e = &self.sfx.banks[bank as usize][entry_idx as usize];
        let mut phi_v0 = 0;
        let mut freq = 1.0f32;
        let rnd = self.view.audio_random;
        if e.sfx_params & SFX_FLAG_14 != 0 {
            freq = 1.0 - ((rnd & 0xF) as f32 / 192.0);
        }
        match bank {
            BANK_PLAYER | BANK_ITEM | BANK_VOICE => {
                if self.audio_base_filter2 != 0 {
                    phi_v0 = 1;
                }
            }
            BANK_ENV | BANK_ENEMY => {
                if self.audio_extra_filter2 != 0 {
                    phi_v0 = 1;
                }
            }
            _ => {}
        }
        if phi_v0 == 1 && e.sfx_params & SFX_FLAG_11 == 0 {
            freq = (freq as f64 * (1.0293 - ((rnd & 0xF) as f32 / 144.0) as f64)) as f32;
        }
        let unk1c = e.dist;
        if e.sfx_params & SFX_FLAG_13 == 0 && e.sfx_params & SFX_FLAG_15 == 0 {
            if unk1c >= 10000.0 {
                freq += 0.2;
            } else {
                freq += 0.2 * (unk1c / 10000.0);
            }
        }
        if e.sfx_params & SFX_PARAM_67_MASK != 0 {
            freq += e.unk_2f as f32 / 192.0;
        }
        freq
    }

    /// `func_800F37B8`: the surround mode's filter, behind the screen and by distance.
    fn func_800f37b8(&self, behind_screen_z: f32, e: &SfxBankEntry, arg2: i8) -> u8 {
        let mut phi_v1: u8 = if e.pos_now.z < behind_screen_z {
            let phi_v0: i8 = if arg2 < 65 { arg2 } else { (0x7F - arg2 as i32) as i8 };
            if phi_v0 < 30 {
                0
            } else {
                let v = ((((phi_v0 as i32 & 0xFFFF) * 10) - 300) / 34) as u8;
                if v != 0 { 0x10u8.wrapping_sub(v) } else { v }
            }
        } else {
            0
        };
        if phi_v1 == 0 && e.sfx_params & SFX_FLAG_9 != 0 {
            phi_v1 = 0xF;
        }
        let phi_f0 = match (e.sfx_params & SFX_PARAM_01_MASK) >> SFX_PARAM_01_SHIFT {
            1 => 12.0f32,
            2 => 9.0,
            3 => 6.0,
            _ => 15.0,
        };
        let phi_f12 = e.dist.min(10000.0 / 5.2);
        phi_v1.wrapping_mul(0x10).wrapping_add(((phi_f0 * phi_f12) / (10000.0 / 5.2)) as u8)
    }

    /// `Audio_SetSfxProperties`: the channel's volume (port 2), reverb, frequency, stereo bits,
    /// filter (port 3) and pan, each sent only when it changes (`sSfxChannelState`).
    fn set_sfx_properties(&mut self, bank: u8, entry_idx: u8, channel_idx: u8) {
        let mut vol = 1.0f32;
        let mut reverb: i8 = 0;
        let mut freq_scale = 1.0f32;
        let mut pan_signed: i8 = 0x40;
        let mut stereo_bits: u8 = 0;
        let mut filter: u8 = 0;
        let mut sp38: i8 = 0;
        let mut base_filter: u8 = 0;
        let b = bank as usize;
        let ci = channel_idx as usize & 0xF;
        match bank {
            BANK_PLAYER | BANK_ITEM | BANK_ENV | BANK_ENEMY | BANK_VOICE | BANK_OCARINA => {
                if bank != BANK_OCARINA && self.sound_mode == SOUNDMODE_SURROUND {
                    let e = &self.sfx.banks[b][entry_idx as usize];
                    sp38 = func_800f3990(e.pos_now.y, e.sfx_params);
                }
                let e = &mut self.sfx.banks[b][entry_idx as usize];
                e.dist = e.dist.sqrt();
                let e = self.sfx.banks[b][entry_idx as usize];
                vol = self.compute_sfx_volume(bank, entry_idx) * self.sfx_f32(e.vol);
                reverb = self.compute_sfx_reverb(bank, entry_idx, channel_idx);
                pan_signed = compute_sfx_pan_signed(e.pos_now.x, e.pos_now.z, e.token);
                freq_scale = self.compute_sfx_freq_scale(bank, entry_idx) * self.sfx_f32(e.freq_scale);
                let mut behind_screen_z = 0.0;
                if self.sound_mode == SOUNDMODE_SURROUND {
                    behind_screen_z = self.tables.behind_screen_z.get(((e.sfx_params & SFX_FLAG_10) >> SFX_FLAG_10_SHIFT) as usize).copied().unwrap_or(0.0);
                    if e.sfx_params & SFX_FLAG_11 == 0 {
                        if e.pos_now.z < behind_screen_z {
                            stereo_bits = 0x10;
                        }
                        let st = self.sfx_channel_state[ci].stereo_bits as u8;
                        stereo_bits = if (st ^ stereo_bits) & 0x10 != 0 { if pan_signed < 0x40 { st ^ 0x14 } else { st ^ 0x18 } } else { st };
                    }
                }
                if self.audio_base_filter != 0 && (bank == BANK_ITEM || bank == BANK_PLAYER || bank == BANK_VOICE) {
                    base_filter = self.audio_base_filter;
                }
                if (base_filter | self.audio_extra_filter) != 0 {
                    filter = base_filter | self.audio_extra_filter;
                } else if self.sound_mode == SOUNDMODE_SURROUND && e.sfx_params & SFX_FLAG_13 == 0 {
                    filter = self.func_800f37b8(behind_screen_z, &e, pan_signed);
                }
            }
            _ => {}
        }

        let st = self.sfx_channel_state[ci];
        let vol_s8: i8 = if st.vol != vol {
            self.sfx_channel_state[ci].vol = vol;
            (vol * 127.0) as u8 as i8
        } else {
            -1
        };
        let c = ((SEQ_PLAYER_SFX as u32) << 16) | ((channel_idx as u32) << 8);
        // CHAN_UPD_SCRIPT_IO (slot 2, sets volume)
        self.queue_cmd_s8((0x6 << 24) | c | 2, vol_s8);
        if reverb != st.reverb {
            self.queue_cmd_s8((0x5 << 24) | c, reverb);
            self.sfx_channel_state[ci].reverb = reverb;
        }
        if freq_scale != st.freq_scale {
            self.queue_cmd_f32((0x4 << 24) | c, freq_scale);
            self.sfx_channel_state[ci].freq_scale = freq_scale;
        }
        if stereo_bits as i8 != st.stereo_bits {
            self.queue_cmd_s8((0xE << 24) | c, (stereo_bits | 0x10) as i8);
            self.sfx_channel_state[ci].stereo_bits = stereo_bits as i8;
        }
        if filter != st.filter {
            // CHAN_UPD_SCRIPT_IO (slot 3, sets filter)
            self.queue_cmd_s8((0x6 << 24) | c | 3, filter as i8);
            self.sfx_channel_state[ci].filter = filter;
        }
        if sp38 as u8 != st.unk_0c {
            // CHAN_UPD_UNK_0F
            self.queue_cmd_s8((0xC << 24) | c, 0x10);
            // CHAN_UPD_UNK_20
            self.queue_cmd_u16((0xD << 24) | c, ((sp38 as i16 as u16) << 8).wrapping_add(0xFF));
            self.sfx_channel_state[ci].unk_0c = sp38 as u8;
        }
        if pan_signed != st.pan_signed {
            self.queue_cmd_s8((0x3 << 24) | c, pan_signed);
            self.sfx_channel_state[ci].pan_signed = pan_signed;
        }
    }

    // ---------------------------------------------------------------------------------------
    // code_800EC960.c: playing with a scale of one's own.

    /// `func_800F3F84`: the footsteps' volume and frequency by speed.
    pub fn func_800f3f84(&mut self, arg0: f32) -> f32 {
        let mut ret = 1.0;
        if arg0 > 6.0 {
            self.d_8016b7a8 = 1.0;
            self.d_8016b7b0 = 1.1;
        } else {
            ret = arg0 / 6.0;
            self.d_8016b7a8 = (ret * 0.225_000_02) + 0.775;
            self.d_8016b7b0 = (ret * 0.2) + 0.9;
        }
        ret
    }

    /// `func_800F4010`: a footstep at `pos` for speed `arg2`, with the metal effect on top.
    pub fn func_800f4010(&mut self, pos: SfxPos, sfx_id: u16, arg2: f32) {
        self.sfx.d_80131c8c = arg2;
        let mut sp24 = self.func_800f3f84(arg2);
        self.play_sfx_general(sfx_id, pos, 4, SfxF32::D8016B7B0, SfxF32::D8016B7A8, SfxS8::Zero);
        let (phi_f0, phi_v0) = if (sfx_id & 0xF0) == 0xB0 {
            sp24 = 1.0;
            (0.3f32, 1u8)
        } else {
            (1.1f32, (self.view.audio_random % 2) as u8)
        };
        if phi_f0 < arg2 && phi_v0 != 0 {
            let sfx_id2 = if sfx_id & 0x80 != 0 { NA_SE_PL_METALEFFECT_ADULT } else { NA_SE_PL_METALEFFECT_KID };
            self.sfx.d_8016b7ac = ((sp24 as f64 * 0.7) + 0.3) as f32;
            self.play_sfx_general(sfx_id2, pos, 4, SfxF32::D8016B7B0, SfxF32::D8016B7AC, SfxS8::Zero);
        }
    }

    /// `func_800F4138`.
    pub fn func_800f4138(&mut self, pos: SfxPos, sfx_id: u16, arg2: f32) {
        self.func_800f3f84(arg2);
        self.play_sfx_general(sfx_id, pos, 4, SfxF32::D8016B7B0, SfxF32::D8016B7A8, SfxS8::Zero);
    }

    /// `func_800F4190`.
    pub fn func_800f4190(&mut self, pos: SfxPos, sfx_id: u16) {
        self.play_sfx_general(sfx_id, pos, 4, SfxF32::D801305B0, SfxF32::One, SfxS8::D801305B4);
    }

    /// `Audio_PlaySfxRandom`.
    pub fn play_sfx_random(&mut self, pos: SfxPos, base_sfx_id: u16, rand_lim: u8) {
        let offset = (self.next_random() % rand_lim.max(1) as u32) as u8;
        self.play_sfx_general(base_sfx_id.wrapping_add(offset as u16), pos, 4, SfxF32::One, SfxF32::One, SfxS8::Zero);
    }

    /// `func_800F4254`: the sword's charge, by level.
    pub fn func_800f4254(&mut self, pos: SfxPos, level: u8) {
        let level = level & 3;
        if level != self.sfx.prev_charge_level {
            self.sfx.d_801305f4 = self.tables.charge_freq_scales.get(level as usize).copied().unwrap_or(1.0);
            if level == 1 || level == 2 {
                self.play_sfx_general(NA_SE_PL_SWORD_CHARGE, pos, 4, SfxF32::D801305F4, SfxF32::One, SfxS8::Zero);
            }
            self.sfx.prev_charge_level = level;
        }
        if level != 0 {
            self.play_sfx_general(NA_SE_IT_SWORD_CHARGE - SFX_FLAG, pos, 4, SfxF32::D801305F4, SfxF32::One, SfxS8::Zero);
        }
    }

    /// `func_800F436C`.
    pub fn func_800f436c(&mut self, pos: SfxPos, sfx_id: u16, arg2: f32) {
        self.d_8016b7d8 = if arg2 < 0.75 { ((arg2 / 0.75) * 0.25) + 0.5 } else { arg2 };
        if self.d_8016b7d8 > 0.5 {
            self.play_sfx_general(sfx_id, pos, 4, SfxF32::D8016B7D8, SfxF32::One, SfxS8::Zero);
        }
    }

    /// `func_800F4414`.
    pub fn func_800f4414(&mut self, pos: SfxPos, sfx_id: u16, mut arg2: f32) {
        self.sfx.d_801305b8 = self.sfx.d_801305b8.wrapping_sub(1);
        if self.sfx.d_801305b8 == 0 {
            self.play_sfx_general(sfx_id, pos, 4, SfxF32::D8016B7D8, SfxF32::One, SfxS8::Zero);
            if arg2 > 2.0 {
                arg2 = 2.0;
            }
            let s = &mut self.sfx;
            s.d_801305b8 = (((s.d_801305c0 as i32 - s.d_801305bc as i32) as f32 * (1.0 - arg2)) as i32 as i8).wrapping_add(s.d_801305c0);
        }
    }

    /// `func_800F44EC`.
    pub fn func_800f44ec(&mut self, arg0: i8, arg1: i8) {
        self.sfx.d_801305b8 = 1;
        self.sfx.d_801305bc = arg1;
        self.sfx.d_801305c0 = arg0;
    }

    /// `func_800F4524`.
    pub fn func_800f4524(&mut self, pos: SfxPos, sfx_id: u16, arg2: i8) {
        self.sfx.d_8016b7dc = arg2;
        self.play_sfx_general(sfx_id, pos, 4, SfxF32::One, SfxF32::One, SfxS8::D8016B7DC);
    }

    /// `func_800F4578`.
    pub fn func_800f4578(&mut self, pos: SfxPos, sfx_id: u16, arg2: f32) {
        self.sfx.d_8016b7e0 = arg2;
        self.play_sfx_general(sfx_id, pos, 4, SfxF32::One, SfxF32::D8016B7E0, SfxS8::Zero);
    }

    /// `Audio_PlaySfxRiver`.
    pub fn play_sfx_river(&mut self, pos: SfxPos, freq_scale: f32) {
        if !self.is_sfx_playing(NA_SE_EV_RIVER_STREAM - SFX_FLAG) {
            self.river_freq_scale_lerp.value = freq_scale;
        } else if freq_scale != self.river_freq_scale_lerp.value {
            let l = &mut self.river_freq_scale_lerp;
            l.target = freq_scale;
            l.remaining_frames = 40;
            l.step = (l.target - l.value) / 40.0;
        }
        self.play_sfx_general(NA_SE_EV_RIVER_STREAM - SFX_FLAG, pos, 4, SfxF32::RiverFreq, SfxF32::One, SfxS8::Zero);
    }

    /// `Audio_PlaySfxWaterfall`.
    pub fn play_sfx_waterfall(&mut self, pos: SfxPos, freq_scale: f32) {
        if !self.is_sfx_playing(NA_SE_EV_WATER_WALL_BIG - SFX_FLAG) {
            self.waterfall_freq_scale_lerp.value = freq_scale;
        } else if freq_scale != self.waterfall_freq_scale_lerp.value {
            let l = &mut self.waterfall_freq_scale_lerp;
            l.target = freq_scale;
            l.remaining_frames = 40;
            l.step = (l.target - l.value) / 40.0;
        }
        self.play_sfx_general(NA_SE_EV_WATER_WALL_BIG - SFX_FLAG, pos, 4, SfxF32::WaterfallFreq, SfxF32::WaterfallFreq, SfxS8::Zero);
    }

    /// `Audio_PlaySfxIncreasinglyTransposed`.
    pub fn play_sfx_increasingly_transposed(&mut self, pos: SfxPos, sfx_id: i16, semitones: &[u8]) {
        let st = semitones.get(self.sfx.increasing_transpose as usize).copied().unwrap_or(0);
        self.play_sfx_general(sfx_id as u16, pos, 4, SfxF32::Pitch(st.wrapping_add(39)), SfxF32::One, SfxS8::Zero);
        if self.sfx.increasing_transpose < 15 {
            self.sfx.increasing_transpose += 1;
        }
    }

    /// `Audio_ResetIncreasingTranspose`.
    pub fn reset_increasing_transpose(&mut self) {
        self.sfx.increasing_transpose = 0;
    }

    /// `Audio_PlaySfxTransposed`.
    pub fn play_sfx_transposed(&mut self, pos: SfxPos, sfx_id: u16, semitone: i8) {
        self.play_sfx_general(sfx_id, pos, 4, SfxF32::Pitch((semitone as i32 + 39) as u8), SfxF32::One, SfxS8::Zero);
    }

    /// `func_800F4C58`: the sound's channels' port 6 to `arg2`, then the sound.
    pub fn func_800f4c58(&mut self, pos: SfxPos, sfx_id: u16, arg2: u8) {
        let bank = sfx_bank_shift(sfx_id);
        let mut phi_s1: u8 = 0;
        for i in 0..bank {
            phi_s1 = phi_s1.wrapping_add(self.channels_per_bank(i));
        }
        let b = bank as usize;
        for i in 0..self.channels_per_bank(bank) as usize {
            let ei = self.sfx.active.get(b).map(|a| a[i.min(MAX_CHANNELS_PER_BANK - 1)].entry_index).unwrap_or(0xFF);
            if ei != 0xFF && self.sfx.banks.get(b).is_some_and(|bk| bk[ei as usize].sfx_id == sfx_id) {
                self.queue_cmd_s8((0x6 << 24) | ((SEQ_PLAYER_SFX as u32) << 16) | ((phi_s1 as u32) << 8) | 6, arg2 as i8);
            }
            phi_s1 = phi_s1.wrapping_add(1);
        }
        self.play_sfx_general(sfx_id, pos, 4, SfxF32::One, SfxF32::One, SfxS8::Zero);
    }

    /// `Audio_PlaySfxGeneralIfNotInCutscene`.
    pub fn play_sfx_general_if_not_in_cutscene(&mut self, sfx_id: u16, pos: SfxPos, arg2: u8, freq_scale: SfxF32, vol: SfxF32, reverb_add: SfxS8) {
        if self.audio_cutscene_flag == 0 {
            self.play_sfx_general(sfx_id, pos, arg2, freq_scale, vol, reverb_add);
        }
    }

    /// `Audio_PlaySfxIfNotInCutscene`.
    pub fn play_sfx_if_not_in_cutscene(&mut self, sfx_id: u16) {
        self.play_sfx_general_if_not_in_cutscene(sfx_id, SfxPos::Default, 4, SfxF32::One, SfxF32::One, SfxS8::Zero);
    }

    /// `AudioMgr_StopAllSfx` (`code_800C3C20.c`): every bank, in `sSfxBankIds`' order (the
    /// banks' own).
    pub fn audio_mgr_stop_all_sfx(&mut self) {
        for bank in [BANK_PLAYER, BANK_ITEM, BANK_ENV, BANK_ENEMY, BANK_SYSTEM, BANK_OCARINA, BANK_VOICE] {
            self.stop_sfx_by_bank(bank);
        }
    }

    // ---------------------------------------------------------------------------------------
    // z_lib.c

    /// `func_80078884`: a sound with no position.
    pub fn func_80078884(&mut self, sfx_id: u16) {
        self.play_sfx_general(sfx_id, SfxPos::Default, 4, SfxF32::One, SfxF32::One, SfxS8::Zero);
    }

    /// `func_800788CC`: the same.
    pub fn func_800788cc(&mut self, sfx_id: u16) {
        self.play_sfx_general(sfx_id, SfxPos::Default, 4, SfxF32::One, SfxF32::One, SfxS8::Zero);
    }

    /// `func_80078914`: a sound at `pos`.
    pub fn func_80078914(&mut self, pos: SfxPos, sfx_id: u16) {
        self.play_sfx_general(sfx_id, pos, 4, SfxF32::One, SfxF32::One, SfxS8::Zero);
    }
}

/// `Audio_ComputeSfxPanSigned`: by the screen position's x against its depth.
pub fn compute_sfx_pan_signed(x: f32, z: f32, _token: u8) -> i8 {
    let abs_x = x.abs().min(8000.0);
    let abs_z = z.abs().min(8000.0);
    let mut pan = if x == 0.0 && z == 0.0 {
        0.5
    } else if abs_z <= abs_x {
        let p = (16000.0 - abs_x) / (3.3 * (16000.0 - abs_z));
        if x >= 0.0 { 1.0 - p } else { p }
    } else {
        (x / (5.076_923_4 * abs_z)) + 0.5 // about 66 / 13
    };
    if abs_z < 50.0 && abs_x < 50.0 {
        pan = ((pan - 0.5) * (abs_x / 50.0) * (abs_x / 50.0)) + 0.5;
    }
    ((pan * 127.0) + 0.5) as i32 as i8
}

/// `func_800F3990`: the surround mode's height.
pub fn func_800f3990(arg0: f32, _sfx_params: u16) -> i8 {
    let mut ret: i8 = 0;
    if arg0 >= 0.0 {
        ret = if arg0 > 625.0 { 127 } else { ((arg0 / 625.0) * 126.0) as i32 as i8 };
    }
    ret | 1
}

/// `NA_SE_*` ids the sound effect code names (`sfx.h`'s enum, from the banks' tables: the id is
/// the bank's `NA_SE_*_BASE` plus one plus the row; the pack's test checks each against its
/// table's name).
pub const NA_SE_PL_WALK_GROUND: u16 = 0x0800;
pub const NA_SE_PL_WALK_SAND: u16 = 0x0801;
pub const NA_SE_PL_WALK_CONCRETE: u16 = 0x0802;
pub const NA_SE_PL_WALK_DIRT: u16 = 0x0803;
pub const NA_SE_PL_WALK_WATER0: u16 = 0x0804;
pub const NA_SE_PL_WALK_WATER1: u16 = 0x0805;
pub const NA_SE_PL_WALK_LADDER: u16 = 0x080A;
pub const NA_SE_PL_WALK_WALL: u16 = 0x080C;
pub const NA_SE_PL_WALK_HEAVYBOOTS: u16 = 0x080D;
pub const NA_SE_PL_JUMP: u16 = 0x0810;
pub const NA_SE_PL_JUMP_HEAVYBOOTS: u16 = 0x081D;
pub const NA_SE_PL_LAND: u16 = 0x0820;
pub const NA_SE_PL_LAND_HEAVYBOOTS: u16 = 0x082D;
pub const NA_SE_PL_SLIPDOWN: u16 = 0x0830;
pub const NA_SE_PL_CLIMB_CLIFF: u16 = 0x0831;
pub const NA_SE_PL_ROLL: u16 = 0x083C;
pub const NA_SE_PL_SKIP: u16 = 0x083D;
pub const NA_SE_PL_BODY_HIT: u16 = 0x083E;
pub const NA_SE_PL_DAMAGE: u16 = 0x083F;
pub const NA_SE_PL_BOUND: u16 = 0x0850;
pub const NA_SE_PL_FACE_UP: u16 = 0x0863;
pub const NA_SE_PL_DIVE_BUBBLE: u16 = 0x0864;
pub const NA_SE_PL_METALEFFECT_KID: u16 = 0x0866;
pub const NA_SE_PL_METALEFFECT_ADULT: u16 = 0x0867;
pub const NA_SE_PL_IN_BUBBLE: u16 = 0x086B;
pub const NA_SE_PL_SWORD_CHARGE: u16 = 0x086D;
pub const NA_SE_PL_ROLL_DUST: u16 = 0x08C1;
pub const NA_SE_IT_SWORD_SWING: u16 = 0x1801;
pub const NA_SE_IT_SHIELD_BOUND: u16 = 0x1806;
pub const NA_SE_IT_SHIELD_REFLECT_SW: u16 = 0x1808;
pub const NA_SE_IT_SWORD_STRIKE: u16 = 0x1811;
pub const NA_SE_IT_HAMMER_SWING: u16 = 0x1812;
pub const NA_SE_IT_SWORD_SWING_HARD: u16 = 0x1818;
pub const NA_SE_IT_SWORD_CHARGE: u16 = 0x1822;
pub const NA_SE_IT_SWORD_STRIKE_HARD: u16 = 0x1824;
pub const NA_SE_IT_REFLECTION_WOOD: u16 = 0x1837;
pub const NA_SE_EV_DOOR_CLOSE: u16 = 0x2801;
pub const NA_SE_EV_RIVER_STREAM: u16 = 0x2806;
pub const NA_SE_EV_WATER_WALL_BIG: u16 = 0x2807;
pub const NA_SE_EV_MAGMA_LEVEL: u16 = 0x280B;
pub const NA_SE_EV_WALL_BROKEN: u16 = 0x2810;
pub const NA_SE_EV_CHICKEN_CRY_M: u16 = 0x2813;
pub const NA_SE_EV_BOMB_DROP_WATER: u16 = 0x2817;
pub const NA_SE_EV_FAIRY_DASH: u16 = 0x281B;
pub const NA_SE_EV_TBOX_UNLOCK: u16 = 0x281F;
pub const NA_SE_EV_TBOX_OPEN: u16 = 0x2820;
pub const NA_SE_SY_TIMER: u16 = 0x2821;
pub const NA_SE_EV_WATER_WALL: u16 = 0x2828;
pub const NA_SE_EV_BOMB_BOUND: u16 = 0x282F;
pub const NA_SE_EV_WATERDROP: u16 = 0x2830;
pub const NA_SE_EV_TORCH: u16 = 0x2831;
pub const NA_SE_EV_MAGMA_LEVEL_M: u16 = 0x2832;
pub const NA_SE_EV_MAGMA_LEVEL_L: u16 = 0x283B;
pub const NA_SE_EV_FOUNTAIN: u16 = 0x2841;
pub const NA_SE_EV_PLANT_BROKEN: u16 = 0x284E;
pub const NA_SE_EV_ROCK_BROKEN: u16 = 0x2852;
pub const NA_SE_EV_COFFIN_CAP_BOUND: u16 = 0x2856;
pub const NA_SE_EV_CROWD: u16 = 0x285D;
pub const NA_SE_EV_NAVY_VANISH: u16 = 0x285F;
pub const NA_SE_EV_WOODDOOR_OPEN: u16 = 0x2866;
pub const NA_SE_EV_TRE_BOX_APPEAR: u16 = 0x287B;
pub const NA_SE_EV_CHAIN_KEY_UNLOCK: u16 = 0x287C;
pub const NA_SE_EV_TRE_BOX_FLASH: u16 = 0x2884;
pub const NA_SE_EV_DIVE_INTO_WATER: u16 = 0x2889;
pub const NA_SE_EV_JUMP_OUT_WATER: u16 = 0x288A;
pub const NA_SE_EV_EARTHQUAKE: u16 = 0x2898;
pub const NA_SE_EV_SARIA_MELODY: u16 = 0x28A6;
pub const NA_SE_EV_FIATY_HEAL: u16 = 0x28A8;
pub const NA_SE_EV_DOG_CRY_EVENING: u16 = 0x28AE;
pub const NA_SE_EV_WOODPLATE_BOUND: u16 = 0x28B4;
pub const NA_SE_EV_BIGBALL_ROLL: u16 = 0x28B8;
pub const NA_SE_EV_SAND_STORM: u16 = 0x28C0;
pub const NA_SE_EV_WATER_BUBBLE: u16 = 0x28CA;
pub const NA_SE_EV_KENJA_ENVIROMENT_0: u16 = 0x28D6;
pub const NA_SE_EV_KENJA_ENVIROMENT_1: u16 = 0x28D7;
pub const NA_SE_EV_IRON_DOOR_OPEN: u16 = 0x28DB;
pub const NA_SE_EV_IRON_DOOR_CLOSE: u16 = 0x28DC;
pub const NA_SE_EV_COW_CRY_LV: u16 = 0x28E5;
pub const NA_SE_EN_MIMICK_BREATH: u16 = 0x39F1;
pub const NA_SE_SY_WIN_OPEN: u16 = 0x4800;
pub const NA_SE_SY_WIN_CLOSE: u16 = 0x4801;
pub const NA_SE_SY_CORRECT_CHIME: u16 = 0x4802;
pub const NA_SE_SY_GET_RUPY: u16 = 0x4803;
pub const NA_SE_SY_ERROR: u16 = 0x4806;
pub const NA_SE_SY_TRE_BOX_APPEAR: u16 = 0x4807;
pub const NA_SE_SY_DECIDE: u16 = 0x4808;
pub const NA_SE_SY_CURSOR: u16 = 0x4809;
pub const NA_SE_SY_HP_RECOVER: u16 = 0x480B;
pub const NA_SE_SY_ATTENTION_ON: u16 = 0x480C;
pub const NA_SE_SY_LOCK_OFF: u16 = 0x480F;
pub const NA_SE_SY_LOCK_ON_HUMAN: u16 = 0x4810;
pub const NA_SE_SY_CAMERA_ZOOM_UP: u16 = 0x4813;
pub const NA_SE_SY_CAMERA_ZOOM_DOWN: u16 = 0x4814;
pub const NA_SE_SY_MESSAGE_PASS: u16 = 0x4818;
pub const NA_SE_SY_HITPOINT_ALARM: u16 = 0x481B;
pub const NA_SE_SY_GET_ITEM: u16 = 0x4824;
pub const NA_SE_SY_MESSAGE_END: u16 = 0x482E;
pub const NA_SE_SY_RUPY_COUNT: u16 = 0x482F;
pub const NA_SE_SY_LOCK_ON: u16 = 0x4830;
pub const NA_SE_SY_GET_BOXITEM: u16 = 0x4831;
pub const NA_SE_SY_ATTENTION_URGENCY: u16 = 0x4837;
pub const NA_SE_OC_ABYSS: u16 = 0x5801;
pub const NA_SE_OC_DOOR_OPEN: u16 = 0x5802;
pub const NA_SE_OC_SECRET_WARP_IN: u16 = 0x5803;
pub const NA_SE_OC_SECRET_HOLE_OUT: u16 = 0x5805;
pub const NA_SE_VO_LI_SWORD_N: u16 = 0x6800;
pub const NA_SE_VO_LI_SWORD_L: u16 = 0x6801;
pub const NA_SE_VO_LI_HANG: u16 = 0x6803;
pub const NA_SE_VO_LI_CLIMB_END: u16 = 0x6804;
pub const NA_SE_VO_LI_DAMAGE_S: u16 = 0x6805;
pub const NA_SE_VO_LI_FREEZE: u16 = 0x6806;
pub const NA_SE_VO_LI_FALL_S: u16 = 0x6807;
pub const NA_SE_VO_LI_FALL_L: u16 = 0x6808;
pub const NA_SE_VO_LI_BREATH_DRINK: u16 = 0x680A;
pub const NA_SE_VO_LI_TAKEN_AWAY: u16 = 0x680C;
pub const NA_SE_VO_LI_SNEEZE: u16 = 0x680E;
pub const NA_SE_VO_LI_SWEAT: u16 = 0x680F;
pub const NA_SE_VO_LI_RELAX: u16 = 0x6811;
pub const NA_SE_VO_LI_AUTO_JUMP: u16 = 0x6814;
pub const NA_SE_VO_LI_HOOKSHOT_HANG: u16 = 0x6819;
pub const NA_SE_VO_LI_LAND_DAMAGE_S: u16 = 0x681A;
pub const NA_SE_VO_NAVY_ENEMY: u16 = 0x6840;
pub const NA_SE_VO_NAVY_HELLO: u16 = 0x6841;
pub const NA_SE_VO_NAVY_HEAR: u16 = 0x6842;
pub const NA_SE_VO_NAVY_CALL: u16 = 0x6843;
pub const NA_SE_VO_NA_HELLO_2: u16 = 0x685F;
pub const NA_SE_VO_SK_LAUGH: u16 = 0x6873;
/// The constants above with their names, for the pack's test.
pub const NAMED_SFX: &[(&str, u16)] = &[
    ("NA_SE_PL_WALK_GROUND", NA_SE_PL_WALK_GROUND),
    ("NA_SE_PL_WALK_SAND", NA_SE_PL_WALK_SAND),
    ("NA_SE_PL_WALK_CONCRETE", NA_SE_PL_WALK_CONCRETE),
    ("NA_SE_PL_WALK_DIRT", NA_SE_PL_WALK_DIRT),
    ("NA_SE_PL_WALK_WATER0", NA_SE_PL_WALK_WATER0),
    ("NA_SE_PL_WALK_WATER1", NA_SE_PL_WALK_WATER1),
    ("NA_SE_PL_WALK_LADDER", NA_SE_PL_WALK_LADDER),
    ("NA_SE_PL_WALK_WALL", NA_SE_PL_WALK_WALL),
    ("NA_SE_PL_WALK_HEAVYBOOTS", NA_SE_PL_WALK_HEAVYBOOTS),
    ("NA_SE_PL_JUMP", NA_SE_PL_JUMP),
    ("NA_SE_PL_JUMP_HEAVYBOOTS", NA_SE_PL_JUMP_HEAVYBOOTS),
    ("NA_SE_PL_LAND", NA_SE_PL_LAND),
    ("NA_SE_PL_LAND_HEAVYBOOTS", NA_SE_PL_LAND_HEAVYBOOTS),
    ("NA_SE_PL_SLIPDOWN", NA_SE_PL_SLIPDOWN),
    ("NA_SE_PL_CLIMB_CLIFF", NA_SE_PL_CLIMB_CLIFF),
    ("NA_SE_PL_ROLL", NA_SE_PL_ROLL),
    ("NA_SE_PL_SKIP", NA_SE_PL_SKIP),
    ("NA_SE_PL_BODY_HIT", NA_SE_PL_BODY_HIT),
    ("NA_SE_PL_DAMAGE", NA_SE_PL_DAMAGE),
    ("NA_SE_PL_BOUND", NA_SE_PL_BOUND),
    ("NA_SE_PL_FACE_UP", NA_SE_PL_FACE_UP),
    ("NA_SE_PL_DIVE_BUBBLE", NA_SE_PL_DIVE_BUBBLE),
    ("NA_SE_PL_METALEFFECT_KID", NA_SE_PL_METALEFFECT_KID),
    ("NA_SE_PL_METALEFFECT_ADULT", NA_SE_PL_METALEFFECT_ADULT),
    ("NA_SE_PL_IN_BUBBLE", NA_SE_PL_IN_BUBBLE),
    ("NA_SE_PL_SWORD_CHARGE", NA_SE_PL_SWORD_CHARGE),
    ("NA_SE_PL_ROLL_DUST", NA_SE_PL_ROLL_DUST),
    ("NA_SE_IT_SWORD_SWING", NA_SE_IT_SWORD_SWING),
    ("NA_SE_IT_SHIELD_BOUND", NA_SE_IT_SHIELD_BOUND),
    ("NA_SE_IT_SHIELD_REFLECT_SW", NA_SE_IT_SHIELD_REFLECT_SW),
    ("NA_SE_IT_SWORD_STRIKE", NA_SE_IT_SWORD_STRIKE),
    ("NA_SE_IT_HAMMER_SWING", NA_SE_IT_HAMMER_SWING),
    ("NA_SE_IT_SWORD_SWING_HARD", NA_SE_IT_SWORD_SWING_HARD),
    ("NA_SE_IT_SWORD_CHARGE", NA_SE_IT_SWORD_CHARGE),
    ("NA_SE_IT_SWORD_STRIKE_HARD", NA_SE_IT_SWORD_STRIKE_HARD),
    ("NA_SE_IT_REFLECTION_WOOD", NA_SE_IT_REFLECTION_WOOD),
    ("NA_SE_EV_DOOR_CLOSE", NA_SE_EV_DOOR_CLOSE),
    ("NA_SE_EV_RIVER_STREAM", NA_SE_EV_RIVER_STREAM),
    ("NA_SE_EV_WATER_WALL_BIG", NA_SE_EV_WATER_WALL_BIG),
    ("NA_SE_EV_MAGMA_LEVEL", NA_SE_EV_MAGMA_LEVEL),
    ("NA_SE_EV_WALL_BROKEN", NA_SE_EV_WALL_BROKEN),
    ("NA_SE_EV_CHICKEN_CRY_M", NA_SE_EV_CHICKEN_CRY_M),
    ("NA_SE_EV_BOMB_DROP_WATER", NA_SE_EV_BOMB_DROP_WATER),
    ("NA_SE_EV_FAIRY_DASH", NA_SE_EV_FAIRY_DASH),
    ("NA_SE_EV_TBOX_UNLOCK", NA_SE_EV_TBOX_UNLOCK),
    ("NA_SE_EV_TBOX_OPEN", NA_SE_EV_TBOX_OPEN),
    ("NA_SE_SY_TIMER", NA_SE_SY_TIMER),
    ("NA_SE_EV_WATER_WALL", NA_SE_EV_WATER_WALL),
    ("NA_SE_EV_BOMB_BOUND", NA_SE_EV_BOMB_BOUND),
    ("NA_SE_EV_WATERDROP", NA_SE_EV_WATERDROP),
    ("NA_SE_EV_TORCH", NA_SE_EV_TORCH),
    ("NA_SE_EV_MAGMA_LEVEL_M", NA_SE_EV_MAGMA_LEVEL_M),
    ("NA_SE_EV_MAGMA_LEVEL_L", NA_SE_EV_MAGMA_LEVEL_L),
    ("NA_SE_EV_FOUNTAIN", NA_SE_EV_FOUNTAIN),
    ("NA_SE_EV_PLANT_BROKEN", NA_SE_EV_PLANT_BROKEN),
    ("NA_SE_EV_ROCK_BROKEN", NA_SE_EV_ROCK_BROKEN),
    ("NA_SE_EV_COFFIN_CAP_BOUND", NA_SE_EV_COFFIN_CAP_BOUND),
    ("NA_SE_EV_CROWD", NA_SE_EV_CROWD),
    ("NA_SE_EV_NAVY_VANISH", NA_SE_EV_NAVY_VANISH),
    ("NA_SE_EV_WOODDOOR_OPEN", NA_SE_EV_WOODDOOR_OPEN),
    ("NA_SE_EV_TRE_BOX_APPEAR", NA_SE_EV_TRE_BOX_APPEAR),
    ("NA_SE_EV_CHAIN_KEY_UNLOCK", NA_SE_EV_CHAIN_KEY_UNLOCK),
    ("NA_SE_EV_TRE_BOX_FLASH", NA_SE_EV_TRE_BOX_FLASH),
    ("NA_SE_EV_DIVE_INTO_WATER", NA_SE_EV_DIVE_INTO_WATER),
    ("NA_SE_EV_JUMP_OUT_WATER", NA_SE_EV_JUMP_OUT_WATER),
    ("NA_SE_EV_EARTHQUAKE", NA_SE_EV_EARTHQUAKE),
    ("NA_SE_EV_SARIA_MELODY", NA_SE_EV_SARIA_MELODY),
    ("NA_SE_EV_FIATY_HEAL", NA_SE_EV_FIATY_HEAL),
    ("NA_SE_EV_DOG_CRY_EVENING", NA_SE_EV_DOG_CRY_EVENING),
    ("NA_SE_EV_WOODPLATE_BOUND", NA_SE_EV_WOODPLATE_BOUND),
    ("NA_SE_EV_BIGBALL_ROLL", NA_SE_EV_BIGBALL_ROLL),
    ("NA_SE_EV_SAND_STORM", NA_SE_EV_SAND_STORM),
    ("NA_SE_EV_WATER_BUBBLE", NA_SE_EV_WATER_BUBBLE),
    ("NA_SE_EV_KENJA_ENVIROMENT_0", NA_SE_EV_KENJA_ENVIROMENT_0),
    ("NA_SE_EV_KENJA_ENVIROMENT_1", NA_SE_EV_KENJA_ENVIROMENT_1),
    ("NA_SE_EV_IRON_DOOR_OPEN", NA_SE_EV_IRON_DOOR_OPEN),
    ("NA_SE_EV_IRON_DOOR_CLOSE", NA_SE_EV_IRON_DOOR_CLOSE),
    ("NA_SE_EV_COW_CRY_LV", NA_SE_EV_COW_CRY_LV),
    ("NA_SE_EN_MIMICK_BREATH", NA_SE_EN_MIMICK_BREATH),
    ("NA_SE_SY_WIN_OPEN", NA_SE_SY_WIN_OPEN),
    ("NA_SE_SY_WIN_CLOSE", NA_SE_SY_WIN_CLOSE),
    ("NA_SE_SY_CORRECT_CHIME", NA_SE_SY_CORRECT_CHIME),
    ("NA_SE_SY_GET_RUPY", NA_SE_SY_GET_RUPY),
    ("NA_SE_SY_ERROR", NA_SE_SY_ERROR),
    ("NA_SE_SY_TRE_BOX_APPEAR", NA_SE_SY_TRE_BOX_APPEAR),
    ("NA_SE_SY_DECIDE", NA_SE_SY_DECIDE),
    ("NA_SE_SY_CURSOR", NA_SE_SY_CURSOR),
    ("NA_SE_SY_HP_RECOVER", NA_SE_SY_HP_RECOVER),
    ("NA_SE_SY_ATTENTION_ON", NA_SE_SY_ATTENTION_ON),
    ("NA_SE_SY_LOCK_OFF", NA_SE_SY_LOCK_OFF),
    ("NA_SE_SY_LOCK_ON_HUMAN", NA_SE_SY_LOCK_ON_HUMAN),
    ("NA_SE_SY_CAMERA_ZOOM_UP", NA_SE_SY_CAMERA_ZOOM_UP),
    ("NA_SE_SY_CAMERA_ZOOM_DOWN", NA_SE_SY_CAMERA_ZOOM_DOWN),
    ("NA_SE_SY_MESSAGE_PASS", NA_SE_SY_MESSAGE_PASS),
    ("NA_SE_SY_HITPOINT_ALARM", NA_SE_SY_HITPOINT_ALARM),
    ("NA_SE_SY_GET_ITEM", NA_SE_SY_GET_ITEM),
    ("NA_SE_SY_MESSAGE_END", NA_SE_SY_MESSAGE_END),
    ("NA_SE_SY_RUPY_COUNT", NA_SE_SY_RUPY_COUNT),
    ("NA_SE_SY_LOCK_ON", NA_SE_SY_LOCK_ON),
    ("NA_SE_SY_GET_BOXITEM", NA_SE_SY_GET_BOXITEM),
    ("NA_SE_SY_ATTENTION_URGENCY", NA_SE_SY_ATTENTION_URGENCY),
    ("NA_SE_OC_ABYSS", NA_SE_OC_ABYSS),
    ("NA_SE_OC_DOOR_OPEN", NA_SE_OC_DOOR_OPEN),
    ("NA_SE_OC_SECRET_WARP_IN", NA_SE_OC_SECRET_WARP_IN),
    ("NA_SE_OC_SECRET_HOLE_OUT", NA_SE_OC_SECRET_HOLE_OUT),
    ("NA_SE_VO_LI_SWORD_N", NA_SE_VO_LI_SWORD_N),
    ("NA_SE_VO_LI_SWORD_L", NA_SE_VO_LI_SWORD_L),
    ("NA_SE_VO_LI_HANG", NA_SE_VO_LI_HANG),
    ("NA_SE_VO_LI_CLIMB_END", NA_SE_VO_LI_CLIMB_END),
    ("NA_SE_VO_LI_DAMAGE_S", NA_SE_VO_LI_DAMAGE_S),
    ("NA_SE_VO_LI_FREEZE", NA_SE_VO_LI_FREEZE),
    ("NA_SE_VO_LI_FALL_S", NA_SE_VO_LI_FALL_S),
    ("NA_SE_VO_LI_FALL_L", NA_SE_VO_LI_FALL_L),
    ("NA_SE_VO_LI_BREATH_DRINK", NA_SE_VO_LI_BREATH_DRINK),
    ("NA_SE_VO_LI_TAKEN_AWAY", NA_SE_VO_LI_TAKEN_AWAY),
    ("NA_SE_VO_LI_SNEEZE", NA_SE_VO_LI_SNEEZE),
    ("NA_SE_VO_LI_SWEAT", NA_SE_VO_LI_SWEAT),
    ("NA_SE_VO_LI_RELAX", NA_SE_VO_LI_RELAX),
    ("NA_SE_VO_LI_AUTO_JUMP", NA_SE_VO_LI_AUTO_JUMP),
    ("NA_SE_VO_LI_HOOKSHOT_HANG", NA_SE_VO_LI_HOOKSHOT_HANG),
    ("NA_SE_VO_LI_LAND_DAMAGE_S", NA_SE_VO_LI_LAND_DAMAGE_S),
    ("NA_SE_VO_NAVY_ENEMY", NA_SE_VO_NAVY_ENEMY),
    ("NA_SE_VO_NAVY_HELLO", NA_SE_VO_NAVY_HELLO),
    ("NA_SE_VO_NAVY_HEAR", NA_SE_VO_NAVY_HEAR),
    ("NA_SE_VO_NAVY_CALL", NA_SE_VO_NAVY_CALL),
    ("NA_SE_VO_NA_HELLO_2", NA_SE_VO_NA_HELLO_2),
    ("NA_SE_VO_SK_LAUGH", NA_SE_VO_SK_LAUGH),
];
