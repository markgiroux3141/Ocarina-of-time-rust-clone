@echo off
rem GAME-08 milestone 2's tests: the Skull Kids (En_Skj: every placement's init, the flight before
rem Saria's Song, the song on the small stump and its heart piece, the memory game's rounds, its
rem failures, the fight and the needles), the target scrub (En_Dnt_Nomal: the bullseyes, the
rem scrub's walk and the seed bag) and the prizes (En_Ex_Item), the exit's run (Route::LostWoods),
rem and the actor schemas (every ported actor's against its code and the game's data).
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test en_skj --test en_dnt_nomal --test en_ex_item --test lost_woods --test schemas %*
exit /b %errorlevel%
