@echo off
rem Inside the Deku Tree (the deku-tree-inside preset: the Kokiri Sword and the Deku Shield worn),
rem from a debug start (--room, --at) where GAME-05 milestone 4a's mechanisms are:
rem   switch     room 0's top floor, by the floor switch (the default): step on it, the web over
rem              room 10's door burns and the golden torches light; then through the sliding door
rem   lobby-top  room 0's top floor, by room 10's door
rem   lobby      room 0's ground floor, by the floor web
rem   room1 .. room10   each room's own start (milestone 4's debug starts; room 11 is the drop
rem              to Gohma). Rooms 2 and 4 to 8 are behind the slingshot's eye switches until
rem              milestone 5, so they're debug starts only.
rem WASD the stick, Q is Z, E is B, R is R (the shield), Space is A, I is C-Up (Navi).
rem   game-dungeon.bat room4
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "WHERE=%~1"
if "%WHERE%"=="" set "WHERE=switch"
set "START="
if /i "%WHERE%"=="switch" set "START=--at=-382,800,-241,40960"
if /i "%WHERE%"=="lobby-top" set "START=--at=-420,800,60,49152"
if /i "%WHERE%"=="lobby" set "START=--at=0,0,480,32768"
if /i "%WHERE%"=="room1" set "START=--room 1 --at=-700,400,760,0"
if /i "%WHERE%"=="room2" set "START=--room 2 --at=-1100,280,1150,0"
if /i "%WHERE%"=="room3" set "START=--room 3 --at=-718,-820,177,8202"
if /i "%WHERE%"=="room4" set "START=--room 4 --at=-74,-880,796,0"
if /i "%WHERE%"=="room5" set "START=--room 5 --at=-1197,-880,1079,49152"
if /i "%WHERE%"=="room6" set "START=--room 6 --at=-1860,-760,900,0"
if /i "%WHERE%"=="room7" set "START=--room 7 --at=-1900,-760,500,0"
if /i "%WHERE%"=="room8" set "START=--room 8 --at=-2550,-760,-480,0"
if /i "%WHERE%"=="room9" set "START=--room 9 --at=-660,-1880,-620,32768"
if /i "%WHERE%"=="room10" set "START=--room 10 --at=-700,800,100,0"
if not defined START (
    echo Where: switch, lobby-top, lobby, or room1 to room10, not %WHERE%.
    exit /b 1
)
"%BIN%\oot.exe" --entrance ENTR_DEKU_TREE_0 --preset deku-tree-inside %START%
exit /b %errorlevel%
