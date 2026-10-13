@echo off
rem GAME-07 milestone 4's tests: the guards' courtyard against the C (the patrols by day and the drain
rem guards by night, a guard looking only on his screen, the search ball's life and its end at a wall,
rem seen up close: "Hey you!" and kicked out by the crates; Camera_Special6's slide and freeze; a
rem rupee taken), Zelda (waiting at the window, her talk taken in front of her, her camera; her
rem questions again on no; only talking after the letter), Impa (waiting for the letter, her lullaby
rem within 130; the field's Deku Nut and her end), the exit's run (Route::Zelda), and the castle's,
rem courtyards' and Temple of Time's draw configs.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test zelda %*
if errorlevel 1 exit /b %errorlevel%
cargo test --release -p oot_game --test scene %*
if errorlevel 1 exit /b %errorlevel%
cargo test --release -p oot_import --test pack ported_draw_configs %*
exit /b %errorlevel%
