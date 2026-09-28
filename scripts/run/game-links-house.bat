@echo off
rem The game from Link's bed (ENTR_LINK_HOME_0) with the Deku Tree's mouth open: the
rem playthrough's route by hand. Out the door, the ladder, the sign, the Kokiri, the bushes,
rem the stream, Mido, the path, the tree. Most of the Project64 checks start here.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
"%BIN%\oot.exe" --entrance ENTR_LINK_HOME_0 --preset deku-tree-open %*
exit /b %errorlevel%
