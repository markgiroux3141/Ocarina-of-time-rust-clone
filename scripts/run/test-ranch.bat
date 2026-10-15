@echo off
rem GAME-08 milestone 3's tests, step (a): the foal (En_Horse_Link_Child: where she is, her runs,
rem Epona's Song, coming to Link), the horses (En_Horse_Normal: the paddock's by day, the stable's
rem and their clones by night, the gallop round path 0), Ingo (En_In: where he works, his texts),
rem adult Malon's init (En_Ma2), the Guays (En_Crow: the dive, the kill, the respawn, the big one),
rem the exit's run (Route::Ranch), and the actor schemas.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test en_horse_link_child --test en_horse_normal --test en_in --test en_ma2 --test en_crow --test ranch --test schemas %*
exit /b %errorlevel%
