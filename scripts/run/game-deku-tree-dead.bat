@echo off
rem The game in front of the Deku Tree after Gohma: the deku-tree-dead save preset
rem (EVENTCHKINF_07 and _09, the Kokiri Emerald). The tree and his mouth in the dead colours.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
"%BIN%\oot.exe" --entrance ENTR_SPOT04_1 --preset deku-tree-dead %*
exit /b %errorlevel%
