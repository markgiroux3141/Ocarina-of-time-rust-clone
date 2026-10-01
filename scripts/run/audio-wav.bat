@echo off
rem Renders WAVs offline through the audio library (ootx audio-wav), into out\audio, then opens
rem the first in your default player:
rem   kokiri_forest.wav   Kokiri Forest's music (sequence 60), 75 s: its loop comes at about 51 s
rem   kokiri_note.wav     font 15's instrument 4 at C4 for a second, then its decay
rem   kokiri_note_c3.wav  the same an octave down
rem   drum.wav            font 3's drum 0
rem   kokiri_sample.wav   instrument 4's sample as stored (--raw), for comparing
rem With arguments, runs ootx audio-wav with them instead (e.g. audio-wav.bat --seq 30 --seconds
rem 60 --wav out\audio\title.wav for the title theme).
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" ootx || exit /b 1
if not "%~1"=="" goto custom
if not exist "%REPO%\out\audio" mkdir "%REPO%\out\audio"
"%BIN%\ootx.exe" audio-wav --seq 60 --seconds 75 --wav "%REPO%\out\audio\kokiri_forest.wav" || exit /b 1
"%BIN%\ootx.exe" audio-wav --font 15 --inst 4 --note 39 --seconds 1 --wav "%REPO%\out\audio\kokiri_note.wav" || exit /b 1
"%BIN%\ootx.exe" audio-wav --font 15 --inst 4 --note 27 --seconds 1 --wav "%REPO%\out\audio\kokiri_note_c3.wav" || exit /b 1
"%BIN%\ootx.exe" audio-wav --font 3 --drum 0 --seconds 1 --wav "%REPO%\out\audio\drum.wav" || exit /b 1
"%BIN%\ootx.exe" audio-wav --font 15 --inst 4 --note 39 --raw --wav "%REPO%\out\audio\kokiri_sample.wav" || exit /b 1
start "" "%REPO%\out\audio\kokiri_forest.wav"
exit /b 0
:custom
"%BIN%\ootx.exe" audio-wav %*
exit /b %errorlevel%
