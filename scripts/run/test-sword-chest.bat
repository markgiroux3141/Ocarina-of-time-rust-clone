@echo off
rem GAME-03 milestone 1's exit test: the Kokiri Sword's chest on a new save (the item held up,
rem its text, the owned bit, B getting the sword through the pause menu's stand-in).
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test chest %*
exit /b %errorlevel%
