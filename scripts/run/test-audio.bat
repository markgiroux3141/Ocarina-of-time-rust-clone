@echo off
rem GAME-04 milestone 1's tests: the microcode on made-up data (no game data), then on the pack's
rem audio: every sample decoded bit for bit, a Kokiri Forest instrument's note (its volume each
rem update against the C's ADSR, its pitch at C4 and C3, its decay), a drum, the reverb's decay,
rem the heap laid out as the C lays it, Kokiri Forest's sequence against the extractor's reading,
rem and the pack's audio data against the ROM and the C.
setlocal
call "%~dp0_env.bat"
cargo test --release -p eng_audio %* || exit /b 1
cargo test --release -p oot_game --test audio %* || exit /b 1
cargo test --release -p oot_import --test pack the_audio_data %*
exit /b %errorlevel%
