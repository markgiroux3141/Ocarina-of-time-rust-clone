@echo off
rem GAME-04b milestone 5's tests: the title cards. Entering Kokiri Forest starts its place name
rem (g_pn_31: 20 frames of delay, in by 10, held, out by 30, drawn centred), and the Deku Tree's
rem intro shows Inside the Deku Tree from its script's CS_MISC 15.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test environment entering_kokiri_forest_shows_its_place_name %*
if errorlevel 1 goto end
cargo test --release -p oot_actors --test cutscene entering_the_deku_tree_the_first_time_plays_its_intro %*
:end
exit /b %errorlevel%
