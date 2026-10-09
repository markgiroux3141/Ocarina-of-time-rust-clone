//! `z_sram.c`: the save on the cartridge's SRAM (docs/adr/0049-saving.md).
//!
//! The SRAM is 32 KiB (`SRAM_SIZE`): a 16-byte header (the sound and Z-targeting settings, the
//! language, the magic `98 09 10 21 "ZELDA"`), then six slots of `SLOT_SIZE` bytes from 0x20
//! (`gSramSlotOffsets`): the three files, then their backups. A slot is what `Sram_WriteSave`
//! copies from `&gSaveContext`: `Save` (`save.h`, 0x1354 bytes, the only part read back and
//! checksummed), the rest of `SaveContext` (the play state's part), and 0x28 bytes past it.
//! Everything is big-endian, as the cartridge holds it.
//!
//! The port keeps the SRAM as an image (`Sram`) and never touches the disk here: the apps bind
//! an image to a file (`out/saves/<ROM SHA-1>.sra`), the tests build theirs in memory.
//! `SaveContext` is flat where the C nests `Save`, `SaveInfo` and `SavePlayerData`;
//! `SaveContext::save_bytes` and `read_save` lay it out field by field in the C's layout.

use crate::item::*;
use crate::save::*;

/// `SRAM_SIZE`, `SRAM_HEADER_SIZE` (`sram.h`).
pub const SRAM_SIZE: usize = 0x8000;
pub const SRAM_HEADER_SIZE: usize = 0x10;
/// `SramHeaderField`: `SRAM_HEADER_SOUND`, `_Z_TARGET`, `_LANGUAGE`, `_MAGIC`.
pub const SRAM_HEADER_SOUND: usize = 0;
pub const SRAM_HEADER_Z_TARGET: usize = 1;
pub const SRAM_HEADER_LANGUAGE: usize = 2;
pub const SRAM_HEADER_MAGIC: usize = 3;
/// `sizeof(Save)`, `sizeof(SaveContext)` (`save.h`).
pub const SAVE_SIZE: usize = 0x1354;
pub const SAVE_CONTEXT_SIZE: usize = 0x1428;
/// `SLOT_SIZE`: `sizeof(SaveContext) + 0x28`.
pub const SLOT_SIZE: usize = SAVE_CONTEXT_SIZE + 0x28;
/// `CHECKSUM_SIZE`: `Save`'s halfwords.
pub const CHECKSUM_SIZE: usize = SAVE_SIZE / 2;
/// Where `save.info.checksum` is, from `Save`'s start.
pub const CHECKSUM_OFFSET: usize = 0x1352;
/// `LANGUAGE_MAX` (`!OOT_NTSC`: English, German, French).
pub const LANGUAGE_MAX: u8 = 3;

/// `SLOT_OFFSET(index)`.
pub const fn slot_offset(index: usize) -> usize {
    SRAM_HEADER_SIZE + 0x10 + index * SLOT_SIZE
}

/// `gSramSlotOffsets`: the three files, then their backups.
pub const SRAM_SLOT_OFFSETS: [u16; 6] = [slot_offset(0) as u16, slot_offset(1) as u16, slot_offset(2) as u16, slot_offset(3) as u16, slot_offset(4) as u16, slot_offset(5) as u16];

/// `sSramDefaultHeader`: `SOUND_SETTING_STEREO`, `Z_TARGET_SETTING_SWITCH`, `LANGUAGE_ENG`
/// (`!OOT_NTSC`), the magic.
pub const SRAM_DEFAULT_HEADER: [u8; 12] = [0, 0, LANGUAGE_ENG, 0x98, 0x09, 0x10, 0x21, b'Z', b'E', b'L', b'D', b'A'];

/// `offsetof(SaveContext, ...)` of what the file select shows: `DEATHS`, `NAME`, `N64DD`,
/// `HEALTH_CAP`, `QUEST`, `DEFENSE`, `HEALTH` (`OOT_PAL`), and `newf` (`GET_NEWF`).
pub const DEATHS: usize = 0x22;
pub const NAME: usize = 0x24;
pub const N64DD: usize = 0x2C;
pub const HEALTH_CAP: usize = 0x2E;
pub const QUEST: usize = 0xA4;
pub const DEFENSE: usize = 0xCF;
pub const HEALTH: usize = 0x30;
pub const NEWF: usize = 0x1C;

/// `ENTR_*` (`entrance_table.h`) `Sram_OpenSave` enters by.
const ENTR_DODONGOS_CAVERN_0: u16 = 0x004;
const ENTR_JABU_JABU_0: u16 = 0x028;
const ENTR_FOREST_TEMPLE_0: u16 = 0x169;
const ENTR_FIRE_TEMPLE_0: u16 = 0x165;
const ENTR_WATER_TEMPLE_0: u16 = 0x010;
const ENTR_SPIRIT_TEMPLE_0: u16 = 0x082;
const ENTR_SHADOW_TEMPLE_0: u16 = 0x037;
const ENTR_GANONS_TOWER_0: u16 = 0x41B;
const ENTR_TEMPLE_OF_TIME_7: u16 = 0x5F4;
/// `ENTR_DEKU_TREE_0`.
pub const ENTR_DEKU_TREE_0: u16 = 0x000;

