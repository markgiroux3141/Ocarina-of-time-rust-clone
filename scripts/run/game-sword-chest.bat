@echo off
rem GAME-03 milestone 1's exit by hand: a new save (no sword, no shield) in front of the Kokiri
rem Sword's chest. The crawl (milestone 2) isn't ported, so this enters Kokiri Forest
rem (ENTR_SPOT04_0), changes to room 2 and puts Link 34 in front of the chest, facing it.
rem Space (A) opens it; Space through the text; then Enter (Start: the pause menu's stand-in)
rem puts the sword on B, and E swings it.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
"%BIN%\oot.exe" --entrance ENTR_SPOT04_0 --room 2 --at=-232,178,2211,0 %*
exit /b %errorlevel%
