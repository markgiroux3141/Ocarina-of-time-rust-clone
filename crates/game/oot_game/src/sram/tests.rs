//! `z_sram.c` against the C: the slots' places and sizes (`sram.h`, `save.h`), the save's layout
//! (`save.h`'s offsets), the checksum, and each function's effect on an SRAM image. The file
//! select's stand-ins (`crate::file_select`) are checked here too.

use super::*;
use crate::file_select::{self, FileError};

fn be16_at(b: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([b[at], b[at + 1]])
}

fn be32_at(b: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// A child's new save at Link's house (`SaveContext::new`, "LINK").
fn new_save() -> SaveContext {
    SaveContext::new(ENTR_LINKS_HOUSE_0, false, crate::env::clock_time(10, 0) as u16)
}

#[test]
fn the_slots_are_where_the_c_puts_them() {
    // sizeof(Save) 0x1354, sizeof(SaveContext) 0x1428 (save.h); SLOT_SIZE adds 0x28.
    assert_eq!((SAVE_SIZE, SAVE_CONTEXT_SIZE, SLOT_SIZE, CHECKSUM_SIZE), (0x1354, 0x1428, 0x1450, 0x9AA));
    // SLOT_OFFSET(i): SRAM_HEADER_SIZE + 0x10 + i * SLOT_SIZE.
    assert_eq!(SRAM_SLOT_OFFSETS, [0x20, 0x1470, 0x28C0, 0x3D10, 0x5160, 0x65B0]);
    // The last backup ends at 0x7A00, inside the 32 KiB.
    assert_eq!(slot_offset(5) + SLOT_SIZE, 0x7A00);
    assert!(slot_offset(6) > SRAM_SIZE - SLOT_SIZE);
}

#[test]
fn a_new_save_is_laid_out_as_save_h() {
    let b = new_save().save_bytes();
    assert_eq!(b.len(), SAVE_SIZE);
    // Save: entranceIndex, linkAge (1 the child), cutsceneIndex, dayTime (10:00), nightFlag.
    assert_eq!((be32_at(&b, 0x00), be32_at(&b, 0x04), be32_at(&b, 0x08)), (ENTR_LINKS_HOUSE_0 as u32, 1, 0));
    assert_eq!(be16_at(&b, 0x0C), crate::env::clock_time(10, 0) as u16);
    // playerData: no newf, "LINK", three hearts (healthCapacity 0x2E, health 0x30), the normal
    // meter (magic 0x33), Link's house (savedSceneId 0x66), both ages' equips empty (0x40, 0x4A).
    assert_eq!(&b[NEWF..NEWF + 6], &[0; 6]);
    assert_eq!(&b[NAME..NAME + 8], &[0x15, 0x12, 0x17, 0x14, 0x3E, 0x3E, 0x3E, 0x3E]);
    assert_eq!((be16_at(&b, HEALTH_CAP), be16_at(&b, HEALTH), b[0x33]), (0x30, 0x30, 0x30));
    assert_eq!(be16_at(&b, 0x66), SCENE_LINKS_HOUSE);
    assert_eq!(&b[0x40..0x48], &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00]);
    // equips (0x68): nothing on the buttons, the Kokiri tunic and boots worn (0x1100 at 0x70).
    assert_eq!(&b[0x68..0x70], &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00]);
    assert_eq!(be16_at(&b, 0x70), 0x1100);
    // inventory (0x74): no items, the Kokiri tunic and boots owned (equipment at 0x9C), no keys
    // (dungeonKeys at 0xBC: -1).
    assert!(b[0x74..0x8C].iter().all(|&x| x == 0xFF));
    assert_eq!(be16_at(&b, 0x9C), 0x1100);
    assert!(b[0xBC..0xCF].iter().all(|&x| x == 0xFF));
    // sceneFlags (0xD4, 0x1C each): the Water Temple's (5) swch 0x40000000.
    assert_eq!(be32_at(&b, 0xD4 + 5 * 0x1C + 4), 0x4000_0000);
    // infTable (0xEF8): [INFTABLE_INDEX_1DX] 1.
    assert_eq!(be16_at(&b, 0xEF8 + 2 * INFTABLE_INDEX_1DX), 1);
    // horseData (0x1348): Hyrule Field, (-1840, 72, 5497), -0x6AD9.
    let horse: Vec<i16> = (0..5).map(|i| be16_at(&b, 0x1348 + i * 2) as i16).collect();
    assert_eq!(horse, [0x51, -1840, 72, 5497, -0x6AD9]);
    // The file select's QUEST and DEFENSE offsets land on questItems and defenseHearts.
    assert_eq!((QUEST, DEFENSE), (0x74 + 0x30, 0x74 + 0x5B));
}