/// `sDungeonEntrances`: by scene, the dungeons' entrances (the two collapses' are unused:
/// `Sram_OpenSave` sends them to Ganon's tower).
const DUNGEON_ENTRANCES: [u16; 16] = [
    ENTR_DEKU_TREE_0,
    ENTR_DODONGOS_CAVERN_0,
    ENTR_JABU_JABU_0,
    ENTR_FOREST_TEMPLE_0,
    ENTR_FIRE_TEMPLE_0,
    ENTR_WATER_TEMPLE_0,
    ENTR_SPIRIT_TEMPLE_0,
    ENTR_SHADOW_TEMPLE_0,
    0x098, // ENTR_BOTTOM_OF_THE_WELL_0
    0x088, // ENTR_ICE_CAVERN_0
    ENTR_GANONS_TOWER_0,
    0x008, // ENTR_GERUDO_TRAINING_GROUND_0
    0x486, // ENTR_THIEVES_HIDEOUT_0
    0x467, // ENTR_INSIDE_GANONS_CASTLE_0
    0x179, // ENTR_GANONS_TOWER_COLLAPSE_INTERIOR_0
    0x56C, // ENTR_INSIDE_GANONS_CASTLE_COLLAPSE_0
];

/// `gSpoilingItems`, `gSpoilingItemReverts` (`z_parameter.c`): the odd mushroom, the eyeball
/// frog and the eye drops go back to Cojiro and the prescription.
const SPOILING_ITEMS: [u8; 3] = [0x30, 0x35, 0x36];
const SPOILING_ITEM_REVERTS: [u8; 3] = [0x2F, 0x34, 0x34];

/// The cartridge's SRAM (`z_ss_sram.c`'s device): the image, and how many writes it has taken
/// (the apps write it to its file when that changes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sram {
    pub bytes: Vec<u8>,
    pub writes: u32,
}

impl Default for Sram {
    /// A fresh image: zeros, which every slot's checksum accepts (zero sums to zero): three
    /// empty files.
    fn default() -> Sram {
        Sram { bytes: vec![0; SRAM_SIZE], writes: 0 }
    }
}

impl Sram {
    /// An image from a file's bytes (`SRAM_SIZE` of them).
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Sram, String> {
        if bytes.len() != SRAM_SIZE {
            return Err(format!("an SRAM image is {SRAM_SIZE:#x} bytes, not {:#x}", bytes.len()));
        }
        Ok(Sram { bytes, writes: 0 })
    }

    /// `SRAM_READ`: `SsSram_ReadWrite(OS_READ)` at `addr` from the SRAM's start.
    pub fn read(&self, addr: usize, dst: &mut [u8]) {
        dst.copy_from_slice(&self.bytes[addr..addr + dst.len()]);
    }

    /// `SRAM_WRITE`: `SsSram_ReadWrite(OS_WRITE)`.
    pub fn write(&mut self, addr: usize, src: &[u8]) {
        self.bytes[addr..addr + src.len()].copy_from_slice(src);
        self.writes += 1;
    }
}

/// `SramContext`: the read buffer (`Sram_Alloc`; play has none, `Sram_Init` is empty).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SramContext {
    pub read_buff: Vec<u8>,
}

impl SramContext {
    /// `Sram_Alloc`: the `SRAM_SIZE` read buffer.
    pub fn sram_alloc() -> SramContext {
        SramContext { read_buff: vec![0; SRAM_SIZE] }
    }
}

/// The parts of `FileSelectState` (`file_select_state.h`) `z_sram.c` reads and writes: what the
/// file select shows of each file, and the file picked.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileSelectState {
    pub n64dd_flag: u8,
    pub deaths: [u16; 3],
    pub file_names: [[u8; 8]; 3],
    pub health_capacities: [u16; 3],
    pub quest_items: [u32; 3],
    pub n64dd_flags: [i16; 3],
    pub defense: [i8; 3],
    pub health: [u16; 3],
    pub button_index: i16,
    pub selected_file_index: i16,
    pub copy_dest_file_index: i16,
}

impl FileSelectState {
    /// The file select's fields of `slot` from the read buffer (`MemCopy` from `readBuff +
    /// SLOT_OFFSET(slot) + DEATHS` and the others).
    fn read_slot(&mut self, read_buff: &[u8], file: usize, offset: usize) {
        let b = &read_buff[offset..];
        self.deaths[file] = be16(b, DEATHS);
        self.file_names[file].copy_from_slice(&b[NAME..NAME + 8]);
        self.health_capacities[file] = be16(b, HEALTH_CAP);
        self.quest_items[file] = u32::from_be_bytes([b[QUEST], b[QUEST + 1], b[QUEST + 2], b[QUEST + 3]]);
        self.n64dd_flags[file] = be16(b, N64DD) as i16;
        self.defense[file] = b[DEFENSE] as i8;
        self.health[file] = be16(b, HEALTH);
    }
}

