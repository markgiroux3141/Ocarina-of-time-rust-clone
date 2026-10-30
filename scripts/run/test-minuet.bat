@echo off
rem GAME-13 milestone 2's tests: the runs (the Lost Woods past Mido, Sheik's Minuet, the Minuet's
rem warp, the hookshot up to the Forest Temple), En_Mb, En_Xc (the Minuet's type 6), Obj_Mure3 and
rem the schemas.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test minuet_runs --test en_mb --test en_xc --test obj_mure3 --test schemas %*
exit /b %errorlevel%