#[test]
fn clearing_the_info_zeroes_every_byte_after_the_save_words() {
    let mut s = SaveContext::debug(0, false, 0);
    s.scarecrow_long_song = [7; 0x360];
    s.high_scores = [3; 7];
    s.fw.set = 1;
    s.clear_info();
    let b = s.save_bytes();
    // bzero(&gSaveContext.save.info, sizeof(SaveInfo)): from 0x1C to the end.
    assert!(b[0x1C..].iter().all(|&x| x == 0), "a SaveInfo field clear_info misses");
}

#[test]
fn a_save_reads_back_as_it_was_written() {
    for adult in [false, true] {
        let mut s = SaveContext::debug(0x0CD, adult, 0x8000);
        s.scarecrow_spawn_song_set = 1;
        s.scarecrow_spawn_song[3] = 9;
        s.total_days = 12;
        s.fw.pos = [1, -2, 3];
        s.unk_e8c[0] = 0xAB;
        let b = s.save_bytes();
        let mut t = SaveContext::new(0, !adult, 0);
        t.read_save(&b);
        assert_eq!(t.save_bytes(), b);
        assert_eq!((t.adult, t.entrance_index, t.day_time, t.total_days), (adult, 0x0CD, 0x8000, 12));
        assert_eq!((t.inventory, t.equips, t.scarecrow_spawn_song[3], t.fw.pos, t.unk_e8c[0]), (s.inventory, s.equips, 9, [1, -2, 3], 0xAB));
        assert_eq!(t.scene_flags, s.scene_flags);
    }
    // ENTR_LOAD_OPENING's -1 is an s32 -1.
    let mut s = new_save();
    s.entrance_index = ENTR_LOAD_OPENING;
    assert_eq!(be32_at(&s.save_bytes(), 0), 0xFFFF_FFFF);
}

#[test]
fn the_checksum_sums_the_saves_halfwords_but_its_own() {
    let mut b = vec![0u8; SAVE_SIZE];
    b[0] = 0x12;
    b[1] = 0x34;
    b[0x1350] = 0xFF;
    b[0x1351] = 0xFF;
    // The checksum's own halfword (0x1352) isn't counted.
    b[0x1352] = 0x55;
    // 0x1234 + 0xFFFF wraps to 0x1233.
    assert_eq!(checksum(&b), 0x1233);
    assert_eq!(checksum(&vec![0; SAVE_SIZE]), 0);
}

#[test]
fn write_save_puts_the_slot_and_its_backup_with_the_checksum() {
    let mut s = new_save();
    s.file_num = 1;
    s.health = 0x20;
    s.save_scene_flags_for_test(0x00, 0x8);
    let mut sram = Sram::default();
    sram_write_save(&mut s, &mut sram);
    assert_eq!(sram.writes, 2);
    let (a, b) = (SRAM_SLOT_OFFSETS[1] as usize, SRAM_SLOT_OFFSETS[4] as usize);
    assert_eq!(&sram.bytes[a..a + SLOT_SIZE], &sram.bytes[b..b + SLOT_SIZE]);
    let slot = &sram.bytes[a..a + SLOT_SIZE];
    assert_eq!(be16_at(slot, CHECKSUM_OFFSET), s.checksum);
    assert_eq!(checksum(slot), s.checksum);
    assert_eq!(be16_at(slot, HEALTH), 0x20);
    // The tail: fileNum (0x1354) 1, buttonStatus (0x13E2), language (0x1409); the 0x28 bytes
    // past SaveContext zeros.
    assert_eq!(be32_at(slot, 0x1354), 1);
    assert_eq!(&slot[0x13E2..0x13E7], &s.button_status);
    assert_eq!(slot[0x1409], LANGUAGE_ENG);
    assert!(slot[SAVE_CONTEXT_SIZE..].iter().all(|&x| x == 0));
    // Nothing else written.
    assert!(sram.bytes[..a].iter().all(|&x| x == 0));
    // The map select's file has no slot: nothing written.
    let mut d = SaveContext::debug(0, false, 0);
    let mut sram = Sram::default();
    sram_write_save(&mut d, &mut sram);
    assert_eq!(sram.writes, 0);
}

