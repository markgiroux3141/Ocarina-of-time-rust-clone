@echo off
rem GAME-05 milestone 3a's tests: Link's guard (entering, holding, leaving; a blow blocked; a
rem Deku nut bounced back; a fire blow burning the Deku Shield away), Camera_Battle1 against the
rem C's arithmetic, a hit mark's life frame by frame, the withered Deku Baba's and the Keese's
rem states and damage tables, and the exit's run (Route::Combat).
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test guard --test enemies --test effects --test combat %* || exit /b 1
cargo test --release -p oot_game --lib camera::tests %*
exit /b %errorlevel%
