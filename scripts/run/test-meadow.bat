@echo off
rem GAME-08 milestone 1's tests: Saria (her types, texts, the Kokiri Forest greeting, the meadow's
rem lesson and the song given), the Wolfos (its rise, its damage, its death's switch flag, a child's
rem fight), the meadow's gate and blocks (Bg_Spot05_Soko), the exit's run (Route::Meadow), and the
rem actor schemas (every ported actor's against its code and the game's data).
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test en_sa --test en_wf --test meadow --test schemas %*
exit /b %errorlevel%
