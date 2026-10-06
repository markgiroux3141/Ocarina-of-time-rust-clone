@echo off
rem Inside the Deku Tree with ten Deku Sticks on C-Left (the deku-tree-sticks preset: also the
rem Kokiri Sword and the Deku Shield worn), from a debug start (--room, --at, --switch):
rem   torch    room 0's middle floor by its golden torch, lit (flag 0x27, as if the top floor's
rem            switch were pressed): the default. Round the floor to the left, over the gap, is
rem            the web over room 1's door; the crate with the Gold Skulltula is to the right
rem   room3    room 3 by its golden torch, lit (0x02), the door to room 4 open (0x15, its eye
rem            switch's): carry the fire to room 4's two timed torches
rem   room10   room 10 by its wooden torch (always lit): the timed torch below drops a chest; the
rem            floor switch raises the three platforms
rem   room5    room 5: the spiked log, the floating block, the water
rem   room2    room 2 on the lift that shakes and falls
rem J is C-Left: the stick out, J again to swing it. A while standing puts it away. E (B) takes
rem out the sword. WASD the stick, Q is Z, R the shield, Space A, I is C-Up (Navi).
rem   game-sticks.bat room3
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "WHERE=%~1"
if "%WHERE%"=="" set "WHERE=torch"
set "START="
if /i "%WHERE%"=="torch" set "START=--at=330,360,100,16384 --switch 0x27"
if /i "%WHERE%"=="room3" set "START=--room 3 --at=-102,-880,330,32768 --switch 0x02,0x15"
if /i "%WHERE%"=="room10" set "START=--room 10 --at=-700,800,-60,49152"
if /i "%WHERE%"=="room5" set "START=--room 5 --at=-1197,-880,1079,49152"
if /i "%WHERE%"=="room2" set "START=--room 2 --at=-1214,408,1208,0"
if not defined START (
    echo Where: torch, room3, room10, room5 or room2, not %WHERE%.
    exit /b 1
)
"%BIN%\oot.exe" --entrance ENTR_DEKU_TREE_0 --preset deku-tree-sticks %START%
exit /b %errorlevel%
