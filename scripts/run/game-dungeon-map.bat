@echo off
rem Inside the Deku Tree's lobby (1F) with its compass, and 3F, 2F and 1F visited (the
rem deku-tree-compass preset: rooms 0 to 2 seen). Enter (Start) opens the pause menu; R turns it
rem to the map page, the cursor arriving on the L arrow: D (the stick right) onto the floors.
rem W and S move between the visited floors, each floor's room maps loading (the room Link is in
rem pulses on his floor; the compass marks the floor's unopened chests and the boss's skull's
rem floor). D again moves to the items' column (the compass); D once more to the R arrow. Enter
rem closes the menu.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
"%BIN%\oot.exe" --entrance ENTR_DEKU_TREE_0 --preset deku-tree-compass %*
exit /b %errorlevel%
