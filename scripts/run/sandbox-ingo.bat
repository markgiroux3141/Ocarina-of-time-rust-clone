@echo off
rem Headless: GAME-12 milestone 2's scripted runs. ingo-rental (adult-awake: Ingo's rental, the
rem corral, Epona mounted), ingo-race (ingo-rental: the rental, 0x2038 from horseback, the race's
rem intro to its start), ingo-lost (ingo-race-lost: Ingo's gloat) and ingo-won (ingo-first-win: the
rem second race called). (--child: the save the presets start from; they make Link an adult.)
rem Writes the traces and screenshots to out\run\. (The frames move when a run changes: the
rem trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_LON_LON_RANCH_0 --child --preset adult-awake --script ingo-rental ^
    --trace out\run\ingo-rental.json --screenshot out\run\ingo-rental.png --shots-at 100,400,800,1000,1250 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_LON_LON_RANCH_0 --child --preset ingo-rental --script ingo-race ^
    --trace out\run\ingo-race.json --screenshot out\run\ingo-race.png --shots-at 300,800,950,1000,1040 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_LON_LON_RANCH_8 --child --preset ingo-race-lost --script ingo-lost ^
    --trace out\run\ingo-lost.json --screenshot out\run\ingo-lost.png --shots-at 30,60 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_LON_LON_RANCH_7 --child --preset ingo-first-win --script ingo-won ^
    --trace out\run\ingo-won.json --screenshot out\run\ingo-won.png --shots-at 30,150 %*
exit /b %errorlevel%
