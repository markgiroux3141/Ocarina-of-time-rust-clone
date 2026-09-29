@echo off
rem GAME-03 milestone 3's tests: Mido (blocking, his texts and flags, stepping aside), the Kokiri
rem shop (the shelves, browsing, buying, the rupee check, Start in the play frame), and the exit
rem test, the scripted run from Link's bed on a new save to the Deku Shield and past Mido.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test mido --test shop %* || exit /b 1
cargo test --release -p oot_actors --test playthrough a_new_save_to_mido_and_the_shop %*
exit /b %errorlevel%
