# 0049: Saving: `z_sram.c` whole on an SRAM image, the pause menu's save prompt, and the file select's load stood in for

- **Status:** accepted, built in GAME-05 milestone 5c (2026-10-09)
- **Date:** 2026-10-09
- **Builds on:** [ADR 0019](0019-inventory-and-saves.md) (`SaveContext::new` is
  `Sram_InitNewSave`), [ADR 0032](0032-damage-death-and-the-game-over-stand-in.md) (the game over's
  states), [ADR 0047](0047-the-pause-menu.md) and [ADR 0048](0048-the-pause-map-and-the-game-over.md)
  (the pause menu's recorder and the prompt page).

## Context

Milestone 5c (agreed 2026-10-09, one milestone) ports `z_sram.c`'s save and load whole, the slots
in the C's layout, the pause menu's save prompt (B in `PAUSE_STATE_MAIN`,
`PAUSE_STATE_SAVE_PROMPT`, `KaleidoScope_DrawPages`' save page), the game over's
`Sram_WriteSave`, and a stand-in for the file select's load. Several things needed a decision:

- **Where the save lives.** The cartridge's SRAM is 32 KiB: a 16-byte header, then six slots of
  `SLOT_SIZE` (0x1450) bytes, the three files and their backups. A save file is user data: it
  must stay out of the repo, and out of the tests and goldens.
- **The port's `SaveContext`** is flat where the C nests `Save`, `SaveInfo` and `SavePlayerData`,
  holds some fields with other types (`bool`s, `u16` entrances), and lacked the parts of `Save`
  nothing ported reads (Farore's Wind, the high scores, the scarecrow's songs, Epona, the
  checksum and the padding). A slot is `SLOT_SIZE` bytes from `&gSaveContext`: `Save`, the rest of
  `SaveContext`, and 0x28 bytes of whatever follows it in RAM.
- **Loading.** The title screen and the file select (`z_file_choose.c`) aren't ported. In this
  debug ROM file 1 goes through the map select after its load.
- **The debug starts and the sandbox's runs** shouldn't write to the user's file.
- **The file select's other screens** (erase, copy) call `Sram_EraseSave` and `Sram_CopySave`.

## Decision

- **`oot_game::sram` is `z_sram.c`** for this ROM's branches, function by function:
  `Sram_InitNewSave` and `Sram_InitDebugSave` (`SaveContext::init_new_save`, `init_debug_save`,
  ported in place, which `SaveContext::new` and `debug` now build on), `Sram_OpenSave`,
  `Sram_WriteSave`, `Sram_VerifyAndLoadAllSaves`, `Sram_InitSave`, `Sram_EraseSave`,
  `Sram_CopySave`, `Sram_WriteSramHeader`, `Sram_InitSram`, `Sram_Alloc`, with
  `gSramSlotOffsets`, `sSramDefaultHeader`, `sDungeonEntrances`, `gSpoilingItems`. The iQue's
  path isn't this ROM.
- **The SRAM is an image** (`sram::Sram`: the 32 KiB and a count of the writes, standing in for
  `SsSram_ReadWrite`) on the play state (`PlayState::sram`), carried across `Play_Init` with the
  save. **`oot_game` never touches the disk** (the user's choice): the apps bind the image to a
  file, written whole after each write (`eng_asset::write_file_atomic`: beside it, then renamed).
- **The save file is `out\saves\<ROM SHA-1>.sra`** (the user's choice: the repo's ignored `out`,
  named by the ROM as the packs are; `oot_game::pack::save_path`), or under `$OOT_SAVE_DIR`, which
  `_env.bat` sets to the repo's `out\saves`. It holds the cartridge's bytes, big-endian, the header
  and the six slots where the C puts them.
- **`SaveContext` holds every field of `Save`** with the C's names (the user's choice), and
  `SaveContext::save_bytes`, `slot_bytes` and `read_save` lay it out field by field, checking the
  offsets in `save.h`'s comments as they go (one visitor for reading and writing, so the two
  can't drift). The port's `bool`s are the C's `u8`s and `s32`s (written 0 or 1, read as nonzero),
  its `u16` entrances the C's `s32`s and `s16`s (sign-extended: -1 stays -1); a value the C would
  only hold through a bug doesn't survive a round trip. **The slot's tail** is written from the
  fields of `SaveContext`'s play part the port has, zeros where it has none (the timers, the
  magic meter's state, the minigames, the dog, `nextDayTime`, `skyboxTime` ...) and for the 0x28
  bytes past the struct; nothing reads it back.
- **The file select's load is stood in for** (`oot_game::file_select`): the title's
  `Sram_InitSram`, then `Sram_Alloc` and `Sram_VerifyAndLoadAllSaves` on a save as
  `SaveContext_Init` leaves it; `FileSelect_LoadGame` ported (with its sword `@bug`); file 1 then
  through `MapSelect_LoadGame`, whose pick is an entrance; a new file as the name entry makes it,
  named "LINK" (`gSaveContext.fileNum`, then `Sram_InitSave` with `dayTime` kept round it).
  `SaveContext::file_select_new` now goes through it (a fresh SRAM's file 2) and comes out as it
  did.
  - **The game and the sandbox take `--file N`** (1 to 3; file 1 with `--entrance`); `--file N
    --new-file` makes a new file in an empty one (one in use is refused, as the file select only
    names new files in empty slots). The start is tried up front, so an empty file stops the game
    with a message rather than playing something else.
  - **The debug starts** (`--entrance`, a preset, `--new-file` alone) are file 2 ("ZELDAZ", as
    the name entry would leave it) of an SRAM in memory: their saves never reach the disk. The
    sandbox plays on a fresh image unless `--sram PATH` (read at the start, written at the end),
    and never touches the user's file.
  - **F5 is the console's reset** in the window (`PlayState::console_reset`): with `--file N`, file
    N read again from the disk and loaded; for a debug start, file 2 of its image if it has saved
    there, else the same start again. A scripted run resets through `Task::Reset`
    (`Playthrough::take_reset`). The audio's game side carries on across the reset, as across a
    scene change; the title screen and its music aren't ported.
- **The save prompt is ported whole:** B's three branches in `PAUSE_STATE_MAIN`,
  `PAUSE_STATE_SAVE_PROMPT`'s states (`RETURN_TO_MENU`, never set here, included), and
  `KaleidoScope_DrawPages`' prompt page drawn for both prompts (`SAVE_TEXS(LANGUAGE_ENG)` or
  `sGameOverTexs`; "Would you like to save?" with the cursor and Yes/No until saved, nothing
  after: "Game saved." is `!PLATFORM_GC`'s). On GameCube saving closes the menu 3 frames on.
  `gPauseSave10ENGTex`, the one tile the save page doesn't share with the game over's, is a new
  bake: **pack format 26** (`out/data23`).
- **The game over's Yes** calls `Sram_WriteSave`.
- **`ootx sram`** lists the save file's files (name, hearts, deaths, the saved scene, the
  checksums), and stands in for the file select's other screens through the ported functions:
  `verify` (the title's checks, writing what they put right), `erase N` (`Sram_EraseSave`),
  `copy N M` (`Sram_CopySave`, into an empty file only, as the file select allows).
- **Faithful bugs kept** (`@bug (game)`): `Sram_VerifyAndLoadAllSaves` clearing `dayTime` as 32
  bits; a file restored from a good backup written with the checksum the check cleared (0), so the
  next boot restores it again until a save puts it right; `Sram_WriteSave` summing three times
  (two dropped); `FileSelect_LoadGame`'s `gBitFlags[-1]` (0 here); the map select's file (`fileNum`
  0xFF) indexing past `gSramSlotOffsets`, which the port can't reproduce: it writes nothing and
  logs. Not a bug but kept: `Sram_EraseSave` leaves the slot with `sNewSaveChecksum`'s 0, so the
  next boot finds it bad and rebuilds it as a new save; `SLOT_OCCUPIED` ors "ZELDAZ"'s letters.
- **Logged:** `Sram_OpenSave`'s copy of the scarecrow's songs out to the ocarina (not ported; the
  save keeps them); `Sram_InitSram`'s header settings other than stereo, "Switch" and English
  (the port plays with those); `Audio_SetSoundOutputMode` is the boot's stereo, as a fresh header
  gives. `Sram_InitSram`'s D-Right on controller 3 is ported behind an argument the port's one
  controller never sets.

## Consequences

- The pause menu saves: B, Yes, and the file holds the game's bytes; a file loads back where
  `Sram_OpenSave` says (a dungeon at its entrance, elsewhere Link's house or the Temple of Time),
  with three hearts at least. The game over's Yes saves too.
- **A fresh image is zeros**, which every slot's checksum accepts (zero sums to zero): three empty
  files, with no debug file 1 (an image of 0xFF bytes would have its files rebuilt, file 1 as the
  debug save). Writing the header is the title's first write.
- The runs that start from presets now carry "ZELDAZ" and `fileNum` 1 in their save; nothing they
  do reads either. `SaveContext::new` and `debug` gain Epona's `horseData` and the debug save its
  "ZELDAZ", which nothing ported reads.
- **The goldens' traces don't change**; the menu's frame doesn't either (the save prompt only
  draws when B opens it). New: `save` (the trace, the screenshot and the final SRAM image, hashed
  as bytes: `golden.py`'s `{sram}`), `save_prompt_turn`, `save_prompt`.
