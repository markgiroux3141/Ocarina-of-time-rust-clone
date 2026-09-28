@echo off
rem GAME-02's exit test: Bg_Treemouth, and the scripted run from Link's bed into the Deku Tree.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test playthrough %*
exit /b %errorlevel%