fn be16(b: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([b[at], b[at + 1]])
}

/// `SLOT_OCCUPIED` (`file_select.h`): any of `newf`'s letters where "ZELDAZ" has it.
pub fn slot_occupied(read_buff: &[u8], slot: usize) -> bool {
    let at = SRAM_SLOT_OFFSETS[slot] as usize + NEWF;
    read_buff[at..at + 6].iter().zip(b"ZELDAZ").any(|(a, b)| a == b)
}

/// The sum of `Save`'s big-endian halfwords, its checksum field taken as 0 (what
/// `Sram_WriteSave` and `Sram_VerifyAndLoadAllSaves` compute with `checksum.value` cleared).
pub fn checksum(save: &[u8]) -> u16 {
    (0..CHECKSUM_SIZE).filter(|&i| i * 2 != CHECKSUM_OFFSET).fold(0u16, |sum, i| sum.wrapping_add(be16(save, i * 2)))
}

/// Reading or writing the C's layout, field by field (`SaveContext::visit_save`).
trait Io {
    fn at(&self) -> usize;
    fn bytes(&mut self, v: &mut [u8]);
    fn pad(&mut self, n: usize);
    fn u8(&mut self, v: &mut u8) {
        let mut b = [*v];
        self.bytes(&mut b);
        *v = b[0];
    }
    fn i8(&mut self, v: &mut i8) {
        let mut b = [*v as u8];
        self.bytes(&mut b);
        *v = b[0] as i8;
    }
    fn u16(&mut self, v: &mut u16) {
        let mut b = v.to_be_bytes();
        self.bytes(&mut b);
        *v = u16::from_be_bytes(b);
    }
    fn i16(&mut self, v: &mut i16) {
        let mut b = v.to_be_bytes();
        self.bytes(&mut b);
        *v = i16::from_be_bytes(b);
    }
    fn u32(&mut self, v: &mut u32) {
        let mut b = v.to_be_bytes();
        self.bytes(&mut b);
        *v = u32::from_be_bytes(b);
    }
    fn i32(&mut self, v: &mut i32) {
        let mut b = v.to_be_bytes();
        self.bytes(&mut b);
        *v = i32::from_be_bytes(b);
    }
    fn f32(&mut self, v: &mut f32) {
        let mut b = v.to_be_bytes();
        self.bytes(&mut b);
        *v = f32::from_be_bytes(b);
    }
    /// A `u8` the port holds as a `bool` (0 or 1 written; anything but 0 read as true).
    fn bool8(&mut self, v: &mut bool) {
        let mut b = *v as u8;
        self.u8(&mut b);
        *v = b != 0;
    }
    /// An `s32` the port holds as a `bool`.
    fn bool32(&mut self, v: &mut bool) {
        let mut b = *v as i32;
        self.i32(&mut b);
        *v = b != 0;
    }
    fn equips(&mut self, e: &mut ItemEquips) {
        self.bytes(&mut e.button_items);
        self.bytes(&mut e.c_button_slots);
        self.pad(1);
        self.u16(&mut e.equipment);
    }
    fn check(&self, offset: usize) {
        debug_assert_eq!(self.at(), offset, "the save's layout is off");
    }
}

struct Writer<'a> {
    buf: &'a mut [u8],
    at: usize,
}

impl Io for Writer<'_> {
    fn at(&self) -> usize {
        self.at
    }
    fn bytes(&mut self, v: &mut [u8]) {
        self.buf[self.at..self.at + v.len()].copy_from_slice(v);
        self.at += v.len();
    }
    fn pad(&mut self, n: usize) {
        self.at += n;
    }
}

struct Reader<'a> {
    buf: &'a [u8],
    at: usize,
}

impl Io for Reader<'_> {
    fn at(&self) -> usize {
        self.at
    }
    fn bytes(&mut self, v: &mut [u8]) {
        v.copy_from_slice(&self.buf[self.at..self.at + v.len()]);
        self.at += v.len();
    }
    fn pad(&mut self, n: usize) {
        self.at += n;
    }
}

