@echo off
rem GAME-05 milestone 5b-1's tests: the pause menu (z_kaleido_setup.c, z_kaleido_scope_call.c,
rem z_kaleido_scope.c's frame, z_kaleido_item.c): Start opening it frame by frame, the item
rem page's cursor and the stick's repeat, an equip with its icon flying to the C button, a wrong
rem age's error and grey icon, a page turn, closing and the game resumed, the equipment page's
rem stand-in, every quad baked, the scene stopped behind it; the menu's tables and the C buttons'
rem swap; the stick and slingshot tests that equip through it; the exit's run (Route::Pause).
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test pause --test pause_run --test stick --test slingshot %* || exit /b 1
cargo test --release -p oot_game --lib kaleido
exit /b %errorlevel%
