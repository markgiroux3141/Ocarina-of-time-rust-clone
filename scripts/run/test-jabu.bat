@echo off
rem GAME-10 milestone 4's tests: Jabu-Jabu's draw config (the breathing, the pulse quickened by a
rem hit, the walls on their own bones), its switches, moving parts and spouts (Bg_Bdan_Switch,
rem Bg_Bdan_Objects, En_Siofuki), the floor drifting into room 2's holes, the enemies (En_Bubble,
rem En_Bili, En_Brob, En_Weiyer, En_Eiyer, En_Rr), the runs (Route::JabuCow, JabuElevator,
rem JabuBoomerang, JabuDoor14) with the hand test's starts, and the actor schemas. Milestone 5's:
rem Ruto (En_Ru1), the tentacles (En_Ba), the barriers (En_Bx), the crates (Obj_Kibako), the room
rem timers (Obj_Roomtimer), and the runs Route::JabuRuto, JabuWeb, JabuCrystal. Milestone 8's: Big
rem Octo (En_Bigokuta), the Bari (En_Vali), the Tailpasarans (En_Tp), the platform's one-points 3070,
rem 3080, 3090 (in jabu_objects), and the runs Route::JabuSwitch, JabuOcto.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test jabu_objects --test jabu_runs --test en_bubble --test en_bili --test en_brob --test en_weiyer --test en_eiyer --test en_rr --test en_ru1 --test en_ba --test en_bx --test obj_kibako --test obj_roomtimer --test en_bigokuta --test en_vali --test en_tp --test jabu_octo_runs --test schemas %*
if errorlevel 1 exit /b %errorlevel%
cargo test --release -p oot_game --test scene jabu
exit /b %errorlevel%
