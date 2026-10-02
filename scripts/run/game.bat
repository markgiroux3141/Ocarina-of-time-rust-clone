@echo off
rem The game in a window. With no arguments: Kokiri Forest (ENTR_KOKIRI_FOREST_0), child Link, 10:00.
rem Extra arguments go to the game, e.g.: game.bat --entrance ENTR_KOKIRI_FOREST_3 --time 18:00
rem Keys: WASD, Space = A, E = B, Q = Z, I/J/K/L = C, P = placeholder markers.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
"%BIN%\oot.exe" %*
exit /b %errorlevel%
