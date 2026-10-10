@echo off
rem GAME-06 milestone 1a's tests: Demo_Effect against the C (each type's init, the goddesses'
rem lights and what they leave, the light rings, the blue orb, the light, the Triforce, the jewels,
rem the Song of Time blocks' time warp on its curve skeleton, the vertices its draws write), the
rem curve skeletons (oot_game::skel_curve), the game's --cutscene start, and the exit's run
rem (Route::Emerald: Kokiri Forest's cutscene layer 6, the emerald and the Deku Tree's death).
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test demo_effect %*
if errorlevel 1 exit /b %errorlevel%
cargo test --release -p oot_game skel_curve %*
if errorlevel 1 exit /b %errorlevel%
cargo test --release -p oot --test start %*
exit /b %errorlevel%
