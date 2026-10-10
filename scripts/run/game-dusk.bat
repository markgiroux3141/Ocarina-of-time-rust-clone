@echo off
rem Hyrule Field in front of the drawbridge at 17:00 (ENTR_HYRULE_FIELD_0, the deku-tree-dead
rem preset): the sun setting in the west with its lens flare, the field's music stopping after
rem 17:10, the dog at 18:00, the drawbridge rising for the night, the night's critters after
rem 19:00. With "night", 21:00 (the moon); with "dawn", 5:50 (the morning's critters at 6:30,
rem the cock's crow and a new day, the music at 7:00).
rem WASD the stick, Space A.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "clock=17:00"
if /i "%~1"=="night" (
    set "clock=21:00"
    shift
)
if /i "%~1"=="dawn" (
    set "clock=05:50"
    shift
)
"%BIN%\oot.exe" --entrance ENTR_HYRULE_FIELD_0 --preset deku-tree-dead --time %clock% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
