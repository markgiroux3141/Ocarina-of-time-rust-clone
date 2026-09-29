@echo off
rem GAME-03 milestone 4's tests: the cutscene scripts from the pack, the spline camera, the Deku
rem Tree's talk (yes opens his mouth, no asks again), his scene's intro on the first entry, a
rem forced text holding Link, and the exit test, the scripted run from Link's bed on a new save
rem past Mido into the Deku Tree, no preset.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test cutscene %* || exit /b 1
cargo test --release -p oot_actors --test playthrough a_new_save_into_the_deku_tree %*
exit /b %errorlevel%