impl SaveContext {
    /// A scene's chest flags, as `Play_SaveSceneFlags` would leave them.
    fn save_scene_flags_for_test(&mut self, scene: usize, chest: u32) {
        self.scene_flags[scene].chest = chest;
    }
}

#[test]
fn a_fresh_sram_verifies_as_three_empty_files() {
    let mut sram = Sram::default();
    let mut ctx = SramContext::sram_alloc();
    let mut fs = FileSelectState::default();
    let mut s = SaveContext::new(0, false, 0x1234);
    sram_verify_and_load_all_saves(&mut fs, &mut s, &mut ctx, &mut sram);
    // Zeros sum to zero: every checksum holds, nothing is written.
    assert_eq!(sram.writes, 0);
    assert!((0..3).all(|i| !slot_occupied(&ctx.read_buff, i)));
    // The save holds file 3's zeros, with the dayTime it came in with.
    assert_eq!((s.health_capacity, s.day_time, s.adult), (0, 0x1234, true));
}

#[test]
fn bad_slots_and_backups_are_rebuilt_file_1_as_the_debug_save() {
    let mut sram = Sram { bytes: vec![0xFF; SRAM_SIZE], writes: 0 };
    let mut ctx = SramContext::sram_alloc();
    let mut fs = FileSelectState::default();
    let mut s = SaveContext::new(0, false, 0x4000);
    sram_verify_and_load_all_saves(&mut fs, &mut s, &mut ctx, &mut sram);
    // Each file and its backup written.
    assert_eq!(sram.writes, 6);
    for slot in 0..6 {
        let o = SRAM_SLOT_OFFSETS[slot] as usize;
        let b = &sram.bytes[o..o + SAVE_SIZE];
        assert_eq!(checksum(b), be16_at(b, CHECKSUM_OFFSET), "slot {slot}");
    }
    // File 1 the debug save ("ZELDAZ"); linkAge was cleared to 0, so it's the adult's: the
    // Master Sword on B. Files 2 and 3 a new save: empty (no newf).
    assert!(slot_occupied(&ctx.read_buff, 0));
    assert!(!slot_occupied(&ctx.read_buff, 1) && !slot_occupied(&ctx.read_buff, 2));
    let o = SRAM_SLOT_OFFSETS[0] as usize;
    let mut f1 = SaveContext::new(0, false, 0);
    f1.read_save(&ctx.read_buff[o..o + SAVE_SIZE]);
    assert_eq!((f1.adult, f1.equips.button_items[0], f1.health_capacity, f1.entrance_index), (true, ITEM_SWORD_MASTER, 0xE0, ENTR_HYRULE_FIELD_0));
    // The file select's fields: file 1's 14 hearts, files 2's and 3's three.
    assert_eq!(fs.health_capacities, [0xE0, 0x30, 0x30]);
    assert_eq!(fs.file_names[0], [0x15, 0x12, 0x17, 0x14, 0x3E, 0x3E, 0x3E, 0x3E]);
    assert_eq!(s.day_time, 0x4000);
}

#[test]
fn a_bad_file_with_a_good_backup_is_restored_without_its_checksum() {
    let mut s = new_save();
    s.file_num = 2;
    s.newf = *b"ZELDAZ";
    s.rupees = 77;
    let mut sram = Sram::default();
    sram_write_save(&mut s, &mut sram);
    let main = SRAM_SLOT_OFFSETS[2] as usize;
    // A bit flips in file 3.
    sram.bytes[main + 0x34] ^= 0x01;
    let mut ctx = SramContext::sram_alloc();
    let mut fs = FileSelectState::default();
    let mut t = SaveContext::new(0, false, 0);
    sram_verify_and_load_all_saves(&mut fs, &mut t, &mut ctx, &mut sram);
    // The backup's bytes back in the file, but with the checksum the check cleared (@bug).
    assert_eq!(be16_at(&sram.bytes, main + 0x34), 77);
    assert_eq!(be16_at(&sram.bytes, main + CHECKSUM_OFFSET), 0);
    assert_eq!(t.rupees, 77);
    // Loading it works; the next save puts the checksum right.
    let mut u = file_select::load_game(&mut sram, 3, None).unwrap();
    assert_eq!(u.rupees, 77);
    sram_write_save(&mut u, &mut sram);
    let b = &sram.bytes[main..main + SAVE_SIZE];
    assert_eq!(checksum(b), be16_at(b, CHECKSUM_OFFSET));
}

