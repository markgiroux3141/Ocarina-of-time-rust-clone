@echo off
rem GAME-07 milestone 1's tests: Castle Town's actors against the C (child Malon, the haggling
rem townsfolk, the dancing couple, the cucco girl and her cucco, a gossip stone, the butterflies,
rem the Temple of Time's altar, windows and warp pad, the market's balcony camera), and the exit's
rem run (Route::Market: into the market, a townsman, Malon, the castle's road and its intro).
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test market %*
exit /b %errorlevel%
