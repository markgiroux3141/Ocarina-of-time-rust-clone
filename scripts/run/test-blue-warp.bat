@echo off
rem GAME-05 milestone 6b's tests: the blue warp (Door_Warp1) against the C: growing in, taking
rem Link (one-point 9703, his walk to its centre), his float and the way out with the Kokiri Emerald
rem (and a second time's), the destination warp killed at Kokiri Forest's arrival, the portal's
rem bake on its two matrices; Player's blue-warp arrival into the emerald's cutscene; the exit's
rem run (Route::BlueWarp); then 6a's Gohma tests.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test blue_warp --test gohma --test gohma_run %*
exit /b %errorlevel%