#[test]
fn open_save_enters_by_the_saved_scene() {
    let entrance = |scene: u16, adult: bool| {
        let mut s = new_save();
        s.adult = adult;
        s.saved_scene_id = scene;
        s.file_num = 1;
        let mut ctx = SramContext::sram_alloc();
        let o = SRAM_SLOT_OFFSETS[1] as usize;
        ctx.read_buff[o..o + SAVE_SIZE].copy_from_slice(&s.save_bytes());
        let mut t = SaveContext::new(0, false, 0);
        t.file_num = 1;
        sram_open_save(&mut t, &ctx);
        t.entrance_index
    };
    // Dungeons by sDungeonEntrances; the boss rooms by their dungeon's; the collapse, Ganondorf
    // and Ganon by Ganon's tower; elsewhere Link's house for a child, the Temple of Time for an
    // adult; Link's house for both.
    assert_eq!(entrance(0x00, false), ENTR_DEKU_TREE_0);
    assert_eq!(entrance(0x03, true), 0x169);
    assert_eq!(entrance(0x0D, true), 0x467);
    assert_eq!(entrance(0x11, false), ENTR_DEKU_TREE_0);
    assert_eq!(entrance(0x18, true), 0x037);
    assert_eq!(entrance(0x0E, true), 0x41B);
    assert_eq!(entrance(0x19, true), 0x41B);
    assert_eq!(entrance(0x4F, true), 0x41B);
    assert_eq!(entrance(0x55, false), ENTR_LINKS_HOUSE_0);
    assert_eq!(entrance(0x55, true), 0x5F4);
    assert_eq!(entrance(SCENE_LINKS_HOUSE, true), ENTR_LINKS_HOUSE_0);
}

#[test]
fn open_save_puts_right_what_the_c_puts_right() {
    let mut s = new_save();
    s.file_num = 0;
    s.adult = true;
    s.health = 4;
    s.magic_level = 1;
    // Zelda's letter without the lullaby, on C-Down.
    s.set_event_chk_inf(EVENTCHKINF_OBTAINED_ZELDAS_LETTER);
    s.inventory.items[SLOT_TRADE_CHILD] = ITEM_ZELDAS_LETTER;
    s.equips.button_items[2] = ITEM_ZELDAS_LETTER;
    // The odd mushroom (0x30) in the adult's trade slot, on C-Right.
    s.inventory.items[SLOT_TRADE_ADULT] = 0x30;
    s.equips.button_items[3] = 0x30;
    let mut ctx = SramContext::sram_alloc();
    let o = SRAM_SLOT_OFFSETS[0] as usize;
    ctx.read_buff[o..o + SAVE_SIZE].copy_from_slice(&s.save_bytes());
    let mut t = SaveContext::new(0, false, 0);
    t.file_num = 0;
    sram_open_save(&mut t, &ctx);
    assert_eq!((t.health, t.magic_level), (0x30, 0));
    assert!(!t.get_event_chk_inf(EVENTCHKINF_OBTAINED_ZELDAS_LETTER));
    assert_eq!((t.inventory.items[SLOT_TRADE_CHILD], t.equips.button_items[2]), (ITEM_CHICKEN, ITEM_CHICKEN));
    // gSpoilingItemReverts: Cojiro (0x2F).
    assert_eq!((t.inventory.items[SLOT_TRADE_ADULT], t.equips.button_items[3]), (0x2F, 0x2F));
    // The adult without the Master Sword gets it, on B (OOT_VERSION >= NTSC_1_1).
    assert!(t.check_owned_equip(EQUIP_TYPE_SWORD, EQUIP_INV_SWORD_MASTER));
    assert_eq!((t.equips.button_items[0], t.cur_equip_value(EQUIP_TYPE_SWORD)), (ITEM_SWORD_MASTER, EQUIP_VALUE_SWORD_MASTER));
}

