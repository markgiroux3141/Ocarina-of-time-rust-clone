@echo off
rem Headless: Phase 4's exit run, from the file select's new file through the opening, C-Up to
rem Navi, and on into the Deku Tree. Writes its trace and screenshots to out\run\: the narration
rem over Link asleep, the nightmare, the flight through the village, Navi waking Link, her text
rem as he stands, the talk to her in Kokiri Forest, and the Deku Tree.
rem (The frames move when the run changes: the trace's "step" and "cutscene" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --new-file --child --script new-file-deku-tree ^
    --trace out\run\new_file_deku_tree.json --screenshot out\run\new_file_deku_tree.png --shots-at 300,1000,2500,3300,4000,5000,11748 %*
exit /b %errorlevel%
