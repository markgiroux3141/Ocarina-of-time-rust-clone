@echo off
rem Headless: the Mido and shop run (Phase 5's exit run: the sword's chest, the rupees, Mido's
rem chests, the shop, Mido) with the audio offline: its audio log (every sound effect request
rem with where it is, the sequence commands, the library's commands) into
rem out\run\mido_shop_audio.json (the golden case mido_shop_audio), and its sound as
rem out\run\mido_shop.wav (about 5 minutes); then opens the WAV.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist "%REPO%\out\run" mkdir "%REPO%\out\run"
"%BIN%\oot_sandbox.exe" --entrance ENTR_LINK_HOME_0 --child --script mido-shop --audio-log "%REPO%\out\run\mido_shop_audio.json" --wav "%REPO%\out\run\mido_shop.wav" %* || exit /b 1
start "" "%REPO%\out\run\mido_shop.wav"
exit /b 0
