@echo off
rem The Lost Woods from Kokiri Forest with Saria's Song and the slingshot (meadow-saria). West
rem (left at the first fork) to the Skull Kid on his stump: stand on the small stump in front of
rem him, take out the ocarina and play Saria's Song (a piece of heart). East (right at the first
rem fork): the target across the hollow; shoot three bullseyes in a row (C-Right, aim, let go) and
rem the scrub comes up with a bigger seed bag. Down in the hollow (off the clearing's north end),
rem onto the stump between the two Skull Kids, take out the ocarina: play back their notes,
rem three rounds (the third gives a piece of heart).
rem "stump": on room 9's small stump. "target": room 1's clearing facing the target.
rem "game": on the memory game's stump.
rem WASD the stick (Shift walks), Space A, E B, Q Z, R R, J C-Left, K C-Down (the ocarina),
rem L C-Right (the slingshot), I C-Up. Saria's Song: K L J K L J.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "at="
if /i "%~1"=="stump" (
    set "at=--room 9 --at=-951,20,52,-24576"
    shift
)
if /i "%~1"=="target" (
    set "at=--room 1 --at=960,0,-30,16384"
    shift
)
if /i "%~1"=="game" (
    set "at=--room 1 --at=1123,-140,340,8192"
    shift
)
"%BIN%\oot.exe" --entrance ENTR_LOST_WOODS_0 --preset meadow-saria %at% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
