@echo off
rem GAME-07 milestone 3's tests: the ocarina against the C (out with C-Left and its free play, the
rem staff's notes, B to stop; Saria's Song: the box's modes, its fanfare, Oceff_Wipe3, Navi's call;
rem the Sun's Song by night: Oceff_Spot, the clock at 400 to dawn; the Song of Storms: Oceff_Storm,
rem En_Okarina_Effect's rain and its end; Shot_Sun's fairy for the Song of Storms and not for another
rem song; an En_Okarina_Tag's check, a wrong try and the right song; the meadow's script teaching
rem Saria's Song and hearing it back, a wrong note first), the exit's run (Route::Ocarina), and the
rem Song of Time at Obj_Timeblock.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test ocarina --test timeblocks %*
if errorlevel 1 exit /b %errorlevel%
cargo test --release -p oot_game --lib audio::ocarina %*
exit /b %errorlevel%
