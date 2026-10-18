@echo off
rem GAME-09 milestone 6's tests: the magic meter (oot_game's magic and interface), the Great Fairy
rem (Bg_Dy_Yoseizo: every reward, the revisit, her cutscene; En_Dy_Extra; Door_Warp1's orange warp),
rem the spin attack's effects and Player's charged spin (En_M_Thunder, Eff_Dust), the falling rocks
rem (En_Encount2, En_Fire_Rock), the return entrances, the three runs (Route::FairyWall,
rem GreatFairy, SummitOwl), the owl without magic, and the actor schemas.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test summit --test return_entrances --test bg_dy_yoseizo --test spin_attack --test en_encount2 --test schemas %*
if errorlevel 1 exit /b %errorlevel%
cargo test --release -p oot_game magic interface
exit /b %errorlevel%
