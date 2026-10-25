@echo off
rem Headless: GAME-12 milestone 1's scripted runs. ride-epona (adult-awake, the ranch's corral:
rem Epona's Song, on her, over a fence, off her) and field-ride (epona, Hyrule Field below the
rem ranch's gate: on her, up the ramp and into the ranch on her). (--child: the save the presets
rem start from; they make Link an adult.) Writes the traces and screenshots to out\run\. (The
rem frames move when a run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_LON_LON_RANCH_0 --child --preset adult-awake --script ride-epona ^
    --trace out\run\ride-epona.json --screenshot out\run\ride-epona.png --shots-at 100,300,450,600,700,790 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_HYRULE_FIELD_6 --child --preset epona --script field-ride ^
    --trace out\run\field-ride.json --screenshot out\run\field-ride.png --shots-at 300,430,480,520,560,620 %*
exit /b %errorlevel%
