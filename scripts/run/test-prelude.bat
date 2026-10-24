@echo off
rem GAME-11 milestone 3's tests: the four runs (Route::Prelude, SwordBack, SwordAgain,
rem PreludeWarp) each ending as its preset (Link hidden through the warp's white-out and arrival),
rem Sheik (En_Xc), Demo_Kankyo, and the actor schemas.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test time_travel_runs --test en_xc --test demo_kankyo --test schemas %*
exit /b %errorlevel%
