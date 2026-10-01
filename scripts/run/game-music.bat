@echo off
rem The game in Kokiri Forest with another sequence forced in place of its own
rem (--music 30, the title theme: Environment_ForcePlaySequence). The game plays each
rem scene's own music by itself now (game.bat); with arguments, game-music.bat --music <n>
rem forces another (62 the Lost Woods, 85 the shop).
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
if not "%~1"=="" goto custom
"%BIN%\oot.exe" --music 30
exit /b %errorlevel%
:custom
"%BIN%\oot.exe" %*
exit /b %errorlevel%
