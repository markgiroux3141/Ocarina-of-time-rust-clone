@echo off
rem GAME-12 milestone 3's tests: the run (Route::EponaWon) ending as epona-won, the four leaps'
rem cutscenes, and the riding and Ingo tests it stands on.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test epona_won_run --test en_in --test bg_ingate --test en_horse --test schemas %*
exit /b %errorlevel%