#[test]
fn init_save_names_a_new_file_and_writes_it_whole() {
    let mut sram = Sram::default();
    let (mut s, mut ctx, mut fs) = file_select::title_and_file_select(&mut sram);
    // Sram_InitSram wrote the header: the magic, stereo, Switch, English.
    assert_eq!(&sram.bytes[..12], &SRAM_DEFAULT_HEADER);
    let writes = sram.writes;
    fs.button_index = 2;
    fs.file_names[2] = [0x0A; 8];
    fs.n64dd_flag = 1;
    s.file_num = 2;
    sram_init_save(&mut fs, &mut s, &mut ctx, &mut sram);
    // One write: the whole SRAM.
    assert_eq!(sram.writes, writes + 1);
    assert!(slot_occupied(&sram.bytes, 2));
    let o = SRAM_SLOT_OFFSETS[2] as usize;
    let b = &sram.bytes[o..o + SAVE_SIZE];
    assert_eq!(&b[NEWF..NEWF + 6], b"ZELDAZ");
    assert_eq!(&b[NAME..NAME + 8], &[0x0A; 8]);
    assert_eq!((be32_at(b, 0), be32_at(b, 4), be32_at(b, 8)), (ENTR_LINKS_HOUSE_0 as u32, 1, 0xFFF1));
    assert_eq!((be16_at(b, N64DD), checksum(b)), (1, be16_at(b, CHECKSUM_OFFSET)));
    // The backup the same; the tail untouched (only sizeof(Save) goes in the buffer).
    let k = SRAM_SLOT_OFFSETS[5] as usize;
    assert_eq!(&sram.bytes[k..k + SAVE_SIZE], b);
    assert!(sram.bytes[o + SAVE_SIZE..o + SLOT_SIZE].iter().all(|&x| x == 0));
    assert_eq!((fs.health_capacities[2], fs.file_names[2]), (0x30, [0x0A; 8]));
    // File 1 is the debug save, without the opening.
    fs.button_index = 0;
    s.file_num = 0;
    sram_init_save(&mut fs, &mut s, &mut ctx, &mut sram);
    let b = &sram.bytes[0x20..0x20 + SAVE_SIZE];
    assert_eq!((be16_at(b, HEALTH_CAP), be32_at(b, 8)), (0xE0, 0));
}

#[test]
fn erase_leaves_a_new_save_whose_checksum_fails_next_time() {
    let mut sram = Sram::default();
    file_select::new_game(&mut sram, 2, None).unwrap();
    let (mut s, mut ctx, mut fs) = file_select::title_and_file_select(&mut sram);
    assert!(slot_occupied(&ctx.read_buff, 1));
    fs.selected_file_index = 1;
    sram_erase_save(&mut fs, &mut s, &mut ctx, &mut sram);
    // Sram_InitNewSave's info (sNewSaveChecksum 0) over the file and its backup.
    let o = SRAM_SLOT_OFFSETS[1] as usize;
    assert_eq!((be16_at(&sram.bytes, o + CHECKSUM_OFFSET), be16_at(&sram.bytes, o + HEALTH_CAP)), (0, 0x30));
    assert!(!slot_occupied(&sram.bytes, 1));
    // The next boot finds both bad and makes them a new save, checksummed.
    let before = sram.writes;
    file_select::title_and_file_select(&mut sram);
    assert_eq!(sram.writes, before + 2);
    let b = &sram.bytes[o..o + SAVE_SIZE];
    assert_eq!(checksum(b), be16_at(b, CHECKSUM_OFFSET));
}

#[test]
fn copy_puts_one_file_over_another_and_its_backup() {
    let mut sram = Sram::default();
    let mut s = file_select::new_game(&mut sram, 2, None).unwrap();
    s.rupees = 99;
    sram_write_save(&mut s, &mut sram);
    let (mut t, mut ctx, mut fs) = file_select::title_and_file_select(&mut sram);
    fs.selected_file_index = 1;
    fs.copy_dest_file_index = 2;
    sram_copy_save(&mut fs, &mut t, &mut ctx, &mut sram);
    let (a, b, k) = (SRAM_SLOT_OFFSETS[1] as usize, SRAM_SLOT_OFFSETS[2] as usize, SRAM_SLOT_OFFSETS[5] as usize);
    assert_eq!(&sram.bytes[a..a + SAVE_SIZE], &sram.bytes[b..b + SAVE_SIZE]);
    assert_eq!(&sram.bytes[a..a + SAVE_SIZE], &sram.bytes[k..k + SAVE_SIZE]);
    assert_eq!(t.rupees, 99);
    assert_eq!(fs.file_names[2], file_select::NEW_FILE_NAME);
    // File 3 loads; it still says fileNum 1 (the copy's), which the load replaces.
    let u = file_select::load_game(&mut sram, 3, None).unwrap();
    assert_eq!((u.rupees, u.file_num), (99, 2));
}

