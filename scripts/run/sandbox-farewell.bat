@echo off
rem Headless: GAME-06 milestone 2's scripted run: from ENTR_KOKIRI_FOREST_11 (deku-tree-dead),
rem Mido's talk, west through the forest to the Lost Woods' exit, Saria's goodbye on the bridge and
rem the Fairy Ocarina, Hyrule Field's intro, the owl's talk ("OK" at his question) and his flight.
rem Writes its trace and screenshots to out\run\: Saria and Link on the bridge, the ocarina held
rem out, the owl talking, the owl flying off.
rem (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_KOKIRI_FOREST_11 --child --preset deku-tree-dead --script farewell ^
    --trace out\run\farewell.json --screenshot out\run\farewell.png --shots-at 1800,2000,3300,3800,4560 %*
exit /b %errorlevel%
