@echo off
rem Imports the asset pack from the ROM and decomp in oot.toml into the session's data folder
rem (about 11 s). Only needed after a pack format change; the game imports by itself if
rem there's no pack.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
if not exist "%OOT_DATA_DIR%" mkdir "%OOT_DATA_DIR%"
"%BIN%\oot.exe" import %*
exit /b %errorlevel%
