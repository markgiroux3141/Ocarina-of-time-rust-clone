@echo off
rem GAME-10 milestone 1's tests: Player's water (the dive by the scale and the bottom pick-up,
rem swimming in from an entrance, the currents, Zora's Domain's finite water box, the cutscene holds
rem in water, the underwater view: lights, quake, distortion), the bottles (the swing and catch,
rem drinking, a fairy let out, a fish let out), the fish (En_Fish), the bugs (En_Insect), the
rem ripples, splashes and bubbles, the bottle run (Route::Bottle), the older water tests, and the
rem actor schemas.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test water_bottles --test water --test en_fish --test en_insect --test schemas %*
if errorlevel 1 exit /b %errorlevel%
cargo test --release -p oot_game view effect
exit /b %errorlevel%