impl SaveContext {
    /// `Save` in the C's layout (`save.h`; the offsets checked are its comments' second column,
    /// from `Save`'s start). The port's `bool`s are the C's `u8`s and `s32`s (`linkAge`: 0 the
    /// adult), its `u16` entrances the C's `s32`s and `s16`s (sign-extended).
    fn visit_save(&mut self, io: &mut impl Io) {
        let mut entrance = self.entrance_index as i16 as i32;
        io.i32(&mut entrance);
        self.entrance_index = entrance as u16;
        let mut link_age = if self.adult { LINK_AGE_ADULT as i32 } else { LINK_AGE_CHILD as i32 };
        io.i32(&mut link_age);
        self.adult = link_age == LINK_AGE_ADULT as i32;
        let mut cutscene_index = self.cutscene_index as i32;
        io.i32(&mut cutscene_index);
        self.cutscene_index = cutscene_index as u16;
        io.u16(&mut self.day_time);
        io.pad(2);
        io.bool32(&mut self.night_flag);
        io.i32(&mut self.total_days);
        io.i32(&mut self.bgs_day_count);
        // SaveInfo: playerData.
        io.check(0x1C);
        io.bytes(&mut self.newf);
        io.u16(&mut self.deaths);
        io.bytes(&mut self.player_name);
        io.i16(&mut self.n64dd_flag);
        io.i16(&mut self.health_capacity);
        io.i16(&mut self.health);
        io.i8(&mut self.magic_level);
        io.i8(&mut self.magic);
        io.i16(&mut self.rupees);
        io.u16(&mut self.sword_health);
        io.u16(&mut self.navi_timer);
        io.bool8(&mut self.is_magic_acquired);
        io.u8(&mut self.unk_3b);
        io.bool8(&mut self.is_double_magic_acquired);
        io.bool8(&mut self.is_double_defense_acquired);
        io.bool8(&mut self.bgs_flag);
        io.u8(&mut self.ocarina_game_round_num);
        io.check(0x40);
        io.equips(&mut self.child_equips);
        io.equips(&mut self.adult_equips);
        io.u32(&mut self.unk_54);
        io.bytes(&mut self.unk_58);
        let mut saved_scene_id = self.saved_scene_id as i16;
        io.i16(&mut saved_scene_id);
        self.saved_scene_id = saved_scene_id as u16;
        io.check(0x68);
        io.equips(&mut self.equips);
        io.pad(2);
        io.check(0x74);
        let inv = &mut self.inventory;
        io.bytes(&mut inv.items);
        for a in &mut inv.ammo {
            io.i8(a);
        }
        io.u16(&mut inv.equipment);
        io.pad(2);
        io.u32(&mut inv.upgrades);
        io.u32(&mut inv.quest_items);
        io.bytes(&mut inv.dungeon_items);
        for k in &mut inv.dungeon_keys {
            io.i8(k);
        }
        io.i8(&mut inv.defense_hearts);
        io.i16(&mut inv.gs_tokens);
        io.pad(2);
        io.check(0xD4);
        self.scene_flags.resize(SCENE_FLAGS_COUNT, SavedSceneFlags::default());
        for f in &mut self.scene_flags {
            for v in [&mut f.chest, &mut f.swch, &mut f.clear, &mut f.collect, &mut f.unk, &mut f.rooms, &mut f.floors] {
                io.u32(v);
            }
        }
        io.check(0xE64);
        let fw = &mut self.fw;
        for v in &mut fw.pos {
            io.i32(v);
        }
        for v in [&mut fw.yaw, &mut fw.player_params, &mut fw.entrance_index, &mut fw.room_index, &mut fw.set, &mut fw.temp_swch_flags, &mut fw.temp_collect_flags] {
            io.i32(v);
        }
        io.bytes(&mut self.unk_e8c);
        io.check(0xE9C);
        for v in &mut self.gs_flags {
            io.i32(v);
        }
        io.bytes(&mut self.unk_eb4);
        for v in &mut self.high_scores {
            io.i32(v);
        }
        io.check(0xED4);
        for v in &mut self.event_chk_inf {
            io.u16(v);
        }
        for v in &mut self.item_get_inf {
            io.u16(v);
        }
        for v in &mut self.inf_table {
            io.u16(v);
        }
        io.bytes(&mut self.unk_f34);
        io.u32(&mut self.world_map_area_data);
        io.bytes(&mut self.unk_f3c);
        io.check(0xF40);
        io.u8(&mut self.scarecrow_long_song_set);
        io.bytes(&mut self.scarecrow_long_song);
        io.bytes(&mut self.unk_12a1);
        io.u8(&mut self.scarecrow_spawn_song_set);
        io.bytes(&mut self.scarecrow_spawn_song);
        io.bytes(&mut self.unk_1346);
        io.check(0x1348);
        let h = &mut self.horse_data;
        io.i16(&mut h.scene_id);
        for v in &mut h.pos {
            io.i16(v);
        }
        io.i16(&mut h.angle);
        io.u16(&mut self.checksum);
        io.check(SAVE_SIZE);
    }

