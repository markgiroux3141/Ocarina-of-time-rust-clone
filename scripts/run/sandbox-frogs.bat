@echo off
rem Headless: GAME-10 milestone 9's two scripted runs. Route::Frogs (Zora's River on the frogs'
rem log, ocarina-c): the log's text, the six songs and their rewards, the song game.
rem Route::Bonooru (Lake Hylia in front of Bonooru, ocarina-c): the spawn song recorded, then
rem played back by him.
rem Writes the traces and screenshots to out\run\. (The frames move when a run changes: the
rem trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_ZORAS_RIVER_0 --child --preset ocarina-c --script frogs ^
    --trace out\run\frogs.json --screenshot out\run\frogs.png --shots-at 70,110,400,700,1300,3000,3500,3650,3800,4050 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_LAKE_HYLIA_0 --child --preset ocarina-c --script bonooru ^
    --trace out\run\bonooru.json --screenshot out\run\bonooru.png --shots-at 170,250,300,500,600,700 %*
exit /b %errorlevel%
