@echo off
rem Inside the Deku Tree with the Fairy Slingshot on C-Right (L), ten Deku nuts on C-Down (K) and
rem ten Deku Sticks on C-Left (J) (the deku-tree-slingshot preset: also the Kokiri Sword and the
rem Deku Shield worn), from a debug start (--room, --at):
rem   room1    room 1, 250 in front of its eye switch over the door to room 2: the default. L
rem            takes the slingshot out and aims in first person: the stick aims (pushed up aims
rem            down), hold L to keep it drawn, let go to shoot; Space (A) puts it down
rem   room3    room 3 under its eye switch over the door to room 4
rem   room2    room 2's floor in front of the ladder up by the door to room 1
rem   room10   room 10 in front of the slingshot's chest: it drops once the room's enemies
rem            are gone
rem K throws a nut (a flash that stuns). I (C-Up) looks around in first person; I again stops.
rem Q (Z) with the slingshot raised aims in third person. WASD the stick, Space A, E is B,
rem R the shield, Enter Start.
rem   game-slingshot.bat room3
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "WHERE=%~1"
if "%WHERE%"=="" set "WHERE=room1"
set "START="
if /i "%WHERE%"=="room1" set "START=--room 1 --at=-743,400,741,57344"
if /i "%WHERE%"=="room3" set "START=--room 3 --at=-76,-880,351,0"
if /i "%WHERE%"=="room2" set "START=--room 2 --at=-1130,280,1360,30533"
if /i "%WHERE%"=="room10" set "START=--room 10 --at=-1082,820,300,0"
if not defined START (
    echo Where: room1, room3, room2 or room10, not %WHERE%.
    exit /b 1
)
"%BIN%\oot.exe" --entrance ENTR_DEKU_TREE_0 --preset deku-tree-slingshot %START%
exit /b %errorlevel%
