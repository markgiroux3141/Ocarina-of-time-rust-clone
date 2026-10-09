@echo off
rem Inside the Deku Tree (Master Quest) at its entrance with two hearts and the Fairy Slingshot
rem owned but on no button (the deku-tree-save preset): a debug start, file 2 of an SRAM in
rem memory (nothing reaches your save file). Enter (Start) opens the pause menu: WASD to the
rem slingshot, J (C-Left) equips it; E (B) turns the save prompt in, "Would you like to save?";
rem Space (A) on Yes saves and the menu closes (on No, or E or Enter, it closes without saving).
rem F5 is the console's reset: file 2 loads back, at the Deku Tree's entrance with three hearts
rem and the slingshot on C-Left. (game-file.bat plays a file of your save file.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
"%BIN%\oot.exe" --entrance ENTR_DEKU_TREE_0 --preset deku-tree-save %*
exit /b %errorlevel%
