@echo off
rem GAME-05 milestone 5c's tests: saving. z_sram.c against the C (the slots and the save's
rem layout, the checksum, Sram_WriteSave, Sram_VerifyAndLoadAllSaves with its repairs,
rem Sram_OpenSave's entrances and fixes, Sram_InitSave, Sram_EraseSave, Sram_CopySave,
rem Sram_InitSram) and the file select's stand-ins; the pause menu's save prompt (B opening it,
rem its turn, No, B and Start closing it, Yes saving and closing the menu, its page drawn, every
rem quad baked); the game over's Yes writing the save; the exit's run (saved from the menu, the
rem console's reset, the file loaded back); then 5b's pause and game over tests again.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test save --test pause --test pause_run --test game_over_screens %* || exit /b 1
cargo test --release -p oot_game --lib sram save:: kaleido
exit /b %errorlevel%
