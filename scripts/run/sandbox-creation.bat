@echo off
rem Headless: GAME-06 milestone 1b's scripted run: the cutscene map's layer 5 from its start
rem (--cutscene 0xFFF1 on the deku-tree-dead preset), A through the texts, parts 2 to 9 to the
rem terminator's ENTR_KOKIRI_FOREST_11. Writes its trace and screenshots to out\run\: the
rem goddesses' blue rain, Din's rocks, Gerudo Valley, Nayru's rings, the Triforce, the emerald.
rem (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_CUTSCENE_MAP_0 --child --preset deku-tree-dead --cutscene 0xFFF1 --script creation ^
    --trace out\run\creation.json --screenshot out\run\creation.png --shots-at 800,1500,1600,1800,2300,3700 %*
exit /b %errorlevel%
