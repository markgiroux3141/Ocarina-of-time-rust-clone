@echo off
rem GAME-05 milestone 6c's tests: the exit from room 9 into Queen Gohma's room (Route::BossRoom:
rem the door, room 11's floor, her room's corridor), then the whole Deku Tree from one start (the
rem travel test: every room's connection, room 11, her fight and the heart, the blue warp to the
rem Deku Tree's emerald cutscene, part 1), then 6a's and 6b's tests, and the game's debug starts
rem (--clear for game-boss-room.bat).
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test boss_room --test travel --test gohma --test gohma_run --test blue_warp %* || exit /b 1
cargo test --release -p oot --test start
exit /b %errorlevel%
