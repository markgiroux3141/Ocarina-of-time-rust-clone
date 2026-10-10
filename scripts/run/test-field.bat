@echo off
rem GAME-06 milestone 4's tests: Hyrule Field's actors against the C (the Peahat and its larvae,
rem the Stalchild and its spawner, the parts a body breaks into, the enemy music, the roll's bonk
rem into a tree, a grotto's hole), and the exit's runs (Route::Field: a Peahat by day, Castle
rem Town's entrance; Route::FieldNight: Stalchildren at 20:00, the owl, Kakariko).
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test field %*
exit /b %errorlevel%
