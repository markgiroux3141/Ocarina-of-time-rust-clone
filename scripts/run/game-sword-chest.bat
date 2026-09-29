@echo off
rem A shortcut to the Kokiri Sword's chest on a new save (no sword, no shield): enters Kokiri
rem Forest (ENTR_SPOT04_0), changes to room 2 and puts Link 34 in front of the chest, facing it
rem (the game's --room and --at debug start). Space (A) opens it; Space through the text; then
rem Enter (Start: the pause menu's stand-in) puts the sword on B, and E swings it.
rem The game's way there, through the crawlspace and past the boulder (GAME-03 milestone 2), starts
rem in Link's bed: game-new-save.bat.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
"%BIN%\oot.exe" --entrance ENTR_SPOT04_0 --room 2 --at=-232,178,2211,0 %*
exit /b %errorlevel%
