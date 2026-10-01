@echo off
rem GAME-03 milestone 5's tests: Navi and the Kokiri fairies (En_Elf), the opening from the file
rem select's new file through its four cutscene layers, and Phase 4's exit test, the scripted
rem run from the new file's first frame through the opening into the Deku Tree.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test navi %* || exit /b 1
cargo test --release -p oot_actors --test cutscene the_opening %* || exit /b 1
cargo test --release -p oot_actors --test playthrough a_new_file_into_the_deku_tree %*
exit /b %errorlevel%
