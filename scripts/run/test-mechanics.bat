@echo off
rem GAME-05 milestone 4a's tests: Door_Shutter and Player's sliding door (through room 10's door
rem and barred behind, unbarred by the room's clear or a switch, a small key spent), the small
rem key counter, the floor, eye and crystal switches, the torches and the Keese set alight at
rem one, the webs (bouncing, broken by a fall, burnt), Navi's hint tags, the quakes, the map and
rem compass data, every room's debug start, and the exit's run (Route::Shutter).
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test doors --test switches --test torches --test elf_msg --test webs --test map --test debug_starts --test shutter_run %* || exit /b 1
cargo test --release -p oot_game --lib -- quake interface::tests::the_small_key || exit /b 1
cargo test --release -p oot_import --test pack the_map_tables
exit /b %errorlevel%
