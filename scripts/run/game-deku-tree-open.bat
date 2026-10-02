@echo off
rem The game in front of the Deku Tree (Kokiri Forest's spawn 1), his mouth open: the
rem deku-tree-open save preset (EVENTCHKINF_0C and _05). Walk in for the Deku Tree's scene.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
"%BIN%\oot.exe" --entrance ENTR_KOKIRI_FOREST_1 --preset deku-tree-open %*
exit /b %errorlevel%
