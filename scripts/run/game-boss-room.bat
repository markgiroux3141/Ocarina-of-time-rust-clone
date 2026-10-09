@echo off
rem Inside the Deku Tree, room 9 with its hint scrubs' puzzle solved (--clear 9: its door to room 11
rem open), a debug start with the sword, the shield, the Fairy Slingshot on C-Right (L), Deku nuts
rem on C-Down (K) and Deku Sticks on C-Left (J) (the deku-tree-slingshot preset): through the door,
rem onto room 11's floor, which takes Link into Queen Gohma's room; then her fight, the heart and
rem the blue warp out to the Deku Tree (game-gohma.bat says how the fight goes).
rem WASD the stick, Q is Z, E is B, Space A, R the shield, I is C-Up (Navi), Enter Start.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
"%BIN%\oot.exe" --entrance ENTR_DEKU_TREE_0 --preset deku-tree-slingshot --clear 9 --room 9 --at=-660,-1880,-620,32768 %*
exit /b %errorlevel%
