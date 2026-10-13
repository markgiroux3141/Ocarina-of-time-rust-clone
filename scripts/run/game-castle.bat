@echo off
rem Hyrule Castle's road by day (ENTR_HYRULE_CASTLE_0) with the Cucco on C-Left (castle-cucco):
rem up the vine, past the sentries to Talon asleep by the crates, the Cucco shown to him.
rem "talon": start by Talon, facing him. "egg": with the Weird Egg instead (castle-egg: the gate
rem guard's bribe, getting caught). "night": the egg at 22:00 (the sentries' shorter sight).
rem "dawn": Hyrule Field at 06:25 with the egg (it hatches at 06:30). "kakariko": Kakariko's gate
rem and its guard, the pots.
rem WASD the stick (Shift to walk), Space A, E B (the sword), Q Z (lock on), R the shield, J C-Left
rem (the Cucco or the egg: shown to whoever Link faces).
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "clock=10:00"
set "entr=ENTR_HYRULE_CASTLE_0"
set "preset=castle-cucco"
set "at="
if /i "%~1"=="talon" (
    set "at=--at=950,1561,-299,-16384"
    shift
)
if /i "%~1"=="egg" (
    set "preset=castle-egg"
    shift
)
if /i "%~1"=="night" (
    set "preset=castle-egg"
    set "clock=22:00"
    shift
)
if /i "%~1"=="dawn" (
    set "entr=ENTR_HYRULE_FIELD_0"
    set "preset=castle-egg"
    set "clock=06:25"
    shift
)
if /i "%~1"=="kakariko" (
    set "entr=ENTR_KAKARIKO_VILLAGE_0"
    set "preset=deku-tree-dead"
    shift
)
"%BIN%\oot.exe" --entrance %entr% --preset %preset% --time %clock% %at% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
