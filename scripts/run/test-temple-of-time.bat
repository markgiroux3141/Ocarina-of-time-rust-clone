@echo off
rem GAME-11 milestone 2's tests: the five runs (Route::DoorOfTime, FirstPull, Awakening,
rem TempleExit, Ruins) each ending as its preset, chamberCutsceneNum, Bg_Spot00_Break, Rauru,
rem Sheik (types 9, 1 and 2), and the actor schemas.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test temple_of_time_runs --test bg_spot00_break --test en_rl --test en_xc --test schemas %*
exit /b %errorlevel%
