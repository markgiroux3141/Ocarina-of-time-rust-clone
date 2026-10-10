@echo off
rem Headless: GAME-06 milestone 4's scripted runs. By day (Route::Field): a grounded Peahat fought
rem from 600 east of it, then over the stream, up the lowland's slope and over the drawbridge into
rem Castle Town's entrance. At 20:00 (Route::FieldNight): two Stalchildren fought north of Lon Lon
rem Ranch, east over the stream bed, the owl at Kakariko's stairs, the ones that followed, and up
rem into Kakariko. Writes both traces and screenshots to out\run\.
rem (The frames move when the runs change: the traces' "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_HYRULE_FIELD_0 --child --preset deku-tree-dead --script field ^
    --trace out\run\field.json --screenshot out\run\field.png --shots-at 250,330 %*
if errorlevel 1 exit /b %errorlevel%
"%BIN%\oot_sandbox.exe" --entrance ENTR_HYRULE_FIELD_0 --child --preset deku-tree-dead --time 20:00 --script field-night ^
    --trace out\run\field-night.json --screenshot out\run\field-night.png --shots-at 80,700,1100 %*
exit /b %errorlevel%
