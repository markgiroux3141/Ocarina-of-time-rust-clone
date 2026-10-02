@echo off
rem GAME-04b milestone 1's tests: one-point cutscenes. The crawlspace's exits (9601 and 9602,
rem the spline out on a sub camera, then the main camera back without a jump), an attention
rem cutscene (Camera_Demo5 into Camera_Unique9, its chime, Link held), the falling chest's shot
rem (4500), and the one-point tables in the pack against the ROM.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test onepoint %* || exit /b 1
cargo test --release -p oot_actors --test crawl %* || exit /b 1
cargo test --release -p oot_import --test pack the_one_point_tables_are_the_roms %*
exit /b %errorlevel%
