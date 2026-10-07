@echo off
rem GAME-05 milestone 5a's tests: Player's Fairy Slingshot (out from C-Right, raised in first
rem person, drawn with a seed, shot on letting go, the string's spring, the flick with no seeds,
rem lowered, aiming Z-targeted), C-Up's first-person look, a Deku nut thrown, Start's stand-in
rem putting nuts and the slingshot on C buttons, the first-person draw; En_Arrow (seeds and nuts),
rem En_M_Fire1, Effect_Ss_Stone1 and the screen's flash; the eye switches and room 2's ladder hit
rem by real seeds; the room travel test; the debug starts; the exit's run (Route::Slingshot);
rem the knockdown's camera (BACKLOG #18).
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test slingshot --test slingshot_run --test arrows --test travel --test debug_starts --test damage %* || exit /b 1
cargo test --release -p eng_collision --lib
exit /b %errorlevel%
