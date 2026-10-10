@echo off
rem Kokiri Forest where the emerald's chain leaves Link (ENTR_KOKIRI_FOREST_11, the deku-tree-dead
rem preset): Mido blocks the path out of the meadow until you talk to him; then the forest is
rem yours. The Lost Woods' exit (north-west, up the hill past the Kokiri) leads to the bridge:
rem Saria's goodbye and the Fairy Ocarina, then Hyrule Field's intro and the owl below his perch,
rem west and north of where you arrive. With "bridge", straight onto the bridge
rem (ENTR_LOST_WOODS_9); with "field", Hyrule Field from the bridge's end.
rem WASD the stick, Space A.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "entr=ENTR_KOKIRI_FOREST_11"
if /i "%~1"=="bridge" (
    set "entr=ENTR_LOST_WOODS_9"
    shift
)
if /i "%~1"=="field" (
    set "entr=ENTR_HYRULE_FIELD_3"
    shift
)
"%BIN%\oot.exe" --entrance %entr% --preset deku-tree-dead %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
