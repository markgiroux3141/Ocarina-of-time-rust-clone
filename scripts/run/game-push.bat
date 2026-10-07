@echo off
rem Inside the Deku Tree (the deku-tree-inside preset: the Kokiri Sword and the Deku Shield
rem worn), from one of GAME-05 milestone 4c's debug starts (--room, --at):
rem   room3    room 3's upper floor behind the push block (the default). Walk up to it (Navi has
rem            a hint there), hold Space (A) to grab it, and push the stick towards it to push it
rem            a block length at a time; the stick away from it pulls. Off the floor's end it
rem            drops into the pit with a chime; climb down beside it and back up onto it
rem   room7    room 7 by the gravestones and the Song of Time's hidden stair (no ocarina yet)
rem   room2    room 2 under the ledge with the three rocks (no bombs yet), by the hidden blocks
rem WASD the stick, Space A, E is B, Q is Z, R the shield, I is C-Up (Navi).
rem   game-push.bat room7
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "WHERE=%~1"
if "%WHERE%"=="" set "WHERE=room3"
set "START="
if /i "%WHERE%"=="room3" set "START=--room 3 --at=-700,-810,-290,16384"
if /i "%WHERE%"=="room7" set "START=--room 7 --at=-1925,-760,360,32768"
if /i "%WHERE%"=="room2" set "START=--room 2 --at=-1290,480,1440,16384"
if not defined START (
    echo Where: room3, room7 or room2, not %WHERE%.
    exit /b 1
)
"%BIN%\oot.exe" --entrance ENTR_DEKU_TREE_0 --preset deku-tree-inside %START%
exit /b %errorlevel%
