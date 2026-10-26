@echo off
rem Headless: GAME-12 milestone 3's run, epona-won (ingo-second-win at ENTR_LON_LON_RANCH_7: Ingo's
rem gate scene, the gallop over the shut gate, the field's leap; its end is epona-won). (--child:
rem the save the presets start from; they make Link an adult.) Writes the trace and screenshots to
rem out\run\. (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_LON_LON_RANCH_7 --child --preset ingo-second-win --script epona-won ^
    --trace out\run\epona-won.json --screenshot out\run\epona-won.png --shots-at 150,383,600,683,720,800,900 %*
exit /b %errorlevel%
