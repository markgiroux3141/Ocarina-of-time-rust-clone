@echo off
rem GAME-10 milestone 4's tests: Jabu-Jabu's draw config (the breathing, the pulse quickened by a
rem hit, the walls on their own bones), its switches, moving parts and spouts (Bg_Bdan_Switch,
rem Bg_Bdan_Objects, En_Siofuki), the floor drifting into room 2's holes, the enemies (En_Bubble,
rem En_Bili, En_Brob, En_Weiyer, En_Eiyer, En_Rr), the runs (Route::JabuCow, JabuElevator,
rem JabuBoomerang, JabuDoor14) with the hand test's starts, and the actor schemas.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test jabu_objects --test jabu_runs --test en_bubble --test en_bili --test en_brob --test en_weiyer --test en_eiyer --test en_rr --test schemas %*
if errorlevel 1 exit /b %errorlevel%
cargo test --release -p oot_game --test scene jabu
exit /b %errorlevel%
