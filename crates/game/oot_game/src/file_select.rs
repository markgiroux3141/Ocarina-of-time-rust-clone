//! The file select's load, stood in for (docs/adr/0049-saving.md). The title screen
//! (`z_title.c`), the file select (`z_file_choose.c`, `z_file_nameset.c`) and the map select
//! (`z_select.c`) aren't ported; what they do to the save is, around `crate::sram`:
//! - the title's `Sram_InitSram`, then the file select's `Sram_Alloc` and
//!   `Sram_VerifyAndLoadAllSaves` ([`title_and_file_select`]);
//! - a file picked: `FileSelect_LoadGame` ([`file_select_load_game`]); in this debug ROM file 1
//!   then goes to the map select, whose pick is `MapSelect_LoadGame` ([`map_select_load_game`]);
//! - a new file named: the name entry's `Sram_InitSave` ([`new_game`]).
//!
//! The game's and the sandbox's `--file N` and the console's reset (F5) go through
//! [`load_game`], `--file N --new-file` through [`new_game`].

use crate::save::*;
use crate::sram::*;

/// Why a file can't be loaded or made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileError {
    /// Only files 1 to 3 exist.
    NoSuchFile(usize),
    /// The file is empty (`SLOT_OCCUPIED`): the file select would make a new one.
    Empty(usize),
    /// The file holds a save: the file select only names new files in empty ones.
    Occupied(usize),
    /// File 1 goes through the map select in this debug ROM: it needs an entrance.
    NeedsEntrance,
}

impl std::fmt::Display for FileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileError::NoSuchFile(n) => write!(f, "there is no file {n}: the files are 1, 2 and 3"),
            FileError::Empty(n) => write!(f, "file {n} is empty: make it with --new-file"),
            FileError::Occupied(n) => write!(f, "file {n} holds a save: a new file needs an empty one (ootx sram erase {n})"),
            FileError::NeedsEntrance => write!(f, "file 1 goes through this debug ROM's map select: it needs --entrance"),
        }
    }
}

impl std::error::Error for FileError {}

/// The name the stand-in's name entry gives a new file: "LINK" (`FILENAME_UPPERCASE`).
pub const NEW_FILE_NAME: [u8; 8] = [0x15, 0x12, 0x17, 0x14, FILENAME_SPACE, FILENAME_SPACE, FILENAME_SPACE, FILENAME_SPACE];

/// `HUD_VISIBILITY_ALL`, `NEXT_CS_INDEX_NONE` (`save.h`).
const HUD_VISIBILITY_ALL: u16 = 50;
const NEXT_CS_INDEX_NONE: u16 = 0xFFEF;

/// The title screen and the file select's start on `sram`: the save as `SaveContext_Init`
/// leaves it (the title's opening isn't ported), `Sram_InitSram`, `Sram_Alloc` and
/// `Sram_VerifyAndLoadAllSaves`.
pub fn title_and_file_select(sram: &mut Sram) -> (SaveContext, SramContext, FileSelectState) {
    let mut save = SaveContext::blank(0, false, 0);
    let mut sram_ctx = SramContext::sram_alloc();
    sram_init_sram(&mut save, &mut sram_ctx, sram, false);
    let mut file_select = FileSelectState::default();
    sram_verify_and_load_all_saves(&mut file_select, &mut save, &mut sram_ctx, sram);
    (save, sram_ctx, file_select)
}

fn check_file(file: usize, map_select_entrance: Option<u16>) -> Result<(), FileError> {
    if !(1..=3).contains(&file) {
        return Err(FileError::NoSuchFile(file));
    }
    if file == 1 && map_select_entrance.is_none() {
        return Err(FileError::NeedsEntrance);
    }
    Ok(())
}

/// File `file` (1 to 3) loaded from `sram` as the file select loads it, file 1 then entering by
/// `map_select_entrance` as the map select does.
pub fn load_game(sram: &mut Sram, file: usize, map_select_entrance: Option<u16>) -> Result<SaveContext, FileError> {
    check_file(file, map_select_entrance)?;
    let (mut save, sram_ctx, _) = title_and_file_select(sram);
    if !slot_occupied(&sram_ctx.read_buff, file - 1) {
        return Err(FileError::Empty(file));
    }
    file_select_load_game(&mut save, &sram_ctx, file - 1);
    if let Some(entrance) = map_select_entrance.filter(|_| file == 1) {
        map_select_load_game(&mut save, entrance);
    }
    Ok(save)
}

/// A new file in the empty file `file` (1 to 3) as the name entry makes it (named "LINK":
/// `gSaveContext.fileNum`, then `Sram_InitSave` with `dayTime` kept round it), then loaded as
/// [`load_game`] loads it.
pub fn new_game(sram: &mut Sram, file: usize, map_select_entrance: Option<u16>) -> Result<SaveContext, FileError> {
    check_file(file, map_select_entrance)?;
    let (mut save, mut sram_ctx, mut file_select) = title_and_file_select(sram);
    if slot_occupied(&sram_ctx.read_buff, file - 1) {
        return Err(FileError::Occupied(file));
    }
    file_select.button_index = (file - 1) as i16;
    file_select.file_names[file - 1] = NEW_FILE_NAME;
    save.file_num = (file - 1) as i32;
    let day_time = save.day_time;
    sram_init_save(&mut file_select, &mut save, &mut sram_ctx, sram);
    save.day_time = day_time;
    file_select_load_game(&mut save, &sram_ctx, file - 1);
    if let Some(entrance) = map_select_entrance.filter(|_| file == 1) {
        map_select_load_game(&mut save, entrance);
    }
    Ok(save)
}

