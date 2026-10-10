@echo off
rem Hyrule Field by day (ENTR_HYRULE_FIELD_0, the deku-tree-dead preset), Link 600 east of the
rem grounded Peahat west of the castle: it rises and comes at him; its root under its body is
rem where the sword hurts it. With "night", 20:00 on the grass north of Lon Lon Ranch: the
rem Stalchildren rise round Link (the owl waits at Kakariko's stairs, east over the stream bed).
rem WASD the stick, Space A, E B (the sword), Q Z (lock on), R the shield.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "clock=10:00"
set "at=-4258,-300,-216,-16384"
if /i "%~1"=="night" (
    set "clock=20:00"
    set "at=1500,0,2000,16384"
    shift
)
"%BIN%\oot.exe" --entrance ENTR_HYRULE_FIELD_0 --preset deku-tree-dead --time %clock% --at=%at% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
