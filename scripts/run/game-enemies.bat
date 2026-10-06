@echo off
rem Inside the Deku Tree (the deku-tree-inside preset: the Kokiri Sword and the Deku Shield worn),
rem from a debug start in the room of one of GAME-05 milestone 3b's enemies (--room, --at; the
rem other rooms' doors and drops aren't solid until milestone 4):
rem   scrub      room 4, 250 in front of a Mad Scrub (the default)
rem   hint       room 9, by the three hint scrubs (knock them out in order with their own nuts)
rem   shop       room 3, by the Business Scrub (knock it out: the salesman)
rem   skulltula  room 5, 150 in front of a Skulltula on its thread
rem   walltula   room 0, under a Skullwalltula on the wall
rem   gold       room 0, by a Gold Skulltula
rem   larva      room 0, on the ledge by a Gohma egg (another drops from the ceiling)
rem WASD the stick, Q is Z, E is B, R is R (the shield), Space is A.
rem   game-enemies.bat hint
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "WHO=%~1"
if "%WHO%"=="" set "WHO=scrub"
set "START="
if /i "%WHO%"=="scrub" set "START=--room 4 --at=-74,-880,796,0"
if /i "%WHO%"=="hint" set "START=--room 9 --at=-660,-1880,-620,32768"
if /i "%WHO%"=="shop" set "START=--room 3 --at=-718,-820,177,8202"
if /i "%WHO%"=="skulltula" set "START=--room 5 --at=-1197,-880,1079,49152"
if /i "%WHO%"=="walltula" set "START=--at=95,0,-230,32768"
if /i "%WHO%"=="gold" set "START=--at=320,360,310,54236"
if /i "%WHO%"=="larva" set "START=--at=300,360,330,27534"
if not defined START (
    echo Which enemy: scrub, hint, shop, skulltula, walltula, gold or larva, not %WHO%.
    exit /b 1
)
"%BIN%\oot.exe" --entrance ENTR_DEKU_TREE_0 --preset deku-tree-inside %START%
exit /b %errorlevel%
