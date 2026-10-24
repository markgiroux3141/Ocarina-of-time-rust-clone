@echo off
rem Headless: GAME-11 milestone 2's scripted runs, chained by presets. door-of-time
rem (ocarina-of-time: the intro, the ocarina onto C-Down, the Song of Time, the door opening),
rem first-pull (door-of-time: the reveal, the pull, Ganondorf, the Chamber of the Sages),
rem awakening (light-medallion: Sheik), temple-exit (adult-awake: out to shrine_r) and ruins
rem (adult-awake: the ruined market round its ReDeads, its entrance, the field by the broken
rem drawbridge). (--child: the save the presets start from; light-medallion and adult-awake make
rem Link an adult.)
rem Writes the traces and screenshots to out\run\. (The frames move when a run changes: the
rem trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_TEMPLE_OF_TIME_0 --child --preset ocarina-of-time --script door-of-time ^
    --trace out\run\door-of-time.json --screenshot out\run\door-of-time.png --shots-at 60,190,420,700,1000,1300 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_TEMPLE_OF_TIME_0 --child --preset door-of-time --script first-pull ^
    --trace out\run\first-pull.json --screenshot out\run\first-pull.png --shots-at 400,700,1100,1500,2100,2700,3300,4400,5100 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_TEMPLE_OF_TIME_4 --child --preset light-medallion --script awakening ^
    --trace out\run\awakening.json --screenshot out\run\awakening.png --shots-at 100,300,500,800,1100,1500,1900,2300 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_TEMPLE_OF_TIME_0 --child --preset adult-awake --script temple-exit ^
    --trace out\run\temple-exit.json --screenshot out\run\temple-exit.png --shots-at 100,300 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_TEMPLE_OF_TIME_EXTERIOR_DAY_1 --child --preset adult-awake --script ruins ^
    --trace out\run\ruins.json --screenshot out\run\ruins.png --shots-at 200,600,700,900 %*
exit /b %errorlevel%
