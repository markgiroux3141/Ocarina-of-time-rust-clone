@echo off
rem Headless: GAME-05 milestone 3b's scripted run inside the Deku Tree, from a debug start in
rem room 4: Link holds the guard until the Mad Scrub's nut bounces back off the Deku Shield and
rem knocks it out of its flower, then locks on, runs it down and slashes it. Writes its trace and
rem screenshots to out\run\: the scrub spitting, knocked out, caught, and dying.
rem (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_DEKU_TREE_0 --child --preset deku-tree-inside --script scrub ^
    --trace out\run\scrub.json --screenshot out\run\scrub.png --shots-at 40,75,140,160 %*
exit /b %errorlevel%
