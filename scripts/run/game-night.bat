@echo off
rem The game in Kokiri Forest at 20:00, with sound: by night the forest plays its nature
rem ambience (the stream, the crickets and the other critters) instead of its music
rem (Environment_PlaySceneSequence). Time doesn't pass yet, so it stays night.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
"%BIN%\oot.exe" --entrance ENTR_KOKIRI_FOREST_0 --time 20:00 %*
exit /b %errorlevel%
