@echo off
rem Headless: GAME-13 milestone 2's scripted runs. lost-woods-adult (hookshot: the Lost Woods, Saria's
rem Song to Mido, on into the meadow), minuet (hookshot, the ramp's top: Sheik's lesson), minuet-warp
rem (minuet, Hyrule Field: the warp to the meadow's pad) and forest-temple (minuet, the meadow's top:
rem the hookshot into the branch, the ledge, the temple). (--child: the save the presets start from;
rem they make Link an adult.) Writes the traces and screenshots to out\run\. (The frames move when a
rem run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_LOST_WOODS_0 --child --preset hookshot --script lost-woods-adult ^
    --trace out\run\lost-woods-adult.json --screenshot out\run\lost-woods-adult.png --shots-at 560,640,990,1500 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_SACRED_FOREST_MEADOW_0 --child --preset hookshot --script minuet ^
    --trace out\run\minuet.json --screenshot out\run\minuet.png --shots-at 60,420,700,1160 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_HYRULE_FIELD_7 --child --preset minuet --script minuet-warp ^
    --trace out\run\minuet-warp.json --screenshot out\run\minuet-warp.png --shots-at 80,240,330,430 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_SACRED_FOREST_MEADOW_0 --child --preset minuet --script forest-temple ^
    --trace out\run\forest-temple.json --screenshot out\run\forest-temple.png --shots-at 45,78,100,190 %*
exit /b %errorlevel%
