@echo off
rem Headless: GAME-13 milestone 1's scripted runs. dampe-race (epona-won: down to Dampe, his
rem greeting, the race on, the first door opened) and hookshot (hookshot, Goron City's ledge: the
rem hookshot aimed at the stone post, the pull, up onto it). (--child: the save the presets start
rem from; they make Link an adult.) Writes the traces and screenshots to out\run\. (The frames
rem move when a run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_WINDMILL_AND_DAMPES_GRAVE_0 --child --preset epona-won --script dampe-race ^
    --trace out\run\dampe-race.json --screenshot out\run\dampe-race.png --shots-at 200,380,700,1000 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_GORON_CITY_2 --child --preset hookshot --script hookshot ^
    --trace out\run\hookshot.json --screenshot out\run\hookshot.png --shots-at 36,44,48,60 %*
exit /b %errorlevel%
