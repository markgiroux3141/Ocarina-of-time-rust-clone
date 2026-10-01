@echo off
rem The game with sound (GAME-04 milestone 1): Kokiri Forest with its music, sequence 60, played
rem by the ported audio library through your default output device. The game doesn't start
rem music by itself yet (that's milestone 2): --music picks the sequence, e.g. 30 for the title
rem theme, 62 for the Lost Woods, 85 for the shop, 28 inside the Deku Tree (game.bat --music 62).
rem Kokiri Forest's loops after about 51 s. No sound effects yet.
rem The HUD's audio line names the device. Any other game flags work too (game-music.bat
rem --new-file).
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
"%BIN%\oot.exe" --music 60 %*
exit /b %errorlevel%
