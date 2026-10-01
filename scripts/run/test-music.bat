@echo off
rem GAME-04 milestone 2's tests: the scenes' music headless, through the audio library offline.
rem Kokiri Forest from a new game: its sound settings, the spec change and the music's start
rem as the C queues them, the first Audio_Update's commands, the wait for the reset, then the
rem forest's sequence playing and looping; from Link's house into the forest, the exit's fade
rem and the music resumed; the forest at night, its nature ambience and critters. Then the
rem game's side of the audio boundary (the boot view, the spec change's paths) and the pack's
rem game audio tables and sound settings against the C and the ROM.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test music %* || exit /b 1
cargo test --release -p oot_game --test audio -- the_boot_view the_spec_change %* || exit /b 1
cargo test --release -p oot_import --test pack the_games_audio %*
exit /b %errorlevel%
