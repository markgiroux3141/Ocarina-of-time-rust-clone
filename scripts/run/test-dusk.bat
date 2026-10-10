@echo off
rem GAME-06 milestone 3's tests: the clock against the C (the rooms' time settings, time passing
rem by day and by night and waiting for messages, a new day's count and the cock's crow, the
rem Sun's Song), the field's music at dusk, the sun and the moon, the skybox filters, the lens
rem flare, the drawbridge's night, the depth probe on the GPU, and the exit's run (Route::Dusk:
rem Hyrule Field from 17:00 to the night).
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test clock %*
if errorlevel 1 exit /b %errorlevel%
cargo test --release -p oot_game clock %*
if errorlevel 1 exit /b %errorlevel%
cargo test --release -p oot --test depth_probe %*
exit /b %errorlevel%
