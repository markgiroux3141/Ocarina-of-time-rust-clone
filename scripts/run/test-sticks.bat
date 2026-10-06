@echo off
rem GAME-05 milestone 4b's tests: the Deku Stick (out from C-Left and put away, B taking out the
rem sword instead, none left, the Start stand-in, lit at a torch and burning down, breaking on a
rem target and a wall, swimming with it lit, room 4's timed torches and room 10's chest), the
rem torches and webs it lights and burns, Bg_Ydan_Hasi (the floating block, the water, the
rem rising platforms), Bg_Ydan_Maruta (the spiked log, the ladder), Obj_Kibako2 (the crates),
rem Obj_Lift, Effect_Ss_Kakera and its callers (the bushes', rocks' and boulder's pieces), the
rem Gold Skulltula behind the crate, the writable water boxes, and the exit's run (Route::Stick).
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test stick --test stick_run --test torches --test webs --test hasi --test maruta --test platform --test crates --test lift --test fragments --test skulltulas %* || exit /b 1
cargo test --release -p eng_collision --lib
exit /b %errorlevel%
