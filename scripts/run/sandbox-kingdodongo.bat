@echo off
rem Headless: GAME-09 milestone 4's two scripted runs. Route::KingDodongo (the room's spawn,
rem cavern-boss): a bomb on the cracked floor, the drop, King Dodongo's intro to the fight's start.
rem Route::Ruby (the arena's east rim after his defeat, cavern-boss-beaten): the heart container, the
rem blue warp, the Goron's Ruby's cutscene to Death Mountain Trail's spawn 5.
rem Writes the traces and screenshots to out\run\. (The frames move when a run changes: the
rem trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_DODONGOS_CAVERN_BOSS_0 --child --preset cavern-boss --script kingdodongo ^
    --trace out\run\kingdodongo.json --screenshot out\run\kingdodongo.png --shots-at 120,180,260,350,420,480,560,620,680,731 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_DODONGOS_CAVERN_BOSS_0 --child --preset cavern-boss-beaten --script ruby ^
    --trace out\run\ruby.json --screenshot out\run\ruby.png --shots-at 30,200,400,500,700,900,1200,1500,1900,2400,2800,3057 %*
exit /b %errorlevel%
