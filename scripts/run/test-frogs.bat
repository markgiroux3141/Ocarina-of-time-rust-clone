@echo off
rem GAME-10 milestone 9's tests: the frogs (En_Fr, one-point 4110), the scarecrows (En_Kakasi,
rem En_Kakasi2, En_Kakasi3, one-points 2260, 2270, 2280), the message box's frog and scarecrow
rem modes (in the ocarina's tests), the two runs (Route::Frogs, Route::Bonooru) and the actor
rem schemas. The ocarina's recording's own tests are oot_game's (cargo test -p oot_game).
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test en_fr --test en_kakasi --test en_kakasi2 --test en_kakasi3 --test ocarina --test frogs_runs --test schemas %*
exit /b %errorlevel%