    /// The rest of `SaveContext` after `Save` (0x1354 to 0x1428), from the fields the port has;
    /// what it doesn't have (the timers, the magic meter's state, the minigames, the dog, the
    /// Sun's Song, `nextDayTime`, `skyboxTime`, `worldMapArea` ...) is zeros. Never read back.
    fn visit_tail(&mut self, io: &mut impl Io) {
        io.check(SAVE_SIZE);
        io.i32(&mut self.file_num);
        io.pad(4);
        let mut game_mode = self.game_mode as i32;
        io.i32(&mut game_mode);
        let mut scene_layer = self.scene_layer as i32;
        io.i32(&mut scene_layer);
        io.i32(&mut self.respawn_flag);
        io.check(0x1368);
        for r in &mut self.respawn {
            io.f32(&mut r.pos.x);
            io.f32(&mut r.pos.y);
            io.f32(&mut r.pos.z);
            io.i16(&mut r.yaw);
            io.i16(&mut r.player_params);
            let mut e = r.entrance_index as i16;
            io.i16(&mut e);
            io.u8(&mut r.room_index);
            io.i8(&mut r.data);
            io.u32(&mut r.temp_swch_flags);
            io.u32(&mut r.temp_collect_flags);
        }
        io.check(0x13BC);
        io.f32(&mut self.entrance_speed);
        io.u16(&mut self.entrance_sound);
        io.pad(1);
        io.bool8(&mut self.retain_weather_mode);
        // dogParams.
        io.pad(2);
        io.u8(&mut self.env_hazard_text_trigger_flags);
        io.bool8(&mut self.show_title_card);
        // nayrusLoveTimer, unk_13CA.
        io.pad(4);
        io.i16(&mut self.rupee_accumulator);
        // timerState, timerSeconds, subTimerState, subTimerSeconds, timerX, timerY, unk_13DE.
        io.pad(0x12);
        io.check(0x13E0);
        io.u8(&mut self.seq_id);
        io.u8(&mut self.nature_ambience_id);
        io.bytes(&mut self.button_status);
        io.u8(&mut self.force_rising_button_alphas);
        io.u16(&mut self.next_hud_visibility_mode);
        io.u16(&mut self.hud_visibility_mode);
        io.u16(&mut self.hud_visibility_mode_timer);
        io.u16(&mut self.prev_hud_visibility_mode);
        // magicState, prevMagicState, magicCapacity, magicFillTarget, magicTarget.
        io.pad(10);
        io.check(0x13FA);
        for v in &mut self.event_inf {
            io.u16(v);
        }
        io.u16(&mut self.map_index);
        // minigameState, minigameScore, unk_1408.
        io.pad(5);
        io.u8(&mut self.language);
        io.u8(&mut self.sound_setting);
        io.pad(1);
        io.u8(&mut self.z_target_setting);
        io.pad(1);
        io.u16(&mut self.forced_seq_id);
        io.u8(&mut self.cutscene_transition_control);
        io.pad(1);
        io.u16(&mut self.next_cutscene_index);
        io.u8(&mut self.cutscene_trigger);
        // chamberCutsceneNum, nextDayTime.
        io.pad(3);
        io.check(0x1418);
        let mut fade = self.trans_fade_duration as u8;
        io.u8(&mut fade);
        io.u8(&mut self.trans_wipe_speed);
        // skyboxTime, dogIsLost.
        io.pad(3);
        io.u8(&mut self.next_transition_type);
        // unk_141E, worldMapArea, sunsSongState.
        io.pad(6);
        io.i16(&mut self.health_accumulator);
        io.pad(2);
        io.check(SAVE_CONTEXT_SIZE);
    }

    /// `Save`'s bytes (`sizeof(Save)` from `&gSaveContext`).
    pub fn save_bytes(&self) -> Vec<u8> {
        let mut b = vec![0; SAVE_SIZE];
        self.clone().visit_save(&mut Writer { buf: &mut b, at: 0 });
        b
    }

    /// A slot's bytes (`SLOT_SIZE` from `&gSaveContext`): `Save`, the rest of `SaveContext`,
    /// then the 0x28 bytes that follow `gSaveContext` in RAM, which the port writes as zeros.
    pub fn slot_bytes(&self) -> Vec<u8> {
        let mut b = vec![0; SLOT_SIZE];
        let mut s = self.clone();
        let mut w = Writer { buf: &mut b, at: 0 };
        s.visit_save(&mut w);
        s.visit_tail(&mut w);
        b
    }

    /// `MemCopy(&gSaveContext, buff, sizeof(Save))`.
    pub fn read_save(&mut self, buff: &[u8]) {
        self.visit_save(&mut Reader { buf: &buff[..SAVE_SIZE], at: 0 });
    }
}

