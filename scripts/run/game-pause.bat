@echo off
rem Inside the Deku Tree's lobby with the Fairy Slingshot owned but on no button (the
rem deku-tree-slingshot-owned preset: ten Deku Sticks on C-Left, ten Deku nuts on C-Down, the
rem Kokiri Sword and the Deku Shield worn). Enter (Start) opens the pause menu on its item page:
rem WASD moves the cursor, J, K and L (C-Left, C-Down, C-Right) equip the item under it, R and Q
rem (R and Z) turn the pages, Enter closes the menu. The other pages show their backgrounds only
rem (GAME-05 milestone 5b-1); E (B) opens the save prompt (5c: a debug start saves to an SRAM
rem in memory, not your save file); T (L), the debug inventory editor, isn't ported.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
"%BIN%\oot.exe" --entrance ENTR_DEKU_TREE_0 --preset deku-tree-slingshot-owned %*
exit /b %errorlevel%
