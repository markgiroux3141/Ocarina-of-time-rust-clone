@echo off
rem GAME-06 milestone 1b's tests: Demo_Kankyo against the C (the cutscene map's rain hiding the
rem room, Din's rocks on their cues, the clouds, the Door of Time, the warp sparkles and their
rem leave, the sparkles), Bg_Spot09_Obj (Gerudo Valley's bridges and tent), Bg_Spot16_Doughnut
rem (Death Mountain's cloud ring), the normal sky's loads, the two new scene draw configs against
rem the C interpreter, and the exit's runs (Route::Creation, parts 2 to 9; then the whole chain
rem from the blue warp).
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test demo_kankyo %*
if errorlevel 1 exit /b %errorlevel%
cargo test --release -p oot_import --test pack ported_draw_configs %*
exit /b %errorlevel%