/// `Sram_OpenSave`: the file (`fileNum`) from the read buffer into the save, then where it
/// enters by its saved scene, at least three hearts, the scarecrow songs, Zelda's letter taken
/// back without the lullaby, an adult's Master Sword, the spoiled trade items, `magicLevel` 0.
pub fn sram_open_save(save: &mut SaveContext, sram_ctx: &SramContext) {
    // "Create personal file"
    let i = SRAM_SLOT_OFFSETS[save.file_num as usize] as usize;
    save.read_save(&sram_ctx.read_buff[i..i + SAVE_SIZE]);
    save.entrance_index = match save.saved_scene_id {
        // SCENE_DEKU_TREE .. SCENE_INSIDE_GANONS_CASTLE.
        0x00..=0x0D => DUNGEON_ENTRANCES[save.saved_scene_id as usize],
        0x11 => ENTR_DEKU_TREE_0,
        0x12 => ENTR_DODONGOS_CAVERN_0,
        0x13 => ENTR_JABU_JABU_0,
        0x14 => ENTR_FOREST_TEMPLE_0,
        0x15 => ENTR_FIRE_TEMPLE_0,
        0x16 => ENTR_WATER_TEMPLE_0,
        0x17 => ENTR_SPIRIT_TEMPLE_0,
        0x18 => ENTR_SHADOW_TEMPLE_0,
        // SCENE_GANONS_TOWER_COLLAPSE_INTERIOR, _INSIDE_GANONS_CASTLE_COLLAPSE,
        // SCENE_GANONDORF_BOSS, SCENE_GANONS_TOWER_COLLAPSE_EXTERIOR, SCENE_GANON_BOSS.
        0x0E | 0x0F | 0x19 | 0x1A | 0x4F => ENTR_GANONS_TOWER_0,
        // LINK_AGE_IN_YEARS == YEARS_CHILD.
        s if s != SCENE_LINKS_HOUSE && save.adult => ENTR_TEMPLE_OF_TIME_7,
        _ => ENTR_LINKS_HOUSE_0,
    };
    if save.health < 0x30 {
        save.health = 0x30;
    }
    if save.scarecrow_long_song_set != 0 || save.scarecrow_spawn_song_set != 0 {
        // MemCopy into gScarecrowLongSongPtr / gScarecrowSpawnSongPtr: the ocarina's.
        log::info!("Sram_OpenSave: the scarecrow's songs aren't copied out (the ocarina isn't ported); the save keeps them");
    }
    // Zelda's cutscene watched but no lullaby: the cutscene back, the letter a chicken again.
    if save.get_event_chk_inf(EVENTCHKINF_OBTAINED_ZELDAS_LETTER) && !save.check_quest_item(QUEST_SONG_LULLABY) {
        save.event_chk_inf[(EVENTCHKINF_OBTAINED_ZELDAS_LETTER >> 4) as usize] &= !(1 << (EVENTCHKINF_OBTAINED_ZELDAS_LETTER & 0xF));
        save.set_inv_content(ITEM_ZELDAS_LETTER, ITEM_CHICKEN);
        for j in 1..4 {
            if save.equips.button_items[j] == ITEM_ZELDAS_LETTER {
                save.equips.button_items[j] = ITEM_CHICKEN;
            }
        }
    }
    if save.adult && !save.check_owned_equip(EQUIP_TYPE_SWORD, EQUIP_INV_SWORD_MASTER) {
        save.inventory.equipment |= owned_equip_flag(EQUIP_TYPE_SWORD, EQUIP_INV_SWORD_MASTER);
        // OOT_VERSION >= NTSC_1_1: worn, on B.
        save.equips.button_items[0] = ITEM_SWORD_MASTER;
        save.equips.equipment &= !(0xF << (EQUIP_TYPE_SWORD * 4));
        save.equips.equipment |= EQUIP_VALUE_SWORD_MASTER << (EQUIP_TYPE_SWORD * 4);
    }
    for i in 0..SPOILING_ITEMS.len() {
        // INV_CONTENT(ITEM_TRADE_ADULT): the adult's trade slot.
        if save.inventory.items[SLOT_TRADE_ADULT] == SPOILING_ITEMS[i] {
            save.set_inv_content(SPOILING_ITEM_REVERTS[i], SPOILING_ITEM_REVERTS[i]);
            for j in 1..4 {
                if save.equips.button_items[j] == SPOILING_ITEMS[i] {
                    save.equips.button_items[j] = SPOILING_ITEM_REVERTS[i];
                }
            }
        }
    }
    save.magic_level = 0;
}

/// `Sram_WriteSave`: the checksum into the save, then the slot (`SLOT_SIZE` from
/// `&gSaveContext`) written to the file's place and to its backup's.
pub fn sram_write_save(save: &mut SaveContext, sram: &mut Sram) {
    save.checksum = 0;
    save.checksum = checksum(&save.save_bytes());
    // (It sums the save twice more, between and after the writes, into nothing.)
    let file = save.file_num as usize;
    if file >= 3 {
        // @bug (game): the map select's file (0xFF) indexes past gSramSlotOffsets, and the slot
        // goes wherever the halfwords there point. The port has no such memory: it writes nothing.
        log::warn!("Sram_WriteSave: fileNum {:#x} has no slot (gSramSlotOffsets[{file}]): nothing written", save.file_num);
        return;
    }
    let slot = save.slot_bytes();
    sram.write(SRAM_SLOT_OFFSETS[file] as usize, &slot);
    sram.write(SRAM_SLOT_OFFSETS[file + 3] as usize, &slot);
}