/// `FileSelect_LoadGame` (`z_file_choose.c`): `fileNum`, `Sram_OpenSave`, `GAMEMODE_NORMAL`
/// (file 1, `DEBUG_FEATURES`: on to the map select; the others to `Play_Init`), then the
/// resets: the respawn, the music, the title card, the event flags, the HUD, the transition and
/// cutscene, every button enabled, the magic to grow back, `naviTimer`, and B's sword.
pub fn file_select_load_game(save: &mut SaveContext, sram_ctx: &SramContext, button_index: usize) {
    // (NA_SE_SY_FSEL_DECIDE_L: the file select's.)
    save.file_num = button_index as i32;
    sram_open_save(save, sram_ctx);
    save.game_mode = GAMEMODE_NORMAL;
    save.respawn[RESPAWN_MODE_DOWN].entrance_index = ENTR_LOAD_OPENING;
    save.respawn_flag = 0;
    save.seq_id = crate::audio::NA_BGM_DISABLED as u8;
    save.nature_ambience_id = crate::audio::NATURE_ID_DISABLED;
    save.show_title_card = true;
    // (dogParams, timerState, subTimerState: not ported.)
    save.event_inf = [0; 4];
    save.prev_hud_visibility_mode = HUD_VISIBILITY_ALL;
    // (nayrusLoveTimer: not ported.)
    save.health_accumulator = 0;
    // (magicState, prevMagicState: the meter isn't ported.)
    save.forced_seq_id = crate::audio::NA_BGM_GENERAL_SFX;
    // (skyboxTime: not ported.)
    save.next_transition_type = TRANS_NEXT_TYPE_DEFAULT;
    save.next_cutscene_index = NEXT_CS_INDEX_NONE;
    save.cutscene_trigger = 0;
    // (chamberCutsceneNum, nextDayTime: not ported.)
    save.retain_weather_mode = false;
    save.button_status = [crate::interface::BTN_ENABLED; 5];
    save.force_rising_button_alphas = 0;
    save.next_hud_visibility_mode = 0;
    save.hud_visibility_mode = 0;
    save.hud_visibility_mode_timer = 0;
    // (magicCapacity 0, magicFillTarget the saved magic: the meter grows back to it.)
    save.magic_level = 0;
    save.magic = 0;
    save.navi_timer = 0;
    if !matches!(save.equips.button_items[0], crate::item::ITEM_SWORD_KOKIRI | crate::item::ITEM_SWORD_MASTER | crate::item::ITEM_SWORD_BIGGORON | crate::item::ITEM_GIANTS_KNIFE) {
        save.equips.button_items[0] = crate::item::ITEM_NONE;
        let sword_equip_value = (crate::item::EQUIP_MASKS[crate::item::EQUIP_TYPE_SWORD] & save.equips.equipment) >> (crate::item::EQUIP_TYPE_SWORD * 4);
        save.equips.equipment &= crate::item::EQUIP_NEG_MASKS[crate::item::EQUIP_TYPE_SWORD];
        // @bug (game): swordEquipValue is 0 with no sword worn (a new file's), and
        // OWNED_EQUIP_FLAG(EQUIP_TYPE_SWORD, -1) reads gBitFlags[-1], the word before the table in
        // `code`: 0 in this ROM (gBitFlags at ROM 0xB9E2C0), and nothing changes.
        if sword_equip_value != 0 {
            save.inventory.equipment ^= crate::item::owned_equip_flag(crate::item::EQUIP_TYPE_SWORD, sword_equip_value - 1);
        }
    }
}

/// `MapSelect_LoadGame` (`z_select.c`) after file 1's load: the save kept (the map select's own
/// file, `fileNum` 0xFF, is `SaveContext::debug`), every button enabled, the HUD's alpha types
/// cleared, the entrance picked, the respawn and the music reset, the title card shown.
pub fn map_select_load_game(save: &mut SaveContext, entrance_index: u16) {
    if save.file_num == 0xFF {
        save.init_debug_save();
        save.magic_level = 0;
        save.magic = 0;
    }
    save.button_status = [crate::interface::BTN_ENABLED; 5];
    save.force_rising_button_alphas = 0;
    save.next_hud_visibility_mode = 0;
    save.hud_visibility_mode = 0;
    save.hud_visibility_mode_timer = 0;
    // (SEQCMD_STOP_SEQUENCE(SEQ_PLAYER_BGM_MAIN, 0): the map select's music; gWeatherMode
    // WEATHER_MODE_CLEAR: a new play state's.)
    save.entrance_index = entrance_index;
    save.respawn_flag = 0;
    save.respawn[RESPAWN_MODE_DOWN].entrance_index = ENTR_LOAD_OPENING;
    save.seq_id = crate::audio::NA_BGM_DISABLED as u8;
    save.nature_ambience_id = crate::audio::NATURE_ID_DISABLED;
    save.show_title_card = true;
}
