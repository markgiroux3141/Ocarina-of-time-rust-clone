@echo off
rem GAME-09 milestone 1's tests: the bomb (En_Bom: in hand, its fuse, its blast, put down, its
rem sphere), the Bomb Flower (En_Bombf: the tug without the bracelet, the lift and the hand-over,
rem the regrowth, a blast setting it off), the fuse's sparks (Effect_Ss_G_Spk), what blasts break
rem (Dodongo's Cavern's entrance wall and its opening, the lobby's boulders), the Peahat's death
rem (its bomb), the exit's run (Route::Bombs), and the actor schemas.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test bombs --test field --test schemas %*
exit /b %errorlevel%
