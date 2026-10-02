@echo off
rem GAME-04b milestone 4's tests: the environment across frames. The nightmare's rain (20 drops,
rem 2 more every 8 frames) and lightning (three bolts 9500 ahead, each drawn through its eight
rem textures after its wait), a cutscene's light setting override blending in, and a lightning
rem strike's flash raising the ambient light.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test environment %*
exit /b %errorlevel%
