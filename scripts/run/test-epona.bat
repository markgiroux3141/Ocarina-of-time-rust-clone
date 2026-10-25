@echo off
rem GAME-12 milestone 1's tests: the two riding runs (Route::RideEpona, FieldRide) each ending as
rem its preset, En_Horse and Bg_Umajump, the horse camera, and the actor schemas.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test riding_runs --test en_horse --test camera --test schemas %*
exit /b %errorlevel%