/// `Sram_VerifyAndLoadAllSaves`: the SRAM read; each file's checksum checked, a bad one
/// restored from its backup, a bad backup too made a new save (file 1 the debug save, with
/// `DEBUG_FEATURES`) and written to both; then the file select's fields of the three files.
/// The save is left holding the last slot read, with the `dayTime` it came in with.
pub fn sram_verify_and_load_all_saves(file_select: &mut FileSelectState, save: &mut SaveContext, sram_ctx: &mut SramContext, sram: &mut Sram) {
    sram_ctx.read_buff.fill(0);
    sram.read(0, &mut sram_ctx.read_buff);
    let day_time = save.day_time;
    for slot_num in 0..3 {
        let offset = SRAM_SLOT_OFFSETS[slot_num] as usize;
        let buff = &sram_ctx.read_buff[offset..offset + SAVE_SIZE];
        save.read_save(buff);
        let old_checksum = save.checksum;
        save.checksum = 0;
        // (OOT_VERSION < PAL_1_0's `j` isn't this ROM.)
        if checksum(buff) == old_checksum {
            // "SAVE data OK!!!!"
            continue;
        }
        log::info!("Sram_VerifyAndLoadAllSaves: file {}'s checksum is bad: its backup", slot_num + 1);
        let offset = SRAM_SLOT_OFFSETS[slot_num + 3] as usize;
        let buff = &sram_ctx.read_buff[offset..offset + SAVE_SIZE];
        save.read_save(buff);
        let old_checksum = save.checksum;
        save.checksum = 0;
        if checksum(buff) != old_checksum {
            log::info!("Sram_VerifyAndLoadAllSaves: file {}'s backup is bad too: a new save", slot_num + 1);
            save.entrance_index = 0;
            save.adult = true;
            save.cutscene_index = 0;
            // @bug (game): dayTime (u16) cleared as 32 bits, into the padding after it: harmless
            // here, nightFlag being cleared next.
            save.day_time = 0;
            save.night_flag = false;
            save.total_days = 0;
            save.bgs_day_count = 0;
            // DEBUG_FEATURES: file 1 the debug save.
            if slot_num == 0 {
                save.init_debug_save();
                save.newf = *b"ZELDAZ";
            } else {
                save.init_new_save();
            }
            save.checksum = checksum(&save.save_bytes());
            let slot = save.slot_bytes();
            sram.write(SRAM_SLOT_OFFSETS[slot_num + 3] as usize, &slot);
        }
        // @bug (game): with the backup good, the save's checksum is still the 0 the check
        // left, and the file is written with it: next time its checksum is bad again, and the
        // backup restores it again. A save (Sram_WriteSave) puts it right.
        let slot = save.slot_bytes();
        sram.write(SRAM_SLOT_OFFSETS[slot_num] as usize, &slot);
    }
    sram_ctx.read_buff.fill(0);
    sram.read(0, &mut sram_ctx.read_buff);
    save.day_time = day_time;
    for file in 0..3 {
        file_select.read_slot(&sram_ctx.read_buff, file, slot_offset(file));
    }
}

/// `Sram_InitSave`: the name entry's new file in `fileNum`'s slot: file 1 the debug save
/// (`DEBUG_FEATURES`), the others a new save; Link's house, child, 10:00, the opening
/// (`CS_INDEX_1`; none for file 1); the name entered; "ZELDAZ"; the checksum; written to the
/// slot and its backup with the whole SRAM; the file select's fields of the file.
pub fn sram_init_save(file_select: &mut FileSelectState, save: &mut SaveContext, sram_ctx: &mut SramContext, sram: &mut Sram) {
    if file_select.button_index != 0 {
        save.init_new_save();
    } else {
        save.init_debug_save();
    }
    save.entrance_index = ENTR_LINKS_HOUSE_0;
    save.adult = false;
    save.day_time = crate::env::clock_time(10, 0) as u16;
    save.cutscene_index = 0xFFF1;
    if file_select.button_index == 0 {
        save.cutscene_index = 0;
    }
    save.player_name = file_select.file_names[file_select.button_index as usize];
    save.newf = *b"ZELDAZ";
    save.n64dd_flag = file_select.n64dd_flag as i16;
    save.checksum = checksum(&save.save_bytes());
    let bytes = save.save_bytes();
    let file = save.file_num as usize;
    for slot in [file, file + 3] {
        let offset = SRAM_SLOT_OFFSETS[slot] as usize;
        sram_ctx.read_buff[offset..offset + SAVE_SIZE].copy_from_slice(&bytes);
    }
    sram.write(0, &sram_ctx.read_buff);
    // "SAVE end"
    file_select.read_slot(&sram_ctx.read_buff, file, SRAM_SLOT_OFFSETS[file] as usize);
}

