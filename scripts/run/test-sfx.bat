@echo off
rem GAME-04 milestone 3's tests: the sound effects headless, through the audio library offline.
rem Walking in Kokiri Forest plays a footstep on the C's frames, for the floor underfoot, and
rem each sounds on the sound effects' sequence; the message box sounds its passes, its end and
rem its close. Then the pack's sound effect tables against the C.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test sfx %* || exit /b 1
cargo test --release -p oot_import --test pack the_sound_effects %*
exit /b %errorlevel%
