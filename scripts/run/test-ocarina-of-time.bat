@echo off
rem GAME-10 milestone 7's tests: the Ocarina of Time (Item_Ocarina: the moat's, Impa's throws),
rem Ganondorf's cape (En_Ganon_Mant), the two runs (Route::Escape, Route::OcarinaOfTime) and the
rem actor schemas.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test item_ocarina --test en_ganon_mant --test ocarina_of_time_runs --test schemas %*
exit /b %errorlevel%
