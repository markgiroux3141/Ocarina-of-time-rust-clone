@echo off
rem GAME-06 milestone 2's tests: Demo_Sa on the bridge against the C (her cues, the fade, the
rem ocarina), the soft soil (Obj_Bean), the owls (En_Owl: their flags, the field's owl's talk,
rem his question, one-point cutscene 8700, his flight), En_Ko child 3's place with the emerald,
rem Play_Init's Hyrule Field and Kokiri Forest layers, the Lost Woods' draw config against the C
rem interpreter, and the exit's run (Route::Farewell: Mido, Saria's goodbye and the Fairy
rem Ocarina, Hyrule Field's intro, the owl).
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test farewell %*
if errorlevel 1 exit /b %errorlevel%
cargo test --release -p oot_import --test pack ported_draw_configs %*
exit /b %errorlevel%
