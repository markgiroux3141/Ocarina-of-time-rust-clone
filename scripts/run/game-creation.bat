@echo off
rem The Kokiri Emerald's chain after the blue warp (the deku-tree-dead preset), started in a
rem cutscene layer as this debug ROM's map select can. With no argument the cutscene map's layer 5
rem (0xFFF1): Ganondorf, then the creation, part by part, to Kokiri Forest's ENTR_KOKIRI_FOREST_11.
rem Or one part: "goddesses" (the cutscene map's layer 4, the blue rain), "din" (Gerudo Valley's
rem layer 5, the rocks), "valley" (its layer 4), "nayru" (Death Mountain Trail's layer 4),
rem "triforce" (the cutscene map's layer 6). Each part goes on into the next.
rem Space through the texts. WASD the stick, Space A.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "entr=ENTR_CUTSCENE_MAP_0"
set "cs=0xFFF1"
if /i "%~1"=="goddesses" (
    set "cs=0xFFF0"
    shift
)
if /i "%~1"=="din" (
    set "entr=ENTR_GERUDO_VALLEY_0"
    shift
)
if /i "%~1"=="valley" (
    set "entr=ENTR_GERUDO_VALLEY_0"
    set "cs=0xFFF0"
    shift
)
if /i "%~1"=="nayru" (
    set "entr=ENTR_DEATH_MOUNTAIN_TRAIL_0"
    set "cs=0xFFF0"
    shift
)
if /i "%~1"=="triforce" (
    set "cs=0xFFF2"
    shift
)
"%BIN%\oot.exe" --entrance %entr% --preset deku-tree-dead --cutscene %cs% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
