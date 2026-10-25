@echo off
rem GAME-12 milestone 2's tests: the four Ingo runs (Route::IngoRental, IngoRace, IngoLost,
rem IngoWon), En_In (the adult's Ingos, the rental's minute), Bg_Ingate, En_Horse_Game_Check, and
rem the actor schemas.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test ingo_runs --test en_in --test bg_ingate --test en_horse_game_check --test schemas %*
exit /b %errorlevel%