/// `Sram_EraseSave`: a new save's info over the selected file and its backup (the slot written
/// whole from the save, its checksum `sNewSaveChecksum`'s 0).
pub fn sram_erase_save(file_select: &mut FileSelectState, save: &mut SaveContext, sram_ctx: &mut SramContext, sram: &mut Sram) {
    save.init_new_save();
    let file = file_select.selected_file_index as usize;
    for slot in [file, file + 3] {
        let offset = SRAM_SLOT_OFFSETS[slot] as usize;
        sram_ctx.read_buff[offset..offset + SAVE_SIZE].copy_from_slice(&save.save_bytes());
        sram.write(offset, &save.slot_bytes());
        if slot == file {
            file_select.n64dd_flags[file] = be16(&sram_ctx.read_buff, offset + N64DD) as i16;
        }
    }
    // "CLEAR END"
}

/// `Sram_CopySave`: the selected file into the save, then over the destination file and its
/// backup, with the whole SRAM; the file select's fields of the destination. (The bytes are
/// copied as read: the save is what `MemCopy` makes of them.)
pub fn sram_copy_save(file_select: &mut FileSelectState, save: &mut SaveContext, sram_ctx: &mut SramContext, sram: &mut Sram) {
    let src = SRAM_SLOT_OFFSETS[file_select.selected_file_index as usize] as usize;
    let bytes = sram_ctx.read_buff[src..src + SAVE_SIZE].to_vec();
    save.read_save(&bytes);
    let dest = file_select.copy_dest_file_index as usize;
    for slot in [dest, dest + 3] {
        let offset = SRAM_SLOT_OFFSETS[slot] as usize;
        sram_ctx.read_buff[offset..offset + SAVE_SIZE].copy_from_slice(&bytes);
    }
    sram.write(0, &sram_ctx.read_buff);
    file_select.read_slot(&sram_ctx.read_buff, dest, SRAM_SLOT_OFFSETS[dest] as usize);
    // "Copy end"
}

/// `Sram_WriteSramHeader`: the read buffer's first 16 bytes to the header.
pub fn sram_write_sram_header(sram_ctx: &SramContext, sram: &mut Sram) {
    sram.write(0, &sram_ctx.read_buff[..SRAM_HEADER_SIZE]);
}

/// `Sram_InitSram` (the title screen's): the SRAM read; a header whose magic is wrong rewritten
/// (`sSramDefaultHeader`, keeping the language: `PLATFORM_GC && OOT_PAL`); the sound and
/// Z-targeting settings and the language read (a language past `LANGUAGE_MAX` made English and
/// written). With `DEBUG_FEATURES`, D-Right held on controller 3 (`dright_on_controller_3`; the
/// port has one controller) fills the SRAM with a ramp, "SRAM destruction". The caller then does
/// `Audio_SetSoundOutputMode(soundSetting)`.
pub fn sram_init_sram(save: &mut SaveContext, sram_ctx: &mut SramContext, sram: &mut Sram, dright_on_controller_3: bool) {
    sram.read(0, &mut sram_ctx.read_buff);
    for i in 0..SRAM_DEFAULT_HEADER.len() - SRAM_HEADER_MAGIC {
        if SRAM_DEFAULT_HEADER[i + SRAM_HEADER_MAGIC] != sram_ctx.read_buff[i + SRAM_HEADER_MAGIC] {
            // "SRAM destruction!!!!!!": the rest of the loop then finds the header it wrote.
            save.language = sram_ctx.read_buff[SRAM_HEADER_LANGUAGE];
            sram_ctx.read_buff[..SRAM_DEFAULT_HEADER.len()].copy_from_slice(&SRAM_DEFAULT_HEADER);
            sram_ctx.read_buff[SRAM_HEADER_LANGUAGE] = save.language;
            sram_write_sram_header(sram_ctx, sram);
        }
    }
    save.sound_setting = sram_ctx.read_buff[SRAM_HEADER_SOUND] & 3;
    save.z_target_setting = sram_ctx.read_buff[SRAM_HEADER_Z_TARGET] & 1;
    save.language = sram_ctx.read_buff[SRAM_HEADER_LANGUAGE];
    if save.language >= LANGUAGE_MAX {
        save.language = LANGUAGE_ENG;
        sram_ctx.read_buff[SRAM_HEADER_LANGUAGE] = save.language;
        sram_write_sram_header(sram_ctx, sram);
    }
    if dright_on_controller_3 {
        sram_ctx.read_buff.fill(0);
        for i in 0..CHECKSUM_SIZE {
            sram_ctx.read_buff[i] = i as u8;
        }
        sram.write(0, &sram_ctx.read_buff);
        // "SRAM destruction!!!!!!"
    }
    if save.language != LANGUAGE_ENG || save.sound_setting != 0 || save.z_target_setting != 0 {
        log::warn!(
            "Sram_InitSram: the header's language {}, sound {} and Z-targeting {}: the port plays in English, in stereo, with \"Switch\" targeting",
            save.language,
            save.sound_setting,
            save.z_target_setting
        );
    }
}

#[cfg(test)]
mod tests;