#[test]
fn init_sram_checks_the_header() {
    // A header with a wrong magic is rewritten, keeping its language byte; the settings are
    // masked (sound & 3, Z-targeting & 1).
    let mut sram = Sram::default();
    sram.bytes[..3].copy_from_slice(&[0xFF, 0xFF, 2]);
    let mut ctx = SramContext::sram_alloc();
    let mut s = SaveContext::new(0, false, 0);
    sram_init_sram(&mut s, &mut ctx, &mut sram, false);
    assert_eq!(&sram.bytes[..12], &[0, 0, 2, 0x98, 0x09, 0x10, 0x21, b'Z', b'E', b'L', b'D', b'A']);
    assert_eq!((s.sound_setting, s.z_target_setting, s.language), (0, 0, 2));
    assert_eq!(sram.writes, 1);
    // A good header is kept; a language past LANGUAGE_MAX made English and written.
    sram.bytes[..3].copy_from_slice(&[0xFF, 0xFF, 7]);
    sram_init_sram(&mut s, &mut ctx, &mut sram, false);
    assert_eq!((s.sound_setting, s.z_target_setting, s.language, sram.bytes[2], sram.bytes[0]), (3, 1, LANGUAGE_ENG, LANGUAGE_ENG, 0xFF));
    assert_eq!(sram.writes, 2);
    // D-Right on controller 3: the ramp over CHECKSUM_SIZE bytes, zeros after.
    sram_init_sram(&mut s, &mut ctx, &mut sram, true);
    assert_eq!(&sram.bytes[..4], &[0, 1, 2, 3]);
    assert_eq!((sram.bytes[0x100], sram.bytes[CHECKSUM_SIZE - 1], sram.bytes[CHECKSUM_SIZE]), (0, (CHECKSUM_SIZE - 1) as u8, 0));
}

#[test]
fn a_slot_is_occupied_by_any_letter_of_zeldaz() {
    let mut b = vec![0u8; SRAM_SIZE];
    assert!(!slot_occupied(&b, 0));
    // SLOT_OCCUPIED ors the six letters: an 'E' in newf[1] alone will do.
    b[0x20 + NEWF + 1] = b'E';
    assert!(slot_occupied(&b, 0));
    assert!(!slot_occupied(&b, 1));
}

#[test]
fn the_stand_ins_load_and_make_files_as_the_file_select_does() {
    let mut sram = Sram::default();
    assert_eq!(file_select::load_game(&mut sram, 2, None), Err(FileError::Empty(2)));
    assert_eq!(file_select::load_game(&mut sram, 4, None), Err(FileError::NoSuchFile(4)));
    assert_eq!(file_select::load_game(&mut sram, 1, None), Err(FileError::NeedsEntrance));
    let s = file_select::new_game(&mut sram, 2, None).unwrap();
    assert_eq!(file_select::new_game(&mut sram, 2, None), Err(FileError::Occupied(2)));
    // The new file starts the opening at Link's house, as SaveContext::file_select_new does.
    assert_eq!((s.entrance_index, s.cutscene_index, s.adult, s.file_num), (ENTR_LINKS_HOUSE_0, 0xFFF1, false, 1));
    assert_eq!(s, SaveContext::file_select_new());
    // Loaded again: the same.
    let t = file_select::load_game(&mut sram, 2, None).unwrap();
    assert_eq!(t, s);
    // File 1 through the map select: the save kept, the entrance picked.
    let u = file_select::new_game(&mut sram, 1, Some(ENTR_DEKU_TREE_0)).unwrap();
    assert_eq!((u.entrance_index, u.health_capacity, u.file_num, u.respawn[RESPAWN_MODE_DOWN].entrance_index), (ENTR_DEKU_TREE_0, 0xE0, 0, ENTR_LOAD_OPENING));
}
