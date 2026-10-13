@echo off
rem GAME-07 milestone 2's tests: Hyrule Castle's grounds against the C (Talon asleep, woken by the
rem Cucco shown and his run home, the milk crates, the sentries and the throw-out's circle, the
rem gate guard and Lon Lon's fence, Kakariko's gate and its guard, the pots, the torch, the egg's
rem hatch at dawn, Player_UpdateBottleHeld, the moat's water camera), and the exits' runs
rem (Route::Castle: Talon, the crates, the drain into the courtyard; Route::CastleCaught).
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test castle %*
exit /b %errorlevel%
