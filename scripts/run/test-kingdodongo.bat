@echo off
rem GAME-09 milestone 4's tests: King Dodongo (Boss_Dodongo: his intro and second try, the bomb
rem eaten and its blast inside him, the sword while he's down, the seed while he inhales, his death,
rem the decay, the lava floor's room texture and its wobble, the cleared room at init, the screen
rem fill), his fire breath (En_Bdfire), the lava's effects (Effect_Ss_G_Magma, G_Magma2), the warp's
rem branch (Door_Warp1), Darunia and the Gorons of the Ruby's cutscene (Demo_Du, Demo_Go), the two
rem runs (Route::KingDodongo, Route::Ruby), the camera's up and the actor schemas.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test boss_dodongo --test king_dodongo --test demo_du --test demo_go --test blue_warp --test schemas %*
if errorlevel 1 exit /b %errorlevel%
cargo test --release -p oot_game camera
exit /b %errorlevel%
