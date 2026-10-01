@echo off
rem Looks a sound effect up in the pack's tables (gSfxParams): by id (0x2800), by name
rem (NA_SE_EV_DOOR_OPEN) or by part of one (DOOR). Prints its importance and parameters, and
rem the constant for oot_game::audio::sfx. Asks for one if none is given.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" ootx || exit /b 1
set "q=%~1"
if not "%q%"=="" goto run
set /p "q=Sound effect (id, name or part of one): " || exit /b 0
:run
"%BIN%\ootx.exe" sfx "%q%"
exit /b %errorlevel%
